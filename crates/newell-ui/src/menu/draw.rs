// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Menu drawing: the title bar and the popup interior.

use super::{MenuDef, MenuEntry, MenuIds, MenuState, capped};
use crate::widgets::{
    Canvas, Cyclic, WidgetContext, cell, draw_checkbox, draw_cyclic, owned_blank, text_len,
};
use crate::{CellContent, CellGrid, GridRect, WidgetState};

fn to_u32(v: u64) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

/// Cell rect of the title of menu `index` in `bar`: one space of padding either side, laid
/// out left to right on the first row and clipped to `bar`. `None` if `index` is out of range.
#[must_use]
pub fn title_rect(bar: GridRect, menus: &[MenuDef<'_>], index: usize) -> Option<GridRect> {
    let menus = capped(menus);
    let def = menus.get(index)?;
    let offset: u64 = menus[..index]
        .iter()
        .map(|m| u64::from(text_len(m.label)) + 2)
        .sum();
    let col = to_u32(u64::from(bar.col()) + offset);
    let rect = GridRect::new(col, bar.row(), text_len(def.label).saturating_add(2), 1);
    Some(rect.intersection(&bar))
}

/// Popup frame size `(width, height)` for `def`: widest row (label; checkbox label plus
/// glyph; cyclic widest value plus arrows and spacing) plus padding and border, one row per
/// entry plus border.
#[must_use]
pub fn popup_size(def: &MenuDef<'_>) -> (u32, u32) {
    let widest = def
        .entries()
        .iter()
        .map(|e| match e {
            MenuEntry::Item(item) => text_len(item.label),
            MenuEntry::Checkbox(cb) => text_len(cb.label).saturating_add(2),
            MenuEntry::Cyclic(cy) => cy
                .values
                .iter()
                .map(|v| text_len(v))
                .max()
                .unwrap_or(0)
                .saturating_add(6),
            MenuEntry::Separator => 0,
        })
        .max()
        .unwrap_or(0);
    let rows = u32::try_from(def.entries().len()).unwrap_or(u32::MAX);
    (widest.saturating_add(4), rows.saturating_add(2))
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
    }
}

/// Draws the open menu's entries into `inner` (the popup interior handed to the
/// [`crate::compose_overlays`] callback). Items own their row (`ids.item`), separators are
/// owned by the popup surface. Checkbox rows reuse [`draw_checkbox`]; cyclic rows reuse
/// [`draw_cyclic`] with the `◀` / `▶` cells owned by `ids.cycle_prev` / `ids.cycle_next` and
/// the value cells owned by the row. Draws nothing if no menu is open. Rows past `inner` are cut.
pub fn draw_menu_popup(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    inner: GridRect,
    ids: MenuIds,
    menus: &[MenuDef<'_>],
    state: &MenuState,
) {
    let Some(menu) = state.open_menu() else {
        return;
    };
    let Some(def) = capped(menus).get(menu) else {
        return;
    };
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
                let ctl = GridRect::new(
                    row.col().saturating_add(1),
                    row.row(),
                    row.width().saturating_sub(2),
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
}
