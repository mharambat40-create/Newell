// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

#![allow(clippy::unwrap_used)]
use newell_render::{
    load_font,
    navigation_widgets::{NavOverlay, NavSnapshot},
};
use newell_ui::{GridMetrics, navigation_widgets::*};
fn wait<F: std::future::Future>(f: F) -> F::Output {
    use std::{
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
    };
    struct W(std::thread::Thread);
    impl Wake for W {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let w = Waker::from(Arc::new(W(std::thread::current())));
    let mut cx = Context::from_waker(&w);
    let mut f = std::pin::pin!(f);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::park(),
        }
    }
}
fn basis(n: V3) -> NavBasis {
    let f = mul(unit(n), -1.);
    let h = if f[0].abs() + f[1].abs() < 1e-12 {
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
fn render(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    overlay: &NavOverlay,
    format: wgpu::TextureFormat,
) -> (Vec<u8>, u32) {
    render_on(device, queue, overlay, format, wgpu::Color::BLACK)
}
fn render_on(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    overlay: &NavOverlay,
    format: wgpu::TextureFormat,
    clear: wgpu::Color,
) -> (Vec<u8>, u32) {
    render_layer(device, queue, overlay, format, clear, None)
}
fn render_layer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    overlay: &NavOverlay,
    format: wgpu::TextureFormat,
    clear: wgpu::Color,
    geometry: Option<&wgpu::RenderPipeline>,
) -> (Vec<u8>, u32) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("navigator test surface"),
        size: wgpu::Extent3d {
            width: 512,
            height: 512,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 512 * 512 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut e = device.create_command_encoder(&Default::default());
    drop(e.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    }));
    if let Some(pipeline) = geometry {
        let mut pass = e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("visible geometry beneath navigation"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
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
        pass.set_pipeline(pipeline);
        pass.draw(0..6, 0..1);
    }
    let draws = overlay.draw(&mut e, &view);
    e.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(2048),
                rows_per_image: Some(512),
            },
        },
        texture.size(),
    );
    queue.submit(Some(e.finish()));
    let (tx, rx) = std::sync::mpsc::channel();
    staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    device.poll(wgpu::PollType::Wait).unwrap();
    rx.recv().unwrap().unwrap();
    let pixels = staging.slice(..).get_mapped_range().to_vec();
    staging.unmap();
    (pixels, draws)
}
#[test]
#[ignore = "requires a GPU adapter"]
fn navigation_pipelines_draw_only_selected_kind_and_mask_popup_blanks() {
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = wait(instance.request_adapter(&Default::default())).unwrap();
    eprintln!("navigation adapter: {:?}", adapter.get_info());
    let (device, queue) = wait(adapter.request_device(&Default::default())).unwrap();
    let font =
        load_font(include_bytes!("../../../../assets/FiraCode-Regular.ttf").to_vec()).unwrap();
    for format in [
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureFormat::Rgba8Unorm,
    ] {
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut overlay = NavOverlay::new(&device, format);
        let error = wait(device.pop_error_scope());
        assert!(error.is_none(), "{error:?}");
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        for k in [2, 7] {
            for kind in [NavigatorKind::Gizmo, NavigatorKind::Cube] {
                for n in [[0., 0., 1.], [0., -1., 0.], [1., -1., 1.], [-1., 1., -1.]] {
                    let m = GridMetrics::from_k(k).unwrap();
                    let rect = PixelRect {
                        x: 0.,
                        y: 0.,
                        width: 512.,
                        height: 512.,
                    };
                    let layout = layout_navigation(rect, rect, m, true, kind, None);
                    let geometry = NavGeometry::new(layout, basis(n), m).unwrap();
                    let s = NavSnapshot {
                        geometry,
                        ui_masks: Vec::new(),
                        surface: [512, 512],
                        camera_revision: 1,
                        hovered: None,
                        hovered_cube: None,
                    };
                    overlay
                        .prepare(&device, &queue, &font, Some(s.clone()))
                        .unwrap();
                    let (pixels, draws) = render(&device, &queue, &overlay, format);
                    assert_eq!(draws, if kind == NavigatorKind::Gizmo { 2 } else { 3 });
                    let lit = pixels
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .filter(|c| c[0] > 0 || c[1] > 0 || c[2] > 0)
                        .count();
                    let error = wait(device.pop_error_scope());
                    assert!(error.is_none(), "{error:?}");
                    device.push_error_scope(wgpu::ErrorFilter::Validation);
                    assert!(lit > 100, "{kind:?} k={k} normal={n:?}, lit={lit}");
                    // Opaque materials must not depend on the scene/theme background.
                    let white = render_on(&device, &queue, &overlay, format, wgpu::Color::WHITE).0;
                    let tile = s.geometry.layout.tile.unwrap();
                    let q = s.geometry.q;
                    if n == [0., 0., 1.] {
                        let (x, y) = if kind == NavigatorKind::Cube {
                            (48. * q, 64. * q)
                        } else {
                            (79. * q, 53. * q)
                        };
                        let index = (((tile.y + y) as usize) * 512 + (tile.x + x) as usize) * 4;
                        assert_eq!(&pixels[index..index + 3], &white[index..index + 3]);
                        if kind == NavigatorKind::Gizmo && k == 2 {
                            let at = |x: usize, y: usize| {
                                (((tile.y as usize) + y) * 512 + (tile.x as usize) + x) * 4
                            };
                            let negative = at(32, 48);
                            let positive = at(64, 48);
                            // These points lie outside the finite glass square and all caps.
                            assert_eq!(&pixels[negative..negative + 3], &[0; 3]);
                            assert!(pixels[positive..positive + 3].iter().any(|v| *v > 10));
                        }
                    }
                    let repeated = render(&device, &queue, &overlay, format).0;
                    assert_eq!(pixels, repeated);
                    if format == wgpu::TextureFormat::Rgba8UnormSrgb && k == 2 {
                        std::fs::write(
                            std::env::temp_dir()
                                .join(format!("newell-navigation-{kind:?}-{n:?}.rgba")),
                            &pixels,
                        )
                        .unwrap();
                    }
                    let mut masked = s;
                    masked.ui_masks = vec![rect];
                    overlay
                        .prepare(&device, &queue, &font, Some(masked))
                        .unwrap();
                    assert!(
                        render(&device, &queue, &overlay, format)
                            .0
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .all(|c| c[..3] == [0; 3])
                    );
                    overlay.prepare(&device, &queue, &font, None).unwrap();
                    assert_eq!(render(&device, &queue, &overlay, format).1, 0);
                }
            }
        }
        assert!(wait(device.pop_error_scope()).is_none());
    }
}

#[test]
#[ignore = "requires a GPU adapter"]
fn all_six_axis_marker_materials_change_only_the_controller_hover_target() {
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = wait(instance.request_adapter(&Default::default())).unwrap();
    eprintln!("marker material adapter: {:?}", adapter.get_info());
    let (device, queue) = wait(adapter.request_device(&Default::default())).unwrap();
    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let font =
        load_font(include_bytes!("../../../../assets/FiraCode-Regular.ttf").to_vec()).unwrap();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut overlay = NavOverlay::new(&device, format);
    let m = GridMetrics::from_k(4).unwrap();
    let r = PixelRect {
        x: 0.,
        y: 0.,
        width: 512.,
        height: 512.,
    };
    let geometry = NavGeometry::new(
        layout_navigation(r, r, m, true, NavigatorKind::Gizmo, None),
        basis([1., -1., 1.]),
        m,
    )
    .unwrap();
    let mut snapshot = NavSnapshot {
        geometry,
        ui_masks: Vec::new(),
        surface: [512, 512],
        camera_revision: 1,
        hovered: None,
        hovered_cube: None,
    };
    overlay
        .prepare(&device, &queue, &font, Some(snapshot.clone()))
        .unwrap();
    let rest = render(&device, &queue, &overlay, format).0;
    let tile = snapshot.geometry.layout.tile.unwrap();
    let q = snapshot.geometry.q;
    let palette = [[245u8, 34, 53], [0, 181, 42], [48, 103, 245]];
    let pixel = |p: (f64, f64)| ((p.1 as usize) * 512 + p.0 as usize) * 4;
    for (direction, center) in snapshot.geometry.caps() {
        let cap = AxisCap::from_direction(direction).unwrap();
        snapshot.hovered = Some(cap);
        overlay
            .prepare(&device, &queue, &font, Some(snapshot.clone()))
            .unwrap();
        let hovered = render(&device, &queue, &overlay, format).0;
        let c = (tile.x + center[0], tile.y + center[1]);
        let radius = if cap.positive {
            POSITIVE_DISC_RADIUS * q
        } else {
            NEGATIVE_RING_OUTER_RADIUS * q
        };
        let sample = if cap.positive {
            (c.0, c.1 + 5. * q)
        } else {
            (c.0 + (NEGATIVE_RING_OUTER_RADIUS - 0.5) * q, c.1)
        };
        assert_eq!(snapshot.geometry.hit(sample), Some(NavPart::Axis(cap)));
        let i = pixel(sample);
        let axis = match cap.axis {
            SignedAxis::X => 0,
            SignedAxis::Y => 1,
            SignedAxis::Z => 2,
        };
        if cap.positive {
            for (a, b) in rest[i..i + 3].iter().zip(palette[axis]) {
                assert!((i16::from(*a) - i16::from(b)).abs() <= 2);
            }
            assert_eq!(
                &hovered[i..i + 3],
                &[0; 3],
                "hover interior must expose the scene"
            );
        } else {
            for (a, b) in hovered[i..i + 3].iter().zip(palette[axis]) {
                assert!((i16::from(*a) - i16::from(b)).abs() <= 2);
            }
            assert!(
                rest[i..i + 3]
                    .iter()
                    .zip(&hovered[i..i + 3])
                    .any(|(a, b)| a < b)
            );
            let middle = pixel(c);
            assert_eq!(&rest[middle..middle + 3], &[0; 3]);
            assert_eq!(&hovered[middle..middle + 3], &[0; 3]);
        }
        for y in 0..512 {
            for x in 0..512 {
                if ((x as f64 + 0.5) - c.0).hypot((y as f64 + 0.5) - c.1) > radius + 1. {
                    let i = (y * 512 + x) * 4;
                    assert_eq!(
                        &rest[i..i + 4],
                        &hovered[i..i + 4],
                        "unhovered pixel changed for {cap:?}"
                    );
                }
            }
        }
        if cap.positive && cap.axis == SignedAxis::X {
            std::fs::write(
                std::env::temp_dir().join("newell-focused-gizmo-hover.rgba"),
                &hovered,
            )
            .unwrap();
            std::fs::write(
                std::env::temp_dir().join("newell-focused-gizmo-rest.rgba"),
                &rest,
            )
            .unwrap();
        }
    }
    snapshot.hovered = None;
    overlay
        .prepare(&device, &queue, &font, Some(snapshot))
        .unwrap();
    assert_eq!(rest, render(&device, &queue, &overlay, format).0);
    assert!(wait(device.pop_error_scope()).is_none());
}

#[test]
#[ignore = "requires a GPU adapter"]
fn cube_bevels_are_transparent_until_the_hit_patch_is_hovered() {
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = wait(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = wait(adapter.request_device(&Default::default())).unwrap();
    let font =
        load_font(include_bytes!("../../../../assets/FiraCode-Regular.ttf").to_vec()).unwrap();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let m = GridMetrics::from_k(4).unwrap();
    let rect = PixelRect {
        x: 0.,
        y: 0.,
        width: 512.,
        height: 512.,
    };
    let geometry = NavGeometry::new(
        layout_navigation(rect, rect, m, true, NavigatorKind::Cube, None),
        basis([1., -1., 1.]),
        m,
    )
    .unwrap();
    let tile = geometry.layout.tile.unwrap();
    let mut snapshot = NavSnapshot {
        geometry,
        ui_masks: Vec::new(),
        surface: [512, 512],
        camera_revision: 1,
        hovered: None,
        hovered_cube: None,
    };
    let mut overlay = NavOverlay::new(&device, format);
    overlay
        .prepare(&device, &queue, &font, Some(snapshot.clone()))
        .unwrap();
    let rest = render(&device, &queue, &overlay, format).0;
    let background = &rest[(5 * 512 + 5) * 4..(5 * 512 + 5) * 4 + 4];
    let sample_patch = |dimension: usize| {
        cube_patches()
            .iter()
            .filter(|patch| patch.direction.dimensions() == dimension)
            .find_map(|patch| {
                let center = mul(
                    patch.vertices.iter().copied().fold([0.; 3], add),
                    1. / patch.vertices.len() as f64,
                );
                let projected = snapshot.geometry.project(center);
                let point = (tile.x + projected[0], tile.y + projected[1]);
                if snapshot.geometry.hit(point) != Some(NavPart::Cube(patch.direction)) {
                    return None;
                }
                let pixel = (point.0.round() as usize, point.1.round() as usize);
                let index = (pixel.1 * 512 + pixel.0) * 4;
                (rest[index..index + 4] == *background).then_some((patch.direction, point, index))
            })
            .expect("a front bevel patch must be transparent at rest and remain clickable")
    };
    let edge = sample_patch(2);
    let vertex = sample_patch(3);
    for (direction, point, _) in [edge, vertex] {
        assert_eq!(snapshot.geometry.hit(point), Some(NavPart::Cube(direction)));
    }
    snapshot.hovered_cube = Some(edge.0);
    overlay
        .prepare(&device, &queue, &font, Some(snapshot.clone()))
        .unwrap();
    let hovered = render(&device, &queue, &overlay, format).0;
    assert_ne!(
        &hovered[edge.2..edge.2 + 4],
        background,
        "hover restores the selected edge material"
    );
    assert_eq!(&hovered[vertex.2..vertex.2 + 4], background);
    snapshot.hovered_cube = Some(vertex.0);
    overlay
        .prepare(&device, &queue, &font, Some(snapshot))
        .unwrap();
    let vertex_hovered = render(&device, &queue, &overlay, format).0;
    assert_eq!(&vertex_hovered[edge.2..edge.2 + 4], background);
    assert_ne!(&vertex_hovered[vertex.2..vertex.2 + 4], background);
}

#[test]
#[ignore = "requires a GPU adapter"]
fn transparent_widgets_preserve_visible_geometry_and_hollow_interiors() {
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = wait(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = wait(adapter.request_device(&Default::default())).unwrap();
    let font =
        load_font(include_bytes!("../../../../assets/FiraCode-Regular.ttf").to_vec()).unwrap();
    let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label:Some("geometry under navigation"),source:wgpu::ShaderSource::Wgsl(r#"
        struct Out { @builtin(position) p:vec4<f32>, @location(0) color:vec3<f32> }
        @vertex fn vs(@builtin(vertex_index) i:u32)->Out {
            var points=array<vec2<f32>,6>(vec2<f32>(0,1),vec2<f32>(1,1),vec2<f32>(0,0),vec2<f32>(0,0),vec2<f32>(1,1),vec2<f32>(1,0));
            var o:Out;o.p=vec4<f32>(points[i],0.5,1);o.color=select(vec3<f32>(0.18,0.35,0.22),vec3<f32>(0.32,0.22,0.38),i>=3u);return o;
        }
        @fragment fn fs(v:Out)->@location(0) vec4<f32> {return vec4<f32>(v.color,1);}
        "#.into()),
    });
    for format in [
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureFormat::Rgba8Unorm,
    ] {
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("background triangle geometry"),
            layout: None,
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
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let mut overlay = NavOverlay::new(&device, format);
        let baseline = render_layer(
            &device,
            &queue,
            &overlay,
            format,
            wgpu::Color::BLACK,
            Some(&pipeline),
        )
        .0;
        let metrics = GridMetrics::from_k(4).unwrap();
        let rect = PixelRect {
            x: 0.,
            y: 0.,
            width: 512.,
            height: 512.,
        };
        for kind in [NavigatorKind::Gizmo, NavigatorKind::Cube] {
            let geometry = NavGeometry::new(
                layout_navigation(rect, rect, metrics, true, kind, None),
                basis([1., -1., 1.]),
                metrics,
            )
            .unwrap();
            let tile = geometry.layout.tile.unwrap();
            let q = geometry.q;
            let mut snapshot = NavSnapshot {
                geometry,
                ui_masks: vec![],
                surface: [512, 512],
                camera_revision: 1,
                hovered: None,
                hovered_cube: None,
            };
            overlay
                .prepare(&device, &queue, &font, Some(snapshot.clone()))
                .unwrap();
            let pixels = render_layer(
                &device,
                &queue,
                &overlay,
                format,
                wgpu::Color::BLACK,
                Some(&pipeline),
            )
            .0;
            let alpha = render_on(&device, &queue, &overlay, format, wgpu::Color::TRANSPARENT).0;
            let at = |p: (f64, f64)| (p.1 as usize * 512 + p.0 as usize) * 4;
            let corner = at((tile.x + 2., tile.y + 2.));
            assert_eq!(&alpha[corner..corner + 4], &[0; 4]);
            assert_eq!(&pixels[corner..corner + 4], &baseline[corner..corner + 4]);
            // Every fully uncovered pixel must preserve the actual pre-rendered triangles.
            for (i, a) in alpha.as_chunks::<4>().0.iter().enumerate() {
                if a[3] == 0 {
                    assert_eq!(&pixels[i * 4..i * 4 + 4], &baseline[i * 4..i * 4 + 4]);
                }
            }
            if kind == NavigatorKind::Cube {
                assert!(alpha.as_chunks::<4>().0.iter().any(|p| p[3] == 255)); // solid faces retained
            } else {
                for (direction, center) in snapshot.geometry.caps() {
                    let cap = AxisCap::from_direction(direction).unwrap();
                    let center = (tile.x + center[0], tile.y + center[1]);
                    let sample = if cap.positive {
                        (center.0, center.1 + 5. * q)
                    } else {
                        center
                    };
                    assert_eq!(snapshot.geometry.hit(center), Some(NavPart::Axis(cap)));
                    snapshot.hovered = Some(cap);
                    overlay
                        .prepare(&device, &queue, &font, Some(snapshot.clone()))
                        .unwrap();
                    let hovered = render_layer(
                        &device,
                        &queue,
                        &overlay,
                        format,
                        wgpu::Color::BLACK,
                        Some(&pipeline),
                    )
                    .0;
                    let transparent =
                        render_on(&device, &queue, &overlay, format, wgpu::Color::TRANSPARENT).0;
                    let i = at(sample);
                    assert_eq!(&transparent[i..i + 4], &[0; 4], "{cap:?}");
                    assert_eq!(&hovered[i..i + 4], &baseline[i..i + 4], "{cap:?}");
                    if cap.positive && cap.axis == SignedAxis::X && format.is_srgb() {
                        std::fs::write(
                            std::env::temp_dir().join("newell-transparent-hover-geometry.rgba"),
                            hovered,
                        )
                        .unwrap();
                    }
                }
            }
            if format.is_srgb() {
                std::fs::write(
                    std::env::temp_dir().join(format!("newell-transparent-{kind:?}-geometry.rgba")),
                    pixels,
                )
                .unwrap();
            }
            snapshot.ui_masks = vec![rect];
            overlay
                .prepare(&device, &queue, &font, Some(snapshot))
                .unwrap();
            assert_eq!(
                render_layer(
                    &device,
                    &queue,
                    &overlay,
                    format,
                    wgpu::Color::BLACK,
                    Some(&pipeline)
                )
                .0,
                baseline,
                "UI masks keep precedence"
            );
        }
    }
}
