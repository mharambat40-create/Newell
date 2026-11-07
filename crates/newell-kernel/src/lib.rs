// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! SolidCore geometry boundary.
mod solidcore;
mod solidcore_mesh;
mod types;
pub use solidcore::SolidCoreGeometry as NewellGeometry;
pub use types::*;
#[cfg(feature = "rank-research")]
mod sketch;
#[cfg(feature = "rank-research")]
pub use sketch::rank::{
    RankAssessment, RankElimination, RankInputProvenance, RankJobIdentity, translate as assess_rank,
};
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

    /// Sampled geometry edges; generated inside the SolidCore boundary.
    #[must_use]
    pub fn feature_edges(&self) -> &NewellEdgePolylines {
        &self.feature_edges
    }
}

/// Errors translated from the kernel boundary into Newell-owned terms.
#[derive(Debug, thiserror::Error)]
pub enum NewellKernelError {
    /// Dimensions and tessellation inputs must be finite positive numbers.
    #[error("{parameter} must be a finite positive value")]
    InvalidParameter { parameter: &'static str },
    /// SolidCore rejected a requested modeling or tessellation operation.
    #[error("SolidCore operation failed: {message}")]
    Operation { message: String },
    /// The kernel returned data that cannot be represented by the renderer's f32 buffers.
    #[error("SolidCore returned a non-finite mesh value")]
    NonFiniteMesh,
    /// Conversion to renderer coordinates collapsed or reversed actual triangle detail.
    #[error("geometry detail cannot be represented in renderer coordinates")]
    MeshPrecisionLoss,
}

/// Standalone owned material for fixtures and navigation meshes.
pub struct NewellSolid {
    owner: NewellGeometry,
    key: NewellShapeKey,
}
impl NewellSolid {
    pub fn box_solid(dx: f64, dy: f64, dz: f64) -> Result<Self, NewellKernelError> {
        for (value, parameter) in [(dx, "dx"), (dy, "dy"), (dz, "dz")] {
            if !value.is_finite() || value <= 0. {
                return Err(NewellKernelError::InvalidParameter { parameter });
            }
        }
        let mut owner = NewellGeometry::new();
        let key = owner.box_shape([dx, dy, dz], [0.; 3])?;
        Ok(Self { owner, key })
    }
    pub fn volume(&self) -> Result<f64, NewellKernelError> {
        self.owner.volume(self.key)
    }
    pub fn validate(&self) -> Result<(), NewellKernelError> {
        self.volume().map(|_| ())
    }
    pub fn tessellate(&self, deflection: f64) -> Result<NewellMesh, NewellKernelError> {
        self.owner.tessellate(self.key, deflection)
    }
}
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

fn operation_error(error: impl std::fmt::Display) -> NewellKernelError {
    NewellKernelError::Operation {
        message: error.to_string(),
    }
}
fn finite_f32(value: f64) -> Result<f32, NewellKernelError> {
    if value.is_finite() && value >= f64::from(f32::MIN) && value <= f64::from(f32::MAX) {
        Ok(value as f32)
    } else {
        Err(NewellKernelError::NonFiniteMesh)
    }
}
