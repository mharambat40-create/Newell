// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Model-owned geometric recipes and generation-qualified subshape references.
use crate::{EntityId, EntityKind, Length, ModelError};

/// Identity of one immutable exact geometric result, independent of mesh deflection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GeometryGeneration(pub u64);
/// A bounded face of an exact entity generation, in the kernel's stable face order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FaceRef {
    pub entity: EntityId,
    pub generation: GeometryGeneration,
    pub face: u32,
}
/// Profile boundary occurrences always retain their real planar supporting face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProfileRef {
    Face(FaceRef),
    Wire {
        support: FaceRef,
        wire: u32,
    },
    Edge {
        support: FaceRef,
        wire: u32,
        edge: u32,
    },
}
impl ProfileRef {
    /// Supporting face qualifies both the geometry generation and the sweep plane.
    #[must_use]
    pub const fn support(self) -> FaceRef {
        match self {
            Self::Face(face)
            | Self::Wire { support: face, .. }
            | Self::Edge { support: face, .. } => face,
        }
    }
}
/// A selected straight edge occurrence carrying its real supporting-face generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectionRef {
    pub support: FaceRef,
    pub wire: u32,
    pub edge: u32,
}
/// Only active target conditions belong to a frozen extrusion recipe.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExtrudeTargets {
    pub main: Option<FaceRef>,
    pub start: Option<FaceRef>,
    pub end: Option<FaceRef>,
    pub second: Option<FaceRef>,
    pub nested: Option<FaceRef>,
}
impl ExtrudeTargets {
    pub fn references(self) -> impl Iterator<Item = FaceRef> {
        [self.main, self.start, self.end, self.second, self.nested]
            .into_iter()
            .flatten()
    }
}
/// Exact startup geometry recipe. Metadata-only bases do not acquire fabricated geometry.
#[derive(Debug, Clone, PartialEq)]
pub enum BaseGeometryRecipe {
    Box {
        size: [Length; 3],
    },
    /// Headless regression fixture; never admitted by the production application.
    #[cfg(feature = "test-support")]
    TestBox {
        size: [Length; 3],
        transform: [[f64; 4]; 4],
    },
    /// Exact curved source for worker/replay tests, not an interactive profile producer.
    #[cfg(feature = "test-support")]
    TestCubic {
        control: [[f64; 2]; 4],
        depth: Length,
        transform: [[f64; 4]; 4],
    },
}
/// Kernel-derived supporting plane frame, represented only by owned Newell data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfileFrame {
    pub origin: [f64; 3],
    pub normal: [f64; 3],
    pub u: [f64; 3],
    pub v: [f64; 3],
}
/// Oriented occurrence information; this is not a viewport triangle or global edge identity.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileEdge {
    pub edge: u32,
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub straight: bool,
}
/// A real boundary wire on this supporting face.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileWire {
    pub wire: u32,
    pub closed: bool,
    pub edges: Vec<ProfileEdge>,
}
/// Kernel-certified unique construction role used for private history replay remapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaceProvenance {
    Unknown,
    Base(u32),
    StartCap,
    EndCap,
    Wall { wire: u32, edge: u32 },
}
/// Face ordinal and eligibility supplied by the exact geometry generation.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileFace {
    pub face: u32,
    pub provenance: FaceProvenance,
    pub frame: ProfileFrame,
    pub wires: Vec<ProfileWire>,
}
/// Immutable geometry description used by model preflight and candidate row projection.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityGeometry {
    /// Source-wire construction key when proven by exact sweep/domain construction.
    /// Split Boolean components may share a key; uniqueness is checked before remapping.
    pub source_wire: Option<u32>,
    pub entity: EntityId,
    pub generation: GeometryGeneration,
    pub kind: EntityKind,
    pub faces: Vec<ProfileFace>,
    /// Exact B-Rep volume in mm3 for Body, area in mm2 for Surface.
    pub measure: f64,
    pub face_count: u32,
}
impl EntityGeometry {
    /// Validates a straight direction occurrence against the exact source generation.
    ///
    /// # Errors
    /// Rejects stale/missing/nonlinear directions.
    pub fn resolve_direction(&self, reference: DirectionRef) -> Result<&ProfileEdge, ModelError> {
        self.resolve_profile(
            ProfileRef::Edge {
                support: reference.support,
                wire: reference.wire,
                edge: reference.edge,
            },
            false,
        )?
        .wires
        .iter()
        .find(|wire| wire.wire == reference.wire)
        .and_then(|wire| {
            wire.edges
                .iter()
                .find(|edge| edge.edge == reference.edge && edge.straight)
        })
        .ok_or(ModelError::InvalidGeometryReference)
    }
    /// Resolves only generation-qualified existing boundary occurrences.
    ///
    /// # Errors
    /// Rejects stale generations, absent faces/wires/edges and invalid closed-face profiles.
    pub fn resolve_profile(
        &self,
        reference: ProfileRef,
        solid: bool,
    ) -> Result<&ProfileFace, ModelError> {
        let support = reference.support();
        if support.entity != self.entity || support.generation != self.generation {
            return Err(ModelError::InvalidGeometryReference);
        }
        let face = self
            .faces
            .iter()
            .find(|face| face.face == support.face)
            .ok_or(ModelError::InvalidGeometryReference)?;
        match reference {
            ProfileRef::Face(_) => {
                if face.wires.is_empty()
                    || face
                        .wires
                        .iter()
                        .any(|wire| !wire.closed || wire.edges.is_empty())
                {
                    return Err(ModelError::InvalidGeometryReference);
                }
            }
            ProfileRef::Wire { wire, .. } if !solid => {
                if !face
                    .wires
                    .iter()
                    .any(|value| value.wire == wire && !value.edges.is_empty())
                {
                    return Err(ModelError::InvalidGeometryReference);
                }
            }
            ProfileRef::Edge { wire, edge, .. } if !solid => {
                if !face.wires.iter().any(|value| {
                    value.wire == wire && value.edges.iter().any(|value| value.edge == edge)
                }) {
                    return Err(ModelError::InvalidGeometryReference);
                }
            }
            _ => return Err(ModelError::InvalidGeometryReference),
        }
        Ok(face)
    }
}
impl crate::Document {
    /// Active entity versions immediately before an operation, or at the current history tip.
    ///
    /// # Errors
    /// Rejects an unknown boundary and invalid consumed-version history.
    pub fn active_entities_at(
        &self,
        before: Option<crate::FeatureId>,
    ) -> Result<std::collections::BTreeSet<EntityId>, ModelError> {
        let mut active = std::collections::BTreeSet::new();
        let mut found = before.is_none();
        for (id, feature) in self.features() {
            if Some(*id) == before {
                found = true;
                break;
            }
            if let crate::Feature::Extrude(extrude) = feature {
                for consumed in extrude.consumed_inputs() {
                    if !active.remove(consumed) {
                        return Err(ModelError::InvalidGeometryManifest);
                    }
                }
            }
            active.extend(feature.outputs().iter().copied());
        }
        if !found {
            return Err(ModelError::InvalidGeometryReference);
        }
        Ok(active)
    }
    /// Last finalized complete geometry generation; zero means no geometry is installed yet.
    #[must_use]
    pub const fn geometry_generation(&self) -> GeometryGeneration {
        self.geometry_generation
    }
    /// Exact base recipe, if this entity was created as a real model seed.
    #[must_use]
    pub fn base_geometry_recipe(&self, entity: EntityId) -> Option<&BaseGeometryRecipe> {
        self.base_geometry.get(&entity)
    }
    /// Last accepted exact generation manifest for an entity.
    #[must_use]
    pub fn entity_geometry(&self, entity: EntityId) -> Option<&EntityGeometry> {
        self.entity_geometry.get(&entity)
    }
}

use crate::{
    Document, ExtrudeFeature, ExtrudeGeometry, ExtrudeResult, ExtrudeSpec, ExtrudeTermination,
    Feature, FeatureId, HistoryLayoutRow, HistoryNodeId, NewEntity, ParameterValue,
    parameters::{ParameterDraft, ParameterDraftOwner},
};
use std::collections::{BTreeMap, BTreeSet};
/// Retained field-27 references are validation inputs even when inactive in the geometry recipe.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtrudeValidationRefs {
    pub retained_merge_scope: Vec<EntityId>,
}
/// The only supported document preparation requests, frozen before geometry work.
#[derive(Debug, Clone, PartialEq)]
pub enum DocumentEdit {
    ExtrudeInsert {
        spec: ExtrudeSpec,
        validation: ExtrudeValidationRefs,
    },
    ExtrudeReplace {
        feature: FeatureId,
        spec: ExtrudeSpec,
        validation: ExtrudeValidationRefs,
    },
    ParameterDefinitions,
}
impl DocumentEdit {
    fn spec(&self) -> Option<&ExtrudeSpec> {
        match self {
            Self::ExtrudeInsert { spec, .. } | Self::ExtrudeReplace { spec, .. } => Some(spec),
            Self::ParameterDefinitions => None,
        }
    }
    fn validation(&self) -> Option<&ExtrudeValidationRefs> {
        match self {
            Self::ExtrudeInsert { validation, .. } | Self::ExtrudeReplace { validation, .. } => {
                Some(validation)
            }
            Self::ParameterDefinitions => None,
        }
    }
}
/// One exact connected output, measured and described by the geometry worker.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryComponent {
    pub source_wire: Option<u32>,
    pub kind: EntityKind,
    pub faces: Vec<ProfileFace>,
    pub measure: f64,
    pub face_count: u32,
}
/// Output identity during private replay, before persistent entity allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GeometryOutputRef {
    pub feature: FeatureId,
    pub component: u32,
}
/// Complete results for one replayed feature, including a valid zero-output Remove.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryFeatureResult {
    pub feature: FeatureId,
    pub consumed: Vec<GeometryOutputRef>,
    pub components: Vec<GeometryComponent>,
}
/// Owned kernel-to-model result. Finalization checks this against the frozen request.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryResultManifest {
    pub edit: DocumentEdit,
    pub generation: GeometryGeneration,
    pub features: Vec<GeometryFeatureResult>,
}
/// Immutable authority sealed together with the candidate document after geometry succeeds.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitManifest {
    edit: DocumentEdit,
    insertion: Option<FeatureId>,
    feature: Option<FeatureId>,
    generation: GeometryGeneration,
    outputs: BTreeMap<FeatureId, Vec<EntityId>>,
    parameter_epoch: u64,
}
impl CommitManifest {
    /// Frozen request, including its validation-only payload.
    #[must_use]
    pub fn edit(&self) -> &DocumentEdit {
        &self.edit
    }
    /// Inserted/replaced feature identity, if this was a feature edit.
    #[must_use]
    pub const fn feature(&self) -> Option<FeatureId> {
        self.feature
    }
    /// Accepted output mapping, in deterministic worker component order.
    #[must_use]
    pub fn outputs(&self) -> &BTreeMap<FeatureId, Vec<EntityId>> {
        &self.outputs
    }
    /// Exact staged generation associated with the worker token.
    #[must_use]
    pub const fn generation(&self) -> GeometryGeneration {
        self.generation
    }
}
/// Owned outcome recorded after one successful app document acceptance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitReceipt {
    pub feature: Option<FeatureId>,
    pub outputs: Vec<EntityId>,
    pub generation: GeometryGeneration,
}
/// Semantic scratch state with a private owner reservation; it cannot be accepted as a document.
#[derive(Debug, Clone)]
pub struct DocumentCandidate {
    source: Box<Document>,
    candidate: Document,
    edit: DocumentEdit,
    insertion: Option<FeatureId>,
    feature: Option<FeatureId>,
}
/// Finalized authority. There is no public constructor or plain-document conversion.
#[derive(Debug, Clone)]
pub struct PreparedDocument {
    authority: PreparedAuthority,
    source: Box<Document>,
    candidate: Document,
    manifest: CommitManifest,
}
/// Opaque identity of one finalized result, retained by its native geometry token.
/// Independent finalizations never share authority, even at the same generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedAuthority(u64);
static NEXT_PREPARED_AUTHORITY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
impl PreparedAuthority {
    fn reserve() -> Result<Self, ModelError> {
        use std::sync::atomic::Ordering::Relaxed;
        let mut current = NEXT_PREPARED_AUTHORITY.load(Relaxed);
        loop {
            let next = current
                .checked_add(1)
                .ok_or(ModelError::IdentityExhausted)?;
            match NEXT_PREPARED_AUTHORITY.compare_exchange_weak(current, next, Relaxed, Relaxed) {
                Ok(_) => return Ok(Self(current)),
                Err(observed) => current = observed,
            }
        }
    }
}
impl PreparedDocument {
    /// Qualifies the exact finalized result without exposing a constructor.
    #[must_use]
    pub const fn authority(&self) -> PreparedAuthority {
        self.authority
    }
    /// Read-only finalized recipes and owned geometry descriptions.
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.candidate
    }
    /// Read-only sealed commit information for binding staged scene items.
    #[must_use]
    pub fn manifest(&self) -> &CommitManifest {
        &self.manifest
    }
    /// Consumes finalized authority only after validating it against the still-accepted source.
    ///
    /// # Errors
    /// Rejects stale sources and changed or invalid sealed validation inputs.
    pub fn accept(self, source: &Document) -> Result<Document, ModelError> {
        source.validate_prepared(&self)?;
        Ok(self.candidate)
    }
}
fn bindings(spec: &ExtrudeSpec) -> BTreeSet<crate::formula::ParameterId> {
    let spec = spec.inputs();
    let mut result = BTreeSet::new();
    let mut values: Vec<_> = spec
        .starting_offset
        .iter()
        .chain(spec.end_offset.iter())
        .chain(spec.second_end.iter())
        .chain(spec.second_end_offset.iter())
        .chain(spec.main_clearance.iter())
        .collect();
    if spec.termination == ExtrudeTermination::Blind {
        values.push(&spec.depth);
    }
    for value in values {
        if let ParameterValue::Bound(id) = value {
            result.insert(*id);
        }
    }
    result
}
impl Document {
    fn validate_retained(&self, edit: &DocumentEdit, boundary: usize) -> Result<(), ModelError> {
        let spec = edit.spec().map(ExtrudeSpec::inputs);
        let references = edit
            .validation()
            .into_iter()
            .flat_map(|validation| validation.retained_merge_scope.iter().copied())
            .chain(
                spec.as_ref()
                    .into_iter()
                    .flat_map(|spec| spec.merge_scope.iter().copied()),
            )
            .chain(
                spec.as_ref()
                    .into_iter()
                    .flat_map(|spec| spec.targets.references())
                    .map(|reference| reference.entity),
            )
            .chain(
                spec.as_ref()
                    .and_then(|spec| spec.along_geometry)
                    .map(|reference| reference.support.entity),
            )
            .chain(
                spec.as_ref()
                    .and_then(|spec| spec.profile_geometry)
                    .map(|reference| reference.support().entity),
            );
        for entity in references {
            if self.entity(entity).is_none() {
                return Err(ModelError::InvalidGeometryReference);
            }
            let producers: Vec<_> = self
                .features
                .iter()
                .enumerate()
                .filter(|(_, (_, feature))| feature.outputs().contains(&entity))
                .collect();
            if producers.len() != 1 || producers[0].0 >= boundary {
                return Err(ModelError::InvalidGeometryReference);
            }
        }
        if let Some(spec) = spec.as_ref() {
            if spec.result != ExtrudeResult::New && !spec.merge_with_all {
                let before = self.features().get(boundary).map(|(feature, _)| *feature);
                let active = self.active_entities_at(before)?;
                let kind = if spec.geometry == ExtrudeGeometry::Solid {
                    EntityKind::Body
                } else {
                    EntityKind::Surface
                };
                for target in &spec.merge_scope {
                    if !active.contains(target)
                        || self
                            .entity(*target)
                            .is_none_or(|entity| entity.kind() != kind)
                    {
                        return Err(ModelError::InvalidGeometryReference);
                    }
                }
            }
            for target in spec.targets.references() {
                self.entity_geometry(target.entity)
                    .ok_or(ModelError::InvalidGeometryReference)?
                    .resolve_profile(ProfileRef::Face(target), true)?;
            }
            let reference = spec
                .profile_geometry
                .ok_or(ModelError::InvalidGeometryReference)?;
            self.entity_geometry(reference.support().entity)
                .ok_or(ModelError::InvalidGeometryReference)?
                .resolve_profile(reference, spec.geometry == ExtrudeGeometry::Solid)?;
            if let Some(direction) = spec.along_geometry {
                self.entity_geometry(direction.support.entity)
                    .ok_or(ModelError::InvalidGeometryReference)?
                    .resolve_direction(direction)?;
            }
            if spec
                .profile
                .is_some_and(|id| id != reference.support().entity)
            {
                return Err(ModelError::InvalidGeometryReference);
            }
        }
        Ok(())
    }
    /// Validates a frozen edit on an isolated document before any geometric work.
    ///
    /// # Errors
    /// Rejects stale reservations, references, ownership, quantities and identity exhaustion.
    pub fn prepare_edit(
        &self,
        edit: &DocumentEdit,
        parameters: &ParameterDraft,
        insertion: Option<FeatureId>,
    ) -> Result<DocumentCandidate, ModelError> {
        parameters.validate_source(self)?;
        let (feature, boundary) = match edit {
            DocumentEdit::ExtrudeInsert { .. } => {
                if parameters.owner() != (ParameterDraftOwner::InsertExtrude { before: insertion })
                {
                    return Err(ModelError::StaleParameterDraft);
                }
                self.next_feature
                    .checked_add(1)
                    .ok_or(ModelError::IdentityExhausted)?;
                let feature = FeatureId(self.next_feature);
                if parameters.feature_owner() != Some(feature) {
                    return Err(ModelError::StaleParameterDraft);
                }
                (Some(feature), self.insertion_index(insertion)?)
            }
            DocumentEdit::ExtrudeReplace { feature, .. } => {
                if parameters.owner() != ParameterDraftOwner::EditExtrude(*feature)
                    || !matches!(self.feature(*feature), Some(Feature::Extrude(_)))
                {
                    return Err(ModelError::StaleParameterDraft);
                }
                (Some(*feature), self.insertion_index(Some(*feature))?)
            }
            DocumentEdit::ParameterDefinitions => {
                if parameters.owner() != ParameterDraftOwner::Document {
                    return Err(ModelError::StaleParameterDraft);
                }
                (None, self.features.len())
            }
        };
        self.validate_retained(edit, boundary)?;
        let mut candidate = self.clone();
        let active = edit.spec().map_or_else(BTreeSet::new, bindings);
        candidate.parameter_registry = parameters.prepared_registry(&active)?;
        if let Some(spec) = edit.spec() {
            crate::validate_extrude(spec, candidate.parameters())?;
        }
        match edit {
            DocumentEdit::ExtrudeInsert { spec, .. } => {
                let layout_index = insertion
                    .and_then(|id| {
                        candidate
                            .layout
                            .iter()
                            .position(|row| row.node == HistoryNodeId::Operation(id))
                    })
                    .unwrap_or(candidate.layout.len());
                let depth = candidate
                    .layout
                    .get(layout_index)
                    .map_or(0, |row| row.depth);
                let id = candidate.insert_feature(
                    boundary,
                    Feature::Extrude(Box::new(ExtrudeFeature {
                        consumed: Vec::new(),
                        name: "Extrude".into(),
                        outputs: Vec::new(),
                        spec: spec.active_recipe(),
                    })),
                );
                if Some(id) != feature {
                    return Err(ModelError::StaleParameterDraft);
                }
                candidate.layout.insert(
                    layout_index,
                    HistoryLayoutRow {
                        node: HistoryNodeId::Operation(id),
                        depth,
                    },
                );
            }
            DocumentEdit::ExtrudeReplace { spec, .. } => {
                let (_, value) = candidate
                    .features
                    .get_mut(boundary)
                    .ok_or(ModelError::InvalidGeometryManifest)?;
                let Feature::Extrude(extrude) = value else {
                    return Err(ModelError::InvalidGeometryManifest);
                };
                extrude.spec = spec.active_recipe();
            }
            DocumentEdit::ParameterDefinitions => {}
        }
        let order = candidate
            .features
            .iter()
            .enumerate()
            .map(|(position, (feature, _))| (*feature, position))
            .collect();
        for (position, (feature, value)) in candidate.features.iter().enumerate() {
            if let Feature::Extrude(extrude) = value {
                crate::validate_extrude(&extrude.spec, candidate.parameters())?;
                candidate.parameters().validate_feature_inputs(
                    *feature,
                    position,
                    &bindings(&extrude.spec),
                    &order,
                )?;
            }
        }
        Ok(DocumentCandidate {
            source: Box::new(self.clone()),
            candidate,
            edit: edit.clone(),
            insertion,
            feature,
        })
    }
    /// Rechecks sealed authority against the exact still-accepted source; no geometry runs here.
    ///
    /// # Errors
    /// Rejects stale sources, generation mismatches and missing/forward retained references.
    pub fn validate_prepared(&self, prepared: &PreparedDocument) -> Result<(), ModelError> {
        if prepared.source.as_ref() != self
            || prepared.manifest.generation.0 == 0
            || prepared.manifest.parameter_epoch != prepared.candidate.parameters().epoch()
        {
            return Err(ModelError::InvalidGeometryManifest);
        }
        let boundary = match &prepared.manifest.edit {
            DocumentEdit::ExtrudeReplace { feature, .. } => self.insertion_index(Some(*feature))?,
            _ => self.insertion_index(prepared.manifest.insertion)?,
        };
        self.validate_retained(&prepared.manifest.edit, boundary)?;
        Ok(())
    }
}
fn remap_face(
    reference: FaceRef,
    source: &Document,
    candidate: &Document,
) -> Result<FaceRef, ModelError> {
    let old = source
        .entity_geometry(reference.entity)
        .ok_or(ModelError::InvalidGeometryReference)?;
    let new = candidate
        .entity_geometry(reference.entity)
        .ok_or(ModelError::InvalidGeometryReference)?;
    if reference.generation != old.generation {
        return Err(ModelError::InvalidGeometryReference);
    }
    if new.generation == reference.generation {
        return Ok(reference);
    }
    let face = old
        .faces
        .iter()
        .find(|face| face.face == reference.face)
        .ok_or(ModelError::InvalidGeometryReference)?;
    if face.provenance == FaceProvenance::Unknown {
        return Err(ModelError::InvalidGeometryReference);
    }
    let matches: Vec<_> = new
        .faces
        .iter()
        .filter(|new| new.provenance == face.provenance)
        .collect();
    if matches.len() != 1 {
        return Err(ModelError::InvalidGeometryReference);
    }
    Ok(FaceRef {
        entity: reference.entity,
        generation: new.generation,
        face: matches[0].face,
    })
}
/// Cap boundaries copy the input face's ordered wire/edge construction keys.
/// Follow that construction chain to the Box seed; coordinate changes do not
/// erase a proven key, and unrelated/boolean/ambiguous constructions never match.
fn boundary_construction_survives(
    mut old: FaceRef,
    mut new: FaceRef,
    source: &Document,
    candidate: &Document,
) -> bool {
    for _ in 0..=source.features().len() {
        let Some(old_face) = source
            .entity_geometry(old.entity)
            .and_then(|geometry| geometry.faces.iter().find(|face| face.face == old.face))
        else {
            return false;
        };
        let Some(new_face) = candidate
            .entity_geometry(new.entity)
            .and_then(|geometry| geometry.faces.iter().find(|face| face.face == new.face))
        else {
            return false;
        };
        if old_face.wires == new_face.wires {
            return true;
        }
        if old_face.provenance != new_face.provenance
            || old_face.wires.len() != new_face.wires.len()
            || old_face.wires.iter().zip(&new_face.wires).any(|(a, b)| {
                a.wire != b.wire
                    || a.closed != b.closed
                    || a.edges.len() != b.edges.len()
                    || a.edges
                        .iter()
                        .zip(&b.edges)
                        .any(|(a, b)| a.edge != b.edge || a.straight != b.straight)
            })
        {
            return false;
        }
        match old_face.provenance {
            FaceProvenance::Base(_) => {
                return old.entity == new.entity
                    && matches!(
                        (
                            source.base_geometry_recipe(old.entity),
                            candidate.base_geometry_recipe(new.entity)
                        ),
                        (
                            Some(BaseGeometryRecipe::Box { .. }),
                            Some(BaseGeometryRecipe::Box { .. })
                        )
                    );
            }
            FaceProvenance::StartCap | FaceProvenance::EndCap => {}
            _ => return false,
        }
        let Some((feature, Feature::Extrude(previous))) = source
            .features()
            .iter()
            .find(|(_, feature)| feature.outputs().contains(&old.entity))
        else {
            return false;
        };
        let Some(Feature::Extrude(current)) = candidate.feature(*feature) else {
            return false;
        };
        if previous.spec.result != ExtrudeResult::New || current.spec.result != ExtrudeResult::New {
            return false;
        }
        let (Some(ProfileRef::Face(a)), Some(ProfileRef::Face(b))) =
            (previous.spec.profile, current.spec.profile)
        else {
            return false;
        };
        if !remap_face(a, source, candidate).is_ok_and(|reference| reference == b) {
            return false;
        }
        old = a;
        new = b;
    }
    false
}

fn remap_spec(
    spec: &mut ExtrudeSpec,
    source: &Document,
    candidate: &Document,
) -> Result<(), ModelError> {
    let mut inputs = spec.inputs();
    if let Some(profile) = inputs.profile_geometry {
        let support = profile.support();
        let remapped = remap_face(support, source, candidate)?;
        if support != remapped && !matches!(profile, ProfileRef::Face(_)) {
            let old = source
                .entity_geometry(support.entity)
                .and_then(|entity| entity.faces.iter().find(|face| face.face == support.face))
                .ok_or(ModelError::InvalidGeometryReference)?;
            let new = candidate
                .entity_geometry(remapped.entity)
                .and_then(|entity| entity.faces.iter().find(|face| face.face == remapped.face))
                .ok_or(ModelError::InvalidGeometryReference)?;
            if old.wires != new.wires
                && !boundary_construction_survives(support, remapped, source, candidate)
            {
                return Err(ModelError::InvalidGeometryReference);
            }
        }
        inputs.profile_geometry = Some(match profile {
            ProfileRef::Face(_) => ProfileRef::Face(remapped),
            ProfileRef::Wire { wire, .. } => ProfileRef::Wire {
                support: remapped,
                wire,
            },
            ProfileRef::Edge { wire, edge, .. } => ProfileRef::Edge {
                support: remapped,
                wire,
                edge,
            },
        });
    }
    for reference in [
        &mut inputs.targets.main,
        &mut inputs.targets.start,
        &mut inputs.targets.end,
        &mut inputs.targets.second,
        &mut inputs.targets.nested,
    ]
    .into_iter()
    .flatten()
    {
        *reference = remap_face(*reference, source, candidate)?;
    }
    if let Some(reference) = &mut inputs.along_geometry {
        let remapped = remap_face(reference.support, source, candidate)?;
        if reference.support != remapped {
            let old = source
                .entity_geometry(reference.support.entity)
                .ok_or(ModelError::InvalidGeometryReference)?
                .resolve_direction(*reference)?;
            let new = candidate
                .entity_geometry(remapped.entity)
                .ok_or(ModelError::InvalidGeometryReference)?
                .resolve_direction(DirectionRef {
                    support: remapped,
                    wire: reference.wire,
                    edge: reference.edge,
                })?;
            if old != new
                && !boundary_construction_survives(reference.support, remapped, source, candidate)
            {
                return Err(ModelError::InvalidGeometryReference);
            }
        }
        reference.support = remapped;
    }
    *spec = inputs.into_spec()?;
    Ok(())
}
fn same_profile_construction(
    old: Option<ProfileRef>,
    new: Option<ProfileRef>,
    source: &Document,
    candidate: &Document,
) -> bool {
    let (Some(old), Some(new)) = (old, new) else {
        return false;
    };
    if old == new {
        return true;
    }
    let same_boundary = match (old, new) {
        (ProfileRef::Face(_), ProfileRef::Face(_)) => true,
        (ProfileRef::Wire { wire: a, .. }, ProfileRef::Wire { wire: b, .. }) => {
            a == b
                && boundary_construction_survives(old.support(), new.support(), source, candidate)
        }
        (
            ProfileRef::Edge {
                wire: a, edge: x, ..
            },
            ProfileRef::Edge {
                wire: b, edge: y, ..
            },
        ) => {
            a == b
                && x == y
                && boundary_construction_survives(old.support(), new.support(), source, candidate)
        }
        _ => false,
    };
    same_boundary
        && remap_face(old.support(), source, candidate)
            .is_ok_and(|reference| reference == new.support())
}
impl DocumentCandidate {
    /// Read-only validated recipes for deterministic worker replay.
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.candidate
    }
    /// Reserved operation identity used during replay.
    #[must_use]
    pub const fn feature(&self) -> Option<FeatureId> {
        self.feature
    }
    /// Seals actual geometry outputs and their generation together with the semantic candidate.
    ///
    /// # Errors
    /// Rejects missing/extra/reordered outputs, changed requests, wrong kinds or exhausted ids.
    pub fn finalize(
        mut self,
        result: GeometryResultManifest,
    ) -> Result<PreparedDocument, ModelError> {
        if result.edit != self.edit
            || result.generation.0 == 0
            || result.features.len() != self.candidate.features.len()
        {
            return Err(ModelError::InvalidGeometryManifest);
        }
        let mut output_map: BTreeMap<FeatureId, Vec<EntityId>> = BTreeMap::new();
        let mut active = BTreeSet::new();
        for (position, geometry) in result.features.into_iter().enumerate() {
            let (id, mut value) = self
                .candidate
                .features
                .get(position)
                .cloned()
                .ok_or(ModelError::InvalidGeometryManifest)?;
            if id != geometry.feature {
                return Err(ModelError::InvalidGeometryManifest);
            }
            if let Feature::Extrude(extrude) = &mut value {
                remap_spec(&mut extrude.spec, &self.source, &self.candidate)?;
            }
            self.candidate.features[position].1 = value.clone();
            let feature = &value;
            let mut consumed = Vec::new();
            for reference in &geometry.consumed {
                let entity = *output_map
                    .get(&reference.feature)
                    .and_then(|outputs| outputs.get(usize::try_from(reference.component).ok()?))
                    .ok_or(ModelError::InvalidGeometryManifest)?;
                if !active.remove(&entity) {
                    return Err(ModelError::InvalidGeometryManifest);
                }
                consumed.push(entity);
            }
            let expected_kind = match feature {
                Feature::Base { .. } => None,
                Feature::Extrude(extrude) => Some(match extrude.spec.geometry {
                    ExtrudeGeometry::Solid => EntityKind::Body,
                    ExtrudeGeometry::Surface => EntityKind::Surface,
                }),
            };
            if let Feature::Extrude(extrude) = feature {
                let inputs = extrude.spec.inputs();
                if (extrude.spec.result == ExtrudeResult::New) != consumed.is_empty() {
                    return Err(ModelError::InvalidGeometryManifest);
                }
                let kind = if extrude.spec.geometry == ExtrudeGeometry::Solid {
                    EntityKind::Body
                } else {
                    EntityKind::Surface
                };
                for entity in &consumed {
                    if self
                        .candidate
                        .entity(*entity)
                        .is_none_or(|entity| entity.kind() != kind)
                        || (!inputs.merge_with_all && !inputs.merge_scope.contains(entity))
                    {
                        return Err(ModelError::InvalidGeometryManifest);
                    }
                }
                for target in inputs
                    .targets
                    .references()
                    .chain(inputs.along_geometry.map(|reference| reference.support))
                {
                    if !self.candidate.features[..position]
                        .iter()
                        .any(|(_, feature)| feature.outputs().contains(&target.entity))
                    {
                        return Err(ModelError::InvalidGeometryReference);
                    }
                    self.candidate
                        .entity_geometry(target.entity)
                        .ok_or(ModelError::InvalidGeometryReference)?
                        .resolve_profile(ProfileRef::Face(target), true)?;
                }
                if extrude.spec.result == ExtrudeResult::New
                    && extrude.spec.geometry == ExtrudeGeometry::Surface
                {
                    let profile = extrude
                        .spec
                        .profile
                        .ok_or(ModelError::InvalidGeometryReference)?;
                    let support = profile.support();
                    let face = self
                        .candidate
                        .entity_geometry(support.entity)
                        .and_then(|geometry| {
                            geometry.faces.iter().find(|face| face.face == support.face)
                        })
                        .ok_or(ModelError::InvalidGeometryReference)?;
                    let expected: BTreeSet<_> = match profile {
                        ProfileRef::Face(_) => face.wires.iter().map(|wire| wire.wire).collect(),
                        ProfileRef::Wire { wire, .. } | ProfileRef::Edge { wire, .. } => {
                            BTreeSet::from([wire])
                        }
                    };
                    let actual: BTreeSet<_> = geometry
                        .components
                        .iter()
                        .filter_map(|component| component.source_wire)
                        .collect();
                    if actual != expected || actual.len() != geometry.components.len() {
                        return Err(ModelError::InvalidGeometryManifest);
                    }
                }
                if geometry.components.is_empty() && extrude.spec.result != ExtrudeResult::Remove {
                    return Err(ModelError::InvalidGeometryManifest);
                }
                if let Some(reference) = inputs.profile_geometry {
                    let entity = reference.support().entity;
                    let producer = self.candidate.features[..position]
                        .iter()
                        .find(|(_, feature)| feature.outputs().contains(&entity));
                    if producer.is_none() {
                        return Err(ModelError::InvalidGeometryReference);
                    }
                    self.candidate
                        .entity_geometry(entity)
                        .ok_or(ModelError::InvalidGeometryReference)?
                        .resolve_profile(
                            reference,
                            extrude.spec.geometry == ExtrudeGeometry::Solid,
                        )?;
                }
            }
            let mut outputs = match feature {
                Feature::Base { outputs, .. } => outputs.clone(),
                Feature::Extrude(_) => Vec::new(),
            };
            let base_outputs: Vec<_> = outputs
                .iter()
                .copied()
                .filter(|entity| self.candidate.base_geometry.contains_key(entity))
                .collect();
            if expected_kind.is_none() && geometry.components.len() != base_outputs.len() {
                return Err(ModelError::InvalidGeometryManifest);
            }
            let component_count = geometry.components.len();
            for (component_index, component) in geometry.components.into_iter().enumerate() {
                if !component.measure.is_finite()
                    || component.measure <= 0.
                    || component.face_count == 0
                    || (component.kind == EntityKind::Body
                        && (component.face_count < 3 || component.source_wire.is_some()))
                {
                    return Err(ModelError::InvalidGeometryManifest);
                }
                if expected_kind.is_some_and(|kind| kind != component.kind)
                    || !matches!(component.kind, EntityKind::Body | EntityKind::Surface)
                {
                    return Err(ModelError::InvalidGeometryManifest);
                }
                let (entity, generation) = if let Some(entity) =
                    base_outputs.get(component_index).copied()
                {
                    let generation = self
                        .source
                        .entity_geometry(entity)
                        .filter(|old| {
                            old.faces == component.faces
                                && old.kind == component.kind
                                && old.measure == component.measure
                                && old.face_count == component.face_count
                        })
                        .map_or(result.generation, |old| old.generation);
                    (entity, generation)
                } else {
                    let constructed = match (self.source.feature(id), self.candidate.feature(id)) {
                        (Some(Feature::Extrude(old)), Some(Feature::Extrude(new)))
                            if old.spec.result == ExtrudeResult::New
                                && new.spec.result == ExtrudeResult::New
                                && old.spec.geometry == new.spec.geometry
                                && old.outputs.len() == component_count
                                && same_profile_construction(
                                    old.spec.profile,
                                    new.spec.profile,
                                    &self.source,
                                    &self.candidate,
                                ) =>
                        {
                            let retained = if component_count == 1 {
                                old.outputs.first().copied()
                            } else if let Some(wire) = component.source_wire {
                                let matching: Vec<_> = old
                                    .outputs
                                    .iter()
                                    .copied()
                                    .filter(|entity| {
                                        self.source.entity_geometry(*entity).is_some_and(
                                            |geometry| geometry.source_wire == Some(wire),
                                        )
                                    })
                                    .collect();
                                (matching.len() == 1).then(|| matching[0])
                            } else {
                                None
                            };
                            retained.and_then(|entity| {
                                self.source.entity_geometry(entity).map(|old| {
                                    let unchanged = old.faces == component.faces
                                        && old.kind == component.kind
                                        && old.measure == component.measure
                                        && old.face_count == component.face_count;
                                    (
                                        entity,
                                        if unchanged {
                                            old.generation
                                        } else {
                                            result.generation
                                        },
                                    )
                                })
                            })
                        }
                        _ => None,
                    };
                    // Keep unaffected result identities only after the complete recipe,
                    // resolved inputs, consumed versions and actual geometry agree.
                    let reusable = match (self.source.feature(id), self.candidate.feature(id)) {
                        (Some(Feature::Extrude(old)), Some(Feature::Extrude(new)))
                            if old.spec == new.spec
                                && old.consumed_inputs() == consumed.as_slice() =>
                        {
                            let same = bindings(&new.spec).iter().all(|parameter| {
                                self.source
                                    .parameters()
                                    .definition(*parameter)
                                    .map(|p| p.value)
                                    == self
                                        .candidate
                                        .parameters()
                                        .definition(*parameter)
                                        .map(|p| p.value)
                            });
                            same.then(|| old.outputs.get(component_index).copied())
                                .flatten()
                                .and_then(|entity| {
                                    self.source
                                        .entity_geometry(entity)
                                        .filter(|old| {
                                            old.faces == component.faces
                                                && old.kind == component.kind
                                                && old.measure == component.measure
                                                && old.face_count == component.face_count
                                        })
                                        .map(|old| (entity, old.generation))
                                })
                        }
                        _ => None,
                    };
                    if let Some(output) = constructed.or(reusable) {
                        output
                    } else {
                        self.candidate
                            .next_entity
                            .checked_add(1)
                            .ok_or(ModelError::IdentityExhausted)?;
                        let entity = self.candidate.insert_outputs(vec![NewEntity {
                            name: "Extrude".into(),
                            kind: component.kind,
                            visible: true,
                        }])[0];
                        (entity, result.generation)
                    }
                };
                if expected_kind.is_some() {
                    outputs.push(entity);
                }
                self.candidate.entity_geometry.insert(
                    entity,
                    EntityGeometry {
                        source_wire: component.source_wire,
                        entity,
                        generation,
                        kind: component.kind,
                        faces: component.faces,
                        measure: component.measure,
                        face_count: component.face_count,
                    },
                );
            }
            if let Some((_, Feature::Extrude(extrude))) = self.candidate.features.get_mut(position)
            {
                extrude.outputs = outputs.clone();
                extrude.consumed = consumed;
            } else if !consumed.is_empty() {
                return Err(ModelError::InvalidGeometryManifest);
            }
            active.extend(outputs.iter().copied());
            output_map.insert(id, outputs);
        }
        self.candidate.geometry_generation = result.generation;
        let manifest = CommitManifest {
            edit: self.edit,
            insertion: self.insertion,
            feature: self.feature,
            generation: result.generation,
            outputs: output_map,
            parameter_epoch: self.candidate.parameters().epoch(),
        };
        let prepared = PreparedDocument {
            authority: PreparedAuthority::reserve()?,
            source: self.source,
            candidate: self.candidate,
            manifest,
        };
        prepared.source.validate_prepared(&prepared)?;
        Ok(prepared)
    }
}
