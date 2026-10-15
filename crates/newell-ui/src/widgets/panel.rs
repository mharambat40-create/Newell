// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Dynamic ActionPanel composition (M2.10).
//!
//! An [`ActionPanel`] is plain data: a title and an ordered list of [`PanelRow`]s that the
//! caller rebuilds from its command state. [`ActionPanel::draw`] composes the rows into a
//! [`CellGrid`] with the M2.7 primitives; nothing is retained between frames
//! (ARCHITECTURE.md §3.4).
//!
//! # Progressive disclosure
//!
//! A row that does not apply is **absent from the list**, not hidden or disabled. Rows
//! below it move up; no slot is reserved for it (`docs/ui_design/README.md` §5, §7.8).
//!
//! # Layout
//!
//! Inside a single-line border: a title row, a horizontal separator, then one grid row per
//! panel row, in list order. Rows that do not fit the rect's height are clipped (the first rows
//! that fit are shown, in order). A panel smaller than 2x2 draws only the clear.
//!
//! # Widget ids
//!
//! Ids derive from `base_id`, the caller's stable [`PanelRow::key`] and the control's slot
//! in the row (see [`ActionPanel::widget_id`]), never from the row's position. Rows
//! appearing or disappearing therefore never change the ids of other rows. Keys must be
//! unique: a row repeating an earlier key is skipped, as is a row whose ids would overflow
//! `u32`. Skipped rows take no space.
//!
//! # Full overwrite
//!
//! The whole panel rect is reset to default cells before drawing, so output depends only on
//! the panel and the context, never on the previous frame's cells.

use super::WidgetContext;
use crate::{
    BorderAxis, BorderLine, Cell, CellGrid, GridRect, WidgetId, WidgetState, draw_border,
    draw_button, draw_checkbox, draw_cyclic, draw_label, draw_numeric_field, draw_segmented,
    draw_separator, draw_text_field,
};
use crate::{Cyclic, Segment, Segmented};

/// Widget ids reserved per row key. A row has at most this many controls (extra segments
/// or buttons are not drawn).
pub const ROW_ID_STRIDE: u32 = 64;

/// What a [`PanelRow`] shows. Interactive kinds own ids by slot, as noted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelRowKind {
    /// Plain text. No ids.
    Label(String),
    /// Horizontal line. No ids.
    Separator,
    /// Slot `i` = segment `i`.
    Segmented { labels: Vec<String>, active: usize },
    /// Slot 0 = `◀`, slot 1 = `▶`.
    Cyclic { values: Vec<String>, current: usize },
    /// Slot 0. `state.checked` selects the glyph.
    Checkbox { label: String },
    /// Label column then `[text ☰]`. Slot 0 = field, slot 1 = formula button.
    Numeric { label: String, text: String },
    /// Label column then `[text]`. Slot 0.
    Text { label: String, text: String },
    /// Buttons side by side, e.g. OK / Cancel. Slot `i` = button `i`.
    Buttons { labels: Vec<String> },
}

/// One row of an [`ActionPanel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelRow {
    /// Stable identity of the row, chosen by the caller; derives the widget ids.
    pub key: u32,
    /// Cells of left indentation (e.g. for rows disclosed by the row above).
    pub indent: u32,
    pub state: WidgetState,
    pub kind: PanelRowKind,
}

impl PanelRow {
    #[must_use]
    pub fn new(key: u32, kind: PanelRowKind) -> Self {
        Self {
            key,
            indent: 0,
            state: WidgetState::default(),
            kind,
        }
    }

    #[must_use]
    pub fn indented(mut self, indent: u32) -> Self {
        self.indent = indent;
        self
    }

    #[must_use]
    pub fn with_state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Number of id slots the row's controls use.
    fn slots(&self) -> usize {
        let cap = ROW_ID_STRIDE as usize;
        match &self.kind {
            PanelRowKind::Label(_) | PanelRowKind::Separator => 0,
            PanelRowKind::Segmented { labels, .. } => labels.len().min(cap),
            PanelRowKind::Buttons { labels } => labels.len().min(cap),
            PanelRowKind::Cyclic { .. } | PanelRowKind::Numeric { .. } => 2,
            PanelRowKind::Checkbox { .. } | PanelRowKind::Text { .. } => 1,
        }
    }
}

/// The plain state of a panel: title and the rows currently applicable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionPanel {
    pub title: String,
    /// First id of this panel; panels use disjoint ranges by choosing distinct bases.
    pub base_id: u32,
    /// Width of the label column of [`PanelRowKind::Numeric`] / [`PanelRowKind::Text`] rows.
    pub label_width: u32,
    pub rows: Vec<PanelRow>,
}

impl ActionPanel {
    #[must_use]
    pub fn new(title: impl Into<String>, base_id: u32) -> Self {
        Self {
            title: title.into(),
            base_id,
            label_width: 12,
            rows: Vec::new(),
        }
    }

    /// Appends a row (rows are drawn in push order).
    pub fn push(&mut self, row: PanelRow) {
        self.rows.push(row);
    }

    /// Id of control `slot` of the row with `key`, or `None` if `slot` is not below
    /// [`ROW_ID_STRIDE`] or the id would overflow.
    #[must_use]
    pub fn widget_id(&self, key: u32, slot: u32) -> Option<WidgetId> {
        if slot >= ROW_ID_STRIDE {
            return None;
        }
        key.checked_mul(ROW_ID_STRIDE)?
            .checked_add(slot)?
            .checked_add(self.base_id)
            .map(WidgetId::new)
    }

    /// Ids of every slot of `row`, or `None` if any overflows.
    fn row_ids(&self, row: &PanelRow) -> Option<Vec<WidgetId>> {
        (0..row.slots())
            .map(|slot| self.widget_id(row.key, u32::try_from(slot).ok()?))
            .collect()
    }

    /// Composes the panel into `rect` of `grid`. Returns how many rows were laid out inside
    /// `rect` (skipped and clipped rows are not counted; cells off the grid are dropped).
    pub fn draw(&self, grid: &mut CellGrid, ctx: &WidgetContext<'_>, rect: GridRect) -> usize {
        let visible = rect.intersection(&GridRect::from_grid(grid));
        for row in visible.row()..visible.row().saturating_add(visible.height()) {
            for col in visible.col()..visible.col().saturating_add(visible.width()) {
                if let Some(slot) = grid.get_mut(col, row) {
                    *slot = Cell::default();
                }
            }
        }
        draw_border(grid, rect, BorderLine::Single, ctx.styles.normal);
        let inner = rect.inset(1);
        if inner.is_empty() {
            return 0;
        }
        draw_label(grid, inner, &self.title, ctx.styles.focused);
        let (_, body) = inner.split_top(1);
        let (sep, body) = body.split_top(1);
        draw_separator(
            grid,
            sep,
            BorderAxis::Horizontal,
            BorderLine::Single,
            ctx.styles.normal,
        );

        let mut drawn = 0usize;
        let mut seen: Vec<u32> = Vec::new();
        for row in &self.rows {
            let (line, _) = body.split_top(1);
            let at = body
                .row()
                .saturating_add(u32::try_from(drawn).unwrap_or(u32::MAX));
            let line = GridRect::new(line.col(), at, line.width(), line.height());
            if seen.contains(&row.key) {
                continue;
            }
            let Some(ids) = self.row_ids(row) else {
                continue;
            };
            seen.push(row.key);
            if drawn >= body.height() as usize {
                break;
            }
            self.draw_row(grid, ctx, line, row, &ids);
            drawn += 1;
        }
        drawn
    }

    fn draw_row(
        &self,
        grid: &mut CellGrid,
        ctx: &WidgetContext<'_>,
        line: GridRect,
        row: &PanelRow,
        ids: &[WidgetId],
    ) {
        let (_, area) = line.split_left(row.indent);
        let state = row.state;
        match &row.kind {
            PanelRowKind::Label(text) => draw_label(grid, area, text, ctx.plain_style(state)),
            PanelRowKind::Separator => draw_separator(
                grid,
                area,
                BorderAxis::Horizontal,
                BorderLine::Single,
                ctx.styles.normal,
            ),
            PanelRowKind::Segmented { labels, active } => {
                let segments: Vec<Segment<'_>> = labels
                    .iter()
                    .zip(ids)
                    .map(|(label, &id)| Segment { id, label })
                    .collect();
                let sel = Segmented {
                    segments: &segments,
                    active: *active,
                    state,
                };
                draw_segmented(grid, ctx, area, &sel);
            }
            PanelRowKind::Cyclic { values, current } => {
                let values: Vec<&str> = values.iter().map(String::as_str).collect();
                let sel = Cyclic {
                    prev: ids[0],
                    next: ids[1],
                    values: &values,
                    current: *current,
                    state,
                };
                draw_cyclic(grid, ctx, area, &sel);
            }
            PanelRowKind::Checkbox { label } => {
                draw_checkbox(grid, ctx, area, ids[0], state, label);
            }
            PanelRowKind::Numeric { label, text } => {
                let (name, field) = area.split_left(self.label_width);
                draw_label(grid, name, label, ctx.plain_style(state));
                draw_numeric_field(grid, ctx, field, ids[0], ids[1], state, text);
            }
            PanelRowKind::Text { label, text } => {
                let (name, field) = area.split_left(self.label_width);
                draw_label(grid, name, label, ctx.plain_style(state));
                draw_text_field(grid, ctx, field, ids[0], state, text);
            }
            PanelRowKind::Buttons { labels } => {
                let n = u32::try_from(ids.len()).unwrap_or(u32::MAX).max(1);
                let avail = area.width().saturating_sub(n - 1);
                let (each, extra) = (avail / n, avail % n);
                let mut col = u64::from(area.col());
                for (i, (label, &id)) in labels.iter().zip(ids).enumerate() {
                    let w = each + u32::from(u32::try_from(i).is_ok_and(|i| i < extra));
                    let col32 = u32::try_from(col).unwrap_or(u32::MAX);
                    draw_button(
                        grid,
                        ctx,
                        GridRect::new(col32, area.row(), w, 1),
                        id,
                        state,
                        label,
                    );
                    col += u64::from(w) + 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;
    use crate::{InteractionState, SharedBorder, StyleId, WidgetStyles};

    fn s(t: &str) -> String {
        t.to_owned()
    }

    /// Mock command state: a mode selector, an option checkbox, and the rows it discloses.
    #[derive(Clone, Copy)]
    struct Mock {
        mode: usize,
        option: bool,
        amount: bool,
    }

    const K_MODE: u32 = 0;
    const K_OPTION: u32 = 1;
    const K_KIND: u32 = 2;
    const K_AMOUNT: u32 = 3;
    const K_DONE: u32 = 9;

    fn compose(m: Mock) -> ActionPanel {
        let mut p = ActionPanel::new("Mock", 0);
        p.push(PanelRow::new(
            K_MODE,
            PanelRowKind::Segmented {
                labels: vec![s("A"), s("B")],
                active: m.mode,
            },
        ));
        p.push(
            PanelRow::new(K_OPTION, PanelRowKind::Checkbox { label: s("Opt") }).with_state(
                WidgetState {
                    checked: m.option,
                    ..WidgetState::default()
                },
            ),
        );
        if m.option {
            p.push(
                PanelRow::new(
                    K_KIND,
                    PanelRowKind::Cyclic {
                        values: vec![s("One"), s("Two")],
                        current: 0,
                    },
                )
                .indented(2),
            );
            if m.amount {
                p.push(
                    PanelRow::new(
                        K_AMOUNT,
                        PanelRowKind::Numeric {
                            label: s("Amt"),
                            text: s("1 mm"),
                        },
                    )
                    .indented(2),
                );
            }
        }
        p.push(PanelRow::new(
            K_DONE,
            PanelRowKind::Buttons {
                labels: vec![s("OK"), s("Cancel")],
            },
        ));
        p
    }

    fn render(p: &ActionPanel, cols: u32, rows: u32) -> (CellGrid, usize) {
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let mut g = grid(cols, rows);
        let n = p.draw(
            &mut g,
            &ctx(&styles, &inter),
            GridRect::new(0, 0, cols, rows),
        );
        (g, n)
    }

    fn texts(g: &CellGrid) -> Vec<String> {
        (0..g.rows()).map(|r| text_row(g, r)).collect()
    }

    fn all_ids(g: &CellGrid) -> Vec<(u32, u32, u32)> {
        let mut v = Vec::new();
        for r in 0..g.rows() {
            for c in 0..g.cols() {
                if let Some(id) = g.get(c, r).and_then(|x| x.widget_id) {
                    v.push((r, c, id.get()));
                }
            }
        }
        v
    }

    fn ids_of(g: &CellGrid) -> std::collections::BTreeSet<u32> {
        all_ids(g).into_iter().map(|(_, _, i)| i).collect()
    }

    const BASE: Mock = Mock {
        mode: 0,
        option: false,
        amount: false,
    };

    #[test]
    fn rows_are_drawn_in_list_order_inside_a_border() {
        let (g, n) = render(&compose(BASE), 24, 8);
        assert_eq!(n, 3);
        let t = texts(&g);
        assert!(t[0].starts_with('┌') && t[0].ends_with('┐'));
        assert!(t[1].contains("Mock"));
        assert!(t[2].contains('─'));
        // Row text: segmented, checkbox, buttons - in that order, one row each.
        assert!(t[3].contains('A') && t[3].contains('B') && t[3].contains('│'));
        assert!(t[4].contains("□.Opt"));
        assert!(t[5].contains("OK") && t[5].contains("Cancel"));
        assert_eq!(t[6], "│......................│");
    }

    #[test]
    fn border_is_a_real_frame() {
        let (g, _) = render(&compose(BASE), 24, 8);
        assert_eq!(g.get(0, 0).unwrap().content, crate::CellContent::Glyph('┌'));
        assert_eq!(
            g.get(23, 7).unwrap().content,
            crate::CellContent::Glyph('┘')
        );
    }

    #[test]
    fn rows_insert_and_remove_without_reserved_slots() {
        let closed = render(&compose(BASE), 24, 10);
        let open = render(
            &compose(Mock {
                option: true,
                ..BASE
            }),
            24,
            10,
        );
        let deep = render(
            &compose(Mock {
                option: true,
                amount: true,
                ..BASE
            }),
            24,
            10,
        );
        assert_eq!((closed.1, open.1, deep.1), (3, 4, 5));
        let (tc, to, td) = (texts(&closed.0), texts(&open.0), texts(&deep.0));
        // Opening inserts a physical row: the buttons move down, nothing sits in a gap.
        assert!(tc[5].contains("OK") && tc[6].trim_matches(['.', '│']).is_empty());
        assert!(to[5].contains("One") && to[6].contains("OK"));
        assert!(td[6].contains("Amt") && td[7].contains("OK"));
        // Closing again restores exactly the closed rendering (no residue).
        let back = render(&compose(BASE), 24, 10);
        assert_eq!(back.0, closed.0);
    }

    #[test]
    fn closed_state_has_no_cells_for_disclosed_rows() {
        let (g, _) = render(&compose(BASE), 24, 10);
        let ids = ids_of(&g);
        let p = compose(BASE);
        for slot in 0..2 {
            assert!(!ids.contains(&p.widget_id(K_KIND, slot).unwrap().get()));
            assert!(!ids.contains(&p.widget_id(K_AMOUNT, slot).unwrap().get()));
        }
    }

    #[test]
    fn widget_ids_are_stable_when_unrelated_rows_change() {
        let owners = |m: Mock, key: u32| -> Vec<u32> {
            let p = compose(m);
            let (g, _) = render(&p, 24, 10);
            let mine: Vec<u32> = (0..ROW_ID_STRIDE)
                .filter_map(|slot| p.widget_id(key, slot).map(WidgetId::get))
                .collect();
            let mut v: Vec<u32> = ids_of(&g)
                .into_iter()
                .filter(|i| mine.contains(i))
                .collect();
            v.sort_unstable();
            v
        };
        let closed = Mock { ..BASE };
        let open = Mock {
            option: true,
            ..BASE
        };
        for key in [K_MODE, K_OPTION, K_DONE] {
            assert_eq!(owners(closed, key), owners(open, key), "key {key}");
            assert!(!owners(closed, key).is_empty());
        }
        let p = compose(BASE);
        assert_eq!(p.widget_id(K_DONE, 1), Some(WidgetId::new(9 * 64 + 1)));
        assert_eq!(p.widget_id(K_MODE, 64), None);
        assert_eq!(p.widget_id(u32::MAX, 0), None);
    }

    #[test]
    fn row_position_does_not_affect_ids() {
        let p = compose(BASE);
        let (a, _) = render(&p, 24, 10);
        let p2 = compose(Mock {
            option: true,
            ..BASE
        });
        let (b, _) = render(&p2, 24, 10);
        // The buttons row moved one line down but kept its ids.
        let btn = |g: &CellGrid| -> Vec<u32> {
            all_ids(g)
                .into_iter()
                .filter(|(_, _, i)| (9 * 64..10 * 64).contains(i))
                .map(|(_, _, i)| i)
                .collect()
        };
        assert_eq!(btn(&a), btn(&b));
    }

    #[test]
    fn state_transitions_change_only_the_affected_cells() {
        let (a, _) = render(&compose(BASE), 24, 10);
        let (b, _) = render(&compose(Mock { mode: 1, ..BASE }), 24, 10);
        assert_ne!(a, b);
        // Only the segmented row differs.
        let row = |g: &CellGrid, r: u32| -> Vec<Cell> {
            (0..24).map(|c| *g.get(c, r).unwrap()).collect()
        };
        for r in 0..10 {
            assert_eq!(row(&a, r) == row(&b, r), r != 3, "row {r}");
        }
        // The active segment moved: the double edges follow it (no colour needed).
        assert_ne!(text_row(&a, 3), text_row(&b, 3).replace('║', "│"));
        let (c, _) = render(
            &compose(Mock {
                option: true,
                ..BASE
            }),
            24,
            10,
        );
        assert!(text_row(&c, 4).contains("■.Opt"));
    }

    #[test]
    fn clipping_shows_the_first_rows_that_fit() {
        let full = compose(Mock {
            option: true,
            amount: true,
            ..BASE
        });
        // Border 2 + title + separator = 4 rows of chrome; 2 left for rows.
        let (g, n) = render(&full, 24, 6);
        assert_eq!(n, 2);
        assert!(text_row(&g, 3).contains('A'));
        assert!(text_row(&g, 4).contains("□.Opt") || text_row(&g, 4).contains("■.Opt"));
        assert!(!texts(&g).iter().any(|t| t.contains("OK")));
        // Never writes outside the rect.
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let mut big = grid(24, 10);
        full.draw(&mut big, &ctx(&styles, &inter), GridRect::new(2, 1, 20, 6));
        assert_eq!(text_row(&big, 0), "........................");
        assert_eq!(text_row(&big, 7), "........................");
        assert!(text_row(&big, 3).starts_with(".."));
        assert!(text_row(&big, 3).ends_with(".."));
    }

    #[test]
    fn tiny_and_off_grid_panels_do_not_panic() {
        let p = compose(Mock {
            option: true,
            amount: true,
            ..BASE
        });
        for (w, h) in [(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 3)] {
            let (g, n) = render(&p, w, h);
            assert_eq!(n, 0, "{w}x{h}");
            assert!(all_ids(&g).is_empty(), "{w}x{h}");
        }
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let mut g = grid(6, 4);
        let before = g.clone();
        p.draw(&mut g, &ctx(&styles, &inter), GridRect::new(50, 50, 10, 10));
        assert_eq!(g, before);
        // Narrow panel: rows that cannot fit draw nothing but never panic.
        let (_, n) = render(&p, 6, 10);
        assert_eq!(n, 5);
        p.draw(
            &mut g,
            &ctx(&styles, &inter),
            GridRect::new(u32::MAX - 2, 0, u32::MAX, u32::MAX),
        );
    }

    #[test]
    fn full_overwrite_leaves_no_stale_metadata() {
        let p = compose(Mock {
            option: true,
            amount: true,
            ..BASE
        });
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let stale = Cell {
            content: crate::CellContent::Glyph('#'),
            style: StyleId::new(3),
            widget_id: Some(id(9999)),
            shared_border: Some(SharedBorder {
                axis: crate::BorderAxis::Vertical,
                low: Some(id(8)),
                high: Some(id(7)),
            }),
        };
        let mut dirty = grid(24, 10);
        for r in 0..10 {
            for c in 0..24 {
                *dirty.get_mut(c, r).unwrap() = stale;
            }
        }
        let rect = GridRect::new(0, 0, 24, 10);
        p.draw(&mut dirty, &ctx(&styles, &inter), rect);
        let (clean, _) = render(&p, 24, 10);
        assert_eq!(dirty, clean);
        // Shrinking the row list erases the rows that disappeared.
        let small = compose(BASE);
        small.draw(&mut dirty, &ctx(&styles, &inter), rect);
        assert_eq!(dirty, render(&small, 24, 10).0);
    }

    #[test]
    fn duplicate_and_overflowing_keys_are_skipped_without_taking_space() {
        let mut p = ActionPanel::new("T", 0);
        p.push(PanelRow::new(
            1,
            PanelRowKind::Checkbox { label: s("first") },
        ));
        p.push(PanelRow::new(
            1,
            PanelRowKind::Checkbox { label: s("dupe") },
        ));
        p.push(PanelRow::new(
            u32::MAX,
            PanelRowKind::Checkbox { label: s("huge") },
        ));
        p.push(PanelRow::new(2, PanelRowKind::Label(s("last"))));
        let (g, n) = render(&p, 12, 8);
        assert_eq!(n, 2);
        let t = texts(&g);
        assert!(t[3].contains("first"));
        assert!(t[4].contains("last"));
        assert!(!t.iter().any(|r| r.contains("dupe") || r.contains("huge")));
    }

    #[test]
    fn rows_compose_from_m2_7_primitives() {
        // Each row's cells equal what the primitive draws alone in the same rect.
        let p = compose(Mock {
            option: true,
            amount: true,
            ..BASE
        });
        let (g, _) = render(&p, 24, 10);
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let c = ctx(&styles, &inter);
        let mut want = grid(24, 10);
        let st = WidgetState::default();
        let seg = [
            Segment {
                id: p.widget_id(K_MODE, 0).unwrap(),
                label: "A",
            },
            Segment {
                id: p.widget_id(K_MODE, 1).unwrap(),
                label: "B",
            },
        ];
        draw_segmented(
            &mut want,
            &c,
            GridRect::new(1, 3, 22, 1),
            &Segmented {
                segments: &seg,
                active: 0,
                state: st,
            },
        );
        draw_checkbox(
            &mut want,
            &c,
            GridRect::new(1, 4, 22, 1),
            p.widget_id(K_OPTION, 0).unwrap(),
            WidgetState {
                checked: true,
                ..st
            },
            "Opt",
        );
        draw_numeric_field(
            &mut want,
            &c,
            GridRect::new(15, 6, 8, 1),
            p.widget_id(K_AMOUNT, 0).unwrap(),
            p.widget_id(K_AMOUNT, 1).unwrap(),
            st,
            "1 mm",
        );
        for (r, c0, w) in [(3, 1, 22), (4, 1, 22), (6, 15, 8)] {
            for col in c0..c0 + w {
                assert_eq!(g.get(col, r), want.get(col, r), "row {r} col {col}");
            }
        }
    }

    #[test]
    fn buttons_split_the_width_with_a_gap() {
        let mut p = ActionPanel::new("T", 0);
        p.push(PanelRow::new(
            0,
            PanelRowKind::Buttons {
                labels: vec![s("OK"), s("Cancel")],
            },
        ));
        let (g, _) = render(&p, 14, 6);
        // inner width 12: 11 cells to share, first button 6, gap, second 5.
        assert_eq!(owner_row(&g, 3), "-000000-11111-");
    }

    #[test]
    fn repeated_composition_is_deterministic() {
        let p = compose(Mock {
            option: true,
            amount: true,
            ..BASE
        });
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let rect = GridRect::new(0, 0, 24, 10);
        let mut g = grid(24, 10);
        p.draw(&mut g, &ctx(&styles, &inter), rect);
        let first = g.clone();
        p.draw(&mut g, &ctx(&styles, &inter), rect);
        assert_eq!(g, first);
        assert_eq!(render(&p, 24, 10).0, first);
    }

    #[test]
    fn empty_panel_and_zero_sized_rects_draw_only_chrome_or_nothing() {
        let empty = ActionPanel::new("E", 0);
        let (g, n) = render(&empty, 10, 6);
        assert_eq!(n, 0);
        assert!(all_ids(&g).is_empty());
        assert!(text_row(&g, 1).contains('E'));
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let mut g = grid(6, 4);
        let before = g.clone();
        for (w, h) in [(0, 0), (0, 4), (6, 0)] {
            compose(BASE).draw(&mut g, &ctx(&styles, &inter), GridRect::new(0, 0, w, h));
            assert_eq!(g, before, "{w}x{h}");
        }
    }

    #[test]
    fn very_many_rows_are_clipped_without_growth() {
        let mut p = ActionPanel::new("Many", 0);
        for key in 0..20_000 {
            p.push(PanelRow::new(key, PanelRowKind::Checkbox { label: s("x") }));
        }
        let (g, n) = render(&p, 12, 9);
        // 9 rows - 2 border - title - separator.
        assert_eq!(n, 5);
        assert_eq!(all_ids(&g).len(), 5 * 10);
        assert_eq!(g.cells().len(), 12 * 9);
    }
}
