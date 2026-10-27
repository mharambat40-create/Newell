// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Worker-local exact geometry arena. Only owned recipes/meshes leave its thread.
use crate::{
    NewellBooleanMode, NewellKernelError, NewellMesh, NewellProfileFace, NewellSolid,
    operation_error,
};
use brepkit_math::{mat::Mat4, vec::Vec3};
use brepkit_topology::{SolidId, Topology};
use std::collections::BTreeMap;

/// Arena-local Newell identity, never a BrepKIT handle or a cross-thread geometry token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NewellShapeKey(u64);
/// All topology is created and retained inside the one geometry worker.
#[derive(Debug, Default)]
pub struct NewellGeometry {
    topology: Topology,
    solids: BTreeMap<NewellShapeKey, SolidId>,
    next: u64,
}
impl NewellGeometry {
    /// Creates an empty worker-local arena, with no native data supplied by the caller.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    fn insert(&mut self, solid: SolidId) -> Result<NewellShapeKey, NewellKernelError> {
        let next = self
            .next
            .checked_add(1)
            .ok_or_else(|| NewellKernelError::Operation {
                message: "geometry identity exhausted".into(),
            })?;
        let key = NewellShapeKey(self.next);
        self.solids.insert(key, solid);
        self.next = next;
        Ok(key)
    }
    fn solid(&self, key: NewellShapeKey) -> Result<SolidId, NewellKernelError> {
        self.solids
            .get(&key)
            .copied()
            .ok_or_else(|| NewellKernelError::Operation {
                message: "geometry key is not in this arena".into(),
            })
    }
    /// Executes a real box recipe at the given origin.
    ///
    /// # Errors
    /// Rejects invalid dimensions/origin and failed exact construction.
    pub fn box_shape(
        &mut self,
        size: [f64; 3],
        origin: [f64; 3],
    ) -> Result<NewellShapeKey, NewellKernelError> {
        if size.iter().any(|v| !v.is_finite() || *v <= 1e-7)
            || origin.iter().any(|v| !v.is_finite())
        {
            return Err(NewellKernelError::InvalidParameter {
                parameter: "box recipe",
            });
        }
        let solid =
            brepkit_operations::primitives::make_box(&mut self.topology, size[0], size[1], size[2])
                .map_err(operation_error)?;
        brepkit_operations::transform::transform_solid(
            &mut self.topology,
            solid,
            &Mat4::translation(origin[0], origin[1], origin[2]),
        )
        .map_err(operation_error)?;
        self.insert(solid)
    }
    /// Resolves and sweeps a genuine planar face of an existing generation's shape.
    ///
    /// # Errors
    /// Rejects absent faces, tangential Along, invalid extents and kernel failure.
    pub fn extrude_face(
        &mut self,
        profile: NewellShapeKey,
        face: u32,
        along: Option<[f64; 3]>,
        start: f64,
        end: f64,
    ) -> Result<NewellShapeKey, NewellKernelError> {
        if !start.is_finite() || !end.is_finite() || end - start <= 1e-7 {
            return Err(NewellKernelError::InvalidParameter {
                parameter: "extrusion interval",
            });
        }
        let source = self.solid(profile)?;
        let faces = brepkit_topology::explorer::solid_faces(&self.topology, source)
            .map_err(operation_error)?;
        let face = *faces
            .get(usize::try_from(face).map_err(operation_error)?)
            .ok_or_else(|| NewellKernelError::Operation {
                message: "missing profile face".into(),
            })?;
        let frame = crate::profile::supporting_frame(&self.topology, face)?;
        let normal = Vec3::new(frame.normal[0], frame.normal[1], frame.normal[2]);
        let direction = along.unwrap_or(frame.normal);
        if direction.iter().any(|v| !v.is_finite()) {
            return Err(NewellKernelError::InvalidParameter { parameter: "Along" });
        }
        let direction = Vec3::new(direction[0], direction[1], direction[2])
            .normalize()
            .map_err(operation_error)?;
        if direction.dot(normal).abs() <= 1e-12 {
            return Err(NewellKernelError::InvalidParameter {
                parameter: "tangential Along",
            });
        }
        // Copy the selected face into fresh topology occurrences before displacement: a
        // profile is also part of a historical body and must never be moved in place.
        let face = brepkit_operations::copy::copy_face(&mut self.topology, face)
            .map_err(operation_error)?;
        let shift = direction * start;
        brepkit_operations::transform::transform_face(
            &mut self.topology,
            face,
            &Mat4::translation(shift.x(), shift.y(), shift.z()),
        )
        .map_err(operation_error)?;
        let solid =
            brepkit_operations::extrude::extrude(&mut self.topology, face, direction, end - start)
                .map_err(operation_error)?;
        self.validate_native(solid)?;
        self.insert(solid)
    }
    fn validate_native(&self, solid: SolidId) -> Result<(), NewellKernelError> {
        let validation = brepkit_operations::validate::validate_solid(&self.topology, solid)
            .map_err(operation_error)?;
        if !validation.is_valid() {
            return Err(NewellKernelError::Operation {
                message: format!("invalid result: {:?}", validation.issues),
            });
        }
        Ok(())
    }
    /// Exact solid operation. A full cut returns None, distinct from a kernel failure.
    ///
    /// # Errors
    /// Rejects mesh fallback and invalid or failed B-Rep results.
    pub fn boolean(
        &mut self,
        target: NewellShapeKey,
        tool: NewellShapeKey,
        mode: NewellBooleanMode,
    ) -> Result<Option<NewellShapeKey>, NewellKernelError> {
        use brepkit_operations::boolean::{BooleanOp, boolean, mesh_fallback_count};
        let (a, b) = (self.solid(target)?, self.solid(tool)?);
        let before = mesh_fallback_count();
        let result = boolean(
            &mut self.topology,
            match mode {
                NewellBooleanMode::Add => BooleanOp::Fuse,
                NewellBooleanMode::Remove => BooleanOp::Cut,
            },
            a,
            b,
        );
        if before != mesh_fallback_count() {
            return Err(NewellKernelError::Operation {
                message: "exact boolean attempted mesh fallback".into(),
            });
        }
        let solid = match result {
            Err(brepkit_operations::OperationsError::EmptyResult { .. })
                if mode == NewellBooleanMode::Remove =>
            {
                return Ok(None);
            }
            result => result.map_err(operation_error)?,
        };
        if self.topology.is_empty_solid(solid) {
            return Ok(None);
        }
        self.validate_native(solid)?;
        self.insert(solid).map(Some)
    }
    fn owned_snapshot(&self, key: NewellShapeKey) -> Result<NewellSolid, NewellKernelError> {
        Ok(NewellSolid {
            topology: self.topology.clone(),
            solid: self.solid(key)?,
            measured_volume: None,
        })
    }
    /// Generates the owned mesh for a staged scene. No native handle leaves the arena.
    ///
    /// # Errors
    /// Propagates missing shape and tessellation errors.
    pub fn tessellate(
        &self,
        key: NewellShapeKey,
        deflection: f64,
    ) -> Result<NewellMesh, NewellKernelError> {
        self.owned_snapshot(key)?.tessellate(deflection)
    }
    /// Exact generation's candidate faces and oriented boundary occurrences.
    ///
    /// # Errors
    /// Propagates missing shapes and invalid topology.
    pub fn profile_faces(
        &self,
        key: NewellShapeKey,
    ) -> Result<Vec<NewellProfileFace>, NewellKernelError> {
        self.owned_snapshot(key)?.profile_faces()
    }
    /// Measures the actual B-Rep independently of the display mesh.
    ///
    /// # Errors
    /// Propagates missing shape and kernel measurement errors.
    pub fn volume(&self, key: NewellShapeKey) -> Result<f64, NewellKernelError> {
        self.owned_snapshot(key)?.volume()
    }
}
