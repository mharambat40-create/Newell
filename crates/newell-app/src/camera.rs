// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! GPU-independent, f64 camera mathematics owned by the Controller.

use newell_render::{CameraSnapshot, SceneViewport};

/// World-space vector, in model units.
pub type Vec3 = [f64; 3];
/// Column-major matrix, with column vectors and WGPU zero-to-one clip depth.
pub type Matrix = [[f64; 4]; 4];
/// Absolute lower bound on navigation depth in model units.
pub const MIN_DISTANCE: f64 = 0.001;
/// Orbit stays this far from a world-up singularity.
pub const POLE_MARGIN: f64 = 0.5_f64.to_radians();
/// Angular response per logical pointer pixel.
pub const ORBIT_SENSITIVITY: f64 = 0.005;
const WORLD_UP: Vec3 = [0.0, 0.0, 1.0];

pub(crate) fn add(a: Vec3, b: Vec3) -> Vec3 {
    std::array::from_fn(|i| a[i] + b[i])
}
pub(crate) fn sub(a: Vec3, b: Vec3) -> Vec3 {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(crate) fn scale(a: Vec3, k: f64) -> Vec3 {
    a.map(|v| v * k)
}
pub(crate) fn dot(a: Vec3, b: Vec3) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
pub(crate) fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(crate) fn length(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}
pub(crate) fn unit(a: Vec3) -> Vec3 {
    let n = length(a);
    if n > 1e-14 && n.is_finite() {
        scale(a, 1.0 / n)
    } else {
        [0.0; 3]
    }
}
pub(crate) fn finite(a: Vec3) -> bool {
    a.iter().all(|v| v.is_finite())
}

/// Unit quaternion in scalar-first convention, rotating camera-local vectors to world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion([f64; 4]);

impl Quaternion {
    fn normalized(q: [f64; 4]) -> Self {
        let n = q.iter().map(|v| v * v).sum::<f64>().sqrt();
        if n > 1e-14 && n.is_finite() {
            Self(q.map(|v| v / n))
        } else {
            Self([1.0, 0.0, 0.0, 0.0])
        }
    }
    /// Rotation around a unit world-space axis.
    #[must_use]
    pub fn axis_angle(axis: Vec3, angle: f64) -> Self {
        let axis = unit(axis);
        let (s, c) = (angle * 0.5).sin_cos();
        Self::normalized([c, axis[0] * s, axis[1] * s, axis[2] * s])
    }
    /// Quaternion norm, useful for invariants without exposing redundant orientation state.
    #[must_use]
    pub fn norm(self) -> f64 {
        self.0.iter().map(|v| v * v).sum::<f64>().sqrt()
    }
    /// Inverse of a unit quaternion.
    #[must_use]
    pub fn inverse(self) -> Self {
        Self([self.0[0], -self.0[1], -self.0[2], -self.0[3]])
    }
    /// Composition; the right-hand rotation is applied first.
    #[must_use]
    pub fn compose(self, b: Self) -> Self {
        let [w, x, y, z] = self.0;
        let [v, a, c, d] = b.0;
        Self::normalized([
            w * v - x * a - y * c - z * d,
            w * a + x * v + y * d - z * c,
            w * c - x * d + y * v + z * a,
            w * d + x * c - y * a + z * v,
        ])
    }
    /// Rotates a world or camera vector.
    #[must_use]
    pub fn rotate(self, v: Vec3) -> Vec3 {
        let u = [self.0[1], self.0[2], self.0[3]];
        add(
            add(v, scale(cross(u, v), 2.0 * self.0[0])),
            scale(cross(u, cross(u, v)), 2.0),
        )
    }
    fn from_basis(r: Vec3, u: Vec3, f: Vec3) -> Self {
        let b = scale(f, -1.0);
        let m = [[r[0], u[0], b[0]], [r[1], u[1], b[1]], [r[2], u[2], b[2]]];
        let trace = m[0][0] + m[1][1] + m[2][2];
        let q = if trace > 0.0 {
            let s = (trace + 1.0).sqrt() * 2.0;
            [
                0.25 * s,
                (m[2][1] - m[1][2]) / s,
                (m[0][2] - m[2][0]) / s,
                (m[1][0] - m[0][1]) / s,
            ]
        } else {
            let i = if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
                0
            } else if m[1][1] > m[2][2] {
                1
            } else {
                2
            };
            let j = (i + 1) % 3;
            let k = (i + 2) % 3;
            let s = (1.0 + m[i][i] - m[j][j] - m[k][k]).max(0.0).sqrt() * 2.0;
            let mut q = [0.0; 4];
            q[0] = (m[k][j] - m[j][k]) / s;
            q[i + 1] = 0.25 * s;
            q[j + 1] = (m[j][i] + m[i][j]) / s;
            q[k + 1] = (m[k][i] + m[i][k]) / s;
            q
        };
        Self::normalized(q)
    }
    fn same_sign(self, previous: Self) -> Self {
        if self
            .0
            .iter()
            .zip(previous.0)
            .map(|(a, b)| a * b)
            .sum::<f64>()
            < 0.0
        {
            Self(self.0.map(|v| -v))
        } else {
            self
        }
    }
}

/// A world-space picking ray. Orthographic rays have distinct origins and parallel directions.
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

/// Controller camera: orientation is authoritative; no independently mutable up or target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub eye: Vec3,
    pub orientation: Quaternion,
    pub focal_distance: f64,
    pub last_heading: Vec3,
    pub perspective: bool,
    pub vertical_fov: f64,
    pub orthographic_height: f64,
    pub lens_shift: [f64; 2],
    pub viewport: SceneViewport,
}

impl Default for Camera {
    fn default() -> Self {
        Self::look_at([3.4, 3.0, 3.2], [0.5; 3], SceneViewport::default())
    }
}

impl Camera {
    /// Constructs a finite, level camera. Exact vertical views use a deterministic heading.
    #[must_use]
    pub fn look_at(eye: Vec3, target: Vec3, viewport: SceneViewport) -> Self {
        let forward = unit(sub(target, eye));
        let heading = unit([forward[0], forward[1], 0.0]);
        let heading = if length(heading) > 0.0 {
            heading
        } else {
            [0.0, 1.0, 0.0]
        };
        let right = unit(cross(heading, WORLD_UP));
        let up = cross(right, forward);
        let focal_distance = length(sub(target, eye)).max(MIN_DISTANCE);
        Self {
            eye,
            orientation: Quaternion::from_basis(right, up, forward),
            focal_distance,
            last_heading: heading,
            perspective: true,
            vertical_fov: 45.0_f64.to_radians(),
            orthographic_height: 2.0 * focal_distance * (22.5_f64.to_radians()).tan(),
            lens_shift: [0.0; 2],
            viewport,
        }
    }
    /// Orthonormal screen right, screen up and view forward.
    #[must_use]
    pub fn basis(self) -> (Vec3, Vec3, Vec3) {
        (
            self.orientation.rotate([1.0, 0.0, 0.0]),
            self.orientation.rotate([0.0, 1.0, 0.0]),
            self.orientation.rotate([0.0, 0.0, -1.0]),
        )
    }
    /// Derived centre of the camera-facing navigation plane.
    #[must_use]
    pub fn target(self) -> Vec3 {
        add(self.eye, scale(self.basis().2, self.focal_distance))
    }
    /// View matrix, world to right-handed view coordinates.
    #[must_use]
    pub fn view(self) -> Matrix {
        let (r, u, f) = self.basis();
        [
            [r[0], u[0], -f[0], 0.0],
            [r[1], u[1], -f[1], 0.0],
            [r[2], u[2], -f[2], 0.0],
            [-dot(r, self.eye), -dot(u, self.eye), dot(f, self.eye), 1.0],
        ]
    }
    /// WGPU clip projection with a true orthographic lens translation.
    #[must_use]
    pub fn projection(self, near: f64, far: f64) -> Matrix {
        let aspect = self.viewport.width.max(1) as f64 / self.viewport.height.max(1) as f64;
        if self.perspective {
            let k = 1.0 / (self.vertical_fov * 0.5).tan();
            [
                [k / aspect, 0.0, 0.0, 0.0],
                [0.0, k, 0.0, 0.0],
                [
                    -self.lens_shift[0],
                    -self.lens_shift[1],
                    far / (near - far),
                    -1.0,
                ],
                [0.0, 0.0, near * far / (near - far), 0.0],
            ]
        } else {
            let hh = self.orthographic_height * 0.5;
            [
                [1.0 / (hh * aspect), 0.0, 0.0, 0.0],
                [0.0, 1.0 / hh, 0.0, 0.0],
                [0.0, 0.0, 1.0 / (near - far), 0.0],
                [
                    self.lens_shift[0],
                    self.lens_shift[1],
                    near / (near - far),
                    1.0,
                ],
            ]
        }
    }
    /// Clip planes encompass model depth and stay valid even when the object is behind the eye.
    #[must_use]
    pub fn clip(self, sphere: (Vec3, f64)) -> (f64, f64) {
        let depth = dot(sub(sphere.0, self.eye), self.basis().2);
        let near = (depth - sphere.1 * 1.1).max(1e-7);
        (near, (depth + sphere.1 * 1.1).max(near + 0.001))
    }
    /// Produces the immutable finite-f32 payload consumed by the renderer.
    #[must_use]
    pub fn snapshot(self, sphere: (Vec3, f64), revision: u64) -> Option<CameraSnapshot> {
        let (near, far) = self.clip(sphere);
        let v = self.view();
        let p = self.projection(near, far);
        let convert = |m: Matrix| m.map(|column| column.map(|v| v as f32));
        CameraSnapshot::new(convert(v), convert(p), convert(multiply(p, v)), revision)
    }
    /// Ray through a physical subpixel. Captured gestures may query outside the rectangle.
    #[must_use]
    pub fn ray(self, point: (f64, f64)) -> Option<Ray> {
        if self.viewport.width == 0
            || self.viewport.height == 0
            || !point.0.is_finite()
            || !point.1.is_finite()
        {
            return None;
        }
        let w = self.viewport.width as f64;
        let h = self.viewport.height as f64;
        let x = 2.0 * (point.0 - self.viewport.x as f64) / w - 1.0 - self.lens_shift[0];
        let y = 1.0 - 2.0 * (point.1 - self.viewport.y as f64) / h - self.lens_shift[1];
        let (r, u, f) = self.basis();
        let aspect = w / h;
        let hh = if self.perspective {
            (self.vertical_fov * 0.5).tan()
        } else {
            self.orthographic_height * 0.5
        };
        let offset = add(scale(r, x * hh * aspect), scale(u, y * hh));
        Some(if self.perspective {
            Ray {
                origin: self.eye,
                direction: unit(add(f, offset)),
            }
        } else {
            Ray {
                origin: add(self.eye, offset),
                direction: f,
            }
        })
    }
    /// Screen projection used by bounds clipping and GPU-independent regression tests.
    #[must_use]
    pub fn project(self, point: Vec3) -> Option<(f64, f64)> {
        let clip = transform(
            multiply(self.projection(1e-7, 1e12), self.view()),
            [point[0], point[1], point[2], 1.0],
        );
        if clip[3] <= 0.0 || !clip.iter().all(|v| v.is_finite()) {
            return None;
        }
        Some((
            self.viewport.x as f64 + (clip[0] / clip[3] + 1.0) * self.viewport.width as f64 * 0.5,
            self.viewport.y as f64 + (1.0 - clip[1] / clip[3]) * self.viewport.height as f64 * 0.5,
        ))
    }
    /// Level screen-space yaw, followed by pitch clamped BEFORE constructing a rotation.
    /// This ordering avoids the plan's post-rotation asin fold at large pitch angles.
    #[must_use]
    pub fn orbit(self, pivot: Vec3, displacement: (f64, f64), display_scale: f64) -> Self {
        if displacement == (0.0, 0.0)
            || !displacement.0.is_finite()
            || !displacement.1.is_finite()
            || !display_scale.is_finite()
            || display_scale <= 0.0
        {
            return self;
        }
        let (heading, requested) = self.orbit_angles(displacement, display_scale);
        let limit = std::f64::consts::FRAC_PI_2 - POLE_MARGIN;
        let elevation = requested.clamp(-limit, limit);
        let f = add(
            scale(heading, elevation.cos()),
            scale(WORLD_UP, elevation.sin()),
        );
        let r = unit(cross(f, WORLD_UP));
        let u = cross(r, f);
        let orientation = Quaternion::from_basis(r, u, f).same_sign(self.orientation);
        let delta = orientation.compose(self.orientation.inverse());
        Self {
            eye: add(pivot, delta.rotate(sub(self.eye, pivot))),
            orientation,
            last_heading: heading,
            ..self
        }
    }
    fn orbit_angles(self, displacement: (f64, f64), display_scale: f64) -> (Vec3, f64) {
        let (_, up, forward) = self.basis();
        let alpha = -ORBIT_SENSITIVITY * displacement.0 / display_scale;
        let beta = -ORBIT_SENSITIVITY * displacement.1 / display_scale;
        let limit = std::f64::consts::FRAC_PI_2 - POLE_MARGIN;
        let base_elevation = forward[2].clamp(-1.0, 1.0).asin();
        let exact_pole = base_elevation.abs() > limit;
        // At an exact axis view use the stored heading rather than deriving it from a
        // nearly zero horizontal vector. The first real drag departs into the orbit band.
        let yawed = if exact_pole {
            let angle = base_elevation.clamp(-limit, limit);
            let f = add(
                scale(self.last_heading, angle.cos()),
                scale(WORLD_UP, angle.sin()),
            );
            let r = unit(cross(self.last_heading, WORLD_UP));
            Quaternion::axis_angle(cross(r, f), alpha).rotate(f)
        } else {
            Quaternion::axis_angle(up, alpha).rotate(forward)
        };
        let mut heading = unit([yawed[0], yawed[1], 0.0]);
        if length(heading) == 0.0 {
            heading = self.last_heading;
        }
        (heading, yawed[2].clamp(-1.0, 1.0).asin() + beta)
    }
    pub(crate) fn pitch_excess_pixels(self, displacement: (f64, f64), display_scale: f64) -> f64 {
        let (_, requested) = self.orbit_angles(displacement, display_scale);
        let limit = std::f64::consts::FRAC_PI_2 - POLE_MARGIN;
        (requested - requested.clamp(-limit, limit)) * display_scale / ORBIT_SENSITIVITY
    }
    /// Pan on a frozen camera-facing plane; rays work beyond the viewport boundary.
    #[must_use]
    pub fn pan(self, start: (f64, f64), end: (f64, f64), depth: f64) -> Self {
        let f = self.basis().2;
        let plane = add(self.eye, scale(f, depth.max(MIN_DISTANCE)));
        let hits = self
            .ray(start)
            .and_then(|a| plane_hit(a, plane, f))
            .zip(self.ray(end).and_then(|b| plane_hit(b, plane, f)));
        if let Some((a, b)) = hits {
            Self {
                eye: add(self.eye, sub(a, b)),
                ..self
            }
        } else {
            self
        }
    }
    /// Cursor-anchor-preserving zoom in perspective and orthographic projection.
    #[must_use]
    pub fn zoom(self, anchor: Vec3, point: (f64, f64), steps: f64, maximum: f64) -> Self {
        if !steps.is_finite() || steps == 0.0 || !finite(anchor) {
            return self;
        }
        let factor = (-0.12 * steps.clamp(-100.0, 100.0)).exp();
        if self.perspective {
            let depth = dot(sub(anchor, self.eye), self.basis().2);
            if depth <= 0.0 {
                return self;
            }
            // Intersect the anchor-depth and focal-depth scale intervals, preserving
            // anchoring rather than clamping focal distance independently afterwards.
            let lo = (MIN_DISTANCE / depth).max(MIN_DISTANCE / self.focal_distance);
            let hi = (maximum / depth).min(maximum / self.focal_distance).max(lo);
            // A changed mesh can leave the existing pose beyond new bounds: move toward
            // the valid interval without snapping when the wheel requests the other direction.
            let factor = if factor < 1.0 {
                factor.max(lo).min(1.0)
            } else {
                factor.min(hi).max(1.0)
            };
            Self {
                eye: add(anchor, scale(sub(self.eye, anchor), factor)),
                focal_distance: self.focal_distance * factor,
                ..self
            }
        } else {
            let height = (self.orthographic_height * factor).clamp(MIN_DISTANCE, maximum * 2.0);
            let mut next = Self {
                orthographic_height: height,
                ..self
            };
            let f = self.basis().2;
            if let Some(after) = next.ray(point).and_then(|ray| plane_hit(ray, anchor, f)) {
                next.eye = add(next.eye, sub(anchor, after));
            }
            next
        }
    }
    /// Changes projection with the same image scale at focal depth.
    pub fn set_perspective(&mut self, perspective: bool) -> bool {
        if self.perspective == perspective {
            return false;
        }
        if perspective {
            let target = self.target();
            self.focal_distance = (self.orthographic_height
                / (2.0 * (self.vertical_fov * 0.5).tan()))
            .max(MIN_DISTANCE);
            self.eye = sub(target, scale(self.basis().2, self.focal_distance));
        } else {
            self.orthographic_height = 2.0 * self.focal_distance * (self.vertical_fov * 0.5).tan();
        }
        self.perspective = perspective;
        true
    }
    /// Preserves physical pixel position at every depth, without changing eye or orientation.
    pub fn resize(&mut self, new: SceneViewport) {
        if self.viewport == new {
            return;
        }
        let old = self.viewport;
        if old.width > 0 && old.height > 0 && new.width > 0 && new.height > 0 {
            let focal = old.height as f64 / (2.0 * (self.vertical_fov * 0.5).tan());
            let dx =
                (old.x as f64 + old.width as f64 * 0.5) - (new.x as f64 + new.width as f64 * 0.5);
            let dy =
                (new.y as f64 + new.height as f64 * 0.5) - (old.y as f64 + old.height as f64 * 0.5);
            self.lens_shift = [
                (self.lens_shift[0] * old.width as f64 + 2.0 * dx) / new.width as f64,
                (self.lens_shift[1] * old.height as f64 + 2.0 * dy) / new.height as f64,
            ];
            self.vertical_fov = 2.0 * (new.height as f64 / (2.0 * focal)).atan();
            self.orthographic_height *= new.height as f64 / old.height as f64;
        }
        self.viewport = new;
    }
    /// Fit distance for a sphere, including both viewport axes and a 10% margin.
    #[must_use]
    pub fn fit_distance(self, radius: f64) -> f64 {
        let aspect = self.viewport.width.max(1) as f64 / self.viewport.height.max(1) as f64;
        let half = ((self.vertical_fov * 0.5).tan() * aspect.min(1.0)).atan();
        (1.1 * radius.max(MIN_DISTANCE) / half.sin()).max(MIN_DISTANCE)
    }
    /// Aligns to the closest principal view, preserving the derived focal target.
    pub fn normal_to_axis(&mut self) {
        let target = self.target();
        let f = self.basis().2;
        let axis = (0..3)
            .max_by(|a, b| f[*a].abs().total_cmp(&f[*b].abs()))
            .unwrap_or(2);
        let mut direction = [0.0; 3];
        direction[axis] = f[axis].signum();
        let heading = if axis == 2 {
            self.last_heading
        } else {
            direction
        };
        let r = unit(cross(heading, WORLD_UP));
        let u = cross(r, direction);
        self.orientation = Quaternion::from_basis(r, u, direction).same_sign(self.orientation);
        self.last_heading = heading;
        self.eye = sub(target, scale(direction, self.focal_distance));
    }
}

/// Intersection with a camera-facing plane in front of the ray origin.
#[must_use]
pub fn plane_hit(ray: Ray, point: Vec3, normal: Vec3) -> Option<Vec3> {
    let divisor = dot(ray.direction, normal);
    if divisor.abs() < 1e-12 {
        return None;
    }
    let t = dot(sub(point, ray.origin), normal) / divisor;
    (t >= 0.0 && t.is_finite()).then(|| add(ray.origin, scale(ray.direction, t)))
}
/// Column-major matrix product.
#[must_use]
pub fn multiply(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|c| std::array::from_fn(|r| (0..4).map(|i| a[i][r] * b[c][i]).sum()))
}
/// Homogeneous column-vector transformation.
#[must_use]
pub fn transform(m: Matrix, v: [f64; 4]) -> [f64; 4] {
    std::array::from_fn(|r| (0..4).map(|i| m[i][r] * v[i]).sum())
}
