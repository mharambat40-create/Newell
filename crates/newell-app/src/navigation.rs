// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Per-window camera controller and gesture ownership, independent of the GPU.

use crate::camera::{Camera, MIN_DISTANCE, Vec3, dot, plane_hit, scale, sub};
use crate::input::NavigationGesture;
use crate::picking::{PickHit, PickingScene};
use newell_kernel::NewellMesh;
use newell_render::{CameraSnapshot, SceneViewport, exposed_regions};
use winit::event::{MouseButton, TouchPhase};

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
    /// Cancels captures without altering pose. Ongoing trackpad streams stay rejected.
    pub fn cancel(&mut self) {
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
    /// Immutable payload for the View. Invalid float conversion never enters GPU uniforms.
    #[must_use]
    pub fn snapshot(&self) -> Option<CameraSnapshot> {
        self.camera.snapshot(
            self.scene.bounds().map_or(([0.5; 3], 1.0), |b| b.sphere()),
            self.revision,
        )
    }
}

#[cfg(test)]
mod click_certificate_tests {
    use super::*;
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
}
