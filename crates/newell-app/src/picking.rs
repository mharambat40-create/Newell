// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! CPU geometric queries over the Newell-owned displayed tessellation.

use crate::camera::{
    Camera, Ray, Vec3, add, cross, dot, finite, length, multiply, scale, sub, transform,
};
use newell_kernel::NewellMesh;
use newell_render::{SceneViewport, exposed_regions};
use std::cell::RefCell;

/// World-space axis-aligned bounds. The radius encloses all points in this box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}
impl Bounds {
    fn empty() -> Self {
        Self {
            min: [f64::INFINITY; 3],
            max: [f64::NEG_INFINITY; 3],
        }
    }
    fn include(&mut self, p: Vec3) {
        for (i, v) in p.into_iter().enumerate() {
            self.min[i] = self.min[i].min(v);
            self.max[i] = self.max[i].max(v);
        }
    }
    fn valid(self) -> bool {
        finite(self.min) && finite(self.max)
    }
    /// Centre of the world-space AABB, never a screen-space centre substituted as a target.
    #[must_use]
    pub fn centre(self) -> Vec3 {
        scale(add(self.min, self.max), 0.5)
    }
    /// Conservative sphere for fit and clipping.
    #[must_use]
    pub fn sphere(self) -> (Vec3, f64) {
        (self.centre(), length(sub(self.max, self.min)) * 0.5)
    }
}

/// Nearest positive intersection with the rendered tessellation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickHit {
    pub point: Vec3,
    pub distance: f64,
    pub face_index: usize,
    pub mesh_revision: u64,
}

#[derive(Debug)]
struct Triangle {
    vertices: [Vec3; 3],
    bounds: Bounds,
    face: usize,
}
#[derive(Debug)]
struct VisibleCache {
    camera: Camera,
    occlusions: Vec<SceneViewport>,
    bounds: Option<Bounds>,
}

/// Geometry cache replaced atomically with the displayed mesh; no upstream kernel types.
#[derive(Debug, Default)]
pub struct PickingScene {
    triangles: Vec<Triangle>,
    bounds: Option<Bounds>,
    revision: u64,
    visible: RefCell<Option<VisibleCache>>,
}
impl PickingScene {
    /// Copies the current tessellation into an immutable triangle query cache.
    pub fn set_mesh(&mut self, mesh: &NewellMesh) {
        self.triangles.clear();
        self.bounds = None;
        self.revision = self.revision.wrapping_add(1);
        self.visible.get_mut().take();
        let mut all = Bounds::empty();
        for p in mesh.positions() {
            let point = p.map(f64::from);
            if finite(point) {
                all.include(point);
            }
        }
        self.bounds = all.valid().then_some(all);
        for (ordinal, indices) in mesh.indices().as_chunks::<3>().0.iter().enumerate() {
            let points = indices
                .iter()
                .map(|index| {
                    mesh.positions()
                        .get(*index as usize)
                        .map(|p| p.map(f64::from))
                })
                .collect::<Option<Vec<_>>>();
            let Some(points) = points else {
                continue;
            };
            let vertices = [points[0], points[1], points[2]];
            if !vertices.iter().all(|p| finite(*p)) {
                continue;
            }
            let mut bounds = Bounds::empty();
            for p in vertices {
                bounds.include(p);
            }
            let face = mesh
                .face_offsets()
                .windows(2)
                .position(|r| ordinal * 3 >= r[0] as usize && ordinal * 3 < (r[1] as usize))
                .unwrap_or(0);
            self.triangles.push(Triangle {
                vertices,
                bounds,
                face,
            });
        }
    }
    /// Revision changes on every mesh replacement, including identical preview geometry.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    /// Bounds of all displayed geometry, or None for an empty scene.
    #[must_use]
    pub const fn bounds(&self) -> Option<Bounds> {
        self.bounds
    }
    /// Tests all displayed triangles; UI hit-testing is a separate constant-time operation.
    #[must_use]
    pub fn pick(&self, ray: Ray) -> Option<PickHit> {
        if !finite(ray.origin) || !finite(ray.direction) {
            return None;
        }
        self.triangles
            .iter()
            .filter(|triangle| ray_box(ray, triangle.bounds))
            .filter_map(|triangle| {
                intersect_triangle(ray, triangle.vertices).map(|distance| PickHit {
                    point: add(ray.origin, scale(ray.direction, distance)),
                    distance,
                    face_index: triangle.face,
                    mesh_revision: self.revision,
                })
            })
            .min_by(|a, b| a.distance.total_cmp(&b.distance))
    }
    /// AABB of triangle portions clipped to the frustum and exposed viewport rectangles.
    /// Self-occluded surfaces are included; hidden entities are absent from the displayed mesh.
    #[must_use]
    pub fn visible_bounds(&self, camera: Camera, occlusions: &[SceneViewport]) -> Option<Bounds> {
        if let Some(cache) = self.visible.borrow().as_ref()
            && cache.camera == camera
            && cache.occlusions == occlusions
        {
            return cache.bounds;
        }
        let bounds = self.calculate_visible(camera, occlusions);
        *self.visible.borrow_mut() = Some(VisibleCache {
            camera,
            occlusions: occlusions.to_vec(),
            bounds,
        });
        bounds
    }
    fn calculate_visible(&self, camera: Camera, occlusions: &[SceneViewport]) -> Option<Bounds> {
        let sphere = self.bounds?.sphere();
        let (near, far) = camera.clip(sphere);
        let matrix = multiply(camera.projection(near, far), camera.view());
        let viewport = camera.viewport;
        if viewport.width == 0 || viewport.height == 0 {
            return None;
        }
        let mut result = Bounds::empty();
        for region in exposed_regions(viewport, occlusions) {
            let left = 2.0 * (region.x as f64 - viewport.x as f64) / viewport.width as f64 - 1.0;
            let right = left + 2.0 * region.width as f64 / viewport.width as f64;
            let top = 1.0 - 2.0 * (region.y as f64 - viewport.y as f64) / viewport.height as f64;
            let bottom = top - 2.0 * region.height as f64 / viewport.height as f64;
            let planes = [
                [1.0, 0.0, 0.0, -left],
                [-1.0, 0.0, 0.0, right],
                [0.0, 1.0, 0.0, -bottom],
                [0.0, -1.0, 0.0, top],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, -1.0, 1.0],
            ];
            for triangle in &self.triangles {
                let mut polygon = triangle
                    .vertices
                    .iter()
                    .map(|world| ClipVertex {
                        world: *world,
                        clip: transform(matrix, [world[0], world[1], world[2], 1.0]),
                    })
                    .collect::<Vec<_>>();
                for plane in planes {
                    polygon = clip_polygon(&polygon, plane);
                    if polygon.is_empty() {
                        break;
                    }
                }
                for vertex in polygon {
                    if finite(vertex.world) {
                        result.include(vertex.world);
                    }
                }
            }
        }
        result.valid().then_some(result)
    }
}

#[derive(Clone, Copy)]
struct ClipVertex {
    world: Vec3,
    clip: [f64; 4],
}
fn clip_polygon(polygon: &[ClipVertex], plane: [f64; 4]) -> Vec<ClipVertex> {
    let signed = |v: ClipVertex| {
        v.clip
            .into_iter()
            .zip(plane)
            .map(|(a, b)| a * b)
            .sum::<f64>()
    };
    let Some(mut previous) = polygon.last().copied() else {
        return Vec::new();
    };
    let mut output = Vec::with_capacity(polygon.len() + 1);
    for current in polygon.iter().copied() {
        let a = signed(previous);
        let b = signed(current);
        if (a >= 0.0) != (b >= 0.0) {
            let t = a / (a - b);
            output.push(ClipVertex {
                world: add(previous.world, scale(sub(current.world, previous.world), t)),
                clip: std::array::from_fn(|i| {
                    previous.clip[i] + t * (current.clip[i] - previous.clip[i])
                }),
            });
        }
        if b >= 0.0 {
            output.push(current);
        }
        previous = current;
    }
    output
}
fn ray_box(ray: Ray, bounds: Bounds) -> bool {
    let mut lo = 0.0_f64;
    let mut hi = f64::INFINITY;
    for i in 0..3 {
        if ray.direction[i].abs() < 1e-14 {
            if ray.origin[i] < bounds.min[i] || ray.origin[i] > bounds.max[i] {
                return false;
            }
        } else {
            let a = (bounds.min[i] - ray.origin[i]) / ray.direction[i];
            let b = (bounds.max[i] - ray.origin[i]) / ray.direction[i];
            lo = lo.max(a.min(b));
            hi = hi.min(a.max(b));
        }
    }
    hi >= lo
}
/// Two-sided tessellation intersection; degeneracy is rejected relative to triangle scale.
#[must_use]
pub fn intersect_triangle(ray: Ray, triangle: [Vec3; 3]) -> Option<f64> {
    let [a, b, c] = triangle;
    let e1 = sub(b, a);
    let e2 = sub(c, a);
    let h = cross(ray.direction, e2);
    let det = dot(e1, h);
    if det.abs() <= 1e-12 * length(e1) * length(e2) || det == 0.0 {
        return None;
    }
    let inv = 1.0 / det;
    let s = sub(ray.origin, a);
    let u = dot(s, h) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross(s, e1);
    let v = dot(ray.direction, q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = dot(e2, q) * inv;
    (t > 0.0 && t.is_finite()).then_some(t)
}
