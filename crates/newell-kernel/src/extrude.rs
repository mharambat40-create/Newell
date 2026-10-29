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
use brepkit_topology::{FaceId, SolidId, Topology};
use std::collections::BTreeMap;

/// Arena-local Newell identity, never a BrepKIT handle or a cross-thread geometry token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NewellShapeKey(u64);
/// All topology is created and retained inside the one geometry worker.
#[derive(Debug, Default)]
pub struct NewellGeometry {
    pub(crate) topology: Topology,
    solids: BTreeMap<NewellShapeKey, SolidId>,
    pub(crate) surfaces: BTreeMap<NewellShapeKey, Vec<FaceId>>,
    pub(crate) surface_recipes: BTreeMap<NewellShapeKey, crate::surface::SurfaceSweepRecipe>,
    pub(crate) surface_domains: BTreeMap<NewellShapeKey, crate::surface::SurfaceDomainRecipe>,
    pub(crate) surface_wire_keys: BTreeMap<NewellShapeKey, u32>,
    pub(crate) surface_seams:
        BTreeMap<NewellShapeKey, std::collections::BTreeSet<brepkit_topology::EdgeId>>,
    provenance: BTreeMap<NewellShapeKey, BTreeMap<u32, crate::NewellFaceProvenance>>,
    next: u64,
    cancellation: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
}
impl NewellGeometry {
    /// Construction key of an uncapped connected source-wire sweep, if proven.
    #[must_use]
    pub fn source_wire(&self, key: NewellShapeKey) -> Option<u32> {
        self.surface_wire_keys.get(&key).copied()
    }
    /// Creates an empty worker-local arena, with no native data supplied by the caller.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Installs the worker request's cancellation flag for Newell-owned loops.
    /// Native BrepKIT calls remain indivisible.
    pub fn set_cancellation_flag(&mut self, flag: std::sync::Arc<std::sync::atomic::AtomicBool>) {
        self.cancellation = Some(flag);
    }
    pub(crate) fn check_cancelled(&self) -> Result<(), NewellKernelError> {
        if self
            .cancellation
            .as_ref()
            .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Acquire))
        {
            Err(operation_error("geometry request cancelled"))
        } else {
            Ok(())
        }
    }
    fn reserve_key(&mut self) -> Result<NewellShapeKey, NewellKernelError> {
        let next = self
            .next
            .checked_add(1)
            .ok_or_else(|| NewellKernelError::Operation {
                message: "geometry identity exhausted".into(),
            })?;
        let key = NewellShapeKey(self.next);
        self.next = next;
        Ok(key)
    }
    pub(crate) fn insert(&mut self, solid: SolidId) -> Result<NewellShapeKey, NewellKernelError> {
        let key = self.reserve_key()?;
        self.solids.insert(key, solid);
        Ok(key)
    }
    pub(crate) fn insert_surface(
        &mut self,
        faces: Vec<FaceId>,
    ) -> Result<NewellShapeKey, NewellKernelError> {
        if faces.is_empty() {
            return Err(operation_error("empty surface"));
        }
        let key = self.reserve_key()?;
        self.surfaces.insert(key, faces);
        Ok(key)
    }
    pub(crate) fn shape_faces(
        &self,
        key: NewellShapeKey,
    ) -> Result<Vec<FaceId>, NewellKernelError> {
        if let Some(faces) = self.surfaces.get(&key) {
            return Ok(faces.clone());
        }
        brepkit_topology::explorer::solid_faces(&self.topology, self.solid(key)?)
            .map_err(operation_error)
    }
    pub(crate) fn solid(&self, key: NewellShapeKey) -> Result<SolidId, NewellKernelError> {
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
        let key = self.insert(solid)?;
        let roles = crate::profile::describe_faces(&self.topology, self.shape_faces(key)?)?
            .into_iter()
            .map(|face| (face.face, crate::NewellFaceProvenance::Base(face.face)))
            .collect();
        self.provenance.insert(key, roles);
        Ok(key)
    }
    /// Constructs an exact curved base fixture within the same native arena.
    /// It is excluded from production builds and supplies no UI profile workflow.
    ///
    /// # Errors
    /// Reports invalid fixture coordinates and native construction/transform errors.
    #[cfg(feature = "test-support")]
    pub fn cubic_shape_for_test(
        &mut self,
        control: [[f64; 2]; 4],
        depth: f64,
        transform: [[f64; 4]; 4],
    ) -> Result<NewellShapeKey, NewellKernelError> {
        use brepkit_math::{nurbs::curve::NurbsCurve, vec::Point3};
        use brepkit_topology::{
            edge::{Edge, EdgeCurve},
            face::{Face, FaceSurface},
            wire::{OrientedEdge, Wire},
        };
        if !depth.is_finite() || depth <= 1e-7 || !control.iter().flatten().all(|v| v.is_finite()) {
            return Err(operation_error("invalid cubic fixture"));
        }
        let curve = NurbsCurve::new(
            3,
            vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control
                .iter()
                .map(|p| Point3::new(p[0], p[1], 0.))
                .collect(),
            vec![1.; 4],
        )
        .map_err(operation_error)?;
        let edge =
            brepkit_topology::builder::make_nurbs_edge_from_curve(&mut self.topology, &curve, 1e-7);
        let e = self.topology.edge(edge).map_err(operation_error)?;
        let chord = self
            .topology
            .add_edge(Edge::new(e.end(), e.start(), EdgeCurve::Line));
        let wire = self.topology.add_wire(
            Wire::new(
                vec![
                    OrientedEdge::new(edge, true),
                    OrientedEdge::new(chord, true),
                ],
                true,
            )
            .map_err(operation_error)?,
        );
        let face = self.topology.add_face(Face::new(
            wire,
            vec![],
            FaceSurface::Plane {
                normal: Vec3::new(0., 0., 1.),
                d: 0.,
            },
        ));
        let solid = brepkit_operations::extrude::extrude(
            &mut self.topology,
            face,
            Vec3::new(0., 0., 1.),
            depth,
        )
        .map_err(operation_error)?;
        let key = self.insert(solid)?;
        self.transform_shape_for_test(key, transform)?;
        let roles = crate::profile::describe_faces(&self.topology, self.shape_faces(key)?)?
            .into_iter()
            .map(|f| (f.face, crate::NewellFaceProvenance::Base(f.face)))
            .collect();
        self.provenance.insert(key, roles);
        Ok(key)
    }
    /// Applies a fixture's exact affine map before any model reference is issued.
    ///
    /// # Errors
    /// Rejects nonfinite/singular transforms and native transform failures.
    #[cfg(feature = "test-support")]
    pub fn transform_shape_for_test(
        &mut self,
        key: NewellShapeKey,
        transform: [[f64; 4]; 4],
    ) -> Result<(), NewellKernelError> {
        if !transform.iter().flatten().all(|v| v.is_finite()) {
            return Err(operation_error("invalid fixture transform"));
        }
        let matrix = Mat4(transform);
        matrix.inverse().map_err(operation_error)?;
        let solid = self.solid(key)?;
        brepkit_operations::transform::transform_solid(&mut self.topology, solid, &matrix)
            .map_err(operation_error)
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
        let faces = self.shape_faces(profile)?;
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
    pub(crate) fn validate_native(&self, solid: SolidId) -> Result<(), NewellKernelError> {
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
        if self.surfaces.contains_key(&key) {
            return self.surface_mesh(key, deflection);
        }
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
        let mut faces = crate::profile::describe_faces(&self.topology, self.shape_faces(key)?)?;
        for face in &mut faces {
            if let Some(role) = self
                .provenance
                .get(&key)
                .and_then(|roles| roles.get(&face.face))
            {
                face.provenance = *role;
            }
        }
        Ok(faces)
    }
    /// Assigns only uniquely proven cap/wall roles to a completed extrusion tool.
    ///
    /// # Errors
    /// Rejects absent supports and invalid directions/topology.
    pub fn tag_extrude(
        &mut self,
        key: NewellShapeKey,
        profile: (NewellShapeKey, u32),
        direction: [f64; 3],
    ) -> Result<(), NewellKernelError> {
        use crate::NewellFaceProvenance as Role;
        let d = Vec3::new(direction[0], direction[1], direction[2])
            .normalize()
            .map_err(operation_error)?;
        let source = self
            .profile_faces(profile.0)?
            .into_iter()
            .find(|face| face.face == profile.1)
            .ok_or_else(|| operation_error("missing profile support"))?;
        let faces = crate::profile::describe_faces(&self.topology, self.shape_faces(key)?)?;
        let point = |p: [f64; 3]| Vec3::new(p[0], p[1], p[2]);
        let source_n = point(source.frame.normal);
        let source_origin = point(source.frame.origin);
        let projection = |p: [f64; 3]| {
            let p = point(p);
            p + d * (source_n.dot(source_origin - p) / source_n.dot(d))
        };
        if self.is_surface(key) && !self.surface_wire_keys.contains_key(&key) && !faces.is_empty() {
            let keys: Vec<_> = source
                .wires
                .iter()
                .filter(|wire| {
                    faces
                        .iter()
                        .flat_map(|face| &face.wires)
                        .flat_map(|wire| &wire.edges)
                        .all(|edge| {
                            [edge.start, edge.end].into_iter().all(|p| {
                                let p = projection(p);
                                wire.edges.iter().filter(|edge| edge.straight).any(|edge| {
                                    let a = point(edge.start);
                                    let e = point(edge.end) - a;
                                    let t = (p - a).dot(e) / e.dot(e);
                                    (-1e-7..=1. + 1e-7).contains(&t)
                                        && (p - a - e * t).length() <= 1e-7
                                })
                            })
                        })
                })
                .map(|wire| wire.wire)
                .collect();
            if keys.len() == 1 {
                self.surface_wire_keys.insert(key, keys[0]);
            }
        }
        let mut roles = BTreeMap::new();
        let mut counts = BTreeMap::new();
        for face in faces {
            let n = Vec3::new(
                face.frame.normal[0],
                face.frame.normal[1],
                face.frame.normal[2],
            );
            let alignment = n.dot(d);
            let role = if alignment > 1e-12 {
                Role::EndCap
            } else if alignment < -1e-12 {
                Role::StartCap
            } else {
                let origin = Vec3::new(
                    face.frame.origin[0],
                    face.frame.origin[1],
                    face.frame.origin[2],
                );
                let mut matches = Vec::new();
                for wire in &source.wires {
                    for edge in &wire.edges {
                        if !edge.straight {
                            continue;
                        }
                        let point = |p: [f64; 3]| Vec3::new(p[0], p[1], p[2]);
                        let scale = (point(edge.start) - origin)
                            .length()
                            .max((point(edge.end) - origin).length())
                            .max(origin.length());
                        let eps = 1e-7_f64.max(scale * 1e-10);
                        if n.dot(point(edge.start) - origin).abs() <= eps
                            && n.dot(point(edge.end) - origin).abs() <= eps
                            && face.wires.iter().flat_map(|wire| &wire.edges).all(|wall| {
                                [wall.start, wall.end].into_iter().all(|p| {
                                    let p = projection(p);
                                    let a = point(edge.start);
                                    let e = point(edge.end) - a;
                                    let t = (p - a).dot(e) / e.dot(e);
                                    (-eps..=1. + eps).contains(&t)
                                        && (p - a - e * t).length() <= eps
                                })
                            })
                        {
                            matches.push(Role::Wall {
                                wire: wire.wire,
                                edge: edge.edge,
                            });
                        }
                    }
                }
                if matches.len() == 1 {
                    matches[0]
                } else {
                    Role::Unknown
                }
            };
            *counts.entry(role).or_insert(0usize) += 1;
            roles.insert(face.face, role);
        }
        for role in roles.values_mut() {
            if counts.get(role).copied().unwrap_or(0) != 1 {
                *role = Role::Unknown;
            }
        }
        self.provenance.insert(key, roles);
        Ok(())
    }
    /// Resolves the selected straight boundary occurrence against current native geometry.
    ///
    /// # Errors
    /// Rejects missing/nonstraight occurrences and collapsed directions.
    pub fn direction(
        &self,
        key: NewellShapeKey,
        face: u32,
        wire: u32,
        edge: u32,
    ) -> Result<[f64; 3], NewellKernelError> {
        let face = self
            .profile_faces(key)?
            .into_iter()
            .find(|value| value.face == face)
            .ok_or_else(|| operation_error("direction support"))?;
        let edge = face
            .wires
            .iter()
            .find(|value| value.wire == wire)
            .and_then(|wire| {
                wire.edges
                    .iter()
                    .find(|value| value.edge == edge && value.straight)
            })
            .ok_or_else(|| operation_error("direction edge"))?;
        let d = Vec3::new(
            edge.end[0] - edge.start[0],
            edge.end[1] - edge.start[1],
            edge.end[2] - edge.start[2],
        )
        .normalize()
        .map_err(operation_error)?;
        Ok([d.x(), d.y(), d.z()])
    }
    /// Complete native face count, including curved faces and cavity shells.
    ///
    /// # Errors
    /// Propagates missing shape and invalid topology.
    pub fn face_count(&self, key: NewellShapeKey) -> Result<usize, NewellKernelError> {
        Ok(self.shape_faces(key)?.len())
    }
    /// Measures the actual B-Rep independently of the display mesh.
    ///
    /// # Errors
    /// Propagates missing shape and kernel measurement errors.
    pub fn volume(&self, key: NewellShapeKey) -> Result<f64, NewellKernelError> {
        self.owned_snapshot(key)?.volume()
    }
}

/// Exact changed-target inventory and connected result bodies.
#[derive(Debug)]
pub struct NewellBodyResult {
    pub outputs: Vec<NewellShapeKey>,
    pub changed_targets: Vec<usize>,
}
impl NewellGeometry {
    /// Separates native disconnected outer shells while retaining each cavity with its owner.
    ///
    /// # Errors
    /// Rejects ambiguous cavities, invalid components and topology failures.
    pub fn body_components(
        &mut self,
        key: NewellShapeKey,
    ) -> Result<Vec<NewellShapeKey>, NewellKernelError> {
        use brepkit_topology::{shell::Shell, solid::Solid};
        let solid = self.solid(key)?;
        let value = self.topology.solid(solid).map_err(operation_error)?;
        let voids = value.inner_shells().to_vec();
        let faces = self
            .topology
            .shell(value.outer_shell())
            .map_err(operation_error)?
            .faces()
            .to_vec();
        let mut edges: BTreeMap<_, Vec<usize>> = BTreeMap::new();
        for (index, id) in faces.iter().enumerate() {
            let face = self.topology.face(*id).map_err(operation_error)?;
            for wire in std::iter::once(face.outer_wire()).chain(face.inner_wires().iter().copied())
            {
                for edge in self.topology.wire(wire).map_err(operation_error)?.edges() {
                    edges.entry(edge.edge()).or_default().push(index);
                }
            }
        }
        let mut neighbors = vec![Vec::new(); faces.len()];
        for uses in edges.values() {
            for index in uses {
                neighbors[*index].extend(uses.iter().copied().filter(|other| other != index));
            }
        }
        let mut visited = vec![false; faces.len()];
        let mut groups = Vec::new();
        for first in 0..faces.len() {
            if visited[first] {
                continue;
            }
            let mut group = Vec::new();
            let mut work = vec![first];
            visited[first] = true;
            while let Some(index) = work.pop() {
                group.push(faces[index]);
                for next in &neighbors[index] {
                    if !visited[*next] {
                        visited[*next] = true;
                        work.push(*next);
                    }
                }
            }
            groups.push(group);
        }
        if groups.len() == 1 {
            self.validate_native(solid)?;
            return Ok(vec![key]);
        }
        let mut components = Vec::new();
        for group in groups {
            let shell = self
                .topology
                .add_shell(Shell::new(group).map_err(operation_error)?);
            components.push(self.topology.add_solid(Solid::new(shell, Vec::new())));
        }
        for shell in voids {
            let face = *self
                .topology
                .shell(shell)
                .map_err(operation_error)?
                .faces()
                .first()
                .ok_or_else(|| operation_error("empty cavity"))?;
            let wire = self
                .topology
                .face(face)
                .map_err(operation_error)?
                .outer_wire();
            let edge = self
                .topology
                .wire(wire)
                .map_err(operation_error)?
                .edges()
                .first()
                .ok_or_else(|| operation_error("empty cavity wire"))?
                .edge();
            let vertex = self.topology.edge(edge).map_err(operation_error)?.start();
            let point = self
                .topology
                .vertex(vertex)
                .map_err(operation_error)?
                .point();
            let mut owners = Vec::new();
            for component in &components {
                if brepkit_operations::classify::classify_point(
                    &self.topology,
                    *component,
                    point,
                    0.001,
                    1e-7,
                )
                .map_err(operation_error)?
                    == brepkit_operations::classify::PointClassification::Inside
                {
                    owners.push(*component);
                }
            }
            if owners.len() != 1 {
                return Err(operation_error("ambiguous cavity component"));
            }
            self.topology
                .solid_mut(owners[0])
                .map_err(operation_error)?
                .add_inner_shell(shell);
        }
        let keys = components
            .into_iter()
            .map(|solid| {
                self.validate_native(solid)?;
                self.insert(solid)
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.order_components(keys)
    }
    /// Applies regular solid union/difference and reports only targets actually changed.
    ///
    /// # Errors
    /// Rejects no changed target, invalid/nonmanifold unions and native exact failures.
    pub fn combine_bodies(
        &mut self,
        tool: NewellShapeKey,
        targets: &[NewellShapeKey],
        mode: NewellBooleanMode,
    ) -> Result<NewellBodyResult, NewellKernelError> {
        self.combine_bodies_cancellable(tool, targets, mode, || false)
    }
    /// Performs the same exact operation, checking cancellation between operands.
    /// An individual native Boolean call still runs to completion.
    ///
    /// # Errors
    /// Returns the ordinary operation errors or cancellation without publishing geometry.
    pub fn combine_bodies_cancellable(
        &mut self,
        tool: NewellShapeKey,
        targets: &[NewellShapeKey],
        mode: NewellBooleanMode,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<NewellBodyResult, NewellKernelError> {
        let mut check = || {
            if cancelled() {
                Err(operation_error("geometry request cancelled"))
            } else {
                Ok(())
            }
        };
        check()?;
        let mut changed = Vec::new();
        let mut outputs = Vec::new();
        let mut union = tool;
        match mode {
            NewellBooleanMode::Add => {
                let mut pending: Vec<_> = (0..targets.len()).collect();
                loop {
                    let mut progress = false;
                    let mut remaining = Vec::new();
                    for index in pending {
                        check()?;
                        let merged = self
                            .boolean(union, targets[index], mode)?
                            .ok_or_else(|| operation_error("empty union"))?;
                        let components = self.body_components(merged)?;
                        if components.len() == 1 {
                            changed.push(index);
                            union = components[0];
                            progress = true;
                        } else {
                            remaining.push(index);
                        }
                    }
                    if !progress || remaining.is_empty() {
                        break;
                    }
                    pending = remaining;
                }
                changed.sort_unstable();
            }
            NewellBooleanMode::Remove => {
                for (index, target) in targets.iter().copied().enumerate() {
                    check()?;
                    let old = self.volume(target)?;
                    let result = self.boolean(target, tool, mode)?;
                    let measure = result
                        .map(|key| self.volume(key))
                        .transpose()?
                        .unwrap_or(0.);
                    if measure > old + 1e-9_f64.max(old.abs() * 1e-10) {
                        return Err(operation_error("difference increased the target volume"));
                    }
                    if (old - measure).abs() <= 1e-9_f64.max(old.abs() * 1e-10) {
                        continue;
                    }
                    changed.push(index);
                    if let Some(result) = result {
                        outputs.extend(self.body_components(result)?);
                    }
                }
            }
        }
        if changed.is_empty() {
            return Err(operation_error("NoTargetIntersection"));
        }
        if mode == NewellBooleanMode::Add {
            if changed.len() == 1 {
                let old = self.volume(targets[changed[0]])?;
                let result = self.volume(union)?;
                if result <= old + 1e-9_f64.max(old.abs() * 1e-10) {
                    return Err(operation_error("NoTargetIntersection"));
                }
            }
            outputs.push(union);
        }
        check()?;
        Ok(NewellBodyResult {
            outputs,
            changed_targets: changed,
        })
    }
}
