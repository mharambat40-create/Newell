// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M2.19: the CommandBar popups against `docs/ui_design/Command_Bar.txt`, read from the file
//! itself so a change of the reference fails here.
#![allow(clippy::unwrap_used)]

use newell_ui::{
    CellContent, CellGrid, GridMetrics, HitTestResult, ShellKey, ShellState, UiPoint, hit_test,
};

const REFERENCE: &str = include_str!("../../../../docs/ui_design/Command_Bar.txt");
const TITLES: [&str; 7] = [
    "Workbench",
    "Datum",
    "Create",
    "Shape",
    "Modify",
    "Pattern",
    "Utilities",
];

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

/// The reference box titled `title`, up to and including its `╰` edge.
fn reference_box(title: &str) -> Vec<Vec<char>> {
    let top = format!("╭─{title}─");
    let lines: Vec<&str> = REFERENCE.lines().collect();
    let start = lines.iter().position(|l| l.starts_with(&top)).unwrap();
    let len = lines[start..]
        .iter()
        .position(|l| l.starts_with('╰'))
        .unwrap();
    lines[start..=start + len]
        .iter()
        .map(|l| l.chars().collect())
        .collect()
}

/// `(row, col)` of the popup whose top edge is `top`.
fn find(grid: &CellGrid, top: &str) -> Option<(u32, u32)> {
    (0..grid.rows()).find_map(|r| {
        let text: String = row_chars(grid, r).into_iter().collect();
        text.find(top)
            .map(|byte| (r, u32::try_from(text[..byte].chars().count()).unwrap()))
    })
}

fn open(s: &mut ShellState, index: usize) {
    s.key(ShellKey::CommandBar);
    for _ in 0..index {
        s.key(ShellKey::Right);
    }
    s.key(ShellKey::Down);
}

fn hit_on(grid: &CellGrid, needle: &str) -> Option<HitTestResult> {
    let pat: Vec<char> = needle.chars().collect();
    let metrics = GridMetrics::new(6, 10).unwrap();
    (0..grid.rows()).find_map(|row| {
        let col = row_chars(grid, row)
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

fn click(s: &mut ShellState, needle: &str) -> bool {
    let hit = hit_on(&s.compose().unwrap(), needle);
    assert!(hit.is_some(), "{needle:?} must be on screen");
    s.click(hit)
}

#[test]
fn every_popup_matches_the_reference_and_is_centred() {
    for (i, title) in TITLES.iter().enumerate() {
        let mut s = shell();
        open(&mut s, i);
        let want = reference_box(title);
        let (h, w) = (want.len() as u32, want[0].len() as u32);
        assert_eq!(w, 19, "{title}");
        let g = s.compose().unwrap();
        let top: String = want[0].iter().collect();
        let found = find(&g, &top);
        assert!(found.is_some(), "{title} not on screen");
        let (row, col) = found.unwrap();
        assert_eq!((col, row), ((82 - w) / 2, (30 - h) / 2), "{title} position");
        for (k, line) in want.iter().enumerate() {
            let got: Vec<char> = row_chars(&g, row + k as u32)
                .into_iter()
                .skip(col as usize)
                .take(line.len())
                .collect();
            assert_eq!(&got, line, "row {k} of {title}");
        }
    }
}

#[test]
fn reference_positions_at_the_minimum_grid() {
    let mut s = shell();
    open(&mut s, 0);
    let g = s.compose().unwrap();
    assert_eq!(find(&g, "╭─Workbench"), Some((9, 31)));
    let mut s = shell();
    open(&mut s, 1);
    let g = s.compose().unwrap();
    assert_eq!(find(&g, "╭─Datum"), Some((12, 31)));
}

#[test]
fn popups_clamp_on_a_grid_smaller_than_the_popup() {
    let mut s = shell();
    open(&mut s, 0);
    assert!(s.resize(12, 6));
    let _ = s.compose();
    assert!(s.resize(0, 0));
    let _ = s.compose();
    s.key(ShellKey::Escape);
}

/// Every reference mnemonic (`[S]plit [F]ace` -> `SF`) activates its command from the open
/// popup and closes it; the token is scoped to the popup.
#[test]
fn every_reference_mnemonic_dispatches() {
    for (i, title) in TITLES.iter().enumerate() {
        for line in &reference_box(title)[1..] {
            let label: String = line.iter().collect();
            let mut token = String::new();
            let mut rest = label.as_str();
            while let Some(start) = rest.find('[') {
                let end = rest.find(']').unwrap();
                token.push_str(&rest[start + 1..end]);
                rest = &rest[end + 1..];
            }
            if token.is_empty() {
                continue;
            }
            let mut s = shell();
            open(&mut s, i);
            assert!(s.compose().is_ok());
            for c in token.chars() {
                s.key(ShellKey::Char(c));
            }
            let g = s.compose().unwrap();
            let top = format!("╭─{title}─");
            assert!(find(&g, &top).is_none(), "{title}: {label:?} ({token})");
        }
    }
}

#[test]
fn pointer_opens_switches_activates_and_dismisses() {
    let mut s = shell();
    assert!(click(&mut s, "Create"));
    assert!(find(&s.compose().unwrap(), "╭─Create").is_some());
    // Another title switches menu.
    assert!(click(&mut s, "Modify"));
    let g = s.compose().unwrap();
    assert!(find(&g, "╭─Create").is_none() && find(&g, "╭─Modify").is_some());
    // The same title again closes it.
    assert!(click(&mut s, "Modify"));
    assert!(find(&s.compose().unwrap(), "╭─Modify").is_none());
    // An item activates and closes; Workbench rows set the StatusBar's workbench.
    click(&mut s, "Workbench");
    assert!(click(&mut s, "Mesh"));
    assert_eq!(s.workbench(), 2);
    assert!(find(&s.compose().unwrap(), "╭─Workbench").is_none());
    // A press elsewhere dismisses and is consumed.
    click(&mut s, "Create");
    assert!(click(&mut s, "3D Viewport"));
    assert!(find(&s.compose().unwrap(), "╭─Create").is_none());
}

#[test]
fn mock_commands_leave_the_workbench_untouched() {
    let mut s = shell();
    open(&mut s, 3);
    s.key(ShellKey::Char('t'));
    assert_eq!(s.workbench(), 0);
}

#[test]
fn menu_bar_still_opens_and_replaces_the_command_popup() {
    let mut s = shell();
    open(&mut s, 2);
    s.key(ShellKey::Alt('f'));
    let g = s.compose().unwrap();
    assert!(find(&g, "╭─Create").is_none());
    assert!(find(&g, "╭─File").is_some());
}

#[test]
fn command_bar_matches_the_corrected_reference_header() {
    let reference = REFERENCE
        .lines()
        .find(|line| line.trim_start().starts_with("Workbench │"))
        .unwrap();
    let g = shell().compose().unwrap();
    let row: String = row_chars(&g, 3).into_iter().collect();
    assert_eq!(row.trim_matches('║').trim(), reference.trim());
}

#[test]
fn hidden_command_bar_has_no_pointer_targets_and_view_menu_stays_available() {
    let mut s = shell();
    let shown = s.compose().unwrap();
    let (row, col) = find(&shown, "Workbench").unwrap();
    let old_hit = hit_test(
        UiPoint {
            x: col * 6 + 3,
            y: row * 10 + 5,
        },
        GridMetrics::new(6, 10).unwrap(),
        &shown,
    )
    .unwrap();
    assert!(old_hit.widget_id.is_some());

    s.key(ShellKey::Alt('v'));
    assert!(click(&mut s, "Task bar"));
    let hidden = s.compose().unwrap();
    assert!(find(&hidden, "Workbench").is_none());
    let hidden_hit = hit_test(
        UiPoint {
            x: col * 6 + 3,
            y: row * 10 + 5,
        },
        GridMetrics::new(6, 10).unwrap(),
        &hidden,
    )
    .unwrap();
    assert_ne!(hidden_hit.widget_id, old_hit.widget_id);
    s.click(Some(hidden_hit));
    assert!(find(&s.compose().unwrap(), "╭─Create").is_none());

    s.key(ShellKey::Escape);
    s.key(ShellKey::Alt('f'));
    assert!(find(&s.compose().unwrap(), "╭─File").is_some());
}
