// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! The Loft Action Box (M2.22): deterministic, UI-only state composed into the exact ASCII of
//! `docs/ui_design/Action_Box_Loft.txt` (`README_UI.md` §11–§12).
//!
//! Rows: the Create header, `Profile`, `Start type` and `End type` (each `◀ Normal | Tengent |
//! Match ▶` with a `Magnitude` or `Entity` child), `Guide`, `Path`, `Connection` (with `Start`
//! and `End`), `Merge with all` for `Add` and `Remove`, the footer. The reference spells the
//! second value `Tengent`; it is reproduced as written. No geometry, BrepKIT, real selection
//! or formula evaluation is involved.

use crate::action_box::ActionBox;
use crate::create_box::{
    ActionBoxUi, ActionState, K_ACTIONS, K_GEOMETRY, K_MERGE, K_MERGE_TARGETS, K_PROFILE, K_RESULT,
    push_footer, push_head, push_merge,
};
use crate::extrude::{ExtrudeGeometryType, ExtrudeResultMode, MockSelection};
use crate::{WidgetId, cycle_next, cycle_prev};

const K_START: u32 = 3;
const K_START_KIND: u32 = 4;
const K_START_MAGNITUDE: u32 = 5;
const K_START_ENTITY: u32 = 6;
const K_END: u32 = 7;
const K_END_KIND: u32 = 8;
const K_END_MAGNITUDE: u32 = 9;
const K_END_ENTITY: u32 = 10;
const K_GUIDE: u32 = 11;
const K_GUIDE_SELECTION: u32 = 12;
const K_PATH: u32 = 13;
const K_PATH_SELECTION: u32 = 14;
const K_CONNECTION: u32 = 15;
const K_CONNECTION_START: u32 = 16;
const K_CONNECTION_END: u32 = 17;

const SELECT_KEYS: [u32; 8] = [
    K_PROFILE,
    K_START_ENTITY,
    K_END_ENTITY,
    K_GUIDE_SELECTION,
    K_PATH_SELECTION,
    K_CONNECTION_START,
    K_CONNECTION_END,
    K_MERGE_TARGETS,
];

/// `Normal | Tengent | Match` (cyclic, in this order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoftEndType {
    #[default]
    Normal,
    Tangent,
    Match,
}

impl LoftEndType {
    /// The reference spells the tangent condition `Tengent`.
    const fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Tangent => "Tengent",
            Self::Match => "Match",
        }
    }

    const fn at(index: usize) -> Self {
        match index {
            1 => Self::Tangent,
            2 => Self::Match,
            _ => Self::Normal,
        }
    }

    const fn prev(self) -> Self {
        Self::at(cycle_prev(self as usize, 3))
    }

    const fn next(self) -> Self {
        Self::at(cycle_next(self as usize, 3))
    }
}

/// The editable mock values of the Loft box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoftValues {
    pub geometry: ExtrudeGeometryType,
    pub result: ExtrudeResultMode,
    pub profile: MockSelection,
    pub start_type: bool,
    pub start_kind: LoftEndType,
    pub start_entity: MockSelection,
    pub end_type: bool,
    pub end_kind: LoftEndType,
    pub end_entity: MockSelection,
    pub guide: bool,
    pub guide_selection: MockSelection,
    pub path: bool,
    pub path_selection: MockSelection,
    pub connection: bool,
    pub connection_start: MockSelection,
    pub connection_end: MockSelection,
    pub merge_with_all: bool,
    pub merge_targets: MockSelection,
}

impl Default for LoftValues {
    /// `Merge with all` starts checked, as in the reference.
    fn default() -> Self {
        Self {
            geometry: ExtrudeGeometryType::default(),
            result: ExtrudeResultMode::default(),
            profile: MockSelection::default(),
            start_type: false,
            start_kind: LoftEndType::default(),
            start_entity: MockSelection::default(),
            end_type: false,
            end_kind: LoftEndType::default(),
            end_entity: MockSelection::default(),
            guide: false,
            guide_selection: MockSelection::default(),
            path: false,
            path_selection: MockSelection::default(),
            connection: false,
            connection_start: MockSelection::default(),
            connection_end: MockSelection::default(),
            merge_with_all: true,
            merge_targets: MockSelection::default(),
        }
    }
}

/// UI state of the Loft Action Box.
pub type LoftUi = ActionState<LoftValues>;

impl LoftUi {
    /// `Start type` / `End type`: collapsed, or expanded with its `◀ kind ▶` and the child row
    /// (`Magnitude` for `Normal` / `Tengent`, `Entity` for `Match`).
    #[allow(clippy::too_many_arguments)]
    fn end_type(
        &self,
        b: &mut ActionBox,
        keys: (u32, u32, u32, u32),
        on: bool,
        kind: LoftEndType,
        entity: MockSelection,
        (label, gap): (&str, &str),
    ) {
        let (key, kind_key, magnitude_key, entity_key) = keys;
        if !on {
            b.push(self.row().contour("│ ").check_fill(key, false, label).end());
            return;
        }
        let width = u32::try_from(label.chars().count() + 2).unwrap_or(0);
        b.push(
            self.row()
                .contour("│ ")
                .check(key, true, label, width)
                .text(gap)
                .cycle(kind_key, kind.label(), 11)
                .end(),
        );
        if kind == LoftEndType::Match {
            b.push(
                self.row()
                    .contour("│")
                    .text(" ╰╴Entity    ")
                    .select(entity_key, entity.count())
                    .end(),
            );
        } else {
            b.push(
                self.row()
                    .contour("│")
                    .text(" ╰╴Magnitude ")
                    .numeric(
                        magnitude_key,
                        crate::numeric::field(crate::ActionOperation::Loft, magnitude_key),
                    )
                    .end(),
            );
        }
    }

    /// `□ label` or `■ label [Select…]` with the field starting at column 14.
    fn with_selection(
        &self,
        b: &mut ActionBox,
        (key, selection_key): (u32, u32),
        on: bool,
        label: &str,
        selection: MockSelection,
    ) {
        if !on {
            b.push(self.row().contour("│ ").check_fill(key, false, label).end());
            return;
        }
        let width = u32::try_from(label.chars().count() + 2).unwrap_or(0);
        let gap = " ".repeat(12usize.saturating_sub(width as usize));
        b.push(
            self.row()
                .contour("│ ")
                .check(key, true, label, width)
                .text(&gap)
                .select(
                    selection_key,
                    self.selection_count(selection_key, selection.count()),
                )
                .end(),
        );
    }
}

impl ActionBoxUi for LoftUi {
    fn base_id(&self) -> u32 {
        self.base()
    }

    fn compose_box(&self) -> Option<ActionBox> {
        if !self.is_open() {
            return None;
        }
        let v = &self.values;
        let mut b = ActionBox::default();
        push_head(&mut b, self.base(), "Loft", v.geometry, v.result);
        b.push(
            self.row()
                .contour("│")
                .text(" Profile     ")
                .select(
                    K_PROFILE,
                    self.selection_count(K_PROFILE, v.profile.count()),
                )
                .end(),
        );
        self.end_type(
            &mut b,
            (K_START, K_START_KIND, K_START_MAGNITUDE, K_START_ENTITY),
            v.start_type,
            v.start_kind,
            v.start_entity,
            ("Start type", "   "),
        );
        self.end_type(
            &mut b,
            (K_END, K_END_KIND, K_END_MAGNITUDE, K_END_ENTITY),
            v.end_type,
            v.end_kind,
            v.end_entity,
            ("End type", "     "),
        );
        self.with_selection(
            &mut b,
            (K_GUIDE, K_GUIDE_SELECTION),
            v.guide,
            "Guide",
            v.guide_selection,
        );
        self.with_selection(
            &mut b,
            (K_PATH, K_PATH_SELECTION),
            v.path,
            "Path",
            v.path_selection,
        );
        b.push(
            self.row()
                .contour("│ ")
                .check_fill(K_CONNECTION, v.connection, "Connection")
                .end(),
        );
        if v.connection {
            b.push(
                self.row()
                    .contour("│")
                    .text(" │ Start     ")
                    .select(
                        K_CONNECTION_START,
                        self.selection_count(K_CONNECTION_START, v.connection_start.count()),
                    )
                    .end(),
            );
            b.push(
                self.row()
                    .contour("│")
                    .text(" ╰╴End       ")
                    .select(
                        K_CONNECTION_END,
                        self.selection_count(K_CONNECTION_END, v.connection_end.count()),
                    )
                    .end(),
            );
        }
        if v.result != ExtrudeResultMode::New {
            push_merge(
                &mut b,
                self.base(),
                v.merge_with_all,
                self.selection_count(K_MERGE_TARGETS, v.merge_targets.count()),
            );
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
            (K_START, 0) => v.start_type = !v.start_type,
            (K_START_KIND, 0) => v.start_kind = v.start_kind.prev(),
            (K_START_KIND, 1) => v.start_kind = v.start_kind.next(),
            (K_END, 0) => v.end_type = !v.end_type,
            (K_END_KIND, 0) => v.end_kind = v.end_kind.prev(),
            (K_END_KIND, 1) => v.end_kind = v.end_kind.next(),
            (K_GUIDE, 0) => v.guide = !v.guide,
            (K_PATH, 0) => v.path = !v.path,
            (K_CONNECTION, 0) => v.connection = !v.connection,
            (K_PROFILE, 0) => v.profile = v.profile.next(),
            (K_START_ENTITY, 0) => v.start_entity = v.start_entity.next(),
            (K_END_ENTITY, 0) => v.end_entity = v.end_entity.next(),
            (K_GUIDE_SELECTION, 0) => v.guide_selection = v.guide_selection.next(),
            (K_PATH_SELECTION, 0) => v.path_selection = v.path_selection.next(),
            (K_CONNECTION_START, 0) => v.connection_start = v.connection_start.next(),
            (K_CONNECTION_END, 0) => v.connection_end = v.connection_end.next(),
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

    fn numeric_operation(&self) -> crate::ActionOperation {
        crate::ActionOperation::Loft
    }

    fn selection_descriptor(&self, key: u32) -> Option<crate::SelectionFieldDescriptor> {
        use crate::SelectionFieldRole::*;
        if !SELECT_KEYS.contains(&key) {
            return None;
        }
        let role = match key {
            K_MERGE_TARGETS => MergeScope,
            K_GUIDE_SELECTION | K_PATH_SELECTION => Path,
            _ => EntityReference,
        };
        Some(crate::SelectionFieldDescriptor {
            field: crate::SelectionFieldKey {
                operation: crate::ActionOperation::Loft,
                key,
            },
            role,
            ordered: false,
            max_count: if matches!(role, MergeScope | Bodies) {
                None
            } else {
                Some(1)
            },
        })
    }
    fn set_selection_counts(&mut self, counts: &std::collections::BTreeMap<u32, usize>) {
        self.selection_counts = Some(counts.clone());
    }
    fn select_keys(&self) -> &'static [u32] {
        &SELECT_KEYS
    }
}
