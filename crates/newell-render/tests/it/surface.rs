// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use newell_render::RenderError;
use newell_render::surface::{FrameAction, SurfaceState, classify, configurable_extent};
use proptest::prelude::*;
use wgpu::SurfaceError;

proptest! {
    #[test]
    fn configurable_extent_iff_nonzero(w in prop_oneof![Just(0u32), Just(u32::MAX), any::<u32>()], h in any::<u32>()) {
        let expected = (w != 0 && h != 0).then_some((w, h));
        prop_assert_eq!(configurable_extent(w, h), expected);
    }
}

#[test]
fn classify_maps_each_acquire_result() {
    assert!(matches!(classify(Ok(false)), FrameAction::Present));
    assert!(matches!(
        classify(Ok(true)),
        FrameAction::PresentAndReconfigure
    ));
    assert!(matches!(
        classify(Err(SurfaceError::Outdated)),
        FrameAction::Reconfigure
    ));
    assert!(matches!(
        classify(Err(SurfaceError::Lost)),
        FrameAction::Reconfigure
    ));
    assert!(matches!(
        classify(Err(SurfaceError::Timeout)),
        FrameAction::Skip
    ));
    assert!(matches!(
        classify(Err(SurfaceError::OutOfMemory)),
        FrameAction::Fatal(RenderError::OutOfMemory)
    ));
    assert!(matches!(
        classify(Err(SurfaceError::Other)),
        FrameAction::Fatal(RenderError::Acquire(SurfaceError::Other))
    ));
}

#[test]
fn zero_size_is_inactive_and_never_configures() {
    for (w, h) in [(0, 0), (0, 5), (5, 0)] {
        let s = SurfaceState::new(w, h);
        assert!(!s.is_active());
        assert_eq!(s.pending_configure(), None);
    }
}

#[test]
fn configure_pending_until_marked_configured() {
    let mut s = SurfaceState::new(800, 600);
    assert!(s.is_active());
    assert_eq!(s.pending_configure(), Some((800, 600)));
    s.mark_configured();
    assert_eq!(s.pending_configure(), None);
}

#[test]
fn reconfigure_actions_set_pending_and_others_do_not() {
    for (action, expect) in [
        (FrameAction::Reconfigure, true),
        (FrameAction::PresentAndReconfigure, true),
        (FrameAction::Present, false),
        (FrameAction::Skip, false),
    ] {
        let mut s = SurfaceState::new(10, 10);
        s.mark_configured();
        s.apply(&action);
        assert_eq!(s.pending_configure().is_some(), expect, "{action:?}");
    }
}

#[test]
fn minimise_then_restore_reconfigures() {
    let mut s = SurfaceState::new(800, 600);
    s.mark_configured();
    s.resize(0, 0);
    assert!(!s.is_active());
    s.resize(800, 600);
    assert_eq!(s.pending_configure(), Some((800, 600)));
}
