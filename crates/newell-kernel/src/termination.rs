// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Exact finite-face termination. Conservative bounds size query carriers only.
use crate::{NewellGeometry, NewellKernelError, NewellShapeKey, operation_error};
use brepkit_math::vec::Vec3;
use brepkit_topology::{FaceId, SolidId};

fn invalid(parameter: &'static str) -> NewellKernelError {
    NewellKernelError::InvalidParameter { parameter }
}
impl NewellGeometry {
    fn planar_face(
        &self,
        shape: NewellShapeKey,
        ordinal: u32,
    ) -> Result<FaceId, NewellKernelError> {
        let faces = self.shape_faces(shape)?;
        let face = *faces
            .get(usize::try_from(ordinal).map_err(operation_error)?)
            .ok_or_else(|| invalid("target face"))?;
        crate::profile::supporting_frame(&self.topology, face)?;
        Ok(face)
    }
    fn face_projection_bounds(
        &self,
        face: FaceId,
        axis: Vec3,
    ) -> Result<(f64, f64), NewellKernelError> {
        Ok((
            -self.face_set_projection_max(&[face], axis * (-1.))?,
            self.face_set_projection_max(&[face], axis)?,
        ))
    }
    fn shape_projection_max(&self, shape: SolidId, axis: Vec3) -> Result<f64, NewellKernelError> {
        let faces = brepkit_topology::explorer::solid_faces(&self.topology, shape)
            .map_err(operation_error)?;
        self.face_set_projection_max(&faces, axis)
    }
    fn face_set_projection_max(
        &self,
        faces: &[FaceId],
        axis: Vec3,
    ) -> Result<f64, NewellKernelError> {
        use brepkit_topology::edge::EdgeCurve;
        let mut maximum = f64::NEG_INFINITY;
        let project =
            |point: brepkit_math::vec::Point3| axis.dot(Vec3::new(point.x(), point.y(), point.z()));
        for face in faces {
            let face = self.topology.face(*face).map_err(operation_error)?;
            for wire in std::iter::once(face.outer_wire()).chain(face.inner_wires().iter().copied())
            {
                for occurrence in self.topology.wire(wire).map_err(operation_error)?.edges() {
                    let edge = self
                        .topology
                        .edge(occurrence.edge())
                        .map_err(operation_error)?;
                    let start = self
                        .topology
                        .vertex(edge.start())
                        .map_err(operation_error)?
                        .point();
                    let end = self
                        .topology
                        .vertex(edge.end())
                        .map_err(operation_error)?
                        .point();
                    maximum = maximum.max(project(start)).max(project(end));
                    let (a, b) = edge.curve().domain_with_endpoints(start, end);
                    let conic_max =
                        |center: brepkit_math::vec::Point3, u: Vec3, v: Vec3, ra: f64, rb: f64| {
                            let angle = (rb * axis.dot(v)).atan2(ra * axis.dot(u));
                            let lo = a.min(b);
                            let hi = a.max(b);
                            let station = angle
                                + ((lo - angle) / std::f64::consts::TAU).ceil()
                                    * std::f64::consts::TAU;
                            (station <= hi + 1e-12).then(|| {
                                project(center) + (ra * axis.dot(u)).hypot(rb * axis.dot(v))
                            })
                        };
                    match edge.curve() {
                        EdgeCurve::Line => {}
                        EdgeCurve::Circle(c) => {
                            if let Some(value) = conic_max(
                                c.center(),
                                c.u_axis(),
                                c.v_axis(),
                                c.radius(),
                                c.radius(),
                            ) {
                                maximum = maximum.max(value);
                            }
                        }
                        EdgeCurve::Ellipse(c) => {
                            if let Some(value) = conic_max(
                                c.center(),
                                c.u_axis(),
                                c.v_axis(),
                                c.semi_major(),
                                c.semi_minor(),
                            ) {
                                maximum = maximum.max(value);
                            }
                        }
                        EdgeCurve::NurbsCurve(curve) => {
                            let mut span = curve.clone();
                            let (lo, hi) = (a.min(b), a.max(b));
                            let (d0, d1) = span.domain();
                            if lo > d0 {
                                span = brepkit_math::nurbs::knot_ops::curve_split(&span, lo)
                                    .map_err(operation_error)?
                                    .1;
                            }
                            if hi < d1 {
                                span = brepkit_math::nurbs::knot_ops::curve_split(&span, hi)
                                    .map_err(operation_error)?
                                    .0;
                            }
                            let mut work = vec![(span, 0u32)];
                            let mut visits = 0usize;
                            let mut best = f64::NEG_INFINITY;
                            let mut certified = f64::NEG_INFINITY;
                            while let Some((curve, depth)) = work.pop() {
                                visits += 1;
                                if visits > 65536 || depth > 64 {
                                    return Err(invalid("curve extremum complexity"));
                                }
                                if curve
                                    .weights()
                                    .iter()
                                    .any(|weight| !weight.is_finite() || *weight <= 0.)
                                {
                                    return Err(invalid("rational curve bound"));
                                }
                                let upper = curve
                                    .control_points()
                                    .iter()
                                    .map(|p| project(*p))
                                    .fold(f64::NEG_INFINITY, f64::max);
                                let (lo, hi) = curve.domain();
                                let mid = lo + (hi - lo) * 0.5;
                                let witness = [lo, mid, hi]
                                    .map(|t| project(curve.evaluate(t)))
                                    .into_iter()
                                    .fold(f64::NEG_INFINITY, f64::max);
                                best = best.max(witness);
                                let tolerance = 1e-9_f64.max(1e-11 * upper.abs());
                                if upper <= best + tolerance {
                                    certified = certified.max(upper);
                                    continue;
                                }
                                let (left, right) =
                                    brepkit_math::nurbs::knot_ops::curve_split(&curve, mid)
                                        .map_err(operation_error)?;
                                work.push((left, depth + 1));
                                work.push((right, depth + 1));
                            }
                            maximum = maximum.max(best).max(certified);
                        }
                    }
                }
            }
        }
        if !maximum.is_finite() {
            return Err(invalid("intersection station"));
        }
        Ok(maximum)
    }
    /// Terminates beyond the furthest actual forward intersection with eligible bodies.
    /// Visibility and explicit merge scope never enter this query.
    ///
    /// # Errors
    /// Rejects no-hit, invalid directions, overflow and failed exact intersections.
    pub fn through_all_station(
        &mut self,
        profile: (NewellShapeKey, u32),
        direction: [f64; 3],
        start: f64,
        targets: &[NewellShapeKey],
    ) -> Result<f64, NewellKernelError> {
        use brepkit_operations::boolean::{BooleanOp, boolean, mesh_fallback_count};
        let d = Vec3::new(direction[0], direction[1], direction[2])
            .normalize()
            .map_err(operation_error)?;
        let frame = crate::profile::supporting_frame(
            &self.topology,
            self.planar_face(profile.0, profile.1)?,
        )?;
        if let Some(line) = self.line_profile(profile, crate::NewellBoundary::Face)? {
            if xyz_vector(line.frame.normal).dot(d).abs() <= 1e-12 {
                return Err(invalid("tangential Along"));
            }
            if let Some(end) = self.line_through_all(&line, d, start, targets, true)? {
                return Ok(end);
            }
        }
        match self.curved_through_all_station(profile,crate::NewellBoundary::Face,d,start,targets,true) {
            Ok(end)=>return Ok(end),
            Err(NewellKernelError::InvalidParameter{parameter:"ThroughAll curved target needs a certified intersection route"})=>{},
            Err(error)=>return Err(error),
        }
        let origin = Vec3::new(frame.origin[0], frame.origin[1], frame.origin[2]);
        let n = Vec3::new(frame.normal[0], frame.normal[1], frame.normal[2]);
        let denominator = n.dot(d);
        if denominator.abs() <= 1e-12 {
            return Err(invalid("tangential Along"));
        }
        let axis = n * (1. / denominator);
        let reference = axis.dot(origin);
        let mut bound = start;
        for target in targets {
            let bounds = brepkit_operations::measure::solid_bounding_box(
                &self.topology,
                self.solid(*target)?,
            )
            .map_err(operation_error)?;
            for x in [bounds.min.x(), bounds.max.x()] {
                for y in [bounds.min.y(), bounds.max.y()] {
                    for z in [bounds.min.z(), bounds.max.z()] {
                        bound = bound.max(axis.dot(Vec3::new(x, y, z)) - reference);
                    }
                }
            }
        }
        let eps = 1e-7_f64.max(1e-10 * (reference.abs().max(bound.abs()).max(start.abs())));
        if !start.is_finite() || bound <= start + eps {
            return Err(invalid("NoThroughAllHit"));
        }
        let bound = bound + (bound - start).max(1.);
        let prism = self.extrude_face(
            profile.0,
            profile.1,
            Some([d.x(), d.y(), d.z()]),
            start,
            bound,
        )?;
        let prism = self.solid(prism)?;
        let mut farthest = f64::NEG_INFINITY;
        for target_key in targets {
            let target = self.solid(*target_key)?;
            let faces = self.shape_faces(*target_key)?;
            // A parallel far face that covers the complete section proves an actual
            // hit at the body's global farthest station. This exact region proof
            // avoids the pinned spline/box common path's mesh fallback.
            if faces.iter().all(|face| {
                self.topology.face(*face).is_ok_and(|face| {
                    matches!(
                        face.surface(),
                        brepkit_topology::face::FaceSurface::Plane { .. }
                    )
                })
            }) {
                let far = self.shape_projection_max(target, axis)? - reference;
                if far <= start + eps {
                    continue;
                }
                let mut covered = false;
                for (ordinal, face) in faces.iter().enumerate() {
                    let frame = crate::profile::supporting_frame(&self.topology, *face)?;
                    let normal = Vec3::new(frame.normal[0], frame.normal[1], frame.normal[2]);
                    if normal.cross(n).length() > 1e-10 {
                        continue;
                    }
                    match self.curved_through_all_station(profile,crate::NewellBoundary::Face,d,start,targets,true) {
            Ok(end)=>return Ok(end),
            Err(NewellKernelError::InvalidParameter{parameter:"ThroughAll curved target needs a certified intersection route"})=>{},
            Err(error)=>return Err(error),
        }
        let origin = Vec3::new(frame.origin[0], frame.origin[1], frame.origin[2]);
                    if (axis.dot(origin) - reference - far).abs() > eps {
                        continue;
                    }
                    match self.starting_station(
                        profile,
                        [d.x(), d.y(), d.z()],
                        (
                            *target_key,
                            u32::try_from(ordinal).map_err(operation_error)?,
                        ),
                    ) {
                        Ok(station) => {
                            farthest = farthest.max(station);
                            covered = true;
                            break;
                        }
                        Err(NewellKernelError::InvalidParameter {
                            parameter: "starting target trim coverage",
                        }) => {}
                        Err(error) => return Err(error),
                    }
                }
                if covered {
                    continue;
                }
            }
            let before = mesh_fallback_count();
            let common = boolean(&mut self.topology, BooleanOp::Intersect, prism, target);
            if before != mesh_fallback_count() {
                return Err(invalid(
                    "ThroughAll exact intersection attempted mesh fallback",
                ));
            }
            let common = match common {
                Err(brepkit_operations::OperationsError::EmptyResult { .. }) => continue,
                result => result.map_err(operation_error)?,
            };
            if self.topology.is_empty_solid(common) {
                continue;
            }
            self.validate_native(common)?;
            let hit = self.shape_projection_max(common, axis)? - reference;
            if hit > start + eps {
                farthest = farthest.max(hit);
            }
        }
        if !farthest.is_finite() {
            return Err(invalid("NoThroughAllHit"));
        }
        let endpoint = farthest + (10. * eps).max(1e-6);
        if !endpoint.is_finite() {
            return Err(invalid("ThroughAll overflow"));
        }
        Ok(endpoint)
    }
    /// Resolves a parallel finite starting target and proves complete section coverage.
    ///
    /// # Errors
    /// Rejects tilted starts, tangent directions, missed trims and failed exact operations.
    pub fn starting_station(
        &mut self,
        profile: (NewellShapeKey, u32),
        direction: [f64; 3],
        target: (NewellShapeKey, u32),
    ) -> Result<f64, NewellKernelError> {
        let pface = self.planar_face(profile.0, profile.1)?;
        let tface = self.planar_face(target.0, target.1)?;
        let p = crate::profile::supporting_frame(&self.topology, pface)?;
        let t = crate::profile::supporting_frame(&self.topology, tface)?;
        let vector = |a: [f64; 3]| Vec3::new(a[0], a[1], a[2]);
        let n = vector(p.normal);
        let tn = vector(t.normal);
        if n.cross(tn).length() > 1e-10 {
            return Err(invalid("tilted starting target"));
        }
        let d = vector(direction).normalize().map_err(operation_error)?;
        let denominator = n.dot(d);
        if denominator.abs() <= 1e-12 {
            return Err(invalid("tangential Along"));
        }
        let station = n.dot(vector(t.origin) - vector(p.origin)) / denominator;
        if !station.is_finite() {
            return Err(invalid("starting station"));
        }
        let d = [d.x(), d.y(), d.z()];
        let probe =
            self.extrude_face(profile.0, profile.1, Some(d), station - 0.5, station + 0.5)?;
        let carrier = self.extrude_face(target.0, target.1, Some(d), -1., 1.)?;
        if self
            .boolean(probe, carrier, crate::NewellBooleanMode::Remove)?
            .is_some()
        {
            return Err(invalid("starting target trim coverage"));
        }
        Ok(station)
    }
    /// Reflects sweep stations about a displaced profile section, preserving oblique Along.
    ///
    /// # Errors
    /// Rejects invalid frames, nonfinite matrices and native copy/transform/validation failures.
    pub fn mirror_sweep(
        &mut self,
        shape: NewellShapeKey,
        profile: (NewellShapeKey, u32),
        direction: [f64; 3],
        start: f64,
    ) -> Result<NewellShapeKey, NewellKernelError> {
        let face = self.planar_face(profile.0, profile.1)?;
        let frame = crate::profile::supporting_frame(&self.topology, face)?;
        let vector = |a: [f64; 3]| Vec3::new(a[0], a[1], a[2]);
        let d = vector(direction).normalize().map_err(operation_error)?;
        let n = vector(frame.normal);
        let denominator = n.dot(d);
        if !start.is_finite() || denominator.abs() <= 1e-12 {
            return Err(invalid("symmetric frame"));
        }
        let origin = vector(frame.origin) + d * start;
        let translation = d * (2. * n.dot(origin) / denominator);
        let dv = [d.x(), d.y(), d.z()];
        let nv = frame.normal;
        let mut matrix = brepkit_math::mat::Mat4::identity();
        for (row, d) in dv.iter().enumerate() {
            for (column, n) in nv.iter().enumerate() {
                matrix.0[row][column] -= 2. * d * n / denominator;
            }
        }
        matrix.0[0][3] = translation.x();
        matrix.0[1][3] = translation.y();
        matrix.0[2][3] = translation.z();
        let source = self.solid(shape)?;
        let result = brepkit_operations::copy::copy_solid(&mut self.topology, source)
            .map_err(operation_error)?;
        brepkit_operations::transform::transform_solid(&mut self.topology, result, &matrix)
            .map_err(operation_error)?;
        self.validate_native(result)?;
        self.insert(result)
    }
    /// Sweeps to a finite planar target, preserving its actual trim and holes.
    /// A target carrier is an auxiliary exact B-Rep: its bound never becomes an endpoint.
    ///
    /// # Errors
    /// Rejects parallel rays, a target behind any part of the start section, incomplete
    /// trim coverage, nonpositive clearance-adjusted reach and failed exact booleans.
    pub fn extrude_to_face(
        &mut self,
        profile: (NewellShapeKey, u32),
        direction: [f64; 3],
        start: f64,
        target: (NewellShapeKey, u32),
        clearance: f64,
    ) -> Result<NewellShapeKey, NewellKernelError> {
        if !start.is_finite()
            || !clearance.is_finite()
            || clearance < 0.
            || direction.iter().any(|value| !value.is_finite())
        {
            return Err(invalid("Up to extent"));
        }
        let d = Vec3::new(direction[0], direction[1], direction[2])
            .normalize()
            .map_err(operation_error)?;
        let pface = self.planar_face(profile.0, profile.1)?;
        let tface = self.planar_face(target.0, target.1)?;
        let frame = crate::profile::supporting_frame(&self.topology, tface)?;
        let n = Vec3::new(frame.normal[0], frame.normal[1], frame.normal[2]);
        let origin = Vec3::new(frame.origin[0], frame.origin[1], frame.origin[2]);
        let denominator = n.dot(d);
        if denominator.abs() <= 1e-12 {
            return Err(invalid("target parallel to sweep"));
        }
        let (projection_low, projection_high) = self.face_projection_bounds(pface, n)?;
        let stations = [projection_low, projection_high]
            .map(|value| (n.dot(origin) - value) / denominator - clearance);
        let low = stations[0].min(stations[1]);
        let high = stations[0].max(stations[1]);
        if !low.is_finite() || !high.is_finite() || low - start <= 1e-7 {
            return Err(invalid(
                "target does not lie beyond the entire start section",
            ));
        }
        let margin = (high - start).max(1.);
        let end = high + margin;
        let carrier_length = high - start + 2. * margin;
        if !end.is_finite() || !carrier_length.is_finite() {
            return Err(invalid("Up to bound"));
        }
        let direction = [d.x(), d.y(), d.z()];
        let reverse = direction.map(|value| -value);
        // Displacing the target by -d*clearance gives the approved ray clearance,
        // even for a tilted target. All target boundary curves are copied exactly.
        let carrier = self.extrude_face(
            target.0,
            target.1,
            Some(reverse),
            clearance,
            clearance + carrier_length,
        )?;
        let probe_depth = ((low - start) * 0.5).min(margin);
        let probe = self.extrude_face(
            profile.0,
            profile.1,
            Some(direction),
            start,
            start + probe_depth,
        )?;
        // Whole-region inclusion, rather than vertex/centroid testing: a missed hole or
        // concave gap leaves a nonempty exact difference and rejects the entire recipe.
        if self
            .boolean(probe, carrier, crate::NewellBooleanMode::Remove)?
            .is_some()
        {
            return Err(invalid(
                "target trim does not cover the entire swept section",
            ));
        }
        let prism = self.extrude_face(profile.0, profile.1, Some(direction), start, end)?;
        // Keep the exact prism's ruled walls and move only its far cap along the
        // sweep rays. This is an invertible affine map on that cap, preserving
        // polynomial/rational boundaries. The walls retain their native supports;
        // their moved boundary occurrences trim them to the tilted endpoint.
        // The pinned generic NURBS boolean can omit this cap despite validation.
        let result = self.solid(prism)?;
        let pframe = crate::profile::supporting_frame(&self.topology, pface)?;
        let pn = Vec3::new(pframe.normal[0], pframe.normal[1], pframe.normal[2]);
        let po = Vec3::new(pframe.origin[0], pframe.origin[1], pframe.origin[2]);
        let pd = pn.dot(d);
        let cap_height = pn.dot(po) + end * pd;
        let faces = brepkit_topology::explorer::solid_faces(&self.topology, result)
            .map_err(operation_error)?;
        let mut caps = Vec::new();
        for face in faces {
            if let Ok(frame) = crate::profile::supporting_frame(&self.topology, face) {
                let normal = Vec3::new(frame.normal[0], frame.normal[1], frame.normal[2]);
                let origin = Vec3::new(frame.origin[0], frame.origin[1], frame.origin[2]);
                if normal.cross(pn).length() < 1e-10
                    && normal.dot(d) > 1e-12
                    && (pn.dot(origin) - cap_height).abs() <= 1e-7_f64.max(1e-10 * cap_height.abs())
                {
                    caps.push(face);
                }
            }
        }
        if caps.len() != 1 {
            return Err(invalid("ambiguous far cap"));
        }
        let mut matrix = brepkit_math::mat::Mat4::identity();
        let dv = direction;
        let nv = frame.normal;
        let pv = pframe.normal;
        for (row, value) in dv.iter().enumerate() {
            for (column, (p, n)) in pv.iter().zip(nv).enumerate() {
                matrix.0[row][column] += value * (p / pd - n / denominator);
            }
        }
        let translation =
            d * ((n.dot(origin) - clearance * denominator) / denominator - cap_height / pd);
        matrix.0[0][3] = translation.x();
        matrix.0[1][3] = translation.y();
        matrix.0[2][3] = translation.z();
        brepkit_operations::transform::transform_face(&mut self.topology, caps[0], &matrix)
            .map_err(operation_error)?;
        self.validate_native(result)?;
        Ok(prism)
    }
}

/// One actual bounded planar face, with clearance measured along the side direction.
#[derive(Debug, Clone, Copy)]
pub struct NewellFaceTarget {
    pub shape: NewellShapeKey,
    pub face: u32,
    pub clearance: f64,
}
#[derive(Debug, Clone, Copy)]
pub enum NewellSurfaceStart {
    Distance(f64),
    UpTo(NewellFaceTarget),
}
#[derive(Debug, Clone)]
pub enum NewellSurfaceExtent {
    Blind(f64),
    UpTo(NewellFaceTarget),
    ThroughAll(Vec<NewellShapeKey>),
}
#[derive(Debug, Clone)]
pub enum NewellSurfaceSides {
    One,
    Symmetric,
    Second {
        extent: NewellSurfaceExtent,
        reverse: bool,
    },
}
/// Owned resolved quantities and references; no UI or model enum enters the ACL.
#[derive(Debug, Clone)]
pub struct NewellSurfaceExtrude {
    pub profile: (NewellShapeKey, u32),
    pub boundary: crate::NewellBoundary,
    pub direction: [f64; 3],
    pub start: NewellSurfaceStart,
    pub extent: NewellSurfaceExtent,
    pub sides: NewellSurfaceSides,
}
struct LineProfile {
    face: FaceId,
    frame: crate::NewellProfileFrame,
    edges: Vec<[brepkit_math::vec::Point3; 2]>,
}
fn xyz_point(p: [f64; 3]) -> brepkit_math::vec::Point3 {
    brepkit_math::vec::Point3::new(p[0], p[1], p[2])
}
fn xyz_vector(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0], p[1], p[2])
}
fn side_frame(
    edge: [brepkit_math::vec::Point3; 2],
    d: Vec3,
) -> Result<crate::NewellProfileFrame, NewellKernelError> {
    let u = (edge[1] - edge[0]).normalize().map_err(operation_error)?;
    let n = u.cross(d).normalize().map_err(operation_error)?;
    let v = n.cross(u);
    let origin = n * n.dot(Vec3::new(edge[0].x(), edge[0].y(), edge[0].z()));
    Ok(crate::NewellProfileFrame {
        origin: [origin.x(), origin.y(), origin.z()],
        normal: [n.x(), n.y(), n.z()],
        u: [u.x(), u.y(), u.z()],
        v: [v.x(), v.y(), v.z()],
    })
}
fn side_coordinates(
    edge: [brepkit_math::vec::Point3; 2],
    d: Vec3,
    p: brepkit_math::vec::Point3,
) -> Result<(f64, f64), NewellKernelError> {
    let e = edge[1] - edge[0];
    let r = p - edge[0];
    let ed = e.dot(d);
    let det = e.dot(e) - ed * ed;
    if !det.is_finite() || det <= 1e-24 {
        return Err(invalid("degenerate side frame"));
    }
    let u = (r.dot(e) - ed * r.dot(d)) / det;
    Ok((u, r.dot(d) - ed * u))
}
fn clip_parameter(
    low: &mut f64,
    high: &mut f64,
    value: f64,
    slope: f64,
    minimum: f64,
    maximum: f64,
) -> bool {
    if slope.abs() <= 1e-14 {
        return value >= minimum && value <= maximum;
    }
    let a = (minimum - value) / slope;
    let b = (maximum - value) / slope;
    *low = low.max(a.min(b));
    *high = high.min(a.max(b));
    *high >= *low
}
fn coincident_parameterized_curves(a:&brepkit_math::nurbs::curve::NurbsCurve,b:&brepkit_math::nurbs::curve::NurbsCurve,reverse:bool)->Result<bool,NewellKernelError> {
    use brepkit_math::nurbs::{curve::NurbsCurve,decompose::curve_degree_elevate,knot_ops::curve_knot_insert};
    let normalize=|curve:&NurbsCurve,reverse:bool|->Result<NurbsCurve,NewellKernelError>{
        let (lo,hi)=curve.domain();let mut points=curve.control_points().to_vec();let mut weights=curve.weights().to_vec();let mut knots:Vec<_>=curve.knots().iter().map(|k|(k-lo)/(hi-lo)).collect();
        if reverse {points.reverse();weights.reverse();knots=knots.iter().rev().map(|k|1.-k).collect();}
        NurbsCurve::new(curve.degree(),knots,points,weights).map_err(operation_error)
    };
    let degree=a.degree().max(b.degree());
    let mut a=normalize(&curve_degree_elevate(a,degree-a.degree()).map_err(operation_error)?,false)?;
    let mut b=normalize(&curve_degree_elevate(b,degree-b.degree()).map_err(operation_error)?,reverse)?;
    let mut knots:Vec<_>=a.knots().iter().chain(b.knots()).copied().filter(|k|*k>0. && *k<1.).collect();knots.sort_by(f64::total_cmp);knots.dedup();
    for knot in knots {
        let ac=a.knots().iter().filter(|k|**k==knot).count();let bc=b.knots().iter().filter(|k|**k==knot).count();
        if ac<bc {a=curve_knot_insert(&a,knot,bc-ac).map_err(operation_error)?;}
        if bc<ac {b=curve_knot_insert(&b,knot,ac-bc).map_err(operation_error)?;}
    }
    if a.knots()!=b.knots() || a.control_points().len()!=b.control_points().len(){return Ok(false);}
    let scale=a.weights()[0]/b.weights()[0];
    Ok(a.control_points().iter().zip(b.control_points()).zip(a.weights().iter().zip(b.weights())).all(|((a,b),(wa,wb))|(*a-*b).length()<=1e-10 && (*wa-*wb*scale).abs()<=1e-12*wa.abs().max(1.)))
}

#[derive(Default)]
struct CurveIntersections { hits: Vec<[f64;2]>, overlaps: Vec<[f64;4]> }
/// Queries exact rational curves by their certified positive-weight control
/// hulls. This replaces pinned 4.2.2's missed circle/line Bezier-clip hits.
fn exact_curve_intersections(a:&brepkit_math::nurbs::curve::NurbsCurve,b:&brepkit_math::nurbs::curve::NurbsCurve)->Result<CurveIntersections,NewellKernelError> {
    use brepkit_math::nurbs::{curve::NurbsCurve,decompose::{curve_to_bezier_segments,curve_degree_elevate},knot_ops::curve_split};
    fn bounds(curve:&NurbsCurve)->([f64;3],[f64;3]) {
        let mut low=[f64::INFINITY;3];let mut high=[f64::NEG_INFINITY;3];
        for p in curve.control_points(){for (i,v) in [p.x(),p.y(),p.z()].into_iter().enumerate(){low[i]=low[i].min(v);high[i]=high[i].max(v);}}(low,high)
    }
    fn parameter(curve:&NurbsCurve,fraction:f64)->f64 {
        let (lo,hi)=curve.domain();let weights=curve.weights();let t=fraction*weights[0]/(weights[1]*(1.-fraction)+fraction*weights[0]);lo+(hi-lo)*t
    }
    fn same(a:&NurbsCurve,b:&NurbsCurve,reverse:bool)->Result<bool,NewellKernelError> {
        let degree=a.degree().max(b.degree());let aa=curve_degree_elevate(a,degree-a.degree()).map_err(operation_error)?;let bb=curve_degree_elevate(b,degree-b.degree()).map_err(operation_error)?;
        if aa.control_points().len()!=bb.control_points().len(){return Ok(false);}
        let last=bb.control_points().len()-1;let scale=aa.weights()[0]/bb.weights()[if reverse {last} else {0}];
        Ok(aa.control_points().iter().zip(aa.weights()).enumerate().all(|(i,(p,w))| {let j=if reverse {last-i} else {i};(*p-bb.control_points()[j]).length()<=1e-10 && (*w-bb.weights()[j]*scale).abs()<=1e-12*w.abs().max(1.)}))
    }
    let mut result=CurveIntersections::default();
    let (a0,a1)=a.domain();let (b0,b1)=b.domain();
    if coincident_parameterized_curves(a,b,false)? {result.overlaps.push([a0,a1,b0,b1]);return Ok(result);}
    if coincident_parameterized_curves(a,b,true)? {result.overlaps.push([a0,a1,b1,b0]);return Ok(result);}
    let mut work=Vec::new();
    for aa in curve_to_bezier_segments(a).map_err(operation_error)? {for bb in curve_to_bezier_segments(b).map_err(operation_error)? {work.push((aa.clone(),bb,0u32));}}
    let mut visits=0usize;
    while let Some((a,b,depth))=work.pop() {
        visits+=1;if visits>262144 || depth>96 {return Err(invalid("exact curve intersection complexity"));}
        if a.weights().iter().chain(b.weights()).any(|w|!w.is_finite()||*w<=0.) {return Err(invalid("rational curve bound"));}
        let (al,ah)=bounds(&a);let (bl,bh)=bounds(&b);
        if (0..3).any(|i|al[i]>bh[i]+1e-10 || bl[i]>ah[i]+1e-10) {continue;}
        if a.degree()==1 && b.degree()==1 {
            let p=a.control_points()[0];let q=b.control_points()[0];let u=a.control_points()[1]-p;let v=b.control_points()[1]-q;let w=q-p;let normal=u.cross(v);let square=normal.dot(normal);
            if square>1e-24*u.dot(u)*v.dot(v) {
                let t=w.cross(v).dot(normal)/square;let s=w.cross(u).dot(normal)/square;
                if (-1e-10..=1.+1e-10).contains(&t) && (-1e-10..=1.+1e-10).contains(&s) && ((p+u*t)-(q+v*s)).length()<=1e-8 {result.hits.push([parameter(&a,t.clamp(0.,1.)),parameter(&b,s.clamp(0.,1.))]);}
            } else if u.dot(u)>0. && v.dot(v)>0. && w.cross(u).length()<=1e-9*u.length() {
                let x=w.dot(u)/u.dot(u);let y=(w+v).dot(u)/u.dot(u);let low=x.min(y).max(0.);let high=x.max(y).min(1.);
                if high>=low {let point_low=p+u*low;let point_high=p+u*high;result.overlaps.push([parameter(&a,low),parameter(&a,high),parameter(&b,((point_low-q).dot(v)/v.dot(v)).clamp(0.,1.)),parameter(&b,((point_high-q).dot(v)/v.dot(v)).clamp(0.,1.))]);}
            }
            continue;
        }
        let (a0,a1)=a.domain();let (b0,b1)=b.domain();
        if same(&a,&b,false)? {result.overlaps.push([a0,a1,b0,b1]);continue;}
        if same(&a,&b,true)? {result.overlaps.push([a0,a1,b1,b0]);continue;}
        let ad=(0..3).map(|i|(ah[i]-al[i]).powi(2)).sum::<f64>().sqrt();let bd=(0..3).map(|i|(bh[i]-bl[i]).powi(2)).sum::<f64>().sqrt();
        if ad.max(bd)<=1e-9 {
            let ta=a0+(a1-a0)*0.5;let tb=b0+(b1-b0)*0.5;
            if (a.evaluate(ta)-b.evaluate(tb)).length()<=4e-9 {result.hits.push([ta,tb]);}continue;
        }
        if ad>=bd {let (left,right)=curve_split(&a,a0+(a1-a0)*0.5).map_err(operation_error)?;work.push((left,b.clone(),depth+1));work.push((right,b,depth+1));}
        else {let (left,right)=curve_split(&b,b0+(b1-b0)*0.5).map_err(operation_error)?;work.push((a.clone(),left,depth+1));work.push((a,right,depth+1));}
    }
    result.hits.sort_by(|a,b|a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));result.hits.dedup_by(|a,b|(a[0]-b[0]).abs()<=1e-9 && (a[1]-b[1]).abs()<=1e-9);Ok(result)
}

fn trimmed_curve(curve:&brepkit_math::nurbs::curve::NurbsCurve,lo:f64,hi:f64)->Result<brepkit_math::nurbs::curve::NurbsCurve,NewellKernelError> {
    let mut result=curve.clone();let (a,b)=curve.domain();
    if lo>a {result=brepkit_math::nurbs::knot_ops::curve_split(&result,lo).map_err(operation_error)?.1;}
    if hi<b {result=brepkit_math::nurbs::knot_ops::curve_split(&result,hi).map_err(operation_error)?.0;}Ok(result)
}
fn curve_station_max(curve:&brepkit_math::nurbs::curve::NurbsCurve,station:crate::surface::SweepStation)->Result<f64,NewellKernelError> {
    let mut work=vec![(curve.clone(),0u32)];let mut best=f64::NEG_INFINITY;let mut certified=f64::NEG_INFINITY;let mut visits=0usize;
    while let Some((curve,depth))=work.pop() {
        visits+=1;if visits>65536||depth>64 {return Err(invalid("curve extremum complexity"));}
        if curve.weights().iter().any(|w|!w.is_finite()||*w<=0.) {return Err(invalid("rational curve bound"));}
        let upper=curve.control_points().iter().map(|p|station.value(*p)).fold(f64::NEG_INFINITY,f64::max);
        let (lo,hi)=curve.domain();let mid=lo+(hi-lo)*0.5;
        for t in [lo,mid,hi] {best=best.max(station.value(curve.evaluate(t)));}
        if upper<=best+1e-9 {certified=certified.max(upper);continue;}
        let (left,right)=brepkit_math::nurbs::knot_ops::curve_split(&curve,mid).map_err(operation_error)?;work.push((left,depth+1));work.push((right,depth+1));
    }
    Ok(certified.max(best))
}
impl NewellGeometry {
    fn parallel_sheet_boundaries(&self,face:FaceId,d:Vec3)->Result<Option<(brepkit_math::nurbs::curve::NurbsCurve,brepkit_math::nurbs::curve::NurbsCurve)>,NewellKernelError> {
        use brepkit_topology::{edge::EdgeCurve,face::FaceSurface};
        let face=self.topology.face(face).map_err(operation_error)?;
        if !face.inner_wires().is_empty() {return Ok(None);}
        match face.surface() {
            FaceSurface::Nurbs(surface) if surface.degree_v()==1 && surface.control_points().iter().all(|row|row.len()==2)=>{
                let low:Vec<_>=surface.control_points().iter().map(|row|row[0]).collect();let high:Vec<_>=surface.control_points().iter().map(|row|row[1]).collect();
                if low.iter().zip(&high).any(|(a,b)|(*b-*a).cross(d).length()>1e-9) {return Ok(None);}
                let curve=|points:Vec<_>,index|brepkit_math::nurbs::curve::NurbsCurve::new(surface.degree_u(),surface.knots_u().to_vec(),points,surface.weights().iter().map(|row|row[index]).collect()).map_err(operation_error);
                let low=curve(low,0)?;let high=curve(high,1)?;
                // A full ruled patch must still have the original four bounds.
                // Trimmed boolean faces cannot be treated as untrimmed supports.
                let edges=self.topology.wire(face.outer_wire()).map_err(operation_error)?.edges();
                if edges.len()!=4 {return Ok(None);}
                let mut bounds=Vec::new();
                for occurrence in edges {
                    let edge=self.topology.edge(occurrence.edge()).map_err(operation_error)?;
                    if matches!(edge.curve(),EdgeCurve::Line) {continue;}
                    let a=self.topology.vertex(edge.start()).map_err(operation_error)?.point();let b=self.topology.vertex(edge.end()).map_err(operation_error)?.point();
                    if let Some(curve)=crate::surface::boundary_curve(edge.curve(),a,b,true)? {bounds.push(curve);}
                }
                if bounds.len()!=2 {return Ok(None);}
                let covered=|a:&brepkit_math::nurbs::curve::NurbsCurve,b:&brepkit_math::nurbs::curve::NurbsCurve|->Result<bool,NewellKernelError>{let hits=exact_curve_intersections(a,b)?;let (lo,hi)=a.domain();let mut intervals:Vec<_>=hits.overlaps.into_iter().map(|p|(p[0].min(p[1]),p[0].max(p[1]))).collect();intervals.sort_by(|a,b|a.0.total_cmp(&b.0));let mut cursor=lo;for (a,b) in intervals {if a>cursor+1e-9 {return Ok(false);}cursor=cursor.max(b);}Ok(cursor>=hi-1e-9)};
                if !(covered(&low,&bounds[0])? && covered(&high,&bounds[1])? || covered(&low,&bounds[1])? && covered(&high,&bounds[0])?) {return Ok(None);}
                Ok(Some((low,high)))
            }
            FaceSurface::Cylinder(cylinder) if cylinder.axis().cross(d).length()<=1e-12=>{
                let mut curves=Vec::new();
                for occurrence in self.topology.wire(face.outer_wire()).map_err(operation_error)?.edges() {
                    let edge=self.topology.edge(occurrence.edge()).map_err(operation_error)?;if matches!(edge.curve(),EdgeCurve::Line){continue;}
                    let a=self.topology.vertex(edge.start()).map_err(operation_error)?.point();let b=self.topology.vertex(edge.end()).map_err(operation_error)?.point();
                    if let Some(curve)=crate::surface::boundary_curve(edge.curve(),a,b,true)? {curves.push(curve);}
                }
                if curves.len()==2 {Ok(Some((curves.remove(0),curves.remove(0))))} else {Ok(None)}
            }
            _=>Ok(None),
        }
    }
    fn curved_through_all_station(&self,profile:(NewellShapeKey,u32),boundary:crate::NewellBoundary,d:Vec3,start:f64,targets:&[NewellShapeKey],filled:bool)->Result<f64,NewellKernelError> {
        use brepkit_math::nurbs::curve::NurbsCurve;
        use crate::surface::SweepStation as Station;
        let source_face=self.planar_face(profile.0,profile.1)?;let source_frame=crate::profile::supporting_frame(&self.topology,source_face)?;
        let curves=self.selected_boundary_curves(profile,boundary)?;let normal=xyz_vector(source_frame.normal);let denominator=normal.dot(d);
        if denominator.abs()<=1e-12 {return Err(invalid("tangential Along"));}
        let global=Station{gradient:normal*(1./denominator),constant:-normal.dot(xyz_vector(source_frame.origin))/denominator};
        let source_trim=self.exact_planar_trim(NewellFaceTarget{shape:profile.0,face:profile.1,clearance:0.})?;
        let projected=|curve:&NurbsCurve|->Result<NurbsCurve,NewellKernelError>{NurbsCurve::new(curve.degree(),curve.knots().to_vec(),curve.control_points().iter().map(|p|*p-d*global.value(*p)).collect(),curve.weights().to_vec()).map_err(operation_error)};
        let mut farthest=f64::NEG_INFINITY;let mut unresolved=false;
        for target in targets {
            for (ordinal,face) in self.shape_faces(*target)?.into_iter().enumerate() {
                if matches!(self.topology.face(face).map_err(operation_error)?.surface(),brepkit_topology::face::FaceSurface::Plane{..}) {
                    let reference=NewellFaceTarget{shape:*target,face:u32::try_from(ordinal).map_err(operation_error)?,clearance:0.};
                    let trim=self.exact_planar_trim(reference)?;let n=xyz_vector(trim.frame.normal);
                    if n.dot(d).abs()<=1e-12 {
                        farthest=farthest.max(self.parallel_plane_chain_hit(&trim,&curves,source_frame,d,start,global)?);
                        if filled {for curve in trim.loops.iter().flatten() {let hit=projected(curve)?;for (lo,hi) in source_trim.covered_intervals(&hit)? {farthest=farthest.max(curve_station_max(&trimmed_curve(curve,lo,hi)?,global)?);}}}
                        continue;
                    }
                    let (station,_)=self.exact_target_station(reference,d,1.)?;
                    for curve in &curves {
                        let hit=NurbsCurve::new(curve.degree(),curve.knots().to_vec(),curve.control_points().iter().map(|p|station.point(*p,d)).collect(),curve.weights().to_vec()).map_err(operation_error)?;
                        for (lo,hi) in trim.covered_intervals(&hit)? {farthest=farthest.max(curve_station_max(&trimmed_curve(curve,lo,hi)?,station)?);}
                    }
                    if filled {
                        for curve in trim.loops.iter().flatten() {let hit=projected(curve)?;for (lo,hi) in source_trim.covered_intervals(&hit)? {farthest=farthest.max(curve_station_max(&trimmed_curve(curve,lo,hi)?,global)?);}}
                    }
                } else if let Some((low,high))=self.parallel_sheet_boundaries(face,d)? {
                    for end in [&low,&high] {
                        let hit=projected(end)?;
                        if filled {for (lo,hi) in source_trim.covered_intervals(&hit)? {farthest=farthest.max(curve_station_max(&trimmed_curve(end,lo,hi)?,global)?);}}
                        for source in &curves {
                            let intersections=exact_curve_intersections(source,&hit)?;
                            for pair in intersections.hits {farthest=farthest.max(global.value(end.evaluate(pair[1])));}
                            for overlap in intersections.overlaps {let (lo,hi)=(overlap[2].min(overlap[3]),overlap[2].max(overlap[3]));farthest=farthest.max(curve_station_max(&trimmed_curve(end,lo,hi)?,global)?);}
                        }
                    }
                } else {unresolved=true;}
            }
        }
        if unresolved {return Err(invalid("ThroughAll curved target needs a certified intersection route"));}
        if !farthest.is_finite() || farthest<=start+1e-7 {return Err(invalid("NoThroughAllHit"));}
        let scale=curves.iter().flat_map(|curve|curve.control_points()).flat_map(|p|[p.x().abs(),p.y().abs(),p.z().abs()]).fold(farthest.abs(),f64::max);
        let eps=1e-7_f64.max(1e-10*scale);let end=farthest+(10.*eps).max(1e-6);
        if !end.is_finite(){return Err(invalid("ThroughAll overflow"));}Ok(end)
    }
}

impl NewellGeometry {
    fn parallel_plane_chain_hit(&self,trim:&ExactPlanarTrim,curves:&[brepkit_math::nurbs::curve::NurbsCurve],source_frame:crate::NewellProfileFrame,d:Vec3,start:f64,global:crate::surface::SweepStation)->Result<f64,NewellKernelError> {
        use brepkit_math::nurbs::curve::NurbsCurve;
        let source_normal=xyz_vector(source_frame.normal);let target_normal=xyz_vector(trim.frame.normal);let source_origin=xyz_point(source_frame.origin);
        let transverse=target_normal-source_normal*target_normal.dot(source_normal);
        let center=source_origin+transverse*(target_normal.dot(xyz_vector(trim.frame.origin)-xyz_vector(source_frame.origin))/transverse.dot(transverse));
        let axis=source_normal.cross(target_normal).normalize().map_err(operation_error)?;
        let extent=curves.iter().flat_map(|curve|curve.control_points()).map(|p|(*p-center).length()).fold(1.,f64::max)+1.;
        let carrier=NurbsCurve::new(1,vec![0.,0.,1.,1.],vec![center-axis*extent,center+axis*extent],vec![1.,1.]).map_err(operation_error)?;
        let bound=trim.loops.iter().flatten().flat_map(|curve|curve.control_points()).map(|p|global.value(*p)).fold(start,f64::max)+1.;
        let line=|a,b|NurbsCurve::new(1,vec![0.,0.,1.,1.],vec![a,b],vec![1.,1.]).map_err(operation_error);
        let mut far=f64::NEG_INFINITY;
        for curve in curves {
            let intersections=exact_curve_intersections(curve,&carrier)?;
            for pair in intersections.hits {
                let p=curve.evaluate(pair[0]);let ray=line(p+d*(start+1e-7),p+d*bound)?;
                for (_,hi) in trim.covered_intervals(&ray)? {far=far.max(start+1e-7+(bound-start-1e-7)*hi);}
            }
            for interval in intersections.overlaps {
                let a=curve.evaluate(interval[0]);let b=curve.evaluate(interval[1]);
                if (a-b).length()<=1e-7 {continue;}
                let corners=[a+d*(start+1e-7),b+d*(start+1e-7),b+d*bound,a+d*bound];
                let wire=(0..4).map(|i|line(corners[i],corners[(i+1)%4])).collect::<Result<Vec<_>,_>>()?;
                let region=crate::boolean::Region{outer:corners.iter().map(|p|crate::boolean::project(trim.frame,*p)).collect(),holes:Vec::new()};
                let strip=ExactPlanarTrim{frame:trim.frame,loops:vec![wire],linear:Some(region)};
                for boundary in trim.loops.iter().flatten() {for (lo,hi) in strip.covered_intervals(boundary)? {far=far.max(curve_station_max(&trimmed_curve(boundary,lo,hi)?,global)?);}}
            }
        }
        Ok(far)
    }
}

/// Exact planar trim queries over the face's original rational boundary curves.
/// Curve intersections partition the tested domain; no sampled polygon is modeled.
struct ExactPlanarTrim {
    frame: crate::NewellProfileFrame,
    loops: Vec<Vec<brepkit_math::nurbs::curve::NurbsCurve>>,
    linear: Option<crate::boolean::Region>,
}
impl ExactPlanarTrim {
    fn contains(&self,point:brepkit_math::vec::Point3)->Result<bool,NewellKernelError> {
        if let Some(region) = &self.linear { return Ok(crate::boolean::contains_point(region,crate::boolean::project(self.frame,point))); }
        use brepkit_math::{nurbs::curve::NurbsCurve,vec::Point3};
        let u=xyz_vector(self.frame.u);let v=xyz_vector(self.frame.v);
        let extent=self.loops.iter().flatten().flat_map(|curve|curve.control_points()).map(|p|(*p-point).length()).fold(1.,f64::max)+1.;
        for attempt in 0..32 {
            let angle=0.371_390_676_354_103_7 + f64::from(attempt)*1.618_033_988_749_895;
            let direction=u*angle.cos()+v*angle.sin();
            let ray=NurbsCurve::new(1,vec![0.,0.,1.,1.],vec![point,point+direction*extent],vec![1.,1.]).map_err(operation_error)?;
            let mut parity=false;let mut ambiguous=false;
            for boundary in &self.loops {
                let mut crossings=Vec::new();
                for curve in boundary {
                    let result=exact_curve_intersections(curve,&ray)?;
                    if result.overlaps.iter().any(|span|span[2].min(span[3])*extent<=1e-7) {return Ok(true);}
                    if !result.overlaps.is_empty() {ambiguous=true;break;}
                    let (lo,hi)=curve.domain();
                    for hit in result.hits {
                        if (curve.evaluate(hit[0])-point).length()<=1e-7 {return Ok(true);}
                        if hit[0]<=lo+1e-9*(hi-lo) || hit[0]>=hi-1e-9*(hi-lo) {ambiguous=true;break;}
                        let delta=(hi-lo)*1e-6;
                        let left=curve.evaluate((hit[0]-delta).max(lo));let right=curve.evaluate((hit[0]+delta).min(hi));
                        let side=|p:Point3| (p-point).dot(u)*direction.dot(v)-(p-point).dot(v)*direction.dot(u);
                        if side(left)*side(right)<0. {crossings.push(hit[1]);}
                    }
                    if ambiguous {break;}
                }
                if ambiguous {break;}
                crossings.sort_by(f64::total_cmp);crossings.dedup_by(|a,b|(*a-*b).abs()*extent<=1e-7);
                parity^=crossings.len()%2==1;
            }
            if !ambiguous {return Ok(parity);}
        }
        Err(invalid("AmbiguousTarget trim query"))
    }
    fn covered_intervals(&self,curve:&brepkit_math::nurbs::curve::NurbsCurve)->Result<Vec<(f64,f64)>,NewellKernelError> {
        let (lo,hi)=curve.domain();let mut breaks=vec![lo,hi];
        for boundary in self.loops.iter().flatten() {
            let result=exact_curve_intersections(curve,boundary)?;
            breaks.extend(result.hits.into_iter().map(|hit|hit[0]));
            for overlap in result.overlaps {breaks.extend([overlap[0],overlap[1]]);}
        }
        breaks.retain(|t|t.is_finite() && *t>=lo && *t<=hi);breaks.sort_by(f64::total_cmp);breaks.dedup_by(|a,b|(*a-*b).abs()<=1e-10*(hi-lo));
        let mut result=Vec::new();
        for span in breaks.windows(2) {if self.contains(curve.evaluate(span[0]+(span[1]-span[0])*0.5))? {result.push((span[0],span[1]));}}
        Ok(result)
    }
    fn covers(&self,curve:&brepkit_math::nurbs::curve::NurbsCurve)->Result<bool,NewellKernelError> {
        if let Some(region) = &self.linear
            && curve.degree() == 1 && curve.control_points().len() == 2 {
            return Ok(crate::boolean::contains_segment(region,crate::boolean::project(self.frame,curve.control_points()[0]),crate::boolean::project(self.frame,curve.control_points()[1])));
        }
        let (lo,hi)=curve.domain();if !self.contains(curve.evaluate(lo))? || !self.contains(curve.evaluate(hi))? {return Ok(false);}
        let mut cursor=lo;
        for (a,b) in self.covered_intervals(curve)? {if a>cursor+1e-10*(hi-lo) {return Ok(false);}cursor=cursor.max(b);}
        Ok(cursor>=hi-1e-10*(hi-lo))
    }
}
impl NewellGeometry {
    fn exact_planar_trim(&self,target:NewellFaceTarget)->Result<ExactPlanarTrim,NewellKernelError> {
        let face=self.planar_face(target.shape,target.face)?;
        let frame=crate::profile::supporting_frame(&self.topology,face)?;
        let count=self.topology.face(face).map_err(operation_error)?.inner_wires().len()+1;
        let mut loops=Vec::new();
        for wire in 0..count {loops.push(self.selected_boundary_curves((target.shape,target.face),crate::NewellBoundary::Wire(u32::try_from(wire).map_err(operation_error)?))?);}
        let linear=if loops.iter().flatten().all(|curve|curve.degree()==1 && curve.control_points().len()==2) { Some(crate::boolean::line_region(&self.topology,face,frame)?) } else { None };
        Ok(ExactPlanarTrim {frame,loops,linear})
    }
    fn exact_target_station(&self,target:NewellFaceTarget,d:Vec3,sign:f64)->Result<(crate::surface::SweepStation,ExactPlanarTrim),NewellKernelError> {
        if !target.clearance.is_finite() || target.clearance<0. {return Err(invalid("target clearance"));}
        let trim=self.exact_planar_trim(target)?;let n=xyz_vector(trim.frame.normal);let denominator=n.dot(d);
        if denominator.abs()<=1e-12 {return Err(invalid("target parallel to sweep"));}
        Ok((crate::surface::SweepStation{gradient:n*(-1./denominator),constant:n.dot(xyz_vector(trim.frame.origin))/denominator-sign*target.clearance},trim))
    }
    fn exact_chain_coverage(&self,curves:&[brepkit_math::nurbs::curve::NurbsCurve],d:Vec3,station:crate::surface::SweepStation,trim:&ExactPlanarTrim)->Result<(),NewellKernelError> {
        use brepkit_math::nurbs::curve::NurbsCurve;
        for curve in curves {
            let projected=NurbsCurve::new(curve.degree(),curve.knots().to_vec(),curve.control_points().iter().map(|p|station.point(*p,d)).collect(),curve.weights().to_vec()).map_err(operation_error)?;
            if !trim.covers(&projected)? {return Err(invalid("target trim does not cover the swept chain"));}
        }
        Ok(())
    }
}

impl NewellGeometry {
    fn selected_boundary_curves(
        &self,
        profile: (NewellShapeKey, u32),
        boundary: crate::NewellBoundary,
    ) -> Result<Vec<brepkit_math::nurbs::curve::NurbsCurve>, NewellKernelError> {
        use brepkit_math::nurbs::curve::NurbsCurve;
        let face = self.planar_face(profile.0, profile.1)?;
        let face = self.topology.face(face).map_err(operation_error)?;
        let mut curves = Vec::new();
        for (wi, wire) in std::iter::once(face.outer_wire())
            .chain(face.inner_wires().iter().copied())
            .enumerate()
        {
            if match boundary {
                crate::NewellBoundary::Face => false,
                crate::NewellBoundary::Wire(index)
                | crate::NewellBoundary::Edge { wire: index, .. } => {
                    usize::try_from(index).map_err(operation_error)? != wi
                }
            } {
                continue;
            }
            for (ei, occurrence) in self
                .topology
                .wire(wire)
                .map_err(operation_error)?
                .edges()
                .iter()
                .enumerate()
            {
                if let crate::NewellBoundary::Edge { edge, .. } = boundary
                    && usize::try_from(edge).map_err(operation_error)? != ei
                {
                    continue;
                }
                let edge = self
                    .topology
                    .edge(occurrence.edge())
                    .map_err(operation_error)?;
                let a = self
                    .topology
                    .vertex(edge.start())
                    .map_err(operation_error)?
                    .point();
                let b = self
                    .topology
                    .vertex(edge.end())
                    .map_err(operation_error)?
                    .point();
                curves.push(
                    match crate::surface::boundary_curve(edge.curve(), a, b, true)? {
                        Some(curve) => curve,
                        None => NurbsCurve::new(1, vec![0., 0., 1., 1.], vec![a, b], vec![1., 1.])
                            .map_err(operation_error)?,
                    },
                );
            }
        }
        if curves.is_empty() {
            return Err(invalid("missing profile occurrence"));
        }
        Ok(curves)
    }
    fn certify_boundary_domain(
        &self,
        curves: &[brepkit_math::nurbs::curve::NurbsCurve],
        d: Vec3,
        station: crate::surface::SweepStation,
        trim: Option<(crate::NewellProfileFrame, &crate::boolean::Region)>,
        positive: Option<crate::surface::SweepStation>,
    ) -> Result<(), NewellKernelError> {
        use brepkit_math::vec::Point2;
        // A rational curve with positive weights lies in its control hull.
        // Subdivision tightens that exact bound; sampled witnesses only reject.
        // No polygon or sampled chord is used to construct model geometry.
        fn in_hull(point: Point2, points: &[Point2]) -> bool {
            let cross = |a: Point2, b: Point2, c: Point2| {
                (b.x() - a.x()) * (c.y() - a.y()) - (b.y() - a.y()) * (c.x() - a.x())
            };
            let mut points = points.to_vec();
            points.sort_by(|a, b| a.x().total_cmp(&b.x()).then(a.y().total_cmp(&b.y())));
            points.dedup();
            if points.len() < 3 {
                return false;
            }
            let mut hull = Vec::new();
            for p in &points {
                while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], *p) <= 0.
                {
                    hull.pop();
                }
                hull.push(*p);
            }
            let lower = hull.len();
            for p in points.iter().rev().skip(1) {
                while hull.len() > lower
                    && cross(hull[hull.len() - 2], hull[hull.len() - 1], *p) <= 0.
                {
                    hull.pop();
                }
                hull.push(*p);
            }
            hull.pop();
            hull.len() >= 3
                && (0..hull.len())
                    .all(|i| cross(hull[i], hull[(i + 1) % hull.len()], point) >= -1e-14)
        }
        let mut work: Vec<_> = curves.iter().cloned().map(|curve| (curve, 0u32)).collect();
        let mut count = 0usize;
        while let Some((curve, depth)) = work.pop() {
            count += 1;
            if count > 65536 || depth > 64 {
                return Err(invalid("boundary containment complexity"));
            }
            if curve.weights().iter().any(|w| !w.is_finite() || *w <= 0.) {
                return Err(invalid("rational curve bound"));
            }
            let points = curve.control_points();
            let reach = positive.is_none_or(|reach| points.iter().all(|p| reach.value(*p) > 1e-7));
            let covered = if let Some((frame, region)) = trim {
                let projected: Vec<_> = points
                    .iter()
                    .map(|p| crate::boolean::project(frame, station.point(*p, d)))
                    .collect();
                projected.iter().all(|a| {
                    projected
                        .iter()
                        .all(|b| crate::boolean::contains_segment(region, *a, *b))
                }) && !region
                    .holes
                    .iter()
                    .flatten()
                    .any(|p| in_hull(*p, &projected))
            } else {
                true
            };
            if reach && covered {
                continue;
            }
            let (lo, hi) = curve.domain();
            let mid = lo + (hi - lo) * 0.5;
            for t in [lo, mid, hi] {
                let p = curve.evaluate(t);
                if positive.is_some_and(|reach| reach.value(p) <= 1e-7) {
                    return Err(invalid("target is not beyond the displaced start"));
                }
                if let Some((frame, region)) = trim
                    && !crate::boolean::contains_point(
                        region,
                        crate::boolean::project(frame, station.point(p, d)),
                    )
                {
                    return Err(invalid("target trim does not cover the swept chain"));
                }
            }
            let (left, right) =
                brepkit_math::nurbs::knot_ops::curve_split(&curve, mid).map_err(operation_error)?;
            work.push((left, depth + 1));
            work.push((right, depth + 1));
        }
        Ok(())
    }
    fn curved_surface_extrude(
        &mut self,
        request: NewellSurfaceExtrude,
        d: Vec3,
    ) -> Result<Vec<NewellShapeKey>, NewellKernelError> {
        use crate::surface::SweepStation as Station;
        let curves = self.selected_boundary_curves(request.profile, request.boundary)?;
        let face = self.planar_face(request.profile.0, request.profile.1)?;
        let frame = crate::profile::supporting_frame(&self.topology, face)?;
        if xyz_vector(frame.normal).dot(d).abs() <= 1e-12 {
            return Err(invalid("tangential Along"));
        }
        let start = match request.start {
            NewellSurfaceStart::Distance(value) if value.is_finite() => value,
            NewellSurfaceStart::Distance(_) => return Err(invalid("starting distance")),
            NewellSurfaceStart::UpTo(target) => {
                let (station, trim) = self.exact_target_station(target, d, 1.)?;
                let target_frame = trim.frame;
                if xyz_vector(frame.normal)
                    .cross(xyz_vector(target_frame.normal))
                    .length()
                    > 1e-12
                {
                    return Err(invalid("InvalidStartPlane"));
                }
                self.exact_chain_coverage(&curves,d,station,&trim)?;
                station.value(xyz_point(frame.origin))
            }
        };
        let resolve = |extent: &NewellSurfaceExtent,
                       sign: f64|
         -> Result<Station, NewellKernelError> {
            let station = match extent {
                NewellSurfaceExtent::Blind(depth) if depth.is_finite() && *depth > 1e-7 => {
                    Station::fixed(start + sign * depth)
                }
                NewellSurfaceExtent::Blind(_) => return Err(invalid("nonpositive side depth")),
                NewellSurfaceExtent::UpTo(target) => {
                    let (station, trim) = self.exact_target_station(*target, d, sign)?;
                    let mut raw = station;
                    raw.constant += sign * target.clearance;
                    self.exact_chain_coverage(&curves,d,raw,&trim)?;
                    station
                }
                NewellSurfaceExtent::ThroughAll(targets) => Station::fixed(self.curved_through_all_station(request.profile,request.boundary,d,start,targets,false)?),
            };
            self.certify_boundary_domain(
                &curves,
                d,
                station,
                None,
                Some(Station {
                    gradient: station.gradient * sign,
                    constant: sign * (station.constant - start),
                }),
            )?;
            Ok(station)
        };
        let main = resolve(&request.extent, 1.)?;
        let (low, high) = match &request.sides {
            NewellSurfaceSides::One => (Station::fixed(start), main),
            NewellSurfaceSides::Symmetric => (
                Station {
                    gradient: main.gradient * (-1.),
                    constant: 2. * start - main.constant,
                },
                main,
            ),
            NewellSurfaceSides::Second {
                extent,
                reverse: false,
            } => (resolve(extent, -1.)?, main),
            NewellSurfaceSides::Second {
                extent,
                reverse: true,
            } => {
                let second = resolve(extent, 1.)?;
                let difference = Station {
                    gradient: main.gradient - second.gradient,
                    constant: main.constant - second.constant,
                };
                if curves.iter().all(|curve| {
                    curve
                        .control_points()
                        .iter()
                        .all(|p| difference.value(*p) >= 0.)
                }) {
                    (Station::fixed(start), main)
                } else if curves.iter().all(|curve| {
                    curve
                        .control_points()
                        .iter()
                        .all(|p| difference.value(*p) <= 0.)
                }) {
                    (Station::fixed(start), second)
                } else {
                    return Err(invalid(
                        "crossing curved side endpoints need exact curve splitting",
                    ));
                }
            }
        };
        self.sweep_surface_between(
            request.profile,
            request.boundary,
            [d.x(), d.y(), d.z()],
            low,
            high,
        )
    }
}

impl NewellGeometry {
    /// Proves an identical rational generating curve after affine parameter
    /// normalization, exact knot splitting and homogeneous degree elevation.
    /// Only a constant translation along the common sweep direction is removed.
    pub(crate) fn equivalent_sweep_support(
        &self,
        reference: crate::surface::SurfaceSweepRecipe,
        other: crate::surface::SurfaceSweepRecipe,
    ) -> Result<Option<(f64, bool)>, NewellKernelError> {
        use brepkit_math::{nurbs::curve::NurbsCurve, vec::Point3};
        if reference.direction != other.direction {
            return Ok(None);
        }
        fn normalized(curve: &NurbsCurve, reverse: bool) -> Result<NurbsCurve, NewellKernelError> {
            let (lo, hi) = curve.domain();
            let mut points = curve.control_points().to_vec();
            let mut weights = curve.weights().to_vec();
            let mut knots: Vec<_> = curve.knots().iter().map(|k| (k - lo) / (hi - lo)).collect();
            if reverse {
                points.reverse();
                weights.reverse();
                knots = knots.iter().rev().map(|k| 1. - k).collect();
            }
            NurbsCurve::new(curve.degree(), knots, points, weights).map_err(operation_error)
        }
        fn part(curve: &NurbsCurve, lo: f64, hi: f64) -> Result<NurbsCurve, NewellKernelError> {
            let mut result = curve.clone();
            if lo > 0. {
                result = brepkit_math::nurbs::knot_ops::curve_split(&result, lo)
                    .map_err(operation_error)?
                    .1;
            }
            if hi < 1. {
                result = brepkit_math::nurbs::knot_ops::curve_split(&result, hi)
                    .map_err(operation_error)?
                    .0;
            }
            Ok(result)
        }
        fn elevated(curve: &NurbsCurve, degree: usize) -> Result<Vec<[f64; 4]>, NewellKernelError> {
            let scale = curve
                .weights()
                .first()
                .copied()
                .ok_or_else(|| invalid("empty rational boundary"))?;
            let mut points: Vec<_> = curve
                .control_points()
                .iter()
                .zip(curve.weights())
                .map(|(p, w)| {
                    [
                        p.x() * w / scale,
                        p.y() * w / scale,
                        p.z() * w / scale,
                        w / scale,
                    ]
                })
                .collect();
            if points.len() != curve.degree() + 1 {
                return Err(invalid("Bezier support decomposition"));
            }
            while points.len() < degree + 1 {
                let n = points.len();
                let mut next = vec![points[0]];
                for i in 1..n {
                    let a = i as f64 / n as f64;
                    next.push(std::array::from_fn(|j| {
                        a * points[i - 1][j] + (1. - a) * points[i][j]
                    }));
                }
                next.push(points[n - 1]);
                points = next;
            }
            Ok(points)
        }
        let source = self.selected_boundary_curves(reference.profile, reference.boundary)?;
        let other = self.selected_boundary_curves(other.profile, other.boundary)?;
        if source.len() != other.len() {
            return Ok(None);
        }
        let d = xyz_vector(reference.direction);
        for reverse in [false, true] {
            let mut offset = None;
            let mut matches = true;
            for (index, a) in source.iter().enumerate() {
                let b = &other[if reverse {
                    other.len() - 1 - index
                } else {
                    index
                }];
                let a = normalized(a, false)?;
                let b = normalized(b, reverse)?;
                let mut knots: Vec<_> = a
                    .knots()
                    .iter()
                    .chain(b.knots())
                    .copied()
                    .filter(|k| *k >= 0. && *k <= 1.)
                    .collect();
                knots.sort_by(f64::total_cmp);
                knots.dedup();
                for span in knots.windows(2) {
                    if span[1] - span[0] <= 1e-14 {
                        continue;
                    }
                    let degree = a.degree().max(b.degree());
                    let aa = elevated(&part(&a, span[0], span[1])?, degree)?;
                    let bb = elevated(&part(&b, span[0], span[1])?, degree)?;
                    // Weight normalization is local to each exact Bezier span.
                    let wa = aa[0][3];
                    let wb = bb[0][3];
                    for (a, b) in aa.iter().zip(&bb) {
                        if (a[3] / wa - b[3] / wb).abs() > 1e-11 {
                            matches = false;
                            break;
                        }
                        let pa = Point3::new(a[0] / a[3], a[1] / a[3], a[2] / a[3]);
                        let pb = Point3::new(b[0] / b[3], b[1] / b[3], b[2] / b[3]);
                        let delta = pb - pa;
                        let station = delta.dot(d);
                        let shift = *offset.get_or_insert(station);
                        if (delta - d * station).length() > 1e-9 || (station - shift).abs() > 1e-9 {
                            matches = false;
                            break;
                        }
                    }
                    if !matches {
                        break;
                    }
                }
                if !matches {
                    break;
                }
            }
            if matches {
                return Ok(offset.map(|offset| (offset, reverse)));
            }
        }
        Ok(None)
    }
}

impl NewellGeometry {
    fn line_profile(
        &self,
        profile: (NewellShapeKey, u32),
        boundary: crate::NewellBoundary,
    ) -> Result<Option<LineProfile>, NewellKernelError> {
        let face = self.planar_face(profile.0, profile.1)?;
        let frame = crate::profile::supporting_frame(&self.topology, face)?;
        let description = self
            .profile_faces(profile.0)?
            .into_iter()
            .find(|face| face.face == profile.1)
            .ok_or_else(|| invalid("profile support"))?;
        let mut edges = Vec::new();
        for wire in description.wires {
            if match boundary {
                crate::NewellBoundary::Face => false,
                crate::NewellBoundary::Wire(index)
                | crate::NewellBoundary::Edge { wire: index, .. } => index != wire.wire,
            } {
                continue;
            }
            for edge in wire.edges {
                if let crate::NewellBoundary::Edge { edge: index, .. } = boundary
                    && index != edge.edge
                {
                    continue;
                }
                if !edge.straight {
                    return Ok(None);
                }
                edges.push([xyz_point(edge.start), xyz_point(edge.end)]);
            }
        }
        if edges.is_empty() {
            return Err(invalid("missing profile occurrence"));
        }
        Ok(Some(LineProfile { face, frame, edges }))
    }
    fn line_start(
        &self,
        profile: &LineProfile,
        d: Vec3,
        start: NewellSurfaceStart,
    ) -> Result<f64, NewellKernelError> {
        match start {
            NewellSurfaceStart::Distance(value) if value.is_finite() => Ok(value),
            NewellSurfaceStart::Distance(_) => Err(invalid("starting distance")),
            NewellSurfaceStart::UpTo(target) => {
                let (endpoint, trim) = self.exact_target_station(target,d,1.)?;
                let frame = trim.frame;
                let n = xyz_vector(frame.normal);
                if n.cross(xyz_vector(profile.frame.normal)).length() > 1e-12 {
                    return Err(invalid("InvalidStartPlane"));
                }
                let denominator = n.dot(d);
                if denominator.abs() <= 1e-12 {
                    return Err(invalid("tangential starting ray"));
                }
                let station = n.dot(xyz_vector(frame.origin) - xyz_vector(profile.frame.origin))
                    / denominator;
                for edge in &profile.edges {
                    let curve = brepkit_math::nurbs::curve::NurbsCurve::new(1,vec![0.,0.,1.,1.],edge.to_vec(),vec![1.,1.]).map_err(operation_error)?;
                    self.exact_chain_coverage(&[curve],d,endpoint,&trim)?;
                }
                Ok(station)
            }
        }
    }
    fn line_endpoint(
        &self,
        edge: [brepkit_math::vec::Point3; 2],
        d: Vec3,
        start: f64,
        extent: &NewellSurfaceExtent,
        sign: f64,
    ) -> Result<[f64; 2], NewellKernelError> {
        let end = match extent {
            NewellSurfaceExtent::Blind(value) => {
                if !value.is_finite() || *value <= 1e-7 {
                    return Err(invalid("nonpositive side depth"));
                }
                [start + sign * value; 2]
            }
            NewellSurfaceExtent::UpTo(target) => {
                let (endpoint, trim) = self.exact_target_station(*target,d,sign)?;
                let frame = trim.frame;
                let n = xyz_vector(frame.normal);
                let h = n.dot(xyz_vector(frame.origin));
                let denominator = n.dot(d);
                if denominator.abs() <= 1e-12 {
                    return Err(invalid("target parallel to sweep"));
                }
                let raw = edge.map(|p| (h - n.dot(Vec3::new(p.x(), p.y(), p.z()))) / denominator);
                let curve = brepkit_math::nurbs::curve::NurbsCurve::new(1,vec![0.,0.,1.,1.],edge.to_vec(),vec![1.,1.]).map_err(operation_error)?;
                let mut unadjusted = endpoint; unadjusted.constant += sign * target.clearance;
                self.exact_chain_coverage(&[curve],d,unadjusted,&trim)?;
                raw.map(|value| value - sign * target.clearance)
            }
            NewellSurfaceExtent::ThroughAll(_) => {
                return Err(invalid("unresolved ThroughAll endpoint"));
            }
        };
        if end
            .iter()
            .any(|value| !value.is_finite() || sign * (value - start) <= 1e-7)
        {
            return Err(invalid("target is not beyond the displaced start"));
        }
        Ok(end)
    }
    fn line_through_all(
        &self,
        profile: &LineProfile,
        d: Vec3,
        start: f64,
        targets: &[NewellShapeKey],
        filled: bool,
    ) -> Result<Option<f64>, NewellKernelError> {
        use crate::boolean::{contains_point, line_intervals, line_region, project, unproject};
        let mut faces = Vec::new();
        let mut scale = profile
            .edges
            .iter()
            .flatten()
            .map(|p| p.x().abs().max(p.y().abs()).max(p.z().abs()))
            .fold(start.abs(), f64::max);
        for target in targets {
            for face in self.shape_faces(*target)? {
                let Ok(frame) = crate::profile::supporting_frame(&self.topology, face) else {
                    return Ok(None);
                };
                let Ok(region) = line_region(&self.topology, face, frame) else {
                    return Ok(None);
                };
                for point in region.outer.iter().chain(region.holes.iter().flatten()) {
                    let p = unproject(frame, *point);
                    scale = scale.max(p.x().abs()).max(p.y().abs()).max(p.z().abs());
                }
                faces.push((frame, region));
            }
        }
        let eps = 1e-7_f64.max(scale * 1e-10);
        let mut farthest = f64::NEG_INFINITY;
        let normal = xyz_vector(profile.frame.normal);
        let axis = normal * (1. / normal.dot(d));
        let origin = xyz_vector(profile.frame.origin);
        let reference = axis.dot(origin);
        if filled {
            let source = line_region(&self.topology, profile.face, profile.frame)?;
            for (frame, region) in &faces {
                for point in region.outer.iter().chain(region.holes.iter().flatten()) {
                    let p = unproject(*frame, *point);
                    let station = axis.dot(Vec3::new(p.x(), p.y(), p.z())) - reference;
                    if station > start + eps
                        && contains_point(&source, project(profile.frame, p - d * station))
                    {
                        farthest = farthest.max(station);
                    }
                }
            }
        }
        for edge in &profile.edges {
            let side = side_frame(*edge, d)?;
            let n = xyz_vector(side.normal);
            let h = n.dot(xyz_vector(side.origin));
            for (frame, region) in &faces {
                let m = xyz_vector(frame.normal);
                let k = m.dot(xyz_vector(frame.origin));
                let cross = n.cross(m);
                let length = cross.length();
                let mut segments = Vec::new();
                if length <= 1e-12 {
                    if n.dot(xyz_vector(frame.origin) - xyz_vector(side.origin))
                        .abs()
                        > eps
                    {
                        continue;
                    }
                    for boundary in std::iter::once(&region.outer).chain(&region.holes) {
                        for (a, b) in boundary
                            .iter()
                            .zip(boundary.iter().cycle().skip(1))
                            .take(boundary.len())
                        {
                            segments.push([unproject(*frame, *a), unproject(*frame, *b)]);
                        }
                    }
                } else {
                    let point = (m.cross(cross) * h + cross.cross(n) * k) * (1. / cross.dot(cross));
                    let point = brepkit_math::vec::Point3::new(point.x(), point.y(), point.z());
                    let direction = cross * (1. / length);
                    let spans = line_intervals(
                        std::slice::from_ref(region),
                        project(*frame, point),
                        brepkit_math::vec::Vec2::new(
                            direction.dot(xyz_vector(frame.u)),
                            direction.dot(xyz_vector(frame.v)),
                        ),
                    );
                    for (a, b, _) in spans {
                        segments.push([point + direction * a, point + direction * b]);
                    }
                }
                for segment in segments {
                    let a = side_coordinates(*edge, d, segment[0])?;
                    let b = side_coordinates(*edge, d, segment[1])?;
                    let (mut low, mut high) = (0_f64, 1_f64);
                    if !clip_parameter(&mut low, &mut high, a.0, b.0 - a.0, 0., 1.)
                        || !clip_parameter(
                            &mut low,
                            &mut high,
                            a.1,
                            b.1 - a.1,
                            start + eps,
                            f64::INFINITY,
                        )
                    {
                        continue;
                    }
                    farthest = farthest
                        .max(a.1 + (b.1 - a.1) * low)
                        .max(a.1 + (b.1 - a.1) * high);
                }
            }
        }
        if !farthest.is_finite() {
            return Err(invalid("NoThroughAllHit"));
        }
        Ok(Some(farthest + (10. * eps).max(1e-6)))
    }
    /// Resolves all line-profile surface extents as exact regions in each sweep plane.
    /// Curves keep the direct exact Blind constructor when no target clipping is needed.
    ///
    /// # Errors
    /// Rejects invalid reach/trim, nonmanifold unions, no-hit and unverified curved predicates.
    pub fn extrude_surface(
        &mut self,
        request: NewellSurfaceExtrude,
    ) -> Result<Vec<NewellShapeKey>, NewellKernelError> {
        let d = xyz_vector(request.direction)
            .normalize()
            .map_err(operation_error)?;
        let Some(profile) = self.line_profile(request.profile, request.boundary)? else {
            return self.curved_surface_extrude(request, d);
        };
        if xyz_vector(profile.frame.normal).dot(d).abs() <= 1e-12 {
            return Err(invalid("tangential Along"));
        }
        let start = self.line_start(&profile, d, request.start)?;
        let extent = if let NewellSurfaceExtent::ThroughAll(targets) = &request.extent {
            let end = match self.line_through_all(&profile,d,start,targets,false)? {
                Some(end)=>end,
                None=>self.curved_through_all_station(request.profile,request.boundary,d,start,targets,false)?,
            };
            NewellSurfaceExtent::Blind(end - start)
        } else {
            request.extent
        };
        let mut groups = Vec::new();
        for edge in profile.edges {
            let frame = side_frame(edge, d)?;
            let end = self.line_endpoint(edge, d, start, &extent, 1.)?;
            let quad = |low: [f64; 2], high: [f64; 2]| crate::boolean::Region {
                outer: vec![
                    crate::boolean::project(frame, edge[0] + d * low[0]),
                    crate::boolean::project(frame, edge[1] + d * low[1]),
                    crate::boolean::project(frame, edge[1] + d * high[1]),
                    crate::boolean::project(frame, edge[0] + d * high[0]),
                ],
                holes: Vec::new(),
            };
            let mut regions = vec![quad([start; 2], end)];
            match &request.sides {
                NewellSurfaceSides::One => {}
                NewellSurfaceSides::Symmetric => {
                    regions = vec![quad(end.map(|value| 2. * start - value), end)]
                }
                NewellSurfaceSides::Second { extent, reverse } => {
                    let sign = if *reverse { 1. } else { -1. };
                    let second = self.line_endpoint(edge, d, start, extent, sign)?;
                    let side = if *reverse {
                        quad([start; 2], second)
                    } else {
                        quad(second, [start; 2])
                    };
                    regions = crate::boolean::region_boolean(
                        &regions,
                        &[side],
                        crate::NewellBooleanMode::Add,
                    )?;
                }
            }
            crate::boolean::merge_groups(
                &mut groups,
                &[crate::boolean::PlaneRegions { frame, regions }],
            )?;
        }
        self.rebuild_line_surfaces(&groups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brepkit_math::{mat::Mat4, vec::Point3};
    use brepkit_topology::{
        builder::make_polygon_wire,
        face::{Face, FaceSurface},
    };
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    #[test]
    fn curved_through_all_uses_finite_hits_parallel_sheets_and_filled_interiors() -> TestResult {
        let solid=crate::NewellProfile::closed_cubic([[0.,0.],[2.,3.],[4.,2.],[6.,0.]])?.extrude([0.,0.,1.],1.)?;
        let mut geometry=NewellGeometry::new();geometry.topology=solid.topology;let source=geometry.insert(solid.solid)?;
        let profile=geometry.profile_faces(source)?.into_iter().find(|face|face.frame.normal==[0.,0.,1.]).ok_or("profile")?;
        let edge=profile.wires[0].edges.iter().find(|edge|!edge.straight).ok_or("cubic")?.edge;
        let boundary=crate::NewellBoundary::Edge{wire:0,edge};
        let a=geometry.box_shape([2.,10.,1.],[0.,-4.,2.])?;
        let b=geometry.box_shape([2.,10.,1.],[4.,-4.,8.])?;
        let miss=geometry.box_shape([1.;3],[100.;3])?;
        let request=NewellSurfaceExtrude{profile:(source,profile.face),boundary,direction:[0.,0.,1.],start:NewellSurfaceStart::Distance(0.),extent:NewellSurfaceExtent::ThroughAll(vec![a,b,miss]),sides:NewellSurfaceSides::One};
        let sheet=geometry.extrude_surface(request.clone())?[0];
        let curves=geometry.selected_boundary_curves((source,profile.face),boundary)?;
        let mut sum=0.;let n=16384;for i in 0..=n {let t=f64::from(i)/f64::from(n);let q=9.-24.*t+9.*t*t;sum+=(if i==0||i==n{1.}else if i%2==0{2.}else{4.})*(36.+q*q).sqrt();}let length=sum/(3.*f64::from(n));
        assert!((geometry.surface_area(sheet)?-(8.+1e-6)*length).abs()<1e-6);
        assert_eq!(curves.len(),1);
        assert!((geometry.through_all_station((source,profile.face),[0.,0.,1.],0.,&[a,b,miss])?-8.-1e-6).abs()<1e-8);
        let tall=geometry.sweep_surface((source,profile.face),boundary,[0.,0.,1.],5.,12.)?[0];
        let mut parallel=request.clone();parallel.extent=NewellSurfaceExtent::ThroughAll(vec![a,tall,miss]);
        let sheet=geometry.extrude_surface(parallel)?[0];assert!((geometry.surface_area(sheet)?-(12.+1e-6)*length).abs()<1e-6);
        let inside=geometry.box_shape([0.2,0.2,1.],[2.7,0.4,10.])?;
        assert!((geometry.through_all_station((source,profile.face),[0.,0.,1.],0.,&[inside])?-10.-1e-6).abs()<1e-8);
        let mut boundary_miss=request;boundary_miss.extent=NewellSurfaceExtent::ThroughAll(vec![inside]);
        assert!(geometry.extrude_surface(boundary_miss).is_err());
        Ok(())
    }
    #[test]
    fn exact_circular_target_trim_classifies_the_whole_chain() -> TestResult {
        let mut geometry=NewellGeometry::new();
        let cylinder=brepkit_operations::primitives::make_cylinder(&mut geometry.topology,2.,1.)?;
        brepkit_operations::transform::transform_solid(&mut geometry.topology,cylinder,&Mat4::translation(0.,0.,5.))?;
        let target=geometry.insert(cylinder)?;
        let face=geometry.profile_faces(target)?.into_iter().find(|face|face.frame.normal==[0.,0.,1.]).ok_or("cap")?.face;
        let trim=geometry.exact_planar_trim(NewellFaceTarget{shape:target,face,clearance:0.})?;
        assert!(trim.contains(Point3::new(0.,0.,6.))?);
        assert!(trim.contains(Point3::new(1.9,0.,6.))?);
        assert!(!trim.contains(Point3::new(2.1,0.,6.))?);
        let source=geometry.box_shape([1.,1.,1.],[-0.5,-0.5,0.])?;
        let profile=geometry.profile_faces(source)?.into_iter().find(|face|face.frame.normal==[0.,0.,1.]).ok_or("profile")?.face;
        let request=NewellSurfaceExtrude{profile:(source,profile),boundary:crate::NewellBoundary::Face,direction:[0.,0.,1.],start:NewellSurfaceStart::Distance(0.),extent:NewellSurfaceExtent::UpTo(NewellFaceTarget{shape:target,face,clearance:0.}),sides:NewellSurfaceSides::One};
        let sheet=geometry.extrude_surface(request)?[0];assert!((geometry.surface_area(sheet)?-20.).abs()<1e-6);
        Ok(())
    }
    #[test]
    fn cubic_open_sheet_terminates_at_a_tilted_trim_with_exact_lifted_boundary() -> TestResult {
        let solid = crate::NewellProfile::closed_cubic([[0., 0.], [2., 3.], [4., 2.], [6., 0.]])?
            .extrude([0., 0., 1.], 1.)?;
        let mut geometry = NewellGeometry::new();
        geometry.topology = solid.topology;
        let source = geometry.insert(solid.solid)?;
        let profile = geometry
            .profile_faces(source)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("profile")?;
        let edge = profile.wires[0]
            .edges
            .iter()
            .find(|edge| !edge.straight)
            .ok_or("cubic")?
            .edge;
        let target = geometry.box_shape([20., 20., 1.], [-10., -10., 0.])?;
        let mut transform = Mat4::rotation_y(-0.1_f64.atan());
        transform.0[2][3] = 5.;
        let native = geometry.solid(target)?;
        brepkit_operations::transform::transform_solid(&mut geometry.topology, native, &transform)?;
        let face = geometry
            .profile_faces(target)?
            .into_iter()
            .find(|face| face.frame.normal[2] < -0.9)
            .ok_or("target")?
            .face;
        let request = NewellSurfaceExtrude {
            profile: (source, profile.face),
            boundary: crate::NewellBoundary::Edge { wire: 0, edge },
            direction: [0., 0., 1.],
            start: NewellSurfaceStart::Distance(0.),
            extent: NewellSurfaceExtent::UpTo(NewellFaceTarget {
                shape: target,
                face,
                clearance: 0.,
            }),
            sides: NewellSurfaceSides::One,
        };
        let result = geometry.extrude_surface(request.clone())?[0];
        // Independent Simpson integral of (4+0.6t)*sqrt(36+(9-24t+9t²)²).
        // The fourth derivative is bounded conservatively by 1e9 here;
        // N=16384 gives a truncation bound below 8e-11 square millimetres.
        let n = 16384;
        let mut sum = 0.;
        for i in 0..=n {
            let t = f64::from(i) / f64::from(n);
            let q = 9. - 24. * t + 9. * t * t;
            let weight = if i == 0 || i == n {
                1.
            } else if i % 2 == 0 {
                2.
            } else {
                4.
            };
            sum += weight * (4. + 0.6 * t) * (36. + q * q).sqrt();
        }
        let expected = sum / (3. * f64::from(n));
        assert!((geometry.surface_area(result)? - expected).abs() < 1e-6);
        assert_eq!(geometry.face_count(result)?, 1);
        assert!(!geometry.tessellate(result, 0.01)?.indices().is_empty());
        let native = geometry.topology.face(geometry.shape_faces(result)?[0])?;
        let FaceSurface::Nurbs(surface) = native.surface() else {
            return Err("lost exact curved support".into());
        };
        assert_eq!(surface.degree_u(), 3);
        assert_eq!(surface.degree_v(), 1);
        for row in surface.control_points() {
            assert!((row[1].z() - 5. - 0.1 * row[1].x()).abs() < 1e-10);
            assert!((row[0].z() - 1.).abs() < 1e-10);
        }
        let mut symmetric = request.clone();
        symmetric.sides = NewellSurfaceSides::Symmetric;
        let doubled = geometry.extrude_surface(symmetric)?[0];
        assert!((geometry.surface_area(doubled)? - 2. * expected).abs() < 1e-6);
        let small = geometry.box_shape([1., 20., 1.], [0., -10., 4.])?;
        let face = geometry
            .profile_faces(small)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("small target")?
            .face;
        let mut missed = request;
        missed.extent = NewellSurfaceExtent::UpTo(NewellFaceTarget {
            shape: small,
            face,
            clearance: 0.,
        });
        assert!(geometry.extrude_surface(missed).is_err());
        Ok(())
    }
    #[test]
    fn renderer_conversion_rejects_lost_body_and_sheet_detail_without_mutating_sources()
    -> TestResult {
        let mut geometry = NewellGeometry::new();
        let body = geometry.box_shape([0.125; 3], [1e7; 3])?;
        assert!(matches!(
            geometry.tessellate(body, 0.05),
            Err(NewellKernelError::MeshPrecisionLoss)
        ));
        assert!((geometry.volume(body)? - 0.125_f64.powi(3)).abs() < 1e-6);
        let face = geometry
            .profile_faces(body)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("profile")?
            .face;
        let sheet = geometry.sweep_surface(
            (body, face),
            crate::NewellBoundary::Face,
            [0., 0., 1.],
            0.,
            0.125,
        )?[0];
        assert!(matches!(
            geometry.tessellate(sheet, 0.05),
            Err(NewellKernelError::MeshPrecisionLoss)
        ));
        assert!((geometry.surface_area(sheet)? - 0.0625).abs() < 1e-6);
        // Large coordinates themselves are valid when the detail survives f32.
        let representable = geometry.box_shape([128.; 3], [1e7; 3])?;
        assert!(
            !geometry
                .tessellate(representable, 0.05)?
                .indices()
                .is_empty()
        );
        Ok(())
    }
    #[test]
    fn surface_target_checks_the_selected_chain_including_interior_holes() -> TestResult {
        let mut geometry = NewellGeometry::new();
        let source = geometry.box_shape([2., 3., 1.], [0.; 3])?;
        let profile = geometry
            .profile_faces(source)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("profile")?;
        let outer = make_polygon_wire(
            &mut geometry.topology,
            &[
                Point3::new(-1., -1., 5.),
                Point3::new(3., -1., 5.),
                Point3::new(3., 4., 5.),
                Point3::new(-1., 4., 5.),
            ],
            1e-7,
        )?;
        let hole = make_polygon_wire(
            &mut geometry.topology,
            &[
                Point3::new(0.75, 1., 5.),
                Point3::new(0.75, 2., 5.),
                Point3::new(1.25, 2., 5.),
                Point3::new(1.25, 1., 5.),
            ],
            1e-7,
        )?;
        let native = geometry.topology.add_face(Face::new(
            outer,
            vec![hole],
            FaceSurface::Plane {
                normal: Vec3::new(0., 0., 1.),
                d: 5.,
            },
        ));
        let target = geometry.insert_surface(vec![native])?;
        let request = NewellSurfaceExtrude {
            profile: (source, profile.face),
            boundary: crate::NewellBoundary::Face,
            direction: [0., 0., 1.],
            start: NewellSurfaceStart::Distance(0.),
            extent: NewellSurfaceExtent::UpTo(NewellFaceTarget {
                shape: target,
                face: 0,
                clearance: 0.,
            }),
            sides: NewellSurfaceSides::One,
        };
        // The hole is inside the supporting face, but misses every selected
        // boundary: an uncapped perimeter sweep is valid and has area 10 * 4.
        let sheet = geometry.extrude_surface(request.clone())?[0];
        assert!((geometry.surface_area(sheet)? - 40.).abs() < 1e-6);
        let crossing = geometry.box_shape([2., 1., 1.], [0., 1.5, 0.])?;
        let profile = geometry
            .profile_faces(crossing)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("crossing profile")?;
        let edge = profile.wires[0]
            .edges
            .iter()
            .find(|edge| (edge.start[1] - 1.5).abs() < 1e-7 && (edge.end[1] - 1.5).abs() < 1e-7)
            .ok_or("crossing edge")?
            .edge;
        let mut rejected = request;
        rejected.profile = (crossing, profile.face);
        rejected.boundary = crate::NewellBoundary::Edge { wire: 0, edge };
        // Both endpoints hit the trim; the middle crosses its hole.
        assert!(geometry.extrude_surface(rejected).is_err());
        Ok(())
    }
    #[test]
    fn surface_start_up_to_requires_only_selected_chain_coverage_and_a_parallel_plane() -> TestResult
    {
        let mut geometry = NewellGeometry::new();
        let source = geometry.box_shape([2., 3., 1.], [0.; 3])?;
        let profile = geometry
            .profile_faces(source)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("profile")?;
        let edge = profile.wires[0]
            .edges
            .iter()
            .find(|edge| edge.start[1].abs() < 1e-7 && edge.end[1].abs() < 1e-7)
            .ok_or("edge")?
            .edge;
        let target = geometry.box_shape([4., 0.2, 1.], [-1., -0.1, 3.])?;
        let target_face = geometry
            .profile_faces(target)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., -1.])
            .ok_or("target")?
            .face;
        let request = NewellSurfaceExtrude {
            profile: (source, profile.face),
            boundary: crate::NewellBoundary::Edge { wire: 0, edge },
            direction: [0., 0., 1.],
            start: NewellSurfaceStart::UpTo(NewellFaceTarget {
                shape: target,
                face: target_face,
                clearance: 0.,
            }),
            extent: NewellSurfaceExtent::Blind(3.),
            sides: NewellSurfaceSides::One,
        };
        let sheet = geometry.extrude_surface(request.clone())?[0];
        assert!((geometry.surface_area(sheet)? - 6.).abs() < 1e-6);
        let mut all_boundaries = request.clone();
        all_boundaries.boundary = crate::NewellBoundary::Face;
        assert!(geometry.extrude_surface(all_boundaries).is_err());
        let native = geometry.solid(target)?;
        brepkit_operations::transform::transform_solid(
            &mut geometry.topology,
            native,
            &Mat4::rotation_y(0.1),
        )?;
        assert!(matches!(
            geometry.extrude_surface(request),
            Err(NewellKernelError::InvalidParameter {
                parameter: "InvalidStartPlane"
            })
        ));
        Ok(())
    }
    #[test]
    fn surface_tilted_end_symmetric_and_second_reverse_use_exact_regions() -> TestResult {
        let mut geometry = NewellGeometry::new();
        let source = geometry.box_shape([2., 3., 1.], [0.; 3])?;
        let face = geometry
            .profile_faces(source)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("profile")?;
        let edge = face.wires[0]
            .edges
            .iter()
            .find(|edge| edge.start[1].abs() < 1e-7 && edge.end[1].abs() < 1e-7)
            .ok_or("edge")?
            .edge;
        let target = geometry.box_shape([20., 20., 1.], [-10., -10., 0.])?;
        let native = geometry.solid(target)?;
        let mut matrix = Mat4::rotation_y(-0.5_f64.atan());
        matrix.0[2][3] = 5.;
        brepkit_operations::transform::transform_solid(&mut geometry.topology, native, &matrix)?;
        let target_face = geometry
            .profile_faces(target)?
            .into_iter()
            .find(|face| face.frame.normal[2] < -0.8)
            .ok_or("target")?
            .face;
        let request = NewellSurfaceExtrude {
            profile: (source, face.face),
            boundary: crate::NewellBoundary::Edge { wire: 0, edge },
            direction: [0., 0., 1.],
            start: NewellSurfaceStart::Distance(0.),
            extent: NewellSurfaceExtent::UpTo(NewellFaceTarget {
                shape: target,
                face: target_face,
                clearance: 0.,
            }),
            sides: NewellSurfaceSides::One,
        };
        let sheet = geometry.extrude_surface(request.clone())?[0];
        assert!((geometry.surface_area(sheet)? - 9.).abs() < 1e-6);
        let mut symmetric = request.clone();
        symmetric.sides = NewellSurfaceSides::Symmetric;
        let sheet = geometry.extrude_surface(symmetric)?[0];
        assert!((geometry.surface_area(sheet)? - 18.).abs() < 1e-6);
        let mut second = request.clone();
        second.sides = NewellSurfaceSides::Second {
            extent: NewellSurfaceExtent::Blind(4.5),
            reverse: true,
        };
        let sheet = geometry.extrude_surface(second)?[0];
        assert!((geometry.surface_area(sheet)? - 9.25).abs() < 1e-6);
        let mut clearance = request;
        clearance.extent = NewellSurfaceExtent::UpTo(NewellFaceTarget {
            shape: target,
            face: target_face,
            clearance: 0.25,
        });
        let sheet = geometry.extrude_surface(clearance)?[0];
        assert!((geometry.surface_area(sheet)? - 8.5).abs() < 1e-6);
        Ok(())
    }
    #[test]
    fn surface_through_all_counts_uncapped_targets_and_skips_a_distant_miss() -> TestResult {
        let mut geometry = NewellGeometry::new();
        let source = geometry.box_shape([2., 3., 1.], [0.; 3])?;
        let face = geometry
            .profile_faces(source)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("profile")?;
        let edge = face.wires[0]
            .edges
            .iter()
            .find(|edge| edge.start[1].abs() < 1e-7 && edge.end[1].abs() < 1e-7)
            .ok_or("edge")?
            .edge;
        let target = geometry.box_shape([2., 3., 1.], [0., 0., 6.])?;
        let support = geometry.box_shape([2., 3., 1.], [0., 0., 11.])?;
        let support_face = geometry
            .profile_faces(support)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("sheet support")?
            .face;
        let sheet = geometry.sweep_surface(
            (support, support_face),
            crate::NewellBoundary::Face,
            [0., 0., 1.],
            0.,
            2.,
        )?[0];
        let miss = geometry.box_shape([1.; 3], [100.; 3])?;
        let result = geometry.extrude_surface(NewellSurfaceExtrude {
            profile: (source, face.face),
            boundary: crate::NewellBoundary::Edge { wire: 0, edge },
            direction: [0., 0., 1.],
            start: NewellSurfaceStart::Distance(0.),
            extent: NewellSurfaceExtent::ThroughAll(vec![target, sheet, miss]),
            sides: NewellSurfaceSides::One,
        })?[0];
        assert!((geometry.surface_area(result)? - 2. * 13.000001).abs() < 1e-6);
        let end = geometry.through_all_station(
            (source, face.face),
            [0., 0., 1.],
            0.,
            &[target, sheet, miss],
        )?;
        assert!((end - 13.000001).abs() < 1e-8);
        Ok(())
    }
    #[test]
    fn cubic_profile_uses_the_generic_finite_tilted_target_path() -> TestResult {
        let solid = crate::NewellProfile::closed_cubic([[0., 0.], [2., 3.], [4., 2.], [6., 0.]])?
            .extrude([0., 0., 1.], 1.)?;
        let mut geometry = NewellGeometry::new();
        geometry.topology = solid.topology;
        let source = geometry.insert(solid.solid)?;
        let target = geometry.box_shape([20., 20., 1.], [-10., -10., 0.])?;
        let target_solid = geometry.solid(target)?;
        let mut transform = Mat4::rotation_y(-0.1_f64.atan());
        transform.0[2][3] = 5.;
        brepkit_operations::transform::transform_solid(
            &mut geometry.topology,
            target_solid,
            &transform,
        )?;
        let profile = geometry
            .profile_faces(source)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("profile")?
            .face;
        let target_face = geometry
            .profile_faces(target)?
            .into_iter()
            .find(|face| face.frame.normal[2] < -0.9)
            .ok_or("target")?
            .face;
        let result = geometry.extrude_to_face(
            (source, profile),
            [0., 0., 1.],
            0.,
            (target, target_face),
            0.,
        )?;
        let actual = geometry.volume(result)?;
        assert!((actual - 32.16).abs() < 1e-6, "actual {actual}");
        assert_eq!(geometry.face_count(result)?, 4);
        let through = geometry.box_shape([20., 20., 1.], [-10., -10., 5.])?;
        let end = geometry.through_all_station((source, profile), [0., 0., 1.], 0., &[through])?;
        assert!((end - 5.000001).abs() < 1e-8, "{end}");
        Ok(())
    }
    #[test]
    fn circular_up_to_keeps_the_exact_curved_boundary_and_tilted_cap() -> TestResult {
        let mut geometry = NewellGeometry::new();
        let cylinder =
            brepkit_operations::primitives::make_cylinder(&mut geometry.topology, 2., 1.)?;
        let source = geometry.insert(cylinder)?;
        let target = geometry.box_shape([10., 10., 1.], [-5., -5., 0.])?;
        let target_solid = geometry.solid(target)?;
        let mut transform = Mat4::rotation_y(0.2);
        transform.0[2][3] = 6.;
        brepkit_operations::transform::transform_solid(
            &mut geometry.topology,
            target_solid,
            &transform,
        )?;
        let profile = geometry
            .profile_faces(source)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("profile")?
            .face;
        let target_face = geometry
            .profile_faces(target)?
            .into_iter()
            .find(|face| face.frame.normal[2] < -0.9)
            .ok_or("target")?
            .face;
        let result = geometry.extrude_to_face(
            (source, profile),
            [0., 0., 1.],
            0.,
            (target, target_face),
            0.,
        )?;
        assert!((geometry.volume(result)? - 20. * std::f64::consts::PI).abs() < 1e-6);
        Ok(())
    }
    #[test]
    fn tilted_target_keeps_its_plane_and_rejects_an_interior_hole() -> TestResult {
        let mut geometry = NewellGeometry::new();
        let source = geometry.box_shape([2., 3., 1.], [0.; 3])?;
        let target = geometry.box_shape([10., 10., 1.], [-4., -4., 0.])?;
        let target_solid = geometry.solid(target)?;
        let angle = 0.2_f64;
        let mut transform = Mat4::rotation_y(angle);
        transform.0[2][3] = 6.;
        brepkit_operations::transform::transform_solid(
            &mut geometry.topology,
            target_solid,
            &transform,
        )?;
        let profile = geometry
            .profile_faces(source)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("profile")?
            .face;
        let target_face = geometry
            .profile_faces(target)?
            .into_iter()
            .find(|face| face.frame.normal[2] < -0.9)
            .ok_or("target")?
            .face;
        let result = geometry.extrude_to_face(
            (source, profile),
            [0., 0., 1.],
            0.,
            (target, target_face),
            0.,
        )?;
        assert!((geometry.volume(result)? - (30. - 6. * angle.tan())).abs() < 1e-6);
        let mesh = geometry.tessellate(result, 0.05)?;
        assert!(mesh.positions().iter().any(|point| {
            (f64::from(point[2]) - (6. - f64::from(point[0]) * angle.tan())).abs() < 1e-5
        }));
        // A hole entirely inside the projected source defeats vertex-only coverage tests.
        let outer = make_polygon_wire(
            &mut geometry.topology,
            &[
                Point3::new(-1., -1., 5.),
                Point3::new(3., -1., 5.),
                Point3::new(3., 4., 5.),
                Point3::new(-1., 4., 5.),
            ],
            1e-7,
        )?;
        let hole = make_polygon_wire(
            &mut geometry.topology,
            &[
                Point3::new(0.75, 1., 5.),
                Point3::new(0.75, 2., 5.),
                Point3::new(1.25, 2., 5.),
                Point3::new(1.25, 1., 5.),
            ],
            1e-7,
        )?;
        let face = geometry.topology.add_face(Face::new(
            outer,
            vec![hole],
            FaceSurface::Plane {
                normal: Vec3::new(0., 0., 1.),
                d: 5.,
            },
        ));
        let solid = brepkit_operations::extrude::extrude(
            &mut geometry.topology,
            face,
            Vec3::new(0., 0., 1.),
            1.,
        )?;
        let holed = geometry.insert(solid)?;
        let face = geometry
            .profile_faces(holed)?
            .into_iter()
            .find(|face| face.frame.normal[2] < -0.9)
            .ok_or("holed target")?
            .face;
        assert!(
            geometry
                .extrude_to_face((source, profile), [0., 0., 1.], 0., (holed, face), 0.)
                .is_err()
        );
        Ok(())
    }
}
