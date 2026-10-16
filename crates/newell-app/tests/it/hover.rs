// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! M2.25: the Controller's ASCII hover target. Redraws, exit, eligibility, recomposition,
//! resize and press behaviour, through the same pointer API the application uses.

#![allow(clippy::unwrap_used)]

use newell_app::controller::{ShellController, Update};
use newell_ui::{CellContent, CellGrid, DEFAULT_K, GridMetrics, ShellKey, UiPoint};

// The 4:9 cell of `k = 5`: 20 x 45 physical pixels.
const W: u32 = 4 * DEFAULT_K;
const H: u32 = 9 * DEFAULT_K;

const NOTHING: Update = Update { grid: false };
const REDRAW: Update = Update { grid: true };

fn controller() -> ShellController {
    ShellController::new(82, 30, GridMetrics::from_k(DEFAULT_K).unwrap()).unwrap()
}

/// Centre of cell `(col, row)` in pixels, as a pointer position.
fn at(col: u32, row: u32) -> Option<UiPoint> {
    Some(UiPoint {
        x: col * W + W / 2,
        y: row * H + H / 2,
    })
}

fn row_text(g: &CellGrid, row: u32) -> String {
    (0..g.cols())
        .map(|col| match g.get(col, row).map(|c| c.content) {
            Some(CellContent::Glyph(c)) => c,
            _ => ' ',
        })
        .collect()
}

/// The cell of `cell_needle` in the first row that contains `row_needle`.
fn cell_of(g: &CellGrid, row_needle: &str, cell_needle: &str) -> (u32, u32) {
    (0..g.rows())
        .find_map(|row| {
            let text = row_text(g, row);
            if !text.contains(row_needle) {
                return None;
            }
            let col = text[..text.find(cell_needle)?].chars().count();
            Some((u32::try_from(col).unwrap(), row))
        })
        .unwrap()
}

/// The Starting Offset checkbox row of the Extrude Action Box (a target).
fn starting(c: &ShellController) -> (u32, u32) {
    cell_of(c.grid(), "Starting Offset", "Starting")
}

#[test]
fn entering_a_target_redraws_once_and_motion_inside_it_does_not() {
    let mut c = controller();
    let before = c.grid().clone();
    let (col, row) = starting(&c);
    assert_eq!(c.pointer_moved(at(col, row)), REDRAW);
    assert!(c.hovered().is_some());
    assert_ne!(c.grid(), &before);
    // Same widget (the checkbox owns its whole row): no hover redraw.
    assert_eq!(c.pointer_moved(at(col + 1, row)), NOTHING);
    assert_eq!(c.pointer_moved(at(col + 3, row)), NOTHING);
    // Exit restores the normal frame, cell for cell, in the same update.
    assert_eq!(c.pointer_moved(None), REDRAW);
    assert_eq!(c.hovered(), None);
    assert_eq!(c.grid(), &before);
    assert_eq!(c.grid(), &c.shell().compose().unwrap());
}

#[test]
fn moving_from_one_target_to_another_redraws_once_each_time() {
    let mut c = controller();
    let (col, row) = cell_of(c.grid(), "Starting Offset", "Starting");
    let (oc, orow) = cell_of(c.grid(), "Cancel", "OK");
    let (cc, crow) = cell_of(c.grid(), "OK", "Cancel");
    assert_eq!(c.pointer_moved(at(col, row)), REDRAW);
    assert_eq!(c.pointer_moved(at(oc, orow)), REDRAW);
    assert_eq!(c.pointer_moved(at(cc, crow)), REDRAW);
    assert_eq!(c.pointer_moved(None), REDRAW);
    assert_eq!(c.grid(), &c.shell().compose().unwrap());
}

#[test]
fn regions_of_one_row_change_the_id_but_not_the_glyphs() {
    let mut c = controller();
    let (fc, fr) = cell_of(c.grid(), "Depth", "+|");
    let (mc, mr) = cell_of(c.grid(), "Depth", "☰");
    assert_eq!(c.pointer_moved(at(fc, fr)), REDRAW);
    let on_field = c.grid().clone();
    let field = c.hovered();
    // The `☰` is another region of the same row: one redraw, the same frame.
    assert_eq!(c.pointer_moved(at(mc, mr)), REDRAW);
    assert_ne!(c.hovered(), field);
    assert_eq!(c.grid(), &on_field);
}

#[test]
fn menu_rows_are_targets_and_moving_between_them_redraws_once() {
    let mut c = controller();
    assert!(c.key(ShellKey::Alt('f')).grid);
    let (nc, nr) = cell_of(c.grid(), "New       ", "New");
    let (oc, or) = cell_of(c.grid(), "Open      ", "Open");
    assert_eq!(c.pointer_moved(at(nc, nr)), REDRAW);
    assert!(row_text(c.grid(), nr).contains("╞ New       ╡"));
    assert_eq!(c.pointer_moved(at(oc, or)), REDRAW);
    assert!(row_text(c.grid(), or).contains("╞ Open      ╡"));
    assert!(!row_text(c.grid(), nr).contains('╞'));
    assert_eq!(c.pointer_moved(None), REDRAW);
    assert_eq!(c.grid(), &c.shell().compose().unwrap());
}

#[test]
fn an_open_menu_covers_the_action_box_and_clears_its_hover() {
    let mut c = controller();
    let (col, row) = starting(&c);
    assert_eq!(c.pointer_moved(at(col, row)), REDRAW);
    assert!(c.hovered().is_some());
    // The File popup opens: the Action Box beneath is no longer a target.
    assert!(c.key(ShellKey::Alt('f')).grid);
    assert_eq!(c.hovered(), None);
    assert_eq!(c.grid(), &c.shell().compose().unwrap());
    // Motion over the covered control acquires nothing.
    assert_eq!(c.pointer_moved(at(col, row)), NOTHING);
    assert_eq!(c.hovered(), None);
}

#[test]
fn a_recomposition_never_acquires_a_target() {
    let mut c = controller();
    let (col, row) = starting(&c);
    assert!(c.key(ShellKey::Alt('f')).grid);
    assert_eq!(c.pointer_moved(at(col, row)), NOTHING);
    // Closing the menu recomposes under the pointer but must not make the control hover.
    assert!(c.key(ShellKey::Escape).grid);
    assert_eq!(c.hovered(), None);
    assert_eq!(c.grid(), &c.shell().compose().unwrap());
    // The first motion after that does acquire it.
    assert_eq!(c.pointer_moved(at(col, row)), REDRAW);
    assert!(c.hovered().is_some());
}

#[test]
fn a_target_under_the_pointer_that_is_covered_loses_its_hover() {
    let mut c = controller();
    let (col, row) = cell_of(c.grid(), "Depth", "☰");
    assert_eq!(c.pointer_moved(at(col, row)), REDRAW);
    assert!(c.hovered().is_some());
    // The press opens the FormulaEditor over the pointer: no target is left under it.
    assert!(c.pointer_pressed().grid);
    assert!(c.shell().formula().is_open());
    assert_eq!(c.hovered(), None);
    assert_eq!(c.grid(), &c.shell().compose().unwrap());
}

#[test]
fn a_press_keeps_the_hover_of_the_control_it_acts_on() {
    let mut c = controller();
    let (col, row) = starting(&c);
    assert_eq!(c.pointer_moved(at(col, row)), REDRAW);
    let target = c.hovered();
    assert!(c.pointer_pressed().grid, "the checkbox toggles");
    assert_eq!(c.hovered(), target);
    // Checked, the row shows its selector too; it is still the same checkbox row, still hovered.
    assert!(row_text(c.grid(), row).contains("╞ ■ Start Offset ◀ Blind ▶  ╡"));
    assert_eq!(c.grid(), &c.shell().compose_hovered(c.hovered()).unwrap().0);
}

#[test]
fn a_resize_clears_the_hover_and_ignores_motion_until_the_first_move_after_it() {
    let mut c = controller();
    let (col, row) = starting(&c);
    assert_eq!(c.pointer_moved(at(col, row)), REDRAW);
    assert_eq!(c.clear_hover(), REDRAW);
    assert_eq!(c.hovered(), None);
    assert_eq!(c.grid(), &c.shell().compose().unwrap());
    // The resize owns the pointer: motion over the target shows nothing.
    assert_eq!(c.pointer_moved_during_resize(at(col, row)), NOTHING);
    assert_eq!(c.hovered(), None);
    // The first move after the resize acquires it.
    assert_eq!(c.pointer_moved(at(col, row)), REDRAW);
    assert!(c.hovered().is_some());
}

#[test]
fn a_scale_change_drops_the_hover_because_the_pointer_is_forgotten() {
    let mut c = controller();
    let (col, row) = starting(&c);
    assert_eq!(c.pointer_moved(at(col, row)), REDRAW);
    let prepared = c
        .prepare_metrics(GridMetrics::from_k(3).unwrap(), 82 * 12, 30 * 27)
        .unwrap();
    assert_eq!(c.commit_metrics(prepared), REDRAW);
    assert_eq!(c.hovered(), None);
    assert_eq!(c.grid(), &c.shell().compose().unwrap());
}

#[test]
fn motion_over_widgets_that_are_not_targets_changes_nothing() {
    let mut c = controller();
    let before = c.grid().clone();
    // MenuBar title, CommandBar label, a tree row, the viewport and the window control.
    let (fc, fr) = cell_of(c.grid(), "File", "File");
    let (tc, tr) = cell_of(c.grid(), "Hub_Features", "Hub");
    let (xc, xr) = cell_of(c.grid(), "[X]", "[X]");
    for (col, row) in [(fc, fr), (tc, tr), (xc, xr), (40, 12)] {
        assert_eq!(c.pointer_moved(at(col, row)), NOTHING, "({col}, {row})");
        assert_eq!(c.grid(), &before);
    }
}

#[test]
fn action_box_labels_and_header_segments_follow_pointer_and_restore() {
    let mut c = controller();
    let normal = c.grid().clone();
    for (row, label) in [
        ("│ Profile ", "Profile"),
        ("Extend", "Blind"),
        ("Solid", "Solid"),
        ("Solid", "Surface"),
        ("New", "New"),
        ("New", "Add"),
        ("New", "Remove"),
    ] {
        let (col, r) = cell_of(&normal, row, label);
        assert_eq!(c.pointer_moved(at(col, r)), REDRAW, "{label}");
        assert!(c.hovered().is_some(), "{label}");
        assert_eq!(c.pointer_moved(at(col, r)), NOTHING);
        assert_eq!(c.pointer_moved(None), REDRAW);
        assert_eq!(c.grid(), &normal);
    }
}
