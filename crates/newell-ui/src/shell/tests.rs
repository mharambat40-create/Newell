// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
use super::compose::{Layout, TOP_ROWS};
use super::data::{self, progress_bar, visible_rows};
use super::*;
use crate::MenuAction;
use crate::OverlayKind;
use crate::{CellContent, CellGrid, GridMetrics, UiPoint, WidgetState, hit_test};

const COLS: u32 = 82;
const ROWS: u32 = 25;

fn metrics() -> GridMetrics {
    GridMetrics::new(6, 10).unwrap()
}

fn shell() -> ShellState {
    let mut shell = ShellState::new(COLS, ROWS);
    shell.set_status(98.0, 65);
    shell
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
    super::tree_tests::published_click(s, hit_at(&g, col, row))
}

fn click_text(s: &mut ShellState, needle: &str) -> bool {
    let g = s.compose().unwrap();
    let (col, row) = find(&g, needle).expect("text must be on screen");
    super::tree_tests::published_click(s, hit_at(&g, col, row))
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
        "Datum",
        "Create",
        "Shape",
        "Modify",
        "Pattern",
        "Utilities",
    ] {
        assert!(command_bar.contains(title), "{title}");
    }
    assert!(text(&g, 5).contains("[-]Features"));
    assert!(text(&g, 15).contains("[-]Elements"));
    assert!(text(&g, 5).contains("3D Viewport"));
    assert!(
        text(&g, 23)
            .contains("Roue V1* │ Part Design │ Fully Constrained │ 98fps │ Progress [███░ ] 65%")
    );
    // The Extrude panel is open at the reference position, under the NavCube.
    assert!(has(&g, "Solid") && has(&g, "Depth") && has(&g, "Symetric"));
    assert!(
        has(&g, "OK") && has(&g, "Cancel"),
        "the panel keeps its OK / Cancel row at 82x25"
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
    assert_eq!(glyph(&g, 72, 5), '╭');
    assert_eq!(glyph(&g, 80, 5), '╮');
    assert_eq!(glyph(&g, 72, 8), '╰');
    assert_eq!(glyph(&g, 80, 8), '╯');
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
    assert_eq!(glyph(&g, 1, 2), '╭');
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
    let (col, row) = find(&g, "Symetric").unwrap();
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
    click_text(&mut s, "Symetric");
    assert_eq!(s.extrude, before);
    assert!(!s.menu_bar.is_active());
}

#[test]
fn view_menu_rows_cycle_and_toggle_without_closing() {
    let mut s = shell();
    click_text(&mut s, "View");
    let g = s.compose().unwrap();
    assert!(has(&g, "Perspective") && has(&g, "Wireframe") && has(&g, "■ Tree panel"));
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
fn theme_pointer_and_keyboard_activation_keep_view_open_and_focused() {
    let mut s = shell();
    assert_eq!(s.theme(), ThemeChoice::Dark);
    assert_eq!(s.shading(), 2);
    click_text(&mut s, "View");
    let theme_id = BAR_IDS.item(2, 3);

    assert!(click_text(&mut s, "Theme [ ◑ Dark  ]"));
    assert_eq!(s.theme(), ThemeChoice::Light);
    assert_eq!(s.menu_bar.open_menu(), Some(2));
    assert_eq!(s.interaction().focused, theme_id);
    assert!(has(&s.compose().unwrap(), "Theme [ ◐ Light ]"));

    assert!(click_text(&mut s, "Theme [ ◐ Light ]"));
    assert_eq!(s.theme(), ThemeChoice::Dark);
    assert_eq!(s.menu_bar.open_menu(), Some(2));
    assert_eq!(s.interaction().focused, theme_id);

    keys(&mut s, &[ShellKey::Enter]);
    assert_eq!(s.theme(), ThemeChoice::Light);
    assert_eq!(s.menu_bar.open_menu(), Some(2));
    assert_eq!(s.interaction().focused, theme_id);
    keys(&mut s, &[ShellKey::Enter]);
    assert_eq!(s.theme(), ThemeChoice::Dark);
    assert_eq!(s.menu_bar.open_menu(), Some(2));
    assert_eq!(s.interaction().focused, theme_id);
}

#[test]
fn ui_scale_endpoint_arrows_are_blank_inert_and_bounded_for_pointer_input() {
    for (endpoint, factor, hidden, active, next_factor) in [
        (
            0,
            SCALE_FACTORS_PERCENT[0],
            BAR_IDS.cycle_prev(2, 2).unwrap(),
            BAR_IDS.cycle_next(2, 2).unwrap(),
            SCALE_FACTORS_PERCENT[1],
        ),
        (
            SCALE_FACTORS_PERCENT.len() - 1,
            SCALE_FACTORS_PERCENT[SCALE_FACTORS_PERCENT.len() - 1],
            BAR_IDS.cycle_next(2, 2).unwrap(),
            BAR_IDS.cycle_prev(2, 2).unwrap(),
            SCALE_FACTORS_PERCENT[SCALE_FACTORS_PERCENT.len() - 2],
        ),
    ] {
        let mut s = shell();
        s.view.scale = endpoint;
        assert!(click_text(&mut s, "View"));
        let g = s.compose().unwrap();
        let (start, row) = find(&g, "UI Scale").unwrap();
        let hidden_col = if endpoint == 0 { start + 9 } else { start + 16 };
        let active_col = if endpoint == 0 { start + 16 } else { start + 9 };
        assert_eq!(glyph(&g, hidden_col, row), ' ');
        assert_eq!(
            hit_at(&g, hidden_col, row).and_then(|hit| hit.widget_id),
            Some(BAR_IDS.popup()),
            "masked arrow cell belongs only to the popup surface"
        );
        assert_eq!(
            hit_at(&g, active_col, row).and_then(|hit| hit.widget_id),
            Some(active)
        );
        assert_eq!(
            glyph(&g, active_col, row),
            if endpoint == 0 { '▶' } else { '◀' }
        );

        let (hovered, lit) = s.compose_hovered(Some(hidden)).unwrap();
        assert!(!lit, "masked arrow has no hover feedback");
        assert_eq!(hovered.get(hidden_col, row), g.get(hidden_col, row));

        assert!(click_at(&mut s, hidden_col, row));
        assert_eq!(s.scale_factor_percent(), factor, "hidden arrow is inert");
        assert_eq!(s.menu_bar.open_menu(), Some(2));
        assert!(click_at(&mut s, active_col, row));
        assert_eq!(s.scale_factor_percent(), next_factor);
        assert_eq!(s.menu_bar.open_menu(), Some(2));
    }
}

#[test]
fn ui_scale_keyboard_steps_between_presets_without_wrapping() {
    let mut s = shell();
    s.key(ShellKey::Alt('v'));
    keys(&mut s, &[ShellKey::Down, ShellKey::Down]); // UI Scale row

    assert_eq!(s.scale_factor_percent(), SCALE_FACTORS_PERCENT[1]);
    s.key(ShellKey::Right);
    assert_eq!(s.scale_factor_percent(), SCALE_FACTORS_PERCENT[2]);
    s.key(ShellKey::Right);
    assert_eq!(s.scale_factor_percent(), SCALE_FACTORS_PERCENT[3]);
    s.key(ShellKey::Right);
    assert_eq!(s.scale_factor_percent(), SCALE_FACTORS_PERCENT[3]);

    s.key(ShellKey::Left);
    assert_eq!(s.scale_factor_percent(), SCALE_FACTORS_PERCENT[2]);
    s.key(ShellKey::Left);
    assert_eq!(s.scale_factor_percent(), SCALE_FACTORS_PERCENT[1]);
    s.key(ShellKey::Left);
    assert_eq!(s.scale_factor_percent(), SCALE_FACTORS_PERCENT[0]);
    s.key(ShellKey::Left);
    assert_eq!(s.scale_factor_percent(), SCALE_FACTORS_PERCENT[0]);
}

#[test]
fn view_visibility_toggles_expand_the_viewport_without_moving_its_scene_anchor() {
    let mut s = shell();
    let initial = s.scene_viewport();
    s.view.checks[0] = false; // Tree panel
    let no_tree = s.scene_viewport();
    assert_eq!(no_tree.col(), 1);
    assert!(no_tree.width() > initial.width());
    s.view.checks[2] = false; // Status bar
    let no_status = s.scene_viewport();
    assert!(no_status.height() > no_tree.height());
    s.view.checks[3] = false; // Navigation cube
    assert!(!has(&s.compose().unwrap(), "NavCube"));
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
    // Shading, UI Scale, Theme and Color precede Tree panel.
    keys(&mut s, &[ShellKey::Down; 5]);
    keys(&mut s, &[ShellKey::Enter]);
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
fn task_bar_toggle_reclaims_two_rows_and_restores_the_layout() {
    let mut s = shell();
    let shown = s.layout();
    assert_eq!(shown.viewport.row(), TOP_ROWS);
    assert_eq!(shown.command.height(), 3);

    assert!(click_text(&mut s, "View"));
    assert!(click_text(&mut s, "Task bar"));
    assert!(!s.view_panel_visible(1));
    let hidden = s.layout();
    assert_eq!(hidden.command.height(), 0);
    assert_eq!(shown.menu, hidden.menu);
    assert_eq!(shown.status, hidden.status);
    assert_eq!(shown.viewport.row() - hidden.viewport.row(), 2);
    assert_eq!(shown.features.row() - hidden.features.row(), 2);

    assert!(click_text(&mut s, "Task bar"));
    assert!(s.view_panel_visible(1));
    assert_eq!(s.layout(), shown);
}

#[test]
fn hidden_task_bar_keeps_the_pane_divider_under_the_pointer() {
    let mut s = shell();
    assert!(click_text(&mut s, "View"));
    assert!(click_text(&mut s, "Task bar"));
    assert!(s.key(ShellKey::Escape));
    assert_eq!(s.layout().features.row(), 2);

    assert!(s.drag_separator(Separator::Split, 5, 12));
    assert_eq!(s.layout().elements.row(), 12);
    assert_eq!(s.separator_at(5, 12), Some(Separator::Split));
}

#[test]
fn hiding_task_bar_closes_its_popup_clears_focus_and_blocks_f6() {
    let mut s = shell();
    assert!(s.command_bar.open(&COMMAND_MENUS, 2));
    s.sync();
    assert!(s.command_bar.is_active());
    assert_eq!(s.command_bar.open_menu(), Some(2));
    assert!(
        s.interaction()
            .focused
            .is_some_and(|id| COMMAND_IDS.decode(id).is_some())
    );
    assert!(
        s.overlays
            .top()
            .is_some_and(|overlay| overlay.id == COMMAND_IDS.popup())
    );

    assert!(s.apply_menu(MenuResult::Toggled(MenuAction::new(ACT_VIEW_CHECK + 1))));
    s.sync();
    assert!(!s.command_bar.is_active());
    assert!(s.overlays.top().is_none());
    assert!(s.interaction().focused.is_none());

    assert!(s.key(ShellKey::CommandBar));
    assert!(!s.command_bar.is_active());
    assert!(s.interaction().focused.is_none());
    assert!(s.key(ShellKey::Alt('f')));
    assert!(s.menu_bar.is_active());
    assert!(has(&s.compose().unwrap(), "╭─File"));
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

    click_text(&mut s, "Modify");
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
    click_text(&mut s, "Symetric");
    assert!(s.extrude().values.symmetric);
    click_text(&mut s, "Create");
    click_text(&mut s, "[E]xtrude");
    assert!(
        s.extrude().values.symmetric,
        "an open panel keeps its edits"
    );

    click_text(&mut s, "Cancel");
    assert!(!s.extrude().is_open() && !has(&s.compose().unwrap(), "Symetric"));
    click_text(&mut s, "Create");
    click_text(&mut s, "[E]xtrude");
    assert!(s.extrude().is_open() && !s.extrude().values.symmetric);
    assert!(has(&s.compose().unwrap(), "Symetric"));
}

// ---- trees ----

#[test]
fn navigation_trees_start_as_in_the_reference() {
    // At 82x25 the Features pane is clipped after `Chamfer_Edges` (the full panel is compared
    // with the reference at 82x28 in `tree_tests`).
    let g = shell().compose().unwrap();
    for label in [
        "Features",
        "Sketch_Profile_Ma…",
        "Hub_Features",
        "Extrude_Base",
        "Sketch_Hole",
        "Extrude_Cut_Center",
        "Fillet_Inner",
        "Chamfer_Edges",
        "Elements",
        "Solid Bodies(2)",
        "Rim_Body",
        "Hub_Body",
        "Surfaces(0)",
        "[+]Construct",
    ] {
        assert!(has(&g, label), "{label}");
    }
    assert!(text(&g, 6).contains("├ □ Sketch_Profile_Ma…║"));
    assert!(text(&g, 7).contains("├[-] Hub_Features"));
    assert!(text(&g, 8).contains("│ ├ Extrude_Base"));
    assert!(text(&g, 9).contains("│ ├ ■ Sketch_Hole"));
    assert!(text(&g, 10).contains("│ ╰╴Extrude_Cut_Center║"));
    assert!(text(&g, 12).contains("│ ─────────────────── ║"));
    assert!(text(&g, 20).contains("╰[+]Construct"));
    assert!(!has(&g, "Plane_XY"), "Construct starts collapsed");
}

#[test]
fn tree_expand_and_collapse_keep_ids_and_trees_independent() {
    let mut s = shell();
    let fillet = s
        .tree_hit(id_of(&s, "Fillet_Inner"), ProjectionGeneration::new(0))
        .unwrap()
        .target;
    let rim = id_of(&s, "Rim_Body");

    assert!(click_text(&mut s, "[-] Hub_Features"));
    let g = s.compose().unwrap();
    assert!(!has(&g, "Extrude_Base") && !has(&g, "Sketch_Hole"));
    assert!(text(&g, 7).contains("├[+] Hub_Features"));
    assert!(has(&g, "Rim_Body"), "the Elements tree is unaffected");
    assert_eq!(
        s.tree_hit(id_of(&s, "Fillet_Inner"), ProjectionGeneration::new(0))
            .unwrap()
            .target,
        fillet,
        "semantic identity ignores position"
    );
    assert_eq!(id_of(&s, "Rim_Body"), rim);

    click_text(&mut s, "[+] Hub_Features");
    assert!(has(&s.compose().unwrap(), "Extrude_Base"));

    click_text(&mut s, "[+]Construct");
    let g = s.compose().unwrap();
    assert!(has(&g, "Plane_XY") && has(&g, "[-]Construct"));

    click_text(&mut s, "[-]Features");
    let g = s.compose().unwrap();
    assert!(!has(&g, "Hub_Features") && !has(&g, "Chamfer_Edges"));
    assert!(!has(&g, "│ ─────"), "no playhead without features");
    assert!(has(&g, "Plane_XY"), "Elements still expanded");
}

#[test]
fn tree_leaf_click_selects_and_focuses_without_changing_the_tree() {
    let mut s = shell();
    let before = s.expanded.clone();
    assert!(click_text(&mut s, "Extrude_Base"));
    assert_eq!(s.expanded.clone(), before);
    assert_eq!(s.interaction().focused, Some(tree_id(Tree::Features, 3)));
    assert_eq!(
        s.selected,
        Some(
            s.projection
                .target(TreeTargetKind::Operation(OpaqueTreeId::new(2)))
        )
    );
}

#[test]
fn visible_rows_compute_guides_from_the_flat_list() {
    let open = initial_open_features();
    let rows = visible_rows(&shell().nodes[0], &open);
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
    let hidden: Vec<usize> = visible_rows(&shell().nodes[0], &closed)
        .iter()
        .map(|r| r.index)
        .collect();
    assert_eq!(hidden, vec![0, 1, 2, 6, 7, 8]);
}

fn initial_open_features() -> Vec<bool> {
    let shell = shell();
    shell.nodes[0]
        .iter()
        .map(|node| shell.expanded.contains(&node.target))
        .collect()
}

// ---- Extrude integration ----

/// A shell at the minimum grid height, tall enough for the fully disclosed Action Box.
fn tall() -> ShellState {
    ShellState::new(COLS, 30)
}

#[test]
fn extrude_panel_discloses_rows_through_the_shell() {
    let mut s = tall();
    assert!(click_text(&mut s, "Add"));
    assert!(has(&s.compose().unwrap(), "Merge with all"));
    assert!(click_text(&mut s, "Starting Offset"));
    assert!(has(&s.compose().unwrap(), "Dist"));
    assert!(click_text(&mut s, "Merge with all"));
    assert!(has(&s.compose().unwrap(), "Targets"));

    // The first `▶` on screen is the termination cycle.
    let g = s.compose().unwrap();
    let (col, row) = (0..30)
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
    click_text(&mut s, "Symetric");
    click_text(&mut s, "OK");
    assert!(!s.extrude().is_open() && !has(&s.compose().unwrap(), "Starting Offset"));
    assert_eq!(s.interaction().focused, None);

    let mut s = shell();
    click_text(&mut s, "Symetric");
    click_text(&mut s, "Cancel");
    assert_eq!(s.extrude().status, crate::ExtrudeStatus::Cancelled);
    assert!(
        !s.extrude().values.symmetric,
        "values restored to the opening ones"
    );
}

const EXAMPLE_TXT: &str = include_str!("../../../../docs/ui_design/Interface_Exemple.txt");

/// The Action Box of `Interface_Exemple.txt` (82 x 28): rows 9..25, columns 50..81. Reference
/// row 19 is the hovered Starting Offset row (`README_UI.md` §25); the default shell has no
/// pointer, so its two hover slots hold the normal `│`.
fn example_box() -> Vec<String> {
    let mut rows: Vec<String> = EXAMPLE_TXT
        .lines()
        .skip(9)
        .take(16)
        .map(|l| l.chars().skip(50).take(31).collect())
        .collect();
    rows[9] = rows[9].replace(['╞', '╡'], "│");
    rows
}

#[test]
fn action_box_equals_the_interface_example_at_the_reference_and_minimum_grids() {
    let want = example_box();
    assert_eq!(want.len(), 16);
    assert_eq!(want[0], "╭─Extrude─────┬───────────────╮");
    for (cols, rows) in [(82, 28), (82, 30), (120, 40)] {
        let g = ShellState::new(cols, rows).compose().unwrap();
        // Right-aligned under the NavCube: the box ends at the viewport's inner right edge.
        let right = cols - 1;
        let left = right - 31;
        let got: Vec<String> = (9..25)
            .map(|r| text(&g, r).chars().skip(left as usize).take(31).collect())
            .collect();
        assert_eq!(got, want, "{cols}x{rows}");
        // Frame cells of the viewport stay intact on both sides of the box.
        assert_eq!(glyph(&g, right, 12), '║', "{cols}x{rows}");
    }
}

#[test]
fn action_box_is_fixed_width_whatever_the_grid() {
    for cols in [82, 100, 160] {
        let s = ShellState::new(cols, 30);
        assert_eq!(s.action_width(), 31, "{cols}");
    }
}

#[test]
fn keyboard_focus_visits_the_action_box_controls_in_reading_order() {
    let mut s = shell();
    let g = s.compose().unwrap();
    let ring = s.focus_ring(&g);
    let boxed: Vec<WidgetId> = ring
        .into_iter()
        .filter(|id| id.get() >= ACTION_BASE && id.get() < ACTION_BASE + ID_SPAN)
        .collect();
    let id = |key: u32, slot: u32| s.extrude().id(key, slot);
    use crate::extrude::{K_ACTIONS, K_GEOMETRY, K_PROFILE, K_RESULT};
    assert_eq!(
        boxed[..7],
        [
            id(K_GEOMETRY, 0),
            id(K_GEOMETRY, 1),
            id(K_RESULT, 0),
            id(K_RESULT, 1),
            id(K_RESULT, 2),
            id(K_PROFILE, 0),
            s.extrude().id(crate::extrude::K_EXTEND, 0),
        ]
    );
    assert_eq!(
        boxed[boxed.len() - 2..],
        [id(K_ACTIONS, 0), id(K_ACTIONS, 1)]
    );
    // Space on the focused `Surface` segment selects it, like a click.
    s.interaction.set_focused(Some(id(K_GEOMETRY, 1)));
    assert!(s.key(ShellKey::Char(' ')));
    assert_eq!(
        s.extrude().values.geometry,
        crate::ExtrudeGeometryType::Surface
    );
    assert!(has(
        &s.compose().unwrap(),
        "╭─Extrude─────┬───────────────╮"
    ));
}

#[test]
fn clicking_header_borders_routes_to_the_adjacent_segment() {
    let mut s = tall();
    let g = s.compose().unwrap();
    // Row 10 is `│    Solid    │    Surface    │` of the box that starts at column 50.
    let m = metrics();
    let divider = 50 + 14;
    let hit = |x| {
        hit_test(UiPoint { x, y: 10 * 10 + 5 }, m, &g)
            .and_then(|h| h.widget_id)
            .unwrap()
    };
    let (left_half, right_half) = (divider * 6, divider * 6 + 5);
    assert_eq!(
        hit(left_half),
        s.extrude().id(crate::extrude::K_GEOMETRY, 0)
    );
    assert_eq!(
        hit(right_half),
        s.extrude().id(crate::extrude::K_GEOMETRY, 1)
    );
    assert!(click_at(&mut s, divider + 3, 10));
    assert_eq!(
        s.extrude().values.geometry,
        crate::ExtrudeGeometryType::Surface
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
    assert!(click_selection_field(s));
}

/// Clicks inside the `Selection` field of the `Up to` branch (`[Select…       ]`).
fn click_selection_field(s: &mut ShellState) -> bool {
    let g = s.compose().unwrap();
    let (col, row) = find(&g, "Selection ").unwrap();
    click_at(s, col + 12, row)
}

fn click_selection_on_row(s: &mut ShellState, label: &str) -> bool {
    let g = s.compose().unwrap();
    let (start, row) = find(&g, label).unwrap();
    let col = text(&g, row)
        .chars()
        .enumerate()
        .skip(start as usize)
        .find_map(|(col, ch)| (ch == '[').then_some(col as u32))
        .unwrap();
    click_at(s, col, row)
}

/// Opens the ordered Profile Selection box from the Action Box `Profile` field.
fn open_profile_selection(s: &mut ShellState) -> WidgetId {
    let profile = s.extrude().id(crate::extrude::K_PROFILE, 0);
    let g = s.compose().unwrap();
    let (col, row) = cell_of(&g, profile);
    assert!(click_at(s, col, row));
    profile
}

#[test]
fn selection_field_opens_the_selection_panel() {
    let mut s = shell();
    open_selection(&mut s);
    assert!(s.selection().is_open());
    let g = s.compose().unwrap();
    assert!(has(&g, "Flange_Body") && has(&g, "Selection"));
    assert!(!s.selection().is_profile_picker());
    assert_eq!(s.interaction().focused, s.selection().focus());
    // The Extrude controls are still the real ones underneath, unchanged.
    assert_eq!(s.extrude().values.selection, crate::MockSelection::None);
}

#[test]
fn axis_tools_and_merge_targets_keep_the_generic_checkbox_modal() {
    for (operation, label) in [
        (ActionOperation::Revolve, "Axis"),
        (ActionOperation::Boolean, "Tools"),
        (ActionOperation::Revolve, "Targets"),
    ] {
        let mut s = shell();
        s.operation = operation;
        if label == "Targets" {
            s.revolve.values.result = crate::ExtrudeResultMode::Add;
            s.revolve.values.merge_with_all = false;
        }
        assert!(
            click_selection_on_row(&mut s, label),
            "{operation:?}: {label}"
        );
        assert!(s.selection().is_open(), "{operation:?}: {label}");
        assert!(!s.selection().is_profile_picker());
        let g = s.compose().unwrap();
        assert!(has(&g, "Selection") && has(&g, "□ Rim_Body"));
        assert!(!has(&g, "╭─Profils"));
    }
}

#[test]
fn profile_field_opens_the_exact_profile_selection_box() {
    let mut s = shell();
    let field = open_profile_selection(&mut s);
    assert!(s.selection().is_profile_picker());
    let overlay = s.overlays.top().expect("Profile Selection modal");
    assert_eq!(overlay.kind, OverlayKind::Modal);
    assert_eq!(overlay.rect, s.profile_selection_frame());
    assert!(
        s.scene_occlusions()
            .contains(&overlay.rect.intersection(&s.scene_viewport()))
    );
    assert_eq!(s.interaction().focused, s.selection().focus());

    let g = s.compose().unwrap();
    let (left, top) = find(&g, "╭─Profils─────────────────────╮").unwrap();
    let rows = [
        "╭─Profils─────────────────────╮",
        "│┌───────────────────────────┐│",
        "││ ☰ Spline_1             × ▲││",
        "││ ☰ Profil_3             × ┃││",
        "││ ☰ Profil_4             ×  ││",
        "││ ☰ Profil_4             ×  ││",
        "││ ☰ Compound_line_1      × ▼││",
        "│└───────────────────────────┘│",
        "│─────────────────────────────│",
        "│      OK      │    Cancel    │",
        "╰─────────────────────────────╯",
    ];
    for (offset, expected) in rows.into_iter().enumerate() {
        let actual: String = text(&g, top + offset as u32)
            .chars()
            .skip(left as usize)
            .take(31)
            .collect();
        assert_eq!(actual, expected, "Profile Selection row {offset}");
    }
    let tree = find(&g, "Hub_Features").unwrap();
    assert_eq!(hit_at(&g, tree.0, tree.1).unwrap().widget_id, None);
    assert!(!click_at(&mut s, tree.0, tree.1));
    assert!(s.selection().is_profile_picker());
    assert_eq!(s.interaction().focused, s.selection().focus());
    assert_ne!(s.interaction().focused, Some(field));
}

#[test]
fn every_profile_field_opens_the_profile_selection_box() {
    for operation in [
        ActionOperation::Extrude,
        ActionOperation::Revolve,
        ActionOperation::Loft,
        ActionOperation::Sweep,
    ] {
        let mut s = shell();
        s.operation = operation;
        open_profile_selection(&mut s);
        assert!(s.selection().is_profile_picker(), "{operation:?}");
        assert!(has(
            &s.compose().unwrap(),
            "╭─Profils─────────────────────╮"
        ));
    }
}

#[test]
fn profile_selection_scroll_escape_ok_and_cancel_keep_selection_semantics() {
    let mut s = shell();
    let field = open_profile_selection(&mut s);
    assert!(s.scroll(-1));
    let g = s.compose().unwrap();
    assert!(has(&g, "☰ Profil_3") && !has(&g, "☰ Spline_1"));

    s.key(ShellKey::Char(' '));
    assert!(
        !s.selection().profile_applied()[0],
        "Profile edit is still a draft"
    );
    s.key(ShellKey::Escape);
    assert!(!s.selection().is_open());
    assert!(
        !s.selection().profile_applied()[0],
        "Escape discards the draft"
    );
    assert_eq!(s.interaction().focused, Some(field));

    let _ = open_profile_selection(&mut s);
    s.key(ShellKey::Char(' '));
    let ok = WidgetId::new(SELECTION_BASE + 3);
    let g = s.compose().unwrap();
    let (col, row) = cell_of(&g, ok);
    assert!(click_at(&mut s, col, row));
    assert!(s.selection().profile_applied()[0], "OK applies the draft");
    assert!(
        !s.selection().applied()[0],
        "generic checkbox state is unchanged"
    );
    assert_eq!(s.interaction().focused, Some(field));

    let _ = open_profile_selection(&mut s);
    s.key(ShellKey::Char(' '));
    let cancel = WidgetId::new(SELECTION_BASE + 4);
    let g = s.compose().unwrap();
    let (col, row) = cell_of(&g, cancel);
    assert!(click_at(&mut s, col, row));
    assert!(
        s.selection().profile_applied()[0],
        "Cancel preserves the applied draft"
    );
    assert_eq!(s.interaction().focused, Some(field));
}

#[test]
fn formula_button_opens_the_formula_editor() {
    let mut s = shell();
    let g = s.compose().unwrap();
    let (col, row) = find(&g, "☰").unwrap();
    assert!(click_at(&mut s, col, row));
    assert!(s.formula().is_open() && !s.selection().is_open());
    let g = s.compose().unwrap();
    assert!(
        has(&g, "╭─Formula") && has(&g, "│┌───"),
        "outlined and empty"
    );
    assert!(!has(&g, "Fx"));
    assert_eq!(s.formula().draft(), "");
    assert_eq!(s.interaction().focused, s.formula().focus());
}

#[test]
fn formula_modal_footer_uses_the_documented_hover_frame() {
    let mut s = shell();
    click_text(&mut s, "☰");
    let cancel = id_of(&s, "Cancel");
    let g = s.compose().unwrap();
    let (_, row) = find(&g, "Cancel").unwrap();
    let (hovered, lit) = s.compose_hovered(Some(cancel)).unwrap();
    assert!(lit);
    assert!(text(&hovered, row).contains('╞'));
    assert!(text(&hovered, row).contains('╡'));
}

#[test]
fn modal_blocks_pointer_and_keyboard_to_the_base_shell() {
    let mut s = shell();
    let g = s.compose().unwrap();
    let (col, row) = find(&g, "☰").unwrap();
    // Uncovered spots: a tree row, both bars, and the right part of the Symetric row (the
    // modal spans columns 21..61).
    let invert_row = find(&g, "Symetric").unwrap().1;
    let spots: Vec<(&str, (u32, u32))> = vec![
        ("Hub_Features", find(&g, "Hub_Features").unwrap()),
        ("Workbench", find(&g, "Workbench").unwrap()),
        ("File", find(&g, "File").unwrap()),
        ("Symetric row", (70, invert_row)),
    ];
    assert!(g.get(70, invert_row).unwrap().widget_id.is_some());
    click_at(&mut s, col, row);
    let g = s.compose().unwrap();

    // Covered and uncovered controls alike have no owner outside the modal.
    for &(needle, (c, r)) in &spots {
        assert_eq!(hit_at(&g, c, r).unwrap().widget_id, None, "{needle}");
    }
    let base = (s.extrude, s.expanded.clone());
    for &(_, (c, r)) in &spots {
        assert!(!click_at(&mut s, c, r));
    }
    assert!(s.formula().is_open());
    assert_eq!(base, (s.extrude, s.expanded.clone()));

    // Keys go to the modal: base shortcuts do nothing, text is typed.
    assert!(!s.key(ShellKey::MenuBar) && !s.menu_bar.is_active());
    assert!(!s.key(ShellKey::Alt('f')) && !s.menu_bar.is_active());
    s.key(ShellKey::Char('x'));
    assert!(s.formula().draft().ends_with('x'));
    s.key(ShellKey::Tab);
    assert_eq!(s.interaction().focused, s.formula().focus());
    assert!(s.formula().is_open());
    assert_eq!(base, (s.extrude, s.expanded.clone()));
}

#[test]
fn formula_cancel_and_ok_restore_focus_and_text() {
    let mut s = shell();
    let button = id_of(&s, "☰");
    click_text(&mut s, "☰");
    type_str(&mut s, "ab");
    s.key(ShellKey::Escape);
    assert!(!s.formula().is_open());
    assert_eq!(s.formula().text(), "");
    assert_eq!(
        s.interaction().focused,
        Some(button),
        "prior focus restored"
    );
    assert!(s.overlays.is_empty());

    click_text(&mut s, "☰");
    assert_eq!(s.formula().draft(), "", "the cancelled draft was dropped");
    type_str(&mut s, "ab");
    s.key(ShellKey::Enter);
    assert_eq!(s.formula().text(), "ab");
    assert_eq!(s.interaction().focused, Some(button));

    click_text(&mut s, "☰");
    assert_eq!(
        s.formula().draft(),
        "ab",
        "the field keeps its confirmed draft"
    );
    s.key(ShellKey::Escape);
}

#[test]
fn formula_drafts_belong_to_their_field() {
    let mut s = shell();
    // Toggle `Starting Offset` on so a second numeric row (`Dist`) is shown.
    click_text(&mut s, "Starting Offset");
    let g = s.compose().unwrap();
    let rows: Vec<(u32, u32)> = (0..ROWS)
        .flat_map(|r| (0..COLS).map(move |c| (c, r)))
        .filter(|&(c, r)| glyph(&g, c, r) == '☰')
        .collect();
    assert_eq!(rows.len(), 2, "two numeric fields");
    click_at(&mut s, rows[0].0, rows[0].1);
    type_str(&mut s, "first");
    s.key(ShellKey::Enter);
    click_at(&mut s, rows[1].0, rows[1].1);
    assert_eq!(s.formula().draft(), "", "the other field is independent");
    type_str(&mut s, "second");
    s.key(ShellKey::Enter);
    click_at(&mut s, rows[0].0, rows[0].1);
    assert_eq!(s.formula().draft(), "first");
    s.key(ShellKey::Escape);
    click_at(&mut s, rows[1].0, rows[1].1);
    assert_eq!(s.formula().draft(), "second");
}

fn depth_field(s: &ShellState) -> (u32, u32) {
    let g = s.compose().unwrap();
    let (col, row) = find(&g, "Depth").unwrap();
    (col + 12, row)
}

#[test]
fn numeric_field_edits_with_caret_limits_sign_and_commit() {
    let mut s = shell();
    let (col, row) = depth_field(&s);
    let g = s.compose().unwrap();
    assert!(has(&g, "[+|0.0 mm       ☰]") && !has(&g, "▏"));
    // Clicking the field starts the edit; the caret appears in its own cell at the end.
    assert!(click_at(&mut s, col, row));
    assert!(s.editing_field().is_some());
    assert!(has(&s.compose().unwrap(), "[+|0.0▏ mm      ☰]"));
    keys(&mut s, &[ShellKey::Edit(EditKey::SelectAll)]);
    type_str(&mut s, "12345678.99");
    assert!(
        has(&s.compose().unwrap(), "[+|12345678.9▏ m☰]"),
        "limit, caret, clipped suffix"
    );
    keys(&mut s, &[ShellKey::Edit(EditKey::Home)]);
    assert!(has(&s.compose().unwrap(), "[+|▏12345678.9 m☰]"));
    type_str(&mut s, "9");
    assert!(
        has(&s.compose().unwrap(), "[+|▏12345678.9 m☰]"),
        "ninth digit refused"
    );
    type_str(&mut s, "-");
    assert!(has(&s.compose().unwrap(), "[-|▏12345678.9 m☰]"));
    keys(&mut s, &[ShellKey::Enter]);
    assert!(s.editing_field().is_none());
    let g = s.compose().unwrap();
    assert!(has(&g, "[-|12345678.9 mm☰]") && !has(&g, "▏"));
}

#[test]
fn clicking_a_numeric_direction_marker_reverses_it_without_editing() {
    let mut s = shell();
    let g = s.compose().unwrap();
    let (bracket, row) = find(&g, "[+|0.0 mm").expect("directional depth field");
    // The marker is immediately after the opening `[`.
    assert!(click_at(&mut s, bracket + 1, row));
    assert!(has(&s.compose().unwrap(), "[-|0.0 mm       ☰]"));
    assert!(s.editing_field().is_none());
    assert!(click_at(&mut s, bracket + 1, row));
    assert!(has(&s.compose().unwrap(), "[+|0.0 mm       ☰]"));
}

#[test]
fn numeric_field_escape_reverts_and_other_clicks_commit() {
    let mut s = shell();
    let (col, row) = depth_field(&s);
    click_at(&mut s, col, row);
    keys(&mut s, &[ShellKey::Edit(EditKey::SelectAll)]);
    type_str(&mut s, "7");
    keys(&mut s, &[ShellKey::Escape]);
    assert!(has(&s.compose().unwrap(), "[+|0.0 mm       ☰]"));
    click_at(&mut s, col, row);
    keys(&mut s, &[ShellKey::Edit(EditKey::SelectAll)]);
    type_str(&mut s, "7");
    // A click elsewhere (a tree row) keeps the value and ends the edit.
    click_text(&mut s, "Hub_Features");
    assert!(s.editing_field().is_none());
    assert!(has(&s.compose().unwrap(), "[+|7.0 mm       ☰]"));
}

#[test]
fn numeric_editing_owns_keys_and_menus_end_it() {
    let mut s = shell();
    let (col, row) = depth_field(&s);
    click_at(&mut s, col, row);
    // Letters are refused but consumed: they are not menu mnemonics while editing.
    assert!(!s.key(ShellKey::Char('f')));
    assert!(!s.menu_bar.is_active());
    type_str(&mut s, "5");
    s.key(ShellKey::Alt('f'));
    assert!(
        s.editing_field().is_none(),
        "opening a menu keeps the edit first"
    );
    assert!(s.menu_bar.is_active());
    assert!(
        has(&s.compose().unwrap(), "[+|0.0 mm       ☰]"),
        "5 was refused"
    );
}

#[test]
fn numeric_enter_and_space_on_the_focused_field_start_the_edit() {
    let mut s = shell();
    let (col, row) = depth_field(&s);
    click_at(&mut s, col, row);
    keys(&mut s, &[ShellKey::Enter]);
    assert!(s.editing_field().is_none());
    keys(&mut s, &[ShellKey::Enter]);
    assert!(s.editing_field().is_some(), "Enter on the focused field");
    keys(&mut s, &[ShellKey::Tab]);
    assert!(
        s.editing_field().is_none(),
        "Tab ends the edit and moves on"
    );
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
    assert!(click_selection_field(&mut s));
    s.key(ShellKey::Char(' '));
    let g = s.compose().unwrap();
    let (col, row) = cell_of(&g, ok);
    click_at(&mut s, col, row);
    assert!(s.selection().applied()[0]);
    assert_eq!(s.interaction().focused, restored);
}

#[test]
fn selection_is_centered_and_preserves_the_source_action_box() {
    let mut s = shell();
    open_selection(&mut s);
    let overlay = s.overlays.top().expect("selection modal");
    assert_eq!(overlay.kind, OverlayKind::Modal);
    let bounds = GridRect::new(0, 0, COLS, ROWS);
    assert_eq!(overlay.rect, crate::center_modal((30, 12), bounds));
    let g = s.compose().unwrap();
    assert!(has(&g, "Selection") && has(&g, "Flange_Body"));
    assert!(s.extrude().is_open(), "the source operation stays open");
}

#[test]
fn centered_selection_scrolls_without_resizing_the_source_panel() {
    let mut s = shell();
    open_selection(&mut s);
    let source = s.action_panel_rect();
    let overlay = s.overlays.top().unwrap().rect;
    assert!(s.scroll(-1), "nine entries exceed the visible modal list");
    assert_eq!(s.overlays.top().unwrap().rect, overlay);
    assert_eq!(s.action_panel_rect(), source);
}

#[test]
fn centered_modals_occlude_the_scene_without_reframing_it() {
    for formula in [false, true] {
        let mut s = tall();
        let scene = s.scene_viewport();
        if formula {
            click_text(&mut s, "☰");
        } else {
            open_selection(&mut s);
        }
        let source = s.action_panel_rect().unwrap();
        let modal = s.overlays.top().expect("centered modal");
        assert_eq!(modal.kind, OverlayKind::Modal);
        assert_eq!(s.scene_viewport(), scene);
        let covered = s.scene_occlusions();
        assert!(covered.contains(&source.intersection(&scene)));
        assert!(covered.contains(&modal.rect.intersection(&scene)));
    }
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
        last.is_some_and(|id| id.get() >= ACTION_BASE),
        "wrapped to the panel"
    );

    let symmetric = id_of(&s, "Symetric");
    for _ in 0..100 {
        if s.interaction().focused == Some(symmetric) {
            break;
        }
        s.key(ShellKey::Tab);
    }
    assert_eq!(s.interaction().focused, Some(symmetric));
    s.key(ShellKey::Char(' '));
    assert!(s.extrude().values.symmetric);
}

/// Focus and press are logical state with no visual of their own: no marker glyph is invented
/// (`ARCHITECTURE.md` §3.5), so the glyphs and owners of the composed grid do not change.
#[test]
fn focus_and_press_draw_no_marker() {
    let mut s = shell();
    let base = s.compose().unwrap();
    let layer = |g: &CellGrid| {
        g.cells()
            .iter()
            .map(|c| (c.content, c.widget_id, c.shared_border))
            .collect::<Vec<_>>()
    };
    for _ in 0..40 {
        s.key(ShellKey::Tab);
        assert!(s.interaction().focused.is_some());
        assert_eq!(layer(&s.compose().unwrap()), layer(&base));
    }
    let hit = hit_at(&base, 4, 10);
    assert!(s.interaction.press(
        hit.and_then(|h| h.widget_id).unwrap_or(WidgetId::new(1)),
        WidgetState::default()
    ));
    assert_eq!(layer(&s.compose().unwrap()), layer(&base));
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
                (ACTION_BASE..SELECTION_BASE).contains(&id.get()),
                (SELECTION_BASE..FORMULA_BASE).contains(&id.get()),
                (FORMULA_BASE..FORMULA_BASE + 4).contains(&id.get()),
                (RENAME_BASE..RENAME_BASE + 4).contains(&id.get()),
                id.get() == PLAYHEAD_ID,
                id.get() == PROGRESS_ID,
                ShellState::window_action(id).is_some(),
            ];
            assert_eq!(owners.iter().filter(|&&o| o).count(), 1, "{id:?}");
        }
    }
}

#[test]
fn id_ranges_are_disjoint_and_trees_fit() {
    const { assert!(MENU_BAR_BASE + MenuIds::SPAN <= COMMAND_BAR_BASE) };
    const { assert!(COMMAND_BAR_BASE + MenuIds::SPAN <= TREE_BASE) };
    const { assert!(TREE_BASE + TREE_SPAN <= ACTION_BASE) };
    const { assert!(ACTION_BASE + ID_SPAN <= SELECTION_BASE) };
    const { assert!(SELECTION_BASE + 14 <= FORMULA_BASE) };
    const { assert!(FORMULA_BASE + 4 <= RENAME_BASE) };
    const { assert!(RENAME_BASE + 4 <= WINDOW_BASE) };
    const { assert!(PLAYHEAD_ID == TREE_BASE + NODE_IDS && PLAYHEAD_ID < ACTION_BASE) };
    const { assert!(TREE_SLOT_CAP == crate::MAX_CELLS) };
    let state = shell();
    for tree in [Tree::Features, Tree::Elements] {
        for (slot, index) in state.tree_slots[tree.slot()].iter().enumerate() {
            for part in TreePart::ALL {
                assert_eq!(
                    state.decode_tree(tree_part_id(tree, slot, part)),
                    Some((tree, *index, part))
                );
            }
        }
        assert_eq!(
            state.decode_tree(tree_id(tree, state.tree_slots[tree.slot()].len())),
            None
        );
    }
    assert_eq!(tree_decode(WidgetId::new(PLAYHEAD_ID)), None);
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
            s.formula.open(&mut s.interaction, "");
            s.sync();
            s.compose().unwrap();
        }
    }
}

#[test]
fn huge_grids_do_not_overflow() {
    let s = ShellState::new(u32::MAX, u32::MAX);
    assert!(s.compose().is_err());
    let layout = Layout::new(u32::MAX, u32::MAX, PanelWidths::default(), u32::MAX, true, true, true);
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

// ---- M2.17: window controls ----

#[test]
fn window_controls_own_stable_ids_in_their_documented_cells() {
    let g = shell().compose().unwrap();
    let (col, row) = find(&g, "[-] [□] [X] ").unwrap();
    assert_eq!((col, row), (COLS - 1 - 12, 1));
    let owner = |c: u32| {
        g.get(c, 1)
            .unwrap()
            .widget_id
            .and_then(ShellState::window_action)
    };
    for (offset, want) in [
        (0, Some(WindowAction::Minimize)),
        (2, Some(WindowAction::Minimize)),
        (3, None),
        (4, Some(WindowAction::ToggleMaximize)),
        (6, Some(WindowAction::ToggleMaximize)),
        (7, None),
        (8, Some(WindowAction::Close)),
        (10, Some(WindowAction::Close)),
        (11, None),
    ] {
        assert_eq!(owner(col + offset), want, "offset {offset}");
    }
    // Ids are the same at any grid size and state, and distinct from every other control.
    let wide = ShellState::new(120, 40).compose().unwrap();
    let id_at = |g: &CellGrid, c: u32| g.get(c, 1).unwrap().widget_id;
    assert_eq!(id_at(&g, col), id_at(&wide, 120 - 13));
    assert_eq!(ShellState::window_action(WidgetId::new(1)), None);
    assert_eq!(ShellState::window_action(WidgetId::new(u32::MAX)), None);
}

#[test]
fn window_control_click_is_not_a_shell_activation() {
    let mut s = shell();
    let before = s.clone();
    let g = s.compose().unwrap();
    assert!(!s.click(hit_at(&g, COLS - 13 + 8, 1)));
    assert_eq!(s, before);
}

#[test]
fn window_drag_region_is_inert_menu_bar_space_only() {
    let mut s = shell();
    let g = s.compose().unwrap();
    assert!(s.window_drag_at(40, 1, &g));
    assert!(s.window_drag_at(COLS - 2, 1, &g), "trailing space");
    assert!(!s.window_drag_at(0, 1, &g), "frame");
    assert!(!s.window_drag_at(COLS - 1, 1, &g), "frame");
    assert!(!s.window_drag_at(2, 1, &g), "menu title");
    assert!(!s.window_drag_at(COLS - 13, 1, &g), "window control");
    assert!(!s.window_drag_at(40, 0, &g), "border row");
    assert!(!s.window_drag_at(40, 3, &g), "command bar row");
    assert!(!s.window_drag_at(500, 1, &g), "outside the grid");
    s.key(ShellKey::Alt('f'));
    let g = s.compose().unwrap();
    assert!(!s.window_drag_at(40, 1, &g), "a popup is open");
}

#[test]
fn window_controls_are_omitted_when_they_would_cover_a_title() {
    let g = ShellState::new(20, 10).compose().unwrap();
    assert!(!has(&g, "[X]"));
}

#[test]
fn m224_fps_field_handles_invalid_and_boundary_values_without_shifting_cells() {
    for (value, expected) in [
        (0.0, "  0fps "),
        (5.0, "  5fps "),
        (98.0, " 98fps "),
        (99.0, " 99fps "),
        (100.0, "100fps "),
        (144.0, "144fps "),
        (998.5, "999fps "),
        (999.0, "999fps "),
        (1000.0, "999fps "),
        (123456.0, "999fps "),
        (-3.0, "  0fps "),
        (-0.4, "  0fps "),
        (f64::NAN, "  0fps "),
        (f64::INFINITY, "  0fps "),
        (f64::NEG_INFINITY, "  0fps "),
    ] {
        assert_eq!(data::fps_field(value), expected);
        assert_eq!(expected.chars().count(), 7);
        let mut s = shell();
        let before = s.compose().unwrap();
        s.set_status(value, 65);
        let after = s.compose().unwrap();
        let line = text(&before, ROWS - 2);
        let byte = line.find("fps").unwrap();
        let field = line[..byte].chars().count() as u32 - 3;
        for row in 0..after.rows() {
            for col in 0..after.cols() {
                let a = before.get(col, row).unwrap();
                let b = after.get(col, row).unwrap();
                assert_eq!(
                    (a.widget_id, a.style, a.shared_border),
                    (b.widget_id, b.style, b.shared_border)
                );
                if row != ROWS - 2 || !(field..field + 7).contains(&col) {
                    assert_eq!(a, b);
                }
            }
        }
    }
}

#[test]
fn m224_progress_matches_all_21_reference_states_at_a_fixed_width() {
    let refs = include_str!("../../../../docs/ui_design/Interface_Elements.txt");
    for percent in (0..=100).step_by(5) {
        let segment = data::progress_segment(percent);
        assert_eq!(segment.chars().count(), 21);
        assert!(
            refs.lines().any(|line| line == segment.trim_end()),
            "{segment:?}"
        );
    }
}

#[test]
fn scene_viewport_uses_the_full_interior_and_stays_stable_under_the_action_box() {
    let shell = ShellState::new(82, 30);
    let scene = shell.scene_viewport();
    let layout = shell.layout();
    let viewport = layout.viewport.inset(1);

    assert_eq!(scene.col(), viewport.col());
    assert_eq!(scene.row(), viewport.row());
    assert_eq!(scene.right(), viewport.right());
    assert_eq!(scene.bottom(), viewport.bottom());
}

#[test]
fn color_checkbox_dedicated_action_keeps_popup_focus_and_visibility_ids() {
    let mut s = shell();
    let sibling = shell();
    let original = s.view.checks;
    assert!(!s.color_enabled());
    assert!(click_text(&mut s, "View"));
    assert!(click_text(&mut s, "Color"));
    for (key, enabled) in [
        (None, true),
        (Some(ShellKey::Enter), false),
        (Some(ShellKey::Char(' ')), true),
    ] {
        if let Some(key) = key {
            assert!(s.key(key));
        }
        assert_eq!(s.color_enabled(), enabled);
        assert_eq!(s.view.checks, original);
        assert_eq!(s.menu_bar.open_menu(), Some(2));
        assert_eq!(s.interaction().focused, BAR_IDS.item(2, 4));
        assert!(has(
            &s.compose().unwrap(),
            if enabled { "■ Color" } else { "□ Color" }
        ));
    }
    assert!(!sibling.color_enabled());
    assert_eq!(sibling.theme(), ThemeChoice::Dark);
    for i in 0..5 {
        let before = s.view.checks;
        s.apply_menu(MenuResult::Toggled(MenuAction::new(210 + i)));
        for (j, previous) in before.iter().enumerate() {
            assert_eq!(s.view.checks[j], *previous ^ (j == i as usize));
        }
        assert!(s.color_enabled());
    }
}

#[test]
fn color_theme_spans_and_popup_title_have_exact_semantic_ownership() {
    use crate::InkRole::{Action, Contour, Text};
    for enabled in [false, true] {
        let mut s = shell();
        s.view.color_enabled = enabled;
        s.key(ShellKey::Alt('v'));
        for (choice, label) in [
            (ThemeChoice::Dark, "Theme [ ◑ Dark  ]"),
            (ThemeChoice::Light, "Theme [ ◐ Light ]"),
        ] {
            s.view.theme = choice;
            let grid = s.compose().unwrap();
            let (x, y) = find(&grid, label).unwrap();
            for (offset, ch) in label.chars().enumerate() {
                let cell = grid.get(x + offset as u32, y).unwrap();
                assert_eq!(
                    cell.ink,
                    if [6, 8, 16].contains(&offset) {
                        Action
                    } else {
                        Text
                    }
                );
                assert_eq!(
                    cell.content,
                    if ch == ' ' {
                        CellContent::Empty
                    } else {
                        CellContent::Glyph(ch)
                    }
                );
                assert_eq!(cell.widget_id, BAR_IDS.item(2, 3));
            }
            assert_eq!(grid.get(x - 2, y).unwrap().ink, Contour);
            assert_eq!(grid.get(x + 17, y).unwrap().ink, Text); // row padding
            let (tx, ty) = find(&grid, "╭─View").unwrap();
            for col in [tx, tx + 1, tx + 6, tx + 20] {
                assert_eq!(grid.get(col, ty).unwrap().ink, Contour);
            }
            for col in tx + 2..tx + 6 {
                assert_eq!(grid.get(col, ty).unwrap().ink, Text);
            }
            let (hovered, lit) = s.compose_hovered(BAR_IDS.item(2, 3)).unwrap();
            assert!(lit);
            for (plain, over) in grid.cells().iter().zip(hovered.cells()) {
                assert_eq!(
                    (plain.ink, plain.style, plain.widget_id, plain.shared_border),
                    (over.ink, over.style, over.widget_id, over.shared_border)
                );
            }
            click_text(&mut s, label);
            assert_ne!(s.theme(), choice);
            assert_eq!(s.color_enabled(), enabled);
            assert_eq!(s.interaction().focused, BAR_IDS.item(2, 3));
        }
    }
}

#[test]
fn color_mixed_action_box_values_and_guides_keep_roles_and_geometry() {
    use crate::InkRole::{Action, Contour, Text};
    let mut s = shell();
    let off = s.compose().unwrap();
    s.view.color_enabled = true;
    let on = s.compose().unwrap();
    for (a, b) in off.cells().iter().zip(on.cells()) {
        assert_eq!(
            (a.ink, a.style, a.widget_id, a.shared_border),
            (b.ink, b.style, b.widget_id, b.shared_border)
        );
        if a.ink != Contour {
            assert_eq!(a.content, b.content);
        }
    }
    let (x, y) = find(&on, "Depth").unwrap();
    assert_eq!(on.get(x - 4, y).unwrap().ink, Contour);
    assert_eq!(on.get(x - 2, y).unwrap().ink, Text);
    let (sign, row) = find(&on, "+|").unwrap();
    assert_eq!(on.get(sign, row).unwrap().ink, Action);
    assert_eq!(on.get(sign + 1, row).unwrap().ink, Contour);
    assert_eq!(on.get(sign + 2, row).unwrap().ink, Text);
    let (p, r) = find(&on, "[Select…").unwrap();
    assert_eq!(on.get(p, r).unwrap().ink, Action);
    assert_eq!(on.get(p + 1, r).unwrap().ink, Text);
    let (tx, ty) = find(&on, "╭─Extrude").unwrap();
    assert_eq!(on.get(tx, ty).unwrap().ink, Contour);
    assert_eq!(on.get(tx + 2, ty).unwrap().ink, Text);
    s.view.color_enabled = false;
    assert_eq!(s.compose().unwrap(), off);
}

#[test]
fn color_modal_title_hover_and_underlying_ink_survive_routing_mask() {
    use crate::InkRole::{Action, Contour, Text};
    for enabled in [false, true] {
        for rename in [false, true] {
            let mut s = ShellState::new(82, 30);
            s.view.color_enabled = enabled;
            let before = s.compose().unwrap();
            assert_eq!(before.get(1, 5).unwrap().ink, Action);
            let title = if rename {
                let id = id_of(&s, "Hub_Body");
                assert!(s.begin_rename(id));
                "Rename"
            } else {
                click_text(&mut s, "☰");
                "Formula"
            };
            let plain = s.compose().unwrap();
            assert_eq!(plain.get(1, 5).unwrap().ink, Action);
            assert_eq!(
                plain.get(1, 5).unwrap().widget_id,
                None,
                "modal masks routing, not ink"
            );
            let (x, y) = find(&plain, &format!("╭─{title}")).unwrap();
            assert_eq!(plain.get(x, y).unwrap().ink, Contour);
            assert_eq!(plain.get(x + 1, y).unwrap().ink, Contour);
            for dx in 0..title.len() {
                assert_eq!(plain.get(x + 2 + dx as u32, y).unwrap().ink, Text);
            }
            assert_eq!(
                plain.get(x + 2 + title.len() as u32, y).unwrap().ink,
                Contour
            );
            let id = id_of(&s, "OK");
            let (hovered, lit) = s.compose_hovered(Some(id)).unwrap();
            assert!(lit);
            for (a, b) in plain.cells().iter().zip(hovered.cells()) {
                assert_eq!(
                    (a.ink, a.style, a.widget_id, a.shared_border),
                    (b.ink, b.style, b.widget_id, b.shared_border)
                );
                if a.content != b.content {
                    assert_eq!(b.ink, Contour);
                }
            }
        }
    }
}

#[test]
fn selection_scrollbars_keep_semantic_ink_and_glyphs_in_every_theme_and_color_mode() {
    use crate::InkRole::{Action, Text};
    for profiles in [false, true] {
        for theme in [ThemeChoice::Light, ThemeChoice::Dark] {
            for color in [false, true] {
                let mut s = shell();
                s.view.theme = theme;
                s.view.color_enabled = color;
                if profiles {
                    open_profile_selection(&mut s);
                } else {
                    open_selection(&mut s);
                }
                let grid = s.compose().unwrap();
                let (x, y) = find(&grid, "▲").unwrap();
                assert_eq!(glyph(&grid, x, y + 1), '┃');
                assert_eq!(glyph(&grid, x, y + 2), ' ');
                assert_eq!(glyph(&grid, x, y + 3), ' ');
                assert_eq!(glyph(&grid, x, y + 4), '▼');
                for dy in [0, 1, 4] {
                    assert_eq!(grid.get(x, y + dy).unwrap().ink, Action);
                }
                for dy in [2, 3] {
                    assert_eq!(grid.get(x, y + dy).unwrap().ink, Text);
                    assert!(grid.get(x, y + dy).unwrap().widget_id.is_none());
                }
                let thumb = grid.get(x, y + 1).unwrap().widget_id.unwrap();
                let (hovered, lit) = s.compose_hovered(Some(thumb)).unwrap();
                assert!(!lit);
                assert_eq!(hovered, grid);
                let draft_focus = s.selection.focus();
                assert!(s.scroll(i32::MIN));
                assert_eq!(s.selection.focus(), draft_focus);
                let bottom = s.compose().unwrap();
                assert_eq!(glyph(&bottom, x, y + 3), '┃');
                assert!(s.scroll(i32::MAX));
                assert_eq!(s.compose().unwrap(), grid);
            }
        }
    }
}
