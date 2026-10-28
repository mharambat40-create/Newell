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
fn named_widget_views_keep_the_contextual_model_pivot_at_its_pixel() {
    let direction = newell_ui::navigation_widgets::ViewDirection::new([1, 1, 1]).unwrap();
    for perspective in [false, true] {
        let mut nav = nav();
        nav.set_perspective(perspective);
        assert!(nav.begin(DragKind::Pan, (400., 300.), 1., true));
        assert!(nav.navigate((600., 390.)));
        let before = nav.camera();
        let center = (400., 300.);
        let pivot = nav
            .pick(center)
            .map(|hit| hit.point)
            .or_else(|| nav.scene().visible_bounds(before, &[]).map(|b| b.centre()))
            .unwrap();
        let pixel_before = before.project(pivot).unwrap();
        assert!(
            (pixel_before.0 - 400.).abs() > 1. || (pixel_before.1 - 300.).abs() > 1.,
            "the fixture must exercise a panned, off-center model"
        );
        let projection = before.perspective;
        let focal_distance = before.focal_distance;
        let orthographic_height = before.orthographic_height;
        let fov = before.vertical_fov;
        let lens_shift = before.lens_shift;

        assert!(nav.align_view(direction));
        let after = nav.camera();
        pixel(after.project(pivot).unwrap(), pixel_before);
        assert_eq!(after.perspective, projection);
        assert_eq!(after.focal_distance, focal_distance);
        assert_eq!(after.orthographic_height, orthographic_height);
        assert_eq!(after.vertical_fov, fov);
        assert_eq!(after.lens_shift, lens_shift);
        assert!(!nav.align_view(direction));
        assert_eq!(
            nav.camera(),
            after,
            "repeating the same orientation must be stable"
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
fn startup_scene_center_is_restored_after_final_viewport_layout_without_refitting() {
    let mut nav = nav();
    nav.set_perspective(false);
    nav.set_layout(SceneViewport::new(180, 72, 920, 648), Vec::new());

    let layout_camera = nav.camera();
    let viewport = layout_camera.viewport;
    let centre = nav.scene().bounds().unwrap().centre();
    let before = nav.camera().project(centre).unwrap();
    let viewport_centre = (
        f64::from(viewport.x) + f64::from(viewport.width) * 0.5,
        f64::from(viewport.y) + f64::from(viewport.height) * 0.5,
    );
    assert_ne!(
        before, viewport_centre,
        "layout shift reproduces the startup offset"
    );

    assert!(nav.center_scene());
    let centered = nav.camera();
    pixel(centered.project(centre).unwrap(), viewport_centre);
    assert_eq!(centered.orientation, layout_camera.orientation);
    assert_eq!(centered.focal_distance, layout_camera.focal_distance);
    assert_eq!(
        centered.orthographic_height,
        layout_camera.orthographic_height
    );
    assert_eq!(centered.viewport, viewport);
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
fn unphased_zoom_wheel_and_pan_can_follow_each_other_without_capture() {
    let start = (400.0, 300.0);
    let mut nav = nav();
    for _ in 0..3 {
        assert_eq!(
            nav.advance_trackpad_pan_scroll((0.0, 10.0), TouchPhase::Moved, start, 1.0, true),
            None
        );
        assert!(nav.zoom(
            start,
            0.1,
            Some((ZoomSource::Scroll, TouchPhase::Moved)),
            true
        ));
        assert!(nav.zoom(start, -0.1, None, true));
        assert!(nav.begin_secondary_pan(start, 1.0, true));
        assert!(nav.advance_pointer((420.0, 310.0)));
        nav.release(DragKind::SecondaryPan);
        assert_eq!(
            nav.advance_trackpad_pan_scroll((0.0, 0.0), TouchPhase::Started, start, 1.0, true),
            Some(false)
        );
        assert_eq!(
            nav.advance_trackpad_pan_scroll((5.0, -2.0), TouchPhase::Moved, start, 1.0, true),
            Some(true)
        );
        nav.advance_trackpad_pan_scroll((0.0, 0.0), TouchPhase::Ended, start, 1.0, true);
        assert!(nav.zoom(start, -0.1, None, true));
    }
}

#[test]
fn unphased_zoom_recovers_after_cancel_ui_rejection_and_mouse_drag() {
    let start = (400.0, 300.0);
    for source in [ZoomSource::Scroll, ZoomSource::Pinch] {
        let mut nav = nav();
        let stream = Some((source, TouchPhase::Moved));
        assert!(nav.zoom(start, 0.1, stream, true));
        nav.cancel();
        assert!(nav.zoom(start, 0.1, stream, true));
        let before = nav.camera();
        assert!(!nav.zoom(start, 0.1, stream, false));
        assert_eq!(nav.camera(), before);
        assert!(nav.zoom(start, -0.1, stream, true));
        assert!(nav.begin_secondary_pan(start, 1.0, true));
        assert!(!nav.zoom(start, 0.1, stream, true));
        nav.release(DragKind::SecondaryPan);
        assert!(nav.zoom(start, -0.1, stream, true));
        assert!(nav.zoom(start, 0.1, None, true));
    }
}

#[test]
fn phased_zoom_keeps_exclusivity_and_recovers_after_end_or_cancelled_tail() {
    let start = (400.0, 300.0);
    for source in [ZoomSource::Scroll, ZoomSource::Pinch] {
        for terminal in [TouchPhase::Ended, TouchPhase::Cancelled] {
            let mut nav = nav();
            nav.zoom(start, 0.0, Some((source, TouchPhase::Started)), true);
            assert!(nav.zoom(start, 0.1, Some((source, TouchPhase::Moved)), true));
            assert!(!nav.zoom(start, 0.1, None, true));
            assert!(!nav.begin_secondary_pan(start, 1.0, true));
            nav.cancel();
            let before = nav.camera();
            assert!(!nav.zoom(start, 0.1, Some((source, TouchPhase::Moved)), true));
            assert!(!nav.zoom(start, 0.1, Some((source, terminal)), true));
            assert_eq!(nav.camera(), before);
            assert!(nav.zoom(start, 0.1, Some((source, TouchPhase::Moved)), true));
            assert!(nav.zoom(start, -0.1, None, true));
            assert!(nav.begin_secondary_pan(start, 1.0, true));
        }
    }
}

#[test]
fn secondary_scroll_terminal_releases_rejection_for_next_unphased_zoom() {
    let start = (400.0, 300.0);
    for terminal in [TouchPhase::Ended, TouchPhase::Cancelled] {
        let mut nav = nav();
        assert!(nav.begin_secondary_pan(start, 1.0, true));
        assert_eq!(
            nav.advance_secondary_pan_scroll((10.0, 0.0), TouchPhase::Started, start),
            Some(true)
        );
        nav.release(DragKind::SecondaryPan);
        assert_eq!(
            nav.advance_secondary_pan_scroll((5.0, 0.0), TouchPhase::Moved, start),
            Some(false)
        );
        assert_eq!(
            nav.advance_secondary_pan_scroll((0.0, 0.0), terminal, start),
            Some(false)
        );
        assert_eq!(
            nav.advance_secondary_pan_scroll((0.0, 10.0), TouchPhase::Moved, start),
            None
        );
        assert!(nav.zoom(
            start,
            0.1,
            Some((ZoomSource::Scroll, TouchPhase::Moved)),
            true
        ));
        assert!(nav.zoom(start, -0.1, None, true));
    }
}

#[test]
fn unphased_secondary_scroll_does_not_hold_next_scroll_after_release_or_cancel() {
    let start = (400.0, 300.0);
    for cancel in [false, true] {
        let mut nav = nav();
        assert!(nav.begin_secondary_pan(start, 1.0, true));
        assert_eq!(
            nav.advance_secondary_pan_scroll((10.0, 0.0), TouchPhase::Moved, start),
            Some(true)
        );
        if cancel {
            nav.cancel();
        } else {
            nav.release(DragKind::SecondaryPan);
        }
        assert_eq!(
            nav.advance_secondary_pan_scroll((0.0, 10.0), TouchPhase::Moved, start),
            None
        );
        assert!(nav.zoom(
            start,
            0.1,
            Some((ZoomSource::Scroll, TouchPhase::Moved)),
            true
        ));
    }
}

#[test]
fn phased_button_free_trackpad_scroll_pans_fractionally_on_both_screen_axes() {
    let start = (400.25, 300.5);
    let mut trackpad = nav();
    let mut expected_pan = nav();
    let pose = trackpad.camera();
    assert_eq!(
        trackpad.advance_trackpad_pan_scroll((0.0, 0.0), TouchPhase::Started, start, 2.0, true,),
        Some(false)
    );
    assert_eq!(trackpad.camera(), pose);
    assert!(expected_pan.begin(DragKind::Pan, start, 2.0, true));
    let mut offset = (0.0, 0.0);
    for (delta, pointer) in [
        ((1.25, -2.5), (401.0, 300.0)),
        ((-0.5, 3.75), (400.5, 301.0)),
    ] {
        offset.0 += delta.0;
        offset.1 += delta.1;
        assert_eq!(
            trackpad.advance_trackpad_pan_scroll(delta, TouchPhase::Moved, pointer, 2.0, true,),
            Some(true)
        );
        assert!(expected_pan.navigate((start.0 + offset.0, start.1 + offset.1)));
        assert_eq!(trackpad.camera(), expected_pan.camera());
        assert_eq!(trackpad.camera().orientation, pose.orientation);
    }
    assert_ne!(trackpad.camera(), pose);
    assert_eq!(
        trackpad.advance_trackpad_pan_scroll((0.0, 0.0), TouchPhase::Ended, start, 2.0, true,),
        Some(false)
    );
    assert!(!trackpad.is_dragging());
}

#[test]
fn trackpad_repeated_start_preserves_capture_anchor_and_accumulated_displacement() {
    let start = (400.25, 300.5);
    for scale in [1.0, 2.0] {
        let mut trackpad = nav();
        let mut expected = nav();
        assert!(expected.begin(DragKind::Pan, start, scale, true));
        let mut offset = (0.0, 0.0);
        // macOS MayBegin and Began both arrive as Started, possibly with a delta.
        for (phase, delta, pointer) in [
            (TouchPhase::Started, (0.0, 0.0), start),
            (TouchPhase::Started, (1.25, -2.5), (410.0, 310.0)),
            (TouchPhase::Moved, (2.0, 3.0), start),
            (TouchPhase::Started, (-0.5, 1.0), (420.0, 320.0)),
            (TouchPhase::Moved, (4.0, -1.0), start),
        ] {
            let changed = trackpad.advance_trackpad_pan_scroll(delta, phase, pointer, scale, true);
            offset.0 += delta.0;
            offset.1 += delta.1;
            let expected_changed = expected.navigate((start.0 + offset.0, start.1 + offset.1));
            assert_eq!(changed, Some(expected_changed));
            assert_eq!(trackpad.camera(), expected.camera());
            assert!(trackpad.is_dragging());
        }
        assert_eq!(
            trackpad.advance_trackpad_pan_scroll((0.0, 0.0), TouchPhase::Ended, start, scale, true),
            Some(false)
        );
        assert!(!trackpad.is_dragging());
        // Ending the stream must still allow a fresh capture.
        trackpad.advance_trackpad_pan_scroll((0.0, 0.0), TouchPhase::Started, start, scale, true);
        assert_eq!(
            trackpad.advance_trackpad_pan_scroll((1.0, 0.0), TouchPhase::Moved, start, scale, true),
            Some(true)
        );
    }
}

#[test]
fn trackpad_cursor_events_do_not_move_camera_between_scroll_events() {
    let start = (400.25, 300.5);
    let mut trackpad = nav();
    let mut expected = nav();
    assert!(expected.begin(DragKind::Pan, start, 2.0, true));
    let mut offset = (0.0, 0.0);
    for (phase, delta, pointer) in [
        (TouchPhase::Started, (0.0, 0.0), start),
        (TouchPhase::Moved, (1.25, -2.5), start),
        (TouchPhase::Moved, (2.0, 3.0), (410.0, 310.0)),
        (TouchPhase::Ended, (0.5, -1.0), start),
    ] {
        // winit emits CursorMoved before each macOS scroll event, even if stationary.
        let before = trackpad.camera();
        assert!(!trackpad.advance_pointer(pointer));
        assert_eq!(trackpad.camera(), before);
        let changed = trackpad.advance_trackpad_pan_scroll(delta, phase, pointer, 2.0, true);
        offset.0 += delta.0;
        offset.1 += delta.1;
        let expected_changed = expected.navigate((start.0 + offset.0, start.1 + offset.1));
        assert_eq!(changed, Some(expected_changed));
        assert_eq!(trackpad.camera(), expected.camera());
    }
    assert!(!trackpad.is_dragging());
}

#[test]
fn trackpad_scroll_ownership_stays_with_its_start_and_cancels_cleanly() {
    let start = (400.0, 300.0);
    let mut camera_nav = nav();
    // UI scroll routing rejects the start before any scene camera change.
    camera_nav.reject_trackpad_pan_scroll(TouchPhase::Started);
    let before = camera_nav.camera();
    assert_eq!(
        camera_nav.advance_trackpad_pan_scroll((8.0, 3.0), TouchPhase::Moved, start, 1.0, true),
        Some(false)
    );
    assert_eq!(camera_nav.camera(), before);
    camera_nav.reject_trackpad_pan_scroll(TouchPhase::Ended);
    assert_eq!(
        camera_nav.advance_trackpad_pan_scroll((0.0, 0.0), TouchPhase::Moved, start, 1.0, true),
        None
    );

    assert_eq!(
        camera_nav.advance_trackpad_pan_scroll((0.0, 0.0), TouchPhase::Started, start, 1.0, true),
        Some(false)
    );
    assert_eq!(
        camera_nav.advance_trackpad_pan_scroll((8.0, 3.0), TouchPhase::Moved, start, 1.0, true),
        Some(true)
    );
    let moved = camera_nav.camera();
    camera_nav.cancel(); // Focus loss, pointer exit, layout and scene changes use this cancellation path.
    assert_eq!(
        camera_nav.advance_trackpad_pan_scroll((5.0, 5.0), TouchPhase::Moved, start, 1.0, true),
        Some(false)
    );
    assert_eq!(camera_nav.camera(), moved);
    camera_nav.reject_trackpad_pan_scroll(TouchPhase::Ended);
    assert_eq!(
        camera_nav.advance_trackpad_pan_scroll((0.0, 0.0), TouchPhase::Moved, start, 1.0, true),
        None
    );

    let mut occluded = nav();
    occluded.set_layout(viewport(), vec![SceneViewport::new(380, 200, 100, 200)]);
    let before = occluded.camera();
    assert_eq!(
        occluded.advance_trackpad_pan_scroll((0.0, 0.0), TouchPhase::Started, start, 1.0, true),
        Some(false)
    );
    assert_eq!(
        occluded.advance_trackpad_pan_scroll(
            (8.0, 3.0),
            TouchPhase::Moved,
            (300.0, 300.0),
            1.0,
            true,
        ),
        Some(false)
    );
    assert_eq!(occluded.camera(), before);
    occluded.reject_trackpad_pan_scroll(TouchPhase::Ended);
}

#[test]
fn unstarted_pixel_scroll_remains_wheel_zoom_and_pinch_is_unchanged() {
    let point = (400.0, 300.0);
    let mut nav = nav();
    assert_eq!(
        nav.advance_trackpad_pan_scroll((0.5, -0.25), TouchPhase::Moved, point, 2.0, true),
        None
    );
    let before_wheel = nav.camera();
    assert!(nav.zoom(
        point,
        0.01,
        Some((ZoomSource::Scroll, TouchPhase::Moved)),
        true,
    ));
    assert_ne!(nav.camera(), before_wheel);
    assert!(!nav.zoom(
        point,
        0.0,
        Some((ZoomSource::Scroll, TouchPhase::Ended)),
        true,
    ));
    let after_wheel = nav.camera();
    assert!(!nav.zoom(
        point,
        0.0,
        Some((ZoomSource::Pinch, TouchPhase::Started)),
        true,
    ));
    assert!(nav.zoom(
        point,
        0.5,
        Some((ZoomSource::Pinch, TouchPhase::Moved)),
        true,
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

#[test]
fn origin_references_never_enter_model_bounds_fitting_or_picking() {
    let mut empty = Navigation::new(camera());
    empty.set_layout(viewport(), vec![]);
    for perspective in [false, true] {
        empty.set_perspective(perspective);
        assert!(empty.snapshot().is_some());
        assert!(empty.scene().bounds().is_none());
        assert!(empty.pick((400., 300.)).is_none());
        assert!(!empty.fit());
    }
    let mut n = nav();
    let bounds = n.scene().bounds();
    for perspective in [false, true] {
        n.set_perspective(perspective);
        for steps in [-8., 4., 12.] {
            n.zoom((400., 300.), steps, None, true);
            let before = n.camera();
            let revision = n.scene().revision();
            assert!(n.snapshot().is_some());
            assert_eq!(n.camera(), before);
            assert_eq!(n.scene().bounds(), bounds);
            assert_eq!(n.scene().revision(), revision);
        }
        n.fit();
        close(n.camera().target(), [0.5; 3]);
        assert_eq!(n.scene().bounds(), bounds);
    }
}
