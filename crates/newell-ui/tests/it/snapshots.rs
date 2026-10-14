// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M2.15: logical reference-state snapshots of the composed shell.
//!
//! Each state is reached through the public pointer API (`click` on a hit-tested cell), the
//! same path the application uses, then the flat `CellGrid` is rendered as text:
//!
//! - the glyph layer: one line per row, `Empty` cells as spaces, so box drawing and text
//!   are readable like `docs/ui_design/`;
//! - the style layer: one character per cell, `.` for the default style, else the style id
//!   in base 36 (`#` above 35), so a lost or moved highlight shows up as a diff.
//!
//! Nothing is normalised: the mock data (document name, fps, progress) is constant, so every
//! cell is deterministic. Widget ids are not snapshotted; hit-testing is covered elsewhere.
#![allow(clippy::unwrap_used)]

use newell_ui::{CellContent, CellGrid, GridMetrics, HitTestResult, ShellState, UiPoint, hit_test};

/// The reference size of `Interface_Exemple.txt`.
const COLS: u32 = 82;
const ROWS: u32 = 25;

fn shell() -> ShellState {
    ShellState::new(COLS, ROWS)
}

fn render(grid: &CellGrid) -> String {
    let (mut glyphs, mut styles) = (String::new(), String::new());
    for row in 0..grid.rows() {
        for col in 0..grid.cols() {
            let Some(cell) = grid.get(col, row) else {
                continue;
            };
            glyphs.push(match cell.content {
                CellContent::Glyph(c) => c,
                CellContent::Empty => ' ',
            });
            styles.push(match cell.style.get() {
                0 => '.',
                n => char::from_digit(u32::from(n), 36).unwrap_or('#'),
            });
        }
        glyphs.push('\n');
        styles.push('\n');
    }
    format!("{glyphs}\n-- styles --\n{styles}")
}

fn snapshot(s: &ShellState) -> String {
    render(&s.compose().unwrap())
}

fn row_text(grid: &CellGrid, row: u32) -> Vec<char> {
    (0..grid.cols())
        .map(|col| match grid.get(col, row).map(|c| c.content) {
            Some(CellContent::Glyph(c)) => c,
            _ => ' ',
        })
        .collect()
}

/// The hit-test result at the first occurrence of `needle` in reading order.
fn hit_on(grid: &CellGrid, needle: &str) -> Option<HitTestResult> {
    let pat: Vec<char> = needle.chars().collect();
    let metrics = GridMetrics::new(6, 10).unwrap();
    (0..grid.rows()).find_map(|row| {
        let col = row_text(grid, row)
            .windows(pat.len())
            .position(|w| w == pat.as_slice())?;
        let col = u32::try_from(col).unwrap();
        hit_test(
            UiPoint {
                x: col * 6 + 3,
                y: row * 10 + 5,
            },
            metrics,
            grid,
        )
    })
}

fn click(s: &mut ShellState, needle: &str) {
    let grid = s.compose().unwrap();
    let hit = hit_on(&grid, needle);
    assert!(hit.is_some(), "{needle:?} must be on screen");
    assert!(s.click(hit), "click on {needle:?} must change the state");
}

#[test]
fn base_shell() {
    insta::assert_snapshot!(snapshot(&shell()));
}

#[test]
fn file_menu_open() {
    let mut s = shell();
    click(&mut s, "File");
    insta::assert_snapshot!(snapshot(&s));
}

#[test]
fn create_menu_open() {
    let mut s = shell();
    click(&mut s, "Create");
    insta::assert_snapshot!(snapshot(&s));
}

#[test]
fn extrude_new() {
    let mut s = shell();
    click(&mut s, "New");
    insta::assert_snapshot!(snapshot(&s));
}

#[test]
fn extrude_add() {
    let mut s = shell();
    click(&mut s, "Add");
    insta::assert_snapshot!(snapshot(&s));
}

#[test]
fn extrude_remove() {
    let mut s = shell();
    click(&mut s, "Remove");
    insta::assert_snapshot!(snapshot(&s));
}

#[test]
fn starting_offset_disabled_then_enabled() {
    let mut s = shell();
    let disabled = snapshot(&s);
    insta::assert_snapshot!("starting_offset_disabled", disabled);
    click(&mut s, "Starting Offset");
    let enabled = snapshot(&s);
    insta::assert_snapshot!("starting_offset_enabled", enabled);
    assert_ne!(disabled, enabled);
}

#[test]
fn selection_panel_open() {
    let mut s = shell();
    click(&mut s, "Add");
    click(&mut s, "Merge with all");
    click(&mut s, "[none");
    assert!(s.modal_open());
    insta::assert_snapshot!(snapshot(&s));
}

#[test]
fn formula_editor_open() {
    let mut s = shell();
    click(&mut s, "☰");
    assert!(s.modal_open());
    insta::assert_snapshot!(snapshot(&s));
}

#[test]
fn features_tree_collapsed() {
    let mut s = shell();
    click(&mut s, "[-] Features");
    insta::assert_snapshot!(snapshot(&s));
}

/// The expanded Features tree is the reference state shown by `base_shell`: re-expanding the
/// collapsed root restores its glyphs exactly, so it needs no snapshot of its own. Only the
/// glyph layer is compared: the clicked row legitimately keeps the focus style.
#[test]
fn features_tree_expanded_is_the_base_shell() {
    let mut s = shell();
    let glyphs = |s: &ShellState| snapshot(s).split("-- styles --").next().map(str::to_owned);
    let base = glyphs(&s);
    click(&mut s, "[-] Features");
    assert_ne!(glyphs(&s), base);
    click(&mut s, "[+] Features");
    assert_eq!(glyphs(&s), base);
}

#[test]
fn narrow_layout() {
    insta::assert_snapshot!(snapshot(&ShellState::new(40, 20)));
}

#[test]
fn wide_layout() {
    insta::assert_snapshot!(snapshot(&ShellState::new(140, 30)));
}
