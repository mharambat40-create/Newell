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

use newell_ui::{
    CellGrid, GridMetrics, ResizeZone, Separator, ShellKey, ShellState, UiError, UiPoint,
    WindowAction, composed_grid_size, hit_test, resize_zone,
};

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
        })
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

    /// The pointer moved to `position` (`None`: it left the window). The position is only
    /// remembered for the next press and used to continue a drag; motion alone never changes
    /// the grid (there is no hover).
    pub fn pointer_moved(&mut self, position: Option<UiPoint>) -> Update {
        self.cursor = position;
        let acted = match (self.drag, position) {
            (Some(separator), Some(p)) => self
                .shell
                .drag_separator(separator, p.x / self.metrics.cell_width_px()),
            _ => false,
        };
        self.refresh(acted)
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
        if let Some(action) = id.and_then(ShellState::window_action) {
            return Press::Window(action);
        }
        if id.is_none() && resizable {
            if let Some(zone) = resize_zone(point, surface, self.metrics) {
                return Press::Resize(zone);
            }
        }
        if hit.is_some_and(|h| self.shell.window_drag_at(h.col, h.row, &self.grid)) {
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

    /// The pointer position last reported, in physical pixels of the window.
    #[must_use]
    pub const fn cursor(&self) -> Option<UiPoint> {
        self.cursor
    }

    /// The primary button was pressed at the last known pointer position.
    pub fn pointer_pressed(&mut self) -> Update {
        let hit = self
            .cursor
            .and_then(|p| hit_test(p, self.metrics, &self.grid));
        if let Some(separator) = hit.and_then(|h| self.shell.separator_at(h.col, h.row)) {
            self.drag = Some(separator);
            return Update::default();
        }
        let acted = self.shell.click(hit);
        self.refresh(acted)
    }

    /// A translated key was pressed.
    pub fn key(&mut self, key: ShellKey) -> Update {
        let acted = self.shell.key(key);
        self.refresh(acted)
    }

    /// Recomposes after an input that acted.
    fn refresh(&mut self, acted: bool) -> Update {
        let mut update = Update::default();
        if acted {
            if let Ok(grid) = self.shell.compose() {
                self.grid = grid;
                update.grid = true;
            }
        }
        update
    }
}
