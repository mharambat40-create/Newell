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
    OwnedDrag(u64),
    OwnedSelectionDrag(SelectionThumbGrab),
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
            grid,
            metrics,
            cursor: None,
            drag: None,
            clock: Duration::ZERO,
            last_press: None,
            hovered: None,
            status: crate::status::StatusClock::default(),
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
    pub fn take_tree_intent(&mut self) -> Option<TreeIntent> {
        self.intent
            .take()
            .filter(|intent| intent.generation() == self.generation)
    }
    pub fn set_tree_projection(
        &mut self,
        projection: ModelTreeProjection,
    ) -> Result<Update, UiError> {
        let mut candidate = self.shell.clone();
        if !candidate.set_tree_projection(projection)? {
            return Ok(Update::default());
        }
        let grid = candidate.compose()?;
        self.publish(candidate, grid, FrameCause::External)?;
        self.drag = None;
        self.last_press = None;
        self.intent = None;
        Ok(Update { grid: true })
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
    pub fn invalidate_document_interactions(&mut self) {
        self.drag = None;
        self.last_press = None;
        self.intent = None;
        self.rename_session = None;
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
    pub fn pointer_released(&mut self) {
        self.drag = None;
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
        self.drag = None;
        self.intent = None;
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
        let mut acted = if let (Some(id), Some(token)) = (id, token) {
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
        self.intent = None;
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
        let origin = validated
            .as_ref()
            .map_or(self.generation, |session| session.origin.generation);
        let Ok((mut grid, mut lit)) = self.shell.compose_hovered(self.hovered) else {
            self.shell = self.published_shell.clone();
            return Update::default();
        };
        if lit && self.widget_under_cursor(&grid) != self.hovered {
            if let Ok((plain, _)) = self.shell.compose_hovered(None) {
                grid = plain;
            }
            lit = false;
        }
        let candidate = self.shell.clone();
        if self.publish(candidate, grid, cause).is_err() {
            self.shell = self.published_shell.clone();
            return Update::default();
        }
        self.hovered = self.hovered.filter(|_| lit);
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
        // Row ids are reusable slots. Preserve hover only across the same semantic target
        // in the same document, still under the stationary pointer in the candidate map.
        if let Some(id) = self.hovered
            && let Some(old_target) = self.published_shell.tree_hover_target(id)
        {
            let under = self.cursor.and_then(|point| hit_test(point, self.metrics, &grid))
                .and_then(|hit| candidate.hover_target(hit, &grid));
            let retained = candidate.tree_hover_target(id) == Some(old_target) && under == Some(id);
            let (composed, lit) = candidate.compose_hovered(retained.then_some(id))?;
            grid = composed;
            if !retained || !lit { self.hovered = None; }
        }
        let owned = matches!((cause, self.drag), (FrameCause::OwnedDrag(capture), Some(CapturedDrag::Separator { id, bound, document, .. })) if capture == id && bound == self.generation && document == candidate.tree_projection().document);
        let selection_owned = matches!((cause, self.drag),
            (FrameCause::OwnedSelectionDrag(anchor), Some(CapturedDrag::SelectionThumb { grab }))
            if anchor == grab && self.published_shell.selection_thumb_grab_valid(grab)
                && candidate.selection_thumb_grab_valid(grab));
        let selection_invalid = matches!(self.drag,
            Some(CapturedDrag::SelectionThumb { grab }) if !candidate.selection_thumb_grab_valid(grab));
        let changed = !candidate.same_tree_routes(&self.published_shell)
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
        if selection_invalid || (changed && !owned && !selection_owned) {
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
        Ok(())
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
        if target == self.hovered {
            return Update::default();
        }
        // A widget that can never be a target (tree rows, bars, window controls) is not composed
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
        self.hovered = hovered;
        if self
            .publish(self.shell.clone(), grid, FrameCause::External)
            .is_err()
        {
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
            Separator::Navigation => [(30, 10), (31, 10), (32, 10)],
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
                    assert_eq!(controller.grid(), &controller.shell().compose().unwrap());
                }
                controller.tick(Duration::from_secs(1));
                assert!(
                    controller.dragging(),
                    "status-only redraw preserves capture"
                );
                controller.pointer_released();
                let frame = controller.grid().clone();
                controller.pointer_moved(at(3, 5));
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
                    controller.pointer_moved(at(3, 5));
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
