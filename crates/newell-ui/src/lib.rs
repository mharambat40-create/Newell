// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Controller: input handling and flat 1D UI cell state (ARCHITECTURE.md §1, §3.4).

mod cell;
mod error;
mod grid;
mod metrics;
mod style;
mod widget_id;

pub use cell::{Cell, CellContent};
pub use error::UiError;
pub use grid::{CellGrid, MAX_CELLS};
pub use metrics::GridMetrics;
pub use style::StyleId;
pub use widget_id::WidgetId;
