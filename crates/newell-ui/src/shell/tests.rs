// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
use super::data::{self, progress_bar, visible_rows};
use super::*;
use crate::{CellContent, CellGrid, GridMetrics, UiPoint, hit_test};

const COLS: u32 = 82;
const ROWS: u32 = 25;

fn metrics() -> GridMetrics {
    GridMetrics::new(6, 10).unwrap()
}

fn shell() -> ShellState {
    ShellState::new(COLS, ROWS)
}

fn text(g: &CellGrid, row: u32) -> String {
    (0..g.cols())
        .map(|col| match g.get(col, row).unwrap().content {
            CellContent::Glyph(ch) => ch,
            CellContent::Empty => ' ',
        })
        .collect()
}

/// `(col, row)` of the first occurrence of `needle` in reading order.
fn find(g: &CellGrid, needle: &str) -> Option<(u32, u32)> {
    (0..g.rows()).find_map(|row| {
        let line: Vec<char> = text(g, row).chars().collect();
        let pat: Vec<char> = needle.chars().collect();
        line.windows(pat.len())
            .position(|w| w == pat.as_slice())
            .map(|col| (u32::try_from(col).unwrap(), row))
    })
}

fn has(g: &CellGrid, needle: &str) -> bool {
    find(g, needle).is_some()
}

fn hit_at(g: &CellGrid, col: u32, row: u32) -> Option<HitTestResult> {
    hit_test(
        UiPoint {
            x: col * 6 + 3,
            y: row * 10 + 5,
        },
        metrics(),
        g,
    )
}

fn click_at(s: &mut ShellState, col: u32, row: u32) -> bool {
    let g = s.compose().unwrap();
    s.click(hit_at(&g, col, row))
}

fn click_text(s: &mut ShellState, needle: &str) -> bool {
    let g = s.compose().unwrap();
    let (col, row) = find(&g, needle).expect("text must be on screen");
    s.click(hit_at(&g, col, row))
}

fn id_of(s: &ShellState, needle: &str) -> WidgetId {
    let g = s.compose().unwrap();
    let (col, row) = find(&g, needle).unwrap();
    hit_at(&g, col, row).and_then(|h| h.widget_id).unwrap()
}

fn cell_of(g: &CellGrid, id: WidgetId) -> (u32, u32) {
    (0..g.rows())
        .find_map(|row| {
            (0..g.cols())
                .find(|&col| g.get(col, row).unwrap().widget_id == Some(id))
                .map(|col| (col, row))
        })
        .unwrap()
}

fn glyph(g: &CellGrid, col: u32, row: u32) -> char {
    match g.get(col, row).unwrap().content {
        CellContent::Glyph(ch) => ch,
        CellContent::Empty => ' ',
    }
}

fn keys(s: &mut ShellState, keys: &[ShellKey]) {
    for &k in keys {
        s.key(k);
    }
}

fn type_str(s: &mut ShellState, t: &str) {
    for c in t.chars() {
        s.key(ShellKey::Char(c));
    }
}

// ---- base composition ----

#[test]
fn base_shell_has_every_region() {
    let s = shell();
    let g = s.compose().unwrap();
    assert_eq!((g.cols(), g.rows()), (COLS, ROWS));
    assert!(!s.modal_open() && s.overlays.is_empty());

    // Frames and merged junctions.
    let corners = [
        (0, 0, '╔'),
        (81, 0, '╗'),
        (0, 2, '╠'),
        (81, 2, '╣'),
        (0, 4, '╠'),
        (24, 4, '╦'),
        (0, 14, '╠'),
        (24, 14, '╣'),
        (0, 22, '╠'),
        (24, 22, '╩'),
        (81, 22, '╣'),
        (0, 24, '╚'),
        (81, 24, '╝'),
    ];
    for (col, row, ch) in corners {
        assert_eq!(glyph(&g, col, row), ch, "({col}, {row})");
    }

    let menu_bar = text(&g, 1);
    for title in ["File", "Edit", "View", "Help", "[-] [□] [X]"] {
        assert!(menu_bar.contains(title), "{title}");
    }
    let command_bar = text(&g, 3);
    for title in [
        "Workbench",
        "Sketch",
        "Create",
        "Operate",
        "Pattern",
        "Refine",
        "Construct",
        "Inspect",
    ] {
        assert!(command_bar.contains(title), "{title}");
    }
    assert!(text(&g, 5).contains("[-] Features"));
    assert!(text(&g, 15).contains("[-] Elements"));
    assert!(text(&g, 5).contains("3D Viewport"));
    assert!(
        text(&g, 23)
            .contains("Roue V1* │ Part design │ Fully Constrained │ 98fps │ Progress [███░ ] 65%")
    );
    // The Extrude panel is open at the reference position, under the NavCube.
    assert!(has(&g, "Solid") && has(&g, "Depth") && has(&g, "Invert"));
    assert!(
        has(&g, "OK") && has(&g, "Cancel"),
        "the panel is not clipped at 82x25"
    );
    assert!(!has(&g, "Merge with all"));
}

#[test]
fn viewport_cells_stay_empty_for_the_clear_colour() {
    let g = shell().compose().unwrap();
    // A viewport cell away from the label, the NavCube and the panel.
    assert_eq!(g.get(30, 12).unwrap().content, CellContent::Empty);
    assert_eq!(g.get(30, 12).unwrap().widget_id, None);
}

#[test]
fn nav_cube_is_a_static_unowned_frame() {
    let mut s = shell();
    // At 82x25 the open panel moves up over the cube's last row; close it first.
    click_text(&mut s, "Cancel");
    let g = s.compose().unwrap();
    assert_eq!(find(&g, "NavCube"), Some((73, 5)));
    assert_eq!(glyph(&g, 72, 5), '┌');
    assert_eq!(glyph(&g, 80, 5), '┐');
    assert_eq!(glyph(&g, 72, 8), '└');
    assert_eq!(glyph(&g, 80, 8), '┘');
    for row in 5..9 {
        for col in 72..81 {
            assert_eq!(g.get(col, row).unwrap().widget_id, None, "({col}, {row})");
        }
    }
    let before = s.clone();
    assert!(!click_at(&mut s, 76, 6));
    assert_eq!(s, before);
}

// ---- menus ----

#[test]
fn file_menu_opens_lists_entries_and_closes_on_activation() {
    let mut s = shell();
    assert!(click_text(&mut s, "File"));
    let g = s.compose().unwrap();
    assert_eq!(glyph(&g, 1, 2), '┌');
    for (row, label) in [
        "New", "Open", "Recent", "Save", "Save As", "Import", "Export", "Option",
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            text(&g, 3 + u32::try_from(row).unwrap()).contains(label),
            "{label}"
        );
    }
    // A popup does not block the rest of the grid.
    let (col, row) = find(&g, "Invert").unwrap();
    assert!(hit_at(&g, col, row).unwrap().widget_id.is_some());

    assert!(click_text(&mut s, "Save As"));
    assert!(!has(&s.compose().unwrap(), "Option"));
    assert!(!s.menu_bar.is_active());
}

#[test]
fn create_menu_opens_and_workbench_updates_the_status_bar() {
    let mut s = shell();
    assert!(click_text(&mut s, "Create"));
    let g = s.compose().unwrap();
    for label in ["[E]xtrude", "[R]evolve", "[L]oft", "[S]weep"] {
        assert!(has(&g, label), "{label}");
    }
    assert!(s.command_bar.open_menu() == Some(2));
    s.key(ShellKey::Escape);
    assert!(!has(&s.compose().unwrap(), "[R]evolve"));

    assert!(click_text(&mut s, "Workbench"));
    assert!(click_text(&mut s, "Assemblies"));
    assert_eq!(s.workbench(), 1);
    assert!(has(&s.compose().unwrap(), "│ Assemblies │"));
}

#[test]
fn only_one_menu_is_open_and_outside_clicks_close_it() {
    let mut s = shell();
    click_text(&mut s, "File");
    click_text(&mut s, "Create");
    assert!(!s.menu_bar.is_active() && s.command_bar.open_menu() == Some(2));
    assert_eq!(s.overlays.len(), 1);
    // A click on the viewport closes the menu and is consumed.
    assert!(click_at(&mut s, 30, 12));
    assert!(!s.command_bar.is_active() && s.overlays.is_empty());
    // A click on a control while a menu is open only closes the menu.
    click_text(&mut s, "File");
    let before = s.extrude;
    click_text(&mut s, "Invert");
    assert_eq!(s.extrude, before);
    assert!(!s.menu_bar.is_active());
}

#[test]
fn view_menu_rows_cycle_and_toggle_without_closing() {
    let mut s = shell();
    click_text(&mut s, "View");
    let g = s.compose().unwrap();
    assert!(has(&g, "Perspective") && has(&g, "Shaded") && has(&g, "■ Tree panel"));
    assert!(has(&g, "□ Grid Overlay"));

    click_text(&mut s, "Grid Overlay");
    let g = s.compose().unwrap();
    assert!(has(&g, "■ Grid Overlay"));
    assert!(s.menu_bar.open_menu().is_some());

    let (col, row) = find(&g, "Perspective").unwrap();
    // The `▶` of the row is its last cell before the padding.
    let next = (col..COLS).find(|&c| glyph(&g, c, row) == '▶').unwrap();
    click_at(&mut s, next, row);
    assert!(has(&s.compose().unwrap(), "Orthographic"));
}

#[test]
fn keyboard_menu_navigation_mirrors_pointer_state() {
    let mut s = shell();
    keys(&mut s, &[ShellKey::MenuBar, ShellKey::Down]);
    assert_eq!(s.menu_bar.open_menu(), Some(0));
    assert!(has(&s.compose().unwrap(), "Recent"));
    assert_eq!(
        s.interaction().focused,
        BAR_IDS.item(0, 0),
        "focus follows the highlighted item"
    );
    s.key(ShellKey::Escape);
    assert!(!s.menu_bar.is_active() && s.interaction().focused.is_none());

    // Alt+V opens View with its first row (a cycle) highlighted; Right cycles it.
    s.key(ShellKey::Alt('v'));
    assert_eq!(s.menu_bar.open_menu(), Some(2));
    s.key(ShellKey::Right);
    assert_eq!(s.view.projection, 1);
    keys(&mut s, &[ShellKey::Down, ShellKey::Down, ShellKey::Enter]);
    assert!(!s.view.checks[0]);
    assert!(s.menu_bar.open_menu().is_some());
    s.key(ShellKey::MenuBar);
    assert!(!s.menu_bar.is_active());

    // F6 starts the CommandBar; Right moves, Down opens.
    keys(
        &mut s,
        &[ShellKey::CommandBar, ShellKey::Right, ShellKey::Right],
    );
    assert_eq!(s.command_bar.current_menu(), Some(2));
    s.key(ShellKey::Down);
    assert_eq!(s.command_bar.open_menu(), Some(2));
}

#[test]
fn typed_mnemonics_assemble_multi_letter_tokens() {
    let mut s = shell();
    click_text(&mut s, "Pattern");
    // `M` alone matches nothing (the entry is `Mi`); the token is kept.
    s.key(ShellKey::Char('m'));
    assert!(s.command_bar.open_menu().is_some());
    s.key(ShellKey::Char('i'));
    assert!(!s.command_bar.is_active(), "[Mi]rror activated");

    click_text(&mut s, "Operate");
    type_str(&mut s, "sb");
    assert!(!s.command_bar.is_active(), "[S]plit [B]ody activated");

    // A broken prefix restarts the token.
    click_text(&mut s, "Pattern");
    type_str(&mut s, "xci");
    assert!(!s.command_bar.is_active(), "[Ci]rcular activated after x");
}

#[test]
fn create_extrude_reopens_a_closed_panel_but_keeps_an_open_one() {
    let mut s = shell();
    click_text(&mut s, "Invert");
    assert!(s.extrude().values.invert);
    click_text(&mut s, "Create");
    click_text(&mut s, "[E]xtrude");
    assert!(s.extrude().values.invert, "an open panel keeps its edits");

    click_text(&mut s, "Cancel");
    assert!(!s.extrude().is_open() && !has(&s.compose().unwrap(), "Invert"));
    click_text(&mut s, "Create");
    click_text(&mut s, "[E]xtrude");
    assert!(s.extrude().is_open() && !s.extrude().values.invert);
    assert!(has(&s.compose().unwrap(), "Invert"));
}

// ---- trees ----

#[test]
fn navigation_trees_start_as_in_the_reference() {
    let g = shell().compose().unwrap();
    for label in [
        "Features",
        "Sketch_Profile_Ma",
        "Hub_Features",
        "Extrude_Base",
        "Sketch_Hole",
        "Extrude_Cut_Cent",
        "Fillet_Inner",
        "Chamfer_Edges",
        "Pattern_Circular",
        "Elements",
        "Solid Bodies(2)",
        "Rim_Body",
        "Hub_Body",
        "Surfaces(0)",
        "[+] Construct",
    ] {
        assert!(has(&g, label), "{label}");
    }
    assert!(text(&g, 6).contains("├ □ Sketch_Profile_Ma"));
    assert!(text(&g, 7).contains("├[-] Hub_Features"));
    assert!(text(&g, 8).contains("│ ├ □ Extrude_Base"));
    assert!(text(&g, 9).contains("│ ├ ■ Sketch_Hole"));
    assert!(text(&g, 10).contains("│ └ □ Extrude_Cut_Cent"));
    assert!(text(&g, 13).contains("└ □ Pattern_Circular"));
    assert!(text(&g, 20).contains("└[+] Construct"));
    assert!(!has(&g, "Plane_XY"), "Construct starts collapsed");
}

#[test]
fn tree_expand_and_collapse_keep_ids_and_trees_independent() {
    let mut s = shell();
    let fillet = id_of(&s, "Fillet_Inner");
    let rim = id_of(&s, "Rim_Body");

    assert!(click_text(&mut s, "Hub_Features"));
    let g = s.compose().unwrap();
    assert!(!has(&g, "Extrude_Base") && !has(&g, "Sketch_Hole"));
    assert!(text(&g, 7).contains("├[+] Hub_Features"));
    assert!(has(&g, "Rim_Body"), "the Elements tree is unaffected");
    assert_eq!(id_of(&s, "Fillet_Inner"), fillet, "ids ignore position");
    assert_eq!(id_of(&s, "Rim_Body"), rim);

    click_text(&mut s, "Hub_Features");
    assert!(has(&s.compose().unwrap(), "Extrude_Base"));

    click_text(&mut s, "[+] Construct");
    let g = s.compose().unwrap();
    assert!(has(&g, "Plane_XY") && has(&g, "[-] Construct"));

    click_text(&mut s, "[-] Features");
    let g = s.compose().unwrap();
    assert!(!has(&g, "Hub_Features") && !has(&g, "Pattern_Circular"));
    assert!(has(&g, "Plane_XY"), "Elements still expanded");
}

#[test]
fn tree_leaf_click_focuses_without_changing_the_tree() {
    let mut s = shell();
    let before = (s.features_open, s.elements_open);
    let id = id_of(&s, "Extrude_Base");
    assert!(click_text(&mut s, "Extrude_Base"));
    assert_eq!((s.features_open, s.elements_open), before);
    assert_eq!(s.interaction().focused, Some(id));
}

#[test]
fn visible_rows_compute_guides_from_the_flat_list() {
    let open = initial_open_features();
    let rows = visible_rows(&data::FEATURES, &open);
    let shape: Vec<(usize, bool, Vec<bool>)> = rows
        .iter()
        .map(|r| (r.index, r.last, r.trunk.clone()))
        .collect();
    assert_eq!(
        shape,
        vec![
            (0, true, vec![]),
            (1, false, vec![]),
            (2, false, vec![]),
            (3, false, vec![true]),
            (4, false, vec![true]),
            (5, true, vec![true]),
            (6, false, vec![]),
            (7, false, vec![]),
            (8, true, vec![]),
        ]
    );
    let mut closed = open;
    closed[2] = false;
    let hidden: Vec<usize> = visible_rows(&data::FEATURES, &closed)
        .iter()
        .map(|r| r.index)
        .collect();
    assert_eq!(hidden, vec![0, 1, 2, 6, 7, 8]);
}

fn initial_open_features() -> [bool; data::MAX_NODES] {
    data::initial_open(&data::FEATURES)
}

// ---- Extrude integration ----

#[test]
fn extrude_panel_discloses_rows_through_the_shell() {
    let mut s = shell();
    assert!(click_text(&mut s, "Add"));
    assert!(has(&s.compose().unwrap(), "Merge with all"));
    assert!(click_text(&mut s, "Starting Offset"));
    assert!(has(&s.compose().unwrap(), "Dist"));
    assert!(click_text(&mut s, "Merge with all"));
    assert!(has(&s.compose().unwrap(), "Merge scope"));

    // The first `▶` on screen is the termination cycle.
    let g = s.compose().unwrap();
    let (col, row) = (0..ROWS)
        .find_map(|r| (0..COLS).find(|&c| glyph(&g, c, r) == '▶').map(|c| (c, r)))
        .unwrap();
    click_at(&mut s, col, row);
    let g = s.compose().unwrap();
    assert_eq!(
        s.extrude().values.termination,
        crate::ExtrudeTermination::UpTo
    );
    assert!(!has(&g, "Depth") && has(&g, "Selection"));
}

#[test]
fn extrude_ok_closes_and_cancel_restores_the_opening_values() {
    let mut s = shell();
    click_text(&mut s, "Invert");
    click_text(&mut s, "OK");
    assert!(!s.extrude().is_open() && !has(&s.compose().unwrap(), "Starting Offset"));
    assert_eq!(s.interaction().focused, None);

    let mut s = shell();
    click_text(&mut s, "Invert");
    click_text(&mut s, "Cancel");
    assert_eq!(s.extrude().status, crate::ExtrudeStatus::Cancelled);
    assert!(
        !s.extrude().values.invert,
        "values restored to the opening ones"
    );
}

// ---- SelectionPanel / FormulaEditor ----

fn open_selection(s: &mut ShellState) {
    // `Selection` only exists for `Up to`.
    let g = s.compose().unwrap();
    let (col, row) = (0..ROWS)
        .find_map(|r| (0..COLS).find(|&c| glyph(&g, c, r) == '▶').map(|c| (c, r)))
        .unwrap();
    click_at(s, col, row);
    assert!(click_text(s, "none"));
}

#[test]
fn selection_field_opens_the_selection_panel() {
    let mut s = shell();
    open_selection(&mut s);
    assert!(s.selection().is_open());
    let g = s.compose().unwrap();
    assert!(has(&g, "Flange_Body") && has(&g, "Selection"));
    assert_eq!(s.interaction().focused, s.selection().focus());
    // The Extrude controls are still the real ones underneath, unchanged.
    assert_eq!(s.extrude().values.selection, crate::MockSelection::None);
}

#[test]
fn formula_button_opens_the_formula_editor() {
    let mut s = shell();
    let g = s.compose().unwrap();
    let (col, row) = find(&g, "☰").unwrap();
    assert!(click_at(&mut s, col, row));
    assert!(s.formula().is_open() && !s.selection().is_open());
    let g = s.compose().unwrap();
    assert!(has(&g, "Formula") && has(&g, "Fx = (#EntraxeA"));
    assert_eq!(s.interaction().focused, s.formula().focus());
}

#[test]
fn modal_blocks_pointer_and_keyboard_to_the_base_shell() {
    let mut s = shell();
    let g = s.compose().unwrap();
    let (col, row) = find(&g, "☰").unwrap();
    // Uncovered spots: a tree row, both bars, and the right part of the Invert row (the
    // modal spans columns 21..61).
    let invert_row = find(&g, "Invert").unwrap().1;
    let spots: Vec<(&str, (u32, u32))> = vec![
        ("Hub_Features", find(&g, "Hub_Features").unwrap()),
        ("Workbench", find(&g, "Workbench").unwrap()),
        ("File", find(&g, "File").unwrap()),
        ("Invert row", (70, invert_row)),
    ];
    assert!(g.get(70, invert_row).unwrap().widget_id.is_some());
    click_at(&mut s, col, row);
    let g = s.compose().unwrap();

    // Covered and uncovered controls alike have no owner outside the modal.
    for &(needle, (c, r)) in &spots {
        assert_eq!(hit_at(&g, c, r).unwrap().widget_id, None, "{needle}");
    }
    let base = (s.extrude, s.features_open, s.elements_open);
    for &(_, (c, r)) in &spots {
        assert!(!click_at(&mut s, c, r));
    }
    assert!(s.formula().is_open());
    assert_eq!(base, (s.extrude, s.features_open, s.elements_open));
    assert!(
        !s.pointer_moved(hit_at(&g, 70, invert_row)),
        "hover stays empty"
    );
    assert_eq!(s.interaction().hovered, None);

    // Keys go to the modal: base shortcuts do nothing, text is typed.
    assert!(!s.key(ShellKey::MenuBar) && !s.menu_bar.is_active());
    assert!(!s.key(ShellKey::Alt('f')) && !s.menu_bar.is_active());
    s.key(ShellKey::Char('x'));
    assert!(s.formula().draft().ends_with('x'));
    s.key(ShellKey::Tab);
    assert_eq!(s.interaction().focused, s.formula().focus());
    assert!(s.formula().is_open());
    assert_eq!(base, (s.extrude, s.features_open, s.elements_open));
}

#[test]
fn formula_cancel_and_ok_restore_focus_and_text() {
    let mut s = shell();
    let button = id_of(&s, "☰");
    click_text(&mut s, "☰");
    type_str(&mut s, "ab");
    s.key(ShellKey::Escape);
    assert!(!s.formula().is_open());
    assert_eq!(s.formula().text(), MOCK_FORMULA);
    assert_eq!(
        s.interaction().focused,
        Some(button),
        "prior focus restored"
    );
    assert!(s.overlays.is_empty());

    click_text(&mut s, "☰");
    assert_eq!(
        s.formula().draft(),
        MOCK_FORMULA,
        "draft restarts from applied"
    );
    type_str(&mut s, "ab");
    s.key(ShellKey::Enter);
    assert_eq!(s.formula().text(), format!("{MOCK_FORMULA}ab"));
    assert_eq!(s.interaction().focused, Some(button));
}

#[test]
fn selection_cancel_and_ok_restore_focus_and_state() {
    let mut s = shell();
    open_selection(&mut s);
    let ok = WidgetId::new(SELECTION_BASE + 3);
    let cancel = WidgetId::new(SELECTION_BASE + 4);
    let field = s.selection().focus();
    assert!(field.is_some());

    // Toggle the first item in the draft, then Cancel.
    s.key(ShellKey::Char(' '));
    assert!(!s.selection().applied()[0]);
    let g = s.compose().unwrap();
    let (col, row) = cell_of(&g, cancel);
    click_at(&mut s, col, row);
    assert!(!s.selection().is_open());
    assert!(!s.selection().applied()[0], "cancel discards");
    assert!(s.selection().applied()[1], "Hub_Body stays applied");
    assert_ne!(s.interaction().focused, field);
    assert!(
        s.interaction().focused.is_some(),
        "focus returns to the field"
    );
    let restored = s.interaction().focused;

    // Reopen, toggle, OK applies.
    click_text(&mut s, "none");
    s.key(ShellKey::Char(' '));
    let g = s.compose().unwrap();
    let (col, row) = cell_of(&g, ok);
    click_at(&mut s, col, row);
    assert!(s.selection().applied()[0]);
    assert_eq!(s.interaction().focused, restored);
}

#[test]
fn escape_closes_a_modal_and_modals_are_exclusive_with_menus() {
    let mut s = shell();
    click_text(&mut s, "File");
    assert!(s.menu_bar.open_menu().is_some());
    // Keyboard opens a modal while a menu is active: the menu closes.
    s.selection.open(&mut s.interaction);
    s.sync();
    assert!(!s.menu_bar.is_active() && s.overlays.len() == 1);
    assert!(has(&s.compose().unwrap(), "Flange_Body"));
    s.key(ShellKey::Escape);
    assert!(s.overlays.is_empty() && !s.modal_open());
}

// ---- keyboard focus ring ----

#[test]
fn tab_walks_trees_then_extrude_controls_and_enter_activates() {
    let mut s = shell();
    s.key(ShellKey::Tab);
    assert_eq!(s.interaction().focused, Some(tree_id(Tree::Features, 0)));
    s.key(ShellKey::Enter);
    assert!(!has(&s.compose().unwrap(), "Hub_Features"));
    s.key(ShellKey::BackTab);
    let last = s.interaction().focused;
    assert!(
        last.is_some_and(|id| id.get() >= EXTRUDE_BASE),
        "wrapped to the panel"
    );

    let invert = id_of(&s, "Invert");
    for _ in 0..100 {
        if s.interaction().focused == Some(invert) {
            break;
        }
        s.key(ShellKey::Tab);
    }
    assert_eq!(s.interaction().focused, Some(invert));
    s.key(ShellKey::Char(' '));
    assert!(s.extrude().values.invert);
}

// ---- determinism, ids, tiny grids ----

#[test]
fn composition_is_deterministic() {
    assert_eq!(shell().compose().unwrap(), shell().compose().unwrap());
    let run = || {
        let mut s = shell();
        click_text(&mut s, "Add");
        click_text(&mut s, "Create");
        s.key(ShellKey::Escape);
        s.key(ShellKey::Tab);
        s
    };
    let (a, b) = (run(), run());
    assert_eq!(a, b);
    assert_eq!(a.compose().unwrap(), b.compose().unwrap());
    assert_ne!(a.compose().unwrap(), shell().compose().unwrap());
}

#[test]
fn every_cell_owner_belongs_to_exactly_one_id_range() {
    let mut s = shell();
    click_text(&mut s, "View");
    for state in [&mut s.clone(), &mut shell()] {
        let g = state.compose().unwrap();
        for cell in g.cells() {
            let Some(id) = cell.widget_id else { continue };
            let owners = [
                BAR_IDS.decode(id).is_some(),
                COMMAND_IDS.decode(id).is_some(),
                tree_decode(id).is_some(),
                (EXTRUDE_BASE..SELECTION_BASE).contains(&id.get()),
                (SELECTION_BASE..FORMULA_BASE).contains(&id.get()),
                (FORMULA_BASE..FORMULA_BASE + 4).contains(&id.get()),
            ];
            assert_eq!(owners.iter().filter(|&&o| o).count(), 1, "{id:?}");
        }
    }
}

#[test]
fn id_ranges_are_disjoint_and_trees_fit() {
    const { assert!(MENU_BAR_BASE + MenuIds::SPAN <= COMMAND_BAR_BASE) };
    const { assert!(COMMAND_BAR_BASE + MenuIds::SPAN <= TREE_BASE) };
    const { assert!(TREE_BASE + TREE_SPAN <= EXTRUDE_BASE) };
    const { assert!(EXTRUDE_BASE + EXTRUDE_SPAN <= SELECTION_BASE) };
    const { assert!(SELECTION_BASE + 14 <= FORMULA_BASE) };
    const { assert!(FEATURES.len() <= MAX_NODES && ELEMENTS.len() <= MAX_NODES) };
    for tree in [Tree::Features, Tree::Elements] {
        for index in 0..tree.nodes().len() {
            assert_eq!(tree_decode(tree_id(tree, index)), Some((tree, index)));
        }
    }
    assert_eq!(tree_decode(tree_id(Tree::Features, FEATURES.len())), None);
    assert_eq!(tree_decode(WidgetId::new(0)), None);
}

#[test]
fn tiny_and_degenerate_grids_are_safe() {
    for cols in 0..=30 {
        for rows in 0..=30 {
            let mut s = ShellState::new(cols, rows);
            let g = s.compose().unwrap();
            assert_eq!((g.cols(), g.rows()), (cols, rows));
            assert_eq!(g, s.compose().unwrap());
            for (col, row) in [(0, 0), (1, 1), (cols / 2, rows / 2), (cols, rows)] {
                let hit = hit_at(&g, col, row);
                s.pointer_moved(hit);
                s.click(hit);
            }
            for k in [
                ShellKey::MenuBar,
                ShellKey::Down,
                ShellKey::Right,
                ShellKey::Enter,
                ShellKey::CommandBar,
                ShellKey::Down,
                ShellKey::Escape,
                ShellKey::Tab,
                ShellKey::Enter,
                ShellKey::Alt('v'),
                ShellKey::Down,
            ] {
                s.key(k);
                s.compose().unwrap();
            }
            s.selection.open(&mut s.interaction);
            s.sync();
            s.compose().unwrap();
            s.key(ShellKey::Tab);
            s.key(ShellKey::Escape);
            s.formula.open(&mut s.interaction);
            s.sync();
            s.compose().unwrap();
        }
    }
}

#[test]
fn huge_grids_do_not_overflow() {
    let s = ShellState::new(u32::MAX, u32::MAX);
    assert!(s.compose().is_err());
    let layout = Layout::new(u32::MAX, u32::MAX);
    assert!(layout.viewport.width() > 0);
    let _ = layout.action_panel(usize::MAX);
}

// ---- keys ----

#[test]
fn control_characters_never_become_typed_keys() {
    for c in [
        '\r', '\n', '\t', '\u{1b}', '\u{7f}', '\u{8}', '\u{0}', '\u{85}',
    ] {
        assert_eq!(ShellKey::character(c), None, "{c:?}");
    }
    assert_eq!(ShellKey::character(' '), Some(ShellKey::Char(' ')));
    assert_eq!(ShellKey::character('é'), Some(ShellKey::Char('é')));
    assert_eq!(ShellKey::alt('f'), Some(ShellKey::Alt('f')));
    assert_eq!(ShellKey::alt('é'), None);
    assert_eq!(ShellKey::alt(' '), None);

    // A directly constructed key is still refused by the shell.
    let mut s = shell();
    let g = s.compose().unwrap();
    let (col, row) = find(&g, "☰").unwrap();
    click_at(&mut s, col, row);
    let draft = s.formula().draft().to_owned();
    assert!(!s.key(ShellKey::Char('\n')));
    assert!(!s.key(ShellKey::Char('\u{7f}')));
    assert_eq!(s.formula().draft(), draft);
    assert!(s.key(ShellKey::Char('é')));
}

#[test]
fn progress_bar_matches_the_reference_table() {
    let expected = [
        "     ",
        "░    ",
        "▒    ",
        "▓    ",
        "█    ",
        "█░   ",
        "█▒   ",
        "█▓   ",
        "██   ",
        "██░  ",
        "██▒  ",
        "██▓  ",
        "███  ",
        "███░ ",
        "███▒ ",
        "███▓ ",
        "████ ",
        "████░",
        "████▒",
        "████▓",
        "█████",
    ];
    for (step, want) in expected.into_iter().enumerate() {
        assert_eq!(
            progress_bar(u32::try_from(step).unwrap() * 5),
            want,
            "{}%",
            step * 5
        );
    }
    assert_eq!(progress_bar(1000), "█████");
    assert_eq!(progress_bar(7), "░    ");
}
