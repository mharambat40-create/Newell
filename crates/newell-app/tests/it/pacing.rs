// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use newell_app::pacing::{FramePacer, advance};
use proptest::prelude::*;

const FPS60: NonZeroU32 = NonZeroU32::MIN.saturating_add(59);

proptest! {
    // Covers now == prev, now < prev (returns prev, so <= now is allowed only then),
    // slot boundaries, and 10 s stalls.
    #[test]
    fn advance_properties(prev_ms in 0u64..10_000, now_ms in 0u64..10_000, interval_ms in 1u64..100) {
        let base = Instant::now();
        let prev = base + Duration::from_millis(prev_ms);
        let now = base + Duration::from_millis(now_ms);
        let interval = Duration::from_millis(interval_ms);
        let next = advance(prev, now, interval);
        if now < prev {
            prop_assert_eq!(next, prev);
        } else {
            prop_assert!(next > now, "strictly after now");
            prop_assert!(next - now <= interval, "within one interval: no burst");
            prop_assert_eq!((next - prev).as_nanos() % interval.as_nanos(), 0, "on grid");
        }
    }
}

#[test]
fn sixty_fps_interval_is_one_sixtieth() {
    let pacer = FramePacer::new(FPS60, Instant::now());
    assert_eq!(pacer.interval(), Duration::from_nanos(16_666_666));
}

#[test]
fn first_frame_is_due_immediately() {
    let now = Instant::now();
    assert!(FramePacer::new(FPS60, now).is_due(now));
}

#[test]
fn pacer_schedule() {
    let base = Instant::now();
    let off = |t: Instant| t.duration_since(base).as_nanos();
    let mut pacer = FramePacer::new(FPS60, base);
    let mut deadlines = Vec::new();
    // on time, 2 ms late, 50 ms stall
    for late_ms in [0u64, 2, 50] {
        let now = pacer.deadline() + Duration::from_millis(late_ms);
        pacer.on_frame(now);
        deadlines.push(off(pacer.deadline()));
    }
    insta::assert_debug_snapshot!(deadlines);
}
