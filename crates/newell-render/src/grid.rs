// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Bounded procedural XY grid. The Controller publishes inverse camera and visibility.
use wgpu::util::DeviceExt;

/// Immutable, Controller-derived grid inputs; no document or interaction state.
#[derive(Clone, Copy, Debug)]
pub struct GridSnapshot {
    pub inverse_view_projection: [[f32; 4]; 4],
    pub view_projection: [[f32; 4]; 4],
    pub dark: bool,
    /// Eye xyz and perspective flag; orthographic rays start on the eye plane.
    pub ray_origin: [f32; 4],
    /// Camera focal-plane model units per physical pixel, shared by every fragment.
    pub world_per_pixel: f32,
}

pub(crate) struct Grid {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}
impl Grid {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("adaptive grid"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("adaptive grid"),
            source: wgpu::ShaderSource::Wgsl(include_str!("grid.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("adaptive grid"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        Self { pipeline, layout }
    }
    pub fn draw(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        scene: &crate::scene::Scene,
        snapshot: GridSnapshot,
    ) {
        let viewport = scene.viewport;
        let depth = &scene.reference_depth;
        let regions = crate::exposed_regions(viewport, &scene.occlusions);
        let mut values: Vec<f32> = snapshot
            .inverse_view_projection
            .into_iter()
            .flatten()
            .chain(snapshot.view_projection.into_iter().flatten())
            .collect();
        values.extend([
            viewport.x as f32,
            viewport.y as f32,
            viewport.width as f32,
            viewport.height as f32,
        ]);
        values.extend([
            if snapshot.dark { 1.0 } else { 0.0 },
            snapshot.world_per_pixel,
            0.0,
            0.0,
        ]);
        values.extend(snapshot.ray_origin);
        let bytes: Vec<u8> = values.into_iter().flat_map(f32::to_ne_bytes).collect();
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grid inputs"),
            contents: &bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
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
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("adaptive grid"),
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.set_viewport(
            viewport.x as f32,
            viewport.y as f32,
            viewport.width as f32,
            viewport.height as f32,
            0.0,
            1.0,
        );
        for rect in regions {
            pass.set_scissor_rect(rect.x, rect.y, rect.width, rect.height);
            pass.draw(0..3, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "needs a GPU"]
    fn adaptive_grid_pipeline_validates() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = crate::renderer::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        )
        .unwrap();
        let (device, _) =
            crate::renderer::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .unwrap();
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let _grid = super::Grid::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
        assert!(crate::renderer::block_on(device.pop_error_scope()).is_none());
    }

    #[test]
    #[ignore = "needs a GPU"]
    fn grid_pixels_visibility_themes_depth_and_extreme_scales() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = crate::renderer::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        )
        .unwrap();
        let (device, queue) =
            crate::renderer::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .unwrap();
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let grid = super::Grid::new(&device, format);
        let mut scene = crate::scene::Scene::new(&device, format, (256, 256));
        scene.set_viewport(&device, crate::SceneViewport::new(0, 0, 256, 256));
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("origin reference validation"),
            size: wgpu::Extent3d {
                width: 256,
                height: 256,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let render =
            |scene: &crate::scene::Scene, snapshot: Option<super::GridSnapshot>, blocked: bool| {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: 256 * 256 * 4,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                let mut encoder = device.create_command_encoder(&Default::default());
                drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(if snapshot.is_some_and(|s| !s.dark) {
                                wgpu::Color::WHITE
                            } else {
                                wgpu::Color::BLACK
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                }));
                drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &scene.reference_depth,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(if blocked { 0.0 } else { 1.0 }),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                }));
                if let Some(snapshot) = snapshot {
                    grid.draw(&device, &mut encoder, &view, scene, snapshot);
                }
                encoder.copy_texture_to_buffer(
                    target.as_image_copy(),
                    wgpu::TexelCopyBufferInfo {
                        buffer: &buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(1024),
                            rows_per_image: Some(256),
                        },
                    },
                    wgpu::Extent3d {
                        width: 256,
                        height: 256,
                        depth_or_array_layers: 1,
                    },
                );
                queue.submit(Some(encoder.finish()));
                buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
                let _ = device.poll(wgpu::PollType::Wait);
                buffer.slice(..).get_mapped_range().to_vec()
            };

        let empty = render(&scene, None, false);
        for scale in [0.001_f32, 1.0, 100.0, 1e6] {
            let snapshot = super::GridSnapshot {
                inverse_view_projection: [
                    [scale, 0., 0., 0.],
                    [0., scale, 0., 0.],
                    [0., 0., 1., 0.],
                    [0., 0., -0.5, 1.],
                ],
                view_projection: [
                    [1. / scale, 0., 0., 0.],
                    [0., 1. / scale, 0., 0.],
                    [0., 0., 1., 0.],
                    [0., 0., 0.5, 1.],
                ],
                dark: true,
                ray_origin: [0.0; 4],
                world_per_pixel: scale / 128.0,
            };
            let visible = render(&scene, Some(snapshot), false);
            assert_ne!(visible, empty, "scale {scale}");
            assert_eq!(render(&scene, Some(snapshot), true), empty, "depth {scale}");
            let light = super::GridSnapshot {
                dark: false,
                ..snapshot
            };
            let light_background = render(&scene, Some(light), true);
            assert!(light_background.iter().all(|v| *v == 255));
            assert_ne!(
                render(&scene, Some(light), false),
                light_background,
                "light contrast {scale}"
            );
            // Zoom within a level transition is continuous and remains finite.
            let mut zoom = snapshot;
            zoom.inverse_view_projection[0][0] *= 1.001;
            zoom.inverse_view_projection[1][1] *= 1.001;
            zoom.world_per_pixel *= 1.001;
            zoom.view_projection[0][0] /= 1.001;
            zoom.view_projection[1][1] /= 1.001;
            let shifted = render(&scene, Some(zoom), false);
            let delta: u64 = visible
                .iter()
                .zip(shifted.iter())
                .map(|(a, b)| u64::from(a.abs_diff(*b)))
                .sum();
            assert!(delta < 256 * 256 * 4, "transition {scale}: {delta}");
            // Model clipping must not bound the grid when no surface occupies a pixel.
            for clip_z in [-10.0, 10.0] {
                let mut clipped = snapshot;
                clipped.view_projection[3][2] = clip_z;
                assert_eq!(
                    render(&scene, Some(clipped), false),
                    visible,
                    "scene depth range leaked into grid"
                );
            }
            // The same world plane and spacing are reconstructed with perspective rays.
            let perspective = super::GridSnapshot {
                inverse_view_projection: [
                    [scale, 0., 0., 0.],
                    [0., scale, 0., 0.],
                    [0., 0., -0.5, -0.5],
                    [0., 0., 0., 1.],
                ],
                ray_origin: [0., 0., 1., 1.],
                ..snapshot
            };
            if scale == 1.0 {
                let perspective_pixels = render(&scene, Some(perspective), false);
                assert!(
                    perspective_pixels
                        .iter()
                        .zip(&visible)
                        .all(|(a, b)| a.abs_diff(*b) <= 1),
                    "perspective plane coverage"
                );
            }
            let mut looking_away = perspective;
            looking_away.inverse_view_projection[3][2] = 2.0;
            assert!(
                render(&scene, Some(looking_away), false) == empty,
                "grid behind camera"
            );
            // Reversing a zoom produces the exact original image: no retained level state.
            assert_eq!(render(&scene, Some(snapshot), false), visible);
        }
        // Both zero-coordinate lines must stay continuous while the blended
        // 1/2/5 spacing family crosses each level boundary. At the crossing,
        // max-coverage blending keeps the two lines from compositing brighter.
        let mut cross_center = None;
        for desired in [
            0.19_f32, 0.1999, 0.2001, 0.4999, 0.5001, 0.999, 1.9, 1.999, 2.001, 2.1, 4.999, 5.001,
            9.99, 19.9, 20.01, 49.99, 50.01, 99.9,
        ] {
            let world_per_pixel = desired / 90.0;
            let footprint_world = world_per_pixel * 128.0;
            let snapshot = super::GridSnapshot {
                inverse_view_projection: [
                    [footprint_world, 0., 0., 0.],
                    [0., footprint_world, 0., 0.],
                    [0., 0., 1., 0.],
                    [0., 0., -0.5, 1.],
                ],
                view_projection: [
                    [1.0 / footprint_world, 0., 0., 0.],
                    [0., 1.0 / footprint_world, 0., 0.],
                    [0., 0., 1., 0.],
                    [0., 0., 0.5, 1.],
                ],
                dark: true,
                ray_origin: [0.0; 4],
                world_per_pixel,
            };
            let cross = render(&scene, Some(snapshot), false);
            let pixel = |x: usize, y: usize| cross[(y * 256 + x) * 4];
            assert!(
                (24..232).all(|x| pixel(x, 128) > 0),
                "X=0 grid line has a gap at level ratio {desired}"
            );
            assert!(
                (24..232).all(|y| pixel(128, y) > 0),
                "Y=0 grid line has a gap at level ratio {desired}"
            );
            let center = [pixel(128, 128), cross[(128 * 256 + 128) * 4 + 1]];
            let adjacent_zero_line = [pixel(131, 128), cross[(128 * 256 + 131) * 4 + 1]];
            assert!(
                center
                    .into_iter()
                    .zip(adjacent_zero_line)
                    .all(|(center, line)| center.abs_diff(line) <= 2),
                "zero-line crossing became brighter than one line at level ratio {desired}: {center:?} vs {adjacent_zero_line:?}"
            );
            if let Some(previous) = cross_center {
                assert!(
                    center
                        .into_iter()
                        .zip(previous)
                        .all(|(actual, expected)| actual.abs_diff(expected) <= 2),
                    "zero-line appearance changed across subdivision transition {desired}: {previous:?} -> {center:?}"
                );
            }
            cross_center = Some(center);
        }
        for viewport in [
            crate::SceneViewport::new(16, 24, 224, 96),
            crate::SceneViewport::new(32, 8, 96, 224),
        ] {
            scene.set_viewport(&device, viewport);
            let snapshot = super::GridSnapshot {
                inverse_view_projection: [
                    [100., 0., 0., 0.],
                    [0., 100., 0., 0.],
                    [0., 0., 1., 0.],
                    [0., 0., -0.5, 1.],
                ],
                view_projection: [
                    [0.01, 0., 0., 0.],
                    [0., 0.01, 0., 0.],
                    [0., 0., 1., 0.],
                    [0., 0., 10., 1.],
                ],
                ray_origin: [0.; 4],
                world_per_pixel: 100. / viewport.width as f32,
                dark: true,
            };
            let full = render(&scene, Some(snapshot), false);
            let occlusion = crate::SceneViewport::new(80, 64, 32, 64);
            scene.set_occlusions(vec![occlusion]);
            let split = render(&scene, Some(snapshot), false);
            for y in 0..256 {
                for x in 0..256 {
                    let i = (y * 256 + x) * 4;
                    if !occlusion.contains(x as f64, y as f64) {
                        assert_eq!(&full[i..i + 4], &split[i..i + 4], "draw-region seam");
                    }
                }
            }
            scene.set_occlusions(vec![]);
            for y in viewport.y..viewport.y + viewport.height {
                assert!(
                    (viewport.x..viewport.x + viewport.width)
                        .any(|x| full[((y * 256 + x) * 4) as usize] > 0),
                    "empty horizontal strip {viewport:?}/{y}"
                );
            }
        }
    }
}
