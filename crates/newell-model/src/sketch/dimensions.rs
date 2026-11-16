// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use super::{Document, DrawGeometry, SketchPrimitive, SketchRecipe};
use crate::{Angle, FeatureId, Length, ModelError, Quantity};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SketchDimensionId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SketchPointRef {
    Origin,
    Point(u64),
    Start(u64),
    End(u64),
    Center(u64),
    Defining(u64, u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistanceBranch {
    Signed,
    Unsigned,
}

/// A driven dimension is evaluated from current analytic geometry on demand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrivenDimension {
    Length(u64),
    Horizontal([SketchPointRef; 2], DistanceBranch),
    Vertical([SketchPointRef; 2], DistanceBranch),
    Distance([SketchPointRef; 2]),
    Angle {
        segment: u64,
        relative_to: Option<u64>,
        branch: DistanceBranch,
    },
    Radius(u64),
    Diameter(u64),
    ArcLength(u64),
}

impl Document {
    pub fn sketch_measurement(
        &self,
        recipe: &SketchRecipe,
        id: SketchDimensionId,
    ) -> Result<Quantity, ModelError> {
        recipe
            .driven_dimensions
            .get(&id)
            .ok_or(ModelError::InvalidSketch("unknown dimension identity"))?
            .measure(self, recipe, None)
    }
}

impl SketchRecipe {
    /// Reject edits whose operand correspondence has not been established.
    pub fn protect_dimension_operands(&self, key: u64) -> Result<(), ModelError> {
        let ids: Vec<_> = self
            .driven_dimensions
            .iter()
            .filter_map(|(id, dimension)| dimension.references(key).then_some(id.0))
            .chain(
                self.segment_equations
                    .iter()
                    .filter_map(|(id, equation)| (equation.segment == key).then_some(id.0)),
            )
            .chain(
                self.joins
                    .iter()
                    .filter_map(|(id, join)| join.keys().contains(&Some(key)).then_some(id.0)),
            )
            .collect();
        if ids.is_empty() {
            Ok(())
        } else {
            Err(ModelError::AffectedSketchDimensions(ids))
        }
    }
}

impl DrivenDimension {
    fn references(self, key: u64) -> bool {
        let point_matches = |point| match point {
            SketchPointRef::Origin => false,
            SketchPointRef::Point(id)
            | SketchPointRef::Start(id)
            | SketchPointRef::End(id)
            | SketchPointRef::Center(id)
            | SketchPointRef::Defining(id, _) => id == key,
        };
        match self {
            Self::Length(id) | Self::Radius(id) | Self::Diameter(id) | Self::ArcLength(id) => {
                id == key
            }
            Self::Horizontal(points, _) | Self::Vertical(points, _) | Self::Distance(points) => {
                points.into_iter().any(point_matches)
            }
            Self::Angle {
                segment,
                relative_to,
                ..
            } => segment == key || relative_to == Some(key),
        }
    }

    pub(super) fn measure(
        self,
        document: &Document,
        recipe: &SketchRecipe,
        replacement: Option<(FeatureId, &SketchRecipe)>,
    ) -> Result<Quantity, ModelError> {
        let primitive = |key| {
            let mut matches = recipe.primitives.iter().filter(|(id, _)| *id == key);
            let &(_, item) = matches
                .next()
                .ok_or(ModelError::InvalidSketch("dimension reference is missing"))?;
            if matches.next().is_some() {
                return Err(ModelError::InvalidSketch(
                    "dimension reference is ambiguous",
                ));
            }
            let item = recipe.resolved(item)?;
            match item {
                SketchPrimitive::Reference { .. }
                | SketchPrimitive::BrepReference { .. }
                | SketchPrimitive::BrepFaceSection { .. } => {
                    document.resolve_sketch_reference_with(recipe.support, item, replacement)
                }
                _ => Ok(item),
            }
        };
        let segment = |key| {
            let (start, end) = primitive(key)?.line().ok_or(ModelError::InvalidSketch(
                "dimension requires a finite segment",
            ))?;
            let delta = difference(start, end);
            if magnitude(delta) < 1e-7 || !magnitude(delta).is_finite() {
                return Err(ModelError::InvalidSketch("degenerate dimension segment"));
            }
            Ok(delta)
        };
        let point = |reference| {
            let Some(key) = reference.geometry() else {
                return Ok([0.0; 2]);
            };
            primitive(key)?.semantic_points(key).into_iter()
                .find_map(|(role, point)| (role == reference).then_some(point))
                .ok_or(ModelError::InvalidSketch("dimension requires a compatible point"))
        };
        let between = |[a, b]: [SketchPointRef; 2]| -> Result<[f64; 2], ModelError> {
            Ok(difference(point(a)?, point(b)?))
        };
        let value = match self {
            Self::Length(key) => magnitude(segment(key)?),
            Self::Horizontal(points, branch) => branch.apply(between(points)?[0]),
            Self::Vertical(points, branch) => branch.apply(between(points)?[1]),
            Self::Distance(points) => magnitude(between(points)?),
            Self::Angle {
                segment: key,
                relative_to,
                branch,
            } => {
                let [x, y] = segment(key)?;
                let [u, v] = relative_to.map(segment).transpose()?.unwrap_or([1.0, 0.0]);
                let (x, y, u, v) = (
                    x / x.hypot(y),
                    y / x.hypot(y),
                    u / u.hypot(v),
                    v / u.hypot(v),
                );
                let radians = (u * y - v * x).atan2(u * x + v * y);
                return Ok(Quantity::Angle(Angle::radians(branch.apply(radians))?));
            }
            Self::Radius(key) | Self::Diameter(key) | Self::ArcLength(key) => {
                let (radius, sweep) = match primitive(key)? {
                    SketchPrimitive::Draw(DrawGeometry::Arc {
                        center,
                        radial,
                        sweep,
                    }) => (magnitude(difference(center, radial)), Some(sweep)),
                    SketchPrimitive::Draw(DrawGeometry::Ellipse { major, minor, .. })
                        if (magnitude(major) - minor).abs() <= 1e-12 * minor =>
                    {
                        (minor, None)
                    }
                    _ => {
                        return Err(ModelError::InvalidSketch(
                            "dimension requires circular geometry",
                        ));
                    }
                };
                match self {
                    Self::Diameter(_) => 2.0 * radius,
                    Self::ArcLength(_) => {
                        radius
                            * sweep
                                .ok_or(ModelError::InvalidSketch(
                                    "arc length requires a circular arc",
                                ))?
                                .abs()
                    }
                    _ => radius,
                }
            }
        };
        Ok(Quantity::Length(Length::millimetres(value)?))
    }
}

impl DistanceBranch {
    fn apply(self, value: f64) -> f64 {
        match self {
            Self::Signed => value,
            Self::Unsigned => value.abs(),
        }
    }
}

fn difference(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [b[0] - a[0], b[1] - a[1]]
}

fn magnitude([x, y]: [f64; 2]) -> f64 {
    x.hypot(y)
}

/// Equations supported by the independent finite-segment solver.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SegmentRelation {
    Horizontal,
    Vertical,
    Length(crate::Length),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SegmentEquation {
    pub segment: u64,
    pub relation: SegmentRelation,
    pub formula: Option<crate::formula::ParameterId>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SegmentSolveReport {
    pub joined: bool,
    pub rank: usize,
    pub degrees_of_freedom: usize,
    pub redundant_equations: usize,
}

impl SketchRecipe {
    /// Solve the independent segment components without mutating the accepted seed on failure.
    pub fn solve_segment_equations(
        &mut self,
        cancelled: impl Fn() -> bool,
    ) -> Result<SegmentSolveReport, ModelError> {
        self.solve_segment_equations_where(|_| true, cancelled)
    }

    pub(super) fn solve_segment_equations_where(
        &mut self,
        included: impl Fn(u64) -> bool,
        cancelled: impl Fn() -> bool,
    ) -> Result<SegmentSolveReport, ModelError> {
        use std::collections::BTreeMap;

        let mut components = BTreeMap::<_, Vec<_>>::new();
        for equation in self
            .segment_equations
            .values()
            .filter(|equation| included(equation.segment))
        {
            components
                .entry(equation.segment)
                .or_default()
                .push(equation.relation);
        }
        let mut solved = Vec::with_capacity(components.len());
        let mut report = SegmentSolveReport::default();
        for (key, equations) in components {
            if cancelled() {
                return Err(ModelError::InvalidSketch("solve cancelled"));
            }
            let (index, start, end) = self.equation_segment(key)?;
            let mut horizontal = false;
            let mut vertical = false;
            let mut length: Option<f64> = None;
            let equation_count = equations.len();
            for equation in equations {
                match equation {
                    SegmentRelation::Horizontal => horizontal = true,
                    SegmentRelation::Vertical => vertical = true,
                    SegmentRelation::Length(value) => {
                        let value = Quantity::Length(value).value_in(crate::Unit::Millimetre)?;
                        if value < 1e-7 {
                            return Err(ModelError::InvalidSketch(
                                "segment length must be positive and above tolerance",
                            ));
                        }
                        if length.is_some_and(|previous| (previous - value).abs() > 1e-7) {
                            return Err(ModelError::InvalidSketch("inconsistent segment lengths"));
                        }
                        length = Some(value);
                    }
                }
            }
            if horizontal && vertical {
                return Err(ModelError::InvalidSketch(
                    "horizontal and vertical conflict on a finite segment",
                ));
            }
            let delta = difference(start, end);
            let old_length = magnitude(delta);
            let mut direction = delta.map(|value| value / old_length);
            if horizontal {
                direction = [1.0_f64.copysign(delta[0]), 0.0];
            }
            if vertical {
                direction = [0.0, 1.0_f64.copysign(delta[1])];
            }
            let wanted = length.unwrap_or(old_length);
            let end = std::array::from_fn(|axis| start[axis] + direction[axis] * wanted);
            let item = SketchPrimitive::Line { start, end };
            item.validate()?;
            if (magnitude(difference(start, end)) - wanted).abs() > 1e-7 {
                return Err(ModelError::InvalidSketch(
                    "segment solution exceeds coordinate precision",
                ));
            }
            let rank = usize::from(horizontal || vertical) + usize::from(length.is_some());
            report.rank += rank;
            report.degrees_of_freedom += 4 - rank;
            report.redundant_equations += equation_count - rank;
            solved.push((index, item));
        }
        if cancelled() {
            return Err(ModelError::InvalidSketch("solve cancelled"));
        }
        for (index, item) in solved {
            self.primitives[index].1 = item;
        }
        Ok(report)
    }

    fn equation_segment(&self, key: u64) -> Result<(usize, [f64; 2], [f64; 2]), ModelError> {
        self.primitives
            .iter()
            .enumerate()
            .find_map(|(index, &(id, item))| match item {
                SketchPrimitive::Line { start, end } if id == key => Some((index, start, end)),
                _ => None,
            })
            .filter(|(_, start, end)| {
                self.corner_formula_for(key).is_none()
                    && !self.rigid_formulas.contains_key(&key)
                    && magnitude(difference(*start, *end)).is_finite()
                    && magnitude(difference(*start, *end)) >= 1e-7
            })
            .ok_or(ModelError::InvalidSketch(
                "equation requires an independent finite segment",
            ))
    }

    pub(super) fn validate_segment_equations(&self) -> Result<(), ModelError> {
        for equation in self.segment_equations.values() {
            let (_, start, end) = self.equation_segment(equation.segment)?;
            let delta = difference(start, end);
            let residual = match equation.relation {
                SegmentRelation::Horizontal => delta[1],
                SegmentRelation::Vertical => delta[0],
                SegmentRelation::Length(value) => {
                    let target = Quantity::Length(value).value_in(crate::Unit::Millimetre)?;
                    if target < 1e-7 {
                        return Err(ModelError::InvalidSketch(
                            "segment length must be positive and above tolerance",
                        ));
                    }
                    magnitude(delta) - target
                }
            };
            if residual.abs() > 1e-7 {
                return Err(ModelError::InvalidSketch(
                    "segment equation requires a new solution",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Feature, Unit, sketch::SketchSupport};

    fn fixture() -> SketchRecipe {
        SketchRecipe {
            support: SketchSupport::XY,
            primitives: vec![
                (
                    1,
                    SketchPrimitive::Line {
                        start: [0.0, 0.0],
                        end: [3.0, 4.0],
                    },
                ),
                (
                    2,
                    SketchPrimitive::Draw(DrawGeometry::circle([2.0, 0.0], [4.0, 0.0])),
                ),
                (
                    3,
                    SketchPrimitive::Draw(DrawGeometry::Arc {
                        center: [0.0, 0.0],
                        radial: [2.0, 0.0],
                        sweep: std::f64::consts::FRAC_PI_2,
                    }),
                ),
            ],
            driven_dimensions: Default::default(),
            segment_equations: Default::default(),
            joins: Default::default(),
            dimension_positions: Default::default(),
            formulas: Default::default(),
            pattern_formulas: Default::default(),
            rigid_formulas: Default::default(),
            corner_formulas: Default::default(),
        }
    }

    #[test]
    fn segment_equations_change_geometry_and_reject_conflicts_atomically() -> Result<(), ModelError>
    {
        let mut recipe = fixture();
        let length = SketchDimensionId(0);
        let horizontal = SketchDimensionId(1);
        let vertical = SketchDimensionId(2);
        recipe.segment_equations.insert(
            length,
            SegmentEquation {
                segment: 1,
                relation: SegmentRelation::Length(Length::millimetres(8.0)?),
                formula: None,
            },
        );
        recipe.segment_equations.insert(
            horizontal,
            SegmentEquation {
                segment: 1,
                relation: SegmentRelation::Horizontal,
                formula: None,
            },
        );
        let report = recipe.solve_segment_equations(|| false)?;
        assert_eq!(
            report,
            SegmentSolveReport {
                joined: false,
                rank: 2,
                degrees_of_freedom: 2,
                redundant_equations: 0
            }
        );
        assert_eq!(
            recipe.primitives[0].1.line(),
            Some(([0.0, 0.0], [8.0, 0.0]))
        );
        let mut document = Document::default();
        let feature = document.insert_sketch(None, recipe.clone())?;
        let saved = recipe.primitives.clone();
        recipe.segment_equations.insert(
            vertical,
            SegmentEquation {
                segment: 1,
                relation: SegmentRelation::Vertical,
                formula: None,
            },
        );
        assert!(recipe.solve_segment_equations(|| false).is_err());
        assert_eq!(recipe.primitives, saved);
        assert!(document.replace_sketch(feature, recipe.clone()).is_err());
        recipe.segment_equations.remove(&vertical);
        assert!(recipe.solve_segment_equations(|| true).is_err());
        assert_eq!(recipe.primitives, saved);
        Ok(())
    }

    #[test]
    fn driving_length_formula_replays_and_rejects_invalid_updates() -> Result<(), ModelError> {
        use crate::parameters::{ParameterDraftOwner, ParameterEdit, ParameterOwner};

        let mut doc = Document::default();
        doc.set_parameter("Width", Length::millimetres(5.0)?)?;
        let mut draft = doc.begin_parameter_draft(ParameterDraftOwner::Document)?;
        let parameter = draft.stage_edit(ParameterEdit {
            parameter: None,
            owner: ParameterOwner::Document,
            source: "2 * #Width".into(),
            destination: crate::QuantityKind::Length,
            source_edit: None,
            source_origins: None,
        })?;
        let mut recipe = fixture();
        let id = SketchDimensionId(1);
        recipe.segment_equations.insert(
            id,
            SegmentEquation {
                segment: 1,
                relation: SegmentRelation::Length(Length::millimetres(10.0)?),
                formula: Some(parameter),
            },
        );
        recipe.solve_segment_equations(|| false)?;
        doc.commit_sketch_with_formulas(None, None, recipe, &draft)?;
        doc.set_parameter("Width", Length::millimetres(7.0)?)?;
        let Feature::Sketch { recipe, .. } = &doc.features()[0].1 else {
            return Err(ModelError::InvalidSketch("missing committed sketch"));
        };
        assert_eq!(recipe.segment_equations[&id].formula, Some(parameter));
        let (start, end) = recipe.primitives[0]
            .1
            .line()
            .ok_or(ModelError::InvalidSketch("missing replayed segment"))?;
        assert!((magnitude(difference(start, end)) - 14.0).abs() < 1e-10);
        let accepted = doc.clone();
        assert!(
            doc.set_parameter("Width", Length::millimetres(0.0)?)
                .is_err()
        );
        assert_eq!(doc, accepted);
        Ok(())
    }

    #[test]
    fn driven_dimensions_measure_without_editing_geometry() -> Result<(), ModelError> {
        use DistanceBranch::{Signed, Unsigned};
        use DrivenDimension::*;
        use SketchPointRef::{End, Origin};
        let mut recipe = fixture();
        let points = [End(1), Origin];
        let measures = [
            (Length(1), Unit::Millimetre, 5.0),
            (Horizontal(points, Signed), Unit::Millimetre, -3.0),
            (Vertical(points, Unsigned), Unit::Millimetre, 4.0),
            (Distance(points), Unit::Millimetre, 5.0),
            (
                Angle {
                    segment: 1,
                    relative_to: None,
                    branch: Signed,
                },
                Unit::Degree,
                4.0_f64.atan2(3.0).to_degrees(),
            ),
            (Radius(2), Unit::Millimetre, 2.0),
            (Diameter(2), Unit::Millimetre, 4.0),
            (ArcLength(3), Unit::Millimetre, std::f64::consts::PI),
        ];
        let mut doc = Document::default();
        for (id, (dimension, _, _)) in (1..).zip(measures) {
            recipe
                .driven_dimensions
                .insert(SketchDimensionId(id), dimension);
        }
        let feature = doc.insert_sketch(None, recipe.clone()).unwrap();
        let Some(Feature::Sketch { recipe: saved, .. }) = doc.feature(feature) else {
            return Err(ModelError::InvalidSketch("missing accepted sketch"));
        };
        assert_eq!(saved, &recipe);
        for (id, (_, unit, expected)) in (1..).zip(measures) {
            let value = doc
                .sketch_measurement(saved, SketchDimensionId(id))
                .unwrap()
                .value_in(unit)
                .unwrap();
            assert!((value - expected).abs() < 1e-12);
        }
        Ok(())
    }

    #[test]
    fn dimension_types_reject_invalid_operands_atomically() {
        use DistanceBranch::Signed;
        use DrivenDimension::*;
        use SketchPointRef::{Origin, Point};
        let invalid = [
            Length(2),
            Horizontal([Origin, Point(1)], Signed),
            Vertical([Origin, Point(1)], Signed),
            Distance([Origin, Point(1)]),
            Angle {
                segment: 1,
                relative_to: Some(2),
                branch: Signed,
            },
            Radius(1),
            Diameter(1),
            ArcLength(2),
        ];
        let mut doc = Document::default();
        for dimension in invalid {
            let mut recipe = fixture();
            recipe
                .driven_dimensions
                .insert(SketchDimensionId(4), dimension);
            assert!(doc.insert_sketch(None, recipe).is_err(), "{dimension:?}");
        }
        assert!(doc.features().is_empty());
    }

    #[test]
    fn replay_resolves_live_link_and_rejects_incompatible_replacement() {
        use super::super::{ReferenceMode, SketchSupport};
        let mut doc = Document::default();
        let source_recipe = fixture();
        let source = doc.insert_sketch(None, source_recipe.clone()).unwrap();
        let mut downstream = fixture();
        downstream.primitives = vec![(
            12,
            SketchPrimitive::Reference {
                feature: source,
                element: 1,
                mode: ReferenceMode::Projected,
            },
        )];
        let id = SketchDimensionId(32);
        downstream
            .driven_dimensions
            .insert(id, DrivenDimension::Length(12));
        doc.insert_sketch(None, downstream.clone()).unwrap();

        let mut moved = source_recipe;
        moved.primitives[0].1 = SketchPrimitive::Line {
            start: [0.0, 0.0],
            end: [0.0, 8.0],
        };
        doc.replace_sketch(source, moved.clone()).unwrap();
        assert_eq!(
            doc.sketch_measurement(&downstream, id)
                .unwrap()
                .value_in(Unit::Millimetre),
            Ok(8.0)
        );

        moved.primitives[0].1 = SketchPrimitive::Draw(DrawGeometry::Point([0.0, 0.0]));
        assert!(doc.replace_sketch(source, moved).is_err());
        assert_eq!(downstream.support, SketchSupport::XY);
        assert_eq!(
            doc.sketch_measurement(&downstream, id)
                .unwrap()
                .value_in(Unit::Millimetre),
            Ok(8.0)
        );
    }
}
