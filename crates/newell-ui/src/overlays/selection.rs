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

use super::list::{SelectionListGeometry, SelectionThumb, draw_selection_scrollbar, round_half_up};
use super::modal::{ModalKey, ModalStatus, step_focus};
use super::{
    ListItem, ModalLayout, Overlay, OverlayKind, OverlayStack, ScrollList, ScrollState,
    center_modal, compose_overlays, draw_ok_cancel, draw_scroll_list, modal_layout,
};
use crate::widgets::{Canvas, WidgetContext, cell, hover_row, owned_blank};
use crate::{
    BorderLine, CellContent, CellGrid, GridRect, InteractionState, WidgetId, WidgetState,
    draw_border, draw_label,
};

/// Frame size, including the border.
const SIZE: (u32, u32) = (30, 12);
/// Profile selection replaces its originating Action Box with this exact reference footprint.
const PROFILE_SIZE: (u32, u32) = (31, 11);
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
/// Selected profiles shown by the Profile picker inside an Action Box.
const PROFILE_ITEMS: [&str; ITEM_COUNT] = [
    "Spline_1",
    "Profil_3",
    "Profil_4",
    "Profil_4",
    "Compound_line_1",
    "Sketch_2",
    "Arc_7",
    "Guide_curve_1",
    "Spline_2",
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
const THUMB: u32 = 14;
const FIRST_ITEM: u32 = 5;
const SPAN: u32 = THUMB + 1;

/// What a widget id of this panel refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Surface,
    Up,
    Down,
    Ok,
    Cancel,
    Item(usize),
    Thumb,
}

/// The two documented presentations of the shared mock selection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionKind {
    Generic,
    Profiles,
}

impl SelectionKind {
    const fn title(self) -> &'static str {
        match self {
            Self::Generic => "Selection",
            Self::Profiles => "Profils",
        }
    }
}

/// Whole-cell geometry derived from the modal's interior.
#[derive(Debug, Clone, Copy)]
struct Geometry {
    layout: ModalLayout,
    list: SelectionListGeometry,
    len: u32,
    /// Visible rows of the list.
    window: u32,
    /// Whether a logical scrollbar gutter exists.
    bar: bool,
}

impl Geometry {
    fn from_layout(layout: ModalLayout, len: u32) -> Self {
        let list = SelectionListGeometry::new(layout.body, len);
        Self {
            layout,
            list,
            len,
            window: list.window,
            bar: list.bar.is_some(),
        }
    }

    fn from_interior(inner: GridRect, len: u32) -> Self {
        Self::from_layout(modal_layout(inner), len)
    }

    fn of(bounds: GridRect, len: u32) -> Self {
        Self::from_interior(center_modal(SIZE, bounds).inset(1), len)
    }

    fn from_action_box(frame: GridRect, len: u32) -> Self {
        let inner = frame.inset(1);
        let (actions, above_actions) = inner.split_bottom(1);
        let (_separator, list_frame) = above_actions.split_bottom(1);
        Self::from_layout(
            ModalLayout {
                title: GridRect::new(inner.col(), inner.row(), inner.width(), 0),
                body: list_frame.inset(1),
                actions,
            },
            len,
        )
    }
}

/// A selection-thumb drag anchor. It preserves the exact content offset and
/// whole-row grab relationship, and is invalidated by external layout changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionThumbGrab {
    surface: WidgetId,
    epoch: u64,
    row: u32,
    offset: u32,
    position: u32,
    len: u32,
    geometry: SelectionListGeometry,
    kind: SelectionKind,
}

impl SelectionThumbGrab {
    fn new(row: u32, offset: u32, geometry: Geometry, kind: SelectionKind) -> Option<Self> {
        let thumb = SelectionThumb::new(geometry.len, offset)?;
        let bar = geometry.list.bar?;
        let track_row = row.checked_sub(bar.row().checked_add(1)?)?;
        if row as u64 >= geometry.list.allocation.bottom()
            || thumb
                .glyphs()
                .get(track_row as usize)
                .is_none_or(|&g| g == ' ')
        {
            return None;
        }
        Some(Self {
            surface: WidgetId::new(0),
            epoch: 0,
            row,
            offset: thumb.offset,
            position: thumb.position,
            len: geometry.len,
            geometry: geometry.list,
            kind,
        })
    }

    fn valid_for(self, geometry: Geometry, kind: SelectionKind) -> bool {
        self.kind == kind
            && self.len == geometry.len
            && self.geometry == geometry.list
            && geometry.len > 5
            && geometry.bar
    }

    fn offset_at(self, row: u32) -> u32 {
        if row == self.row {
            return self.offset;
        }
        let length = 11_u32.saturating_sub(self.len).max(1);
        let travel = 6 - length;
        let wanted = i64::from(self.position) + 2 * (i64::from(row) - i64::from(self.row));
        let top = wanted.clamp(0, i64::from(travel)) as u64;
        round_half_up(u64::from(self.len - 5) * top, u64::from(travel)) as u32
    }
}

/// The SelectionPanel state. Widget ids are `base_id` plus a fixed offset per control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionPanel {
    base_id: u32,
    applied: [bool; ITEM_COUNT],
    profile_applied: [bool; ITEM_COUNT],
    draft: [bool; ITEM_COUNT],
    scroll: ScrollState,
    grab_epoch: Option<u64>,
    focus: Option<WidgetId>,
    return_focus: Option<WidgetId>,
    kind: SelectionKind,
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
            profile_applied: INITIAL,
            draft: INITIAL,
            scroll: ScrollState { offset: 0 },
            grab_epoch: Some(0),
            focus: None,
            return_focus: None,
            kind: SelectionKind::Generic,
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

    /// The Profile Selection box's applied mock state, independent of generic checkboxes.
    #[must_use]
    pub fn profile_applied(&self) -> &[bool] {
        &self.profile_applied
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
        self.open_kind(inter, SelectionKind::Generic);
    }

    /// Opens the Profile Selection box in the originating Action Box footprint.
    pub fn open_profiles(&mut self, inter: &mut InteractionState) {
        self.open_kind(inter, SelectionKind::Profiles);
    }

    /// `true` while this selection state is the Action Box Profile Selection box rather than
    /// the centered, checkbox-based generic Selection modal.
    #[must_use]
    pub fn is_profile_picker(&self) -> bool {
        self.is_open() && self.kind == SelectionKind::Profiles
    }

    /// Number of rows in the exact Profile Selection box, including its outer border.
    #[must_use]
    pub const fn profile_height(&self) -> usize {
        PROFILE_SIZE.1 as usize
    }

    fn open_kind(&mut self, inter: &mut InteractionState, kind: SelectionKind) {
        let stale = inter.focused.is_some_and(|f| self.slot(f).is_some());
        if !self.is_open() && !stale {
            self.return_focus = inter.focused;
        }
        self.draft = match kind {
            SelectionKind::Generic => self.applied,
            SelectionKind::Profiles => self.profile_applied,
        };
        self.kind = kind;
        // Exhaustion disables new grabs rather than letting an old anchor become valid.
        self.grab_epoch = self.grab_epoch.and_then(|epoch| epoch.checked_add(1));
        self.scroll = ScrollState::default();
        self.status = ModalStatus::Open;
        self.focus = Some(self.item_id(0));
        inter.set_focused(self.focus);
    }

    /// Applies a keyboard key within `bounds`. Returns `true` if the key was handled. Keys
    /// are ignored while the panel is closed.
    pub fn key(&mut self, key: ModalKey, bounds: GridRect) -> bool {
        self.key_in(key, Geometry::of(bounds, self.len()))
    }

    /// Applies a key while the panel occupies an Action Box.  The caller supplies the whole
    /// Action Box frame, including its border.
    pub fn key_in_action_box(&mut self, key: ModalKey, frame: GridRect) -> bool {
        self.key_in(key, Geometry::from_action_box(frame, self.len()))
    }

    fn key_in(&mut self, key: ModalKey, geometry: Geometry) -> bool {
        if !self.is_open() {
            return false;
        }
        self.settle(geometry);
        match key {
            ModalKey::Escape => {
                self.cancel();
                true
            }
            ModalKey::Tab | ModalKey::Down => self.move_focus(true, geometry),
            ModalKey::BackTab | ModalKey::Up => self.move_focus(false, geometry),
            ModalKey::Enter | ModalKey::Char(' ') => match self.focus {
                Some(id) => self.activate_in(id, geometry),
                None => false,
            },
            ModalKey::Char(_) | ModalKey::Backspace | ModalKey::Edit(_) => false,
        }
    }

    /// Activates control `id` (pointer or keyboard) within `bounds`. Returns `true` if the
    /// control was active and handled. Ids outside the modal, the surface, disabled arrows and
    /// anything while closed are refused. A handled activation makes `id` the focus.
    pub fn activate(&mut self, id: WidgetId, bounds: GridRect) -> bool {
        self.activate_in(id, Geometry::of(bounds, self.len()))
    }

    /// Activates a control while the panel is drawn inside `frame`, the Action Box frame.
    pub fn activate_in_action_box(&mut self, id: WidgetId, frame: GridRect) -> bool {
        self.activate_in(id, Geometry::from_action_box(frame, self.len()))
    }

    /// Scrolls the centered selection list by whole content rows.
    pub fn scroll_by(&mut self, delta: i32, bounds: GridRect) -> bool {
        self.scroll_delta(i64::from(delta), Geometry::of(bounds, self.len()))
    }

    /// Scrolls the profile picker by whole content rows.
    pub fn scroll_in_action_box(&mut self, delta: i32, frame: GridRect) -> bool {
        self.scroll_delta(
            i64::from(delta),
            Geometry::from_action_box(frame, self.len()),
        )
    }

    pub(crate) fn scroll_wheel(
        &mut self,
        steps: i32,
        bounds: GridRect,
        profile_frame: GridRect,
    ) -> bool {
        self.scroll_delta(-i64::from(steps), self.geometry(bounds, profile_frame))
    }

    fn scroll_delta(&mut self, delta: i64, geometry: Geometry) -> bool {
        if !self.is_open() {
            return false;
        }
        let before = self.scroll.offset;
        self.scroll
            .scroll_by_wide(delta, geometry.len, geometry.window);
        self.settle(geometry);
        self.scroll.offset != before
    }

    fn labels(&self) -> &[&str] {
        match self.kind {
            SelectionKind::Generic => &MOCK_ITEMS,
            SelectionKind::Profiles => &PROFILE_ITEMS,
        }
    }

    fn len(&self) -> u32 {
        u32::try_from(self.labels().len()).unwrap_or(u32::MAX)
    }

    pub(crate) fn reconcile(&mut self, bounds: GridRect, profile_frame: GridRect) {
        if self.is_open() {
            self.settle(self.geometry(bounds, profile_frame));
        }
    }

    fn activate_in(&mut self, id: WidgetId, geometry: Geometry) -> bool {
        if !self.is_open() {
            return false;
        }
        self.settle(geometry);
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
            Some(Slot::Up) => self.scroll.scroll_by(-1, geometry.len, geometry.window),
            Some(Slot::Down) => self.scroll.scroll_by(1, geometry.len, geometry.window),
            Some(Slot::Ok) => self.confirm(),
            Some(Slot::Cancel) => self.cancel(),
            Some(Slot::Surface | Slot::Thumb) | None => {}
        }
        self.settle(geometry);
        true
    }

    /// Opens the centered generic modal in `stack` while it is open, or dismisses it.
    pub fn sync_overlay(&self, stack: &mut OverlayStack, bounds: GridRect) {
        self.sync_overlay_with_profile_frame(stack, bounds, bounds);
    }

    /// Places the Profile Selection modal over its Action Box frame, retaining the stack's
    /// modal input blocking and occlusion.
    pub(crate) fn sync_overlay_with_profile_frame(
        &self,
        stack: &mut OverlayStack,
        bounds: GridRect,
        profile_frame: GridRect,
    ) {
        if self.is_open() {
            stack.open(Overlay {
                id: self.surface(),
                kind: OverlayKind::Modal,
                rect: match self.kind {
                    SelectionKind::Generic => center_modal(SIZE, bounds),
                    SelectionKind::Profiles => profile_frame,
                },
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
                match self.kind {
                    SelectionKind::Generic => self.draw_content(g, ctx, inner),
                    SelectionKind::Profiles => {
                        self.draw_in_action_box(g, ctx, overlay.rect.intersection(&bounds));
                    }
                }
            }
        });
    }

    /// Draws the selection directly in an Action Box footprint.  This deliberately does not
    /// use an overlay: the list replaces the originating Action Box, so the viewport stays
    /// opaque only under that footprint and the standard Action Box footer remains in place.
    pub fn draw_in_action_box(
        &self,
        grid: &mut CellGrid,
        ctx: &WidgetContext<'_>,
        frame: GridRect,
    ) -> bool {
        let frame = frame.intersection(&GridRect::from_grid(grid));
        if frame.width() == 0 || frame.height() == 0 {
            return false;
        }
        for row in frame.row()..frame.row().saturating_add(frame.height()) {
            for col in frame.col()..frame.col().saturating_add(frame.width()) {
                if let Some(slot) = grid.get_mut(col, row) {
                    *slot = cell(CellContent::Empty, ctx.styles.normal, Some(self.surface()));
                }
            }
        }
        draw_border(grid, frame, BorderLine::Single, ctx.styles.normal);
        for row in frame.row()..frame.row().saturating_add(frame.height()) {
            for col in frame.col()..frame.col().saturating_add(frame.width()) {
                if let Some(slot) = grid.get_mut(col, row)
                    && slot.widget_id.is_none()
                {
                    slot.widget_id = Some(self.surface());
                }
            }
        }
        let inner_width = frame.width().saturating_sub(2);
        let title: String = self
            .kind
            .title()
            .chars()
            .take(inner_width.saturating_sub(1) as usize)
            .collect();
        let header_fill =
            "─".repeat(inner_width.saturating_sub(title.chars().count() as u32 + 1) as usize);
        let mut canvas = Canvas::new(grid, frame);
        let end = canvas.contour(0, 0, "╭─", ctx.styles.normal, Some(self.surface()));
        let end = canvas.text(end, 0, &title, ctx.styles.normal, Some(self.surface()));
        canvas.contour(
            end,
            0,
            &format!("{header_fill}╮"),
            ctx.styles.normal,
            Some(self.surface()),
        );
        canvas.contour(
            0,
            frame.height().saturating_sub(1),
            &format!("╰{}╯", "─".repeat(inner_width as usize)),
            ctx.styles.normal,
            Some(self.surface()),
        );
        self.draw_action_content(grid, ctx, frame)
    }

    fn draw_action_content(
        &self,
        grid: &mut CellGrid,
        ctx: &WidgetContext<'_>,
        frame: GridRect,
    ) -> bool {
        let inner = frame.inset(1);
        let (actions, above_actions) = inner.split_bottom(1);
        let (separator, list_frame) = above_actions.split_bottom(1);
        self.draw_selected_items(grid, ctx, list_frame);
        Canvas::new(grid, separator).contour(
            0,
            0,
            &"─".repeat(inner.width() as usize),
            ctx.styles.normal,
            Some(self.surface()),
        );
        draw_ok_cancel(
            grid,
            ctx,
            actions,
            self.id(OK),
            self.id(CANCEL),
            WidgetState::default(),
        );
        self.hover_action_buttons(grid, ctx, frame, actions)
    }

    /// Draws the selected-input list nested inside the picker. The surrounding frame and the
    /// separator deliberately mirror the Formula editor and Action Box grammar.
    fn draw_selected_items(&self, grid: &mut CellGrid, ctx: &WidgetContext<'_>, frame: GridRect) {
        super::for_each_visible(grid, frame, |cell| *cell = crate::Cell::default());
        if frame.width() < 3 || frame.height() < 3 {
            return;
        }
        draw_border(grid, frame, BorderLine::Single, ctx.styles.normal);
        let body = frame.inset(1);
        let rows: Vec<_> = self
            .labels()
            .iter()
            .enumerate()
            .map(|(index, &label)| (self.item_id(index), label))
            .collect();
        self.draw_profile_rows(grid, ctx, body, &rows);
    }

    /// Borrowed IDs keep larger rendering fixtures independent of the fixed panel slots.
    fn draw_profile_rows(
        &self,
        grid: &mut CellGrid,
        ctx: &WidgetContext<'_>,
        body: GridRect,
        rows: &[(WidgetId, &str)],
    ) {
        super::for_each_visible(grid, body, |cell| *cell = crate::Cell::default());
        let len = u32::try_from(rows.len()).unwrap_or(u32::MAX);
        let geometry = SelectionListGeometry::new(body, len);
        let offset = self.scroll.offset.min(geometry.max);
        let content = geometry.content;
        let label_width = content.width().saturating_sub(5) as usize;
        for (visible, &(id, label)) in rows
            .iter()
            .skip(offset as usize)
            .take(geometry.window as usize)
            .enumerate()
        {
            let mut canvas = Canvas::new(grid, geometry.row(visible as u32));
            canvas.fill(0, canvas.width(), 0, owned_blank(ctx.styles.normal, id));
            let label: String = label.chars().take(label_width).collect();
            canvas.text_with_ink(
                1,
                0,
                "☰",
                ctx.styles.normal,
                Some(id),
                crate::InkRole::Action,
            );
            canvas.text(3, 0, &label, ctx.styles.normal, Some(id));
            if content.width() >= 2 {
                canvas.text_with_ink(
                    content.width() - 2,
                    0,
                    "×",
                    ctx.styles.normal,
                    Some(id),
                    crate::InkRole::Action,
                );
            }
        }
        if let Some(thumb) = SelectionThumb::new(len, offset) {
            draw_selection_scrollbar(
                grid,
                ctx,
                geometry,
                thumb,
                [self.id(UP), self.id(DOWN), self.id(THUMB)],
            );
        }
    }

    pub(crate) fn thumb_grab(
        &self,
        id: WidgetId,
        row: u32,
        bounds: GridRect,
        profile_frame: GridRect,
    ) -> Option<SelectionThumbGrab> {
        if !self.is_open() || id != self.id(THUMB) {
            return None;
        }
        let epoch = self.grab_epoch?;
        let mut grab = SelectionThumbGrab::new(
            row,
            self.scroll.offset,
            self.geometry(bounds, profile_frame),
            self.kind,
        )?;
        grab.epoch = epoch;
        grab.surface = self.surface();
        Some(grab)
    }

    pub(crate) fn has_scrollbar(&self, bounds: GridRect, profile_frame: GridRect) -> bool {
        self.is_open() && self.geometry(bounds, profile_frame).bar
    }

    pub(crate) fn thumb_grab_valid(
        &self,
        grab: SelectionThumbGrab,
        bounds: GridRect,
        profile_frame: GridRect,
    ) -> bool {
        self.is_open()
            && grab.surface == self.surface()
            && self.grab_epoch == Some(grab.epoch)
            && grab.valid_for(self.geometry(bounds, profile_frame), self.kind)
    }

    pub(crate) fn drag_thumb(
        &mut self,
        row: u32,
        grab: SelectionThumbGrab,
        bounds: GridRect,
        profile_frame: GridRect,
    ) -> bool {
        if !self.thumb_grab_valid(grab, bounds, profile_frame) {
            return false;
        }
        let before = self.scroll.offset;
        self.scroll.offset = grab.offset_at(row);
        self.settle(self.geometry(bounds, profile_frame));
        self.scroll.offset != before
    }

    fn geometry(&self, bounds: GridRect, profile_frame: GridRect) -> Geometry {
        if self.kind == SelectionKind::Profiles {
            Geometry::from_action_box(profile_frame, self.len())
        } else {
            Geometry::of(bounds, self.len())
        }
    }

    fn hover_action_buttons(
        &self,
        grid: &mut CellGrid,
        ctx: &WidgetContext<'_>,
        frame: GridRect,
        actions: GridRect,
    ) -> bool {
        let ok_width = actions.width().saturating_sub(1) / 2;
        let span = match ctx.interaction.hovered {
            Some(id) if id == self.id(OK) => actions.col()..actions.col().saturating_add(ok_width),
            Some(id) if id == self.id(CANCEL) => {
                actions.col().saturating_add(ok_width).saturating_add(1)
                    ..actions.col().saturating_add(actions.width())
            }
            _ => return false,
        };
        hover_row(grid, actions.row(), frame.col(), span)
    }

    fn draw_content(&self, grid: &mut CellGrid, ctx: &WidgetContext<'_>, inner: GridRect) {
        let geometry = Geometry::from_interior(inner, self.len());
        draw_label(
            grid,
            geometry.layout.title,
            self.kind.title(),
            ctx.styles.normal,
        );
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
            thumb: self.id(THUMB),
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
    fn move_focus(&mut self, forward: bool, geometry: Geometry) -> bool {
        let ring = self.ring(geometry);
        let before = self.focus;
        self.focus = step_focus(&ring, self.focus, forward);
        if let Some(i) = self.focus.and_then(|id| self.item_index(id)) {
            let index = u32::try_from(i).unwrap_or(u32::MAX);
            self.scroll
                .ensure_visible(index, geometry.len, geometry.window);
        }
        self.focus != before
    }

    /// Focus order: every item, then ▲ and ▼ when drawn and enabled, then OK and Cancel.
    fn ring(&self, geometry: Geometry) -> Vec<WidgetId> {
        let mut ring: Vec<WidgetId> = (0..geometry.len as usize)
            .map(|i| self.item_id(i))
            .collect();
        if geometry.bar {
            let offset = self.offset(geometry.window);
            let max = ScrollState::max_offset(geometry.len, geometry.window);
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
        self.scroll.clamp(geometry.len, geometry.window);
        let ring = self.ring(geometry);
        if !self.focus.is_some_and(|f| ring.contains(&f)) {
            self.focus = Some(self.id(OK));
        }
    }

    /// The scroll offset as drawn: clamped to the valid range for `window`.
    fn offset(&self, window: u32) -> u32 {
        self.scroll
            .offset
            .min(ScrollState::max_offset(self.len(), window))
    }

    fn confirm(&mut self) {
        match self.kind {
            SelectionKind::Generic => self.applied = self.draft,
            SelectionKind::Profiles => self.profile_applied = self.draft,
        }
        self.status = ModalStatus::Confirmed;
    }

    fn cancel(&mut self) {
        self.draft = match self.kind {
            SelectionKind::Generic => self.applied,
            SelectionKind::Profiles => self.profile_applied,
        };
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
            THUMB => Slot::Thumb,
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

    /// Frame of 30x12 centred in a 40x14 grid: interior 28x10 at (6, 2);
    /// five content rows (3..=7) and three inert surplus body rows for nine items.
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
            ink: crate::InkRole::Text,
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
        // Items 0..=8, then the enabled arrow, then OK and Cancel: 12 stops.
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
        for _ in 0..4 {
            assert!(p.activate(down(), bounds));
        }
        let mut inter = InteractionState::default();
        let g = frame(&p, &inter, 40, 14);
        assert!(has(&g, "Pin_Body") && !has(&g, "Rim_Body"));
        // Bottom: ▼ is now disabled, so focus left it for OK.
        assert!(!p.activate(down(), bounds));
        assert_eq!(p.focus(), Some(ok()));
        p.sync_focus(&mut inter);
        for _ in 0..4 {
            assert!(p.activate(up(), bounds));
        }
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
    fn scrollbar_thumb_is_hit_testable_and_drags_the_list() {
        let bounds = wide();
        let profile_frame = bounds;
        let mut inter = InteractionState::default();
        let mut p = panel();
        p.open(&mut inter);
        let geometry = Geometry::of(bounds, p.len());
        let row = geometry.layout.body.row() + 1;
        let grab = p
            .thumb_grab(id(BASE + THUMB), row, bounds, profile_frame)
            .unwrap();
        assert!(!p.drag_thumb(row, grab, bounds, profile_frame));
        assert!(p.drag_thumb(row + 3, grab, bounds, profile_frame));
        assert_eq!(p.offset(geometry.window), 4);
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

#[cfg(test)]
mod color_tests {
    use super::*;
    use crate::{InkRole, WidgetStyles};
    #[test]
    fn action_box_title_frames_footer_hover_and_clipping_keep_roles_and_owners() {
        let styles = WidgetStyles::default();
        let mut inter = InteractionState::default();
        let mut panel = SelectionPanel::new(2000);
        panel.open_profiles(&mut inter);
        for width in [31, 12, 7] {
            let frame = GridRect::new(1, 1, width, 12);
            let mut grid = CellGrid::new(40, 16).unwrap();
            panel.draw_in_action_box(
                &mut grid,
                &WidgetContext {
                    styles: &styles,
                    interaction: &inter,
                },
                frame,
            );
            let title_len = panel
                .kind
                .title()
                .chars()
                .count()
                .min(width.saturating_sub(3) as usize) as u32;
            for dx in 0..width {
                let c = grid.get(1 + dx, 1).unwrap();
                assert_eq!(
                    c.ink,
                    if (2..2 + title_len).contains(&dx) {
                        InkRole::Text
                    } else {
                        InkRole::Contour
                    }
                );
                assert_eq!(c.widget_id, Some(panel.surface()));
            }
            for cell in grid.cells() {
                if matches!(
                    cell.content,
                    CellContent::Glyph('☰' | '×' | '▲' | '▼' | '┃' | '╻' | '╹')
                ) {
                    assert_eq!(cell.ink, InkRole::Action);
                }
            }
            assert_eq!(grid.get(0, 1).unwrap(), &crate::Cell::default());
            assert_eq!(grid.get(1 + width, 1).unwrap(), &crate::Cell::default());
            for dx in 0..width {
                assert_eq!(grid.get(1 + dx, 12).unwrap().ink, InkRole::Contour);
            }
            // Separator precedes the action row; both are inside the outer frame.
            for dx in 1..width - 1 {
                assert_eq!(grid.get(1 + dx, 10).unwrap().ink, InkRole::Contour);
            }
            for action in [OK, CANCEL] {
                let mut hover = inter;
                hover.hovered = Some(panel.id(action));
                let mut hovered = CellGrid::new(40, 16).unwrap();
                panel.draw_in_action_box(
                    &mut hovered,
                    &WidgetContext {
                        styles: &styles,
                        interaction: &hover,
                    },
                    frame,
                );
                for (a, b) in grid.cells().iter().zip(hovered.cells()) {
                    assert_eq!(
                        (a.ink, a.style, a.widget_id, a.shared_border),
                        (b.ink, b.style, b.widget_id, b.shared_border)
                    );
                    if a.content != b.content {
                        assert_eq!(b.ink, InkRole::Contour);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod scrollbar_tests {
    use super::super::list::scrollbar_tests::CASES;
    use super::*;
    use crate::{Cell, InkRole, WidgetStyles};

    #[test]
    fn profile_writer_matches_every_state_with_disjoint_item_and_control_ids() {
        let panel = SelectionPanel::new(500);
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let ctx = WidgetContext {
            styles: &styles,
            interaction: &inter,
        };
        for &(len, states) in CASES {
            let rows: Vec<_> = (0..len)
                .map(|i| (WidgetId::new(2000 + i), "same name"))
                .collect();
            for (offset, expected) in states.iter().enumerate() {
                let mut panel = panel.clone();
                panel.scroll.offset = offset as u32;
                let mut grid = CellGrid::new(29, 7).unwrap();
                panel.draw_profile_rows(&mut grid, &ctx, GridRect::new(0, 0, 29, 7), &rows);
                for (y, g) in expected.chars().enumerate() {
                    let cell = grid.get(28, y as u32 + 1).unwrap();
                    assert_eq!(
                        cell.content,
                        if g == ' ' {
                            CellContent::Empty
                        } else {
                            CellContent::Glyph(g)
                        }
                    );
                    assert_eq!(cell.widget_id, (g != ' ').then_some(panel.id(THUMB)));
                    if g != ' ' {
                        assert_eq!(cell.ink, InkRole::Action);
                    }
                }
                for y in 0..5 {
                    let id = rows[offset + y as usize].0;
                    for x in 0..28 {
                        assert_eq!(grid.get(x, y).unwrap().widget_id, Some(id));
                    }
                    assert_eq!(grid.get(1, y).unwrap().ink, InkRole::Action);
                    assert_eq!(grid.get(3, y).unwrap().ink, InkRole::Text);
                    assert_eq!(grid.get(26, y).unwrap().content, CellContent::Glyph('×'));
                    assert_ne!(id, panel.id(THUMB));
                }
                assert_eq!(grid.get(28, 0).unwrap().content, CellContent::Glyph('▲'));
                assert_eq!(grid.get(28, 4).unwrap().content, CellContent::Glyph('▼'));
                assert_eq!(*grid.get(0, 5).unwrap(), Cell::default());
            }
        }
    }

    #[test]
    fn profile_fitting_empty_and_shrinking_lists_clear_controls_and_reclaim_gutter() {
        let panel = SelectionPanel::new(500);
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let ctx = WidgetContext {
            styles: &styles,
            interaction: &inter,
        };
        let mut grid = CellGrid::new(29, 7).unwrap();
        for len in [6, 5, 4, 3, 2, 1, 0] {
            let rows: Vec<_> = (0..len)
                .map(|i| (WidgetId::new(2000 + i), "name"))
                .collect();
            panel.draw_profile_rows(&mut grid, &ctx, GridRect::new(0, 0, 29, 7), &rows);
            if len <= 5 {
                assert!(!grid.cells().iter().any(|cell| {
                    [
                        Some(panel.id(UP)),
                        Some(panel.id(DOWN)),
                        Some(panel.id(THUMB)),
                    ]
                    .contains(&cell.widget_id)
                }));
                if len > 0 {
                    assert_eq!(grid.get(28, 0).unwrap().widget_id, Some(rows[0].0));
                }
            }
        }
        assert!(grid.cells().iter().all(|cell| *cell == Cell::default()));
    }

    #[test]
    fn anchors_preserve_exact_quantized_offsets_and_validate_count_kind_and_layout() {
        let bounds = GridRect::new(0, 0, 40, 14);
        for &(len, states) in CASES {
            let geometry = Geometry::of(bounds, len);
            for (s, state) in states.iter().enumerate() {
                for (i, g) in state.chars().enumerate().filter(|(_, g)| *g != ' ') {
                    let _ = g;
                    let row = geometry.layout.body.row() + 1 + i as u32;
                    let grab =
                        SelectionThumbGrab::new(row, s as u32, geometry, SelectionKind::Generic)
                            .unwrap();
                    assert_eq!(grab.offset_at(row), s as u32);
                    assert_eq!(grab.offset_at(0), 0);
                    assert_eq!(grab.offset_at(u32::MAX), len - 5);
                    assert!(grab.valid_for(geometry, SelectionKind::Generic));
                    assert!(!grab.valid_for(geometry, SelectionKind::Profiles));
                    assert!(!grab.valid_for(Geometry::of(bounds, len + 1), SelectionKind::Generic));
                    assert!(!grab.valid_for(
                        Geometry::of(GridRect::new(0, 0, 50, 14), len),
                        SelectionKind::Generic
                    ));
                }
            }
        }
        for n in 0..=5 {
            assert!(
                SelectionThumbGrab::new(4, 0, Geometry::of(bounds, n), SelectionKind::Generic)
                    .is_none()
            );
        }
    }

    #[test]
    fn reconciliation_clamps_count_changes_preserving_draft_and_focus_identity() {
        let bounds = GridRect::new(0, 0, 40, 14);
        let mut panel = SelectionPanel::new(500);
        panel.open(&mut InteractionState::default());
        panel.scroll.offset = 4;
        panel.focus = Some(panel.id(DOWN));
        let original = panel.draft;
        panel.settle(Geometry::of(bounds, 6));
        assert_eq!(panel.scroll.offset, 1);
        assert_eq!(panel.focus, Some(panel.id(OK)));
        panel.focus = Some(panel.item_id(1));
        panel.settle(Geometry::of(bounds, 5));
        assert_eq!(panel.scroll.offset, 0);
        assert_eq!(panel.focus, Some(panel.item_id(1)));
        panel.settle(Geometry::of(bounds, 6));
        assert_eq!(panel.scroll.offset, 0);
        panel.settle(Geometry::of(bounds, 0));
        assert_eq!(panel.focus, Some(panel.id(OK)));
        assert_eq!(panel.draft, original);
        panel.scroll.offset = 3;
        panel.settle(Geometry::of(bounds, 11));
        panel.settle(Geometry::of(bounds, 1000));
        assert_eq!(panel.scroll.offset, 3);
    }

    #[test]
    fn wheel_extremes_and_duplicate_profiles_keep_selection_and_item_identity() {
        let bounds = GridRect::new(0, 0, 40, 14);
        for profiles in [false, true] {
            let mut panel = SelectionPanel::new(500);
            let mut inter = InteractionState::default();
            if profiles {
                panel.open_profiles(&mut inter);
            } else {
                panel.open(&mut inter);
            }
            assert!(panel.activate_in(panel.item_id(2), panel.geometry(bounds, bounds)));
            let draft = panel.draft;
            assert!(panel.scroll_wheel(i32::MIN, bounds, bounds));
            assert_eq!(panel.scroll.offset, 4);
            assert!(!panel.scroll_wheel(i32::MIN, bounds, bounds));
            assert!(panel.scroll_wheel(i32::MAX, bounds, bounds));
            assert_eq!(panel.scroll.offset, 0);
            assert_eq!(panel.draft, draft);
            assert_ne!(panel.item_id(2), panel.item_id(3));
            assert!(panel.activate_in(panel.id(OK), panel.geometry(bounds, bounds)));
            assert_eq!(
                if profiles {
                    panel.profile_applied
                } else {
                    panel.applied
                },
                draft
            );
        }
    }
}
