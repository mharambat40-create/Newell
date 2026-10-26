// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Run-time scale change (M2.17): the shell controller's prepare/commit pair and the
//! interaction of the scale manager with it. No window or GPU is involved.

#![allow(clippy::unwrap_used)]

use newell_app::controller::ShellController;
use newell_app::scale::{MonitorInfo, ScaleManager, plan_rescale};
use newell_ui::{
    CellContent, CellGrid, GridMetrics, ScaleMode, ShellKey, UiPoint, composed_grid_size, hit_test,
    min_window_size,
};

fn metrics(k: u32) -> GridMetrics {
    GridMetrics::from_k(k).unwrap()
}

fn at(m: GridMetrics, col: u32, row: u32) -> Option<UiPoint> {
    Some(UiPoint {
        x: col * m.cell_width_px() + m.cell_width_px() / 2,
        y: row * m.cell_height_px() + m.cell_height_px() / 2,
    })
}

fn glyph(g: &CellGrid, col: u32, row: u32) -> Option<char> {
    match g.get(col, row)?.content {
        CellContent::Glyph(c) => Some(c),
        CellContent::Empty => None,
    }
}

fn rescale_to(c: &mut ShellController, k: u32, size: (u32, u32)) {
    let prepared = c.prepare_metrics(metrics(k), size.0, size.1).unwrap();
    assert_eq!(prepared.metrics(), metrics(k));
    assert!(c.commit_metrics(prepared).grid);
}

#[test]
fn commit_publishes_metrics_and_grid_together() {
    let mut c = ShellController::new(82, 30, metrics(5)).unwrap();
    let before = c.grid().clone();
    // 1640×1350 at k = 5 is the minimum; at k = 3 the 984×810 minimum gives 82×30 too.
    let prepared = c.prepare_metrics(metrics(3), 984, 810).unwrap();
    assert_eq!(c.metrics(), metrics(5), "prepare touches nothing live");
    assert_eq!(c.grid(), &before);
    assert_eq!(prepared.grid().cols() * prepared.grid().rows(), 82 * 30);
    c.commit_metrics(prepared);
    assert_eq!(c.metrics(), metrics(3));
    assert_eq!((c.grid().cols(), c.grid().rows()), (82, 30));
}

#[test]
fn logical_grid_is_preserved_when_the_surface_still_holds_it() {
    // A 100×40 grid at k = 5 (2000×1800 px) moved to k = 3: the same 100×40 cells are
    // 1200×1080 px; the controller keeps the grid it is told fits.
    let mut c = ShellController::new(100, 40, metrics(5)).unwrap();
    rescale_to(&mut c, 3, (1200, 1080));
    assert_eq!((c.grid().cols(), c.grid().rows()), (100, 40));
}

#[test]
fn shell_state_survives_a_scale_change() {
    let m5 = metrics(5);
    let mut c = ShellController::new(100, 36, m5).unwrap();
    // Open the File menu and move the selection.
    c.pointer_moved(at(m5, 3, 1));
    assert!(c.pointer_pressed().grid);
    assert_eq!(glyph(c.grid(), 1, 2), Some('╭'), "File menu open");
    let state_before = c.shell().interaction().to_owned();
    let popup_open = c.grid().clone();

    rescale_to(&mut c, 3, (100 * 12, 36 * 27));

    assert_eq!(
        c.shell().interaction(),
        &state_before,
        "interaction state kept"
    );
    assert_eq!(
        c.grid(),
        &popup_open,
        "the open menu is recomposed identically"
    );
    assert_eq!(glyph(c.grid(), 1, 2), Some('╭'));
    // Keyboard input still reaches the open menu after the transition.
    assert!(c.key(ShellKey::Escape).grid);
    assert_eq!(glyph(c.grid(), 1, 2), Some('═'));
}

#[test]
fn pointer_is_forgotten_and_a_press_does_nothing_until_it_moves_again() {
    let m5 = metrics(5);
    let mut c = ShellController::new(82, 30, m5).unwrap();
    c.pointer_moved(at(m5, 3, 1));
    assert!(c.cursor().is_some());
    rescale_to(&mut c, 3, (984, 810));
    assert_eq!(
        c.cursor(),
        None,
        "positions of the old pixel space are dropped"
    );
    assert!(!c.pointer_pressed().grid, "nothing is synthesised");
    assert!(!c.dragging());
}

#[test]
fn a_separator_drag_is_cancelled_by_a_scale_change() {
    let m5 = metrics(5);
    let mut c = ShellController::new(100, 36, m5).unwrap();
    let separator_col = (0..100)
        .find(|&col| c.shell().separator_at(col, 10).is_some())
        .unwrap();
    c.pointer_moved(at(m5, separator_col, 10));
    c.pointer_pressed();
    assert!(c.dragging());
    rescale_to(&mut c, 3, (1200, 36 * 27));
    assert!(!c.dragging());
}

#[test]
fn pointer_coordinates_map_through_the_new_metrics() {
    for (k, w, h) in [
        (3, 12, 27),
        (4, 16, 36),
        (5, 20, 45),
        (6, 24, 54),
        (7, 28, 63),
    ] {
        let m5 = metrics(5);
        let mut c = ShellController::new(100, 40, m5).unwrap();
        rescale_to(&mut c, k, (100 * w, 40 * h));
        for (col, row) in [(0, 0), (3, 1), (50, 20), (99, 39)] {
            let p = UiPoint {
                x: col * w + w - 1,
                y: row * h + h - 1,
            };
            let hit = hit_test(p, c.metrics(), c.grid()).unwrap();
            assert_eq!((hit.col, hit.row), (col, row), "k = {k}");
            // The last pixel of a cell and the first of the next belong to different cells.
            let next = UiPoint { x: p.x + 1, y: p.y };
            assert_eq!(
                hit_test(next, c.metrics(), c.grid()).map(|h| h.col),
                Some(col + 1).filter(|&c| c < 100)
            );
        }
    }
}

#[test]
fn round_trip_macbook_full_hd_macbook_restores_cells_grid_and_preference() {
    let macbook = MonitorInfo::new(Some("built-in".into()), (0, 0), (2560, 1664), 2.0);
    let full_hd = MonitorInfo::new(Some("external".into()), (1280, 0), (1920, 1080), 1.0);
    let mut scale = ScaleManager::default();
    scale.start(4);
    scale.set_desired_grid(120, 34);
    let mut c = ShellController::new(120, 34, metrics(4)).unwrap();

    // Startup label 100% maps to factor 80: MacBook k = 4, no change.
    let r = scale
        .evaluate(Some(macbook.clone()), ScaleMode::Windowed)
        .unwrap();
    assert_eq!(r.decision.k, 4);

    // Move to Full HD.
    let r = scale.evaluate(Some(full_hd), ScaleMode::Windowed).unwrap();
    let plan = plan_rescale(&r, scale.desired_grid(), 1.0, (1400, 200), (2400, 1530)).unwrap();
    assert_eq!(plan.metrics, metrics(2));
    rescale_to(&mut c, 2, plan.size);
    assert_eq!(c.metrics(), metrics(2));
    assert!(plan.size.0 >= min_window_size(metrics(2)).0);
    let (cols, rows) = composed_grid_size(metrics(2), plan.size.0, plan.size.1);
    assert_eq!((c.grid().cols(), c.grid().rows()), (cols, rows));
    assert_eq!(
        (cols, rows),
        (120, 34),
        "the logical grid is kept where it fits"
    );

    // And back.
    let r = scale.evaluate(Some(macbook), ScaleMode::Windowed).unwrap();
    let plan = plan_rescale(
        &r,
        scale.desired_grid(),
        2.0,
        (100, 100),
        (cols * 8, rows * 18),
    )
    .unwrap();
    assert_eq!(plan.metrics, metrics(4));
    rescale_to(&mut c, 4, plan.size);
    assert_eq!(c.metrics(), metrics(4));
    assert_eq!((c.grid().cols(), c.grid().rows()), (120, 34));
    assert_eq!(scale.scale_factor_percent(), 80);
}

#[test]
fn startup_scale_factors_pair_metrics_grid_and_minimum_size() {
    // Startup label 100% uses factor 80: Full HD k2 and Retina k4. Cells stay 4:9, minimum
    // stays 82 × 30, and pointer mapping remains cell-aligned on both displays.
    for (monitor, k, cell, min) in [
        ((1920, 1080), 2, (8, 18), (656, 540)),
        ((2560, 1664), 4, (16, 36), (1312, 1080)),
    ] {
        let mon = MonitorInfo::new(None, (0, 0), monitor, 1.0);
        let mut scale = ScaleManager::default();
        scale.start(k);
        let r = scale
            .evaluate(Some(mon), ScaleMode::Windowed)
            .expect("first derivation runs");
        assert_eq!(r.decision.k, k, "{monitor:?}");
        let m = metrics(k);
        assert_eq!((m.cell_width_px(), m.cell_height_px()), cell);
        assert_eq!(min_window_size(m), min);
        let mut c = ShellController::new(82, 30, m).unwrap();
        rescale_to(&mut c, k, min);
        assert_eq!((c.grid().cols(), c.grid().rows()), (82, 30));
        let (cols, rows) = composed_grid_size(m, min.0 - 1, min.1 - 1);
        assert_eq!((cols, rows), (82, 30), "never below the minimum grid");
    }
}
