// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Shell layout and composition: pure functions from [`ShellState`] to a flat [`CellGrid`].

use super::data::{COMMAND_MENUS, status_text, visible_rows, with_bar_menus};
use super::{BAR_IDS, COMMAND_IDS, ShellState, Tree, tree_id};
use crate::{
    BorderLine, CellGrid, GridRect, TreeNodeKind, TreeRow, UiError, WidgetContext, WidgetId,
    WidgetState, WidgetStyles, compose_overlays, draw_border, draw_label, draw_menu_bar,
    draw_menu_popup, draw_tree_row, title_rect,
};

/// Default NavigationPanel width, frame included.
pub(super) const NAV_WIDTH: u32 = 25;
/// Narrowest NavigationPanel, frame included (a smaller grid caps it at half its width).
const NAV_MIN: u32 = 8;
/// Rows above the body: MenuBar and CommandBar frames sharing their middle border.
const TOP_ROWS: u32 = 4;
/// Rows of the StatusBar frame.
const STATUS_ROWS: u32 = 3;
/// NavCube placeholder size, frame included.
const CUBE_WIDTH: u32 = 9;
const CUBE_HEIGHT: u32 = 4;
/// Default Extrude ActionPanel width, frame included (`Interface_Exemple.txt`).
pub(super) const PANEL_WIDTH: u32 = 27;
/// Narrowest ActionPanel, frame included (a narrower viewport caps it at its width).
const PANEL_MIN: u32 = 16;
/// Title row, separator and the two frame rows around the panel's body rows.
const PANEL_CHROME_ROWS: u32 = 4;
/// Window controls drawn at the right end of the MenuBar, with a trailing space.
const WINDOW_CONTROLS: &str = "[-] [□] [X] ";
const WINDOW_CONTROLS_WIDTH: u32 = 12;

/// The user-chosen widths of the two resizable panels, in whole cells. They are wishes: the
/// widths actually used are clamped to the current grid by [`Layout`], so a window that shrinks
/// and grows back restores them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PanelWidths {
    pub nav: u32,
    pub action: u32,
}

impl Default for PanelWidths {
    fn default() -> Self {
        Self {
            nav: NAV_WIDTH,
            action: PANEL_WIDTH,
        }
    }
}

/// NavigationPanel width actually used on a grid `cols` wide: at least [`NAV_MIN`] and at most
/// half the grid, and never more than the grid on a tiny one.
pub(super) fn nav_width(cols: u32, wanted: u32) -> u32 {
    let half = cols / 2;
    wanted.clamp(NAV_MIN.min(half), half)
}

/// ActionPanel width actually used in a viewport interior `inner` cells wide.
pub(super) fn action_width(inner: u32, wanted: u32) -> u32 {
    wanted.clamp(PANEL_MIN.min(inner), inner)
}

/// A draggable vertical separator. Dragging snaps to a character column and resizes the panel
/// on its right (`Navigation`: the NavigationPanel's right border) or the ActionPanel on its
/// left (`Action`: the panel's left border).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Separator {
    Navigation,
    Action,
}

/// The framed regions of the shell, in whole cells. Neighbouring frames share their
/// border cells, so [`draw_border`] merges the junctions (`╠ ╦ ╩ ╣`). Regions of a grid too
/// small to hold them come out empty or clipped; nothing here can overflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Layout {
    pub grid: GridRect,
    pub menu: GridRect,
    pub command: GridRect,
    pub features: GridRect,
    pub elements: GridRect,
    pub viewport: GridRect,
    pub status: GridRect,
    /// Effective NavigationPanel width (frame included).
    pub nav: u32,
    /// Effective ActionPanel width (frame included).
    pub action: u32,
}

impl Layout {
    pub(super) fn new(cols: u32, rows: u32, widths: PanelWidths) -> Self {
        let nav = nav_width(cols, widths.nav);
        let status_top = rows.saturating_sub(STATUS_ROWS);
        // Body frames run from row `TOP_ROWS` to the status frame's top row, inclusive.
        let body = status_top.saturating_sub(TOP_ROWS - 1);
        let features_h = body.saturating_mul(3) / 5;
        let (elements_top, elements_h) = if features_h == 0 {
            (TOP_ROWS, body)
        } else {
            (
                TOP_ROWS.saturating_add(features_h - 1),
                body - features_h + 1,
            )
        };
        let viewport = GridRect::new(
            nav.saturating_sub(1),
            TOP_ROWS,
            cols.saturating_sub(nav.saturating_sub(1)),
            body,
        );
        Self {
            grid: GridRect::new(0, 0, cols, rows),
            menu: GridRect::new(0, 0, cols, 3),
            command: GridRect::new(0, 2, cols, 3),
            features: GridRect::new(0, TOP_ROWS, nav, features_h),
            elements: GridRect::new(0, elements_top, nav, elements_h),
            viewport,
            status: GridRect::new(0, status_top, cols, STATUS_ROWS),
            nav,
            action: action_width(viewport.inset(1).width(), widths.action),
        }
    }

    /// The single row of the MenuBar frame.
    pub(super) fn menu_bar(&self) -> GridRect {
        self.menu.inset(1)
    }

    /// The single row of the CommandBar frame.
    pub(super) fn command_bar(&self) -> GridRect {
        self.command.inset(1)
    }

    /// The separator under cell `(col, row)`, by arithmetic on the layout (no cell scan). The
    /// NavigationPanel's right border spans the body rows; the ActionPanel's left border spans
    /// the panel (`action_rows` body rows) when it is open.
    pub(super) fn separator_at(
        &self,
        col: u32,
        row: u32,
        action_rows: Option<usize>,
    ) -> Option<Separator> {
        let in_rows =
            |r: &GridRect| u64::from(row) >= u64::from(r.row()) && u64::from(row) < r.bottom();
        if self.nav >= 2 && col == self.nav - 1 && in_rows(&self.viewport) {
            return Some(Separator::Navigation);
        }
        let panel = self.action_panel(action_rows?);
        (panel.width() >= 2 && col == panel.col() && in_rows(&panel)).then_some(Separator::Action)
    }

    /// The Extrude ActionPanel: right-aligned, `rows` body rows tall, clipped to the viewport.
    /// It sits directly under the NavCube when it fits there; otherwise it moves up over the
    /// cube rather than losing its OK / Cancel row.
    pub(super) fn action_panel(&self, rows: usize) -> GridRect {
        let inner = self.viewport.inset(1);
        let height = u32::try_from(rows)
            .unwrap_or(u32::MAX)
            .saturating_add(PANEL_CHROME_ROWS);
        let top = CUBE_HEIGHT.min(inner.height().saturating_sub(height));
        inner
            .split_top(top)
            .1
            .split_top(height)
            .0
            .split_right(self.action)
            .0
    }
}

impl ShellState {
    pub(super) fn layout(&self) -> Layout {
        Layout::new(self.cols, self.rows, self.widths)
    }

    /// Composes the whole shell: frames, bars, trees, viewport, NavCube, the Extrude
    /// ActionPanel, then the open overlays on top. The output depends only on this state.
    ///
    /// # Errors
    /// Only if the grid size is rejected by [`CellGrid::new`].
    pub fn compose(&self) -> Result<CellGrid, UiError> {
        let mut grid = CellGrid::new(self.cols, self.rows)?;
        let styles = WidgetStyles::default();
        let ctx = WidgetContext {
            styles: &styles,
            interaction: &self.interaction,
        };
        let layout = self.layout();
        self.draw_chrome(&mut grid, &ctx, &layout);
        if let Some(panel) = self.extrude.compose() {
            panel.draw(&mut grid, &ctx, layout.action_panel(panel.rows.len()));
        }
        self.draw_overlays(&mut grid, &ctx, layout.grid);
        Ok(grid)
    }

    fn draw_chrome(&self, grid: &mut CellGrid, ctx: &WidgetContext<'_>, layout: &Layout) {
        let style = ctx.styles.normal;
        for frame in [
            layout.menu,
            layout.command,
            layout.features,
            layout.elements,
            layout.viewport,
            layout.status,
        ] {
            draw_border(grid, frame, BorderLine::Double, style);
        }

        let bar = layout.menu_bar();
        with_bar_menus(&self.view, |menus| {
            draw_menu_bar(grid, ctx, bar, BAR_IDS, menus, &self.menu_bar);
            // Static window controls, only where they do not cover a title.
            let titles_end = menus
                .len()
                .checked_sub(1)
                .and_then(|last| title_rect(bar, menus, last))
                .map_or(0, |r| r.right());
            let (controls, _) = bar.split_right(WINDOW_CONTROLS_WIDTH);
            if u64::from(controls.col()) >= titles_end {
                draw_label(grid, controls, WINDOW_CONTROLS, style);
            }
        });
        draw_menu_bar(
            grid,
            ctx,
            layout.command_bar(),
            COMMAND_IDS,
            &COMMAND_MENUS,
            &self.command_bar,
        );

        self.draw_tree(grid, ctx, layout.features, Tree::Features);
        self.draw_tree(grid, ctx, layout.elements, Tree::Elements);
        draw_viewport(grid, ctx, layout.viewport);
        draw_label(
            grid,
            layout.status.inset(1),
            &status_text(self.workbench),
            style,
        );
    }

    fn draw_tree(&self, grid: &mut CellGrid, ctx: &WidgetContext<'_>, frame: GridRect, tree: Tree) {
        let inner = frame.inset(1);
        let nodes = tree.nodes();
        let open = match tree {
            Tree::Features => &self.features_open,
            Tree::Elements => &self.elements_open,
        };
        let shown = visible_rows(nodes, open);
        for (line, row) in shown.iter().enumerate().take(inner.height() as usize) {
            let Some(node) = nodes.get(row.index) else {
                continue;
            };
            let at = GridRect::new(
                inner.col(),
                inner
                    .row()
                    .saturating_add(u32::try_from(line).unwrap_or(u32::MAX)),
                inner.width(),
                1,
            );
            let tree_row = TreeRow {
                id: tree_id(tree, row.index),
                kind: if node.branch {
                    TreeNodeKind::Branch
                } else {
                    TreeNodeKind::Leaf
                },
                depth: node.depth,
                last: row.last,
                trunk: &row.trunk,
                label: node.label,
                state: WidgetState {
                    expanded: open.get(row.index).copied().unwrap_or(false),
                    checked: node.checked,
                    ..WidgetState::default()
                },
            };
            draw_tree_row(grid, ctx, at, &tree_row);
        }
    }

    fn draw_overlays(&self, grid: &mut CellGrid, ctx: &WidgetContext<'_>, bounds: GridRect) {
        // At most one of: a menu popup, the SelectionPanel, the FormulaEditor. Each modal's
        // `draw` composes the whole stack, so drawing two would erase the first one's content.
        if self.selection.is_open() {
            self.selection.draw(grid, ctx, &self.overlays, bounds);
        } else if self.formula.is_open() {
            self.formula.draw(grid, ctx, &self.overlays, bounds);
        } else {
            compose_overlays(grid, ctx, &self.overlays, bounds, |g, overlay, inner| {
                if overlay.id == BAR_IDS.popup() {
                    with_bar_menus(&self.view, |menus| {
                        draw_menu_popup(g, ctx, inner, BAR_IDS, menus, &self.menu_bar);
                    });
                } else if overlay.id == COMMAND_IDS.popup() {
                    draw_menu_popup(
                        g,
                        ctx,
                        inner,
                        COMMAND_IDS,
                        &COMMAND_MENUS,
                        &self.command_bar,
                    );
                }
            });
        }
    }

    /// Widget ids of the base shell in reading order: the Features rows, the Elements rows,
    /// then the Extrude ActionPanel controls. This is the keyboard focus ring; it is read
    /// from the composed `grid` only when Tab is pressed, never for pointer hit-testing.
    pub(super) fn focus_ring(&self, grid: &CellGrid) -> Vec<WidgetId> {
        let layout = self.layout();
        let mut ring = Vec::new();
        collect_ids(grid, layout.features.inset(1), &mut ring);
        collect_ids(grid, layout.elements.inset(1), &mut ring);
        if let Some(panel) = self.extrude.compose() {
            collect_ids(grid, layout.action_panel(panel.rows.len()), &mut ring);
        }
        ring
    }
}

fn draw_viewport(grid: &mut CellGrid, ctx: &WidgetContext<'_>, frame: GridRect) {
    let style = ctx.styles.normal;
    let inner = frame.inset(1);
    let (strip, rest) = inner.split_right(CUBE_WIDTH);
    draw_label(grid, rest, " 3D Viewport", style);
    // Static NavCube placeholder: no camera control, no interaction ids.
    let cube = strip.split_top(CUBE_HEIGHT).0;
    if cube.width() == CUBE_WIDTH && cube.height() == CUBE_HEIGHT {
        draw_border(grid, cube, BorderLine::Single, style);
        draw_label(
            grid,
            GridRect::new(cube.col().saturating_add(1), cube.row(), CUBE_WIDTH - 2, 1),
            "NavCube",
            style,
        );
    }
}

/// Appends each distinct widget id of `rect` to `out`, row by row.
fn collect_ids(grid: &CellGrid, rect: GridRect, out: &mut Vec<WidgetId>) {
    let visible = rect.intersection(&GridRect::from_grid(grid));
    for row in visible.row()..visible.row().saturating_add(visible.height()) {
        for col in visible.col()..visible.col().saturating_add(visible.width()) {
            if let Some(id) = grid.get(col, row).and_then(|c| c.widget_id) {
                if !out.contains(&id) {
                    out.push(id);
                }
            }
        }
    }
}
