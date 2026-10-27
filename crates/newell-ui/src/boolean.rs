// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! The Boolean Action Box (M2.22): deterministic, UI-only state composed into the exact ASCII
//! of `docs/ui_design/Action_Box_Boolean.txt` (`README_UI.md` §15–§17).
//!
//! Boolean has no `Solid | Surface` header: its single segmented selector is
//! `Add | Cut | Inter` with the border topology of the active operation. Rows: `Target`,
//! `Tools`, `Offset` (`Cut` only; `Offset all` and its `Dist`, or the explicit `Faces`),
//! `Keep tools`, the footer. No geometry, BrepKIT, real selection or formula evaluation is
//! involved.

use crate::WidgetId;
use crate::action_box::ActionBox;
use crate::create_box::{ActionBoxUi, ActionState, K_ACTIONS, push_footer};
use crate::extrude::MockSelection;

const K_OPERATION: u32 = 0;
const K_TARGET: u32 = 2;
const K_TOOLS: u32 = 3;
const K_OFFSET: u32 = 4;
const K_OFFSET_ALL: u32 = 5;
const K_OFFSET_DIST: u32 = 6;
const K_FACES: u32 = 7;
const K_KEEP_TOOLS: u32 = 8;

const SELECT_KEYS: [u32; 3] = [K_TARGET, K_TOOLS, K_FACES];

/// The border topology of each operation (`Action_Box_Boolean.txt`): the glyph between `Add`
/// and `Cut`, the glyph between `Cut` and `Inter`, and the border row under the selector. The
/// active segment is outlined by solid lines, the others by the dotted `╴` and `┆`.
const HEAD: [(char, char, &str); 3] = [
    ('│', '┆', "├─────────┘╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴╴╴╴╴┤"),
    ('│', '│', "│╴╴╴╴╴╴╴╴╴┴───────┘╴╴╴╴╴╴╴╴╴╴╴┤"),
    ('┆', '│', "│╴╴╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴┴───────────┤"),
];

/// `Add | Cut | Inter`: union, subtraction, intersection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BooleanOperation {
    #[default]
    Add,
    Cut,
    Inter,
}

/// The editable mock values of the Boolean box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BooleanValues {
    pub operation: BooleanOperation,
    pub target: MockSelection,
    pub tools: MockSelection,
    /// `Offset` (`Cut` only).
    pub offset: bool,
    /// `Offset all`; unchecked it reveals `Faces`.
    pub offset_all: bool,
    pub faces: MockSelection,
    pub keep_tools: bool,
}

impl Default for BooleanValues {
    /// `Offset all` starts checked, as in the reference.
    fn default() -> Self {
        Self {
            operation: BooleanOperation::default(),
            target: MockSelection::default(),
            tools: MockSelection::default(),
            offset: false,
            offset_all: true,
            faces: MockSelection::default(),
            keep_tools: false,
        }
    }
}

/// UI state of the Boolean Action Box.
pub type BooleanUi = ActionState<BooleanValues>;

impl BooleanUi {
    /// `Offset` with `Offset all` (a `Dist`) or the explicit `Faces` and `Dist`.
    fn offset(&self, b: &mut ActionBox) {
        let v = &self.values;
        b.push(
            self.row()
                .contour("│ ")
                .check_fill(K_OFFSET, v.offset, "Offset")
                .end(),
        );
        if !v.offset {
            return;
        }
        b.push(
            self.row()
                .contour("│")
                .text(" ╰╴")
                .check_fill(K_OFFSET_ALL, v.offset_all, "Offset all")
                .end(),
        );
        if v.offset_all {
            b.push(
                self.row()
                    .contour("│")
                    .text("   ╰╴Dist  ")
                    .numeric(
                        K_OFFSET_DIST,
                        crate::numeric::field(crate::ActionOperation::Boolean, K_OFFSET_DIST),
                    )
                    .end(),
            );
        } else {
            b.push(
                self.row()
                    .contour("│")
                    .text("   ╰╴Faces   ")
                    .select_or(K_FACES, v.faces.count(), "none")
                    .end(),
            );
            b.push(
                self.row()
                    .contour("│")
                    .text("     Dist  ")
                    .numeric(
                        K_OFFSET_DIST,
                        crate::numeric::field(crate::ActionOperation::Boolean, K_OFFSET_DIST),
                    )
                    .end(),
            );
        }
    }
}

impl ActionBoxUi for BooleanUi {
    fn base_id(&self) -> u32 {
        self.base()
    }

    fn compose_box(&self) -> Option<ActionBox> {
        if !self.is_open() {
            return None;
        }
        let v = &self.values;
        let (add_cut, cut_inter, under) = HEAD[v.operation as usize];
        let mut b = ActionBox::default();
        b.push(
            self.row()
                .contour("╭─")
                .text("Boolean")
                .contour("─┬───────┬───────────╮"),
        );
        b.push(
            self.row()
                .contour("│")
                .button(K_OPERATION, 0, "Add", 9)
                .divider(add_cut, K_OPERATION, 0, 1)
                .button(K_OPERATION, 1, "Cut", 7)
                .divider(cut_inter, K_OPERATION, 1, 2)
                .button(K_OPERATION, 2, "Inter", 11)
                .contour("│"),
        );
        b.push(self.row().contour(under));
        b.push(
            self.row()
                .contour("│")
                .text(" Target      ")
                .select(K_TARGET, self.selection_count(K_TARGET, v.target.count()))
                .end(),
        );
        b.push(
            self.row()
                .contour("│")
                .text(" Tools       ")
                .select(K_TOOLS, self.selection_count(K_TOOLS, v.tools.count()))
                .end(),
        );
        if v.operation == BooleanOperation::Cut {
            self.offset(&mut b);
        }
        b.push(
            self.row()
                .contour("│ ")
                .check_fill(K_KEEP_TOOLS, v.keep_tools, "Keep tools")
                .end(),
        );
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
            (K_OPERATION, 0) => v.operation = BooleanOperation::Add,
            (K_OPERATION, 1) => v.operation = BooleanOperation::Cut,
            (K_OPERATION, 2) => v.operation = BooleanOperation::Inter,
            (K_TARGET, 0) => v.target = v.target.next(),
            (K_TOOLS, 0) => v.tools = v.tools.next(),
            (K_FACES, 0) => v.faces = v.faces.next(),
            (K_OFFSET, 0) => v.offset = !v.offset,
            (K_OFFSET_ALL, 0) => v.offset_all = !v.offset_all,
            (K_KEEP_TOOLS, 0) => v.keep_tools = !v.keep_tools,
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
        crate::ActionOperation::Boolean
    }

    fn selection_descriptor(&self, key: u32) -> Option<crate::SelectionFieldDescriptor> {
        use crate::SelectionFieldRole::*;
        if !SELECT_KEYS.contains(&key) {
            return None;
        }
        let role = match key {
            K_TARGET => BodyTarget,
            K_TOOLS => Bodies,
            K_FACES => UnsupportedTopology,
            _ => EntityReference,
        };
        Some(crate::SelectionFieldDescriptor {
            field: crate::SelectionFieldKey {
                operation: crate::ActionOperation::Boolean,
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
