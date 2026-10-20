// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Controller and window manager (ARCHITECTURE.md §1, §4.3).

pub mod camera;
pub mod navigation;
pub mod picking;

mod app;
pub mod controller;
pub mod input;
pub mod pacing;
pub mod scale;
pub mod status;
pub mod window;

/// Fatal application errors. Crate-local by design (no global error type).
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("event loop failed: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    #[error("window creation failed: {0}")]
    Window(#[from] winit::error::OsError),
    #[error(transparent)]
    Render(#[from] newell_render::RenderError),
    #[error(transparent)]
    Ui(#[from] newell_ui::UiError),
    #[error(transparent)]
    Kernel(#[from] newell_kernel::NewellKernelError),
}

/// Opens the window and runs the event loop until it is closed or a fatal error occurs.
pub fn run() -> Result<(), AppError> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut app = app::App::new();
    event_loop.run_app(&mut app)?;
    app.into_result()
}
