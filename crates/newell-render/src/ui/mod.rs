// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! ASCII UI overlay: flat `CellGrid` -> GPU cell buffer -> one instanced draw call.

mod gpu_cell;
mod pipeline;
mod theme;

pub use gpu_cell::{EMPTY_GLYPH, GPU_CELL_BYTES, GpuCell, cells_to_bytes};
pub use pipeline::UiOverlay;
pub use theme::Theme;
