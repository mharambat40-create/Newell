// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Ordered per-window selection, qualified by the current document instance.

use crate::{DocumentInstanceId, DocumentSession};
use newell_model::{EntityId, Selection};
use newell_ui::{
    CommandInputSessionId, HistoryGap, OpaqueTreeId, SelectionFieldDescriptor, SelectionFieldKey,
    SelectionFieldRole, SelectionRow,
};
use newell_ui::{TreeTarget, TreeTargetKind};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionOp {
    Replace,
    Add,
    Toggle,
    Remove,
    Clear,
}
impl From<newell_ui::SelectionEdit> for SelectionOp {
    fn from(edit: newell_ui::SelectionEdit) -> Self {
        match edit {
            newell_ui::SelectionEdit::Replace => Self::Replace,
            newell_ui::SelectionEdit::Add => Self::Add,
            newell_ui::SelectionEdit::Toggle => Self::Toggle,
            newell_ui::SelectionEdit::Remove => Self::Remove,
            newell_ui::SelectionEdit::Clear => Self::Clear,
        }
    }
}

#[derive(Debug)]
pub(crate) struct CommandSelectionInputs {
    pub(crate) session: CommandInputSessionId,
    pub(crate) document: DocumentInstanceId,
    pub(crate) operation: newell_ui::ActionOperation,
    pub(crate) insertion: HistoryGap,
    pub(crate) values: BTreeMap<SelectionFieldKey, Selection>,
}

pub(crate) fn candidate_rows(
    session: &DocumentSession,
    descriptor: Option<SelectionFieldDescriptor>,
    insertion: Option<HistoryGap>,
) -> Vec<SelectionRow> {
    use SelectionFieldRole::*;
    use newell_model::{ConstructionKind as C, EntityKind as E};
    let mut rows = Vec::new();
    for (feature, value) in session.document().features() {
        if insertion == Some(HistoryGap::Before(OpaqueTreeId::new(feature.get()))) {
            break;
        }
        for id in value.outputs() {
            let Some(entity) = session.document().entity(*id) else {
                continue;
            };
            let eligible = descriptor.is_none_or(|d| match d.role {
                MergeScope | BodyTarget | Bodies => entity.kind() == E::Body,
                SurfaceTarget => matches!(entity.kind(), E::Surface | E::Construction(C::Plane)),
                Axis | Direction => entity.kind() == E::Construction(C::Axis),
                Path => entity.kind() == E::Construction(C::Curve),
                EntityReference => true,
                UnsupportedTopology => false,
            });
            if eligible {
                rows.push(SelectionRow {
                    entity: OpaqueTreeId::new(id.get()),
                    label: entity.name().into(),
                });
            }
        }
    }
    rows
}

#[derive(Debug, Clone)]
pub(crate) struct WindowSelection {
    pub(crate) document: DocumentInstanceId,
    value: Selection,
}
pub(crate) struct SelectionRequest {
    pub(crate) document: DocumentInstanceId,
    pub(crate) op: SelectionOp,
    pub(crate) entities: Vec<EntityId>,
}
impl WindowSelection {
    pub(crate) fn new(document: DocumentInstanceId) -> Self {
        Self {
            document,
            value: Selection::default(),
        }
    }
    pub(crate) fn entities(&self) -> &[EntityId] {
        self.value.entities()
    }
    pub(crate) fn apply(&mut self, request: SelectionRequest, session: &DocumentSession) -> bool {
        if request.document != session.identity()
            || self.document != session.identity()
            || request
                .entities
                .iter()
                .any(|id| session.document().entity(*id).is_none())
        {
            return false;
        }
        let mut input = Selection::default();
        input.replace(request.entities);
        let old = self.value.clone();
        match request.op {
            SelectionOp::Replace => self.value.replace(input.entities().iter().copied()),
            SelectionOp::Add => self.value.add(input.entities().iter().copied()),
            SelectionOp::Toggle => {
                for id in input.entities() {
                    self.value.toggle(*id);
                }
            }
            SelectionOp::Remove => self.value.remove(input.entities().iter().copied()),
            SelectionOp::Clear => self.value.clear(),
        }
        self.value != old
    }
    pub(crate) fn reconcile(&mut self, session: &DocumentSession) -> bool {
        let old = self.entities().to_vec();
        if self.document != session.identity() {
            self.document = session.identity();
            self.value.clear();
        } else {
            self.value.replace(
                old.iter()
                    .copied()
                    .filter(|id| session.document().entity(*id).is_some()),
            );
        }
        old != self.entities()
    }
}
pub(crate) fn resolve_tree_selection(
    session: &DocumentSession,
    target: TreeTarget,
) -> Option<Vec<EntityId>> {
    if target.document != session.identity().tree() {
        return None;
    }
    match target.kind {
        TreeTargetKind::Entity(id) => {
            let id = EntityId::from_raw(id.get());
            session.document().entity(id).map(|_| vec![id])
        }
        TreeTargetKind::Operation(id) => session
            .feature_id(id)
            .and_then(|id| session.document().feature(id))
            .map(|f| f.outputs().to_vec()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use newell_model::{EntityKind, NewEntity};
    use newell_ui::OpaqueTreeId;
    #[test]
    fn ordered_operations_namespace_and_multi_output_resolution() {
        let mut session = DocumentSession::default();
        let f = session
            .add_base_feature(
                "many".into(),
                ["a", "b", "a"]
                    .map(|name| NewEntity {
                        name: name.into(),
                        kind: EntityKind::Body,
                        visible: true,
                    })
                    .into(),
            )
            .unwrap();
        let ids = session.document().feature(f).unwrap().outputs().to_vec();
        let target = TreeTarget {
            document: session.identity().tree(),
            kind: TreeTargetKind::Operation(OpaqueTreeId::new(f.get())),
        };
        assert_eq!(resolve_tree_selection(&session, target), Some(ids.clone()));
        let mut value = WindowSelection::new(session.identity());
        let mut apply = |op, entities| {
            value.apply(
                SelectionRequest {
                    document: session.identity(),
                    op,
                    entities,
                },
                &session,
            )
        };
        assert!(apply(SelectionOp::Replace, vec![ids[1], ids[0], ids[1]]));
        assert!(apply(SelectionOp::Toggle, vec![ids[1], ids[1], ids[2]]));
        assert!(apply(SelectionOp::Remove, vec![ids[0]]));
        assert!(apply(SelectionOp::Add, vec![ids[0], ids[2]]));
        assert_eq!(value.entities(), &[ids[2], ids[0]]);
        let before = value.entities().to_vec();
        assert!(!value.apply(
            SelectionRequest {
                document: session.identity(),
                op: SelectionOp::Replace,
                entities: vec![ids[1], EntityId::from_raw(u64::MAX)],
            },
            &session,
        ));
        assert_eq!(value.entities(), before);
        let empty = session
            .add_base_feature("empty".into(), Vec::new())
            .unwrap();
        assert_eq!(
            resolve_tree_selection(
                &session,
                TreeTarget {
                    document: session.identity().tree(),
                    kind: TreeTargetKind::Operation(OpaqueTreeId::new(empty.get())),
                },
            ),
            Some(Vec::new()),
        );
        let events = session.subscribe();
        assert!(events.try_recv().is_err());
        session.replace(Default::default()).unwrap();
        assert!(value.reconcile(&session));
        assert!(value.entities().is_empty());
        assert!(resolve_tree_selection(&session, target).is_none());
    }
}
