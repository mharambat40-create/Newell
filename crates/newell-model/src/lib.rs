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

use std::collections::BTreeMap;

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
    BaseSolid { name: String, entity: EntityId },
    Extrude(ExtrudeFeature),
}
impl Feature {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::BaseSolid { name, .. } => name,
            Self::Extrude(feature) => &feature.name,
        }
    }
    #[must_use]
    pub const fn entity(&self) -> EntityId {
        match self {
            Self::BaseSolid { entity, .. } => *entity,
            Self::Extrude(feature) => feature.entity,
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
    pub entity: EntityId,
    pub spec: ExtrudeSpec,
}

/// Persisted parametric document.
#[derive(Debug, Clone, Default)]
pub struct Document {
    features: Vec<(FeatureId, Feature)>,
    parameters: BTreeMap<String, Length>,
    next_feature: u64,
    next_entity: u64,
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
    pub fn add_base_solid(&mut self, name: impl Into<String>) -> FeatureId {
        let entity = self.allocate_entity();
        self.push(Feature::BaseSolid {
            name: name.into(),
            entity,
        })
    }
    pub fn commit_extrude(&mut self, spec: ExtrudeSpec) -> Result<FeatureId, ModelError> {
        validate_extrude(&spec, &self.parameters)?;
        let entity = self.allocate_entity();
        Ok(self.push(Feature::Extrude(ExtrudeFeature {
            name: "Extrude".to_owned(),
            entity,
            spec,
        })))
    }
    fn allocate_entity(&mut self) -> EntityId {
        let id = EntityId(self.next_entity);
        self.next_entity = self.next_entity.saturating_add(1);
        id
    }
    fn push(&mut self, feature: Feature) -> FeatureId {
        let id = FeatureId(self.next_feature);
        self.next_feature = self.next_feature.saturating_add(1);
        self.features.push((id, feature));
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
        let CommandState::Extrude { draft } = &self.state else {
            return Ok(None);
        };
        let feature = document.commit_extrude(draft.clone())?;
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
