// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Text and numeric fields.
//!
//! Fields display caller-supplied text only: nothing is parsed, evaluated or edited here.

use super::edit::caret_window;
use super::numeric::NumericView;
use super::{Canvas, WidgetContext, cell, owned_blank};
use crate::{CellContent, CellGrid, GridRect, WidgetId, WidgetState};

/// `[text      ]` on the first row of `rect`, all owned by `id`. The text is left-aligned
/// and truncated. Needs a width of at least 2, else nothing is drawn.
pub fn draw_text_field(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    rect: GridRect,
    id: WidgetId,
    state: WidgetState,
    text: &str,
) {
    if rect.width() < 2 {
        return;
    }
    let style = ctx.style_of(id, state);
    let mut canvas = Canvas::new(grid, rect);
    let last = canvas.width() - 1;
    canvas.fill(0, canvas.width(), 0, owned_blank(style, id));
    let shown: String = text.chars().take((last - 1) as usize).collect();
    canvas.text(1, 0, &shown, style, Some(id));
    for (dx, glyph) in [(0, '['), (last, ']')] {
        canvas.put(dx, 0, cell(CellContent::Glyph(glyph), style, Some(id)));
    }
}

/// `[0.0 mm   ☰]`: a text field whose last two cells (`☰` and the closing `]`) are the
/// formula button, owned by `formula`. `field` owns the opening `[` and the text area.
/// `state` applies to both. Needs a width of at least 3, else nothing is drawn.
pub fn draw_numeric_field(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    rect: GridRect,
    field: WidgetId,
    formula: WidgetId,
    state: WidgetState,
    text: &str,
) {
    if rect.width() < 3 {
        return;
    }
    let field_style = ctx.style_of(field, state);
    let button_style = ctx.style_of(formula, state);
    let mut canvas = Canvas::new(grid, rect);
    let button = canvas.width() - 2;
    canvas.fill(0, button, 0, owned_blank(field_style, field));
    canvas.fill(button, 2, 0, owned_blank(button_style, formula));
    let shown: String = text.chars().take((button - 1) as usize).collect();
    canvas.text(1, 0, &shown, field_style, Some(field));
    canvas.put(
        0,
        0,
        cell(CellContent::Glyph('['), field_style, Some(field)),
    );
    canvas.put(
        button,
        0,
        cell(CellContent::Glyph('☰'), button_style, Some(formula)),
    );
    canvas.put(
        button + 1,
        0,
        cell(CellContent::Glyph(']'), button_style, Some(formula)),
    );
}

/// A numeric field showing an edited value (M2.23): the direction prefix, then the value and
/// its unit in the fixed-width text area, with the caret `▏` composed in while edited. The
/// text area never moves: the caret scrolls the text inside it (`README_UI.md` §5.11).
pub(crate) fn draw_numeric_view(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    rect: GridRect,
    field: WidgetId,
    formula: WidgetId,
    state: WidgetState,
    view: &NumericView,
) {
    if rect.width() < 3 {
        return;
    }
    draw_numeric_field(grid, ctx, rect, field, formula, state, "");
    let style = ctx.style_of(field, state);
    let mut canvas = Canvas::new(grid, rect);
    let button = canvas.width() - 2;
    let prefix = u32::try_from(view.prefix.chars().count()).unwrap_or(0);
    let area = button.saturating_sub(1 + prefix);
    if area == 0 {
        return;
    }
    canvas.text(1, 0, view.prefix, style, Some(field));
    let shown = caret_window(&view.body, &view.suffix, view.caret, area as usize);
    canvas.text(1 + prefix, 0, &shown, style, Some(field));
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::super::{WidgetContext, WidgetStyles};
    use super::*;
    use crate::InteractionState;

    fn r(c: u32, row: u32, w: u32, h: u32) -> GridRect {
        GridRect::new(c, row, w, h)
    }

    fn text_field(width: u32, text: &str, state: WidgetState) -> CellGrid {
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let mut g = grid(width, 1);
        draw_text_field(
            &mut g,
            &ctx(&styles, &inter),
            r(0, 0, width, 1),
            id(3),
            state,
            text,
        );
        g
    }

    fn numeric(width: u32, text: &str) -> CellGrid {
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let mut g = grid(width, 1);
        let c = ctx(&styles, &inter);
        draw_numeric_field(
            &mut g,
            &c,
            r(0, 0, width, 1),
            id(3),
            id(4),
            WidgetState::default(),
            text,
        );
        g
    }

    #[test]
    fn text_field_layout_and_ownership() {
        let g = text_field(12, "none", WidgetState::default());
        assert_eq!(text_row(&g, 0), "[none......]");
        assert_eq!(owner_row(&g, 0), "333333333333");
    }

    #[test]
    fn text_field_truncates_and_handles_small_rects() {
        assert_eq!(
            text_row(&text_field(6, "Normal to", WidgetState::default()), 0),
            "[Norm]"
        );
        assert_eq!(
            text_row(&text_field(2, "x", WidgetState::default()), 0),
            "[]"
        );
        assert_eq!(text_field(1, "x", WidgetState::default()), grid(1, 1));
        assert_eq!(
            text_row(&text_field(5, "", WidgetState::default()), 0),
            "[...]"
        );
    }

    #[test]
    fn text_field_disabled_and_focused_styles() {
        let styles = WidgetStyles::default();
        let off = WidgetState {
            disabled: true,
            ..WidgetState::default()
        };
        assert!(
            text_field(8, "a", off)
                .cells()
                .iter()
                .all(|c| c.style == styles.disabled)
        );
        let mut inter = InteractionState::default();
        inter.set_focused(Some(id(3)));
        let mut g = grid(8, 1);
        draw_text_field(
            &mut g,
            &ctx(&styles, &inter),
            r(0, 0, 8, 1),
            id(3),
            WidgetState::default(),
            "a",
        );
        assert!(g.cells().iter().all(|c| c.style == styles.focused));
    }

    #[test]
    fn numeric_field_has_formula_button() {
        let g = numeric(12, "0.0 mm");
        assert_eq!(text_row(&g, 0), "[0.0.mm...☰]");
        assert_eq!(owner_row(&g, 0), "333333333344");
    }

    #[test]
    fn numeric_field_truncates_and_handles_small_rects() {
        assert_eq!(text_row(&numeric(8, "123456789"), 0), "[12345☰]");
        assert_eq!(text_row(&numeric(3, "5"), 0), "[☰]");
        assert_eq!(owner_row(&numeric(3, "5"), 0), "344");
        assert_eq!(numeric(2, "5"), grid(2, 1));
        assert_eq!(numeric(0, "5"), grid(0, 1));
    }

    #[test]
    fn numeric_field_button_pressed_independently() {
        let styles = WidgetStyles::default();
        let mut inter = InteractionState::default();
        inter.press(id(4), WidgetState::default());
        let mut g = grid(12, 1);
        let c = WidgetContext {
            styles: &styles,
            interaction: &inter,
        };
        draw_numeric_field(
            &mut g,
            &c,
            r(0, 0, 12, 1),
            id(3),
            id(4),
            WidgetState::default(),
            "1",
        );
        assert_eq!(g.get(2, 0).unwrap().style, styles.normal);
        assert_eq!(g.get(10, 0).unwrap().style, styles.pressed);
        assert_eq!(g.get(11, 0).unwrap().style, styles.pressed);
    }

    fn view(prefix: &'static str, body: &str, caret: Option<usize>) -> NumericView {
        NumericView {
            prefix,
            body: body.chars().collect(),
            suffix: " mm".to_owned(),
            caret,
        }
    }

    fn edited(width: u32, v: &NumericView) -> CellGrid {
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let mut g = grid(width, 1);
        let c = ctx(&styles, &inter);
        draw_numeric_view(
            &mut g,
            &c,
            r(0, 0, width, 1),
            id(3),
            id(4),
            WidgetState::default(),
            v,
        );
        g
    }

    #[test]
    fn edited_field_keeps_fixed_cells_and_reference_widths() {
        let row = |g: &CellGrid| -> String {
            (0..g.cols())
                .map(|c| match g.get(c, 0).unwrap().content {
                    CellContent::Glyph(ch) => ch,
                    CellContent::Empty => ' ',
                })
                .collect()
        };
        assert_eq!(
            row(&edited(18, &view("+|", "0.0", None))),
            "[+|0.0 mm       ☰]"
        );
        assert_eq!(
            row(&edited(18, &view("-|", "5.5", Some(1)))),
            "[-|5▏.5 mm      ☰]"
        );
        assert_eq!(
            row(&edited(18, &view("+|", "", Some(0)))),
            "[+|▏ mm         ☰]"
        );
        // The limits of the references fill the 13-cell text area exactly.
        assert_eq!(
            row(&edited(18, &view("+|", "12345678.9", None))),
            "[+|12345678.9 mm☰]"
        );
        let deg = NumericView {
            suffix: " deg".to_owned(),
            ..view("+|", "1234567.8", None)
        };
        assert_eq!(row(&edited(18, &deg)), "[+|1234567.8 deg☰]");
        // Full text with the caret: the suffix is clipped, delimiter, prefix and `☰` stay.
        assert_eq!(
            row(&edited(18, &view("+|", "12345678.9", Some(10)))),
            "[+|12345678.9▏ m☰]"
        );
        assert_eq!(
            row(&edited(18, &view("-|", "12345678.9", Some(0)))),
            "[-|▏12345678.9 m☰]"
        );
        let g = edited(18, &view("+|", "12345678.9", Some(10)));
        assert_eq!(owner_row(&g, 0), "333333333333333344");
    }

    #[test]
    fn edited_field_without_direction_and_tiny_rects() {
        let plain = NumericView {
            suffix: String::new(),
            ..view("", "0.0", Some(3))
        };
        assert_eq!(text_row(&edited(12, &plain), 0), "[0.0▏.....☰]");
        assert_eq!(edited(2, &plain), grid(2, 1));
        for w in 3..6 {
            let _ = edited(w, &view("+|", "123", Some(3)));
        }
    }

    #[test]
    fn fields_are_deterministic() {
        assert_eq!(numeric(12, "0.0 mm"), numeric(12, "0.0 mm"));
        assert_ne!(numeric(12, "0.0 mm"), numeric(12, "1.0 mm"));
    }
}
