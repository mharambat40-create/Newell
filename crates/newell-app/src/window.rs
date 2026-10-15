// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Frameless-window state machines that need no window (M2.17, `ARCHITECTURE.md` §3.1.1).
//!
//! [`ResizeManager`] remembers whether the OS can resize natively and owns the
//! application-managed resize session: a snapshot taken at the press from which every target
//! is re-derived. It holds no `winit` types, so each rule is testable; the `winit` calls live
//! in `app.rs`.

use newell_ui::{GridMetrics, ResizeSnapshot, ResizeTarget, WindowAction, resize_target};

/// Largest window dimension requested by a managed resize: the default `wgpu` limit for a 2D
/// texture, which is the largest surface the renderer is guaranteed to configure.
pub const SURFACE_LIMIT_PX: u32 = 8192;

/// Why a managed resize ended without a normal release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelReason {
    FocusLost,
    ScaleFactorChanged,
    CloseRequested,
    /// The window became maximised or fullscreen.
    StateChanged,
    Escape,
    /// `outer_position()` failed or the pointer was not a finite number.
    InvalidPosition,
}

#[derive(Debug, Clone, Copy)]
struct Session {
    snapshot: ResizeSnapshot,
    /// The last target requested from the OS: an unchanged snapped target sends nothing.
    last: Option<ResizeTarget>,
}

/// Native-resize capability memory and the managed resize session of one window.
#[derive(Debug, Default)]
pub struct ResizeManager {
    native_unsupported: bool,
    session: Option<Session>,
}

impl ResizeManager {
    /// `true` once `drag_resize_window` has returned `NotSupported`.
    #[must_use]
    pub const fn native_unsupported(&self) -> bool {
        self.native_unsupported
    }

    /// Remembers that native resizing is unsupported. Returns `true` only the first time, so the
    /// caller logs it once per window.
    pub fn note_native_unsupported(&mut self) -> bool {
        !std::mem::replace(&mut self.native_unsupported, true)
    }

    /// Starts a managed resize from `snapshot`, replacing any session.
    pub fn begin(&mut self, snapshot: ResizeSnapshot) {
        self.session = Some(Session {
            snapshot,
            last: None,
        });
    }

    /// `true` while a managed resize owns the pointer.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.session.is_some()
    }

    /// The pointer is at screen position `pointer`. Returns the target to request, or `None` if
    /// no resize is active or the snapped target is the one already requested (or the current
    /// window rectangle). The target is a pure function of the snapshot and `pointer`.
    pub fn update(&mut self, pointer: (i64, i64), metrics: GridMetrics) -> Option<ResizeTarget> {
        let session = self.session.as_mut()?;
        let target = resize_target(
            &session.snapshot,
            pointer,
            metrics,
            (SURFACE_LIMIT_PX, SURFACE_LIMIT_PX),
        );
        let current = ResizeTarget {
            origin: session.snapshot.origin,
            size: session.snapshot.size,
        };
        let unchanged = match session.last {
            Some(last) => last == target,
            None => current == target,
        };
        if unchanged {
            return None;
        }
        session.last = Some(target);
        Some(target)
    }

    /// The primary button was released. Returns `true` if a resize ended: the release is
    /// consumed and must not reach any control.
    pub fn release(&mut self) -> bool {
        self.session.take().is_some()
    }

    /// Ends the resize without a release; no further request is sent. Returns `true` if one was
    /// active.
    pub fn cancel(&mut self, _reason: CancelReason) -> bool {
        self.session.take().is_some()
    }
}

/// A window control pressed but not yet released.
///
/// A control acts on the release, over the same control. `winit` on macOS toggles the style
/// mask inside `set_maximized` and `is_maximized`; doing that while the press is being handled
/// swallows the matching mouse-up, so no window call may happen on the press itself.
#[derive(Debug, Default)]
pub struct ControlArm(Option<WindowAction>);

impl ControlArm {
    /// A press landed on `action`'s control.
    pub fn arm(&mut self, action: WindowAction) {
        self.0 = Some(action);
    }

    /// The button was released over `under` (the control below the pointer, if any). Returns the
    /// action to run: only when it is the control that was pressed.
    pub fn release(&mut self, under: Option<WindowAction>) -> Option<WindowAction> {
        let armed = self.0.take()?;
        (under == Some(armed)).then_some(armed)
    }

    /// Forgets the press (focus loss, a new press, close).
    pub fn cancel(&mut self) {
        self.0 = None;
    }

    /// `true` while a control is pressed.
    #[must_use]
    pub const fn is_armed(&self) -> bool {
        self.0.is_some()
    }
}

/// The pointer in screen space: window origin plus the window-relative pointer, floored.
/// `None` for a non-finite pointer. Float-to-integer `as` saturates, so extremes cannot wrap.
#[must_use]
pub fn screen_pointer(origin: (i32, i32), pointer: (f64, f64)) -> Option<(i64, i64)> {
    (pointer.0.is_finite() && pointer.1.is_finite()).then(|| {
        (
            i64::from(origin.0).saturating_add(pointer.0.floor() as i64),
            i64::from(origin.1).saturating_add(pointer.1.floor() as i64),
        )
    })
}

/// What the read-back of a new window found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FitReport {
    /// The inner size is below the minimum.
    pub below_minimum: bool,
    /// The outer rectangle is not inside the monitor rectangle.
    pub exceeds_monitor: bool,
}

/// A rectangle `(origin, size)` in physical screen pixels.
pub type Rect = ((i32, i32), (u32, u32));

/// Compares the actual window with the minimum and the monitor. Nothing is assumed from the
/// request: the caller passes what `inner_size`, `outer_size`, `outer_position` and
/// `current_monitor` returned. An unknown monitor cannot be exceeded.
#[must_use]
pub fn check_fit(
    inner: (u32, u32),
    min: (u32, u32),
    outer: Rect,
    monitor: Option<Rect>,
) -> FitReport {
    let edge = |origin: i32, size: u32| i64::from(origin) + i64::from(size);
    FitReport {
        below_minimum: inner.0 < min.0 || inner.1 < min.1,
        exceeds_monitor: monitor.is_some_and(|m| {
            outer.0.0 < m.0.0
                || outer.0.1 < m.0.1
                || edge(outer.0.0, outer.1.0) > edge(m.0.0, m.1.0)
                || edge(outer.0.1, outer.1.1) > edge(m.0.1, m.1.1)
        }),
    }
}
