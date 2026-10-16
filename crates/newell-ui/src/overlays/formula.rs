// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! FormulaEditor (M2.12, M2.23): a modal two-row text area with OK / Cancel.
//!
//! The artwork is `docs/ui_design/Interface_Elements.txt`, "☰ variants", with an empty text
//! area (`README_UI.md` §22):
//!
//! ```text
//! ╭─Formula─────────────────────────╮
//! │┌───────────────────────────────┐│
//! ││                               ││
//! ││                               ││
//! │└───────────────────────────────┘│
//! │─────────────────────────────────│
//! │       OK       │     Cancel     │
//! ╰─────────────────────────────────╯
//! ```
//!
//! The editor stores and shows the text verbatim. It has no parser and no evaluator, and
//! nothing here refers to model parameters. [`FormulaEditor::open`] takes the text kept for the
//! field being edited, copies it into a draft and focuses the text area. OK applies the draft;
//! Cancel and Escape discard it. Lifecycle, focus restoration and key routing follow
//! [`super::RenameModal`]; Enter on the text area confirms, as in a single-line dialog.
//!
//! The text fills the two rows of the area as one 62-cell line. The caret `▏`
//! (`README_UI.md` §5.11) is composed in while the text area has focus; the visible window
//! starts at `max(0, c + 1 − 62)`. The draft never contains the caret.

use super::modal::{ModalKey, ModalStatus, step_focus};
use super::{Overlay, OverlayKind, OverlayStack, center_modal, compose_overlays};
use crate::widgets::{Canvas, TextEdit, WidgetContext, caret_window, hover_row, shared};
use crate::{BorderAxis, CellGrid, GridRect, InteractionState, SharedBorder, WidgetId};

/// Frame size, including the border.
const SIZE: (u32, u32) = (35, 8);
/// Cells of one text row, and of the two rows together.
const ROW_CELLS: usize = 31;
const AREA_CELLS: usize = 2 * ROW_CELLS;
/// Longest text the editor holds, in characters. Keeps edits bounded.
const MAX_LEN: usize = 256;
const TITLE: &str = "Formula";

/// Rows of the artwork.
const BOX_TOP_ROW: u32 = 1;
const TEXT_ROW: u32 = 2;
const BOX_BOTTOM_ROW: u32 = 4;
const SEPARATOR_ROW: u32 = 5;
const BUTTON_ROW: u32 = 6;

/// Offsets from the base id.
const SURFACE: u32 = 0;
const TEXT: u32 = 1;
const OK: u32 = 2;
const CANCEL: u32 = 3;

/// What a widget id of this editor refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Surface,
    Text,
    Ok,
    Cancel,
}

fn fits(text: &str) -> bool {
    text.chars().count() <= MAX_LEN
}

/// The FormulaEditor state. Widget ids are `base_id` plus a fixed offset per control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaEditor {
    base_id: u32,
    applied: String,
    edit: TextEdit,
    focus: Option<WidgetId>,
    return_focus: Option<WidgetId>,
    status: ModalStatus,
}

impl FormulaEditor {
    /// A closed, empty editor. `base_id` is the id of the modal surface; the controls use the
    /// ids that follow it.
    #[must_use]
    pub fn new(base_id: u32) -> Self {
        Self {
            base_id,
            applied: String::new(),
            edit: TextEdit::default(),
            focus: None,
            return_focus: None,
            status: ModalStatus::Closed,
        }
    }

    /// Whether the editor is shown and taking input.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status == ModalStatus::Open
    }

    /// Lifecycle state, including the outcome of the last close.
    #[must_use]
    pub const fn status(&self) -> ModalStatus {
        self.status
    }

    /// The text applied by the last OK (or given to the last `open`).
    #[must_use]
    pub fn text(&self) -> &str {
        &self.applied
    }

    /// The text being edited while open.
    #[must_use]
    pub fn draft(&self) -> String {
        self.edit.string()
    }

    /// The id that holds keyboard focus while the editor is open.
    #[must_use]
    pub const fn focus(&self) -> Option<WidgetId> {
        self.focus
    }

    /// Id of the modal surface (the frame, including its border).
    #[must_use]
    pub const fn surface(&self) -> WidgetId {
        WidgetId::new(self.base_id)
    }

    /// Opens the editor on `text` (cut to the length limit, caret at its end) and focuses the
    /// text area. Focus held by `inter` is remembered and restored when the editor closes.
    /// Opening an open editor does not change that memory. If `inter` still holds focus on
    /// this editor (no sync after the last close), the earlier memory is kept.
    pub fn open(&mut self, inter: &mut InteractionState, text: &str) {
        let stale = inter.focused.is_some_and(|f| self.slot(f).is_some());
        if !self.is_open() && !stale {
            self.return_focus = inter.focused;
        }
        self.applied = text.chars().take(MAX_LEN).collect();
        self.edit = TextEdit::new(&self.applied);
        self.status = ModalStatus::Open;
        self.focus = Some(self.id(TEXT));
        inter.set_focused(self.focus);
    }

    /// Applies a keyboard key. Returns `true` if the key was handled. Keys are ignored while
    /// the editor is closed.
    ///
    /// On the text area, characters are inserted, Backspace and Delete remove, the caret moves
    /// and selects with the editing keys; Enter confirms. On the buttons, Enter and Space
    /// activate. Focus moves with Tab/Down and BackTab/Up.
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
            ModalKey::Char(c) if self.on_text() => self.edit.insert(c, fits),
            ModalKey::Backspace if self.on_text() => self.edit.backspace(fits),
            ModalKey::Edit(k) if self.on_text() => self.edit.apply(k, fits),
            ModalKey::Enter | ModalKey::Char(' ') => self.activate_focus(),
            ModalKey::Char(_) | ModalKey::Backspace | ModalKey::Edit(_) => false,
        }
    }

    /// Activates control `id` (pointer or keyboard). Returns `true` if the control belongs to
    /// the editor and was handled. The text area takes focus; OK and Cancel close the editor.
    /// Ids outside the editor, and anything while closed, are refused.
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

    /// Mirrors the editor's focus into `inter`. While open, the focus is the editor's. After a
    /// close, a focus still held by one of this editor's ids is replaced by the focus that was
    /// active when the editor opened.
    pub fn sync_focus(&self, inter: &mut InteractionState) {
        if self.is_open() {
            inter.set_focused(self.focus);
        } else if inter.focused.is_some_and(|f| self.slot(f).is_some()) {
            inter.set_focused(self.return_focus);
        }
    }

    /// Draws the editor into the modal that `stack` holds for it. Returns `true` if the hovered
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
    /// `README_UI.md` §5.12): OK is `│       OK       │` and Cancel `│     Cancel     │`, the
    /// divider between them is its shared slot and the last cell of the row the frame.
    fn hover_buttons(&self, grid: &mut CellGrid, ctx: &WidgetContext<'_>, rect: GridRect) -> bool {
        let row = rect.row().saturating_add(BUTTON_ROW);
        let x = rect.col();
        let target = match ctx.interaction.hovered {
            Some(id) if id == self.id(OK) => 1..17,
            Some(id) if id == self.id(CANCEL) => 18..34,
            _ => return false,
        };
        let (from, span) = (
            x,
            x.saturating_add(target.start)..x.saturating_add(target.end),
        );
        hover_row(grid, row, from, span)
    }

    /// The reference artwork over `rect`, clipped to it. The frame cells are owned by the
    /// surface, the outlined text area by its own id, each button by its id and the `│`
    /// between the buttons is a shared border routed by sub-cell position.
    fn draw_art(&self, grid: &mut CellGrid, ctx: &WidgetContext<'_>, rect: GridRect) {
        let style = ctx.styles.normal;
        let (surface, text) = (Some(self.surface()), Some(self.id(TEXT)));
        let (ok, cancel) = (self.id(OK), self.id(CANCEL));
        let inner = (SIZE.0 - 2) as usize;
        let last = SIZE.0 - 1;
        let mut canvas = Canvas::new(grid, rect);

        let fill = "─".repeat(inner - TITLE.chars().count() - 1);
        canvas.text(0, 0, &format!("╭─{TITLE}{fill}╮"), style, surface);

        let outline = "─".repeat(inner - 2);
        for (row, left, right) in [(BOX_TOP_ROW, '┌', '┐'), (BOX_BOTTOM_ROW, '└', '┘')] {
            canvas.text(0, row, "│", style, surface);
            canvas.text(1, row, &format!("{left}{outline}{right}"), style, text);
            canvas.text(last, row, "│", style, surface);
        }

        let caret = self.on_text().then(|| self.edit.caret());
        let shown: Vec<char> = caret_window(self.edit.chars(), "", caret, AREA_CELLS)
            .chars()
            .collect();
        for (row, line) in (TEXT_ROW..).zip(0..2) {
            let from = (line * ROW_CELLS).min(shown.len());
            let to = (from + ROW_CELLS).min(shown.len());
            let cells: String = shown[from..to].iter().collect();
            canvas.text(0, row, "│", style, surface);
            canvas.text(
                1,
                row,
                &format!("│{cells:<width$}│", width = ROW_CELLS),
                style,
                text,
            );
            canvas.text(last, row, "│", style, surface);
        }

        canvas.text(
            0,
            SEPARATOR_ROW,
            &format!("│{}│", "─".repeat(inner)),
            style,
            surface,
        );

        canvas.text(0, BUTTON_ROW, "│", style, surface);
        canvas.text(1, BUTTON_ROW, "       OK       ", style, Some(ok));
        let border = SharedBorder {
            axis: BorderAxis::Vertical,
            low: Some(ok),
            high: Some(cancel),
        };
        canvas.put(17, BUTTON_ROW, shared('│', style, border));
        canvas.text(18, BUTTON_ROW, "     Cancel     ", style, Some(cancel));
        canvas.text(last, BUTTON_ROW, "│", style, surface);

        canvas.text(
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

    fn on_text(&self) -> bool {
        self.focus == Some(self.id(TEXT))
    }

    fn confirm(&mut self) {
        self.applied = self.edit.string();
        self.status = ModalStatus::Confirmed;
    }

    fn cancel(&mut self) {
        self.edit = TextEdit::new(&self.applied);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::testing::{id, text_row};
    use crate::widgets::{EditKey, WidgetStyles};
    use crate::{Cell, CellContent, StyleId};

    const BASE: u32 = 700;
    /// A control that lies under the modal.
    const BACKGROUND: u32 = 900;

    fn bounds() -> GridRect {
        GridRect::new(0, 0, 80, 24)
    }

    fn text_id() -> WidgetId {
        id(BASE + TEXT)
    }

    fn ok() -> WidgetId {
        id(BASE + OK)
    }

    fn cancel() -> WidgetId {
        id(BASE + CANCEL)
    }

    fn typed(s: &str) -> Vec<ModalKey> {
        s.chars().map(ModalKey::Char).collect()
    }

    /// Opens an editor on `initial` with `BACKGROUND` focused under it, then applies `keys`.
    fn open_with(initial: &str, keys: &[ModalKey]) -> (FormulaEditor, InteractionState) {
        let mut inter = InteractionState::default();
        inter.set_focused(Some(id(BACKGROUND)));
        let mut f = FormulaEditor::new(BASE);
        f.open(&mut inter, initial);
        for &k in keys {
            f.key(k);
        }
        (f, inter)
    }

    fn frame(f: &FormulaEditor, inter: &InteractionState, area: GridRect) -> CellGrid {
        let mut g = CellGrid::new(area.width(), area.height()).unwrap();
        let mut stack = OverlayStack::default();
        f.sync_overlay(&mut stack, area);
        let styles = WidgetStyles::default();
        let ctx = WidgetContext {
            styles: &styles,
            interaction: inter,
        };
        f.draw(&mut g, &ctx, &stack, area);
        g
    }

    /// The modal's 8 rows in an 80x24 frame.
    fn lines(f: &FormulaEditor) -> Vec<String> {
        let g = frame(f, &InteractionState::default(), bounds());
        let at = center_modal(SIZE, bounds());
        (at.row()..at.row() + at.height())
            .map(|r| {
                text_row(&g, r)
                    .chars()
                    .skip(at.col() as usize)
                    .take(at.width() as usize)
                    .map(|c| if c == '.' { ' ' } else { c })
                    .collect()
            })
            .collect()
    }

    fn reference(rows: [&str; 2]) -> Vec<String> {
        vec![
            "╭─Formula─────────────────────────╮".to_owned(),
            "│┌───────────────────────────────┐│".to_owned(),
            format!("││{:<31}││", rows[0]),
            format!("││{:<31}││", rows[1]),
            "│└───────────────────────────────┘│".to_owned(),
            "│─────────────────────────────────│".to_owned(),
            "│       OK       │     Cancel     │".to_owned(),
            "╰─────────────────────────────────╯".to_owned(),
        ]
    }

    #[test]
    fn closed_until_opened_and_open_focuses_the_text_area() {
        let mut f = FormulaEditor::new(BASE);
        assert!(!f.is_open());
        assert_eq!(f.status(), ModalStatus::Closed);
        let mut inter = InteractionState::default();
        inter.set_focused(Some(id(BACKGROUND)));
        f.open(&mut inter, "");
        assert!(f.is_open());
        assert_eq!(f.focus(), Some(text_id()));
        assert_eq!(inter.focused, Some(text_id()));
        assert_eq!(f.draft(), "");
    }

    #[test]
    fn modal_is_empty_centered_and_matches_the_reference() {
        let (mut f, _) = open_with("", &[]);
        // Focus on the text area: only the caret is in the empty area.
        assert_eq!(lines(&f), reference(["▏", ""]));
        f.key(ModalKey::Tab);
        assert_eq!(lines(&f), reference(["", ""]));
        let at = center_modal(SIZE, bounds());
        assert_eq!(
            (at.col(), at.row(), at.width(), at.height()),
            (22, 8, 35, 8)
        );
        let mut stack = OverlayStack::default();
        f.sync_overlay(&mut stack, bounds());
        assert!(f.is_open());
    }

    #[test]
    fn typing_moving_selecting_and_deleting_edit_the_draft_only() {
        let (mut f, _) = open_with("", &typed("a+b"));
        assert_eq!(f.draft(), "a+b");
        assert!(f.text().is_empty(), "not applied yet");
        assert!(f.key(ModalKey::Edit(EditKey::Left)));
        assert!(f.key(ModalKey::Char('*')));
        assert_eq!(f.draft(), "a+*b");
        assert_eq!(lines(&f)[2], format!("││{:<31}││", "a+*▏b"));
        assert!(f.key(ModalKey::Edit(EditKey::Home)));
        assert!(f.key(ModalKey::Edit(EditKey::Delete)));
        assert_eq!(f.draft(), "+*b");
        assert!(f.key(ModalKey::Edit(EditKey::SelectAll)));
        assert!(f.key(ModalKey::Char('x')));
        assert_eq!(f.draft(), "x");
        assert!(f.key(ModalKey::Backspace));
        assert!(!f.key(ModalKey::Backspace), "nothing left to delete");
        assert_eq!(f.draft(), "");
    }

    #[test]
    fn two_rows_then_the_window_scrolls_with_the_caret() {
        let text: String = (0..70).map(|i| char::from(b'a' + (i % 26) as u8)).collect();
        let (mut f, _) = open_with(&text[..40], &[]);
        let rows = lines(&f);
        assert_eq!(rows[2], format!("││{}││", &text[..31]));
        assert_eq!(
            rows[3],
            format!("││{:<31}││", format!("{}▏", &text[31..40]))
        );
        // 61 characters + the caret fill the 62 cells exactly.
        f.key(ModalKey::Edit(EditKey::End));
        for c in text[40..61].chars() {
            f.key(ModalKey::Char(c));
        }
        let rows = lines(&f);
        assert_eq!(rows[2], format!("││{}││", &text[..31]));
        assert_eq!(rows[3], format!("││{}▏││", &text[31..61]));
        // One more character scrolls the first one out of the left boundary.
        f.key(ModalKey::Char(text.chars().nth(61).unwrap()));
        let rows = lines(&f);
        assert_eq!(rows[2], format!("││{}││", &text[1..32]));
        assert_eq!(rows[3], format!("││{}▏││", &text[32..62]));
        // Caret at the start: the window is back at the start.
        f.key(ModalKey::Edit(EditKey::Home));
        assert!(lines(&f)[2].starts_with("││▏"));
        assert!(!f.draft().contains('▏'));
    }

    #[test]
    fn draft_is_capped_and_initial_text_is_cut_to_the_same_limit() {
        let long = "x".repeat(MAX_LEN + 10);
        let (mut f, _) = open_with(&long, &[]);
        assert_eq!(f.text().chars().count(), MAX_LEN);
        assert!(!f.key(ModalKey::Char('y')), "already full");
        assert!(f.key(ModalKey::Backspace));
        assert!(f.key(ModalKey::Char('y')));
        assert_eq!(f.draft().chars().count(), MAX_LEN);
    }

    #[test]
    fn ok_confirms_the_draft_and_restores_prior_focus() {
        let (mut f, mut inter) = open_with("a", &typed("bc"));
        assert!(f.activate(ok()));
        assert_eq!(f.status(), ModalStatus::Confirmed);
        assert_eq!(f.text(), "abc");
        f.sync_focus(&mut inter);
        assert_eq!(inter.focused, Some(id(BACKGROUND)));
        assert!(!f.key(ModalKey::Char('!')), "closed");
    }

    #[test]
    fn enter_on_the_text_area_confirms() {
        let (mut f, _) = open_with("", &typed("1"));
        assert!(f.key(ModalKey::Enter));
        assert_eq!(f.status(), ModalStatus::Confirmed);
        assert_eq!(f.text(), "1");
    }

    #[test]
    fn cancel_and_escape_discard_the_draft() {
        for by_button in [true, false] {
            let (mut f, mut inter) = open_with("keep", &typed(" junk"));
            if by_button {
                assert!(f.activate(cancel()));
            } else {
                assert!(f.key(ModalKey::Escape));
            }
            assert_eq!(f.status(), ModalStatus::Cancelled);
            assert_eq!(f.text(), "keep");
            assert_eq!(f.draft(), "keep");
            f.sync_focus(&mut inter);
            assert_eq!(inter.focused, Some(id(BACKGROUND)));
        }
    }

    #[test]
    fn each_open_starts_from_the_text_it_is_given() {
        let (mut f, mut inter) = open_with("first", &typed("1"));
        f.key(ModalKey::Escape);
        f.sync_focus(&mut inter);
        f.open(&mut inter, "second");
        assert_eq!(f.draft(), "second");
        f.open(&mut inter, "");
        assert_eq!(f.draft(), "");
    }

    #[test]
    fn focus_is_trapped_and_wraps_inside_the_editor() {
        let (mut f, _) = open_with("", &[]);
        assert!(f.key(ModalKey::Tab));
        assert_eq!(f.focus(), Some(ok()));
        assert!(f.key(ModalKey::Tab));
        assert_eq!(f.focus(), Some(cancel()));
        assert!(f.key(ModalKey::Tab));
        assert_eq!(f.focus(), Some(text_id()), "wraps to the text area");
        assert!(f.key(ModalKey::BackTab));
        assert_eq!(f.focus(), Some(cancel()), "backward wraps to Cancel");
        let before = f.clone();
        assert!(!f.activate(id(BACKGROUND)));
        assert_eq!(f, before);
    }

    #[test]
    fn typing_is_ignored_off_the_text_area_and_space_activates_buttons() {
        let (mut f, _) = open_with("", &[ModalKey::Tab]);
        assert_eq!(f.focus(), Some(ok()));
        assert!(!f.key(ModalKey::Char('x')));
        assert!(!f.key(ModalKey::Backspace));
        assert!(!f.key(ModalKey::Edit(EditKey::Home)));
        assert_eq!(f.draft(), "");
        assert!(f.key(ModalKey::Char(' ')), "Space activates OK");
        assert_eq!(f.status(), ModalStatus::Confirmed);
    }

    #[test]
    fn pointer_on_the_text_area_moves_focus_back_to_it() {
        let (mut f, _) = open_with("", &[ModalKey::Tab]);
        assert!(f.activate(text_id()));
        assert_eq!(f.focus(), Some(text_id()));
        assert!(f.is_open());
    }

    #[test]
    fn controls_own_their_cells_and_the_surface_blocks_the_rest() {
        let (f, inter) = open_with("", &[]);
        let g = frame(&f, &inter, bounds());
        let at = center_modal(SIZE, bounds());
        let owner = |dx: u32, dy: u32| g.get(at.col() + dx, at.row() + dy).unwrap().widget_id;
        assert_eq!(owner(2, 2), Some(text_id()));
        assert_eq!(owner(32, 3), Some(text_id()));
        assert_eq!(
            owner(10, 1),
            Some(text_id()),
            "the outline is the text area"
        );
        assert_eq!(owner(0, 2), Some(f.surface()));
        assert_eq!(owner(5, 6), Some(ok()));
        assert_eq!(owner(30, 6), Some(cancel()));
        assert_eq!(owner(17, 6), None, "shared border");
        assert_eq!(f.surface(), id(BASE));
        assert_eq!(ok(), id(BASE + 2));
        assert_eq!(cancel(), id(BASE + 3));
    }

    #[test]
    fn covered_background_is_not_hittable_and_ignores_activation() {
        let mut inter = InteractionState::default();
        let mut f = FormulaEditor::new(BASE);
        f.open(&mut inter, "");
        let mut g = CellGrid::new(80, 24).unwrap();
        let covered = Cell {
            content: CellContent::Glyph('x'),
            style: StyleId::DEFAULT,
            widget_id: Some(id(BACKGROUND)),
            shared_border: None,
        };
        for r in 0..24 {
            for c in 0..80 {
                *g.get_mut(c, r).unwrap() = covered;
            }
        }
        let mut stack = OverlayStack::default();
        f.sync_overlay(&mut stack, bounds());
        let styles = WidgetStyles::default();
        let ctx = WidgetContext {
            styles: &styles,
            interaction: &inter,
        };
        f.draw(&mut g, &ctx, &stack, bounds());
        let outside = g.get(0, 0).unwrap();
        assert_eq!(outside.widget_id, None);
        assert_eq!(outside.content, CellContent::Glyph('x'));
        assert_eq!(g.get(23, 8).unwrap().widget_id, Some(f.surface()));
    }

    #[test]
    fn reopening_without_a_sync_keeps_the_prior_focus_memory() {
        let (mut f, mut inter) = open_with("", &[ModalKey::Escape]);
        f.open(&mut inter, "");
        assert!(f.key(ModalKey::Escape));
        f.sync_focus(&mut inter);
        assert_eq!(inter.focused, Some(id(BACKGROUND)));
    }

    #[test]
    fn keys_and_activation_are_ignored_while_closed() {
        let mut f = FormulaEditor::new(BASE);
        assert!(!f.key(ModalKey::Char('a')));
        assert!(!f.key(ModalKey::Enter));
        assert!(!f.key(ModalKey::Escape));
        assert!(!f.activate(ok()));
        assert_eq!(f.status(), ModalStatus::Closed);
        let g = frame(&f, &InteractionState::default(), bounds());
        assert!(g.cells().iter().all(|c| c.widget_id.is_none()));
    }

    #[test]
    fn tiny_and_degenerate_grids_are_safe() {
        let keys = [
            ModalKey::Tab,
            ModalKey::Char('q'),
            ModalKey::Enter,
            ModalKey::BackTab,
            ModalKey::Char(' '),
            ModalKey::Backspace,
            ModalKey::Edit(EditKey::Left),
            ModalKey::Escape,
        ];
        for (w, h) in [(0, 0), (1, 1), (2, 2), (4, 3), (10, 4), (35, 8)] {
            let mut inter = InteractionState::default();
            let mut f = FormulaEditor::new(BASE);
            f.open(&mut inter, "abc");
            for &k in &keys {
                f.key(k);
                f.sync_focus(&mut inter);
            }
            let g = frame(&f, &inter, GridRect::new(0, 0, w, h));
            assert_eq!((g.cols(), g.rows()), (w, h));
            f.activate(ok());
            let mut stray = OverlayStack::default();
            f.sync_overlay(&mut stray, GridRect::new(100, 100, 30, 30));
        }
    }
}
