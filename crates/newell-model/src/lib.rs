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

pub mod formula;
pub mod geometry;
pub mod parameters;
mod quantity;
pub use quantity::{Angle, Direction, Length, Quantity, QuantityKind, Scalar, Unit};

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

/// Full-precision literal or stable registry identity. Formula source and AST
/// belong exclusively to the registry definition.
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterValue {
    /// Full-precision canonical data; formatting is never part of literal construction.
    Literal(Quantity),
    /// Stable definition identity; direction remains field metadata.
    Bound(formula::ParameterId),
}
impl ParameterValue {
    /// Wraps a length directly, retaining its original canonical value.
    #[must_use]
    pub fn literal(length: Length) -> Self {
        Self::Literal(Quantity::Length(length))
    }
    /// Evaluates the length-only compatibility API without reparsing typed literals.
    ///
    /// # Errors
    /// Rejects non-length literals and propagates errors from the existing length evaluator.
    pub fn evaluate(
        &self,
        parameters: &parameters::ParameterRegistry,
    ) -> Result<Length, ModelError> {
        match self {
            Self::Literal(Quantity::Length(length)) => Ok(*length),
            Self::Literal(_) => Err(ModelError::DimensionMismatch),
            Self::Bound(id) => match parameters
                .definition(*id)
                .map(|definition| definition.value)
            {
                Some(Quantity::Length(length)) => Ok(length),
                Some(_) => Err(ModelError::DimensionMismatch),
                None => Err(ModelError::Formula(formula::FormulaError {
                    code: formula::FormulaErrorCode::MissingParameter,
                    span: formula::SourceSpan { start: 0, end: 0 },
                })),
            },
        }
    }
    /// Evaluates a parameter and checks the destination's required dimension.
    ///
    /// # Errors
    /// Rejects dimension mismatch, unsupported non-length expressions, and invalid length
    /// expressions. Command-specific magnitude and range validation happens separately.
    pub fn evaluate_quantity(
        &self,
        expected: QuantityKind,
        parameters: &parameters::ParameterRegistry,
    ) -> Result<Quantity, ModelError> {
        let value = match self {
            Self::Literal(q) => *q,
            Self::Bound(id) => parameters
                .definition(*id)
                .map(|definition| definition.value)
                .ok_or({
                    ModelError::Formula(formula::FormulaError {
                        code: formula::FormulaErrorCode::MissingParameter,
                        span: formula::SourceSpan { start: 0, end: 0 },
                    })
                })?,
        };
        if value.kind() != expected {
            return Err(ModelError::DimensionMismatch);
        }
        Ok(value)
    }
}

/// One real document feature.
#[derive(Debug, Clone, PartialEq)]
pub enum Feature {
    Base {
        name: String,
        outputs: Vec<EntityId>,
    },
    Extrude(Box<ExtrudeFeature>),
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

/// An active offset; hidden UI drafts are never stored in history.
#[derive(Debug, Clone, PartialEq)]
pub enum OffsetSpec {
    Blind {
        distance: ParameterValue,
        direction: Direction,
    },
    UpTo(geometry::FaceRef),
}
#[derive(Debug, Clone, PartialEq)]
pub enum ExtentSpec {
    Blind {
        depth: ParameterValue,
        direction: Direction,
        along: Option<geometry::DirectionRef>,
        start: Option<OffsetSpec>,
        end: Option<OffsetSpec>,
    },
    UpTo {
        target: geometry::FaceRef,
        clearance: Option<ParameterValue>,
    },
    ThroughAll {
        invert: bool,
        start: Option<OffsetSpec>,
    },
}
#[derive(Debug, Clone, PartialEq)]
pub enum SecondSpec {
    Blind {
        depth: ParameterValue,
        direction: Direction,
    },
    UpTo {
        target: geometry::FaceRef,
        end: Option<SecondOffsetSpec>,
    },
}
#[derive(Debug, Clone, PartialEq)]
pub enum SecondOffsetSpec {
    Clearance(ParameterValue),
    UpTo(geometry::FaceRef),
}
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SidesSpec {
    #[default]
    One,
    Symmetric,
    Second(SecondSpec),
}
#[derive(Debug, Clone, PartialEq, Default)]
pub enum MergePolicy {
    #[default]
    All,
    Explicit(Vec<EntityId>),
}
/// Model-owned active recipe. A missing profile is permitted only in an unprepared
/// command draft; geometry preparation rejects it before allocating any output.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtrudeSpec {
    pub profile: Option<geometry::ProfileRef>,
    pub geometry: ExtrudeGeometry,
    pub result: ExtrudeResult,
    pub extent: ExtentSpec,
    pub sides: SidesSpec,
    pub targets: MergePolicy,
}
impl Default for ExtrudeSpec {
    fn default() -> Self {
        Self {
            profile: None,
            geometry: ExtrudeGeometry::Solid,
            result: ExtrudeResult::New,
            extent: ExtentSpec::Blind {
                depth: ParameterValue::literal(Length::ZERO),
                direction: Direction::Forward,
                along: None,
                start: None,
                end: None,
            },
            sides: SidesSpec::One,
            targets: MergePolicy::All,
        }
    }
}
impl ExtrudeSpec {
    /// Stores only effective operands after the frozen edit has been validated.
    /// The original validation envelope remains sealed in the prepared result.
    pub(crate) fn active_recipe(&self) -> Self {
        let mut recipe = self.clone();
        if recipe.result == ExtrudeResult::New {
            recipe.targets = MergePolicy::All;
        } else if let MergePolicy::Explicit(targets) = &mut recipe.targets {
            let mut seen = std::collections::BTreeSet::new();
            targets.retain(|entity| seen.insert(*entity));
        }
        recipe
    }
    /// Derived field projection for command hydration and kernel lowering. It has
    /// no independent lifetime or authority and never contains inactive drafts.
    #[must_use]
    pub fn inputs(&self) -> ExtrudeInputProjection {
        let mut fields = ExtrudeInputProjection {
            profile_geometry: self.profile,
            profile: self.profile.map(|p| p.support().entity),
            geometry: self.geometry,
            result: self.result,
            ..ExtrudeInputProjection::default()
        };
        fn offset(
            value: &Option<OffsetSpec>,
            distance: &mut Option<ParameterValue>,
            direction: &mut Direction,
            target: &mut Option<geometry::FaceRef>,
        ) {
            match value {
                Some(OffsetSpec::Blind {
                    distance: v,
                    direction: d,
                }) => {
                    *distance = Some(v.clone());
                    *direction = *d;
                }
                Some(OffsetSpec::UpTo(face)) => *target = Some(*face),
                None => {}
            }
        }
        match &self.extent {
            ExtentSpec::Blind {
                depth,
                direction,
                along,
                start,
                end,
            } => {
                fields.depth = depth.clone();
                fields.directions.depth = *direction;
                fields.along_geometry = *along;
                offset(
                    start,
                    &mut fields.starting_offset,
                    &mut fields.directions.starting_offset,
                    &mut fields.targets.start,
                );
                offset(
                    end,
                    &mut fields.end_offset,
                    &mut fields.directions.end_offset,
                    &mut fields.targets.end,
                );
            }
            ExtentSpec::UpTo { target, clearance } => {
                fields.termination = ExtrudeTermination::UpTo;
                fields.targets.main = Some(*target);
                fields.main_clearance = clearance.clone();
            }
            ExtentSpec::ThroughAll { invert, start } => {
                fields.termination = ExtrudeTermination::ThroughAll;
                fields.invert = *invert;
                offset(
                    start,
                    &mut fields.starting_offset,
                    &mut fields.directions.starting_offset,
                    &mut fields.targets.start,
                );
            }
        }
        match &self.sides {
            SidesSpec::One => {}
            SidesSpec::Symmetric => fields.symmetric = true,
            SidesSpec::Second(SecondSpec::Blind { depth, direction }) => {
                fields.second_end = Some(depth.clone());
                fields.directions.second_end = *direction;
            }
            SidesSpec::Second(SecondSpec::UpTo { target, end }) => {
                fields.targets.second = Some(*target);
                match end {
                    Some(SecondOffsetSpec::Clearance(value)) => {
                        fields.second_end_offset = Some(value.clone())
                    }
                    Some(SecondOffsetSpec::UpTo(target)) => fields.targets.nested = Some(*target),
                    None => {}
                }
            }
        }
        if let MergePolicy::Explicit(targets) = &self.targets {
            fields.merge_with_all = false;
            fields.merge_scope = targets.clone();
        }
        fields
    }
}
/// Semantic values supplied by the Extrude Action Panel.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtrudeInputProjection {
    /// Generation-qualified real supporting face/boundary for prepared geometry.
    pub profile_geometry: Option<geometry::ProfileRef>,
    pub along_geometry: Option<geometry::DirectionRef>,
    /// Active, generation-qualified target conditions; hidden branches are absent.
    pub targets: geometry::ExtrudeTargets,
    /// Unsigned main Up to clearance, measured toward the displaced start.
    pub main_clearance: Option<ParameterValue>,
    pub profile: Option<EntityId>,
    pub geometry: ExtrudeGeometry,
    pub result: ExtrudeResult,
    pub termination: ExtrudeTermination,
    /// Retained primary magnitude/source; evaluated only for Blind termination.
    pub depth: ParameterValue,
    /// Active Blind starting displacement, or `None` for an inactive/selection branch.
    pub starting_offset: Option<ParameterValue>,
    /// Active Blind end displacement, or `None` for an inactive/selection branch.
    pub end_offset: Option<ParameterValue>,
    /// Unsigned nested offset under second-side UpTo, independent of `second_end`.
    pub second_end_offset: Option<ParameterValue>,
    /// Separate orientations; unused numeric destinations project as Forward.
    pub directions: ExtrudeDirections,
    pub symmetric: bool,
    /// Active second-side Blind magnitude; its reference orientation belongs to that side.
    pub second_end: Option<ParameterValue>,
    pub invert: bool,
    pub merge_with_all: bool,
    pub merge_scope: Vec<EntityId>,
}
/// Independent numeric orientations, all Forward by default; quantities remain magnitudes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExtrudeDirections {
    /// Primary Blind extent orientation.
    pub depth: Direction,
    /// Starting displacement orientation.
    pub starting_offset: Direction,
    /// End displacement orientation.
    pub end_offset: Direction,
    /// Orientation relative to the second-side reference.
    pub second_end: Direction,
}
impl Default for ExtrudeInputProjection {
    fn default() -> Self {
        Self {
            profile_geometry: None,
            along_geometry: None,
            targets: geometry::ExtrudeTargets::default(),
            main_clearance: None,
            profile: None,
            geometry: ExtrudeGeometry::Solid,
            result: ExtrudeResult::New,
            termination: ExtrudeTermination::Blind,
            depth: ParameterValue::literal(Length::ZERO),
            starting_offset: None,
            end_offset: None,
            second_end_offset: None,
            directions: ExtrudeDirections::default(),
            symmetric: false,
            second_end: None,
            invert: false,
            merge_with_all: true,
            merge_scope: Vec::new(),
        }
    }
}

impl ExtrudeInputProjection {
    /// Freezes only active branches. Missing selected targets are errors, not
    /// sentinel references or alternative extent behavior.
    pub fn into_spec(self) -> Result<ExtrudeSpec, ModelError> {
        let offset =
            |distance: Option<ParameterValue>, direction, target: Option<geometry::FaceRef>| {
                target.map(OffsetSpec::UpTo).or_else(|| {
                    distance.map(|distance| OffsetSpec::Blind {
                        distance,
                        direction,
                    })
                })
            };
        let start = offset(
            self.starting_offset,
            self.directions.starting_offset,
            self.targets.start,
        );
        let extent = match self.termination {
            ExtrudeTermination::Blind => ExtentSpec::Blind {
                depth: self.depth,
                direction: self.directions.depth,
                along: self.along_geometry,
                start,
                end: offset(
                    self.end_offset,
                    self.directions.end_offset,
                    self.targets.end,
                ),
            },
            ExtrudeTermination::UpTo => ExtentSpec::UpTo {
                target: self
                    .targets
                    .main
                    .ok_or(ModelError::MissingTarget("Up to"))?,
                clearance: self.main_clearance,
            },
            ExtrudeTermination::ThroughAll => ExtentSpec::ThroughAll {
                invert: self.invert,
                start,
            },
        };
        let sides = if self.symmetric {
            SidesSpec::Symmetric
        } else if let Some(target) = self.targets.second {
            SidesSpec::Second(SecondSpec::UpTo {
                target,
                end: self
                    .targets
                    .nested
                    .map(SecondOffsetSpec::UpTo)
                    .or_else(|| self.second_end_offset.map(SecondOffsetSpec::Clearance)),
            })
        } else if let Some(depth) = self.second_end {
            SidesSpec::Second(SecondSpec::Blind {
                depth,
                direction: self.directions.second_end,
            })
        } else {
            SidesSpec::One
        };
        let profile = self.profile_geometry;
        #[cfg(any(test, feature = "test-support"))]
        let profile = profile.or_else(|| {
            self.profile.map(|entity| {
                geometry::ProfileRef::Face(geometry::FaceRef {
                    entity,
                    generation: geometry::GeometryGeneration(0),
                    face: 0,
                })
            })
        });
        Ok(ExtrudeSpec {
            profile,
            geometry: self.geometry,
            result: self.result,
            extent,
            sides,
            targets: if self.merge_with_all {
                MergePolicy::All
            } else {
                MergePolicy::Explicit(self.merge_scope)
            },
        })
    }
}
/// A committed extrude in the parametric history.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtrudeFeature {
    pub(crate) consumed: Vec<EntityId>,
    pub name: String,
    pub outputs: Vec<EntityId>,
    pub spec: ExtrudeSpec,
}
impl ExtrudeFeature {
    /// Historical input versions replaced by this operation's actual result.
    #[must_use]
    pub fn consumed_inputs(&self) -> &[EntityId] {
        &self.consumed
    }
}

/// Persisted parametric document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Document {
    features: Vec<(FeatureId, Feature)>,
    seeded_features: BTreeSet<FeatureId>,
    parameter_registry: parameters::ParameterRegistry,
    base_geometry: BTreeMap<EntityId, geometry::BaseGeometryRecipe>,
    entity_geometry: BTreeMap<EntityId, geometry::EntityGeometry>,
    geometry_generation: geometry::GeometryGeneration,
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
    pub fn parameters(&self) -> &parameters::ParameterRegistry {
        &self.parameter_registry
    }
    /// Stages a document-owned named literal on this private document value.
    ///
    /// # Errors
    /// Rejects invalid names or invalidated dependencies without changing definitions.
    pub fn set_parameter(
        &mut self,
        name: impl Into<String>,
        value: Length,
    ) -> Result<(), ModelError> {
        let order = self
            .features
            .iter()
            .enumerate()
            .map(|(i, (id, _))| (*id, i))
            .collect();
        let candidate = self.parameter_registry.stage_edit(
            parameters::ParameterEdit {
                parameter: None,
                owner: parameters::ParameterOwner::Document,
                source: format!("#{} := {}mm", name.into(), value.as_millimetres()),
                destination: QuantityKind::Length,
                source_edit: None,
                source_origins: None,
            },
            &order,
        )?;
        self.parameter_registry = candidate.into_registry();
        Ok(())
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

    /// Creates a geometry seed with a stable producer identity, outside operation layout.
    pub fn add_seeded_solid(&mut self, name: impl Into<String>) -> Result<FeatureId, ModelError> {
        let id = self.add_base_solid(name)?;
        self.seeded_features.insert(id);
        self.layout
            .retain(|row| row.node != HistoryNodeId::Operation(id));
        Ok(id)
    }
    /// Seed producers remain available to geometry replay and entity dependencies.
    #[must_use]
    pub fn is_seeded_feature(&self, id: FeatureId) -> bool {
        self.seeded_features.contains(&id)
    }

    pub fn add_base_solid(&mut self, name: impl Into<String>) -> Result<FeatureId, ModelError> {
        let name = name.into();
        let unit = Length::millimetres(1.)?;
        let id = self.add_base_feature(
            name.clone(),
            vec![NewEntity {
                name,
                kind: EntityKind::Body,
                visible: true,
            }],
        )?;
        if let Some(entity) = self
            .feature(id)
            .and_then(|feature| feature.outputs().first())
            .copied()
        {
            self.base_geometry.insert(
                entity,
                geometry::BaseGeometryRecipe::Box { size: [unit; 3] },
            );
        }
        Ok(id)
    }
    /// Supplies an exact base recipe to headless integration tests only.
    ///
    /// # Errors
    /// Rejects the same invalid names/capacity as an ordinary base operation.
    #[cfg(feature = "test-support")]
    pub fn add_base_recipe_for_test(
        &mut self,
        name: &str,
        recipe: geometry::BaseGeometryRecipe,
    ) -> Result<FeatureId, ModelError> {
        let feature = self.add_base_solid(name)?;
        let entity = self
            .feature(feature)
            .and_then(|f| f.outputs().first())
            .copied()
            .ok_or(ModelError::InvalidGeometryReference)?;
        self.base_geometry.insert(entity, recipe);
        Ok(feature)
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
                HistoryNodeId::Operation(id)
                    if self.feature(id).is_some() && !self.is_seeded_feature(id) =>
                {
                    operations.push(id)
                }
                HistoryNodeId::Folder(id) if self.folder(id).is_some() => {}
                _ => return Err(ModelError::InvalidHistoryLayout),
            }
            previous = Some(*row);
        }
        if seen.len() != self.features.len() - self.seeded_features.len() + self.folders.len()
            || !operations.iter().copied().eq(self
                .features
                .iter()
                .map(|(id, _)| *id)
                .filter(|id| !self.is_seeded_feature(*id)))
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
    #[cfg(any(test, feature = "test-support"))]
    pub fn commit_extrude(&mut self, spec: ExtrudeSpec) -> Result<FeatureId, ModelError> {
        self.commit_extrude_before(spec, None)
    }
    fn insertion_index(&self, before: Option<FeatureId>) -> Result<usize, ModelError> {
        before.map_or(Ok(self.features.len()), |id| {
            if self.is_seeded_feature(id) {
                return Err(ModelError::UnknownFeature(id));
            }
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
        let inputs = spec.inputs();
        let references = inputs
            .profile
            .map(|id| (id, ExtrudeReference::Profile))
            .into_iter()
            .chain(
                inputs
                    .merge_scope
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
        validate_extrude(spec, self.parameters())?;
        self.capacity(1)
    }
    #[cfg(any(test, feature = "test-support"))]
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
            Feature::Extrude(Box::new(ExtrudeFeature {
                consumed: Vec::new(),
                name: "Extrude".to_owned(),
                outputs,
                spec,
            })),
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
    pub fn add(&mut self, entities: impl IntoIterator<Item = EntityId>) {
        for entity in entities {
            if !self.entities.contains(&entity) {
                self.entities.push(entity);
            }
        }
    }
    pub fn remove(&mut self, entities: impl IntoIterator<Item = EntityId>) {
        let removed: Vec<_> = entities.into_iter().collect();
        self.entities.retain(|entity| !removed.contains(entity));
    }
    pub fn clear(&mut self) {
        self.entities.clear();
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
        draft: Box<ExtrudeSpec>,
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
    pub fn begin_extrude(&mut self, _selection: &Selection) {
        let draft = ExtrudeSpec {
            profile: None,
            ..ExtrudeSpec::default()
        };
        self.state = CommandState::Extrude {
            draft: Box::new(draft),
        };
        self.preview = None;
    }
    pub fn edit_extrude(
        &mut self,
        draft: ExtrudeSpec,
        document: &Document,
    ) -> Result<(), ModelError> {
        validate_extrude(&draft, document.parameters())?;
        self.preview = Some(draft.clone());
        self.state = CommandState::Extrude {
            draft: Box::new(draft),
        };
        Ok(())
    }
    /// Accepts command-local evaluated parameters without publishing them to the document.
    ///
    /// # Errors
    /// Rejects invalid active quantities; exact geometry remains the worker's responsibility.
    pub fn edit_extrude_with_parameters(
        &mut self,
        draft: ExtrudeSpec,
        parameters: &parameters::ParameterRegistry,
    ) -> Result<(), ModelError> {
        validate_extrude(&draft, parameters)?;
        self.preview = Some(draft.clone());
        self.state = CommandState::Extrude {
            draft: Box::new(draft),
        };
        Ok(())
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn confirm(&mut self, document: &mut Document) -> Result<Option<FeatureId>, ModelError> {
        self.confirm_before(document, None)
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn confirm_before(
        &mut self,
        document: &mut Document,
        before: Option<FeatureId>,
    ) -> Result<Option<FeatureId>, ModelError> {
        let CommandState::Extrude { draft } = &self.state else {
            return Ok(None);
        };
        let feature = document.commit_extrude_before(draft.as_ref().clone(), before)?;
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
    /// Geometry finalization did not prove the frozen request and complete output inventory.
    #[error("invalid prepared geometry manifest")]
    InvalidGeometryManifest,
    /// A subshape is absent, stale, or incompatible with its exact supporting face.
    #[error("invalid or stale geometry reference")]
    InvalidGeometryReference,
    /// Typed parser/registry diagnostic with the preserved source span.
    #[error(transparent)]
    Formula(#[from] formula::FormulaError),
    /// A private parameter reservation belongs to an older accepted document state.
    #[error("parameter draft source has changed; start a fresh command session")]
    StaleParameterDraft,
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
    /// Text does not match the supported numeric literal grammar.
    #[error("invalid numeric literal: {0}")]
    InvalidLiteral(String),
    /// A literal uses a suffix outside the supported unit set.
    #[error("unsupported unit: {0}")]
    UnsupportedUnit(String),
    /// A typed value or recognized suffix has the wrong dimension, including at zero.
    #[error("quantity dimension does not match the destination")]
    DimensionMismatch,
    /// A quantity constructor or literal receives NaN or infinity.
    #[error("quantity must be finite")]
    NonFiniteQuantity,
    /// Parsing or unit conversion cannot produce a finite canonical/result value.
    #[error("quantity conversion overflowed")]
    QuantityOverflow,
    /// Nonzero source data would silently become zero during parsing or conversion.
    #[error("nonzero quantity underflowed to zero")]
    QuantityUnderflow,
    /// The existing expression evaluator does not support the requested non-length dimension.
    #[error("expressions are supported only for lengths")]
    UnsupportedExpression,
    /// A command field received a negative magnitude; orientation is separate data.
    #[error("negative magnitudes are not allowed")]
    NegativeMagnitude,
    /// A magnitude exceeds its command-field range, independently of display precision.
    #[error("quantity is outside the field range")]
    ParameterOutOfRange,
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
    parameters: &parameters::ParameterRegistry,
) -> Result<(), ModelError> {
    let spec = spec.inputs();
    if spec.termination == ExtrudeTermination::Blind {
        validate_active_length(&spec.depth, parameters, true)?;
    }
    if let Some(value) = &spec.starting_offset {
        validate_active_length(value, parameters, false)?;
    }
    if let Some(value) = &spec.end_offset {
        validate_active_length(value, parameters, false)?;
    }
    if let Some(value) = &spec.second_end {
        validate_active_length(value, parameters, true)?;
    }
    if let Some(value) = &spec.main_clearance {
        validate_active_length(value, parameters, false)?;
    }
    if let Some(value) = &spec.second_end_offset {
        validate_active_length(value, parameters, false)?;
    }
    if spec.termination == ExtrudeTermination::UpTo && spec.targets.main.is_none() {
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

fn validate_active_length(
    value: &ParameterValue,
    parameters: &parameters::ParameterRegistry,
    positive: bool,
) -> Result<(), ModelError> {
    let Quantity::Length(length) = value.evaluate_quantity(QuantityKind::Length, parameters)?
    else {
        return Err(ModelError::DimensionMismatch);
    };
    let n = length.as_millimetres();
    if n < 0.0 {
        return Err(ModelError::NegativeMagnitude);
    }
    if n > 99_999_999.9 {
        return Err(ModelError::ParameterOutOfRange);
    }
    if positive && n <= 0.0 {
        return Err(ModelError::NonPositiveDepth);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn draft() -> ExtrudeSpec {
        ExtrudeInputProjection {
            depth: ParameterValue::literal(Length::millimetres(10.0).unwrap()),
            ..ExtrudeInputProjection::default()
        }
        .into_spec()
        .unwrap()
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
                ExtrudeInputProjection {
                    profile: Some(unknown),
                    ..draft().inputs()
                }
                .into_spec()
                .unwrap(),
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
                ExtrudeInputProjection {
                    merge_scope: vec![later],
                    merge_with_all: false,
                    ..draft().inputs()
                }
                .into_spec()
                .unwrap(),
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
            let spec = ExtrudeInputProjection {
                profile,
                merge_scope,
                merge_with_all: false,
                ..draft().inputs()
            }
            .into_spec()
            .unwrap();
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
                ExtrudeInputProjection {
                    profile: Some(earlier),
                    merge_scope: vec![earlier, earlier],
                    merge_with_all: false,
                    ..draft().inputs()
                }
                .into_spec()
                .unwrap(),
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
        document
            .set_parameter("wall", Length::millimetres(2.5).unwrap())
            .unwrap();
        let mut command = CommandController::default();
        command.begin_extrude(&Selection::default());
        let candidate = document
            .parameters()
            .stage_edit(
                parameters::ParameterEdit {
                    parameter: None,
                    owner: parameters::ParameterOwner::Document,
                    source: "#wall + 7.5 mm".into(),
                    destination: QuantityKind::Length,
                    source_edit: None,
                    source_origins: None,
                },
                &Default::default(),
            )
            .unwrap();
        let parameter = candidate.parameter();
        document.parameter_registry = candidate.into_registry();
        let draft = ExtrudeInputProjection {
            depth: ParameterValue::Bound(parameter),
            ..ExtrudeInputProjection::default()
        }
        .into_spec()
        .unwrap();
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
                ExtrudeInputProjection {
                    depth: ParameterValue::literal(Length::millimetres(10.0).unwrap()),
                    ..ExtrudeInputProjection::default()
                }
                .into_spec()
                .unwrap(),
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
        let draft = ExtrudeInputProjection {
            result: ExtrudeResult::Add,
            merge_with_all: false,
            depth: ParameterValue::literal(Length::millimetres(10.0).unwrap()),
            ..ExtrudeInputProjection::default()
        }
        .into_spec()
        .unwrap();
        assert_eq!(
            command.edit_extrude(draft, &document),
            Err(ModelError::MissingMergeTarget)
        );
    }
}

#[cfg(test)]
mod quantity_parameter_tests {
    use super::*;
    #[test]
    fn literal_precision_and_all_extrude_destinations_validate_without_display_roundtrip() {
        let parameters = parameters::ParameterRegistry::default();
        for n in [1.234_567_89, 1.245_678_9] {
            let value = ParameterValue::literal(Length::millimetres(n).unwrap());
            assert_eq!(value.evaluate(&parameters).unwrap().as_millimetres(), n);
        }
        let good = ParameterValue::literal(Length::millimetres(12.3).unwrap());
        let zero = ParameterValue::literal(Length::ZERO);
        let mut spec = ExtrudeInputProjection {
            depth: good.clone(),
            starting_offset: Some(zero.clone()),
            end_offset: Some(zero.clone()),
            second_end: Some(good.clone()),
            second_end_offset: Some(zero.clone()),
            ..ExtrudeInputProjection::default()
        };
        assert!(validate_extrude(&spec.clone().into_spec().unwrap(), &parameters).is_ok());
        for bad in [
            ParameterValue::literal(Length::millimetres(-1.0).unwrap()),
            ParameterValue::Literal(Quantity::Angle(Angle::degrees(0.0).unwrap())),
            ParameterValue::Literal(Quantity::Scalar(Scalar::new(0.0).unwrap())),
            ParameterValue::literal(Length::millimetres(100_000_000.0).unwrap()),
        ] {
            for destination in 0..5 {
                let mut candidate = spec.clone();
                match destination {
                    0 => candidate.depth = bad.clone(),
                    1 => candidate.starting_offset = Some(bad.clone()),
                    2 => candidate.end_offset = Some(bad.clone()),
                    3 => candidate.second_end = Some(bad.clone()),
                    _ => {
                        candidate.second_end = None;
                        candidate.targets.second = Some(geometry::FaceRef {
                            entity: EntityId::from_raw(0),
                            generation: geometry::GeometryGeneration(1),
                            face: 0,
                        });
                        candidate.second_end_offset = Some(bad.clone());
                    }
                };
                assert!(
                    validate_extrude(&candidate.into_spec().unwrap(), &parameters).is_err(),
                    "destination {destination}"
                );
            }
        }
        spec.second_end = Some(zero.clone());
        assert_eq!(
            validate_extrude(&spec.clone().into_spec().unwrap(), &parameters),
            Err(ModelError::NonPositiveDepth)
        );
        spec.second_end = None;
        spec.depth = ParameterValue::Literal(Quantity::Angle(Angle::degrees(42.).unwrap()));
        spec.termination = ExtrudeTermination::ThroughAll;
        assert!(validate_extrude(&spec.clone().into_spec().unwrap(), &parameters).is_ok());
        spec.termination = ExtrudeTermination::Blind;
        spec.depth = good;
        spec.directions = ExtrudeDirections {
            depth: Direction::Reverse,
            starting_offset: Direction::Reverse,
            end_offset: Direction::Forward,
            second_end: Direction::Reverse,
        };
        assert!(validate_extrude(&spec.clone().into_spec().unwrap(), &parameters).is_ok());
        assert_eq!(
            spec.depth.evaluate(&parameters).unwrap().as_millimetres(),
            12.3
        );
    }
}
