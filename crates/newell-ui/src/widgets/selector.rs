// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Segmented and cyclic selectors.
//!
//! The two are different presentations, chosen by intent and never by option count
//! (`docs/ui_design/README.md` §6): a [`Segmented`] selector keeps every alternative
//! visible; a [`Cyclic`] selector shows only the current value between `◀` and `▶`.

use super::{Canvas, WidgetContext, cell, centered, owned_blank, shared, text_len};
use crate::{BorderAxis, CellContent, CellGrid, GridRect, SharedBorder, WidgetId, WidgetState};

/// One alternative of a [`Segmented`] selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment<'a> {
    pub id: WidgetId,
    pub label: &'a str,
}

/// All alternatives visible, `active` marked structurally. `state.disabled` disables every
/// segment; the other flags are ignored (the active segment is shown as selected).
#[derive(Debug, Clone, Copy)]
pub struct Segmented<'a> {
    pub segments: &'a [Segment<'a>],
    pub active: usize,
    pub state: WidgetState,
}

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

/// Draws a segmented selector on the first row of `rect`.
///
/// Layout: `n` segments split the width with one separator column between neighbours, so
/// the rect must be at least `2n - 1` wide (else nothing is drawn). The `width - (n - 1)`
/// segment cells are shared evenly, the first `remainder` segments taking one extra cell.
/// Labels are centred and truncated. The outer left and right edges are not drawn: they
/// belong to the enclosing frame.
///
/// **Active-segment rule.** Separators are `│`. The separators touching the active segment
/// become `║`, so the active segment is boxed by double edges and no colour is needed to
/// see it. (A lone segment has no separators, hence no alternatives to distinguish.) The
/// active segment is also styled as selected.
///
/// Each segment's cells are owned by its [`Segment::id`]. Each separator carries
/// `shared_border` (`Vertical`, `low` = left segment, `high` = right segment) and no
/// `widget_id`, so pointing at either half routes to the adjacent segment.
pub fn draw_segmented(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    rect: GridRect,
    selector: &Segmented<'_>,
) {
    let segments = selector.segments;
    let n = u64::try_from(segments.len()).unwrap_or(u64::MAX);
    if n == 0 || u64::from(rect.width()) < 2 * n - 1 {
        return;
    }
    // width >= 2n - 1 and n <= width, so these fit in u32.
    let avail = u64::from(rect.width()) - (n - 1);
    let (base, extra) = (avail / n, avail % n);
    let sep_style = ctx.plain_style(selector.state);
    let mut canvas = Canvas::new(grid, rect);
    let mut dx = 0_u32;
    for (i, seg) in segments.iter().enumerate() {
        let width = u32::try_from(base + u64::from(u64::try_from(i).unwrap_or(u64::MAX) < extra))
            .unwrap_or(u32::MAX);
        let state = WidgetState {
            selected: i == selector.active,
            disabled: selector.state.disabled,
            ..WidgetState::default()
        };
        let style = ctx.style_of(seg.id, state);
        canvas.fill(dx, width, 0, owned_blank(style, seg.id));
        canvas.text(
            dx.saturating_add(centered(width, text_len(seg.label))),
            0,
            // Truncate to the segment so a long label cannot spill into the separator.
            &seg.label.chars().take(width as usize).collect::<String>(),
            style,
            Some(seg.id),
        );
        dx = dx.saturating_add(width);
        if let Some(next) = segments.get(i + 1) {
            let boxed = i == selector.active || i + 1 == selector.active;
            let glyph = if boxed { '║' } else { '│' };
            let border = SharedBorder {
                axis: BorderAxis::Vertical,
                low: Some(seg.id),
                high: Some(next.id),
            };
            canvas.put(dx, 0, shared(glyph, sep_style, border));
            dx = dx.saturating_add(1);
        }
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
    use crate::{GridMetrics, InteractionState, StyleId, UiPoint, hit_test};

    fn r(c: u32, row: u32, w: u32, h: u32) -> GridRect {
        GridRect::new(c, row, w, h)
    }

    fn seg<'a>(i: u32, label: &'a str) -> Segment<'a> {
        Segment { id: id(i), label }
    }

    fn draw_seg(width: u32, segs: &[Segment<'_>], active: usize, state: WidgetState) -> CellGrid {
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let mut g = grid(width, 1);
        let sel = Segmented {
            segments: segs,
            active,
            state,
        };
        draw_segmented(&mut g, &ctx(&styles, &inter), r(0, 0, width, 1), &sel);
        g
    }

    #[test]
    fn two_segments_show_all_alternatives() {
        let segs = [seg(1, "Solid"), seg(2, "Surface")];
        let g = draw_seg(15, &segs, 0, WidgetState::default());
        assert_eq!(text_row(&g, 0), ".Solid.║Surface");
        assert_eq!(owner_row(&g, 0), "1111111-2222222");
        let g = draw_seg(15, &segs, 1, WidgetState::default());
        assert_eq!(text_row(&g, 0), ".Solid.║Surface");
    }

    #[test]
    fn active_segment_is_boxed_by_double_edges() {
        let segs = [seg(1, "New"), seg(2, "Add"), seg(3, "Remove")];
        let rows: Vec<String> = (0..3)
            .map(|a| text_row(&draw_seg(25, &segs, a, WidgetState::default()), 0))
            .collect();
        assert_eq!(rows[0], "..New...".to_owned() + "║..Add..." + "│Remove.");
        assert_eq!(rows[1], "..New...".to_owned() + "║..Add..." + "║Remove.");
        assert_eq!(rows[2], "..New...".to_owned() + "│..Add..." + "║Remove.");
        // No active segment (out of range): all separators plain.
        let none = draw_seg(25, &segs, 9, WidgetState::default());
        assert_eq!(text_row(&none, 0), "..New...│..Add...│Remove.");
    }

    #[test]
    fn segment_widths_are_deterministic_and_cover_the_rect() {
        let segs = [seg(1, "A"), seg(2, "B"), seg(3, "C")];
        let g = draw_seg(10, &segs, 0, WidgetState::default());
        // avail = 8: widths 3, 3, 2.
        assert_eq!(owner_row(&g, 0), "111-222-33");
        assert_eq!(g, draw_seg(10, &segs, 0, WidgetState::default()));
    }

    #[test]
    fn separators_route_to_both_neighbours_via_hit_test() {
        let segs = [seg(1, "Solid"), seg(2, "Surface")];
        let g = draw_seg(15, &segs, 0, WidgetState::default());
        let sep = g.get(7, 0).unwrap();
        assert_eq!(sep.widget_id, None);
        assert_eq!(
            sep.shared_border,
            Some(SharedBorder {
                axis: BorderAxis::Vertical,
                low: Some(id(1)),
                high: Some(id(2))
            })
        );
        let m = GridMetrics::new(6, 10).unwrap();
        let at = |x| hit_test(UiPoint { x, y: 0 }, m, &g).unwrap().widget_id;
        assert_eq!(at(7 * 6 + 1), Some(id(1)));
        assert_eq!(at(7 * 6 + 4), Some(id(2)));
        assert_eq!(at(3), Some(id(1)));
        assert_eq!(at(10 * 6), Some(id(2)));
    }

    #[test]
    fn segment_styles_and_disabled() {
        let segs = [seg(1, "A"), seg(2, "B")];
        let styles = WidgetStyles::default();
        let g = draw_seg(7, &segs, 1, WidgetState::default());
        assert_eq!(g.get(0, 0).unwrap().style, styles.normal);
        assert_eq!(g.get(5, 0).unwrap().style, styles.selected);
        let off = WidgetState {
            disabled: true,
            ..WidgetState::default()
        };
        let g = draw_seg(7, &segs, 1, off);
        assert!(g.cells().iter().all(|c| c.style == styles.disabled));
        // Ids are still stamped.
        assert_eq!(g.get(0, 0).unwrap().widget_id, Some(id(1)));
    }

    #[test]
    fn segmented_small_and_degenerate_rects() {
        let segs = [seg(1, "New"), seg(2, "Add"), seg(3, "Remove")];
        assert_eq!(draw_seg(4, &segs, 0, WidgetState::default()), grid(4, 1));
        let tight = draw_seg(5, &segs, 0, WidgetState::default());
        assert_eq!(text_row(&tight, 0), "N║A│R");
        assert_eq!(draw_seg(8, &[], 0, WidgetState::default()), grid(8, 1));
        let one = draw_seg(6, &[seg(1, "Only")], 0, WidgetState::default());
        assert_eq!(text_row(&one, 0), ".Only.");
        assert!(one.cells().iter().all(|c| c.shared_border.is_none()));
        // Clipped by the grid: the rect extends past it.
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let mut g = grid(8, 1);
        let sel = Segmented {
            segments: &segs[..2],
            active: 0,
            state: WidgetState::default(),
        };
        draw_segmented(&mut g, &ctx(&styles, &inter), r(0, 0, 16, 1), &sel);
        assert_eq!(text_row(&g, 0), "..New...");
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
    fn selector_type_does_not_depend_on_option_count() {
        // Two choices, segmented: both visible, no arrows.
        let two = draw_seg(
            15,
            &[seg(1, "Solid"), seg(2, "Surface")],
            0,
            WidgetState::default(),
        );
        let text = text_row(&two, 0);
        assert!(text.contains("Solid") && text.contains("Surface"));
        assert!(!text.contains('◀') && !text.contains('▶'));
        // Two choices, cyclic: one value, arrows.
        let g = draw_cyc(
            20,
            &["Perspective", "Orthographic"],
            0,
            WidgetState::default(),
        );
        let text = text_row(&g, 0);
        assert!(text.contains("Perspective") && !text.contains("Orthographic"));
        assert!(text.starts_with('◀') && text.ends_with('▶'));
        // Three choices, cyclic and three choices, segmented.
        let g = draw_cyc(
            20,
            &["Blind", "Up to", "Through all"],
            2,
            WidgetState::default(),
        );
        assert!(text_row(&g, 0).contains("Through.all") && !text_row(&g, 0).contains("Blind"));
        let g = draw_seg(
            25,
            &[seg(1, "New"), seg(2, "Add"), seg(3, "Remove")],
            0,
            WidgetState::default(),
        );
        let text = text_row(&g, 0);
        assert!(text.contains("New") && text.contains("Add") && text.contains("Remove"));
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
    fn segmented_focus_and_press_style_only_the_owned_segment() {
        let segs = [seg(1, "Solid"), seg(2, "Surface")];
        let styles = WidgetStyles::default();
        let sel = Segmented {
            segments: &segs,
            active: 0,
            state: WidgetState::default(),
        };
        let mut inter = InteractionState::default();
        let mut g = grid(15, 1);
        inter.set_focused(Some(id(2)));
        draw_segmented(&mut g, &ctx(&styles, &inter), r(0, 0, 15, 1), &sel);
        // Focus shows on the focused, non-active segment; the active one stays selected.
        assert_eq!(g.get(0, 0).unwrap().style, styles.selected);
        assert_eq!(g.get(8, 0).unwrap().style, styles.focused);
        // Press outranks selection and focus, and only the pressed segment changes.
        assert!(inter.press(id(2), WidgetState::default()));
        draw_segmented(&mut g, &ctx(&styles, &inter), r(0, 0, 15, 1), &sel);
        assert_eq!(g.get(8, 0).unwrap().style, styles.pressed);
        assert_eq!(g.get(14, 0).unwrap().style, styles.pressed);
        assert_eq!(g.get(0, 0).unwrap().style, styles.selected);
        // Pressing the active segment shows pressed rather than selected.
        assert!(inter.press(id(1), WidgetState::default()));
        draw_segmented(&mut g, &ctx(&styles, &inter), r(0, 0, 15, 1), &sel);
        assert_eq!(g.get(0, 0).unwrap().style, styles.pressed);
        // Focus is still on segment 2, so it keeps the focused style.
        assert_eq!(g.get(8, 0).unwrap().style, styles.focused);
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
