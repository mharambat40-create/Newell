// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Deterministic navigation-only word atlas, independent of ASCII glyph cells.
use crate::RenderError;
use ab_glyph::{Font, FontVec, PxScale, ScaleFont, point};
pub const WORDS: [&str; 9] = [
    "Top", "Bottom", "Front", "Back", "Left", "Right", "X", "Y", "Z",
];
pub(crate) const FACE_LABEL_SCALE: f32 = 0.85;
#[derive(Debug, Clone, Copy, Default)]
pub struct Word {
    pub uv: [f32; 4],
    pub size: [f32; 2],
    pub cap: f32,
}
pub struct Labels {
    pub view: wgpu::TextureView,
    pub words: [Word; 9],
}
impl Labels {
    pub fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        font: &FontVec,
        q: f64,
    ) -> Result<Self, RenderError> {
        let mut pixels = vec![0u8; 512 * 512];
        let mut words = [Word::default(); 9];
        let mut x = 2usize;
        let mut y = 2usize;
        let mut shelf = 0usize;
        for (i, text) in WORDS.iter().enumerate() {
            let cap = (if i < 6 { 9. * FACE_LABEL_SCALE } else { 8. }) * q as f32;
            let scaled = font.as_scaled(PxScale::from(cap * font.height_unscaled() / 1377.));
            let mut pen = 0.;
            let mut outlines = Vec::new();
            for ch in text.chars() {
                let id = scaled.glyph_id(ch);
                if id.0 == 0 {
                    return Err(RenderError::MissingGlyph(ch));
                }
                let glyph = id.with_scale_and_position(scaled.scale(), point(pen, 0.));
                pen += scaled.h_advance(id);
                if let Some(outline) = font.outline_glyph(glyph) {
                    outlines.push(outline);
                }
            }
            let left = outlines
                .iter()
                .map(|g| g.px_bounds().min.x)
                .fold(f32::INFINITY, f32::min)
                .floor();
            let top = outlines
                .iter()
                .map(|g| g.px_bounds().min.y)
                .fold(f32::INFINITY, f32::min)
                .floor();
            let right = outlines
                .iter()
                .map(|g| g.px_bounds().max.x)
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil();
            let bottom = outlines
                .iter()
                .map(|g| g.px_bounds().max.y)
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil();
            let w = (right - left) as usize;
            let h = (bottom - top) as usize;
            if x + w + 2 > 512 {
                x = 2;
                y += shelf + 4;
                shelf = 0;
            }
            if y + h + 2 > 512 || w == 0 || h == 0 {
                return Err(RenderError::Navigation(
                    "label atlas exceeds fixed bounds".into(),
                ));
            }
            for outline in outlines {
                let b = outline.px_bounds();
                outline.draw(|gx, gy, c| {
                    let px = x + (b.min.x - left) as usize + gx as usize;
                    let py = y + (b.min.y - top) as usize + gy as usize;
                    if px < 512 && py < 512 {
                        pixels[py * 512 + px] =
                            ((c * 255.).round() as u8).max(pixels[py * 512 + px]);
                    }
                });
            }
            words[i] = Word {
                uv: [
                    x as f32 / 512.,
                    y as f32 / 512.,
                    w as f32 / 512.,
                    h as f32 / 512.,
                ],
                size: [w as f32, h as f32],
                cap,
            };
            x += w + 4;
            shelf = shelf.max(h);
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("navigation words"),
            size: wgpu::Extent3d {
                width: 512,
                height: 512,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(512),
                rows_per_image: Some(512),
            },
            texture.size(),
        );
        Ok(Self {
            view: texture.create_view(&Default::default()),
            words,
        })
    }
}

use newell_ui::navigation_widgets::{CubePatch, NavBasis, V3, add, cross, dot, mul};
/// Stable cube-local ink rectangle. The View projects this with the cube itself.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FaceLabel {
    pub origin: V3,
    pub right: V3,
    pub up: V3,
    pub size: [f32; 2],
    pub word_id: usize,
}
pub(crate) fn face_word_id(normal: [i8; 3]) -> Option<usize> {
    match normal {
        [0, 0, 1] => Some(0),
        [0, 0, -1] => Some(1),
        [0, -1, 0] => Some(2),
        [0, 1, 0] => Some(3),
        [-1, 0, 0] => Some(4),
        [1, 0, 0] => Some(5),
        _ => None,
    }
}
pub(crate) fn face_label(
    patch: &CubePatch,
    word: Word,
    basis: NavBasis,
    q: f32,
    projection_scale_q: f32,
) -> Option<FaceLabel> {
    let word_id = face_word_id(patch.direction.components())?;
    if dot(patch.normal, basis.forward) >= -1e-9 || q <= 0. || word.cap <= 0. {
        return None;
    }
    let normal = patch.normal;
    let up = if normal[2] > 0.5 {
        [0., 1., 0.]
    } else if normal[2] < -0.5 {
        [0., -1., 0.]
    } else {
        [0., 0., 1.]
    };
    let right = cross(up, normal);
    let origin = mul(normal, 0.5);
    let poly = patch
        .vertices
        .iter()
        .map(|v| {
            let v = add(*v, mul(origin, -1.));
            [dot(v, right) as f32, dot(v, up) as f32]
        })
        .collect::<Vec<_>>();
    let size = word.size.map(|v| v / (projection_scale_q * q));
    let previous_size = size.map(|v| v / FACE_LABEL_SCALE);
    let fits = |factor: f32| {
        [-1., 1.].into_iter().all(|x| {
            [-1., 1.].into_iter().all(|y| {
                inside(
                    &poly,
                    [
                        x * previous_size[0] * factor / 2.,
                        y * previous_size[1] * factor / 2.,
                    ],
                    2. / projection_scale_q,
                )
            })
        })
    };
    let mut lo = 6. * q / (word.cap / FACE_LABEL_SCALE);
    let mut hi = 1.;
    if lo > hi || !fits(lo) {
        return None;
    }
    for _ in 0..16 {
        let mid = (lo + hi) / 2.;
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(FaceLabel {
        origin,
        right,
        up,
        size: size.map(|v| v * lo),
        word_id,
    })
}
fn inside(poly: &[[f32; 2]], p: [f32; 2], margin: f32) -> bool {
    let mut sign = 0.;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let d = [b[0] - a[0], b[1] - a[1]];
        let area = d[0] * (p[1] - a[1]) - d[1] * (p[0] - a[0]);
        let len = d[0].hypot(d[1]);
        if len < 1e-6 {
            continue;
        }
        if area.abs() < margin * len {
            return false;
        }
        if sign == 0. {
            sign = area.signum();
        } else if sign * area < 0. {
            return false;
        }
    }
    sign != 0.
}
#[cfg(test)]
mod tests {
    use super::*;
    use newell_ui::navigation_widgets::{cube_patches, cube_planes, unit};
    fn basis(n: V3) -> NavBasis {
        let f = mul(unit(n), -1.);
        let heading = if f[0].abs() + f[1].abs() < 1e-12 {
            [0., 1., 0.]
        } else {
            unit([f[0], f[1], 0.])
        };
        let right = unit(cross(heading, [0., 0., 1.]));
        NavBasis {
            right,
            up: cross(right, f),
            forward: f,
        }
    }
    #[test]
    fn labels_have_stable_face_axes_and_every_ink_corner_is_on_its_primary_face() {
        for (normal, expected_up) in [
            ([0, 0, 1], [0., 1., 0.]),
            ([0, 0, -1], [0., -1., 0.]),
            ([0, -1, 0], [0., 0., 1.]),
            ([0, 1, 0], [0., 0., 1.]),
            ([-1, 0, 0], [0., 0., 1.]),
            ([1, 0, 0], [0., 0., 1.]),
        ] {
            let patch = cube_patches()
                .iter()
                .find(|p| p.direction.components() == normal)
                .unwrap();
            let word = Word {
                size: [47., 9.],
                cap: 9.,
                ..Default::default()
            };
            let label = face_label(patch, word, basis(patch.normal), 1., 45.).unwrap();
            assert_eq!(label.up, expected_up);
            assert_eq!(cross(label.right, label.up), patch.normal);
            for x in [-0.5, 0.5] {
                for y in [-0.5, 0.5] {
                    let corner = add(
                        label.origin,
                        add(
                            mul(label.right, x * f64::from(label.size[0])),
                            mul(label.up, y * f64::from(label.size[1])),
                        ),
                    );
                    assert!((dot(corner, patch.normal) - 0.5).abs() < 1e-12);
                    for (_, n, k) in cube_planes() {
                        assert!(dot(corner, n) <= k + 1e-9);
                    }
                }
            }
            assert!(face_label(patch, word, basis(mul(patch.normal, -1.)), 1., 45.).is_none());
        }
    }
    #[test]
    fn oblique_projection_foreshortens_label_without_changing_its_local_orientation() {
        let patch = cube_patches()
            .iter()
            .find(|p| p.direction.components() == [0, -1, 0])
            .unwrap();
        let word = Word {
            size: [35., 9.],
            cap: 9.,
            ..Default::default()
        };
        let a = face_label(patch, word, basis([0., -1., 0.]), 1., 45.).unwrap();
        let oblique = basis([1., -1., 1.]);
        let b = face_label(patch, word, oblique, 1., 45.).unwrap();
        assert_eq!(a.right, b.right);
        assert_eq!(a.up, b.up);
        assert_eq!(a.size, b.size);
        let projected = oblique.project(mul(b.right, f64::from(b.size[0])));
        assert!(projected[0].hypot(projected[1]) < f64::from(a.size[0]) * 0.9);
        let edge = basis([1., 0., 0.]);
        assert!(face_label(patch, word, edge, 1., 45.).is_none());
    }

    #[test]
    fn cube_face_labels_are_fifteen_percent_smaller_at_every_ui_scale() {
        let patch = cube_patches()
            .iter()
            .find(|p| p.direction.components() == [0, 0, 1])
            .unwrap();
        for q in [1., 2., 3.5] {
            let word = Word {
                size: [12. * q * FACE_LABEL_SCALE, 6. * q * FACE_LABEL_SCALE],
                cap: 9. * q * FACE_LABEL_SCALE,
                ..Default::default()
            };
            let scale = 45.;
            let label = face_label(patch, word, basis(patch.normal), q, scale).unwrap();
            let prior_width = word.size[0] / (scale * q * FACE_LABEL_SCALE);
            let prior_height = word.size[1] / (scale * q * FACE_LABEL_SCALE);
            assert!(
                (label.size[0] - prior_width * FACE_LABEL_SCALE).abs() < 3e-6,
                "q={q} label={} expected={}",
                label.size[0],
                prior_width * FACE_LABEL_SCALE
            );
            assert!((label.size[1] - prior_height * FACE_LABEL_SCALE).abs() < 3e-6);
            assert_eq!(label.origin, mul(patch.normal, 0.5));
            assert_eq!(label.up, [0., 1., 0.]);
            assert_eq!(label.word_id, 0);
        }
    }
}
