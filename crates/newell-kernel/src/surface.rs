// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Uncapped exact polynomial sheets and same-support domain operations.
use crate::{NewellKernelError, operation_error};
use brepkit_math::{
    nurbs::{curve::NurbsCurve, surface::NurbsSurface},
    quadrature::gauss_legendre_points,
    traits::ParametricSurface,
    vec::Point3,
};
use brepkit_topology::{
    FaceId, Topology,
    edge::{Edge, EdgeCurve},
    face::{Face, FaceSurface},
    vertex::Vertex,
    wire::{OrientedEdge, Wire},
};

/// Set operation on an existing target and a generated tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewellBooleanMode {
    Add,
    Remove,
}
/// An uncapped B-Rep sheet. Native handles remain entirely inside the ACL.
#[derive(Debug)]
pub struct NewellSurface {
    topology: Topology,
    face: FaceId,
    source: [[f64; 2]; 4],
    interval: [f64; 2],
}
impl NewellSurface {
    /// Sweeps an exact cubic boundary along Z over a positive station interval.
    ///
    /// # Errors
    /// Rejects nonfinite/degenerate data and kernel construction errors.
    pub fn sweep_cubic(
        source: [[f64; 2]; 4],
        start: f64,
        end: f64,
    ) -> Result<Self, NewellKernelError> {
        if !source.iter().flatten().all(|v| v.is_finite())
            || !start.is_finite()
            || !end.is_finite()
            || end - start <= 1e-7
        {
            return Err(NewellKernelError::InvalidParameter {
                parameter: "surface sweep",
            });
        }
        let mut topology = Topology::new();
        let lo = source.map(|p| Point3::new(p[0], p[1], start));
        let hi = source.map(|p| Point3::new(p[0], p[1], end));
        let (face, _, _) = cubic_band(&mut topology, lo, hi, false)?;
        Ok(Self {
            topology,
            face,
            source,
            interval: [start, end],
        })
    }
    /// Integrates the actual ruled surface Jacobian, independently of display tessellation.
    ///
    /// # Errors
    /// Propagates invalid topology and nonfinite measures.
    pub fn area(&self) -> Result<f64, NewellKernelError> {
        let f = self.topology.face(self.face).map_err(operation_error)?;
        let FaceSurface::Nurbs(surface) = f.surface() else {
            return Err(NewellKernelError::Operation {
                message: "expected ruled surface".into(),
            });
        };
        let (a, b) = surface.domain_u();
        let (c, d) = surface.domain_v();
        let mut area = 0.;
        // Subdivide the nonlinear speed integral; Gaussian integration never changes the B-Rep.
        for part in 0..32 {
            let lo = a + (b - a) * f64::from(part) / 32.;
            let hi = a + (b - a) * f64::from(part + 1) / 32.;
            for x in gauss_legendre_points(16) {
                let u = (lo + hi) * 0.5 + (hi - lo) * 0.5 * x.x;
                for y in gauss_legendre_points(4) {
                    let v = (c + d) * 0.5 + (d - c) * 0.5 * y.x;
                    area += ParametricSurface::partial_u(surface, u, v)
                        .cross(ParametricSurface::partial_v(surface, u, v))
                        .length()
                        * x.w
                        * y.w
                        * (hi - lo)
                        * (d - c)
                        * 0.25;
                }
            }
        }
        if !area.is_finite() || area <= 0. {
            return Err(NewellKernelError::NonFiniteMesh);
        }
        Ok(area)
    }
    /// Combines identical generating curves with proven common Z support.
    /// Returns separate connected sheets when the exact station result is disconnected.
    ///
    /// # Errors
    /// Rejects different supports and operations which change no target.
    pub fn combine(
        &self,
        tool: &Self,
        mode: NewellBooleanMode,
    ) -> Result<Vec<Self>, NewellKernelError> {
        if self.source != tool.source {
            return Err(NewellKernelError::Operation {
                message: "sheet supports are not identical".into(),
            });
        }
        let [a, b] = self.interval;
        let [c, d] = tool.interval;
        let overlap = b.min(d) - a.max(c);
        if overlap <= 1e-7 {
            return Err(NewellKernelError::Operation {
                message: "no target intersection".into(),
            });
        }
        let spans = match mode {
            NewellBooleanMode::Add => vec![[a.min(c), b.max(d)]],
            NewellBooleanMode::Remove => {
                let mut spans = Vec::new();
                if c > a + 1e-7 {
                    spans.push([a, c.min(b)]);
                }
                if d < b - 1e-7 {
                    spans.push([d.max(a), b]);
                }
                spans
            }
        };
        spans
            .into_iter()
            .map(|[lo, hi]| Self::sweep_cubic(self.source, lo, hi))
            .collect()
    }
    /// Tessellates the actual face to verify the exact boundary is usable for display.
    ///
    /// # Errors
    /// Propagates invalid deflection or kernel tessellation failure.
    pub fn triangle_count(&self, deflection: f64) -> Result<usize, NewellKernelError> {
        if !deflection.is_finite() || deflection <= 0. {
            return Err(NewellKernelError::InvalidParameter {
                parameter: "deflection",
            });
        }
        let mesh =
            brepkit_operations::tessellate::tessellate(&self.topology, self.face, deflection)
                .map_err(operation_error)?;
        Ok(mesh.indices.len() / 3)
    }
    /// Number of B-Rep boundary edges, independent of tessellation.
    ///
    /// # Errors
    /// Propagates invalid topology.
    pub fn boundary_edge_count(&self) -> Result<usize, NewellKernelError> {
        Ok(self
            .topology
            .wire(
                self.topology
                    .face(self.face)
                    .map_err(operation_error)?
                    .outer_wire(),
            )
            .map_err(operation_error)?
            .edges()
            .len())
    }
}

/// Shared exact band assembly. The returned two cross edges close the boundary by endpoint identity.
pub(crate) fn cubic_band(
    t: &mut Topology,
    lo: [Point3; 4],
    hi: [Point3; 4],
    reverse_surface: bool,
) -> Result<
    (
        FaceId,
        [brepkit_topology::EdgeId; 2],
        [brepkit_topology::EdgeId; 2],
    ),
    NewellKernelError,
> {
    let a = t.add_vertex(Vertex::new(lo[0], 1e-7));
    let b = t.add_vertex(Vertex::new(lo[3], 1e-7));
    let c = t.add_vertex(Vertex::new(hi[0], 1e-7));
    let d = t.add_vertex(Vertex::new(hi[3], 1e-7));
    let curve = |p: [Point3; 4]| {
        NurbsCurve::new(
            3,
            vec![0., 0., 0., 0., 1., 1., 1., 1.],
            p.to_vec(),
            vec![1.; 4],
        )
        .map_err(operation_error)
    };
    let bottom = t.add_edge(Edge::new(a, b, EdgeCurve::NurbsCurve(curve(lo)?)));
    let top = t.add_edge(Edge::new(c, d, EdgeCurve::NurbsCurve(curve(hi)?)));
    let left = t.add_edge(Edge::new(a, c, EdgeCurve::Line));
    let right = t.add_edge(Edge::new(b, d, EdgeCurve::Line));
    let (grid, edges) = if reverse_surface {
        (
            lo.into_iter()
                .zip(hi)
                .rev()
                .map(|(p, q)| vec![p, q])
                .collect(),
            vec![
                OrientedEdge::new(bottom, false),
                OrientedEdge::new(left, true),
                OrientedEdge::new(top, true),
                OrientedEdge::new(right, false),
            ],
        )
    } else {
        (
            lo.into_iter().zip(hi).map(|(p, q)| vec![p, q]).collect(),
            vec![
                OrientedEdge::new(bottom, true),
                OrientedEdge::new(right, true),
                OrientedEdge::new(top, false),
                OrientedEdge::new(left, false),
            ],
        )
    };
    let surface = NurbsSurface::new(
        3,
        1,
        vec![0., 0., 0., 0., 1., 1., 1., 1.],
        vec![0., 0., 1., 1.],
        grid,
        vec![vec![1.; 2]; 4],
    )
    .map_err(operation_error)?;
    let w = t.add_wire(Wire::new(edges, true).map_err(operation_error)?);
    let face = t.add_face(Face::new(w, vec![], FaceSurface::Nurbs(surface)));
    Ok((face, [bottom, top], [left, right]))
}

/// Boundary occurrence within an actual planar supporting face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewellBoundary {
    Face,
    Wire(u32),
    Edge { wire: u32, edge: u32 },
}

/// A construction proof of one common generating boundary and sweep domain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SurfaceSweepRecipe {
    pub profile: (crate::NewellShapeKey, u32),
    pub boundary: NewellBoundary,
    pub direction: [f64; 3],
    pub start: f64,
    pub end: f64,
    pub start_gradient: [f64; 3],
    pub end_gradient: [f64; 3],
    pub reversed: bool,
}
impl SurfaceSweepRecipe {
    pub(crate) fn same_support(self, other: Self) -> bool {
        self.profile == other.profile
            && self.boundary == other.boundary
            && self.direction == other.direction
    }
}
/// Exact pieces of a generated support, retained across subsequent booleans.
#[derive(Debug, Clone)]
pub(crate) struct SurfaceDomainRecipe {
    pub support: SurfaceSweepRecipe,
    pub pieces: Vec<SweepDomainPiece>,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct SweepDomainPiece {
    pub curve: usize,
    /// Fraction of the original generating curve's parameter domain.
    pub interval: [f64; 2],
    pub low: SweepStation,
    pub high: SweepStation,
}
/// Exact rational conic representation over its oriented edge span.
pub(crate) fn boundary_curve(
    curve: &EdgeCurve,
    a: Point3,
    b: Point3,
    forward: bool,
) -> Result<Option<NurbsCurve>, NewellKernelError> {
    let (mut lo, mut hi) = curve.domain_with_endpoints(a, b);
    if !forward {
        std::mem::swap(&mut lo, &mut hi);
    }
    let conic = match curve {
        EdgeCurve::Line => return Ok(None),
        EdgeCurve::NurbsCurve(curve) => {
            let (d0, d1) = curve.domain();
            let mut span = curve.clone();
            if lo.min(hi) > d0 {
                span = brepkit_math::nurbs::knot_ops::curve_split(&span, lo.min(hi))
                    .map_err(operation_error)?
                    .1;
            }
            if lo.max(hi) < d1 {
                span = brepkit_math::nurbs::knot_ops::curve_split(&span, lo.max(hi))
                    .map_err(operation_error)?
                    .0;
            }
            if hi < lo {
                let EdgeCurve::NurbsCurve(reversed) = EdgeCurve::NurbsCurve(span)
                    .reversed()
                    .map_err(operation_error)?
                else {
                    return Err(operation_error("curve reversal"));
                };
                span = reversed;
            }
            return Ok(Some(span));
        }
        EdgeCurve::Circle(c) => (c.center(), c.u_axis(), c.v_axis(), c.radius(), c.radius()),
        EdgeCurve::Ellipse(c) => (
            c.center(),
            c.u_axis(),
            c.v_axis(),
            c.semi_major(),
            c.semi_minor(),
        ),
    };
    let (center, u, v, ra, rb) = conic;
    let span = hi - lo;
    let pieces = if span.abs() > 3. * std::f64::consts::FRAC_PI_2 + 1e-12 {
        4
    } else if span.abs() > std::f64::consts::PI + 1e-12 {
        3
    } else if span.abs() > std::f64::consts::FRAC_PI_2 + 1e-12 {
        2
    } else {
        1
    };
    let mut points = Vec::new();
    let mut weights = Vec::new();
    let mut knots = vec![0.; 3];
    let point = |angle: f64| center + u * (ra * angle.cos()) + v * (rb * angle.sin());
    points.push(point(lo));
    weights.push(1.);
    for piece in 0..pieces {
        let a = lo + span * f64::from(piece) / f64::from(pieces);
        let b = lo + span * f64::from(piece + 1) / f64::from(pieces);
        let mid = (a + b) * 0.5;
        let weight = ((b - a) * 0.5).cos();
        points.push(center + (u * (ra * mid.cos()) + v * (rb * mid.sin())) * (1. / weight));
        weights.push(weight);
        points.push(point(b));
        weights.push(1.);
        if piece + 1 < pieces {
            let knot = f64::from(piece + 1) / f64::from(pieces);
            knots.extend([knot, knot]);
        }
    }
    knots.extend([1.; 3]);
    NurbsCurve::new(2, knots, points, weights)
        .map(Some)
        .map_err(operation_error)
}

/// Affine endpoint maps preserve exact rational boundary weights and knots.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SweepStation {
    pub gradient: brepkit_math::vec::Vec3,
    pub constant: f64,
}
impl SweepStation {
    pub fn fixed(constant: f64) -> Self {
        Self {
            gradient: brepkit_math::vec::Vec3::new(0., 0., 0.),
            constant,
        }
    }
    pub fn value(self, point: Point3) -> f64 {
        self.gradient.dot(brepkit_math::vec::Vec3::new(
            point.x(),
            point.y(),
            point.z(),
        )) + self.constant
    }
    pub fn point(self, point: Point3, direction: brepkit_math::vec::Vec3) -> Point3 {
        point + direction * self.value(point)
    }
}
impl crate::NewellGeometry {
    /// Builds uncapped exact side faces directly from the selected supporting occurrence.
    /// A face sweeps all its boundaries; each connected wire produces a separate sheet.
    ///
    /// # Errors
    /// Rejects invalid occurrences, tangent directions, collapsed intervals and topology errors.
    pub fn sweep_surface(
        &mut self,
        profile: (crate::NewellShapeKey, u32),
        boundary: NewellBoundary,
        direction: [f64; 3],
        start: f64,
        end: f64,
    ) -> Result<Vec<crate::NewellShapeKey>, NewellKernelError> {
        if !start.is_finite() || !end.is_finite() || end - start <= 1e-7 {
            return Err(operation_error("invalid surface sweep interval"));
        }
        self.sweep_surface_between(
            profile,
            boundary,
            direction,
            SweepStation::fixed(start),
            SweepStation::fixed(end),
        )
    }
    pub(crate) fn sweep_surface_between(
        &mut self,
        profile: (crate::NewellShapeKey, u32),
        boundary: NewellBoundary,
        direction: [f64; 3],
        start: SweepStation,
        end: SweepStation,
    ) -> Result<Vec<crate::NewellShapeKey>, NewellKernelError> {
        use brepkit_math::vec::Vec3;
        use std::collections::BTreeMap;
        let direction = Vec3::new(direction[0], direction[1], direction[2])
            .normalize()
            .map_err(operation_error)?;
        let faces = self.shape_faces(profile.0)?;
        let face = *faces
            .get(usize::try_from(profile.1).map_err(operation_error)?)
            .ok_or_else(|| operation_error("missing supporting face"))?;
        let frame = crate::profile::supporting_frame(&self.topology, face)?;
        let normal = Vec3::new(frame.normal[0], frame.normal[1], frame.normal[2]);
        if normal.dot(direction).abs() <= 1e-12 {
            return Err(operation_error("tangential Along"));
        }
        let face = self.topology.face(face).map_err(operation_error)?;
        let reversed = face.is_reversed();
        let wires: Vec<_> = std::iter::once(face.outer_wire())
            .chain(face.inner_wires().iter().copied())
            .collect();
        let mut results = Vec::new();
        for (ordinal, wire) in wires.into_iter().enumerate() {
            let ordinal = u32::try_from(ordinal).map_err(operation_error)?;
            if match boundary {
                NewellBoundary::Face => false,
                NewellBoundary::Wire(wire) | NewellBoundary::Edge { wire, .. } => wire != ordinal,
            } {
                continue;
            }
            let mut edges: Vec<_> = self
                .topology
                .wire(wire)
                .map_err(operation_error)?
                .edges()
                .iter()
                .copied()
                .enumerate()
                .collect();
            if reversed {
                edges.reverse();
                for (_, edge) in &mut edges {
                    *edge = OrientedEdge::new(edge.edge(), !edge.is_forward());
                }
            }
            let mut side_faces = Vec::new();
            let mut vertices = BTreeMap::new();
            for (ordinal, occurrence) in edges {
                if let NewellBoundary::Edge { edge, .. } = boundary
                    && usize::try_from(edge).map_err(operation_error)? != ordinal
                {
                    continue;
                }
                let edge = self
                    .topology
                    .edge(occurrence.edge())
                    .map_err(operation_error)?;
                let (a, b) = (edge.start(), edge.end());
                let curve = edge.curve().clone();
                let pa = self.topology.vertex(a).map_err(operation_error)?.point();
                let pb = self.topology.vertex(b).map_err(operation_error)?.point();
                let native = boundary_curve(&curve, pa, pb, occurrence.is_forward())?;
                let (a, b, pa, pb) = if occurrence.is_forward() {
                    (a, b, pa, pb)
                } else {
                    (b, a, pb, pa)
                };
                for (vertex, point) in [(a, pa), (b, pb)] {
                    vertices.entry(vertex).or_insert_with(|| {
                        let low = self
                            .topology
                            .add_vertex(Vertex::new(start.point(point, direction), 1e-7));
                        let high = self
                            .topology
                            .add_vertex(Vertex::new(end.point(point, direction), 1e-7));
                        let edge = self
                            .topology
                            .add_edge(Edge::new(low, high, EdgeCurve::Line));
                        (low, high, edge)
                    });
                }
                let (al, ah, left) = *vertices
                    .get(&a)
                    .ok_or_else(|| operation_error("surface vertex"))?;
                let (bl, bh, right) = *vertices
                    .get(&b)
                    .ok_or_else(|| operation_error("surface vertex"))?;
                let (low_curve, high_curve, surface) = if let Some(curve) = native {
                    let low: Vec<_> = curve
                        .control_points()
                        .iter()
                        .map(|p| start.point(*p, direction))
                        .collect();
                    let high: Vec<_> = curve
                        .control_points()
                        .iter()
                        .map(|p| end.point(*p, direction))
                        .collect();
                    let surface = NurbsSurface::new(
                        curve.degree(),
                        1,
                        curve.knots().to_vec(),
                        vec![0., 0., 1., 1.],
                        low.iter().zip(&high).map(|(a, b)| vec![*a, *b]).collect(),
                        curve.weights().iter().map(|w| vec![*w, *w]).collect(),
                    )
                    .map_err(operation_error)?;
                    let low = NurbsCurve::new(
                        curve.degree(),
                        curve.knots().to_vec(),
                        low,
                        curve.weights().to_vec(),
                    )
                    .map_err(operation_error)?;
                    let high = NurbsCurve::new(
                        curve.degree(),
                        curve.knots().to_vec(),
                        high,
                        curve.weights().to_vec(),
                    )
                    .map_err(operation_error)?;
                    (
                        EdgeCurve::NurbsCurve(low),
                        EdgeCurve::NurbsCurve(high),
                        FaceSurface::Nurbs(surface),
                    )
                } else {
                    let normal = (pb - pa)
                        .cross(direction)
                        .normalize()
                        .map_err(operation_error)?;
                    let point = start.point(pa, direction);
                    (
                        EdgeCurve::Line,
                        EdgeCurve::Line,
                        FaceSurface::Plane {
                            normal,
                            d: normal.dot(Vec3::new(point.x(), point.y(), point.z())),
                        },
                    )
                };
                let low = self.topology.add_edge(Edge::new(al, bl, low_curve));
                let high = self.topology.add_edge(Edge::new(ah, bh, high_curve));
                let wire = self.topology.add_wire(
                    Wire::new(
                        vec![
                            OrientedEdge::new(low, true),
                            OrientedEdge::new(right, true),
                            OrientedEdge::new(high, false),
                            OrientedEdge::new(left, false),
                        ],
                        true,
                    )
                    .map_err(operation_error)?,
                );
                side_faces.push(self.topology.add_face(Face::new(wire, vec![], surface)));
            }
            if !side_faces.is_empty() {
                let key = self.insert_surface(side_faces)?;
                self.surface_wire_keys.insert(key, ordinal);
                let boundary = match boundary {
                    NewellBoundary::Face => NewellBoundary::Wire(ordinal),
                    other => other,
                };
                let mut d = [direction.x(), direction.y(), direction.z()];
                let mut low = start.constant;
                let mut high = end.constant;
                let mut lower = start.gradient;
                let mut upper = end.gradient;
                let reversed = d
                    .iter()
                    .find(|value| value.abs() > 1e-12)
                    .is_some_and(|value| *value < 0.);
                if reversed {
                    d = d.map(|v| -v);
                    low = -end.constant;
                    high = -start.constant;
                    lower = end.gradient * -1.;
                    upper = start.gradient * -1.;
                }
                self.surface_recipes.insert(
                    key,
                    SurfaceSweepRecipe {
                        profile,
                        boundary,
                        direction: d,
                        start: low,
                        end: high,
                        start_gradient: [lower.x(), lower.y(), lower.z()],
                        end_gradient: [upper.x(), upper.y(), upper.z()],
                        reversed,
                    },
                );
                results.push(key);
            }
        }
        if results.is_empty() {
            return Err(operation_error("missing selected boundary"));
        }
        self.order_components(results)
    }
}

impl crate::NewellGeometry {
    pub(crate) fn sweep_curve_piece(
        &mut self,
        curve: &NurbsCurve,
        d: brepkit_math::vec::Vec3,
        low: SweepStation,
        high: SweepStation,
    ) -> Result<FaceId, NewellKernelError> {
        let lower: Vec<_> = curve
            .control_points()
            .iter()
            .map(|p| low.point(*p, d))
            .collect();
        let upper: Vec<_> = curve
            .control_points()
            .iter()
            .map(|p| high.point(*p, d))
            .collect();
        let low_curve = NurbsCurve::new(
            curve.degree(),
            curve.knots().to_vec(),
            lower.clone(),
            curve.weights().to_vec(),
        )
        .map_err(operation_error)?;
        let high_curve = NurbsCurve::new(
            curve.degree(),
            curve.knots().to_vec(),
            upper.clone(),
            curve.weights().to_vec(),
        )
        .map_err(operation_error)?;
        let (lo, hi) = curve.domain();
        let a = curve.evaluate(lo);
        let b = curve.evaluate(hi);
        let mut vertices = Vec::new();
        for point in [
            low.point(a, d),
            low.point(b, d),
            high.point(a, d),
            high.point(b, d),
        ] {
            let same = vertices.iter().copied().find(|id| {
                self.topology
                    .vertex(*id)
                    .is_ok_and(|v| (v.point() - point).length() <= 1e-7)
            });
            vertices
                .push(same.unwrap_or_else(|| self.topology.add_vertex(Vertex::new(point, 1e-7))));
        }
        let straight = curve.degree() == 1 && curve.control_points().len() == 2;
        let bottom = self.topology.add_edge(Edge::new(
            vertices[0],
            vertices[1],
            if straight {
                EdgeCurve::Line
            } else {
                EdgeCurve::NurbsCurve(low_curve)
            },
        ));
        let top = self.topology.add_edge(Edge::new(
            vertices[2],
            vertices[3],
            if straight {
                EdgeCurve::Line
            } else {
                EdgeCurve::NurbsCurve(high_curve)
            },
        ));
        let mut edges = vec![OrientedEdge::new(bottom, true)];
        if vertices[1] != vertices[3] {
            let right =
                self.topology
                    .add_edge(Edge::new(vertices[1], vertices[3], EdgeCurve::Line));
            edges.push(OrientedEdge::new(right, true));
        }
        edges.push(OrientedEdge::new(top, false));
        if vertices[0] != vertices[2] {
            let left = self
                .topology
                .add_edge(Edge::new(vertices[0], vertices[2], EdgeCurve::Line));
            edges.push(OrientedEdge::new(left, false));
        }
        let wire = self
            .topology
            .add_wire(Wire::new(edges, true).map_err(operation_error)?);
        let surface = if straight {
            let normal = (b - a).cross(d).normalize().map_err(operation_error)?;
            let point = low.point(a, d);
            FaceSurface::Plane {
                normal,
                d: normal.dot(brepkit_math::vec::Vec3::new(
                    point.x(),
                    point.y(),
                    point.z(),
                )),
            }
        } else {
            FaceSurface::Nurbs(
                NurbsSurface::new(
                    curve.degree(),
                    1,
                    curve.knots().to_vec(),
                    vec![0., 0., 1., 1.],
                    lower
                        .iter()
                        .zip(&upper)
                        .map(|(a, b)| vec![*a, *b])
                        .collect(),
                    curve.weights().iter().map(|w| vec![*w, *w]).collect(),
                )
                .map_err(operation_error)?,
            )
        };
        Ok(self.topology.add_face(Face::new(wire, Vec::new(), surface)))
    }
    /// Welds the complete shared ruling of adjacent exact curve patches. Only
    /// line seams are merged; all rational boundary curves remain unchanged.
    pub(crate) fn weld_sweep_pieces(
        &mut self,
        faces: Vec<FaceId>,
        wire_key: u32,
        artificial: &[[Point3; 2]],
    ) -> Result<crate::NewellShapeKey, NewellKernelError> {
        use std::collections::{BTreeMap, BTreeSet};
        let mut vertices = Vec::<(Point3, brepkit_topology::VertexId)>::new();
        for id in &faces {
            self.check_cancelled()?;
            let face = self.topology.face(*id).map_err(operation_error)?;
            for wire in std::iter::once(face.outer_wire()).chain(face.inner_wires().iter().copied())
            {
                for occurrence in self.topology.wire(wire).map_err(operation_error)?.edges() {
                    let edge = self
                        .topology
                        .edge(occurrence.edge())
                        .map_err(operation_error)?;
                    for id in [edge.start(), edge.end()] {
                        let point = self.topology.vertex(id).map_err(operation_error)?.point();
                        if !vertices.iter().any(|(p, _)| (*p - point).length() <= 1e-7) {
                            vertices.push((point, id));
                        }
                    }
                }
            }
        }
        let mut lines = BTreeMap::new();
        let mut rebuilt = Vec::new();
        let mut uses = BTreeMap::<_, BTreeSet<usize>>::new();
        for (ordinal, id) in faces.into_iter().enumerate() {
            self.check_cancelled()?;
            let face = self.topology.face(id).map_err(operation_error)?.clone();
            let mut wires = Vec::new();
            for wire in std::iter::once(face.outer_wire()).chain(face.inner_wires().iter().copied())
            {
                let edges = self
                    .topology
                    .wire(wire)
                    .map_err(operation_error)?
                    .edges()
                    .to_vec();
                let mut occurrences = Vec::new();
                for occurrence in edges {
                    let edge = self
                        .topology
                        .edge(occurrence.edge())
                        .map_err(operation_error)?
                        .clone();
                    let mut ids = Vec::new();
                    for vertex in [edge.start(), edge.end()] {
                        let point = self
                            .topology
                            .vertex(vertex)
                            .map_err(operation_error)?
                            .point();
                        let index = if let Some(index) = vertices
                            .iter()
                            .position(|(p, _)| (*p - point).length() <= 1e-7)
                        {
                            index
                        } else {
                            let vertex = self.topology.add_vertex(Vertex::new(point, 1e-7));
                            vertices.push((point, vertex));
                            vertices.len() - 1
                        };
                        ids.push(index);
                    }
                    if matches!(edge.curve(), EdgeCurve::Line) {
                        let a = vertices[ids[0]].0;
                        let b = vertices[ids[1]].0;
                        let delta = b - a;
                        let square = delta.length_squared();
                        if square <= 1e-14 {
                            return Err(operation_error("collapsed sweep seam"));
                        }
                        let mut split: Vec<_> = vertices
                            .iter()
                            .enumerate()
                            .filter_map(|(i, (p, _))| {
                                let t = (*p - a).dot(delta) / square;
                                ((-1e-10..=1. + 1e-10).contains(&t)
                                    && (*p - (a + delta * t)).length() <= 1e-7)
                                    .then_some((t, i))
                            })
                            .collect();
                        split.sort_by(|a, b| a.0.total_cmp(&b.0));
                        if !occurrence.is_forward() {
                            split.reverse();
                        }
                        for ends in split.windows(2) {
                            let (a, b) = (ends[0].1, ends[1].1);
                            if a == b {
                                continue;
                            }
                            let pair = (a.min(b), a.max(b));
                            let id = *lines.entry(pair).or_insert_with(|| {
                                self.topology.add_edge(Edge::new(
                                    vertices[pair.0].1,
                                    vertices[pair.1].1,
                                    EdgeCurve::Line,
                                ))
                            });
                            uses.entry(id).or_default().insert(ordinal);
                            occurrences.push(OrientedEdge::new(id, a < b));
                        }
                    } else {
                        let id = self.topology.add_edge(Edge::new(
                            vertices[ids[0]].1,
                            vertices[ids[1]].1,
                            edge.curve().clone(),
                        ));
                        uses.entry(id).or_default().insert(ordinal);
                        occurrences.push(OrientedEdge::new(id, occurrence.is_forward()));
                    }
                }
                wires.push(
                    self.topology
                        .add_wire(Wire::new(occurrences, true).map_err(operation_error)?),
                );
            }
            let mut new = Face::new(wires.remove(0), wires, face.surface().clone());
            new.set_reversed(face.is_reversed());
            rebuilt.push(self.topology.add_face(new));
        }
        if uses.values().any(|faces| faces.len() > 2) {
            return Err(operation_error("nonmanifold sweep seam"));
        }
        let mut seams = BTreeSet::new();
        for (pair, id) in &lines {
            if uses.get(id).is_some_and(|faces| faces.len() == 2)
                && artificial.iter().any(|ends| {
                    let (a, b) = (vertices[pair.0].0, vertices[pair.1].0);
                    let delta = ends[1] - ends[0];
                    let square = delta.length_squared();
                    square > 1e-14
                        && [a, b].iter().all(|p| {
                            let t = (*p - ends[0]).dot(delta) / square;
                            (-1e-10..=1. + 1e-10).contains(&t)
                                && (*p - (ends[0] + delta * t)).length() <= 1e-7
                        })
                })
            {
                seams.insert(*id);
            }
        }
        let key = self.insert_surface(rebuilt)?;
        self.surface_wire_keys.insert(key, wire_key);
        self.surface_seams.insert(key, seams);
        Ok(key)
    }
}

impl crate::NewellGeometry {
    /// Whether an arena identity denotes uncapped faces rather than a closed body.
    #[must_use]
    pub fn is_surface(&self, key: crate::NewellShapeKey) -> bool {
        self.surfaces.contains_key(&key)
    }
    /// Measures the actual full ruled patches produced by surface sweep/domain construction.
    ///
    /// # Errors
    /// Rejects missing sheets, nonfinite measures and native topology errors.
    pub fn surface_area(&self, key: crate::NewellShapeKey) -> Result<f64, NewellKernelError> {
        let faces = self
            .surfaces
            .get(&key)
            .ok_or_else(|| operation_error("missing surface"))?;
        let mut area = 0.;
        for id in faces {
            self.check_cancelled()?;
            let face = self.topology.face(*id).map_err(operation_error)?;
            match face.surface() {
                FaceSurface::Plane { .. } => {
                    area += brepkit_operations::measure::face_area(&self.topology, *id, 0.001)
                        .map_err(operation_error)?
                }
                FaceSurface::Nurbs(surface) => {
                    let (ulo, uhi) = surface.domain_u();
                    let (vlo, vhi) = surface.domain_v();
                    let mut breaks = vec![ulo, uhi];
                    breaks.extend(
                        surface
                            .knots_u()
                            .iter()
                            .copied()
                            .filter(|k| *k > ulo && *k < uhi),
                    );
                    breaks.sort_by(f64::total_cmp);
                    breaks.dedup();
                    for span in breaks.windows(2) {
                        for part in 0..32 {
                            let lo = span[0] + (span[1] - span[0]) * f64::from(part) / 32.;
                            let hi = span[0] + (span[1] - span[0]) * f64::from(part + 1) / 32.;
                            for u in gauss_legendre_points(16) {
                                for v in gauss_legendre_points(4) {
                                    let ut = (lo + hi) * 0.5 + (hi - lo) * 0.5 * u.x;
                                    let vt = (vlo + vhi) * 0.5 + (vhi - vlo) * 0.5 * v.x;
                                    area += surface
                                        .partial_u(ut, vt)
                                        .cross(surface.partial_v(ut, vt))
                                        .length()
                                        * u.w
                                        * v.w
                                        * (hi - lo)
                                        * (vhi - vlo)
                                        * 0.25;
                                }
                            }
                        }
                    }
                }
                _ => return Err(operation_error("unexpected generated sheet support")),
            }
        }
        if !area.is_finite() || area <= 0. {
            return Err(operation_error("invalid surface area"));
        }
        Ok(area)
    }
    pub(crate) fn surface_mesh(
        &self,
        key: crate::NewellShapeKey,
        deflection: f64,
    ) -> Result<crate::NewellMesh, NewellKernelError> {
        use std::collections::BTreeMap;
        if !deflection.is_finite() || deflection <= 0. {
            return Err(operation_error("invalid deflection"));
        }
        let faces = self
            .surfaces
            .get(&key)
            .ok_or_else(|| operation_error("missing surface"))?;
        let mut meshes = Vec::new();
        let mut edges: BTreeMap<_, Vec<FaceId>> = BTreeMap::new();
        for face in faces {
            self.check_cancelled()?;
            let mesh =
                brepkit_operations::tessellate::tessellate(&self.topology, *face, deflection)
                    .map_err(operation_error)?;
            let count = u32::try_from(mesh.indices.len()).map_err(operation_error)?;
            let positions = mesh
                .positions
                .iter()
                .map(crate::point_to_f32)
                .collect::<Result<Vec<_>, _>>()?;
            crate::validate_mesh_precision(&mesh.positions, &positions, &mesh.indices)?;
            meshes.push(crate::NewellMesh {
                positions,
                normals: mesh
                    .normals
                    .iter()
                    .map(crate::vector_to_f32)
                    .collect::<Result<_, _>>()?,
                indices: mesh.indices,
                face_offsets: vec![0, count],
                feature_edges: crate::NewellEdgePolylines {
                    positions: Vec::new(),
                    offsets: vec![0],
                },
            });
            let value = self.topology.face(*face).map_err(operation_error)?;
            for wire in
                std::iter::once(value.outer_wire()).chain(value.inner_wires().iter().copied())
            {
                for edge in self.topology.wire(wire).map_err(operation_error)?.edges() {
                    edges.entry(edge.edge()).or_default().push(*face);
                }
            }
        }
        let refs: Vec<_> = meshes.iter().collect();
        let (mut mesh, _) = crate::NewellMesh::compose(&refs)?;
        let mut lines = crate::NewellEdgePolylines {
            positions: Vec::new(),
            offsets: vec![0],
        };
        for (id, uses) in edges {
            if self
                .surface_seams
                .get(&key)
                .is_some_and(|seams| seams.contains(&id))
            {
                continue;
            }
            if uses.len() == 2 && uses[0] == uses[1] {
                continue;
            }
            let edge = self.topology.edge(id).map_err(operation_error)?;
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
            let (lo, hi) = edge.curve().domain_with_endpoints(a, b);
            let evaluate = |t| edge.curve().evaluate_with_endpoints(t, a, b);
            let mut breaks = vec![lo, hi];
            if let EdgeCurve::NurbsCurve(curve) = edge.curve() {
                breaks.extend(
                    curve
                        .knots()
                        .iter()
                        .copied()
                        .filter(|k| *k > lo.min(hi) && *k < lo.max(hi)),
                );
            }
            breaks.sort_by(f64::total_cmp);
            breaks.dedup();
            if hi < lo {
                breaks.reverse();
            }
            lines.positions.push(crate::point_to_f32(&evaluate(lo))?);
            let mut visits = 0u32;
            for span in breaks.windows(2) {
                let mut work = vec![(span[0], span[1], evaluate(span[0]), evaluate(span[1]), 0u32)];
                while let Some((lo, hi, a, b, depth)) = work.pop() {
                    visits += 1;
                    if visits > 65536 || depth > 24 {
                        return Err(operation_error("edge display complexity"));
                    }
                    let flat = [0.25, 0.5, 0.75].into_iter().all(|fraction| {
                        (evaluate(lo + (hi - lo) * fraction) - (a + (b - a) * fraction)).length()
                            <= deflection
                    });
                    if flat {
                        lines.positions.push(crate::point_to_f32(&b)?);
                    } else {
                        let mid = lo + (hi - lo) * 0.5;
                        let point = evaluate(mid);
                        work.push((mid, hi, point, b, depth + 1));
                        work.push((lo, mid, a, point, depth + 1));
                    }
                }
            }
            lines
                .offsets
                .push(u32::try_from(lines.positions.len()).map_err(operation_error)?);
        }
        mesh.feature_edges = lines;
        Ok(mesh)
    }
}

#[cfg(test)]
mod boundary_sweep_tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    #[test]
    fn face_sweep_keeps_each_hole_wire_identity_and_connected_mesh() -> TestResult {
        use brepkit_topology::builder::make_polygon_wire;
        let mut geometry = crate::NewellGeometry::new();
        let outer = make_polygon_wire(
            &mut geometry.topology,
            &[
                Point3::new(0., 0., 0.),
                Point3::new(10., 0., 0.),
                Point3::new(10., 6., 0.),
                Point3::new(0., 6., 0.),
            ],
            1e-7,
        )?;
        let hole = make_polygon_wire(
            &mut geometry.topology,
            &[
                Point3::new(3., 2., 0.),
                Point3::new(3., 4., 0.),
                Point3::new(5., 4., 0.),
                Point3::new(5., 2., 0.),
            ],
            1e-7,
        )?;
        let face = geometry.topology.add_face(Face::new(
            outer,
            vec![hole],
            FaceSurface::Plane {
                normal: brepkit_math::vec::Vec3::new(0., 0., 1.),
                d: 0.,
            },
        ));
        let solid = brepkit_operations::extrude::extrude(
            &mut geometry.topology,
            face,
            brepkit_math::vec::Vec3::new(0., 0., 1.),
            1.,
        )?;
        let source = geometry.insert(solid)?;
        let support = geometry
            .profile_faces(source)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("top")?;
        for depth in [2., 3.5] {
            let outputs = geometry.sweep_surface(
                (source, support.face),
                NewellBoundary::Face,
                [0., 0., 1.],
                0.,
                depth,
            )?;
            assert_eq!(outputs.len(), 2);
            assert_eq!(
                outputs
                    .iter()
                    .map(|key| geometry.source_wire(*key))
                    .collect::<Vec<_>>(),
                vec![Some(0), Some(1)]
            );
            for (key, perimeter) in outputs.into_iter().zip([32., 8.]) {
                assert!(geometry.is_surface(key));
                assert_eq!(geometry.face_count(key)?, 4);
                assert!((geometry.surface_area(key)? - perimeter * depth).abs() < 1e-6);
                let faces = geometry.shape_faces(key)?;
                let mut incidence = std::collections::BTreeMap::new();
                for face in faces {
                    let wire = geometry.topology.face(face)?.outer_wire();
                    for occurrence in geometry.topology.wire(wire)?.edges() {
                        *incidence.entry(occurrence.edge()).or_insert(0) += 1;
                    }
                }
                assert_eq!(incidence.values().filter(|count| **count == 2).count(), 4);
                assert_eq!(incidence.values().filter(|count| **count == 1).count(), 8);
                let mesh = geometry.tessellate(key, 0.02)?;
                assert_eq!(mesh.face_count(), 4);
                assert!(!mesh.indices().is_empty());
                assert_eq!(mesh.face_offsets().len(), 5);
            }
        }
        Ok(())
    }
    #[test]
    fn common_support_proof_handles_another_source_knot_refinement_and_degree_elevation()
    -> TestResult {
        use brepkit_topology::edge::EdgeCurve;
        let solid = crate::NewellProfile::closed_cubic([[0., 0.], [2., 3.], [4., 2.], [6., 0.]])?
            .extrude([0., 0., 1.], 1.)?;
        let mut geometry = crate::NewellGeometry::new();
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
            .ok_or("curve")?
            .edge;
        let target = geometry.sweep_surface(
            (source, profile.face),
            NewellBoundary::Edge { wire: 0, edge },
            [0., 0., 1.],
            0.,
            3.,
        )?[0];
        let other = geometry.extrude_face(source, profile.face, Some([0., 0., 1.]), 2., 3.)?;
        let other_face = geometry
            .profile_faces(other)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("other profile")?;
        let other_edge = other_face.wires[0]
            .edges
            .iter()
            .find(|edge| !edge.straight)
            .ok_or("other curve")?
            .edge;
        let face = geometry.shape_faces(other)?[usize::try_from(other_face.face)?];
        let wire = geometry.topology.face(face)?.outer_wire();
        let native = geometry.topology.wire(wire)?.edges()[usize::try_from(other_edge)?].edge();
        let EdgeCurve::NurbsCurve(curve) = geometry.topology.edge(native)?.curve() else {
            return Err("lost curve".into());
        };
        let points = curve.control_points();
        assert_eq!(points.len(), 4);
        let elevated = vec![
            points[0],
            points[0] + (points[1] - points[0]) * 0.75,
            points[1] + (points[2] - points[1]) * 0.5,
            points[2] + (points[3] - points[2]) * 0.25,
            points[3],
        ];
        let curve = NurbsCurve::new(
            4,
            vec![2., 2., 2., 2., 2., 5., 5., 5., 5., 5.],
            elevated,
            vec![1.; 5],
        )?;
        let curve = brepkit_math::nurbs::knot_ops::curve_knot_insert(&curve, 3.5, 1)?;
        geometry
            .topology
            .edge_mut(native)?
            .set_curve(EdgeCurve::NurbsCurve(curve));
        let tool = geometry.sweep_surface(
            (other, other_face.face),
            NewellBoundary::Edge {
                wire: 0,
                edge: other_edge,
            },
            [0., 0., 1.],
            -2.,
            1.,
        )?[0];
        let mut sum = 0.;
        let n = 16384;
        for i in 0..=n {
            let t = f64::from(i) / f64::from(n);
            let q = 9. - 24. * t + 9. * t * t;
            sum += (if i == 0 || i == n {
                1.
            } else if i % 2 == 0 {
                2.
            } else {
                4.
            }) * (36. + q * q).sqrt();
        }
        let length = sum / (3. * f64::from(n));
        let union = geometry.combine_surfaces(&[tool], &[target], NewellBooleanMode::Add)?;
        assert_eq!(union.outputs.len(), 1);
        assert!((geometry.surface_area(union.outputs[0])? - 4. * length).abs() < 1e-6);
        let difference =
            geometry.combine_surfaces(&[tool], &[target], NewellBooleanMode::Remove)?;
        assert_eq!(difference.outputs.len(), 1);
        assert!((geometry.surface_area(difference.outputs[0])? - length).abs() < 1e-6);
        assert!(
            !geometry
                .tessellate(union.outputs[0], 0.02)?
                .indices()
                .is_empty()
        );
        Ok(())
    }
    #[test]
    fn nonlinear_common_support_records_an_unfinished_required_case() -> TestResult {
        use brepkit_topology::edge::EdgeCurve;
        let solid = crate::NewellProfile::closed_cubic([[0., 0.], [2., 3.], [4., 2.], [6., 0.]])?
            .extrude([0., 0., 1.], 1.)?;
        let mut geometry = crate::NewellGeometry::new();
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
            .ok_or("curve")?
            .edge;
        let target = geometry.sweep_surface(
            (source, profile.face),
            NewellBoundary::Edge { wire: 0, edge },
            [0., 0., 1.],
            0.,
            3.,
        )?[0];
        let other = geometry.extrude_face(source, profile.face, Some([0., 0., 1.]), 2., 3.)?;
        let profile = geometry
            .profile_faces(other)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("other profile")?;
        let edge = profile.wires[0]
            .edges
            .iter()
            .find(|edge| !edge.straight)
            .ok_or("other curve")?
            .edge;
        let face = geometry.shape_faces(other)?[usize::try_from(profile.face)?];
        let wire = geometry.topology.face(face)?.outer_wire();
        let native = geometry.topology.wire(wire)?.edges()[usize::try_from(edge)?].edge();
        let EdgeCurve::NurbsCurve(curve) = geometry.topology.edge(native)?.curve() else {
            return Err("lost curve".into());
        };
        // C((s+s²)/2), in Bernstein degree six: the same regular cubic trace,
        // with a strictly positive parameter derivative throughout [0,1].
        // This is not a different profile, a zero edge, or a pinched sheet.
        let z = curve.control_points()[0].z();
        let points = [
            [0., 0.],
            [0.5, 0.75],
            [1.2, 1.6],
            [2.1, 363. / 160.],
            [3.2, 47. / 20.],
            [4.5, 1.5],
            [6., 0.],
        ];
        let curve = NurbsCurve::new(
            6,
            [vec![0.; 7], vec![1.; 7]].concat(),
            points.map(|p| Point3::new(p[0], p[1], z)).to_vec(),
            vec![1.; 7],
        )?;
        geometry
            .topology
            .edge_mut(native)?
            .set_curve(EdgeCurve::NurbsCurve(curve));
        let tool = geometry.sweep_surface(
            (other, profile.face),
            NewellBoundary::Edge { wire: 0, edge },
            [0., 0., 1.],
            -2.,
            1.,
        )?[0];
        let area = geometry.surface_area(target)?;
        assert!((geometry.surface_area(tool)? - area).abs() < 1e-7);
        // This assertion records the current diagnostic and operand preservation,
        // NOT acceptance: required Add must yield 4L and Remove must yield L.
        for mode in [NewellBooleanMode::Add, NewellBooleanMode::Remove] {
            for _ in 0..2 {
                let error = geometry
                    .combine_surfaces(&[tool], &[target], mode)
                    .unwrap_err();
                assert!(matches!(error, NewellKernelError::Operation { ref message }
                    if message == "curved sheet trim requires a verified common-domain route"));
                assert_eq!(geometry.surface_area(target)?, area);
                assert!(!geometry.tessellate(target, 0.02)?.indices().is_empty());
            }
        }
        Ok(())
    }
    #[test]
    fn tilted_common_domains_preserve_exact_bounds_identity_and_repeated_booleans() -> TestResult {
        use brepkit_math::vec::Vec3;
        let solid = crate::NewellProfile::closed_cubic([[0., 0.], [2., 3.], [4., 2.], [6., 0.]])?
            .extrude([0., 0., 1.], 1.)?;
        let mut geometry = crate::NewellGeometry::new();
        geometry.topology = solid.topology;
        let source = geometry.insert(solid.solid)?;
        let face = geometry
            .profile_faces(source)?
            .into_iter()
            .find(|f| f.frame.normal == [0., 0., 1.])
            .ok_or("support")?;
        let edge = face.wires[0]
            .edges
            .iter()
            .find(|e| !e.straight)
            .ok_or("cubic")?
            .edge;
        let boundary = NewellBoundary::Edge { wire: 0, edge };
        let ramp = |base, slope| SweepStation {
            gradient: Vec3::new(slope, 0., 0.),
            constant: base,
        };
        let target = geometry.sweep_surface_between(
            (source, face.face),
            boundary,
            [0., 0., 1.],
            ramp(0., 0.),
            ramp(3., 0.1),
        )?[0];
        let tool = geometry.sweep_surface_between(
            (source, face.face),
            boundary,
            [0., 0., 1.],
            ramp(1., 0.),
            ramp(4., 0.2),
        )?[0];
        // Composite Simpson oracle integrates the explicit polynomial speed,
        // independently of the production surface Jacobian integration. For
        // q=9-24t+9t², bounds |q|<=9, |q'|<=24, q''=18 and 36+q²>=36
        // give a fourth derivative bound <2e6 for this weighted integrand.
        // Simpson error 2e6/(180*n^4) plus f64 summation error is <1e-9.
        let n = 16384;
        let mut expected = 0.;
        let mut length = 0.;
        for i in 0..=n {
            let t = f64::from(i) / f64::from(n);
            let speed = (36. + (9. - 24. * t + 9. * t * t).powi(2)).sqrt();
            let w = if i == 0 || i == n {
                1.
            } else if i % 2 == 0 {
                2.
            } else {
                4.
            };
            expected += w * speed * (4. + 1.2 * t);
            length += w * speed;
        }
        expected /= 3. * f64::from(n);
        length /= 3. * f64::from(n);
        let union = geometry.combine_surfaces(&[tool], &[target], NewellBooleanMode::Add)?;
        assert_eq!(union.changed_targets, vec![0]);
        assert_eq!(union.outputs.len(), 1);
        let result = union.outputs[0];
        assert_eq!(geometry.source_wire(result), Some(0));
        assert!((geometry.surface_area(result)? - expected).abs() < 1e-6);
        let ids = geometry.shape_faces(result)?;
        assert_eq!(ids.len(), 1);
        let wire = geometry
            .topology
            .wire(geometry.topology.face(ids[0])?.outer_wire())?;
        assert_eq!(wire.edges().len(), 4);
        for (a, b) in wire
            .edges()
            .iter()
            .zip(wire.edges().iter().cycle().skip(1))
            .take(4)
        {
            let a_edge = geometry.topology.edge(a.edge())?;
            let b_edge = geometry.topology.edge(b.edge())?;
            let end = if a.is_forward() {
                a_edge.end()
            } else {
                a_edge.start()
            };
            let start = if b.is_forward() {
                b_edge.start()
            } else {
                b_edge.end()
            };
            assert_eq!(end, start);
            assert_ne!(a_edge.start(), a_edge.end());
        }
        assert_eq!(
            wire.edges()
                .iter()
                .filter(|e| matches!(
                    geometry.topology.edge(e.edge()).map(|e| e.curve()),
                    Ok(EdgeCurve::NurbsCurve(_))
                ))
                .count(),
            2
        );
        let mesh = geometry.tessellate(result, 0.02)?;
        assert_eq!(mesh.face_count(), 1);
        assert!(!mesh.indices().is_empty());
        let cut = geometry.combine_surfaces(&[tool], &[result], NewellBooleanMode::Remove)?;
        assert_eq!(cut.outputs.len(), 1);
        assert_eq!(geometry.source_wire(cut.outputs[0]), Some(0));
        assert!((geometry.surface_area(cut.outputs[0])? - length).abs() < 1e-6);
        assert!(
            geometry
                .combine_surfaces(&[target], &[result], NewellBooleanMode::Add)
                .is_err()
        );
        let reverse_target = geometry.sweep_surface_between(
            (source, face.face),
            boundary,
            [0., 0., -1.],
            ramp(0., 0.),
            ramp(3., 0.1),
        )?[0];
        let reverse_tool = geometry.sweep_surface_between(
            (source, face.face),
            boundary,
            [0., 0., -1.],
            ramp(1., 0.),
            ramp(4., 0.2),
        )?[0];
        let reverse = geometry.combine_surfaces(
            &[reverse_tool],
            &[reverse_target],
            NewellBooleanMode::Add,
        )?;
        assert_eq!(reverse.outputs.len(), 1);
        assert!((geometry.surface_area(reverse.outputs[0])? - expected).abs() < 1e-6);
        // Crossing envelopes and endpoint-pinched disks retain their exact
        // rational boundaries and remain valid inputs to later booleans.
        let crossing = geometry.sweep_surface_between(
            (source, face.face),
            boundary,
            [0., 0., 1.],
            ramp(1., 0.),
            ramp(3.6, -0.1),
        )?[0];
        let crossed = geometry.combine_surfaces(&[crossing], &[target], NewellBooleanMode::Add)?;
        assert_eq!(crossed.outputs.len(), 1);
        assert_eq!(geometry.source_wire(crossed.outputs[0]), Some(0));
        let mut crossing_area = 0.;
        let mut pinched_area = 0.;
        for i in 0..=n {
            let t = f64::from(i) / f64::from(n);
            let y = 9. * t - 12. * t * t + 3. * t * t * t;
            let speed = (36. + (9. - 24. * t + 9. * t * t).powi(2)).sqrt();
            let w = if i == 0 || i == n {
                1.
            } else if i % 2 == 0 {
                2.
            } else {
                4.
            };
            crossing_area += w * speed * (3. + 0.6 * t).max(3.6 - 0.6 * t);
            pinched_area += w * speed * 0.1 * y;
        }
        crossing_area /= 3. * f64::from(n);
        pinched_area /= 3. * f64::from(n);
        assert!((geometry.surface_area(crossed.outputs[0])? - crossing_area).abs() < 1e-6);
        assert!(
            !geometry
                .tessellate(crossed.outputs[0], 0.02)?
                .indices()
                .is_empty()
        );
        let repeated =
            geometry.combine_surfaces(&[tool], &crossed.outputs, NewellBooleanMode::Remove)?;
        assert_eq!(repeated.outputs.len(), 1);
        assert!((geometry.surface_area(repeated.outputs[0])? - length).abs() < 1e-6);
        let pinched = geometry.sweep_surface_between(
            (source, face.face),
            boundary,
            [0., 0., 1.],
            SweepStation {
                gradient: Vec3::new(0., 0.1, 0.),
                constant: 0.,
            },
            ramp(4., 0.2),
        )?[0];
        let pinched =
            geometry.combine_surfaces(&[pinched], &[target], NewellBooleanMode::Remove)?;
        assert_eq!(pinched.outputs.len(), 1);
        let lens = pinched.outputs[0];
        assert_eq!(geometry.source_wire(lens), Some(0));
        assert!((geometry.surface_area(lens)? - pinched_area).abs() < 1e-6);
        let faces = geometry.shape_faces(lens)?;
        assert_eq!(faces.len(), 1);
        let wire = geometry
            .topology
            .wire(geometry.topology.face(faces[0])?.outer_wire())?;
        assert_eq!(wire.edges().len(), 2);
        assert!(wire.edges().iter().all(|e| matches!(
            geometry.topology.edge(e.edge()).map(|e| e.curve()),
            Ok(EdgeCurve::NurbsCurve(_))
        )));
        assert!(!geometry.tessellate(lens, 0.02)?.indices().is_empty());
        let t = (24. - 252_f64.sqrt()) / 18.;
        let ymax = 9. * t - 12. * t * t + 3. * t * t * t;
        let pinching = geometry.sweep_surface_between(
            (source, face.face),
            boundary,
            [0., 0., 1.],
            SweepStation {
                gradient: Vec3::new(0., -1., 0.),
                constant: ymax,
            },
            ramp(4., 0.2),
        )?[0];
        assert!(
            matches!(geometry.combine_surfaces(&[pinching],&[target],NewellBooleanMode::Remove),Err(NewellKernelError::Operation {message}) if message=="nonmanifold pinched sheet vertex")
        );
        assert_eq!(geometry.source_wire(target), Some(0));
        assert!((geometry.surface_area(result)? - expected).abs() < 1e-6);
        Ok(())
    }
    #[test]
    fn production_common_domain_keeps_cubic_boundaries_for_union_and_split() -> TestResult {
        let solid = crate::NewellProfile::closed_cubic([[0., 0.], [2., 3.], [4., 2.], [6., 0.]])?
            .extrude([0., 0., 1.], 1.)?;
        let mut geometry = crate::NewellGeometry::new();
        geometry.topology = solid.topology;
        let source = geometry.insert(solid.solid)?;
        let face = geometry
            .profile_faces(source)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("support")?;
        let edge = face.wires[0]
            .edges
            .iter()
            .find(|edge| !edge.straight)
            .ok_or("cubic edge")?
            .edge;
        let boundary = NewellBoundary::Edge { wire: 0, edge };
        let target =
            geometry.sweep_surface((source, face.face), boundary, [0., 0., 1.], 0., 3.)?[0];
        let tool = geometry.sweep_surface((source, face.face), boundary, [0., 0., 1.], 1., 4.)?[0];
        let length = geometry.surface_area(target)? / 3.;
        let union = geometry.combine_surfaces(&[tool], &[target], NewellBooleanMode::Add)?;
        assert_eq!(union.outputs.len(), 1);
        assert!((geometry.surface_area(union.outputs[0])? - 4. * length).abs() < 1e-9);
        let target = geometry.sweep_surface(
            (source, face.face),
            NewellBoundary::Wire(0),
            [0., 0., 1.],
            0.,
            4.,
        )?[0];
        let tool = geometry.sweep_surface(
            (source, face.face),
            NewellBoundary::Wire(0),
            [0., 0., 1.],
            1.,
            3.,
        )?[0];
        let split = geometry.combine_surfaces(&[tool], &[target], NewellBooleanMode::Remove)?;
        assert_eq!(split.outputs.len(), 2);
        for shape in split.outputs {
            assert!((geometry.surface_area(shape)? - (length + 6.)).abs() < 1e-8);
            assert_eq!(geometry.face_count(shape)?, 2);
            assert!(!geometry.tessellate(shape, 0.05)?.indices().is_empty());
        }
        Ok(())
    }
    #[test]
    fn circular_and_cubic_boundaries_make_exact_uncapped_sheets_directly() -> TestResult {
        let mut geometry = crate::NewellGeometry::new();
        let source = brepkit_operations::primitives::make_cylinder(&mut geometry.topology, 2., 1.)?;
        let source = geometry.insert(source)?;
        let face = geometry
            .profile_faces(source)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("circle support")?
            .face;
        let shape = geometry.sweep_surface(
            (source, face),
            NewellBoundary::Wire(0),
            [0., 0., 1.],
            0.,
            4.,
        )?[0];
        assert_eq!(geometry.face_count(shape)?, 1);
        assert!((geometry.surface_area(shape)? - 16. * std::f64::consts::PI).abs() < 1e-8);
        let mesh = geometry.tessellate(shape, 0.05)?;
        assert!(!mesh.indices().is_empty());
        assert!(mesh.normals().iter().all(|normal| normal[2].abs() < 1e-6));
        let solid = crate::NewellProfile::closed_cubic([[0., 0.], [2., 3.], [4., 2.], [6., 0.]])?
            .extrude([0., 0., 1.], 1.)?;
        let mut geometry = crate::NewellGeometry::new();
        geometry.topology = solid.topology;
        let source = geometry.insert(solid.solid)?;
        let face = geometry
            .profile_faces(source)?
            .into_iter()
            .find(|face| face.frame.normal == [0., 0., 1.])
            .ok_or("cubic support")?;
        let edge = face.wires[0]
            .edges
            .iter()
            .find(|edge| !edge.straight)
            .ok_or("cubic edge")?
            .edge;
        let shape = geometry.sweep_surface(
            (source, face.face),
            NewellBoundary::Edge { wire: 0, edge },
            [0., 0., 1.],
            0.,
            3.,
        )?[0];
        let reference =
            NewellSurface::sweep_cubic([[0., 0.], [2., 3.], [4., 2.], [6., 0.]], 0., 3.)?;
        assert!((geometry.surface_area(shape)? - reference.area()?).abs() < 1e-9);
        assert!(!geometry.tessellate(shape, 0.05)?.indices().is_empty());
        Ok(())
    }
}
