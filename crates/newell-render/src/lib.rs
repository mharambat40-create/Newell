// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! View: wgpu rendering pipeline for the viewport and ASCII overlay (ARCHITECTURE.md §1).

mod error;
mod renderer;
pub mod surface;

pub use error::RenderError;
pub use renderer::{CLEAR_COLOR, FrameOutcome, Renderer};
