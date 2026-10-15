// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

#![allow(clippy::unwrap_used)]

use newell_app::window::{CancelReason, ControlArm, ResizeManager, check_fit, screen_pointer};
use newell_ui::{GridMetrics, ResizeSnapshot, ResizeTarget, ResizeZone, WindowAction};

fn metrics() -> GridMetrics {
    GridMetrics::from_k(5).unwrap()
}

fn snapshot(zone: ResizeZone) -> ResizeSnapshot {
    ResizeSnapshot {
        origin: (100, 200),
        size: (1640, 1350),
        zone,
        grab: (105, 205),
    }
}

fn started(zone: ResizeZone) -> ResizeManager {
    let mut m = ResizeManager::default();
    m.begin(snapshot(zone));
    m
}

#[test]
fn native_unsupported_is_remembered_and_reported_once() {
    let mut m = ResizeManager::default();
    assert!(!m.native_unsupported());
    assert!(m.note_native_unsupported(), "first time: log");
    assert!(m.native_unsupported());
    assert!(!m.note_native_unsupported(), "never logged again");
    assert!(m.native_unsupported());
}

#[test]
fn no_session_no_request() {
    let mut m = ResizeManager::default();
    assert!(!m.is_active());
    assert_eq!(m.update((500, 500), metrics()), None);
    assert!(!m.release(), "a release with no resize is not consumed");
    assert!(!m.cancel(CancelReason::Escape));
}

#[test]
fn update_requests_only_when_the_snapped_target_changes() {
    let mut m = started(ResizeZone::SouthEast);
    // Same window rectangle: nothing to request.
    assert_eq!(m.update((105, 205), metrics()), None);
    assert_eq!(m.update((110, 210), metrics()), None, "below half a cell");
    let t = m.update((205, 205), metrics()).unwrap();
    assert_eq!(t.size, (1640 + 100, 1350));
    assert_eq!(t.origin, (100, 200));
    // Same snapped target again, from a slightly different pointer: no request.
    assert_eq!(m.update((206, 205), metrics()), None);
    assert!(m.update((305, 205), metrics()).is_some());
    // Coming back to the starting rectangle is a change from the last request.
    assert_eq!(
        m.update((105, 205), metrics()),
        Some(ResizeTarget {
            origin: (100, 200),
            size: (1640, 1350)
        })
    );
}

#[test]
fn targets_do_not_depend_on_previous_updates() {
    let mut a = started(ResizeZone::NorthWest);
    let mut b = started(ResizeZone::NorthWest);
    for p in [(5, 5), (-400, 50), (900, 900), (-77, -33)] {
        a.update(p, metrics());
    }
    assert_eq!(
        a.update((-95, -45), metrics()),
        b.update((-95, -45), metrics())
    );
}

#[test]
fn rapid_pointer_movement_stays_whole_cell_and_anchored() {
    let mut m = started(ResizeZone::NorthWest);
    for step in 0..2000i64 {
        let x = 105 - 3 * step + (step % 7) * 11;
        let y = 205 - 2 * step - (step % 5) * 13;
        if let Some(t) = m.update((x, y), metrics()) {
            assert_eq!(t.size.0 % 20, 0);
            assert_eq!(t.size.1 % 45, 0);
            assert_eq!(i64::from(t.origin.0) + i64::from(t.size.0), 1740);
            assert_eq!(i64::from(t.origin.1) + i64::from(t.size.1), 1550);
            assert!(t.size.0 <= 8192 && t.size.1 <= 8192);
        }
    }
}

#[test]
fn release_is_consumed_once_and_ends_the_resize() {
    let mut m = started(ResizeZone::East);
    assert!(m.is_active());
    assert!(m.release());
    assert!(!m.is_active());
    assert!(!m.release());
    assert_eq!(
        m.update((900, 900), metrics()),
        None,
        "no request after release"
    );
}

#[test]
fn every_cancel_reason_ends_the_resize_and_stops_requests() {
    for reason in [
        CancelReason::FocusLost,
        CancelReason::ScaleFactorChanged,
        CancelReason::CloseRequested,
        CancelReason::StateChanged,
        CancelReason::Escape,
        CancelReason::InvalidPosition,
    ] {
        let mut m = started(ResizeZone::South);
        assert!(m.update((105, 400), metrics()).is_some());
        assert!(m.cancel(reason), "{reason:?}");
        assert!(!m.is_active());
        assert_eq!(m.update((105, 900), metrics()), None, "{reason:?}");
        assert!(!m.release(), "{reason:?}: nothing left to consume");
        assert!(!m.cancel(reason));
    }
}

#[test]
fn cancelling_keeps_the_native_capability() {
    let mut m = started(ResizeZone::South);
    m.note_native_unsupported();
    m.cancel(CancelReason::Escape);
    assert!(m.native_unsupported());
}

#[test]
fn a_new_press_replaces_the_session() {
    let mut m = started(ResizeZone::East);
    m.begin(snapshot(ResizeZone::South));
    let t = m.update((105, 305), metrics()).unwrap();
    assert_eq!(t.size.0, 1640, "the south zone leaves the width alone");
}

#[test]
fn screen_pointer_is_origin_plus_floored_pointer() {
    assert_eq!(screen_pointer((100, -50), (10.9, 3.2)), Some((110, -47)));
    assert_eq!(screen_pointer((0, 0), (-4.5, -0.1)), Some((-5, -1)));
    assert!(screen_pointer((i32::MIN, i32::MAX), (f64::MAX, f64::MIN)).is_some());
    assert_eq!(
        screen_pointer((i32::MAX, 0), (f64::MAX, 0.0)),
        Some((i64::MAX, 0))
    );
    assert_eq!(screen_pointer((0, 0), (f64::NAN, 1.0)), None);
    assert_eq!(screen_pointer((0, 0), (1.0, f64::INFINITY)), None);
    assert_eq!(screen_pointer((0, 0), (f64::NEG_INFINITY, 1.0)), None);
}

#[test]
fn hidpi_retina_scale_uses_physical_pixels_only() {
    // A 2x display: a 1640 x 1350 physical window is 820 x 675 logical. The pointer and the
    // origin arrive physical, so a 10 physical px drag (5 logical) is half a cell.
    let mut m = started(ResizeZone::East);
    let p = screen_pointer((100, 200), (5.0 + 10.0, 5.0)).unwrap();
    let t = m.update(p, metrics()).unwrap();
    assert_eq!(t.size, (1660, 1350));
}

#[test]
fn fit_report_flags_small_windows_and_monitor_overflow() {
    let min = (1640, 1350);
    let monitor = Some(((0, 0), (3024, 1964)));
    let ok = check_fit(min, min, ((100, 100), (1640, 1350)), monitor);
    assert!(!ok.below_minimum && !ok.exceeds_monitor);
    assert!(check_fit((1639, 1350), min, ((0, 0), (1639, 1350)), monitor).below_minimum);
    assert!(check_fit((1640, 1349), min, ((0, 0), (1640, 1349)), monitor).below_minimum);
    // Right/bottom overflow, and a negative origin (left of the monitor).
    assert!(check_fit(min, min, ((2000, 0), (1640, 1350)), monitor).exceeds_monitor);
    assert!(check_fit(min, min, ((0, 1000), (1640, 1350)), monitor).exceeds_monitor);
    assert!(check_fit(min, min, ((-1, 0), (1640, 1350)), monitor).exceeds_monitor);
    // Exactly the monitor is inside; a small display (k = 3 does not fit) is reported; k = 2 (656 × 540) fits Full HD.
    assert!(!check_fit(min, min, ((0, 0), (3024, 1964)), monitor).exceeds_monitor);
    let small = Some(((0, 0), (1366, 768)));
    assert!(check_fit((984, 810), (984, 810), ((0, 0), (984, 810)), small).exceeds_monitor);
    let full_hd = Some(((0, 0), (1920, 1080)));
    assert!(!check_fit((656, 540), (656, 540), ((100, 100), (656, 540)), full_hd).exceeds_monitor);
    // Unknown monitor cannot be exceeded; extreme values do not overflow.
    assert!(
        !check_fit(min, min, ((i32::MAX, i32::MAX), (u32::MAX, u32::MAX)), None).exceeds_monitor
    );
    assert!(check_fit(min, min, ((i32::MAX, 0), (u32::MAX, 5)), monitor).exceeds_monitor);
}

// ---- M2.17 retest: window controls act on the release ----

#[test]
fn a_control_acts_only_when_released_over_the_pressed_control() {
    let mut arm = ControlArm::default();
    assert_eq!(
        arm.release(Some(WindowAction::Close)),
        None,
        "nothing pressed"
    );
    arm.arm(WindowAction::Close);
    assert!(arm.is_armed());
    assert_eq!(
        arm.release(Some(WindowAction::Close)),
        Some(WindowAction::Close)
    );
    assert!(!arm.is_armed());
    assert_eq!(
        arm.release(Some(WindowAction::Close)),
        None,
        "one release, one action"
    );
}

#[test]
fn a_release_elsewhere_or_after_a_cancel_runs_nothing() {
    let mut arm = ControlArm::default();
    arm.arm(WindowAction::ToggleMaximize);
    assert_eq!(arm.release(Some(WindowAction::Close)), None);
    assert!(!arm.is_armed(), "a release always ends the press");
    arm.arm(WindowAction::Minimize);
    assert_eq!(arm.release(None), None);
    arm.arm(WindowAction::Minimize);
    arm.cancel();
    assert_eq!(arm.release(Some(WindowAction::Minimize)), None);
}

/// `winit` on macOS toggles the style mask inside `is_maximized` and `set_maximized`; called
/// while a press is handled that swallowed the mouse-up, which left every managed resize
/// running forever and froze the controls. It may only run when a control fires.
#[test]
fn is_maximized_is_never_called_outside_the_control_action() {
    let source = include_str!("../../src/app.rs");
    let calls: Vec<&str> = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("//") && l.contains(".is_maximized()"))
        .collect();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert!(calls[0].contains("let maximize = !window.is_maximized()"));
}
