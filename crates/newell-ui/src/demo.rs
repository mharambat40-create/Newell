// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Deterministic grid used to validate the M2.4 renderer. Not the application shell (M2.13).

use crate::{BorderLine, CellContent, CellGrid, GridRect, StyleId, UiError, draw_border};

const COLS: u32 = 48;
const ROWS: u32 = 17;

const DEFAULT: StyleId = StyleId::DEFAULT;
const BRIGHT: StyleId = StyleId::new(1);
const DIM: StyleId = StyleId::new(2);
const ACCENT: StyleId = StyleId::new(3);

fn put(grid: &mut CellGrid, col: u32, row: u32, text: &str, style: StyleId) {
    for (offset, ch) in (0..).zip(text.chars()) {
        if let Some(cell) = grid.get_mut(col + offset, row) {
            cell.content = CellContent::Glyph(ch);
            cell.style = style;
        }
    }
}

/// Text, punctuation, a single and a double box, and a 2x2 split of each border family
/// (which produces every junction glyph).
///
/// # Errors
/// Only if the fixed grid size were rejected by [`CellGrid::new`].
pub fn demo_grid() -> Result<CellGrid, UiError> {
    let mut grid = CellGrid::new(COLS, ROWS)?;
    put(&mut grid, 0, 0, "Newell", BRIGHT);
    put(&mut grid, 0, 1, "ABC abc 0123", DEFAULT);

    draw_border(
        &mut grid,
        GridRect::new(0, 2, 6, 3),
        BorderLine::Single,
        DEFAULT,
    );
    put(&mut grid, 1, 3, "Text", ACCENT);
    draw_border(
        &mut grid,
        GridRect::new(0, 5, 6, 3),
        BorderLine::Double,
        DEFAULT,
    );
    put(&mut grid, 1, 6, "Text", ACCENT);

    for (col, line) in [(12, BorderLine::Single), (26, BorderLine::Double)] {
        for (dc, dr) in [(0, 0), (5, 0), (0, 3), (5, 3)] {
            draw_border(
                &mut grid,
                GridRect::new(col + dc, 2 + dr, 6, 4),
                line,
                DEFAULT,
            );
        }
    }

    put(&mut grid, 0, 10, "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~", DIM);
    put(
        &mut grid,
        0,
        11,
        "The quick brown fox jumps over the lazy dog",
        DEFAULT,
    );
    put(
        &mut grid,
        0,
        12,
        "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG",
        DEFAULT,
    );
    put(&mut grid, 0, 13, "gjpqy bdfhklt ()[]{}|;,", DEFAULT);
    put(&mut grid, 0, 14, "─│┌┐└┘┼├┤┬┴ ════════════", DEFAULT);
    put(&mut grid, 0, 15, "═║╔╗╚╝╬╠╣╦╩ ────────────", DEFAULT);
    put(&mut grid, 0, 16, "║════════╬════════║", DEFAULT);
    Ok(grid)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(grid: &CellGrid, col: u32, row: u32) -> Option<char> {
        match grid.get(col, row)?.content {
            CellContent::Glyph(ch) => Some(ch),
            CellContent::Empty => None,
        }
    }

    #[test]
    fn is_deterministic() {
        assert_eq!(demo_grid().unwrap(), demo_grid().unwrap());
        assert_eq!(demo_grid().unwrap().cells().len(), (COLS * ROWS) as usize);
    }

    #[test]
    fn contains_requested_samples() {
        let g = demo_grid().unwrap();
        let row = |r: u32, n: u32| (0..n).filter_map(|c| glyph(&g, c, r)).collect::<String>();
        assert_eq!(row(0, 6), "Newell");
        assert_eq!(row(1, 12), "ABC abc 0123");
        assert_eq!(row(2, 6), "┌────┐");
        assert_eq!(row(3, 6), "│Text│");
        assert_eq!(row(4, 6), "└────┘");
        assert_eq!(row(5, 6), "╔════╗");
        assert_eq!(row(6, 6), "║Text║");
        assert_eq!(row(7, 6), "╚════╝");
    }

    #[test]
    fn splits_produce_junctions() {
        let g = demo_grid().unwrap();
        assert_eq!(glyph(&g, 17, 5), Some('┼'));
        assert_eq!(glyph(&g, 31, 5), Some('╬'));
    }
}
