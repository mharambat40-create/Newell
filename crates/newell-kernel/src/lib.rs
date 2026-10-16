// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Geometric kernel boundary over BrepKIT (Model layer, `ARCHITECTURE.md` §1, §5.1).

use brepkit_operations::primitives::make_box;
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
}

/// A solid owned by Newell. BrepKIT's topology arena and handles never escape this boundary.
#[derive(Debug)]
pub struct NewellSolid {
    topology: Topology,
    solid: SolidId,
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
}

impl NewellSolid {
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
        Ok(Self { topology, solid })
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
        let normals = mesh
            .normals
            .iter()
            .map(vector_to_f32)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(NewellMesh {
            positions,
            normals,
            indices: mesh.indices,
            face_offsets,
        })
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
