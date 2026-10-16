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
fn ui_scale_visits_the_four_reference_states_and_wraps() {
    let want = states("◀  60%");
    assert_eq!(want.len(), 4);
    let mut s = shell();
    open_view(&mut s);
    keys(&mut s, &[ShellKey::Down, ShellKey::Down]); // UI Scale row
    // 100 % is the default; Right visits 120, then wraps to 60, 80, 100.
    for percent in [120, 60, 80, 100] {
        s.key(ShellKey::Right);
        assert_eq!(s.preferred_scale_percent(), percent);
        let g = s.compose().unwrap();
        let idx = [60, 80, 100, 120]
            .iter()
            .position(|&p| p == percent)
            .unwrap();
        let row: String = row_chars(&g, 5).into_iter().collect();
        assert!(
            row.contains(&format!("│ UI Scale {} │", want[idx])),
            "{percent}: {row:?}"
        );
    }
    // Left walks back and wraps the other way.
    s.key(ShellKey::Left);
    assert_eq!(s.preferred_scale_percent(), 80);
}

#[test]
fn projection_and_shading_states_match_the_reference() {
    let projections = states("◀  Perspective");
    let shadings = states("◀    Points");
    assert_eq!((projections.len(), shadings.len()), (2, 6));
    let mut s = shell();
    open_view(&mut s);
    for want in projections.iter().cycle().skip(1).take(2) {
        s.key(ShellKey::Right);
        let row: String = row_chars(&s.compose().unwrap(), 3).into_iter().collect();
        assert!(row.contains(&format!("│ {want} │")), "{want:?} in {row:?}");
    }
    s.key(ShellKey::Down);
    // Default is `Shaded` (index 4); Right goes Flat line, then wraps to Points...
    for i in [5, 0, 1, 2, 3, 4] {
        s.key(ShellKey::Right);
        let row: String = row_chars(&s.compose().unwrap(), 4).into_iter().collect();
        assert!(
            row.contains(&format!("│ {} │", shadings[i])),
            "{i}: {row:?}"
        );
    }
}

#[test]
fn theme_row_switches_and_leaves_the_cells_untouched() {
    let states = states("[ ◐ Light ]");
    assert_eq!(states, ["[ ◐ Light ]", "[ ◑ Dark  ]"]);
    let mut s = shell();
    assert_eq!(s.theme(), ThemeChoice::Light);
    let base = s.compose().unwrap();
    open_view(&mut s);
    keys(&mut s, &[ShellKey::Down, ShellKey::Down, ShellKey::Down]);
    let row: String = row_chars(&s.compose().unwrap(), 6).into_iter().collect();
    assert!(row.contains("│ Theme [ ◐ Light ] │"), "{row:?}");
    s.key(ShellKey::Enter);
    assert_eq!(s.theme(), ThemeChoice::Dark);
    assert_eq!(
        s.compose().unwrap(),
        base,
        "a theme change touches no cell: it is a uniform"
    );
    open_view(&mut s);
    let row: String = row_chars(&s.compose().unwrap(), 6).into_iter().collect();
    assert!(row.contains("│ Theme [ ◑ Dark  ] │"), "{row:?}");
    keys(&mut s, &[ShellKey::Down, ShellKey::Down, ShellKey::Down]);
    s.key(ShellKey::Enter);
    assert_eq!(s.theme(), ThemeChoice::Light);
}

#[test]
fn fullscreen_item_leaves_one_request_and_closes_the_menu() {
    let mut s = shell();
    assert!(!s.take_fullscreen_request());
    open_view(&mut s);
    keys(&mut s, &[ShellKey::Down; 11]);
    s.key(ShellKey::Enter);
    assert!(s.take_fullscreen_request());
    assert!(!s.take_fullscreen_request(), "taken once");
    assert_eq!(s.compose().unwrap(), shell().compose().unwrap());
}

#[test]
fn the_view_popup_fits_the_minimum_grid() {
    // 82 x 30 holds the whole 14-row View popup under the MenuBar.
    let mut s = shell();
    open_view(&mut s);
    let g = s.compose().unwrap();
    assert_popup(&g, &reference_box("╭─View"));
    assert_eq!(s.size(), (82, 30));
}
