// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Principal-plane Sketch recipes, independent of the rendering and geometry kernels.

mod curves;
mod dimensions;
mod joins;
pub use joins::{SketchJoin, add, cross, curve_at, dot, finite_curve_contact, sub, unit};
mod text;
pub use curves::{DrawGeometry, LineEditMode, LineOperation, line_corner, offset_line_chain};
pub use dimensions::{
    DistanceBranch, DrivenDimension, SegmentEquation, SegmentRelation, SegmentSolveReport,
    SketchDimensionId, SketchPointRef,
};
pub use text::{SketchText, TextEdge};

use crate::{Document, Feature, FeatureId, HistoryLayoutRow, HistoryNodeId, ModelError};

pub const MAX_PRIMITIVES: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SketchSupport {
    XY,
    XZ,
    YZ,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceMode {
    Projected,
    Intersect,
}

impl SketchSupport {
    fn world(self, p: [f64; 2]) -> [f64; 3] {
        match self {
            Self::XY => [p[0], p[1], 0.0],
            Self::XZ => [p[0], 0.0, p[1]],
            Self::YZ => [0.0, p[0], p[1]],
        }
    }

    fn local(self, [x, y, z]: [f64; 3]) -> ([f64; 2], f64) {
        match self {
            Self::XY => ([x, y], z),
            Self::XZ => ([x, z], -y),
            Self::YZ => ([y, z], x),
        }
    }
}

#[derive(Clone, Copy)]
struct Similarity {
    origin: [f64; 2],
    translation: [f64; 2],
    radians: f64,
    scale: f64,
    reflected: bool,
}

impl Similarity {
    fn vector(self, [x, y]: [f64; 2]) -> [f64; 2] {
        let (sin, cos) = self.radians.sin_cos();
        let y = if self.reflected { -y } else { y };
        [
            self.scale * (x * cos - y * sin),
            self.scale * (x * sin + y * cos),
        ]
    }

    fn point(self, point: [f64; 2]) -> [f64; 2] {
        let moved = self.vector([point[0] - self.origin[0], point[1] - self.origin[1]]);
        [
            self.translation[0] + moved[0],
            self.translation[1] + moved[1],
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PatternStep {
    Linear([f64; 2]),
    Mirror([[f64; 2]; 2]),
    Circular { center: [f64; 2], radians: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SketchPrimitive {
    BrepFaceSection {
        feature: FeatureId,
        face: crate::geometry::FaceRef,
        interval: u32,
    },
    BrepReference {
        feature: FeatureId,
        edge: crate::geometry::DirectionRef,
        mode: ReferenceMode,
        crossing: u8,
    },
    Reference {
        feature: FeatureId,
        element: u64,
        mode: ReferenceMode,
    },
    PatternInstance {
        master: u64,
        pattern: u64,
        placement: u16,
        step: PatternStep,
    },
    Draw(DrawGeometry),
    EditedLine {
        start: [f64; 2],
        end: [f64; 2],
        source: u64,
        operation: LineOperation,
    },
    Line {
        start: [f64; 2],
        end: [f64; 2],
    },
    Rectangle {
        min: [f64; 2],
        max: [f64; 2],
    },
}

impl SketchPrimitive {
    pub fn line(self) -> Option<([f64; 2], [f64; 2])> {
        match self {
            Self::Line { start, end } | Self::EditedLine { start, end, .. } => Some((start, end)),
            _ => None,
        }
    }

    pub fn endpoint_tangents(self) -> Option<[([f64; 2], [f64; 2]); 2]> {
        if let Some((a, b)) = self.line() {
            let delta = [b[0] - a[0], b[1] - a[1]];
            Some([(a, delta.map(|v| -v)), (b, delta)])
        } else if let Self::Draw(draw) = self {
            draw.endpoint_tangents()
        } else {
            None
        }
    }

    /// Enumerate actual handles, independent of display tessellation and equation eligibility.
    pub fn semantic_points(self, key: u64) -> Vec<(SketchPointRef, [f64; 2])> {
        use SketchPointRef::{Center, Defining, End, Point, Start};
        let mut points = self.endpoint_tangents().map_or_else(Vec::new, |[a, b]| {
            vec![(Start(key), a.0), (End(key), b.0)]
        });
        match self {
            Self::Rectangle { min, max } => points.extend([
                (Defining(key, 0), min),
                (Defining(key, 1), [max[0], min[1]]),
                (Defining(key, 2), max),
                (Defining(key, 3), [min[0], max[1]]),
            ]),
            Self::Draw(DrawGeometry::Point(point)) => points.push((Point(key), point)),
            Self::Draw(
                DrawGeometry::Arc { center, .. }
                | DrawGeometry::EllipticalArc { center, .. }
                | DrawGeometry::Ellipse { center, .. },
            ) => points.push((Center(key), center)),
            Self::Draw(DrawGeometry::Polygon { center, vertex, .. }) => {
                points.extend([(Center(key), center), (Defining(key, 0), vertex)]);
            }
            Self::Draw(DrawGeometry::OrientedRectangle { start, end, width }) => {
                let [dx, dy] = joins::sub(end, start);
                let normal = joins::unit([-dy, dx]).unwrap_or([0.0; 2]);
                let offset = normal.map(|component| component * width);
                points.extend([
                    (Defining(key, 0), start),
                    (Defining(key, 1), end),
                    (Defining(key, 2), joins::add(end, offset)),
                    (Defining(key, 3), joins::add(start, offset)),
                ]);
            }
            Self::Draw(DrawGeometry::Slot { start, end, .. }) => {
                points.extend([(Start(key), start), (End(key), end)]);
            }
            Self::Draw(DrawGeometry::Text(text)) => {
                points.push((Start(key), text.placement_frame().0));
            }
            _ => {}
        }
        points
    }

    pub fn is_independent(self) -> bool {
        !matches!(
            self,
            Self::PatternInstance { .. }
                | Self::Reference { .. }
                | Self::BrepReference { .. }
                | Self::BrepFaceSection { .. }
        )
    }

    fn transformed(self, transform: Similarity) -> Result<Self, ModelError> {
        let item = match self {
            Self::PatternInstance { .. }
            | Self::Reference { .. }
            | Self::BrepReference { .. }
            | Self::BrepFaceSection { .. } => {
                return Err(ModelError::InvalidSketch("select independent geometry"));
            }
            Self::Draw(draw) => Self::Draw(draw.transformed(transform)),
            Self::Line { start, end } | Self::EditedLine { start, end, .. } => Self::Line {
                start: transform.point(start),
                end: transform.point(end),
            },
            Self::Rectangle { min, max } => Self::Draw(DrawGeometry::OrientedRectangle {
                start: transform.point(min),
                end: transform.point([max[0], min[1]]),
                width: (max[1] - min[1])
                    * transform.scale
                    * if transform.reflected { -1.0 } else { 1.0 },
            }),
        };
        item.validate()?;
        Ok(item)
    }

    pub fn placed(self, destination: [f64; 2], handle: [f64; 2]) -> Result<Self, ModelError> {
        let frame = if let Some(line) = self.line() {
            line
        } else if let Self::Draw(draw) = self {
            draw.placement_frame()
        } else if let Self::Rectangle { min, max } = self {
            (min, [max[0], min[1]])
        } else {
            return Err(ModelError::InvalidSketch("select independent geometry"));
        };
        let (operation, _) = LineOperation::transform(frame, frame.0, destination, handle)?;
        let LineOperation::Transform {
            origin,
            destination,
            radians,
            scale,
        } = operation
        else {
            return Err(ModelError::InvalidSketch("invalid transform"));
        };
        self.transformed(Similarity {
            origin,
            translation: destination,
            radians,
            scale,
            reflected: false,
        })
    }

    pub fn mirrored(self, axis: [[f64; 2]; 2]) -> Result<Self, ModelError> {
        let x = axis[1][0] - axis[0][0];
        let y = axis[1][1] - axis[0][1];
        if !x.hypot(y).is_finite() || x.hypot(y) < 1e-7 {
            return Err(ModelError::InvalidSketch(
                "two distinct mirror axis points required",
            ));
        }
        self.transformed(Similarity {
            origin: axis[0],
            translation: axis[0],
            radians: 2.0 * y.atan2(x),
            scale: 1.0,
            reflected: true,
        })
    }

    pub fn validate(self) -> Result<(), ModelError> {
        let valid = match self {
            Self::PatternInstance { .. }
            | Self::Reference { .. }
            | Self::BrepReference { .. }
            | Self::BrepFaceSection { .. } => false,
            Self::Draw(draw) => return draw.validate(),
            Self::Line { start, end } | Self::EditedLine { start, end, .. } => {
                let span = (end[0] - start[0]).hypot(end[1] - start[1]);
                span.is_finite() && span >= 1e-7_f64.max(1e-12 * span)
            }
            Self::Rectangle { min, max } => {
                let spans = [max[0] - min[0], max[1] - min[1]];
                let threshold = 1e-7_f64.max(1e-12 * spans[0].max(spans[1]));
                spans
                    .into_iter()
                    .all(|span| span.is_finite() && span >= threshold)
            }
        };
        if valid {
            Ok(())
        } else {
            Err(ModelError::InvalidSketch("primitive"))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatternFormula {
    pub parameter: crate::formula::ParameterId,
    pub master: u64,
    pub step: PatternStep,
    pub excluded: std::collections::BTreeSet<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigidFormula {
    pub parameter: crate::formula::ParameterId,
    pub original: SketchPrimitive,
    pub translation: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerFormula {
    pub parameters: [Option<crate::formula::ParameterId>; 2],
    pub values: [f64; 2],
    pub sources: [u64; 2],
    pub segments: [([f64; 2], [f64; 2]); 2],
    pub fillet: bool,
}

/// Count includes the master; reserve all possible instance keys before publication.
pub const PATTERN_KEY_SPAN: u64 = 255;

#[derive(Clone, Debug, PartialEq)]
pub struct SketchRecipe {
    pub support: SketchSupport,
    pub primitives: Vec<(u64, SketchPrimitive)>,
    /// Stable parameter definition for each formula-driven primitive.
    pub formulas: std::collections::BTreeMap<u64, crate::formula::ParameterId>,
    pub pattern_formulas: std::collections::BTreeMap<u64, PatternFormula>,
    pub rigid_formulas: std::collections::BTreeMap<u64, RigidFormula>,
    pub dimension_positions: std::collections::BTreeMap<SketchDimensionId, [f64; 2]>,
    pub joins: Box<std::collections::BTreeMap<SketchDimensionId, SketchJoin>>,
    pub segment_equations: std::collections::BTreeMap<SketchDimensionId, SegmentEquation>,
    pub driven_dimensions: std::collections::BTreeMap<SketchDimensionId, DrivenDimension>,
    pub corner_formulas: std::collections::BTreeMap<u64, CornerFormula>,
}

impl SketchRecipe {
    pub fn formula_ids(&self) -> impl Iterator<Item = crate::formula::ParameterId> + '_ {
        self.formulas
            .values()
            .copied()
            .chain(self.pattern_formulas.values().map(|p| p.parameter))
            .chain(self.rigid_formulas.values().map(|p| p.parameter))
            .chain(
                self.segment_equations
                    .values()
                    .filter_map(|equation| equation.formula),
            )
            .chain(
                self.corner_formulas
                    .values()
                    .flat_map(|p| p.parameters.into_iter().flatten()),
            )
    }

    pub fn can_move(&self, key: u64) -> bool {
        !self.formulas.contains_key(&key)
            && !self.rigid_formulas.contains_key(&key)
            && self.corner_formula_for(key).is_none()
    }

    pub fn move_primitive(&mut self, key: u64, item: SketchPrimitive) -> Result<(), ModelError> {
        if !self.can_move(key) {
            return Err(ModelError::InvalidSketch(
                "detach the Formula before moving this result",
            ));
        }
        let mut candidate = self.clone();
        let current = candidate
            .primitives
            .iter_mut()
            .find(|(id, _)| *id == key)
            .ok_or(ModelError::InvalidSketch("missing transform source"))?;
        current.1 = item;
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    pub fn corner_formula_for(&self, key: u64) -> Option<u64> {
        self.corner_formulas.iter().find_map(|(&result, formula)| {
            (result == key || formula.sources.contains(&key)).then_some(result)
        })
    }

    pub fn exclude_placement(&mut self, item: SketchPrimitive) {
        if let SketchPrimitive::PatternInstance {
            pattern, placement, ..
        } = item
            && let Some(formula) = self.pattern_formulas.get_mut(&pattern)
        {
            formula.excluded.insert(placement);
        }
    }

    pub fn evaluate_formulas(
        &mut self,
        registry: &crate::parameters::ParameterRegistry,
    ) -> Result<(), ModelError> {
        if self.formula_ids().next().is_none() {
            return Ok(());
        }
        let mut evaluated = self.primitives.clone();
        for (&key, &id) in &self.formulas {
            let definition = registry.definition(id).ok_or(ModelError::InvalidSketch(
                "missing Sketch Formula definition",
            ))?;
            let item = evaluated
                .iter_mut()
                .find_map(|(item_key, item)| (*item_key == key).then_some(item))
                .ok_or(ModelError::InvalidSketch(
                    "missing Formula-driven primitive",
                ))?;
            match item {
                SketchPrimitive::Draw(DrawGeometry::Polygon { sides, .. }) => {
                    let count = definition.value.value_in(crate::Unit::Unitless)?;
                    if !(3.0..=256.0).contains(&count) || count.fract() != 0.0 {
                        return Err(ModelError::InvalidSketch(
                            "side count must be an integer from 3 to 256",
                        ));
                    }
                    *sides = count as u16;
                }
                SketchPrimitive::Draw(DrawGeometry::Conic { weight, .. }) => {
                    *weight = definition.value.value_in(crate::Unit::Unitless)?;
                }
                SketchPrimitive::Draw(DrawGeometry::Text(text)) => {
                    *text =
                        text.with_height(definition.value.value_in(crate::Unit::Millimetre)?)?;
                }
                _ => {
                    return Err(ModelError::InvalidSketch(
                        "unsupported Formula-driven primitive",
                    ));
                }
            }
            item.validate()?;
        }
        let quantity = |id| {
            registry
                .definition(id)
                .map(|definition| definition.value)
                .ok_or(ModelError::InvalidSketch(
                    "missing Sketch Formula definition",
                ))
        };
        for (&key, formula) in &self.rigid_formulas {
            let item = evaluated
                .iter_mut()
                .find_map(|(id, item)| (*id == key).then_some(item))
                .ok_or(ModelError::InvalidSketch("missing Formula copy"))?;
            *item = formula.original.transformed(Similarity {
                origin: [0.0; 2],
                translation: formula.translation,
                radians: quantity(formula.parameter)?
                    .value_in(crate::Unit::Degree)?
                    .to_radians(),
                scale: 1.0,
                reflected: false,
            })?;
        }
        for (&result, formula) in &self.corner_formulas {
            let mut values = formula.values;
            for (value, binding) in values.iter_mut().zip(formula.parameters) {
                if let Some(id) = binding {
                    *value = quantity(id)?.value_in(crate::Unit::Millimetre)?;
                }
            }
            let ([first, second], join) = line_corner(
                formula.segments[0],
                formula.segments[1],
                values,
                formula.fillet,
            )?;
            for (key, geometry) in [
                (
                    formula.sources[0],
                    SketchPrimitive::Line {
                        start: first.0,
                        end: first.1,
                    },
                ),
                (
                    formula.sources[1],
                    SketchPrimitive::Line {
                        start: second.0,
                        end: second.1,
                    },
                ),
                (result, join),
            ] {
                let item = evaluated
                    .iter_mut()
                    .find_map(|(id, item)| (*id == key).then_some(item))
                    .ok_or(ModelError::InvalidSketch("missing Formula corner result"))?;
                *item = geometry;
            }
        }
        if !self.pattern_formulas.is_empty() {
            evaluated.retain(|(_, item)| {
                !matches!(
                    item, SketchPrimitive::PatternInstance { pattern, .. }
                        if self.pattern_formulas.contains_key(pattern)
                )
            });
            for (&pattern, formula) in &self.pattern_formulas {
                let count = registry
                    .definition(formula.parameter)
                    .ok_or(ModelError::InvalidSketch("missing pattern count Formula"))?
                    .value
                    .value_in(crate::Unit::Unitless)?;
                if !(2.0..=256.0).contains(&count)
                    || count.fract() != 0.0
                    || matches!(formula.step, PatternStep::Circular { radians, .. }
                        if radians.abs() * (count - 1.0) >= std::f64::consts::TAU)
                {
                    return Err(ModelError::InvalidSketch(
                        "Formula gives an invalid pattern count",
                    ));
                }
                for placement in (1..count as u16).filter(|p| !formula.excluded.contains(p)) {
                    let key = pattern
                        .checked_add(u64::from(placement) - 1)
                        .ok_or(ModelError::IdentityExhausted)?;
                    let item = SketchPrimitive::PatternInstance {
                        master: formula.master,
                        pattern,
                        placement,
                        step: formula.step,
                    };
                    if evaluated.len() >= MAX_PRIMITIVES {
                        return Err(ModelError::InvalidSketch(
                            "Formula exceeds the Sketch capacity",
                        ));
                    }
                    evaluated.push((key, item));
                }
            }
            evaluated.sort_by_key(|(key, _)| *key);
            if evaluated.len() > MAX_PRIMITIVES || evaluated.windows(2).any(|w| w[0].0 == w[1].0) {
                return Err(ModelError::InvalidSketch(
                    "Formula exceeds the Sketch capacity",
                ));
            }
        }
        let mut candidate = self.clone();
        candidate.primitives = evaluated;
        let mut changed_equations = Vec::new();
        for equation in candidate.segment_equations.values_mut() {
            let Some(id) = equation.formula else {
                continue;
            };
            let value = registry
                .definition(id)
                .ok_or(ModelError::InvalidSketch("missing driving length Formula"))?
                .value;
            let crate::Quantity::Length(length) = value else {
                return Err(ModelError::DimensionMismatch);
            };
            let SegmentRelation::Length(previous) = &mut equation.relation else {
                return Err(ModelError::InvalidSketch(
                    "Formula requires a length equation",
                ));
            };
            if *previous != length {
                changed_equations.push(equation.segment);
            }
            *previous = length;
        }
        candidate.validate()?;
        if !changed_equations.is_empty() {
            candidate.solve_constraints(&changed_equations, || false)?;
        }
        *self = candidate;
        Ok(())
    }

    pub fn resolved(&self, item: SketchPrimitive) -> Result<SketchPrimitive, ModelError> {
        let SketchPrimitive::PatternInstance {
            master,
            placement,
            step,
            ..
        } = item
        else {
            return Ok(item);
        };
        let source = self
            .primitives
            .iter()
            .find_map(|&(key, item)| (key == master).then_some(item))
            .ok_or(ModelError::InvalidSketch("missing pattern master"))?;
        let n = f64::from(placement);
        let (origin, translation, radians) = match step {
            PatternStep::Mirror(axis) => return source.mirrored(axis),
            PatternStep::Linear(delta) => ([0.0; 2], delta.map(|d| n * d), 0.0),
            PatternStep::Circular { center, radians } => (center, center, n * radians),
        };
        source.transformed(Similarity {
            origin,
            translation,
            radians,
            scale: 1.0,
            reflected: false,
        })
    }

    pub fn has_instances(&self, source: u64) -> bool {
        self.pattern_formulas
            .values()
            .any(|formula| formula.master == source)
            || self.primitives.iter().any(|&(_, item)| {
                matches!(
                    item, SketchPrimitive::PatternInstance { master, .. } if master == source
                )
            })
    }

    /// Copy into an identical principal frame, with an explicit rigid transform.
    pub fn rigid_copy(
        &self,
        support: SketchSupport,
        shift: [f64; 2],
        radians: f64,
    ) -> Result<Vec<SketchPrimitive>, ModelError> {
        if self.support != support {
            return Err(ModelError::InvalidSketch(
                "Insert requires the same support plane",
            ));
        }
        self.primitives
            .iter()
            .map(|&(_, item)| {
                self.resolved(item)?.transformed(Similarity {
                    origin: [0.0; 2],
                    translation: shift,
                    radians,
                    scale: 1.0,
                    reflected: false,
                })
            })
            .collect()
    }

    fn validate(&self) -> Result<(), ModelError> {
        if self.primitives.is_empty() || self.primitives.len() > MAX_PRIMITIVES {
            return Err(ModelError::InvalidSketch("primitive count"));
        }
        let mut keys = std::collections::BTreeSet::new();
        let mut placements = std::collections::BTreeSet::new();
        for &(key, primitive) in &self.primitives {
            if !keys.insert(key) {
                return Err(ModelError::InvalidSketch("primitive identity"));
            }
            if let SketchPrimitive::PatternInstance {
                master,
                pattern,
                placement,
                step,
            } = primitive
            {
                let valid_step = match step {
                    PatternStep::Mirror(_) => placement == 1,
                    PatternStep::Linear([x, y]) => x.hypot(y).is_finite() && x.hypot(y) >= 1e-7,
                    PatternStep::Circular { radians, .. } => {
                        radians.is_finite()
                            && radians.abs() >= 1e-12
                            && radians.abs() * f64::from(placement) < std::f64::consts::TAU
                    }
                };
                if master == key
                    || !keys.contains(&master)
                    || placement == 0
                    || !valid_step
                    || !placements.insert((pattern, placement))
                {
                    return Err(ModelError::InvalidSketch("invalid pattern placement"));
                }
            }
            if !matches!(
                primitive,
                SketchPrimitive::Reference { .. }
                    | SketchPrimitive::BrepReference { .. }
                    | SketchPrimitive::BrepFaceSection { .. }
            ) {
                self.resolved(primitive)?.validate()?;
            }
        }
        Ok(())
    }
}

impl Document {
    /// Recompute against the candidate registry before it can become visible.
    pub(crate) fn reevaluate_sketches(&mut self) -> Result<(), ModelError> {
        for (_, feature) in &mut self.features {
            if let Feature::Sketch { recipe, .. } = feature {
                recipe.evaluate_formulas(&self.parameter_registry)?;
            }
        }
        self.validate_replayed_sketch_references()
    }

    pub fn commit_sketch_with_formulas(
        &mut self,
        before: Option<FeatureId>,
        editing: Option<FeatureId>,
        mut recipe: SketchRecipe,
        parameters: &crate::parameters::ParameterDraft,
    ) -> Result<(), ModelError> {
        parameters.validate_source(self)?;
        let mut candidate = self.clone();
        let active = recipe.formula_ids().collect();
        candidate.parameter_registry = parameters.prepared_registry(&active)?;
        for (_, feature) in &self.features {
            if let Feature::Extrude(extrude) = feature
                && crate::geometry::bindings(&extrude.spec).iter().any(|id| {
                    self.parameters().definition(*id).map(|def| def.value)
                        != candidate.parameters().definition(*id).map(|def| def.value)
                })
            {
                return Err(ModelError::InvalidSketch(
                    "edit shared B-Rep parameters through the document Formula command",
                ));
            }
        }
        candidate.reevaluate_sketches()?;
        recipe.evaluate_formulas(&candidate.parameter_registry)?;
        if let Some(feature) = editing {
            candidate.replace_sketch(feature, recipe)?;
        } else {
            candidate.insert_sketch(before, recipe)?;
        }
        *self = candidate;
        Ok(())
    }

    pub fn resolve_sketch_reference(
        &self,
        support: SketchSupport,
        item: SketchPrimitive,
    ) -> Result<SketchPrimitive, ModelError> {
        self.resolve_sketch_reference_with(support, item, None)
    }

    fn resolve_sketch_reference_with(
        &self,
        support: SketchSupport,
        item: SketchPrimitive,
        replacement: Option<(FeatureId, &SketchRecipe)>,
    ) -> Result<SketchPrimitive, ModelError> {
        if let SketchPrimitive::BrepFaceSection {
            feature,
            face,
            interval,
        } = item
        {
            return self
                .sketch_face_sections(support, feature, face)?
                .get(interval as usize)
                .copied()
                .ok_or(ModelError::InvalidSketch(
                    "B-Rep section interval no longer exists",
                ));
        }
        if let SketchPrimitive::BrepReference {
            feature,
            edge,
            mode,
            crossing,
        } = item
        {
            if !self
                .feature(feature)
                .is_some_and(|feature| feature.outputs().contains(&edge.support.entity))
            {
                return Err(ModelError::InvalidSketch(
                    "B-Rep reference producer no longer owns the edge",
                ));
            }
            let geometry = self
                .entity_geometry(edge.support.entity)
                .ok_or(ModelError::InvalidGeometryReference)?;
            let face = geometry.resolve_profile(
                crate::geometry::ProfileRef::Edge {
                    support: edge.support,
                    wire: edge.wire,
                    edge: edge.edge,
                },
                false,
            )?;
            let segment = face
                .wires
                .iter()
                .find(|wire| wire.wire == edge.wire)
                .and_then(|wire| wire.edges.iter().find(|item| item.edge == edge.edge))
                .ok_or(ModelError::InvalidGeometryReference)?;
            if segment.straight && crossing == 0 {
                return Self::resolve_world_segment(support, mode, segment.start, segment.end);
            }
            let cubic = segment
                .cubic
                .ok_or(ModelError::InvalidSketch("unsupported B-Rep edge curve"))?;
            let controls = cubic.map(|point| support.local(point));
            let planar = controls.map(|(point, _)| point);
            let result = match mode {
                ReferenceMode::Projected if crossing == 0 => {
                    SketchPrimitive::Draw(DrawGeometry::Bezier { controls: planar })
                }
                ReferenceMode::Intersect => {
                    let roots = curves::cubic_plane_roots(controls.map(|(_, z)| z))?;
                    let t = roots.get(usize::from(crossing)).copied().ok_or(
                        ModelError::InvalidSketch("B-Rep edge has no such plane crossing"),
                    )?;
                    SketchPrimitive::Draw(DrawGeometry::Point(DrawGeometry::evaluate_bezier(
                        planar, t,
                    )))
                }
                _ => return Err(ModelError::InvalidGeometryReference),
            };
            result.validate()?;
            return Ok(result);
        }
        let SketchPrimitive::Reference {
            feature,
            element,
            mode,
        } = item
        else {
            return Err(ModelError::InvalidSketch("expected linked reference"));
        };
        let broken = || ModelError::InvalidSketch("reference source is missing or incompatible");
        let source = if let Some((_, recipe)) = replacement.filter(|(id, _)| *id == feature) {
            recipe
        } else if let Some(Feature::Sketch { recipe, .. }) = self.feature(feature) {
            recipe
        } else {
            return Err(broken());
        };
        let item = source
            .primitives
            .iter()
            .find_map(|&(key, item)| (key == element).then_some(item))
            .ok_or_else(broken)?;
        if replacement.is_some_and(|(id, _)| id == feature)
            && matches!(
                item,
                SketchPrimitive::EditedLine {
                    operation: LineOperation::Split { .. },
                    ..
                }
            )
            && self.feature(feature).is_some_and(|feature| match feature {
                Feature::Sketch { recipe, .. } => recipe
                    .primitives
                    .iter()
                    .any(|&(key, before)| key == element && before != item),
                _ => false,
            })
        {
            return Err(ModelError::InvalidSketch("referenced line was split"));
        }
        let geometry = source.resolved(item)?;
        if mode == ReferenceMode::Projected
            && source.support == support
            && geometry.is_independent()
        {
            return Ok(geometry);
        }
        let project = |p| support.local(source.support.world(p));
        if let SketchPrimitive::Draw(DrawGeometry::Point(p)) = geometry
            && mode == ReferenceMode::Projected
        {
            return Ok(SketchPrimitive::Draw(DrawGeometry::Point(project(p).0)));
        }
        if let SketchPrimitive::Draw(DrawGeometry::Ellipse {
            center,
            major,
            minor,
        }) = geometry
            && mode == ReferenceMode::Projected
        {
            let axis = major[0].hypot(major[1]);
            let transverse = [-major[1] * minor / axis, major[0] * minor / axis];
            let world_center = source.support.world(center);
            let local_center = support.local(world_center).0;
            let local_u = support.local(source.support.world(major)).0;
            let local_v = support.local(source.support.world(transverse)).0;
            // Distinct principal planes share exactly one axis.
            let span = [local_u[0].hypot(local_v[0]), local_u[1].hypot(local_v[1])];
            let result = SketchPrimitive::Line {
                start: std::array::from_fn(|i| local_center[i] - span[i]),
                end: std::array::from_fn(|i| local_center[i] + span[i]),
            };
            result.validate()?;
            return Ok(result);
        }
        let (start, end) = geometry.line().ok_or_else(broken)?;
        Self::resolve_world_segment(
            support,
            mode,
            source.support.world(start),
            source.support.world(end),
        )
    }

    /// A transverse cut through certified closed loops on a planar face.
    pub fn sketch_face_sections(
        &self,
        support: SketchSupport,
        feature: FeatureId,
        reference: crate::geometry::FaceRef,
    ) -> Result<Vec<SketchPrimitive>, ModelError> {
        let invalid = || {
            ModelError::InvalidSketch(
                "section requires a planar face with certified transverse boundaries",
            )
        };
        if !self
            .feature(feature)
            .is_some_and(|feature| feature.outputs().contains(&reference.entity))
        {
            return Err(invalid());
        }
        let face = self
            .entity_geometry(reference.entity)
            .ok_or_else(invalid)?
            .resolve_profile(crate::geometry::ProfileRef::Face(reference), true)?;
        let mut crossings = Vec::new();
        for edge in face.wires.iter().flat_map(|wire| &wire.edges) {
            let (a, za) = support.local(edge.start);
            let (b, zb) = support.local(edge.end);
            if za.abs() < 1e-7 || zb.abs() < 1e-7 {
                return Err(invalid());
            }
            if edge.straight {
                if za.signum() != zb.signum() {
                    let t = za / (za - zb);
                    crossings.push(std::array::from_fn::<_, 2, _>(|i| {
                        (1.0 - t) * a[i] + t * b[i]
                    }));
                }
            } else {
                let controls = edge
                    .cubic
                    .ok_or_else(invalid)?
                    .map(|point| support.local(point));
                let z = controls.map(|(_, z)| z);
                let planar = controls.map(|(point, _)| point);
                for t in curves::cubic_plane_roots(z)? {
                    let u = 1.0 - t;
                    let derivative = 3.0
                        * (u * u * (z[1] - z[0])
                            + 2.0 * u * t * (z[2] - z[1])
                            + t * t * (z[3] - z[2]));
                    if derivative.abs() < 1e-7 {
                        return Err(invalid());
                    }
                    crossings.push(DrawGeometry::evaluate_bezier(planar, t));
                }
            }
        }
        let projected_normal = support.local(face.frame.normal).0;
        let axis = usize::from(projected_normal[0].abs() > projected_normal[1].abs());
        crossings.sort_by(|a, b| a[axis].total_cmp(&b[axis]));
        if !crossings.len().is_multiple_of(2)
            || crossings.windows(2).any(|pair| {
                let [x, y] = [pair[1][0] - pair[0][0], pair[1][1] - pair[0][1]];
                !x.hypot(y).is_finite() || x.hypot(y) < 1e-7
            })
        {
            return Err(invalid());
        }
        crossings
            .as_chunks::<2>()
            .0
            .iter()
            .map(|points| {
                let item = SketchPrimitive::Line {
                    start: points[0],
                    end: points[1],
                };
                item.validate()?;
                Ok(item)
            })
            .collect()
    }

    fn resolve_world_segment(
        support: SketchSupport,
        mode: ReferenceMode,
        start: [f64; 3],
        end: [f64; 3],
    ) -> Result<SketchPrimitive, ModelError> {
        let (a, za) = support.local(start);
        let (b, zb) = support.local(end);
        let result = match mode {
            ReferenceMode::Projected => SketchPrimitive::Line { start: a, end: b },
            ReferenceMode::Intersect => {
                if za.abs() < 1e-7 && zb.abs() < 1e-7 {
                    return Err(ModelError::InvalidSketch("line lies in the sketch plane"));
                }
                if za.signum() == zb.signum() && za != 0.0 && zb != 0.0 {
                    return Err(ModelError::InvalidSketch("line misses the sketch plane"));
                }
                let t = za / (za - zb);
                if !(0.0..=1.0).contains(&t) {
                    return Err(ModelError::InvalidSketch("line misses the sketch plane"));
                }
                SketchPrimitive::Draw(DrawGeometry::Point(std::array::from_fn(|i| {
                    (1.0 - t) * a[i] + t * b[i]
                })))
            }
        };
        result.validate()?;
        Ok(result)
    }

    /// Retain a B-Rep link across a new body generation only when its
    /// certified construction role and complete analytic boundary both agree.
    pub(crate) fn remap_replayed_sketch_references(
        &mut self,
        accepted: &Self,
    ) -> Result<(), ModelError> {
        use crate::geometry::{FaceProvenance, FaceRef};
        let descriptions = &self.entity_geometry;
        let remap = |reference: FaceRef| {
            let invalid = || ModelError::InvalidSketch("B-Rep reference lost its certified face");
            let current = descriptions.get(&reference.entity).ok_or_else(invalid)?;
            if current.generation == reference.generation {
                return Ok(reference);
            }
            let previous = accepted
                .entity_geometry(reference.entity)
                .filter(|old| old.generation == reference.generation)
                .and_then(|old| old.faces.iter().find(|face| face.face == reference.face))
                .filter(|face| face.provenance != FaceProvenance::Unknown)
                .ok_or_else(invalid)?;
            let mut matches = current
                .faces
                .iter()
                .filter(|face| face.provenance == previous.provenance);
            let face = matches.next().ok_or_else(invalid)?;
            if matches.next().is_some()
                || face.frame != previous.frame
                || face.wires != previous.wires
            {
                return Err(invalid());
            }
            Ok(FaceRef {
                face: face.face,
                generation: current.generation,
                ..reference
            })
        };
        for (_, feature) in &mut self.features {
            if let Feature::Sketch { recipe, .. } = feature {
                for (_, item) in &mut recipe.primitives {
                    match item {
                        SketchPrimitive::BrepReference { edge, .. } => {
                            edge.support = remap(edge.support)?
                        }
                        SketchPrimitive::BrepFaceSection { face, .. } => *face = remap(*face)?,
                        _ => {}
                    }
                }
            }
        }
        self.validate_replayed_sketch_references()
    }

    pub(crate) fn validate_replayed_sketch_references(&self) -> Result<(), ModelError> {
        for (limit, (_, feature)) in self.features.iter().enumerate() {
            if let Feature::Sketch { recipe, .. } = feature {
                self.validate_sketch_references(recipe, limit, None)?;
            }
        }
        Ok(())
    }

    fn validate_sketch_references(
        &self,
        recipe: &SketchRecipe,
        limit: usize,
        replacement: Option<(FeatureId, &SketchRecipe)>,
    ) -> Result<(), ModelError> {
        for &(_, item) in &recipe.primitives {
            if let SketchPrimitive::Reference { feature, .. }
            | SketchPrimitive::BrepReference { feature, .. }
            | SketchPrimitive::BrepFaceSection { feature, .. } = item
            {
                if !self.features[..limit].iter().any(|(id, _)| *id == feature) {
                    return Err(ModelError::InvalidSketch(
                        "reference must target an earlier Sketch",
                    ));
                }
                self.resolve_sketch_reference_with(recipe.support, item, replacement)?;
            }
        }
        for (id, point) in &recipe.dimension_positions {
            if !point.iter().all(|coordinate| coordinate.is_finite())
                || (!recipe.driven_dimensions.contains_key(id)
                    && !recipe.segment_equations.contains_key(id))
            {
                return Err(ModelError::InvalidSketch("invalid dimension placement"));
            }
        }
        recipe.validate_segment_equations()?;
        recipe.validate_joins()?;
        for dimension in recipe.driven_dimensions.values() {
            dimension.measure(self, recipe, replacement)?;
        }
        Ok(())
    }

    pub fn insert_sketch(
        &mut self,
        before: Option<FeatureId>,
        mut recipe: SketchRecipe,
    ) -> Result<FeatureId, ModelError> {
        recipe.evaluate_formulas(&self.parameter_registry)?;
        recipe.validate()?;
        let index = self.insertion_index(before)?;
        self.validate_sketch_references(&recipe, index, None)?;
        self.next_feature
            .checked_add(1)
            .ok_or(ModelError::IdentityExhausted)?;
        let layout_index = if let Some(id) = before {
            self.layout
                .iter()
                .position(|row| row.node == HistoryNodeId::Operation(id))
                .ok_or(ModelError::InvalidHistoryLayout)?
        } else {
            self.layout.len()
        };
        let depth = self.layout.get(layout_index).map_or(0, |row| row.depth);
        let mut ordinal = self.next_sketch.unwrap_or(1);
        let (name, next) = loop {
            let name = format!("Sketch{ordinal}");
            let next = ordinal
                .checked_add(1)
                .ok_or(ModelError::IdentityExhausted)?;
            if self
                .features
                .iter()
                .all(|(_, feature)| feature.name() != name)
            {
                break (name, next);
            }
            ordinal = next;
        };
        let id = self.insert_feature(
            index,
            Feature::Sketch {
                name,
                recipe,
                visible: true,
            },
        );
        self.layout.insert(
            layout_index,
            HistoryLayoutRow {
                node: HistoryNodeId::Operation(id),
                depth,
            },
        );
        self.next_sketch = Some(next);
        Ok(id)
    }

    /// Updates viewport visibility without changing the stored recipe.
    pub fn set_sketch_visibility(
        &mut self,
        target: FeatureId,
        value: bool,
    ) -> Result<bool, ModelError> {
        let Some(Feature::Sketch { visible, .. }) = self
            .features
            .iter_mut()
            .find_map(|(id, feature)| (*id == target).then_some(feature))
        else {
            return Err(ModelError::UnknownFeature(target));
        };
        let changed = *visible != value;
        *visible = value;
        Ok(changed)
    }

    pub fn replace_sketch(
        &mut self,
        target: FeatureId,
        mut recipe: SketchRecipe,
    ) -> Result<(), ModelError> {
        recipe.evaluate_formulas(&self.parameter_registry)?;
        recipe.validate()?;
        let index = self
            .features
            .iter()
            .position(|(id, _)| *id == target)
            .ok_or(ModelError::UnknownFeature(target))?;
        self.validate_sketch_references(&recipe, index, None)?;
        for (limit, (_, feature)) in self.features.iter().enumerate().skip(index + 1) {
            if let Feature::Sketch {
                recipe: downstream, ..
            } = feature
            {
                self.validate_sketch_references(downstream, limit, Some((target, &recipe)))?;
            }
        }
        let Feature::Sketch { recipe: old, .. } = &mut self.features[index].1 else {
            return Err(ModelError::UnknownFeature(target));
        };
        *old = recipe;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sketch_brep_face_sections_preserve_holes_and_reject_stale_generations() {
        use crate::geometry::{
            EntityGeometry, FaceProvenance, FaceRef, GeometryGeneration, ProfileEdge, ProfileFace,
            ProfileFrame, ProfileWire,
        };
        let mut doc = Document::default();
        let feature = doc.add_base_solid("Fixture").unwrap();
        let entity = doc.feature(feature).unwrap().outputs()[0];
        let generation = GeometryGeneration(1);
        let rectangle = |wire, x0, x1, z0, z1| {
            let vertices = [[x0, 0., z0], [x1, 0., z0], [x1, 0., z1], [x0, 0., z1]];
            ProfileWire {
                wire,
                closed: true,
                edges: (0..4)
                    .map(|i| ProfileEdge {
                        edge: i as u32,
                        start: vertices[i],
                        end: vertices[(i + 1) % 4],
                        straight: true,
                        cubic: None,
                    })
                    .collect(),
            }
        };
        doc.entity_geometry.insert(
            entity,
            EntityGeometry {
                source_wire: None,
                entity,
                generation,
                kind: crate::EntityKind::Body,
                measure: 1.,
                face_count: 1,
                faces: vec![ProfileFace {
                    face: 0,
                    provenance: FaceProvenance::Base(0),
                    frame: ProfileFrame {
                        origin: [0.; 3],
                        normal: [0., 1., 0.],
                        u: [1., 0., 0.],
                        v: [0., 0., -1.],
                    },
                    wires: vec![
                        rectangle(0, -4., 4., -3., 3.),
                        rectangle(1, -1., 1., -2., 2.),
                    ],
                }],
            },
        );
        let face = FaceRef {
            entity,
            generation,
            face: 0,
        };
        let sections = doc
            .sketch_face_sections(SketchSupport::XY, feature, face)
            .unwrap();
        assert_eq!(sections[0].line(), Some(([-4., 0.], [-1., 0.])));
        assert_eq!(sections[1].line(), Some(([1., 0.], [4., 0.])));
        let item = SketchPrimitive::BrepFaceSection {
            feature,
            face,
            interval: 1,
        };
        let sketch = doc
            .insert_sketch(
                None,
                SketchRecipe {
                    support: SketchSupport::XY,
                    primitives: vec![(0, item)],
                    formulas: Default::default(),
                    pattern_formulas: Default::default(),
                    rigid_formulas: Default::default(),
                    driven_dimensions: Default::default(),
                    segment_equations: Default::default(),
                    joins: Default::default(),
                    dimension_positions: Default::default(),
                    corner_formulas: Default::default(),
                },
            )
            .unwrap();
        let accepted = doc.clone();
        doc.entity_geometry.get_mut(&entity).unwrap().generation = GeometryGeneration(2);
        assert!(
            doc.resolve_sketch_reference(SketchSupport::XY, item)
                .is_err()
        );
        doc.remap_replayed_sketch_references(&accepted).unwrap();
        assert!(
            matches!(doc.feature(sketch), Some(Feature::Sketch { recipe, .. })
            if matches!(recipe.primitives[0].1, SketchPrimitive::BrepFaceSection { face, .. }
                if face.generation == GeometryGeneration(2)))
        );
        let accepted = doc.clone();
        let geometry = doc.entity_geometry.get_mut(&entity).unwrap();
        geometry.generation = GeometryGeneration(3);
        geometry.faces[0].wires.pop();
        assert!(doc.remap_replayed_sketch_references(&accepted).is_err());
    }

    #[test]
    fn sketch_corner_and_insert_formula_replay_rejects_invalid_geometry_atomically() {
        use crate::parameters::{ParameterDraftOwner, ParameterEdit, ParameterOwner};
        let mut document = Document::default();
        let width = |value| crate::Length::millimetres(value).unwrap();
        document.set_parameter("Radius", width(1.)).unwrap();
        let mut draft = document
            .begin_parameter_draft(ParameterDraftOwner::Document)
            .unwrap();
        let mut stage = |source: &str, destination| {
            draft
                .stage_edit(ParameterEdit {
                    parameter: None,
                    owner: ParameterOwner::Document,
                    source: source.into(),
                    destination,
                    source_edit: None,
                    source_origins: None,
                })
                .unwrap()
        };
        let radius = stage("#Radius", crate::QuantityKind::Length);
        let angle = stage("90deg * (#Radius / 1mm)", crate::QuantityKind::Angle);
        let first = ([0., 0.], [8., 0.]);
        let second = ([0., 0.], [0., 8.]);
        let line = |(start, end)| SketchPrimitive::Line { start, end };
        let (trims, join) = line_corner(first, second, [1.0; 2], true).unwrap();
        let recipe = SketchRecipe {
            support: SketchSupport::XY,
            primitives: vec![
                (0, line(trims[0])),
                (1, line(trims[1])),
                (2, join),
                (3, line(first)),
            ],
            formulas: Default::default(),
            pattern_formulas: Default::default(),
            driven_dimensions: Default::default(),
            segment_equations: Default::default(),
            joins: Default::default(),
            dimension_positions: Default::default(),
            corner_formulas: [(
                2,
                CornerFormula {
                    parameters: [Some(radius), None],
                    values: [1.0; 2],
                    sources: [0, 1],
                    segments: [first, second],
                    fillet: true,
                },
            )]
            .into(),
            rigid_formulas: [(
                3,
                RigidFormula {
                    parameter: angle,
                    original: line(first),
                    translation: [10., 0.],
                },
            )]
            .into(),
        };
        document
            .commit_sketch_with_formulas(None, None, recipe, &draft)
            .unwrap();
        document.set_parameter("Radius", width(2.)).unwrap();
        let Feature::Sketch { recipe, .. } = &document.features()[0].1 else {
            unreachable!()
        };
        assert!(
            matches!(recipe.primitives[2].1, SketchPrimitive::Draw(DrawGeometry::Arc { radial, .. })
            if (radial[0].hypot(radial[1]) - 2.).abs() < 1e-12)
        );
        let (_, end) = recipe.primitives[3].1.line().unwrap();
        assert!((end[0] - 2.).hypot(end[1]) < 1e-12);
        let before = document.clone();
        assert!(document.set_parameter("Radius", width(9.)).is_err());
        assert_eq!(document, before);
    }

    #[test]
    fn sketch_formula_dependencies_regenerate_geometry_and_preserve_pattern_placements() {
        use crate::parameters::{ParameterDraftOwner, ParameterEdit, ParameterOwner};
        let mut doc = Document::default();
        let set = |doc: &mut Document, n| {
            doc.set_parameter("Width", crate::Length::millimetres(n).unwrap())
        };
        set(&mut doc, 5.0).unwrap();
        let mut draft = doc
            .begin_parameter_draft(ParameterDraftOwner::Document)
            .unwrap();
        let id = draft
            .stage_edit(ParameterEdit {
                parameter: None,
                owner: ParameterOwner::Document,
                source: "#Width / 1mm".into(),
                destination: crate::QuantityKind::Scalar,
                source_edit: None,
                source_origins: None,
            })
            .unwrap();
        let master = SketchPrimitive::Draw(DrawGeometry::Polygon {
            center: [0.; 2],
            vertex: [1., 0.],
            sides: 5,
        });
        let recipe = SketchRecipe {
            support: SketchSupport::XY,
            primitives: vec![(0, master)],
            formulas: [(0, id)].into(),
            rigid_formulas: Default::default(),
            driven_dimensions: Default::default(),
            segment_equations: Default::default(),
            joins: Default::default(),
            dimension_positions: Default::default(),
            corner_formulas: Default::default(),
            pattern_formulas: [(
                1,
                PatternFormula {
                    parameter: id,
                    master: 0,
                    step: PatternStep::Linear([3., 0.]),
                    excluded: [2].into(),
                },
            )]
            .into(),
        };
        doc.commit_sketch_with_formulas(None, None, recipe, &draft)
            .unwrap();
        set(&mut doc, 7.0).unwrap();
        let Feature::Sketch { recipe, .. } = &doc.features()[0].1 else {
            unreachable!()
        };
        assert_eq!(
            recipe
                .primitives
                .iter()
                .map(|(key, _)| *key)
                .collect::<Vec<_>>(),
            vec![0, 1, 3, 4, 5, 6]
        );
        assert!(matches!(
            recipe.primitives[0].1,
            SketchPrimitive::Draw(DrawGeometry::Polygon { sides: 7, .. })
        ));
        let before = doc.clone();
        assert!(set(&mut doc, 2.5).is_err());
        assert_eq!(doc, before);
    }

    #[test]
    fn sketch_reference_resolves_hidden_sources_and_rejects_broken_replacement() {
        let mut doc = Document::default();
        let line = |x| SketchPrimitive::Line {
            start: [x, -4.],
            end: [x + 2., 4.],
        };
        let source = |key, x| SketchRecipe {
            formulas: Default::default(),
            pattern_formulas: Default::default(),
            rigid_formulas: Default::default(),
            driven_dimensions: Default::default(),
            segment_equations: Default::default(),
            joins: Default::default(),
            dimension_positions: Default::default(),
            corner_formulas: Default::default(),
            support: SketchSupport::XZ,
            primitives: vec![(key, line(x))],
        };
        let id = doc.insert_sketch(None, source(7, 2.)).unwrap();
        let link = |mode| SketchPrimitive::Reference {
            feature: id,
            element: 7,
            mode,
        };
        let projected = link(ReferenceMode::Projected);
        let intersection = link(ReferenceMode::Intersect);
        let target = SketchRecipe {
            formulas: Default::default(),
            pattern_formulas: Default::default(),
            rigid_formulas: Default::default(),
            driven_dimensions: Default::default(),
            segment_equations: Default::default(),
            joins: Default::default(),
            dimension_positions: Default::default(),
            corner_formulas: Default::default(),
            support: SketchSupport::XY,
            primitives: vec![(0, projected), (1, intersection)],
        };
        assert!(doc.insert_sketch(Some(id), target.clone()).is_err());
        doc.insert_sketch(None, target).unwrap();
        doc.set_sketch_visibility(id, false).unwrap();
        assert_eq!(
            doc.resolve_sketch_reference(SketchSupport::XY, intersection)
                .unwrap(),
            SketchPrimitive::Draw(DrawGeometry::Point([3., 0.]))
        );
        doc.replace_sketch(id, source(7, 5.)).unwrap();
        assert_eq!(
            doc.resolve_sketch_reference(SketchSupport::XY, projected)
                .unwrap()
                .line(),
            Some(([5., 0.], [7., 0.]))
        );
        let before = doc.clone();
        assert!(doc.replace_sketch(id, source(8, 5.)).is_err());
        assert_eq!(doc, before);
    }

    #[test]
    fn sketch_insert_skips_collisions_and_rejection_does_not_advance_the_ordinal() {
        let mut document = Document::default();
        let recipe = SketchRecipe {
            formulas: Default::default(),
            pattern_formulas: Default::default(),
            rigid_formulas: Default::default(),
            driven_dimensions: Default::default(),
            segment_equations: Default::default(),
            joins: Default::default(),
            dimension_positions: Default::default(),
            corner_formulas: Default::default(),
            support: SketchSupport::XY,
            primitives: vec![(
                0,
                SketchPrimitive::Line {
                    start: [0., 0.],
                    end: [1., 0.],
                },
            )],
        };
        let first = document.insert_sketch(None, recipe.clone()).unwrap();
        document.rename_feature(first, "Sketch2".into()).unwrap();
        let before = document.clone();
        assert!(
            document
                .insert_sketch(
                    None,
                    SketchRecipe {
                        formulas: Default::default(),
                        pattern_formulas: Default::default(),
                        rigid_formulas: Default::default(),
                        driven_dimensions: Default::default(),
                        segment_equations: Default::default(),
                        joins: Default::default(),
                        dimension_positions: Default::default(),
                        corner_formulas: Default::default(),
                        support: SketchSupport::XY,
                        primitives: vec![(
                            0,
                            SketchPrimitive::Line {
                                start: [0.; 2],
                                end: [0.; 2]
                            }
                        )],
                    }
                )
                .is_err()
        );
        assert_eq!(document, before);
        let second = document.insert_sketch(Some(first), recipe).unwrap();
        assert_eq!(document.features()[0].0, second);
        assert_eq!(document.feature(second).unwrap().name(), "Sketch3");
        assert!(document.feature(second).unwrap().outputs().is_empty());
    }
}
