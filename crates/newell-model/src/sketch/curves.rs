// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use std::f64::consts::{PI, TAU};

use crate::ModelError;

type Point = [f64; 2];
type Segment = (Point, Point);

const MIN_LENGTH: f64 = 1e-7;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DrawGeometry {
    Point(Point),
    Text(super::SketchText),
    Polygon {
        center: Point,
        vertex: Point,
        sides: u16,
    },
    OrientedRectangle {
        start: Point,
        end: Point,
        width: f64,
    },
    Slot {
        start: Point,
        end: Point,
        radius: f64,
    },
    Ellipse {
        center: Point,
        major: Point,
        minor: f64,
    },
    Arc {
        center: Point,
        radial: Point,
        sweep: f64,
    },
    EllipticalArc {
        center: Point,
        major: Point,
        minor: f64,
        start: f64,
        sweep: f64,
    },
    Conic {
        start: Point,
        control: Point,
        end: Point,
        weight: f64,
    },
    Bezier {
        controls: [Point; 4],
    },
    ElevatedBezier {
        controls: [Point; 5],
    },
    Spline {
        fit: [Point; 4],
        tangents: Option<[Point; 2]>,
        refined: bool,
    },
}

fn add(a: Point, b: Point) -> Point {
    [a[0] + b[0], a[1] + b[1]]
}

fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}

fn mul(p: Point, scale: f64) -> Point {
    [p[0] * scale, p[1] * scale]
}

fn normal(p: Point) -> Point {
    [-p[1], p[0]]
}

fn length(p: Point) -> f64 {
    p[0].hypot(p[1])
}

fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn signed_height(a: Point, b: Point, c: Point) -> f64 {
    let base = sub(b, a);
    dot(mul(normal(base), 1.0 / length(base)), sub(c, a))
}

impl DrawGeometry {
    pub(super) fn transformed(self, transform: super::Similarity) -> Self {
        let handedness = if transform.reflected { -1.0 } else { 1.0 };
        match self {
            Self::Text(text) => Self::Text(text.transformed(transform)),
            Self::Point(point) => Self::Point(transform.point(point)),
            Self::Polygon {
                center,
                vertex,
                sides,
            } => Self::Polygon {
                center: transform.point(center),
                vertex: transform.point(vertex),
                sides,
            },
            Self::OrientedRectangle { start, end, width } => Self::OrientedRectangle {
                start: transform.point(start),
                end: transform.point(end),
                width: width * transform.scale * handedness,
            },
            Self::Slot { start, end, radius } => Self::Slot {
                start: transform.point(start),
                end: transform.point(end),
                radius: radius * transform.scale,
            },
            Self::Ellipse {
                center,
                major,
                minor,
            } => Self::Ellipse {
                center: transform.point(center),
                major: transform.vector(major),
                minor: minor * transform.scale,
            },
            Self::Arc {
                center,
                radial,
                sweep,
            } => Self::Arc {
                center: transform.point(center),
                radial: transform.vector(radial),
                sweep: sweep * handedness,
            },
            Self::EllipticalArc {
                center,
                major,
                minor,
                start,
                sweep,
            } => Self::EllipticalArc {
                center: transform.point(center),
                major: transform.vector(major),
                minor: minor * transform.scale,
                start: start * handedness,
                sweep: sweep * handedness,
            },
            Self::Conic {
                start,
                control,
                end,
                weight,
            } => Self::Conic {
                start: transform.point(start),
                control: transform.point(control),
                end: transform.point(end),
                weight,
            },
            Self::ElevatedBezier { controls } => Self::ElevatedBezier {
                controls: controls.map(|p| transform.point(p)),
            },
            Self::Spline {
                fit,
                tangents,
                refined,
            } => Self::Spline {
                fit: fit.map(|p| transform.point(p)),
                tangents: tangents.map(|t| t.map(|p| transform.vector(p))),
                refined,
            },
            Self::Bezier { controls } => Self::Bezier {
                controls: controls.map(|p| transform.point(p)),
            },
        }
    }

    pub fn circle(center: Point, edge: Point) -> Self {
        let major = sub(edge, center);
        Self::Ellipse {
            center,
            major,
            minor: length(major),
        }
    }

    pub fn circumcircle(a: Point, b: Point, c: Point) -> Result<Self, ModelError> {
        let ab = sub(b, a);
        let ac = sub(c, a);
        let span = length(ab);
        let height = signed_height(a, b, c);
        if !span.is_finite()
            || span < MIN_LENGTH
            || !height.is_finite()
            || height.abs() < MIN_LENGTH.max(1e-12 * span.max(length(ac)))
        {
            return Err(ModelError::InvalidSketch(
                "three noncollinear points required",
            ));
        }
        let u = mul(ab, 1.0 / span);
        let along = dot(ac, u);
        let across = 0.5 * (height + (along / height) * (along - span));
        let center = add(a, add(mul(u, span * 0.5), mul(normal(u), across)));
        let circle = Self::circle(center, a);
        circle.validate()?;
        Ok(circle)
    }

    pub fn ellipse(center: Point, major_end: Point, side: Point) -> Self {
        Self::Ellipse {
            center,
            major: sub(major_end, center),
            minor: signed_height(center, major_end, side).abs(),
        }
    }

    pub fn oriented_rectangle(start: Point, end: Point, side: Point) -> Self {
        Self::OrientedRectangle {
            start,
            end,
            width: signed_height(start, end, side),
        }
    }

    pub fn slot(start: Point, end: Point, side: Point) -> Self {
        Self::Slot {
            start,
            end,
            radius: signed_height(start, end, side).abs(),
        }
    }

    pub fn center_arc(center: Point, start: Point, end: Point) -> Self {
        let radial = sub(start, center);
        Self::Arc {
            center,
            radial,
            sweep: angle_from(radial, sub(end, center)),
        }
    }

    pub fn three_point_arc(start: Point, through: Point, end: Point) -> Result<Self, ModelError> {
        let Self::Ellipse {
            center,
            major: radial,
            ..
        } = Self::circumcircle(start, through, end)?
        else {
            return Err(ModelError::InvalidSketch("expected circumcircle"));
        };
        let sweep = angle_from(radial, sub(end, center));
        let through_angle = angle_from(radial, sub(through, center));
        Ok(Self::Arc {
            center,
            radial,
            sweep: if through_angle < sweep {
                sweep
            } else {
                sweep - TAU
            },
        })
    }

    pub(super) fn placement_frame(self) -> Segment {
        match self {
            Self::Point(p) => (p, add(p, [1.0, 0.0])),
            Self::Text(text) => text.placement_frame(),
            Self::Polygon { center, vertex, .. } => (center, vertex),
            Self::OrientedRectangle { start, end, .. }
            | Self::Slot { start, end, .. }
            | Self::Conic { start, end, .. } => (start, end),
            Self::Ellipse { center, major, .. } | Self::EllipticalArc { center, major, .. } => {
                (center, add(center, major))
            }
            Self::Arc { center, radial, .. } => (center, add(center, radial)),
            Self::Bezier { controls } => control_frame(controls),
            Self::ElevatedBezier { controls } => control_frame(controls),
            Self::Spline { fit, .. } => (fit[0], fit[1]),
        }
    }

    pub fn endpoint_tangents(self) -> Option<[(Point, Point); 2]> {
        let outward =
            |first, next, previous, last| [(first, sub(first, next)), (last, sub(last, previous))];
        match self {
            Self::Arc {
                center,
                radial,
                sweep,
            } => {
                let (sin, cos) = sweep.sin_cos();
                let end = add(mul(radial, cos), mul(normal(radial), sin));
                Some([
                    (add(center, radial), mul(normal(radial), -sweep.signum())),
                    (add(center, end), mul(normal(end), sweep.signum())),
                ])
            }
            Self::EllipticalArc {
                center,
                major,
                minor,
                start,
                sweep,
            } => {
                let y = mul(normal(major), minor / length(major));
                let at = |t: f64, sign| {
                    let (sin, cos) = t.sin_cos();
                    (
                        add(center, add(mul(major, cos), mul(y, sin))),
                        mul(add(mul(major, -sin), mul(y, cos)), sign),
                    )
                };
                Some([
                    at(start, -sweep.signum()),
                    at(start + sweep, sweep.signum()),
                ])
            }
            Self::Conic {
                start,
                control,
                end,
                ..
            } => Some(outward(start, control, control, end)),
            Self::Bezier {
                controls: [a, b, c, d],
            } => Some(outward(a, b, c, d)),
            Self::ElevatedBezier {
                controls: [a, b, _, d, e],
            } => Some(outward(a, b, d, e)),
            Self::Spline {
                fit: [a, b, c, d],
                tangents,
                ..
            } => Some(tangents.map_or_else(
                || outward(a, b, c, d),
                |[start, end]| [(a, mul(start, -1.0)), (d, end)],
            )),
            _ => None,
        }
    }

    pub fn can_trim_extend(self) -> bool {
        matches!(self, Self::Arc { .. })
    }

    /// Trim or extend a circular arc at crossings with finite straight boundaries.
    pub fn cut_arc(
        self,
        mode: LineEditMode,
        pointer: Point,
        boundaries: impl IntoIterator<Item = Segment>,
    ) -> Result<Vec<Self>, ModelError> {
        let invalid = |message| ModelError::InvalidSketch(message);
        let Self::Arc {
            center,
            radial,
            sweep,
        } = self
        else {
            return Err(invalid("select a circular arc"));
        };
        let radius = length(radial);
        let direction = sweep.signum();
        let end = sweep.abs();
        let eps = (MIN_LENGTH / radius).max(1e-12);
        let angle = |p: Point| {
            let p = sub(p, center);
            (direction * (p[1].atan2(p[0]) - radial[1].atan2(radial[0]))).rem_euclid(TAU)
        };
        if length(sub(pointer, center)) < MIN_LENGTH {
            return Err(invalid("select an arc interval or end"));
        }
        let mut crossings = Vec::new();
        for (a, b) in boundaries {
            let delta = sub(b, a);
            let span = length(delta);
            let u = mul(delta, 1.0 / span);
            let origin = sub(a, center);
            let foot = -dot(origin, u);
            let distance = cross(origin, u).abs();
            if distance >= radius - MIN_LENGTH {
                continue;
            }
            let half = ((radius - distance) * (radius + distance)).sqrt();
            for t in [foot - half, foot + half] {
                if (-MIN_LENGTH..=span + MIN_LENGTH).contains(&t) {
                    crossings.push(angle(add(a, mul(u, t.clamp(0.0, span)))));
                }
            }
        }
        crossings.sort_by(f64::total_cmp);
        crossings.dedup_by(|a, b| (*a - *b).abs() <= eps);
        let make = |a: f64, b: f64| -> Result<Self, ModelError> {
            let (sin, cos) = (direction * a).sin_cos();
            let item = Self::Arc {
                center,
                radial: add(mul(radial, cos), mul(normal(radial), sin)),
                sweep: direction * (b - a),
            };
            item.validate()?;
            Ok(item)
        };
        let at = angle(pointer);
        if mode == LineEditMode::Extend {
            let mut outside = crossings
                .into_iter()
                .filter(|&t| t > end + eps && t < TAU - eps);
            return if at <= end * 0.5 || (at > end && TAU - at < at - end) {
                let cut = outside
                    .next_back()
                    .ok_or_else(|| invalid("no crossing beyond the selected arc end"))?;
                Ok(vec![make(cut - TAU, end)?])
            } else {
                let cut = outside
                    .next()
                    .ok_or_else(|| invalid("no crossing beyond the selected arc end"))?;
                Ok(vec![make(0.0, cut)?])
            };
        }
        if mode != LineEditMode::Trim {
            return Err(invalid("invalid arc cut operation"));
        }
        crossings.retain(|&t| t > eps && t < end - eps);
        if crossings.is_empty() || crossings.iter().any(|&t| (t - at).abs() <= eps) {
            return Err(invalid("select an arc interval between distinct crossings"));
        }
        let cuts: Vec<_> = std::iter::once(0.0)
            .chain(crossings)
            .chain(std::iter::once(end))
            .collect();
        let removed = cuts
            .windows(2)
            .find(|pair| pair[0] < at && at < pair[1])
            .ok_or_else(|| invalid("select the arc interval to remove"))?;
        let mut result = Vec::with_capacity(2);
        if removed[0] > eps {
            result.push(make(0.0, removed[0])?);
        }
        if end - removed[1] > eps {
            result.push(make(removed[1], end)?);
        }
        Ok(result)
    }

    pub fn can_split(self) -> bool {
        matches!(self, Self::Arc { .. } | Self::EllipticalArc { .. })
    }

    pub fn split_arc(self, pointer: Point) -> Result<[Self; 2], ModelError> {
        let invalid = || ModelError::InvalidSketch("select an interior arc direction");
        let (center, axis, minor, start, sweep) = match self {
            Self::Arc {
                center,
                radial,
                sweep,
            } => (center, radial, length(radial), 0.0, sweep),
            Self::EllipticalArc {
                center,
                major,
                minor,
                start,
                sweep,
            } => (center, major, minor, start, sweep),
            _ => return Err(invalid()),
        };
        let delta = sub(pointer, center);
        if length(delta) < MIN_LENGTH {
            return Err(invalid());
        }
        let major = length(axis);
        let u = mul(axis, 1.0 / major);
        let t = (dot(delta, normal(u)) / minor).atan2(dot(delta, u) / major);
        let relative = ((t - start) * sweep.signum()).rem_euclid(TAU) * sweep.signum();
        if relative.abs() <= 1e-12 || relative.abs() >= sweep.abs() - 1e-12 {
            return Err(invalid());
        }
        let results = if matches!(self, Self::Arc { .. }) {
            let (sin, cos) = relative.sin_cos();
            [
                Self::Arc {
                    center,
                    radial: axis,
                    sweep: relative,
                },
                Self::Arc {
                    center,
                    radial: add(mul(axis, cos), mul(normal(axis), sin)),
                    sweep: sweep - relative,
                },
            ]
        } else {
            [
                Self::EllipticalArc {
                    center,
                    major: axis,
                    minor,
                    start,
                    sweep: relative,
                },
                Self::EllipticalArc {
                    center,
                    major: axis,
                    minor,
                    start: start + relative,
                    sweep: sweep - relative,
                },
            ]
        };
        for item in results {
            item.validate()?;
        }
        Ok(results)
    }

    pub fn can_offset(self) -> bool {
        match self {
            Self::Arc { .. } => true,
            Self::Ellipse { major, minor, .. } => (length(major) - minor).abs() <= minor * 1e-12,
            _ => false,
        }
    }

    pub fn offset_circular(self, pointer: Point) -> Result<Self, ModelError> {
        let (center, radial) = match self {
            Self::Arc { center, radial, .. } => (center, radial),
            Self::Ellipse { center, major, .. } if self.can_offset() => (center, major),
            _ => {
                return Err(ModelError::InvalidSketch(
                    "offset supports lines, circles and circular arcs",
                ));
            }
        };
        let radius = length(sub(pointer, center));
        let old = length(radial);
        if (radius - old).abs() < MIN_LENGTH || radius < MIN_LENGTH {
            return Err(ModelError::InvalidSketch(
                "offset must change the radius and stay positive",
            ));
        }
        let radial = mul(radial, radius / old);
        let result = if let Self::Arc { sweep, .. } = self {
            Self::Arc {
                center,
                radial,
                sweep,
            }
        } else {
            Self::Ellipse {
                center,
                major: radial,
                minor: length(radial),
            }
        };
        result.validate()?;
        Ok(result)
    }

    pub fn tangent_arc(start: Point, tangent: Point, end: Point) -> Self {
        let delta = sub(end, start);
        let perpendicular = mul(normal(tangent), 1.0 / length(tangent));
        let distance = length(delta);
        let height = dot(delta, perpendicular);
        let offset = if height.abs() < MIN_LENGTH.max(1e-12 * distance) {
            f64::NAN
        } else {
            0.5 * distance * (distance / height)
        };
        let radial = mul(perpendicular, -offset);
        let center = sub(start, radial);
        let angle = angle_from(radial, sub(end, center));
        Self::Arc {
            center,
            radial,
            sweep: if offset > 0.0 { angle } else { angle - TAU },
        }
    }

    pub fn elliptical_arc(
        center: Point,
        major_end: Point,
        side: Point,
        a: Point,
        b: Point,
    ) -> Self {
        let major = sub(major_end, center);
        let minor = signed_height(center, major_end, side).abs();
        let radius = length(major);
        let u = mul(major, 1.0 / radius);
        let parameter = |p| {
            let local = sub(p, center);
            if length(local) < MIN_LENGTH {
                return f64::NAN;
            }
            (dot(local, normal(u)) / minor).atan2(dot(local, u) / radius)
        };
        let start = parameter(a);
        Self::EllipticalArc {
            center,
            major,
            minor,
            start,
            sweep: (parameter(b) - start).rem_euclid(TAU),
        }
    }

    pub fn evaluate_bezier<const N: usize>(mut controls: [Point; N], t: f64) -> Point {
        for level in (1..N).rev() {
            for i in 0..level {
                controls[i] = add(mul(controls[i], 1.0 - t), mul(controls[i + 1], t));
            }
        }
        controls[0]
    }

    pub fn elevate_cubic(controls: [Point; 4]) -> Self {
        Self::ElevatedBezier {
            controls: [
                controls[0],
                add(mul(controls[0], 0.25), mul(controls[1], 0.75)),
                add(mul(controls[1], 0.5), mul(controls[2], 0.5)),
                add(mul(controls[2], 0.75), mul(controls[3], 0.25)),
                controls[3],
            ],
        }
    }

    /// Piecewise cubic Hermite interpolation with centered interior
    /// tangents and one-sided endpoint tangents.
    fn spline_spans(fit: [Point; 4], ends: Option<[Point; 2]>) -> [[Point; 4]; 3] {
        let [first, last] = ends.unwrap_or([sub(fit[1], fit[0]), sub(fit[3], fit[2])]);
        let tangent = [
            first,
            mul(sub(fit[2], fit[0]), 0.5),
            mul(sub(fit[3], fit[1]), 0.5),
            last,
        ];
        std::array::from_fn(|i| {
            [
                fit[i],
                add(fit[i], mul(tangent[i], 1.0 / 3.0)),
                sub(fit[i + 1], mul(tangent[i + 1], 1.0 / 3.0)),
                fit[i + 1],
            ]
        })
    }

    fn spline_pieces(self) -> Vec<[Point; 4]> {
        let Self::Spline {
            fit,
            tangents,
            refined,
        } = self
        else {
            return Vec::new();
        };
        let [first, middle, last] = Self::spline_spans(fit, tangents);
        if !refined {
            return vec![first, middle, last];
        }
        let [a, b, c, d] = middle;
        let mid = |a, b| add(mul(a, 0.5), mul(b, 0.5));
        let (ab, bc, cd) = (mid(a, b), mid(b, c), mid(c, d));
        let (abc, bcd) = (mid(ab, bc), mid(bc, cd));
        let split = mid(abc, bcd);
        vec![first, [a, ab, abc, split], [split, bcd, cd, d], last]
    }

    pub fn refine(self) -> Option<Self> {
        match self {
            Self::Bezier { controls } => Some(Self::elevate_cubic(controls)),
            Self::Spline {
                fit,
                tangents,
                refined: false,
            } => Some(Self::Spline {
                fit,
                tangents,
                refined: true,
            }),
            _ => None,
        }
    }

    pub fn refinement_segments(self) -> Vec<Segment> {
        if let Some(controls) = self.control_polygon() {
            controls.windows(2).map(|p| (p[0], p[1])).collect()
        } else {
            self.spline_pieces()
                .into_iter()
                .flat_map(|p| [(p[0], p[1]), (p[1], p[2]), (p[2], p[3])])
                .collect()
        }
    }

    pub fn control_polygon(self) -> Option<[Point; 5]> {
        match self {
            Self::ElevatedBezier { controls } => Some(controls),
            _ => None,
        }
    }

    pub fn validate(self) -> Result<(), ModelError> {
        let positive = |value: f64| value.is_finite() && value >= MIN_LENGTH;
        let finite = |p: Point| p.into_iter().all(f64::is_finite);
        let valid = match self {
            Self::Arc {
                center,
                radial,
                sweep,
            } => {
                let radius = length(radial);
                positive(radius)
                    && sweep.is_finite()
                    && sweep.abs() < TAU
                    && positive(radius * sweep.abs())
                    && finite(add(center, [radius; 2]))
                    && finite(sub(center, [radius; 2]))
            }
            Self::EllipticalArc {
                center,
                major,
                minor,
                start,
                sweep,
            } => {
                Self::Ellipse {
                    center,
                    major,
                    minor,
                }
                .validate()
                .is_ok()
                    && start.is_finite()
                    && sweep.is_finite()
                    && sweep.abs() < TAU
                    && positive(minor * sweep.abs())
            }
            Self::Conic {
                start,
                control,
                end,
                weight,
            } => {
                [start, control, end].into_iter().all(finite)
                    && weight.is_finite()
                    && weight > 0.0
                    && positive(length(sub(end, start)))
                    && signed_height(start, end, control).abs() >= MIN_LENGTH
            }
            Self::ElevatedBezier { controls } => {
                controls.into_iter().all(finite)
                    && controls
                        .windows(2)
                        .any(|pair| positive(length(sub(pair[1], pair[0]))))
            }
            Self::Spline { fit, tangents, .. } => {
                fit.windows(2)
                    .all(|pair| positive(length(sub(pair[1], pair[0]))))
                    && tangents.is_none_or(|t| t.into_iter().all(|v| positive(length(v))))
                    && Self::spline_spans(fit, tangents)
                        .into_iter()
                        .flatten()
                        .all(finite)
            }
            Self::Bezier { controls } => {
                controls.into_iter().all(finite)
                    && controls
                        .windows(2)
                        .any(|pair| positive(length(sub(pair[1], pair[0]))))
            }
            Self::Text(text) => return text.validate(),
            Self::Point(p) => finite(p),
            Self::Polygon {
                center,
                vertex,
                sides,
            } => {
                let r = length(sub(vertex, center));
                (3..=256).contains(&sides)
                    && positive(r)
                    && positive(2.0 * r * (PI / f64::from(sides)).sin())
                    && finite(add(center, [r; 2]))
                    && finite(sub(center, [r; 2]))
            }
            Self::OrientedRectangle { start, end, width } => {
                let base = sub(end, start);
                let span = length(base);
                let side = mul(normal(base), width / span);
                positive(span)
                    && positive(width.abs())
                    && finite(add(start, side))
                    && finite(add(end, side))
            }
            Self::Slot { start, end, radius } => {
                positive(length(sub(end, start)))
                    && positive(radius)
                    && [start, end]
                        .into_iter()
                        .all(|p| finite(add(p, [radius; 2])) && finite(sub(p, [radius; 2])))
            }
            Self::Ellipse {
                center,
                major,
                minor,
            } => {
                let radius = length(major);
                let bound = radius.max(minor);
                positive(radius)
                    && positive(minor)
                    && minor <= radius * (1.0 + 1e-12)
                    && finite(add(center, [bound; 2]))
                    && finite(sub(center, [bound; 2]))
            }
        };
        if valid {
            Ok(())
        } else {
            Err(ModelError::InvalidSketch("invalid draw geometry"))
        }
    }

    /// Display chords only. The stored geometry retains its defining parameters.
    pub fn display_segments(self) -> Vec<Segment> {
        match self {
            Self::Text(text) => text.display_segments(),
            Self::Point(_) => Vec::new(),
            Self::Polygon {
                center,
                vertex,
                sides,
            } => {
                let radial = sub(vertex, center);
                ellipse_segments(center, radial, normal(radial), TAU, usize::from(sides))
            }
            Self::OrientedRectangle { start, end, width } => {
                let base = sub(end, start);
                let side = mul(normal(base), width / length(base));
                let a = add(start, side);
                let b = add(end, side);
                vec![(start, end), (end, b), (b, a), (a, start)]
            }
            Self::Slot { start, end, radius } => {
                let base = sub(end, start);
                let side = mul(normal(base), radius / length(base));
                let mut result = vec![
                    (add(start, side), add(end, side)),
                    (sub(end, side), sub(start, side)),
                ];
                result.extend(ellipse_segments(end, side, normal(side), -PI, 32));
                result.extend(ellipse_segments(
                    start,
                    mul(side, -1.0),
                    mul(normal(side), -1.0),
                    -PI,
                    32,
                ));
                result
            }
            Self::Ellipse {
                center,
                major,
                minor,
            } => ellipse_segments(
                center,
                major,
                mul(normal(major), minor / length(major)),
                TAU,
                96,
            ),

            Self::Arc {
                center,
                radial,
                sweep,
            } => ellipse_segments(center, radial, normal(radial), sweep, 96),
            Self::EllipticalArc {
                center,
                major,
                minor,
                start,
                sweep,
            } => {
                let y = mul(normal(major), minor / length(major));
                let (sin, cos) = start.sin_cos();
                ellipse_segments(
                    center,
                    add(mul(major, cos), mul(y, sin)),
                    add(mul(major, -sin), mul(y, cos)),
                    sweep,
                    96,
                )
            }
            Self::Conic {
                start,
                control,
                end,
                weight,
            } => {
                let eval = |t: f64| {
                    let u = 1.0 - t;
                    let a = u * u;
                    let b = 2.0 * t * u * weight;
                    let c = t * t;
                    let denominator = a + b + c;
                    add(
                        add(mul(start, a / denominator), mul(control, b / denominator)),
                        mul(end, c / denominator),
                    )
                };
                (0..64)
                    .map(|i| (eval(f64::from(i) / 64.0), eval(f64::from(i + 1) / 64.0)))
                    .collect()
            }
            Self::ElevatedBezier { controls } => bezier_segments(controls).collect(),
            Self::Spline { .. } => self
                .spline_pieces()
                .into_iter()
                .flat_map(bezier_segments)
                .collect(),
            Self::Bezier { controls } => bezier_segments(controls).collect(),
        }
    }
}

fn bezier_segments<const N: usize>(controls: [Point; N]) -> impl Iterator<Item = Segment> {
    (0..64).map(move |i| {
        (
            DrawGeometry::evaluate_bezier(controls, f64::from(i) / 64.0),
            DrawGeometry::evaluate_bezier(controls, f64::from(i + 1) / 64.0),
        )
    })
}

fn control_frame<const N: usize>(controls: [Point; N]) -> Segment {
    let first = controls[0];
    (
        first,
        controls
            .into_iter()
            .find(|&p| length(sub(p, first)) >= MIN_LENGTH)
            .unwrap_or(first),
    )
}

fn angle_from(radial: Point, end: Point) -> f64 {
    if length(end) < MIN_LENGTH {
        return f64::NAN;
    }
    let a = radial[1].atan2(radial[0]);
    let b = end[1].atan2(end[0]);
    (b - a).rem_euclid(TAU)
}

fn ellipse_segments(center: Point, x: Point, y: Point, sweep: f64, count: usize) -> Vec<Segment> {
    let mut previous = add(center, x);
    let start = previous;
    (1..=count)
        .map(|i| {
            let angle = sweep * i as f64 / count as f64;
            let current = if i == count && sweep.abs() == TAU {
                start
            } else {
                add(center, add(mul(x, angle.cos()), mul(y, angle.sin())))
            };
            let segment = (previous, current);
            previous = current;
            segment
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circumcircle_rejects_collinearity_and_contains_input_points() {
        assert!(DrawGeometry::circumcircle([0., 0.], [2., 0.], [4., 0.]).is_err());
        let circle = DrawGeometry::circumcircle([0., 0.], [2., 0.], [0., 2.]).unwrap();
        let DrawGeometry::Ellipse { center, minor, .. } = circle else {
            unreachable!();
        };
        assert_eq!(center, [1., 1.]);
        assert!((minor - 2_f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn slot_rejects_zero_width_and_caps_extend_outward() {
        assert!(
            DrawGeometry::slot([0., 0.], [4., 0.], [2., 0.])
                .validate()
                .is_err()
        );
        let slot = DrawGeometry::slot([0., 0.], [4., 0.], [2., 1.]);
        slot.validate().unwrap();
        let chords = slot.display_segments();
        assert!(length(sub(chords[17].1, [5., 0.])) < 1e-12);
        assert!(length(sub(chords[49].1, [-1., 0.])) < 1e-12);
    }
}

/// The source interval remains explicit even when the accepted operation
/// replaces a line's endpoints under its existing key.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineOperation {
    Transform {
        origin: Point,
        destination: Point,
        radians: f64,
        scale: f64,
    },
    Mirror {
        axis: [Point; 2],
    },
    Split {
        parameter: f64,
    },
    Trim {
        removed: [f64; 2],
    },
    Extend {
        parameter: f64,
    },
    Offset {
        distance: f64,
    },
}

fn cross(a: Point, b: Point) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

impl LineOperation {
    pub fn transform(
        source: Segment,
        origin: Point,
        destination: Point,
        handle: Point,
    ) -> Result<(Self, Segment), ModelError> {
        let baseline = sub(source.1, source.0);
        let axis = sub(handle, destination);
        let scale = length(axis) / length(baseline);
        if !scale.is_finite() || length(axis) < MIN_LENGTH {
            return Err(ModelError::InvalidSketch("transform needs a nonzero scale"));
        }
        let radians = axis[1].atan2(axis[0]) - baseline[1].atan2(baseline[0]);
        let (sin, cos) = radians.sin_cos();
        let apply = |p| {
            let [x, y] = mul(sub(p, origin), scale);
            add(destination, [x * cos - y * sin, x * sin + y * cos])
        };
        Ok((
            Self::Transform {
                origin,
                destination,
                radians,
                scale,
            },
            (apply(source.0), apply(source.1)),
        ))
    }

    pub fn mirror(source: Segment, axis: [Point; 2]) -> Result<Segment, ModelError> {
        let span = sub(axis[1], axis[0]);
        let width = length(span);
        if !width.is_finite() || width < MIN_LENGTH {
            return Err(ModelError::InvalidSketch(
                "two distinct mirror axis points required",
            ));
        }
        let direction = mul(span, 1.0 / width);
        let reflect = |p| {
            let local = sub(p, axis[0]);
            sub(p, mul(normal(direction), 2.0 * cross(direction, local)))
        };
        Ok((reflect(source.0), reflect(source.1)))
    }

    /// Returns the operation and its one or two finite result segments.
    /// Split and Trim preserve source direction and order.
    pub fn build(
        mode: LineEditMode,
        source: Segment,
        pointer: Point,
        boundaries: impl IntoIterator<Item = Segment>,
    ) -> Result<(Self, Vec<Segment>), ModelError> {
        let (a, b) = source;
        let delta = sub(b, a);
        let span = length(delta);
        let direction = mul(delta, 1.0 / span);
        let cursor = sub(pointer, a);
        let parameter = dot(cursor, direction) / span;
        let eps = (MIN_LENGTH / span).max(1e-12);
        let at = |t| add(a, mul(delta, t));
        let invalid = |message| ModelError::InvalidSketch(message);
        let interior = |t: f64| t > eps && t < 1.0 - eps;
        let (operation, result) = match mode {
            LineEditMode::Split if interior(parameter) => {
                let p = at(parameter);
                (Self::Split { parameter }, vec![(a, p), (p, b)])
            }
            LineEditMode::Split => return Err(invalid("split inside the segment")),
            LineEditMode::Offset => {
                let distance = cross(direction, cursor);
                if distance.abs() < MIN_LENGTH {
                    return Err(invalid("offset distance is zero"));
                }
                let shift = mul(normal(direction), distance);
                (
                    Self::Offset { distance },
                    vec![(add(a, shift), add(b, shift))],
                )
            }
            LineEditMode::Trim | LineEditMode::Extend => {
                let extend_end = parameter >= 0.5;
                let mut cuts = vec![0.0, 1.0];
                for (c, d) in boundaries {
                    let edge = sub(d, c);
                    let edge_span = length(edge);
                    let edge_direction = mul(edge, 1.0 / edge_span);
                    let denominator = cross(direction, edge_direction);
                    let origin = sub(c, a);
                    if denominator.abs() <= 1e-12 {
                        if cross(direction, origin).abs() < MIN_LENGTH {
                            let lo = dot(origin, direction) / span;
                            let hi = dot(sub(d, a), direction) / span;
                            if mode == LineEditMode::Extend
                                || (lo.max(hi) > eps && lo.min(hi) < 1.0 - eps)
                            {
                                return Err(invalid("coincident cutting lines"));
                            }
                        }
                        continue;
                    }
                    let t = cross(origin, edge_direction) / denominator / span;
                    let u = cross(origin, direction) / denominator / edge_span;
                    if t.is_finite() && (0.0..=1.0).contains(&u) {
                        cuts.push(t);
                    }
                }
                cuts.sort_by(f64::total_cmp);
                if mode == LineEditMode::Extend {
                    let t = if extend_end {
                        cuts.into_iter().find(|t| *t > 1.0 + eps)
                    } else {
                        cuts.into_iter().rev().find(|t| *t < -eps)
                    }
                    .ok_or_else(|| invalid("no boundary beyond the selected end"))?;
                    let segment = if extend_end { (a, at(t)) } else { (at(t), b) };
                    (Self::Extend { parameter: t }, vec![segment])
                } else {
                    cuts.retain(|t| *t == 0.0 || *t == 1.0 || interior(*t));
                    cuts.dedup_by(|a, b| (*a - *b).abs() <= eps);
                    if cuts.len() <= 2 || cuts.iter().any(|t| (*t - parameter).abs() <= eps) {
                        return Err(invalid("select an interval between distinct cuts"));
                    }
                    let cut = cuts
                        .windows(2)
                        .find(|w| w[0] < parameter && parameter < w[1])
                        .ok_or_else(|| invalid("select the interval to trim"))?;
                    let mut result = Vec::with_capacity(2);
                    if cut[0] > eps {
                        result.push((a, at(cut[0])));
                    }
                    if cut[1] < 1.0 - eps {
                        result.push((at(cut[1]), b));
                    }
                    (
                        Self::Trim {
                            removed: [cut[0], cut[1]],
                        },
                        result,
                    )
                }
            }
        };
        Ok((operation, result))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEditMode {
    Split,
    Trim,
    Extend,
    Offset,
}

/// Corner operations on two finite segments with a unique shared endpoint.
pub fn line_corner(
    first: Segment,
    second: Segment,
    values: [f64; 2],
    fillet: bool,
) -> Result<([Segment; 2], super::SketchPrimitive), ModelError> {
    use super::SketchPrimitive;
    let invalid = || ModelError::InvalidSketch("invalid line corner or setback");
    let candidates = [
        (first.0, first.1, second.0, second.1, true, true),
        (first.0, first.1, second.1, second.0, true, false),
        (first.1, first.0, second.0, second.1, false, true),
        (first.1, first.0, second.1, second.0, false, false),
    ];
    let mut shared = candidates
        .into_iter()
        .filter(|(a, _, b, _, _, _)| length(sub(*a, *b)) < MIN_LENGTH);
    let (corner, a, _, b, first_start, second_start) = shared.next().ok_or_else(invalid)?;
    if shared.next().is_some() || values.iter().any(|v| !v.is_finite() || *v < MIN_LENGTH) {
        return Err(invalid());
    }
    let len_a = length(sub(a, corner));
    let len_b = length(sub(b, corner));
    let u = mul(sub(a, corner), 1.0 / len_a);
    let v = mul(sub(b, corner), 1.0 / len_b);
    let theta = cross(u, v).abs().atan2(dot(u, v));
    if theta <= 1e-12 || PI - theta <= 1e-12 {
        return Err(invalid());
    }
    let [da, db] = if fillet {
        [values[0] / (theta * 0.5).tan(); 2]
    } else {
        values
    };
    if da < MIN_LENGTH || db < MIN_LENGTH || len_a - da < MIN_LENGTH || len_b - db < MIN_LENGTH {
        return Err(invalid());
    }
    let p = add(corner, mul(u, da));
    let q = add(corner, mul(v, db));
    let result = if fillet {
        SketchPrimitive::Draw(DrawGeometry::tangent_arc(p, mul(u, -1.0), q))
    } else {
        SketchPrimitive::Line { start: p, end: q }
    };
    result.validate()?;
    Ok((
        [
            if first_start { (p, a) } else { (a, p) },
            if second_start { (q, b) } else { (b, q) },
        ],
        result,
    ))
}

/// Isolate roots on monotone intervals of the exact Bernstein polynomial.
/// A contact at a stationary point is admitted once.
pub(super) fn cubic_plane_roots(z: [f64; 4]) -> Result<Vec<f64>, ModelError> {
    let span = z.iter().fold(0.0_f64, |a, b| a.max(b.abs()));
    if !span.is_finite() || span < MIN_LENGTH {
        return Err(ModelError::InvalidSketch(
            "cubic edge lies in the sketch plane",
        ));
    }
    let [p, q, r, s] = z.map(|x| x / span);
    let (a, b, c) = (s - 3.0 * r + 3.0 * q - p, 2.0 * (r - 2.0 * q + p), q - p);
    let mut cuts = vec![0.0, 1.0];
    let mut insert = |t| {
        if t > 0.0 && t < 1.0 {
            cuts.push(t);
        }
    };
    if a.abs() <= 1e-14 {
        if b.abs() > 1e-14 {
            insert(-c / b);
        }
    } else {
        let d = b * b - 4.0 * a * c;
        if d >= 0.0 {
            let q = -0.5 * (b + d.sqrt().copysign(b));
            if q != 0.0 {
                insert(q / a);
                insert(c / q);
            } else {
                insert(-b / (2.0 * a));
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    let eval = |t| DrawGeometry::evaluate_bezier([p, q, r, s].map(|x| [x, 0.0]), t)[0];
    let epsilon = (MIN_LENGTH / span).max(1e-14);
    let mut roots = Vec::new();
    for &t in &cuts {
        if eval(t).abs() <= epsilon {
            roots.push(t);
        }
    }
    for interval in cuts.windows(2) {
        let (mut lo, mut hi) = (interval[0], interval[1]);
        let low = eval(lo);
        let high = eval(hi);
        if low.abs() <= epsilon || high.abs() <= epsilon || low.signum() == high.signum() {
            continue;
        }
        for _ in 0..50 {
            let mid = (lo + hi) * 0.5;
            if eval(mid).signum() == low.signum() {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        roots.push((lo + hi) * 0.5);
    }
    roots.sort_by(f64::total_cmp);
    roots.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    Ok(roots)
}

/// Offset one nonbranching straight chain. Miter joins are exact line
/// intersections; self-intersecting results are rejected as a whole.
pub fn offset_line_chain(
    source: Segment,
    pointer: Point,
    boundaries: impl IntoIterator<Item = Segment>,
) -> Result<Vec<Segment>, ModelError> {
    use std::collections::VecDeque;
    let invalid = |message| ModelError::InvalidSketch(message);
    let mut available: Vec<_> = boundaries.into_iter().collect();
    let mut path = VecDeque::from([source.0, source.1]);
    let coincides = |a, b| length(sub(a, b)) < MIN_LENGTH;
    let mut extend = |path: &mut VecDeque<Point>, front: bool| -> Result<(), ModelError> {
        loop {
            let (Some(&first), Some(&last)) = (path.front(), path.back()) else {
                return Ok(());
            };
            let tip = if front { first } else { last };
            let mut matches = available.iter().enumerate().filter_map(|(i, &(a, b))| {
                if coincides(tip, a) {
                    Some((i, b))
                } else if coincides(tip, b) {
                    Some((i, a))
                } else {
                    None
                }
            });
            let candidate = matches.next();
            if matches.next().is_some() || (candidate.is_some() && coincides(first, last)) {
                return Err(invalid("offset chain branches"));
            }
            let Some((index, next)) = candidate else {
                break;
            };
            available.swap_remove(index);
            if front {
                path.push_front(next);
            } else {
                path.push_back(next);
            }
        }
        Ok(())
    };
    extend(&mut path, false)?;
    extend(&mut path, true)?;
    let mut points: Vec<_> = path.into_iter().collect();
    let closed = coincides(points[0], points[points.len() - 1]);
    if closed {
        points.pop();
    }
    if closed && points.len() < 3 {
        return Err(invalid("closed offset chain needs at least three sides"));
    }
    if points
        .iter()
        .enumerate()
        .any(|(i, p)| points[i + 1..].iter().any(|&q| coincides(*p, q)))
    {
        return Err(invalid("offset chain revisits a vertex"));
    }
    let direction = |a, b| {
        let v = sub(b, a);
        mul(v, 1.0 / length(v))
    };
    let distance = cross(direction(source.0, source.1), sub(pointer, source.0));
    if !distance.is_finite() || distance.abs() < MIN_LENGTH {
        return Err(invalid("offset distance is zero"));
    }
    let n = points.len();
    let mut offset = Vec::with_capacity(n);
    for (i, &p) in points.iter().enumerate() {
        let shift = if !closed && i == 0 {
            mul(normal(direction(p, points[1])), distance)
        } else if !closed && i + 1 == n {
            mul(normal(direction(points[i - 1], p)), distance)
        } else {
            let u = direction(points[(i + n - 1) % n], p);
            let v = direction(p, points[(i + 1) % n]);
            let denominator = 1.0 + dot(u, v);
            if denominator <= 1e-10 {
                return Err(invalid("offset chain reverses direction"));
            }
            mul(add(normal(u), normal(v)), distance / denominator)
        };
        offset.push(add(p, shift));
    }
    let count = if closed { n } else { n - 1 };
    let result: Vec<_> = (0..count)
        .map(|i| (offset[i], offset[(i + 1) % n]))
        .collect();
    for (i, &(a, b)) in result.iter().enumerate() {
        if dot(sub(b, a), direction(points[i], points[(i + 1) % n])) < MIN_LENGTH {
            return Err(invalid("offset chain collapsed or reversed a side"));
        }
        for (j, &(c, d)) in result.iter().enumerate().skip(i + 2) {
            if closed && i == 0 && j + 1 == count {
                continue;
            }
            let (u, v) = (sub(b, a), sub(d, c));
            let delta = sub(c, a);
            let denominator = cross(u, v);
            if denominator.abs() <= 1e-12 * length(u) * length(v) {
                if cross(direction(a, b), delta).abs() < MIN_LENGTH {
                    let along = |p| dot(direction(a, b), sub(p, a));
                    let (x, y) = (along(c), along(d));
                    if x.max(y) >= -MIN_LENGTH && x.min(y) <= length(u) + MIN_LENGTH {
                        return Err(invalid("offset chain overlaps itself"));
                    }
                }
            } else {
                let t = cross(delta, v) / denominator;
                let s = cross(delta, u) / denominator;
                if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&s) {
                    return Err(invalid("offset chain crosses itself"));
                }
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod curved_sketch_tests {
    use super::*;

    #[test]
    fn sketch_arc_trim_and_extend_use_finite_crossings() {
        let arc = DrawGeometry::center_arc([0., 0.], [4., 0.], [-4., 0.]);
        let trimmed = arc
            .cut_arc(LineEditMode::Trim, [3., 1.], [([0., -5.], [0., 5.])])
            .unwrap();
        assert!(
            matches!(trimmed.as_slice(), [DrawGeometry::Arc { sweep, .. }] if (*sweep - PI/2.).abs() < 1e-12)
        );
        let extended = arc
            .cut_arc(LineEditMode::Extend, [-4., 0.], [([0., -5.], [0., -2.])])
            .unwrap();
        assert!(
            matches!(extended.as_slice(), [DrawGeometry::Arc { sweep, .. }] if (*sweep - 1.5*PI).abs() < 1e-12)
        );
        assert!(
            arc.cut_arc(LineEditMode::Trim, [3., 1.], [([4., -5.], [4., 5.])])
                .is_err()
        );
    }

    #[test]
    fn sketch_chain_offset_joins_and_rejects_branches() {
        let a = ([0., 0.], [4., 0.]);
        let b = ([4., 4.], [4., 0.]);
        assert_eq!(
            offset_line_chain(a, [1., 1.], [b]).unwrap(),
            vec![([0., 1.], [3., 1.]), ([3., 1.], [3., 4.])]
        );
        assert!(offset_line_chain(a, [1., 1.], [b, ([4., 0.], [8., 0.])]).is_err());
    }

    #[test]
    fn sketch_spline_refinement_retains_the_handle_defined_curve() {
        let fit = [[0., 0.], [2., 2.], [4., 1.], [7., 0.]];
        let spline = DrawGeometry::Spline {
            fit,
            tangents: Some([[0., 2.], [1., -1.]]),
            refined: false,
        };
        assert_eq!(
            spline.endpoint_tangents(),
            Some([(fit[0], [-0., -2.]), (fit[3], [1., -1.])])
        );
        let before = spline.spline_pieces();
        let after = spline.refine().unwrap().spline_pieces();
        assert_eq!(after.len(), 4);
        for i in 0..=20 {
            let t = f64::from(i) / 20.0;
            let [a, b] = DrawGeometry::evaluate_bezier(before[1], t);
            let [x, y] = if t <= 0.5 {
                DrawGeometry::evaluate_bezier(after[1], 2. * t)
            } else {
                DrawGeometry::evaluate_bezier(after[2], 2. * t - 1.)
            };
            assert!((x - a).hypot(y - b) < 1e-12);
        }
    }

    #[test]
    fn sketch_cubic_brep_crossings_include_tangent_contact_once() {
        let roots = cubic_plane_roots([-0.08, 0.14, -0.14, 0.08]).unwrap();
        assert_eq!(roots.len(), 3);
        for (root, expected) in roots.into_iter().zip([0.2, 0.5, 0.8]) {
            assert!((root - expected).abs() < 1e-12);
        }
        let contact = cubic_plane_roots([1., -1. / 3., -1. / 3., 1.]).unwrap();
        assert_eq!(contact, vec![0.5]);
        assert!(cubic_plane_roots([0.; 4]).is_err());
    }

    #[test]
    fn sketch_chamfer_retains_independent_setbacks() {
        let (_, join) =
            line_corner(([0., 0.], [8., 0.]), ([0., 0.], [0., 8.]), [1., 2.], false).unwrap();
        assert_eq!(join.line(), Some(([1., 0.], [0., 2.])));
    }
}
