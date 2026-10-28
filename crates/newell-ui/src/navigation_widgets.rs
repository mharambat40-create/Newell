// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Fixed navigation geometry shared by Controller hit tests and View projection.
use crate::GridMetrics;
pub type V3 = [f64; 3];
pub fn dot(a: V3, b: V3) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
pub fn add(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] + b[i])
}
pub fn mul(a: V3, k: f64) -> V3 {
    a.map(|v| v * k)
}
pub fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn unit(a: V3) -> V3 {
    let n = dot(a, a).sqrt();
    if n > 1e-14 { mul(a, 1.0 / n) } else { [0.; 3] }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavigatorKind {
    #[default]
    Gizmo,
    Cube,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ViewDirection([i8; 3]);
impl ViewDirection {
    pub fn new(n: [i8; 3]) -> Option<Self> {
        (n != [0; 3] && n.iter().all(|v| (-1..=1).contains(v))).then_some(Self(n))
    }
    pub fn components(self) -> [i8; 3] {
        self.0
    }
    pub fn normal(self) -> V3 {
        unit(self.0.map(f64::from))
    }
    pub fn dimensions(self) -> usize {
        self.0.iter().filter(|v| **v != 0).count()
    }
    pub fn all() -> Vec<Self> {
        let mut out = Vec::with_capacity(26);
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if let Some(n) = Self::new([x, y, z]) {
                        out.push(n);
                    }
                }
            }
        }
        out
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignedAxis {
    X,
    Y,
    Z,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisCap {
    pub axis: SignedAxis,
    pub positive: bool,
}
impl AxisCap {
    pub fn from_direction(d: ViewDirection) -> Option<Self> {
        if d.dimensions() != 1 {
            return None;
        }
        let n = d.components();
        let axis = if n[0] != 0 {
            SignedAxis::X
        } else if n[1] != 0 {
            SignedAxis::Y
        } else {
            SignedAxis::Z
        };
        Some(Self {
            axis,
            positive: n.contains(&1),
        })
    }
    pub fn direction(self) -> ViewDirection {
        let mut n = [0; 3];
        n[match self.axis {
            SignedAxis::X => 0,
            SignedAxis::Y => 1,
            SignedAxis::Z => 2,
        }] = if self.positive { 1 } else { -1 };
        ViewDirection(n)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavPart {
    Cube(ViewDirection),
    Axis(AxisCap),
}
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PixelRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl PixelRect {
    pub fn contains(self, p: (f64, f64)) -> bool {
        p.0 >= self.x && p.1 >= self.y && p.0 < self.x + self.width && p.1 < self.y + self.height
    }
    pub fn intersects(self, b: Self) -> bool {
        self.x < b.x + b.width
            && b.x < self.x + self.width
            && self.y < b.y + b.height
            && b.y < self.y + self.height
    }
    pub fn intersection(self, b: Self) -> Self {
        let x = self.x.max(b.x);
        let y = self.y.max(b.y);
        Self {
            x,
            y,
            width: ((self.x + self.width).min(b.x + b.width) - x).max(0.),
            height: ((self.y + self.height).min(b.y + b.height) - y).max(0.),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NavBasis {
    pub right: V3,
    pub up: V3,
    pub forward: V3,
}
impl NavBasis {
    pub fn valid(self) -> bool {
        [self.right, self.up, self.forward]
            .iter()
            .all(|a| a.iter().all(|v| v.is_finite()) && (dot(*a, *a) - 1.).abs() < 1e-8)
            && dot(self.right, self.up).abs() < 1e-8
            && dot(cross(self.right, self.up), mul(self.forward, -1.)) > 1. - 1e-8
    }
    pub fn project(self, v: V3) -> [f64; 3] {
        [dot(self.right, v), -dot(self.up, v), dot(self.forward, v)]
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NavMetrics {
    pub q: f64,
}
impl NavMetrics {
    pub fn new(m: GridMetrics) -> Self {
        Self {
            q: f64::from(m.cell_width_px()) / 8.,
        }
    }
    pub fn tile_size(self, _kind: NavigatorKind) -> f64 {
        self.q * NAV_TILE_SIZE_Q
    }
}
/// Both corner widgets occupy the same nominal screen-space tile.
pub const NAV_TILE_SIZE_Q: f64 = 96.0;
/// Shared design-unit axis geometry and opacity lanes, serialized verbatim for WGSL.
/// radius sphere/disc/ring, ring width; plane half-side/border/center alpha/edge alpha;
/// shaft width/start alpha/end alpha/front ring alpha; rear ring alpha/sphere light/range/border alpha.
pub const AXIS_STYLE: [[f32; 4]; 4] = [
    [4., 7., 9., 1.],
    [0.35, 0.5, 0.10, 0.16],
    [0.8, 0.35, 0.85, 0.55],
    [0.35, 0.8, 0.2, 0.20],
];
/// Shared palette for corner-widget markers and world-origin references.
pub const NAV_AXIS_COLORS: [[f32; 3]; 3] = [
    [245. / 255., 34. / 255., 53. / 255.],
    [0., 181. / 255., 42. / 255.],
    [48. / 255., 103. / 255., 245. / 255.],
];
/// Compatibility name for the same shared palette.
pub const AXIS_COLORS: [[f32; 3]; 3] = NAV_AXIS_COLORS;
pub const POSITIVE_DISC_RADIUS: f64 = 7.0 * 1.15;
pub const NEGATIVE_RING_OUTER_RADIUS: f64 = POSITIVE_DISC_RADIUS * 0.8;
pub const NEGATIVE_RING_STROKE_WIDTH: f64 = 1.0;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisMarkerMaterial {
    pub fill: [f32; 4],
    pub stroke: [f32; 4],
    pub letter: [f32; 4],
}
pub fn axis_marker_material(cap: AxisCap, hovered: bool) -> AxisMarkerMaterial {
    let rgb = NAV_AXIS_COLORS[match cap.axis {
        SignedAxis::X => 0,
        SignedAxis::Y => 1,
        SignedAxis::Z => 2,
    }];
    let color = [rgb[0], rgb[1], rgb[2], 1.];
    let white = [1.; 4];
    if cap.positive {
        AxisMarkerMaterial {
            fill: if hovered { [0.; 4] } else { color },
            stroke: color,
            letter: if hovered { color } else { white },
        }
    } else {
        AxisMarkerMaterial {
            fill: [0.; 4],
            stroke: if hovered {
                color
            } else {
                // HSV saturation *= 0.8: retain hue and maximum/value.
                let value = rgb.into_iter().fold(0., f32::max);
                let muted = rgb.map(|channel| value - 0.8 * (value - channel));
                [muted[0], muted[1], muted[2], 0.55]
            },
            letter: [0.; 4],
        }
    }
}
pub const CUBE_COLORS: [[f32; 3]; 3] = [
    [217. / 255., 221. / 255., 226. / 255.],
    [200. / 255., 205. / 255., 212. / 255.],
    [190. / 255., 197. / 255., 206. / 255.],
];
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NavLayout {
    pub tile: Option<PixelRect>,
    pub kind: NavigatorKind,
    pub generation: u64,
}
pub fn layout_navigation(
    scene: PixelRect,
    surface: PixelRect,
    metrics: GridMetrics,
    visible: bool,
    kind: NavigatorKind,
    action: Option<PixelRect>,
) -> NavLayout {
    let m = NavMetrics::new(metrics);
    let size = m.tile_size(kind);
    let scene = scene.intersection(surface);
    let mut tile = None;
    if visible && scene.width >= size + 16. * m.q && scene.height >= size + 16. * m.q {
        let r = PixelRect {
            x: (scene.x + scene.width - 8. * m.q - size).round(),
            y: (scene.y + 8. * m.q).round(),
            width: size,
            height: size,
        };
        if !action.is_some_and(|a| a.intersects(r)) {
            tile = Some(r);
        }
    }
    NavLayout {
        tile,
        kind,
        generation: 0,
    }
}
#[derive(Debug, Clone)]
pub struct CubePatch {
    pub direction: ViewDirection,
    pub normal: V3,
    pub limit: f64,
    pub vertices: Vec<V3>,
}
pub fn cube_planes() -> Vec<(ViewDirection, V3, f64)> {
    ViewDirection::all()
        .into_iter()
        .map(|d| {
            (
                d,
                d.components().map(f64::from),
                match d.dimensions() {
                    1 => 0.5,
                    2 => 0.90,
                    _ => 1.26,
                },
            )
        })
        .collect()
}
/// Clip each support polygon by the same planes used for analytic entry picking.
pub fn cube_patches() -> &'static [CubePatch] {
    static PATCHES: std::sync::OnceLock<Vec<CubePatch>> = std::sync::OnceLock::new();
    PATCHES.get_or_init(build_cube_patches)
}
fn build_cube_patches() -> Vec<CubePatch> {
    let planes = cube_planes();
    planes
        .iter()
        .map(|&(direction, n, limit)| {
            let normal = unit(n);
            let a = if normal[2].abs() < 0.9 {
                [0., 0., 1.]
            } else {
                [0., 1., 0.]
            };
            let u = unit(cross(a, normal));
            let v = cross(normal, u);
            let c = mul(n, limit / dot(n, n));
            let mut poly = vec![
                add(c, add(mul(u, -2.), mul(v, -2.))),
                add(c, add(mul(u, 2.), mul(v, -2.))),
                add(c, add(mul(u, 2.), mul(v, 2.))),
                add(c, add(mul(u, -2.), mul(v, 2.))),
            ];
            for &(_, b, k) in &planes {
                let mut clipped = Vec::new();
                for i in 0..poly.len() {
                    let p = poly[i];
                    let q = poly[(i + 1) % poly.len()];
                    let dp = dot(b, p) - k;
                    let dq = dot(b, q) - k;
                    if dp <= 1e-12 {
                        clipped.push(p);
                    }
                    if (dp < -1e-12 && dq > 1e-12) || (dp > 1e-12 && dq < -1e-12) {
                        clipped.push(add(p, mul(add(q, mul(p, -1.)), dp / (dp - dq))));
                    }
                }
                poly = clipped;
            }
            // Consecutive duplicates arise where a clipping plane passes through an existing vertex.
            poly.dedup_by(|a, b| dot(add(*a, mul(*b, -1.)), add(*a, mul(*b, -1.))) < 1e-22);
            if poly.len() > 1
                && dot(
                    add(poly[0], mul(poly[poly.len() - 1], -1.)),
                    add(poly[0], mul(poly[poly.len() - 1], -1.)),
                ) < 1e-22
            {
                poly.pop();
            }
            CubePatch {
                direction,
                normal,
                limit,
                vertices: poly,
            }
        })
        .collect()
}
#[derive(Debug, Clone, PartialEq)]
pub struct NavGeometry {
    pub layout: NavLayout,
    pub basis: NavBasis,
    pub q: f64,
    cube_scale_q: f64,
}
impl NavGeometry {
    pub fn new(layout: NavLayout, basis: NavBasis, metrics: GridMetrics) -> Option<Self> {
        basis.valid().then_some(Self {
            layout,
            basis,
            q: NavMetrics::new(metrics).q,
            cube_scale_q: cube_projection_scale_for(basis),
        })
    }
    pub fn reserves(&self, p: (f64, f64)) -> bool {
        self.hit(p).is_some()
    }
    pub fn project(&self, v: V3) -> [f64; 3] {
        let p = self.basis.project(v);
        let scale = if self.layout.kind == NavigatorKind::Cube {
            self.cube_projection_scale_q()
        } else {
            31.
        };
        [
            self.q * (NAV_TILE_SIZE_Q / 2. + scale * p[0]),
            self.q * (NAV_TILE_SIZE_Q / 2. + scale * p[1]),
            p[2],
        ]
    }
    pub fn hit(&self, p: (f64, f64)) -> Option<NavPart> {
        let tile = self.layout.tile?;
        if !tile.contains(p) {
            return None;
        }
        let p = (p.0 - tile.x, p.1 - tile.y);
        match self.layout.kind {
            NavigatorKind::Cube => self.cube_hit(p).map(NavPart::Cube),
            NavigatorKind::Gizmo => self
                .axis_hit(p)
                .and_then(AxisCap::from_direction)
                .map(NavPart::Axis),
        }
    }
    fn cube_hit(&self, p: (f64, f64)) -> Option<ViewDirection> {
        let scale = self.cube_projection_scale_q();
        let x = (p.0 / self.q - NAV_TILE_SIZE_Q / 2.) / scale;
        let y = -(p.1 / self.q - NAV_TILE_SIZE_Q / 2.) / scale;
        let origin = add(
            add(mul(self.basis.right, x), mul(self.basis.up, y)),
            mul(self.basis.forward, -2.),
        );
        let mut enter = f64::NEG_INFINITY;
        let mut exit = f64::INFINITY;
        let mut hit = None;
        for (id, n, k) in cube_planes() {
            let den = dot(n, self.basis.forward);
            let d = k - dot(n, origin);
            if den.abs() < 1e-12 {
                if d < -1e-12 {
                    return None;
                }
                continue;
            }
            let t = d / den;
            if den < 0. {
                if t > enter + 1e-9 {
                    enter = t;
                    hit = Some(id);
                } else if (t - enter).abs() <= 1e-9
                    && hit.is_none_or(|old: ViewDirection| {
                        id.dimensions() > old.dimensions()
                            || (id.dimensions() == old.dimensions() && id < old)
                    })
                {
                    hit = Some(id);
                }
            } else {
                exit = exit.min(t);
            }
        }
        (enter <= exit + 1e-12 && exit >= 0.)
            .then_some(hit)
            .flatten()
    }
    /// Scales the beveled cube to the current Gizmo's projected outer radius.
    /// Both the projected cube bounds and cap/ring bounds therefore share a screen size.
    pub fn cube_projection_scale_q(&self) -> f64 {
        self.cube_scale_q
    }
    pub fn caps(&self) -> Vec<(ViewDirection, [f64; 3])> {
        let mut caps = ViewDirection::all()
            .into_iter()
            .filter(|d| d.dimensions() == 1)
            .map(|d| (d, self.project(d.components().map(f64::from))))
            .collect::<Vec<_>>();
        caps.sort_by(|a, b| b.1[2].total_cmp(&a.1[2]).then(a.0.cmp(&b.0)));
        caps
    }
    fn axis_hit(&self, p: (f64, f64)) -> Option<ViewDirection> {
        let mut hit = None;
        for (id, c) in self.caps() {
            let radius = ((p.0 - c[0]).powi(2) + (p.1 - c[1]).powi(2)).sqrt() / self.q;
            let positive = id.components().contains(&1);
            let covers = radius
                <= if positive {
                    POSITIVE_DISC_RADIUS
                } else {
                    NEGATIVE_RING_OUTER_RADIUS
                };
            if covers {
                hit = Some(id);
            }
        }
        hit
    }
}
fn cube_projection_scale_for(basis: NavBasis) -> f64 {
    // Preserve the cube baseline when only the gizmo marker diameter changes.
    let gizmo_radius = ViewDirection::all()
        .into_iter()
        .filter(|direction| direction.dimensions() == 1)
        .map(|direction| {
            let projected = basis.project(direction.components().map(f64::from));
            let marker_radius = if direction.components().contains(&1) {
                f64::from(AXIS_STYLE[0][1])
            } else {
                5.6
            };
            31. * projected[0].hypot(projected[1]) + marker_radius
        })
        .fold(0., f64::max);
    let cube_radius = cube_patches()
        .iter()
        .flat_map(|patch| patch.vertices.iter())
        .map(|&v| dot(basis.right, v).hypot(dot(basis.up, v)))
        .fold(0., f64::max);
    if cube_radius > 0. && cube_radius.is_finite() {
        gizmo_radius / cube_radius
    } else {
        0.
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn basis(n: V3) -> NavBasis {
        let f = mul(unit(n), -1.);
        let h = if f[0] == 0. && f[1] == 0. {
            [0., 1., 0.]
        } else {
            unit([f[0], f[1], 0.])
        };
        let r = unit(cross(h, [0., 0., 1.]));
        NavBasis {
            right: r,
            up: cross(r, f),
            forward: f,
        }
    }
    #[test]
    fn all_patches_have_area_and_independent_support_constraints() {
        let patches = cube_patches();
        assert_eq!(patches.len(), 26);
        for p in patches {
            assert!(p.vertices.len() >= 3, "{:?}", p.direction);
            for v in &p.vertices {
                for (_, n, k) in cube_planes() {
                    assert!(dot(n, *v) <= k + 1e-10);
                }
            }
            let area = cross(
                add(p.vertices[1], mul(p.vertices[0], -1.)),
                add(p.vertices[2], mul(p.vertices[0], -1.)),
            );
            assert!(dot(area, p.normal) > 0.);
        }
    }
    #[test]
    fn each_of_26_components_is_hit_at_its_front_center() {
        let metrics = GridMetrics::from_k(2).unwrap();
        for patch in cube_patches() {
            let n = patch.direction.normal();
            let layout = layout_navigation(
                PixelRect {
                    x: 0.,
                    y: 0.,
                    width: 400.,
                    height: 400.,
                },
                PixelRect {
                    x: 0.,
                    y: 0.,
                    width: 400.,
                    height: 400.,
                },
                metrics,
                true,
                NavigatorKind::Cube,
                None,
            );
            let g = NavGeometry::new(layout, basis(n), metrics).unwrap();
            let center = mul(
                patch.vertices.iter().copied().fold([0.; 3], add),
                1. / patch.vertices.len() as f64,
            );
            let p = g.project(center);
            let tile = layout.tile.unwrap();
            assert_eq!(
                g.hit((tile.x + p[0], tile.y + p[1])),
                Some(NavPart::Cube(patch.direction))
            );
        }
    }
    #[test]
    fn selected_layout_threshold_does_not_substitute_kind() {
        for k in 2..=7 {
            let m = GridMetrics::from_k(k).unwrap();
            for kind in [NavigatorKind::Gizmo, NavigatorKind::Cube] {
                let size = NavMetrics::new(m).tile_size(kind) + 8. * f64::from(k);
                let r = PixelRect {
                    x: 0.,
                    y: 0.,
                    width: size,
                    height: size,
                };
                assert!(layout_navigation(r, r, m, true, kind, None).tile.is_some());
                let small = PixelRect {
                    width: size - 0.01,
                    ..r
                };
                let l = layout_navigation(small, small, m, true, kind, None);
                assert!(l.tile.is_none());
                assert_eq!(l.kind, kind);
            }
        }
    }

    #[test]
    fn gizmo_and_cube_share_tile_bounds_and_nominal_widget_extent() {
        for k in 2..=7 {
            let metrics = GridMetrics::from_k(k).unwrap();
            let rect = PixelRect {
                x: 0.,
                y: 0.,
                width: 800.,
                height: 800.,
            };
            let layouts = [NavigatorKind::Gizmo, NavigatorKind::Cube]
                .map(|kind| layout_navigation(rect, rect, metrics, true, kind, None));
            let gizmo_tile = layouts[0].tile.unwrap();
            let cube_tile = layouts[1].tile.unwrap();
            assert_eq!(gizmo_tile, cube_tile);

            for view in [[1., -1., 1.], [3.4, 3., 3.2], [0., 0., 1.]] {
                let view_basis = basis(view);
                let gizmo = NavGeometry::new(layouts[0], view_basis, metrics).unwrap();
                let cube = NavGeometry::new(layouts[1], view_basis, metrics).unwrap();
                let cube_extent = cube_patches()
                    .iter()
                    .flat_map(|patch| patch.vertices.iter())
                    .map(|&v| {
                        let p = cube.project(v);
                        ((p[0] - NAV_TILE_SIZE_Q * cube.q / 2.).powi(2)
                            + (p[1] - NAV_TILE_SIZE_Q * cube.q / 2.).powi(2))
                        .sqrt()
                    })
                    .fold(0., f64::max);
                let gizmo_extent = gizmo
                    .caps()
                    .iter()
                    .map(|(direction, p)| {
                        let radius = if direction.components().contains(&1) {
                            7.
                        } else {
                            5.6
                        };
                        ((p[0] - NAV_TILE_SIZE_Q * gizmo.q / 2.).powi(2)
                            + (p[1] - NAV_TILE_SIZE_Q * gizmo.q / 2.).powi(2))
                        .sqrt()
                            + radius * gizmo.q
                    })
                    .fold(0., f64::max);
                assert!(
                    (cube_extent - gizmo_extent).abs() < 1e-8 * gizmo.q,
                    "cube {cube_extent}, gizmo {gizmo_extent}, q {}, view {view:?}",
                    gizmo.q
                );
            }
        }
    }
}

#[cfg(test)]
mod hit_regressions {
    use super::*;
    fn basis() -> NavBasis {
        let f = unit([-1., 1., -1.]);
        let r = unit(cross(f, [0., 0., 1.]));
        NavBasis {
            right: r,
            up: cross(r, f),
            forward: f,
        }
    }
    // Independent Moller-Trumbore intersection, rather than production half-space clipping.
    fn triangle(origin: V3, ray: V3, a: V3, b: V3, c: V3) -> Option<f64> {
        let e1 = add(b, mul(a, -1.));
        let e2 = add(c, mul(a, -1.));
        let h = cross(ray, e2);
        let det = dot(e1, h);
        if det.abs() < 1e-12 {
            return None;
        }
        let inv = 1. / det;
        let offset = add(origin, mul(a, -1.));
        let u = dot(offset, h) * inv;
        if !(0.000001..=0.999999).contains(&u) {
            return None;
        }
        let q = cross(offset, e1);
        let v = dot(ray, q) * inv;
        if v < 0.000001 || u + v > 0.999999 {
            return None;
        }
        let t = dot(e2, q) * inv;
        (t > 0.).then_some(t)
    }
    #[test]
    fn pixel_ray_hits_agree_with_independent_triangle_intersections() {
        let m = GridMetrics::from_k(2).unwrap();
        let rect = PixelRect {
            x: 0.,
            y: 0.,
            width: 400.,
            height: 400.,
        };
        let l = layout_navigation(rect, rect, m, true, NavigatorKind::Cube, None);
        let g = NavGeometry::new(l, basis(), m).unwrap();
        let t = l.tile.unwrap();
        let patches = cube_patches();
        let mut compared = 0;
        for x in (20..100).step_by(3) {
            for y in (20..100).step_by(3) {
                let px = x as f64 + 0.173;
                let py = y as f64 + 0.317;
                let origin = add(
                    add(
                        mul(
                            g.basis.right,
                            (px - NAV_TILE_SIZE_Q * g.q / 2.) / (g.cube_projection_scale_q() * g.q),
                        ),
                        mul(
                            g.basis.up,
                            -(py - NAV_TILE_SIZE_Q * g.q / 2.)
                                / (g.cube_projection_scale_q() * g.q),
                        ),
                    ),
                    mul(g.basis.forward, -2.),
                );
                let mut nearest = None;
                for patch in patches {
                    for i in 1..patch.vertices.len() - 1 {
                        if let Some(distance) = triangle(
                            origin,
                            g.basis.forward,
                            patch.vertices[0],
                            patch.vertices[i],
                            patch.vertices[i + 1],
                        ) && nearest.is_none_or(|(old, _)| distance < old)
                        {
                            nearest = Some((distance, patch.direction));
                        }
                    }
                }
                if let Some((_, id)) = nearest {
                    assert_eq!(g.hit((t.x + px, t.y + py)), Some(NavPart::Cube(id)));
                    compared += 1;
                }
            }
        }
        assert!(compared > 200);
    }
    #[test]
    fn overlapping_circular_targets_use_frontmost_cap_including_hollow_centers() {
        let m = GridMetrics::from_k(2).unwrap();
        let rect = PixelRect {
            x: 0.,
            y: 0.,
            width: 400.,
            height: 400.,
        };
        let l = layout_navigation(rect, rect, m, true, NavigatorKind::Gizmo, None);
        let g = NavGeometry::new(
            l,
            NavBasis {
                right: [1., 0., 0.],
                up: [0., 1., 0.],
                forward: [0., 0., -1.],
            },
            m,
        )
        .unwrap();
        let t = l.tile.unwrap();
        assert_eq!(
            g.hit((t.x + 48., t.y + 48.)),
            Some(NavPart::Axis(AxisCap {
                axis: SignedAxis::Z,
                positive: true
            }))
        );
        assert_eq!(
            g.hit((t.x + 53., t.y + 48.)),
            Some(NavPart::Axis(AxisCap {
                axis: SignedAxis::Z,
                positive: true
            }))
        );
        assert_eq!(g.hit((t.x + 58., t.y + 48.)), None);
        let negx = g.project([-1., 0., 0.]);
        assert_eq!(
            g.hit((t.x + negx[0], t.y + negx[1])),
            Some(NavPart::Axis(AxisCap {
                axis: SignedAxis::X,
                positive: false
            }))
        );
    }
    #[test]
    fn action_footprint_suppresses_selected_tile_and_hiding_removes_reservation() {
        let m = GridMetrics::from_k(2).unwrap();
        let r = PixelRect {
            x: 0.,
            y: 0.,
            width: 400.,
            height: 400.,
        };
        for kind in [NavigatorKind::Gizmo, NavigatorKind::Cube] {
            let l = layout_navigation(r, r, m, true, kind, None);
            assert!(
                layout_navigation(r, r, m, true, kind, l.tile)
                    .tile
                    .is_none()
            );
            assert!(layout_navigation(r, r, m, false, kind, None).tile.is_none());
        }
    }
}

#[cfg(test)]
mod seam_tests {
    use super::*;
    #[test]
    fn face_edge_and_edge_vertex_boundaries_choose_more_specific_component() {
        let m = GridMetrics::from_k(2).unwrap();
        let r = PixelRect {
            x: 0.,
            y: 0.,
            width: 400.,
            height: 400.,
        };
        for x in [-1, 1] {
            for y in [-1, 1] {
                for z in [-1, 1] {
                    let n = [x, y, z];
                    let f = mul(unit(n.map(f64::from)), -1.);
                    let right = unit(cross(f, [0., 0., 1.]));
                    let b = NavBasis {
                        right,
                        up: cross(right, f),
                        forward: f,
                    };
                    let l = layout_navigation(r, r, m, true, NavigatorKind::Cube, None);
                    let g = NavGeometry::new(l, b, m).unwrap();
                    let tile = l.tile.unwrap();
                    for (point, expected) in [
                        ([f64::from(x) * 0.5, f64::from(y) * 0.4, 0.], [x, y, 0]),
                        (
                            [f64::from(x) * 0.5, f64::from(y) * 0.4, f64::from(z) * 0.36],
                            n,
                        ),
                    ] {
                        let p = g.project(point);
                        assert_eq!(
                            g.hit((tile.x + p[0], tile.y + p[1])),
                            Some(NavPart::Cube(ViewDirection::new(expected).unwrap()))
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn signed_caps_cannot_represent_diagonal_directions() {
        for direction in ViewDirection::all() {
            assert_eq!(
                AxisCap::from_direction(direction).is_some(),
                direction.dimensions() == 1
            );
            if let Some(cap) = AxisCap::from_direction(direction) {
                assert_eq!(cap.direction(), direction);
            }
        }
        assert!(ViewDirection::new([0; 3]).is_none());
        assert!(ViewDirection::new([2, 0, 0]).is_none());
    }
}

#[cfg(test)]
mod marker_material_tests {
    use super::*;
    #[test]
    fn all_six_markers_have_exact_rest_and_hover_materials_and_fixed_sizes() {
        let expected = [[245., 34., 53.], [0., 181., 42.], [48., 103., 245.]];
        assert!(
            (2. * NEGATIVE_RING_OUTER_RADIUS / (2. * POSITIVE_DISC_RADIUS) - 0.8).abs() < 1e-12
        );
        assert_eq!(NEGATIVE_RING_STROKE_WIDTH, 1.);
        assert!((POSITIVE_DISC_RADIUS / 7. - 1.15).abs() < 1e-12);
        for (i, axis) in [SignedAxis::X, SignedAxis::Y, SignedAxis::Z]
            .into_iter()
            .enumerate()
        {
            let rgb = expected[i].map(|v| v / 255.);
            let color = [rgb[0], rgb[1], rgb[2], 1.];
            for positive in [false, true] {
                let cap = AxisCap { axis, positive };
                let rest = axis_marker_material(cap, false);
                let hover = axis_marker_material(cap, true);
                if positive {
                    assert_eq!(rest.fill, color);
                    assert_eq!(rest.letter, [1.; 4]);
                    assert_eq!(rest.stroke, color);
                    assert_eq!(hover.fill, [0.; 4]);
                    assert_eq!(hover.stroke, color);
                    assert_eq!(hover.letter, color);
                } else {
                    assert_eq!(rest.fill, [0.; 4]);
                    assert_eq!(rest.letter, [0.; 4]);
                    let value = rgb.into_iter().fold(0., f32::max);
                    let muted = rgb.map(|c| value - 0.8 * (value - c));
                    assert_eq!(rest.stroke, [muted[0], muted[1], muted[2], 0.55]);
                    let saturation = |v: [f32; 3]| {
                        let max = v.into_iter().fold(0., f32::max);
                        (max - v.into_iter().fold(1., f32::min)) / max
                    };
                    assert!((saturation(muted) - 0.8 * saturation(rgb)).abs() < 1e-6);
                    assert_eq!(hover.fill, [0.; 4]);
                    assert_eq!(hover.letter, [0.; 4]);
                    assert_eq!(hover.stroke, color);
                }
            }
        }
        assert_eq!(AXIS_COLORS, NAV_AXIS_COLORS);
        assert_eq!(AXIS_STYLE[0], [4., 7., 9., 1.]);
    }
    #[test]
    fn enlarged_circular_hits_include_hollow_centers_and_keep_signed_destinations() {
        let m = GridMetrics::from_k(2).unwrap();
        let r = PixelRect {
            x: 0.,
            y: 0.,
            width: 400.,
            height: 400.,
        };
        let f = unit([-1., 1., -1.]);
        let right = unit(cross(f, [0., 0., 1.]));
        let g = NavGeometry::new(
            layout_navigation(r, r, m, true, NavigatorKind::Gizmo, None),
            NavBasis {
                right,
                up: cross(right, f),
                forward: f,
            },
            m,
        )
        .unwrap();
        let t = g.layout.tile.unwrap();
        for (d, c) in g.caps() {
            let cap = AxisCap::from_direction(d).unwrap();
            let radius = if cap.positive {
                POSITIVE_DISC_RADIUS
            } else {
                NEGATIVE_RING_OUTER_RADIUS
            };
            let p = |r: f64| (t.x + c[0] + r, t.y + c[1]);
            assert_eq!(g.hit(p(0.)), Some(NavPart::Axis(cap)));
            assert_eq!(g.hit(p(radius - 1e-5)), Some(NavPart::Axis(cap)));
            assert_eq!(g.hit(p(radius + 1e-5)), None);
            assert_eq!(cap.direction(), d);
            assert!(g.reserves(p(0.)));
            assert!(!g.reserves(p(radius + 1e-5)));
        }
    }
}
