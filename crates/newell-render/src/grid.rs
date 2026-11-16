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

impl GridSnapshot {
    /// Re-express the existing XY grid shader's coordinates in a principal support frame.
    #[must_use]
    pub fn on_origin_plane(self, plane: newell_ui::OriginPlane) -> Self {
        self.in_basis(plane.basis().map(|axis| axis.map(f32::from)))
    }

    fn in_basis(mut self, basis: [[f32; 3]; 3]) -> Self {
        let local = |world: [f32; 4]| {
            let mut result = world;
            for (i, axis) in basis.iter().enumerate() {
                result[i] = (0..3).map(|j| axis[j] * world[j]).sum();
            }
            result
        };
        self.inverse_view_projection = self.inverse_view_projection.map(local);
        self.ray_origin = local(self.ray_origin);
        let world_to_clip = self.view_projection;
        for (i, axis) in basis.iter().enumerate() {
            self.view_projection[i] =
                std::array::from_fn(|row| (0..3).map(|j| world_to_clip[j][row] * axis[j]).sum());
        }
        self
    }
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
        with_grid_readback(check_grid_pixels);
    }

    #[test]
    #[ignore = "needs a GPU"]
    fn sketch_grid_on_xz_has_the_same_plane_local_pixel_coverage_as_xy() {
        with_grid_readback(|_, scene, render| {
            let local = grid_camera(false, 0.0, 2.0, 4.0, 4.0 / 128.0);
            let plane = newell_ui::OriginPlane::XZ;
            let basis = plane.basis().map(|axis| axis.map(f32::from));
            let inverse = std::array::from_fn(|i| std::array::from_fn(|j| basis[j][i]));
            let world = local.in_basis(inverse);
            let expected = render(scene, Some(local), false);
            assert_ne!(expected, render(scene, None, false));
            assert_ne!(expected, render(scene, Some(world), false));
            assert_eq!(
                expected,
                render(scene, Some(world.on_origin_plane(plane)), false)
            );
        });
    }

    fn with_grid_readback(
        check: impl FnOnce(&wgpu::Device, &mut crate::scene::Scene, &GridReadback<'_>),
    ) {
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

        check(&device, &mut scene, &render);
    }

    type GridReadback<'a> =
        dyn Fn(&crate::scene::Scene, Option<super::GridSnapshot>, bool) -> Vec<u8> + 'a;

    fn check_grid_pixels(
        device: &wgpu::Device,
        scene: &mut crate::scene::Scene,
        render: &GridReadback<'_>,
    ) {
        let empty = render(scene, None, false);
        for scale in [0.001_f32, 1.0, 100.0, 1e6] {
            // Keep the plane outside the forward-edge fade at every zoom.
            let eye_distance = 0.5_f32.max(scale / 64.0);
            let snapshot = super::GridSnapshot {
                inverse_view_projection: [
                    [scale, 0., 0., 0.],
                    [0., scale, 0., 0.],
                    [0., 0., 1., 0.],
                    [0., 0., -eye_distance, 1.],
                ],
                view_projection: [
                    [1. / scale, 0., 0., 0.],
                    [0., 1. / scale, 0., 0.],
                    [0., 0., 0.5 / eye_distance, 0.],
                    [0., 0., 0.5, 1.],
                ],
                dark: true,
                ray_origin: [0., 0., -eye_distance, 0.],
                world_per_pixel: scale / 128.0,
            };
            let visible = render(scene, Some(snapshot), false);
            assert_ne!(visible, empty, "scale {scale}");
            assert_eq!(render(scene, Some(snapshot), true), empty, "depth {scale}");
            let light = super::GridSnapshot {
                dark: false,
                ..snapshot
            };
            let light_background = render(scene, Some(light), true);
            assert!(light_background.iter().all(|v| *v == 255));
            assert_ne!(
                render(scene, Some(light), false),
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
            let shifted = render(scene, Some(zoom), false);
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
                    render(scene, Some(clipped), false),
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
                let perspective_pixels = render(scene, Some(perspective), false);
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
                render(scene, Some(looking_away), false) == empty,
                "grid behind camera"
            );
            // Reversing a zoom produces the exact original image: no retained level state.
            assert_eq!(render(scene, Some(snapshot), false), visible);
        }
        // Both zero-coordinate lines must stay continuous while the nested
        // spacing family crosses each level boundary. At the crossing,
        // max-coverage blending keeps the two lines from compositing brighter.
        let mut cross_center = None;
        for desired in [
            0.1999_f32, 0.2001, 0.4471, 0.4473, 0.9999, 1.0001, 2.2359, 2.2363, 4.9999, 5.0001,
            11.179, 11.181, 24.9999, 25.0001, 55.899, 55.905, 124.9999, 125.0001,
        ] {
            let world_per_pixel = desired / 120.0;
            let footprint_world = world_per_pixel * 128.0;
            let snapshot = super::GridSnapshot {
                inverse_view_projection: [
                    [footprint_world, 0., 0., 0.],
                    [0., footprint_world, 0., 0.],
                    [0., 0., 1., 0.],
                    [0., 0., -2., 1.],
                ],
                view_projection: [
                    [1.0 / footprint_world, 0., 0., 0.],
                    [0., 1.0 / footprint_world, 0., 0.],
                    [0., 0., 0.25, 0.],
                    [0., 0., 0.5, 1.],
                ],
                dark: true,
                ray_origin: [0., 0., -2., 0.],
                world_per_pixel,
            };
            let cross = render(scene, Some(snapshot), false);
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
            scene.set_viewport(device, viewport);
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
            let full = render(scene, Some(snapshot), false);
            let occlusion = crate::SceneViewport::new(80, 64, 32, 64);
            scene.set_occlusions(vec![occlusion]);
            let split = render(scene, Some(snapshot), false);
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
    // Camera rotated about X, with matching forward and inverse projections.
    // The auxiliary depth interval is [0, 2] in ortho and [1, 2] in perspective.
    fn grid_camera(
        perspective: bool,
        tilt: f32,
        eye_z: f32,
        scale: f32,
        world_per_pixel: f32,
    ) -> super::GridSnapshot {
        let (s, c) = tilt.sin_cos();
        let (inverse, vp) = if perspective {
            (
                [
                    [scale, 0., 0., 0.],
                    [0., c * scale, s * scale, 0.],
                    [0., 0., -eye_z / 2., -0.5],
                    [0., s, eye_z - c, 1.],
                ],
                [
                    [1. / scale, 0., 0., 0.],
                    [0., c / scale, 2. * s, s],
                    [0., s / scale, -2. * c, -c],
                    [0., -s * eye_z / scale, 2. * c * eye_z - 2., c * eye_z],
                ],
            )
        } else {
            (
                [
                    [scale, 0., 0., 0.],
                    [0., c * scale, s * scale, 0.],
                    [0., 2. * s, -2. * c, 0.],
                    [0., 0., eye_z, 1.],
                ],
                [
                    [1. / scale, 0., 0., 0.],
                    [0., c / scale, s / 2., 0.],
                    [0., s / scale, -c / 2., 0.],
                    [0., -s * eye_z / scale, c * eye_z / 2., 1.],
                ],
            )
        };
        super::GridSnapshot {
            inverse_view_projection: inverse,
            view_projection: vp,
            dark: true,
            ray_origin: [0., 0., eye_z, if perspective { 1. } else { 0. }],
            world_per_pixel,
        }
    }

    fn grid_linear(value: u8) -> f32 {
        let srgb = f32::from(value) / 255.;
        if srgb <= 0.04045 {
            srgb / 12.92
        } else {
            ((srgb + 0.055) / 1.055).powf(2.4)
        }
    }

    fn grid_pixel(image: &[u8], x: usize, y: usize) -> u8 {
        image[(y * 256 + x) * 4]
    }

    #[test]
    #[ignore = "needs a GPU"]
    fn grid_forward_edge_fades_in_both_projections() {
        with_grid_readback(|_, scene, render| {
            let empty = render(scene, None, false);
            for (tilt, eye_z) in [(0.4_f32, 0.), (0.8, 0.)] {
                let snapshot = grid_camera(false, tilt, eye_z, 8., 0.0625);
                let image = render(scene, Some(snapshot), false);
                let edge = 128. * (1. + eye_z / (8. * tilt.sin()));
                let at = |offset: f32| grid_pixel(&image, 128, (edge - offset).floor() as usize);
                let [first, second, third, full] = [8., 20., 34., 60.].map(at);
                assert_eq!(at(-5.), 0, "behind orthographic camera");
                assert!(
                    0 < first && first < second && second < third && third < full,
                    "orthographic edge did not fade: {first}, {second}, {third}, {full}"
                );
                assert!(at(52.).abs_diff(full) <= 1);
                let mut clipped = snapshot;
                clipped.view_projection[3][2] += 10.;
                assert_eq!(render(scene, Some(clipped), false), image);
            }

            let face = |z| {
                render(
                    scene,
                    Some(grid_camera(false, 0.00001, z, 8., 0.0625)),
                    false,
                )
            };
            assert_eq!(face(-0.001), empty);
            let faint = face(0.02);
            let full = face(1.);
            assert!(grid_pixel(&faint, 128, 140) > 0);
            assert!(grid_pixel(&faint, 128, 140) < grid_pixel(&full, 128, 140));
            let mut edge_on = grid_camera(false, 0., 0., 8., 0.0625);
            edge_on.inverse_view_projection[1] = [0., 0., 8., 0.];
            edge_on.inverse_view_projection[2] = [0., 2., 0., 0.];
            edge_on.view_projection[1] = [0., 0., 0.5, 0.];
            edge_on.view_projection[2] = [0., 0.125, 0., 0.];
            assert_eq!(render(scene, Some(edge_on), false), empty);

            let tilt = 1.4_f32;
            let snapshot = grid_camera(true, tilt, 1., 0.6, 0.05);
            let image = render(scene, Some(snapshot), false);
            let horizon = 128. * (1. - tilt.cos() / (0.6 * tilt.sin()));
            let at = |offset: f32| grid_pixel(&image, 128, (horizon + offset).floor() as usize);
            assert_eq!(at(-8.), 0);
            assert_eq!(at(1.), 0);
            assert!(at(10.) > 0 && at(10.) < at(45.), "perspective horizon");
            let mut clipped = snapshot;
            clipped.view_projection[3][2] += 10.;
            assert_eq!(render(scene, Some(clipped), false), image);
            let away = grid_camera(true, std::f32::consts::PI, 1., 0.6, 0.05);
            assert_eq!(render(scene, Some(away), false), empty);
        });
    }

    #[test]
    #[ignore = "needs a GPU"]
    fn grid_zoom_blends_only_nested_lines_in_both_projections() {
        with_grid_readback(|_, scene, render| {
            for perspective in [false, true] {
                let mut snapshot = grid_camera(perspective, 0., 1., 12., 2. / 120.);
                for boundary in [0.2, 1., 5.] {
                    snapshot.world_per_pixel = boundary * (1. - 0.0001) / 120.;
                    let before = render(scene, Some(snapshot), false);
                    snapshot.world_per_pixel = boundary * (1. + 0.0001) / 120.;
                    let after = render(scene, Some(snapshot), false);
                    assert!(before.iter().zip(&after).all(|(a, b)| a.abs_diff(*b) <= 2));
                    snapshot.world_per_pixel = boundary * (1. - 0.0001) / 120.;
                    assert_eq!(render(scene, Some(snapshot), false), before);
                }
                // Check world anchoring and the nested candidates after pan.
                snapshot.world_per_pixel = 2. / 120.;
                for pan in [0., 0.375] {
                    snapshot.inverse_view_projection[3][0] = pan;
                    snapshot.inverse_view_projection[2][0] =
                        if perspective { -pan / 2. } else { 0. };
                    snapshot.view_projection[3][0] = -pan / 12.;
                    snapshot.ray_origin[0] = pan;
                    let image = render(scene, Some(snapshot), false);
                    for vertical in [false, true] {
                        let shift = if vertical { 0. } else { pan };
                        let at = |i| {
                            if vertical {
                                grid_pixel(&image, 140, i)
                            } else {
                                grid_pixel(&image, i, 140)
                            }
                        };
                        let world_at = |i: usize| {
                            let world = (i as f32 + 0.5 - 128.) * 12. / 128.;
                            if vertical { -world } else { world + shift }
                        };
                        for i in 1..255 {
                            let center = at(i);
                            if center > 3 && center >= at(i - 1) && center >= at(i + 1) {
                                let world = world_at(i);
                                assert!(
                                    (world - world.round()).abs() <= 12. / 128.,
                                    "off-lattice line at {world} (perspective={perspective}, vertical={vertical}, pan={pan}, pixels={:?})",
                                    [at(i - 1), center, at(i + 1)]
                                );
                            }
                        }
                        let peak = |line: f32| {
                            (1..255)
                                .filter(|i| (world_at(*i) - line).abs() <= 12. / 128.)
                                .map(at)
                                .max()
                                .unwrap()
                        };
                        assert!(peak(1.) > 3 && peak(5.) > peak(1.));
                    }
                }
            }
        });
    }

    #[test]
    #[ignore = "needs a GPU"]
    fn grid_marks_every_fifth_line_and_softens_with_distance() {
        with_grid_readback(|_, scene, render| {
            let empty = render(scene, None, false);
            for perspective in [false, true] {
                let scale = if perspective { 4. } else { 16. };
                let snapshot = grid_camera(perspective, 0., 4., scale, 2. / 120.);
                let near = render(scene, Some(snapshot), false);
                for vertical in [false, true] {
                    let at = |i| {
                        if vertical {
                            grid_pixel(&near, 133, i)
                        } else {
                            grid_pixel(&near, i, 133)
                        }
                    };
                    for (major, minor) in [(48, 64), (208, 192)] {
                        assert!(at(major) > at(minor));
                        assert_eq!(at(major + 1), 0);
                    }
                }
            }

            let perspective_at = |distance, shift| {
                let scale = 16. / distance;
                let mut snapshot = grid_camera(true, 0., distance, scale, 2. / 120.);
                snapshot.ray_origin[0] = shift;
                snapshot.inverse_view_projection[3][0] = shift;
                snapshot.inverse_view_projection[2][0] = -shift / 2.;
                snapshot.view_projection[3][0] = -shift / scale;
                render(scene, Some(snapshot), false)
            };
            let near = perspective_at(4., 0.);
            let distant = perspective_at(24., 0.);
            assert!(grid_pixel(&distant, 48, 133) < grid_pixel(&near, 48, 133));
            assert!(grid_pixel(&distant, 48, 133) > 0);
            assert_eq!(perspective_at(36., 0.), empty);
            // One pixel from the major center is outside the near profile.
            // The distant profile softens beyond it without moving half coverage.
            let near_shoulder = perspective_at(4., -0.0625);
            let distant_shoulder = perspective_at(24., -0.0625);
            assert_eq!(grid_pixel(&near_shoulder, 49, 133), 0);
            assert!(grid_pixel(&distant_shoulder, 49, 133) > 0);

            for (distance, center) in [(4., near_shoulder), (24., distant_shoulder)] {
                let half_width = perspective_at(distance, 0.015625);
                let ratio = grid_linear(grid_pixel(&half_width, 48, 133))
                    / grid_linear(grid_pixel(&center, 48, 133));
                assert!(
                    (ratio - 0.5).abs() <= 0.07,
                    "major width at distance {distance}: {ratio}"
                );
            }

            let tilted = grid_camera(false, 1.2, 20., 16., 2. / 120.);
            let image = render(scene, Some(tilted), false);
            assert!(grid_pixel(&image, 48, 55) < grid_pixel(&image, 48, 133));
            assert!(grid_pixel(&image, 48, 55) > 0);
            assert_eq!(grid_pixel(&image, 48, 10), 0);
        });
    }

    #[test]
    #[ignore = "needs a GPU"]
    fn grid_fades_equally_above_and_below_the_projected_origin() {
        with_grid_readback(|_, scene, render| {
            let tilt = 0.8_f32;
            let ortho = grid_camera(false, tilt, 4. / tilt.sin(), 8., 0.0625);
            let tilt = 1.2_f32;
            let (sin, cos) = tilt.sin_cos();
            // Shift the perspective eye along its up vector to put the world
            // origin at row 144 rather than the viewport midpoint at row 128.
            let eye_y = -3. * sin + 0.3 * cos;
            let eye_z = 3. * cos + 0.3 * sin;
            let mut perspective = grid_camera(true, tilt, eye_z, 0.8, 3. / 256.);
            perspective.ray_origin[1] = eye_y;
            perspective.inverse_view_projection[2][1] = -eye_y / 2.;
            perspective.inverse_view_projection[3][1] += eye_y;
            for row in 0..4 {
                perspective.view_projection[3][row] -= eye_y * perspective.view_projection[1][row];
            }

            for (snapshot, origin_y, offsets) in [
                (ortho, 192, [6, 22, 38, 62]),
                (perspective, 144, [6, 56, 62, 75]),
            ] {
                let image = render(scene, Some(snapshot), false);
                let mut previous = 255;
                for offset in offsets {
                    let above = grid_pixel(&image, 128, origin_y - offset - 1);
                    let below = grid_pixel(&image, 128, origin_y + offset);
                    assert!(
                        // Perspective changes the local line coverage independently of the fade.
                        above.abs_diff(below) <= 5,
                        "asymmetric fade at offset {offset}: {above} above, {below} below"
                    );
                    assert!(above < previous, "fade stalled at offset {offset}: {above}");
                    previous = above;
                }
                let outer = offsets[3];
                assert_eq!(previous, 0, "visible above symmetric fade");
                assert_eq!(grid_pixel(&image, 128, origin_y + outer), 0);
            }
        });
    }

    #[test]
    #[ignore = "needs a GPU"]
    fn grid_subdivides_by_five_with_two_nominal_widths() {
        with_grid_readback(|_, scene, render| {
            for perspective in [false, true] {
                for pitch in [0.2_f32, 1., 5.] {
                    for progress in [0.1_f32, 0.16, 0.49, 0.51, 0.84, 0.9] {
                        let target = pitch * 5_f32.powf(1. - progress);
                        let render_phase = |phase: f32| {
                            // Eight pixels per fine interval. Translate the
                            // camera to measure the same lines at subpixel phases.
                            let extent = pitch * 16.;
                            let eye_z = pitch * 4.;
                            let scale = if perspective { extent / eye_z } else { extent };
                            let mut snapshot =
                                grid_camera(perspective, 0., eye_z, scale, target / 120.);
                            let shift = (phase - 0.5) * extent / 128.;
                            snapshot.ray_origin[0] = shift;
                            snapshot.ray_origin[1] = -shift;
                            snapshot.inverse_view_projection[3][0] = shift;
                            snapshot.inverse_view_projection[3][1] = -shift;
                            if perspective {
                                snapshot.inverse_view_projection[2][0] = -shift / 2.;
                                snapshot.inverse_view_projection[2][1] = shift / 2.;
                            }
                            snapshot.view_projection[3][0] = -shift / scale;
                            snapshot.view_projection[3][1] = shift / scale;
                            render(scene, Some(snapshot), false)
                        };
                        let center = render_phase(0.);
                        let fine_edge = render_phase(0.5);
                        let major_edge = render_phase(0.625);
                        let opacity = ((progress - 0.15) / 0.7).clamp(0., 1.);
                        let opacity = opacity * opacity * (3. - 2. * opacity);
                        // The fixture packs two full groups into 256 pixels,
                        // so the eight-pixel pitch activates density filtering.
                        let fine_density = 0.6_f32.powi(2) * (3. - 2. * 0.6);
                        for vertical in [false, true] {
                            let sample = |image: &[u8], offset: isize| {
                                let i = (128 + offset) as usize;
                                grid_linear(if vertical {
                                    grid_pixel(image, 130, i)
                                } else {
                                    grid_pixel(image, i, 130)
                                })
                            };
                            for index in -10_isize..=10 {
                                let offset = index * 8;
                                let peak = sample(&center, offset);
                                if index % 5 != 0 && opacity == 0. {
                                    assert_eq!(peak, 0., "inserted line visible before transition");
                                    continue;
                                }
                                let major = index % 25 == 0 || (index % 5 == 0 && progress >= 0.5);
                                let expected = if index % 5 == 0 {
                                    1. - opacity * (1. - fine_density)
                                } else {
                                    opacity * fine_density
                                };
                                if expected >= 0.01 {
                                    assert!(
                                        peak > 0.,
                                        "missing line {index} at progress {progress}"
                                    );
                                }
                                if !perspective || index.abs() <= 4 {
                                    assert!(
                                        (peak - 0.20 * expected).abs() <= 0.005,
                                        "line {index} opacity at progress {progress}: {peak}"
                                    );
                                }
                                // Very faint insertion samples are quantized.
                                if peak >= 0.02 {
                                    let image = if major { &major_edge } else { &fine_edge };
                                    let ratio = sample(image, offset) / peak;
                                    assert!(
                                        (ratio - 0.5).abs() <= 0.08,
                                        "line {index} width at progress {progress}: {ratio}"
                                    );
                                }
                                assert_eq!(sample(&center, offset + 4), 0., "off-lattice line");
                            }
                        }
                    }
                }
            }
        });
    }
}
