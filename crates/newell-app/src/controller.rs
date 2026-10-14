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
    CellGrid, GridMetrics, Separator, ShellKey, ShellState, UiError, UiPoint, WidgetId, hit_test,
};

/// What the caller must do after an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Update {
    /// The grid was recomposed: upload it.
    pub grid: bool,
    /// The hovered widget changed: send the hover id to the shader.
    pub hover: bool,
}

impl Update {
    /// `true` if anything must be uploaded or redrawn.
    #[must_use]
    pub const fn any(self) -> bool {
        self.grid || self.hover
    }

    /// Both effects of two updates.
    #[must_use]
    pub const fn merge(self, other: Self) -> Self {
        Self {
            grid: self.grid || other.grid,
            hover: self.hover || other.hover,
        }
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

    /// The widget under the pointer, for the shader's hover uniform.
    #[must_use]
    pub const fn hovered(&self) -> Option<WidgetId> {
        self.shell.interaction().hovered
    }

    /// The shell state (read-only).
    #[must_use]
    pub const fn shell(&self) -> &ShellState {
        &self.shell
    }

    /// The pointer moved to `position` (`None`: it left the window).
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
    /// the whole cells that fit (see [`GridMetrics::grid_size`]). A size that does not change
    /// the cell grid does nothing.
    pub fn resize(&mut self, width_px: u32, height_px: u32) -> Update {
        let (cols, rows) = self.metrics.grid_size(width_px, height_px);
        let acted = self.shell.resize(cols, rows);
        self.refresh(acted)
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

    /// Recomposes after an input that acted, then re-resolves hover: the cells under a
    /// stationary pointer may have a new owner.
    fn refresh(&mut self, acted: bool) -> Update {
        let mut update = Update::default();
        if acted {
            if let Ok(grid) = self.shell.compose() {
                self.grid = grid;
                update.grid = true;
            }
        }
        update.hover = self.resolve_hover();
        update
    }

    fn resolve_hover(&mut self) -> bool {
        let hit = self
            .cursor
            .and_then(|p| hit_test(p, self.metrics, &self.grid));
        self.shell.pointer_moved(hit)
    }
}
