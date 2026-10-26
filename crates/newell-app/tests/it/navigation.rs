// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

#![allow(clippy::unwrap_used)]

use newell_app::camera::{Camera, POLE_MARGIN, Ray};
use newell_app::navigation::{CLICK_DRAG_THRESHOLD_LOGICAL_PX, DragKind, Navigation, ZoomSource};
use newell_app::picking::{PickingScene, intersect_triangle};
use newell_kernel::NewellSolid;
use newell_render::SceneViewport;
use winit::event::{MouseButton, TouchPhase};

fn viewport() -> SceneViewport {
    SceneViewport::new(0, 0, 800, 600)
}
fn camera() -> Camera {
    Camera::look_at([0.5, -5.0, 0.5], [0.5; 3], viewport())
}
fn scene() -> PickingScene {
    let mut scene = PickingScene::default();
    scene.set_mesh(
        &NewellSolid::box_solid(1.0, 1.0, 1.0)
            .unwrap()
            .tessellate(0.05)
            .unwrap(),
    );
    scene
}
fn nav() -> Navigation {
    let mut nav = Navigation::new(camera());
    nav.set_mesh(
        &NewellSolid::box_solid(1.0, 1.0, 1.0)
            .unwrap()
            .tessellate(0.05)
            .unwrap(),
    );
    nav
}
fn close(a: [f64; 3], b: [f64; 3]) {
    for i in 0..3 {
        assert!((a[i] - b[i]).abs() < 1e-9, "{a:?} != {b:?}");
    }
}
fn pixel(a: (f64, f64), b: (f64, f64)) {
    assert!(
        (a.0 - b.0).abs() < 1e-7 && (a.1 - b.1).abs() < 1e-7,
        "{a:?} != {b:?}"
    );
}

#[test]
fn tessellation_query_returns_nearest_positive_world_point_and_revision() {
    let scene = scene();
    let ray = camera().ray((400.0, 300.0)).unwrap();
    let hit = scene.pick(ray).unwrap();
    close(hit.point, [0.5, 0.0, 0.5]);
    assert!((hit.distance - 5.0).abs() < 1e-12);
    assert!(hit.face_index < 6);
    assert_eq!(hit.mesh_revision, scene.revision());
    assert!(
        scene
            .pick(Ray {
                origin: ray.origin,
                direction: ray.direction.map(|v| -v)
            })
            .is_none()
    );
}

#[test]
fn triangle_intersection_matches_barycentric_fixture_and_rejects_degeneracy() {
    let triangle = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
    let ray = Ray {
        origin: [0.5, 0.75, 2.0],
        direction: [0.0, 0.0, -1.0],
    };
    assert_eq!(intersect_triangle(ray, triangle), Some(2.0));
    assert!(intersect_triangle(ray, [[0.0; 3]; 3]).is_none());
    assert!(
        intersect_triangle(
            Ray {
                direction: [0.0, 0.0, 1.0],
                ..ray
            },
            triangle
        )
        .is_none()
    );
    assert!(
        intersect_triangle(
            Ray {
                origin: [3.0, 3.0, 2.0],
                ..ray
            },
            triangle
        )
        .is_none()
    );
}

#[test]
fn visible_bounds_clip_to_exposed_regions_and_invalidate_cached_results() {
    let scene = scene();
    let mut camera = camera();
    camera.set_perspective(false);
    let full = scene.visible_bounds(camera, &[]).unwrap();
    close(full.centre(), [0.5; 3]);
    let hidden = SceneViewport::new(400, 0, 400, 600);
    let left = scene.visible_bounds(camera, &[hidden]).unwrap();
    assert!((left.min[0] - 0.0).abs() < 1e-12);
    assert!((left.max[0] - 0.5).abs() < 1e-12);
    assert!((left.centre()[0] - 0.25).abs() < 1e-12);
    close(
        scene.visible_bounds(camera, &[]).unwrap().centre(),
        [0.5; 3],
    );
    assert!(scene.visible_bounds(camera, &[viewport()]).is_none());
    let away = Camera::look_at([100.0, -5.0, 0.5], [100.0, 0.5, 0.5], viewport());
    assert!(scene.visible_bounds(away, &[]).is_none());
}

#[test]
fn bounds_clipping_handles_frustum_edges_and_near_plane_crossings() {
    let scene = scene();
    let inside = Camera::look_at([0.5, 0.5, 0.5], [0.5, 1.0, 0.5], viewport());
    let bounds = scene.visible_bounds(inside, &[]).unwrap();
    assert!(bounds.min[1] > 0.5);
    assert!((bounds.max[1] - 1.0).abs() < 1e-12);
    assert!(
        bounds
            .min
            .iter()
            .chain(bounds.max.iter())
            .all(|v| v.is_finite())
    );
    let mut shifted = camera();
    shifted.lens_shift = [1.0, 0.0];
    let partial = scene.visible_bounds(shifted, &[]).unwrap();
    assert!(partial.max[0] <= 0.5 + 1e-9);
}

#[test]
fn orbit_press_on_hit_or_miss_never_reframes_and_pivot_stays_at_its_pixel() {
    for point in [(400.0, 300.0), (420.0, 285.0), (5.0, 5.0)] {
        let mut nav = nav();
        let before = nav.camera();
        let snapshot = nav.snapshot();
        let pivot = nav
            .pick(point)
            .map(|h| h.point)
            .unwrap_or_else(|| nav.scene().visible_bounds(before, &[]).unwrap().centre());
        assert!(nav.begin(DragKind::Orbit, point, 1.0, true));
        assert_eq!(before, nav.camera());
        assert_eq!(snapshot, nav.snapshot());
        assert!(!nav.navigate(point));
        assert!(nav.navigate((point.0 + 100.0, point.1 - 40.0)));
        pixel(
            before.project(pivot).unwrap(),
            nav.camera().project(pivot).unwrap(),
        );
    }
}

#[test]
fn primary_drag_orbits_only_after_the_click_drag_threshold() {
    let mut nav = nav();
    let start = (400.0, 300.0);
    let before = nav.camera();
    assert!(nav.begin_primary_orbit(start, 1.0, true));
    assert!(nav.primary_orbit_pending());
    assert!(!nav.advance_primary_orbit((start.0 + CLICK_DRAG_THRESHOLD_LOGICAL_PX, start.1,)));
    assert_eq!(nav.camera(), before);
    assert!(!nav.is_dragging());
    assert!(nav.advance_primary_orbit((start.0 + CLICK_DRAG_THRESHOLD_LOGICAL_PX + 1.0, start.1,)));
    assert!(nav.is_dragging());
    assert_ne!(nav.camera(), before);
}

#[test]
fn primary_click_and_ui_press_never_start_an_orbit() {
    let mut nav = nav();
    let before = nav.camera();
    assert!(nav.begin_primary_orbit((400.0, 300.0), 1.0, true));
    nav.release(DragKind::PrimaryOrbit);
    assert!(!nav.primary_orbit_pending());
    assert!(!nav.is_dragging());
    assert!(!nav.advance_primary_orbit((430.0, 300.0)));
    assert_eq!(nav.camera(), before);

    // Application routing passes `eligible = false` for controls, menus, panels and resize zones.
    assert!(!nav.begin_primary_orbit((400.0, 300.0), 1.0, false));
    assert_eq!(nav.camera(), before);
}

#[test]
fn secondary_pan_and_middle_pan_share_contextual_depth_without_orbiting() {
    let start = (400.0, 300.0);
    let mut right = nav();
    let before_pan = right.camera();
    assert_eq!(DragKind::from_auxiliary_button(MouseButton::Left), None);
    assert_eq!(
        DragKind::from_auxiliary_button(MouseButton::Right),
        Some(DragKind::SecondaryPan)
    );
    assert_eq!(
        DragKind::from_auxiliary_button(MouseButton::Middle),
        Some(DragKind::Pan)
    );
    assert!(right.begin_secondary_pan(start, 1.0, true));
    assert!(right.secondary_pan_pending());
    assert!(!right.is_dragging());
    assert!(right.advance_secondary_pan((430.0, 280.0)));
    right.release(DragKind::SecondaryPan);
    assert!(!right.is_dragging());
    assert_ne!(right.camera(), before_pan);
    assert_eq!(right.camera().orientation, before_pan.orientation);

    let mut middle = nav();
    assert!(middle.begin(DragKind::Pan, start, 1.0, true));
    assert!(middle.navigate((430.0, 280.0)));
    middle.release(DragKind::Pan);
    assert!(!middle.is_dragging());
    assert_eq!(middle.camera(), right.camera());
}

#[test]
fn primary_orbit_release_and_cancellation_clear_pending_or_captured_state() {
    let mut nav = nav();
    let start = (400.0, 300.0);
    let before = nav.camera();
    assert!(nav.begin_primary_orbit(start, 1.0, true));
    nav.cancel(); // Focus loss and window exit use this path.
    assert!(!nav.primary_orbit_pending());
    assert!(!nav.is_dragging());
    assert_eq!(nav.camera(), before);

    assert!(nav.begin_primary_orbit(start, 1.0, true));
    assert!(nav.advance_primary_orbit((410.0, 300.0)));
    assert!(nav.is_dragging());
    nav.release(DragKind::PrimaryOrbit);
    assert!(!nav.is_dragging());
}

#[test]
fn edge_starts_and_event_partition_do_not_change_empty_scene_orbit_response() {
    let mut outcomes = Vec::new();
    for start in [(400.0, 300.0), (1.0, 1.0), (799.0, 599.0)] {
        let mut nav = Navigation::new(camera());
        nav.begin(DragKind::Orbit, start, 1.0, true);
        for fraction in [0.1, 0.3, 0.7, 1.0] {
            nav.navigate((start.0 + 70.0 * fraction, start.1 - 30.0 * fraction));
        }
        outcomes.push(nav.camera());
    }
    close(outcomes[0].eye, outcomes[1].eye);
    close(outcomes[0].eye, outcomes[2].eye);
}

#[test]
fn pole_overscroll_does_not_delay_reversal_in_same_gesture() {
    let mut nav = nav();
    nav.begin(DragKind::Orbit, (400.0, 300.0), 1.0, true);
    nav.navigate((400.0, -10000.0));
    assert!((nav.camera().basis().2[2] - POLE_MARGIN.cos()).abs() < 1e-12);
    nav.navigate((400.0, -9999.0));
    assert!(nav.camera().basis().2[2] < POLE_MARGIN.cos() - 1e-8);
}

#[test]
fn unrelated_button_release_cannot_end_or_replace_capture() {
    let mut nav = nav();
    assert!(nav.begin(DragKind::Orbit, (400.0, 300.0), 1.0, true));
    assert!(!nav.begin(DragKind::Pan, (400.0, 300.0), 1.0, true));
    nav.release(DragKind::Pan);
    assert!(nav.is_dragging());
    nav.release(DragKind::Orbit);
    assert!(!nav.is_dragging());
}

#[test]
fn pan_uses_geometric_depth_and_continues_outside_viewport() {
    let mut nav = nav();
    let point = (420.0, 300.0);
    let hit = nav.pick(point).unwrap();
    let orientation = nav.camera().orientation;
    nav.begin(DragKind::Pan, point, 1.0, true);
    nav.navigate((900.0, 700.0));
    pixel(nav.camera().project(hit.point).unwrap(), (900.0, 700.0));
    assert_eq!(orientation, nav.camera().orientation);
}

#[test]
fn occlusion_and_shell_ownership_reject_gestures_and_picks() {
    let mut nav = nav();
    nav.set_layout(viewport(), vec![SceneViewport::new(380, 200, 100, 200)]);
    let before = nav.camera();
    for point in [
        (400.0, 300.0),
        (-1.0, 100.0),
        (800.0, 300.0),
        (100.0, 600.0),
        (f64::NAN, 0.0),
    ] {
        assert!(!nav.begin(DragKind::Orbit, point, 1.0, true));
        assert!(!nav.begin(DragKind::Pan, point, 1.0, true));
        assert!(!nav.zoom(point, 1.0, None, true));
        assert!(nav.pick(point).is_none());
    }
    assert!(!nav.begin(DragKind::Orbit, (200.0, 300.0), 1.0, false));
    assert!(!nav.zoom((200.0, 300.0), 1.0, None, false));
    assert_eq!(before, nav.camera());
}

#[test]
fn mesh_projection_and_layout_changes_cancel_capture_without_recentring() {
    let mesh = NewellSolid::box_solid(2.0, 2.0, 2.0)
        .unwrap()
        .tessellate(0.05)
        .unwrap();
    let mut nav = nav();
    nav.begin(DragKind::Orbit, (400.0, 300.0), 1.0, true);
    nav.navigate((420.0, 290.0));
    let before = nav.camera();
    nav.set_mesh(&mesh);
    assert_eq!(before, nav.camera());
    assert!(!nav.is_dragging());
    nav.begin(DragKind::Pan, (400.0, 300.0), 1.0, true);
    nav.set_layout(viewport(), vec![SceneViewport::new(650, 0, 150, 600)]);
    assert_eq!(before, nav.camera());
    assert!(!nav.is_dragging());
    nav.set_layout(viewport(), vec![]);
    assert_eq!(before, nav.camera());
    nav.begin(DragKind::Pan, (400.0, 300.0), 1.0, true);
    nav.set_layout(SceneViewport::new(30, 20, 650, 700), vec![]);
    assert!(!nav.is_dragging());
    pixel(
        before.project([0.5; 3]).unwrap(),
        nav.camera().project([0.5; 3]).unwrap(),
    );
    nav.begin(DragKind::Orbit, (400.0, 300.0), 1.0, true);
    nav.set_perspective(false);
    assert!(!nav.is_dragging());
}

#[test]
fn cancelled_capture_preserves_pose_and_rejects_trackpad_tail() {
    let mut nav = nav();
    let before = nav.camera();
    nav.begin(DragKind::Pan, (400.0, 300.0), 1.0, true);
    nav.cancel();
    assert_eq!(before, nav.camera());
    assert!(!nav.navigate((500.0, 300.0)));
    let stream = |phase| Some((ZoomSource::Scroll, phase));
    nav.zoom((400.0, 300.0), 0.0, stream(TouchPhase::Started), true);
    nav.cancel();
    assert!(!nav.zoom((400.0, 300.0), 1.0, stream(TouchPhase::Moved), true));
    assert_eq!(before, nav.camera());
}

#[test]
fn phased_zoom_freezes_surface_anchor_including_zero_delta_start() {
    for source in [ZoomSource::Scroll, ZoomSource::Pinch] {
        let mut nav = nav();
        let point = (420.0, 285.0);
        let hit = nav.pick(point).unwrap();
        let initial = nav.camera().project(hit.point).unwrap();
        assert!(!nav.zoom(point, 0.0, Some((source, TouchPhase::Started)), true));
        for _ in 0..20 {
            assert!(nav.zoom(point, 0.05, Some((source, TouchPhase::Moved)), true));
            pixel(nav.camera().project(hit.point).unwrap(), initial);
        }
        nav.zoom(point, 0.0, Some((source, TouchPhase::Ended)), true);
    }
}

#[test]
fn gesture_started_under_ui_stays_rejected_when_pointer_enters_scene() {
    let mut nav = nav();
    let before = nav.camera();
    for phase in [TouchPhase::Started, TouchPhase::Moved, TouchPhase::Ended] {
        let eligible = phase != TouchPhase::Started;
        assert!(!nav.zoom(
            (400.0, 300.0),
            1.0,
            Some((ZoomSource::Scroll, phase)),
            eligible
        ));
    }
    assert_eq!(before, nav.camera());
    assert!(nav.zoom(
        (400.0, 300.0),
        1.0,
        Some((ZoomSource::Scroll, TouchPhase::Started)),
        true
    ));
}

#[test]
fn fit_recovers_after_pan_and_contains_all_vertices_beside_action_box() {
    for perspective in [true, false] {
        let mut nav = nav();
        nav.set_perspective(perspective);
        nav.set_layout(viewport(), vec![SceneViewport::new(450, 0, 350, 600)]);
        nav.begin(DragKind::Pan, (100.0, 100.0), 1.0, true);
        nav.navigate((3000.0, 3000.0));
        nav.release(DragKind::Pan);
        let orientation = nav.camera().orientation;
        assert!(nav.fit());
        assert_eq!(orientation, nav.camera().orientation);
        for x in [0.0, 1.0] {
            for y in [0.0, 1.0] {
                for z in [0.0, 1.0] {
                    let point = nav.camera().project([x, y, z]).unwrap();
                    assert!(nav.visible(point), "hidden {point:?}");
                }
            }
        }
    }
}

#[test]
fn missing_scene_and_fully_occluded_scene_have_safe_fallbacks() {
    let mut nav = Navigation::new(camera());
    assert!(nav.begin(DragKind::Orbit, (10.0, 10.0), 1.0, true));
    assert!(nav.navigate((20.0, 15.0)));
    nav.cancel();
    assert!(!nav.fit());
    assert!(nav.snapshot().is_some());
    nav.set_layout(viewport(), vec![viewport()]);
    assert!(!nav.begin(DragKind::Orbit, (400.0, 300.0), 1.0, true));
    assert!(!nav.fit());
}

#[test]
fn immutable_view_snapshot_matches_controller_matrices_and_revision() {
    let mut nav = nav();
    let first = nav.snapshot().unwrap();
    let camera = nav.camera();
    let matrix = camera.view();
    for (actual, expected) in first.view().iter().flatten().zip(matrix.iter().flatten()) {
        assert_eq!(*actual, *expected as f32);
    }
    nav.begin(DragKind::Pan, (400.0, 300.0), 1.0, true);
    nav.navigate((430.0, 300.0));
    let next = nav.snapshot().unwrap();
    assert!(next.revision() > first.revision());
    assert_ne!(next.view_projection(), first.view_projection());
}

#[test]
fn missing_pointer_at_stream_start_cannot_leak_into_viewport() {
    let mut nav = nav();
    assert!(!nav.zoom(
        (f64::NAN, f64::NAN),
        0.0,
        Some((ZoomSource::Pinch, TouchPhase::Started)),
        true
    ));
    assert!(!nav.zoom(
        (400.0, 300.0),
        1.0,
        Some((ZoomSource::Pinch, TouchPhase::Moved)),
        true
    ));
}

#[test]
fn invalid_motion_does_not_corrupt_active_capture_or_uniforms() {
    let mut nav = nav();
    nav.begin(DragKind::Orbit, (400.0, 300.0), 1.0, true);
    let before = nav.camera();
    assert!(!nav.navigate((f64::INFINITY, 300.0)));
    assert_eq!(before, nav.camera());
    assert!(nav.snapshot().is_some());
}

#[test]
fn primary_click_jitter_preserves_camera_snapshot_and_face_picking() {
    let mut nav = nav();
    let start = (400.25, 300.5);
    let before = nav.camera();
    let snapshot = nav.snapshot();
    let hit = nav.pick(start).unwrap();
    assert!(nav.begin_primary_orbit(start, 2.0, true));
    for point in [(401.25, 301.5), (406.25, 300.5), start] {
        assert!(!nav.advance_primary_orbit(point));
        assert_eq!(nav.camera(), before);
        assert_eq!(nav.snapshot(), snapshot);
    }
    nav.release(DragKind::PrimaryOrbit);
    assert_eq!(nav.pick(start), Some(hit));
    assert_eq!(nav.camera(), before);
    assert!(!nav.navigate((450.0, 300.0)));
}

#[test]
fn primary_orbit_reuses_existing_orbit_pose_pivot_and_subpixel_start() {
    for start in [(420.25, 285.75), (799.5, 599.5)] {
        for display_scale in [1.0, 2.0] {
            let end = (start.0 + 60.0, start.1 - 40.0);
            let mut primary = nav();
            let mut existing = nav();
            assert!(primary.begin_primary_orbit(start, display_scale, true));
            assert!(existing.begin(DragKind::Orbit, start, display_scale, true));
            assert!(primary.advance_primary_orbit(end));
            assert!(existing.navigate(end));
            assert_eq!(primary.camera(), existing.camera());
            assert!(!primary.primary_orbit_pending());
            // Subsequent captured motion also uses the same orbit path.
            let next = (end.0 + 20.0, end.1 + 10.0);
            primary.navigate(next);
            existing.navigate(next);
            assert_eq!(primary.camera(), existing.camera());
        }
    }
}

#[test]
fn primary_threshold_is_radial_and_scaled_in_logical_pixels() {
    for display_scale in [1.0, 2.0] {
        let mut nav = nav();
        let start = (400.25, 300.5);
        let threshold = CLICK_DRAG_THRESHOLD_LOGICAL_PX * display_scale;
        let before = nav.snapshot();
        nav.begin_primary_orbit(start, display_scale, true);
        assert!(!nav.advance_primary_orbit((start.0 + threshold, start.1)));
        assert_eq!(nav.snapshot(), before);
        // Both components are below the per-axis threshold, but their radius exceeds it.
        assert!(
            nav.advance_primary_orbit((start.0 + threshold * 0.75, start.1 + threshold * 0.75))
        );
        assert!(nav.is_dragging());
    }
}

#[test]
fn pending_primary_press_keeps_its_camera_frozen_until_capture_or_release() {
    let mut nav = nav();
    let start = (400.0, 300.0);
    nav.begin_primary_orbit(start, 1.0, true);
    let before = nav.camera();
    assert!(!nav.zoom(start, 1.0, None, true));
    assert!(!nav.zoom(
        start,
        1.0,
        Some((ZoomSource::Pinch, TouchPhase::Started)),
        true
    ));
    assert_eq!(nav.camera(), before);
    nav.release(DragKind::PrimaryOrbit);
    assert!(nav.zoom(start, 1.0, None, true));
    assert!(nav.zoom(
        start,
        1.0,
        Some((ZoomSource::Pinch, TouchPhase::Started)),
        true
    ));
}

#[test]
fn primary_orbit_only_its_own_release_ends_pending_or_captured_drag() {
    let mut nav = nav();
    let start = (400.0, 300.0);
    nav.begin_primary_orbit(start, 1.0, true);
    for kind in [DragKind::Orbit, DragKind::Pan] {
        nav.release(kind);
        assert!(nav.primary_orbit_pending());
    }
    nav.advance_primary_orbit((420.0, 300.0));
    for kind in [DragKind::Orbit, DragKind::Pan] {
        nav.release(kind);
        assert!(nav.is_dragging());
    }
    let before_release = nav.camera();
    nav.release(DragKind::PrimaryOrbit);
    assert!(!nav.is_dragging());
    assert!(!nav.primary_orbit_pending());
    assert!(!nav.navigate((440.0, 300.0)));
    assert_eq!(nav.camera(), before_release);
}

#[test]
fn focus_loss_or_window_exit_cancel_primary_pending_and_captured_drag() {
    for captured in [false, true] {
        let mut nav = nav();
        nav.begin_primary_orbit((400.0, 300.0), 1.0, true);
        if captured {
            nav.advance_primary_orbit((420.0, 300.0));
        }
        let before = nav.camera();
        // App's Focused(false) and CursorLeft branches both invoke this cancellation.
        nav.cancel();
        // A release delivered after exiting the window must also be harmless.
        nav.release(DragKind::PrimaryOrbit);
        assert!(!nav.is_dragging());
        assert!(!nav.primary_orbit_pending());
        assert!(!nav.advance_primary_orbit((450.0, 300.0)));
        assert!(!nav.navigate((450.0, 300.0)));
        assert_eq!(nav.camera(), before);
        assert!(nav.begin(DragKind::Orbit, (400.0, 300.0), 1.0, true));
        assert!(nav.navigate((420.0, 300.0)));
    }
}

#[test]
fn primary_press_rejects_opaque_pixels_and_nonfinite_motion() {
    let mut nav = nav();
    nav.set_layout(viewport(), vec![SceneViewport::new(380, 200, 100, 200)]);
    let before = nav.camera();
    assert!(!nav.begin_primary_orbit((400.0, 300.0), 1.0, true));
    assert!(!nav.begin_primary_orbit((-1.0, 300.0), 1.0, true));
    assert!(!nav.begin_primary_orbit((200.0, 300.0), 1.0, false));
    assert!(!nav.advance_primary_orbit((500.0, 300.0)));
    assert_eq!(nav.camera(), before);
    assert!(nav.begin_primary_orbit((200.0, 300.0), 1.0, true));
    assert!(!nav.advance_primary_orbit((f64::NAN, 300.0)));
    assert_eq!(nav.camera(), before);
}

#[test]
fn primary_pending_drag_is_cancelled_by_layout_mesh_or_projection_changes() {
    for change in 0..3 {
        let mut nav = nav();
        nav.begin_primary_orbit((400.0, 300.0), 1.0, true);
        match change {
            0 => nav.set_layout(viewport(), vec![SceneViewport::new(700, 0, 100, 600)]),
            1 => nav.set_mesh(
                &NewellSolid::box_solid(2.0, 2.0, 2.0)
                    .unwrap()
                    .tessellate(0.05)
                    .unwrap(),
            ),
            _ => {
                nav.set_perspective(false);
            }
        }
        let after_change = nav.camera();
        assert!(!nav.primary_orbit_pending());
        assert!(!nav.advance_primary_orbit((450.0, 300.0)));
        assert_eq!(nav.camera(), after_change);
    }
}

#[test]
fn secondary_click_jitter_preserves_camera_snapshot_and_picking() {
    let mut nav = nav();
    let start = (400.25, 300.5);
    let before = nav.camera();
    let snapshot = nav.snapshot();
    let hit = nav.pick(start);
    assert!(nav.begin_secondary_pan(start, 2.0, true));
    for point in [(401.25, 301.5), (408.25, 300.5), start] {
        assert!(!nav.advance_secondary_pan(point));
        assert_eq!(nav.camera(), before);
        assert_eq!(nav.snapshot(), snapshot);
    }
    nav.release(DragKind::SecondaryPan);
    assert!(!nav.secondary_pan_pending());
    assert!(!nav.advance_secondary_pan((450.0, 300.0)));
    assert_eq!(nav.pick(start), hit);
    assert_eq!(nav.camera(), before);
}

#[test]
fn primary_and_secondary_drags_reject_ui_and_occluded_pixels() {
    let mut nav = nav();
    nav.set_layout(viewport(), vec![SceneViewport::new(380, 200, 100, 200)]);
    let before = nav.camera();
    for (point, eligible) in [((400.0, 300.0), true), ((200.0, 300.0), false)] {
        assert!(!nav.begin_primary_orbit(point, 1.0, eligible));
        assert!(!nav.begin_secondary_pan(point, 1.0, eligible));
        assert!(!nav.has_pending_drag());
        assert!(!nav.advance_pending_drag((500.0, 300.0)));
    }
    assert_eq!(nav.camera(), before);
}

#[test]
fn secondary_pan_release_focus_loss_and_scene_changes_cancel_capture() {
    let start = (400.0, 300.0);
    for captured in [false, true] {
        for cancel_by_release in [false, true] {
            let mut nav = nav();
            assert!(nav.begin_secondary_pan(start, 1.0, true));
            if captured {
                assert!(nav.advance_secondary_pan((420.0, 300.0)));
            }
            let before = nav.camera();
            nav.release(DragKind::Pan);
            assert_eq!(nav.secondary_pan_pending(), !captured);
            assert_eq!(nav.is_dragging(), captured);
            if cancel_by_release {
                nav.release(DragKind::SecondaryPan);
            } else {
                nav.cancel(); // Focus loss and pointer exit use this path.
                nav.release(DragKind::SecondaryPan); // A late release is harmless.
            }
            assert!(!nav.secondary_pan_pending());
            assert!(!nav.is_dragging());
            assert!(!nav.advance_secondary_pan((450.0, 300.0)));
            assert!(!nav.navigate((450.0, 300.0)));
            assert_eq!(nav.camera(), before);
        }
    }
    for captured in [false, true] {
        for change in 0..3 {
            let mut nav = nav();
            nav.begin_secondary_pan(start, 1.0, true);
            if captured {
                assert!(nav.advance_secondary_pan((420.0, 300.0)));
            }
            match change {
                0 => nav.set_layout(viewport(), vec![SceneViewport::new(380, 200, 100, 200)]),
                1 => nav.set_mesh(
                    &NewellSolid::box_solid(2.0, 2.0, 2.0)
                        .unwrap()
                        .tessellate(0.05)
                        .unwrap(),
                ),
                _ => {
                    nav.set_perspective(false);
                }
            }
            let after_change = nav.camera();
            assert!(!nav.secondary_pan_pending());
            assert!(!nav.is_dragging());
            assert!(!nav.advance_secondary_pan((450.0, 300.0)));
            assert!(!nav.navigate((450.0, 300.0)));
            assert_eq!(nav.camera(), after_change);
        }
    }
}

#[test]
fn secondary_binding_leaves_wheel_and_pinch_zoom_available() {
    let point = (400.0, 300.0);
    let mut nav = nav();
    assert!(nav.begin_secondary_pan(point, 1.0, true));
    nav.release(DragKind::SecondaryPan);
    let before = nav.camera();
    assert!(nav.zoom(point, 1.0, None, true));
    assert_ne!(nav.camera(), before);
    let after_wheel = nav.camera();
    assert!(!nav.zoom(
        point,
        0.0,
        Some((ZoomSource::Pinch, TouchPhase::Started)),
        true
    ));
    assert!(nav.zoom(
        point,
        0.5,
        Some((ZoomSource::Pinch, TouchPhase::Moved)),
        true
    ));
    nav.zoom(
        point,
        0.0,
        Some((ZoomSource::Pinch, TouchPhase::Ended)),
        true,
    );
    assert_ne!(nav.camera(), after_wheel);
}

#[test]
fn secondary_click_pixel_scroll_crosses_threshold_into_existing_pan() {
    let start = (400.25, 300.5);
    let mut trackpad = nav();
    let mut pointer = nav();
    let before = trackpad.camera();
    assert!(trackpad.begin_secondary_pan(start, 2.0, true));
    assert!(pointer.begin_secondary_pan(start, 2.0, true));
    assert_eq!(
        trackpad.advance_secondary_pan_scroll((0.0, 0.0), TouchPhase::Started, start),
        Some(false)
    );
    assert_eq!(
        trackpad.advance_secondary_pan_scroll((6.0, 0.0), TouchPhase::Moved, start),
        Some(false)
    );
    assert_eq!(trackpad.camera(), before);
    assert_eq!(
        trackpad.advance_secondary_pan_scroll((6.0, -10.0), TouchPhase::Moved, start),
        Some(true)
    );
    assert!(pointer.advance_secondary_pan((start.0 + 12.0, start.1 - 10.0)));
    assert_eq!(trackpad.camera(), pointer.camera());
    assert_eq!(trackpad.camera().orientation, before.orientation);
}

#[test]
fn secondary_scroll_and_pointer_motion_combine_without_jump() {
    let start = (400.0, 300.0);
    let mut mixed = nav();
    let mut pointer = nav();
    mixed.begin_secondary_pan(start, 1.0, true);
    pointer.begin_secondary_pan(start, 1.0, true);
    assert_eq!(
        mixed.advance_secondary_pan_scroll((10.0, -5.0), TouchPhase::Started, start),
        Some(true)
    );
    assert!(mixed.advance_pointer((420.0, 290.0)));
    assert!(pointer.advance_secondary_pan((430.0, 285.0)));
    assert_eq!(mixed.camera(), pointer.camera());
    assert_eq!(
        mixed.advance_secondary_pan_scroll((5.0, 5.0), TouchPhase::Ended, (420.0, 290.0)),
        Some(true)
    );
    assert!(pointer.navigate((435.0, 290.0)));
    assert_eq!(mixed.camera(), pointer.camera());
}

#[test]
fn pointer_routing_preserves_primary_orbit_and_middle_pan() {
    let start = (400.0, 300.0);
    let end = (425.0, 285.0);
    let mut primary = nav();
    let mut expected_orbit = nav();
    primary.begin_primary_orbit(start, 1.0, true);
    expected_orbit.begin_primary_orbit(start, 1.0, true);
    assert!(primary.advance_pointer(end));
    assert!(expected_orbit.advance_primary_orbit(end));
    assert_eq!(primary.camera(), expected_orbit.camera());

    let mut middle = nav();
    let mut expected_pan = nav();
    middle.begin(DragKind::Pan, start, 1.0, true);
    expected_pan.begin(DragKind::Pan, start, 1.0, true);
    assert!(middle.advance_pointer(end));
    assert!(expected_pan.navigate(end));
    assert_eq!(middle.camera(), expected_pan.camera());
}

#[test]
fn secondary_scroll_owns_only_its_pressed_stream_and_rejects_momentum() {
    let start = (400.0, 300.0);
    let mut nav = nav();
    assert_eq!(
        nav.advance_secondary_pan_scroll((20.0, 0.0), TouchPhase::Started, start),
        None
    );
    nav.begin_primary_orbit(start, 1.0, true);
    assert_eq!(
        nav.advance_secondary_pan_scroll((20.0, 0.0), TouchPhase::Started, start),
        None
    );
    nav.release(DragKind::PrimaryOrbit);
    nav.begin(DragKind::Pan, start, 1.0, true);
    assert_eq!(
        nav.advance_secondary_pan_scroll((20.0, 0.0), TouchPhase::Started, start),
        None
    );
    nav.release(DragKind::Pan);

    nav.begin_secondary_pan(start, 1.0, true);
    assert_eq!(
        nav.advance_secondary_pan_scroll((20.0, 0.0), TouchPhase::Started, start),
        Some(true)
    );
    nav.release(DragKind::SecondaryPan);
    let after_release = nav.camera();
    assert_eq!(
        nav.advance_secondary_pan_scroll((20.0, 0.0), TouchPhase::Moved, start),
        Some(false)
    );
    assert_eq!(nav.camera(), after_release);
    assert!(!nav.zoom(
        start,
        1.0,
        Some((ZoomSource::Scroll, TouchPhase::Moved)),
        true
    ));
    assert_eq!(
        nav.advance_secondary_pan_scroll((0.0, 0.0), TouchPhase::Ended, start),
        Some(false)
    );
    assert_eq!(
        nav.advance_secondary_pan_scroll((0.0, 0.0), TouchPhase::Started, start),
        None
    );
    assert!(!nav.zoom(
        start,
        0.0,
        Some((ZoomSource::Scroll, TouchPhase::Started)),
        true
    ));
    assert!(nav.zoom(
        start,
        1.0,
        Some((ZoomSource::Scroll, TouchPhase::Moved)),
        true
    ));
}

#[test]
fn secondary_scroll_rejects_ui_starts_and_invalid_deltas() {
    let mut nav = nav();
    let start = (400.0, 300.0);
    nav.set_layout(viewport(), vec![SceneViewport::new(380, 200, 100, 200)]);
    assert!(!nav.begin_secondary_pan(start, 1.0, true));
    assert_eq!(
        nav.advance_secondary_pan_scroll((30.0, 0.0), TouchPhase::Started, start),
        None
    );
    nav.set_layout(viewport(), Vec::new());
    nav.begin_secondary_pan(start, 1.0, true);
    let before = nav.camera();
    assert_eq!(
        nav.advance_secondary_pan_scroll((f64::NAN, 0.0), TouchPhase::Started, start),
        Some(false)
    );
    assert_eq!(nav.camera(), before);
    nav.cancel(); // Focus loss and pointer exit use this path.
    assert_eq!(
        nav.advance_secondary_pan_scroll((30.0, 0.0), TouchPhase::Moved, start),
        Some(false)
    );
    assert_eq!(nav.camera(), before);
}
