// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Logical pane borders written into a [`CellGrid`] as box-drawing glyphs.
//!
//! Borders are plain `CellContent::Glyph` cells; no rasterisation happens here.
//! Where a border meets an existing border glyph the two are merged into the
//! matching junction instead of overwriting it.
//!
//! Supported vocabulary: the single (`┌ ┐ └ ┘ ─ │ ├ ┤ ┬ ┴ ┼`) and double
//! (`╔ ╗ ╚ ╝ ═ ║ ╠ ╣ ╦ ╩ ╬`) families. Junctions mixing single and double
//! arms (e.g. `╞`) are outside this vocabulary: such a merge promotes the whole
//! junction to the double family so the result is always a supported glyph.
//!
//! Merging is order-independent for glyph topology: drawing borders A then B
//! yields the same glyphs as B then A. `StyleId` is last-writer-wins: a cell's
//! final style is the one supplied by the most recent `draw_border` call that
//! touched it.

use super::GridRect;
use crate::{CellContent, CellGrid, StyleId};

/// Line family used for a border.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderLine {
    Single,
    Double,
}

/// Weight of one arm of a box-drawing glyph. Ordered so `max` merges arms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Arm {
    None,
    Single,
    Double,
}

/// The four arms of a glyph: north, east, south, west.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Arms([Arm; 4]);

const N: usize = 0;
const E: usize = 1;
const S: usize = 2;
const W: usize = 3;

const fn arms(n: Arm, e: Arm, s: Arm, w: Arm) -> Arms {
    Arms([n, e, s, w])
}

use Arm::{Double as D, None as O, Single as L};

/// Every glyph in the supported vocabulary and its arms.
const GLYPHS: [(char, Arms); 22] = [
    ('─', arms(O, L, O, L)),
    ('│', arms(L, O, L, O)),
    ('┌', arms(O, L, L, O)),
    ('┐', arms(O, O, L, L)),
    ('└', arms(L, L, O, O)),
    ('┘', arms(L, O, O, L)),
    ('├', arms(L, L, L, O)),
    ('┤', arms(L, O, L, L)),
    ('┬', arms(O, L, L, L)),
    ('┴', arms(L, L, O, L)),
    ('┼', arms(L, L, L, L)),
    ('═', arms(O, D, O, D)),
    ('║', arms(D, O, D, O)),
    ('╔', arms(O, D, D, O)),
    ('╗', arms(O, O, D, D)),
    ('╚', arms(D, D, O, O)),
    ('╝', arms(D, O, O, D)),
    ('╠', arms(D, D, D, O)),
    ('╣', arms(D, O, D, D)),
    ('╦', arms(O, D, D, D)),
    ('╩', arms(D, D, O, D)),
    ('╬', arms(D, D, D, D)),
];

fn decode(c: char) -> Option<Arms> {
    GLYPHS.iter().find(|(g, _)| *g == c).map(|(_, a)| *a)
}

/// Merges two arm sets and resolves mixed weights to the heaviest family.
fn merge(a: Arms, b: Arms) -> Arms {
    let mut out = [Arm::None; 4];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = a.0[i].max(b.0[i]);
    }
    let top = out.iter().copied().max().unwrap_or(Arm::None);
    for slot in &mut out {
        if *slot != Arm::None {
            *slot = top;
        }
    }
    Arms(out)
}

fn encode(a: Arms) -> Option<char> {
    GLYPHS.iter().find(|(_, g)| *g == a).map(|(c, _)| *c)
}

/// Position of a cell along one axis of the rectangle.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pos {
    First,
    Mid,
    Last,
}

/// Arms of the border piece at a cell with the given column/row position.
/// Corners point inward; edges run along the edge.
fn piece(line: BorderLine, col: Pos, row: Pos) -> Arms {
    let w = match line {
        BorderLine::Single => Arm::Single,
        BorderLine::Double => Arm::Double,
    };
    let mut a = [Arm::None; 4];
    match col {
        Pos::First => a[E] = w,
        Pos::Last => a[W] = w,
        Pos::Mid => {
            a[E] = w;
            a[W] = w;
        }
    }
    match row {
        Pos::First => a[S] = w,
        Pos::Last => a[N] = w,
        Pos::Mid => {
            a[N] = w;
            a[S] = w;
        }
    }
    // A pure edge only runs along itself: drop the axis that is not on an edge.
    if col == Pos::Mid {
        a[N] = Arm::None;
        a[S] = Arm::None;
    }
    if row == Pos::Mid {
        a[E] = Arm::None;
        a[W] = Arm::None;
    }
    Arms(a)
}

fn pos(v: u32, first: u32, last: u32) -> Pos {
    if v == first {
        Pos::First
    } else if v == last {
        Pos::Last
    } else {
        Pos::Mid
    }
}

fn put(grid: &mut CellGrid, col: u32, row: u32, new: Arms, style: StyleId) {
    let Some(cell) = grid.get_mut(col, row) else {
        return;
    };
    let merged = match cell.content {
        CellContent::Glyph(c) => decode(c).map_or(new, |old| merge(old, new)),
        CellContent::Empty => new,
    };
    if let Some(c) = encode(merged) {
        cell.content = CellContent::Glyph(c);
        cell.style = style;
        cell.widget_id = None;
    }
}

/// Draws the one-cell-thick outline of `rect` into `grid`, clipped to the grid.
///
/// The outline lies inside `rect`. Rects narrower or shorter than two cells
/// have no interior-bounded outline and draw nothing. Cells written take
/// `style` and `widget_id = None`; all other cells are untouched.
pub fn draw_border(grid: &mut CellGrid, rect: GridRect, line: BorderLine, style: StyleId) {
    if rect.width() < 2 || rect.height() < 2 {
        return;
    }
    let visible = rect.intersection(&GridRect::from_grid(grid));
    if visible.is_empty() {
        return;
    }
    // Last column/row of the rect; exact since width/height >= 2.
    let last_col = rect.col() + (rect.width() - 1);
    let last_row = rect.row() + (rect.height() - 1);
    let (c0, c1) = (visible.col(), visible.col() + (visible.width() - 1));
    let (r0, r1) = (visible.row(), visible.row() + (visible.height() - 1));

    // Only visit visible edge cells so huge off-grid rects stay cheap.
    for col in c0..=c1 {
        for row in [rect.row(), last_row] {
            if (r0..=r1).contains(&row) {
                let p = piece(
                    line,
                    pos(col, rect.col(), last_col),
                    pos(row, rect.row(), last_row),
                );
                put(grid, col, row, p, style);
            }
        }
    }
    for row in r0..=r1 {
        // Corners were already written by the horizontal pass.
        if row == rect.row() || row == last_row {
            continue;
        }
        for col in [rect.col(), last_col] {
            if (c0..=c1).contains(&col) {
                let p = piece(line, pos(col, rect.col(), last_col), Pos::Mid);
                put(grid, col, row, p, style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cell, WidgetId};

    fn rows(g: &CellGrid) -> Vec<String> {
        (0..g.rows())
            .map(|r| {
                (0..g.cols())
                    .map(|c| match g.get(c, r).map(|x| x.content) {
                        Some(CellContent::Glyph(ch)) => ch,
                        _ => '.',
                    })
                    .collect()
            })
            .collect()
    }

    fn grid(c: u32, r: u32) -> CellGrid {
        CellGrid::new(c, r).unwrap()
    }

    fn rect(c: u32, r: u32, w: u32, h: u32) -> GridRect {
        GridRect::new(c, r, w, h)
    }

    const ST: StyleId = StyleId::DEFAULT;

    #[test]
    fn simple_single_rectangle() {
        let mut g = grid(6, 4);
        draw_border(&mut g, rect(0, 0, 6, 4), BorderLine::Single, ST);
        assert_eq!(rows(&g), ["┌────┐", "│....│", "│....│", "└────┘"]);
    }

    #[test]
    fn simple_double_rectangle_with_offset() {
        let mut g = grid(6, 5);
        draw_border(&mut g, rect(1, 1, 4, 3), BorderLine::Double, ST);
        assert_eq!(rows(&g), ["......", ".╔══╗.", ".║..║.", ".╚══╝.", "......"]);
    }

    #[test]
    fn smallest_valid_rectangle() {
        let mut g = grid(2, 2);
        draw_border(&mut g, rect(0, 0, 2, 2), BorderLine::Single, ST);
        assert_eq!(rows(&g), ["┌┐", "└┘"]);
    }

    #[test]
    fn degenerate_rects_draw_nothing() {
        for (w, h) in [(0, 0), (0, 4), (4, 0), (1, 4), (4, 1), (1, 1)] {
            let mut g = grid(5, 5);
            draw_border(&mut g, rect(0, 0, w, h), BorderLine::Single, ST);
            assert_eq!(g, grid(5, 5), "{w}x{h}");
        }
    }

    #[test]
    fn clipped_at_grid_edges() {
        let mut g = grid(4, 3);
        draw_border(&mut g, rect(2, 1, 6, 6), BorderLine::Single, ST);
        assert_eq!(rows(&g), ["....", "..┌─", "..│."]);
        let mut g = grid(4, 3);
        draw_border(&mut g, rect(0, 0, 2, 9), BorderLine::Single, ST);
        assert_eq!(rows(&g), ["┌┐..", "││..", "││.."]);
    }

    #[test]
    fn far_corner_visible_only() {
        let mut g = grid(4, 3);
        // Bottom-right corner lands on (3, 2); the rest is off-grid to the left/top.
        let r = GridRect::new(0, 0, 4, 3);
        draw_border(&mut g, r, BorderLine::Double, ST);
        assert_eq!(g.get(3, 2).unwrap().content, CellContent::Glyph('╝'));
    }

    #[test]
    fn off_grid_and_huge_rects_are_safe() {
        let mut g = grid(4, 3);
        draw_border(&mut g, rect(10, 10, 5, 5), BorderLine::Single, ST);
        assert_eq!(g, grid(4, 3));
        draw_border(
            &mut g,
            rect(u32::MAX - 3, u32::MAX - 3, u32::MAX, u32::MAX),
            BorderLine::Single,
            ST,
        );
        assert_eq!(g, grid(4, 3));
        draw_border(
            &mut g,
            rect(0, 0, u32::MAX, u32::MAX),
            BorderLine::Single,
            ST,
        );
        assert_eq!(rows(&g)[0], "┌───");
        assert_eq!(rows(&g)[1], "│...");
        let mut z = grid(0, 0);
        draw_border(&mut z, rect(0, 0, 5, 5), BorderLine::Single, ST);
        assert!(z.cells().is_empty());
    }

    #[test]
    fn tee_junctions_between_adjacent_panes() {
        let mut g = grid(7, 4);
        draw_border(&mut g, rect(0, 0, 4, 4), BorderLine::Double, ST);
        draw_border(&mut g, rect(3, 0, 4, 4), BorderLine::Double, ST);
        assert_eq!(rows(&g), ["╔══╦══╗", "║..║..║", "║..║..║", "╚══╩══╝"]);
    }

    #[test]
    fn cross_junction_and_side_tees() {
        let mut g = grid(5, 5);
        for (c, r) in [(0, 0), (2, 0), (0, 2), (2, 2)] {
            draw_border(&mut g, rect(c, r, 3, 3), BorderLine::Single, ST);
        }
        assert_eq!(rows(&g), ["┌─┬─┐", "│.│.│", "├─┼─┤", "│.│.│", "└─┴─┘"]);
    }

    #[test]
    fn merging_is_order_independent() {
        let a = rect(0, 0, 4, 4);
        let b = rect(3, 0, 4, 4);
        let mut g1 = grid(7, 4);
        draw_border(&mut g1, a, BorderLine::Single, ST);
        draw_border(&mut g1, b, BorderLine::Single, ST);
        let mut g2 = grid(7, 4);
        draw_border(&mut g2, b, BorderLine::Single, ST);
        draw_border(&mut g2, a, BorderLine::Single, ST);
        assert_eq!(g1, g2);
    }

    #[test]
    fn redrawing_is_idempotent() {
        let mut g = grid(5, 4);
        draw_border(&mut g, rect(0, 0, 5, 4), BorderLine::Double, ST);
        let once = g.clone();
        draw_border(&mut g, rect(0, 0, 5, 4), BorderLine::Double, ST);
        assert_eq!(g, once);
    }

    #[test]
    fn mixed_weights_promote_to_double() {
        let mut g = grid(5, 3);
        draw_border(&mut g, rect(0, 0, 5, 3), BorderLine::Double, ST);
        draw_border(&mut g, rect(2, 0, 3, 3), BorderLine::Single, ST);
        assert_eq!(rows(&g), ["╔═╦═╗", "║.│.║", "╚═╩═╝"]);
    }

    #[test]
    fn non_border_glyph_is_overwritten() {
        let mut g = grid(3, 3);
        g.get_mut(0, 0).unwrap().content = CellContent::Glyph('x');
        draw_border(&mut g, rect(0, 0, 3, 3), BorderLine::Single, ST);
        assert_eq!(g.get(0, 0).unwrap().content, CellContent::Glyph('┌'));
    }

    #[test]
    fn only_border_cells_change_and_widget_is_cleared() {
        let styled = Cell {
            content: CellContent::Glyph('Z'),
            style: StyleId::new(7),
            widget_id: Some(WidgetId::new(3)),
        };
        let mut g = grid(4, 4);
        for r in 0..4 {
            for c in 0..4 {
                *g.get_mut(c, r).unwrap() = styled;
            }
        }
        draw_border(
            &mut g,
            rect(0, 0, 4, 4),
            BorderLine::Single,
            StyleId::new(2),
        );
        // Interior untouched.
        for (c, r) in [(1, 1), (2, 1), (1, 2), (2, 2)] {
            assert_eq!(g.get(c, r), Some(&styled));
        }
        // Border cells carry the caller's style and no widget.
        let corner = g.get(0, 0).unwrap();
        assert_eq!(corner.style, StyleId::new(2));
        assert_eq!(corner.widget_id, None);
        assert_eq!(corner.content, CellContent::Glyph('┌'));
    }

    #[test]
    fn glyph_table_is_consistent() {
        for (c, a) in GLYPHS {
            assert_eq!(decode(c), Some(a));
            assert_eq!(encode(a), Some(c));
            assert!(a.0.iter().filter(|x| **x != Arm::None).count() >= 2);
        }
        // N/E/S/W constants index as documented.
        assert_eq!(decode('┌').unwrap().0[E], Arm::Single);
        assert_eq!(decode('┌').unwrap().0[S], Arm::Single);
        assert_eq!(decode('┌').unwrap().0[N], Arm::None);
        assert_eq!(decode('┌').unwrap().0[W], Arm::None);
    }
}
