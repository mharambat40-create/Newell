// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

#![allow(clippy::unwrap_used)]

use newell_app::controller::{ShellController, Update};
use newell_ui::{CellContent, CellGrid, GridMetrics, ShellKey, UiPoint};

const W: u32 = 24;
const H: u32 = 40;

fn controller() -> ShellController {
    ShellController::new(82, 25, GridMetrics::new(W, H).unwrap()).unwrap()
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

const NOTHING: Update = Update {
    grid: false,
    hover: false,
};

#[test]
fn starts_with_the_composed_reference_shell() {
    let c = controller();
    assert_eq!((c.grid().cols(), c.grid().rows()), (82, 25));
    assert_eq!(c.hovered(), None);
    assert_eq!(glyph(c.grid(), 0, 0), Some('╔'));
    assert_eq!(c.grid(), &c.shell().compose().unwrap());
}

#[test]
fn hover_follows_the_pointer_without_recomposing() {
    let mut c = controller();
    let before = c.grid().clone();
    let u = c.pointer_moved(at(3, 1)); // File
    assert_eq!(
        u,
        Update {
            grid: false,
            hover: true
        }
    );
    let file = c.hovered();
    assert!(file.is_some());
    assert_eq!(c.pointer_moved(at(4, 1)), NOTHING, "same widget");
    assert!(c.pointer_moved(at(3, 3)).hover); // Workbench
    assert_ne!(c.hovered(), file);
    assert!(c.pointer_moved(at(30, 12)).hover); // empty viewport
    assert_eq!(c.hovered(), None);
    c.pointer_moved(at(3, 1));
    assert!(c.pointer_moved(None).hover, "pointer left the window");
    assert_eq!(c.hovered(), None);
    assert_eq!(c.grid(), &before);
}

#[test]
fn press_opens_the_file_menu_and_escape_closes_it() {
    let mut c = controller();
    c.pointer_moved(at(3, 1));
    let u = c.pointer_pressed();
    assert!(u.grid);
    assert_eq!(glyph(c.grid(), 1, 2), Some('┌'));
    assert!(c.key(ShellKey::Escape).grid);
    assert_eq!(glyph(c.grid(), 1, 2), Some('═'));
}

#[test]
fn hover_is_resolved_again_after_the_grid_changes_under_a_still_pointer() {
    let mut c = controller();
    c.pointer_moved(at(3, 3)); // the Workbench title
    let title = c.hovered();
    let u = c.key(ShellKey::Alt('f')); // File popup now covers that cell
    assert_eq!(
        u,
        Update {
            grid: true,
            hover: true
        }
    );
    assert_ne!(c.hovered(), title);
    assert!(c.hovered().is_some(), "now the File popup's New row");
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
    assert_eq!(c.hovered(), None);
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
    let a = Update {
        grid: true,
        hover: false,
    };
    let b = Update {
        grid: false,
        hover: true,
    };
    assert_eq!(
        a.merge(b),
        Update {
            grid: true,
            hover: true
        }
    );
    assert_eq!(NOTHING.merge(NOTHING), NOTHING);
    assert!(!NOTHING.any() && a.any() && b.any());
}

#[test]
fn degenerate_grids_are_safe() {
    let metrics = GridMetrics::new(W, H).unwrap();
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
