// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! `App`: winit `ApplicationHandler` mapping window events to `Renderer` calls.
//!
//! Continuous 60 FPS redraw is a bootstrap mechanism to validate the loop, not the final
//! redraw architecture (ARCHITECTURE.md §4.1 remains the target).

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;

use newell_render::{GLYPH_CELL_HEIGHT, GLYPH_CELL_WIDTH, Renderer};
use newell_ui::GridMetrics;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

use crate::AppError;
use crate::controller::{ShellController, Update};
use crate::input::{translate_key, ui_point};
use crate::pacing::FramePacer;

const TARGET_FPS: NonZeroU32 = NonZeroU32::MIN.saturating_add(59);
const TITLE: &str = "Newell";
const INITIAL_SIZE: LogicalSize<f64> = LogicalSize::new(1280.0, 800.0);

pub(crate) struct App {
    // Field order is drop order: the renderer (surface) is dropped before the window.
    renderer: Option<Renderer>,
    window: Option<Arc<Window>>,
    shell: Option<ShellController>,
    modifiers: ModifiersState,
    pacer: FramePacer,
    error: Option<AppError>,
}

impl App {
    pub(crate) fn new() -> Self {
        Self {
            renderer: None,
            window: None,
            shell: None,
            modifiers: ModifiersState::empty(),
            pacer: FramePacer::new(TARGET_FPS, Instant::now()),
            error: None,
        }
    }

    pub(crate) fn into_result(self) -> Result<(), AppError> {
        self.error.map_or(Ok(()), Err)
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl Into<AppError>) {
        self.error = Some(error.into());
        event_loop.exit();
    }

    /// Uploads what `update` says changed and asks for a redraw.
    fn apply(&mut self, update: Update) {
        if !update.any() {
            return;
        }
        if let (Some(renderer), Some(shell)) = (&mut self.renderer, &self.shell) {
            if update.grid {
                renderer.set_ui_grid(shell.grid(), shell.metrics());
            }
            renderer.set_hovered(shell.hovered());
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn resize_to_window(&mut self) {
        if let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) {
            let size = window.inner_size();
            renderer.resize(size.width, size.height);
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Can be called more than once (macOS, Android).
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(INITIAL_SIZE);
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => return self.fail(event_loop, error),
        };
        let size = window.inner_size();
        match Renderer::new(window.clone(), size.width, size.height) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(error) => return self.fail(event_loop, error),
        }
        // The grid is sized once from the initial window; responsive resizing is M2.14.
        let metrics = match GridMetrics::new(GLYPH_CELL_WIDTH, GLYPH_CELL_HEIGHT) {
            Ok(metrics) => metrics,
            Err(error) => return self.fail(event_loop, error),
        };
        let cols = size.width / GLYPH_CELL_WIDTH;
        let rows = size.height / GLYPH_CELL_HEIGHT;
        let shell = match ShellController::new(cols, rows, metrics) {
            Ok(shell) => shell,
            Err(error) => return self.fail(event_loop, error),
        };
        if let Some(renderer) = &mut self.renderer {
            renderer.set_ui_grid(shell.grid(), metrics);
        }
        self.shell = Some(shell);
        window.request_redraw();
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = &self.window else { return };
        if window.id() != id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                window.request_redraw();
            }
            // A Resized may not follow; re-read the physical size.
            WindowEvent::ScaleFactorChanged { .. } => {
                self.resize_to_window();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let point = ui_point(position.x, position.y);
                if let Some(shell) = &mut self.shell {
                    let update = shell.pointer_moved(point);
                    self.apply(update);
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if let Some(shell) = &mut self.shell {
                    let update = shell.pointer_moved(None);
                    self.apply(update);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some(shell) = &mut self.shell {
                    let update = shell.pointer_pressed();
                    self.apply(update);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let keys = translate_key(
                    &event.logical_key,
                    event.physical_key,
                    event.text.as_deref(),
                    self.modifiers,
                );
                if let Some(shell) = &mut self.shell {
                    let update = keys
                        .into_iter()
                        .fold(Update::default(), |acc, key| acc.merge(shell.key(key)));
                    self.apply(update);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer {
                    if let Err(error) = renderer.render() {
                        return self.fail(event_loop, error);
                    }
                }
                self.pacer.on_frame(Instant::now());
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            // macOS may not deliver Resized(0, 0) on minimise, so also ask the window.
            // If this reports nothing, a Timeout from acquire is the non-fatal Skip path.
            let minimised = window.is_minimized().unwrap_or(false);
            if !minimised && self.pacer.is_due(Instant::now()) {
                window.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.pacer.deadline()));
    }
}
