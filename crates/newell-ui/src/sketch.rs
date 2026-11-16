// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Built-in Sketch support and the window-local setup presentation.

use crate::action_box::{ActionBox, Row};
use crate::create_box::{ActionBoxUi, K_ACTIONS};
use crate::{
    ActionOperation, OriginReference, SelectionFieldDescriptor, SelectionFieldKey,
    SelectionFieldRole, WidgetId,
};

pub(crate) const WIDTH: u32 = 27;
const PLANE: u32 = 0;
pub(crate) const GRID: u32 = 1;
pub(crate) const ASSOCIATIVE: u32 = 10;
pub(crate) const TRANSFORM_MOVE: u32 = 11;
pub(crate) const SPLINE_HANDLES: u32 = 13;
pub(crate) const OFFSET_CHAIN: u32 = 14;
pub const SKETCH_COUNT: u32 = 2;
pub const SKETCH_WEIGHT: u32 = 3;
pub(crate) const SKETCH_RADIUS: u32 = 4;
pub(crate) const SKETCH_SETBACK: u32 = 5;
pub(crate) const SKETCH_SETBACK_TWO: u32 = 12;
pub(crate) const SKETCH_TEXT_HEIGHT: u32 = 6;
pub(crate) const SKETCH_LINEAR_COUNT: u32 = 7;
pub(crate) const SKETCH_CIRCULAR_COUNT: u32 = 8;
pub(crate) const SKETCH_DIMENSION_LENGTH: u32 = 20;
pub(crate) const SKETCH_INSERT_ANGLE: u32 = 9;
pub(crate) const LINE_ACTION: u32 = 601;
pub(crate) const RECTANGLE_ACTION: u32 = 602;

/// Stable identity of a built-in principal Sketch support.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginPlane {
    XY,
    XZ,
    YZ,
}

impl OriginPlane {
    pub const ALL: [Self; 3] = [Self::XY, Self::XZ, Self::YZ];

    pub const fn from_reference(reference: OriginReference) -> Option<Self> {
        match reference {
            OriginReference::XY => Some(Self::XY),
            OriginReference::XZ => Some(Self::XZ),
            OriginReference::YZ => Some(Self::YZ),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::XY => OriginReference::XY.label(),
            Self::XZ => OriginReference::XZ.label(),
            Self::YZ => OriginReference::YZ.label(),
        }
    }

    /// The world-space (u, v, normal) basis at the world origin.
    pub const fn basis(self) -> [[i8; 3]; 3] {
        match self {
            Self::XY => [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
            Self::XZ => [[1, 0, 0], [0, 0, 1], [0, -1, 0]],
            Self::YZ => [[0, 1, 0], [0, 0, 1], [1, 0, 0]],
        }
    }
}

/// Accepted Sketch state projected from the window controller.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SketchMode {
    #[default]
    Closed,
    Setup(Option<OriginPlane>),
    Active(OriginPlane),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawTool {
    Point,
    Polygon,
    Slot,
    CenterRectangle,
    OrientedRectangle,
    Circle,
    Circumcircle,
    Ellipse,
    ThreePointArc,
    CenterArc,
    Bezier,
    TangentArc,
    EllipticalArc,
    Split,
    Offset,
    Trim,
    Extend,
    Mirror,
    Insert,
    Conic,
    LinearPattern,
    CircularPattern,
    Projected,
    Intersect,
    Spline,
    AddControlPoints,
    Fillet,
    Chamfer,
    Transform,
    Text,
    DrivingLength,
    DrivenLength,
    Horizontal,
    Vertical,
}

impl DrawTool {
    pub const fn second_numeric_key(self) -> Option<u32> {
        match self {
            Self::Chamfer => Some(SKETCH_SETBACK_TWO),
            _ => None,
        }
    }

    pub(crate) fn has_numeric_key(self, key: u32) -> bool {
        self.numeric_key() == Some(key) || self.second_numeric_key() == Some(key)
    }

    pub const fn numeric_key(self) -> Option<u32> {
        match self {
            Self::Polygon => Some(SKETCH_COUNT),
            Self::LinearPattern => Some(SKETCH_LINEAR_COUNT),
            Self::CircularPattern => Some(SKETCH_CIRCULAR_COUNT),
            Self::Conic => Some(SKETCH_WEIGHT),
            Self::Fillet => Some(SKETCH_RADIUS),
            Self::Chamfer => Some(SKETCH_SETBACK),
            Self::Text => Some(SKETCH_TEXT_HEIGHT),
            Self::Insert => Some(SKETCH_INSERT_ANGLE),
            Self::DrivingLength => Some(SKETCH_DIMENSION_LENGTH),
            _ => None,
        }
    }

    pub(crate) const fn action(self) -> u32 {
        603 + self as u32
    }

    pub(crate) fn from_action(action: u32) -> Option<Self> {
        [
            Self::Point,
            Self::Polygon,
            Self::Slot,
            Self::CenterRectangle,
            Self::OrientedRectangle,
            Self::Circle,
            Self::Circumcircle,
            Self::Ellipse,
            Self::ThreePointArc,
            Self::CenterArc,
            Self::Bezier,
            Self::TangentArc,
            Self::EllipticalArc,
            Self::Split,
            Self::Offset,
            Self::Trim,
            Self::Extend,
            Self::Mirror,
            Self::Insert,
            Self::Conic,
            Self::LinearPattern,
            Self::CircularPattern,
            Self::Projected,
            Self::Intersect,
            Self::Spline,
            Self::AddControlPoints,
            Self::Fillet,
            Self::Chamfer,
            Self::Transform,
            Self::Text,
            Self::DrivingLength,
            Self::DrivenLength,
            Self::Horizontal,
            Self::Vertical,
        ]
        .into_iter()
        .find(|tool| tool.action() == action)
    }
}

/// A local Sketch lifecycle request, separate from the CAD commit path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SketchIntent {
    Confirm,
    Cancel,
    Escape,
    Delete,
    Detach,
    Line,
    Rectangle,
    Draw(DrawTool),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SketchUi {
    base: u32,
    pub(crate) mode: SketchMode,
    pub(crate) ready: bool,
    pub(crate) grid: bool,
    pub(crate) tool: Option<DrawTool>,
    pub(crate) text: String,
    pub(crate) associative: bool,
    pub(crate) transform_move: bool,
    pub(crate) spline_handles: bool,
    pub(crate) offset_chain: bool,
}

impl SketchUi {
    pub(crate) const fn new(base: u32) -> Self {
        Self {
            base,
            mode: SketchMode::Closed,
            ready: false,
            grid: false,
            tool: None,
            text: String::new(),
            associative: false,
            transform_move: false,
            spline_handles: false,
            offset_chain: false,
        }
    }
}

impl ActionBoxUi for SketchUi {
    fn base_id(&self) -> u32 {
        self.base
    }

    fn compose_box(&self) -> Option<ActionBox> {
        let (support, active) = match self.mode {
            SketchMode::Setup(support) => (support, false),
            SketchMode::Active(plane) => (Some(plane), true),
            SketchMode::Closed => return None,
        };
        let row = || Row::with_width(self.base, WIDTH);
        let mut panel = ActionBox::default();
        panel.push(
            row()
                .contour("╭─")
                .text("Sketch")
                .contour("──────────────────╮"),
        );
        let plane = row().contour("│").text(" Plane   ");
        panel.push(if active {
            plane
                .text(&format!("[{:<14}]", support.map_or("", OriginPlane::label)))
                .end()
        } else {
            plane
                .select_or(PLANE, 0, support.map_or("Select…", OriginPlane::label))
                .end()
        });
        for controls in [
            &["Construction", "Slice"][..],
            &[
                "Sketch Grid",
                "Profile",
                "Point",
                "Dimension",
                "Constraints",
                "Projected Geometries",
                "Construction",
            ],
        ] {
            panel.push(row().contour("│─────────────────────────│"));
            for label in controls {
                let control = row().contour("│");
                panel.push(if active && *label == "Sketch Grid" {
                    control.text(" ").check_fill(GRID, self.grid, label).end()
                } else {
                    control.text(&format!(" □ {label}")).end()
                });
            }
        }
        if let Some(key) = self.tool.and_then(DrawTool::numeric_key)
            && let Some(field) = self.numeric_descriptor(key)
        {
            let label = match self.tool {
                Some(DrawTool::Polygon) => " Sides   ",
                Some(DrawTool::Conic) => " Weight  ",
                Some(DrawTool::Fillet) => " Radius  ",
                Some(DrawTool::Chamfer) => " First   ",
                Some(DrawTool::Text) => " Height  ",
                Some(DrawTool::Insert) => " Angle   ",
                Some(DrawTool::DrivingLength) => " Length  ",
                _ => " Count   ",
            };
            panel.push(row().contour("│─────────────────────────│"));
            panel.push(row().contour("│").text(label).numeric(key, field).end());
        }
        if let Some(key) = self.tool.and_then(DrawTool::second_numeric_key)
            && let Some(field) = self.numeric_descriptor(key)
        {
            panel.push(
                row()
                    .contour("│")
                    .text(" Second  ")
                    .numeric(key, field)
                    .end(),
            );
        }
        panel.push(row().contour("│─────────────────────────│"));
        if self.tool == Some(DrawTool::Offset) {
            panel.push(
                row()
                    .contour("│")
                    .text(" ")
                    .check_fill(OFFSET_CHAIN, self.offset_chain, "Connected lines")
                    .end(),
            );
            panel.push(row().contour("│─────────────────────────│"));
        }
        if self.tool == Some(DrawTool::Spline) {
            panel.push(
                row()
                    .contour("│")
                    .text(" ")
                    .check_fill(SPLINE_HANDLES, self.spline_handles, "Endpoint tangents")
                    .end(),
            );
            panel.push(row().contour("│─────────────────────────│"));
        }
        if self.tool == Some(DrawTool::Transform) {
            panel.push(
                row()
                    .contour("│")
                    .text(" ")
                    .check_fill(TRANSFORM_MOVE, self.transform_move, "Move source")
                    .end(),
            );
            panel.push(row().contour("│─────────────────────────│"));
        }
        if self.tool == Some(DrawTool::Mirror) {
            panel.push(
                row()
                    .contour("│")
                    .text(" ")
                    .check_fill(ASSOCIATIVE, self.associative, "Associative")
                    .end(),
            );
            panel.push(row().contour("│─────────────────────────│"));
        }
        if self.tool == Some(DrawTool::Text) {
            let tail: String = self
                .text
                .chars()
                .rev()
                .take(16)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            panel.push(row().contour("│").text(&format!(" Text    {tail}")).end());
            panel.push(row().contour("│─────────────────────────│"));
        }
        let footer = row().contour("│");
        let footer = if if active {
            self.ready
        } else {
            support.is_some()
        } {
            footer
                .action_button(K_ACTIONS, 0, "OK", 12)
                .divider('│', K_ACTIONS, 0, 1)
        } else {
            footer.text("     OK     ").contour("│")
        };
        panel.push(footer.action_button(K_ACTIONS, 1, "Cancel", 12).end());
        panel.push(row().contour("╰─────────────────────────╯"));
        Some(panel)
    }

    fn activate_id(&mut self, _id: WidgetId) -> bool {
        false
    }
    fn showing(&self) -> bool {
        self.mode != SketchMode::Closed
    }
    fn restart(&mut self) {}
    fn numeric_operation(&self) -> ActionOperation {
        ActionOperation::Sketch
    }
    fn select_keys(&self) -> &'static [u32] {
        &[PLANE]
    }
    fn selection_descriptor(&self, key: u32) -> Option<SelectionFieldDescriptor> {
        (key == PLANE).then_some(SelectionFieldDescriptor {
            field: SelectionFieldKey {
                operation: ActionOperation::Sketch,
                key,
            },
            role: SelectionFieldRole::SketchPlane,
            ordered: false,
            max_count: Some(1),
        })
    }
    fn set_selection_counts(&mut self, _counts: &std::collections::BTreeMap<u32, usize>) {}

    fn live_control(&self, id: WidgetId) -> Option<(u32, u32)> {
        match (self.mode, self.decode(id)?) {
            (SketchMode::Setup(_), (PLANE, 0)) => Some((PLANE, 0)),
            (SketchMode::Active(_), (key, slot))
                if slot <= 1 && self.tool.is_some_and(|tool| tool.has_numeric_key(key)) =>
            {
                Some((key, slot))
            }
            (SketchMode::Active(_), (OFFSET_CHAIN, 0)) if self.tool == Some(DrawTool::Offset) => {
                Some((OFFSET_CHAIN, 0))
            }
            (SketchMode::Active(_), (SPLINE_HANDLES, 0)) if self.tool == Some(DrawTool::Spline) => {
                Some((SPLINE_HANDLES, 0))
            }
            (SketchMode::Active(_), (TRANSFORM_MOVE, 0))
                if self.tool == Some(DrawTool::Transform) =>
            {
                Some((TRANSFORM_MOVE, 0))
            }
            (SketchMode::Active(_), (ASSOCIATIVE, 0)) if self.tool == Some(DrawTool::Mirror) => {
                Some((ASSOCIATIVE, 0))
            }
            (SketchMode::Active(_), (GRID, 0)) => Some((GRID, 0)),
            (SketchMode::Setup(Some(_)), (K_ACTIONS, 0)) => Some((K_ACTIONS, 0)),
            (SketchMode::Active(_), (K_ACTIONS, 0)) if self.ready => Some((K_ACTIONS, 0)),
            (mode, (K_ACTIONS, 1)) if mode != SketchMode::Closed => Some((K_ACTIONS, 1)),
            _ => None,
        }
    }
}
