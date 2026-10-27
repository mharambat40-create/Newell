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
