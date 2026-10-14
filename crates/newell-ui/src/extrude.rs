// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Reference Extrude ActionPanel (M2.11): deterministic, UI-only mock state.
//!
//! [`ExtrudeUi`] is plain data. [`ExtrudeUi::compose`] rebuilds an [`ActionPanel`] from it
//! with the M2.10 framework and the M2.7 primitives; [`ExtrudeUi::activate`] applies a
//! pointer or keyboard activation, identified by [`WidgetId`], to the same state. Nothing here
//! touches geometry, BrepKIT, the model, real selection or formula evaluation.
//!
//! # Progressive disclosure
//!
//! Rows that do not apply are absent from the composed list (README §5, §7.8). Order is fixed:
//!
//! 1. `Solid | Surface` and `New | Add | Remove` (segmented, always present);
//! 2. termination (`◀ Blind ▶`, cyclic, always present);
//! 3. `Depth` only for `Blind`; `Selection` only for `Up to`; nothing for `Through all`;
//! 4. `Invert`;
//! 5. `Starting Offset`; when checked it discloses a `Blind` / `Up to` cycle and then `Dist`
//!    (`Blind`) or `Target` (`Up to`);
//! 6. `Symmetric`;
//! 7. `Second end position`, only while `Symmetric` is unchecked;
//! 8. `Merge with all`, only when the result is `Add` or `Remove`; when unchecked it
//!    discloses `Merge scope`;
//! 9. separator and `OK` / `Cancel`.
//!
//! Widget ids come from [`ActionPanel::widget_id`] with the stable `K_*` row keys, so they
//! never depend on which rows are currently disclosed.

use crate::{
    ActionPanel, PanelRow, PanelRowKind, ROW_ID_STRIDE, WidgetId, WidgetState, cycle_next,
    cycle_prev,
};

const K_GEOMETRY: u32 = 0;
const K_RESULT: u32 = 1;
const K_TERMINATION: u32 = 2;
const K_DEPTH: u32 = 3;
const K_SELECTION: u32 = 4;
const K_INVERT: u32 = 5;
const K_START_OFFSET: u32 = 6;
const K_START_MODE: u32 = 7;
const K_START_DIST: u32 = 8;
const K_START_TARGET: u32 = 9;
const K_SYMMETRIC: u32 = 10;
const K_SECOND_END: u32 = 11;
const K_MERGE_ALL: u32 = 12;
const K_MERGE_SCOPE: u32 = 13;
const K_SEPARATOR: u32 = 14;
const K_ACTIONS: u32 = 15;

/// Mock field text; numeric fields are UI-only and never edited or evaluated.
const MOCK_DISTANCE: &str = "0.0 mm";
/// Element count shown for [`MockSelection::Many`].
const MOCK_MANY: u32 = 3;
const INDENT: u32 = 2;

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

/// End condition of the starting offset: `Blind | Up to` (cyclic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtrudeOffsetMode {
    #[default]
    Blind,
    UpTo,
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

    /// Field text: `none`, `1 Element`, `3 Elements`.
    #[must_use]
    pub fn text(self) -> String {
        match self {
            Self::None => "none".to_owned(),
            Self::One => "1 Element".to_owned(),
            Self::Many => format!("{} Elements", self.count()),
        }
    }

    /// Mock edit: `none` -> `1 Element` -> `N Elements` -> `none`.
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
    pub termination: ExtrudeTermination,
    pub selection: MockSelection,
    pub invert: bool,
    pub starting_offset: bool,
    pub start_mode: ExtrudeOffsetMode,
    pub start_target: MockSelection,
    pub symmetric: bool,
    pub second_end: bool,
    pub merge_with_all: bool,
    pub merge_scope: MockSelection,
}

impl Default for ExtrudeValues {
    /// `Merge with all` starts checked, as in the reference.
    fn default() -> Self {
        Self {
            geometry: ExtrudeGeometryType::default(),
            result: ExtrudeResultMode::default(),
            termination: ExtrudeTermination::default(),
            selection: MockSelection::default(),
            invert: false,
            starting_offset: false,
            start_mode: ExtrudeOffsetMode::default(),
            start_target: MockSelection::default(),
            symmetric: false,
            second_end: false,
            merge_with_all: true,
            merge_scope: MockSelection::default(),
        }
    }
}

/// UI state of the Extrude ActionPanel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtrudeUi {
    base_id: u32,
    pub values: ExtrudeValues,
    opened: ExtrudeValues,
    pub status: ExtrudeStatus,
}

impl ExtrudeUi {
    /// Opens the panel with default values; `base_id` is the panel's first widget id.
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

    /// Whether the panel is still shown (neither OK nor Cancel was activated).
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status == ExtrudeStatus::Editing
    }

    /// Reopens with default values.
    pub fn reopen(&mut self) {
        *self = Self::new(self.base_id);
    }

    /// Composes the panel for the current values, or `None` once OK / Cancel closed it.
    #[must_use]
    pub fn compose(&self) -> Option<ActionPanel> {
        if !self.is_open() {
            return None;
        }
        let v = &self.values;
        let mut p = ActionPanel::new("Extrude", self.base_id);
        let s = |t: &str| t.to_owned();
        let check = |on: bool| WidgetState {
            checked: on,
            ..WidgetState::default()
        };
        let checkbox = |key: u32, label: &str, on: bool| {
            PanelRow::new(key, PanelRowKind::Checkbox { label: s(label) }).with_state(check(on))
        };
        let numeric = |key: u32, label: &str| {
            PanelRow::new(
                key,
                PanelRowKind::Numeric {
                    label: s(label),
                    text: s(MOCK_DISTANCE),
                },
            )
        };
        let field = |key: u32, label: &str, sel: MockSelection| {
            PanelRow::new(
                key,
                PanelRowKind::Text {
                    label: s(label),
                    text: sel.text(),
                },
            )
        };

        p.push(PanelRow::new(
            K_GEOMETRY,
            PanelRowKind::Segmented {
                labels: vec![s("Solid"), s("Surface")],
                active: v.geometry as usize,
            },
        ));
        p.push(PanelRow::new(
            K_RESULT,
            PanelRowKind::Segmented {
                labels: vec![s("New"), s("Add"), s("Remove")],
                active: v.result as usize,
            },
        ));
        p.push(PanelRow::new(
            K_TERMINATION,
            PanelRowKind::Cyclic {
                values: vec![s("Blind"), s("Up to"), s("Through all")],
                current: v.termination as usize,
            },
        ));
        match v.termination {
            ExtrudeTermination::Blind => p.push(numeric(K_DEPTH, "Depth")),
            ExtrudeTermination::UpTo => p.push(field(K_SELECTION, "Selection", v.selection)),
            ExtrudeTermination::ThroughAll => {}
        }
        p.push(checkbox(K_INVERT, "Invert", v.invert));
        p.push(checkbox(
            K_START_OFFSET,
            "Starting Offset",
            v.starting_offset,
        ));
        if v.starting_offset {
            p.push(
                PanelRow::new(
                    K_START_MODE,
                    PanelRowKind::Cyclic {
                        values: vec![s("Blind"), s("Up to")],
                        current: v.start_mode as usize,
                    },
                )
                .indented(INDENT),
            );
            p.push(
                match v.start_mode {
                    ExtrudeOffsetMode::Blind => numeric(K_START_DIST, "Dist"),
                    ExtrudeOffsetMode::UpTo => field(K_START_TARGET, "Target", v.start_target),
                }
                .indented(INDENT),
            );
        }
        p.push(checkbox(K_SYMMETRIC, "Symmetric", v.symmetric));
        if !v.symmetric {
            p.push(checkbox(K_SECOND_END, "Second end position", v.second_end));
        }
        if v.result != ExtrudeResultMode::New {
            p.push(checkbox(K_MERGE_ALL, "Merge with all", v.merge_with_all));
            if !v.merge_with_all {
                p.push(field(K_MERGE_SCOPE, "Merge scope", v.merge_scope).indented(INDENT));
            }
        }
        p.push(PanelRow::new(K_SEPARATOR, PanelRowKind::Separator));
        p.push(PanelRow::new(
            K_ACTIONS,
            PanelRowKind::Buttons {
                labels: vec![s("OK"), s("Cancel")],
            },
        ));
        Some(p)
    }

    /// Applies activation of `id` to the mock state. Returns `true` if the state changed.
    ///
    /// Ids outside this panel, controls that are not currently disclosed, the mock numeric
    /// fields and their formula buttons (M2.12) change nothing. A closed panel ignores
    /// everything.
    pub fn activate(&mut self, id: WidgetId) -> bool {
        if !self.is_open() {
            return false;
        }
        let Some(rel) = id.get().checked_sub(self.base_id) else {
            return false;
        };
        let (key, slot) = (rel / ROW_ID_STRIDE, rel % ROW_ID_STRIDE);
        let Some(panel) = self.compose() else {
            return false;
        };
        // Only controls present in the composed panel are activatable.
        if !panel.rows.iter().any(|r| r.key == key) {
            return false;
        }
        let before = (self.values, self.status);
        let v = &mut self.values;
        match (key, slot) {
            (K_GEOMETRY, 0) => v.geometry = ExtrudeGeometryType::Solid,
            (K_GEOMETRY, 1) => v.geometry = ExtrudeGeometryType::Surface,
            (K_RESULT, 0) => v.result = ExtrudeResultMode::New,
            (K_RESULT, 1) => v.result = ExtrudeResultMode::Add,
            (K_RESULT, 2) => v.result = ExtrudeResultMode::Remove,
            (K_TERMINATION, 0) => {
                v.termination = termination_at(cycle_prev(v.termination as usize, 3));
            }
            (K_TERMINATION, 1) => {
                v.termination = termination_at(cycle_next(v.termination as usize, 3));
            }
            (K_START_MODE, 0 | 1) => {
                // Two values: either arrow flips.
                v.start_mode = match v.start_mode {
                    ExtrudeOffsetMode::Blind => ExtrudeOffsetMode::UpTo,
                    ExtrudeOffsetMode::UpTo => ExtrudeOffsetMode::Blind,
                };
            }
            (K_SELECTION, 0) => v.selection = v.selection.next(),
            (K_START_TARGET, 0) => v.start_target = v.start_target.next(),
            (K_MERGE_SCOPE, 0) => v.merge_scope = v.merge_scope.next(),
            (K_INVERT, 0) => v.invert = !v.invert,
            (K_START_OFFSET, 0) => v.starting_offset = !v.starting_offset,
            (K_SYMMETRIC, 0) => v.symmetric = !v.symmetric,
            (K_SECOND_END, 0) => v.second_end = !v.second_end,
            (K_MERGE_ALL, 0) => v.merge_with_all = !v.merge_with_all,
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

    /// Id of control `slot` of the row `key`, as drawn by the composed panel.
    #[cfg(test)]
    fn id(&self, key: u32, slot: u32) -> WidgetId {
        ActionPanel::new("", self.base_id)
            .widget_id(key, slot)
            .unwrap()
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
mod tests {
    use super::*;
    use crate::{CellContent, CellGrid, GridRect, InteractionState, WidgetStyles};

    const BASE: u32 = 1000;

    fn ui() -> ExtrudeUi {
        ExtrudeUi::new(BASE)
    }

    fn click(u: &mut ExtrudeUi, key: u32, slot: u32) -> bool {
        let id = u.id(key, slot);
        u.activate(id)
    }

    fn keys(u: &ExtrudeUi) -> Vec<u32> {
        u.compose().unwrap().rows.iter().map(|r| r.key).collect()
    }

    fn render(u: &ExtrudeUi, cols: u32, rows: u32) -> Option<(CellGrid, usize)> {
        let p = u.compose()?;
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let ctx = crate::WidgetContext {
            styles: &styles,
            interaction: &inter,
        };
        let mut g = CellGrid::new(cols, rows).unwrap();
        let n = p.draw(&mut g, &ctx, GridRect::new(0, 0, cols, rows));
        Some((g, n))
    }

    fn text(g: &CellGrid) -> Vec<String> {
        (0..g.rows())
            .map(|r| {
                (0..g.cols())
                    .map(|c| match g.get(c, r).map(|x| x.content) {
                        Some(CellContent::Glyph(ch)) => ch,
                        _ => ' ',
                    })
                    .collect()
            })
            .collect()
    }

    fn has(g: &CellGrid, needle: &str) -> bool {
        text(g).iter().any(|r| r.contains(needle))
    }

    #[test]
    fn default_state_and_row_order() {
        let u = ui();
        assert_eq!(
            keys(&u),
            [
                K_GEOMETRY,
                K_RESULT,
                K_TERMINATION,
                K_DEPTH,
                K_INVERT,
                K_START_OFFSET,
                K_SYMMETRIC,
                K_SECOND_END,
                K_SEPARATOR,
                K_ACTIONS
            ]
        );
        let (g, n) = render(&u, 32, 20).unwrap();
        assert_eq!(n, 10);
        let t = text(&g);
        assert!(t[1].contains("Extrude"));
        assert!(t[3].contains("Solid") && t[3].contains("Surface"));
        assert!(t[4].contains("New") && t[4].contains("Add") && t[4].contains("Remove"));
        assert!(t[5].contains('◀') && t[5].contains("Blind") && t[5].contains('▶'));
        assert!(t[6].contains("Depth") && t[6].contains("0.0 mm") && t[6].contains('☰'));
        assert!(t[7].contains("□ Invert"));
        assert!(t[12].contains("OK") && t[12].contains("Cancel"));
        assert_eq!(render(&u, 32, 20).unwrap().0, g);
    }

    #[test]
    fn solid_surface_keeps_both_segments_and_same_rows() {
        let mut u = ui();
        let solid = keys(&u);
        assert!(click(&mut u, K_GEOMETRY, 1));
        assert_eq!(u.values.geometry, ExtrudeGeometryType::Surface);
        assert_eq!(keys(&u), solid);
        let (g, _) = render(&u, 32, 20).unwrap();
        assert!(text(&g)[3].contains("Solid") && text(&g)[3].contains("Surface"));
        assert!(!click(&mut u, K_GEOMETRY, 1));
        assert!(click(&mut u, K_GEOMETRY, 0));
        assert_eq!(u.values.geometry, ExtrudeGeometryType::Solid);
    }

    #[test]
    fn result_modes_keep_all_segments_and_gate_merge_controls() {
        let mut u = ui();
        assert!(!keys(&u).contains(&K_MERGE_ALL));
        for (slot, mode) in [(1, ExtrudeResultMode::Add), (2, ExtrudeResultMode::Remove)] {
            assert!(click(&mut u, K_RESULT, slot));
            assert_eq!(u.values.result, mode);
            assert!(keys(&u).contains(&K_MERGE_ALL));
            let (g, _) = render(&u, 32, 20).unwrap();
            assert!(has(&g, "New") && has(&g, "Add") && has(&g, "Remove"));
            assert!(has(&g, "■ Merge with all"));
        }
        assert!(click(&mut u, K_RESULT, 0));
        assert!(!keys(&u).contains(&K_MERGE_ALL));
        assert!(!keys(&u).contains(&K_MERGE_SCOPE));
    }

    #[test]
    fn merge_scope_appears_only_when_merge_with_all_is_unchecked() {
        let mut u = ui();
        click(&mut u, K_RESULT, 1);
        assert!(!keys(&u).contains(&K_MERGE_SCOPE));
        assert!(click(&mut u, K_MERGE_ALL, 0));
        let k = keys(&u);
        let at = |key| k.iter().position(|&x| x == key).unwrap();
        assert_eq!(at(K_MERGE_SCOPE), at(K_MERGE_ALL) + 1);
        let (g, _) = render(&u, 32, 20).unwrap();
        assert!(has(&g, "□ Merge with all") && has(&g, "Merge scope") && has(&g, "none"));
        assert!(click(&mut u, K_MERGE_SCOPE, 0));
        assert!(
            render(&u, 32, 20)
                .map(|(g, _)| has(&g, "1 Element"))
                .unwrap()
        );
        // Hidden control is not activatable even by a stale id.
        assert!(click(&mut u, K_MERGE_ALL, 0));
        let before = u.values;
        assert!(!click(&mut u, K_MERGE_SCOPE, 0));
        assert_eq!(u.values, before);
        // Merge scope disappears with the result mode too.
        click(&mut u, K_MERGE_ALL, 0);
        click(&mut u, K_RESULT, 0);
        assert!(!keys(&u).contains(&K_MERGE_SCOPE));
    }

    #[test]
    fn termination_is_cyclic_and_gates_depth_and_selection() {
        let mut u = ui();
        let has_key = |u: &ExtrudeUi, k| keys(u).contains(&k);
        assert!(has_key(&u, K_DEPTH) && !has_key(&u, K_SELECTION));
        // next: Blind -> Up to -> Through all -> Blind
        assert!(click(&mut u, K_TERMINATION, 1));
        assert_eq!(u.values.termination, ExtrudeTermination::UpTo);
        assert!(!has_key(&u, K_DEPTH) && has_key(&u, K_SELECTION));
        assert!(click(&mut u, K_TERMINATION, 1));
        assert_eq!(u.values.termination, ExtrudeTermination::ThroughAll);
        assert!(!has_key(&u, K_DEPTH) && !has_key(&u, K_SELECTION));
        let (g, _) = render(&u, 32, 20).unwrap();
        assert!(has(&g, "Through all") && !has(&g, "Depth") && !has(&g, "Selection"));
        assert!(click(&mut u, K_TERMINATION, 1));
        assert_eq!(u.values.termination, ExtrudeTermination::Blind);
        // prev wraps backwards.
        assert!(click(&mut u, K_TERMINATION, 0));
        assert_eq!(u.values.termination, ExtrudeTermination::ThroughAll);
        assert!(click(&mut u, K_TERMINATION, 0));
        assert_eq!(u.values.termination, ExtrudeTermination::UpTo);
    }

    #[test]
    fn hidden_rows_are_physically_absent() {
        let mut u = ui();
        let (blind, n_blind) = render(&u, 32, 20).unwrap();
        click(&mut u, K_TERMINATION, 1);
        click(&mut u, K_TERMINATION, 1);
        let (through, n_through) = render(&u, 32, 20).unwrap();
        // One row fewer; everything below Depth moved up, no blank placeholder.
        assert_eq!(n_through + 1, n_blind);
        assert_eq!(text(&through)[6].trim_matches(['│', ' ']), "□ Invert");
        assert_ne!(blind, through);
        // No cell of the absent row carries one of its ids.
        let p = u.compose().unwrap();
        let ids: Vec<u32> = (0..2)
            .map(|s| p.widget_id(K_DEPTH, s).unwrap().get())
            .collect();
        for r in 0..through.rows() {
            for c in 0..through.cols() {
                let owner = through.get(c, r).and_then(|x| x.widget_id);
                assert!(owner.is_none_or(|o| !ids.contains(&o.get())));
            }
        }
    }

    #[test]
    fn selection_shows_mock_counts_only() {
        let mut u = ui();
        click(&mut u, K_TERMINATION, 1);
        let (g, _) = render(&u, 32, 20).unwrap();
        assert!(has(&g, "Selection") && has(&g, "none"));
        for (want, count) in [("1 Element", 1), ("3 Elements", 3), ("none", 0)] {
            assert!(click(&mut u, K_SELECTION, 0));
            assert_eq!(u.values.selection.count(), count);
            assert!(has(&render(&u, 32, 20).unwrap().0, want));
        }
        assert_eq!(MockSelection::One.text(), "1 Element");
        assert_eq!(MockSelection::Many.text(), "3 Elements");
        // Selection is not offered under Blind.
        click(&mut u, K_TERMINATION, 0);
        assert!(!click(&mut u, K_SELECTION, 0));
    }

    #[test]
    fn starting_offset_discloses_mode_and_dist_or_target() {
        let mut u = ui();
        let closed = keys(&u);
        assert!(click(&mut u, K_START_OFFSET, 0));
        let k = keys(&u);
        let at = |key| k.iter().position(|&x| x == key).unwrap();
        assert_eq!(at(K_START_MODE), at(K_START_OFFSET) + 1);
        assert_eq!(at(K_START_DIST), at(K_START_OFFSET) + 2);
        assert!(!k.contains(&K_START_TARGET));
        let (g, _) = render(&u, 32, 20).unwrap();
        assert!(has(&g, "■ Starting Offset") && has(&g, "Dist"));
        assert!(click(&mut u, K_START_MODE, 1));
        let k = keys(&u);
        assert!(k.contains(&K_START_TARGET) && !k.contains(&K_START_DIST));
        assert!(has(&render(&u, 32, 20).unwrap().0, "Target"));
        assert!(click(&mut u, K_START_TARGET, 0));
        assert_eq!(u.values.start_target, MockSelection::One);
        // Both arrows flip the two-value cycle.
        assert!(click(&mut u, K_START_MODE, 0));
        assert_eq!(u.values.start_mode, ExtrudeOffsetMode::Blind);
        // Unchecking removes the disclosed rows again.
        assert!(click(&mut u, K_START_OFFSET, 0));
        assert_eq!(keys(&u), closed);
        // Independent of the end condition.
        click(&mut u, K_TERMINATION, 1);
        click(&mut u, K_TERMINATION, 1);
        assert!(keys(&u).contains(&K_START_OFFSET));
    }

    #[test]
    fn invert_symmetric_and_second_end_toggle_with_glyphs() {
        let mut u = ui();
        assert!(click(&mut u, K_INVERT, 0));
        assert!(has(&render(&u, 32, 20).unwrap().0, "■ Invert"));
        assert!(click(&mut u, K_INVERT, 0));
        assert!(has(&render(&u, 32, 20).unwrap().0, "□ Invert"));
        assert!(click(&mut u, K_SECOND_END, 0));
        assert!(has(&render(&u, 32, 20).unwrap().0, "■ Second end position"));
        // Symmetric removes Second end position (and its click does nothing).
        assert!(click(&mut u, K_SYMMETRIC, 0));
        assert!(!keys(&u).contains(&K_SECOND_END));
        assert!(!has(&render(&u, 32, 20).unwrap().0, "Second end"));
        assert!(!click(&mut u, K_SECOND_END, 0));
        assert!(u.values.second_end);
        assert!(click(&mut u, K_SYMMETRIC, 0));
        assert!(has(&render(&u, 32, 20).unwrap().0, "■ Second end position"));
    }

    #[test]
    fn numeric_fields_and_foreign_ids_change_nothing() {
        let mut u = ui();
        let before = u;
        assert!(!click(&mut u, K_DEPTH, 0));
        assert!(!click(&mut u, K_DEPTH, 1));
        assert!(!u.activate(WidgetId::new(0)));
        assert!(!u.activate(WidgetId::new(BASE + 63)));
        assert!(!u.activate(WidgetId::new(u32::MAX)));
        assert_eq!(u, before);
    }

    #[test]
    fn ok_confirms_and_cancel_restores_opening_values() {
        let mut ok = ui();
        click(&mut ok, K_RESULT, 2);
        click(&mut ok, K_INVERT, 0);
        assert!(click(&mut ok, K_ACTIONS, 0));
        assert_eq!(ok.status, ExtrudeStatus::Confirmed);
        assert!(!ok.is_open() && ok.compose().is_none());
        assert_eq!(ok.values.result, ExtrudeResultMode::Remove);
        assert!(!click(&mut ok, K_INVERT, 0));
        assert!(render(&ok, 32, 20).is_none());

        let mut cancel = ui();
        click(&mut cancel, K_RESULT, 2);
        click(&mut cancel, K_TERMINATION, 1);
        assert!(click(&mut cancel, K_ACTIONS, 1));
        assert_eq!(cancel.status, ExtrudeStatus::Cancelled);
        assert_eq!(cancel.values, ExtrudeValues::default());
        assert!(cancel.compose().is_none());

        cancel.reopen();
        assert!(cancel.is_open());
        assert_eq!(cancel.values, ExtrudeValues::default());
        assert!(cancel.compose().is_some());
    }

    #[test]
    fn disclosed_rows_do_not_change_other_rows_ids() {
        let mut u = ui();
        let ids_of = |u: &ExtrudeUi, key| {
            let (g, _) = render(u, 32, 22).unwrap();
            let mut v = Vec::new();
            for r in 0..g.rows() {
                for c in 0..g.cols() {
                    if let Some(id) = g.get(c, r).and_then(|x| x.widget_id) {
                        let rel = id.get() - BASE;
                        if rel / ROW_ID_STRIDE == key {
                            v.push(id.get());
                        }
                    }
                }
            }
            v.sort_unstable();
            v.dedup();
            v
        };
        let before = ids_of(&u, K_ACTIONS);
        click(&mut u, K_RESULT, 1);
        click(&mut u, K_MERGE_ALL, 0);
        click(&mut u, K_START_OFFSET, 0);
        click(&mut u, K_TERMINATION, 1);
        assert_eq!(ids_of(&u, K_ACTIONS), before);
        assert_eq!(before.len(), 2);
    }

    #[test]
    fn full_disclosure_row_order_is_deterministic() {
        let mut u = ui();
        click(&mut u, K_RESULT, 1);
        click(&mut u, K_MERGE_ALL, 0);
        click(&mut u, K_START_OFFSET, 0);
        click(&mut u, K_TERMINATION, 1);
        assert_eq!(
            keys(&u),
            [
                K_GEOMETRY,
                K_RESULT,
                K_TERMINATION,
                K_SELECTION,
                K_INVERT,
                K_START_OFFSET,
                K_START_MODE,
                K_START_DIST,
                K_SYMMETRIC,
                K_SECOND_END,
                K_MERGE_ALL,
                K_MERGE_SCOPE,
                K_SEPARATOR,
                K_ACTIONS
            ]
        );
        assert_eq!(u.compose(), u.compose());
    }

    #[test]
    fn tiny_and_degenerate_grids_are_safe() {
        let mut u = ui();
        click(&mut u, K_RESULT, 1);
        click(&mut u, K_MERGE_ALL, 0);
        click(&mut u, K_START_OFFSET, 0);
        for (w, h) in [(0, 0), (1, 1), (2, 2), (4, 4), (5, 3), (8, 6), (40, 3)] {
            if let Some((g, n)) = render(&u, w, h) {
                assert!(n <= h as usize, "{w}x{h}");
                assert_eq!(g.cells().len(), (w * h) as usize);
            }
        }
        // A short grid shows the first rows that fit, in order.
        let (g, n) = render(&u, 32, 7).unwrap();
        assert_eq!(n, 3);
        assert!(has(&g, "Solid") && has(&g, "Remove") && has(&g, "Blind"));
        assert!(!has(&g, "OK"));
        // Off-grid placement draws nothing and does not panic.
        let p = u.compose().unwrap();
        let styles = WidgetStyles::default();
        let inter = InteractionState::default();
        let ctx = crate::WidgetContext {
            styles: &styles,
            interaction: &inter,
        };
        let mut g = CellGrid::new(6, 4).unwrap();
        let blank = g.clone();
        p.draw(&mut g, &ctx, GridRect::new(100, 100, 30, 30));
        assert_eq!(g, blank);
    }
}
