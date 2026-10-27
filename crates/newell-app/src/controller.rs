// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Per-window shell controller: owns the [`ShellState`] and the composed grid, and turns
//! pointer and key input into "what must be re-uploaded" ([`Update`]).
//!
//! It contains no `winit` or GPU types, so it is testable without a window or an adapter.

use std::time::Duration;

use newell_ui::{
    CellGrid, GridMetrics, GridRect, ModelTreeProjection, ProjectionGeneration, ResizeZone,
    SelectionThumbGrab, Separator, ShellKey, ShellState, TreeDocumentId, TreeHitToken, TreeIntent,
    UiError, UiPoint, WidgetId, WindowAction, composed_grid_size, hit_test, resize_zone,
};

/// Two presses on the same tree-row name at most this far apart are a double-click
/// (`README_UI.md` §23). `winit` 0.30 reports no click count, so the controller measures it.
pub const DOUBLE_CLICK: Duration = Duration::from_millis(500);

/// Who owns a primary-button press (`README_UI.md` §25.1 precedence).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// A window control `[-] [□] [X]`.
    Window(WindowAction),
    /// A resize zone of the outer frame ring.
    Resize(ResizeZone),
    /// Unused MenuBar space: drag the window.
    Drag,
    /// A shell control or nothing: normal routing already applied.
    Shell(Update),
}

/// A wheel event can belong to a panel even when its scroll limit prevents movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PanelScroll {
    pub handled: bool,
    pub update: Update,
}

/// OS cursor implied by the exact pointer target and press precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorHint {
    Default,
    Resize(ResizeZone),
    HorizontalResize,
    VerticalResize,
    Move,
    Grab,
    Grabbing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CapturedDrag {
    Separator {
        kind: Separator,
        id: u64,
        origin: ProjectionGeneration,
        bound: ProjectionGeneration,
        document: TreeDocumentId,
    },
    SelectionThumb {
        grab: SelectionThumbGrab,
    },
}

#[derive(Debug, Clone, Copy)]
enum FrameCause {
    External,
    /// Hover was resolved against the current published semantic map.
    PointerHover,
    OwnedDrag(u64),
    OwnedSelectionDrag(SelectionThumbGrab),
}

/// A lookup through an existing published slot; the generation is metadata only here.
fn semantic_tree_target(shell: &ShellState, id: WidgetId) -> Option<newell_ui::TreeTarget> {
    shell
        .tree_hit(id, ProjectionGeneration::new(0))
        .map(|hit| hit.target)
}

fn allocate_interaction_identity(next: &mut u64) -> Result<u64, UiError> {
    let following = next
        .checked_add(1)
        .ok_or(UiError::InteractionIdentityExhausted)?;
    let identity = *next;
    *next = following;
    Ok(identity)
}

#[derive(Debug, Clone)]
struct RenameSession {
    id: u64,
    origin: TreeHitToken,
    bound: ProjectionGeneration,
    source_name: String,
}

/// What the caller must do after an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Update {
    /// The grid was recomposed: upload it.
    pub grid: bool,
}

impl Update {
    /// `true` if anything must be uploaded or redrawn.
    #[must_use]
    pub const fn any(self) -> bool {
        self.grid
    }

    /// Both effects of two updates.
    #[must_use]
    pub const fn merge(self, other: Self) -> Self {
        Self {
            grid: self.grid || other.grid,
        }
    }
}

/// A scale change prepared by [`ShellController::prepare_metrics`], not yet published.
#[derive(Debug)]
pub struct PreparedScale {
    metrics: GridMetrics,
    shell: ShellState,
    grid: CellGrid,
}

impl PreparedScale {
    /// The composed grid, to be uploaded with the metrics' atlas.
    #[must_use]
    pub const fn grid(&self) -> &CellGrid {
        &self.grid
    }

    /// The metrics the grid was composed for.
    #[must_use]
    pub const fn metrics(&self) -> GridMetrics {
        self.metrics
    }
}

/// The shell state of one window plus its last composed grid and pointer position.
#[derive(Debug)]
pub struct ShellController {
    shell: ShellState,
    published_shell: ShellState,
    grid: CellGrid,
    metrics: GridMetrics,
    generation: ProjectionGeneration,
    next_drag_id: u64,
    next_rename_id: u64,
    rename_session: Option<RenameSession>,
    intent: Option<TreeIntent>,
    selection_request: Option<newell_ui::SelectionUiRequest>,
    selection_session: Option<(newell_ui::SelectionOpen, ProjectionGeneration)>,
    cursor: Option<UiPoint>,
    /// The separator being dragged (pointer capture): temporary interaction state.
    drag: Option<CapturedDrag>,
    /// Monotonic time of the input being handled, set by the application ([`Self::set_time`]).
    clock: Duration,
    /// The last press on a renamable tree-row name and when it happened (double-click state).
    last_press: Option<(TreeHitToken, Duration)>,
    /// The ASCII hover target (M2.25): the widget under the pointer when it is eligible, else
    /// `None`. Transient; the grid is composed with it and nothing else stores it.
    hovered: Option<WidgetId>,
    status: crate::status::StatusClock,
    numeric_origin: Option<(
        newell_ui::TreeDocumentId,
        newell_ui::CommandInputSessionId,
        newell_ui::ActionOperation,
        ProjectionGeneration,
        std::collections::BTreeSet<newell_ui::NumericFieldKey>,
    )>,
    #[cfg(test)]
    pub(crate) fail_numeric_confirm: u8,
    #[cfg(test)]
    pub(crate) fail_numeric_formula: u8,
}

impl ShellController {
    /// A controller for `cols` x `rows` cells of `metrics` pixels.
    ///
    /// # Errors
    /// If the grid size is rejected by [`CellGrid::new`].
    pub fn new(cols: u32, rows: u32, metrics: GridMetrics) -> Result<Self, UiError> {
        Self::with_shell(ShellState::new_application(cols, rows), metrics)
    }

    pub fn with_shell(shell: ShellState, metrics: GridMetrics) -> Result<Self, UiError> {
        let grid = shell.compose()?;
        Ok(Self {
            published_shell: shell.clone(),
            shell,
            generation: ProjectionGeneration::new(0),
            next_drag_id: 0,
            next_rename_id: 0,
            rename_session: None,
            intent: None,
            selection_request: None,
            selection_session: None,
            grid,
            metrics,
            cursor: None,
            drag: None,
            clock: Duration::ZERO,
            last_press: None,
            hovered: None,
            status: crate::status::StatusClock::default(),
            numeric_origin: None,
            #[cfg(test)]
            fail_numeric_confirm: 0,
            #[cfg(test)]
            fail_numeric_formula: 0,
        })
    }

    /// Tells the controller the monotonic time of the input events that follow (time since any
    /// fixed start). Double-click and progress timing share this injectable clock.
    pub fn set_time(&mut self, now: Duration) {
        self.clock = self.clock.max(now);
    }

    /// Counts one actually presented frame, never skipped renders or wakeups.
    pub fn presented(&mut self) {
        self.status.presented();
    }

    /// Advance runtime status independently of input, using a monotonic injected clock.
    pub fn tick(&mut self, now: Duration) -> Update {
        self.set_time(now);
        let due = self.status.advance(self.clock);
        self.shell
            .set_status(self.status.fps, self.status.progress.percent());
        self.refresh(due)
    }

    /// Earliest FPS/progress refresh, in time since the application's epoch.
    #[must_use]
    pub fn status_deadline(&self) -> Duration {
        self.status.deadline(self.clock)
    }

    /// The last composed grid.
    #[must_use]
    pub const fn grid(&self) -> &CellGrid {
        &self.grid
    }

    /// Cell size in physical pixels.
    #[must_use]
    pub const fn metrics(&self) -> GridMetrics {
        self.metrics
    }

    /// The shell state (read-only).
    #[must_use]
    pub const fn shell(&self) -> &ShellState {
        &self.shell
    }

    /// Consumes a View > Reset Cam request.
    pub fn take_camera_reset(&mut self) -> bool {
        self.shell.take_camera_reset()
    }

    /// Consumes a View > Normal to request.
    pub fn take_normal_view(&mut self) -> bool {
        self.shell.take_normal_view()
    }

    /// Whole-cell area where the 3D scene is drawn.
    #[must_use]
    pub fn scene_viewport(&self) -> GridRect {
        self.shell.scene_viewport()
    }

    pub fn scene_occlusions(&self) -> Vec<GridRect> {
        self.shell.scene_occlusions()
    }

    /// `true` when the pointer is on visible scene pixels and no UI owns input.
    #[must_use]
    pub fn scene_input_enabled(&self) -> bool {
        self.shell.scene_input_enabled()
            && !self.dragging()
            && self
                .cursor
                .and_then(|point| hit_test(point, self.metrics, &self.grid))
                .is_some_and(|hit| hit.widget_id.is_none())
            && self.cursor.is_some_and(|point| {
                let col = point.x / self.metrics.cell_width_px();
                let row = point.y / self.metrics.cell_height_px();
                let rect = self.scene_viewport();
                col >= rect.col()
                    && row >= rect.row()
                    && col < rect.col() + rect.width()
                    && row < rect.row() + rect.height()
                    && !self.scene_occlusions().iter().any(|rect| {
                        col >= rect.col()
                            && row >= rect.row()
                            && col < rect.col() + rect.width()
                            && row < rect.row() + rect.height()
                    })
            })
    }

    /// Classifies the OS cursor from the same O(1) hit and precedence used by `press`.
    #[must_use]
    pub fn cursor_hint(&self, surface: (u32, u32), resizable: bool) -> CursorHint {
        let Some(point) = self.cursor else {
            return CursorHint::Default;
        };
        if let Some(drag) = self.drag {
            let available = match drag {
                CapturedDrag::SelectionThumb { grab } => {
                    self.shell.selection_thumb_grab_valid(grab)
                }
                CapturedDrag::Separator { .. } => self.shell.captured_drag_available(false),
            };
            return if available {
                CursorHint::Grabbing
            } else {
                CursorHint::Default
            };
        }
        let hit = hit_test(point, self.metrics, &self.grid);
        let id = hit.and_then(|h| h.widget_id);
        if id.is_some_and(|id| ShellState::window_action(id).is_some()) {
            return CursorHint::Default;
        }
        if id.is_none()
            && resizable
            && let Some(zone) = resize_zone(point, surface, self.metrics)
        {
            return CursorHint::Resize(zone);
        }
        if hit.is_some_and(|h| self.shell.window_drag_at(h.col, h.row, &self.grid)) {
            return CursorHint::Move;
        }
        if let Some(hit) = hit {
            if self.shell.selection_thumb_grab(&hit).is_some() {
                return CursorHint::Grab;
            }
            match self.shell.drag_at(&hit) {
                Some(Separator::Navigation) => return CursorHint::HorizontalResize,
                Some(Separator::Split) => return CursorHint::VerticalResize,
                Some(Separator::Playhead) => return CursorHint::Grab,
                None => {}
            }
        }
        CursorHint::Default
    }

    pub const fn generation(&self) -> ProjectionGeneration {
        self.generation
    }
    pub fn set_selection_modifiers(&mut self, shift: bool, toggle: bool) {
        self.shell.set_selection_modifiers(shift, toggle);
    }
    fn capture_numeric_origin(&mut self) {
        if self.shell.numeric_session().is_some() {
            self.shell.set_numeric_origin(self.generation);
        }
        self.numeric_origin = self
            .shell
            .numeric_session()
            .map(|(document, session, operation)| {
                let keys = self
                    .shell
                    .numeric_descriptors()
                    .into_iter()
                    .filter(|d| self.shell.numeric_field_visible(d.field))
                    .map(|d| d.field)
                    .collect();
                (document, session, operation, self.generation, keys)
            });
    }
    /// Validates against the pre-input certificate, including fields hidden by that input.
    /// Current post-input visibility cannot replace this check for a legitimate blur edit.
    pub fn numeric_origin_allows(&self, request: &newell_ui::NumericRequest) -> bool {
        self.numeric_origin
            .as_ref()
            .is_some_and(|(document, session, operation, origin, keys)| {
                request.destination.document == *document
                    && request.destination.session == *session
                    && request.destination.field.operation == *operation
                    && request.origin == *origin
                    && keys.contains(&request.destination.field)
            })
    }
    /// Drains both shell mirrors so a publication failure cannot replay an emitted candidate.
    pub fn take_numeric_requests(&mut self) -> Vec<newell_ui::NumericRequest> {
        let requests = self.shell.take_numeric_requests();
        self.published_shell.take_numeric_requests();
        requests
    }
    /// Drains checked numeric interaction-identity errors from both shell mirrors.
    pub fn take_numeric_error(&mut self) -> Option<UiError> {
        let error = self.shell.take_numeric_error();
        self.published_shell.take_numeric_error();
        error
    }
    /// Drains outbound OK from both mirrors, retaining the UI's exact acknowledgement record.
    pub fn take_numeric_confirm(&mut self) -> Option<newell_ui::NumericConfirmRequest> {
        let request = self.shell.take_numeric_confirm();
        self.published_shell.take_numeric_confirm();
        request
    }
    /// Drains outbound Formula opening from both mirrors, retaining its delivery obligation.
    pub fn take_numeric_formula_open(&mut self) -> Option<newell_ui::NumericFormulaOpenRequest> {
        let request = self.shell.take_numeric_formula_open();
        self.published_shell.take_numeric_formula_open();
        request
    }
    /// Drains an explicit Formula OK from both publication mirrors exactly once.
    pub fn take_formula_apply(&mut self) -> Option<newell_ui::FormulaApplyRequest> {
        let request = self.shell.take_formula_apply();
        self.published_shell.take_formula_apply();
        request
    }
    /// Publishes the already resolved Formula outcome without parsing it again.
    pub fn finish_formula_apply(
        &mut self,
        request: &newell_ui::FormulaApplyRequest,
        projection: newell_ui::NumericProjection,
        diagnostic: Option<String>,
    ) -> Result<Update, UiError> {
        let mut candidate = self.shell.clone();
        if !candidate.finish_formula_apply(request, projection, diagnostic)? {
            return Ok(Update::default());
        }
        let grid = candidate.compose()?;
        self.publish(candidate, grid, FrameCause::External)?;
        Ok(Update { grid: true })
    }
    /// Validates and publishes accepted numeric views through the candidate-frame boundary.
    ///
    /// # Errors
    /// Propagates projection validation, composition, and generation-capacity errors.
    pub fn set_numeric_projection(
        &mut self,
        value: newell_ui::NumericProjection,
    ) -> Result<Update, UiError> {
        let mut candidate = self.shell.clone();
        if !candidate.set_numeric_projection(value)? {
            return Ok(Update::default());
        }
        let grid = candidate.compose()?;
        self.publish(candidate, grid, FrameCause::External)?;
        Ok(Update { grid: true })
    }
    /// Publishes the rejection of a matching outstanding edit without addressing a newer one.
    ///
    /// # Errors
    /// Propagates UI composition or publication errors; stale rejections are ignored.
    pub fn reject_numeric_edit(
        &mut self,
        destination: newell_ui::NumericDestination,
        sequence: u64,
    ) -> Result<Update, UiError> {
        let mut candidate = self.shell.clone();
        if !candidate.reject_numeric_edit(destination, sequence)? {
            return Ok(Update::default());
        }
        let grid = candidate.compose()?;
        self.publish(candidate, grid, FrameCause::External)?;
        Ok(Update { grid: true })
    }
    /// Publishes the stored terminal outcome and accepted views in one candidate frame.
    /// The application has already resolved the model transaction; this only delivers it.
    pub(crate) fn finish_numeric_confirm_with_projection(
        &mut self,
        request: newell_ui::NumericConfirmRequest,
        accepted: bool,
        projection: newell_ui::NumericProjection,
    ) -> Result<Update, UiError> {
        #[cfg(test)]
        if self.fail_numeric_confirm > 0 {
            self.fail_numeric_confirm -= 1;
            return Err(UiError::InvalidNumericProjection);
        }
        let mut candidate = self.shell.clone();
        candidate.set_numeric_projection(projection)?;
        if !candidate.finish_numeric_confirm(request, accepted)? {
            return Ok(Update::default());
        }
        let grid = candidate.compose()?;
        self.publish(candidate, grid, FrameCause::External)?;
        Ok(Update { grid: true })
    }
    /// Publishes one exact resolved OK outcome without invoking model confirmation.
    ///
    /// # Errors
    /// Propagates composition/publication errors, leaving the pending UI attempt available.
    pub fn finish_numeric_confirm(
        &mut self,
        request: newell_ui::NumericConfirmRequest,
        accepted: bool,
    ) -> Result<Update, UiError> {
        #[cfg(test)]
        if self.fail_numeric_confirm > 0 {
            self.fail_numeric_confirm -= 1;
            return Err(UiError::InvalidNumericProjection);
        }
        let mut candidate = self.shell.clone();
        if !candidate.finish_numeric_confirm(request, accepted)? {
            return Ok(Update::default());
        }
        let grid = candidate.compose()?;
        self.publish(candidate, grid, FrameCause::External)?;
        Ok(Update { grid: true })
    }
    /// Publishes a deferred Formula opening and its accepted source together, exactly once.
    ///
    /// # Errors
    /// Propagates projection, composition, and publication errors without consuming the opening.
    pub fn open_numeric_formula(
        &mut self,
        request: newell_ui::NumericFormulaOpenRequest,
        value: newell_ui::NumericProjection,
    ) -> Result<Update, UiError> {
        #[cfg(test)]
        if self.fail_numeric_formula > 0 {
            self.fail_numeric_formula -= 1;
            return Err(UiError::InvalidNumericProjection);
        }
        let mut candidate = self.shell.clone();
        if !candidate.open_numeric_formula(request, value)? {
            return Ok(Update::default());
        }
        let grid = candidate.compose()?;
        self.publish(candidate, grid, FrameCause::External)?;
        Ok(Update { grid: true })
    }
    /// Captures provenance and resolves a raw field on focus loss, never confirming a command.
    pub fn numeric_focus_lost(&mut self) -> Update {
        self.capture_numeric_origin();
        let acted = self.shell.numeric_focus_lost();
        self.refresh(acted)
    }
    pub fn take_command_start(&mut self) -> Option<newell_ui::ActionOperation> {
        let start = self.shell.take_command_start();
        self.published_shell.take_command_start();
        start
    }
    pub fn take_selection_request(&mut self) -> Option<newell_ui::SelectionUiRequest> {
        self.selection_request.take()
    }
    pub fn open_selection(&mut self, request: newell_ui::SelectionOpen) -> Result<Update, UiError> {
        let mut candidate = self.shell.clone();
        candidate.open_selection(request.clone())?;
        let grid = candidate.compose()?;
        self.publish(candidate, grid, FrameCause::External)?;
        self.selection_session = Some((request, self.generation));
        self.selection_request = None;
        Ok(Update { grid: true })
    }
    pub fn set_entity_selection(
        &mut self,
        selection: newell_ui::EntitySelectionView,
    ) -> Result<Update, UiError> {
        let mut candidate = self.shell.clone();
        if !candidate.set_entity_selection(selection)? {
            return Ok(Update::default());
        }
        let grid = candidate.compose()?;
        self.publish(candidate, grid, FrameCause::External)?;
        Ok(Update { grid: true })
    }
    pub fn set_selection_view(
        &mut self,
        view: newell_ui::LocalSelectionView,
    ) -> Result<Update, UiError> {
        self.set_document_view(self.shell.tree_projection().clone(), view)
    }
    pub fn set_document_view(
        &mut self,
        projection: ModelTreeProjection,
        view: newell_ui::LocalSelectionView,
    ) -> Result<Update, UiError> {
        self.set_document_view_and_candidates(projection, view, None)
    }
    pub(crate) fn set_document_view_and_candidates(
        &mut self,
        projection: ModelTreeProjection,
        view: newell_ui::LocalSelectionView,
        rows: Option<Vec<newell_ui::SelectionRow>>,
    ) -> Result<Update, UiError> {
        let changed_projection = projection != *self.shell.tree_projection();
        let mut candidate = self.shell.clone();
        if projection.document != candidate.tree_projection().document {
            candidate.invalidate_numeric_session();
        }
        let mut changed = candidate.set_tree_projection(projection)?;
        changed |= candidate.set_entity_selection(view.selection)?;
        changed |= candidate.set_selection_fields(&view.fields)?;
        if let Some(rows) = rows {
            changed |= candidate.reconcile_selection_rows(rows)?;
        }
        if !changed {
            return Ok(Update::default());
        }
        let grid = candidate.compose()?;
        self.publish(candidate, grid, FrameCause::External)?;
        if changed_projection {
            self.numeric_origin = None;
        }
        Ok(Update { grid: true })
    }
    pub fn take_tree_intent(&mut self) -> Option<TreeIntent> {
        self.intent
            .take()
            .filter(|intent| intent.generation() == self.generation)
    }
    pub fn set_tree_projection(
        &mut self,
        projection: ModelTreeProjection,
    ) -> Result<Update, UiError> {
        let playhead_captured = self.playhead_capture_active();
        let mut candidate = self.shell.clone();
        if projection.document != candidate.tree_projection().document {
            candidate.invalidate_numeric_session();
        }
        if !candidate.set_tree_projection(projection)? {
            return Ok(Update::default());
        }
        let grid = candidate.compose()?;
        self.publish(candidate, grid, FrameCause::External)?;
        self.numeric_origin = None;
        self.drag = None;
        self.last_press = None;
        self.intent = None;
        let update = if playhead_captured {
            self.resume_hover_from_published_target()
        } else {
            Update::default()
        };
        Ok(Update { grid: true }.merge(update))
    }
    pub fn set_playhead(&mut self, gap: newell_ui::PlacedHistoryGap) -> Result<Update, UiError> {
        let mut candidate = self.shell.clone();
        if !candidate.set_playhead(gap)? {
            return Ok(Update::default());
        }
        let grid = candidate.compose()?;
        self.publish(candidate, grid, FrameCause::External)?;
        Ok(Update { grid: true })
    }
    pub fn invalidate_document_interactions(&mut self) -> Update {
        self.numeric_origin = None;
        self.shell.invalidate_numeric_session();
        self.published_shell.invalidate_numeric_session();
        let playhead_captured = self.playhead_capture_active();
        self.drag = None;
        self.last_press = None;
        self.intent = None;
        self.rename_session = None;
        self.selection_session = None;
        self.selection_request = None;
        if playhead_captured {
            self.resume_hover_from_published_target()
        } else {
            Update::default()
        }
    }

    /// Executes only a hit captured from this exact published semantic map.
    pub fn activate_tree_hit(&mut self, id: WidgetId, token: TreeHitToken) -> Update {
        self.intent = None;
        if !self.shell.validate_tree_hit(id, token, self.generation) {
            return Update::default();
        }
        let acted = self
            .shell
            .activate_tree_hit(id, token, self.generation, false);
        self.refresh(acted)
    }

    /// The pointer moved to `position` (`None`: it left the window). The position is remembered
    /// for the next press and used to continue a drag. Motion changes the grid only through the
    /// ASCII hover target (M2.25): it is resolved from the cell under the pointer and recomposed
    /// only when that target changes.
    pub fn pointer_moved(&mut self, position: Option<UiPoint>) -> Update {
        self.cursor = position;
        if position.is_none() {
            self.drag = None;
        }
        let cause = match self.drag {
            Some(CapturedDrag::Separator {
                id,
                bound,
                document,
                ..
            }) if bound == self.generation && document == self.shell.tree_projection().document => {
                FrameCause::OwnedDrag(id)
            }
            Some(CapturedDrag::SelectionThumb { grab })
                if self.shell.selection_thumb_grab_valid(grab) =>
            {
                FrameCause::OwnedSelectionDrag(grab)
            }
            _ => FrameCause::External,
        };
        if let Some(CapturedDrag::SelectionThumb { grab }) = self.drag
            && !self.shell.selection_thumb_grab_valid(grab)
        {
            self.drag = None;
        }
        let acted = match (self.drag, position) {
            (
                Some(CapturedDrag::Separator {
                    kind,
                    bound,
                    document,
                    ..
                }),
                Some(p),
            ) if bound == self.generation
                && document == self.shell.tree_projection().document
                && self.shell.captured_drag_available(false) =>
            {
                self.shell.drag_separator(
                    kind,
                    p.x / self.metrics.cell_width_px(),
                    p.y / self.metrics.cell_height_px(),
                )
            }
            (Some(CapturedDrag::SelectionThumb { grab }), Some(p)) => self
                .shell
                .drag_selection_thumb(p.y / self.metrics.cell_height_px(), grab),
            _ => false,
        };
        let update = self.refresh_with_cause(acted, cause);
        update.merge(self.set_hover(self.widget_under_cursor(&self.grid)))
    }

    /// The pointer moved while a resize owns it (`README_UI.md` §25.1): the position is kept
    /// for the next press, but no hover is shown until the first move after the resize.
    pub fn pointer_moved_during_resize(&mut self, position: Option<UiPoint>) -> Update {
        self.cursor = position;
        self.clear_hover()
    }

    /// Removes the hover target, for a resize that starts or owns the pointer. Recomposes only
    /// if a target was shown.
    pub fn clear_hover(&mut self) -> Update {
        self.set_hover(None)
    }

    /// The ASCII hover target currently shown, if any (M2.25).
    #[must_use]
    pub const fn hovered(&self) -> Option<WidgetId> {
        self.hovered
    }

    /// The primary button was released: any separator drag ends.
    pub fn pointer_released(&mut self) -> Update {
        let playhead_captured = self.playhead_capture_active();
        self.drag = None;
        if playhead_captured {
            self.resume_hover_from_published_target()
        } else {
            Update::default()
        }
    }

    /// Routes a wheel or trackpad scroll to an in-place shell panel before scene navigation.
    pub fn scroll(&mut self, steps: i32) -> Update {
        self.scroll_panel(steps).update
    }
    pub fn scroll_panel(&mut self, steps: i32) -> PanelScroll {
        self.intent = None;
        if self.drag.is_some() {
            return PanelScroll {
                handled: true,
                update: Update::default(),
            };
        }
        if self.shell.selection().is_open() {
            let acted = self.shell.scroll(steps);
            return PanelScroll {
                handled: true,
                update: self.refresh(acted),
            };
        }
        let Some(point) = self.cursor else {
            return PanelScroll::default();
        };
        let handled = self.shell.scroll_tree_at(
            point.x / self.metrics.cell_width_px(),
            point.y / self.metrics.cell_height_px(),
            steps,
        );
        PanelScroll {
            handled: handled.is_some(),
            update: self.refresh(handled.unwrap_or(false)),
        }
    }

    /// `true` while a separator is being dragged.
    #[must_use]
    pub const fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// The window is now `width_px` x `height_px` physical pixels: the shell is re-laid out on
    /// the whole cells that fit, never below 82 x 30 (see [`composed_grid_size`]; a smaller
    /// surface is cropped by the renderer). A size that does not change the cell grid does
    /// nothing.
    pub fn resize(&mut self, width_px: u32, height_px: u32) -> Update {
        let (cols, rows) = composed_grid_size(self.metrics, width_px, height_px);
        let acted = self.shell.resize(cols, rows);
        if acted {
            self.drag = None;
            self.last_press = None;
        }
        self.refresh(acted)
    }

    /// Phase A of a scale change: the shell re-laid out for `metrics` on a surface of
    /// `width_px` x `height_px`, composed on a copy. Nothing live is touched, so a failure
    /// leaves this controller exactly as it was.
    ///
    /// # Errors
    /// If the grid cannot be composed.
    pub fn prepare_metrics(
        &self,
        metrics: GridMetrics,
        width_px: u32,
        height_px: u32,
    ) -> Result<PreparedScale, UiError> {
        let (cols, rows) = composed_grid_size(metrics, width_px, height_px);
        let mut shell = self.shell.clone();
        shell.resize(cols, rows);
        let grid = shell.compose()?;
        Ok(PreparedScale {
            metrics,
            shell,
            grid,
        })
    }

    /// Phase B of a scale change: publishes the prepared metrics, shell and grid together.
    /// Logical state (focus, selection, expansion, open overlays) lives in the shell and
    /// survives. The pointer position was reported in the old pixel space and may belong to
    /// another monitor, so it is forgotten (nothing is synthesised: the next `CursorMoved`
    /// sets it) and a separator drag is cancelled.
    pub fn commit_metrics(&mut self, prepared: PreparedScale) -> Update {
        let Some(generation) = self.generation.get().checked_add(1) else {
            return Update::default();
        };
        self.metrics = prepared.metrics;
        self.generation = ProjectionGeneration::new(generation);
        self.shell = prepared.shell;
        self.published_shell = self.shell.clone();
        self.grid = prepared.grid;
        self.intent = None;
        self.cursor = None;
        self.drag = None;
        self.last_press = None;
        self.hovered = None;
        if let Some(session) = &mut self.rename_session {
            session.bound = self.generation;
        }
        Update { grid: true }
    }

    /// The primary button was pressed at the last known pointer position on a window whose
    /// surface is `surface` physical pixels. Precedence: a window control, then (when
    /// `resizable`) a resize zone on a cell with no widget, then unused MenuBar space, then
    /// normal shell routing. Only the last case changes shell state.
    pub fn press(&mut self, surface: (u32, u32), resizable: bool) -> Press {
        self.drag = None;
        let Some(point) = self.cursor else {
            return Press::Shell(self.pointer_pressed());
        };
        let hit = hit_test(point, self.metrics, &self.grid);
        let id = hit.and_then(|h| h.widget_id);
        // A press that is not for the shell is another target pressed: it ends a double-click.
        if let Some(action) = id.and_then(ShellState::window_action) {
            self.last_press = None;
            return Press::Window(action);
        }
        if id.is_none()
            && resizable
            && let Some(zone) = resize_zone(point, surface, self.metrics)
        {
            self.last_press = None;
            return Press::Resize(zone);
        }
        if hit.is_some_and(|h| self.shell.window_drag_at(h.col, h.row, &self.grid)) {
            self.last_press = None;
            return Press::Drag;
        }
        Press::Shell(self.pointer_pressed())
    }

    /// The window control under the last known pointer position, if any.
    #[must_use]
    pub fn window_control_at_cursor(&self) -> Option<WindowAction> {
        let hit = hit_test(self.cursor?, self.metrics, &self.grid)?;
        hit.widget_id.and_then(ShellState::window_action)
    }

    /// `true` once after the View > Fullscreen item was activated.
    pub fn take_fullscreen_request(&mut self) -> bool {
        self.shell.take_fullscreen_request()
    }

    /// The pointer position last reported, in physical pixels of the window.
    #[must_use]
    pub const fn cursor(&self) -> Option<UiPoint> {
        self.cursor
    }

    /// The primary button was pressed at the last known pointer position. A press on a
    /// separator (panel border, Features / Elements divider, playhead row) starts a drag. A
    /// second press on the same renamable tree-row name within [`DOUBLE_CLICK`] opens the Rename
    /// modal after the press has done its normal work (select); a press anywhere else in between
    /// ends the sequence.
    pub fn pointer_pressed(&mut self) -> Update {
        if self.shell.numeric_delivery_pending() {
            return Update::default();
        }
        self.capture_numeric_origin();
        self.drag = None;
        self.intent = None;
        if self.shell.selection().is_open()
            && self.shell.selection().request().is_some()
            && self
                .selection_session
                .as_ref()
                .is_none_or(|(_, bound)| *bound != self.generation)
        {
            return Update::default();
        }
        if self.shell.rename().is_open()
            && self
                .rename_session
                .as_ref()
                .is_none_or(|session| session.bound != self.generation)
        {
            return Update::default();
        }
        let hit = self
            .cursor
            .and_then(|p| hit_test(p, self.metrics, &self.grid));
        if let Some(hit) = hit.as_ref()
            && let Some(grab) = self.shell.selection_thumb_grab(hit)
        {
            self.last_press = None;
            self.drag = Some(CapturedDrag::SelectionThumb { grab });
            return Update::default();
        }
        if let Some(separator) = hit.as_ref().and_then(|h| self.shell.drag_at(h)) {
            self.last_press = None;
            if let Ok(id) = allocate_interaction_identity(&mut self.next_drag_id) {
                self.drag = Some(CapturedDrag::Separator {
                    kind: separator,
                    id,
                    origin: self.generation,
                    bound: self.generation,
                    document: self.shell.tree_projection().document,
                });
                if separator == Separator::Playhead {
                    return self.clear_tree_hover();
                }
            }
            return Update::default();
        }
        let id = hit.and_then(|h| h.widget_id);
        if id.is_some_and(|id| self.shell.progress_press(id)) {
            self.last_press = None;
            self.status.progress.press(self.clock);
            let changed = self
                .shell
                .set_status(self.status.fps, self.status.progress.percent());
            return self.refresh(changed);
        }
        let double = self.register_press(id);
        let token = id.and_then(|id| self.shell.tree_hit(id, self.generation));
        let mut acted = if double {
            false
        } else if let (Some(id), Some(token)) = (id, token) {
            self.shell
                .activate_tree_hit(id, token, self.generation, false)
        } else {
            self.shell.click(hit)
        };
        if let (true, Some(id), Some(origin)) = (double, id, token)
            && let Ok(session_id) = allocate_interaction_identity(&mut self.next_rename_id)
            && self.shell.begin_rename(id)
            && let Some((_, source)) = self.shell.rename_source()
        {
            self.rename_session = Some(RenameSession {
                id: session_id,
                origin,
                bound: self.generation,
                source_name: source.to_owned(),
            });
            acted = true;
        }
        self.refresh(acted)
    }

    /// Records a press on `id` and tells whether it completes a double-click: the previous
    /// press was on the same renamable name, at most [`DOUBLE_CLICK`] earlier, with no other
    /// press in between. A completed double-click is consumed, so a third press starts over.
    fn register_press(&mut self, id: Option<WidgetId>) -> bool {
        let previous = self.last_press.take();
        let Some(id) = id.filter(|&id| self.shell.is_rename_target(id)) else {
            return false;
        };
        let Some(token) = self.shell.tree_hit(id, self.generation) else {
            return false;
        };
        let double = previous.is_some_and(|(first, at)| {
            first == token && self.clock.saturating_sub(at) <= DOUBLE_CLICK
        });
        if !double {
            self.last_press = Some((token, self.clock));
        }
        double
    }

    /// A translated key was pressed.
    pub fn key(&mut self, key: ShellKey) -> Update {
        if self.shell.numeric_delivery_pending() {
            return Update::default();
        }
        self.capture_numeric_origin();
        self.intent = None;
        if self.shell.selection().is_open()
            && self.shell.selection().request().is_some()
            && self
                .selection_session
                .as_ref()
                .is_none_or(|(_, bound)| *bound != self.generation)
        {
            return Update::default();
        }
        if self.shell.rename().is_open()
            && self
                .rename_session
                .as_ref()
                .is_none_or(|session| session.bound != self.generation)
        {
            return Update::default();
        }
        let tree = self.shell.interaction().focused.and_then(|id| {
            self.shell
                .tree_hit(id, self.generation)
                .map(|token| (id, token))
        });
        let acted = if matches!(key, ShellKey::Enter | ShellKey::Char(' '))
            && let Some((id, token)) = tree
        {
            self.shell
                .activate_tree_hit(id, token, self.generation, true)
        } else {
            self.shell.key(key)
        };
        self.refresh(acted)
    }

    /// Recomposes after an input that acted. The hover target is kept only while the pointer is
    /// still on it and it is still eligible; it is never acquired here (M2.25).
    fn refresh(&mut self, acted: bool) -> Update {
        self.refresh_with_cause(acted, FrameCause::External)
    }
    fn refresh_with_cause(&mut self, acted: bool, cause: FrameCause) -> Update {
        if !acted {
            return Update::default();
        }
        let validated = self
            .rename_session
            .clone()
            .filter(|session| session.bound == self.generation);
        let selection_validated = self
            .selection_session
            .clone()
            .filter(|(_, bound)| *bound == self.generation);
        let origin = validated
            .as_ref()
            .map_or(self.generation, |session| session.origin.generation);
        let requested_hover = self.hover_allowed_during_capture(self.hovered, &self.shell);
        let tree_hover = requested_hover
            .is_some_and(|id| semantic_tree_target(&self.published_shell, id).is_some());
        let Ok((mut grid, mut lit)) = self.shell.compose_hovered(requested_hover) else {
            self.shell = self.published_shell.clone();
            return Update::default();
        };
        if !tree_hover && lit && self.widget_under_cursor(&grid) != requested_hover {
            if let Ok((plain, _)) = self.shell.compose_hovered(None) {
                grid = plain;
            }
            lit = false;
        }
        let candidate = self.shell.clone();
        let previous_hover = self.hovered;
        // Keep the old tree target's certificate until publish resolves the current slot.
        self.hovered = requested_hover.filter(|_| tree_hover || lit);
        if self.publish(candidate, grid, cause).is_err() {
            self.shell = self.published_shell.clone();
            self.hovered = previous_hover;
            return Update::default();
        }
        // Publication owns semantic hover validation and capture-end reacquisition.
        // Never restore a reusable slot id after that decision.
        self.selection_request =
            self.shell
                .take_selection_request()
                .filter(|request| match request {
                    newell_ui::SelectionUiRequest::OpenField(d) => {
                        self.shell.selection_field_visible(d.field)
                    }
                    newell_ui::SelectionUiRequest::Complete(c) => {
                        selection_validated.as_ref().is_some_and(|(r, _)| {
                            r.session == c.session
                                && r.document == c.document
                                && r.destination == c.destination
                        })
                    }
                });
        if !self.shell.selection().is_open() {
            self.selection_session = None;
        }
        // The completion is taken only after this event's successful publication. Rename's
        // own modal close certifies this one completion, never the original name hit.
        self.intent = self.shell.take_tree_intent(self.generation, origin);
        if self
            .intent
            .as_ref()
            .is_some_and(|intent| matches!(intent.kind(), newell_ui::TreeIntentKind::Rename { .. }))
        {
            let certified = validated.as_ref().is_some_and(|session| {
                session.id < self.next_rename_id && self.intent.as_ref().is_some_and(|intent| intent.target() == session.origin.target && matches!(intent.kind(), newell_ui::TreeIntentKind::Rename { source_name, .. } if *source_name == session.source_name))
            });
            if !certified {
                self.intent = None;
            }
        }
        if !self.shell.rename().is_open() {
            self.rename_session = None;
        }
        self.published_shell = self.shell.clone();
        Update { grid: true }
    }
    fn publish(
        &mut self,
        candidate: ShellState,
        mut grid: CellGrid,
        cause: FrameCause,
    ) -> Result<(), UiError> {
        let playhead_captured = self.playhead_capture_active();
        let mut hovered = self.hovered;
        if playhead_captured
            && hovered.is_some_and(|id| semantic_tree_target(&self.shell, id).is_some())
        {
            hovered = None;
            grid = candidate.compose_hovered(None)?.0;
        }
        // Row ids are reusable slots. Preserve hover only across the same semantic target
        // in the same document, still under the stationary pointer in the candidate map.
        if !matches!(cause, FrameCause::PointerHover)
            && let Some(id) = hovered
            && let Some(old_target) = semantic_tree_target(&self.published_shell, id)
        {
            let under = self
                .cursor
                .and_then(|point| hit_test(point, self.metrics, &grid))
                .and_then(|hit| candidate.hover_target(hit, &grid));
            let retained =
                under.filter(|id| semantic_tree_target(&candidate, *id) == Some(old_target));
            let (composed, lit) = candidate.compose_hovered(retained)?;
            grid = composed;
            hovered = retained.filter(|_| lit);
        }
        let owned = matches!((cause, self.drag), (FrameCause::OwnedDrag(capture), Some(CapturedDrag::Separator { id, bound, document, .. })) if capture == id && bound == self.generation && document == candidate.tree_projection().document);
        let selection_owned = matches!((cause, self.drag),
            (FrameCause::OwnedSelectionDrag(anchor), Some(CapturedDrag::SelectionThumb { grab }))
            if anchor == grab && self.published_shell.selection_thumb_grab_valid(grab)
                && candidate.selection_thumb_grab_valid(grab));
        let selection_invalid = matches!(self.drag,
            Some(CapturedDrag::SelectionThumb { grab }) if !candidate.selection_thumb_grab_valid(grab));
        let changed = !candidate
            .selection()
            .same_routes(self.published_shell.selection())
            || !candidate.same_tree_routes(&self.published_shell)
            || (grid.cols(), grid.rows()) != (self.grid.cols(), self.grid.rows())
            || !grid
                .cells()
                .iter()
                .map(|cell| (cell.widget_id, cell.shared_border))
                .eq(self
                    .grid
                    .cells()
                    .iter()
                    .map(|cell| (cell.widget_id, cell.shared_border)));
        let generation = if changed {
            ProjectionGeneration::new(
                self.generation
                    .get()
                    .checked_add(1)
                    .ok_or(UiError::ProjectionGenerationExhausted)?,
            )
        } else {
            self.generation
        };
        let capture_invalidated = selection_invalid || (changed && !owned && !selection_owned);
        if capture_invalidated {
            self.drag = None;
            self.last_press = None;
        }
        if owned
            && let Some(CapturedDrag::Separator {
                kind,
                id,
                origin,
                document,
                ..
            }) = self.drag
        {
            self.drag = Some(CapturedDrag::Separator {
                kind,
                id,
                origin,
                bound: generation,
                document,
            });
        }
        self.generation = generation;
        self.selection_session = self
            .selection_session
            .take()
            .filter(|(request, _)| {
                candidate.selection().is_open()
                    && candidate.selection().request().is_some_and(|r| {
                        r.session == request.session
                            && r.document == request.document
                            && r.destination == request.destination
                    })
            })
            .map(|(request, _)| (request, generation));
        self.rename_session = self.rename_session.take().filter(|session| {
            candidate.rename_source().is_some_and(|(target, source)| {
                target == session.origin.target && source == session.source_name
            })
        });
        if let Some(session) = &mut self.rename_session {
            session.bound = generation;
        }
        self.shell = candidate;
        self.published_shell = self.shell.clone();
        self.grid = grid;
        self.hovered = hovered;
        if playhead_captured && capture_invalidated {
            let _ = self.resume_hover_from_published_target();
        }
        Ok(())
    }

    fn playhead_capture_active(&self) -> bool {
        matches!(
            self.drag,
            Some(CapturedDrag::Separator {
                kind: Separator::Playhead,
                ..
            })
        )
    }

    fn clear_tree_hover(&mut self) -> Update {
        if self
            .hovered
            .is_some_and(|id| semantic_tree_target(&self.shell, id).is_some())
        {
            self.set_hover(None)
        } else {
            Update::default()
        }
    }

    fn hover_allowed_during_capture(
        &self,
        target: Option<WidgetId>,
        shell: &ShellState,
    ) -> Option<WidgetId> {
        target.filter(|id| {
            !self.playhead_capture_active() || semantic_tree_target(shell, *id).is_none()
        })
    }

    /// Ends playhead suppression from the current published hit map, never from a saved row id.
    fn resume_hover_from_published_target(&mut self) -> Update {
        let target = self.widget_under_cursor(&self.grid);
        let Ok((grid, lit)) = self.shell.compose_hovered(target) else {
            return Update::default();
        };
        let hovered = target.filter(|_| lit);
        let changed = self.hovered != hovered || self.grid != grid;
        self.hovered = hovered;
        self.grid = grid;
        Update { grid: changed }
    }

    /// The widget under the last known pointer position in `grid`, in O(1) (`ARCHITECTURE.md`
    /// §3.4), with hover-only inheritance for Action Box labels and cyclic values.
    fn widget_under_cursor(&self, grid: &CellGrid) -> Option<WidgetId> {
        let point = self.cursor?;
        self.shell
            .hover_target(hit_test(point, self.metrics, grid)?, grid)
    }

    /// Makes `target` the hover target if the composer highlights it, else none. Recomposes
    /// only when the stored target changes; a target that is not eligible changes nothing.
    fn set_hover(&mut self, target: Option<WidgetId>) -> Update {
        let target = self.hover_allowed_during_capture(target, &self.shell);
        if target == self.hovered {
            return Update::default();
        }
        // A widget that can never be a target (inert bars and window controls) is not composed
        // for: nothing is hovered before and after, so there is nothing to recompose.
        let candidate = target.filter(|&id| ShellState::may_hover(id));
        if candidate.is_none() && self.hovered.is_none() {
            return Update::default();
        }
        let Ok((grid, lit)) = self.shell.compose_hovered(candidate) else {
            return Update::default();
        };
        let hovered = candidate.filter(|_| lit);
        if hovered == self.hovered {
            return Update::default();
        }
        let previous_hover = self.hovered;
        self.hovered = hovered;
        if self
            .publish(self.shell.clone(), grid, FrameCause::PointerHover)
            .is_err()
        {
            self.hovered = previous_hover;
            return Update::default();
        }
        Update { grid: true }
    }
}

#[cfg(test)]
mod tree_contract_tests {
    use super::*;
    use newell_ui::{CellContent, FeatureNodeId, FeatureTreeItem, OpaqueTreeId, TreeIntentKind};

    fn reference() -> ShellController {
        ShellController::with_shell(ShellState::new(82, 40), GridMetrics::new(4, 9).unwrap())
            .unwrap()
    }
    fn at(col: u32, row: u32) -> Option<UiPoint> {
        Some(UiPoint {
            x: col * 4 + 2,
            y: row * 9 + 4,
        })
    }
    fn find(controller: &ShellController, needle: &str) -> (WidgetId, UiPoint) {
        let pattern: Vec<_> = needle.chars().collect();
        (0..controller.grid.rows())
            .find_map(|row| {
                let text: Vec<_> = (0..controller.grid.cols())
                    .map(|col| match controller.grid.get(col, row).unwrap().content {
                        CellContent::Glyph(c) => c,
                        CellContent::Empty => ' ',
                    })
                    .collect();
                text.windows(pattern.len())
                    .position(|window| window == pattern)
                    .map(|col| {
                        (
                            controller
                                .grid
                                .get(col as u32, row)
                                .unwrap()
                                .widget_id
                                .unwrap(),
                            at(col as u32, row).unwrap(),
                        )
                    })
            })
            .expect("requested control is visible")
    }
    fn colored_reference(enabled: bool) -> ShellController {
        let mut controller = reference();
        if enabled {
            controller.key(ShellKey::Alt('v'));
            for _ in 0..4 {
                controller.key(ShellKey::Down);
            }
            controller.key(ShellKey::Enter);
            controller.key(ShellKey::Escape);
        }
        assert_eq!(controller.shell().color_enabled(), enabled);
        controller
    }

    fn capture(controller: &mut ShellController, kind: Separator) {
        let point = match kind {
            Separator::Navigation => at(24, 10),
            Separator::Split => at(10, 16),
            Separator::Playhead => at(10, 12),
        };
        controller.pointer_moved(point);
        controller.pointer_pressed();
        assert!(controller.dragging());
    }
    fn movements(kind: Separator) -> [(u32, u32); 3] {
        match kind {
            Separator::Navigation => [(25, 10), (26, 10), (27, 10)],
            Separator::Split => [(10, 17), (10, 18), (10, 19)],
            Separator::Playhead => [(10, 6), (10, 10), (10, 13)],
        }
    }

    #[test]
    fn all_owned_drags_rebind_across_three_own_frames_then_release() {
        for enabled in [false, true] {
            for kind in [Separator::Navigation, Separator::Split, Separator::Playhead] {
                let mut controller = colored_reference(enabled);
                capture(&mut controller, kind);
                let (id, origin) = match controller.drag {
                    Some(CapturedDrag::Separator { id, origin, .. }) => Some((id, origin)),
                    _ => None,
                }
                .expect("owned capture");
                for (col, row) in movements(kind) {
                    let old_generation = controller.generation;
                    assert!(
                        controller.pointer_moved(at(col, row)).grid,
                        "{kind:?} {col},{row}"
                    );
                    assert!(controller.generation > old_generation);
                    assert!(
                        matches!(controller.drag, Some(CapturedDrag::Separator { id: same, origin: original, bound, .. }) if same == id && original == origin && bound == controller.generation)
                    );
                    assert_eq!(
                        controller.grid(),
                        &controller
                            .shell()
                            .compose_hovered(controller.hovered())
                            .unwrap()
                            .0
                    );
                }
                controller.tick(Duration::from_secs(1));
                assert!(
                    controller.dragging(),
                    "status-only redraw preserves capture"
                );
                controller.pointer_released();
                let frame = controller.grid().clone();
                controller.pointer_moved(at(40, 5));
                assert!(!controller.dragging());
                assert_eq!(controller.grid(), &frame);
            }
        }
    }

    #[test]
    fn owned_drags_cancel_after_movements_on_overlay_replacement_resize_and_exit() {
        for enabled in [false, true] {
            for kind in [Separator::Navigation, Separator::Split, Separator::Playhead] {
                for interrupt in 0..4 {
                    let mut controller = colored_reference(enabled);
                    capture(&mut controller, kind);
                    for (col, row) in movements(kind) {
                        controller.pointer_moved(at(col, row));
                    }
                    match interrupt {
                        0 => {
                            controller.key(ShellKey::MenuBar);
                        }
                        1 => {
                            controller
                                .set_tree_projection(ModelTreeProjection::empty(
                                    TreeDocumentId::new(99),
                                ))
                                .unwrap();
                        }
                        2 => {
                            controller.resize(100 * 4, 45 * 9);
                        }
                        _ => {
                            controller.pointer_moved(None);
                        }
                    }
                    assert!(!controller.dragging(), "{kind:?} interruption {interrupt}");
                    let frame = controller.grid().clone();
                    controller.pointer_moved(at(40, 5));
                    assert_eq!(controller.grid(), &frame);
                }
            }
        }
    }

    #[test]
    fn reused_cells_reorder_replacement_parts_and_overlay_reject_old_hits() {
        for enabled in [false, true] {
            let mut controller = colored_reference(enabled);
            let (id, _) = find(&controller, "Extrude_Base");
            let token = controller
                .shell
                .tree_hit(id, controller.generation)
                .unwrap();
            controller
                .set_tree_projection({
                    let mut p = ModelTreeProjection::empty(TreeDocumentId::new(1));
                    p.features = (0..100)
                        .map(|id| FeatureTreeItem {
                            id: FeatureNodeId::Operation(OpaqueTreeId::new(id)),
                            name: format!("live_{id}"),
                            depth: 1,
                            visibility: None,
                        })
                        .collect();
                    p
                })
                .unwrap();
            assert!(!controller.activate_tree_hit(id, token).grid);
            let (id, point) = find(&controller, "live_0");
            let token = controller
                .shell
                .tree_hit(id, controller.generation)
                .unwrap();
            controller.pointer_moved(Some(point));
            controller.scroll_panel(-20);
            assert!(
                !controller.activate_tree_hit(id, token).grid,
                "visible cell was reused"
            );
            let (id, _) = find(&controller, "live_20");
            let token = controller
                .shell
                .tree_hit(id, controller.generation)
                .unwrap();
            controller.key(ShellKey::MenuBar);
            assert!(
                !controller.activate_tree_hit(id, token).grid,
                "uncovered menu background is blocked too"
            );
            controller.key(ShellKey::Escape);
            let token = controller
                .shell
                .tree_hit(id, controller.generation)
                .unwrap();
            let mut changed = controller.shell.tree_projection().clone();
            changed.features[20].name = "longer_live_operation".into();
            let generation = controller.generation;
            controller.set_tree_projection(changed).unwrap();
            assert!(controller.generation > generation, "Name/Row width changed");
            assert!(!controller.activate_tree_hit(id, token).grid);
        }
    }

    #[test]
    fn rename_completion_is_certified_once_at_close_with_original_hit_generation() {
        for enabled in [false, true] {
            let mut controller = colored_reference(enabled);
            let (_, point) = find(&controller, "Hub_Body");
            controller.pointer_moved(Some(point));
            controller.pointer_pressed();
            let origin = controller.generation;
            controller.set_time(Duration::from_millis(100));
            controller.pointer_pressed();
            assert!(controller.shell.rename().is_open());
            assert!(controller.generation > origin);
            let session_id = controller.rename_session.as_ref().unwrap().id;
            // A layout transition can rebind this owned session, without reviving other hits.
            controller.resize(100 * 4, 45 * 9);
            assert_eq!(controller.rename_session.as_ref().unwrap().id, session_id);
            controller.key(ShellKey::Char('x'));
            let validated = controller.generation;
            let (_, ok) = find(&controller, "OK");
            controller.pointer_moved(Some(ok));
            controller.pointer_pressed();
            assert!(!controller.shell.rename().is_open());
            let intent = controller.take_tree_intent().unwrap();
            assert_eq!(intent.origin_generation(), origin);
            assert_eq!(intent.generation(), controller.generation);
            assert!(intent.generation() > validated);
            assert!(
                matches!(intent.kind(), TreeIntentKind::Rename { source_name, name } if source_name == "Hub_Body" && name == "Hub_Bodyx")
            );
            assert!(controller.take_tree_intent().is_none());
        }
    }

    #[test]
    fn rename_close_certificate_is_lost_on_intervening_map_change() {
        for enabled in [false, true] {
            let mut controller = colored_reference(enabled);
            let (_, point) = find(&controller, "Rim_Body");
            controller.pointer_moved(Some(point));
            controller.pointer_pressed();
            controller.set_time(Duration::from_millis(100));
            controller.pointer_pressed();
            controller.key(ShellKey::Enter);
            assert!(controller.intent.is_some());
            controller.key(ShellKey::MenuBar);
            assert!(controller.take_tree_intent().is_none());
        }
    }

    #[test]
    fn rename_survives_source_preserving_refresh_but_cancels_on_changed_source() {
        for enabled in [false, true] {
            let mut controller = colored_reference(enabled);
            let (_, point) = find(&controller, "Rim_Body");
            controller.pointer_moved(Some(point));
            controller.pointer_pressed();
            controller.set_time(Duration::from_millis(100));
            controller.pointer_pressed();
            let origin = controller.rename_session.as_ref().unwrap().origin;
            let mut projection = controller.shell.tree_projection().clone();
            projection.features[0].name.push_str("_external");
            controller.set_tree_projection(projection).unwrap();
            assert_eq!(controller.rename_session.as_ref().unwrap().origin, origin);
            assert_eq!(
                controller.rename_session.as_ref().unwrap().bound,
                controller.generation
            );
            let mut projection = controller.shell.tree_projection().clone();
            projection.elements[0].name.push_str("_external");
            controller.set_tree_projection(projection).unwrap();
            assert!(!controller.shell.rename().is_open());
            assert!(controller.rename_session.is_none());
            assert!(controller.take_tree_intent().is_none());
        }
    }

    #[test]
    fn failed_frame_preparation_keeps_grid_projection_generation_and_capture_binding() {
        for enabled in [false, true] {
            let mut controller = colored_reference(enabled);
            controller.generation = ProjectionGeneration::new(u64::MAX);
            capture(&mut controller, Separator::Navigation);
            let frame = controller.grid.clone();
            let shell = controller.shell.clone();
            let capture = controller.drag;
            assert!(!controller.pointer_moved(at(30, 10)).grid);
            assert_eq!(controller.grid, frame);
            assert_eq!(controller.shell, shell);
            assert_eq!(controller.drag, capture);
            assert_eq!(controller.generation.get(), u64::MAX);
            assert_eq!(
                controller.set_tree_projection(ModelTreeProjection::empty(TreeDocumentId::new(1))),
                Err(UiError::ProjectionGenerationExhausted)
            );
            assert_eq!(controller.grid, frame);
            assert_eq!(controller.shell, shell);
            assert_eq!(controller.drag, capture);
        }
    }
    #[test]
    fn color_ink_and_glyph_only_publication_preserves_generation_and_capture() {
        let mut controller = colored_reference(true);
        capture(&mut controller, Separator::Navigation);
        let capture = controller.drag;
        let generation = controller.generation;
        let mut grid = controller.grid.clone();
        let cell = grid.get_mut(0, 0).unwrap();
        cell.ink = newell_ui::InkRole::Action;
        cell.content = CellContent::Glyph('x');
        controller
            .publish(controller.shell.clone(), grid, FrameCause::External)
            .unwrap();
        assert_eq!(controller.generation, generation);
        assert_eq!(controller.drag, capture);
    }
}

#[cfg(test)]
mod treeview_addendum_tests {
    use super::*;
    use newell_ui::{
        CellContent, ElementGroup, ElementTreeItem, EntitySelectionView, FeatureNodeId,
        FeatureTreeItem, GapDisplayLocation, InkRole, OpaqueTreeId, TreeTargetKind,
    };

    fn point(col: u32, row: u32) -> Option<UiPoint> {
        Some(UiPoint {
            x: col * 8 + 4,
            y: row * 18 + 9,
        })
    }
    fn projection(count: u64) -> ModelTreeProjection {
        let mut p = ModelTreeProjection::empty(TreeDocumentId::new(123));
        p.features = (0..count)
            .map(|id| FeatureTreeItem {
                id: FeatureNodeId::Operation(OpaqueTreeId::new(id)),
                name: format!("operation_{id}"),
                depth: 1,
                visibility: None,
            })
            .collect();
        p
    }
    fn controller(p: ModelTreeProjection) -> ShellController {
        let mut s = ShellState::new_application(82, 40);
        s.set_tree_projection(p).unwrap();
        s.set_nav_width(25);
        ShellController::with_shell(s, GridMetrics::from_k(2).unwrap()).unwrap()
    }
    fn projection_with_elements(count: u64) -> ModelTreeProjection {
        let mut p = projection(count);
        p.elements = (0..count)
            .map(|id| ElementTreeItem {
                id: OpaqueTreeId::new(100 + id),
                source_feature: OpaqueTreeId::new(id),
                output_ordinal: 0,
                group: ElementGroup::Bodies,
                name: format!("entity_{id}"),
                visible: true,
            })
            .collect();
        p
    }
    fn playhead_cell(c: &ShellController) -> (u32, u32) {
        c.grid
            .cells()
            .iter()
            .enumerate()
            .find_map(|(index, cell)| {
                (cell.content == CellContent::Glyph('─') && cell.widget_id.is_some()).then_some((
                    u32::try_from(index % c.grid.cols() as usize).unwrap(),
                    u32::try_from(index / c.grid.cols() as usize).unwrap(),
                ))
            })
            .unwrap()
    }
    fn text(c: &ShellController, row: u32) -> String {
        (0..c.grid.cols())
            .map(|col| match c.grid.get(col, row).unwrap().content {
                CellContent::Glyph(ch) => ch,
                CellContent::Empty => ' ',
            })
            .collect()
    }
    fn row(c: &ShellController, needle: &str) -> u32 {
        (0..c.grid.rows())
            .find(|r| {
                (0..c.shell.nav_width())
                    .map(|x| match c.grid.get(x, *r).unwrap().content {
                        CellContent::Glyph(ch) => ch,
                        _ => ' ',
                    })
                    .collect::<String>()
                    .contains(needle)
            })
            .unwrap()
    }
    fn assert_plain(c: &ShellController) {
        assert!(c.hovered.is_none());
        assert_eq!(c.grid, c.shell.compose().unwrap());
    }
    #[test]
    fn slot_reuse_scroll_deletion_and_document_replacement_clear_stationary_hover() {
        for change in 0..3 {
            let p = projection(40);
            let mut c = controller(p.clone());
            let r = row(&c, "operation_0");
            c.pointer_moved(point(5, r));
            let old = c.hovered.unwrap();
            let semantic = semantic_tree_target(&c.shell, old).unwrap();
            assert_eq!(
                semantic.kind,
                TreeTargetKind::Operation(OpaqueTreeId::new(0))
            );
            match change {
                0 => {
                    c.scroll_panel(-1);
                }
                1 => {
                    let mut n = p;
                    n.features.remove(0);
                    c.set_tree_projection(n).unwrap();
                }
                _ => {
                    let mut n = p;
                    n.document = TreeDocumentId::new(999);
                    c.set_tree_projection(n).unwrap();
                }
            }
            assert_plain(&c);
            // Motion explicitly acquires the new semantic owner of the same physical cell.
            c.pointer_moved(point(5, r));
            assert!(c.hovered.is_some());
            assert_ne!(
                semantic_tree_target(&c.shell, c.hovered.unwrap()),
                Some(semantic)
            );
        }
    }
    #[test]
    fn failed_semantic_replacement_preserves_the_published_hover_and_map() {
        let mut p = projection(4);
        let mut c = controller(p.clone());
        let r = row(&c, "operation_0");
        c.pointer_moved(point(5, r));
        let hovered = c.hovered;
        let grid = c.grid.clone();
        let shell = c.shell.clone();
        c.generation = ProjectionGeneration::new(u64::MAX);
        p.features.remove(0);
        assert_eq!(
            c.set_tree_projection(p),
            Err(UiError::ProjectionGenerationExhausted)
        );
        assert_eq!(c.hovered, hovered);
        assert_eq!(c.grid, grid);
        assert_eq!(c.shell, shell);
        assert_eq!(c.published_shell, shell);
        assert_eq!(c.generation, ProjectionGeneration::new(u64::MAX));
    }
    #[test]
    fn surviving_row_parts_keep_hover_and_same_width_refresh_keeps_identity() {
        let p = projection(2);
        let mut c = controller(p.clone());
        let r = row(&c, "operation_0");
        assert!(c.pointer_moved(point(5, r)).grid);
        let id = c.hovered.unwrap();
        let generation = c.generation;
        assert!(
            !c.pointer_moved(point(2, r)).grid,
            "guide normalizes to the same row"
        );
        assert_eq!(c.hovered, Some(id));
        assert_eq!(c.generation, generation);
        let mut n = p;
        n.features[1].name = "operation_Z".into();
        c.set_tree_projection(n).unwrap();
        assert_eq!(c.hovered, Some(id));
        assert_eq!(c.grid, c.shell.compose_hovered(Some(id)).unwrap().0);
        c.pointer_moved(None);
        assert_plain(&c);
    }
    #[test]
    fn surviving_target_under_stationary_pointer_rebinds_its_reallocated_row_slot() {
        for input_refresh in [false, true] {
            let mut p = projection(4);
            p.features.insert(
                0,
                FeatureTreeItem {
                    id: FeatureNodeId::Folder(OpaqueTreeId::new(10)),
                    name: "Folder".into(),
                    depth: 1,
                    visibility: None,
                },
            );
            p.features[1].depth = 2;
            let mut c = controller(p.clone());
            c.set_playhead(
                p.placed_gap(GapDisplayLocation::BeforeNode(p.features[1].id))
                    .unwrap(),
            )
            .unwrap();
            let host = row(&c, "Folder");
            c.pointer_moved(point(4, host));
            c.pointer_pressed();
            c.pointer_released();
            let r = row(&c, "operation_2");
            c.pointer_moved(point(5, r));
            let old_id = c.hovered.unwrap();
            let target = semantic_tree_target(&c.shell, old_id).unwrap();
            // Removing the collapsed folder and its child removes one visible node, while
            // the retained anchor's standalone gap adds one line: the target stays in place,
            // but its node-only visible-slot id changes.
            p.features.drain(..2);
            if input_refresh {
                c.shell.set_tree_projection(p).unwrap();
                c.refresh(true);
            } else {
                c.set_tree_projection(p).unwrap();
            }
            assert_eq!(row(&c, "operation_2"), r);
            let new_id = c.hovered.unwrap();
            assert_ne!(new_id, old_id);
            assert_eq!(semantic_tree_target(&c.shell, new_id), Some(target));
            assert_eq!(c.grid, c.shell.compose_hovered(Some(new_id)).unwrap().0);
        }
    }
    #[test]
    fn inline_drag_preserves_anchor_on_host_then_rebinds_owned_moves() {
        let mut p = projection(4);
        p.features.insert(
            0,
            FeatureTreeItem {
                id: FeatureNodeId::Folder(OpaqueTreeId::new(10)),
                name: "Folder".into(),
                depth: 1,
                visibility: None,
            },
        );
        p.features[1].depth = 2;
        p.features[2].depth = 2;
        let mut c = controller(p.clone());
        let gap = p
            .placed_gap(GapDisplayLocation::BeforeNode(p.features[2].id))
            .unwrap();
        c.set_playhead(gap).unwrap();
        let host = row(&c, "Folder");
        c.pointer_moved(point(4, host));
        c.pointer_pressed();
        c.pointer_released();
        assert_eq!(c.shell.playhead(), gap);
        let right = c.shell.nav_width() - 2;
        assert_eq!(
            c.grid.get(right, host).unwrap().content,
            CellContent::Glyph('─')
        );
        c.pointer_moved(point(right, host));
        // The collapsed-folder inline playhead obeys the same hover suppression as the
        // standalone playhead.
        // The collapsed children (operation_0/1) are intentionally hidden. Seed hover on
        // a visible sibling so the test reaches capture without changing collapse policy.
        let stale_row = row(&c, "operation_2");
        c.pointer_moved(point(5, stale_row));
        let stale_hover = c.hovered.unwrap();
        let (hovered, lit) = c.shell.compose_hovered(Some(stale_hover)).unwrap();
        assert!(lit);
        c.grid = hovered;
        c.cursor = point(right, host);
        c.pointer_pressed();
        assert!(c.dragging());
        assert_eq!(c.hovered, None);
        assert_plain(&c);
        let (id, origin) = match c.drag {
            Some(CapturedDrag::Separator { id, origin, .. }) => (id, origin),
            _ => unreachable!(),
        };
        let generation = c.generation;
        c.pointer_moved(point(right - 1, host));
        assert_eq!(c.shell.playhead(), gap);
        assert_eq!(
            c.generation, generation,
            "the host-row no-op publishes no new map"
        );
        assert!(
            matches!(c.drag,Some(CapturedDrag::Separator{id:same,origin:old,bound,..})
            if same==id && old==origin && bound==generation)
        );
        for (needle, operation) in [("operation_2", 2), ("operation_3", 3)] {
            let dest = row(&c, needle);
            let generation = c.generation;
            c.pointer_moved(point(5, dest));
            assert_eq!(
                c.shell.playhead(),
                p.placed_gap(GapDisplayLocation::AfterSubtree(FeatureNodeId::Operation(
                    OpaqueTreeId::new(operation)
                )))
                .unwrap()
            );
            assert!(c.generation > generation);
            assert_eq!(c.hovered, None);
            assert_plain(&c);
            assert!(
                matches!(c.drag,Some(CapturedDrag::Separator{id:same,origin:old,bound,..})
                if same==id && old==origin && bound==c.generation)
            );
        }
        c.pointer_released();
        assert!(!c.dragging());
    }

    #[test]
    fn playhead_capture_clears_and_suppresses_both_tree_hovers_without_changing_selection() {
        let p = projection_with_elements(4);
        let selected_entity = OpaqueTreeId::new(101);
        let mut c = controller(p.clone());
        c.set_entity_selection(EntitySelectionView {
            document: p.document,
            entities: vec![selected_entity],
        })
        .unwrap();
        let feature_selected_row = row(&c, "operation_1");
        let element_selected_row = row(&c, "entity_1");
        assert!(text(&c, feature_selected_row).contains('┣'));
        assert!(text(&c, element_selected_row).contains('┣'));

        // Seed a current tree hover, then place the last pointer position on the playhead
        // without delivering an intervening motion update. Capture must clear the stale hover.
        let old_hover_row = row(&c, "operation_0");
        let hover_point = point(6, old_hover_row).unwrap();
        c.pointer_moved(Some(hover_point));
        let old_hover = c.hovered.unwrap();
        let (hovered_grid, lit) = c.shell.compose_hovered(Some(old_hover)).unwrap();
        assert!(lit);
        c.grid = hovered_grid;
        let (playhead_col, playhead_row) = playhead_cell(&c);
        c.cursor = point(playhead_col, playhead_row);
        assert!(c.pointer_pressed().grid);
        assert!(c.playhead_capture_active());
        assert_eq!(c.hovered, None);
        assert_eq!(c.grid, c.shell.compose().unwrap());

        for needle in ["operation_2", "entity_0", "entity_1"] {
            let row = row(&c, needle);
            c.pointer_moved(point(6, row));
            assert!(c.dragging(), "capture survives movement over {needle}");
            assert_eq!(c.hovered, None, "tree hover suppressed over {needle}");
            assert_eq!(c.grid, c.shell.compose().unwrap());
            assert!(text(&c, feature_selected_row).contains('┣'));
            assert!(text(&c, element_selected_row).contains('┣'));
        }

        assert!(c.pointer_released().grid);
        assert!(!c.dragging());
        assert!(c.hovered.is_some(), "release re-resolves current row hover");
        let element_row_after_release = row(&c, "entity_1");
        assert!(text(&c, element_row_after_release).contains('┣'));
        assert!(text(&c, element_row_after_release).contains('╣'));
        assert_eq!(
            c.grid,
            c.shell.compose_hovered(c.hovered).unwrap().0,
            "selected glyphs remain while current hover resumes"
        );
    }

    #[test]
    fn playhead_cancellation_re_resolves_hover_from_republished_semantics() {
        let p = projection_with_elements(4);
        let mut c = controller(p.clone());
        let (playhead_col, playhead_row) = playhead_cell(&c);
        c.pointer_moved(point(playhead_col, playhead_row));
        c.pointer_pressed();
        assert!(c.playhead_capture_active());
        let old_row = row(&c, "entity_0");
        c.pointer_moved(point(6, old_row));
        assert_eq!(c.hovered, None);

        let mut replacement = p;
        replacement.features.swap(0, 1);
        replacement.elements.swap(0, 1);
        c.set_tree_projection(replacement).unwrap();

        assert!(!c.playhead_capture_active());
        let point = c.cursor.unwrap();
        let expected = c
            .shell
            .hover_target(hit_test(point, c.metrics, &c.grid).unwrap(), &c.grid);
        assert_eq!(c.hovered, expected);
        let semantic = semantic_tree_target(&c.shell, c.hovered.unwrap()).unwrap();
        assert_eq!(
            semantic.kind,
            TreeTargetKind::Entity(OpaqueTreeId::new(101))
        );
        assert!(text(&c, row(&c, "entity_1")).contains('╣'));
    }
    #[test]
    fn tree_hover_color_border_is_contour_and_selected_tail_is_text() {
        let mut c =
            ShellController::with_shell(ShellState::new(82, 30), GridMetrics::from_k(2).unwrap())
                .unwrap();
        // Toggle existing View > Color through its established keyboard route.
        c.key(ShellKey::Alt('v'));
        for _ in 0..4 {
            c.key(ShellKey::Down);
        }
        c.key(ShellKey::Enter);
        c.key(ShellKey::Escape);
        assert!(c.shell.color_enabled());
        let r = row(&c, "Extrude_Cut_Center");
        c.pointer_moved(point(6, r));
        c.pointer_pressed();
        c.pointer_released();
        assert_eq!(c.grid.get(24, r).unwrap().content, CellContent::Glyph('╡'));
        assert_eq!(c.grid.get(24, r).unwrap().ink, InkRole::Contour);
        assert_eq!(c.grid.get(4, r).unwrap().content, CellContent::Glyph('┗'));
        assert_eq!(c.grid.get(5, r).unwrap().content, CellContent::Glyph('╸'));
        assert_eq!(c.grid.get(4, r).unwrap().ink, InkRole::Text);
        c.pointer_moved(None);
        assert_eq!(c.grid.get(24, r).unwrap().content, CellContent::Glyph('│'));
    }
}
