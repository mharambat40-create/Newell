// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Test helpers call `unwrap` for clear failure messages. The `clippy.toml` allowance only covers `#[test]` bodies.
#![allow(clippy::unwrap_used)]

use newell_render::ui::{Theme, UiOverlay};
use newell_render::{GlyphAtlas, load_font};
use newell_ui::{Cell, CellContent, CellGrid, DEFAULT_K, GridMetrics, StyleId, demo_grid};

const FONT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/FiraCode-Regular.ttf"
);

fn metrics() -> GridMetrics {
    GridMetrics::from_k(DEFAULT_K).unwrap()
}

/// The cell size in physical pixels, from the one metrics value.
fn cell_size() -> (u32, u32) {
    let m = metrics();
    (m.cell_width_px(), m.cell_height_px())
}

/// Frame produced by [`render`]: width, height, RGBA8 pixels, draw calls.
type Frame = (u32, u32, Vec<u8>, u32);

fn gray(c: f32) -> u8 {
    (c * 255.0).round() as u8
}

/// Opaque RGBA of the theme background / foreground.
fn background(theme: Theme) -> [u8; 4] {
    let [v, ..] = theme.background().map(gray);
    [v, v, v, 255]
}

fn foreground(theme: Theme) -> [u8; 4] {
    let [v, ..] = theme.foreground().map(gray);
    [v, v, v, 255]
}

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
    render(&demo_grid().unwrap(), &[Theme::Dark]).pop().unwrap()
}

/// Renders `grid` once per entry of `themes` with the same overlay, changing only the theme
/// between frames (no `set_grid` in between). The frame is cleared to the theme background,
/// as the renderer does.
fn render(grid: &CellGrid, themes: &[Theme]) -> Vec<Frame> {
    render_in(grid, themes, None)
}

/// As [`render`], into a `surface` of `(width, height)` pixels instead of the grid's own size
/// (a smaller one crops the grid).
fn render_in(grid: &CellGrid, themes: &[Theme], surface: Option<(u32, u32)>) -> Vec<Frame> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter =
        block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();
    let (device, queue) =
        block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();

    let font = load_font(std::fs::read(FONT_PATH).unwrap()).unwrap();
    let (cell_w, cell_h) = cell_size();
    let (width, height) = surface.unwrap_or((grid.cols() * cell_w, grid.rows() * cell_h));
    let format = wgpu::TextureFormat::Rgba8Unorm;

    let atlas = GlyphAtlas::build(&font, metrics()).unwrap();
    let mut overlay = UiOverlay::new(&device, &queue, format, atlas);
    overlay.set_grid(&device, &queue, grid);
    overlay.set_viewport(&queue, width, height);

    themes
        .iter()
        .map(|&theme| {
            overlay.set_theme(&queue, theme);
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

            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let [r, g, b, a] = background(theme).map(|c| f64::from(c) / 255.0);
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
        })
        .collect()
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
    let (cell_w, cell_h) = cell_size();
    let theme = Theme::Dark;
    let (width, height, pixels, draws) = render_demo();
    let (bg, fg) = (background(theme), foreground(theme));
    assert_eq!(draws, 1, "the whole overlay is one draw call");

    if let Ok(path) = std::env::var("NEWELL_UI_DUMP") {
        let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
        for px in pixels.chunks(4) {
            ppm.extend_from_slice(&px[..3]);
        }
        std::fs::write(path, ppm).unwrap();
    }

    // 1:1 sampling: every pixel is exactly the atlas coverage blended between the two theme
    // colours, whatever the cell's style. This rules out bleed from neighbouring slots, ink
    // outside the cell and any colour other than the pair.
    let atlas = GlyphAtlas::build(
        &load_font(std::fs::read(FONT_PATH).unwrap()).unwrap(),
        metrics(),
    )
    .unwrap();
    let grid = demo_grid().unwrap();
    for row in 0..grid.rows() {
        for col in 0..grid.cols() {
            let cell = grid.get(col, row).unwrap();
            let (face, back) = (fg, bg);
            let rect = match cell.content {
                newell_ui::CellContent::Glyph(ch) => atlas.get(ch).map(|g| g.rect),
                newell_ui::CellContent::Empty => None,
            };
            for y in 0..cell_h {
                for x in 0..cell_w {
                    let cov = rect.map_or(0, |r| {
                        atlas.coverage()[((r.y + y) * atlas.width() + r.x + x) as usize]
                    });
                    let got = pixel(&pixels, width, col * cell_w + x, row * cell_h + y);
                    for c in 0..3 {
                        let expect = f32::from(back[c])
                            + (f32::from(face[c]) - f32::from(back[c])) * f32::from(cov) / 255.0;
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

    let cell = |col: u32, row: u32| (col * cell_w, row * cell_h);
    // An empty cell stays background (transparent handling), a glyph cell has ink.
    let (ex, ey) = cell(47, 16);
    for y in 0..cell_h {
        for x in 0..cell_w {
            assert_eq!(pixel(&pixels, width, ex + x, ey + y), bg);
        }
    }
    let (nx, ny) = cell(0, 0);
    let lit = (0..cell_h)
        .flat_map(|y| (0..cell_w).map(move |x| (x, y)))
        .filter(|&(x, y)| pixel(&pixels, width, nx + x, ny + y) != bg)
        .count();
    assert!(lit > 20, "'N' should cover pixels, got {lit}");

    // Horizontal box strokes of adjacent cells (row 16, cols 1 and 2) join across the cell boundary.
    let (bx, by) = cell(1, 16);
    let mut stroke_rows = 0;
    for y in 0..cell_h {
        let left = pixel(&pixels, width, bx + cell_w - 1, by + y);
        let right = pixel(&pixels, width, bx + cell_w, by + y);
        assert_eq!(left, right, "gap at stroke row {y}");
        stroke_rows += usize::from(left != bg);
    }
    assert!(stroke_rows > 0, "═ should have ink");
}

/// A grid exercising every style slot and empty cells.
fn style_grid() -> CellGrid {
    let mut grid = CellGrid::new(8, 1).unwrap();
    for (col, ch, style) in [
        (0, Some('B'), 0),
        (1, Some('t'), 1),
        (2, Some('n'), 2),
        (3, Some('X'), 3),
        (4, None, 1),
        (5, None, 0),
        (6, Some('█'), 3),
        (7, Some('Y'), u16::MAX),
    ] {
        *grid.get_mut(col, 0).unwrap() = Cell {
            content: ch.map_or(CellContent::Empty, CellContent::Glyph),
            style: StyleId::new(style),
            widget_id: None,
            shared_border: None,
        };
    }
    grid
}

fn cell_pixels(frame: &Frame, col: u32) -> Vec<[u8; 4]> {
    let (width, _, pixels, _) = frame;
    let (cell_w, cell_h) = cell_size();
    (0..cell_h)
        .flat_map(|y| (0..cell_w).map(move |x| (x, y)))
        .map(|(x, y)| pixel(pixels, *width, col * cell_w + x, y))
        .collect()
}

/// Manual: `cargo test -p newell-render --test it monochrome -- --ignored`.
/// Every pixel is a grey level (R = G = B, opaque): only black, white and the anti-aliased
/// blend between them reach the screen, in both themes and for every style slot.
#[test]
#[ignore = "needs a GPU"]
fn monochrome_output_has_no_colour_in_both_themes() {
    for grid in [style_grid(), demo_grid().unwrap()] {
        let frames = render(&grid, &[Theme::Light, Theme::Dark]);
        assert!(frames.iter().all(|f| f.3 == 1), "still one draw call");
        for frame in &frames {
            for px in frame.2.chunks(4) {
                assert!(px[0] == px[1] && px[1] == px[2] && px[3] == 255, "{px:?}");
            }
        }
    }
}

/// Manual: `cargo test -p newell-render --test it theme -- --ignored`.
/// Switching theme only exchanges foreground and background: the Light frame is the exact
/// inverse of the Dark frame, with no grid upload in between.
#[test]
#[ignore = "needs a GPU"]
fn light_theme_is_the_exact_inverse_of_dark() {
    for grid in [style_grid(), demo_grid().unwrap()] {
        let frames = render(&grid, &[Theme::Light, Theme::Dark, Theme::Light]);
        let (light, dark, again) = (&frames[0], &frames[1], &frames[2]);
        assert_eq!(light.2, again.2, "theme switching is reversible");
        for (l, d) in light.2.chunks(4).zip(dark.2.chunks(4)) {
            for c in 0..3 {
                assert!(
                    (i16::from(l[c]) + i16::from(d[c]) - 255).abs() <= 2,
                    "light {l:?} dark {d:?}"
                );
            }
        }
    }
}

/// Manual: `cargo test -p newell-render --test it state_styles -- --ignored`.
/// Styles never reach the GPU: the same glyphs with every style slot render identically, and
/// empty cells stay at the background, so state is carried by glyph structure only.
#[test]
#[ignore = "needs a GPU"]
fn style_slots_do_not_change_the_rendering() {
    let styled = style_grid();
    let mut plain = styled.clone();
    for col in 0..plain.cols() {
        plain.get_mut(col, 0).unwrap().style = StyleId::DEFAULT;
    }
    for theme in [Theme::Light, Theme::Dark] {
        let a = render(&styled, &[theme]).pop().unwrap();
        let b = render(&plain, &[theme]).pop().unwrap();
        assert_eq!(a.2, b.2);
        let bg = background(theme);
        assert!(cell_pixels(&a, 4).iter().all(|p| *p == bg), "empty cell");
        assert!(cell_pixels(&a, 5).iter().all(|p| *p == bg), "empty cell");
        assert!(cell_pixels(&a, 1).iter().any(|p| *p != bg), "glyph has ink");
    }
}

/// Manual: `cargo test -p newell-render --test it exact_colours -- --ignored`.
/// Light is exactly black on white and Dark exactly white on black: an empty cell is the pure
/// background, the interior of a full block is the pure foreground, nothing else is needed.
#[test]
#[ignore = "needs a GPU"]
fn exact_colours_in_both_themes() {
    let mut grid = CellGrid::new(2, 1).unwrap();
    grid.get_mut(0, 0).unwrap().content = CellContent::Glyph('█');
    let frames = render(&grid, &[Theme::Light, Theme::Dark]);
    for (frame, (fg, bg)) in frames.iter().zip([
        ([0, 0, 0, 255], [255, 255, 255, 255]),
        ([255; 4], [0, 0, 0, 255]),
    ]) {
        let (_, cell_h) = cell_size();
        // Rows well inside the line box of the block (top = k div 2, L = 8k).
        let pixels = cell_pixels(frame, 0);
        let (cell_w, _) = cell_size();
        let inside = |x: u32, y: u32| pixels[(y * cell_w + x) as usize];
        assert_eq!(inside(cell_w / 2, cell_h / 2), fg);
        assert!(cell_pixels(frame, 1).iter().all(|p| *p == bg));
    }
    assert_eq!(background(Theme::default()), [255; 4]);
    assert_eq!(foreground(Theme::default()), [0, 0, 0, 255]);
}

/// Manual: `cargo test -p newell-render --test it cropped -- --ignored`.
/// M2.17: a surface smaller than the composed 82 x 30 grid crops it: no panic, one draw call,
/// and the visible pixels equal the top-left of the uncropped frame.
#[test]
#[ignore = "needs a GPU"]
fn surface_smaller_than_the_grid_is_cropped_not_scaled() {
    let grid = demo_grid().unwrap();
    let (cell_w, cell_h) = cell_size();
    let full = render(&grid, &[Theme::Dark]).pop().unwrap();
    let (w, h) = (full.0 / 2 + 3, full.1 / 2 + 5);
    let cropped = render_in(&grid, &[Theme::Dark], Some((w, h)))
        .pop()
        .unwrap();
    assert_eq!((cropped.0, cropped.1, cropped.3), (w, h, 1));
    assert!(w > cell_w && h > cell_h);
    for y in 0..h {
        for x in 0..w {
            assert_eq!(
                pixel(&cropped.2, w, x, y),
                pixel(&full.2, full.0, x, y),
                "({x},{y})"
            );
        }
    }
}

/// Draws `overlay` once into a throw-away target and returns the draw calls recorded.
fn draw_calls(device: &wgpu::Device, overlay: &UiOverlay, width: u32, height: u32) -> u32 {
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
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
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
    });
    let draws = overlay.draw(&mut pass);
    drop(pass);
    draws
}

/// The run-time scale change of M2.17: another cell size is activated together with a grid
/// built for it, atlases are cached per size, and there is still one draw call.
#[test]
#[ignore = "needs a GPU"]
fn overlay_switches_cell_size_at_run_time_and_reuses_atlases() {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter =
        block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();
    let (device, queue) =
        block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let font = load_font(std::fs::read(FONT_PATH).unwrap()).unwrap();
    let m5 = GridMetrics::from_k(5).unwrap();
    let m2 = GridMetrics::from_k(2).unwrap();
    let grid = demo_grid().unwrap();

    let mut overlay = UiOverlay::new(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8Unorm,
        GlyphAtlas::build(&font, m5).unwrap(),
    );
    overlay.set_grid(&device, &queue, &grid);
    overlay.set_viewport(&queue, grid.cols() * 20, grid.rows() * 45);
    assert_eq!(overlay.cell_size(), (20, 45));
    assert_eq!(draw_calls(&device, &overlay, 64, 64), 1);

    // An unknown size is refused and nothing changes.
    assert!(!overlay.activate(&device, &queue, (8, 18), &grid));
    assert_eq!(overlay.cell_size(), (20, 45));

    // Add the k = 2 atlas: adding does not change what is drawn.
    overlay.add_atlas(&device, &queue, GlyphAtlas::build(&font, m2).unwrap());
    assert_eq!(overlay.atlas_count(), 2);
    assert_eq!(overlay.cell_size(), (20, 45));

    assert!(overlay.activate(&device, &queue, (8, 18), &grid));
    assert_eq!(overlay.cell_size(), (8, 18));
    assert_eq!(draw_calls(&device, &overlay, 64, 64), 1);

    // Back to k = 5 and k = 2 again: the cache serves both, nothing is added.
    for size in [(20, 45), (8, 18), (20, 45)] {
        assert!(overlay.activate(&device, &queue, size, &grid));
        overlay.add_atlas(&device, &queue, GlyphAtlas::build(&font, m2).unwrap());
        assert_eq!(overlay.atlas_count(), 2);
        assert_eq!(overlay.cell_size(), size);
        assert_eq!(draw_calls(&device, &overlay, 64, 64), 1);
    }
}
