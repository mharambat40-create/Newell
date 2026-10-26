// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! M2.24 controller timing, stable hit regions and overlay/resize precedence.
#![allow(clippy::unwrap_used)]
use newell_app::controller::{Press, ShellController};
use newell_ui::{CellContent, CellGrid, GridMetrics, ShellKey, UiPoint, hit_test};
use std::time::Duration;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn controller() -> ShellController {
    ShellController::new(82, 30, GridMetrics::new(8, 18).unwrap()).unwrap()
}
fn text(g: &CellGrid, row: u32) -> String {
    (0..g.cols())
        .map(|col| match g.get(col, row).unwrap().content {
            CellContent::Glyph(c) => c,
            _ => ' ',
        })
        .collect()
}
fn progress(c: &ShellController) -> (u32, u32) {
    (0..c.grid().rows())
        .find_map(|row| {
            let line = text(c.grid(), row);
            let at = line.find("Progress [")?;
            Some((line[..at].chars().count() as u32, row))
        })
        .unwrap()
}
fn point(col: u32, row: u32) -> Option<UiPoint> {
    Some(UiPoint {
        x: col * 8 + 3,
        y: row * 18 + 9,
    })
}
fn click(c: &mut ShellController, col: u32, row: u32) {
    c.pointer_moved(point(col, row));
    c.press((656, 540), true);
    c.pointer_released();
}
fn percentage(c: &ShellController) -> String {
    let (col, row) = progress(c);
    text(c.grid(), row)
        .chars()
        .skip(col as usize)
        .take(21)
        .collect()
}

#[test]
fn progress_starts_at_zero_and_only_its_21_cells_activate_it() {
    for offset in 0..21 {
        let mut c = controller();
        let (col, row) = progress(&c);
        let id = hit_test(point(col, row).unwrap(), c.metrics(), c.grid())
            .unwrap()
            .widget_id;
        assert!(id.is_some());
        for dx in 0..21 {
            assert_eq!(
                hit_test(point(col + dx, row).unwrap(), c.metrics(), c.grid())
                    .unwrap()
                    .widget_id,
                id
            );
        }
        click(&mut c, col + offset, row);
        assert_eq!(c.hovered(), None);
        c.tick(ms(1999));
        assert!(percentage(&c).ends_with("95% "));
        c.tick(ms(2000));
        assert!(percentage(&c).ends_with("100%"));
        assert_eq!(
            hit_test(point(col, row).unwrap(), c.metrics(), c.grid())
                .unwrap()
                .widget_id,
            id
        );
    }
    for offset in [-1, 21] {
        let mut c = controller();
        let (col, row) = progress(&c);
        click(&mut c, col.saturating_add_signed(offset), row);
        c.tick(ms(2000));
        assert_eq!(percentage(&c), "Progress [     ] 0%  ");
    }
}

#[test]
fn held_pointer_and_mid_animation_presses_do_not_restart_or_queue() {
    let mut c = controller();
    let (col, row) = progress(&c);
    c.pointer_moved(point(col, row));
    c.pointer_pressed();
    for t in [500, 1000, 1500, 1999] {
        c.set_time(ms(t));
        c.pointer_moved(point(col + 1, row));
        c.pointer_pressed();
    }
    c.tick(ms(2000));
    assert!(percentage(&c).ends_with("100%"));
    c.pointer_released();
    c.tick(ms(5000));
    assert!(percentage(&c).ends_with("100%"));
    click(&mut c, col, row);
    c.tick(ms(6999));
    assert!(percentage(&c).ends_with("5%  "));
    c.tick(ms(7000));
    assert!(percentage(&c).ends_with("0%  "));
}

#[test]
fn menus_consume_progress_presses_without_click_through() {
    let mut c = controller();
    let (col, row) = progress(&c);
    c.key(ShellKey::Alt('f'));
    click(&mut c, col, row);
    c.tick(ms(5000));
    assert_eq!(percentage(&c), "Progress [     ] 0%  ");
    click(&mut c, col, row);
    c.tick(ms(10000));
    assert!(percentage(&c).ends_with("100%"));
}

#[test]
fn resize_ring_keeps_priority_with_an_open_menu() {
    let mut c = controller();
    c.key(ShellKey::Alt('f'));
    c.pointer_moved(point(0, 15));
    assert!(matches!(c.press((656, 540), true), Press::Resize(_)));
    c.tick(ms(5000));
    assert!(percentage(&c).ends_with("0%  "));
}

#[test]
fn measured_fps_updates_only_status_cells_and_idle_has_no_progress_deadline() {
    let mut c = controller();
    let before = c.grid().clone();
    assert_eq!(c.status_deadline(), ms(200));
    for _ in 0..12 {
        c.presented();
    }
    assert!(!c.tick(ms(199)).any());
    assert!(c.tick(ms(200)).any());
    assert!(text(c.grid(), 28).contains(" 60fps "));
    for row in 0..c.grid().rows() {
        for col in 0..c.grid().cols() {
            if row != 28 {
                assert_eq!(before.get(col, row), c.grid().get(col, row));
            }
        }
    }
    assert_eq!(c.status_deadline(), ms(400));
    c.tick(ms(400));
    assert!(text(c.grid(), 28).contains("  0fps "));
}

#[test]
fn a_non_dismissing_formula_modal_blocks_progress_without_dismissing_it() {
    // This modal test needs the reference Action Box; live startup intentionally has none.
    let mut c = ShellController::with_shell(
        newell_ui::ShellState::new(82, 30),
        GridMetrics::new(8, 18).unwrap(),
    )
    .unwrap();
    let (col, row) = (0..c.grid().rows())
        .find_map(|row| {
            let line = text(c.grid(), row);
            if !line.contains("Depth") {
                return None;
            }
            let at = line.find('☰')?;
            Some((line[..at].chars().count() as u32, row))
        })
        .unwrap();
    click(&mut c, col, row);
    assert!(c.shell().formula().is_open());
    let (col, row) = progress(&c);
    click(&mut c, col, row);
    c.tick(ms(5000));
    assert!(c.shell().formula().is_open());
    assert_eq!(percentage(&c), "Progress [     ] 0%  ");
}
