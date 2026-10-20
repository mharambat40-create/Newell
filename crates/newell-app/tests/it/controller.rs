// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

#![allow(clippy::unwrap_used)]

use newell_app::controller::{ShellController, Update};
use newell_ui::{CellContent, CellGrid, DEFAULT_K, GridMetrics, ShellKey, UiPoint};

// The 4:9 cell of `k = 5`: 20 x 45 physical pixels.
const W: u32 = 4 * DEFAULT_K;
const H: u32 = 9 * DEFAULT_K;

fn controller() -> ShellController {
    ShellController::new(82, 25, GridMetrics::from_k(DEFAULT_K).unwrap()).unwrap()
}

/// Centre of cell `(col, row)` in pixels.
fn at(col: u32, row: u32) -> Option<UiPoint> {
    Some(UiPoint {
        x: col * W + W / 2,
        y: row * H + H / 2,
    })
}

fn glyph(g: &CellGrid, col: u32, row: u32) -> Option<char> {
    match g.get(col, row)?.content {
        CellContent::Glyph(c) => Some(c),
        CellContent::Empty => None,
    }
}

fn find_glyph(g: &CellGrid, ch: char) -> (u32, u32) {
    (0..g.rows())
        .find_map(|r| {
            (0..g.cols())
                .find(|&c| glyph(g, c, r) == Some(ch))
                .map(|c| (c, r))
        })
        .unwrap()
}

const NOTHING: Update = Update { grid: false };

#[test]
fn default_cell_is_exactly_twenty_by_forty_five() {
    let m = GridMetrics::from_k(DEFAULT_K).unwrap();
    assert_eq!((m.cell_width_px(), m.cell_height_px()), (20, 45));
    assert_eq!((W, H), (20, 45));
    assert_eq!(controller().metrics(), m);
}

#[test]
fn starts_with_the_composed_reference_shell() {
    let c = controller();
    assert_eq!((c.grid().cols(), c.grid().rows()), (82, 25));
    assert_eq!(glyph(c.grid(), 0, 0), Some('╔'));
    assert_eq!(c.grid(), &c.shell().compose().unwrap());
}

/// Moving the pointer over widgets that are not hover targets (M2.25), without a press, changes
/// nothing. Hover over the targets is covered in `hover.rs`.
#[test]
fn pointer_motion_without_a_press_changes_nothing() {
    let mut c = controller();
    let before = c.grid().clone();
    let shell_before = c.shell().interaction().to_owned();
    for p in [at(3, 1), at(4, 1), at(3, 3), at(30, 12), at(3, 1), None] {
        assert_eq!(c.pointer_moved(p), NOTHING);
        assert_eq!(c.grid(), &before);
        assert_eq!(c.shell().interaction(), &shell_before);
    }
    assert!(!c.dragging());
}

#[test]
fn press_opens_the_file_menu_and_escape_closes_it() {
    let mut c = controller();
    c.pointer_moved(at(3, 1));
    let u = c.pointer_pressed();
    assert!(u.grid);
    assert_eq!(glyph(c.grid(), 1, 2), Some('╭'));
    assert!(c.key(ShellKey::Escape).grid);
    assert_eq!(glyph(c.grid(), 1, 2), Some('═'));
}

#[test]
fn a_press_acts_on_the_cell_under_the_pointer_after_the_grid_changed() {
    let mut c = controller();
    c.pointer_moved(at(3, 3)); // the Workbench title
    assert!(c.key(ShellKey::Alt('f')).grid); // File popup now covers that cell
    // Interaction is resolved at press time, from the current grid.
    assert!(c.pointer_pressed().grid, "pressed the File popup's row");
}

#[test]
fn press_without_a_pointer_position_does_nothing() {
    let mut c = controller();
    assert_eq!(c.pointer_pressed(), NOTHING);
    c.pointer_moved(at(30, 12));
    assert_eq!(
        c.pointer_pressed(),
        NOTHING,
        "an empty cell is not a control"
    );
}

#[test]
fn formula_modal_opens_from_the_button_and_blocks_the_background() {
    let mut c = controller();
    let (col, row) = find_glyph(c.grid(), '☰');
    c.pointer_moved(at(col, row));
    assert!(c.pointer_pressed().grid);
    assert!(c.shell().formula().is_open());
    // The pointer now sits on the modal; the background owns nothing.
    c.pointer_moved(at(3, 1));
    assert_eq!(c.pointer_pressed(), NOTHING);
    assert!(c.shell().formula().is_open());
    // Typing reaches the editor; a control character does not.
    assert!(c.key(ShellKey::Char('z')).grid);
    assert_eq!(c.key(ShellKey::Char('\n')), NOTHING);
    assert!(c.shell().formula().draft().ends_with('z'));
    assert!(c.key(ShellKey::Escape).grid);
    assert!(!c.shell().formula().is_open());
}

#[test]
fn update_merge_combines_effects() {
    let a = Update { grid: true };
    assert_eq!(a.merge(NOTHING), a);
    assert_eq!(NOTHING.merge(a), a);
    assert_eq!(NOTHING.merge(NOTHING), NOTHING);
    assert!(!NOTHING.any() && a.any());
}

#[test]
fn degenerate_grids_are_safe() {
    let metrics = GridMetrics::from_k(DEFAULT_K).unwrap();
    for (cols, rows) in [(0, 0), (1, 1), (3, 2), (10, 6), (82, 3)] {
        let mut c = ShellController::new(cols, rows, metrics).unwrap();
        c.pointer_moved(at(0, 0));
        c.pointer_pressed();
        for key in [
            ShellKey::MenuBar,
            ShellKey::Down,
            ShellKey::Enter,
            ShellKey::Tab,
        ] {
            c.key(key);
        }
        c.pointer_moved(None);
    }
    assert!(ShellController::new(u32::MAX, u32::MAX, metrics).is_err());
}

/// Every glyph the shell can show must exist in the renderer's atlas set.
#[test]
fn every_shell_glyph_is_in_the_atlas_set() {
    let atlas = newell_render::glyph_atlas::canonical_glyphs();
    let check = |c: &ShellController| {
        for cell in c.grid().cells() {
            if let CellContent::Glyph(ch) = cell.content {
                assert!(atlas.contains(&ch), "{ch:?} is not in the glyph set");
            }
        }
    };
    let mut c = controller();
    check(&c);
    // Each menu (View carries the cycles and checkboxes), then both modals.
    for key in [
        ShellKey::Alt('f'),
        ShellKey::Alt('e'),
        ShellKey::Alt('v'),
        ShellKey::Alt('h'),
    ] {
        c.key(key);
        check(&c);
    }
    c.key(ShellKey::Escape);
    for _ in 0..8 {
        c.key(ShellKey::CommandBar);
        c.key(ShellKey::Down);
        check(&c);
        c.key(ShellKey::Right);
        c.key(ShellKey::Escape);
    }
    let (col, row) = find_glyph(c.grid(), '☰');
    c.pointer_moved(at(col, row));
    c.pointer_pressed();
    check(&c);
    c.key(ShellKey::Escape);
    let (col, row) = find_glyph(c.grid(), '▶');
    c.pointer_moved(at(col, row));
    c.pointer_pressed(); // termination -> `Up to`: the Selection field appears
    c.key(ShellKey::Tab);
    check(&c);
}

// ---- M2.14: responsive resizing and separator dragging ----

#[test]
fn resize_recomputes_whole_cells_from_pixels() {
    let mut c = controller();
    let update = c.resize(100 * W + W - 1, 30 * H + H - 1);
    assert!(update.grid);
    assert_eq!((c.grid().cols(), c.grid().rows()), (100, 30));
    assert_eq!(c.grid(), &c.shell().compose().unwrap());
    // A size that leaves the cell grid unchanged uploads nothing.
    assert_eq!(c.resize(100 * W + 1, 30 * H + 1), NOTHING);
}

#[test]
fn resize_to_zero_and_back_restores_the_grid() {
    let mut c = controller();
    c.resize(100 * W, 40 * H);
    let original = c.grid().clone();
    // M2.17: a smaller (or minimised) surface never composes below 82 x 30.
    assert!(c.resize(0, 0).grid);
    assert_eq!((c.grid().cols(), c.grid().rows()), (82, 30));
    c.resize(W - 1, 5 * H);
    assert_eq!((c.grid().cols(), c.grid().rows()), (82, 30));
    c.resize(100 * W, 40 * H);
    assert_eq!(c.grid(), &original);
}

#[test]
fn dragging_the_navigation_separator_snaps_to_columns() {
    let mut c = controller();
    // The NavigationPanel's right border is column 24.
    c.pointer_moved(at(24, 10));
    assert_eq!(c.pointer_pressed(), NOTHING);
    assert!(c.dragging());
    let update = c.pointer_moved(at(30, 10));
    assert!(update.grid);
    assert_eq!(c.shell().nav_width(), 31);
    // Within the same column nothing changes, wherever in the cell the pointer is.
    let again = c.pointer_moved(Some(UiPoint {
        x: 30 * W + W - 1,
        y: 10 * H,
    }));
    assert!(!again.grid);
    assert_eq!(c.grid(), &c.shell().compose().unwrap());
    c.pointer_released();
    assert!(!c.dragging());
    assert!(!c.pointer_moved(at(40, 10)).grid, "no drag after release");
    assert_eq!(c.shell().nav_width(), 31);
}

#[test]
fn dragging_outside_the_window_clamps() {
    let mut c = controller();
    c.pointer_moved(at(24, 10));
    c.pointer_pressed();
    assert!(c.pointer_moved(Some(UiPoint { x: u32::MAX, y: 0 })).grid);
    assert_eq!(c.shell().nav_width(), 41);
    // Leaving the window keeps the capture.
    c.pointer_moved(None);
    assert!(c.dragging());
    c.pointer_released();
}

#[test]
fn the_action_box_has_a_fixed_width_and_no_draggable_border() {
    let mut c = controller();
    // The box's left border (column 50 at 82 columns) is inert frame, not a separator.
    c.pointer_moved(at(50, 15));
    c.pointer_pressed();
    assert!(!c.dragging());
    assert!(!c.pointer_moved(at(40, 15)).grid);
    assert_eq!(c.shell().action_width(), 31);
    c.pointer_released();
}

#[test]
fn pressing_elsewhere_starts_no_drag() {
    let mut c = controller();
    c.pointer_moved(at(40, 10));
    c.pointer_pressed();
    assert!(!c.dragging());
}

#[test]
fn a_drag_survives_a_resize() {
    let mut c = controller();
    c.pointer_moved(at(24, 10));
    c.pointer_pressed();
    c.resize(120 * W, 40 * H);
    assert!(c.dragging());
    assert!(c.pointer_moved(at(50, 10)).grid);
    assert_eq!(c.shell().nav_width(), 51);
}

// ---- M2.17: window controls, drag and resize routing ----

use newell_app::controller::Press;
use newell_ui::{ResizeZone, WindowAction};

const COLS: u32 = 82;
const ROWS: u32 = 30;
const SURFACE: (u32, u32) = (COLS * W, ROWS * H);

fn window_controller() -> ShellController {
    ShellController::new(COLS, ROWS, GridMetrics::from_k(DEFAULT_K).unwrap()).unwrap()
}

fn press_at(c: &mut ShellController, col: u32, row: u32) -> Press {
    c.pointer_moved(at(col, row));
    c.press(SURFACE, true)
}

#[test]
fn window_controls_have_documented_cells_and_route_actions() {
    let mut c = window_controller();
    // MenuBar row 1, inner width 80: `[-] [□] [X] ` fills the last 12 inner cells.
    let (col, row) = find_glyph(c.grid(), '□');
    assert_eq!((col - 1, row), (COLS - 1 - 12 + 4, 1));
    let first = COLS - 1 - 12;
    for (offset, action) in [
        (0, WindowAction::Minimize),
        (1, WindowAction::Minimize),
        (2, WindowAction::Minimize),
        (4, WindowAction::ToggleMaximize),
        (5, WindowAction::ToggleMaximize),
        (6, WindowAction::ToggleMaximize),
        (8, WindowAction::Close),
        (9, WindowAction::Close),
        (10, WindowAction::Close),
    ] {
        assert_eq!(
            press_at(&mut c, first + offset, 1),
            Press::Window(action),
            "offset {offset}"
        );
    }
    // The gaps and the trailing space are inert MenuBar space: a drag.
    for offset in [3, 7, 11] {
        assert_eq!(press_at(&mut c, first + offset, 1), Press::Drag);
    }
}

#[test]
fn window_control_press_changes_no_shell_state() {
    let mut c = window_controller();
    let before = c.grid().clone();
    let first = COLS - 1 - 12;
    assert!(matches!(press_at(&mut c, first, 1), Press::Window(_)));
    assert_eq!(c.grid(), &before);
    assert!(!c.dragging());
    assert_eq!(c.shell().interaction().pressed, None);
}

#[test]
fn window_controls_win_over_an_open_menu() {
    let mut c = window_controller();
    c.key(ShellKey::Alt('f'));
    let first = COLS - 1 - 12;
    assert_eq!(
        press_at(&mut c, first + 8, 1),
        Press::Window(WindowAction::Close)
    );
}

#[test]
fn unused_menu_bar_space_drags_the_window() {
    let mut c = window_controller();
    assert_eq!(press_at(&mut c, 40, 1), Press::Drag);
    // A menu title is a control, not a drag region.
    let (col, row) = find_glyph(c.grid(), 'F');
    assert!(matches!(press_at(&mut c, col, row), Press::Shell(_)));
}

#[test]
fn drag_is_not_started_while_a_popup_or_modal_is_open() {
    let mut c = window_controller();
    c.key(ShellKey::Alt('f'));
    // The click closes the menu (existing behaviour) instead of dragging the window.
    assert!(matches!(press_at(&mut c, 40, 1), Press::Shell(u) if u.grid));
    // Modal.
    let (col, row) = find_glyph(c.grid(), '☰');
    assert!(matches!(press_at(&mut c, col, row), Press::Shell(_)));
    assert_ne!(press_at(&mut c, 40, 1), Press::Drag);
}

#[test]
fn eight_resize_zones_on_the_frame_ring() {
    let mut c = window_controller();
    let (last_col, last_row) = (COLS - 1, ROWS - 1);
    for (col, row, zone) in [
        (0, 0, ResizeZone::NorthWest),
        (40, 0, ResizeZone::North),
        (last_col, 0, ResizeZone::NorthEast),
        (0, 10, ResizeZone::West),
        (last_col, 10, ResizeZone::East),
        (0, last_row, ResizeZone::SouthWest),
        (40, last_row, ResizeZone::South),
        (last_col, last_row, ResizeZone::SouthEast),
    ] {
        assert_eq!(
            press_at(&mut c, col, row),
            Press::Resize(zone),
            "({col},{row})"
        );
    }
}

#[test]
fn resize_press_sets_no_widget_state_and_changes_no_grid() {
    let mut c = window_controller();
    let before = c.grid().clone();
    assert!(matches!(press_at(&mut c, 0, 10), Press::Resize(_)));
    assert_eq!(c.grid(), &before);
    assert_eq!(c.shell().interaction().pressed, None);
    assert!(!c.dragging());
}

#[test]
fn resize_zone_is_not_started_over_a_menu_and_does_not_close_it() {
    let mut c = window_controller();
    c.key(ShellKey::Alt('f'));
    let before = c.grid().clone();
    assert_eq!(press_at(&mut c, 0, 10), Press::Resize(ResizeZone::West));
    assert_eq!(c.grid(), &before, "an overlay is not dismissed");
    // A menu item in row 1 never starts a resize.
    let (col, row) = find_glyph(c.grid(), 'E');
    assert!(matches!(press_at(&mut c, col, row), Press::Shell(_)));
}

#[test]
fn resize_is_inactive_when_not_resizable() {
    let mut c = window_controller();
    c.pointer_moved(at(0, 10));
    assert!(matches!(c.press(SURFACE, false), Press::Shell(_)));
    // Controls still work while maximised.
    c.pointer_moved(at(COLS - 1 - 12, 1));
    assert!(matches!(c.press(SURFACE, false), Press::Window(_)));
}

#[test]
fn separators_and_shell_controls_keep_working() {
    let mut c = window_controller();
    c.pointer_moved(at(24, 10));
    assert_eq!(c.press(SURFACE, true), Press::Shell(NOTHING));
    assert!(c.dragging());
    c.pointer_released();
    let (col, row) = find_glyph(c.grid(), '☰');
    // The FormulaEditor opens.
    assert!(matches!(press_at(&mut c, col, row), Press::Shell(u) if u.grid));
}

#[test]
fn leftover_pixels_are_the_east_and_south_zone_and_a_press_without_cursor_is_shell() {
    let mut c = window_controller();
    c.pointer_moved(Some(UiPoint {
        x: SURFACE.0 + 7,
        y: 500,
    }));
    assert_eq!(
        c.press((SURFACE.0 + 10, SURFACE.1), true),
        Press::Resize(ResizeZone::East)
    );
    c.pointer_moved(None);
    assert!(matches!(c.press(SURFACE, true), Press::Shell(_)));
}

#[test]
fn minimum_grid_is_enforced_by_resize() {
    let mut c = window_controller();
    c.resize(10 * W, 5 * H);
    assert_eq!((c.grid().cols(), c.grid().rows()), (82, 30));
    c.resize(82 * W - 1, 30 * H - 1);
    assert_eq!((c.grid().cols(), c.grid().rows()), (82, 30));
    c.resize(82 * W, 30 * H);
    assert_eq!((c.grid().cols(), c.grid().rows()), (82, 30));
    c.resize(83 * W, 31 * H);
    assert_eq!((c.grid().cols(), c.grid().rows()), (83, 31));
}

#[test]
fn window_events_preserve_shell_state_and_panel_widths() {
    let mut c = window_controller();
    // Collapse a tree node, widen the navigation panel, then shrink below the minimum and grow.
    let (col, row) = find_glyph(c.grid(), '[');
    press_at(&mut c, col, row);
    c.pointer_moved(at(24, 10));
    c.press(SURFACE, true);
    c.pointer_moved(at(30, 10));
    c.pointer_released();
    let nav = c.shell().nav_width();
    let before = c.shell().clone();
    c.resize(0, 0);
    c.resize(120 * W, 40 * H);
    c.resize(SURFACE.0, SURFACE.1);
    assert_eq!(c.shell().nav_width(), nav);
    assert_eq!(c.shell().interaction(), before.interaction());
    assert_eq!(c.grid(), &c.shell().compose().unwrap());
}

// ---- M2.17 retest: a control acts on the release, over the control that was pressed ----

#[test]
fn window_control_is_found_under_the_pointer_only_on_its_cells() {
    let mut c = window_controller();
    let first = COLS - 1 - 12;
    assert_eq!(c.window_control_at_cursor(), None, "no pointer yet");
    for (col, row, expected) in [
        (first, 1, Some(WindowAction::Minimize)),
        (first + 5, 1, Some(WindowAction::ToggleMaximize)),
        (first + 9, 1, Some(WindowAction::Close)),
        // The gap, the frame ring above the bar and a menu title are not controls.
        (first + 3, 1, None),
        (first + 9, 0, None),
        (2, 1, None),
    ] {
        c.pointer_moved(at(col, row));
        assert_eq!(c.window_control_at_cursor(), expected, "({col}, {row})");
    }
    c.pointer_moved(None);
    assert_eq!(c.window_control_at_cursor(), None);
}

#[test]
fn the_pointer_stays_known_while_it_is_outside_the_window() {
    // A managed resize keeps feeding the pointer, including positions past the surface.
    let mut c = window_controller();
    c.pointer_moved(at(3, 3));
    c.pointer_moved(Some(UiPoint {
        x: SURFACE.0 + 500,
        y: SURFACE.1 + 500,
    }));
    assert_eq!(c.window_control_at_cursor(), None);
    c.pointer_moved(at(COLS - 1 - 12 + 9, 1));
    assert_eq!(c.window_control_at_cursor(), Some(WindowAction::Close));
}

#[test]
fn scene_navigation_respects_visible_pixels_menus_modals_and_action_box() {
    let mut c = window_controller();
    let viewport = c.scene_viewport();
    let point = at(viewport.col() + 2, viewport.row() + 4);
    c.pointer_moved(point);
    assert!(c.scene_input_enabled());
    let before = c.scene_viewport();
    for rect in c.scene_occlusions() {
        c.pointer_moved(at(rect.col() + 1, rect.row() + 1));
        assert!(!c.scene_input_enabled(), "opaque Action Box interior");
    }
    c.pointer_moved(at(0, 0));
    assert!(!c.scene_input_enabled());
    c.pointer_moved(None);
    assert!(!c.scene_input_enabled());
    c.pointer_moved(point);
    c.key(ShellKey::MenuBar);
    assert!(!c.scene_input_enabled());
    c.key(ShellKey::Escape);
    assert!(c.scene_input_enabled());
    let (col, row) = find_glyph(c.grid(), '☰');
    press_at(&mut c, col, row);
    c.pointer_moved(point);
    assert!(!c.scene_input_enabled(), "modal blocks the entire scene");
    c.key(ShellKey::Escape);
    assert_eq!(before, c.scene_viewport());
}

#[test]
fn opening_and_closing_action_box_does_not_reframe_scene() {
    fn click_text(c: &mut ShellController, text: &str) {
        let chars: Vec<_> = text.chars().collect();
        let (col, row) = (0..c.grid().rows())
            .find_map(|row| {
                (0..c.grid().cols()).find_map(|col| {
                    chars
                        .iter()
                        .enumerate()
                        .all(|(i, ch)| glyph(c.grid(), col + i as u32, row) == Some(*ch))
                        .then_some((col, row))
                })
            })
            .unwrap();
        press_at(c, col, row);
        c.pointer_released();
    }
    let mut c = window_controller();
    let viewport = c.scene_viewport();
    let occlusions = c.scene_occlusions();
    assert!(!occlusions.is_empty());
    click_text(&mut c, "Cancel");
    assert_eq!(c.scene_viewport(), viewport);
    assert!(c.scene_occlusions().is_empty());
    click_text(&mut c, "Create");
    click_text(&mut c, "[E]xtrude");
    assert_eq!(c.scene_viewport(), viewport);
    assert_eq!(c.scene_occlusions(), occlusions);
}
