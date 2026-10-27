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
/// Exact startup geometry recipe. Metadata-only bases do not acquire fabricated geometry.
#[derive(Debug, Clone, PartialEq)]
pub enum BaseGeometryRecipe {
    Box { size: [Length; 3] },
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
/// Face ordinal and eligibility supplied by the exact geometry generation.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileFace {
    pub face: u32,
    pub frame: ProfileFrame,
    pub wires: Vec<ProfileWire>,
}
/// Immutable geometry description used by model preflight and candidate row projection.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityGeometry {
    pub entity: EntityId,
    pub generation: GeometryGeneration,
    pub kind: EntityKind,
    pub faces: Vec<ProfileFace>,
}
impl EntityGeometry {
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
    pub kind: EntityKind,
    pub faces: Vec<ProfileFace>,
}
/// Complete results for one replayed feature, including a valid zero-output Remove.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryFeatureResult {
    pub feature: FeatureId,
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
    source: Box<Document>,
    candidate: Document,
    manifest: CommitManifest,
}
impl PreparedDocument {
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
    let mut result = BTreeSet::new();
    let mut values: Vec<_> = spec
        .starting_offset
        .iter()
        .chain(spec.end_offset.iter())
        .chain(spec.second_end.iter())
        .chain(spec.second_end_offset.iter())
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
        let references = edit
            .validation()
            .into_iter()
            .flat_map(|validation| validation.retained_merge_scope.iter().copied())
            .chain(
                edit.spec()
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
        if let Some(spec) = edit.spec() {
            let reference = spec
                .profile_geometry
                .ok_or(ModelError::InvalidGeometryReference)?;
            self.entity_geometry(reference.support().entity)
                .ok_or(ModelError::InvalidGeometryReference)?
                .resolve_profile(reference, spec.geometry == ExtrudeGeometry::Solid)?;
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
                    Feature::Extrude(ExtrudeFeature {
                        name: "Extrude".into(),
                        outputs: Vec::new(),
                        spec: spec.clone(),
                    }),
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
                extrude.spec = spec.clone();
            }
            DocumentEdit::ParameterDefinitions => {}
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
        let mut output_map = BTreeMap::new();
        for (position, geometry) in result.features.into_iter().enumerate() {
            let (id, feature) = self
                .candidate
                .features
                .get(position)
                .ok_or(ModelError::InvalidGeometryManifest)?;
            if *id != geometry.feature {
                return Err(ModelError::InvalidGeometryManifest);
            }
            let id = *id;
            let expected_kind = match feature {
                Feature::Base { .. } => None,
                Feature::Extrude(extrude) => Some(match extrude.spec.geometry {
                    ExtrudeGeometry::Solid => EntityKind::Body,
                    ExtrudeGeometry::Surface => EntityKind::Surface,
                }),
            };
            if let Feature::Extrude(extrude) = feature {
                if geometry.components.is_empty() && extrude.spec.result != ExtrudeResult::Remove {
                    return Err(ModelError::InvalidGeometryManifest);
                }
                if let Some(reference) = extrude.spec.profile_geometry {
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
            for (component_index, component) in geometry.components.into_iter().enumerate() {
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
                        .filter(|old| old.faces == component.faces && old.kind == component.kind)
                        .map_or(result.generation, |old| old.generation);
                    (entity, generation)
                } else {
                    // Reuse a prior output only when the complete Blind/New recipe and its
                    // resolved quantities/support are unchanged. Other result modes must
                    // receive fresh identities until their replay manifests prove equivalence.
                    let reusable = match (self.source.feature(id), self.candidate.feature(id)) {
                        (Some(Feature::Extrude(old)), Some(Feature::Extrude(new)))
                            if old.spec == new.spec
                                && new.spec.result == ExtrudeResult::New
                                && new.spec.termination == ExtrudeTermination::Blind =>
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
                                        })
                                        .map(|old| (entity, old.generation))
                                })
                        }
                        _ => None,
                    };
                    if let Some(output) = reusable {
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
                        entity,
                        generation,
                        kind: component.kind,
                        faces: component.faces,
                    },
                );
            }
            if let Some((_, Feature::Extrude(extrude))) = self.candidate.features.get_mut(position)
            {
                extrude.outputs = outputs.clone();
            }
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
            source: self.source,
            candidate: self.candidate,
            manifest,
        };
        prepared.source.validate_prepared(&prepared)?;
        Ok(prepared)
    }
}
