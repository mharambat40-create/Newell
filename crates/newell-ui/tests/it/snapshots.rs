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

use newell_ui::{
    CellContent, CellGrid, GridMetrics, HitTestResult, ShellKey, ShellState, UiPoint, WidgetId,
    hit_test,
};

/// The minimum window grid (`ARCHITECTURE.md` §3.1.1): 82 x 30. The reference of
/// `Interface_Exemple.txt` is 82 x 28; the two extra body rows are blank (`README_UI.md` §25),
/// so the 16-row Action Box is shown whole.
const COLS: u32 = 82;
const ROWS: u32 = 30;

fn shell() -> ShellState {
    let mut shell = ShellState::new(COLS, ROWS);
    shell.set_status(98.0, 65);
    shell
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
    let mut s = s.clone();
    s.set_status(98.0, 65);
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
    // The first Select field is Profile; target this ordinary entity field explicitly.
    let grid = s.compose().unwrap();
    let (col, row) = (0..grid.rows())
        .find_map(|row| {
            let chars = row_text(&grid, row);
            let line: String = chars.iter().collect();
            line.contains("Targets").then(|| {
                let col = chars.iter().position(|&c| c == '[').unwrap();
                (u32::try_from(col).unwrap(), row)
            })
        })
        .expect("Targets field must be visible");
    let hit = hit_test(
        UiPoint {
            x: col * 6 + 3,
            y: row * 10 + 5,
        },
        GridMetrics::new(6, 10).unwrap(),
        &grid,
    );
    assert!(s.click(hit));
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
    click(&mut s, "[-]Features");
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
    click(&mut s, "[-]Features");
    assert_ne!(glyphs(&s), base);
    click(&mut s, "[+]Features");
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

/// M2.20: the Rename modal over the shell, started like the Controller does after a
/// double-click (the recognition itself is tested in `newell-app`).
#[test]
fn rename_modal_open() {
    let mut s = ShellState::new(COLS, 28);
    let grid = s.compose().unwrap();
    let id = hit_on(&grid, "Hub_Body").and_then(|h| h.widget_id).unwrap();
    assert!(s.begin_rename(id));
    insta::assert_snapshot!(snapshot(&s));
}

/// M2.20: the playhead dragged onto `Hub_Features`: only the separator line moves.
#[test]
fn playhead_after_hub_features() {
    use newell_ui::Separator;
    let mut s = ShellState::new(COLS, 28);
    let grid = s.compose().unwrap();
    let row = hit_on(&grid, "Hub_Features").unwrap().row;
    assert!(s.drag_separator(Separator::Playhead, 10, row));
    insta::assert_snapshot!(snapshot(&s));
}

/// M2.20: the Features / Elements divider dragged up to row 12.
#[test]
fn divider_dragged_up() {
    use newell_ui::Separator;
    let mut s = ShellState::new(COLS, 28);
    assert!(s.drag_separator(Separator::Split, 10, 12));
    insta::assert_snapshot!(snapshot(&s));
}

/// Opens the `menu`-th CommandBar popup and activates an item by its mnemonic.
fn command(s: &mut ShellState, menu: usize, mnemonic: char) {
    s.key(ShellKey::CommandBar);
    for _ in 0..menu {
        s.key(ShellKey::Right);
    }
    s.key(ShellKey::Down);
    s.key(ShellKey::Char(mnemonic));
}

#[test]
fn revolve_partial_up_to() {
    let mut s = shell();
    command(&mut s, 2, 'R');
    click(&mut s, "Full Revolve");
    click(&mut s, "▶");
    insta::assert_snapshot!(snapshot(&s));
}

#[test]
fn loft_start_type_match() {
    let mut s = shell();
    command(&mut s, 2, 'L');
    click(&mut s, "□ Start type");
    click(&mut s, "◀");
    insta::assert_snapshot!(snapshot(&s));
}

#[test]
fn sweep_keep_orientation_twist() {
    let mut s = shell();
    command(&mut s, 2, 'S');
    click(&mut s, "□ Keep orientation");
    click(&mut s, "□ Twist");
    insta::assert_snapshot!(snapshot(&s));
}

#[test]
fn boolean_cut_offset_faces() {
    let mut s = shell();
    command(&mut s, 4, 'B');
    click(&mut s, " Cut ");
    click(&mut s, "□ Offset");
    click(&mut s, "■ Offset all");
    insta::assert_snapshot!(snapshot(&s));
}

/// M2.25: the widget under the first `cell_needle` of the row that contains `row_needle`.
fn id_in_row(grid: &CellGrid, row_needle: &str, cell_needle: &str) -> WidgetId {
    let metrics = GridMetrics::new(6, 10).unwrap();
    (0..grid.rows())
        .find_map(|row| {
            let text: String = row_text(grid, row).into_iter().collect();
            if !text.contains(row_needle) {
                return None;
            }
            let col = text[..text.find(cell_needle)?].chars().count();
            let col = u32::try_from(col).unwrap();
            hit_test(
                UiPoint {
                    x: col * 6 + 3,
                    y: row * 10 + 5,
                },
                metrics,
                grid,
            )?
            .widget_id
        })
        .unwrap()
}

/// M2.25: the shell with the hover target `id` highlighted (`README_UI.md` §5.12).
fn hovered(s: &ShellState, id: WidgetId) -> String {
    let (grid, lit) = s.compose_hovered(Some(id)).unwrap();
    assert!(lit, "{id:?} must be highlighted");
    render(&grid)
}

#[test]
fn hover_menu_row() {
    let mut s = shell();
    assert!(s.key(ShellKey::Alt('f')));
    let id = id_in_row(&s.compose().unwrap(), "New       ", "New");
    insta::assert_snapshot!(hovered(&s, id));
}

#[test]
fn hover_parameter_row() {
    let s = shell();
    let id = id_in_row(&s.compose().unwrap(), "Along     [", "[Select");
    insta::assert_snapshot!(hovered(&s, id));
}

/// Row 19 of `Interface_Exemple.txt`: the Starting Offset checkbox under the pointer.
#[test]
fn hover_checkbox_row_nineteen() {
    let s = shell();
    let id = id_in_row(&s.compose().unwrap(), "Starting Offset", "Starting");
    insta::assert_snapshot!(hovered(&s, id));
}

#[test]
fn hover_button_ok() {
    let s = shell();
    let id = id_in_row(&s.compose().unwrap(), "Cancel", "OK");
    insta::assert_snapshot!(hovered(&s, id));
}

#[test]
fn hover_button_cancel() {
    let s = shell();
    let id = id_in_row(&s.compose().unwrap(), "OK", "Cancel");
    insta::assert_snapshot!(hovered(&s, id));
}

#[test]
fn view_menu_color_unchecked() {
    let mut s = shell();
    s.key(ShellKey::Alt('v'));
    insta::assert_snapshot!(snapshot(&s));
}

#[test]
fn view_menu_color_checked_dark() {
    let mut s = shell();
    s.key(ShellKey::Alt('v'));
    for _ in 0..4 {
        s.key(ShellKey::Down);
    }
    s.key(ShellKey::Enter);
    insta::assert_snapshot!(snapshot(&s));
}

#[test]
fn view_menu_color_checked_light() {
    let mut s = shell();
    s.key(ShellKey::Alt('v'));
    for _ in 0..4 {
        s.key(ShellKey::Down);
    }
    s.key(ShellKey::Enter);
    s.key(ShellKey::Up);
    s.key(ShellKey::Enter);
    insta::assert_snapshot!(snapshot(&s));
}
