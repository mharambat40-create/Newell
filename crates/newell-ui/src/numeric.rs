// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Model-free numeric editing exchange types.

use crate::{ActionOperation, CommandInputSessionId, ProjectionGeneration, TreeDocumentId};

/// Stable semantic field identity, independent of widget base, row, label, and active branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NumericFieldKey {
    /// Panel owning this field's contract and local cache.
    pub operation: ActionOperation,
    /// Existing operation-local control key; the value field uses widget slot zero.
    pub key: u32,
}
/// Fixed presentation and literal-input unit, mapped to model units by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericUnit {
    /// Length with a fixed ` mm` suffix.
    Millimetres,
    /// Angle with a fixed ` deg` suffix.
    Degrees,
    /// Dimensionless magnitude with no suffix.
    Unitless,
}
/// Independent orientation, never a sign applied to the magnitude or formula source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NumericDirection {
    /// The field's forward reference, drawn as `+|` when directional.
    #[default]
    Forward,
    /// The field's reversed reference, drawn as `-|` when directional.
    Reverse,
}
/// Explicit editor metadata for one field, including fields retained while inactive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumericFieldDescriptor {
    /// Operation and stable control key.
    pub field: NumericFieldKey,
    /// Fixed unit outside the editable magnitude.
    pub unit: NumericUnit,
    /// Whether the field exposes an independent orientation control.
    pub directional: bool,
    /// Explicit initial display text, without prefix or suffix.
    pub default_magnitude: &'static str,
    /// Compact editor integer-digit limit; this does not limit stored quantity precision.
    pub integer_digits: u8,
}
/// Exact document, command lifetime, and field authorized to receive an edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumericDestination {
    /// Application-supplied document namespace; replacement invalidates older requests.
    pub document: TreeDocumentId,
    /// Application-allocated command-input lifetime, shared with selection inputs.
    pub session: CommandInputSessionId,
    /// Semantic field within that lifetime.
    pub field: NumericFieldKey,
}
/// Candidate edit sent to the application for validation, without any model types in the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumericEdit {
    /// Explicit magnitude replacement; successful application clears the prior formula.
    Literal {
        /// The compact editor emits normalized text without a unit or orientation prefix;
        /// the application validates the literal grammar at its boundary.
        magnitude: String,
        /// Independent orientation accepted with the magnitude.
        direction: NumericDirection,
    },
    /// Orientation-only change, preserving canonical precision and the formula source.
    Direction(NumericDirection),
    /// Exact confirmed source; an empty string returns to the retained literal.
    Formula(String),
    /// Failed lexical acceptance such as empty text or `.`; never an accepted value.
    Incomplete(String),
}
/// Qualified candidate carrying its sequence and the pre-input routing certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericRequest {
    /// The only draft entry this candidate may address.
    pub destination: NumericDestination,
    /// Checked, increasing sequence within the command session.
    pub sequence: u64,
    /// Published generation captured before the originating input could hide the field.
    pub origin: ProjectionGeneration,
    /// Candidate requiring app/model validation before accepted draft replacement.
    pub edit: NumericEdit,
}
/// Rounded presentation of one accepted app-owned value; not a storage representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericFieldView {
    /// Semantic destination within its enclosing projection.
    pub field: NumericFieldKey,
    /// Magnitude only, with one fractional digit and no prefix or suffix.
    pub magnitude: String,
    /// Orientation shown independently of the magnitude.
    pub direction: NumericDirection,
    /// Exact accepted formula source, possibly empty.
    pub formula: String,
}
/// Complete accepted field views for one command session, including inactive cached fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericProjection {
    /// Namespace qualifying the session and its fields.
    pub document: TreeDocumentId,
    /// Command-input lifetime receiving this projection.
    pub session: CommandInputSessionId,
    /// Operation owning every projected field.
    pub operation: ActionOperation,
    /// Highest consumed sequence, including rejected candidates and resolved Formula openings.
    pub accepted_sequence: u64,
    /// Exactly one view for each known field of the operation.
    pub fields: Vec<NumericFieldView>,
}
/// Checked per-session identity allocated for every explicit OK, even without a numeric edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NumericConfirmAttemptId(pub u64);
/// One explicit OK attempt awaiting acceptance and an exact terminal acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumericConfirmRequest {
    /// Originating document namespace.
    pub document: TreeDocumentId,
    /// Originating command-input lifetime.
    pub session: CommandInputSessionId,
    /// Panel whose terminal state may be acknowledged.
    pub operation: ActionOperation,
    /// Distinguishes retries at the same numeric barrier.
    pub attempt: NumericConfirmAttemptId,
    /// Numeric prefix to resolve before confirmation; never the attempt identity.
    pub through_sequence: u64,
}
/// Deferred Formula opening, initialized only after preceding field acceptance resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumericFormulaOpenRequest {
    /// Field whose resulting accepted source initializes the modal.
    pub destination: NumericDestination,
    /// Opening identity reserved after any same-event blur edit.
    pub sequence: u64,
    /// Preceding numeric prefix, including a candidate that may be rejected.
    pub through_sequence: u64,
    /// Pre-input published generation certifying the Formula control's field.
    pub origin: ProjectionGeneration,
}

pub(crate) const fn field(operation: ActionOperation, key: u32) -> NumericFieldDescriptor {
    use ActionOperation::*;
    let unit = match operation {
        Revolve => NumericUnit::Degrees,
        Loft => NumericUnit::Unitless,
        Sweep if key == 7 => NumericUnit::Degrees,
        Sweep => NumericUnit::Unitless,
        _ => NumericUnit::Millimetres,
    };
    let directional = match operation {
        Extrude => key != 24,
        Revolve => matches!(key, 6 | 10 | 19),
        Sweep => key == 7,
        Boolean => true,
        Loft => false,
    };
    NumericFieldDescriptor {
        field: NumericFieldKey { operation, key },
        unit,
        directional,
        default_magnitude: "0.0",
        integer_digits: if matches!(unit, NumericUnit::Degrees) {
            7
        } else {
            8
        },
    }
}
pub(crate) fn descriptors(operation: ActionOperation) -> Vec<NumericFieldDescriptor> {
    use ActionOperation::*;
    let keys: &[u32] = match operation {
        Extrude => &[4, 11, 15, 20, 24],
        Revolve => &[6, 10, 14, 19, 24],
        Loft => &[5, 9],
        Sweep => &[7, 9],
        Boolean => &[6],
    };
    keys.iter().map(|key| field(operation, *key)).collect()
}
pub(crate) fn descriptor(operation: ActionOperation, key: u32) -> Option<NumericFieldDescriptor> {
    descriptors(operation)
        .into_iter()
        .find(|d| d.field.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn descriptor_inventory_has_fifteen_unique_fields() {
        let ops = [
            ActionOperation::Extrude,
            ActionOperation::Revolve,
            ActionOperation::Loft,
            ActionOperation::Sweep,
            ActionOperation::Boolean,
        ];
        let fields: Vec<_> = ops
            .into_iter()
            .flat_map(|op| (0..=24).filter_map(move |key| descriptor(op, key)))
            .collect();
        assert_eq!(fields.len(), 15);
        let unique: std::collections::BTreeSet<_> = fields.iter().map(|d| d.field).collect();
        assert_eq!(unique.len(), 15);
    }
}

/// One explicit Formula OK, independent of a command OK and its numeric barrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormulaAttemptId(pub u64);
/// Model-free application request. The modal stays open until this exact result is delivered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaApplyRequest {
    pub destination: NumericDestination,
    /// Unique opening sequence within the command lifetime.
    pub modal_session: u64,
    pub attempt: FormulaAttemptId,
    pub through_sequence: u64,
    pub origin: ProjectionGeneration,
    pub source: String,
    /// Original byte position for each surviving source character; inserted characters are None.
    pub source_origins: Vec<Option<usize>>,
}
