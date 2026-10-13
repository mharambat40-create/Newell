// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Whole-cell rectangle with deterministic split and clip operations.

use crate::CellGrid;

/// A rectangle of whole cells.
///
/// Regions are half-open: it covers columns `[col, col + width)` and rows
/// `[row, row + height)`. A zero width or height is valid and describes an
/// empty region.
///
/// The constructor clamps `width`/`height` so the far edge never exceeds
/// `2^32`, therefore no operation here can wrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridRect {
    col: u32,
    row: u32,
    width: u32,
    height: u32,
}

/// Length of `[start, start + len)` clamped to the `u32` coordinate space.
fn clamp_len(start: u32, len: u32) -> u32 {
    let room = (1_u64 << 32) - u64::from(start);
    u32::try_from(u64::from(len).min(room)).unwrap_or(u32::MAX)
}

/// Builds a rect from 64-bit bounds; `start <= 2^32` and `len <= u32::MAX`.
/// An empty rect starting exactly at `2^32` saturates its origin to `u32::MAX`.
fn from_wide(col: u64, row: u64, width: u64, height: u64) -> GridRect {
    let to_u32 = |v: u64| u32::try_from(v).unwrap_or(u32::MAX);
    GridRect::new(to_u32(col), to_u32(row), to_u32(width), to_u32(height))
}

impl GridRect {
    /// Creates a rect, clamping extents that would pass the `u32` coordinate space.
    #[must_use]
    pub fn new(col: u32, row: u32, width: u32, height: u32) -> Self {
        Self {
            col,
            row,
            width: clamp_len(col, width),
            height: clamp_len(row, height),
        }
    }

    /// The rect covering a whole grid, anchored at the origin.
    #[must_use]
    pub fn from_grid(grid: &CellGrid) -> Self {
        Self::new(0, 0, grid.cols(), grid.rows())
    }

    #[must_use]
    pub const fn col(&self) -> u32 {
        self.col
    }

    #[must_use]
    pub const fn row(&self) -> u32 {
        self.row
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Exclusive right edge, `col + width`. Widened so it is always exact.
    #[must_use]
    pub const fn right(&self) -> u64 {
        self.col as u64 + self.width as u64
    }

    /// Exclusive bottom edge, `row + height`. Widened so it is always exact.
    #[must_use]
    pub const fn bottom(&self) -> u64 {
        self.row as u64 + self.height as u64
    }

    /// True if the rect covers no cells.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// The overlap of two rects. Disjoint or merely touching rects yield an
    /// empty rect (zero width and height).
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        let col = u64::from(self.col.max(other.col));
        let row = u64::from(self.row.max(other.row));
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        if right <= col || bottom <= row {
            return from_wide(col, row, 0, 0);
        }
        from_wide(col, row, right - col, bottom - row)
    }

    /// Shrinks the rect by `amount` cells on every side. Collapses to an empty
    /// rect (centred where possible) instead of underflowing.
    #[must_use]
    pub fn inset(&self, amount: u32) -> Self {
        let shrink = |start: u32, len: u32| {
            let cut = u64::from(amount).saturating_mul(2);
            if cut >= u64::from(len) {
                (u64::from(start) + u64::from(len) / 2, 0)
            } else {
                (u64::from(start) + u64::from(amount), u64::from(len) - cut)
            }
        };
        let (col, width) = shrink(self.col, self.width);
        let (row, height) = shrink(self.row, self.height);
        from_wide(col, row, width, height)
    }

    /// Splits off up to `cols` columns from the left: `(left, remainder)`.
    /// A request larger than the width takes everything.
    #[must_use]
    pub fn split_left(&self, cols: u32) -> (Self, Self) {
        let take = cols.min(self.width);
        let left = Self::new(self.col, self.row, take, self.height);
        let rest = from_wide(
            u64::from(self.col) + u64::from(take),
            u64::from(self.row),
            u64::from(self.width - take),
            u64::from(self.height),
        );
        (left, rest)
    }

    /// Splits off up to `cols` columns from the right: `(right, remainder)`.
    #[must_use]
    pub fn split_right(&self, cols: u32) -> (Self, Self) {
        let take = cols.min(self.width);
        let keep = self.width - take;
        let right = from_wide(
            u64::from(self.col) + u64::from(keep),
            u64::from(self.row),
            u64::from(take),
            u64::from(self.height),
        );
        let rest = Self::new(self.col, self.row, keep, self.height);
        (right, rest)
    }

    /// Splits off up to `rows` rows from the top: `(top, remainder)`.
    #[must_use]
    pub fn split_top(&self, rows: u32) -> (Self, Self) {
        let take = rows.min(self.height);
        let top = Self::new(self.col, self.row, self.width, take);
        let rest = from_wide(
            u64::from(self.col),
            u64::from(self.row) + u64::from(take),
            u64::from(self.width),
            u64::from(self.height - take),
        );
        (top, rest)
    }

    /// Splits off up to `rows` rows from the bottom: `(bottom, remainder)`.
    #[must_use]
    pub fn split_bottom(&self, rows: u32) -> (Self, Self) {
        let take = rows.min(self.height);
        let keep = self.height - take;
        let bottom = from_wide(
            u64::from(self.col),
            u64::from(self.row) + u64::from(keep),
            u64::from(self.width),
            u64::from(take),
        );
        let rest = Self::new(self.col, self.row, self.width, keep);
        (bottom, rest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(col: u32, row: u32, w: u32, h: u32) -> GridRect {
        GridRect::new(col, row, w, h)
    }

    fn dims(g: GridRect) -> (u32, u32, u32, u32) {
        (g.col(), g.row(), g.width(), g.height())
    }

    #[test]
    fn normal_rect_half_open() {
        let g = r(2, 3, 10, 4);
        assert_eq!(dims(g), (2, 3, 10, 4));
        assert_eq!((g.right(), g.bottom()), (12, 7));
        assert!(!g.is_empty());
    }

    #[test]
    fn zero_sizes_are_valid_and_empty() {
        assert!(r(1, 1, 0, 5).is_empty());
        assert!(r(1, 1, 5, 0).is_empty());
        assert!(r(1, 1, 0, 0).is_empty());
        assert_eq!(r(1, 1, 0, 5).right(), 1);
    }

    #[test]
    fn from_grid_covers_grid() {
        let g = CellGrid::new(7, 3).unwrap();
        assert_eq!(dims(GridRect::from_grid(&g)), (0, 0, 7, 3));
        let z = CellGrid::new(0, 0).unwrap();
        assert!(GridRect::from_grid(&z).is_empty());
    }

    #[test]
    fn constructor_clamps_instead_of_wrapping() {
        let g = r(u32::MAX, u32::MAX, u32::MAX, u32::MAX);
        assert_eq!((g.width(), g.height()), (1, 1));
        assert_eq!((g.right(), g.bottom()), (1 << 32, 1 << 32));
        let h = r(0, 0, u32::MAX, u32::MAX);
        assert_eq!(h.right(), u64::from(u32::MAX));
    }

    #[test]
    fn intersection_with_self() {
        let g = r(2, 3, 10, 4);
        assert_eq!(g.intersection(&g), g);
    }

    #[test]
    fn intersection_full_overlap() {
        let outer = r(0, 0, 20, 20);
        let inner = r(5, 5, 3, 3);
        assert_eq!(outer.intersection(&inner), inner);
        assert_eq!(inner.intersection(&outer), inner);
    }

    #[test]
    fn intersection_partial() {
        let a = r(0, 0, 10, 10);
        let b = r(6, 8, 10, 10);
        assert_eq!(dims(a.intersection(&b)), (6, 8, 4, 2));
        assert_eq!(a.intersection(&b), b.intersection(&a));
    }

    #[test]
    fn intersection_disjoint_is_empty() {
        let a = r(0, 0, 4, 4);
        assert!(a.intersection(&r(10, 10, 4, 4)).is_empty());
        assert!(a.intersection(&r(10, 0, 4, 4)).is_empty());
    }

    #[test]
    fn intersection_touching_edges_is_empty() {
        let a = r(0, 0, 4, 4);
        assert!(a.intersection(&r(4, 0, 4, 4)).is_empty());
        assert!(a.intersection(&r(0, 4, 4, 4)).is_empty());
        assert!(a.intersection(&r(4, 4, 4, 4)).is_empty());
    }

    #[test]
    fn intersection_with_empty_is_empty() {
        let a = r(0, 0, 10, 10);
        assert!(a.intersection(&r(3, 3, 0, 5)).is_empty());
        assert!(a.intersection(&r(3, 3, 5, 0)).is_empty());
        assert!(r(3, 3, 0, 0).intersection(&a).is_empty());
    }

    #[test]
    fn intersection_near_max() {
        let a = r(u32::MAX - 5, 0, 100, 10);
        let b = r(u32::MAX - 2, 0, 100, 10);
        assert_eq!(dims(a.intersection(&b)), (u32::MAX - 2, 0, 3, 10));
        let far = r(u32::MAX, u32::MAX, 1, 1);
        assert!(far.intersection(&r(0, 0, 5, 5)).is_empty());
    }

    #[test]
    fn inset_shrinks_and_collapses() {
        assert_eq!(dims(r(0, 0, 10, 6).inset(1)), (1, 1, 8, 4));
        assert_eq!(dims(r(0, 0, 10, 6).inset(0)), (0, 0, 10, 6));
        assert!(r(0, 0, 2, 2).inset(1).is_empty());
        assert!(r(5, 5, 3, 9).inset(2).is_empty());
        assert!(r(0, 0, 10, 10).inset(u32::MAX).is_empty());
        assert!(r(u32::MAX, u32::MAX, 1, 1).inset(1).is_empty());
    }

    #[test]
    fn split_left_cases() {
        let g = r(5, 2, 100, 7);
        let (l, rest) = g.split_left(20);
        assert_eq!(dims(l), (5, 2, 20, 7));
        assert_eq!(dims(rest), (25, 2, 80, 7));
        let (l, rest) = g.split_left(0);
        assert!(l.is_empty());
        assert_eq!(rest, g);
        let (l, rest) = g.split_left(100);
        assert_eq!(l, g);
        assert_eq!(rest.width(), 0);
        let (l, rest) = r(0, 0, 10, 3).split_left(20);
        assert_eq!((l.width(), rest.width()), (10, 0));
        let (l, rest) = r(0, 0, 0, 3).split_left(4);
        assert!(l.is_empty() && rest.is_empty());
    }

    #[test]
    fn split_right_cases() {
        let g = r(5, 2, 100, 7);
        let (rt, rest) = g.split_right(20);
        assert_eq!(dims(rt), (85, 2, 20, 7));
        assert_eq!(dims(rest), (5, 2, 80, 7));
        let (rt, rest) = g.split_right(0);
        assert!(rt.is_empty());
        assert_eq!(rest, g);
        let (rt, rest) = g.split_right(100);
        assert_eq!(rt, g);
        assert_eq!(rest.width(), 0);
        let (rt, rest) = r(0, 0, 10, 3).split_right(20);
        assert_eq!((rt.width(), rest.width()), (10, 0));
        let (rt, rest) = r(0, 0, 0, 3).split_right(4);
        assert!(rt.is_empty() && rest.is_empty());
    }

    #[test]
    fn split_top_cases() {
        let g = r(2, 5, 7, 100);
        let (t, rest) = g.split_top(20);
        assert_eq!(dims(t), (2, 5, 7, 20));
        assert_eq!(dims(rest), (2, 25, 7, 80));
        let (t, rest) = g.split_top(0);
        assert!(t.is_empty());
        assert_eq!(rest, g);
        let (t, rest) = g.split_top(100);
        assert_eq!(t, g);
        assert_eq!(rest.height(), 0);
        let (t, rest) = r(0, 0, 3, 10).split_top(20);
        assert_eq!((t.height(), rest.height()), (10, 0));
        let (t, rest) = r(0, 0, 3, 0).split_top(4);
        assert!(t.is_empty() && rest.is_empty());
    }

    #[test]
    fn split_bottom_cases() {
        let g = r(2, 5, 7, 100);
        let (b, rest) = g.split_bottom(20);
        assert_eq!(dims(b), (2, 85, 7, 20));
        assert_eq!(dims(rest), (2, 5, 7, 80));
        let (b, rest) = g.split_bottom(0);
        assert!(b.is_empty());
        assert_eq!(rest, g);
        let (b, rest) = g.split_bottom(100);
        assert_eq!(b, g);
        assert_eq!(rest.height(), 0);
        let (b, rest) = r(0, 0, 3, 10).split_bottom(20);
        assert_eq!((b.height(), rest.height()), (10, 0));
        let (b, rest) = r(0, 0, 3, 0).split_bottom(4);
        assert!(b.is_empty() && rest.is_empty());
    }

    #[test]
    fn splits_partition_the_rect() {
        let g = r(3, 4, 50, 30);
        for n in [0, 1, 49, 50, 51, u32::MAX] {
            let (a, b) = g.split_left(n);
            assert_eq!(a.width() + b.width(), 50);
            assert_eq!(a.right(), u64::from(b.col()));
            let (a, b) = g.split_right(n);
            assert_eq!(a.width() + b.width(), 50);
            assert_eq!(b.right(), u64::from(a.col()));
            let (a, b) = g.split_top(n);
            assert_eq!(a.height() + b.height(), 30);
            assert_eq!(a.bottom(), u64::from(b.row()));
            let (a, b) = g.split_bottom(n);
            assert_eq!(a.height() + b.height(), 30);
            assert_eq!(b.bottom(), u64::from(a.row()));
        }
    }

    #[test]
    fn splits_near_max_do_not_wrap() {
        let g = r(u32::MAX - 9, u32::MAX - 9, 10, 10);
        for n in [0, 5, 10, u32::MAX] {
            let (a, b) = g.split_left(n);
            assert_eq!(a.width() + b.width(), 10);
            assert!(b.col() >= g.col());
            let (a, b) = g.split_right(n);
            assert_eq!(a.width() + b.width(), 10);
            assert!(a.col() >= g.col());
            let (a, b) = g.split_top(n);
            assert_eq!(a.height() + b.height(), 10);
            assert!(b.row() >= g.row());
            let (a, b) = g.split_bottom(n);
            assert_eq!(a.height() + b.height(), 10);
            assert!(a.row() >= g.row());
        }
        let full = r(0, 0, u32::MAX, u32::MAX);
        let (a, b) = full.split_left(u32::MAX);
        assert_eq!((a.width(), b.width(), b.col()), (u32::MAX, 0, u32::MAX));
        let (a, b) = full.split_top(1);
        assert_eq!((a.height(), b.row(), b.height()), (1, 1, u32::MAX - 1));
    }
}
