// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Private migration meshing bridge. Inputs always include the actual trimmed boundary.
use crate::{NewellEdgePolylines, NewellKernelError, NewellMesh, operation_error};
use nurbs_grid_mesh::{LoopPoint, MeshOptions, NurbsSurface};

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
fn unit(n: [f64; 3]) -> Result<[f64; 3], NewellKernelError> {
    let length = dot(n, n).sqrt();
    if !length.is_finite() || length <= 0. {
        return Err(operation_error("degenerate mesh normal"));
    }
    Ok(n.map(|x| x / length))
}
#[cfg(test)]
fn checked_mesh(
    points: Vec<[f64; 3]>,
    normals: Vec<[f64; 3]>,
    triangles: Vec<[u32; 3]>,
    feature_edges: NewellEdgePolylines,
) -> Result<NewellMesh, NewellKernelError> {
    checked_mesh_cancellable(points, normals, triangles, feature_edges, &|| Ok(()))
}

fn checked_mesh_cancellable(
    points: Vec<[f64; 3]>,
    normals: Vec<[f64; 3]>,
    triangles: Vec<[u32; 3]>,
    feature_edges: NewellEdgePolylines,
    check_cancelled: &dyn Fn() -> Result<(), NewellKernelError>,
) -> Result<NewellMesh, NewellKernelError> {
    check_cancelled()?;
    if triangles.is_empty() {
        return Err(operation_error("empty face tessellation"));
    }
    if points.len() != normals.len() {
        return Err(operation_error("mesh normal count"));
    }
    let positions: Vec<_> = points
        .iter()
        .map(|p| {
            check_cancelled()?;
            Ok([
                crate::finite_f32(p[0])?,
                crate::finite_f32(p[1])?,
                crate::finite_f32(p[2])?,
            ])
        })
        .collect::<Result<_, NewellKernelError>>()?;
    let normals = normals
        .into_iter()
        .map(|n| {
            check_cancelled()?;
            let n = unit(n)?;
            Ok([
                crate::finite_f32(n[0])?,
                crate::finite_f32(n[1])?,
                crate::finite_f32(n[2])?,
            ])
        })
        .collect::<Result<Vec<_>, NewellKernelError>>()?;
    let mut indices = Vec::new();
    for t in triangles {
        check_cancelled()?;
        let exact = t.map(|i| points.get(i as usize));
        let rounded = t.map(|i| positions.get(i as usize));
        let [Some(a), Some(b), Some(c)] = exact else {
            return Err(operation_error("mesh triangle index"));
        };
        let [Some(ar), Some(br), Some(cr)] = rounded else {
            return Err(operation_error("mesh triangle index"));
        };
        let before = cross(sub(*b, *a), sub(*c, *a));
        let after = cross(
            sub(br.map(f64::from), ar.map(f64::from)),
            sub(cr.map(f64::from), ar.map(f64::from)),
        );
        if dot(before, before) <= 0. {
            return Err(operation_error("degenerate mesh triangle"));
        }
        if dot(before, after) <= 0. {
            println!(
                "MESH_PRECISION_LOSS,triangle={t:?},exact={exact:?},rounded={rounded:?},before={before:?},after={after:?}"
            );
            return Err(NewellKernelError::MeshPrecisionLoss);
        }
        indices.extend(t);
    }
    let end = u32::try_from(indices.len()).map_err(operation_error)?;
    Ok(NewellMesh {
        positions,
        normals,
        indices,
        face_offsets: vec![0, end],
        feature_edges,
    })
}

/// Curved trimmed face; exact UV/3D boundary samples come from its native occurrences.
#[cfg(test)]
pub(super) fn trimmed_face(
    surface: &NurbsSurface,
    loops: &[Vec<LoopPoint>],
    reversed: bool,
    deflection: f64,
    feature_edges: NewellEdgePolylines,
) -> Result<NewellMesh, NewellKernelError> {
    trimmed_face_cancellable(surface, loops, reversed, deflection, feature_edges, &|| {
        Ok(())
    })
}

#[cfg(test)]
pub(super) fn trimmed_face_cancellable(
    surface: &NurbsSurface,
    loops: &[Vec<LoopPoint>],
    reversed: bool,
    deflection: f64,
    feature_edges: NewellEdgePolylines,
    check_cancelled: &dyn Fn() -> Result<(), NewellKernelError>,
) -> Result<NewellMesh, NewellKernelError> {
    trimmed_face_with_normals(
        surface,
        loops,
        reversed,
        deflection,
        feature_edges,
        &FaceMeshChecks {
            check_cancelled,
            normal_at: &|uv| {
                let eval = surface.evaluate(uv[0], uv[1]).map_err(operation_error)?;
                Ok(cross(eval.du, eval.dv))
            },
        },
    )
}

struct FaceMeshChecks<'a> {
    check_cancelled: &'a dyn Fn() -> Result<(), NewellKernelError>,
    normal_at: &'a dyn Fn([f64; 2]) -> Result<[f64; 3], NewellKernelError>,
}

fn trimmed_face_with_normals(
    surface: &NurbsSurface,
    loops: &[Vec<LoopPoint>],
    reversed: bool,
    deflection: f64,
    feature_edges: NewellEdgePolylines,
    checks: &FaceMeshChecks<'_>,
) -> Result<NewellMesh, NewellKernelError> {
    let check_cancelled = checks.check_cancelled;
    check_cancelled()?;
    validate_deflection(deflection)?;
    let mut mesh =
        nurbs_grid_mesh::mesh_trimmed(surface, loops, reversed, &MeshOptions::new(deflection))
            .map_err(operation_error)?;
    check_cancelled()?;
    if mesh.capped_spans != 0 {
        return Err(operation_error("mesh refinement limit"));
    }
    canonicalize_inserted_boundary_points(
        &mut mesh,
        loops,
        surface.domain(),
        deflection,
        check_cancelled,
    )?;

    let normals = mesh
        .uvs
        .iter()
        .map(|uv| {
            check_cancelled()?;
            let n = (checks.normal_at)(*uv)?;
            Ok(if reversed { n.map(|x| -x) } else { n })
        })
        .collect::<Result<Vec<_>, NewellKernelError>>()?;
    checked_mesh_cancellable(
        mesh.positions,
        normals,
        mesh.triangles,
        feature_edges,
        check_cancelled,
    )
}

/// Planar native faces in Newell slot order; native IDs never become persistent identity.
#[cfg(test)]
pub(super) fn planar_native(
    kernel: &solidcore::Kernel,
    body: &solidcore::Brep,
    slots: &[solidcore::FaceId],
    deflection: f64,
) -> Result<NewellMesh, NewellKernelError> {
    planar_native_cancellable(kernel, body, slots, deflection, &|| Ok(()))
}

pub(super) fn planar_native_cancellable(
    kernel: &solidcore::Kernel,
    body: &solidcore::Brep,
    slots: &[solidcore::FaceId],
    deflection: f64,
    check_cancelled: &dyn Fn() -> Result<(), NewellKernelError>,
) -> Result<NewellMesh, NewellKernelError> {
    check_cancelled()?;
    validate_deflection(deflection)?;
    let mut meshes = Vec::new();
    let mut feature_edges_seen = std::collections::BTreeSet::new();
    for &face in slots {
        check_cancelled()?;
        let surface = solidcore::topology::face::p2::surface(body, face)
            .map_err(operation_error)?
            .ok_or_else(|| operation_error("missing face surface"))?;
        if !matches!(
            surface.kind(),
            solidcore::surface::surface::SurfaceKind::Plane(_)
        ) {
            return Err(operation_error(
                "curved face requires exact NURBS and UV trim conversion",
            ));
        }
        let boundary = solidcore::query::api_queries::face_get_boundary_loops(body, face)
            .map_err(operation_error)?;
        let mut points = Vec::new();
        let mut loops = Vec::new();
        let mut edges = NewellEdgePolylines::default();
        edges.offsets.push(0);
        let mut edge_index = 0;
        for count in boundary.edge_counts {
            let mut ring = Vec::new();
            for _ in 0..count {
                check_cancelled()?;
                let id = *boundary
                    .edges
                    .get(edge_index)
                    .ok_or_else(|| operation_error("boundary edge"))?;
                let orientation = *boundary
                    .orientations
                    .get(edge_index)
                    .ok_or_else(|| operation_error("boundary orientation"))?;
                let edge = body.edge(id).map_err(operation_error)?;
                let curve = edge
                    .curve
                    .as_ref()
                    .ok_or_else(|| operation_error("missing boundary curve"))?;
                let mut sample = curve
                    .tessellate(kernel, &edge.interval, deflection * 0.5, 5., 1)
                    .map_err(operation_error)?
                    .points;
                anchor_native_curve_endpoints(body, edge, &mut sample)?;
                if orientation == solidcore::nurbs::b_spline_surface_n_lib::p2::OrientType::Opposite
                {
                    sample.reverse();
                }
                if sample.len() < 2 {
                    return Err(operation_error("incomplete edge samples"));
                }
                if feature_edges_seen.insert(id) {
                    for p in &sample {
                        edges.positions.push([
                            crate::finite_f32(p.x)?,
                            crate::finite_f32(p.y)?,
                            crate::finite_f32(p.z)?,
                        ]);
                    }
                    edges
                        .offsets
                        .push(u32::try_from(edges.positions.len()).map_err(operation_error)?);
                }
                sample.pop(); // next oriented edge owns this common endpoint
                for p in sample {
                    ring.push(points.len());
                    points.push([p.x, p.y, p.z]);
                }
                edge_index += 1;
            }
            loops.push(ring);
        }
        let outer = loops
            .first()
            .ok_or_else(|| operation_error("missing face loops"))?;
        let origin = *outer
            .first()
            .and_then(|i| points.get(*i))
            .ok_or_else(|| operation_error("empty outer loop"))?;
        // The pinned triangulator projects absolute points before its shoelace
        // area check. Rebase only its display-space input to avoid cancellation
        // for small faces far from world origin. Returned indices still address
        // the untouched native world-space samples; no model/tolerance changes.
        let local_points = points
            .iter()
            .map(|p| {
                check_cancelled()?;
                Ok(sub(*p, origin))
            })
            .collect::<Result<Vec<_>, NewellKernelError>>()?;
        let triangles = nurbs_grid_mesh::triangulate_planar(&local_points, outer, &loops[1..])
            .map_err(operation_error)?;
        let mut normal = [0.; 3];
        for pair in outer.windows(2) {
            let a = points[pair[0]];
            let b = points[pair[1]];
            let c = cross(sub(a, origin), sub(b, origin));
            for i in 0..3 {
                normal[i] += c[i];
            }
        }
        if let (Some(&first), Some(&last)) = (outer.first(), outer.last()) {
            let c = cross(sub(points[last], origin), sub(points[first], origin));
            for i in 0..3 {
                normal[i] += c[i];
            }
        }
        let normal = unit(normal)?;
        let primary =
            solidcore::topology::face::p2::upward_faceuse(body, face).map_err(operation_error)?;
        let display = crate::solidcore::native_display_faceuse(body, face)?;
        let normal = if primary == Some(display) {
            normal
        } else {
            normal.map(|n| -n)
        };
        let triangles = triangles
            .into_iter()
            .map(|t| {
                let mut t = t.map(|i| u32::try_from(i).map_err(operation_error));
                let mut ids = [0; 3];
                for (i, id) in t.iter_mut().enumerate() {
                    ids[i] = id
                        .as_ref()
                        .copied()
                        .map_err(|e| operation_error(e.to_string()))?;
                }
                let n = cross(
                    sub(points[ids[1] as usize], points[ids[0] as usize]),
                    sub(points[ids[2] as usize], points[ids[0] as usize]),
                );
                if dot(n, normal) < 0. {
                    ids.swap(1, 2);
                }
                Ok(ids)
            })
            .collect::<Result<Vec<_>, NewellKernelError>>()?;
        let normals = vec![normal; points.len()];
        meshes.push(checked_mesh_cancellable(
            points,
            normals,
            triangles,
            edges,
            check_cancelled,
        )?);
    }
    let refs: Vec<_> = meshes.iter().collect();
    check_cancelled()?;
    let mesh = NewellMesh::compose(&refs)?.0;
    check_cancelled()?;
    Ok(mesh)
}

pub(super) fn native_nurbs(
    source: &solidcore::nurbs::srf_basic::Surface,
) -> Result<NurbsSurface, NewellKernelError> {
    source.is_valid().map_err(operation_error)?;
    let mut poles = Vec::new();
    let mut weights = Vec::new();
    let rational = source.is_rational();
    for i in 0..=source.n() {
        for j in 0..=source.m() {
            let p = source.cpt(i, j).map_err(operation_error)?;
            let w = if rational { p.w } else { 1. };
            if !w.is_finite() || w <= 0. {
                return Err(operation_error("unsupported NURBS weight"));
            }
            poles.push([p.x / w, p.y / w, p.z / w]);
            weights.push(w);
        }
    }
    NurbsSurface::new(
        source.p(),
        source.q(),
        source.u_knots().to_vec(),
        source.v_knots().to_vec(),
        source.n() + 1,
        source.m() + 1,
        poles,
        if rational { Some(&weights) } else { None },
    )
    .map_err(operation_error)
}

#[test]
fn solidcore_migration_curved_trim_hole_mesh() -> Result<(), Box<dyn std::error::Error>> {
    use solidcore::nurbs::{math::CPoint, srf_basic::Surface};
    let net = (0..3)
        .map(|i| {
            (0..2)
                .map(|j| {
                    CPoint::from_point(solidcore::math::Vec3::new(
                        i as f64 * 0.5,
                        j as f64,
                        if i == 2 { 1. } else { 0. },
                    ))
                })
                .collect()
        })
        .collect();
    let source = Surface::new(
        net,
        2,
        1,
        vec![0., 0., 0., 1., 1., 1.],
        vec![0., 0., 1., 1.],
    )?;
    let surface = native_nurbs(&source)?;
    for _ in 0..10 {
        for deflection in [f64::INFINITY, 0., f64::NAN, -1.] {
            assert!(matches!(
                trimmed_face(
                    &surface,
                    &[],
                    false,
                    deflection,
                    NewellEdgePolylines::default()
                ),
                Err(NewellKernelError::InvalidParameter {
                    parameter: "deflection"
                })
            ));
        }
    }
    let mut loops = Vec::new();
    for corners in [
        [[0.1, 0.1], [0.9, 0.1], [0.9, 0.9], [0.1, 0.9]],
        [[0.35, 0.35], [0.35, 0.55], [0.55, 0.55], [0.55, 0.35]],
    ] {
        let mut ring = Vec::new();
        for edge in 0..4 {
            for step in 0..32 {
                let t = step as f64 / 32.;
                let a = corners[edge];
                let b = corners[(edge + 1) % 4];
                let u = a[0] + t * (b[0] - a[0]);
                let v = a[1] + t * (b[1] - a[1]);
                let p = source.eval(
                    u,
                    v,
                    solidcore::nurbs::knots::Side::Right,
                    solidcore::nurbs::knots::Side::Right,
                )?;
                ring.push(LoopPoint {
                    u,
                    v,
                    p: [p.x, p.y, p.z],
                });
            }
        }
        loops.push(ring);
    }
    for deflection in [0.05, 0.01] {
        for reversed in [false, true] {
            let mesh = trimmed_face(
                &surface,
                &loops,
                reversed,
                deflection,
                NewellEdgePolylines::default(),
            )?;
            assert_eq!(mesh.face_count(), 1);
            assert_eq!(
                mesh.face_offsets(),
                &[0, u32::try_from(mesh.indices().len())?]
            );
            let mut area = 0.;
            for t in mesh.indices().as_chunks::<3>().0 {
                let p = t
                    .iter()
                    .map(|&i| mesh.positions()[i as usize].map(f64::from))
                    .collect::<Vec<_>>();
                let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
                area += n[2].abs() * 0.5;
                assert!(
                    if reversed { n[2] < 0. } else { n[2] > 0. },
                    "winding reversed={reversed} n={n:?} p={p:?}"
                );
                let center = [
                    (p[0][0] + p[1][0] + p[2][0]) / 3.,
                    (p[0][1] + p[1][1] + p[2][1]) / 3.,
                ];
                assert!(
                    !(center[0] > 0.35 && center[0] < 0.55 && center[1] > 0.35 && center[1] < 0.55)
                );
                for &i in t {
                    let p = mesh.positions()[i as usize].map(f64::from);
                    assert!(
                        (0.1 - 1e-7..=0.9 + 1e-7).contains(&p[0])
                            && (0.1 - 1e-7..=0.9 + 1e-7).contains(&p[1])
                    );
                    assert!((p[2] - p[0] * p[0]).abs() < deflection);
                    let normal = mesh.normals()[i as usize].map(f64::from);
                    assert!(dot(n, normal) > 0.);
                }
            }
            assert!((area - 0.60).abs() < 1e-6, "trimmed UV area {area}");
        }
    }
    Ok(())
}

/// Remove only redundant inserted points on an original straight trim-polyline
/// segment. Every caller-supplied boundary sample remains an exact retained anchor.
/// No interior point or two distinct source samples may be welded by this route.
type BoundaryAnchor = (LoopPoint, [f64; 3], [f64; 3]);

fn canonicalize_inserted_boundary_points(
    mesh: &mut nurbs_grid_mesh::TrimmedMesh,
    loops: &[Vec<LoopPoint>],
    surface_domain: [[f64; 2]; 2],
    deflection: f64,
    check_cancelled: &dyn Fn() -> Result<(), NewellKernelError>,
) -> Result<(), NewellKernelError> {
    use std::collections::BTreeMap;
    let key = |p: [f64; 3]| -> Result<[u32; 3], NewellKernelError> {
        Ok([
            crate::finite_f32(p[0])?.to_bits(),
            crate::finite_f32(p[1])?.to_bits(),
            crate::finite_f32(p[2])?.to_bits(),
        ])
    };
    let mut anchors: BTreeMap<[u32; 3], Vec<BoundaryAnchor>> = BTreeMap::new();
    for ring in loops {
        for (i, p) in ring.iter().enumerate() {
            check_cancelled()?;
            let prev = ring[(i + ring.len() - 1) % ring.len()].p;
            let next = ring[(i + 1) % ring.len()].p;
            anchors.entry(key(p.p)?).or_default().push((*p, prev, next));
        }
    }
    let exact: BTreeMap<_, _> = mesh
        .positions
        .iter()
        .enumerate()
        .map(|(i, p)| (p.map(f64::to_bits), i))
        .collect();
    let samples: Vec<_> = loops.iter().flatten().copied().collect();
    // Pinned trim.rs::nudge moves UV by 2e-9 of the clipped *grid* domain per
    // settling pass. Only generated samples inside this known perturbation
    // envelope may be restored to an exact, unique caller-owned trim sample.
    // Grid clipping keeps bracketing lines outside the trim-loop bounds, so the
    // loop span is not a valid bound. Every grid line remains inside the full
    // supplied NURBS domain; its span gives the truthful conservative envelope.
    let uv_shift = [0, 1]
        .map(|i| 2e-9 * (surface_domain[i][1] - surface_domain[i][0]) * (loops.len() + 1) as f64);
    let originals: std::collections::BTreeSet<_> =
        samples.iter().map(|p| p.p.map(f64::to_bits)).collect();
    let mut remap: Vec<_> = (0..mesh.positions.len()).collect();
    for (i, &point) in mesh.positions.iter().enumerate() {
        check_cancelled()?;
        if !originals.contains(&point.map(f64::to_bits)) {
            let uv = *mesh
                .uvs
                .get(i)
                .ok_or_else(|| operation_error("mesh UV count"))?;
            let candidates: Vec<_> = samples
                .iter()
                .filter(|p| {
                    (p.u - uv[0]).abs() <= uv_shift[0] && (p.v - uv[1]).abs() <= uv_shift[1]
                })
                .collect();
            if let [anchor] = candidates.as_slice()
                && let Some(&target) = exact.get(&anchor.p.map(f64::to_bits))
            {
                let displacement = sub(point, anchor.p);
                if dot(displacement, displacement).sqrt() <= deflection * 1e-3 {
                    remap[i] = target;
                    mesh.uvs[target] = [anchor.u, anchor.v];
                    continue;
                }
            }
        }
        let Some(candidates) = anchors.get(&key(point)?) else {
            continue;
        };
        let [(anchor, prev, next)] = candidates.as_slice() else {
            continue;
        };
        let Some(&target) = exact.get(&anchor.p.map(f64::to_bits)) else {
            continue;
        };
        if point == anchor.p {
            mesh.uvs[target] = [anchor.u, anchor.v];
            continue;
        }
        let on_segment = |a: [f64; 3], b: [f64; 3]| {
            let ab = sub(b, a);
            let ap = sub(point, a);
            let d = dot(ab, ab);
            if d == 0. {
                return false;
            }
            let t = dot(ap, ab) / d;
            if !(0.0..=1.0).contains(&t) {
                return false;
            }
            let residual = sub(ap, ab.map(|x| x * t));
            // Arithmetic roundoff, not a model/display tolerance. This only
            // verifies membership on the unchanged input polyline segment.
            let scale = a
                .into_iter()
                .chain(b)
                .chain(point)
                .fold(1_f64, |m, x| m.max(x.abs()));
            dot(residual, residual).sqrt() <= 64. * f64::EPSILON * scale
        };
        if on_segment(*prev, anchor.p) || on_segment(anchor.p, *next) {
            remap[i] = target;
            mesh.uvs[target] = [anchor.u, anchor.v];
        }
    }
    let mut triangles = Vec::with_capacity(mesh.triangles.len());
    for original in &mesh.triangles {
        check_cancelled()?;
        let mut mapped = [0; 3];
        for i in 0..3 {
            mapped[i] = u32::try_from(
                *remap
                    .get(original[i] as usize)
                    .ok_or_else(|| operation_error("mesh index"))?,
            )
            .map_err(operation_error)?;
        }
        if mapped[0] == mapped[1] || mapped[1] == mapped[2] || mapped[0] == mapped[2] {
            if mapped == *original {
                return Err(operation_error("degenerate source mesh triangle"));
            }
            continue;
        }
        triangles.push(mapped);
    }
    mesh.triangles = triangles;
    Ok(())
}

/// Full native body bridge for the currently supported analytic/NURBS surface families.
/// slots is a generation-local Newell-to-native occurrence map, supplied by the owner.
#[cfg(test)]
pub(super) fn native_body(
    kernel: &solidcore::Kernel,
    body: &solidcore::Brep,
    slots: &[solidcore::FaceId],
    deflection: f64,
) -> Result<NewellMesh, NewellKernelError> {
    native_body_cancellable(kernel, body, slots, deflection, &|| Ok(()))
}

pub(super) fn native_body_cancellable(
    kernel: &solidcore::Kernel,
    body: &solidcore::Brep,
    slots: &[solidcore::FaceId],
    deflection: f64,
    check_cancelled: &dyn Fn() -> Result<(), NewellKernelError>,
) -> Result<NewellMesh, NewellKernelError> {
    use solidcore::nurbs::b_spline_surface_n_lib::p2::OrientType;
    use solidcore::surface::surface::SurfaceKind;
    use solidcore::topology::edgeuse::p1 as eu;
    check_cancelled()?;
    validate_deflection(deflection)?;
    let mut meshes = Vec::new();
    let mut all_edges = std::collections::BTreeSet::new();
    for &face in slots {
        check_cancelled()?;
        let surface = solidcore::topology::face::p2::surface(body, face)
            .map_err(operation_error)?
            .ok_or_else(|| operation_error("missing surface"))?;
        let boundary = solidcore::query::api_queries::face_get_boundary_loops(body, face)
            .map_err(operation_error)?;
        all_edges.extend(boundary.edges);
        let mut mesh = if matches!(surface.kind(), SurfaceKind::Plane(_)) {
            planar_native_cancellable(kernel, body, &[face], deflection, check_cancelled)?
        } else {
            let kind = surface.kind_with_nurb(kernel).map_err(operation_error)?;
            let base = match kind {
                SurfaceKind::BSpline(s) => s,
                SurfaceKind::SurfOfExtrusion(s) => s.base(),
                SurfaceKind::SurfOfRevolution(s) => s.base(),
                SurfaceKind::Cone(s) => s.rev().base(),
                SurfaceKind::Sphere(s) => s.base().base(),
                SurfaceKind::Torus(s) => s.base().base(),
                _ => return Err(operation_error("unsupported native surface conversion")),
            };
            let nurb = base
                .gw_nurb()
                .ok_or_else(|| operation_error("native NURBS conversion missing"))?;
            let neutral = native_nurbs(nurb)?;
            let faceuse = crate::solidcore::native_display_faceuse(body, face)?;
            let mut loops = Vec::new();
            for lu in eu::faceuse_loopuses(body, faceuse, false, None).map_err(operation_error)? {
                let mut ring = Vec::new();
                let mut first_vertex = None;
                let mut last_vertex = None;
                for edgeuse in eu::loopuse_edgeuses(body, lu).map_err(operation_error)? {
                    check_cancelled()?;
                    let occurrence = body.edgeuse(edgeuse).map_err(operation_error)?;
                    let edge = body
                        .edge(
                            occurrence
                                .edge
                                .ok_or_else(|| operation_error("missing edge occurrence"))?,
                        )
                        .map_err(operation_error)?;
                    let curve = edge
                        .curve
                        .as_ref()
                        .ok_or_else(|| operation_error("missing edge curve"))?;
                    let trim = eu::uv_trim_curve(body, edgeuse)
                        .map_err(operation_error)?
                        .0
                        .ok_or_else(|| operation_error("missing actual UV trim"))?;
                    let mut sample = curve
                        .tessellate(kernel, &edge.interval, deflection * 0.5, 5., 1)
                        .map_err(operation_error)?;
                    if sample.points.len() != sample.parameters.len() || sample.points.len() < 2 {
                        return Err(operation_error("edge sample correspondence"));
                    }
                    anchor_native_curve_endpoints(body, edge, &mut sample.points)?;
                    let mut endpoints = edge.vertices;
                    let mut pairs: Vec<_> =
                        sample.parameters.into_iter().zip(sample.points).collect();
                    if occurrence.orientation == OrientType::Opposite {
                        pairs.reverse();
                        endpoints.reverse();
                    }
                    if last_vertex.is_some_and(|vertex| vertex != endpoints[0]) {
                        return Err(operation_error("disconnected oriented trim occurrences"));
                    }
                    first_vertex.get_or_insert(endpoints[0]);
                    for (sample_index, (t, p)) in pairs.into_iter().enumerate() {
                        let uv = trim.evaluate_point(kernel, t).map_err(operation_error)?;
                        let point = LoopPoint {
                            u: uv.x,
                            v: uv.y,
                            p: [p.x, p.y, p.z],
                        };
                        // The same topological vertex can have two distinct UV
                        // occurrences at a seam or collapsed endpoint. Keep both;
                        // dropping the previous edge's last UV cuts across the face.
                        if sample_index == 0
                            && let Some(last) = ring.last()
                            && same_trim_uv(
                                last,
                                &point,
                                &neutral,
                                kernel,
                                surface,
                                body.tolerance(),
                            )?
                        {
                            continue;
                        }
                        ring.push(point);
                    }
                    last_vertex = Some(endpoints[1]);
                }
                if first_vertex != last_vertex {
                    return Err(operation_error("native trim loop is not closed"));
                }
                if let (Some(first), Some(last)) = (ring.first(), ring.last())
                    && same_trim_uv(first, last, &neutral, kernel, surface, body.tolerance())?
                {
                    ring.pop();
                }
                loops.push(ring);
            }
            let reversed =
                body.faceuse(faceuse).map_err(operation_error)?.orientation == OrientType::Opposite;
            let domain = surface
                .natural_uv_domain_with_nurb(kernel)
                .map_err(operation_error)?;
            trimmed_face_with_normals(
                &neutral,
                &loops,
                reversed,
                deflection,
                NewellEdgePolylines::default(),
                &FaceMeshChecks {
                    check_cancelled,
                    normal_at: &|uv| {
                        let eval = neutral.evaluate(uv[0], uv[1]).map_err(operation_error)?;
                        let normal = cross(eval.du, eval.dv);
                        let norm2 = dot(normal, normal);
                        if norm2.is_finite() && norm2 > 0. {
                            return Ok(normal);
                        }
                        // A collapsed trim endpoint may have a zero first derivative.
                        // Native EvaluateNormal computes its one-sided limit from the
                        // same exact surface; no triangle/similarity normal recovery.
                        use solidcore::nurbs::knots::Side;
                        let u_side = if uv[0] >= domain.max().x {
                            Side::Left
                        } else {
                            Side::Right
                        };
                        let v_side = if uv[1] >= domain.max().y {
                            Side::Left
                        } else {
                            Side::Right
                        };
                        let normal = surface
                            .evaluate_normal(
                                kernel,
                                solidcore::geom::vector2d::Point2::new(uv[0], uv[1]),
                                u_side,
                                v_side,
                            )
                            .map_err(operation_error)?;
                        Ok([normal.x, normal.y, normal.z])
                    },
                },
            )?
        };
        mesh.feature_edges = NewellEdgePolylines::default();
        meshes.push(mesh);
    }
    let refs: Vec<_> = meshes.iter().collect();
    check_cancelled()?;
    let mut mesh = NewellMesh::compose(&refs)?.0;
    let mut features = NewellEdgePolylines::default();
    features.offsets.push(0);
    // Seam/tangency queries may lazily construct UV data. Keep that cache work on
    // a private clone so meshing cannot mutate the owner's exact model state.
    let mut feature_body = body.clone_standing_for_self().map_err(operation_error)?;
    for id in all_edges {
        check_cancelled()?;
        if solidcore::topology::edge::is_seam(kernel, &mut feature_body, id)
            .map_err(operation_error)?
            && solidcore::topology::edge::is_tangent_edge(
                kernel,
                &mut feature_body,
                id,
                1e-12 * 180. / std::f64::consts::PI,
                false,
            )
            .map_err(operation_error)?
        {
            continue;
        }
        let edge = body.edge(id).map_err(operation_error)?;
        let curve = edge
            .curve
            .as_ref()
            .ok_or_else(|| operation_error("missing feature curve"))?;
        let mut points = curve
            .tessellate(kernel, &edge.interval, deflection * 0.5, 5., 1)
            .map_err(operation_error)?
            .points;
        anchor_native_curve_endpoints(body, edge, &mut points)?;
        for p in points {
            features.positions.push([
                crate::finite_f32(p.x)?,
                crate::finite_f32(p.y)?,
                crate::finite_f32(p.z)?,
            ]);
        }
        features
            .offsets
            .push(u32::try_from(features.positions.len()).map_err(operation_error)?);
    }
    mesh.feature_edges = features;
    check_cancelled()?;
    Ok(mesh)
}

#[cfg(test)]
pub(super) fn precision_probe(points: Vec<[f64; 3]>) -> Result<NewellMesh, NewellKernelError> {
    checked_mesh(
        points,
        vec![[0., 0., 1.]; 3],
        vec![[0, 1, 2]],
        NewellEdgePolylines::default(),
    )
}

fn validate_deflection(deflection: f64) -> Result<(), NewellKernelError> {
    if !deflection.is_finite() || deflection <= 0. {
        return Err(NewellKernelError::InvalidParameter {
            parameter: "deflection",
        });
    }
    Ok(())
}

fn same_trim_uv(
    a: &LoopPoint,
    b: &LoopPoint,
    neutral: &NurbsSurface,
    kernel: &solidcore::Kernel,
    surface: &solidcore::surface::surface::Surface,
    tolerance: f64,
) -> Result<bool, NewellKernelError> {
    if a.p != b.p {
        return Ok(false);
    }
    let domain = neutral.domain();
    let auv = [a.u, a.v];
    let buv = [b.u, b.v];
    let spans = domain.map(|d| (d[1] - d[0]).abs());
    if (0..2).all(|i| {
        (auv[i] - buv[i]).abs()
            <= (64. * f64::EPSILON * auv[i].abs().max(buv[i].abs()).max(1.)).min(spans[i] * 1e-12)
    }) {
        return Ok(true);
    }
    // Identity is already proven by the caller's shared native vertex. Distinct
    // UV charts at poles/seams remain distinct. Reconcile only a regular,
    // constant-normal chart within its existing native face tolerance.
    let eval_a = neutral.evaluate(a.u, a.v).map_err(operation_error)?;
    let eval_b = neutral.evaluate(b.u, b.v).map_err(operation_error)?;
    let mut normals = Vec::new();
    for eval in [eval_a, eval_b] {
        let normal = cross(eval.du, eval.dv);
        let lengths = [dot(eval.du, eval.du).sqrt(), dot(eval.dv, eval.dv).sqrt()];
        if (0..2).any(|i| !lengths[i].is_finite() || lengths[i] <= tolerance / spans[i])
            || dot(normal, normal) <= 0.
        {
            return Ok(false);
        }
        normals.push(unit(normal)?);
    }
    let cross_norm = cross(normals[0], normals[1]);
    if dot(cross_norm, cross_norm).sqrt() > 64. * f64::EPSILON || dot(normals[0], normals[1]) <= 0.
    {
        return Ok(false);
    }
    let point_delta = sub(eval_a.point, eval_b.point);
    if dot(point_delta, point_delta).sqrt() > tolerance {
        return Ok(false);
    }
    let mut uv_tolerance = f64::INFINITY;
    for uv in [auv, buv] {
        uv_tolerance = uv_tolerance.min(
            solidcore::surface::surface::p1::map_to_2d(
                kernel,
                tolerance,
                solidcore::geom::vector2d::Point2::new(uv[0], uv[1]),
                surface,
            )
            .map_err(operation_error)?,
        );
    }
    Ok(uv_tolerance.is_finite()
        && uv_tolerance > 0.
        && (0..2).all(|i| (auv[i] - buv[i]).abs() <= uv_tolerance.min(spans[i] * 1e-6)))
}

/// Anchor only curve endpoints to their actual topological vertex, qualified by
/// that existing native edge/vertex tolerance. This is incidence provenance, not
/// geometric matching, and changes no model data or modeling tolerance.
fn anchor_native_curve_endpoints(
    body: &solidcore::Brep,
    edge: &solidcore::topology::Edge,
    points: &mut [solidcore::math::Point3],
) -> Result<(), NewellKernelError> {
    if points.len() < 2 {
        return Err(operation_error("incomplete native edge samples"));
    }
    for (index, vertex) in [(0, edge.vertices[0]), (points.len() - 1, edge.vertices[1])] {
        let vertex = body.vertex(vertex).map_err(operation_error)?;
        let point = points
            .get_mut(index)
            .ok_or_else(|| operation_error("native endpoint sample"))?;
        let tolerance = edge.tolerance + vertex.tolerance;
        if !tolerance.is_finite() || tolerance <= 0. || point.distance(vertex.point) > tolerance {
            return Err(operation_error(
                "curve endpoint exceeds native vertex tolerance",
            ));
        }
        *point = vertex.point;
    }
    Ok(())
}
