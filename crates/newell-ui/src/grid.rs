// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Flat row-major cell grid.

use crate::{Cell, UiError};

/// Upper bound on the number of cells in a grid.
///
/// 4 194 304 (2^22) cells covers an 8K (7680x4320) window at a 3x5 px cell
/// (about 2.2M cells) with headroom, while bounding a grid to tens of MiB
/// rather than allowing pathological allocations.
pub const MAX_CELLS: u32 = 1 << 22;

/// Row-major grid of cells stored in one flat `Vec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellGrid {
    cols: u32,
    rows: u32,
    cells: Vec<Cell>,
}

impl CellGrid {
    /// Creates a grid of default cells. Zero-sized dimensions are valid.
    ///
    /// # Errors
    /// [`UiError::CellCountOverflow`] if the cell count overflows, and
    /// [`UiError::TooManyCells`] if it exceeds [`MAX_CELLS`]. Nothing is
    /// allocated before validation.
    pub fn new(cols: u32, rows: u32) -> Result<Self, UiError> {
        let count = cols
            .checked_mul(rows)
            .ok_or(UiError::CellCountOverflow { cols, rows })?;
        if count > MAX_CELLS {
            return Err(UiError::TooManyCells {
                cols,
                rows,
                max: MAX_CELLS,
            });
        }
        let len = usize::try_from(count).map_err(|_| UiError::CellCountOverflow { cols, rows })?;
        Ok(Self {
            cols,
            rows,
            cells: vec![Cell::default(); len],
        })
    }

    #[must_use]
    pub const fn cols(&self) -> u32 {
        self.cols
    }

    #[must_use]
    pub const fn rows(&self) -> u32 {
        self.rows
    }

    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Flat row-major index of `(col, row)`, or `None` if out of bounds.
    #[must_use]
    pub fn index(&self, col: u32, row: u32) -> Option<usize> {
        if col >= self.cols || row >= self.rows {
            return None;
        }
        // In bounds, so row * cols + col < cells.len() <= MAX_CELLS.
        let idx = u64::from(row) * u64::from(self.cols) + u64::from(col);
        usize::try_from(idx).ok()
    }

    #[must_use]
    pub fn get(&self, col: u32, row: u32) -> Option<&Cell> {
        self.index(col, row).and_then(|i| self.cells.get(i))
    }

    #[must_use]
    pub fn get_mut(&mut self, col: u32, row: u32) -> Option<&mut Cell> {
        let i = self.index(col, row)?;
        self.cells.get_mut(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CellContent, StyleId, WidgetId};

    #[test]
    fn normal_grid() {
        let g = CellGrid::new(4, 3).unwrap();
        assert_eq!((g.cols(), g.rows()), (4, 3));
        assert_eq!(g.cells().len(), 12);
    }

    #[test]
    fn zero_sized_grids_are_valid_and_unindexable() {
        for (c, r) in [(0, 0), (0, 5), (5, 0)] {
            let mut g = CellGrid::new(c, r).unwrap();
            assert!(g.cells().is_empty());
            assert_eq!(g.index(0, 0), None);
            assert!(g.get(0, 0).is_none());
            assert!(g.get_mut(0, 0).is_none());
        }
    }

    #[test]
    fn indexing_known_positions() {
        let g = CellGrid::new(4, 3).unwrap();
        assert_eq!(g.index(0, 0), Some(0));
        assert_eq!(g.index(3, 0), Some(3));
        assert_eq!(g.index(0, 1), Some(4));
        assert_eq!(g.index(2, 1), Some(6));
        assert_eq!(g.index(3, 2), Some(11));
    }

    #[test]
    fn out_of_bounds_is_none() {
        let g = CellGrid::new(4, 3).unwrap();
        assert_eq!(g.index(4, 0), None);
        assert_eq!(g.index(0, 3), None);
        assert_eq!(g.index(u32::MAX, u32::MAX), None);
        assert!(g.get(4, 2).is_none());
    }

    #[test]
    fn get_mut_writes_to_row_major_slot() {
        let mut g = CellGrid::new(4, 3).unwrap();
        let widget = Some(WidgetId::new(5));
        if let Some(c) = g.get_mut(2, 1) {
            c.content = CellContent::Glyph('x');
            c.widget_id = widget;
        }
        assert_eq!(g.cells()[6].content, CellContent::Glyph('x'));
        assert_eq!(g.get(2, 1).and_then(|c| c.widget_id), widget);
        assert_eq!(g.get(1, 2), Some(&Cell::default()));
    }

    #[test]
    fn new_cells_are_default() {
        let g = CellGrid::new(3, 3).unwrap();
        assert!(g.cells().iter().all(|c| c.content == CellContent::Empty
            && c.style == StyleId::DEFAULT
            && c.widget_id.is_none()));
    }

    #[test]
    fn multiplication_overflow_is_error() {
        assert_eq!(
            CellGrid::new(u32::MAX, u32::MAX),
            Err(UiError::CellCountOverflow {
                cols: u32::MAX,
                rows: u32::MAX
            })
        );
        assert!(matches!(
            CellGrid::new(65_536, 65_536),
            Err(UiError::CellCountOverflow { .. })
        ));
    }

    #[test]
    fn above_max_cells_is_error() {
        assert!(matches!(
            CellGrid::new(MAX_CELLS, 2),
            Err(UiError::TooManyCells { .. })
        ));
        assert!(matches!(
            CellGrid::new(MAX_CELLS + 1, 1),
            Err(UiError::TooManyCells { .. })
        ));
        assert!(matches!(
            CellGrid::new(1, MAX_CELLS + 1),
            Err(UiError::TooManyCells { .. })
        ));
    }

    #[test]
    fn extreme_dimension_with_zero_does_not_fail() {
        assert!(CellGrid::new(u32::MAX, 0).is_ok());
        assert!(CellGrid::new(0, u32::MAX).is_ok());
    }

    #[test]
    fn exactly_max_cells_is_accepted() {
        let g = CellGrid::new(2048, 2048).unwrap();
        assert_eq!(g.cells().len(), MAX_CELLS as usize);
    }
}
