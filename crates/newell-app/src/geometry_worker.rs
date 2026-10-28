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
    pub worker_epoch: u64,
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
    authority: newell_model::geometry::PreparedAuthority,
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
    /// Binds acceptance to the exact finalized result retained in this native stage.
    #[must_use]
    pub fn authorizes(self, prepared: &PreparedDocument) -> bool {
        self.authority == prepared.authority()
            && self.generation == prepared.manifest().generation()
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
    pub visible: bool,
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
    Ready(Box<PreparedGeometry>),
    Failed {
        id: GeometryJobId,
        error: GeometryFailure,
    },
}
#[derive(Debug, Default)]
struct Inbox {
    #[cfg(test)]
    shutdown: bool,
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
    alive: Arc<AtomicBool>,
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
        let alive = Arc::new(AtomicBool::new(true));
        let worker_alive = Arc::clone(&alive);
        let (wake, receiver) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("newell-geometry".into())
            .spawn(move || {
                struct Exit<'a> { alive: &'a AtomicBool, wake: &'a dyn Fn() }
                impl Drop for Exit<'_> {
                    fn drop(&mut self) { self.alive.store(false, Ordering::Release); (self.wake)(); }
                }
                let _exit = Exit { alive: &worker_alive, wake: &wake_completion };
                let mut generations: BTreeMap<u64, StagedGeneration> = BTreeMap::new();
                let mut committed = None;
                let mut next_generation = 0u64;
                while receiver.recv().is_ok() {
                    let job = {
                        let Ok(mut inbox) = worker_inbox.lock() else {
                            break;
                        };
                        #[cfg(test)]
                        if inbox.shutdown { break; }
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
                                authority: prepared.authority(),
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
                                WorkerReply::Ready(Box::new(PreparedGeometry {
                                    id,
                                    prepared,
                                    token,
                                    scene,
                                }))
                            }
                        }
                        Err(error) => WorkerReply::Failed { id, error },
                    };
                    let Ok(mut inbox) = worker_inbox.lock() else {
                        break;
                    };
                    if let Some(WorkerReply::Ready(old)) = inbox.reply.replace(reply)
                        && committed != Some(old.token)
                    {
                        generations.remove(&old.token.generation.0);
                    }
                    drop(inbox);
                    wake_completion();
                }
            })
            .map_err(|error| transport(&error.to_string()))?;
        Ok(Self { inbox, wake, epoch, alive })
    }
    /// Worker lifetime is observable without waiting for an active kernel call.
    pub fn is_alive(&self) -> bool { self.alive.load(Ordering::Acquire) }
    #[cfg(test)]
    pub(crate) fn stop_for_test(&self) -> Result<(), AppError> {
        self.inbox.lock().map_err(|_| transport("worker mailbox failed"))?.shutdown = true;
        self.signal()
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
        if job.id.worker_epoch != self.epoch {
            return Err(transport("wrong job worker lifetime"));
        }
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
        if !self.is_alive() { return Err(transport("worker has stopped; retry the command")); }
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
        provenance: match face.provenance {
            newell_kernel::NewellFaceProvenance::Unknown => geometry::FaceProvenance::Unknown,
            newell_kernel::NewellFaceProvenance::Base(key) => geometry::FaceProvenance::Base(key),
            newell_kernel::NewellFaceProvenance::StartCap => geometry::FaceProvenance::StartCap,
            newell_kernel::NewellFaceProvenance::EndCap => geometry::FaceProvenance::EndCap,
            newell_kernel::NewellFaceProvenance::Wall { wire, edge } => {
                geometry::FaceProvenance::Wall { wire, edge }
            }
        },
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
fn replayed_face(
    reference: geometry::FaceRef,
    document: &Document,
    entities: &BTreeMap<EntityId, NewellShapeKey>,
    native: &NewellGeometry,
) -> Result<(NewellShapeKey, u32), GeometryFailure> {
    let old = document
        .entity_geometry(reference.entity)
        .ok_or(newell_model::ModelError::InvalidGeometryReference)?
        .resolve_profile(geometry::ProfileRef::Face(reference), true)?;
    let key = *entities
        .get(&reference.entity)
        .ok_or(newell_model::ModelError::InvalidGeometryReference)?;
    let mut matches = Vec::new();
    for face in native.profile_faces(key)? {
        let face = description(face);
        if old.provenance != geometry::FaceProvenance::Unknown {
            if old.provenance == face.provenance {
                matches.push(face.face);
            }
        } else if old == &face {
            matches.push(face.face);
        }
    }
    if matches.len() != 1 {
        return Err(newell_model::ModelError::InvalidGeometryReference.into());
    }
    Ok((key, matches[0]))
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
    let mut active: BTreeMap<geometry::GeometryOutputRef, NewellShapeKey> = BTreeMap::new();
    let mut origins: BTreeMap<EntityId, geometry::GeometryOutputRef> = BTreeMap::new();
    let mut feature_shapes = Vec::new();
    let mut results = Vec::new();
    for (feature_id, feature) in document.features() {
        cancelled(job)?;
        let mut shapes = Vec::new();
        let mut consumed = Vec::new();
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
                let spec = extrude.spec.inputs();
                let profile_reference = spec
                    .profile_geometry
                    .ok_or(newell_model::ModelError::InvalidGeometryReference)?;
                let face = profile_reference.support();
                if spec.geometry == ExtrudeGeometry::Solid
                    && !matches!(profile_reference, geometry::ProfileRef::Face(_))
                {
                    return Err(newell_model::ModelError::InvalidGeometryReference.into());
                }
                let (source, face_index) = replayed_face(face, document, &entities, &geometry)?;
                let quantity =
                    |value: Option<&newell_model::ParameterValue>| -> Result<f64, GeometryFailure> {
                        Ok(value
                            .map(|value| value.evaluate(document.parameters()))
                            .transpose()?
                            .map_or(0., |length| length.as_millimetres()))
                    };
                let frame = geometry
                    .profile_faces(source)?
                    .into_iter()
                    .find(|value| value.face == face_index)
                    .ok_or(newell_model::ModelError::InvalidGeometryReference)?
                    .frame;
                let mut direction = if let Some(reference) = spec.along_geometry {
                    let (source, face) =
                        replayed_face(reference.support, document, &entities, &geometry)?;
                    geometry.direction(source, face, reference.wire, reference.edge)?
                } else {
                    frame.normal
                };
                if (spec.termination == ExtrudeTermination::Blind
                    && spec.directions.depth == Direction::Reverse)
                    || (spec.termination == ExtrudeTermination::ThroughAll && spec.invert)
                {
                    direction = direction.map(|v| -v);
                }
                let profile = (source, face_index);
                let start = if spec.geometry == ExtrudeGeometry::Solid
                    && let Some(target) = spec.targets.start
                {
                    geometry.starting_station(
                        profile,
                        direction,
                        replayed_face(target, document, &entities, &geometry)?,
                    )?
                } else {
                    quantity(spec.starting_offset.as_ref())?
                        * if spec.directions.starting_offset == Direction::Reverse {
                            -1.
                        } else {
                            1.
                        }
                };
                if spec.geometry == ExtrudeGeometry::Surface {
                    use newell_kernel::{
                        NewellFaceTarget, NewellSurfaceExtent as Extent,
                        NewellSurfaceSides as Sides, NewellSurfaceStart as Start,
                    };
                    let target =
                        |reference, clearance| -> Result<NewellFaceTarget, GeometryFailure> {
                            let (shape, face) =
                                replayed_face(reference, document, &entities, &geometry)?;
                            Ok(NewellFaceTarget {
                                shape,
                                face,
                                clearance,
                            })
                        };
                    let start = if let Some(reference) = spec.targets.start {
                        Start::UpTo(target(reference, 0.)?)
                    } else {
                        Start::Distance(start)
                    };
                    let extent = if let Some(reference) = spec.targets.end.or(spec.targets.main) {
                        Extent::UpTo(target(reference, quantity(spec.main_clearance.as_ref())?)?)
                    } else if spec.termination == ExtrudeTermination::ThroughAll {
                        Extent::ThroughAll(active.values().copied().collect())
                    } else {
                        Extent::Blind(
                            quantity(Some(&spec.depth))?
                                + quantity(spec.end_offset.as_ref())?
                                    * if spec.directions.end_offset == Direction::Reverse {
                                        -1.
                                    } else {
                                        1.
                                    },
                        )
                    };
                    let sides = if spec.symmetric {
                        Sides::Symmetric
                    } else if let Some(reference) = spec.targets.nested.or(spec.targets.second) {
                        Sides::Second {
                            extent: Extent::UpTo(target(
                                reference,
                                quantity(spec.second_end_offset.as_ref())?,
                            )?),
                            reverse: false,
                        }
                    } else if let Some(value) = &spec.second_end {
                        Sides::Second {
                            extent: Extent::Blind(quantity(Some(value))?),
                            reverse: spec.directions.second_end == Direction::Reverse,
                        }
                    } else {
                        Sides::One
                    };
                    let boundary = match profile_reference {
                        geometry::ProfileRef::Face(_) => newell_kernel::NewellBoundary::Face,
                        geometry::ProfileRef::Wire { wire, .. } => {
                            newell_kernel::NewellBoundary::Wire(wire)
                        }
                        geometry::ProfileRef::Edge { wire, edge, .. } => {
                            newell_kernel::NewellBoundary::Edge { wire, edge }
                        }
                    };
                    shapes = geometry.extrude_surface(newell_kernel::NewellSurfaceExtrude {
                        profile,
                        boundary,
                        direction,
                        start,
                        extent,
                        sides,
                    })?;
                } else {
                    let target = spec.targets.end.or(spec.targets.main);
                    let mut shape = if let Some(target) = target {
                        geometry.extrude_to_face(
                            profile,
                            direction,
                            start,
                            replayed_face(target, document, &entities, &geometry)?,
                            quantity(spec.main_clearance.as_ref())?,
                        )?
                    } else if spec.termination == ExtrudeTermination::ThroughAll {
                        let targets: Vec<_> = active.values().copied().collect();
                        let end =
                            geometry.through_all_station(profile, direction, start, &targets)?;
                        geometry.extrude_face(source, face_index, Some(direction), start, end)?
                    } else {
                        let depth = quantity(Some(&spec.depth))?;
                        let adjustment = quantity(spec.end_offset.as_ref())?
                            * if spec.directions.end_offset == Direction::Reverse {
                                -1.
                            } else {
                                1.
                            };
                        let reach = depth + adjustment;
                        if reach <= 1e-7 {
                            return Err(newell_model::ModelError::NonPositiveDepth.into());
                        }
                        geometry.extrude_face(
                            source,
                            face_index,
                            Some(direction),
                            start,
                            start + reach,
                        )?
                    };
                    let second = if spec.symmetric {
                        Some(geometry.mirror_sweep(shape, profile, direction, start)?)
                    } else if let Some(target) = spec.targets.nested.or(spec.targets.second) {
                        Some(geometry.extrude_to_face(
                            profile,
                            direction.map(|v| -v),
                            -start,
                            replayed_face(target, document, &entities, &geometry)?,
                            quantity(spec.second_end_offset.as_ref())?,
                        )?)
                    } else if let Some(second) = &spec.second_end {
                        let reach = quantity(Some(second))?;
                        let second_direction = if spec.directions.second_end == Direction::Reverse {
                            direction
                        } else {
                            direction.map(|v| -v)
                        };
                        let second_start = if spec.directions.second_end == Direction::Reverse {
                            start
                        } else {
                            -start
                        };
                        Some(geometry.extrude_face(
                            source,
                            face_index,
                            Some(second_direction),
                            second_start,
                            second_start + reach,
                        )?)
                    } else {
                        None
                    };
                    if let Some(second) = second {
                        shape = geometry
                            .boolean(shape, second, newell_kernel::NewellBooleanMode::Add)?
                            .ok_or(newell_model::ModelError::InvalidGeometryManifest)?;
                    }
                    shapes.push(shape);
                }
                if spec.result == ExtrudeResult::New {
                    for shape in &shapes {
                        geometry.tag_extrude(*shape, (source, face_index), direction)?;
                    }
                }
                if spec.result != ExtrudeResult::New {
                    let surface = spec.geometry == ExtrudeGeometry::Surface;
                    let targets: Vec<_> = if spec.merge_with_all {
                        active
                            .iter()
                            .filter(|(_, key)| geometry.is_surface(**key) == surface)
                            .map(|(reference, key)| (*reference, *key))
                            .collect()
                    } else {
                        let mut targets = Vec::new();
                        for entity in &spec.merge_scope {
                            let reference = *origins
                                .get(entity)
                                .ok_or(newell_model::ModelError::InvalidGeometryReference)?;
                            let key = *active
                                .get(&reference)
                                .ok_or(newell_model::ModelError::InvalidGeometryReference)?;
                            if geometry.is_surface(key) != surface {
                                return Err(
                                    newell_model::ModelError::InvalidGeometryReference.into()
                                );
                            }
                            targets.push((reference, key));
                        }
                        targets
                    };
                    let keys: Vec<_> = targets.iter().map(|(_, key)| *key).collect();
                    let mode = if spec.result == ExtrudeResult::Add {
                        newell_kernel::NewellBooleanMode::Add
                    } else {
                        newell_kernel::NewellBooleanMode::Remove
                    };
                    let (outputs, changed) = if surface {
                        let result = geometry.combine_surfaces(&shapes, &keys, mode)?;
                        (result.outputs, result.changed_targets)
                    } else {
                        if shapes.len() != 1 {
                            return Err(newell_model::ModelError::InvalidGeometryManifest.into());
                        }
                        let result = geometry.combine_bodies(shapes[0], &keys, mode)?;
                        (result.outputs, result.changed_targets)
                    };
                    for index in changed {
                        let reference = targets
                            .get(index)
                            .ok_or(newell_model::ModelError::InvalidGeometryManifest)?
                            .0;
                        active.remove(&reference);
                        consumed.push(reference);
                    }
                    shapes = outputs;
                }
                for (entity, shape) in extrude.outputs.iter().zip(&shapes) {
                    entities.insert(*entity, *shape);
                }
            }
        }
        for (component, shape) in shapes.iter().enumerate() {
            let ordinal = if let Feature::Base { outputs, .. } = feature {
                let entity = outputs
                    .iter()
                    .filter(|entity| document.base_geometry_recipe(**entity).is_some())
                    .nth(component)
                    .ok_or(newell_model::ModelError::InvalidGeometryManifest)?;
                outputs
                    .iter()
                    .position(|output| output == entity)
                    .ok_or(newell_model::ModelError::InvalidGeometryManifest)?
            } else {
                component
            };
            let reference = geometry::GeometryOutputRef {
                feature: *feature_id,
                component: u32::try_from(ordinal).map_err(|_| {
                    GeometryFailure::Transport("component capacity exhausted".into())
                })?,
            };
            active.insert(reference, *shape);
            if let Some(entity) = feature.outputs().get(ordinal) {
                origins.insert(*entity, reference);
            }
        }
        let components = shapes
            .iter()
            .map(|shape| {
                Ok(geometry::GeometryComponent {
                    kind: if geometry.is_surface(*shape) {
                        newell_model::EntityKind::Surface
                    } else {
                        newell_model::EntityKind::Body
                    },
                    measure: if geometry.is_surface(*shape) {
                        geometry.surface_area(*shape)?
                    } else {
                        geometry.volume(*shape)?
                    },
                    face_count: u32::try_from(geometry.face_count(*shape)?).map_err(|_| {
                        GeometryFailure::Transport("face count capacity exhausted".into())
                    })?,
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
            consumed,
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
    let active = prepared.document().active_entities_at(None)?;
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
            if !active.contains(&entity) {
                continue;
            }
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
                visible: prepared
                    .document()
                    .entity(entity)
                    .is_some_and(|entity| entity.visible()),
            });
        }
    }
    let references: Vec<_> = items.iter().map(|item| &item.mesh).collect();
    NewellMesh::compose(&references)?;
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
                worker_epoch: 1,
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
            WorkerReply::Ready(ready) => Ok(*ready),
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
    fn token_rejects_another_finalized_result_with_the_same_document_and_generation()
    -> Result<(), Box<dyn std::error::Error>> {
        let (sender, wake) = mpsc::channel();
        let worker = GeometryWorker::start(1, move || {
            let _ = sender.send(());
        })?;
        let mut session = DocumentSession::default();
        session.add_base_solid("Base Solid")?;
        session.bind_geometry_worker(1)?;
        let parameters = session
            .document()
            .begin_parameter_draft(newell_model::parameters::ParameterDraftOwner::Document)?;
        let request = job(&session, 1, DocumentEdit::ParameterDefinitions, parameters);
        worker.submit(request.clone())?;
        let initial = ready(&worker, &wake)?;
        // A separate, valid finalization can use the same source and generation.
        // Its native stage is not the one retained behind the returned token.
        let (substitute, _, _) = prepare(&request, initial.token.generation())?;
        assert_eq!(initial.prepared.document(), substitute.document());
        assert_ne!(initial.prepared.authority(), substitute.authority());
        let source = session.document().clone();
        let version = session.version();
        let events = session.subscribe();
        assert!(
            session
                .accept_prepared(version, substitute, initial.token)
                .is_err()
        );
        assert_eq!(session.document(), &source);
        assert_eq!(session.version(), version);
        assert_eq!(session.geometry_token(), None);
        assert_eq!(events.try_iter().count(), 0);
        session.accept_prepared(version, initial.prepared, initial.token)?;
        assert_eq!(session.geometry_token(), Some(initial.token));
        assert_eq!(events.try_iter().count(), 1);
        Ok(())
    }
    #[test]
    fn real_worker_finalizes_base_and_named_local_extrude_before_one_acceptance()
    -> Result<(), Box<dyn std::error::Error>> {
        let (sender, wake) = mpsc::channel();
        let worker = GeometryWorker::start(1, move || {
            let _ = sender.send(());
        })?;
        let mut session = DocumentSession::default();
        let base = session.add_seeded_solid("Base Solid")?;
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
        let spec = newell_model::ExtrudeInputProjection {
            profile_geometry: Some(reference),
            depth: newell_model::ParameterValue::Bound(depth),
            starting_offset: Some(newell_model::ParameterValue::Bound(start)),
            ..newell_model::ExtrudeInputProjection::default()
        }
        .into_spec()
        .unwrap();
        let edit = DocumentEdit::ExtrudeInsert {
            spec,
            validation: geometry::ExtrudeValidationRefs { retained_merge_scope: vec![entity,entity] },
        };
        let unchanged = session.document().clone();
        let version = session.version();
        let notifications = session.subscribe();
        let mut missing = edit.clone();
        if let DocumentEdit::ExtrudeInsert { validation, .. } = &mut missing {
            validation.retained_merge_scope.push(EntityId::from_raw(u64::MAX));
        }
        assert!(session.document().prepare_edit(&missing,&parameters,None).is_err());
        let frozen = session.document().prepare_edit(&edit,&parameters,None)?;
        worker.submit(job(&session, 2, edit.clone(), parameters))?;
        let result = ready(&worker, &wake)?;
        assert_eq!(result.prepared.manifest().edit(), &edit);
        let inventory = result.prepared.document().features().iter().map(|(feature,value)| {
            let components = value.outputs().iter().map(|entity| {
                let geometry = result.prepared.document().entity_geometry(*entity).ok_or("missing actual geometry")?;
                Ok(geometry::GeometryComponent { kind:geometry.kind, faces:geometry.faces.clone(), measure:geometry.measure,face_count:geometry.face_count })
            }).collect::<Result<Vec<_>, &str>>()?;
            Ok(geometry::GeometryFeatureResult { feature:*feature,consumed:Vec::new(),components })
        }).collect::<Result<Vec<_>, &str>>()?;
        let manifest = geometry::GeometryResultManifest { edit:edit.clone(),generation:result.token.generation(),features:inventory };
        // Establish that this is a valid, real kernel inventory before altering
        // only the frozen validation payload supplied to finalization.
        assert_eq!(frozen.clone().finalize(manifest.clone())?.document(), result.prepared.document());
        let mut altered = manifest;
        if let DocumentEdit::ExtrudeInsert { validation,.. } = &mut altered.edit { validation.retained_merge_scope.clear(); }
        assert!(matches!(frozen.finalize(altered),Err(newell_model::ModelError::InvalidGeometryManifest)));
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
