// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M2.20: the NavigationPanel trees compared with `Interface_Exemple.txt`, the Features /
//! Elements divider, the playhead and the Rename flow.
use super::data::{self, Line, Node, NodeKind, child_count, full_label, visible_lines};
use super::*;
use crate::{CellContent, CellGrid, GridMetrics, UiPoint, hit_test};

/// The canonical reference, read from `docs/ui_design/` (never edited here).
const REFERENCE: &str = include_str!("../../../../docs/ui_design/Interface_Exemple.txt");

fn metrics() -> GridMetrics {
    GridMetrics::new(6, 10).unwrap()
}

fn text(g: &CellGrid, row: u32) -> String {
    (0..g.cols())
        .map(|col| match g.get(col, row).unwrap().content {
            CellContent::Glyph(ch) => ch,
            CellContent::Empty => ' ',
        })
        .collect()
}

fn find(g: &CellGrid, needle: &str) -> Option<(u32, u32)> {
    let pat: Vec<char> = needle.chars().collect();
    (0..g.rows()).find_map(|row| {
        let line: Vec<char> = text(g, row).chars().collect();
        line.windows(pat.len())
            .position(|w| w == pat.as_slice())
            .map(|col| (u32::try_from(col).unwrap(), row))
    })
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

fn click_text(s: &mut ShellState, needle: &str) -> bool {
    let g = s.compose().unwrap();
    let (col, row) = find(&g, needle).expect("text must be on screen");
    s.click(hit_at(&g, col, row))
}

/// The id under the first cell of `needle`.
fn id_of(s: &ShellState, needle: &str) -> WidgetId {
    let g = s.compose().unwrap();
    let (col, row) = find(&g, needle).unwrap();
    hit_at(&g, col, row).and_then(|h| h.widget_id).unwrap()
}

fn type_str(s: &mut ShellState, t: &str) {
    for c in t.chars() {
        s.key(ShellKey::Char(c));
    }
}

/// Columns `0..25` of `row`: the NavigationPanel with its right border.
fn panel(g: &CellGrid, row: u32) -> String {
    text(g, row).chars().take(25).collect()
}

fn reference_panel() -> Vec<String> {
    REFERENCE
        .lines()
        .skip(4)
        .take(22)
        .map(|l| l.chars().take(25).collect())
        .collect()
}

fn ref_shell() -> ShellState {
    ShellState::new(82, 28)
}

// ---- reference comparison ----

#[test]
fn navigation_panel_equals_the_reference_cell_for_cell_at_82x28() {
    let g = ref_shell().compose().unwrap();
    let expected = reference_panel();
    assert_eq!(expected.len(), 22);
    for (i, want) in expected.iter().enumerate() {
        let row = 4 + u32::try_from(i).unwrap();
        assert_eq!(&panel(&g, row), want, "row {row}");
    }
}

#[test]
fn extra_rows_belong_to_elements_and_the_features_pane_keeps_its_height() {
    let g28 = ref_shell().compose().unwrap();
    let g30 = ShellState::new(82, 30).compose().unwrap();
    // `README_UI.md` §25: with snapshot rows 26 and 27 (0-based 25, 26) removed the rest is
    // the reference; the two rows are blank body rows.
    let kept: Vec<u32> = (0..30).filter(|r| *r != 25 && *r != 26).collect();
    for (i, row) in kept.iter().enumerate() {
        let want = panel(&g28, u32::try_from(i).unwrap());
        assert_eq!(panel(&g30, *row), want, "row {row}");
    }
    for row in [25, 26] {
        assert_eq!(panel(&g30, row), format!("║{}║", " ".repeat(23)));
    }
    // And a taller window moves the extra rows to Elements only.
    let tall = ShellState::new(82, 50).compose().unwrap();
    for row in 4..17 {
        assert_eq!(panel(&tall, row), panel(&g28, row), "row {row}");
    }
}

#[test]
fn every_state_keeps_the_reference_vocabulary_of_glyphs() {
    // Collapse and expand: markers and connectors stay in the documented set.
    let mut s = ref_shell();
    click_text(&mut s, "[-] Hub_Features");
    click_text(&mut s, "[+]Construct");
    let g = s.compose().unwrap();
    let rows: Vec<String> = (4..26).map(|r| panel(&g, r)).collect();
    let line = |s: &str| format!("║{s:<23}║");
    assert_eq!(rows[3], line(" ├[+] Hub_Features"));
    assert_eq!(rows[4], line(" ├ Fillet_Inner"));
    assert_eq!(rows[5], line(" │ ─────────────────── "));
    assert_eq!(rows[17], line(" ├ Surfaces(0)"));
    assert_eq!(rows[18], line(" ╰[-]Construct"));
    assert_eq!(rows[19], line("   ╰╴□ Plane_XY"));
}

// ---- truncation and the minimum width ----

#[test]
fn long_names_are_truncated_with_an_ellipsis_in_the_last_cell() {
    let mut s = ref_shell();
    let g = s.compose().unwrap();
    assert!(text(&g, 6).starts_with("║ ├ □ Sketch_Profile_Ma…║"));
    // A wider panel shows the whole name; a narrower one cuts earlier, always with `…`.
    s.set_nav_width(30);
    assert!(text(&s.compose().unwrap(), 6).starts_with("║ ├ □ Sketch_Profile_Main"));
    s.set_nav_width(14);
    let g = s.compose().unwrap();
    assert!(text(&g, 6).starts_with("║ ├ □ Sketch…║"), "{}", text(&g, 6));
    assert!(
        text(&g, 10).starts_with("║ │ ╰╴Extrud…║"),
        "{}",
        text(&g, 10)
    );
}

#[test]
fn the_navigation_panel_is_never_narrower_than_14_cells() {
    let mut s = ref_shell();
    assert!(s.set_nav_width(1));
    assert_eq!(s.nav_width(), 14);
    s.drag_separator(Separator::Navigation, 0, 0);
    assert_eq!(s.nav_width(), 14);
    s.drag_separator(Separator::Navigation, 5, 0);
    assert_eq!(s.nav_width(), 14);
    // The wish is clamped, not forgotten: a wide grid honours a later wish.
    s.drag_separator(Separator::Navigation, 19, 0);
    assert_eq!(s.nav_width(), 20);
}

// ---- dynamic counts ----

#[test]
fn counts_are_derived_from_the_tree_not_stored() {
    let nodes = [
        Node {
            label: "Group",
            depth: 1,
            kind: NodeKind::Category { counted: true },
            check: None,
            open: true,
        },
        Node {
            label: "a",
            depth: 2,
            kind: NodeKind::Named,
            check: None,
            open: false,
        },
        Node {
            label: "b",
            depth: 2,
            kind: NodeKind::Named,
            check: None,
            open: false,
        },
        Node {
            label: "b_child",
            depth: 3,
            kind: NodeKind::Named,
            check: None,
            open: false,
        },
        Node {
            label: "c",
            depth: 2,
            kind: NodeKind::Named,
            check: None,
            open: false,
        },
    ];
    assert_eq!(child_count(&nodes, 0), 3, "direct children only");
    assert_eq!(full_label(&nodes, 0, None), "Group(3)");
    assert_eq!(full_label(&nodes, 1, None), "a");
    assert_eq!(full_label(&nodes, 2, Some("renamed")), "renamed");
    assert_eq!(full_label(&data::ELEMENTS, 1, None), "Solid Bodies(2)");
    assert_eq!(full_label(&data::ELEMENTS, 4, None), "Surfaces(0)");
    assert_eq!(full_label(&data::ELEMENTS, 5, None), "Construct");
}

// ---- divider ----

#[test]
fn divider_is_the_shared_border_row_between_the_panes() {
    let s = ref_shell();
    assert_eq!(s.separator_at(5, 16), Some(Separator::Split));
    assert_eq!(s.separator_at(1, 16), Some(Separator::Split));
    assert_eq!(s.separator_at(23, 16), Some(Separator::Split));
    assert_eq!(
        s.separator_at(0, 16),
        None,
        "the frame ring is a resize zone"
    );
    assert_eq!(s.separator_at(24, 16), Some(Separator::Navigation));
    assert_eq!(s.separator_at(5, 15), None);
    assert_eq!(s.separator_at(5, 17), None);
    assert_eq!(s.separator_at(30, 16), None);
    let g = s.compose().unwrap();
    let hit = hit_at(&g, 5, 16).unwrap();
    assert_eq!(s.drag_at(&hit), Some(Separator::Split));
}

#[test]
fn dragging_the_divider_moves_it_by_whole_rows_and_clamps() {
    let mut s = ref_shell();
    assert!(s.drag_separator(Separator::Split, 40, 12));
    let g = s.compose().unwrap();
    assert_eq!(panel(&g, 12), format!("╠{}╣", "═".repeat(23)));
    assert!(text(&g, 5).starts_with("║[-]Features"));
    assert!(
        !s.drag_separator(Separator::Split, 40, 12),
        "same row: no change"
    );
    assert!(text(&g, 13).starts_with("║[-]Elements"), "{}", text(&g, 13));

    // Three rows is the least a pane keeps; the body is 22 rows, so Features tops out at 20.
    s.drag_separator(Separator::Split, 0, 0);
    assert_eq!(
        panel(&s.compose().unwrap(), 6),
        format!("╠{}╣", "═".repeat(23))
    );
    s.drag_separator(Separator::Split, 0, u32::MAX);
    let g = s.compose().unwrap();
    assert_eq!(panel(&g, 23), format!("╠{}╣", "═".repeat(23)));
    assert!(text(&g, 24).starts_with("║[-]Elements"));
}

#[test]
fn the_divider_keeps_its_proportion_when_the_window_is_resized() {
    let mut s = ref_shell();
    s.drag_separator(Separator::Split, 0, 12);
    let heights = |s: &ShellState| {
        let l = s.layout();
        (l.features.height(), l.elements.height())
    };
    assert_eq!(heights(&s), (9, 14));
    s.resize(82, 38); // body 22 -> 32: 9 * 32 / 22 = 13.1
    assert_eq!(heights(&s), (13, 20));
    s.resize(82, 28);
    assert_eq!(heights(&s), (9, 14), "the user's split comes back");
    // Smaller than both minimum panes: clamped, never inverted.
    s.resize(82, 12);
    let (f, e) = heights(&s);
    assert!(f + e >= 4 && f <= s.layout().viewport.height());
    s.resize(82, 28);
    assert_eq!(heights(&s), (9, 14));
}

#[test]
fn the_default_split_is_the_reference_one_at_every_height() {
    for rows in [25, 28, 30, 40, 60] {
        let l = ShellState::new(82, rows).layout();
        let body = l.viewport.height();
        assert_eq!(l.features.height(), (body * 3 / 5).min(13), "{rows} rows");
        assert_eq!(l.features.row() + l.features.height() - 1, l.elements.row());
    }
}

#[test]
fn divider_cannot_be_dragged_while_a_menu_or_modal_owns_input() {
    let mut s = ref_shell();
    assert!(s.key(ShellKey::MenuBar));
    assert_eq!(s.separator_at(5, 16), None);
    assert!(s.key(ShellKey::MenuBar));
    s.begin_rename(id_of(&s, "Hub_Body"));
    assert!(s.rename().is_open());
    assert_eq!(s.separator_at(5, 16), None);
}

// ---- playhead ----

fn rows_of(s: &ShellState, from: u32, to: u32) -> Vec<String> {
    let g = s.compose().unwrap();
    (from..=to).map(|r| panel(&g, r)).collect()
}

const PLAYHEAD_ROW: &str = "║ │ ─────────────────── ║";

#[test]
fn the_playhead_is_a_line_between_two_features_and_not_a_node() {
    let s = ref_shell();
    let g = s.compose().unwrap();
    assert_eq!(panel(&g, 12), PLAYHEAD_ROW);
    let hit = hit_at(&g, 10, 12).unwrap();
    assert_eq!(hit.widget_id, Some(WidgetId::new(PLAYHEAD_ID)));
    assert_eq!(
        tree_decode(WidgetId::new(PLAYHEAD_ID)),
        None,
        "not a feature"
    );
    assert_eq!(s.drag_at(&hit), Some(Separator::Playhead));
    assert!(!ShellState::is_rename_target(WidgetId::new(PLAYHEAD_ID)));

    // Pressing it selects nothing and takes no focus.
    let mut s = ref_shell();
    assert!(!s.click(Some(hit)));
    assert_eq!((s.selected, s.interaction().focused), (None, None));
    // Tab never lands on it.
    let ring = s.focus_ring(&s.compose().unwrap());
    assert!(!ring.contains(&WidgetId::new(PLAYHEAD_ID)));
    for _ in 0..60 {
        s.key(ShellKey::Tab);
        assert_ne!(s.interaction().focused, Some(WidgetId::new(PLAYHEAD_ID)));
    }
}

#[test]
fn dragging_the_playhead_moves_it_between_top_level_features() {
    let mut s = ref_shell();
    let before = rows_of(&s, 5, 15);

    // Onto Hub_Features (row 7): after its whole subtree, before Fillet_Inner.
    assert!(s.drag_separator(Separator::Playhead, 10, 7));
    let after = rows_of(&s, 5, 15);
    assert_eq!(after[5], "║ │ ╰╴Extrude_Cut_Center║");
    assert_eq!(after[6], PLAYHEAD_ROW);
    assert_eq!(after[7], "║ ├ Fillet_Inner        ║");
    // Nesting and every other row are untouched: only the separator line moved.
    let strip = |rows: &[String]| {
        rows.iter()
            .filter(|r| *r != PLAYHEAD_ROW)
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(strip(&before), strip(&after));
    assert!(
        !s.drag_separator(Separator::Playhead, 10, 7),
        "same gap: no change"
    );

    // Above the tree: after the first feature. Below it: before the last one.
    assert!(s.drag_separator(Separator::Playhead, 10, 0));
    let g = s.compose().unwrap();
    assert_eq!(panel(&g, 6), "║ ├ □ Sketch_Profile_Ma…║");
    assert_eq!(panel(&g, 7), PLAYHEAD_ROW);
    assert_eq!(panel(&g, 8), "║ ├[-] Hub_Features     ║");
    assert!(s.drag_separator(Separator::Playhead, 10, 40));
    let g = s.compose().unwrap();
    assert_eq!(panel(&g, 12), "║ ├ Chamfer_Edges       ║");
    assert_eq!(panel(&g, 13), PLAYHEAD_ROW);
    assert_eq!(panel(&g, 14), "║ ╰╴Pattern_Circular    ║");

    // A row inside an expanded group still means that group.
    assert!(s.drag_separator(Separator::Playhead, 10, 9));
    assert_eq!(panel(&s.compose().unwrap(), 11), PLAYHEAD_ROW);
    // Dragging over the separator itself changes nothing.
    assert!(!s.drag_separator(Separator::Playhead, 10, 11));
}

#[test]
fn the_playhead_survives_collapsing_and_expanding_groups() {
    let mut s = ref_shell();
    click_text(&mut s, "[-] Hub_Features");
    let g = s.compose().unwrap();
    // Collapsed: Features, Sketch, Hub, Fillet, playhead, Chamfer, Pattern.
    assert_eq!(panel(&g, 9), PLAYHEAD_ROW);
    assert_eq!(panel(&g, 10), "║ ├ Chamfer_Edges       ║");
    click_text(&mut s, "[+] Hub_Features");
    assert_eq!(panel(&s.compose().unwrap(), 12), PLAYHEAD_ROW);

    // The valid range is between two top-level features whatever the wish was.
    let nodes = &data::FEATURES;
    assert_eq!(data::top_level_count(nodes), 5);
    assert_eq!(data::clamp_playhead(0, 5), Some(1));
    assert_eq!(data::clamp_playhead(99, 5), Some(4));
    assert_eq!(data::clamp_playhead(3, 1), None);
    let open = data::initial_open(nodes);
    let lines = visible_lines(nodes, &open, Some(3));
    let at = lines.iter().position(|l| *l == Line::Playhead).unwrap();
    assert!(matches!(&lines[at + 1], Line::Node(r) if nodes[r.index].label == "Chamfer_Edges"));
    assert_eq!(visible_lines(nodes, &open, None).len(), lines.len() - 1);
}

// ---- selection, expansion, checkboxes ----

#[test]
fn marker_checkbox_and_name_are_separate_targets() {
    let mut s = ref_shell();
    let marker = id_of(&s, "[-] Hub_Features");
    let name = id_of(&s, "Hub_Features");
    assert_ne!(marker, name);
    assert_eq!(tree_decode(marker).map(|d| d.2), Some(TreePart::Marker));
    assert_eq!(tree_decode(name).map(|d| d.2), Some(TreePart::Name));

    // The name selects and does not expand.
    assert!(click_text(&mut s, "Hub_Features"));
    assert_eq!(s.selected, Some((Tree::Features, 2)));
    assert!(s.features_open[2]);
    // The marker expands and leaves the selection alone.
    s.selected = None;
    assert!(click_text(&mut s, "[-] Hub_Features"));
    assert!(!s.features_open[2] && s.selected.is_none());
    click_text(&mut s, "[+] Hub_Features");

    // The checkbox flips the visibility glyph and nothing else.
    assert!(click_text(&mut s, "□ Sketch_Profile"));
    let g = s.compose().unwrap();
    assert!(panel(&g, 6).starts_with("║ ├ ■ Sketch_Profile"));
    assert!(s.selected.is_none());
    click_text(&mut s, "■ Sketch_Profile");
    assert!(panel(&s.compose().unwrap(), 6).starts_with("║ ├ □ Sketch_Profile"));
    // A row without a checkbox has none to flip.
    let before = s.checks;
    click_text(&mut s, "Fillet_Inner");
    assert_eq!(before, s.checks);
}

#[test]
fn selection_and_focus_draw_no_marker_in_the_trees() {
    let mut s = ref_shell();
    let glyphs = |s: &ShellState| {
        let g = s.compose().unwrap();
        (0..g.rows()).map(|r| text(&g, r)).collect::<Vec<_>>()
    };
    let base = glyphs(&s);
    click_text(&mut s, "Rim_Body");
    s.key(ShellKey::Tab);
    assert_eq!(glyphs(&s), base);
}

#[test]
fn enter_and_space_expand_the_focused_branch_row() {
    let mut s = ref_shell();
    click_text(&mut s, "Hub_Features");
    assert_eq!(s.interaction().focused, Some(tree_id(Tree::Features, 2)));
    s.key(ShellKey::Enter);
    assert!(!s.features_open[2]);
    s.key(ShellKey::Char(' '));
    assert!(s.features_open[2]);
    // A leaf row has nothing to expand.
    click_text(&mut s, "Extrude_Base");
    let open = s.features_open;
    s.key(ShellKey::Enter);
    assert_eq!(open, s.features_open);
}

// ---- Rename ----

#[test]
fn only_user_named_rows_are_rename_targets() {
    let s = ref_shell();
    for (needle, yes) in [
        ("Sketch_Profile_Ma", true),
        ("Hub_Features", true),
        ("Extrude_Base", true),
        ("Rim_Body", true),
        ("Hub_Body", true),
        ("Features", false),
        ("Elements", false),
        ("Solid Bodies", false),
        ("Surfaces(0)", false),
        ("Construct", false),
        ("[-] Hub_Features", false),
        ("□ Sketch_Profile", false),
        ("■ Rim_Body", false),
    ] {
        assert_eq!(
            ShellState::is_rename_target(id_of(&s, needle)),
            yes,
            "{needle}"
        );
    }
    let g = s.compose().unwrap();
    // The blank cells after a name and the guides are the row, not its name.
    let trailing = hit_at(&g, 20, 11).and_then(|h| h.widget_id).unwrap();
    assert!(!ShellState::is_rename_target(trailing));
}

#[test]
fn rename_opens_centred_on_the_full_name() {
    let mut s = ref_shell();
    assert!(
        !s.begin_rename(id_of(&s, "Solid Bodies")),
        "a header is no target"
    );
    assert!(!s.rename().is_open());

    assert!(s.begin_rename(id_of(&s, "Sketch_Profile_Ma")));
    assert!(s.rename().is_open() && s.modal_open());
    assert_eq!(
        s.rename().draft(),
        "Sketch_Profile_Main",
        "not the `…` text"
    );
    let g = s.compose().unwrap();
    let want = [
        "╭─Rename──────────────────╮",
        "│[Sketch_Profile_Main▏   ]│",
        "│─────────────────────────│",
        "│     OK     │   Cancel   │",
        "╰─────────────────────────╯",
    ];
    // 82 x 28: (82 - 27) / 2 = 27, (28 - 5) / 2 = 11.
    for (i, line) in want.iter().enumerate() {
        let row = 11 + u32::try_from(i).unwrap();
        let got: String = text(&g, row).chars().skip(27).take(27).collect();
        assert_eq!(&got, line, "row {row}");
    }
    assert_eq!(s.interaction().focused, s.rename().focus());
}

#[test]
fn rename_ok_changes_the_name_and_keeps_ids_and_focus() {
    let mut s = ref_shell();
    let name_id = id_of(&s, "Hub_Body");
    let row_id = tree_id(Tree::Elements, 3);
    click_text(&mut s, "Hub_Body");
    s.begin_rename(name_id);
    type_str(&mut s, "_2");
    s.key(ShellKey::Backspace);
    type_str(&mut s, "3");
    assert!(s.key(ShellKey::Enter));
    assert!(!s.rename().is_open() && !s.modal_open());
    let g = s.compose().unwrap();
    assert!(
        panel(&g, 20).starts_with("║ │ ╰╴□ Hub_Body_3"),
        "{}",
        panel(&g, 20)
    );
    assert_eq!(id_of(&s, "Hub_Body_3"), name_id, "the id is stable");
    assert_eq!(s.interaction().focused, Some(row_id), "focus is restored");
    assert_eq!(s.selected, Some((Tree::Elements, 3)));

    // Renaming again starts from the new full name.
    s.begin_rename(name_id);
    assert_eq!(s.rename().draft(), "Hub_Body_3");
}

#[test]
fn rename_cancel_and_escape_keep_the_old_name_and_restore_focus() {
    let mut s = ref_shell();
    let name_id = id_of(&s, "Rim_Body");
    let row_id = tree_id(Tree::Elements, 2);
    click_text(&mut s, "Rim_Body");
    s.begin_rename(name_id);
    type_str(&mut s, "xx");
    s.key(ShellKey::Escape);
    assert!(!s.rename().is_open());
    assert!(has_text(&s, "Rim_Body") && !has_text(&s, "Rim_Bodyxx"));
    assert_eq!(s.interaction().focused, Some(row_id));

    s.begin_rename(name_id);
    type_str(&mut s, "yy");
    assert!(click_text(&mut s, "Cancel"));
    assert!(!s.rename().is_open());
    assert!(has_text(&s, "Rim_Body") && !has_text(&s, "Rim_Bodyyy"));
    assert_eq!(s.interaction().focused, Some(row_id));

    // The OK button works with the pointer too.
    s.begin_rename(name_id);
    type_str(&mut s, "!");
    assert!(click_text(&mut s, "OK"));
    assert!(has_text(&s, "Rim_Body!"));
}

fn has_text(s: &ShellState, needle: &str) -> bool {
    find(&s.compose().unwrap(), needle).is_some()
}

#[test]
fn a_long_new_name_is_truncated_in_the_tree_but_edited_in_full() {
    let mut s = ref_shell();
    let id = id_of(&s, "Hub_Body");
    s.begin_rename(id);
    for _ in 0.."Hub_Body".len() {
        s.key(ShellKey::Backspace);
    }
    type_str(&mut s, "A_very_long_element_name_here");
    s.key(ShellKey::Enter);
    let g = s.compose().unwrap();
    assert_eq!(panel(&g, 20), "║ │ ╰╴□ A_very_long_ele…║");
    s.begin_rename(id);
    assert_eq!(s.rename().draft(), "A_very_long_element_name_here");
}

#[test]
fn rename_is_modal_and_never_empties_a_name() {
    let mut s = ref_shell();
    let id = id_of(&s, "Hub_Body");
    s.begin_rename(id);
    let before = (s.features_open, s.elements_open, s.selected);
    // The base shell is blocked, for pointer and keyboard.
    let g = s.compose().unwrap();
    let (col, row) = find(&g, "Hub_Features").unwrap();
    assert_eq!(hit_at(&g, col, row).unwrap().widget_id, None);
    assert!(!s.click(hit_at(&g, col, row)));
    assert!(!s.key(ShellKey::MenuBar) && !s.key(ShellKey::Alt('f')));
    assert_eq!(before, (s.features_open, s.elements_open, s.selected));
    assert!(!s.begin_rename(id_of_modal_free()), "no second modal");

    for _ in 0.."Hub_Body".len() {
        s.key(ShellKey::Backspace);
    }
    s.key(ShellKey::Enter);
    assert!(s.rename().is_open(), "an empty name is refused");
    type_str(&mut s, "ok");
    s.key(ShellKey::Enter);
    assert!(has_text(&s, "ok"));
}

fn id_of_modal_free() -> WidgetId {
    tree_part_id(Tree::Features, 1, TreePart::Name)
}

#[test]
fn renaming_is_per_row_and_does_not_touch_other_rows_or_trees() {
    let mut s = ref_shell();
    s.begin_rename(id_of(&s, "Extrude_Base"));
    type_str(&mut s, "_v2");
    s.key(ShellKey::Enter);
    let g = s.compose().unwrap();
    assert!(find(&g, "Extrude_Base_v2").is_some());
    assert!(find(&g, "Sketch_Hole").is_some() && find(&g, "Rim_Body").is_some());
    assert_eq!(s.names[0][3].as_deref(), Some("Extrude_Base_v2"));
    assert!(s.names[1].iter().all(Option::is_none));
    // The ellipsis row of the reference is unaffected.
    assert!(panel(&g, 6).ends_with("Sketch_Profile_Ma…║"));
}

#[test]
fn tiny_grids_survive_every_tree_interaction() {
    for cols in [0, 1, 7, 20, 30, 82] {
        for rows in [0, 1, 4, 9, 14, 28] {
            let mut s = ShellState::new(cols, rows);
            for separator in [Separator::Navigation, Separator::Split, Separator::Playhead] {
                s.drag_separator(separator, 3, 5);
                s.drag_separator(separator, u32::MAX, u32::MAX);
                s.drag_separator(separator, 0, 0);
            }
            s.begin_rename(tree_part_id(Tree::Features, 1, TreePart::Name));
            s.key(ShellKey::Char('x'));
            s.key(ShellKey::Enter);
            let g = s.compose().unwrap();
            assert_eq!((g.cols(), g.rows()), (cols, rows));
        }
    }
}
