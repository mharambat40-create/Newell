// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! I0: exact pinned-kernel experiments kept inside Newell's kernel boundary.
use brepkit_math::{
    mat::Mat4,
    nurbs::curve::NurbsCurve,
    vec::{Point3, Vec3},
};
use brepkit_operations::{
    boolean::{BooleanOp, boolean, mesh_fallback_count},
    extrude::extrude,
    measure::{face_area, solid_volume},
    primitives::make_box,
    transform::transform_solid,
    validate::validate_solid,
};
use brepkit_topology::{
    FaceId, Topology,
    builder::{make_circle_edge, make_polygon_wire},
    edge::{Edge, EdgeCurve},
    face::{Face, FaceSurface},
    wire::{OrientedEdge, Wire},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const TOL: f64 = 1e-7;
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-6_f64.max(expected.abs() * 1e-6),
        "actual {actual}, expected {expected}"
    );
}
fn polygon(t: &mut Topology, points: &[[f64; 3]]) -> Result<FaceId, Box<dyn std::error::Error>> {
    let pts: Vec<_> = points
        .iter()
        .map(|p| Point3::new(p[0], p[1], p[2]))
        .collect();
    let w = make_polygon_wire(t, &pts, TOL)?;
    Ok(t.add_face(Face::new(
        w,
        vec![],
        FaceSurface::Plane {
            normal: Vec3::new(0., 0., 1.),
            d: 0.,
        },
    )))
}
fn cubic(t: &mut Topology) -> Result<FaceId, Box<dyn std::error::Error>> {
    let curve = NurbsCurve::new(
        3,
        vec![0., 0., 0., 0., 1., 1., 1., 1.],
        vec![
            Point3::new(0., 0., 0.),
            Point3::new(2., 3., 0.),
            Point3::new(4., 2., 0.),
            Point3::new(6., 0., 0.),
        ],
        vec![1.; 4],
    )?;
    let edge = brepkit_topology::builder::make_nurbs_edge_from_curve(t, &curve, TOL);
    let e = t.edge(edge)?;
    let chord = t.add_edge(Edge::new(e.end(), e.start(), EdgeCurve::Line));
    let wire = t.add_wire(Wire::new(
        vec![
            OrientedEdge::new(edge, true),
            OrientedEdge::new(chord, true),
        ],
        true,
    )?);
    Ok(t.add_face(Face::new(
        wire,
        vec![],
        FaceSurface::Plane {
            normal: Vec3::new(0., 0., 1.),
            d: 0.,
        },
    )))
}
#[test]
fn holed_planar_profile_has_exact_volume_and_valid_manifold() -> TestResult {
    let mut t = Topology::new();
    let face = polygon(
        &mut t,
        &[[0., 0., 0.], [10., 0., 0.], [10., 6., 0.], [0., 6., 0.]],
    )?;
    let hole = make_polygon_wire(
        &mut t,
        &[
            Point3::new(2., 2., 0.),
            Point3::new(2., 4., 0.),
            Point3::new(4., 4., 0.),
            Point3::new(4., 2., 0.),
        ],
        TOL,
    )?;
    t.face_mut(face)?.inner_wires_mut().push(hole);
    let solid = extrude(&mut t, face, Vec3::new(0., 0., 1.), 5.)?;
    close(solid_volume(&t, solid, 0.001)?, 280.);
    assert!(validate_solid(&t, solid)?.is_valid());
    let oblique = extrude(&mut t, face, Vec3::new(0.6, 0., 0.8), 5.)?;
    close(solid_volume(&t, oblique, 0.001)?, 224.);
    Ok(())
}
#[test]
fn circular_profile_uses_a_true_circle_not_polygon_chords() -> TestResult {
    let mut t = Topology::new();
    let edge = make_circle_edge(
        &mut t,
        Point3::new(0., 0., 0.),
        Vec3::new(0., 0., 1.),
        2.,
        TOL,
    )?;
    let w = t.add_wire(Wire::new(vec![OrientedEdge::new(edge, true)], true)?);
    let face = t.add_face(Face::new(
        w,
        vec![],
        FaceSurface::Plane {
            normal: Vec3::new(0., 0., 1.),
            d: 0.,
        },
    ));
    let solid = extrude(&mut t, face, Vec3::new(0., 0., 1.), 3.)?;
    close(solid_volume(&t, solid, 0.001)?, 12. * std::f64::consts::PI);
    assert!(validate_solid(&t, solid)?.is_valid());
    assert!(
        brepkit_topology::explorer::solid_faces(&t, solid)?
            .iter()
            .any(|f| matches!(
                t.face(*f).map(|f| f.surface()),
                Ok(FaceSurface::Cylinder(_))
            ))
    );
    Ok(())
}
#[test]
fn nonconic_cubic_profile_and_extrusion_match_independent_integrals() -> TestResult {
    let mut t = Topology::new();
    let face = cubic(&mut t)?;
    let native_area = face_area(&t, face, 0.001)?;
    println!("pinned native cubic face area: {native_area:.15}; analytic 7.5");
    let profile =
        newell_kernel::NewellProfile::closed_cubic([[0., 0.], [2., 3.], [4., 2.], [6., 0.]])?;
    close(profile.area()?, 7.5);
    let solid = extrude(&mut t, face, Vec3::new(0., 0., 1.), 4.)?;
    println!(
        "pinned native cubic solid volume: {:.15}",
        solid_volume(&t, solid, 0.001)?
    );
    close(profile.extrude([0., 0., 1.], 4.)?.volume()?, 30.);
    assert!(validate_solid(&t, solid)?.is_valid());
    assert!(
        brepkit_topology::explorer::solid_faces(&t, solid)?
            .iter()
            .any(|f| matches!(t.face(*f).map(|f| f.surface()), Ok(FaceSurface::Nurbs(_))))
    );
    Ok(())
}
#[test]
fn solid_union_cut_are_exact_and_full_removal_is_distinct() -> TestResult {
    let mut t = Topology::new();
    let a = make_box(&mut t, 10., 10., 10.)?;
    let b = make_box(&mut t, 10., 10., 10.)?;
    transform_solid(&mut t, b, &Mat4::translation(5., 0., 0.))?;
    let before = mesh_fallback_count();
    let add = boolean(&mut t, BooleanOp::Fuse, a, b)?;
    close(solid_volume(&t, add, 0.001)?, 1500.);
    let cut = boolean(&mut t, BooleanOp::Cut, a, b)?;
    close(solid_volume(&t, cut, 0.001)?, 500.);
    assert_eq!(
        mesh_fallback_count(),
        before,
        "mesh fallback cannot establish exact geometry"
    );
    assert!(validate_solid(&t, add)?.is_valid());
    assert!(validate_solid(&t, cut)?.is_valid());
    let empty = boolean(&mut t, BooleanOp::Cut, a, a);
    assert!(
        matches!(
            empty,
            Err(brepkit_operations::OperationsError::EmptyResult { .. })
        ) || empty.as_ref().is_ok_and(|id| t.is_empty_solid(*id))
    );
    Ok(())
}

#[test]
fn nonconic_sheet_overlap_retains_exact_support_and_uncapped_boundaries() -> TestResult {
    use newell_kernel::{NewellBooleanMode, NewellSurface};
    let control = [[0., 0.], [2., 3.], [4., 2.], [6., 0.]];
    let a = NewellSurface::sweep_cubic(control, 0., 3.)?;
    let b = NewellSurface::sweep_cubic(control, 1., 4.)?;
    // Independent composite Simpson oracle, using the fixture polynomial derivative.
    // For q=9-24t+9t²: |q|<=9, |q'|<=24, q''=18.
    // With r=36+q², r>=36 and the chain rule bounds
    // |(sqrt(r))''''| < 250_000 on [0,1]. Simpson truncation is
    // therefore <=250_000/(180*N^4). Sequential positive summation
    // adds <1e-10 mm rounding error (N*epsilon bound, speed<11).
    let count = 16384;
    let error_bound = 250_000. / (180. * f64::from(count).powi(4)) + 1e-10;
    assert!(error_bound < 1e-9);
    let speed = |t: f64| (36. + (9. - 24. * t + 9. * t * t).powi(2)).sqrt();
    let mut integral = speed(0.) + speed(1.);
    for i in 1..count {
        integral += if i % 2 == 0 { 2. } else { 4. } * speed(f64::from(i) / f64::from(count));
    }
    let length = integral / (3. * f64::from(count));
    close(a.area()?, 3. * length);
    assert_eq!(a.boundary_edge_count()?, 4);
    assert!(a.triangle_count(0.01)? > 0);
    let add = a.combine(&b, NewellBooleanMode::Add)?;
    let remove = a.combine(&b, NewellBooleanMode::Remove)?;
    assert_eq!(add.len(), 1);
    assert_eq!(remove.len(), 1);
    close(add[0].area()?, 4. * length);
    close(remove[0].area()?, length);
    Ok(())
}
#[test]
fn nonconic_tilted_cap_measures_actual_brep_and_is_manifold() -> TestResult {
    let profile =
        newell_kernel::NewellProfile::closed_cubic([[0., 0.], [2., 3.], [4., 2.], [6., 0.]])?;
    let solid = profile.up_to_x_plane(5., 0.1)?;
    close(solid.volume()?, 39.66);
    solid.validate()?;
    assert!(solid.tessellate(0.01)?.indices().len() > 6);
    Ok(())
}

#[test]
fn straight_and_circular_sweeps_supply_real_uncapped_faces() -> TestResult {
    let mut t = Topology::new();
    // The selected edge belongs to this genuine planar support, as production requires.
    let support = polygon(
        &mut t,
        &[[0., 0., 0.], [10., 0., 0.], [10., 1., 0.], [0., 1., 0.]],
    )?;
    let solid = extrude(&mut t, support, Vec3::new(0., 0., 1.), 5.)?;
    let mut found = false;
    for id in brepkit_topology::explorer::solid_faces(&t, solid)? {
        let f = t.face(id)?;
        if f.effective_plane_normal()
            .is_some_and(|n| n.y().abs() > 0.99)
        {
            close(face_area(&t, id, 0.001)?, 50.);
            assert_eq!(t.wire(f.outer_wire())?.edges().len(), 4);
            assert!(
                !brepkit_operations::tessellate::tessellate(&t, id, 0.01)?
                    .indices
                    .is_empty()
            );
            found = true;
        }
    }
    assert!(found);
    let (sheet, area) = cylindrical_sheet(&mut t, 0., 3.)?;
    close(area, 12. * std::f64::consts::PI);
    let f = t.face(sheet)?;
    let edges = t.wire(f.outer_wire())?.edges();
    assert_eq!(
        edges
            .iter()
            .filter(|e| matches!(
                t.edge(e.edge()).map(|e| e.curve()),
                Ok(EdgeCurve::Circle(_))
            ))
            .count(),
        2
    );
    assert!(
        !brepkit_operations::tessellate::tessellate(&t, sheet, 0.01)?
            .indices
            .is_empty()
    );
    Ok(())
}
fn cylindrical_sheet(
    t: &mut Topology,
    start: f64,
    end: f64,
) -> Result<(FaceId, f64), Box<dyn std::error::Error>> {
    let edge = make_circle_edge(
        t,
        Point3::new(0., 0., start),
        Vec3::new(0., 0., 1.),
        2.,
        TOL,
    )?;
    let wire = t.add_wire(Wire::new(vec![OrientedEdge::new(edge, true)], true)?);
    let support = t.add_face(Face::new(
        wire,
        vec![],
        FaceSurface::Plane {
            normal: Vec3::new(0., 0., 1.),
            d: start,
        },
    ));
    let solid = extrude(t, support, Vec3::new(0., 0., 1.), end - start)?;
    let face = brepkit_topology::explorer::solid_faces(t, solid)?
        .into_iter()
        .find(|id| {
            matches!(
                t.face(*id).map(|f| f.surface()),
                Ok(FaceSurface::Cylinder(_))
            )
        })
        .ok_or("missing exact cylindrical face")?;
    Ok((face, face_area(t, face, 0.001)?))
}
#[test]
fn same_cylinder_domain_operations_preserve_circular_rims() -> TestResult {
    let mut t = Topology::new();
    let (_, a) = cylindrical_sheet(&mut t, 0., 3.)?;
    let (_, b) = cylindrical_sheet(&mut t, 1., 4.)?;
    let (union, u) = cylindrical_sheet(&mut t, 0., 4.)?;
    let (difference, d) = cylindrical_sheet(&mut t, 0., 1.)?;
    close(a, 12. * std::f64::consts::PI);
    close(b, a);
    close(u, 16. * std::f64::consts::PI);
    close(d, 4. * std::f64::consts::PI);
    for id in [union, difference] {
        assert!(matches!(t.face(id)?.surface(), FaceSurface::Cylinder(_)));
        for edge in t.wire(t.face(id)?.outer_wire())?.edges() {
            assert!(matches!(
                t.edge(edge.edge())?.curve(),
                EdgeCurve::Circle(_) | EdgeCurve::Line
            ));
        }
    }
    Ok(())
}
#[test]
fn coplanar_polygon_arrangement_retains_holes_and_conserves_area() -> TestResult {
    use brepkit_math::{
        polygon_boolean::{BooleanOp as SheetOp, polygon_boolean},
        vec::Point2,
    };
    let rectangle = |x: f64, y: f64, w: f64, h: f64| {
        vec![
            Point2::new(x, y),
            Point2::new(x + w, y),
            Point2::new(x + w, y + h),
            Point2::new(x, y + h),
        ]
    };
    let a = rectangle(0., 0., 10., 5.);
    let b = rectangle(5., 0., 10., 5.);
    let add = polygon_boolean(&a, &b, SheetOp::Union, TOL);
    let remove = polygon_boolean(&a, &b, SheetOp::Difference, TOL);
    close(add.area(), 75.);
    close(remove.area(), 25.);
    let outer = rectangle(0., 0., 10., 6.);
    let inner = rectangle(2., 2., 2., 2.);
    let hole = polygon_boolean(&outer, &inner, SheetOp::Difference, TOL);
    close(hole.area(), 56.);
    assert_eq!(hole.holes.len(), 1);
    assert_eq!(hole.outer.len(), 1);
    let mut t = Topology::new();
    let wire3 = |points: &[Point2]| {
        points
            .iter()
            .map(|p| Point3::new(p.x(), p.y(), 0.))
            .collect::<Vec<_>>()
    };
    let outer_wire = make_polygon_wire(&mut t, &wire3(&hole.outer[0]), TOL)?;
    let inner_wire = make_polygon_wire(&mut t, &wire3(&hole.holes[0]), TOL)?;
    let face = t.add_face(Face::new(
        outer_wire,
        vec![inner_wire],
        FaceSurface::Plane {
            normal: Vec3::new(0., 0., 1.),
            d: 0.,
        },
    ));
    close(face_area(&t, face, 0.001)?, 56.);
    let adjacent = rectangle(10., 0., 10., 5.);
    let joined = polygon_boolean(&a, &adjacent, SheetOp::Union, TOL);
    close(joined.area(), 100.);
    assert_eq!(joined.outer.len(), 1);
    Ok(())
}
#[test]
fn tilted_polygon_cap_is_an_exact_planar_loft() -> TestResult {
    let mut t = Topology::new();
    let bottom = polygon(
        &mut t,
        &[[0., 0., 0.], [10., 0., 0.], [10., 6., 0.], [0., 6., 0.]],
    )?;
    let top = polygon(
        &mut t,
        &[[0., 0., 5.], [10., 0., 6.], [10., 6., 6.], [0., 6., 5.]],
    )?;
    let n = Vec3::new(-0.1, 0., 1.).normalize()?;
    t.face_mut(top)?.set_surface(FaceSurface::Plane {
        normal: n,
        d: 5. / 1.01_f64.sqrt(),
    });
    let solid = brepkit_operations::loft::loft(&mut t, &[bottom, top])?;
    close(solid_volume(&t, solid, 0.001)?, 330.);
    assert!(validate_solid(&t, solid)?.is_valid());
    Ok(())
}

#[test]
fn tilted_circle_cap_retains_exact_rational_boundary() -> TestResult {
    let solid = newell_kernel::NewellProfile::circular_up_to_x_plane(2., 5., 0.1)?;
    close(solid.volume()?, 20. * std::f64::consts::PI);
    solid.validate()?;
    assert!(!solid.tessellate(0.01)?.indices().is_empty());
    Ok(())
}

#[test]
fn actual_cube_faces_keep_mesh_ordinals_frames_and_historical_geometry() -> TestResult {
    let cube = newell_kernel::NewellSolid::box_solid(1., 1., 1.)?;
    let faces = cube.profile_faces()?;
    assert_eq!(faces.len(), 6);
    for deflection in [0.05, 0.005] {
        let mesh = cube.tessellate(deflection)?;
        assert_eq!(mesh.face_count(), faces.len());
        for face in &faces {
            let ordinal = usize::try_from(face.face)?;
            let start = usize::try_from(mesh.face_offsets()[ordinal])?;
            let end = usize::try_from(mesh.face_offsets()[ordinal + 1])?;
            assert!(end > start);
            for &index in &mesh.indices()[start..end] {
                let point = mesh.positions()[usize::try_from(index)?];
                let signed = (0..3)
                    .map(|i| (f64::from(point[i]) - face.frame.origin[i]) * face.frame.normal[i])
                    .sum::<f64>();
                assert!(signed.abs() < 1e-6);
            }
            let profile = cube.face_profile(face.face)?;
            assert_eq!(profile.frame()?, face.frame);
            close(profile.extrude(face.frame.normal, 3.)?.volume()?, 3.);
            assert!(profile.extrude(face.frame.u, 3.).is_err());
        }
    }
    let mut arena = newell_kernel::NewellGeometry::new();
    let base = arena.box_shape([1.; 3], [0.; 3])?;
    let base_faces = arena.profile_faces(base)?;
    for face in &base_faces {
        assert_eq!(
            face.provenance,
            newell_kernel::NewellFaceProvenance::Base(face.face)
        );
        let shape = arena.extrude_face(base, face.face, None, 2., 7.)?;
        close(arena.volume(shape)?, 5.);
        close(arena.volume(base)?, 1.);
        assert_eq!(arena.profile_faces(base)?, base_faces);
    }
    // The same geometric X edge occurs under two distinct planar supports.
    let mut supports = Vec::new();
    for face in &faces {
        for wire in &face.wires {
            for edge in &wire.edges {
                let same = |a: [f64; 3], b: [f64; 3]| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-7);
                if (same(edge.start, [0., 0., 0.]) && same(edge.end, [1., 0., 0.]))
                    || (same(edge.end, [0., 0., 0.]) && same(edge.start, [1., 0., 0.]))
                {
                    supports.push(face.frame.normal);
                }
            }
        }
    }
    assert_eq!(supports.len(), 2);
    assert!(supports.contains(&[0., 0., -1.]));
    assert!(supports.contains(&[0., -1., 0.]));
    Ok(())
}

#[test]
fn finite_up_to_preserves_profile_and_retracts_clearance() -> TestResult {
    let mut geometry = newell_kernel::NewellGeometry::new();
    let source = geometry.box_shape([2., 3., 1.], [0.; 3])?;
    let target = geometry.box_shape([4., 5., 1.], [-1., -1., 5.])?;
    let profile = geometry
        .profile_faces(source)?
        .into_iter()
        .find(|face| face.frame.normal == [0., 0., 1.])
        .ok_or("top face")?
        .face;
    let target_face = geometry
        .profile_faces(target)?
        .into_iter()
        .find(|face| face.frame.normal == [0., 0., -1.])
        .ok_or("bottom face")?
        .face;
    let result = geometry.extrude_to_face(
        (source, profile),
        [0., 0., 1.],
        0.,
        (target, target_face),
        0.,
    )?;
    close(geometry.volume(result)?, 24.);
    let result = geometry.extrude_to_face(
        (source, profile),
        [0., 0., 1.],
        0.,
        (target, target_face),
        0.5,
    )?;
    close(geometry.volume(result)?, 21.);
    close(geometry.volume(source)?, 6.);
    let small = geometry.box_shape([1., 5., 1.], [-0.5, -1., 5.])?;
    let small_face = geometry
        .profile_faces(small)?
        .into_iter()
        .find(|face| face.frame.normal == [0., 0., -1.])
        .ok_or("small face")?
        .face;
    assert!(
        geometry
            .extrude_to_face((source, profile), [0., 0., 1.], 0., (small, small_face), 0.)
            .is_err()
    );
    assert!(
        geometry
            .extrude_to_face(
                (source, profile),
                [0., 0., 1.],
                0.,
                (target, target_face),
                4.
            )
            .is_err()
    );
    Ok(())
}

#[test]
fn through_all_uses_actual_multiple_hits_and_ignores_a_distant_miss() -> TestResult {
    let mut geometry = newell_kernel::NewellGeometry::new();
    let source = geometry.box_shape([2., 3., 1.], [0.; 3])?;
    let profile = geometry
        .profile_faces(source)?
        .into_iter()
        .find(|face| face.frame.normal == [0., 0., 1.])
        .ok_or("top face")?
        .face;
    let a = geometry.box_shape([2., 3., 1.], [0., 0., 5.])?;
    let b = geometry.box_shape([2., 3., 1.], [0., 0., 9.])?;
    let miss = geometry.box_shape([2., 3., 1.], [100., 100., 100.])?;
    let end =
        geometry.through_all_station((source, profile), [0., 0., 1.], 0., &[source, a, b, miss])?;
    assert!((end - 9.000001).abs() < 1e-9, "{end}");
    assert!(
        geometry
            .through_all_station((source, profile), [0., 0., 1.], 0., &[source, miss])
            .is_err()
    );
    let result = geometry.extrude_face(source, profile, Some([0., 0., 1.]), 0., end)?;
    close(geometry.volume(result)?, 6. * 9.000001);
    Ok(())
}

#[test]
fn symmetric_oblique_sweep_reflects_stations_about_one_displaced_section() -> TestResult {
    let mut geometry = newell_kernel::NewellGeometry::new();
    let source = geometry.box_shape([2., 3., 1.], [0.; 3])?;
    let profile = geometry
        .profile_faces(source)?
        .into_iter()
        .find(|face| face.frame.normal == [0., 0., 1.])
        .ok_or("top face")?
        .face;
    let direction = [0.6, 0., 0.8];
    let main = geometry.extrude_face(source, profile, Some(direction), 2., 7.)?;
    let second = geometry.mirror_sweep(main, (source, profile), direction, 2.)?;
    close(geometry.volume(main)?, 24.);
    close(geometry.volume(second)?, 24.);
    let result = geometry
        .boolean(main, second, newell_kernel::NewellBooleanMode::Add)?
        .ok_or("empty union")?;
    close(geometry.volume(result)?, 48.);
    let target = geometry.box_shape([20., 20., 1.], [-10., -10., 3.])?;
    let target_face = geometry
        .profile_faces(target)?
        .into_iter()
        .find(|face| face.frame.normal == [0., 0., -1.])
        .ok_or("starting target")?
        .face;
    let station = geometry.starting_station((source, profile), direction, (target, target_face))?;
    assert!((station - 2.5).abs() < 1e-10);
    Ok(())
}

#[test]
fn real_face_wire_and_edge_surface_sweeps_are_uncapped_exact_sheets() -> TestResult {
    use newell_kernel::NewellBoundary;
    let mut geometry = newell_kernel::NewellGeometry::new();
    let source = geometry.box_shape([2., 3., 1.], [0.; 3])?;
    let profile = geometry
        .profile_faces(source)?
        .into_iter()
        .find(|face| face.frame.normal == [0., 0., 1.])
        .ok_or("top face")?
        .face;
    for boundary in [NewellBoundary::Face, NewellBoundary::Wire(0)] {
        let shapes = geometry.sweep_surface((source, profile), boundary, [0., 0., 1.], 0., 4.)?;
        assert_eq!(shapes.len(), 1);
        let shape = shapes[0];
        assert!(geometry.is_surface(shape));
        assert_eq!(geometry.face_count(shape)?, 4);
        close(geometry.surface_area(shape)?, 40.);
        let mesh = geometry.tessellate(shape, 0.05)?;
        assert_eq!(mesh.face_count(), 4);
        assert!(
            mesh.normals().iter().all(|normal| normal[2].abs() < 1e-6),
            "no cap normal"
        );
        assert!(!mesh.feature_edges().positions().is_empty());
    }
    let shape = geometry.sweep_surface(
        (source, profile),
        NewellBoundary::Edge { wire: 0, edge: 0 },
        [0., 0., 1.],
        2.,
        6.,
    )?[0];
    assert_eq!(geometry.face_count(shape)?, 1);
    let face = geometry
        .profile_faces(source)?
        .into_iter()
        .find(|face| face.face == profile)
        .ok_or("profile")?;
    let edge = &face.wires[0].edges[0];
    let length = (0..3)
        .map(|axis| (edge.end[axis] - edge.start[axis]).powi(2))
        .sum::<f64>()
        .sqrt();
    close(geometry.surface_area(shape)?, length * 4.);
    Ok(())
}

#[test]
fn body_results_distinguish_split_full_removal_and_unchanged_targets() -> TestResult {
    use newell_kernel::NewellBooleanMode;
    let mut geometry = newell_kernel::NewellGeometry::new();
    let body = geometry.box_shape([10.; 3], [0.; 3])?;
    let slab = geometry.box_shape([2., 10., 10.], [4., 0., 0.])?;
    let split = geometry.combine_bodies(slab, &[body], NewellBooleanMode::Remove)?;
    assert_eq!(split.changed_targets, [0]);
    assert_eq!(split.outputs.len(), 2);
    for key in split.outputs {
        close(geometry.volume(key)?, 400.);
    }
    let removed = geometry.combine_bodies(body, &[body], NewellBooleanMode::Remove)?;
    assert_eq!(removed.changed_targets, [0]);
    assert!(removed.outputs.is_empty());
    let miss = geometry.box_shape([10.; 3], [100.; 3])?;
    assert!(
        geometry
            .combine_bodies(miss, &[body], NewellBooleanMode::Remove)
            .is_err()
    );
    let tool = geometry.box_shape([10.; 3], [5., 0., 0.])?;
    let union = geometry.combine_bodies(tool, &[miss, body], NewellBooleanMode::Add)?;
    assert_eq!(union.changed_targets, [1]);
    assert_eq!(union.outputs.len(), 1);
    close(geometry.volume(union.outputs[0])?, 1500.);
    Ok(())
}

#[test]
fn planar_sheet_union_difference_weld_and_split_real_uncapped_faces() -> TestResult {
    use newell_kernel::{NewellBooleanMode, NewellBoundary};
    let mut geometry = newell_kernel::NewellGeometry::new();
    let source = geometry.box_shape([2., 3., 1.], [0.; 3])?;
    let face = geometry
        .profile_faces(source)?
        .into_iter()
        .find(|face| face.frame.normal == [0., 0., 1.])
        .ok_or("support")?
        .face;
    let target =
        geometry.sweep_surface((source, face), NewellBoundary::Face, [0., 0., 1.], 0., 3.)?[0];
    let tool =
        geometry.sweep_surface((source, face), NewellBoundary::Face, [0., 0., 1.], 1., 4.)?[0];
    let union = geometry.combine_surfaces(&[tool], &[target], NewellBooleanMode::Add)?;
    assert_eq!(union.changed_targets, [0]);
    assert_eq!(union.outputs.len(), 1);
    close(geometry.surface_area(union.outputs[0])?, 40.);
    assert_eq!(geometry.face_count(union.outputs[0])?, 4);
    let cut = geometry.combine_surfaces(&[tool], &[target], NewellBooleanMode::Remove)?;
    assert_eq!(cut.outputs.len(), 1);
    close(geometry.surface_area(cut.outputs[0])?, 10.);
    let target =
        geometry.sweep_surface((source, face), NewellBoundary::Face, [0., 0., 1.], 0., 4.)?[0];
    let tool =
        geometry.sweep_surface((source, face), NewellBoundary::Face, [0., 0., 1.], 1., 3.)?[0];
    let split = geometry.combine_surfaces(&[tool], &[target], NewellBooleanMode::Remove)?;
    assert_eq!(split.outputs.len(), 2);
    for sheet in split.outputs {
        close(geometry.surface_area(sheet)?, 10.);
        assert!(!geometry.tessellate(sheet, 0.05)?.indices().is_empty());
    }
    let empty = geometry.combine_surfaces(&[target], &[target], NewellBooleanMode::Remove)?;
    assert!(empty.outputs.is_empty());
    Ok(())
}

fn planar_sheet(
    geometry: &mut newell_kernel::NewellGeometry,
    x: f64,
    z: f64,
    width: f64,
    height: f64,
) -> Result<newell_kernel::NewellShapeKey, Box<dyn std::error::Error>> {
    let source = geometry.box_shape([width, 1., 1.], [x, 0., z - 1.])?;
    let face = geometry
        .profile_faces(source)?
        .into_iter()
        .find(|face| face.frame.normal == [0., 0., 1.])
        .ok_or("support")?;
    let edge = face.wires[0]
        .edges
        .iter()
        .find(|edge| edge.start[1].abs() < 1e-7 && edge.end[1].abs() < 1e-7)
        .ok_or("edge")?
        .edge;
    Ok(geometry.sweep_surface(
        (source, face.face),
        newell_kernel::NewellBoundary::Edge { wire: 0, edge },
        [0., 0., 1.],
        0.,
        height,
    )?[0])
}
#[test]
fn sheet_booleans_preserve_existing_holes_and_reject_transverse_area_removal() -> TestResult {
    use newell_kernel::{NewellBooleanMode, NewellBoundary};
    let mut geometry = newell_kernel::NewellGeometry::new();
    let target = planar_sheet(&mut geometry, 0., 0., 10., 10.)?;
    let cutter = planar_sheet(&mut geometry, 4., 4., 2., 2.)?;
    let holed = geometry
        .combine_surfaces(&[cutter], &[target], NewellBooleanMode::Remove)?
        .outputs[0];
    close(geometry.surface_area(holed)?, 96.);
    assert_eq!(geometry.profile_faces(holed)?[0].wires.len(), 2);
    let fill = planar_sheet(&mut geometry, 5., 4., 2., 2.)?;
    let partial = geometry
        .combine_surfaces(&[fill], &[holed], NewellBooleanMode::Add)?
        .outputs[0];
    close(geometry.surface_area(partial)?, 98.);
    assert_eq!(geometry.profile_faces(partial)?[0].wires.len(), 2);
    assert!(!geometry.tessellate(partial, 0.05)?.indices().is_empty());
    let target = planar_sheet(&mut geometry, 0., 0., 1., 1.)?;
    let source = geometry.box_shape([1.; 3], [-0.5, -0.5, -1.])?;
    let face = geometry
        .profile_faces(source)?
        .into_iter()
        .find(|face| face.frame.normal == [0., 0., 1.])
        .ok_or("transverse support")?;
    let edge = face.wires[0]
        .edges
        .iter()
        .find(|edge| (edge.start[0] - 0.5).abs() < 1e-7 && (edge.end[0] - 0.5).abs() < 1e-7)
        .ok_or("transverse edge")?
        .edge;
    let tool = geometry.sweep_surface(
        (source, face.face),
        NewellBoundary::Edge { wire: 0, edge },
        [0., 0., 1.],
        0.,
        1.,
    )?[0];
    assert!(
        geometry
            .combine_surfaces(&[tool], &[target], NewellBooleanMode::Remove)
            .is_err()
    );
    assert!(
        geometry
            .combine_surfaces(&[tool], &[target], NewellBooleanMode::Add)
            .is_err()
    );
    close(geometry.surface_area(target)?, 1.);
    Ok(())
}
