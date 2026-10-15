// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Tree row primitive.
//!
//! One row of a tree, composed from plain data the Controller derives from its tree and
//! expansion state. No tree structure is stored or walked here.

use super::{Canvas, WidgetContext, owned_blank};
use crate::{CellGrid, GridRect, WidgetId, WidgetState};

/// Whether a row can expand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeNodeKind {
    /// Shows `[-]` (`state.expanded`) or `[+]`.
    Branch,
    /// Shows `□` or `■` (`state.checked`). The meaning of the marker is left to the caller
    /// (see `docs/ui_design/README.md` §12.2).
    Leaf,
}

/// Data for one tree row.
///
/// - `depth`: nesting level; 0 is a root row, which has no guide cells.
/// - `last`: this row is the last child of its parent (`└` instead of `├`).
/// - `trunk`: for each ancestor level between the root and the parent (`depth - 1` entries),
///   whether that ancestor has later siblings, so a `│` guide continues through this row.
///   Missing entries count as `false`.
#[derive(Debug, Clone, Copy)]
pub struct TreeRow<'a> {
    pub id: WidgetId,
    pub kind: TreeNodeKind,
    pub depth: u32,
    pub last: bool,
    pub trunk: &'a [bool],
    pub label: &'a str,
    pub state: WidgetState,
}

/// Draws a tree row on the first row of `rect`; every cell of that row (guides, marker,
/// label and trailing blanks) is owned by `row.id`, so a click anywhere on the row routes to it.
///
/// Layout, matching the reference trees:
///
/// ```text
/// [-] Features            depth 0, branch
///  ├[-] Hub_Features      depth 1, branch
///  │ ├ □ Sketch_Hole      depth 2, leaf, ancestor continues
///  │ └ ■ Hub_Body         depth 2, last leaf
/// ```
///
/// The row uses one style (selected, focused, ...).
/// Content past the rect width is clipped.
pub fn draw_tree_row(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    rect: GridRect,
    row: &TreeRow<'_>,
) {
    let style = ctx.style_of(row.id, row.state);
    let mut canvas = Canvas::new(grid, rect);
    canvas.fill(0, canvas.width(), 0, owned_blank(style, row.id));
    let id = Some(row.id);
    let mut dx = 0;
    if row.depth > 0 {
        // Guides past the last visible column are clipped anyway. Generating only the levels
        // that can show keeps a huge `depth` from allocating or iterating per level.
        let levels = (row.depth - 1).min(canvas.reach() / 2);
        let mut guides = String::from(" ");
        for level in 0..levels {
            let open = row.trunk.get(level as usize).copied().unwrap_or(false);
            guides.push(if open { '│' } else { ' ' });
            guides.push(' ');
        }
        guides.push(if row.last { '└' } else { '├' });
        dx = canvas.text(dx, 0, &guides, ctx.styles.normal, id);
    }
    let marker = match (row.kind, row.state) {
        (TreeNodeKind::Branch, s) if s.expanded => "[-] ",
        (TreeNodeKind::Branch, _) => "[+] ",
        (TreeNodeKind::Leaf, s) if s.checked && row.depth > 0 => " ■ ",
        (TreeNodeKind::Leaf, _) if row.depth > 0 => " □ ",
        (TreeNodeKind::Leaf, s) if s.checked => "■ ",
        (TreeNodeKind::Leaf, _) => "□ ",
    };
    let dx = canvas.text(dx, 0, marker, style, id);
    canvas.text(dx, 0, row.label, style, id);
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::super::{WidgetContext, WidgetStyles};
    use super::*;
    use crate::{InteractionState, StyleId};

    const W: u32 = 24;

    fn row<'a>(
        kind: TreeNodeKind,
        depth: u32,
        last: bool,
        trunk: &'a [bool],
        label: &'a str,
    ) -> TreeRow<'a> {
        TreeRow {
            id: id(5),
            kind,
            depth,
            last,
            trunk,
            label,
            state: WidgetState::default(),
        }
    }

    fn draw(r: &TreeRow<'_>, inter: &InteractionState) -> CellGrid {
        let styles = WidgetStyles::default();
        let mut g = grid(W, 1);
        draw_tree_row(&mut g, &ctx(&styles, inter), GridRect::new(0, 0, W, 1), r);
        g
    }

    fn padded(s: &str) -> String {
        format!("{s:.<24}")
    }

    fn text(r: &TreeRow<'_>) -> String {
        text_row(&draw(r, &InteractionState::default()), 0)
    }

    fn state(expanded: bool, checked: bool) -> WidgetState {
        WidgetState {
            expanded,
            checked,
            ..WidgetState::default()
        }
    }

    #[test]
    fn root_branch_shows_expanded_state() {
        let mut r = row(TreeNodeKind::Branch, 0, true, &[], "Features");
        r.state = state(true, false);
        assert_eq!(text(&r), padded("[-].Features"));
        r.state = state(false, false);
        assert_eq!(text(&r), padded("[+].Features"));
    }

    #[test]
    fn nested_rows_match_reference_shape() {
        let mut r = row(TreeNodeKind::Leaf, 1, false, &[], "Sketch_Profile");
        assert_eq!(
            text(&r),
            ".├.□.Sketch_Profile....."
                .replace('.', " ")
                .replace(' ', ".")
        );
        r = row(TreeNodeKind::Branch, 1, false, &[], "Hub_Features");
        r.state = state(true, false);
        assert_eq!(text(&r), padded(".├[-].Hub_Features"));
        r = row(TreeNodeKind::Branch, 1, true, &[], "Construct");
        assert_eq!(text(&r), padded(".└[+].Construct"));
        r = row(TreeNodeKind::Leaf, 2, false, &[true], "Sketch_Hole");
        assert_eq!(text(&r), padded(".│.├.□.Sketch_Hole"));
        r = row(TreeNodeKind::Leaf, 2, true, &[true], "Hub_Body");
        r.state = state(false, true);
        assert_eq!(text(&r), padded(".│.└.■.Hub_Body"));
        r = row(TreeNodeKind::Leaf, 2, true, &[false], "x");
        assert_eq!(text(&r), padded("...└.□.x"));
        r = row(TreeNodeKind::Leaf, 3, false, &[], "y");
        assert_eq!(text(&r), padded(".....├.□.y"));
    }

    #[test]
    fn indentation_grows_two_cells_per_depth() {
        for depth in 1..5_u32 {
            let r = row(TreeNodeKind::Leaf, depth, false, &[], "n");
            let g = draw(&r, &InteractionState::default());
            let connector = (2 * depth - 1) as usize;
            let chars: Vec<char> = text_row(&g, 0).chars().collect();
            assert_eq!(chars[connector], '├');
            assert!(chars[..connector].iter().all(|c| *c == '.'));
        }
    }

    #[test]
    fn root_leaf_has_no_guides() {
        let r = row(TreeNodeKind::Leaf, 0, true, &[], "Solid");
        assert_eq!(text(&r), padded("□.Solid"));
    }

    #[test]
    fn whole_row_is_owned_by_one_id() {
        let r = row(TreeNodeKind::Leaf, 2, false, &[true], "Sketch_Hole");
        let g = draw(&r, &InteractionState::default());
        assert_eq!(owner_row(&g, 0), "5".repeat(W as usize));
    }

    #[test]
    fn styles_for_guides_selection_focus_and_disabled() {
        let styles = WidgetStyles::default();
        let mut r = row(TreeNodeKind::Leaf, 1, false, &[], "Sketch");
        let g = draw(&r, &InteractionState::default());
        assert_eq!(g.get(1, 0).unwrap().style, styles.normal);
        assert_eq!(g.get(3, 0).unwrap().style, styles.normal);
        r.state = WidgetState {
            selected: true,
            ..WidgetState::default()
        };
        let g = draw(&r, &InteractionState::default());
        assert_eq!(g.get(1, 0).unwrap().style, styles.normal);
        assert_eq!(g.get(3, 0).unwrap().style, styles.selected);
        assert_eq!(g.get(W - 1, 0).unwrap().style, styles.selected);
        r.state = WidgetState::default();
        let mut inter = InteractionState::default();
        inter.set_focused(Some(id(5)));
        assert_eq!(draw(&r, &inter).get(3, 0).unwrap().style, styles.focused);
        r.state = WidgetState {
            disabled: true,
            ..WidgetState::default()
        };
        assert_eq!(draw(&r, &inter).get(3, 0).unwrap().style, styles.disabled);
        assert_eq!(StyleId::DEFAULT, styles.normal);
    }

    #[test]
    fn clipping_and_small_rects() {
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let c = WidgetContext {
            styles: &styles,
            interaction: &inter,
        };
        let r = row(TreeNodeKind::Leaf, 2, false, &[true], "Sketch_Hole");
        let mut g = grid(8, 1);
        draw_tree_row(&mut g, &c, GridRect::new(0, 0, 5, 1), &r);
        assert_eq!(text_row(&g, 0), ".│.├....");
        assert_eq!(owner_row(&g, 0), "55555---");
        let mut e = grid(8, 1);
        draw_tree_row(&mut e, &c, GridRect::new(0, 0, 0, 1), &r);
        draw_tree_row(&mut e, &c, GridRect::new(0, 0, 8, 0), &r);
        draw_tree_row(&mut e, &c, GridRect::new(100, 0, 8, 1), &r);
        assert_eq!(e, grid(8, 1));
    }

    #[test]
    fn extreme_depth_is_clipped_without_building_guides() {
        // Regression: guides used to be generated for every level before clipping, so
        // `depth = u32::MAX` allocated billions of cells. The visible output must be the
        // same as for a shallower row whose connector lies past the edge.
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let trunk = [true; 4];
        let deep = TreeRow {
            depth: u32::MAX,
            trunk: &trunk,
            ..row(TreeNodeKind::Leaf, 0, false, &[], "x")
        };
        let mut g = grid(8, 1);
        draw_tree_row(
            &mut g,
            &ctx(&styles, &inter),
            GridRect::new(0, 0, u32::MAX, 1),
            &deep,
        );
        assert_eq!(text_row(&g, 0), ".│.│.│.│");
        assert_eq!(owner_row(&g, 0), "55555555");
        let shallow = TreeRow { depth: 5, ..deep };
        let mut s = grid(8, 1);
        draw_tree_row(
            &mut s,
            &ctx(&styles, &inter),
            GridRect::new(0, 0, u32::MAX, 1),
            &shallow,
        );
        assert_eq!(g, s);
    }

    #[test]
    fn deterministic() {
        let r = row(TreeNodeKind::Branch, 1, true, &[], "Solid Bodies(2)");
        let i = InteractionState::default();
        assert_eq!(draw(&r, &i), draw(&r, &i));
    }
}
