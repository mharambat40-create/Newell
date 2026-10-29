// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Model-independent selection projections and one-shot modal transactions.

use crate::{ActionOperation, ModelTreeProjection, OpaqueTreeId, TreeDocumentId, UiError};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Ordered entity membership copied from the application for one document instance.
/// Installing this projection does not emit a user-selection request.
pub struct EntitySelectionView {
    /// Namespace qualifying the entity identities in this value.
    pub document: TreeDocumentId,
    /// Unique entity identities in selection order.
    pub entities: Vec<OpaqueTreeId>,
}
impl EntitySelectionView {
    /// Rejects a different document, duplicate identities or entities absent from the projection.
    pub fn validate(&self, projection: &ModelTreeProjection) -> Result<(), UiError> {
        if self.document != projection.document
            || !unique(&self.entities)
            || self
                .entities
                .iter()
                .any(|id| !projection.elements.iter().any(|e| e.id == *id))
        {
            return Err(UiError::InvalidSelection);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// An ordered membership edit; input validation and mutation belong to the application.
pub enum SelectionEdit {
    /// Replace membership with the unique input identities in input order.
    Replace,
    /// Append identities that are not already members.
    Add,
    /// Visit each unique input once, removing members and appending nonmembers.
    Toggle,
    /// Remove input identities while preserving survivor order.
    Remove,
    /// Remove all membership.
    Clear,
}
impl SelectionEdit {
    /// Maps the Controller modifier snapshot; toggle takes precedence over Shift.
    pub const fn modifiers(shift: bool, toggle: bool) -> Self {
        if toggle {
            Self::Toggle
        } else if shift {
            Self::Add
        } else {
            Self::Replace
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
/// A model-free candidate; equal labels do not imply equal identities.
pub struct SelectionRow {
    /// Opaque row key. Whole-entity lists use entity ids; geometry fields use captured app row keys.
    pub entity: OpaqueTreeId,
    /// Full stored name, clipped only when composing the panel.
    pub label: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// A semantic command input, independent of its current WidgetId or visible row.
pub struct SelectionFieldKey {
    /// Command owning the input; different commands may reuse numeric keys.
    pub operation: ActionOperation,
    /// The Action Box's semantic selection-control key.
    pub key: u32,
}
impl SelectionFieldKey {
    /// Fields whose row keys resolve to a captured supporting face/boundary in the app.
    #[must_use]
    pub const fn geometry_qualified(self) -> bool {
        matches!(self.operation, ActionOperation::Extrude)
            && matches!(self.key, 2 | 5 | 6 | 12 | 16 | 21 | 25)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Candidate policy interpreted by the application, without model types in the UI.
pub enum SelectionFieldRole {
    /// Closed solid bodies participating in a merge.
    MergeScope,
    /// One closed solid target.
    BodyTarget,
    /// A collection of closed solid bodies.
    Bodies,
    /// A surface or construction plane target.
    SurfaceTarget,
    /// A construction axis.
    Axis,
    /// A direction represented by a construction axis.
    Direction,
    /// A construction curve reference, without geometry evaluation.
    Path,
    /// An entity reference with no additional geometry contract.
    EntityReference,
    /// An input with no supported stable entity kind and therefore no candidates.
    UnsupportedTopology,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Candidate policy and cardinality for an existing command input.
pub struct SelectionFieldDescriptor {
    /// Semantic input receiving this value.
    pub field: SelectionFieldKey,
    /// Application-side candidate filter.
    pub role: SelectionFieldRole,
    /// Whether the command input treats selection order as meaningful.
    pub ordered: bool,
    /// Maximum membership count, or no cardinality limit.
    pub max_count: Option<usize>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
/// Applied command-input membership count, independent of the open panel draft.
pub struct SelectionFieldCount {
    /// Semantic input receiving this value.
    pub field: SelectionFieldKey,
    /// Number of applied identities in this input.
    pub count: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
/// Window membership and command-input counts published together.
pub struct LocalSelectionView {
    /// Application-owned window selection projected into the UI.
    pub selection: EntitySelectionView,
    /// Built-in world-origin references selected in this window, by semantic identity.
    pub origin_references: Vec<crate::OriginReference>,
    /// Complete field counts, including zero counts for empty inputs.
    pub fields: Vec<SelectionFieldCount>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Application-allocated identity of one modal transaction, never reused on restart.
pub struct SelectionSessionId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Application-allocated identity of one command-input lifetime.
pub struct CommandInputSessionId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The only state that a validated panel completion may replace.
pub enum SelectionDestination {
    /// The authoritative selection of the current window.
    Window,
    /// One input of the exact originating command session.
    Field {
        /// Command lifetime that opened the panel.
        session: CommandInputSessionId,
        /// Semantic input within that lifetime.
        field: SelectionFieldKey,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
/// Application-supplied candidates and applied snapshot for an isolated panel draft.
pub struct SelectionOpen {
    /// Exact modal transaction to which this value belongs.
    pub session: SelectionSessionId,
    /// Namespace qualifying the entity identities in this value.
    pub document: TreeDocumentId,
    /// State to replace after a certified, validated confirmation.
    pub destination: SelectionDestination,
    /// Candidates in document feature/output order after application filtering.
    pub rows: Vec<SelectionRow>,
    /// Unique initial membership in selection order; every identity must be a candidate.
    pub applied: Vec<OpaqueTreeId>,
    /// Maximum membership count, or no cardinality limit.
    pub max_count: Option<usize>,
}
impl SelectionOpen {
    /// Checks UI capacity, unique identities, initial candidate membership and cardinality.
    /// Document existence and destination lifetime are validated by the application.
    pub fn validate(&self) -> Result<(), UiError> {
        let ids: Vec<_> = self.rows.iter().map(|r| r.entity).collect();
        if ids.len() > crate::MAX_CELLS as usize
            || !unique(&ids)
            || !unique(&self.applied)
            || self.applied.iter().any(|id| !ids.contains(id))
            || self.max_count.is_some_and(|max| self.applied.len() > max)
        {
            return Err(UiError::InvalidSelection);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
/// A close result; editing the draft alone produces neither outcome nor applied mutation.
pub enum SelectionOutcome {
    /// Ordered draft membership submitted for application-side validation.
    Confirmed(Vec<OpaqueTreeId>),
    /// Discard the draft without changing its destination.
    Cancelled,
}
#[derive(Debug, Clone, PartialEq, Eq)]
/// One-shot close payload, certified by the Controller and revalidated by the application.
pub struct SelectionCompletion {
    /// Exact modal transaction to which this value belongs.
    pub session: SelectionSessionId,
    /// Namespace qualifying the entity identities in this value.
    pub document: TreeDocumentId,
    /// State to replace after a certified, validated confirmation.
    pub destination: SelectionDestination,
    /// Confirmation or cancellation of this transaction.
    pub outcome: SelectionOutcome,
}
#[derive(Debug, Clone, PartialEq, Eq)]
/// User intent consumed by the application; installing projections emits no intent.
pub enum SelectionUiRequest {
    /// Request candidates for a live command input instead of opening runtime mock rows.
    OpenField(SelectionFieldDescriptor),
    /// Deliver the certified close of one panel transaction.
    Complete(SelectionCompletion),
}
pub(crate) fn unique(ids: &[OpaqueTreeId]) -> bool {
    ids.iter().copied().collect::<BTreeSet<_>>().len() == ids.len()
}

/// One explicit Utilities > Properties activation, independent of entity selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertiesRequestId(pub u64);
/// The focused Features operation captured under the published pre-input generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertiesOpenRequest {
    pub id: PropertiesRequestId,
    pub target: crate::TreeTarget,
    pub origin: crate::ProjectionGeneration,
}
