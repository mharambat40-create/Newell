// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Grid-native UI primitives (M2.7).
//!
//! Each primitive is a pure function that writes whole cells into a [`CellGrid`] inside a
//! [`GridRect`]. There are no widget objects, callbacks or registries: the caller owns the
//! logical state ([`WidgetState`], selector values, tree expansion) and re-composes the grid
//! from it, then the flat cell buffer is what the renderer consumes (ARCHITECTURE.md §3.4).
//!
//! # Conventions
//!
//! - **Clipping.** Every write is limited to `rect` and to the grid. A rect that is too
//!   small for a primitive draws nothing (documented per primitive); it never panics and
//!   never writes outside `rect`.
//! - **Full overwrite.** A primitive overwrites every cell it owns (content, style,
//!   `widget_id`, `shared_border`), so output depends only on the arguments, not on what
//!   the grid held before. Blank cells inside an interactive primitive are
//!   `CellContent::Empty` stamped with the primitive's [`WidgetId`], so the whole control is
//!   hit-testable and the hover left-edge lands on its first cell.
//! - **One row.** Except [`draw_button`] (which owns its whole rect), primitives use only
//!   the first row of `rect`.
//! - **Styles.** Styles come from [`WidgetStyles`] (indices into the existing placeholder
//!   palette). Hover is never composed here: it is shader-side (ARCHITECTURE.md §3.5).
//! - **Disabled.** A disabled control keeps its [`WidgetId`] (stable identity;
//!   `InteractionState::press` already refuses it) and uses the disabled style. Whether a
//!   disabled control may be hovered is Controller policy.
//!
//! # Shared-border hover rule
//!
//! A separator cell between two interactive neighbours carries `shared_border` routing and
//! no `widget_id`. It is **visually neutral** for hover: it has no GPU owner, so it never
//! paints the left-edge stripe. When a control's visual left edge is such a border, the
//! stripe is painted on the control's first owned cell, one cell inside the border. The
//! border is still routed by the M2.5 sub-cell rule, so pointing at either half of it
//! hovers the adjacent control. See `docs/plans/M2-ui-framework.md` (M2.7 decisions).

mod button;
mod field;
mod selector;
mod tree;

use std::ops::Range;

use crate::{Cell, CellContent, CellGrid, GridRect, InteractionState, StyleId, WidgetId};
use crate::{SharedBorder, WidgetState};

pub use button::{draw_button, draw_checkbox, draw_label, draw_separator};
pub use field::{draw_numeric_field, draw_text_field};
pub use selector::{
    Cyclic, Segment, Segmented, cycle_next, cycle_prev, draw_cyclic, draw_segmented,
};
pub use tree::{TreeNodeKind, TreeRow, draw_tree_row};

/// Styles used by the primitives, as indices into the existing placeholder palette.
///
/// No colours are decided here: the defaults reuse the palette slots that `newell-render`
/// already resolves (`0` text, `1` bright, `2` dim, `3` accent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetStyles {
    pub normal: StyleId,
    pub focused: StyleId,
    pub pressed: StyleId,
    pub selected: StyleId,
    pub disabled: StyleId,
    /// Secondary structure such as tree guide lines.
    pub muted: StyleId,
}

impl Default for WidgetStyles {
    fn default() -> Self {
        Self {
            normal: StyleId::DEFAULT,
            focused: StyleId::new(1),
            pressed: StyleId::new(1),
            selected: StyleId::new(3),
            disabled: StyleId::new(2),
            muted: StyleId::new(2),
        }
    }
}

/// Read-only inputs shared by every primitive: the style table and the transient
/// interaction state (focus and press are shown through style; hover is not).
#[derive(Debug, Clone, Copy)]
pub struct WidgetContext<'a> {
    pub styles: &'a WidgetStyles,
    pub interaction: &'a InteractionState,
}

impl WidgetContext<'_> {
    /// Style of non-interactive structure (separators, values): disabled or normal.
    #[must_use]
    pub fn plain_style(&self, state: WidgetState) -> StyleId {
        if state.disabled {
            self.styles.disabled
        } else {
            self.styles.normal
        }
    }

    /// Style of widget `id` in `state`. Precedence: disabled, pressed, selected, focused,
    /// normal.
    #[must_use]
    pub fn style_of(&self, id: WidgetId, state: WidgetState) -> StyleId {
        let s = self.styles;
        if state.disabled {
            s.disabled
        } else if self.interaction.pressed == Some(id) {
            s.pressed
        } else if state.selected {
            s.selected
        } else if self.interaction.focused == Some(id) {
            s.focused
        } else {
            s.normal
        }
    }
}

/// Number of cells `text` occupies (one per `char`), saturating.
pub(crate) fn text_len(text: &str) -> u32 {
    u32::try_from(text.chars().count()).unwrap_or(u32::MAX)
}

/// Offset that centres `len` cells in `width` cells (0 if it does not fit).
pub(crate) fn centered(width: u32, len: u32) -> u32 {
    width.saturating_sub(len) / 2
}

/// A clipped drawing surface: offsets are relative to `rect`'s origin.
pub(crate) struct Canvas<'a> {
    grid: &'a mut CellGrid,
    rect: GridRect,
    visible: GridRect,
}

impl<'a> Canvas<'a> {
    pub(crate) fn new(grid: &'a mut CellGrid, rect: GridRect) -> Self {
        let visible = rect.intersection(&GridRect::from_grid(grid));
        Self {
            grid,
            rect,
            visible,
        }
    }

    pub(crate) fn width(&self) -> u32 {
        self.rect.width()
    }

    pub(crate) fn height(&self) -> u32 {
        self.rect.height()
    }

    /// Offsets at or past this column are outside the grid, so nothing there can be seen.
    /// Bounded by the grid, unlike [`Canvas::width`].
    pub(crate) fn reach(&self) -> u32 {
        self.visible_cols().end
    }

    /// Column offsets (relative to the rect) that are inside the grid.
    fn visible_cols(&self) -> Range<u32> {
        let start = self.visible.col().saturating_sub(self.rect.col());
        start..start.saturating_add(self.visible.width())
    }

    fn visible_rows(&self) -> Range<u32> {
        let start = self.visible.row().saturating_sub(self.rect.row());
        start..start.saturating_add(self.visible.height())
    }

    /// Writes one cell at `(dx, dy)` if it is inside the rect and the grid.
    pub(crate) fn put(&mut self, dx: u32, dy: u32, cell: Cell) {
        if dx >= self.rect.width() || dy >= self.rect.height() {
            return;
        }
        let (Some(col), Some(row)) = (
            self.rect.col().checked_add(dx),
            self.rect.row().checked_add(dy),
        ) else {
            return;
        };
        if let Some(slot) = self.grid.get_mut(col, row) {
            *slot = cell;
        }
    }

    /// Fills `[dx, dx + len)` on row `dy` with `cell`, visiting only visible cells.
    pub(crate) fn fill(&mut self, dx: u32, len: u32, dy: u32, cell: Cell) {
        let cols = self.visible_cols();
        if !self.visible_rows().contains(&dy) {
            return;
        }
        let from = dx.max(cols.start);
        let to = dx.saturating_add(len).min(cols.end);
        for x in from..to {
            self.put(x, dy, cell);
        }
    }

    /// Fills column `dx` on every row of the rect, visiting only visible rows.
    pub(crate) fn fill_col(&mut self, dx: u32, cell: Cell) {
        for y in self.visible_rows() {
            self.put(dx, y, cell);
        }
    }

    /// Writes `text` from `(dx, dy)`, one cell per `char`; a space is an empty cell. Returns
    /// the offset just past the text (saturating). Characters past the rect edge are dropped.
    pub(crate) fn text(
        &mut self,
        dx: u32,
        dy: u32,
        text: &str,
        style: StyleId,
        widget_id: Option<WidgetId>,
    ) -> u32 {
        let mut x = dx;
        for ch in text.chars() {
            if x >= self.rect.width() {
                break;
            }
            let content = if ch == ' ' {
                CellContent::Empty
            } else {
                CellContent::Glyph(ch)
            };
            self.put(x, dy, cell(content, style, widget_id));
            x = x.saturating_add(1);
        }
        dx.saturating_add(text_len(text))
    }
}

/// An ordinary cell (no shared-border routing).
pub(crate) fn cell(content: CellContent, style: StyleId, widget_id: Option<WidgetId>) -> Cell {
    Cell {
        content,
        style,
        widget_id,
        shared_border: None,
    }
}

/// A blank cell owned by `widget_id`.
pub(crate) fn owned_blank(style: StyleId, widget_id: WidgetId) -> Cell {
    cell(CellContent::Empty, style, Some(widget_id))
}

/// A separator glyph shared by two controls, routed by the M2.5 sub-cell rule.
pub(crate) fn shared(ch: char, style: StyleId, border: SharedBorder) -> Cell {
    Cell {
        content: CellContent::Glyph(ch),
        style,
        widget_id: None,
        shared_border: Some(border),
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    pub fn grid(cols: u32, rows: u32) -> CellGrid {
        CellGrid::new(cols, rows).unwrap()
    }

    /// Row text, `.` for empty cells.
    pub fn text_row(g: &CellGrid, row: u32) -> String {
        (0..g.cols())
            .map(|c| match g.get(c, row).map(|x| x.content) {
                Some(CellContent::Glyph(ch)) => ch,
                _ => '.',
            })
            .collect()
    }

    /// Row owners, `-` for none, else the raw id as a hex digit.
    pub fn owner_row(g: &CellGrid, row: u32) -> String {
        (0..g.cols())
            .map(|c| match g.get(c, row).and_then(|x| x.widget_id) {
                Some(id) => char::from_digit(id.get(), 16).unwrap(),
                None => '-',
            })
            .collect()
    }

    pub fn id(v: u32) -> WidgetId {
        WidgetId::new(v)
    }

    pub fn ctx<'a>(styles: &'a WidgetStyles, inter: &'a InteractionState) -> WidgetContext<'a> {
        WidgetContext {
            styles,
            interaction: inter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;
    use crate::BorderAxis;

    #[test]
    fn style_precedence() {
        let styles = WidgetStyles::default();
        let mut inter = InteractionState::default();
        let w = id(1);
        let plain = WidgetState::default();
        assert_eq!(ctx(&styles, &inter).style_of(w, plain), styles.normal);
        inter.set_focused(Some(w));
        assert_eq!(ctx(&styles, &inter).style_of(w, plain), styles.focused);
        let selected = WidgetState {
            selected: true,
            ..plain
        };
        assert_eq!(ctx(&styles, &inter).style_of(w, selected), styles.selected);
        inter.press(w, plain);
        assert_eq!(ctx(&styles, &inter).style_of(w, selected), styles.pressed);
        let disabled = WidgetState {
            disabled: true,
            ..selected
        };
        assert_eq!(ctx(&styles, &inter).style_of(w, disabled), styles.disabled);
        // Focus and press belong to one widget only.
        assert_eq!(ctx(&styles, &inter).style_of(id(2), plain), styles.normal);
    }

    #[test]
    fn default_styles_use_existing_palette_slots() {
        let s = WidgetStyles::default();
        assert_eq!(s.normal, StyleId::DEFAULT);
        assert!(
            [s.focused, s.pressed, s.selected, s.disabled, s.muted]
                .iter()
                .all(|st| st.get() <= 3)
        );
    }

    #[test]
    fn canvas_never_writes_outside_rect_or_grid() {
        let mut g = grid(6, 3);
        let before = g.clone();
        let mut c = Canvas::new(&mut g, GridRect::new(2, 1, 2, 1));
        c.text(0, 0, "abcdef", StyleId::DEFAULT, None);
        c.text(0, 5, "zz", StyleId::DEFAULT, None);
        c.fill(0, u32::MAX, 0, owned_blank(StyleId::DEFAULT, id(1)));
        assert_ne!(g, before);
        assert_eq!(text_row(&g, 1), "......");
        assert_eq!(owner_row(&g, 1), "--11--");
        assert_eq!(text_row(&g, 0), "......");
        assert_eq!(owner_row(&g, 2), "------");
    }

    #[test]
    fn interactive_primitives_overwrite_stale_owner_and_shared_border() {
        // Every cell a primitive owns (by `widget_id` or `shared_border`) must match a
        // clean draw, whatever the grid held before. Stale `widget_id` and `shared_border`
        // values from the previous frame must not survive inside the owned cells.
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let c = ctx(&styles, &inter);
        let rect = GridRect::new(0, 0, 12, 1);
        let state = WidgetState::default();
        let segs = [
            Segment {
                id: id(2),
                label: "A",
            },
            Segment {
                id: id(3),
                label: "B",
            },
        ];
        let values = ["Blind"];
        type Draw<'a> = Box<dyn Fn(&mut CellGrid) + 'a>;
        let draws: Vec<Draw<'_>> = vec![
            Box::new(|g| draw_button(g, &c, rect, id(2), state, "OK")),
            Box::new(|g| draw_checkbox(g, &c, rect, id(2), state, "Invert")),
            Box::new(|g| draw_text_field(g, &c, rect, id(2), state, "12.5")),
            Box::new(|g| draw_numeric_field(g, &c, rect, id(2), id(3), state, "1 mm")),
            Box::new(|g| {
                let sel = Segmented {
                    segments: &segs,
                    active: 0,
                    state,
                };
                draw_segmented(g, &c, rect, &sel);
            }),
            Box::new(|g| {
                let sel = Cyclic {
                    prev: id(2),
                    next: id(3),
                    values: &values,
                    current: 0,
                    state,
                };
                draw_cyclic(g, &c, rect, &sel);
            }),
            Box::new(|g| {
                let row = TreeRow {
                    id: id(2),
                    kind: TreeNodeKind::Leaf,
                    depth: 1,
                    last: true,
                    trunk: &[],
                    label: "Sketch",
                    state,
                };
                draw_tree_row(g, &c, rect, &row);
            }),
        ];
        let stale = Cell {
            content: CellContent::Glyph('#'),
            style: StyleId::new(3),
            widget_id: Some(id(9)),
            shared_border: Some(SharedBorder {
                axis: BorderAxis::Vertical,
                low: Some(id(8)),
                high: Some(id(7)),
            }),
        };
        for (i, draw) in draws.iter().enumerate() {
            let mut clean = grid(12, 1);
            draw(&mut clean);
            let mut dirty = grid(12, 1);
            for col in 0..12 {
                *dirty.get_mut(col, 0).unwrap() = stale;
            }
            draw(&mut dirty);
            for col in 0..12 {
                let owned = clean.get(col, 0).unwrap();
                if owned.widget_id.is_some() || owned.shared_border.is_some() {
                    assert_eq!(
                        dirty.get(col, 0),
                        clean.get(col, 0),
                        "primitive {i}, col {col}"
                    );
                }
            }
        }
    }

    #[test]
    fn canvas_handles_off_grid_and_huge_rects() {
        let mut g = grid(4, 2);
        let mut c = Canvas::new(&mut g, GridRect::new(u32::MAX - 1, 0, u32::MAX, 5));
        c.fill(0, u32::MAX, 0, owned_blank(StyleId::DEFAULT, id(1)));
        c.text(0, 0, "x", StyleId::DEFAULT, None);
        assert_eq!(g, grid(4, 2));
        let mut c = Canvas::new(&mut g, GridRect::new(0, 0, u32::MAX, u32::MAX));
        c.fill(0, u32::MAX, 1, owned_blank(StyleId::DEFAULT, id(2)));
        assert_eq!(owner_row(&g, 1), "2222");
        assert_eq!(owner_row(&g, 0), "----");
    }
}
