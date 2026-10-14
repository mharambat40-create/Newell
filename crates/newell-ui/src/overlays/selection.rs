// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! SelectionPanel (M2.12): a modal, scrollable list of mock checkbox rows with OK / Cancel.
//!
//! The panel is plain data. [`SelectionPanel::open`] copies the applied selection into a
//! draft and moves focus into the modal. Checkboxes edit only the draft; OK applies it, while
//! Cancel and Escape discard it. The entries are the fixed `MOCK_ITEMS`: nothing here
//! selects real geometry.
//!
//! # Composition
//!
//! The panel is an ordinary M2.8 modal. [`SelectionPanel::sync_overlay`] opens its
//! [`Overlay`] in the caller's [`OverlayStack`], [`SelectionPanel::draw`] fills it through
//! [`compose_overlays`] (which also blocks the covered background), and
//! [`SelectionPanel::sync_focus`] mirrors the modal's focus into [`InteractionState`]. Call
//! `sync_overlay` and `sync_focus` after every input, so that the overlay is dismissed and
//! the prior focus is restored on close.
//!
//! # Focus
//!
//! Focus cycles through the items, then the enabled scroll arrows, then OK and Cancel, and
//! wraps at both ends, so it never leaves the modal. An arrow that the scroll position has
//! disabled is not in the ring. If focus sits on a control that has just become unavailable,
//! it moves to OK.

use super::modal::{ModalKey, ModalStatus, step_focus};
use super::{
    ListItem, ModalLayout, Overlay, OverlayKind, OverlayStack, ScrollList, ScrollState,
    center_modal, compose_overlays, draw_ok_cancel, draw_scroll_list, modal_layout,
};
use crate::widgets::WidgetContext;
use crate::{CellGrid, GridRect, InteractionState, WidgetId, WidgetState, draw_label};

/// Frame size, including the border.
const SIZE: (u32, u32) = (30, 12);
/// Number of mock entries.
const ITEM_COUNT: usize = 9;
/// Mock entries in list order. Fixed text; they name no model element.
const MOCK_ITEMS: [&str; ITEM_COUNT] = [
    "Rim_Body",
    "Hub_Body",
    "Inner_Surface",
    "Flange_Body",
    "Shaft_Body",
    "Cover_Surface",
    "Boss_Body",
    "Rib_Surface",
    "Pin_Body",
];
/// Mock applied selection when the panel is first created: `Hub_Body` is checked, as in the
/// reference.
const INITIAL: [bool; ITEM_COUNT] = [false, true, false, false, false, false, false, false, false];

/// Offsets from the base id. Items follow the four controls.
const SURFACE: u32 = 0;
const UP: u32 = 1;
const DOWN: u32 = 2;
const OK: u32 = 3;
const CANCEL: u32 = 4;
const FIRST_ITEM: u32 = 5;
const ITEMS: u32 = 9;
const SPAN: u32 = FIRST_ITEM + ITEMS;

/// What a widget id of this panel refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Surface,
    Up,
    Down,
    Ok,
    Cancel,
    Item(usize),
}

/// Whole-cell geometry derived from the modal's interior.
#[derive(Debug, Clone, Copy)]
struct Geometry {
    layout: ModalLayout,
    /// Visible rows of the list.
    window: u32,
    /// Whether the scrollbar is drawn (needs a two-column body of at least two rows).
    bar: bool,
}

impl Geometry {
    fn from_interior(inner: GridRect) -> Self {
        let layout = modal_layout(inner);
        let body = layout.body;
        Self {
            layout,
            window: body.height(),
            bar: body.width() >= 2 && body.height() >= 2,
        }
    }

    fn of(bounds: GridRect) -> Self {
        Self::from_interior(center_modal(SIZE, bounds).inset(1))
    }
}

/// The SelectionPanel state. Widget ids are `base_id` plus a fixed offset per control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionPanel {
    base_id: u32,
    applied: [bool; ITEM_COUNT],
    draft: [bool; ITEM_COUNT],
    scroll: ScrollState,
    focus: Option<WidgetId>,
    return_focus: Option<WidgetId>,
    status: ModalStatus,
}

impl SelectionPanel {
    /// A closed panel. `base_id` is the id of the modal surface; the controls use the ids
    /// that follow it (`base_id + 1 ..= base_id + 13`).
    #[must_use]
    pub const fn new(base_id: u32) -> Self {
        Self {
            base_id,
            applied: INITIAL,
            draft: INITIAL,
            scroll: ScrollState { offset: 0 },
            focus: None,
            return_focus: None,
            status: ModalStatus::Closed,
        }
    }

    /// Whether the panel is shown and taking input.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status == ModalStatus::Open
    }

    /// Lifecycle state, including the outcome of the last close.
    #[must_use]
    pub const fn status(&self) -> ModalStatus {
        self.status
    }

    /// The applied selection, one flag per mock entry.
    #[must_use]
    pub fn applied(&self) -> &[bool] {
        &self.applied
    }

    /// The id that holds keyboard focus while the panel is open.
    #[must_use]
    pub const fn focus(&self) -> Option<WidgetId> {
        self.focus
    }

    /// Id of the modal surface (the frame, including its border).
    #[must_use]
    pub const fn surface(&self) -> WidgetId {
        WidgetId::new(self.base_id)
    }

    /// Opens the panel: the draft restarts from the applied selection, the list is scrolled
    /// to the top and focus moves to the first item. Focus held by `inter` is remembered and
    /// restored when the panel closes. Opening an open panel does not change that memory. If
    /// `inter` still holds focus on this panel (no sync after the last close), the earlier
    /// memory is kept.
    pub fn open(&mut self, inter: &mut InteractionState) {
        let stale = inter.focused.is_some_and(|f| self.slot(f).is_some());
        if !self.is_open() && !stale {
            self.return_focus = inter.focused;
        }
        self.draft = self.applied;
        self.scroll = ScrollState::default();
        self.status = ModalStatus::Open;
        self.focus = Some(self.item_id(0));
        inter.set_focused(self.focus);
    }

    /// Applies a keyboard key within `bounds`. Returns `true` if the key was handled. Keys
    /// are ignored while the panel is closed.
    pub fn key(&mut self, key: ModalKey, bounds: GridRect) -> bool {
        if !self.is_open() {
            return false;
        }
        match key {
            ModalKey::Escape => {
                self.cancel();
                true
            }
            ModalKey::Tab | ModalKey::Down => self.move_focus(true, bounds),
            ModalKey::BackTab | ModalKey::Up => self.move_focus(false, bounds),
            ModalKey::Enter | ModalKey::Char(' ') => match self.focus {
                Some(id) => self.activate(id, bounds),
                None => false,
            },
            ModalKey::Char(_) | ModalKey::Backspace => false,
        }
    }

    /// Activates control `id` (pointer or keyboard) within `bounds`. Returns `true` if the
    /// control was active and handled. Ids outside the modal, the surface, disabled arrows and
    /// anything while closed are refused. A handled activation makes `id` the focus.
    pub fn activate(&mut self, id: WidgetId, bounds: GridRect) -> bool {
        if !self.is_open() {
            return false;
        }
        let geometry = Geometry::of(bounds);
        if !self.ring(geometry).contains(&id) {
            return false;
        }
        self.focus = Some(id);
        match self.slot(id) {
            Some(Slot::Item(i)) => {
                if let Some(checked) = self.draft.get_mut(i) {
                    *checked = !*checked;
                }
            }
            Some(Slot::Up) => self.scroll.scroll_by(-1, ITEMS, geometry.window),
            Some(Slot::Down) => self.scroll.scroll_by(1, ITEMS, geometry.window),
            Some(Slot::Ok) => self.confirm(),
            Some(Slot::Cancel) => self.cancel(),
            Some(Slot::Surface) | None => {}
        }
        self.settle(geometry);
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

    /// Mirrors the modal's focus into `inter`. While open, the focus is the modal's. After a
    /// close, a focus still held by one of this panel's ids is replaced by the focus that was
    /// active when the panel opened.
    pub fn sync_focus(&self, inter: &mut InteractionState) {
        if self.is_open() {
            inter.set_focused(self.focus);
        } else if inter.focused.is_some_and(|f| self.slot(f).is_some()) {
            inter.set_focused(self.return_focus);
        }
    }

    /// Draws the panel's content into the modal that `stack` holds for it. Other overlays in
    /// `stack` are left to their own content callbacks; here they only cover the background.
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
        let geometry = Geometry::from_interior(inner);
        draw_label(grid, geometry.layout.title, "Selection", ctx.styles.normal);
        let items: Vec<ListItem<'_>> = MOCK_ITEMS
            .iter()
            .zip(self.draft.iter())
            .enumerate()
            .map(|(i, (&label, &checked))| ListItem {
                id: self.item_id(i),
                label,
                state: WidgetState {
                    checked,
                    ..WidgetState::default()
                },
            })
            .collect();
        let list = ScrollList {
            items: &items,
            scroll: ScrollState {
                offset: self.offset(geometry.window),
            },
            up: self.id(UP),
            down: self.id(DOWN),
        };
        draw_scroll_list(grid, ctx, geometry.layout.body, &list);
        draw_ok_cancel(
            grid,
            ctx,
            geometry.layout.actions,
            self.id(OK),
            self.id(CANCEL),
            WidgetState::default(),
        );
    }

    /// Moves focus one step through the ring and scrolls the list to show an item that
    /// receives it.
    fn move_focus(&mut self, forward: bool, bounds: GridRect) -> bool {
        let geometry = Geometry::of(bounds);
        let ring = self.ring(geometry);
        let before = self.focus;
        self.focus = step_focus(&ring, self.focus, forward);
        if let Some(i) = self.focus.and_then(|id| self.item_index(id)) {
            let index = u32::try_from(i).unwrap_or(u32::MAX);
            self.scroll.ensure_visible(index, ITEMS, geometry.window);
        }
        self.focus != before
    }

    /// Focus order: every item, then ▲ and ▼ when drawn and enabled, then OK and Cancel.
    fn ring(&self, geometry: Geometry) -> Vec<WidgetId> {
        let mut ring: Vec<WidgetId> = (0..ITEM_COUNT).map(|i| self.item_id(i)).collect();
        if geometry.bar {
            let offset = self.offset(geometry.window);
            let max = ScrollState::max_offset(ITEMS, geometry.window);
            if offset > 0 {
                ring.push(self.id(UP));
            }
            if offset < max {
                ring.push(self.id(DOWN));
            }
        }
        ring.push(self.id(OK));
        ring.push(self.id(CANCEL));
        ring
    }

    /// Repairs focus after a change: a focus outside the ring moves to OK.
    fn settle(&mut self, geometry: Geometry) {
        let ring = self.ring(geometry);
        if !self.focus.is_some_and(|f| ring.contains(&f)) {
            self.focus = Some(self.id(OK));
        }
    }

    /// The scroll offset as drawn: clamped to the valid range for `window`.
    fn offset(&self, window: u32) -> u32 {
        self.scroll
            .offset
            .min(ScrollState::max_offset(ITEMS, window))
    }

    fn confirm(&mut self) {
        self.applied = self.draft;
        self.status = ModalStatus::Confirmed;
    }

    fn cancel(&mut self) {
        self.draft = self.applied;
        self.status = ModalStatus::Cancelled;
    }

    fn id(&self, offset: u32) -> WidgetId {
        WidgetId::new(self.base_id.wrapping_add(offset))
    }

    fn item_id(&self, index: usize) -> WidgetId {
        let offset = u32::try_from(index).unwrap_or(u32::MAX);
        self.id(FIRST_ITEM.saturating_add(offset))
    }

    fn item_index(&self, id: WidgetId) -> Option<usize> {
        match self.slot(id)? {
            Slot::Item(i) => Some(i),
            _ => None,
        }
    }

    fn slot(&self, id: WidgetId) -> Option<Slot> {
        let rel = id.get().wrapping_sub(self.base_id);
        Some(match rel {
            SURFACE => Slot::Surface,
            UP => Slot::Up,
            DOWN => Slot::Down,
            OK => Slot::Ok,
            CANCEL => Slot::Cancel,
            r if (FIRST_ITEM..SPAN).contains(&r) => {
                Slot::Item(usize::try_from(r - FIRST_ITEM).unwrap_or(usize::MAX))
            }
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::WidgetStyles;
    use crate::widgets::testing::{id, text_row};
    use crate::{Cell, CellContent, StyleId};

    const BASE: u32 = 500;
    /// A control that lies under the modal.
    const BACKGROUND: u32 = 900;

    fn panel() -> SelectionPanel {
        SelectionPanel::new(BASE)
    }

    /// Frame of 30x12 centred in a 40x14 grid: interior 28x10 at (6, 2); the list has 8
    /// visible rows (3..=10) for 9 items.
    fn wide() -> GridRect {
        GridRect::new(0, 0, 40, 14)
    }

    fn item(i: u32) -> WidgetId {
        id(BASE + FIRST_ITEM + i)
    }

    fn ok() -> WidgetId {
        id(BASE + OK)
    }

    fn cancel() -> WidgetId {
        id(BASE + CANCEL)
    }

    fn up() -> WidgetId {
        id(BASE + UP)
    }

    fn down() -> WidgetId {
        id(BASE + DOWN)
    }

    /// Draws the panel into a fresh `cols` x `rows` grid, syncing overlay and focus first.
    fn frame(p: &SelectionPanel, inter: &InteractionState, cols: u32, rows: u32) -> CellGrid {
        let bounds = GridRect::new(0, 0, cols, rows);
        let mut g = CellGrid::new(cols, rows).unwrap();
        let mut stack = OverlayStack::default();
        p.sync_overlay(&mut stack, bounds);
        let styles = WidgetStyles::default();
        let ctx = WidgetContext {
            styles: &styles,
            interaction: inter,
        };
        p.draw(&mut g, &ctx, &stack, bounds);
        g
    }

    /// Whether any row shows `needle`; `text_row` draws empty cells as `.`, read here as spaces.
    fn has(g: &CellGrid, needle: &str) -> bool {
        (0..g.rows()).any(|r| text_row(g, r).replace('.', " ").contains(needle))
    }

    /// A grid whose every cell is an unrelated control with a glyph, to be covered.
    fn background(cols: u32, rows: u32) -> CellGrid {
        let mut g = CellGrid::new(cols, rows).unwrap();
        let covered = Cell {
            content: CellContent::Glyph('x'),
            style: StyleId::DEFAULT,
            widget_id: Some(id(BACKGROUND)),
            shared_border: None,
        };
        for r in 0..rows {
            for col in 0..cols {
                if let Some(slot) = g.get_mut(col, r) {
                    *slot = covered;
                }
            }
        }
        g
    }

    /// Opens the panel with the given focus under it, then applies `keys` in order.
    fn open_with(keys: &[ModalKey]) -> (SelectionPanel, InteractionState) {
        let mut inter = InteractionState::default();
        inter.set_focused(Some(id(BACKGROUND)));
        let mut p = panel();
        p.open(&mut inter);
        for &k in keys {
            p.key(k, wide());
        }
        (p, inter)
    }

    #[test]
    fn closed_until_opened_and_open_focuses_first_item() {
        let mut p = panel();
        assert!(!p.is_open());
        assert_eq!(p.status(), ModalStatus::Closed);
        let mut inter = InteractionState::default();
        inter.set_focused(Some(id(BACKGROUND)));
        p.open(&mut inter);
        assert!(p.is_open());
        assert_eq!(p.focus(), Some(item(0)));
        assert_eq!(inter.focused, Some(item(0)));
        // The applied selection starts with Hub_Body checked, as in the reference.
        assert!(p.applied()[1]);
        assert!(!p.applied()[0]);
    }

    #[test]
    fn draw_shows_title_checkboxes_scroll_indicator_and_actions() {
        let mut inter = InteractionState::default();
        let mut p = panel();
        p.open(&mut inter);
        let g = frame(&p, &inter, 40, 14);
        assert!(has(&g, "Selection"));
        assert!(has(&g, "□ Rim_Body"));
        assert!(has(&g, "■ Hub_Body"));
        assert!(has(&g, "▲") && has(&g, "▼"));
        assert!(has(&g, "OK") && has(&g, "Cancel"));
        assert!(!has(&g, "Pin_Body"), "beyond the window");
    }

    #[test]
    fn focus_cycles_inside_the_modal_and_wraps() {
        let (mut p, mut inter) = open_with(&[]);
        let bounds = wide();
        // Items 0..=8, then ▼ (offset 0 of 1), then OK and Cancel: 12 stops.
        let mut seen = Vec::new();
        for _ in 0..12 {
            assert!(p.key(ModalKey::Tab, bounds));
            seen.push(p.focus().unwrap());
        }
        assert!(
            seen.iter()
                .all(|f| (BASE + 1..BASE + SPAN).contains(&f.get()))
        );
        assert!(!seen.contains(&id(BACKGROUND)));
        assert_eq!(p.focus(), Some(item(0)), "wraps to the first item");
        assert!(p.key(ModalKey::BackTab, bounds));
        assert_eq!(p.focus(), Some(cancel()), "backward from the first wraps");
        p.sync_focus(&mut inter);
        assert_eq!(inter.focused, Some(cancel()));
    }

    #[test]
    fn escape_cancels_restores_selection_and_prior_focus() {
        let (mut p, mut inter) = open_with(&[ModalKey::Enter]);
        assert!(!p.applied()[0], "draft only");
        assert!(p.key(ModalKey::Escape, wide()));
        assert_eq!(p.status(), ModalStatus::Cancelled);
        assert!(!p.is_open());
        assert!(!p.applied()[0]);
        p.sync_focus(&mut inter);
        assert_eq!(inter.focused, Some(id(BACKGROUND)));
        // Keys do nothing once closed.
        assert!(!p.key(ModalKey::Tab, wide()));
    }

    #[test]
    fn ok_applies_the_draft_and_restores_prior_focus() {
        let (mut p, mut inter) = open_with(&[ModalKey::Char(' ')]);
        assert!(p.activate(ok(), wide()));
        assert_eq!(p.status(), ModalStatus::Confirmed);
        assert!(p.applied()[0]);
        assert!(p.applied()[1]);
        p.sync_focus(&mut inter);
        assert_eq!(inter.focused, Some(id(BACKGROUND)));
    }

    #[test]
    fn cancel_button_discards_the_draft() {
        let (mut p, _) = open_with(&[ModalKey::Char(' ')]);
        assert!(p.activate(cancel(), wide()));
        assert_eq!(p.status(), ModalStatus::Cancelled);
        assert!(!p.applied()[0]);
        // Reopening starts again from the applied selection.
        let mut inter = InteractionState::default();
        p.open(&mut inter);
        let g = frame(&p, &inter, 40, 14);
        assert!(has(&g, "□ Rim_Body"));
    }

    #[test]
    fn item_toggles_by_enter_space_and_pointer() {
        let (mut p, _) = open_with(&[ModalKey::Enter, ModalKey::Char(' ')]);
        // Enter and Space both toggle the focused item: back to unchecked.
        assert_eq!(p.focus(), Some(item(0)));
        assert!(p.activate(item(1), wide()), "pointer on Hub_Body");
        assert_eq!(p.focus(), Some(item(1)));
        assert!(p.activate(ok(), wide()));
        assert_eq!(
            p.applied(),
            &[
                false, false, false, false, false, false, false, false, false
            ]
        );
    }

    #[test]
    fn scroll_arrows_clamp_and_disable_at_the_ends() {
        let (mut p, _) = open_with(&[]);
        let bounds = wide();
        // Top: ▲ is disabled and refused; ▼ scrolls.
        assert!(!p.activate(up(), bounds));
        assert!(p.activate(down(), bounds));
        let mut inter = InteractionState::default();
        let g = frame(&p, &inter, 40, 14);
        assert!(has(&g, "Pin_Body") && !has(&g, "Rim_Body"));
        // Bottom: ▼ is now disabled, so focus left it for OK.
        assert!(!p.activate(down(), bounds));
        assert_eq!(p.focus(), Some(ok()));
        p.sync_focus(&mut inter);
        assert!(p.activate(up(), bounds));
        let g = frame(&p, &inter, 40, 14);
        assert!(has(&g, "Rim_Body") && !has(&g, "Pin_Body"));
        assert!(!p.activate(up(), bounds), "back at the top");
    }

    #[test]
    fn focus_movement_scrolls_to_keep_focus_visible() {
        let (mut p, mut inter) = open_with(&[]);
        for _ in 0..8 {
            p.key(ModalKey::Tab, wide());
        }
        assert_eq!(p.focus(), Some(item(8)));
        p.sync_focus(&mut inter);
        let g = frame(&p, &inter, 40, 14);
        assert!(has(&g, "Pin_Body") && !has(&g, "Rim_Body"));
        // Backward from the last visible item keeps it visible too.
        p.key(ModalKey::Up, wide());
        assert_eq!(p.focus(), Some(item(7)));
        let g = frame(&p, &inter, 40, 14);
        assert!(has(&g, "Rib_Surface"));
    }

    #[test]
    fn covered_background_is_not_hittable_and_ignores_activation() {
        let bounds = wide();
        let mut inter = InteractionState::default();
        let mut p = panel();
        p.open(&mut inter);
        let mut g = background(40, 14);
        let mut stack = OverlayStack::default();
        p.sync_overlay(&mut stack, bounds);
        let styles = WidgetStyles::default();
        let ctx = WidgetContext {
            styles: &styles,
            interaction: &inter,
        };
        p.draw(&mut g, &ctx, &stack, bounds);
        // Outside the frame: the background stays visible but owns no pointer target.
        let outside = g.get(0, 0).unwrap();
        assert_eq!(outside.widget_id, None);
        assert_eq!(outside.content, CellContent::Glyph('x'));
        // The frame owns its own cells.
        assert_eq!(g.get(5, 1).unwrap().widget_id, Some(p.surface()));
        // A stale id from under the modal does nothing.
        let before = p.clone();
        assert!(!p.activate(id(BACKGROUND), bounds));
        assert_eq!(p, before);
    }

    #[test]
    fn ids_are_stable_across_scroll_and_focus() {
        let (mut p, mut inter) = open_with(&[]);
        let ids = |g: &CellGrid| -> Vec<u32> {
            let mut v: Vec<u32> = g
                .cells()
                .iter()
                .filter_map(|c| c.widget_id.map(WidgetId::get))
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        let first = frame(&p, &inter, 40, 14);
        assert!(p.activate(down(), wide()));
        p.sync_focus(&mut inter);
        let second = frame(&p, &inter, 40, 14);
        for g in [&first, &second] {
            let owned = ids(g);
            assert!(owned.contains(&(BASE + OK)) && owned.contains(&(BASE + CANCEL)));
            assert!(owned.iter().all(|v| (BASE..BASE + SPAN).contains(v)));
        }
        assert_eq!(p.surface(), id(BASE));
        assert_eq!(item(0), id(BASE + 5));
        assert_eq!(ok(), id(BASE + 3));
    }

    #[test]
    fn reopening_without_a_sync_does_not_restore_to_its_own_control() {
        let (mut p, mut inter) = open_with(&[ModalKey::Escape]);
        // Close, then reopen while the stale focus (one of our items) is still in `inter`.
        assert!(!p.is_open());
        p.open(&mut inter);
        assert!(p.key(ModalKey::Escape, wide()));
        p.sync_focus(&mut inter);
        assert_eq!(inter.focused, Some(id(BACKGROUND)));
    }

    #[test]
    fn keys_and_activation_are_ignored_while_closed() {
        let mut p = panel();
        for k in [
            ModalKey::Tab,
            ModalKey::Enter,
            ModalKey::Escape,
            ModalKey::Char(' '),
        ] {
            assert!(!p.key(k, wide()));
        }
        assert!(!p.activate(ok(), wide()));
        assert_eq!(p.status(), ModalStatus::Closed);
        let inter = InteractionState::default();
        let g = frame(&p, &inter, 40, 14);
        assert!(g.cells().iter().all(|c| c.widget_id.is_none()));
    }

    #[test]
    fn tiny_and_degenerate_grids_are_safe() {
        let keys = [
            ModalKey::Tab,
            ModalKey::Down,
            ModalKey::Enter,
            ModalKey::Up,
            ModalKey::Char(' '),
            ModalKey::BackTab,
            ModalKey::Escape,
        ];
        for (w, h) in [(0, 0), (1, 1), (2, 2), (4, 3), (10, 4), (30, 12)] {
            let bounds = GridRect::new(0, 0, w, h);
            let mut inter = InteractionState::default();
            let mut p = panel();
            p.open(&mut inter);
            for &k in &keys {
                p.key(k, bounds);
                p.sync_focus(&mut inter);
            }
            let g = frame(&p, &inter, w, h);
            assert_eq!((g.cols(), g.rows()), (w, h));
            // An activation on an off-grid rect is also harmless.
            p.activate(ok(), GridRect::new(100, 100, 30, 30));
        }
    }
}
