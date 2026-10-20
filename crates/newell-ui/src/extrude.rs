// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! The Extrude Action Box (M2.21): deterministic, UI-only state composed into the exact ASCII
//! of `docs/ui_design/Action_Box_Extrude.txt`.
//!
//! [`ExtrudeUi`] is plain data. [`ExtrudeUi::compose`] rebuilds the [`ActionBox`] from it, row
//! by row, following `README_UI.md` §2.4–§2.6, §7–§8 and §33: the complete base box is the
//! authority, a variant replaces only its own line or branch, and no documented sibling
//! control is ever deleted. [`ExtrudeUi::activate`] applies a pointer or keyboard activation,
//! identified by [`WidgetId`], to the same state. Nothing here touches geometry, BrepKIT, the
//! model, real selection or formula evaluation.
//!
//! # Rows
//!
//! Frame and header (`Solid | Surface`, `New | Add | Remove`, six border topologies), then
//!
//! 1. `Profile` and `Extend ◀ … ▶`, always present;
//! 2. the `Extend` branch: `Blind` = `Depth`, `Along`, `Starting Offset`, `End Offset`;
//!    `Up to` = `Selection`, `Offset`; `Through all` = `Invert`, `Start Offset`;
//! 3. `Symetric`, then `Second end position` / `2nd side` while `Symetric` is off;
//! 4. `Merge with all` (and `Targets` while it is off) for `Add` and `Remove`;
//! 5. the separator and `OK | Cancel`.
//!
//! Offsets that disclose children use the documented expanded forms; an expanded offset that is
//! the last child of its branch uses the prefix transformation of §2.6. The open reference
//! points of §2.6 (label drift, no `End Offset` under `Up to` / `Through all`, no expanded
//! `Offset`) are reproduced as written.
//!
//! # Widget ids
//!
//! Every control has a fixed key (`K_*`); its id is `base + key * KEY_SLOTS + slot`, so ids never
//! depend on which rows are currently shown.

#[cfg(test)]
use crate::action_box::KEY_SLOTS;
use crate::action_box::{ActionBox, Row};
use crate::create_box::{
    ActionBoxUi, K_NESTED, K_NESTED_MODE, K_NESTED_TARGET, offset_children, offset_head,
    push_footer, push_head, push_merge, push_nested_end,
};
use crate::{WidgetId, cycle_next, cycle_prev};

pub(crate) use crate::create_box::{K_ACTIONS, K_GEOMETRY, K_PROFILE, K_RESULT};
use crate::create_box::{K_MERGE, K_MERGE_TARGETS, K_NESTED_DIST};

pub(crate) const K_EXTEND: u32 = 3;
const K_DEPTH: u32 = 4;
const K_ALONG: u32 = 5;
const K_SELECTION: u32 = 6;
const K_OFFSET: u32 = 7;
const K_INVERT: u32 = 8;
const K_START: u32 = 9;
const K_START_MODE: u32 = 10;
const K_START_DIST: u32 = 11;
const K_START_TARGET: u32 = 12;
const K_END: u32 = 13;
const K_END_MODE: u32 = 14;
const K_END_DIST: u32 = 15;
const K_END_TARGET: u32 = 16;
const K_SYMMETRIC: u32 = 17;
const K_SECOND: u32 = 18;
const K_SECOND_MODE: u32 = 19;
const K_SECOND_DEPTH: u32 = 20;
const K_SECOND_TARGET: u32 = 21;

/// Keys of the numeric rows (field slot 0, formula `☰` slot 1).
const NUMERIC_KEYS: [u32; 5] = [
    K_DEPTH,
    K_START_DIST,
    K_END_DIST,
    K_SECOND_DEPTH,
    K_NESTED_DIST,
];
/// Keys of the selection fields (slot 0 the field, slot 1 the `☰` of a populated one).
const SELECT_KEYS: [u32; 8] = [
    K_PROFILE,
    K_ALONG,
    K_SELECTION,
    K_START_TARGET,
    K_END_TARGET,
    K_SECOND_TARGET,
    K_NESTED_TARGET,
    K_MERGE_TARGETS,
];

/// Mock field text of the numeric rows; numeric fields are UI-only (M2.23 edits them).
const MOCK_DIST: &str = "+|0.0 mm";
const MOCK_DIST_UNSIGNED: &str = "0.0 mm";
/// Element count shown for [`MockSelection::Many`].
const MOCK_MANY: u32 = 3;

/// `Solid | Surface`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtrudeGeometryType {
    #[default]
    Solid,
    Surface,
}

/// `New | Add | Remove`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtrudeResultMode {
    #[default]
    New,
    Add,
    Remove,
}

/// `Blind | Up to | Through all` (cyclic, in this order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtrudeTermination {
    #[default]
    Blind,
    UpTo,
    ThroughAll,
}

impl ExtrudeTermination {
    const fn label(self) -> &'static str {
        match self {
            Self::Blind => "Blind",
            Self::UpTo => "Up to",
            Self::ThroughAll => "Through all",
        }
    }
}

/// End condition of an offset or of the second side: `Blind | Up to` (cyclic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtrudeOffsetMode {
    #[default]
    Blind,
    UpTo,
}

impl ExtrudeOffsetMode {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Blind => "Blind",
            Self::UpTo => "Up to",
        }
    }

    /// Two values: either arrow flips.
    pub(crate) const fn flipped(self) -> Self {
        match self {
            Self::Blind => Self::UpTo,
            Self::UpTo => Self::Blind,
        }
    }
}

/// Deterministic mock selection: a count only, never element names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MockSelection {
    #[default]
    None,
    One,
    Many,
}

impl MockSelection {
    /// Number of mock elements.
    #[must_use]
    pub const fn count(self) -> u32 {
        match self {
            Self::None => 0,
            Self::One => 1,
            Self::Many => MOCK_MANY,
        }
    }

    /// Field text: `Select…`, `1 Element`, `3 Elements`.
    #[must_use]
    pub fn text(self) -> String {
        match self {
            Self::None => "Select…".to_owned(),
            Self::One => "1 Element".to_owned(),
            Self::Many => format!("{} Elements", self.count()),
        }
    }

    /// Mock edit: none -> `1 Element` -> `N Elements` -> none.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::None => Self::One,
            Self::One => Self::Many,
            Self::Many => Self::None,
        }
    }
}

/// Whether the command is still being edited. OK and Cancel only move this (and, for
/// Cancel, restore the opening values).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtrudeStatus {
    #[default]
    Editing,
    Confirmed,
    Cancelled,
}

/// The editable mock values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtrudeValues {
    pub geometry: ExtrudeGeometryType,
    pub result: ExtrudeResultMode,
    pub profile: MockSelection,
    pub termination: ExtrudeTermination,
    /// `Along` (Blind).
    pub along: MockSelection,
    /// `Selection` (Up to).
    pub selection: MockSelection,
    /// `Offset` (Up to).
    pub offset: bool,
    /// `Invert` (Through all).
    pub invert: bool,
    /// `Starting Offset` (Blind) / `Start Offset` (Through all): one control, two labels.
    pub starting_offset: bool,
    pub start_mode: ExtrudeOffsetMode,
    pub start_target: MockSelection,
    /// `End Offset` (Blind).
    pub end_offset: bool,
    pub end_mode: ExtrudeOffsetMode,
    pub end_target: MockSelection,
    pub symmetric: bool,
    /// `Second end position` while off, `2nd side` while on.
    pub second_side: bool,
    pub second_mode: ExtrudeOffsetMode,
    pub second_target: MockSelection,
    /// `End Offset` nested in `2nd side = Up to`.
    pub nested_end: bool,
    pub nested_mode: ExtrudeOffsetMode,
    pub nested_target: MockSelection,
    pub merge_with_all: bool,
    pub merge_targets: MockSelection,
}

impl Default for ExtrudeValues {
    /// `Merge with all` starts checked, as in the reference.
    fn default() -> Self {
        Self {
            geometry: ExtrudeGeometryType::default(),
            result: ExtrudeResultMode::default(),
            profile: MockSelection::default(),
            termination: ExtrudeTermination::default(),
            along: MockSelection::default(),
            selection: MockSelection::default(),
            offset: false,
            invert: false,
            starting_offset: false,
            start_mode: ExtrudeOffsetMode::default(),
            start_target: MockSelection::default(),
            end_offset: false,
            end_mode: ExtrudeOffsetMode::default(),
            end_target: MockSelection::default(),
            symmetric: false,
            second_side: false,
            second_mode: ExtrudeOffsetMode::default(),
            second_target: MockSelection::default(),
            nested_end: false,
            nested_mode: ExtrudeOffsetMode::default(),
            nested_target: MockSelection::default(),
            merge_with_all: true,
            merge_targets: MockSelection::default(),
        }
    }
}

/// UI state of the Extrude Action Box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtrudeUi {
    base_id: u32,
    pub values: ExtrudeValues,
    opened: ExtrudeValues,
    pub status: ExtrudeStatus,
}

impl ExtrudeUi {
    /// Opens the box with default values; `base_id` is its first widget id.
    #[must_use]
    pub fn new(base_id: u32) -> Self {
        let values = ExtrudeValues::default();
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

    fn row(&self) -> Row {
        Row::new(self.base_id)
    }

    /// Stable id of the primary Depth field for the model adapter.
    #[must_use]
    pub fn depth_field_id(&self) -> WidgetId {
        WidgetId::new(self.base_id + K_DEPTH * crate::action_box::KEY_SLOTS)
    }

    /// Composes the box for the current values, or `None` once OK / Cancel closed it.
    pub(crate) fn compose(&self) -> Option<ActionBox> {
        if !self.is_open() {
            return None;
        }
        let v = &self.values;
        let mut b = ActionBox::default();
        push_head(&mut b, self.base_id, "Extrude", v.geometry, v.result);
        b.push(
            self.row()
                .text("│ Profile     ")
                .select(K_PROFILE, v.profile.count())
                .end(),
        );
        b.push(
            self.row()
                .text("│ Extend   ")
                .cycle(K_EXTEND, v.termination.label(), 17)
                .end(),
        );
        match v.termination {
            ExtrudeTermination::Blind => self.blind_branch(&mut b),
            ExtrudeTermination::UpTo => self.up_to_branch(&mut b),
            ExtrudeTermination::ThroughAll => self.through_all_branch(&mut b),
        }
        b.push(
            self.row()
                .text("│ ")
                .check_fill(K_SYMMETRIC, v.symmetric, "Symetric")
                .end(),
        );
        if !v.symmetric {
            self.second_side(&mut b);
        }
        if v.result != ExtrudeResultMode::New {
            push_merge(&mut b, self.base_id, v.merge_with_all, v.merge_targets);
        }
        push_footer(&mut b, self.base_id);
        Some(b)
    }

    fn blind_branch(&self, b: &mut ActionBox) {
        let v = &self.values;
        b.push(
            self.row()
                .text("│ │ Depth   ")
                .numeric(K_DEPTH, MOCK_DIST)
                .end(),
        );
        b.push(
            self.row()
                .text("│ │ Along     ")
                .select(K_ALONG, v.along.count())
                .end(),
        );
        if v.starting_offset {
            offset_head(
                b,
                self.base_id,
                "│ │ ",
                (K_START, K_START_MODE),
                ("Start Offset", 1),
                v.start_mode,
            );
            offset_children(
                b,
                self.base_id,
                "│ │ ╰╴",
                (K_START_DIST, K_START_TARGET),
                v.start_mode,
                v.start_target,
                ("Dist  ", MOCK_DIST),
            );
        } else {
            b.push(
                self.row()
                    .text("│ │ ")
                    .check_fill(K_START, false, "Starting Offset")
                    .end(),
            );
        }
        if v.end_offset {
            offset_head(
                b,
                self.base_id,
                "│ ╰╴",
                (K_END, K_END_MODE),
                ("End Offset", 3),
                v.end_mode,
            );
            offset_children(
                b,
                self.base_id,
                "│   ╰╴",
                (K_END_DIST, K_END_TARGET),
                v.end_mode,
                v.end_target,
                ("Dist  ", MOCK_DIST),
            );
        } else {
            b.push(
                self.row()
                    .text("│ ╰╴")
                    .check_fill(K_END, false, "End Offset")
                    .end(),
            );
        }
    }

    fn up_to_branch(&self, b: &mut ActionBox) {
        let v = &self.values;
        b.push(
            self.row()
                .text("│ │ Selection ")
                .select(K_SELECTION, v.selection.count())
                .end(),
        );
        b.push(
            self.row()
                .text("│ ╰╴")
                .check_fill(K_OFFSET, v.offset, "Offset")
                .end(),
        );
    }

    fn through_all_branch(&self, b: &mut ActionBox) {
        let v = &self.values;
        b.push(
            self.row()
                .text("│ │ ")
                .check_fill(K_INVERT, v.invert, "Invert")
                .end(),
        );
        if v.starting_offset {
            offset_head(
                b,
                self.base_id,
                "│ ╰╴",
                (K_START, K_START_MODE),
                ("Start Offset", 1),
                v.start_mode,
            );
            offset_children(
                b,
                self.base_id,
                "│   ╰╴",
                (K_START_DIST, K_START_TARGET),
                v.start_mode,
                v.start_target,
                ("Dist  ", MOCK_DIST),
            );
        } else {
            b.push(
                self.row()
                    .text("│ ╰╴")
                    .check_fill(K_START, false, "Start Offset")
                    .end(),
            );
        }
    }

    /// `Second end position` while collapsed (base box), the `2nd side` branch while expanded.
    fn second_side(&self, b: &mut ActionBox) {
        let v = &self.values;
        if !v.second_side {
            b.push(
                self.row()
                    .text("│ ")
                    .check_fill(K_SECOND, false, "Second end position")
                    .end(),
            );
            return;
        }
        b.push(
            self.row()
                .text("│ ╰╴")
                .check(K_SECOND, true, "2nd side", 10)
                .text("     ")
                .cycle(K_SECOND_MODE, v.second_mode.label(), 9)
                .end(),
        );
        match v.second_mode {
            ExtrudeOffsetMode::Blind => b.push(
                self.row()
                    .text("│   ╰╴Depth ")
                    .numeric(K_SECOND_DEPTH, MOCK_DIST)
                    .end(),
            ),
            ExtrudeOffsetMode::UpTo => {
                b.push(
                    self.row()
                        .text("│   │ Target  ")
                        .select(K_SECOND_TARGET, v.second_target.count())
                        .end(),
                );
                push_nested_end(
                    b,
                    self.base_id,
                    v.nested_end,
                    v.nested_mode,
                    v.nested_target,
                    ("Dist  ", MOCK_DIST_UNSIGNED),
                );
            }
        }
    }

    /// Applies activation of `id` to the mock state. Returns `true` if the state changed.
    ///
    /// Ids outside this box, controls that are not currently shown, the mock numeric fields
    /// and their formula buttons (M2.23) change nothing. A closed box ignores everything.
    pub fn activate(&mut self, id: WidgetId) -> bool {
        // Only controls present in the composed box are activatable.
        let Some((key, slot)) = self.live_control(id) else {
            return false;
        };
        let before = (self.values, self.status);
        let v = &mut self.values;
        match (key, slot) {
            (K_GEOMETRY, 0) => v.geometry = ExtrudeGeometryType::Solid,
            (K_GEOMETRY, 1) => v.geometry = ExtrudeGeometryType::Surface,
            (K_RESULT, 0) => v.result = ExtrudeResultMode::New,
            (K_RESULT, 1) => v.result = ExtrudeResultMode::Add,
            (K_RESULT, 2) => v.result = ExtrudeResultMode::Remove,
            (K_EXTEND, 0) => v.termination = termination_at(cycle_prev(v.termination as usize, 3)),
            (K_EXTEND, 1) => v.termination = termination_at(cycle_next(v.termination as usize, 3)),
            (K_START_MODE, 0 | 1) => v.start_mode = v.start_mode.flipped(),
            (K_END_MODE, 0 | 1) => v.end_mode = v.end_mode.flipped(),
            (K_SECOND_MODE, 0 | 1) => v.second_mode = v.second_mode.flipped(),
            (K_NESTED_MODE, 0 | 1) => v.nested_mode = v.nested_mode.flipped(),
            (K_PROFILE, 0) => v.profile = v.profile.next(),
            (K_ALONG, 0) => v.along = v.along.next(),
            (K_SELECTION, 0) => v.selection = v.selection.next(),
            (K_START_TARGET, 0) => v.start_target = v.start_target.next(),
            (K_END_TARGET, 0) => v.end_target = v.end_target.next(),
            (K_SECOND_TARGET, 0) => v.second_target = v.second_target.next(),
            (K_NESTED_TARGET, 0) => v.nested_target = v.nested_target.next(),
            (K_MERGE_TARGETS, 0) => v.merge_targets = v.merge_targets.next(),
            (K_OFFSET, 0) => v.offset = !v.offset,
            (K_INVERT, 0) => v.invert = !v.invert,
            (K_START, 0) => v.starting_offset = !v.starting_offset,
            (K_END, 0) => v.end_offset = !v.end_offset,
            (K_SYMMETRIC, 0) => v.symmetric = !v.symmetric,
            (K_SECOND, 0) => v.second_side = !v.second_side,
            (K_NESTED, 0) => v.nested_end = !v.nested_end,
            (K_MERGE, 0) => v.merge_with_all = !v.merge_with_all,
            (K_ACTIONS, 0) => {
                self.opened = self.values;
                self.status = ExtrudeStatus::Confirmed;
            }
            (K_ACTIONS, 1) => {
                self.values = self.opened;
                self.status = ExtrudeStatus::Cancelled;
            }
            _ => {}
        }
        before != (self.values, self.status)
    }

    /// Id of control `slot` of the control `key`, as drawn by the composed box.
    #[cfg(test)]
    pub(crate) fn id(&self, key: u32, slot: u32) -> WidgetId {
        WidgetId::new(self.base_id + key * KEY_SLOTS + slot)
    }
}

impl ActionBoxUi for ExtrudeUi {
    fn base_id(&self) -> u32 {
        self.base_id
    }

    fn compose_box(&self) -> Option<ActionBox> {
        self.compose()
    }

    fn activate_id(&mut self, id: WidgetId) -> bool {
        self.activate(id)
    }

    fn showing(&self) -> bool {
        self.is_open()
    }

    fn restart(&mut self) {
        self.reopen();
    }

    fn numeric_keys(&self) -> &'static [u32] {
        &NUMERIC_KEYS
    }

    fn select_keys(&self) -> &'static [u32] {
        &SELECT_KEYS
    }
}

fn termination_at(index: usize) -> ExtrudeTermination {
    match index {
        1 => ExtrudeTermination::UpTo,
        2 => ExtrudeTermination::ThroughAll,
        _ => ExtrudeTermination::Blind,
    }
}

#[cfg(test)]
#[path = "extrude_tests.rs"]
mod tests;
