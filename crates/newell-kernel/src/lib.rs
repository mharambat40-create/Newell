// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Geometric kernel boundary over BrepKIT (Model layer, `ARCHITECTURE.md` §1, §5.1).

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
