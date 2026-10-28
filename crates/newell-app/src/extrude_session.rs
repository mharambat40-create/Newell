// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! App-owned preparation lifetime. NumericDelivery remains the sole terminal-outcome owner.
use crate::{
    AppError, DocumentVersion,
    geometry_worker::{
        GeometryJobId, NewellGeometryToken, PrepareGeometry, PreparedGeometry, WorkerReply,
    },
};
use newell_model::{
    Document, FeatureId,
    geometry::{DocumentEdit, GeometryGeneration},
    parameters::ParameterDraft,
};
use newell_ui::{CommandInputSessionId, NumericConfirmRequest};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Geometry work purpose; an explicit command OK is never inferred from a preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationPurpose {
    Preview,
    Confirm(NumericConfirmRequest),
}
/// A completion's meaning after all lifetime and semantic source checks.
#[derive(Debug)]
pub enum PreparationEvent {
    Preview(Box<PreparedGeometry>),
    Confirm {
        request: NumericConfirmRequest,
        ready: Box<PreparedGeometry>,
    },
    Rejected {
        request: Option<NumericConfirmRequest>,
        diagnostic: String,
    },
    Release(NewellGeometryToken),
    Ignored,
}
/// Single command lifetime and latest preparation. It owns no kernel handles or GPU objects.
#[derive(Debug)]
pub struct ExtrudeGeometrySession {
    source: DocumentVersion,
    command: CommandInputSessionId,
    worker_epoch: u64,
    source_generation: GeometryGeneration,
    insertion: Option<FeatureId>,
    draft_revision: u64,
    serial: u64,
    parameter_epoch: u64,
    cancellation: Arc<AtomicBool>,
    pending: Option<(GeometryJobId, PreparationPurpose)>,
    ready: Option<Box<PreparedGeometry>>,
    closed: bool,
}
impl ExtrudeGeometrySession {
    /// Captures the accepted source and insertion boundary for one new command.
    #[must_use]
    pub fn new(
        source: DocumentVersion,
        command: CommandInputSessionId,
        worker_epoch: u64,
        source_generation: GeometryGeneration,
        insertion: Option<FeatureId>,
        parameter_epoch: u64,
    ) -> Self {
        Self {
            source,
            command,
            worker_epoch,
            source_generation,
            insertion,
            draft_revision: 0,
            serial: 0,
            parameter_epoch,
            cancellation: Arc::new(AtomicBool::new(false)),
            pending: None,
            ready: None,
            closed: false,
        }
    }
    /// Invalidates earlier preparations only after an accepted semantic edit.
    ///
    /// # Errors
    /// Reports lifetime/counter exhaustion before changing any certificate.
    pub fn advance_draft(
        &mut self,
        parameter_epoch: u64,
    ) -> Result<Option<NewellGeometryToken>, AppError> {
        if self.closed {
            return Err(AppError::InvalidNumericRequest);
        }
        let next = self
            .draft_revision
            .checked_add(1)
            .ok_or(AppError::DocumentCounterExhausted)?;
        self.cancellation.store(true, Ordering::Release);
        self.cancellation = Arc::new(AtomicBool::new(false));
        self.draft_revision = next;
        self.parameter_epoch = parameter_epoch;
        self.pending = None;
        Ok(self.ready.take().map(|ready| ready.token))
    }
    /// Freezes the exact draft and numeric attempt; this performs no geometry or model commit.
    ///
    /// # Errors
    /// Rejects stale documents/reservations, exhausted identities and wrong OK namespaces.
    pub fn freeze(
        &mut self,
        current: DocumentVersion,
        base: &Document,
        edit: DocumentEdit,
        parameters: &ParameterDraft,
        purpose: PreparationPurpose,
    ) -> Result<PrepareGeometry, AppError> {
        if self.closed
            || current != self.source
            || base.geometry_generation() != self.source_generation
            || parameters.registry().epoch() != self.parameter_epoch
        {
            return Err(AppError::InvalidNumericRequest);
        }
        parameters.validate_source(base)?;
        if let PreparationPurpose::Confirm(request) = purpose
            && (request.document != self.source.instance.tree() || request.session != self.command)
        {
            return Err(AppError::InvalidNumericRequest);
        }
        let serial = self
            .serial
            .checked_add(1)
            .ok_or(AppError::DocumentCounterExhausted)?;
        let id = GeometryJobId {
            worker_epoch: self.worker_epoch,
            document: self.source.instance,
            session: self.command,
            base_revision: self.source.revision,
            draft_revision: self.draft_revision,
            serial,
            parameter_epoch: self.parameter_epoch,
            source_generation: self.source_generation,
        };
        self.serial = serial;
        self.pending = Some((id, purpose));
        Ok(PrepareGeometry {
            id,
            base: base.clone(),
            edit,
            insertion: self.insertion,
            parameters: parameters.clone(),
            cancelled: Arc::clone(&self.cancellation),
        })
    }
    fn eligible(&self, id: GeometryJobId, current: DocumentVersion) -> bool {
        !self.closed
            && current == self.source
            && self.pending.is_some_and(|(expected, _)| expected == id)
            && id.worker_epoch == self.worker_epoch
            && id.document == self.source.instance
            && id.session == self.command
            && id.base_revision == self.source.revision
            && id.draft_revision == self.draft_revision
            && id.serial == self.serial
            && id.parameter_epoch == self.parameter_epoch
            && id.source_generation == self.source_generation
    }
    /// Resolves a worker completion under the current accepted version, ignoring stale errors too.
    #[must_use]
    pub fn receive(&mut self, reply: WorkerReply, current: DocumentVersion) -> PreparationEvent {
        match reply {
            WorkerReply::Ready(ready) => {
                if !self.eligible(ready.id, current)
                    || !ready.token.authorizes(&ready.prepared)
                    || ready.token.worker_epoch() != self.worker_epoch
                    || !ready
                        .token
                        .matches(self.source.instance, ready.prepared.manifest().generation())
                {
                    return PreparationEvent::Release(ready.token);
                }
                let Some((_, purpose)) = self.pending.take() else {
                    return PreparationEvent::Release(ready.token);
                };
                match purpose {
                    PreparationPurpose::Preview => PreparationEvent::Preview(ready),
                    PreparationPurpose::Confirm(request) => {
                        PreparationEvent::Confirm { request, ready }
                    }
                }
            }
            WorkerReply::Failed { id, error } => {
                if !self.eligible(id, current) {
                    return PreparationEvent::Ignored;
                }
                let request = self.pending.take().and_then(|(_, purpose)| match purpose {
                    PreparationPurpose::Confirm(request) => Some(request),
                    PreparationPurpose::Preview => None,
                });
                PreparationEvent::Rejected {
                    request,
                    diagnostic: error.to_string(),
                }
            }
        }
    }
    /// Retains a current successful preview after its complete scene was published.
    ///
    /// # Errors
    /// Rejects a stale preview without replacing the previous ready authority.
    pub fn retain_preview(
        &mut self,
        ready: Box<PreparedGeometry>,
        current: DocumentVersion,
    ) -> Result<Option<NewellGeometryToken>, AppError> {
        if self.closed
            || !ready.token.authorizes(&ready.prepared)
            || ready.token.worker_epoch() != self.worker_epoch
            || !ready
                .token
                .matches(self.source.instance, ready.prepared.manifest().generation())
            || current != self.source
            || ready.id.document != self.source.instance
            || ready.id.session != self.command
            || ready.id.base_revision != self.source.revision
            || ready.id.draft_revision != self.draft_revision
            || ready.id.serial != self.serial
            || ready.id.parameter_epoch != self.parameter_epoch
            || ready.id.source_generation != self.source_generation
        {
            return Err(AppError::InvalidNumericRequest);
        }
        Ok(self.ready.replace(ready).map(|old| old.token))
    }
    /// Reuses exact current preview authority for OK; callers still consume/check the numeric barrier.
    pub fn take_ready(&mut self, current: DocumentVersion) -> Option<Box<PreparedGeometry>> {
        if self.closed || current != self.source {
            return None;
        }
        self.ready.take()
    }
    /// Tears down an uncommitted lifetime immediately, without waiting for native work.
    pub fn cancel(&mut self) -> Option<NewellGeometryToken> {
        self.cancellation.store(true, Ordering::Release);
        self.closed = true;
        self.pending = None;
        self.ready.take().map(|ready| ready.token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocumentSession, geometry_worker::GeometryFailure};
    #[test]
    fn newer_ready_wins_and_late_ready_is_released_without_replacing_the_preview()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::geometry_worker::GeometryWorker;
        use std::{sync::mpsc, time::Duration};
        let mut document = DocumentSession::default();
        document.add_base_solid("Base Solid")?;
        let parameters = document
            .document()
            .begin_parameter_draft(newell_model::parameters::ParameterDraftOwner::Document)?;
        let mut session = ExtrudeGeometrySession::new(
            document.version(),
            CommandInputSessionId(1),
            1,
            document.document().geometry_generation(),
            None,
            parameters.registry().epoch(),
        );
        let (sender, wake) = mpsc::channel();
        let worker = GeometryWorker::start(1, move || {
            let _ = sender.send(());
        })?;
        let prepare = |session: &mut ExtrudeGeometrySession| -> Result<WorkerReply, Box<dyn std::error::Error>> {
            let request = session.freeze(document.version(), document.document(),
                DocumentEdit::ParameterDefinitions, &parameters, PreparationPurpose::Preview)?;
            worker.submit(request)?;
            wake.recv_timeout(Duration::from_secs(10))?;
            Ok(worker.drain()?.ok_or("missing reply")?)
        };
        let old = prepare(&mut session)?;
        let old_token = match &old {
            WorkerReply::Ready(ready) => ready.token,
            _ => return Err("old job failed".into()),
        };
        session.advance_draft(parameters.registry().epoch())?;
        let new = prepare(&mut session)?;
        let PreparationEvent::Preview(ready) = session.receive(new, document.version()) else {
            return Err("current preview rejected".into());
        };
        let new_token = ready.token;
        session.retain_preview(ready, document.version())?;
        assert!(
            matches!(session.receive(old, document.version()), PreparationEvent::Release(token) if token == old_token)
        );
        worker.release(old_token)?;
        let current = session
            .take_ready(document.version())
            .ok_or("lost current preview")?;
        assert_eq!(current.token, new_token);
        session.retain_preview(current, document.version())?;
        assert_eq!(session.cancel(), Some(new_token));
        assert!(session.take_ready(document.version()).is_none());
        worker.release(new_token)?;
        assert_eq!(document.document().features().len(), 1);
        Ok(())
    }
    #[test]
    fn stale_failures_and_document_changes_cannot_replace_current_command_state()
    -> Result<(), AppError> {
        let mut document = DocumentSession::default();
        let parameters = document
            .document()
            .begin_parameter_draft(newell_model::parameters::ParameterDraftOwner::Document)?;
        let mut session = ExtrudeGeometrySession::new(
            document.version(),
            CommandInputSessionId(1),
            1,
            document.document().geometry_generation(),
            None,
            0,
        );
        let a = session.freeze(
            document.version(),
            document.document(),
            DocumentEdit::ParameterDefinitions,
            &parameters,
            PreparationPurpose::Preview,
        )?;
        session.advance_draft(0)?;
        let b = session.freeze(
            document.version(),
            document.document(),
            DocumentEdit::ParameterDefinitions,
            &parameters,
            PreparationPurpose::Preview,
        )?;
        assert!(a.cancelled.load(Ordering::Acquire));
        assert!(!b.cancelled.load(Ordering::Acquire));
        assert!(matches!(
            session.receive(
                WorkerReply::Failed {
                    id: a.id,
                    error: GeometryFailure::Cancelled
                },
                document.version()
            ),
            PreparationEvent::Ignored
        ));
        document.add_base_solid("changed")?;
        assert!(matches!(
            session.receive(
                WorkerReply::Failed {
                    id: b.id,
                    error: GeometryFailure::Cancelled
                },
                document.version()
            ),
            PreparationEvent::Ignored
        ));
        assert!(
            session
                .freeze(
                    document.version(),
                    document.document(),
                    DocumentEdit::ParameterDefinitions,
                    &parameters,
                    PreparationPurpose::Preview
                )
                .is_err()
        );
        session.cancel();
        assert!(b.cancelled.load(Ordering::Acquire));
        Ok(())
    }
}

/// Per-window runtime state; worker topology and terminal UI delivery remain separate owners.
#[derive(Debug)]
pub(crate) struct GeometryRuntime {
    pub worker: crate::geometry_worker::GeometryWorker,
    pub coordinator: Option<ExtrudeGeometrySession>,
    pub latest_edit: Option<(DocumentEdit, u64)>,
    pub bootstrap: Option<GeometryJobId>,
    pub bootstrap_serial: u64,
    pub epoch: u64,
    pub committed_scene: Option<crate::geometry_worker::SceneSnapshot>,
    pub displayed_scene: Option<crate::geometry_worker::SceneSnapshot>,
}
impl GeometryRuntime {
    pub fn new(worker: crate::geometry_worker::GeometryWorker, epoch: u64) -> Self {
        Self {
            worker,
            coordinator: None,
            latest_edit: None,
            bootstrap: None,
            bootstrap_serial: 0,
            epoch,
            committed_scene: None,
            displayed_scene: None,
        }
    }
}
impl ExtrudeGeometrySession {
    pub(crate) fn transport_failed(&mut self) -> Option<NumericConfirmRequest> {
        self.cancellation.store(true, Ordering::Release);
        self.ready = None;
        self.pending.take().and_then(|(_, purpose)| match purpose {
            PreparationPurpose::Confirm(request) => Some(request), PreparationPurpose::Preview => None,
        })
    }
    pub(crate) fn computing(&self) -> bool {
        self.pending.is_some()
    }
    pub(crate) fn confirm_pending(&self) -> bool {
        self.pending
            .is_some_and(|(_, purpose)| matches!(purpose, PreparationPurpose::Confirm(_)))
    }
    pub(crate) fn has_work(&self) -> bool {
        self.computing() || self.ready.is_some()
    }
    pub(crate) fn invalidate_preview(&mut self) -> Option<NewellGeometryToken> {
        self.cancellation.store(true, Ordering::Release);
        self.cancellation = Arc::new(AtomicBool::new(false));
        self.pending = None;
        self.ready.take().map(|ready| ready.token)
    }
}
