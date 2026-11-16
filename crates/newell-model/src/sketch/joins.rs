// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use super::{
    DrawGeometry, SegmentSolveReport, SketchDimensionId, SketchPointRef, SketchPrimitive as Shape,
    SketchRecipe,
};
use crate::ModelError;
use std::{
    collections::{BTreeMap, BTreeSet},
    f64::consts::TAU,
};

type Point = [f64; 2];
const POSITION: f64 = 1e-7;
const ANGLE: f64 = 1e-8;
const PASSES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SketchJoin {
    Coincident(SketchPointRef, SketchPointRef),
    PointOnCurve {
        point: SketchPointRef,
        curve: u64,
        parameter: f64,
    },
    Tangent {
        point: SketchPointRef,
        source: SketchPointRef,
        opposite: bool,
    },
}

impl SketchPointRef {
    pub fn geometry(self) -> Option<u64> {
        match self {
            Self::Origin => None,
            Self::Point(key) | Self::Start(key) | Self::End(key) | Self::Center(key) | Self::Defining(key, _) => Some(key),
        }
    }
}

impl SketchJoin {
    pub fn keys(self) -> [Option<u64>; 2] {
        match self {
            Self::Coincident(point, source) | Self::Tangent { point, source, .. } => {
                [point.geometry(), source.geometry()]
            }
            Self::PointOnCurve { point, curve, .. } => [point.geometry(), Some(curve)],
        }
    }
}

fn invalid() -> ModelError {
    ModelError::InvalidSketch("invalid Sketch join operand")
}

pub fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
pub fn cross(a: Point, b: Point) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
pub fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}
pub fn add(a: Point, b: Point) -> Point {
    [a[0] + b[0], a[1] + b[1]]
}
fn length(a: Point) -> f64 {
    a[0].hypot(a[1])
}
pub fn unit(a: Point) -> Option<Point> {
    let norm = length(a);
    (norm.is_finite() && norm >= POSITION).then(|| a.map(|v| v / norm))
}
fn rotate(p: Point, angle: f64) -> Point {
    let (sin, cos) = angle.sin_cos();
    [p[0] * cos - p[1] * sin, p[0] * sin + p[1] * cos]
}

pub fn finite_curve_contact(shape: Shape, aim: Point) -> Option<(Point, f64)> {
    let parameter = match shape {
        Shape::Line { start, end } => {
            let delta = sub(end, start);
            let direction = unit(delta)?;
            (dot(sub(aim, start), direction) / length(delta)).clamp(0., 1.)
        }
        Shape::Draw(DrawGeometry::Arc {
            center,
            radial,
            sweep,
        }) => {
            let toward = unit(sub(aim, center))?;
            let radial = unit(radial)?;
            let angle =
                (sweep.signum() * cross(radial, toward).atan2(dot(radial, toward))).rem_euclid(TAU);
            if angle <= sweep.abs() {
                angle / sweep.abs()
            } else {
                let ends = shape.endpoint_tangents()?;
                if length(sub(aim, ends[0].0)) <= length(sub(aim, ends[1].0)) {
                    0.
                } else {
                    1.
                }
            }
        }
        _ => return None,
    };
    Some((curve_at(shape, parameter)?, parameter))
}

pub fn curve_at(shape: Shape, parameter: f64) -> Option<Point> {
    if !(0. ..=1.).contains(&parameter) {
        return None;
    }
    match shape {
        Shape::Line { start, end } => Some(add(start, sub(end, start).map(|v| v * parameter))),
        Shape::Draw(DrawGeometry::Arc {
            center,
            radial,
            sweep,
        }) => Some(add(center, rotate(radial, sweep * parameter))),
        _ => None,
    }
}

fn endpoint(shape: Shape, point: SketchPointRef) -> Option<(Point, Point)> {
    let index = match point {
        SketchPointRef::Start(_) => 0,
        SketchPointRef::End(_) => 1,
        _ => return None,
    };
    match shape {
        Shape::Line { .. } | Shape::Draw(DrawGeometry::Arc { .. }) => {
            shape.endpoint_tangents().map(|ends| ends[index])
        }
        _ => None,
    }
}

fn point_at(shape: Shape, reference: SketchPointRef) -> Option<Point> {
    match (reference, shape) {
        (SketchPointRef::Point(_), Shape::Draw(DrawGeometry::Point(p)))
        | (SketchPointRef::Center(_), Shape::Draw(DrawGeometry::Arc { center: p, .. })) => Some(p),
        (
            SketchPointRef::Center(_),
            Shape::Draw(DrawGeometry::Ellipse {
                center,
                major,
                minor,
            }),
        ) if (length(major) - minor).abs() <= 1e-12 * minor => Some(center),
        _ => endpoint(shape, reference).map(|value| value.0),
    }
}

impl SketchRecipe {
    pub fn next_relation_id(&self) -> Result<SketchDimensionId, ModelError> {
        let last = self
            .segment_equations
            .keys()
            .chain(self.driven_dimensions.keys())
            .chain(self.joins.keys())
            .max();
        Ok(SketchDimensionId(
            last.map_or(Some(0), |id| id.0.checked_add(1))
                .ok_or(ModelError::IdentityExhausted)?,
        ))
    }

    pub fn join_eligible(&self, key: u64) -> bool {
        !self.formulas.contains_key(&key)
            && !self.rigid_formulas.contains_key(&key)
            && self.corner_formula_for(key).is_none()
    }

    pub fn join_point(&self, reference: SketchPointRef) -> Option<Point> {
        let Some(key) = reference.geometry() else {
            return Some([0.; 2]);
        };
        if !self.join_eligible(key) {
            return None;
        }
        let index = self
            .primitives
            .binary_search_by_key(&key, |pair| pair.0)
            .ok();
        let shape = index.map(|index| self.primitives[index].1).or_else(|| {
            self.primitives
                .iter()
                .find_map(|&(id, shape)| (key == id).then_some(shape))
        })?;
        point_at(shape, reference)
    }

    pub(super) fn validate_joins(&self) -> Result<(), ModelError> {
        let shapes: BTreeMap<_, _> = self.primitives.iter().copied().collect();
        for (&id, &join) in self.joins.iter() {
            if self.segment_equations.contains_key(&id)
                || self.driven_dimensions.contains_key(&id)
                || join
                    .keys()
                    .into_iter()
                    .flatten()
                    .any(|key| !self.join_eligible(key))
                || !join_satisfied(&shapes, join)?
            {
                return Err(ModelError::AffectedSketchDimensions(vec![id.0]));
            }
        }
        Ok(())
    }

    pub fn solve_placement(
        &self,
        cancelled: impl Fn() -> bool,
    ) -> Result<SegmentSolveReport, ModelError> {
        if cancelled() {
            return Err(ModelError::InvalidSketch("solve cancelled"));
        }
        self.validate()?;
        self.validate_segment_equations()?;
        self.validate_joins()?;
        Ok(SegmentSolveReport {
            joined: !self.joins.is_empty(),
            ..Default::default()
        })
    }

    pub fn solve_constraints(
        &mut self,
        anchors: &[u64],
        cancelled: impl Fn() -> bool,
    ) -> Result<SegmentSolveReport, ModelError> {
        if self.joins.is_empty() {
            return self.solve_segment_equations(cancelled);
        }
        let mut affected: BTreeSet<_> = anchors.iter().copied().collect();
        if affected.is_empty() {
            affected.extend(
                self.joins
                    .values()
                    .flat_map(|join| join.keys().into_iter().flatten()),
            );
        }
        let mut neighbors = BTreeMap::<u64, Vec<u64>>::new();
        for join in self.joins.values() {
            if let [Some(a), Some(b)] = join.keys() {
                neighbors.entry(a).or_default().push(b);
                neighbors.entry(b).or_default().push(a);
            }
        }
        let mut pending: Vec<_> = affected.iter().copied().collect();
        while let Some(key) = pending.pop() {
            if cancelled() {
                return Err(ModelError::InvalidSketch("solve cancelled"));
            }
            for &neighbor in neighbors.get(&key).into_iter().flatten() {
                if affected.insert(neighbor) {
                    pending.push(neighbor);
                }
            }
        }
        let joins: Vec<_> = self
            .joins
            .iter()
            .filter(|(_, join)| {
                join.keys()
                    .into_iter()
                    .flatten()
                    .any(|key| affected.contains(&key))
            })
            .collect();
        if joins.is_empty() {
            return self.solve_segment_equations_where(|key| affected.contains(&key), cancelled);
        }
        let mut candidate = self.clone();
        candidate.solve_segment_equations_where(|key| affected.contains(&key), &cancelled)?;
        let mut shapes: BTreeMap<_, _> = candidate.primitives.iter().copied().collect();
        for _ in 0..PASSES {
            if cancelled() {
                return Err(ModelError::InvalidSketch("solve cancelled"));
            }
            for &(_, &join) in &joins {
                if !join_satisfied(&shapes, join)? {
                    correct(&mut shapes, join, anchors)?;
                }
            }
            for (key, shape) in &mut candidate.primitives {
                if let Some(&replacement) = shapes.get(key) {
                    *shape = replacement;
                }
            }
            if candidate.validate_segment_equations().is_ok() && candidate.validate_joins().is_ok()
            {
                for join in candidate.joins.values_mut() {
                    if let SketchJoin::PointOnCurve {
                        point,
                        curve,
                        parameter,
                    } = join
                    {
                        *parameter = finite_curve_contact(
                            *shapes.get(curve).ok_or_else(invalid)?,
                            get_point(&shapes, *point)?,
                        )
                        .ok_or_else(invalid)?
                        .1;
                    }
                }
                candidate.validate()?;
                if cancelled() {
                    return Err(ModelError::InvalidSketch("solve cancelled"));
                }
                *self = candidate;
                return Ok(SegmentSolveReport {
                    joined: true,
                    ..Default::default()
                });
            }
            candidate.solve_segment_equations_where(|key| affected.contains(&key), &cancelled)?;
            shapes = candidate.primitives.iter().copied().collect();
        }
        Err(ModelError::AffectedSketchDimensions(
            joins.iter().map(|(id, _)| id.0).collect(),
        ))
    }
}

fn get_point(
    shapes: &BTreeMap<u64, Shape>,
    reference: SketchPointRef,
) -> Result<Point, ModelError> {
    let Some(key) = reference.geometry() else {
        return Ok([0.; 2]);
    };
    shapes
        .get(&key)
        .and_then(|&shape| point_at(shape, reference))
        .ok_or_else(invalid)
}

fn get_endpoint(
    shapes: &BTreeMap<u64, Shape>,
    reference: SketchPointRef,
) -> Result<(Point, Point), ModelError> {
    let (point, tangent) = reference
        .geometry()
        .and_then(|key| shapes.get(&key))
        .and_then(|&shape| endpoint(shape, reference))
        .ok_or_else(invalid)?;
    Ok((point, unit(tangent).ok_or_else(invalid)?))
}

fn join_satisfied(shapes: &BTreeMap<u64, Shape>, join: SketchJoin) -> Result<bool, ModelError> {
    let [a, b] = join.keys();
    if a == b {
        return Err(invalid());
    }
    Ok(match join {
        SketchJoin::Coincident(a, b) => {
            length(sub(get_point(shapes, a)?, get_point(shapes, b)?)) <= POSITION
        }
        SketchJoin::PointOnCurve {
            point,
            curve,
            parameter,
        } => {
            let shape = *shapes.get(&curve).ok_or_else(invalid)?;
            if curve_at(shape, parameter).is_none() {
                return Err(invalid());
            }
            let point = get_point(shapes, point)?;
            let (contact, _) = finite_curve_contact(shape, point).ok_or_else(invalid)?;
            length(sub(point, contact)) <= POSITION
        }
        SketchJoin::Tangent {
            point,
            source,
            opposite,
        } => {
            let (a, u) = get_endpoint(shapes, point)?;
            let (b, v) = get_endpoint(shapes, source)?;
            length(sub(a, b)) <= POSITION
                && cross(u, v).abs() <= ANGLE
                && (dot(u, v) < 0.) == opposite
        }
    })
}

fn correct(
    shapes: &mut BTreeMap<u64, Shape>,
    join: SketchJoin,
    anchors: &[u64],
) -> Result<(), ModelError> {
    let movable = |reference: SketchPointRef| {
        reference
            .geometry()
            .is_some_and(|id| !anchors.contains(&id))
    };
    match join {
        SketchJoin::Coincident(mut point, mut source) => {
            if !movable(point) {
                std::mem::swap(&mut point, &mut source);
            }
            if movable(point) {
                move_point(shapes, point, get_point(shapes, source)?, None)?;
            }
        }
        SketchJoin::PointOnCurve { point, curve, .. } => {
            let p = get_point(shapes, point)?;
            let shape = *shapes.get(&curve).ok_or_else(invalid)?;
            let (contact, _) = finite_curve_contact(shape, p).ok_or_else(invalid)?;
            if movable(point) {
                move_point(shapes, point, contact, None)?;
            } else if !anchors.contains(&curve) {
                shapes.insert(curve, translate(shape, sub(p, contact))?);
            }
        }
        SketchJoin::Tangent {
            mut point,
            mut source,
            opposite,
        } => {
            if !movable(point) {
                std::mem::swap(&mut point, &mut source);
            }
            if movable(point) {
                let (contact, tangent) = get_endpoint(shapes, source)?;
                move_point(
                    shapes,
                    point,
                    contact,
                    Some(tangent.map(|v| if opposite { -v } else { v })),
                )?;
            }
        }
    }
    Ok(())
}

fn translate(shape: Shape, delta: Point) -> Result<Shape, ModelError> {
    Ok(match shape {
        Shape::Line { start, end } => Shape::Line {
            start: add(start, delta),
            end: add(end, delta),
        },
        Shape::Draw(DrawGeometry::Arc {
            center,
            radial,
            sweep,
        }) => Shape::Draw(DrawGeometry::Arc {
            center: add(center, delta),
            radial,
            sweep,
        }),
        Shape::Draw(DrawGeometry::Ellipse {
            center,
            major,
            minor,
        }) => Shape::Draw(DrawGeometry::Ellipse {
            center: add(center, delta),
            major,
            minor,
        }),
        Shape::Draw(DrawGeometry::Point(point)) => {
            Shape::Draw(DrawGeometry::Point(add(point, delta)))
        }
        _ => return Err(invalid()),
    })
}

fn move_point(
    shapes: &mut BTreeMap<u64, Shape>,
    reference: SketchPointRef,
    target: Point,
    tangent: Option<Point>,
) -> Result<(), ModelError> {
    let key = reference.geometry().ok_or_else(invalid)?;
    let shape = *shapes.get(&key).ok_or_else(invalid)?;
    let original = point_at(shape, reference).ok_or_else(invalid)?;
    let replacement = if let Some(tangent) = tangent {
        let (_, old) = get_endpoint(shapes, reference)?;
        let angle = cross(old, tangent).atan2(dot(old, tangent));
        match shape {
            Shape::Line { start, end } => Shape::Line {
                start: add(target, rotate(sub(start, original), angle)),
                end: add(target, rotate(sub(end, original), angle)),
            },
            Shape::Draw(DrawGeometry::Arc {
                center,
                radial,
                sweep,
            }) => Shape::Draw(DrawGeometry::Arc {
                center: add(target, rotate(sub(center, original), angle)),
                radial: rotate(radial, angle),
                sweep,
            }),
            _ => return Err(invalid()),
        }
    } else if let Shape::Line { start, end } = shape {
        let changed = match reference {
            SketchPointRef::Start(_) => Shape::Line { start: target, end },
            SketchPointRef::End(_) => Shape::Line { start, end: target },
            _ => return Err(invalid()),
        };
        changed.validate().map_or_else(
            |_| translate(shape, sub(target, original)),
            |()| Ok(changed),
        )?
    } else {
        translate(shape, sub(target, original))?
    };
    replacement.validate()?;
    shapes.insert(key, replacement);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Length,
        sketch::{SegmentEquation, SegmentRelation, SketchSupport},
    };

    fn recipe() -> SketchRecipe {
        SketchRecipe {
            support: SketchSupport::XY,
            primitives: vec![
                (
                    0,
                    Shape::Line {
                        start: [0., 0.],
                        end: [5., 0.],
                    },
                ),
                (
                    1,
                    Shape::Draw(DrawGeometry::tangent_arc([5., 0.], [1., 0.], [7., 2.])),
                ),
                (2, Shape::Draw(DrawGeometry::Point([2., 0.]))),
                (
                    3,
                    Shape::Line {
                        start: [3., 20.],
                        end: [4., 20.],
                    },
                ),
            ],
            formulas: Default::default(),
            rigid_formulas: Default::default(),
            corner_formulas: Default::default(),
            pattern_formulas: Default::default(),
            segment_equations: Default::default(),
            driven_dimensions: Default::default(),
            dimension_positions: Default::default(),
            joins: Default::default(),
        }
    }

    #[test]
    fn joined_edit_moves_tangent_branch_and_keeps_unrelated_geometry() -> Result<(), ModelError> {
        let mut recipe = recipe();
        for join in [
            SketchJoin::Coincident(SketchPointRef::Start(1), SketchPointRef::End(0)),
            SketchJoin::Tangent {
                point: SketchPointRef::Start(1),
                source: SketchPointRef::End(0),
                opposite: true,
            },
            SketchJoin::PointOnCurve {
                point: SketchPointRef::Point(2),
                curve: 0,
                parameter: 0.4,
            },
        ] {
            recipe.joins.insert(recipe.next_relation_id()?, join);
        }
        recipe.solve_placement(|| false)?;
        for relation in [
            SegmentRelation::Horizontal,
            SegmentRelation::Length(Length::millimetres(8.)?),
        ] {
            recipe.segment_equations.insert(
                recipe.next_relation_id()?,
                SegmentEquation {
                    segment: 0,
                    relation,
                    formula: None,
                },
            );
        }
        let unrelated = recipe.primitives[3];
        let seed = recipe.clone();
        assert!(recipe.solve_constraints(&[0], || true).is_err());
        assert_eq!(recipe, seed);
        assert!(recipe.solve_constraints(&[0], || false)?.joined);
        assert_eq!(recipe.join_point(SketchPointRef::End(0)), Some([8., 0.]));
        assert_eq!(recipe.join_point(SketchPointRef::Start(1)), Some([8., 0.]));
        assert_eq!(recipe.primitives[3], unrelated);
        assert!(
            matches!(recipe.joins.values().last(), Some(SketchJoin::PointOnCurve { parameter, .. }) if (*parameter-0.25).abs() < 1e-9)
        );
        recipe.validate_joins()?;
        assert!(recipe.protect_dimension_operands(1).is_err());
        recipe.joins.insert(
            recipe.next_relation_id()?,
            SketchJoin::Coincident(SketchPointRef::End(0), SketchPointRef::Origin),
        );
        let prior = recipe.clone();
        assert!(recipe.solve_constraints(&[0], || false).is_err());
        assert_eq!(recipe, prior);
        Ok(())
    }

    #[test]
    fn signed_arc_projection_respects_its_finite_interval() {
        let arc = Shape::Draw(DrawGeometry::Arc {
            center: [0., 0.],
            radial: [1., 0.],
            sweep: -std::f64::consts::FRAC_PI_2,
        });
        let (p, t) = finite_curve_contact(arc, [2., -2.]).unwrap();
        assert!((t - 0.5).abs() < 1e-9);
        assert!((p[0] + p[1]).abs() < 1e-9);
        assert_eq!(finite_curve_contact(arc, [0., 2.]).unwrap().1, 0.);
    }
}
