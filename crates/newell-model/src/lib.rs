// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Parametric document and command transactions (M4).
//!
//! This crate owns CAD semantics only. It deliberately has no dependency on the ASCII UI,
//! renderer, or geometric-kernel implementation. The application adapts command values to the
//! UI and asks `newell-kernel` to build the preview/committed shape.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

/// Stable identifier of a parametric feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeatureId(u64);
impl FeatureId {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Stable identifier of an entity resulting from a feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(u64);
impl EntityId {
    /// Reconstructs a persisted identifier. Callers cannot allocate document ids with it.
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A length stored in millimetres, the document's canonical engineering unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Length(f64);
impl Length {
    pub fn millimetres(value: f64) -> Result<Self, ModelError> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(ModelError::NonFiniteLength)
        }
    }
    #[must_use]
    pub const fn as_millimetres(self) -> f64 {
        self.0
    }
    #[must_use]
    pub fn display(self) -> String {
        format!("{:.1} mm", self.0)
    }
}

/// A parameter is either a literal length or a small expression evaluated against named values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterValue {
    Literal(String),
    Expression(String),
}
impl ParameterValue {
    #[must_use]
    pub fn literal(length: Length) -> Self {
        Self::Literal(length.display())
    }
    pub fn evaluate(&self, parameters: &BTreeMap<String, Length>) -> Result<Length, ModelError> {
        evaluate_length(
            match self {
                Self::Literal(source) | Self::Expression(source) => source,
            },
            parameters,
        )
    }
}

/// One real document feature.
#[derive(Debug, Clone, PartialEq)]
pub enum Feature {
    Base {
        name: String,
        outputs: Vec<EntityId>,
    },
    Extrude(ExtrudeFeature),
}
impl Feature {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Base { name, .. } => name,
            Self::Extrude(feature) => &feature.name,
        }
    }
    #[must_use]
    pub fn outputs(&self) -> &[EntityId] {
        match self {
            Self::Base { outputs, .. } => outputs,
            Self::Extrude(feature) => &feature.outputs,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtrudeGeometry {
    #[default]
    Solid,
    Surface,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtrudeResult {
    #[default]
    New,
    Add,
    Remove,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtrudeTermination {
    #[default]
    Blind,
    UpTo,
    ThroughAll,
}

/// Semantic values supplied by the Extrude Action Panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtrudeSpec {
    pub profile: Option<EntityId>,
    pub geometry: ExtrudeGeometry,
    pub result: ExtrudeResult,
    pub termination: ExtrudeTermination,
    pub depth: ParameterValue,
    pub starting_offset: Option<ParameterValue>,
    pub end_offset: Option<ParameterValue>,
    pub symmetric: bool,
    pub second_end: Option<ParameterValue>,
    pub invert: bool,
    pub merge_with_all: bool,
    pub merge_scope: Vec<EntityId>,
}
impl Default for ExtrudeSpec {
    fn default() -> Self {
        Self {
            profile: None,
            geometry: ExtrudeGeometry::Solid,
            result: ExtrudeResult::New,
            termination: ExtrudeTermination::Blind,
            depth: ParameterValue::Literal("0.0 mm".to_owned()),
            starting_offset: None,
            end_offset: None,
            symmetric: false,
            second_end: None,
            invert: false,
            merge_with_all: true,
            merge_scope: Vec::new(),
        }
    }
}

/// A committed extrude in the parametric history.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtrudeFeature {
    pub name: String,
    pub outputs: Vec<EntityId>,
    pub spec: ExtrudeSpec,
}

/// Persisted parametric document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Document {
    features: Vec<(FeatureId, Feature)>,
    parameters: BTreeMap<String, Length>,
    next_feature: u64,
    next_entity: u64,
    next_folder: u64,
    entities: BTreeMap<EntityId, Entity>,
    folders: BTreeMap<FolderId, FeatureFolder>,
    layout: Vec<HistoryLayoutRow>,
}

/// Stable identity of a history folder; independent of names and operation ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FolderId(u64);
impl FolderId {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructionKind {
    Plane,
    Axis,
    Curve,
    Point,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Body,
    Surface,
    Construction(ConstructionKind),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entity {
    id: EntityId,
    name: String,
    kind: EntityKind,
    visible: bool,
}
impl Entity {
    #[must_use]
    pub const fn id(&self) -> EntityId {
        self.id
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub const fn kind(&self) -> EntityKind {
        self.kind
    }
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.visible
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEntity {
    pub name: String,
    pub kind: EntityKind,
    pub visible: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HistoryNodeId {
    Operation(FeatureId),
    Folder(FolderId),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryLayoutRow {
    pub node: HistoryNodeId,
    pub depth: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureFolder {
    id: FolderId,
    name: String,
}
impl FeatureFolder {
    #[must_use]
    pub const fn id(&self) -> FolderId {
        self.id
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtrudeReference {
    Profile,
    MergeScope { index: usize },
}
impl Document {
    #[must_use]
    pub fn features(&self) -> &[(FeatureId, Feature)] {
        &self.features
    }

    /// Resolves a feature id in this document's history.
    #[must_use]
    pub fn feature(&self, id: FeatureId) -> Option<&Feature> {
        self.features
            .iter()
            .find_map(|(current, feature)| (*current == id).then_some(feature))
    }
    #[must_use]
    pub fn parameters(&self) -> &BTreeMap<String, Length> {
        &self.parameters
    }
    pub fn set_parameter(&mut self, name: impl Into<String>, value: Length) {
        self.parameters.insert(name.into(), value);
    }
    #[must_use]
    pub fn entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }
    #[must_use]
    pub fn folder(&self, id: FolderId) -> Option<&FeatureFolder> {
        self.folders.get(&id)
    }
    #[must_use]
    pub fn history_layout(&self) -> &[HistoryLayoutRow] {
        &self.layout
    }

    pub fn add_base_solid(&mut self, name: impl Into<String>) -> Result<FeatureId, ModelError> {
        let name = name.into();
        self.add_base_feature(
            name.clone(),
            vec![NewEntity {
                name,
                kind: EntityKind::Body,
                visible: true,
            }],
        )
    }
    pub fn add_base_feature(
        &mut self,
        name: String,
        outputs: Vec<NewEntity>,
    ) -> Result<FeatureId, ModelError> {
        valid_name(&name)?;
        for output in &outputs {
            valid_name(&output.name)?;
        }
        self.capacity(outputs.len())?;
        let outputs = self.insert_outputs(outputs);
        let id = self.insert_feature(self.features.len(), Feature::Base { name, outputs });
        self.layout.push(HistoryLayoutRow {
            node: HistoryNodeId::Operation(id),
            depth: 0,
        });
        Ok(id)
    }
    pub fn create_folder(&mut self, name: String) -> Result<FolderId, ModelError> {
        valid_name(&name)?;
        let next = self
            .next_folder
            .checked_add(1)
            .ok_or(ModelError::IdentityExhausted)?;
        let id = FolderId(self.next_folder);
        self.folders.insert(id, FeatureFolder { id, name });
        self.layout.push(HistoryLayoutRow {
            node: HistoryNodeId::Folder(id),
            depth: 0,
        });
        self.next_folder = next;
        Ok(id)
    }
    pub fn set_history_layout(&mut self, rows: Vec<HistoryLayoutRow>) -> Result<bool, ModelError> {
        let mut seen = BTreeSet::new();
        let mut operations = Vec::new();
        let mut previous: Option<HistoryLayoutRow> = None;
        for row in &rows {
            if !seen.insert(row.node) || previous.is_none() && row.depth != 0 {
                return Err(ModelError::InvalidHistoryLayout);
            }
            if let Some(prev) = previous
                && row.depth > prev.depth
                && (row.depth.checked_sub(prev.depth) != Some(1)
                    || !matches!(prev.node, HistoryNodeId::Folder(_)))
            {
                return Err(ModelError::InvalidHistoryLayout);
            }
            match row.node {
                HistoryNodeId::Operation(id) if self.feature(id).is_some() => operations.push(id),
                HistoryNodeId::Folder(id) if self.folder(id).is_some() => {}
                _ => return Err(ModelError::InvalidHistoryLayout),
            }
            previous = Some(*row);
        }
        if seen.len() != self.features.len() + self.folders.len()
            || !operations
                .iter()
                .copied()
                .eq(self.features.iter().map(|(id, _)| *id))
        {
            return Err(ModelError::InvalidHistoryLayout);
        }
        if rows == self.layout {
            return Ok(false);
        }
        self.layout = rows;
        Ok(true)
    }
    pub fn rename_feature(&mut self, id: FeatureId, name: String) -> Result<bool, ModelError> {
        let feature = self
            .features
            .iter_mut()
            .find_map(|(current, value)| (*current == id).then_some(value))
            .ok_or(ModelError::UnknownFeature(id))?;
        let old = match feature {
            Feature::Base { name, .. } => name,
            Feature::Extrude(value) => &mut value.name,
        };
        rename(old, name)
    }
    pub fn rename_folder(&mut self, id: FolderId, name: String) -> Result<bool, ModelError> {
        rename(
            &mut self
                .folders
                .get_mut(&id)
                .ok_or(ModelError::UnknownFolder(id))?
                .name,
            name,
        )
    }
    pub fn rename_entity(&mut self, id: EntityId, name: String) -> Result<bool, ModelError> {
        rename(
            &mut self
                .entities
                .get_mut(&id)
                .ok_or(ModelError::UnknownEntity(id))?
                .name,
            name,
        )
    }
    pub fn set_entity_visibility(
        &mut self,
        id: EntityId,
        visible: bool,
    ) -> Result<bool, ModelError> {
        let entity = self
            .entities
            .get_mut(&id)
            .ok_or(ModelError::UnknownEntity(id))?;
        let changed = entity.visible != visible;
        entity.visible = visible;
        Ok(changed)
    }
    pub fn commit_extrude(&mut self, spec: ExtrudeSpec) -> Result<FeatureId, ModelError> {
        self.commit_extrude_before(spec, None)
    }
    fn insertion_index(&self, before: Option<FeatureId>) -> Result<usize, ModelError> {
        before.map_or(Ok(self.features.len()), |id| {
            self.features
                .iter()
                .position(|(current, _)| *current == id)
                .ok_or(ModelError::UnknownFeature(id))
        })
    }
    fn capacity(&self, outputs: usize) -> Result<(), ModelError> {
        self.next_feature
            .checked_add(1)
            .ok_or(ModelError::IdentityExhausted)?;
        self.next_entity
            .checked_add(u64::try_from(outputs).map_err(|_| ModelError::IdentityExhausted)?)
            .ok_or(ModelError::IdentityExhausted)?;
        Ok(())
    }
    pub fn validate_extrude_insertion(
        &self,
        spec: &ExtrudeSpec,
        before: Option<FeatureId>,
    ) -> Result<(), ModelError> {
        let insertion_index = self.insertion_index(before)?;
        let references = spec
            .profile
            .map(|id| (id, ExtrudeReference::Profile))
            .into_iter()
            .chain(
                spec.merge_scope
                    .iter()
                    .enumerate()
                    .map(|(index, id)| (*id, ExtrudeReference::MergeScope { index })),
            );
        for (entity, reference) in references {
            if !self.entities.contains_key(&entity) {
                return Err(ModelError::UnknownReferencedEntity { entity, reference });
            }
            let mut producers =
                self.features
                    .iter()
                    .enumerate()
                    .flat_map(|(index, (id, feature))| {
                        feature
                            .outputs()
                            .iter()
                            .filter(move |output| **output == entity)
                            .map(move |_| (index, *id))
                    });
            let Some((index, producer)) = producers.next() else {
                return Err(ModelError::InvalidEntityProducer { entity });
            };
            if producers.next().is_some() {
                return Err(ModelError::InvalidEntityProducer { entity });
            }
            if index >= insertion_index {
                return Err(ModelError::ReferenceNotBeforeInsertion {
                    entity,
                    producer,
                    reference,
                    insertion_index,
                });
            }
        }
        validate_extrude(spec, &self.parameters)?;
        self.capacity(1)
    }
    pub fn commit_extrude_before(
        &mut self,
        spec: ExtrudeSpec,
        before: Option<FeatureId>,
    ) -> Result<FeatureId, ModelError> {
        self.validate_extrude_insertion(&spec, before)?;
        let index = self.insertion_index(before)?;
        let layout_index = before
            .and_then(|id| {
                self.layout
                    .iter()
                    .position(|row| row.node == HistoryNodeId::Operation(id))
            })
            .unwrap_or(self.layout.len());
        let depth = self.layout.get(layout_index).map_or(0, |row| row.depth);
        let kind = match spec.geometry {
            ExtrudeGeometry::Solid => EntityKind::Body,
            ExtrudeGeometry::Surface => EntityKind::Surface,
        };
        let outputs = self.insert_outputs(vec![NewEntity {
            name: "Extrude".to_owned(),
            kind,
            visible: true,
        }]);
        let id = self.insert_feature(
            index,
            Feature::Extrude(ExtrudeFeature {
                name: "Extrude".to_owned(),
                outputs,
                spec,
            }),
        );
        self.layout.insert(
            layout_index,
            HistoryLayoutRow {
                node: HistoryNodeId::Operation(id),
                depth,
            },
        );
        Ok(id)
    }
    // Capacity and names have been validated before either infallible mutation helper.
    fn insert_outputs(&mut self, outputs: Vec<NewEntity>) -> Vec<EntityId> {
        outputs
            .into_iter()
            .map(|output| {
                let id = EntityId(self.next_entity);
                self.next_entity += 1;
                self.entities.insert(
                    id,
                    Entity {
                        id,
                        name: output.name,
                        kind: output.kind,
                        visible: output.visible,
                    },
                );
                id
            })
            .collect()
    }
    fn insert_feature(&mut self, index: usize, feature: Feature) -> FeatureId {
        let id = FeatureId(self.next_feature);
        self.next_feature += 1;
        self.features.insert(index, (id, feature));
        id
    }
}

/// Selection shared by viewport, Feature tree, Elements tree and selection fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    entities: Vec<EntityId>,
}
impl Selection {
    #[must_use]
    pub fn entities(&self) -> &[EntityId] {
        &self.entities
    }
    pub fn replace(&mut self, entities: impl IntoIterator<Item = EntityId>) {
        self.entities.clear();
        for entity in entities {
            if !self.entities.contains(&entity) {
                self.entities.push(entity);
            }
        }
    }
    pub fn toggle(&mut self, entity: EntityId) {
        if let Some(index) = self.entities.iter().position(|current| *current == entity) {
            self.entities.remove(index);
        } else {
            self.entities.push(entity);
        }
    }
}

/// Generic command state used by the application; the UI only projects this state.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum CommandState {
    #[default]
    Idle,
    Extrude {
        draft: ExtrudeSpec,
    },
}

/// One command transaction, including a non-persisted preview.
#[derive(Debug, Clone, Default)]
pub struct CommandController {
    state: CommandState,
    preview: Option<ExtrudeSpec>,
}
impl CommandController {
    #[must_use]
    pub const fn state(&self) -> &CommandState {
        &self.state
    }
    #[must_use]
    pub const fn preview(&self) -> Option<&ExtrudeSpec> {
        self.preview.as_ref()
    }
    pub fn begin_extrude(&mut self, selection: &Selection) {
        let draft = ExtrudeSpec {
            profile: selection.entities().first().copied(),
            ..ExtrudeSpec::default()
        };
        self.state = CommandState::Extrude { draft };
        self.preview = None;
    }
    pub fn edit_extrude(
        &mut self,
        draft: ExtrudeSpec,
        document: &Document,
    ) -> Result<(), ModelError> {
        validate_extrude(&draft, document.parameters())?;
        self.preview = Some(draft.clone());
        self.state = CommandState::Extrude { draft };
        Ok(())
    }
    pub fn confirm(&mut self, document: &mut Document) -> Result<Option<FeatureId>, ModelError> {
        self.confirm_before(document, None)
    }
    pub fn confirm_before(
        &mut self,
        document: &mut Document,
        before: Option<FeatureId>,
    ) -> Result<Option<FeatureId>, ModelError> {
        let CommandState::Extrude { draft } = &self.state else {
            return Ok(None);
        };
        let feature = document.commit_extrude_before(draft.clone(), before)?;
        self.cancel();
        Ok(Some(feature))
    }
    pub fn cancel(&mut self) {
        self.state = CommandState::Idle;
        self.preview = None;
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ModelError {
    #[error("document identity capacity exhausted")]
    IdentityExhausted,
    #[error("unknown feature {0:?}")]
    UnknownFeature(FeatureId),
    #[error("unknown folder {0:?}")]
    UnknownFolder(FolderId),
    #[error("unknown entity {0:?}")]
    UnknownEntity(EntityId),
    #[error("name must not be empty")]
    EmptyName,
    #[error("invalid history layout")]
    InvalidHistoryLayout,
    #[error("unknown entity {entity:?} referenced by {reference:?}")]
    UnknownReferencedEntity {
        entity: EntityId,
        reference: ExtrudeReference,
    },
    #[error("entity {entity:?} has no unique producer")]
    InvalidEntityProducer { entity: EntityId },
    #[error(
        "entity {entity:?} from {producer:?} referenced by {reference:?} is not before insertion {insertion_index}"
    )]
    ReferenceNotBeforeInsertion {
        entity: EntityId,
        producer: FeatureId,
        reference: ExtrudeReference,
        insertion_index: usize,
    },
    #[error("a length must be finite")]
    NonFiniteLength,
    #[error("invalid length expression: {0}")]
    InvalidExpression(String),
    #[error("an extrusion depth must be greater than zero")]
    NonPositiveDepth,
    #[error("{0} termination requires a selected target")]
    MissingTarget(&'static str),
    #[error("Add or Remove requires a merge target when Merge with all is disabled")]
    MissingMergeTarget,
}

fn valid_name(name: &str) -> Result<(), ModelError> {
    if name.trim().is_empty() {
        Err(ModelError::EmptyName)
    } else {
        Ok(())
    }
}
fn rename(old: &mut String, name: String) -> Result<bool, ModelError> {
    valid_name(&name)?;
    if *old == name {
        return Ok(false);
    }
    *old = name;
    Ok(true)
}

fn validate_extrude(
    spec: &ExtrudeSpec,
    parameters: &BTreeMap<String, Length>,
) -> Result<(), ModelError> {
    let depth = spec.depth.evaluate(parameters)?;
    if spec.termination == ExtrudeTermination::Blind && depth.as_millimetres() <= 0.0 {
        return Err(ModelError::NonPositiveDepth);
    }
    if spec.termination == ExtrudeTermination::UpTo && spec.profile.is_none() {
        return Err(ModelError::MissingTarget("Up to"));
    }
    if matches!(spec.result, ExtrudeResult::Add | ExtrudeResult::Remove)
        && !spec.merge_with_all
        && spec.merge_scope.is_empty()
    {
        return Err(ModelError::MissingMergeTarget);
    }
    Ok(())
}

fn evaluate_length(
    source: &str,
    parameters: &BTreeMap<String, Length>,
) -> Result<Length, ModelError> {
    let compact = source.trim().trim_end_matches("mm").trim();
    if let Ok(value) = compact.parse::<f64>() {
        return Length::millimetres(value);
    }
    let mut total = 0.0;
    let mut sign = 1.0;
    for token in compact.split_whitespace() {
        match token {
            "+" => sign = 1.0,
            "-" => sign = -1.0,
            name => {
                let value = name
                    .parse::<f64>()
                    .ok()
                    .or_else(|| parameters.get(name).map(|length| length.as_millimetres()));
                let Some(value) = value else {
                    return Err(ModelError::InvalidExpression(source.to_owned()));
                };
                total += sign * value;
                sign = 1.0;
            }
        }
    }
    if total.is_finite() {
        Length::millimetres(total)
    } else {
        Err(ModelError::InvalidExpression(source.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn draft() -> ExtrudeSpec {
        ExtrudeSpec {
            depth: ParameterValue::Literal("10 mm".to_owned()),
            ..ExtrudeSpec::default()
        }
    }
    #[test]
    fn insertion_preflights_all_explicit_references_atomically() {
        let mut document = Document::default();
        let first = document.add_base_solid("first").unwrap();
        let second = document.add_base_solid("second").unwrap();
        let earlier = document.feature(first).unwrap().outputs()[0];
        let later = document.feature(second).unwrap().outputs()[0];
        let unknown = EntityId::from_raw(999);
        let unchanged = document.clone();
        assert_eq!(
            document.commit_extrude_before(
                ExtrudeSpec {
                    profile: Some(unknown),
                    ..draft()
                },
                Some(FeatureId(999))
            ),
            Err(ModelError::UnknownFeature(FeatureId(999)))
        );
        assert_eq!(
            document, unchanged,
            "boundary validation precedes even missing references"
        );
        assert_eq!(
            document.commit_extrude_before(
                ExtrudeSpec {
                    merge_scope: vec![later],
                    merge_with_all: true,
                    ..draft()
                },
                Some(first)
            ),
            Err(ModelError::ReferenceNotBeforeInsertion {
                entity: later,
                producer: second,
                reference: ExtrudeReference::MergeScope { index: 0 },
                insertion_index: 0
            })
        );
        assert_eq!(
            document, unchanged,
            "a strictly later producer is a forward reference too"
        );
        let cases = [
            (
                Some(unknown),
                vec![later],
                ModelError::UnknownReferencedEntity {
                    entity: unknown,
                    reference: ExtrudeReference::Profile,
                },
            ),
            (
                Some(later),
                vec![],
                ModelError::ReferenceNotBeforeInsertion {
                    entity: later,
                    producer: second,
                    reference: ExtrudeReference::Profile,
                    insertion_index: 1,
                },
            ),
            (
                Some(earlier),
                vec![earlier, later],
                ModelError::ReferenceNotBeforeInsertion {
                    entity: later,
                    producer: second,
                    reference: ExtrudeReference::MergeScope { index: 1 },
                    insertion_index: 1,
                },
            ),
            (
                None,
                vec![earlier, unknown],
                ModelError::UnknownReferencedEntity {
                    entity: unknown,
                    reference: ExtrudeReference::MergeScope { index: 1 },
                },
            ),
        ];
        for (profile, merge_scope, error) in cases {
            let spec = ExtrudeSpec {
                profile,
                merge_scope,
                ..draft()
            };
            let mut command = CommandController::default();
            command.edit_extrude(spec.clone(), &document).unwrap();
            let original = document.clone();
            let state = command.state().clone();
            let preview = command.preview().cloned();
            assert_eq!(
                document.validate_extrude_insertion(&spec, Some(second)),
                Err(error)
            );
            assert!(command.confirm_before(&mut document, Some(second)).is_err());
            assert_eq!(document, original);
            assert_eq!(command.state(), &state);
            assert_eq!(command.preview(), preview.as_ref());
        }
        let id = document
            .commit_extrude_before(
                ExtrudeSpec {
                    profile: Some(earlier),
                    merge_scope: vec![earlier, earlier],
                    ..draft()
                },
                Some(second),
            )
            .unwrap();
        assert_eq!(id.get(), 2);
        assert_eq!(
            document
                .features()
                .iter()
                .map(|(id, _)| *id)
                .collect::<Vec<_>>(),
            vec![first, id, second]
        );
        assert_eq!(document.feature(id).unwrap().outputs()[0].get(), 2);
    }
    #[test]
    fn folders_preserve_history_and_insertion_parent() {
        let mut document = Document::default();
        let first = document
            .add_base_feature("empty operation".into(), vec![])
            .unwrap();
        let second = document
            .add_base_feature(
                "multiple".into(),
                vec![
                    NewEntity {
                        name: "surface".into(),
                        kind: EntityKind::Surface,
                        visible: false,
                    },
                    NewEntity {
                        name: "plane".into(),
                        kind: EntityKind::Construction(ConstructionKind::Plane),
                        visible: true,
                    },
                ],
            )
            .unwrap();
        let a = document.create_folder("A".into()).unwrap();
        let b = document.create_folder("B".into()).unwrap();
        let rows = vec![
            HistoryLayoutRow {
                node: HistoryNodeId::Folder(a),
                depth: 0,
            },
            HistoryLayoutRow {
                node: HistoryNodeId::Operation(first),
                depth: 1,
            },
            HistoryLayoutRow {
                node: HistoryNodeId::Folder(b),
                depth: 0,
            },
            HistoryLayoutRow {
                node: HistoryNodeId::Operation(second),
                depth: 1,
            },
        ];
        assert!(document.set_history_layout(rows.clone()).unwrap());
        assert!(!document.set_history_layout(rows).unwrap());
        let original = document.clone();
        assert!(document.set_history_layout(vec![]).is_err());
        assert_eq!(document, original);
        let inserted = document
            .commit_extrude_before(draft(), Some(second))
            .unwrap();
        assert_eq!(
            document.history_layout()[3],
            HistoryLayoutRow {
                node: HistoryNodeId::Operation(inserted),
                depth: 1
            }
        );
        assert_eq!(document.history_layout()[2].node, HistoryNodeId::Folder(b));
        assert_eq!(document.feature(second).unwrap().outputs().len(), 2);
    }
    #[test]
    fn allocation_exhaustion_and_names_are_atomic() {
        let mut document = Document {
            next_entity: u64::MAX,
            ..Document::default()
        };
        let original = document.clone();
        assert_eq!(
            document.add_base_solid("body"),
            Err(ModelError::IdentityExhausted)
        );
        assert_eq!(
            document.commit_extrude(draft()),
            Err(ModelError::IdentityExhausted)
        );
        assert_eq!(document, original);
        document.next_entity = 0;
        let id = document.add_base_solid("body").unwrap();
        let name = format!(" {} ", "long".repeat(40));
        assert!(document.rename_feature(id, name.clone()).unwrap());
        assert_eq!(document.feature(id).unwrap().name(), name);
        assert!(!document.rename_feature(id, name).unwrap());
        assert_eq!(
            document.rename_feature(id, "  ".into()),
            Err(ModelError::EmptyName)
        );
        let entity = document.feature(id).unwrap().outputs()[0];
        assert_eq!(document.entity(entity).unwrap().name(), "body");
    }
    #[test]
    fn expression_parameter_drives_a_committed_feature() {
        let mut document = Document::default();
        document.set_parameter("wall", Length::millimetres(2.5).unwrap());
        let mut command = CommandController::default();
        command.begin_extrude(&Selection::default());
        let draft = ExtrudeSpec {
            depth: ParameterValue::Expression("wall + 7.5 mm".to_owned()),
            ..ExtrudeSpec::default()
        };
        command.edit_extrude(draft, &document).unwrap();
        assert!(command.preview().is_some());
        assert!(command.confirm(&mut document).unwrap().is_some());
        assert!(matches!(command.state(), CommandState::Idle));
        assert_eq!(document.features().len(), 1);
    }
    #[test]
    fn cancel_discards_preview_without_touching_history() {
        let document = Document::default();
        let mut command = CommandController::default();
        command.begin_extrude(&Selection::default());
        command
            .edit_extrude(
                ExtrudeSpec {
                    depth: ParameterValue::Literal("10 mm".to_owned()),
                    ..ExtrudeSpec::default()
                },
                &document,
            )
            .unwrap();
        command.cancel();
        assert!(command.preview().is_none());
        assert!(document.features().is_empty());
    }
    #[test]
    fn merge_scope_is_required_for_targeted_boolean_results() {
        let document = Document::default();
        let mut command = CommandController::default();
        command.begin_extrude(&Selection::default());
        let draft = ExtrudeSpec {
            result: ExtrudeResult::Add,
            merge_with_all: false,
            depth: ParameterValue::Literal("10 mm".to_owned()),
            ..ExtrudeSpec::default()
        };
        assert_eq!(
            command.edit_extrude(draft, &document),
            Err(ModelError::MissingMergeTarget)
        );
    }
}
