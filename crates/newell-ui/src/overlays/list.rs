// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Scrolling checkbox list with a scrollbar (`▲ ┃ ▼`).

use crate::widgets::{Canvas, WidgetContext, draw_checkbox};
use crate::{Cell, CellContent, CellGrid, GridRect, WidgetId, WidgetState};

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
        self.scroll_by_wide(i64::from(delta), len, window);
    }

    pub(crate) fn scroll_by_wide(&mut self, delta: i64, len: u32, window: u32) {
        self.clamp(len, window);
        let offset = u64::from(self.offset);
        let moved = if delta < 0 {
            offset.saturating_sub(delta.unsigned_abs())
        } else {
            offset.saturating_add(delta.unsigned_abs())
        };
        self.offset = moved.min(u64::from(Self::max_offset(len, window))) as u32;
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
    /// Identity of the draggable thumb.
    pub thumb: WidgetId,
}

/// Logical selection-list geometry; clipping never changes the five-row window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectionListGeometry {
    pub allocation: GridRect,
    pub content: GridRect,
    pub bar: Option<GridRect>,
    pub window: u32,
    pub max: u32,
}

impl SelectionListGeometry {
    pub fn new(allocation: GridRect, len: u32) -> Self {
        let window = len.min(5);
        let (bar, content) = if len > 5 && allocation.width() > 0 {
            let (bar, content) = allocation.split_right(1);
            (Some(GridRect::new(bar.col(), bar.row(), 1, 5)), content)
        } else {
            (None, allocation)
        };
        Self {
            allocation,
            content: GridRect::new(content.col(), content.row(), content.width(), window)
                .intersection(&allocation),
            bar,
            window,
            max: len.saturating_sub(5),
        }
    }

    pub fn row(self, index: u32) -> GridRect {
        GridRect::new(
            self.content.col(),
            self.content.row().saturating_add(index),
            self.content.width(),
            1,
        )
        .intersection(&self.content)
    }
}

/// Nonnegative rational rounding, with exact ties rounded upward.
pub(crate) fn round_half_up(a: u64, b: u64) -> u64 {
    if b == 0 {
        return 0;
    }
    a / b + u64::from(a % b >= b / 2 + b % 2)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectionThumb {
    pub offset: u32,
    pub max: u32,
    pub length: u32,
    pub travel: u32,
    pub position: u32,
}

impl SelectionThumb {
    pub fn new(len: u32, offset: u32) -> Option<Self> {
        if len <= 5 {
            return None;
        }
        let max = len - 5;
        let offset = offset.min(max);
        let length = 11_u32.saturating_sub(len).max(1);
        let travel = 6 - length;
        let position = round_half_up(u64::from(travel) * u64::from(offset), u64::from(max)) as u32;
        Some(Self {
            offset,
            max,
            length,
            travel,
            position,
        })
    }

    pub fn glyphs(self) -> [char; 3] {
        std::array::from_fn(|row| {
            let occupied = |slot| slot >= self.position && slot < self.position + self.length;
            match (occupied(row as u32 * 2), occupied(row as u32 * 2 + 1)) {
                (true, true) => '┃',
                (true, false) => '╹',
                (false, true) => '╻',
                (false, false) => ' ',
            }
        })
    }
}

/// Writes the same clipped, five-cell scrollbar for both selection presentations.
pub(crate) fn draw_selection_scrollbar(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    geometry: SelectionListGeometry,
    thumb: SelectionThumb,
    ids: [WidgetId; 3],
) {
    let Some(bar) = geometry.bar else {
        return;
    };
    let track = thumb.glyphs();
    for (dy, glyph) in ['▲', track[0], track[1], track[2], '▼']
        .into_iter()
        .enumerate()
    {
        let (id, disabled) = match dy {
            0 => (Some(ids[0]), thumb.offset == 0),
            4 => (Some(ids[1]), thumb.offset == thumb.max),
            _ => ((glyph != ' ').then_some(ids[2]), false),
        };
        let style = id.map_or(ctx.styles.normal, |id| {
            ctx.style_of(
                id,
                WidgetState {
                    disabled,
                    ..WidgetState::default()
                },
            )
        });
        let rect = GridRect::new(bar.col(), bar.row().saturating_add(dy as u32), 1, 1)
            .intersection(&geometry.allocation);
        Canvas::new(grid, rect).fill_col(
            0,
            crate::widgets::cell_with_ink(
                if glyph == ' ' {
                    CellContent::Empty
                } else {
                    CellContent::Glyph(glyph)
                },
                style,
                id,
                if id.is_some() {
                    crate::InkRole::Action
                } else {
                    crate::InkRole::Text
                },
            ),
        );
    }
}

/// Draws at most five content rows; a gutter appears only for more than five items.
/// `rect` is an allocation/clip, not the logical scroll window. All its cells are
/// overwritten, including unused rows and a gutter removed after list shrinkage.
pub fn draw_scroll_list(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    rect: GridRect,
    list: &ScrollList<'_>,
) {
    super::for_each_visible(grid, rect, |c| *c = Cell::default());
    let len = u32::try_from(list.items.len()).unwrap_or(u32::MAX);
    let geometry = SelectionListGeometry::new(rect, len);
    let offset = list.scroll.offset.min(geometry.max);
    for (i, item) in list
        .items
        .iter()
        .skip(offset as usize)
        .take(geometry.window as usize)
        .enumerate()
    {
        draw_checkbox(
            grid,
            ctx,
            geometry.row(i as u32),
            item.id,
            item.state,
            item.label,
        );
    }
    if let Some(thumb) = SelectionThumb::new(len, offset) {
        draw_selection_scrollbar(grid, ctx, geometry, thumb, [list.up, list.down, list.thumb]);
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;
    use crate::{InkRole, InteractionState, WidgetStyles};
    #[test]
    fn color_list_values_are_text_and_checkbox_arrows_and_thumb_are_action() {
        let styles = WidgetStyles::default();
        let interaction = InteractionState::default();
        let ctx = WidgetContext {
            styles: &styles,
            interaction: &interaction,
        };
        let items: Vec<_> = (0..12)
            .map(|i| ListItem {
                id: WidgetId::new(i + 10),
                label: "│[+]☰ ordinary value",
                state: WidgetState::default(),
            })
            .collect();
        let list = ScrollList {
            items: &items,
            scroll: ScrollState::default(),
            up: WidgetId::new(30),
            down: WidgetId::new(31),
            thumb: WidgetId::new(32),
        };
        let mut grid = CellGrid::new(24, 6).unwrap();
        for y in 0..6 {
            for x in 0..24 {
                grid.get_mut(x, y).unwrap().ink = InkRole::Contour;
            }
        }
        draw_scroll_list(&mut grid, &ctx, GridRect::new(0, 0, 24, 6), &list);
        for y in 0..5 {
            assert_eq!(grid.get(0, y).unwrap().ink, InkRole::Action);
            for x in 1..23 {
                assert_eq!(grid.get(x, y).unwrap().ink, InkRole::Text);
            }
        }
        for (y, glyph) in [(0, '▲'), (4, '▼')] {
            let cell = grid.get(23, y).unwrap();
            assert_eq!(cell.content, CellContent::Glyph(glyph));
            assert_eq!(cell.ink, InkRole::Action);
        }
        let thumb = grid
            .cells()
            .iter()
            .find(|cell| matches!(cell.content, CellContent::Glyph('┃' | '╹' | '╻')))
            .unwrap();
        assert_eq!(thumb.ink, InkRole::Action);
        assert_eq!(thumb.widget_id, Some(list.thumb));
    }
}

#[cfg(test)]
pub(crate) mod scrollbar_tests {
    use super::*;
    use crate::{BorderAxis, InkRole, InteractionState, SharedBorder, WidgetStyles};

    pub(crate) const CASES: &[(u32, &[&str])] = &[
        (6, &["┃┃╹", "╻┃┃"]),
        (7, &["┃┃ ", "╻┃╹", " ┃┃"]),
        (8, &["┃╹ ", "╻┃ ", " ┃╹", " ╻┃"]),
        (9, &["┃  ", "╻╹ ", " ┃ ", " ╻╹", "  ┃"]),
        (10, &["╹  ", "╻  ", " ╹ ", " ╻ ", "  ╹", "  ╻"]),
        (11, &["╹  ", "╻  ", " ╹ ", " ╻ ", " ╻ ", "  ╹", "  ╻"]),
    ];

    fn items(len: u32) -> Vec<ListItem<'static>> {
        (0..len)
            .map(|i| ListItem {
                id: WidgetId::new(100 + i),
                label: "duplicate long name",
                state: WidgetState {
                    checked: i % 2 == 1,
                    ..WidgetState::default()
                },
            })
            .collect()
    }

    fn draw(grid: &mut CellGrid, rect: GridRect, items: &[ListItem<'_>], offset: u32) {
        draw_scroll_list(
            grid,
            &WidgetContext {
                styles: &WidgetStyles::default(),
                interaction: &InteractionState::default(),
            },
            rect,
            &ScrollList {
                items,
                scroll: ScrollState { offset },
                up: WidgetId::new(1),
                down: WidgetId::new(2),
                thumb: WidgetId::new(3),
            },
        );
    }

    fn glyph(grid: &CellGrid, x: u32, y: u32) -> char {
        match grid.get(x, y).unwrap().content {
            CellContent::Glyph(g) => g,
            _ => ' ',
        }
    }

    #[test]
    fn every_canonical_state_has_exact_track_content_and_hit_ownership() {
        for &(len, states) in CASES {
            let items = items(len);
            for (s, expected) in states.iter().enumerate() {
                let mut grid = CellGrid::new(16, 8).unwrap();
                draw(&mut grid, GridRect::new(1, 1, 14, 6), &items, s as u32);
                assert_eq!(
                    SelectionThumb::new(len, s as u32)
                        .unwrap()
                        .glyphs()
                        .iter()
                        .collect::<String>(),
                    *expected
                );
                assert_eq!(glyph(&grid, 14, 1), '▲');
                assert_eq!(glyph(&grid, 14, 5), '▼');
                for (i, g) in expected.chars().enumerate() {
                    let cell = grid.get(14, i as u32 + 2).unwrap();
                    assert_eq!(glyph(&grid, 14, i as u32 + 2), g, "N={len}, s={s}");
                    assert_eq!(cell.widget_id, (g != ' ').then_some(WidgetId::new(3)));
                    assert_eq!(cell.shared_border, None);
                    assert_eq!(
                        cell.ink,
                        if g == ' ' {
                            InkRole::Text
                        } else {
                            InkRole::Action
                        }
                    );
                }
                for i in 0..5 {
                    assert_eq!(
                        grid.get(1, i + 1).unwrap().widget_id,
                        Some(items[s + i as usize].id)
                    );
                    assert_eq!(
                        glyph(&grid, 1, i + 1),
                        if items[s + i as usize].state.checked {
                            '■'
                        } else {
                            '□'
                        }
                    );
                    assert_eq!(
                        grid.get(13, i + 1).unwrap().widget_id,
                        Some(items[s + i as usize].id)
                    );
                }
                assert!(grid.get(1, 6).unwrap().widget_id.is_none());
                assert!(grid.get(14, 6).unwrap().widget_id.is_none());
                assert_eq!(
                    grid.get(14, 1).unwrap().style == WidgetStyles::default().disabled,
                    s == 0
                );
                assert_eq!(
                    grid.get(14, 5).unwrap().style == WidgetStyles::default().disabled,
                    s + 1 == states.len()
                );
            }
        }
    }

    #[test]
    fn fitting_empty_growth_and_shrink_clear_the_old_gutter_and_metadata() {
        let rect = GridRect::new(0, 0, 16, 7);
        let mut grid = CellGrid::new(16, 7).unwrap();
        for len in [5, 6, 5, 9, 6, 0, 1, 2, 3, 4, 5] {
            for y in 0..7 {
                for x in 0..16 {
                    let cell = grid.get_mut(x, y).unwrap();
                    cell.shared_border = Some(SharedBorder {
                        axis: BorderAxis::Vertical,
                        low: Some(WidgetId::new(99)),
                        high: None,
                    });
                    cell.ink = InkRole::Contour;
                }
            }
            let it = items(len);
            let s = 4_u32.min(len.saturating_sub(5));
            draw(&mut grid, rect, &it, u32::MAX);
            for y in 0..7 {
                for x in 0..16 {
                    let cell = grid.get(x, y).unwrap();
                    assert!(cell.shared_border.is_none());
                    if len <= 5 {
                        assert!(
                            ![
                                Some(WidgetId::new(1)),
                                Some(WidgetId::new(2)),
                                Some(WidgetId::new(3))
                            ]
                            .contains(&cell.widget_id)
                        );
                    }
                    if y >= len.min(5) {
                        assert!(cell.widget_id.is_none());
                    }
                }
            }
            if len > 0 {
                assert_eq!(grid.get(0, 0).unwrap().widget_id, Some(it[s as usize].id));
            }
            if len <= 5 && len > 0 {
                assert_eq!(grid.get(15, 0).unwrap().widget_id, Some(it[0].id));
            }
        }
    }

    #[test]
    fn large_lists_round_monotonically_without_quantizing_content() {
        let it = items(1000);
        let mut grid = CellGrid::new(8, 5).unwrap();
        let mut previous = 0;
        let mut positions = std::collections::BTreeSet::new();
        for offset in 0..=995 {
            let thumb = SelectionThumb::new(1000, offset).unwrap();
            assert!(thumb.position >= previous);
            previous = thumb.position;
            positions.insert(thumb.position);
            draw(&mut grid, GridRect::new(0, 0, 8, 5), &it, offset);
            assert_eq!(
                grid.get(0, 0).unwrap().widget_id,
                Some(it[offset as usize].id)
            );
        }
        assert_eq!(
            positions.into_iter().collect::<Vec<_>>(),
            [0, 1, 2, 3, 4, 5]
        );
        for len in [6, 10, 11, 15, 1000, u32::MAX] {
            for offset in [0, 1, len / 2, len - 5, u32::MAX] {
                let t = SelectionThumb::new(len, offset).unwrap();
                let a = u128::from(t.travel) * u128::from(offset.min(t.max));
                let b = u128::from(t.max);
                assert_eq!(u128::from(t.position), (2 * a + b) / (2 * b));
            }
            assert_eq!(SelectionThumb::new(len, 0).unwrap().position, 0);
            let end = SelectionThumb::new(len, u32::MAX).unwrap();
            assert_eq!(end.position, end.travel);
        }
        assert_eq!(round_half_up(1, 2), 1);
        assert_eq!(round_half_up(3, 2), 2);
        assert_eq!(round_half_up(2, 5), 0);
        assert_eq!(round_half_up(3, 5), 1);
        assert_eq!(round_half_up(10, 4), 3);
        assert_eq!(SelectionThumb::new(15, 1).unwrap().position, 1);
        assert_eq!(SelectionThumb::new(15, 3).unwrap().position, 2);
    }

    #[test]
    fn constrained_allocations_crop_without_moving_bar_or_window() {
        let it = items(11);
        for w in [0, 1, 2, 6, 24] {
            for h in 0..8 {
                let mut grid = CellGrid::new(12, 10).unwrap();
                let sentinel =
                    crate::widgets::owned_blank(WidgetStyles::default().normal, WidgetId::new(999));
                for y in 0..10 {
                    for x in 0..12 {
                        *grid.get_mut(x, y).unwrap() = sentinel;
                    }
                }
                let rect = GridRect::new(2, 1, w, h);
                draw(&mut grid, rect, &it, u32::MAX);
                for y in 0..10 {
                    for x in 0..12 {
                        let cell = grid.get(x, y).unwrap();
                        if x < 2
                            || u64::from(x) >= rect.right()
                            || y < 1
                            || u64::from(y) >= rect.bottom()
                        {
                            assert_eq!(*cell, sentinel);
                        } else if h > 0 && y == 1 && w > 1 {
                            if x == 2 {
                                assert_eq!(cell.widget_id, Some(it[6].id));
                            }
                        }
                    }
                }
                if w > 0 && w <= 10 && h > 0 {
                    assert_eq!(glyph(&grid, 2 + w - 1, 1), '▲');
                }
                if w > 0 && w <= 10 && h >= 5 {
                    assert_eq!(glyph(&grid, 2 + w - 1, 5), '▼');
                }
            }
        }
    }

    #[test]
    fn stale_offsets_are_clamped_before_signed_deltas() {
        let mut s = ScrollState { offset: u32::MAX };
        s.scroll_by(-1, 9, 5);
        assert_eq!(s.offset, 3);
        s.scroll_by(i32::MIN, 9, 5);
        assert_eq!(s.offset, 0);
        s.scroll_by(i32::MAX, 9, 5);
        assert_eq!(s.offset, 4);
        s.scroll_by_wide(i64::MIN, 9, 5);
        assert_eq!(s.offset, 0);
        s.scroll_by_wide(i64::MAX, u32::MAX, 5);
        assert_eq!(s.offset, u32::MAX - 5);
        s.clamp(1000, 5);
        assert_eq!(s.offset, 995);
        s.clamp(6, 5);
        assert_eq!(s.offset, 1);
        s.clamp(0, 0);
        assert_eq!(s.offset, 0);
        s.ensure_visible(99, 0, 5);
        assert_eq!(s.offset, 0);
    }
}
