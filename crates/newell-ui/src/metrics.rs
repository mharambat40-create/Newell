// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Physical cell metrics.

use crate::{MAX_CELLS, UiError};

/// Validated pixel size of one grid cell. Both dimensions are non-zero.
///
/// The 3:5 width:height ratio is a design target and is not enforced here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridMetrics {
    cell_width_px: u32,
    cell_height_px: u32,
}

impl GridMetrics {
    /// # Errors
    /// Returns [`UiError::InvalidMetrics`] if either dimension is zero.
    pub const fn new(cell_width_px: u32, cell_height_px: u32) -> Result<Self, UiError> {
        if cell_width_px == 0 || cell_height_px == 0 {
            return Err(UiError::InvalidMetrics {
                width_px: cell_width_px,
                height_px: cell_height_px,
            });
        }
        Ok(Self {
            cell_width_px,
            cell_height_px,
        })
    }

    /// Whole cells `(cols, rows)` that fit a `width_px` x `height_px` window: floor division,
    /// so a partial cell is never counted. The row count is reduced if the grid would pass
    /// [`MAX_CELLS`]. A zero dimension gives a zero-sized grid.
    #[must_use]
    pub const fn grid_size(&self, width_px: u32, height_px: u32) -> (u32, u32) {
        let cols = width_px / self.cell_width_px;
        let rows = height_px / self.cell_height_px;
        if cols > 0 && rows > MAX_CELLS / cols {
            (cols, MAX_CELLS / cols)
        } else {
            (cols, rows)
        }
    }

    #[must_use]
    pub const fn cell_width_px(&self) -> u32 {
        self.cell_width_px
    }

    #[must_use]
    pub const fn cell_height_px(&self) -> u32 {
        self.cell_height_px
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_metrics() {
        let m = GridMetrics::new(9, 15).unwrap();
        assert_eq!(m.cell_width_px(), 9);
        assert_eq!(m.cell_height_px(), 15);
    }

    #[test]
    fn zero_width_rejected() {
        assert_eq!(
            GridMetrics::new(0, 15),
            Err(UiError::InvalidMetrics {
                width_px: 0,
                height_px: 15
            })
        );
    }

    #[test]
    fn zero_height_rejected() {
        assert!(matches!(
            GridMetrics::new(9, 0),
            Err(UiError::InvalidMetrics { .. })
        ));
    }

    #[test]
    fn both_zero_rejected() {
        assert!(matches!(
            GridMetrics::new(0, 0),
            Err(UiError::InvalidMetrics { .. })
        ));
    }

    #[test]
    fn grid_size_is_whole_cells() {
        let m = GridMetrics::new(8, 20).unwrap();
        assert_eq!(m.grid_size(80, 40), (10, 2));
        assert_eq!(m.grid_size(87, 59), (10, 2), "partial cells are dropped");
        assert_eq!(m.grid_size(7, 19), (0, 0));
        assert_eq!(m.grid_size(0, 0), (0, 0));
        assert_eq!(m.grid_size(0, 400), (0, 20));
        assert_eq!(m.grid_size(800, 0), (100, 0));
    }

    #[test]
    fn grid_size_never_exceeds_the_cell_cap() {
        let m = GridMetrics::new(1, 1).unwrap();
        let (cols, rows) = m.grid_size(u32::MAX, u32::MAX);
        assert!(u64::from(cols) * u64::from(rows) <= u64::from(MAX_CELLS));
        assert!(crate::CellGrid::new(cols, rows).is_ok());
        assert_eq!(m.grid_size(u32::MAX, u32::MAX), (cols, rows));
    }

    #[test]
    fn non_3_5_ratio_accepted() {
        assert!(GridMetrics::new(1, 1).is_ok());
    }
}
