// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Exact line-region decomposition and recombination, preserving input holes.
use crate::{NewellKernelError, operation_error};
use brepkit_math::{
    polygon_boolean::{BooleanOp, PolygonBooleanResult, polygon_boolean, signed_area},
    predicates::winding_number,
    vec::Point2,
};
use std::collections::BTreeMap;
const EPS: f64 = 1e-7;
type Triangle = [Point2; 3];
#[derive(Debug, Clone)]
pub(crate) struct Region {
    pub outer: Vec<Point2>,
    pub holes: Vec<Vec<Point2>>,
}
impl Region {
    pub(crate) fn contains(&self, point: Point2) -> bool {
        winding_number(point, &self.outer) != 0
            && !self
                .holes
                .iter()
                .any(|hole| winding_number(point, hole) != 0)
    }
}
fn regions(result: PolygonBooleanResult) -> Result<Vec<Region>, NewellKernelError> {
    let mut regions: Vec<_> = result
        .outer
        .into_iter()
        .map(|outer| Region {
            outer,
            holes: Vec::new(),
        })
        .collect();
    for hole in result.holes {
        let point = *hole
            .first()
            .ok_or_else(|| operation_error("empty polygon hole"))?;
        let owners: Vec<_> = regions
            .iter()
            .enumerate()
            .filter(|(_, region)| winding_number(point, &region.outer) != 0)
            .map(|(index, _)| index)
            .collect();
        if owners.len() != 1 {
            return Err(operation_error("ambiguous polygon hole"));
        }
        regions[owners[0]].holes.push(hole);
    }
    Ok(regions)
}
fn triangulate(region: &Region) -> Result<Vec<Triangle>, NewellKernelError> {
    use brepkit_math::cdt::Cdt;
    let all: Vec<_> = region
        .outer
        .iter()
        .chain(region.holes.iter().flatten())
        .copied()
        .collect();
    if all.len() < 3 {
        return Err(operation_error("degenerate planar region"));
    }
    let mut low = [f64::INFINITY; 2];
    let mut high = [f64::NEG_INFINITY; 2];
    for point in &all {
        if !point.x().is_finite() || !point.y().is_finite() {
            return Err(operation_error("nonfinite planar region"));
        }
        low[0] = low[0].min(point.x());
        low[1] = low[1].min(point.y());
        high[0] = high[0].max(point.x());
        high[1] = high[1].max(point.y());
    }
    let mut cdt = Cdt::new((Point2::new(low[0], low[1]), Point2::new(high[0], high[1])));
    for boundary in std::iter::once(&region.outer).chain(region.holes.iter()) {
        let ids = boundary
            .iter()
            .map(|point| cdt.insert_point(*point).map_err(operation_error))
            .collect::<Result<Vec<_>, _>>()?;
        if ids.len() < 3 {
            return Err(operation_error("degenerate region boundary"));
        }
        for (a, b) in ids.iter().zip(ids.iter().cycle().skip(1)).take(ids.len()) {
            cdt.insert_constraint(*a, *b).map_err(operation_error)?;
        }
    }
    let points = cdt.vertices();
    let mut result = Vec::new();
    for (a, b, c) in cdt.triangles() {
        let triangle = [points[a], points[b], points[c]];
        let center = Point2::new(
            (triangle[0].x() + triangle[1].x() + triangle[2].x()) / 3.,
            (triangle[0].y() + triangle[1].y() + triangle[2].y()) / 3.,
        );
        if region.contains(center) {
            result.push(triangle);
        }
    }
    Ok(result)
}
fn subtract(
    mut source: Vec<Triangle>,
    cutters: &[Triangle],
) -> Result<Vec<Triangle>, NewellKernelError> {
    for cutter in cutters {
        let mut next = Vec::new();
        for triangle in source {
            let result = polygon_boolean(&triangle, cutter, BooleanOp::Difference, EPS);
            for region in regions(result)? {
                next.extend(triangulate(&region)?);
            }
        }
        source = next;
        if source.is_empty() {
            break;
        }
        if source.len() > 65536 {
            return Err(operation_error("planar arrangement complexity"));
        }
    }
    Ok(source)
}
#[cfg(test)]
fn triangles_area(triangles: &[Triangle]) -> f64 {
    triangles
        .iter()
        .map(|triangle| signed_area(triangle).abs())
        .sum()
}
fn point_index(points: &mut Vec<Point2>, point: Point2) -> usize {
    if let Some(index) = points.iter().position(|p| (*p - point).length() <= EPS) {
        index
    } else {
        let index = points.len();
        points.push(point);
        index
    }
}
fn join_triangles(triangles: &[Triangle]) -> Result<Vec<Region>, NewellKernelError> {
    let mut points = Vec::new();
    let mut triangles_indices = Vec::new();
    for triangle in triangles {
        let mut triangle = *triangle;
        if signed_area(&triangle) < 0. {
            triangle.swap(1, 2);
        }
        triangles_indices.push(triangle.map(|point| point_index(&mut points, point)));
    }
    let mut counts: BTreeMap<(usize, usize), i32> = BTreeMap::new();
    // Split every boundary at all arrangement vertices before cancelling internal
    // edges. A triangulation's T-junction can never become an artificial sheet rim.
    for triangle in triangles_indices {
        for (a, b) in triangle.iter().zip(triangle.iter().cycle().skip(1)).take(3) {
            let start = points[*a];
            let direction = points[*b] - start;
            let length2 = direction.dot(direction);
            if length2 <= EPS * EPS {
                continue;
            }
            let mut split = vec![(0., *a), (1., *b)];
            for (index, point) in points.iter().enumerate() {
                let t = (*point - start).dot(direction) / length2;
                if t > 0. && t < 1. && (*point - (start + direction * t)).length() <= EPS {
                    split.push((t, index));
                }
            }
            split.sort_by(|a, b| a.0.total_cmp(&b.0));
            split.dedup_by_key(|p| p.1);
            for part in split.windows(2) {
                let (a, b) = (part[0].1, part[1].1);
                if a == b {
                    continue;
                }
                let (edge, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                *counts.entry(edge).or_default() += sign;
            }
        }
    }
    let mut next = BTreeMap::new();
    let mut incoming = BTreeMap::new();
    for ((a, b), count) in counts {
        let (a, b) = match count {
            0 => continue,
            1 => (a, b),
            -1 => (b, a),
            _ => return Err(operation_error("overlapping planar arrangement")),
        };
        if next.insert(a, b).is_some() || incoming.insert(b, a).is_some() {
            return Err(operation_error("nonmanifold planar junction"));
        }
    }
    let mut loops = Vec::new();
    while let Some((&first, _)) = next.first_key_value() {
        let mut boundary = Vec::new();
        let mut current = first;
        loop {
            boundary.push(points[current]);
            current = next
                .remove(&current)
                .ok_or_else(|| operation_error("open planar arrangement"))?;
            if current == first {
                break;
            }
            if boundary.len() > points.len() {
                return Err(operation_error("cyclic planar arrangement"));
            }
        }
        if boundary.len() >= 3 && signed_area(&boundary).abs() > EPS * EPS {
            loops.push(boundary);
        }
    }
    let mut result = PolygonBooleanResult::default();
    for boundary in loops {
        if signed_area(&boundary) > 0. {
            result.outer.push(boundary);
        } else {
            result.holes.push(boundary);
        }
    }
    regions(result)
}
/// Boolean regions use exact line triangles solely as a decomposition. Only the
/// recombined outer/hole loops become B-Rep boundaries; no display mesh is involved.
pub(crate) fn region_boolean(
    a: &[Region],
    b: &[Region],
    mode: crate::NewellBooleanMode,
) -> Result<Vec<Region>, NewellKernelError> {
    let mut left = Vec::new();
    for region in a {
        let triangles = triangulate(region)?;
        let remainder = subtract(triangles, &left)?;
        left.extend(remainder);
    }
    let mut right = Vec::new();
    for region in b {
        let triangles = triangulate(region)?;
        let remainder = subtract(triangles, &right)?;
        right.extend(remainder);
    }
    let result = match mode {
        crate::NewellBooleanMode::Add => {
            let extra = subtract(right, &left)?;
            left.extend(extra);
            left
        }
        crate::NewellBooleanMode::Remove => subtract(left, &right)?,
    };
    join_triangles(&result)
}
pub(crate) fn region_area(regions: &[Region]) -> f64 {
    regions
        .iter()
        .map(|region| {
            signed_area(&region.outer).abs()
                - region
                    .holes
                    .iter()
                    .map(|hole| signed_area(hole).abs())
                    .sum::<f64>()
        })
        .sum()
}

pub(crate) fn shared_boundary(a: &[Region], b: &[Region]) -> bool {
    for a in a {
        for aloop in std::iter::once(&a.outer).chain(&a.holes) {
            for (a0, a1) in aloop
                .iter()
                .zip(aloop.iter().cycle().skip(1))
                .take(aloop.len())
            {
                let direction = *a1 - *a0;
                let length = direction.length();
                if length <= EPS {
                    continue;
                }
                let direction = direction * (1. / length);
                for b in b {
                    for bloop in std::iter::once(&b.outer).chain(&b.holes) {
                        for (b0, b1) in bloop
                            .iter()
                            .zip(bloop.iter().cycle().skip(1))
                            .take(bloop.len())
                        {
                            let x0 = (*b0 - *a0).dot(direction);
                            let x1 = (*b1 - *a0).dot(direction);
                            if (*b0 - (*a0 + direction * x0)).length() <= EPS
                                && (*b1 - (*a0 + direction * x1)).length() <= EPS
                                && length.min(x0.max(x1)) - 0_f64.max(x0.min(x1)) > EPS
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}
pub(crate) fn line_intervals(
    regions: &[Region],
    origin: Point2,
    direction: brepkit_math::vec::Vec2,
) -> Vec<(f64, f64, bool)> {
    let cross =
        |a: brepkit_math::vec::Vec2, b: brepkit_math::vec::Vec2| a.x() * b.y() - a.y() * b.x();
    let length2 = direction.dot(direction);
    if length2 <= 1e-24 {
        return Vec::new();
    }
    let mut result = Vec::new();
    for region in regions {
        let mut stations = Vec::new();
        for boundary in std::iter::once(&region.outer).chain(&region.holes) {
            for (a, b) in boundary
                .iter()
                .zip(boundary.iter().cycle().skip(1))
                .take(boundary.len())
            {
                let edge = *b - *a;
                let denominator = cross(direction, edge);
                if denominator.abs() > 1e-12 * direction.length() * edge.length() {
                    let t = cross(*a - origin, direction) / denominator;
                    if (-EPS..=1. + EPS).contains(&t) {
                        stations.push(cross(*a - origin, edge) / denominator);
                    }
                } else if cross(*a - origin, direction).abs() <= EPS * direction.length() {
                    stations.push((*a - origin).dot(direction) / length2);
                    stations.push((*b - origin).dot(direction) / length2);
                }
            }
        }
        stations.sort_by(f64::total_cmp);
        stations.dedup_by(|a, b| (*a - *b).abs() <= EPS);
        for span in stations.windows(2) {
            if span[1] - span[0] <= EPS {
                continue;
            }
            let point = origin + direction * ((span[0] + span[1]) * 0.5);
            let boundary = std::iter::once(&region.outer)
                .chain(&region.holes)
                .any(|boundary| {
                    boundary
                        .iter()
                        .zip(boundary.iter().cycle().skip(1))
                        .take(boundary.len())
                        .any(|(a, b)| {
                            let edge = *b - *a;
                            let t = (point - *a).dot(edge) / edge.dot(edge);
                            (-EPS..=1. + EPS).contains(&t)
                                && (point - (*a + edge * t)).length() <= EPS
                        })
                });
            if boundary || region.contains(point) {
                result.push((span[0], span[1], boundary));
            }
        }
    }
    result
}

use crate::{NewellBooleanMode, NewellGeometry, NewellProfileFrame, NewellShapeKey};
use brepkit_math::vec::{Point3, Vec3};
use brepkit_topology::{
    FaceId,
    edge::{Edge, EdgeCurve},
    face::{Face, FaceSurface},
    vertex::Vertex,
    wire::{OrientedEdge, Wire},
};
#[derive(Clone)]
pub(crate) struct PlaneRegions {
    pub(crate) frame: NewellProfileFrame,
    pub(crate) regions: Vec<Region>,
}
fn vector(a: [f64; 3]) -> Vec3 {
    Vec3::new(a[0], a[1], a[2])
}
pub(crate) fn project(frame: NewellProfileFrame, p: Point3) -> Point2 {
    let relative = Vec3::new(p.x(), p.y(), p.z()) - vector(frame.origin);
    Point2::new(relative.dot(vector(frame.u)), relative.dot(vector(frame.v)))
}
pub(crate) fn unproject(frame: NewellProfileFrame, p: Point2) -> Point3 {
    let p = vector(frame.origin) + vector(frame.u) * p.x() + vector(frame.v) * p.y();
    Point3::new(p.x(), p.y(), p.z())
}
fn same_plane(a: NewellProfileFrame, b: NewellProfileFrame) -> bool {
    let n = vector(a.normal);
    n.cross(vector(b.normal)).length() <= 1e-10
        && n.dot(vector(a.origin) - vector(b.origin)).abs() <= EPS
}
fn reframe(group: &PlaneRegions, frame: NewellProfileFrame) -> Vec<Region> {
    group
        .regions
        .iter()
        .map(|region| Region {
            outer: region
                .outer
                .iter()
                .map(|p| project(frame, unproject(group.frame, *p)))
                .collect(),
            holes: region
                .holes
                .iter()
                .map(|hole| {
                    hole.iter()
                        .map(|p| project(frame, unproject(group.frame, *p)))
                        .collect()
                })
                .collect(),
        })
        .collect()
}
pub(crate) fn merge_groups(
    groups: &mut Vec<PlaneRegions>,
    other: &[PlaneRegions],
) -> Result<(), NewellKernelError> {
    for group in other {
        if let Some(existing) = groups
            .iter_mut()
            .find(|existing| same_plane(existing.frame, group.frame))
        {
            existing.regions = region_boolean(
                &existing.regions,
                &reframe(group, existing.frame),
                NewellBooleanMode::Add,
            )?;
        } else {
            groups.push(group.clone());
        }
    }
    Ok(())
}
fn groups_connect(a: &[PlaneRegions], b: &[PlaneRegions]) -> Result<bool, NewellKernelError> {
    let mut connected = false;
    for a in a {
        for b in b {
            if same_plane(a.frame, b.frame) {
                let other = reframe(b, a.frame);
                let merged = region_boolean(&a.regions, &other, NewellBooleanMode::Add)?;
                let overlap = region_area(&a.regions) + region_area(&other) - region_area(&merged);
                connected |= overlap > EPS * EPS || shared_boundary(&a.regions, &other);
            } else {
                let n = vector(a.frame.normal);
                let m = vector(b.frame.normal);
                let cross = n.cross(m);
                let length = cross.length();
                if length <= 1e-10 {
                    continue;
                }
                let h = n.dot(vector(a.frame.origin));
                let k = m.dot(vector(b.frame.origin));
                let point = (m.cross(cross) * h + cross.cross(n) * k) * (1. / cross.dot(cross));
                let direction = cross * (1. / length);
                let point = Point3::new(point.x(), point.y(), point.z());
                let spans = |group: &PlaneRegions| {
                    line_intervals(
                        &group.regions,
                        project(group.frame, point),
                        brepkit_math::vec::Vec2::new(
                            direction.dot(vector(group.frame.u)),
                            direction.dot(vector(group.frame.v)),
                        ),
                    )
                };
                for left in spans(a) {
                    for right in spans(b) {
                        if left.1.min(right.1) - left.0.max(right.0) > EPS {
                            if !left.2 || !right.2 {
                                return Err(operation_error("nonmanifold transverse sheet union"));
                            }
                            connected = true;
                        }
                    }
                }
            }
        }
    }
    Ok(connected)
}
/// Connected sheet results and the exact target versions they replace.
#[derive(Debug)]
pub struct NewellSurfaceResult {
    pub outputs: Vec<NewellShapeKey>,
    pub changed_targets: Vec<usize>,
}
#[derive(Clone)]
struct DomainBand {
    piece: crate::surface::SweepDomainPiece,
    tool: bool,
    owners: std::collections::BTreeSet<usize>,
    common: std::collections::BTreeSet<usize>,
}
impl NewellGeometry {
    fn line_surface_groups(
        &self,
        key: NewellShapeKey,
    ) -> Result<Vec<PlaneRegions>, NewellKernelError> {
        let faces = self
            .surfaces
            .get(&key)
            .ok_or_else(|| operation_error("surface target required"))?;
        let mut result: Vec<PlaneRegions> = Vec::new();
        for id in faces {
            if !matches!(
                self.topology.face(*id).map_err(operation_error)?.surface(),
                brepkit_topology::face::FaceSurface::Plane { .. }
            ) {
                return Err(operation_error(
                    "curved sheet trim requires a verified common-domain route",
                ));
            }
            let frame = crate::profile::supporting_frame(&self.topology, *id)?;
            let existing = result
                .iter()
                .position(|group| same_plane(group.frame, frame));
            let projection = existing.map_or(frame, |index| result[index].frame);
            let face = self.topology.face(*id).map_err(operation_error)?;
            let mut loops = Vec::new();
            for wire in std::iter::once(face.outer_wire()).chain(face.inner_wires().iter().copied())
            {
                let mut points = Vec::new();
                for occurrence in self.topology.wire(wire).map_err(operation_error)?.edges() {
                    let edge = self
                        .topology
                        .edge(occurrence.edge())
                        .map_err(operation_error)?;
                    if !matches!(edge.curve(), EdgeCurve::Line) {
                        return Err(operation_error(
                            "curved sheet trim requires a verified common-domain route",
                        ));
                    }
                    let vertex = if occurrence.is_forward() {
                        edge.start()
                    } else {
                        edge.end()
                    };
                    points.push(project(
                        projection,
                        self.topology
                            .vertex(vertex)
                            .map_err(operation_error)?
                            .point(),
                    ));
                }
                loops.push(points);
            }
            if loops.is_empty() {
                return Err(operation_error("missing sheet boundary"));
            }
            let region = Region {
                outer: loops.remove(0),
                holes: loops,
            };
            if let Some(index) = existing {
                result[index].regions =
                    region_boolean(&result[index].regions, &[region], NewellBooleanMode::Add)?;
            } else {
                result.push(PlaneRegions {
                    frame: projection,
                    regions: vec![region],
                });
            }
        }
        Ok(result)
    }
    /// Exact regular planar sheet union and area difference, including existing holes.
    /// Reconstruction welds actual boundaries and rejects nonmanifold crossings.
    ///
    /// # Errors
    /// Rejects unchanged targets, nonmanifold results, missing common-domain proof or native errors.
    pub fn combine_surfaces(
        &mut self,
        tools: &[NewellShapeKey],
        targets: &[NewellShapeKey],
        mode: NewellBooleanMode,
    ) -> Result<NewellSurfaceResult, NewellKernelError> {
        if let Some(result) = self.combine_partitioned_sweeps(tools, targets, mode)? {
            return Ok(result);
        }
        let mut tool = Vec::new();
        for key in tools {
            merge_groups(&mut tool, &self.line_surface_groups(*key)?)?;
        }
        let target_groups = targets
            .iter()
            .map(|key| self.line_surface_groups(*key))
            .collect::<Result<Vec<_>, _>>()?;
        let mut changed = Vec::new();
        let mut outputs = Vec::new();
        match mode {
            NewellBooleanMode::Add => {
                let mut result = tool;
                let mut pending: Vec<_> = (0..targets.len()).collect();
                loop {
                    let mut progress = false;
                    let mut remaining = Vec::new();
                    for index in pending {
                        if groups_connect(&result, &target_groups[index])? {
                            merge_groups(&mut result, &target_groups[index])?;
                            changed.push(index);
                            progress = true;
                        } else {
                            remaining.push(index);
                        }
                    }
                    if !progress || remaining.is_empty() {
                        break;
                    }
                    pending = remaining;
                }
                if changed.len() == 1 {
                    let old: f64 = target_groups[changed[0]]
                        .iter()
                        .map(|group| region_area(&group.regions))
                        .sum();
                    let new: f64 = result.iter().map(|group| region_area(&group.regions)).sum();
                    if new <= old + EPS * EPS {
                        return Err(operation_error("NoTargetIntersection"));
                    }
                }
                if !changed.is_empty() {
                    outputs = self.rebuild_line_surfaces(&result)?;
                }
            }
            NewellBooleanMode::Remove => {
                for (index, groups) in target_groups.iter().enumerate() {
                    let old: f64 = groups.iter().map(|group| region_area(&group.regions)).sum();
                    let mut result = groups.clone();
                    for group in &mut result {
                        if let Some(cutter) = tool
                            .iter()
                            .find(|other| same_plane(group.frame, other.frame))
                        {
                            group.regions = region_boolean(
                                &group.regions,
                                &reframe(cutter, group.frame),
                                mode,
                            )?;
                        }
                    }
                    let new: f64 = result.iter().map(|group| region_area(&group.regions)).sum();
                    if old - new > EPS * EPS {
                        changed.push(index);
                        outputs.extend(self.rebuild_line_surfaces(&result)?);
                    }
                }
            }
        }
        if changed.is_empty() {
            return Err(operation_error("NoTargetIntersection"));
        }
        changed.sort_unstable();
        Ok(NewellSurfaceResult {
            outputs,
            changed_targets: changed,
        })
    }
    fn domain_recipe(
        &self,
        key: NewellShapeKey,
    ) -> Result<Option<crate::surface::SurfaceDomainRecipe>, NewellKernelError> {
        use crate::surface::{SurfaceDomainRecipe, SweepDomainPiece, SweepStation};
        if let Some(recipe) = self.surface_domains.get(&key) {
            return Ok(Some(recipe.clone()));
        }
        let Some(support) = self.surface_recipes.get(&key).copied() else {
            return Ok(None);
        };
        let curves = self.selected_boundary_curves(support.profile, support.boundary)?;
        Ok(Some(SurfaceDomainRecipe {
            support,
            pieces: (0..curves.len())
                .map(|curve| SweepDomainPiece {
                    curve,
                    interval: [0., 1.],
                    low: SweepStation {
                        gradient: vector(support.start_gradient),
                        constant: support.start,
                    },
                    high: SweepStation {
                        gradient: vector(support.end_gradient),
                        constant: support.end,
                    },
                })
                .collect(),
        }))
    }
    /// Exact arrangement on generated ruled supports. Every switch is a root of
    /// a rational curve/plane equation; display tessellation is never an input.
    fn combine_partitioned_sweeps(
        &mut self,
        tools: &[NewellShapeKey],
        targets: &[NewellShapeKey],
        mode: NewellBooleanMode,
    ) -> Result<Option<NewellSurfaceResult>, NewellKernelError> {
        use crate::surface::{SweepDomainPiece, SweepStation};
        let mut input = Vec::new();
        for key in tools.iter().chain(targets) {
            let Some(recipe) = self.domain_recipe(*key)? else {
                return Ok(None);
            };
            input.push(recipe);
        }
        let mut templates = Vec::<crate::surface::SurfaceSweepRecipe>::new();
        for recipe in &mut input {
            self.check_cancelled()?;
            let mut canonical = None;
            for reference in &templates {
                if recipe.support.same_support(*reference) {
                    canonical = Some((*reference, 0., false));
                    break;
                }
                if let Some((offset, reverse)) =
                    self.equivalent_sweep_support(*reference, recipe.support)?
                {
                    canonical = Some((*reference, offset, reverse));
                    break;
                }
            }
            if let Some((reference, offset, reverse)) = canonical {
                let count = self
                    .selected_boundary_curves(reference.profile, reference.boundary)?
                    .len();
                for piece in &mut recipe.pieces {
                    for station in [&mut piece.low, &mut piece.high] {
                        station.constant +=
                            offset * (1. + station.gradient.dot(vector(reference.direction)));
                    }
                    if reverse {
                        piece.curve = count - 1 - piece.curve;
                        piece.interval = [1. - piece.interval[1], 1. - piece.interval[0]];
                    }
                }
                recipe.support = reference;
            } else {
                templates.push(recipe.support);
            }
        }
        if templates.is_empty() {
            return Err(operation_error("missing surface tool"));
        }
        let mut bounds = Vec::new();
        for recipe in &input {
            let curves =
                self.selected_boundary_curves(recipe.support.profile, recipe.support.boundary)?;
            let mut lo = [f64::INFINITY; 3];
            let mut hi = [f64::NEG_INFINITY; 3];
            for piece in &recipe.pieces {
                let curve = &curves[piece.curve];
                let (a, b) = curve.domain();
                let curve = crate::termination::trimmed_curve(
                    curve,
                    a + (b - a) * piece.interval[0],
                    a + (b - a) * piece.interval[1],
                )?;
                for p in curve.control_points() {
                    for station in [piece.low, piece.high] {
                        let p = station.point(*p, vector(recipe.support.direction));
                        for (axis, value) in [p.x(), p.y(), p.z()].iter().enumerate() {
                            lo[axis] = lo[axis].min(*value);
                            hi[axis] = hi[axis].max(*value);
                        }
                    }
                }
            }
            bounds.push((lo, hi));
        }
        for (i, a) in input.iter().enumerate() {
            for (j, b) in input.iter().enumerate().skip(i + 1) {
                if a.support.same_support(b.support) {
                    continue;
                }
                if a.support.profile == b.support.profile
                    && a.support.direction == b.support.direction
                    && matches!(
                        (a.support.boundary, b.support.boundary),
                        (
                            crate::NewellBoundary::Wire(_),
                            crate::NewellBoundary::Wire(_)
                        )
                    )
                {
                    continue;
                }
                // Disjoint exact control hull bounds are a culling certificate, not
                // replacement model geometry. Unknown overlaps retain the general
                // planar route and must never be silently called disjoint.
                if (0..3).any(|axis| {
                    bounds[i].1[axis] < bounds[j].0[axis] - EPS
                        || bounds[j].1[axis] < bounds[i].0[axis] - EPS
                }) {
                    continue;
                }
                return Ok(None);
            }
        }
        let mut outputs = Vec::new();
        let mut changed = std::collections::BTreeSet::new();
        for support in templates {
            self.check_cancelled()?;
            let curves = self.selected_boundary_curves(support.profile, support.boundary)?;
            let frame = self
                .profile_faces(support.profile.0)?
                .into_iter()
                .find(|f| f.face == support.profile.1)
                .ok_or_else(|| operation_error("missing sweep support"))?
                .frame;
            let relevant: Vec<_> = input
                .iter()
                .enumerate()
                .filter(|(_, r)| r.support.same_support(support))
                .collect();
            let mut bands = Vec::<DomainBand>::new();
            for (curve_index, curve) in curves.iter().enumerate() {
                self.check_cancelled()?;
                let (lo, hi) = curve.domain();
                let pieces: Vec<_> = relevant
                    .iter()
                    .flat_map(|(owner, r)| {
                        r.pieces
                            .iter()
                            .filter(move |p| p.curve == curve_index)
                            .map(move |p| (*owner, *p))
                    })
                    .collect();
                let stations: Vec<_> = pieces.iter().flat_map(|(_, p)| [p.low, p.high]).collect();
                let mut breaks = vec![0., 1.];
                breaks.extend(pieces.iter().flat_map(|(_, p)| p.interval));
                for (i, a) in stations.iter().enumerate() {
                    self.check_cancelled()?;
                    for b in &stations[i + 1..] {
                        let diff = SweepStation {
                            gradient: a.gradient - b.gradient,
                            constant: a.constant - b.constant,
                        };
                        breaks.extend(
                            self.side_crossings(curve, frame, diff)?
                                .into_iter()
                                .map(|t| (t - lo) / (hi - lo)),
                        );
                    }
                }
                breaks.sort_by(f64::total_cmp);
                breaks.dedup_by(|a, b| (*a - *b).abs() <= 1e-11);
                for interval in breaks.windows(2) {
                    self.check_cancelled()?;
                    if interval[1] - interval[0] <= 1e-12 {
                        continue;
                    }
                    let middle = (interval[0] + interval[1]) * 0.5;
                    let part = crate::termination::trimmed_curve(
                        curve,
                        lo + (hi - lo) * interval[0],
                        lo + (hi - lo) * interval[1],
                    )?;
                    let active: Vec<_> = pieces
                        .iter()
                        .filter(|(_, p)| p.interval[0] <= middle && middle <= p.interval[1])
                        .collect();
                    let point = curve.evaluate(lo + (hi - lo) * middle);
                    let mut endpoints: Vec<_> =
                        active.iter().flat_map(|(_, p)| [p.low, p.high]).collect();
                    endpoints.sort_by(|a, b| a.value(point).total_cmp(&b.value(point)));
                    endpoints.dedup_by(|a, b| (a.value(point) - b.value(point)).abs() <= 1e-10);
                    for pair in endpoints.windows(2) {
                        let diff = SweepStation {
                            gradient: pair[0].gradient - pair[1].gradient,
                            constant: pair[0].constant - pair[1].constant,
                        };
                        if crate::termination::curve_station_max(&part, diff)? > 1e-7 {
                            return Err(operation_error("uncertified sweep domain ordering"));
                        }
                    }
                    let mut cells = Vec::new();
                    for pair in endpoints.windows(2) {
                        let t = (pair[0].value(point) + pair[1].value(point)) * 0.5;
                        if pair[1].value(point) - pair[0].value(point) <= 1e-10 {
                            continue;
                        }
                        let within = |p: &SweepDomainPiece| {
                            p.low.value(point) < t && t < p.high.value(point)
                        };
                        let tool = active.iter().any(|(i, p)| *i < tools.len() && within(p));
                        let owners: std::collections::BTreeSet<_> = active
                            .iter()
                            .filter(|(i, p)| *i >= tools.len() && within(p))
                            .map(|(i, _)| *i - tools.len())
                            .collect();
                        if tool && !owners.is_empty() && mode == NewellBooleanMode::Remove {
                            changed.extend(&owners);
                        }
                        cells.push((pair[0], pair[1], tool, owners));
                    }
                    let owner_sets: Vec<Option<usize>> = if mode == NewellBooleanMode::Add {
                        vec![None]
                    } else {
                        relevant
                            .iter()
                            .filter(|(i, _)| *i >= tools.len())
                            .map(|(i, _)| Some(*i - tools.len()))
                            .collect()
                    };
                    for owner in owner_sets {
                        let mut current: Option<DomainBand> = None;
                        for (low, high, tool, owners) in &cells {
                            let keep = owner.map_or(*tool || !owners.is_empty(), |i| {
                                !tool && owners.contains(&i)
                            });
                            if keep {
                                if let Some(band) = &mut current {
                                    band.piece.high = *high;
                                    band.tool |= *tool;
                                    band.owners.extend(owners);
                                    band.common.retain(|i| owners.contains(i));
                                } else {
                                    current = Some(DomainBand {
                                        piece: SweepDomainPiece {
                                            curve: curve_index,
                                            interval: [interval[0], interval[1]],
                                            low: *low,
                                            high: *high,
                                        },
                                        tool: *tool,
                                        owners: owner.map_or_else(
                                            || owners.clone(),
                                            |i| [i].into_iter().collect(),
                                        ),
                                        common: owners.clone(),
                                    });
                                }
                            } else if let Some(band) = current.take() {
                                bands.push(band);
                            }
                        }
                        if let Some(band) = current {
                            bands.push(band);
                        }
                    }
                }
            }
            if mode == NewellBooleanMode::Remove {
                for owner in changed.iter().copied().collect::<Vec<_>>() {
                    let subset: Vec<_> = bands
                        .iter()
                        .filter(|b| b.owners.contains(&owner))
                        .cloned()
                        .collect();
                    for (key, _) in self.assemble_domain_bands(support, &curves, &subset)? {
                        outputs.push(key);
                    }
                }
            } else {
                for (key, indices) in self.assemble_domain_bands(support, &curves, &bands)? {
                    let mut owners = std::collections::BTreeSet::new();
                    let mut common: Option<std::collections::BTreeSet<usize>> = None;
                    let mut tool = false;
                    for i in indices {
                        tool |= bands[i].tool;
                        owners.extend(&bands[i].owners);
                        if let Some(common) = &mut common {
                            common.retain(|o| bands[i].common.contains(o));
                        } else {
                            common = Some(bands[i].common.clone());
                        }
                    }
                    if !tool {
                        continue;
                    }
                    let unchanged = owners.len() == 1
                        && common
                            .as_ref()
                            .is_some_and(|c| c.contains(owners.first().unwrap_or(&usize::MAX)));
                    if unchanged {
                        continue;
                    }
                    changed.extend(owners);
                    outputs.push(key);
                }
            }
        }
        if changed.is_empty() {
            return Err(operation_error("NoTargetIntersection"));
        }
        Ok(Some(NewellSurfaceResult {
            outputs,
            changed_targets: changed.into_iter().collect(),
        }))
    }
    fn assemble_domain_bands(
        &mut self,
        support: crate::surface::SurfaceSweepRecipe,
        curves: &[brepkit_math::nurbs::curve::NurbsCurve],
        bands: &[DomainBand],
    ) -> Result<Vec<(NewellShapeKey, Vec<usize>)>, NewellKernelError> {
        use crate::surface::SurfaceDomainRecipe;
        use std::collections::{BTreeMap, BTreeSet};
        if bands.is_empty() {
            return Ok(Vec::new());
        }
        let d = vector(support.direction);
        let mut faces = Vec::new();
        let mut artificial = Vec::new();
        for band in bands {
            self.check_cancelled()?;
            let curve = &curves[band.piece.curve];
            let (lo, hi) = curve.domain();
            let [a, b] = band.piece.interval;
            let part =
                crate::termination::trimmed_curve(curve, lo + (hi - lo) * a, lo + (hi - lo) * b)?;
            let (direction, low, high) = if support.reversed {
                (
                    d * (-1.),
                    crate::surface::SweepStation {
                        gradient: band.piece.high.gradient * (-1.),
                        constant: -band.piece.high.constant,
                    },
                    crate::surface::SweepStation {
                        gradient: band.piece.low.gradient * (-1.),
                        constant: -band.piece.low.constant,
                    },
                )
            } else {
                (d, band.piece.low, band.piece.high)
            };
            faces.push(self.sweep_curve_piece(&part, direction, low, high)?);
            for t in [a, b] {
                if t > 1e-11 && t < 1. - 1e-11 {
                    let p = curve.evaluate(lo + (hi - lo) * t);
                    artificial.push([band.piece.low.point(p, d), band.piece.high.point(p, d)]);
                }
            }
        }
        let wire = match support.boundary {
            crate::NewellBoundary::Wire(w) | crate::NewellBoundary::Edge { wire: w, .. } => w,
            crate::NewellBoundary::Face => return Err(operation_error("missing domain wire")),
        };
        let all = self.weld_sweep_pieces(faces, wire, &artificial)?;
        let faces = self.shape_faces(all)?;
        let mut uses = BTreeMap::<_, Vec<usize>>::new();
        let mut vertices = BTreeMap::<_, BTreeSet<usize>>::new();
        for (i, id) in faces.iter().enumerate() {
            let face = self.topology.face(*id).map_err(operation_error)?;
            for occurrence in self
                .topology
                .wire(face.outer_wire())
                .map_err(operation_error)?
                .edges()
            {
                uses.entry(occurrence.edge()).or_default().push(i);
                let edge = self
                    .topology
                    .edge(occurrence.edge())
                    .map_err(operation_error)?;
                for vertex in [edge.start(), edge.end()] {
                    vertices.entry(vertex).or_default().insert(i);
                }
            }
        }
        let mut adjacency = vec![BTreeSet::new(); faces.len()];
        for list in uses.values() {
            let unique: BTreeSet<_> = list.iter().copied().collect();
            if list.len() > 2 {
                return Err(operation_error("nonmanifold domain edge"));
            }
            for a in &unique {
                adjacency[*a].extend(unique.iter().filter(|b| *b != a));
            }
        }
        // A point contact without an incident connecting edge is not a manifold
        // sheet. A collapsed endpoint of one disk (two curved bounds) is valid.
        for (vertex, incident) in vertices {
            let Some(first) = incident.first().copied() else {
                continue;
            };
            let mut reached = BTreeSet::from([first]);
            let mut work = vec![first];
            while let Some(i) = work.pop() {
                for (edge, list) in &uses {
                    let native = self.topology.edge(*edge).map_err(operation_error)?;
                    if native.start() != vertex && native.end() != vertex {
                        continue;
                    }
                    if list.contains(&i) {
                        for j in list {
                            if reached.insert(*j) {
                                work.push(*j);
                            }
                        }
                    }
                }
            }
            if !incident.is_subset(&reached) {
                return Err(operation_error("nonmanifold pinched sheet vertex"));
            }
        }
        let mut pending: BTreeSet<_> = (0..faces.len()).collect();
        let mut result = Vec::new();
        while let Some(first) = pending.pop_first() {
            let mut indices = vec![first];
            let mut cursor = 0;
            while cursor < indices.len() {
                for j in &adjacency[indices[cursor]] {
                    if pending.remove(j) {
                        indices.push(*j);
                    }
                }
                cursor += 1;
            }
            indices.sort_unstable();
            let key = if indices.len() == faces.len() {
                all
            } else {
                self.insert_surface(indices.iter().map(|i| faces[*i]).collect())?
            };
            self.surface_wire_keys.insert(key, wire);
            if let Some(seams) = self.surface_seams.get(&all).cloned() {
                self.surface_seams.insert(key, seams);
            }
            self.surface_domains.insert(
                key,
                SurfaceDomainRecipe {
                    support,
                    pieces: indices.iter().map(|i| bands[*i].piece).collect(),
                },
            );
            result.push((key, indices));
        }
        Ok(result)
    }
    pub(crate) fn rebuild_line_surfaces(
        &mut self,
        groups: &[PlaneRegions],
    ) -> Result<Vec<NewellShapeKey>, NewellKernelError> {
        use std::collections::BTreeSet;
        let mut points: Vec<Point3> = Vec::new();
        let mut regions = Vec::new();
        for group in groups {
            for region in &group.regions {
                let mut loops = Vec::new();
                for boundary in std::iter::once(&region.outer).chain(&region.holes) {
                    let mut ids = Vec::new();
                    for point in boundary {
                        let point = unproject(group.frame, *point);
                        let index = if let Some(index) =
                            points.iter().position(|p| (*p - point).length() <= EPS)
                        {
                            index
                        } else {
                            let index = points.len();
                            points.push(point);
                            index
                        };
                        ids.push(index);
                    }
                    loops.push(ids);
                }
                regions.push((group.frame, loops));
            }
        }
        let vertices: Vec<_> = points
            .iter()
            .map(|p| self.topology.add_vertex(Vertex::new(*p, EPS)))
            .collect();
        let mut edges = BTreeMap::new();
        let mut edge_uses: BTreeMap<_, Vec<(usize, bool)>> = BTreeMap::new();
        let mut faces: Vec<FaceId> = Vec::new();
        let mut vertex_faces: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); points.len()];
        for (frame, loops) in regions {
            let mut wires = Vec::new();
            let face_index = faces.len();
            for boundary in loops {
                let mut occurrences = Vec::new();
                for (a, b) in boundary
                    .iter()
                    .zip(boundary.iter().cycle().skip(1))
                    .take(boundary.len())
                {
                    let direction = points[*b] - points[*a];
                    let length2 = direction.dot(direction);
                    if length2 <= EPS * EPS {
                        continue;
                    }
                    let mut split = vec![(0., *a), (1., *b)];
                    for (index, point) in points.iter().enumerate() {
                        let t = (*point - points[*a]).dot(direction) / length2;
                        if t > 0.
                            && t < 1.
                            && (*point - (points[*a] + direction * t)).length() <= EPS
                        {
                            split.push((t, index));
                        }
                    }
                    split.sort_by(|a, b| a.0.total_cmp(&b.0));
                    split.dedup_by_key(|p| p.1);
                    for span in split.windows(2) {
                        let (a, b) = (span[0].1, span[1].1);
                        if a == b {
                            continue;
                        }
                        let pair = (a.min(b), a.max(b));
                        let forward = a < b;
                        let edge = *edges.entry(pair).or_insert_with(|| {
                            self.topology.add_edge(Edge::new(
                                vertices[pair.0],
                                vertices[pair.1],
                                EdgeCurve::Line,
                            ))
                        });
                        occurrences.push(OrientedEdge::new(edge, forward));
                        edge_uses
                            .entry(pair)
                            .or_default()
                            .push((face_index, forward));
                        vertex_faces[a].insert(face_index);
                        vertex_faces[b].insert(face_index);
                    }
                }
                if occurrences.len() < 3 {
                    return Err(operation_error("collapsed sheet region"));
                }
                wires.push(
                    self.topology
                        .add_wire(Wire::new(occurrences, true).map_err(operation_error)?),
                );
            }
            if wires.is_empty() {
                return Err(operation_error("missing sheet region boundary"));
            }
            faces.push(self.topology.add_face(Face::new(
                wires.remove(0),
                wires,
                FaceSurface::Plane {
                    normal: vector(frame.normal),
                    d: vector(frame.normal).dot(vector(frame.origin)),
                },
            )));
        }
        let mut vertex_neighbors: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
        let mut neighbors = vec![Vec::new(); faces.len()];
        let mut boundary_incidence = vec![0u32; points.len()];
        for ((a, b), uses) in &edge_uses {
            match uses.as_slice() {
                [(face, _)] => {
                    let _ = face;
                    boundary_incidence[*a] += 1;
                    boundary_incidence[*b] += 1;
                }
                [(left, lforward), (right, rforward)] if left != right => {
                    neighbors[*left].push((*right, lforward == rforward));
                    neighbors[*right].push((*left, lforward == rforward));
                    for vertex in [*a, *b] {
                        vertex_neighbors
                            .entry((vertex, *left))
                            .or_default()
                            .push(*right);
                        vertex_neighbors
                            .entry((vertex, *right))
                            .or_default()
                            .push(*left);
                    }
                }
                _ => return Err(operation_error("nonmanifold sheet edge")),
            }
        }
        if boundary_incidence
            .iter()
            .any(|count| !matches!(count, 0 | 2))
        {
            return Err(operation_error("nonmanifold sheet boundary vertex"));
        }
        let mut orientation = vec![None; faces.len()];
        let mut groups = Vec::new();
        for first in 0..faces.len() {
            if orientation[first].is_some() {
                continue;
            }
            let mut group = Vec::new();
            let mut work = vec![first];
            orientation[first] = Some(false);
            while let Some(index) = work.pop() {
                group.push(faces[index]);
                let flip =
                    orientation[index].ok_or_else(|| operation_error("sheet orientation"))?;
                for (next, relative) in &neighbors[index] {
                    let next_flip = flip ^ relative;
                    if let Some(existing) = orientation[*next] {
                        if existing != next_flip {
                            return Err(operation_error("nonorientable sheet union"));
                        }
                    } else {
                        orientation[*next] = Some(next_flip);
                        work.push(*next);
                    }
                }
            }
            groups.push(group);
        }
        // Every vertex link must be connected; merely sharing one vertex cannot join sheets.
        for (vertex, incident) in vertex_faces.iter().enumerate() {
            let Some(first) = incident.first().copied() else {
                continue;
            };
            let mut seen = BTreeSet::from([first]);
            let mut work = vec![first];
            while let Some(index) = work.pop() {
                for next in vertex_neighbors.get(&(vertex, index)).into_iter().flatten() {
                    if seen.insert(*next) {
                        work.push(*next);
                    }
                }
            }
            if seen.len() != incident.len() {
                return Err(operation_error("nonmanifold sheet vertex link"));
            }
        }
        for (index, flip) in orientation.into_iter().enumerate() {
            if flip == Some(true) {
                self.topology
                    .face_mut(faces[index])
                    .map_err(operation_error)?
                    .set_reversed(true);
            }
        }
        let keys = groups
            .into_iter()
            .map(|faces| self.insert_surface(faces))
            .collect::<Result<Vec<_>, _>>()?;
        self.order_components(keys)
    }
}

pub(crate) fn line_region(
    topology: &brepkit_topology::Topology,
    face: FaceId,
    frame: NewellProfileFrame,
) -> Result<Region, NewellKernelError> {
    let face = topology.face(face).map_err(operation_error)?;
    let mut loops = Vec::new();
    for wire in std::iter::once(face.outer_wire()).chain(face.inner_wires().iter().copied()) {
        let mut points = Vec::new();
        for occurrence in topology.wire(wire).map_err(operation_error)?.edges() {
            let edge = topology.edge(occurrence.edge()).map_err(operation_error)?;
            if !matches!(edge.curve(), EdgeCurve::Line) {
                return Err(operation_error(
                    "nonlinear target trim requires exact containment proof",
                ));
            }
            let vertex = if occurrence.is_forward() {
                edge.start()
            } else {
                edge.end()
            };
            points.push(project(
                frame,
                topology.vertex(vertex).map_err(operation_error)?.point(),
            ));
        }
        loops.push(points);
    }
    if loops.is_empty() {
        return Err(operation_error("missing planar boundary"));
    }
    Ok(Region {
        outer: loops.remove(0),
        holes: loops,
    })
}
pub(crate) fn contains_point(region: &Region, point: Point2) -> bool {
    region.contains(point)
        || std::iter::once(&region.outer)
            .chain(&region.holes)
            .any(|boundary| {
                boundary
                    .iter()
                    .zip(boundary.iter().cycle().skip(1))
                    .take(boundary.len())
                    .any(|(a, b)| {
                        let d = *b - *a;
                        let length2 = d.dot(d);
                        if length2 <= EPS * EPS {
                            return (*a - point).length() <= EPS;
                        }
                        let t = (point - *a).dot(d) / length2;
                        (0_f64..=1.).contains(&t) && (point - (*a + d * t)).length() <= EPS
                    })
            })
}
pub(crate) fn contains_segment(region: &Region, a: Point2, b: Point2) -> bool {
    if !contains_point(region, a) || !contains_point(region, b) {
        return false;
    }
    let d = b - a;
    if d.length() <= EPS {
        return true;
    }
    let mut intervals = line_intervals(std::slice::from_ref(region), a, d);
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut covered = 0_f64;
    let tolerance = EPS / d.length();
    for (low, high, _) in intervals {
        if low > covered + tolerance {
            return false;
        }
        covered = covered.max(high);
        if covered >= 1. - tolerance {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point2> {
        vec![
            Point2::new(x0, y0),
            Point2::new(x1, y0),
            Point2::new(x1, y1),
            Point2::new(x0, y1),
        ]
    }
    #[test]
    fn line_regions_keep_input_holes_and_recombine_internal_triangles()
    -> Result<(), NewellKernelError> {
        let a = Region {
            outer: rectangle(0., 0., 10., 10.),
            holes: vec![rectangle(3., 3., 7., 7.)],
        };
        let b = Region {
            outer: rectangle(5., 0., 15., 10.),
            holes: vec![rectangle(11., 2., 13., 4.)],
        };
        let union = region_boolean(
            std::slice::from_ref(&a),
            std::slice::from_ref(&b),
            crate::NewellBooleanMode::Add,
        )?;
        assert!(
            (region_area(&union) - 138.).abs() < 1e-5,
            "{}",
            region_area(&union)
        );
        assert_eq!(union.len(), 1);
        assert_eq!(union[0].holes.len(), 2);
        let cut = region_boolean(
            std::slice::from_ref(&a),
            std::slice::from_ref(&b),
            crate::NewellBooleanMode::Remove,
        )?;
        assert!(
            (region_area(&cut) - 42.).abs() < 1e-5,
            "{}",
            region_area(&cut)
        );
        assert_eq!(cut.len(), 1);
        assert!(cut[0].holes.is_empty());
        let triangles = triangulate(&a)?;
        assert!((triangles_area(&triangles) - 84.).abs() < 1e-8);
        Ok(())
    }
}
