// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Pure frame pacing. No winit.

use std::num::NonZeroU32;
use std::time::{Duration, Instant};

/// Fixed-interval deadline tracker. The first frame is due immediately.
#[derive(Debug, Clone, Copy)]
pub struct FramePacer {
    interval: Duration,
    deadline: Instant,
}

impl FramePacer {
    /// `NonZeroU32` makes division by zero unrepresentable.
    pub fn new(target_fps: NonZeroU32, now: Instant) -> Self {
        Self {
            interval: Duration::from_secs(1) / target_fps.get(),
            deadline: now,
        }
    }

    /// Time between frames.
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// When the next frame is due.
    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    /// True once `now` has reached the deadline.
    pub fn is_due(&self, now: Instant) -> bool {
        now >= self.deadline
    }

    /// Call after a frame is rendered at `now`.
    pub fn on_frame(&mut self, now: Instant) {
        self.deadline = advance(self.deadline, now, self.interval);
    }
}

/// Smallest `prev + k * interval` (k >= 1) strictly greater than `now`.
/// Missed slots are skipped, not replayed, so a stall never produces a burst.
/// If `now < prev` the frame was early: returns `prev` unchanged (still due).
pub fn advance(prev: Instant, now: Instant, interval: Duration) -> Instant {
    if now < prev {
        return prev;
    }
    let interval_ns = interval.as_nanos().max(1);
    let elapsed_ns = now.saturating_duration_since(prev).as_nanos();
    let offset_ns = (elapsed_ns / interval_ns + 1) * interval_ns;
    u64::try_from(offset_ns)
        .ok()
        .and_then(|ns| prev.checked_add(Duration::from_nanos(ns)))
        .unwrap_or(now)
}
