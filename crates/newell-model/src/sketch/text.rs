// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use crate::ModelError;
use ttf_parser::{Face, OutlineBuilder};

type Point = [f64; 2];
type Segment = (Point, Point);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextEdge {
    Line([Point; 2]),
    Quadratic([Point; 3]),
    Cubic([Point; 4]),
}

/// Bounded UTF-8 text in the bundled Fira Code Regular face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SketchText {
    utf8: [u8; 64],
    len: u8,
    origin: Point,
    radians: f64,
    height: f64,
    reflected: bool,
}

fn invalid() -> ModelError {
    ModelError::InvalidSketch(
        "text needs 1–64 UTF-8 bytes, supported glyphs and a valid baseline and height",
    )
}

impl SketchText {
    pub fn new(
        text: &str,
        origin: Point,
        direction: Point,
        height: f64,
    ) -> Result<Self, ModelError> {
        if text.len() > 64
            || text.trim().is_empty()
            || text.bytes().any(|b| !(b' '..=b'~').contains(&b))
        {
            return Err(invalid());
        }
        let delta = [direction[0] - origin[0], direction[1] - origin[1]];
        if delta[0].hypot(delta[1]) < 1e-7 {
            return Err(invalid());
        }
        let mut utf8 = [0; 64];
        utf8[..text.len()].copy_from_slice(text.as_bytes());
        let result = Self {
            utf8,
            len: text.len() as u8,
            origin,
            radians: delta[1].atan2(delta[0]),
            height,
            reflected: false,
        };
        result.validate()?;
        Ok(result)
    }

    pub(super) fn placement_frame(self) -> Segment {
        let (sin, cos) = self.radians.sin_cos();
        (
            self.origin,
            [
                self.origin[0] + self.height * cos,
                self.origin[1] + self.height * sin,
            ],
        )
    }

    pub(super) fn with_height(mut self, height: f64) -> Result<Self, ModelError> {
        self.height = height;
        self.validate()?;
        Ok(self)
    }

    pub(super) fn transformed(mut self, transform: super::Similarity) -> Self {
        self.origin = transform.point(self.origin);
        self.radians = transform.radians
            + if transform.reflected {
                -self.radians
            } else {
                self.radians
            };
        self.height *= transform.scale;
        self.reflected ^= transform.reflected;
        self
    }

    fn source(&self) -> Result<&str, ModelError> {
        std::str::from_utf8(&self.utf8[..usize::from(self.len)]).map_err(|_| invalid())
    }

    fn font() -> Result<Face<'static>, ModelError> {
        Face::parse(include_bytes!("../../../../assets/FiraCode-Regular.ttf"), 0)
            .map_err(|_| invalid())
    }

    pub(super) fn validate(self) -> Result<(), ModelError> {
        if !self.origin.into_iter().all(f64::is_finite)
            || !self.radians.is_finite()
            || !self.height.is_finite()
            || self.height < 1e-7
        {
            return Err(invalid());
        }
        let font = Self::font()?;
        if self
            .source()?
            .chars()
            .any(|c| font.glyph_index(c).is_none())
        {
            return Err(invalid());
        }
        Ok(())
    }

    pub fn contours(self) -> Result<Vec<Vec<TextEdge>>, ModelError> {
        let font = Self::font()?;
        let source = self.source()?;
        let mut outline = FontOutline {
            pen: 0.0,
            current: [0.0; 2],
            first: [0.0; 2],
            edges: Vec::new(),
            contours: Vec::new(),
        };
        for c in source.chars() {
            let glyph = font.glyph_index(c).ok_or_else(invalid)?;
            font.outline_glyph(glyph, &mut outline);
            outline.pen += f64::from(font.glyph_hor_advance(glyph).unwrap_or(0));
        }
        let scale = self.height / f64::from(font.units_per_em());
        let (sin, cos) = self.radians.sin_cos();
        let transform = |[x, y]: Point| {
            let y = if self.reflected { -y } else { y };
            [
                self.origin[0] + scale * (x * cos - y * sin),
                self.origin[1] + scale * (x * sin + y * cos),
            ]
        };
        for contour in &mut outline.contours {
            for edge in contour {
                *edge = match *edge {
                    TextEdge::Line(p) => TextEdge::Line(p.map(transform)),
                    TextEdge::Quadratic(p) => TextEdge::Quadratic(p.map(transform)),
                    TextEdge::Cubic(p) => TextEdge::Cubic(p.map(transform)),
                };
            }
        }
        Ok(outline.contours)
    }

    pub(super) fn display_segments(self) -> Vec<Segment> {
        let mut result = Vec::new();
        fn chord<const N: usize>(controls: [Point; N], output: &mut Vec<Segment>) {
            let mut previous = controls[0];
            for i in 1..=12 {
                let point = super::DrawGeometry::evaluate_bezier(controls, f64::from(i) / 12.0);
                output.push((previous, point));
                previous = point;
            }
        }
        for edge in self.contours().into_iter().flatten().flatten() {
            match edge {
                TextEdge::Line([a, b]) => result.push((a, b)),
                TextEdge::Quadratic(points) => chord(points, &mut result),
                TextEdge::Cubic(points) => chord(points, &mut result),
            }
        }
        result
    }
}

struct FontOutline {
    pen: f64,
    current: Point,
    first: Point,
    edges: Vec<TextEdge>,
    contours: Vec<Vec<TextEdge>>,
}

impl FontOutline {
    fn point(&self, x: f32, y: f32) -> Point {
        [self.pen + f64::from(x), f64::from(y)]
    }

    fn line(&mut self, end: Point) {
        if end != self.current {
            self.edges.push(TextEdge::Line([self.current, end]));
        }
        self.current = end;
    }
}

impl OutlineBuilder for FontOutline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.current = self.point(x, y);
        self.first = self.current;
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.line(self.point(x, y));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let end = self.point(x, y);
        self.edges
            .push(TextEdge::Quadratic([self.current, self.point(x1, y1), end]));
        self.current = end;
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let end = self.point(x, y);
        self.edges.push(TextEdge::Cubic([
            self.current,
            self.point(x1, y1),
            self.point(x2, y2),
            end,
        ]));
        self.current = end;
    }
    fn close(&mut self) {
        self.line(self.first);
        if !self.edges.is_empty() {
            self.contours.push(std::mem::take(&mut self.edges));
        }
    }
}
