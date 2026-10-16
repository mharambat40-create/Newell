// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! The Revolve Action Box (M2.22): deterministic, UI-only state composed into the exact ASCII
//! of `docs/ui_design/Action_Box_Revolve.txt` (`README_UI.md` §9–§10).
//!
//! Rows: the Create header, `Profile`, `Axis`, then either `■ Full Revolve` alone or the partial
//! branch (`□ Full Revolve ◀ Blind ▶` / `◀ Up to ▶`) with its `Angle` or `Target`, `Start Offset`,
//! `End Offset` (`Up to` only), `Symetric` and `2nd side`; `Merge with all` for `Add` and
//! `Remove`; the footer. An active `Symetric` replaces `2nd side` and becomes the last child
//! (`│ ╰╴■ Symetric`). No geometry, BrepKIT, real selection or formula evaluation is involved.

use crate::WidgetId;
use crate::action_box::ActionBox;
use crate::create_box::{
    ActionBoxUi, ActionState, K_ACTIONS, K_GEOMETRY, K_MERGE, K_MERGE_TARGETS, K_NESTED,
    K_NESTED_MODE, K_NESTED_TARGET, K_PROFILE, K_RESULT, offset_children, offset_head, push_footer,
    push_head, push_merge, push_nested_end,
};
use crate::extrude::{ExtrudeGeometryType, ExtrudeOffsetMode, ExtrudeResultMode, MockSelection};

const K_AXIS: u32 = 3;
const K_FULL: u32 = 4;
const K_FULL_MODE: u32 = 5;
const K_ANGLE: u32 = 6;
const K_TARGET: u32 = 7;
const K_START: u32 = 8;
const K_START_MODE: u32 = 9;
const K_START_ANGLE: u32 = 10;
const K_START_TARGET: u32 = 11;
const K_END: u32 = 12;
const K_END_MODE: u32 = 13;
const K_END_ANGLE: u32 = 14;
const K_END_TARGET: u32 = 15;
const K_SYMMETRIC: u32 = 16;
const K_SECOND: u32 = 17;
const K_SECOND_MODE: u32 = 18;
const K_SECOND_ANGLE: u32 = 19;
const K_SECOND_TARGET: u32 = 20;

const NUMERIC_KEYS: [u32; 5] = [
    K_ANGLE,
    K_START_ANGLE,
    K_END_ANGLE,
    K_SECOND_ANGLE,
    crate::create_box::K_NESTED_DIST,
];
const SELECT_KEYS: [u32; 8] = [
    K_PROFILE,
    K_AXIS,
    K_TARGET,
    K_START_TARGET,
    K_END_TARGET,
    K_SECOND_TARGET,
    K_NESTED_TARGET,
    K_MERGE_TARGETS,
];

/// Mock field texts (`[+|0.0 deg      ☰]`, `[0.0 deg      ☰]`).
const ANGLE: &str = "+|0.0 deg";
const ANGLE_UNSIGNED: &str = "0.0 deg";

/// The editable mock values of the Revolve box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevolveValues {
    pub geometry: ExtrudeGeometryType,
    pub result: ExtrudeResultMode,
    pub profile: MockSelection,
    pub axis: MockSelection,
    /// `■ Full Revolve` (the reference default) hides the partial branch.
    pub full: bool,
    /// `Blind | Up to` of the partial revolution.
    pub mode: ExtrudeOffsetMode,
    /// `Target` (`Up to`).
    pub target: MockSelection,
    pub start_offset: bool,
    pub start_mode: ExtrudeOffsetMode,
    pub start_target: MockSelection,
    /// `End Offset` (`Up to` only).
    pub end_offset: bool,
    pub end_mode: ExtrudeOffsetMode,
    pub end_target: MockSelection,
    pub symmetric: bool,
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

impl Default for RevolveValues {
    /// `Full Revolve` and `Merge with all` start checked, as in the reference.
    fn default() -> Self {
        Self {
            geometry: ExtrudeGeometryType::default(),
            result: ExtrudeResultMode::default(),
            profile: MockSelection::default(),
            axis: MockSelection::default(),
            full: true,
            mode: ExtrudeOffsetMode::default(),
            target: MockSelection::default(),
            start_offset: false,
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

/// UI state of the Revolve Action Box.
pub type RevolveUi = ActionState<RevolveValues>;

impl RevolveUi {
    fn partial(&self, b: &mut ActionBox) {
        let v = &self.values;
        let base = self.base();
        b.push(
            self.row()
                .text("│ ")
                .check(K_FULL, false, "Full Revolve", 14)
                .text("   ")
                .cycle(K_FULL_MODE, v.mode.label(), 9)
                .end(),
        );
        match v.mode {
            ExtrudeOffsetMode::Blind => b.push(
                self.row()
                    .text("│ │ Angle   ")
                    .numeric(K_ANGLE, ANGLE)
                    .end(),
            ),
            ExtrudeOffsetMode::UpTo => b.push(
                self.row()
                    .text("│ │ Target    ")
                    .select(K_TARGET, v.target.count())
                    .end(),
            ),
        }
        if v.start_offset {
            offset_head(
                b,
                base,
                "│ │ ",
                (K_START, K_START_MODE),
                ("Start Offset", 1),
                v.start_mode,
            );
            offset_children(
                b,
                base,
                "│ │ ╰╴",
                (K_START_ANGLE, K_START_TARGET),
                v.start_mode,
                v.start_target,
                ("Angle ", ANGLE),
            );
        } else {
            b.push(
                self.row()
                    .text("│ │ ")
                    .check_fill(K_START, false, "Start Offset")
                    .end(),
            );
        }
        if v.mode == ExtrudeOffsetMode::UpTo {
            if v.end_offset {
                offset_head(
                    b,
                    base,
                    "│ │ ",
                    (K_END, K_END_MODE),
                    ("End Offset", 3),
                    v.end_mode,
                );
                offset_children(
                    b,
                    base,
                    "│ │ ╰╴",
                    (K_END_ANGLE, K_END_TARGET),
                    v.end_mode,
                    v.end_target,
                    ("Angle   ", ANGLE_UNSIGNED),
                );
            } else {
                b.push(
                    self.row()
                        .text("│ │ ")
                        .check_fill(K_END, false, "End Offset")
                        .end(),
                );
            }
        }
        if v.symmetric {
            b.push(
                self.row()
                    .text("│ ╰╴")
                    .check_fill(K_SYMMETRIC, true, "Symetric")
                    .end(),
            );
            return;
        }
        b.push(
            self.row()
                .text("│ │ ")
                .check_fill(K_SYMMETRIC, false, "Symetric")
                .end(),
        );
        self.second_side(b);
    }

    /// `2nd side`, the last child: collapsed, `Blind` (`Angle`) or `Up to` (`Target` and the
    /// nested `End Offset`).
    fn second_side(&self, b: &mut ActionBox) {
        let v = &self.values;
        if !v.second_side {
            b.push(
                self.row()
                    .text("│ ╰╴")
                    .check_fill(K_SECOND, false, "2nd side")
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
                    .text("│   ╰╴Angle ")
                    .numeric(K_SECOND_ANGLE, ANGLE)
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
                    self.base(),
                    v.nested_end,
                    v.nested_mode,
                    v.nested_target,
                    ("Angle ", ANGLE_UNSIGNED),
                );
            }
        }
    }
}

impl ActionBoxUi for RevolveUi {
    fn base_id(&self) -> u32 {
        self.base()
    }

    fn compose_box(&self) -> Option<ActionBox> {
        if !self.is_open() {
            return None;
        }
        let v = &self.values;
        let mut b = ActionBox::default();
        push_head(&mut b, self.base(), "Revolve", v.geometry, v.result);
        b.push(
            self.row()
                .text("│ Profile     ")
                .select(K_PROFILE, v.profile.count())
                .end(),
        );
        b.push(
            self.row()
                .text("│ Axis        ")
                .select(K_AXIS, v.axis.count())
                .end(),
        );
        if v.full {
            b.push(
                self.row()
                    .text("│ ")
                    .check_fill(K_FULL, true, "Full Revolve")
                    .end(),
            );
        } else {
            self.partial(&mut b);
        }
        if v.result != ExtrudeResultMode::New {
            push_merge(&mut b, self.base(), v.merge_with_all, v.merge_targets);
        }
        push_footer(&mut b, self.base());
        Some(b)
    }

    fn activate_id(&mut self, id: WidgetId) -> bool {
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
            (K_FULL, 0) => v.full = !v.full,
            (K_FULL_MODE, 0 | 1) => v.mode = v.mode.flipped(),
            (K_START_MODE, 0 | 1) => v.start_mode = v.start_mode.flipped(),
            (K_END_MODE, 0 | 1) => v.end_mode = v.end_mode.flipped(),
            (K_SECOND_MODE, 0 | 1) => v.second_mode = v.second_mode.flipped(),
            (K_NESTED_MODE, 0 | 1) => v.nested_mode = v.nested_mode.flipped(),
            (K_PROFILE, 0) => v.profile = v.profile.next(),
            (K_AXIS, 0) => v.axis = v.axis.next(),
            (K_TARGET, 0) => v.target = v.target.next(),
            (K_START_TARGET, 0) => v.start_target = v.start_target.next(),
            (K_END_TARGET, 0) => v.end_target = v.end_target.next(),
            (K_SECOND_TARGET, 0) => v.second_target = v.second_target.next(),
            (K_NESTED_TARGET, 0) => v.nested_target = v.nested_target.next(),
            (K_MERGE_TARGETS, 0) => v.merge_targets = v.merge_targets.next(),
            (K_START, 0) => v.start_offset = !v.start_offset,
            (K_END, 0) => v.end_offset = !v.end_offset,
            (K_SYMMETRIC, 0) => v.symmetric = !v.symmetric,
            (K_SECOND, 0) => v.second_side = !v.second_side,
            (K_NESTED, 0) => v.nested_end = !v.nested_end,
            (K_MERGE, 0) => v.merge_with_all = !v.merge_with_all,
            (K_ACTIONS, 0) => self.confirm(),
            (K_ACTIONS, 1) => self.cancel(),
            _ => {}
        }
        before != (self.values, self.status)
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
