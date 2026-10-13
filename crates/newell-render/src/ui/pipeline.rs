// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! GPU side of the ASCII overlay: atlas texture, sampler, cell buffer, pipeline, one draw.

use newell_ui::{CellGrid, GridMetrics};
use wgpu::util::DeviceExt;

use super::gpu_cell::{GPU_CELL_BYTES, cells_to_bytes};
use crate::glyph_atlas::{GLYPH_CELL_HEIGHT, GLYPH_CELL_WIDTH, GlyphAtlas};

const UNIFORM_BYTES: usize = 40;
/// Vertices per cell quad (two triangles).
const QUAD_VERTICES: u32 = 6;

/// Draws a whole `CellGrid` with one instanced draw call: one instance per cell, anchored at the
/// top-left of the target, in whole cells of `GridMetrics` pixels.
pub struct UiOverlay {
    atlas: GlyphAtlas,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniforms: wgpu::Buffer,
    cells: wgpu::Buffer,
    cell_count: u32,
    cols: u32,
    cell_px: (u32, u32),
    viewport_px: (u32, u32),
    srgb_target: bool,
}

impl UiOverlay {
    /// `format` is the colour format of the pass the overlay is drawn into.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        atlas: GlyphAtlas,
    ) -> Self {
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("ui glyph atlas"),
                size: wgpu::Extent3d {
                    width: atlas.width(),
                    height: atlas.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            atlas.coverage(),
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // The atlas has no padding between slots: nearest only, never linear.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ui atlas sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui uniforms"),
            size: UNIFORM_BYTES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let layout_entry = |binding, visibility, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty,
            count: None,
        };
        let both = wgpu::ShaderStages::VERTEX_FRAGMENT;
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui bind layout"),
            entries: &[
                layout_entry(
                    0,
                    both,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(UNIFORM_BYTES as u64),
                    },
                ),
                layout_entry(
                    1,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                layout_entry(
                    2,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                ),
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui bind group"),
            layout: &bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui pipeline layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: GPU_CELL_BYTES as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Uint32, 1 => Uint32],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            atlas,
            pipeline,
            bind_group,
            uniforms,
            cells: Self::cell_buffer(device, GPU_CELL_BYTES as u64),
            cell_count: 0,
            cols: 0,
            cell_px: (GLYPH_CELL_WIDTH, GLYPH_CELL_HEIGHT),
            viewport_px: (1, 1),
            srgb_target: format.is_srgb(),
        }
    }

    fn cell_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui cells"),
            size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Uploads the whole grid. `metrics` is the on-screen size of one cell in physical pixels.
    pub fn set_grid(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        grid: &CellGrid,
        metrics: GridMetrics,
    ) {
        let bytes = cells_to_bytes(grid, &self.atlas);
        if !bytes.is_empty() {
            if bytes.len() as u64 > self.cells.size() {
                self.cells = Self::cell_buffer(device, bytes.len() as u64);
            }
            queue.write_buffer(&self.cells, 0, &bytes);
        }
        self.cell_count = u32::try_from(grid.cells().len()).unwrap_or(0);
        self.cols = grid.cols();
        self.cell_px = (metrics.cell_width_px(), metrics.cell_height_px());
        self.write_uniforms(queue);
    }

    /// Render target size in physical pixels.
    pub fn set_viewport(&mut self, queue: &wgpu::Queue, width: u32, height: u32) {
        self.viewport_px = (width.max(1), height.max(1));
        self.write_uniforms(queue);
    }

    fn write_uniforms(&self, queue: &wgpu::Queue) {
        let floats = [
            self.viewport_px.0 as f32,
            self.viewport_px.1 as f32,
            self.cell_px.0 as f32,
            self.cell_px.1 as f32,
            GLYPH_CELL_WIDTH as f32,
            GLYPH_CELL_HEIGHT as f32,
            self.atlas.width() as f32,
            self.atlas.height() as f32,
        ];
        let mut bytes = Vec::with_capacity(UNIFORM_BYTES);
        for value in floats {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&self.cols.to_le_bytes());
        bytes.extend_from_slice(&u32::from(self.srgb_target).to_le_bytes());
        queue.write_buffer(&self.uniforms, 0, &bytes);
    }

    /// Records the overlay into `pass`: exactly one draw call for the whole grid, none for an
    /// empty grid. Returns the number of draw calls recorded.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) -> u32 {
        if self.cell_count == 0 {
            return 0;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.cells.slice(..));
        pass.draw(0..QUAD_VERTICES, 0..self.cell_count);
        1
    }
}
