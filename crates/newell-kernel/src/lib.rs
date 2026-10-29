// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Geometric kernel boundary over BrepKIT (Model layer, `ARCHITECTURE.md` §1, §5.1).

#[cfg(test)]
mod sketch;

mod boolean;
mod extrude;
pub use boolean::NewellSurfaceResult;
mod termination;
pub use extrude::{NewellBodyResult, NewellGeometry, NewellShapeKey};
pub use termination::{
    NewellFaceTarget, NewellSurfaceExtent, NewellSurfaceExtrude, NewellSurfaceSides,
    NewellSurfaceStart,
};
mod surface;
pub use surface::{NewellBooleanMode, NewellBoundary, NewellSurface};
mod profile;
pub use profile::{
    NewellFaceProvenance, NewellProfile, NewellProfileEdge, NewellProfileFace, NewellProfileFrame,
    NewellProfileWire,
};

use brepkit_operations::primitives::make_box;
use brepkit_operations::tessellate::sample_solid_edges_filtered;
use brepkit_operations::tessellate::tessellate_solid_grouped_with_tolerance;
use brepkit_topology::{SolidId, Topology};

/// Mesh data owned by Newell and safe to pass to the renderer.
///
/// Positions and normals are tightly packed XYZ triples. `indices` addresses `positions`; faces
/// are contiguous ranges in `indices`, delimited by `face_offsets`.
#[derive(Debug, Clone, PartialEq)]
pub struct NewellMesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
    face_offsets: Vec<u32>,
    feature_edges: NewellEdgePolylines,
}

/// Sampled B-Rep feature edges, independent of triangle tessellation.
/// Each range `offsets[i]..offsets[i + 1]` is one polyline.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NewellEdgePolylines {
    positions: Vec<[f32; 3]>,
    offsets: Vec<u32>,
}

impl NewellEdgePolylines {
    #[must_use]
    pub fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }

    #[must_use]
    pub fn offsets(&self) -> &[u32] {
        &self.offsets
    }
}

impl NewellMesh {
    /// Vertex positions in model coordinates.
    #[must_use]
    pub fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }

    /// Smooth vertex normals corresponding one-for-one with positions.
    #[must_use]
    pub fn normals(&self) -> &[[f32; 3]] {
        &self.normals
    }

    /// Triangle indices, in triples.
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Face-to-index boundaries. The final entry equals `indices().len()`.
    #[must_use]
    pub fn face_offsets(&self) -> &[u32] {
        &self.face_offsets
    }

    /// Number of B-Rep faces represented by this mesh.
    #[must_use]
    pub fn face_count(&self) -> usize {
        self.face_offsets.len().saturating_sub(1)
    }

    /// Sampled geometry edges; generated inside the BrepKIT boundary.
    #[must_use]
    pub fn feature_edges(&self) -> &NewellEdgePolylines {
        &self.feature_edges
    }
}

/// A solid owned by Newell. BrepKIT's topology arena and handles never escape this boundary.
#[derive(Debug)]
pub struct NewellSolid {
    topology: Topology,
    solid: SolidId,
    measured_volume: Option<f64>,
}

/// Errors translated from the kernel boundary into Newell-owned terms.
#[derive(Debug, thiserror::Error)]
pub enum NewellKernelError {
    /// Dimensions and tessellation inputs must be finite positive numbers.
    #[error("{parameter} must be a finite positive value")]
    InvalidParameter { parameter: &'static str },
    /// BrepKIT rejected a requested modeling or tessellation operation.
    #[error("BrepKIT operation failed: {message}")]
    Operation { message: String },
    /// The kernel returned data that cannot be represented by the renderer's f32 buffers.
    #[error("BrepKIT returned a non-finite mesh value")]
    NonFiniteMesh,
    /// Conversion to renderer coordinates collapsed or reversed actual triangle detail.
    #[error("geometry detail cannot be represented in renderer coordinates")]
    MeshPrecisionLoss,
}

impl NewellSolid {
    /// Measures the actual B-Rep using the pinned kernel.
    ///
    /// # Errors
    /// Propagates kernel measurement errors.
    pub fn volume(&self) -> Result<f64, NewellKernelError> {
        if let Some(volume) = self.measured_volume {
            return Ok(volume);
        }
        if let Some(volume) = profile::parallel_sweep_volume(&self.topology, self.solid)? {
            return Ok(volume);
        }
        brepkit_operations::measure::solid_volume(&self.topology, self.solid, 0.001)
            .map_err(operation_error)
    }
    /// Checks the B-Rep manifold and geometry using the pinned kernel.
    ///
    /// # Errors
    /// Returns the concrete validation issues when the solid is invalid.
    pub fn validate(&self) -> Result<(), NewellKernelError> {
        let report = brepkit_operations::validate::validate_solid(&self.topology, self.solid)
            .map_err(operation_error)?;
        if !report.is_valid() {
            return Err(NewellKernelError::Operation {
                message: format!("{:?}", report.issues),
            });
        }
        Ok(())
    }
    /// Constructs an axis-aligned box rooted at the model origin.
    ///
    /// # Errors
    /// If a dimension is not finite and positive, or BrepKIT cannot create the solid.
    pub fn box_solid(dx: f64, dy: f64, dz: f64) -> Result<Self, NewellKernelError> {
        for (value, parameter) in [(dx, "dx"), (dy, "dy"), (dz, "dz")] {
            if !value.is_finite() || value <= 0.0 {
                return Err(NewellKernelError::InvalidParameter { parameter });
            }
        }
        let mut topology = Topology::new();
        let solid = make_box(&mut topology, dx, dy, dz).map_err(operation_error)?;
        Ok(Self {
            topology,
            solid,
            measured_volume: None,
        })
    }

    /// Creates a renderer-ready triangle mesh without exposing BrepKIT topology.
    ///
    /// # Errors
    /// If `deflection` is invalid, tessellation fails, or a generated coordinate is not finite.
    pub fn tessellate(&self, deflection: f64) -> Result<NewellMesh, NewellKernelError> {
        if !deflection.is_finite() || deflection <= 0.0 {
            return Err(NewellKernelError::InvalidParameter {
                parameter: "deflection",
            });
        }
        let (mesh, face_offsets) =
            tessellate_solid_grouped_with_tolerance(&self.topology, self.solid, deflection, 0.0)
                .map_err(operation_error)?;
        let positions = mesh
            .positions
            .iter()
            .map(point_to_f32)
            .collect::<Result<Vec<_>, _>>()?;
        validate_mesh_precision(&mesh.positions, &positions, &mesh.indices)?;
        let normals = mesh
            .normals
            .iter()
            .map(vector_to_f32)
            .collect::<Result<Vec<_>, _>>()?;
        // Match the surface tessellator's linear-only edge sampling. BrepKIT filters
        // co-surface seams while retaining distinct topological feature boundaries.
        let sampled =
            sample_solid_edges_filtered(&self.topology, self.solid, deflection, 0.0, true)
                .map_err(operation_error)?;
        let mut feature_edges = NewellEdgePolylines {
            positions: Vec::new(),
            offsets: vec![0],
        };
        for (edge_number, &start) in sampled.offsets.iter().enumerate() {
            let end = sampled
                .offsets
                .get(edge_number + 1)
                .copied()
                .unwrap_or(sampled.positions.len());
            let Some(points) = sampled.positions.get(start..end) else {
                return Err(NewellKernelError::Operation {
                    message: "invalid sampled feature edge range".to_owned(),
                });
            };
            let mut line = Vec::with_capacity(points.len());
            for point in points {
                let position = point_to_f32(point)?;
                if line.last() != Some(&position) {
                    line.push(position);
                }
            }
            if line.len() < 2 {
                continue;
            }
            feature_edges.positions.extend(line);
            feature_edges
                .offsets
                .push(u32::try_from(feature_edges.positions.len()).map_err(|_| {
                    NewellKernelError::Operation {
                        message: "too many sampled feature edge vertices".to_owned(),
                    }
                })?);
        }
        Ok(NewellMesh {
            positions,
            normals,
            indices: mesh.indices,
            face_offsets,
            feature_edges,
        })
    }
}

fn operation_error(error: impl std::fmt::Display) -> NewellKernelError {
    NewellKernelError::Operation {
        message: error.to_string(),
    }
}

fn validate_mesh_precision(
    original: &[brepkit_math::vec::Point3],
    converted: &[[f32; 3]],
    indices: &[u32],
) -> Result<(), NewellKernelError> {
    use brepkit_math::vec::Point3;
    if original.len() != converted.len() || !indices.len().is_multiple_of(3) {
        return Err(operation_error("invalid mesh buffer lengths"));
    }
    for triangle in indices.as_chunks::<3>().0 {
        let mut exact = [Point3::new(0., 0., 0.); 3];
        let mut rounded = exact;
        for (corner, index) in triangle.iter().enumerate() {
            let index = usize::try_from(*index).map_err(operation_error)?;
            exact[corner] = *original
                .get(index)
                .ok_or_else(|| operation_error("mesh vertex index"))?;
            let point = converted
                .get(index)
                .ok_or_else(|| operation_error("mesh vertex index"))?;
            rounded[corner] = Point3::new(
                f64::from(point[0]),
                f64::from(point[1]),
                f64::from(point[2]),
            );
        }
        let before = (exact[1] - exact[0]).cross(exact[2] - exact[0]);
        let after = (rounded[1] - rounded[0]).cross(rounded[2] - rounded[0]);
        // Degenerate source triangles are the tessellator's responsibility. An
        // actual area may never disappear or flip merely during conversion.
        if before.length_squared() > 0. && before.dot(after) <= 0. {
            return Err(NewellKernelError::MeshPrecisionLoss);
        }
    }
    Ok(())
}

fn finite_f32(value: f64) -> Result<f32, NewellKernelError> {
    if value.is_finite() && value >= f64::from(f32::MIN) && value <= f64::from(f32::MAX) {
        Ok(value as f32)
    } else {
        Err(NewellKernelError::NonFiniteMesh)
    }
}

fn point_to_f32(point: &brepkit_math::vec::Point3) -> Result<[f32; 3], NewellKernelError> {
    Ok([
        finite_f32(point.x())?,
        finite_f32(point.y())?,
        finite_f32(point.z())?,
    ])
}

fn vector_to_f32(vector: &brepkit_math::vec::Vec3) -> Result<[f32; 3], NewellKernelError> {
    Ok([
        finite_f32(vector.x())?,
        finite_f32(vector.y())?,
        finite_f32(vector.z())?,
    ])
}

#[cfg(test)]
mod tests {
    use super::NewellSolid;
    use brepkit_math::mat::Mat4;
    use brepkit_operations::boolean::{BooleanOp, BooleanOptions, boolean_with_options};
    use brepkit_operations::primitives::make_cylinder;
    use brepkit_operations::tessellate::sample_solid_edges_filtered;
    use brepkit_operations::transform::transform_solid;
    use brepkit_topology::Topology;

    #[test]
    fn cylinder_omits_periodic_seam_but_keeps_both_rims() {
        let mut topology = Topology::new();
        let solid = make_cylinder(&mut topology, 1.0, 2.0).unwrap();
        let mesh = NewellSolid {
            topology,
            solid,
            measured_volume: None,
        }
        .tessellate(0.05)
        .unwrap();
        let edges = mesh.feature_edges();
        assert_eq!(edges.offsets().len(), 3);
        assert!(edges.positions().len() > 16);
        assert!(edges.offsets().windows(2).all(|r| r[1] > r[0] + 2));
    }

    #[test]
    fn coplanar_boolean_face_splits_are_not_feature_edges() {
        let mut topology = Topology::new();
        let a = super::make_box(&mut topology, 10.0, 10.0, 10.0).unwrap();
        let b = super::make_box(&mut topology, 10.0, 6.0, 10.0).unwrap();
        transform_solid(&mut topology, b, &Mat4::translation(10.0, 0.0, 0.0)).unwrap();
        let fused = boolean_with_options(
            &mut topology,
            BooleanOp::Fuse,
            a,
            b,
            BooleanOptions {
                unify_faces: false,
                ..Default::default()
            },
        )
        .unwrap();
        let all = sample_solid_edges_filtered(&topology, fused, 0.1, 0.0, false).unwrap();
        let filtered = NewellSolid {
            topology,
            solid: fused,
            measured_volume: None,
        }
        .tessellate(0.1)
        .unwrap();
        assert_eq!(
            all.offsets.len() - (filtered.feature_edges().offsets().len() - 1),
            3,
            "the three coplanar seams must be absent"
        );
    }
}

/// Exact item ranges in a composed display/query mesh. B-Rep ordinals remain item-local.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewellMeshRange {
    pub index_start: u32,
    pub index_end: u32,
    pub face_start: u32,
    pub face_end: u32,
}
impl NewellMesh {
    /// Composes owned display buffers with explicit per-item face/index ranges.
    /// This only joins meshes; it never creates or substitutes model geometry.
    ///
    /// # Errors
    /// Rejects index/count overflow and malformed input buffers before publishing a mesh.
    pub fn compose(items: &[&Self]) -> Result<(Self, Vec<NewellMeshRange>), NewellKernelError> {
        let mut mesh = Self {
            positions: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
            face_offsets: vec![0],
            feature_edges: NewellEdgePolylines {
                positions: Vec::new(),
                offsets: vec![0],
            },
        };
        let mut ranges = Vec::new();
        let capacity_error = || NewellKernelError::Operation {
            message: "composed scene mesh exceeds index capacity".into(),
        };
        for item in items {
            let vertex = u32::try_from(mesh.positions.len()).map_err(|_| capacity_error())?;
            let index_start = u32::try_from(mesh.indices.len()).map_err(|_| capacity_error())?;
            let face_start = u32::try_from(mesh.face_count()).map_err(|_| capacity_error())?;
            if item.positions.len() != item.normals.len()
                || item.face_offsets.first() != Some(&0)
                || item.face_offsets.last().copied() != u32::try_from(item.indices.len()).ok()
                || item.indices.len() % 3 != 0
            {
                return Err(NewellKernelError::NonFiniteMesh);
            }
            let total = mesh
                .positions
                .len()
                .checked_add(item.positions.len())
                .ok_or_else(capacity_error)?;
            u32::try_from(total).map_err(|_| capacity_error())?;
            for index in &item.indices {
                if usize::try_from(*index).map_or(true, |index| index >= item.positions.len()) {
                    return Err(NewellKernelError::NonFiniteMesh);
                }
                mesh.indices
                    .push(vertex.checked_add(*index).ok_or_else(capacity_error)?);
            }
            mesh.positions.extend_from_slice(&item.positions);
            mesh.normals.extend_from_slice(&item.normals);
            for offset in item.face_offsets.iter().skip(1) {
                mesh.face_offsets.push(
                    index_start
                        .checked_add(*offset)
                        .ok_or_else(capacity_error)?,
                );
            }
            let edge_start =
                u32::try_from(mesh.feature_edges.positions.len()).map_err(|_| capacity_error())?;
            mesh.feature_edges
                .positions
                .extend_from_slice(&item.feature_edges.positions);
            for offset in item.feature_edges.offsets.iter().skip(1) {
                mesh.feature_edges
                    .offsets
                    .push(edge_start.checked_add(*offset).ok_or_else(capacity_error)?);
            }
            let index_end = u32::try_from(mesh.indices.len()).map_err(|_| capacity_error())?;
            let face_end = u32::try_from(mesh.face_count()).map_err(|_| capacity_error())?;
            ranges.push(NewellMeshRange {
                index_start,
                index_end,
                face_start,
                face_end,
            });
        }
        Ok((mesh, ranges))
    }
}

#[cfg(test)]
mod sketch_feasibility {
    use super::*;
    use brepkit_math::vec::{Point3, Vec3};
    use brepkit_topology::{
        Topology,
        builder::{make_circle_edge, make_polygon_wire},
        face::{Face, FaceSurface},
        wire::{OrientedEdge, Wire},
    };
    /// Uses the very same face-copy/transform/extrude/validate arena method as
    /// M4's worker. The source is exact native topology, including a circular hole.
    #[test]
    fn m50_real_m4_extrusion_preserves_oriented_holed_profiles()
    -> Result<(), Box<dyn std::error::Error>> {
        for shift in [0., 1e9] {
            for normal in [Vec3::new(0., 0., 1.), Vec3::new(0., 0., -1.)] {
                let mut topology = Topology::new();
                let wire = make_polygon_wire(
                    &mut topology,
                    &[
                        Point3::new(shift, shift, 0.),
                        Point3::new(shift + 10., shift, 0.),
                        Point3::new(shift + 10., shift + 10., 0.),
                        Point3::new(shift, shift + 10., 0.),
                    ],
                    1e-7,
                )?;
                let circle = make_circle_edge(
                    &mut topology,
                    Point3::new(shift + 5., shift + 5., 0.),
                    Vec3::new(0., 0., 1.),
                    0.1,
                    1e-7,
                )?;
                let hole =
                    topology.add_wire(Wire::new(vec![OrientedEdge::new(circle, false)], true)?);
                let face = topology.add_face(Face::new(
                    wire,
                    vec![hole],
                    FaceSurface::Plane { normal, d: 0. },
                ));
                let solid = brepkit_operations::extrude::extrude(
                    &mut topology,
                    face,
                    Vec3::new(0., 0., 1.),
                    1.,
                )?;
                let mut arena = NewellGeometry::new();
                arena.topology = topology;
                let source = arena.insert(solid)?;
                let profile = arena
                    .profile_faces(source)?
                    .into_iter()
                    .find(|f| f.wires.len() == 2)
                    .ok_or("lost hole")?;
                let began = std::time::Instant::now();
                let output = arena.extrude_face(source, profile.face, None, 0., 2.)?;
                let expected = (100. - std::f64::consts::PI * 0.01) * 2.;
                assert!((arena.volume(output)? - expected).abs() < 1e-5);
                println!(
                    "M4_HOLE,shift={shift:e},normal={normal:?},volume={:e},arena_extrude_us={:.3}",
                    arena.volume(output)?,
                    began.elapsed().as_secs_f64() * 1e6
                );
                let caps = arena.profile_faces(output)?;
                assert!(caps.iter().filter(|f| f.wires.len() == 2).count() >= 2);
            }
        }
        Ok(())
    }
    #[test]
    fn m50_real_m4_arena_preserves_cubic_profile() -> Result<(), Box<dyn std::error::Error>> {
        let profile = NewellProfile::closed_cubic([[0., 0.], [2., 3.], [4., 2.], [6., 0.]])?;
        let seed = profile.extrude([0., 0., 1.], 1.)?;
        let mut arena = NewellGeometry::new();
        arena.topology = seed.topology;
        let source = arena.insert(seed.solid)?;
        let face = arena
            .profile_faces(source)?
            .into_iter()
            .find(|f| f.frame.normal[2].abs() > 0.9)
            .ok_or("missing cubic cap")?;
        let result = arena.extrude_face(source, face.face, None, 0., 2.)?;
        assert!((arena.volume(result)? - 15.).abs() < 1e-6);
        Ok(())
    }
    /// The font outlines remain exact polynomial edges. Known O/B/8 contours
    /// provide independent hole counts; this is not a general region extractor.
    #[test]
    fn m50_exact_font_contours_survive_real_m4_extrusion() -> Result<(), Box<dyn std::error::Error>>
    {
        use ab_glyph::{Font, FontRef, OutlineCurve};
        use brepkit_math::{nurbs::curve::NurbsCurve, quadrature::gauss_legendre_points};
        use brepkit_topology::{
            edge::{Edge, EdgeCurve},
            vertex::Vertex,
        };
        let font = FontRef::try_from_slice(include_bytes!("../../../assets/FiraCode-Regular.ttf"))?;
        let scale = 10. / f64::from(font.units_per_em().ok_or("missing font units")?);
        // Independent fontTools 4.60.1 AreaPen integration of the bundled glyf data.
        for (character, expected_holes, raw_area) in [
            ('O', 1, 563343.5416666667),
            ('B', 2, 637555.),
            ('8', 2, 589791.541666667),
        ] {
            let outline = font
                .outline(font.glyph_id(character))
                .ok_or("missing glyph")?;
            let mut contours: Vec<Vec<Vec<ab_glyph::Point>>> = Vec::new();
            let mut active = Vec::new();
            let mut synthetic_closures = 0;
            for c in outline.curves {
                // ab_glyph appends a closing line even when the final segment
                // already reaches its start. Keep this parser artifact out of topology.
                if matches!(c, OutlineCurve::Line(a,b) if a==b) {
                    synthetic_closures += 1;
                    continue;
                }
                let points = match c {
                    OutlineCurve::Line(a, b) => vec![a, b],
                    OutlineCurve::Quad(a, b, c) => vec![a, b, c],
                    OutlineCurve::Cubic(a, b, c, d) => vec![a, b, c, d],
                };
                for point in &points {
                    assert!((point.x * 2.).fract() == 0. && (point.y * 2.).fract() == 0.);
                }
                if let Some(previous) = active.last().and_then(|p: &Vec<ab_glyph::Point>| p.last())
                {
                    assert_eq!(*previous, points[0]);
                }
                active.push(points);
                if active.first().and_then(|p| p.first()) == active.last().and_then(|p| p.last()) {
                    contours.push(std::mem::take(&mut active));
                }
            }
            println!(
                "FONT_OUTLINE,{character},contours={},synthetic_zero_length_closures={synthetic_closures}",
                contours.len()
            );
            assert!(active.is_empty());
            assert_eq!(contours.len(), expected_holes + 1);
            let mut topology = Topology::new();
            let mut loops = Vec::new();
            for contour in contours {
                let start = contour[0][0];
                let anchor = topology.add_vertex(Vertex::new(
                    Point3::new(f64::from(start.x) * scale, f64::from(start.y) * scale, 0.),
                    1e-7,
                ));
                let mut previous = anchor;
                let mut edges = Vec::new();
                let mut twice_area = 0.;
                let count = contour.len();
                for (i, points) in contour.into_iter().enumerate() {
                    let degree = points.len() - 1;
                    let controls: Vec<_> = points
                        .iter()
                        .map(|p| Point3::new(f64::from(p.x) * scale, f64::from(p.y) * scale, 0.))
                        .collect();
                    let end = if i + 1 == count {
                        anchor
                    } else {
                        topology
                            .add_vertex(Vertex::new(*controls.last().ok_or("empty control")?, 1e-7))
                    };
                    let curve = NurbsCurve::new(
                        degree,
                        std::iter::repeat_n(0., degree + 1)
                            .chain(std::iter::repeat_n(1., degree + 1))
                            .collect(),
                        controls,
                        vec![1.; degree + 1],
                    )?;
                    for q in gauss_legendre_points(8) {
                        let t = (q.x + 1.) / 2.;
                        let p = curve.evaluate(t);
                        let derivative = curve.derivatives(t, 1)[1];
                        twice_area += q.w * (p.x() * derivative.y() - p.y() * derivative.x()) / 2.;
                    }
                    let edge =
                        topology.add_edge(Edge::new(previous, end, EdgeCurve::NurbsCurve(curve)));
                    edges.push(OrientedEdge::new(edge, true));
                    previous = end;
                }
                loops.push((twice_area / 2., edges));
            }
            loops.sort_by(|a, b| b.0.abs().total_cmp(&a.0.abs()));
            let area = loops[0].0.abs() - loops.iter().skip(1).map(|p| p.0.abs()).sum::<f64>();
            assert!(area > 0.);
            assert!((area - raw_area * scale * scale).abs() < 1e-10);
            let mut wires = Vec::new();
            for (i, (signed_area, mut edges)) in loops.into_iter().enumerate() {
                if (signed_area > 0.) != (i == 0) {
                    edges = edges
                        .into_iter()
                        .rev()
                        .map(|e| OrientedEdge::new(e.edge(), false))
                        .collect();
                }
                wires.push(topology.add_wire(Wire::new(edges, true)?));
            }
            let face = topology.add_face(Face::new(
                wires[0],
                wires[1..].to_vec(),
                FaceSurface::Plane {
                    normal: Vec3::new(0., 0., 1.),
                    d: 0.,
                },
            ));
            let seed = brepkit_operations::extrude::extrude(
                &mut topology,
                face,
                Vec3::new(0., 0., 1.),
                1.,
            )?;
            let mut arena = NewellGeometry::new();
            arena.topology = topology;
            let source = arena.insert(seed)?;
            let profile = arena
                .profile_faces(source)?
                .into_iter()
                .find(|f| f.wires.len() == expected_holes + 1)
                .ok_or("missing font cap holes")?;
            let began = std::time::Instant::now();
            let output = arena.extrude_face(source, profile.face, None, 0., 2.)?;
            let duration = began.elapsed();
            let volume = arena.volume(output)?;
            assert!((volume - area * 2.).abs() < 1e-6);
            assert_eq!(
                arena
                    .profile_faces(output)?
                    .iter()
                    .filter(|f| f.wires.len() == expected_holes + 1)
                    .count(),
                2
            );
            println!(
                "M4_FONT,{character},holes={expected_holes},area={area:e},volume={volume:e},expected_volume={:e},arena_extrude_us={:.3},coordinates=exact_half_font_units_promoted_before_scaling",
                area * 2.,
                duration.as_secs_f64() * 1e6
            );
        }
        Ok(())
    }
}
