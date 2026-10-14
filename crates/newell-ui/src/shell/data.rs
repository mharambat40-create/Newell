// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Deterministic mock data of the reference shell: menus, trees and status text.
//!
//! Everything here is fixed text from `docs/ui_design/` (`Application_Menu_Bar.txt`,
//! `Command_Bar.txt`, `Interface_Exemple.txt`). No entry refers to the model.

use crate::{MenuAction, MenuCheckbox, MenuCycle, MenuDef, MenuEntry, MenuItem, Mnemonic};

// Semantic action ids returned by the menus (`MenuAction`). Only the ones below the
// "effects" comment change shell state; the rest are accepted and ignored.
pub(super) const ACT_WORKBENCH: u32 = 100;
pub(super) const ACT_VIEW_PROJECTION: u32 = 200;
pub(super) const ACT_VIEW_SHADING: u32 = 201;
pub(super) const ACT_VIEW_CHECK: u32 = 210;
pub(super) const ACT_EXTRUDE: u32 = 300;

const fn mn(text: &'static str) -> Option<Mnemonic> {
    if text.is_empty() {
        None
    } else {
        Some(Mnemonic::new(text))
    }
}

const fn item(action: u32, label: &'static str, mnemonic: &'static str) -> MenuEntry<'static> {
    MenuEntry::Item(MenuItem {
        action: MenuAction::new(action),
        label,
        mnemonic: mn(mnemonic),
        enabled: true,
    })
}

const fn menu(
    label: &'static str,
    mnemonic: &'static str,
    entries: &'static [MenuEntry<'static>],
) -> MenuDef<'static> {
    MenuDef {
        label,
        mnemonic: mn(mnemonic),
        entries,
    }
}

const FILE: [MenuEntry<'static>; 8] = [
    item(10, "New", ""),
    item(11, "Open", ""),
    item(12, "Recent", ""),
    item(13, "Save", ""),
    item(14, "Save As", ""),
    item(15, "Import", ""),
    item(16, "Export", ""),
    item(17, "Option", ""),
];
const EDIT: [MenuEntry<'static>; 3] = [
    item(20, "Undo", ""),
    item(21, "Redo", ""),
    item(22, "Recompute", ""),
];
const HELP: [MenuEntry<'static>; 6] = [
    item(40, "Tutorial", ""),
    item(41, "Docs", ""),
    item(42, "Shortcuts", ""),
    item(43, "Bug", ""),
    item(44, "Update", ""),
    item(45, "About", ""),
];

/// Workbench names; the menu rows and the StatusBar share them.
pub(super) const WORKBENCHES: [&str; 9] = [
    "Part design",
    "Assemblies",
    "Mesh",
    "Parametric",
    "Drawing",
    "Simulation",
    "Animation",
    "Manufacture",
    "PLM",
];
const WORKBENCH: [MenuEntry<'static>; 9] = [
    item(ACT_WORKBENCH, WORKBENCHES[0], ""),
    item(ACT_WORKBENCH + 1, WORKBENCHES[1], ""),
    item(ACT_WORKBENCH + 2, WORKBENCHES[2], ""),
    item(ACT_WORKBENCH + 3, WORKBENCHES[3], ""),
    item(ACT_WORKBENCH + 4, WORKBENCHES[4], ""),
    item(ACT_WORKBENCH + 5, WORKBENCHES[5], ""),
    item(ACT_WORKBENCH + 6, WORKBENCHES[6], ""),
    item(ACT_WORKBENCH + 7, WORKBENCHES[7], ""),
    item(ACT_WORKBENCH + 8, WORKBENCHES[8], ""),
];
// The reference has no Sketch group; one mock row keeps the title a normal menu.
const SKETCH: [MenuEntry<'static>; 1] = [item(290, "New Sketch", "")];
const CREATE: [MenuEntry<'static>; 4] = [
    item(ACT_EXTRUDE, "[E]xtrude", "E"),
    item(301, "[R]evolve", "R"),
    item(302, "[L]oft", "L"),
    item(303, "[S]weep", "S"),
];
const OPERATE: [MenuEntry<'static>; 4] = [
    item(310, "[B]oolean", "B"),
    item(311, "[S]plit [F]ace", "SF"),
    item(312, "[S]plit [B]ody", "SB"),
    item(313, "[T]ransform", "T"),
];
const PATTERN: [MenuEntry<'static>; 4] = [
    item(320, "[Mi]rror", "Mi"),
    item(321, "[Li]near", "Li"),
    item(322, "[Ci]rcular", "Ci"),
    item(323, "[Cu]rve", "Cu"),
];
const REFINE: [MenuEntry<'static>; 4] = [
    item(330, "[F]illet", "F"),
    item(331, "[C]hamfer", "C"),
    item(332, "[S]hell", "S"),
    item(333, "[D]raft", "D"),
];
const CONSTRUCT: [MenuEntry<'static>; 3] = [
    item(340, "[Pl]ane", "Pl"),
    item(341, "[A]xis", "A"),
    item(342, "[P]oint", "P"),
];
const INSPECT: [MenuEntry<'static>; 4] = [
    item(350, "[M]easure", "M"),
    item(351, "[S]plit [V]iew", "SV"),
    item(352, "Curvature", ""),
    item(353, "Properties", ""),
];

/// The CommandBar groups, in reference order. Plain static data: they have no mutable
/// rows.
pub(super) static COMMAND_MENUS: [MenuDef<'static>; 8] = [
    menu("Workbench", "", &WORKBENCH),
    menu("Sketch", "", &SKETCH),
    menu("Create", "", &CREATE),
    menu("Operate", "", &OPERATE),
    menu("Pattern", "", &PATTERN),
    menu("Refine", "", &REFINE),
    menu("Construct", "", &CONSTRUCT),
    menu("Inspect", "", &INSPECT),
];

pub(super) const PROJECTION: [&str; 2] = ["Perspective", "Orthographic"];
pub(super) const SHADING: [&str; 6] = [
    "Points",
    "Wireframe",
    "Hidden line",
    "No shading",
    "Shaded",
    "Flat line",
];
pub(super) const VIEW_CHECKS: usize = 5;
const VIEW_CHECK_LABELS: [&str; VIEW_CHECKS] = [
    "Tree panel",
    "Task bar",
    "Status bar",
    "Navigation cube",
    "Grid Overlay",
];

/// The mutable mock rows of the View menu: the two cycles and the five checkboxes. They
/// change only what the menu shows (README §7.4); nothing in the viewport follows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ViewMock {
    pub projection: usize,
    pub shading: usize,
    pub checks: [bool; VIEW_CHECKS],
}

impl Default for ViewMock {
    fn default() -> Self {
        Self {
            projection: 0,
            shading: 4,
            checks: [true, true, true, true, false],
        }
    }
}

fn view_entries(v: &ViewMock) -> [MenuEntry<'static>; 10] {
    let cycle = |action: u32, values: &'static [&'static str], current: usize| {
        MenuEntry::Cyclic(MenuCycle {
            action: MenuAction::new(action),
            values,
            current,
            enabled: true,
        })
    };
    let check = |i: usize| {
        MenuEntry::Checkbox(MenuCheckbox {
            action: MenuAction::new(ACT_VIEW_CHECK + u32::try_from(i).unwrap_or(0)),
            label: VIEW_CHECK_LABELS[i],
            checked: v.checks[i],
            mnemonic: None,
            enabled: true,
        })
    };
    [
        cycle(ACT_VIEW_PROJECTION, &PROJECTION, v.projection),
        cycle(ACT_VIEW_SHADING, &SHADING, v.shading),
        check(0),
        check(1),
        check(2),
        check(3),
        check(4),
        item(220, "[N]ormal to", "N"),
        item(221, "Reset Cam", ""),
        item(222, "Fullscreen", ""),
    ]
}

/// Calls `f` with the MenuBar menus (File, Edit, View, Help) for the current View rows.
pub(super) fn with_bar_menus<R>(view: &ViewMock, f: impl FnOnce(&[MenuDef<'_>]) -> R) -> R {
    let entries = view_entries(view);
    let menus = [
        menu("File", "F", &FILE),
        menu("Edit", "E", &EDIT),
        MenuDef {
            label: "View",
            mnemonic: mn("V"),
            entries: &entries,
        },
        menu("Help", "H", &HELP),
    ];
    f(&menus)
}

/// One node of a mock tree, in a flat pre-order list: nesting is the `depth` field, not
/// a pointer structure.
#[derive(Debug, Clone, Copy)]
pub(super) struct Node {
    pub label: &'static str,
    pub depth: u32,
    pub branch: bool,
    /// Marker of a leaf (`■` / `□`); its meaning is open (README §12.2).
    pub checked: bool,
    /// Initial expansion of a branch.
    pub open: bool,
}

const fn node(label: &'static str, depth: u32, branch: bool, checked: bool, open: bool) -> Node {
    Node {
        label,
        depth,
        branch,
        checked,
        open,
    }
}

/// Maximum nodes per tree: the id range reserved for each tree.
pub(super) const MAX_NODES: usize = 64;

pub(super) const FEATURES: [Node; 9] = [
    node("Features", 0, true, false, true),
    node("Sketch_Profile_Main", 1, false, false, false),
    node("Hub_Features", 1, true, false, true),
    node("Extrude_Base", 2, false, false, false),
    node("Sketch_Hole", 2, false, true, false),
    node("Extrude_Cut_Center", 2, false, false, false),
    node("Fillet_Inner", 1, false, false, false),
    node("Chamfer_Edges", 1, false, false, false),
    node("Pattern_Circular", 1, false, false, false),
];

pub(super) const ELEMENTS: [Node; 7] = [
    node("Elements", 0, true, false, true),
    node("Solid Bodies(2)", 1, true, false, true),
    node("Rim_Body", 2, false, true, false),
    node("Hub_Body", 2, false, false, false),
    node("Surfaces(0)", 1, false, false, false),
    node("Construct", 1, true, false, false),
    node("Plane_XY", 2, false, false, false),
];

/// Initial expansion flags of a tree.
pub(super) fn initial_open(nodes: &[Node]) -> [bool; MAX_NODES] {
    let mut open = [false; MAX_NODES];
    for (slot, n) in open.iter_mut().zip(nodes) {
        *slot = n.open;
    }
    open
}

/// A visible tree row: the node plus the guide data `draw_tree_row` needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Row {
    pub index: usize,
    pub last: bool,
    pub trunk: Vec<bool>,
}

/// The rows currently shown: every node whose ancestors are all expanded, with the
/// last-sibling flag and the continuing ancestor guides of each.
pub(super) fn visible_rows(nodes: &[Node], open: &[bool]) -> Vec<Row> {
    // A node is the last of its siblings if the next node at its depth or shallower is
    // shallower (or absent).
    let last: Vec<bool> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            nodes[i + 1..]
                .iter()
                .find(|m| m.depth <= n.depth)
                .is_none_or(|m| m.depth < n.depth)
        })
        .collect();
    let mut rows = Vec::new();
    let mut ancestors: Vec<usize> = Vec::new();
    let mut hidden_below: Option<u32> = None;
    for (i, n) in nodes.iter().enumerate() {
        ancestors.truncate(n.depth as usize);
        ancestors.push(i);
        if let Some(depth) = hidden_below {
            if n.depth > depth {
                continue;
            }
            hidden_below = None;
        }
        let trunk = (1..n.depth as usize)
            .map(|level| ancestors.get(level).is_some_and(|&a| !last[a]))
            .collect();
        rows.push(Row {
            index: i,
            last: last[i],
            trunk,
        });
        if n.branch && !open.get(i).copied().unwrap_or(false) {
            hidden_below = Some(n.depth);
        }
    }
    rows
}

/// Mock StatusBar values (deterministic: no live frame rate).
pub(super) const DOCUMENT: &str = "Roue V1*";
pub(super) const CONSTRAINT: &str = "Fully Constrained";
pub(super) const FPS: u32 = 98;
pub(super) const PROGRESS: u32 = 65;

/// Progress bar of five cells, as in `Interface_Exemple.txt`: each cell is 20 %, in steps
/// of 5 % shown as ` ░▒▓█`. `percent` is clamped to 100.
pub(super) fn progress_bar(percent: u32) -> String {
    let steps = percent.min(100) / 5;
    let (full, part) = ((steps / 4) as usize, (steps % 4) as usize);
    (0..5)
        .map(|cell| {
            if cell < full {
                '█'
            } else if cell == full {
                [' ', '░', '▒', '▓'][part]
            } else {
                ' '
            }
        })
        .collect()
}

/// The StatusBar text for the given workbench.
pub(super) fn status_text(workbench: usize) -> String {
    format!(
        " {DOCUMENT} │ {} │ {CONSTRAINT} │ {FPS}fps │ Progress [{}] {PROGRESS}%",
        WORKBENCHES
            .get(workbench)
            .copied()
            .unwrap_or(WORKBENCHES[0]),
        progress_bar(PROGRESS),
    )
}
