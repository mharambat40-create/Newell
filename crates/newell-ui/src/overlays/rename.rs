// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! The `Rename` modal (M2.20): one single-line text field with OK / Cancel.
//!
//! The artwork is `docs/ui_design/Interface_Elements.txt`, "Validation Operation Box",
//! reproduced cell for cell (`README_UI.md` §23):
//!
//! ```text
//! ╭─Rename──────────────────╮
//! │[Rim_Body               ]│
//! │─────────────────────────│
//! │     OK     │   Cancel   │
//! ╰─────────────────────────╯
//! ```
//!
//! [`RenameModal::open`] starts from the full stored name. OK applies the draft (a name that is
//! empty after trimming is refused and the modal stays open); Cancel and Escape keep the old
//! name. The modal only edits text: renaming a tree row is the shell's job. Lifecycle, focus
//! restoration and key routing follow [`super::FormulaEditor`].
//!
//! The caret `▏` (`README_UI.md` §5.11) is composed into the field while the field has focus. It
//! sits after the last character and has its own cell; when the text is wider than the field the
//! visible window starts at `max(0, c + 1 − N)`, so the caret stays visible. The draft never
//! contains the caret.

use super::modal::{ModalKey, ModalStatus, step_focus};
use super::{Overlay, OverlayKind, OverlayStack, center_modal, compose_overlays};
use crate::widgets::{Canvas, WidgetContext, hover_row, shared};
use crate::{BorderAxis, CellGrid, GridRect, InteractionState, SharedBorder, WidgetId};

/// Frame size, including the border.
const SIZE: (u32, u32) = (27, 5);
/// Cells of the text between the field's `[` and `]`.
const FIELD_CELLS: usize = 23;
/// The text-editing caret (`README_UI.md` §5.11).
const CARET: char = '▏';
const TITLE: &str = "Rename";

/// Rows of the artwork.
const FIELD_ROW: u32 = 1;
const SEPARATOR_ROW: u32 = 2;
const BUTTON_ROW: u32 = 3;

/// Offsets from the base id.
const SURFACE: u32 = 0;
const TEXT: u32 = 1;
const OK: u32 = 2;
const CANCEL: u32 = 3;

/// What a widget id of this modal refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Surface,
    Text,
    Ok,
    Cancel,
}

/// The Rename modal state. Widget ids are `base_id` plus a fixed offset per control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameModal {
    base_id: u32,
    applied: String,
    draft: String,
    focus: Option<WidgetId>,
    return_focus: Option<WidgetId>,
    status: ModalStatus,
}

impl RenameModal {
    /// A closed modal. `base_id` is the id of the modal surface; the controls use the ids
    /// that follow it.
    #[must_use]
    pub const fn new(base_id: u32) -> Self {
        Self {
            base_id,
            applied: String::new(),
            draft: String::new(),
            focus: None,
            return_focus: None,
            status: ModalStatus::Closed,
        }
    }

    /// Whether the modal is shown and taking input.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status == ModalStatus::Open
    }

    /// Lifecycle state, including the outcome of the last close.
    #[must_use]
    pub const fn status(&self) -> ModalStatus {
        self.status
    }

    /// The name applied by the last OK.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.applied
    }

    /// The name being edited while open.
    #[must_use]
    pub fn draft(&self) -> &str {
        &self.draft
    }

    /// The id that holds keyboard focus while the modal is open.
    #[must_use]
    pub const fn focus(&self) -> Option<WidgetId> {
        self.focus
    }

    /// Id of the modal surface (the frame, including its border).
    #[must_use]
    pub const fn surface(&self) -> WidgetId {
        WidgetId::new(self.base_id)
    }

    /// Opens the modal on the complete stored `name` and focuses
    /// the field. Focus held by `inter` is remembered and restored when the modal closes;
    /// opening an open modal, or one whose focus has not been synced away yet, keeps the
    /// earlier memory.
    pub fn open(&mut self, inter: &mut InteractionState, name: &str) {
        let stale = inter.focused.is_some_and(|f| self.slot(f).is_some());
        if !self.is_open() && !stale {
            self.return_focus = inter.focused;
        }
        self.applied = name.to_owned();
        self.draft.clone_from(&self.applied);
        self.status = ModalStatus::Open;
        self.focus = Some(self.id(TEXT));
        inter.set_focused(self.focus);
    }

    /// Applies a keyboard key. Returns `true` if the key was handled; keys are ignored while
    /// the modal is closed. In the field characters are inserted, Backspace deletes and Enter
    /// confirms; on a button Enter and Space activate. Focus moves with Tab/Down and
    /// BackTab/Up.
    pub fn key(&mut self, key: ModalKey) -> bool {
        if !self.is_open() {
            return false;
        }
        match key {
            ModalKey::Escape => {
                self.cancel();
                true
            }
            ModalKey::Tab | ModalKey::Down => self.move_focus(true),
            ModalKey::BackTab | ModalKey::Up => self.move_focus(false),
            ModalKey::Enter if self.on_text() => {
                self.confirm();
                true
            }
            ModalKey::Char(c) if self.on_text() => self.push(c),
            ModalKey::Backspace if self.on_text() => self.draft.pop().is_some(),
            ModalKey::Enter | ModalKey::Char(' ') => self.activate_focus(),
            ModalKey::Char(_) | ModalKey::Backspace | ModalKey::Edit(_) => false,
        }
    }

    /// Activates control `id` (pointer or keyboard). The field takes focus; OK confirms and
    /// Cancel closes. Ids outside the modal, and anything while closed, are refused.
    pub fn activate(&mut self, id: WidgetId) -> bool {
        if !self.is_open() {
            return false;
        }
        match self.slot(id) {
            Some(Slot::Text) => self.focus = Some(id),
            Some(Slot::Ok) => {
                self.focus = Some(id);
                self.confirm();
            }
            Some(Slot::Cancel) => {
                self.focus = Some(id);
                self.cancel();
            }
            Some(Slot::Surface) | None => return false,
        }
        true
    }

    /// Opens the modal in `stack` while it is open, or dismisses it.
    pub fn sync_overlay(&self, stack: &mut OverlayStack, bounds: GridRect) {
        if self.is_open() {
            stack.open(Overlay {
                id: self.surface(),
                kind: OverlayKind::Modal,
                rect: center_modal(SIZE, bounds),
            });
        } else {
            stack.dismiss(self.surface());
        }
    }

    /// Mirrors the modal's focus into `inter`. After a close, a focus still held by one of this
    /// modal's ids is replaced by the focus that was active when it opened.
    pub fn sync_focus(&self, inter: &mut InteractionState) {
        if self.is_open() {
            inter.set_focused(self.focus);
        } else if inter.focused.is_some_and(|f| self.slot(f).is_some()) {
            inter.set_focused(self.return_focus);
        }
    }

    /// Draws the modal into the frame that `stack` holds for it. Returns `true` if the hovered
    /// button was highlighted (M2.25).
    pub fn draw(
        &self,
        grid: &mut CellGrid,
        ctx: &WidgetContext<'_>,
        stack: &OverlayStack,
        bounds: GridRect,
    ) -> bool {
        let mut lit = false;
        compose_overlays(grid, ctx, stack, bounds, |g, overlay, _inner| {
            if overlay.id == self.surface() {
                let rect = overlay.rect.intersection(&bounds);
                self.draw_art(g, ctx, rect);
                lit |= self.hover_buttons(g, ctx, rect);
            }
        });
        lit
    }

    /// Substitutes the frame of the hovered button of the OK / Cancel row (M2.25,
    /// `README_UI.md` §5.12): OK is `│     OK     │` and Cancel `│   Cancel   │`, the divider
    /// between them is its shared slot and the last cell of the row the frame.
    fn hover_buttons(&self, grid: &mut CellGrid, ctx: &WidgetContext<'_>, rect: GridRect) -> bool {
        let row = rect.row().saturating_add(BUTTON_ROW);
        let x = rect.col();
        let target = match ctx.interaction.hovered {
            Some(id) if id == self.id(OK) => 1..13,
            Some(id) if id == self.id(CANCEL) => 14..26,
            _ => return false,
        };
        let (from, span) = (
            x,
            x.saturating_add(target.start)..x.saturating_add(target.end),
        );
        hover_row(grid, row, from, span)
    }

    /// The reference artwork over `rect`, clipped to it. The frame cells are owned by the
    /// surface, the field by its own id, each button by its id and the `│` between the buttons
    /// is a shared border routed by sub-cell position.
    fn draw_art(&self, grid: &mut CellGrid, ctx: &WidgetContext<'_>, rect: GridRect) {
        let style = ctx.styles.normal;
        let (surface, field) = (Some(self.surface()), Some(self.id(TEXT)));
        let (ok, cancel) = (self.id(OK), self.id(CANCEL));
        let inner = (SIZE.0 - 2) as usize;
        let mut canvas = Canvas::new(grid, rect);

        let fill = "─".repeat(inner - TITLE.chars().count() - 1);
        let title_end = canvas.contour(0, 0, "╭─", style, surface);
        let title_end = canvas.text(title_end, 0, TITLE, style, surface);
        canvas.contour(title_end, 0, &format!("{fill}╮"), style, surface);

        canvas.contour(0, FIELD_ROW, "│", style, surface);
        let shown = field_window(&self.draft, self.on_text());
        canvas.contour(1, FIELD_ROW, "[", style, field);
        let end = canvas.text(2, FIELD_ROW, &shown, style, field);
        canvas.contour(end, FIELD_ROW, "]", style, field);
        canvas.contour(SIZE.0 - 1, FIELD_ROW, "│", style, surface);

        canvas.contour(
            0,
            SEPARATOR_ROW,
            &format!("│{}│", "─".repeat(inner)),
            style,
            surface,
        );

        canvas.contour(0, BUTTON_ROW, "│", style, surface);
        canvas.text(1, BUTTON_ROW, "     OK     ", style, Some(ok));
        let border = SharedBorder {
            axis: BorderAxis::Vertical,
            low: Some(ok),
            high: Some(cancel),
        };
        canvas.put(13, BUTTON_ROW, shared('│', style, border));
        canvas.text(14, BUTTON_ROW, "   Cancel   ", style, Some(cancel));
        canvas.contour(SIZE.0 - 1, BUTTON_ROW, "│", style, surface);

        canvas.contour(
            0,
            SIZE.1 - 1,
            &format!("╰{}╯", "─".repeat(inner)),
            style,
            surface,
        );
    }

    fn move_focus(&mut self, forward: bool) -> bool {
        let ring = [self.id(TEXT), self.id(OK), self.id(CANCEL)];
        self.focus = step_focus(&ring, self.focus, forward);
        true
    }

    fn activate_focus(&mut self) -> bool {
        self.focus.is_some_and(|id| self.activate(id))
    }

    /// Appends a character; clipping belongs to the field's visible window only.
    fn push(&mut self, c: char) -> bool {
        self.draft.push(c);
        true
    }

    fn on_text(&self) -> bool {
        self.focus == Some(self.id(TEXT))
    }

    /// Applies the complete draft. A name that is empty after trimming is refused: the modal
    /// stays open.
    fn confirm(&mut self) {
        let name = self.draft.trim();
        if name.is_empty() {
            return;
        }
        self.applied.clone_from(&self.draft);
        self.status = ModalStatus::Confirmed;
    }

    pub(crate) fn cancel(&mut self) {
        self.draft.clone_from(&self.applied);
        self.status = ModalStatus::Cancelled;
    }

    fn id(&self, offset: u32) -> WidgetId {
        WidgetId::new(self.base_id.wrapping_add(offset))
    }

    fn slot(&self, id: WidgetId) -> Option<Slot> {
        match id.get().wrapping_sub(self.base_id) {
            SURFACE => Some(Slot::Surface),
            TEXT => Some(Slot::Text),
            OK => Some(Slot::Ok),
            CANCEL => Some(Slot::Cancel),
            _ => None,
        }
    }
}

/// The [`FIELD_CELLS`] cells of the field for `text`: with the caret after the last character
/// when `caret`, scrolled by `max(0, c + 1 − N)` so the caret stays visible (`README_UI.md`
/// §5.11), else the start of the text. Padded with spaces to the field width.
fn field_window(text: &str, caret: bool) -> String {
    let mut cells: Vec<char> = text.chars().collect();
    let start = if caret {
        cells.push(CARET);
        cells.len().saturating_sub(FIELD_CELLS)
    } else {
        0
    };
    let mut shown: String = cells.into_iter().skip(start).take(FIELD_CELLS).collect();
    let pad = FIELD_CELLS.saturating_sub(shown.chars().count());
    shown.extend(std::iter::repeat_n(' ', pad));
    shown
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::WidgetStyles;
    use crate::widgets::testing::{id, text_row};

    const BASE: u32 = 800;

    fn bounds() -> GridRect {
        GridRect::new(0, 0, 40, 12)
    }

    fn open_modal(name: &str) -> (RenameModal, InteractionState) {
        let mut inter = InteractionState::default();
        inter.set_focused(Some(id(5)));
        let mut m = RenameModal::new(BASE);
        m.open(&mut inter, name);
        (m, inter)
    }

    fn typed(m: &mut RenameModal, s: &str) {
        for c in s.chars() {
            m.key(ModalKey::Char(c));
        }
    }

    fn compose(m: &RenameModal) -> (CellGrid, GridRect) {
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let ctx = WidgetContext {
            styles: &styles,
            interaction: &inter,
        };
        let mut stack = OverlayStack::default();
        m.sync_overlay(&mut stack, bounds());
        let mut g = CellGrid::new(40, 12).unwrap();
        m.draw(&mut g, &ctx, &stack, bounds());
        (g, center_modal(SIZE, bounds()))
    }

    fn lines(g: &CellGrid, at: GridRect) -> Vec<String> {
        (at.row()..at.row() + at.height())
            .map(|r| {
                text_row(g, r)
                    .chars()
                    .skip(at.col() as usize)
                    .take(at.width() as usize)
                    .map(|c| if c == '.' { ' ' } else { c })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn artwork_is_the_reference_with_the_caret_in_the_field() {
        let (m, _) = open_modal("Rim_Body");
        let (g, at) = compose(&m);
        assert_eq!(
            lines(&g, at),
            [
                "╭─Rename──────────────────╮",
                "│[Rim_Body▏              ]│",
                "│─────────────────────────│",
                "│     OK     │   Cancel   │",
                "╰─────────────────────────╯",
            ]
        );
    }

    #[test]
    fn caret_is_only_drawn_while_the_field_has_focus() {
        let (mut m, _) = open_modal("Rim_Body");
        m.key(ModalKey::Tab);
        let (g, at) = compose(&m);
        assert_eq!(lines(&g, at)[1], "│[Rim_Body               ]│");
        assert!(!m.draft().contains(CARET));
    }

    #[test]
    fn full_field_scrolls_so_the_caret_stays_visible() {
        let n = FIELD_CELLS;
        // N - 1 characters plus the caret fill the field exactly.
        let almost = "a".repeat(n - 1);
        assert_eq!(field_window(&almost, true), format!("{almost}▏"));
        // N characters: the first one leaves the left boundary.
        let full = "b".repeat(n - 1) + "c";
        assert_eq!(
            field_window(&full, true),
            format!("{}c▏", "b".repeat(n - 2))
        );
        // N + 1 characters: one more cell scrolls.
        let over = "d".repeat(n - 1) + "ef";
        assert_eq!(
            field_window(&over, true),
            format!("{}ef▏", "d".repeat(n - 3))
        );
        assert_eq!(field_window("", true), format!("▏{}", " ".repeat(n - 1)));
        assert_eq!(field_window("abc", false).chars().count(), n);
    }

    #[test]
    fn opens_on_the_full_name_and_enter_confirms() {
        let (mut m, mut inter) = open_modal("Rim_Body");
        assert!(m.is_open() && m.draft() == "Rim_Body");
        assert_eq!(m.focus(), Some(m.id(TEXT)));
        m.sync_focus(&mut inter);
        assert_eq!(inter.focused, m.focus());
        typed(&mut m, "_2");
        m.key(ModalKey::Backspace);
        typed(&mut m, "3");
        assert!(m.key(ModalKey::Enter));
        assert_eq!(
            (m.status(), m.text()),
            (ModalStatus::Confirmed, "Rim_Body_3")
        );
        m.sync_focus(&mut inter);
        assert_eq!(inter.focused, Some(id(5)), "focus returns where it was");
    }

    #[test]
    fn escape_and_cancel_keep_the_old_name() {
        let (mut m, mut inter) = open_modal("Hub_Body");
        typed(&mut m, "xyz");
        m.key(ModalKey::Escape);
        assert_eq!(
            (m.status(), m.text(), m.draft()),
            (ModalStatus::Cancelled, "Hub_Body", "Hub_Body")
        );
        m.sync_focus(&mut inter);
        assert_eq!(inter.focused, Some(id(5)));
        m.open(&mut inter, "Hub_Body");
        typed(&mut m, "q");
        assert!(m.activate(m.id(CANCEL)));
        assert_eq!((m.status(), m.text()), (ModalStatus::Cancelled, "Hub_Body"));
    }

    #[test]
    fn an_empty_name_is_refused_and_the_modal_stays_open() {
        let (mut m, _) = open_modal("a");
        m.key(ModalKey::Backspace);
        typed(&mut m, "  ");
        m.key(ModalKey::Enter);
        assert!(m.is_open());
        assert!(m.activate(m.id(OK)));
        assert!(m.is_open());
        m.activate(m.id(TEXT));
        typed(&mut m, " b ");
        m.key(ModalKey::Enter);
        assert_eq!((m.status(), m.text()), (ModalStatus::Confirmed, "   b "));
    }

    #[test]
    fn focus_ring_and_button_keys() {
        let (mut m, _) = open_modal("n");
        m.key(ModalKey::Tab);
        assert_eq!(m.focus(), Some(m.id(OK)));
        m.key(ModalKey::Tab);
        assert_eq!(m.focus(), Some(m.id(CANCEL)));
        m.key(ModalKey::Tab);
        assert_eq!(m.focus(), Some(m.id(TEXT)));
        m.key(ModalKey::BackTab);
        m.key(ModalKey::Enter);
        assert_eq!(m.status(), ModalStatus::Cancelled);
        assert!(!m.key(ModalKey::Char('x')), "closed: keys are ignored");
    }

    #[test]
    fn long_names_round_trip_and_reopening_keeps_the_focus_memory() {
        let name = format!(" {} ", "x".repeat(200));
        let (mut m, mut inter) = open_modal(&name);
        assert_eq!(m.draft(), name);
        assert!(m.key(ModalKey::Enter));
        assert_eq!(m.text(), name);
        m.open(&mut inter, &name);
        assert!(m.key(ModalKey::Char('y')));
        assert_eq!(m.draft(), format!("{name}y"));
        m.open(&mut inter, "again");
        m.key(ModalKey::Escape);
        m.sync_focus(&mut inter);
        assert_eq!(inter.focused, Some(id(5)));
    }

    #[test]
    fn controls_own_their_cells_and_the_surface_blocks_the_rest() {
        let (m, _) = open_modal("n");
        let (g, at) = compose(&m);
        let owner = |dx: u32, dy: u32| g.get(at.col() + dx, at.row() + dy).unwrap().widget_id;
        assert_eq!(owner(0, 0), Some(m.surface()));
        assert_eq!(owner(5, FIELD_ROW), Some(m.id(TEXT)));
        assert_eq!(owner(3, BUTTON_ROW), Some(m.id(OK)));
        assert_eq!(owner(20, BUTTON_ROW), Some(m.id(CANCEL)));
        assert_eq!(owner(13, BUTTON_ROW), None, "routed by sub-cell position");
        assert!(
            g.get(13 + at.col(), at.row() + BUTTON_ROW)
                .unwrap()
                .shared_border
                .is_some()
        );
    }

    #[test]
    fn small_bounds_clip_without_panicking() {
        let (m, _) = open_modal("name");
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let ctx = WidgetContext {
            styles: &styles,
            interaction: &inter,
        };
        for (w, h) in [(0, 0), (1, 1), (10, 3), (26, 4), (27, 5)] {
            let b = GridRect::new(0, 0, w, h);
            let mut stack = OverlayStack::default();
            m.sync_overlay(&mut stack, b);
            let mut g = CellGrid::new(w.max(1), h.max(1)).unwrap();
            m.draw(&mut g, &ctx, &stack, b);
        }
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;
    use crate::{InkRole, InteractionState, WidgetStyles};
    #[test]
    fn title_edges_fields_and_hover_boundaries_have_separate_ink() {
        let mut modal = RenameModal::new(2000);
        let mut inter = InteractionState::default();
        modal.open(&mut inter, "label with │ [+] ☰");
        let styles = WidgetStyles::default();
        let frame = GridRect::new(1, 1, SIZE.0, SIZE.1);
        let mut grid = CellGrid::new(SIZE.0 + 2, SIZE.1 + 2).unwrap();
        modal.draw_art(
            &mut grid,
            &WidgetContext {
                styles: &styles,
                interaction: &inter,
            },
            frame,
        );
        for dx in 0..SIZE.0 {
            let c = grid.get(1 + dx, 1).unwrap();
            assert_eq!(
                c.ink,
                if (2..2 + TITLE.len() as u32).contains(&dx) {
                    InkRole::Text
                } else {
                    InkRole::Contour
                }
            );
            assert_eq!(c.widget_id, Some(modal.surface()));
        }
        assert_eq!(grid.get(0, 1).unwrap(), &crate::Cell::default());
        let mut cropped = CellGrid::new(8, SIZE.1 + 2).unwrap();
        modal.draw_art(
            &mut cropped,
            &WidgetContext {
                styles: &styles,
                interaction: &inter,
            },
            frame,
        );
        for y in 0..cropped.rows() {
            for x in 0..cropped.cols() {
                assert_eq!(cropped.get(x, y), grid.get(x, y));
            }
        }
    }
}
