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
use newell_kernel::{NewellMesh, NewellSolid};
use newell_model::{CommandController, CommandState, EntityId, Selection};
use newell_render::{FrameOutcome, Renderer, SceneDisplayMode, SceneViewport, Theme};
use newell_ui::{
    ActionOperation, DEFAULT_SCALE_FACTOR_PERCENT, ExtrudeStatus, FeatureNodeId,
    GapDisplayLocation, GridMetrics, HistoryGap, OpaqueTreeId, ResizeSnapshot, ResizeZone,
    ScaleMode, ThemeChoice, TreeDocumentId, WindowAction, composed_grid_size, effective_scale,
    fit_window, k_fit, min_window_size,
};
use newell_ui::{
    CommandInputSessionId, EntitySelectionView, LocalSelectionView, SelectionDestination,
    SelectionEdit, SelectionFieldCount, SelectionFieldDescriptor, SelectionOpen, SelectionOutcome,
    SelectionRow, SelectionSessionId, SelectionUiRequest,
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
    extrude_latch: ExtrudeCommitLatch,
    committed_cursor: Option<(TreeDocumentId, OpaqueTreeId)>,
    layout_retry_rearm_pending: bool,
    #[cfg(test)]
    last_ui_publication: Option<newell_ui::CellGrid>,
    selection: WindowSelection,
    command_inputs: Option<CommandSelectionInputs>,
    numeric_inputs: Option<crate::parameters::CommandNumericInputs>,
    numeric_cache:
        std::collections::BTreeMap<ActionOperation, crate::parameters::CommandNumericInputs>,
    numeric_delivery: Option<NumericDelivery>,
    numeric_last_attempt: u64,
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
    pending_scene_click: Option<PendingSceneClick>,
    scene_selected: bool,
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
                        let count = self
                            .command_inputs
                            .as_ref()
                            .and_then(|inputs| inputs.values.get(&d.field))
                            .map_or(0, |v| v.entities().len());
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
        if self.selection.apply(request, &self.document) {
            self.publish_selection()
        } else {
            Update::default()
        }
    }
    fn update_scene_selection(&mut self) {
        let selected = self.scene_entity.is_some_and(|binding| {
            binding.document == self.document.identity()
                && binding.mesh_revision == self.navigation.scene().revision()
                && self.document.document().entity(binding.entity).is_some()
                && self.selection.entities().contains(&binding.entity)
        });
        if let Some(renderer) = &mut self.renderer {
            renderer.set_scene_selected(selected);
        }
        if self.scene_selected != selected {
            self.scene_selected = selected;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
    fn install_scene_mesh(&mut self, mesh: NewellMesh, entity: Option<EntityId>) {
        self.pending_scene_click = None;
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
        if pending.document != self.document.identity()
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
            let Some(binding) = self.scene_entity.filter(|b| {
                b.document == pending.document
                    && b.mesh_revision == hit.mesh_revision
                    && self.document.document().entity(b.entity).is_some()
            }) else {
                return Update::default();
            };
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
    fn start_command_inputs(&mut self, operation: ActionOperation) -> Update {
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
        });
        self.numeric_inputs = Some(numeric_inputs);
        self.numeric_last_attempt = 0;
        self.numeric_committed = false;
        self.numeric_preview_suspended = false;
        self.numeric_delivery = None;
        self.active_selection = None;
        self.extrude_latch = ExtrudeCommitLatch {
            armed: operation == ActionOperation::Extrude,
            insertion: Some((self.document.identity().tree(), insertion)),
        };
        self.commands.cancel();
        let selection_update = self.publish_selection();
        selection_update.merge(self.publish_numeric_view())
    }
    fn panel_rows(&self, request: &SelectionOpen) -> Option<Vec<SelectionRow>> {
        if request.document != self.document.identity().tree() {
            return None;
        }
        match request.destination {
            SelectionDestination::Window => Some(candidate_rows(&self.document, None, None)),
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
        let Some(inputs) = self.command_inputs.as_ref() else {
            return Update::default();
        };
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
        request
            .applied
            .retain(|id| request.rows.iter().any(|r| r.entity == *id));
        match self
            .shell
            .as_mut()
            .map(|s| s.open_selection(request.clone()))
        {
            Some(Ok(update)) => {
                self.next_selection_id = next;
                self.active_selection = Some(request);
                self.navigation.cancel();
                self.pending_scene_click = None;
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
            extrude_latch: ExtrudeCommitLatch::default(),
            committed_cursor: None,
            layout_retry_rearm_pending: false,
            #[cfg(test)]
            last_ui_publication: None,
            selection: WindowSelection::new(document_identity),
            command_inputs: None,
            numeric_inputs: None,
            numeric_cache: std::collections::BTreeMap::new(),
            numeric_delivery: None,
            numeric_last_attempt: 0,
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
            pending_scene_click: None,
            scene_selected: false,
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
                if let NumericDeliveryKind::Confirm(request, true, feature, _) = pending.kind
                    && (request.operation != ActionOperation::Extrude || feature.is_some())
                {
                    self.numeric_cache.remove(&request.operation);
                    self.numeric_inputs = None;
                    self.command_inputs = None;
                    self.active_selection = None;
                    self.numeric_committed = false;
                    update = update.merge(self.publish_selection());
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
                        inputs.apply(request, descriptor)
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
        let result = inputs
            .parameter_draft
            .validate_source(self.document.document())
            .map_err(AppError::from)
            .and_then(|()| inputs.apply_formula(&request));
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
    fn complete_numeric_confirm(
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
                self.extrude_latch = ExtrudeCommitLatch {
                    armed: true,
                    insertion: Some((request.document, selections.insertion)),
                };
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
        self.extrude_latch = ExtrudeCommitLatch::default();
        self.numeric_preview_suspended = !accepted;
        self.queue_numeric_delivery(NumericDeliveryKind::Confirm(
            request, accepted, feature, projection,
        ))
    }
    fn sync_extrude_command(&mut self) {
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
            self.extrude_latch = ExtrudeCommitLatch::default();
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
                        .depth
                        .evaluate(self.document.document().parameters())
                {
                    self.set_preview_depth(depth.as_millimetres());
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
        self.install_scene_mesh(mesh, None);
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
        let mut invalidation_update = Update::default();
        if self.selection.document != self.document.identity() {
            self.extrude_latch = ExtrudeCommitLatch::default();
            self.committed_cursor = None;
            self.commands.cancel();
            self.command_inputs = None;
            self.numeric_inputs = None;
            self.numeric_cache.clear();
            self.numeric_delivery = None;
            self.numeric_committed = false;
            self.active_selection = None;
            self.scene_entity = None;
            self.pending_scene_click = None;
            self.navigation.cancel();
            if let Some(shell) = &mut self.shell {
                invalidation_update = shell.invalidate_document_interactions();
            }
        }
        self.selection.reconcile(&self.document);
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
            && self
                .shell
                .as_ref()
                .is_some_and(|s| s.shell().entity_selection() == &self.selection_view().selection)
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
        update = update.merge(self.deliver_numeric(
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
            self.numeric_delivery.as_ref().and_then(|d| d.retry_at),
        ]
        .into_iter()
        .flatten()
        .min()
    }
    fn finish_shell_input(&mut self, mut update: Update) {
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
        if let Some(intent) = intent
            && self
                .shell
                .as_ref()
                .is_some_and(|s| s.generation() == intent.generation())
            && intent.target().document == self.document.identity().tree()
        {
            if let newell_ui::TreeIntentKind::Select { edit } = intent.kind() {
                if let Some(entities) = resolve_tree_selection(&self.document, intent.target()) {
                    update = update.merge(self.handle_selection_request(SelectionRequest {
                        document: self.document.identity(),
                        op: (*edit).into(),
                        entities,
                    }));
                }
            } else if let Err(error) = self
                .document
                .apply_tree_edit(intent.target(), intent.kind())
            {
                log(&error.to_string());
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
                if scene_input
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
        }
        self.navigation
            .set_layout(scene_viewport(&shell), scene_occlusions(&shell));
        let base = match self.document.add_base_solid("Base Solid") {
            Ok(base) => base,
            Err(error) => return self.fail(event_loop, error),
        };
        let entity = self
            .document
            .document()
            .feature(base)
            .and_then(|f| f.outputs().first().copied());
        let solid = match NewellSolid::box_solid(1.0, 1.0, 1.0) {
            Ok(solid) => solid,
            Err(error) => return self.fail(event_loop, error),
        };
        let mesh = match solid.tessellate(0.05) {
            Ok(mesh) => mesh,
            Err(error) => return self.fail(event_loop, error),
        };
        self.install_scene_mesh(mesh, entity);
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
                let point = self
                    .scene_pointer
                    .map_or((f64::NAN, f64::NAN), |p| (p.x, p.y));
                let certificate = self.navigation.finish_primary(point);
                let selection_update = self.finish_scene_selection(point, certificate);
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
                    if matches!(delta, MouseScrollDelta::PixelDelta(_)) {
                        self.navigation.reject_trackpad_pan_scroll(phase);
                    }
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
                    return;
                }
                let eligible = !self.resize.is_active()
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
                        return;
                    }
                    // macOS winit emits phased PixelDelta events for trackpad scrolling. A
                    // Started stream over the scene uses the same frozen-depth pan as mouse
                    // drags; unphased PixelDelta and LineDelta continue through wheel zoom.
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
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                if let Some(shell) = &mut self.shell {
                    shell.set_selection_modifiers(
                        self.modifiers.shift_key(),
                        self.modifiers.control_key()
                            || (cfg!(target_os = "macos") && self.modifiers.super_key()),
                    );
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                self.navigation.cancel();
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
                    && d.role == SelectionFieldRole::Direction
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
        assert!(
            app.command_inputs.as_ref().unwrap().values[&d.field]
                .entities()
                .is_empty()
        );
        click(&mut app, "OK");
        assert_eq!(app.command_inputs.as_ref().unwrap().session, session);
        assert_eq!(
            app.command_inputs.as_ref().unwrap().values[&d.field].entities(),
            &[ids[2]]
        );
        assert_eq!(
            app.shell
                .as_ref()
                .unwrap()
                .shell()
                .selection_field_count(d.field),
            1
        );
        assert_eq!(app.selection.entities(), &[ids[1]]);
        let update = app.open_field_selection(d);
        app.apply(update);
        key(&mut app, ShellKey::Char(' '));
        key(&mut app, ShellKey::Escape);
        assert_eq!(
            app.command_inputs.as_ref().unwrap().values[&d.field].entities(),
            &[ids[2]]
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
    fn formula(a: &mut App, field: u32, source: &str) {
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
    fn committed(a: &App) -> &newell_model::ExtrudeSpec {
        a.document
            .document()
            .features()
            .last()
            .and_then(|(_, feature)| match feature {
                Feature::Extrude(f) => Some(&f.spec),
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
        let mut a = app();
        edit(&mut a, 4, "12.3", false, true);
        activate(&mut a, 9, 0);
        edit(&mut a, 11, "4.5", false, false);
        activate(&mut a, 9, 0);
        assert_eq!(value(&a, 11), 4.5);
        assert!(a.commands.preview().unwrap().starting_offset.is_none());
        activate(&mut a, 9, 0);
        assert_eq!(
            mm(a.commands
                .preview()
                .unwrap()
                .starting_offset
                .as_ref()
                .unwrap()),
            4.5
        );
        activate(&mut a, 10, 0);
        assert!(a.commands.preview().unwrap().starting_offset.is_none());
        activate(&mut a, 10, 0);
        activate(&mut a, 18, 0);
        edit(&mut a, 20, "8.9", true, true);
        activate(&mut a, 17, 0);
        assert!(a.commands.preview().unwrap().second_end.is_none());
        activate(&mut a, 17, 0);
        assert_eq!(value(&a, 20), 8.9);
        activate(&mut a, 19, 0);
        activate(&mut a, 22, 0);
        edit(&mut a, 24, "2.4", false, true);
        activate(&mut a, 28, 0);
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
        let mut a = app();
        formula(&mut a, 4, "2mm + 3mm");
        assert_eq!(value(&a, 4), 5.);
        activate(&mut a, 28, 0);
        assert!(matches!(
            committed(&a).depth,
            newell_model::ParameterValue::Expression(_)
        ));
        assert_eq!(mm(&committed(&a).depth), 5.);
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
