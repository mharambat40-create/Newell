// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Character-grid layout primitives: whole-cell rectangles, splits, clipping
//! and logical pane borders. Everything here is in logical cell coordinates.

mod border;
mod rect;

pub use border::{BorderLine, draw_border};
pub use rect::GridRect;
