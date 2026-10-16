// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M2.20: double-click Rename recognition, the Features / Elements divider and the playhead,
//! driven through the pointer API the application uses.

#![allow(clippy::unwrap_used)]

use std::time::Duration;

use newell_app::controller::{DOUBLE_CLICK, Press, ShellController};
use newell_ui::{CellContent, CellGrid, DEFAULT_K, GridMetrics, ShellKey, UiPoint};

const W: u32 = 4 * DEFAULT_K;
const H: u32 = 9 * DEFAULT_K;
const SURFACE: (u32, u32) = (82 * W, 28 * H);

fn controller() -> ShellController {
    ShellController::new(82, 28, GridMetrics::from_k(DEFAULT_K).unwrap()).unwrap()
}

fn at(col: u32, row: u32) -> Option<UiPoint> {
    Some(UiPoint {
        x: col * W + W / 2,
        y: row * H + H / 2,
    })
}

fn row_text(g: &CellGrid, row: u32) -> String {
    (0..g.cols())
        .map(|c| match g.get(c, row).unwrap().content {
            CellContent::Glyph(ch) => ch,
            CellContent::Empty => ' ',
        })
        .collect()
}

/// `(col, row)` of the first occurrence of `needle`.
fn find(c: &ShellController, needle: &str) -> (u32, u32) {
    let pat: Vec<char> = needle.chars().collect();
    let g = c.grid();
    (0..g.rows())
        .find_map(|row| {
            let line: Vec<char> = row_text(g, row).chars().collect();
            line.windows(pat.len())
                .position(|w| w == pat.as_slice())
                .map(|col| (u32::try_from(col).unwrap(), row))
        })
        .unwrap()
}

fn ms(v: u64) -> Duration {
    Duration::from_millis(v)
}

/// A press at the first cell of `needle` at time `t`.
fn press_on(c: &mut ShellController, needle: &str, t: Duration) {
    let (col, row) = find(c, needle);
    c.pointer_moved(at(col, row));
    c.set_time(t);
    assert!(matches!(c.press(SURFACE, true), Press::Shell(_)));
}

fn rename_open(c: &ShellController) -> bool {
    c.shell().rename().is_open()
}

#[test]
fn double_click_threshold_is_500_ms() {
    assert_eq!(DOUBLE_CLICK, ms(500));
}

#[test]
fn two_presses_within_500_ms_on_the_same_name_open_rename() {
    let mut c = controller();
    press_on(&mut c, "Hub_Body", ms(1_000));
    assert!(!rename_open(&c), "the first press only selects");
    press_on(&mut c, "Hub_Body", ms(1_300));
    assert!(rename_open(&c));
    assert_eq!(c.shell().rename().draft(), "Hub_Body");
    assert!(row_text(c.grid(), 11).contains("╭─Rename"));
}

#[test]
fn exactly_500_ms_counts_and_501_ms_does_not() {
    let mut c = controller();
    press_on(&mut c, "Rim_Body", ms(0));
    press_on(&mut c, "Rim_Body", ms(500));
    assert!(rename_open(&c));

    let mut c = controller();
    press_on(&mut c, "Rim_Body", ms(0));
    press_on(&mut c, "Rim_Body", ms(501));
    assert!(!rename_open(&c));
    // The late press starts a new sequence: a third press in time completes it.
    press_on(&mut c, "Rim_Body", ms(700));
    assert!(rename_open(&c));
}

#[test]
fn a_different_row_or_an_intervening_press_breaks_the_double_click() {
    let mut c = controller();
    press_on(&mut c, "Hub_Body", ms(0));
    press_on(&mut c, "Rim_Body", ms(100));
    assert!(!rename_open(&c), "different row");

    let mut c = controller();
    press_on(&mut c, "Hub_Body", ms(0));
    press_on(&mut c, "Fillet_Inner", ms(100));
    press_on(&mut c, "Hub_Body", ms(200));
    assert!(!rename_open(&c), "another target in between");

    // A press on nothing, and on a window control, also ends the sequence.
    let mut c = controller();
    press_on(&mut c, "Hub_Body", ms(0));
    c.pointer_moved(at(40, 12));
    c.set_time(ms(50));
    c.press(SURFACE, true);
    press_on(&mut c, "Hub_Body", ms(100));
    assert!(!rename_open(&c), "a press on empty space");

    let mut c = controller();
    press_on(&mut c, "Hub_Body", ms(0));
    let (col, row) = find(&c, "[X]");
    c.pointer_moved(at(col, row));
    c.set_time(ms(50));
    assert!(matches!(c.press(SURFACE, true), Press::Window(_)));
    press_on(&mut c, "Hub_Body", ms(100));
    assert!(!rename_open(&c), "a press on a window control");
}

#[test]
fn markers_checkboxes_headers_and_the_playhead_are_not_rename_targets() {
    for needle in [
        "[-] Hub_Features",
        "□ Sketch_Profile",
        "■ Rim_Body",
        "Solid Bodies",
        "Surfaces(0)",
        "[+]Construct",
        "Features",
        "[-]Elements",
    ] {
        // Both presses hit the same cell, even if the first one moved the row.
        let mut c = controller();
        let (col, row) = find(&c, needle);
        for t in [0, 100] {
            c.pointer_moved(at(col, row));
            c.set_time(ms(t));
            c.press(SURFACE, true);
        }
        assert!(!rename_open(&c), "{needle}");
    }
    let mut c = controller();
    for t in [0, 100] {
        c.pointer_moved(at(10, 12));
        c.set_time(ms(t));
        c.press(SURFACE, true);
    }
    assert!(!rename_open(&c), "the playhead row");
}

#[test]
fn a_double_click_is_consumed_and_the_modal_edits_confirms_and_cancels() {
    let mut c = controller();
    press_on(&mut c, "Hub_Body", ms(0));
    press_on(&mut c, "Hub_Body", ms(100));
    assert!(rename_open(&c));
    for ch in "_2".chars() {
        assert!(c.key(ShellKey::Char(ch)).grid);
    }
    assert!(c.key(ShellKey::Enter).grid);
    assert!(!rename_open(&c));
    assert!(row_text(c.grid(), 20).contains("╰╴□ Hub_Body_2"));

    // Escape keeps the name; the row can be renamed again after a fresh double-click.
    press_on(&mut c, "Hub_Body_2", ms(1_000));
    press_on(&mut c, "Hub_Body_2", ms(1_100));
    assert!(rename_open(&c));
    c.key(ShellKey::Char('x'));
    assert!(c.key(ShellKey::Escape).grid);
    assert!(row_text(c.grid(), 20).contains("╰╴□ Hub_Body_2 "));

    // The consumed double-click does not chain: press, press (opens), cancel, press again.
    press_on(&mut c, "Hub_Body_2", ms(2_000));
    press_on(&mut c, "Hub_Body_2", ms(2_100));
    c.key(ShellKey::Escape);
    press_on(&mut c, "Hub_Body_2", ms(2_200));
    assert!(!rename_open(&c));
}

#[test]
fn the_first_press_selects_and_focuses_the_row() {
    let mut c = controller();
    press_on(&mut c, "Extrude_Base", ms(0));
    assert!(c.shell().interaction().focused.is_some());
    assert!(!rename_open(&c));
}

#[test]
fn dragging_the_divider_follows_the_pointer_row_by_row() {
    let mut c = controller();
    // The divider is the shared border row 16 (columns 1..=23).
    c.pointer_moved(at(10, 16));
    assert!(matches!(c.press(SURFACE, true), Press::Shell(_)));
    assert!(c.dragging());
    assert!(c.pointer_moved(at(10, 12)).grid);
    assert!(row_text(c.grid(), 12).starts_with("╠═══"));
    assert!(row_text(c.grid(), 13).starts_with("║[-]Elements"));
    // Within the same row nothing changes, wherever in the cell the pointer is.
    assert!(
        !c.pointer_moved(Some(UiPoint {
            x: 10 * W,
            y: 12 * H + H - 1
        }))
        .grid
    );
    c.pointer_moved(Some(UiPoint { x: 0, y: u32::MAX }));
    assert!(row_text(c.grid(), 23).starts_with("╠═══"), "clamped");
    c.pointer_released();
    assert!(!c.dragging());
    assert!(!c.pointer_moved(at(10, 8)).grid, "no drag after release");
}

#[test]
fn the_divider_keeps_its_proportion_through_a_window_resize() {
    // 82 x 30 is the smallest composed grid: body 24 rows.
    let mut c = ShellController::new(82, 30, GridMetrics::from_k(DEFAULT_K).unwrap()).unwrap();
    c.pointer_moved(at(10, 16));
    c.press((82 * W, 30 * H), true);
    c.pointer_moved(at(10, 12));
    c.pointer_released();
    let panes = |c: &ShellController| row_text(c.grid(), 12).starts_with("╠═══");
    assert!(panes(&c), "9-row Features pane");
    // Body 24 -> 34 rows: 9 * 34 / 24 = 12.75, so the pane is 13 rows and its border row 16.
    c.resize(82 * W, 40 * H);
    assert!(row_text(c.grid(), 16).starts_with("╠═══"));
    assert!(!panes(&c));
    c.resize(82 * W, 30 * H);
    assert!(panes(&c), "the user's split comes back");
}

#[test]
fn dragging_the_playhead_row_moves_only_the_separator() {
    let mut c = controller();
    let before: Vec<String> = (5..15).map(|r| row_text(c.grid(), r)).collect();
    c.pointer_moved(at(10, 12)); // the playhead row
    assert!(matches!(c.press(SURFACE, true), Press::Shell(_)));
    assert!(c.dragging());
    assert!(
        c.pointer_moved(at(10, 9)).grid,
        "onto Sketch_Hole: after Hub_Features"
    );
    assert!(row_text(c.grid(), 11).contains("│ ───────────────────"));
    c.pointer_released();
    let after: Vec<String> = (5..15).map(|r| row_text(c.grid(), r)).collect();
    let strip = |rows: &[String]| {
        rows.iter()
            .map(|r| r.chars().take(25).collect::<String>())
            .filter(|r| !r.contains("───"))
            .collect::<Vec<_>>()
    };
    assert_eq!(strip(&before), strip(&after), "nesting is untouched");
}

#[test]
fn modal_and_drag_state_do_not_leak_into_each_other() {
    let mut c = controller();
    press_on(&mut c, "Hub_Body", ms(0));
    press_on(&mut c, "Hub_Body", ms(10));
    assert!(rename_open(&c));
    // While the modal is open the divider and playhead cannot be grabbed.
    c.pointer_moved(at(10, 16));
    c.press(SURFACE, true);
    assert!(!c.dragging());
    c.pointer_moved(at(10, 12));
    c.press(SURFACE, true);
    assert!(!c.dragging());
}
