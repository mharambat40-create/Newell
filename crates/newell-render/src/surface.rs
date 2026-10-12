// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Pure surface logic: extent validity, acquire-result mapping, configure state.
//! No GPU access, so everything here is testable without a device.

use crate::RenderError;

/// Some((w, h)) iff both dimensions are non-zero.
pub fn configurable_extent(width: u32, height: u32) -> Option<(u32, u32)> {
    (width != 0 && height != 0).then_some((width, height))
}

/// What to do with the result of a surface acquire.
#[derive(Debug)]
pub enum FrameAction {
    Present,
    /// Present this frame, reconfigure before the next.
    PresentAndReconfigure,
    /// Reconfigure and skip this frame.
    Reconfigure,
    Skip,
    Fatal(RenderError),
}

/// Input is the acquire result reduced to `Ok(suboptimal)` or `Err(SurfaceError)`,
/// which keeps this pure (a `SurfaceTexture` owns a live texture).
pub fn classify(acquire: Result<bool, wgpu::SurfaceError>) -> FrameAction {
    match acquire {
        Ok(false) => FrameAction::Present,
        Ok(true) => FrameAction::PresentAndReconfigure,
        Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => FrameAction::Reconfigure,
        // Also how an occluded or hidden surface shows up (wgpu 26 has no Occluded variant).
        Err(wgpu::SurfaceError::Timeout) => FrameAction::Skip,
        Err(wgpu::SurfaceError::OutOfMemory) => FrameAction::Fatal(RenderError::OutOfMemory),
        Err(e @ wgpu::SurfaceError::Other) => FrameAction::Fatal(RenderError::Acquire(e)),
    }
}

/// Whether the surface can be drawn to, and whether it must be (re)configured first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceState {
    extent: Option<(u32, u32)>,
    needs_configure: bool,
}

impl SurfaceState {
    /// Starts unconfigured: the first configure is always pending if the extent is valid.
    pub fn new(width: u32, height: u32) -> Self {
        let extent = configurable_extent(width, height);
        Self {
            extent,
            needs_configure: extent.is_some(),
        }
    }

    /// Active means non-zero size. Zero size (minimised) is inactive and never configures.
    pub fn is_active(&self) -> bool {
        self.extent.is_some()
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        *self = Self::new(width, height);
    }

    /// The extent to configure with, iff active and a configure is pending.
    pub fn pending_configure(&self) -> Option<(u32, u32)> {
        self.extent.filter(|_| self.needs_configure)
    }

    pub fn mark_configured(&mut self) {
        self.needs_configure = false;
    }

    /// Records the reconfigure requests of a frame action.
    pub fn apply(&mut self, action: &FrameAction) {
        if matches!(
            action,
            FrameAction::Reconfigure | FrameAction::PresentAndReconfigure
        ) {
            self.needs_configure = true;
        }
    }
}
