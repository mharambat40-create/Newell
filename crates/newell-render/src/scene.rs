// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Scene mesh upload and immutable camera-uniform consumption for M3.

use crate::{CameraSnapshot, SceneViewport, exposed_regions};
use newell_kernel::NewellMesh;
use std::collections::BTreeMap;
use wgpu::util::DeviceExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum SceneDisplayMode {
    Points = 0,
    Line = 1,
    #[default]
    Wireframe = 2,
    HiddenLine = 3,
    NoShading = 4,
    Shaded = 5,
    Default = 6,
}

impl SceneDisplayMode {
    const fn shader_id(self) -> u32 {
        self as u32
    }

    const fn geometry_lines(self) -> bool {
        matches!(self, Self::Line | Self::HiddenLine | Self::Default)
    }
}

/// GPU scene resources. Navigation and CPU geometric queries belong to the Controller.
pub(crate) struct Scene {
    pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    depth_pipeline: wgpu::RenderPipeline,
    visible_pipeline: wgpu::RenderPipeline,
    hidden_pipeline: wgpu::RenderPipeline,
    point_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    line_layout: wgpu::BindGroupLayout,
    line_bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    vertex: Option<wgpu::Buffer>,
    index: Option<wgpu::Buffer>,
    edge: Option<wgpu::Buffer>,
    point_vertex: Option<wgpu::Buffer>,
    geometry_vertex: Option<wgpu::Buffer>,
    mesh: Option<NewellMesh>,
    index_count: u32,
    edge_count: u32,
    point_count: u32,
    geometry_count: u32,
    depth: wgpu::TextureView,
    target_size: (u32, u32),
    viewport: SceneViewport,
    occlusions: Vec<SceneViewport>,
    camera: CameraSnapshot,
    display_mode: SceneDisplayMode,
    selected: bool,
}

impl Scene {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        size: (u32, u32),
    ) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene camera uniform"),
            size: 112,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene bind layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(112),
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene bind group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("scene.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene pipeline layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = scene_pipeline(
            device,
            &pipeline_layout,
            &shader,
            format,
            wgpu::PrimitiveTopology::TriangleList,
            true,
        );
        let line_pipeline = scene_pipeline(
            device,
            &pipeline_layout,
            &shader,
            format,
            wgpu::PrimitiveTopology::LineList,
            false,
        );
        let depth_pipeline = depth_pipeline(device, &pipeline_layout, &shader);
        let line_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("geometry line bind layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(112),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let line_shader = device.create_shader_module(wgpu::include_wgsl!("scene_lines.wgsl"));
        let line_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("geometry line pipeline layout"),
            bind_group_layouts: &[&line_layout],
            push_constant_ranges: &[],
        });
        let visible_pipeline = geometry_line_pipeline(
            device,
            &line_pipeline_layout,
            &line_shader,
            format,
            "fs_visible",
        );
        let hidden_pipeline = geometry_line_pipeline(
            device,
            &line_pipeline_layout,
            &line_shader,
            format,
            "fs_hidden",
        );
        let point_shader = device.create_shader_module(wgpu::include_wgsl!("scene_points.wgsl"));
        let point_pipeline = scene_pipeline(
            device,
            &pipeline_layout,
            &point_shader,
            format,
            wgpu::PrimitiveTopology::TriangleList,
            false,
        );
        let target_size = (size.0.max(1), size.1.max(1));
        let depth = depth_view(device, target_size);
        let line_bind_group = geometry_line_bind_group(device, &line_layout, &uniform, &depth);
        Self {
            pipeline,
            line_pipeline,
            depth_pipeline,
            visible_pipeline,
            hidden_pipeline,
            point_pipeline,
            bind_group,
            line_layout,
            line_bind_group,
            uniform,
            vertex: None,
            index: None,
            edge: None,
            point_vertex: None,
            geometry_vertex: None,
            mesh: None,
            index_count: 0,
            edge_count: 0,
            point_count: 0,
            geometry_count: 0,
            depth,
            target_size,
            viewport: SceneViewport::default(),
            occlusions: Vec::new(),
            camera: CameraSnapshot::default(),
            display_mode: SceneDisplayMode::default(),
            selected: false,
        }
    }

    pub(crate) fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if size != self.target_size {
            self.target_size = size;
            self.depth = depth_view(device, size);
            self.line_bind_group =
                geometry_line_bind_group(device, &self.line_layout, &self.uniform, &self.depth);
        }
    }

    pub(crate) fn set_viewport(&mut self, device: &wgpu::Device, viewport: SceneViewport) {
        self.viewport = viewport;
        self.rebuild_geometry_lines(device);
    }

    pub(crate) fn set_occlusions(&mut self, occlusions: Vec<SceneViewport>) {
        self.occlusions = occlusions;
    }

    pub(crate) fn set_camera(&mut self, device: &wgpu::Device, camera: CameraSnapshot) {
        self.camera = camera;
        self.rebuild_geometry_lines(device);
    }

    pub(crate) fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }

    pub(crate) fn set_display_mode(&mut self, mode: SceneDisplayMode) {
        self.display_mode = mode;
    }

    pub(crate) fn set_mesh(&mut self, device: &wgpu::Device, mesh: NewellMesh) {
        let mut vertices = Vec::with_capacity(mesh.positions().len() * 24);
        for (position, normal) in mesh.positions().iter().zip(mesh.normals()) {
            for component in position.iter().chain(normal) {
                vertices.extend_from_slice(&component.to_ne_bytes());
            }
        }
        let mut indices = Vec::with_capacity(mesh.indices().len() * 4);
        for index in mesh.indices() {
            indices.extend_from_slice(&index.to_ne_bytes());
        }
        let mut edges = Vec::with_capacity(mesh.indices().len() * 2);
        for triangle in mesh.indices().as_chunks::<3>().0 {
            for edge in [
                [triangle[0], triangle[1]],
                [triangle[1], triangle[2]],
                [triangle[2], triangle[0]],
            ] {
                for index in edge {
                    edges.extend_from_slice(&index.to_ne_bytes());
                }
            }
        }
        let mut points = Vec::with_capacity(mesh.positions().len() * 6 * 24);
        for position in mesh.positions() {
            for corner in [
                [-1.0_f32, -1.0],
                [1.0, -1.0],
                [1.0, 1.0],
                [-1.0, -1.0],
                [1.0, 1.0],
                [-1.0, 1.0],
            ] {
                for value in position {
                    points.extend_from_slice(&value.to_ne_bytes());
                }
                for value in corner {
                    points.extend_from_slice(&value.to_ne_bytes());
                }
                points.extend_from_slice(&0.0_f32.to_ne_bytes());
            }
        }
        self.vertex = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("scene vertices"),
                contents: &vertices,
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );
        self.index = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("scene indices"),
                contents: &indices,
                usage: wgpu::BufferUsages::INDEX,
            }),
        );
        self.edge = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("scene edges"),
                contents: &edges,
                usage: wgpu::BufferUsages::INDEX,
            }),
        );
        self.point_vertex = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("scene point quads"),
                contents: &points,
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );
        self.index_count = u32::try_from(mesh.indices().len()).unwrap_or(u32::MAX);
        self.edge_count = u32::try_from(edges.len() / 4).unwrap_or(u32::MAX);
        self.point_count = u32::try_from(points.len() / 24).unwrap_or(u32::MAX);
        self.mesh = Some(mesh);
        self.rebuild_geometry_lines(device);
    }

    fn rebuild_geometry_lines(&mut self, device: &wgpu::Device) {
        let Some(mesh) = &self.mesh else { return };
        let mut vertices = Vec::<f32>::new();
        let feature = mesh.feature_edges();
        let mut feature_segments = Vec::new();
        for pair in feature.offsets().windows(2) {
            let Some(line) = feature.positions().get(pair[0] as usize..pair[1] as usize) else {
                continue;
            };
            append_polyline(&mut vertices, line, self.camera, self.viewport);
            feature_segments.extend(line.windows(2).map(|segment| (segment[0], segment[1])));
        }

        // Triangle adjacency supplies view-dependent contours on smooth faces.
        // It never supplies ordinary interior mesh edges to these modes.
        let mut uses: BTreeMap<(u32, u32), Vec<bool>> = BTreeMap::new();
        for triangle in mesh.indices().as_chunks::<3>().0 {
            let [Some(&a), Some(&b), Some(&c)] = [
                mesh.positions().get(triangle[0] as usize),
                mesh.positions().get(triangle[1] as usize),
                mesh.positions().get(triangle[2] as usize),
            ] else {
                continue;
            };
            let va = view_position(self.camera.view(), a);
            let vb = view_position(self.camera.view(), b);
            let vc = view_position(self.camera.view(), c);
            let normal = cross(sub(vb, va), sub(vc, va));
            if dot(normal, normal) <= 1.0e-20 {
                continue;
            }
            let facing = if self.camera.projection()[3][3].abs() < 0.5 {
                dot(normal, scale(add(add(va, vb), vc), -1.0)) > 0.0
            } else {
                normal[2] > 0.0
            };
            for (a, b) in [
                (triangle[0], triangle[1]),
                (triangle[1], triangle[2]),
                (triangle[2], triangle[0]),
            ] {
                if a != b {
                    uses.entry((a.min(b), a.max(b))).or_default().push(facing);
                }
            }
        }
        let mut silhouette_edges = Vec::new();
        for ((a, b), facing) in uses {
            if !(facing.len() == 1 || (facing.len() == 2 && facing[0] != facing[1])) {
                continue;
            }
            let [Some(&position_a), Some(&position_b)] = [
                mesh.positions().get(a as usize),
                mesh.positions().get(b as usize),
            ] else {
                continue;
            };
            if dot(sub(position_a, position_b), sub(position_a, position_b)) <= 1.0e-20
                || feature_segments.iter().any(|&(start, end)| {
                    on_segment(position_a, start, end) && on_segment(position_b, start, end)
                })
            {
                continue;
            }
            silhouette_edges.push((a, b));
        }
        for chain in silhouette_chains(&silhouette_edges) {
            let line: Vec<_> = chain
                .into_iter()
                .filter_map(|id| mesh.positions().get(id as usize).copied())
                .collect();
            append_polyline(&mut vertices, &line, self.camera, self.viewport);
        }
        self.geometry_count = u32::try_from(vertices.len() / 4).unwrap_or(u32::MAX);
        let mut bytes = Vec::with_capacity(vertices.len() * 4);
        for value in vertices {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        self.geometry_vertex = (!bytes.is_empty()).then(|| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("scene geometry edge vertices"),
                contents: &bytes,
                usage: wgpu::BufferUsages::VERTEX,
            })
        });
    }

    pub(crate) fn draw(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let (Some(vertex), Some(index)) = (&self.vertex, &self.index) else {
            return;
        };
        if self.index_count == 0 || self.viewport.width == 0 || self.viewport.height == 0 {
            return;
        }
        let bytes = scene_uniforms(self.camera, self.display_mode, self.viewport, self.selected);
        queue.write_buffer(&self.uniform, 0, &bytes);
        let depth_only = matches!(
            self.display_mode,
            SceneDisplayMode::Line | SceneDisplayMode::HiddenLine
        );
        let color = [Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })];
        let color_attachments = if depth_only { &[][..] } else { &color[..] };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("3d viewport"),
            color_attachments,
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: if self.display_mode.geometry_lines() {
                        wgpu::StoreOp::Store
                    } else {
                        wgpu::StoreOp::Discard
                    },
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_viewport(
            self.viewport.x as f32,
            self.viewport.y as f32,
            self.viewport.width as f32,
            self.viewport.height as f32,
            0.0,
            1.0,
        );
        pass.set_bind_group(0, &self.bind_group, &[]);
        if self.display_mode == SceneDisplayMode::Points {
            let Some(points) = &self.point_vertex else {
                return;
            };
            pass.set_pipeline(&self.point_pipeline);
            pass.set_vertex_buffer(0, points.slice(..));
            for rect in exposed_regions(self.viewport, &self.occlusions) {
                pass.set_scissor_rect(rect.x, rect.y, rect.width, rect.height);
                pass.draw(0..self.point_count, 0..1);
            }
            return;
        }
        pass.set_vertex_buffer(0, vertex.slice(..));
        let (pipeline, draw_edges) = match self.display_mode {
            SceneDisplayMode::Wireframe => (&self.line_pipeline, true),
            SceneDisplayMode::Line | SceneDisplayMode::HiddenLine => (&self.depth_pipeline, false),
            _ => (&self.pipeline, false),
        };
        pass.set_pipeline(pipeline);
        if draw_edges {
            let Some(edge) = &self.edge else { return };
            pass.set_index_buffer(edge.slice(..), wgpu::IndexFormat::Uint32);
        } else {
            pass.set_index_buffer(index.slice(..), wgpu::IndexFormat::Uint32);
        }
        let count = if draw_edges {
            self.edge_count
        } else {
            self.index_count
        };
        for rect in exposed_regions(self.viewport, &self.occlusions) {
            pass.set_scissor_rect(rect.x, rect.y, rect.width, rect.height);
            pass.draw_indexed(0..count, 0, 0..1);
        }
        drop(pass);
        if !self.display_mode.geometry_lines() || self.geometry_count == 0 {
            return;
        }
        let Some(lines) = &self.geometry_vertex else {
            return;
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("visible and hidden geometry edges"),
            color_attachments: &color,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_viewport(
            self.viewport.x as f32,
            self.viewport.y as f32,
            self.viewport.width as f32,
            self.viewport.height as f32,
            0.0,
            1.0,
        );
        pass.set_bind_group(0, &self.line_bind_group, &[]);
        pass.set_vertex_buffer(0, lines.slice(..));
        if self.display_mode == SceneDisplayMode::HiddenLine {
            pass.set_pipeline(&self.hidden_pipeline);
            for rect in exposed_regions(self.viewport, &self.occlusions) {
                pass.set_scissor_rect(rect.x, rect.y, rect.width, rect.height);
                pass.draw(0..self.geometry_count, 0..1);
            }
        }
        pass.set_pipeline(&self.visible_pipeline);
        for rect in exposed_regions(self.viewport, &self.occlusions) {
            pass.set_scissor_rect(rect.x, rect.y, rect.width, rect.height);
            pass.draw(0..self.geometry_count, 0..1);
        }
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: [f32; 3], factor: f32) -> [f32; 3] {
    [a[0] * factor, a[1] * factor, a[2] * factor]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn view_position(matrix: [[f32; 4]; 4], point: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| {
        matrix[0][row] * point[0]
            + matrix[1][row] * point[1]
            + matrix[2][row] * point[2]
            + matrix[3][row]
    })
}

fn clip_position(matrix: [[f32; 4]; 4], point: [f32; 3]) -> [f32; 4] {
    std::array::from_fn(|row| {
        matrix[0][row] * point[0]
            + matrix[1][row] * point[1]
            + matrix[2][row] * point[2]
            + matrix[3][row]
    })
}

fn projected_segment(
    a: [f32; 3],
    b: [f32; 3],
    camera: CameraSnapshot,
    viewport: SceneViewport,
) -> Option<(f32, f32, f32)> {
    let a = clip_position(camera.view_projection(), a);
    let b = clip_position(camera.view_projection(), b);
    let (mut start, mut end) = (0.0_f32, 1.0_f32);
    // Clip in homogeneous coordinates before dividing by w. In particular this
    // avoids a huge or reversed dash phase when a segment crosses the near plane.
    for plane in [[0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 1.0, 0.0]] {
        let da = a.iter().zip(plane).map(|(x, p)| x * p).sum::<f32>() - 1.0e-6;
        let db = b.iter().zip(plane).map(|(x, p)| x * p).sum::<f32>() - 1.0e-6;
        if da < 0.0 && db < 0.0 {
            return None;
        }
        if da < 0.0 || db < 0.0 {
            let t = da / (da - db);
            if da < 0.0 {
                start = start.max(t);
            } else {
                end = end.min(t);
            }
        }
    }
    if end <= start {
        return None;
    }
    let clipped_a: [f32; 4] = std::array::from_fn(|i| a[i] + (b[i] - a[i]) * start);
    let clipped_b: [f32; 4] = std::array::from_fn(|i| a[i] + (b[i] - a[i]) * end);
    let dx =
        (clipped_b[0] / clipped_b[3] - clipped_a[0] / clipped_a[3]) * viewport.width as f32 * 0.5;
    let dy =
        (clipped_b[1] / clipped_b[3] - clipped_a[1] / clipped_a[3]) * viewport.height as f32 * 0.5;
    let length = dx.hypot(dy);
    length
        .is_finite()
        .then_some((start, end, length.min(1.0e7)))
}

fn append_polyline(
    vertices: &mut Vec<f32>,
    points: &[[f32; 3]],
    camera: CameraSnapshot,
    viewport: SceneViewport,
) {
    let mut phase = 0.0;
    for segment in points.windows(2) {
        let Some((start, end, length)) =
            projected_segment(segment[0], segment[1], camera, viewport)
        else {
            continue;
        };
        if end - start < 1.0e-5
            || dot(sub(segment[1], segment[0]), sub(segment[1], segment[0])) <= 1.0e-20
        {
            continue;
        }
        let range = end - start;
        let from_phase = phase - length * start / range;
        phase += length;
        let to_phase = phase + length * (1.0 - end) / range;
        vertices.extend_from_slice(&[segment[0][0], segment[0][1], segment[0][2], from_phase]);
        vertices.extend_from_slice(&[segment[1][0], segment[1][1], segment[1][2], to_phase]);
    }
}

fn on_segment(point: [f32; 3], start: [f32; 3], end: [f32; 3]) -> bool {
    let edge = sub(end, start);
    let length_sq = dot(edge, edge);
    if length_sq <= 1.0e-20 {
        return false;
    }
    let relative = sub(point, start);
    let t = dot(relative, edge) / length_sq;
    (-1.0e-5..=1.0 + 1.0e-5).contains(&t)
        && dot(cross(relative, edge), cross(relative, edge)) <= length_sq * 1.0e-10
}

fn silhouette_chains(edges: &[(u32, u32)]) -> Vec<Vec<u32>> {
    let mut adjacency: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (index, &(a, b)) in edges.iter().enumerate() {
        adjacency.entry(a).or_default().push(index);
        adjacency.entry(b).or_default().push(index);
    }
    let mut seen = vec![false; edges.len()];
    let mut chains = Vec::new();
    for (&vertex, connected) in &adjacency {
        if connected.len() == 2 {
            continue;
        }
        for &edge in connected {
            if !seen[edge] {
                chains.push(trace_silhouette(vertex, edge, edges, &adjacency, &mut seen));
            }
        }
    }
    for (index, &(start, _)) in edges.iter().enumerate() {
        if !seen[index] {
            chains.push(trace_silhouette(start, index, edges, &adjacency, &mut seen));
        }
    }
    chains
}

fn trace_silhouette(
    start: u32,
    first: usize,
    edges: &[(u32, u32)],
    adjacency: &BTreeMap<u32, Vec<usize>>,
    seen: &mut [bool],
) -> Vec<u32> {
    let mut chain = vec![start];
    let mut current = start;
    let mut edge = first;
    loop {
        seen[edge] = true;
        let (a, b) = edges[edge];
        let next = if a == current { b } else { a };
        chain.push(next);
        let Some(next_edges) = adjacency.get(&next) else {
            break;
        };
        if next_edges.len() != 2 {
            break;
        }
        let Some(&unused) = next_edges.iter().find(|&&candidate| !seen[candidate]) else {
            break;
        };
        current = next;
        edge = unused;
    }
    chain
}

fn geometry_line_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
    depth: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("geometry line bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(depth),
            },
        ],
    })
}

fn geometry_line_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    fragment_entry: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("geometry line pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 16,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment_entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn depth_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scene depth only pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 24,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
            }],
        },
        fragment: None,
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: Some(wgpu::Face::Back),
            front_face: wgpu::FrontFace::Ccw,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            // Move the depth-only faces behind their coincident one-pixel
            // contour. The slope term covers pixel-centre depth variation on
            // oblique faces without moving the line or changing its width.
            bias: wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.0,
                clamp: 0.0,
            },
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn scene_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    topology: wgpu::PrimitiveTopology,
    depth_write: bool,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scene pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 24,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology,
            cull_mode: (topology == wgpu::PrimitiveTopology::TriangleList)
                .then_some(wgpu::Face::Back),
            front_face: wgpu::FrontFace::Ccw,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: depth_write,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn scene_uniforms(
    camera: CameraSnapshot,
    display_mode: SceneDisplayMode,
    viewport: SceneViewport,
    selected: bool,
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(112);
    for column in camera.view_projection() {
        for value in column {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }
    // Directional light as a vec4, aligned with the WGSL uniform layout.
    for value in [0.4_f32, 0.6, 0.7, display_mode.shader_id() as f32] {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
    for value in [viewport.width as f32, viewport.height as f32, 0.0, 0.0] {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
    for value in [if selected { 1.10_f32 } else { 1.0 }, 0.0, 0.0, 0.0] {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
    bytes
}

fn depth_view(device: &wgpu::Device, size: (u32, u32)) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("scene depth"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

#[cfg(test)]
mod tests {
    use super::{
        Scene, SceneDisplayMode, clip_position, cross, dot, projected_segment, silhouette_chains,
        sub,
    };
    use crate::{CameraSnapshot, SceneViewport};
    use newell_kernel::NewellSolid;
    use wgpu::util::DeviceExt;

    #[test]
    fn near_plane_clipping_keeps_screen_dash_length_finite() {
        let viewport = SceneViewport::new(0, 0, 256, 192);
        let (start, end, length) = projected_segment(
            [-1.0, 0.0, -1.0],
            [1.0, 0.0, 1.0],
            CameraSnapshot::default(),
            viewport,
        )
        .unwrap();
        assert!((start - 0.5).abs() < 0.001);
        assert_eq!(end, 1.0);
        assert!((length - 128.0).abs() < 0.1);
    }

    #[test]
    fn selection_uniform_is_aligned_and_does_not_change_camera_light_or_viewport() {
        let camera = CameraSnapshot::default();
        let viewport = SceneViewport::new(4, 8, 256, 192);
        let plain = super::scene_uniforms(camera, SceneDisplayMode::Shaded, viewport, false);
        let selected = super::scene_uniforms(camera, SceneDisplayMode::Shaded, viewport, true);
        assert_eq!(plain.len(), 112);
        assert_eq!(selected.len(), 112);
        assert_eq!(&plain[..96], &selected[..96]);
        assert_eq!(&plain[96..100], &1.0_f32.to_ne_bytes());
        assert_eq!(&selected[96..100], &1.1_f32.to_ne_bytes());
        assert_eq!(&selected[100..], &[0; 12]);
        assert_eq!(
            selected,
            super::scene_uniforms(camera, SceneDisplayMode::Shaded, viewport, true)
        );
    }

    #[test]
    #[ignore = "needs a GPU"]
    fn selection_is_ten_percent_once_in_every_mode_and_restores_original_pixels() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter =
            block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let mesh = NewellSolid::box_solid(1.0, 1.0, 1.0)
            .unwrap()
            .tessellate(0.05)
            .unwrap();
        for format in [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ] {
            let mut scene = Scene::new(&device, format, (256, 192));
            scene.set_viewport(&device, SceneViewport::new(0, 0, 256, 192));
            scene.set_camera(&device, three_quarter_camera(false));
            scene.set_mesh(&device, mesh.clone());
            scene.set_occlusions(vec![SceneViewport::new(100, 75, 25, 20)]);
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("selection regression target"),
                size: wgpu::Extent3d {
                    width: 256,
                    height: 192,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = target.create_view(&wgpu::TextureViewDescriptor::default());
            let render = |scene: &Scene| {
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("selection regression clear"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.005,
                                g: 0.01,
                                b: 0.015,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                }));
                scene.draw(&queue, &mut encoder, &view);
                let readback = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("selection readback"),
                    size: 256 * 192 * 4,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                encoder.copy_texture_to_buffer(
                    target.as_image_copy(),
                    wgpu::TexelCopyBufferInfo {
                        buffer: &readback,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(256 * 4),
                            rows_per_image: Some(192),
                        },
                    },
                    wgpu::Extent3d {
                        width: 256,
                        height: 192,
                        depth_or_array_layers: 1,
                    },
                );
                queue.submit(Some(encoder.finish()));
                readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
                device.poll(wgpu::PollType::Wait).unwrap();
                let bytes = readback.slice(..).get_mapped_range().to_vec();
                readback.unmap();
                bytes
            };
            for mode in [
                SceneDisplayMode::Points,
                SceneDisplayMode::Wireframe,
                SceneDisplayMode::Line,
                SceneDisplayMode::HiddenLine,
                SceneDisplayMode::NoShading,
                SceneDisplayMode::Shaded,
                SceneDisplayMode::Default,
            ] {
                scene.set_display_mode(mode);
                scene.set_selected(false);
                let plain = render(&scene);
                scene.set_selected(true);
                let selected = render(&scene);
                scene.set_selected(true);
                assert_eq!(selected, render(&scene), "no accumulation: {mode:?}");
                scene.set_selected(false);
                assert_eq!(plain, render(&scene), "exact restore: {mode:?}");
                let background = &plain[..4];
                let mut geometry = 0;
                for (index, (a, b)) in plain
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(selected.as_chunks::<4>().0)
                    .enumerate()
                {
                    assert_eq!(a[3], b[3]);
                    let x = index % 256;
                    let y = index / 256;
                    if a == background || ((100..125).contains(&x) && (75..95).contains(&y)) {
                        assert_eq!(a, b, "background/occlusion unchanged: {mode:?}");
                        continue;
                    }
                    geometry += 1;
                    for channel in 0..3 {
                        let decode = |value: u8| {
                            let c = f64::from(value) / 255.0;
                            if format.is_srgb() {
                                if c <= 0.04045 {
                                    c / 12.92
                                } else {
                                    ((c + 0.055) / 1.055).powf(2.4)
                                }
                            } else {
                                c
                            }
                        };
                        assert!(
                            (decode(b[channel]) - (decode(a[channel]) * 1.10).min(1.0)).abs()
                                < 0.012,
                            "linear RGB multiplier: {format:?}/{mode:?} {a:?} -> {b:?}"
                        );
                    }
                }
                assert!(geometry > 0, "mode must draw geometry: {mode:?}");
            }
        }
    }

    #[test]
    fn silhouette_segments_form_continuous_polyline_and_loop() {
        assert_eq!(silhouette_chains(&[(0, 1), (2, 1)]), vec![vec![0, 1, 2]]);
        let loop_chain = silhouette_chains(&[(0, 1), (1, 2), (2, 0)]);
        assert_eq!(loop_chain.len(), 1);
        assert_eq!(loop_chain[0].first(), loop_chain[0].last());
        assert_eq!(loop_chain[0].len(), 4);
    }

    /// Manual: `cargo test -p newell-render scene_pipeline_renders_a_kernel_mesh -- --ignored`.
    #[test]
    #[ignore = "needs a GPU"]
    fn scene_pipeline_renders_a_kernel_mesh() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter =
            block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let mut scene = Scene::new(&device, wgpu::TextureFormat::Rgba8Unorm, (256, 192));
        assert_eq!(
            scene.display_mode,
            SceneDisplayMode::Wireframe,
            "new scenes start in wireframe"
        );
        scene.set_viewport(&device, SceneViewport::new(0, 0, 256, 192));
        let mesh = NewellSolid::box_solid(1.0, 1.0, 1.0)
            .unwrap()
            .tessellate(0.05)
            .unwrap();
        scene.set_mesh(&device, mesh);
        // The View accepts exactly the Controller payload; resize, occlusion and
        // mesh uploads cannot mutate it. CPU navigation tests live in newell-app.
        let identity = three_quarter_camera(true);
        let snapshot = CameraSnapshot::new(
            identity.view(),
            identity.projection(),
            identity.view_projection(),
            17,
        )
        .unwrap();
        scene.set_camera(&device, snapshot);
        scene.set_occlusions(vec![SceneViewport::new(200, 0, 56, 192)]);
        scene.set_viewport(&device, SceneViewport::new(0, 0, 256, 192));
        scene.set_mesh(
            &device,
            NewellSolid::box_solid(1.0, 1.0, 1.0)
                .unwrap()
                .tessellate(0.05)
                .unwrap(),
        );
        assert_eq!(
            scene.geometry_count, 24,
            "12 box edges, no tessellation lines"
        );
        assert_eq!(scene.camera, snapshot);
        scene.set_occlusions(Vec::new());
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene test target"),
            size: wgpu::Extent3d {
                width: 256,
                height: 192,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        for perspective in [false, true] {
            let mut line_frame = Vec::new();
            let mut hidden_frame = Vec::new();
            let mut wireframe_frame = Vec::new();
            let mut shaded_frame = Vec::new();
            let mut default_frame = Vec::new();
            scene.set_camera(&device, three_quarter_camera(perspective));
            for mode in [
                SceneDisplayMode::Points,
                SceneDisplayMode::Line,
                SceneDisplayMode::Wireframe,
                SceneDisplayMode::HiddenLine,
                SceneDisplayMode::NoShading,
                SceneDisplayMode::Shaded,
                SceneDisplayMode::Default,
            ] {
                scene.set_display_mode(mode);
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("scene test clear"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                }));
                device.push_error_scope(wgpu::ErrorFilter::Validation);
                scene.draw(&queue, &mut encoder, &view);
                let readback = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("scene test readback"),
                    size: 256 * 192 * 4,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                encoder.copy_texture_to_buffer(
                    target.as_image_copy(),
                    wgpu::TexelCopyBufferInfo {
                        buffer: &readback,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(256 * 4),
                            rows_per_image: Some(192),
                        },
                    },
                    wgpu::Extent3d {
                        width: 256,
                        height: 192,
                        depth_or_array_layers: 1,
                    },
                );
                queue.submit(Some(encoder.finish()));
                let _ = device.poll(wgpu::PollType::Wait);
                readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
                let _ = device.poll(wgpu::PollType::Wait);
                let bytes = readback.slice(..).get_mapped_range();
                if std::env::var_os("NEWELL_SCENE_DUMP").is_some() {
                    let mut ppm = b"P6\n256 192\n255\n".to_vec();
                    for pixel in bytes.as_chunks::<4>().0 {
                        ppm.extend_from_slice(&pixel[..3]);
                    }
                    let path = format!("/tmp/newell-scene-{perspective}-{mode:?}.ppm");
                    std::fs::write(path, ppm).unwrap();
                }
                match mode {
                    SceneDisplayMode::Line => line_frame = bytes.to_vec(),
                    SceneDisplayMode::HiddenLine => hidden_frame = bytes.to_vec(),
                    SceneDisplayMode::Wireframe => wireframe_frame = bytes.to_vec(),
                    SceneDisplayMode::Shaded => shaded_frame = bytes.to_vec(),
                    SceneDisplayMode::Default => default_frame = bytes.to_vec(),
                    _ => {}
                }
                drop(bytes);
                readback.unmap();
                assert!(block_on(device.pop_error_scope()).is_none(), "{mode:?}");
            }
            let lit = |frame: &[u8]| {
                frame
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|px| px[..3] != [0, 0, 0])
                    .count()
            };
            assert!(
                lit(&wireframe_frame) > lit(&line_frame) + 40,
                "mesh diagonals only in Wireframe"
            );
            assert!(
                lit(&hidden_frame) > lit(&line_frame),
                "occluded edges become dashed in Hidden Line"
            );
            assert!(
                lit(&default_frame) > lit(&line_frame),
                "Default includes shaded surfaces"
            );
            assert_ne!(
                shaded_frame, default_frame,
                "Default adds visible geometry edges"
            );
        }

        // One edge crosses the box projection: its exposed ends stay solid while
        // the portion behind the box is omitted in Line and dashed in Hidden Line.
        scene.set_occlusions(Vec::new());
        let face_on_view = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-0.5, -0.5, -3.0, 1.0],
        ];
        let face_on_projection = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, -0.2, 0.0],
            [0.0, 0.0, -0.2, 1.0],
        ];
        let face_on_vp = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, -0.2, 0.0],
            [-0.5, -0.5, 0.4, 1.0],
        ];
        scene.set_camera(
            &device,
            CameraSnapshot::new(face_on_view, face_on_projection, face_on_vp, 2).unwrap(),
        );
        let mut bytes = Vec::new();
        for value in [-0.5_f32, 0.5, 0.0, 0.0, 1.5, 0.5, 0.0, 256.0] {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        scene.geometry_vertex = Some(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("partial occlusion test edge"),
                contents: &bytes,
                usage: wgpu::BufferUsages::VERTEX,
            },
        ));
        scene.geometry_count = 2;
        let mut render = |mode, mask_edge| {
            scene.set_occlusions(if mask_edge {
                vec![SceneViewport::new(24, 90, 16, 12)]
            } else {
                Vec::new()
            });
            scene.set_display_mode(mode);
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("partial occlusion readback"),
                size: 256 * 192 * 4,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("partial occlusion clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            }));
            scene.draw(&queue, &mut encoder, &view);
            encoder.copy_texture_to_buffer(
                target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(1024),
                        rows_per_image: Some(192),
                    },
                },
                wgpu::Extent3d {
                    width: 256,
                    height: 192,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(Some(encoder.finish()));
            readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
            let _ = device.poll(wgpu::PollType::Wait);
            let mapped = readback.slice(..).get_mapped_range();
            let pixels = mapped.to_vec();
            drop(mapped);
            readback.unmap();
            pixels
        };
        let line = render(SceneDisplayMode::Line, false);
        let hidden = render(SceneDisplayMode::HiddenLine, false);
        let masked = render(SceneDisplayMode::Line, true);
        let lit = |pixels: &[u8], left: usize, right: usize| {
            (95..=97)
                .flat_map(|y| (left..right).map(move |x| (x, y)))
                .filter(|&(x, y)| {
                    let offset = (y * 256 + x) * 4;
                    pixels[offset..offset + 3] != [0, 0, 0]
                })
                .count()
        };
        assert!(lit(&line, 16, 48) > 0, "exposed edge end is solid");
        assert_eq!(lit(&line, 80, 176), 0, "occluded middle is absent in Line");
        assert!(lit(&hidden, 80, 176) > 0, "occluded middle is dashed");
        assert!(lit(&hidden, 80, 176) < 96, "occluded middle has dash gaps");
        assert_eq!(lit(&masked, 24, 40), 0, "UI occlusion clips the line pass");
    }

    /// A shallow view from below exposes the box's far bottom contour. The
    /// surface depth pass must not break that one-pixel line into short runs.
    #[test]
    #[ignore = "needs a GPU"]
    fn visible_box_contour_remains_solid_at_shallow_angle() {
        const SIZE: u32 = 768;
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter =
            block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let mut scene = Scene::new(&device, wgpu::TextureFormat::Rgba8Unorm, (SIZE, SIZE));
        scene.set_viewport(&device, SceneViewport::new(0, 0, SIZE, SIZE));
        scene.set_mesh(
            &device,
            NewellSolid::box_solid(1.0, 1.0, 1.0)
                .unwrap()
                .tessellate(0.05)
                .unwrap(),
        );
        let eye = [0.8, 3.5, -0.3];
        let center = [0.5, 0.5, 0.5];
        let normalized = |v: [f32; 3]| {
            let n = dot(v, v).sqrt();
            [v[0] / n, v[1] / n, v[2] / n]
        };
        let backward = normalized(sub(eye, center));
        let right = normalized(cross([0.0, 0.0, 1.0], backward));
        let up = cross(backward, right);
        let view_matrix = [
            [right[0], up[0], backward[0], 0.0],
            [right[1], up[1], backward[1], 0.0],
            [right[2], up[2], backward[2], 0.0],
            [-dot(right, eye), -dot(up, eye), -dot(backward, eye), 1.0],
        ];
        let projection = [
            [0.9, 0.0, 0.0, 0.0],
            [0.0, 0.9, 0.0, 0.0],
            [0.0, 0.0, -0.2, 0.0],
            [0.0, 0.0, -0.2, 1.0],
        ];
        let view_projection = std::array::from_fn(|column| {
            std::array::from_fn(|row| {
                (0..4)
                    .map(|k| projection[k][row] * view_matrix[column][k])
                    .sum()
            })
        });
        let camera = CameraSnapshot::new(view_matrix, projection, view_projection, 1).unwrap();
        scene.set_camera(&device, camera);
        scene.set_display_mode(SceneDisplayMode::Line);
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shallow box contour target"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shallow box contour readback"),
            size: u64::from(SIZE * SIZE * 4),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shallow box contour clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        }));
        scene.draw(&queue, &mut encoder, &view);
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(SIZE * 4),
                    rows_per_image: Some(SIZE),
                },
            },
            wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::PollType::Wait);
        let pixels = readback.slice(..).get_mapped_range();
        let project = |point| {
            let clip = clip_position(camera.view_projection(), point);
            (
                (clip[0] / clip[3] * 0.5 + 0.5) * SIZE as f32,
                (0.5 - clip[1] / clip[3] * 0.5) * SIZE as f32,
            )
        };
        let lit_at = |a: (f32, f32), b: (f32, f32), i: i32| {
            let t = i as f32 / 100.0;
            let x = (a.0 + (b.0 - a.0) * t).round() as i32;
            let y = (a.1 + (b.1 - a.1) * t).round() as i32;
            (-1..=1).any(|dy| {
                (-1..=1).any(|dx| {
                    let (px, py) = (x + dx, y + dy);
                    if px < 0 || py < 0 || px >= SIZE as i32 || py >= SIZE as i32 {
                        return false;
                    }
                    let offset = (py as usize * SIZE as usize + px as usize) * 4;
                    pixels[offset..offset + 3] != [0, 0, 0]
                })
            })
        };
        let a = project([0.0, 0.0, 0.0]);
        let b = project([1.0, 0.0, 0.0]);
        let missing = (5..95).filter(|&i| !lit_at(a, b, i)).count();
        assert!(
            missing <= 1,
            "visible box contour lost {missing}/90 samples"
        );

        // The parallel rear top edge lies behind the front face. The bias
        // must not turn its intended occlusion into a visible solid line.
        let hidden_a = project([0.0, 0.0, 1.0]);
        let hidden_b = project([1.0, 0.0, 1.0]);
        let false_visible = (10..90).filter(|&i| lit_at(hidden_a, hidden_b, i)).count();
        assert_eq!(false_visible, 0, "rear top edge leaked through the box");
    }

    fn three_quarter_camera(perspective: bool) -> CameraSnapshot {
        let eye = [2.5, 2.0, 3.0];
        let center = [0.5, 0.5, 0.5];
        let normalize = |v: [f32; 3]| {
            let n = dot(v, v).sqrt();
            [v[0] / n, v[1] / n, v[2] / n]
        };
        let backward = normalize(sub(eye, center));
        let right = normalize(cross([0.0, 0.0, 1.0], backward));
        let up = cross(backward, right);
        let view = [
            [right[0], up[0], backward[0], 0.0],
            [right[1], up[1], backward[1], 0.0],
            [right[2], up[2], backward[2], 0.0],
            [-dot(right, eye), -dot(up, eye), -dot(backward, eye), 1.0],
        ];
        let projection = if perspective {
            [
                [1.299038, 0.0, 0.0, 0.0],
                [0.0, 1.7320508, 0.0, 0.0],
                [0.0, 0.0, -1.010101, -1.0],
                [0.0, 0.0, -0.1010101, 0.0],
            ]
        } else {
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, -0.2, 0.0],
                [0.0, 0.0, -0.2, 1.0],
            ]
        };
        let view_projection = std::array::from_fn(|column| {
            std::array::from_fn(|row| (0..4).map(|k| projection[k][row] * view[column][k]).sum())
        });
        CameraSnapshot::new(view, projection, view_projection, 1).unwrap()
    }

    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        use std::sync::Arc;
        use std::task::{Context, Poll, Wake, Waker};

        struct WakerThread(std::thread::Thread);
        impl Wake for WakerThread {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }

        let waker = Waker::from(Arc::new(WakerThread(std::thread::current())));
        let mut context = Context::from_waker(&waker);
        let mut future = std::pin::pin!(future);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(value) => return value,
                Poll::Pending => std::thread::park(),
            }
        }
    }
}
