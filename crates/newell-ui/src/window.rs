// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Frameless-window arithmetic (M2.17): minimum size, start-up scale, resize zones and the
//! application-managed resize target (`ARCHITECTURE.md` §3.1.1, `README_UI.md` §25.1).
//!
//! Everything here is pure integer arithmetic on physical pixels. There are no `winit` types
//! and no cell scans; the caller owns the window and the small state machine.

use crate::{GridMetrics, UiPoint};

/// Minimum grid: 82 columns.
pub const MIN_COLS: u32 = 82;
/// Minimum grid: 30 rows.
pub const MIN_ROWS: u32 = 30;
/// Smallest and largest UI scale `k` (`ARCHITECTURE.md` §3.1.1).
pub const MIN_K: u32 = 2;
pub const MAX_K: u32 = 7;
/// Minimum window width per unit of `k`: `82 * 4`.
const WIDTH_PER_K: u32 = 328;
/// Minimum window height per unit of `k`: `30 * 9`.
const HEIGHT_PER_K: u32 = 270;

/// Minimum inner window size in physical pixels for `metrics`: `82·W × 30·H`.
#[must_use]
pub const fn min_window_size(metrics: GridMetrics) -> (u32, u32) {
    (
        MIN_COLS.saturating_mul(metrics.cell_width_px()),
        MIN_ROWS.saturating_mul(metrics.cell_height_px()),
    )
}

/// The cell grid composed for a surface: whole cells, never below 82 × 30. A smaller surface
/// (asynchronous resize, rounding, an OS that ignores the minimum) is cropped, not re-laid out.
#[must_use]
pub const fn composed_grid_size(metrics: GridMetrics, width_px: u32, height_px: u32) -> (u32, u32) {
    let (cols, rows) = metrics.grid_size(width_px, height_px);
    (
        if cols < MIN_COLS { MIN_COLS } else { cols },
        if rows < MIN_ROWS { MIN_ROWS } else { rows },
    )
}

/// The five UI Scale preference values in percent (`ARCHITECTURE.md` §3.1.1).
pub const SCALE_PERCENTS: [u32; 5] = [60, 80, 100, 120, 140];
/// The default preference, 100 %.
pub const DEFAULT_SCALE_PERCENT: u32 = 100;
/// The largest monitor reference scale: `k = 5` (20 × 45 px) is the design reference size.
pub const MAX_REFERENCE_K: u32 = 5;

/// Whether the window is windowed (including maximised) or fullscreen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleMode {
    Windowed,
    Fullscreen,
}

/// A screen rectangle `(origin, size)` in physical pixels.
pub type ScreenRect = ((i32, i32), (u32, u32));

/// Monitor height left to the window: `h - h/16` windowed (the OS menu bar is estimated, never
/// measured), all of `h` fullscreen.
#[must_use]
pub const fn available_height(monitor_height: u32, mode: ScaleMode) -> u32 {
    match mode {
        ScaleMode::Windowed => monitor_height - monitor_height / 16,
        ScaleMode::Fullscreen => monitor_height,
    }
}

/// The largest `k` whose 82 × 30 grid fits the monitor (`mon_w` × `mon_h` physical pixels):
/// `min(⌊w/328⌋, ⌊available_height/270⌋)`. Unclamped: it can be below 2 or above 7.
#[must_use]
pub const fn k_fit(monitor: (u32, u32), mode: ScaleMode) -> u32 {
    let by_width = monitor.0 / WIDTH_PER_K;
    let by_height = available_height(monitor.1, mode) / HEIGHT_PER_K;
    if by_width < by_height {
        by_width
    } else {
        by_height
    }
}

/// The `k` that means "100 %" on this monitor: a compact monitor (`k_fit(windowed) <= 3`, e.g.
/// Full HD) uses one step below its fit, at least 2; any other uses `min(k_fit, 5)`. It is taken
/// from the windowed fit in both modes so fullscreen does not change what 100 % means. Pixels
/// only: no monitor name is read.
#[must_use]
pub fn monitor_reference_scale(monitor: (u32, u32)) -> u32 {
    let fit = k_fit(monitor, ScaleMode::Windowed);
    if fit <= 3 {
        fit.saturating_sub(1).max(MIN_K)
    } else {
        fit.min(MAX_REFERENCE_K)
    }
}

/// The derived scale of one (monitor, mode, preference) tuple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaleDecision {
    /// `effective_k`, 2…7.
    pub k: u32,
    /// `monitor_reference_scale`, 2…5.
    pub reference: u32,
    /// `k_req`: the rounded request before clamping.
    pub requested: u32,
    /// `false` if even `k = 2` does not fit the monitor (`k_fit < 2`).
    pub fits: bool,
}

/// `effective_k` from the monitor's physical size, the mode and the preference in percent
/// (clamped to 60…140). `None` or a zero size is an unknown monitor: reference 5, cap 7.
/// `k_req = (reference · percent + 50) / 100` (round half up; no tie exists because every
/// preference is a multiple of 20), `k = clamp(k_req, 2, clamp(k_fit, 2, 7))`.
#[must_use]
pub fn effective_scale(
    monitor: Option<(u32, u32)>,
    mode: ScaleMode,
    percent: u32,
) -> ScaleDecision {
    let percent = percent.clamp(SCALE_PERCENTS[0], SCALE_PERCENTS[4]);
    let known = monitor.filter(|&(w, h)| w > 0 && h > 0);
    let (reference, cap, fits) = match known {
        Some(size) => {
            let fit = k_fit(size, mode);
            (
                monitor_reference_scale(size),
                fit.clamp(MIN_K, MAX_K),
                fit >= MIN_K,
            )
        }
        None => (MAX_REFERENCE_K, MAX_K, true),
    };
    let requested = (reference * percent + 50) / 100;
    ScaleDecision {
        k: requested.clamp(MIN_K, cap),
        reference,
        requested,
        fits,
    }
}

/// The monitor rectangle usable by a window: the monitor minus `h/16` at the top edge in
/// windowed mode (estimate of the menu bar), the whole monitor in fullscreen.
#[must_use]
pub fn usable_rect(monitor: ScreenRect, mode: ScaleMode) -> ScreenRect {
    let ((x, y), (w, h)) = monitor;
    let reserve = h - available_height(h, mode);
    ((x, y.saturating_add_unsigned(reserve)), (w, h - reserve))
}

/// The window rectangle chosen for a monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowFit {
    /// Inner size in whole cells of the metrics.
    pub size: (u32, u32),
    pub origin: (i32, i32),
    /// `false` if the 82 × 30 minimum does not fit the usable rectangle (it overflows).
    pub fits: bool,
}

/// Safe window rectangle for `metrics` on `monitor` (`ARCHITECTURE.md` §3.1.1 *Safe window
/// fit*): the desired grid clamped to `[82, ⌊w/W⌋] × [30, ⌊usable_h/H⌋]` whole cells, then the
/// position shifted the minimum distance so the whole rectangle lies in the usable rectangle.
/// When the minimum does not fit, the top-left corner of the usable rectangle is used.
/// Pure integer arithmetic; extremes saturate.
#[must_use]
pub fn fit_window(
    metrics: GridMetrics,
    monitor: ScreenRect,
    mode: ScaleMode,
    desired_grid: (u32, u32),
    origin: (i32, i32),
) -> WindowFit {
    let (cell_w, cell_h) = (metrics.cell_width_px(), metrics.cell_height_px());
    let ((ux, uy), (uw, uh)) = usable_rect(monitor, mode);
    let max_cols = (uw / cell_w).max(MIN_COLS);
    let max_rows = (uh / cell_h).max(MIN_ROWS);
    let (cols, rows) = composed_grid_size(
        metrics,
        desired_grid
            .0
            .clamp(MIN_COLS, max_cols)
            .saturating_mul(cell_w),
        desired_grid
            .1
            .clamp(MIN_ROWS, max_rows)
            .saturating_mul(cell_h),
    );
    let size = (cols.saturating_mul(cell_w), rows.saturating_mul(cell_h));
    let place = |pos: i32, start: i32, room: u32, extent: u32| -> i32 {
        let lo = i64::from(start);
        let hi = lo + i64::from(room) - i64::from(extent);
        let clamped = if hi < lo {
            lo
        } else {
            i64::from(pos).clamp(lo, hi)
        };
        i32::try_from(clamped).unwrap_or(start)
    };
    WindowFit {
        size,
        origin: (
            place(origin.0, ux, uw, size.0),
            place(origin.1, uy, uh, size.1),
        ),
        fits: size.0 <= uw && size.1 <= uh,
    }
}

/// A window-control action (`[-] [□] [X]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowAction {
    Minimize,
    ToggleMaximize,
    Close,
}

/// An edge or corner of the resize ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeZone {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

impl ResizeZone {
    const fn west(self) -> bool {
        matches!(self, Self::West | Self::NorthWest | Self::SouthWest)
    }
    const fn east(self) -> bool {
        matches!(self, Self::East | Self::NorthEast | Self::SouthEast)
    }
    const fn north(self) -> bool {
        matches!(self, Self::North | Self::NorthWest | Self::NorthEast)
    }
    const fn south(self) -> bool {
        matches!(self, Self::South | Self::SouthWest | Self::SouthEast)
    }
}

/// The resize zone under `point` on a `surface` of `(width, height)` physical pixels: west
/// `x < W`, east `x >= sw - W`, north `y < H`, south `y >= sh - H`; a corner is one cell and
/// the leftover pixels past the last whole cell belong to east/south. Integer comparisons
/// only. A point outside the surface, or an empty surface, has no zone. Whether the cell under
/// the point is a control is the caller's check (a control always wins).
#[must_use]
pub const fn resize_zone(
    point: UiPoint,
    surface: (u32, u32),
    metrics: GridMetrics,
) -> Option<ResizeZone> {
    let (sw, sh) = surface;
    if point.x >= sw || point.y >= sh {
        return None;
    }
    let (w, h) = (metrics.cell_width_px(), metrics.cell_height_px());
    let west = point.x < w;
    let east = !west && point.x >= sw.saturating_sub(w);
    let north = point.y < h;
    let south = !north && point.y >= sh.saturating_sub(h);
    match (west, east, north, south) {
        (true, _, true, _) => Some(ResizeZone::NorthWest),
        (true, _, _, true) => Some(ResizeZone::SouthWest),
        (true, _, _, _) => Some(ResizeZone::West),
        (_, true, true, _) => Some(ResizeZone::NorthEast),
        (_, true, _, true) => Some(ResizeZone::SouthEast),
        (_, true, _, _) => Some(ResizeZone::East),
        (_, _, true, _) => Some(ResizeZone::North),
        (_, _, _, true) => Some(ResizeZone::South),
        _ => None,
    }
}

/// Window state captured when a managed resize starts. All values are physical pixels in
/// screen space; `grab` is the pointer position (`origin + pointer`) at the press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResizeSnapshot {
    pub origin: (i32, i32),
    pub size: (u32, u32),
    pub zone: ResizeZone,
    pub grab: (i64, i64),
}

/// The requested window rectangle: inner size and outer origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResizeTarget {
    pub origin: (i32, i32),
    pub size: (u32, u32),
}

/// The target for the pointer at screen position `pointer`, derived from `snapshot` only (a pure
/// function: nothing accumulates). The size is rounded to whole cells and clamped to
/// `[82·W × 30·H, max]`; a west/north edge moves the origin so the opposite edge stays fixed,
/// computed from the clamped size, so the window stops at the minimum instead of sliding.
/// Extreme values saturate. An axis the zone does not touch is unchanged.
#[must_use]
pub fn resize_target(
    snapshot: &ResizeSnapshot,
    pointer: (i64, i64),
    metrics: GridMetrics,
    max: (u32, u32),
) -> ResizeTarget {
    let zone = snapshot.zone;
    let dx = pointer.0.saturating_sub(snapshot.grab.0);
    let dy = pointer.1.saturating_sub(snapshot.grab.1);
    let (x, w) = axis(
        snapshot.origin.0,
        snapshot.size.0,
        dx,
        zone.east(),
        zone.west(),
        (metrics.cell_width_px(), MIN_COLS, max.0),
    );
    let (y, h) = axis(
        snapshot.origin.1,
        snapshot.size.1,
        dy,
        zone.south(),
        zone.north(),
        (metrics.cell_height_px(), MIN_ROWS, max.1),
    );
    ResizeTarget {
        origin: (x, y),
        size: (w, h),
    }
}

/// One axis: `positive` grows with `delta` (east/south), `negative` shrinks with it and moves
/// the origin (west/north). `cell` is `(cell px, min cells, max px)`.
fn axis(
    origin: i32,
    size: u32,
    delta: i64,
    positive: bool,
    negative: bool,
    (cell, min_cells, max_px): (u32, u32, u32),
) -> (i32, u32) {
    let change = if positive {
        delta
    } else if negative {
        0i64.saturating_sub(delta)
    } else {
        return (origin, size);
    };
    let cell = i64::from(cell);
    let raw = i64::from(size).saturating_add(change);
    let max_cells = (i64::from(max_px) / cell).max(i64::from(min_cells));
    let cells = (raw.saturating_add(cell / 2))
        .div_euclid(cell)
        .clamp(i64::from(min_cells), max_cells);
    let snapped = cells * cell;
    let new_origin = if negative {
        i64::from(origin) + (i64::from(size) - snapped)
    } else {
        i64::from(origin)
    };
    (
        i32::try_from(new_origin.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap_or(origin),
        u32::try_from(snapped).unwrap_or(size),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(k: u32) -> GridMetrics {
        GridMetrics::from_k(k).unwrap()
    }

    fn pt(x: u32, y: u32) -> UiPoint {
        UiPoint { x, y }
    }

    // ---- minimum size ----

    #[test]
    fn minimum_size_is_328k_by_270k() {
        for k in MIN_K..=MAX_K {
            assert_eq!(min_window_size(m(k)), (328 * k, 270 * k), "k = {k}");
        }
    }

    #[test]
    fn composed_grid_never_below_82_by_30() {
        let metrics = m(5);
        let (w, h) = min_window_size(metrics);
        assert_eq!(composed_grid_size(metrics, w, h), (82, 30));
        assert_eq!(composed_grid_size(metrics, w - 1, h - 1), (82, 30));
        assert_eq!(composed_grid_size(metrics, 0, 0), (82, 30));
        assert_eq!(composed_grid_size(metrics, w + 20, h + 45), (83, 31));
        assert_eq!(composed_grid_size(metrics, u32::MAX, 0).1, 30);
    }

    // ---- automatic per-monitor scale (ARCHITECTURE.md §3.1.1 example table) ----

    const WINDOWED: ScaleMode = ScaleMode::Windowed;
    const FULLSCREEN: ScaleMode = ScaleMode::Fullscreen;

    fn row(monitor: (u32, u32), mode: ScaleMode) -> [u32; 5] {
        SCALE_PERCENTS.map(|p| effective_scale(Some(monitor), mode, p).k)
    }

    #[test]
    fn example_table_windowed_and_fullscreen() {
        type Row = ((u32, u32), [u32; 5], [u32; 5]);
        let table: [Row; 7] = [
            ((1366, 768), [2, 2, 2, 2, 2], [2, 2, 2, 2, 2]),
            ((1920, 1080), [2, 2, 2, 2, 3], [2, 2, 2, 2, 3]),
            ((1920, 1200), [2, 3, 4, 4, 4], [2, 3, 4, 4, 4]),
            ((2560, 1440), [3, 4, 5, 5, 5], [3, 4, 5, 5, 5]),
            ((2560, 1664), [3, 4, 5, 5, 5], [3, 4, 5, 6, 6]),
            ((3024, 1964), [3, 4, 5, 6, 6], [3, 4, 5, 6, 7]),
            ((3840, 2160), [3, 4, 5, 6, 7], [3, 4, 5, 6, 7]),
        ];
        for (monitor, windowed, fullscreen) in table {
            assert_eq!(row(monitor, WINDOWED), windowed, "{monitor:?} windowed");
            assert_eq!(
                row(monitor, FULLSCREEN),
                fullscreen,
                "{monitor:?} fullscreen"
            );
        }
    }

    #[test]
    fn worked_cases_macbook_and_full_hd() {
        let macbook = effective_scale(Some((2560, 1664)), WINDOWED, 100);
        let full_hd = effective_scale(Some((1920, 1080)), WINDOWED, 100);
        assert_eq!((macbook.k, macbook.reference), (5, 5));
        assert_eq!((full_hd.k, full_hd.reference), (2, 2));
        let (m5, m2) = (m(macbook.k), m(full_hd.k));
        assert_eq!((m5.cell_width_px(), m5.cell_height_px()), (20, 45));
        assert_eq!((m2.cell_width_px(), m2.cell_height_px()), (8, 18));
        assert_eq!(min_window_size(m2), (656, 540));
        assert_eq!(min_window_size(m5), (1640, 1350));
        // A Retina panel reporting the scaled resolution 2940×1912 also gives 100 % = k 5.
        assert_eq!(effective_scale(Some((2940, 1912)), WINDOWED, 100).k, 5);
    }

    #[test]
    fn effective_k_is_bounded_monotone_and_below_the_cap() {
        for w in (600..4500).step_by(97) {
            for h in (500..2600).step_by(89) {
                let mut previous = 0;
                for percent in SCALE_PERCENTS {
                    for mode in [WINDOWED, FULLSCREEN] {
                        let d = effective_scale(Some((w, h)), mode, percent);
                        assert!((MIN_K..=MAX_K).contains(&d.k));
                        assert!(d.k <= k_fit((w, h), mode).clamp(MIN_K, MAX_K));
                        assert!((MIN_K..=MAX_REFERENCE_K).contains(&d.reference));
                        assert_eq!(d.fits, k_fit((w, h), mode) >= MIN_K);
                    }
                    let k = effective_scale(Some((w, h)), WINDOWED, percent).k;
                    assert!(k >= previous, "monotone in the percentage");
                    previous = k;
                }
            }
        }
    }

    #[test]
    fn rounding_has_no_ties() {
        for reference in MIN_K..=MAX_REFERENCE_K {
            for percent in SCALE_PERCENTS {
                assert_ne!(reference * percent % 100, 50, "{reference} × {percent}");
            }
        }
        // Round half up on the values that are not exact multiples.
        assert_eq!(
            effective_scale(Some((1920, 1080)), FULLSCREEN, 120).requested,
            2
        );
        assert_eq!(
            effective_scale(Some((1920, 1080)), WINDOWED, 140).requested,
            3
        );
        // Reference 2 requests 1.2 and 1.6 at 60 / 80 %: both are raised to 2.
        assert_eq!(
            effective_scale(Some((1920, 1080)), WINDOWED, 60).requested,
            1
        );
        assert_eq!(
            effective_scale(Some((1920, 1080)), WINDOWED, 80).requested,
            2
        );
    }

    #[test]
    fn reference_is_two_for_compact_monitors_and_capped_at_five() {
        // k_fit(windowed) <= 3 -> max(k_fit - 1, 2); above -> min(k_fit, 5). Pixels only.
        for (monitor, expected) in [
            ((640, 480), 2),
            ((1366, 768), 2),
            ((1920, 1080), 2),
            ((1920, 1200), 4),
            ((2560, 1440), 5),
            ((2560, 1664), 5),
            ((2940, 1912), 5),
            ((3024, 1964), 5),
            ((3840, 2160), 5),
        ] {
            assert_eq!(monitor_reference_scale(monitor), expected, "{monitor:?}");
        }
        for w in (300..4500).step_by(61) {
            for h in (300..2600).step_by(53) {
                let fit = k_fit((w, h), WINDOWED);
                let reference = monitor_reference_scale((w, h));
                assert!((2..=5).contains(&reference));
                assert_ne!(reference, 3, "3 is never a reference: {w}×{h}");
                if fit <= 3 {
                    assert_eq!(reference, fit.saturating_sub(1).max(2));
                } else {
                    assert_eq!(reference, fit.min(5));
                }
            }
        }
    }

    #[test]
    fn full_hd_and_retina_regression_at_100_percent() {
        // Full HD: k = 2, cells 8 × 18. Retina: k = 5, cells 20 × 45. Both keep 4:9, 82 × 30.
        for (monitor, k, cell) in [((1920, 1080), 2, (8, 18)), ((2560, 1664), 5, (20, 45))] {
            let d = effective_scale(Some(monitor), WINDOWED, DEFAULT_SCALE_PERCENT);
            assert_eq!(d.k, k, "{monitor:?}");
            let metrics = m(d.k);
            assert_eq!((metrics.cell_width_px(), metrics.cell_height_px()), cell);
            assert_eq!(metrics.cell_width_px() * 9, metrics.cell_height_px() * 4);
            let (mw, mh) = min_window_size(metrics);
            assert_eq!((mw, mh), (82 * cell.0, 30 * cell.1));
            assert_eq!(composed_grid_size(metrics, mw, mh), (82, 30));
            // Hit-test round trip on these metrics: the cell centre maps back to its cell.
            let grid = crate::CellGrid::new(82, 30).unwrap();
            for (col, row) in [(0, 0), (81, 29), (40, 15)] {
                let p = pt(col * cell.0 + cell.0 / 2, row * cell.1 + cell.1 / 2);
                let hit = crate::hit_test(p, metrics, &grid).unwrap();
                assert_eq!((hit.col, hit.row), (col, row));
            }
        }
    }

    #[test]
    fn full_hd_k2_window_fits_and_is_restored_through_the_fit() {
        let monitor = ((0, 0), (1920, 1080));
        let fit = fit_window(m(2), monitor, WINDOWED, (82, 30), (5000, 5000));
        assert_eq!(fit.size, (656, 540));
        assert!(fit.fits);
        assert_eq!(fit.origin, (1920 - 656, 1080 - 540));
        // A larger desired grid is clamped to whole cells of the usable rectangle: 1013 / 18 = 56.
        let big = fit_window(m(2), monitor, WINDOWED, (300, 100), (0, 0));
        assert_eq!(big.size, (240 * 8, 56 * 18));
    }

    #[test]
    fn reference_does_not_change_between_modes() {
        let monitor = (1920, 1080);
        assert_eq!(
            effective_scale(Some(monitor), WINDOWED, 100).reference,
            effective_scale(Some(monitor), FULLSCREEN, 100).reference
        );
    }

    #[test]
    fn small_monitor_clamps_several_percentages_to_one_k() {
        assert_eq!(row((1920, 1080), WINDOWED), [2, 2, 2, 2, 3]);
        assert!(effective_scale(Some((1366, 768)), WINDOWED, 100).fits);
        assert!(effective_scale(Some((1920, 1080)), WINDOWED, 100).fits);
        // k = 2 does not fit a monitor below 656 × 540 usable: 82 × 30 stays, k stays 2.
        let tiny = effective_scale(Some((640, 480)), WINDOWED, 100);
        assert_eq!((tiny.k, tiny.fits), (2, false));
    }

    #[test]
    fn unknown_monitor_uses_reference_five() {
        for unknown in [None, Some((0, 1080)), Some((1920, 0))] {
            let d = effective_scale(unknown, WINDOWED, 100);
            assert_eq!((d.k, d.reference, d.fits), (5, 5, true));
        }
        assert_eq!(effective_scale(None, WINDOWED, 140).k, 7);
        assert_eq!(effective_scale(None, WINDOWED, 60).k, 3);
        assert_eq!(effective_scale(None, WINDOWED, 0).k, 3);
        assert_eq!(effective_scale(None, WINDOWED, 0).requested, 3);
        assert_eq!(effective_scale(None, WINDOWED, 9999).k, 7);
    }

    #[test]
    fn extreme_monitors_do_not_panic() {
        assert_eq!(effective_scale(Some((1, 1)), WINDOWED, 100).k, 2);
        assert_eq!(
            effective_scale(Some((u32::MAX, u32::MAX)), WINDOWED, 140).k,
            7
        );
        assert_eq!(
            effective_scale(Some((u32::MAX, u32::MAX)), FULLSCREEN, 100).k,
            5
        );
    }

    #[test]
    fn k_fit_boundaries() {
        // 1640×1350 + reserve: the 82×30 grid of k = 5 fits exactly.
        assert_eq!(k_fit((1640, 1440), WINDOWED), 5);
        assert_eq!(k_fit((1639, 1440), WINDOWED), 4);
        assert_eq!(k_fit((2560, 1440), FULLSCREEN), 5);
        assert_eq!(k_fit((1920, 1080), FULLSCREEN), 4);
    }

    // ---- window fit ----

    #[test]
    fn usable_rect_reserves_the_top_in_windowed_mode_only() {
        assert_eq!(
            usable_rect(((10, 20), (1920, 1080)), WINDOWED),
            ((10, 20 + 67), (1920, 1013))
        );
        assert_eq!(
            usable_rect(((10, 20), (1920, 1080)), FULLSCREEN),
            ((10, 20), (1920, 1080))
        );
    }

    #[test]
    fn moving_a_large_grid_to_full_hd_clamps_to_whole_cells_inside_the_monitor() {
        let monitor = ((2560, 0), (1920, 1080));
        let fit = fit_window(m(3), monitor, WINDOWED, (100, 40), (2000, 300));
        // 100 × 12 = 1200 wide fits; 40 × 27 = 1080 > usable 1013 -> 37 rows.
        assert_eq!(fit.size, (1200, 37 * 27));
        assert!(fit.fits);
        assert_eq!(
            fit.origin.0, 2560,
            "left of the monitor: moved the minimum distance"
        );
        assert_eq!(
            fit.origin.1,
            67 + 1013 - 37 * 27,
            "too low: moved up the minimum distance"
        );
        let ((ux, uy), (uw, uh)) = usable_rect(monitor, WINDOWED);
        assert!(fit.origin.0 >= ux && fit.origin.1 >= uy);
        assert!(i64::from(fit.origin.0) + i64::from(fit.size.0) <= i64::from(ux) + i64::from(uw));
        assert!(i64::from(fit.origin.1) + i64::from(fit.size.1) <= i64::from(uy) + i64::from(uh));
    }

    #[test]
    fn fit_never_goes_below_the_minimum_grid_and_restores_the_desired_grid() {
        let full_hd = ((0, 0), (1920, 1080));
        let retina = ((0, 0), (2560, 1664));
        let tiny = fit_window(m(3), full_hd, WINDOWED, (1, 1), (0, 0));
        assert_eq!(tiny.size, min_window_size(m(3)));
        let big = fit_window(m(5), retina, WINDOWED, (120, 34), (50, 200));
        assert_eq!(big.size, (2400, 1530));
        let small = fit_window(m(3), full_hd, WINDOWED, (120, 34), (50, 200));
        assert_eq!(small.size, (1440, 918));
        let back = fit_window(m(5), retina, WINDOWED, (120, 34), small.origin);
        assert_eq!(
            back.size, big.size,
            "the desired grid survives the round trip"
        );
    }

    #[test]
    fn fit_shifts_the_minimum_distance_on_every_side() {
        let monitor = ((100, 50), (1920, 1080));
        let (w, h) = (984i32, 810i32);
        let at = |x, y| fit_window(m(3), monitor, WINDOWED, (82, 30), (x, y)).origin;
        assert_eq!(at(0, 0), (100, 50 + 67));
        assert_eq!(at(5000, 5000), (100 + 1920 - w, 50 + 1080 - h));
        assert_eq!(at(300, 200), (300, 200), "already inside: unchanged");
        assert_eq!(at(i32::MIN, i32::MIN), (100, 50 + 67));
        assert_eq!(at(i32::MAX, i32::MAX), (100 + 1920 - w, 50 + 1080 - h));
    }

    #[test]
    fn fit_when_nothing_fits_uses_the_top_left_and_reports() {
        let monitor = ((0, 0), (1366, 768));
        let fit = fit_window(m(3), monitor, WINDOWED, (82, 30), (500, 500));
        assert!(!fit.fits);
        assert_eq!(fit.size, (984, 810));
        assert_eq!(
            fit.origin,
            (382, 48),
            "horizontal room exists, vertical does not"
        );
    }

    #[test]
    fn fit_works_with_negative_monitor_origins() {
        let monitor = ((-1920, -200), (1920, 1080));
        let fit = fit_window(m(3), monitor, WINDOWED, (82, 30), (-2500, 900));
        assert_eq!(fit.origin.0, -1920);
        assert!(fit.origin.1 + 810 <= -200 + 1080);
        assert!(fit.origin.1 >= -200 + 67);
    }

    // ---- resize zones ----

    #[test]
    fn eight_zones_and_interior() {
        for k in [3, 5, 7] {
            let metrics = m(k);
            let (w, h) = (metrics.cell_width_px(), metrics.cell_height_px());
            let (sw, sh) = (90 * w, 40 * h);
            let z = |x, y| resize_zone(pt(x, y), (sw, sh), metrics);
            assert_eq!(z(0, 0), Some(ResizeZone::NorthWest));
            assert_eq!(z(w - 1, h - 1), Some(ResizeZone::NorthWest));
            assert_eq!(z(w, 0), Some(ResizeZone::North));
            assert_eq!(z(sw - w - 1, h - 1), Some(ResizeZone::North));
            assert_eq!(z(sw - w, 0), Some(ResizeZone::NorthEast));
            assert_eq!(z(sw - 1, h - 1), Some(ResizeZone::NorthEast));
            assert_eq!(z(0, h), Some(ResizeZone::West));
            assert_eq!(z(w - 1, sh - h - 1), Some(ResizeZone::West));
            assert_eq!(z(sw - 1, h), Some(ResizeZone::East));
            assert_eq!(z(sw - w, sh - h - 1), Some(ResizeZone::East));
            assert_eq!(z(0, sh - h), Some(ResizeZone::SouthWest));
            assert_eq!(z(w, sh - h), Some(ResizeZone::South));
            assert_eq!(z(sw - w - 1, sh - 1), Some(ResizeZone::South));
            assert_eq!(z(sw - w, sh - h), Some(ResizeZone::SouthEast));
            assert_eq!(z(sw - 1, sh - 1), Some(ResizeZone::SouthEast));
            assert_eq!(z(w, h), None);
            assert_eq!(z(sw - w - 1, sh - h - 1), None);
        }
    }

    #[test]
    fn leftover_pixels_belong_to_east_and_south() {
        let metrics = m(5);
        // 3 px right of and 7 px below the last whole cell.
        let (sw, sh) = (82 * 20 + 3, 30 * 45 + 7);
        let z = |x, y| resize_zone(pt(x, y), (sw, sh), metrics);
        assert_eq!(z(sw - 1, 500), Some(ResizeZone::East));
        assert_eq!(z(500, sh - 1), Some(ResizeZone::South));
        // The zone is one cell wide counted from the surface edge, so the last whole cell's
        // right part is the interior.
        assert_eq!(z(sw - 20, 500), Some(ResizeZone::East));
        assert_eq!(z(sw - 21, 500), None);
        assert_eq!(z(sw - 1, sh - 1), Some(ResizeZone::SouthEast));
    }

    #[test]
    fn zones_outside_and_degenerate_surfaces() {
        let metrics = m(5);
        assert_eq!(resize_zone(pt(10, 10), (0, 0), metrics), None);
        assert_eq!(resize_zone(pt(1640, 5), (1640, 1350), metrics), None);
        assert_eq!(resize_zone(pt(5, 1350), (1640, 1350), metrics), None);
        assert_eq!(
            resize_zone(pt(u32::MAX, u32::MAX), (1640, 1350), metrics),
            None
        );
        // A surface narrower than two cells: west wins, no panic.
        assert_eq!(
            resize_zone(pt(25, 500), (30, 1350), metrics),
            Some(ResizeZone::East)
        );
        assert_eq!(
            resize_zone(pt(5, 500), (30, 1350), metrics),
            Some(ResizeZone::West)
        );
        // A surface cropped below the minimum still has a ring.
        assert_eq!(
            resize_zone(pt(0, 0), (100, 100), metrics),
            Some(ResizeZone::NorthWest)
        );
    }

    // ---- resize target ----

    const MAX: (u32, u32) = (8192, 8192);

    fn snap(zone: ResizeZone, origin: (i32, i32), size: (u32, u32)) -> ResizeSnapshot {
        ResizeSnapshot {
            origin,
            size,
            zone,
            grab: (i64::from(origin.0) + 5, i64::from(origin.1) + 5),
        }
    }

    fn at(s: &ResizeSnapshot, dx: i64, dy: i64) -> (i64, i64) {
        (s.grab.0 + dx, s.grab.1 + dy)
    }

    #[test]
    fn east_south_keep_the_origin() {
        let metrics = m(5);
        let s = snap(ResizeZone::SouthEast, (100, 200), (1640, 1350));
        let t = resize_target(&s, at(&s, 205, 100), metrics, MAX);
        assert_eq!(t.origin, (100, 200));
        // 1640 + 205 = 1845 -> round(92.25) = 92 cells; 1350 + 100 = 1450 -> 32.2 -> 32 rows.
        assert_eq!(t.size, (92 * 20, 32 * 45));
        let east = snap(ResizeZone::East, (100, 200), (1640, 1350));
        let t = resize_target(&east, at(&east, 205, 999), metrics, MAX);
        assert_eq!((t.origin, t.size), ((100, 200), (92 * 20, 1350)));
        let south = snap(ResizeZone::South, (100, 200), (1640, 1350));
        let t = resize_target(&south, at(&south, 999, 100), metrics, MAX);
        assert_eq!((t.origin, t.size), ((100, 200), (1640, 32 * 45)));
    }

    #[test]
    fn west_north_keep_the_opposite_edge() {
        let metrics = m(5);
        let s = snap(ResizeZone::NorthWest, (100, 200), (1640, 1350));
        let t = resize_target(&s, at(&s, -200, -90), metrics, MAX);
        assert_eq!(t.size, (92 * 20, 32 * 45));
        assert_eq!(t.origin.0 + t.size.0 as i32, 100 + 1640, "right edge fixed");
        assert_eq!(
            t.origin.1 + t.size.1 as i32,
            200 + 1350,
            "bottom edge fixed"
        );
        let west = snap(ResizeZone::West, (100, 200), (1640, 1350));
        let t = resize_target(&west, at(&west, -200, 50), metrics, MAX);
        assert_eq!((t.origin.1, t.size.1), (200, 1350));
        assert_eq!(t.origin.0 + t.size.0 as i32, 1740);
    }

    #[test]
    fn mixed_corners_move_only_their_axes() {
        let metrics = m(5);
        let ne = snap(ResizeZone::NorthEast, (100, 200), (1640, 1350));
        let t = resize_target(&ne, at(&ne, 200, -90), metrics, MAX);
        assert_eq!(t.origin.0, 100);
        assert_eq!(t.origin.1 + t.size.1 as i32, 1550);
        let sw = snap(ResizeZone::SouthWest, (100, 200), (1640, 1350));
        let t = resize_target(&sw, at(&sw, -200, 90), metrics, MAX);
        assert_eq!(t.origin.1, 200);
        assert_eq!(t.origin.0 + t.size.0 as i32, 1740);
    }

    #[test]
    fn size_is_whole_cells_and_stops_at_the_minimum() {
        let metrics = m(5);
        let (mw, mh) = min_window_size(metrics);
        let s = snap(ResizeZone::NorthWest, (100, 200), (1700, 1400));
        let t = resize_target(&s, at(&s, 5000, 5000), metrics, MAX);
        assert_eq!(t.size, (mw, mh));
        assert_eq!(t.origin.0 + mw as i32, 1800, "origin stops, no sliding");
        assert_eq!(t.origin.1 + mh as i32, 1600);
        for dx in -300..300 {
            let t = resize_target(&s, at(&s, dx, dx), metrics, MAX);
            assert_eq!(t.size.0 % 20, 0);
            assert_eq!(t.size.1 % 45, 0);
            assert!(t.size.0 >= mw && t.size.1 >= mh);
        }
        let se = snap(ResizeZone::SouthEast, (0, 0), (1640, 1350));
        assert_eq!(
            resize_target(&se, at(&se, -9999, -9999), metrics, MAX).size,
            (mw, mh)
        );
    }

    #[test]
    fn size_is_limited_by_the_surface_maximum() {
        let metrics = m(5);
        let s = snap(ResizeZone::SouthEast, (0, 0), (1640, 1350));
        let t = resize_target(&s, at(&s, 99999, 99999), metrics, (8192, 8192));
        assert_eq!(t.size, (409 * 20, 182 * 45));
        let nw = snap(ResizeZone::NorthWest, (0, 0), (1640, 1350));
        let t = resize_target(&nw, at(&nw, -99999, -99999), metrics, (8192, 8192));
        assert_eq!(t.size, (409 * 20, 182 * 45));
        assert_eq!(t.origin.0, 1640 - 409 * 20);
        // A maximum below the minimum cannot break the minimum.
        let t = resize_target(&s, at(&s, 99999, 99999), metrics, (10, 10));
        assert_eq!(t.size, min_window_size(metrics));
    }

    #[test]
    fn extreme_values_saturate_without_panic() {
        let metrics = m(3);
        for zone in [
            ResizeZone::North,
            ResizeZone::South,
            ResizeZone::East,
            ResizeZone::West,
            ResizeZone::NorthEast,
            ResizeZone::NorthWest,
            ResizeZone::SouthEast,
            ResizeZone::SouthWest,
        ] {
            let s = ResizeSnapshot {
                origin: (i32::MIN, i32::MAX),
                size: (u32::MAX, u32::MAX),
                zone,
                grab: (i64::MIN, i64::MAX),
            };
            for p in [
                (i64::MIN, i64::MIN),
                (i64::MAX, i64::MAX),
                (i64::MIN, i64::MAX),
                (0, 0),
            ] {
                let _ = resize_target(&s, p, metrics, (u32::MAX, u32::MAX));
                let _ = resize_target(&s, p, metrics, MAX);
            }
        }
    }

    #[test]
    fn negative_origin_is_valid() {
        let metrics = m(5);
        let s = snap(ResizeZone::West, (-1500, -40), (1640, 1350));
        let t = resize_target(&s, at(&s, -100, 0), metrics, MAX);
        assert_eq!(t.origin.0 + t.size.0 as i32, 140);
        assert_eq!(t.size.0, 87 * 20);
    }

    #[test]
    fn target_is_a_pure_function_of_snapshot_and_pointer() {
        let metrics = m(5);
        let s = snap(ResizeZone::NorthWest, (10, 10), (1640, 1350));
        let a = resize_target(&s, at(&s, -77, -33), metrics, MAX);
        // Evaluating other pointers in between changes nothing.
        let _ = resize_target(&s, at(&s, 400, 400), metrics, MAX);
        let _ = resize_target(&s, at(&s, -900, 12), metrics, MAX);
        assert_eq!(a, resize_target(&s, at(&s, -77, -33), metrics, MAX));
    }

    #[test]
    fn hidpi_physical_coordinates_are_consistent() {
        // Retina 2x: everything is in physical pixels, so k = 5 at 2x is the same arithmetic.
        let metrics = m(5);
        let s = snap(ResizeZone::East, (0, 0), (1640, 1350));
        // A 1 logical px move is 2 physical px: below half a cell, no change of size.
        assert_eq!(resize_target(&s, at(&s, 2, 0), metrics, MAX).size.0, 1640);
        // Half a cell (10 px) rounds up to a whole cell.
        assert_eq!(resize_target(&s, at(&s, 10, 0), metrics, MAX).size.0, 1660);
        assert_eq!(resize_target(&s, at(&s, 9, 0), metrics, MAX).size.0, 1640);
    }
}
