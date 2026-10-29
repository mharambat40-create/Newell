// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M5.0 exact profile probes; deliberately no polygon substitute for curves.
use super::sketch_solver::{ProbeResult, cancel_boundary, distribution};
use brepkit_math::vec::{Point3, Vec3};
use brepkit_operations::{
    extrude::extrude,
    measure::solid_volume,
    offset_wire::{JoinType, offset_wire_with_join},
    validate::validate_solid,
};
use brepkit_topology::{
    FaceId, Topology,
    builder::{make_circle_edge, make_polygon_wire},
    face::{Face, FaceSurface},
    wire::{OrientedEdge, Wire},
};
use std::{hint::black_box, time::Instant};
fn holed(shift: f64, flipped: bool) -> Result<(Topology, FaceId), Box<dyn std::error::Error>> {
    let mut t = Topology::new();
    let outer = make_polygon_wire(
        &mut t,
        &[[0., 0.], [10., 0.], [10., 10.], [0., 10.]]
            .map(|p| Point3::new(p[0] + shift, p[1] + shift, 0.)),
        1e-7,
    )?;
    let edge = make_circle_edge(
        &mut t,
        Point3::new(shift + 5., shift + 5., 0.),
        Vec3::new(0., 0., 1.),
        0.1,
        1e-7,
    )?;
    let hole = t.add_wire(Wire::new(vec![OrientedEdge::new(edge, false)], true)?);
    let face = t.add_face(Face::new(
        outer,
        vec![hole],
        FaceSurface::Plane {
            normal: Vec3::new(0., 0., if flipped { -1. } else { 1. }),
            d: 0.,
        },
    ));
    Ok((t, face))
}
pub(crate) fn run(iterations: usize) -> ProbeResult {
    advanced_profiles()?;
    for shift in [0., 1e9] {
        for flipped in [false, true] {
            let mut times = Vec::new();
            let mut setups = Vec::new();
            let mut measures = Vec::new();
            let mut validations = Vec::new();
            let mut max_error = 0_f64;
            let mut invalid = 0;
            for _ in 0..iterations {
                let begin = Instant::now();
                let (mut t, face) = holed(shift, flipped)?;
                setups.push(begin.elapsed().as_secs_f64() * 1e6);
                let start = Instant::now();
                let solid = extrude(&mut t, face, Vec3::new(0., 0., 1.), 2.)?;
                times.push(start.elapsed().as_secs_f64() * 1e6);
                let phase = Instant::now();
                let volume = solid_volume(&t, solid, 1e-5)?;
                measures.push(phase.elapsed().as_secs_f64() * 1e6);
                let expected = (100. - std::f64::consts::PI * 0.01) * 2.;
                max_error = max_error.max((volume - expected).abs());
                let phase = Instant::now();
                let valid = validate_solid(&t, solid)?.is_valid();
                validations.push(phase.elapsed().as_secs_f64() * 1e6);
                if !valid {
                    invalid += 1;
                }
            }
            println!(
                "EXTRUDE,exact_circular_hole,shift={shift:e},flipped={flipped},volume_abs_error={max_error:e},invalid={invalid},tolerance_mm=1e-7"
            );
            distribution(&format!("native_holed_extrude_{shift:e}_{flipped}"), times);
            distribution(&format!("wire_face_setup_{shift:e}_{flipped}"), setups);
            distribution(&format!("native_volume_{shift:e}_{flipped}"), measures);
            distribution(&format!("native_validate_{shift:e}_{flipped}"), validations);
        }
        let (mut t, face) = holed(shift, false)?;
        cancel_boundary(&format!("dependent_extrude_{shift:e}"), || {
            black_box(extrude(&mut t, face, Vec3::new(0., 0., 1.), 2.)).ok();
        })?;
    }
    cancel_boundary("wire_and_face_construction", || {
        black_box(holed(0., false)).ok();
    })?;
    for join in [JoinType::Intersection, JoinType::Arc, JoinType::Chamfer] {
        let (mut t, face) = holed(0., false)?;
        let result = offset_wire_with_join(&mut t, face, 0.2, join);
        println!(
            "OFFSET,holed_polygon,join={join:?},result={result:?},hole_and_self_intersection_certification=Unproven"
        );
        let mut t = Topology::new();
        let e = make_circle_edge(
            &mut t,
            Point3::new(0., 0., 0.),
            Vec3::new(0., 0., 1.),
            2.,
            1e-7,
        )?;
        let wire = t.add_wire(Wire::new(vec![OrientedEdge::new(e, true)], true)?);
        let face = t.add_face(Face::new(
            wire,
            vec![],
            FaceSurface::Plane {
                normal: Vec3::new(0., 0., 1.),
                d: 0.,
            },
        ));
        let result = offset_wire_with_join(&mut t, face, 0.2, join);
        println!(
            "OFFSET,exact_periodic_circle,join={join:?},result={result:?},analytic_expected_radius=2.2"
        );
        if let Ok(wire) = result {
            let mut lines = 0;
            let mut circles = 0;
            let mut radial_error = 0_f64;
            let mut chord_error = 0_f64;
            for oe in t.wire(wire)?.edges() {
                let edge = t.edge(oe.edge())?;
                let a = t.vertex(edge.start())?.point();
                let b = t.vertex(edge.end())?.point();
                radial_error = radial_error.max((a.x().hypot(a.y()) - 2.2).abs());
                match edge.curve() {
                    brepkit_topology::edge::EdgeCurve::Line => {
                        lines += 1;
                        chord_error = chord_error
                            .max((((a.x() + b.x()) / 2.).hypot((a.y() + b.y()) / 2.) - 2.2).abs());
                    }
                    brepkit_topology::edge::EdgeCurve::Circle(_) => circles += 1,
                    _ => {}
                }
            }
            println!(
                "OFFSET_ACCURACY,circle,join={join:?},lines={lines},circles={circles},endpoint_radius_error={radial_error:e},chord_midpoint_radius_error={chord_error:e},certified_bound=Unavailable"
            );
        }
    }
    // Existing public profile seam used for exact cubic M4 feasibility, not a full app command.
    let profile =
        newell_kernel::NewellProfile::closed_cubic([[0., 0.], [2., 3.], [4., 2.], [6., 0.]])?;
    let t = Instant::now();
    let body = profile.extrude([0., 0., 1.], 4.)?;
    let volume = body.volume()?;
    assert!((volume - 30.).abs() < 1e-6);
    body.validate()?;
    println!(
        "M4_PROFILE,cubic,volume={volume},expected=30,us={:.3},full_app_path=Unproven",
        t.elapsed().as_secs_f64() * 1e6
    );
    // Real M4 arena entry point, using its actual face-copy/transform/validation path.
    let mut arena = newell_kernel::NewellGeometry::new();
    let source = arena.box_shape([2., 3., 4.], [0., 0., 0.])?;
    let faces = arena.profile_faces(source)?;
    for face in faces {
        let result = arena.extrude_face(source, face.face, None, 0., 2.);
        println!(
            "M4_ARENA,face={},result={:?}",
            face.face,
            result.as_ref().map(|key| arena.volume(*key))
        );
    }
    Ok(())
}
#[test]
fn exact_cubic_profile_extrusion_matches_independent_integral() -> ProbeResult {
    let p = newell_kernel::NewellProfile::closed_cubic([[0., 0.], [2., 3.], [4., 2.], [6., 0.]])?;
    assert!((p.area()? - 7.5).abs() < 1e-7);
    let body = p.extrude([0., 0., 1.], 4.)?;
    assert!((body.volume()? - 30.).abs() < 1e-6);
    body.validate()?;
    Ok(())
}

#[test]
fn profile_capability_dossier_executes() -> ProbeResult {
    run(1)
}

fn advanced_profiles() -> ProbeResult {
    use brepkit_math::nurbs::curve::NurbsCurve;
    use brepkit_topology::edge::{Edge, EdgeCurve};
    let w = std::f64::consts::FRAC_1_SQRT_2;
    for (name, degree, controls, weights, knots, expected) in [
        (
            "ellipse_periodic",
            2,
            vec![
                [2., 0.],
                [2., 1.],
                [0., 1.],
                [-2., 1.],
                [-2., 0.],
                [-2., -1.],
                [0., -1.],
                [2., -1.],
                [2., 0.],
            ],
            vec![1., w, 1., w, 1., w, 1., w, 1.],
            vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
            2. * std::f64::consts::PI,
        ),
        (
            "rational_quadratic_arc",
            2,
            vec![[1., 0.], [1., 1.], [0., 1.]],
            vec![1., w, 1.],
            vec![0., 0., 0., 1., 1., 1.],
            std::f64::consts::PI / 4. - 0.5,
        ),
        (
            "parabolic_conic",
            2,
            vec![[-1., 1.], [0., -1.], [1., 1.]],
            vec![1.; 3],
            vec![0., 0., 0., 1., 1., 1.],
            4. / 3.,
        ),
        (
            "cubic_bezier",
            3,
            vec![[0., 0.], [2., 3.], [4., 2.], [6., 0.]],
            vec![1.; 4],
            vec![0., 0., 0., 0., 1., 1., 1., 1.],
            7.5,
        ),
    ] {
        for shift in [0., 1e9] {
            let mut t = Topology::new();
            let curve = NurbsCurve::new(
                degree,
                knots.clone(),
                controls
                    .iter()
                    .map(|p| Point3::new(p[0] + shift, p[1] + shift, 0.))
                    .collect(),
                weights.clone(),
            )?;
            let e = brepkit_topology::builder::make_nurbs_edge_from_curve(&mut t, &curve, 1e-7);
            let edge = t.edge(e)?;
            let mut edges = vec![OrientedEdge::new(e, true)];
            if edge.start() != edge.end() {
                let chord = Edge::new(edge.end(), edge.start(), EdgeCurve::Line);
                let id = t.add_edge(chord);
                edges.push(OrientedEdge::new(id, true));
            }
            let wire = t.add_wire(Wire::new(edges, true)?);
            let face = t.add_face(Face::new(
                wire,
                vec![],
                FaceSurface::Plane {
                    normal: Vec3::new(0., 0., 1.),
                    d: 0.,
                },
            ));
            for join in [JoinType::Intersection, JoinType::Arc, JoinType::Chamfer] {
                let offset = offset_wire_with_join(&mut t, face, 0.2, join);
                println!(
                    "OFFSET_ADVANCED,{name},shift={shift:e},join={join:?},result={offset:?},certified_error_bound=Unavailable"
                );
            }
            let time = Instant::now();
            let result = extrude(&mut t, face, Vec3::new(0., 0., 1.), 2.);
            match result {
                Ok(solid) => {
                    let measured = solid_volume(&t, solid, 1e-5)?;
                    let valid = validate_solid(&t, solid)?.is_valid();
                    println!(
                        "ADVANCED_EXTRUDE,{name},shift={shift:e},expected_volume={},native_volume={measured},error={:e},valid={valid},us={:.3}",
                        2. * expected,
                        (measured - 2. * expected).abs(),
                        time.elapsed().as_secs_f64() * 1e6
                    );
                }
                Err(e) => println!("ADVANCED_EXTRUDE,{name},shift={shift:e},error={e:?}"),
            }
        }
    }
    // Three nested loops: the default bounded outer cell has only its immediate
    // hole; the depth-two island is a separate selectable material region.
    for shift in [0., 1e9] {
        let mut t = Topology::new();
        let square = |t: &mut Topology, lo: f64, hi: f64| {
            make_polygon_wire(
                t,
                &[
                    Point3::new(lo + shift, lo + shift, 0.),
                    Point3::new(hi + shift, lo + shift, 0.),
                    Point3::new(hi + shift, hi + shift, 0.),
                    Point3::new(lo + shift, hi + shift, 0.),
                ],
                1e-7,
            )
        };
        let outer = square(&mut t, 0., 10.)?;
        let hole = square(&mut t, 2., 8.)?;
        let island = square(&mut t, 4., 6.)?;
        let outerface = t.add_face(Face::new(
            outer,
            vec![hole],
            FaceSurface::Plane {
                normal: Vec3::new(0., 0., 1.),
                d: 0.,
            },
        ));
        let islandface = t.add_face(Face::new(
            island,
            vec![],
            FaceSurface::Plane {
                normal: Vec3::new(0., 0., 1.),
                d: 0.,
            },
        ));
        let a = extrude(&mut t, outerface, Vec3::new(0., 0., 1.), 2.)?;
        let b = extrude(&mut t, islandface, Vec3::new(0., 0., 1.), 2.)?;
        println!(
            "NESTING,manual_exact_faces,shift={shift:e},outer_expected=128,outer_volume={},island_expected=8,island_volume={},automatic_arrangement=Unproven",
            solid_volume(&t, a, 1e-5)?,
            solid_volume(&t, b, 1e-5)?
        );
    }
    Ok(())
}

#[cfg(feature = "m50-feasibility")]
#[test]
fn vendor_candidate_circle_offset_reaches_exact_topology() -> ProbeResult {
    use brepkit_math::curves::Circle3D;
    use brepkit_operations::certified_offset::{OffsetFailure, circle};
    for shift in [0., 1e9] {
        let source = Circle3D::new(Point3::new(shift, shift, 0.), Vec3::new(0., 0., 1.), 2.)?;
        for distance in [-0.25, 0.25] {
            let result =
                circle(&source, distance, 1, false).map_err(|e| format!("offset: {e:?}"))?;
            let radius = 2. + distance;
            assert!(result.radius_enclosure[0] <= radius && result.radius_enclosure[1] >= radius);
            assert_eq!(result.circle.u_axis(), source.u_axis());
            assert_eq!(result.circle.v_axis(), source.v_axis());
            let mut t = Topology::new();
            let e = make_circle_edge(
                &mut t,
                result.circle.center(),
                result.circle.normal(),
                result.circle.radius(),
                1e-7,
            )?;
            let w = t.add_wire(Wire::new(vec![OrientedEdge::new(e, true)], true)?);
            let f = t.add_face(Face::new(
                w,
                vec![],
                FaceSurface::Plane {
                    normal: Vec3::new(0., 0., 1.),
                    d: 0.,
                },
            ));
            let solid = extrude(&mut t, f, Vec3::new(0., 0., 1.), 2.)?;
            let expected = std::f64::consts::PI * radius * radius * 2.;
            assert!((solid_volume(&t, solid, 1e-5)? - expected).abs() < 1e-5);
            assert!(validate_solid(&t, solid)?.is_valid());
        }
        assert_eq!(
            circle(&source, 0., 0, false).err(),
            Some(OffsetFailure::BudgetExceeded)
        );
        assert_eq!(
            circle(&source, 0., 1, true).err(),
            Some(OffsetFailure::Cancelled)
        );
        assert_eq!(
            circle(&source, -2., 1, false).err(),
            Some(OffsetFailure::Invalid)
        );
    }
    Ok(())
}
