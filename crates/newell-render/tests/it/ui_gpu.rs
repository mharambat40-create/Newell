// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Test helpers call `unwrap` for clear failure messages. The `clippy.toml` allowance only covers `#[test]` bodies.
#![allow(clippy::unwrap_used)]

use newell_render::ui::{UiOverlay, resolve_style};
use newell_render::{GLYPH_CELL_HEIGHT, GLYPH_CELL_WIDTH, GlyphAtlas, load_font};
use newell_ui::{GridMetrics, demo_grid};

const FONT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/FiraCode-Regular.ttf"
);
const BACKGROUND: [u8; 4] = [20, 20, 23, 255];

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};
    struct W(std::thread::Thread);
    impl Wake for W {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(W(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut f = std::pin::pin!(f);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::park(),
        }
    }
}

/// Renders the demo grid offscreen through `UiOverlay`; returns (width, height, RGBA8 rows, draw calls).
fn render_demo() -> (u32, u32, Vec<u8>, u32) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter =
        block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();
    let (device, queue) =
        block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();

    let font = load_font(std::fs::read(FONT_PATH).unwrap()).unwrap();
    let grid = demo_grid().unwrap();
    let (width, height) = (
        grid.cols() * GLYPH_CELL_WIDTH,
        grid.rows() * GLYPH_CELL_HEIGHT,
    );
    let format = wgpu::TextureFormat::Rgba8Unorm;

    let mut overlay = UiOverlay::new(&device, &queue, format, GlyphAtlas::build(&font).unwrap());
    overlay.set_grid(
        &device,
        &queue,
        &grid,
        GridMetrics::new(GLYPH_CELL_WIDTH, GLYPH_CELL_HEIGHT).unwrap(),
    );
    overlay.set_viewport(&queue, width, height);

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
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
    let row_bytes = width * 4;
    let padded = row_bytes.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(padded * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let [r, g, b, a] = BACKGROUND.map(|c| f64::from(c) / 255.0);
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    let draws = overlay.draw(&mut pass);
    drop(pass);
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.unwrap());
    device.poll(wgpu::PollType::Wait).unwrap();
    let mapped = readback.slice(..).get_mapped_range();
    let mut pixels = Vec::with_capacity((row_bytes * height) as usize);
    for row in mapped.chunks(padded as usize).take(height as usize) {
        pixels.extend_from_slice(&row[..row_bytes as usize]);
    }
    (width, height, pixels, draws)
}

fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * width + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

/// Manual: `cargo test -p newell-render --test it ui_gpu -- --ignored`.
/// Set `NEWELL_UI_DUMP=/path/out.ppm` to keep the rendered frame for visual inspection.
#[test]
#[ignore = "needs a GPU"]
fn demo_grid_renders_with_one_draw_call() {
    let (width, height, pixels, draws) = render_demo();
    assert_eq!(draws, 1, "the whole overlay is one draw call");

    if let Ok(path) = std::env::var("NEWELL_UI_DUMP") {
        let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
        for px in pixels.chunks(4) {
            ppm.extend_from_slice(&px[..3]);
        }
        std::fs::write(path, ppm).unwrap();
    }

    // 1:1 sampling: every pixel is exactly the atlas coverage blended over the background. This
    // rules out bleed from neighbouring slots and ink outside the cell.
    let atlas = GlyphAtlas::build(&load_font(std::fs::read(FONT_PATH).unwrap()).unwrap()).unwrap();
    let grid = demo_grid().unwrap();
    for row in 0..grid.rows() {
        for col in 0..grid.cols() {
            let cell = grid.get(col, row).unwrap();
            let color = resolve_style(cell.style);
            let rect = match cell.content {
                newell_ui::CellContent::Glyph(ch) => atlas.get(ch).map(|g| g.rect),
                newell_ui::CellContent::Empty => None,
            };
            for y in 0..GLYPH_CELL_HEIGHT {
                for x in 0..GLYPH_CELL_WIDTH {
                    let cov = rect.map_or(0, |r| {
                        atlas.coverage()[((r.y + y) * atlas.width() + r.x + x) as usize]
                    });
                    let got = pixel(
                        &pixels,
                        width,
                        col * GLYPH_CELL_WIDTH + x,
                        row * GLYPH_CELL_HEIGHT + y,
                    );
                    for c in 0..3 {
                        let expect = f32::from(BACKGROUND[c])
                            + (f32::from(color[c]) - f32::from(BACKGROUND[c])) * f32::from(cov)
                                / 255.0;
                        assert!(
                            (f32::from(got[c]) - expect).abs() <= 2.0,
                            "cell ({col},{row}) px ({x},{y}) ch {c}: got {} expected {expect}",
                            got[c]
                        );
                    }
                }
            }
        }
    }

    let cell = |col: u32, row: u32| (col * GLYPH_CELL_WIDTH, row * GLYPH_CELL_HEIGHT);
    // An empty cell stays background (transparent handling), a glyph cell has ink.
    let (ex, ey) = cell(47, 16);
    for y in 0..GLYPH_CELL_HEIGHT {
        for x in 0..GLYPH_CELL_WIDTH {
            assert_eq!(pixel(&pixels, width, ex + x, ey + y), BACKGROUND);
        }
    }
    let (nx, ny) = cell(0, 0);
    let lit = (0..GLYPH_CELL_HEIGHT)
        .flat_map(|y| (0..GLYPH_CELL_WIDTH).map(move |x| (x, y)))
        .filter(|&(x, y)| pixel(&pixels, width, nx + x, ny + y) != BACKGROUND)
        .count();
    assert!(lit > 20, "'N' should cover pixels, got {lit}");

    // Horizontal box strokes of adjacent cells (row 16, cols 1 and 2) join across the cell boundary.
    let (bx, by) = cell(1, 16);
    let mut stroke_rows = 0;
    for y in 0..GLYPH_CELL_HEIGHT {
        let left = pixel(&pixels, width, bx + GLYPH_CELL_WIDTH - 1, by + y);
        let right = pixel(&pixels, width, bx + GLYPH_CELL_WIDTH, by + y);
        assert_eq!(left, right, "gap at stroke row {y}");
        stroke_rows += usize::from(left != BACKGROUND);
    }
    assert!(stroke_rows > 0, "═ should have ink");
}
