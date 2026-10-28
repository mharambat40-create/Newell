// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M2.18: the MenuBar popups against `docs/ui_design/Application_Menu_Bar.txt`, read from the
//! file itself so a change of the reference fails here.
#![allow(clippy::unwrap_used)]

use newell_ui::{CellContent, CellGrid, ShellKey, ShellState, ThemeChoice};

const REFERENCE: &str = include_str!("../../../../docs/ui_design/Application_Menu_Bar.txt");

fn shell() -> ShellState {
    ShellState::new(82, 30)
}

fn row_chars(grid: &CellGrid, row: u32) -> Vec<char> {
    (0..grid.cols())
        .map(|col| match grid.get(col, row).map(|c| c.content) {
            Some(CellContent::Glyph(c)) => c,
            _ => ' ',
        })
        .collect()
}

/// The lines of the reference box whose top edge starts with `top` (`╭─View`), up to and
/// including its `╰` edge.
fn reference_box(top: &str) -> Vec<Vec<char>> {
    let lines: Vec<&str> = REFERENCE.lines().collect();
    let start = lines.iter().position(|l| l.starts_with(top)).unwrap();
    let len = lines[start..]
        .iter()
        .position(|l| l.starts_with('╰'))
        .unwrap();
    lines[start..=start + len]
        .iter()
        .map(|l| l.chars().collect())
        .collect()
}

/// The open popup of `grid`, found by its top edge, compared row for row with `expected`.
fn assert_popup(grid: &CellGrid, expected: &[Vec<char>]) {
    let top = expected[0].iter().collect::<String>();
    let found = (0..grid.rows()).find_map(|r| {
        let text: String = row_chars(grid, r).into_iter().collect();
        text.find(&top)
            .map(|byte| (r, text[..byte].chars().count()))
    });
    assert!(found.is_some(), "popup {top:?} not on screen");
    let (row, col) = found.unwrap();
    for (i, want) in expected.iter().enumerate() {
        let got: Vec<char> = row_chars(grid, row + u32::try_from(i).unwrap())
            .into_iter()
            .skip(col)
            .take(want.len())
            .collect();
        assert_eq!(&got, want, "row {i} of {top:?}");
    }
}

fn keys(s: &mut ShellState, keys: &[ShellKey]) {
    for &k in keys {
        s.key(k);
    }
}

fn open_view(s: &mut ShellState) {
    keys(s, &[ShellKey::Alt('v')]);
}

#[test]
fn menu_bar_titles_use_the_reference_separators() {
    let g = shell().compose().unwrap();
    let row: String = row_chars(&g, 1).into_iter().collect();
    assert!(row.starts_with("║ File │ Edit │ View │ Help "), "{row:?}");
    assert!(row.ends_with("[-] [□] [X] ║"));
    let command: String = row_chars(&g, 3).into_iter().collect();
    assert!(
        command.starts_with("║ Workbench │ Datum │ Create │ Shape │ Modify │ Pattern │ Utilities ")
    );
}

#[test]
fn file_edit_and_help_popups_match_the_reference() {
    for (key, top) in [('f', "╭─File"), ('e', "╭─Edit"), ('h', "╭─Help")] {
        let mut s = shell();
        s.key(ShellKey::Alt(key));
        assert_popup(&s.compose().unwrap(), &reference_box(top));
    }
}

#[test]
fn view_popup_matches_the_reference_in_its_default_state() {
    let mut s = shell();
    open_view(&mut s);
    assert_popup(&s.compose().unwrap(), &reference_box("╭─View"));
}

/// The reference's comma-separated state lists that follow the View box.
fn states(prefix: &str) -> Vec<String> {
    REFERENCE
        .lines()
        .find(|l| l.starts_with(prefix))
        .unwrap()
        .split(", ")
        .map(str::to_owned)
        .collect()
}

#[test]
fn ui_scale_labels_keep_the_original_rendering_factors_and_stop_at_endpoints() {
    let labels = states("│ UI Scale    80%");
    assert_eq!(
        labels,
        [
            "│ UI Scale    80% ▶ │",
            "│ UI Scale ◀ 100% ▶ │",
            "│ UI Scale ◀ 120% ▶ │",
            "│ UI Scale ◀ 140%   │",
        ]
    );
    let mut s = shell();
    assert_eq!(
        s.scale_factor_percent(),
        80,
        "100% label uses former 80% factor"
    );
    open_view(&mut s);
    keys(&mut s, &[ShellKey::Down, ShellKey::Down]); // UI Scale row
    for (index, factor) in [(2, 100), (3, 120), (3, 120)] {
        s.key(ShellKey::Right);
        assert_eq!(s.scale_factor_percent(), factor);
        let row: String = row_chars(&s.compose().unwrap(), 5).into_iter().collect();
        assert!(row.contains(&labels[index]), "{factor}: {row:?}");
    }
    for (index, factor) in [(2, 100), (1, 80), (0, 60), (0, 60)] {
        s.key(ShellKey::Left);
        assert_eq!(s.scale_factor_percent(), factor);
        let row: String = row_chars(&s.compose().unwrap(), 5).into_iter().collect();
        assert!(row.contains(&labels[index]), "{factor}: {row:?}");
    }
    // Left at 80% keeps the former 60% rendering factor instead of wrapping.
    assert_eq!(s.scale_factor_percent(), 60);
}

#[test]
fn projection_and_shading_states_match_the_reference() {
    let projections = states("◀  Perspective");
    let shadings = states("◀    Points");
    assert_eq!((projections.len(), shadings.len()), (2, 7));
    let mut s = shell();
    open_view(&mut s);
    for want in projections.iter().cycle().take(2) {
        s.key(ShellKey::Right);
        let row: String = row_chars(&s.compose().unwrap(), 3).into_iter().collect();
        assert!(row.contains(&format!("│ {want} │")), "{want:?} in {row:?}");
    }
    s.key(ShellKey::Down);
    // Startup is `Hidden Line` (index 3); Right visits the next seven cyclic states.
    for i in [4, 5, 6, 0, 1, 2, 3] {
        s.key(ShellKey::Right);
        let row: String = row_chars(&s.compose().unwrap(), 4).into_iter().collect();
        assert!(
            row.contains(&format!("│ {} │", shadings[i])),
            "{i}: {row:?}"
        );
    }
    // Left visits the same states in reverse and wraps back to Hidden Line.
    for i in [2, 1, 0, 6, 5, 4, 3] {
        s.key(ShellKey::Left);
        let row: String = row_chars(&s.compose().unwrap(), 4).into_iter().collect();
        assert!(
            row.contains(&format!("│ {} │", shadings[i])),
            "{i}: {row:?}"
        );
    }
}

#[test]
fn theme_row_switches_and_keeps_the_view_popup_open() {
    let states = states("[ ◐ Light ]");
    assert_eq!(states, ["[ ◐ Light ]", "[ ◑ Dark  ]"]);
    let mut s = shell();
    assert_eq!(s.theme(), ThemeChoice::Dark);
    open_view(&mut s);
    keys(&mut s, &[ShellKey::Down, ShellKey::Down, ShellKey::Down]);
    let row: String = row_chars(&s.compose().unwrap(), 6).into_iter().collect();
    assert!(row.contains("│ Theme [ ◑ Dark  ] │"), "{row:?}");
    s.key(ShellKey::Enter);
    assert_eq!(s.theme(), ThemeChoice::Light);
    let row: String = row_chars(&s.compose().unwrap(), 6).into_iter().collect();
    assert!(row.contains("│ Theme [ ◐ Light ] │"), "{row:?}");
    s.key(ShellKey::Enter);
    assert_eq!(s.theme(), ThemeChoice::Dark);
    let row: String = row_chars(&s.compose().unwrap(), 6).into_iter().collect();
    assert!(row.contains("│ Theme [ ◑ Dark  ] │"), "{row:?}");
}

#[test]
fn fullscreen_item_leaves_one_request_and_closes_the_menu() {
    let mut s = shell();
    assert!(!s.take_fullscreen_request());
    open_view(&mut s);
    keys(&mut s, &[ShellKey::Down; 13]);
    s.key(ShellKey::Enter);
    assert!(s.take_fullscreen_request());
    assert!(!s.take_fullscreen_request(), "taken once");
    assert_eq!(s.compose().unwrap(), shell().compose().unwrap());
}

#[test]
fn the_view_popup_fits_the_minimum_grid() {
    // 82 x 30 holds the whole 15-row View popup under the MenuBar.
    let mut s = shell();
    open_view(&mut s);
    let g = s.compose().unwrap();
    assert_popup(&g, &reference_box("╭─View"));
    assert_eq!(s.size(), (82, 30));
}
