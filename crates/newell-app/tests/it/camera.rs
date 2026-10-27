// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

#![allow(clippy::unwrap_used)]

use newell_app::camera::{
    Camera, MIN_DISTANCE, POLE_MARGIN, Quaternion, Ray, multiply, plane_hit, transform,
};
use newell_render::SceneViewport;
use proptest::prelude::*;

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn scale(a: [f64; 3], k: f64) -> [f64; 3] {
    a.map(|v| v * k)
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn close(a: [f64; 3], b: [f64; 3]) {
    assert!(norm(sub(a, b)) < 1e-9, "{a:?} != {b:?}");
}
fn pixel(a: (f64, f64), b: (f64, f64)) {
    assert!(
        (a.0 - b.0).abs() < 1e-7 && (a.1 - b.1).abs() < 1e-7,
        "{a:?} != {b:?}"
    );
}
fn camera() -> Camera {
    Camera::look_at(
        [3.4, 3.0, 3.2],
        [0.5; 3],
        SceneViewport::new(20, 10, 800, 600),
    )
}

#[test]
fn default_camera_uses_orthographic_projection() {
    assert!(!Camera::default().perspective);
}

fn level(camera: Camera) {
    let (r, u, f) = camera.basis();
    for v in [r, u, f] {
        assert!((norm(v) - 1.0).abs() < 1e-12);
    }
    assert!(dot(r, u).abs() < 1e-12 && dot(u, f).abs() < 1e-12 && dot(r, f).abs() < 1e-12);
    assert!(r[2].abs() < 1e-12);
    assert!(u[2] >= 0.0);
    let projected_up = sub([0.0, 0.0, 1.0], scale(f, f[2]));
    close(u, scale(projected_up, 1.0 / norm(projected_up)));
    assert!((camera.orientation.norm() - 1.0).abs() < 1e-12);
}

#[test]
fn right_and_up_orbit_are_direct_and_zero_displacement_is_exact() {
    let c = camera();
    let pivot = c.target();
    let front = sub(pivot, scale(c.basis().2, 0.4));
    let before = c.project(front).unwrap();
    let right = c.orbit(pivot, (40.0, 0.0), 1.0);
    let up = c.orbit(pivot, (0.0, -40.0), 1.0);
    assert!(right.project(front).unwrap().0 > before.0);
    assert!(up.project(front).unwrap().1 < before.1);
    assert_eq!(c, c.orbit(pivot, (0.0, 0.0), 1.0));
    level(right);
    level(up);
}

#[test]
fn rotation_is_display_scale_invariant_and_uses_screen_axes() {
    let c = camera();
    let p = c.target();
    let a = c.orbit(p, (90.0, 30.0), 1.0);
    let b = c.orbit(p, (180.0, 60.0), 2.0);
    close(a.eye, b.eye);
    close(a.basis().2, b.basis().2);
    // Looking from an unrelated world direction still follows screen right/up.
    let c = Camera::look_at([-4.0, 1.0, 2.0], [0.0; 3], c.viewport);
    let front = sub(c.target(), scale(c.basis().2, 0.5));
    let original = c.project(front).unwrap();
    assert!(
        c.orbit(c.target(), (50.0, 0.0), 1.0)
            .project(front)
            .unwrap()
            .0
            > original.0
    );
    assert!(
        c.orbit(c.target(), (0.0, -50.0), 1.0)
            .project(front)
            .unwrap()
            .1
            < original.1
    );
}

#[test]
fn off_centre_pivot_is_fixed_in_camera_space_after_horizon_correction() {
    let c = camera();
    let pivot = plane_hit(c.ray((650.0, 180.0)).unwrap(), c.target(), c.basis().2).unwrap();
    let before = c.orientation.inverse().rotate(sub(pivot, c.eye));
    let rotated = c.orbit(pivot, (180.0, -85.0), 1.0);
    close(
        before,
        rotated
            .orientation
            .inverse()
            .rotate(sub(pivot, rotated.eye)),
    );
    pixel(c.project(pivot).unwrap(), rotated.project(pivot).unwrap());
    assert!((norm(sub(c.eye, pivot)) - norm(sub(rotated.eye, pivot))).abs() < 1e-12);
    level(rotated);
}

#[test]
fn arbitrarily_large_pitch_stops_before_poles_without_heading_fold() {
    let c = camera();
    for dy in [-1e8, -4000.0, -1000.0, 1000.0, 4000.0, 1e8] {
        let next = c.orbit(c.target(), (0.0, dy), 1.0);
        level(next);
        let f = next.basis().2;
        assert!(f[2].abs() <= POLE_MARGIN.cos() + 1e-12);
        let h = [f[0], f[1], 0.0];
        assert!(dot(h, c.last_heading) > 0.0, "heading flipped at {dy}");
    }
}

#[test]
fn repeated_mixed_gestures_have_no_roll_or_distance_drift() {
    let mut c = camera();
    let pivot = c.target();
    let distance = norm(sub(c.eye, pivot));
    for i in 0..5000 {
        c = c.orbit(pivot, ((i % 17) as f64 - 8.0, (i % 23) as f64 - 11.0), 1.0);
        level(c);
    }
    assert!((norm(sub(c.eye, pivot)) - distance).abs() < 1e-9);
}

#[test]
fn vertical_axis_views_are_finite_and_depart_with_a_stable_heading() {
    for eye in [[0.0, 0.0, 5.0], [0.0, 0.0, -5.0]] {
        let mut c = Camera::look_at(eye, [0.0; 3], camera().viewport);
        c.normal_to_axis();
        assert!((c.basis().2[2].abs() - 1.0).abs() < 1e-12);
        for delta in [(1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            let moved = c.orbit([0.0; 3], delta, 1.0);
            level(moved);
            assert!(moved.eye.iter().all(|v| v.is_finite()));
        }
    }
}

#[test]
fn quaternion_composition_matches_vector_rotation_and_inverse() {
    let a = Quaternion::axis_angle([0.0, 0.0, 1.0], 0.7);
    let b = Quaternion::axis_angle([1.0, 0.0, 0.0], -0.3);
    let v = [2.0, -1.0, 0.5];
    close(a.compose(b).rotate(v), a.rotate(b.rotate(v)));
    close(a.inverse().rotate(a.rotate(v)), v);
}

#[test]
fn view_matrix_maps_eye_to_origin_and_forward_to_negative_z() {
    let c = camera();
    let e = transform(c.view(), [c.eye[0], c.eye[1], c.eye[2], 1.0]);
    close([e[0], e[1], e[2]], [0.0; 3]);
    let p = c.target();
    let t = transform(c.view(), [p[0], p[1], p[2], 1.0]);
    close([t[0], t[1], t[2]], [0.0, 0.0, -c.focal_distance]);
}

#[test]
fn project_ray_round_trip_supports_both_projections_and_shifted_lenses() {
    for perspective in [true, false] {
        let mut c = camera();
        c.set_perspective(perspective);
        c.lens_shift = [0.35, -0.2];
        for point in [(35.5, 20.25), (640.25, 500.75), (419.5, 309.5)] {
            let ray = c.ray(point).unwrap();
            let world = add(ray.origin, scale(ray.direction, 7.0));
            pixel(c.project(world).unwrap(), point);
            let world = plane_hit(ray, c.target(), c.basis().2).unwrap();
            pixel(c.project(world).unwrap(), point);
        }
        let a = c.ray((100.0, 100.0)).unwrap();
        let b = c.ray((700.0, 500.0)).unwrap();
        if perspective {
            close(a.origin, b.origin);
            assert!(norm(sub(a.direction, b.direction)) > 0.1);
        } else {
            close(a.direction, b.direction);
            assert!(norm(sub(a.origin, b.origin)) > 0.1);
        }
    }
}

#[test]
fn projection_maps_near_and_far_to_wgpu_depth_range() {
    for perspective in [true, false] {
        let mut c = camera();
        c.set_perspective(perspective);
        let p = c.projection(0.03, 120.0);
        for (depth, expected) in [(0.03, 0.0), (120.0, 1.0)] {
            let v = transform(p, [0.0, 0.0, -depth, 1.0]);
            assert!((v[2] / v[3] - expected).abs() < 1e-12);
        }
    }
}

#[test]
fn resize_preserves_pixels_for_all_depths_in_both_projections() {
    for perspective in [true, false] {
        let mut c = camera();
        c.set_perspective(perspective);
        c.lens_shift = [0.2, -0.1];
        let original = c;
        c.resize(SceneViewport::new(80, 45, 500, 720));
        assert_eq!(c.eye, original.eye);
        assert_eq!(c.orientation, original.orientation);
        for p in [[0.0; 3], [1.0; 3], [0.2, 0.8, 0.4]] {
            pixel(c.project(p).unwrap(), original.project(p).unwrap());
        }
        c.resize(original.viewport);
        for p in [[0.0; 3], [1.0; 3]] {
            pixel(c.project(p).unwrap(), original.project(p).unwrap());
        }
    }
}

#[test]
fn pan_matches_pointer_motion_and_scales_with_depth_and_height() {
    let c = camera();
    let start = (620.0, 280.0);
    let end = (1000.0, -50.0);
    let anchor = plane_hit(c.ray(start).unwrap(), c.target(), c.basis().2).unwrap();
    let moved = c.pan(start, end, c.focal_distance);
    pixel(moved.project(anchor).unwrap(), end);
    assert_eq!(moved.orientation, c.orientation);
    let a = sub(moved.eye, c.eye);
    let b = sub(c.pan(start, end, c.focal_distance * 2.0).eye, c.eye);
    close(b, scale(a, 2.0));
    let mut big = c;
    big.viewport = SceneViewport::new(20, 10, 1600, 1200);
    close(
        sub(big.pan(start, end, c.focal_distance).eye, big.eye),
        scale(a, 0.5),
    );
}

#[test]
fn zoom_keeps_surface_anchor_at_same_pixel_in_both_projections() {
    for perspective in [true, false] {
        let mut c = camera();
        c.set_perspective(perspective);
        c.lens_shift = [0.2, -0.1];
        let point = (570.0, 270.0);
        let anchor = plane_hit(
            c.ray(point).unwrap(),
            add(c.eye, scale(c.basis().2, 2.0)),
            c.basis().2,
        )
        .unwrap();
        let next = c.zoom(anchor, point, 2.0, 100.0);
        pixel(next.project(anchor).unwrap(), point);
        assert_eq!(next.orientation, c.orientation);
        if !perspective {
            assert!((dot(sub(next.eye, c.eye), c.basis().2)).abs() < 1e-12);
        }
    }
}

#[test]
fn zoom_enforces_limits_and_fractional_partition_is_equivalent() {
    let c = camera();
    let point = (550.0, 270.0);
    let anchor = plane_hit(c.ray(point).unwrap(), c.target(), c.basis().2).unwrap();
    for (steps, expected) in [(1e8, MIN_DISTANCE), (-1e8, 20.0)] {
        let next = c.zoom(anchor, point, steps, 20.0);
        assert!((next.focal_distance - expected).abs() < 1e-9);
        pixel(next.project(anchor).unwrap(), point);
    }
    let whole = c.zoom(anchor, point, 1.0, 20.0);
    let mut split = c;
    for _ in 0..100 {
        split = split.zoom(anchor, point, 0.01, 20.0);
    }
    close(whole.eye, split.eye);
    assert!((whole.focal_distance - split.focal_distance).abs() < 1e-12);
}

#[test]
fn projection_toggle_keeps_focal_plane_scale_and_normal_view_keeps_target() {
    let mut c = camera();
    let point = plane_hit(c.ray((550.0, 250.0)).unwrap(), c.target(), c.basis().2).unwrap();
    let before = c.project(point).unwrap();
    let eye = c.eye;
    c.set_perspective(false);
    pixel(before, c.project(point).unwrap());
    assert_eq!(eye, c.eye);
    c.set_perspective(true);
    pixel(before, c.project(point).unwrap());
    let target = c.target();
    c.normal_to_axis();
    close(target, c.target());
    assert!(c.basis().2.iter().any(|v| v.abs() > 1.0 - 1e-12));
}

#[test]
fn invalid_input_and_nonfinite_uniform_conversion_are_rejected() {
    let c = camera();
    assert!(c.ray((f64::NAN, 0.0)).is_none());
    assert_eq!(c, c.orbit(c.target(), (f64::INFINITY, 0.0), 1.0));
    assert_eq!(c, c.pan((0.0, 0.0), (f64::NAN, 0.0), 1.0));
    assert_eq!(c, c.zoom(c.target(), (420.0, 310.0), f64::NAN, 20.0));
    let invalid = Camera {
        eye: [f64::NAN; 3],
        ..c
    };
    assert!(invalid.snapshot(([0.5; 3], 1.0), 1).is_none());
    let zero = Camera {
        viewport: SceneViewport::default(),
        ..c
    };
    assert!(zero.ray((0.0, 0.0)).is_none());
    assert!(
        plane_hit(
            Ray {
                origin: [0.0; 3],
                direction: [1.0, 0.0, 0.0]
            },
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0]
        )
        .is_none()
    );
}

#[test]
fn sphere_fit_is_model_scale_and_aspect_aware() {
    let c = camera();
    let narrow = Camera {
        viewport: SceneViewport::new(0, 0, 200, 600),
        ..c
    };
    assert!(narrow.fit_distance(1.0) > c.fit_distance(1.0));
    assert!((c.fit_distance(10.0) / c.fit_distance(1.0) - 10.0).abs() < 1e-12);
}

proptest! {
    #[test]
    fn quaternion_orbit_invariants_over_many_drag_directions(dx in -1500.0f64..1500.0,dy in -1500.0f64..1500.0,px in -2.0f64..2.0,py in -2.0f64..2.0){
        let c=camera();let pivot=[px,py,0.5];let next=c.orbit(pivot,(dx,dy),1.0);level(next);
        close(c.orientation.inverse().rotate(sub(pivot,c.eye)),next.orientation.inverse().rotate(sub(pivot,next.eye)));
        let m=multiply(next.projection(0.01,100.0),next.view());prop_assert!(m.iter().flatten().all(|v|v.is_finite()));
    }
}

#[test]
fn returning_from_zoomed_orthographic_view_preserves_focal_plane_framing() {
    let mut c = camera();
    c.set_perspective(false);
    let point = (610.0, 220.0);
    let anchor = plane_hit(c.ray(point).unwrap(), c.target(), c.basis().2).unwrap();
    c = c.zoom(anchor, point, 2.0, 100.0);
    let target = c.target();
    let reference = add(target, scale(c.basis().0, 0.3));
    let pixel_before = c.project(reference).unwrap();
    c.set_perspective(true);
    close(target, c.target());
    pixel(pixel_before, c.project(reference).unwrap());
}
