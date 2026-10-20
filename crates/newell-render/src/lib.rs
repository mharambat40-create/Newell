// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! View: wgpu rendering pipeline for the viewport and ASCII overlay (ARCHITECTURE.md §1).

mod camera;
pub use camera::{CameraMatrix, CameraSnapshot, SceneViewport, exposed_regions};

mod error;
pub mod glyph_atlas;
mod renderer;
mod scene;
pub mod surface;
pub mod ui;

pub use error::RenderError;
pub use glyph_atlas::{CellLayout, GlyphAtlas, GlyphInfo, GlyphRect, canonical_glyphs, load_font};
pub use renderer::{FrameOutcome, Renderer};
pub use ui::Theme;
