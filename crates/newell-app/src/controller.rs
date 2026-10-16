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
    CellGrid, GridMetrics, GridRect, ResizeZone, Separator, ShellKey, ShellState, UiError, UiPoint,
    WidgetId, WindowAction, composed_grid_size, hit_test, resize_zone,
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
    grid: CellGrid,
    metrics: GridMetrics,
    cursor: Option<UiPoint>,
    /// The separator being dragged (pointer capture): temporary interaction state.
    drag: Option<Separator>,
    /// Monotonic time of the input being handled, set by the application ([`Self::set_time`]).
    clock: Duration,
    /// The last press on a renamable tree-row name and when it happened (double-click state).
    last_press: Option<(WidgetId, Duration)>,
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
        let shell = ShellState::new(cols, rows);
        let grid = shell.compose()?;
        Ok(Self {
            shell,
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

    /// Whole-cell area where the 3D scene is drawn.
    #[must_use]
    pub fn scene_viewport(&self) -> GridRect {
        self.shell.scene_viewport()
    }

    /// `true` when a scene click cannot be intercepted by a menu or modal.
    #[must_use]
    pub fn scene_input_enabled(&self) -> bool {
        self.shell.scene_input_enabled()
    }

    /// The pointer moved to `position` (`None`: it left the window). The position is remembered
    /// for the next press and used to continue a drag. Motion changes the grid only through the
    /// ASCII hover target (M2.25): it is resolved from the cell under the pointer and recomposed
    /// only when that target changes.
    pub fn pointer_moved(&mut self, position: Option<UiPoint>) -> Update {
        self.cursor = position;
        let acted = match (self.drag, position) {
            (Some(separator), Some(p)) => self.shell.drag_separator(
                separator,
                p.x / self.metrics.cell_width_px(),
                p.y / self.metrics.cell_height_px(),
            ),
            _ => false,
        };
        let update = self.refresh(acted);
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
        self.metrics = prepared.metrics;
        self.shell = prepared.shell;
        self.grid = prepared.grid;
        self.cursor = None;
        self.drag = None;
        self.last_press = None;
        self.hovered = None;
        Update { grid: true }
    }

    /// The primary button was pressed at the last known pointer position on a window whose
    /// surface is `surface` physical pixels. Precedence: a window control, then (when
    /// `resizable`) a resize zone on a cell with no widget, then unused MenuBar space, then
    /// normal shell routing. Only the last case changes shell state.
    pub fn press(&mut self, surface: (u32, u32), resizable: bool) -> Press {
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
        let hit = self
            .cursor
            .and_then(|p| hit_test(p, self.metrics, &self.grid));
        if let Some(separator) = hit.as_ref().and_then(|h| self.shell.drag_at(h)) {
            self.last_press = None;
            self.drag = Some(separator);
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
        let mut acted = self.shell.click(hit);
        if let (true, Some(id)) = (double, id) {
            acted |= self.shell.begin_rename(id);
        }
        self.refresh(acted)
    }

    /// Records a press on `id` and tells whether it completes a double-click: the previous
    /// press was on the same renamable name, at most [`DOUBLE_CLICK`] earlier, with no other
    /// press in between. A completed double-click is consumed, so a third press starts over.
    fn register_press(&mut self, id: Option<WidgetId>) -> bool {
        let previous = self.last_press.take();
        let Some(id) = id.filter(|&id| ShellState::is_rename_target(id)) else {
            return false;
        };
        let double = previous.is_some_and(|(first, at)| {
            first == id && self.clock.saturating_sub(at) <= DOUBLE_CLICK
        });
        if !double {
            self.last_press = Some((id, self.clock));
        }
        double
    }

    /// A translated key was pressed.
    pub fn key(&mut self, key: ShellKey) -> Update {
        let acted = self.shell.key(key);
        self.refresh(acted)
    }

    /// Recomposes after an input that acted. The hover target is kept only while the pointer is
    /// still on it and it is still eligible; it is never acquired here (M2.25).
    fn refresh(&mut self, acted: bool) -> Update {
        if !acted {
            return Update::default();
        }
        let Ok((mut grid, mut lit)) = self.shell.compose_hovered(self.hovered) else {
            return Update::default();
        };
        if lit && self.widget_under_cursor(&grid) != self.hovered {
            // The target moved from under the pointer or was covered: drop the highlight.
            if let Ok((plain, _)) = self.shell.compose_hovered(None) {
                grid = plain;
            }
            lit = false;
        }
        self.hovered = self.hovered.filter(|_| lit);
        self.grid = grid;
        Update { grid: true }
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
        self.grid = grid;
        Update { grid: true }
    }
}
