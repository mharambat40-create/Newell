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

use crate::selection::{
    CommandSelectionInputs, SelectionOp, SelectionRequest, WindowSelection, candidate_rows,
    resolve_tree_selection,
};
use newell_kernel::NewellMesh;
#[cfg(test)]
use newell_kernel::NewellSolid;
#[cfg(test)]
use newell_model::CommandState;
use newell_model::{CommandController, EntityId, Selection};
use newell_render::{FrameOutcome, Renderer, SceneDisplayMode, SceneViewport, Theme};
#[cfg(test)]
use newell_ui::ExtrudeStatus;
use newell_ui::{
    ActionOperation, DEFAULT_SCALE_FACTOR_PERCENT, FeatureNodeId, GapDisplayLocation, GridMetrics,
    HistoryGap, OpaqueTreeId, ResizeSnapshot, ResizeZone, ScaleMode, ThemeChoice, TreeDocumentId,
    WindowAction, composed_grid_size, effective_scale, fit_window, k_fit, min_window_size,
};
use newell_ui::{
    CommandInputSessionId, EntitySelectionView, LocalSelectionView, OriginPlane,
    SelectionDestination, SelectionEdit, SelectionFieldCount, SelectionFieldDescriptor,
    SelectionOpen, SelectionOutcome, SelectionRow, SelectionSessionId, SelectionUiRequest,
    SketchIntent, SketchMode,
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
use crate::navigation_widgets::{NavInteraction, axis_hover, cube_hover};
use crate::scale::{MonitorInfo, Rescale, ScaleManager, plan_rescale, rect_in_window_space};
use crate::window::{CancelReason, ControlArm, ResizeManager, check_fit, screen_pointer};
use newell_render::navigation_widgets::NavSnapshot;
use newell_ui::navigation_widgets::{NavBasis, NavGeometry, PixelRect, layout_navigation};
use std::sync::mpsc::Receiver;

#[derive(Debug)]
pub(crate) enum AppEvent {
    GeometryReady,
}
#[derive(Debug, Clone, Copy)]
struct PublishedSceneBinding {
    range: newell_kernel::NewellMeshRange,
    binding: crate::geometry_worker::SceneItemBinding,
}
#[derive(Debug)]
struct ScenePublication {
    snapshot: crate::geometry_worker::SceneSnapshot,
    mesh: NewellMesh,
    bindings: Vec<PublishedSceneBinding>,
    gpu: Option<newell_render::PreparedScene>,
}
#[derive(Debug)]
struct PendingScenePublication {
    snapshot: crate::geometry_worker::SceneSnapshot,
    version: crate::DocumentVersion,
    staged: Option<ScenePublication>,
    preview: Option<Box<crate::geometry_worker::PreparedGeometry>>,
    attempts: u8,
    retry_at: Option<Instant>,
}
const TITLE: &str = "Newell";

#[derive(Debug, Clone, Copy)]
struct SceneEntityBinding {
    document: crate::DocumentInstanceId,
    mesh_revision: u64,
    entity: EntityId,
}
#[derive(Debug, Clone, Copy)]
struct PendingSceneClick {
    document: crate::DocumentInstanceId,
    generation: newell_ui::ProjectionGeneration,
    mesh_revision: u64,
    edit: SelectionEdit,
}
#[derive(Debug, Clone, Copy)]
struct PendingOriginClick {
    document: crate::DocumentInstanceId,
    generation: newell_ui::ProjectionGeneration,
    mesh_revision: u64,
    navigation_revision: u64,
    reference: newell_ui::OriginReference,
    start: (f64, f64),
    display_scale: f64,
    edit: SelectionEdit,
}
#[derive(Clone, Copy)]
struct SketchSession {
    document: crate::DocumentInstanceId,
    command: CommandInputSessionId,
    mode: SketchMode,
}

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

#[cfg(test)]
#[derive(Debug, Default)]
struct ExtrudeCommitLatch {
    armed: bool,
    insertion: Option<(TreeDocumentId, HistoryGap)>,
}

#[derive(Debug, Clone)]
enum NumericDeliveryKind {
    FormulaApply(
        newell_ui::FormulaApplyRequest,
        newell_ui::NumericProjection,
        Option<String>,
    ),
    View(newell_ui::NumericProjection),
    Formula(
        newell_ui::NumericFormulaOpenRequest,
        newell_ui::NumericProjection,
    ),
    Confirm(
        newell_ui::NumericConfirmRequest,
        bool,
        Option<newell_model::FeatureId>,
        newell_ui::NumericProjection,
    ),
}
#[derive(Debug, Clone, Copy)]
struct ExtrudeEditTarget {
    version: crate::DocumentVersion,
    feature: newell_model::FeatureId,
}
#[derive(Debug)]
struct PendingPropertiesEditor {
    certificate: crate::controller::CertifiedPropertiesOpen,
    target: ExtrudeEditTarget,
    values: newell_ui::ExtrudeValues,
    numeric: crate::parameters::CommandNumericInputs,
    selections: CommandSelectionInputs,
}
#[derive(Debug, Clone)]
struct NumericDelivery {
    kind: NumericDeliveryKind,
    retry_at: Option<Instant>,
    attempts: u8,
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
    #[cfg(test)]
    extrude_latch: ExtrudeCommitLatch,
    committed_cursor: Option<(TreeDocumentId, OpaqueTreeId)>,
    layout_retry_rearm_pending: bool,
    #[cfg(test)]
    last_ui_publication: Option<newell_ui::CellGrid>,
    selection: WindowSelection,
    origin_selection: Vec<newell_ui::OriginReference>,
    face_selection: Vec<newell_model::geometry::FaceRef>,
    command_inputs: Option<CommandSelectionInputs>,
    numeric_inputs: Option<crate::parameters::CommandNumericInputs>,
    numeric_cache:
        std::collections::BTreeMap<ActionOperation, crate::parameters::CommandNumericInputs>,
    numeric_delivery: Option<NumericDelivery>,
    numeric_last_attempt: u64,
    extrude_edit_target: Option<ExtrudeEditTarget>,
    properties_editor: Option<PendingPropertiesEditor>,
    properties_close_source: Option<crate::DocumentVersion>,
    properties_retry_at: Option<Instant>,
    properties_attempts: u8,
    last_properties_id: u64,
    numeric_committed: bool,
    numeric_preview_suspended: bool,
    active_selection: Option<SelectionOpen>,
    next_selection_id: u64,
    next_command_id: u64,
    selection_view_pending: bool,
    selection_retry_at: Option<Instant>,
    selection_attempts: u8,
    selection_suspended: bool,
    #[cfg(test)]
    fail_next_selection: u8,
    commands: CommandController,
    /// Entity represented by the initial viewport solid.
    scene_entity: Option<SceneEntityBinding>,
    geometry: Option<crate::extrude_session::GeometryRuntime>,
    geometry_wake: Option<Arc<dyn Fn() + Send + Sync>>,
    pending_scene: Option<PendingScenePublication>,
    #[cfg(test)]
    fail_scene_publish: u8,
    scene_bindings: Vec<PublishedSceneBinding>,
    scene_available: bool,
    scene_initialized: bool,
    pending_scene_click: Option<PendingSceneClick>,
    pending_origin_click: Option<PendingOriginClick>,
    sketch: Option<SketchSession>,
    sketch_edit: Option<crate::sketch::Edit>,
    scene_selected: bool,
    modifiers: ModifiersState,
    navigation: Navigation,
    nav_geometry: Option<NavGeometry>,
    nav_hover: Option<newell_ui::navigation_widgets::AxisCap>,
    nav_cube_hover: Option<newell_ui::navigation_widgets::ViewDirection>,
    nav_interaction: NavInteraction,
    nav_generation: u64,
    nav_ui_masks: Vec<PixelRect>,
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
    pub(crate) fn enable_geometry_worker(
        &mut self,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<(), AppError> {
        let epoch = self
            .geometry
            .as_ref()
            .map_or(Some(1), |runtime| runtime.epoch.checked_add(1))
            .ok_or(AppError::DocumentCounterExhausted)?;
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(wake);
        let completion = Arc::clone(&wake);
        let worker = crate::geometry_worker::GeometryWorker::start(epoch, move || completion())?;
        self.geometry_wake = Some(wake);
        self.document.bind_geometry_worker(epoch)?;
        self.geometry = Some(crate::extrude_session::GeometryRuntime::new(worker, epoch));
        Ok(())
    }
    fn geometry_status(&mut self, message: Option<String>) -> Update {
        match self
            .shell
            .as_mut()
            .map(|shell| shell.set_contextual_status(message))
        {
            Some(Ok(update)) => update,
            Some(Err(error)) => {
                log(&error.to_string());
                Update::default()
            }
            None => Update::default(),
        }
    }
    fn scene_unavailable(&mut self) {
        self.scene_available = false;
        self.pending_scene_click = None;
        self.navigation.cancel();
        self.nav_interaction.cancel();
        if let Some(renderer) = &mut self.renderer {
            renderer.set_scene_available(false);
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
    fn release_geometry(&self, token: crate::geometry_worker::NewellGeometryToken) {
        if self.document.geometry_token() == Some(token) {
            return;
        }
        if let Some(runtime) = &self.geometry
            && let Err(error) = runtime.worker.release(token)
        {
            log(&error.to_string());
        }
    }
    fn drop_pending_preview_scene(&mut self) {
        if self
            .pending_scene
            .as_ref()
            .is_some_and(|pending| pending.preview.is_some())
            && let Some(pending) = self.pending_scene.take()
            && let Some(ready) = pending.preview
        {
            self.release_geometry(ready.token);
        }
    }
    fn begin_geometry_command(&mut self) {
        self.drop_pending_preview_scene();
        let (Some(runtime), Some(numeric), Some(selection)) = (
            &mut self.geometry,
            &self.numeric_inputs,
            &self.command_inputs,
        ) else {
            return;
        };
        if let Some(mut old) = runtime.coordinator.take()
            && let Some(token) = old.cancel()
        {
            let _ = runtime.worker.release(token);
        }
        let insertion = match selection.insertion {
            HistoryGap::End => None,
            HistoryGap::Before(id) => self.document.feature_id(id),
        };
        runtime.coordinator = Some(crate::extrude_session::ExtrudeGeometrySession::new(
            self.document.version(),
            numeric.session,
            runtime.epoch,
            self.document.document().geometry_generation(),
            insertion,
            numeric.parameter_draft.registry().epoch(),
        ));
        runtime.latest_edit = None;
    }
    fn queue_geometry_bootstrap(&mut self) -> Result<(), AppError> {
        use crate::geometry_worker::{GeometryJobId, PrepareGeometry};
        let Some(runtime) = &mut self.geometry else {
            return Err(AppError::GeometryTransport(
                "geometry worker unavailable".into(),
            ));
        };
        if runtime.bootstrap.is_some() {
            return Ok(());
        }
        let parameters = self
            .document
            .document()
            .begin_parameter_draft(newell_model::parameters::ParameterDraftOwner::Document)?;
        let serial = runtime
            .bootstrap_serial
            .checked_add(1)
            .ok_or(AppError::DocumentCounterExhausted)?;
        let id = GeometryJobId {
            worker_epoch: runtime.epoch,
            document: self.document.identity(),
            session: CommandInputSessionId(0),
            base_revision: self.document.revision(),
            draft_revision: 0,
            serial,
            parameter_epoch: parameters.registry().epoch(),
            source_generation: self.document.document().geometry_generation(),
        };
        runtime.worker.submit(PrepareGeometry {
            #[cfg(test)]
            kernel_probe: None,
            id,
            base: self.document.document().clone(),
            edit: newell_model::geometry::DocumentEdit::ParameterDefinitions,
            insertion: None,
            parameters,
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })?;
        runtime.bootstrap = Some(id);
        runtime.bootstrap_serial = serial;
        Ok(())
    }
    fn prepare_scene_publication(
        &self,
        snapshot: &crate::geometry_worker::SceneSnapshot,
    ) -> Result<ScenePublication, AppError> {
        use crate::geometry_worker::SceneItemBinding;
        if snapshot.document != self.document.identity() {
            return Err(AppError::InvalidDocumentProjection);
        }
        let visible: Vec<_> = snapshot
            .items
            .iter()
            .filter(|item| {
                self.document
                    .document()
                    .entity(item.entity)
                    .map_or(item.visible, |entity| entity.visible())
            })
            .collect();
        let references: Vec<_> = visible.iter().map(|item| &item.mesh).collect();
        let (mesh, ranges) = NewellMesh::compose(&references)?;
        let bindings = visible
            .iter()
            .zip(ranges)
            .map(|(item, range)| PublishedSceneBinding {
                range,
                binding: item.binding,
            })
            .collect();
        let gpu = if let Some(renderer) = &self.renderer {
            let items = visible
                .iter()
                .map(|item| {
                    let selected_faces: Vec<u32> = match item.binding {
                        SceneItemBinding::Committed { entity, generation } => self
                            .face_selection
                            .iter()
                            .filter(|face| face.entity == entity && face.generation == generation)
                            .map(|face| face.face)
                            .collect(),
                        SceneItemBinding::Preview { .. } => Vec::new(),
                    };
                    newell_render::SceneMeshItem {
                        mesh: item.mesh.clone(),
                        selected: matches!(item.binding, SceneItemBinding::Committed{entity,..} if self.selection.entities().contains(&entity)) && selected_faces.is_empty(),
                        selected_faces,
                    }
                })
                .collect();
            Some(
                renderer.prepare_scene_items_for_camera(
                    items,
                    self.navigation
                        .snapshot_for_mesh(&mesh, !self.scene_initialized)
                        .ok_or(AppError::InvalidDocumentProjection)?,
                )?,
            )
        } else {
            None
        };
        Ok(ScenePublication {
            snapshot: snapshot.clone(),
            mesh,
            bindings,
            gpu,
        })
    }
    fn queue_scene_publication(
        &mut self,
        snapshot: crate::geometry_worker::SceneSnapshot,
        preview: Option<Box<crate::geometry_worker::PreparedGeometry>>,
    ) {
        if let Some(old) = self.pending_scene.take()
            && let Some(ready) = old.preview
        {
            self.release_geometry(ready.token);
        }
        self.scene_unavailable();
        self.pending_scene = Some(PendingScenePublication {
            snapshot,
            version: self.document.version(),
            staged: None,
            preview,
            attempts: 0,
            retry_at: None,
        });
    }
    fn restore_committed_scene(&mut self) {
        let scene = self
            .geometry
            .as_ref()
            .and_then(|runtime| runtime.committed_scene.clone());
        if let Some(scene) = scene {
            self.queue_scene_publication(scene, None);
        }
    }
    fn republish_scene_visibility(&mut self) {
        let scene = self.geometry.as_ref().and_then(|runtime| {
            runtime
                .displayed_scene
                .clone()
                .or_else(|| runtime.committed_scene.clone())
        });
        if let Some(scene) = scene {
            self.queue_scene_publication(scene, None);
        }
    }
    fn stage_pending_scene(&mut self, now: Instant, rearm: bool) -> bool {
        let Some(mut pending) = self.pending_scene.take() else {
            return true;
        };
        if pending.snapshot.document != self.document.identity() {
            if let Some(ready) = pending.preview {
                self.release_geometry(ready.token);
            }
            return true;
        }
        if pending.version != self.document.version() {
            if let Some(ready) = pending.preview.take() {
                self.release_geometry(ready.token);
                self.restore_committed_scene();
                return false;
            }
            if pending.snapshot.generation != self.document.document().geometry_generation() {
                self.restore_committed_scene();
                return false;
            }
            pending.version = self.document.version();
            pending.staged = None;
        }
        if pending.staged.is_none() {
            if pending.attempts > 0 {
                if pending.retry_at.is_some_and(|at| now < at)
                    || (pending.retry_at.is_none() && !rearm)
                {
                    self.pending_scene = Some(pending);
                    return false;
                }
                if rearm && pending.retry_at.is_none() {
                    pending.attempts = 0;
                }
            }
            match self.prepare_scene_publication(&pending.snapshot) {
                Ok(publication) => pending.staged = Some(publication),
                Err(error) => {
                    log(&error.to_string());
                    pending.attempts = pending.attempts.saturating_add(1);
                    pending.retry_at = if pending.attempts == 1 {
                        Some(now + Duration::from_millis(100))
                    } else {
                        None
                    };
                    self.pending_scene = Some(pending);
                    return false;
                }
            }
        }
        self.pending_scene = Some(pending);
        true
    }
    fn publish_scene_gpu(&mut self, publication: &mut ScenePublication) -> Result<(), AppError> {
        #[cfg(test)]
        if self.fail_scene_publish > 0 {
            self.fail_scene_publish -= 1;
            return Err(AppError::InvalidDocumentProjection);
        }
        if let Some(renderer) = &mut self.renderer
            && let Some(gpu) = publication.gpu.take()
        {
            if !renderer.owns_prepared_scene(&gpu) {
                return Err(AppError::InvalidDocumentProjection);
            }
            renderer.publish_prepared_scene(gpu)?;
        }
        Ok(())
    }
    fn publish_pending_scene(&mut self, now: Instant) -> Update {
        if self.projection_sync.acknowledged_version() != Some(self.document.version()) {
            return Update::default();
        }
        let Some(mut pending) = self.pending_scene.take() else {
            return Update::default();
        };
        let Some(mut publication) = pending.staged.take() else {
            self.pending_scene = Some(pending);
            return Update::default();
        };
        let clear_diagnostic = pending.preview.is_some() || self.numeric_committed;
        if pending.version != self.document.version() {
            pending.staged = None;
            self.pending_scene = Some(pending);
            return Update::default();
        }
        if let Err(error) = self.publish_scene_gpu(&mut publication) {
            log(&error.to_string());
            pending.staged = None;
            pending.attempts = pending.attempts.saturating_add(1);
            pending.retry_at = (pending.attempts == 1).then_some(now + Duration::from_millis(100));
            self.pending_scene = Some(pending);
            return Update::default();
        }
        self.pending_scene_click = None;
        self.navigation.set_mesh(&publication.mesh);
        self.nav_interaction.cancel();
        if !self.scene_initialized {
            self.navigation.center_scene();
            self.scene_initialized = true;
        }
        self.scene_bindings = publication.bindings;
        self.scene_entity = None;
        self.scene_available = true;
        if let Some(runtime) = &mut self.geometry {
            runtime.displayed_scene = Some(publication.snapshot);
        }
        if let Some(ready) = pending.preview {
            let current = self.document.version();
            let result = self
                .geometry
                .as_mut()
                .and_then(|runtime| runtime.coordinator.as_mut())
                .map(|coordinator| coordinator.retain_preview(ready, current));
            match result {
                Some(Ok(Some(old))) => self.release_geometry(old),
                Some(Err(error)) => log(&error.to_string()),
                _ => {}
            }
        }
        if let Some(renderer) = &mut self.renderer {
            renderer.set_scene_available(true);
            if let Some(camera) = self.navigation.snapshot() {
                renderer.set_scene_camera(camera);
            }
        }
        self.update_scene_selection();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        if clear_diagnostic {
            self.geometry_status(None)
        } else {
            Update { grid: false }
        }
    }
    fn observe_geometry_lifetime(&mut self) {
        if self.geometry.is_none() || self.numeric_committed {
            return;
        }
        let changed = self.numeric_inputs.as_ref().is_some_and(|inputs| {
            self.shell.as_ref().is_none_or(|shell| {
                !shell.shell().command_open() || shell.shell().operation() != inputs.operation
            })
        });
        if !changed {
            return;
        }
        self.drop_pending_preview_scene();
        if let Some(runtime) = &mut self.geometry {
            if let Some(mut coordinator) = runtime.coordinator.take()
                && let Some(token) = coordinator.cancel()
            {
                let _ = runtime.worker.release(token);
            }
            runtime.latest_edit = None;
        }
        let preview = self
            .geometry
            .as_ref()
            .and_then(|runtime| runtime.displayed_scene.as_ref())
            .is_some_and(|scene| {
                scene.generation != self.document.document().geometry_generation()
            });
        if preview {
            self.restore_committed_scene();
        }
        self.commands.cancel();
        if self
            .shell
            .as_ref()
            .is_none_or(|shell| !shell.shell().command_open())
        {
            if let Some(inputs) = self.numeric_inputs.take() {
                self.numeric_cache.remove(&inputs.operation);
            }
            self.command_inputs = None;
            self.active_selection = None;
            self.numeric_delivery = None;
            self.extrude_edit_target = None;
        }
    }
    fn geometry_edit(&self) -> Result<newell_model::geometry::DocumentEdit, AppError> {
        let numeric = self
            .numeric_inputs
            .as_ref()
            .ok_or(AppError::InvalidNumericRequest)?;
        let selections = self
            .command_inputs
            .as_ref()
            .ok_or(AppError::InvalidNumericRequest)?;
        if numeric.operation != ActionOperation::Extrude {
            return Ok(newell_model::geometry::DocumentEdit::ParameterDefinitions);
        }
        let values = &self
            .shell
            .as_ref()
            .ok_or(AppError::InvalidNumericRequest)?
            .shell()
            .extrude()
            .values;
        let spec = crate::parameters::build_extrude_spec(values, numeric, selections)?;
        if spec.profile.is_none() {
            return Err(newell_model::ModelError::InvalidGeometryReference.into());
        }
        let retained = selections
            .values
            .get(&newell_ui::SelectionFieldKey {
                operation: ActionOperation::Extrude,
                key: 27,
            })
            .map_or_else(Vec::new, |value| value.entities().to_vec());
        let validation = newell_model::geometry::ExtrudeValidationRefs {
            retained_merge_scope: retained,
        };
        if let Some(target) = self.extrude_edit_target {
            if target.version != self.document.version() {
                return Err(AppError::InvalidNumericRequest);
            }
            Ok(newell_model::geometry::DocumentEdit::ExtrudeReplace {
                feature: target.feature,
                spec,
                validation,
            })
        } else {
            Ok(newell_model::geometry::DocumentEdit::ExtrudeInsert { spec, validation })
        }
    }
    fn invalidate_geometry_preview(&mut self) {
        let token = self
            .geometry
            .as_mut()
            .and_then(|runtime| runtime.coordinator.as_mut())
            .and_then(|coordinator| {
                if coordinator.confirm_pending() {
                    None
                } else {
                    coordinator.invalidate_preview()
                }
            });
        if let Some(token) = token {
            self.release_geometry(token);
        }
        self.drop_pending_preview_scene();
        if self
            .geometry
            .as_ref()
            .and_then(|runtime| runtime.displayed_scene.as_ref())
            .is_some_and(|scene| scene.generation != self.document.document().geometry_generation())
        {
            self.restore_committed_scene();
        }
    }
    fn sync_extrude_command(&mut self) {
        if self.geometry.is_some() {
            self.sync_geometry_preview();
        } else {
            #[cfg(test)]
            self.sync_legacy_extrude_command();
        }
    }
    fn sync_geometry_preview(&mut self) {
        if self.numeric_committed {
            return;
        }
        let Some(shell) = &self.shell else {
            return;
        };
        if shell.shell().operation() != ActionOperation::Extrude || !shell.shell().command_open() {
            return;
        }
        if self.command_inputs.is_none() {
            let update = self.start_command_inputs(ActionOperation::Extrude);
            self.apply(update);
        }
        let Some(shell) = &self.shell else {
            return;
        };
        if self.numeric_preview_suspended
            || shell.shell().numeric_editing()
            || shell.shell().formula().is_open()
        {
            let token = self
                .geometry
                .as_mut()
                .and_then(|runtime| runtime.coordinator.as_mut())
                .and_then(|coordinator| {
                    if coordinator.confirm_pending() {
                        None
                    } else {
                        coordinator.invalidate_preview()
                    }
                });
            if let Some(token) = token {
                self.release_geometry(token);
            }
            self.drop_pending_preview_scene();
            if self
                .geometry
                .as_ref()
                .and_then(|runtime| runtime.displayed_scene.as_ref())
                .is_some_and(|scene| {
                    scene.generation != self.document.document().geometry_generation()
                })
            {
                self.restore_committed_scene();
            }
            return;
        }
        if self
            .geometry
            .as_ref()
            .and_then(|runtime| runtime.coordinator.as_ref())
            .is_some_and(|coordinator| coordinator.confirm_pending())
        {
            return;
        }
        let edit = match self.geometry_edit() {
            Ok(edit) => edit,
            Err(error) => {
                self.invalidate_geometry_preview();
                let update = self.geometry_status(Some(error.to_string()));
                self.apply(update);
                return;
            }
        };
        let Some(numeric) = &self.numeric_inputs else {
            return;
        };
        let epoch = numeric.parameter_draft.registry().epoch();
        let changed = self
            .geometry
            .as_ref()
            .is_some_and(|runtime| runtime.latest_edit.as_ref() != Some(&(edit.clone(), epoch)));
        if !changed
            && (self
                .geometry
                .as_ref()
                .and_then(|runtime| runtime.coordinator.as_ref())
                .is_some_and(|coordinator| coordinator.has_work())
                || self
                    .pending_scene
                    .as_ref()
                    .is_some_and(|pending| pending.preview.is_some()))
        {
            return;
        }
        if changed {
            self.drop_pending_preview_scene();
            if let Some(runtime) = &mut self.geometry {
                if runtime.latest_edit.is_some()
                    && let Some(coordinator) = &mut runtime.coordinator
                {
                    match coordinator.advance_draft(epoch) {
                        Ok(Some(token)) => {
                            let _ = runtime.worker.release(token);
                        }
                        Ok(None) => {}
                        Err(error) => {
                            log(&error.to_string());
                            return;
                        }
                    }
                } else if let Some(coordinator) = &mut runtime.coordinator {
                    let _ = coordinator.advance_draft(epoch);
                }
                runtime.latest_edit = Some((edit.clone(), epoch));
            }
            if self
                .geometry
                .as_ref()
                .and_then(|runtime| runtime.displayed_scene.as_ref())
                .is_some_and(|scene| {
                    scene.generation != self.document.document().geometry_generation()
                })
            {
                self.restore_committed_scene();
            }
        }
        let result = (|| -> Result<(), AppError> {
            let numeric = self
                .numeric_inputs
                .as_ref()
                .ok_or(AppError::InvalidNumericRequest)?;
            let runtime = self
                .geometry
                .as_mut()
                .ok_or(AppError::InvalidNumericRequest)?;
            let coordinator = runtime
                .coordinator
                .as_mut()
                .ok_or(AppError::InvalidNumericRequest)?;
            let job = coordinator.freeze(
                self.document.version(),
                self.document.document(),
                edit,
                &numeric.parameter_draft,
                crate::extrude_session::PreparationPurpose::Preview,
            )?;
            runtime.worker.submit(job)
        })();
        if let Err(error) = result {
            let update = self.geometry_status(Some(error.to_string()));
            self.apply(update);
        }
    }
    fn complete_numeric_confirm(
        &mut self,
        request: newell_ui::NumericConfirmRequest,
        rejected: bool,
    ) -> Update {
        #[cfg(test)]
        if self.geometry.is_none() {
            return self.complete_legacy_numeric_confirm(request, rejected);
        }
        if self.geometry.is_none() {
            return Update::default();
        }
        if self.shell.as_ref().is_none_or(|shell| {
            !shell.shell().numeric_confirm_pending(request)
                || shell.shell().numeric_session()
                    != Some((request.document, request.session, request.operation))
        }) {
            return Update::default();
        }
        if !self.numeric_committed
            && self
                .geometry
                .as_ref()
                .is_some_and(|runtime| !runtime.worker.is_alive())
        {
            let result = (|| -> Result<(), AppError> {
                let wake = self
                    .geometry_wake
                    .clone()
                    .ok_or(AppError::InvalidNumericRequest)?;
                let committed = self
                    .geometry
                    .as_ref()
                    .and_then(|runtime| runtime.committed_scene.clone());
                let displayed = self
                    .geometry
                    .as_ref()
                    .and_then(|runtime| runtime.displayed_scene.clone());
                self.drop_pending_preview_scene();
                self.enable_geometry_worker(move || wake())?;
                if let Some(runtime) = &mut self.geometry {
                    runtime.committed_scene = committed;
                    runtime.displayed_scene = displayed;
                }
                self.begin_geometry_command();
                Ok(())
            })();
            if let Err(error) = result {
                return self.geometry_status(Some(error.to_string()));
            }
        }
        let Some(inputs) = self.numeric_inputs.as_ref().filter(|inputs| {
            inputs.document == self.document.identity()
                && inputs.session == request.session
                && inputs.operation == request.operation
                && request.attempt.0 > self.numeric_last_attempt
        }) else {
            return Update::default();
        };
        self.numeric_last_attempt = request.attempt.0;
        let projection = match inputs.project() {
            Ok(projection) => projection,
            Err(error) => {
                log(&error.to_string());
                return Update::default();
            }
        };
        if rejected || inputs.consumed_sequence < request.through_sequence {
            return self.queue_numeric_delivery(NumericDeliveryKind::Confirm(
                request, false, None, projection,
            ));
        }
        if request.operation != ActionOperation::Extrude {
            match inputs
                .parameter_draft
                .prepared_registry(&std::collections::BTreeSet::new())
            {
                Ok(registry) if &registry == self.document.document().parameters() => {
                    return self.queue_numeric_delivery(NumericDeliveryKind::Confirm(
                        request, true, None, projection,
                    ));
                }
                Err(error) => {
                    let update = self.geometry_status(Some(error.to_string()));
                    return update.merge(self.queue_numeric_delivery(
                        NumericDeliveryKind::Confirm(request, false, None, projection),
                    ));
                }
                _ => {}
            }
        }
        let edit = match self.geometry_edit() {
            Ok(edit) => edit,
            Err(error) => {
                let update = self.geometry_status(Some(error.to_string()));
                return update.merge(self.queue_numeric_delivery(NumericDeliveryKind::Confirm(
                    request, false, None, projection,
                )));
            }
        };
        let epoch = self
            .numeric_inputs
            .as_ref()
            .map_or(0, |inputs| inputs.parameter_draft.registry().epoch());
        let same = self
            .geometry
            .as_ref()
            .is_some_and(|runtime| runtime.latest_edit.as_ref() == Some(&(edit.clone(), epoch)));
        if same
            && let Some(ready) = self
                .geometry
                .as_mut()
                .and_then(|runtime| runtime.coordinator.as_mut())
                .and_then(|coordinator| coordinator.take_ready(self.document.version()))
        {
            return self.accept_geometry_confirmation(request, ready);
        }
        self.drop_pending_preview_scene();
        let result = (|| -> Result<(), AppError> {
            let runtime = self
                .geometry
                .as_mut()
                .ok_or(AppError::InvalidNumericRequest)?;
            let coordinator = runtime
                .coordinator
                .as_mut()
                .ok_or(AppError::InvalidNumericRequest)?;
            if !same {
                if let Some(token) = coordinator.advance_draft(epoch)? {
                    let _ = runtime.worker.release(token);
                }
                runtime.latest_edit = Some((edit.clone(), epoch));
            }
            let inputs = self
                .numeric_inputs
                .as_ref()
                .ok_or(AppError::InvalidNumericRequest)?;
            let job = coordinator.freeze(
                self.document.version(),
                self.document.document(),
                edit,
                &inputs.parameter_draft,
                crate::extrude_session::PreparationPurpose::Confirm(request),
            )?;
            runtime.worker.submit(job)
        })();
        if let Err(error) = result {
            if let Some(coordinator) = self
                .geometry
                .as_mut()
                .and_then(|runtime| runtime.coordinator.as_mut())
            {
                coordinator.invalidate_preview();
            }
            let update = self.geometry_status(Some(error.to_string()));
            return update.merge(self.queue_numeric_delivery(NumericDeliveryKind::Confirm(
                request, false, None, projection,
            )));
        }
        Update::default()
    }
    fn accept_geometry_confirmation(
        &mut self,
        request: newell_ui::NumericConfirmRequest,
        ready: Box<crate::geometry_worker::PreparedGeometry>,
    ) -> Update {
        use crate::geometry_worker::{PreparedGeometry, SceneItemBinding};
        let PreparedGeometry {
            id,
            prepared,
            token,
            mut scene,
        } = *ready;
        let expected = crate::DocumentVersion {
            instance: id.document,
            revision: id.base_revision,
        };
        let Some(inputs) = self.numeric_inputs.as_ref().filter(|inputs| {
            inputs.session == request.session
                && inputs.document == self.document.identity()
                && inputs.operation == request.operation
        }) else {
            self.release_geometry(token);
            return Update::default();
        };
        let projection = match inputs.project() {
            Ok(projection) => projection,
            Err(error) => {
                self.release_geometry(token);
                log(&error.to_string());
                return Update::default();
            }
        };
        let old_token = self.document.geometry_token();
        let receipt = match self.document.accept_prepared(expected, prepared, token) {
            Ok(receipt) => receipt,
            Err(error) => {
                self.release_geometry(token);
                let update = self.geometry_status(Some(error.to_string()));
                return update.merge(self.queue_numeric_delivery(NumericDeliveryKind::Confirm(
                    request, false, None, projection,
                )));
            }
        };
        self.numeric_committed = true;
        self.commands.cancel();
        #[cfg(test)]
        {
            self.extrude_latch = ExtrudeCommitLatch::default();
        }
        self.scene_unavailable();
        if let Some(feature) = receipt.feature {
            self.committed_cursor = Some((request.document, OpaqueTreeId::new(feature.get())));
        }
        self.selection.reconcile(&self.document);
        self.reconcile_face_selection();
        if !receipt.outputs.is_empty() {
            self.selection.apply(
                SelectionRequest {
                    document: self.document.identity(),
                    op: SelectionOp::Add,
                    entities: receipt.outputs.clone(),
                },
                &self.document,
            );
        }
        for item in &mut scene.items {
            item.binding = SceneItemBinding::Committed {
                entity: item.entity,
                generation: item.generation,
            };
        }
        if let Some(runtime) = &mut self.geometry {
            runtime.committed_scene = Some(scene.clone());
            if let Err(error) = runtime.worker.retain_committed(token) {
                log(&error.to_string());
            }
            if let Some(old) = old_token
                && old != token
                && let Err(error) = runtime.worker.release(old)
            {
                log(&error.to_string());
            }
        }
        self.queue_scene_publication(scene, None);
        self.numeric_delivery = Some(NumericDelivery {
            kind: NumericDeliveryKind::Confirm(request, true, receipt.feature, projection),
            retry_at: None,
            attempts: 0,
        });
        self.poll_document_projection(Instant::now(), ProjectionTrigger::Notification)
    }
    fn drain_sketch_solve(&mut self) -> Update {
        let Some(runtime) = &self.geometry else {
            return Update::default();
        };
        let reply = match runtime.worker.drain_sketch() {
            Ok(Some(reply)) => reply,
            Ok(None) => return Update::default(),
            Err(error) => {
                if let Some(edit) = &mut self.sketch_edit {
                    edit.cancel_solve();
                }
                return self
                    .publish_sketch_ready()
                    .merge(self.geometry_status(Some(error.to_string())));
            }
        };
        let Some(edit) = &mut self.sketch_edit else {
            return Update::default();
        };
        if !edit
            .solve
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, &reply.cancelled))
        {
            return Update::default();
        }
        edit.solve = None;
        let pending_draw = edit.pending_draw.take();
        let message = if edit.source != self.document.version() {
            Some("Sketch source changed during solve".into())
        } else {
            match reply.result {
                Ok((recipe, report)) => {
                    if let Some(key) = pending_draw {
                        edit.install_draw(recipe, key);
                        return self
                            .publish_sketch_ready()
                            .merge(self.geometry_status(None));
                    }
                    edit.recipe = recipe;
                    edit.set_tool(crate::sketch::Tool::Select);
                    if report.joined {
                        Some("Joined Sketch solved (rank unavailable)".into())
                    } else {
                        (report.rank > 0).then(|| {
                            format!(
                                "Segment equations: rank {}, {} DOF, {} redundant",
                                report.rank, report.degrees_of_freedom, report.redundant_equations,
                            )
                        })
                    }
                }
                Err(error) => Some(error.to_string()),
            }
        };
        self.publish_sketch_ready()
            .merge(self.geometry_status(message))
    }

    fn drain_worker_reply(&mut self) -> Update {
        use crate::extrude_session::PreparationEvent;
        use crate::geometry_worker::{SceneItemBinding, WorkerReply};
        let reply = match self.geometry.as_ref().map(|runtime| runtime.worker.drain()) {
            Some(Ok(Some(reply))) => reply,
            Some(Err(error)) => {
                let request = self
                    .geometry
                    .as_mut()
                    .and_then(|runtime| runtime.coordinator.as_mut())
                    .and_then(|coordinator| coordinator.transport_failed());
                self.drop_pending_preview_scene();
                self.restore_committed_scene();
                let update = self.geometry_status(Some(error.to_string()));
                if let Some(request) = request
                    && let Some(inputs) = &self.numeric_inputs
                    && let Ok(projection) = inputs.project()
                {
                    return update.merge(self.queue_numeric_delivery(
                        NumericDeliveryKind::Confirm(request, false, None, projection),
                    ));
                }
                return update;
            }
            _ => return Update::default(),
        };
        let id = match &reply {
            WorkerReply::Ready(ready) => ready.id,
            WorkerReply::Failed { id, .. } => *id,
        };
        let bootstrap = self
            .geometry
            .as_ref()
            .is_some_and(|runtime| runtime.bootstrap == Some(id));
        if bootstrap {
            if let Some(runtime) = &mut self.geometry {
                runtime.bootstrap = None;
            }
            if id.document != self.document.identity()
                || id.base_revision != self.document.revision()
            {
                if let WorkerReply::Ready(ready) = reply {
                    self.release_geometry(ready.token);
                }
                if let Err(error) = self.queue_geometry_bootstrap() {
                    log(&error.to_string());
                }
                return Update::default();
            }
            match reply {
                WorkerReply::Failed { error, .. } => {
                    return self.geometry_status(Some(error.to_string()));
                }
                WorkerReply::Ready(ready) => {
                    let crate::geometry_worker::PreparedGeometry {
                        id,
                        prepared,
                        token,
                        mut scene,
                    } = *ready;
                    let expected = crate::DocumentVersion {
                        instance: id.document,
                        revision: id.base_revision,
                    };
                    if let Err(error) = self.document.accept_prepared(expected, prepared, token) {
                        self.release_geometry(token);
                        return self.geometry_status(Some(error.to_string()));
                    }
                    // Bootstrap republishes derived B-Rep data for this same
                    // revision. A Sketch opened while it ran keeps its history anchor.
                    if let Some(edit) = &mut self.sketch_edit
                        && edit.source == expected
                    {
                        edit.source = self.document.version();
                    }
                    for item in &mut scene.items {
                        item.binding = SceneItemBinding::Committed {
                            entity: item.entity,
                            generation: item.generation,
                        };
                    }
                    if let Some(runtime) = &mut self.geometry {
                        runtime.committed_scene = Some(scene.clone());
                        if let Err(error) = runtime.worker.retain_committed(token) {
                            log(&error.to_string());
                        }
                    }
                    self.queue_scene_publication(scene, None);
                    return self
                        .poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
                }
            }
        }
        let current = self.document.version();
        let event = if let Some(coordinator) = self
            .geometry
            .as_mut()
            .and_then(|runtime| runtime.coordinator.as_mut())
        {
            coordinator.receive(reply, current)
        } else {
            match reply {
                WorkerReply::Ready(ready) => PreparationEvent::Release(ready.token),
                _ => PreparationEvent::Ignored,
            }
        };
        match event {
            PreparationEvent::Preview(ready) => {
                self.queue_scene_publication(ready.scene.clone(), Some(ready));
                self.poll_document_projection(Instant::now(), ProjectionTrigger::Notification)
            }
            PreparationEvent::Confirm { request, ready } => {
                self.accept_geometry_confirmation(request, ready)
            }
            PreparationEvent::Rejected {
                request,
                diagnostic,
            } => {
                let update = self.geometry_status(Some(diagnostic));
                if let Some(request) = request
                    && let Some(inputs) = &self.numeric_inputs
                    && let Ok(projection) = inputs.project()
                {
                    return update.merge(self.queue_numeric_delivery(
                        NumericDeliveryKind::Confirm(request, false, None, projection),
                    ));
                }
                update
            }
            PreparationEvent::Release(token) => {
                self.release_geometry(token);
                Update::default()
            }
            PreparationEvent::Ignored => Update::default(),
        }
    }
    fn selection_view(&self) -> LocalSelectionView {
        let fields = self
            .shell
            .as_ref()
            .map(|shell| {
                shell
                    .shell()
                    .field_descriptors()
                    .into_iter()
                    .map(|d| {
                        let count = self.command_inputs.as_ref().map_or(0, |inputs| {
                            if crate::selection::geometry_field(d.field) {
                                inputs.geometry_values.get(&d.field).map_or(0, Vec::len)
                            } else {
                                inputs
                                    .values
                                    .get(&d.field)
                                    .map_or(0, |value| value.entities().len())
                            }
                        });
                        SelectionFieldCount {
                            field: d.field,
                            count,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        LocalSelectionView {
            selection: EntitySelectionView {
                document: self.document.identity().tree(),
                entities: self
                    .selection
                    .entities()
                    .iter()
                    .map(|id| OpaqueTreeId::new(id.get()))
                    .collect(),
            },
            origin_references: self.origin_selection.clone(),
            fields,
        }
    }
    fn mark_selection_pending(&mut self) {
        self.selection_view_pending = true;
        self.selection_attempts = 0;
        self.selection_suspended = false;
        self.selection_retry_at = None;
    }
    fn selection_published(&mut self) {
        self.selection_view_pending = false;
        self.selection_retry_at = None;
        self.selection_attempts = 0;
        self.selection_suspended = false;
    }
    fn publish_selection(&mut self) -> Update {
        self.mark_selection_pending();
        self.update_scene_selection();
        self.retry_selection(Instant::now(), false)
    }
    fn retry_selection(&mut self, now: Instant, rearm: bool) -> Update {
        if !self.selection_view_pending {
            return Update::default();
        }
        if rearm && self.selection_suspended {
            self.selection_suspended = false;
            self.selection_attempts = 0;
        }
        if self.selection_suspended || self.selection_retry_at.is_some_and(|t| now < t) {
            return Update::default();
        }
        if self
            .shell
            .as_ref()
            .is_none_or(|s| s.shell().tree_projection().document != self.document.identity().tree())
        {
            return Update::default();
        }
        let view = self.selection_view();
        self.selection_attempts += 1;
        self.selection_retry_at = None;
        #[cfg(test)]
        let injected = if self.fail_next_selection > 0 {
            self.fail_next_selection -= 1;
            true
        } else {
            false
        };
        #[cfg(not(test))]
        let injected = false;
        let result = if injected {
            Err(newell_ui::UiError::InvalidSelection)
        } else {
            self.shell
                .as_mut()
                .map(|shell| shell.set_selection_view(view))
                .unwrap_or(Ok(Update::default()))
        };
        match result {
            Ok(update) => {
                self.selection_published();
                update
            }
            Err(error) => {
                if self.selection_attempts == 1 {
                    self.selection_retry_at = now.checked_add(Duration::from_millis(100));
                } else {
                    self.selection_suspended = true;
                }
                log(&format!("selection refresh pending: {error}"));
                Update::default()
            }
        }
    }
    fn handle_selection_request(&mut self, request: SelectionRequest) -> Update {
        let clear_faces = matches!(request.op, SelectionOp::Replace | SelectionOp::Clear);
        let affected_entities = request.entities.clone();
        let clears_references = matches!(request.op, SelectionOp::Replace | SelectionOp::Clear)
            && request.document == self.document.identity()
            && self
                .document
                .document()
                .active_entities_at(None)
                .is_ok_and(|active| request.entities.iter().all(|id| active.contains(id)));
        let mut changed = self.selection.apply(request, &self.document);
        if clear_faces && !self.face_selection.is_empty() {
            self.face_selection.clear();
            changed = true;
        } else if !affected_entities.is_empty() {
            let old = self.face_selection.len();
            self.face_selection
                .retain(|face| !affected_entities.contains(&face.entity));
            changed |= old != self.face_selection.len();
        }
        if clears_references && !self.origin_selection.is_empty() {
            self.origin_selection.clear();
            changed = true;
        }
        if changed {
            self.publish_selection()
        } else {
            Update::default()
        }
    }
    fn handle_face_selection_request(
        &mut self,
        entity: EntityId,
        face: newell_model::geometry::FaceRef,
        op: SelectionOp,
    ) -> Update {
        let Some(geometry) = self.document.document().entity_geometry(entity) else {
            return Update::default();
        };
        if face.entity != entity
            || face.generation != geometry.generation
            || !geometry
                .faces
                .iter()
                .any(|candidate| candidate.face == face.face)
        {
            return Update::default();
        }
        let face_was_selected = self.face_selection.contains(&face);
        let entity_op = match op {
            SelectionOp::Toggle if face_was_selected => SelectionOp::Remove,
            SelectionOp::Toggle => SelectionOp::Add,
            _ => op,
        };
        let mut changed = self.selection.apply(
            SelectionRequest {
                document: self.document.identity(),
                op: entity_op,
                entities: vec![entity],
            },
            &self.document,
        );
        match op {
            SelectionOp::Replace => {
                if self.face_selection != [face] {
                    self.face_selection.clear();
                    self.face_selection.push(face);
                    changed = true;
                }
            }
            SelectionOp::Add => {
                if !self.face_selection.contains(&face) {
                    self.face_selection.push(face);
                    changed = true;
                }
            }
            SelectionOp::Toggle => {
                if let Some(index) = self
                    .face_selection
                    .iter()
                    .position(|selected| *selected == face)
                {
                    self.face_selection.remove(index);
                } else {
                    self.face_selection.push(face);
                }
                changed = true;
            }
            SelectionOp::Remove => {
                let old = self.face_selection.len();
                self.face_selection.retain(|selected| *selected != face);
                changed |= old != self.face_selection.len();
            }
            SelectionOp::Clear => {
                changed |= !self.face_selection.is_empty();
                self.face_selection.clear();
            }
        }
        if changed {
            self.publish_selection()
        } else {
            Update::default()
        }
    }
    fn select_origin_reference(
        &mut self,
        reference: newell_ui::OriginReference,
        edit: SelectionEdit,
    ) -> Update {
        let mut changed = false;
        match edit {
            SelectionEdit::Replace => {
                changed |= self.selection.apply(
                    SelectionRequest {
                        document: self.document.identity(),
                        op: SelectionOp::Clear,
                        entities: Vec::new(),
                    },
                    &self.document,
                );
                if !self.face_selection.is_empty() {
                    self.face_selection.clear();
                    changed = true;
                }
                if self.origin_selection != [reference] {
                    self.origin_selection.clear();
                    self.origin_selection.push(reference);
                    changed = true;
                }
            }
            SelectionEdit::Add => {
                if !self.origin_selection.contains(&reference) {
                    self.origin_selection.push(reference);
                    changed = true;
                }
            }
            SelectionEdit::Toggle => {
                if let Some(index) = self
                    .origin_selection
                    .iter()
                    .position(|value| *value == reference)
                {
                    self.origin_selection.remove(index);
                } else {
                    self.origin_selection.push(reference);
                }
                changed = true;
            }
            SelectionEdit::Remove => {
                let old = self.origin_selection.len();
                self.origin_selection.retain(|value| *value != reference);
                changed |= old != self.origin_selection.len();
            }
            SelectionEdit::Clear => {
                changed |= !self.origin_selection.is_empty();
                self.origin_selection.clear();
                if !self.face_selection.is_empty() {
                    self.face_selection.clear();
                    changed = true;
                }
                changed |= self.selection.apply(
                    SelectionRequest {
                        document: self.document.identity(),
                        op: SelectionOp::Clear,
                        entities: Vec::new(),
                    },
                    &self.document,
                );
            }
        }
        if changed {
            self.publish_selection()
        } else {
            Update::default()
        }
    }
    fn update_scene_selection(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            renderer.set_scene_origin_selection(self.origin_selection.clone());
        }
        if self.geometry.is_some() {
            if !self.scene_available {
                return;
            }
            for (index, binding) in self.scene_bindings.iter().enumerate() {
                let (selected, faces) = match binding.binding {
                    crate::geometry_worker::SceneItemBinding::Committed { entity, generation } => {
                        let faces: Vec<u32> = self
                            .face_selection
                            .iter()
                            .filter(|face| face.entity == entity && face.generation == generation)
                            .map(|face| face.face)
                            .collect();
                        (
                            self.selection.entities().contains(&entity) && faces.is_empty(),
                            faces,
                        )
                    }
                    _ => (false, Vec::new()),
                };
                if let Some(renderer) = &mut self.renderer {
                    renderer.set_scene_item_selected(index, selected);
                    renderer.set_scene_item_selected_faces(index, faces);
                }
            }
            return;
        }
        let mut faces = Vec::new();
        if let Some(binding) = self.scene_entity
            && let Some(geometry) = self.document.document().entity_geometry(binding.entity)
        {
            faces.extend(
                self.face_selection
                    .iter()
                    .filter(|face| {
                        face.entity == binding.entity && face.generation == geometry.generation
                    })
                    .map(|face| face.face),
            );
        }
        let selected = faces.is_empty()
            && self.scene_entity.is_some_and(|binding| {
                binding.document == self.document.identity()
                    && binding.mesh_revision == self.navigation.scene().revision()
                    && self.document.document().entity(binding.entity).is_some()
                    && self.selection.entities().contains(&binding.entity)
            });
        if let Some(renderer) = &mut self.renderer {
            renderer.set_scene_selected(selected);
            renderer.set_scene_item_selected_faces(0, faces);
        }
        if self.scene_selected != selected {
            self.scene_selected = selected;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
    fn reconcile_face_selection(&mut self) {
        self.face_selection.retain(|face| {
            self.document
                .document()
                .entity_geometry(face.entity)
                .is_some_and(|geometry| {
                    geometry.generation == face.generation
                        && geometry.faces.iter().any(|item| item.face == face.face)
                })
        });
    }
    #[cfg(test)]
    fn install_scene_mesh(&mut self, mesh: NewellMesh, entity: Option<EntityId>) {
        self.pending_scene_click = None;
        self.scene_available = true;
        self.nav_interaction.cancel();
        self.navigation.set_mesh(&mesh);
        self.scene_entity = entity
            .filter(|id| self.document.document().entity(*id).is_some())
            .map(|entity| SceneEntityBinding {
                document: self.document.identity(),
                mesh_revision: self.navigation.scene().revision(),
                entity,
            });
        if let Some(renderer) = &mut self.renderer {
            renderer.set_scene_mesh(mesh);
            if let Some(camera) = self.navigation.snapshot() {
                renderer.set_scene_camera(camera);
            }
        }
        self.update_scene_selection();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
    fn finish_scene_selection(
        &mut self,
        point: (f64, f64),
        certificate: Option<(f64, f64)>,
    ) -> Update {
        let pending = self.pending_scene_click.take();
        let (Some(pending), Some(origin)) = (pending, certificate) else {
            return Update::default();
        };
        if (self.geometry.is_some() && !self.scene_available)
            || pending.document != self.document.identity()
            || pending.mesh_revision != self.navigation.scene().revision()
            || self
                .shell
                .as_ref()
                .is_none_or(|s| s.generation() != pending.generation || !s.scene_input_enabled())
            || !self.navigation.visible(point)
        {
            return Update::default();
        }
        let entities = if let Some(hit) = self.navigation.pick(origin) {
            if self.geometry.is_some() {
                let Some(item) = self.scene_bindings.iter().find(|item| {
                    usize::try_from(item.range.face_start)
                        .is_ok_and(|start| hit.face_index >= start)
                        && usize::try_from(item.range.face_end)
                            .is_ok_and(|end| hit.face_index < end)
                }) else {
                    return Update::default();
                };
                let crate::geometry_worker::SceneItemBinding::Committed { entity, generation } =
                    item.binding
                else {
                    return Update::default();
                };
                if self
                    .document
                    .document()
                    .entity_geometry(entity)
                    .is_none_or(|geometry| geometry.generation != generation)
                {
                    return Update::default();
                }
                let face = u32::try_from(hit.face_index - item.range.face_start as usize).ok();
                return face.map_or_else(Update::default, |face| {
                    self.handle_face_selection_request(
                        entity,
                        newell_model::geometry::FaceRef {
                            entity,
                            generation,
                            face,
                        },
                        pending.edit.into(),
                    )
                });
            }
            let Some(binding) = self.scene_entity.filter(|b| {
                b.document == pending.document
                    && b.mesh_revision == hit.mesh_revision
                    && self.document.document().entity(b.entity).is_some()
            }) else {
                return Update::default();
            };
            if let Some(geometry) = self.document.document().entity_geometry(binding.entity)
                && let Ok(face) = u32::try_from(hit.face_index)
                && geometry
                    .faces
                    .iter()
                    .any(|candidate| candidate.face == face)
            {
                return self.handle_face_selection_request(
                    binding.entity,
                    newell_model::geometry::FaceRef {
                        entity: binding.entity,
                        generation: geometry.generation,
                        face,
                    },
                    pending.edit.into(),
                );
            }
            vec![binding.entity]
        } else {
            if pending.edit != SelectionEdit::Replace {
                return Update::default();
            }
            Vec::new()
        };
        self.handle_selection_request(SelectionRequest {
            document: pending.document,
            op: pending.edit.into(),
            entities,
        })
    }
    fn finish_origin_reference_click(&mut self, point: (f64, f64)) -> Option<Update> {
        let pending = self.pending_origin_click.take()?;
        let dx = (point.0 - pending.start.0) / pending.display_scale;
        let dy = (point.1 - pending.start.1) / pending.display_scale;
        if !point.0.is_finite()
            || !point.1.is_finite()
            || dx.mul_add(dx, dy * dy) > crate::navigation::CLICK_DRAG_THRESHOLD_LOGICAL_PX.powi(2)
            || pending.document != self.document.identity()
            || pending.mesh_revision != self.navigation.scene().revision()
            || pending.navigation_revision != self.navigation.revision()
            || self.shell.as_ref().is_none_or(|shell| {
                shell.generation() != pending.generation || !shell.scene_input_enabled()
            })
            || self.navigation.pick_origin_reference(point) != Some(pending.reference)
        {
            return Some(Update::default());
        }
        let selection = self.select_origin_reference(pending.reference, pending.edit);
        let binding = if self
            .live_sketch()
            .is_some_and(|draft| draft.mode == SketchMode::Setup(None))
            && let Some(plane) = OriginPlane::from_reference(pending.reference)
        {
            self.set_sketch_mode(SketchMode::Setup(Some(plane)), true)
        } else {
            Update::default()
        };
        Some(selection.merge(binding))
    }
    fn start_command_inputs(&mut self, operation: ActionOperation) -> Update {
        if operation != ActionOperation::Sketch {
            self.sketch = None;
            if let Some(edit) = self.sketch_edit.take() {
                self.navigation.leave_sketch(edit.entry_camera);
            }
            if self.geometry.is_some()
                && self.projection_sync.acknowledged_version() != Some(self.document.version())
            {
                return Update::default();
            }
        }
        if self.shell.as_ref().is_none_or(|s| {
            s.shell().tree_projection().document != self.document.identity().tree()
                || s.shell().operation() != operation
        }) {
            return Update::default();
        }
        let Some(next) = self.next_command_id.checked_add(1) else {
            log(&newell_ui::UiError::InteractionIdentityExhausted.to_string());
            return Update::default();
        };
        self.next_command_id = next;
        if operation == ActionOperation::Sketch {
            return self.start_sketch(CommandInputSessionId(next));
        }
        self.extrude_edit_target = None;
        let Some(shell) = &self.shell else {
            return Update::default();
        };
        let numeric_descriptors = shell.shell().numeric_descriptors();
        let mut numeric_inputs = match crate::parameters::CommandNumericInputs::new(
            self.document.identity(),
            CommandInputSessionId(next),
            operation,
            &numeric_descriptors,
        ) {
            Ok(inputs) => inputs,
            Err(error) => {
                log(&error.to_string());
                return Update::default();
            }
        };
        if let Some(outgoing) = self.numeric_inputs.as_ref() {
            self.numeric_cache
                .insert(outgoing.operation, outgoing.clone());
        }
        if let Some(cached) = self
            .numeric_cache
            .get(&operation)
            .filter(|i| i.document == self.document.identity())
        {
            numeric_inputs.values = cached.values.clone();
        }
        let insertion = shell.shell().playhead().insertion;
        let owner = if operation == ActionOperation::Extrude {
            let before = match insertion {
                HistoryGap::End => None,
                HistoryGap::Before(id) => match self.document.feature_id(id) {
                    Some(id) => Some(id),
                    None => {
                        log("missing insertion anchor");
                        return Update::default();
                    }
                },
            };
            newell_model::parameters::ParameterDraftOwner::InsertExtrude { before }
        } else {
            newell_model::parameters::ParameterDraftOwner::Document
        };
        numeric_inputs.parameter_draft = match self.document.document().begin_parameter_draft(owner)
        {
            Ok(draft) => draft,
            Err(error) => {
                log(&error.to_string());
                return Update::default();
            }
        };
        // A new command lifetime cannot inherit unpublished parameter identities from an old draft.
        for value in numeric_inputs.values.values_mut() {
            value.formula.clear();
        }

        let mut values = std::collections::BTreeMap::new();
        for d in shell
            .shell()
            .field_descriptors()
            .into_iter()
            .filter(|d| d.field.operation == operation)
        {
            let candidates = candidate_rows(
                &self.document,
                Some(d),
                (operation == ActionOperation::Extrude).then_some(insertion),
            );
            let ids = self
                .selection
                .entities()
                .iter()
                .copied()
                .filter(|id| candidates.iter().any(|r| r.entity.get() == id.get()));
            let mut selected = Selection::default();
            selected.replace(ids.take(d.max_count.unwrap_or(usize::MAX)));
            values.insert(d.field, selected);
        }
        self.command_inputs = Some(CommandSelectionInputs {
            session: CommandInputSessionId(next),
            document: self.document.identity(),
            operation,
            insertion,
            values,
            geometry_values: std::collections::BTreeMap::new(),
            geometry_picker: None,
        });
        self.numeric_inputs = Some(numeric_inputs);
        self.begin_geometry_command();
        self.numeric_last_attempt = 0;
        self.numeric_committed = false;
        self.numeric_preview_suspended = false;
        self.numeric_delivery = None;
        self.active_selection = None;
        #[cfg(test)]
        {
            self.extrude_latch = ExtrudeCommitLatch {
                armed: operation == ActionOperation::Extrude,
                insertion: Some((self.document.identity().tree(), insertion)),
            };
        }
        self.commands.cancel();
        let selection_update = self.publish_selection();
        selection_update.merge(self.publish_numeric_view())
    }
    fn live_sketch(&self) -> Option<SketchSession> {
        self.sketch
            .filter(|draft| draft.document == self.document.identity())
    }

    fn start_sketch(&mut self, command: CommandInputSessionId) -> Update {
        let mut selected = self
            .origin_selection
            .iter()
            .copied()
            .filter_map(OriginPlane::from_reference);
        let support = selected.next().filter(|_| selected.next().is_none());
        let mode = SketchMode::Setup(support);
        let Some(shell) = &mut self.shell else {
            return Update::default();
        };
        let Ok(update) = shell.set_sketch_mode(mode, false) else {
            return Update::default();
        };

        if self.command_inputs.is_some() {
            self.invalidate_geometry_preview();
            if let Some(runtime) = &mut self.geometry {
                if let Some(mut outgoing) = runtime.coordinator.take()
                    && let Some(token) = outgoing.cancel()
                {
                    let _ = runtime.worker.release(token);
                }
                runtime.latest_edit = None;
            }
        }
        self.commands.cancel();
        self.command_inputs = None;
        if let Some(outgoing) = self.numeric_inputs.take() {
            self.numeric_cache.insert(outgoing.operation, outgoing);
        }
        self.numeric_delivery = None;
        self.numeric_committed = false;
        self.extrude_edit_target = None;
        self.active_selection = None;
        self.pending_scene_click = None;
        self.pending_origin_click = None;
        self.sketch = Some(SketchSession {
            document: self.document.identity(),
            command,
            mode,
        });
        update.merge(self.publish_selection())
    }

    fn set_sketch_mode(&mut self, mode: SketchMode, focus_plane: bool) -> Update {
        let Some(mut draft) = self.live_sketch() else {
            return Update::default();
        };
        let Some(shell) = &mut self.shell else {
            return Update::default();
        };
        let Ok(update) = shell.set_sketch_mode(mode, focus_plane) else {
            return Update::default();
        };
        draft.mode = mode;
        self.sketch = (mode != SketchMode::Closed).then_some(draft);
        self.pending_origin_click = None;
        if !matches!(mode, SketchMode::Setup(_)) {
            self.active_selection = None;
        }
        update
    }

    fn enter_sketch_edit(
        &mut self,
        plane: OriginPlane,
        target: Option<newell_model::FeatureId>,
        recipe: newell_model::sketch::SketchRecipe,
    ) -> Update {
        let before = match self
            .shell
            .as_ref()
            .map(|shell| shell.shell().playhead().insertion)
        {
            Some(HistoryGap::Before(id)) => match self.document.feature_id(id) {
                Some(id) => Some(id),
                None => {
                    return self.geometry_status(Some("Missing Sketch insertion anchor".into()));
                }
            },
            _ => None,
        };
        let update = self.set_sketch_mode(SketchMode::Active(plane), false);
        if !update.any() {
            return update;
        }
        self.sketch_edit = Some(crate::sketch::Edit::new(
            self.document.version(),
            before,
            target,
            self.navigation.camera(),
            recipe,
        ));
        self.navigation.enter_sketch(plane, Instant::now());
        let mut numeric_update = Update::default();
        if let Some((draft, shell)) = self.live_sketch().zip(self.shell.as_ref()) {
            let operation = ActionOperation::Sketch;
            let descriptors = shell.shell().numeric_descriptors();
            if let (Ok(mut inputs), Ok(parameters)) = (
                crate::parameters::CommandNumericInputs::new(
                    self.document.identity(),
                    draft.command,
                    operation,
                    &descriptors,
                ),
                self.document
                    .document()
                    .begin_parameter_draft(newell_model::parameters::ParameterDraftOwner::Document),
            ) {
                inputs.parameter_draft = parameters;
                self.numeric_inputs = Some(inputs);
                numeric_update = self.publish_numeric_view();
            }
        }
        self.publish_sketch_ready()
            .merge(update)
            .merge(numeric_update)
    }

    fn close_sketch_edit(&mut self) -> Update {
        let update = self.set_sketch_mode(SketchMode::Closed, false);
        self.numeric_inputs = None;
        self.numeric_delivery = None;
        if let Some(mut edit) = self.sketch_edit.take() {
            edit.cancel_solve();
            self.navigation.leave_sketch(edit.entry_camera);
        }
        update
    }

    fn publish_sketch_ready(&mut self) -> Update {
        let ready = self
            .sketch_edit
            .as_ref()
            .is_some_and(crate::sketch::Edit::ready);
        let tool = self
            .sketch_edit
            .as_ref()
            .and_then(crate::sketch::Edit::draw_tool);
        self.shell
            .as_mut()
            .and_then(|shell| shell.set_sketch_ready(ready, tool).ok())
            .unwrap_or_default()
    }

    fn start_sketch_link(&mut self, tool: newell_ui::DrawTool) -> Update {
        use newell_model::sketch::{ReferenceMode, SketchPrimitive};
        let Some(edit) = self.sketch_edit.as_mut() else {
            return Update::default();
        };
        let document = self.document.document();
        let source = self
            .shell
            .as_ref()
            .and_then(|shell| shell.shell().focused_features_operation())
            .and_then(|target| match target.kind {
                newell_ui::TreeTargetKind::Operation(id) => self.document.feature_id(id),
                _ => None,
            });
        let limit = edit.target.or(edit.before);
        let Some((feature, source)) = document
            .features()
            .iter()
            .take_while(|(id, _)| Some(*id) != limit)
            .find(|(id, _)| Some(*id) == source)
        else {
            return self.geometry_status(Some(
                "Focus an earlier Sketch or B-Rep producer in Features".into(),
            ));
        };
        let mode = if tool == newell_ui::DrawTool::Projected {
            ReferenceMode::Projected
        } else {
            ReferenceMode::Intersect
        };
        let mut preview = Vec::new();
        let mut admit = |item| {
            if let Ok(geometry) = document.resolve_sketch_reference(edit.recipe.support, item)
                && !preview
                    .iter()
                    .any(|(_, existing): &(SketchPrimitive, SketchPrimitive)| {
                        *existing == geometry
                            || match (existing.line(), geometry.line()) {
                                (Some((a, b)), Some((c, d))) => a == d && b == c,
                                _ => false,
                            }
                    })
            {
                preview.push((item, geometry));
            }
        };
        if let newell_model::Feature::Sketch { recipe, .. } = source {
            for &(element, _) in &recipe.primitives {
                admit(SketchPrimitive::Reference {
                    feature: *feature,
                    element,
                    mode,
                });
            }
        } else {
            use newell_model::geometry::{DirectionRef, FaceRef};
            for &entity in source.outputs() {
                let Some(geometry) = document.entity_geometry(entity) else {
                    continue;
                };
                for face in &geometry.faces {
                    let support = FaceRef {
                        entity,
                        generation: geometry.generation,
                        face: face.face,
                    };
                    if mode == ReferenceMode::Intersect
                        && let Ok(sections) =
                            document.sketch_face_sections(edit.recipe.support, *feature, support)
                        && !sections.is_empty()
                    {
                        for (index, _) in sections.iter().enumerate() {
                            if let Ok(interval) = u32::try_from(index) {
                                admit(SketchPrimitive::BrepFaceSection {
                                    feature: *feature,
                                    face: support,
                                    interval,
                                });
                            }
                        }
                        continue;
                    }
                    for wire in &face.wires {
                        for edge in wire
                            .edges
                            .iter()
                            .filter(|edge| edge.straight || edge.cubic.is_some())
                        {
                            let count = if mode == ReferenceMode::Intersect && !edge.straight {
                                3
                            } else {
                                1
                            };
                            for crossing in 0..count {
                                admit(SketchPrimitive::BrepReference {
                                    feature: *feature,
                                    edge: DirectionRef {
                                        support,
                                        wire: wire.wire,
                                        edge: edge.edge,
                                    },
                                    mode,
                                    crossing,
                                });
                            }
                        }
                    }
                }
            }
        }
        if preview.is_empty() {
            return self.geometry_status(Some("No compatible reference geometry".into()));
        }
        edit.link_from(tool, preview);
        self.publish_sketch_ready()
    }

    fn handle_sketch_intent(&mut self, intent: SketchIntent) -> Update {
        use crate::sketch::Tool;
        match (intent, self.live_sketch().map(|draft| draft.mode)) {
            (SketchIntent::Confirm, Some(SketchMode::Setup(Some(plane)))) => {
                self.enter_sketch_edit(plane, None, crate::sketch::empty_recipe(plane))
            }
            (SketchIntent::Confirm, Some(SketchMode::Active(_))) => {
                let Some(edit) = self.sketch_edit.as_ref().filter(|edit| edit.ready()) else {
                    return Update::default();
                };
                let result = if let Some(inputs) = self.numeric_inputs.as_ref().filter(|inputs| {
                    inputs.operation == ActionOperation::Sketch
                        && edit.recipe.formula_ids().next().is_some()
                }) {
                    self.document.commit_formula_sketch(
                        edit.source,
                        edit.before,
                        edit.target,
                        edit.recipe.clone(),
                        &inputs.parameter_draft,
                    )
                } else {
                    self.document.commit_sketch(
                        edit.source,
                        edit.before,
                        edit.target,
                        edit.recipe.clone(),
                    )
                };
                match result {
                    Ok(()) => self
                        .close_sketch_edit()
                        .merge(self.geometry_status(None))
                        .merge(self.poll_document_projection(
                            Instant::now(),
                            ProjectionTrigger::UserActivation,
                        )),
                    Err(error) => self.geometry_status(Some(error.to_string())),
                }
            }
            (SketchIntent::Cancel, Some(_)) => self.close_sketch_edit(),
            (SketchIntent::Escape, Some(SketchMode::Setup(_))) => self.close_sketch_edit(),
            (SketchIntent::Draw(newell_ui::DrawTool::Insert), Some(SketchMode::Active(_))) => {
                let recipe = self
                    .shell
                    .as_ref()
                    .and_then(|shell| shell.shell().focused_features_operation())
                    .and_then(|target| match target.kind {
                        newell_ui::TreeTargetKind::Operation(id) => self.document.feature_id(id),
                        _ => None,
                    })
                    .and_then(|id| {
                        let edit = self.sketch_edit.as_ref()?;
                        (edit.target != Some(id))
                            .then(|| self.document.document().feature(id))
                            .flatten()
                    })
                    .and_then(|feature| match feature {
                        newell_model::Feature::Sketch { recipe, .. } => Some(recipe.clone()),
                        _ => None,
                    });
                let Some((edit, recipe)) = self.sketch_edit.as_mut().zip(recipe) else {
                    return self.geometry_status(Some(
                        "Focus another Sketch in Features before Insert".into(),
                    ));
                };
                if edit.recipe.support != recipe.support {
                    return self
                        .geometry_status(Some("Insert requires the same support plane".into()));
                }
                let mut recipe = recipe;
                for (_, item) in &mut recipe.primitives {
                    if matches!(
                        item,
                        newell_model::sketch::SketchPrimitive::Reference { .. }
                            | newell_model::sketch::SketchPrimitive::BrepReference { .. }
                            | newell_model::sketch::SketchPrimitive::BrepFaceSection { .. }
                    ) {
                        match self
                            .document
                            .document()
                            .resolve_sketch_reference(recipe.support, *item)
                        {
                            Ok(geometry) => *item = geometry,
                            Err(error) => return self.geometry_status(Some(error.to_string())),
                        }
                    }
                }
                edit.insert_from(recipe);
                self.publish_sketch_ready()
            }
            (
                SketchIntent::Draw(
                    tool @ (newell_ui::DrawTool::Projected | newell_ui::DrawTool::Intersect),
                ),
                Some(SketchMode::Active(_)),
            ) => self.start_sketch_link(tool),
            (intent, Some(SketchMode::Active(_))) => {
                if let Some(edit) = &mut self.sketch_edit {
                    match intent {
                        SketchIntent::Line => edit.set_tool(Tool::Line),
                        SketchIntent::Rectangle => edit.set_tool(Tool::Rectangle),
                        SketchIntent::Draw(tool) => edit.set_tool(Tool::Draw(tool)),
                        SketchIntent::Delete => {
                            if let Err(error) = edit.delete_selected() {
                                return self.geometry_status(Some(error.to_string()));
                            }
                        }
                        SketchIntent::Detach => {
                            edit.refresh_reference_picks(self.document.document());
                            if let Err(error) = edit.detach_selected() {
                                return self.geometry_status(Some(error.to_string()));
                            }
                        }
                        SketchIntent::Escape if edit.escape() => return self.close_sketch_edit(),
                        _ => {}
                    }
                }
                self.publish_sketch_ready()
            }
            _ => Update::default(),
        }
    }

    fn sketch_strokes(&self) -> Vec<newell_render::SketchStroke> {
        let camera = self.navigation.camera();
        let scale = self
            .window
            .as_ref()
            .map_or(1.0, |window| window.scale_factor());
        let editing = self.sketch_edit.as_ref().and_then(|edit| edit.target);
        let mut strokes: Vec<_> = self
            .document
            .document()
            .features()
            .iter()
            .filter_map(|(id, feature)| match feature {
                newell_model::Feature::Sketch {
                    recipe,
                    visible: true,
                    ..
                } if Some(*id) != editing => Some(recipe),
                _ => None,
            })
            .flat_map(|recipe| {
                crate::sketch::saved_strokes(recipe, camera)
                    .chain(crate::sketch::linked_strokes(
                        recipe,
                        self.document.document(),
                        camera,
                    ))
                    .chain(crate::sketch::dimension_strokes(
                        recipe,
                        self.document.document(),
                        camera,
                        scale,
                    ))
                    .chain(crate::sketch::join_strokes(recipe, camera, scale))
            })
            .collect();
        if let Some(edit) = &self.sketch_edit {
            let pointer = self.sketch_pointer();
            if let Some((p, local)) = pointer {
                let bypass = self.modifiers.control_key();
                let exposed = self.sketch_exposed_regions(camera.viewport);
                edit.update_snap(local, p, scale, camera, bypass, &exposed);
                let rect = exposed
                    .iter()
                    .find(|rect| rect.contains(p.0, p.1))
                    .copied()
                    .unwrap_or(camera.viewport);
                strokes.extend(edit.snap_marks(camera, [p.0, p.1], scale, rect, bypass));
            } else {
                edit.clear_live_snap();
            }
            strokes.extend(edit.strokes(camera, pointer.map(|(_, local)| edit.snapped_local(local))));
            strokes.extend(crate::sketch::join_strokes(&edit.recipe, camera, scale));
            strokes.extend(crate::sketch::dimension_strokes(
                &edit.recipe,
                self.document.document(),
                camera,
                scale,
            ));
            strokes.extend(crate::sketch::linked_strokes(
                &edit.recipe,
                self.document.document(),
                camera,
            ));
        }
        strokes
    }

    fn sketch_exposed_regions(&self, viewport: SceneViewport) -> Vec<SceneViewport> {
        let occlusions = self.shell.as_ref().map_or_else(Vec::new, scene_occlusions);
        newell_render::exposed_regions(viewport, &occlusions)
    }

    fn sketch_pointer(&self) -> Option<((f64, f64), [f64; 2])> {
        let edit = self.sketch_edit.as_ref()?;
        if self.navigation.has_capture() || self.nav_interaction.active() || self.native_resizing {
            return None;
        }
        self.scene_pointer
            .filter(|_| {
                self.shell
                    .as_ref()
                    .is_some_and(ShellController::scene_input_enabled)
            })
            .filter(|p| self.scene_navigation_at((p.x, p.y)))
            .and_then(|p| {
                let point = (p.x, p.y);
                let local = self
                    .navigation
                    .sketch_point(crate::sketch::ui_plane(edit.recipe.support), point)?;
                Some((point, local))
            })
    }

    fn sketch_auxiliary_admitted(&self, point: (f64, f64)) -> bool {
        !self.occluded
            && !self.resize.is_active()
            && !self.native_resizing
            && !self.nav_interaction.active()
            && !self.nav_geometry.as_ref().is_some_and(|g| g.reserves(point))
            && self.navigation.visible(point)
            && self.shell.as_ref().is_some_and(|shell| {
                shell.navigation_admission((self.last_size.width, self.last_size.height), self.resizable())
                    && shell.shell().editing_field().is_none()
            })
    }

    fn finish_sketch_secondary(&mut self, point: (f64, f64)) -> Update {
        let admitted = self.sketch_auxiliary_admitted(point);
        let (click, camera_changed) = self.navigation.finish_secondary(point);
        if camera_changed {
            if let (Some(renderer), Some(camera)) =
                (&mut self.renderer, self.navigation.snapshot())
            {
                renderer.set_scene_camera(camera);
            }
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        let Some(edit) = &mut self.sketch_edit else {
            return Update::default();
        };
        if edit.secondary_press.take().is_some_and(|press| {
            admitted && click
                && press.navigation == self.navigation.revision()
                && self.shell.as_ref().is_some_and(|shell| shell.generation() == press.layout)
        }) && !edit.is_select() {
            edit.set_tool(crate::sketch::Tool::Select);
            return self.publish_sketch_ready().merge(self.geometry_status(None));
        }
        Update::default()
    }

    fn begin_sketch_click(&mut self, point: (f64, f64), scale: f64) {
        let (Some(edit), Some(shell)) = (&mut self.sketch_edit, &self.shell) else {
            return;
        };
        if shell.scene_input_enabled()
            && self
                .navigation
                .sketch_point(crate::sketch::ui_plane(edit.recipe.support), point)
                .is_some()
        {
            edit.press = Some(crate::sketch::Click {
                point,
                navigation: self.navigation.revision(),
                layout: shell.generation(),
                scale,
            });
        }
    }

    fn finish_sketch_click(&mut self, point: (f64, f64)) -> Update {
        let camera = self.navigation.camera();
        let exposed = self.sketch_exposed_regions(camera.viewport);
        let Some(edit) = &mut self.sketch_edit else {
            return Update::default();
        };
        let Some(crate::sketch::Click {
            point: start,
            navigation: revision,
            layout: generation,
            scale,
        }) = edit.press.take()
        else {
            return Update::default();
        };
        if revision != self.navigation.revision()
            || (point.0 - start.0).hypot(point.1 - start.1) > 4.0 * scale
            || self.shell.as_ref().is_none_or(|shell| {
                shell.generation() != generation || !shell.scene_input_enabled()
            })
        {
            return Update::default();
        }
        let plane = crate::sketch::ui_plane(edit.recipe.support);
        let Some(local) = self.navigation.sketch_point(plane, point) else {
            return Update::default();
        };
        if edit.solve.is_some() {
            return Update::default();
        }
        edit.refresh_reference_picks(self.document.document());
        if edit.is_select() {
            let message = edit.select_click(point, scale, camera, &exposed);
            return self.geometry_status(message);
        }
        let placement = if edit.magnetic_tool() {
            local
        } else {
            edit.update_snap(local, point, scale, camera, self.modifiers.control_key(), &exposed);
            edit.snapped_local(local)
        };
        let mut draw_key = None;
        let mut anchors = Vec::new();
        let candidate =
            if let Some(candidate) = edit.segment_edit(placement, point, scale, camera) {
                match candidate {
                    Ok(Some(recipe)) => {
                        anchors.extend(recipe.segment_equations.iter().filter_map(
                            |(id, equation)| {
                                (edit.recipe.segment_equations.get(id) != Some(equation))
                                    .then_some(equation.segment)
                            },
                        ));
                        Some(recipe)
                    }
                    Ok(None) => {
                        return self.publish_sketch_ready().merge(
                            self.geometry_status(Some("Click to place the dimension".into())),
                        );
                    }
                    Err(error) => return self.geometry_status(Some(error.to_string())),
                }
            } else if edit.magnetic_tool() {
                match edit.magnetic_click(
                    local,
                    point,
                    scale,
                    camera,
                    self.modifiers.control_key(),
                    &exposed,
                ) {
                    Ok(Some(placement)) => {
                        draw_key = Some(placement.key);
                        Some(placement.recipe)
                    }
                    Ok(None) => None,
                    Err(error) => return self.geometry_status(Some(error.to_string())),
                }
            } else {
                if let Err(error) = edit.click(placement, point, scale, camera) {
                    return self.geometry_status(Some(error.to_string()));
                }
                None
            };
        if let Some(recipe) = candidate {
            let Some(runtime) = &self.geometry else {
                return self
                    .geometry_status(Some("Sketch solve requires the geometry worker".into()));
            };
            let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
            if let Err(error) = runtime
                .worker
                .submit_sketch(crate::geometry_worker::SketchSolve {
                    recipe,
                    cancelled: Arc::clone(&cancelled),
                    draw: draw_key.is_some(),
                    anchors,
                })
            {
                return self.geometry_status(Some(error.to_string()));
            }
            edit.solve = Some(cancelled);
            edit.pending_draw = draw_key;
            return self
                .publish_sketch_ready()
                .merge(self.geometry_status(Some("Solving Sketch".into())));
        }
        self.publish_sketch_ready()
            .merge(self.geometry_status(None))
    }

    fn panel_rows(&self, request: &SelectionOpen) -> Option<Vec<SelectionRow>> {
        if request.document != self.document.identity().tree() {
            return None;
        }
        match request.destination {
            SelectionDestination::Window => Some(candidate_rows(&self.document, None, None)),
            SelectionDestination::Field { session, field }
                if field.operation == ActionOperation::Sketch =>
            {
                let draft = self.live_sketch()?;
                let shell = self.shell.as_ref()?;
                (draft.command == session
                    && matches!(draft.mode, SketchMode::Setup(_))
                    && shell.shell().selection_field_visible(field))
                .then(|| {
                    crate::selection::sketch_plane_rows()
                        .map(|(_, row)| row)
                        .collect()
                })
            }
            SelectionDestination::Field { session, field } => {
                let inputs = self.command_inputs.as_ref().filter(|i| {
                    i.session == session
                        && i.document == self.document.identity()
                        && i.operation == field.operation
                })?;
                if let HistoryGap::Before(id) = inputs.insertion
                    && self.document.feature_id(id).is_none()
                {
                    return None;
                }
                let shell = self.shell.as_ref()?;
                if !shell.shell().selection_field_visible(field) {
                    return None;
                }
                let descriptor = shell
                    .shell()
                    .field_descriptors()
                    .into_iter()
                    .find(|d| d.field == field)?;
                if crate::selection::geometry_field(field) {
                    let surface = shell.shell().extrude().values.geometry
                        == newell_ui::ExtrudeGeometryType::Surface;
                    return crate::selection::geometry_candidates(
                        &self.document,
                        field,
                        inputs.insertion,
                        surface,
                    )
                    .ok()
                    .map(|candidates| {
                        candidates
                            .into_iter()
                            .map(|candidate| candidate.row)
                            .collect()
                    });
                }
                if field.operation == ActionOperation::Extrude
                    && descriptor.role == newell_ui::SelectionFieldRole::MergeScope
                    && shell.shell().extrude().values.geometry
                        == newell_ui::ExtrudeGeometryType::Surface
                {
                    let mut filter = descriptor;
                    filter.role = newell_ui::SelectionFieldRole::EntityReference;
                    return Some(
                        candidate_rows(&self.document, Some(filter), Some(inputs.insertion))
                            .into_iter()
                            .filter(|row| {
                                self.document
                                    .document()
                                    .entity(EntityId::from_raw(row.entity.get()))
                                    .is_some_and(|entity| {
                                        entity.kind() == newell_model::EntityKind::Surface
                                    })
                            })
                            .collect(),
                    );
                }
                Some(candidate_rows(
                    &self.document,
                    Some(descriptor),
                    (field.operation == ActionOperation::Extrude).then_some(inputs.insertion),
                ))
            }
        }
    }
    fn handle_selection_ui(&mut self, request: SelectionUiRequest) -> Update {
        match request {
            SelectionUiRequest::OpenField(d) => self.open_field_selection(d),
            SelectionUiRequest::Complete(completion) => {
                let matches_active = self.active_selection.as_ref().is_some_and(|r| {
                    r.session == completion.session
                        && r.document == completion.document
                        && r.destination == completion.destination
                });
                if !matches_active {
                    return Update::default();
                }
                let Some(active) = self.active_selection.take() else {
                    return Update::default();
                };
                let Some(rows) = self.panel_rows(&active) else {
                    return Update::default();
                };
                let SelectionOutcome::Confirmed(ids) = completion.outcome else {
                    return Update::default();
                };
                if active.max_count.is_some_and(|max| ids.len() > max)
                    || ids.iter().any(|id| !rows.iter().any(|r| r.entity == *id))
                    || ids
                        .iter()
                        .copied()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len()
                        != ids.len()
                {
                    return Update::default();
                }
                if let SelectionDestination::Field { field, .. } = active.destination
                    && crate::selection::geometry_field(field)
                {
                    let Some(inputs) = self.command_inputs.as_mut() else {
                        return Update::default();
                    };
                    let Some(picker) = inputs.geometry_picker.take() else {
                        return Update::default();
                    };
                    if picker.version != self.document.version()
                        || picker.session != active.session
                        || picker.field != field
                    {
                        return Update::default();
                    }
                    let references = ids
                        .iter()
                        .map(|id| {
                            picker
                                .candidates
                                .iter()
                                .find(|candidate| candidate.row.entity == *id)
                                .map(|candidate| candidate.reference)
                        })
                        .collect::<Option<Vec<_>>>();
                    let Some(references) = references else {
                        return Update::default();
                    };
                    inputs.geometry_values.insert(field, references);
                    if let Some(legacy) = inputs.values.get_mut(&field) {
                        legacy.clear();
                    }
                    return self.publish_selection();
                }
                if matches!(active.destination, SelectionDestination::Field { field, .. }
                    if field.operation == ActionOperation::Sketch)
                {
                    let [id] = ids.as_slice() else {
                        return Update::default();
                    };
                    return crate::selection::sketch_plane_rows()
                        .find(|(_, row)| row.entity == *id)
                        .map_or_else(Update::default, |(plane, _)| {
                            self.set_sketch_mode(SketchMode::Setup(Some(plane)), false)
                        });
                }
                let entities = ids
                    .iter()
                    .map(|id| EntityId::from_raw(id.get()))
                    .collect::<Vec<_>>();
                match active.destination {
                    SelectionDestination::Window => {
                        self.handle_selection_request(SelectionRequest {
                            document: self.document.identity(),
                            op: SelectionOp::Replace,
                            entities,
                        })
                    }
                    SelectionDestination::Field { field, .. } => {
                        if let Some(value) = self
                            .command_inputs
                            .as_mut()
                            .and_then(|i| i.values.get_mut(&field))
                        {
                            value.replace(entities);
                        }
                        self.publish_selection()
                    }
                }
            }
        }
    }
    fn open_field_selection(&mut self, descriptor: SelectionFieldDescriptor) -> Update {
        let Some(next) = self.next_selection_id.checked_add(1) else {
            log(&newell_ui::UiError::InteractionIdentityExhausted.to_string());
            return Update::default();
        };
        let Some(shell) = &self.shell else {
            return Update::default();
        };
        if !shell.shell().selection_field_visible(descriptor.field)
            || !shell.shell().field_descriptors().contains(&descriptor)
        {
            return Update::default();
        }
        if descriptor.role == newell_ui::SelectionFieldRole::SketchPlane {
            let Some(draft) = self.live_sketch() else {
                return Update::default();
            };
            let SketchMode::Setup(support) = draft.mode else {
                return Update::default();
            };
            let applied = crate::selection::sketch_plane_rows()
                .filter_map(|(plane, row)| (Some(plane) == support).then_some(row.entity))
                .collect();
            let mut request = SelectionOpen {
                session: SelectionSessionId(next),
                document: self.document.identity().tree(),
                destination: SelectionDestination::Field {
                    session: draft.command,
                    field: descriptor.field,
                },
                rows: Vec::new(),
                applied,
                max_count: descriptor.max_count,
            };
            let Some(rows) = self.panel_rows(&request) else {
                return Update::default();
            };
            request.rows = rows;
            return self.publish_field_picker(request, None);
        }
        let Some(inputs) = self.command_inputs.as_ref() else {
            return Update::default();
        };
        let mut request = SelectionOpen {
            session: SelectionSessionId(next),
            document: self.document.identity().tree(),
            destination: SelectionDestination::Field {
                session: inputs.session,
                field: descriptor.field,
            },
            rows: Vec::new(),
            applied: inputs
                .values
                .get(&descriptor.field)
                .map(|v| {
                    v.entities()
                        .iter()
                        .map(|id| OpaqueTreeId::new(id.get()))
                        .collect()
                })
                .unwrap_or_default(),
            max_count: descriptor.max_count,
        };
        let Some(rows) = self.panel_rows(&request) else {
            return Update::default();
        };
        request.rows = rows;
        let picker = if crate::selection::geometry_field(descriptor.field) {
            let surface = self.shell.as_ref().is_some_and(|shell| {
                shell.shell().extrude().values.geometry == newell_ui::ExtrudeGeometryType::Surface
            });
            let candidates = match crate::selection::geometry_candidates(
                &self.document,
                descriptor.field,
                inputs.insertion,
                surface,
            ) {
                Ok(candidates) => candidates,
                Err(error) => {
                    log(&error.to_string());
                    return Update::default();
                }
            };
            request.applied = candidates
                .iter()
                .filter(|candidate| {
                    inputs
                        .geometry_values
                        .get(&descriptor.field)
                        .is_some_and(|values| values.contains(&candidate.reference))
                })
                .map(|candidate| candidate.row.entity)
                .collect();
            Some(crate::selection::GeometryPicker {
                version: self.document.version(),
                session: request.session,
                field: descriptor.field,
                candidates,
            })
        } else {
            None
        };
        request
            .applied
            .retain(|id| request.rows.iter().any(|r| r.entity == *id));
        self.publish_field_picker(request, picker)
    }

    fn publish_field_picker(
        &mut self,
        request: SelectionOpen,
        picker: Option<crate::selection::GeometryPicker>,
    ) -> Update {
        match self
            .shell
            .as_mut()
            .map(|s| s.open_selection(request.clone()))
        {
            Some(Ok(update)) => {
                self.next_selection_id = request.session.0;
                if let Some(inputs) = self.command_inputs.as_mut() {
                    inputs.geometry_picker = picker;
                }
                self.active_selection = Some(request);
                self.navigation.cancel();
                self.nav_interaction.cancel();
                self.pending_scene_click = None;
                self.pending_origin_click = None;
                update
            }
            _ => Update::default(),
        }
    }
    pub(crate) fn new() -> Self {
        let mut document = DocumentSession::default();
        let document_events = document.subscribe();
        let document_identity = document.identity();
        Self {
            renderer: None,
            window: None,
            shell: None,
            document,
            document_events,
            projection_sync: ProjectionSync::default(),
            #[cfg(test)]
            extrude_latch: ExtrudeCommitLatch::default(),
            committed_cursor: None,
            layout_retry_rearm_pending: false,
            #[cfg(test)]
            last_ui_publication: None,
            selection: WindowSelection::new(document_identity),
            origin_selection: Vec::new(),
            face_selection: Vec::new(),
            command_inputs: None,
            numeric_inputs: None,
            numeric_cache: std::collections::BTreeMap::new(),
            numeric_delivery: None,
            numeric_last_attempt: 0,
            extrude_edit_target: None,
            properties_editor: None,
            properties_close_source: None,
            properties_retry_at: None,
            properties_attempts: 0,
            last_properties_id: 0,
            numeric_committed: false,
            numeric_preview_suspended: false,
            active_selection: None,
            next_selection_id: 0,
            next_command_id: 0,
            selection_view_pending: false,
            selection_retry_at: None,
            selection_attempts: 0,
            selection_suspended: false,
            #[cfg(test)]
            fail_next_selection: 0,
            commands: CommandController::default(),
            scene_entity: None,
            geometry: None,
            geometry_wake: None,
            pending_scene: None,
            #[cfg(test)]
            fail_scene_publish: 0,
            scene_bindings: Vec::new(),
            scene_available: false,
            scene_initialized: false,
            pending_scene_click: None,
            pending_origin_click: None,
            sketch: None,
            sketch_edit: None,
            scene_selected: false,
            modifiers: ModifiersState::empty(),
            navigation: Navigation::default(),
            nav_geometry: None,
            nav_hover: None,
            nav_cube_hover: None,
            nav_interaction: NavInteraction::default(),
            nav_generation: 0,
            nav_ui_masks: Vec::new(),
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

    fn publish_numeric_view(&mut self) -> Update {
        let Some(inputs) = self.numeric_inputs.as_ref() else {
            return Update::default();
        };
        match inputs.project() {
            Ok(projection) => self.queue_numeric_delivery(NumericDeliveryKind::View(projection)),
            Err(error) => {
                log(&error.to_string());
                Update::default()
            }
        }
    }
    fn queue_numeric_delivery(&mut self, kind: NumericDeliveryKind) -> Update {
        self.numeric_delivery = Some(NumericDelivery {
            kind,
            retry_at: None,
            attempts: 0,
        });
        self.deliver_numeric(Instant::now(), false)
    }
    fn deliver_numeric(&mut self, now: Instant, rearm: bool) -> Update {
        if self.geometry.is_some()
            && self.numeric_committed
            && (self.pending_scene.is_some()
                || !self.scene_available
                || self.projection_sync.acknowledged_version() != Some(self.document.version()))
        {
            return Update::default();
        }
        let Some(mut pending) = self.numeric_delivery.take() else {
            return Update::default();
        };
        if pending.attempts > 0 {
            if pending.retry_at.is_none() && !rearm {
                self.numeric_delivery = Some(pending);
                return Update::default();
            }
            if pending.retry_at.is_some_and(|at| now < at) {
                self.numeric_delivery = Some(pending);
                return Update::default();
            }
            if rearm && pending.retry_at.is_none() {
                pending.attempts = 0;
            }
        }
        let result = if let Some(shell) = self.shell.as_mut() {
            match &pending.kind {
                NumericDeliveryKind::FormulaApply(request, projection, diagnostic) => {
                    shell.finish_formula_apply(request, projection.clone(), diagnostic.clone())
                }
                NumericDeliveryKind::View(projection) => {
                    shell.set_numeric_projection(projection.clone())
                }
                NumericDeliveryKind::Formula(request, projection) => {
                    shell.open_numeric_formula(*request, projection.clone())
                }
                NumericDeliveryKind::Confirm(request, accepted, _, projection) => shell
                    .finish_numeric_confirm_with_projection(
                        *request,
                        *accepted,
                        projection.clone(),
                    ),
            }
        } else {
            Err(newell_ui::UiError::InvalidNumericProjection)
        };
        match result {
            Ok(mut update) => {
                let formula_accepted = matches!(&pending.kind,NumericDeliveryKind::FormulaApply(_,_,diagnostic) if diagnostic.is_none());
                if let NumericDeliveryKind::Confirm(request, true, feature, _) = pending.kind
                    && (request.operation != ActionOperation::Extrude || feature.is_some())
                {
                    self.numeric_cache.remove(&request.operation);
                    self.numeric_inputs = None;
                    self.command_inputs = None;
                    self.extrude_edit_target = None;
                    self.active_selection = None;
                    self.numeric_committed = false;
                    update = update.merge(self.publish_selection());
                }
                if formula_accepted {
                    self.sync_extrude_command();
                }
                update
            }
            Err(error) => {
                log(&error.to_string());
                pending.attempts = pending.attempts.saturating_add(1);
                pending.retry_at = if pending.attempts == 1 {
                    Some(now + Duration::from_millis(100))
                } else {
                    None
                };
                self.numeric_delivery = Some(pending);
                Update::default()
            }
        }
    }
    fn sync_sketch_numeric(&mut self) {
        use newell_model::Unit;
        let Some((edit, inputs)) = self.sketch_edit.as_mut().zip(self.numeric_inputs.as_ref())
        else {
            return;
        };
        if inputs.operation == ActionOperation::Sketch {
            if let Some(shell) = self.shell.as_ref() {
                shell.shell().sketch_text().clone_into(&mut edit.text);
                edit.associative_mirror = shell.shell().sketch_associative_mirror();
                edit.transform_move = shell.shell().sketch_transform_move();
                edit.spline_handles = shell.shell().sketch_spline_handles();
                edit.offset_chain = shell.shell().sketch_offset_chain();
            }
            let input = |key: Option<u32>| {
                let field = key.map(|key| newell_ui::NumericFieldKey {
                    operation: ActionOperation::Sketch,
                    key,
                });
                let accepted = field.and_then(|key| inputs.values.get(&key));
                let formula = field.zip(accepted).and_then(|(key, value)| {
                    if value.formula.is_empty() {
                        None
                    } else {
                        inputs.bindings.get(&key).copied()
                    }
                });
                let value = accepted.and_then(|value| {
                    value
                        .quantity
                        .value_in(Unit::Unitless)
                        .or_else(|_| value.quantity.value_in(Unit::Millimetre))
                        .or_else(|_| value.quantity.value_in(Unit::Degree))
                        .ok()
                });
                (value, formula)
            };
            (edit.numeric, edit.numeric_formula) =
                input(edit.draw_tool().and_then(newell_ui::DrawTool::numeric_key));
            (edit.second_numeric, edit.second_formula) = input(
                edit.draw_tool()
                    .and_then(newell_ui::DrawTool::second_numeric_key),
            );
        }
    }

    fn process_numeric_requests(&mut self) -> (Update, bool) {
        let Some(shell) = self.shell.as_mut() else {
            return (Update::default(), false);
        };
        if let Some(inputs) = self.numeric_inputs.as_mut() {
            inputs.active_fields = inputs
                .descriptors
                .keys()
                .copied()
                .filter(|field| shell.shell().numeric_field_visible(*field))
                .collect();
        }
        let requests = shell.take_numeric_requests();
        let allowed: Vec<_> = requests
            .iter()
            .map(|r| shell.numeric_origin_allows(r))
            .collect();
        let mut rejected = false;
        let mut rejections = Vec::new();
        if let Some(error) = shell.take_numeric_error() {
            log(&error.to_string());
            rejected = true;
        }
        for (request, allowed) in requests.into_iter().zip(allowed) {
            let destination = request.destination;
            let sequence = request.sequence;
            let result = if let Some(inputs) = self.numeric_inputs.as_mut() {
                if request.destination.document != self.document.identity().tree() {
                    Err(AppError::InvalidNumericRequest)
                } else if let Some(descriptor) =
                    inputs.descriptors.get(&request.destination.field).copied()
                {
                    if allowed {
                        let used = self.sketch_edit.as_ref().is_some_and(|edit| {
                            inputs
                                .bindings
                                .get(&destination.field)
                                .is_some_and(|id| edit.recipe.formula_ids().any(|used| used == *id))
                        });
                        if used && inputs.operation == ActionOperation::Sketch {
                            let mut next = inputs.clone();
                            next.bindings.remove(&destination.field);
                            let accepted = next.apply(request, descriptor);
                            if accepted.is_ok() {
                                *inputs = next;
                            } else {
                                inputs.consumed_sequence = next.consumed_sequence;
                            }
                            accepted
                        } else {
                            inputs.apply(request, descriptor)
                        }
                    } else {
                        inputs.consume_sequence(&request);
                        Err(AppError::InvalidNumericRequest)
                    }
                } else {
                    inputs.consume_sequence(&request);
                    Err(AppError::InvalidNumericRequest)
                }
            } else {
                Err(AppError::InvalidNumericRequest)
            };
            if let Err(error) = result {
                rejected = true;
                rejections.push((destination, sequence));
                log(&error.to_string());
            }
        }
        let mut update = Update::default();
        for (destination, sequence) in rejections {
            match shell.reject_numeric_edit(destination, sequence) {
                Ok(next) => update = update.merge(next),
                Err(error) => log(&error.to_string()),
            }
        }
        (update, rejected)
    }
    fn complete_numeric_formula(
        &mut self,
        request: newell_ui::NumericFormulaOpenRequest,
    ) -> Update {
        let certificate = newell_ui::NumericRequest {
            destination: request.destination,
            sequence: request.sequence,
            origin: request.origin,
            edit: newell_ui::NumericEdit::Formula(String::new()),
        };
        if self.shell.as_ref().is_none_or(|shell| {
            !shell.numeric_origin_allows(&certificate)
                || !shell.shell().numeric_formula_pending(request)
        }) {
            return Update::default();
        }
        let Some(inputs) = self.numeric_inputs.as_mut().filter(|i| {
            i.document == self.document.identity()
                && i.document.tree() == request.destination.document
                && i.session == request.destination.session
                && i.operation == request.destination.field.operation
                && request.sequence > i.consumed_sequence
                && request.through_sequence <= i.consumed_sequence
                && i.values.contains_key(&request.destination.field)
        }) else {
            return Update::default();
        };
        inputs.consumed_sequence = request.sequence;
        match inputs.project() {
            Ok(projection) => {
                self.queue_numeric_delivery(NumericDeliveryKind::Formula(request, projection))
            }
            Err(error) => {
                log(&error.to_string());
                Update::default()
            }
        }
    }
    fn complete_formula_apply(&mut self, request: newell_ui::FormulaApplyRequest) -> Update {
        let certificate = newell_ui::NumericRequest {
            destination: request.destination,
            sequence: request.attempt.0,
            origin: request.origin,
            edit: newell_ui::NumericEdit::Formula(request.source.clone()),
        };
        if self.shell.as_ref().is_none_or(|shell| {
            !shell.shell().formula_apply_pending(&request)
                || !shell.numeric_origin_allows(&certificate)
        }) {
            return Update::default();
        }
        let Some(inputs) = self
            .numeric_inputs
            .as_mut()
            .filter(|i| i.document == self.document.identity())
        else {
            return Update::default();
        };
        let mut candidate = inputs.clone();
        if self.sketch_edit.as_ref().is_some_and(|edit| {
            inputs
                .bindings
                .get(&request.destination.field)
                .is_some_and(|id| edit.recipe.formula_ids().any(|used| used == *id))
        }) && inputs.operation == ActionOperation::Sketch
        {
            candidate.bindings.remove(&request.destination.field);
        }
        let result = candidate
            .parameter_draft
            .validate_source(self.document.document())
            .map_err(AppError::from)
            .and_then(|()| candidate.apply_formula(&request));
        if result.is_ok() {
            *inputs = candidate;
        }
        inputs.consumed_sequence = inputs.consumed_sequence.max(request.attempt.0);
        let diagnostic = result.err().map(|error| error.to_string());
        self.numeric_preview_suspended = diagnostic.is_some();
        let projection = match inputs.project() {
            Ok(projection) => projection,
            Err(error) => {
                log(&error.to_string());
                return Update::default();
            }
        };
        self.queue_numeric_delivery(NumericDeliveryKind::FormulaApply(
            request, projection, diagnostic,
        ))
    }
    #[cfg(test)]
    fn complete_legacy_numeric_confirm(
        &mut self,
        request: newell_ui::NumericConfirmRequest,
        rejected: bool,
    ) -> Update {
        if self.shell.as_ref().is_none_or(|shell| {
            shell.shell().operation() != request.operation
                || !shell.shell().numeric_confirm_pending(request)
                || shell.shell().numeric_session()
                    != Some((request.document, request.session, request.operation))
        }) {
            return Update::default();
        }
        let Some(inputs) = self.numeric_inputs.as_ref().filter(|i| {
            i.document == self.document.identity()
                && i.document.tree() == request.document
                && i.session == request.session
                && i.operation == request.operation
                && request.attempt.0 > self.numeric_last_attempt
        }) else {
            return Update::default();
        };
        self.numeric_last_attempt = request.attempt.0;
        let barrier = inputs.consumed_sequence >= request.through_sequence;
        let projection = match inputs.project() {
            Ok(p) => p,
            Err(error) => {
                log(&error.to_string());
                return Update::default();
            }
        };
        let mut feature = None;
        let accepted = if rejected || !barrier {
            false
        } else if request.operation != ActionOperation::Extrude {
            true
        } else {
            let result = (|| -> Result<Option<newell_model::FeatureId>, AppError> {
                let shell = self.shell.as_ref().ok_or(AppError::InvalidNumericRequest)?;
                let selections = self
                    .command_inputs
                    .as_ref()
                    .ok_or(AppError::InvalidNumericRequest)?;
                let spec = crate::parameters::build_extrude_spec(
                    &shell.shell().extrude().values,
                    inputs,
                    selections,
                )?;
                #[cfg(test)]
                {
                    self.extrude_latch = ExtrudeCommitLatch {
                        armed: true,
                        insertion: Some((request.document, selections.insertion)),
                    };
                }
                let latch = std::mem::take(&mut self.extrude_latch);
                let (_, gap) = latch.insertion.ok_or(AppError::InvalidNumericRequest)?;
                let before = match gap {
                    HistoryGap::End => None,
                    HistoryGap::Before(id) => Some(
                        self.document
                            .feature_id(id)
                            .ok_or(AppError::InvalidDocumentProjection)?,
                    ),
                };
                self.document
                    .confirm_before(&mut self.commands, spec, before)
            })();
            match result {
                Ok(Some(id)) => {
                    feature = Some(id);
                    self.numeric_committed = true;
                    self.committed_cursor = Some((request.document, OpaqueTreeId::new(id.get())));
                    true
                }
                Ok(None) => false,
                Err(error) => {
                    log(&error.to_string());
                    false
                }
            }
        };
        // Every attempt consumes the latch, including lexical/validation failures.
        #[cfg(test)]
        {
            self.extrude_latch = ExtrudeCommitLatch::default();
        }
        self.numeric_preview_suspended = !accepted;
        self.queue_numeric_delivery(NumericDeliveryKind::Confirm(
            request, accepted, feature, projection,
        ))
    }
    #[cfg(test)]
    fn sync_legacy_extrude_command(&mut self) {
        if self.numeric_preview_suspended
            || self.numeric_committed
            || self
                .numeric_delivery
                .as_ref()
                .is_some_and(|d| !matches!(d.kind, NumericDeliveryKind::View(_)))
        {
            return;
        }
        if self.projection_sync.acknowledged_document() != Some(self.document.identity().tree())
            || self.shell.as_ref().is_none_or(|s| {
                s.shell().tree_projection().document != self.document.identity().tree()
            })
        {
            return;
        }
        if self.command_inputs.is_none()
            && self.shell.as_ref().is_some_and(|s| {
                s.shell().operation() == ActionOperation::Extrude
                    && s.shell().extrude().status == ExtrudeStatus::Editing
            })
        {
            let update = self.start_command_inputs(ActionOperation::Extrude);
            self.apply(update);
        }
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        if shell.shell().operation() != ActionOperation::Extrude {
            self.commands.cancel();
            #[cfg(test)]
            {
                self.extrude_latch = ExtrudeCommitLatch::default();
            }
            return;
        }
        match shell.shell().extrude().status {
            ExtrudeStatus::Editing => {
                let Some(numeric) = self.numeric_inputs.as_ref() else {
                    return;
                };
                let Some(selections) = self.command_inputs.as_ref() else {
                    return;
                };
                let Ok(spec) = crate::parameters::build_extrude_spec(
                    &shell.shell().extrude().values,
                    numeric,
                    selections,
                ) else {
                    return;
                };
                if matches!(self.commands.state(), CommandState::Idle) {
                    self.commands.begin_extrude(&Selection::default());
                }
                if self
                    .commands
                    .edit_extrude(spec, self.document.document())
                    .is_ok()
                    && let Some(preview) = self.commands.preview()
                    && let Ok(depth) = preview
                        .inputs()
                        .depth
                        .evaluate(self.document.document().parameters())
                {
                    let _ = depth;
                    self.scene_entity = None;
                    self.pending_scene_click = None;
                }
            }
            ExtrudeStatus::Confirmed => {}
            ExtrudeStatus::Cancelled => {
                if self.command_inputs.is_none() && !self.extrude_latch.armed {
                    return;
                }
                self.numeric_cache.remove(&ActionOperation::Extrude);
                self.command_inputs = None;
                self.numeric_inputs = None;
                self.active_selection = None;
                #[cfg(test)]
                {
                    self.extrude_latch = ExtrudeCommitLatch::default();
                }
                self.commands.cancel();
                self.pending_scene_click = None;
            }
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
            if self.sketch_edit.is_none() {
                self.navigation.set_perspective(perspective);
            }
            if reset_camera && self.sketch_edit.is_none() {
                self.navigation.fit();
            }
            if normal_view && self.sketch_edit.is_some() {
                self.navigation.finish_sketch_alignment();
            }
            if normal_view
                && self.sketch_edit.is_none()
                && self
                    .navigation
                    .start_normal_to_axis_transition(Instant::now())
                && let Some(window) = &self.window
            {
                window.request_redraw();
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
        let mut invalidation_update = Update::default();
        if self.selection.document != self.document.identity() {
            self.face_selection.clear();
            self.sketch = None;
            if let Some(edit) = self.sketch_edit.take() {
                self.navigation.leave_sketch(edit.entry_camera);
            }
            self.pending_origin_click = None;
            #[cfg(test)]
            {
                self.extrude_latch = ExtrudeCommitLatch::default();
            }
            self.committed_cursor = None;
            self.commands.cancel();
            self.command_inputs = None;
            self.numeric_inputs = None;
            self.numeric_cache.clear();
            self.extrude_edit_target = None;
            self.properties_editor = None;
            self.properties_close_source = None;
            self.properties_retry_at = None;
            self.numeric_delivery = None;
            self.numeric_committed = false;
            self.active_selection = None;
            self.drop_pending_preview_scene();
            self.pending_scene = None;
            self.scene_bindings.clear();
            self.scene_unavailable();
            if let Some(runtime) = &mut self.geometry {
                if let Some(mut coordinator) = runtime.coordinator.take() {
                    coordinator.cancel();
                }
                runtime.latest_edit = None;
                runtime.bootstrap = None;
                runtime.committed_scene = None;
                runtime.displayed_scene = None;
                if let Err(error) = runtime.worker.replace_document(self.document.identity()) {
                    log(&error.to_string());
                }
            }
            if self.geometry.is_some()
                && let Err(error) = self.queue_geometry_bootstrap()
            {
                log(&error.to_string());
            }
            self.scene_entity = None;
            self.pending_scene_click = None;
            self.navigation.cancel_captures();
            self.nav_interaction.cancel();
            if let Some(shell) = &mut self.shell {
                invalidation_update = shell.invalidate_document_interactions();
            }
        }
        let rearm = matches!(
            trigger,
            ProjectionTrigger::UserActivation | ProjectionTrigger::LayoutPublished
        );
        if !self.stage_pending_scene(now, rearm) {
            return invalidation_update;
        }
        self.selection.reconcile(&self.document);
        self.reconcile_face_selection();
        if let Some(inputs) = &mut self.command_inputs {
            for value in inputs.values.values_mut() {
                let ids = value
                    .entities()
                    .iter()
                    .copied()
                    .filter(|id| self.document.document().entity(*id).is_some())
                    .collect::<Vec<_>>();
                value.replace(ids);
            }
        }
        if self
            .scene_entity
            .is_some_and(|b| self.document.document().entity(b.entity).is_none())
        {
            self.scene_entity = None;
        }
        self.update_scene_selection();
        let view = self.selection_view();
        let rows = self
            .active_selection
            .as_ref()
            .and_then(|request| self.panel_rows(request));
        let Some(shell) = &mut self.shell else {
            return Update::default();
        };
        let mut update = invalidation_update.merge(
            match self.projection_sync.poll(
                &self.document,
                &self.document_events,
                now,
                trigger,
                |projection| {
                    shell
                        .set_document_view_and_candidates(projection, view, rows)
                        .map_err(AppError::from)
                },
            ) {
                Ok(update) => update.unwrap_or_default(),
                Err(error) => {
                    log(&format!("tree refresh pending: {error}"));
                    Update::default()
                }
            },
        );
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
        if self.projection_sync.acknowledged_document() == Some(self.document.identity().tree())
            && self.shell.as_ref().is_some_and(|s| {
                s.shell().entity_selection() == &self.selection_view().selection
                    && s.shell().origin_selection() == self.origin_selection
            })
        {
            // The document publisher satisfies only a matching complete local view.
            let fields_match = self.shell.as_ref().is_some_and(|s| {
                self.selection_view()
                    .fields
                    .iter()
                    .all(|f| s.shell().selection_field_count(f.field) == f.count)
            });
            if fields_match {
                self.selection_published();
            }
        }
        update = update.merge(self.retry_selection(
            now,
            matches!(
                trigger,
                ProjectionTrigger::UserActivation | ProjectionTrigger::LayoutPublished
            ),
        ));
        update = update.merge(self.publish_pending_scene(now));
        update = update.merge(self.deliver_numeric(
            now,
            matches!(
                trigger,
                ProjectionTrigger::UserActivation | ProjectionTrigger::LayoutPublished
            ),
        ));
        update = update.merge(self.deliver_properties(
            now,
            matches!(
                trigger,
                ProjectionTrigger::UserActivation | ProjectionTrigger::LayoutPublished
            ),
        ));
        update
    }
    fn next_document_retry_deadline(&self) -> Option<Instant> {
        [
            self.projection_sync.deadline(),
            self.selection_retry_at,
            self.properties_retry_at,
            self.numeric_delivery.as_ref().and_then(|d| d.retry_at),
            self.pending_scene
                .as_ref()
                .and_then(|pending| pending.retry_at),
        ]
        .into_iter()
        .flatten()
        .min()
    }
    fn capture_properties_action(&mut self) -> Update {
        if self
            .shell
            .as_ref()
            .and_then(ShellController::pending_properties_close)
            .is_some()
            && self.properties_close_source.is_none()
        {
            self.properties_close_source = Some(self.document.version());
            self.properties_attempts = 1;
            self.properties_retry_at = Some(Instant::now() + Duration::from_millis(100));
        }
        let request = self
            .shell
            .as_mut()
            .and_then(ShellController::take_properties_open);
        request.map_or_else(Update::default, |request| {
            self.prepare_properties_editor(request)
        })
    }
    fn prepare_properties_editor(
        &mut self,
        certificate: crate::controller::CertifiedPropertiesOpen,
    ) -> Update {
        let request = certificate.request;
        if request.id.0 <= self.last_properties_id {
            return Update::default();
        }
        self.last_properties_id = request.id.0;
        let source = self
            .properties_close_source
            .take()
            .unwrap_or(self.document.version());
        self.properties_retry_at = None;
        self.properties_attempts = 0;
        if source != self.document.version()
            || self.projection_sync.acknowledged_version() != Some(source)
            || request.origin != certificate.validated_generation
            || request.target.document != source.instance.tree()
            || self
                .shell
                .as_ref()
                .is_none_or(|shell| shell.generation() != certificate.closing_generation)
        {
            return Update::default();
        }
        let newell_ui::TreeTargetKind::Operation(operation) = request.target.kind else {
            return Update::default();
        };
        let Some(feature) = self.document.feature_id(operation) else {
            return self.geometry_status(Some("Focus a feature in Features".into()));
        };
        let Some(next) = self.next_command_id.checked_add(1) else {
            return self.geometry_status(Some("Command session capacity exhausted".into()));
        };
        match self.document.document().feature(feature) {
            Some(newell_model::Feature::Sketch { recipe, .. }) => {
                let recipe = recipe.clone();
                self.next_command_id = next;
                let update = self.start_sketch(CommandInputSessionId(next));
                if !update.any() {
                    return update;
                }
                return update.merge(self.enter_sketch_edit(
                    crate::sketch::ui_plane(recipe.support),
                    Some(feature),
                    recipe,
                ));
            }
            Some(newell_model::Feature::Extrude(_)) => {}
            _ => return self.geometry_status(Some("Focus a Sketch or Extrude in Features".into())),
        }
        let prepared = crate::parameters::hydrate_extrude_inputs(
            self.document.identity(),
            CommandInputSessionId(next),
            self.document.document(),
            feature,
        );
        match prepared {
            Ok((values, numeric, selections)) => {
                self.next_command_id = next;
                self.properties_editor = Some(PendingPropertiesEditor {
                    certificate,
                    target: ExtrudeEditTarget {
                        version: source,
                        feature,
                    },
                    values,
                    numeric,
                    selections,
                });
                Update::default()
            }
            Err(error) => self.geometry_status(Some(error.to_string())),
        }
    }
    fn properties_failed(&mut self, now: Instant) {
        self.properties_attempts = self.properties_attempts.saturating_add(1);
        self.properties_retry_at =
            (self.properties_attempts == 1).then_some(now + Duration::from_millis(100));
    }
    fn deliver_properties(&mut self, now: Instant, rearm: bool) -> Update {
        let close = self
            .shell
            .as_ref()
            .and_then(ShellController::pending_properties_close)
            .is_some();
        if !close && self.properties_editor.is_none() {
            self.properties_close_source = None;
            self.properties_retry_at = None;
            return Update::default();
        }
        if self.numeric_committed {
            return Update::default();
        }
        if self.properties_attempts > 0 {
            if self.properties_retry_at.is_none() && !rearm {
                return Update::default();
            }
            if self.properties_retry_at.is_some_and(|at| now < at) {
                return Update::default();
            }
            if self.properties_retry_at.is_none() && rearm {
                self.properties_attempts = 0;
            }
        }
        let mut update = Update::default();
        if close {
            let valid = self.properties_close_source == Some(self.document.version())
                && self.projection_sync.acknowledged_version() == Some(self.document.version());
            if !valid {
                if let Some(shell) = &mut self.shell {
                    shell.discard_properties_close();
                }
                self.properties_close_source = None;
                self.properties_retry_at = None;
                return update;
            }
            let result = self
                .shell
                .as_mut()
                .map(ShellController::retry_properties_close);
            match result {
                Some(Ok(next)) => {
                    update = update.merge(next);
                    update = update.merge(self.capture_properties_action());
                }
                _ => {
                    self.properties_failed(now);
                    return update;
                }
            }
        }
        let Some(pending) = self.properties_editor.take() else {
            return update;
        };
        if pending.target.version != self.document.version()
            || self.projection_sync.acknowledged_version() != Some(pending.target.version)
            || self
                .shell
                .as_ref()
                .is_none_or(|shell| shell.generation() != pending.certificate.closing_generation)
        {
            self.properties_retry_at = None;
            return update;
        }
        let projection = match pending.numeric.project() {
            Ok(value) => value,
            Err(error) => return update.merge(self.geometry_status(Some(error.to_string()))),
        };
        let fields: Vec<_> = pending
            .selections
            .values
            .iter()
            .map(|(field, value)| SelectionFieldCount {
                field: *field,
                count: pending
                    .selections
                    .geometry_values
                    .get(field)
                    .map_or(value.entities().len(), Vec::len),
            })
            .collect();
        let result = self
            .shell
            .as_mut()
            .ok_or(newell_ui::UiError::InvalidNumericProjection)
            .and_then(|shell| shell.open_extrude_properties(pending.values, projection, &fields));
        match result {
            Ok(next) => {
                self.restore_committed_scene();
                self.extrude_edit_target = Some(pending.target);
                self.numeric_inputs = Some(pending.numeric);
                self.command_inputs = Some(pending.selections);
                self.sketch = None;
                if let Some(edit) = self.sketch_edit.take() {
                    self.navigation.leave_sketch(edit.entry_camera);
                }
                self.pending_origin_click = None;
                self.numeric_last_attempt = 0;
                self.numeric_committed = false;
                self.numeric_preview_suspended = false;
                self.numeric_delivery = None;
                self.active_selection = None;
                self.properties_retry_at = None;
                self.properties_attempts = 0;
                self.begin_geometry_command();
                self.sync_geometry_preview();
                update.merge(next)
            }
            Err(_) => {
                self.properties_editor = Some(pending);
                self.properties_failed(now);
                update
            }
        }
    }
    fn finish_shell_input(&mut self, mut update: Update) {
        update = update.merge(self.capture_properties_action());
        update = update.merge(self.deliver_properties(Instant::now(), true));
        self.observe_geometry_lifetime();
        if update.any() {
            self.numeric_preview_suspended = false;
        }
        if self
            .numeric_delivery
            .as_ref()
            .is_some_and(|d| !matches!(d.kind, NumericDeliveryKind::View(_)))
        {
            update = update.merge(self.deliver_numeric(Instant::now(), true));
            self.apply(update);
            return;
        }
        let (numeric_update, rejected) = self.process_numeric_requests();
        update = update.merge(numeric_update);
        let formula_apply = self
            .shell
            .as_mut()
            .and_then(ShellController::take_formula_apply);
        let formula_open = self
            .shell
            .as_mut()
            .and_then(ShellController::take_numeric_formula_open);
        let confirmation = self
            .shell
            .as_mut()
            .and_then(ShellController::take_numeric_confirm);
        let intent = self
            .shell
            .as_mut()
            .and_then(ShellController::take_tree_intent);
        let selection_request = self
            .shell
            .as_mut()
            .and_then(ShellController::take_selection_request);
        let started = self
            .shell
            .as_mut()
            .and_then(ShellController::take_command_start);
        if let Some(operation) = started {
            update = update.merge(self.start_command_inputs(operation));
        }
        if let Some(intent) = self
            .shell
            .as_mut()
            .and_then(ShellController::take_sketch_intent)
        {
            update = update.merge(self.handle_sketch_intent(intent));
        }
        if let Some(intent) = intent
            && self
                .shell
                .as_ref()
                .is_some_and(|s| s.generation() == intent.generation())
            && intent.target().document == self.document.identity().tree()
        {
            if let newell_ui::TreeIntentKind::Select { edit } = intent.kind() {
                if let newell_ui::TreeTargetKind::OriginReference(reference) = intent.target().kind
                {
                    update = update.merge(self.select_origin_reference(reference, *edit));
                } else if let Some(entities) =
                    resolve_tree_selection(&self.document, intent.target())
                {
                    update = update.merge(self.handle_selection_request(SelectionRequest {
                        document: self.document.identity(),
                        op: (*edit).into(),
                        entities,
                    }));
                }
            } else {
                match self
                    .document
                    .apply_tree_edit(intent.target(), intent.kind())
                {
                    Ok(true)
                        if matches!(
                            intent.kind(),
                            newell_ui::TreeIntentKind::SetVisibility { .. }
                        ) =>
                    {
                        let is_sketch = match intent.target().kind {
                            newell_ui::TreeTargetKind::Operation(id) => self
                                .document
                                .feature_id(id)
                                .and_then(|id| self.document.document().feature(id))
                                .is_some_and(|feature| {
                                    matches!(feature, newell_model::Feature::Sketch { .. })
                                }),
                            _ => false,
                        };
                        if !is_sketch {
                            self.republish_scene_visibility();
                        }
                    }
                    Ok(_) => {}
                    Err(error) => log(&error.to_string()),
                }
            }
        }
        if let Some(request) = selection_request {
            update = update.merge(self.handle_selection_ui(request));
        }
        if let Some(request) = formula_apply {
            update = update.merge(self.complete_formula_apply(request));
        } else if let Some(request) = formula_open {
            update = update.merge(self.complete_numeric_formula(request));
        } else if let Some(request) = confirmation {
            update = update.merge(self.complete_numeric_confirm(request, rejected));
        } else {
            update = update.merge(self.publish_numeric_view());
        }
        self.sync_extrude_command();
        if self
            .shell
            .as_ref()
            .is_some_and(|s| !s.shell().command_open())
            && self.command_inputs.is_some()
            && !self.numeric_committed
            && self.numeric_delivery.is_none()
        {
            if let Some(numeric) = self.numeric_inputs.as_ref() {
                self.numeric_cache.remove(&numeric.operation);
            }
            self.command_inputs = None;
            self.numeric_inputs = None;
            self.active_selection = None;
            update = update.merge(self.publish_selection());
        }
        self.sync_sketch_numeric();
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
            self.navigation.cancel_captures();
            self.nav_interaction.cancel();
            self.apply(result.update);
        }
        result.handled
    }

    /// Publishes one layout/basis to hit testing and rendering, independently of UI ink.
    fn sync_navigation_widgets(&mut self) {
        let Some(shell) = &self.shell else {
            return;
        };
        let viewport = scene_viewport(shell);
        let metrics = shell.metrics();
        let pixel = |r: newell_ui::GridRect| PixelRect {
            x: f64::from(r.col() * metrics.cell_width_px()),
            y: f64::from(r.row() * metrics.cell_height_px()),
            width: f64::from(r.width() * metrics.cell_width_px()),
            height: f64::from(r.height() * metrics.cell_height_px()),
        };
        let scene = PixelRect {
            x: f64::from(viewport.x),
            y: f64::from(viewport.y),
            width: f64::from(viewport.width),
            height: f64::from(viewport.height),
        };
        let surface = PixelRect {
            x: 0.,
            y: 0.,
            width: f64::from(self.last_size.width),
            height: f64::from(self.last_size.height),
        };
        let mut layout = layout_navigation(
            scene,
            surface,
            metrics,
            shell.shell().view_panel_visible(3),
            shell.shell().navigator_kind(),
            shell.shell().action_panel_rect().map(pixel),
        );
        let ui = scene_occlusions(shell);
        let ui_masks = shell
            .scene_occlusions()
            .into_iter()
            .map(pixel)
            .collect::<Vec<_>>();
        let changed = self.nav_geometry.as_ref().is_none_or(|g| {
            g.layout.tile != layout.tile
                || g.layout.kind != layout.kind
                || g.q != f64::from(metrics.cell_width_px()) / 8.
        }) || self.nav_ui_masks != ui_masks;
        if changed {
            self.nav_generation = self.nav_generation.wrapping_add(1);
            self.nav_interaction.cancel();
            self.navigation.cancel();
        }
        layout.generation = self.nav_generation;
        self.nav_ui_masks = ui_masks.clone();
        let occlusions = ui.clone();
        let before_layout = self.navigation.revision();
        self.navigation.set_layout(viewport, occlusions.clone());
        if self.navigation.revision() != before_layout
            && let Some(edit) = &mut self.sketch_edit
        {
            edit.clear_gesture();
        }
        let (right, up, forward) = self.navigation.camera().basis();
        let geometry = NavGeometry::new(layout, NavBasis { right, up, forward }, metrics);
        let hovered = axis_hover(
            geometry.as_ref(),
            self.scene_pointer.map(|p| (p.x, p.y)),
            !self.occluded
                && !self.resize.is_active()
                && !self.native_resizing
                && shell.navigation_admission(
                    (self.last_size.width, self.last_size.height),
                    self.resizable(),
                ),
            self.nav_interaction.active() || self.navigation.has_capture(),
        );
        let hovered_cube = cube_hover(
            geometry.as_ref(),
            self.scene_pointer.map(|p| (p.x, p.y)),
            !self.occluded
                && !self.resize.is_active()
                && !self.native_resizing
                && shell.navigation_admission(
                    (self.last_size.width, self.last_size.height),
                    self.resizable(),
                ),
            self.nav_interaction.active() || self.navigation.has_capture(),
        );
        if self.nav_hover != hovered || self.nav_cube_hover != hovered_cube {
            self.nav_hover = hovered;
            self.nav_cube_hover = hovered_cube;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        let snapshot = geometry.clone().map(|geometry| NavSnapshot {
            geometry,
            ui_masks,
            surface: [self.last_size.width, self.last_size.height],
            camera_revision: self.navigation.revision(),
            hovered,
            hovered_cube,
        });
        if let Some(renderer) = &mut self.renderer {
            if let Err(error) = renderer.set_navigation(snapshot) {
                log(&error.to_string());
                self.nav_geometry = None;
                self.nav_hover = None;
                self.nav_cube_hover = None;
                self.nav_interaction.cancel();
                self.navigation.set_layout(viewport, ui.clone());
                renderer.set_scene_occlusions(ui);
                return;
            }
            renderer.set_scene_occlusions(occlusions);
            if let Some(camera) = self.navigation.snapshot() {
                renderer.set_scene_camera(camera);
            }
        }
        self.nav_geometry = geometry;
        self.nav_interaction
            .validate(self.nav_geometry.as_ref(), self.navigation.revision());
    }

    /// Uploads what `update` says changed and asks for a redraw.
    fn apply(&mut self, mut update: Update) {
        self.sync_view_choices();
        self.sync_navigation_widgets();
        if self.sketch_edit.is_some() {
            update = update.merge(self.publish_sketch_ready());
        }
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
                if let Some(camera) = self.navigation.snapshot() {
                    renderer.set_scene_camera(camera);
                }
            }
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Targets own input even when hollow; uncovered tile pixels remain scene input.
    fn scene_navigation_at(&self, point: (f64, f64)) -> bool {
        self.navigation.has_capture()
            || self
                .nav_geometry
                .as_ref()
                .is_none_or(|g| !g.reserves(point))
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
        let icon = if !self.resize.is_active()
            && !self.native_resizing
            && !self.navigation.is_dragging()
            && shell.navigation_admission((size.width, size.height), self.resizable())
            && self.scene_pointer.is_some_and(|p| {
                self.nav_geometry
                    .as_ref()
                    .and_then(|g| g.hit((p.x, p.y)))
                    .is_some()
            }) {
            CursorIcon::Pointer
        } else {
            icon
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
        if self.sketch_edit.is_some() && self.navigation.has_secondary_pan() {
            return;
        }
        self.pending_origin_click = None;
        if let Some(edit) = &mut self.sketch_edit {
            edit.press = None;
        }
        self.sync_navigation_widgets();
        if !self.resize.is_active()
            && !self.native_resizing
            && !self.navigation.is_dragging()
            && self.shell.as_ref().is_some_and(|shell| {
                shell.navigation_admission(
                    (self.last_size.width, self.last_size.height),
                    self.resizable(),
                )
            })
            && let Some(pointer) = self.scene_pointer
            && let Some(g) = &self.nav_geometry
            && g.reserves((pointer.x, pointer.y))
        {
            self.pending_scene_click = None;
            if self.sketch_edit.is_some() {
                return;
            }
            self.navigation.cancel_captures();
            self.nav_interaction.press(
                g,
                (pointer.x, pointer.y),
                self.window.as_ref().map_or(1., |w| w.scale_factor()),
                self.navigation.revision(),
            );
            if let Some(shell) = &mut self.shell {
                let update = shell.clear_hover();
                self.apply(update);
            }
            return;
        }

        self.navigation.cancel_captures();
        self.nav_interaction.cancel();
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
                if self.sketch_edit.is_some() {
                    if scene_input
                        && cursor.is_some()
                        && !self.native_resizing
                        && let Some(pointer) = self.scene_pointer
                    {
                        self.begin_sketch_click((pointer.x, pointer.y), window.scale_factor());
                    }
                    return;
                }
                let origin_reference = if scene_input && cursor.is_some() && !self.native_resizing {
                    self.scene_pointer.and_then(|pointer| {
                        self.navigation
                            .pick_origin_reference((pointer.x, pointer.y))
                            .map(|reference| (pointer, reference))
                    })
                } else {
                    None
                };
                if let Some((pointer, reference)) = origin_reference {
                    self.pending_scene_click = None;
                    if let Some(shell) = &self.shell {
                        self.pending_origin_click = Some(PendingOriginClick {
                            document: self.document.identity(),
                            generation: shell.generation(),
                            mesh_revision: self.navigation.scene().revision(),
                            navigation_revision: self.navigation.revision(),
                            reference,
                            start: (pointer.x, pointer.y),
                            display_scale: window.scale_factor(),
                            edit: SelectionEdit::modifiers(
                                self.modifiers.shift_key(),
                                self.modifiers.control_key()
                                    || (cfg!(target_os = "macos") && self.modifiers.super_key()),
                            ),
                        });
                    }
                } else if scene_input
                    && cursor.is_some()
                    && !self.native_resizing
                    && let Some(pointer) = self.scene_pointer
                    && self.navigation.begin_primary_orbit(
                        (pointer.x, pointer.y),
                        window.scale_factor(),
                        true,
                    )
                    && let Some(shell) = &self.shell
                {
                    self.pending_scene_click = Some(PendingSceneClick {
                        document: self.document.identity(),
                        generation: shell.generation(),
                        mesh_revision: self.navigation.scene().revision(),
                        edit: SelectionEdit::modifiers(
                            self.modifiers.shift_key(),
                            self.modifiers.control_key()
                                || (cfg!(target_os = "macos") && self.modifiers.super_key()),
                        ),
                    });
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

impl ApplicationHandler<AppEvent> for App {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::GeometryReady => {
                let update = self.drain_sketch_solve().merge(self.drain_worker_reply());
                self.apply(update);
            }
        }
    }
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
        }
        self.shell = Some(shell);
        self.window = Some(window.clone());
        if let Err(error) = self.queue_geometry_bootstrap() {
            return self.fail(event_loop, error);
        }
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
        // Startup reconciliation can move the viewport after its first camera layout. Recenter
        // the initial scene once against the final viewport while preserving its framing.
        if self.navigation.center_scene()
            && let (Some(renderer), Some(camera)) = (&mut self.renderer, self.navigation.snapshot())
        {
            renderer.set_scene_camera(camera);
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
        if matches!(
            &event,
            WindowEvent::ScaleFactorChanged { .. }
                | WindowEvent::Resized(_)
                | WindowEvent::Focused(false)
                | WindowEvent::CursorLeft { .. }
                | WindowEvent::Occluded(true)
                | WindowEvent::CloseRequested
        ) {
            self.nav_interaction.cancel();
        }
        self.sync_navigation_widgets();
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
                if let Some(edit) = &mut self.sketch_edit {
                    edit.clear_gesture();
                }
                window.set_cursor(CursorIcon::Default);
                self.scene_pointer = None;
                self.pending_origin_click = None;
                self.navigation.cancel();
                self.nav_interaction.cancel();
                self.native_resizing = false;
                self.cancel_resize(CancelReason::FocusLost);
                self.control.cancel();
                let update = if let Some(shell) = &mut self.shell {
                    shell
                        .pointer_moved(None)
                        .merge(shell.pointer_released())
                        .merge(shell.numeric_focus_lost())
                } else {
                    Update::default()
                };
                self.finish_shell_input(update);
            }
            WindowEvent::Moved(_) => {
                // A move may be the middle of a drag across monitors: the hosting monitor is
                // evaluated once the window has been still (see `SETTLE`).
                self.moved_at = Some(Instant::now());
            }
            WindowEvent::Resized(size) => {
                self.nav_interaction.cancel();
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
                if let Some(edit) = &mut self.sketch_edit {
                    edit.reset_marker_cycle_after_motion((position.x, position.y), window.scale_factor());
                    window.request_redraw();
                }
                if self.nav_interaction.active() {
                    self.nav_interaction.advance((position.x, position.y));
                    self.update_os_cursor();
                    return;
                }
                let point = ui_point(position.x, position.y);
                let camera_changed = self.navigation.advance_pointer((position.x, position.y));
                if !self.navigation.primary_orbit_pending() {
                    self.pending_scene_click = None;
                }
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
                self.pending_origin_click = None;
                if let Some(edit) = &self.sketch_edit {
                    edit.clear_live_snap();
                    window.request_redraw();
                }
                self.navigation.cancel();
                self.nav_interaction.cancel();
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
                if self.nav_interaction.active() {
                    let p = self
                        .scene_pointer
                        .map_or((f64::NAN, f64::NAN), |p| (p.x, p.y));
                    if let Some(direction) = self.nav_interaction.release(
                        self.nav_geometry.as_ref(),
                        p,
                        self.navigation.revision(),
                    ) {
                        self.navigation
                            .start_view_transition(direction, Instant::now());
                        window.request_redraw();
                    }
                    self.sync_navigation_widgets();
                    return;
                }
                self.native_resizing = false;
                let point = self
                    .scene_pointer
                    .map_or((f64::NAN, f64::NAN), |p| (p.x, p.y));
                if self.sketch_edit.is_some() {
                    let update = self.finish_sketch_click(point);
                    self.apply(update);
                }
                let certificate = self.navigation.finish_primary(point);
                let origin_selection = self.finish_origin_reference_click(point);
                let selection_update = origin_selection
                    .unwrap_or_else(|| self.finish_scene_selection(point, certificate));
                self.apply(selection_update);
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
                    let hover_update = self
                        .shell
                        .as_mut()
                        .map_or(Update::default(), ShellController::pointer_released);
                    self.apply(hover_update);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: button @ (MouseButton::Right | MouseButton::Middle),
                ..
            } => {
                if let Some(cursor) = self.scene_pointer {
                    let eligible = !self.nav_interaction.active()
                        && self.scene_navigation_at((cursor.x, cursor.y))
                        && !self.resize.is_active()
                        && !self.native_resizing
                        && self
                            .shell
                            .as_ref()
                            .is_some_and(ShellController::scene_input_enabled);
                    let point = (cursor.x, cursor.y);
                    let eligible = if let Some(edit) = &self.sketch_edit {
                        eligible && edit.press.is_none() && self.sketch_auxiliary_admitted(point)
                    } else {
                        eligible
                    };
                    if eligible {
                        self.navigation.finish_sketch_alignment();
                    }
                    if let Some(kind) = DragKind::from_auxiliary_button(button) {
                        if kind == DragKind::SecondaryPan {
                            let scale = window.scale_factor();
                            if self.navigation.begin_secondary_pan(point, scale, eligible)
                                && let (Some(edit), Some(shell)) = (&mut self.sketch_edit, &self.shell)
                            {
                                edit.secondary_press = Some(crate::sketch::Click {
                                    point, scale,
                                    navigation: self.navigation.revision(),
                                    layout: shell.generation(),
                                });
                            }
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
                if button == MouseButton::Right && self.sketch_edit.is_some() {
                    let point = self.scene_pointer.map_or((f64::NAN, f64::NAN), |p| (p.x, p.y));
                    let update = self.finish_sketch_secondary(point);
                    self.apply(update);
                } else if let Some(kind) = DragKind::from_auxiliary_button(button) {
                    self.navigation.release(kind);
                }
            }
            WindowEvent::MouseWheel { delta, phase, .. } => {
                if self.resize.is_active() || self.native_resizing {
                    if matches!(delta, MouseScrollDelta::PixelDelta(_)) {
                        self.navigation.reject_trackpad_pan_scroll(phase);
                    }
                    self.sync_navigation_widgets();
                    return;
                }
                let point = self
                    .scene_pointer
                    .map_or((f64::NAN, f64::NAN), |cursor| (cursor.x, cursor.y));
                // UI-owned scrolling keeps first refusal, including a zero-delta start phase.
                let panel_steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(delta) => (delta.y / 40.0) as f32,
                };
                if self.scroll_shell_panel(panel_steps.round() as i32) {
                    if matches!(delta, MouseScrollDelta::PixelDelta(_)) {
                        self.navigation.reject_trackpad_pan_scroll(phase);
                    }
                    self.sync_navigation_widgets();
                    return;
                }
                let eligible = !self.nav_interaction.active()
                    && self.scene_navigation_at(point)
                    && !self.resize.is_active()
                    && !self.native_resizing
                    && self
                        .shell
                        .as_ref()
                        .is_some_and(ShellController::scene_input_enabled);
                if let MouseScrollDelta::PixelDelta(pixels) = delta {
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
                        self.sync_navigation_widgets();
                        return;
                    }
                    // macOS winit emits phased PixelDelta events for trackpad scrolling. A
                    // Started stream over the scene uses the same frozen-depth pan as mouse
                    // drags; unphased PixelDelta and LineDelta continue through wheel zoom.
                    if eligible {
                        self.navigation.finish_sketch_alignment();
                    }
                    if let Some(changed) = self.navigation.advance_trackpad_pan_scroll(
                        (pixels.x, pixels.y),
                        phase,
                        point,
                        window.scale_factor(),
                        eligible,
                    ) {
                        if changed {
                            if let (Some(renderer), Some(camera)) =
                                (&mut self.renderer, self.navigation.snapshot())
                            {
                                renderer.set_scene_camera(camera);
                            }
                            window.request_redraw();
                        }
                        self.sync_navigation_widgets();
                        return;
                    }
                }
                let steps = zoom_steps(delta, window.scale_factor());
                let stream = match delta {
                    MouseScrollDelta::LineDelta(..) => None,
                    MouseScrollDelta::PixelDelta(_) => Some((ZoomSource::Scroll, phase)),
                };
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
                let point = self
                    .scene_pointer
                    .map_or((f64::NAN, f64::NAN), |cursor| (cursor.x, cursor.y));
                let eligible = !self.nav_interaction.active()
                    && self.scene_navigation_at(point)
                    && !self.resize.is_active()
                    && !self.native_resizing
                    && self
                        .shell
                        .as_ref()
                        .is_some_and(ShellController::scene_input_enabled);
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
                    self.nav_interaction.cancel();
                }
                self.occluded = occluded;
                if !occluded {
                    window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                if let Some((point, local)) = self.sketch_pointer()
                    && let Some(edit) = &self.sketch_edit
                {
                    let camera = self.navigation.camera();
                    let exposed = self.sketch_exposed_regions(camera.viewport);
                    edit.update_snap(
                        local,
                        point,
                        window.scale_factor(),
                        camera,
                        self.modifiers.control_key(),
                        &exposed,
                    );
                    window.request_redraw();
                }
                if let Some(shell) = &mut self.shell {
                    shell.set_selection_modifiers(
                        self.modifiers.shift_key(),
                        self.modifiers.control_key()
                            || (cfg!(target_os = "macos") && self.modifiers.super_key()),
                    );
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let scene_capture =
                    self.navigation.has_capture() || self.pending_origin_click.is_some();
                self.navigation.cancel();
                self.nav_interaction.cancel();
                self.pending_scene_click = None;
                if event.logical_key == Key::Named(NamedKey::Escape)
                    && self
                        .shell
                        .as_ref()
                        .is_some_and(|s| s.shell().selection().is_open())
                {
                    if let Some(shell) = &mut self.shell {
                        let update = shell.key(newell_ui::ShellKey::Escape);
                        self.finish_shell_input(update);
                    }
                    return;
                }
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
                if self.sketch_edit.is_none()
                    && event.logical_key == Key::Named(NamedKey::Home)
                    && self.scene_pointer.is_some_and(|p| {
                        self.navigation.visible((p.x, p.y)) && self.scene_navigation_at((p.x, p.y))
                    })
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
                if event.logical_key == Key::Named(NamedKey::Escape)
                    && scene_capture
                    && self.live_sketch().is_some()
                {
                    self.pending_origin_click = None;
                    return;
                }
                if event.logical_key == Key::Named(NamedKey::Tab)
                    && !scene_capture
                    && let Some(pointer) = self.scene_pointer
                {
                    let point = (pointer.x, pointer.y);
                    if self.sketch_auxiliary_admitted(point) {
                        let camera = self.navigation.camera();
                        let exposed = self.sketch_exposed_regions(camera.viewport);
                        if let Some(edit) = &mut self.sketch_edit {
                            edit.refresh_reference_picks(self.document.document());
                            if let Some(message) = edit.cycle_marker(
                                point, window.scale_factor(), camera, &exposed, self.modifiers.shift_key(),
                            ) {
                                let update = self.geometry_status(Some(message));
                                self.apply(update);
                                window.request_redraw();
                                return;
                            }
                        }
                    }
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
                // Layout reconciliation may update the camera lens/viewport. Publish
                // grid inputs only after the scene camera has been reconciled.
                self.sync_navigation_widgets();
                let grid = self
                    .shell
                    .as_ref()
                    .filter(|shell| shell.shell().view_panel_visible(4))
                    .and_then(|shell| {
                        let dark = shell.shell().theme() == newell_ui::ThemeChoice::Dark;
                        if let Some(edit) = &self.sketch_edit {
                            self.navigation.sketch_grid_snapshot(
                                dark,
                                crate::sketch::ui_plane(edit.recipe.support),
                            )
                        } else {
                            self.navigation.grid_snapshot(dark)
                        }
                    });
                let strokes = self.sketch_strokes();
                if let Some(renderer) = &mut self.renderer {
                    renderer.set_grid(grid);
                    renderer.set_sketch_strokes(strokes, window.scale_factor() as f32);
                    if let Some(camera) = self.navigation.snapshot() {
                        renderer.set_scene_camera(camera);
                    }
                }
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
        self.sync_navigation_widgets();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.clone() else {
            return event_loop.set_control_flow(ControlFlow::Wait);
        };
        // macOS may not deliver Resized(0, 0) on minimise, so also ask the window.
        // Hidden windows wait for an OS event instead of refreshing unseen status cells.
        let hidden = self.occluded || window.is_minimized().unwrap_or(false);
        if hidden {
            self.navigation.cancel_view_transition();
        }
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
            if self.navigation.advance_view_transition(now) {
                self.sync_navigation_widgets();
                window.request_redraw();
            }
            if let Some(deadline) = self.navigation.transition_deadline(now) {
                wake = Some(wake.map_or(deadline, |other: Instant| other.min(deadline)));
            }
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

    #[test]
    fn fresh_application_starts_with_an_empty_model_and_idle_command_controller() {
        let app = App::new();
        let projection =
            crate::document::project_document(app.document.identity(), app.document.document())
                .unwrap();

        assert!(app.document.document().features().is_empty());
        assert!(projection.features.is_empty());
        assert!(projection.elements.is_empty());
        assert_eq!(app.commands.state(), &CommandState::Idle);
        assert!(app.commands.preview().is_none());
    }

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
                    .enumerate()
                    .find(|(col, window)| {
                        *window == pattern
                            && shell.grid().get(*col as u32, row).is_some_and(|cell| {
                                cell.widget_id.is_some() || cell.shared_border.is_some()
                            })
                    })
                    .map(|(col, _)| UiPoint {
                        x: col as u32 * 4 + 2,
                        y: row * 9 + 4,
                    })
            })
            .expect(needle);
        shell.pointer_moved(Some(point));
        let update = shell.pointer_pressed();
        shell.pointer_released();
        app.finish_shell_input(update);
    }
    pub(super) fn key(app: &mut App, key: ShellKey) {
        let update = app.shell.as_mut().unwrap().key(key);
        app.finish_shell_input(update);
    }

    pub(super) fn sketch_row(app: &App, needle: &str) -> String {
        let grid = app.shell.as_ref().unwrap().grid();
        (0..grid.rows())
            .map(|row| {
                (0..grid.cols())
                    .map(|col| match grid.get(col, row).unwrap().content {
                        CellContent::Glyph(c) => c,
                        CellContent::Empty => ' ',
                    })
                    .collect::<String>()
            })
            .find(|line| line.contains(needle))
            .expect(needle)
    }

    fn begin_sketch(app: &mut App) {
        click(app, "Datum");
        click(app, "[S]ketch");
        assert!(sketch_row(app, "Workbench").contains("Datum"));
        assert_eq!(app.shell.as_ref().unwrap().shell().action_width(), 27);
    }

    fn sketch_viewport_click(app: &mut App, reference: newell_ui::OriginReference) {
        let shell = app.shell.as_ref().unwrap();
        let viewport = scene_viewport(shell);
        app.navigation.set_layout(viewport, scene_occlusions(shell));
        let point = (viewport.y..viewport.y + viewport.height)
            .flat_map(|y| (viewport.x..viewport.x + viewport.width).map(move |x| (x, y)))
            .find(|&(x, y)| {
                app.navigation
                    .pick_origin_reference((f64::from(x), f64::from(y)))
                    == Some(reference)
            })
            .expect("visible origin reference");
        let shell = app.shell.as_mut().unwrap();
        shell.pointer_moved(Some(UiPoint {
            x: point.0,
            y: point.1,
        }));
        let point = (f64::from(point.0), f64::from(point.1));
        app.pending_origin_click = Some(PendingOriginClick {
            document: app.document.identity(),
            generation: shell.generation(),
            mesh_revision: app.navigation.scene().revision(),
            navigation_revision: app.navigation.revision(),
            reference,
            start: point,
            display_scale: 1.0,
            edit: SelectionEdit::Replace,
        });
        let update = app.finish_origin_reference_click(point).unwrap();
        app.apply(update);
    }

    #[test]
    fn sketch_draw_and_commit_two_supports_without_consuming_cancelled_name() {
        use newell_model::{Feature, sketch::SketchPrimitive};
        let mut app = live_app();
        let entry = app.navigation.camera();
        let (sender, wake) = std::sync::mpsc::channel();
        app.enable_geometry_worker(move || {
            let _ = sender.send(());
        })
        .unwrap();
        app.queue_geometry_bootstrap().unwrap();
        for (plane, cancel) in [
            (OriginPlane::XY, false),
            (OriginPlane::XZ, true),
            (OriginPlane::YZ, false),
        ] {
            app.origin_selection = vec![match plane {
                OriginPlane::XY => newell_ui::OriginReference::XY,
                OriginPlane::XZ => newell_ui::OriginReference::XZ,
                OriginPlane::YZ => newell_ui::OriginReference::YZ,
            }];
            begin_sketch(&mut app);
            click(&mut app, "OK");
            if plane == OriginPlane::XY {
                let opened = app.sketch_edit.as_ref().unwrap().source;
                super::real_extrude_tests::pump(&mut app, &wake).unwrap();
                assert!(app.document.revision() > opened.revision);
                assert_eq!(
                    app.sketch_edit.as_ref().unwrap().source,
                    app.document.version()
                );
            }
            app.navigation.finish_sketch_alignment();
            let [u, v, n] = plane.basis().map(|axis| axis.map(f64::from));
            let (right, up, forward) = app.navigation.camera().basis();
            for (a, b) in [(right, u), (up, v), (forward, n.map(|x| -x))] {
                assert!((crate::camera::dot(a, b) - 1.0).abs() < 1e-12);
            }
            let mut points = Vec::new();
            let viewport = app.navigation.camera().viewport;
            'points: for y in (viewport.y + 18..viewport.y + viewport.height).step_by(18) {
                for x in (viewport.x + 8..viewport.x + viewport.width).step_by(12) {
                    let shell = app.shell.as_mut().unwrap();
                    shell.pointer_moved(Some(UiPoint { x, y }));
                    if shell.scene_input_enabled()
                        && points.last().is_none_or(|p: &UiPoint| p.x != x && p.y != y)
                    {
                        points.push(UiPoint { x, y });
                        if points.len() == 2 {
                            break 'points;
                        }
                        break;
                    }
                }
            }
            assert_eq!(points.len(), 2);
            for label in ["[L]ine", "[R]ectangle"] {
                click(&mut app, "Draw");
                click(&mut app, label);
                for p in &points {
                    let shell = app.shell.as_mut().unwrap();
                    shell.pointer_moved(Some(*p));
                    let point = (f64::from(p.x), f64::from(p.y));
                    app.begin_sketch_click(point, 1.0);
                    let update = app.finish_sketch_click(point);
                    app.apply(update);
                }
            }
            let edit = app.sketch_edit.as_ref().unwrap();
            assert_eq!(edit.recipe.primitives.len(), 2);
            assert!(matches!(
                edit.recipe.primitives[0].1,
                SketchPrimitive::Line { .. }
            ));
            assert!(matches!(
                edit.recipe.primitives[1].1,
                SketchPrimitive::Rectangle { .. }
            ));
            let revision = app.document.revision();
            click(&mut app, if cancel { "Cancel" } else { "OK" });
            assert_eq!(app.document.revision(), revision + u64::from(!cancel));
            assert!(app.sketch_edit.is_none());
            assert!(sketch_row(&app, "Workbench").contains("Datum"));
        }
        let features = app.document.document().features();
        assert_eq!(features.len(), 2);
        assert_eq!(features[0].1.name(), "Sketch1");
        assert_eq!(features[1].1.name(), "Sketch2");
        assert!(matches!(features[1].1, Feature::Sketch { .. }));
        assert_eq!(app.navigation.camera().orientation, entry.orientation);
    }

    #[test]
    fn sketch_construction_tools_preview_reject_and_apply_through_the_menu() {
        use newell_model::sketch::SketchPrimitive;
        let mut app = live_app();
        app.origin_selection = vec![newell_ui::OriginReference::XY];
        begin_sketch(&mut app);
        click(&mut app, "OK");
        app.navigation.finish_sketch_alignment();
        let camera = app.navigation.camera();
        let a = [0., 0.];
        let b = [8., 0.];
        let c = [1., 3.];
        let d = [9., 4.];
        for (index, (menu, label, inputs)) in [
            ("Draw", "[P]oint", &[a][..]),
            ("Draw", "Polygon", &[a, b][..]),
            ("Draw", "Slot", &[a, b, c][..]),
            ("Draw", "Center Rectangle", &[a, c][..]),
            ("Draw", "3-Points Rectangle", &[a, b, c][..]),
            ("Draw", "[C]ircle", &[a, b][..]),
            ("Draw", "3-Pts Circle", &[a, b, c][..]),
            ("Draw", "Ellipse", &[a, b, c][..]),
            ("Curve", "3 Points Arc", &[a, b, c][..]),
            ("Curve", "Center Point Arc", &[a, b, c][..]),
            ("Curve", "Bezier", &[a, b, c, d][..]),
            ("Curve", "Elliptical Arc", &[a, b, c, b, d][..]),
            ("Curve", "Conic", &[a, b, c][..]),
            ("Curve", "Spline", &[a, b, c, d][..]),
        ]
        .into_iter()
        .enumerate()
        {
            click(&mut app, menu);
            click(&mut app, label);
            let edit = app.sketch_edit.as_mut().unwrap();
            let (&last, initial) = inputs.split_last().unwrap();
            for &point in initial {
                edit.click(point, (0., 0.), 1., camera).unwrap();
            }
            let before = edit.recipe.clone();
            let saved = edit.strokes(camera, None).len();
            assert!(edit.strokes(camera, Some(last)).len() > saved, "{label}");
            assert_eq!(edit.recipe, before);
            if !matches!(label, "Bezier" | "Spline") && !initial.is_empty() {
                assert!(edit.click(a, (0., 0.), 1., camera).is_err(), "{label}");
                assert_eq!(edit.recipe, before);
            }
            if matches!(label, "Bezier" | "Spline") {
                assert!(!edit.escape());
                assert_eq!(edit.recipe, before);
                for &point in initial {
                    edit.click(point, (0., 0.), 1., camera).unwrap();
                }
            }
            edit.click(last, (0., 0.), 1., camera).unwrap();
            assert_eq!(edit.recipe.primitives.len(), index + 1);
            assert!(matches!(
                edit.recipe.primitives[index].1,
                SketchPrimitive::Draw(_) | SketchPrimitive::Rectangle { .. }
            ));
            assert!(!edit.escape());
        }
        let update = app.publish_sketch_ready();
        app.apply(update);
        click(&mut app, "OK");
        assert!(app.sketch_edit.is_none());
        assert!(app.sketch_strokes().len() > 8);
    }

    #[test]
    fn sketch_formula_changes_polygon_count_and_rejects_a_non_integer() {
        let mut app = live_app();
        app.origin_selection = vec![newell_ui::OriginReference::XY];
        begin_sketch(&mut app);
        click(&mut app, "OK");
        app.navigation.finish_sketch_alignment();
        click(&mut app, "Draw");
        click(&mut app, "Polygon");
        super::numeric_binding_tests::formula(&mut app, newell_ui::SKETCH_COUNT, "2 + 3");
        let camera = app.navigation.camera();
        let edit = app.sketch_edit.as_mut().unwrap();
        edit.click([0., 0.], (0., 0.), 1., camera).unwrap();
        assert_eq!(edit.strokes(camera, Some([8., 0.])).len(), 5);
        edit.click([8., 0.], (0., 0.), 1., camera).unwrap();
        super::numeric_binding_tests::formula(&mut app, newell_ui::SKETCH_COUNT, "3.5");
        let edit = app.sketch_edit.as_mut().unwrap();
        let prior = edit.recipe.clone();
        edit.click([0., 0.], (0., 0.), 1., camera).unwrap();
        assert!(edit.click([8., 0.], (0., 0.), 1., camera).is_err());
        assert_eq!(edit.recipe, prior);
        key(&mut app, ShellKey::Escape);
        click(&mut app, "OK");
        assert!(app.sketch_edit.is_none());
        assert!(matches!(&app.document.document().features()[0].1,
            newell_model::Feature::Sketch { recipe, .. }
            if recipe.formula_ids().next().is_some_and(|id| app.document.document().parameters().definition(id).is_some())
        ));
    }

    #[test]
    fn sketch_text_routes_keys_and_places_font_contours() {
        let mut app = live_app();
        app.origin_selection = vec![newell_ui::OriginReference::XY];
        begin_sketch(&mut app);
        click(&mut app, "OK");
        app.navigation.finish_sketch_alignment();
        click(&mut app, "Draw");
        click(&mut app, "Text");
        key(&mut app, ShellKey::Char('O'));
        let camera = app.navigation.camera();
        let edit = app.sketch_edit.as_mut().unwrap();
        edit.click([0., 0.], (0., 0.), 1., camera).unwrap();
        assert!(edit.click([0., 0.], (0., 0.), 1., camera).is_err());
        assert!(edit.strokes(camera, Some([8., 0.])).len() > 8);
        edit.click([8., 0.], (0., 0.), 1., camera).unwrap();
        assert_eq!(edit.recipe.primitives.len(), 1);
    }

    #[test]
    fn sketch_corner_tools_preview_and_trim_both_source_lines() {
        for name in ["Fillet", "Chamfer"] {
            let mut app = live_app();
            app.origin_selection = vec![newell_ui::OriginReference::XY];
            begin_sketch(&mut app);
            click(&mut app, "OK");
            app.navigation.finish_sketch_alignment();
            click(&mut app, "Draw");
            click(&mut app, "[L]ine");
            let camera = app.navigation.camera();
            let edit = app.sketch_edit.as_mut().unwrap();
            for point in [[8., 0.], [0., 0.], [0., 8.]] {
                edit.click(point, (0., 0.), 1., camera).unwrap();
            }
            click(&mut app, "Modify");
            click(&mut app, name);
            let edit = app.sketch_edit.as_mut().unwrap();
            let horizontal = camera.project([6., 0., 0.]).unwrap();
            let vertical = camera.project([0., 6., 0.]).unwrap();
            edit.click([6., 0.], horizontal, 1., camera).unwrap();
            let before = edit.recipe.clone();
            assert!(edit.click([6., 0.], horizontal, 1., camera).is_err());
            assert_eq!(edit.recipe, before);
            edit.click([0., 6.], vertical, 1., camera).unwrap();
            assert!(edit.strokes(camera, None).len() > 2);
            edit.click([0., 6.], vertical, 1., camera).unwrap();
            assert_eq!(edit.recipe.primitives.len(), 3);
            assert_eq!(edit.recipe.primitives[0].0, before.primitives[0].0);
            assert_eq!(edit.recipe.primitives[1].0, before.primitives[1].0);
        }
    }

    #[test]
    fn sketch_control_point_refinement_preserves_the_curve_and_key() {
        use newell_model::sketch::{DrawGeometry, SketchPrimitive};
        let mut app = live_app();
        app.origin_selection = vec![newell_ui::OriginReference::XY];
        begin_sketch(&mut app);
        click(&mut app, "OK");
        app.navigation.finish_sketch_alignment();
        click(&mut app, "Curve");
        click(&mut app, "Bezier");
        let camera = app.navigation.camera();
        let controls = [[0., 0.], [1., 3.], [5., 3.], [6., 0.]];
        let edit = app.sketch_edit.as_mut().unwrap();
        for point in controls {
            edit.click(point, (0., 0.), 1., camera).unwrap();
        }
        click(&mut app, "Curve");
        click(&mut app, "Add Control Points");
        let edit = app.sketch_edit.as_mut().unwrap();
        let [x, y] = DrawGeometry::evaluate_bezier(controls, 0.5);
        let point = camera.project([x, y, 0.]).unwrap();
        assert!(edit.click([x, y], (-100., -100.), 1., camera).is_err());
        edit.click([x, y], point, 1., camera).unwrap();
        assert!(edit.strokes(camera, None).len() > 64);
        edit.click([x, y], point, 1., camera).unwrap();
        let elevated = DrawGeometry::elevate_cubic(controls);
        assert_eq!(
            edit.recipe.primitives,
            vec![(0, SketchPrimitive::Draw(elevated))]
        );
        let elevated = elevated.control_polygon().unwrap();
        for i in 0..=16 {
            let t = f64::from(i) / 16.;
            let [a, b] = DrawGeometry::evaluate_bezier(controls, t);
            let [x, y] = DrawGeometry::evaluate_bezier(elevated, t);
            assert!((x - a).hypot(y - b) < 1e-12);
        }
    }

    #[test]
    fn sketch_spline_handles_and_curved_move_route_through_the_action_box() {
        use newell_model::sketch::{DrawGeometry, SketchPrimitive};
        let mut app = live_app();
        app.origin_selection = vec![newell_ui::OriginReference::XY];
        begin_sketch(&mut app);
        click(&mut app, "OK");
        app.navigation.finish_sketch_alignment();
        click(&mut app, "Curve");
        click(&mut app, "Spline");
        click(&mut app, "Endpoint tangents");
        let camera = app.navigation.camera();
        let edit = app.sketch_edit.as_mut().unwrap();
        for point in [[0., 0.], [2., 3.], [4., 1.], [6., 0.], [0., 2.], [7., -1.]] {
            edit.click(point, (0., 0.), 1., camera).unwrap();
        }
        assert!(matches!(
            edit.recipe.primitives.as_slice(),
            [(
                0,
                SketchPrimitive::Draw(DrawGeometry::Spline {
                    tangents: Some([[0., 2.], [1., -1.]]),
                    ..
                })
            )]
        ));
        click(&mut app, "Pattern");
        click(&mut app, "Transform");
        click(&mut app, "Move source");
        let edit = app.sketch_edit.as_mut().unwrap();
        let hit = camera.project([2., 3., 0.]).unwrap();
        edit.click([2., 3.], hit, 1., camera).unwrap();
        edit.click([10., 0.], hit, 1., camera).unwrap();
        edit.click([12., 3.], hit, 1., camera).unwrap();
        assert!(
            matches!(edit.recipe.primitives.as_slice(), [(0, SketchPrimitive::Draw(
            DrawGeometry::Spline { fit, .. }))] if fit[0] == [10.,0.])
        );
    }

    #[test]
    fn sketch_patterns_keep_the_master_and_resolve_instances_in_order() {
        use newell_model::sketch::SketchPrimitive;
        for name in ["Linear Pattern", "Circular Pattern", "Transform"] {
            let mut app = live_app();
            app.origin_selection = vec![newell_ui::OriginReference::XY];
            begin_sketch(&mut app);
            click(&mut app, "OK");
            app.navigation.finish_sketch_alignment();
            click(&mut app, "Draw");
            click(&mut app, "[L]ine");
            let camera = app.navigation.camera();
            let a = [0., 0.];
            let b = [8., 0.];
            let edit = app.sketch_edit.as_mut().unwrap();
            edit.click(a, (0., 0.), 1., camera).unwrap();
            edit.click(b, (0., 0.), 1., camera).unwrap();
            click(&mut app, "Pattern");
            click(&mut app, name);
            let edit = app.sketch_edit.as_mut().unwrap();
            let hit = camera.project([4., 0., 0.]).unwrap();
            edit.click([4., 0.], hit, 1., camera).unwrap();
            edit.click(a, hit, 1., camera).unwrap();
            if name == "Circular Pattern" {
                edit.click(b, hit, 1., camera).unwrap();
            }
            let before = edit.recipe.clone();
            assert!(edit.click(a, hit, 1., camera).is_err());
            assert_eq!(edit.recipe, before);
            let end = if name == "Circular Pattern" {
                [8., 3.]
            } else {
                [1., 4.]
            };
            assert!(edit.strokes(camera, Some(end)).len() > 1);
            edit.click(end, hit, 1., camera).unwrap();
            assert_eq!(edit.recipe.primitives[0], before.primitives[0]);
            if name == "Transform" {
                assert_eq!(edit.recipe.primitives.len(), 2);
            } else {
                assert_eq!(edit.recipe.primitives.len(), 6);
                for (index, &(_, instance)) in edit.recipe.primitives.iter().enumerate().skip(1) {
                    assert!(
                        matches!(instance, SketchPrimitive::PatternInstance { master: 0, placement, .. }
                        if usize::from(placement) == index)
                    );
                    assert!(edit.recipe.resolved(instance).is_ok());
                }
            }
            let update = app.publish_sketch_ready();
            app.apply(update);
            click(&mut app, "OK");
            assert!(app.sketch_edit.is_none());
        }
    }

    #[test]
    fn sketch_tangent_arc_uses_a_unique_line_endpoint() {
        use newell_model::sketch::{DrawGeometry, SketchPrimitive};
        let mut app = live_app();
        app.origin_selection = vec![newell_ui::OriginReference::XY];
        begin_sketch(&mut app);
        click(&mut app, "OK");
        app.navigation.finish_sketch_alignment();
        click(&mut app, "Draw");
        click(&mut app, "[L]ine");
        let camera = app.navigation.camera();
        let edit = app.sketch_edit.as_mut().unwrap();
        edit.click([0., 0.], (0., 0.), 1., camera).unwrap();
        edit.click([8., 0.], (0., 0.), 1., camera).unwrap();
        click(&mut app, "Curve");
        click(&mut app, "Tangent Arc");
        let edit = app.sketch_edit.as_mut().unwrap();
        let tip = camera.project([8., 0., 0.]).unwrap();
        edit.click([8., 0.], tip, 1., camera).unwrap();
        let before = edit.recipe.clone();
        assert!(edit.click([12., 0.], tip, 1., camera).is_err());
        assert_eq!(edit.recipe, before);
        assert!(edit.strokes(camera, Some([12., 4.])).len() > 1);
        edit.click([12., 4.], tip, 1., camera).unwrap();
        assert!(matches!(
            edit.recipe.primitives[1].1,
            SketchPrimitive::Draw(DrawGeometry::Arc { center, sweep, .. })
                if (center[0] - 8.).hypot(center[1] - 4.) < 1e-12
                    && (sweep - std::f64::consts::FRAC_PI_2).abs() < 1e-12
        ));
    }

    #[test]
    fn sketch_line_edits_and_mirror_preview_and_reject_atomically() {
        use newell_model::sketch::SketchPrimitive;
        let mut app = live_app();
        app.origin_selection = vec![newell_ui::OriginReference::XY];
        begin_sketch(&mut app);
        click(&mut app, "OK");
        app.navigation.finish_sketch_alignment();
        let camera = app.navigation.camera();
        for (menu, label, invalid, good, count) in [
            ("Modify", "Split", [0., 0.], [3., 0.], 4),
            ("Modify", "Offset", [3., 0.], [3., 2.], 4),
            ("Modify", "Trim", [4., 0.], [2., 0.], 3),
            ("Modify", "Extend", [-1., 0.], [7., 0.], 3),
            ("Pattern", "Mirror", [0., 1.], [8., 1.], 4),
        ] {
            click(&mut app, menu);
            click(&mut app, label);
            let edit = app.sketch_edit.as_mut().unwrap();
            let before = edit.recipe.clone();
            edit.recipe.primitives.clear();
            for (start, end) in [
                ([0., 0.], [8., 0.]),
                ([4., -3.], [4., 3.]),
                ([10., -3.], [10., 3.]),
            ] {
                edit.recipe.primitives.push((
                    edit.recipe.primitives.len() as u64 + 10,
                    SketchPrimitive::Line { start, end },
                ));
            }
            let target = camera.project([2., 0., 0.]).unwrap();
            edit.click([2., 0.], target, 1., camera).unwrap();
            if label == "Mirror" {
                edit.click([0., 1.], target, 1., camera).unwrap();
            }
            let source = edit.recipe.clone();
            assert!(edit.click(invalid, target, 1., camera).is_err(), "{label}");
            assert_eq!(edit.recipe, source);
            let normal = edit.strokes(camera, None).len();
            let preview = edit.strokes(camera, Some(good)).len();
            if matches!(label, "Split" | "Offset" | "Mirror") {
                assert!(preview > normal);
            }
            edit.click(good, target, 1., camera).unwrap();
            assert_eq!(edit.recipe.primitives.len(), count, "{label}");
            assert_eq!(edit.recipe.primitives[0].0, 10);
            edit.recipe = before;
        }
        click(&mut app, "Cancel");
        assert!(app.document.document().features().is_empty());
    }

    #[test]
    fn sketch_projected_cubic_brep_uses_native_polynomial_controls()
    -> Result<(), Box<dyn std::error::Error>> {
        use newell_model::{
            Document, Length, geometry::BaseGeometryRecipe, sketch::SketchPrimitive,
        };
        let mut app = live_app();
        let mut document = Document::default();
        let controls = [[0., -2.], [2., 3.], [4., 3.], [6., -2.]];
        document.add_base_recipe_for_test(
            "Cubic B-Rep",
            BaseGeometryRecipe::TestCubic {
                control: controls,
                depth: Length::millimetres(1.)?,
                transform: [
                    [1., 0., 0., 0.],
                    [0., 1., 0., 0.],
                    [0., 0., 1., 0.],
                    [0., 0., 0., 1.],
                ],
            },
        )?;
        let (send, wake) = std::sync::mpsc::channel();
        app.enable_geometry_worker(move || {
            let _ = send.send(());
        })?;
        app.document.replace(document)?;
        app.queue_geometry_bootstrap()?;
        super::real_extrude_tests::pump(&mut app, &wake)?;
        app.origin_selection = vec![newell_ui::OriginReference::XY];
        begin_sketch(&mut app);
        click(&mut app, "OK");
        app.navigation.finish_sketch_alignment();
        click(&mut app, "Cubic B-Rep");
        click(&mut app, "Reference");
        click(&mut app, "Projected");
        let camera = app.navigation.camera();
        let edit = app.sketch_edit.as_mut().ok_or("missing sketch")?;
        assert!(edit.strokes(camera, None).len() > 20);
        edit.click([0., 0.], (0., 0.), 1., camera)?;
        let projected: Vec<_> = edit
            .recipe
            .primitives
            .iter()
            .map(|&(_, item)| {
                app.document
                    .document()
                    .resolve_sketch_reference(newell_model::sketch::SketchSupport::XY, item)
            })
            .collect();
        let matches = |a: [[f64; 2]; 4], b: [[f64; 2]; 4]| {
            a.into_iter()
                .zip(b)
                .all(|(a, b)| (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-8)
        };
        assert!(projected.iter().any(|item| matches!(item,
            Ok(SketchPrimitive::Draw(newell_model::sketch::DrawGeometry::Bezier { controls: p }))
                if matches(*p, controls) || matches(*p, [controls[3],controls[2],controls[1],controls[0]])
        )), "projected controls: {projected:?}");
        let update = app.publish_sketch_ready();
        app.apply(update);
        click(&mut app, "OK");
        assert!(app.sketch_edit.is_none());
        app.origin_selection = vec![newell_ui::OriginReference::XZ];
        begin_sketch(&mut app);
        click(&mut app, "OK");
        app.navigation.finish_sketch_alignment();
        click(&mut app, "Cubic B-Rep");
        click(&mut app, "Reference");
        click(&mut app, "Intersect");
        let camera = app.navigation.camera();
        let edit = app.sketch_edit.as_mut().ok_or("missing section draft")?;
        assert!(!edit.strokes(camera, None).is_empty());
        edit.click([0., 0.], (0., 0.), 1., camera)?;
        assert!(
            edit.recipe
                .primitives
                .iter()
                .any(|&(_, item)| matches!(item, SketchPrimitive::BrepFaceSection { .. }))
        );
        Ok(())
    }

    #[test]
    fn sketch_projected_brep_edges_use_the_real_worker_generation()
    -> Result<(), Box<dyn std::error::Error>> {
        use newell_model::sketch::SketchPrimitive;
        let (mut app, _) = super::real_extrude_tests::runtime_app()?;
        app.origin_selection = vec![newell_ui::OriginReference::XY];
        begin_sketch(&mut app);
        click(&mut app, "OK");
        app.navigation.finish_sketch_alignment();
        click(&mut app, "Base Solid");
        click(&mut app, "Reference");
        click(&mut app, "Projected");
        let camera = app.navigation.camera();
        let edit = app.sketch_edit.as_mut().ok_or("missing Sketch draft")?;
        assert!(!edit.strokes(camera, None).is_empty());
        edit.click([0., 0.], (0., 0.), 1., camera)?;
        assert!(
            edit.recipe
                .primitives
                .iter()
                .all(|(_, primitive)| matches!(primitive, SketchPrimitive::BrepReference { .. }))
        );
        let update = app.publish_sketch_ready();
        app.apply(update);
        click(&mut app, "OK");
        assert!(app.sketch_edit.is_none());
        Ok(())
    }

    #[test]
    fn sketch_reference_commands_preview_cancel_and_place_typed_links() {
        use newell_model::sketch::{ReferenceMode, SketchPrimitive};
        let mut app = live_app();
        let mut source = crate::sketch::empty_recipe(OriginPlane::XZ);
        source.primitives.push((
            7,
            SketchPrimitive::Line {
                start: [2., -4.],
                end: [5., 4.],
            },
        ));
        app.document
            .commit_sketch(app.document.version(), None, None, source)
            .unwrap();
        app.finish_shell_input(Update::default());
        app.origin_selection = vec![newell_ui::OriginReference::XY];
        begin_sketch(&mut app);
        click(&mut app, "OK");
        app.navigation.finish_sketch_alignment();
        click(&mut app, "Sketch1");
        let camera = app.navigation.camera();
        for (label, mode) in [
            ("Projected", ReferenceMode::Projected),
            ("Intersect", ReferenceMode::Intersect),
        ] {
            click(&mut app, "Reference");
            click(&mut app, label);
            let edit = app.sketch_edit.as_mut().unwrap();
            assert!(!edit.strokes(camera, None).is_empty());
            assert!(!edit.escape());
            assert!(edit.strokes(camera, None).is_empty());
            click(&mut app, "Reference");
            click(&mut app, label);
            let edit = app.sketch_edit.as_mut().unwrap();
            edit.click([0., 0.], (0., 0.), 1., camera).unwrap();
            assert!(matches!(edit.recipe.primitives.last(),
                Some((_, SketchPrimitive::Reference { element: 7, mode: actual, .. })) if *actual == mode));
        }
        let update = app.publish_sketch_ready();
        app.apply(update);
        click(&mut app, "OK");
        assert!(app.sketch_edit.is_none());
        assert_eq!(app.document.document().features().len(), 2);
    }

    #[test]
    fn sketch_insert_copies_a_hidden_same_plane_recipe_without_linking() {
        use newell_model::sketch::SketchPrimitive;
        let mut app = live_app();
        let mut source = crate::sketch::empty_recipe(OriginPlane::XY);
        source.primitives.push((
            2,
            SketchPrimitive::Line {
                start: [0., 0.],
                end: [2., 0.],
            },
        ));
        app.document
            .commit_sketch(app.document.version(), None, None, source.clone())
            .unwrap();
        app.finish_shell_input(Update::default());
        click(&mut app, "■ Sketch1");
        app.origin_selection = vec![newell_ui::OriginReference::XY];
        begin_sketch(&mut app);
        click(&mut app, "OK");
        app.navigation.finish_sketch_alignment();
        click(&mut app, "Sketch1");
        click(&mut app, "Reference");
        click(&mut app, "Insert");
        let camera = app.navigation.camera();
        let edit = app.sketch_edit.as_mut().unwrap();
        assert_eq!(edit.strokes(camera, Some([0., 4.])).len(), 1);
        edit.click([0., 4.], (0., 0.), 1., camera).unwrap();
        assert_eq!(
            edit.recipe.primitives,
            vec![(
                0,
                SketchPrimitive::Line {
                    start: [0., 4.],
                    end: [2., 4.],
                }
            )]
        );
        let update = app.publish_sketch_ready();
        app.apply(update);
        click(&mut app, "OK");
        assert!(app.sketch_edit.is_none());
        assert_eq!(app.document.document().features().len(), 2);
    }

    #[test]
    fn sketch_segment_commands_solve_on_worker_and_reopen_records()
    -> Result<(), Box<dyn std::error::Error>> {
        use newell_model::sketch::SketchPrimitive;
        let mut app = live_app();
        let (sender, wake) = std::sync::mpsc::channel();
        app.enable_geometry_worker(move || {
            let _ = sender.send(());
        })?;
        app.origin_selection = vec![newell_ui::OriginReference::XY];
        begin_sketch(&mut app);
        click(&mut app, "OK");
        app.navigation.finish_sketch_alignment();
        let camera = app.navigation.camera();
        let viewport = camera.viewport;
        let screen = (
            f64::from(viewport.x + viewport.width / 2),
            f64::from(viewport.y + viewport.height / 2),
        );
        let center = app
            .navigation
            .sketch_point(OriginPlane::XY, screen)
            .ok_or("missing sketch point")?;
        app.sketch_edit
            .as_mut()
            .ok_or("missing draft")?
            .recipe
            .primitives
            .push((
                0,
                SketchPrimitive::Line {
                    start: [center[0] - 2.0, center[1] - 2.0],
                    end: [center[0] + 2.0, center[1] + 2.0],
                },
            ));
        let update = app.publish_sketch_ready();
        app.apply(update);

        let apply =
            |app: &mut App, label, point: (f64, f64)| -> Result<(), Box<dyn std::error::Error>> {
                click(app, "Constraints");
                click(app, label);
                app.shell
                    .as_mut()
                    .ok_or("missing shell")?
                    .pointer_moved(Some(UiPoint {
                        x: point.0 as u32,
                        y: point.1 as u32,
                    }));
                for _ in 0..2 {
                    app.begin_sketch_click(point, 1.0);
                    let update = app.finish_sketch_click(point);
                    app.apply(update);
                }
                if app
                    .sketch_edit
                    .as_ref()
                    .is_none_or(|edit| edit.solve.is_none())
                {
                    return Err("command did not queue a solve".into());
                }
                wake.recv_timeout(Duration::from_secs(3))?;
                let update = app.drain_sketch_solve();
                app.apply(update);
                Ok(())
            };

        apply(&mut app, "[D]imension", screen)?;
        let line = app
            .sketch_edit
            .as_ref()
            .ok_or("missing solved draft")?
            .recipe
            .primitives[0]
            .1;
        let (start, end) = line.line().ok_or("missing solved line")?;
        assert!(((end[0] - start[0]).hypot(end[1] - start[1]) - 10.0).abs() < 1e-7);
        apply(&mut app, "Driven Length", screen)?;
        let recipe = app
            .sketch_edit
            .as_ref()
            .ok_or("missing measured draft")?
            .recipe
            .clone();
        assert_eq!(recipe.primitives[0].1, line);
        assert_eq!(recipe.segment_equations.len(), 1);
        assert_eq!(recipe.driven_dimensions.len(), 1);
        assert!(app.sketch_strokes().len() > 2);
        click(&mut app, "OK");
        click(&mut app, "Sketch1");
        click(&mut app, "Utilities");
        click(&mut app, "Properties");
        assert_eq!(
            app.sketch_edit
                .as_ref()
                .ok_or("missing reopened draft")?
                .recipe,
            recipe
        );
        Ok(())
    }

    #[test]
    fn sketch_magnetic_join_survives_driving_length_and_properties()
    -> Result<(), Box<dyn std::error::Error>> {
        use newell_model::sketch::{SketchJoin, SketchPointRef};
        let mut app = live_app();
        let (sender, wake) = std::sync::mpsc::channel();
        app.enable_geometry_worker(move || {
            let _ = sender.send(());
        })?;
        app.origin_selection = vec![newell_ui::OriginReference::XY];
        begin_sketch(&mut app);
        click(&mut app, "OK");
        app.navigation.finish_sketch_alignment();
        let viewport = app.navigation.camera().viewport;
        let x = f64::from(viewport.x);
        let y = f64::from(viewport.y + viewport.height / 2);
        let start = (x + 24., y - 75.);
        let approximate_end = (x + 97., y - 72.);
        let end = (approximate_end.0, start.1);
        let arc_end = (end.0 + 36., end.1 + 27.);
        let pointer_click = |app: &mut App, point: (f64, f64)| {
            app.scene_pointer = Some(PhysicalPosition::new(point.0, point.1));
            app.shell.as_mut().unwrap().pointer_moved(Some(UiPoint {
                x: point.0 as u32,
                y: point.1 as u32,
            }));
            app.sketch_strokes();
            app.begin_sketch_click(point, 1.);
            assert!(
                app.sketch_edit.as_ref().unwrap().press.is_some(),
                "unadmitted pointer {point:?}"
            );
            let update = app.finish_sketch_click(point);
            app.apply(update);
        };
        let drain = |app: &mut App| -> Result<(), Box<dyn std::error::Error>> {
            assert!(
                app.sketch_edit
                    .as_ref()
                    .is_some_and(|edit| edit.solve.is_some())
            );
            wake.recv_timeout(Duration::from_secs(3))?;
            let update = app.drain_sketch_solve();
            app.apply(update);
            Ok(())
        };
        click(&mut app, "Draw");
        click(&mut app, "[L]ine");
        pointer_click(&mut app, start);
        pointer_click(&mut app, approximate_end);
        assert!(
            app.sketch_edit
                .as_ref()
                .unwrap()
                .recipe
                .primitives
                .is_empty()
        );
        drain(&mut app)?;
        click(&mut app, "Curve");
        click(&mut app, "Tangent Arc");
        pointer_click(&mut app, (end.0 + 3., end.1 + 2.));
        pointer_click(&mut app, arc_end);
        assert_eq!(app.sketch_edit.as_ref().unwrap().recipe.primitives.len(), 1);
        drain(&mut app)?;
        let recipe = app.sketch_edit.as_ref().unwrap().recipe.clone();
        assert_eq!(recipe.segment_equations.len(), 1);
        assert_eq!(recipe.joins.len(), 2);
        assert!(recipe.joins.values().any(|join| matches!(
            join,
            SketchJoin::Tangent {
                point: SketchPointRef::Start(1),
                source: SketchPointRef::End(0),
                ..
            }
        )));
        click(&mut app, "OK");
        click(&mut app, "Sketch1");
        click(&mut app, "Utilities");
        click(&mut app, "Properties");
        assert_eq!(app.sketch_edit.as_ref().unwrap().recipe, recipe);
        app.navigation.finish_sketch_alignment();
        click(&mut app, "Constraints");
        click(&mut app, "[D]imension");
        let camera = app.navigation.camera();
        let (a, b) = recipe.primitives[0].1.line().unwrap();
        let mid = camera.project([(a[0] + b[0]) / 2., a[1], 0.]).unwrap();
        pointer_click(&mut app, mid);
        pointer_click(&mut app, (mid.0, mid.1 + 32.));
        drain(&mut app)?;
        let solved = &app.sketch_edit.as_ref().unwrap().recipe;
        assert_eq!(solved.joins, recipe.joins);
        let (_, new_end) = solved.primitives[0].1.line().unwrap();
        assert!((new_end[0] - b[0]).abs() > 1e-4);
        let (contact, tangency) = solved.primitives[1].1.endpoint_tangents().unwrap()[0];
        assert!((contact[0] - new_end[0]).hypot(contact[1] - new_end[1]) < 1e-7);
        assert!(tangency[1].abs() / tangency[0].hypot(tangency[1]) < 1e-8);
        let solved = solved.clone();
        click(&mut app, "OK");
        click(&mut app, "Sketch1");
        click(&mut app, "Utilities");
        click(&mut app, "Properties");
        assert_eq!(app.sketch_edit.as_ref().unwrap().recipe, solved);
        Ok(())
    }

    #[test]
    fn sketch_properties_cancels_then_replaces_under_the_same_identity() {
        use newell_model::{Feature, sketch::SketchPrimitive};
        let mut app = live_app();
        let mut recipe = crate::sketch::empty_recipe(OriginPlane::XY);
        recipe.primitives.push((
            0,
            SketchPrimitive::Line {
                start: [0., 0.],
                end: [1., 1.],
            },
        ));
        let version = app.document.version();
        app.document
            .commit_sketch(version, None, None, recipe.clone())
            .unwrap();
        app.finish_shell_input(Update::default());
        let id = app.document.document().features()[0].0;
        assert_eq!(app.sketch_strokes().len(), 1);
        let geometry_token = app.document.geometry_token();
        click(&mut app, "■ Sketch1");
        assert!(sketch_row(&app, "Sketch1").contains("□ Sketch1"));
        assert!(app.sketch_strokes().is_empty());
        assert_eq!(app.document.geometry_token(), geometry_token);
        let original_version = app.document.version();
        for cancel in [true, false] {
            click(&mut app, "Sketch1");
            assert!(
                app.shell
                    .as_ref()
                    .unwrap()
                    .shell()
                    .focused_features_operation()
                    .is_some()
            );
            assert_eq!(
                app.projection_sync.acknowledged_version(),
                Some(app.document.version())
            );
            click(&mut app, "Utilities");
            assert!(
                app.shell
                    .as_ref()
                    .unwrap()
                    .shell()
                    .focused_features_operation()
                    .is_some()
            );
            click(&mut app, "Properties");
            assert_eq!(app.sketch_strokes().len(), 1);
            let edit = app.sketch_edit.as_mut().unwrap();
            assert_eq!(edit.target, Some(id));
            assert_eq!(edit.recipe, recipe);
            edit.recipe.primitives[0].1 = SketchPrimitive::Line {
                start: [0., 0.],
                end: [2., 2.],
            };
            click(&mut app, if cancel { "Cancel" } else { "OK" });
            assert_eq!(
                app.document.revision(),
                original_version.revision + u64::from(!cancel)
            );
            let features = app.document.document().features();
            assert_eq!(features.len(), 1);
            let (actual_id, feature) = &features[0];
            let (name, actual) = match feature {
                Feature::Sketch { name, recipe, .. } => Some((name, recipe)),
                _ => None,
            }
            .expect("expected a Sketch");
            assert_eq!(*actual_id, id);
            assert_eq!(name, "Sketch1");
            assert_eq!(*actual == recipe, cancel);
            assert!(app.sketch_strokes().is_empty());
        }
        click(&mut app, "□ Sketch1");
        assert!(sketch_row(&app, "Sketch1").contains("■ Sketch1"));
        assert_eq!(app.sketch_strokes().len(), 1);
        assert_eq!(app.document.geometry_token(), geometry_token);
        click(&mut app, "Sketch1");
        click(&mut app, "Utilities");
        click(&mut app, "Properties");
        assert_eq!(
            app.sketch_strokes().len(),
            1,
            "editing replaces the saved strokes"
        );
        click(&mut app, "Cancel");
    }

    fn confirm_and_exit_sketch(app: &mut App, plane: OriginPlane) {
        let before = app.document.version();
        let camera = app.navigation.camera();
        let menu = sketch_row(app, "File");
        click(app, "OK");
        assert_eq!(app.live_sketch().unwrap().mode, SketchMode::Active(plane));
        assert!(
            sketch_row(app, "Workbench")
                .contains("Workbench │ Draw │ Curve │ Modify │ Pattern │ Reference │ Constraints")
        );
        assert_eq!(sketch_row(app, "File"), menu);
        click(app, "Draw");
        assert!(sketch_row(app, "╭─Draw").contains("╭─Draw───────────────╮"));
        assert!(sketch_row(app, "[L]ine").contains("│ [L]ine             │"));
        key(app, ShellKey::Escape);
        assert!(app.live_sketch().is_some());
        key(app, ShellKey::Escape);
        assert!(app.live_sketch().is_none());
        assert!(sketch_row(app, "Workbench").contains("Datum"));
        assert_eq!(app.document.version(), before);
        assert_eq!(app.navigation.camera(), camera);
        assert!(app.document.document().features().is_empty());
        assert!(app.selection.entities().is_empty());
    }

    #[test]
    fn sketch_plane_first_uses_origin_selection_and_switches_command_bar_only_on_ok() {
        let mut app = live_app();
        sketch_viewport_click(&mut app, newell_ui::OriginReference::XY);
        begin_sketch(&mut app);
        assert!(sketch_row(&app, "Plane").contains("│ Plane   [XY            ]│"));
        assert_eq!(OriginPlane::XY.basis(), [[1, 0, 0], [0, 1, 0], [0, 0, 1]]);
        confirm_and_exit_sketch(&mut app, OriginPlane::XY);
    }

    #[test]
    fn sketch_first_uses_origin_hit_and_rejects_point_and_axis() {
        let mut app = live_app();
        begin_sketch(&mut app);
        assert!(sketch_row(&app, "Plane").contains("│ Plane   [Select…       ]│"));
        for reference in [newell_ui::OriginReference::O, newell_ui::OriginReference::X] {
            sketch_viewport_click(&mut app, reference);
            assert_eq!(app.live_sketch().unwrap().mode, SketchMode::Setup(None));
        }
        assert!(!app.handle_sketch_intent(SketchIntent::Confirm).any());
        sketch_viewport_click(&mut app, newell_ui::OriginReference::XZ);
        assert!(sketch_row(&app, "Plane").contains("[XZ            ]"));
        let field = app.shell.as_ref().unwrap().shell().interaction().focused;
        key(&mut app, ShellKey::Enter);
        assert!(app.active_selection.is_some());
        key(&mut app, ShellKey::Escape);
        assert_eq!(
            app.shell.as_ref().unwrap().shell().interaction().focused,
            field
        );
        assert_eq!(OriginPlane::XZ.basis(), [[1, 0, 0], [0, 0, 1], [0, -1, 0]]);
        confirm_and_exit_sketch(&mut app, OriginPlane::XZ);
    }

    #[test]
    fn sketch_field_first_replacement_and_cancel_share_one_draft() {
        let mut app = live_app();
        begin_sketch(&mut app);
        click(&mut app, "[Select…");
        let initial = app.active_selection.clone().unwrap();
        assert!(
            matches!(initial.destination, SelectionDestination::Field { field, .. }
            if field.operation == ActionOperation::Sketch)
        );
        assert!(initial.applied.is_empty());
        key(&mut app, ShellKey::Escape);
        assert_eq!(app.live_sketch().unwrap().mode, SketchMode::Setup(None));
        let field_focus = app.shell.as_ref().unwrap().shell().interaction().focused;
        key(&mut app, ShellKey::Enter);
        click(&mut app, "□ XY");
        click(&mut app, "OK");
        assert!(sketch_row(&app, "Plane").contains("[XY            ]"));
        assert_eq!(
            app.shell.as_ref().unwrap().shell().interaction().focused,
            field_focus
        );
        click(&mut app, "[XY");
        click(&mut app, "□ YZ");
        key(&mut app, ShellKey::Escape);
        assert!(sketch_row(&app, "Plane").contains("[XY            ]"));
        key(&mut app, ShellKey::Enter);
        click(&mut app, "□ YZ");
        click(&mut app, "OK");
        assert!(sketch_row(&app, "Plane").contains("[YZ            ]"));
        assert!(app.origin_selection.is_empty());
        assert_eq!(OriginPlane::YZ.basis(), [[0, 1, 0], [0, 0, 1], [1, 0, 0]]);
        click(&mut app, "Cancel");
        assert!(app.live_sketch().is_none());
        assert!(sketch_row(&app, "Workbench").contains("Datum"));
        begin_sketch(&mut app);
        assert!(sketch_row(&app, "Plane").contains("Select…"));
        click(&mut app, "Cancel");
        assert!(app.live_sketch().is_none());

        begin_sketch(&mut app);
        click(&mut app, "[Select…");
        let stale = newell_ui::SelectionCompletion {
            session: initial.session,
            document: initial.document,
            destination: initial.destination,
            outcome: SelectionOutcome::Confirmed(vec![initial.rows[0].entity]),
        };
        assert!(
            !app.handle_selection_ui(SelectionUiRequest::Complete(stale))
                .any()
        );
        click(&mut app, "□ YZ");
        click(&mut app, "OK");
        confirm_and_exit_sketch(&mut app, OriginPlane::YZ);
    }

    #[test]
    fn origin_reference_viewport_and_tree_routes_share_one_selection_identity() {
        let mut app = App::new();
        app.shell = Some(ShellController::new(82, 40, GridMetrics::new(4, 9).unwrap()).unwrap());
        let update = app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        app.apply(update);
        let viewport = SceneViewport::new(0, 0, 640, 480);
        app.navigation.set_layout(viewport, Vec::new());
        let camera_before = app.navigation.camera();
        let origin = camera_before.project([0.0; 3]).unwrap();
        assert_eq!(
            app.navigation.pick_origin_reference(origin),
            Some(newell_ui::OriginReference::O)
        );
        let update =
            app.select_origin_reference(newell_ui::OriginReference::O, SelectionEdit::Replace);
        assert_eq!(app.navigation.camera(), camera_before);
        app.apply(update);
        assert_eq!(app.origin_selection, [newell_ui::OriginReference::O]);
        assert!(app.selection.entities().is_empty());
        let shell = app.shell.as_ref().unwrap().shell();
        assert!(
            shell
                .origin_selection()
                .contains(&newell_ui::OriginReference::O)
        );
        assert!(
            shell.is_entity_target_selected(shell.tree_projection().target(
                newell_ui::TreeTargetKind::OriginReference(newell_ui::OriginReference::O)
            ))
        );
        let camera_after_selection = app.navigation.camera();
        assert!(app.pending_scene_click.is_none());

        click(&mut app, "YZ");
        assert_eq!(app.origin_selection, [newell_ui::OriginReference::YZ]);
        let shell = app.shell.as_ref().unwrap().shell();
        assert!(
            shell.is_entity_target_selected(shell.tree_projection().target(
                newell_ui::TreeTargetKind::OriginReference(newell_ui::OriginReference::YZ)
            ))
        );
        assert_eq!(app.navigation.camera(), camera_after_selection);
    }
    fn depth(app: &mut App) {
        click(app, "0.0 mm");
        key(app, ShellKey::Edit(newell_ui::EditKey::SelectAll));
        key(app, ShellKey::Char('1'));
        key(app, ShellKey::Char('0'));
        key(app, ShellKey::Enter);
        assert_eq!(
            app.numeric_inputs.as_ref().unwrap().values[&newell_ui::NumericFieldKey {
                operation: ActionOperation::Extrude,
                key: 4
            }]
                .quantity
                .value_in(newell_model::Unit::Millimetre)
                .unwrap(),
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
            let update = app.handle_selection_request(SelectionRequest {
                document: app.document.identity(),
                op: SelectionOp::Replace,
                entities: vec![id],
            });
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
            // First user activation captures the already selected insertion anchor.
            if enabled {
                click(&mut app, "View");
                click(&mut app, "Color");
                key(&mut app, ShellKey::Escape);
            }
            app.finish_shell_input(Update::default());
            depth(&mut app);
            // A deliberately stale/forward command reference still fails the model
            // preflight even though the candidate chooser correctly filters it out.
            app.command_inputs
                .as_mut()
                .unwrap()
                .values
                .get_mut(&newell_ui::SelectionFieldKey {
                    operation: ActionOperation::Extrude,
                    key: 2,
                })
                .unwrap()
                .replace([id]);
            app.finish_shell_input(Update::default());
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
            x: (shell.shell().nav_width() - 1) * 4 + 2,
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
    fn replacement_cancels_capture_before_any_projection_has_been_acknowledged() {
        let mut app = App::new();
        app.shell = Some(
            ShellController::with_shell(
                ShellState::new_application(82, 40),
                GridMetrics::new(4, 9).unwrap(),
            )
            .unwrap(),
        );
        let now = Instant::now();
        app.projection_sync.fail_next_projection = 1;
        let update = app.poll_document_projection(now, ProjectionTrigger::Notification);
        assert!(!update.grid);
        assert_eq!(app.projection_sync.acknowledged_document(), None);
        let shell = app.shell.as_mut().unwrap();
        let point = UiPoint {
            x: (shell.shell().nav_width() - 1) * 4 + 2,
            y: 10 * 9 + 4,
        };
        shell.pointer_moved(Some(point));
        shell.pointer_pressed();
        assert!(shell.dragging());
        let old_grid = shell.grid().clone();
        let old_generation = shell.generation();
        let old_width = shell.shell().nav_width();
        app.document
            .replace(newell_model::Document::default())
            .unwrap();
        app.projection_sync.fail_next_projection = 1;
        let update = app.poll_document_projection(now, ProjectionTrigger::Notification);
        assert!(!update.grid);
        assert_eq!(app.projection_sync.acknowledged_document(), None);
        assert_eq!(app.selection.document, app.document.identity());
        let shell = app.shell.as_mut().unwrap();
        assert!(!shell.dragging());
        assert_eq!(shell.grid(), &old_grid);
        assert_eq!(shell.generation(), old_generation);
        shell.pointer_moved(Some(UiPoint {
            x: point.x + 16,
            ..point
        }));
        shell.pointer_released();
        assert_eq!(shell.shell().nav_width(), old_width);
        assert!(!shell.dragging());
        let update = app.poll_document_projection(
            now + Duration::from_millis(100),
            ProjectionTrigger::RetryDeadline,
        );
        app.apply(update);
        assert_eq!(
            app.projection_sync.acknowledged_document(),
            Some(app.document.identity().tree())
        );
        assert_eq!(
            app.shell
                .as_ref()
                .unwrap()
                .shell()
                .tree_projection()
                .document,
            app.document.identity().tree()
        );
        assert!(!app.shell.as_ref().unwrap().dragging());
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
                text.contains("■ long_")
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
    fn real_tree_edits_publish_once_and_reproject_shared_selection() {
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
        assert_eq!(app.selection.entities(), &[entity]);
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
        assert_eq!(app.selection.entities(), &[entity]);
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
            SceneDisplayMode::HiddenLine
        );
        shell.key(ShellKey::Alt('v'));
        shell.key(ShellKey::Down);
        for expected in [
            SceneDisplayMode::NoShading,
            SceneDisplayMode::Shaded,
            SceneDisplayMode::Default,
            SceneDisplayMode::Points,
            SceneDisplayMode::Line,
            SceneDisplayMode::Wireframe,
            SceneDisplayMode::HiddenLine,
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

#[cfg(test)]
mod shared_selection_tests {
    use super::tests::{click, key};
    use super::*;
    use newell_model::{ConstructionKind, EntityKind, NewEntity};
    use newell_ui::{SelectionFieldRole, ShellKey, UiPoint};

    fn app() -> (App, Vec<EntityId>) {
        let mut app = App::new();
        app.shell = Some(ShellController::new(100, 45, GridMetrics::new(4, 9).unwrap()).unwrap());
        let f = app
            .document
            .add_base_feature(
                "outputs".into(),
                [
                    ("body-a", EntityKind::Body),
                    ("surface-b", EntityKind::Surface),
                    ("axis-c", EntityKind::Construction(ConstructionKind::Axis)),
                ]
                .map(|(name, kind)| NewEntity {
                    name: name.into(),
                    kind,
                    visible: true,
                })
                .into(),
            )
            .unwrap();
        let ids = app
            .document
            .document()
            .feature(f)
            .unwrap()
            .outputs()
            .to_vec();
        let update = app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        app.apply(update);
        (app, ids)
    }
    fn select(app: &mut App, op: SelectionOp, entities: Vec<EntityId>) {
        let update = app.handle_selection_request(SelectionRequest {
            document: app.document.identity(),
            op,
            entities,
        });
        app.apply(update);
    }
    fn open_window(app: &mut App) -> SelectionOpen {
        app.next_selection_id = app.next_selection_id.checked_add(1).unwrap();
        let request = SelectionOpen {
            session: SelectionSessionId(app.next_selection_id),
            document: app.document.identity().tree(),
            destination: SelectionDestination::Window,
            rows: candidate_rows(&app.document, None, None),
            applied: app
                .selection
                .entities()
                .iter()
                .map(|id| OpaqueTreeId::new(id.get()))
                .collect(),
            max_count: None,
        };
        let update = app
            .shell
            .as_mut()
            .unwrap()
            .open_selection(request.clone())
            .unwrap();
        app.active_selection = Some(request.clone());
        app.apply(update);
        request
    }
    #[test]
    fn operation_outputs_modifiers_visibility_and_both_tree_views_share_one_ordered_state() {
        let (mut app, ids) = app();
        let revision = app.document.revision();
        let events = app.document.subscribe();
        click(&mut app, "outputs");
        assert_eq!(app.selection.entities(), ids);
        let shell = app.shell.as_ref().unwrap().shell();
        assert_eq!(
            shell.entity_selection().entities,
            ids.iter()
                .map(|id| OpaqueTreeId::new(id.get()))
                .collect::<Vec<_>>()
        );
        assert_eq!(shell.operation_selection(OpaqueTreeId::new(0)), (3, 3));
        app.shell
            .as_mut()
            .unwrap()
            .set_selection_modifiers(false, true);
        click(&mut app, "body-a");
        assert_eq!(app.selection.entities(), &ids[1..]);
        assert_eq!(
            app.shell
                .as_ref()
                .unwrap()
                .shell()
                .operation_selection(OpaqueTreeId::new(0)),
            (2, 3)
        );
        click(&mut app, "Bodies(1)");
        assert_eq!(app.selection.entities(), &ids[1..]);
        assert_eq!(app.document.revision(), revision);
        assert!(events.try_recv().is_err());
        click(&mut app, "■ body-a");
        assert!(!app.document.document().entity(ids[0]).unwrap().visible());
        assert_eq!(app.selection.entities(), &ids[1..]);
        assert_eq!(events.try_iter().count(), 1);
        assert_eq!(
            app.last_ui_publication.as_ref(),
            Some(app.shell.as_ref().unwrap().grid())
        );
    }
    #[test]
    fn window_panel_drafts_commit_once_cancel_and_reject_old_completions() {
        let (mut app, ids) = app();
        select(&mut app, SelectionOp::Replace, vec![ids[1], ids[0]]);
        let revision = app.document.revision();
        let events = app.document.subscribe();
        let request = open_window(&mut app);
        key(&mut app, ShellKey::Char(' '));
        assert_eq!(app.selection.entities(), &[ids[1], ids[0]]);
        key(&mut app, ShellKey::Escape);
        assert_eq!(app.selection.entities(), &[ids[1], ids[0]]);
        open_window(&mut app);
        key(&mut app, ShellKey::Char(' '));
        click(&mut app, "OK");
        assert_eq!(app.selection.entities(), &[ids[1]]);
        let update = app.handle_selection_ui(SelectionUiRequest::Complete(
            newell_ui::SelectionCompletion {
                session: request.session,
                document: request.document,
                destination: request.destination,
                outcome: SelectionOutcome::Confirmed(Vec::new()),
            },
        ));
        assert!(!update.any());
        assert_eq!(app.selection.entities(), &[ids[1]]);
        assert_eq!(app.document.revision(), revision);
        assert!(events.try_recv().is_err());
    }
    #[test]
    fn field_transaction_latches_and_updates_only_originating_input() {
        let (mut app, ids) = app();
        select(&mut app, SelectionOp::Replace, vec![ids[0]]);
        click(&mut app, "Create");
        click(&mut app, "[E]xtrude");
        click(&mut app, "Add");
        click(&mut app, "Merge with all");
        let session = app.command_inputs.as_ref().unwrap().session;
        let d = app
            .shell
            .as_ref()
            .unwrap()
            .shell()
            .field_descriptors()
            .into_iter()
            .find(|d| {
                d.field.operation == ActionOperation::Extrude
                    && d.role == SelectionFieldRole::MergeScope
            })
            .unwrap();
        select(&mut app, SelectionOp::Replace, vec![ids[1]]);
        let revision = app.document.revision();
        let events = app.document.subscribe();
        let update = app.open_field_selection(d);
        app.apply(update);
        assert_eq!(
            app.shell
                .as_ref()
                .unwrap()
                .shell()
                .selection()
                .request()
                .unwrap()
                .rows
                .len(),
            1
        );
        key(&mut app, ShellKey::Char(' '));
        assert_eq!(
            app.command_inputs.as_ref().unwrap().values[&d.field].entities(),
            &[ids[0]]
        );
        click(&mut app, "OK");
        assert_eq!(app.command_inputs.as_ref().unwrap().session, session);
        assert_eq!(
            app.command_inputs.as_ref().unwrap().values[&d.field].entities(),
            &[]
        );
        assert_eq!(
            app.shell
                .as_ref()
                .unwrap()
                .shell()
                .selection_field_count(d.field),
            0
        );
        assert_eq!(app.selection.entities(), &[ids[1]]);
        let update = app.open_field_selection(d);
        app.apply(update);
        key(&mut app, ShellKey::Char(' '));
        key(&mut app, ShellKey::Escape);
        assert_eq!(
            app.command_inputs.as_ref().unwrap().values[&d.field].entities(),
            &[]
        );
        assert_eq!(app.document.revision(), revision);
        assert!(events.try_recv().is_err());
        let seeded = app
            .command_inputs
            .as_ref()
            .unwrap()
            .values
            .iter()
            .find(|(k, _)| k.key == 2)
            .unwrap()
            .1
            .entities();
        assert_eq!(seeded, &[ids[0]]);
    }
    #[test]
    fn local_publication_retries_without_a_document_event_and_replacement_invalidates_every_session()
     {
        let (mut app, ids) = app();
        let events = app.document.subscribe();
        app.fail_next_selection = 1;
        select(&mut app, SelectionOp::Replace, vec![ids[0]]);
        assert!(app.selection_view_pending);
        let deadline = app.selection_retry_at.unwrap();
        let update = app.poll_document_projection(
            deadline - Duration::from_millis(1),
            ProjectionTrigger::Notification,
        );
        assert!(!update.grid);
        let update = app.poll_document_projection(deadline, ProjectionTrigger::RetryDeadline);
        app.apply(update);
        assert!(!app.selection_view_pending);
        assert_eq!(
            app.shell
                .as_ref()
                .unwrap()
                .shell()
                .entity_selection()
                .entities,
            vec![OpaqueTreeId::new(ids[0].get())]
        );
        assert!(events.try_recv().is_err());
        let old_request = open_window(&mut app);
        app.document.replace(Default::default()).unwrap();
        app.projection_sync.fail_next_projection = 1;
        let update = app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        assert!(!update.grid);
        assert!(app.selection.entities().is_empty());
        assert!(app.active_selection.is_none());
        assert!(!app.scene_selected);
        let update = app.handle_selection_ui(SelectionUiRequest::Complete(
            newell_ui::SelectionCompletion {
                session: old_request.session,
                document: old_request.document,
                destination: old_request.destination,
                outcome: SelectionOutcome::Confirmed(old_request.applied),
            },
        ));
        assert!(!update.any());
        assert!(app.selection.entities().is_empty());
    }
    #[test]
    fn stale_completion_cannot_consume_a_newer_window_transaction() {
        let (mut app, ids) = app();
        let stale = open_window(&mut app);
        key(&mut app, ShellKey::Escape);
        let current = open_window(&mut app);
        let update = app.handle_selection_ui(SelectionUiRequest::Complete(
            newell_ui::SelectionCompletion {
                session: stale.session,
                document: stale.document,
                destination: stale.destination,
                outcome: SelectionOutcome::Confirmed(vec![OpaqueTreeId::new(ids[1].get())]),
            },
        ));
        assert!(!update.any());
        assert_eq!(
            app.active_selection.as_ref().unwrap().session,
            current.session
        );
        key(&mut app, ShellKey::Char(' '));
        click(&mut app, "OK");
        assert_eq!(app.selection.entities(), &[ids[0]]);
    }
    #[test]
    fn rename_reconciles_open_draft_and_replacement_blocks_old_modal_input_before_retry() {
        let (mut app, ids) = app();
        open_window(&mut app);
        key(&mut app, ShellKey::Char(' '));
        app.document
            .rename_entity(ids[0], "new-body-name".into())
            .unwrap();
        let update = app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        app.apply(update);
        let panel = app.shell.as_ref().unwrap().shell().selection();
        assert_eq!(panel.request().unwrap().rows[0].label, "new-body-name");
        assert_eq!(panel.draft_entities(), &[OpaqueTreeId::new(ids[0].get())]);
        app.document.replace(Default::default()).unwrap();
        app.projection_sync.fail_next_projection = 1;
        let update = app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        assert!(!update.any());
        assert!(app.active_selection.is_none());
        let panel = app.shell.as_ref().unwrap().shell().selection().clone();
        let update = app.shell.as_mut().unwrap().key(ShellKey::Char(' '));
        assert!(!update.any());
        assert_eq!(app.shell.as_ref().unwrap().shell().selection(), &panel);
        let deadline = app.next_document_retry_deadline().unwrap();
        let update = app.poll_document_projection(deadline, ProjectionTrigger::RetryDeadline);
        app.apply(update);
        assert!(!app.shell.as_ref().unwrap().shell().selection().is_open());
        assert!(!app.shell.as_ref().unwrap().shell().command_open());
    }
    #[test]
    fn failed_local_publication_suspends_after_one_retry_then_rearms_without_events() {
        let (mut app, ids) = app();
        let events = app.document.subscribe();
        let revision = app.document.revision();
        app.fail_next_selection = 2;
        select(&mut app, SelectionOp::Replace, vec![ids[0]]);
        let deadline = app.selection_retry_at.unwrap();
        app.poll_document_projection(deadline, ProjectionTrigger::RetryDeadline);
        assert!(app.selection_suspended);
        assert!(app.selection_retry_at.is_none());
        let generation = app.shell.as_ref().unwrap().generation();
        for _ in 0..3 {
            app.poll_document_projection(
                deadline + Duration::from_secs(1),
                ProjectionTrigger::Notification,
            );
        }
        assert!(app.selection_suspended);
        assert_eq!(app.shell.as_ref().unwrap().generation(), generation);
        let update = app.poll_document_projection(
            deadline + Duration::from_secs(1),
            ProjectionTrigger::UserActivation,
        );
        app.apply(update);
        assert!(!app.selection_view_pending);
        assert_eq!(app.document.revision(), revision);
        assert!(events.try_recv().is_err());
    }
    fn arm(app: &mut App, point: (f64, f64), edit: SelectionEdit) {
        assert!(app.navigation.begin_primary_orbit(point, 1.0, true));
        app.pending_scene_click = Some(PendingSceneClick {
            document: app.document.identity(),
            generation: app.shell.as_ref().unwrap().generation(),
            mesh_revision: app.navigation.scene().revision(),
            edit,
        });
    }
    fn release(app: &mut App, point: (f64, f64)) {
        let certificate = app.navigation.finish_primary(point);
        let update = app.finish_scene_selection(point, certificate);
        app.apply(update);
    }
    #[test]
    fn real_mesh_click_release_drag_miss_preview_and_selection_brightness_binding() {
        let (mut app, ids) = app();
        let viewport = scene_viewport(app.shell.as_ref().unwrap());
        app.navigation.set_layout(viewport, Vec::new());
        let mesh = NewellSolid::box_solid(1.0, 1.0, 1.0)
            .unwrap()
            .tessellate(0.05)
            .unwrap();
        app.install_scene_mesh(mesh.clone(), Some(ids[0]));
        let point = app.navigation.camera().project([0.5, 0.5, 0.5]).unwrap();
        assert!(app.navigation.pick(point).is_some());
        // The controller's exposed-pixel eligibility uses its current cursor too.
        app.shell.as_mut().unwrap().pointer_moved(Some(UiPoint {
            x: point.0 as u32,
            y: point.1 as u32,
        }));
        arm(&mut app, point, SelectionEdit::Replace);
        assert!(app.selection.entities().is_empty());
        release(&mut app, point);
        assert_eq!(app.selection.entities(), &[ids[0]]);
        assert!(app.scene_selected);
        let miss = (f64::from(viewport.x + 1), f64::from(viewport.y + 1));
        assert!(app.navigation.pick(miss).is_none());
        app.shell.as_mut().unwrap().pointer_moved(Some(UiPoint {
            x: miss.0 as u32,
            y: miss.1 as u32,
        }));
        for edit in [SelectionEdit::Add, SelectionEdit::Toggle] {
            arm(&mut app, miss, edit);
            release(&mut app, miss);
            assert_eq!(app.selection.entities(), &[ids[0]]);
        }
        arm(&mut app, miss, SelectionEdit::Replace);
        release(&mut app, miss);
        assert!(app.selection.entities().is_empty());
        assert!(!app.scene_selected);
        app.shell.as_mut().unwrap().pointer_moved(Some(UiPoint {
            x: point.0 as u32,
            y: point.1 as u32,
        }));
        select(&mut app, SelectionOp::Replace, vec![ids[1]]);
        arm(&mut app, point, SelectionEdit::Replace);
        app.install_scene_mesh(mesh.clone(), Some(ids[0]));
        release(&mut app, point);
        assert_eq!(app.selection.entities(), &[ids[1]]);
        select(&mut app, SelectionOp::Clear, Vec::new());
        arm(&mut app, point, SelectionEdit::Replace);
        app.navigation
            .advance_primary_orbit((point.0 + 10.0, point.1));
        release(&mut app, point);
        assert!(app.selection.entities().is_empty());
        assert!(!app.scene_selected);
        // Use the new camera projection after orbit.
        let point = app.navigation.camera().project([0.5, 0.5, 0.5]).unwrap();
        app.shell.as_mut().unwrap().pointer_moved(Some(UiPoint {
            x: point.0 as u32,
            y: point.1 as u32,
        }));
        select(&mut app, SelectionOp::Replace, vec![ids[1]]);
        app.install_scene_mesh(mesh, None);
        arm(&mut app, point, SelectionEdit::Replace);
        release(&mut app, point);
        assert_eq!(app.selection.entities(), &[ids[1]]);
        assert!(!app.scene_selected);
    }
}

#[cfg(test)]
mod numeric_binding_tests {
    use super::tests::{click, key, live_app};
    use super::*;
    use newell_model::{Direction, Feature, Length, Quantity, Unit};
    use newell_ui::{EditKey, NumericFieldKey, ShellKey, UiPoint, WidgetId};

    #[test]
    fn placeholder_focus_and_empty_exits_never_mutate_accepted_or_document_state() {
        let mut a = app();
        let document = a.document.document().clone();
        let version = a.document.version();
        let accepted = a.numeric_inputs.as_ref().unwrap().values.clone();
        let parameters = a.numeric_inputs.as_ref().unwrap().parameter_draft.clone();
        let sequence = a.numeric_inputs.as_ref().unwrap().consumed_sequence;
        for exit in [ShellKey::Enter, ShellKey::Escape] {
            activate(&mut a, 4, 0);
            assert!(a.shell.as_ref().unwrap().shell().numeric_editing());
            assert_eq!(
                a.numeric_inputs.as_ref().unwrap().consumed_sequence,
                sequence
            );
            key(&mut a, exit);
            assert_eq!(a.numeric_inputs.as_ref().unwrap().values, accepted);
            assert_eq!(
                a.numeric_inputs.as_ref().unwrap().parameter_draft,
                parameters
            );
            assert_eq!(a.document.version(), version);
            assert_eq!(a.document.document(), &document);
            assert!(!a.numeric_committed);
        }
        activate(&mut a, 4, 0);
        let update = a.shell.as_mut().unwrap().numeric_focus_lost();
        a.finish_shell_input(update);
        assert_eq!(
            a.numeric_inputs.as_ref().unwrap().consumed_sequence,
            sequence
        );
        assert_eq!(a.numeric_inputs.as_ref().unwrap().values, accepted);
        assert_eq!(a.document.version(), version);
        assert_eq!(a.document.document(), &document);
        assert!(a.shell.as_mut().unwrap().take_numeric_requests().is_empty());
    }

    #[test]
    fn explicit_zero_and_formula_zero_survive_focus_rejection_and_formula_cancel() {
        let mut a = app();
        edit(&mut a, 4, "0", false, true);
        assert_eq!(value(&a, 4), 0.0);
        assert!(!a.numeric_inputs.as_ref().unwrap().values[&field(4)].initial_placeholder);
        let sequence = a.numeric_inputs.as_ref().unwrap().consumed_sequence;
        activate(&mut a, 4, 0);
        key(&mut a, ShellKey::Enter);
        assert_eq!(
            a.numeric_inputs.as_ref().unwrap().consumed_sequence,
            sequence
        );
        edit(&mut a, 4, ".", false, true);
        assert_eq!(value(&a, 4), 0.0);
        assert!(!a.numeric_inputs.as_ref().unwrap().values[&field(4)].initial_placeholder);
        formula(&mut a, 4, "0mm");
        assert!(!a.shell.as_ref().unwrap().shell().formula().is_open());
        let accepted = a.numeric_inputs.as_ref().unwrap().values[&field(4)].clone();
        assert!(!accepted.initial_placeholder);
        activate(&mut a, 4, 0);
        key(&mut a, ShellKey::Enter);
        activate(&mut a, 4, 1);
        assert_eq!(a.shell.as_ref().unwrap().shell().formula().draft(), "0mm");
        key(&mut a, ShellKey::Escape);
        assert_eq!(
            a.numeric_inputs.as_ref().unwrap().values[&field(4)],
            accepted
        );
    }

    #[test]
    fn direct_formula_and_empty_blur_preserve_initial_placeholder() {
        let mut a = app();
        for with_empty_edit in [false, true] {
            if with_empty_edit {
                activate(&mut a, 4, 0);
            }
            activate(&mut a, 4, 1);
            assert!(a.shell.as_ref().unwrap().shell().formula().is_open());
            assert_eq!(a.shell.as_ref().unwrap().shell().formula().draft(), "");
            assert!(!a.shell.as_ref().unwrap().shell().numeric_editing());
            key(&mut a, ShellKey::Escape);
            assert!(a.numeric_inputs.as_ref().unwrap().values[&field(4)].initial_placeholder);
            assert!(!a.shell.as_ref().unwrap().shell().numeric_editing());
        }
        formula(&mut a, 4, "");
        assert!(a.numeric_inputs.as_ref().unwrap().values[&field(4)].initial_placeholder);
        let before = a.numeric_inputs.as_ref().unwrap().values.clone();
        formula(&mut a, 4, "#missing");
        assert!(a.shell.as_ref().unwrap().shell().formula().is_open());
        assert_eq!(a.numeric_inputs.as_ref().unwrap().values, before);
        key(&mut a, ShellKey::Escape);
    }

    fn field(key: u32) -> NumericFieldKey {
        NumericFieldKey {
            operation: ActionOperation::Extrude,
            key,
        }
    }
    fn app() -> App {
        let mut a = live_app();
        a.finish_shell_input(Update::default());
        a
    }
    fn activate(a: &mut App, key: u32, slot: u32) {
        let shell = a.shell.as_mut().unwrap();
        let base = shell.shell().extrude().depth_field_id().get() - 4 * 4;
        let id = WidgetId::new(base + key * 4 + slot);
        let point = (0..shell.grid().rows())
            .find_map(|r| {
                (0..shell.grid().cols())
                    .find(|c| shell.grid().get(*c, r).unwrap().widget_id == Some(id))
                    .map(|c| UiPoint {
                        x: c * 4 + 2,
                        y: r * 9 + 4,
                    })
            })
            .expect("visible stable control");
        shell.pointer_moved(Some(point));
        let update = shell.pointer_pressed();
        shell.pointer_released();
        a.finish_shell_input(update);
    }
    fn edit(a: &mut App, field: u32, text: &str, reverse: bool, accept: bool) {
        activate(a, field, 0);
        key(a, ShellKey::Edit(EditKey::SelectAll));
        for c in text.chars() {
            key(a, ShellKey::Char(c));
        }
        if reverse {
            key(a, ShellKey::Char('-'));
        }
        if accept {
            key(a, ShellKey::Enter);
        }
    }
    pub(super) fn formula(a: &mut App, field: u32, source: &str) {
        activate(a, field, 1);
        assert!(a.shell.as_ref().unwrap().shell().formula().is_open());
        key(a, ShellKey::Edit(EditKey::SelectAll));
        for c in source.chars() {
            key(a, ShellKey::Char(c));
        }
        key(a, ShellKey::Enter);
    }
    fn value(a: &App, key: u32) -> f64 {
        a.numeric_inputs.as_ref().unwrap().values[&field(key)]
            .quantity
            .value_in(Unit::Millimetre)
            .unwrap()
    }
    fn committed(a: &App) -> newell_model::ExtrudeInputProjection {
        a.document
            .document()
            .features()
            .last()
            .and_then(|(_, feature)| match feature {
                Feature::Extrude(f) => Some(f.spec.inputs()),
                _ => None,
            })
            .unwrap()
    }
    fn mm(p: &newell_model::ParameterValue) -> f64 {
        p.evaluate(&newell_model::parameters::ParameterRegistry::default())
            .unwrap()
            .as_millimetres()
    }

    #[test]
    fn unequal_distances_and_independent_directions_commit_once() {
        let mut a = app();
        let events = a.document.subscribe();
        edit(&mut a, 4, "12.3", false, true);
        activate(&mut a, 9, 0);
        edit(&mut a, 11, "4.5", true, true);
        activate(&mut a, 13, 0);
        edit(&mut a, 15, "6.7", false, true);
        activate(&mut a, 18, 0);
        edit(&mut a, 20, "8.9", true, false);
        activate(&mut a, 28, 0);
        let spec = committed(&a);
        assert_eq!(mm(&spec.depth), 12.3);
        assert_eq!(mm(spec.starting_offset.as_ref().unwrap()), 4.5);
        assert_eq!(mm(spec.end_offset.as_ref().unwrap()), 6.7);
        assert_eq!(mm(spec.second_end.as_ref().unwrap()), 8.9);
        assert_eq!(spec.directions.starting_offset, Direction::Reverse);
        assert_eq!(spec.directions.second_end, Direction::Reverse);
        assert_eq!(spec.directions.depth, Direction::Forward);
        assert_eq!(a.document.revision(), 1);
        assert_eq!(events.try_iter().count(), 1);
        assert!(a.numeric_inputs.is_none());
        for _ in 0..3 {
            a.finish_shell_input(Update::default());
            a.sync_extrude_command();
        }
        assert_eq!(a.document.document().features().len(), 1);
    }
    #[test]
    fn nested_second_offset_is_separate_and_hidden_values_survive_branches() {
        let (mut a, wake) = super::real_extrude_tests::runtime_app().unwrap();
        super::real_extrude_tests::open(&mut a);
        super::real_extrude_tests::profile(&mut a);
        edit(&mut a, 4, "12.3", false, true);
        activate(&mut a, 9, 0);
        edit(&mut a, 11, "4.5", false, false);
        activate(&mut a, 9, 0);
        assert_eq!(value(&a, 11), 4.5);
        assert!(
            (match a.geometry_edit().unwrap() {
                newell_model::geometry::DocumentEdit::ExtrudeInsert { spec, .. } =>
                    Some(spec.inputs()),
                _ => None,
            })
            .unwrap()
            .starting_offset
            .is_none()
        );
        activate(&mut a, 9, 0);
        assert_eq!(
            mm((match a.geometry_edit().unwrap() {
                newell_model::geometry::DocumentEdit::ExtrudeInsert { spec, .. } =>
                    Some(spec.inputs()),
                _ => None,
            })
            .unwrap()
            .starting_offset
            .as_ref()
            .unwrap()),
            4.5
        );
        activate(&mut a, 10, 0);
        assert!(matches!(
            a.geometry_edit(),
            Err(AppError::Model(newell_model::ModelError::MissingTarget(_)))
        ));
        activate(&mut a, 10, 0);
        activate(&mut a, 18, 0);
        edit(&mut a, 20, "8.9", true, true);
        activate(&mut a, 17, 0);
        assert!(
            (match a.geometry_edit().unwrap() {
                newell_model::geometry::DocumentEdit::ExtrudeInsert { spec, .. } =>
                    Some(spec.inputs()),
                _ => None,
            })
            .unwrap()
            .second_end
            .is_none()
        );
        activate(&mut a, 17, 0);
        assert_eq!(value(&a, 20), 8.9);
        activate(&mut a, 19, 0);
        activate(&mut a, 21, 0);
        super::tests::click(&mut a, "Base Solid / Face 2");
        super::tests::click(&mut a, "OK");
        activate(&mut a, 22, 0);
        edit(&mut a, 24, "2.4", false, true);
        activate(&mut a, 28, 0);
        super::real_extrude_tests::pump(&mut a, &wake).unwrap();
        assert!(
            matches!(
                a.document.document().features().last(),
                Some((_, Feature::Extrude(_)))
            ),
            "Extrude did not commit: {}; {:?}",
            super::tests::sketch_row(&a, "Part Design"),
            a.geometry_edit()
        );
        let spec = committed(&a);
        assert!(spec.second_end.is_none());
        assert!(spec.end_offset.is_none());
        assert_eq!(mm(spec.second_end_offset.as_ref().unwrap()), 2.4);
    }
    #[test]
    fn named_local_inputs_rename_and_disjoint_source_edits_keep_stable_ids() {
        let mut a = app();
        let document = a.document.document().clone();
        formula(&mut a, 4, "#Depth := 12mm");
        assert!(!a.shell.as_ref().unwrap().shell().formula().is_open());
        let depth = a.numeric_inputs.as_ref().unwrap().bindings[&field(4)];
        activate(&mut a, 9, 0);
        formula(&mut a, 11, "#Depth/2");
        assert_eq!(value(&a, 11), 6.);
        formula(&mut a, 4, "#Renamed := 12mm");
        assert_eq!(
            a.numeric_inputs.as_ref().unwrap().bindings[&field(4)],
            depth
        );
        activate(&mut a, 11, 1);
        key(&mut a, ShellKey::Enter);
        assert_eq!(value(&a, 11), 6.);
        activate(&mut a, 11, 1);
        key(&mut a, ShellKey::Backspace);
        key(&mut a, ShellKey::Char('3'));
        key(&mut a, ShellKey::Enter);
        assert!(!a.shell.as_ref().unwrap().shell().formula().is_open());
        assert_eq!(value(&a, 11), 4.);
        assert_eq!(
            a.numeric_inputs.as_ref().unwrap().values[&field(11)].formula,
            "#Depth/3"
        );
        formula(&mut a, 11, "#Depth/3");
        assert!(a.shell.as_ref().unwrap().shell().formula().is_open());
        assert_eq!(value(&a, 11), 4.);
        key(&mut a, ShellKey::Escape);
        assert_eq!(a.document.document(), &document);
        assert_eq!(a.document.revision(), 0);
        activate(&mut a, 28, 1);
        assert_eq!(a.document.document(), &document);
    }
    #[test]
    fn precision_noop_direction_escape_and_formula_replacement_are_independent() {
        let mut a = app();
        let exact = 1.234_567_89;
        a.numeric_inputs
            .as_mut()
            .unwrap()
            .values
            .get_mut(&field(4))
            .unwrap()
            .quantity = Quantity::Length(Length::millimetres(exact).unwrap());
        a.numeric_inputs
            .as_mut()
            .unwrap()
            .values
            .get_mut(&field(4))
            .unwrap()
            .initial_placeholder = false;
        let projection = a.numeric_inputs.as_ref().unwrap().project().unwrap();
        a.shell
            .as_mut()
            .unwrap()
            .set_numeric_projection(projection)
            .unwrap();
        for _ in 0..2 {
            activate(&mut a, 4, 0);
            key(&mut a, ShellKey::Enter);
            assert_eq!(value(&a, 4), exact);
        }
        formula(&mut a, 4, "1.23456789mm");
        activate(&mut a, 4, 0);
        key(&mut a, ShellKey::Char('-'));
        key(&mut a, ShellKey::Enter);
        assert_eq!(value(&a, 4), exact);
        assert_eq!(
            a.numeric_inputs.as_ref().unwrap().values[&field(4)].formula,
            "1.23456789mm"
        );
        edit(&mut a, 4, "5.0", false, false);
        key(&mut a, ShellKey::Escape);
        assert_eq!(value(&a, 4), exact);
        edit(&mut a, 4, "1.3", false, true);
        assert_eq!(value(&a, 4), 1.3);
        assert_eq!(
            a.numeric_inputs.as_ref().unwrap().values[&field(4)].formula,
            ""
        );
    }
    #[test]
    fn same_event_formula_open_uses_accepted_literal_and_rejection_preserves_source() {
        for incomplete in [false, true] {
            for cancel in [false, true] {
                let mut a = app();
                edit(&mut a, 4, "12.3", false, true);
                formula(&mut a, 4, "12.3mm");
                edit(
                    &mut a,
                    4,
                    if incomplete { "." } else { "7.5" },
                    false,
                    false,
                );
                activate(&mut a, 4, 1);
                assert_eq!(
                    a.shell.as_ref().unwrap().shell().formula().draft(),
                    if incomplete { "12.3mm" } else { "" }
                );
                assert_eq!(value(&a, 4), if incomplete { 12.3 } else { 7.5 });
                key(
                    &mut a,
                    if cancel {
                        ShellKey::Escape
                    } else {
                        ShellKey::Enter
                    },
                );
                assert_eq!(
                    a.numeric_inputs.as_ref().unwrap().values[&field(4)].formula,
                    if incomplete { "12.3mm" } else { "" }
                );
            }
        }
    }
    #[test]
    fn formula_delivery_retry_opens_once_without_reapplying_edit_or_resetting_editor() {
        let mut a = app();
        edit(&mut a, 4, "12.3", false, true);
        formula(&mut a, 4, "12.3mm");
        edit(&mut a, 4, "7.5", false, false);
        a.shell.as_mut().unwrap().fail_numeric_formula = 1;
        activate(&mut a, 4, 1);
        assert!(!a.shell.as_ref().unwrap().shell().formula().is_open());
        let pending = a.numeric_delivery.clone().unwrap();
        let request = match pending.kind {
            NumericDeliveryKind::Formula(request, _) => Some(request),
            _ => None,
        }
        .unwrap();
        let sequence = a.numeric_inputs.as_ref().unwrap().consumed_sequence;
        let at = a.next_document_retry_deadline().unwrap();
        a.poll_document_projection(at, ProjectionTrigger::RetryDeadline);
        assert!(a.shell.as_ref().unwrap().shell().formula().is_open());
        assert_eq!(a.shell.as_ref().unwrap().shell().formula().draft(), "");
        key(&mut a, ShellKey::Char('x'));
        a.complete_numeric_formula(request);
        assert_eq!(a.shell.as_ref().unwrap().shell().formula().draft(), "x");
        assert_eq!(
            a.numeric_inputs.as_ref().unwrap().consumed_sequence,
            sequence
        );
        assert_eq!(value(&a, 4), 7.5);
    }
    #[test]
    fn incomplete_final_input_rejects_ok_and_explicit_retry_commits() {
        let mut a = app();
        edit(&mut a, 4, "12.3", false, true);
        edit(&mut a, 4, ".", false, false);
        activate(&mut a, 28, 0);
        assert_eq!(a.document.revision(), 0);
        assert_eq!(
            a.shell.as_ref().unwrap().shell().extrude().status,
            ExtrudeStatus::Editing
        );
        assert_eq!(value(&a, 4), 12.3);
        assert_eq!(a.numeric_last_attempt, 1);
        assert!(!a.extrude_latch.armed);
        a.finish_shell_input(Update::default());
        assert_eq!(a.document.revision(), 0);
        activate(&mut a, 28, 0);
        assert_eq!(a.document.revision(), 1);
    }
    #[test]
    fn final_revision_failure_keeps_drafts_preview_document_and_panel_atomic() {
        let mut a = app();
        edit(&mut a, 4, "12.3", false, true);
        a.document.set_revision_for_test(u64::MAX);
        let events = a.document.subscribe();
        let original = a.document.document().clone();
        let state = a.commands.state().clone();
        let preview = a.commands.preview().cloned();
        let session = a.command_inputs.as_ref().unwrap().session;
        edit(&mut a, 4, "14.5", false, false);
        activate(&mut a, 28, 0);
        assert_eq!(a.document.document(), &original);
        assert_eq!(a.document.revision(), u64::MAX);
        assert!(events.try_recv().is_err());
        assert_eq!(a.commands.state(), &state);
        assert_eq!(a.commands.preview(), preview.as_ref());
        assert_eq!(value(&a, 4), 14.5);
        assert_eq!(a.command_inputs.as_ref().unwrap().session, session);
        assert_eq!(
            a.shell.as_ref().unwrap().shell().extrude().status,
            ExtrudeStatus::Editing
        );
        assert!(!a.extrude_latch.armed);
        a.sync_extrude_command();
        a.finish_shell_input(Update::default());
        assert_eq!(a.commands.state(), &state);
        assert_eq!(a.numeric_last_attempt, 1);
    }
    #[test]
    fn committed_outcome_retries_closure_without_a_second_feature_or_notification() {
        let mut a = app();
        edit(&mut a, 4, "12.3", false, true);
        let events = a.document.subscribe();
        a.shell.as_mut().unwrap().fail_numeric_confirm = 2;
        activate(&mut a, 28, 0);
        assert_eq!(a.document.revision(), 1);
        assert!(a.numeric_committed);
        assert!(a.numeric_inputs.is_some());
        assert_eq!(
            a.shell.as_ref().unwrap().shell().extrude().status,
            ExtrudeStatus::Editing
        );
        let at = a.next_document_retry_deadline().unwrap();
        a.poll_document_projection(at, ProjectionTrigger::RetryDeadline);
        assert!(a.numeric_delivery.as_ref().unwrap().retry_at.is_none());
        a.poll_document_projection(at + Duration::from_secs(1), ProjectionTrigger::Notification);
        assert!(a.numeric_committed);
        key(&mut a, ShellKey::Tab);
        assert!(a.numeric_delivery.is_none());
        assert!(a.numeric_inputs.is_none());
        assert_eq!(
            a.shell.as_ref().unwrap().shell().extrude().status,
            ExtrudeStatus::Confirmed
        );
        assert_eq!(a.document.document().features().len(), 1);
        assert_eq!(events.try_iter().count(), 1);
        a.sync_extrude_command();
        assert_eq!(a.document.revision(), 1);
    }
    #[test]
    fn caches_rebind_to_fresh_sessions_and_cancel_restores_defaults() {
        let mut a = app();
        edit(&mut a, 4, "12.3", false, true);
        let old = a.numeric_inputs.as_ref().unwrap().session;
        click(&mut a, "Create");
        click(&mut a, "[R]evolve");
        assert_eq!(
            a.numeric_inputs.as_ref().unwrap().operation,
            ActionOperation::Revolve
        );
        assert_ne!(a.numeric_inputs.as_ref().unwrap().session, old);
        activate(&mut a, 4, 0);
        edit(&mut a, 6, "42.5", true, true);
        activate(&mut a, 28, 0);
        assert_eq!(a.document.revision(), 0);
        click(&mut a, "Create");
        click(&mut a, "[E]xtrude");
        assert_eq!(value(&a, 4), 12.3);
        activate(&mut a, 28, 1);
        assert_eq!(a.document.revision(), 0);
        assert!(a.numeric_inputs.is_none());
        click(&mut a, "Create");
        click(&mut a, "[E]xtrude");
        assert_eq!(value(&a, 4), 0.0);
    }
    #[test]
    fn invalid_formula_stays_open_and_cancel_preserves_the_accepted_offset() {
        let mut a = app();
        edit(&mut a, 4, "12.3", false, true);
        activate(&mut a, 9, 0);
        edit(&mut a, 11, "4.5", false, true);
        let before = a.numeric_inputs.as_ref().unwrap().parameter_draft.clone();
        formula(&mut a, 11, "#missing + 2mm");
        assert!(a.shell.as_ref().unwrap().shell().formula().is_open());
        assert_eq!(
            a.shell.as_ref().unwrap().shell().formula().draft(),
            "#missing + 2mm"
        );
        assert_eq!(value(&a, 11), 4.5);
        assert_eq!(a.document.revision(), 0);
        assert_eq!(a.numeric_inputs.as_ref().unwrap().parameter_draft, before);
        key(&mut a, ShellKey::Escape);
        assert!(!a.shell.as_ref().unwrap().shell().formula().is_open());
        activate(&mut a, 28, 0);
        assert_eq!(a.document.revision(), 1);
        assert_eq!(mm(committed(&a).starting_offset.as_ref().unwrap()), 4.5);
    }
    #[test]
    fn depth_expression_is_evaluated_and_invalid_source_is_not_accepted() {
        let (mut a, wake) = super::real_extrude_tests::runtime_app().unwrap();
        super::real_extrude_tests::open(&mut a);
        super::real_extrude_tests::profile(&mut a);
        formula(&mut a, 4, "2mm + 3mm");
        assert_eq!(value(&a, 4), 5.);
        activate(&mut a, 28, 0);
        super::real_extrude_tests::pump(&mut a, &wake).unwrap();
        assert!(matches!(
            committed(&a).depth,
            newell_model::ParameterValue::Bound(_)
        ));
        assert_eq!(
            committed(&a)
                .depth
                .evaluate(a.document.document().parameters())
                .unwrap()
                .as_millimetres(),
            5.
        );
        let mut a = app();
        formula(&mut a, 4, "not-defined");
        assert!(a.shell.as_ref().unwrap().shell().formula().is_open());
        key(&mut a, ShellKey::Escape);
        activate(&mut a, 3, 1);
        activate(&mut a, 3, 1);
        activate(&mut a, 28, 0);
        assert_eq!(a.document.revision(), 1);
        assert!(matches!(
            committed(&a).depth,
            newell_model::ParameterValue::Literal(_)
        ));
    }
    #[test]
    fn selection_only_retry_has_a_new_attempt_at_the_same_numeric_barrier() {
        let mut a = live_app();
        let base = a.document.add_base_solid("base").unwrap();
        a.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        a.finish_shell_input(Update::default());
        let entity = a.document.document().feature(base).unwrap().outputs()[0];
        edit(&mut a, 4, "12.3", false, true);
        activate(&mut a, 1, 1);
        activate(&mut a, 26, 0);
        let i = a.numeric_inputs.as_ref().unwrap();
        let old = newell_ui::NumericConfirmRequest {
            document: i.document.tree(),
            session: i.session,
            operation: i.operation,
            attempt: newell_ui::NumericConfirmAttemptId(1),
            through_sequence: i.consumed_sequence,
        };
        activate(&mut a, 28, 0);
        assert_eq!(a.document.revision(), 1);
        assert_eq!(a.numeric_last_attempt, 1);
        a.command_inputs
            .as_mut()
            .unwrap()
            .values
            .get_mut(&newell_ui::SelectionFieldKey {
                operation: ActionOperation::Extrude,
                key: 27,
            })
            .unwrap()
            .replace([entity]);
        a.shell.as_mut().unwrap().fail_numeric_confirm = 1;
        activate(&mut a, 28, 0);
        let current = match a.numeric_delivery.as_ref().unwrap().kind {
            NumericDeliveryKind::Confirm(r, _, _, _) => Some(r),
            _ => None,
        }
        .unwrap();
        assert_eq!(old.through_sequence, current.through_sequence);
        assert_ne!(old.attempt, current.attempt);
        assert_eq!(a.document.revision(), 2);
        assert!(
            !a.shell
                .as_mut()
                .unwrap()
                .finish_numeric_confirm(old, false)
                .unwrap()
                .any()
        );
        assert_eq!(
            a.shell.as_ref().unwrap().shell().extrude().status,
            ExtrudeStatus::Editing
        );
        let at = a.next_document_retry_deadline().unwrap();
        a.poll_document_projection(at, ProjectionTrigger::RetryDeadline);
        assert_eq!(
            a.shell.as_ref().unwrap().shell().extrude().status,
            ExtrudeStatus::Confirmed
        );
        assert_eq!(a.document.revision(), 2);
    }
    #[test]
    fn every_local_panel_keeps_its_numeric_dimensions_and_closes_without_a_model_command() {
        let mut a = app();
        click(&mut a, "Create");
        click(&mut a, "[R]evolve");
        activate(&mut a, 4, 0);
        edit(&mut a, 6, "42.5", true, true);
        activate(&mut a, 8, 0);
        edit(&mut a, 10, "3.2", true, true);
        activate(&mut a, 17, 0);
        edit(&mut a, 19, "6.7", true, true);
        activate(&mut a, 5, 0);
        activate(&mut a, 12, 0);
        edit(&mut a, 14, "4.5", false, true);
        activate(&mut a, 18, 0);
        activate(&mut a, 22, 0);
        edit(&mut a, 24, "2.4", false, true);
        let d = a.numeric_inputs.as_ref().unwrap();
        for (key, n) in [(6, 42.5), (10, 3.2), (14, 4.5), (19, 6.7), (24, 2.4)] {
            let v = &d.values[&NumericFieldKey {
                operation: ActionOperation::Revolve,
                key,
            }];
            assert!(matches!(v.quantity, Quantity::Angle(_)));
            assert!((v.quantity.value_in(Unit::Degree).unwrap() - n).abs() < 1e-12);
        }
        activate(&mut a, 28, 0);
        assert_eq!(a.document.revision(), 0);
        click(&mut a, "Create");
        click(&mut a, "[L]oft");
        activate(&mut a, 3, 0);
        edit(&mut a, 5, "1.5", false, true);
        activate(&mut a, 7, 0);
        edit(&mut a, 9, "0.0", false, true);
        formula(&mut a, 5, "1.5");
        assert!(
            a.numeric_inputs
                .as_ref()
                .unwrap()
                .values
                .values()
                .all(|v| matches!(v.quantity, Quantity::Scalar(_)))
        );
        activate(&mut a, 28, 0);
        assert_eq!(a.document.revision(), 0);
        click(&mut a, "Create");
        click(&mut a, "[S]weep");
        activate(&mut a, 5, 0);
        activate(&mut a, 6, 0);
        edit(&mut a, 7, "32.1", true, true);
        activate(&mut a, 8, 0);
        edit(&mut a, 9, "0.0", false, true);
        assert!(matches!(
            a.numeric_inputs.as_ref().unwrap().values[&NumericFieldKey {
                operation: ActionOperation::Sweep,
                key: 7
            }]
                .quantity,
            Quantity::Angle(_)
        ));
        activate(&mut a, 28, 0);
        assert_eq!(a.document.revision(), 0);
        click(&mut a, "Modify");
        click(&mut a, "Boolean");
        activate(&mut a, 0, 1);
        activate(&mut a, 4, 0);
        edit(&mut a, 6, "3.4", true, true);
        activate(&mut a, 5, 0);
        let v = &a.numeric_inputs.as_ref().unwrap().values[&NumericFieldKey {
            operation: ActionOperation::Boolean,
            key: 6,
        }];
        assert_eq!(v.quantity.value_in(Unit::Millimetre).unwrap(), 3.4);
        assert_eq!(v.direction, Direction::Reverse);
        activate(&mut a, 28, 0);
        assert_eq!(a.document.revision(), 0);
        assert_eq!(a.commands.state(), &CommandState::Idle);
    }
    #[test]
    fn namespace_replacement_invalidates_old_formula_delivery_and_command_requests() {
        let mut a = app();
        edit(&mut a, 4, "12.3", false, true);
        a.shell.as_mut().unwrap().fail_numeric_formula = 1;
        activate(&mut a, 4, 1);
        let old = match a.numeric_delivery.as_ref().unwrap().kind {
            NumericDeliveryKind::Formula(request, _) => Some(request),
            _ => None,
        }
        .unwrap();
        a.document
            .replace(newell_model::Document::default())
            .unwrap();
        a.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        assert!(a.numeric_delivery.is_none());
        assert!(a.numeric_cache.is_empty());
        assert!(a.numeric_inputs.is_none());
        assert!(!a.shell.as_ref().unwrap().shell().formula().is_open());
        click(&mut a, "Create");
        click(&mut a, "[E]xtrude");
        let session = a.numeric_inputs.as_ref().unwrap().session;
        a.complete_numeric_formula(old);
        assert_eq!(a.numeric_inputs.as_ref().unwrap().session, session);
        assert_eq!(value(&a, 4), 0.0);
        assert!(!a.shell.as_ref().unwrap().shell().formula().is_open());
    }
    #[test]
    fn exhausted_session_allocation_installs_neither_draft_and_cannot_confirm_an_old_operation() {
        let mut a = app();
        edit(&mut a, 4, "12.3", false, true);
        let session = a.numeric_inputs.as_ref().unwrap().session;
        a.next_command_id = u64::MAX;
        click(&mut a, "Create");
        click(&mut a, "[R]evolve");
        assert_eq!(a.numeric_inputs.as_ref().unwrap().session, session);
        assert_eq!(a.command_inputs.as_ref().unwrap().session, session);
        assert!(
            a.shell
                .as_ref()
                .unwrap()
                .shell()
                .numeric_session()
                .is_none()
        );
        activate(&mut a, 28, 0);
        assert!(a.shell.as_ref().unwrap().shell().command_open());
        assert_eq!(a.document.revision(), 0);
    }

    #[test]
    fn tab_focus_loss_and_unsigned_input_accept_or_restore_without_confirming() {
        let mut a = app();
        edit(&mut a, 4, "12.3", false, false);
        key(&mut a, ShellKey::Tab);
        assert_eq!(value(&a, 4), 12.3);
        assert_eq!(a.document.revision(), 0);
        edit(&mut a, 4, "7.5", false, false);
        let update = a.shell.as_mut().unwrap().numeric_focus_lost();
        a.finish_shell_input(update);
        assert_eq!(value(&a, 4), 7.5);
        assert_eq!(a.document.revision(), 0);
        assert!(a.shell.as_ref().unwrap().shell().editing_field().is_none());
        edit(&mut a, 4, ".", false, false);
        let update = a.shell.as_mut().unwrap().numeric_focus_lost();
        a.finish_shell_input(update);
        assert_eq!(value(&a, 4), 7.5);
        activate(&mut a, 18, 0);
        activate(&mut a, 19, 0);
        activate(&mut a, 22, 0);
        edit(&mut a, 24, "2.4", true, true);
        let v = &a.numeric_inputs.as_ref().unwrap().values[&field(24)];
        assert_eq!(v.direction, Direction::Forward);
        assert_eq!(v.quantity.value_in(Unit::Millimetre).unwrap(), 2.4);
    }
}
#[cfg(test)]
mod navigation_widget_tests {
    use super::*;
    fn app() -> App {
        let mut app = App::new();
        let m = GridMetrics::from_k(2).unwrap();
        app.last_size = PhysicalSize::new(656, 540);
        app.shell = Some(ShellController::new(82, 30, m).unwrap());
        app.sync_navigation_widgets();
        app
    }
    #[test]
    fn hollow_marker_centers_capture_and_empty_tile_pixels_pass_to_scene() {
        let mut app = app();
        let geometry = app.nav_geometry.clone().unwrap();
        let tile = geometry.layout.tile.unwrap();
        let empty = (tile.x + 1., tile.y + 1.);
        assert!(geometry.hit(empty).is_none());
        assert!(!geometry.reserves(empty));
        assert!(app.navigation.visible(empty));
        assert!(app.scene_navigation_at(empty));
        for (direction, center) in geometry.caps() {
            let point = (tile.x + center[0], tile.y + center[1]);
            let expected = newell_ui::navigation_widgets::NavPart::Axis(
                newell_ui::navigation_widgets::AxisCap::from_direction(direction).unwrap(),
            );
            assert_eq!(geometry.hit(point), Some(expected));
            assert!(
                app.navigation.visible(point),
                "scene still renders beneath the marker"
            );
            assert!(!app.scene_navigation_at(point));
            app.scene_pointer = Some(PhysicalPosition::new(point.0, point.1));
            app.shell
                .as_mut()
                .unwrap()
                .pointer_moved(Some(newell_ui::UiPoint {
                    x: point.0 as u32,
                    y: point.1 as u32,
                }));
            app.left_pressed();
            assert!(app.pending_scene_click.is_none());
            assert!(!app.navigation.primary_orbit_pending());
            assert!(app.nav_interaction.active());
            assert_eq!(
                app.nav_interaction
                    .release(Some(&geometry), point, app.navigation.revision()),
                Some(direction)
            );
        }
        assert!(app.navigation.begin_primary_orbit(empty, 1., true));
        let (_, center) = geometry.caps()[0];
        assert!(
            app.scene_navigation_at((tile.x + center[0], tile.y + center[1])),
            "an existing scene capture keeps ownership"
        );
    }
    #[test]
    fn reserved_press_never_arms_scene_selection_and_fps_recompose_keeps_capture() {
        let mut app = app();
        let g = app.nav_geometry.as_ref().unwrap();
        let tile = g.layout.tile.unwrap();
        let p = g
            .caps()
            .into_iter()
            .find_map(|(d, c)| {
                let p = (tile.x + c[0], tile.y + c[1]);
                (g.hit(p)
                    == Some(newell_ui::navigation_widgets::NavPart::Axis(
                        newell_ui::navigation_widgets::AxisCap::from_direction(d).unwrap(),
                    )))
                .then_some(p)
            })
            .unwrap();
        app.scene_pointer = Some(PhysicalPosition::new(p.0, p.1));
        app.shell
            .as_mut()
            .unwrap()
            .pointer_moved(Some(newell_ui::UiPoint {
                x: p.0 as u32,
                y: p.1 as u32,
            }));
        let camera = app.navigation.camera();
        let generation = app.nav_generation;
        app.left_pressed();
        assert!(app.nav_interaction.active());
        assert!(app.pending_scene_click.is_none());
        assert!(!app.navigation.primary_orbit_pending());
        assert_eq!(app.navigation.camera(), camera);
        app.sync_navigation_widgets();
        assert_eq!(app.nav_generation, generation);
        assert!(matches!(
            app.nav_interaction,
            NavInteraction::Pressed { .. }
        ));
        app.nav_interaction.advance((p.0 + 9., p.1));
        assert!(
            app.nav_interaction
                .release(app.nav_geometry.as_ref(), p, app.navigation.revision())
                .is_none()
        );
        assert_eq!(app.navigation.camera(), camera);
    }
    #[test]
    fn menu_open_blocks_widget_and_changes_only_ui_occlusion() {
        let mut app = app();
        let camera = app.navigation.camera();
        let update = app
            .shell
            .as_mut()
            .unwrap()
            .key(newell_ui::ShellKey::Alt('v'));
        app.apply(update);
        assert!(!app.shell.as_ref().unwrap().scene_input_enabled());
        assert!(!app.nav_ui_masks.is_empty());
        assert_eq!(app.navigation.camera(), camera);
    }
    fn point_at_positive_cap(app: &mut App) -> (f64, f64) {
        let g = app.nav_geometry.as_ref().unwrap();
        let t = g.layout.tile.unwrap();
        let p = g
            .caps()
            .into_iter()
            .find_map(|(d, c)| {
                let cap = newell_ui::navigation_widgets::AxisCap::from_direction(d).unwrap();
                let p = (t.x + c[0], t.y + c[1]);
                (cap.positive
                    && g.hit(p) == Some(newell_ui::navigation_widgets::NavPart::Axis(cap)))
                .then_some(p)
            })
            .unwrap();
        app.scene_pointer = Some(PhysicalPosition::new(p.0, p.1));
        app.shell
            .as_mut()
            .unwrap()
            .pointer_moved(Some(newell_ui::UiPoint {
                x: p.0 as u32,
                y: p.1 as u32,
            }));
        app.sync_navigation_widgets();
        assert!(app.nav_hover.is_some());
        p
    }
    #[test]
    fn marker_hover_is_controller_owned_and_recomputed_without_pointer_motion() {
        let mut app = app();
        let p = point_at_positive_cap(&mut app);
        let pointer = app.scene_pointer;
        let before = app.nav_geometry.as_ref().unwrap().basis;
        app.navigation
            .align_view(newell_ui::navigation_widgets::ViewDirection::new([0, 0, 1]).unwrap());
        app.sync_navigation_widgets();
        assert_ne!(before, app.nav_geometry.as_ref().unwrap().basis);
        assert_eq!(app.scene_pointer, pointer);
        let expected = app
            .nav_geometry
            .as_ref()
            .unwrap()
            .hit(p)
            .and_then(|part| match part {
                newell_ui::navigation_widgets::NavPart::Axis(cap) => Some(cap),
                _ => None,
            });
        assert_eq!(app.nav_hover, expected);
        point_at_positive_cap(&mut app);
        app.native_resizing = true;
        app.sync_navigation_widgets();
        assert_eq!(app.nav_hover, None);
        app.native_resizing = false;
        app.sync_navigation_widgets();
        assert!(app.nav_hover.is_some());
        app.scene_pointer = None;
        app.sync_navigation_widgets();
        assert_eq!(app.nav_hover, None);
    }
    #[test]
    fn marker_hover_clears_on_menu_hide_press_and_scene_capture() {
        let mut app = app();
        let p = point_at_positive_cap(&mut app);
        app.nav_interaction.press(
            app.nav_geometry.as_ref().unwrap(),
            p,
            1.,
            app.navigation.revision(),
        );
        app.sync_navigation_widgets();
        assert_eq!(app.nav_hover, None);
        app.nav_interaction
            .release(app.nav_geometry.as_ref(), p, app.navigation.revision());
        app.sync_navigation_widgets();
        assert!(app.nav_hover.is_some());
        let v = app.navigation.camera().viewport;
        assert!(app.navigation.begin_primary_orbit(
            (f64::from(v.x + v.width / 2), f64::from(v.y + v.height / 2)),
            1.,
            true
        ));
        app.sync_navigation_widgets();
        assert_eq!(app.nav_hover, None);
        app.navigation.cancel();
        app.sync_navigation_widgets();
        assert!(app.nav_hover.is_some());
        let update = app
            .shell
            .as_mut()
            .unwrap()
            .key(newell_ui::ShellKey::Alt('v'));
        app.apply(update);
        assert_eq!(app.nav_hover, None);
        for _ in 0..8 {
            app.shell.as_mut().unwrap().key(newell_ui::ShellKey::Down);
        }
        let update = app.shell.as_mut().unwrap().key(newell_ui::ShellKey::Enter);
        app.apply(update);
        assert!(!app.shell.as_ref().unwrap().shell().view_panel_visible(3));
        assert!(app.nav_geometry.as_ref().unwrap().layout.tile.is_none());
        assert_eq!(app.nav_hover, None);
    }
}

#[cfg(test)]
mod real_extrude_tests {
    use super::*;
    use newell_ui::{EditKey, ShellKey, UiPoint, WidgetId};
    use std::sync::mpsc;
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    #[test]
    fn dead_worker_keeps_the_document_and_explicit_ok_rebuilds_in_a_new_lifetime() -> TestResult {
        let (mut app, wake) = runtime_app()?;
        open(&mut app);
        profile(&mut app);
        depth(&mut app, "#Depth := 2mm", true);
        pump(&mut app, &wake)?;
        let source = app.document.document().clone();
        let version = app.document.version();
        let token = app.document.geometry_token();
        let events = app.document.subscribe();
        let bindings = app
            .numeric_inputs
            .as_ref()
            .ok_or("inputs")?
            .bindings
            .clone();
        app.geometry
            .as_ref()
            .ok_or("runtime")?
            .worker
            .stop_for_test()?;
        // A completion wake may already be queued. Wakes request inspection;
        // only the lifetime flag certifies that the worker has actually exited.
        let stopped_by = Instant::now() + Duration::from_secs(10);
        while app.geometry.as_ref().ok_or("runtime")?.worker.is_alive() {
            let remaining = stopped_by.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("worker did not exit after shutdown".into());
            }
            wake.recv_timeout(remaining)?;
        }
        assert!(!app.geometry.as_ref().ok_or("runtime")?.worker.is_alive());
        let update = app.drain_worker_reply();
        app.apply(update);
        app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        assert_eq!(app.document.document(), &source);
        assert_eq!(app.document.version(), version);
        assert_eq!(app.document.geometry_token(), token);
        assert_eq!(events.try_iter().count(), 0);
        assert!(app.scene_available);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        assert_eq!(app.geometry.as_ref().ok_or("runtime")?.epoch, 2);
        assert!(app.geometry.as_ref().ok_or("runtime")?.worker.is_alive());
        assert_eq!(app.document.revision(), version.revision + 1);
        assert_eq!(app.document.document().features().len(), 2);
        assert_eq!(events.try_iter().count(), 1);
        let newell_model::Feature::Extrude(extrude) = &app.document.document().features()[1].1
        else {
            return Err("missing Extrude".into());
        };
        let newell_model::ExtentSpec::Blind {
            depth: newell_model::ParameterValue::Bound(parameter),
            ..
        } = extrude.spec.extent
        else {
            return Err("lost binding".into());
        };
        assert!(bindings.values().any(|id| *id == parameter));
        assert_eq!(
            app.document.geometry_token().ok_or("token")?.worker_epoch(),
            2
        );
        Ok(())
    }
    #[test]
    fn successful_current_geometry_clears_the_prior_visible_error() -> TestResult {
        let (mut app, wake) = runtime_app()?;
        open(&mut app);
        profile(&mut app);
        let source = app.document.document().clone();
        let version = app.document.version();
        let events = app.document.subscribe();
        let text = |app: &App| -> String {
            app.shell
                .as_ref()
                .unwrap()
                .grid()
                .cells()
                .iter()
                .map(|cell| match cell.content {
                    newell_ui::CellContent::Glyph(ch) => ch,
                    _ => ' ',
                })
                .collect()
        };
        // Enter a real invalid value; an untouched placeholder is not accepted input.
        depth(&mut app, "0", false);
        pump(&mut app, &wake)?;
        assert!(text(&app).contains("greater than zero"), "{}", text(&app));
        assert_eq!(app.document.document(), &source);
        assert_eq!(app.document.version(), version);
        assert_eq!(events.try_iter().count(), 0);
        depth(&mut app, "2", false);
        pump(&mut app, &wake)?;
        assert!(!text(&app).contains("greater than zero"));
        assert_eq!(app.document.document(), &source);
        assert_eq!(app.document.version(), version);
        assert_eq!(events.try_iter().count(), 0);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        assert!(!text(&app).contains("greater than zero"));
        assert_eq!(app.document.document().features().len(), 2);
        assert_eq!(app.document.revision(), version.revision + 1);
        assert_eq!(events.try_iter().count(), 1);
        Ok(())
    }
    pub(super) fn pump(
        app: &mut App,
        wake: &mpsc::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for _ in 0..64 {
            let computing = app.geometry.as_ref().is_some_and(|runtime| {
                runtime.bootstrap.is_some()
                    || runtime
                        .coordinator
                        .as_ref()
                        .is_some_and(|coordinator| coordinator.computing())
            });
            if computing {
                wake.recv_timeout(Duration::from_secs(10))?;
                let update = app.drain_worker_reply();
                app.apply(update);
                continue;
            }
            if let Some(deadline) = app.next_document_retry_deadline() {
                let update =
                    app.poll_document_projection(deadline, ProjectionTrigger::RetryDeadline);
                app.apply(update);
                continue;
            }
            return Ok(());
        }
        Err("geometry did not settle within bounded replies".into())
    }
    pub(super) fn runtime_app() -> Result<(App, mpsc::Receiver<()>), Box<dyn std::error::Error>> {
        let mut app = super::tests::live_app();
        if app
            .shell
            .as_ref()
            .is_some_and(|shell| shell.shell().command_open())
        {
            super::tests::click(&mut app, "Cancel");
        }
        let (sender, wake) = mpsc::channel();
        app.enable_geometry_worker(move || {
            let _ = sender.send(());
        })?;
        app.document.add_base_solid("Base Solid")?;
        app.queue_geometry_bootstrap()?;
        pump(&mut app, &wake)?;
        assert!(app.scene_available);
        assert_eq!(app.scene_bindings.len(), 1);
        assert_eq!(app.document.document().features().len(), 1);
        Ok((app, wake))
    }

    #[test]
    fn generation_qualified_face_selection_is_independent_and_multiselects() -> TestResult {
        let (mut app, _wake) = runtime_app()?;
        let feature = app.document.document().features()[0].0;
        let entity = app.document.document().feature(feature).unwrap().outputs()[0];
        let geometry = app
            .document
            .document()
            .entity_geometry(entity)
            .unwrap()
            .clone();
        let point = app.navigation.camera().project([0.5, 0.5, 0.5]).unwrap();
        let hit = app.navigation.pick(point).ok_or("viewport face pick")?;
        let binding = app
            .scene_bindings
            .iter()
            .find(|binding| {
                usize::try_from(binding.range.face_start).is_ok_and(|start| hit.face_index >= start)
                    && usize::try_from(binding.range.face_end).is_ok_and(|end| hit.face_index < end)
            })
            .ok_or("face scene binding")?;
        let crate::geometry_worker::SceneItemBinding::Committed {
            entity: hit_entity,
            generation,
        } = binding.binding
        else {
            return Err("committed face binding".into());
        };
        assert_eq!(hit_entity, entity);
        let first = newell_model::geometry::FaceRef {
            entity,
            generation,
            face: u32::try_from(hit.face_index - binding.range.face_start as usize)?,
        };
        let second = newell_model::geometry::FaceRef {
            entity,
            generation: geometry.generation,
            face: geometry
                .faces
                .iter()
                .find(|face| face.face != first.face)
                .ok_or("second face")?
                .face,
        };

        assert!(app.navigation.begin_primary_orbit(point, 1.0, true));
        app.pending_scene_click = Some(PendingSceneClick {
            document: app.document.identity(),
            generation: app.shell.as_ref().unwrap().generation(),
            mesh_revision: app.navigation.scene().revision(),
            edit: SelectionEdit::Replace,
        });
        let certificate = app.navigation.finish_primary(point);
        let update = app.finish_scene_selection(point, certificate);
        app.apply(update);
        assert_eq!(app.selection.entities(), &[entity]);
        assert_eq!(app.face_selection, [first]);
        app.handle_face_selection_request(entity, second, SelectionOp::Add);
        assert_eq!(app.selection.entities(), &[entity]);
        assert_eq!(app.face_selection, [first, second]);

        // The linked tree row selects the entity as a whole and clears the
        // subshape override; it does not leave a stale face accent behind.
        app.handle_selection_request(SelectionRequest {
            document: app.document.identity(),
            op: SelectionOp::Replace,
            entities: vec![entity],
        });
        assert!(app.face_selection.is_empty());
        assert_eq!(app.selection.entities(), &[entity]);

        app.handle_face_selection_request(entity, first, SelectionOp::Replace);
        let stale = newell_model::geometry::FaceRef {
            generation: newell_model::geometry::GeometryGeneration(
                first.generation.0.saturating_add(1),
            ),
            ..first
        };
        app.handle_face_selection_request(entity, stale, SelectionOp::Add);
        assert_eq!(app.face_selection, [first]);
        Ok(())
    }

    #[test]
    fn entity_visibility_republishes_only_the_stable_entity_after_rename() -> TestResult {
        let (mut app, wake) = runtime_app()?;
        let base_feature = app.document.document().features()[0].0;
        let base_entity = app
            .document
            .document()
            .feature(base_feature)
            .unwrap()
            .outputs()[0];
        let other_feature = app.document.add_base_solid("Other Solid")?;
        app.queue_geometry_bootstrap()?;
        pump(&mut app, &wake)?;
        let other_entity = app
            .document
            .document()
            .feature(other_feature)
            .unwrap()
            .outputs()[0];
        assert_eq!(app.scene_bindings.len(), 2);

        app.document
            .rename_feature(base_feature, "Renamed Base Operation".into())?;
        app.document
            .rename_entity(base_entity, "Renamed Cube".into())?;
        let update = app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        app.apply(update);
        let geometry_generation = app.document.document().geometry_generation();
        let original_geometry = app
            .document
            .document()
            .entity_geometry(base_entity)
            .ok_or("base solid geometry")?;
        let original_measure = original_geometry.measure;
        let original_entity_generation = original_geometry.generation;
        let revision = app.document.revision();

        // The Features control resolves the output ID after both labels change.
        super::tests::click(&mut app, "■ Renamed Ba…");
        assert_eq!(app.document.revision(), revision + 1);
        assert!(
            !app.document
                .document()
                .entity(base_entity)
                .unwrap()
                .visible()
        );
        assert!(
            app.document
                .document()
                .entity(other_entity)
                .unwrap()
                .visible()
        );
        assert_eq!(app.document.document().features().len(), 2);
        assert_eq!(
            app.document.document().geometry_generation(),
            geometry_generation
        );
        let hidden_geometry = app
            .document
            .document()
            .entity_geometry(base_entity)
            .ok_or("retained base geometry")?;
        assert_eq!(hidden_geometry.generation, original_entity_generation);
        assert_eq!(hidden_geometry.measure, original_measure);
        let visible_bindings: Vec<_> = app
            .scene_bindings
            .iter()
            .filter_map(|item| match item.binding {
                crate::geometry_worker::SceneItemBinding::Committed { entity, .. } => Some(entity),
                crate::geometry_worker::SceneItemBinding::Preview { .. } => None,
            })
            .collect();
        assert_eq!(visible_bindings, [other_entity]);
        assert!(app.navigation.scene().bounds().is_some());
        assert!(app.scene_available);
        let projection = app.shell.as_ref().unwrap().shell().tree_projection();
        assert!(projection.elements.iter().any(|item| {
            item.id.get() == base_entity.get() && !item.visible && item.name == "Renamed Cube"
        }));
        assert!(
            projection
                .elements
                .iter()
                .any(|item| { item.id.get() == other_entity.get() && item.visible })
        );
        assert!(projection.features.iter().any(|item| {
            item.name == "Renamed Base Operation" && item.visibility == Some(false)
        }));

        super::tests::click(&mut app, "□ Renamed …");
        assert!(
            app.document
                .document()
                .entity(base_entity)
                .unwrap()
                .visible()
        );
        assert_eq!(
            app.document.document().geometry_generation(),
            geometry_generation
        );
        let restored_bindings: Vec<_> = app
            .scene_bindings
            .iter()
            .filter_map(|item| match item.binding {
                crate::geometry_worker::SceneItemBinding::Committed { entity, .. } => Some(entity),
                crate::geometry_worker::SceneItemBinding::Preview { .. } => None,
            })
            .collect();
        assert_eq!(restored_bindings, [base_entity, other_entity]);
        assert!(
            app.shell
                .as_ref()
                .unwrap()
                .shell()
                .tree_projection()
                .features
                .iter()
                .any(|item| item.name == "Renamed Base Operation" && item.visibility == Some(true))
        );
        assert!(app.document.document().feature(base_feature).is_some());
        assert!(app.scene_available);
        Ok(())
    }
    fn activate(app: &mut App, key: u32, slot: u32) {
        let shell = app.shell.as_mut().unwrap();
        let base = shell.shell().extrude().depth_field_id().get() - 16;
        let id = WidgetId::new(base + key * 4 + slot);
        let point = (0..shell.grid().rows())
            .find_map(|row| {
                (0..shell.grid().cols())
                    .find(|col| shell.grid().get(*col, row).unwrap().widget_id == Some(id))
                    .map(|col| UiPoint {
                        x: col * 4 + 2,
                        y: row * 9 + 4,
                    })
            })
            .expect("visible command control");
        shell.pointer_moved(Some(point));
        let update = shell.pointer_pressed();
        shell.pointer_released();
        app.finish_shell_input(update);
    }
    pub(super) fn open(app: &mut App) {
        super::tests::click(app, "Create");
        super::tests::click(app, "[E]xtrude");
    }
    fn depth(app: &mut App, source: &str, formula: bool) {
        activate(app, 4, u32::from(formula));
        super::tests::key(app, ShellKey::Edit(EditKey::SelectAll));
        for character in source.chars() {
            super::tests::key(app, ShellKey::Char(character));
        }
        super::tests::key(app, ShellKey::Enter);
    }
    pub(super) fn profile(app: &mut App) {
        let version = app.document.version();
        activate(app, 2, 0);
        assert!(app.shell.as_ref().unwrap().shell().selection().is_open());
        // Only the generic modal's real candidate row applies a supporting face.
        super::tests::click(app, "Base Solid / Face 1");
        super::tests::click(app, "OK");
        assert_eq!(app.document.version(), version);
        assert!(
            app.command_inputs
                .as_ref()
                .unwrap()
                .geometry_values
                .get(&newell_ui::SelectionFieldKey {
                    operation: ActionOperation::Extrude,
                    key: 2
                })
                .is_some_and(|values| values.len() == 1)
        );
    }
    #[test]
    fn real_face_formula_preview_cancel_and_exactly_once_confirmation() -> TestResult {
        let (mut app, wake) = runtime_app()?;
        let baseline = app.document.document().clone();
        let version = app.document.version();
        open(&mut app);
        profile(&mut app);
        depth(&mut app, "#Depth := 5.25mm", true);
        pump(&mut app, &wake)?;
        assert_eq!(app.document.document(), &baseline);
        assert_eq!(app.document.version(), version);
        assert_eq!(app.scene_bindings.len(), 2);
        assert!(app.scene_bindings.iter().any(|item| matches!(
            item.binding,
            crate::geometry_worker::SceneItemBinding::Preview { .. }
        )));
        assert_eq!(
            app.geometry
                .as_ref()
                .unwrap()
                .displayed_scene
                .as_ref()
                .unwrap()
                .items
                .len(),
            2
        );
        activate(&mut app, 28, 1);
        pump(&mut app, &wake)?;
        assert_eq!(app.document.document(), &baseline);
        assert_eq!(app.scene_bindings.len(), 1);
        assert_eq!(
            app.geometry
                .as_ref()
                .unwrap()
                .displayed_scene
                .as_ref()
                .unwrap()
                .generation,
            app.document.document().geometry_generation()
        );
        open(&mut app);
        profile(&mut app);
        depth(&mut app, "#Depth := 5.25mm", true);
        pump(&mut app, &wake)?;
        let before = app.document.revision();
        let events = app.document.subscribe();
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        assert_eq!(app.document.revision(), before + 1);
        assert_eq!(app.document.document().features().len(), 2);
        assert_eq!(events.try_iter().count(), 1);
        let (feature, operation) = app
            .document
            .document()
            .features()
            .last()
            .ok_or("missing committed extrude")?;
        let newell_model::Feature::Extrude(extrude) = operation else {
            return Err("wrong feature".into());
        };
        let output = extrude.outputs[0];
        let geometry = app
            .document
            .document()
            .entity_geometry(output)
            .ok_or("missing actual geometry")?;
        assert!((geometry.measure - 5.25).abs() < 1e-6);
        assert_eq!(geometry.face_count, 6);
        let newell_model::ParameterValue::Bound(parameter) = extrude.spec.inputs().depth else {
            return Err("formula identity was not retained".into());
        };
        assert_eq!(
            app.document
                .document()
                .parameters()
                .definition(parameter)
                .map(|definition| definition.owner),
            Some(newell_model::parameters::ParameterOwner::Feature(*feature))
        );
        assert!(app.scene_bindings.iter().all(|item| matches!(
            item.binding,
            crate::geometry_worker::SceneItemBinding::Committed { .. }
        )));
        assert!(app.selection.entities().contains(&output));
        assert!(app.scene_available);
        for _ in 0..3 {
            app.finish_shell_input(Update::default());
        }
        assert_eq!(app.document.revision(), before + 1);
        assert_eq!(app.document.document().features().len(), 2);
        Ok(())
    }
    fn properties(app: &mut App) {
        super::tests::click(app, "Utilities");
        super::tests::click(app, "Properties");
    }
    #[test]
    fn committed_scene_upload_retries_once_then_suspends_without_repeating_the_commit() -> TestResult
    {
        for failures in [1, 2] {
            let (mut app, wake) = runtime_app()?;
            open(&mut app);
            profile(&mut app);
            depth(&mut app, "2", false);
            pump(&mut app, &wake)?;
            let revision = app.document.revision();
            let events = app.document.subscribe();
            app.fail_scene_publish = failures;
            activate(&mut app, 28, 0);
            assert_eq!(app.document.revision(), revision + 1);
            let token = app.document.geometry_token();
            let accepted = app.document.document().clone();
            assert!(app.numeric_committed);
            assert!(!app.scene_available);
            let deadline = app
                .pending_scene
                .as_ref()
                .and_then(|pending| pending.retry_at)
                .ok_or("scene retry missing")?;
            assert!(
                !app.geometry
                    .as_ref()
                    .ok_or("runtime")?
                    .coordinator
                    .as_ref()
                    .ok_or("coordinator")?
                    .computing()
            );
            app.poll_document_projection(deadline, ProjectionTrigger::RetryDeadline);
            if failures == 2 {
                let pending = app.pending_scene.as_ref().ok_or("suspended scene")?;
                assert_eq!(pending.attempts, 2);
                assert!(pending.retry_at.is_none());
                assert!(!app.scene_available);
                assert!(app.numeric_delivery.is_some());
                for _ in 0..3 {
                    app.poll_document_projection(
                        deadline + Duration::from_secs(1),
                        ProjectionTrigger::Notification,
                    );
                }
                assert_eq!(
                    app.pending_scene
                        .as_ref()
                        .ok_or("suspended scene")?
                        .attempts,
                    2
                );
                assert_eq!(app.document.document(), &accepted);
                app.poll_document_projection(
                    deadline + Duration::from_secs(2),
                    ProjectionTrigger::UserActivation,
                );
            }
            assert!(app.pending_scene.is_none());
            assert!(app.scene_available);
            assert!(app.numeric_delivery.is_none());
            assert_eq!(app.document.revision(), revision + 1);
            assert_eq!(app.document.document(), &accepted);
            assert_eq!(app.document.geometry_token(), token);
            assert_eq!(events.try_iter().count(), 1);
            assert_eq!(app.document.document().features().len(), 2);
        }
        Ok(())
    }
    #[test]
    fn properties_replays_suffix_and_remaps_a_unique_cap_atomically() -> TestResult {
        let (mut app, wake) = runtime_app()?;
        open(&mut app);
        profile(&mut app);
        depth(&mut app, "#Depth := 3mm", true);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        let first = app.document.document().features().last().ok_or("first")?.0;
        let first_output = app
            .document
            .document()
            .feature(first)
            .ok_or("first")?
            .outputs()[0];
        let cap = app
            .document
            .document()
            .entity_geometry(first_output)
            .ok_or("first geometry")?
            .faces
            .iter()
            .find(|face| face.provenance == newell_model::geometry::FaceProvenance::EndCap)
            .ok_or("end cap")?
            .face;
        open(&mut app);
        activate(&mut app, 2, 0);
        let update = app.shell.as_mut().ok_or("shell")?.scroll(i32::MIN);
        app.finish_shell_input(update);
        super::tests::click(&mut app, &format!("Extrude / Face {}", u64::from(cap) + 1));
        super::tests::click(&mut app, "OK");
        depth(&mut app, "2", false);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        let second = app.document.document().features().last().ok_or("second")?.0;
        let second_output = app
            .document
            .document()
            .feature(second)
            .ok_or("second")?
            .outputs()[0];
        let revision = app.document.revision();
        let events = app.document.subscribe();
        super::tests::click(&mut app, "Extrude");
        properties(&mut app);
        assert_eq!(
            app.extrude_edit_target.map(|target| target.feature),
            Some(first)
        );
        depth(&mut app, "#Depth := 4.5mm", true);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        assert_eq!(app.document.revision(), revision + 1);
        assert_eq!(events.try_iter().count(), 1);
        let document = app.document.document();
        assert_eq!(document.features().len(), 3);
        assert_eq!(
            document.feature(first).ok_or("first")?.outputs(),
            &[first_output]
        );
        assert_eq!(
            document.feature(second).ok_or("second")?.outputs(),
            &[second_output]
        );
        let first_geometry = document
            .entity_geometry(first_output)
            .ok_or("first geometry")?;
        let second_geometry = document
            .entity_geometry(second_output)
            .ok_or("second geometry")?;
        let newell_model::Feature::Extrude(extrude) = document.feature(second).ok_or("second")?
        else {
            return Err("wrong feature".into());
        };
        assert_eq!(
            extrude
                .spec
                .inputs()
                .profile_geometry
                .ok_or("profile")?
                .support()
                .generation,
            first_geometry.generation
        );
        assert!(second_geometry.faces.iter().any(|face| face.provenance
            == newell_model::geometry::FaceProvenance::EndCap
            && (face.frame.origin[2] + 6.5).abs() < 1e-6));
        assert!((first_geometry.measure - 4.5).abs() < 1e-6);
        assert!((second_geometry.measure - 2.).abs() < 1e-6);
        Ok(())
    }
    #[test]
    fn properties_can_reopen_zero_output_remove_and_cancel_is_read_only() -> TestResult {
        let (mut app, wake) = runtime_app()?;
        open(&mut app);
        profile(&mut app);
        depth(&mut app, "-2", false);
        activate(&mut app, 1, 2);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        let feature = app.document.document().features().last().ok_or("remove")?.0;
        assert!(
            app.document
                .document()
                .feature(feature)
                .ok_or("remove")?
                .outputs()
                .is_empty()
        );
        let baseline = app.document.document().clone();
        let revision = app.document.revision();
        super::tests::click(&mut app, "Extrude");
        properties(&mut app);
        assert_eq!(
            app.extrude_edit_target.map(|target| target.feature),
            Some(feature)
        );
        depth(&mut app, "0.5", false);
        pump(&mut app, &wake)?;
        activate(&mut app, 28, 1);
        pump(&mut app, &wake)?;
        assert_eq!(app.document.document(), &baseline);
        assert_eq!(app.document.revision(), revision);
        assert!(app.numeric_inputs.is_none());
        assert!(app.extrude_edit_target.is_none());
        super::tests::click(&mut app, "Extrude");
        properties(&mut app);
        depth(&mut app, "0.5", false);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        assert_eq!(app.document.revision(), revision + 1);
        assert_eq!(app.document.document().features().len(), 2);
        let output = app
            .document
            .document()
            .feature(feature)
            .ok_or("remove")?
            .outputs()[0];
        assert!(
            (app.document
                .document()
                .entity_geometry(output)
                .ok_or("remaining body")?
                .measure
                - 0.5)
                .abs()
                < 1e-6
        );
        Ok(())
    }
    #[test]
    fn properties_close_and_editor_retry_once_without_opening_early_or_mutating_document()
    -> TestResult {
        let (mut app, wake) = runtime_app()?;
        open(&mut app);
        profile(&mut app);
        depth(&mut app, "2", false);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        let source = app.document.document().clone();
        let events = app.document.subscribe();
        super::tests::click(&mut app, "Extrude");
        app.shell.as_mut().ok_or("shell")?.fail_properties_close = 1;
        app.shell.as_mut().ok_or("shell")?.fail_properties_editor = 1;
        properties(&mut app);
        assert!(!app.shell.as_ref().ok_or("shell")?.shell().command_open());
        assert!(app.extrude_edit_target.is_none());
        assert_eq!(app.document.document(), &source);
        assert!(app.next_document_retry_deadline().is_some());
        pump(&mut app, &wake)?;
        assert!(app.shell.as_ref().ok_or("shell")?.shell().command_open());
        assert!(app.extrude_edit_target.is_some());
        assert_eq!(app.last_properties_id, 1);
        assert_eq!(app.document.document(), &source);
        assert_eq!(events.try_iter().count(), 0);
        let revision = app.document.revision();
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        assert_eq!(app.document.revision(), revision + 1);
        assert_eq!(app.document.document().features().len(), 2);
        Ok(())
    }
    #[test]
    fn properties_editor_suspends_after_two_failures_and_rearms_without_a_model_change()
    -> TestResult {
        let (mut app, wake) = runtime_app()?;
        open(&mut app);
        profile(&mut app);
        depth(&mut app, "2", false);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        let source = app.document.document().clone();
        let version = app.document.version();
        let events = app.document.subscribe();
        super::tests::click(&mut app, "Extrude");
        app.shell.as_mut().ok_or("shell")?.fail_properties_editor = 2;
        properties(&mut app);
        let deadline = app.properties_retry_at.ok_or("first retry")?;
        assert!(app.properties_editor.is_some());
        app.deliver_properties(deadline, false);
        assert!(app.properties_editor.is_some());
        assert!(app.properties_retry_at.is_none());
        assert_eq!(app.properties_attempts, 2);
        for _ in 0..3 {
            app.deliver_properties(deadline + Duration::from_secs(1), false);
        }
        assert!(!app.shell.as_ref().ok_or("shell")?.shell().command_open());
        assert!(app.extrude_edit_target.is_none());
        assert_eq!(app.document.version(), version);
        assert_eq!(app.document.document(), &source);
        app.deliver_properties(deadline + Duration::from_secs(2), true);
        assert!(app.shell.as_ref().ok_or("shell")?.shell().command_open());
        assert!(app.extrude_edit_target.is_some());
        assert!(app.properties_editor.is_none());
        assert_eq!(app.last_properties_id, 1);
        assert_eq!(app.document.version(), version);
        assert_eq!(app.document.document(), &source);
        assert_eq!(events.try_iter().count(), 0);
        pump(&mut app, &wake)?;
        Ok(())
    }
    #[test]
    fn properties_pending_editor_is_discarded_when_its_source_document_is_replaced() -> TestResult {
        let (mut app, wake) = runtime_app()?;
        open(&mut app);
        profile(&mut app);
        depth(&mut app, "2", false);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        super::tests::click(&mut app, "Extrude");
        app.shell.as_mut().ok_or("shell")?.fail_properties_editor = 1;
        properties(&mut app);
        let deadline = app.properties_retry_at.ok_or("first retry")?;
        assert!(app.properties_editor.is_some());
        app.document.replace(newell_model::Document::default())?;
        let version = app.document.version();
        app.poll_document_projection(deadline, ProjectionTrigger::Notification);
        app.deliver_properties(deadline + Duration::from_secs(1), true);
        assert!(app.properties_editor.is_none());
        assert!(app.properties_retry_at.is_none());
        assert!(app.extrude_edit_target.is_none());
        assert!(app.numeric_inputs.is_none());
        assert!(!app.shell.as_ref().ok_or("shell")?.shell().command_open());
        assert_eq!(app.document.version(), version);
        assert!(app.document.document().features().is_empty());
        pump(&mut app, &wake)?;
        Ok(())
    }
    #[test]
    fn properties_uses_focused_operation_preserves_bindings_and_replaces_one_feature() -> TestResult
    {
        let (mut app, wake) = runtime_app()?;
        open(&mut app);
        profile(&mut app);
        depth(&mut app, "#Depth := 3.25mm", true);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        let (feature, parameter) =
            match app.document.document().features().last().ok_or("feature")? {
                (id, newell_model::Feature::Extrude(extrude)) => {
                    let newell_model::ParameterValue::Bound(parameter) =
                        extrude.spec.inputs().depth
                    else {
                        return Err("binding".into());
                    };
                    (*id, parameter)
                }
                _ => return Err("wrong feature".into()),
            };
        super::tests::click(&mut app, "Extrude");
        let base = app.document.document().features()[0].1.outputs()[0];
        let update = app.handle_selection_request(SelectionRequest {
            document: app.document.identity(),
            op: SelectionOp::Replace,
            entities: vec![base],
        });
        app.apply(update);
        let revision = app.document.revision();
        let events = app.document.subscribe();
        properties(&mut app);
        assert_eq!(
            app.extrude_edit_target.map(|target| target.feature),
            Some(feature)
        );
        assert_eq!(app.selection.entities(), &[base]);
        let inputs = app.numeric_inputs.as_ref().ok_or("edit draft")?;
        assert_eq!(
            inputs.bindings.get(&newell_ui::NumericFieldKey {
                operation: ActionOperation::Extrude,
                key: 4
            }),
            Some(&parameter)
        );
        depth(&mut app, "#Depth := 4.5mm", true);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        assert_eq!(app.document.revision(), revision + 1);
        assert_eq!(events.try_iter().count(), 1);
        assert_eq!(app.document.document().features().len(), 2);
        assert_eq!(
            app.document
                .document()
                .features()
                .last()
                .ok_or("edited feature")?
                .0,
            feature
        );
        let newell_model::Feature::Extrude(extrude) =
            app.document.document().feature(feature).ok_or("feature")?
        else {
            return Err("wrong feature".into());
        };
        assert_eq!(
            extrude.spec.inputs().depth,
            newell_model::ParameterValue::Bound(parameter)
        );
        assert!(
            (app.document
                .document()
                .entity_geometry(extrude.outputs[0])
                .ok_or("edited geometry")?
                .measure
                - 4.5)
                .abs()
                < 1e-6
        );
        Ok(())
    }
    #[test]
    fn real_add_replaces_the_active_body_without_changing_its_historical_input() -> TestResult {
        let (mut app, wake) = runtime_app()?;
        let base = app.document.document().features()[0].1.outputs()[0];
        let baseline = app
            .document
            .document()
            .entity_geometry(base)
            .cloned()
            .ok_or("baseline geometry")?;
        open(&mut app);
        profile(&mut app);
        depth(&mut app, "2", false);
        activate(&mut app, 1, 1);
        pump(&mut app, &wake)?;
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        let document = app.document.document();
        let newell_model::Feature::Extrude(extrude) =
            &document.features().last().ok_or("feature")?.1
        else {
            return Err("wrong feature".into());
        };
        assert_eq!(extrude.consumed_inputs(), &[base]);
        let output = extrude.outputs[0];
        assert_ne!(output, base);
        assert!((document.entity_geometry(output).ok_or("result")?.measure - 3.).abs() < 1e-6);
        assert_eq!(document.entity_geometry(base), Some(&baseline));
        assert!(document.entity(base).is_some_and(|entity| entity.visible()));
        assert_eq!(
            document
                .active_entities_at(None)?
                .into_iter()
                .collect::<Vec<_>>(),
            vec![output]
        );
        assert_eq!(app.scene_bindings.len(), 1);
        assert_eq!(
            app.shell
                .as_ref()
                .ok_or("shell")?
                .shell()
                .tree_projection()
                .elements
                .len(),
            1
        );
        Ok(())
    }
    #[test]
    fn real_remove_can_commit_zero_outputs_and_retains_its_operation() -> TestResult {
        let (mut app, wake) = runtime_app()?;
        let base = app.document.document().features()[0].1.outputs()[0];
        let revision = app.document.revision();
        open(&mut app);
        profile(&mut app);
        depth(&mut app, "-2", false);
        activate(&mut app, 1, 2);
        pump(&mut app, &wake)?;
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        let document = app.document.document();
        assert_eq!(app.document.revision(), revision + 1);
        let newell_model::Feature::Extrude(extrude) =
            &document.features().last().ok_or("feature")?.1
        else {
            return Err("wrong feature".into());
        };
        assert!(extrude.outputs.is_empty());
        assert_eq!(extrude.consumed_inputs(), &[base]);
        assert!(document.active_entities_at(None)?.is_empty());
        assert!(document.entity(base).is_some());
        assert!(app.scene_bindings.is_empty());
        assert!(app.selection.entities().is_empty());
        assert!(
            app.shell
                .as_ref()
                .ok_or("shell")?
                .shell()
                .tree_projection()
                .elements
                .is_empty()
        );
        assert_eq!(document.features().len(), 2);
        assert!(app.scene_available);
        Ok(())
    }
    #[test]
    fn real_surface_add_and_remove_replace_only_active_surface_versions() -> TestResult {
        let (mut app, wake) = runtime_app()?;
        open(&mut app);
        activate(&mut app, 0, 1);
        profile(&mut app);
        depth(&mut app, "3", false);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        let first = app
            .document
            .document()
            .features()
            .last()
            .ok_or("first sheet")?
            .1
            .outputs()[0];
        open(&mut app);
        activate(&mut app, 0, 1);
        profile(&mut app);
        depth(&mut app, "5", false);
        activate(&mut app, 1, 1);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        let newell_model::Feature::Extrude(add) = &app
            .document
            .document()
            .features()
            .last()
            .ok_or("add sheet")?
            .1
        else {
            return Err("wrong feature".into());
        };
        assert_eq!(add.consumed_inputs(), &[first]);
        let added = add.outputs[0];
        assert!(
            (app.document
                .document()
                .entity_geometry(added)
                .ok_or("added geometry")?
                .measure
                - 20.)
                .abs()
                < 1e-6
        );
        assert_eq!(app.scene_bindings.len(), 2);
        open(&mut app);
        activate(&mut app, 0, 1);
        profile(&mut app);
        depth(&mut app, "2", false);
        activate(&mut app, 1, 2);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        let newell_model::Feature::Extrude(remove) = &app
            .document
            .document()
            .features()
            .last()
            .ok_or("remove sheet")?
            .1
        else {
            return Err("wrong feature".into());
        };
        assert_eq!(remove.consumed_inputs(), &[added]);
        let output = remove.outputs[0];
        assert!(
            (app.document
                .document()
                .entity_geometry(output)
                .ok_or("remaining geometry")?
                .measure
                - 12.)
                .abs()
                < 1e-6
        );
        assert_eq!(app.scene_bindings.len(), 2);
        assert_eq!(app.document.document().active_entities_at(None)?.len(), 2);
        assert!(
            !app.document
                .document()
                .active_entities_at(None)?
                .contains(&first)
        );
        assert!(
            !app.document
                .document()
                .active_entities_at(None)?
                .contains(&added)
        );
        Ok(())
    }
    #[test]
    fn real_surface_through_all_uses_hidden_active_sheet_geometry() -> TestResult {
        let (mut app, wake) = runtime_app()?;
        open(&mut app);
        activate(&mut app, 0, 1);
        profile(&mut app);
        depth(&mut app, "3", false);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        let sheet = app
            .document
            .document()
            .features()
            .last()
            .ok_or("sheet")?
            .1
            .outputs()[0];
        app.document.set_entity_visibility(sheet, false)?;
        let update = app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        app.apply(update);
        let revision = app.document.revision();
        open(&mut app);
        activate(&mut app, 0, 1);
        profile(&mut app);
        activate(&mut app, 3, 1);
        activate(&mut app, 3, 1);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        assert_eq!(app.document.revision(), revision + 1);
        let document = app.document.document();
        let output = document.features().last().ok_or("ThroughAll")?.1.outputs()[0];
        assert!(
            (document.entity_geometry(output).ok_or("geometry")?.measure - 12.000004).abs() < 1e-6
        );
        assert!(
            document
                .entity(sheet)
                .is_some_and(|entity| !entity.visible())
        );
        assert_eq!(document.active_entities_at(None)?.len(), 3);
        assert_eq!(app.scene_bindings.len(), 2);
        Ok(())
    }
    #[test]
    fn real_surface_new_publishes_uncapped_faces_and_surface_metadata() -> TestResult {
        let (mut app, wake) = runtime_app()?;
        let revision = app.document.revision();
        open(&mut app);
        activate(&mut app, 0, 1);
        profile(&mut app);
        depth(&mut app, "3", false);
        pump(&mut app, &wake)?;
        assert_eq!(app.document.revision(), revision);
        assert_eq!(app.scene_bindings.len(), 2);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        assert_eq!(app.document.revision(), revision + 1);
        let document = app.document.document();
        let output = document.features().last().ok_or("feature")?.1.outputs()[0];
        let geometry = document.entity_geometry(output).ok_or("surface geometry")?;
        assert_eq!(geometry.kind, newell_model::EntityKind::Surface);
        assert_eq!(geometry.face_count, 4);
        assert!((geometry.measure - 12.).abs() < 1e-6);
        let scene = app
            .geometry
            .as_ref()
            .and_then(|runtime| runtime.committed_scene.as_ref())
            .ok_or("scene")?;
        let item = scene
            .items
            .iter()
            .find(|item| item.entity == output)
            .ok_or("surface item")?;
        assert!(
            item.mesh
                .normals()
                .iter()
                .all(|normal| normal[2].abs() < 1e-6)
        );
        assert!(app.selection.entities().contains(&output));
        assert!(app.scene_available);
        Ok(())
    }
    #[test]
    fn real_up_to_face_and_formula_clearance_commit_exact_geometry() -> TestResult {
        let (mut app, wake) = runtime_app()?;
        open(&mut app);
        profile(&mut app);
        depth(&mut app, "5", false);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        let document = app.document.document();
        let output = document.features().last().ok_or("extrude")?.1.outputs()[0];
        let target = document
            .entity_geometry(output)
            .ok_or("geometry")?
            .faces
            .iter()
            .find(|face| {
                (face.frame.origin[2] + 5.).abs() < 1e-7 && face.frame.normal[2].abs() > 0.99
            })
            .ok_or("termination face")?
            .face;
        let revision = app.document.revision();
        open(&mut app);
        profile(&mut app);
        activate(&mut app, 3, 1);
        activate(&mut app, 6, 0);
        let update = app.shell.as_mut().ok_or("shell")?.scroll(i32::MIN);
        app.finish_shell_input(update);
        super::tests::click(
            &mut app,
            &format!("Extrude / Face {}", u64::from(target) + 1),
        );
        super::tests::click(&mut app, "OK");
        activate(&mut app, 7, 0);
        activate(&mut app, 29, 1);
        for character in "#Clearance := 0.25mm".chars() {
            super::tests::key(&mut app, ShellKey::Char(character));
        }
        super::tests::key(&mut app, ShellKey::Enter);
        pump(&mut app, &wake)?;
        assert_eq!(app.document.revision(), revision);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        assert_eq!(app.document.revision(), revision + 1);
        let document = app.document.document();
        let feature = document.features().last().ok_or("new feature")?;
        let newell_model::Feature::Extrude(extrude) = &feature.1 else {
            return Err("wrong feature".into());
        };
        assert_eq!(
            extrude.spec.inputs().termination,
            newell_model::ExtrudeTermination::UpTo
        );
        assert!(extrude.spec.inputs().targets.main.is_some());
        assert!(matches!(
            extrude.spec.inputs().main_clearance,
            Some(newell_model::ParameterValue::Bound(_))
        ));
        let geometry = document
            .entity_geometry(extrude.outputs[0])
            .ok_or("result geometry")?;
        assert!((geometry.measure - 4.75).abs() < 1e-6);
        assert_eq!(geometry.face_count, 6);
        assert!(app.scene_available);
        Ok(())
    }
    #[test]
    fn cancel_after_real_preview_preserves_unrelated_accepted_metadata_and_formula_source()
    -> TestResult {
        let (mut app, wake) = runtime_app()?;
        open(&mut app);
        profile(&mut app);
        depth(&mut app, "#Depth := 3.25mm", true);
        pump(&mut app, &wake)?;
        activate(&mut app, 4, 1);
        assert_eq!(
            app.shell.as_ref().ok_or("shell")?.shell().formula().draft(),
            "#Depth := 3.25mm"
        );
        super::tests::key(&mut app, ShellKey::Escape);
        let base = app.document.document().features()[0].1.outputs()[0];
        app.document
            .rename_entity(base, "Unrelated accepted rename".into())?;
        let update = app.poll_document_projection(Instant::now(), ProjectionTrigger::Notification);
        app.apply(update);
        let accepted = app.document.document().clone();
        let version = app.document.version();
        let events = app.document.subscribe();
        activate(&mut app, 28, 1);
        pump(&mut app, &wake)?;
        assert_eq!(app.document.document(), &accepted);
        assert_eq!(app.document.version(), version);
        assert_eq!(events.try_iter().count(), 0);
        assert_eq!(app.scene_bindings.len(), 1);
        assert!(app.scene_bindings.iter().all(|binding| matches!(
            binding.binding,
            crate::geometry_worker::SceneItemBinding::Committed { .. }
        )));
        Ok(())
    }
    #[test]
    fn real_mode_termination_side_and_scope_matrix_measures_all_outputs() -> TestResult {
        let mut cases = 0;
        for surface in [false, true] {
            for result in 0..3 {
                for termination in 0..3 {
                    for side in 0..4 {
                        let scopes: &[bool] = if result == 0 { &[true] } else { &[true, false] };
                        for all in scopes {
                            let (mut app, wake) = runtime_app()?;
                            // Both fixtures are genuine committed Extrudes from a
                            // picker-selected seed face, not injected manifests.
                            open(&mut app);
                            if surface {
                                activate(&mut app, 0, 1);
                            }
                            profile(&mut app);
                            depth(&mut app, "2", false);
                            activate(&mut app, 28, 0);
                            pump(&mut app, &wake)?;
                            let target = app
                                .document
                                .document()
                                .features()
                                .last()
                                .ok_or("merge feature")?
                                .1
                                .outputs()[0];
                            app.document.rename_entity(target, "Merge Target".into())?;
                            let update = app.poll_document_projection(
                                Instant::now(),
                                ProjectionTrigger::Notification,
                            );
                            app.apply(update);
                            open(&mut app);
                            profile(&mut app);
                            depth(&mut app, "5", false);
                            activate(&mut app, 28, 0);
                            pump(&mut app, &wake)?;
                            let limit = app
                                .document
                                .document()
                                .features()
                                .last()
                                .ok_or("limit feature")?
                                .1
                                .outputs()[0];
                            app.document.rename_entity(limit, "Limit Body".into())?;
                            let update = app.poll_document_projection(
                                Instant::now(),
                                ProjectionTrigger::Notification,
                            );
                            app.apply(update);
                            let limit_face = app
                                .document
                                .document()
                                .entity_geometry(limit)
                                .ok_or("limit")?
                                .faces
                                .iter()
                                .find(|f| {
                                    (f.frame.origin[2] + 5.).abs() < 1e-7
                                        && f.frame.normal[2].abs() > 0.99
                                })
                                .ok_or("limit cap")?
                                .face;
                            open(&mut app);
                            if surface {
                                activate(&mut app, 0, 1);
                            }
                            profile(&mut app);
                            depth(&mut app, "6", false);
                            if result != 0 {
                                activate(&mut app, 1, result);
                            }
                            for _ in 0..termination {
                                activate(&mut app, 3, 1);
                            }
                            if termination == 1 {
                                activate(&mut app, 6, 0);
                                let update = app.shell.as_mut().ok_or("shell")?.scroll(i32::MIN);
                                app.finish_shell_input(update);
                                super::tests::click(
                                    &mut app,
                                    &format!("Limit Body / Face {}", u64::from(limit_face) + 1),
                                );
                                super::tests::click(&mut app, "OK");
                            }
                            match side {
                                1 => activate(&mut app, 17, 0),
                                2 | 3 => {
                                    activate(&mut app, 18, 0);
                                    activate(&mut app, 20, 0);
                                    super::tests::key(&mut app, ShellKey::Edit(EditKey::SelectAll));
                                    for c in (if side == 2 { "3" } else { "-3" }).chars() {
                                        super::tests::key(&mut app, ShellKey::Char(c));
                                    }
                                    super::tests::key(&mut app, ShellKey::Enter);
                                }
                                _ => {}
                            }
                            if !all {
                                activate(&mut app, 26, 0);
                                activate(&mut app, 27, 0);
                                let picker = app
                                    .shell
                                    .as_ref()
                                    .ok_or("shell")?
                                    .shell()
                                    .selection()
                                    .request()
                                    .cloned()
                                    .ok_or("scope picker")?;
                                for selected in &picker.applied {
                                    if selected.get() != target.get() {
                                        let row = picker
                                            .rows
                                            .iter()
                                            .find(|row| row.entity == *selected)
                                            .ok_or("applied scope")?;
                                        super::tests::click(&mut app, &format!("■ {}", row.label));
                                    }
                                }
                                if !picker.applied.iter().any(|id| id.get() == target.get()) {
                                    super::tests::click(&mut app, "□ Merge Target");
                                }
                                super::tests::click(&mut app, "OK");
                                let field = newell_ui::SelectionFieldKey {
                                    operation: ActionOperation::Extrude,
                                    key: 27,
                                };
                                assert_eq!(
                                    app.command_inputs.as_ref().ok_or("inputs")?.values[&field]
                                        .entities(),
                                    &[target]
                                );
                            }
                            pump(&mut app, &wake)?;
                            let before = app.document.version();
                            let events = app.document.subscribe();
                            activate(&mut app, 28, 0);
                            pump(&mut app, &wake)?;
                            if app.document.revision() == before.revision {
                                let shell = app.shell.as_ref().ok_or("shell")?;
                                let status: String = shell
                                    .grid()
                                    .cells()
                                    .iter()
                                    .rev()
                                    .take(shell.grid().cols() as usize)
                                    .rev()
                                    .map(|c| match c.content {
                                        newell_ui::CellContent::Glyph(ch) => ch,
                                        _ => ' ',
                                    })
                                    .collect();
                                eprintln!(
                                    "matrix failed recipe={:?}; grid={status}",
                                    app.geometry.as_ref().map(|r| &r.latest_edit)
                                );
                            }
                            assert_eq!(
                                app.document.revision(),
                                before.revision + 1,
                                "surface={surface}, result={result}, termination={termination}, side={side}, all={all}"
                            );
                            assert_eq!(events.try_iter().count(), 1);
                            let document = app.document.document();
                            let newell_model::Feature::Extrude(extrude) =
                                &document.features().last().ok_or("feature")?.1
                            else {
                                return Err("wrong feature".into());
                            };
                            if result == 2 {
                                assert!(extrude.outputs.is_empty());
                                assert!(extrude.consumed_inputs().contains(&target));
                            } else {
                                assert_eq!(extrude.outputs.len(), 1);
                                let main = if termination == 0 {
                                    6.
                                } else if termination == 1 {
                                    5.
                                } else {
                                    5.000001
                                };
                                let mut reach = match side {
                                    1 => 2. * main,
                                    2 => main + 3.,
                                    _ => main,
                                };
                                if result == 1 && *all && !surface && (side == 0 || side == 3) {
                                    reach += 1.;
                                }
                                let shape = document
                                    .entity_geometry(extrude.outputs[0])
                                    .ok_or("output")?;
                                let expected = reach * if surface { 4. } else { 1. };
                                assert!(
                                    (shape.measure - expected).abs() < 1e-5,
                                    "surface={surface}, result={result}, termination={termination}, side={side}, all={all}: actual={}, expected={expected}",
                                    shape.measure
                                );
                                assert_eq!(
                                    shape.kind,
                                    if surface {
                                        newell_model::EntityKind::Surface
                                    } else {
                                        newell_model::EntityKind::Body
                                    }
                                );
                            }
                            assert_eq!(
                                app.scene_bindings.len(),
                                document.active_entities_at(None)?.len()
                            );
                            assert_eq!(
                                app.shell
                                    .as_ref()
                                    .ok_or("shell")?
                                    .shell()
                                    .tree_projection()
                                    .elements
                                    .len(),
                                app.scene_bindings.len()
                            );
                            assert!(app.scene_available);
                            cases += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(cases, 120);
        Ok(())
    }
    #[test]
    fn missing_profile_cannot_create_a_feature_or_an_arbitrary_box() -> TestResult {
        let (mut app, wake) = runtime_app()?;
        let before = app.document.document().clone();
        let token = app.document.geometry_token();
        open(&mut app);
        depth(&mut app, "7.0", false);
        activate(&mut app, 28, 0);
        pump(&mut app, &wake)?;
        assert_eq!(app.document.document(), &before);
        assert_eq!(app.document.geometry_token(), token);
        assert_eq!(app.scene_bindings.len(), 1);
        assert!(app.shell.as_ref().unwrap().shell().command_open());
        Ok(())
    }
}
