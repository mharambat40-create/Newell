// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Parts shared by every Action Box (M2.21 grammar, M2.22 operations): the Create header, the
//! OK / Cancel footer, `Merge with all`, the expanded-offset rows, the generic UI state and the
//! id decoding every operation uses.
//!
//! Everything composes into an [`ActionBox`] row by row from literal reference text and the grid
//! primitives; nothing here is laid out by an engine, retained, or connected to geometry.

use crate::WidgetId;
use crate::action_box::{ActionBox, KEY_SLOTS, Row};
use crate::extrude::{
    ExtrudeGeometryType, ExtrudeOffsetMode, ExtrudeResultMode, ExtrudeStatus, MockSelection,
};

pub(crate) const K_GEOMETRY: u32 = 0;
pub(crate) const K_RESULT: u32 = 1;
pub(crate) const K_PROFILE: u32 = 2;
/// The nested `End Offset` of a `2nd side` (shared by Extrude and Revolve).
pub(crate) const K_NESTED: u32 = 22;
pub(crate) const K_NESTED_MODE: u32 = 23;
pub(crate) const K_NESTED_DIST: u32 = 24;
pub(crate) const K_NESTED_TARGET: u32 = 25;
pub(crate) const K_MERGE: u32 = 26;
pub(crate) const K_MERGE_TARGETS: u32 = 27;
pub(crate) const K_ACTIONS: u32 = 28;

/// Ids reserved for one Action Box: 32 control keys of [`KEY_SLOTS`] slots. Only one box is
/// shown at a time, so all of them share the same id range.
pub(crate) const ID_SPAN: u32 = 32 * KEY_SLOTS;

/// The four border rows of the header for each `(geometry, result)`, in the order
/// Solid/New, Solid/Add, Solid/Remove, Surface/New, Surface/Add, Surface/Remove
/// (`Interface_Elements.txt`, "Head Action Box variants"): the border under `Solid | Surface`,
/// the glyph between `New` and `Add`, the glyph between `Add` and `Remove`, and the border
/// under `New | Add | Remove`. The active segment is outlined by solid lines, the others by
/// the dotted `╴` and `┆`.
const HEAD: [(&str, char, char, &str); 6] = [
    (
        "├───────┬─────┘╴╴╴┐╴╴╴╴╴╴╴╴╴╴╴┤",
        '│',
        '┆',
        "├───────┘╴╴╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴╴╴╴╴┤",
    ),
    (
        "├───────┬─────┴───┐╴╴╴╴╴╴╴╴╴╴╴┤",
        '│',
        '│',
        "│╴╴╴╴╴╴╴┴─────────┘╴╴╴╴╴╴╴╴╴╴╴┤",
    ),
    (
        "├───────┬─────┘╴╴╴┬───────────┤",
        '┆',
        '│',
        "├╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴╴╴┴───────────┤",
    ),
    (
        "├───────┐╴╴╴╴╴┴───┬───────────┤",
        '│',
        '┆',
        "├───────┘╴╴╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴╴╴╴╴┤",
    ),
    (
        "│╴╴╴╴╴╴╴┬─────┴───┬───────────┤",
        '│',
        '│',
        "│╴╴╴╴╴╴╴┴─────────┘╴╴╴╴╴╴╴╴╴╴╴┤",
    ),
    (
        "│╴╴╴╴╴╴╴┐╴╴╴╴╴┴───┬───────────┤",
        '┆',
        '│',
        "│╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴╴╴┴───────────┤",
    ),
];

const SEPARATOR_ROW: &str = "│─────────────────────────────│";
const BOTTOM_ROW: &str = "╰─────────────────────────────╯";

/// `╭─Title─────┬───────────────╮`: the title sits in the top border, the `┬` over the
/// `Solid | Surface` divider.
#[cfg(test)]
pub(crate) fn top_border(title: &str) -> String {
    let shown: String = title.chars().take(12).collect();
    format!("╭─{shown:─<12}┬───────────────╮")
}

fn top_row(base: u32, title: &str) -> Row {
    let shown: String = title.chars().take(12).collect();
    Row::new(base)
        .contour("╭─")
        .text(&shown)
        .contour(&"─".repeat(12 - shown.chars().count()))
        .contour("┬───────────────╮")
}

/// Top border, `Solid | Surface` and `New | Add | Remove` with the border topology of the
/// active pair (`README_UI.md` §6).
pub(crate) fn push_head(
    b: &mut ActionBox,
    base: u32,
    title: &str,
    geometry: ExtrudeGeometryType,
    result: ExtrudeResultMode,
) {
    let (under_type, new_add, add_remove, under_mode) =
        HEAD[geometry as usize * 3 + result as usize];
    b.push(top_row(base, title));
    b.push(
        Row::new(base)
            .contour("│")
            .button(K_GEOMETRY, 0, "Solid", 13)
            .divider('│', K_GEOMETRY, 0, 1)
            .button(K_GEOMETRY, 1, "Surface", 15)
            .contour("│"),
    );
    b.push(Row::new(base).contour(under_type));
    b.push(
        Row::new(base)
            .contour("│")
            .button(K_RESULT, 0, "New", 7)
            .divider(new_add, K_RESULT, 0, 1)
            .button(K_RESULT, 1, "Add", 9)
            .divider(add_remove, K_RESULT, 1, 2)
            .button(K_RESULT, 2, "Remove", 11)
            .contour("│"),
    );
    b.push(Row::new(base).contour(under_mode));
}

/// The separator, `OK | Cancel` and the bottom border.
pub(crate) fn push_footer(b: &mut ActionBox, base: u32) {
    b.push(Row::new(base).contour(SEPARATOR_ROW));
    b.push(
        Row::new(base)
            .contour("│")
            .action_button(K_ACTIONS, 0, "OK", 14)
            .divider('│', K_ACTIONS, 0, 1)
            .action_button(K_ACTIONS, 1, "Cancel", 14)
            .contour("│"),
    );
    b.push(Row::new(base).contour(BOTTOM_ROW));
}

/// `Merge with all`, and `Targets` while it is unchecked.
pub(crate) fn push_merge(b: &mut ActionBox, base: u32, on: bool, targets: MockSelection) {
    b.push(
        Row::new(base)
            .contour("│ ")
            .check_fill(K_MERGE, on, "Merge with all")
            .end(),
    );
    if !on {
        b.push(
            Row::new(base)
                .contour("│")
                .text(" ╰╴Targets   ")
                .select(K_MERGE_TARGETS, targets.count())
                .end(),
        );
    }
}

/// `■ <label> <gap> ◀ Blind ▶` of an expanded offset, after `prefix`. `gap` blanks separate
/// the label from the selector so every `◀` sits in column 19 (`End Offset` is two cells
/// shorter than `Start Offset`).
pub(crate) fn offset_head(
    b: &mut ActionBox,
    base: u32,
    prefix: &str,
    (key, mode_key): (u32, u32),
    (label, gap): (&str, usize),
    mode: ExtrudeOffsetMode,
) {
    let width = u32::try_from(label.chars().count() + 2).unwrap_or(0);
    b.push(
        Row::new(base)
            .contour("│")
            .text(&prefix.chars().skip(1).collect::<String>())
            .check(key, true, label, width)
            .text(&" ".repeat(gap))
            .cycle(mode_key, mode.label(), 9)
            .end(),
    );
}

/// The child of an expanded offset: the `blind` row (`label` then a numeric field showing
/// `text`) for `Blind`, `Target` for `Up to`. `prefix` ends with the branch glyphs.
pub(crate) fn offset_children(
    b: &mut ActionBox,
    base: u32,
    prefix: &str,
    keys: (u32, u32),
    mode: ExtrudeOffsetMode,
    target: MockSelection,
    (label, text): (&str, &str),
) {
    match mode {
        ExtrudeOffsetMode::Blind => b.push(
            Row::new(base)
                .contour("│")
                .text(&format!(
                    "{}{label}",
                    prefix.chars().skip(1).collect::<String>()
                ))
                .numeric(keys.0, text)
                .end(),
        ),
        ExtrudeOffsetMode::UpTo => b.push(
            Row::new(base)
                .contour("│")
                .text(&format!(
                    "{}Target  ",
                    prefix.chars().skip(1).collect::<String>()
                ))
                .select(keys.1, target.count())
                .end(),
        ),
    }
}

/// The `End Offset` nested in `2nd side = Up to`: collapsed, or expanded with `Dist` / `Angle`
/// (`blind` = label and field text) or `Target`.
pub(crate) fn push_nested_end(
    b: &mut ActionBox,
    base: u32,
    on: bool,
    mode: ExtrudeOffsetMode,
    target: MockSelection,
    (label, text): (&str, &str),
) {
    if !on {
        b.push(
            Row::new(base)
                .contour("│")
                .text("   ╰╴")
                .check_fill(K_NESTED, false, "End Offset")
                .end(),
        );
        return;
    }
    offset_head(
        b,
        base,
        "│   ╰╴",
        (K_NESTED, K_NESTED_MODE),
        ("End Offset", 1),
        mode,
    );
    match mode {
        ExtrudeOffsetMode::Blind => b.push(
            Row::new(base)
                .contour("│")
                .text(&format!("     ╰╴{label}"))
                .numeric(K_NESTED_DIST, text)
                .end(),
        ),
        ExtrudeOffsetMode::UpTo => b.push(
            Row::new(base)
                .contour("│")
                .text("     ╰╴Target")
                .select(K_NESTED_TARGET, target.count())
                .end(),
        ),
    }
}

/// UI state of one Action Box other than Extrude: plain values, the values the box opened
/// with (restored by Cancel) and whether it is still being edited. The operation modules
/// implement `ActionBoxUi` for their instantiation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionState<V> {
    base_id: u32,
    pub values: V,
    opened: V,
    pub status: ExtrudeStatus,
}

impl<V: Copy + Default + PartialEq> ActionState<V> {
    /// Opens the box with default values; `base_id` is its first widget id.
    #[must_use]
    pub fn new(base_id: u32) -> Self {
        let values = V::default();
        Self {
            base_id,
            values,
            opened: values,
            status: ExtrudeStatus::Editing,
        }
    }

    /// Whether the box is still shown (neither OK nor Cancel was activated).
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status == ExtrudeStatus::Editing
    }

    /// Reopens with default values.
    pub fn reopen(&mut self) {
        *self = Self::new(self.base_id);
    }

    pub(crate) const fn base(&self) -> u32 {
        self.base_id
    }

    pub(crate) fn row(&self) -> Row {
        Row::new(self.base_id)
    }

    /// OK: the box closes and keeps its values.
    pub(crate) fn confirm(&mut self) {
        self.opened = self.values;
        self.status = ExtrudeStatus::Confirmed;
    }

    /// Cancel: the box closes and the values it opened with return.
    pub(crate) fn cancel(&mut self) {
        self.values = self.opened;
        self.status = ExtrudeStatus::Cancelled;
    }
}

/// What the shell needs of any Action Box, so one dispatch serves all five.
pub(crate) trait ActionBoxUi {
    fn base_id(&self) -> u32;
    /// The composed box, or `None` once OK / Cancel closed it.
    fn compose_box(&self) -> Option<ActionBox>;
    /// Applies activation of `id` to the mock state; `true` if the state changed.
    fn activate_id(&mut self, id: WidgetId) -> bool;
    fn showing(&self) -> bool;
    /// Reopens with default values.
    fn restart(&mut self);
    /// Keys of the numeric rows (field slot 0, formula `☰` slot 1).
    fn numeric_keys(&self) -> &'static [u32];
    /// Keys of the selection fields (slot 0 the field, slot 1 the `☰` of a populated one).
    fn select_keys(&self) -> &'static [u32];

    /// `(key, slot)` of `id` if it lies in this box's id range.
    fn decode(&self, id: WidgetId) -> Option<(u32, u32)> {
        let rel = id.get().checked_sub(self.base_id())?;
        (rel < ID_SPAN).then_some((rel / KEY_SLOTS, rel % KEY_SLOTS))
    }

    /// `true` if the control `key` is part of the current composition.
    fn shows(&self, key: u32) -> bool {
        self.compose_box().is_some_and(|b| b.has_key(key))
    }

    /// `true` if `id` is the field (not the `☰`) of a numeric row.
    fn is_numeric_field(&self, id: WidgetId) -> bool {
        self.decode(id).is_some_and(|(key, slot)| {
            slot == 0 && self.numeric_keys().contains(&key) && self.shows(key)
        })
    }

    /// `true` if `id` is the `☰` button of a numeric row.
    fn is_formula_button(&self, id: WidgetId) -> bool {
        self.decode(id).is_some_and(|(key, slot)| {
            slot == 1 && self.numeric_keys().contains(&key) && self.shows(key)
        })
    }

    /// `true` if `id` is a selection field or the `☰` of a populated one.
    fn is_selection_field(&self, id: WidgetId) -> bool {
        self.decode(id).is_some_and(|(key, slot)| {
            slot < 2 && self.select_keys().contains(&key) && self.shows(key)
        })
    }

    /// `true` if `id` belongs to the universally shared Profile selection field. Profile
    /// selection is an ordered list, so it uses the Profile Selection box rather than the
    /// generic checkbox selection modal.
    fn is_profile_selection_field(&self, id: WidgetId) -> bool {
        self.decode(id)
            .is_some_and(|(key, slot)| slot < 2 && key == K_PROFILE && self.shows(key))
    }

    /// `(key, slot)` of `id` if it is a control currently shown by an open box.
    fn live_control(&self, id: WidgetId) -> Option<(u32, u32)> {
        if !self.showing() {
            return None;
        }
        let (key, slot) = self.decode(id)?;
        self.shows(key).then_some((key, slot))
    }
}
