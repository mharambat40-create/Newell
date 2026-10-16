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

use crate::{
    DEFAULT_SCALE_PERCENT, MenuAction, MenuCheckbox, MenuCycle, MenuDef, MenuEntry, MenuItem,
    Mnemonic, SCALE_PERCENTS,
};

// Semantic action ids returned by the menus (`MenuAction`). Only the ones below the
// "effects" comment change shell state; the rest are accepted and ignored.
pub(super) const ACT_WORKBENCH: u32 = 100;
pub(super) const ACT_VIEW_PROJECTION: u32 = 200;
pub(super) const ACT_VIEW_SHADING: u32 = 201;
pub(super) const ACT_VIEW_SCALE: u32 = 202;
pub(super) const ACT_VIEW_CHECK: u32 = 210;
pub(super) const ACT_VIEW_THEME: u32 = 223;
pub(super) const ACT_VIEW_FULLSCREEN: u32 = 222;
pub(super) const ACT_EXTRUDE: u32 = 300;
pub(super) const ACT_REVOLVE: u32 = 301;
pub(super) const ACT_LOFT: u32 = 302;
pub(super) const ACT_SWEEP: u32 = 303;
pub(super) const ACT_BOOLEAN: u32 = 310;

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
// Commands remain UI-only mocks except the existing Action Box dispatch.
const DATUM: [MenuEntry<'static>; 4] = [
    item(290, "[S]ketch", "S"),
    item(340, "[Pl]ane", "Pl"),
    item(341, "[A]xis", "A"),
    item(342, "[P]oint", "P"),
];
const CREATE: [MenuEntry<'static>; 11] = [
    item(ACT_EXTRUDE, "[E]xtrude", "E"),
    item(ACT_REVOLVE, "[R]evolve", "R"),
    item(ACT_LOFT, "[L]oft", "L"),
    item(ACT_SWEEP, "[S]weep", "S"),
    MenuEntry::Separator,
    item(360, "Hole", ""),
    item(361, "Thread", ""),
    item(362, "Thicken", ""),
    MenuEntry::Separator,
    item(363, "Helix", ""),
    item(364, "Primitive Body", ""),
];
const SHAPE: [MenuEntry<'static>; 8] = [
    item(365, "Projected Curve", ""),
    item(366, "Bridging Curve", ""),
    item(367, "Composit Curve", ""),
    item(368, "Intersect Curve", ""),
    MenuEntry::Separator,
    item(369, "Offset Surface", ""),
    item(370, "Fill Surface", ""),
    item(371, "Constrained", ""),
];
const MODIFY: [MenuEntry<'static>; 13] = [
    item(330, "[F]illet", "F"),
    item(331, "[C]hamfer", "C"),
    item(332, "Shell", "S"),
    item(333, "Draft", "D"),
    item(372, "Face Blend", ""),
    MenuEntry::Separator,
    item(ACT_BOOLEAN, "Boolean", "B"),
    item(311, "Split Face", "SF"),
    item(312, "Split Body", "SB"),
    MenuEntry::Separator,
    item(313, "Transform", "T"),
    item(373, "Delete Face", ""),
    item(374, "Move Face", ""),
];
const PATTERN: [MenuEntry<'static>; 4] = [
    item(320, "Mirror", "Mi"),
    item(321, "Linear", "Li"),
    item(322, "Circular", "Ci"),
    item(323, "Curve", "Cu"),
];
const UTILITIES: [MenuEntry<'static>; 4] = [
    item(350, "[M]esure", "M"),
    item(351, "Split View", "SV"),
    item(352, "Curvature", ""),
    item(353, "Properties", ""),
];

/// Width of every CommandBar popup frame in `Command_Bar.txt`.
pub(super) const COMMAND_POPUP_WIDTH: u32 = 19;

/// The seven CommandBar groups in corrected reference order.
pub(super) static COMMAND_MENUS: [MenuDef<'static>; 7] = [
    menu("Workbench", "", &WORKBENCH),
    menu("Datum", "", &DATUM),
    menu("Create", "", &CREATE),
    menu("Shape", "", &SHAPE),
    menu("Modify", "", &MODIFY),
    menu("Pattern", "", &PATTERN),
    menu("Utilities", "", &UTILITIES),
];

pub(super) const PROJECTION: [&str; 2] = ["Perspective", "Orthographic"];
// `Wireframe ` carries a trailing space: the reference row `◀  Wireframe    ▶` sits two cells
// from `◀` and four from `▶`, one cell left of where centring would put it (an irregularity
// of `Application_Menu_Bar.txt`, reproduced as written).
pub(super) const SHADING: [&str; 6] = [
    "Points",
    "Wireframe ",
    "Hidden line",
    "No shading",
    "Shaded",
    "Flat line",
];
/// The UI Scale values as the menu shows them: right-aligned to four cells (` 60%`, `100%`).
pub(super) const SCALE_LABELS: [&str; 4] = [" 60%", " 80%", "100%", "120%"];
const _: () = assert!(SCALE_PERCENTS.len() == SCALE_LABELS.len());

/// Index of `percent` in [`SCALE_PERCENTS`]; the default for an unknown value.
fn scale_index(percent: u32) -> usize {
    SCALE_PERCENTS
        .iter()
        .position(|&p| p == percent)
        .or_else(|| {
            SCALE_PERCENTS
                .iter()
                .position(|&p| p == DEFAULT_SCALE_PERCENT)
        })
        .unwrap_or(0)
}

/// The two UI themes as the user sees them (`ARCHITECTURE.md` §3.6): Light is black on
/// white, Dark is white on black. The renderer maps this to its colour pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    #[default]
    Light,
    Dark,
}

impl ThemeChoice {
    /// The other theme.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }

    /// The View menu row: `Theme [ ◐ Light ]` / `Theme [ ◑ Dark  ]`.
    const fn row(self) -> &'static str {
        match self {
            Self::Light => "Theme [ ◐ Light ]",
            Self::Dark => "Theme [ ◑ Dark  ]",
        }
    }
}

pub(super) const VIEW_CHECKS: usize = 5;
const VIEW_CHECK_LABELS: [&str; VIEW_CHECKS] = [
    "Tree panel",
    "Task bar",
    "Status bar",
    "Navigation cube",
    "Grid Overlay",
];

/// The mutable rows of the View menu: the projection and shading mocks, the five visibility
/// checkboxes, and the user's UI Scale preference and theme. The mocks change only what the
/// menu shows (README §7.4); the preference and theme are read by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ViewMock {
    pub projection: usize,
    pub shading: usize,
    /// Index into [`SCALE_PERCENTS`]: the stored `preferred_scale_percent`.
    pub scale: usize,
    pub theme: ThemeChoice,
    pub checks: [bool; VIEW_CHECKS],
}

impl ViewMock {
    pub(super) fn scale_percent(&self) -> u32 {
        SCALE_PERCENTS
            .get(self.scale)
            .copied()
            .unwrap_or(DEFAULT_SCALE_PERCENT)
    }
}

impl Default for ViewMock {
    fn default() -> Self {
        Self {
            projection: 0,
            shading: 4,
            scale: scale_index(DEFAULT_SCALE_PERCENT),
            theme: ThemeChoice::Light,
            checks: [true, true, true, true, false],
        }
    }
}

fn view_entries(v: &ViewMock) -> [MenuEntry<'static>; 12] {
    let cycle =
        |action: u32, label: &'static str, values: &'static [&'static str], current: usize| {
            MenuEntry::Cyclic(MenuCycle {
                action: MenuAction::new(action),
                label,
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
        cycle(ACT_VIEW_PROJECTION, "", &PROJECTION, v.projection),
        cycle(ACT_VIEW_SHADING, "", &SHADING, v.shading),
        cycle(ACT_VIEW_SCALE, "UI Scale", &SCALE_LABELS, v.scale),
        item(ACT_VIEW_THEME, v.theme.row(), ""),
        check(0),
        check(1),
        check(2),
        check(3),
        check(4),
        item(220, "[N]ormal to", "N"),
        item(221, "Reset Cam", ""),
        item(ACT_VIEW_FULLSCREEN, "Fullscreen", ""),
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

/// What a tree row is. A category header (`Features`, `Solid Bodies(2)`, ...) is structure:
/// it is not renamed and sits directly after its marker. A named row is a user-named Feature or
/// Element (`README_UI.md` §23, §26).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NodeKind {
    /// `counted`: the label is followed by `(N)`, N being the number of its children.
    Category {
        counted: bool,
    },
    Named,
}

/// One node of a mock tree, in a flat pre-order list: nesting is the `depth` field, not
/// a pointer structure. Whether a node is a branch follows from the list: it is one when the
/// next node is deeper ([`has_children`]).
#[derive(Debug, Clone, Copy)]
pub(super) struct Node {
    pub label: &'static str,
    pub depth: u32,
    pub kind: NodeKind,
    /// Initial visibility checkbox (`■` shown / `□` hidden); `None`: the row has no checkbox.
    pub check: Option<bool>,
    /// Initial expansion of a branch.
    pub open: bool,
}

const fn category(label: &'static str, depth: u32, counted: bool, open: bool) -> Node {
    Node {
        label,
        depth,
        kind: NodeKind::Category { counted },
        check: None,
        open,
    }
}

const fn named(label: &'static str, depth: u32, check: Option<bool>, open: bool) -> Node {
    Node {
        label,
        depth,
        kind: NodeKind::Named,
        check,
        open,
    }
}

/// Maximum nodes per tree: the id range reserved for each tree.
pub(super) const MAX_NODES: usize = 64;

pub(super) const FEATURES: [Node; 9] = [
    category("Features", 0, false, true),
    named("Sketch_Profile_Main", 1, Some(false), false),
    named("Hub_Features", 1, None, true),
    named("Extrude_Base", 2, None, false),
    named("Sketch_Hole", 2, Some(true), false),
    named("Extrude_Cut_Center", 2, None, false),
    named("Fillet_Inner", 1, None, false),
    named("Chamfer_Edges", 1, None, false),
    named("Pattern_Circular", 1, None, false),
];

pub(super) const ELEMENTS: [Node; 7] = [
    category("Elements", 0, false, true),
    category("Solid Bodies", 1, true, true),
    named("Rim_Body", 2, Some(true), false),
    named("Hub_Body", 2, Some(false), false),
    category("Surfaces", 1, true, false),
    category("Construct", 1, false, false),
    named("Plane_XY", 2, Some(false), false),
];

/// Features before the playhead separator in the reference (`Interface_Exemple.txt`: after
/// `Fillet_Inner`, the third top-level feature).
pub(super) const DEFAULT_PLAYHEAD: usize = 3;

/// Initial expansion flags of a tree.
pub(super) fn initial_open(nodes: &[Node]) -> [bool; MAX_NODES] {
    let mut open = [false; MAX_NODES];
    for (slot, n) in open.iter_mut().zip(nodes) {
        *slot = n.open;
    }
    open
}

/// Initial visibility checkboxes of a tree (`false` for rows without one).
pub(super) fn initial_checks(nodes: &[Node]) -> [bool; MAX_NODES] {
    let mut checks = [false; MAX_NODES];
    for (slot, n) in checks.iter_mut().zip(nodes) {
        *slot = n.check.unwrap_or(false);
    }
    checks
}

/// `true` if node `index` has children: the next node is deeper.
pub(super) fn has_children(nodes: &[Node], index: usize) -> bool {
    match (nodes.get(index), nodes.get(index + 1)) {
        (Some(node), Some(next)) => next.depth > node.depth,
        _ => false,
    }
}

/// Number of direct children of node `index`.
pub(super) fn child_count(nodes: &[Node], index: usize) -> usize {
    let Some(node) = nodes.get(index) else {
        return 0;
    };
    nodes[index + 1..]
        .iter()
        .take_while(|n| n.depth > node.depth)
        .filter(|n| n.depth == node.depth + 1)
        .count()
}

/// The full text of node `index`: its renamed text if the user gave one, else its label,
/// followed by `(N)` for a counted category. Counts are derived from the tree, never stored.
pub(super) fn full_label(nodes: &[Node], index: usize, renamed: Option<&str>) -> String {
    let Some(node) = nodes.get(index) else {
        return String::new();
    };
    if let Some(name) = renamed {
        return name.to_owned();
    }
    match node.kind {
        NodeKind::Category { counted: true } => {
            format!("{}({})", node.label, child_count(nodes, index))
        }
        _ => node.label.to_owned(),
    }
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
        if has_children(nodes, i) && !open.get(i).copied().unwrap_or(false) {
            hidden_below = Some(n.depth);
        }
    }
    rows
}

/// One display line of a tree panel: a node row, or the playhead separator of the Features
/// tree (`│ ─────────────────── `), which is not a node and has no index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Line {
    Node(Row),
    Playhead,
}

/// Number of top-level nodes (depth 1) of a tree.
pub(super) fn top_level_count(nodes: &[Node]) -> usize {
    nodes.iter().filter(|n| n.depth == 1).count()
}

/// Brings a wished playhead position into the valid range `1..=top - 1`: the separator sits
/// between two top-level features, as in the reference. `None` if there are fewer than two.
pub(super) fn clamp_playhead(gap: usize, top: usize) -> Option<usize> {
    (top >= 2).then(|| gap.clamp(1, top - 1))
}

/// The lines currently shown. With `playhead = Some(g)` the separator line is inserted
/// directly before the top-level node that has `g` top-level nodes (and their subtrees) before
/// it. Nesting is untouched: the separator is a line, never a node.
pub(super) fn visible_lines(nodes: &[Node], open: &[bool], playhead: Option<usize>) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut top_seen = 0;
    for row in visible_rows(nodes, open) {
        if nodes.get(row.index).is_some_and(|n| n.depth == 1) {
            if playhead == Some(top_seen) {
                lines.push(Line::Playhead);
            }
            top_seen += 1;
        }
        lines.push(Line::Node(row));
    }
    lines
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
