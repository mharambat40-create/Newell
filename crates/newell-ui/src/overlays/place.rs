// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Overlay placement: pure functions from rects to rects.

use crate::GridRect;

fn to_u32(v: u64) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

/// Places a popup of `size` (`(width, height)`) next to `anchor`, inside `bounds`.
///
/// - Horizontally the popup starts at the anchor's column and is shifted left as far as
///   needed to fit.
/// - Vertically it opens below the anchor; if that does not fit it flips above; if neither
///   fits, it takes the side with more room and its height is truncated to it.
/// - The size is first limited to `bounds`, so the result always lies inside `bounds` (it
///   may be empty). An anchor outside `bounds` is clamped to its edge.
#[must_use]
pub fn place_popup(anchor: GridRect, size: (u32, u32), bounds: GridRect) -> GridRect {
    let width = size.0.min(bounds.width());
    let want = u64::from(size.1.min(bounds.height()));
    let (left, top) = (u64::from(bounds.col()), u64::from(bounds.row()));
    let (right, bottom) = (bounds.right(), bounds.bottom());

    // `width <= bounds.width()`, so `right - width >= left`.
    let col = u64::from(anchor.col()).clamp(left, right - u64::from(width));

    let below = anchor.bottom().clamp(top, bottom);
    let above = u64::from(anchor.row()).clamp(top, bottom);
    let (room_below, room_above) = (bottom - below, above - top);
    let (row, height) = if room_below >= want {
        (below, want)
    } else if room_above >= want {
        (above - want, want)
    } else if room_below >= room_above {
        (below, room_below)
    } else {
        (top, room_above)
    };
    GridRect::new(to_u32(col), to_u32(row), width, to_u32(height))
}

/// Centres a modal of `size` in `bounds`, limiting the size to `bounds`.
#[must_use]
pub fn center_modal(size: (u32, u32), bounds: GridRect) -> GridRect {
    let width = size.0.min(bounds.width());
    let height = size.1.min(bounds.height());
    GridRect::new(
        bounds.col() + (bounds.width() - width) / 2,
        bounds.row() + (bounds.height() - height) / 2,
        width,
        height,
    )
}

/// Standard modal interior: a title row, a body, and an action row at the bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModalLayout {
    /// Top row of the interior.
    pub title: GridRect,
    /// Everything between the title and the actions row.
    pub body: GridRect,
    /// Bottom row of the interior, for the OK / Cancel row.
    pub actions: GridRect,
}

/// Splits a modal interior (as handed to the [`super::compose_overlays`] callback) into
/// title, body and actions. Rows are assigned title first, then actions; an interior shorter
/// than three rows leaves the body (and then the actions) empty.
#[must_use]
pub fn modal_layout(inner: GridRect) -> ModalLayout {
    let (title, rest) = inner.split_top(1);
    let (actions, body) = rest.split_bottom(1);
    ModalLayout {
        title,
        body,
        actions,
    }
}
