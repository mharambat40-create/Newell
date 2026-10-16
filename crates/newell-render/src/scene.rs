// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Scene mesh upload, perspective camera and CPU ray picking for M3.

use newell_kernel::NewellMesh;
use wgpu::util::DeviceExt;

/// A physical-pixel rectangle occupied by the 3D viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SceneViewport {
    /// Left pixel coordinate.
    pub x: u32,
    /// Top pixel coordinate.
    pub y: u32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl SceneViewport {
    /// Makes a physical viewport rectangle.
    #[must_use]
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn contains(self, x: f64, y: f64) -> bool {
        x >= f64::from(self.x)
            && y >= f64::from(self.y)
            && x < f64::from(self.x.saturating_add(self.width))
            && y < f64::from(self.y.saturating_add(self.height))
            && self.width > 0
            && self.height > 0
    }
}

/// A face hit from a viewport pick. Face indices refer to `NewellMesh::face_offsets()` order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickHit {
    /// B-Rep face ordinal represented by the hit triangle.
    pub face_index: usize,
    /// Distance from the camera eye in model units.
    pub distance: f32,
}

/// Fixed perspective camera used by the first M3 viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    eye: [f32; 3],
    target: [f32; 3],
    up: [f32; 3],
    vertical_fov_radians: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: [3.4, 3.0, 3.2],
            target: [0.5, 0.5, 0.5],
            up: [0.0, 0.0, 1.0],
            vertical_fov_radians: 45.0_f32.to_radians(),
        }
    }
}

impl Camera {
    /// View-projection matrix for the supplied viewport aspect ratio.
    #[must_use]
    pub fn view_projection(self, aspect: f32) -> [[f32; 4]; 4] {
        multiply(
            perspective(self.vertical_fov_radians, aspect, 0.01, 100.0),
            self.view(),
        )
    }

    fn view(self) -> [[f32; 4]; 4] {
        let forward = normalize(sub(self.target, self.eye));
        let side = normalize(cross(forward, self.up));
        let up = cross(side, forward);
        [
            [side[0], up[0], -forward[0], 0.0],
            [side[1], up[1], -forward[1], 0.0],
            [side[2], up[2], -forward[2], 0.0],
            [
                -dot(side, self.eye),
                -dot(up, self.eye),
                dot(forward, self.eye),
                1.0,
            ],
        ]
    }

    fn ray(self, viewport: SceneViewport, x: f64, y: f64) -> Option<Ray> {
        if !viewport.contains(x, y) {
            return None;
        }
        let width = viewport.width as f32;
        let height = viewport.height as f32;
        let ndc_x = (((x as f32 - viewport.x as f32) / width) * 2.0) - 1.0;
        let ndc_y = 1.0 - (((y as f32 - viewport.y as f32) / height) * 2.0);
        let forward = normalize(sub(self.target, self.eye));
        let side = normalize(cross(forward, self.up));
        let up = cross(side, forward);
        let half_height = (self.vertical_fov_radians * 0.5).tan();
        let half_width = half_height * (width / height);
        Some(Ray {
            origin: self.eye,
            direction: normalize(add(
                add(forward, scale(side, ndc_x * half_width)),
                scale(up, ndc_y * half_height),
            )),
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct Ray {
    origin: [f32; 3],
    direction: [f32; 3],
}

/// GPU scene resources. The CPU mesh is deliberately retained for exact input picking.
pub(crate) struct Scene {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    vertex: Option<wgpu::Buffer>,
    index: Option<wgpu::Buffer>,
    index_count: u32,
    depth: wgpu::TextureView,
    target_size: (u32, u32),
    viewport: SceneViewport,
    camera: Camera,
    mesh: Option<NewellMesh>,
}

impl Scene {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        size: (u32, u32),
    ) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene camera uniform"),
            size: 80,
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
                    min_binding_size: wgpu::BufferSize::new(80),
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
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 24,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let target_size = (size.0.max(1), size.1.max(1));
        Self {
            pipeline,
            bind_group,
            uniform,
            vertex: None,
            index: None,
            index_count: 0,
            depth: depth_view(device, target_size),
            target_size,
            viewport: SceneViewport::default(),
            camera: Camera::default(),
            mesh: None,
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
        self.index_count = u32::try_from(mesh.indices().len()).unwrap_or(u32::MAX);
        self.mesh = Some(mesh);
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
        let aspect = self.viewport.width as f32 / self.viewport.height as f32;
        let mut bytes = Vec::with_capacity(80);
        for column in self.camera.view_projection(aspect) {
            for value in column {
                bytes.extend_from_slice(&value.to_ne_bytes());
            }
        }
        // Directional light as a vec4, aligned with the WGSL uniform layout.
        for value in [0.4_f32, 0.6, 0.7, 0.0] {
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, vertex.slice(..));
        pass.set_index_buffer(index.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }

    pub(crate) fn pick(&self, x: f64, y: f64) -> Option<PickHit> {
        let mesh = self.mesh.as_ref()?;
        let ray = self.camera.ray(self.viewport, x, y)?;
        pick_mesh(ray, mesh)
    }
}

fn pick_mesh(ray: Ray, mesh: &NewellMesh) -> Option<PickHit> {
    let mut closest = None;
    let (triangles, remainder) = mesh.indices().as_chunks::<3>();
    if !remainder.is_empty() {
        return None;
    }
    for (triangle, indices) in triangles.iter().enumerate() {
        let [a, b, c] = [
            *mesh.positions().get(indices[0] as usize)?,
            *mesh.positions().get(indices[1] as usize)?,
            *mesh.positions().get(indices[2] as usize)?,
        ];
        if let Some(distance) = intersect(ray, a, b, c) {
            let face_index = mesh.face_offsets().windows(2).position(|range| {
                triangle as u32 * 3 >= range[0] && triangle as u32 * 3 < range[1]
            })?;
            let hit = PickHit {
                face_index,
                distance,
            };
            if closest.is_none_or(|old: PickHit| hit.distance < old.distance) {
                closest = Some(hit);
            }
        }
    }
    closest
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

fn intersect(ray: Ray, a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Option<f32> {
    let edge_one = sub(b, a);
    let edge_two = sub(c, a);
    let p = cross(ray.direction, edge_two);
    let determinant = dot(edge_one, p);
    if determinant.abs() < 0.000_001 {
        return None;
    }
    let inverse = 1.0 / determinant;
    let t = sub(ray.origin, a);
    let u = dot(t, p) * inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross(t, edge_one);
    let v = dot(ray.direction, q) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = dot(edge_two, q) * inverse;
    (distance > 0.000_001).then_some(distance)
}

fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fov * 0.5).tan();
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, (near * far) / (near - far), 0.0],
    ]
}

fn multiply(left: [[f32; 4]; 4], right: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = [[0.0; 4]; 4];
    for column in 0..4 {
        for row in 0..4 {
            result[column][row] = (0..4).map(|i| left[i][row] * right[column][i]).sum();
        }
    }
    result
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn scale(value: [f32; 3], factor: f32) -> [f32; 3] {
    [value[0] * factor, value[1] * factor, value[2] * factor]
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
fn normalize(value: [f32; 3]) -> [f32; 3] {
    scale(value, 1.0 / dot(value, value).sqrt())
}

#[cfg(test)]
mod tests {
    use super::{Camera, Scene, SceneViewport, pick_mesh};
    use newell_kernel::NewellSolid;

    #[test]
    fn centre_ray_picks_the_kernel_box_and_outside_is_ignored() {
        let solid = NewellSolid::box_solid(1.0, 1.0, 1.0).unwrap();
        let mesh = solid.tessellate(0.05).unwrap();
        let viewport = SceneViewport::new(20, 10, 800, 600);
        let ray = Camera::default().ray(viewport, 420.0, 310.0).unwrap();
        let hit = pick_mesh(ray, &mesh).unwrap();

        assert!(hit.face_index < mesh.face_count());
        assert!(hit.distance > 0.0);
        assert!(Camera::default().ray(viewport, 19.0, 10.0).is_none());
    }

    #[test]
    fn camera_matrix_stays_finite_for_a_standard_viewport() {
        assert!(
            Camera::default()
                .view_projection(4.0 / 3.0)
                .iter()
                .flatten()
                .all(|value| value.is_finite())
        );
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
        scene.set_viewport(SceneViewport::new(0, 0, 256, 192));
        let mesh = NewellSolid::box_solid(1.0, 1.0, 1.0)
            .unwrap()
            .tessellate(0.05)
            .unwrap();
        scene.set_mesh(&device, mesh);
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
