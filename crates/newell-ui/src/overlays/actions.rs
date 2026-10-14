// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! The OK / Cancel row.

use crate::widgets::{Canvas, WidgetContext, draw_button, shared};
use crate::{BorderAxis, CellGrid, GridRect, SharedBorder, WidgetId, WidgetState};

/// `OK │ Cancel`: two buttons separated by a shared `│` that routes by sub-cell position
/// (M2.5), owning every cell of `rect`.
///
/// `ok_state` lets the caller disable OK; Cancel is always activatable. A rect narrower
/// than 3 cells (one cell per button plus the separator) or with no height draws nothing.
pub fn draw_ok_cancel(
    grid: &mut CellGrid,
    ctx: &WidgetContext<'_>,
    rect: GridRect,
    ok: WidgetId,
    cancel: WidgetId,
    ok_state: WidgetState,
) {
    if rect.width() < 3 || rect.height() == 0 {
        return;
    }
    let (ok_rect, remaining) = rect.split_left((rect.width() - 1) / 2);
    let (sep, cancel_rect) = remaining.split_left(1);
    draw_button(grid, ctx, ok_rect, ok, ok_state, "OK");
    draw_button(
        grid,
        ctx,
        cancel_rect,
        cancel,
        WidgetState::default(),
        "Cancel",
    );
    let border = SharedBorder {
        axis: BorderAxis::Vertical,
        low: Some(ok),
        high: Some(cancel),
    };
    Canvas::new(grid, sep).fill_col(0, shared('│', ctx.styles.normal, border));
}
