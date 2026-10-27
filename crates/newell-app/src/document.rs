// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Document ownership, successful-change publication and retained UI synchronization.

use crate::AppError;
use newell_model::{
    CommandController, Document, EntityId, EntityKind, ExtrudeSpec, FeatureId, FolderId,
    HistoryLayoutRow, HistoryNodeId, NewEntity,
};
use newell_ui::{
    ElementGroup, ElementTreeItem, FeatureNodeId, FeatureTreeItem, ModelTreeProjection,
    OpaqueTreeId, TreeDocumentId, TreeIntentKind, TreeLabels, TreeTarget, TreeTargetKind,
};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

/// Document namespace allocated by one long-lived application session.
/// Replacement advances this identity even when model ids and revisions restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentInstanceId(u64);
impl DocumentInstanceId {
    /// Converts the session identity to the UI's model-independent namespace.
    pub const fn tree(self) -> TreeDocumentId {
        TreeDocumentId::new(self.0)
    }
}
/// Complete accepted document version; revision numbers are never compared across namespaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentVersion {
    pub instance: DocumentInstanceId,
    pub revision: u64,
}
/// Notification sent after a successful persistent mutation or document replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentNotification {
    /// A mutation committed in the existing namespace.
    Changed {
        document: DocumentInstanceId,
        revision: u64,
    },
    /// A new namespace replaced the old document; its revision starts at zero.
    Replaced {
        document: DocumentInstanceId,
        revision: u64,
    },
}
/// Owns the current document, replacement identity, revision and change subscribers.
///
/// Mutation wrappers publish once after a commit. Errors and unchanged metadata publish
/// nothing. Keep this session alive across replacement to avoid reusing its namespaces.
#[derive(Debug)]
pub struct DocumentSession {
    document: Document,
    identity: DocumentInstanceId,
    revision: u64,
    senders: Vec<Sender<DocumentNotification>>,
    geometry_token: Option<crate::geometry_worker::NewellGeometryToken>,
    worker_epoch: Option<u64>,
}
impl Default for DocumentSession {
    fn default() -> Self {
        Self {
            document: Document::default(),
            identity: DocumentInstanceId(1),
            revision: 0,
            senders: Vec::new(),
            geometry_token: None,
            worker_epoch: None,
        }
    }
}
impl DocumentSession {
    /// Immutable access to the current model state.
    pub const fn document(&self) -> &Document {
        &self.document
    }
    /// Namespace of the current document.
    pub const fn identity(&self) -> DocumentInstanceId {
        self.identity
    }
    /// Committed revision, comparable only within the current namespace.
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    /// Source certificate for a frozen worker request.
    #[must_use]
    pub const fn version(&self) -> DocumentVersion {
        DocumentVersion {
            instance: self.identity,
            revision: self.revision,
        }
    }
    /// Selects the current worker lifetime; old tokens cannot authorize a later acceptance.
    ///
    /// # Errors
    /// Rejects zero or a reused/older worker epoch.
    pub fn bind_geometry_worker(&mut self, epoch: u64) -> Result<(), AppError> {
        if epoch == 0 || self.worker_epoch.is_some_and(|old| epoch <= old) {
            return Err(AppError::InvalidDocumentProjection);
        }
        self.worker_epoch = Some(epoch);
        self.geometry_token = None;
        Ok(())
    }
    /// Currently accepted native geometry association, independent of UI delivery retries.
    #[must_use]
    pub const fn geometry_token(&self) -> Option<crate::geometry_worker::NewellGeometryToken> {
        self.geometry_token
    }
    /// Consumes finalized model authority and its exact retained worker token once.
    ///
    /// # Errors
    /// Checks version, lifetime, sealed validation inputs and revision capacity before mutation.
    pub fn accept_prepared(
        &mut self,
        expected: DocumentVersion,
        prepared: newell_model::geometry::PreparedDocument,
        token: crate::geometry_worker::NewellGeometryToken,
    ) -> Result<newell_model::geometry::CommitReceipt, AppError> {
        if expected != self.version()
            || self.worker_epoch != Some(token.worker_epoch())
            || !token.matches(self.identity, prepared.manifest().generation())
        {
            return Err(AppError::InvalidDocumentProjection);
        }
        let revision = self.next_revision()?;
        self.document.validate_prepared(&prepared)?;
        let feature = prepared.manifest().feature();
        let receipt = newell_model::geometry::CommitReceipt {
            feature,
            outputs: feature
                .and_then(|feature| prepared.manifest().outputs().get(&feature).cloned())
                .unwrap_or_default(),
            generation: prepared.manifest().generation(),
        };
        let document = prepared.accept(&self.document)?;
        self.document = document;
        self.geometry_token = Some(token);
        self.changed(revision);
        Ok(receipt)
    }
    /// Subscribes to future changes; callers seed their initial projection from current state.
    pub fn subscribe(&mut self) -> Receiver<DocumentNotification> {
        let (sender, receiver) = mpsc::channel();
        self.senders.push(sender);
        receiver
    }
    fn next_revision(&self) -> Result<u64, AppError> {
        self.revision
            .checked_add(1)
            .ok_or(AppError::DocumentCounterExhausted)
    }
    fn publish(&mut self, notification: DocumentNotification) {
        self.senders
            .retain(|sender| sender.send(notification).is_ok());
    }
    #[cfg(test)]
    pub(crate) fn set_revision_for_test(&mut self, revision: u64) {
        self.revision = revision;
    }
    fn changed(&mut self, revision: u64) {
        self.revision = revision;
        self.publish(DocumentNotification::Changed {
            document: self.identity,
            revision,
        });
    }
    /// Installs a document in a fresh namespace and publishes its initial revision zero.
    /// Identity exhaustion leaves the current document and subscribers unchanged.
    pub fn replace(&mut self, document: Document) -> Result<(), AppError> {
        let identity = DocumentInstanceId(
            self.identity
                .0
                .checked_add(1)
                .ok_or(AppError::DocumentCounterExhausted)?,
        );
        self.document = document;
        self.geometry_token = None;
        self.identity = identity;
        self.revision = 0;
        self.publish(DocumentNotification::Replaced {
            document: identity,
            revision: 0,
        });
        Ok(())
    }
    /// Appends a base operation with one visible body bearing the same initial name.
    pub fn add_base_solid(&mut self, name: &str) -> Result<FeatureId, AppError> {
        let revision = self.next_revision()?;
        let id = self.document.add_base_solid(name)?;
        self.changed(revision);
        Ok(id)
    }
    /// Appends a base operation with outputs in the supplied order, including an empty list.
    pub fn add_base_feature(
        &mut self,
        name: String,
        outputs: Vec<NewEntity>,
    ) -> Result<FeatureId, AppError> {
        let revision = self.next_revision()?;
        let id = self.document.add_base_feature(name, outputs)?;
        self.changed(revision);
        Ok(id)
    }
    /// Appends an empty root folder without changing operation order.
    pub fn create_folder(&mut self, name: String) -> Result<FolderId, AppError> {
        let revision = self.next_revision()?;
        let id = self.document.create_folder(name)?;
        self.changed(revision);
        Ok(id)
    }
    /// Changes folder grouping while preserving model operation order; returns whether changed.
    pub fn set_history_layout(&mut self, rows: Vec<HistoryLayoutRow>) -> Result<bool, AppError> {
        let revision = self.next_revision()?;
        let changed = self.document.set_history_layout(rows)?;
        if changed {
            self.changed(revision);
        }
        Ok(changed)
    }
    fn edit_metadata(
        &mut self,
        edit: impl FnOnce(&mut Document) -> Result<bool, newell_model::ModelError>,
    ) -> Result<bool, AppError> {
        let revision = self.next_revision()?;
        let changed = edit(&mut self.document)?;
        if changed {
            self.changed(revision);
        }
        Ok(changed)
    }
    /// Renames an operation independently of its entities; returns whether changed.
    pub fn rename_feature(&mut self, id: FeatureId, name: String) -> Result<bool, AppError> {
        self.edit_metadata(|document| document.rename_feature(id, name))
    }
    /// Renames a folder without changing its identity or grouping; returns whether changed.
    pub fn rename_folder(&mut self, id: FolderId, name: String) -> Result<bool, AppError> {
        self.edit_metadata(|document| document.rename_folder(id, name))
    }
    /// Renames an entity independently of its producing operation; returns whether changed.
    pub fn rename_entity(&mut self, id: EntityId, name: String) -> Result<bool, AppError> {
        self.edit_metadata(|document| document.rename_entity(id, name))
    }
    /// Persists tree visibility metadata; this does not filter viewport geometry.
    pub fn set_entity_visibility(&mut self, id: EntityId, visible: bool) -> Result<bool, AppError> {
        self.edit_metadata(|document| document.set_entity_visibility(id, visible))
    }
    /// Commits the latest UI draft before an operation, or at End when `before` is `None`.
    /// Reference and capacity preflight precede candidate editing; failure preserves the
    /// live command, model, revision and notification stream.
    pub fn confirm_before(
        &mut self,
        commands: &mut CommandController,
        spec: ExtrudeSpec,
        before: Option<FeatureId>,
    ) -> Result<Option<FeatureId>, AppError> {
        self.document.validate_extrude_insertion(&spec, before)?;
        let revision = self.next_revision()?;
        let mut candidate = commands.clone();
        candidate.edit_extrude(spec, &self.document)?;
        let feature = candidate.confirm_before(&mut self.document, before)?;
        if feature.is_some() {
            *commands = candidate;
            self.changed(revision);
        }
        Ok(feature)
    }
    /// Resolves a projected operation id in the current document.
    /// The caller separately validates the projection's document namespace.
    pub fn feature_id(&self, id: OpaqueTreeId) -> Option<FeatureId> {
        self.document
            .features()
            .iter()
            .find_map(|(current, _)| (current.get() == id.get()).then_some(*current))
    }
    fn folder_id(&self, id: OpaqueTreeId) -> Option<FolderId> {
        self.document
            .history_layout()
            .iter()
            .find_map(|row| match row.node {
                HistoryNodeId::Folder(current) if current.get() == id.get() => Some(current),
                _ => None,
            })
    }
    /// Applies an edit after namespace, target-kind and expected-source checks.
    /// Returns `false` for stale targets, mismatched sources and unchanged values.
    /// The controller must already have validated the intent's frame generation.
    pub fn apply_tree_edit(
        &mut self,
        target: TreeTarget,
        intent: &TreeIntentKind,
    ) -> Result<bool, AppError> {
        if target.document != self.identity.tree() {
            return Ok(false);
        }
        let changed = match (target.kind, intent) {
            (TreeTargetKind::Operation(id), TreeIntentKind::Rename { source_name, name }) => {
                let Some(id) = self.feature_id(id) else {
                    return Ok(false);
                };
                if self
                    .document
                    .feature(id)
                    .is_none_or(|feature| feature.name() != source_name)
                {
                    return Ok(false);
                }
                self.rename_feature(id, name.clone())?
            }
            (TreeTargetKind::Folder(id), TreeIntentKind::Rename { source_name, name }) => {
                let Some(id) = self.folder_id(id) else {
                    return Ok(false);
                };
                if self
                    .document
                    .folder(id)
                    .is_none_or(|folder| folder.name() != source_name)
                {
                    return Ok(false);
                }
                self.rename_folder(id, name.clone())?
            }
            (TreeTargetKind::Entity(id), TreeIntentKind::Rename { source_name, name }) => {
                let id = EntityId::from_raw(id.get());
                if self.document.entity(id).is_none_or(|entity| {
                    entity.kind() != EntityKind::Body || entity.name() != source_name
                }) {
                    return Ok(false);
                }
                self.rename_entity(id, name.clone())?
            }
            (
                TreeTargetKind::Entity(id),
                TreeIntentKind::SetVisibility {
                    expected_visible,
                    visible,
                },
            ) => {
                let id = EntityId::from_raw(id.get());
                if self
                    .document
                    .entity(id)
                    .is_none_or(|entity| entity.visible() != *expected_visible)
                {
                    return Ok(false);
                }
                self.set_entity_visibility(id, *visible)?
            }
            _ => false,
        };
        Ok(changed)
    }
}

/// Converts stored history/layout and ordered outputs into model-independent UI rows.
/// Names and visibility are copied without truncation; entity-map order is never display order.
pub fn project_document(
    instance: DocumentInstanceId,
    document: &Document,
) -> Result<ModelTreeProjection, AppError> {
    let mut features = Vec::new();
    for row in document.history_layout() {
        let (id, name) = match row.node {
            HistoryNodeId::Operation(id) => (
                FeatureNodeId::Operation(OpaqueTreeId::new(id.get())),
                document
                    .feature(id)
                    .ok_or(AppError::InvalidDocumentProjection)?
                    .name(),
            ),
            HistoryNodeId::Folder(id) => (
                FeatureNodeId::Folder(OpaqueTreeId::new(id.get())),
                document
                    .folder(id)
                    .ok_or(AppError::InvalidDocumentProjection)?
                    .name(),
            ),
        };
        features.push(FeatureTreeItem {
            id,
            name: name.into(),
            depth: row
                .depth
                .checked_add(1)
                .ok_or(AppError::InvalidDocumentProjection)?,
            visibility: None,
        });
    }
    let mut elements = Vec::new();
    for (feature, value) in document.features() {
        for (output_ordinal, id) in value.outputs().iter().enumerate() {
            let entity = document
                .entity(*id)
                .ok_or(AppError::InvalidDocumentProjection)?;
            let group = match entity.kind() {
                EntityKind::Body => ElementGroup::Bodies,
                EntityKind::Surface => ElementGroup::Surfaces,
                EntityKind::Construction(_) => ElementGroup::Construct,
            };
            elements.push(ElementTreeItem {
                id: OpaqueTreeId::new(id.get()),
                source_feature: OpaqueTreeId::new(feature.get()),
                output_ordinal,
                group,
                name: entity.name().into(),
                visible: entity.visible(),
            });
        }
    }
    let projection = ModelTreeProjection {
        document: instance.tree(),
        features,
        elements,
        labels: TreeLabels::default(),
    };
    projection.validate()?;
    Ok(projection)
}

#[derive(Debug)]
struct PendingProjection {
    document: DocumentInstanceId,
    revision: u64,
    attempts: u8,
    retry_at: Option<Instant>,
    suspended: bool,
}
#[derive(Debug, Default)]
pub(crate) struct ProjectionSync {
    acknowledged: Option<(DocumentInstanceId, u64)>,
    pending: Option<PendingProjection>,
    #[cfg(test)]
    pub(crate) fail_next_projection: u8,
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum ProjectionTrigger {
    Notification,
    RetryDeadline,
    UserActivation,
    LayoutPublished,
}
impl ProjectionSync {
    /// Last acknowledged complete document projection version.
    #[must_use]
    pub fn acknowledged_version(&self) -> Option<DocumentVersion> {
        self.acknowledged
            .map(|(instance, revision)| DocumentVersion { instance, revision })
    }
    pub(crate) fn acknowledged_document(&self) -> Option<TreeDocumentId> {
        self.acknowledged_version()
            .map(|version| version.instance.tree())
    }
    pub(crate) fn deadline(&self) -> Option<Instant> {
        self.pending.as_ref().and_then(|pending| pending.retry_at)
    }
    pub(crate) fn poll<T>(
        &mut self,
        session: &DocumentSession,
        receiver: &Receiver<DocumentNotification>,
        now: Instant,
        trigger: ProjectionTrigger,
        publish: impl FnOnce(ModelTreeProjection) -> Result<T, AppError>,
    ) -> Result<Option<T>, AppError> {
        // Draining is not acknowledgment. Ignore superseded namespaces and coalesce valid
        // notices into the actual current revision, including startup before subscription.
        let notified = receiver
            .try_iter()
            .filter_map(|event| match event {
                DocumentNotification::Changed { document, revision }
                | DocumentNotification::Replaced { document, revision } => {
                    (document == session.identity() && revision <= session.revision())
                        .then_some(revision)
                }
            })
            .max();
        let current = (
            session.identity(),
            session.revision().max(notified.unwrap_or(0)),
        );
        if self.acknowledged == Some(current) {
            return Ok(None);
        }
        if self
            .pending
            .as_ref()
            .is_none_or(|pending| (pending.document, pending.revision) != current)
        {
            self.pending = Some(PendingProjection {
                document: current.0,
                revision: current.1,
                attempts: 0,
                retry_at: None,
                suspended: false,
            });
        }
        let Some(pending) = &mut self.pending else {
            return Ok(None);
        };
        if pending.suspended
            && matches!(
                trigger,
                ProjectionTrigger::UserActivation | ProjectionTrigger::LayoutPublished
            )
        {
            pending.attempts = 0;
            pending.suspended = false;
        }
        if pending.suspended
            || pending.attempts != 0 && pending.retry_at.is_none_or(|deadline| now < deadline)
        {
            return Ok(None);
        }
        pending.attempts += 1;
        pending.retry_at = None;
        #[cfg(test)]
        let failed = if self.fail_next_projection > 0 {
            self.fail_next_projection -= 1;
            true
        } else {
            false
        };
        #[cfg(not(test))]
        let failed = false;
        let result = if failed {
            Err(AppError::InvalidDocumentProjection)
        } else {
            project_document(current.0, session.document()).and_then(publish)
        };
        match result {
            Ok(update) => {
                self.acknowledged = Some(current);
                self.pending = None;
                Ok(Some(update))
            }
            Err(error) => {
                if pending.attempts == 1 {
                    pending.retry_at = now.checked_add(Duration::from_millis(100));
                } else {
                    pending.suspended = true;
                }
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::{ShellController, Update};
    use newell_model::{ConstructionKind, ExtrudeReference, ModelError, ParameterValue, Selection};
    use newell_ui::{GridMetrics, ShellState};

    fn controller() -> ShellController {
        ShellController::new(82, 30, GridMetrics::new(4, 9).unwrap()).unwrap()
    }
    fn poll(
        sync: &mut ProjectionSync,
        session: &DocumentSession,
        events: &Receiver<DocumentNotification>,
        shell: &mut ShellController,
        time: Instant,
        trigger: ProjectionTrigger,
    ) -> Result<Option<Update>, AppError> {
        sync.poll(session, events, time, trigger, |projection| {
            shell
                .set_tree_projection(projection)
                .map_err(AppError::from)
        })
    }
    fn draft() -> ExtrudeSpec {
        ExtrudeSpec {
            depth: ParameterValue::literal(newell_model::Length::millimetres(10.0).unwrap()),
            ..ExtrudeSpec::default()
        }
    }

    #[test]
    fn real_conversion_preserves_history_outputs_names_kinds_and_folder_parent() {
        let mut session = DocumentSession::default();
        let first = session.add_base_feature("empty".into(), vec![]).unwrap();
        let second = session
            .add_base_feature(
                "many".into(),
                [
                    ("surface", EntityKind::Surface, false),
                    ("body", EntityKind::Body, true),
                    (
                        "plane",
                        EntityKind::Construction(ConstructionKind::Plane),
                        true,
                    ),
                    (
                        "axis",
                        EntityKind::Construction(ConstructionKind::Axis),
                        false,
                    ),
                    (
                        "curve",
                        EntityKind::Construction(ConstructionKind::Curve),
                        true,
                    ),
                    (
                        "point",
                        EntityKind::Construction(ConstructionKind::Point),
                        false,
                    ),
                ]
                .into_iter()
                .map(|(name, kind, visible)| NewEntity {
                    name: name.into(),
                    kind,
                    visible,
                })
                .collect(),
            )
            .unwrap();
        let a = session.create_folder("A".into()).unwrap();
        let b = session.create_folder("B".into()).unwrap();
        session
            .set_history_layout(vec![
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
            ])
            .unwrap();
        let mut commands = CommandController::default();
        let inserted = session
            .confirm_before(&mut commands, draft(), Some(second))
            .unwrap()
            .unwrap();
        let projection = project_document(session.identity(), session.document()).unwrap();
        assert_eq!(
            projection
                .features
                .iter()
                .map(|item| (item.name.as_str(), item.depth))
                .collect::<Vec<_>>(),
            vec![
                ("A", 1),
                ("empty", 2),
                ("B", 1),
                ("Extrude", 2),
                ("many", 2)
            ]
        );
        assert_eq!(
            projection
                .elements
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "Extrude", "surface", "body", "plane", "axis", "curve", "point"
            ]
        );
        assert_eq!(projection.elements[0].source_feature.get(), inserted.get());
        assert_eq!(
            projection.elements[1..]
                .iter()
                .map(|item| item.output_ordinal)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4, 5]
        );
        assert_eq!(
            projection
                .elements
                .iter()
                .filter(|item| item.group == ElementGroup::Construct)
                .count(),
            4
        );
        assert_eq!(projection.labels.bodies, "Bodies");
        assert!(
            projection
                .features
                .iter()
                .all(|item| item.visibility.is_none())
        );
        let mut ui = ShellState::new_application(82, 30);
        ui.set_tree_projection(projection).unwrap();
        assert_eq!(ui.playhead(), newell_ui::PlacedHistoryGap::default());
    }

    #[test]
    fn notifications_follow_only_committed_mutations_and_reference_errors_are_atomic() {
        let mut session = DocumentSession::default();
        let base = session.add_base_solid("base").unwrap();
        let entity = session.document().feature(base).unwrap().outputs()[0];
        let later_feature = session.add_base_solid("later").unwrap();
        let later_entity = session.document().feature(later_feature).unwrap().outputs()[0];
        let events = session.subscribe();
        let mut command = CommandController::default();
        command.begin_extrude(&Selection::default());
        command.edit_extrude(draft(), session.document()).unwrap();
        let document = session.document().clone();
        let state = command.state().clone();
        let preview = command.preview().cloned();
        let revision = session.revision();
        for spec in [
            ExtrudeSpec {
                profile: Some(entity),
                ..draft()
            },
            ExtrudeSpec {
                profile: Some(EntityId::from_raw(999)),
                ..draft()
            },
            ExtrudeSpec {
                merge_scope: vec![entity],
                merge_with_all: true,
                ..draft()
            },
            ExtrudeSpec {
                merge_scope: vec![EntityId::from_raw(999)],
                ..draft()
            },
            ExtrudeSpec {
                merge_scope: vec![later_entity],
                merge_with_all: true,
                ..draft()
            },
        ] {
            assert!(
                session
                    .confirm_before(&mut command, spec, Some(base))
                    .is_err()
            );
            assert_eq!(session.document(), &document);
            assert_eq!(session.revision(), revision);
            assert_eq!(command.state(), &state);
            assert_eq!(command.preview(), preview.as_ref());
            assert!(events.try_recv().is_err());
        }
        let error = session
            .confirm_before(
                &mut command,
                ExtrudeSpec {
                    profile: Some(EntityId::from_raw(999)),
                    merge_scope: vec![entity],
                    ..draft()
                },
                Some(base),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            AppError::Model(ModelError::UnknownReferencedEntity {
                reference: ExtrudeReference::Profile,
                ..
            })
        ));
        let inserted = session
            .confirm_before(
                &mut command,
                ExtrudeSpec {
                    profile: Some(entity),
                    merge_scope: vec![entity, entity],
                    ..draft()
                },
                None,
            )
            .unwrap()
            .unwrap();
        assert_eq!(inserted.get(), 2);
        assert_eq!(events.try_iter().count(), 1);
        assert_eq!(session.revision(), revision + 1);
        let target = TreeTarget {
            document: session.identity().tree(),
            kind: TreeTargetKind::Entity(OpaqueTreeId::new(entity.get())),
        };
        assert!(
            !session
                .apply_tree_edit(
                    target,
                    &TreeIntentKind::Rename {
                        source_name: "base".into(),
                        name: "base".into()
                    }
                )
                .unwrap()
        );
        assert!(events.try_recv().is_err());
        assert!(
            session
                .apply_tree_edit(
                    target,
                    &TreeIntentKind::SetVisibility {
                        expected_visible: true,
                        visible: false
                    }
                )
                .unwrap()
        );
        assert!(
            !session
                .apply_tree_edit(
                    target,
                    &TreeIntentKind::SetVisibility {
                        expected_visible: true,
                        visible: false
                    }
                )
                .unwrap()
        );
        assert_eq!(events.try_iter().count(), 1);
    }

    #[test]
    fn failed_publication_retains_obligation_and_recovers_at_one_hundred_ms() {
        let mut session = DocumentSession::default();
        let events = session.subscribe();
        let mut sync = ProjectionSync::default();
        let mut shell = controller();
        let now = Instant::now();
        poll(
            &mut sync,
            &session,
            &events,
            &mut shell,
            now,
            ProjectionTrigger::Notification,
        )
        .unwrap();
        let old = shell.grid().clone();
        let generation = shell.generation();
        let acknowledged = sync.acknowledged;
        session.add_base_solid("new").unwrap();
        sync.fail_next_projection = 1;
        assert!(
            poll(
                &mut sync,
                &session,
                &events,
                &mut shell,
                now,
                ProjectionTrigger::Notification
            )
            .is_err()
        );
        assert!(events.try_recv().is_err(), "drain does not acknowledge");
        assert_eq!(sync.acknowledged, acknowledged);
        assert_eq!(shell.grid(), &old);
        assert_eq!(shell.generation(), generation);
        assert_eq!(sync.deadline(), Some(now + Duration::from_millis(100)));
        assert!(
            poll(
                &mut sync,
                &session,
                &events,
                &mut shell,
                now + Duration::from_millis(99),
                ProjectionTrigger::UserActivation
            )
            .unwrap()
            .is_none()
        );
        assert!(
            poll(
                &mut sync,
                &session,
                &events,
                &mut shell,
                now + Duration::from_millis(100),
                ProjectionTrigger::RetryDeadline
            )
            .unwrap()
            .unwrap()
            .grid
        );
        assert_eq!(
            sync.acknowledged,
            Some((session.identity(), session.revision()))
        );
        assert!(sync.pending.is_none());
        assert_ne!(shell.grid(), &old);
    }

    #[test]
    fn two_failures_suspend_until_a_specific_rearm_trigger() {
        for trigger in [
            ProjectionTrigger::UserActivation,
            ProjectionTrigger::LayoutPublished,
        ] {
            let mut session = DocumentSession::default();
            let events = session.subscribe();
            let mut sync = ProjectionSync {
                fail_next_projection: 2,
                ..ProjectionSync::default()
            };
            let mut shell = controller();
            let now = Instant::now();
            assert!(
                poll(
                    &mut sync,
                    &session,
                    &events,
                    &mut shell,
                    now,
                    ProjectionTrigger::Notification
                )
                .is_err()
            );
            assert!(
                poll(
                    &mut sync,
                    &session,
                    &events,
                    &mut shell,
                    now + Duration::from_millis(100),
                    ProjectionTrigger::RetryDeadline
                )
                .is_err()
            );
            assert!(sync.pending.as_ref().unwrap().suspended);
            assert!(sync.deadline().is_none());
            session.publish(DocumentNotification::Changed {
                document: session.identity(),
                revision: session.revision(),
            });
            for step in 1..10 {
                assert!(
                    poll(
                        &mut sync,
                        &session,
                        &events,
                        &mut shell,
                        now + Duration::from_secs(step),
                        ProjectionTrigger::RetryDeadline
                    )
                    .unwrap()
                    .is_none()
                );
            }
            assert!(
                poll(
                    &mut sync,
                    &session,
                    &events,
                    &mut shell,
                    now + Duration::from_secs(11),
                    trigger
                )
                .unwrap()
                .is_some()
            );
            assert!(sync.pending.is_none());
        }
    }

    #[test]
    fn revisions_coalesce_and_replacement_supersedes_old_deadlines_and_acks() {
        let mut session = DocumentSession::default();
        let events = session.subscribe();
        let mut shell = controller();
        let mut sync = ProjectionSync::default();
        let now = Instant::now();
        poll(
            &mut sync,
            &session,
            &events,
            &mut shell,
            now,
            ProjectionTrigger::Notification,
        )
        .unwrap();
        session.add_base_solid("one").unwrap();
        sync.fail_next_projection = 1;
        assert!(
            poll(
                &mut sync,
                &session,
                &events,
                &mut shell,
                now,
                ProjectionTrigger::Notification
            )
            .is_err()
        );
        session.add_base_solid("two").unwrap();
        session.add_base_solid("three").unwrap();
        poll(
            &mut sync,
            &session,
            &events,
            &mut shell,
            now + Duration::from_millis(10),
            ProjectionTrigger::Notification,
        )
        .unwrap();
        assert_eq!(sync.acknowledged, Some((session.identity(), 3)));
        assert!(sync.deadline().is_none());
        let old_identity = session.identity();
        session.replace(Document::default()).unwrap();
        session.publish(DocumentNotification::Changed {
            document: old_identity,
            revision: 999,
        });
        sync.fail_next_projection = 1;
        assert!(
            poll(
                &mut sync,
                &session,
                &events,
                &mut shell,
                now + Duration::from_millis(20),
                ProjectionTrigger::Notification
            )
            .is_err()
        );
        assert_eq!(sync.pending.as_ref().unwrap().document, session.identity());
        assert_eq!(sync.pending.as_ref().unwrap().revision, 0);
        poll(
            &mut sync,
            &session,
            &events,
            &mut shell,
            now + Duration::from_millis(120),
            ProjectionTrigger::RetryDeadline,
        )
        .unwrap();
        assert_eq!(sync.acknowledged, Some((session.identity(), 0)));
        assert_eq!(
            shell.shell().tree_projection().document,
            session.identity().tree()
        );
        assert!(shell.shell().tree_projection().features.is_empty());
    }
}
