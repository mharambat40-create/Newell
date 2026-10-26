// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! ASCII hover highlight (M2.25, `docs/ui_design/README_UI.md` §5.12): the composition-time
//! substitution of the border glyphs of one hovered row.
//!
//! Only glyphs change. Cell count, text, `widget_id`, style and `shared_border` of every cell
//! are untouched, so a hovered grid has the same hit map as the normal one.

use std::ops::Range;

use crate::{CellContent, CellGrid};

/// The border slot found left of a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    /// A `│` at this column: becomes `╞`.
    Pipe(u32),
    /// A `╰╴` pair whose `╰` is at this column: becomes `╘` and a space.
    Branch(u32),
}

fn glyph(grid: &CellGrid, col: u32, row: u32) -> Option<char> {
    match grid.get(col, row)?.content {
        CellContent::Glyph(ch) => Some(ch),
        CellContent::Empty => None,
    }
}

fn set_content(grid: &mut CellGrid, col: u32, row: u32, content: CellContent) {
    if let Some(cell) = grid.get_mut(col, row) {
        cell.content = content;
    }
}

/// The innermost `│` or `╰╴` pair left of `start`, scanning no further left than `from`.
fn left_slot(grid: &CellGrid, row: u32, from: u32, start: u32) -> Option<Slot> {
    let mut col = start;
    while col > from {
        col -= 1;
        match glyph(grid, col, row) {
            Some('│') => return Some(Slot::Pipe(col)),
            Some('╴') if col > from && glyph(grid, col - 1, row) == Some('╰') => {
                return Some(Slot::Branch(col - 1));
            }
            _ => {}
        }
    }
    None
}

/// Substitutes the hover slots of the target whose columns are `span` on `row` (`README_UI.md`
/// §5.12): the innermost left slot, searched from `span.start - 1` down to `from`, and the `│`
/// at `span.end`. A slot holding any other glyph is left alone, and a missing slot is not
/// created. Returns `true` if at least one slot was substituted.
pub(crate) fn hover_row(grid: &mut CellGrid, row: u32, from: u32, span: Range<u32>) -> bool {
    let mut lit = false;
    match left_slot(grid, row, from, span.start) {
        Some(Slot::Pipe(col)) => {
            set_content(grid, col, row, CellContent::Glyph('╞'));
            lit = true;
        }
        Some(Slot::Branch(col)) => {
            set_content(grid, col, row, CellContent::Glyph('╘'));
            set_content(grid, col + 1, row, CellContent::Empty);
            lit = true;
        }
        None => {}
    }
    if glyph(grid, span.end, row) == Some('│') {
        set_content(grid, span.end, row, CellContent::Glyph('╡'));
        lit = true;
    }
    lit
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cell, SharedBorder, StyleId, WidgetId};

    /// One row holding `text`, a space being an empty cell.
    fn line(text: &str) -> CellGrid {
        let mut g = CellGrid::new(u32::try_from(text.chars().count()).unwrap(), 1).unwrap();
        for (col, ch) in text.chars().enumerate() {
            let content = if ch == ' ' {
                CellContent::Empty
            } else {
                CellContent::Glyph(ch)
            };
            g.get_mut(u32::try_from(col).unwrap(), 0).unwrap().content = content;
        }
        g
    }

    /// The row as text, empty cells as spaces.
    fn text(g: &CellGrid) -> String {
        (0..g.cols())
            .map(|c| match g.get(c, 0).unwrap().content {
                CellContent::Glyph(ch) => ch,
                CellContent::Empty => ' ',
            })
            .collect()
    }

    /// Columns of `needle` in the first row of `g` (the needle is ASCII-free of the cases below).
    fn col_of(g: &CellGrid, needle: char) -> u32 {
        (0..g.cols())
            .find(|&c| glyph(g, c, 0) == Some(needle))
            .unwrap()
    }

    #[test]
    fn menu_row_substitutes_frame_both_sides() {
        let mut g = line("│ Bridging Curve  │");
        let lit = hover_row(&mut g, 0, 0, 1..18);
        assert!(lit);
        assert_eq!(text(&g), "╞ Bridging Curve  ╡");
    }

    #[test]
    fn parameter_row_uses_innermost_branch() {
        let mut g = line("│ │ Along     [Select…       ]│");
        assert!(hover_row(&mut g, 0, 0, 14..30));
        assert_eq!(text(&g), "│ ╞ Along     [Select…       ]╡");
    }

    #[test]
    fn checkbox_row_turns_connector_into_corner() {
        let mut g = line("│ ╰╴□ End Offset              │");
        assert!(hover_row(&mut g, 0, 0, 4..30));
        assert_eq!(text(&g), "│ ╘ □ End Offset              ╡");
    }

    #[test]
    fn button_slots_are_the_neighbours_of_the_button() {
        let mut ok = line("│      OK      │    Cancel    │");
        assert!(hover_row(&mut ok, 0, 0, 1..15));
        assert_eq!(text(&ok), "╞      OK      ╡    Cancel    │");

        let mut cancel = line("│      OK      │    Cancel    │");
        assert!(hover_row(&mut cancel, 0, 0, 16..30));
        assert_eq!(text(&cancel), "│      OK      ╞    Cancel    ╡");
    }

    #[test]
    fn nested_connector_only_changes_the_innermost_pair() {
        let mut g = line("│ │ ╰╴Dist  [+|0.0 mm       ☰]│");
        assert!(hover_row(&mut g, 0, 0, 6..30));
        assert_eq!(text(&g), "│ │ ╘ Dist  [+|0.0 mm       ☰]╡");
    }

    #[test]
    fn other_glyphs_are_never_substituted() {
        let mut g = line("║ Along     ║");
        assert!(!hover_row(&mut g, 0, 0, 2..10));
        assert_eq!(text(&g), "║ Along     ║");
    }

    #[test]
    fn target_without_slots_is_not_lit() {
        let mut g = line("  Label  ");
        assert!(!hover_row(&mut g, 0, 0, 2..7));
        assert_eq!(text(&g), "  Label  ");
    }

    #[test]
    fn substitution_keeps_identity_of_every_cell() {
        let mut g = line("│ Bridging │");
        let id = WidgetId::new(7);
        let border = SharedBorder {
            axis: crate::BorderAxis::Vertical,
            low: Some(id),
            high: None,
        };
        let style = StyleId::new(3);
        *g.get_mut(0, 0).unwrap() = Cell {
            ink: crate::InkRole::Text,
            content: CellContent::Glyph('│'),
            style,
            widget_id: Some(id),
            shared_border: Some(border),
        };
        let before = g.clone();
        hover_row(&mut g, 0, 0, 1..11);
        let after = &g;
        assert_eq!(after.get(0, 0).unwrap().content, CellContent::Glyph('╞'));
        assert_eq!(after.get(0, 0).unwrap().widget_id, Some(id));
        assert_eq!(after.get(0, 0).unwrap().style, style);
        assert_eq!(after.get(0, 0).unwrap().shared_border, Some(border));
        for col in 1..g.cols() {
            let (a, b) = (before.get(col, 0).unwrap(), after.get(col, 0).unwrap());
            assert_eq!(a.widget_id, b.widget_id, "col {col}");
            assert_eq!(a.style, b.style, "col {col}");
            assert_eq!(a.shared_border, b.shared_border, "col {col}");
        }
        assert_eq!(col_of(&g, '╡'), 11);
    }

    #[test]
    fn right_slot_must_be_a_pipe_and_may_be_off_grid() {
        let mut g = line("│ Ab");
        assert!(hover_row(&mut g, 0, 0, 1..3));
        assert_eq!(text(&g), "╞ Ab");
        let mut past = line("│ Ab");
        assert!(hover_row(&mut past, 0, 0, 1..9));
        assert_eq!(text(&past), "╞ Ab");
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;
    use crate::{BorderAxis, InkRole, SharedBorder, StyleId, WidgetId};
    #[test]
    fn replacement_glyphs_and_blank_keep_every_replaced_cells_metadata() {
        for ink in [InkRole::Text, InkRole::Contour, InkRole::Action] {
            for (before, after, start) in [("│ value│", "╞ value╡", 1), ("╰╴value│", "╘ value╡", 2)]
            {
                let mut g = CellGrid::new(8, 1).unwrap();
                for (x, ch) in before.chars().enumerate() {
                    let c = g.get_mut(x as u32, 0).unwrap();
                    c.content = if ch == ' ' {
                        CellContent::Empty
                    } else {
                        CellContent::Glyph(ch)
                    };
                    c.ink = ink;
                    c.style = StyleId::new(2);
                    c.widget_id = Some(WidgetId::new(x as u32));
                    c.shared_border = Some(SharedBorder {
                        axis: BorderAxis::Vertical,
                        low: c.widget_id,
                        high: None,
                    });
                }
                let old = g.clone();
                assert!(hover_row(&mut g, 0, 0, start..7));
                for ((a, b), ch) in old.cells().iter().zip(g.cells()).zip(after.chars()) {
                    assert_eq!(
                        b.content,
                        if ch == ' ' {
                            CellContent::Empty
                        } else {
                            CellContent::Glyph(ch)
                        }
                    );
                    assert_eq!(
                        (a.ink, a.style, a.widget_id, a.shared_border),
                        (b.ink, b.style, b.widget_id, b.shared_border)
                    );
                }
            }
        }
    }
}
