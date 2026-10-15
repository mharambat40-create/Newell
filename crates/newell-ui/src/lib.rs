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
mod extrude;
mod grid;
mod hit_test;
mod interaction;
mod layout;
mod menu;
mod metrics;
mod overlays;
mod shell;
mod style;
mod widget_id;
mod widgets;
mod window;

pub use cell::{Cell, CellContent};
pub use demo::demo_grid;
pub use error::UiError;
pub use extrude::{
    ExtrudeGeometryType, ExtrudeOffsetMode, ExtrudeResultMode, ExtrudeStatus, ExtrudeTermination,
    ExtrudeUi, ExtrudeValues, MockSelection,
};
pub use grid::{CellGrid, MAX_CELLS};
pub use hit_test::{BorderAxis, HitTestResult, SharedBorder, UiPoint, hit_test};
pub use interaction::{InteractionState, WidgetState};
pub use layout::{BorderLine, GridRect, draw_border};
pub use menu::{
    CycleStep, MAX_ENTRIES, MAX_MENUS, MAX_MNEMONIC, MenuAction, MenuCheckbox, MenuCycle, MenuDef,
    MenuEntry, MenuIds, MenuItem, MenuKey, MenuResult, MenuState, MenuTarget, Mnemonic,
    draw_menu_bar, draw_menu_popup, popup_size, title_rect,
};
pub use metrics::{DEFAULT_K, GridMetrics};
pub use overlays::{
    FormulaEditor, ListItem, ModalKey, ModalLayout, ModalStatus, Overlay, OverlayKind,
    OverlayStack, ScrollList, ScrollState, SelectionPanel, center_modal, compose_overlays,
    draw_ok_cancel, draw_scroll_list, modal_layout, place_popup,
};
pub use shell::{Separator, ShellKey, ShellState};
pub use style::StyleId;
pub use widget_id::WidgetId;
pub use widgets::{
    ActionPanel, Cyclic, PanelRow, PanelRowKind, ROW_ID_STRIDE, Segment, Segmented, TreeNodeKind,
    TreeRow, WidgetContext, WidgetStyles, cycle_next, cycle_prev, draw_button, draw_checkbox,
    draw_cyclic, draw_label, draw_numeric_field, draw_segmented, draw_separator, draw_text_field,
    draw_tree_row,
};
pub use window::{
    DEFAULT_SCALE_PERCENT, MAX_K, MAX_REFERENCE_K, MIN_COLS, MIN_K, MIN_ROWS, ResizeSnapshot,
    ResizeTarget, ResizeZone, SCALE_PERCENTS, ScaleDecision, ScaleMode, ScreenRect, WindowAction,
    WindowFit, available_height, composed_grid_size, effective_scale, fit_window, k_fit,
    min_window_size, monitor_reference_scale, resize_target, resize_zone, usable_rect,
};
