// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Renderer: surface, device, queue, clear pass, present.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};

use newell_kernel::NewellMesh;
use newell_ui::{CellGrid, GridMetrics};

use crate::RenderError;
use ab_glyph::FontVec;

use crate::glyph_atlas::{GlyphAtlas, load_font};
use crate::scene::Scene;
use crate::surface::{FrameAction, SurfaceState, classify};
use crate::ui::{Theme, UiOverlay};
use crate::{CameraSnapshot, SceneViewport};

const FONT: &[u8] = include_bytes!("../../../assets/FiraCode-Regular.ttf");

/// Background clear colour of the frame: the theme background (pure black or white). The 3D
/// viewport is specified separately (`ARCHITECTURE.md` §3.6) and draws over this later.
fn clear_color(theme: Theme) -> wgpu::Color {
    let [r, g, b] = theme.background().map(f64::from);
    wgpu::Color { r, g, b, a: 1.0 }
}

/// Result of a successful `Renderer::render` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameOutcome {
    /// A frame was submitted and presented.
    Presented,
    /// Nothing was drawn (inactive surface, or no texture available this frame).
    Skipped,
}

/// Owns the surface, device and queue, and draws the viewport and the ASCII overlay.
pub struct Renderer {
    // Declaration order is drop order: the surface (which holds the window handle)
    // goes before the device that created it.
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    state: SurfaceState,
    ui: UiOverlay,
    /// The parsed font, kept to build the atlas of another `k` at run time.
    font: FontVec,
    /// Atlases rasterised since start-up (cached sizes are reused, not rebuilt).
    atlas_builds: u32,
    theme: Theme,
    /// The M3 scene is drawn before the transparent ASCII overlay.
    scene: Scene,
    /// Fed by both the uncaptured-error handler and the device-lost callback.
    gpu_errors: Receiver<RenderError>,
}

impl Renderer {
    /// `window` is an owned handle (e.g. `Arc<winit::window::Window>`), so the surface is
    /// `'static`. `width`/`height` are physical pixels. A zero size yields an inactive
    /// renderer until a non-zero `resize`. `metrics` fixes the cell size of the glyph atlas for
    /// the lifetime of the renderer.
    pub fn new(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
        metrics: GridMetrics,
    ) -> Result<Self, RenderError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance.create_surface(window)?;
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))?;
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;

        let (tx, gpu_errors) = mpsc::channel();
        let error_tx = tx.clone();
        // The default handler panics, which ARCHITECTURE.md §5 forbids.
        device.on_uncaptured_error(Box::new(move |error| {
            let _ = error_tx.send(error.into());
        }));
        device.set_device_lost_callback(move |reason, message| {
            let _ = tx.send(RenderError::DeviceLost(format!("{reason:?}: {message}")));
        });

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| caps.formats.first().copied())
            .ok_or(RenderError::NoSurfaceFormat)?;
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: Vec::new(),
        };
        let font = load_font(FONT.to_vec())?;
        let atlas = GlyphAtlas::build(&font, metrics)?;
        let ui = UiOverlay::new(&device, &queue, format, atlas);
        let scene = Scene::new(&device, format, (width, height));
        Ok(Self {
            surface,
            device,
            queue,
            config,
            state: SurfaceState::new(width, height),
            ui,
            font,
            atlas_builds: 1,
            theme: Theme::default(),
            scene,
            gpu_errors,
        })
    }

    /// Physical pixels. A zero dimension (minimised) deactivates the surface without error.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.state.resize(width, height);
        self.scene.resize(&self.device, (width, height));
    }

    /// Uploads an owned Newell mesh to the 3D viewport. BrepKIT types cannot cross this API.
    pub fn set_scene_mesh(&mut self, mesh: NewellMesh) {
        self.scene.set_mesh(&self.device, mesh);
    }

    /// Sets the physical-pixel rectangle in which the 3D scene is drawn.
    pub fn set_scene_viewport(&mut self, viewport: SceneViewport) {
        self.scene.set_viewport(viewport);
    }

    /// Masks one popup-sized rectangle from the scene while preserving the rest of its viewport.
    pub fn set_scene_occlusions(&mut self, occlusions: Vec<SceneViewport>) {
        self.scene.set_occlusions(occlusions);
    }

    /// Applies the View menu's shading index to the 3D scene.
    pub fn set_scene_shading(&mut self, shading: usize) {
        self.scene.set_shading(shading);
    }

    /// Consumes immutable camera matrices prepared by the window Controller.
    pub fn set_scene_camera(&mut self, camera: CameraSnapshot) {
        self.scene.set_camera(camera);
    }

    /// Replaces the ASCII overlay with `grid`, drawn from the top-left in whole cells of the
    /// renderer's metrics. The whole grid is uploaded; it is drawn by one draw call.
    pub fn set_ui_grid(&mut self, grid: &CellGrid) {
        self.ui.set_grid(&self.device, &self.queue, grid);
    }

    /// Builds (or finds in the cache) the glyph atlas of `metrics` without changing what is
    /// drawn. This is the only fallible step of a scale change: on `Err` nothing has changed.
    ///
    /// # Errors
    /// If the atlas cannot be built for `metrics`.
    pub fn prepare_metrics(&mut self, metrics: GridMetrics) -> Result<(), RenderError> {
        let size = (metrics.cell_width_px(), metrics.cell_height_px());
        if !self.ui.has_atlas(size) {
            let atlas = GlyphAtlas::build(&self.font, metrics)?;
            self.atlas_builds += 1;
            self.ui.add_atlas(&self.device, &self.queue, atlas);
        }
        Ok(())
    }

    /// Switches to the cell size of `metrics` and uploads `grid`, composed for those metrics,
    /// in one call: atlas, cell buffer and shader uniforms change together, so no frame mixes
    /// an old atlas with a new grid. The atlas is built only if this size has not been used
    /// before. A zero-sized or inactive surface is unaffected.
    ///
    /// # Errors
    /// If the atlas cannot be built; the previous metrics and grid stay in place.
    pub fn set_metrics(
        &mut self,
        metrics: GridMetrics,
        grid: &CellGrid,
    ) -> Result<(), RenderError> {
        self.prepare_metrics(metrics)?;
        let size = (metrics.cell_width_px(), metrics.cell_height_px());
        // `prepare_metrics` guarantees the atlas exists, so this always activates it.
        let _ = self.ui.activate(&self.device, &self.queue, size, grid);
        Ok(())
    }

    /// Cell size `(W, H)` the overlay is currently drawn with.
    pub fn cell_size(&self) -> (u32, u32) {
        self.ui.cell_size()
    }

    /// Atlases rasterised so far; a size that returns is served from the cache.
    pub fn atlas_builds(&self) -> u32 {
        self.atlas_builds
    }

    /// Sets the UI theme (`Light` or `Dark`). Only the shader's theme uniform and the clear
    /// colour change; the grid is not re-uploaded and the draw is still one call.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.ui.set_theme(&self.queue, theme);
    }

    /// Acquires, clears to the theme background, submits, presents. Returns `Skipped` when the
    /// surface is inactive or has nothing to draw to; errors on fatal or reported GPU errors.
    pub fn render(&mut self) -> Result<FrameOutcome, RenderError> {
        self.check_gpu_errors()?;
        if !self.state.is_active() {
            return Ok(FrameOutcome::Skipped);
        }
        if let Some((width, height)) = self.state.pending_configure() {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.state.mark_configured();
            self.ui.set_viewport(&self.queue, width, height);
        }

        let (frame, acquire) = match self.surface.get_current_texture() {
            Ok(frame) => {
                let suboptimal = frame.suboptimal;
                (Some(frame), Ok(suboptimal))
            }
            Err(error) => (None, Err(error)),
        };
        let action = classify(acquire);
        self.state.apply(&action);
        let frame = match (action, frame) {
            (FrameAction::Present | FrameAction::PresentAndReconfigure, Some(frame)) => frame,
            (FrameAction::Fatal(error), _) => return Err(error),
            _ => return Ok(FrameOutcome::Skipped),
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color(self.theme)),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        }));
        self.scene.draw(&self.queue, &mut encoder, &view);
        // Separate overlay pass: the viewport pass above stays untouched.
        let mut overlay = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ascii ui"),
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
        self.ui.draw(&mut overlay);
        drop(overlay);
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        self.check_gpu_errors()?;
        Ok(FrameOutcome::Presented)
    }

    fn check_gpu_errors(&self) -> Result<(), RenderError> {
        match self.gpu_errors.try_recv() {
            Ok(error) => Err(error),
            Err(_) => Ok(()),
        }
    }
}

/// Minimal `block_on` so no async-runtime dependency is needed.
fn block_on<F: Future>(future: F) -> F::Output {
    struct ThreadWaker(Thread);
    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(ThreadWaker(thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(output) => return output,
            Poll::Pending => thread::park(),
        }
    }
}
