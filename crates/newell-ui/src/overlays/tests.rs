// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
use super::*;
use crate::widgets::WidgetStyles;
use crate::widgets::testing::{grid, id, owner_row, text_row};
use crate::{
    BorderAxis, GridMetrics, InteractionState, SharedBorder, UiPoint, WidgetState, hit_test,
};

fn r(c: u32, row: u32, w: u32, h: u32) -> GridRect {
    GridRect::new(c, row, w, h)
}

fn dims(g: GridRect) -> (u32, u32, u32, u32) {
    (g.col(), g.row(), g.width(), g.height())
}

fn inside(inner: GridRect, outer: GridRect) -> bool {
    inner.is_empty() || inner.intersection(&outer) == inner
}

fn overlay(v: u32, kind: OverlayKind, rect: GridRect) -> Overlay {
    Overlay {
        id: id(v),
        kind,
        rect,
    }
}

/// Composes `stack` over `g` with no content.
fn compose(g: &mut CellGrid, stack: &OverlayStack, bounds: GridRect) {
    let styles = WidgetStyles::default();
    let inter = InteractionState::default();
    let ctx = WidgetContext {
        styles: &styles,
        interaction: &inter,
    };
    compose_overlays(g, &ctx, stack, bounds, |_, _, _| {});
}

/// Fills `g` with a background of `#` cells owned by widget 1 with a shared border.
fn background(cols: u32, rows: u32) -> CellGrid {
    let mut g = grid(cols, rows);
    for row in 0..rows {
        for col in 0..cols {
            *g.get_mut(col, row).unwrap() = Cell {
                content: CellContent::Glyph('#'),
                style: StyleId::new(2),
                widget_id: Some(id(1)),
                shared_border: (col == 0).then_some(SharedBorder {
                    axis: BorderAxis::Vertical,
                    low: Some(id(1)),
                    high: Some(id(1)),
                }),
            };
        }
    }
    g
}

// ---- placement -------------------------------------------------------------------------

#[test]
fn popup_opens_below_and_left_aligned() {
    let b = r(0, 0, 40, 20);
    assert_eq!(dims(place_popup(r(5, 1, 6, 1), (10, 4), b)), (5, 2, 10, 4));
}

#[test]
fn popup_shifts_left_at_right_edge() {
    let b = r(0, 0, 40, 20);
    assert_eq!(
        dims(place_popup(r(35, 1, 4, 1), (10, 4), b)),
        (30, 2, 10, 4)
    );
}

#[test]
fn popup_flips_above_when_below_does_not_fit() {
    let b = r(0, 0, 40, 20);
    assert_eq!(
        dims(place_popup(r(5, 17, 6, 1), (10, 4), b)),
        (5, 13, 10, 4)
    );
}

#[test]
fn popup_truncates_to_larger_side_when_neither_fits() {
    let b = r(0, 0, 20, 10);
    // Anchor on row 3: 6 rows below, 3 above; popup wants 8 (limited to 8 by bounds).
    assert_eq!(dims(place_popup(r(0, 3, 4, 1), (5, 8), b)), (0, 4, 5, 6));
    // Anchor on row 7: 2 rows below, 7 above.
    assert_eq!(dims(place_popup(r(0, 7, 4, 1), (5, 20), b)), (0, 0, 5, 7));
}

#[test]
fn popup_is_limited_to_bounds_and_offset_bounds() {
    let b = r(10, 5, 8, 6);
    let p = place_popup(r(0, 0, 3, 1), (99, 99), b);
    assert!(inside(p, b));
    assert_eq!((p.width(), p.col()), (8, 10));
    let p = place_popup(r(500, 500, 3, 1), (4, 2), b);
    assert!(inside(p, b));
    assert_eq!(p.width(), 4);
}

#[test]
fn popup_in_degenerate_bounds_is_empty_and_never_panics() {
    for b in [r(0, 0, 0, 0), r(3, 3, 0, 5), r(3, 3, 5, 0)] {
        let p = place_popup(r(1, 1, 2, 1), (4, 4), b);
        assert!(p.is_empty(), "{b:?} -> {p:?}");
        assert!(inside(p, b));
    }
    let huge = r(u32::MAX - 1, u32::MAX - 1, u32::MAX, u32::MAX);
    let p = place_popup(huge, (u32::MAX, u32::MAX), r(0, 0, u32::MAX, u32::MAX));
    assert!(inside(p, r(0, 0, u32::MAX, u32::MAX)));
    let _ = center_modal(
        (u32::MAX, u32::MAX),
        r(u32::MAX, u32::MAX, u32::MAX, u32::MAX),
    );
}

#[test]
fn modal_is_centred_and_limited() {
    let b = r(0, 0, 21, 11);
    assert_eq!(dims(center_modal((11, 5), b)), (5, 3, 11, 5));
    assert_eq!(dims(center_modal((99, 99), b)), (0, 0, 21, 11));
    assert!(center_modal((4, 4), r(2, 2, 0, 0)).is_empty());
    assert_eq!(dims(center_modal((4, 4), r(10, 20, 8, 6))), (12, 21, 4, 4));
}

#[test]
fn modal_layout_splits_title_body_actions() {
    let l = modal_layout(r(2, 3, 10, 6));
    assert_eq!(dims(l.title), (2, 3, 10, 1));
    assert_eq!(dims(l.body), (2, 4, 10, 4));
    assert_eq!(dims(l.actions), (2, 8, 10, 1));
    let tiny = modal_layout(r(0, 0, 4, 2));
    assert_eq!((tiny.title.height(), tiny.actions.height()), (1, 1));
    assert!(tiny.body.is_empty());
    let one = modal_layout(r(0, 0, 4, 1));
    assert_eq!(one.title.height(), 1);
    assert!(one.actions.is_empty() && one.body.is_empty());
    let none = modal_layout(r(0, 0, 0, 0));
    assert!(none.title.is_empty() && none.body.is_empty() && none.actions.is_empty());
}

// ---- stack / dismissal ---------------------------------------------------------------------

#[test]
fn stack_open_dismiss_and_order() {
    let mut s = OverlayStack::default();
    assert!(s.is_empty() && s.top().is_none() && !s.has_modal());
    let a = overlay(1, OverlayKind::Popup, r(0, 0, 4, 4));
    let b = overlay(2, OverlayKind::Modal, r(1, 1, 4, 4));
    s.open(a);
    s.open(b);
    assert_eq!(s.len(), 2);
    assert!(s.has_modal() && s.is_open(id(1)));
    assert_eq!(s.top(), Some(&b));
    assert_eq!(s.iter().copied().collect::<Vec<_>>(), vec![a, b]);
    // Reopening moves to the top and keeps ids unique.
    s.open(a);
    assert_eq!(s.iter().copied().collect::<Vec<_>>(), vec![b, a]);
    assert!(s.dismiss(id(2)));
    assert!(!s.dismiss(id(2)));
    assert!(!s.has_modal());
    assert_eq!(s.dismiss_top(), Some(a));
    assert_eq!(s.dismiss_top(), None);
}

// ---- composition ------------------------------------------------------------------------

#[test]
fn popup_composition_frames_and_owns_surface() {
    let bounds = r(0, 0, 12, 6);
    let mut g = background(12, 6);
    let mut s = OverlayStack::default();
    s.open(overlay(7, OverlayKind::Popup, r(2, 1, 6, 4)));
    compose(&mut g, &s, bounds);
    assert_eq!(text_row(&g, 0), "############");
    assert_eq!(text_row(&g, 1), "##┌────┐####");
    assert_eq!(text_row(&g, 2), "##│....│####");
    assert_eq!(text_row(&g, 4), "##└────┘####");
    // Surface (border and interior) is owned by the overlay; the rest keeps its owner.
    assert_eq!(owner_row(&g, 1), "117777771111");
    assert_eq!(owner_row(&g, 2), "117777771111");
    assert_eq!(owner_row(&g, 5), "111111111111");
    // A popup does not block the background.
    assert_eq!(g.get(0, 0).unwrap().widget_id, Some(id(1)));
}

#[test]
fn covered_cells_lose_all_stale_metadata() {
    let bounds = r(0, 0, 12, 6);
    let mut g = background(12, 6);
    // Stale shared border sits in column 0; cover it.
    let mut s = OverlayStack::default();
    s.open(overlay(7, OverlayKind::Popup, r(0, 1, 5, 3)));
    compose(&mut g, &s, bounds);
    for row in 1..4 {
        for col in 0..5 {
            let c = g.get(col, row).unwrap();
            assert_eq!(c.widget_id, Some(id(7)), "({col},{row})");
            assert_eq!(c.shared_border, None, "({col},{row})");
            assert_eq!(c.style, StyleId::DEFAULT);
            assert_ne!(c.content, CellContent::Glyph('#'));
        }
    }
    // Uncovered neighbours keep theirs.
    assert!(g.get(0, 0).unwrap().shared_border.is_some());
    assert_eq!(g.get(5, 2).unwrap().content, CellContent::Glyph('#'));
}

#[test]
fn modal_blocks_background_but_preserves_its_content() {
    let bounds = r(0, 0, 12, 6);
    let mut g = background(12, 6);
    let before = g.clone();
    let mut s = OverlayStack::default();
    s.open(overlay(9, OverlayKind::Modal, center_modal((6, 4), bounds)));
    compose(&mut g, &s, bounds);
    assert_eq!(text_row(&g, 0), "############");
    assert_eq!(text_row(&g, 1), "###╔════╗###");
    assert_eq!(text_row(&g, 2), "###║....║###");
    for row in 0..6 {
        for col in 0..12 {
            let c = g.get(col, row).unwrap();
            let in_frame = (3..9).contains(&col) && (1..5).contains(&row);
            if in_frame {
                assert_eq!(c.widget_id, Some(id(9)));
            } else {
                // Visible content kept, interaction removed.
                let was = before.get(col, row).unwrap();
                assert_eq!((c.content, c.style), (was.content, was.style));
                assert_eq!((c.widget_id, c.shared_border), (None, None));
            }
        }
    }
}

#[test]
fn modal_blocks_only_inside_bounds() {
    let mut g = background(12, 6);
    let bounds = r(0, 0, 6, 6);
    let mut s = OverlayStack::default();
    s.open(overlay(9, OverlayKind::Modal, r(1, 1, 3, 3)));
    compose(&mut g, &s, bounds);
    assert_eq!(g.get(5, 5).unwrap().widget_id, None);
    assert_eq!(g.get(6, 5).unwrap().widget_id, Some(id(1)));
}

#[test]
fn overlay_ordering_later_covers_earlier() {
    let bounds = r(0, 0, 12, 6);
    let a = overlay(1, OverlayKind::Popup, r(1, 1, 6, 3));
    let b = overlay(2, OverlayKind::Popup, r(4, 2, 6, 3));
    let mut ab = OverlayStack::default();
    ab.open(a);
    ab.open(b);
    let mut ba = OverlayStack::default();
    ba.open(b);
    ba.open(a);
    let mut g1 = grid(12, 6);
    compose(&mut g1, &ab, bounds);
    let mut g2 = grid(12, 6);
    compose(&mut g2, &ba, bounds);
    // Overlap cell (5, 2): inside both frames.
    assert_eq!(g1.get(5, 2).unwrap().widget_id, Some(id(2)));
    assert_eq!(g2.get(5, 2).unwrap().widget_id, Some(id(1)));
    assert_ne!(g1, g2);
    // Same stack, same output.
    let mut g3 = grid(12, 6);
    compose(&mut g3, &ab, bounds);
    assert_eq!(g1, g3);
    // Top overlay's border is intact over the lower one.
    assert_eq!(g1.get(4, 2).unwrap().content, CellContent::Glyph('┌'));
}

#[test]
fn modal_over_popup_blocks_the_popup_too() {
    let bounds = r(0, 0, 12, 6);
    let mut g = grid(12, 6);
    let mut s = OverlayStack::default();
    s.open(overlay(1, OverlayKind::Popup, r(0, 0, 4, 3)));
    s.open(overlay(2, OverlayKind::Modal, r(6, 2, 5, 3)));
    compose(&mut g, &s, bounds);
    assert_eq!(g.get(1, 1).unwrap().widget_id, None);
    assert_eq!(g.get(1, 1).unwrap().content, CellContent::Empty);
    assert_eq!(g.get(0, 0).unwrap().content, CellContent::Glyph('┌'));
    assert_eq!(g.get(7, 3).unwrap().widget_id, Some(id(2)));
}

#[test]
fn overlay_is_clipped_to_bounds_and_grid() {
    let mut g = background(10, 5);
    let bounds = r(2, 1, 5, 3);
    let mut s = OverlayStack::default();
    s.open(overlay(7, OverlayKind::Popup, r(0, 0, 40, 40)));
    compose(&mut g, &s, bounds);
    assert_eq!(text_row(&g, 0), "##########");
    assert_eq!(text_row(&g, 1), "##┌───┐###");
    assert_eq!(text_row(&g, 2), "##│...│###");
    assert_eq!(text_row(&g, 3), "##└───┘###");
    assert_eq!(text_row(&g, 4), "##########");
    // Overlay larger than the grid itself: nothing outside is written.
    let mut g = grid(4, 3);
    let mut s = OverlayStack::default();
    s.open(overlay(7, OverlayKind::Modal, r(2, 1, 100, 100)));
    compose(&mut g, &s, r(0, 0, u32::MAX, u32::MAX));
    assert_eq!(g.cells().len(), 12);
    assert_eq!(g.get(1, 1).unwrap().widget_id, None);
    assert_eq!(g.get(2, 1).unwrap().content, CellContent::Glyph('╔'));
}

#[test]
fn zero_and_tiny_overlays() {
    let bounds = r(0, 0, 6, 4);
    for rect in [
        r(1, 1, 0, 0),
        r(1, 1, 0, 3),
        r(1, 1, 3, 0),
        r(1, 1, 1, 1),
        r(1, 1, 2, 1),
        r(1, 1, 1, 3),
    ] {
        let mut g = background(6, 4);
        let before = g.clone();
        let mut s = OverlayStack::default();
        s.open(overlay(7, OverlayKind::Popup, rect));
        let mut inner = None;
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let ctx = WidgetContext {
            styles: &styles,
            interaction: &inter,
        };
        compose_overlays(&mut g, &ctx, &s, bounds, |_, _, i| inner = Some(i));
        assert!(inner.unwrap().is_empty(), "{rect:?}");
        // Only cells of the rect may change.
        for row in 0..4 {
            for col in 0..6 {
                let in_rect =
                    (1..1 + rect.width()).contains(&col) && (1..1 + rect.height()).contains(&row);
                if !in_rect {
                    assert_eq!(g.get(col, row), before.get(col, row), "{rect:?}");
                }
            }
        }
    }
    // Empty grid.
    let mut g = grid(0, 0);
    let mut s = OverlayStack::default();
    s.open(overlay(7, OverlayKind::Modal, r(0, 0, 5, 5)));
    compose(&mut g, &s, r(0, 0, 5, 5));
    assert!(g.cells().is_empty());
}

#[test]
fn content_callback_gets_interior_in_draw_order() {
    let bounds = r(0, 0, 20, 10);
    let mut g = grid(20, 10);
    let mut s = OverlayStack::default();
    s.open(overlay(1, OverlayKind::Popup, r(0, 0, 6, 4)));
    s.open(overlay(2, OverlayKind::Modal, r(8, 2, 10, 6)));
    let styles = WidgetStyles::default();
    let inter = InteractionState::default();
    let ctx = WidgetContext {
        styles: &styles,
        interaction: &inter,
    };
    let mut seen = Vec::new();
    compose_overlays(&mut g, &ctx, &s, bounds, |_, o, inner| {
        seen.push((o.id, dims(inner)));
    });
    assert_eq!(seen, vec![(id(1), (1, 1, 4, 2)), (id(2), (9, 3, 8, 4))]);
}

#[test]
fn composition_is_deterministic_and_independent_of_previous_frame() {
    let bounds = r(0, 0, 14, 8);
    let mut s = OverlayStack::default();
    s.open(overlay(1, OverlayKind::Popup, r(1, 1, 6, 4)));
    s.open(overlay(2, OverlayKind::Modal, r(5, 2, 8, 5)));
    let mut clean = grid(14, 8);
    compose(&mut clean, &s, bounds);
    // Covered cells must not depend on what was there. Content outside stays as it was.
    let mut dirty = background(14, 8);
    compose(&mut dirty, &s, bounds);
    for row in 0..8 {
        for col in 0..14 {
            let covered = (1..7).contains(&col) && (1..5).contains(&row)
                || (5..13).contains(&col) && (2..7).contains(&row);
            if covered {
                assert_eq!(dirty.get(col, row), clean.get(col, row), "({col},{row})");
            }
        }
    }
    let mut again = background(14, 8);
    compose(&mut again, &s, bounds);
    assert_eq!(dirty, again);
}

#[test]
fn hit_testing_through_a_modal_composition() {
    let m = GridMetrics::new(6, 10).unwrap();
    let at = |col: u32, row: u32| UiPoint {
        x: col * 6 + 1,
        y: row * 10 + 1,
    };
    let bounds = r(0, 0, 12, 6);
    let mut g = background(12, 6);
    let mut s = OverlayStack::default();
    s.open(overlay(9, OverlayKind::Modal, r(3, 1, 6, 4)));
    compose(&mut g, &s, bounds);
    let hit = |col, row| hit_test(at(col, row), m, &g).unwrap().widget_id;
    assert_eq!(hit(4, 2), Some(id(9)));
    assert_eq!(hit(3, 1), Some(id(9)), "border belongs to the surface");
    assert_eq!(hit(0, 0), None, "blocked background");
    assert_eq!(hit(11, 5), None);
}

// ---- OK / Cancel ------------------------------------------------------------------------------

fn with_ctx<T>(f: impl FnOnce(&WidgetContext<'_>) -> T) -> T {
    let styles = WidgetStyles::default();
    let inter = InteractionState::default();
    f(&WidgetContext {
        styles: &styles,
        interaction: &inter,
    })
}

#[test]
fn ok_cancel_layout_and_widget_ids() {
    let mut g = grid(15, 1);
    with_ctx(|c| {
        draw_ok_cancel(
            &mut g,
            c,
            r(0, 0, 15, 1),
            id(4),
            id(5),
            WidgetState::default(),
        );
    });
    assert_eq!(text_row(&g, 0), "..OK...│Cancel.");
    // 15 wide: OK 7 | sep | Cancel 7.
    assert_eq!(owner_row(&g, 0), "4444444-5555555");
    let sep = g.get(7, 0).unwrap();
    assert_eq!(sep.widget_id, None);
    assert_eq!(
        sep.shared_border,
        Some(SharedBorder {
            axis: BorderAxis::Vertical,
            low: Some(id(4)),
            high: Some(id(5)),
        })
    );
}

#[test]
fn ok_cancel_hit_testing_including_separator_halves() {
    let m = GridMetrics::new(6, 10).unwrap();
    let mut g = grid(15, 1);
    with_ctx(|c| {
        draw_ok_cancel(
            &mut g,
            c,
            r(0, 0, 15, 1),
            id(4),
            id(5),
            WidgetState::default(),
        );
    });
    let hit = |x, y| hit_test(UiPoint { x, y }, m, &g).unwrap().widget_id;
    assert_eq!(hit(3, 5), Some(id(4)));
    assert_eq!(hit(14 * 6, 5), Some(id(5)));
    assert_eq!(hit(7 * 6, 5), Some(id(4)), "left half of separator");
    assert_eq!(hit(7 * 6 + 5, 5), Some(id(5)), "right half of separator");
}

#[test]
fn ok_cancel_disabled_ok_and_small_rects() {
    let disabled = WidgetState {
        disabled: true,
        ..WidgetState::default()
    };
    let mut g = grid(9, 1);
    with_ctx(|c| draw_ok_cancel(&mut g, c, r(0, 0, 9, 1), id(4), id(5), disabled));
    let styles = WidgetStyles::default();
    assert_eq!(g.get(0, 0).unwrap().style, styles.disabled);
    assert_eq!(
        g.get(0, 0).unwrap().widget_id,
        Some(id(4)),
        "identity is stable"
    );
    assert_eq!(g.get(8, 0).unwrap().style, styles.normal);
    // Narrowest drawable row, then too narrow / too short: nothing drawn.
    let mut g = grid(3, 1);
    with_ctx(|c| {
        draw_ok_cancel(
            &mut g,
            c,
            r(0, 0, 3, 1),
            id(4),
            id(5),
            WidgetState::default(),
        )
    });
    assert_eq!(owner_row(&g, 0), "4-5");
    for rect in [r(0, 0, 2, 1), r(0, 0, 0, 0), r(0, 0, 9, 0), r(50, 50, 9, 1)] {
        let mut g = grid(9, 1);
        with_ctx(|c| draw_ok_cancel(&mut g, c, rect, id(4), id(5), WidgetState::default()));
        assert_eq!(g, grid(9, 1), "{rect:?}");
    }
}

#[test]
fn ok_cancel_multi_row_owns_whole_rect_and_clears_stale_metadata() {
    let mut g = background(9, 3);
    with_ctx(|c| {
        draw_ok_cancel(
            &mut g,
            c,
            r(0, 0, 9, 3),
            id(4),
            id(5),
            WidgetState::default(),
        );
    });
    assert_eq!(owner_row(&g, 0), "4444-5555");
    assert_eq!(owner_row(&g, 2), "4444-5555");
    // Stale column-0 shared border from the background is gone.
    assert_eq!(g.get(0, 1).unwrap().shared_border, None);
    assert_eq!(text_row(&g, 1), ".OK.│Canc");
}

// ---- scroll state ---------------------------------------------------------------------------------

#[test]
fn scroll_state_bounds() {
    let mut s = ScrollState::default();
    s.scroll_by(-5, 10, 4);
    assert_eq!(s.offset, 0);
    s.scroll_by(3, 10, 4);
    assert_eq!(s.offset, 3);
    s.scroll_by(i32::MAX, 10, 4);
    assert_eq!(s.offset, 6, "stops at len - window");
    s.scroll_by(i32::MIN, 10, 4);
    assert_eq!(s.offset, 0);
    // Everything fits, or empty list / window.
    s.scroll_by(5, 3, 4);
    assert_eq!(s.offset, 0);
    s.scroll_by(5, 0, 0);
    assert_eq!(s.offset, 0);
    s.scroll_by(5, 5, 0);
    assert_eq!(s.offset, 5, "zero window may scroll to len");
    // Clamp after shrink.
    let mut s = ScrollState { offset: 99 };
    s.clamp(10, 4);
    assert_eq!(s.offset, 6);
    s.clamp(2, 4);
    assert_eq!(s.offset, 0);
}

#[test]
fn ensure_visible_scrolls_minimally() {
    let mut s = ScrollState { offset: 3 };
    s.ensure_visible(4, 10, 4);
    assert_eq!(s.offset, 3, "already visible");
    s.ensure_visible(1, 10, 4);
    assert_eq!(s.offset, 1);
    s.ensure_visible(8, 10, 4);
    assert_eq!(s.offset, 5);
    s.ensure_visible(999, 10, 4);
    assert_eq!(s.offset, 6, "past the end is the last item");
    s.ensure_visible(0, 0, 4);
    assert_eq!(s.offset, 0);
    s.ensure_visible(3, 10, 0);
    assert_eq!(s.offset, 0);
}

// ---- scroll list drawing --------------------------------------------------------------------------

const LABELS: [&str; 5] = ["Rim_Body", "Hub_Body", "Inner", "Outer", "Shaft"];

fn items(checked: &[usize]) -> Vec<ListItem<'static>> {
    LABELS
        .iter()
        .enumerate()
        .map(|(i, l)| ListItem {
            id: id(10 + u32::try_from(i).unwrap()),
            label: l,
            state: WidgetState {
                checked: checked.contains(&i),
                ..WidgetState::default()
            },
        })
        .collect()
}

fn draw_list(g: &mut CellGrid, rect: GridRect, offset: u32, items: &[ListItem<'_>]) {
    with_ctx(|c| {
        let list = ScrollList {
            items,
            scroll: ScrollState { offset },
            up: id(2),
            down: id(3),
        };
        draw_scroll_list(g, c, rect, &list);
    });
}

#[test]
fn list_draws_window_with_scrollbar() {
    let it = items(&[1]);
    let mut g = grid(12, 3);
    draw_list(&mut g, r(0, 0, 12, 3), 0, &it);
    assert_eq!(text_row(&g, 0), "□.Rim_Body.▲");
    assert_eq!(text_row(&g, 1), "■.Hub_Body.┃");
    assert_eq!(text_row(&g, 2), "□.Inner....▼");
    // Arrows own their cells; the thumb and track do not.
    assert_eq!(g.get(11, 0).unwrap().widget_id, Some(id(2)));
    assert_eq!(g.get(11, 1).unwrap().widget_id, None);
    assert_eq!(g.get(11, 2).unwrap().widget_id, Some(id(3)));
    // Up is disabled at the top; down is not.
    let styles = WidgetStyles::default();
    assert_eq!(g.get(11, 0).unwrap().style, styles.disabled);
    assert_eq!(g.get(11, 2).unwrap().style, styles.normal);
    // Whole row is owned by its checkbox.
    assert_eq!(g.get(9, 1).unwrap().widget_id, Some(id(11)));
}

#[test]
fn list_scrolled_and_clamped_offsets() {
    let it = items(&[]);
    let mut g = grid(12, 3);
    draw_list(&mut g, r(0, 0, 12, 3), 2, &it);
    assert_eq!(text_row(&g, 0), "□.Inner....▲");
    assert_eq!(text_row(&g, 2), "□.Shaft....▼");
    let styles = WidgetStyles::default();
    assert_eq!(
        g.get(11, 2).unwrap().style,
        styles.disabled,
        "at the bottom"
    );
    // Out-of-range offset behaves as the maximum.
    let mut g2 = grid(12, 3);
    draw_list(&mut g2, r(0, 0, 12, 3), u32::MAX, &it);
    assert_eq!(g, g2);
}

#[test]
fn thumb_tracks_position_and_proportion() {
    let it = items(&[]);
    let thumb_rows = |h: u32, offset: u32| -> String {
        let mut g = grid(6, h);
        draw_list(&mut g, r(0, 0, 6, h), offset, &it);
        (0..h)
            .map(|row| match g.get(5, row).unwrap().content {
                CellContent::Glyph(c) => c,
                CellContent::Empty => ' ',
            })
            .collect()
    };
    // 5 items, window 4: track 2, thumb 1.
    assert_eq!(thumb_rows(4, 0), "▲┃ ▼");
    assert_eq!(thumb_rows(4, 1), "▲ ┃▼");
    // Everything fits: thumb fills the track.
    assert_eq!(thumb_rows(6, 0), "▲┃┃┃┃▼");
    // Height 2: arrows only. Height 3: one-cell track.
    assert_eq!(thumb_rows(2, 0), "▲▼");
    assert_eq!(thumb_rows(3, 0), "▲┃▼");
    assert_eq!(thumb_rows(3, 2), "▲┃▼");
}

#[test]
fn thumb_reaches_the_end_exactly_at_max_offset() {
    let many: Vec<ListItem<'static>> = (0..100)
        .map(|i| ListItem {
            id: id(100 + i),
            label: "x",
            state: WidgetState::default(),
        })
        .collect();
    let mut g = grid(4, 12);
    draw_list(&mut g, r(0, 0, 4, 12), 88, &many);
    let glyph = |row| g.get(3, row).unwrap().content;
    assert_eq!(glyph(10), CellContent::Glyph('┃'), "last track row");
    assert_eq!(glyph(1), CellContent::Empty);
    let mut g = grid(4, 12);
    draw_list(&mut g, r(0, 0, 4, 12), 0, &many);
    assert_eq!(g.get(3, 1).unwrap().content, CellContent::Glyph('┃'));
    assert_eq!(g.get(3, 2).unwrap().content, CellContent::Empty);
}

#[test]
fn list_small_and_zero_regions() {
    let it = items(&[]);
    // Width 1: items only, no scrollbar.
    let mut g = grid(4, 3);
    draw_list(&mut g, r(0, 0, 1, 3), 0, &it);
    assert_eq!(text_row(&g, 0), "□...");
    assert_eq!(text_row(&g, 2), "□...");
    // Height 1: no scrollbar, one row.
    let mut g = grid(6, 2);
    draw_list(&mut g, r(0, 0, 6, 1), 1, &it);
    assert_eq!(text_row(&g, 0), "□.Hub_");
    assert_eq!(text_row(&g, 1), "......");
    // Zero regions and an empty list never panic and leave the outside alone.
    for rect in [r(0, 0, 0, 0), r(0, 0, 0, 3), r(0, 0, 5, 0), r(99, 99, 5, 5)] {
        let mut g = grid(6, 3);
        draw_list(&mut g, rect, 0, &it);
        assert_eq!(g, grid(6, 3), "{rect:?}");
    }
    let mut g = grid(6, 3);
    draw_list(&mut g, r(0, 0, 6, 3), 4, &[]);
    assert_eq!(text_row(&g, 0), "     ▲".replace(' ', "."));
    assert_eq!(text_row(&g, 2), ".....▼");
}

#[test]
fn list_is_clipped_to_the_grid_and_rect() {
    let it = items(&[]);
    // Rect hangs off the bottom-right of the grid: nothing outside is written, and the
    // scrollbar belongs to the rect's real right edge (off-grid here).
    let mut g = grid(6, 2);
    draw_list(&mut g, r(2, 1, 50, 50), 0, &it);
    assert_eq!(text_row(&g, 0), "......");
    assert_eq!(text_row(&g, 1), "..□.Ri");
    // Huge rect is cheap and bounded.
    let mut g = grid(4, 3);
    draw_list(&mut g, r(0, 0, u32::MAX, u32::MAX), 0, &it);
    assert_eq!(g.get(3, 2).unwrap().content, CellContent::Glyph('n'));
}

#[test]
fn list_overwrites_stale_cells_and_is_deterministic() {
    let it = items(&[0]);
    let rect = r(1, 0, 10, 4);
    let mut clean = grid(12, 4);
    draw_list(&mut clean, rect, 1, &it);
    let mut dirty = background(12, 4);
    draw_list(&mut dirty, rect, 1, &it);
    for row in 0..4 {
        for col in 1..11 {
            assert_eq!(dirty.get(col, row), clean.get(col, row), "({col},{row})");
        }
    }
    // Outside the rect the background is untouched.
    assert_eq!(dirty.get(0, 0), background(12, 4).get(0, 0));
    assert_eq!(dirty.get(11, 3), background(12, 4).get(11, 3));
    let mut again = background(12, 4);
    draw_list(&mut again, rect, 1, &it);
    assert_eq!(dirty, again);
}

#[test]
fn list_widget_ids_hit_test() {
    let m = GridMetrics::new(6, 10).unwrap();
    let it = items(&[]);
    let mut g = grid(12, 3);
    draw_list(&mut g, r(0, 0, 12, 3), 1, &it);
    let hit = |col: u32, row: u32| {
        hit_test(
            UiPoint {
                x: col * 6 + 2,
                y: row * 10 + 2,
            },
            m,
            &g,
        )
        .unwrap()
        .widget_id
    };
    assert_eq!(hit(4, 0), Some(id(11)));
    assert_eq!(hit(4, 2), Some(id(13)));
    assert_eq!(hit(11, 0), Some(id(2)));
    assert_eq!(hit(11, 2), Some(id(3)));
    assert_eq!(hit(11, 1), None);
}

#[test]
fn selection_panel_style_composition_end_to_end() {
    // Modal + title + list + OK/Cancel, the shape M2.12 will build on.
    let bounds = r(0, 0, 30, 12);
    let mut g = background(30, 12);
    let mut s = OverlayStack::default();
    s.open(overlay(
        50,
        OverlayKind::Modal,
        center_modal((20, 8), bounds),
    ));
    let it = items(&[1]);
    let styles = WidgetStyles::default();
    let inter = InteractionState::default();
    let ctx = WidgetContext {
        styles: &styles,
        interaction: &inter,
    };
    compose_overlays(&mut g, &ctx, &s, bounds, |g, _, inner| {
        let l = modal_layout(inner);
        crate::draw_label(g, l.title, "Selection", styles.normal);
        let list = ScrollList {
            items: &it,
            scroll: ScrollState::default(),
            up: id(2),
            down: id(3),
        };
        draw_scroll_list(g, &ctx, l.body, &list);
        draw_ok_cancel(g, &ctx, l.actions, id(4), id(5), WidgetState::default());
    });
    assert_eq!(text_row(&g, 2), "#####╔══════════════════╗#####");
    assert_eq!(text_row(&g, 3), "#####║Selection.........║#####");
    assert_eq!(text_row(&g, 4), "#####║□.Rim_Body.......▲║#####");
    assert_eq!(text_row(&g, 5), "#####║■.Hub_Body.......┃║#####");
    assert_eq!(text_row(&g, 8), "#####║...OK...│.Cancel..║#####");
    assert_eq!(g.get(8, 8).unwrap().widget_id, Some(id(4)));
    assert_eq!(g.get(20, 8).unwrap().widget_id, Some(id(5)));
    assert_eq!(g.get(0, 0).unwrap().widget_id, None);
}
