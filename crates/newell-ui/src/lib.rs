// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Controller: input handling and flat 1D UI cell state (ARCHITECTURE.md §1, §3.4).

mod cell;
mod demo;
mod error;
mod grid;
mod hit_test;
mod interaction;
mod layout;
mod metrics;
mod style;
mod widget_id;
mod widgets;

pub use cell::{Cell, CellContent};
pub use demo::demo_grid;
pub use error::UiError;
pub use grid::{CellGrid, MAX_CELLS};
pub use hit_test::{BorderAxis, HitTestResult, SharedBorder, UiPoint, hit_test};
pub use interaction::{InteractionState, WidgetState};
pub use layout::{BorderLine, GridRect, draw_border};
pub use metrics::GridMetrics;
pub use style::StyleId;
pub use widget_id::WidgetId;
pub use widgets::{
    Cyclic, Segment, Segmented, TreeNodeKind, TreeRow, WidgetContext, WidgetStyles, cycle_next,
    cycle_prev, draw_button, draw_checkbox, draw_cyclic, draw_label, draw_numeric_field,
    draw_segmented, draw_separator, draw_text_field, draw_tree_row,
};
