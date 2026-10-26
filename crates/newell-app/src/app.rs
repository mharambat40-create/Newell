// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! `App`: winit `ApplicationHandler` mapping window events to `Renderer` calls.
//!
//! Rendering is event-driven: inputs, a 5 Hz measured-FPS sample and timed progress steps.

use std::sync::Arc;
use std::time::{Duration, Instant};

use newell_kernel::NewellSolid;
use newell_model::{
    CommandController, CommandState, EntityId, ExtrudeGeometry, ExtrudeResult, ExtrudeSpec,
    ExtrudeTermination, ParameterValue, Selection,
};
use newell_render::{FrameOutcome, Renderer, SceneDisplayMode, SceneViewport, Theme};
use newell_ui::{
    ActionOperation, DEFAULT_SCALE_FACTOR_PERCENT, ExtrudeGeometryType, ExtrudeResultMode,
    ExtrudeStatus, ExtrudeTermination as UiExtrudeTermination, FeatureNodeId, GapDisplayLocation,
    GridMetrics, HistoryGap, OpaqueTreeId, ResizeSnapshot, ResizeZone, ScaleMode, ThemeChoice,
    TreeDocumentId, WindowAction, composed_grid_size, effective_scale, fit_window, k_fit,
    min_window_size,
};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::error::ExternalError;
use winit::event::{ElementState, InnerSizeWriter, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::monitor::MonitorHandle;
use winit::window::{CursorIcon, Fullscreen, ResizeDirection, Window, WindowId};

use crate::AppError;
use crate::controller::{CursorHint, Press, ShellController, Update};
use crate::document::{DocumentNotification, DocumentSession, ProjectionSync, ProjectionTrigger};
use crate::input::{pinch_steps, translate_key, ui_point, zoom_steps};
use crate::navigation::{DragKind, Navigation, ZoomSource};
use crate::scale::{MonitorInfo, Rescale, ScaleManager, plan_rescale, rect_in_window_space};
use crate::window::{CancelReason, ControlArm, ResizeManager, check_fit, screen_pointer};
use std::sync::mpsc::Receiver;

const TITLE: &str = "Newell";
/// A monitor change seen while the window is still being moved is applied once the window has
/// been still this long. The OS drag loop streams `Moved` events and resizing the window under
/// the user's hand fights it; the change is applied once, for the monitor where it comes to rest.
const SETTLE: Duration = Duration::from_millis(150);
/// `winit` 0.30 has no monitor-change event: the hosting monitor is re-read this often.
const MONITOR_POLL: Duration = Duration::from_millis(500);
/// Longest time frames are held back after a transition while the OS applies the new size.
const PRESENT_GATE: Duration = Duration::from_millis(250);

/// Reports a condition on stderr (the application has no logging framework yet).
fn log(message: &str) {
    eprintln!("newell: {message}");
}

/// Converts the shell-owned scene cells to the renderer's physical-pixel viewport.
fn scene_viewport(shell: &ShellController) -> SceneViewport {
    let rect = shell.scene_viewport();
    let metrics = shell.metrics();
    SceneViewport::new(
        rect.col().saturating_mul(metrics.cell_width_px()),
        rect.row().saturating_mul(metrics.cell_height_px()),
        rect.width().saturating_mul(metrics.cell_width_px()),
        rect.height().saturating_mul(metrics.cell_height_px()),
    )
}

fn scene_occlusions(shell: &ShellController) -> Vec<SceneViewport> {
    let metrics = shell.metrics();
    shell
        .scene_occlusions()
        .into_iter()
        .map(|rect| {
            SceneViewport::new(
                rect.col().saturating_mul(metrics.cell_width_px()),
                rect.row().saturating_mul(metrics.cell_height_px()),
                rect.width().saturating_mul(metrics.cell_width_px()),
                rect.height().saturating_mul(metrics.cell_height_px()),
            )
        })
        .collect()
}

fn scene_display_mode(shading: usize) -> SceneDisplayMode {
    match shading {
        0 => SceneDisplayMode::Points,
        1 => SceneDisplayMode::Line,
        2 => SceneDisplayMode::Wireframe,
        3 => SceneDisplayMode::HiddenLine,
        4 => SceneDisplayMode::NoShading,
        5 => SceneDisplayMode::Shaded,
        6 => SceneDisplayMode::Default,
        _ => SceneDisplayMode::Wireframe,
    }
}

const fn direction(zone: ResizeZone) -> ResizeDirection {
    match zone {
        ResizeZone::North => ResizeDirection::North,
        ResizeZone::South => ResizeDirection::South,
        ResizeZone::East => ResizeDirection::East,
        ResizeZone::West => ResizeDirection::West,
        ResizeZone::NorthEast => ResizeDirection::NorthEast,
        ResizeZone::NorthWest => ResizeDirection::NorthWest,
        ResizeZone::SouthEast => ResizeDirection::SouthEast,
        ResizeZone::SouthWest => ResizeDirection::SouthWest,
    }
}

/// A snapshot of `monitor` in its own physical pixels (`MonitorHandle::size` and `position`).
fn monitor_info(monitor: &MonitorHandle) -> MonitorInfo {
    let (position, size) = (monitor.position(), monitor.size());
    MonitorInfo::new(
        monitor_identity(monitor),
        (position.x, position.y),
        (size.width, size.height),
        monitor.scale_factor(),
    )
}

/// An identity for change detection and logs. The display id on macOS (`name()` is slow there).
#[cfg(target_os = "macos")]
fn monitor_identity(monitor: &MonitorHandle) -> Option<String> {
    use winit::platform::macos::MonitorHandleExtMacOS;
    Some(format!("display {}", monitor.native_id()))
}

#[cfg(not(target_os = "macos"))]
fn monitor_identity(monitor: &MonitorHandle) -> Option<String> {
    monitor.name()
}

/// The monitor that actually hosts `window`: its own `current_monitor`, not the primary one.
/// Only if the OS cannot say (a window not yet on screen) is the primary monitor, then the first
/// available one, used as a stand-in; the choice is re-derived by the next poll.
fn host_monitor(window: &Window) -> Option<MonitorHandle> {
    window
        .current_monitor()
        .or_else(|| window.primary_monitor())
        .or_else(|| window.available_monitors().next())
}

fn scale_mode(window: &Window) -> ScaleMode {
    if window.fullscreen().is_some() {
        ScaleMode::Fullscreen
    } else {
        ScaleMode::Windowed
    }
}

/// Frames are held back until the OS reports `size` or the deadline passes.
#[derive(Debug, Clone, Copy)]
struct PresentGate {
    size: (u32, u32),
    deadline: Instant,
}

#[derive(Debug, Default)]
struct ExtrudeCommitLatch {
    armed: bool,
    insertion: Option<(TreeDocumentId, HistoryGap)>,
}

fn ui_palette(shell: &newell_ui::ShellState) -> newell_render::UiPalette {
    let theme = match shell.theme() {
        ThemeChoice::Light => Theme::Light,
        ThemeChoice::Dark => Theme::Dark,
    };
    newell_render::UiPalette::new(theme, shell.color_enabled())
}

pub(crate) struct App {
    // Field order is drop order: the renderer (surface) is dropped before the window.
    renderer: Option<Renderer>,
    window: Option<Arc<Window>>,
    shell: Option<ShellController>,
    /// Persistent CAD state. The UI projects commands from this model; it never owns geometry.
    document: DocumentSession,
    document_events: Receiver<DocumentNotification>,
    projection_sync: ProjectionSync,
    extrude_latch: ExtrudeCommitLatch,
    committed_cursor: Option<(TreeDocumentId, OpaqueTreeId)>,
    layout_retry_rearm_pending: bool,
    #[cfg(test)]
    last_ui_publication: Option<newell_ui::CellGrid>,
    selection: Selection,
    commands: CommandController,
    /// Entity represented by the initial viewport solid.
    scene_entity: Option<EntityId>,
    modifiers: ModifiersState,
    navigation: Navigation,
    scene_pointer: Option<PhysicalPosition<f64>>,
    error: Option<AppError>,
    resize: ResizeManager,
    /// Last non-zero physical inner size reported by the OS.
    last_size: PhysicalSize<u32>,
    /// Preference, derived `k`, last evaluated inputs and the user's grid (`ARCHITECTURE.md`
    /// §3.1.1). The only owner of `effective_k`.
    scale: ScaleManager,
    /// The last `Moved`: a monitor change is applied once the window has been still.
    moved_at: Option<Instant>,
    last_poll: Instant,
    /// The window was not at its inside-the-monitor position after a scale-factor change
    /// (applied mid-drag): fix it when the window settles.
    reposition_pending: Option<(i32, i32)>,
    gate: Option<PresentGate>,
    /// A native (OS) resize is in progress: its `Resized` events are the user's grid.
    native_resizing: bool,
    /// Identities of the connected monitors, to log connection and disconnection.
    known_monitors: Vec<String>,
    drag_failure_logged: bool,
    /// A window control pressed and waiting for its release.
    control: ControlArm,
    /// Our own maximised state. `Window::is_maximized` toggles the style mask on macOS (see
    /// [`ControlArm`]) and is only called when the control fires, never on pointer input.
    maximized: bool,
    /// The OS reported the window as fully hidden (minimised, covered): no frame is delivered.
    occluded: bool,
    /// Monitor size for which a refused fullscreen request was already logged.
    fullscreen_refusal_logged: Option<(u32, u32)>,
    view_fullscreen: bool,
    /// Start of the monotonic clock handed to the controller for double-click timing.
    epoch: Instant,
}

impl App {
    pub(crate) fn new() -> Self {
        let mut document = DocumentSession::default();
        let document_events = document.subscribe();
        Self {
            renderer: None,
            window: None,
            shell: None,
            document,
            document_events,
            projection_sync: ProjectionSync::default(),
            extrude_latch: ExtrudeCommitLatch::default(),
            committed_cursor: None,
            layout_retry_rearm_pending: false,
            #[cfg(test)]
            last_ui_publication: None,
            selection: Selection::default(),
            commands: CommandController::default(),
            scene_entity: None,
            modifiers: ModifiersState::empty(),
            navigation: Navigation::default(),
            scene_pointer: None,
            error: None,
            resize: ResizeManager::default(),
            last_size: PhysicalSize::new(0, 0),
            scale: ScaleManager::default(),
            moved_at: None,
            last_poll: Instant::now(),
            reposition_pending: None,
            gate: None,
            native_resizing: false,
            known_monitors: Vec::new(),
            drag_failure_logged: false,
            control: ControlArm::default(),
            maximized: false,
            occluded: false,
            fullscreen_refusal_logged: None,
            view_fullscreen: false,
            epoch: Instant::now(),
        }
    }

    pub(crate) fn into_result(self) -> Result<(), AppError> {
        self.error.map_or(Ok(()), Err)
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl Into<AppError>) {
        self.error = Some(error.into());
        event_loop.exit();
    }

    /// Projects the UI-only Extrude editor into the M4 command controller. The application is
    /// the sole adapter between ASCII widget values and the model; neither side depends on the
    /// other. A valid draft immediately replaces the viewport mesh as a preview.
    fn sync_extrude_command(&mut self) {
        let Some(shell) = &self.shell else { return };
        if shell.shell().operation() != ActionOperation::Extrude {
            self.extrude_latch = ExtrudeCommitLatch::default();
            self.commands.cancel();
            return;
        }
        if self.projection_sync.acknowledged_document() != Some(self.document.identity().tree()) {
            self.extrude_latch = ExtrudeCommitLatch::default();
            return;
        }
        let ui = shell.shell().extrude();
        let depth = shell.shell().extrude_depth_source();
        let geometry = match ui.values.geometry {
            ExtrudeGeometryType::Solid => ExtrudeGeometry::Solid,
            ExtrudeGeometryType::Surface => ExtrudeGeometry::Surface,
        };
        let result = match ui.values.result {
            ExtrudeResultMode::New => ExtrudeResult::New,
            ExtrudeResultMode::Add => ExtrudeResult::Add,
            ExtrudeResultMode::Remove => ExtrudeResult::Remove,
        };
        let termination = match ui.values.termination {
            UiExtrudeTermination::Blind => ExtrudeTermination::Blind,
            UiExtrudeTermination::UpTo => ExtrudeTermination::UpTo,
            UiExtrudeTermination::ThroughAll => ExtrudeTermination::ThroughAll,
        };
        let spec = ExtrudeSpec {
            profile: self.selection.entities().first().copied(),
            geometry,
            result,
            termination,
            depth: ParameterValue::Literal(depth),
            starting_offset: ui
                .values
                .starting_offset
                .then(|| ParameterValue::Literal("0.0 mm".to_owned())),
            end_offset: ui
                .values
                .end_offset
                .then(|| ParameterValue::Literal("0.0 mm".to_owned())),
            symmetric: ui.values.symmetric,
            second_end: ui
                .values
                .second_side
                .then(|| ParameterValue::Literal("0.0 mm".to_owned())),
            invert: ui.values.invert,
            merge_with_all: ui.values.merge_with_all,
            merge_scope: if ui.values.merge_with_all {
                Vec::new()
            } else {
                self.selection.entities().to_vec()
            },
        };
        match ui.status {
            ExtrudeStatus::Editing => {
                if !self.extrude_latch.armed {
                    self.extrude_latch = ExtrudeCommitLatch {
                        armed: true,
                        insertion: Some((
                            self.document.identity().tree(),
                            shell.shell().playhead().insertion,
                        )),
                    };
                }
                if matches!(self.commands.state(), CommandState::Idle) {
                    self.commands.begin_extrude(&self.selection);
                }
                if self
                    .commands
                    .edit_extrude(spec, self.document.document())
                    .is_ok()
                    && let Some(preview) = self.commands.preview()
                    && let Ok(depth) = preview
                        .depth
                        .evaluate(self.document.document().parameters())
                {
                    self.set_preview_depth(depth.as_millimetres());
                }
            }
            ExtrudeStatus::Confirmed => {
                let latch = std::mem::take(&mut self.extrude_latch);
                let Some((document, gap)) = latch.insertion.filter(|_| latch.armed) else {
                    return;
                };
                if document != self.document.identity().tree() {
                    return;
                }
                let before = match gap {
                    HistoryGap::End => None,
                    HistoryGap::Before(id) => {
                        let Some(feature) = self.document.feature_id(id) else {
                            self.commands.cancel();
                            return;
                        };
                        Some(feature)
                    }
                };
                match self
                    .document
                    .confirm_before(&mut self.commands, spec, before)
                {
                    Ok(Some(id)) => {
                        self.committed_cursor = Some((document, OpaqueTreeId::new(id.get())));
                    }
                    Ok(None) => {}
                    Err(error) => log(&error.to_string()),
                }
            }
            ExtrudeStatus::Cancelled => {
                self.extrude_latch = ExtrudeCommitLatch::default();
                self.commands.cancel();
                self.set_preview_depth(10.0);
            }
        }
    }

    fn set_preview_depth(&mut self, depth_mm: f64) {
        let scaled = (depth_mm / 10.0).clamp(0.01, 100.0);
        let Ok(solid) = NewellSolid::box_solid(1.0, 1.0, scaled) else {
            return;
        };
        let Ok(mesh) = solid.tessellate(0.05) else {
            return;
        };
        if let Some(renderer) = &mut self.renderer {
            self.navigation.set_mesh(&mesh);
            renderer.set_scene_mesh(mesh);
            if let Some(camera) = self.navigation.snapshot() {
                renderer.set_scene_camera(camera);
            }
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Carries the View menu's choices out of the shell: the theme goes to the renderer's
    /// uniform, a changed UI Scale preference to the scale manager (one transition if
    /// `effective_k` changes), and a Fullscreen request to the window.
    fn sync_view_choices(&mut self) {
        let Some(shell) = &mut self.shell else { return };
        let palette = ui_palette(shell.shell());
        let shading = shell.shell().shading();
        let perspective = shell.shell().perspective();
        let scale_factor_percent = shell.shell().scale_factor_percent();
        let fullscreen = shell.take_fullscreen_request();
        let reset_camera = shell.take_camera_reset();
        let normal_view = shell.take_normal_view();
        if let Some(renderer) = &mut self.renderer {
            renderer.set_palette(palette);
            renderer.set_scene_display_mode(scene_display_mode(shading));
            self.navigation.set_perspective(perspective);
            if reset_camera {
                self.navigation.fit();
            }
            if normal_view {
                self.navigation.normal_to_axis();
            }
            if let Some(camera) = self.navigation.snapshot() {
                renderer.set_scene_camera(camera);
            }
        }
        if scale_factor_percent != self.scale.scale_factor_percent() {
            self.scale.set_scale_factor_percent(scale_factor_percent);
            // A preference whose `k` is unchanged only stores the preference.
            self.reconcile_with("preference", None, true);
        }
        if fullscreen {
            self.toggle_fullscreen();
        }
    }

    /// View > Fullscreen: leaves fullscreen, or enters it on the hosting monitor unless even
    /// `k = 2` would not fit there (refused, logged once per monitor size). The derivation of
    /// `k` and the window fit stay with the M2.17 transition, run by `reconcile`.
    fn toggle_fullscreen(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        self.cancel_resize(CancelReason::StateChanged);
        if window.fullscreen().is_some() {
            window.set_fullscreen(None);
            self.view_fullscreen = false;
        } else {
            let monitor = host_monitor(&window);
            let size = monitor.as_ref().map(|m| (m.size().width, m.size().height));
            if size.is_some_and(|size| k_fit(size, ScaleMode::Fullscreen) < 2) {
                if self.fullscreen_refusal_logged != size {
                    self.fullscreen_refusal_logged = size;
                    log(
                        "fullscreen refused: the monitor is smaller than the 82 x 30 minimum grid at k = 2",
                    );
                }
                return;
            }
            window.set_fullscreen(Some(Fullscreen::Borderless(monitor)));
            self.view_fullscreen = true;
        }
        self.reconcile("fullscreen", None);
    }

    fn poll_document_projection(&mut self, now: Instant, trigger: ProjectionTrigger) -> Update {
        if self
            .projection_sync
            .acknowledged_document()
            .is_some_and(|id| id != self.document.identity().tree())
        {
            self.extrude_latch = ExtrudeCommitLatch::default();
            self.committed_cursor = None;
            self.commands.cancel();
            if let Some(shell) = &mut self.shell {
                shell.invalidate_document_interactions();
            }
        }
        let Some(shell) = &mut self.shell else {
            return Update::default();
        };
        let mut update = match self.projection_sync.poll(
            &self.document,
            &self.document_events,
            now,
            trigger,
            |projection| {
                shell
                    .set_tree_projection(projection)
                    .map_err(AppError::from)
            },
        ) {
            Ok(update) => update.unwrap_or_default(),
            Err(error) => {
                log(&format!("tree refresh pending: {error}"));
                Update::default()
            }
        };
        if let Some((document, operation)) = self.committed_cursor
            && shell.shell().tree_projection().document == document
            && self.projection_sync.acknowledged_document() == Some(document)
        {
            let display = GapDisplayLocation::AfterSubtree(FeatureNodeId::Operation(operation));
            if let Ok(gap) = shell.shell().tree_projection().placed_gap(display) {
                match shell.set_playhead(gap) {
                    Ok(cursor) => {
                        update = update.merge(cursor);
                        self.committed_cursor = None;
                    }
                    Err(error) => log(&error.to_string()),
                }
            }
        }
        update
    }
    fn next_document_retry_deadline(&self) -> Option<Instant> {
        self.projection_sync.deadline()
    }
    fn finish_shell_input(&mut self, update: Update) {
        if let Some(shell) = &mut self.shell
            && let Some(intent) = shell.take_tree_intent()
            && intent.generation() == shell.generation()
            && intent.target().document == self.document.identity().tree()
            && let Err(error) = self
                .document
                .apply_tree_edit(intent.target(), intent.kind())
        {
            log(&error.to_string());
        }
        self.sync_extrude_command();
        let projection =
            self.poll_document_projection(Instant::now(), ProjectionTrigger::UserActivation);
        self.apply(update.merge(projection));
    }
    fn scroll_shell_panel(&mut self, steps: i32) -> bool {
        let Some(shell) = &mut self.shell else {
            return false;
        };
        let result = shell.scroll_panel(steps);
        if result.handled {
            self.navigation.cancel();
            self.apply(result.update);
        }
        result.handled
    }

    /// Uploads what `update` says changed and asks for a redraw.
    fn apply(&mut self, update: Update) {
        self.sync_view_choices();
        self.update_os_cursor();
        if !update.any() {
            return;
        }
        if let Some(shell) = &self.shell
            && update.grid
        {
            #[cfg(test)]
            {
                self.last_ui_publication = Some(shell.grid().clone());
            }
            if let Some(renderer) = &mut self.renderer {
                renderer.set_ui_grid(shell.grid());
                renderer.set_scene_viewport(scene_viewport(shell));
                renderer.set_scene_occlusions(scene_occlusions(shell));
                self.navigation
                    .set_layout(scene_viewport(shell), scene_occlusions(shell));
                if let Some(camera) = self.navigation.snapshot() {
                    renderer.set_scene_camera(camera);
                }
            }
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Updates the native cursor from the shell's O(1) hit target and press precedence.
    fn update_os_cursor(&self) {
        let (Some(window), Some(shell)) = (&self.window, &self.shell) else {
            return;
        };
        let size = window.inner_size();
        let icon = match shell.cursor_hint((size.width, size.height), self.resizable()) {
            CursorHint::Default => CursorIcon::Default,
            CursorHint::Resize(zone) => direction(zone).into(),
            CursorHint::HorizontalResize => CursorIcon::EwResize,
            CursorHint::VerticalResize => CursorIcon::NsResize,
            CursorHint::Move => CursorIcon::Move,
            CursorHint::Grab => CursorIcon::Grab,
            CursorHint::Grabbing => CursorIcon::Grabbing,
        };
        window.set_cursor(icon);
    }

    fn resize_to_window(&mut self) {
        if let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) {
            let size = window.inner_size();
            renderer.resize(size.width, size.height);
            self.resize_shell(size.width, size.height);
        }
    }

    /// Re-lays out the shell on the whole cells of the new window size and uploads the grid.
    fn resize_shell(&mut self, width: u32, height: u32) {
        if let Some(shell) = &mut self.shell {
            let update = shell.resize(width, height);
            self.layout_retry_rearm_pending |= update.grid;
            self.apply(update);
        }
    }

    /// The minimum inner size of the session's metrics, if the shell exists.
    fn min_size(&self) -> Option<PhysicalSize<u32>> {
        let (w, h) = min_window_size(self.shell.as_ref()?.metrics());
        Some(PhysicalSize::new(w, h))
    }

    /// Re-applies the minimum inner size; the OS may have dropped or rescaled it.
    fn apply_min_size(&self) {
        if let (Some(window), Some(min)) = (&self.window, self.min_size()) {
            window.set_min_inner_size(Some(min));
        }
    }

    /// Reads back what the OS actually created and logs, once, a window below the minimum or
    /// outside its monitor. Neither the request nor the monitor size is assumed to be honoured.
    fn verify_window(&self, window: &Window, min: (u32, u32)) {
        let inner = window.inner_size();
        let outer_size = window.outer_size();
        let outer = window
            .outer_position()
            .ok()
            .map(|p| ((p.x, p.y), (outer_size.width, outer_size.height)));
        let monitor = host_monitor(window)
            .map(|m| rect_in_window_space(&monitor_info(&m), window.scale_factor()));
        let report = check_fit(
            (inner.width, inner.height),
            min,
            outer.unwrap_or(((0, 0), (0, 0))),
            outer.and(monitor),
        );
        if report.below_minimum {
            log(&format!(
                "the window is {}x{} px, below the {}x{} px minimum; the grid is cropped",
                inner.width, inner.height, min.0, min.1
            ));
        }
        if report.exceeds_monitor {
            log("the window is not inside its monitor; the OS placed it there");
        }
    }

    /// Logs every runtime value the scale derivation used and produced (`ARCHITECTURE.md`
    /// §3.1.1; coordinates are the ones `winit` reports, never the macOS Display Settings).
    fn log_runtime(&self, reason: &str, window: &Window, rescale: &Rescale) {
        let monitor = rescale.monitor.as_ref().map_or_else(
            || "unknown".to_owned(),
            |m| {
                format!(
                    "{} pos={:?} size={}x{} scale_factor={}",
                    m.name.as_deref().unwrap_or("?"),
                    m.position,
                    m.size.0,
                    m.size.1,
                    m.scale_factor()
                )
            },
        );
        let (inner, outer) = (window.inner_size(), window.outer_size());
        let metrics = self.shell.as_ref().map(ShellController::metrics);
        let cell = metrics.map_or((0, 0), |m| (m.cell_width_px(), m.cell_height_px()));
        // `inner`/`outer` are read before the new size is applied: the OS reports it later.
        log(&format!(
            "scale[{reason}] monitor {monitor} | window outer_pos={:?} inner={}x{} outer={}x{} scale_factor={} mode={:?} | scale_factor={}% reference={} requested={} effective_k={} previous_k={:?} fits={} | cell_before={}x{} desired_grid={:?}",
            window.outer_position().ok().map(|p| (p.x, p.y)),
            inner.width,
            inner.height,
            outer.width,
            outer.height,
            window.scale_factor(),
            rescale.mode,
            self.scale.scale_factor_percent(),
            rescale.decision.reference,
            rescale.decision.requested,
            rescale.decision.k,
            rescale.previous_k,
            rescale.decision.fits,
            cell.0,
            cell.1,
            self.scale.desired_grid(),
        ));
    }

    /// Logs the connected monitors when the set changes (connection, disconnection, a resolution
    /// or scale change of any of them).
    fn log_monitors(&mut self, window: &Window) {
        let monitors: Vec<String> = window
            .available_monitors()
            .map(|m| {
                let i = monitor_info(&m);
                format!(
                    "{} pos={:?} size={}x{} scale_factor={}",
                    i.name.as_deref().unwrap_or("?"),
                    i.position,
                    i.size.0,
                    i.size.1,
                    i.scale_factor()
                )
            })
            .collect();
        if monitors != self.known_monitors {
            log(&format!("monitors: [{}]", monitors.join("; ")));
            self.known_monitors = monitors;
        }
    }

    /// Re-derives `effective_k` for the monitor that hosts the window now, and applies the
    /// change, if any, as one transition. Idempotent: unchanged inputs do nothing. The window's
    /// own size never feeds the derivation, so the resize this causes cannot start another.
    ///
    /// `writer` is the `ScaleFactorChanged` size writer, when called from that event: the target
    /// size is then written through it. Returns `true` if the writer was used.
    fn reconcile(&mut self, reason: &str, writer: Option<&mut InnerSizeWriter>) -> bool {
        self.reconcile_with(reason, writer, false)
    }

    /// [`Self::reconcile`]; with `skip_same_k` a change that leaves `effective_k` as it is
    /// (a UI Scale preference several percentages share) records its inputs and stops: no
    /// transition runs. A window still being moved is not skipped, so the pending monitor
    /// change is applied with it.
    fn reconcile_with(
        &mut self,
        reason: &str,
        writer: Option<&mut InnerSizeWriter>,
        skip_same_k: bool,
    ) -> bool {
        let Some(window) = self.window.clone() else {
            return false;
        };
        let monitor = host_monitor(&window).map(|m| monitor_info(&m));
        let Some(rescale) = self.scale.evaluate(monitor, scale_mode(&window)) else {
            return false;
        };
        if skip_same_k && !rescale.k_changed && self.moved_at.is_none() {
            return false;
        }
        self.log_runtime(reason, &window, &rescale);
        if rescale.log_limit {
            log(
                "the display is smaller than the 82 x 30 minimum grid at the smallest scale (k = 2); keeping k = 2",
            );
        }
        match self.apply_rescale(&window, &rescale, writer) {
            Ok(used) => used,
            Err(error) => {
                // The whole transition is abandoned; the old state is intact and the same
                // inputs are not retried until one changes.
                log(&format!("scale change abandoned: {error}"));
                self.scale.revert(rescale.previous_k);
                false
            }
        }
    }

    /// The transition (`ARCHITECTURE.md` §3.1.1): everything fallible is prepared first with
    /// no live state touched; the commit then publishes metrics, atlas, grid, GPU buffers,
    /// minimum size and pointer mapping together in this one event-loop turn.
    fn apply_rescale(
        &mut self,
        window: &Window,
        rescale: &Rescale,
        writer: Option<&mut InnerSizeWriter>,
    ) -> Result<bool, AppError> {
        let inner = window.inner_size();
        let origin = window.outer_position().ok().map(|p| (p.x, p.y));
        let mut plan = plan_rescale(
            rescale,
            self.scale.desired_grid(),
            window.scale_factor(),
            origin.unwrap_or((0, 0)),
            (inner.width, inner.height),
        )?;
        // Fullscreen and maximised windows are sized by the OS.
        let os_owned = rescale.mode == ScaleMode::Fullscreen || self.maximized;
        if os_owned {
            plan.size = (inner.width, inner.height);
        }
        // Phase A: prepare.
        let prepared = if rescale.k_changed {
            let (Some(renderer), Some(shell)) = (self.renderer.as_mut(), self.shell.as_ref())
            else {
                return Ok(false);
            };
            renderer.prepare_metrics(plan.metrics)?;
            Some(shell.prepare_metrics(plan.metrics, plan.size.0, plan.size.1)?)
        } else {
            None
        };
        // Phase B: commit. Nothing below can fail.
        self.cancel_resize(CancelReason::StateChanged);
        self.control.cancel();
        if let Some(prepared) = prepared
            && let (Some(renderer), Some(shell)) = (self.renderer.as_mut(), self.shell.as_mut())
        {
            renderer.set_metrics(prepared.metrics(), prepared.grid())?;
            let update = shell.commit_metrics(prepared);
            self.layout_retry_rearm_pending |= update.grid;
            renderer.set_scene_viewport(scene_viewport(shell));
            renderer.set_scene_occlusions(scene_occlusions(shell));
            self.navigation
                .set_layout(scene_viewport(shell), scene_occlusions(shell));
            if let Some(camera) = self.navigation.snapshot() {
                renderer.set_scene_camera(camera);
            }
        }
        let min = min_window_size(plan.metrics);
        log(&format!(
            "scale applied: effective_k={} cell={}x{} min_window={}x{} target_size={}x{} target_origin={:?} atlases_built={} desired_grid={:?}",
            rescale.decision.k,
            plan.metrics.cell_width_px(),
            plan.metrics.cell_height_px(),
            min.0,
            min.1,
            plan.size.0,
            plan.size.1,
            plan.origin,
            self.renderer
                .as_ref()
                .map_or(0, newell_render::Renderer::atlas_builds),
            self.scale.desired_grid(),
        ));
        window.set_min_inner_size(Some(PhysicalSize::new(min.0, min.1)));
        let target = PhysicalSize::new(plan.size.0, plan.size.1);
        let mut wrote = false;
        if !os_owned {
            // Size first, then position: macOS derives the vertical flip from the frame.
            match writer {
                Some(writer) => wrote = writer.request_inner_size(target).is_ok(),
                None => {
                    let _ = window.request_inner_size(target);
                }
            }
            if wrote {
                // Mid scale-factor event (often mid-drag): place it once settled.
                self.reposition_pending = Some(plan.origin);
            } else {
                // Resizing moves the top-left corner on macOS (the frame is anchored at the
                // bottom-left), so compare with the position read back after the size
                // request, not with the one read before it.
                let now = window.outer_position().ok().map(|p| (p.x, p.y));
                if origin.is_some() && now != Some(plan.origin) {
                    window.set_outer_position(PhysicalPosition::new(plan.origin.0, plan.origin.1));
                }
            }
        }
        // `last_size` is what a `ScaleFactorChanged` that finds nothing to change keeps: it must
        // be the size this transition asked for, not the size from before it.
        self.last_size = target;
        if wrote {
            // The OS reports the size after this event returns: hold frames until it does.
            self.gate = Some(PresentGate {
                size: plan.size,
                deadline: Instant::now() + PRESENT_GATE,
            });
        } else {
            self.resize_to_window();
            let actual = window.inner_size();
            if actual.width > 0 && actual.height > 0 {
                self.last_size = actual;
            }
            if !os_owned && (actual.width, actual.height) != plan.size {
                self.gate = Some(PresentGate {
                    size: plan.size,
                    deadline: Instant::now() + PRESENT_GATE,
                });
            }
            self.verify_window(window, min);
        }
        Ok(wrote)
    }

    /// Applies a deferred monitor change once the window has stopped moving, then places a
    /// window that a scale-factor change left partly off its monitor.
    fn settle(&mut self) {
        self.reconcile("settled", None);
        if let Some(origin) = self.reposition_pending.take() {
            self.reposition_inside(origin);
        }
    }

    /// One bounded correction: shifts (and if needed shrinks to whole cells) the window so it
    /// lies inside the usable rectangle of its monitor. Never loops: it changes nothing if the
    /// window already is inside.
    fn reposition_inside(&mut self, intended: (i32, i32)) {
        let (Some(window), Some(shell)) = (self.window.clone(), self.shell.as_ref()) else {
            return;
        };
        if self.maximized || window.fullscreen().is_some() {
            return;
        }
        let (Some(monitor), Ok(origin)) = (
            host_monitor(&window).map(|m| monitor_info(&m)),
            window.outer_position(),
        ) else {
            return;
        };
        let metrics = shell.metrics();
        let inner = window.inner_size();
        let rect = rect_in_window_space(&monitor, window.scale_factor());
        let grid = composed_grid_size(metrics, inner.width, inner.height);
        let fit = fit_window(metrics, rect, ScaleMode::Windowed, grid, intended);
        if fit.size != (inner.width, inner.height) {
            let _ = window.request_inner_size(PhysicalSize::new(fit.size.0, fit.size.1));
        }
        // Compare with the position read back after any size change (macOS moves the top-left
        // corner when it resizes).
        let now = window.outer_position().unwrap_or(origin);
        if fit.origin != (now.x, now.y) {
            window.set_outer_position(PhysicalPosition::new(fit.origin.0, fit.origin.1));
        }
    }

    /// `true` if the window can be resized from the grid: not maximised, not fullscreen.
    fn resizable(&self) -> bool {
        !self.maximized
            && self
                .window
                .as_ref()
                .is_some_and(|w| w.fullscreen().is_none())
    }

    fn cancel_resize(&mut self, reason: CancelReason) {
        self.resize.cancel(reason);
    }

    /// A left press: window control, resize zone, MenuBar drag or normal shell routing.
    fn left_pressed(&mut self) {
        self.navigation.cancel();
        // A press while a resize or control press is pending means its release was missed.
        self.cancel_resize(CancelReason::StateChanged);
        self.control.cancel();
        let resizable = self.resizable();
        let Some(window) = self.window.clone() else {
            return;
        };
        let size = window.inner_size();
        let (press, cursor, scene_input) = {
            let Some(shell) = &mut self.shell else { return };
            shell.set_time(self.epoch.elapsed());
            let cursor = shell.cursor();
            let scene_input = shell.scene_input_enabled();
            (
                shell.press((size.width, size.height), resizable),
                cursor,
                scene_input,
            )
        };
        match press {
            Press::Shell(update) => {
                self.finish_shell_input(update);
                if scene_input && let Some(cursor) = cursor {
                    let point = (f64::from(cursor.x), f64::from(cursor.y));
                    if !self.native_resizing
                        && let Some(pointer) = self.scene_pointer
                    {
                        self.navigation.begin_primary_orbit(
                            (pointer.x, pointer.y),
                            window.scale_factor(),
                            true,
                        );
                    }
                    if let Some(hit) = self.navigation.pick(point) {
                        if let Some(entity) = self.scene_entity {
                            self.selection.replace([entity]);
                        }
                        log(&format!(
                            "picked viewport face {} at {:.3}; {} entity selected",
                            hit.face_index,
                            hit.distance,
                            self.selection.entities().len()
                        ));
                    }
                }
            }
            Press::Window(action) => self.control.arm(action),
            Press::Drag => self.start_drag(&window),
            Press::Resize(zone) => self.start_resize(&window, zone),
        }
        if !matches!(press, Press::Shell(_)) {
            let update =
                self.poll_document_projection(Instant::now(), ProjectionTrigger::UserActivation);
            self.apply(update);
        }
    }

    fn window_action(
        &mut self,
        event_loop: &ActiveEventLoop,
        window: &Window,
        action: WindowAction,
    ) {
        self.cancel_resize(CancelReason::StateChanged);
        match action {
            WindowAction::Minimize => window.set_minimized(true),
            // Fullscreen is left by its own control (M2.18), not by maximise.
            WindowAction::ToggleMaximize => {
                if window.fullscreen().is_none() {
                    let maximize = !window.is_maximized();
                    window.set_maximized(maximize);
                    self.maximized = maximize;
                }
            }
            WindowAction::Close => event_loop.exit(),
        }
    }

    fn start_drag(&mut self, window: &Window) {
        if let Err(error) = window.drag_window()
            && !std::mem::replace(&mut self.drag_failure_logged, true)
        {
            log(&format!("window dragging failed: {error}"));
        }
    }

    /// Native resize first; on `NotSupported` (macOS) remember it, log once and manage the
    /// resize here. `Ignored` and `Os` errors are logged and do nothing.
    /// Removes the ASCII hover target and uploads the grid if it changed (M2.25).
    fn clear_hover(&mut self) {
        let update = self
            .shell
            .as_mut()
            .map(ShellController::clear_hover)
            .unwrap_or_default();
        self.apply(update);
    }

    fn start_resize(&mut self, window: &Window, zone: ResizeZone) {
        self.clear_hover();
        if !self.resize.native_unsupported() {
            match window.drag_resize_window(direction(zone)) {
                Ok(()) => {
                    self.native_resizing = true;
                    return;
                }
                Err(ExternalError::NotSupported(_)) => {
                    if self.resize.note_native_unsupported() {
                        log(
                            "native window resizing is not supported here; using application-managed resizing",
                        );
                    }
                }
                Err(error) => return log(&format!("window resizing failed: {error}")),
            }
        }
        let (Ok(origin), Some(cursor)) = (
            window.outer_position(),
            self.shell.as_ref().and_then(ShellController::cursor),
        ) else {
            return;
        };
        let origin = (origin.x, origin.y);
        let Some(grab) = screen_pointer(origin, (f64::from(cursor.x), f64::from(cursor.y))) else {
            return;
        };
        let size = window.inner_size();
        self.resize.begin(ResizeSnapshot {
            origin,
            size: (size.width, size.height),
            zone,
            grab,
        });
    }

    /// The pointer moved during a managed resize: re-derive the target from the snapshot and
    /// request it (size first, then position) when the snapped target changed.
    fn resize_pointer(&mut self, position: PhysicalPosition<f64>) {
        let (Some(window), Some(metrics)) = (
            &self.window,
            self.shell.as_ref().map(ShellController::metrics),
        ) else {
            return;
        };
        if self.maximized || window.fullscreen().is_some() {
            return self.cancel_resize(CancelReason::StateChanged);
        }
        let Ok(origin) = window.outer_position() else {
            return self.cancel_resize(CancelReason::InvalidPosition);
        };
        let Some(pointer) = screen_pointer((origin.x, origin.y), (position.x, position.y)) else {
            return self.cancel_resize(CancelReason::InvalidPosition);
        };
        if let Some(target) = self.resize.update(pointer, metrics) {
            // The user chose this grid: it is what a later monitor change tries to keep.
            let (cols, rows) = composed_grid_size(metrics, target.size.0, target.size.1);
            self.scale.set_desired_grid(cols, rows);
            // Order matters: macOS derives the vertical flip of `set_outer_position` from the
            // current frame. The size is applied asynchronously and reported by `Resized`.
            let _ = window.request_inner_size(PhysicalSize::new(target.size.0, target.size.1));
            window.set_outer_position(PhysicalPosition::new(target.origin.0, target.origin.1));
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Can be called more than once (macOS, Android).
        if self.window.is_some() {
            return;
        }
        // The monitor that will host the window is not known before it exists (a new macOS
        // window opens on the screen of the key window, which is not necessarily the primary
        // monitor). Create the window hidden with the primary monitor's scale, ask the window
        // which monitor hosts it, then derive and apply the real scale before it is shown.
        let primary = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next())
            .map(|m| (m.size().width, m.size().height));
        let k = effective_scale(primary, ScaleMode::Windowed, DEFAULT_SCALE_FACTOR_PERCENT).k;
        self.scale.start(k);
        // One metrics value for the atlas, layout and hit-testing.
        let metrics = match GridMetrics::from_k(k) {
            Ok(metrics) => metrics,
            Err(error) => return self.fail(event_loop, error),
        };
        let (min_w, min_h) = min_window_size(metrics);
        let min = PhysicalSize::new(min_w, min_h);
        let attributes = Window::default_attributes()
            .with_title(TITLE)
            .with_decorations(false)
            // macOS gives a borderless `Resizable` window its own edge hotspots, which resize
            // without snapping to whole cells and take the press before `winit` sees it. The
            // grid owns every resize instead (the managed path below).
            .with_resizable(!cfg!(target_os = "macos"))
            .with_visible(false)
            .with_inner_size(min)
            .with_min_inner_size(min);
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => return self.fail(event_loop, error),
        };
        let size = window.inner_size();
        self.last_size = size;
        match Renderer::new(window.clone(), size.width, size.height, metrics) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(error) => return self.fail(event_loop, error),
        }
        let (cols, rows) = composed_grid_size(metrics, size.width, size.height);
        let shell = match ShellController::new(cols, rows, metrics) {
            Ok(shell) => shell,
            Err(error) => return self.fail(event_loop, error),
        };
        if let Some(renderer) = &mut self.renderer {
            renderer.set_ui_grid(shell.grid());
            renderer.set_scene_viewport(scene_viewport(&shell));
            renderer.set_scene_occlusions(scene_occlusions(&shell));
            self.navigation
                .set_layout(scene_viewport(&shell), scene_occlusions(&shell));
            let solid = match NewellSolid::box_solid(1.0, 1.0, 1.0) {
                Ok(solid) => solid,
                Err(error) => return self.fail(event_loop, error),
            };
            let mesh = match solid.tessellate(0.05) {
                Ok(mesh) => mesh,
                Err(error) => return self.fail(event_loop, error),
            };
            self.navigation.set_mesh(&mesh);
            renderer.set_scene_mesh(mesh);
            if let Some(camera) = self.navigation.snapshot() {
                renderer.set_scene_camera(camera);
            }
        }
        let base = match self.document.add_base_solid("Base Solid") {
            Ok(base) => base,
            Err(error) => return self.fail(event_loop, error),
        };
        self.scene_entity = self
            .document
            .document()
            .feature(base)
            .and_then(|feature| feature.outputs().first().copied());
        self.shell = Some(shell);
        self.window = Some(window.clone());
        let update = self.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        self.apply(update);
        self.log_monitors(&window);
        // Derive the scale of the monitor that really hosts the window, before it is shown.
        let hosted = window.current_monitor().is_some();
        if hosted {
            self.reconcile("startup", None);
        }
        window.set_visible(true);
        if !hosted {
            self.reconcile("startup (after show)", None);
        }
        self.verify_window(
            &window,
            min_window_size(
                self.shell
                    .as_ref()
                    .map_or(metrics, ShellController::metrics),
            ),
        );
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.clone() else {
            return;
        };
        if window.id() != id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                self.cancel_resize(CancelReason::CloseRequested);
                event_loop.exit();
            }
            WindowEvent::Focused(true) => {
                if self.moved_at.is_none() {
                    self.reconcile("focus", None);
                }
            }
            WindowEvent::Focused(false) => {
                window.set_cursor(CursorIcon::Default);
                self.scene_pointer = None;
                self.navigation.cancel();
                self.native_resizing = false;
                self.cancel_resize(CancelReason::FocusLost);
                self.control.cancel();
                let update = if let Some(shell) = &mut self.shell {
                    let update = shell.pointer_moved(None);
                    shell.pointer_released();
                    update
                } else {
                    Update::default()
                };
                self.apply(update);
            }
            WindowEvent::Moved(_) => {
                // A move may be the middle of a drag across monitors: the hosting monitor is
                // evaluated once the window has been still (see `SETTLE`).
                self.moved_at = Some(Instant::now());
            }
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    self.last_size = size;
                }
                if self
                    .gate
                    .is_some_and(|g| g.size == (size.width, size.height))
                {
                    self.gate = None;
                }
                if self.native_resizing {
                    // An OS-managed resize by the user: this is the grid they chose.
                    if let Some(shell) = &self.shell {
                        let (cols, rows) =
                            composed_grid_size(shell.metrics(), size.width, size.height);
                        self.scale.set_desired_grid(cols, rows);
                    }
                }
                if self.resize.is_active() && !self.resizable() {
                    self.cancel_resize(CancelReason::StateChanged);
                }
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                self.resize_shell(size.width, size.height);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                // A display resolution change may arrive as a `Resized` alone.
                if self.moved_at.is_none() && !self.resize.is_active() {
                    self.reconcile("resized", None);
                }
            }
            // A Resized may not follow; re-read the physical size.
            WindowEvent::ScaleFactorChanged {
                mut inner_size_writer,
                ..
            } => {
                self.cancel_resize(CancelReason::ScaleFactorChanged);
                // The window now sits on a monitor with another scale factor: derive its scale
                // now and write the target size through the writer, so the OS applies the new
                // cell grid instead of a logical-points conversion of the old size. The
                // position is fixed when the window settles.
                self.moved_at = Some(Instant::now());
                if !self.reconcile("scale-factor", Some(&mut inner_size_writer)) {
                    // Nothing changed: keep the physical surface size (the grid is in
                    // physical pixels) and the minimum; neither request is assumed to succeed.
                    if let Some(min) = self.min_size() {
                        let keep = PhysicalSize::new(
                            self.last_size.width.max(min.width),
                            self.last_size.height.max(min.height),
                        );
                        let _ = inner_size_writer.request_inner_size(keep);
                    }
                    self.apply_min_size();
                    self.resize_to_window();
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } if self.resize.is_active() => {
                // The shell keeps the pointer for the next press; it sees no drag and no hover.
                let update = self
                    .shell
                    .as_mut()
                    .map(|shell| {
                        shell.pointer_moved_during_resize(ui_point(position.x, position.y))
                    })
                    .unwrap_or_default();
                self.apply(update);
                self.resize_pointer(position);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.scene_pointer = Some(position);
                let point = ui_point(position.x, position.y);
                let camera_changed = self.navigation.advance_pointer((position.x, position.y));
                if self.navigation.is_dragging() {
                    if camera_changed {
                        window.request_redraw();
                        if let (Some(renderer), Some(camera)) =
                            (&mut self.renderer, self.navigation.snapshot())
                        {
                            renderer.set_scene_camera(camera);
                        }
                    }
                    if let Some(shell) = &mut self.shell {
                        shell.pointer_moved_during_resize(point);
                    }
                    return;
                }
                // An OS-managed resize owns the pointer: no hover until its first move after it.
                let native = self.native_resizing;
                if let Some(shell) = &mut self.shell {
                    let update = if native {
                        shell.pointer_moved_during_resize(point)
                    } else {
                        shell.pointer_moved(point)
                    };
                    self.apply(update);
                }
            }
            WindowEvent::CursorLeft { .. } => {
                window.set_cursor(CursorIcon::Default);
                self.scene_pointer = None;
                self.navigation.cancel();
                if let Some(shell) = &mut self.shell {
                    let update = shell.pointer_moved(None);
                    self.apply(update);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.left_pressed(),
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.native_resizing = false;
                self.navigation.release(DragKind::PrimaryOrbit);
                // The release that ends a managed resize is consumed.
                if !self.resize.release() {
                    let under = self
                        .shell
                        .as_ref()
                        .and_then(ShellController::window_control_at_cursor);
                    if let Some(action) = self.control.release(under)
                        && let Some(window) = self.window.clone()
                    {
                        self.window_action(event_loop, &window, action);
                    }
                    if let Some(shell) = &mut self.shell {
                        shell.pointer_released();
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: button @ (MouseButton::Right | MouseButton::Middle),
                ..
            } => {
                if let Some(cursor) = self.scene_pointer {
                    let eligible = !self.resize.is_active()
                        && !self.native_resizing
                        && self
                            .shell
                            .as_ref()
                            .is_some_and(ShellController::scene_input_enabled);
                    if let Some(kind) = DragKind::from_auxiliary_button(button) {
                        if kind == DragKind::SecondaryPan {
                            self.navigation.begin_secondary_pan(
                                (cursor.x, cursor.y),
                                window.scale_factor(),
                                eligible,
                            );
                        } else {
                            self.navigation.begin(
                                kind,
                                (cursor.x, cursor.y),
                                window.scale_factor(),
                                eligible,
                            );
                        }
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: button @ (MouseButton::Right | MouseButton::Middle),
                ..
            } => {
                if let Some(kind) = DragKind::from_auxiliary_button(button) {
                    self.navigation.release(kind);
                }
            }
            WindowEvent::MouseWheel { delta, phase, .. } => {
                if self.resize.is_active() || self.native_resizing {
                    return;
                }
                if let MouseScrollDelta::PixelDelta(pixels) = delta {
                    let point = self
                        .scene_pointer
                        .map_or((f64::NAN, f64::NAN), |cursor| (cursor.x, cursor.y));
                    if let Some(changed) = self.navigation.advance_secondary_pan_scroll(
                        (pixels.x, pixels.y),
                        phase,
                        point,
                    ) {
                        if changed {
                            if let (Some(renderer), Some(camera)) =
                                (&mut self.renderer, self.navigation.snapshot())
                            {
                                renderer.set_scene_camera(camera);
                            }
                            window.request_redraw();
                        }
                        return;
                    }
                }
                let steps = zoom_steps(delta, window.scale_factor());
                // Panel scrolling keeps its existing convention, independently of camera sensitivity.
                let panel_steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(delta) => (delta.y / 40.0) as f32,
                };
                if self.scroll_shell_panel(panel_steps.round() as i32) {
                    return;
                }
                let eligible = !self.resize.is_active()
                    && !self.native_resizing
                    && self
                        .shell
                        .as_ref()
                        .is_some_and(ShellController::scene_input_enabled);
                let stream = match delta {
                    MouseScrollDelta::LineDelta(..) => None,
                    MouseScrollDelta::PixelDelta(_) => Some((ZoomSource::Scroll, phase)),
                };
                let point = self
                    .scene_pointer
                    .map_or((f64::NAN, f64::NAN), |cursor| (cursor.x, cursor.y));
                if self
                    .navigation
                    .zoom(point, f64::from(steps), stream, eligible)
                {
                    if let (Some(renderer), Some(camera)) =
                        (&mut self.renderer, self.navigation.snapshot())
                    {
                        renderer.set_scene_camera(camera);
                    }
                    window.request_redraw();
                }
            }
            WindowEvent::PinchGesture { delta, phase, .. } => {
                let eligible = !self.resize.is_active()
                    && !self.native_resizing
                    && self
                        .shell
                        .as_ref()
                        .is_some_and(ShellController::scene_input_enabled);
                let point = self
                    .scene_pointer
                    .map_or((f64::NAN, f64::NAN), |cursor| (cursor.x, cursor.y));
                if self.navigation.zoom(
                    point,
                    f64::from(pinch_steps(delta)),
                    Some((ZoomSource::Pinch, phase)),
                    eligible,
                ) {
                    if let (Some(renderer), Some(camera)) =
                        (&mut self.renderer, self.navigation.snapshot())
                    {
                        renderer.set_scene_camera(camera);
                    }
                    window.request_redraw();
                }
            }
            WindowEvent::Occluded(occluded) => {
                if occluded {
                    self.navigation.cancel();
                }
                self.occluded = occluded;
                if !occluded {
                    window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                self.navigation.cancel();
                if event.logical_key == Key::Named(NamedKey::Escape) && self.view_fullscreen {
                    if let Some(window) = &self.window {
                        window.set_fullscreen(None);
                    }
                    self.view_fullscreen = false;
                    self.reconcile("fullscreen", None);
                    let update = self.poll_document_projection(
                        Instant::now(),
                        ProjectionTrigger::UserActivation,
                    );
                    self.apply(update);
                    return;
                }
                // Home fits only when the pointer is on unobstructed scene pixels.
                if event.logical_key == Key::Named(NamedKey::Home)
                    && self.scene_pointer.is_some()
                    && !self.resize.is_active()
                    && !self.native_resizing
                    && self.shell.as_ref().is_some_and(|shell| {
                        shell.scene_input_enabled() && shell.shell().editing_field().is_none()
                    })
                {
                    self.navigation.fit();
                    if let (Some(renderer), Some(camera)) =
                        (&mut self.renderer, self.navigation.snapshot())
                    {
                        renderer.set_scene_camera(camera);
                    }
                    window.request_redraw();
                    let update = self.poll_document_projection(
                        Instant::now(),
                        ProjectionTrigger::UserActivation,
                    );
                    self.apply(update);
                    return;
                }
                // Escape cancels a managed resize and is not seen by the shell.
                if event.logical_key == Key::Named(NamedKey::Escape)
                    && self.resize.cancel(CancelReason::Escape)
                {
                    let update = self.poll_document_projection(
                        Instant::now(),
                        ProjectionTrigger::UserActivation,
                    );
                    self.apply(update);
                    return;
                }
                let keys = translate_key(
                    &event.logical_key,
                    event.physical_key,
                    event.text.as_deref(),
                    self.modifiers,
                );
                if keys.is_empty() {
                    let update = self.poll_document_projection(
                        Instant::now(),
                        ProjectionTrigger::UserActivation,
                    );
                    self.apply(update);
                }
                for key in keys {
                    if let Some(shell) = &mut self.shell {
                        let update = shell.key(key);
                        self.finish_shell_input(update);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(gate) = self.gate {
                    if Instant::now() < gate.deadline {
                        // The OS has not reported the new size yet: keep the last frame.
                        return;
                    }
                    self.gate = None;
                    let actual = window.inner_size();
                    if (actual.width, actual.height) != gate.size {
                        log(&format!(
                            "the window is {}x{} px, not the requested {}x{} px; the grid follows the actual size",
                            actual.width, actual.height, gate.size.0, gate.size.1
                        ));
                    }
                }
                if let Some(renderer) = &mut self.renderer {
                    match renderer.render() {
                        Ok(FrameOutcome::Presented) => {
                            if let Some(shell) = &mut self.shell {
                                shell.presented();
                            }
                        }
                        Ok(FrameOutcome::Skipped) => {}
                        Err(error) => self.fail(event_loop, error),
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.clone() else {
            return event_loop.set_control_flow(ControlFlow::Wait);
        };
        // macOS may not deliver Resized(0, 0) on minimise, so also ask the window.
        // Hidden windows wait for an OS event instead of refreshing unseen status cells.
        let hidden = self.occluded || window.is_minimized().unwrap_or(false);
        let now = Instant::now();
        let mut wake = None;
        if let Some(moved) = self.moved_at {
            if now.duration_since(moved) >= SETTLE {
                self.moved_at = None;
                self.settle();
            } else {
                wake = Some(moved + SETTLE);
            }
        } else if now.duration_since(self.last_poll) >= MONITOR_POLL {
            // Monitor resolution, display scale, fullscreen and connection changes arrive
            // with no event of their own.
            self.last_poll = now;
            self.reconcile("poll", None);
            if let Some(window) = self.window.clone() {
                self.log_monitors(&window);
            }
        }
        // No continuous redraw loop. Hidden windows wait for input instead of spinning
        // on expired status deadlines; the monotonic animation catches up when restored.
        if !hidden {
            let trigger = if std::mem::take(&mut self.layout_retry_rearm_pending) {
                ProjectionTrigger::LayoutPublished
            } else {
                ProjectionTrigger::RetryDeadline
            };
            let update = self.poll_document_projection(now, trigger);
            self.apply(update);
            if let Some(deadline) = self.next_document_retry_deadline() {
                wake = Some(wake.map_or(deadline, |other: Instant| other.min(deadline)));
            }
            let elapsed = self.epoch.elapsed();
            if let Some(shell) = &mut self.shell {
                let update = shell.tick(elapsed);
                self.apply(update);
            }
            if let Some(shell) = &self.shell {
                let deadline = self.epoch + shell.status_deadline();
                wake = Some(wake.map_or(deadline, |other| other.min(deadline)));
            }
            // Keep monitor reconciliation independent from render/FPS sampling.
            let poll = self.last_poll + MONITOR_POLL;
            if poll > now {
                wake = Some(wake.map_or(poll, |other| other.min(poll)));
            }
            if let Some(gate) = self.gate {
                if gate.deadline > now {
                    wake = Some(wake.map_or(gate.deadline, |other| other.min(gate.deadline)));
                } else {
                    window.request_redraw();
                }
            }
        }

        event_loop.set_control_flow(wake.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use newell_ui::{CellContent, GridMetrics, ShellKey, ShellState, UiPoint};

    pub(super) fn live_app() -> App {
        let mut app = App::new();
        app.shell = Some(
            ShellController::with_shell(ShellState::new(82, 40), GridMetrics::new(4, 9).unwrap())
                .unwrap(),
        );
        let update = app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        app.apply(update);
        app
    }
    pub(super) fn click(app: &mut App, needle: &str) {
        let shell = app.shell.as_mut().unwrap();
        let pattern: Vec<_> = needle.chars().collect();
        let point = (0..shell.grid().rows())
            .find_map(|row| {
                let text: Vec<_> = (0..shell.grid().cols())
                    .map(|col| match shell.grid().get(col, row).unwrap().content {
                        CellContent::Glyph(c) => c,
                        CellContent::Empty => ' ',
                    })
                    .collect();
                text.windows(pattern.len())
                    .position(|window| window == pattern)
                    .map(|col| UiPoint {
                        x: col as u32 * 4 + 2,
                        y: row * 9 + 4,
                    })
            })
            .expect("requested control is visible");
        shell.pointer_moved(Some(point));
        let update = shell.pointer_pressed();
        shell.pointer_released();
        app.finish_shell_input(update);
    }
    pub(super) fn key(app: &mut App, key: ShellKey) {
        let update = app.shell.as_mut().unwrap().key(key);
        app.finish_shell_input(update);
    }
    fn depth(app: &mut App) {
        click(app, "0.0 mm");
        key(app, ShellKey::Edit(newell_ui::EditKey::SelectAll));
        key(app, ShellKey::Char('1'));
        key(app, ShellKey::Char('0'));
        key(app, ShellKey::Enter);
        let source = app.shell.as_ref().unwrap().shell().extrude_depth_source();
        assert_eq!(
            ParameterValue::Literal(source)
                .evaluate(app.document.document().parameters())
                .unwrap()
                .as_millimetres(),
            10.0
        );
    }

    #[test]
    fn startup_command_controller_is_idle_without_preview() {
        let app = App::new();
        assert_eq!(app.commands.state(), &CommandState::Idle);
        assert!(app.commands.preview().is_none());
    }

    #[test]
    fn one_confirmation_is_consumed_before_subsequent_tree_and_keyboard_events() {
        for enabled in [false, true] {
            let mut app = live_app();
            if enabled {
                click(&mut app, "View");
                click(&mut app, "Color");
                key(&mut app, ShellKey::Escape);
            }
            let events = app.document.subscribe();
            app.finish_shell_input(Update::default());
            depth(&mut app);
            click(&mut app, "OK");
            assert_eq!(app.document.document().features().len(), 1);
            assert_eq!(events.try_iter().count(), 1);
            assert!(!app.extrude_latch.armed);
            click(&mut app, "Features");
            for input in [
                ShellKey::Tab,
                ShellKey::Enter,
                ShellKey::Char('x'),
                ShellKey::Escape,
            ] {
                key(&mut app, input);
            }
            assert_eq!(app.document.document().features().len(), 1);
            assert!(events.try_recv().is_err());
            click(&mut app, "Create");
            click(&mut app, "[E]xtrude");
            depth(&mut app);
            click(&mut app, "OK");
            assert_eq!(app.document.document().features().len(), 2);
            assert_eq!(events.try_iter().count(), 1);
        }
    }

    #[test]
    fn invalid_insert_consumes_latch_without_changing_command_document_or_revision() {
        for enabled in [false, true] {
            let mut app = live_app();
            let base = app.document.add_base_solid("base").unwrap();
            let update =
                app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
            app.apply(update);
            let id = app.document.document().feature(base).unwrap().outputs()[0];
            app.selection.replace([id]);
            let shell = app.shell.as_mut().unwrap();
            let gap = shell
                .shell()
                .tree_projection()
                .placed_gap(GapDisplayLocation::BeforeNode(FeatureNodeId::Operation(
                    OpaqueTreeId::new(base.get()),
                )))
                .unwrap();
            shell.set_playhead(gap).unwrap();
            // First user activation captures the already selected insertion anchor.
            if enabled {
                click(&mut app, "View");
                click(&mut app, "Color");
                key(&mut app, ShellKey::Escape);
            }
            app.finish_shell_input(Update::default());
            depth(&mut app);
            let document = app.document.document().clone();
            let revision = app.document.revision();
            let state = app.commands.state().clone();
            let preview = app.commands.preview().cloned();
            let events = app.document.subscribe();
            click(&mut app, "OK");
            assert_eq!(app.document.document(), &document);
            assert_eq!(app.document.revision(), revision);
            assert_eq!(app.commands.state(), &state);
            assert_eq!(app.commands.preview(), preview.as_ref());
            assert!(events.try_recv().is_err());
            assert!(!app.extrude_latch.armed);
            click(&mut app, "Features");
            key(&mut app, ShellKey::Tab);
            assert_eq!(app.commands.state(), &state);
            assert_eq!(app.document.document(), &document);
            assert!(events.try_recv().is_err());
        }
    }

    #[test]
    fn tree_scroll_publishes_the_same_grid_the_controller_hit_tests() {
        let mut app = live_app();
        for index in 0..40 {
            app.document
                .add_base_solid(&format!("operation_{index}"))
                .unwrap();
        }
        let update = app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        app.apply(update);
        let before = app.shell.as_ref().unwrap().grid().clone();
        app.shell
            .as_mut()
            .unwrap()
            .pointer_moved(Some(UiPoint { x: 20, y: 72 }));
        assert!(app.scroll_shell_panel(-5));
        let shell = app.shell.as_ref().unwrap();
        assert_ne!(shell.grid(), &before);
        assert_eq!(app.last_ui_publication.as_ref(), Some(shell.grid()));
        assert!(app.scroll_shell_panel(i32::MIN));
        app.last_ui_publication = None;
        assert!(app.scroll_shell_panel(-1));
        assert!(
            app.last_ui_publication.is_none(),
            "a tree owns the wheel at its limit without another publication"
        );
    }

    #[test]
    fn replacement_cancels_old_frame_authorization_even_if_publication_fails() {
        let mut app = live_app();
        app.document.add_base_solid("old").unwrap();
        let update = app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        app.apply(update);
        let shell = app.shell.as_mut().unwrap();
        shell.pointer_moved(Some(UiPoint {
            x: 24 * 4 + 2,
            y: 10 * 9 + 4,
        }));
        shell.pointer_pressed();
        assert!(shell.dragging());
        let old = shell.grid().clone();
        app.document
            .replace(newell_model::Document::default())
            .unwrap();
        app.projection_sync.fail_next_projection = 1;
        let update = app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        assert!(!update.grid);
        assert!(!app.shell.as_ref().unwrap().dragging());
        assert_eq!(app.shell.as_ref().unwrap().grid(), &old);
        assert!(!app.extrude_latch.armed);
    }

    #[test]
    fn extrude_uses_captured_insertion_anchor_even_if_cursor_moves_while_editing() {
        let mut app = live_app();
        let base = app.document.add_base_solid("base").unwrap();
        let update = app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        app.apply(update);
        let shell = app.shell.as_mut().unwrap();
        let gap = shell
            .shell()
            .tree_projection()
            .placed_gap(GapDisplayLocation::BeforeNode(FeatureNodeId::Operation(
                OpaqueTreeId::new(base.get()),
            )))
            .unwrap();
        shell.set_playhead(gap).unwrap();
        app.finish_shell_input(Update::default());
        depth(&mut app);
        app.shell
            .as_mut()
            .unwrap()
            .set_playhead(newell_ui::PlacedHistoryGap::default())
            .unwrap();
        click(&mut app, "OK");
        let operations = app.document.document().features();
        assert_eq!(operations.len(), 2);
        assert_eq!(operations[0].0.get(), 1);
        assert_eq!(operations[1].0, base);
        assert_eq!(
            app.shell.as_ref().unwrap().shell().playhead().insertion,
            HistoryGap::Before(OpaqueTreeId::new(base.get()))
        );
    }

    #[test]
    fn long_real_entity_name_round_trips_through_confirm_without_a_notification() {
        let mut app = live_app();
        let name = format!("{} ", "long_name".repeat(15));
        let feature = app.document.add_base_solid(&name).unwrap();
        let entity = app.document.document().feature(feature).unwrap().outputs()[0];
        let update = app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        app.apply(update);
        let shell = app.shell.as_ref().unwrap();
        let row = (0..shell.grid().rows())
            .find(|row| {
                let text: String = (0..shell.grid().cols())
                    .map(|col| match shell.grid().get(col, *row).unwrap().content {
                        CellContent::Glyph(c) => c,
                        CellContent::Empty => ' ',
                    })
                    .collect();
                text.contains("■ long_name")
            })
            .unwrap();
        let shell = app.shell.as_mut().unwrap();
        shell.pointer_moved(Some(UiPoint {
            x: 8 * 4 + 2,
            y: row * 9 + 4,
        }));
        shell.pointer_pressed();
        shell.set_time(Duration::from_millis(100));
        shell.pointer_pressed();
        assert!(shell.shell().rename().is_open());
        assert_eq!(shell.shell().rename().draft(), name);
        let events = app.document.subscribe();
        key(&mut app, ShellKey::Enter);
        assert_eq!(app.document.document().entity(entity).unwrap().name(), name);
        assert!(events.try_recv().is_err());
    }

    #[test]
    fn real_tree_edits_publish_once_and_reproject_without_shared_selection() {
        let mut app = live_app();
        let feature = app.document.add_base_solid("base").unwrap();
        let entity = app.document.document().feature(feature).unwrap().outputs()[0];
        let update = app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        app.apply(update);
        let events = app.document.subscribe();
        click(&mut app, "■ base");
        assert!(!app.document.document().entity(entity).unwrap().visible());
        assert_eq!(events.try_iter().count(), 1);
        assert_eq!(
            app.last_ui_publication.as_ref(),
            Some(app.shell.as_ref().unwrap().grid())
        );
        let projection = app.shell.as_ref().unwrap().shell().tree_projection();
        assert_eq!(projection.labels.bodies, "Bodies");
        assert_eq!(
            projection.elements.len(),
            1,
            "hidden bodies remain in the group count"
        );
        key(&mut app, ShellKey::Enter);
        assert!(
            events.try_recv().is_err(),
            "keyboard leaf activation does not toggle visibility"
        );
        assert!(app.selection.entities().is_empty());
        click(&mut app, "base");
        click(&mut app, "base");
        assert!(app.shell.as_ref().unwrap().shell().rename().is_open());
        for c in "_renamed".chars() {
            key(&mut app, ShellKey::Char(c));
        }
        key(&mut app, ShellKey::Enter);
        assert_eq!(
            app.document.document().feature(feature).unwrap().name(),
            "base_renamed"
        );
        assert_eq!(
            app.document.document().entity(entity).unwrap().name(),
            "base",
            "entity names are independent"
        );
        assert_eq!(events.try_iter().count(), 1);
        assert_eq!(
            app.last_ui_publication.as_ref(),
            Some(app.shell.as_ref().unwrap().grid())
        );
        assert_eq!(
            app.shell
                .as_ref()
                .unwrap()
                .shell()
                .tree_projection()
                .features[0]
                .name,
            "base_renamed"
        );
        assert!(app.selection.entities().is_empty());
    }
}

#[cfg(test)]
mod display_mode_tests {
    use super::scene_display_mode;
    use newell_render::SceneDisplayMode;
    use newell_ui::{ShellKey, ShellState};

    #[test]
    fn the_shell_cycle_maps_to_all_renderer_modes_without_changing_startup() {
        let mut shell = ShellState::new(82, 30);
        assert_eq!(
            scene_display_mode(shell.shading()),
            SceneDisplayMode::Wireframe
        );
        shell.key(ShellKey::Alt('v'));
        shell.key(ShellKey::Down);
        for expected in [
            SceneDisplayMode::HiddenLine,
            SceneDisplayMode::NoShading,
            SceneDisplayMode::Shaded,
            SceneDisplayMode::Default,
            SceneDisplayMode::Points,
            SceneDisplayMode::Line,
            SceneDisplayMode::Wireframe,
        ] {
            shell.key(ShellKey::Right);
            assert_eq!(scene_display_mode(shell.shading()), expected);
        }
    }
}

#[cfg(test)]
mod color_tests {
    use super::tests::{click, key, live_app};
    use super::*;
    use newell_ui::ShellKey;
    #[test]
    fn color_and_both_theme_directions_publish_visible_rows_without_document_mutation() {
        let mut app = live_app();
        app.finish_shell_input(Update::default());
        let events = app.document.subscribe();
        let revision = app.document.revision();
        let identity = app.document.identity();
        let shading = app.shell.as_ref().unwrap().shell().shading();
        click(&mut app, "View");
        for enabled in [false, true] {
            if enabled {
                click(&mut app, "Color");
            }
            for theme in [Theme::Light, Theme::Dark] {
                app.last_ui_publication = None;
                click(&mut app, "Theme");
                let shell = app.shell.as_ref().unwrap();
                assert_eq!(
                    ui_palette(shell.shell()),
                    newell_render::UiPalette::new(theme, enabled)
                );
                assert_eq!(app.last_ui_publication.as_ref(), Some(shell.grid()));
                let text: String = shell
                    .grid()
                    .cells()
                    .iter()
                    .map(|c| match c.content {
                        newell_ui::CellContent::Glyph(c) => c,
                        _ => ' ',
                    })
                    .collect();
                assert!(text.contains(if theme == Theme::Light {
                    "Theme [ ◐ Light ]"
                } else {
                    "Theme [ ◑ Dark  ]"
                }));
            }
        }
        for (input, enabled) in [
            (Some(ShellKey::Enter), false),
            (Some(ShellKey::Char(' ')), true),
        ] {
            // Select Color without activating it: the previous Theme row stays at index 3.
            key(&mut app, ShellKey::Down);
            if let Some(input) = input {
                key(&mut app, input);
            }
            assert_eq!(app.shell.as_ref().unwrap().shell().color_enabled(), enabled);
            assert_eq!(
                app.last_ui_publication.as_ref(),
                Some(app.shell.as_ref().unwrap().grid())
            );
            // Return focus to Theme so the next iteration selects Color again.
            key(&mut app, ShellKey::Up);
        }
        assert_eq!(app.document.identity(), identity);
        assert_eq!(app.document.revision(), revision);
        assert!(events.try_recv().is_err());
        assert_eq!(app.shell.as_ref().unwrap().shell().shading(), shading);
        assert!(app.shell.as_mut().unwrap().take_tree_intent().is_none());
    }

    #[test]
    fn color_survives_real_projection_refresh_retry_and_document_replacement() {
        let mut app = live_app();
        click(&mut app, "View");
        click(&mut app, "Color");
        key(&mut app, ShellKey::Escape);
        let palette = ui_palette(app.shell.as_ref().unwrap().shell());
        let base = app.document.add_base_solid("old").unwrap();
        app.projection_sync.fail_next_projection = 1;
        let now = Instant::now();
        let old = app.shell.as_ref().unwrap().grid().clone();
        assert!(
            !app.poll_document_projection(now, ProjectionTrigger::Notification)
                .grid
        );
        assert_eq!(app.shell.as_ref().unwrap().grid(), &old);
        assert!(app.next_document_retry_deadline().is_some());
        let update = app.poll_document_projection(
            now + Duration::from_millis(100),
            ProjectionTrigger::RetryDeadline,
        );
        assert!(update.grid);
        app.apply(update);
        app.document
            .rename_feature(base, "renamed │[+]☰".into())
            .unwrap();
        let entity = app.document.document().feature(base).unwrap().outputs()[0];
        app.document.set_entity_visibility(entity, false).unwrap();
        let update = app.poll_document_projection(
            now + Duration::from_secs(1),
            ProjectionTrigger::Notification,
        );
        app.apply(update);
        assert_eq!(ui_palette(app.shell.as_ref().unwrap().shell()), palette);
        assert_eq!(
            app.last_ui_publication.as_ref(),
            Some(app.shell.as_ref().unwrap().grid())
        );
        app.document
            .replace(newell_model::Document::default())
            .unwrap();
        let update = app.poll_document_projection(
            now + Duration::from_secs(2),
            ProjectionTrigger::Notification,
        );
        app.apply(update);
        assert_eq!(ui_palette(app.shell.as_ref().unwrap().shell()), palette);
        assert!(
            app.shell
                .as_ref()
                .unwrap()
                .shell()
                .tree_projection()
                .features
                .is_empty()
        );
    }
}
