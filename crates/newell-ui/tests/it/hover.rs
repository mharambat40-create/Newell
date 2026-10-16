// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M2.25: the ASCII hover highlight of the composed shell (`docs/ui_design/README_UI.md` §5.12
//! and §25). A hovered target is composed through `ShellState::compose_hovered` with the id a
//! pointer on that cell resolves to, then compared with the normal composition.
#![allow(clippy::unwrap_used)]

use newell_ui::{
    CellContent, CellGrid, GridMetrics, ShellKey, ShellState, UiPoint, WidgetId, hit_test,
};

const COLS: u32 = 82;
const ROWS: u32 = 30;
/// First column of the Action Box on an 82-column grid: it ends at column 80, the viewport
/// border is at 81 (`Interface_Exemple.txt`).
const BOX_LEFT: usize = 50;
const BOX_WIDTH: usize = 31;
const REFERENCE: &str = include_str!("../../../../docs/ui_design/Interface_Exemple.txt");

fn shell() -> ShellState {
    ShellState::new(COLS, ROWS)
}

fn chars(g: &CellGrid, row: u32) -> String {
    (0..g.cols())
        .map(|col| match g.get(col, row).map(|c| c.content) {
            Some(CellContent::Glyph(c)) => c,
            _ => ' ',
        })
        .collect()
}

/// The cell of `cell_needle` in the first row that contains `row_needle`.
fn locate(g: &CellGrid, row_needle: &str, cell_needle: &str) -> (u32, u32) {
    (0..g.rows())
        .find_map(|row| {
            let text = chars(g, row);
            if !text.contains(row_needle) {
                return None;
            }
            let byte = text.find(cell_needle)?;
            let col = text[..byte].chars().count();
            Some((u32::try_from(col).unwrap(), row))
        })
        .unwrap()
}

/// The widget under the first `cell_needle` of the row containing `row_needle`.
fn id_in(g: &CellGrid, row_needle: &str, cell_needle: &str) -> WidgetId {
    let (col, row) = locate(g, row_needle, cell_needle);
    hit_test(point(col, row), metrics(), g)
        .and_then(|h| h.widget_id)
        .unwrap()
}

fn metrics() -> GridMetrics {
    GridMetrics::new(8, 18).unwrap()
}

/// Centre of cell `(col, row)` in pixels.
fn point(col: u32, row: u32) -> UiPoint {
    UiPoint {
        x: col * 8 + 3,
        y: row * 18 + 9,
    }
}

/// The 31 Action Box cells of the row containing `needle`.
fn box_row(g: &CellGrid, needle: &str) -> String {
    let (_, row) = locate(g, needle, needle);
    chars(g, row)
        .chars()
        .skip(BOX_LEFT)
        .take(BOX_WIDTH)
        .collect()
}

/// The 31 Action Box cells of row `number` (1-based) of `Interface_Exemple.txt`.
fn reference_box(number: usize) -> String {
    REFERENCE
        .lines()
        .nth(number - 1)
        .unwrap()
        .chars()
        .skip(BOX_LEFT)
        .take(BOX_WIDTH)
        .collect()
}

/// The hovered composition of `s` with `id`, which must be eligible.
fn hovered(s: &ShellState, id: WidgetId) -> CellGrid {
    let (grid, lit) = s.compose_hovered(Some(id)).unwrap();
    assert!(lit, "{id:?} must be highlighted");
    grid
}

/// Cell by cell, `hovered` differs from `normal` only in glyphs, in at most `slots` cells, with
/// the same size, identities and hit-test results everywhere.
fn assert_only_slots_differ(normal: &CellGrid, hovered: &CellGrid, slots: usize) {
    assert_eq!(
        (normal.cols(), normal.rows()),
        (hovered.cols(), hovered.rows())
    );
    let mut differ = 0;
    for row in 0..normal.rows() {
        for col in 0..normal.cols() {
            let (a, b) = (
                normal.get(col, row).unwrap(),
                hovered.get(col, row).unwrap(),
            );
            assert_eq!(
                (a.widget_id, a.style, a.shared_border),
                (b.widget_id, b.style, b.shared_border),
                "identity of cell {col},{row}"
            );
            if a.content != b.content {
                differ += 1;
            }
            assert_eq!(
                hit_test(point(col, row), metrics(), normal),
                hit_test(point(col, row), metrics(), hovered),
                "hit-test of cell {col},{row}"
            );
        }
    }
    assert!((1..=slots).contains(&differ), "{differ} glyphs differ");
}

#[test]
fn checkbox_row_is_reference_row_nineteen_when_hovered() {
    let s = shell();
    let normal = s.compose().unwrap();
    assert_eq!(
        box_row(&normal, "Starting Offset"),
        "│ │ □ Starting Offset         │"
    );
    let id = id_in(&normal, "Starting Offset", "Starting");
    let hover = hovered(&s, id);
    assert_eq!(box_row(&hover, "Starting Offset"), reference_box(19));
    assert_eq!(reference_box(19), "│ ╞ □ Starting Offset         ╡");
    assert_only_slots_differ(&normal, &hover, 3);
}

#[test]
fn connector_checkbox_row_turns_its_pair_into_the_corner() {
    let s = shell();
    let normal = s.compose().unwrap();
    assert_eq!(
        box_row(&normal, "End Offset"),
        "│ ╰╴□ End Offset              │"
    );
    let hover = hovered(&s, id_in(&normal, "End Offset", "End"));
    assert_eq!(
        box_row(&hover, "End Offset"),
        "│ ╘ □ End Offset              ╡"
    );
    assert_only_slots_differ(&normal, &hover, 3);
}

#[test]
fn parameter_rows_light_their_branch_and_right_border() {
    let s = shell();
    let normal = s.compose().unwrap();
    assert_eq!(box_row(&normal, "Along"), "│ │ Along     [Select…       ]│");
    let along = hovered(&s, id_in(&normal, "Along", "[Select"));
    assert_eq!(box_row(&along, "Along"), "│ ╞ Along     [Select…       ]╡");
    assert_only_slots_differ(&normal, &along, 2);

    // The whole row is the target: the numeric field and its `☰` light the same slots.
    let field = hovered(&s, id_in(&normal, "Depth", "+|0.0"));
    let formula = hovered(&s, id_in(&normal, "Depth", "☰"));
    for grid in [&field, &formula] {
        assert_eq!(box_row(grid, "Depth"), "│ ╞ Depth   [+|0.0 mm       ☰]╡");
        assert_only_slots_differ(&normal, grid, 2);
    }

    // A row without a branch takes the frame on its left.
    let profile = hovered(&s, id_in(&normal, "Profile     [", "[Select"));
    assert_eq!(
        box_row(&profile, "Profile     ["),
        "╞ Profile     [Select…       ]╡"
    );
}

#[test]
fn buttons_light_only_their_own_slots() {
    let s = shell();
    let normal = s.compose().unwrap();
    assert_eq!(
        box_row(&normal, "Cancel"),
        "│      OK      │    Cancel    │"
    );

    let ok = hovered(&s, id_in(&normal, "Cancel", "OK"));
    assert_eq!(box_row(&ok, "Cancel"), "╞      OK      ╡    Cancel    │");
    assert_only_slots_differ(&normal, &ok, 2);

    let cancel = hovered(&s, id_in(&normal, "OK", "Cancel"));
    assert_eq!(box_row(&cancel, "OK"), "│      OK      ╞    Cancel    ╡");
    assert_only_slots_differ(&normal, &cancel, 2);
}

#[test]
fn menu_row_lights_its_frame_and_nothing_else() {
    let mut s = shell();
    assert!(s.key(ShellKey::Alt('f')));
    let normal = s.compose().unwrap();
    let (_, row) = locate(&normal, "New       ", "New");
    assert!(chars(&normal, row).contains("│ New       │"));
    let hover = hovered(&s, id_in(&normal, "New       ", "New"));
    assert!(chars(&hover, row).contains("╞ New       ╡"));
    assert_only_slots_differ(&normal, &hover, 2);
}

#[test]
fn cyclic_menu_row_is_a_menu_target_on_its_arrows() {
    let mut s = shell();
    assert!(s.key(ShellKey::Alt('v')));
    let normal = s.compose().unwrap();
    let (_, row) = locate(&normal, "UI Scale", "◀");
    let hover = hovered(&s, id_in(&normal, "UI Scale", "◀"));
    let line = chars(&hover, row);
    assert!(line.contains('╞') && line.contains('╡'), "{line:?}");
    assert_only_slots_differ(&normal, &hover, 2);
}

#[test]
fn an_open_menu_covers_the_action_box_for_hover() {
    let mut s = shell();
    let normal = s.compose().unwrap();
    let along = id_in(&normal, "Along", "[Select");
    assert!(s.key(ShellKey::Alt('f')));
    let covered = s.compose_hovered(Some(along)).unwrap();
    assert!(!covered.1, "a covered control never hovers");
    assert_eq!(covered.0, s.compose().unwrap());
}

#[test]
fn formula_modal_ok_hovers_and_the_box_beneath_does_not() {
    let mut s = shell();
    let normal = s.compose().unwrap();
    let field_formula = id_in(&normal, "Depth", "☰");
    let (col, row) = locate(&normal, "Depth", "☰");
    assert!(s.click(hit_test(point(col, row), metrics(), &normal)));
    assert!(s.formula().is_open());
    let grid = s.compose().unwrap();
    let ok = id_in(&grid, "       OK       ", "OK");
    assert!(!s.compose_hovered(Some(field_formula)).unwrap().1);
    let hover = hovered(&s, ok);
    let (_, modal_row) = locate(&grid, "       OK       ", "OK");
    assert!(chars(&hover, modal_row).contains("╞       OK       ╡"));
    assert_only_slots_differ(&grid, &hover, 2);
}

#[test]
fn rename_modal_ok_hovers_as_its_own_row() {
    let mut s = shell();
    let normal = s.compose().unwrap();
    let name = id_in(&normal, "Hub_Features", "Hub");
    assert!(s.begin_rename(name));
    let grid = s.compose().unwrap();
    let ok = id_in(&grid, "     OK     ", "OK");
    let hover = hovered(&s, ok);
    let (_, modal_row) = locate(&grid, "     OK     ", "OK");
    assert!(chars(&hover, modal_row).contains("╞     OK     ╡   Cancel   │"));
    assert_only_slots_differ(&grid, &hover, 2);
}

#[test]
fn non_targets_never_light() {
    let s = shell();
    let normal = s.compose().unwrap();
    // Tree rows, the MenuBar title and the window controls.
    for (row_needle, cell_needle) in [("Hub_Features", "Hub"), ("File", "File"), ("[X]", "[X]")] {
        let id = id_in(&normal, row_needle, cell_needle);
        let (grid, lit) = s.compose_hovered(Some(id)).unwrap();
        assert!(!lit, "{cell_needle:?} must not hover");
        assert_eq!(grid, normal, "{cell_needle:?}");
    }
}

#[test]
fn only_action_box_and_popup_ids_may_hover() {
    let s = shell();
    let normal = s.compose().unwrap();
    for (row_needle, cell_needle) in [
        ("Hub_Features", "Hub"),
        ("File", "File"),
        ("Workbench", "Workbench"),
        ("[X]", "[X]"),
    ] {
        let id = id_in(&normal, row_needle, cell_needle);
        assert!(!ShellState::may_hover(id), "{cell_needle:?} never hovers");
    }
    for (row_needle, cell_needle) in [
        ("Along     [", "[Select"),
        ("Cancel", "OK"),
        ("Cancel", "Cancel"),
    ] {
        let id = id_in(&normal, row_needle, cell_needle);
        assert!(ShellState::may_hover(id), "{cell_needle:?} may hover");
    }
}

fn open_operation(menu: usize, mnemonic: char) -> ShellState {
    let mut s = shell();
    s.key(ShellKey::CommandBar);
    for _ in 0..menu {
        s.key(ShellKey::Right);
    }
    s.key(ShellKey::Down);
    s.key(ShellKey::Char(mnemonic));
    s
}

#[test]
fn all_action_box_headers_highlight_only_the_hovered_segment() {
    for (menu, mnemonic) in [(2, 'E'), (2, 'R'), (2, 'L'), (2, 'S'), (4, 'B')] {
        let s = open_operation(menu, mnemonic);
        let normal = s.compose().unwrap();
        let segments: &[(&str, &str, usize, usize)] =
            if s.operation() == newell_ui::ActionOperation::Boolean {
                &[
                    ("  Cut ", "Add", 0, 10),
                    ("  Cut ", "Cut  ", 10, 18),
                    ("  Cut ", "Inter", 18, 30),
                ]
            } else {
                &[
                    ("Solid", "Solid", 0, 14),
                    ("Solid", "Surface", 14, 30),
                    ("New", "New", 0, 8),
                    ("New", "Add", 8, 18),
                    ("New", "Remove", 18, 30),
                ]
            };
        for &(row, label, left, right) in segments {
            let h = hovered(&s, id_in(&normal, row, label));
            let mut expected: Vec<char> = box_row(&normal, row).chars().collect();
            expected[left] = '╞';
            expected[right] = '╡';
            assert_eq!(
                box_row(&h, row),
                expected.into_iter().collect::<String>(),
                "{mnemonic} {label}"
            );
            assert_only_slots_differ(&normal, &h, 2);
        }
        // All button ids of every header (including Boolean) must be eligible.
        let (_, footer) = locate(&normal, "Cancel", "Cancel");
        for row in 0..footer {
            if !chars(&normal, row).contains("New")
                && !chars(&normal, row).contains("Solid")
                && !chars(&normal, row).contains("Cut")
            {
                continue;
            }
            for col in BOX_LEFT as u32 + 1..BOX_LEFT as u32 + 30 {
                if let Some(id) = normal.get(col, row).unwrap().widget_id {
                    let h = hovered(&s, id);
                    assert_only_slots_differ(&normal, &h, 2);
                }
            }
        }
    }
}

#[test]
fn loft_profile_start_type_and_magnitude_labels_hover_without_changing_clicks() {
    let mut s = open_operation(2, 'L');
    let g = s.compose().unwrap();
    let id = id_in(&g, "Start type", "Start");
    let (col, row) = locate(&g, "Start type", "Start");
    assert!(s.click(hit_test(point(col, row), metrics(), &g)));
    let normal = s.compose().unwrap();
    for (row_text, label) in [
        ("Profile     [", "Profile"),
        ("Start type", "Normal"),
        ("Magnitude", "Magnitude"),
    ] {
        let (col, row) = locate(&normal, row_text, label);
        let hit = hit_test(point(col, row), metrics(), &normal).unwrap();
        assert!(
            hit.widget_id.is_none(),
            "label must retain its inert click routing"
        );
        let target = s.hover_target(hit, &normal).unwrap();
        let h = hovered(&s, target);
        assert!(box_row(&h, row_text).ends_with('╡'));
        assert_only_slots_differ(&normal, &h, 3);
        assert_eq!(
            s.hover_target(hit_test(point(col, row), metrics(), &h).unwrap(), &h),
            Some(target)
        );
    }
    for arrow in ["◀", "▶"] {
        let h = hovered(&s, id_in(&normal, "Start type", arrow));
        assert_eq!(
            box_row(&h, "Start type"),
            box_row(&hovered(&s, id), "Start type")
        );
    }
    let (col, row) = locate(&normal, "Profile     [", "Profile");
    let label_hit = hit_test(point(col, row), metrics(), &normal).unwrap();
    assert!(s.key(ShellKey::Alt('f')));
    assert_eq!(s.hover_target(label_hit, &s.compose().unwrap()), None);
}
