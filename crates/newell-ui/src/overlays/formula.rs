// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! FormulaEditor (M2.12): a modal single text area with OK / Cancel.
//!
//! The editor stores and shows the text verbatim. It has no parser and no evaluator, and
//! nothing here refers to model parameters. [`FormulaEditor::open`] copies the applied text
//! into a draft and focuses the text area. OK applies the draft; Cancel and Escape discard it.
//!
//! Composition, focus and input follow [`super::SelectionPanel`]: see its module docs.
//! Enter on the text area confirms, as in a single-line dialog.

use super::modal::{ModalKey, ModalStatus, step_focus};
use super::{
    Overlay, OverlayKind, OverlayStack, center_modal, compose_overlays, draw_ok_cancel,
    modal_layout,
};
use crate::widgets::{Canvas, WidgetContext, owned_blank};
use crate::{CellGrid, GridRect, InteractionState, WidgetId, WidgetState, draw_label};

/// Frame size, including the border. The text area is the body: 38 cells wide, 5 rows.
const SIZE: (u32, u32) = (40, 9);
/// Longest text the editor holds, in characters. Keeps edits bounded.
const MAX_LEN: usize = 256;
/// Caret shown after the text while the text area has focus.
const CARET: char = '_';

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

/// The FormulaEditor state. Widget ids are `base_id` plus a fixed offset per control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaEditor {
    base_id: u32,
    applied: String,
    draft: String,
    focus: Option<WidgetId>,
    return_focus: Option<WidgetId>,
    status: ModalStatus,
}

impl FormulaEditor {
    /// A closed editor holding `initial` (cut to `MAX_LEN` characters). `base_id` is the id
    /// of the modal surface; the controls use the ids that follow it.
    #[must_use]
    pub fn new(base_id: u32, initial: &str) -> Self {
        let applied: String = initial.chars().take(MAX_LEN).collect();
        Self {
            base_id,
            draft: applied.clone(),
            applied,
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

    /// The applied text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.applied
    }

    /// The text being edited while open.
    #[must_use]
    pub fn draft(&self) -> &str {
        &self.draft
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

    /// Opens the editor: the draft restarts from the applied text and focus moves to the
    /// text area. Focus held by `inter` is remembered and restored when the editor closes.
    /// Opening an open editor does not change that memory. If `inter` still holds focus on
    /// this editor (no sync after the last close), the earlier memory is kept.
    pub fn open(&mut self, inter: &mut InteractionState) {
        let stale = inter.focused.is_some_and(|f| self.slot(f).is_some());
        if !self.is_open() && !stale {
            self.return_focus = inter.focused;
        }
        self.draft = self.applied.clone();
        self.status = ModalStatus::Open;
        self.focus = Some(self.id(TEXT));
        inter.set_focused(self.focus);
    }

    /// Applies a keyboard key. Returns `true` if the key was handled. Keys are ignored while
    /// the editor is closed.
    ///
    /// On the text area, characters are inserted and Backspace deletes; Enter confirms. On
    /// the buttons, Enter and Space activate. Focus moves with Tab/Down and BackTab/Up.
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
            ModalKey::Char(_) | ModalKey::Backspace => false,
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

    /// Draws the editor into the modal that `stack` holds for it.
    pub fn draw(
        &self,
        grid: &mut CellGrid,
        ctx: &WidgetContext<'_>,
        stack: &OverlayStack,
        bounds: GridRect,
    ) {
        compose_overlays(grid, ctx, stack, bounds, |g, overlay, inner| {
            if overlay.id == self.surface() {
                self.draw_content(g, ctx, inner);
            }
        });
    }

    fn draw_content(&self, grid: &mut CellGrid, ctx: &WidgetContext<'_>, inner: GridRect) {
        let layout = modal_layout(inner);
        draw_label(grid, layout.title, "Formula", ctx.styles.normal);

        let id = self.id(TEXT);
        let style = ctx.style_of(id, WidgetState::default());
        let mut canvas = Canvas::new(grid, layout.body);
        for dy in 0..canvas.height() {
            canvas.fill(0, canvas.width(), dy, owned_blank(style, id));
        }
        let width = canvas.width() as usize;
        if width > 0 {
            let mut chars: Vec<char> = self.draft.chars().collect();
            if self.on_text() {
                chars.push(CARET);
            }
            for (row, line) in chars.chunks(width).enumerate() {
                let text: String = line.iter().collect();
                let dy = u32::try_from(row).unwrap_or(u32::MAX);
                canvas.text(0, dy, &text, style, Some(id));
            }
        }

        draw_ok_cancel(
            grid,
            ctx,
            layout.actions,
            self.id(OK),
            self.id(CANCEL),
            WidgetState::default(),
        );
    }

    fn move_focus(&mut self, forward: bool) -> bool {
        let ring = self.ring();
        self.focus = step_focus(&ring, self.focus, forward);
        true
    }

    fn activate_focus(&mut self) -> bool {
        match self.focus {
            Some(id) => self.activate(id),
            None => false,
        }
    }

    /// Appends `c` to the draft unless the draft is at `MAX_LEN`.
    fn push(&mut self, c: char) -> bool {
        if self.draft.chars().count() >= MAX_LEN {
            return false;
        }
        self.draft.push(c);
        true
    }

    fn on_text(&self) -> bool {
        self.focus == Some(self.id(TEXT))
    }

    fn confirm(&mut self) {
        self.applied = self.draft.clone();
        self.status = ModalStatus::Confirmed;
    }

    fn cancel(&mut self) {
        self.draft = self.applied.clone();
        self.status = ModalStatus::Cancelled;
    }

    /// Focus order: text area, OK, Cancel.
    fn ring(&self) -> [WidgetId; 3] {
        [self.id(TEXT), self.id(OK), self.id(CANCEL)]
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
    use crate::widgets::WidgetStyles;
    use crate::widgets::testing::{id, text_row};
    use crate::{Cell, CellContent, StyleId};

    const BASE: u32 = 700;
    /// A control that lies under the modal.
    const BACKGROUND: u32 = 900;
    const INITIAL: &str = "Fx = (#EntraxeA - 2mm) / 5 * 3";

    fn bounds() -> GridRect {
        GridRect::new(0, 0, 40, 12)
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

    /// Opens an editor with `BACKGROUND` focused under it, then applies `keys`.
    fn open_with(initial: &str, keys: &[ModalKey]) -> (FormulaEditor, InteractionState) {
        let mut inter = InteractionState::default();
        inter.set_focused(Some(id(BACKGROUND)));
        let mut f = FormulaEditor::new(BASE, initial);
        f.open(&mut inter);
        for &k in keys {
            f.key(k);
        }
        (f, inter)
    }

    /// Draws the editor into a fresh `cols` x `rows` grid, syncing overlay and focus first.
    fn frame(f: &FormulaEditor, inter: &InteractionState, cols: u32, rows: u32) -> CellGrid {
        let bounds = GridRect::new(0, 0, cols, rows);
        let mut g = CellGrid::new(cols, rows).unwrap();
        let mut stack = OverlayStack::default();
        f.sync_overlay(&mut stack, bounds);
        let styles = WidgetStyles::default();
        let ctx = WidgetContext {
            styles: &styles,
            interaction: inter,
        };
        f.draw(&mut g, &ctx, &stack, bounds);
        g
    }

    /// Whether any row shows `needle`; `text_row` draws empty cells as `.`, read here as spaces.
    fn has(g: &CellGrid, needle: &str) -> bool {
        (0..g.rows()).any(|r| text_row(g, r).replace('.', " ").contains(needle))
    }

    /// Body row `row` of the 40x12 frame, without the border column.
    fn body_text(g: &CellGrid, row: u32) -> String {
        text_row(g, row).chars().skip(1).take(38).collect()
    }

    #[test]
    fn closed_until_opened_and_open_focuses_the_text_area() {
        let mut f = FormulaEditor::new(BASE, INITIAL);
        assert!(!f.is_open());
        assert_eq!(f.status(), ModalStatus::Closed);
        assert_eq!(f.text(), INITIAL);
        let mut inter = InteractionState::default();
        inter.set_focused(Some(id(BACKGROUND)));
        f.open(&mut inter);
        assert!(f.is_open());
        assert_eq!(f.focus(), Some(text_id()));
        assert_eq!(inter.focused, Some(text_id()));
        assert_eq!(f.draft(), INITIAL);
    }

    #[test]
    fn typing_and_backspace_edit_the_draft_only() {
        let (mut f, _) = open_with("", &typed("a+b"));
        assert_eq!(f.draft(), "a+b");
        assert!(f.text().is_empty(), "not applied yet");
        assert!(f.key(ModalKey::Char(' ')));
        assert!(f.key(ModalKey::Backspace));
        assert_eq!(f.draft(), "a+b");
        assert!(f.key(ModalKey::Backspace));
        assert_eq!(f.draft(), "a+");
        assert!(f.key(ModalKey::Backspace));
        assert!(f.key(ModalKey::Backspace));
        assert!(!f.key(ModalKey::Backspace), "nothing left to delete");
        assert_eq!(f.draft(), "");
    }

    #[test]
    fn draft_is_capped_and_initial_text_is_cut_to_the_same_limit() {
        let long = "x".repeat(MAX_LEN + 10);
        let (mut f, _) = open_with(&long, &[]);
        assert_eq!(f.text().chars().count(), MAX_LEN);
        assert!(!f.key(ModalKey::Char('y')), "already full");
        assert_eq!(f.draft().chars().count(), MAX_LEN);
        assert!(f.key(ModalKey::Backspace));
        assert!(f.key(ModalKey::Char('y')));
        assert_eq!(f.draft().chars().count(), MAX_LEN);
    }

    #[test]
    fn ok_confirms_the_draft_and_restores_prior_focus() {
        let (mut f, mut inter) = open_with(INITIAL, &typed(" * 2"));
        assert!(f.activate(ok()));
        assert_eq!(f.status(), ModalStatus::Confirmed);
        assert_eq!(f.text(), "Fx = (#EntraxeA - 2mm) / 5 * 3 * 2");
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
    fn cancel_restores_the_prior_text() {
        let (mut f, mut inter) = open_with(INITIAL, &typed(" + junk"));
        assert!(f.activate(cancel()));
        assert_eq!(f.status(), ModalStatus::Cancelled);
        assert_eq!(f.text(), INITIAL);
        assert_eq!(f.draft(), INITIAL);
        f.sync_focus(&mut inter);
        assert_eq!(inter.focused, Some(id(BACKGROUND)));
    }

    #[test]
    fn escape_cancels_and_discards_the_draft() {
        let (mut f, mut inter) = open_with(INITIAL, &typed("zz"));
        assert!(f.key(ModalKey::Escape));
        assert_eq!(f.status(), ModalStatus::Cancelled);
        assert_eq!(f.text(), INITIAL);
        f.sync_focus(&mut inter);
        assert_eq!(inter.focused, Some(id(BACKGROUND)));
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
        // A stale id from under the modal does nothing.
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
        assert_eq!(f.draft(), "");
        assert!(f.key(ModalKey::Char(' ')), "Space activates OK");
        assert_eq!(f.status(), ModalStatus::Confirmed);
        assert_eq!(f.text(), "");
    }

    #[test]
    fn pointer_on_the_text_area_moves_focus_back_to_it() {
        let (mut f, _) = open_with("", &[ModalKey::Tab]);
        assert!(f.activate(text_id()));
        assert_eq!(f.focus(), Some(text_id()));
        assert!(f.is_open());
    }

    #[test]
    fn draw_wraps_the_text_and_shows_the_caret_only_while_focused() {
        // Body is 38 cells wide and 5 rows. 50 characters plus the caret wrap onto two rows.
        // Empty cells read as `.` in `text_row`, so the expectations are prefixes.
        let digits = "0123456789".repeat(5);
        let (f, inter) = open_with(&digits, &[]);
        let g = frame(&f, &inter, 40, 12);
        assert_eq!(body_text(&g, 3), digits[..38]);
        assert!(body_text(&g, 4).starts_with(&format!("{}_", &digits[38..])));
        assert!(has(&g, "Formula"));
        assert!(has(&g, "OK") && has(&g, "Cancel"));
        // Off the text area (focus on OK): no caret.
        let (f2, mut inter2) = open_with(&digits, &[ModalKey::Tab]);
        f2.sync_focus(&mut inter2);
        let g2 = frame(&f2, &inter2, 40, 12);
        assert!(body_text(&g2, 4).starts_with(&digits[38..]));
        assert!(!has(&g2, "_"));
    }

    #[test]
    fn ids_are_stable_and_the_text_area_owns_its_body() {
        let (f, inter) = open_with(INITIAL, &[]);
        let g = frame(&f, &inter, 40, 12);
        // Body row 3, first cell (inside the border column).
        assert_eq!(g.get(1, 3).unwrap().widget_id, Some(text_id()));
        assert_eq!(g.get(38, 3).unwrap().widget_id, Some(text_id()));
        assert_eq!(f.surface(), id(BASE));
        assert_eq!(ok(), id(BASE + 2));
        assert_eq!(cancel(), id(BASE + 3));
        assert!(has(&g, "Fx = (#EntraxeA"));
    }

    #[test]
    fn covered_background_is_not_hittable_and_ignores_activation() {
        let mut inter = InteractionState::default();
        let mut f = FormulaEditor::new(BASE, INITIAL);
        f.open(&mut inter);
        let mut g = CellGrid::new(40, 12).unwrap();
        let covered = Cell {
            content: CellContent::Glyph('x'),
            style: StyleId::DEFAULT,
            widget_id: Some(id(BACKGROUND)),
            shared_border: None,
        };
        for r in 0..12 {
            for c in 0..40 {
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
        assert_eq!(g.get(5, 1).unwrap().widget_id, Some(f.surface()));
    }

    #[test]
    fn reopening_without_a_sync_keeps_the_prior_focus_memory() {
        let (mut f, mut inter) = open_with(INITIAL, &[ModalKey::Escape]);
        f.open(&mut inter);
        assert!(f.key(ModalKey::Escape));
        f.sync_focus(&mut inter);
        assert_eq!(inter.focused, Some(id(BACKGROUND)));
    }

    #[test]
    fn keys_and_activation_are_ignored_while_closed() {
        let mut f = FormulaEditor::new(BASE, INITIAL);
        assert!(!f.key(ModalKey::Char('a')));
        assert!(!f.key(ModalKey::Enter));
        assert!(!f.key(ModalKey::Escape));
        assert!(!f.activate(ok()));
        assert_eq!(f.text(), INITIAL);
        assert_eq!(f.status(), ModalStatus::Closed);
        let g = frame(&f, &InteractionState::default(), 40, 12);
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
            ModalKey::Escape,
        ];
        for (w, h) in [(0, 0), (1, 1), (2, 2), (4, 3), (10, 4), (40, 9)] {
            let mut inter = InteractionState::default();
            let mut f = FormulaEditor::new(BASE, INITIAL);
            f.open(&mut inter);
            for &k in &keys {
                f.key(k);
                f.sync_focus(&mut inter);
            }
            let g = frame(&f, &inter, w, h);
            assert_eq!((g.cols(), g.rows()), (w, h));
            // Off-grid draws and activations are harmless as well.
            f.activate(ok());
            let mut stray = OverlayStack::default();
            f.sync_overlay(&mut stray, GridRect::new(100, 100, 30, 30));
        }
    }
}
