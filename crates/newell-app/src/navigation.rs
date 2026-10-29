// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Per-window camera controller and gesture ownership, independent of the GPU.

use crate::camera::{Camera, MIN_DISTANCE, Vec3, dot, plane_hit, scale, sub, transform};
use crate::input::NavigationGesture;
use crate::picking::{PickHit, PickingScene};
use newell_kernel::NewellMesh;
use newell_render::{CameraSnapshot, SceneViewport, exposed_regions};
use std::time::{Duration, Instant};
use winit::event::{MouseButton, TouchPhase};

const WORLD_REFERENCE_CLIP_SPHERE: (Vec3, f64) = ([0.0; 3], 1.01);

fn clip_to_screen(clip: [f64; 4], viewport: SceneViewport) -> Option<(f64, f64)> {
    if clip[3] <= 1.0e-6
        || clip[2] < 0.0
        || clip[2] > clip[3]
        || !clip.iter().all(|value| value.is_finite())
    {
        return None;
    }
    Some((
        f64::from(viewport.x) + (clip[0] / clip[3] + 1.0) * f64::from(viewport.width) * 0.5,
        f64::from(viewport.y) + (1.0 - clip[1] / clip[3]) * f64::from(viewport.height) * 0.5,
    ))
}

fn point_distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn point_along(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

fn segment_distance(point: (f64, f64), start: (f64, f64), end: (f64, f64)) -> (f64, f64) {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let length_sq = dx.mul_add(dx, dy * dy);
    let t = if length_sq <= f64::EPSILON {
        0.0
    } else {
        (((point.0 - start.0) * dx + (point.1 - start.1) * dy) / length_sq).clamp(0.0, 1.0)
    };
    (t, point_distance(point, point_along(start, end, t)))
}

fn screen_to_world_parameter(screen_t: f64, start_w: f64, end_w: f64) -> f64 {
    let denominator = start_w.mul_add(screen_t, end_w * (1.0 - screen_t));
    if denominator.abs() <= f64::EPSILON {
        screen_t
    } else {
        screen_t * start_w / denominator
    }
}

fn enclosing_sphere(a: (Vec3, f64), b: (Vec3, f64)) -> (Vec3, f64) {
    let offset = sub(b.0, a.0);
    let distance = crate::camera::length(offset);
    if a.1 >= distance + b.1 {
        return a;
    }
    if b.1 >= distance + a.1 {
        return b;
    }
    if distance <= f64::EPSILON {
        return (a.0, a.1.max(b.1));
    }
    let radius = (distance + a.1 + b.1) * 0.5;
    (
        crate::camera::add(a.0, scale(offset, (radius - a.1) / distance)),
        radius,
    )
}

/// Mouse mapping belongs to the Controller rather than the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragKind {
    /// A deferred primary-button orbit after the pointer has crossed the click-drag threshold.
    PrimaryOrbit,
    Orbit,
    /// A deferred secondary-button pan after the pointer has crossed the click-drag threshold.
    SecondaryPan,
    /// A phased, button-free trackpad scroll pan over exposed scene pixels.
    TrackpadPan,
    Pan,
}

impl DragKind {
    /// Auxiliary mouse buttons keep distinct capture ownership while sharing the pan path.
    #[must_use]
    pub const fn from_auxiliary_button(button: MouseButton) -> Option<Self> {
        match button {
            MouseButton::Right => Some(Self::SecondaryPan),
            MouseButton::Middle => Some(Self::Pan),
            _ => None,
        }
    }
}

/// Logical-pixel pointer movement needed to capture a primary or secondary drag.
///
/// Selection waits for a certified release. Keeping this slop in logical pixels makes
/// clicks equally stable on standard and Retina displays.
pub const CLICK_DRAG_THRESHOLD_LOGICAL_PX: f64 = 4.0;
/// Distinct phase streams cannot steal each other's zoom anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomSource {
    Scroll,
    Pinch,
}
#[derive(Debug, Clone, Copy)]
struct Drag {
    kind: DragKind,
    start: (f64, f64),
    camera: Camera,
    pivot: Vec3,
    depth: f64,
    display_scale: f64,
    mesh_revision: u64,
}
#[derive(Debug, Clone, Copy)]
struct Zoom {
    source: ZoomSource,
    anchor: Vec3,
    point: (f64, f64),
}
#[derive(Debug, Clone, Copy)]
struct ViewTransition {
    started: Instant,
    from: Camera,
    to: Camera,
    pivot: Vec3,
    screen_point: (f64, f64),
    pivot_depth: f64,
    viewport: SceneViewport,
    mesh_revision: u64,
}
const VIEW_TRANSITION_DURATION: Duration = Duration::from_millis(250);
const VIEW_TRANSITION_FRAME: Duration = Duration::from_millis(16);

/// The only mutable camera/navigation owner. View snapshots contain matrices only.
#[derive(Debug, Default)]
pub struct Navigation {
    camera: Camera,
    scene: PickingScene,
    occlusions: Vec<SceneViewport>,
    drag: Option<Drag>,
    pending_drag: Option<Drag>,
    /// Physical-pixel displacement supplied by macOS scroll events during a secondary press.
    secondary_scroll_offset: (f64, f64),
    /// Keeps the same phased scroll stream out of UI scrolling/zoom after button release.
    secondary_scroll_stream: bool,
    /// Ownership and accumulated physical-pixel displacement for a phased PixelDelta pan.
    trackpad_pan_stream: bool,
    trackpad_pan_anchor: Option<(f64, f64)>,
    trackpad_pan_offset: (f64, f64),
    trackpad_pan_gate: NavigationGesture,
    zoom: Option<Zoom>,
    scroll_gate: NavigationGesture,
    pinch_gate: NavigationGesture,
    transition: Option<ViewTransition>,
    revision: u64,
}
impl Navigation {
    /// Creates an isolated window camera, useful for CPU-only integrations.
    #[must_use]
    pub fn new(camera: Camera) -> Self {
        Self {
            camera,
            ..Self::default()
        }
    }
    /// Read-only pose; navigation cannot be mutated through a View API.
    #[must_use]
    pub const fn camera(&self) -> Camera {
        self.camera
    }
    /// Current geometry queries for controller selection and verification.
    #[must_use]
    pub const fn scene(&self) -> &PickingScene {
        &self.scene
    }
    /// Synchronizes the CPU query cache with a displayed mesh, preserving pose.
    pub fn set_mesh(&mut self, mesh: &NewellMesh) {
        self.cancel();
        self.scene.set_mesh(mesh);
        self.revision = self.revision.wrapping_add(1);
    }
    /// Layout affects projection/occlusion only; captures cannot use stale coordinates.
    pub fn set_layout(&mut self, viewport: SceneViewport, occlusions: Vec<SceneViewport>) {
        if self.camera.viewport != viewport || self.occlusions != occlusions {
            self.cancel();
            self.camera.resize(viewport);
            self.occlusions = occlusions;
            self.revision = self.revision.wrapping_add(1);
        }
    }
    /// Fixed-small-footprint scene eligibility supplements the shell's O(1) grid lookup.
    #[must_use]
    pub fn visible(&self, point: (f64, f64)) -> bool {
        self.camera.viewport.contains(point.0, point.1)
            && !self.occlusions.iter().any(|r| r.contains(point.0, point.1))
    }
    /// Nearest positive hit only when exposed scene pixels own the pointer.
    #[must_use]
    pub fn pick(&self, point: (f64, f64)) -> Option<PickHit> {
        if !self.visible(point) {
            return None;
        }
        self.scene.pick(self.camera.ray(point)?)
    }
    /// Picks a renderer-owned reference from its actual projected world geometry.
    /// Translucently occluded references remain eligible; camera clipping and exposed
    /// viewport admission still apply.
    #[must_use]
    pub fn pick_origin_reference(&self, point: (f64, f64)) -> Option<newell_ui::OriginReference> {
        use newell_ui::OriginReference as R;
        const AXIS_LENGTH_PX: f64 = 192.0;
        const PLANE_OUTER_PX: f64 = 156.0;
        const PLANE_INSET: f64 = 0.2;
        const AXIS_TOLERANCE_PX: f64 = 5.0;
        if !self.visible(point) {
            return None;
        }
        let camera = self.camera;
        let snapshot = self.snapshot()?;
        let view_projection = snapshot
            .view_projection()
            .map(|column| column.map(f64::from));
        let origin_clip = transform(view_projection, [0.0, 0.0, 0.0, 1.0]);
        if origin_clip[3] <= 1.0e-6 || origin_clip[2] < 0.0 || origin_clip[2] > origin_clip[3] {
            return None;
        }
        let projection = snapshot.projection();
        let pixels_per_world = f64::from(projection[1][1].abs())
            * f64::from(camera.viewport.height)
            / (2.0 * origin_clip[3]);
        let world_scale = AXIS_LENGTH_PX / pixels_per_world;
        if !world_scale.is_finite() || world_scale <= 0.0 {
            return None;
        }
        let origin_screen = clip_to_screen(origin_clip, camera.viewport)?;
        let origin_distance = point_distance(point, origin_screen);
        if origin_distance <= 5.0 {
            return Some(R::O);
        }

        let mut axis_hit: Option<(f64, usize, R)> = None;
        for (axis, reference) in [R::X, R::Y, R::Z].into_iter().enumerate() {
            let mut end = [0.0; 3];
            end[axis] = world_scale;
            let end_clip = transform(view_projection, [end[0], end[1], end[2], 1.0]);
            let Some(end_screen) = clip_to_screen(end_clip, camera.viewport) else {
                continue;
            };
            let (screen_t, distance) = segment_distance(point, origin_screen, end_screen);
            if distance > AXIS_TOLERANCE_PX {
                continue;
            }
            let dash_distance = point_distance(
                origin_screen,
                point_along(origin_screen, end_screen, screen_t),
            );
            if dash_distance.rem_euclid(10.0) >= 6.0 {
                continue;
            }
            let world_t = screen_to_world_parameter(screen_t, origin_clip[3], end_clip[3]);
            let mut world_point = [0.0; 3];
            world_point[axis] = world_scale * world_t;
            let depth = dot(sub(world_point, camera.eye), camera.basis().2);
            let candidate = (depth, axis, reference);
            if axis_hit.is_none_or(|best| {
                depth.total_cmp(&best.0).is_lt()
                    || (depth.total_cmp(&best.0).is_eq() && axis < best.1)
            }) {
                axis_hit = Some(candidate);
            }
        }
        if let Some((_, _, reference)) = axis_hit {
            return Some(reference);
        }

        let ray = camera.ray(point)?;
        let mut plane_hit_choice: Option<(f64, usize, R)> = None;
        for (index, (reference, normal_axis, a, b)) in
            [(R::XY, 2, 0, 1), (R::XZ, 1, 0, 2), (R::YZ, 0, 1, 2)]
                .into_iter()
                .enumerate()
        {
            let mut normal = [0.0; 3];
            normal[normal_axis] = 1.0;
            let Some(hit) = plane_hit(ray, [0.0; 3], normal) else {
                continue;
            };
            let lo = PLANE_OUTER_PX * PLANE_INSET / AXIS_LENGTH_PX * world_scale;
            let hi = PLANE_OUTER_PX / AXIS_LENGTH_PX * world_scale;
            if !(lo..=hi).contains(&hit[a]) || !(lo..=hi).contains(&hit[b]) {
                continue;
            }
            let clip = transform(view_projection, [hit[0], hit[1], hit[2], 1.0]);
            if clip[3] <= 1.0e-6 || clip[2] < 0.0 || clip[2] > clip[3] {
                continue;
            }
            let depth = dot(sub(hit, camera.eye), camera.basis().2);
            if plane_hit_choice.is_none_or(|best| {
                depth.total_cmp(&best.0).is_lt()
                    || (depth.total_cmp(&best.0).is_eq() && index < best.1)
            }) {
                plane_hit_choice = Some((depth, index, reference));
            }
        }
        plane_hit_choice.map(|(_, _, reference)| reference)
    }
    fn pivot(&self, point: (f64, f64)) -> Vec3 {
        self.pick(point)
            .map(|hit| hit.point)
            .or_else(|| {
                self.scene
                    .visible_bounds(self.camera, &self.occlusions)
                    .map(|b| b.centre())
            })
            .unwrap_or_else(|| self.camera.target())
    }
    fn anchor(&self, point: (f64, f64)) -> Option<Vec3> {
        self.pick(point).map(|hit| hit.point).or_else(|| {
            self.camera
                .ray(point)
                .and_then(|ray| plane_hit(ray, self.camera.target(), self.camera.basis().2))
        })
    }
    /// Admits a drag without changing any camera matrix or look target.
    pub fn begin(
        &mut self,
        kind: DragKind,
        point: (f64, f64),
        display_scale: f64,
        eligible: bool,
    ) -> bool {
        if !eligible
            || !self.visible(point)
            || self.drag.is_some()
            || self.pending_drag.is_some()
            || self.zoom.is_some()
            || !display_scale.is_finite()
            || display_scale <= 0.0
        {
            return false;
        }
        if !matches!(kind, DragKind::PrimaryOrbit | DragKind::SecondaryPan) {
            self.transition = None;
        }
        let pivot = self.pivot(point);
        let depth = dot(sub(pivot, self.camera.eye), self.camera.basis().2).max(MIN_DISTANCE);
        self.drag = Some(Drag {
            kind,
            start: point,
            camera: self.camera,
            pivot,
            depth,
            display_scale,
            mesh_revision: self.scene.revision(),
        });
        true
    }
    /// Starts a primary-button press over exposed scene pixels without changing the camera.
    /// The press becomes an orbit capture only after [`CLICK_DRAG_THRESHOLD_LOGICAL_PX`].
    pub fn begin_primary_orbit(
        &mut self,
        point: (f64, f64),
        display_scale: f64,
        eligible: bool,
    ) -> bool {
        self.begin_pending(DragKind::PrimaryOrbit, point, display_scale, eligible)
    }
    /// Starts a secondary-button press over exposed scene pixels without changing the camera.
    /// After the threshold, the existing contextual-depth pan path owns the drag.
    pub fn begin_secondary_pan(
        &mut self,
        point: (f64, f64),
        display_scale: f64,
        eligible: bool,
    ) -> bool {
        let started = self.begin_pending(DragKind::SecondaryPan, point, display_scale, eligible);
        if started {
            self.secondary_scroll_offset = (0.0, 0.0);
            self.secondary_scroll_stream = false;
        }
        started
    }
    fn begin_pending(
        &mut self,
        kind: DragKind,
        point: (f64, f64),
        display_scale: f64,
        eligible: bool,
    ) -> bool {
        if !self.begin(kind, point, display_scale, eligible) {
            return false;
        }
        self.pending_drag = self.drag.take();
        true
    }
    /// Advances a pending press. Returns `true` only when it changed the camera.
    /// After the threshold is crossed it uses the captured orbit or pan path.
    pub fn advance_pending_drag(&mut self, point: (f64, f64)) -> bool {
        if !point.0.is_finite() || !point.1.is_finite() {
            return false;
        }
        let Some(pending) = self.pending_drag else {
            return false;
        };
        let dx = (point.0 - pending.start.0) / pending.display_scale;
        let dy = (point.1 - pending.start.1) / pending.display_scale;
        if dx.mul_add(dx, dy * dy) <= CLICK_DRAG_THRESHOLD_LOGICAL_PX.powi(2) {
            return false;
        }
        self.pending_drag = None;
        self.drag = Some(pending);
        self.transition = None;
        self.navigate(point)
    }
    /// Advances the pending primary press without affecting a secondary press.
    pub fn advance_primary_orbit(&mut self, point: (f64, f64)) -> bool {
        self.primary_orbit_pending() && self.advance_pending_drag(point)
    }
    /// Advances the pending secondary press without affecting a primary press.
    pub fn advance_secondary_pan(&mut self, point: (f64, f64)) -> bool {
        self.secondary_pan_pending() && self.advance_pending_drag(point)
    }
    /// `true` while a primary-button press is waiting to cross the orbit threshold.
    #[must_use]
    pub const fn primary_orbit_pending(&self) -> bool {
        matches!(
            self.pending_drag,
            Some(Drag {
                kind: DragKind::PrimaryOrbit,
                ..
            })
        )
    }
    /// `true` while a secondary-button press is waiting to cross the pan threshold.
    #[must_use]
    pub const fn secondary_pan_pending(&self) -> bool {
        matches!(
            self.pending_drag,
            Some(Drag {
                kind: DragKind::SecondaryPan,
                ..
            })
        )
    }
    /// `true` while either press is waiting to cross the click-drag threshold.
    #[must_use]
    pub const fn has_pending_drag(&self) -> bool {
        self.pending_drag.is_some()
    }
    fn secondary_pan_active(&self) -> bool {
        self.pending_drag
            .or(self.drag)
            .is_some_and(|drag| drag.kind == DragKind::SecondaryPan)
    }
    /// Routes pointer movement through the captured drag, including any two-finger scroll
    /// displacement accumulated while the secondary button is held.
    pub fn advance_pointer(&mut self, point: (f64, f64)) -> bool {
        // Scroll deltas alone advance this capture; macOS also emits CursorMoved
        // before scroll events, without including the accumulated pan displacement.
        if self
            .drag
            .is_some_and(|drag| drag.kind == DragKind::TrackpadPan)
        {
            return false;
        }
        let point = if self.secondary_pan_active() {
            (
                point.0 + self.secondary_scroll_offset.0,
                point.1 + self.secondary_scroll_offset.1,
            )
        } else {
            point
        };
        if self.has_pending_drag() {
            self.advance_pending_drag(point)
        } else {
            self.navigate(point)
        }
    }
    /// A macOS two-finger movement may arrive as a pixel scroll while the secondary click
    /// remains pressed. Treat that stream as motion of the existing pan capture. `None`
    /// leaves ordinary scrolling to the UI or zoom controller.
    pub fn advance_secondary_pan_scroll(
        &mut self,
        delta: (f64, f64),
        phase: TouchPhase,
        pointer: (f64, f64),
    ) -> Option<bool> {
        if !self.secondary_pan_active() {
            if !self.secondary_scroll_stream || phase == TouchPhase::Started {
                self.secondary_scroll_stream = false;
                return None;
            }
            if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.secondary_scroll_stream = false;
                self.scroll_gate.update(phase, false);
            }
            return Some(false);
        }
        // Reject the same scroll stream's momentum after button release; a fresh Started
        // phase can still begin ordinary cursor-targeted zoom.
        self.scroll_gate.update(phase, false);
        self.secondary_scroll_stream = match phase {
            TouchPhase::Started => true,
            TouchPhase::Moved => self.secondary_scroll_stream,
            TouchPhase::Ended | TouchPhase::Cancelled => false,
        };
        if phase == TouchPhase::Cancelled || !delta.0.is_finite() || !delta.1.is_finite() {
            return Some(false);
        }
        let next = (
            self.secondary_scroll_offset.0 + delta.0,
            self.secondary_scroll_offset.1 + delta.1,
        );
        if !next.0.is_finite() || !next.1.is_finite() {
            return Some(false);
        }
        self.secondary_scroll_offset = next;
        Some(self.advance_pointer(pointer))
    }
    /// Marks a phased PixelDelta stream as owned by the UI. Later phases cannot start a
    /// viewport pan if the pointer leaves that UI target before the stream ends.
    pub fn reject_trackpad_pan_scroll(&mut self, phase: TouchPhase) {
        if phase == TouchPhase::Started {
            self.trackpad_pan_stream = true;
            self.trackpad_pan_gate.update(phase, false);
            self.release(DragKind::TrackpadPan);
        } else if self.trackpad_pan_stream {
            self.trackpad_pan_gate.update(phase, false);
            self.release(DragKind::TrackpadPan);
            if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.trackpad_pan_stream = false;
            }
        }
    }
    /// Routes a phased PixelDelta gesture through the existing contextual-depth pan math.
    /// `PixelDelta` is already in physical pixels; it is deliberately not scaled again.
    /// `None` means this was not a stream started by this router, leaving it available to zoom.
    pub fn advance_trackpad_pan_scroll(
        &mut self,
        delta: (f64, f64),
        phase: TouchPhase,
        pointer: (f64, f64),
        display_scale: f64,
        eligible: bool,
    ) -> Option<bool> {
        // macOS MayBegin and Began both map to Started. Once captured, keep the
        // original anchor/depth and accumulate either event's delta normally.
        let phase = if phase == TouchPhase::Started
            && self
                .drag
                .is_some_and(|drag| drag.kind == DragKind::TrackpadPan)
        {
            TouchPhase::Moved
        } else {
            phase
        };
        if phase == TouchPhase::Started {
            self.trackpad_pan_stream = true;
            self.trackpad_pan_anchor = Some(pointer);
            self.trackpad_pan_offset = (0.0, 0.0);
            let allowed = self
                .trackpad_pan_gate
                .update(phase, eligible && self.visible(pointer));
            if !allowed {
                self.release(DragKind::TrackpadPan);
                return Some(false);
            }
            if !self.begin(DragKind::TrackpadPan, pointer, display_scale, true) {
                self.trackpad_pan_gate.cancel();
                return Some(false);
            }
        } else if !self.trackpad_pan_stream {
            return None;
        } else {
            let allowed = self
                .trackpad_pan_gate
                .update(phase, eligible && self.visible(pointer));
            if !allowed {
                self.release(DragKind::TrackpadPan);
                if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                    self.trackpad_pan_stream = false;
                    self.trackpad_pan_anchor = None;
                }
                return Some(false);
            }
        }
        if phase == TouchPhase::Cancelled || !delta.0.is_finite() || !delta.1.is_finite() {
            self.release(DragKind::TrackpadPan);
            self.trackpad_pan_gate.cancel();
            if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.trackpad_pan_stream = false;
                self.trackpad_pan_anchor = None;
            }
            return Some(false);
        }
        let next = (
            self.trackpad_pan_offset.0 + delta.0,
            self.trackpad_pan_offset.1 + delta.1,
        );
        if !next.0.is_finite() || !next.1.is_finite() {
            self.release(DragKind::TrackpadPan);
            self.trackpad_pan_gate.cancel();
            return Some(false);
        }
        self.trackpad_pan_offset = next;
        let anchor = self.trackpad_pan_anchor.unwrap_or(pointer);
        let changed = self.navigate((
            anchor.0 + self.trackpad_pan_offset.0,
            anchor.1 + self.trackpad_pan_offset.1,
        ));
        if phase == TouchPhase::Ended {
            self.release(DragKind::TrackpadPan);
            self.trackpad_pan_stream = false;
            self.trackpad_pan_anchor = None;
        }
        Some(changed)
    }
    /// Whether mouse capture owns motion, including zero-displacement events.
    #[must_use]
    pub const fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }
    /// Advances a captured drag using a frozen start pose; no pointer-edge remapping.
    pub fn navigate(&mut self, point: (f64, f64)) -> bool {
        if !point.0.is_finite() || !point.1.is_finite() {
            return false;
        }
        let Some(mut drag) = self.drag else {
            return false;
        };
        if drag.mesh_revision != self.scene.revision() {
            self.cancel();
            return false;
        }
        let before = self.camera;
        self.camera = match drag.kind {
            DragKind::PrimaryOrbit | DragKind::Orbit => {
                let displacement = (point.0 - drag.start.0, point.1 - drag.start.1);
                let next = drag
                    .camera
                    .orbit(drag.pivot, displacement, drag.display_scale);
                // Remove pitch overscroll at a pole so reversing the pointer responds
                // immediately. Unsaturated motion remains start-snapshot deterministic.
                drag.start.1 -= drag
                    .camera
                    .pitch_excess_pixels(displacement, drag.display_scale);
                next
            }
            DragKind::SecondaryPan | DragKind::Pan | DragKind::TrackpadPan => {
                drag.camera.pan(drag.start, point, drag.depth)
            }
        };
        self.drag = Some(drag);
        self.record(before)
    }
    /// Only the initiating mouse button releases a captured drag.
    pub fn finish_primary(&mut self, point: (f64, f64)) -> Option<(f64, f64)> {
        let click = self
            .pending_drag
            .filter(|pending| {
                let dx = (point.0 - pending.start.0) / pending.display_scale;
                let dy = (point.1 - pending.start.1) / pending.display_scale;
                pending.kind == DragKind::PrimaryOrbit
                    && point.0.is_finite()
                    && point.1.is_finite()
                    && pending.mesh_revision == self.scene.revision()
                    && pending.camera == self.camera
                    && dx.mul_add(dx, dy * dy) <= CLICK_DRAG_THRESHOLD_LOGICAL_PX.powi(2)
                    && self.visible(point)
            })
            .map(|pending| pending.start);
        self.release(DragKind::PrimaryOrbit);
        click
    }
    pub fn release(&mut self, kind: DragKind) {
        if kind == DragKind::SecondaryPan {
            self.secondary_scroll_offset = (0.0, 0.0);
        }
        if kind == DragKind::TrackpadPan {
            self.trackpad_pan_offset = (0.0, 0.0);
            self.trackpad_pan_anchor = None;
        }
        if self.pending_drag.is_some_and(|drag| drag.kind == kind) {
            self.pending_drag = None;
        }
        if self.drag.is_some_and(|drag| drag.kind == kind) {
            self.drag = None;
        }
    }
    /// Any accepted or pending scene gesture suppresses navigation marker hover.
    pub fn has_capture(&self) -> bool {
        self.drag.is_some() || self.pending_drag.is_some() || self.zoom.is_some()
    }
    /// Cancels captures without altering pose. Ongoing trackpad streams stay rejected.
    pub fn cancel(&mut self) {
        self.cancel_captures();
        self.transition = None;
    }
    /// Clears direct/pending gestures while allowing an automatic view transition to continue.
    pub fn cancel_captures(&mut self) {
        self.drag = None;
        self.pending_drag = None;
        self.secondary_scroll_offset = (0.0, 0.0);
        self.trackpad_pan_offset = (0.0, 0.0);
        self.trackpad_pan_anchor = None;
        self.zoom = None;
        self.scroll_gate.cancel();
        self.pinch_gate.cancel();
        self.trackpad_pan_gate.cancel();
    }
    fn record(&mut self, before: Camera) -> bool {
        let changed = self.camera != before;
        if changed {
            self.revision = self.revision.wrapping_add(1);
        }
        changed
    }
    /// Applies a discrete wheel event or a phase of a smooth scroll/pinch stream.
    /// Started with zero delta still freezes ownership and anchor at the actual start.
    pub fn zoom(
        &mut self,
        point: (f64, f64),
        steps: f64,
        stream: Option<(ZoomSource, TouchPhase)>,
        eligible: bool,
    ) -> bool {
        let visible =
            eligible && self.visible(point) && self.drag.is_none() && self.pending_drag.is_none();
        let mut end = false;
        let capture = if let Some((source, phase)) = stream {
            let gate = match source {
                ZoomSource::Scroll => &mut self.scroll_gate,
                ZoomSource::Pinch => &mut self.pinch_gate,
            };
            let allowed = gate.update(phase, visible);
            // Moved-only input has no lifecycle to release a frozen anchor later.
            // Apply it once, just like a discrete wheel event, without retaining capture.
            end = !gate.is_active();
            if (phase == TouchPhase::Started || !allowed)
                && self.zoom.is_some_and(|zoom| zoom.source == source)
            {
                self.zoom = None;
            }
            if !allowed {
                return false;
            }
            if self.zoom.is_some_and(|zoom| zoom.source != source) {
                return false;
            }
            if self.zoom.is_none() {
                let Some(anchor) = self.anchor(point) else {
                    return false;
                };
                self.zoom = Some(Zoom {
                    source,
                    anchor,
                    point,
                });
            }
            self.zoom
        } else {
            if !visible || self.zoom.is_some() {
                return false;
            }
            self.anchor(point).map(|anchor| Zoom {
                source: ZoomSource::Scroll,
                anchor,
                point,
            })
        };
        let Some(capture) = capture else {
            return false;
        };
        self.transition = None;
        let before = self.camera;
        self.camera = self.camera.zoom(
            capture.anchor,
            capture.point,
            steps,
            self.maximum_distance(),
        );
        if end {
            self.zoom = None;
        }
        self.record(before)
    }
    /// Model-scale-aware distance limit; independent of pointer anchoring and pan.
    #[must_use]
    pub fn maximum_distance(&self) -> f64 {
        4.0 * self
            .camera
            .fit_distance(self.scene.bounds().map_or(1.0, |b| b.sphere().1))
    }
    /// Projection switches preserve scale at the focal plane.
    pub fn set_perspective(&mut self, perspective: bool) -> bool {
        if self.camera.perspective == perspective {
            return false;
        }
        self.cancel();
        let before = self.camera;
        self.camera.set_perspective(perspective);
        self.record(before)
    }
    /// Current camera/mesh/layout generation for immutable widget capture certificates.
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    /// Routes widget orientations through the existing sole camera authority.
    pub fn align_view(&mut self, direction: newell_ui::navigation_widgets::ViewDirection) -> bool {
        self.cancel();
        let before = self.camera;
        let viewport = self.camera.viewport;
        let anchor_pixel = (
            f64::from(viewport.x) + f64::from(viewport.width) * 0.5,
            f64::from(viewport.y) + f64::from(viewport.height) * 0.5,
        );
        let contextual_pivot = self.pivot(anchor_pixel);
        let (pivot, screen_point) = self
            .camera
            .project(contextual_pivot)
            .map(|screen| (contextual_pivot, screen))
            .or_else(|| {
                let target = self.camera.target();
                self.camera.project(target).map(|screen| (target, screen))
            })
            .unzip();
        if let (Some(pivot), Some(screen_point)) = (pivot, screen_point) {
            self.camera.align_view_about(direction, pivot, screen_point);
        }
        self.record(before)
    }
    /// Starts a 250 ms automatic-view transition around the current contextual pivot.
    pub fn start_view_transition(
        &mut self,
        direction: newell_ui::navigation_widgets::ViewDirection,
        now: Instant,
    ) -> bool {
        let from = self.prepare_transition_start(now);
        let viewport = from.viewport;
        let center_pixel = (
            f64::from(viewport.x) + f64::from(viewport.width) * 0.5,
            f64::from(viewport.y) + f64::from(viewport.height) * 0.5,
        );
        let pivot = self.pivot(center_pixel);
        let Some(screen_point) = from.project(pivot) else {
            return false;
        };
        let pivot_depth = dot(sub(pivot, from.eye), from.basis().2);
        if !pivot_depth.is_finite() || pivot_depth <= MIN_DISTANCE {
            return false;
        }
        let mut to = from;
        if !to.align_view_about(direction, pivot, screen_point) || to == from {
            self.transition = None;
            return false;
        }
        self.install_transition(now, from, to, pivot, screen_point, pivot_depth)
    }
    /// Starts the same transition for the existing closest-principal-axis command.
    pub fn start_normal_to_axis_transition(&mut self, now: Instant) -> bool {
        let from = self.prepare_transition_start(now);
        let viewport = from.viewport;
        let point = (
            f64::from(viewport.x) + f64::from(viewport.width) * 0.5,
            f64::from(viewport.y) + f64::from(viewport.height) * 0.5,
        );
        let pivot = self.pivot(point);
        let Some(screen_point) = from.project(pivot) else {
            return false;
        };
        let pivot_depth = dot(sub(pivot, from.eye), from.basis().2);
        let mut to = from;
        to.normal_to_axis();
        let Some(anchored) = to.interpolate_view_about(to, 1.0, pivot, screen_point, pivot_depth)
        else {
            self.transition = None;
            return false;
        };
        to = anchored;
        if to == from {
            self.transition = None;
            return false;
        }
        self.install_transition(now, from, to, pivot, screen_point, pivot_depth)
    }
    fn prepare_transition_start(&mut self, now: Instant) -> Camera {
        if self.transition.is_some() {
            self.advance_view_transition(now);
        }
        self.cancel_captures();
        self.camera
    }
    fn install_transition(
        &mut self,
        now: Instant,
        from: Camera,
        to: Camera,
        pivot: Vec3,
        screen_point: (f64, f64),
        pivot_depth: f64,
    ) -> bool {
        if !pivot_depth.is_finite() || pivot_depth <= MIN_DISTANCE || to == from {
            self.transition = None;
            return false;
        }
        self.transition = Some(ViewTransition {
            started: now,
            from,
            to,
            pivot,
            screen_point,
            pivot_depth,
            viewport: from.viewport,
            mesh_revision: self.scene.revision(),
        });
        self.advance_view_transition(now);
        self.transition.is_some()
    }
    /// Advances an active transition and reports whether the displayed pose changed.
    pub fn advance_view_transition(&mut self, now: Instant) -> bool {
        let Some(transition) = self.transition else {
            return false;
        };
        if transition.viewport != self.camera.viewport
            || transition.mesh_revision != self.scene.revision()
        {
            self.transition = None;
            return false;
        }
        let raw = (now
            .saturating_duration_since(transition.started)
            .as_secs_f64()
            / VIEW_TRANSITION_DURATION.as_secs_f64())
        .clamp(0.0, 1.0);
        let amount = raw * raw * (3.0 - 2.0 * raw);
        let next = if raw <= 0.0 {
            Some(transition.from)
        } else if raw >= 1.0 {
            Some(transition.to)
        } else {
            transition.from.interpolate_view_about(
                transition.to,
                amount,
                transition.pivot,
                transition.screen_point,
                transition.pivot_depth,
            )
        };
        let Some(next) = next else {
            self.transition = None;
            return false;
        };
        let before = self.camera;
        self.camera = next;
        if let Some(mut pending) = self.pending_drag {
            pending.camera = self.camera;
            pending.pivot = self.pivot(pending.start);
            pending.depth =
                dot(sub(pending.pivot, self.camera.eye), self.camera.basis().2).max(MIN_DISTANCE);
            self.pending_drag = Some(pending);
        }
        if raw >= 1.0 {
            self.transition = None;
        }
        self.record(before)
    }
    /// Next event-loop wake required while a view transition is active.
    #[must_use]
    pub fn transition_deadline(&self, now: Instant) -> Option<Instant> {
        let transition = self.transition?;
        Some((now + VIEW_TRANSITION_FRAME).min(transition.started + VIEW_TRANSITION_DURATION))
    }
    /// Cancels only an automatic view animation when its window is no longer renderable.
    pub fn cancel_view_transition(&mut self) {
        self.transition = None;
    }
    /// Explicit principal-axis alignment, with a stable vertical-view heading.
    pub fn normal_to_axis(&mut self) -> bool {
        self.cancel();
        let before = self.camera;
        self.camera.normal_to_axis();
        self.record(before)
    }
    /// Fits the complete displayed sphere inside the largest exposed scene rectangle.
    pub fn fit(&mut self) -> bool {
        let Some(bounds) = self.scene.bounds() else {
            return false;
        };
        let Some(region) = exposed_regions(self.camera.viewport, &self.occlusions)
            .into_iter()
            .filter(|r| r.width > 0 && r.height > 0)
            .max_by_key(|r| u64::from(r.width) * u64::from(r.height))
        else {
            return false;
        };
        self.cancel();
        let before = self.camera;
        let viewport = self.camera.viewport;
        let (centre, radius) = bounds.sphere();
        let region_fov = 2.0
            * ((self.camera.vertical_fov * 0.5).tan() * region.height as f64
                / viewport.height as f64)
                .atan();
        let distance = Camera {
            viewport: region,
            vertical_fov: region_fov,
            ..self.camera
        }
        .fit_distance(radius);
        self.camera.eye = sub(centre, scale(self.camera.basis().2, distance));
        self.camera.focal_distance = distance;
        self.camera.orthographic_height = 2.2 * radius * viewport.height as f64
            / (region.height as f64)
                .min(region.width as f64 * viewport.height as f64 / viewport.width as f64);
        self.camera.lens_shift = [
            (2.0 * (region.x as f64 - viewport.x as f64) + region.width as f64)
                / viewport.width as f64
                - 1.0,
            1.0 - (2.0 * (region.y as f64 - viewport.y as f64) + region.height as f64)
                / viewport.height as f64,
        ];
        self.record(before)
    }
    /// Centres the displayed scene in the current viewport without changing its framing.
    /// Used once after startup layout settles; ordinary viewport resizes preserve their
    /// existing screen-space position.
    pub fn center_scene(&mut self) -> bool {
        let Some(bounds) = self.scene.bounds() else {
            return false;
        };
        let before = self.camera;
        let centre = bounds.centre();
        let forward = self.camera.basis().2;
        self.camera.eye = sub(centre, scale(forward, self.camera.focal_distance));
        self.camera.lens_shift = [0.0; 2];
        self.record(before)
    }
    /// Computes the exact post-publication matrices without mutating the displayed query cache.
    /// GPU resources can therefore be uploaded before the new mesh is made visible.
    pub(crate) fn snapshot_for_mesh(
        &self,
        mesh: &NewellMesh,
        center: bool,
    ) -> Option<CameraSnapshot> {
        let mut minimum = [f64::INFINITY; 3];
        let mut maximum = [f64::NEG_INFINITY; 3];
        for index in mesh.indices() {
            let point = mesh.positions().get(usize::try_from(*index).ok()?)?;
            for axis in 0..3 {
                minimum[axis] = minimum[axis].min(f64::from(point[axis]));
                maximum[axis] = maximum[axis].max(f64::from(point[axis]));
            }
        }
        let bounds = (!mesh.indices().is_empty()).then_some(crate::picking::Bounds {
            min: minimum,
            max: maximum,
        });
        let mut camera = self.camera;
        let mut revision = self.revision.wrapping_add(1);
        if center && let Some(bounds) = bounds {
            let before = camera;
            camera.eye = sub(
                bounds.centre(),
                scale(camera.basis().2, camera.focal_distance),
            );
            camera.lens_shift = [0.; 2];
            if camera != before {
                revision = revision.wrapping_add(1);
            }
        }
        camera.snapshot(
            enclosing_sphere(
                bounds.map_or(([0.5; 3], 1.), |bounds| bounds.sphere()),
                WORLD_REFERENCE_CLIP_SPHERE,
            ),
            revision,
        )
    }
    /// Immutable payload for the View. Invalid float conversion never enters GPU uniforms.
    #[must_use]
    pub fn snapshot(&self) -> Option<CameraSnapshot> {
        self.camera.snapshot(
            enclosing_sphere(
                self.scene.bounds().map_or(([0.5; 3], 1.0), |b| b.sphere()),
                WORLD_REFERENCE_CLIP_SPHERE,
            ),
            self.revision,
        )
    }
}

#[cfg(test)]
mod click_certificate_tests {
    use super::*;

    #[test]
    fn camera_clip_encloses_world_references_without_reframing_model_bounds() {
        let model_sphere = ([40.0, 0.0, 0.0], 2.0);
        let refs = WORLD_REFERENCE_CLIP_SPHERE;
        let combined = enclosing_sphere(model_sphere, refs);
        assert!(
            crate::camera::length(sub(combined.0, model_sphere.0)) + model_sphere.1
                <= combined.1 + 1e-9
        );
        assert!(crate::camera::length(combined.0) + refs.1 <= combined.1 + 1e-9);

        let camera = Camera::look_at(
            [0.0, 0.0, 20.0],
            [0.0, 0.0, 0.0],
            SceneViewport::new(0, 0, 640, 480),
        );
        let (near, far) = camera.clip(combined);
        let origin_depth = crate::camera::dot(sub([0.0; 3], camera.eye), camera.basis().2);
        assert!(origin_depth >= near && origin_depth <= far);
    }

    #[test]
    fn click_release_checks_final_displacement_dpi_button_owner_and_cancellation() {
        for scale in [1.0, 2.0] {
            let mut n = Navigation::default();
            n.set_layout(SceneViewport::new(0, 0, 300, 200), Vec::new());
            assert!(n.begin_primary_orbit((100.0, 100.0), scale, true));
            n.release(DragKind::SecondaryPan);
            assert_eq!(
                n.finish_primary((100.0 + 4.0 * scale, 100.0)),
                Some((100.0, 100.0))
            );
            assert!(n.begin_primary_orbit((100.0, 100.0), scale, true));
            assert_eq!(n.finish_primary((100.0 + 4.01 * scale, 100.0)), None);
            assert!(n.begin_primary_orbit((100.0, 100.0), scale, true));
            n.advance_primary_orbit((100.0 + 5.0 * scale, 100.0));
            assert_eq!(n.finish_primary((100.0, 100.0)), None);
            assert!(n.begin_primary_orbit((100.0, 100.0), scale, true));
            n.cancel();
            assert_eq!(n.finish_primary((100.0, 100.0)), None);
        }
    }

    #[test]
    fn origin_reference_picking_uses_projected_geometry_priority_and_ui_occlusion() {
        use newell_ui::OriginReference as R;
        let viewport = SceneViewport::new(0, 0, 640, 480);
        let mut camera = Camera::look_at([0.0, 0.0, 20.0], [0.0; 3], viewport);
        camera.set_perspective(false);
        let mut nav = Navigation::new(camera);
        nav.set_layout(viewport, Vec::new());
        let origin = camera.project([0.0; 3]).unwrap();
        assert_eq!(nav.pick_origin_reference(origin), Some(R::O));
        assert_eq!(
            nav.pick_origin_reference((origin.0 + 10.0, origin.1)),
            Some(R::X)
        );
        assert_eq!(
            nav.pick_origin_reference((origin.0 + 80.0, origin.1 - 80.0)),
            Some(R::XY)
        );

        nav.set_layout(
            viewport,
            vec![SceneViewport::new(
                origin.0.floor() as u32 - 1,
                origin.1.floor() as u32 - 1,
                3,
                3,
            )],
        );
        assert_eq!(nav.pick_origin_reference(origin), None);

        let away = Camera::look_at([0.0, 0.0, -20.0], [0.0, 0.0, -40.0], viewport);
        nav = Navigation::new(away);
        nav.set_layout(viewport, Vec::new());
        assert_eq!(nav.pick_origin_reference((320.0, 240.0)), None);
    }

    #[test]
    fn automatic_view_transition_keeps_context_pivot_and_restarts_from_displayed_pose() {
        use newell_ui::navigation_widgets::ViewDirection;
        let viewport = SceneViewport::new(0, 0, 640, 480);
        for perspective in [false, true] {
            let mut camera = Camera::look_at([7.0, 5.0, 6.0], [3.0, 2.0, 1.0], viewport);
            camera.set_perspective(perspective);
            let pivot = [2.0, 2.5, 1.5];
            let screen = camera.project(pivot).unwrap();
            let depth = dot(sub(pivot, camera.eye), camera.basis().2);
            let mut target = camera;
            target.align_view_about(ViewDirection::new([0, 0, 1]).unwrap(), pivot, screen);
            let middle = camera
                .interpolate_view_about(target, 0.5, pivot, screen, depth)
                .unwrap();
            let projected = middle.project(pivot).unwrap();
            assert!((projected.0 - screen.0).abs() < 1e-8);
            assert!((projected.1 - screen.1).abs() < 1e-8);
            assert_eq!(middle.perspective, perspective);
            assert_eq!(middle.orthographic_height, camera.orthographic_height);
            assert_eq!(middle.vertical_fov, camera.vertical_fov);
        }

        let mut nav = Navigation::new(Camera::look_at([7.0, 5.0, 6.0], [3.0, 2.0, 1.0], viewport));
        nav.set_layout(viewport, Vec::new());
        let now = Instant::now();
        assert!(nav.start_view_transition(ViewDirection::new([0, 0, 1]).unwrap(), now));
        assert!(nav.advance_view_transition(now + Duration::from_millis(125)));
        let displayed = nav.camera();
        assert!(nav.start_view_transition(
            ViewDirection::new([0, -1, 0]).unwrap(),
            now + Duration::from_millis(125),
        ));
        assert_eq!(nav.camera(), displayed);
        let from_second = nav.camera();
        let viewport_center = (
            f64::from(viewport.x) + f64::from(viewport.width) * 0.5,
            f64::from(viewport.y) + f64::from(viewport.height) * 0.5,
        );
        let pivot = nav.pivot(viewport_center);
        let screen = from_second.project(pivot).unwrap();
        let mut expected = from_second;
        expected.align_view_about(ViewDirection::new([0, -1, 0]).unwrap(), pivot, screen);
        assert!(nav.advance_view_transition(now + Duration::from_millis(375)));
        assert_eq!(nav.camera(), expected);
        assert!(
            nav.transition_deadline(now + Duration::from_millis(375))
                .is_none()
        );

        assert!(nav.start_view_transition(
            ViewDirection::new([0, 0, 1]).unwrap(),
            now + Duration::from_millis(400),
        ));
        assert!(nav.begin(DragKind::Pan, (320.0, 240.0), 1.0, true));
        assert!(
            nav.transition_deadline(now + Duration::from_millis(401))
                .is_none()
        );
        nav.cancel();
        assert!(!nav.advance_view_transition(now + Duration::from_millis(700)));
    }
}
