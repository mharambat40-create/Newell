// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use super::{
    Camera, DrawGeometry, DrawTool, Edit, Primitive, SketchRecipe, SketchSupport, SketchText, Tool,
    project_segments, ui_plane,
};
use newell_model::{
    ModelError,
    sketch::{
        SegmentEquation, SegmentRelation, SketchJoin, SketchPointRef as Ref, add, cross, dot,
        finite_curve_contact, sub, unit,
    },
};
use newell_render::{SceneViewport, SketchStroke};
use std::f64::consts::TAU;

type Point = [f64; 2];
const ACQUIRE: f64 = 8.;
const RELEASE: f64 = 12.;
const EXACT: f64 = 1e-7;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Target {
    Point(Ref),
    Curve(u64, usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Hit {
    target: Target,
    point: Point,
    parameter: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Placement {
    pub(super) pointer: Point,
    pub local: Point,
    pub primitive: Option<Primitive>,
    hit: Option<Hit>,
    axis: Option<Axis>,
    tangent: Option<(Ref, bool)>,
}

impl Placement {
    fn signature(self) -> (Option<Target>, Option<Axis>, Option<(Ref, bool)>) {
        (self.hit.map(|hit| hit.target), self.axis, self.tangent)
    }
}

pub(crate) struct DrawAcceptance {
    pub recipe: SketchRecipe,
    pub key: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Slot {
    Point,
    Start,
    End,
    Center,
}

impl Slot {
    fn reference(self, key: u64) -> Ref {
        match self {
            Self::Point => Ref::Point(key),
            Self::Start => Ref::Start(key),
            Self::End => Ref::End(key),
            Self::Center => Ref::Center(key),
        }
    }
}

fn distance(a: Point, b: Point) -> f64 {
    let [u, v] = sub(a, b);
    u.hypot(v)
}

fn shape(recipe: &SketchRecipe, key: u64) -> Option<Primitive> {
    recipe
        .primitives
        .iter()
        .find_map(|&(id, item)| (key == id).then_some(item))
}

fn tangent(recipe: &SketchRecipe, reference: Ref) -> Option<Point> {
    let item = shape(recipe, reference.geometry()?)?;
    if recipe.join_point(reference).is_none() || !matches!(reference, Ref::Start(_) | Ref::End(_)) {
        return None;
    }
    unit(item.endpoint_tangents()?[usize::from(matches!(reference, Ref::End(_)))].1)
}

fn project(support: SketchSupport, camera: Camera, point: Point) -> Option<Point> {
    let [u, v, _] = ui_plane(support).basis();
    camera
        .project(std::array::from_fn(|i| {
            f64::from(u[i]) * point[0] + f64::from(v[i]) * point[1]
        }))
        .map(|(x, y)| [x, y])
}

impl Edit {
    fn active_slot(&self) -> Option<Slot> {
        match self.tool {
            Tool::Line => Some(if self.start.is_none() {
                Slot::Start
            } else {
                Slot::End
            }),
            Tool::Draw(DrawTool::Point) => Some(Slot::Point),
            Tool::Draw(DrawTool::ThreePointArc) if self.start.is_none() => Some(Slot::Start),
            Tool::Draw(DrawTool::ThreePointArc) if self.second.is_some() => Some(Slot::End),
            Tool::Draw(DrawTool::CenterArc) => Some(if self.start.is_none() {
                Slot::Center
            } else if self.second.is_none() {
                Slot::Start
            } else {
                Slot::End
            }),
            Tool::Draw(DrawTool::TangentArc) => Some(if self.start.is_none() {
                Slot::Start
            } else {
                Slot::End
            }),
            _ => None,
        }
    }

    pub(crate) fn magnetic_tool(&self) -> bool {
        matches!(
            self.tool,
            Tool::Line
                | Tool::Draw(
                    DrawTool::Point
                        | DrawTool::ThreePointArc
                        | DrawTool::CenterArc
                        | DrawTool::TangentArc
                )
        )
    }

    fn locus(&self) -> Option<(Point, f64)> {
        if self.tool != Tool::Draw(DrawTool::CenterArc) {
            return None;
        }
        let center = self.start?;
        Some((center, distance(center, self.second?)))
    }

    fn target_hits(
        &self,
        local: Point,
        pointer: Point,
        scale: f64,
        camera: Camera,
        previous: Option<Target>,
        exposed: &[SceneViewport],
    ) -> Vec<Hit> {
        let explicit_tangent = self.tool == Tool::Draw(DrawTool::TangentArc)
            && self.active_slot() == Some(Slot::Start);
        let mut hits = Vec::new();
        let mut admit = |hit: Hit, priority, key, role| {
            let Some(screen) = project(self.recipe.support, camera, hit.point) else {
                return;
            };
            if !exposed
                .iter()
                .any(|rect| rect.contains(screen[0], screen[1]))
            {
                return;
            }
            let radius = if previous == Some(hit.target) {
                RELEASE
            } else {
                ACQUIRE
            };
            let gap = distance(screen, pointer);
            if gap <= radius * scale {
                hits.push((priority, previous != Some(hit.target), gap, key, role, hit));
            }
        };
        let locus = self.locus();
        let compatible =
            |p| locus.is_none_or(|(center, radius)| (distance(p, center) - radius).abs() <= EXACT);
        if !explicit_tangent && compatible([0.; 2]) {
            admit(
                Hit {
                    target: Target::Point(Ref::Origin),
                    point: [0.; 2],
                    parameter: 0.,
                },
                0,
                0,
                0,
            );
        }
        for (role, handle) in self.handles(camera, exposed).into_iter().enumerate() {
            if compatible(handle.local)
                && (!explicit_tangent || tangent(&self.recipe, handle.reference).is_some())
            {
                admit(
                    Hit { target: Target::Point(handle.reference), point: handle.local, parameter: 0.0 },
                    0, handle.reference.geometry().unwrap_or(0), role + 1,
                );
            }
        }
        for &(key, raw_item) in &self.recipe.primitives {
            let Some(item) = self.reference_picks.get(&key).copied()
                .or_else(|| self.recipe.resolved(raw_item).ok()) else { continue };
            if !explicit_tangent {
                let contacts = if let Some((center, radius)) = locus {
                    circle_contacts(item, center, radius, local)
                } else {
                    finite_curve_contact(item, local).into_iter().collect()
                };
                for (role, (point, parameter)) in contacts.into_iter().enumerate() {
                    if parameter > 1e-10 && parameter < 1. - 1e-10 {
                        admit(
                            Hit {
                                target: Target::Curve(key, role),
                                point,
                                parameter,
                            },
                            1,
                            key,
                            role,
                        );
                    }
                }
            }
        }
        hits.sort_by(|a, b| {
            (a.0, a.1)
                .cmp(&(b.0, b.1))
                .then_with(|| a.2.total_cmp(&b.2))
                .then_with(|| (a.3, a.4).cmp(&(b.3, b.4)))
        });
        hits.into_iter().map(|entry| entry.5).collect()
    }

    fn placement(
        &self,
        local: Point,
        pointer: Point,
        scale: f64,
        camera: Camera,
        bypass: bool,
        exposed: &[SceneViewport],
    ) -> Placement {
        let previous = self.snap.get();
        let raw = Placement {
            pointer,
            local,
            primitive: self.preview(local),
            hit: None,
            axis: None,
            tangent: None,
        };
        if bypass || self.solve.is_some() || self.snap_suppressed_at.get() == Some(pointer) {
            return raw;
        }
        let slot = self.active_slot();
        let hits = self.target_hits(local, pointer, scale, camera,
            previous.and_then(|p| p.hit.map(|h| h.target)), exposed);
        let hit = hits.iter().copied().find(|hit| {
            slot.is_none() || (self.candidate(hit.point).is_ok()
                && !(slot != Some(Slot::End) && self.start == Some(hit.point)))
        });
        let hit = hit.filter(|hit| {
            self.tool != Tool::Draw(DrawTool::TangentArc)
                || self.start.is_some()
                || !hits.iter().any(|other| {
                    other.target != hit.target
                        && (distance(other.point, local) - distance(hit.point, local)).abs() <= EXACT
                })
        });
        let at = hit.map_or(local, |hit| {
            if self.coordinate_compatible(hit.point) { hit.point } else { local }
        });
        let mut proposal = Placement {
            local: at,
            primitive: self.preview(at),
            hit,
            ..raw
        };
        if slot != Some(Slot::End) {
            return proposal;
        }
        let start_hit = self
            .attachments
            .iter()
            .find_map(|&(slot, hit)| match (slot, hit.target) {
                (Slot::Start, Target::Point(reference)) => Some((hit.point, reference)),
                _ => None,
            });
        if let Some((start, reference)) = start_hit
            && let Some(direction) = tangent(&self.recipe, reference)
            && let Some(original) = proposal.primitive
            && let Some(ends) = raw.primitive.and_then(Primitive::endpoint_tangents)
            && let Some(provisional) = unit(ends[0].1)
        {
            let limit =
                if previous.is_some_and(|p| p.tangent.is_some_and(|(id, _)| id == reference)) {
                    8.0_f64
                } else {
                    5.0_f64
                };
            let explicit = self.tool == Tool::Draw(DrawTool::TangentArc);
            if explicit || cross(provisional, direction).abs() <= limit.to_radians().sin() {
                let replacement = match self.tool {
                    Tool::Line if hit.is_none() => Some(Primitive::Line {
                        start,
                        end: add(start, direction.map(|v| v * dot(sub(at, start), direction))),
                    }),
                    Tool::Draw(DrawTool::ThreePointArc) => {
                        self.tangent_three_point(start, direction, original, at, hit.is_some())
                    }
                    _ => Some(original),
                };
                if let Some(item) = replacement.filter(|item| item.validate().is_ok())
                    && let Some(ends) = item.endpoint_tangents()
                    && let Some(snapped) = unit(ends[0].1)
                    && cross(snapped, direction).abs() <= 1e-8
                {
                    proposal.primitive = Some(item);
                    proposal.local = ends[1].0;
                    proposal.tangent = Some((reference, dot(snapped, direction) < 0.));
                }
            }
        }
        if self.tool == Tool::Line
            && let Some(start) = self.start
            && let Some(direction) = unit(sub(local, start))
        {
            let axis = if direction[0].abs() >= direction[1].abs() {
                Axis::Horizontal
            } else {
                Axis::Vertical
            };
            let index = usize::from(axis == Axis::Horizontal);
            let limit = if previous.is_some_and(|p| p.axis == Some(axis)) {
                8.0_f64
            } else {
                5.0_f64
            };
            let exact_required = hit.is_some() || proposal.tangent.is_some();
            if exact_required {
                if (proposal.local[index] - start[index]).abs() <= EXACT {
                    proposal.axis = Some(axis);
                }
            } else if direction[index].abs() <= limit.to_radians().sin() {
                let mut end = at;
                end[index] = start[index];
                if let Ok(Some(item)) = self.candidate(end) {
                    proposal.primitive = Some(item);
                    proposal.local = end;
                    proposal.axis = Some(axis);
                }
            }
        }
        proposal
    }

    fn tangent_three_point(
        &self,
        start: Point,
        direction: Point,
        original: Primitive,
        end: Point,
        exact: bool,
    ) -> Option<Primitive> {
        let through = self.second?;
        let Primitive::Draw(DrawGeometry::Arc {
            sweep: original_sweep,
            ..
        }) = original
        else {
            return None;
        };
        let DrawGeometry::Arc { center, radial, .. } =
            DrawGeometry::tangent_arc(start, direction, through)
        else {
            return None;
        };
        let from_center = sub(end, center);
        let radius = distance(center, start);
        if exact && (distance(center, end) - radius).abs() > EXACT {
            return None;
        }
        let end = add(center, unit(from_center)?.map(|v| v * radius));
        let oriented = |p| {
            let p = unit(sub(p, center))?;
            let radial = unit(radial)?;
            Some((original_sweep.signum() * cross(radial, p).atan2(dot(radial, p))).rem_euclid(TAU))
        };
        let angle = oriented(end)?;
        if oriented(through)? >= angle {
            return None;
        }
        Some(Primitive::Draw(DrawGeometry::Arc {
            center,
            radial,
            sweep: angle * original_sweep.signum(),
        }))
    }

    pub(crate) fn update_snap(
        &self,
        local: Point,
        point: (f64, f64),
        scale: f64,
        camera: Camera,
        bypass: bool,
        exposed: &[SceneViewport],
    ) {
        if self.solve.is_some() {
            return;
        }
        if self.snap_suppressed_at.get() != Some([point.0, point.1]) {
            self.snap_suppressed_at.set(None);
        }
        self.snap.set(Some(self.placement(local, [point.0, point.1], scale, camera, bypass, exposed)));
    }

    pub(crate) fn snapped_local(&self, raw: Point) -> Point {
        self.snap.get().map_or(raw, |proposal| proposal.local)
    }

    fn persistent_hit(&self, hit: Hit) -> bool {
        match hit.target {
            Target::Point(reference) => self.recipe.join_point(reference).is_some(),
            Target::Curve(key, _) => self.recipe.join_eligible(key)
                && shape(&self.recipe, key).is_some_and(|shape| {
                    newell_model::sketch::curve_at(shape, hit.parameter).is_some()
                }),
        }
    }

    pub(crate) fn clear_live_snap(&self) {
        self.snap.set(None);
        self.snap_suppressed_at.set(None);
    }

    pub(crate) fn magnetic_click(
        &mut self,
        local: Point,
        point: (f64, f64),
        scale: f64,
        camera: Camera,
        bypass: bool,
        exposed: &[SceneViewport],
    ) -> Result<Option<DrawAcceptance>, ModelError> {
        let previous = self.snap.get();
        let bypass = bypass || self.snap_suppressed_at.get() == Some([point.0, point.1]);
        let placement = self.placement(local, [point.0, point.1], scale, camera, bypass, exposed);
        if !bypass
            && previous.is_some_and(|p| {
                p.signature() != (None, None, None) && p.signature() != placement.signature()
            })
        {
            self.clear_live_snap();
            return Err(ModelError::InvalidSketch(
                "snap target changed; adjust or hold Ctrl",
            ));
        }
        let slot = self.active_slot();
        if self.tool == Tool::Draw(DrawTool::TangentArc)
            && self.start.is_none()
            && let Some(
                hit @ Hit {
                    target: Target::Point(reference),
                    ..
                },
            ) = placement.hit.filter(|hit| self.persistent_hit(*hit))
        {
            self.start = Some(hit.point);
            self.second = tangent(&self.recipe, reference);
            self.attachments.push((Slot::Start, hit));
            self.clear_live_snap();
            return Ok(None);
        }
        // A preview may be a helper stroke for an unfinished arc.
        if self.candidate(placement.local)?.is_none() {
            self.click(placement.local, point, scale, camera)?;
            if let (Some(slot), Some(hit)) = (slot, placement.hit.filter(|hit| hit.point == placement.local && self.persistent_hit(*hit))) {
                self.attachments.push((slot, hit));
            }
            self.clear_live_snap();
            return Ok(None);
        }
        let item = placement
            .primitive
            .ok_or(ModelError::InvalidSketch("invalid snapped geometry"))?;
        let mut attachments = if bypass {
            Vec::new()
        } else {
            self.attachments.clone()
        };
        if let (Some(slot), Some(hit)) = (slot, placement.hit.filter(|hit| hit.point == placement.local && self.persistent_hit(*hit))) {
            attachments.push((slot, hit));
        }
        if attachments.is_empty() && placement.axis.is_none() && placement.tangent.is_none() {
            self.click(placement.local, point, scale, camera)?;
            self.finish_draw_state(self.next_key.saturating_sub(1));
            return Ok(None);
        }
        if self.recipe.primitives.len() >= newell_model::sketch::MAX_PRIMITIVES
            || self.next_key == u64::MAX
        {
            return Err(ModelError::InvalidSketch("Sketch placement capacity"));
        }
        let key = self.next_key;
        let mut recipe = self.recipe.clone();
        recipe.primitives.push((key, item));
        for (slot, hit) in attachments {
            let point = slot.reference(key);
            let join = match hit.target {
                Target::Point(reference) => SketchJoin::Coincident(point, reference),
                Target::Curve(curve, _) => SketchJoin::PointOnCurve {
                    point,
                    curve,
                    parameter: hit.parameter,
                },
            };
            recipe.joins.insert(recipe.next_relation_id()?, join);
        }
        if let Some((source, opposite)) = placement.tangent {
            recipe.joins.insert(
                recipe.next_relation_id()?,
                SketchJoin::Tangent {
                    point: Ref::Start(key),
                    source,
                    opposite,
                },
            );
        }
        if let Some(axis) = placement.axis {
            let relation = match axis {
                Axis::Horizontal => SegmentRelation::Horizontal,
                Axis::Vertical => SegmentRelation::Vertical,
            };
            recipe.segment_equations.insert(
                recipe.next_relation_id()?,
                SegmentEquation {
                    segment: key,
                    relation,
                    formula: None,
                },
            );
        }
        Ok(Some(DrawAcceptance { recipe, key }))
    }

    pub(crate) fn finish_draw_state(&mut self, key: u64) {
        self.clear_gesture();
        if self.tool == Tool::Line
            && let Some(point) = self.recipe.join_point(Ref::End(key))
        {
            self.chain = true;
            self.start = Some(point);
            self.attachments.push((
                Slot::Start,
                Hit {
                    point,
                    parameter: 0.,
                    target: Target::Point(Ref::End(key)),
                },
            ));
        }
    }

    pub(super) fn snap_preview(&self) -> Option<Primitive> {
        self.snap.get().and_then(|proposal| proposal.primitive)
    }

    pub(crate) fn snap_marks(
        &self,
        camera: Camera,
        pointer: Point,
        scale: f64,
        viewport: SceneViewport,
        bypass: bool,
    ) -> Vec<SketchStroke> {
        let mut strokes = Vec::new();
        if bypass || self.solve.is_some() {
            return strokes;
        }
        let Some(proposal) = self.snap.get() else {
            return strokes;
        };
        let mut labels = Vec::new();
        for (_, hit) in self
            .attachments
            .iter()
            .copied()
            .chain(proposal.hit.map(|hit| (Slot::Point, hit)))
        {
            let label = match hit.target {
                Target::Point(_) => "C",
                Target::Curve(..) => "P",
            };
            if self.magnetic_tool() && self.persistent_hit(hit) && !labels.contains(&label) {
                labels.push(label);
            }
            if let Some(center) = project(self.recipe.support, camera, hit.point) {
                diamond(&mut strokes, center, 4. * scale);
            }
        }
        if let Some(axis) = proposal.axis {
            labels.push(match axis {
                Axis::Horizontal => "H",
                Axis::Vertical => "V",
            });
        }
        let explicit = self.tool == Tool::Draw(DrawTool::TangentArc);
        let source = proposal.tangent.map(|(source, _)| source).or_else(|| {
            if !explicit {
                return None;
            }
            let hit = self
                .attachments
                .first()
                .map(|(_, hit)| *hit)
                .or(proposal.hit)?;
            let Target::Point(reference) = hit.target else {
                return None;
            };
            tangent(&self.recipe, reference)?;
            Some(reference)
        });
        if let Some(source) = source {
            labels.push("T");
            if let Some(curve) = source.geometry().and_then(|key| shape(&self.recipe, key)) {
                trace(&mut strokes, self.recipe.support, curve, camera, scale);
            }
        }
        if !labels.is_empty() {
            let caption = labels.join(" ");
            let width = 8. * caption.len() as f64 * scale;
            let x = f64::from(viewport.x);
            let y = f64::from(viewport.y);
            let r = x + f64::from(viewport.width);
            let b = y + f64::from(viewport.height);
            let margin = 12. * scale;
            let left = if pointer[0] + margin + width <= r {
                pointer[0] + margin
            } else {
                (pointer[0] - margin - width).max(x)
            };
            let baseline = if pointer[1] - 2. * margin >= y {
                pointer[1] - margin
            } else {
                (pointer[1] + 2. * margin).min(b)
            };
            label_strokes(&mut strokes, &caption, [left, baseline], scale);
        }
        strokes
    }
}

fn circle_contacts(item: Primitive, center: Point, radius: f64, aim: Point) -> Vec<(Point, f64)> {
    let points = match item {
        Primitive::Line { start, end } => {
            let Some(direction) = unit(sub(end, start)) else {
                return Vec::new();
            };
            let foot = add(
                start,
                direction.map(|v| v * dot(sub(center, start), direction)),
            );
            let height_squared = radius.powi(2) - distance(center, foot).powi(2);
            if height_squared < 0. {
                return Vec::new();
            }
            [-1., 1.]
                .map(|sign| add(foot, direction.map(|v| sign * v * height_squared.sqrt())))
                .to_vec()
        }
        Primitive::Draw(DrawGeometry::Arc {
            center: other,
            radial,
            ..
        }) => {
            let gap = distance(other, center);
            let Some(direction) = unit(sub(other, center)) else {
                return finite_curve_contact(item, aim)
                    .filter(|(p, _)| (distance(*p, center) - radius).abs() <= EXACT)
                    .into_iter()
                    .collect();
            };
            let along = (gap * gap + radius * radius - dot(radial, radial)) / (2. * gap);
            let height_squared = radius * radius - along * along;
            if height_squared < 0. {
                return Vec::new();
            }
            let foot = add(center, direction.map(|v| v * along));
            let normal = [-direction[1], direction[0]];
            [-1., 1.]
                .map(|sign| add(foot, normal.map(|v| sign * v * height_squared.sqrt())))
                .to_vec()
        }
        _ => return Vec::new(),
    };
    let mut contacts: Vec<_> = points
        .into_iter()
        .filter_map(|p| {
            let (contact, parameter) = finite_curve_contact(item, p)?;
            (distance(p, contact) <= EXACT).then_some((contact, parameter))
        })
        .collect();
    contacts.sort_by(|a, b| a.1.total_cmp(&b.1));
    contacts.dedup_by(|a, b| (a.1 - b.1).abs() <= 1e-12);
    contacts
}

fn stroke(out: &mut Vec<SketchStroke>, a: Point, b: Point) {
    out.push(SketchStroke {
        start: a.map(|x| x as f32),
        end: b.map(|x| x as f32),
        selected: false,
    });
}

fn diamond(out: &mut Vec<SketchStroke>, center: Point, radius: f64) {
    let corners = [[radius, 0.], [0., radius], [-radius, 0.], [0., -radius]];
    for i in 0..4 {
        stroke(
            out,
            add(center, corners[i]),
            add(center, corners[(i + 1) % 4]),
        );
    }
}

fn label_strokes(out: &mut Vec<SketchStroke>, label: &str, at: Point, scale: f64) {
    let [x, y] = at;
    if let Ok(text) = SketchText::new(label, [x, -y], [x + 1., -y], 12. * scale) {
        for ([x, y], [u, v]) in DrawGeometry::Text(text).display_segments() {
            stroke(out, [x, -y], [u, -v]);
        }
    }
}

fn clip_screen_segment(
    camera: Camera,
    start: Point,
    end: Point,
    padding: f64,
) -> Option<(f64, f64)> {
    let viewport = camera.viewport;
    let x = f64::from(viewport.x);
    let y = f64::from(viewport.y);
    let lower = [x - padding, y - padding];
    let upper = [
        x + f64::from(viewport.width) + padding,
        y + f64::from(viewport.height) + padding,
    ];
    let mut first = 0.0_f64;
    let mut last = 1.0_f64;
    for i in 0..2 {
        let delta = end[i] - start[i];
        if delta == 0. {
            if start[i] < lower[i] || start[i] > upper[i] {
                return None;
            }
        } else {
            let a = (lower[i] - start[i]) / delta;
            let b = (upper[i] - start[i]) / delta;
            first = first.max(a.min(b));
            last = last.min(a.max(b));
        }
    }
    (first <= last).then_some((first, last))
}

fn trace(
    out: &mut Vec<SketchStroke>,
    support: SketchSupport,
    item: Primitive,
    camera: Camera,
    scale: f64,
) {
    let dash = 3. * scale;
    let mut phase = 0.;
    for ((ax, ay), (bx, by)) in project_segments(support, item, camera) {
        let delta = [bx - ax, by - ay];
        let Some(direction) = unit(delta) else {
            continue;
        };
        let length = distance([ax, ay], [bx, by]);
        // Clip the source trace to an expanded screen rectangle before emitting dashes.
        let Some((first, last)) = clip_screen_segment(camera, [ax, ay], [bx, by], dash) else {
            phase = (phase + length).rem_euclid(2. * dash);
            continue;
        };
        let normal = [-direction[1] * dash, direction[0] * dash];
        let origin = add([ax, ay], normal);
        let mut d = (first * length + phase).div_euclid(2. * dash) * 2. * dash - phase;
        let limit = (f64::from(camera.viewport.width).hypot(f64::from(camera.viewport.height))
            / (2. * dash))
            .ceil() as usize
            + 4;
        for _ in 0..limit {
            if d >= last * length {
                break;
            }
            let start = d.max(first * length);
            let end = (d + dash).min(last * length);
            if end > start {
                stroke(
                    out,
                    add(origin, direction.map(|v| v * start)),
                    add(origin, direction.map(|v| v * end)),
                );
            }
            d += 2. * dash;
        }
        phase = (phase + length).rem_euclid(2. * dash);
    }
}

pub(crate) fn join_strokes(recipe: &SketchRecipe, camera: Camera, scale: f64) -> Vec<SketchStroke> {
    let mut strokes = Vec::new();
    let mut contacts = std::collections::BTreeMap::new();
    for &join in recipe.joins.values() {
        let (reference, label) = match join {
            SketchJoin::Coincident(reference, _) => (reference, "C"),
            SketchJoin::PointOnCurve { point, .. } => (point, "P"),
            SketchJoin::Tangent { point, .. } => (point, "T"),
        };
        let order = match reference {
            Ref::Origin => (0, 0),
            Ref::Point(id) => (id, 1),
            Ref::Start(id) => (id, 2),
            Ref::End(id) => (id, 3),
            Ref::Center(id) => (id, 4),
            Ref::Defining(..) => continue,
        };
        contacts
            .entry(order)
            .or_insert_with(|| (reference, Vec::new()))
            .1
            .push(label);
    }
    for (_, (reference, labels)) in contacts {
        if let Some(point) = recipe
            .join_point(reference)
            .and_then(|point| project(recipe.support, camera, point))
        {
            label_strokes(
                &mut strokes,
                &labels.join(" "),
                add(point, [6. * scale, -6. * scale]),
                scale,
            );
        }
    }
    strokes
}

#[cfg(test)]
mod tests {
    use super::*;
    use newell_ui::OriginPlane;

    #[test]
    fn magnetic_positions_prioritize_points_and_latch_in_logical_pixels() {
        let mut camera =
            Camera::look_at([0., 0., 10.], [0.; 3], SceneViewport::new(0, 0, 800, 600));
        camera.set_perspective(false);
        let mut recipe = super::super::empty_recipe(OriginPlane::XY);
        recipe.primitives = vec![
            (1, Primitive::Draw(DrawGeometry::Point([3., 1.]))),
            (2, Primitive::Draw(DrawGeometry::Point([3., 1.]))),
            (
                3,
                Primitive::Line {
                    start: [2., 1.],
                    end: [4., 1.],
                },
            ),
        ];
        let mut edit = Edit::new(
            crate::document::DocumentSession::default().version(),
            None,
            None,
            camera,
            recipe,
        );
        edit.set_tool(Tool::Draw(DrawTool::Point));
        for height in [10., 20.] {
            camera.orthographic_height = height;
            let exposed = [camera.viewport];
            let [x, y] = project(SketchSupport::XY, camera, [3., 1.]).unwrap();
            let hit_at = |offset: f64| [3., 1. - offset * height / 600.];
            edit.clear_live_snap();
            for (offset, expected) in [(15., true), (21., true), (25., false)] {
                edit.update_snap(hit_at(offset), (x, y + offset), 2., camera, false, &exposed);
                assert_eq!(
                    edit.snap.get().unwrap().hit.map(|hit| hit.target)
                        == Some(Target::Point(Ref::Point(1))),
                    expected
                );
            }
            edit.update_snap([3., 1.], (x, y), 2., camera, true, &exposed);
            assert!(edit.snap.get().unwrap().hit.is_none());
            edit.update_snap([3., 1.], (x, y), 2., camera, false, &exposed);
            assert!(edit.snap.get().unwrap().hit.is_some());
        }
    }

    #[test]
    fn magnetic_axis_releases_and_exact_point_wins_over_direction() {
        let mut camera =
            Camera::look_at([0., 0., 10.], [0.; 3], SceneViewport::new(0, 0, 800, 600));
        camera.set_perspective(false);
        let exposed = [camera.viewport];
        let recipe = super::super::empty_recipe(OriginPlane::XY);
        let mut edit = Edit::new(
            crate::document::DocumentSession::default().version(),
            None,
            None,
            camera,
            recipe,
        );
        edit.set_tool(Tool::Line);
        edit.start = Some([1., 1.]);
        for (angle, axis) in [
            (4., Some(Axis::Horizontal)),
            (7., Some(Axis::Horizontal)),
            (9., None),
        ] {
            let p = [4., 1. + 3. * f64::to_radians(angle).tan()];
            let [x, y] = project(SketchSupport::XY, camera, p).unwrap();
            edit.update_snap(p, (x, y), 1., camera, false, &exposed);
            assert_eq!(edit.snap.get().unwrap().axis, axis);
        }

        let target = [4., 1.2];
        edit.recipe
            .primitives
            .push((1, Primitive::Draw(DrawGeometry::Point(target))));
        let [x, y] = project(SketchSupport::XY, camera, target).unwrap();
        edit.update_snap(target, (x, y), 1., camera, false, &exposed);
        assert_eq!(
            edit.snap.get().unwrap().hit.unwrap().target,
            Target::Point(Ref::Point(1))
        );
        assert_eq!(edit.snap.get().unwrap().axis, None);
        edit.recipe.primitives.clear();
        assert!(
            edit.magnetic_click(target, (x, y), 1., camera, false, &exposed)
                .is_err()
        );
        assert_eq!(edit.start, Some([1., 1.]));

        edit.recipe
            .primitives
            .push((1, Primitive::Draw(DrawGeometry::Point(target))));
        edit.update_snap(target, (x, y), 1., camera, false, &exposed);
        assert!(!edit.escape());
        edit.update_snap(target, (x, y), 1., camera, false, &exposed);
        assert!(edit.snap.get().unwrap().hit.is_none());
        edit.update_snap(target, (x + 1., y), 1., camera, false, &exposed);
        assert!(edit.snap.get().unwrap().hit.is_some());
    }

    #[test]
    fn magnetic_three_point_arc_preserves_through_click_and_solves_tangent()
    -> Result<(), ModelError> {
        let mut camera =
            Camera::look_at([0., 0., 10.], [0.; 3], SceneViewport::new(0, 0, 1000, 600));
        camera.set_perspective(false);
        let exposed = [camera.viewport];
        let mut recipe = super::super::empty_recipe(OriginPlane::XY);
        recipe.primitives.push((
            0,
            Primitive::Line {
                start: [1., 1.],
                end: [3., 1.],
            },
        ));
        let mut edit = Edit::new(
            crate::document::DocumentSession::default().version(),
            None,
            None,
            camera,
            recipe,
        );
        edit.set_tool(Tool::Draw(DrawTool::ThreePointArc));
        let through = [3. + 2.0_f64.sqrt(), 3. - 2.0_f64.sqrt()];
        for p in [[3., 1.], through] {
            let [x, y] = project(SketchSupport::XY, camera, p).unwrap();
            assert!(
                edit.magnetic_click(p, (x, y), 1., camera, false, &exposed)?
                    .is_none()
            );
        }
        let p = [5.05, 3.];
        let [x, y] = project(SketchSupport::XY, camera, p).unwrap();
        edit.update_snap(p, (x, y), 1., camera, false, &exposed);
        let placement = edit
            .magnetic_click(p, (x, y), 1., camera, false, &exposed)?
            .unwrap();
        assert_eq!(placement.recipe.joins.len(), 2);
        placement.recipe.solve_placement(|| false)?;
        let (snapped_through, param) =
            finite_curve_contact(placement.recipe.primitives[1].1, through).unwrap();
        assert!(param > 0. && param < 1.);
        assert!(distance(snapped_through, through) < 1e-7);
        Ok(())
    }
}
