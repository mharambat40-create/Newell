// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Shell layout and composition: pure functions from [`ShellState`] to a flat [`CellGrid`].

use super::data::{
    self, COMMAND_MENUS, Line, NodeKind, full_label, has_children, status_text, with_bar_menus,
};
use super::{BAR_IDS, COMMAND_IDS, PLAYHEAD_ID, ShellState, Tree, TreePart, WINDOW_BASE};
use crate::widgets::Canvas;
use crate::{
    BorderLine, CellGrid, GridRect, InteractionState, TreeNodeKind, TreeRow, TreeRowIds, UiError,
    WidgetContext, WidgetId, WidgetState, WidgetStyles, compose_overlays, draw_border, draw_label,
    draw_menu_bar, draw_menu_popup, title_rect,
};

/// Default NavigationPanel width, frame included.
pub(super) const NAV_WIDTH: u32 = 25;
/// Narrowest NavigationPanel, frame included (a smaller grid caps it at half its width).
const NAV_MIN: u32 = 14;
/// Rows above the body when both MenuBar and CommandBar are visible.
pub(super) const TOP_ROWS: u32 = 4;
/// Shared border row at the bottom of the MenuBar, also the body top when Task bar is hidden.
const MENU_BODY_TOP: u32 = 2;
/// Features pane height of the reference, frame included (`Interface_Exemple.txt`: rows 5..17).
/// At the default split every row of the body beyond the reference body belongs to Elements
/// (`README_UI.md` §25).
const DEFAULT_FEATURES_ROWS: u32 = 13;
/// Shortest Features or Elements pane, frame included: border, header and border.
const PANE_MIN: u32 = 3;
/// Rows of the StatusBar frame.
const STATUS_ROWS: u32 = 3;
/// NavCube placeholder size, frame included.
const CUBE_WIDTH: u32 = 9;
const CUBE_HEIGHT: u32 = 4;
/// Action Box width, frame included: fixed (`README_UI.md` §2.2, `Interface_Exemple.txt`).
pub(super) const PANEL_WIDTH: u32 = crate::action_box::BOX_WIDTH;
/// Window controls drawn at the right end of the MenuBar, with a trailing space.
const WINDOW_CONTROLS: [&str; 3] = ["[-]", "[□]", "[X]"];
const WINDOW_CONTROLS_WIDTH: u32 = 12;

/// The Features / Elements split the user dragged: the Features pane (frame included) was
/// `features` rows in a body of `body` rows. A body of another height scales it, so a window
/// resize keeps the proportion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Split {
    pub features: u32,
    pub body: u32,
}

/// The user-chosen sizes of the resizable panels, in whole cells. They are wishes: the sizes
/// actually used are clamped to the current grid by [`Layout`], so a window that shrinks and
/// grows back restores them. `split` is `None` until the divider is dragged (the reference
/// split). The Action Box is not resizable: its width is fixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PanelWidths {
    pub nav: u32,
    pub split: Option<Split>,
}

impl Default for PanelWidths {
    fn default() -> Self {
        Self {
            nav: NAV_WIDTH,
            split: None,
        }
    }
}

/// Features pane height (frame included) in a body of `body` rows: the reference height (at
/// most three fifths of the body) by default, else the dragged split scaled to the body. At
/// least [`PANE_MIN`] rows each for the two panes where the body allows it.
pub(super) fn features_rows(body: u32, split: Option<Split>) -> u32 {
    let wanted = match split {
        Some(s) if s.body > 0 => {
            let scaled = (u64::from(s.features) * u64::from(body) + u64::from(s.body) / 2)
                / u64::from(s.body);
            u32::try_from(scaled).unwrap_or(u32::MAX)
        }
        _ => (body.saturating_mul(3) / 5).min(DEFAULT_FEATURES_ROWS),
    };
    // The panes share one border row, so Elements gets `body - features + 1` rows.
    let max = body.saturating_add(1).saturating_sub(PANE_MIN);
    wanted.clamp(PANE_MIN.min(max), max)
}

/// NavigationPanel width actually used on a grid `cols` wide: at least [`NAV_MIN`] and at most
/// half the grid, and never more than the grid on a tiny one.
pub(super) fn nav_width(cols: u32, wanted: u32, content_max: u32) -> u32 {
    let half = (cols / 2).min(content_max);
    wanted.clamp(NAV_MIN.min(half), half)
}

/// Action Box width actually used in a viewport interior `inner` cells wide: the fixed
/// [`PANEL_WIDTH`], or the interior if that is narrower (the box is then cropped).
pub(super) fn action_width(inner: u32) -> u32 {
    PANEL_WIDTH.min(inner)
}

/// A draggable separator. Dragging snaps to a character column and resizes the panel
/// on its right (`Navigation`: the NavigationPanel's right border).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Separator {
    Navigation,
    /// The horizontal divider between the Features and Elements panes (their shared border).
    Split,
    /// The playhead separator row of the Features tree (`│ ─────────────────── `).
    Playhead,
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
    /// Effective Action Box width (frame included).
    pub action: u32,
}

impl Layout {
    pub(super) fn new(
        cols: u32,
        rows: u32,
        widths: PanelWidths,
        content_max: u32,
        show_tree: bool,
        show_command_bar: bool,
        show_status: bool,
    ) -> Self {
        let top_rows = if show_command_bar {
            TOP_ROWS
        } else {
            MENU_BODY_TOP
        };
        let nav = if show_tree {
            nav_width(cols, widths.nav, content_max)
        } else {
            0
        };
        let status_height = if show_status { STATUS_ROWS } else { 0 };
        let status_top = rows.saturating_sub(status_height);
        // Body frames run from row `top_rows` to the status frame's top row, inclusive,
        // or to the grid's last row when the status bar is hidden.
        let body = if show_status {
            status_top.saturating_sub(top_rows - 1)
        } else {
            rows.saturating_sub(top_rows)
        };
        let features_h = features_rows(body, widths.split);
        let (elements_top, elements_h) = if features_h == 0 {
            (top_rows, body)
        } else {
            (
                top_rows.saturating_add(features_h - 1),
                body - features_h + 1,
            )
        };
        let viewport = GridRect::new(
            nav.saturating_sub(1),
            top_rows,
            cols.saturating_sub(nav.saturating_sub(1)),
            body,
        );
        Self {
            grid: GridRect::new(0, 0, cols, rows),
            menu: GridRect::new(0, 0, cols, 3),
            command: if show_command_bar {
                GridRect::new(0, MENU_BODY_TOP, cols, 3)
            } else {
                GridRect::new(0, MENU_BODY_TOP, cols, 0)
            },
            features: GridRect::new(0, top_rows, nav, features_h),
            elements: GridRect::new(0, elements_top, nav, elements_h),
            viewport,
            status: GridRect::new(0, status_top, cols, status_height),
            nav,
            action: action_width(viewport.inset(1).width()),
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
    /// NavigationPanel's right border spans the body rows.
    pub(super) fn separator_at(&self, col: u32, row: u32) -> Option<Separator> {
        let in_rows =
            |r: &GridRect| u64::from(row) >= u64::from(r.row()) && u64::from(row) < r.bottom();
        if self.nav >= 2 && col == self.nav - 1 && in_rows(&self.viewport) {
            return Some(Separator::Navigation);
        }
        // The divider is the shared border row of the two panes, between the frame ring and
        // the panel's right border.
        if self.features.height() > 0
            && self.elements.height() > 0
            && row == self.elements.row()
            && col >= 1
            && col + 1 < self.nav
        {
            return Some(Separator::Split);
        }
        None
    }

    /// The Extrude Action Box: right-aligned, `rows` rows tall (frame included), clipped to
    /// the viewport. It sits directly under the NavCube when it fits there; otherwise it moves
    /// up over the cube rather than losing its OK / Cancel row.
    pub(super) fn action_panel(&self, rows: usize) -> GridRect {
        let inner = self.viewport.inset(1);
        let height = u32::try_from(rows).unwrap_or(u32::MAX);
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
        Layout::new(
            self.cols,
            self.rows,
            self.widths,
            self.tree_content_max,
            self.view_panel_visible(0),
            self.view_panel_visible(1),
            self.view_panel_visible(2),
        )
    }

    /// Composes the whole shell: frames, bars, trees, viewport, NavCube, the Extrude
    /// Action Box, then the open overlays on top. The output depends only on this state.
    ///
    /// # Errors
    /// Only if the grid size is rejected by [`CellGrid::new`].
    pub fn compose(&self) -> Result<CellGrid, UiError> {
        self.compose_hovered(None).map(|(grid, _)| grid)
    }

    /// Composes the shell with the ASCII hover target `hovered` (M2.25, `README_UI.md` §5.12).
    /// The target is highlighted only where it is eligible: while an overlay is open, only the
    /// topmost overlay may be hovered, so the Action Box beneath it is never highlighted. The
    /// grid equals [`Self::compose`] apart from the substituted border glyphs, and `hovered` is
    /// not stored. Returns the grid and whether the hovered target was highlighted.
    ///
    /// # Errors
    /// Only if the grid size is rejected by [`CellGrid::new`].
    pub fn compose_hovered(&self, hovered: Option<WidgetId>) -> Result<(CellGrid, bool), UiError> {
        let hovered = hovered.map(|id| {
            super::tree_decode(id).map_or(id, |(tree, slot, _)| {
                super::tree_part_id(tree, slot, TreePart::Row)
            })
        });
        let mut grid = CellGrid::new(self.cols, self.rows)?;
        let styles = WidgetStyles::default();
        let below = InteractionState {
            hovered: hovered.filter(|_| self.input_free()),
            ..self.interaction
        };
        let above = InteractionState {
            hovered,
            ..self.interaction
        };
        let base = WidgetContext {
            styles: &styles,
            interaction: &below,
        };
        let top = WidgetContext {
            styles: &styles,
            interaction: &above,
        };
        let layout = self.layout();
        self.draw_chrome(&mut grid, &base, &layout);
        let mut lit = false;
        if !self.selection.is_profile_picker()
            && let Some(panel) = self.panel().compose_box()
        {
            let (_, drawn) = panel.draw_with(
                &mut grid,
                &base,
                layout.action_panel(panel.height()),
                Some(self.fields()),
            );
            lit |= drawn;
        }
        lit |= self.draw_overlays(&mut grid, &top, layout.grid);
        if self.color_enabled() {
            single_contours(&mut grid);
        }
        if self.input_free()
            && let Some(id) = hovered
        {
            for (tree, frame) in [
                (Tree::Features, layout.features),
                (Tree::Elements, layout.elements),
            ] {
                if let Some((shown_tree, slot, _)) = super::tree_decode(id)
                    && shown_tree == tree
                    && let Some(index) = self.tree_slots[tree.slot()].get(slot)
                    && let Some(line) = self
                        .tree_lines(tree)
                        .iter()
                        .skip(self.tree_scroll[tree.slot()])
                        .take(frame.inset(1).height() as usize)
                        .position(|line| matches!(line, Line::Node(row) if row.index == *index))
                    && frame.width() >= 2
                    && let Some(cell) = grid.get_mut(
                        frame.col() + frame.width() - 1,
                        frame.inset(1).row() + u32::try_from(line).unwrap_or(u32::MAX),
                    )
                {
                    cell.content =
                        crate::CellContent::Glyph(if self.color_enabled() { '╡' } else { '╣' });
                    lit = true;
                }
            }
        }
        Ok((grid, lit))
    }

    fn draw_chrome(&self, grid: &mut CellGrid, ctx: &WidgetContext<'_>, layout: &Layout) {
        let style = ctx.styles.normal;
        for frame in [
            layout.menu,
            layout.features,
            layout.elements,
            layout.viewport,
            layout.status,
        ] {
            draw_border(grid, frame, BorderLine::Double, style);
        }
        if layout.command.height() > 0 {
            draw_border(grid, layout.command, BorderLine::Double, style);
        }

        let bar = layout.menu_bar();
        with_bar_menus(&self.view, |menus| {
            draw_menu_bar(grid, ctx, bar, BAR_IDS, menus, &self.menu_bar);
            // Window controls (stable ids), only where they do not cover a title.
            let titles_end = menus
                .len()
                .checked_sub(1)
                .and_then(|last| title_rect(bar, menus, last))
                .map_or(0, |r| r.right());
            let (controls, _) = bar.split_right(WINDOW_CONTROLS_WIDTH);
            if u64::from(controls.col()) >= titles_end {
                let mut canvas = Canvas::new(grid, controls);
                for (i, label) in WINDOW_CONTROLS.iter().enumerate() {
                    let id = WidgetId::new(WINDOW_BASE + u32::try_from(i).unwrap_or(0));
                    canvas.text_with_ink(
                        4 * u32::try_from(i).unwrap_or(0),
                        0,
                        label,
                        style,
                        Some(id),
                        crate::InkRole::Action,
                    );
                }
            }
        });
        if layout.command.height() > 0 {
            draw_menu_bar(
                grid,
                ctx,
                layout.command_bar(),
                COMMAND_IDS,
                &COMMAND_MENUS,
                &self.command_bar,
            );
        }

        self.draw_tree(grid, ctx, layout.features, Tree::Features);
        self.draw_tree(grid, ctx, layout.elements, Tree::Elements);
        draw_viewport(grid, ctx, layout.viewport, self.view_panel_visible(3));
        let mut status = status_text(self.workbench, self.fps, self.progress);
        if let Some(diagnostic) = &self.formula_diagnostic {
            status = status.replacen(data::CONSTRAINT, diagnostic, 1);
        }
        if layout.status.height() > 0 {
            draw_label(grid, layout.status.inset(1), &status, style);
        }
        // Stamp only the fixed progress segment, never adjacent separators or the frame.

        if layout.status.height() > 0
            && let Some(byte) = status.find("Progress [")
        {
            let offset = status[..byte].chars().count() as u32;
            let inner = layout.status.inset(1);
            for dx in offset..offset.saturating_add(21).min(inner.width()) {
                if let Some(cell) = grid.get_mut(inner.col() + dx, inner.row()) {
                    cell.widget_id = Some(WidgetId::new(super::PROGRESS_ID));
                }
            }
        }
    }

    /// The lines of `tree` currently shown (before clipping to the pane): the visible nodes
    /// and, in the Features tree, the playhead separator.
    pub(super) fn tree_lines(&self, tree: Tree) -> Vec<Line> {
        let nodes = &self.nodes[tree.slot()];
        let open: Vec<_> = nodes
            .iter()
            .map(|node| self.expanded.contains(&node.target))
            .collect();
        let rows = data::visible_rows(nodes, &open);
        let boundary = (tree == Tree::Features
            && self.inline_playhead_host().is_none()
            && self
                .expanded
                .contains(&self.projection.target(super::TreeTargetKind::FeaturesRoot)))
        .then(|| self.projection.display_boundary(self.playhead.display).ok())
        .flatten();
        let mut lines = Vec::new();
        let mut inserted = false;
        for row in rows {
            if !inserted && boundary.is_some_and(|boundary| row.index > boundary) {
                lines.push(Line::Playhead);
                inserted = true;
            }
            lines.push(Line::Node(row));
        }
        if boundary.is_some() && !inserted {
            lines.push(Line::Playhead);
        }
        lines
    }

    fn draw_tree(&self, grid: &mut CellGrid, ctx: &WidgetContext<'_>, frame: GridRect, tree: Tree) {
        let inner = frame.inset(1);
        let nodes = &self.nodes[tree.slot()];
        let inline_host = if tree == Tree::Features {
            self.inline_playhead_host()
        } else {
            None
        };
        for (line, shown) in self
            .tree_lines(tree)
            .iter()
            .skip(self.tree_scroll[tree.slot()])
            .take(inner.height() as usize)
            .enumerate()
        {
            let at = GridRect::new(
                inner.col(),
                inner
                    .row()
                    .saturating_add(u32::try_from(line).unwrap_or(u32::MAX)),
                inner.width(),
                1,
            );
            let row = match shown {
                Line::Playhead => {
                    draw_playhead(grid, ctx, at, self.playhead_depth(), &self.playhead_trunk());
                    continue;
                }
                Line::Node(row) => row,
            };
            let Some(node) = nodes.get(row.index) else {
                continue;
            };
            let label = full_label(nodes, row.index, None);
            let tree_row = TreeRow {
                ids: TreeRowIds {
                    row: self.node_id(tree, row.index, TreePart::Row),
                    marker: self.node_id(tree, row.index, TreePart::Marker),
                    check: self.node_id(tree, row.index, TreePart::Check),
                    name: self.node_id(tree, row.index, TreePart::Name),
                },
                kind: if has_children(nodes, row.index) {
                    TreeNodeKind::Branch
                } else {
                    TreeNodeKind::Leaf
                },
                depth: node.depth,
                last: row.last,
                trunk: &row.trunk,
                spaced: node.kind == NodeKind::Named,
                check: node.check,
                label: &label,
                state: WidgetState {
                    expanded: self.expanded.contains(&node.target),
                    selected: self.is_entity_target_selected(node.target),
                    ..WidgetState::default()
                },
            };
            let inline = (inline_host == Some(node.target)).then_some(WidgetId::new(PLAYHEAD_ID));
            tree_row.draw_with_inline(grid, ctx, at, inline);
        }
    }

    /// Draws the open overlay layer. Returns `true` if the hovered target was highlighted.
    fn draw_overlays(
        &self,
        grid: &mut CellGrid,
        ctx: &WidgetContext<'_>,
        bounds: GridRect,
    ) -> bool {
        // At most one of: a menu popup, the SelectionPanel, the FormulaEditor, the Rename modal. Each modal's
        // `draw` composes the whole stack, so drawing two would erase the first one's content.
        if self.selection.is_open() {
            self.selection.draw(grid, ctx, &self.overlays, bounds);
            false
        } else if self.formula.is_open() {
            self.formula.draw(grid, ctx, &self.overlays, bounds)
        } else if self.rename.is_open() {
            self.rename.draw(grid, ctx, &self.overlays, bounds)
        } else {
            let mut lit = false;
            compose_overlays(grid, ctx, &self.overlays, bounds, |g, overlay, inner| {
                if overlay.id == BAR_IDS.popup() {
                    with_bar_menus(&self.view, |menus| {
                        lit |= draw_menu_popup(g, ctx, inner, BAR_IDS, menus, &self.menu_bar);
                    });
                } else if overlay.id == COMMAND_IDS.popup() {
                    lit |= draw_menu_popup(
                        g,
                        ctx,
                        inner,
                        COMMAND_IDS,
                        &COMMAND_MENUS,
                        &self.command_bar,
                    );
                }
            });
            lit
        }
    }

    /// Widget ids of the base shell in reading order: the Features rows, the Elements rows
    /// (the row id of each node shown, never the playhead), then the Extrude Action Box
    /// controls. This is the keyboard focus ring; the panel ids are read from the composed
    /// `grid` only when Tab is pressed, never for pointer hit-testing.
    pub(super) fn focus_ring(&self, grid: &CellGrid) -> Vec<WidgetId> {
        let layout = self.layout();
        let mut ring = Vec::new();
        for tree in [Tree::Features, Tree::Elements] {
            ring.extend(
                (0..self.tree_slots[tree.slot()].len()).map(|slot| super::tree_id(tree, slot)),
            );
        }
        if let Some(panel) = self.panel().compose_box() {
            collect_ids(grid, layout.action_panel(panel.height()), &mut ring);
        }
        ring
    }
}

/// Boundary guides are inert Text cells; only the horizontal stroke admits capture.
fn draw_playhead(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    rect: GridRect,
    depth: u32,
    trunk: &[bool],
) {
    let style = ctx.styles.normal;
    let mut canvas = Canvas::new(grid, rect);
    for level in 0..depth.min(canvas.reach() / 2) {
        if trunk.get(level as usize).copied().unwrap_or(false) {
            canvas.text(
                level.saturating_mul(2).saturating_add(1),
                0,
                "│",
                style,
                None,
            );
        }
    }
    let start = depth.saturating_mul(2).saturating_add(1);
    let count = canvas.width().saturating_sub(start).min(canvas.reach());
    canvas.text(
        start,
        0,
        &"─".repeat(count as usize),
        style,
        Some(WidgetId::new(PLAYHEAD_ID)),
    );
}

fn draw_viewport(grid: &mut CellGrid, ctx: &WidgetContext<'_>, frame: GridRect, show_cube: bool) {
    let style = ctx.styles.normal;
    let inner = frame.inset(1);
    let (strip, rest) = inner.split_right(CUBE_WIDTH);
    draw_label(grid, rest, " 3D Viewport", style);
    // Static NavCube placeholder: no camera control, no interaction ids.
    let cube = strip.split_top(CUBE_HEIGHT).0;
    if show_cube && cube.width() == CUBE_WIDTH && cube.height() == CUBE_HEIGHT {
        draw_border(grid, cube, BorderLine::Single, style);
        // Exact rounded corners of Interface_Exemple.txt; the placeholder is inert.
        for (dx, dy, glyph) in [
            (0, 0, '╭'),
            (CUBE_WIDTH - 1, 0, '╮'),
            (0, CUBE_HEIGHT - 1, '╰'),
            (CUBE_WIDTH - 1, CUBE_HEIGHT - 1, '╯'),
        ] {
            if let Some(cell) = grid.get_mut(cube.col() + dx, cube.row() + dy) {
                cell.content = crate::CellContent::Glyph(glyph);
            }
        }
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
            if let Some(id) = grid.get(col, row).and_then(|c| c.widget_id)
                && !out.contains(&id)
            {
                out.push(id);
            }
        }
    }
}

/// Color presentation changes only structural glyphs, after existing hover composition.
fn single_contours(grid: &mut CellGrid) {
    for row in 0..grid.rows() {
        for col in 0..grid.cols() {
            let Some(cell) = grid.get_mut(col, row) else {
                continue;
            };
            if cell.ink != crate::InkRole::Contour {
                continue;
            }
            if let crate::CellContent::Glyph(ch) = cell.content {
                let single = match ch {
                    '═' => '─',
                    '║' => '│',
                    '╔' => '┌',
                    '╗' => '┐',
                    '╚' => '└',
                    '╝' => '┘',
                    '╠' => '├',
                    '╣' => '┤',
                    '╦' => '┬',
                    '╩' => '┴',
                    '╬' => '┼',
                    _ => continue,
                };
                cell.content = crate::CellContent::Glyph(single);
            }
        }
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;
    use crate::{BorderAxis, CellContent, InkRole, SharedBorder, StyleId, WidgetId};
    #[test]
    fn eleven_frame_mappings_preserve_metadata_and_exclude_other_roles() {
        let doubles = "═║╔╗╚╝╠╣╦╩╬─│╭╮╰╯┆╞╡╘";
        let singles = "─│┌┐└┘├┤┬┴┼─│╭╮╰╯┆╞╡╘";
        for ink in [InkRole::Text, InkRole::Contour, InkRole::Action] {
            let mut grid = CellGrid::new(doubles.chars().count() as u32, 1).unwrap();
            for (x, ch) in doubles.chars().enumerate() {
                let cell = grid.get_mut(x as u32, 0).unwrap();
                cell.content = CellContent::Glyph(ch);
                cell.ink = ink;
                cell.style = StyleId::new(3);
                cell.widget_id = Some(WidgetId::new(12));
                cell.shared_border = Some(SharedBorder {
                    axis: BorderAxis::Vertical,
                    low: Some(WidgetId::new(1)),
                    high: Some(WidgetId::new(2)),
                });
            }
            let before = grid.clone();
            single_contours(&mut grid);
            let expected = if ink == InkRole::Contour {
                singles
            } else {
                doubles
            };
            for ((a, b), ch) in before
                .cells()
                .iter()
                .zip(grid.cells())
                .zip(expected.chars())
            {
                assert_eq!(b.content, CellContent::Glyph(ch));
                assert_eq!(
                    (a.ink, a.style, a.widget_id, a.shared_border),
                    (b.ink, b.style, b.widget_id, b.shared_border)
                );
            }
        }
    }
}
