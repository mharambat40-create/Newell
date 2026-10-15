// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Physical cell metrics.

use crate::{MAX_CELLS, UiError};

/// Default cell scale `k`: 20 x 45 physical pixels (`ARCHITECTURE.md` §3.1).
pub const DEFAULT_K: u32 = 5;

/// Cell width in pixels per unit of `k`.
const WIDTH_PER_K: u32 = 4;
/// Cell height in pixels per unit of `k`.
const HEIGHT_PER_K: u32 = 9;

/// Validated pixel size of one grid cell. Both dimensions are non-zero.
///
/// Production metrics come from [`GridMetrics::from_k`]: `W = 4k`, `H = 9k`, exactly 4:9
/// (`ARCHITECTURE.md` §3.1). [`GridMetrics::new`] accepts any non-zero size so that pure layout
/// and hit-testing arithmetic can be tested with other ratios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridMetrics {
    cell_width_px: u32,
    cell_height_px: u32,
}

impl GridMetrics {
    /// The 4:9 cell of scale `k`: `4k` x `9k` physical pixels.
    ///
    /// # Errors
    /// Returns [`UiError::InvalidMetrics`] if `k` is zero or the size overflows `u32`.
    pub const fn from_k(k: u32) -> Result<Self, UiError> {
        match (k.checked_mul(WIDTH_PER_K), k.checked_mul(HEIGHT_PER_K)) {
            (Some(w), Some(h)) => Self::new(w, h),
            _ => Err(UiError::InvalidMetrics {
                width_px: 0,
                height_px: 0,
            }),
        }
    }

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
    fn any_ratio_accepted_by_new() {
        assert!(GridMetrics::new(1, 1).is_ok());
    }

    #[test]
    fn from_k_is_exactly_four_by_nine() {
        for (k, w, h) in [
            (3, 12, 27),
            (4, 16, 36),
            (5, 20, 45),
            (6, 24, 54),
            (7, 28, 63),
        ] {
            let m = GridMetrics::from_k(k).unwrap();
            assert_eq!((m.cell_width_px(), m.cell_height_px()), (w, h), "k = {k}");
            assert_eq!(m.cell_width_px() * 9, m.cell_height_px() * 4);
        }
        assert_eq!(GridMetrics::from_k(DEFAULT_K), GridMetrics::new(20, 45));
    }

    #[test]
    fn from_k_rejects_zero_and_overflow() {
        assert!(GridMetrics::from_k(0).is_err());
        assert!(GridMetrics::from_k(u32::MAX).is_err());
    }
}
