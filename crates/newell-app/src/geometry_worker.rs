// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! One geometry thread, one newest waiting job and a bounded owned-result mailbox.
use crate::{AppError, DocumentInstanceId};
use newell_kernel::{NewellGeometry, NewellMesh, NewellShapeKey};
use newell_model::{
    Direction, Document, EntityId, ExtrudeGeometry, ExtrudeResult, ExtrudeTermination, Feature,
    FeatureId,
    geometry::{self, DocumentEdit, GeometryGeneration, PreparedDocument},
    parameters::ParameterDraft,
};
use newell_ui::CommandInputSessionId;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
};

/// Exact request lifetime, semantic source and latest-draft certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeometryJobId {
    pub document: DocumentInstanceId,
    pub session: CommandInputSessionId,
    pub base_revision: u64,
    pub draft_revision: u64,
    pub serial: u64,
    pub parameter_epoch: u64,
    pub source_generation: GeometryGeneration,
}
/// Owned preparation input. Native topology is constructed only after this reaches the worker.
#[derive(Debug, Clone)]
pub struct PrepareGeometry {
    pub id: GeometryJobId,
    pub base: Document,
    pub edit: DocumentEdit,
    pub insertion: Option<FeatureId>,
    pub parameters: ParameterDraft,
    pub cancelled: Arc<AtomicBool>,
}
/// Integer authority scoped to worker lifetime, document and finalized generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewellGeometryToken {
    epoch: u64,
    document: DocumentInstanceId,
    generation: GeometryGeneration,
}
impl NewellGeometryToken {
    /// Worker lifetime qualifying every integer in this token.
    #[must_use]
    pub const fn worker_epoch(self) -> u64 {
        self.epoch
    }
    /// Finalized model generation carried by this token.
    #[must_use]
    pub const fn generation(self) -> GeometryGeneration {
        self.generation
    }
    /// Checks the app document namespace and sealed generation before acceptance.
    #[must_use]
    pub fn matches(self, document: DocumentInstanceId, generation: GeometryGeneration) -> bool {
        self.document == document && self.generation == generation
    }
}
/// Picking uses committed ids only; preview bindings are a separate namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneItemBinding {
    Committed {
        entity: EntityId,
        generation: GeometryGeneration,
    },
    Preview {
        session: CommandInputSessionId,
        draft_revision: u64,
        feature: FeatureId,
        component: u32,
    },
}
/// One immutable item in the complete hypothetical scene.
#[derive(Debug, Clone)]
pub struct SceneItem {
    pub entity: EntityId,
    pub generation: GeometryGeneration,
    pub binding: SceneItemBinding,
    pub mesh: NewellMesh,
    pub selected: bool,
}
/// Complete owned scene, never a tool-only preview.
#[derive(Debug, Clone)]
pub struct SceneSnapshot {
    pub document: DocumentInstanceId,
    pub generation: GeometryGeneration,
    pub items: Vec<SceneItem>,
}
/// Finalized document authority paired with retained worker geometry and its scene.
#[derive(Debug)]
pub struct PreparedGeometry {
    pub id: GeometryJobId,
    pub prepared: PreparedDocument,
    pub token: NewellGeometryToken,
    pub scene: SceneSnapshot,
}
/// Owned diagnostics contain no UI/GPU or native topology payloads.
#[derive(Debug, thiserror::Error)]
pub enum GeometryFailure {
    #[error(transparent)]
    Model(#[from] newell_model::ModelError),
    #[error(transparent)]
    Kernel(#[from] newell_kernel::NewellKernelError),
    #[error("geometry transport: {0}")]
    Transport(String),
    #[error("geometry request cancelled")]
    Cancelled,
}
/// Completion contains finalized authority, never an unfinalized Document.
#[derive(Debug)]
pub enum WorkerReply {
    Ready(PreparedGeometry),
    Failed {
        id: GeometryJobId,
        error: GeometryFailure,
    },
}
#[derive(Debug, Default)]
struct Inbox {
    waiting: Option<PrepareGeometry>,
    reply: Option<WorkerReply>,
    retain: Option<NewellGeometryToken>,
    release: BTreeSet<u64>,
    replacement: Option<DocumentInstanceId>,
}
/// Nonblocking UI transport. Drop never waits for an uninterruptible kernel call.
#[derive(Debug)]
pub struct GeometryWorker {
    inbox: Arc<Mutex<Inbox>>,
    wake: SyncSender<()>,
    epoch: u64,
}
struct StagedGeneration {
    token: NewellGeometryToken,
    _geometry: NewellGeometry,
}
fn transport(message: &str) -> AppError {
    AppError::GeometryTransport(message.into())
}
impl GeometryWorker {
    /// Starts one dedicated worker. The wake callback normally posts a winit user event.
    ///
    /// # Errors
    /// Reports thread creation failure without changing the accepted document.
    pub fn start(
        epoch: u64,
        wake_completion: impl Fn() + Send + 'static,
    ) -> Result<Self, AppError> {
        if epoch == 0 {
            return Err(transport("invalid worker lifetime"));
        }
        let inbox = Arc::new(Mutex::new(Inbox::default()));
        let worker_inbox = Arc::clone(&inbox);
        let (wake, receiver) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("newell-geometry".into())
            .spawn(move || {
                let mut generations: BTreeMap<u64, StagedGeneration> = BTreeMap::new();
                let mut committed = None;
                let mut next_generation = 0u64;
                while receiver.recv().is_ok() {
                    let job = {
                        let Ok(mut inbox) = worker_inbox.lock() else {
                            break;
                        };
                        if let Some(document) = inbox.replacement.take() {
                            generations.retain(|_, state| state.token.document == document);
                            committed = committed
                                .filter(|token: &NewellGeometryToken| token.document == document);
                        }
                        if let Some(token) = inbox.retain.take()
                            && generations
                                .get(&token.generation.0)
                                .is_some_and(|state| state.token == token)
                        {
                            committed = Some(token);
                        }
                        for serial in std::mem::take(&mut inbox.release) {
                            if committed.is_none_or(|token| token.generation.0 != serial) {
                                generations.remove(&serial);
                            }
                        }
                        inbox.waiting.take()
                    };
                    let Some(job) = job else {
                        continue;
                    };
                    let id = job.id;
                    let prepared = next_generation
                        .max(job.base.geometry_generation().0)
                        .checked_add(1)
                        .ok_or_else(|| {
                            GeometryFailure::Transport("generation capacity exhausted".into())
                        })
                        .and_then(|next| {
                            next_generation = next;
                            prepare(&job, GeometryGeneration(next))
                        });
                    let reply = match prepared {
                        Ok((prepared, scene, geometry)) => {
                            let token = NewellGeometryToken {
                                epoch,
                                document: id.document,
                                generation: prepared.manifest().generation(),
                            };
                            let Ok(mut inbox) = worker_inbox.lock() else {
                                break;
                            };
                            if let Some(WorkerReply::Ready(old)) = inbox.reply.take() {
                                generations.remove(&old.token.generation.0);
                            }
                            if generations.len() >= 4 {
                                WorkerReply::Failed {
                                    id,
                                    error: GeometryFailure::Transport(
                                        "release superseded geometry before preparing more".into(),
                                    ),
                                }
                            } else {
                                generations.insert(
                                    token.generation.0,
                                    StagedGeneration {
                                        token,
                                        _geometry: geometry,
                                    },
                                );
                                WorkerReply::Ready(PreparedGeometry {
                                    id,
                                    prepared,
                                    token,
                                    scene,
                                })
                            }
                        }
                        Err(error) => WorkerReply::Failed { id, error },
                    };
                    let Ok(mut inbox) = worker_inbox.lock() else {
                        break;
                    };
                    if let Some(WorkerReply::Ready(old)) = inbox.reply.replace(reply) {
                        if committed != Some(old.token) {
                            generations.remove(&old.token.generation.0);
                        }
                    }
                    drop(inbox);
                    wake_completion();
                }
            })
            .map_err(|error| transport(&error.to_string()))?;
        Ok(Self { inbox, wake, epoch })
    }
    fn signal(&self) -> Result<(), AppError> {
        match self.wake.try_send(()) {
            Ok(()) | Err(TrySendError::Full(())) => Ok(()),
            Err(TrySendError::Disconnected(())) => Err(transport("worker has stopped")),
        }
    }
    /// Coalesces waiting work. Replaced inputs have never allocated native geometry.
    ///
    /// # Errors
    /// Reports a failed worker transport.
    pub fn submit(&self, job: PrepareGeometry) -> Result<(), AppError> {
        self.inbox
            .lock()
            .map_err(|_| transport("worker mailbox failed"))?
            .waiting = Some(job);
        self.signal()
    }
    /// Drains the newest completion once; waking never performs geometry on the UI thread.
    ///
    /// # Errors
    /// Reports mailbox poisoning rather than panicking.
    pub fn drain(&self) -> Result<Option<WorkerReply>, AppError> {
        Ok(self
            .inbox
            .lock()
            .map_err(|_| transport("worker mailbox failed"))?
            .reply
            .take())
    }
    /// Advises retention after the model has already accepted the finalized authority.
    ///
    /// # Errors
    /// Reports a wrong worker lifetime or unavailable transport.
    pub fn retain_committed(&self, token: NewellGeometryToken) -> Result<(), AppError> {
        if token.epoch != self.epoch {
            return Err(transport("wrong worker lifetime"));
        }
        self.inbox
            .lock()
            .map_err(|_| transport("worker mailbox failed"))?
            .retain = Some(token);
        self.signal()
    }
    /// Releases a superseded preview/result; committed generations remain retained.
    ///
    /// # Errors
    /// Rejects a foreign token or an overfull control mailbox.
    pub fn release(&self, token: NewellGeometryToken) -> Result<(), AppError> {
        if token.epoch != self.epoch {
            return Err(transport("wrong worker lifetime"));
        }
        let mut inbox = self
            .inbox
            .lock()
            .map_err(|_| transport("worker mailbox failed"))?;
        if inbox.release.len() >= 4 && !inbox.release.contains(&token.generation.0) {
            return Err(transport("release mailbox full"));
        }
        inbox.release.insert(token.generation.0);
        drop(inbox);
        self.signal()
    }
    /// Invalidates old document results without waiting for the active kernel call.
    ///
    /// # Errors
    /// Reports an unavailable mailbox or worker.
    pub fn replace_document(&self, document: DocumentInstanceId) -> Result<(), AppError> {
        let mut inbox = self
            .inbox
            .lock()
            .map_err(|_| transport("worker mailbox failed"))?;
        inbox.replacement = Some(document);
        if inbox
            .waiting
            .as_ref()
            .is_some_and(|job| job.id.document != document)
        {
            inbox.waiting = None;
        }
        if inbox.reply.as_ref().is_some_and(|reply| match reply {
            WorkerReply::Ready(ready) => ready.id.document != document,
            WorkerReply::Failed { id, .. } => id.document != document,
        }) {
            inbox.reply = None;
        }
        drop(inbox);
        self.signal()
    }
}
fn cancelled(job: &PrepareGeometry) -> Result<(), GeometryFailure> {
    if job.cancelled.load(Ordering::Acquire) {
        Err(GeometryFailure::Cancelled)
    } else {
        Ok(())
    }
}
fn description(face: newell_kernel::NewellProfileFace) -> geometry::ProfileFace {
    geometry::ProfileFace {
        face: face.face,
        frame: geometry::ProfileFrame {
            origin: face.frame.origin,
            normal: face.frame.normal,
            u: face.frame.u,
            v: face.frame.v,
        },
        wires: face
            .wires
            .into_iter()
            .map(|wire| geometry::ProfileWire {
                wire: wire.wire,
                closed: wire.closed,
                edges: wire
                    .edges
                    .into_iter()
                    .map(|edge| geometry::ProfileEdge {
                        edge: edge.edge,
                        start: edge.start,
                        end: edge.end,
                        straight: edge.straight,
                    })
                    .collect(),
            })
            .collect(),
    }
}
fn prepare(
    job: &PrepareGeometry,
    generation: GeometryGeneration,
) -> Result<(PreparedDocument, SceneSnapshot, NewellGeometry), GeometryFailure> {
    cancelled(job)?;
    if job.id.source_generation != job.base.geometry_generation()
        || job.id.parameter_epoch != job.parameters.registry().epoch()
    {
        return Err(newell_model::ModelError::StaleParameterDraft.into());
    }
    let candidate = job
        .base
        .prepare_edit(&job.edit, &job.parameters, job.insertion)?;
    let document = candidate.document();
    let mut geometry = NewellGeometry::new();
    let mut entities: BTreeMap<EntityId, NewellShapeKey> = BTreeMap::new();
    let mut feature_shapes = Vec::new();
    let mut results = Vec::new();
    for (feature_id, feature) in document.features() {
        cancelled(job)?;
        let mut shapes = Vec::new();
        match feature {
            Feature::Base { outputs, .. } => {
                for entity in outputs {
                    if let Some(geometry::BaseGeometryRecipe::Box { size }) =
                        document.base_geometry_recipe(*entity)
                    {
                        let shape = geometry
                            .box_shape(size.map(|length| length.as_millimetres()), [0.; 3])?;
                        entities.insert(*entity, shape);
                        shapes.push(shape);
                    }
                }
            }
            Feature::Extrude(extrude) => {
                let spec = &extrude.spec;
                // Foundation replay is deliberately inactive in App until all command modes
                // are connected; no metadata confirmation delegates to this limited path.
                if spec.geometry != ExtrudeGeometry::Solid
                    || spec.result != ExtrudeResult::New
                    || spec.termination != ExtrudeTermination::Blind
                {
                    return Err(GeometryFailure::Transport(
                        "this recipe is outside the current Blind/New replay increment".into(),
                    ));
                }
                let profile = spec
                    .profile_geometry
                    .ok_or(newell_model::ModelError::InvalidGeometryReference)?;
                let geometry::ProfileRef::Face(face) = profile else {
                    return Err(newell_model::ModelError::InvalidGeometryReference.into());
                };
                let source = *entities
                    .get(&face.entity)
                    .ok_or(newell_model::ModelError::InvalidGeometryReference)?;
                let depth = spec.depth.evaluate(document.parameters())?.as_millimetres();
                let start = spec
                    .starting_offset
                    .as_ref()
                    .map(|value| value.evaluate(document.parameters()))
                    .transpose()?
                    .map_or(0., |length| length.as_millimetres())
                    * if spec.directions.starting_offset == Direction::Reverse {
                        -1.
                    } else {
                        1.
                    };
                let adjustment = spec
                    .end_offset
                    .as_ref()
                    .map(|value| value.evaluate(document.parameters()))
                    .transpose()?
                    .map_or(0., |length| length.as_millimetres())
                    * if spec.directions.end_offset == Direction::Reverse {
                        -1.
                    } else {
                        1.
                    };
                let frame = geometry
                    .profile_faces(source)?
                    .into_iter()
                    .find(|value| value.face == face.face)
                    .ok_or(newell_model::ModelError::InvalidGeometryReference)?
                    .frame;
                let along = if spec.directions.depth == Direction::Reverse {
                    Some(frame.normal.map(|v| -v))
                } else {
                    None
                };
                let reach = depth + adjustment;
                let shape = geometry.extrude_face(
                    source,
                    face.face,
                    along,
                    if spec.symmetric { start - reach } else { start },
                    start + reach,
                )?;
                for entity in &extrude.outputs {
                    entities.insert(*entity, shape);
                }
                shapes.push(shape);
            }
        }
        let components = shapes
            .iter()
            .map(|shape| {
                Ok(geometry::GeometryComponent {
                    kind: newell_model::EntityKind::Body,
                    faces: geometry
                        .profile_faces(*shape)?
                        .into_iter()
                        .map(description)
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, GeometryFailure>>()?;
        results.push(geometry::GeometryFeatureResult {
            feature: *feature_id,
            components,
        });
        feature_shapes.push((*feature_id, shapes));
    }
    cancelled(job)?;
    let prepared = candidate.finalize(geometry::GeometryResultManifest {
        edit: job.edit.clone(),
        generation,
        features: results,
    })?;
    let mut items = Vec::new();
    for (feature, shapes) in feature_shapes {
        let outputs = prepared
            .manifest()
            .outputs()
            .get(&feature)
            .ok_or(newell_model::ModelError::InvalidGeometryManifest)?;
        let outputs: Vec<_> = outputs
            .iter()
            .copied()
            .filter(|entity| prepared.document().entity_geometry(*entity).is_some())
            .collect();
        if outputs.len() != shapes.len() {
            return Err(newell_model::ModelError::InvalidGeometryManifest.into());
        }
        for (component, (entity, shape)) in outputs.into_iter().zip(shapes).enumerate() {
            cancelled(job)?;
            let description = prepared
                .document()
                .entity_geometry(entity)
                .ok_or(newell_model::ModelError::InvalidGeometryManifest)?;
            let committed = job
                .base
                .entity_geometry(entity)
                .is_some_and(|source| source.generation == description.generation);
            let binding = if committed {
                SceneItemBinding::Committed {
                    entity,
                    generation: description.generation,
                }
            } else {
                SceneItemBinding::Preview {
                    session: job.id.session,
                    draft_revision: job.id.draft_revision,
                    feature,
                    component: u32::try_from(component).map_err(|_| {
                        GeometryFailure::Transport("component capacity exhausted".into())
                    })?,
                }
            };
            items.push(SceneItem {
                entity,
                generation: description.generation,
                binding,
                mesh: geometry.tessellate(shape, 0.05)?,
                selected: false,
            });
        }
    }
    Ok((
        prepared,
        SceneSnapshot {
            document: job.id.document,
            generation,
            items,
        },
        geometry,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DocumentSession;
    use std::time::Duration;
    fn job(
        session: &DocumentSession,
        serial: u64,
        edit: DocumentEdit,
        parameters: ParameterDraft,
    ) -> PrepareGeometry {
        PrepareGeometry {
            id: GeometryJobId {
                document: session.identity(),
                session: CommandInputSessionId(1),
                base_revision: session.revision(),
                draft_revision: serial,
                serial,
                parameter_epoch: parameters.registry().epoch(),
                source_generation: session.document().geometry_generation(),
            },
            base: session.document().clone(),
            edit,
            insertion: None,
            parameters,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
    fn ready(
        worker: &GeometryWorker,
        wake: &mpsc::Receiver<()>,
    ) -> Result<PreparedGeometry, Box<dyn std::error::Error>> {
        wake.recv_timeout(Duration::from_secs(10))?;
        match worker.drain()?.ok_or("missing worker completion")? {
            WorkerReply::Ready(ready) => Ok(ready),
            WorkerReply::Failed { error, .. } => Err(error.into()),
        }
    }
    #[test]
    fn messages_are_send_without_any_topology_payload() {
        fn is_send<T: Send>() {}
        is_send::<PrepareGeometry>();
        is_send::<WorkerReply>();
        is_send::<PreparedDocument>();
    }
    #[test]
    fn real_worker_finalizes_base_and_named_local_extrude_before_one_acceptance()
    -> Result<(), Box<dyn std::error::Error>> {
        let (sender, wake) = mpsc::channel();
        let worker = GeometryWorker::start(1, move || {
            let _ = sender.send(());
        })?;
        let mut session = DocumentSession::default();
        let base = session.add_base_solid("Base Solid")?;
        session.bind_geometry_worker(1)?;
        let parameters = session
            .document()
            .begin_parameter_draft(newell_model::parameters::ParameterDraftOwner::Document)?;
        worker.submit(job(
            &session,
            1,
            DocumentEdit::ParameterDefinitions,
            parameters,
        ))?;
        let initial = ready(&worker, &wake)?;
        assert_eq!(initial.scene.items.len(), 1);
        assert_eq!(initial.scene.items[0].mesh.face_count(), 6);
        let first_token = initial.token;
        let receipt =
            session.accept_prepared(session.version(), initial.prepared, initial.token)?;
        assert_eq!(receipt.feature, None);
        worker.retain_committed(first_token)?;
        let entity = session
            .document()
            .feature(base)
            .ok_or("base missing")?
            .outputs()[0];
        let geometry = session
            .document()
            .entity_geometry(entity)
            .ok_or("geometry missing")?;
        let face = geometry
            .faces
            .iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("top face missing")?;
        let reference = geometry::ProfileRef::Face(geometry::FaceRef {
            entity,
            generation: geometry.generation,
            face: face.face,
        });
        let mut parameters = session.document().begin_parameter_draft(
            newell_model::parameters::ParameterDraftOwner::InsertExtrude { before: None },
        )?;
        let owner = parameters.feature_owner();
        let definition = |source: &str| newell_model::parameters::ParameterEdit {
            parameter: None,
            owner: newell_model::parameters::ParameterOwner::Document,
            source: source.into(),
            destination: newell_model::QuantityKind::Length,
            source_edit: None,
            source_origins: None,
        };
        let depth = parameters.stage_edit(definition("#Depth := 3mm"))?;
        let start = parameters.stage_edit(definition("#Depth/2"))?;
        let spec = newell_model::ExtrudeSpec {
            profile_geometry: Some(reference),
            depth: newell_model::ParameterValue::Bound(depth),
            starting_offset: Some(newell_model::ParameterValue::Bound(start)),
            ..newell_model::ExtrudeSpec::default()
        };
        let edit = DocumentEdit::ExtrudeInsert {
            spec,
            validation: geometry::ExtrudeValidationRefs::default(),
        };
        let unchanged = session.document().clone();
        let version = session.version();
        let notifications = session.subscribe();
        worker.submit(job(&session, 2, edit, parameters))?;
        let result = ready(&worker, &wake)?;
        assert_eq!(session.document(), &unchanged);
        assert_eq!(result.scene.items.len(), 2);
        assert!(matches!(
            result.scene.items[0].binding,
            SceneItemBinding::Committed { .. }
        ));
        assert!(matches!(
            result.scene.items[1].binding,
            SceneItemBinding::Preview { .. }
        ));
        let positions = result.scene.items[1].mesh.positions();
        let min = positions.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min);
        let max = positions
            .iter()
            .map(|p| p[2])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((min - 2.5).abs() < 1e-6);
        assert!((max - 5.5).abs() < 1e-6);
        let duplicate = result.prepared.clone();
        let token = result.token;
        let receipt = session.accept_prepared(version, result.prepared, token)?;
        assert_eq!(receipt.feature, owner);
        assert_eq!(receipt.outputs.len(), 1);
        assert_eq!(session.revision(), version.revision + 1);
        assert!(session.accept_prepared(version, duplicate, token).is_err());
        assert_eq!(notifications.try_iter().count(), 1);
        assert_eq!(session.geometry_token(), Some(token));
        worker.retain_committed(token)?;
        worker.release(first_token)?;
        Ok(())
    }
    #[test]
    fn cancelled_work_cannot_finalize_a_document() -> Result<(), Box<dyn std::error::Error>> {
        let (sender, wake) = mpsc::channel();
        let worker = GeometryWorker::start(1, move || {
            let _ = sender.send(());
        })?;
        let session = DocumentSession::default();
        let parameters = session
            .document()
            .begin_parameter_draft(newell_model::parameters::ParameterDraftOwner::Document)?;
        let request = job(&session, 1, DocumentEdit::ParameterDefinitions, parameters);
        request.cancelled.store(true, Ordering::Release);
        worker.submit(request)?;
        wake.recv_timeout(Duration::from_secs(10))?;
        assert!(matches!(
            worker.drain()?,
            Some(WorkerReply::Failed {
                error: GeometryFailure::Cancelled,
                ..
            })
        ));
        assert_eq!(session.revision(), 0);
        Ok(())
    }
}
