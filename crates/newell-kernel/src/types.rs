// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Newell-owned geometry identities and operation requests.
use crate::{NewellKernelError, operation_error};
/// Unique construction role, independent of native arena indices and dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NewellFaceProvenance {
    Unknown,
    Base(u32),
    StartCap,
    EndCap,
    Wall { wire: u32, edge: u32 },
}

/// Canonical supporting-plane coordinates, independent of tessellation and edge traversal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NewellProfileFrame {
    pub origin: [f64; 3],
    pub normal: [f64; 3],
    pub u: [f64; 3],
    pub v: [f64; 3],
}
/// One oriented edge occurrence qualified by its planar supporting face and wire.
#[derive(Debug, Clone, PartialEq)]
pub struct NewellProfileEdge {
    pub edge: u32,
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub straight: bool,
}
/// A genuine connected boundary wire on a supporting face.
#[derive(Debug, Clone, PartialEq)]
pub struct NewellProfileWire {
    pub wire: u32,
    pub closed: bool,
    pub edges: Vec<NewellProfileEdge>,
}
/// One real face, in exactly the order of `NewellMesh::face_offsets`.
#[derive(Debug, Clone, PartialEq)]
pub struct NewellProfileFace {
    pub face: u32,
    pub provenance: NewellFaceProvenance,
    pub frame: NewellProfileFrame,
    pub wires: Vec<NewellProfileWire>,
}

/// Set operation on an existing target and a generated tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewellBooleanMode {
    Add,
    Remove,
}

/// Boundary occurrence within an actual planar supporting face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewellBoundary {
    Face,
    Wire(u32),
    Edge { wire: u32, edge: u32 },
}

/// Connected sheet results and the exact target versions they replace.
#[derive(Debug)]
pub struct NewellSurfaceResult {
    pub outputs: Vec<NewellShapeKey>,
    pub changed_targets: Vec<usize>,
}

/// Arena-local Newell identity, never a SolidCore handle or a cross-thread geometry token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NewellShapeKey(u64);
impl NewellShapeKey {
    pub(crate) fn allocate(next: &mut u64) -> Result<Self, NewellKernelError> {
        let incremented = next
            .checked_add(1)
            .ok_or_else(|| operation_error("geometry identity exhausted"))?;
        let key = Self(*next);
        *next = incremented;
        Ok(key)
    }
}

/// Exact changed-target inventory and connected result bodies.
#[derive(Debug)]
pub struct NewellBodyResult {
    pub outputs: Vec<NewellShapeKey>,
    pub changed_targets: Vec<usize>,
}

/// One actual bounded planar face, with clearance measured along the side direction.
#[derive(Debug, Clone, Copy)]
pub struct NewellFaceTarget {
    pub shape: NewellShapeKey,
    pub face: u32,
    pub clearance: f64,
}
#[derive(Debug, Clone, Copy)]
pub enum NewellSurfaceStart {
    Distance(f64),
    UpTo(NewellFaceTarget),
}
#[derive(Debug, Clone)]
pub enum NewellSurfaceExtent {
    Blind(f64),
    UpTo(NewellFaceTarget),
    ThroughAll(Vec<NewellShapeKey>),
}
#[derive(Debug, Clone)]
pub enum NewellSurfaceSides {
    One,
    Symmetric,
    Second {
        extent: NewellSurfaceExtent,
        reverse: bool,
    },
}
/// Owned resolved quantities and references; no UI or model enum enters the ACL.
#[derive(Debug, Clone)]
pub struct NewellSurfaceExtrude {
    pub profile: (NewellShapeKey, u32),
    pub boundary: crate::NewellBoundary,
    pub direction: [f64; 3],
    pub start: NewellSurfaceStart,
    pub extent: NewellSurfaceExtent,
    pub sides: NewellSurfaceSides,
}
