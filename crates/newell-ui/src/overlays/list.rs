// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Scrolling checkbox list with a scrollbar (`▲ ┃ ▼`).

use crate::widgets::{Canvas, WidgetContext, cell, draw_checkbox, owned_blank};
use crate::{Cell, CellContent, CellGrid, GridRect, StyleId, WidgetId, WidgetState};

/// Scroll position of a list: the index of the first visible item.
///
/// The offset is plain Controller state. Every method takes the item count and the window
/// (visible rows) and keeps the result within `0..=len - window` (0 if everything fits), so
/// a stale offset after the list shrinks or the window grows is harmless.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScrollState {
    /// Index of the first visible item.
    pub offset: u32,
}

impl ScrollState {
    /// Largest valid offset.
    #[must_use]
    pub const fn max_offset(len: u32, window: u32) -> u32 {
        len.saturating_sub(window)
    }

    /// Pulls the offset back inside the valid range.
    pub fn clamp(&mut self, len: u32, window: u32) {
        self.offset = self.offset.min(Self::max_offset(len, window));
    }

    /// Scrolls by `delta` rows (negative is up), stopping at both ends.
    pub fn scroll_by(&mut self, delta: i32, len: u32, window: u32) {
        let moved = if delta < 0 {
            self.offset.saturating_sub(delta.unsigned_abs())
        } else {
            self.offset.saturating_add(delta.unsigned_abs())
        };
        self.offset = moved.min(Self::max_offset(len, window));
    }

    /// Scrolls the least amount needed to make `index` visible. An index past the end is
    /// treated as the last item; an empty window leaves the offset at its clamped value.
    pub fn ensure_visible(&mut self, index: u32, len: u32, window: u32) {
        self.clamp(len, window);
        if window == 0 {
            return;
        }
        let index = index.min(len.saturating_sub(1));
        if index < self.offset {
            self.offset = index;
        } else if index - self.offset >= window {
            self.offset = index - (window - 1);
        }
    }
}

/// One checkbox row of a list.
#[derive(Debug, Clone, Copy)]
pub struct ListItem<'a> {
    /// Identity of the row's checkbox.
    pub id: WidgetId,
    /// Text shown after the checkbox.
    pub label: &'a str,
    /// Checked, focus and disabled state of the row.
    pub state: WidgetState,
}

/// What [`draw_scroll_list`] needs: the items, the scroll offset and the scrollbar arrows.
#[derive(Debug, Clone, Copy)]
pub struct ScrollList<'a> {
    /// All rows, not only the visible window.
    pub items: &'a [ListItem<'a>],
    /// Current scroll position.
    pub scroll: ScrollState,
    /// Identity of the `▲` arrow.
    pub up: WidgetId,
    /// Identity of the `▼` arrow.
    pub down: WidgetId,
}

/// Draws a list of checkbox rows with a scrollbar in the last column of `rect`.
///
/// - The window is `rect.height()` rows; the offset is clamped (not stored) so any offset is
///   accepted. Rows past the last item are blank and unowned.
/// - The scrollbar (`▲`, track, `▼`) needs width ≥ 2 and height ≥ 2; otherwise the items use
///   the whole rect. `▲`/`▼` are owned by `up`/`down`, take the disabled style at the
///   respective end, and are the only scrollbar cells that are hit-testable. The `┃` thumb
///   needs height ≥ 3 and shows position and proportion; when every item fits it fills the
///   track.
/// - Every cell of `rect` is overwritten, so no stale metadata survives.
pub fn draw_scroll_list(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    rect: GridRect,
    list: &ScrollList<'_>,
) {
    super::for_each_visible(grid, rect, |c| *c = Cell::default());
    let len = u32::try_from(list.items.len()).unwrap_or(u32::MAX);
    let window = rect.height();
    let offset = list.scroll.offset.min(ScrollState::max_offset(len, window));
    let has_bar = rect.width() >= 2 && window >= 2;
    let (item_width, bar) = if has_bar {
        let (bar, content) = rect.split_right(1);
        (content.width(), Some(bar))
    } else {
        (rect.width(), None)
    };

    let first = usize::try_from(offset).unwrap_or(usize::MAX);
    for (i, item) in list
        .items
        .iter()
        .skip(first)
        .take(window as usize)
        .enumerate()
    {
        let dy = u32::try_from(i).unwrap_or(u32::MAX);
        let row = GridRect::new(rect.col(), rect.row().saturating_add(dy), item_width, 1);
        draw_checkbox(grid, ctx, row, item.id, item.state, item.label);
    }
    if let Some(bar) = bar {
        draw_scrollbar(grid, ctx, bar, list, (offset, len));
    }
}

/// Draws the one-column scrollbar for `(offset, len)`. `bar` has width 1 and height ≥ 2.
fn draw_scrollbar(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    bar: GridRect,
    list: &ScrollList<'_>,
    (offset, len): (u32, u32),
) {
    let window = bar.height();
    let max = ScrollState::max_offset(len, window);
    let end_state = |at_end: bool| WidgetState {
        disabled: at_end,
        ..WidgetState::default()
    };
    let (up, rest) = bar.split_top(1);
    let (down, track) = rest.split_bottom(1);
    for (rect, glyph, id, at_end) in [
        (up, '▲', list.up, offset == 0),
        (down, '▼', list.down, offset >= max),
    ] {
        let style = ctx.style_of(id, end_state(at_end));
        let mut canvas = Canvas::new(grid, rect);
        canvas.fill_col(0, owned_blank(style, id));
        canvas.text(0, 0, &glyph.to_string(), style, Some(id));
    }

    // Thumb, in u64 so `track * window` cannot overflow.
    let track_len = u64::from(track.height());
    if track_len == 0 {
        return;
    }
    let shown = u64::from(window.min(len));
    let thumb = (track_len * shown / u64::from(len.max(1))).clamp(1, track_len);
    let start = if max == 0 {
        0
    } else {
        (track_len - thumb) * u64::from(offset) / u64::from(max)
    };
    let to_u32 = |v: u64| u32::try_from(v).unwrap_or(u32::MAX);
    let thumb_rect = GridRect::new(
        track.col(),
        track.row().saturating_add(to_u32(start)),
        1,
        to_u32(thumb),
    );
    let style: StyleId = ctx.styles.normal;
    Canvas::new(grid, thumb_rect).fill_col(0, cell(CellContent::Glyph('┃'), style, None));
}
