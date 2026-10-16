// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! `App`: winit `ApplicationHandler` mapping window events to `Renderer` calls.
//!
//! Continuous 60 FPS redraw is a bootstrap mechanism to validate the loop, not the final
//! redraw architecture (ARCHITECTURE.md §4.1 remains the target).

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use newell_render::{Renderer, Theme};
use newell_ui::{
    DEFAULT_SCALE_PERCENT, GridMetrics, ResizeSnapshot, ResizeZone, ScaleMode, ThemeChoice,
    WindowAction, composed_grid_size, effective_scale, fit_window, k_fit, min_window_size,
};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::error::ExternalError;
use winit::event::{ElementState, InnerSizeWriter, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::monitor::MonitorHandle;
use winit::window::{Fullscreen, ResizeDirection, Window, WindowId};

use crate::AppError;
use crate::controller::{Press, ShellController, Update};
use crate::input::{translate_key, ui_point};
use crate::pacing::FramePacer;
use crate::scale::{MonitorInfo, Rescale, ScaleManager, plan_rescale, rect_in_window_space};
use crate::window::{CancelReason, ControlArm, ResizeManager, check_fit, screen_pointer};

const TARGET_FPS: NonZeroU32 = NonZeroU32::MIN.saturating_add(59);
const TITLE: &str = "Newell";
/// A monitor change seen while the window is still being moved is applied once the window has
/// been still this long. The OS drag loop streams `Moved` events and resizing the window under
/// the user's hand fights it; the change is applied once, for the monitor where it comes to rest.
const SETTLE: Duration = Duration::from_millis(150);
/// `winit` 0.30 has no monitor-change event: the hosting monitor is re-read this often.
const MONITOR_POLL: Duration = Duration::from_millis(500);
/// Longest time frames are held back after a transition while the OS applies the new size.
const PRESENT_GATE: Duration = Duration::from_millis(250);

/// Reports a condition on stderr (the application has no logging framework yet).
fn log(message: &str) {
    eprintln!("newell: {message}");
}

const fn direction(zone: ResizeZone) -> ResizeDirection {
    match zone {
        ResizeZone::North => ResizeDirection::North,
        ResizeZone::South => ResizeDirection::South,
        ResizeZone::East => ResizeDirection::East,
        ResizeZone::West => ResizeDirection::West,
        ResizeZone::NorthEast => ResizeDirection::NorthEast,
        ResizeZone::NorthWest => ResizeDirection::NorthWest,
        ResizeZone::SouthEast => ResizeDirection::SouthEast,
        ResizeZone::SouthWest => ResizeDirection::SouthWest,
    }
}

/// A snapshot of `monitor` in its own physical pixels (`MonitorHandle::size` and `position`).
fn monitor_info(monitor: &MonitorHandle) -> MonitorInfo {
    let (position, size) = (monitor.position(), monitor.size());
    MonitorInfo::new(
        monitor_identity(monitor),
        (position.x, position.y),
        (size.width, size.height),
        monitor.scale_factor(),
    )
}

/// An identity for change detection and logs. The display id on macOS (`name()` is slow there).
#[cfg(target_os = "macos")]
fn monitor_identity(monitor: &MonitorHandle) -> Option<String> {
    use winit::platform::macos::MonitorHandleExtMacOS;
    Some(format!("display {}", monitor.native_id()))
}

#[cfg(not(target_os = "macos"))]
fn monitor_identity(monitor: &MonitorHandle) -> Option<String> {
    monitor.name()
}

/// The monitor that actually hosts `window`: its own `current_monitor`, not the primary one.
/// Only if the OS cannot say (a window not yet on screen) is the primary monitor, then the first
/// available one, used as a stand-in; the choice is re-derived by the next poll.
fn host_monitor(window: &Window) -> Option<MonitorHandle> {
    window
        .current_monitor()
        .or_else(|| window.primary_monitor())
        .or_else(|| window.available_monitors().next())
}

fn scale_mode(window: &Window) -> ScaleMode {
    if window.fullscreen().is_some() {
        ScaleMode::Fullscreen
    } else {
        ScaleMode::Windowed
    }
}

/// Frames are held back until the OS reports `size` or the deadline passes.
#[derive(Debug, Clone, Copy)]
struct PresentGate {
    size: (u32, u32),
    deadline: Instant,
}

pub(crate) struct App {
    // Field order is drop order: the renderer (surface) is dropped before the window.
    renderer: Option<Renderer>,
    window: Option<Arc<Window>>,
    shell: Option<ShellController>,
    modifiers: ModifiersState,
    pacer: FramePacer,
    error: Option<AppError>,
    resize: ResizeManager,
    /// Last non-zero physical inner size reported by the OS.
    last_size: PhysicalSize<u32>,
    /// Preference, derived `k`, last evaluated inputs and the user's grid (`ARCHITECTURE.md`
    /// §3.1.1). The only owner of `effective_k`.
    scale: ScaleManager,
    /// The last `Moved`: a monitor change is applied once the window has been still.
    moved_at: Option<Instant>,
    last_poll: Instant,
    /// The window was not at its inside-the-monitor position after a scale-factor change
    /// (applied mid-drag): fix it when the window settles.
    reposition_pending: Option<(i32, i32)>,
    gate: Option<PresentGate>,
    /// A native (OS) resize is in progress: its `Resized` events are the user's grid.
    native_resizing: bool,
    /// Identities of the connected monitors, to log connection and disconnection.
    known_monitors: Vec<String>,
    drag_failure_logged: bool,
    /// A window control pressed and waiting for its release.
    control: ControlArm,
    /// Our own maximised state. `Window::is_maximized` toggles the style mask on macOS (see
    /// [`ControlArm`]) and is only called when the control fires, never on pointer input.
    maximized: bool,
    /// The OS reported the window as fully hidden (minimised, covered): no frame is delivered.
    occluded: bool,
    /// Monitor size for which a refused fullscreen request was already logged.
    fullscreen_refusal_logged: Option<(u32, u32)>,
    /// Start of the monotonic clock handed to the controller for double-click timing.
    epoch: Instant,
}

impl App {
    pub(crate) fn new() -> Self {
        Self {
            renderer: None,
            window: None,
            shell: None,
            modifiers: ModifiersState::empty(),
            pacer: FramePacer::new(TARGET_FPS, Instant::now()),
            error: None,
            resize: ResizeManager::default(),
            last_size: PhysicalSize::new(0, 0),
            scale: ScaleManager::default(),
            moved_at: None,
            last_poll: Instant::now(),
            reposition_pending: None,
            gate: None,
            native_resizing: false,
            known_monitors: Vec::new(),
            drag_failure_logged: false,
            control: ControlArm::default(),
            maximized: false,
            occluded: false,
            fullscreen_refusal_logged: None,
            epoch: Instant::now(),
        }
    }

    pub(crate) fn into_result(self) -> Result<(), AppError> {
        self.error.map_or(Ok(()), Err)
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl Into<AppError>) {
        self.error = Some(error.into());
        event_loop.exit();
    }

    /// Carries the View menu's choices out of the shell: the theme goes to the renderer's
    /// uniform, a changed UI Scale preference to the scale manager (one transition if
    /// `effective_k` changes), and a Fullscreen request to the window.
    fn sync_view_choices(&mut self) {
        let Some(shell) = &mut self.shell else { return };
        let theme = match shell.shell().theme() {
            ThemeChoice::Light => Theme::Light,
            ThemeChoice::Dark => Theme::Dark,
        };
        let percent = shell.shell().preferred_scale_percent();
        let fullscreen = shell.take_fullscreen_request();
        if let Some(renderer) = &mut self.renderer {
            renderer.set_theme(theme);
        }
        if percent != self.scale.preferred_percent() {
            self.scale.set_preferred_percent(percent);
            // A preference whose `k` is unchanged only stores the preference.
            self.reconcile_with("preference", None, true);
        }
        if fullscreen {
            self.toggle_fullscreen();
        }
    }

    /// View > Fullscreen: leaves fullscreen, or enters it on the hosting monitor unless even
    /// `k = 2` would not fit there (refused, logged once per monitor size). The derivation of
    /// `k` and the window fit stay with the M2.17 transition, run by `reconcile`.
    fn toggle_fullscreen(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        self.cancel_resize(CancelReason::StateChanged);
        if window.fullscreen().is_some() {
            window.set_fullscreen(None);
        } else {
            let monitor = host_monitor(&window);
            let size = monitor.as_ref().map(|m| (m.size().width, m.size().height));
            if size.is_some_and(|size| k_fit(size, ScaleMode::Fullscreen) < 2) {
                if self.fullscreen_refusal_logged != size {
                    self.fullscreen_refusal_logged = size;
                    log(
                        "fullscreen refused: the monitor is smaller than the 82 x 30 minimum grid at k = 2",
                    );
                }
                return;
            }
            window.set_fullscreen(Some(Fullscreen::Borderless(monitor)));
        }
        self.reconcile("fullscreen", None);
    }

    /// Uploads what `update` says changed and asks for a redraw.
    fn apply(&mut self, update: Update) {
        self.sync_view_choices();
        if !update.any() {
            return;
        }
        if let (Some(renderer), Some(shell)) = (&mut self.renderer, &self.shell) {
            if update.grid {
                renderer.set_ui_grid(shell.grid());
            }
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn resize_to_window(&mut self) {
        if let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) {
            let size = window.inner_size();
            renderer.resize(size.width, size.height);
            self.resize_shell(size.width, size.height);
        }
    }

    /// Re-lays out the shell on the whole cells of the new window size and uploads the grid.
    fn resize_shell(&mut self, width: u32, height: u32) {
        if let Some(shell) = &mut self.shell {
            let update = shell.resize(width, height);
            self.apply(update);
        }
    }

    /// The minimum inner size of the session's metrics, if the shell exists.
    fn min_size(&self) -> Option<PhysicalSize<u32>> {
        let (w, h) = min_window_size(self.shell.as_ref()?.metrics());
        Some(PhysicalSize::new(w, h))
    }

    /// Re-applies the minimum inner size; the OS may have dropped or rescaled it.
    fn apply_min_size(&self) {
        if let (Some(window), Some(min)) = (&self.window, self.min_size()) {
            window.set_min_inner_size(Some(min));
        }
    }

    /// Reads back what the OS actually created and logs, once, a window below the minimum or
    /// outside its monitor. Neither the request nor the monitor size is assumed to be honoured.
    fn verify_window(&self, window: &Window, min: (u32, u32)) {
        let inner = window.inner_size();
        let outer_size = window.outer_size();
        let outer = window
            .outer_position()
            .ok()
            .map(|p| ((p.x, p.y), (outer_size.width, outer_size.height)));
        let monitor = host_monitor(window)
            .map(|m| rect_in_window_space(&monitor_info(&m), window.scale_factor()));
        let report = check_fit(
            (inner.width, inner.height),
            min,
            outer.unwrap_or(((0, 0), (0, 0))),
            outer.and(monitor),
        );
        if report.below_minimum {
            log(&format!(
                "the window is {}x{} px, below the {}x{} px minimum; the grid is cropped",
                inner.width, inner.height, min.0, min.1
            ));
        }
        if report.exceeds_monitor {
            log("the window is not inside its monitor; the OS placed it there");
        }
    }

    /// Logs every runtime value the scale derivation used and produced (`ARCHITECTURE.md`
    /// §3.1.1; coordinates are the ones `winit` reports, never the macOS Display Settings).
    fn log_runtime(&self, reason: &str, window: &Window, rescale: &Rescale) {
        let monitor = rescale.monitor.as_ref().map_or_else(
            || "unknown".to_owned(),
            |m| {
                format!(
                    "{} pos={:?} size={}x{} scale_factor={}",
                    m.name.as_deref().unwrap_or("?"),
                    m.position,
                    m.size.0,
                    m.size.1,
                    m.scale_factor()
                )
            },
        );
        let (inner, outer) = (window.inner_size(), window.outer_size());
        let metrics = self.shell.as_ref().map(ShellController::metrics);
        let cell = metrics.map_or((0, 0), |m| (m.cell_width_px(), m.cell_height_px()));
        // `inner`/`outer` are read before the new size is applied: the OS reports it later.
        log(&format!(
            "scale[{reason}] monitor {monitor} | window outer_pos={:?} inner={}x{} outer={}x{} scale_factor={} mode={:?} | preferred={}% reference={} requested={} effective_k={} previous_k={:?} fits={} | cell_before={}x{} desired_grid={:?}",
            window.outer_position().ok().map(|p| (p.x, p.y)),
            inner.width,
            inner.height,
            outer.width,
            outer.height,
            window.scale_factor(),
            rescale.mode,
            self.scale.preferred_percent(),
            rescale.decision.reference,
            rescale.decision.requested,
            rescale.decision.k,
            rescale.previous_k,
            rescale.decision.fits,
            cell.0,
            cell.1,
            self.scale.desired_grid(),
        ));
    }

    /// Logs the connected monitors when the set changes (connection, disconnection, a resolution
    /// or scale change of any of them).
    fn log_monitors(&mut self, window: &Window) {
        let monitors: Vec<String> = window
            .available_monitors()
            .map(|m| {
                let i = monitor_info(&m);
                format!(
                    "{} pos={:?} size={}x{} scale_factor={}",
                    i.name.as_deref().unwrap_or("?"),
                    i.position,
                    i.size.0,
                    i.size.1,
                    i.scale_factor()
                )
            })
            .collect();
        if monitors != self.known_monitors {
            log(&format!("monitors: [{}]", monitors.join("; ")));
            self.known_monitors = monitors;
        }
    }

    /// Re-derives `effective_k` for the monitor that hosts the window now, and applies the
    /// change, if any, as one transition. Idempotent: unchanged inputs do nothing. The window's
    /// own size never feeds the derivation, so the resize this causes cannot start another.
    ///
    /// `writer` is the `ScaleFactorChanged` size writer, when called from that event: the target
    /// size is then written through it. Returns `true` if the writer was used.
    fn reconcile(&mut self, reason: &str, writer: Option<&mut InnerSizeWriter>) -> bool {
        self.reconcile_with(reason, writer, false)
    }

    /// [`Self::reconcile`]; with `skip_same_k` a change that leaves `effective_k` as it is
    /// (a UI Scale preference several percentages share) records its inputs and stops: no
    /// transition runs. A window still being moved is not skipped, so the pending monitor
    /// change is applied with it.
    fn reconcile_with(
        &mut self,
        reason: &str,
        writer: Option<&mut InnerSizeWriter>,
        skip_same_k: bool,
    ) -> bool {
        let Some(window) = self.window.clone() else {
            return false;
        };
        let monitor = host_monitor(&window).map(|m| monitor_info(&m));
        let Some(rescale) = self.scale.evaluate(monitor, scale_mode(&window)) else {
            return false;
        };
        if skip_same_k && !rescale.k_changed && self.moved_at.is_none() {
            return false;
        }
        self.log_runtime(reason, &window, &rescale);
        if rescale.log_limit {
            log(
                "the display is smaller than the 82 x 30 minimum grid at the smallest scale (k = 2); keeping k = 2",
            );
        }
        match self.apply_rescale(&window, &rescale, writer) {
            Ok(used) => used,
            Err(error) => {
                // The whole transition is abandoned; the old state is intact and the same
                // inputs are not retried until one changes.
                log(&format!("scale change abandoned: {error}"));
                self.scale.revert(rescale.previous_k);
                false
            }
        }
    }

    /// The transition (`ARCHITECTURE.md` §3.1.1): everything fallible is prepared first with
    /// no live state touched; the commit then publishes metrics, atlas, grid, GPU buffers,
    /// minimum size and pointer mapping together in this one event-loop turn.
    fn apply_rescale(
        &mut self,
        window: &Window,
        rescale: &Rescale,
        writer: Option<&mut InnerSizeWriter>,
    ) -> Result<bool, AppError> {
        let inner = window.inner_size();
        let origin = window.outer_position().ok().map(|p| (p.x, p.y));
        let mut plan = plan_rescale(
            rescale,
            self.scale.desired_grid(),
            window.scale_factor(),
            origin.unwrap_or((0, 0)),
            (inner.width, inner.height),
        )?;
        // Fullscreen and maximised windows are sized by the OS.
        let os_owned = rescale.mode == ScaleMode::Fullscreen || self.maximized;
        if os_owned {
            plan.size = (inner.width, inner.height);
        }
        // Phase A: prepare.
        let prepared = if rescale.k_changed {
            let (Some(renderer), Some(shell)) = (self.renderer.as_mut(), self.shell.as_ref())
            else {
                return Ok(false);
            };
            renderer.prepare_metrics(plan.metrics)?;
            Some(shell.prepare_metrics(plan.metrics, plan.size.0, plan.size.1)?)
        } else {
            None
        };
        // Phase B: commit. Nothing below can fail.
        self.cancel_resize(CancelReason::StateChanged);
        self.control.cancel();
        if let Some(prepared) = prepared {
            if let (Some(renderer), Some(shell)) = (self.renderer.as_mut(), self.shell.as_mut()) {
                renderer.set_metrics(prepared.metrics(), prepared.grid())?;
                let _ = shell.commit_metrics(prepared);
            }
        }
        let min = min_window_size(plan.metrics);
        log(&format!(
            "scale applied: effective_k={} cell={}x{} min_window={}x{} target_size={}x{} target_origin={:?} atlases_built={} desired_grid={:?}",
            rescale.decision.k,
            plan.metrics.cell_width_px(),
            plan.metrics.cell_height_px(),
            min.0,
            min.1,
            plan.size.0,
            plan.size.1,
            plan.origin,
            self.renderer
                .as_ref()
                .map_or(0, newell_render::Renderer::atlas_builds),
            self.scale.desired_grid(),
        ));
        window.set_min_inner_size(Some(PhysicalSize::new(min.0, min.1)));
        let target = PhysicalSize::new(plan.size.0, plan.size.1);
        let mut wrote = false;
        if !os_owned {
            // Size first, then position: macOS derives the vertical flip from the frame.
            match writer {
                Some(writer) => wrote = writer.request_inner_size(target).is_ok(),
                None => {
                    let _ = window.request_inner_size(target);
                }
            }
            if wrote {
                // Mid scale-factor event (often mid-drag): place it once settled.
                self.reposition_pending = Some(plan.origin);
            } else {
                // Resizing moves the top-left corner on macOS (the frame is anchored at the
                // bottom-left), so compare with the position read back after the size
                // request, not with the one read before it.
                let now = window.outer_position().ok().map(|p| (p.x, p.y));
                if origin.is_some() && now != Some(plan.origin) {
                    window.set_outer_position(PhysicalPosition::new(plan.origin.0, plan.origin.1));
                }
            }
        }
        // `last_size` is what a `ScaleFactorChanged` that finds nothing to change keeps: it must
        // be the size this transition asked for, not the size from before it.
        self.last_size = target;
        if wrote {
            // The OS reports the size after this event returns: hold frames until it does.
            self.gate = Some(PresentGate {
                size: plan.size,
                deadline: Instant::now() + PRESENT_GATE,
            });
        } else {
            self.resize_to_window();
            let actual = window.inner_size();
            if actual.width > 0 && actual.height > 0 {
                self.last_size = actual;
            }
            if !os_owned && (actual.width, actual.height) != plan.size {
                self.gate = Some(PresentGate {
                    size: plan.size,
                    deadline: Instant::now() + PRESENT_GATE,
                });
            }
            self.verify_window(window, min);
        }
        Ok(wrote)
    }

    /// Applies a deferred monitor change once the window has stopped moving, then places a
    /// window that a scale-factor change left partly off its monitor.
    fn settle(&mut self) {
        self.reconcile("settled", None);
        if let Some(origin) = self.reposition_pending.take() {
            self.reposition_inside(origin);
        }
    }

    /// One bounded correction: shifts (and if needed shrinks to whole cells) the window so it
    /// lies inside the usable rectangle of its monitor. Never loops: it changes nothing if the
    /// window already is inside.
    fn reposition_inside(&mut self, intended: (i32, i32)) {
        let (Some(window), Some(shell)) = (self.window.clone(), self.shell.as_ref()) else {
            return;
        };
        if self.maximized || window.fullscreen().is_some() {
            return;
        }
        let (Some(monitor), Ok(origin)) = (
            host_monitor(&window).map(|m| monitor_info(&m)),
            window.outer_position(),
        ) else {
            return;
        };
        let metrics = shell.metrics();
        let inner = window.inner_size();
        let rect = rect_in_window_space(&monitor, window.scale_factor());
        let grid = composed_grid_size(metrics, inner.width, inner.height);
        let fit = fit_window(metrics, rect, ScaleMode::Windowed, grid, intended);
        if fit.size != (inner.width, inner.height) {
            let _ = window.request_inner_size(PhysicalSize::new(fit.size.0, fit.size.1));
        }
        // Compare with the position read back after any size change (macOS moves the top-left
        // corner when it resizes).
        let now = window.outer_position().unwrap_or(origin);
        if fit.origin != (now.x, now.y) {
            window.set_outer_position(PhysicalPosition::new(fit.origin.0, fit.origin.1));
        }
    }

    /// `true` if the window can be resized from the grid: not maximised, not fullscreen.
    fn resizable(&self) -> bool {
        !self.maximized
            && self
                .window
                .as_ref()
                .is_some_and(|w| w.fullscreen().is_none())
    }

    fn cancel_resize(&mut self, reason: CancelReason) {
        self.resize.cancel(reason);
    }

    /// A left press: window control, resize zone, MenuBar drag or normal shell routing.
    fn left_pressed(&mut self) {
        // A press while a resize or control press is pending means its release was missed.
        self.cancel_resize(CancelReason::StateChanged);
        self.control.cancel();
        let resizable = self.resizable();
        let Some(window) = self.window.clone() else {
            return;
        };
        let size = window.inner_size();
        let Some(shell) = &mut self.shell else { return };
        shell.set_time(self.epoch.elapsed());
        match shell.press((size.width, size.height), resizable) {
            Press::Shell(update) => self.apply(update),
            Press::Window(action) => self.control.arm(action),
            Press::Drag => self.start_drag(&window),
            Press::Resize(zone) => self.start_resize(&window, zone),
        }
    }

    fn window_action(
        &mut self,
        event_loop: &ActiveEventLoop,
        window: &Window,
        action: WindowAction,
    ) {
        self.cancel_resize(CancelReason::StateChanged);
        match action {
            WindowAction::Minimize => window.set_minimized(true),
            // Fullscreen is left by its own control (M2.18), not by maximise.
            WindowAction::ToggleMaximize => {
                if window.fullscreen().is_none() {
                    let maximize = !window.is_maximized();
                    window.set_maximized(maximize);
                    self.maximized = maximize;
                }
            }
            WindowAction::Close => event_loop.exit(),
        }
    }

    fn start_drag(&mut self, window: &Window) {
        if let Err(error) = window.drag_window() {
            if !std::mem::replace(&mut self.drag_failure_logged, true) {
                log(&format!("window dragging failed: {error}"));
            }
        }
    }

    /// Native resize first; on `NotSupported` (macOS) remember it, log once and manage the
    /// resize here. `Ignored` and `Os` errors are logged and do nothing.
    /// Removes the ASCII hover target and uploads the grid if it changed (M2.25).
    fn clear_hover(&mut self) {
        let update = self
            .shell
            .as_mut()
            .map(ShellController::clear_hover)
            .unwrap_or_default();
        self.apply(update);
    }

    fn start_resize(&mut self, window: &Window, zone: ResizeZone) {
        self.clear_hover();
        if !self.resize.native_unsupported() {
            match window.drag_resize_window(direction(zone)) {
                Ok(()) => {
                    self.native_resizing = true;
                    return;
                }
                Err(ExternalError::NotSupported(_)) => {
                    if self.resize.note_native_unsupported() {
                        log(
                            "native window resizing is not supported here; using application-managed resizing",
                        );
                    }
                }
                Err(error) => return log(&format!("window resizing failed: {error}")),
            }
        }
        let (Ok(origin), Some(cursor)) = (
            window.outer_position(),
            self.shell.as_ref().and_then(ShellController::cursor),
        ) else {
            return;
        };
        let origin = (origin.x, origin.y);
        let Some(grab) = screen_pointer(origin, (f64::from(cursor.x), f64::from(cursor.y))) else {
            return;
        };
        let size = window.inner_size();
        self.resize.begin(ResizeSnapshot {
            origin,
            size: (size.width, size.height),
            zone,
            grab,
        });
    }

    /// The pointer moved during a managed resize: re-derive the target from the snapshot and
    /// request it (size first, then position) when the snapped target changed.
    fn resize_pointer(&mut self, position: PhysicalPosition<f64>) {
        let (Some(window), Some(metrics)) = (
            &self.window,
            self.shell.as_ref().map(ShellController::metrics),
        ) else {
            return;
        };
        if self.maximized || window.fullscreen().is_some() {
            return self.cancel_resize(CancelReason::StateChanged);
        }
        let Ok(origin) = window.outer_position() else {
            return self.cancel_resize(CancelReason::InvalidPosition);
        };
        let Some(pointer) = screen_pointer((origin.x, origin.y), (position.x, position.y)) else {
            return self.cancel_resize(CancelReason::InvalidPosition);
        };
        if let Some(target) = self.resize.update(pointer, metrics) {
            // The user chose this grid: it is what a later monitor change tries to keep.
            let (cols, rows) = composed_grid_size(metrics, target.size.0, target.size.1);
            self.scale.set_desired_grid(cols, rows);
            // Order matters: macOS derives the vertical flip of `set_outer_position` from the
            // current frame. The size is applied asynchronously and reported by `Resized`.
            let _ = window.request_inner_size(PhysicalSize::new(target.size.0, target.size.1));
            window.set_outer_position(PhysicalPosition::new(target.origin.0, target.origin.1));
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Can be called more than once (macOS, Android).
        if self.window.is_some() {
            return;
        }
        // The monitor that will host the window is not known before it exists (a new macOS
        // window opens on the screen of the key window, which is not necessarily the primary
        // monitor). Create the window hidden with the primary monitor's scale, ask the window
        // which monitor hosts it, then derive and apply the real scale before it is shown.
        let primary = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next())
            .map(|m| (m.size().width, m.size().height));
        let k = effective_scale(primary, ScaleMode::Windowed, DEFAULT_SCALE_PERCENT).k;
        self.scale.start(k);
        // One metrics value for the atlas, layout and hit-testing.
        let metrics = match GridMetrics::from_k(k) {
            Ok(metrics) => metrics,
            Err(error) => return self.fail(event_loop, error),
        };
        let (min_w, min_h) = min_window_size(metrics);
        let min = PhysicalSize::new(min_w, min_h);
        let attributes = Window::default_attributes()
            .with_title(TITLE)
            .with_decorations(false)
            // macOS gives a borderless `Resizable` window its own edge hotspots, which resize
            // without snapping to whole cells and take the press before `winit` sees it. The
            // grid owns every resize instead (the managed path below).
            .with_resizable(!cfg!(target_os = "macos"))
            .with_visible(false)
            .with_inner_size(min)
            .with_min_inner_size(min);
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => return self.fail(event_loop, error),
        };
        let size = window.inner_size();
        self.last_size = size;
        match Renderer::new(window.clone(), size.width, size.height, metrics) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(error) => return self.fail(event_loop, error),
        }
        let (cols, rows) = composed_grid_size(metrics, size.width, size.height);
        let shell = match ShellController::new(cols, rows, metrics) {
            Ok(shell) => shell,
            Err(error) => return self.fail(event_loop, error),
        };
        if let Some(renderer) = &mut self.renderer {
            renderer.set_ui_grid(shell.grid());
        }
        self.shell = Some(shell);
        self.window = Some(window.clone());
        self.log_monitors(&window);
        // Derive the scale of the monitor that really hosts the window, before it is shown.
        let hosted = window.current_monitor().is_some();
        if hosted {
            self.reconcile("startup", None);
        }
        window.set_visible(true);
        if !hosted {
            self.reconcile("startup (after show)", None);
        }
        self.verify_window(
            &window,
            min_window_size(
                self.shell
                    .as_ref()
                    .map_or(metrics, ShellController::metrics),
            ),
        );
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = &self.window else { return };
        if window.id() != id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                self.cancel_resize(CancelReason::CloseRequested);
                event_loop.exit();
            }
            WindowEvent::Focused(true) => {
                if self.moved_at.is_none() {
                    self.reconcile("focus", None);
                }
            }
            WindowEvent::Focused(false) => {
                self.native_resizing = false;
                self.cancel_resize(CancelReason::FocusLost);
                self.control.cancel();
                if let Some(shell) = &mut self.shell {
                    shell.pointer_released();
                }
            }
            WindowEvent::Moved(_) => {
                // A move may be the middle of a drag across monitors: the hosting monitor is
                // evaluated once the window has been still (see `SETTLE`).
                self.moved_at = Some(Instant::now());
            }
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    self.last_size = size;
                }
                if self
                    .gate
                    .is_some_and(|g| g.size == (size.width, size.height))
                {
                    self.gate = None;
                }
                if self.native_resizing {
                    // An OS-managed resize by the user: this is the grid they chose.
                    if let Some(shell) = &self.shell {
                        let (cols, rows) =
                            composed_grid_size(shell.metrics(), size.width, size.height);
                        self.scale.set_desired_grid(cols, rows);
                    }
                }
                if self.resize.is_active() && !self.resizable() {
                    self.cancel_resize(CancelReason::StateChanged);
                }
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                self.resize_shell(size.width, size.height);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                // A display resolution change may arrive as a `Resized` alone.
                if self.moved_at.is_none() && !self.resize.is_active() {
                    self.reconcile("resized", None);
                }
            }
            // A Resized may not follow; re-read the physical size.
            WindowEvent::ScaleFactorChanged {
                mut inner_size_writer,
                ..
            } => {
                self.cancel_resize(CancelReason::ScaleFactorChanged);
                // The window now sits on a monitor with another scale factor: derive its scale
                // now and write the target size through the writer, so the OS applies the new
                // cell grid instead of a logical-points conversion of the old size. The
                // position is fixed when the window settles.
                self.moved_at = Some(Instant::now());
                if !self.reconcile("scale-factor", Some(&mut inner_size_writer)) {
                    // Nothing changed: keep the physical surface size (the grid is in
                    // physical pixels) and the minimum; neither request is assumed to succeed.
                    if let Some(min) = self.min_size() {
                        let keep = PhysicalSize::new(
                            self.last_size.width.max(min.width),
                            self.last_size.height.max(min.height),
                        );
                        let _ = inner_size_writer.request_inner_size(keep);
                    }
                    self.apply_min_size();
                    self.resize_to_window();
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } if self.resize.is_active() => {
                // The shell keeps the pointer for the next press; it sees no drag and no hover.
                let update = self
                    .shell
                    .as_mut()
                    .map(|shell| {
                        shell.pointer_moved_during_resize(ui_point(position.x, position.y))
                    })
                    .unwrap_or_default();
                self.apply(update);
                self.resize_pointer(position);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let point = ui_point(position.x, position.y);
                // An OS-managed resize owns the pointer: no hover until its first move after it.
                let native = self.native_resizing;
                if let Some(shell) = &mut self.shell {
                    let update = if native {
                        shell.pointer_moved_during_resize(point)
                    } else {
                        shell.pointer_moved(point)
                    };
                    self.apply(update);
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if let Some(shell) = &mut self.shell {
                    let update = shell.pointer_moved(None);
                    self.apply(update);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.left_pressed(),
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.native_resizing = false;
                // The release that ends a managed resize is consumed.
                if !self.resize.release() {
                    let under = self
                        .shell
                        .as_ref()
                        .and_then(ShellController::window_control_at_cursor);
                    if let Some(action) = self.control.release(under) {
                        if let Some(window) = self.window.clone() {
                            self.window_action(event_loop, &window, action);
                        }
                    }
                    if let Some(shell) = &mut self.shell {
                        shell.pointer_released();
                    }
                }
            }
            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                if !occluded {
                    window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                // Escape cancels a managed resize and is not seen by the shell.
                if event.logical_key == Key::Named(NamedKey::Escape)
                    && self.resize.cancel(CancelReason::Escape)
                {
                    return;
                }
                let keys = translate_key(
                    &event.logical_key,
                    event.physical_key,
                    event.text.as_deref(),
                    self.modifiers,
                );
                if let Some(shell) = &mut self.shell {
                    let update = keys
                        .into_iter()
                        .fold(Update::default(), |acc, key| acc.merge(shell.key(key)));
                    self.apply(update);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(gate) = self.gate {
                    if Instant::now() < gate.deadline {
                        // The OS has not reported the new size yet: keep the last frame.
                        self.pacer.on_frame(Instant::now());
                        return;
                    }
                    self.gate = None;
                    let actual = window.inner_size();
                    if (actual.width, actual.height) != gate.size {
                        log(&format!(
                            "the window is {}x{} px, not the requested {}x{} px; the grid follows the actual size",
                            actual.width, actual.height, gate.size.0, gate.size.1
                        ));
                    }
                }
                if let Some(renderer) = &mut self.renderer {
                    if let Err(error) = renderer.render() {
                        return self.fail(event_loop, error);
                    }
                }
                self.pacer.on_frame(Instant::now());
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.clone() else {
            return event_loop.set_control_flow(ControlFlow::Wait);
        };
        // macOS may not deliver Resized(0, 0) on minimise, so also ask the window.
        // A hidden window gets no `RedrawRequested`, so the pacer deadline would stay in the
        // past and `WaitUntil` would spin the loop: wait for the next event (`Occluded(false)`,
        // `Resized`) instead.
        let hidden = self.occluded || window.is_minimized().unwrap_or(false);
        let now = Instant::now();
        let mut wake = None;
        if let Some(moved) = self.moved_at {
            if now.duration_since(moved) >= SETTLE {
                self.moved_at = None;
                self.settle();
            } else {
                wake = Some(moved + SETTLE);
            }
        } else if now.duration_since(self.last_poll) >= MONITOR_POLL {
            // Monitor resolution, display scale, fullscreen and connection changes arrive
            // with no event of their own.
            self.last_poll = now;
            self.reconcile("poll", None);
            if let Some(window) = self.window.clone() {
                self.log_monitors(&window);
            }
        }
        let plan = self.pacer.schedule(Instant::now(), hidden);
        if plan.redraw {
            window.request_redraw();
        }
        let wake = match (plan.wake, wake) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        event_loop.set_control_flow(wake.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
    }
}
