// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Label, separator, button and checkbox.

use super::{Canvas, WidgetContext, centered, owned_blank, text_len};
use crate::{
    BorderAxis, BorderLine, CellContent, CellGrid, GridRect, StyleId, WidgetId, WidgetState,
};

/// Left-aligned, non-interactive text on the first row of `rect`. Cells carry no
/// `widget_id`; cells beyond the text are untouched.
pub fn draw_label(grid: &mut CellGrid, rect: GridRect, text: &str, style: StyleId) {
    Canvas::new(grid, rect).text(0, 0, text, style, None);
}

/// A non-interactive line filling the first row (`Horizontal`) or first column
/// (`Vertical`) of `rect`: `─ │` for [`BorderLine::Single`], `═ ║` for `Double`.
pub fn draw_separator(
    grid: &mut CellGrid,
    rect: GridRect,
    axis: BorderAxis,
    line: BorderLine,
    style: StyleId,
) {
    let (glyph, horizontal) = match (axis, line) {
        (BorderAxis::Horizontal, BorderLine::Single) => ('─', true),
        (BorderAxis::Horizontal, BorderLine::Double) => ('═', true),
        (BorderAxis::Vertical, BorderLine::Single) => ('│', false),
        (BorderAxis::Vertical, BorderLine::Double) => ('║', false),
    };
    let sep = super::cell_with_ink(
        CellContent::Glyph(glyph),
        style,
        None,
        crate::InkRole::Contour,
    );
    let mut canvas = Canvas::new(grid, rect);
    if horizontal {
        canvas.fill(0, canvas.width(), 0, sep);
    } else {
        canvas.fill_col(0, sep);
    }
}

/// A button owning every cell of `rect`, with `label` centred on the middle row and
/// truncated to the width. Draws nothing for an empty rect.
pub fn draw_button(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    rect: GridRect,
    id: WidgetId,
    state: WidgetState,
    label: &str,
) {
    let style = ctx.style_of(id, state);
    let mut canvas = Canvas::new(grid, rect);
    for dy in 0..canvas.height() {
        canvas.fill(0, canvas.width(), dy, owned_blank(style, id));
    }
    let mid = canvas.height().saturating_sub(1) / 2;
    let dx = centered(canvas.width(), text_len(label));
    canvas.text(dx, mid, label, style, Some(id));
}

/// `□ label` / `■ label` on the first row; the glyph (not colour) shows
/// `state.checked`. The whole row of `rect` is owned by `id`.
pub fn draw_checkbox(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    rect: GridRect,
    id: WidgetId,
    state: WidgetState,
    label: &str,
) {
    let style = ctx.style_of(id, state);
    let mut canvas = Canvas::new(grid, rect);
    canvas.fill(0, canvas.width(), 0, owned_blank(style, id));
    let glyph = if state.checked { "■ " } else { "□ " };
    let dx = canvas.text(0, 0, glyph, style, Some(id));
    canvas.ink_at(0, 0, crate::InkRole::Action);
    canvas.text(dx, 0, label, style, Some(id));
}

#[cfg(test)]
mod tests {
    use super::super::WidgetStyles;
    use super::super::testing::*;
    use super::*;
    use crate::{Cell, InteractionState, SharedBorder};

    const ST: StyleId = StyleId::DEFAULT;

    fn r(c: u32, row: u32, w: u32, h: u32) -> GridRect {
        GridRect::new(c, row, w, h)
    }

    #[test]
    fn label_is_left_aligned_unowned_and_clipped() {
        let mut g = grid(10, 2);
        draw_label(&mut g, r(1, 0, 5, 1), "Hello world", StyleId::new(2));
        assert_eq!(text_row(&g, 0), ".Hello....");
        assert_eq!(owner_row(&g, 0), "----------");
        assert_eq!(g.get(1, 0).unwrap().style, StyleId::new(2));
        assert_eq!(text_row(&g, 1), "..........");
        let mut empty = grid(4, 1);
        draw_label(&mut empty, r(0, 0, 0, 1), "x", ST);
        draw_label(&mut empty, r(0, 0, 4, 0), "x", ST);
        draw_label(&mut empty, r(9, 9, 4, 4), "x", ST);
        assert_eq!(empty, grid(4, 1));
    }

    #[test]
    fn separators_horizontal_and_vertical() {
        let mut g = grid(6, 3);
        draw_separator(
            &mut g,
            r(1, 1, 4, 5),
            BorderAxis::Horizontal,
            BorderLine::Single,
            ST,
        );
        assert_eq!(text_row(&g, 1), ".────.");
        assert_eq!(text_row(&g, 2), "......");
        draw_separator(
            &mut g,
            r(0, 0, 5, 2),
            BorderAxis::Vertical,
            BorderLine::Double,
            ST,
        );
        assert_eq!(text_row(&g, 0), "║.....");
        assert_eq!(text_row(&g, 1), "║────.");
        assert_eq!(text_row(&g, 2), "......");
        draw_separator(
            &mut g,
            r(0, 2, 6, 1),
            BorderAxis::Horizontal,
            BorderLine::Double,
            ST,
        );
        assert_eq!(text_row(&g, 2), "══════");
        assert!(
            g.cells()
                .iter()
                .all(|c| c.widget_id.is_none() && c.shared_border.is_none())
        );
    }

    #[test]
    fn button_owns_rect_and_centres_label() {
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let mut g = grid(10, 4);
        draw_button(
            &mut g,
            &ctx(&styles, &inter),
            r(1, 0, 8, 3),
            id(1),
            WidgetState::default(),
            "OK",
        );
        assert_eq!(text_row(&g, 1), "....OK....");
        assert_eq!(owner_row(&g, 0), "-11111111-");
        assert_eq!(owner_row(&g, 2), "-11111111-");
        assert_eq!(owner_row(&g, 3), "----------");
        assert_eq!(g.get(4, 1).unwrap().widget_id, Some(id(1)));
    }

    #[test]
    fn button_truncates_and_handles_small_rects() {
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let c = ctx(&styles, &inter);
        let mut g = grid(6, 1);
        draw_button(
            &mut g,
            &c,
            r(0, 0, 4, 1),
            id(1),
            WidgetState::default(),
            "Cancel",
        );
        assert_eq!(text_row(&g, 0), "Canc..");
        assert_eq!(owner_row(&g, 0), "1111--");
        let mut e = grid(6, 1);
        draw_button(
            &mut e,
            &c,
            r(0, 0, 0, 1),
            id(1),
            WidgetState::default(),
            "x",
        );
        draw_button(
            &mut e,
            &c,
            r(0, 0, 3, 0),
            id(1),
            WidgetState::default(),
            "x",
        );
        assert_eq!(e, grid(6, 1));
    }

    #[test]
    fn button_states_change_style_not_cells() {
        let styles = WidgetStyles::default();
        let mut inter = InteractionState::default();
        let draw = |inter: &InteractionState, state| {
            let mut g = grid(6, 1);
            draw_button(
                &mut g,
                &ctx(&styles, inter),
                r(0, 0, 6, 1),
                id(1),
                state,
                "OK",
            );
            g
        };
        let normal = draw(&inter, WidgetState::default());
        let disabled = draw(
            &inter,
            WidgetState {
                disabled: true,
                ..WidgetState::default()
            },
        );
        assert_eq!(disabled.get(2, 0).unwrap().style, styles.disabled);
        assert_eq!(text_row(&normal, 0), text_row(&disabled, 0));
        assert_eq!(owner_row(&normal, 0), owner_row(&disabled, 0));
        inter.set_focused(Some(id(1)));
        assert_eq!(
            draw(&inter, WidgetState::default())
                .get(2, 0)
                .unwrap()
                .style,
            styles.focused
        );
        inter.press(id(1), WidgetState::default());
        assert_eq!(
            draw(&inter, WidgetState::default())
                .get(2, 0)
                .unwrap()
                .style,
            styles.pressed
        );
    }

    #[test]
    fn checkbox_glyph_shows_checked_state() {
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let c = ctx(&styles, &inter);
        let mut g = grid(14, 2);
        draw_checkbox(
            &mut g,
            &c,
            r(0, 0, 14, 2),
            id(2),
            WidgetState::default(),
            "Invert",
        );
        let checked = WidgetState {
            checked: true,
            ..WidgetState::default()
        };
        draw_checkbox(&mut g, &c, r(0, 1, 14, 1), id(3), checked, "Invert");
        assert_eq!(text_row(&g, 0), "□.Invert......");
        assert_eq!(text_row(&g, 1), "■.Invert......");
        // The whole first row is owned; a taller rect does not own extra rows.
        assert_eq!(owner_row(&g, 0), "22222222222222");
        assert_eq!(owner_row(&g, 1), "33333333333333");
    }

    #[test]
    fn checkbox_disabled_and_clipped() {
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let c = ctx(&styles, &inter);
        let mut g = grid(8, 1);
        let state = WidgetState {
            disabled: true,
            checked: true,
            ..WidgetState::default()
        };
        draw_checkbox(&mut g, &c, r(0, 0, 3, 1), id(2), state, "Invert");
        assert_eq!(text_row(&g, 0), "■.I.....");
        assert_eq!(owner_row(&g, 0), "222-----");
        assert_eq!(g.get(0, 0).unwrap().style, styles.disabled);
        let mut e = grid(8, 1);
        draw_checkbox(&mut e, &c, r(0, 0, 0, 1), id(2), state, "x");
        assert_eq!(e, grid(8, 1));
    }

    #[test]
    fn primitives_overwrite_previous_cells() {
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let c = ctx(&styles, &inter);
        let mut dirty = grid(8, 1);
        for col in 0..8 {
            *dirty.get_mut(col, 0).unwrap() = Cell {
                ink: crate::InkRole::Text,
                content: CellContent::Glyph('#'),
                style: StyleId::new(3),
                widget_id: Some(id(9)),
                shared_border: Some(SharedBorder {
                    axis: BorderAxis::Vertical,
                    low: None,
                    high: None,
                }),
            };
        }
        let mut clean = grid(8, 1);
        for g in [&mut dirty, &mut clean] {
            draw_checkbox(g, &c, r(0, 0, 6, 1), id(2), WidgetState::default(), "A");
        }
        assert_eq!(text_row(&dirty, 0)[..6], text_row(&clean, 0)[..6]);
        assert_eq!(&dirty.cells()[..6], &clean.cells()[..6]);
        assert_eq!(dirty.get(7, 0).unwrap().widget_id, Some(id(9)));
    }
}
