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
    #[cfg(test)]
    pub(crate) kernel_probe: Option<KernelCallProbe>,
}
#[cfg(test)]
#[derive(Debug, Clone)]
pub(crate) struct KernelCallProbe {
    entered: mpsc::Sender<std::time::Instant>,
    returned: mpsc::Sender<std::time::Instant>,
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
                struct Exit<'a> {
                    alive: &'a AtomicBool,
                    wake: &'a dyn Fn(),
                }
                impl Drop for Exit<'_> {
                    fn drop(&mut self) {
                        self.alive.store(false, Ordering::Release);
                        (self.wake)();
                    }
                }
                let _exit = Exit {
                    alive: &worker_alive,
                    wake: &wake_completion,
                };
                let mut generations: BTreeMap<u64, StagedGeneration> = BTreeMap::new();
                let mut committed = None;
                let mut next_generation = 0u64;
                while receiver.recv().is_ok() {
                    let job = {
                        let Ok(mut inbox) = worker_inbox.lock() else {
                            break;
                        };
                        #[cfg(test)]
                        if inbox.shutdown {
                            break;
                        }
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
                    let prepared = if job.cancelled.load(Ordering::Acquire) {
                        Err(GeometryFailure::Cancelled)
                    } else {
                        prepared
                    };
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
        Ok(Self {
            inbox,
            wake,
            epoch,
            alive,
        })
    }
    /// Worker lifetime is observable without waiting for an active kernel call.
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }
    #[cfg(test)]
    pub(crate) fn stop_for_test(&self) -> Result<(), AppError> {
        self.inbox
            .lock()
            .map_err(|_| transport("worker mailbox failed"))?
            .shutdown = true;
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
        if !self.is_alive() {
            return Err(transport("worker has stopped; retry the command"));
        }
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
    geometry.set_cancellation_flag(Arc::clone(&job.cancelled));
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
                    #[cfg(test)]
                    match document.base_geometry_recipe(*entity) {
                        Some(geometry::BaseGeometryRecipe::TestCubic {
                            control,
                            depth,
                            transform,
                        }) => {
                            let shape = geometry.cubic_shape_for_test(
                                *control,
                                depth.as_millimetres(),
                                *transform,
                            )?;
                            entities.insert(*entity, shape);
                            shapes.push(shape);
                        }
                        Some(geometry::BaseGeometryRecipe::TestBox { size, transform }) => {
                            let shape =
                                geometry.box_shape(size.map(|v| v.as_millimetres()), [0.; 3])?;
                            geometry.transform_shape_for_test(shape, *transform)?;
                            entities.insert(*entity, shape);
                            shapes.push(shape);
                        }
                        _ => {}
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
                        #[cfg(test)]
                        if let Some(probe) = &job.kernel_probe {
                            let _ = probe.entered.send(std::time::Instant::now());
                        }
                        let result = geometry.combine_surfaces(&shapes, &keys, mode);
                        #[cfg(test)]
                        if let Some(probe) = &job.kernel_probe {
                            let _ = probe.returned.send(std::time::Instant::now());
                        }
                        cancelled(job)?;
                        let result = result?;
                        (result.outputs, result.changed_targets)
                    } else {
                        if shapes.len() != 1 {
                            return Err(newell_model::ModelError::InvalidGeometryManifest.into());
                        }
                        #[cfg(test)]
                        if let Some(probe) = &job.kernel_probe {
                            let _ = probe.entered.send(std::time::Instant::now());
                        }
                        let result =
                            geometry.combine_bodies_cancellable(shapes[0], &keys, mode, || {
                                job.cancelled.load(Ordering::Acquire)
                            });
                        #[cfg(test)]
                        if let Some(probe) = &job.kernel_probe {
                            let _ = probe.returned.send(std::time::Instant::now());
                        }
                        cancelled(job)?;
                        let result = result?;
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
                    source_wire: geometry.source_wire(*shape),
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
    cancelled(job)?;
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
    #[test]
    fn insertion_uses_prefix_and_deduplicates_operands_without_dropping_validation()
    -> Result<(), Box<dyn std::error::Error>> {
        use newell_model::{
            Direction, ExtentSpec, ExtrudeResult, ExtrudeSpec, Length, MergePolicy, ParameterValue,
            geometry::{ExtrudeValidationRefs, FaceRef, ProfileRef},
            parameters::ParameterDraftOwner,
        };
        let (sender, wake) = mpsc::channel();
        let worker = GeometryWorker::start(1, move || {
            let _ = sender.send(());
        })?;
        let mut session = DocumentSession::default();
        let base = session.add_base_solid("Base Solid")?;
        session.bind_geometry_worker(1)?;
        let parameters = session
            .document()
            .begin_parameter_draft(ParameterDraftOwner::Document)?;
        worker.submit(job(
            &session,
            1,
            DocumentEdit::ParameterDefinitions,
            parameters,
        ))?;
        let initial = ready(&worker, &wake)?;
        session.accept_prepared(session.version(), initial.prepared, initial.token)?;
        worker.retain_committed(initial.token)?;
        let entity = session.document().feature(base).ok_or("base")?.outputs()[0];
        let geometry = session
            .document()
            .entity_geometry(entity)
            .ok_or("geometry")?;
        let profile = ProfileRef::Face(FaceRef {
            entity,
            generation: geometry.generation,
            face: geometry
                .faces
                .iter()
                .find(|face| face.frame.normal == [0., 0., -1.])
                .ok_or("bottom")?
                .face,
        });
        let blind = |depth| ExtentSpec::Blind {
            depth: ParameterValue::literal(Length::millimetres(depth).unwrap()),
            direction: Direction::Forward,
            along: None,
            start: None,
            end: None,
        };
        let parameters = session
            .document()
            .begin_parameter_draft(ParameterDraftOwner::InsertExtrude { before: None })?;
        let later = parameters.feature_owner().ok_or("later")?;
        worker.submit(job(
            &session,
            2,
            DocumentEdit::ExtrudeInsert {
                spec: ExtrudeSpec {
                    profile: Some(profile),
                    extent: blind(5.),
                    ..ExtrudeSpec::default()
                },
                validation: ExtrudeValidationRefs::default(),
            },
            parameters,
        ))?;
        let result = ready(&worker, &wake)?;
        session.accept_prepared(session.version(), result.prepared, result.token)?;
        worker.retain_committed(result.token)?;
        let later_entity = session.document().feature(later).ok_or("later")?.outputs()[0];
        // Refresh the generation-scoped reference after the accepted replay.
        let profile = ProfileRef::Face(FaceRef {
            generation: session
                .document()
                .entity_geometry(entity)
                .ok_or("base")?
                .generation,
            ..profile.support()
        });
        let source = session.document().clone();
        let version = session.version();
        let token = session.geometry_token();
        let events = session.subscribe();
        let parameters =
            session
                .document()
                .begin_parameter_draft(ParameterDraftOwner::InsertExtrude {
                    before: Some(later),
                })?;
        let through = ExtrudeSpec {
            profile: Some(profile),
            extent: ExtentSpec::ThroughAll {
                invert: false,
                start: None,
            },
            ..ExtrudeSpec::default()
        };
        let mut request = job(
            &session,
            3,
            DocumentEdit::ExtrudeInsert {
                spec: through.clone(),
                validation: ExtrudeValidationRefs::default(),
            },
            parameters.clone(),
        );
        request.insertion = Some(later);
        worker.submit(request)?;
        wake.recv_timeout(Duration::from_secs(10))?;
        assert!(matches!(worker.drain()?, Some(WorkerReply::Failed { .. })));
        // Even an inactive field-27 reference cannot reach forward into the suffix.
        let invalid = DocumentEdit::ExtrudeInsert {
            spec: through,
            validation: ExtrudeValidationRefs {
                retained_merge_scope: vec![later_entity],
            },
        };
        assert!(
            session
                .document()
                .prepare_edit(&invalid, &parameters, Some(later))
                .is_err()
        );
        assert_eq!(session.document(), &source);
        assert_eq!(session.version(), version);
        assert_eq!(session.geometry_token(), token);
        assert_eq!(events.try_iter().count(), 0);
        let inserted = parameters.feature_owner().ok_or("inserted")?;
        let edit = DocumentEdit::ExtrudeInsert {
            spec: ExtrudeSpec {
                profile: Some(profile),
                extent: blind(2.),
                result: ExtrudeResult::Add,
                targets: MergePolicy::Explicit(vec![entity, entity]),
                ..ExtrudeSpec::default()
            },
            validation: ExtrudeValidationRefs {
                retained_merge_scope: vec![entity, entity],
            },
        };
        let mut request = job(&session, 4, edit.clone(), parameters);
        request.insertion = Some(later);
        worker.submit(request)?;
        let result = ready(&worker, &wake)?;
        assert_eq!(result.prepared.manifest().edit(), &edit);
        let newell_model::Feature::Extrude(feature) = result
            .prepared
            .document()
            .feature(inserted)
            .ok_or("inserted")?
        else {
            return Err("wrong feature".into());
        };
        assert_eq!(feature.spec.targets, MergePolicy::Explicit(vec![entity]));
        let output = feature.outputs[0];
        assert!(
            (result
                .prepared
                .document()
                .entity_geometry(output)
                .ok_or("union")?
                .measure
                - 3.)
                .abs()
                < 1e-6
        );
        session.accept_prepared(version, result.prepared, result.token)?;
        assert_eq!(
            session
                .document()
                .features()
                .iter()
                .map(|(id, _)| *id)
                .collect::<Vec<_>>(),
            vec![base, inserted, later]
        );
        assert_eq!(session.document().active_entities_at(None)?.len(), 2);
        assert_eq!(
            session.document().feature(later).ok_or("later")?.outputs(),
            &[later_entity]
        );
        assert_eq!(events.try_iter().count(), 1);
        Ok(())
    }
    #[test]
    fn cap_edge_replay_keeps_unique_identities_and_lost_suffix_is_atomic()
    -> Result<(), Box<dyn std::error::Error>> {
        use newell_model::{
            Direction, ExtentSpec, ExtrudeGeometry, ExtrudeSpec, ParameterValue,
            geometry::{ExtrudeValidationRefs, FaceProvenance, FaceRef, ProfileRef},
            parameters::ParameterDraftOwner,
        };
        let (sender, wake) = mpsc::channel();
        let worker = GeometryWorker::start(1, move || {
            let _ = sender.send(());
        })?;
        let mut session = DocumentSession::default();
        let base = session.add_base_solid("Base Solid")?;
        session.bind_geometry_worker(1)?;
        let commit = |session: &mut DocumentSession,
                      request: PrepareGeometry|
         -> Result<(), Box<dyn std::error::Error>> {
            worker.submit(request)?;
            let result = ready(&worker, &wake)?;
            let old = session.geometry_token();
            let token = result.token;
            session.accept_prepared(session.version(), result.prepared, token)?;
            worker.retain_committed(token)?;
            if let Some(old) = old {
                worker.release(old)?;
            }
            Ok(())
        };
        let parameters = session
            .document()
            .begin_parameter_draft(ParameterDraftOwner::Document)?;
        let request = job(&session, 1, DocumentEdit::ParameterDefinitions, parameters);
        commit(&mut session, request)?;
        let base_entity = session.document().feature(base).ok_or("base")?.outputs()[0];
        let geometry = session
            .document()
            .entity_geometry(base_entity)
            .ok_or("base geometry")?;
        let original = ProfileRef::Face(FaceRef {
            entity: base_entity,
            generation: geometry.generation,
            face: geometry
                .faces
                .iter()
                .find(|face| face.frame.normal == [0., 0., -1.])
                .ok_or("bottom")?
                .face,
        });
        let mut parameters = session
            .document()
            .begin_parameter_draft(ParameterDraftOwner::InsertExtrude { before: None })?;
        let parameter = parameters.stage_edit(newell_model::parameters::ParameterEdit {
            parameter: None,
            owner: newell_model::parameters::ParameterOwner::Document,
            source: "#Depth := 3mm".into(),
            destination: newell_model::QuantityKind::Length,
            source_edit: None,
            source_origins: None,
        })?;
        let first = parameters.feature_owner().ok_or("first owner")?;
        let spec = ExtrudeSpec {
            profile: Some(original),
            extent: ExtentSpec::Blind {
                depth: ParameterValue::Bound(parameter),
                direction: Direction::Forward,
                along: None,
                start: None,
                end: None,
            },
            ..ExtrudeSpec::default()
        };
        let request = job(
            &session,
            2,
            DocumentEdit::ExtrudeInsert {
                spec,
                validation: ExtrudeValidationRefs::default(),
            },
            parameters,
        );
        commit(&mut session, request)?;
        let first_entity = session.document().feature(first).ok_or("first")?.outputs()[0];
        let geometry = session
            .document()
            .entity_geometry(first_entity)
            .ok_or("first geometry")?;
        let cap = geometry
            .faces
            .iter()
            .find(|face| face.provenance == FaceProvenance::EndCap)
            .ok_or("cap")?;
        let boundary = ProfileRef::Edge {
            support: FaceRef {
                entity: first_entity,
                generation: geometry.generation,
                face: cap.face,
            },
            wire: cap.wires[0].wire,
            edge: cap.wires[0].edges[0].edge,
        };
        let parameters = session
            .document()
            .begin_parameter_draft(ParameterDraftOwner::InsertExtrude { before: None })?;
        let second = parameters.feature_owner().ok_or("second owner")?;
        let spec = ExtrudeSpec {
            profile: Some(boundary),
            geometry: ExtrudeGeometry::Surface,
            extent: ExtentSpec::Blind {
                depth: ParameterValue::literal(newell_model::Length::millimetres(2.)?),
                direction: Direction::Forward,
                along: None,
                start: None,
                end: None,
            },
            ..ExtrudeSpec::default()
        };
        let request = job(
            &session,
            3,
            DocumentEdit::ExtrudeInsert {
                spec,
                validation: ExtrudeValidationRefs::default(),
            },
            parameters,
        );
        commit(&mut session, request)?;
        let second_entity = session
            .document()
            .feature(second)
            .ok_or("second")?
            .outputs()[0];
        let mut parameters = session
            .document()
            .begin_parameter_draft(ParameterDraftOwner::EditExtrude(first))?;
        let edited_parameter = parameters.stage_edit(newell_model::parameters::ParameterEdit {
            parameter: Some(parameter),
            owner: newell_model::parameters::ParameterOwner::Document,
            source: "#Depth := 4.5mm".into(),
            destination: newell_model::QuantityKind::Length,
            source_edit: None,
            source_origins: None,
        })?;
        assert_eq!(parameter, edited_parameter);
        let newell_model::Feature::Extrude(feature) =
            session.document().feature(first).ok_or("first")?
        else {
            return Err("wrong feature".into());
        };
        let spec = feature.spec.clone();
        let revision = session.revision();
        let events = session.subscribe();
        let request = job(
            &session,
            4,
            DocumentEdit::ExtrudeReplace {
                feature: first,
                spec,
                validation: ExtrudeValidationRefs::default(),
            },
            parameters,
        );
        commit(&mut session, request)?;
        assert_eq!(session.revision(), revision + 1);
        assert_eq!(events.try_iter().count(), 1);
        assert_eq!(
            session.document().feature(first).ok_or("first")?.outputs(),
            &[first_entity]
        );
        assert_eq!(
            session
                .document()
                .feature(second)
                .ok_or("second")?
                .outputs(),
            &[second_entity]
        );
        let newell_model::Feature::Extrude(feature) =
            session.document().feature(second).ok_or("second")?
        else {
            return Err("wrong suffix".into());
        };
        assert_eq!(
            feature.spec.profile.ok_or("profile")?.support().generation,
            session
                .document()
                .entity_geometry(first_entity)
                .ok_or("geometry")?
                .generation
        );
        assert!(
            (session
                .document()
                .entity_geometry(second_entity)
                .ok_or("sheet")?
                .measure
                - 2.)
                .abs()
                < 1e-6
        );
        let source = session.document().clone();
        let version = session.version();
        let token = session.geometry_token();
        let events = session.subscribe();
        let geometry = session
            .document()
            .entity_geometry(base_entity)
            .ok_or("base")?;
        let other = ProfileRef::Face(FaceRef {
            entity: base_entity,
            generation: geometry.generation,
            face: geometry
                .faces
                .iter()
                .find(|face| face.frame.normal == [0., 0., 1.])
                .ok_or("top")?
                .face,
        });
        let newell_model::Feature::Extrude(feature) =
            session.document().feature(first).ok_or("first")?
        else {
            return Err("wrong first".into());
        };
        let mut spec = feature.spec.clone();
        spec.profile = Some(other);
        let parameters = session
            .document()
            .begin_parameter_draft(ParameterDraftOwner::EditExtrude(first))?;
        worker.submit(job(
            &session,
            5,
            DocumentEdit::ExtrudeReplace {
                feature: first,
                spec,
                validation: ExtrudeValidationRefs::default(),
            },
            parameters,
        ))?;
        wake.recv_timeout(Duration::from_secs(10))?;
        assert!(matches!(worker.drain()?, Some(WorkerReply::Failed { .. })));
        assert_eq!(session.document(), &source);
        assert_eq!(session.version(), version);
        assert_eq!(session.geometry_token(), token);
        assert_eq!(events.try_iter().count(), 0);
        Ok(())
    }
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
            kernel_probe: None,
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
            merge_with_all: false,
            merge_scope: vec![entity, entity],
            ..newell_model::ExtrudeInputProjection::default()
        }
        .into_spec()
        .unwrap();
        let edit = DocumentEdit::ExtrudeInsert {
            spec,
            validation: geometry::ExtrudeValidationRefs {
                retained_merge_scope: vec![entity, entity],
            },
        };
        let unchanged = session.document().clone();
        let version = session.version();
        let notifications = session.subscribe();
        let mut missing = edit.clone();
        if let DocumentEdit::ExtrudeInsert { validation, .. } = &mut missing {
            validation
                .retained_merge_scope
                .push(EntityId::from_raw(u64::MAX));
        }
        assert!(
            session
                .document()
                .prepare_edit(&missing, &parameters, None)
                .is_err()
        );
        let frozen = session.document().prepare_edit(&edit, &parameters, None)?;
        worker.submit(job(&session, 2, edit.clone(), parameters))?;
        let result = ready(&worker, &wake)?;
        assert_eq!(result.prepared.manifest().edit(), &edit);
        let newell_model::Feature::Extrude(feature) = result
            .prepared
            .document()
            .feature(owner.ok_or("owner")?)
            .ok_or("feature")?
        else {
            return Err("wrong feature".into());
        };
        assert_eq!(feature.spec.targets, newell_model::MergePolicy::All);
        let inventory = result
            .prepared
            .document()
            .features()
            .iter()
            .map(|(feature, value)| {
                let components = value
                    .outputs()
                    .iter()
                    .map(|entity| {
                        let geometry = result
                            .prepared
                            .document()
                            .entity_geometry(*entity)
                            .ok_or("missing actual geometry")?;
                        Ok(geometry::GeometryComponent {
                            source_wire: geometry.source_wire,
                            kind: geometry.kind,
                            faces: geometry.faces.clone(),
                            measure: geometry.measure,
                            face_count: geometry.face_count,
                        })
                    })
                    .collect::<Result<Vec<_>, &str>>()?;
                Ok(geometry::GeometryFeatureResult {
                    feature: *feature,
                    consumed: Vec::new(),
                    components,
                })
            })
            .collect::<Result<Vec<_>, &str>>()?;
        let manifest = geometry::GeometryResultManifest {
            edit: edit.clone(),
            generation: result.token.generation(),
            features: inventory,
        };
        // Establish that this is a valid, real kernel inventory before altering
        // only the frozen validation payload supplied to finalization.
        assert_eq!(
            frozen.clone().finalize(manifest.clone())?.document(),
            result.prepared.document()
        );
        let mut altered = manifest;
        if let DocumentEdit::ExtrudeInsert { validation, .. } = &mut altered.edit {
            validation.retained_merge_scope.clear();
        }
        assert!(matches!(
            frozen.finalize(altered),
            Err(newell_model::ModelError::InvalidGeometryManifest)
        ));
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
    fn cancellation_during_native_boolean_returns_promptly_rejects_late_work_and_recovers()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::extrude_session::{
            ExtrudeGeometrySession, PreparationEvent, PreparationPurpose,
        };
        use newell_model::{
            Direction, ExtentSpec, ExtrudeResult, ExtrudeSpec, Length, ParameterValue,
            geometry::{ExtrudeValidationRefs, FaceRef, ProfileRef},
            parameters::ParameterDraftOwner,
        };
        use std::time::Instant;
        let (sender, wake) = mpsc::channel();
        let worker = GeometryWorker::start(1, move || {
            let _ = sender.send(());
        })?;
        let mut session = DocumentSession::default();
        let first = session.add_base_solid("Base Solid")?;
        // Actual overlapping B-Reps force the production exact Boolean operand loop.
        for i in 0..127 {
            session.add_base_solid(&format!("Target {i}"))?;
        }
        session.bind_geometry_worker(1)?;
        let parameters = session
            .document()
            .begin_parameter_draft(ParameterDraftOwner::Document)?;
        worker.submit(job(
            &session,
            1,
            DocumentEdit::ParameterDefinitions,
            parameters,
        ))?;
        let initial = ready(&worker, &wake)?;
        session.accept_prepared(session.version(), initial.prepared, initial.token)?;
        worker.retain_committed(initial.token)?;
        let entity = session.document().feature(first).ok_or("base")?.outputs()[0];
        let shape = session
            .document()
            .entity_geometry(entity)
            .ok_or("geometry")?;
        let profile = ProfileRef::Face(FaceRef {
            entity,
            generation: shape.generation,
            face: shape
                .faces
                .iter()
                .find(|f| f.frame.normal == [0., 0., -1.])
                .ok_or("bottom")?
                .face,
        });
        let parameters = session
            .document()
            .begin_parameter_draft(ParameterDraftOwner::InsertExtrude { before: None })?;
        let spec = ExtrudeSpec {
            profile: Some(profile),
            result: ExtrudeResult::Add,
            extent: ExtentSpec::Blind {
                depth: ParameterValue::literal(Length::millimetres(2.)?),
                direction: Direction::Forward,
                along: None,
                start: None,
                end: None,
            },
            ..ExtrudeSpec::default()
        };
        let mut command = ExtrudeGeometrySession::new(
            session.version(),
            CommandInputSessionId(2),
            1,
            session.document().geometry_generation(),
            None,
            parameters.registry().epoch(),
        );
        let mut request = command.freeze(
            session.version(),
            session.document(),
            DocumentEdit::ExtrudeInsert {
                spec,
                validation: ExtrudeValidationRefs::default(),
            },
            &parameters,
            PreparationPurpose::Preview,
        )?;
        let flag = Arc::clone(&request.cancelled);
        let (entered, entering) = mpsc::channel();
        let (returned, returning) = mpsc::channel();
        request.kernel_probe = Some(KernelCallProbe { entered, returned });
        let unchanged = session.document().clone();
        let version = session.version();
        let notifications = session.subscribe();
        worker.submit(request)?;
        let begin = entering.recv_timeout(Duration::from_secs(10))?;
        std::thread::sleep(Duration::from_millis(5));
        assert!(
            matches!(returning.try_recv(), Err(mpsc::TryRecvError::Empty)),
            "native Boolean workload finished before cancellation; measurement invalid"
        );
        let cancelling = Instant::now();
        assert_eq!(command.cancel(), None);
        let reducer_latency = cancelling.elapsed();
        assert!(reducer_latency < Duration::from_millis(100));
        assert!(flag.load(Ordering::Acquire));
        assert_eq!(session.document(), &unchanged);
        assert_eq!(session.version(), version);
        assert_eq!(notifications.try_iter().count(), 0);
        let end = returning.recv_timeout(Duration::from_secs(10))?;
        wake.recv_timeout(Duration::from_secs(10))?;
        let reply = worker.drain()?.ok_or("missing cancelled reply")?;
        assert!(matches!(
            &reply,
            WorkerReply::Failed {
                error: GeometryFailure::Cancelled,
                ..
            }
        ));
        assert!(matches!(
            command.receive(reply, session.version()),
            PreparationEvent::Ignored
        ));
        assert!(command.take_ready(session.version()).is_none());
        // A later ordinary command is processed by the same worker. It is still
        // accepted once through real finalized authority, with no cancelled feature.
        let recovery_start = Instant::now();
        let parameters = session
            .document()
            .begin_parameter_draft(ParameterDraftOwner::Document)?;
        worker.submit(job(
            &session,
            3,
            DocumentEdit::ParameterDefinitions,
            parameters,
        ))?;
        let recovered = ready(&worker, &wake)?;
        let feature_count = session.document().features().len();
        session.accept_prepared(session.version(), recovered.prepared, recovered.token)?;
        assert_eq!(session.document().features().len(), feature_count);
        assert_eq!(session.revision(), version.revision + 1);
        assert_eq!(notifications.try_iter().count(), 1);
        println!(
            "native Boolean batch until cooperative boundary: {:?}; cancel reducer: {:?}; uninterruptible remainder: {:?}; subsequent real preparation: {:?}",
            end.duration_since(begin),
            reducer_latency,
            end.saturating_duration_since(cancelling),
            recovery_start.elapsed()
        );
        // M5.0 R3: a distinct window/session certificate uses the same real
        // worker after native cancellation, without publishing a second edit.
        let parameters = session
            .document()
            .begin_parameter_draft(ParameterDraftOwner::Document)?;
        let mut other = job(&session, 4, DocumentEdit::ParameterDefinitions, parameters);
        other.id.session = CommandInputSessionId(2);
        let other_start = Instant::now();
        worker.submit(other)?;
        let completed = ready(&worker, &wake)?;
        assert_eq!(completed.id.session, CommandInputSessionId(2));
        assert_eq!(completed.id.serial, 4);
        assert_eq!(session.revision(), version.revision + 1);
        worker.release(completed.token)?;
        println!(
            "R3_REAL_WORKER,other_session=2,queue_to_completed_us={:.3},committed_revision_unchanged=true,press_query_and_full_sketch_scheduler=Unproven",
            other_start.elapsed().as_secs_f64() * 1e6
        );
        Ok(())
    }
    #[test]
    fn partial_curved_through_all_real_worker_replays_both_directions_and_rolls_back_suffix()
    -> Result<(), Box<dyn std::error::Error>> {
        use newell_model::{
            Direction, ExtentSpec, ExtrudeGeometry, ExtrudeSpec, Length, ParameterValue,
            geometry::{BaseGeometryRecipe, ExtrudeValidationRefs, FaceRef, ProfileRef},
            parameters::ParameterDraftOwner,
        };
        for reverse in [false, true] {
            let (sender, wake) = mpsc::channel();
            let worker = GeometryWorker::start(1, move || {
                let _ = sender.send(());
            })?;
            let mut document = Document::default();
            let cubic = document.add_base_recipe_for_test(
                "Curved source",
                BaseGeometryRecipe::TestCubic {
                    control: [[0., 0.], [2., 3.], [4., 2.], [6., 0.]],
                    depth: Length::millimetres(1.)?,
                    transform: [
                        [1., 0., 0., 0.],
                        [0., 0.6, -0.8, 0.],
                        [0., 0.8, 0.6, 4.],
                        [0., 0., 0., 1.],
                    ],
                },
            )?;
            let source = document.add_base_recipe_for_test(
                "Partial footprint",
                BaseGeometryRecipe::TestBox {
                    size: [
                        Length::millimetres(1.)?,
                        Length::millimetres(0.1)?,
                        Length::millimetres(1.)?,
                    ],
                    transform: [
                        [1., 0., 0., 2.],
                        [0., 1., 0., -2.8],
                        [0., 0., 1., if reverse { 12. } else { 0. }],
                        [0., 0., 0., 1.],
                    ],
                },
            )?;
            let mut session = DocumentSession::default();
            session.replace(document)?;
            session.bind_geometry_worker(1)?;
            let parameters = session
                .document()
                .begin_parameter_draft(ParameterDraftOwner::Document)?;
            worker.submit(job(
                &session,
                1,
                DocumentEdit::ParameterDefinitions,
                parameters,
            ))?;
            let initial = ready(&worker, &wake)?;
            session.accept_prepared(session.version(), initial.prepared, initial.token)?;
            worker.retain_committed(initial.token)?;
            let cubic_entity = session.document().feature(cubic).ok_or("cubic")?.outputs()[0];
            let shape = session
                .document()
                .entity_geometry(cubic_entity)
                .ok_or("cubic geometry")?;
            let face = shape
                .faces
                .iter()
                .find(|f| f.frame.normal[1] < -0.79 && f.frame.normal[2] > 0.59)
                .ok_or("cubic top")?;
            let edge = face.wires[0]
                .edges
                .iter()
                .find(|e| !e.straight)
                .ok_or("curve")?
                .edge;
            let profile = ProfileRef::Edge {
                support: FaceRef {
                    entity: cubic_entity,
                    generation: shape.generation,
                    face: face.face,
                },
                wire: 0,
                edge,
            };
            let blind = |depth| -> Result<ExtentSpec, newell_model::ModelError> {
                Ok(ExtentSpec::Blind {
                    depth: ParameterValue::literal(Length::millimetres(depth)?),
                    direction: Direction::Forward,
                    along: None,
                    start: None,
                    end: None,
                })
            };
            let target_spec = ExtrudeSpec {
                profile: Some(profile),
                geometry: ExtrudeGeometry::Surface,
                extent: blind(4.)?,
                ..ExtrudeSpec::default()
            };
            let parameters = session
                .document()
                .begin_parameter_draft(ParameterDraftOwner::InsertExtrude { before: None })?;
            let target_feature = parameters.feature_owner().ok_or("target owner")?;
            worker.submit(job(
                &session,
                2,
                DocumentEdit::ExtrudeInsert {
                    spec: target_spec.clone(),
                    validation: ExtrudeValidationRefs::default(),
                },
                parameters,
            ))?;
            let result = ready(&worker, &wake)?;
            session.accept_prepared(session.version(), result.prepared, result.token)?;
            worker.retain_committed(result.token)?;
            let target = session
                .document()
                .feature(target_feature)
                .ok_or("target")?
                .outputs()[0];
            let source_entity = session
                .document()
                .feature(source)
                .ok_or("source")?
                .outputs()[0];
            let shape = session
                .document()
                .entity_geometry(source_entity)
                .ok_or("footprint")?;
            let face = shape
                .faces
                .iter()
                .find(|f| {
                    if reverse {
                        f.frame.normal[2] < -0.99
                    } else {
                        f.frame.normal[2] > 0.99
                    }
                })
                .ok_or("source face")?;
            let spec = ExtrudeSpec {
                profile: Some(ProfileRef::Face(FaceRef {
                    entity: source_entity,
                    generation: shape.generation,
                    face: face.face,
                })),
                extent: ExtentSpec::ThroughAll {
                    invert: false,
                    start: None,
                },
                ..ExtrudeSpec::default()
            };
            let parameters = session
                .document()
                .begin_parameter_draft(ParameterDraftOwner::InsertExtrude { before: None })?;
            let owner = parameters.feature_owner().ok_or("owner")?;
            let before = session.version();
            let notifications = session.subscribe();
            worker.submit(job(
                &session,
                3,
                DocumentEdit::ExtrudeInsert {
                    spec,
                    validation: ExtrudeValidationRefs::default(),
                },
                parameters,
            ))?;
            let result = ready(&worker, &wake)?;
            let output = result
                .prepared
                .document()
                .feature(owner)
                .ok_or("ThroughAll")?
                .outputs()[0];
            let shape = result
                .prepared
                .document()
                .entity_geometry(output)
                .ok_or("output geometry")?;
            let t = (24. - 252_f64.sqrt()) / 18.;
            let y = 9. * t - 12. * t * t + 3. * t * t * t;
            let reach = if reverse {
                12. - 6.025 - 1.25 * (16. / 9.) + 1e-6
            } else {
                5.1 + 1.25 * y + 1e-6
            };
            assert!(
                (shape.measure - 0.1 * reach).abs() < 1e-6,
                "reverse={reverse}: {}, expected {}",
                shape.measure,
                0.1 * reach
            );
            assert!(
                result
                    .scene
                    .items
                    .iter()
                    .any(|i| i.entity == output && !i.mesh.indices().is_empty())
            );
            session.accept_prepared(before, result.prepared, result.token)?;
            worker.retain_committed(result.token)?;
            assert_eq!(notifications.try_iter().count(), 1);
            let unchanged = session.document().clone();
            let version = session.version();
            let token = session.geometry_token();
            // Shortening the curved producer removes every finite hit in the
            // retained suffix. The real replay must reject the whole candidate.
            let parameters = session
                .document()
                .begin_parameter_draft(ParameterDraftOwner::EditExtrude(target_feature))?;
            let mut bad = target_spec.clone();
            bad.extent = blind(2.)?;
            worker.submit(job(
                &session,
                4,
                DocumentEdit::ExtrudeReplace {
                    feature: target_feature,
                    spec: bad,
                    validation: ExtrudeValidationRefs::default(),
                },
                parameters,
            ))?;
            wake.recv_timeout(Duration::from_secs(10))?;
            assert!(matches!(worker.drain()?, Some(WorkerReply::Failed { .. })));
            assert_eq!(session.document(), &unchanged);
            assert_eq!(session.version(), version);
            assert_eq!(session.geometry_token(), token);
            assert_eq!(notifications.try_iter().count(), 0);
            assert_eq!(
                session
                    .document()
                    .feature(target_feature)
                    .ok_or("retained target")?
                    .outputs(),
                &[target]
            );
            // Explicit recovery/replay preserves both producer and suffix output ids.
            let parameters = session
                .document()
                .begin_parameter_draft(ParameterDraftOwner::EditExtrude(target_feature))?;
            worker.submit(job(
                &session,
                5,
                DocumentEdit::ExtrudeReplace {
                    feature: target_feature,
                    spec: target_spec,
                    validation: ExtrudeValidationRefs::default(),
                },
                parameters,
            ))?;
            let result = ready(&worker, &wake)?;
            assert_eq!(
                result
                    .prepared
                    .document()
                    .feature(owner)
                    .ok_or("suffix")?
                    .outputs(),
                &[output]
            );
            assert_eq!(
                result
                    .prepared
                    .document()
                    .feature(target_feature)
                    .ok_or("producer")?
                    .outputs(),
                &[target]
            );
            session.accept_prepared(version, result.prepared, result.token)?;
            assert_eq!(notifications.try_iter().count(), 1);
        }
        Ok(())
    }
    fn accept_fixture(
        session: &mut DocumentSession,
        worker: &GeometryWorker,
        result: PreparedGeometry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let old = session.geometry_token();
        session.accept_prepared(session.version(), result.prepared, result.token)?;
        worker.retain_committed(result.token)?;
        if let Some(old) = old {
            worker.release(old)?;
        }
        Ok(())
    }
    #[test]
    fn curved_crossing_and_pinched_scopes_use_real_preparation_identity_and_atomic_rejection()
    -> Result<(), Box<dyn std::error::Error>> {
        use newell_model::{
            Direction, ExtentSpec, ExtrudeGeometry, ExtrudeResult, ExtrudeSpec, Length,
            MergePolicy, OffsetSpec, ParameterValue, SecondSpec, SidesSpec,
            geometry::{BaseGeometryRecipe, ExtrudeValidationRefs, FaceRef, ProfileRef},
            parameters::ParameterDraftOwner,
        };
        let value = |x| -> Result<ParameterValue, newell_model::ModelError> {
            Ok(ParameterValue::literal(Length::millimetres(x)?))
        };
        let t = (24. - 252_f64.sqrt()) / 18.;
        let ymax = 9. * t - 12. * t * t + 3. * t * t * t;
        for pinched in [false, true] {
            for all in [false, true] {
                let (sender, wake) = mpsc::channel();
                let worker = GeometryWorker::start(1, move || {
                    let _ = sender.send(());
                })?;
                let mut document = Document::default();
                let cubic = document.add_base_recipe_for_test(
                    "Cubic",
                    BaseGeometryRecipe::TestCubic {
                        control: [[0., 0.], [2., 3.], [4., 2.], [6., 0.]],
                        depth: Length::millimetres(1.)?,
                        transform: [
                            [1., 0., 0., 0.],
                            [0., 1., 0., 0.],
                            [0., 0., 1., 0.],
                            [0., 0., 0., 1.],
                        ],
                    },
                )?;
                let other = document.add_base_recipe_for_test(
                    "Disjoint cubic",
                    BaseGeometryRecipe::TestCubic {
                        control: [[0., 0.], [2., 3.], [4., 2.], [6., 0.]],
                        depth: Length::millimetres(1.)?,
                        transform: [
                            [1., 0., 0., 100.],
                            [0., 1., 0., 0.],
                            [0., 0., 1., 0.],
                            [0., 0., 0., 1.],
                        ],
                    },
                )?;
                let mut planes = Vec::new();
                for (name, sx, sy, z) in [
                    ("A", 0.1, 0., 3.9),
                    ("Cross", -0.1, 0., 4.7),
                    ("High", 0.2, 0., 4.8),
                    ("Low", 0., 0.1, 0.9),
                    ("Pinch", 0., -1., 2. + ymax),
                ] {
                    planes.push(document.add_base_recipe_for_test(
                        name,
                        BaseGeometryRecipe::TestBox {
                            size: [
                                Length::millimetres(10.)?,
                                Length::millimetres(10.)?,
                                Length::millimetres(1.)?,
                            ],
                            transform: [
                                [1., 0., 0., -1.],
                                [0., 1., 0., -1.],
                                [sx, sy, 1., z],
                                [0., 0., 0., 1.],
                            ],
                        },
                    )?);
                }
                let mut session = DocumentSession::default();
                session.replace(document)?;
                session.bind_geometry_worker(1)?;
                let parameters = session
                    .document()
                    .begin_parameter_draft(ParameterDraftOwner::Document)?;
                worker.submit(job(
                    &session,
                    1,
                    DocumentEdit::ParameterDefinitions,
                    parameters,
                ))?;
                accept_fixture(&mut session, &worker, ready(&worker, &wake)?)?;
                let entity = session.document().feature(cubic).ok_or("source")?.outputs()[0];
                let shape = session
                    .document()
                    .entity_geometry(entity)
                    .ok_or("geometry")?;
                let face = shape
                    .faces
                    .iter()
                    .find(|f| f.frame.normal == [0., 0., 1.])
                    .ok_or("support")?;
                let edge = face.wires[0]
                    .edges
                    .iter()
                    .find(|e| !e.straight)
                    .ok_or("curve")?
                    .edge;
                let profile = ProfileRef::Edge {
                    support: FaceRef {
                        entity,
                        generation: shape.generation,
                        face: face.face,
                    },
                    wire: 0,
                    edge,
                };
                let references: Vec<_> = planes
                    .iter()
                    .map(|id| {
                        let entity = session.document().feature(*id).unwrap().outputs()[0];
                        let shape = session.document().entity_geometry(entity).unwrap();
                        let face = shape
                            .faces
                            .iter()
                            .find(|f| f.frame.normal[2] < -0.7)
                            .unwrap();
                        FaceRef {
                            entity,
                            generation: shape.generation,
                            face: face.face,
                        }
                    })
                    .collect();
                let spec = ExtrudeSpec {
                    profile: Some(profile),
                    geometry: ExtrudeGeometry::Surface,
                    extent: ExtentSpec::UpTo {
                        target: references[0],
                        clearance: None,
                    },
                    ..ExtrudeSpec::default()
                };
                let parameters = session
                    .document()
                    .begin_parameter_draft(ParameterDraftOwner::InsertExtrude { before: None })?;
                let target_feature = parameters.feature_owner().ok_or("owner")?;
                worker.submit(job(
                    &session,
                    2,
                    DocumentEdit::ExtrudeInsert {
                        spec,
                        validation: ExtrudeValidationRefs::default(),
                    },
                    parameters,
                ))?;
                accept_fixture(&mut session, &worker, ready(&worker, &wake)?)?;
                let target = session
                    .document()
                    .feature(target_feature)
                    .ok_or("target")?
                    .outputs()[0];
                let other_entity = session
                    .document()
                    .feature(other)
                    .ok_or("other base")?
                    .outputs()[0];
                let other_shape = session
                    .document()
                    .entity_geometry(other_entity)
                    .ok_or("other geometry")?;
                let other_face = other_shape
                    .faces
                    .iter()
                    .find(|f| f.frame.normal == [0., 0., 1.])
                    .ok_or("other support")?;
                let other_edge = other_face.wires[0]
                    .edges
                    .iter()
                    .find(|e| !e.straight)
                    .ok_or("other cubic edge")?
                    .edge;
                let other_profile = ProfileRef::Edge {
                    support: FaceRef {
                        entity: other_entity,
                        generation: other_shape.generation,
                        face: other_face.face,
                    },
                    wire: 0,
                    edge: other_edge,
                };
                let miss_spec = ExtrudeSpec {
                    profile: Some(other_profile),
                    geometry: ExtrudeGeometry::Surface,
                    extent: ExtentSpec::Blind {
                        depth: value(1.)?,
                        direction: Direction::Forward,
                        along: None,
                        start: Some(OffsetSpec::Blind {
                            distance: value(10.)?,
                            direction: Direction::Forward,
                        }),
                        end: None,
                    },
                    ..ExtrudeSpec::default()
                };
                let parameters = session
                    .document()
                    .begin_parameter_draft(ParameterDraftOwner::InsertExtrude { before: None })?;
                let miss_feature = parameters.feature_owner().ok_or("miss")?;
                worker.submit(job(
                    &session,
                    3,
                    DocumentEdit::ExtrudeInsert {
                        spec: miss_spec,
                        validation: ExtrudeValidationRefs::default(),
                    },
                    parameters,
                ))?;
                accept_fixture(&mut session, &worker, ready(&worker, &wake)?)?;
                let miss = session
                    .document()
                    .feature(miss_feature)
                    .ok_or("miss")?
                    .outputs()[0];
                let spec = ExtrudeSpec {
                    profile: Some(profile),
                    geometry: ExtrudeGeometry::Surface,
                    result: if pinched {
                        ExtrudeResult::Remove
                    } else {
                        ExtrudeResult::Add
                    },
                    extent: ExtentSpec::Blind {
                        depth: value(1.)?,
                        direction: Direction::Forward,
                        along: None,
                        start: Some(OffsetSpec::Blind {
                            distance: value(if pinched { 2. } else { 1. })?,
                            direction: Direction::Forward,
                        }),
                        end: Some(OffsetSpec::UpTo(references[if pinched { 2 } else { 1 }])),
                    },
                    sides: if pinched {
                        SidesSpec::Second(SecondSpec::UpTo {
                            target: references[3],
                            end: None,
                        })
                    } else {
                        SidesSpec::One
                    },
                    targets: if all {
                        MergePolicy::All
                    } else {
                        MergePolicy::Explicit(vec![miss, target, target])
                    },
                };
                let validation = ExtrudeValidationRefs {
                    retained_merge_scope: vec![miss, target, target],
                };
                let parameters = session
                    .document()
                    .begin_parameter_draft(ParameterDraftOwner::InsertExtrude { before: None })?;
                let feature = parameters.feature_owner().ok_or("operation")?;
                let revision = session.revision();
                let events = session.subscribe();
                worker.submit(job(
                    &session,
                    4,
                    DocumentEdit::ExtrudeInsert {
                        spec: spec.clone(),
                        validation: validation.clone(),
                    },
                    parameters,
                ))?;
                let result = ready(&worker, &wake)?;
                let newell_model::Feature::Extrude(extrude) = result
                    .prepared
                    .document()
                    .feature(feature)
                    .ok_or("result")?
                else {
                    return Err("wrong feature".into());
                };
                assert_eq!(extrude.consumed_inputs(), &[target]);
                assert_eq!(extrude.outputs.len(), 1);
                let output = extrude.outputs[0];
                let geometry = result
                    .prepared
                    .document()
                    .entity_geometry(output)
                    .ok_or("actual sheet")?;
                let mut expected = 0.;
                let n = 16384;
                for i in 0..=n {
                    let t = f64::from(i) / f64::from(n);
                    let y = 9. * t - 12. * t * t + 3. * t * t * t;
                    let w = if i == 0 || i == n {
                        1.
                    } else if i % 2 == 0 {
                        2.
                    } else {
                        4.
                    };
                    let height = if pinched {
                        0.1 * y
                    } else {
                        (3. + 0.6 * t).max(3.6 - 0.6 * t)
                    };
                    expected += w * (36. + (9. - 24. * t + 9. * t * t).powi(2)).sqrt() * height;
                }
                expected /= 3. * f64::from(n);
                assert!(
                    (geometry.measure - expected).abs() < 1e-6,
                    "pinched={pinched}, all={all}: {} vs {expected}",
                    geometry.measure
                );
                assert_eq!(geometry.source_wire, Some(0));
                assert!(
                    result
                        .scene
                        .items
                        .iter()
                        .any(|i| i.entity == output && !i.mesh.indices().is_empty())
                );
                assert!(
                    result
                        .prepared
                        .document()
                        .active_entities_at(None)?
                        .contains(&miss)
                );
                accept_fixture(&mut session, &worker, result)?;
                assert_eq!(session.revision(), revision + 1);
                assert_eq!(events.try_iter().count(), 1);
                // A valid tool can leave two disks meeting at one interior point.
                // This required nonmanifold result is rejected, with no persistent ids,
                // references, geometry authority, notifications or history consumed.
                let bad = ExtrudeSpec {
                    profile: Some(profile),
                    geometry: ExtrudeGeometry::Surface,
                    result: ExtrudeResult::Remove,
                    extent: ExtentSpec::Blind {
                        depth: value(1.)?,
                        direction: Direction::Forward,
                        along: None,
                        start: Some(OffsetSpec::Blind {
                            distance: value(2.)?,
                            direction: Direction::Forward,
                        }),
                        end: Some(OffsetSpec::UpTo(references[2])),
                    },
                    sides: SidesSpec::Second(SecondSpec::UpTo {
                        target: references[4],
                        end: None,
                    }),
                    targets: MergePolicy::Explicit(vec![output]),
                };
                let unchanged = session.document().clone();
                let version = session.version();
                let token = session.geometry_token();
                for serial in [5, 6] {
                    let parameters = session.document().begin_parameter_draft(
                        ParameterDraftOwner::InsertExtrude { before: None },
                    )?;
                    worker.submit(job(
                        &session,
                        serial,
                        DocumentEdit::ExtrudeInsert {
                            spec: bad.clone(),
                            validation: ExtrudeValidationRefs {
                                retained_merge_scope: vec![output],
                            },
                        },
                        parameters,
                    ))?;
                    wake.recv_timeout(Duration::from_secs(10))?;
                    assert!(
                        matches!(worker.drain()?,Some(WorkerReply::Failed {error:GeometryFailure::Kernel(newell_kernel::NewellKernelError::Operation {message}),..}) if message=="nonmanifold pinched sheet vertex")
                    );
                    assert_eq!(session.document(), &unchanged);
                    assert_eq!(session.version(), version);
                    assert_eq!(session.geometry_token(), token);
                    assert_eq!(events.try_iter().count(), 0);
                }
                let parameters = session
                    .document()
                    .begin_parameter_draft(ParameterDraftOwner::EditExtrude(feature))?;
                worker.submit(job(
                    &session,
                    7,
                    DocumentEdit::ExtrudeReplace {
                        feature,
                        spec,
                        validation,
                    },
                    parameters,
                ))?;
                let result = ready(&worker, &wake)?;
                assert_eq!(
                    result
                        .prepared
                        .document()
                        .feature(feature)
                        .ok_or("edited result")?
                        .outputs(),
                    &[output]
                );
                accept_fixture(&mut session, &worker, result)?;
                assert_eq!(events.try_iter().count(), 1);
                // The reconstructed/pinched result is a real later merge operand;
                // complete removal proves zero outputs instead of an arbitrary error.
                let parameters = session
                    .document()
                    .begin_parameter_draft(ParameterDraftOwner::InsertExtrude { before: None })?;
                let remover = parameters.feature_owner().ok_or("remover")?;
                let full = ExtrudeSpec {
                    profile: Some(profile),
                    geometry: ExtrudeGeometry::Surface,
                    result: ExtrudeResult::Remove,
                    extent: ExtentSpec::Blind {
                        depth: value(20.)?,
                        direction: Direction::Forward,
                        along: None,
                        start: None,
                        end: None,
                    },
                    targets: MergePolicy::Explicit(vec![output]),
                    ..ExtrudeSpec::default()
                };
                worker.submit(job(
                    &session,
                    8,
                    DocumentEdit::ExtrudeInsert {
                        spec: full,
                        validation: ExtrudeValidationRefs {
                            retained_merge_scope: vec![output],
                        },
                    },
                    parameters,
                ))?;
                let result = ready(&worker, &wake)?;
                assert!(
                    result
                        .prepared
                        .document()
                        .feature(remover)
                        .ok_or("full removal")?
                        .outputs()
                        .is_empty()
                );
                accept_fixture(&mut session, &worker, result)?;
                assert!(session.document().active_entities_at(None)?.contains(&miss));
                assert_eq!(events.try_iter().count(), 1);
            }
        }
        Ok(())
    }
    fn modifier_crossings(surface: bool, family: usize) -> Result<(), Box<dyn std::error::Error>> {
        use newell_model::{
            Direction, ExtentSpec, ExtrudeGeometry, ExtrudeResult, ExtrudeSpec, Length,
            MergePolicy, OffsetSpec, ParameterValue, SecondOffsetSpec, SecondSpec, SidesSpec,
            geometry::{
                BaseGeometryRecipe, DirectionRef, ExtrudeValidationRefs, FaceRef, ProfileRef,
            },
            parameters::ParameterDraftOwner,
        };
        let value = |x| ParameterValue::literal(Length::millimetres(x).unwrap());
        let (sender, wake) = mpsc::channel();
        let worker = GeometryWorker::start(1, move || {
            let _ = sender.send(());
        })?;
        let mut document = Document::default();
        let source = document.add_base_solid("Source")?;
        let mut references = Vec::new();
        for (name, z) in [("Negative", -4.), ("Positive", 2.)] {
            references.push(document.add_base_recipe_for_test(
                name,
                BaseGeometryRecipe::TestBox {
                    size: [
                        Length::millimetres(1.)?,
                        Length::millimetres(1.)?,
                        Length::millimetres(2.)?,
                    ],
                    transform: [
                        [1., 0., 0., 0.],
                        [0., 1., 0., 0.],
                        [0., 0., 1., z],
                        [0., 0., 0., 1.],
                    ],
                },
            )?);
        }
        let mut baseline = DocumentSession::default();
        baseline.replace(document)?;
        baseline.bind_geometry_worker(1)?;
        let parameters = baseline
            .document()
            .begin_parameter_draft(ParameterDraftOwner::Document)?;
        worker.submit(job(
            &baseline,
            1,
            DocumentEdit::ParameterDefinitions,
            parameters,
        ))?;
        accept_fixture(&mut baseline, &worker, ready(&worker, &wake)?)?;
        let entity = baseline
            .document()
            .feature(source)
            .ok_or("source")?
            .outputs()[0];
        let shape = baseline
            .document()
            .entity_geometry(entity)
            .ok_or("source geometry")?;
        let face = shape
            .faces
            .iter()
            .find(|f| f.frame.normal == [0., 0., -1.])
            .ok_or("bottom")?;
        let profile = ProfileRef::Face(FaceRef {
            entity,
            generation: shape.generation,
            face: face.face,
        });
        let along = shape
            .faces
            .iter()
            .find_map(|f| {
                f.wires.iter().find_map(|w| {
                    w.edges
                        .iter()
                        .find(|e| (e.end[2] - e.start[2]).abs() > 0.99)
                        .map(|e| {
                            (
                                DirectionRef {
                                    support: FaceRef {
                                        entity,
                                        generation: shape.generation,
                                        face: f.face,
                                    },
                                    wire: w.wire,
                                    edge: e.edge,
                                },
                                e.end[2] > e.start[2],
                            )
                        })
                })
            })
            .ok_or("axis occurrence")?;
        let mut planes = Vec::new();
        for feature in &references {
            let entity = baseline
                .document()
                .feature(*feature)
                .ok_or("plane body")?
                .outputs()[0];
            let shape = baseline
                .document()
                .entity_geometry(entity)
                .ok_or("planes")?;
            for z in if feature == &references[0] {
                [-4., -2.]
            } else {
                [2., 4.]
            } {
                let face = shape
                    .faces
                    .iter()
                    .find(|f| {
                        (f.frame.origin[2] - z).abs() < 1e-7 && f.frame.normal[2].abs() > 0.99
                    })
                    .ok_or("plane")?;
                planes.push(FaceRef {
                    entity,
                    generation: shape.generation,
                    face: face.face,
                });
            }
        }
        let parameters = baseline
            .document()
            .begin_parameter_draft(ParameterDraftOwner::InsertExtrude { before: None })?;
        let target_feature = parameters.feature_owner().ok_or("target")?;
        let spec = ExtrudeSpec {
            profile: Some(profile),
            geometry: if surface {
                ExtrudeGeometry::Surface
            } else {
                ExtrudeGeometry::Solid
            },
            extent: ExtentSpec::Blind {
                depth: value(3.),
                direction: Direction::Forward,
                along: None,
                start: None,
                end: None,
            },
            sides: SidesSpec::Symmetric,
            ..ExtrudeSpec::default()
        };
        worker.submit(job(
            &baseline,
            2,
            DocumentEdit::ExtrudeInsert {
                spec,
                validation: ExtrudeValidationRefs::default(),
            },
            parameters,
        ))?;
        accept_fixture(&mut baseline, &worker, ready(&worker, &wake)?)?;
        let target = baseline
            .document()
            .feature(target_feature)
            .ok_or("target")?
            .outputs()[0];
        let mut serial = 3;
        let mut accepted = 0;
        let mut rejected = 0;
        let mut session = DocumentSession::default();
        session.bind_geometry_worker(1)?;
        let count = if family == 0 {
            100
        } else if family == 1 {
            3
        } else {
            8
        };
        for variant in 0..count {
            let reverse = if family == 0 {
                variant % 2 == 1
            } else {
                family == 2 && variant >= 4
            };
            let (near, far, opposite_near, opposite_far) = if reverse {
                (planes[2], planes[3], planes[1], planes[0])
            } else {
                (planes[1], planes[0], planes[2], planes[3])
            };
            let mut start = 0.;
            let start_kind = if family == 0 {
                (variant / 4) % 5
            } else if family == 2 {
                variant % 4
            } else {
                0
            };
            let start_spec = match start_kind {
                1 => {
                    start = if family == 2 { 0.5 } else { 2. };
                    Some(OffsetSpec::Blind {
                        distance: value(start),
                        direction: Direction::Forward,
                    })
                }
                2 => {
                    start = if family == 2 { -0.5 } else { -2. };
                    Some(OffsetSpec::Blind {
                        distance: value(-start),
                        direction: Direction::Reverse,
                    })
                }
                3 => {
                    start = 2.;
                    Some(OffsetSpec::UpTo(near))
                }
                4 => Some(OffsetSpec::Blind {
                    distance: value(0.),
                    direction: Direction::Forward,
                }),
                _ => None,
            };
            let mut end = if family == 0 {
                start + 5.
            } else if family == 1 {
                4. - if variant == 1 { 0.25 } else { 0. }
            } else {
                4.000001
            };
            let extent = if family == 0 {
                let end_spec = match variant / 20 {
                    1 => {
                        end += 1.;
                        Some(OffsetSpec::Blind {
                            distance: value(1.),
                            direction: Direction::Forward,
                        })
                    }
                    2 => {
                        end -= 1.;
                        Some(OffsetSpec::Blind {
                            distance: value(1.),
                            direction: Direction::Reverse,
                        })
                    }
                    3 => {
                        end = 4.;
                        Some(OffsetSpec::UpTo(far))
                    }
                    4 => Some(OffsetSpec::Blind {
                        distance: value(0.),
                        direction: Direction::Forward,
                    }),
                    _ => None,
                };
                let use_along = (variant / 2) % 2 == 1;
                let forward = if use_along {
                    along.1 == reverse
                } else {
                    !reverse
                };
                ExtentSpec::Blind {
                    depth: value(5.),
                    direction: if forward {
                        Direction::Forward
                    } else {
                        Direction::Reverse
                    },
                    along: use_along.then_some(along.0),
                    start: start_spec,
                    end: end_spec,
                }
            } else if family == 1 {
                ExtentSpec::UpTo {
                    target: far,
                    clearance: if variant == 0 {
                        None
                    } else {
                        Some(value(if variant == 1 { 0.25 } else { 0. }))
                    },
                }
            } else {
                ExtentSpec::ThroughAll {
                    invert: reverse,
                    start: start_spec,
                }
            };
            for side in 0..7 {
                let (low, high) = match side {
                    1 => (2. * start - end, end),
                    2 => (start - 3., end),
                    3 => (start, end.max(start + 3.)),
                    4 => (-4., end),
                    5 => (-3.75, end),
                    6 => (-2., end),
                    _ => (start, end),
                };
                let sides = match side {
                    1 => SidesSpec::Symmetric,
                    2 | 3 => SidesSpec::Second(SecondSpec::Blind {
                        depth: value(3.),
                        direction: if side == 2 {
                            Direction::Forward
                        } else {
                            Direction::Reverse
                        },
                    }),
                    4..=6 => SidesSpec::Second(SecondSpec::UpTo {
                        target: opposite_far,
                        end: match side {
                            5 => Some(SecondOffsetSpec::Clearance(value(0.25))),
                            6 => Some(SecondOffsetSpec::UpTo(opposite_near)),
                            _ => None,
                        },
                    }),
                    _ => SidesSpec::One,
                };
                for result in 0..3 {
                    let scopes: &[bool] = if result == 0 { &[true] } else { &[true, false] };
                    for all in scopes {
                        session.replace(baseline.document().clone())?;
                        worker.replace_document(session.identity())?;
                        let spec = ExtrudeSpec {
                            profile: Some(profile),
                            geometry: if surface {
                                ExtrudeGeometry::Surface
                            } else {
                                ExtrudeGeometry::Solid
                            },
                            result: match result {
                                1 => ExtrudeResult::Add,
                                2 => ExtrudeResult::Remove,
                                _ => ExtrudeResult::New,
                            },
                            extent: extent.clone(),
                            sides: sides.clone(),
                            targets: if *all {
                                MergePolicy::All
                            } else {
                                MergePolicy::Explicit(vec![target])
                            },
                        };
                        let parameters = session.document().begin_parameter_draft(
                            ParameterDraftOwner::InsertExtrude { before: None },
                        )?;
                        let feature = parameters.feature_owner().ok_or("feature")?;
                        let source = session.document().clone();
                        let version = session.version();
                        let events = session.subscribe();
                        worker.submit(job(
                            &session,
                            serial,
                            DocumentEdit::ExtrudeInsert {
                                spec,
                                validation: ExtrudeValidationRefs {
                                    retained_merge_scope: if *all { vec![] } else { vec![target] },
                                },
                            },
                            parameters,
                        ))?;
                        serial += 1;
                        let unchanged_add = result == 1
                            && (surface || !all)
                            && low >= -3. - 1e-7
                            && high <= 3. + 1e-7;
                        if (side == 6 && start == -2.) || unchanged_add {
                            wake.recv_timeout(Duration::from_secs(10))?;
                            let Some(WorkerReply::Failed {
                                error: GeometryFailure::Kernel(error),
                                ..
                            }) = worker.drain()?
                            else {
                                return Err("missing geometric rejection".into());
                            };
                            if side == 6 && start == -2. {
                                let expected = if surface {
                                    "target is not beyond the displaced start"
                                } else {
                                    "target does not lie beyond the entire start section"
                                };
                                assert!(
                                    matches!(error,newell_kernel::NewellKernelError::InvalidParameter {parameter} if parameter==expected),
                                    "unexpected collapsed-side error: {error}"
                                );
                            } else {
                                assert!(
                                    matches!(error,newell_kernel::NewellKernelError::Operation {message} if message=="NoTargetIntersection")
                                );
                            }
                            assert_eq!(session.document(), &source);
                            assert_eq!(session.version(), version);
                            assert_eq!(events.try_iter().count(), 0);
                            rejected += 1;
                            continue;
                        }
                        let prepared=ready(&worker,&wake).map_err(|e|format!("surface={surface},family={family},variant={variant},side={side},result={result},all={all}: {e}"))?;
                        let output = prepared
                            .prepared
                            .document()
                            .feature(feature)
                            .ok_or("feature output")?
                            .outputs();
                        let (a, b) = if reverse { (low, high) } else { (-high, -low) };
                        let targets: &[(f64, f64)] = if surface || !all {
                            &[(-3., 3.)]
                        } else {
                            &[(0., 1.), (-4., -2.), (2., 4.), (-3., 3.)]
                        };
                        let mut expected = Vec::new();
                        if result == 0 {
                            expected.push(b - a);
                        } else if result == 1 {
                            let bound = if surface || !all { 3_f64 } else { 4_f64 };
                            expected.push(b.max(bound) - a.min(-bound));
                        } else {
                            for (x, y) in targets {
                                if b.min(*y) - a.max(*x) > 1e-7 {
                                    if a > *x + 1e-7 {
                                        expected.push(a - *x);
                                    }
                                    if b < *y - 1e-7 {
                                        expected.push(*y - b);
                                    }
                                }
                            }
                        }
                        expected.sort_by(f64::total_cmp);
                        let mut actual: Vec<_> = output
                            .iter()
                            .map(|e| {
                                prepared
                                    .prepared
                                    .document()
                                    .entity_geometry(*e)
                                    .ok_or("result geometry")
                                    .map(|g| g.measure / if surface { 4. } else { 1. })
                            })
                            .collect::<Result<_, _>>()?;
                        actual.sort_by(f64::total_cmp);
                        assert_eq!(
                            actual.len(),
                            expected.len(),
                            "surface={surface},family={family},variant={variant},side={side},result={result},all={all}: actual={actual:?}, expected={expected:?}"
                        );
                        for (actual, expected) in actual.iter().zip(&expected) {
                            assert!(
                                (actual - expected).abs() < 1e-6,
                                "surface={surface},family={family},variant={variant},side={side},result={result},all={all}: {actual} vs {expected}"
                            );
                        }
                        assert_eq!(
                            prepared.scene.items.len(),
                            prepared.prepared.document().active_entities_at(None)?.len()
                        );
                        for item in &prepared.scene.items {
                            assert!(!item.mesh.indices().is_empty());
                        }
                        assert_eq!(session.document(), &source);
                        assert_eq!(session.version(), version);
                        accept_fixture(&mut session, &worker, prepared)?;
                        assert_eq!(events.try_iter().count(), 1);
                        assert_eq!(session.revision(), version.revision + 1);
                        accepted += 1;
                    }
                }
            }
        }
        println!(
            "MODIFIER_MATRIX surface={surface},family={family},accepted={accepted},deterministic_invalid={rejected}"
        );
        assert_eq!(accepted + rejected, count * 35);
        Ok(())
    }
    #[test]
    fn solid_blind_modifier_crossings() -> Result<(), Box<dyn std::error::Error>> {
        modifier_crossings(false, 0)
    }
    #[test]
    fn surface_blind_modifier_crossings() -> Result<(), Box<dyn std::error::Error>> {
        modifier_crossings(true, 0)
    }
    #[test]
    fn solid_up_to_modifier_crossings() -> Result<(), Box<dyn std::error::Error>> {
        modifier_crossings(false, 1)
    }
    #[test]
    fn surface_up_to_modifier_crossings() -> Result<(), Box<dyn std::error::Error>> {
        modifier_crossings(true, 1)
    }
    #[test]
    fn solid_through_all_modifier_crossings() -> Result<(), Box<dyn std::error::Error>> {
        modifier_crossings(false, 2)
    }
    #[test]
    fn surface_through_all_modifier_crossings() -> Result<(), Box<dyn std::error::Error>> {
        modifier_crossings(true, 2)
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
