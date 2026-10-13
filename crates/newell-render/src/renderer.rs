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

use newell_ui::{CellGrid, GridMetrics};

use crate::RenderError;
use crate::glyph_atlas::{GlyphAtlas, load_font};
use crate::surface::{FrameAction, SurfaceState, classify};
use crate::ui::UiOverlay;

const FONT: &[u8] = include_bytes!("../../../assets/FiraCode-Regular.ttf");

/// Placeholder dark neutral; theming is not in scope.
pub const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.08,
    g: 0.08,
    b: 0.09,
    a: 1.0,
};

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
    /// Fed by both the uncaptured-error handler and the device-lost callback.
    gpu_errors: Receiver<RenderError>,
}

impl Renderer {
    /// `window` is an owned handle (e.g. `Arc<winit::window::Window>`), so the surface is
    /// `'static`. `width`/`height` are physical pixels. A zero size yields an inactive
    /// renderer until a non-zero `resize`.
    pub fn new(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
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
        let atlas = GlyphAtlas::build(&load_font(FONT.to_vec())?)?;
        let ui = UiOverlay::new(&device, &queue, format, atlas);
        Ok(Self {
            surface,
            device,
            queue,
            config,
            state: SurfaceState::new(width, height),
            ui,
            gpu_errors,
        })
    }

    /// Physical pixels. A zero dimension (minimised) deactivates the surface without error.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.state.resize(width, height);
    }

    /// Replaces the ASCII overlay with `grid`, drawn from the top-left in whole cells of `metrics`
    /// physical pixels. The whole grid is uploaded; it is drawn by one draw call.
    pub fn set_ui_grid(&mut self, grid: &CellGrid, metrics: GridMetrics) {
        self.ui.set_grid(&self.device, &self.queue, grid, metrics);
    }

    /// Acquires, clears to `CLEAR_COLOR`, submits, presents. Returns `Skipped` when the
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
                    load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        }));
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
