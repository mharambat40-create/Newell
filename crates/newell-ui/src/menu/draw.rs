// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Menu drawing: the title bar and the popup interior.

use super::{MenuDef, MenuEntry, MenuIds, MenuState, capped};
use crate::widgets::{
    Canvas, Cyclic, WidgetContext, cell, draw_checkbox, draw_cyclic, hover_row, owned_blank,
    text_len,
};
use crate::{CellContent, CellGrid, GridRect, WidgetId, WidgetState};

/// Cells a cyclic control adds to its widest value: `◀`, two spaces of margin either side of
/// the centred value in an unlabelled row (`◀  Perspective  ▶`)...
const CYCLE_BARE: u32 = 5;
/// ...and `◀`, a space, the value, a space, `▶` in a labelled row (`UI Scale ◀ 100% ▶`).
const CYCLE_LABELLED: u32 = 4;
/// Narrowest popup frame: the File, Edit and Help frames of `Application_Menu_Bar.txt` are all
/// 13 cells wide, wider than File's own longest entry.
const MIN_POPUP_WIDTH: u32 = 13;

fn to_u32(v: u64) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

/// Cells from the start of the bar to the title of menu `index`: each earlier title, its two
/// padding spaces and the separator after it.
fn title_offset(menus: &[MenuDef<'_>], index: usize) -> u64 {
    menus[..index]
        .iter()
        .map(|m| u64::from(text_len(m.label)) + 3)
        .sum()
}

/// Cell rect of the title of menu `index` in `bar`: one space of padding either side, laid
/// out left to right on the first row with a one-cell `│` separator between titles
/// (`║ File │ Edit │ View │ Help`), clipped to `bar`. `None` if `index` is out of range.
#[must_use]
pub fn title_rect(bar: GridRect, menus: &[MenuDef<'_>], index: usize) -> Option<GridRect> {
    let menus = capped(menus);
    let def = menus.get(index)?;
    let col = to_u32(u64::from(bar.col()) + title_offset(menus, index));
    let rect = GridRect::new(col, bar.row(), text_len(def.label).saturating_add(2), 1);
    Some(rect.intersection(&bar))
}

/// Popup frame size `(width, height)` for `def`: widest row (label; checkbox label plus
/// glyph; cyclic widest value plus arrows and spacing, after its label if any) plus padding and
/// border, one row per entry plus border.
#[must_use]
pub fn popup_size(def: &MenuDef<'_>) -> (u32, u32) {
    let widest = def
        .entries()
        .iter()
        .map(|e| match e {
            MenuEntry::Item(item) => text_len(item.label),
            MenuEntry::Checkbox(cb) => text_len(cb.label).saturating_add(2),
            MenuEntry::Cyclic(cy) => {
                let value = cy.values.iter().map(|v| text_len(v)).max().unwrap_or(0);
                match text_len(cy.label) {
                    0 => value.saturating_add(CYCLE_BARE),
                    label => label
                        .saturating_add(1)
                        .saturating_add(value + CYCLE_LABELLED),
                }
            }
            MenuEntry::Separator => 0,
        })
        .max()
        .unwrap_or(0);
    let rows = u32::try_from(def.entries().len()).unwrap_or(u32::MAX);
    (
        widest.saturating_add(4).max(MIN_POPUP_WIDTH),
        rows.saturating_add(2),
    )
}

/// Draws the titles on the first row of `bar`. Each title owns its cells (`ids.title(i)`);
/// the open menu's title uses the selected style. Cells of the row outside titles are blank.
pub fn draw_menu_bar(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    bar: GridRect,
    ids: MenuIds,
    menus: &[MenuDef<'_>],
    state: &MenuState,
) {
    let mut canvas = Canvas::new(grid, bar);
    canvas.fill(
        0,
        canvas.width(),
        0,
        cell(CellContent::Empty, ctx.styles.normal, None),
    );
    for (i, def) in capped(menus).iter().enumerate() {
        let (Some(id), Some(rect)) = (ids.title(i), title_rect(bar, menus, i)) else {
            continue;
        };
        let ws = WidgetState {
            selected: state.open_menu() == Some(i),
            ..WidgetState::default()
        };
        let style = ctx.style_of(id, ws);
        let mut canvas = Canvas::new(grid, rect);
        canvas.fill(0, canvas.width(), 0, owned_blank(style, id));
        canvas.text(1, 0, def.label, style, Some(id));
        if i + 1 < capped(menus).len() {
            // Unclipped end of the title, so a title cut by the bar cannot move its separator.
            let end =
                u64::from(bar.col()) + title_offset(menus, i) + u64::from(text_len(def.label)) + 2;
            let sep = GridRect::new(to_u32(end), bar.row(), 1, 1).intersection(&bar);
            Canvas::new(grid, sep).put(
                0,
                0,
                cell(CellContent::Glyph('│'), ctx.styles.normal, None),
            );
        }
    }
}

/// Draws the open menu's entries into `inner` (the popup interior handed to the
/// [`crate::compose_overlays`] callback). Items own their row (`ids.item`), separators are
/// owned by the popup surface. Checkbox rows reuse [`draw_checkbox`]; cyclic rows reuse
/// [`draw_cyclic`] with the `◀` / `▶` cells owned by `ids.cycle_prev` / `ids.cycle_next` and
/// the value cells owned by the row. Draws nothing if no menu is open. Rows past `inner` are cut.
/// An enabled row holding the hovered target is drawn with its frame substituted (M2.25); the
/// return value tells whether that happened.
pub fn draw_menu_popup(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    inner: GridRect,
    ids: MenuIds,
    menus: &[MenuDef<'_>],
    state: &MenuState,
) -> bool {
    let Some(menu) = state.open_menu() else {
        return false;
    };
    let Some(def) = capped(menus).get(menu) else {
        return false;
    };
    let mut lit = false;
    draw_popup_frame(grid, inner, def.label);
    for (r, entry) in def
        .entries()
        .iter()
        .enumerate()
        .take(inner.height() as usize)
    {
        let row = GridRect::new(
            inner.col(),
            inner.row().saturating_add(to_u32(r as u64)),
            inner.width(),
            1,
        );
        let mut canvas = Canvas::new(grid, row);
        match entry {
            MenuEntry::Checkbox(cb) => {
                let Some(id) = ids.item(menu, r) else {
                    continue;
                };
                let ws = WidgetState {
                    disabled: !cb.enabled,
                    checked: cb.checked,
                    ..WidgetState::default()
                };
                let style = ctx.style_of(id, ws);
                canvas.fill(0, canvas.width(), 0, owned_blank(style, id));
                let inner_row = GridRect::new(
                    row.col().saturating_add(1),
                    row.row(),
                    row.width().saturating_sub(1),
                    1,
                );
                draw_checkbox(grid, ctx, inner_row, id, ws, cb.label);
                if cb.enabled && hovers(ctx, &[Some(id)]) {
                    lit |= hover_menu_row(grid, row);
                }
            }
            MenuEntry::Cyclic(cy) => {
                let (Some(id), Some(prev), Some(next)) = (
                    ids.item(menu, r),
                    ids.cycle_prev(menu, r),
                    ids.cycle_next(menu, r),
                ) else {
                    continue;
                };
                let ws = WidgetState {
                    disabled: !cy.enabled,
                    ..WidgetState::default()
                };
                let style = ctx.style_of(id, ws);
                canvas.fill(0, canvas.width(), 0, owned_blank(style, id));
                let label = text_len(cy.label);
                let used = if label == 0 {
                    0
                } else {
                    label.saturating_add(1)
                };
                if label > 0 {
                    canvas.text(1, 0, cy.label, style, Some(id));
                }
                let ctl = GridRect::new(
                    row.col().saturating_add(1).saturating_add(used),
                    row.row(),
                    row.width().saturating_sub(2).saturating_sub(used),
                    1,
                );
                let sel = Cyclic {
                    prev,
                    next,
                    values: cy.values,
                    current: cy.current,
                    state: ws,
                };
                draw_cyclic(grid, ctx, ctl, &sel);
                // The value area is unowned in `draw_cyclic`; give it to the row, in the
                // row style, so a click there is on the item and the highlight shows.
                for dx in 1..ctl.width().saturating_sub(1) {
                    if let Some(c) = grid.get_mut(ctl.col().saturating_add(dx), ctl.row()) {
                        c.widget_id = Some(id);
                        c.style = style;
                    }
                }
                if cy.enabled && hovers(ctx, &[Some(id), Some(prev), Some(next)]) {
                    lit |= hover_menu_row(grid, row);
                }
            }
            MenuEntry::Item(item) => {
                let Some(id) = ids.item(menu, r) else {
                    continue;
                };
                let ws = WidgetState {
                    disabled: !item.enabled,
                    ..WidgetState::default()
                };
                let style = ctx.style_of(id, ws);
                canvas.fill(0, canvas.width(), 0, owned_blank(style, id));
                canvas.text(1, 0, item.label, style, Some(id));
                if item.enabled && hovers(ctx, &[Some(id)]) {
                    lit |= hover_menu_row(grid, row);
                }
            }
            MenuEntry::Separator => {
                let sep = cell(
                    CellContent::Glyph('─'),
                    ctx.styles.normal,
                    Some(ids.popup()),
                );
                canvas.fill(0, canvas.width(), 0, sep);
            }
        }
    }
    lit
}

/// `true` if the hovered target is one of `ids` (M2.25).
fn hovers(ctx: &WidgetContext<'_>, ids: &[Option<WidgetId>]) -> bool {
    ctx.interaction
        .hovered
        .is_some_and(|hovered| ids.contains(&Some(hovered)))
}

/// Substitutes the frame of menu row `row` (M2.25): the frame cells on either side of it are
/// the slots, and no other cell of the popup is touched.
fn hover_menu_row(grid: &mut CellGrid, row: GridRect) -> bool {
    hover_row(
        grid,
        row.row(),
        row.col().saturating_sub(1),
        row.col()..row.col().saturating_add(row.width()),
    )
}

/// Rounds the frame around `inner` and puts `title` in its top edge (`╭─View────╮`). The
/// overlay frame already owns these cells; only their glyphs change. Cells cut by the grid
/// edge are skipped, and a title longer than the edge is cut.
fn draw_popup_frame(grid: &mut CellGrid, inner: GridRect, title: &str) {
    if inner.width() == 0 || inner.height() == 0 || inner.col() == 0 || inner.row() == 0 {
        return;
    }
    let (left, top) = (inner.col() - 1, inner.row() - 1);
    let right = inner.col().saturating_add(inner.width());
    let bottom = inner.row().saturating_add(inner.height());
    let mut set = |col: u32, row: u32, glyph: char| {
        if let Some(c) = grid.get_mut(col, row) {
            c.content = CellContent::Glyph(glyph);
        }
    };
    set(left, top, '╭');
    set(right, top, '╮');
    set(left, bottom, '╰');
    set(right, bottom, '╯');
    for (dx, glyph) in title.chars().enumerate() {
        let col = left.saturating_add(2).saturating_add(to_u32(dx as u64));
        if col < right {
            set(col, top, glyph);
        }
    }
}
