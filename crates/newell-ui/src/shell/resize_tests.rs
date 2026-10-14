// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M2.14: whole-cell responsive layout, panel widths and separator dragging.
use super::compose::{NAV_WIDTH, PANEL_WIDTH};
use super::*;
use crate::{CellContent, CellGrid, GridMetrics, UiPoint, hit_test};

fn glyph(g: &CellGrid, col: u32, row: u32) -> Option<char> {
    match g.get(col, row)?.content {
        CellContent::Glyph(c) => Some(c),
        CellContent::Empty => None,
    }
}

/// Cells whose separator is `which`, in reading order.
fn separator_cells(s: &ShellState, which: Separator) -> Vec<(u32, u32)> {
    let (cols, rows) = s.size();
    (0..rows)
        .flat_map(|r| (0..cols).map(move |c| (c, r)))
        .filter(|&(c, r)| s.separator_at(c, r) == Some(which))
        .collect()
}

#[test]
fn resize_recomputes_the_grid_dimensions() {
    let mut s = ShellState::new(82, 25);
    assert!(s.resize(120, 40));
    assert_eq!(s.size(), (120, 40));
    let g = s.compose().unwrap();
    assert_eq!((g.cols(), g.rows()), (120, 40));
    assert!(!s.resize(120, 40), "same size is not a change");
}

#[test]
fn reference_narrow_and_wide_layouts() {
    let reference = ShellState::new(82, 25);
    assert_eq!(
        (reference.nav_width(), reference.action_width()),
        (NAV_WIDTH, PANEL_WIDTH)
    );
    let wide = ShellState::new(240, 60);
    assert_eq!(
        (wide.nav_width(), wide.action_width()),
        (NAV_WIDTH, PANEL_WIDTH)
    );
    // Narrow: the NavigationPanel is capped at half the grid, the ActionPanel at the viewport.
    let narrow = ShellState::new(30, 25);
    assert_eq!(narrow.nav_width(), 15);
    assert_eq!(narrow.action_width(), 14);
    let tight = ShellState::new(20, 25);
    assert_eq!((tight.nav_width(), tight.action_width()), (10, 9));
}

#[test]
fn layout_is_a_pure_function_of_state() {
    let mut a = ShellState::new(82, 25);
    a.set_nav_width(30);
    let mut b = ShellState::new(10, 5);
    b.set_nav_width(30); // clamped while small ...
    b.resize(82, 25);
    // ... so the wish differs, but equal wishes on equal grids compose identically.
    let mut c = ShellState::new(82, 25);
    c.set_nav_width(30);
    assert_eq!(a.compose().unwrap(), c.compose().unwrap());
    assert_eq!(a.compose().unwrap(), a.compose().unwrap());
    assert_ne!(
        a.compose().unwrap(),
        ShellState::new(82, 25).compose().unwrap()
    );
}

#[test]
fn resize_path_does_not_matter() {
    let direct = ShellState::new(100, 30);
    let mut routed = ShellState::new(82, 25);
    routed.resize(0, 0);
    routed.resize(9, 3);
    routed.resize(100, 30);
    assert_eq!(direct.compose().unwrap(), routed.compose().unwrap());
    assert_eq!(direct, routed);
}

#[test]
fn nav_width_is_clamped() {
    let mut s = ShellState::new(82, 25);
    assert!(s.set_nav_width(1));
    assert_eq!(s.nav_width(), 8);
    assert!(s.set_nav_width(999));
    assert_eq!(s.nav_width(), 41, "at most half the grid");
    assert!(!s.set_nav_width(41), "no change reports false");
    // A grid smaller than the minimum caps it at half.
    let mut small = ShellState::new(10, 25);
    small.set_nav_width(1);
    assert_eq!(small.nav_width(), 5);
}

#[test]
fn action_width_is_clamped() {
    let mut s = ShellState::new(82, 25);
    assert!(s.set_action_width(0));
    assert_eq!(s.action_width(), 16);
    assert!(s.set_action_width(u32::MAX));
    assert_eq!(s.action_width(), 56, "at most the viewport interior");
    let mut small = ShellState::new(24, 25);
    small.set_action_width(0);
    assert_eq!(
        small.action_width(),
        11,
        "interior of 11 is below the minimum"
    );
}

#[test]
fn nav_separator_is_the_panels_right_border_on_the_body_rows() {
    let s = ShellState::new(82, 25);
    let cells = separator_cells(&s, Separator::Navigation);
    assert!(!cells.is_empty());
    assert!(cells.iter().all(|&(c, _)| c == 24));
    assert_eq!(cells.first().map(|&(_, r)| r), Some(4));
    assert_eq!(cells.last().map(|&(_, r)| r), Some(22));
    // The cell is a drawn vertical border junction in the composed grid.
    assert!(glyph(&s.compose().unwrap(), 24, 10).is_some());
    assert_eq!(s.separator_at(23, 10), None);
    assert_eq!(s.separator_at(24, 3), None, "above the body");
}

#[test]
fn action_separator_is_the_panels_left_border() {
    let s = ShellState::new(82, 25);
    let cells = separator_cells(&s, Separator::Action);
    assert!(!cells.is_empty());
    // The viewport interior ends at column 81; a 27-wide panel starts at 54.
    assert!(cells.iter().all(|&(c, _)| c == 54));
}

#[test]
fn dragging_snaps_to_character_columns() {
    let mut s = ShellState::new(82, 25);
    assert!(s.drag_separator(Separator::Navigation, 30));
    assert_eq!(s.nav_width(), 31);
    let g = s.compose().unwrap();
    assert!(
        separator_cells(&s, Separator::Navigation)
            .iter()
            .all(|&(c, _)| c == 30)
    );
    assert!(glyph(&g, 30, 10).is_some());
    assert!(
        !s.drag_separator(Separator::Navigation, 30),
        "same column: no change"
    );
    // Far past either edge clamps instead of failing.
    s.drag_separator(Separator::Navigation, u32::MAX);
    assert_eq!(s.nav_width(), 41);
    s.drag_separator(Separator::Navigation, 0);
    assert_eq!(s.nav_width(), 8);

    assert!(s.drag_separator(Separator::Action, 50));
    assert_eq!(s.action_width(), 31, "interior ends at column 81");
}

#[test]
fn dragging_changes_the_composed_regions() {
    let mut s = ShellState::new(82, 25);
    let before = s.compose().unwrap();
    s.drag_separator(Separator::Navigation, 40);
    assert_ne!(before, s.compose().unwrap());
    s.drag_separator(Separator::Navigation, 24);
    assert_eq!(
        before,
        s.compose().unwrap(),
        "dragging back restores the layout"
    );
}

#[test]
fn panels_cannot_be_dragged_while_a_modal_or_menu_owns_input() {
    let mut s = ShellState::new(82, 25);
    assert!(s.key(ShellKey::MenuBar));
    assert_eq!(s.separator_at(24, 10), None);
    assert!(s.key(ShellKey::MenuBar));
    assert_eq!(s.separator_at(24, 10), Some(Separator::Navigation));
}

#[test]
fn state_is_stable_across_resize() {
    let mut s = ShellState::new(82, 25);
    s.set_nav_width(32);
    s.set_action_width(30);
    assert!(s.key(ShellKey::Tab));
    let focused = s.interaction().focused;
    assert!(focused.is_some());
    let original = s.compose().unwrap();
    let widths = (s.nav_width(), s.action_width());

    s.resize(20, 8);
    assert_ne!(s.nav_width(), widths.0, "shrunk while small");
    assert_eq!(s.interaction().focused, focused);
    s.resize(82, 25);
    assert_eq!(
        (s.nav_width(), s.action_width()),
        widths,
        "the user's widths come back"
    );
    assert_eq!(s.interaction().focused, focused);
    assert_eq!(s.compose().unwrap(), original);
}

#[test]
fn open_menu_survives_a_resize() {
    let mut s = ShellState::new(82, 25);
    assert!(s.key(ShellKey::Alt('F')));
    s.resize(30, 12);
    assert!(s.compose().is_ok());
    s.resize(82, 25);
    assert!(s.compose().is_ok());
}

#[test]
fn tiny_and_zero_dimensions_stay_in_bounds() {
    let mut s = ShellState::new(82, 25);
    for (cols, rows) in [
        (0, 0),
        (1, 1),
        (0, 50),
        (50, 0),
        (2, 2),
        (3, 3),
        (5, 100),
        (100, 5),
        (7, 7),
    ] {
        s.resize(cols, rows);
        let g = s.compose().unwrap();
        assert_eq!((g.cols(), g.rows()), (cols, rows));
        assert!(s.nav_width() <= cols / 2 + cols % 2, "{cols}x{rows}");
        assert!(s.nav_width() <= cols);
        for row in 0..rows {
            for col in 0..cols {
                let _ = s.separator_at(col, row);
            }
        }
        s.drag_separator(Separator::Navigation, 3);
        s.drag_separator(Separator::Action, 3);
        s.key(ShellKey::Tab);
        s.click(None);
        assert_eq!(s.compose().unwrap().cols(), cols);
    }
}

#[test]
fn hit_testing_follows_the_resized_grid() {
    let mut s = ShellState::new(82, 25);
    s.resize(60, 20);
    let g = s.compose().unwrap();
    let m = GridMetrics::new(6, 10).unwrap();
    let inside = hit_test(
        UiPoint {
            x: 59 * 6 + 3,
            y: 19 * 10 + 5,
        },
        m,
        &g,
    );
    assert!(inside.is_some());
    let outside = hit_test(UiPoint { x: 60 * 6, y: 5 }, m, &g);
    assert!(outside.is_none(), "pixels past the new grid hit nothing");
}
