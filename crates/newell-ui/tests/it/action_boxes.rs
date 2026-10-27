// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M2.22: the Revolve, Loft, Sweep and Boolean Action Boxes inside the shell, reached through
//! the CommandBar and the public pointer and keyboard API, compared with the `.txt` references.
#![allow(clippy::unwrap_used)]

use newell_ui::{
    ActionOperation, CellContent, CellGrid, GridMetrics, HitTestResult, ShellKey, ShellState,
    UiPoint, hit_test,
};

const COLS: u32 = 82;
const ROWS: u32 = 30;

fn shell() -> ShellState {
    ShellState::new(COLS, ROWS)
}

fn row_chars(grid: &CellGrid, row: u32) -> Vec<char> {
    (0..grid.cols())
        .map(|col| match grid.get(col, row).map(|c| c.content) {
            Some(CellContent::Glyph(c)) => c,
            _ => ' ',
        })
        .collect()
}

fn text(s: &ShellState) -> Vec<String> {
    let g = s.compose().unwrap();
    (0..g.rows())
        .map(|r| row_chars(&g, r).into_iter().collect())
        .collect()
}

fn shows(s: &ShellState, needle: &str) -> bool {
    text(s).iter().any(|r| r.contains(needle))
}

fn hit_on(grid: &CellGrid, needle: &str) -> Option<HitTestResult> {
    let pat: Vec<char> = needle.chars().collect();
    let metrics = GridMetrics::new(8, 18).unwrap();
    (0..grid.rows()).find_map(|row| {
        let col = row_chars(grid, row)
            .windows(pat.len())
            .position(|w| w == pat.as_slice())?;
        let col = u32::try_from(col).unwrap();
        hit_test(
            UiPoint {
                x: col * 8 + 3,
                y: row * 18 + 9,
            },
            metrics,
            grid,
        )
    })
}

fn click(s: &mut ShellState, needle: &str) -> bool {
    let hit = hit_on(&s.compose().unwrap(), needle);
    assert!(hit.is_some(), "{needle:?} must be on screen");
    s.click(hit)
}

/// Clicks the `[` of the field on the row that contains `label`.
fn click_field(s: &mut ShellState, label: &str) -> bool {
    let g = s.compose().unwrap();
    let metrics = GridMetrics::new(8, 18).unwrap();
    let (row, col) = (0..g.rows())
        .find_map(|r| {
            let chars = row_chars(&g, r);
            let line: String = chars.iter().collect();
            line.contains(label)
                .then(|| (r, chars.iter().position(|&c| c == '[').unwrap()))
        })
        .unwrap();
    let hit = hit_test(
        UiPoint {
            x: u32::try_from(col).unwrap() * 8 + 3,
            y: row * 18 + 9,
        },
        metrics,
        &g,
    );
    s.click(hit)
}

/// Opens the `menu`-th CommandBar popup and types the mnemonic of one of its items.
fn command(s: &mut ShellState, menu: usize, mnemonic: char) {
    s.key(ShellKey::CommandBar);
    for _ in 0..menu {
        s.key(ShellKey::Right);
    }
    s.key(ShellKey::Down);
    s.key(ShellKey::Char(mnemonic));
}

fn create(s: &mut ShellState, mnemonic: char) {
    command(s, 2, mnemonic);
}

/// The composed rows from `top` (a row starting with `╭─Title`) down to its `╰`, cropped to
/// the 31 columns of the box.
fn box_rows(s: &ShellState, title: &str) -> Vec<String> {
    let all = text(s);
    let top = format!("╭─{title}─");
    let found = all
        .iter()
        .enumerate()
        .find_map(|(i, r)| r.find(&top).map(|byte| (i, r[..byte].chars().count())));
    assert!(found.is_some(), "{title} box not shown");
    let (start, col) = found.unwrap();
    let mut out = Vec::new();
    for line in &all[start..] {
        let cut: String = line.chars().skip(col).take(31).collect();
        let last = cut.starts_with('╰');
        out.push(cut);
        if last {
            break;
        }
    }
    out
}

/// The complete boxes of a reference file.
fn reference(file: &str) -> Vec<Vec<String>> {
    let (mut out, mut cur) = (Vec::new(), Vec::new());
    for line in file.lines() {
        if line.starts_with('╭') {
            cur.clear();
        }
        if line.starts_with('╭') || !cur.is_empty() {
            cur.push(line.to_owned());
        }
        if line.starts_with('╰') {
            out.push(std::mem::take(&mut cur));
        }
    }
    out
}

const REVOLVE: &str = include_str!("../../../../docs/ui_design/Action_Box_Revolve.txt");
const LOFT: &str = include_str!("../../../../docs/ui_design/Action_Box_Loft.txt");
const SWEEP: &str = include_str!("../../../../docs/ui_design/Action_Box_Sweep.txt");
const BOOLEAN: &str = include_str!("../../../../docs/ui_design/Action_Box_Boolean.txt");

#[test]
fn the_command_bar_shows_each_operation_box_exactly() {
    let mut s = shell();
    assert_eq!(s.operation(), ActionOperation::Extrude);
    for (mnemonic, op, title, file) in [
        ('R', ActionOperation::Revolve, "Revolve", REVOLVE),
        ('L', ActionOperation::Loft, "Loft", LOFT),
        ('S', ActionOperation::Sweep, "Sweep", SWEEP),
    ] {
        create(&mut s, mnemonic);
        assert_eq!(s.operation(), op);
        assert_eq!(box_rows(&s, title), reference(file)[0], "{title}");
        assert!(!shows(&s, "╭─Extrude"));
    }
    command(&mut s, 4, 'B');
    assert_eq!(s.operation(), ActionOperation::Boolean);
    assert_eq!(box_rows(&s, "Boolean"), reference(BOOLEAN)[0]);
    create(&mut s, 'E');
    assert_eq!(s.operation(), ActionOperation::Extrude);
    assert!(shows(&s, "╭─Extrude"));
}

#[test]
fn clicking_the_menu_items_selects_the_operation() {
    let mut s = shell();
    assert!(click(&mut s, "Create"));
    assert!(click(&mut s, "[R]evolve"));
    assert_eq!(s.operation(), ActionOperation::Revolve);
    assert!(click(&mut s, "Modify"));
    assert!(click(&mut s, "Boolean"));
    assert_eq!(s.operation(), ActionOperation::Boolean);
}

#[test]
fn each_box_keeps_its_state_while_another_is_shown() {
    let mut s = shell();
    assert!(click(&mut s, "□ Symetric"));
    assert!(s.extrude().values.symmetric);
    create(&mut s, 'R');
    assert!(click(&mut s, "Full Revolve"));
    assert!(!s.revolve().values.full);
    create(&mut s, 'L');
    assert!(click(&mut s, "□ Guide"));
    // Back: every box shows what it held.
    create(&mut s, 'E');
    assert!(shows(&s, "■ Symetric") && s.extrude().values.symmetric);
    create(&mut s, 'R');
    assert!(shows(&s, "□ Full Revolve") && !s.revolve().values.full);
    create(&mut s, 'L');
    assert!(shows(&s, "■ Guide") && s.loft().values.guide);
    // Switching between Create boxes does not alter the shared header state.
    assert!(s.extrude().is_open() && s.revolve().is_open() && s.loft().is_open());
}

#[test]
fn ok_and_cancel_close_the_box_and_the_menu_reopens_it() {
    let mut s = shell();
    create(&mut s, 'S');
    assert!(click(&mut s, "□ Scale"));
    assert!(click(&mut s, "OK"));
    assert!(!s.sweep().is_open() && !shows(&s, "╭─Sweep"));
    assert_eq!(s.interaction().focused, None);
    // Reopening starts from defaults.
    create(&mut s, 'S');
    assert!(s.sweep().is_open() && shows(&s, "□ Scale"));
    assert!(click(&mut s, "Cancel"));
    assert!(!shows(&s, "╭─Sweep"));
    // The other boxes are unaffected.
    assert!(s.extrude().is_open());
    command(&mut s, 4, 'B');
    assert!(shows(&s, "╭─Boolean"));
    assert!(click(&mut s, "Cancel"));
    assert!(!s.boolean().is_open() && !shows(&s, "╭─Boolean"));
}

#[test]
fn selection_fields_and_formula_buttons_open_the_existing_overlays() {
    let mut s = shell();
    create(&mut s, 'R');
    assert!(click_field(&mut s, "Axis"));
    assert!(s.modal_open() && s.selection().is_open());
    assert!(s.key(ShellKey::Escape));
    assert!(!s.modal_open());
    assert!(click(&mut s, "Full Revolve"));
    assert!(click(&mut s, "☰"));
    assert!(s.formula().is_open());
    s.key(ShellKey::Escape);
    assert!(!s.modal_open());
    // The ☰ of a populated selection field manages the selection; the field itself edits it.
    command(&mut s, 4, 'B');
    assert!(click_field(&mut s, "Tools"));
    assert!(s.selection().is_open());
}

#[test]
fn keyboard_reaches_every_new_box() {
    let mut s = shell();
    create(&mut s, 'S');
    let before = s.sweep().clone();
    // Tab walks the tree rows first, then the Sweep controls; Enter activates the focused one.
    // The first Sweep control in the ring is the `Solid` segment, which changes nothing, so
    // keep going until a control changes the box or opens a modal.
    for _ in 0..80 {
        s.key(ShellKey::Tab);
        s.key(ShellKey::Enter);
        if *s.sweep() != before || s.modal_open() {
            break;
        }
    }
    assert!(*s.sweep() != before || s.modal_open());
}

#[test]
fn switching_the_operation_drops_a_panel_focus() {
    let mut s = shell();
    assert!(click(&mut s, "□ Symetric"));
    assert!(s.interaction().focused.is_some());
    create(&mut s, 'R');
    assert_eq!(s.interaction().focused, None);
}

#[test]
fn boxes_are_cropped_not_squeezed_on_a_narrow_grid() {
    let mut s = shell();
    command(&mut s, 4, 'B');
    for (cols, rows) in [(60, 30), (50, 12), (36, 8), (0, 0)] {
        s.resize(cols, rows);
        let g = s.compose().unwrap();
        assert_eq!((g.cols(), g.rows()), (cols, rows));
    }
    s.resize(COLS, ROWS);
    assert_eq!(box_rows(&s, "Boolean"), reference(BOOLEAN)[0]);
    assert_eq!(s.action_width(), 31);
}
