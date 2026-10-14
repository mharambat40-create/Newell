// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Overlay primitives (M2.8): popups, modal panels, a scrolling list and an OK / Cancel row.
//!
//! Overlays are composed **last** into the same flat [`CellGrid`] (ARCHITECTURE.md §3.4,
//! `docs/plans/M2-ui-framework.md` §11.4). Nothing here is retained: an [`OverlayStack`] is
//! plain Controller state (which overlays are open, bottom to top) and
//! [`compose_overlays`] regenerates the cells from it every frame.
//!
//! # Conventions
//!
//! - **Z-order is stack order.** Overlays are drawn bottom to top, so later overlays cover
//!   earlier ones. There is no separate layer number and no GPU pass.
//! - **Overlay-local clipping.** An overlay's rect is intersected with the `bounds` given to
//!   [`compose_overlays`]; it shrinks to fit rather than being cropped. Placement helpers
//!   ([`place_popup`], [`center_modal`]) always return a rect inside `bounds`.
//! - **Covered cells are fully overwritten.** The frame replaces content, style, `widget_id`
//!   and `shared_border` of every cell it covers, so no metadata from the cells underneath
//!   (or from a previous frame) survives and covered controls cannot be hit.
//! - **Surface ownership.** Every cell of the frame (border included) is owned by the
//!   overlay's [`WidgetId`]; a hit on it means "inside the overlay, on no control".
//! - **Modal blocking.** A modal also clears `widget_id` / `shared_border` on every cell of
//!   `bounds` outside its frame, while leaving the visible content intact: the background
//!   stays drawn but no longer receives pointer events (§15.1). Keyboard focus trapping and
//!   Escape/Cancel focus return remain Controller logic.

mod actions;
mod formula;
mod list;
mod modal;
mod place;
mod selection;

use crate::widgets::{WidgetContext, cell};
use crate::{BorderLine, Cell, CellContent, CellGrid, GridRect, StyleId, WidgetId, draw_border};

pub use actions::draw_ok_cancel;
pub use formula::FormulaEditor;
pub use list::{ListItem, ScrollList, ScrollState, draw_scroll_list};
pub use modal::{ModalKey, ModalStatus};
pub use place::{ModalLayout, center_modal, modal_layout, place_popup};
pub use selection::SelectionPanel;

/// Kind of an overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    /// A drop-down: single-line border, does not block the rest of the grid.
    Popup,
    /// A modal panel: double-line border, blocks pointer events on the whole `bounds`.
    Modal,
}

/// One open overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overlay {
    /// Identity of the overlay surface; also the dismissal key.
    pub id: WidgetId,
    /// Popup or modal; selects the border style and whether `bounds` is blocked.
    pub kind: OverlayKind,
    /// Frame rect including the border. Clipped to `bounds` when composed.
    pub rect: GridRect,
}

/// The open overlays, bottom to top. Plain state; not a widget tree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OverlayStack {
    layers: Vec<Overlay>,
}

impl OverlayStack {
    /// Opens `overlay` on top. An overlay already open with the same id is replaced (and
    /// moved to the top), so ids stay unique.
    pub fn open(&mut self, overlay: Overlay) {
        self.layers.retain(|o| o.id != overlay.id);
        self.layers.push(overlay);
    }

    /// Closes the overlay `id`. Returns `false` if it was not open.
    pub fn dismiss(&mut self, id: WidgetId) -> bool {
        let before = self.layers.len();
        self.layers.retain(|o| o.id != id);
        self.layers.len() != before
    }

    /// Closes and returns the topmost overlay (the Escape / Cancel path).
    pub fn dismiss_top(&mut self) -> Option<Overlay> {
        self.layers.pop()
    }

    /// The topmost overlay, if any.
    #[must_use]
    pub fn top(&self) -> Option<&Overlay> {
        self.layers.last()
    }

    /// `true` if the overlay `id` is open.
    #[must_use]
    pub fn is_open(&self, id: WidgetId) -> bool {
        self.layers.iter().any(|o| o.id == id)
    }

    /// `true` if any open overlay is modal.
    #[must_use]
    pub fn has_modal(&self) -> bool {
        self.layers.iter().any(|o| o.kind == OverlayKind::Modal)
    }

    /// Number of open overlays.
    #[must_use]
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// `true` if no overlay is open.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// Overlays in draw order, bottom to top.
    pub fn iter(&self) -> impl Iterator<Item = &Overlay> {
        self.layers.iter()
    }
}

/// Sets every in-grid cell of `rect` through `f`. Visits only visible cells, so huge
/// off-grid rects stay cheap.
fn for_each_visible(grid: &mut CellGrid, rect: GridRect, mut f: impl FnMut(&mut Cell)) {
    let vis = rect.intersection(&GridRect::from_grid(grid));
    for row in vis.row()..vis.row().saturating_add(vis.height()) {
        for col in vis.col()..vis.col().saturating_add(vis.width()) {
            if let Some(c) = grid.get_mut(col, row) {
                f(c);
            }
        }
    }
}

/// Draws the frame of `overlay` clipped to `bounds` and returns the interior available to
/// content (the frame rect inset by the border).
fn draw_frame(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    overlay: &Overlay,
    bounds: GridRect,
) -> GridRect {
    let rect = overlay.rect.intersection(&bounds);
    let id = overlay.id;
    if overlay.kind == OverlayKind::Modal {
        for_each_visible(grid, bounds, |c| {
            c.widget_id = None;
            c.shared_border = None;
        });
    }
    let style: StyleId = ctx.styles.normal;
    for_each_visible(grid, rect, |c| {
        *c = cell(CellContent::Empty, style, Some(id));
    });
    let line = match overlay.kind {
        OverlayKind::Popup => BorderLine::Single,
        OverlayKind::Modal => BorderLine::Double,
    };
    draw_border(grid, rect, line, style);
    // The border writer leaves cells unowned; the surface owns them.
    for_each_visible(grid, rect, |c| c.widget_id = Some(id));
    rect.inset(1)
}

/// Composes every overlay of `stack` into `grid`, bottom to top, within `bounds`.
///
/// For each overlay the frame is drawn (see the module docs) and then `content` is called
/// with the overlay and its interior rect so the caller can draw M2.7 primitives, a
/// [`draw_scroll_list`] or a [`draw_ok_cancel`] row into it. The interior is empty when
/// the overlay is too small to hold content. Output depends only on the arguments.
pub fn compose_overlays(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    stack: &OverlayStack,
    bounds: GridRect,
    mut content: impl FnMut(&mut CellGrid, &Overlay, GridRect),
) {
    for overlay in stack.iter() {
        let inner = draw_frame(grid, ctx, overlay, bounds);
        content(grid, overlay, inner);
    }
}

#[cfg(test)]
mod tests;
