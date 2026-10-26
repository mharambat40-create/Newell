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
    DEFAULT_SCALE_FACTOR_PERCENT, MenuAction, MenuCheckbox, MenuCycle, MenuDef, MenuEntry,
    MenuItem, Mnemonic, SCALE_FACTORS_PERCENT,
};

// Semantic action ids returned by the menus (`MenuAction`). Only the ones below the
// "effects" comment change shell state; the rest are accepted and ignored.
pub(super) const ACT_WORKBENCH: u32 = 100;
pub(super) const ACT_VIEW_PROJECTION: u32 = 200;
pub(super) const ACT_VIEW_SHADING: u32 = 201;
pub(super) const ACT_VIEW_SCALE: u32 = 202;
pub(super) const ACT_VIEW_CHECK: u32 = 210;
pub(super) const ACT_VIEW_THEME: u32 = 223;
pub(super) const ACT_VIEW_COLOR: u32 = 224;
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
        action_offsets: &[],
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
pub(super) const SHADING: [&str; 7] = [
    "Points",
    "Line",
    "Wireframe ",
    "Hidden Line",
    "No shading",
    "Shaded",
    "Default",
];
/// UI Scale labels, paired by index with the legacy rendering multipliers.
pub(super) const SCALE_LABELS: [&str; 4] = [" 80%", "100%", "120%", "140%"];
const _: () = assert!(SCALE_FACTORS_PERCENT.len() == SCALE_LABELS.len());

/// Index of the rendering multiplier in [`SCALE_FACTORS_PERCENT`]; default if unknown.
fn scale_index(scale_factor_percent: u32) -> usize {
    SCALE_FACTORS_PERCENT
        .iter()
        .position(|&p| p == scale_factor_percent)
        .or_else(|| {
            SCALE_FACTORS_PERCENT
                .iter()
                .position(|&p| p == DEFAULT_SCALE_FACTOR_PERCENT)
        })
        .unwrap_or(0)
}

/// Per-window Theme choice, independent of Color (`ARCHITECTURE.md` §3.6).
/// Unchecked Color uses black on white in Light and white on black in Dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    Light,
    #[default]
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

    /// The View menu row: `Theme [ ◑ Dark  ]` / `Theme [ ◐ Light ]`.
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

/// Window-local View choices. Color is separate from the five visibility checkboxes;
/// the application synchronizes these presentation choices without editing document data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ViewMock {
    pub projection: usize,
    pub shading: usize,
    /// Index pairing the displayed label with its rendering multiplier.
    pub scale: usize,
    pub theme: ThemeChoice,
    pub color_enabled: bool,
    pub checks: [bool; VIEW_CHECKS],
}

impl ViewMock {
    pub(super) fn scale_factor_percent(&self) -> u32 {
        SCALE_FACTORS_PERCENT
            .get(self.scale)
            .copied()
            .unwrap_or(DEFAULT_SCALE_FACTOR_PERCENT)
    }
}

impl Default for ViewMock {
    fn default() -> Self {
        Self {
            projection: 0,
            shading: 2,
            scale: scale_index(DEFAULT_SCALE_FACTOR_PERCENT),
            theme: ThemeChoice::Dark,
            color_enabled: false,
            checks: [true, true, true, true, false],
        }
    }
}

fn view_entries(v: &ViewMock) -> [MenuEntry<'static>; 13] {
    let cycle =
        |action: u32, label: &'static str, values: &'static [&'static str], current: usize| {
            MenuEntry::Cyclic(MenuCycle {
                action: MenuAction::new(action),
                label,
                values,
                current,
                bounded: action == ACT_VIEW_SCALE,
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
        MenuEntry::Item(MenuItem {
            action: MenuAction::new(ACT_VIEW_THEME),
            label: v.theme.row(),
            mnemonic: None,
            enabled: true,
            action_offsets: &[6, 8, 16],
        }),
        MenuEntry::Checkbox(MenuCheckbox {
            action: MenuAction::new(ACT_VIEW_COLOR),
            label: "Color",
            checked: v.color_enabled,
            mnemonic: None,
            enabled: true,
        }),
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

/// A derived rendering row. Identity and values come from the published projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NodeKind {
    Category { counted: bool },
    Named,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Node {
    pub label: String,
    pub depth: u32,
    pub kind: NodeKind,
    pub check: Option<bool>,
    pub target: super::TreeTarget,
}
pub(super) fn projection_nodes(projection: &super::ModelTreeProjection) -> [Vec<Node>; 2] {
    use super::{ElementGroup, FeatureNodeId, TreeTargetKind as K};
    let node = |label: String, depth, kind, check, target| Node {
        label,
        depth,
        kind,
        check,
        target: projection.target(target),
    };
    let category = |counted| NodeKind::Category { counted };
    let mut features = vec![node(
        "Features".into(),
        0,
        category(false),
        None,
        K::FeaturesRoot,
    )];
    features.extend(projection.features.iter().map(|item| {
        node(
            item.name.clone(),
            item.depth,
            NodeKind::Named,
            item.visibility,
            match item.id {
                FeatureNodeId::Operation(id) => K::Operation(id),
                FeatureNodeId::Folder(id) => K::Folder(id),
            },
        )
    }));
    let mut elements = vec![node(
        "Elements".into(),
        0,
        category(false),
        None,
        K::ElementsRoot,
    )];
    for (group, label, counted) in [
        (ElementGroup::Bodies, &projection.labels.bodies, true),
        (ElementGroup::Surfaces, &projection.labels.surfaces, true),
        (ElementGroup::Construct, &projection.labels.construct, false),
    ] {
        elements.push(node(
            label.clone(),
            1,
            category(counted),
            None,
            K::Group(group),
        ));
        elements.extend(
            projection
                .elements
                .iter()
                .filter(|item| item.group == group)
                .map(|item| {
                    node(
                        item.name.clone(),
                        2,
                        NodeKind::Named,
                        Some(item.visible),
                        K::Entity(item.id),
                    )
                }),
        );
    }
    [features, elements]
}

/// Historical reference artwork, represented through the same contract as live documents.
pub(super) fn reference_trees() -> (super::ModelTreeProjection, super::TreeInitialState) {
    use super::*;
    let operation = |id| FeatureNodeId::Operation(OpaqueTreeId::new(id));
    let folder = FeatureNodeId::Folder(OpaqueTreeId::new(1));
    let mut features = Vec::new();
    for (id, name, depth, visibility) in [
        (1, "Sketch_Profile_Main", 1, Some(false)),
        (2, "Extrude_Base", 2, None),
        (3, "Sketch_Hole", 2, Some(true)),
        (4, "Extrude_Cut_Center", 2, None),
        (5, "Fillet_Inner", 1, None),
        (6, "Chamfer_Edges", 1, None),
        (7, "Pattern_Circular", 1, None),
    ] {
        if id == 2 {
            features.push(FeatureTreeItem {
                id: folder,
                name: "Hub_Features".into(),
                depth: 1,
                visibility: None,
            });
        }
        features.push(FeatureTreeItem {
            id: operation(id),
            name: name.into(),
            depth,
            visibility,
        });
    }
    let elements = [
        (1, 2, ElementGroup::Bodies, "Rim_Body", true),
        (2, 4, ElementGroup::Bodies, "Hub_Body", false),
        (3, 1, ElementGroup::Construct, "Plane_XY", false),
    ]
    .into_iter()
    .map(|(id, source, group, name, visible)| ElementTreeItem {
        id: OpaqueTreeId::new(id),
        source_feature: OpaqueTreeId::new(source),
        output_ordinal: 0,
        group,
        name: name.into(),
        visible,
    })
    .collect();
    let projection = ModelTreeProjection {
        document: TreeDocumentId::new(0),
        features,
        elements,
        labels: TreeLabels {
            bodies: "Solid Bodies".into(),
            ..TreeLabels::default()
        },
    };
    let mut initial = projection.initial_state();
    initial
        .expanded
        .insert(projection.target(TreeTargetKind::Folder(OpaqueTreeId::new(1))));
    let gap = |id, display| PlacedHistoryGap {
        insertion: HistoryGap::Before(OpaqueTreeId::new(id)),
        display,
    };
    initial.playhead = gap(6, GapDisplayLocation::BeforeNode(operation(6)));
    initial.gap_policy = HistoryGapPolicy::ExplicitGaps(vec![
        gap(2, GapDisplayLocation::BeforeNode(folder)),
        gap(5, GapDisplayLocation::BeforeNode(operation(5))),
        gap(6, GapDisplayLocation::BeforeNode(operation(6))),
        gap(7, GapDisplayLocation::BeforeNode(operation(7))),
    ]);
    (projection, initial)
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

/// Mock StatusBar values (deterministic: no live frame rate).
pub(super) const DOCUMENT: &str = "Roue V1*";
pub(super) const CONSTRAINT: &str = "Fully Constrained";

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

/// A fixed seven-cell FPS field. Invalid values map to zero; finite values saturate.
pub(super) fn fps_field(value: f64) -> String {
    let fps = if value.is_finite() && value >= 0.0 {
        value.round().min(999.0) as u32
    } else {
        0
    };
    format!("{fps:>3}fps ")
}

/// Fixed 21-cell interactive progress segment, including trailing percentage padding.
pub(super) fn progress_segment(percent: u32) -> String {
    let percent = percent.min(100);
    format!(
        "Progress [{}] {:<4}",
        progress_bar(percent),
        format!("{percent}%")
    )
}

/// The StatusBar text with externally supplied runtime values.
pub(super) fn status_text(workbench: usize, fps: u32, progress: u32) -> String {
    format!(
        " {DOCUMENT} │ {} │ {CONSTRAINT} │{}│ {}",
        if workbench == 0 {
            "Part Design"
        } else {
            WORKBENCHES.get(workbench).copied().unwrap_or("Part Design")
        },
        fps_field(f64::from(fps)),
        progress_segment(progress),
    )
}
