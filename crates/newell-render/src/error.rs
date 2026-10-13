// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Render-layer errors.

/// Errors returned by the renderer. Crate-local by design (no global error type).
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("no compatible GPU adapter: {0}")]
    Adapter(#[from] wgpu::RequestAdapterError),
    #[error("surface creation failed: {0}")]
    Surface(#[from] wgpu::CreateSurfaceError),
    #[error("device request failed: {0}")]
    Device(#[from] wgpu::RequestDeviceError),
    #[error("surface reports no supported format")]
    NoSurfaceFormat,
    #[error("out of GPU memory")]
    OutOfMemory,
    #[error("surface acquire failed: {0:?}")]
    Acquire(wgpu::SurfaceError),
    #[error("uncaptured GPU error: {0}")]
    Uncaptured(String),
    /// Device loss is reported, not recovered, in M1.3.
    #[error("GPU device lost: {0}")]
    DeviceLost(String),
}

impl From<wgpu::Error> for RenderError {
    fn from(error: wgpu::Error) -> Self {
        match error {
            wgpu::Error::OutOfMemory { .. } => Self::OutOfMemory,
            other => Self::Uncaptured(other.to_string()),
        }
    }
}
