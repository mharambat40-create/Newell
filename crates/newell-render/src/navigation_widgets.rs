// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Dedicated navigation targets and immutable scene-independent drawing.
use crate::{
    RenderError,
    navigation_labels::{Labels, face_label, face_word_id},
};
use ab_glyph::FontVec;
use newell_ui::navigation_widgets::{
    AXIS_STYLE, AxisCap, CUBE_COLORS, NAV_AXIS_COLORS, NAV_TILE_SIZE_Q, NEGATIVE_RING_OUTER_RADIUS,
    NEGATIVE_RING_STROKE_WIDTH, NavGeometry, NavigatorKind, POSITIVE_DISC_RADIUS, PixelRect,
    ViewDirection, axis_marker_material, cube_patches,
};
use wgpu::util::DeviceExt;
#[derive(Debug, Clone, PartialEq)]
pub struct NavSnapshot {
    pub geometry: NavGeometry,
    pub ui_masks: Vec<PixelRect>,
    pub surface: [u32; 2],
    pub camera_revision: u64,
    pub hovered: Option<AxisCap>,
    pub hovered_cube: Option<ViewDirection>,
}
const COLOR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub struct NavOverlay {
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    sampler: wgpu::Sampler,
    cube: wgpu::RenderPipeline,
    axis: wgpu::RenderPipeline,
    label: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    labels: Option<(f64, Labels)>,
    target: Option<Target>,
    snapshot: Option<NavSnapshot>,
    cube_vertex: Option<wgpu::Buffer>,
    cube_count: u32,
    label_vertex: Option<wgpu::Buffer>,
    label_count: u32,
    bind: Option<wgpu::BindGroup>,
    composite_bind: Option<wgpu::BindGroup>,
    srgb: bool,
}
struct Target {
    size: [u32; 2],
    msaa: wgpu::TextureView,
    resolve: wgpu::TextureView,
    depth: wgpu::TextureView,
}
fn buffer(
    device: &wgpu::Device,
    name: &str,
    bytes: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(name),
        contents: if bytes.is_empty() { &[0; 16] } else { bytes },
        usage,
    })
}
fn floats(out: &mut Vec<u8>, values: impl IntoIterator<Item = f32>) {
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
}
fn ints(out: &mut Vec<u8>, values: impl IntoIterator<Item = u32>) {
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
}
fn texture(
    device: &wgpu::Device,
    size: [u32; 2],
    format: wgpu::TextureFormat,
    samples: u32,
    usage: wgpu::TextureUsages,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("navigation local target"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    entries: (&str, &str),
    format: wgpu::TextureFormat,
    vertices: &[wgpu::VertexBufferLayout<'_>],
    depth: bool,
) -> wgpu::RenderPipeline {
    let local = entries.0 != "vs_composite";
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(entries.0),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(entries.0),
            compilation_options: Default::default(),
            buffers: vertices,
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(entries.1),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: if depth { Some(wgpu::Face::Back) } else { None },
            ..Default::default()
        },
        depth_stencil: local.then_some(wgpu::DepthStencilState {
            format: DEPTH,
            depth_write_enabled: depth,
            depth_compare: if depth {
                wgpu::CompareFunction::Less
            } else {
                wgpu::CompareFunction::Always
            },
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: if local { 4 } else { 1 },
            ..Default::default()
        },
        multiview: None,
        cache: None,
    })
}
impl NavOverlay {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let both = wgpu::ShaderStages::VERTEX_FRAGMENT;
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: both,
            ty,
            count: None,
        };
        let storage = wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("navigation layout"),
            entries: &[
                entry(
                    0,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(128),
                    },
                ),
                entry(1, storage),
                entry(
                    2,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    3,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                ),
                entry(4, storage),
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("navigation pipelines"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("navigation_widgets.wgsl"));
        let axis_shader = device.create_shader_module(wgpu::include_wgsl!("navigation_axis.wgsl"));
        let cube = pipeline(
            device,
            &pl,
            &shader,
            ("vs_cube", "fs_cube"),
            COLOR,
            &[wgpu::VertexBufferLayout {
                array_stride: 48,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Uint32,2=>Float32x3,3=>Uint32,4=>Float32x4],
            }],
            true,
        );
        let axis = pipeline(
            device,
            &pl,
            &axis_shader,
            ("vs_axis", "fs_axis"),
            COLOR,
            &[],
            false,
        );
        let label = pipeline(
            device,
            &pl,
            &shader,
            ("vs_label", "fs_label"),
            COLOR,
            &[wgpu::VertexBufferLayout {
                array_stride: 96,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![0=>Float32x4,1=>Float32x4,2=>Float32x4,3=>Float32x4,4=>Uint32x4,5=>Float32x4],
            }],
            false,
        );
        let composite = pipeline(
            device,
            &pl,
            &shader,
            ("vs_composite", "fs_composite"),
            format,
            &[],
            false,
        );
        let uniform = buffer(
            device,
            "navigation uniforms",
            &[0; 128],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("navigation filtered coverage"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            layout,
            uniform,
            sampler,
            cube,
            axis,
            label,
            composite,
            labels: None,
            target: None,
            snapshot: None,
            cube_vertex: None,
            cube_count: 0,
            label_vertex: None,
            label_count: 0,
            bind: None,
            composite_bind: None,
            srgb: format.is_srgb(),
        }
    }
    /// Prepares coverage resources before a scale transition becomes live.
    pub fn prepare_metrics(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        font: &FontVec,
        metrics: newell_ui::GridMetrics,
    ) -> Result<(), RenderError> {
        let q = f64::from(metrics.cell_width_px()) / 8.;
        if self.labels.as_ref().is_none_or(|(old, _)| *old != q) {
            self.labels = Some((q, Labels::build(device, queue, font, q)?));
        }
        Ok(())
    }
    pub fn clear(&mut self) {
        self.snapshot = None;
        self.target = None;
        self.bind = None;
        self.composite_bind = None;
    }
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        font: &FontVec,
        snapshot: Option<NavSnapshot>,
    ) -> Result<(), RenderError> {
        let Some(s) = snapshot else {
            self.clear();
            return Ok(());
        };
        let Some(tile) = s.geometry.layout.tile else {
            self.clear();
            return Ok(());
        };
        if ![tile.x, tile.y, tile.width, tile.height]
            .iter()
            .all(|v| v.is_finite())
            || tile.x < 0.
            || tile.y < 0.
            || tile.width < 1.
            || tile.height < 1.
            || tile.width > f64::from(device.limits().max_texture_dimension_2d)
            || tile.height > f64::from(device.limits().max_texture_dimension_2d)
            || s.ui_masks.iter().any(|r| {
                ![r.x, r.y, r.width, r.height].iter().all(|v| v.is_finite())
                    || r.width < 0.
                    || r.height < 0.
            })
        {
            self.clear();
            return Err(RenderError::Navigation(
                "invalid physical navigation bounds".into(),
            ));
        }
        if !s.geometry.basis.valid()
            || !s.geometry.q.is_finite()
            || !(1.0..=3.5).contains(&s.geometry.q)
            || s.surface.contains(&0)
        {
            self.clear();
            return Err(RenderError::Navigation("invalid snapshot".into()));
        }
        if self.snapshot.as_ref() == Some(&s) {
            return Ok(());
        }
        if self.labels.as_ref().is_none_or(|(q, _)| *q != s.geometry.q) {
            self.labels = Some((
                s.geometry.q,
                Labels::build(device, queue, font, s.geometry.q)?,
            ));
        }
        let Some((_, labels)) = &self.labels else {
            return Err(RenderError::Navigation("missing prepared labels".into()));
        };
        let size = [tile.width.round() as u32, tile.height.round() as u32];
        if self.target.as_ref().is_none_or(|t| t.size != size) {
            self.target = Some(Target {
                size,
                msaa: texture(
                    device,
                    size,
                    COLOR,
                    4,
                    wgpu::TextureUsages::RENDER_ATTACHMENT,
                ),
                resolve: texture(
                    device,
                    size,
                    COLOR,
                    1,
                    wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                ),
                depth: texture(
                    device,
                    size,
                    DEPTH,
                    4,
                    wgpu::TextureUsages::RENDER_ATTACHMENT,
                ),
            });
        }
        let Some(target) = &self.target else {
            return Err(RenderError::Navigation("missing local target".into()));
        };
        let g = &s.geometry;
        let b = g.basis;
        let q = g.q as f32;
        let mut uniform = Vec::with_capacity(128);
        for v in [b.right, b.up, b.forward] {
            floats(&mut uniform, v.map(|v| v as f32));
            floats(&mut uniform, [0.]);
        }
        floats(
            &mut uniform,
            [tile.x as f32, tile.y as f32, size[0] as f32, size[1] as f32],
        );
        floats(&mut uniform, [48. * q, 48. * q, q, 31. * q]);
        floats(
            &mut uniform,
            [
                (NAV_TILE_SIZE_Q / 2.) as f32 * q,
                (NAV_TILE_SIZE_Q / 2.) as f32 * q,
                g.cube_projection_scale_q() as f32 * q,
                0.,
            ],
        );
        let hovered_patch = s
            .hovered_cube
            .and_then(|direction| {
                cube_patches()
                    .iter()
                    .position(|patch| patch.direction == direction)
                    .map(|id| id as u32 + 1)
            })
            .unwrap_or(0);
        ints(
            &mut uniform,
            [
                hovered_patch,
                u32::from(g.layout.kind == NavigatorKind::Cube),
                u32::from(self.srgb),
                s.ui_masks.len() as u32,
            ],
        );
        floats(
            &mut uniform,
            [s.surface[0] as f32, s.surface[1] as f32, 512., 512.],
        );
        queue.write_buffer(&self.uniform, 0, &uniform);
        let mut style = Vec::new();
        for color in NAV_AXIS_COLORS.into_iter().chain(CUBE_COLORS).chain([
            [201. / 255., 205. / 255., 210. / 255.],
            [32. / 255., 37. / 255., 43. / 255.],
            [221. / 255., 229. / 255., 235. / 255.],
            [0.; 3],
            [0.; 3],
            [0.; 3],
        ]) {
            floats(&mut style, color);
            floats(&mut style, [1.]);
        }
        for word in &labels.words[6..] {
            floats(&mut style, word.uv);
        }
        for word in &labels.words[6..] {
            floats(&mut style, [word.size[0], word.size[1], 0., 0.]);
        }
        for (d, _) in g.caps() {
            floats(&mut style, d.components().map(f32::from));
            floats(&mut style, [0.]);
        }
        let mut corner_style = AXIS_STYLE;
        corner_style[0][1] = POSITIVE_DISC_RADIUS as f32;
        corner_style[0][2] = (NEGATIVE_RING_OUTER_RADIUS - NEGATIVE_RING_STROKE_WIDTH / 2.) as f32;
        for lane in corner_style {
            floats(&mut style, lane);
        }
        for (direction, _) in g.caps() {
            let cap = AxisCap::from_direction(direction)
                .ok_or_else(|| RenderError::Navigation("invalid cap direction".into()))?;
            let material = axis_marker_material(cap, s.hovered == Some(cap));
            for lane in [material.fill, material.stroke, material.letter] {
                floats(&mut style, lane);
            }
        }
        let style = buffer(
            device,
            "navigation immutable materials",
            &style,
            wgpu::BufferUsages::STORAGE,
        );
        let patches = cube_patches();
        let mut vertices = Vec::new();
        let mut labels_bytes = Vec::new();
        let mut clip_bytes = Vec::new();
        self.cube_count = 0;
        self.label_count = 0;
        if g.layout.kind == NavigatorKind::Cube {
            for (id, patch) in patches.iter().enumerate() {
                for i in 1..patch.vertices.len().saturating_sub(1) {
                    for v in [patch.vertices[0], patch.vertices[i], patch.vertices[i + 1]] {
                        floats(&mut vertices, v.map(|x| x as f32));
                        ints(&mut vertices, [id as u32]);
                        floats(&mut vertices, patch.normal.map(|x| x as f32));
                        ints(
                            &mut vertices,
                            patch.direction.dimensions() as u32 - 1
                                ..patch.direction.dimensions() as u32,
                        );
                        floats(&mut vertices, [0.; 4]);
                        self.cube_count += 1;
                    }
                }
                let Some(word_id) = face_word_id(patch.direction.components()) else {
                    continue;
                };
                let word = labels.words[word_id];
                let Some(label) = face_label(patch, word, b, q, g.cube_projection_scale_q() as f32)
                else {
                    continue;
                };
                let poly = patch
                    .vertices
                    .iter()
                    .map(|v| g.project(*v))
                    .collect::<Vec<_>>();
                floats(&mut labels_bytes, label.origin.map(|v| v as f32));
                floats(&mut labels_bytes, [0.]);
                floats(&mut labels_bytes, label.right.map(|v| v as f32));
                floats(&mut labels_bytes, [label.size[0]]);
                floats(&mut labels_bytes, label.up.map(|v| v as f32));
                floats(&mut labels_bytes, [label.size[1]]);
                floats(&mut labels_bytes, word.uv);
                ints(
                    &mut labels_bytes,
                    [
                        (clip_bytes.len() / 16) as u32,
                        poly.len() as u32,
                        label.word_id as u32,
                        0,
                    ],
                );
                floats(&mut labels_bytes, [37. / 255., 42. / 255., 49. / 255., 1.]);
                for p in poly {
                    floats(&mut clip_bytes, [p[0] as f32, p[1] as f32, 0., 0.]);
                }
                self.label_count += 1;
            }
        }
        self.cube_vertex = Some(buffer(
            device,
            "navigation cube vertices",
            &vertices,
            wgpu::BufferUsages::VERTEX,
        ));
        self.label_vertex = Some(buffer(
            device,
            "navigation label instances",
            &labels_bytes,
            wgpu::BufferUsages::VERTEX,
        ));
        let clips = buffer(
            device,
            "navigation face clips",
            &clip_bytes,
            wgpu::BufferUsages::STORAGE,
        );
        let mut masks = Vec::new();
        for r in &s.ui_masks {
            floats(
                &mut masks,
                [
                    r.x as f32,
                    r.y as f32,
                    (r.x + r.width) as f32,
                    (r.y + r.height) as f32,
                ],
            );
        }
        let masks = buffer(
            device,
            "navigation ui masks",
            &masks,
            wgpu::BufferUsages::STORAGE,
        );
        let bind = |view: &wgpu::TextureView, clips: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("navigation immutable bindings"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: style.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: clips.as_entire_binding(),
                    },
                ],
            })
        };
        self.bind = Some(bind(&labels.view, &clips));
        self.composite_bind = Some(bind(&target.resolve, &masks));
        self.snapshot = Some(s);
        Ok(())
    }
    /// Records all dedicated navigation work; count includes actual composite invocation.
    pub fn draw(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) -> u32 {
        let (Some(s), Some(t), Some(bind), Some(composite)) = (
            &self.snapshot,
            &self.target,
            &self.bind,
            &self.composite_bind,
        ) else {
            return 0;
        };
        let mut count = 0;
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("navigation local"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.msaa,
                    depth_slice: None,
                    resolve_target: Some(&t.resolve),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_bind_group(0, bind, &[]);
            if s.geometry.layout.kind == NavigatorKind::Gizmo {
                pass.set_pipeline(&self.axis);
                pass.draw(0..6, 0..1);
                count += 1;
            } else {
                if let Some(v) = &self.cube_vertex {
                    pass.set_pipeline(&self.cube);
                    pass.set_vertex_buffer(0, v.slice(..));
                    pass.draw(0..self.cube_count, 0..1);
                    count += 1;
                }
                if self.label_count > 0
                    && let Some(v) = &self.label_vertex
                {
                    pass.set_pipeline(&self.label);
                    pass.set_vertex_buffer(0, v.slice(..));
                    pass.draw(0..6, 0..self.label_count);
                    count += 1;
                }
            }
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("navigation composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.composite);
        pass.set_bind_group(0, composite, &[]);
        pass.draw(0..6, 0..1);
        count + 1
    }
}
