// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Scene mesh upload and immutable camera-uniform consumption for M3.

use crate::{CameraSnapshot, SceneViewport, exposed_regions};
use newell_kernel::NewellMesh;
use wgpu::util::DeviceExt;

/// GPU scene resources. Navigation and CPU geometric queries belong to the Controller.
pub(crate) struct Scene {
    pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    point_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    vertex: Option<wgpu::Buffer>,
    index: Option<wgpu::Buffer>,
    edge: Option<wgpu::Buffer>,
    point_vertex: Option<wgpu::Buffer>,
    index_count: u32,
    edge_count: u32,
    point_count: u32,
    depth: wgpu::TextureView,
    target_size: (u32, u32),
    viewport: SceneViewport,
    occlusions: Vec<SceneViewport>,
    camera: CameraSnapshot,
    shading: u32,
}

impl Scene {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        size: (u32, u32),
    ) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene camera uniform"),
            size: 96,
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
                    min_binding_size: wgpu::BufferSize::new(96),
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
        Self {
            pipeline,
            line_pipeline,
            point_pipeline,
            bind_group,
            uniform,
            vertex: None,
            index: None,
            edge: None,
            point_vertex: None,
            index_count: 0,
            edge_count: 0,
            point_count: 0,
            depth: depth_view(device, target_size),
            target_size,
            viewport: SceneViewport::default(),
            occlusions: Vec::new(),
            camera: CameraSnapshot::default(),
            shading: 4,
        }
    }

    pub(crate) fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if size != self.target_size {
            self.target_size = size;
            self.depth = depth_view(device, size);
        }
    }

    pub(crate) fn set_viewport(&mut self, viewport: SceneViewport) {
        self.viewport = viewport;
    }

    pub(crate) fn set_occlusions(&mut self, occlusions: Vec<SceneViewport>) {
        self.occlusions = occlusions;
    }

    pub(crate) fn set_camera(&mut self, camera: CameraSnapshot) {
        self.camera = camera;
    }

    pub(crate) fn set_shading(&mut self, shading: usize) {
        self.shading = u32::try_from(shading.min(5)).unwrap_or(4);
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
        let mut bytes = Vec::with_capacity(96);
        for column in self.camera.view_projection() {
            for value in column {
                bytes.extend_from_slice(&value.to_ne_bytes());
            }
        }
        // Directional light as a vec4, aligned with the WGSL uniform layout.
        for value in [0.4_f32, 0.6, 0.7, self.shading as f32] {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        for value in [
            self.viewport.width as f32,
            self.viewport.height as f32,
            0.0,
            0.0,
        ] {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        queue.write_buffer(&self.uniform, 0, &bytes);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("3d viewport"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
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
        if self.shading == 0 {
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
        let (pipeline, draw_edges) = match self.shading {
            1 | 2 => (&self.line_pipeline, true),
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
        // Standard overlays edges on the shaded model. It intentionally keeps the shaded
        // triangle pass first, so the edge pass is visible without changing depth ownership.
        if self.shading == 5 {
            let Some(edge) = &self.edge else { return };
            pass.set_pipeline(&self.line_pipeline);
            pass.set_index_buffer(edge.slice(..), wgpu::IndexFormat::Uint32);
            for rect in exposed_regions(self.viewport, &self.occlusions) {
                pass.set_scissor_rect(rect.x, rect.y, rect.width, rect.height);
                pass.draw_indexed(0..self.edge_count, 0, 0..1);
            }
        }
    }
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

#[cfg(test)]
mod tests {
    use super::Scene;
    use crate::{CameraSnapshot, SceneViewport};
    use newell_kernel::NewellSolid;

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
        scene.set_viewport(SceneViewport::new(0, 0, 256, 192));
        let mesh = NewellSolid::box_solid(1.0, 1.0, 1.0)
            .unwrap()
            .tessellate(0.05)
            .unwrap();
        scene.set_mesh(&device, mesh);
        // The View accepts exactly the Controller payload; resize, occlusion and
        // mesh uploads cannot mutate it. CPU navigation tests live in newell-app.
        let identity = CameraSnapshot::default();
        let snapshot = CameraSnapshot::new(
            identity.view(),
            identity.projection(),
            identity.view_projection(),
            17,
        )
        .unwrap();
        scene.set_camera(snapshot);
        scene.set_occlusions(vec![SceneViewport::new(200, 0, 56, 192)]);
        scene.set_viewport(SceneViewport::new(0, 0, 256, 192));
        scene.set_mesh(
            &device,
            NewellSolid::box_solid(1.0, 1.0, 1.0)
                .unwrap()
                .tessellate(0.05)
                .unwrap(),
        );
        assert_eq!(scene.camera, snapshot);
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
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
        queue.submit(Some(encoder.finish()));
        let _ = device.poll(wgpu::PollType::Wait);
        assert!(block_on(device.pop_error_scope()).is_none());
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
