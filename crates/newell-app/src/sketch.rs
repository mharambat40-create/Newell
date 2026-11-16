// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Private window-local principal-plane sketch edits.

mod snap;
pub(crate) use snap::join_strokes;

use crate::{camera::Camera, document::DocumentVersion};
use newell_model::{
    FeatureId,
    sketch::{
        CornerFormula, DrawGeometry, LineEditMode, LineOperation, MAX_PRIMITIVES, PATTERN_KEY_SPAN,
        PatternFormula, PatternStep, RigidFormula, SketchPrimitive as Primitive, SketchRecipe,
        SketchSupport, SketchText, line_corner, offset_line_chain,
    },
};
use newell_ui::{DrawTool, OriginPlane};

pub(crate) fn empty_recipe(plane: OriginPlane) -> SketchRecipe {
    let support = match plane {
        OriginPlane::XY => SketchSupport::XY,
        OriginPlane::XZ => SketchSupport::XZ,
        OriginPlane::YZ => SketchSupport::YZ,
    };
    SketchRecipe {
        support,
        primitives: Vec::new(),
        formulas: Default::default(),
        pattern_formulas: Default::default(),
        rigid_formulas: Default::default(),
        driven_dimensions: Default::default(),
        segment_equations: Default::default(),
        joins: Default::default(),
        dimension_positions: Default::default(),
        corner_formulas: Default::default(),
    }
}

pub(crate) const fn ui_plane(support: SketchSupport) -> OriginPlane {
    match support {
        SketchSupport::XY => OriginPlane::XY,
        SketchSupport::XZ => OriginPlane::XZ,
        SketchSupport::YZ => OriginPlane::YZ,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tool {
    Select,
    Line,
    Rectangle,
    Draw(DrawTool),
}

pub(crate) struct Click {
    pub point: (f64, f64),
    pub navigation: u64,
    pub layout: newell_ui::ProjectionGeneration,
    pub scale: f64,
}

#[derive(Clone, Copy)]
struct Handle {
    reference: newell_model::sketch::SketchPointRef,
    local: [f64; 2],
    screen: [f64; 2],
}

type HandleRef = newell_model::sketch::SketchPointRef;

struct MarkerCycle {
    anchor: (f64, f64),
    choices: Vec<HandleRef>,
    selected: usize,
}

pub(crate) struct Edit {
    pub source: DocumentVersion,
    pub before: Option<FeatureId>,
    pub target: Option<FeatureId>,
    pub entry_camera: Camera,
    pub recipe: SketchRecipe,
    pub press: Option<Click>,
    pub secondary_press: Option<Click>,
    pub solve: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub(crate) pending_draw: Option<u64>,
    snap: std::cell::Cell<Option<snap::Placement>>,
    snap_suppressed_at: std::cell::Cell<Option<[f64; 2]>>,
    attachments: Vec<(snap::Slot, snap::Hit)>,
    tool: Tool,
    pub(crate) numeric: Option<f64>,
    pub(crate) associative_mirror: bool,
    pub(crate) transform_move: bool,
    pub(crate) spline_handles: bool,
    pub(crate) offset_chain: bool,
    pub(crate) second_numeric: Option<f64>,
    pub(crate) second_formula: Option<newell_model::formula::ParameterId>,
    pub(crate) numeric_formula: Option<newell_model::formula::ParameterId>,
    pub(crate) text: String,
    start: Option<[f64; 2]>,
    second: Option<[f64; 2]>,
    third: Option<[f64; 2]>,
    fourth: Option<[f64; 2]>,
    fifth: Option<[f64; 2]>,
    chain: bool,
    selected: Option<u64>,
    selected_handle: Option<HandleRef>,
    marker_cycle: Option<MarkerCycle>,
    line_target: Option<u64>,
    dimension_target: Option<u64>,
    corner_second: Option<u64>,
    copy_source: Option<SketchRecipe>,
    reference_preview: Vec<(Primitive, Primitive)>,
    reference_picks: std::collections::BTreeMap<u64, Primitive>,
    next_key: u64,
}

impl Edit {
    pub fn new(
        source: DocumentVersion,
        before: Option<FeatureId>,
        target: Option<FeatureId>,
        entry_camera: Camera,
        recipe: SketchRecipe,
    ) -> Self {
        let next_key = recipe
            .primitives
            .iter()
            .map(|(key, _)| *key)
            .max()
            .map_or(0, |key| key.saturating_add(1));
        let next_key = recipe
            .pattern_formulas
            .keys()
            .next_back()
            .map_or(next_key, |last| {
                next_key.max(last.saturating_add(PATTERN_KEY_SPAN))
            });
        Self {
            source,
            before,
            target,
            entry_camera,
            recipe,
            next_key,
            press: None,
            secondary_press: None,
            solve: None,
            pending_draw: None,
            snap: std::cell::Cell::new(None),
            snap_suppressed_at: std::cell::Cell::new(None),
            attachments: Vec::new(),
            tool: Tool::Select,
            numeric: None,
            associative_mirror: false,
            transform_move: false,
            spline_handles: false,
            offset_chain: false,
            numeric_formula: None,
            second_numeric: None,
            second_formula: None,
            text: String::new(),
            start: None,
            second: None,
            third: None,
            fourth: None,
            fifth: None,
            chain: false,
            selected: None,
            selected_handle: None,
            marker_cycle: None,
            line_target: None,
            dimension_target: None,
            corner_second: None,
            copy_source: None,
            reference_preview: Vec::new(),
            reference_picks: Default::default(),
        }
    }

    pub fn ready(&self) -> bool {
        self.solve.is_none()
            && !self.recipe.primitives.is_empty()
            && (self.start.is_none() || self.chain)
            && self.line_target.is_none()
            && self.dimension_target.is_none()
            && self.reference_preview.is_empty()
    }

    pub fn is_select(&self) -> bool {
        self.tool == Tool::Select
    }

    pub fn draw_tool(&self) -> Option<DrawTool> {
        match self.tool {
            Tool::Draw(tool) => Some(tool),
            _ => None,
        }
    }

    pub fn set_tool(&mut self, tool: Tool) {
        self.cancel_solve();
        self.tool = tool;
        self.copy_source = None;
        self.reference_preview.clear();
        self.clear_gesture();
        self.selected = None;
        self.selected_handle = None;
        self.marker_cycle = None;
    }

    pub fn cancel_solve(&mut self) {
        self.pending_draw = None;
        if let Some(flag) = self.solve.take() {
            flag.store(true, std::sync::atomic::Ordering::Release);
        }
    }

    pub fn segment_edit(
        &mut self,
        local: [f64; 2],
        point: (f64, f64),
        scale: f64,
        camera: Camera,
    ) -> Option<Result<Option<SketchRecipe>, newell_model::ModelError>> {
        use newell_model::{
            Length, ModelError,
            sketch::{DrivenDimension, SegmentEquation, SegmentRelation},
        };
        let tool = self.draw_tool()?;
        if !matches!(
            tool,
            DrawTool::DrivingLength
                | DrawTool::DrivenLength
                | DrawTool::Horizontal
                | DrawTool::Vertical
        ) {
            return None;
        }
        Some((|| {
            let is_dimension = matches!(tool, DrawTool::DrivenLength | DrawTool::DrivingLength);
            let key = if let Some(key) = self.dimension_target {
                key
            } else {
                let (hit, ambiguous) = self.pick(point, scale, camera);
                let (key, _) = hit
                    .filter(|(_, item)| !ambiguous && matches!(item, Primitive::Line { .. }))
                    .ok_or(ModelError::InvalidSketch("select one independent line"))?;
                if is_dimension {
                    self.dimension_target = Some(key);
                    return Ok(None);
                }
                key
            };
            let mut recipe = self.recipe.clone();
            let existing = if tool == DrawTool::DrivenLength {
                recipe
                    .driven_dimensions
                    .iter()
                    .find_map(|(&id, dimension)| {
                        (*dimension == DrivenDimension::Length(key)).then_some(id)
                    })
            } else if tool == DrawTool::DrivingLength {
                recipe.segment_equations.iter().find_map(|(&id, equation)| {
                    (equation.segment == key
                        && matches!(equation.relation, SegmentRelation::Length(_)))
                    .then_some(id)
                })
            } else {
                None
            };
            let id = match existing {
                Some(id) => id,
                None => recipe.next_relation_id()?,
            };
            if is_dimension {
                recipe.dimension_positions.insert(id, local);
            }
            if tool == DrawTool::DrivenLength {
                recipe
                    .driven_dimensions
                    .insert(id, DrivenDimension::Length(key));
                return Ok(Some(recipe));
            }
            let relation = match tool {
                DrawTool::Horizontal => SegmentRelation::Horizontal,
                DrawTool::Vertical => SegmentRelation::Vertical,
                _ => SegmentRelation::Length(Length::millimetres(self.numeric.unwrap_or(10.0))?),
            };
            recipe.segment_equations.insert(
                id,
                SegmentEquation {
                    segment: key,
                    relation,
                    formula: (tool == DrawTool::DrivingLength)
                        .then_some(self.numeric_formula)
                        .flatten(),
                },
            );
            Ok(Some(recipe))
        })())
    }

    pub fn insert_from(&mut self, recipe: SketchRecipe) {
        self.set_tool(Tool::Draw(DrawTool::Insert));
        self.copy_source = Some(recipe);
    }

    pub fn link_from(&mut self, tool: DrawTool, preview: Vec<(Primitive, Primitive)>) {
        self.set_tool(Tool::Draw(tool));
        self.reference_preview = preview;
    }

    fn append(&mut self, result: Vec<Primitive>) -> Result<(), newell_model::ModelError> {
        if self.recipe.primitives.len() + result.len() > MAX_PRIMITIVES {
            return Err(newell_model::ModelError::InvalidSketch("capacity"));
        }
        let next = self
            .next_key
            .checked_add(result.len() as u64)
            .ok_or(newell_model::ModelError::IdentityExhausted)?;
        self.recipe
            .primitives
            .extend((self.next_key..next).zip(result));
        self.next_key = next;
        Ok(())
    }

    fn append_pattern(
        &mut self,
        instances: Vec<Primitive>,
    ) -> Result<(), newell_model::ModelError> {
        let binding = self
            .numeric_formula
            .zip(instances.first())
            .and_then(|(parameter, first)| {
                let Primitive::PatternInstance {
                    master,
                    pattern,
                    step,
                    ..
                } = *first
                else {
                    return None;
                };
                Some((
                    pattern,
                    PatternFormula {
                        parameter,
                        master,
                        step,
                        excluded: Default::default(),
                    },
                ))
            });
        let reserved = binding
            .as_ref()
            .map(|(pattern, _)| {
                pattern
                    .checked_add(PATTERN_KEY_SPAN)
                    .ok_or(newell_model::ModelError::IdentityExhausted)
            })
            .transpose()?;
        self.append(instances)?;
        if let Some((pattern, formula)) = binding {
            self.recipe.pattern_formulas.insert(pattern, formula);
        }
        if let Some(end) = reserved {
            self.next_key = end;
        }
        Ok(())
    }

    fn corner_result(
        &self,
        a: u64,
        b: u64,
    ) -> Result<([Primitive; 2], Primitive), newell_model::ModelError> {
        if self.recipe.corner_formula_for(a).is_some()
            || self.recipe.corner_formula_for(b).is_some()
            || self.recipe.rigid_formulas.contains_key(&a)
            || self.recipe.rigid_formulas.contains_key(&b)
        {
            return Err(newell_model::ModelError::InvalidSketch(
                "detach the existing corner Formula before editing its lines",
            ));
        }
        if a == b || self.recipe.has_instances(a) || self.recipe.has_instances(b) {
            return Err(newell_model::ModelError::InvalidSketch(
                "choose two distinct lines without pattern instances",
            ));
        }
        let (lines, join) = line_corner(
            self.source_line(a)?,
            self.source_line(b)?,
            [
                self.numeric.unwrap_or(1.0),
                self.second_numeric.unwrap_or(1.0),
            ],
            self.tool == Tool::Draw(DrawTool::Fillet),
        )?;
        let trims = lines.map(|(start, end)| Primitive::Line { start, end });
        trims[0].validate()?;
        trims[1].validate()?;
        Ok((trims, join))
    }

    fn refined(&self, key: u64) -> Option<DrawGeometry> {
        self.recipe
            .primitives
            .iter()
            .find_map(|&(id, item)| match item {
                Primitive::Draw(draw) if id == key => draw.refine(),
                _ => None,
            })
    }

    fn pattern_result(
        &self,
        key: u64,
        end: [f64; 2],
    ) -> Result<Vec<Primitive>, newell_model::ModelError> {
        use newell_model::ModelError;
        let count = self.numeric.unwrap_or(6.0);
        if !(2.0..=256.0).contains(&count) || count.fract() != 0.0 {
            return Err(ModelError::InvalidSketch(
                "pattern count must be an integer from 2 to 256",
            ));
        }
        if self.recipe.primitives.len() + count as usize - 1 > MAX_PRIMITIVES {
            return Err(ModelError::InvalidSketch("capacity"));
        }
        let anchor = self
            .start
            .ok_or(ModelError::InvalidSketch("missing pattern origin"))?;
        let delta = [end[0] - anchor[0], end[1] - anchor[1]];
        let step = if self.tool == Tool::Draw(DrawTool::LinearPattern) {
            if delta[0].hypot(delta[1]) < 1e-7 {
                return Err(ModelError::InvalidSketch("pattern spacing is zero"));
            }
            PatternStep::Linear(delta)
        } else {
            let arm = self
                .second
                .ok_or(ModelError::InvalidSketch("missing pattern start direction"))?;
            let radians = (delta[1].atan2(delta[0])
                - (arm[1] - anchor[1]).atan2(arm[0] - anchor[0]))
            .rem_euclid(std::f64::consts::TAU);
            if delta[0].hypot(delta[1]) < 1e-7
                || radians < 1e-12
                || radians * (count - 1.0) >= std::f64::consts::TAU
            {
                return Err(ModelError::InvalidSketch(
                    "pattern must have distinct angular placements",
                ));
            }
            PatternStep::Circular {
                center: anchor,
                radians,
            }
        };
        (1..count as u16)
            .map(|placement| {
                let instance = Primitive::PatternInstance {
                    master: key,
                    pattern: self.next_key,
                    placement,
                    step,
                };
                self.recipe.resolved(instance)?;
                Ok(instance)
            })
            .collect()
    }

    fn transform_result(
        &self,
        key: u64,
        handle: [f64; 2],
    ) -> Result<Primitive, newell_model::ModelError> {
        let source = self
            .recipe
            .primitives
            .iter()
            .find_map(|&(id, item)| (id == key).then_some(item))
            .ok_or(newell_model::ModelError::InvalidSketch(
                "missing transform source",
            ))?;
        let destination = self.start.ok_or(newell_model::ModelError::InvalidSketch(
            "missing transform destination",
        ))?;
        source.placed(destination, handle)
    }

    fn mirror_result(
        &self,
        key: u64,
        axis: [[f64; 2]; 2],
    ) -> Result<Primitive, newell_model::ModelError> {
        let item = if self.associative_mirror {
            Primitive::PatternInstance {
                master: key,
                pattern: self.next_key,
                placement: 1,
                step: PatternStep::Mirror(axis),
            }
        } else {
            self.recipe
                .primitives
                .iter()
                .find_map(|&(id, item)| (id == key).then_some(item))
                .ok_or(newell_model::ModelError::InvalidSketch(
                    "missing mirror source",
                ))?
                .mirrored(axis)?
        };
        self.recipe.resolved(item)?;
        Ok(item)
    }

    fn source_line(&self, key: u64) -> Result<([f64; 2], [f64; 2]), newell_model::ModelError> {
        self.recipe
            .primitives
            .iter()
            .find_map(|&(id, primitive)| (id == key).then(|| primitive.line()).flatten())
            .ok_or(newell_model::ModelError::InvalidSketch("missing line"))
    }

    pub(crate) fn install_draw(&mut self, recipe: SketchRecipe, key: u64) {
        self.recipe = recipe;
        self.next_key = key + 1;
        self.finish_draw_state(key);
    }

    pub fn clear_gesture(&mut self) {
        if self.pending_draw.is_some() {
            self.cancel_solve();
        }
        self.clear_live_snap();
        self.attachments.clear();
        self.start = None;
        self.line_target = None;
        self.dimension_target = None;
        self.corner_second = None;
        self.second = None;
        self.third = None;
        self.fourth = None;
        self.fifth = None;
        self.chain = false;
        self.press = None;
        self.secondary_press = None;
        self.marker_cycle = None;
    }

    pub fn escape(&mut self) -> bool {
        let pointer = self.snap.get().map(|placement| placement.pointer);
        self.cancel_solve();
        if !self.reference_preview.is_empty() {
            self.set_tool(Tool::Select);
        } else if self.start.is_some()
            || self.line_target.is_some()
            || self.dimension_target.is_some()
        {
            self.clear_gesture();
        } else if self.tool != Tool::Select {
            self.set_tool(Tool::Select);
        } else {
            return true;
        }
        self.snap_suppressed_at.set(pointer);
        false
    }

    pub fn delete_selected(&mut self) -> Result<(), newell_model::ModelError> {
        if let Some(key) = self.selected {
            self.recipe.protect_dimension_operands(key)?;
            if self.recipe.corner_formula_for(key).is_some() {
                return Err(newell_model::ModelError::InvalidSketch(
                    "detach the corner Formula before deleting its result",
                ));
            }
            if self.recipe.has_instances(key) {
                return Err(newell_model::ModelError::InvalidSketch(
                    "remove pattern instances before deleting their master",
                ));
            }
            self.selected = None;
            self.recipe.formulas.remove(&key);
            self.recipe.rigid_formulas.remove(&key);
            if let Some(item) = self
                .recipe
                .primitives
                .iter()
                .find_map(|&(id, item)| (id == key).then_some(item))
            {
                self.recipe.exclude_placement(item);
            }
            self.recipe.primitives.retain(|(item, _)| *item != key);
        }
        Ok(())
    }

    pub fn refresh_reference_picks(&mut self, document: &newell_model::Document) {
        self.reference_picks = self
            .recipe
            .primitives
            .iter()
            .filter_map(|&(id, item)| {
                if !matches!(
                    item,
                    Primitive::Reference { .. }
                        | Primitive::BrepReference { .. }
                        | Primitive::BrepFaceSection { .. }
                ) {
                    return None;
                }
                Some((
                    id,
                    document
                        .resolve_sketch_reference(self.recipe.support, item)
                        .ok()?,
                ))
            })
            .collect();
    }

    pub fn detach_selected(&mut self) -> Result<(), newell_model::ModelError> {
        let error = || {
            newell_model::ModelError::InvalidSketch(
                "select a pattern instance or linked reference to detach",
            )
        };
        let key = self.selected.ok_or_else(error)?;
        if let Some(result) = self.recipe.corner_formula_for(key) {
            self.recipe.corner_formulas.remove(&result);
            return Ok(());
        }
        if self.recipe.formulas.remove(&key).is_some()
            || self.recipe.rigid_formulas.remove(&key).is_some()
        {
            return Ok(());
        }
        let item = self
            .recipe
            .primitives
            .iter()
            .find_map(|&(id, item)| (key == id).then_some(item))
            .ok_or_else(error)?;
        let geometry = match item {
            Primitive::PatternInstance { .. } => self.recipe.resolved(item)?,
            Primitive::Reference { .. }
            | Primitive::BrepReference { .. }
            | Primitive::BrepFaceSection { .. } => {
                self.reference_picks.get(&key).copied().ok_or_else(error)?
            }
            _ => return Err(error()),
        };
        self.recipe.protect_dimension_operands(key)?;
        let new_key = self.next_key;
        let next_key = self
            .next_key
            .checked_add(1)
            .ok_or(newell_model::ModelError::IdentityExhausted)?;
        self.recipe.exclude_placement(item);
        if let Some(entry) = self.recipe.primitives.iter_mut().find(|(id, _)| *id == key) {
            *entry = (new_key, geometry);
        }
        self.next_key = next_key;
        self.selected = Some(new_key);
        Ok(())
    }

    fn candidate(&self, end: [f64; 2]) -> Result<Option<Primitive>, newell_model::ModelError> {
        let rectangle = |a: [f64; 2], b: [f64; 2]| Primitive::Rectangle {
            min: std::array::from_fn(|i| a[i].min(b[i])),
            max: std::array::from_fn(|i| a[i].max(b[i])),
        };
        if self.tool == Tool::Draw(DrawTool::Point) {
            return Ok(Some(Primitive::Draw(DrawGeometry::Point(end))));
        }
        let Some(start) = self.start else {
            return Ok(None);
        };
        let primitive = match self.tool {
            Tool::Line => Primitive::Line { start, end },
            Tool::Rectangle => rectangle(start, end),
            Tool::Select => return Ok(None),
            Tool::Draw(DrawTool::CenterRectangle) => {
                rectangle(std::array::from_fn(|i| start[i] + (start[i] - end[i])), end)
            }
            Tool::Draw(tool) => Primitive::Draw(match tool {
                DrawTool::Text => DrawGeometry::Text(SketchText::new(
                    &self.text,
                    start,
                    end,
                    self.numeric.unwrap_or(1.0),
                )?),
                DrawTool::Circle => DrawGeometry::circle(start, end),
                DrawTool::Polygon => DrawGeometry::Polygon {
                    center: start,
                    vertex: end,
                    sides: {
                        let value = self.numeric.unwrap_or(6.0);
                        if !(3.0..=256.0).contains(&value) || value.fract() != 0.0 {
                            return Err(newell_model::ModelError::InvalidSketch(
                                "side count must be an integer from 3 to 256",
                            ));
                        }
                        value as u16
                    },
                },
                _ => {
                    let Some(second) = self.second else {
                        return Ok(None);
                    };
                    match tool {
                        DrawTool::OrientedRectangle => {
                            DrawGeometry::oriented_rectangle(start, second, end)
                        }
                        DrawTool::Slot => DrawGeometry::slot(start, second, end),
                        DrawTool::Circumcircle => DrawGeometry::circumcircle(start, second, end)?,
                        DrawTool::Ellipse => DrawGeometry::ellipse(start, second, end),
                        DrawTool::Conic => DrawGeometry::Conic {
                            start,
                            control: second,
                            end,
                            weight: self.numeric.unwrap_or(1.0),
                        },
                        DrawTool::ThreePointArc => {
                            DrawGeometry::three_point_arc(start, second, end)?
                        }
                        DrawTool::CenterArc => DrawGeometry::center_arc(start, second, end),
                        DrawTool::TangentArc => DrawGeometry::tangent_arc(start, second, end),
                        DrawTool::EllipticalArc => {
                            let (Some(third), Some(fourth)) = (self.third, self.fourth) else {
                                return Ok(None);
                            };
                            DrawGeometry::elliptical_arc(start, second, third, fourth, end)
                        }
                        DrawTool::Bezier | DrawTool::Spline => {
                            let Some(third) = self.third else {
                                return Ok(None);
                            };
                            let points = [start, second, third, end];
                            if tool == DrawTool::Spline {
                                if self.spline_handles {
                                    let (Some(last), Some(handle)) = (self.fourth, self.fifth)
                                    else {
                                        return Ok(None);
                                    };
                                    let tangent =
                                        |a: [f64; 2], b: [f64; 2]| [b[0] - a[0], b[1] - a[1]];
                                    DrawGeometry::Spline {
                                        fit: [start, second, third, last],
                                        tangents: Some([
                                            tangent(start, handle),
                                            tangent(last, end),
                                        ]),
                                        refined: false,
                                    }
                                } else {
                                    DrawGeometry::Spline {
                                        fit: points,
                                        tangents: None,
                                        refined: false,
                                    }
                                }
                            } else {
                                DrawGeometry::Bezier { controls: points }
                            }
                        }
                        _ => return Ok(None),
                    }
                }
            }),
        };
        primitive.validate()?;
        Ok(Some(primitive))
    }

    fn preview(&self, end: [f64; 2]) -> Option<Primitive> {
        self.candidate(end).ok().flatten().or_else(|| {
            if self.tool == Tool::Draw(DrawTool::Spline)
                && self.spline_handles
                && let (Some(a), Some(b), Some(c)) = (self.start, self.second, self.third)
            {
                return Some(Primitive::Draw(DrawGeometry::Spline {
                    fit: [a, b, c, self.fourth.unwrap_or(end)],
                    tangents: None,
                    refined: false,
                }));
            }
            if self.tool == Tool::Draw(DrawTool::EllipticalArc)
                && let (Some(center), Some(major), Some(side)) =
                    (self.start, self.second, self.third)
            {
                let ellipse = DrawGeometry::ellipse(center, major, side);
                return ellipse.validate().ok().map(|()| Primitive::Draw(ellipse));
            }
            if matches!(self.tool, Tool::Draw(DrawTool::Bezier | DrawTool::Spline))
                && self.third.is_none()
            {
                return Some(Primitive::Line {
                    start: self.second.or(self.start)?,
                    end,
                });
            }
            if self.second.is_none()
                && matches!(
                    self.tool,
                    Tool::Draw(
                        DrawTool::OrientedRectangle
                            | DrawTool::Slot
                            | DrawTool::Circumcircle
                            | DrawTool::Ellipse
                            | DrawTool::ThreePointArc
                            | DrawTool::CenterArc
                            | DrawTool::Conic
                            | DrawTool::EllipticalArc
                            | DrawTool::Bezier
                    )
                )
            {
                Some(Primitive::Line {
                    start: self.start?,
                    end,
                })
            } else {
                None
            }
        })
    }

    fn coordinate_compatible(&self, local: [f64; 2]) -> bool {
        if let Some(key) = self.line_target {
            if let Some(mode) = self.line_mode() {
                return self.line_result(mode, key, local).is_ok();
            }
            match self.tool {
                Tool::Draw(DrawTool::Transform) if self.start.is_some() => {
                    return self.transform_result(key, local).is_ok();
                }
                Tool::Draw(DrawTool::Mirror) if self.start.is_some() => {
                    return self.start.is_some_and(|start| self.mirror_result(key, [start, local]).is_ok());
                }
                Tool::Draw(DrawTool::CircularPattern) if self.start.is_some() && self.second.is_none() => {
                    return self.start != Some(local);
                }
                Tool::Draw(DrawTool::LinearPattern | DrawTool::CircularPattern)
                    if self.start.is_some() => return self.pattern_result(key, local).is_ok(),
                _ => {}
            }
        }
        self.candidate(local).is_ok()
    }

    fn line_mode(&self) -> Option<LineEditMode> {
        match self.tool {
            Tool::Draw(DrawTool::Trim) => Some(LineEditMode::Trim),
            Tool::Draw(DrawTool::Extend) => Some(LineEditMode::Extend),
            Tool::Draw(DrawTool::Split) => Some(LineEditMode::Split),
            Tool::Draw(DrawTool::Offset) => Some(LineEditMode::Offset),
            _ => None,
        }
    }

    fn line_result(
        &self,
        mode: LineEditMode,
        key: u64,
        local: [f64; 2],
    ) -> Result<Vec<Primitive>, newell_model::ModelError> {
        let boundaries = self
            .recipe
            .primitives
            .iter()
            .filter_map(|&(id, primitive)| (id != key).then(|| primitive.line()).flatten());
        if let Some(Primitive::Draw(draw)) = self
            .recipe
            .primitives
            .iter()
            .find_map(|&(id, item)| (id == key).then_some(item))
        {
            return match mode {
                LineEditMode::Offset => Ok(vec![Primitive::Draw(draw.offset_circular(local)?)]),
                LineEditMode::Split => Ok(draw.split_arc(local)?.map(Primitive::Draw).into()),
                LineEditMode::Trim | LineEditMode::Extend => Ok(draw
                    .cut_arc(mode, local, boundaries)?
                    .into_iter()
                    .map(Primitive::Draw)
                    .collect()),
            };
        }
        let source = self.source_line(key)?;
        if mode == LineEditMode::Offset && self.offset_chain {
            return offset_line_chain(source, local, boundaries)?
                .into_iter()
                .map(|(start, end)| {
                    let item = Primitive::Line { start, end };
                    item.validate()?;
                    Ok(item)
                })
                .collect();
        }
        let (operation, segments) = LineOperation::build(mode, source, local, boundaries)?;
        segments
            .into_iter()
            .map(|(start, end)| {
                let item = Primitive::EditedLine {
                    start,
                    end,
                    source: key,
                    operation,
                };
                item.validate()?;
                Ok(item)
            })
            .collect()
    }

    fn apply_line_edit(
        &mut self,
        mode: LineEditMode,
        key: u64,
        local: [f64; 2],
    ) -> Result<(), newell_model::ModelError> {
        if mode != LineEditMode::Offset {
            self.recipe.protect_dimension_operands(key)?;
        }
        if self.recipe.has_instances(key)
            && matches!(mode, LineEditMode::Split | LineEditMode::Trim)
        {
            return Err(newell_model::ModelError::InvalidSketch(
                "pattern master would split or lose its original interval",
            ));
        }
        if mode != LineEditMode::Offset
            && (self.recipe.corner_formula_for(key).is_some()
                || self.recipe.rigid_formulas.contains_key(&key))
        {
            return Err(newell_model::ModelError::InvalidSketch(
                "detach the Formula before editing this result",
            ));
        }
        let result = self.line_result(mode, key, local)?;
        let replace = mode != LineEditMode::Offset;
        let added = result.len() - usize::from(replace);
        if self.recipe.primitives.len() + added > MAX_PRIMITIVES {
            return Err(newell_model::ModelError::InvalidSketch("capacity"));
        }
        let next = self
            .next_key
            .checked_add(added as u64)
            .ok_or(newell_model::ModelError::IdentityExhausted)?;
        let mut result = result.into_iter();
        if replace {
            let target = self
                .recipe
                .primitives
                .iter_mut()
                .find(|(id, _)| *id == key)
                .ok_or(newell_model::ModelError::InvalidSketch("missing line"))?;
            target.1 = result
                .next()
                .ok_or(newell_model::ModelError::InvalidSketch("empty edit"))?;
        }
        self.recipe
            .primitives
            .extend((self.next_key..next).zip(result));
        self.next_key = next;
        self.line_target = None;
        Ok(())
    }

    fn handles(&self, camera: Camera, exposed: &[newell_render::SceneViewport]) -> Vec<Handle> {
        let recipe = self.display_recipe();
        let mut handles: Vec<_> = recipe
            .primitives
            .iter()
            .filter_map(|&(key, item)| {
                let geometry = self.reference_picks.get(&key).copied()
                    .or_else(|| recipe.resolved(item).ok())?;
                Some(geometry.semantic_points(key))
            })
            .flatten()
            .filter_map(|(reference, local)| {
                let screen = project_point(recipe.support, camera, local)?;
                exposed.iter().any(|rect| rect.contains(screen[0], screen[1]))
                    .then_some(Handle { reference, local, screen })
            })
            .collect();
        handles.sort_by_key(|handle| (handle.reference.geometry(), handle.reference));
        handles
    }

    fn marker_hits(
        &self,
        point: (f64, f64),
        scale: f64,
        camera: Camera,
        exposed: &[newell_render::SceneViewport],
    ) -> Vec<Handle> {
        let distance = |h: &Handle| (point.0 - h.screen[0]).hypot(point.1 - h.screen[1]);
        let mut hits = self.handles(camera, exposed);
        hits.retain(|handle| distance(handle) <= 6.0 * scale);
        hits.sort_by(|a, b| distance(a).total_cmp(&distance(b)));
        hits
    }

    fn cycle_index(&self, hits: &[Handle], point: (f64, f64), scale: f64) -> Option<usize> {
        let cycle = self.marker_cycle.as_ref()?;
        ((point.0 - cycle.anchor.0).hypot(point.1 - cycle.anchor.1) <= 2.0 * scale
            && hits.iter().map(|hit| hit.reference).eq(cycle.choices.iter().copied()))
            .then_some(cycle.selected)
    }

    pub fn cycle_marker(
        &mut self,
        point: (f64, f64),
        scale: f64,
        camera: Camera,
        exposed: &[newell_render::SceneViewport],
        reverse: bool,
    ) -> Option<String> {
        if !self.is_select() || self.press.is_some() || self.solve.is_some() {
            return None;
        }
        let hits = self.marker_hits(point, scale, camera, exposed);
        if hits.len() < 2 {
            self.marker_cycle = None;
            return None;
        }
        let current = self.cycle_index(&hits, point, scale).unwrap_or(0);
        let step = if reverse { hits.len() - 1 } else { 1 };
        let selected = (current + step) % hits.len();
        let anchor = self.marker_cycle.as_ref().filter(|_| self.cycle_index(&hits, point, scale).is_some())
            .map_or(point, |cycle| cycle.anchor);
        self.marker_cycle = Some(MarkerCycle {
            anchor,
            choices: hits.iter().map(|hit| hit.reference).collect(),
            selected,
        });
        let reference = hits[selected].reference;
        self.selected = reference.geometry();
        self.selected_handle = Some(reference);
        Some(format!("Sketch point {reference:?}"))
    }

    pub fn reset_marker_cycle_after_motion(&mut self, point: (f64, f64), scale: f64) {
        if self.marker_cycle.as_ref().is_some_and(|cycle| {
            (point.0 - cycle.anchor.0).hypot(point.1 - cycle.anchor.1) > 2.0 * scale
        }) {
            self.marker_cycle = None;
        }
    }

    fn marker_hit(
        &self, point: (f64, f64), scale: f64, camera: Camera, exposed: &[newell_render::SceneViewport],
    ) -> Option<Handle> {
        let hits = self.marker_hits(point, scale, camera, exposed);
        hits.get(self.cycle_index(&hits, point, scale).unwrap_or(0)).copied()
    }

    pub fn select_click(
        &mut self, point: (f64, f64), scale: f64, camera: Camera, exposed: &[newell_render::SceneViewport],
    ) -> Option<String> {
        self.selected_handle = self.marker_hit(point, scale, camera, exposed).map(|h| h.reference);
        self.selected = self.selected_handle.and_then(HandleRef::geometry)
            .or_else(|| self.pick(point, scale, camera).0.map(|(key, _)| key));
        self.selected_handle.map(|reference| format!("Sketch point {reference:?}"))
    }

    fn display_recipe(&self) -> &SketchRecipe {
        &self.recipe
    }

    fn pick(
        &self,
        point: (f64, f64),
        scale: f64,
        camera: Camera,
    ) -> (Option<(u64, Primitive)>, bool) {
        let mut best = 5.0 * scale;
        let mut hit = None;
        let mut ambiguous = false;
        for &(key, item) in &self.recipe.primitives {
            let Ok(geometry) = self
                .reference_picks
                .get(&key)
                .copied()
                .map_or_else(|| self.recipe.resolved(item), Ok)
            else {
                continue;
            };
            for (a, b) in project_segments(self.recipe.support, geometry, camera) {
                let v = [b.0 - a.0, b.1 - a.1];
                let length = v[0].hypot(v[1]);
                if length <= 0.0 {
                    continue;
                }
                let d = [point.0 - a.0, point.1 - a.1];
                let t =
                    ((d[0] * (v[0] / length) + d[1] * (v[1] / length)) / length).clamp(0.0, 1.0);
                let distance = (d[0] - t * v[0]).hypot(d[1] - t * v[1]);
                if hit.is_some_and(|(id, _)| id != key) && (distance - best).abs() <= 1e-6 {
                    ambiguous = true;
                } else if distance < best {
                    best = distance;
                    hit = Some((key, item));
                    ambiguous = false;
                }
            }
        }
        (hit, ambiguous)
    }

    pub fn click(
        &mut self,
        local: [f64; 2],
        point: (f64, f64),
        scale: f64,
        camera: Camera,
    ) -> Result<(), newell_model::ModelError> {
        if !self.reference_preview.is_empty() {
            let items = self
                .reference_preview
                .iter()
                .map(|(item, _)| *item)
                .collect();
            self.append(items)?;
            self.set_tool(Tool::Select);
            return Ok(());
        }
        if let Some(source) = &self.copy_source {
            let copy = source.rigid_copy(
                self.recipe.support,
                local,
                self.numeric.unwrap_or(0.0).to_radians(),
            )?;
            let originals = self
                .numeric_formula
                .map(|_| source.rigid_copy(self.recipe.support, [0.0; 2], 0.0))
                .transpose()?;
            let first = self.next_key;
            self.append(copy)?;
            if let Some((parameter, originals)) = self.numeric_formula.zip(originals) {
                for (key, original) in (first..self.next_key).zip(originals) {
                    self.recipe.rigid_formulas.insert(
                        key,
                        RigidFormula {
                            parameter,
                            original,
                            translation: local,
                        },
                    );
                }
            }
            return Ok(());
        }
        let corner = matches!(self.tool, Tool::Draw(DrawTool::Fillet | DrawTool::Chamfer));
        if corner && let (Some(a), Some(b)) = (self.line_target, self.corner_second) {
            self.recipe.protect_dimension_operands(a)?;
            self.recipe.protect_dimension_operands(b)?;
            let (trims, join) = self.corner_result(a, b)?;
            let parameters = [self.numeric_formula, self.second_formula];
            let formula = if parameters.iter().any(Option::is_some) {
                Some(CornerFormula {
                    parameters,
                    values: [
                        self.numeric.unwrap_or(1.0),
                        self.second_numeric.unwrap_or(1.0),
                    ],
                    sources: [a, b],
                    segments: [self.source_line(a)?, self.source_line(b)?],
                    fillet: self.tool == Tool::Draw(DrawTool::Fillet),
                })
            } else {
                None
            };
            let result = self.next_key;
            self.append(vec![join])?;
            if let Some(formula) = formula {
                self.recipe.corner_formulas.insert(result, formula);
            }
            for (key, item) in &mut self.recipe.primitives {
                if *key == a {
                    *item = trims[0];
                }
                if *key == b {
                    *item = trims[1];
                }
            }
            self.clear_gesture();
            return Ok(());
        }
        let refine = self.tool == Tool::Draw(DrawTool::AddControlPoints);
        if refine && let Some(key) = self.line_target {
            let curve = self
                .refined(key)
                .ok_or(newell_model::ModelError::InvalidSketch(
                    "select a cubic Bezier or an unrefined spline",
                ))?;
            curve.validate()?;
            if let Some((_, item)) = self.recipe.primitives.iter_mut().find(|(id, _)| *id == key) {
                *item = Primitive::Draw(curve);
            }
            self.clear_gesture();
            return Ok(());
        }
        let transform = self.tool == Tool::Draw(DrawTool::Transform);
        if transform && let Some(key) = self.line_target {
            if self.start.is_some() {
                let result = self.transform_result(key, local)?;
                if self.transform_move {
                    self.recipe.protect_dimension_operands(key)?;
                    self.recipe.move_primitive(key, result)?;
                } else {
                    self.append(vec![result])?;
                }
                self.clear_gesture();
            } else {
                self.start = Some(local);
            }
            return Ok(());
        }
        let mirror = self.tool == Tool::Draw(DrawTool::Mirror);
        let pattern = matches!(
            self.tool,
            Tool::Draw(DrawTool::LinearPattern | DrawTool::CircularPattern)
        );
        if pattern && let Some(key) = self.line_target {
            if self.start.is_none() {
                self.start = Some(local);
            } else if self.tool == Tool::Draw(DrawTool::CircularPattern) && self.second.is_none() {
                let Some(start) = self.start else {
                    return Ok(());
                };
                Primitive::Line { start, end: local }.validate()?;
                self.second = Some(local);
            } else {
                let instances = self.pattern_result(key, local)?;
                self.append_pattern(instances)?;
                self.clear_gesture();
            }
            return Ok(());
        }
        if mirror && let Some(key) = self.line_target {
            if let Some(first) = self.start {
                let item = self.mirror_result(key, [first, local])?;
                self.append(vec![item])?;
                self.clear_gesture();
            } else {
                self.start = Some(local);
            }
            return Ok(());
        }
        if let Some(mode) = self.line_mode()
            && let Some(key) = self.line_target
        {
            return self.apply_line_edit(mode, key, local);
        }
        if self.tool == Tool::Select
            || mirror
            || transform
            || pattern
            || refine
            || corner
            || self.line_mode().is_some()
        {
            let (hit, ambiguous) = self.pick(point, scale, camera);
            if self.tool == Tool::Select {
                self.selected = hit.map(|(key, _)| key);
            } else {
                let (key, _) = hit
                    .filter(|(_, item)| {
                        !ambiguous
                            && if refine {
                                matches!(item, Primitive::Draw(draw) if draw.refine().is_some())
                            } else if pattern || mirror || transform {
                                item.is_independent()
                            } else if matches!(self.line_mode(), Some(LineEditMode::Trim | LineEditMode::Extend)) {
                                item.line().is_some() || matches!(item, Primitive::Draw(draw) if draw.can_trim_extend())
                            } else if self.line_mode() == Some(LineEditMode::Split) {
                                item.line().is_some()
                                    || matches!(item, Primitive::Draw(draw) if draw.can_split())
                            } else if self.line_mode() == Some(LineEditMode::Offset) {
                                item.line().is_some()
                                    || matches!(item, Primitive::Draw(draw) if draw.can_offset())
                            } else {
                                item.line().is_some()
                            }
                    })
                    .ok_or(newell_model::ModelError::InvalidSketch("select one line"))?;
                if corner && let Some(first) = self.line_target {
                    self.corner_result(first, key)?;
                    self.corner_second = Some(key);
                } else {
                    self.line_target = Some(key);
                }
            }
        } else if self.tool == Tool::Draw(DrawTool::TangentArc) && self.start.is_none() {
            let [u, v, _] = ui_plane(self.recipe.support).basis();
            let mut best = 5.0 * scale;
            let mut endpoint = None;
            let mut ambiguous = false;
            for &(_, item) in &self.recipe.primitives {
                let Some(endpoints) = item.endpoint_tangents() else {
                    continue;
                };
                for (tip, tangent) in endpoints {
                    if tangent[0].hypot(tangent[1]) < 1e-7 {
                        continue;
                    }
                    let world = std::array::from_fn(|i| {
                        tip[0] * f64::from(u[i]) + tip[1] * f64::from(v[i])
                    });
                    if let Some((x, y)) = camera.project(world) {
                        let distance = (x - point.0).hypot(y - point.1);
                        if endpoint.is_some() && (distance - best).abs() <= 1e-6 {
                            ambiguous = true;
                        } else if distance < best {
                            best = distance;
                            endpoint = Some((tip, tangent));
                            ambiguous = false;
                        }
                    }
                }
            }
            let Some((tip, tangent)) = endpoint.filter(|_| !ambiguous) else {
                return Err(newell_model::ModelError::InvalidSketch(
                    "select one endpoint with a nonzero tangent",
                ));
            };
            self.start = Some(tip);
            self.second = Some(tangent);
        } else if let Some(primitive) = self.candidate(local)? {
            if self.recipe.primitives.len() >= MAX_PRIMITIVES {
                return Err(newell_model::ModelError::InvalidSketch("capacity"));
            }
            let next = self
                .next_key
                .checked_add(1)
                .ok_or(newell_model::ModelError::IdentityExhausted)?;
            self.recipe.primitives.push((self.next_key, primitive));
            if let Some(parameter) = self.numeric_formula
                && matches!(
                    primitive,
                    Primitive::Draw(
                        DrawGeometry::Polygon { .. }
                            | DrawGeometry::Conic { .. }
                            | DrawGeometry::Text(_)
                    )
                )
            {
                self.recipe.formulas.insert(self.next_key, parameter);
            }
            self.next_key = next;
            self.chain = self.tool == Tool::Line;
            self.start = self.chain.then_some(local);
            self.second = None;
            self.third = None;
            self.fourth = None;
            self.fifth = None;
        } else if self.tool == Tool::Draw(DrawTool::Spline)
            && self.spline_handles
            && self.fourth.is_some()
            && self.fifth.is_none()
        {
            let Some(start) = self.start else {
                return Ok(());
            };
            Primitive::Line { start, end: local }.validate()?;
            self.fifth = Some(local);
        } else if let Some(center) = self.start.filter(|_| self.third.is_some()) {
            Primitive::Line {
                start: center,
                end: local,
            }
            .validate()?;
            self.fourth = Some(local);
        } else if let Some(second) = self.second {
            if self.tool == Tool::Draw(DrawTool::EllipticalArc) {
                let Some(center) = self.start else {
                    return Ok(());
                };
                DrawGeometry::ellipse(center, second, local).validate()?;
            }
            Primitive::Line {
                start: second,
                end: local,
            }
            .validate()?;
            self.third = Some(local);
        } else if let Some(start) = self.start {
            Primitive::Line { start, end: local }.validate()?;
            self.second = Some(local);
        } else {
            self.start = Some(local);
        }
        Ok(())
    }

    pub fn strokes(
        &self,
        camera: Camera,
        pointer: Option<[f64; 2]>,
    ) -> Vec<newell_render::SketchStroke> {
        let line_preview = pointer.and_then(|p| {
            self.line_result(self.line_mode()?, self.line_target?, p)
                .ok()
        });
        let copy_preview = pointer.and_then(|p| {
            if let Some(source) = &self.copy_source {
                source
                    .rigid_copy(
                        self.recipe.support,
                        p,
                        self.numeric.unwrap_or(0.0).to_radians(),
                    )
                    .ok()
            } else if self.tool == Tool::Draw(DrawTool::Mirror) {
                Some(vec![
                    self.mirror_result(self.line_target?, [self.start?, p])
                        .ok()?,
                ])
            } else if self.tool == Tool::Draw(DrawTool::Transform) {
                Some(vec![self.transform_result(self.line_target?, p).ok()?])
            } else if matches!(
                self.tool,
                Tool::Draw(DrawTool::LinearPattern | DrawTool::CircularPattern)
            ) {
                self.pattern_result(self.line_target?, p).ok()
            } else {
                None
            }
        });
        let controls = if self.tool == Tool::Draw(DrawTool::AddControlPoints) {
            self.line_target
                .and_then(|key| self.refined(key))
                .map(DrawGeometry::refinement_segments)
        } else {
            None
        };
        let corner_preview = self
            .line_target
            .zip(self.corner_second)
            .and_then(|(a, b)| self.corner_result(a, b).ok());
        let hide_target = (line_preview.is_some()
            && self.line_mode() != Some(LineEditMode::Offset))
            || (copy_preview.is_some()
                && self.tool == Tool::Draw(DrawTool::Transform)
                && self.transform_move);
        self.recipe
            .primitives
            .iter()
            .filter(|(key, _)| {
                (!hide_target || Some(*key) != self.line_target)
                    && (corner_preview.is_none()
                        || (Some(*key) != self.line_target && Some(*key) != self.corner_second))
            })
            .map(|&(key, item)| {
                (
                    item,
                    self.selected == Some(key)
                        || self.line_target == Some(key)
                        || self.dimension_target == Some(key),
                )
            })
            .chain(
                corner_preview
                    .into_iter()
                    .flat_map(|([a, b], join)| [(a, true), (b, true), (join, true)]),
            )
            .chain(copy_preview.into_iter().flatten().map(|item| (item, true)))
            .chain(line_preview.into_iter().flatten().map(|item| (item, true)))
            .chain(
                pointer
                    .and_then(|p| self.snap_preview().or_else(|| self.preview(p)))
                    .map(|item| (item, false)),
            )
            .flat_map(|(item, selected)| {
                self.recipe
                    .resolved(item)
                    .ok()
                    .into_iter()
                    .flat_map(move |geometry| {
                        primitive_strokes(self.recipe.support, geometry, camera, selected)
                    })
            })
            .chain(controls.into_iter().flatten().flat_map(|(start, end)| {
                primitive_strokes(
                    self.recipe.support,
                    Primitive::Line { start, end },
                    camera,
                    false,
                )
            }))
            .chain(self.reference_preview.iter().flat_map(|&(_, geometry)| {
                reference_strokes(self.recipe.support, geometry, camera)
            }))
            .collect()
    }
}

fn project_point(support: SketchSupport, camera: Camera, point: [f64; 2]) -> Option<[f64; 2]> {
    let [u, v, _] = ui_plane(support).basis();
    let (x, y) = camera.project(std::array::from_fn(|i| {
        f64::from(u[i]) * point[0] + f64::from(v[i]) * point[1]
    }))?;
    Some([x, y])
}

fn project_segments(
    support: SketchSupport,
    item: Primitive,
    camera: Camera,
) -> impl Iterator<Item = ((f64, f64), (f64, f64))> + use<> {
    let [u, v, _] = ui_plane(support).basis();
    let project = move |p: [f64; 2]| {
        camera.project(std::array::from_fn(|i| {
            f64::from(u[i]) * p[0] + f64::from(v[i]) * p[1]
        }))
    };
    if let Primitive::Draw(DrawGeometry::Point(point)) = item {
        let Some((x, y)) = project(point) else {
            return Vec::new().into_iter();
        };
        return vec![((x - 3., y), (x + 3., y)), ((x, y - 3.), (x, y + 3.))].into_iter();
    }
    let segments = match item {
        Primitive::PatternInstance { .. }
        | Primitive::Reference { .. }
        | Primitive::BrepReference { .. }
        | Primitive::BrepFaceSection { .. } => Vec::new(),
        Primitive::Draw(draw) => draw.display_segments(),
        Primitive::Line { start, end } | Primitive::EditedLine { start, end, .. } => {
            vec![(start, end)]
        }
        Primitive::Rectangle { min, max } => {
            let b = [max[0], min[1]];
            let d = [min[0], max[1]];
            vec![(min, b), (b, max), (max, d), (d, min)]
        }
    };
    segments
        .into_iter()
        .filter_map(|(a, b)| Some((project(a)?, project(b)?)))
        .collect::<Vec<_>>()
        .into_iter()
}

pub(crate) fn linked_strokes<'a>(
    recipe: &'a SketchRecipe,
    document: &'a newell_model::Document,
    camera: Camera,
) -> impl Iterator<Item = newell_render::SketchStroke> + 'a {
    recipe
        .primitives
        .iter()
        .filter_map(move |&(_, item)| document.resolve_sketch_reference(recipe.support, item).ok())
        .flat_map(move |geometry| reference_strokes(recipe.support, geometry, camera))
}

fn reference_strokes(
    support: SketchSupport,
    geometry: Primitive,
    camera: Camera,
) -> impl Iterator<Item = newell_render::SketchStroke> {
    project_segments(support, geometry, camera).flat_map(|(a, b)| {
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let count = ((dx.hypot(dy) / 12.0).ceil() as usize).clamp(1, 512);
        (0..count).map(move |i| {
            let t = i as f64 / count as f64;
            let u = (i as f64 + 0.55) / count as f64;
            let at = |s| [(a.0 + s * dx) as f32, (a.1 + s * dy) as f32];
            newell_render::SketchStroke {
                start: at(t),
                end: at(u),
                selected: false,
            }
        })
    })
}

pub(crate) fn dimension_strokes<'a>(
    recipe: &'a SketchRecipe,
    document: &'a newell_model::Document,
    camera: Camera,
    scale: f64,
) -> impl Iterator<Item = newell_render::SketchStroke> + 'a {
    use newell_model::{
        Quantity, Unit,
        sketch::{DrivenDimension, SegmentRelation},
    };
    let equations = recipe
        .segment_equations
        .iter()
        .filter_map(|(&id, equation)| {
            let text = match equation.relation {
                SegmentRelation::Horizontal => "H".to_owned(),
                SegmentRelation::Vertical => "V".to_owned(),
                SegmentRelation::Length(length) => {
                    Quantity::Length(length).display(Unit::Millimetre).ok()?
                }
            };
            Some((id, equation.segment, text, 1.0))
        });
    let driven = recipe
        .driven_dimensions
        .iter()
        .filter_map(move |(&id, dimension)| {
            let DrivenDimension::Length(segment) = *dimension else {
                return None;
            };
            let value = document
                .sketch_measurement(recipe, id)
                .ok()?
                .display(Unit::Millimetre)
                .ok()?;
            Some((id, segment, format!("({value})"), -1.0))
        });
    equations
        .chain(driven)
        .flat_map(move |(id, key, label, side)| {
            let result = (|| {
                let item = recipe
                    .primitives
                    .iter()
                    .find_map(|&(id, item)| (id == key).then_some(item))?;
                let (a, b) = project_segments(recipe.support, item, camera).next()?;
                let center = [(a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5];
                let is_constraint = label.len() == 1;
                let origin = [
                    center[0] + if is_constraint { 6.0 * scale } else { 0.0 },
                    center[1] - side * if is_constraint { 32.0 } else { 13.0 } * scale,
                ];
                let origin = if let Some(point) = recipe.dimension_positions.get(&id) {
                    let [u, v, _] = ui_plane(recipe.support).basis();
                    let (x, y) = camera.project(std::array::from_fn(|i| {
                        f64::from(u[i]) * point[0] + f64::from(v[i]) * point[1]
                    }))?;
                    [x, y]
                } else {
                    origin
                };
                let text = SketchText::new(
                    &label,
                    [origin[0], -origin[1]],
                    [origin[0] + 1.0, -origin[1]],
                    16.0 * scale,
                )
                .ok()?;
                let mut strokes = vec![newell_render::SketchStroke {
                    start: [center[0] as f32, center[1] as f32],
                    end: [origin[0] as f32, origin[1] as f32],
                    selected: false,
                }];
                strokes.extend(DrawGeometry::Text(text).display_segments().into_iter().map(
                    |(a, b)| newell_render::SketchStroke {
                        start: [a[0] as f32, -a[1] as f32],
                        end: [b[0] as f32, -b[1] as f32],
                        selected: false,
                    },
                ));
                Some(strokes)
            })();
            result.into_iter().flatten()
        })
}

pub(crate) fn saved_strokes(
    recipe: &SketchRecipe,
    camera: Camera,
) -> impl Iterator<Item = newell_render::SketchStroke> + '_ {
    recipe
        .primitives
        .iter()
        .filter_map(move |&(_, item)| recipe.resolved(item).ok())
        .flat_map(move |item| primitive_strokes(recipe.support, item, camera, false))
}

fn primitive_strokes(
    support: SketchSupport,
    item: Primitive,
    camera: Camera,
    selected: bool,
) -> impl Iterator<Item = newell_render::SketchStroke> {
    project_segments(support, item, camera).map(move |(a, b)| newell_render::SketchStroke {
        start: [a.0 as f32, a.1 as f32],
        end: [b.0 as f32, b.1 as f32],
        selected,
    })
}
