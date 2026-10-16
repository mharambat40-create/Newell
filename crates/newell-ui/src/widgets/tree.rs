// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Tree row primitive.
//!
//! One row of a tree, composed from plain data the Controller derives from its tree and
//! expansion state. No tree structure is stored or walked here. The grammar is the
//! NavigationPanel of `docs/ui_design/Interface_Exemple.txt` (`README_UI.md` §26).

use super::{Canvas, WidgetContext, owned_blank, text_len};
use crate::{CellGrid, GridRect, WidgetId, WidgetState};

/// Whether a row can expand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeNodeKind {
    /// Shows `[-]` (`state.expanded`) or `[+]`.
    Branch,
    /// Has no expansion marker.
    Leaf,
}

/// The widget ids of the parts of one row. They are distinct so a press tells a marker, a
/// visibility checkbox and a name apart; every other cell of the row (guides, spacing, trailing
/// blanks) belongs to `row`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeRowIds {
    pub row: WidgetId,
    pub marker: WidgetId,
    pub check: WidgetId,
    pub name: WidgetId,
}

/// Data for one tree row.
///
/// - `depth`: nesting level; 0 is a root row, which has no guide cells.
/// - `last`: this row is the last child of its parent (`╰` instead of `├`).
/// - `trunk`: for each ancestor level between the root and the parent (`depth - 1` entries),
///   whether that ancestor has later siblings, so a `│` guide continues through this row.
///   Missing entries count as `false`.
/// - `spaced`: the name of a branch follows its marker after one blank (`├[-] Hub_Features`),
///   as for a user-named row; a category header follows its marker directly
///   (`├[-]Solid Bodies(2)`).
/// - `check`: the visibility checkbox of the row, `■` for `Some(true)`, `□` for `Some(false)`.
#[derive(Debug, Clone, Copy)]
pub struct TreeRow<'a> {
    pub ids: TreeRowIds,
    pub kind: TreeNodeKind,
    pub depth: u32,
    pub last: bool,
    pub trunk: &'a [bool],
    pub spaced: bool,
    pub check: Option<bool>,
    pub label: &'a str,
    pub state: WidgetState,
}

/// `label` fitted to `room` cells: unchanged if it fits, else its leading characters with the
/// final cell replaced by `…` (U+2026). One scalar is one cell.
#[must_use]
pub fn ellipsize(label: &str, room: u32) -> String {
    if text_len(label) <= room {
        return label.to_owned();
    }
    let keep = (room as usize).saturating_sub(1);
    let mut cut: String = label.chars().take(keep).collect();
    if room > 0 {
        cut.push('…');
    }
    cut
}

/// Draws a tree row on the first row of `rect`. Every cell of the row is owned: guides,
/// spacing and trailing blanks by `ids.row`, the `[-]`/`[+]` marker by `ids.marker`, the
/// `□`/`■` cell by `ids.check` and the name by `ids.name`.
///
/// Layout, matching the reference trees (`.` is a blank):
///
/// ```text
/// [-]Features             depth 0 branch, category (no blank before the name)
/// .├.□.Sketch_Hole        depth 1 leaf with checkbox
/// .├[-].Hub_Features      depth 1 branch, named (`spaced`)
/// .│.├.Extrude_Base       depth 2 leaf
/// .│.╰╴Extrude_Cut        last leaf: `╰╴` replaces the connector and the blank
/// .╰[+]Construct          last branch: the marker follows `╰` directly
/// ```
///
/// A name that does not fit the cells left of it is shortened by [`ellipsize`]. The row uses
/// one style (selected, focused, ...). Content past the rect width is clipped.
pub fn draw_tree_row(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    rect: GridRect,
    row: &TreeRow<'_>,
) {
    let ids = row.ids;
    let style = ctx.style_of(ids.row, row.state);
    let normal = ctx.styles.normal;
    let mut canvas = Canvas::new(grid, rect);
    canvas.fill(0, canvas.width(), 0, owned_blank(style, ids.row));
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
        guides.push(if row.last { '╰' } else { '├' });
        dx = canvas.text(dx, 0, &guides, normal, Some(ids.row));
        if row.kind == TreeNodeKind::Leaf {
            let tail = if row.last { "╴" } else { " " };
            dx = canvas.text(dx, 0, tail, normal, Some(ids.row));
        }
    }
    if row.kind == TreeNodeKind::Branch {
        let marker = if row.state.expanded { "[-]" } else { "[+]" };
        dx = canvas.text(dx, 0, marker, style, Some(ids.marker));
        if row.spaced {
            dx = canvas.text(dx, 0, " ", style, Some(ids.row));
        }
    }
    if let Some(on) = row.check {
        dx = canvas.text(dx, 0, if on { "■" } else { "□" }, style, Some(ids.check));
        dx = canvas.text(dx, 0, " ", style, Some(ids.row));
    }
    let room = canvas.width().saturating_sub(dx);
    canvas.text(dx, 0, &ellipsize(row.label, room), style, Some(ids.name));
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::super::{WidgetContext, WidgetStyles};
    use super::*;
    use crate::{InteractionState, StyleId};

    const W: u32 = 24;

    /// Row 5 with marker 6, check 7 and name 8.
    fn ids() -> TreeRowIds {
        TreeRowIds {
            row: id(5),
            marker: id(6),
            check: id(7),
            name: id(8),
        }
    }

    fn row<'a>(
        kind: TreeNodeKind,
        depth: u32,
        last: bool,
        trunk: &'a [bool],
        label: &'a str,
    ) -> TreeRow<'a> {
        TreeRow {
            ids: ids(),
            kind,
            depth,
            last,
            trunk,
            spaced: false,
            check: None,
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

    fn state(expanded: bool) -> WidgetState {
        WidgetState {
            expanded,
            ..WidgetState::default()
        }
    }

    #[test]
    fn root_branch_shows_expanded_state() {
        let mut r = row(TreeNodeKind::Branch, 0, true, &[], "Features");
        r.state = state(true);
        assert_eq!(text(&r), padded("[-]Features"));
        r.state = state(false);
        assert_eq!(text(&r), padded("[+]Features"));
    }

    #[test]
    fn nested_rows_match_the_reference_shapes() {
        let mut r = row(TreeNodeKind::Leaf, 1, false, &[], "Sketch_Profile");
        r.check = Some(false);
        assert_eq!(text(&r), padded(".├.□.Sketch_Profile"));
        r.check = Some(true);
        assert_eq!(text(&r), padded(".├.■.Sketch_Profile"));
        r = row(TreeNodeKind::Branch, 1, false, &[], "Hub_Features");
        r.state = state(true);
        r.spaced = true;
        assert_eq!(text(&r), padded(".├[-].Hub_Features"));
        r = row(TreeNodeKind::Branch, 1, false, &[], "Solid Bodies(2)");
        r.state = state(true);
        assert_eq!(text(&r), padded(".├[-]Solid.Bodies(2)"));
        r = row(TreeNodeKind::Branch, 1, true, &[], "Construct");
        assert_eq!(text(&r), padded(".╰[+]Construct"));
        r = row(TreeNodeKind::Leaf, 1, false, &[], "Fillet_Inner");
        assert_eq!(text(&r), padded(".├.Fillet_Inner"));
        r = row(TreeNodeKind::Leaf, 1, true, &[], "Pattern");
        assert_eq!(text(&r), padded(".╰╴Pattern"));
        r = row(TreeNodeKind::Leaf, 2, false, &[true], "Extrude_Base");
        assert_eq!(text(&r), padded(".│.├.Extrude_Base"));
        r.check = Some(true);
        r.label = "Sketch_Hole";
        assert_eq!(text(&r), padded(".│.├.■.Sketch_Hole"));
        r = row(TreeNodeKind::Leaf, 2, true, &[true], "Extrude_Cut");
        assert_eq!(text(&r), padded(".│.╰╴Extrude_Cut"));
        r.check = Some(false);
        r.label = "Hub_Body";
        assert_eq!(text(&r), padded(".│.╰╴□.Hub_Body"));
        r = row(TreeNodeKind::Leaf, 2, true, &[false], "x");
        assert_eq!(text(&r), padded("...╰╴x"));
        r = row(TreeNodeKind::Leaf, 3, false, &[], "y");
        assert_eq!(text(&r), padded(".....├.y"));
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
        let mut r = row(TreeNodeKind::Leaf, 0, true, &[], "Solid");
        r.check = Some(false);
        assert_eq!(text(&r), padded("□.Solid"));
    }

    #[test]
    fn long_names_end_in_an_ellipsis_in_the_last_cell() {
        // The reference row: `Sketch_Profile_Ma…` fills the 23-cell panel interior exactly.
        let mut r = row(TreeNodeKind::Leaf, 1, false, &[], "Sketch_Profile_Main");
        r.check = Some(false);
        let g = {
            let styles = WidgetStyles::default();
            let inter = InteractionState::default();
            let mut g = grid(23, 1);
            draw_tree_row(
                &mut g,
                &ctx(&styles, &inter),
                GridRect::new(0, 0, 23, 1),
                &r,
            );
            g
        };
        assert_eq!(text_row(&g, 0), ".├.□.Sketch_Profile_Ma…");
        assert_eq!(ellipsize("abcdef", 6), "abcdef");
        assert_eq!(ellipsize("abcdefg", 6), "abcde…");
        assert_eq!(ellipsize("abc", 1), "…");
        assert_eq!(ellipsize("abc", 0), "");
        assert_eq!(ellipsize("", 0), "");
        assert_eq!(ellipsize("é", 1), "é");
    }

    #[test]
    fn every_part_has_its_own_owner() {
        let mut r = row(TreeNodeKind::Branch, 1, false, &[], "Hub");
        r.spaced = true;
        r.check = Some(true);
        r.state = state(true);
        let g = draw(&r, &InteractionState::default());
        // " ├[-] ■ Hub" then blanks: guides 5, marker 6, check 7, name 8.
        assert_eq!(owner_row(&g, 0), format!("55666575888{}", "5".repeat(13)));
    }

    #[test]
    fn styles_for_guides_selection_focus_and_disabled() {
        let styles = WidgetStyles::default();
        let mut r = row(TreeNodeKind::Leaf, 1, false, &[], "Sketch");
        r.check = Some(false);
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
