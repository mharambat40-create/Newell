// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M2.24 runtime status: actual presented-frame samples and one timed progress control.
//! Time is injected as monotonic elapsed duration; no window, GPU or wall clock is needed.

use std::time::Duration;

const FPS_INTERVAL: Duration = Duration::from_millis(200);

/// One progress animation. Each direction has exactly twenty timed steps.
#[derive(Debug, Clone, Default)]
pub struct Progress {
    percent: u32,
    run: Option<(Duration, bool)>,
}

impl Progress {
    pub const fn percent(&self) -> u32 {
        self.percent
    }
    pub const fn active(&self) -> bool {
        self.run.is_some()
    }

    /// Complete elapsed work before accepting another independent endpoint press.
    pub fn press(&mut self, now: Duration) -> bool {
        self.advance(now);
        if self.run.is_some() {
            return false;
        }
        self.run = Some((now, self.percent == 0));
        true
    }

    pub fn advance(&mut self, now: Duration) -> bool {
        let Some((start, up)) = self.run else {
            return false;
        };
        let duration = Duration::from_secs(2);
        let elapsed = now.saturating_sub(start).min(duration);
        // Up uses floor; down uses ceil of the remaining fraction. Integer arithmetic
        // gives 95% at 4.99s ascending and 5% at 0.999s descending.
        let steps = (elapsed.as_nanos() * 20 / duration.as_nanos()) as u32;
        let value = if up { steps * 5 } else { 100 - steps * 5 };
        let changed = self.percent != value;
        self.percent = value;
        if elapsed == duration {
            self.run = None;
        }
        changed
    }

    pub fn deadline(&self, now: Duration) -> Option<Duration> {
        let (start, _) = self.run?;
        let step = Duration::from_millis(100);
        let index = now.saturating_sub(start).as_nanos() / step.as_nanos() + 1;
        let index = u32::try_from(index.min(20)).ok()?;
        Some(start.saturating_add(step * index))
    }
}

/// Runtime-only measurement state. Frames are counted only after successful presentation.
#[derive(Debug, Clone)]
pub struct StatusClock {
    pub progress: Progress,
    pub fps: f64,
    frames: u64,
    sampled_at: Duration,
    fps_deadline: Duration,
}

impl Default for StatusClock {
    fn default() -> Self {
        Self {
            progress: Progress::default(),
            fps: 0.0,
            frames: 0,
            sampled_at: Duration::ZERO,
            fps_deadline: FPS_INTERVAL,
        }
    }
}

impl StatusClock {
    pub fn presented(&mut self) {
        self.frames = self.frames.saturating_add(1);
    }

    /// True when a status refresh is due, including an unchanged FPS sample at 5 Hz.
    pub fn advance(&mut self, now: Duration) -> bool {
        let mut changed = self.progress.advance(now);
        if now >= self.fps_deadline {
            let elapsed = now.saturating_sub(self.sampled_at).as_secs_f64();
            self.fps = if elapsed > 0.0 {
                self.frames as f64 / elapsed
            } else {
                0.0
            };
            self.frames = 0;
            self.sampled_at = now;
            let skipped =
                now.saturating_sub(self.fps_deadline).as_nanos() / FPS_INTERVAL.as_nanos() + 1;
            let skipped = u32::try_from(skipped).unwrap_or(u32::MAX);
            self.fps_deadline = self.fps_deadline.saturating_add(FPS_INTERVAL * skipped);
            changed = true;
        }
        changed
    }

    /// Progress adds no deadline while idle; the FPS sampler retains its own phase.
    pub fn deadline(&self, now: Duration) -> Duration {
        self.progress
            .deadline(now)
            .map_or(self.fps_deadline, |p| p.min(self.fps_deadline))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn endpoints_are_reached_only_at_exact_deadlines() {
        let mut p = Progress::default();
        assert_eq!(p.percent(), 0);
        assert_eq!(p.deadline(ms(0)), None);
        assert!(p.press(ms(0)));
        assert_eq!(p.deadline(ms(0)), Some(ms(100)));
        p.advance(ms(1990));
        assert_eq!(p.percent(), 95);
        assert!(p.active());
        assert!(!p.press(ms(1999)));
        p.advance(Duration::from_secs(2) - Duration::from_nanos(1));
        assert_eq!(p.percent(), 95);
        p.advance(ms(2000));
        assert_eq!(p.percent(), 100);
        assert!(!p.active());
        assert!(p.press(ms(2000)));
        p.advance(ms(3999));
        assert_eq!(p.percent(), 5);
        assert!(p.active());
        p.advance(Duration::from_secs(4) - Duration::from_nanos(1));
        assert_eq!(p.percent(), 5);
        p.advance(ms(4000));
        assert_eq!(p.percent(), 0);
        assert!(!p.active());
    }

    #[test]
    fn every_state_uses_floor_up_and_ceil_down_with_no_early_endpoint() {
        for elapsed in 0..=2000 {
            let mut up = Progress::default();
            up.press(ms(0));
            up.advance(ms(elapsed));
            let value = u64::from(up.percent());
            assert!(value * 2000 <= 100 * elapsed);
            assert!(value == 100 || (value + 5) * 2000 > 100 * elapsed);
            if elapsed % 100 == 0 {
                assert_eq!(value, elapsed / 20);
            }
        }
        for elapsed in 0..=2000 {
            let mut down = Progress {
                percent: 100,
                run: None,
            };
            down.press(ms(0));
            down.advance(ms(elapsed));
            let value = u64::from(down.percent());
            assert!(value * 2000 >= 100 * (2000 - elapsed));
            assert!(value == 0 || (value - 5) * 2000 < 100 * (2000 - elapsed));
            if elapsed % 100 == 0 {
                assert_eq!(value, (2000 - elapsed) / 20);
            }
        }
    }

    #[test]
    fn skipped_frames_and_ignored_clicks_do_not_change_the_timeline() {
        let mut direct = Progress::default();
        direct.press(ms(30));
        let mut stepped = direct.clone();
        for t in 30..=1029 {
            stepped.advance(ms(t));
            assert!(!stepped.press(ms(t)));
        }
        direct.advance(ms(1029));
        assert_eq!(direct.percent(), stepped.percent());
        assert_eq!(direct.deadline(ms(4029)), stepped.deadline(ms(4029)));
        direct.advance(ms(2030));
        stepped.advance(ms(2030));
        assert_eq!(direct.percent(), 100);
        assert_eq!(stepped.percent(), 100);
    }

    #[test]
    fn fps_counts_presentations_and_keeps_its_independent_five_hz_phase() {
        let mut s = StatusClock::default();
        assert_eq!(s.deadline(ms(0)), ms(200));
        for _ in 0..12 {
            s.presented();
        }
        assert!(!s.advance(ms(199)));
        assert!(s.advance(ms(200)));
        assert_eq!(s.fps, 60.0);
        s.progress.press(ms(210));
        assert_eq!(s.deadline(ms(210)), ms(310));
        s.advance(ms(310));
        assert_eq!(s.fps, 60.0);
        assert_eq!(s.deadline(ms(310)), ms(400));
        assert!(s.advance(ms(400)));
        assert_eq!(s.deadline(ms(400)), ms(410));
        s.advance(ms(2210));
        assert_eq!(s.progress.percent(), 100);
        assert!(s.progress.press(ms(2210)));
        assert_eq!(s.deadline(ms(2210)), ms(2310));
    }

    #[test]
    fn late_samples_measure_elapsed_time_and_skip_missed_wakeups() {
        let mut s = StatusClock::default();
        for _ in 0..5 {
            s.presented();
        }
        assert!(s.advance(ms(1000)));
        assert_eq!(s.fps, 5.0);
        assert_eq!(s.deadline(ms(1000)), ms(1200));
        assert!(!s.advance(ms(1000)));
    }
}
