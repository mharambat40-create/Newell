// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Application and command menus (M2.9): plain state, keyboard and pointer handling.
//!
//! A menu set is caller-owned static data ([`MenuDef`] slices); [`MenuState`] is the only
//! mutable state. Keys and clicks mutate that state and return a [`MenuResult`] carrying at
//! most a semantic [`MenuAction`]; nothing is executed here. The same state and the same
//! code serve the MenuBar and the CommandBar (each owns its own [`MenuState`] and
//! [`MenuIds`]). Drawing ([`draw_menu_bar`], [`draw_menu_popup`]) regenerates cells from
//! the state every frame, and the drop-down is an ordinary M2.8 popup [`Overlay`].
//!
//! # Conventions
//!
//! - **Capacity.** At most [`MAX_MENUS`] menus and [`MAX_ENTRIES`] entries per menu are
//!   considered; extras are ignored everywhere (keys, drawing, ids) so ids never alias.
//! - **Escape closes everything.** From any state, Escape leaves menu navigation.
//! - **Wrapping.** Left/Right and Up/Down wrap; Up/Down skip separators and disabled items.
//! - **Mnemonics are short tokens** ([`Mnemonic`], 1 to [`MAX_MNEMONIC`] ASCII letters or
//!   digits, e.g. `M`, `Mi`, `Pl`), matched exactly and case-insensitively; `M` never
//!   matches `Mi`. Labels are drawn verbatim, so a caller may write `[Mi]rror`. Within an
//!   open menu a unique match activates the item; an ambiguous one moves the highlight to
//!   the next match. Top-level mnemonics open that menu. The controller assembles the typed
//!   token and decides when to send [`MenuKey::Mnemonic`] (e.g. Alt+key while closed).
//! - **Entry kinds.** Command items, separators, checkboxes and cyclic rows. Checkbox and
//!   cyclic rows only *request* a change ([`MenuResult::Toggled`], [`MenuResult::Cycled`]);
//!   the caller owns the logical state and re-supplies it in the next [`MenuDef`]. Those
//!   requests keep the popup open; command activation closes it.
//! - **Cyclic rows.** Left/Right while one is highlighted request previous/next instead of
//!   switching top-level menu; Enter/Space on one is consumed without a request.
//! - **Dismissal.** A click that is not on a menu title, the popup or an item closes the
//!   menu and is reported as handled (consumed).
//! - **No scrolling.** A popup taller than its bounds shows its first rows only.

mod draw;

use crate::{GridRect, WidgetId, center_modal, place_popup};
use crate::{HitTestResult, InteractionState, Overlay, OverlayKind, OverlayStack};

pub use draw::{draw_menu_bar, draw_menu_popup, popup_size, title_rect};

/// Maximum number of menus considered per menu set.
pub const MAX_MENUS: usize = 64;
/// Maximum number of entries considered per menu.
pub const MAX_ENTRIES: usize = 256;

/// Semantic identifier of a menu item, chosen by the caller. Returned on activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MenuAction(u32);

impl MenuAction {
    /// Creates an action id.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Longest mnemonic token, in characters.
pub const MAX_MNEMONIC: usize = 4;

/// A mnemonic token: 1 to [`MAX_MNEMONIC`] ASCII letters or digits, stored lowercased inline.
/// Anything else (empty, too long, non-ASCII) yields an empty token that matches nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mnemonic {
    bytes: [u8; MAX_MNEMONIC],
    len: u8,
}

impl Mnemonic {
    const EMPTY: Self = Self {
        bytes: [0; MAX_MNEMONIC],
        len: 0,
    };

    /// Builds a token from `text` (case-insensitive).
    #[must_use]
    pub const fn new(text: &str) -> Self {
        let src = text.as_bytes();
        if src.is_empty() || src.len() > MAX_MNEMONIC {
            return Self::EMPTY;
        }
        let mut bytes = [0; MAX_MNEMONIC];
        let mut i = 0;
        while i < src.len() {
            if !src[i].is_ascii_alphanumeric() {
                return Self::EMPTY;
            }
            bytes[i] = src[i].to_ascii_lowercase();
            i += 1;
        }
        Self {
            bytes,
            len: src.len() as u8,
        }
    }

    /// Builds a one-character token.
    #[must_use]
    pub const fn from_char(c: char) -> Self {
        if c.is_ascii_alphanumeric() {
            Self {
                bytes: [c.to_ascii_lowercase() as u8, 0, 0, 0],
                len: 1,
            }
        } else {
            Self::EMPTY
        }
    }

    /// `true` for a token that can never match.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn matches(self, other: Self) -> bool {
        !self.is_empty() && self == other
    }
}

/// An activatable menu item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuItem<'a> {
    pub action: MenuAction,
    pub label: &'a str,
    pub mnemonic: Option<Mnemonic>,
    pub enabled: bool,
}

/// A checkbox row; `checked` is the caller's logical state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuCheckbox<'a> {
    pub action: MenuAction,
    pub label: &'a str,
    pub checked: bool,
    pub mnemonic: Option<Mnemonic>,
    pub enabled: bool,
}

/// A cyclic row showing `values[current]` between `◀` and `▶`; `current` is the caller's
/// logical state. A non-empty `label` is drawn before the arrows, one space apart
/// (`UI Scale ◀ 100% ▶`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuCycle<'a> {
    pub action: MenuAction,
    pub label: &'a str,
    pub values: &'a [&'a str],
    pub current: usize,
    pub enabled: bool,
}

/// Direction of a requested cyclic change; apply with [`crate::cycle_next`] /
/// [`crate::cycle_prev`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleStep {
    Previous,
    Next,
}

/// One row of a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuEntry<'a> {
    Item(MenuItem<'a>),
    Separator,
    Checkbox(MenuCheckbox<'a>),
    Cyclic(MenuCycle<'a>),
}

impl MenuEntry<'_> {
    /// `true` for an enabled row (the only kind keyboard navigation can land on).
    #[must_use]
    pub const fn is_selectable(&self) -> bool {
        match self {
            Self::Item(i) => i.enabled,
            Self::Checkbox(c) => c.enabled,
            Self::Cyclic(c) => c.enabled,
            Self::Separator => false,
        }
    }

    fn matches_mnemonic(&self, key: Mnemonic) -> bool {
        match self {
            Self::Item(i) => i.enabled && i.mnemonic.is_some_and(|m| m.matches(key)),
            Self::Checkbox(c) => c.enabled && c.mnemonic.is_some_and(|m| m.matches(key)),
            Self::Cyclic(_) | Self::Separator => false,
        }
    }
}

/// A top-level menu: a title and its entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuDef<'a> {
    pub label: &'a str,
    pub mnemonic: Option<Mnemonic>,
    pub entries: &'a [MenuEntry<'a>],
}

impl<'a> MenuDef<'a> {
    /// Entries within [`MAX_ENTRIES`].
    #[must_use]
    pub fn entries(&self) -> &'a [MenuEntry<'a>] {
        &self.entries[..self.entries.len().min(MAX_ENTRIES)]
    }
}

pub(crate) fn capped<'m, 'a>(menus: &'m [MenuDef<'a>]) -> &'m [MenuDef<'a>] {
    &menus[..menus.len().min(MAX_MENUS)]
}

/// Keys the menu system reacts to (already translated from window events).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKey {
    Left,
    Right,
    Up,
    Down,
    Enter,
    Space,
    Escape,
    Mnemonic(Mnemonic),
}

/// Outcome of feeding input to a [`MenuState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuResult {
    /// Not meaningful to the menu; the caller may route it elsewhere.
    Ignored,
    /// The state changed or the input was consumed.
    Handled,
    /// An enabled command item was activated; the menu is now closed.
    Activated(MenuAction),
    /// A checkbox row requests a toggle; the popup stays open.
    Toggled(MenuAction),
    /// A cyclic row requests a previous/next value; the popup stays open.
    Cycled(MenuAction, CycleStep),
}

/// What a menu [`WidgetId`] refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuTarget {
    Title(usize),
    /// The popup surface (frame, separators): inside the menu, on no item.
    Popup,
    Item {
        menu: usize,
        entry: usize,
    },
    /// The `◀` control of a cyclic row.
    CyclePrev {
        menu: usize,
        entry: usize,
    },
    /// The `▶` control of a cyclic row.
    CycleNext {
        menu: usize,
        entry: usize,
    },
}

/// Stable [`WidgetId`] scheme of one menu set, derived from a caller-chosen base.
///
/// Layout (offsets from `base`): titles `0..MAX_MENUS`, popup surface `MAX_MENUS`, items
/// from `MAX_MENUS + 1`, [`MAX_ENTRIES`] per menu, 3 ids per entry (the row,
/// then the `◀` and `▶` controls of a cyclic row). The ids depend only on indices, never on
/// state or position. Two sets must use bases at least [`MenuIds::SPAN`] apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuIds {
    base: u32,
}

const POPUP_OFFSET: usize = MAX_MENUS;
const ITEMS_OFFSET: usize = MAX_MENUS + 1;
/// Ids reserved per entry: row, cyclic `◀`, cyclic `▶`.
const ENTRY_STRIDE: usize = 3;
const PER_MENU: usize = MAX_ENTRIES * ENTRY_STRIDE;

impl MenuIds {
    /// Number of ids reserved from `base`.
    pub const SPAN: u32 = (ITEMS_OFFSET + MAX_MENUS * PER_MENU) as u32;

    /// Creates the scheme; `base` is limited so every id fits in `u32`.
    #[must_use]
    pub const fn new(base: u32) -> Self {
        let max = u32::MAX - Self::SPAN;
        Self {
            base: if base > max { max } else { base },
        }
    }

    fn at(self, offset: usize) -> WidgetId {
        WidgetId::new(self.base + u32::try_from(offset).unwrap_or(0))
    }

    /// Id of the title of menu `index` (`None` past [`MAX_MENUS`]).
    #[must_use]
    pub fn title(self, index: usize) -> Option<WidgetId> {
        (index < MAX_MENUS).then(|| self.at(index))
    }

    /// Id of the popup surface.
    #[must_use]
    pub fn popup(self) -> WidgetId {
        self.at(POPUP_OFFSET)
    }

    /// Id of entry `entry` of menu `menu` (`None` past the capacities).
    #[must_use]
    pub fn item(self, menu: usize, entry: usize) -> Option<WidgetId> {
        self.sub(menu, entry, 0)
    }

    /// Id of the `◀` control of cyclic entry `entry` of menu `menu`.
    #[must_use]
    pub fn cycle_prev(self, menu: usize, entry: usize) -> Option<WidgetId> {
        self.sub(menu, entry, 1)
    }

    /// Id of the `▶` control of cyclic entry `entry` of menu `menu`.
    #[must_use]
    pub fn cycle_next(self, menu: usize, entry: usize) -> Option<WidgetId> {
        self.sub(menu, entry, 2)
    }

    fn sub(self, menu: usize, entry: usize, part: usize) -> Option<WidgetId> {
        (menu < MAX_MENUS && entry < MAX_ENTRIES)
            .then(|| self.at(ITEMS_OFFSET + menu * PER_MENU + entry * ENTRY_STRIDE + part))
    }

    /// Inverse of the id functions; `None` for ids outside this set.
    #[must_use]
    pub fn decode(self, id: WidgetId) -> Option<MenuTarget> {
        let v = usize::try_from(id.get().checked_sub(self.base)?).ok()?;
        if v < MAX_MENUS {
            Some(MenuTarget::Title(v))
        } else if v == POPUP_OFFSET {
            Some(MenuTarget::Popup)
        } else {
            let r = v - ITEMS_OFFSET;
            if r >= MAX_MENUS * PER_MENU {
                return None;
            }
            let (menu, entry) = (r / PER_MENU, r % PER_MENU / ENTRY_STRIDE);
            Some(match r % ENTRY_STRIDE {
                0 => MenuTarget::Item { menu, entry },
                1 => MenuTarget::CyclePrev { menu, entry },
                _ => MenuTarget::CycleNext { menu, entry },
            })
        }
    }
}

/// Menu navigation state: whether navigation is active, the current top-level menu, whether
/// its popup is open and which entry is highlighted. Plain `Copy` data; indices are
/// re-validated against the menu set on every use, so stale values are harmless.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuState {
    active: bool,
    menu: usize,
    open: bool,
    item: Option<usize>,
}

/// Next entry satisfying `pred` from `from` (exclusive), wrapping; with no valid `from`,
/// the first (`forward`) or last entry. `None` if nothing qualifies.
fn step(
    entries: &[MenuEntry<'_>],
    from: Option<usize>,
    forward: bool,
    pred: impl Fn(&MenuEntry<'_>) -> bool,
) -> Option<usize> {
    let n = entries.len();
    let from = from.filter(|&i| i < n);
    (1..=n)
        .map(|k| match (from, forward) {
            (Some(i), true) => (i + k) % n,
            (Some(i), false) => (i + n - k % n) % n,
            (None, true) => k - 1,
            (None, false) => n - k,
        })
        .find(|&i| pred(&entries[i]))
}

impl MenuState {
    /// `true` while menu navigation is active (bar highlighted or a popup open).
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.active
    }

    /// The current top-level menu while navigation is active.
    #[must_use]
    pub const fn current_menu(&self) -> Option<usize> {
        if self.active { Some(self.menu) } else { None }
    }

    /// The top-level menu whose popup is open.
    #[must_use]
    pub const fn open_menu(&self) -> Option<usize> {
        if self.active && self.open {
            Some(self.menu)
        } else {
            None
        }
    }

    /// The highlighted entry of the open popup.
    #[must_use]
    pub const fn highlighted(&self) -> Option<usize> {
        if self.active && self.open {
            self.item
        } else {
            None
        }
    }

    /// Leaves menu navigation and closes any popup.
    pub fn close(&mut self) {
        *self = Self::default();
    }

    /// Starts keyboard navigation on the first menu, popup closed. `false` if `menus` is
    /// empty.
    pub fn begin(&mut self, menus: &[MenuDef<'_>]) -> bool {
        if capped(menus).is_empty() {
            return false;
        }
        *self = Self {
            active: true,
            ..Self::default()
        };
        true
    }

    /// Opens menu `index` with its first enabled entry highlighted. `false` (state
    /// unchanged) if `index` is out of range.
    pub fn open(&mut self, menus: &[MenuDef<'_>], index: usize) -> bool {
        self.open_at(menus, index, true)
    }

    fn open_at(&mut self, menus: &[MenuDef<'_>], index: usize, highlight: bool) -> bool {
        let Some(def) = capped(menus).get(index) else {
            return false;
        };
        let item = if highlight {
            step(def.entries(), None, true, |e| e.is_selectable())
        } else {
            None
        };
        *self = Self {
            active: true,
            menu: index,
            open: true,
            item,
        };
        true
    }

    fn activate(&mut self, entry: Option<&MenuEntry<'_>>) -> MenuResult {
        match entry {
            Some(MenuEntry::Item(item)) if item.enabled => {
                self.close();
                MenuResult::Activated(item.action)
            }
            Some(MenuEntry::Checkbox(cb)) if cb.enabled => MenuResult::Toggled(cb.action),
            _ => MenuResult::Handled,
        }
    }

    /// Requests a cyclic change if `entry` is an enabled cyclic row.
    fn cycle(entry: Option<&MenuEntry<'_>>, step: CycleStep) -> MenuResult {
        match entry {
            Some(MenuEntry::Cyclic(c)) if c.enabled => MenuResult::Cycled(c.action, step),
            _ => MenuResult::Handled,
        }
    }

    /// Applies a key. See the module docs for the semantics.
    pub fn handle_key(&mut self, menus: &[MenuDef<'_>], key: MenuKey) -> MenuResult {
        let menus = capped(menus);
        if self.active && self.menu >= menus.len() {
            self.close();
        }
        if let MenuKey::Mnemonic(c) = key {
            return self.mnemonic(menus, c);
        }
        if !self.active {
            return MenuResult::Ignored;
        }
        let n = menus.len();
        let entries = menus[self.menu].entries();
        match key {
            MenuKey::Escape => self.close(),
            MenuKey::Left | MenuKey::Right
                if self.open
                    && self
                        .item
                        .and_then(|i| entries.get(i))
                        .is_some_and(|e| matches!(e, MenuEntry::Cyclic(c) if c.enabled)) =>
            {
                let step = if key == MenuKey::Right {
                    CycleStep::Next
                } else {
                    CycleStep::Previous
                };
                return Self::cycle(self.item.and_then(|i| entries.get(i)), step);
            }
            MenuKey::Left | MenuKey::Right => {
                let next = if key == MenuKey::Right {
                    (self.menu + 1) % n
                } else {
                    (self.menu + n - 1) % n
                };
                if self.open {
                    self.open_at(menus, next, true);
                } else {
                    self.menu = next;
                }
            }
            MenuKey::Up | MenuKey::Down => {
                let from = if self.open { self.item } else { None };
                self.item = step(entries, from, key == MenuKey::Down, |e| e.is_selectable());
                self.open = true;
            }
            MenuKey::Enter | MenuKey::Space => {
                if !self.open {
                    self.open_at(menus, self.menu, true);
                } else {
                    let entry = self.item.and_then(|i| entries.get(i));
                    return self.activate(entry);
                }
            }
            MenuKey::Mnemonic(_) => {}
        }
        MenuResult::Handled
    }

    fn mnemonic(&mut self, menus: &[MenuDef<'_>], c: Mnemonic) -> MenuResult {
        if self.open_menu().is_some_and(|m| m < menus.len()) {
            let entries = menus[self.menu].entries();
            let count = entries.iter().filter(|e| e.matches_mnemonic(c)).count();
            let pred = |e: &MenuEntry<'_>| e.matches_mnemonic(c);
            match count {
                0 => {}
                1 => {
                    let entry = step(entries, None, true, pred).and_then(|i| entries.get(i));
                    return self.activate(entry);
                }
                _ => {
                    self.item = step(entries, self.item, true, pred);
                    return MenuResult::Handled;
                }
            }
        }
        let top = menus
            .iter()
            .position(|m| m.mnemonic.is_some_and(|k| k.matches(c)));
        match top {
            Some(i) if self.open_at(menus, i, true) => MenuResult::Handled,
            _ => MenuResult::Ignored,
        }
    }

    /// Applies a pointer click given the real [`crate::hit_test()`] result.
    ///
    /// Title: opens that menu (or closes it if already open). Enabled item: activates.
    /// Disabled item / popup surface: consumed, no change. Anything else: closes the menu
    /// if navigation was active (consumed), else [`MenuResult::Ignored`].
    pub fn handle_click(
        &mut self,
        menus: &[MenuDef<'_>],
        ids: MenuIds,
        hit: Option<HitTestResult>,
    ) -> MenuResult {
        let menus = capped(menus);
        let target = hit.and_then(|h| h.widget_id).and_then(|w| ids.decode(w));
        match target {
            Some(MenuTarget::Title(i)) if i < menus.len() => {
                if self.open_menu() == Some(i) {
                    self.close();
                } else {
                    self.open_at(menus, i, false);
                }
                MenuResult::Handled
            }
            Some(MenuTarget::Item { menu, entry }) if self.open_menu() == Some(menu) => {
                let e = menus.get(menu).and_then(|m| m.entries().get(entry));
                let result = self.activate(e);
                if matches!(result, MenuResult::Toggled(_)) {
                    self.item = Some(entry);
                }
                result
            }
            Some(MenuTarget::CyclePrev { menu, entry } | MenuTarget::CycleNext { menu, entry })
                if self.open_menu() == Some(menu) =>
            {
                let step = if matches!(target, Some(MenuTarget::CycleNext { .. })) {
                    CycleStep::Next
                } else {
                    CycleStep::Previous
                };
                let e = menus.get(menu).and_then(|m| m.entries().get(entry));
                let result = Self::cycle(e, step);
                if result != MenuResult::Handled {
                    self.item = Some(entry);
                }
                result
            }
            Some(MenuTarget::Popup) if self.open => MenuResult::Handled,
            _ if self.active => {
                self.close();
                MenuResult::Handled
            }
            _ => MenuResult::Ignored,
        }
    }

    /// The widget that should hold keyboard focus: the highlighted item, else the current
    /// title while navigating.
    #[must_use]
    pub fn focus_id(&self, ids: MenuIds) -> Option<WidgetId> {
        if !self.active {
            return None;
        }
        self.highlighted()
            .and_then(|e| ids.item(self.menu, e))
            .or_else(|| ids.title(self.menu))
    }

    /// Mirrors the state into [`InteractionState::focused`]: while active, focus follows
    /// [`MenuState::focus_id`]; when inactive, focus is cleared only if it is still one of
    /// this set's ids (other widgets keep theirs).
    pub fn sync_focus(&self, inter: &mut InteractionState, ids: MenuIds) {
        if self.active {
            inter.set_focused(self.focus_id(ids));
        } else if inter.focused.is_some_and(|f| ids.decode(f).is_some()) {
            inter.set_focused(None);
        }
    }

    /// Opens (or dismisses) the popup [`Overlay`] in `stack` to match the state. The popup
    /// is anchored under the title in `bar` and clipped to `bounds` by [`place_popup`].
    pub fn sync_overlay(
        &self,
        stack: &mut OverlayStack,
        menus: &[MenuDef<'_>],
        ids: MenuIds,
        bar: GridRect,
        bounds: GridRect,
    ) {
        let menus = capped(menus);
        let open = self
            .open_menu()
            .and_then(|i| Some((menus.get(i)?, title_rect(bar, menus, i)?)));
        match open {
            Some((def, anchor)) => stack.open(Overlay {
                id: ids.popup(),
                kind: OverlayKind::Popup,
                rect: place_popup(anchor, popup_size(def), bounds),
            }),
            None => {
                stack.dismiss(ids.popup());
            }
        }
    }

    /// Like [`MenuState::sync_overlay`] for the CommandBar (`README_UI.md` §29.1): the popup is
    /// centred in `bounds` instead of anchored under its title, and is at least `min_width`
    /// cells wide (the fixed 18-cell frames of `Command_Bar.txt`).
    pub fn sync_centered_overlay(
        &self,
        stack: &mut OverlayStack,
        menus: &[MenuDef<'_>],
        ids: MenuIds,
        bounds: GridRect,
        min_width: u32,
    ) {
        match self.open_menu().and_then(|i| capped(menus).get(i)) {
            Some(def) => {
                let (width, height) = popup_size(def);
                stack.open(Overlay {
                    id: ids.popup(),
                    kind: OverlayKind::Popup,
                    rect: center_modal((width.max(min_width), height), bounds),
                });
            }
            None => {
                stack.dismiss(ids.popup());
            }
        }
    }
}

#[cfg(test)]
mod tests;
