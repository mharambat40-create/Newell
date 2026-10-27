// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Application-owned validated drafts for command numeric fields.

use std::collections::BTreeMap;

use newell_model::{Direction, Quantity, Unit};
use newell_ui::{
    ActionOperation, NumericDirection, NumericEdit, NumericFieldDescriptor, NumericFieldKey,
    NumericFieldView, NumericProjection, NumericRequest, NumericUnit,
};

use crate::{AppError, document::DocumentInstanceId};
use newell_ui::CommandInputSessionId;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NumericDraftValue {
    pub quantity: Quantity,
    pub direction: Direction,
    pub formula: String,
}
#[derive(Debug, Clone)]
pub(crate) struct CommandNumericInputs {
    pub document: DocumentInstanceId,
    pub session: CommandInputSessionId,
    pub operation: ActionOperation,
    pub consumed_sequence: u64,
    pub values: BTreeMap<NumericFieldKey, NumericDraftValue>,
    pub descriptors: BTreeMap<NumericFieldKey, NumericFieldDescriptor>,
    pub parameter_draft: newell_model::parameters::ParameterDraft,
    pub bindings: BTreeMap<NumericFieldKey, newell_model::formula::ParameterId>,
    pub active_fields: std::collections::BTreeSet<NumericFieldKey>,
}
impl CommandNumericInputs {
    pub fn new(
        document: DocumentInstanceId,
        session: CommandInputSessionId,
        operation: ActionOperation,
        descriptors: &[NumericFieldDescriptor],
    ) -> Result<Self, AppError> {
        let mut values = BTreeMap::new();
        for d in descriptors
            .iter()
            .filter(|d| d.field.operation == operation)
        {
            let unit = model_unit(d.unit);
            let quantity = Quantity::parse_literal(d.default_magnitude, unit)?;
            if values
                .insert(
                    d.field,
                    NumericDraftValue {
                        quantity,
                        direction: Direction::Forward,
                        formula: String::new(),
                    },
                )
                .is_some()
            {
                return Err(AppError::InvalidNumericRequest);
            }
        }
        Ok(Self {
            document,
            session,
            operation,
            consumed_sequence: 0,
            values,
            descriptors: descriptors.iter().map(|d| (d.field, *d)).collect(),
            parameter_draft: newell_model::Document::default()
                .begin_parameter_draft(newell_model::parameters::ParameterDraftOwner::Document)?,
            bindings: BTreeMap::new(),
            active_fields: descriptors
                .iter()
                .map(|descriptor| descriptor.field)
                .collect(),
        })
    }
    pub fn consume_sequence(&mut self, request: &NumericRequest) -> bool {
        if request.destination.document != self.document.tree()
            || request.destination.session != self.session
            || request.destination.field.operation != self.operation
            || request.sequence <= self.consumed_sequence
        {
            return false;
        }
        self.consumed_sequence = request.sequence;
        true
    }
    pub fn apply(
        &mut self,
        request: NumericRequest,
        descriptor: NumericFieldDescriptor,
    ) -> Result<bool, AppError> {
        if !self.consume_sequence(&request) {
            return Err(AppError::InvalidNumericRequest);
        }
        if request.destination.field != descriptor.field
            || self.descriptors.get(&descriptor.field) != Some(&descriptor)
        {
            return Err(AppError::InvalidNumericRequest);
        }
        let literal_replacement = matches!(&request.edit, NumericEdit::Literal { .. });
        let old = self
            .values
            .get(&descriptor.field)
            .ok_or(AppError::InvalidNumericRequest)?;
        let mut next = old.clone();
        match request.edit {
            NumericEdit::Literal {
                magnitude,
                direction,
            } => {
                let quantity = Quantity::parse_literal(&magnitude, model_unit(descriptor.unit))?;
                let value = quantity.value_in(model_unit(descriptor.unit))?;
                if value < 0.0 {
                    return Err(newell_model::ModelError::NegativeMagnitude.into());
                }
                let max = if descriptor.unit == NumericUnit::Degrees {
                    9_999_999.9
                } else {
                    99_999_999.9
                };
                if value > max {
                    return Err(newell_model::ModelError::ParameterOutOfRange.into());
                }
                if !descriptor.directional && direction == NumericDirection::Reverse {
                    return Err(AppError::InvalidNumericRequest);
                }
                next.quantity = quantity;
                next.direction = model_direction(direction);
                next.formula.clear();
            }
            NumericEdit::Direction(direction) => {
                if !descriptor.directional && direction == NumericDirection::Reverse {
                    return Err(AppError::InvalidNumericRequest);
                }
                next.direction = model_direction(direction);
            }
            NumericEdit::Formula(_) => return Err(AppError::InvalidNumericRequest),
            NumericEdit::Incomplete(raw) => {
                return Err(newell_model::ModelError::InvalidLiteral(raw).into());
            }
        }
        let changed = &next != old;
        if literal_replacement && let Some(id) = self.bindings.get(&descriptor.field).copied() {
            let mut parameters = self.parameter_draft.clone();
            let roots = self.active_parameters();
            parameters.stage_literal_active(id, next.quantity, &roots)?;
            self.parameter_draft = parameters;
            for (field, id) in &self.bindings {
                if *field != descriptor.field {
                    let value = self
                        .parameter_draft
                        .registry()
                        .definition(*id)
                        .ok_or(AppError::InvalidNumericRequest)?
                        .value;
                    self.values
                        .get_mut(field)
                        .ok_or(AppError::InvalidNumericRequest)?
                        .quantity = value;
                }
            }
        }
        if changed {
            self.values.insert(descriptor.field, next);
        }
        Ok(changed)
    }
    pub fn active_parameters(
        &self,
    ) -> std::collections::BTreeSet<newell_model::formula::ParameterId> {
        self.active_fields
            .iter()
            .filter_map(|field| self.bindings.get(field).copied())
            .collect()
    }
    pub fn apply_formula(
        &mut self,
        request: &newell_ui::FormulaApplyRequest,
    ) -> Result<(), AppError> {
        if request.destination.document != self.document.tree()
            || request.destination.session != self.session
            || request.destination.field.operation != self.operation
            || request.through_sequence > self.consumed_sequence
            || request.attempt.0 <= self.consumed_sequence
        {
            return Err(AppError::InvalidNumericRequest);
        }
        self.consumed_sequence = request.attempt.0;
        let field = request.destination.field;
        let old = self
            .values
            .get(&field)
            .ok_or(AppError::InvalidNumericRequest)?;
        let mut candidate = self.clone();
        if request.source.trim().is_empty() {
            if let Some(id) = self.bindings.get(&field).copied() {
                let roots = self.active_parameters();
                candidate
                    .parameter_draft
                    .stage_literal_active(id, old.quantity, &roots)?;
            }
        } else {
            let roots = self.active_parameters();
            let id = candidate.parameter_draft.stage_edit_active(
                newell_model::parameters::ParameterEdit {
                    parameter: self.bindings.get(&field).copied(),
                    owner: newell_model::parameters::ParameterOwner::Document,
                    source: request.source.clone(),
                    destination: old.quantity.kind(),
                    source_edit: None,
                    source_origins: Some(request.source_origins.clone()),
                },
                &roots,
            )?;
            candidate.bindings.insert(field, id);
        }
        for (key, id) in &candidate.bindings {
            let definition = candidate
                .parameter_draft
                .registry()
                .definition(*id)
                .ok_or(AppError::InvalidNumericRequest)?;
            let value = candidate
                .values
                .get_mut(key)
                .ok_or(AppError::InvalidNumericRequest)?;
            value.quantity = definition.value;
        }
        candidate
            .values
            .get_mut(&field)
            .ok_or(AppError::InvalidNumericRequest)?
            .formula = if request.source.trim().is_empty() {
            String::new()
        } else {
            request.source.clone()
        };
        candidate.project()?;
        *self = candidate;
        Ok(())
    }
    pub fn project(&self) -> Result<NumericProjection, AppError> {
        let fields = self
            .values
            .iter()
            .map(|(field, value)| {
                Ok(NumericFieldView {
                    field: *field,
                    magnitude: format!(
                        "{:.1}",
                        value.quantity.value_in(model_unit(
                            self.descriptors
                                .get(field)
                                .ok_or(AppError::InvalidNumericRequest)?
                                .unit
                        ))?
                    ),
                    direction: ui_direction(value.direction),
                    formula: value.formula.clone(),
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        Ok(NumericProjection {
            document: self.document.tree(),
            session: self.session,
            operation: self.operation,
            accepted_sequence: self.consumed_sequence,
            fields,
        })
    }
}
fn model_unit(unit: NumericUnit) -> Unit {
    match unit {
        NumericUnit::Millimetres => Unit::Millimetre,
        NumericUnit::Degrees => Unit::Degree,
        NumericUnit::Unitless => Unit::Unitless,
    }
}
fn model_direction(direction: NumericDirection) -> Direction {
    match direction {
        NumericDirection::Forward => Direction::Forward,
        NumericDirection::Reverse => Direction::Reverse,
    }
}
fn ui_direction(direction: Direction) -> NumericDirection {
    match direction {
        Direction::Forward => NumericDirection::Forward,
        Direction::Reverse => NumericDirection::Reverse,
    }
}
pub(crate) fn build_extrude_spec(
    ui: &newell_ui::ExtrudeValues,
    numeric: &CommandNumericInputs,
    selections: &crate::selection::CommandSelectionInputs,
) -> Result<newell_model::ExtrudeSpec, AppError> {
    use newell_model::{
        ExtrudeDirections, ExtrudeGeometry, ExtrudeResult, ExtrudeSpec, ExtrudeTermination,
        ParameterValue,
    };
    use newell_ui::{ExtrudeOffsetMode as Mode, ExtrudeTermination as Term};
    if numeric.document != selections.document
        || numeric.session != selections.session
        || numeric.operation != ActionOperation::Extrude
        || selections.operation != ActionOperation::Extrude
    {
        return Err(AppError::InvalidNumericRequest);
    }
    let value = |key| {
        numeric
            .values
            .get(&NumericFieldKey {
                operation: ActionOperation::Extrude,
                key,
            })
            .ok_or(AppError::InvalidNumericRequest)
    };
    let parameter = |key: u32, active: bool| -> Result<Option<ParameterValue>, AppError> {
        if !active {
            return Ok(None);
        }
        let v = value(key)?;
        if !v.formula.is_empty() {
            return Err(AppError::UnsupportedNumericFormula(NumericFieldKey {
                operation: ActionOperation::Extrude,
                key,
            }));
        }
        Ok(Some(ParameterValue::Literal(v.quantity)))
    };
    let start = matches!(ui.termination, Term::Blind | Term::ThroughAll)
        && ui.starting_offset
        && ui.start_mode == Mode::Blind;
    let end = ui.termination == Term::Blind && ui.end_offset && ui.end_mode == Mode::Blind;
    let second = !ui.symmetric && ui.second_side && ui.second_mode == Mode::Blind;
    let nested = !ui.symmetric
        && ui.second_side
        && ui.second_mode == Mode::UpTo
        && ui.nested_end
        && ui.nested_mode == Mode::Blind;
    let direction = |key: u32, active: bool| -> Result<Direction, AppError> {
        Ok(if active {
            value(key)?.direction
        } else {
            Direction::Forward
        })
    };
    let depth = value(4)?;
    let selection = |key| {
        selections.values.get(&newell_ui::SelectionFieldKey {
            operation: ActionOperation::Extrude,
            key,
        })
    };
    Ok(ExtrudeSpec {
        profile_geometry: None,
        profile: selection(2).and_then(|v| v.entities().first().copied()),
        geometry: match ui.geometry {
            newell_ui::ExtrudeGeometryType::Solid => ExtrudeGeometry::Solid,
            newell_ui::ExtrudeGeometryType::Surface => ExtrudeGeometry::Surface,
        },
        result: match ui.result {
            newell_ui::ExtrudeResultMode::New => ExtrudeResult::New,
            newell_ui::ExtrudeResultMode::Add => ExtrudeResult::Add,
            newell_ui::ExtrudeResultMode::Remove => ExtrudeResult::Remove,
        },
        termination: match ui.termination {
            Term::Blind => ExtrudeTermination::Blind,
            Term::UpTo => ExtrudeTermination::UpTo,
            Term::ThroughAll => ExtrudeTermination::ThroughAll,
        },
        depth: if depth.formula.is_empty() {
            ParameterValue::Literal(depth.quantity)
        } else {
            ParameterValue::Expression(depth.formula.clone())
        },
        starting_offset: parameter(11, start)?,
        end_offset: parameter(15, end)?,
        second_end: parameter(20, second)?,
        second_end_offset: parameter(24, nested)?,
        directions: ExtrudeDirections {
            depth: direction(4, ui.termination == Term::Blind)?,
            starting_offset: direction(11, start)?,
            end_offset: direction(15, end)?,
            second_end: direction(20, second)?,
        },
        symmetric: ui.symmetric,
        invert: ui.invert,
        merge_with_all: ui.merge_with_all,
        merge_scope: if ui.merge_with_all {
            Vec::new()
        } else {
            selection(27)
                .map(|v| v.entities().to_vec())
                .ok_or(AppError::InvalidNumericRequest)?
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DocumentSession;
    use newell_ui::{NumericDestination, ProjectionGeneration, ShellState};
    fn draft() -> CommandNumericInputs {
        let doc = DocumentSession::default();
        CommandNumericInputs::new(
            doc.identity(),
            CommandInputSessionId(1),
            ActionOperation::Extrude,
            &ShellState::new(82, 40).numeric_descriptors(),
        )
        .unwrap()
    }
    fn request(
        d: &CommandNumericInputs,
        key: u32,
        sequence: u64,
        edit: NumericEdit,
    ) -> NumericRequest {
        NumericRequest {
            destination: NumericDestination {
                document: d.document.tree(),
                session: d.session,
                field: NumericFieldKey {
                    operation: d.operation,
                    key,
                },
            },
            sequence,
            origin: ProjectionGeneration::new(1),
            edit,
        }
    }
    #[test]
    fn rejected_requests_are_atomic_consumed_once_and_do_not_poison_later_edits() {
        let mut d = draft();
        let field = NumericFieldKey {
            operation: ActionOperation::Extrude,
            key: 4,
        };
        let desc = d.descriptors[&field];
        d.values.get_mut(&field).unwrap().quantity =
            Quantity::Length(newell_model::Length::millimetres(1.23456789).unwrap());
        d.values.get_mut(&field).unwrap().formula = "wall + 7.5 mm".into();
        let original = d.values.clone();
        let r = request(&d, 4, 1, NumericEdit::Direction(NumericDirection::Reverse));
        assert!(d.apply(r.clone(), desc).unwrap());
        assert_eq!(d.values[&field].quantity, original[&field].quantity);
        assert_eq!(d.values[&field].formula, original[&field].formula);
        assert!(d.apply(r, desc).is_err());
        for (sequence, magnitude) in [
            (2, "."),
            (3, "-1"),
            (4, "2 deg"),
            (5, "1e999"),
            (6, "100000000"),
        ] {
            let before = d.values.clone();
            assert!(
                d.apply(
                    request(
                        &d,
                        4,
                        sequence,
                        NumericEdit::Literal {
                            magnitude: magnitude.into(),
                            direction: NumericDirection::Forward
                        }
                    ),
                    desc
                )
                .is_err()
            );
            assert_eq!(d.values, before);
            assert_eq!(d.consumed_sequence, sequence);
        }
        let before = d.values.clone();
        assert!(
            d.apply(
                request(&d, 4, 7, NumericEdit::Formula("x".repeat(257))),
                desc
            )
            .is_err()
        );
        assert_eq!(d.values, before);
        let mut stale = request(&d, 4, 8, NumericEdit::Direction(NumericDirection::Forward));
        stale.destination.session = CommandInputSessionId(0);
        assert!(d.apply(stale, desc).is_err());
        assert_eq!(d.consumed_sequence, 7);
        let mut stale = request(&d, 4, 8, NumericEdit::Direction(NumericDirection::Forward));
        stale.destination.document = newell_ui::TreeDocumentId::new(99);
        assert!(d.apply(stale, desc).is_err());
        assert_eq!(d.consumed_sequence, 7);
        let mut wrong = request(&d, 4, 8, NumericEdit::Direction(NumericDirection::Forward));
        wrong.destination.field.operation = ActionOperation::Revolve;
        assert!(d.apply(wrong, desc).is_err());
        let unsigned = NumericFieldKey {
            operation: ActionOperation::Extrude,
            key: 24,
        };
        assert!(
            d.apply(
                request(&d, 24, 8, NumericEdit::Direction(NumericDirection::Reverse)),
                d.descriptors[&unsigned]
            )
            .is_err()
        );
        assert!(
            d.apply(
                request(
                    &d,
                    4,
                    9,
                    NumericEdit::Literal {
                        magnitude: "1.3".into(),
                        direction: NumericDirection::Reverse
                    }
                ),
                desc
            )
            .unwrap()
        );
        assert_eq!(d.values[&field].formula, "");
        assert_eq!(
            d.values[&field]
                .quantity
                .value_in(Unit::Millimetre)
                .unwrap(),
            1.3
        );
        assert_eq!(d.project().unwrap().accepted_sequence, 9);
    }
}
