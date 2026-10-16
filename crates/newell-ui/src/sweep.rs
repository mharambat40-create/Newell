// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! The Sweep Action Box (M2.22): deterministic, UI-only state composed into the exact ASCII of
//! `docs/ui_design/Action_Box_Sweep.txt` (`README_UI.md` §13–§14).
//!
//! Rows: the Create header, `Profile`, `Path`, `Full path`, `Keep orientation` (with its `Twist`
//! child, whose angle shares the row), `Scale` (whose value shares the row), `Merge with all`
//! for `Add` and `Remove`, the footer. No geometry, BrepKIT, real selection or formula
//! evaluation is involved.

use crate::WidgetId;
use crate::action_box::ActionBox;
use crate::create_box::{
    ActionBoxUi, ActionState, K_ACTIONS, K_GEOMETRY, K_MERGE, K_MERGE_TARGETS, K_PROFILE, K_RESULT,
    push_footer, push_head, push_merge,
};
use crate::extrude::{ExtrudeGeometryType, ExtrudeResultMode, MockSelection};

const K_PATH: u32 = 3;
const K_FULL_PATH: u32 = 4;
const K_KEEP: u32 = 5;
const K_TWIST: u32 = 6;
const K_TWIST_ANGLE: u32 = 7;
const K_SCALE: u32 = 8;
const K_SCALE_VALUE: u32 = 9;

const NUMERIC_KEYS: [u32; 2] = [K_TWIST_ANGLE, K_SCALE_VALUE];
const SELECT_KEYS: [u32; 3] = [K_PROFILE, K_PATH, K_MERGE_TARGETS];

/// Mock field texts: `[+|0.0 deg      ☰]` and the unitless `[0.0          ☰]`.
const TWIST: &str = "+|0.0 deg";
const SCALE: &str = "0.0";

/// The editable mock values of the Sweep box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepValues {
    pub geometry: ExtrudeGeometryType,
    pub result: ExtrudeResultMode,
    pub profile: MockSelection,
    pub path: MockSelection,
    pub full_path: bool,
    pub keep_orientation: bool,
    /// `Twist` (child of `Keep orientation`).
    pub twist: bool,
    pub scale: bool,
    pub merge_with_all: bool,
    pub merge_targets: MockSelection,
}

impl Default for SweepValues {
    /// `Merge with all` starts checked, as in the reference.
    fn default() -> Self {
        Self {
            geometry: ExtrudeGeometryType::default(),
            result: ExtrudeResultMode::default(),
            profile: MockSelection::default(),
            path: MockSelection::default(),
            full_path: false,
            keep_orientation: false,
            twist: false,
            scale: false,
            merge_with_all: true,
            merge_targets: MockSelection::default(),
        }
    }
}

/// UI state of the Sweep Action Box.
pub type SweepUi = ActionState<SweepValues>;

impl ActionBoxUi for SweepUi {
    fn base_id(&self) -> u32 {
        self.base()
    }

    fn compose_box(&self) -> Option<ActionBox> {
        if !self.is_open() {
            return None;
        }
        let v = &self.values;
        let mut b = ActionBox::default();
        push_head(&mut b, self.base(), "Sweep", v.geometry, v.result);
        b.push(
            self.row()
                .text("│ Profile     ")
                .select(K_PROFILE, v.profile.count())
                .end(),
        );
        b.push(
            self.row()
                .text("│ Path        ")
                .select(K_PATH, v.path.count())
                .end(),
        );
        b.push(
            self.row()
                .text("│ ")
                .check_fill(K_FULL_PATH, v.full_path, "Full path")
                .end(),
        );
        b.push(
            self.row()
                .text("│ ")
                .check_fill(K_KEEP, v.keep_orientation, "Keep orientation")
                .end(),
        );
        if v.keep_orientation {
            if v.twist {
                b.push(
                    self.row()
                        .text("│ ╰╴")
                        .check(K_TWIST, true, "Twist", 7)
                        .text(" ")
                        .numeric(K_TWIST_ANGLE, TWIST)
                        .end(),
                );
            } else {
                b.push(
                    self.row()
                        .text("│ ╰╴")
                        .check_fill(K_TWIST, false, "Twist")
                        .end(),
                );
            }
        }
        if v.scale {
            b.push(
                self.row()
                    .text("│ ")
                    .check(K_SCALE, true, "Scale", 7)
                    .text("     ")
                    .numeric(K_SCALE_VALUE, SCALE)
                    .end(),
            );
        } else {
            b.push(
                self.row()
                    .text("│ ")
                    .check_fill(K_SCALE, false, "Scale")
                    .end(),
            );
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
            (K_PROFILE, 0) => v.profile = v.profile.next(),
            (K_PATH, 0) => v.path = v.path.next(),
            (K_FULL_PATH, 0) => v.full_path = !v.full_path,
            (K_KEEP, 0) => v.keep_orientation = !v.keep_orientation,
            (K_TWIST, 0) => v.twist = !v.twist,
            (K_SCALE, 0) => v.scale = !v.scale,
            (K_MERGE_TARGETS, 0) => v.merge_targets = v.merge_targets.next(),
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
