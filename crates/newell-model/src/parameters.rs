// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Private parameter candidates with stable names, ownership and deterministic DAG evaluation.
use crate::{
    FeatureId, Quantity, QuantityKind,
    formula::{self, FormulaError, FormulaErrorCode, ParameterId, ParsedExpression, SourceSpan},
};
use std::collections::{BTreeMap, BTreeSet};

/// A referenceable case-sensitive ASCII name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParameterName(String);
impl ParameterName {
    /// Validates the approved identifier grammar.
    ///
    /// # Errors
    /// Rejects an invalid or overlong identifier.
    pub fn new(name: String) -> Result<Self, FormulaError> {
        if !formula::valid_name(&name) {
            return Err(failure(FormulaErrorCode::Syntax));
        }
        Ok(Self(name))
    }
    /// Returns the spelling, independent of identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
/// Ownership constrains references, but never introduces a dependency edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterOwner {
    Document,
    Feature(FeatureId),
}
/// One accepted definition and its independently preserved source/AST/value.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterDefinition {
    pub id: ParameterId,
    pub name: Option<ParameterName>,
    pub owner: ParameterOwner,
    pub source: String,
    pub parsed: ParsedExpression,
    pub value: Quantity,
}
/// Atomic edit of one field's definition. An existing identity keeps its owner.
#[derive(Debug, Clone)]
pub struct ParameterEdit {
    pub parameter: Option<ParameterId>,
    pub owner: ParameterOwner,
    pub source: String,
    pub destination: QuantityKind,
    pub source_edit: Option<formula::FormulaSourceEdit>,
    pub source_origins: Option<Vec<Option<usize>>>,
}
/// Committed definitions. All mutations first produce a private candidate.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParameterRegistry {
    definitions: BTreeMap<ParameterId, ParameterDefinition>,
    names: BTreeMap<String, ParameterId>,
    next_id: u64,
    epoch: u64,
}
/// Evaluated candidate; the source registry is unchanged on every failure.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterCandidate {
    registry: ParameterRegistry,
    parameter: ParameterId,
}
impl ParameterCandidate {
    /// Registry ready for its owning document transaction; this does not publish it.
    #[must_use]
    pub fn registry(&self) -> &ParameterRegistry {
        &self.registry
    }
    /// Destination identity reserved in this private candidate.
    #[must_use]
    pub const fn parameter(&self) -> ParameterId {
        self.parameter
    }
    /// Consumes the candidate into a still-private document draft.
    #[must_use]
    pub fn into_registry(self) -> ParameterRegistry {
        self.registry
    }
}
fn failure(code: FormulaErrorCode) -> FormulaError {
    FormulaError {
        code,
        span: SourceSpan { start: 0, end: 0 },
    }
}
impl ParameterRegistry {
    /// Definition by stable identity.
    #[must_use]
    pub fn definition(&self, id: ParameterId) -> Option<&ParameterDefinition> {
        self.definitions.get(&id)
    }
    /// Current name index for newly entered tokens only.
    #[must_use]
    pub fn names(&self) -> &BTreeMap<String, ParameterId> {
        &self.names
    }
    /// Monotonic accepted candidate generation.
    #[must_use]
    pub const fn epoch(&self) -> u64 {
        self.epoch
    }
    /// Checks a feature's active input owners independently from parameter-to-parameter edges.
    ///
    /// # Errors
    /// Allows intrinsic inputs owned by this feature, rejects external owners at/after it.
    pub fn validate_feature_inputs(
        &self,
        feature: FeatureId,
        position: usize,
        inputs: &BTreeSet<ParameterId>,
        order: &BTreeMap<FeatureId, usize>,
    ) -> Result<(), FormulaError> {
        for input in inputs {
            let definition = self
                .definition(*input)
                .ok_or_else(|| failure(FormulaErrorCode::MissingParameter))?;
            match definition.owner {
                ParameterOwner::Document => {}
                ParameterOwner::Feature(owner) if owner == feature => {}
                ParameterOwner::Feature(owner)
                    if order.get(&owner).is_some_and(|owner| *owner < position) => {}
                _ => return Err(failure(FormulaErrorCode::ForwardDependency)),
            }
        }
        Ok(())
    }
    /// Produces a fully evaluated candidate; feature positions include the private owner reservation.
    ///
    /// # Errors
    /// Rejects names, limits, cycles, missing ids and dependencies at/after an external owner.
    pub fn stage_edit(
        &self,
        edit: ParameterEdit,
        order: &BTreeMap<FeatureId, usize>,
    ) -> Result<ParameterCandidate, FormulaError> {
        self.stage_edit_roots(edit, order, None)
    }
    fn stage_edit_roots(
        &self,
        edit: ParameterEdit,
        order: &BTreeMap<FeatureId, usize>,
        roots: Option<&BTreeSet<ParameterId>>,
    ) -> Result<ParameterCandidate, FormulaError> {
        let mut candidate = self.clone();
        let (header, _) = formula::definition_header(&edit.source)?;
        let named_target = header
            .as_ref()
            .and_then(|name| candidate.names.get(name).copied());
        let existing = if edit
            .parameter
            .and_then(|id| candidate.definition(id))
            .is_some_and(|definition| definition.name.is_none())
            && named_target.is_some()
        {
            named_target
        } else {
            edit.parameter.or(named_target)
        };
        let id = if let Some(id) = existing {
            if !candidate.definitions.contains_key(&id) {
                return Err(failure(FormulaErrorCode::MissingParameter));
            }
            id
        } else {
            if candidate.definitions.len() >= 1024 {
                return Err(failure(FormulaErrorCode::TooComplex));
            }
            let id = ParameterId(candidate.next_id);
            candidate.next_id = candidate
                .next_id
                .checked_add(1)
                .ok_or_else(|| failure(FormulaErrorCode::TooComplex))?;
            id
        };
        let old = candidate.definitions.get(&id);
        let owner = old.map_or(edit.owner, |d| d.owner);
        let old_name = old.and_then(|d| d.name.clone());
        let header_changed =
            old.is_none_or(|definition| definition.parsed.name() != header.as_deref());
        let name = if header_changed {
            header
                .map(ParameterName::new)
                .transpose()?
                .or(old_name.clone())
        } else {
            old_name.clone()
        };
        if let Some(name) = &name {
            if candidate
                .names
                .get(name.as_str())
                .is_some_and(|other| *other != id)
            {
                return Err(failure(FormulaErrorCode::Syntax));
            }
            if let Some(old) = old_name {
                candidate.names.remove(old.as_str());
            }
            candidate.names.insert(name.as_str().to_owned(), id);
        }
        // Bind self-reference to the reserved identity so it reports Cycle, never UnknownName.
        let parsed = if let (Some(old), Some(origins)) = (old, edit.source_origins.as_ref()) {
            formula::reparse_with_origins(&old.parsed, &edit.source, origins, &candidate.names)?
        } else if let (Some(old), Some(source_edit)) = (old, edit.source_edit.as_ref()) {
            let parsed = formula::edit_expression(&old.parsed, source_edit, &candidate.names)?;
            if parsed.source() != edit.source {
                return Err(failure(FormulaErrorCode::Syntax));
            }
            parsed
        } else if let Some(old) = old
            .filter(|old| old.source == edit.source && old.parsed.destination() == edit.destination)
        {
            old.parsed.clone()
        } else {
            formula::parse_expression(&edit.source, edit.destination, &candidate.names)?
        };
        if parsed.destination() != edit.destination {
            return Err(failure(FormulaErrorCode::Dimension));
        }
        // The cached placeholder is never observable: reevaluation must succeed before return.
        let value = crate::Scalar::new(0.)
            .map(Quantity::Scalar)
            .map_err(|_| failure(FormulaErrorCode::NonFinite))?;
        candidate.definitions.insert(
            id,
            ParameterDefinition {
                id,
                name,
                owner,
                source: edit.source,
                parsed,
                value,
            },
        );
        candidate.reevaluate(
            order,
            roots.map(|roots| {
                let mut roots = roots.clone();
                roots.insert(id);
                roots
            }),
        )?;
        if candidate.definitions != self.definitions || candidate.names != self.names {
            candidate.epoch = candidate
                .epoch
                .checked_add(1)
                .ok_or_else(|| failure(FormulaErrorCode::TooComplex))?;
        }
        Ok(ParameterCandidate {
            registry: candidate,
            parameter: id,
        })
    }
    /// Replaces an existing definition with an exact typed literal, retaining id/name/owner.
    ///
    /// # Errors
    /// Rejects missing ids or any dependent definition made invalid by the change.
    pub fn stage_literal(
        &self,
        id: ParameterId,
        value: Quantity,
        order: &BTreeMap<FeatureId, usize>,
    ) -> Result<ParameterCandidate, FormulaError> {
        self.stage_literal_roots(id, value, order, None)
    }
    fn stage_literal_roots(
        &self,
        id: ParameterId,
        value: Quantity,
        order: &BTreeMap<FeatureId, usize>,
        roots: Option<&BTreeSet<ParameterId>>,
    ) -> Result<ParameterCandidate, FormulaError> {
        let mut candidate = self.clone();
        let definition = candidate
            .definitions
            .get_mut(&id)
            .ok_or_else(|| failure(FormulaErrorCode::MissingParameter))?;
        definition.parsed = formula::literal_expression(value);
        definition.source = definition.parsed.source().to_owned();
        candidate.reevaluate(
            order,
            roots.map(|roots| {
                let mut roots = roots.clone();
                roots.insert(id);
                roots
            }),
        )?;
        if candidate.definitions != self.definitions || candidate.names != self.names {
            candidate.epoch = candidate
                .epoch
                .checked_add(1)
                .ok_or_else(|| failure(FormulaErrorCode::TooComplex))?;
        }
        Ok(ParameterCandidate {
            registry: candidate,
            parameter: id,
        })
    }
    /// Renames only the name index; accepted reference tokens retain their identities and source.
    ///
    /// # Errors
    /// Rejects unknown identities, colliding names and exhausted epochs.
    pub fn rename(
        &self,
        id: ParameterId,
        name: ParameterName,
    ) -> Result<ParameterCandidate, FormulaError> {
        if self
            .names
            .get(name.as_str())
            .is_some_and(|other| *other != id)
        {
            return Err(failure(FormulaErrorCode::Syntax));
        }
        let mut candidate = self.clone();
        let definition = candidate
            .definitions
            .get_mut(&id)
            .ok_or_else(|| failure(FormulaErrorCode::MissingParameter))?;
        if let Some(old) = &definition.name {
            candidate.names.remove(old.as_str());
        }
        candidate.names.insert(name.as_str().to_owned(), id);
        definition.name = Some(name);
        if candidate.definitions != self.definitions || candidate.names != self.names {
            candidate.epoch = candidate
                .epoch
                .checked_add(1)
                .ok_or_else(|| failure(FormulaErrorCode::TooComplex))?;
        }
        Ok(ParameterCandidate {
            registry: candidate,
            parameter: id,
        })
    }
    fn reachable(
        &self,
        mut roots: BTreeSet<ParameterId>,
    ) -> Result<BTreeSet<ParameterId>, FormulaError> {
        roots.extend(self.names.values().copied());
        let mut pending: Vec<_> = roots.iter().copied().collect();
        while let Some(id) = pending.pop() {
            let definition = self
                .definition(id)
                .ok_or_else(|| failure(FormulaErrorCode::MissingParameter))?;
            for reference in definition.parsed.references() {
                if roots.insert(reference.parameter) {
                    pending.push(reference.parameter);
                }
            }
        }
        Ok(roots)
    }
    fn reevaluate(
        &mut self,
        order: &BTreeMap<FeatureId, usize>,
        roots: Option<BTreeSet<ParameterId>>,
    ) -> Result<(), FormulaError> {
        let active = match roots {
            Some(roots) => self.reachable(roots)?,
            None => self.definitions.keys().copied().collect(),
        };
        let mut counts = BTreeMap::new();
        let mut consumers: BTreeMap<ParameterId, Vec<ParameterId>> = BTreeMap::new();
        let mut edge_count = 0usize;
        for (&id, definition) in &self.definitions {
            if !active.contains(&id) {
                continue;
            }
            if let ParameterOwner::Feature(owner) = definition.owner
                && !order.contains_key(&owner)
            {
                return Err(failure(FormulaErrorCode::ForwardDependency));
            }
            let dependencies: BTreeSet<_> = definition
                .parsed
                .references()
                .iter()
                .map(|r| r.parameter)
                .collect();
            edge_count = edge_count
                .checked_add(dependencies.len())
                .ok_or_else(|| failure(FormulaErrorCode::TooComplex))?;
            if edge_count > 8192 {
                return Err(failure(FormulaErrorCode::TooComplex));
            }
            for &dependency in &dependencies {
                let producer = self
                    .definitions
                    .get(&dependency)
                    .ok_or_else(|| failure(FormulaErrorCode::MissingParameter))?;
                match (definition.owner, producer.owner) {
                    (_, ParameterOwner::Document) => {}
                    (ParameterOwner::Feature(a), ParameterOwner::Feature(b)) if a == b => {}
                    (ParameterOwner::Feature(a), ParameterOwner::Feature(b))
                        if order.get(&b).zip(order.get(&a)).is_some_and(|(b, a)| b < a) => {}
                    _ => return Err(failure(FormulaErrorCode::ForwardDependency)),
                }
                consumers.entry(dependency).or_default().push(id);
            }
            counts.insert(id, dependencies.len());
        }
        let mut ready: BTreeSet<_> = counts
            .iter()
            .filter_map(|(&id, &count)| (count == 0).then_some(id))
            .collect();
        let mut evaluated = BTreeMap::new();
        while let Some(id) = ready.pop_first() {
            let definition = self
                .definitions
                .get(&id)
                .ok_or_else(|| failure(FormulaErrorCode::MissingParameter))?;
            let value = definition
                .parsed
                .evaluate(|dependency| evaluated.get(&dependency).copied())?;
            evaluated.insert(id, value);
            if let Some(list) = consumers.get(&id) {
                for consumer in list {
                    let count = counts
                        .get_mut(consumer)
                        .ok_or_else(|| failure(FormulaErrorCode::MissingParameter))?;
                    *count = count
                        .checked_sub(1)
                        .ok_or_else(|| failure(FormulaErrorCode::Cycle))?;
                    if *count == 0 {
                        ready.insert(*consumer);
                    }
                }
            }
        }
        if evaluated.len() != active.len() {
            return Err(failure(FormulaErrorCode::Cycle));
        }
        for (id, value) in evaluated {
            self.definitions
                .get_mut(&id)
                .ok_or_else(|| failure(FormulaErrorCode::MissingParameter))?
                .value = value;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Length;
    fn edit(source: &str, owner: ParameterOwner) -> ParameterEdit {
        ParameterEdit {
            parameter: None,
            owner,
            source: source.into(),
            destination: QuantityKind::Length,
            source_edit: None,
            source_origins: None,
        }
    }
    #[test]
    fn unchanged_definitions_keep_the_semantic_epoch() -> Result<(), FormulaError> {
        let order = BTreeMap::new();
        let candidate = ParameterRegistry::default()
            .stage_edit(edit("#D := 12mm", ParameterOwner::Document), &order)?;
        let id = candidate.parameter();
        let registry = candidate.into_registry();
        let mut same = edit("#D := 12mm", ParameterOwner::Document);
        same.parameter = Some(id);
        assert_eq!(registry.stage_edit(same, &order)?.registry(), &registry);
        assert_eq!(
            registry
                .rename(id, ParameterName::new("D".into())?)?
                .registry(),
            &registry
        );
        Ok(())
    }
    #[test]
    fn local_ownership_is_not_an_edge_and_failures_are_atomic() -> Result<(), FormulaError> {
        let owner = ParameterOwner::Feature(FeatureId(5));
        let order = BTreeMap::from([(FeatureId(5), 1), (FeatureId(6), 2)]);
        let original = ParameterRegistry::default();
        let depth = original.stage_edit(edit("#D := 12mm", owner), &order)?;
        assert!(original.definitions.is_empty());
        assert_eq!(original.next_id, 0);
        let d = depth.parameter();
        let registry = depth.into_registry();
        let offset = registry.stage_edit(edit("#Offset := #D/2", owner), &order)?;
        assert_eq!(
            offset
                .registry()
                .definition(offset.parameter())
                .map(|d| d.value),
            Some(Quantity::Length(
                Length::millimetres(6.).map_err(|_| failure(FormulaErrorCode::NonFinite))?
            ))
        );
        let committed = offset.into_registry();
        let mut cycle = edit("#D := #Offset", owner);
        cycle.parameter = Some(d);
        assert_eq!(
            committed.stage_edit(cycle, &order).map_err(|e| e.code),
            Err(FormulaErrorCode::Cycle)
        );
        assert_eq!(committed.epoch(), 2);
        assert_eq!(
            original
                .stage_edit(edit("#A := #A+1mm", owner), &order)
                .map_err(|e| e.code),
            Err(FormulaErrorCode::Cycle)
        );
        Ok(())
    }
    #[test]
    fn rename_reuse_and_owner_ordering() -> Result<(), FormulaError> {
        let order = BTreeMap::from([(FeatureId(1), 0), (FeatureId(2), 1)]);
        let doc = ParameterOwner::Document;
        let earlier = ParameterOwner::Feature(FeatureId(1));
        let later = ParameterOwner::Feature(FeatureId(2));
        let first = ParameterRegistry::default().stage_edit(edit("#A := 12mm", earlier), &order)?;
        let a = first.parameter();
        let second = first
            .into_registry()
            .stage_edit(edit("#B := #A/2", later), &order)?;
        let b = second.parameter();
        let registry = second
            .into_registry()
            .rename(a, ParameterName::new("Renamed".into())?)?
            .into_registry();
        let registry = registry
            .stage_edit(edit("#A := 90mm", doc), &order)?
            .into_registry();
        let unchanged = registry
            .definition(b)
            .ok_or_else(|| failure(FormulaErrorCode::MissingParameter))?
            .source
            .clone();
        let reaccept = registry.stage_edit(
            ParameterEdit {
                parameter: Some(b),
                ..edit(&unchanged, later)
            },
            &order,
        )?;
        assert_eq!(
            reaccept
                .registry()
                .definition(b)
                .map(|d| d.parsed.references()[0].parameter),
            Some(a)
        );
        assert_eq!(
            registry
                .stage_edit(
                    ParameterEdit {
                        parameter: Some(a),
                        ..edit("#B", later)
                    },
                    &order
                )
                .map_err(|e| e.code),
            Err(FormulaErrorCode::ForwardDependency)
        );
        assert_eq!(
            registry
                .stage_edit(edit("#X := #B", doc), &order)
                .map_err(|e| e.code),
            Err(FormulaErrorCode::ForwardDependency)
        );
        Ok(())
    }
}

/// Owning transaction and captured insertion boundary of a parameter draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterDraftOwner {
    Document,
    InsertExtrude { before: Option<FeatureId> },
    EditExtrude(FeatureId),
}
/// Command-local definitions and reservation. Nothing here mutates the source document.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterDraft {
    source: Box<crate::Document>,
    registry: ParameterRegistry,
    owner: ParameterDraftOwner,
    feature: Option<FeatureId>,
    order: BTreeMap<FeatureId, usize>,
}
impl ParameterDraft {
    pub(crate) const fn owner(&self) -> ParameterDraftOwner {
        self.owner
    }
    /// Privately reserved insertion owner or existing edited feature.
    #[must_use]
    pub const fn feature_owner(&self) -> Option<FeatureId> {
        self.feature
    }
    /// Currently accepted private definitions.
    #[must_use]
    pub fn registry(&self) -> &ParameterRegistry {
        &self.registry
    }
    /// Stages one accepted field edit atomically within this command.
    ///
    /// # Errors
    /// Leaves the draft unchanged on any parse, dependency or ownership failure.
    pub fn stage_edit(&mut self, mut edit: ParameterEdit) -> Result<ParameterId, FormulaError> {
        edit.owner = self
            .feature
            .map_or(ParameterOwner::Document, ParameterOwner::Feature);
        let candidate = self.registry.stage_edit(edit, &self.order)?;
        let id = candidate.parameter();
        self.registry = candidate.into_registry();
        Ok(id)
    }
    /// Stages a field edit with only active anonymous consumers and all named/committed definitions.
    ///
    /// # Errors
    /// Rejects reachable dependency failures atomically, retaining disabled anonymous drafts.
    pub fn stage_edit_active(
        &mut self,
        mut edit: ParameterEdit,
        active: &BTreeSet<ParameterId>,
    ) -> Result<ParameterId, FormulaError> {
        edit.owner = self
            .feature
            .map_or(ParameterOwner::Document, ParameterOwner::Feature);
        let mut roots = active.clone();
        roots.extend(self.source.parameter_registry.definitions.keys().copied());
        let candidate = self
            .registry
            .stage_edit_roots(edit, &self.order, Some(&roots))?;
        let id = candidate.parameter();
        self.registry = candidate.into_registry();
        Ok(id)
    }
    /// Accepts a literal with the same active-consumer rules as Formula.
    ///
    /// # Errors
    /// Rejects reachable failures without changing the draft.
    pub fn stage_literal_active(
        &mut self,
        id: ParameterId,
        value: Quantity,
        active: &BTreeSet<ParameterId>,
    ) -> Result<(), FormulaError> {
        let mut roots = active.clone();
        roots.extend(self.source.parameter_registry.definitions.keys().copied());
        self.registry = self
            .registry
            .stage_literal_roots(id, value, &self.order, Some(&roots))?
            .into_registry();
        Ok(())
    }
    /// Produces only reachable active/named definitions for the eventual atomic document transaction.
    ///
    /// # Errors
    /// Revalidates all reachable definitions and owners before publication.
    pub fn prepared_registry(
        &self,
        active: &BTreeSet<ParameterId>,
    ) -> Result<ParameterRegistry, FormulaError> {
        let mut roots = active.clone();
        roots.extend(self.source.parameter_registry.definitions.keys().copied());
        let reachable = self.registry.reachable(roots)?;
        let mut candidate = self.registry.clone();
        candidate.definitions.retain(|id, _| reachable.contains(id));
        candidate.reevaluate(&self.order, None)?;
        if candidate.definitions == self.source.parameter_registry.definitions {
            return Ok(self.source.parameter_registry.clone());
        }
        let retained_next = candidate
            .definitions
            .keys()
            .next_back()
            .map(|id| {
                id.0.checked_add(1)
                    .ok_or_else(|| failure(FormulaErrorCode::TooComplex))
            })
            .transpose()?
            .unwrap_or(0);
        candidate.next_id = retained_next.max(self.source.parameter_registry.next_id);
        Ok(candidate)
    }
    /// Accepts a literal replacement without round-tripping canonical data through display text.
    ///
    /// # Errors
    /// Rejects missing definitions and invalidated dependents atomically.
    pub fn stage_literal(&mut self, id: ParameterId, value: Quantity) -> Result<(), FormulaError> {
        self.registry = self
            .registry
            .stage_literal(id, value, &self.order)?
            .into_registry();
        Ok(())
    }
    /// Ensures a reservation still belongs to exactly the accepted source state.
    ///
    /// # Errors
    /// A stale source cannot reuse a reservation; the app must begin a fresh command session.
    pub fn validate_source(&self, source: &crate::Document) -> Result<(), crate::ModelError> {
        if self.source.as_ref() != source {
            return Err(crate::ModelError::StaleParameterDraft);
        }
        Ok(())
    }
}
impl crate::Document {
    /// Committed stable definitions; callers can only create private edit candidates.
    #[must_use]
    pub fn parameter_registry(&self) -> &ParameterRegistry {
        &self.parameter_registry
    }
    /// Captures a source document and reserves an owner without consuming any committed id.
    ///
    /// # Errors
    /// Rejects missing/non-Extrude edit targets, missing insertion anchors and identity exhaustion.
    pub fn begin_parameter_draft(
        &self,
        owner: ParameterDraftOwner,
    ) -> Result<ParameterDraft, crate::ModelError> {
        let mut order: BTreeMap<_, _> = self
            .features
            .iter()
            .enumerate()
            .map(|(i, (id, _))| (*id, i))
            .collect();
        let feature = match owner {
            ParameterDraftOwner::Document => None,
            ParameterDraftOwner::InsertExtrude { before } => {
                let position = match before {
                    Some(id) => *order
                        .get(&id)
                        .ok_or(crate::ModelError::StaleParameterDraft)?,
                    None => self.features.len(),
                };
                self.next_feature
                    .checked_add(1)
                    .ok_or(crate::ModelError::IdentityExhausted)?;
                for value in order.values_mut() {
                    if *value >= position {
                        *value = value
                            .checked_add(1)
                            .ok_or(crate::ModelError::IdentityExhausted)?;
                    }
                }
                let reserved = FeatureId(self.next_feature);
                order.insert(reserved, position);
                Some(reserved)
            }
            ParameterDraftOwner::EditExtrude(id) => {
                if !matches!(self.feature(id), Some(crate::Feature::Extrude(_))) {
                    return Err(crate::ModelError::StaleParameterDraft);
                }
                Some(id)
            }
        };
        Ok(ParameterDraft {
            source: Box::new(self.clone()),
            registry: self.parameter_registry.clone(),
            owner,
            feature,
            order,
        })
    }
}

#[cfg(test)]
mod draft_tests {
    use super::*;
    #[test]
    fn reservations_are_private_reusable_and_source_bound() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut document = crate::Document::default();
        document.add_base_solid("Base")?;
        let snapshot = document.clone();
        let next = document.next_feature;
        let mut draft =
            document.begin_parameter_draft(ParameterDraftOwner::InsertExtrude { before: None })?;
        assert_eq!(draft.feature_owner(), Some(FeatureId(next)));
        let id = draft.stage_edit(ParameterEdit {
            parameter: None,
            owner: ParameterOwner::Document,
            source: "#Depth := 12mm".into(),
            destination: QuantityKind::Length,
            source_edit: None,
            source_origins: None,
        })?;
        assert_eq!(
            draft.registry().definition(id).map(|d| d.owner),
            Some(ParameterOwner::Feature(FeatureId(next)))
        );
        assert_eq!(document, snapshot);
        draft.validate_source(&document)?;
        drop(draft);
        assert_eq!(
            document
                .begin_parameter_draft(ParameterDraftOwner::InsertExtrude { before: None })?
                .feature_owner(),
            Some(FeatureId(next))
        );
        let draft =
            document.begin_parameter_draft(ParameterDraftOwner::InsertExtrude { before: None })?;
        document.add_base_solid("Changed")?;
        assert_eq!(
            draft.validate_source(&document),
            Err(crate::ModelError::StaleParameterDraft)
        );
        Ok(())
    }
}

#[cfg(test)]
mod identity_tests {
    use super::*;
    fn edit(source: &str, id: Option<ParameterId>) -> ParameterEdit {
        ParameterEdit {
            parameter: id,
            owner: ParameterOwner::Document,
            source: source.into(),
            destination: QuantityKind::Length,
            source_edit: None,
            source_origins: None,
        }
    }
    #[test]
    fn unchanged_old_header_does_not_undo_a_rename_or_capture_a_reused_name()
    -> Result<(), FormulaError> {
        let order = BTreeMap::new();
        let first = ParameterRegistry::default().stage_edit(edit("#A := 12mm", None), &order)?;
        let a = first.parameter();
        let registry = first
            .into_registry()
            .rename(a, ParameterName::new("B".into())?)?
            .into_registry();
        let registry = registry
            .stage_edit(edit("#A := 90mm", None), &order)?
            .into_registry();
        let candidate = registry.stage_edit(edit("#A := 12mm", Some(a)), &order)?;
        assert_eq!(
            candidate
                .registry()
                .definition(a)
                .and_then(|d| d.name.as_ref())
                .map(ParameterName::as_str),
            Some("B")
        );
        assert_eq!(candidate.registry().names().get("B"), Some(&a));
        Ok(())
    }
    #[test]
    fn literal_replacement_keeps_canonical_bits_and_updates_dependents()
    -> Result<(), Box<dyn std::error::Error>> {
        let order = BTreeMap::new();
        let first = ParameterRegistry::default().stage_edit(edit("#A := 12mm", None), &order)?;
        let a = first.parameter();
        let second = first
            .into_registry()
            .stage_edit(edit("#B := #A/2", None), &order)?;
        let b = second.parameter();
        let exact = 1.23456789;
        let registry = second
            .into_registry()
            .stage_literal(
                a,
                Quantity::Length(crate::Length::millimetres(exact)?),
                &order,
            )?
            .into_registry();
        assert_eq!(
            registry.definition(a).map(|d| d.value),
            Some(Quantity::Length(crate::Length::millimetres(exact)?))
        );
        assert_eq!(
            registry.definition(b).map(|d| d.value),
            Some(Quantity::Length(crate::Length::millimetres(exact / 2.)?))
        );
        assert_eq!(
            registry
                .definition(a)
                .and_then(|d| d.name.as_ref())
                .map(ParameterName::as_str),
            Some("A")
        );
        Ok(())
    }
}

#[cfg(test)]
mod reachability_tests {
    use super::*;
    fn edit(source: &str, id: Option<ParameterId>) -> ParameterEdit {
        ParameterEdit {
            parameter: id,
            owner: ParameterOwner::Document,
            source: source.into(),
            destination: QuantityKind::Length,
            source_edit: None,
            source_origins: None,
        }
    }
    #[test]
    fn disabled_anonymous_consumers_do_not_block_named_changes_or_get_published()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = crate::Document::default();
        let mut draft =
            source.begin_parameter_draft(ParameterDraftOwner::InsertExtrude { before: None })?;
        let d = draft.stage_edit(edit("#D := 12mm", None))?;
        let hidden = draft.stage_edit(edit("1mm/(#D/1mm)", None))?;
        draft.stage_edit_active(edit("#D := 0mm", Some(d)), &BTreeSet::new())?;
        let published = draft.prepared_registry(&BTreeSet::new())?;
        assert!(published.definition(d).is_some());
        assert!(published.definition(hidden).is_none());
        assert_eq!(
            draft
                .prepared_registry(&BTreeSet::from([hidden]))
                .map_err(|e| e.code),
            Err(FormulaErrorCode::DivideByZero)
        );
        assert_eq!(source.parameter_registry, ParameterRegistry::default());
        Ok(())
    }
    #[test]
    fn an_unpublished_anonymous_only_panel_consumes_no_document_counter()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = crate::Document::default();
        let mut draft = source.begin_parameter_draft(ParameterDraftOwner::Document)?;
        draft.stage_edit(edit("12mm", None))?;
        assert_eq!(
            draft.prepared_registry(&BTreeSet::new())?,
            ParameterRegistry::default()
        );
        Ok(())
    }
}
