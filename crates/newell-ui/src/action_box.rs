// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! The exact-grid Action Box (M2.21, `docs/ui_design/README_UI.md` §2, §33, §36).
//!
//! An [`ActionBox`] is plain data rebuilt from semantic state on every composition: an ordered
//! list of rows, each exactly [`BOX_WIDTH`] cells wide, made of literal frame text and the grid
//! primitives of `widgets` (checkbox, cyclic selector, text/numeric field, button). Nothing is
//! retained between compositions and nothing is laid out by a generic engine: every row says
//! where each of its cells goes, as the `.txt` reference does.
//!
//! [`ActionBox::draw_with`] composes into a private 31-wide scratch grid and copies the part that
//! fits the target rect, so a box wider than its region is cropped at the right edge (and
//! never changes shape).
//!
//! # Widget ids
//!
//! A control is identified by `(key, slot)` and its id is `base + key * KEY_SLOTS + slot`.
//! Keys are chosen once by the caller and never depend on which rows are currently shown, so
//! rows appearing or disappearing never renumber other controls.

use std::ops::Range;

use crate::widgets::{Canvas, NumericFields, draw_numeric_view, hover_row, shared};
use crate::{
    BorderAxis, CellGrid, Cyclic, GridRect, SharedBorder, WidgetContext, WidgetId, WidgetState,
    draw_button, draw_checkbox, draw_cyclic, draw_numeric_field, draw_text_field,
};

/// External width of every Action Box row, frame included.
pub(crate) const BOX_WIDTH: u32 = 31;
/// Column of the right border.
const LAST_COL: u32 = BOX_WIDTH - 1;
/// Widget ids reserved per control key (the most any control uses is 3 segments).
pub(crate) const KEY_SLOTS: u32 = 4;

/// One piece of a row, positioned at an absolute column of the 31-cell row.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Item {
    Text {
        col: u32,
        text: String,
    },
    /// A frame glyph shared by two controls (routed by the sub-cell rule).
    Divider {
        col: u32,
        glyph: char,
        low: WidgetId,
        high: WidgetId,
    },
    Button {
        col: u32,
        width: u32,
        id: WidgetId,
        label: String,
    },
    Check {
        col: u32,
        width: u32,
        id: WidgetId,
        checked: bool,
        label: String,
    },
    Cycle {
        col: u32,
        width: u32,
        prev: WidgetId,
        next: WidgetId,
        value: String,
    },
    Numeric {
        col: u32,
        width: u32,
        field: WidgetId,
        formula: WidgetId,
        text: String,
    },
    TextField {
        col: u32,
        width: u32,
        id: WidgetId,
        text: String,
    },
}

/// One row of an [`ActionBox`], built left to right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Row {
    base: u32,
    col: u32,
    items: Vec<Item>,
    keys: Vec<u32>,
}

impl Row {
    /// An empty row; control ids derive from `base`.
    pub(crate) fn new(base: u32) -> Self {
        Self {
            base,
            col: 0,
            items: Vec::new(),
            keys: Vec::new(),
        }
    }

    fn id(&self, key: u32, slot: u32) -> WidgetId {
        WidgetId::new(
            self.base
                .saturating_add(key.saturating_mul(KEY_SLOTS))
                .saturating_add(slot),
        )
    }

    fn note(&mut self, key: u32) {
        if !self.keys.contains(&key) {
            self.keys.push(key);
        }
    }

    /// Cells left from the cursor to the last interior cell (the right border is column 30).
    fn rest(&self) -> u32 {
        LAST_COL.saturating_sub(self.col)
    }

    fn push(mut self, width: u32, item: Item) -> Self {
        self.items.push(item);
        self.col = self.col.saturating_add(width);
        self
    }

    /// Literal text at the cursor, one cell per `char` (a space is an empty cell).
    pub(crate) fn text(self, text: &str) -> Self {
        let width = u32::try_from(text.chars().count()).unwrap_or(u32::MAX);
        let col = self.col;
        self.push(
            width,
            Item::Text {
                col,
                text: text.to_owned(),
            },
        )
    }

    /// A frame glyph between two controls of the same key: the left half routes to `low`,
    /// the right half to `high`.
    pub(crate) fn divider(mut self, glyph: char, key: u32, low: u32, high: u32) -> Self {
        self.note(key);
        let (low, high) = (self.id(key, low), self.id(key, high));
        let col = self.col;
        self.push(
            1,
            Item::Divider {
                col,
                glyph,
                low,
                high,
            },
        )
    }

    /// A button owning `width` cells with `label` centred (a segment of a selector).
    pub(crate) fn button(self, key: u32, slot: u32, label: &str, width: u32) -> Self {
        self.push_button(key, slot, label, width)
    }

    /// An OK / Cancel button of the footer: a hover target (M2.25).
    pub(crate) fn action_button(self, key: u32, slot: u32, label: &str, width: u32) -> Self {
        self.push_button(key, slot, label, width)
    }

    fn push_button(mut self, key: u32, slot: u32, label: &str, width: u32) -> Self {
        self.note(key);
        let id = self.id(key, slot);
        let col = self.col;
        self.push(
            width,
            Item::Button {
                col,
                width,
                id,
                label: label.to_owned(),
            },
        )
    }

    /// `□ label` / `■ label` over `width` cells; the whole span is owned by slot 0.
    pub(crate) fn check(mut self, key: u32, checked: bool, label: &str, width: u32) -> Self {
        self.note(key);
        let id = self.id(key, 0);
        let col = self.col;
        self.push(
            width,
            Item::Check {
                col,
                width,
                id,
                checked,
                label: label.to_owned(),
            },
        )
    }

    /// A checkbox that owns every cell up to the right border.
    pub(crate) fn check_fill(self, key: u32, checked: bool, label: &str) -> Self {
        let width = self.rest();
        self.check(key, checked, label, width)
    }

    /// `◀ value ▶` over `width` cells; `◀` is slot 0, `▶` slot 1.
    pub(crate) fn cycle(mut self, key: u32, value: &str, width: u32) -> Self {
        self.note(key);
        let (prev, next) = (self.id(key, 0), self.id(key, 1));
        let col = self.col;
        self.push(
            width,
            Item::Cycle {
                col,
                width,
                prev,
                next,
                value: value.to_owned(),
            },
        )
    }

    /// `[text      ☰]` up to the right border: the field is slot 0, `☰]` slot 1.
    pub(crate) fn numeric(mut self, key: u32, text: &str) -> Self {
        self.note(key);
        let (field, formula) = (self.id(key, 0), self.id(key, 1));
        let (col, width) = (self.col, self.rest());
        self.push(
            width,
            Item::Numeric {
                col,
                width,
                field,
                formula,
                text: text.to_owned(),
            },
        )
    }

    /// A selection field up to the right border (README §5.5): `[Select…       ]` for no
    /// element (one id, slot 0), `[1 Element    ☰]` / `[N Elements   ☰]` otherwise (slot 0 the
    /// field, slot 1 the `☰` that manages the selection).
    pub(crate) fn select(self, key: u32, count: u32) -> Self {
        self.select_or(key, count, "Select…")
    }

    /// As [`Row::select`], with `empty` shown instead of `Select…` while nothing is selected
    /// (`Faces` of Boolean shows `none`).
    pub(crate) fn select_or(mut self, key: u32, count: u32, empty: &str) -> Self {
        if count == 0 {
            self.note(key);
            let id = self.id(key, 0);
            let (col, width) = (self.col, self.rest());
            return self.push(
                width,
                Item::TextField {
                    col,
                    width,
                    id,
                    text: empty.to_owned(),
                },
            );
        }
        let text = if count == 1 {
            "1 Element".to_owned()
        } else {
            format!("{count} Elements")
        };
        self.note(key);
        let (field, formula) = (self.id(key, 0), self.id(key, 1));
        let (col, width) = (self.col, self.rest());
        self.push(
            width,
            Item::Numeric {
                col,
                width,
                field,
                formula,
                text,
            },
        )
    }

    /// Closes a content row: blanks up to column 30 and the right border `│`.
    pub(crate) fn end(self) -> Self {
        let border = LAST_COL;
        let mut row = self;
        row.col = border;
        row.push(
            1,
            Item::Text {
                col: border,
                text: "│".to_owned(),
            },
        )
    }
}

/// The composed rows of one Action Box.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ActionBox {
    rows: Vec<Row>,
}

impl ActionBox {
    pub(crate) fn push(&mut self, row: Row) {
        self.rows.push(row);
    }

    /// Number of rows, frame included.
    pub(crate) fn height(&self) -> usize {
        self.rows.len()
    }

    /// `true` if a control with `key` is part of this composition.
    pub(crate) fn has_key(&self, key: u32) -> bool {
        self.rows.iter().any(|r| r.keys.contains(&key))
    }

    /// Composes the box into `rect` of `grid` and returns the number of rows that fit.
    ///
    /// The box is drawn at its true 31-cell width into a scratch grid first, then the cells
    /// inside `rect` and the grid are copied over: a narrower or shorter `rect` crops the
    /// right and bottom, it never squeezes the box. Every copied cell is fully overwritten.
    #[cfg(test)]
    pub(crate) fn draw(
        &self,
        grid: &mut CellGrid,
        ctx: &WidgetContext<'_>,
        rect: GridRect,
    ) -> usize {
        self.draw_with(grid, ctx, rect, None).0
    }

    /// The mock text of numeric field `id` (`+|0.0 mm`), if this box has such a field.
    pub(crate) fn numeric_text(&self, id: WidgetId) -> Option<&str> {
        self.rows
            .iter()
            .flat_map(|r| &r.items)
            .find_map(|item| match item {
                Item::Numeric { field, text, .. } if *field == id => Some(text.as_str()),
                _ => None,
            })
    }

    /// `true` when `(col, row)`, relative to this box, is the visible direction marker of
    /// numeric field `id`.  The marker is one cell after the opening `[`.
    pub(crate) fn numeric_sign_at(&self, id: WidgetId, col: u32, row: u32) -> bool {
        self.rows.get(row as usize).is_some_and(|line| {
            line.items.iter().any(|item| match item {
                Item::Numeric {
                    col: start,
                    field,
                    text,
                    ..
                } => {
                    *field == id
                        && matches!(text.as_bytes().get(0..2), Some(b"+|" | b"-|"))
                        && col == start.saturating_add(1)
                }
                _ => false,
            })
        })
    }

    /// As [`ActionBox::draw_with`], showing the edited values of `fields` in its numeric fields.
    /// Returns the number of rows that fit and whether the hovered target was highlighted.
    pub(crate) fn draw_with(
        &self,
        grid: &mut CellGrid,
        ctx: &WidgetContext<'_>,
        rect: GridRect,
        fields: Option<&NumericFields>,
    ) -> (usize, bool) {
        let height = u32::try_from(self.rows.len()).unwrap_or(u32::MAX);
        let Ok(mut scratch) = CellGrid::new(BOX_WIDTH, height) else {
            return (0, false);
        };
        let mut lit = false;
        for (y, row) in (0..height).zip(&self.rows) {
            lit |= draw_row(&mut scratch, ctx, y, row, fields);
        }
        let target = rect.intersection(&GridRect::from_grid(grid));
        let (cols, rows) = (target.width().min(BOX_WIDTH), target.height().min(height));
        for dy in 0..rows {
            for dx in 0..cols {
                // `target` lies inside the grid and `scratch` is at least `cols` x `rows`.
                let src = scratch.get(dx, dy).copied().unwrap_or_default();
                if let (Some(c), Some(r)) =
                    (target.col().checked_add(dx), target.row().checked_add(dy))
                    && let Some(slot) = grid.get_mut(c, r)
                {
                    *slot = src;
                }
            }
        }
        (rows as usize, lit)
    }
}

/// The columns of the hover target `hovered` in `row` (M2.25), if the row holds it: a checkbox
/// or a field/cyclic selector makes the whole row a target; each header/footer button
/// highlights only its own boundaries.
fn hover_span(row: &Row, hovered: WidgetId) -> Option<Range<u32>> {
    row.items.iter().find_map(|item| match item {
        Item::Check { col, id, .. } | Item::TextField { col, id, .. } if *id == hovered => {
            Some(*col..LAST_COL)
        }
        Item::Numeric {
            col,
            field,
            formula,
            ..
        } if hovered == *field || hovered == *formula => Some(*col..LAST_COL),
        Item::Cycle {
            col, prev, next, ..
        } if hovered == *prev || hovered == *next => Some(*col..LAST_COL),
        Item::Button { col, width, id, .. } if *id == hovered => {
            Some(*col..col.saturating_add(*width))
        }
        _ => None,
    })
}

/// Draws row `y` of `row` into the scratch grid, then substitutes its hover slots if the
/// hovered target is in it (`ctx.interaction.hovered`). Returns `true` if it was highlighted.
fn draw_row(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    y: u32,
    row: &Row,
    fields: Option<&NumericFields>,
) -> bool {
    draw_items(grid, ctx, y, row, fields);
    let Some(hovered) = ctx.interaction.hovered else {
        return false;
    };
    let Some(span) = hover_span(row, hovered) else {
        return false;
    };
    if row
        .items
        .iter()
        .any(|item| matches!(item, Item::Button { id, .. } if *id == hovered))
    {
        // Header segments may use a dashed divider. Substitute only the two exact
        // boundaries, never scan across an adjacent button.
        for (col, glyph) in [(span.start.saturating_sub(1), '╞'), (span.end, '╡')] {
            if let Some(cell) = grid.get_mut(col, y)
                && matches!(cell.content, crate::CellContent::Glyph('│' | '┆'))
            {
                cell.content = crate::CellContent::Glyph(glyph);
            }
        }
        true
    } else {
        hover_row(grid, y, 0, span)
    }
}

fn draw_items(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    y: u32,
    row: &Row,
    fields: Option<&NumericFields>,
) {
    let at = |col: u32, width: u32| GridRect::new(col, y, width, 1);
    let state = WidgetState::default();
    for item in &row.items {
        match item {
            Item::Text { col, text } => {
                let mut canvas = Canvas::new(grid, at(*col, BOX_WIDTH.saturating_sub(*col)));
                canvas.text(0, 0, text, ctx.styles.normal, None);
            }
            Item::Divider {
                col,
                glyph,
                low,
                high,
            } => {
                let border = SharedBorder {
                    axis: BorderAxis::Vertical,
                    low: Some(*low),
                    high: Some(*high),
                };
                let mut canvas = Canvas::new(grid, at(*col, 1));
                canvas.put(0, 0, shared(*glyph, ctx.styles.normal, border));
            }
            Item::Button {
                col,
                width,
                id,
                label,
                ..
            } => draw_button(grid, ctx, at(*col, *width), *id, state, label),
            Item::Check {
                col,
                width,
                id,
                checked,
                label,
            } => {
                let state = WidgetState {
                    checked: *checked,
                    ..state
                };
                draw_checkbox(grid, ctx, at(*col, *width), *id, state, label);
            }
            Item::Cycle {
                col,
                width,
                prev,
                next,
                value,
            } => {
                let values = [value.as_str()];
                let sel = Cyclic {
                    prev: *prev,
                    next: *next,
                    values: &values,
                    current: 0,
                    state,
                };
                draw_cyclic(grid, ctx, at(*col, *width), &sel);
            }
            Item::Numeric {
                col,
                width,
                field,
                formula,
                text,
            } => match fields.and_then(|f| f.view(*field, text)) {
                Some(view) => {
                    draw_numeric_view(grid, ctx, at(*col, *width), *field, *formula, state, &view);
                }
                None => {
                    draw_numeric_field(grid, ctx, at(*col, *width), *field, *formula, state, text)
                }
            },
            Item::TextField {
                col,
                width,
                id,
                text,
            } => draw_text_field(grid, ctx, at(*col, *width), *id, state, text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CellContent, InteractionState, WidgetStyles};

    const BASE: u32 = 100;

    fn render(rows: Vec<Row>, rect: GridRect, cols: u32, grid_rows: u32) -> CellGrid {
        let mut b = ActionBox::default();
        for r in rows {
            b.push(r);
        }
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let ctx = WidgetContext {
            styles: &styles,
            interaction: &inter,
        };
        let mut g = CellGrid::new(cols, grid_rows).unwrap();
        b.draw(&mut g, &ctx, rect);
        g
    }

    fn line(g: &CellGrid, row: u32) -> String {
        (0..g.cols())
            .map(|c| match g.get(c, row).map(|x| x.content) {
                Some(CellContent::Glyph(ch)) => ch,
                _ => ' ',
            })
            .collect()
    }

    fn sample() -> Vec<Row> {
        vec![
            Row::new(BASE).text("│ Profile     ").select(0, 0).end(),
            Row::new(BASE)
                .text("│ │ Depth   ")
                .numeric(1, "+|0.0 mm")
                .end(),
            Row::new(BASE)
                .text("│ ")
                .check_fill(2, true, "Symetric")
                .end(),
        ]
    }

    #[test]
    fn rows_are_31_cells_with_borders() {
        let g = render(sample(), GridRect::new(0, 0, 31, 3), 31, 3);
        assert_eq!(line(&g, 0), "│ Profile     [Select…       ]│");
        assert_eq!(line(&g, 1), "│ │ Depth   [+|0.0 mm       ☰]│");
        assert_eq!(line(&g, 2), "│ ■ Symetric                  │");
    }

    #[test]
    fn ids_derive_from_key_and_slot_only() {
        let g = render(sample(), GridRect::new(0, 0, 31, 3), 31, 3);
        let id = |c, r| g.get(c, r).and_then(|x| x.widget_id).map(WidgetId::get);
        assert_eq!(id(14, 0), Some(BASE));
        assert_eq!(id(12, 1), Some(BASE + KEY_SLOTS));
        assert_eq!(id(28, 1), Some(BASE + KEY_SLOTS + 1));
        assert_eq!(id(2, 2), Some(BASE + 2 * KEY_SLOTS));
        assert_eq!(id(0, 0), None);
        assert_eq!(id(30, 0), None);
    }

    #[test]
    fn narrow_and_short_rects_crop_instead_of_squeezing() {
        let g = render(sample(), GridRect::new(2, 1, 10, 2), 40, 5);
        assert_eq!(line(&g, 1), format!("{:<40}", "  │ Profile "));
        assert_eq!(line(&g, 2), format!("{:<40}", "  │ │ Depth "));
        assert_eq!(line(&g, 0), " ".repeat(40));
        assert_eq!(line(&g, 3), " ".repeat(40));
        // Off-grid and empty rects draw nothing and do not panic.
        let blank = render(sample(), GridRect::new(100, 100, 31, 3), 6, 4);
        assert_eq!(blank, CellGrid::new(6, 4).unwrap());
        let _ = render(sample(), GridRect::new(0, 0, 0, 0), 6, 4);
        let _ = render(
            sample(),
            GridRect::new(u32::MAX - 1, u32::MAX - 1, 31, 3),
            6,
            4,
        );
    }

    #[test]
    fn keys_are_reported_and_dividers_route_to_both_neighbours() {
        let mut b = ActionBox::default();
        b.push(
            Row::new(BASE)
                .text("│")
                .button(5, 0, "A", 3)
                .divider('│', 5, 0, 1)
                .button(5, 1, "B", 3)
                .end(),
        );
        assert!(b.has_key(5) && !b.has_key(6));
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let ctx = WidgetContext {
            styles: &styles,
            interaction: &inter,
        };
        let mut g = CellGrid::new(31, 1).unwrap();
        assert_eq!(b.draw(&mut g, &ctx, GridRect::new(0, 0, 31, 1)), 1);
        let sep = g.get(4, 0).unwrap();
        assert_eq!(sep.widget_id, None);
        let border = sep.shared_border.unwrap();
        assert_eq!(border.low, Some(WidgetId::new(BASE + 5 * KEY_SLOTS)));
        assert_eq!(border.high, Some(WidgetId::new(BASE + 5 * KEY_SLOTS + 1)));
    }
}
