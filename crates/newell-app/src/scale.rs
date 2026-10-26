// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Automatic per-monitor UI scale of one window (M2.17, `ARCHITECTURE.md` §3.1.1).
//!
//! [`ScaleManager`] owns the three separate values: the user's `scale_factor_percent`, the
//! monitor's reference scale and `effective_k`, plus the grid the user last chose. It decides
//! *whether* a re-derivation is needed from a monitor snapshot and *what* the window must
//! become ([`plan_rescale`]); the `winit` and GPU calls live in `app.rs`. There are no `winit`
//! types here, so every rule is testable without a window or a display.
//!
//! `effective_k` is a pure function of (monitor size, mode, preference). The window's own size
//! and position are outputs, never inputs, so a resize cannot change `k` and the transition
//! cannot feed back into itself.

use newell_ui::{
    DEFAULT_SCALE_FACTOR_PERCENT, GridMetrics, MIN_COLS, MIN_ROWS, ScaleDecision, ScaleMode,
    ScreenRect, UiError, effective_scale, fit_window,
};

/// What is known about the monitor that hosts the window, in the monitor's own physical pixels.
///
/// On macOS `size` is the display mode's points times the backing scale factor and `position`
/// is the display origin in points times the same factor, so positions of monitors with
/// different scale factors are not in one pixel space (see [`rect_in_window_space`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorInfo {
    /// Identity for logs and change detection. Never used to choose a scale.
    pub name: Option<String>,
    pub position: (i32, i32),
    pub size: (u32, u32),
    /// `f64::to_bits` of the monitor's scale factor (kept as bits so the type is `Eq`).
    scale_bits: u64,
}

impl MonitorInfo {
    #[must_use]
    pub fn new(
        name: Option<String>,
        position: (i32, i32),
        size: (u32, u32),
        scale_factor: f64,
    ) -> Self {
        Self {
            name,
            position,
            size,
            scale_bits: scale_factor.to_bits(),
        }
    }

    #[must_use]
    pub fn scale_factor(&self) -> f64 {
        f64::from_bits(self.scale_bits)
    }

    /// The monitor rectangle in its own pixels.
    #[must_use]
    pub const fn rect(&self) -> ScreenRect {
        (self.position, self.size)
    }
}

/// The monitor rectangle expressed in the pixel space of a window whose scale factor is
/// `window_scale`. Window positions and sizes are logical points times the window's scale
/// factor, and monitor positions are logical points times the monitor's, so across monitors
/// of different scale factors the two must be brought to one space before they are compared.
/// When the scale factors are equal (the window is on that monitor) this is the identity.
#[must_use]
pub fn rect_in_window_space(monitor: &MonitorInfo, window_scale: f64) -> ScreenRect {
    let monitor_scale = monitor.scale_factor();
    if !(window_scale.is_finite() && monitor_scale.is_finite())
        || window_scale <= 0.0
        || monitor_scale <= 0.0
        || (window_scale - monitor_scale).abs() < f64::EPSILON
    {
        return monitor.rect();
    }
    let ratio = window_scale / monitor_scale;
    let scale = |v: i32| (f64::from(v) * ratio).round() as i32;
    let scale_u = |v: u32| (f64::from(v) * ratio).round() as u32;
    (
        (scale(monitor.position.0), scale(monitor.position.1)),
        (scale_u(monitor.size.0), scale_u(monitor.size.1)),
    )
}

/// The inputs of one derivation.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Input {
    monitor: Option<MonitorInfo>,
    mode: ScaleMode,
    scale_factor_percent: u32,
}

/// A re-derivation that changed an input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rescale {
    pub decision: ScaleDecision,
    pub monitor: Option<MonitorInfo>,
    pub mode: ScaleMode,
    /// The `k` in use before (`None` before the first derivation).
    pub previous_k: Option<u32>,
    /// `true` if the cell size changes: the atlas, metrics, grid and buffers are replaced.
    pub k_changed: bool,
    /// `true` the first time a monitor on which even `k = 2` does not fit is seen.
    pub log_limit: bool,
}

/// Per-window scale state: preference, derived `k`, the last evaluated inputs and the grid the
/// user last chose.
#[derive(Debug)]
pub struct ScaleManager {
    scale_factor_percent: u32,
    k: Option<u32>,
    input: Option<Input>,
    /// The grid the user last chose; derived changes never overwrite it, so a window moved to
    /// a smaller monitor and back regains it.
    desired: (u32, u32),
    limit_logged_for: Option<(u32, u32)>,
}

impl Default for ScaleManager {
    fn default() -> Self {
        Self {
            scale_factor_percent: DEFAULT_SCALE_FACTOR_PERCENT,
            k: None,
            input: None,
            desired: (MIN_COLS, MIN_ROWS),
            limit_logged_for: None,
        }
    }
}

impl ScaleManager {
    /// The rendering multiplier selected by the View menu. Never changed by a monitor.
    #[must_use]
    pub const fn scale_factor_percent(&self) -> u32 {
        self.scale_factor_percent
    }

    /// Stores the selected rendering multiplier. The next [`Self::evaluate`] re-derives `k`.
    pub fn set_scale_factor_percent(&mut self, scale_factor_percent: u32) {
        self.scale_factor_percent = scale_factor_percent;
    }

    /// The `k` currently in use, if any.
    #[must_use]
    pub const fn effective_k(&self) -> Option<u32> {
        self.k
    }

    /// Declares the `k` the window was created with, before the first derivation.
    pub fn start(&mut self, k: u32) {
        self.k = Some(k);
        self.input = None;
    }

    /// The grid the user last chose.
    #[must_use]
    pub const fn desired_grid(&self) -> (u32, u32) {
        self.desired
    }

    /// Records the grid the user chose (a window resize by the user, never a derived one).
    /// Never below 82 × 30.
    pub fn set_desired_grid(&mut self, cols: u32, rows: u32) {
        self.desired = (cols.max(MIN_COLS), rows.max(MIN_ROWS));
    }

    /// Derives the scale for `monitor` and `mode`. Returns `None` if the inputs equal the last
    /// evaluated ones (idempotent: nothing to do), otherwise the change to apply. The new
    /// inputs are recorded at once, so a failed application is not retried until an input
    /// changes ([`Self::revert`]).
    pub fn evaluate(&mut self, monitor: Option<MonitorInfo>, mode: ScaleMode) -> Option<Rescale> {
        let input = Input {
            monitor,
            mode,
            scale_factor_percent: self.scale_factor_percent,
        };
        if self.input.as_ref() == Some(&input) {
            return None;
        }
        let decision = effective_scale(
            input.monitor.as_ref().map(|m| m.size),
            mode,
            self.scale_factor_percent,
        );
        let previous_k = self.k;
        self.k = Some(decision.k);
        let size = input.monitor.as_ref().map(|m| m.size);
        let log_limit = !decision.fits && self.limit_logged_for != size;
        if !decision.fits {
            self.limit_logged_for = size;
        }
        let rescale = Rescale {
            decision,
            monitor: input.monitor.clone(),
            mode,
            previous_k,
            k_changed: previous_k != Some(decision.k),
            log_limit,
        };
        self.input = Some(input);
        Some(rescale)
    }

    /// The application of a [`Rescale`] failed and was abandoned: the window keeps
    /// `previous_k`. The inputs stay recorded, so the same tuple is not retried.
    pub fn revert(&mut self, previous_k: Option<u32>) {
        self.k = previous_k;
    }
}

/// The window the transition must produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RescalePlan {
    pub metrics: GridMetrics,
    /// Inner size in physical pixels, whole cells, never below 82 × 30.
    pub size: (u32, u32),
    pub origin: (i32, i32),
    /// `false` if the minimum grid does not fit the monitor (logged by the caller).
    pub fits: bool,
}

/// Window size and position for `rescale` (`ARCHITECTURE.md` §3.1.1 *Safe window fit*).
///
/// `window_scale` is the window's scale factor and `origin` its outer position, both in the
/// window's pixel space; `inner` is its current inner size. In fullscreen the OS owns size and
/// position, so the plan keeps them and only the metrics change. An unknown monitor keeps the
/// desired grid and the origin.
///
/// # Errors
/// If `k` does not give valid metrics.
pub fn plan_rescale(
    rescale: &Rescale,
    desired: (u32, u32),
    window_scale: f64,
    origin: (i32, i32),
    inner: (u32, u32),
) -> Result<RescalePlan, UiError> {
    let metrics = GridMetrics::from_k(rescale.decision.k)?;
    let Some(monitor) = &rescale.monitor else {
        let fit = fit_window(
            metrics,
            ((i32::MIN / 2, i32::MIN / 2), (u32::MAX / 2, u32::MAX / 2)),
            rescale.mode,
            desired,
            origin,
        );
        return Ok(RescalePlan {
            metrics,
            size: fit.size,
            origin,
            fits: true,
        });
    };
    if rescale.mode == ScaleMode::Fullscreen {
        return Ok(RescalePlan {
            metrics,
            size: inner,
            origin,
            fits: rescale.decision.fits,
        });
    }
    let rect = rect_in_window_space(monitor, window_scale);
    let fit = fit_window(metrics, rect, rescale.mode, desired, origin);
    Ok(RescalePlan {
        metrics,
        size: fit.size,
        origin: fit.origin,
        fits: fit.fits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use newell_ui::min_window_size;

    const WINDOWED: ScaleMode = ScaleMode::Windowed;

    /// MacBook Air Retina: scale factor 2, at the origin of the desktop.
    fn macbook() -> MonitorInfo {
        MonitorInfo::new(Some("built-in".into()), (0, 0), (2560, 1664), 2.0)
    }

    /// External Full HD to the right of the MacBook. macOS positions in points times the
    /// monitor's own scale: the display origin x = 1280 points × 1.0.
    fn full_hd_right() -> MonitorInfo {
        MonitorInfo::new(Some("external".into()), (1280, 0), (1920, 1080), 1.0)
    }

    /// External Full HD to the left of the MacBook, with its top above the MacBook's.
    fn full_hd_left() -> MonitorInfo {
        MonitorInfo::new(Some("external".into()), (-1920, -300), (1920, 1080), 1.0)
    }

    fn k_of(manager: &mut ScaleManager, monitor: &MonitorInfo) -> u32 {
        manager
            .evaluate(Some(monitor.clone()), WINDOWED)
            .unwrap()
            .decision
            .k
    }

    #[test]
    fn startup_label_100_uses_the_former_80_factor_on_retina() {
        let mut m = ScaleManager::default();
        m.start(4);
        let r = m.evaluate(Some(macbook()), WINDOWED).unwrap();
        assert_eq!((r.decision.k, r.k_changed), (4, false));
        assert_eq!(m.scale_factor_percent(), 80);
        let metrics = GridMetrics::from_k(r.decision.k).unwrap();
        assert_eq!(
            (metrics.cell_width_px(), metrics.cell_height_px()),
            (16, 36)
        );
    }

    #[test]
    fn launching_directly_on_full_hd_gives_k2_and_8_by_18() {
        // The window is created with the primary monitor's k (5), but the actual host is the
        // external monitor: the first derivation must change it.
        let mut m = ScaleManager::default();
        m.start(5);
        let r = m.evaluate(Some(full_hd_right()), WINDOWED).unwrap();
        assert_eq!(
            (r.decision.k, r.k_changed, r.previous_k),
            (2, true, Some(5))
        );
        assert_eq!(
            m.scale_factor_percent(),
            80,
            "the startup factor is untouched"
        );
        let metrics = GridMetrics::from_k(2).unwrap();
        assert_eq!((metrics.cell_width_px(), metrics.cell_height_px()), (8, 18));
    }

    #[test]
    fn macbook_to_full_hd_reduces_k_and_back_restores_it() {
        let mut m = ScaleManager::default();
        m.start(4);
        assert_eq!(k_of(&mut m, &macbook()), 4);
        assert_eq!(k_of(&mut m, &full_hd_right()), 2);
        assert_eq!(k_of(&mut m, &macbook()), 4);
        assert_eq!(m.scale_factor_percent(), 80);
        assert_eq!(m.effective_k(), Some(4));
    }

    #[test]
    fn identical_inputs_are_a_no_op() {
        let mut m = ScaleManager::default();
        m.start(5);
        assert!(m.evaluate(Some(macbook()), WINDOWED).is_some());
        for _ in 0..10 {
            assert!(
                m.evaluate(Some(macbook()), WINDOWED).is_none(),
                "idempotent"
            );
        }
        assert!(m.evaluate(Some(full_hd_right()), WINDOWED).is_some());
        assert!(m.evaluate(Some(full_hd_right()), WINDOWED).is_none());
    }

    #[test]
    fn a_monitor_change_with_the_same_k_is_reported_without_a_k_change() {
        let mut m = ScaleManager::default();
        m.start(2);
        assert!(m.evaluate(Some(full_hd_right()), WINDOWED).is_some());
        let r = m.evaluate(Some(full_hd_left()), WINDOWED).unwrap();
        assert!(!r.k_changed);
    }

    #[test]
    fn resolution_and_display_scale_changes_rederive_k() {
        let mut m = ScaleManager::default();
        m.set_scale_factor_percent(100);
        m.start(5);
        assert_eq!(k_of(&mut m, &macbook()), 5);
        // macOS "scaled resolution" change: same display, other backing size and factor.
        let scaled = MonitorInfo::new(Some("built-in".into()), (0, 0), (1920, 1080), 1.0);
        assert_eq!(k_of(&mut m, &scaled), 2);
        let more_space = MonitorInfo::new(Some("built-in".into()), (0, 0), (3024, 1964), 2.0);
        assert_eq!(k_of(&mut m, &more_space), 5);
    }

    /// The UI Scale control steps through the four preferences; on 1920 × 1080 every step gives
    /// `k = 2`, so no step runs a transition (`k_changed` false).
    #[test]
    fn full_hd_preference_steps_change_no_k() {
        let mut m = ScaleManager::default();
        m.start(2);
        assert_eq!(k_of(&mut m, &full_hd_right()), 2);
        let mut seen = Vec::new();
        for percent in [120, 60, 80, 100] {
            m.set_scale_factor_percent(percent);
            let r = m.evaluate(Some(full_hd_right()), WINDOWED).unwrap();
            seen.push((percent, r.decision.k, r.k_changed));
            assert_eq!(m.scale_factor_percent(), percent);
        }
        assert_eq!(
            seen,
            [
                (120, 2, false),
                (60, 2, false),
                (80, 2, false),
                (100, 2, false)
            ]
        );
    }

    /// On the Retina panel the four preferences are `k` = 3 / 4 / 5 / 5 and the preference
    /// is stored independently of the derived `k`.
    #[test]
    fn retina_preference_ladder_keeps_the_stored_percent() {
        let mut m = ScaleManager::default();
        m.start(5);
        let ks: Vec<u32> = [60, 80, 100, 120]
            .into_iter()
            .map(|p| {
                m.set_scale_factor_percent(p);
                let k = k_of(&mut m, &macbook());
                assert_eq!(m.scale_factor_percent(), p);
                k
            })
            .collect();
        assert_eq!(ks, [3, 4, 5, 5]);
    }

    /// `ARCHITECTURE.md` §3.1.1 worked table: `effective_k` for 60 / 80 / 100 / 120 %.
    #[test]
    fn effective_k_matches_the_architecture_table_for_every_preference() {
        type Row = ((u32, u32), [u32; 4], [u32; 4]);
        let rows: [Row; 7] = [
            ((1366, 768), [2, 2, 2, 2], [2, 2, 2, 2]),
            ((1920, 1080), [2, 2, 2, 2], [2, 2, 2, 2]),
            ((1920, 1200), [2, 3, 4, 4], [2, 3, 4, 4]),
            ((2560, 1440), [3, 4, 5, 5], [3, 4, 5, 5]),
            ((2560, 1664), [3, 4, 5, 5], [3, 4, 5, 6]),
            ((3024, 1964), [3, 4, 5, 6], [3, 4, 5, 6]),
            ((3840, 2160), [3, 4, 5, 6], [3, 4, 5, 6]),
        ];
        for (size, windowed, fullscreen) in rows {
            let monitor = MonitorInfo::new(None, (0, 0), size, 1.0);
            for (mode, expected) in [(WINDOWED, windowed), (ScaleMode::Fullscreen, fullscreen)] {
                let ks: Vec<u32> = [60, 80, 100, 120]
                    .into_iter()
                    .map(|p| {
                        let mut m = ScaleManager::default();
                        m.set_scale_factor_percent(p);
                        m.evaluate(Some(monitor.clone()), mode).unwrap().decision.k
                    })
                    .collect();
                assert_eq!(ks, expected, "{size:?} {mode:?}");
            }
        }
    }

    /// The full UI Scale cycle 60 → 80 → 100 → 120 → 60 on one manager: the stored
    /// preference follows every step and the wrap returns to the 60 % cell.
    #[test]
    fn full_cycle_wraps_back_to_60() {
        for (monitor, expected) in [
            (full_hd_right(), [2, 2, 2, 2, 2]),
            (
                MonitorInfo::new(None, (0, 0), (1920, 1200), 1.0),
                [2, 3, 4, 4, 2],
            ),
            (
                MonitorInfo::new(None, (0, 0), (3840, 2160), 1.0),
                [3, 4, 5, 6, 3],
            ),
        ] {
            let mut m = ScaleManager::default();
            m.start(expected[2]);
            let mut ks = Vec::new();
            for p in [60, 80, 100, 120, 60] {
                m.set_scale_factor_percent(p);
                let k = match m.evaluate(Some(monitor.clone()), WINDOWED) {
                    Some(r) => r.decision.k,
                    None => m.effective_k().unwrap(),
                };
                assert_eq!(m.scale_factor_percent(), p);
                assert_eq!(m.effective_k(), Some(k));
                ks.push(k);
            }
            assert_eq!(ks, expected, "{:?}", monitor.size);
        }
    }

    #[test]
    fn fullscreen_and_preference_changes_rederive_k() {
        let mut m = ScaleManager::default();
        m.start(2);
        assert_eq!(k_of(&mut m, &full_hd_right()), 2);
        let fs = m
            .evaluate(Some(full_hd_right()), ScaleMode::Fullscreen)
            .unwrap();
        assert_eq!(fs.decision.k, 2, "100 % is the same in both modes");
        let back = m.evaluate(Some(full_hd_right()), WINDOWED).unwrap();
        assert_eq!(back.decision.k, 2);
        m.set_scale_factor_percent(120);
        assert_eq!(k_of(&mut m, &macbook()), 5, "clamped to the monitor's cap");
        m.set_scale_factor_percent(60);
        assert_eq!(k_of(&mut m, &macbook()), 3);
        assert_eq!(m.scale_factor_percent(), 60);
    }

    #[test]
    fn unknown_monitor_uses_reference_five_then_adapts() {
        let mut m = ScaleManager::default();
        m.start(5);
        let r = m.evaluate(None, WINDOWED).unwrap();
        assert_eq!(r.decision.k, 4, "unknown monitor uses startup factor 80%");
        assert_eq!(
            k_of(&mut m, &full_hd_right()),
            2,
            "re-derived when information arrives"
        );
    }

    #[test]
    fn failed_application_is_not_retried_until_an_input_changes() {
        let mut m = ScaleManager::default();
        m.start(5);
        let r = m.evaluate(Some(full_hd_right()), WINDOWED).unwrap();
        m.revert(r.previous_k);
        assert_eq!(m.effective_k(), Some(5));
        assert!(m.evaluate(Some(full_hd_right()), WINDOWED).is_none());
        assert!(m.evaluate(Some(macbook()), WINDOWED).is_some());
    }

    #[test]
    fn the_small_monitor_limit_is_reported_once_per_monitor_size() {
        let small = MonitorInfo::new(None, (0, 0), (640, 480), 1.0);
        let mut m = ScaleManager::default();
        m.start(5);
        let first = m.evaluate(Some(small.clone()), WINDOWED).unwrap();
        assert!(first.log_limit && !first.decision.fits);
        let moved = MonitorInfo::new(None, (10, 0), (640, 480), 1.0);
        assert!(!m.evaluate(Some(moved), WINDOWED).unwrap().log_limit);
        assert_eq!(first.decision.k, 2);
    }

    #[test]
    fn desired_grid_is_never_below_the_minimum_and_survives_derived_changes() {
        let mut m = ScaleManager::default();
        assert_eq!(m.desired_grid(), (82, 30));
        m.set_desired_grid(1, 1);
        assert_eq!(m.desired_grid(), (82, 30));
        m.set_desired_grid(120, 34);
        m.start(5);
        let _ = k_of(&mut m, &full_hd_right());
        let _ = k_of(&mut m, &macbook());
        assert_eq!(m.desired_grid(), (120, 34));
    }

    fn plan(rescale: &Rescale, desired: (u32, u32), scale: f64, origin: (i32, i32)) -> RescalePlan {
        plan_rescale(rescale, desired, scale, origin, (1640, 1350)).unwrap()
    }

    #[test]
    fn plan_moving_to_full_hd_shrinks_cells_and_keeps_the_logical_grid_when_it_fits() {
        let mut m = ScaleManager::default();
        m.start(5);
        m.set_desired_grid(82, 30);
        let r = m.evaluate(Some(full_hd_right()), WINDOWED).unwrap();
        // The window is still at its MacBook-sized origin, partly off the external monitor.
        let p = plan(&r, m.desired_grid(), 1.0, (1500, 700));
        assert_eq!(p.size, min_window_size(p.metrics));
        assert_eq!(p.size, (656, 540));
        assert!(p.fits);
        let ((ux, uy), (uw, uh)) = newell_ui::usable_rect(full_hd_right().rect(), WINDOWED);
        assert!(p.origin.0 >= ux && p.origin.1 >= uy);
        assert!(i64::from(p.origin.0) + 656 <= i64::from(ux) + i64::from(uw));
        assert!(i64::from(p.origin.1) + 540 <= i64::from(uy) + i64::from(uh));
    }

    #[test]
    fn plan_clamps_a_big_desired_grid_and_returns_to_it() {
        let mut m = ScaleManager::default();
        m.start(5);
        m.set_desired_grid(100, 70);
        let down = m.evaluate(Some(full_hd_right()), WINDOWED).unwrap();
        let on_hd = plan(&down, m.desired_grid(), 1.0, (1300, 100));
        assert_eq!(on_hd.size.0, 800);
        assert_eq!(on_hd.size.1, 56 * 18, "70 rows clamp to the usable height");
        assert!(on_hd.size.1 <= 1013, "inside the usable height");
        let up = m.evaluate(Some(macbook()), WINDOWED).unwrap();
        let on_mac = plan(&up, m.desired_grid(), 2.0, (100, 100));
        assert_eq!(
            on_mac.size.0, 1600,
            "the desired 100 columns are back at 16 px"
        );
        assert_eq!(on_mac.size.1 % 36, 0);
    }

    #[test]
    fn plan_handles_other_monitor_arrangements_and_origins() {
        let mut m = ScaleManager::default();
        m.start(5);
        m.set_desired_grid(82, 30);
        let r = m.evaluate(Some(full_hd_left()), WINDOWED).unwrap();
        // The window is on the MacBook (x ≥ 0) while the monitor is left of the origin.
        let p = plan(&r, (82, 30), 1.0, (400, 200));
        let ((ux, uy), (uw, uh)) = newell_ui::usable_rect(full_hd_left().rect(), WINDOWED);
        assert!(p.origin.0 >= ux && i64::from(p.origin.0) + 656 <= i64::from(ux) + i64::from(uw));
        assert!(p.origin.1 >= uy && i64::from(p.origin.1) + 540 <= i64::from(uy) + i64::from(uh));
        assert!(
            p.origin.0 < 0 && p.origin.1 < 540,
            "negative origins are respected"
        );
    }

    #[test]
    fn rect_in_window_space_converts_between_scale_factors() {
        // Equal factors: identity.
        assert_eq!(rect_in_window_space(&macbook(), 2.0), macbook().rect());
        // A 2× window and a 1× monitor at 1280 points: the monitor starts at 2560 window px.
        assert_eq!(
            rect_in_window_space(&full_hd_right(), 2.0),
            ((2560, 0), (3840, 2160))
        );
        // A 1× window and a 2× monitor: halved.
        assert_eq!(rect_in_window_space(&macbook(), 1.0), ((0, 0), (1280, 832)));
        // Garbage factors fall back to the monitor's own rectangle.
        assert_eq!(rect_in_window_space(&macbook(), 0.0), macbook().rect());
        assert_eq!(rect_in_window_space(&macbook(), f64::NAN), macbook().rect());
    }

    #[test]
    fn plan_in_a_hidpi_mismatch_stays_inside_the_monitor_in_window_pixels() {
        // The window still reports scale 2 while it is already on the external 1× monitor.
        let mut m = ScaleManager::default();
        m.start(5);
        let r = m.evaluate(Some(full_hd_right()), WINDOWED).unwrap();
        let p = plan(&r, (82, 30), 2.0, (2700, 100));
        let ((ux, _), (uw, _)) =
            newell_ui::usable_rect(rect_in_window_space(&full_hd_right(), 2.0), WINDOWED);
        assert!(
            p.origin.0 >= ux
                && i64::from(p.origin.0) + i64::from(p.size.0) <= i64::from(ux) + i64::from(uw)
        );
    }

    #[test]
    fn plan_in_fullscreen_keeps_size_and_origin() {
        let mut m = ScaleManager::default();
        m.start(2);
        let _ = m.evaluate(Some(full_hd_right()), WINDOWED);
        let r = m.evaluate(Some(full_hd_right()), ScaleMode::Fullscreen);
        // 100 % is the same k in both modes, so only the mode input changed.
        let p = plan_rescale(&r.unwrap(), (82, 30), 1.0, (1280, 0), (1920, 1080)).unwrap();
        assert_eq!((p.size, p.origin), ((1920, 1080), (1280, 0)));
    }

    #[test]
    fn plan_minimum_and_maximum_window_dimensions() {
        let mut m = ScaleManager::default();
        m.start(5);
        let r = m.evaluate(Some(macbook()), WINDOWED).unwrap();
        let min = plan(&r, (1, 1), 2.0, (0, 0));
        assert_eq!(min.size, (1312, 1080));
        let max = plan(&r, (10_000, 10_000), 2.0, (0, 0));
        assert!(max.size.0 <= 2560 && max.size.1 <= 1664 - 104);
        assert_eq!(max.size.0 % 16, 0);
        assert_eq!(max.size.1 % 36, 0);
    }

    #[test]
    fn plan_when_nothing_fits_keeps_82_by_30_at_k2() {
        let tiny = MonitorInfo::new(None, (0, 0), (640, 480), 1.0);
        let mut m = ScaleManager::default();
        m.start(5);
        let r = m.evaluate(Some(tiny), WINDOWED).unwrap();
        let p = plan(&r, (82, 30), 1.0, (0, 0));
        assert_eq!(p.size, (656, 540));
        assert!(!p.fits);
    }
}
