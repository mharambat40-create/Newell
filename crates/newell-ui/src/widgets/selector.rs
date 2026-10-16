// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Cyclic selector.
//!
//! A [`Cyclic`] selector shows only the current value between `◀` and `▶`. The segmented
//! selectors (`Solid | Surface`, `New | Add | Remove`) are not a widget: their border topology
//! is part of the Action Box header composed by `extrude` (`README_UI.md` §5.3, M2.21).

use super::{Canvas, WidgetContext, cell, centered, text_len};
use crate::{CellContent, CellGrid, GridRect, WidgetId, WidgetState};

/// One ordered value shown at a time. `prev` / `next` own the `◀` / `▶` cells. The value
/// text is not interactive. An out-of-range `current` shows no value.
#[derive(Debug, Clone, Copy)]
pub struct Cyclic<'a> {
    pub prev: WidgetId,
    pub next: WidgetId,
    pub values: &'a [&'a str],
    pub current: usize,
    pub state: WidgetState,
}

/// Index after `current`, wrapping to 0. Returns 0 when `len` is 0.
#[must_use]
pub const fn cycle_next(current: usize, len: usize) -> usize {
    if len == 0 || current.saturating_add(1) >= len {
        0
    } else {
        current + 1
    }
}

/// Index before `current`, wrapping to the last. Returns 0 when `len` is 0.
#[must_use]
pub const fn cycle_prev(current: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else if current == 0 || current > len {
        len - 1
    } else {
        current - 1
    }
}

/// Draws a cyclic selector on the first row of `rect`: `◀` in the first cell, `▶` in the
/// last, the current value centred (and truncated) between them. Needs a width of at least
/// 2, else nothing is drawn. Only the arrow cells are owned (by `prev` / `next`); the value
/// area is blank-filled and unowned.
pub fn draw_cyclic(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    rect: GridRect,
    selector: &Cyclic<'_>,
) {
    if rect.width() < 2 {
        return;
    }
    let mut canvas = Canvas::new(grid, rect);
    let last = canvas.width() - 1;
    let inner = last - 1;
    let value_style = ctx.plain_style(selector.state);
    canvas.fill(1, inner, 0, cell(CellContent::Empty, value_style, None));
    if let Some(value) = selector.values.get(selector.current) {
        let shown: String = value.chars().take(inner as usize).collect();
        canvas.text(
            1 + centered(inner, text_len(&shown)),
            0,
            &shown,
            value_style,
            None,
        );
    }
    for (dx, glyph, id) in [(0, '◀', selector.prev), (last, '▶', selector.next)] {
        let style = ctx.style_of(id, selector.state);
        canvas.put(dx, 0, cell(CellContent::Glyph(glyph), style, Some(id)));
    }
}

#[cfg(test)]
mod tests {
    use super::super::WidgetStyles;
    use super::super::testing::*;
    use super::*;
    use crate::{InteractionState, StyleId};

    fn r(c: u32, row: u32, w: u32, h: u32) -> GridRect {
        GridRect::new(c, row, w, h)
    }

    fn draw_cyc(width: u32, values: &[&str], current: usize, state: WidgetState) -> CellGrid {
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let mut g = grid(width, 1);
        let sel = Cyclic {
            prev: id(1),
            next: id(2),
            values,
            current,
            state,
        };
        draw_cyclic(&mut g, &ctx(&styles, &inter), r(0, 0, width, 1), &sel);
        g
    }

    #[test]
    fn cyclic_shows_only_current_value_between_arrows() {
        let values = ["Blind", "Up to", "Through all"];
        let g = draw_cyc(20, &values, 0, WidgetState::default());
        assert_eq!(text_row(&g, 0), "◀......Blind.......▶");
        assert_eq!(owner_row(&g, 0), "1------------------2");
        let g = draw_cyc(20, &values, 1, WidgetState::default());
        assert_eq!(text_row(&g, 0), "◀......Up.to.......▶");
        assert!(!text_row(&g, 0).contains("Blind"));
    }

    #[test]
    fn cyclic_edge_cases() {
        let values = ["Through all"];
        assert_eq!(
            text_row(&draw_cyc(8, &values, 0, WidgetState::default()), 0),
            "◀Throug▶"
        );
        assert_eq!(
            text_row(&draw_cyc(2, &values, 0, WidgetState::default()), 0),
            "◀▶"
        );
        assert_eq!(draw_cyc(1, &values, 0, WidgetState::default()), grid(1, 1));
        assert_eq!(
            text_row(&draw_cyc(6, &values, 5, WidgetState::default()), 0),
            "◀....▶"
        );
        assert_eq!(
            text_row(&draw_cyc(6, &[], 0, WidgetState::default()), 0),
            "◀....▶"
        );
    }

    #[test]
    fn cyclic_disabled_styles_arrows_and_value() {
        let styles = WidgetStyles::default();
        let off = WidgetState {
            disabled: true,
            ..WidgetState::default()
        };
        let g = draw_cyc(10, &["Blind"], 0, off);
        assert!(g.cells().iter().all(|c| c.style == styles.disabled));
        assert_eq!(g.get(0, 0).unwrap().widget_id, Some(id(1)));
        assert_eq!(g.get(9, 0).unwrap().widget_id, Some(id(2)));
        assert_eq!(StyleId::DEFAULT, styles.normal);
    }

    #[test]
    fn cyclic_focus_and_press_style_only_the_arrows() {
        let values = ["Blind"];
        let styles = WidgetStyles::default();
        let sel = Cyclic {
            prev: id(1),
            next: id(2),
            values: &values,
            current: 0,
            state: WidgetState::default(),
        };
        let mut inter = InteractionState::default();
        let mut g = grid(10, 1);
        inter.set_focused(Some(id(1)));
        draw_cyclic(&mut g, &ctx(&styles, &inter), r(0, 0, 10, 1), &sel);
        assert_eq!(g.get(0, 0).unwrap().style, styles.focused);
        assert_eq!(g.get(9, 0).unwrap().style, styles.normal);
        // The value is unowned and never takes the arrow's focus or press style.
        assert_eq!(g.get(4, 0).unwrap().style, styles.normal);
        assert!(inter.press(id(2), WidgetState::default()));
        draw_cyclic(&mut g, &ctx(&styles, &inter), r(0, 0, 10, 1), &sel);
        assert_eq!(g.get(9, 0).unwrap().style, styles.pressed);
        assert_eq!(g.get(0, 0).unwrap().style, styles.focused);
        assert_eq!(g.get(4, 0).unwrap().style, styles.normal);
    }

    #[test]
    fn cycle_wraps_in_both_directions() {
        assert_eq!(
            [cycle_next(0, 3), cycle_next(1, 3), cycle_next(2, 3)],
            [1, 2, 0]
        );
        assert_eq!(
            [cycle_prev(0, 3), cycle_prev(1, 3), cycle_prev(2, 3)],
            [2, 0, 1]
        );
        assert_eq!((cycle_next(0, 1), cycle_prev(0, 1)), (0, 0));
        assert_eq!((cycle_next(0, 0), cycle_prev(0, 0)), (0, 0));
        assert_eq!((cycle_next(9, 3), cycle_prev(9, 3)), (0, 2));
        assert_eq!(cycle_prev(usize::MAX, 3), 2);
        assert_eq!(cycle_next(usize::MAX, 3), 0);
    }
}
