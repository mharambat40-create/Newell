// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Exact profile construction and curve-based measures inside the kernel boundary.
use crate::{NewellKernelError, NewellSolid, operation_error};
use brepkit_math::{
    nurbs::curve::NurbsCurve,
    quadrature::gauss_legendre_points,
    vec::{Point3, Vec3},
};
use brepkit_topology::{
    FaceId, Topology,
    edge::{Edge, EdgeCurve},
    face::{Face, FaceSurface},
    wire::{OrientedEdge, Wire},
};

/// Unique construction role, independent of native arena indices and dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NewellFaceProvenance {
    Unknown,
    Base(u32),
    StartCap,
    EndCap,
    Wall { wire: u32, edge: u32 },
}
/// A bounded planar profile owning its exact B-Rep and supporting plane.
#[derive(Debug)]
pub struct NewellProfile {
    topology: Topology,
    face: FaceId,
}
impl NewellProfile {
    /// Constructs an exact circular section capped at z = intercept + slope*x.
    /// This bounded feasibility helper uses rational circle boundaries, never polygon chords.
    ///
    /// # Errors
    /// Rejects nonfinite/degenerate dimensions or a cap at/behind the source section.
    pub fn circular_up_to_x_plane(
        radius: f64,
        intercept: f64,
        slope: f64,
    ) -> Result<NewellSolid, NewellKernelError> {
        use brepkit_math::nurbs::surface::NurbsSurface;
        use brepkit_topology::{shell::Shell, solid::Solid, vertex::Vertex};
        if !radius.is_finite()
            || radius <= 1e-7
            || !intercept.is_finite()
            || !slope.is_finite()
            || intercept - slope.abs() * radius <= 1e-7
        {
            return Err(NewellKernelError::InvalidParameter {
                parameter: "circular termination",
            });
        }
        let mut topology = Topology::new();
        let xy = [
            [1., 0.],
            [1., 1.],
            [0., 1.],
            [-1., 1.],
            [-1., 0.],
            [-1., -1.],
            [0., -1.],
            [1., -1.],
            [1., 0.],
        ];
        let low: Vec<_> = xy
            .iter()
            .map(|p| Point3::new(p[0] * radius, p[1] * radius, 0.))
            .collect();
        let high: Vec<_> = low
            .iter()
            .map(|p| Point3::new(p.x(), p.y(), intercept + slope * p.x()))
            .collect();
        let w = std::f64::consts::FRAC_1_SQRT_2;
        let weights = vec![1., w, 1., w, 1., w, 1., w, 1.];
        let knots = vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.];
        let a = topology.add_vertex(Vertex::new(low[0], 1e-7));
        let b = topology.add_vertex(Vertex::new(high[0], 1e-7));
        let bottom = topology.add_edge(Edge::new(
            a,
            a,
            EdgeCurve::NurbsCurve(
                NurbsCurve::new(2, knots.clone(), low.clone(), weights.clone())
                    .map_err(operation_error)?,
            ),
        ));
        let top = topology.add_edge(Edge::new(
            b,
            b,
            EdgeCurve::NurbsCurve(
                NurbsCurve::new(2, knots.clone(), high.clone(), weights.clone())
                    .map_err(operation_error)?,
            ),
        ));
        let seam = topology.add_edge(Edge::new(a, b, EdgeCurve::Line));
        let wire = topology.add_wire(
            Wire::new(
                vec![
                    OrientedEdge::new(bottom, true),
                    OrientedEdge::new(seam, true),
                    OrientedEdge::new(top, false),
                    OrientedEdge::new(seam, false),
                ],
                true,
            )
            .map_err(operation_error)?,
        );
        let surface = NurbsSurface::new(
            2,
            1,
            knots,
            vec![0., 0., 1., 1.],
            low.into_iter().zip(high).map(|(p, q)| vec![p, q]).collect(),
            weights.into_iter().map(|w| vec![w, w]).collect(),
        )
        .map_err(operation_error)?;
        let side = topology.add_face(Face::new(wire, vec![], FaceSurface::Nurbs(surface)));
        let wire = topology.add_wire(
            Wire::new(vec![OrientedEdge::new(bottom, false)], true).map_err(operation_error)?,
        );
        let base = topology.add_face(Face::new(
            wire,
            vec![],
            FaceSurface::Plane {
                normal: Vec3::new(0., 0., -1.),
                d: 0.,
            },
        ));
        let wire = topology.add_wire(
            Wire::new(vec![OrientedEdge::new(top, true)], true).map_err(operation_error)?,
        );
        let normal = Vec3::new(-slope, 0., 1.)
            .normalize()
            .map_err(operation_error)?;
        let cap = topology.add_face(Face::new(
            wire,
            vec![],
            FaceSurface::Plane {
                normal,
                d: intercept / (1. + slope * slope).sqrt(),
            },
        ));
        let shell = topology.add_shell(Shell::new(vec![base, side, cap]).map_err(operation_error)?);
        let solid = topology.add_solid(Solid::new(shell, vec![]));
        let measured_volume = Some(ruled_volume(&topology, solid)?);
        Ok(NewellSolid {
            topology,
            solid,
            measured_volume,
        })
    }
    /// Constructs a non-rational cubic XY boundary closed by its straight chord.
    ///
    /// # Errors
    /// Rejects nonfinite coordinates, degenerate area, or invalid kernel topology.
    pub fn closed_cubic(control: [[f64; 2]; 4]) -> Result<Self, NewellKernelError> {
        if !control.iter().flatten().all(|v| v.is_finite()) {
            return Err(NewellKernelError::InvalidParameter {
                parameter: "control points",
            });
        }
        let curve = NurbsCurve::new(
            3,
            vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control
                .iter()
                .map(|p| Point3::new(p[0], p[1], 0.))
                .collect(),
            vec![1.; 4],
        )
        .map_err(operation_error)?;
        let mut topology = Topology::new();
        let edge =
            brepkit_topology::builder::make_nurbs_edge_from_curve(&mut topology, &curve, 1e-7);
        let e = topology.edge(edge).map_err(operation_error)?;
        let chord = topology.add_edge(Edge::new(e.end(), e.start(), EdgeCurve::Line));
        let wire = topology.add_wire(
            Wire::new(
                vec![
                    OrientedEdge::new(edge, true),
                    OrientedEdge::new(chord, true),
                ],
                true,
            )
            .map_err(operation_error)?,
        );
        let face = topology.add_face(Face::new(
            wire,
            vec![],
            FaceSurface::Plane {
                normal: Vec3::new(0., 0., 1.),
                d: 0.,
            },
        ));
        let profile = Self { topology, face };
        if profile.area()? <= 1e-14 {
            return Err(NewellKernelError::InvalidParameter {
                parameter: "profile area",
            });
        }
        Ok(profile)
    }
    /// Caps the actual cubic prism at the plane z = intercept + slope*x.
    ///
    /// # Errors
    /// Rejects backward/degenerate extents and invalid construction.
    pub fn up_to_x_plane(
        &self,
        intercept: f64,
        slope: f64,
    ) -> Result<NewellSolid, NewellKernelError> {
        let face = self.topology.face(self.face).map_err(operation_error)?;
        let wire = self
            .topology
            .wire(face.outer_wire())
            .map_err(operation_error)?;
        let edge = wire
            .edges()
            .first()
            .ok_or_else(|| NewellKernelError::Operation {
                message: "missing cubic".into(),
            })?;
        let edge = self.topology.edge(edge.edge()).map_err(operation_error)?;
        let EdgeCurve::NurbsCurve(curve) = edge.curve() else {
            return Err(NewellKernelError::Operation {
                message: "expected cubic".into(),
            });
        };
        let lo: [Point3; 4] = curve.control_points().try_into().map_err(operation_error)?;
        let hi = lo.map(|p| Point3::new(p.x(), p.y(), intercept + slope * p.x()));
        if !intercept.is_finite()
            || !slope.is_finite()
            || hi.iter().any(|p| p.z() <= 1e-7 || !p.z().is_finite())
        {
            return Err(NewellKernelError::InvalidParameter {
                parameter: "termination plane",
            });
        }
        let mut topology = Topology::new();
        let (side, [bottom, top], [left, right]) =
            crate::surface::cubic_band(&mut topology, lo, hi, true)?;
        let e = topology.edge(bottom).map_err(operation_error)?;
        let bottom_chord = topology.add_edge(Edge::new(e.end(), e.start(), EdgeCurve::Line));
        let e = topology.edge(top).map_err(operation_error)?;
        let top_chord = topology.add_edge(Edge::new(e.end(), e.start(), EdgeCurve::Line));
        let bottom_wire = topology.add_wire(
            Wire::new(
                vec![
                    OrientedEdge::new(bottom, true),
                    OrientedEdge::new(bottom_chord, true),
                ],
                true,
            )
            .map_err(operation_error)?,
        );
        let bottom_face = topology.add_face(Face::new(
            bottom_wire,
            vec![],
            FaceSurface::Plane {
                normal: Vec3::new(0., 0., -1.),
                d: 0.,
            },
        ));
        let top_wire = topology.add_wire(
            Wire::new(
                vec![
                    OrientedEdge::new(top_chord, false),
                    OrientedEdge::new(top, false),
                ],
                true,
            )
            .map_err(operation_error)?,
        );
        let normal = Vec3::new(-slope, 0., 1.)
            .normalize()
            .map_err(operation_error)?;
        let top_face = topology.add_face(Face::new(
            top_wire,
            vec![],
            FaceSurface::Plane {
                normal,
                d: intercept / (1. + slope * slope).sqrt(),
            },
        ));
        let end_wire = topology.add_wire(
            Wire::new(
                vec![
                    OrientedEdge::new(bottom_chord, false),
                    OrientedEdge::new(right, true),
                    OrientedEdge::new(top_chord, true),
                    OrientedEdge::new(left, false),
                ],
                true,
            )
            .map_err(operation_error)?,
        );
        let end_face = topology.add_face(Face::new(
            end_wire,
            vec![],
            FaceSurface::Plane {
                normal: Vec3::new(0., -1., 0.),
                d: 0.,
            },
        ));
        let shell = topology.add_shell(
            brepkit_topology::shell::Shell::new(vec![bottom_face, top_face, side, end_face])
                .map_err(operation_error)?,
        );
        let solid = topology.add_solid(brepkit_topology::solid::Solid::new(shell, vec![]));
        let measured_volume = Some(ruled_volume(&topology, solid)?);
        Ok(NewellSolid {
            topology,
            solid,
            measured_volume,
        })
    }
    /// Integrates the actual polynomial B-Rep boundary with Green's theorem.
    /// Does not replace curved edges by a tessellated polygon.
    ///
    /// # Errors
    /// Propagates missing topology or nonfinite geometry.
    pub fn area(&self) -> Result<f64, NewellKernelError> {
        planar_area(&self.topology, self.face)
    }
    /// Produces the actual exact solid using BrepKIT's face extrusion.
    ///
    /// # Errors
    /// Rejects invalid distance/direction or failed kernel operations.
    pub fn extrude(
        &self,
        direction: [f64; 3],
        distance: f64,
    ) -> Result<NewellSolid, NewellKernelError> {
        if !distance.is_finite() || distance <= 1e-7 || !direction.iter().all(|v| v.is_finite()) {
            return Err(NewellKernelError::InvalidParameter {
                parameter: "extent",
            });
        }
        let d = Vec3::new(direction[0], direction[1], direction[2])
            .normalize()
            .map_err(operation_error)?;
        let normal = self.frame()?.normal;
        if d.dot(Vec3::new(normal[0], normal[1], normal[2])).abs() <= 1e-12 {
            return Err(NewellKernelError::InvalidParameter {
                parameter: "direction",
            });
        }
        let mut topology = self.topology.clone();
        let solid = brepkit_operations::extrude::extrude(&mut topology, self.face, d, distance)
            .map_err(operation_error)?;
        // This constructor knows the swept polynomial side faces cover their complete
        // rectangular UV domains. Integrate the resulting B-Rep, not area * requested depth.
        let measured_volume = Some(ruled_volume(&topology, solid)?);
        Ok(NewellSolid {
            topology,
            solid,
            measured_volume,
        })
    }
}

fn planar_area(topology: &Topology, id: FaceId) -> Result<f64, NewellKernelError> {
    let face = topology.face(id).map_err(operation_error)?;
    let n = face
        .effective_plane_normal()
        .ok_or_else(|| NewellKernelError::Operation {
            message: "profile is not planar".into(),
        })?;
    let axis = if n.z().abs() < 0.9 {
        Vec3::new(0., 0., 1.)
    } else {
        Vec3::new(1., 0., 0.)
    };
    let u = (axis - n * n.dot(axis))
        .normalize()
        .map_err(operation_error)?;
    let v = n.cross(u);
    let project = |p: Point3| {
        let p = Vec3::new(p.x(), p.y(), p.z());
        (p.dot(u), p.dot(v))
    };
    let wire = topology.wire(face.outer_wire()).map_err(operation_error)?;
    let mut sum = 0.;
    for oe in wire.edges() {
        let e = topology.edge(oe.edge()).map_err(operation_error)?;
        let a = topology.vertex(e.start()).map_err(operation_error)?.point();
        let b = topology.vertex(e.end()).map_err(operation_error)?.point();
        let area = match e.curve() {
            EdgeCurve::Line => {
                let a = project(a);
                let b = project(b);
                a.0 * b.1 - a.1 * b.0
            }
            EdgeCurve::NurbsCurve(c) => {
                let (lo, hi) = c.domain();
                let mut integral = 0.;
                for part in 0..32 {
                    let start = lo + (hi - lo) * f64::from(part) / 32.;
                    let end = lo + (hi - lo) * f64::from(part + 1) / 32.;
                    for q in gauss_legendre_points(8) {
                        let t = (start + end) * 0.5 + (end - start) * 0.5 * q.x;
                        let p = c.evaluate(t);
                        let deriv = c.derivatives(t, 1);
                        let d = deriv.get(1).ok_or_else(|| NewellKernelError::Operation {
                            message: "missing curve derivative".into(),
                        })?;
                        let p = project(p);
                        integral += q.w * (p.0 * d.dot(v) - p.1 * d.dot(u)) * (end - start) * 0.5;
                    }
                }
                integral
            }
            _ => {
                return Err(NewellKernelError::Operation {
                    message: "unexpected polynomial profile curve".into(),
                });
            }
        };
        sum += if oe.is_forward() { area } else { -area };
    }
    let area = (sum * 0.5_f64).abs();
    if !area.is_finite() {
        return Err(NewellKernelError::NonFiniteMesh);
    }
    Ok(area)
}

/// Only called for this module's exact polynomial extrusions, before any trims/booleans.
fn ruled_volume(
    topology: &Topology,
    solid: brepkit_topology::SolidId,
) -> Result<f64, NewellKernelError> {
    use brepkit_math::traits::ParametricSurface;
    let mut sum = 0.;
    for id in brepkit_topology::explorer::solid_faces(topology, solid).map_err(operation_error)? {
        let face = topology.face(id).map_err(operation_error)?;
        let sign = if face.is_reversed() { -1. } else { 1. };
        match face.surface() {
            FaceSurface::Plane { normal, d } => {
                sum += sign * planar_area(topology, id)? * d / normal.length() / 3.;
            }
            FaceSurface::Nurbs(surface) => {
                let (a, b) = surface.domain_u();
                let (c, d) = surface.domain_v();
                for part in 0..32 {
                    let start = a + (b - a) * f64::from(part) / 32.;
                    let end = a + (b - a) * f64::from(part + 1) / 32.;
                    for x in gauss_legendre_points(8) {
                        for y in gauss_legendre_points(8) {
                            let u = (start + end) * 0.5 + (end - start) * 0.5 * x.x;
                            let v = (c + d) * 0.5 + (d - c) * 0.5 * y.x;
                            let p = surface.evaluate(u, v);
                            let n = ParametricSurface::partial_u(surface, u, v)
                                .cross(ParametricSurface::partial_v(surface, u, v));
                            sum += sign
                                * Vec3::new(p.x(), p.y(), p.z()).dot(n)
                                * x.w
                                * y.w
                                * (end - start)
                                * (d - c)
                                / 12.;
                        }
                    }
                }
            }
            _ => {
                return Err(NewellKernelError::Operation {
                    message: "unexpected polynomial extrusion surface".into(),
                });
            }
        }
    }
    if !sum.is_finite() {
        return Err(NewellKernelError::NonFiniteMesh);
    }
    Ok(sum.abs())
}

/// Canonical supporting-plane coordinates, independent of tessellation and edge traversal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NewellProfileFrame {
    pub origin: [f64; 3],
    pub normal: [f64; 3],
    pub u: [f64; 3],
    pub v: [f64; 3],
}
/// One oriented edge occurrence qualified by its planar supporting face and wire.
#[derive(Debug, Clone, PartialEq)]
pub struct NewellProfileEdge {
    pub edge: u32,
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub straight: bool,
}
/// A genuine connected boundary wire on a supporting face.
#[derive(Debug, Clone, PartialEq)]
pub struct NewellProfileWire {
    pub wire: u32,
    pub closed: bool,
    pub edges: Vec<NewellProfileEdge>,
}
/// One real face, in exactly the order of `NewellMesh::face_offsets`.
#[derive(Debug, Clone, PartialEq)]
pub struct NewellProfileFace {
    pub face: u32,
    pub provenance: NewellFaceProvenance,
    pub frame: NewellProfileFrame,
    pub wires: Vec<NewellProfileWire>,
}
fn xyz(v: Vec3) -> [f64; 3] {
    [v.x(), v.y(), v.z()]
}
fn point_xyz(p: Point3) -> [f64; 3] {
    [p.x(), p.y(), p.z()]
}
pub(crate) fn supporting_frame(
    topology: &Topology,
    face: FaceId,
) -> Result<NewellProfileFrame, NewellKernelError> {
    let face = topology.face(face).map_err(operation_error)?;
    let FaceSurface::Plane { normal, d } = face.surface() else {
        return Err(NewellKernelError::Operation {
            message: "profile support is not planar".into(),
        });
    };
    let length = normal.length();
    if !length.is_finite() || length <= 1e-12 {
        return Err(NewellKernelError::InvalidParameter {
            parameter: "supporting plane",
        });
    }
    let sign = if face.is_reversed() { -1. } else { 1. };
    let n = *normal * (sign / length);
    let h = *d * sign / length;
    let axes = [
        Vec3::new(1., 0., 0.),
        Vec3::new(0., 1., 0.),
        Vec3::new(0., 0., 1.),
    ];
    let mut axis = axes[0];
    let mut alignment = n.dot(axis).abs();
    for next in &axes[1..] {
        let next_alignment = n.dot(*next).abs();
        if next_alignment < alignment {
            axis = *next;
            alignment = next_alignment;
        }
    }
    let u = (axis - n * n.dot(axis))
        .normalize()
        .map_err(operation_error)?;
    let v = n.cross(u);
    Ok(NewellProfileFrame {
        origin: xyz(n * h),
        normal: xyz(n),
        u: xyz(u),
        v: xyz(v),
    })
}
impl NewellSolid {
    /// Enumerates actual planar supporting faces and their oriented boundary occurrences.
    /// Face ordinals are tied to the B-Rep generation, never triangle indices or deflection.
    ///
    /// # Errors
    /// Propagates invalid topology and identity capacity errors.
    pub fn profile_faces(&self) -> Result<Vec<NewellProfileFace>, NewellKernelError> {
        describe_faces(
            &self.topology,
            brepkit_topology::explorer::solid_faces(&self.topology, self.solid)
                .map_err(operation_error)?,
        )
    }
    /// Resolves a planar face of this exact B-Rep as an owned closed profile.
    ///
    /// # Errors
    /// Rejects a missing ordinal or nonplanar supporting face.
    pub fn face_profile(&self, ordinal: u32) -> Result<NewellProfile, NewellKernelError> {
        let faces = brepkit_topology::explorer::solid_faces(&self.topology, self.solid)
            .map_err(operation_error)?;
        let face = *faces
            .get(usize::try_from(ordinal).map_err(operation_error)?)
            .ok_or_else(|| NewellKernelError::Operation {
                message: "missing profile face".into(),
            })?;
        supporting_frame(&self.topology, face)?;
        Ok(NewellProfile {
            topology: self.topology.clone(),
            face,
        })
    }
}
impl NewellProfile {
    /// Effective oriented supporting plane, with the deterministic approved frame.
    ///
    /// # Errors
    /// Rejects invalid/nonplanar support.
    pub fn frame(&self) -> Result<NewellProfileFrame, NewellKernelError> {
        supporting_frame(&self.topology, self.face)
    }
}

pub(crate) fn describe_faces(
    topology: &Topology,
    faces: Vec<FaceId>,
) -> Result<Vec<NewellProfileFace>, NewellKernelError> {
    let mut result = Vec::new();
    for (ordinal, id) in faces.into_iter().enumerate() {
        let face = topology.face(id).map_err(operation_error)?;
        if !matches!(face.surface(), FaceSurface::Plane { .. }) {
            continue;
        }
        let frame = supporting_frame(topology, id)?;
        let mut wires = Vec::new();
        for (wire_index, wire_id) in std::iter::once(face.outer_wire())
            .chain(face.inner_wires().iter().copied())
            .enumerate()
        {
            let wire = topology.wire(wire_id).map_err(operation_error)?;
            let mut edges = Vec::new();
            let mut occurrences: Vec<_> = wire.edges().iter().enumerate().collect();
            if face.is_reversed() {
                occurrences.reverse();
            }
            for (edge_index, oe) in occurrences {
                let edge = topology.edge(oe.edge()).map_err(operation_error)?;
                let a = topology
                    .vertex(edge.start())
                    .map_err(operation_error)?
                    .point();
                let b = topology
                    .vertex(edge.end())
                    .map_err(operation_error)?
                    .point();
                let (a, b) = if oe.is_forward() ^ face.is_reversed() {
                    (a, b)
                } else {
                    (b, a)
                };
                edges.push(NewellProfileEdge {
                    edge: u32::try_from(edge_index).map_err(operation_error)?,
                    start: point_xyz(a),
                    end: point_xyz(b),
                    straight: matches!(edge.curve(), EdgeCurve::Line),
                });
            }
            wires.push(NewellProfileWire {
                wire: u32::try_from(wire_index).map_err(operation_error)?,
                closed: wire.is_closed(),
                edges,
            });
        }
        result.push(NewellProfileFace {
            face: u32::try_from(ordinal).map_err(operation_error)?,
            provenance: NewellFaceProvenance::Unknown,
            frame,
            wires,
        });
    }
    Ok(result)
}

/// Exact B-Rep flux for planar caps and sides ruled parallel to one common direction.
/// Trimmed sides have zero flux in that direction; only their actual cap wires are
/// integrated. This avoids upstream mesh-based planar spline-cap measurements.
pub(crate) fn parallel_sweep_volume(
    topology: &Topology,
    solid: brepkit_topology::SolidId,
) -> Result<Option<f64>, NewellKernelError> {
    let faces =
        brepkit_topology::explorer::solid_faces(topology, solid).map_err(operation_error)?;
    let reference = faces
        .first()
        .and_then(|face| topology.face(*face).ok())
        .and_then(|face| topology.wire(face.outer_wire()).ok())
        .and_then(|wire| wire.edges().first())
        .and_then(|occurrence| topology.edge(occurrence.edge()).ok())
        .and_then(|edge| topology.vertex(edge.start()).ok())
        .map(|vertex| vertex.point())
        .ok_or_else(|| operation_error("missing volume reference vertex"))?;
    let mut direction = None;
    for id in &faces {
        let face = topology.face(*id).map_err(operation_error)?;
        let candidate = match face.surface() {
            FaceSurface::Plane { .. } => continue,
            FaceSurface::Cylinder(c) => c.axis(),
            FaceSurface::Nurbs(surface) => {
                let points = surface.control_points();
                let weights = surface.weights();
                if points.len() != weights.len()
                    || points.iter().zip(weights).any(|(p, w)| p.len() != w.len())
                {
                    return Ok(None);
                }
                let pairs: Vec<_> =
                    if surface.degree_v() == 1 && points.iter().all(|row| row.len() == 2) {
                        points
                            .iter()
                            .zip(weights)
                            .filter(|(p, w)| p.len() == 2 && w.len() == 2)
                            .map(|(p, w)| ([p[0], p[1]], [w[0], w[1]]))
                            .collect()
                    } else if surface.degree_u() == 1 && points.len() == 2 && weights.len() == 2 {
                        points[0]
                            .iter()
                            .zip(&points[1])
                            .zip(weights[0].iter().zip(&weights[1]))
                            .map(|((a, b), (wa, wb))| ([*a, *b], [*wa, *wb]))
                            .collect()
                    } else {
                        return Ok(None);
                    };
                let Some((points, _)) = pairs.first() else {
                    return Ok(None);
                };
                let candidate = (points[1] - points[0])
                    .normalize()
                    .map_err(operation_error)?;
                let mut ratio = None;
                for (points, weights) in pairs {
                    if weights.iter().any(|w| !w.is_finite() || *w <= 0.) {
                        return Ok(None);
                    }
                    let step = points[1] - points[0];
                    if step.cross(candidate).length() > 1e-10 * step.length().max(1.) {
                        return Ok(None);
                    }
                    let value = weights[1] / weights[0];
                    let first = *ratio.get_or_insert(value);
                    if (value - first).abs() > 1e-12 * first.abs().max(1.) {
                        return Ok(None);
                    }
                }
                candidate
            }
            _ => return Ok(None),
        };
        let candidate = candidate.normalize().map_err(operation_error)?;
        let d = *direction.get_or_insert(candidate);
        if d.cross(candidate).length() > 1e-10 {
            return Ok(None);
        }
    }
    let direction = direction.unwrap_or(Vec3::new(0., 0., 1.));
    let mut volume = 0.;
    for id in faces {
        let face = topology.face(id).map_err(operation_error)?;
        if !matches!(face.surface(), FaceSurface::Plane { .. }) {
            continue;
        }
        let frame = supporting_frame(topology, id)?;
        let vector = |a: [f64; 3]| Vec3::new(a[0], a[1], a[2]);
        let n = vector(frame.normal);
        let factor = n.dot(direction);
        if factor.abs() <= 1e-14 {
            continue;
        }
        let u = vector(frame.u);
        let v = vector(frame.v);
        // Integrate in a frame near the actual face. World-plane origins can
        // put a tiny profile millions of units from its integration origin,
        // losing its area and first moments through cancellation.
        let occurrence = topology
            .wire(face.outer_wire())
            .map_err(operation_error)?
            .edges()
            .first()
            .ok_or_else(|| operation_error("empty cap boundary"))?;
        let anchor = topology
            .vertex(
                topology
                    .edge(occurrence.edge())
                    .map_err(operation_error)?
                    .start(),
            )
            .map_err(operation_error)?
            .point();
        let origin = Vec3::new(anchor.x(), anchor.y(), anchor.z());
        let project = |p: Point3| {
            let p = Vec3::new(p.x(), p.y(), p.z()) - origin;
            (p.dot(u), p.dot(v))
        };
        let mut moments = [0.; 3];
        for (index, wire) in std::iter::once(face.outer_wire())
            .chain(face.inner_wires().iter().copied())
            .enumerate()
        {
            let mut loop_moments = [0.; 3];
            for occurrence in topology.wire(wire).map_err(operation_error)?.edges() {
                let edge = topology.edge(occurrence.edge()).map_err(operation_error)?;
                let a = topology
                    .vertex(edge.start())
                    .map_err(operation_error)?
                    .point();
                let b = topology
                    .vertex(edge.end())
                    .map_err(operation_error)?
                    .point();
                let (mut lo, mut hi) = edge.curve().domain_with_endpoints(a, b);
                if !occurrence.is_forward() {
                    std::mem::swap(&mut lo, &mut hi);
                }
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
                for span in breaks.windows(2) {
                    for part in 0..32 {
                        let lo = span[0] + (span[1] - span[0]) * f64::from(part) / 32.;
                        let hi = span[0] + (span[1] - span[0]) * f64::from(part + 1) / 32.;
                        for q in gauss_legendre_points(8) {
                            let t = (lo + hi) * 0.5 + (hi - lo) * 0.5 * q.x;
                            let p = project(edge.curve().evaluate_with_endpoints(t, a, b));
                            let derivative = match edge.curve() {
                                EdgeCurve::Line => b - a,
                                EdgeCurve::NurbsCurve(c) => *c
                                    .derivatives(t, 1)
                                    .get(1)
                                    .ok_or_else(|| operation_error("missing curve derivative"))?,
                                EdgeCurve::Circle(c) => {
                                    c.u_axis() * (-c.radius() * t.sin())
                                        + c.v_axis() * (c.radius() * t.cos())
                                }
                                EdgeCurve::Ellipse(c) => {
                                    c.u_axis() * (-c.semi_major() * t.sin())
                                        + c.v_axis() * (c.semi_minor() * t.cos())
                                }
                            };
                            let (dx, dy) = (derivative.dot(u), derivative.dot(v));
                            let weight = q.w * (hi - lo) * 0.25;
                            loop_moments[0] += weight * (p.0 * dy - p.1 * dx);
                            loop_moments[1] += weight * p.0 * p.0 * dy;
                            loop_moments[2] -= weight * p.1 * p.1 * dx;
                        }
                    }
                }
            }
            let sign = if index == 0 { 1. } else { -1. } * loop_moments[0].signum();
            for axis in 0..3 {
                moments[axis] += sign * loop_moments[axis];
            }
        }
        volume += factor
            * (direction.dot(anchor - reference) * moments[0]
                + direction.dot(u) * moments[1]
                + direction.dot(v) * moments[2]);
    }
    if !volume.is_finite() {
        return Err(operation_error("nonfinite B-Rep volume"));
    }
    Ok(Some(volume.abs()))
}
