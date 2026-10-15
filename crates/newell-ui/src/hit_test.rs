// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! O(1) pixel-to-cell hit-testing and shared-border routing (ARCHITECTURE.md §3.4).
//!
//! Flow: pixel -> `(col, row)` by integer division -> `CellGrid` lookup ->
//! `Cell::widget_id`, or sub-cell routing if the cell is a [`SharedBorder`].
//! No widget or bounding-box iteration takes place.

use crate::{CellGrid, GridMetrics, WidgetId};

/// Pointer position in physical pixels, origin at the top-left of the grid.
///
/// Callers convert from window events (and drop negative coordinates, which are
/// outside the grid by definition).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiPoint {
    pub x: u32,
    pub y: u32,
}

/// Which way a shared separator cell runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderAxis {
    /// Vertical separator (e.g. `│`): neighbours are left (`low`) and right (`high`).
    Vertical,
    /// Horizontal separator (e.g. `─`): neighbours are above (`low`) and below (`high`).
    Horizontal,
}

/// Routing targets of a cell that visually separates two adjacent regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedBorder {
    pub axis: BorderAxis,
    /// Owner of the left / upper half of the cell.
    pub low: Option<WidgetId>,
    /// Owner of the right / lower half of the cell.
    pub high: Option<WidgetId>,
}

/// Result of a hit-test inside the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitTestResult {
    pub col: u32,
    pub row: u32,
    /// Resolved widget, or `None` if the cell (or routed half) has no owner.
    pub widget_id: Option<WidgetId>,
}

/// Picks the owner of a shared-border cell from the pixel offset inside it.
///
/// Rule: along the border's axis, the offset belongs to `low` iff
/// `local < ceil(cell_size / 2)`, otherwise to `high`. For an even cell size the
/// exact midpoint pixel (and the tie) therefore goes to `high` (right/lower);
/// for an odd size the single central pixel goes to `low` (left/upper).
/// Offsets are taken modulo the cell size, so any pixel coordinate is accepted.
#[must_use]
pub(crate) const fn route_shared_border(
    border: &SharedBorder,
    position: UiPoint,
    metrics: GridMetrics,
) -> Option<WidgetId> {
    let (local, size) = match border.axis {
        BorderAxis::Vertical => (
            position.x % metrics.cell_width_px(),
            metrics.cell_width_px(),
        ),
        BorderAxis::Horizontal => (
            position.y % metrics.cell_height_px(),
            metrics.cell_height_px(),
        ),
    };
    if local < size.div_ceil(2) {
        border.low
    } else {
        border.high
    }
}

/// Maps a pixel to a grid cell and its widget in O(1).
///
/// Cell `(col, row)` covers pixels `[col*w, (col+1)*w) x [row*h, (row+1)*h)`
/// (half-open). Pixels at or beyond `cols*w` / `rows*h`, and every pixel of a
/// zero-sized grid, return `None`. `GridMetrics` guarantees non-zero divisors.
#[must_use]
pub fn hit_test(position: UiPoint, metrics: GridMetrics, grid: &CellGrid) -> Option<HitTestResult> {
    let col = position.x / metrics.cell_width_px();
    let row = position.y / metrics.cell_height_px();
    let cell = grid.get(col, row)?;
    let widget_id = match &cell.shared_border {
        Some(border) => route_shared_border(border, position, metrics),
        None => cell.widget_id,
    };
    Some(HitTestResult {
        col,
        row,
        widget_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: u32 = 6;
    const H: u32 = 10;

    fn m() -> GridMetrics {
        GridMetrics::new(W, H).unwrap()
    }

    fn p(x: u32, y: u32) -> UiPoint {
        UiPoint { x, y }
    }

    fn id(v: u32) -> Option<WidgetId> {
        Some(WidgetId::new(v))
    }

    /// 4x3 grid: (1,1) = widget 7, (2,1) = vertical border 1|2,
    /// (0,2) = horizontal border 3/4, (3,2) = vertical border with a `None` side.
    fn grid() -> CellGrid {
        let mut g = CellGrid::new(4, 3).unwrap();
        g.get_mut(1, 1).unwrap().widget_id = id(7);
        let c = g.get_mut(2, 1).unwrap();
        c.widget_id = id(99); // must be ignored: shared_border wins
        c.shared_border = Some(SharedBorder {
            axis: BorderAxis::Vertical,
            low: id(1),
            high: id(2),
        });
        g.get_mut(0, 2).unwrap().shared_border = Some(SharedBorder {
            axis: BorderAxis::Horizontal,
            low: id(3),
            high: id(4),
        });
        g.get_mut(3, 2).unwrap().shared_border = Some(SharedBorder {
            axis: BorderAxis::Vertical,
            low: id(5),
            high: None,
        });
        g
    }

    fn widget(x: u32, y: u32) -> Option<WidgetId> {
        hit_test(p(x, y), m(), &grid()).unwrap().widget_id
    }

    /// For every cell of a grid, the pixels at its four corners and its centre map back to that
    /// cell, with the 4:9 metrics every `k` gives (the same W and H the renderer draws with).
    #[test]
    fn corners_and_centre_round_trip_at_every_k() {
        let g = CellGrid::new(7, 5).unwrap();
        for k in 2..=7 {
            let m = GridMetrics::from_k(k).unwrap();
            let (w, h) = (m.cell_width_px(), m.cell_height_px());
            for (col, row) in (0..g.rows()).flat_map(|r| (0..g.cols()).map(move |c| (c, r))) {
                let (x0, y0) = (col * w, row * h);
                for (x, y) in [
                    (x0, y0),
                    (x0 + w - 1, y0),
                    (x0, y0 + h - 1),
                    (x0 + w - 1, y0 + h - 1),
                    (x0 + w / 2, y0 + h / 2),
                ] {
                    let hit = hit_test(p(x, y), m, &g).unwrap();
                    assert_eq!((hit.col, hit.row), (col, row), "k = {k} at ({x}, {y})");
                }
            }
            assert!(hit_test(p(g.cols() * w, 0), m, &g).is_none());
            assert!(hit_test(p(0, g.rows() * h), m, &g).is_none());
        }
    }

    #[test]
    fn first_pixel_of_first_cell() {
        let r = hit_test(p(0, 0), m(), &grid()).unwrap();
        assert_eq!((r.col, r.row, r.widget_id), (0, 0, None));
    }

    #[test]
    fn last_pixel_inside_cell() {
        let r = hit_test(p(W - 1, H - 1), m(), &grid()).unwrap();
        assert_eq!((r.col, r.row), (0, 0));
    }

    #[test]
    fn exact_horizontal_boundary_moves_to_next_column() {
        let r = hit_test(p(W, 0), m(), &grid()).unwrap();
        assert_eq!((r.col, r.row), (1, 0));
    }

    #[test]
    fn exact_vertical_boundary_moves_to_next_row() {
        let r = hit_test(p(0, H), m(), &grid()).unwrap();
        assert_eq!((r.col, r.row), (0, 1));
    }

    #[test]
    fn last_valid_grid_pixel() {
        let r = hit_test(p(4 * W - 1, 3 * H - 1), m(), &grid()).unwrap();
        assert_eq!((r.col, r.row), (3, 2));
    }

    #[test]
    fn one_pixel_outside_right_edge() {
        assert_eq!(hit_test(p(4 * W, 0), m(), &grid()), None);
    }

    #[test]
    fn one_pixel_outside_bottom_edge() {
        assert_eq!(hit_test(p(0, 3 * H), m(), &grid()), None);
    }

    #[test]
    fn zero_sized_grids_never_hit() {
        for (c, r) in [(0, 0), (0, 4), (4, 0)] {
            let g = CellGrid::new(c, r).unwrap();
            assert_eq!(hit_test(p(0, 0), m(), &g), None);
            assert_eq!(hit_test(p(3, 3), m(), &g), None);
        }
    }

    #[test]
    fn zero_metrics_unreachable_through_validated_constructor() {
        // hit_test only accepts GridMetrics, whose constructor rejects zero,
        // so the divisions above can never divide by zero.
        assert!(GridMetrics::new(0, H).is_err());
        assert!(GridMetrics::new(W, 0).is_err());
        assert!(GridMetrics::new(0, 0).is_err());
    }

    #[test]
    fn normal_widget_hit_anywhere_in_cell() {
        for (x, y) in [
            (W, H),
            (2 * W - 1, H),
            (W, 2 * H - 1),
            (2 * W - 1, 2 * H - 1),
        ] {
            assert_eq!(widget(x, y), id(7));
        }
    }

    #[test]
    fn cell_without_widget_is_hit_with_none() {
        let r = hit_test(p(3 * W, 0), m(), &grid()).unwrap();
        assert_eq!((r.col, r.row, r.widget_id), (3, 0, None));
    }

    #[test]
    fn shared_vertical_border_routes_left_and_right() {
        let x0 = 2 * W;
        let y = H + 3;
        assert_eq!(widget(x0, y), id(1));
        assert_eq!(widget(x0 + 2, y), id(1));
        assert_eq!(widget(x0 + 3, y), id(2)); // even width: midpoint -> high
        assert_eq!(widget(x0 + W - 1, y), id(2));
        // y does not influence a vertical border
        assert_eq!(widget(x0, H), id(1));
        assert_eq!(widget(x0, 2 * H - 1), id(1));
    }

    #[test]
    fn shared_horizontal_border_routes_upper_and_lower() {
        let y0 = 2 * H;
        assert_eq!(widget(2, y0), id(3));
        assert_eq!(widget(2, y0 + 4), id(3));
        assert_eq!(widget(2, y0 + 5), id(4)); // even height: midpoint -> high
        assert_eq!(widget(2, y0 + H - 1), id(4));
        // x does not influence a horizontal border
        assert_eq!(widget(0, y0 + 4), id(3));
        assert_eq!(widget(W - 1, y0 + 4), id(3));
    }

    #[test]
    fn tie_goes_to_high_for_even_and_low_for_odd_sizes() {
        let b = SharedBorder {
            axis: BorderAxis::Vertical,
            low: id(1),
            high: id(2),
        };
        let even = GridMetrics::new(4, 4).unwrap();
        assert_eq!(route_shared_border(&b, p(1, 0), even), id(1));
        assert_eq!(route_shared_border(&b, p(2, 0), even), id(2));
        let odd = GridMetrics::new(5, 5).unwrap();
        assert_eq!(route_shared_border(&b, p(2, 0), odd), id(1)); // centre pixel
        assert_eq!(route_shared_border(&b, p(3, 0), odd), id(2));
        let one = GridMetrics::new(1, 1).unwrap();
        assert_eq!(route_shared_border(&b, p(0, 0), one), id(1));
    }

    #[test]
    fn border_with_empty_side_routes_to_none() {
        let x0 = 3 * W;
        let y = 2 * H;
        assert_eq!(widget(x0, y), id(5));
        assert_eq!(widget(x0 + W - 1, y), None);
    }

    #[test]
    fn routing_is_modulo_periodic_and_deterministic() {
        let b = SharedBorder {
            axis: BorderAxis::Vertical,
            low: id(1),
            high: id(2),
        };
        for local in 0..W {
            let first = route_shared_border(&b, p(local, 0), m());
            for k in [1, 2, 1000, 700_000] {
                assert_eq!(route_shared_border(&b, p(local + k * W, 0), m()), first);
            }
            assert_eq!(first, route_shared_border(&b, p(local, 0), m()));
        }
    }

    #[test]
    fn extreme_pixel_values_do_not_overflow() {
        let g = grid();
        for pt in [p(u32::MAX, 0), p(0, u32::MAX), p(u32::MAX, u32::MAX)] {
            assert_eq!(hit_test(pt, m(), &g), None);
        }
        let big = GridMetrics::new(u32::MAX, u32::MAX).unwrap();
        let one = CellGrid::new(1, 1).unwrap();
        let r = hit_test(p(u32::MAX - 1, u32::MAX - 1), big, &one).unwrap();
        assert_eq!((r.col, r.row), (0, 0));
        assert_eq!(hit_test(p(u32::MAX, 0), big, &one), None); // col 1 of 1
        let b = SharedBorder {
            axis: BorderAxis::Horizontal,
            low: id(1),
            high: id(2),
        };
        assert_eq!(route_shared_border(&b, p(0, u32::MAX), big), id(1)); // MAX % MAX = 0
        assert_eq!(route_shared_border(&b, p(0, u32::MAX - 1), big), id(2));
        assert_eq!(route_shared_border(&b, p(0, 0), big), id(1));
    }

    #[test]
    fn one_pixel_wide_non_square_cells() {
        // 1x3 cells: each x is its own column; y uses the 3-pixel height.
        let m = GridMetrics::new(1, 3).unwrap();
        let g = CellGrid::new(3, 2).unwrap();
        let r = hit_test(p(2, 4), m, &g).unwrap();
        assert_eq!((r.col, r.row), (2, 1));
        assert_eq!(hit_test(p(3, 0), m, &g), None);
        assert_eq!(hit_test(p(0, 6), m, &g), None);

        // Vertical: local x is always 0, so the low side owns every pixel.
        let v = SharedBorder {
            axis: BorderAxis::Vertical,
            low: id(1),
            high: id(2),
        };
        assert_eq!(route_shared_border(&v, p(5, 0), m), id(1));

        // Horizontal: ceil(3/2) = 2, so local y 0..2 -> low, 2 -> high.
        let h = SharedBorder {
            axis: BorderAxis::Horizontal,
            low: id(1),
            high: id(2),
        };
        assert_eq!(route_shared_border(&h, p(0, 0), m), id(1));
        assert_eq!(route_shared_border(&h, p(0, 1), m), id(1));
        assert_eq!(route_shared_border(&h, p(0, 2), m), id(2));
        assert_eq!(route_shared_border(&h, p(0, 3), m), id(1)); // modulo wraps
    }

    #[test]
    fn one_pixel_high_non_square_cells() {
        // 3x1 cells: each y is its own row; x uses the 3-pixel width.
        let m = GridMetrics::new(3, 1).unwrap();
        let g = CellGrid::new(2, 3).unwrap();
        let r = hit_test(p(4, 2), m, &g).unwrap();
        assert_eq!((r.col, r.row), (1, 2));
        assert_eq!(hit_test(p(0, 3), m, &g), None);
        assert_eq!(hit_test(p(6, 0), m, &g), None);

        // Horizontal: local y is always 0, so the low side owns every pixel.
        let h = SharedBorder {
            axis: BorderAxis::Horizontal,
            low: id(1),
            high: id(2),
        };
        assert_eq!(route_shared_border(&h, p(0, 9), m), id(1));

        // Vertical: ceil(3/2) = 2, so local x 0..2 -> low, 2 -> high.
        let v = SharedBorder {
            axis: BorderAxis::Vertical,
            low: id(1),
            high: id(2),
        };
        assert_eq!(route_shared_border(&v, p(0, 0), m), id(1));
        assert_eq!(route_shared_border(&v, p(1, 0), m), id(1));
        assert_eq!(route_shared_border(&v, p(2, 0), m), id(2));
        assert_eq!(route_shared_border(&v, p(3, 0), m), id(1)); // modulo wraps
    }
}
