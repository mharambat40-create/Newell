// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! GPU side of the ASCII overlay: atlas texture, sampler, cell buffer, pipeline, one draw.

use newell_ui::CellGrid;
use wgpu::util::DeviceExt;

use super::gpu_cell::{GPU_CELL_BYTES, cells_to_bytes};
use super::theme::Theme;
use crate::glyph_atlas::GlyphAtlas;

const UNIFORM_BYTES: usize = 64;
/// Vertices per cell quad (two triangles).
const QUAD_VERTICES: u32 = 6;

/// One atlas with its GPU texture, bound to the shared uniforms and sampler.
struct AtlasSlot {
    atlas: GlyphAtlas,
    bind_group: wgpu::BindGroup,
}

impl AtlasSlot {
    fn size(&self) -> (u32, u32) {
        let layout = self.atlas.layout();
        (layout.cell_width, layout.cell_height)
    }
}

/// Draws a whole `CellGrid` with one instanced draw call: one instance per cell, anchored at the
/// top-left of the target, in whole cells of the atlas' pixel size (one atlas texel per screen pixel).
pub struct UiOverlay {
    /// Every atlas built so far, one per cell size, reused when a size returns.
    atlases: Vec<AtlasSlot>,
    /// Index into `atlases` of the atlas the grid is drawn with.
    active: usize,
    pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniforms: wgpu::Buffer,
    cells: wgpu::Buffer,
    cell_count: u32,
    /// Cell size of the atlas the `cells` buffer was built with. A frame is drawn only if it is
    /// the active atlas' size, so cells and atlas can never be mismatched.
    cells_size: Option<(u32, u32)>,
    cols: u32,
    viewport_px: (u32, u32),
    srgb_target: bool,
    theme: Theme,
}

impl UiOverlay {
    /// `format` is the colour format of the pass the overlay is drawn into.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        atlas: GlyphAtlas,
    ) -> Self {
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
        let slot = Self::atlas_slot(device, queue, &bind_layout, &uniforms, &sampler, atlas);

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
                    attributes: &wgpu::vertex_attr_array![0 => Uint32],
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
            atlases: vec![slot],
            active: 0,
            pipeline,
            bind_layout,
            sampler,
            uniforms,
            cells: Self::cell_buffer(device, GPU_CELL_BYTES as u64),
            cell_count: 0,
            cells_size: None,
            cols: 0,
            viewport_px: (1, 1),
            srgb_target: format.is_srgb(),
            theme: Theme::default(),
        }
    }

    /// Uploads `atlas` as a texture and binds it with the shared uniforms and sampler.
    fn atlas_slot(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        uniforms: &wgpu::Buffer,
        sampler: &wgpu::Sampler,
        atlas: GlyphAtlas,
    ) -> AtlasSlot {
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui bind group"),
            layout,
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
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        AtlasSlot { atlas, bind_group }
    }

    /// The cell size `(W, H)` of the active atlas.
    pub fn cell_size(&self) -> (u32, u32) {
        self.atlases[self.active].size()
    }

    /// Number of atlases held (one per cell size used so far).
    pub fn atlas_count(&self) -> usize {
        self.atlases.len()
    }

    /// `true` if an atlas for the cell size `size` is already held.
    pub fn has_atlas(&self, size: (u32, u32)) -> bool {
        self.atlases.iter().any(|slot| slot.size() == size)
    }

    /// Adds `atlas` to the cache unless one for its cell size exists. Does not change the active
    /// atlas: the caller activates it together with a grid built for it ([`Self::activate`]).
    pub fn add_atlas(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, atlas: GlyphAtlas) {
        let layout = atlas.layout();
        if !self.has_atlas((layout.cell_width, layout.cell_height)) {
            let slot = Self::atlas_slot(
                device,
                queue,
                &self.bind_layout,
                &self.uniforms,
                &self.sampler,
                atlas,
            );
            self.atlases.push(slot);
        }
    }

    /// Makes the cached atlas of cell size `size` the active one and uploads `grid` built for
    /// it in the same call, so the cells, the atlas and the uniforms always agree. Returns
    /// `false` and changes nothing if no such atlas is held.
    pub fn activate(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: (u32, u32),
        grid: &CellGrid,
    ) -> bool {
        let Some(index) = self.atlases.iter().position(|slot| slot.size() == size) else {
            return false;
        };
        self.active = index;
        self.set_grid(device, queue, grid);
        true
    }

    fn cell_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui cells"),
            size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Uploads the whole grid. A cell is drawn at the atlas' cell size in physical pixels.
    pub fn set_grid(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, grid: &CellGrid) {
        let bytes = cells_to_bytes(grid, &self.atlases[self.active].atlas);
        if !bytes.is_empty() {
            if bytes.len() as u64 > self.cells.size() {
                self.cells = Self::cell_buffer(device, bytes.len() as u64);
            }
            queue.write_buffer(&self.cells, 0, &bytes);
        }
        self.cell_count = u32::try_from(grid.cells().len()).unwrap_or(0);
        self.cells_size = Some(self.cell_size());
        self.cols = grid.cols();
        self.write_uniforms(queue);
    }

    /// Render target size in physical pixels.
    pub fn set_viewport(&mut self, queue: &wgpu::Queue, width: u32, height: u32) {
        self.viewport_px = (width.max(1), height.max(1));
        self.write_uniforms(queue);
    }

    /// Sets the theme. Rewrites only the small uniform buffer: no cell upload, no extra draw.
    pub fn set_theme(&mut self, queue: &wgpu::Queue, theme: Theme) {
        if self.theme != theme {
            self.theme = theme;
            self.write_uniforms(queue);
        }
    }

    fn write_uniforms(&self, queue: &wgpu::Queue) {
        let atlas = &self.atlases[self.active].atlas;
        let layout = atlas.layout();
        let (cell_w, cell_h) = (layout.cell_width as f32, layout.cell_height as f32);
        let floats = [
            self.viewport_px.0 as f32,
            self.viewport_px.1 as f32,
            cell_w,
            cell_h,
            cell_w,
            cell_h,
            atlas.width() as f32,
            atlas.height() as f32,
        ];
        let mut bytes = Vec::with_capacity(UNIFORM_BYTES);
        for value in floats {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&self.cols.to_le_bytes());
        bytes.extend_from_slice(&u32::from(self.srgb_target).to_le_bytes());
        // Pads `cols` / `srgb_target` to the 16-byte alignment of the colour vector.
        bytes.extend_from_slice(&[0; 8]);
        for c in self.theme.foreground() {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
        bytes.extend_from_slice(&1.0f32.to_le_bytes());
        queue.write_buffer(&self.uniforms, 0, &bytes);
    }

    /// Records the overlay into `pass`: exactly one draw call for the whole grid, none for an
    /// empty grid. Returns the number of draw calls recorded.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) -> u32 {
        if self.cell_count == 0 || self.cells_size != Some(self.cell_size()) {
            return 0;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.atlases[self.active].bind_group, &[]);
        pass.set_vertex_buffer(0, self.cells.slice(..));
        pass.draw(0..QUAD_VERTICES, 0..self.cell_count);
        1
    }
}
