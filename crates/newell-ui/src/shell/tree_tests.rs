// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M2.20: the NavigationPanel golden fixture, the Features /
//! Elements divider, the playhead and the Rename flow.
use super::data::{self, Line, Node, NodeKind, child_count, full_label};
use super::*;
use crate::{CellContent, CellGrid, GridMetrics, UiPoint, hit_test};

#[path = "../../tests/support/reference_shell.rs"]
mod reference_shell;

fn metrics() -> GridMetrics {
    GridMetrics::new(6, 10).unwrap()
}

trait ReferenceInput {
    fn fixture_key(&mut self, key: ShellKey) -> bool;
    fn fixture_click(&mut self, hit: Option<HitTestResult>) -> bool;
}
impl ReferenceInput for ShellState {
    fn fixture_key(&mut self, key: ShellKey) -> bool {
        let generation = ProjectionGeneration::new(0);
        let tree = self
            .interaction
            .focused
            .and_then(|id| self.tree_hit(id, generation).map(|token| (id, token)));
        let changed = if matches!(key, ShellKey::Enter | ShellKey::Char(' '))
            && let Some((id, token)) = tree
        {
            self.activate_tree_hit(id, token, generation, true)
        } else {
            self.key(key)
        };
        respond_fixture(self);
        changed
    }
    fn fixture_click(&mut self, hit: Option<HitTestResult>) -> bool {
        let changed = published_click(self, hit);
        respond_fixture(self);
        changed
    }
}
pub(super) fn published_click(shell: &mut ShellState, hit: Option<HitTestResult>) -> bool {
    let generation = ProjectionGeneration::new(0);
    if let Some(id) = hit.and_then(|hit| hit.widget_id)
        && let Some(token) = shell.tree_hit(id, generation)
    {
        shell.activate_tree_hit(id, token, generation, false)
    } else {
        shell.click(hit)
    }
}
pub(super) fn respond_fixture(shell: &mut ShellState) {
    let Some((target, edit)) = shell.take_tree_edit() else {
        return;
    };
    if matches!(edit, TreeIntentKind::Select { .. }) {
        return;
    }
    let mut projection = shell.tree_projection().clone();
    let mut supported = true;
    match (target.kind, edit) {
        (TreeTargetKind::Operation(id), TreeIntentKind::Rename { name, .. }) => {
            projection
                .features
                .iter_mut()
                .find(|item| item.id == FeatureNodeId::Operation(id))
                .unwrap()
                .name = name
        }
        (TreeTargetKind::Folder(id), TreeIntentKind::Rename { name, .. }) => {
            projection
                .features
                .iter_mut()
                .find(|item| item.id == FeatureNodeId::Folder(id))
                .unwrap()
                .name = name
        }
        (TreeTargetKind::Entity(id), TreeIntentKind::Rename { name, .. }) => {
            projection
                .elements
                .iter_mut()
                .find(|item| item.id == id)
                .unwrap()
                .name = name
        }
        (TreeTargetKind::Operation(id), TreeIntentKind::SetVisibility { visible, .. }) => {
            projection
                .features
                .iter_mut()
                .find(|item| item.id == FeatureNodeId::Operation(id))
                .unwrap()
                .visibility = Some(visible)
        }
        (TreeTargetKind::Entity(id), TreeIntentKind::SetVisibility { visible, .. }) => {
            projection
                .elements
                .iter_mut()
                .find(|item| item.id == id)
                .unwrap()
                .visible = visible
        }
        _ => supported = false,
    }
    assert!(
        supported,
        "fixture edit must have a supported semantic target"
    );
    shell.set_tree_projection(projection).unwrap();
}

fn text(g: &CellGrid, row: u32) -> String {
    (0..g.cols())
        .map(|col| match g.get(col, row).unwrap().content {
            CellContent::Glyph(ch) => ch,
            CellContent::Empty => ' ',
        })
        .collect()
}

fn find(g: &CellGrid, needle: &str) -> Option<(u32, u32)> {
    let pat: Vec<char> = needle.chars().collect();
    (0..g.rows()).find_map(|row| {
        let line: Vec<char> = text(g, row).chars().collect();
        line.windows(pat.len())
            .position(|w| w == pat.as_slice())
            .map(|col| (u32::try_from(col).unwrap(), row))
    })
}

fn hit_at(g: &CellGrid, col: u32, row: u32) -> Option<HitTestResult> {
    hit_test(
        UiPoint {
            x: col * 6 + 3,
            y: row * 10 + 5,
        },
        metrics(),
        g,
    )
}

fn click_text(s: &mut ShellState, needle: &str) -> bool {
    let g = s.compose().unwrap();
    let (col, row) = find(&g, needle).expect("text must be on screen");
    s.fixture_click(hit_at(&g, col, row))
}

fn click_tree_marker(s: &mut ShellState, target: TreeTarget) -> bool {
    let grid = s.compose().unwrap();
    let hit = (0..grid.rows())
        .find_map(|row| {
            (0..grid.cols()).find_map(|col| {
                let hit = hit_at(&grid, col, row)?;
                let token = s.tree_hit(hit.widget_id?, ProjectionGeneration::new(0))?;
                (token.target == target && token.part == TreePart::Marker).then_some(hit)
            })
        })
        .expect("the semantic expansion marker is visible");
    s.fixture_click(Some(hit))
}

/// The id under the first cell of `needle`.
fn id_of(s: &ShellState, needle: &str) -> WidgetId {
    let g = s.compose().unwrap();
    let (col, row) = find(&g, needle).unwrap();
    hit_at(&g, col, row).and_then(|h| h.widget_id).unwrap()
}

fn type_str(s: &mut ShellState, t: &str) {
    for c in t.chars() {
        s.fixture_key(ShellKey::Char(c));
    }
}

/// Columns `0..25` of `row`: the NavigationPanel with its right border.
fn panel(g: &CellGrid, row: u32) -> String {
    text(g, row).chars().take(25).collect()
}

fn reference_panel() -> Vec<String> {
    reference_shell::rows(28)
        .iter()
        .skip(4)
        .take(22)
        .map(|l| l.chars().take(25).collect())
        .collect()
}

fn ref_shell() -> ShellState {
    ShellState::new(82, 28)
}

fn live_projection(count: u64) -> ModelTreeProjection {
    let mut projection = ModelTreeProjection::empty(TreeDocumentId::new(1));
    projection.features = (0..count)
        .map(|id| FeatureTreeItem {
            id: FeatureNodeId::Operation(OpaqueTreeId::new(id)),
            name: format!("operation_{id}"),
            depth: 1,
            visibility: None,
        })
        .collect();
    projection
}
fn live_shell(projection: ModelTreeProjection) -> ShellState {
    let mut shell = ShellState::new_application(82, 30);
    shell.set_tree_projection(projection).unwrap();
    shell
}

#[test]
fn live_empty_and_single_operation_have_end_cursor_and_empty_groups() {
    for count in [0, 1] {
        let mut s = live_shell(live_projection(count));
        assert_eq!(s.playhead(), PlacedHistoryGap::default());
        assert!(s.tree_lines(Tree::Features).contains(&Line::Playhead));
        assert!(has_text(&s, "Bodies(0)"));
        assert!(has_text(&s, "Surfaces(0)"));
        assert!(has_text(&s, "Construct"));
        assert!(!has_text(&s, "Solid Bodies"));
        assert!(!has_text(&s, "Hub_Features"));
        assert_eq!(has_children(&s.nodes[0], 0), count != 0);
        assert!(!has_children(&s.nodes[1], 1));
        s.drag_separator(Separator::Playhead, 5, 5);
        assert_eq!(
            s.playhead().insertion,
            if count == 0 {
                HistoryGap::End
            } else {
                HistoryGap::Before(OpaqueTreeId::new(0))
            },
            "dropping on the root is above the first history item"
        );
        s.drag_separator(Separator::Playhead, 5, 100);
        assert_eq!(s.playhead(), PlacedHistoryGap::default());
    }
}

#[test]
fn live_grouping_filters_output_stream_without_sorting_or_hiding_entities() {
    let mut projection = live_projection(2);
    projection.elements = [
        (10, 1, 0, ElementGroup::Surfaces, "s", true),
        (20, 0, 1, ElementGroup::Bodies, "z", false),
        (30, 0, 0, ElementGroup::Construct, "p", true),
        (40, 1, 1, ElementGroup::Bodies, "a", true),
    ]
    .into_iter()
    .map(
        |(id, source, ordinal, group, name, visible)| ElementTreeItem {
            id: OpaqueTreeId::new(id),
            source_feature: OpaqueTreeId::new(source),
            output_ordinal: ordinal,
            group,
            name: name.into(),
            visible,
        },
    )
    .collect();
    let s = live_shell(projection);
    assert_eq!(
        s.nodes[1]
            .iter()
            .map(|node| node.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Elements",
            "Bodies",
            "z",
            "a",
            "Surfaces",
            "s",
            "Construct",
            "p"
        ]
    );
    assert_eq!(full_label(&s.nodes[1], 1, None), "Bodies(2)");
    assert_eq!(full_label(&s.nodes[1], 4, None), "Surfaces(1)");
    assert_eq!(s.nodes[1][2].check, Some(false));
    assert_eq!(s.nodes[1][3].check, Some(true));
    assert!(
        !s.expanded.contains(
            &s.projection
                .target(TreeTargetKind::Group(ElementGroup::Surfaces))
        )
    );
}

#[test]
fn insertion_anchor_and_structural_location_are_distinct_at_folder_boundaries() {
    let mut projection = live_projection(2);
    let a = FeatureNodeId::Folder(OpaqueTreeId::new(100));
    let b = FeatureNodeId::Folder(OpaqueTreeId::new(200));
    projection.features[0].depth = 2;
    projection.features[1].depth = 2;
    projection.features.insert(
        0,
        FeatureTreeItem {
            id: a,
            name: "A".into(),
            depth: 1,
            visibility: None,
        },
    );
    projection.features.insert(
        2,
        FeatureTreeItem {
            id: b,
            name: "B".into(),
            depth: 1,
            visibility: None,
        },
    );
    let gap = projection
        .placed_gap(GapDisplayLocation::AfterSubtree(a))
        .unwrap();
    let equivalent = projection
        .placed_gap(GapDisplayLocation::BeforeNode(b))
        .unwrap();
    assert_eq!(gap.insertion, HistoryGap::Before(OpaqueTreeId::new(1)));
    assert_eq!(equivalent.insertion, gap.insertion);
    assert_ne!(equivalent.display, gap.display);
    let mut s = live_shell(projection);
    assert!(s.set_playhead(gap).unwrap());
    let lines = s.tree_lines(Tree::Features);
    let separator = lines
        .iter()
        .position(|line| *line == Line::Playhead)
        .unwrap();
    assert!(matches!(&lines[separator - 1], Line::Node(row) if s.nodes[0][row.index].label == "A"));
    assert!(matches!(&lines[separator + 1], Line::Node(row) if s.nodes[0][row.index].label == "B"));
    s.drag_separator(Separator::Playhead, 3, 6);
    assert_eq!(
        s.playhead(),
        gap,
        "dropping on A selects after A's whole subtree"
    );
    let mut refreshed = s.projection.clone();
    refreshed.features[0].name = "renamed A".into();
    s.set_tree_projection(refreshed).unwrap();
    assert_eq!(s.playhead(), gap);
    let first = s
        .projection
        .placed_gap(GapDisplayLocation::BeforeNode(a))
        .unwrap();
    s.set_playhead(first).unwrap();
    assert_eq!(
        s.playhead().insertion,
        HistoryGap::Before(OpaqueTreeId::new(0))
    );
    s.set_playhead(PlacedHistoryGap::default()).unwrap();
    assert_eq!(s.playhead().insertion, HistoryGap::End);
}

#[test]
fn projection_reorder_and_removal_reconcile_selection_focus_cursor_and_rename_by_identity() {
    let mut s = live_shell(live_projection(4));
    click_text(&mut s, "operation_1");
    let target = s.tree_focus.unwrap();
    let gap = s
        .projection
        .placed_gap(GapDisplayLocation::BeforeNode(FeatureNodeId::Operation(
            OpaqueTreeId::new(1),
        )))
        .unwrap();
    s.set_playhead(gap).unwrap();
    let mut reordered = s.projection.clone();
    reordered.features.swap(1, 3);
    s.set_tree_projection(reordered).unwrap();
    assert!(
        s.entity_selection.entities.is_empty(),
        "a zero-output operation selects no entity"
    );
    assert_eq!(s.tree_focus, Some(target));
    assert_eq!(s.playhead(), gap);
    assert!(s.begin_rename(id_of(&s, "operation_1")));
    let mut removed = s.projection.clone();
    removed
        .features
        .retain(|item| item.id != FeatureNodeId::Operation(OpaqueTreeId::new(1)));
    s.set_tree_projection(removed).unwrap();
    assert!(!s.rename().is_open());
    assert!(s.take_tree_edit().is_none());
    assert!(s.entity_selection.entities.is_empty() && s.tree_focus.is_none());
    assert_eq!(
        s.playhead(),
        PlacedHistoryGap::default(),
        "removed last operation has no old successor"
    );
}

#[test]
fn replacement_does_not_reuse_semantics_or_presentation_even_when_raw_ids_repeat() {
    let mut s = live_shell(live_projection(3));
    s.set_nav_width(30);
    click_text(&mut s, "operation_0");
    s.begin_rename(id_of(&s, "operation_0"));
    let mut replaced = live_projection(3);
    replaced.document = TreeDocumentId::new(2);
    s.set_tree_projection(replaced).unwrap();
    assert_eq!(s.nav_width(), 18);
    assert!(s.entity_selection.entities.is_empty() && s.tree_focus.is_none());
    assert!(!s.rename().is_open());
    assert!(s.take_tree_edit().is_none());
    assert_eq!(s.playhead(), PlacedHistoryGap::default());
}

#[test]
fn scrolling_reuses_bounded_visible_slots_without_losing_semantic_state() {
    let mut s = live_shell(live_projection(200));
    click_text(&mut s, "operation_0");
    let selected = s.entity_selection.entities.clone();
    let old = s.projection.target(TreeTargetKind::FeaturesRoot);
    assert_eq!(
        s.tree_hit(tree_id(Tree::Features, 0), ProjectionGeneration::new(0))
            .unwrap()
            .target,
        old
    );
    assert_eq!(s.scroll_tree_at(5, 8, -100), Some(true));
    assert!(
        s.interaction.focused.is_none(),
        "wheel may leave semantic focus offscreen"
    );
    assert_eq!(s.entity_selection.entities, selected);
    assert!(s.tree_slots[0].len() <= s.layout().features.inset(1).height() as usize);
    let current = s
        .tree_hit(tree_id(Tree::Features, 0), ProjectionGeneration::new(1))
        .unwrap()
        .target;
    assert_ne!(current, old);
    s.key(ShellKey::Enter);
    assert_eq!(s.entity_selection.entities, selected);
    s.key(ShellKey::Tab);
    assert_eq!(s.tree_focus, Some(current));
    let old_top = s.nodes[0][s.tree_slots[0][0]].target;
    let mut refreshed = s.projection.clone();
    refreshed.features.insert(
        0,
        FeatureTreeItem {
            id: FeatureNodeId::Operation(OpaqueTreeId::new(999)),
            name: "inserted".into(),
            depth: 1,
            visibility: None,
        },
    );
    s.set_tree_projection(refreshed).unwrap();
    assert_eq!(s.nodes[0][s.tree_slots[0][0]].target, old_top);
}

#[test]
fn invalid_projection_is_rejected_without_changing_any_state() {
    let mut s = live_shell(live_projection(2));
    for invalid in [
        {
            let mut p = s.projection.clone();
            p.features[1].id = p.features[0].id;
            p
        },
        {
            let mut p = s.projection.clone();
            p.features[1].depth = 2;
            p
        },
        {
            let mut p = s.projection.clone();
            p.elements.push(ElementTreeItem {
                id: OpaqueTreeId::new(1),
                source_feature: OpaqueTreeId::new(999),
                output_ordinal: 0,
                group: ElementGroup::Bodies,
                name: "bad".into(),
                visible: true,
            });
            p
        },
    ] {
        let before = s.clone();
        assert_eq!(
            s.set_tree_projection(invalid),
            Err(crate::UiError::InvalidTreeProjection)
        );
        assert_eq!(s, before);
    }
}

#[test]
fn nested_separator_uses_ancestor_guides_and_collapse_retains_internal_anchor() {
    let mut projection = live_projection(1);
    let folder = FeatureNodeId::Folder(OpaqueTreeId::new(10));
    projection.features[0].depth = 2;
    projection.features.insert(
        0,
        FeatureTreeItem {
            id: folder,
            name: "last_folder".into(),
            depth: 1,
            visibility: None,
        },
    );
    let gap = projection
        .placed_gap(GapDisplayLocation::BeforeNode(FeatureNodeId::Operation(
            OpaqueTreeId::new(0),
        )))
        .unwrap();
    let mut s = live_shell(projection);
    let folder_target = s
        .projection
        .target(TreeTargetKind::Folder(OpaqueTreeId::new(10)));
    s.expanded.insert(folder_target);
    s.set_playhead(gap).unwrap();
    assert!(has_text(&s, "   │ ─"));
    assert!(click_tree_marker(&mut s, folder_target));
    assert_eq!(s.playhead(), gap);
    assert!(!has_text(&s, "operation_0"));
    assert!(!s.tree_lines(Tree::Features).contains(&Line::Playhead));
    // The current content-based width cap leaves six label cells in this fixture.
    assert!(has_text(&s, "last_… ───║"));
}

#[test]
fn collapse_distinguishes_internal_last_child_gap_from_external_folder_boundary() {
    let mut projection = live_projection(2);
    let a = FeatureNodeId::Folder(OpaqueTreeId::new(10));
    let b = FeatureNodeId::Folder(OpaqueTreeId::new(11));
    for item in &mut projection.features {
        item.depth = 2;
    }
    for (index, id, name) in [(1, b, "folder_b"), (0, a, "folder_a")] {
        projection.features.insert(
            index,
            FeatureTreeItem {
                id,
                name: name.into(),
                depth: 1,
                visibility: None,
            },
        );
    }
    for (display, internal) in [
        (
            GapDisplayLocation::AfterSubtree(FeatureNodeId::Operation(OpaqueTreeId::new(0))),
            true,
        ),
        (GapDisplayLocation::AfterSubtree(a), false),
        (GapDisplayLocation::BeforeNode(b), false),
    ] {
        let gap = projection.placed_gap(display).unwrap();
        assert_eq!(gap.insertion, HistoryGap::Before(OpaqueTreeId::new(1)));
        let mut shell = live_shell(projection.clone());
        let a_target = shell
            .projection
            .target(TreeTargetKind::Folder(OpaqueTreeId::new(10)));
        shell.expanded.insert(a_target);
        shell.set_playhead(gap).unwrap();
        shell.rebuild_tree_slots();
        assert!(click_tree_marker(&mut shell, a_target));
        assert_eq!(shell.playhead(), gap, "collapse retains both components");
        assert_eq!(shell.inline_playhead_host(), internal.then_some(a_target));
        assert_eq!(
            shell.tree_lines(Tree::Features).contains(&Line::Playhead),
            !internal
        );
        assert!(click_tree_marker(&mut shell, a_target));
        assert_eq!(
            shell.playhead(),
            gap,
            "expansion restores the original location"
        );
        assert_eq!(shell.inline_playhead_host(), None);
        assert!(shell.tree_lines(Tree::Features).contains(&Line::Playhead));
    }
}

#[test]
fn refresh_scroll_anchor_skips_surviving_rows_hidden_by_new_folders() {
    for case in 0..3 {
        let mut shell = live_shell(live_projection(45));
        let anchor = if case == 2 { 40 } else { 12 };
        shell.tree_scroll[Tree::Features.slot()] = anchor + 1; // Features root precedes operations.
        assert!(shell.tree_focus.is_none());
        let mut projection = shell.projection.clone();
        let folder_index = if case == 1 {
            projection.features.remove(anchor);
            anchor
        } else {
            anchor
        };
        let hidden_count = if case == 2 { 5 } else { 1 };
        for item in &mut projection.features[folder_index..folder_index + hidden_count] {
            item.depth = 2;
        }
        projection.features.insert(
            folder_index,
            FeatureTreeItem {
                id: FeatureNodeId::Folder(OpaqueTreeId::new(100)),
                name: "new_collapsed_folder".into(),
                depth: 1,
                visibility: None,
            },
        );
        shell.set_tree_projection(projection).unwrap();
        let expected = match case {
            0 => 13,
            1 => 14,
            _ => 39,
        };
        let lines = shell.tree_lines(Tree::Features);
        let expected_line = lines
            .iter()
            .position(|line| {
                matches!(line,
            Line::Node(row) if shell.nodes[Tree::Features.slot()][row.index].target.kind
                == TreeTargetKind::Operation(OpaqueTreeId::new(expected)))
            })
            .unwrap();
        let height = shell.layout().features.inset(1).height() as usize;
        assert_eq!(
            shell.tree_scroll[Tree::Features.slot()],
            expected_line.min(lines.len().saturating_sub(height)),
            "case {case}"
        );
        assert!(shell.tree_focus.is_none());
    }
}

#[test]
fn removed_rename_target_clears_positional_focus_and_initial_gap_pairs_are_validated() {
    let mut s = live_shell(live_projection(2));
    s.begin_rename(id_of(&s, "operation_0"));
    let mut projection = s.projection.clone();
    projection.features.remove(0);
    s.set_tree_projection(projection).unwrap();
    assert!(s.interaction.focused.is_none());
    assert!(!s.rename().is_open());
    assert!(s.take_tree_edit().is_none());
    let (projection, mut initial) = data::reference_trees();
    initial.validate(&projection).unwrap();
    let HistoryGapPolicy::ExplicitGaps(gaps) = &mut initial.gap_policy else {
        return;
    };
    gaps[0].insertion = HistoryGap::Before(OpaqueTreeId::new(1));
    assert_eq!(
        initial.validate(&projection),
        Err(crate::UiError::InvalidTreeProjection)
    );
}

// ---- reference comparison ----

#[test]
fn navigation_panel_equals_the_reference_cell_for_cell_at_82x28() {
    let (projection, initial) = data::reference_trees();
    initial.validate(&projection).unwrap();
    let g = ref_shell().compose().unwrap();
    let expected = reference_panel();
    assert_eq!(expected.len(), 22);
    for (i, want) in expected.iter().enumerate() {
        let row = 4 + u32::try_from(i).unwrap();
        assert_eq!(&panel(&g, row), want, "row {row}");
    }
}

#[test]
fn extra_rows_belong_to_elements_and_the_features_pane_keeps_its_height() {
    let g28 = ref_shell().compose().unwrap();
    let g30 = ShellState::new(82, 30).compose().unwrap();
    // `README_UI.md` §25: with snapshot rows 26 and 27 (0-based 25, 26) removed the rest is
    // the reference; the two rows are blank body rows.
    let kept: Vec<u32> = (0..30).filter(|r| *r != 25 && *r != 26).collect();
    for (i, row) in kept.iter().enumerate() {
        let want = panel(&g28, u32::try_from(i).unwrap());
        assert_eq!(panel(&g30, *row), want, "row {row}");
    }
    for row in [25, 26] {
        assert_eq!(panel(&g30, row), format!("║{}║", " ".repeat(23)));
    }
    // And a taller window moves the extra rows to Elements only.
    let tall = ShellState::new(82, 50).compose().unwrap();
    for row in 4..17 {
        assert_eq!(panel(&tall, row), panel(&g28, row), "row {row}");
    }
}

#[test]
fn every_state_keeps_the_reference_vocabulary_of_glyphs() {
    // Collapse and expand: markers and connectors stay in the documented set.
    let mut s = ref_shell();
    click_text(&mut s, "[-] Hub_Features");
    click_text(&mut s, "[+]Construct");
    let g = s.compose().unwrap();
    let rows: Vec<String> = (4..26).map(|r| panel(&g, r)).collect();
    let line = |s: &str| format!("║{s:<23}║");
    assert_eq!(rows[3], line(" ├[+] Hub_Features"));
    assert_eq!(rows[4], line(" ├ Fillet_Inner"));
    assert_eq!(rows[5], line(" │ ────────────────────"));
    assert_eq!(rows[17], line(" ├ Surfaces(0)"));
    assert_eq!(rows[18], line(" ╰[-]Construct"));
    assert_eq!(rows[19], line("   ╰╴□ Plane_XY"));
}

// ---- truncation and the minimum width ----

#[test]
fn long_names_are_truncated_with_an_ellipsis_in_the_last_cell() {
    let mut s = ref_shell();
    let g = s.compose().unwrap();
    assert!(text(&g, 6).starts_with("║ ├ □ Sketch_Profile_Ma…║"));
    // A wider panel shows the whole name; a narrower one cuts earlier, always with `…`.
    s.set_nav_width(30);
    assert!(text(&s.compose().unwrap(), 6).starts_with("║ ├ □ Sketch_Profile_Main"));
    s.set_nav_width(14);
    let g = s.compose().unwrap();
    assert!(text(&g, 6).starts_with("║ ├ □ Sketch…║"), "{}", text(&g, 6));
    assert!(
        text(&g, 10).starts_with("║ │ ╰╴Extrud…║"),
        "{}",
        text(&g, 10)
    );
}

#[test]
fn the_navigation_panel_is_never_narrower_than_14_cells() {
    let mut s = ref_shell();
    assert!(s.set_nav_width(1));
    assert_eq!(s.nav_width(), 14);
    s.drag_separator(Separator::Navigation, 0, 0);
    assert_eq!(s.nav_width(), 14);
    s.drag_separator(Separator::Navigation, 5, 0);
    assert_eq!(s.nav_width(), 14);
    // The wish is clamped, not forgotten: a wide grid honours a later wish.
    s.drag_separator(Separator::Navigation, 19, 0);
    assert_eq!(s.nav_width(), 20);
}

// ---- dynamic counts ----

#[test]
fn counts_are_derived_from_the_tree_not_stored() {
    let nodes = [
        Node {
            label: "Group".into(),
            depth: 1,
            kind: NodeKind::Category { counted: true },
            check: None,
            target: TreeTarget {
                document: TreeDocumentId::new(0),
                kind: TreeTargetKind::FeaturesRoot,
            },
        },
        Node {
            label: "a".into(),
            depth: 2,
            kind: NodeKind::Named,
            check: None,
            target: TreeTarget {
                document: TreeDocumentId::new(0),
                kind: TreeTargetKind::FeaturesRoot,
            },
        },
        Node {
            label: "b".into(),
            depth: 2,
            kind: NodeKind::Named,
            check: None,
            target: TreeTarget {
                document: TreeDocumentId::new(0),
                kind: TreeTargetKind::FeaturesRoot,
            },
        },
        Node {
            label: "b_child".into(),
            depth: 3,
            kind: NodeKind::Named,
            check: None,
            target: TreeTarget {
                document: TreeDocumentId::new(0),
                kind: TreeTargetKind::FeaturesRoot,
            },
        },
        Node {
            label: "c".into(),
            depth: 2,
            kind: NodeKind::Named,
            check: None,
            target: TreeTarget {
                document: TreeDocumentId::new(0),
                kind: TreeTargetKind::FeaturesRoot,
            },
        },
    ];
    assert_eq!(child_count(&nodes, 0), 3, "direct children only");
    assert_eq!(full_label(&nodes, 0, None), "Group(3)");
    assert_eq!(full_label(&nodes, 1, None), "a");
    assert_eq!(full_label(&nodes, 2, Some("renamed")), "renamed");
    assert_eq!(
        full_label(&ref_shell().nodes[1], 1, None),
        "Solid Bodies(2)"
    );
    assert_eq!(full_label(&ref_shell().nodes[1], 4, None), "Surfaces(0)");
    assert_eq!(full_label(&ref_shell().nodes[1], 5, None), "Construct");
}

// ---- divider ----

#[test]
fn divider_is_the_shared_border_row_between_the_panes() {
    let s = ref_shell();
    assert_eq!(s.separator_at(5, 16), Some(Separator::Split));
    assert_eq!(s.separator_at(1, 16), Some(Separator::Split));
    assert_eq!(s.separator_at(23, 16), Some(Separator::Split));
    assert_eq!(
        s.separator_at(0, 16),
        None,
        "the frame ring is a resize zone"
    );
    assert_eq!(s.separator_at(24, 16), Some(Separator::Navigation));
    assert_eq!(s.separator_at(5, 15), None);
    assert_eq!(s.separator_at(5, 17), None);
    assert_eq!(s.separator_at(30, 16), None);
    let g = s.compose().unwrap();
    let hit = hit_at(&g, 5, 16).unwrap();
    assert_eq!(s.drag_at(&hit), Some(Separator::Split));
}

#[test]
fn dragging_the_divider_moves_it_by_whole_rows_and_clamps() {
    let mut s = ref_shell();
    assert!(s.drag_separator(Separator::Split, 40, 12));
    let g = s.compose().unwrap();
    assert_eq!(panel(&g, 12), format!("╠{}╣", "═".repeat(23)));
    assert!(text(&g, 5).starts_with("║[-]Features"));
    assert!(
        !s.drag_separator(Separator::Split, 40, 12),
        "same row: no change"
    );
    assert!(text(&g, 13).starts_with("║[-]Elements"), "{}", text(&g, 13));

    // Three rows is the least a pane keeps; the body is 22 rows, so Features tops out at 20.
    s.drag_separator(Separator::Split, 0, 0);
    assert_eq!(
        panel(&s.compose().unwrap(), 6),
        format!("╠{}╣", "═".repeat(23))
    );
    s.drag_separator(Separator::Split, 0, u32::MAX);
    let g = s.compose().unwrap();
    assert_eq!(panel(&g, 23), format!("╠{}╣", "═".repeat(23)));
    assert!(text(&g, 24).starts_with("║[-]Elements"));
}

#[test]
fn the_divider_keeps_its_proportion_when_the_window_is_resized() {
    let mut s = ref_shell();
    s.drag_separator(Separator::Split, 0, 12);
    let heights = |s: &ShellState| {
        let l = s.layout();
        (l.features.height(), l.elements.height())
    };
    assert_eq!(heights(&s), (9, 14));
    s.resize(82, 38); // body 22 -> 32: 9 * 32 / 22 = 13.1
    assert_eq!(heights(&s), (13, 20));
    s.resize(82, 28);
    assert_eq!(heights(&s), (9, 14), "the user's split comes back");
    // Smaller than both minimum panes: clamped, never inverted.
    s.resize(82, 12);
    let (f, e) = heights(&s);
    assert!(f + e >= 4 && f <= s.layout().viewport.height());
    s.resize(82, 28);
    assert_eq!(heights(&s), (9, 14));
}

#[test]
fn the_default_split_is_the_reference_one_at_every_height() {
    for rows in [25, 28, 30, 40, 60] {
        let l = ShellState::new(82, rows).layout();
        let body = l.viewport.height();
        assert_eq!(l.features.height(), (body * 3 / 5).min(13), "{rows} rows");
        assert_eq!(l.features.row() + l.features.height() - 1, l.elements.row());
    }
}

#[test]
fn divider_cannot_be_dragged_while_a_menu_or_modal_owns_input() {
    let mut s = ref_shell();
    assert!(s.fixture_key(ShellKey::MenuBar));
    assert_eq!(s.separator_at(5, 16), None);
    assert!(s.fixture_key(ShellKey::MenuBar));
    s.begin_rename(id_of(&s, "Hub_Body"));
    assert!(s.rename().is_open());
    assert_eq!(s.separator_at(5, 16), None);
}

// ---- playhead ----

fn rows_of(s: &ShellState, from: u32, to: u32) -> Vec<String> {
    let g = s.compose().unwrap();
    (from..=to).map(|r| panel(&g, r)).collect()
}

const PLAYHEAD_ROW: &str = "║ │ ────────────────────║";

#[test]
fn the_playhead_is_a_line_between_two_features_and_not_a_node() {
    let s = ref_shell();
    let g = s.compose().unwrap();
    assert_eq!(panel(&g, 12), PLAYHEAD_ROW);
    let hit = hit_at(&g, 10, 12).unwrap();
    assert_eq!(hit.widget_id, Some(WidgetId::new(PLAYHEAD_ID)));
    assert_eq!(
        tree_decode(WidgetId::new(PLAYHEAD_ID)),
        None,
        "not a feature"
    );
    assert_eq!(s.drag_at(&hit), Some(Separator::Playhead));
    assert!(!s.is_rename_target(WidgetId::new(PLAYHEAD_ID)));

    // Pressing it selects nothing and takes no focus.
    let mut s = ref_shell();
    assert!(!s.fixture_click(Some(hit)));
    assert!(s.entity_selection.entities.is_empty());
    assert_eq!(s.interaction().focused, None);
    // Tab never lands on it.
    let ring = s.focus_ring(&s.compose().unwrap());
    assert!(!ring.contains(&WidgetId::new(PLAYHEAD_ID)));
    for _ in 0..60 {
        s.fixture_key(ShellKey::Tab);
        assert_ne!(s.interaction().focused, Some(WidgetId::new(PLAYHEAD_ID)));
    }
}

#[test]
fn dragging_the_playhead_moves_it_between_top_level_features() {
    let mut s = ref_shell();
    let before = rows_of(&s, 5, 15);

    // Onto Hub_Features (row 7): after its whole subtree, before Fillet_Inner.
    assert!(s.drag_separator(Separator::Playhead, 10, 7));
    let after = rows_of(&s, 5, 15);
    assert_eq!(after[5], "║ │ ╰╴Extrude_Cut_Center║");
    assert_eq!(after[6], PLAYHEAD_ROW);
    assert_eq!(after[7], "║ ├ Fillet_Inner        ║");
    // Nesting and every other row are untouched: only the separator line moved.
    let strip = |rows: &[String]| {
        rows.iter()
            .filter(|r| *r != PLAYHEAD_ROW)
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(strip(&before), strip(&after));
    assert!(
        !s.drag_separator(Separator::Playhead, 10, 7),
        "same gap: no change"
    );

    // Above the tree: after the first feature. Below it: before the last one.
    assert!(s.drag_separator(Separator::Playhead, 10, 0));
    let g = s.compose().unwrap();
    assert_eq!(panel(&g, 6), "║ ├ □ Sketch_Profile_Ma…║");
    assert_eq!(panel(&g, 7), PLAYHEAD_ROW);
    assert_eq!(panel(&g, 8), "║ ├[-] Hub_Features     ║");
    assert!(s.drag_separator(Separator::Playhead, 10, 40));
    let g = s.compose().unwrap();
    assert_eq!(panel(&g, 12), "║ ├ Chamfer_Edges       ║");
    assert_eq!(panel(&g, 13), PLAYHEAD_ROW);
    assert_eq!(panel(&g, 14), "║ ╰╴Pattern_Circular    ║");

    // A row inside an expanded group still means that group.
    assert!(s.drag_separator(Separator::Playhead, 10, 9));
    assert_eq!(panel(&s.compose().unwrap(), 11), PLAYHEAD_ROW);
    // Dragging over the separator itself changes nothing.
    assert!(!s.drag_separator(Separator::Playhead, 10, 11));
}

#[test]
fn the_playhead_survives_collapsing_and_expanding_groups() {
    let mut s = ref_shell();
    click_text(&mut s, "[-] Hub_Features");
    let g = s.compose().unwrap();
    // Collapsed: Features, Sketch, Hub, Fillet, playhead, Chamfer, Pattern.
    assert_eq!(panel(&g, 9), PLAYHEAD_ROW);
    assert_eq!(panel(&g, 10), "║ ├ Chamfer_Edges       ║");
    click_text(&mut s, "[+] Hub_Features");
    assert_eq!(panel(&s.compose().unwrap(), 12), PLAYHEAD_ROW);

    let gaps = match &s.gap_policy {
        HistoryGapPolicy::ExplicitGaps(gaps) => Some(gaps),
        _ => None,
    }
    .expect("reference policy");
    assert_eq!(gaps.len(), 4);
    assert_eq!(
        gaps[0],
        PlacedHistoryGap {
            insertion: HistoryGap::Before(OpaqueTreeId::new(2)),
            display: GapDisplayLocation::BeforeNode(FeatureNodeId::Folder(OpaqueTreeId::new(1)))
        }
    );
    assert_eq!(s.projection.placed_gap(gaps[0].display).unwrap(), gaps[0]);
    let lines = s.tree_lines(Tree::Features);
    let at = lines
        .iter()
        .position(|line| *line == Line::Playhead)
        .unwrap();
    assert!(
        matches!(&lines[at + 1], Line::Node(row) if s.nodes[0][row.index].label == "Chamfer_Edges")
    );
}

// ---- selection, expansion, checkboxes ----

#[test]
fn marker_checkbox_and_name_are_separate_targets() {
    let mut s = ref_shell();
    let marker = id_of(&s, "[-] Hub_Features");
    let name = id_of(&s, "Hub_Features");
    assert_ne!(marker, name);
    assert_eq!(tree_decode(marker).map(|d| d.2), Some(TreePart::Marker));
    assert_eq!(tree_decode(name).map(|d| d.2), Some(TreePart::Name));

    // The name selects and does not expand.
    assert!(click_text(&mut s, "Hub_Features"));
    assert!(
        s.entity_selection.entities.is_empty(),
        "folders only receive focus"
    );
    assert_eq!(
        s.tree_focus,
        Some(
            s.projection
                .target(TreeTargetKind::Folder(OpaqueTreeId::new(1)))
        )
    );
    assert!(
        s.expanded.contains(
            &s.projection
                .target(TreeTargetKind::Folder(OpaqueTreeId::new(1)))
        )
    );
    // The marker expands and leaves the selection alone.
    s.entity_selection.entities.clear();
    assert!(click_text(&mut s, "[-] Hub_Features"));
    assert!(
        !s.expanded.contains(
            &s.projection
                .target(TreeTargetKind::Folder(OpaqueTreeId::new(1)))
        ) && s.entity_selection.entities.is_empty()
    );
    click_text(&mut s, "[+] Hub_Features");

    // The checkbox flips the visibility glyph and nothing else.
    assert!(click_text(&mut s, "□ Sketch_Profile"));
    let g = s.compose().unwrap();
    assert!(panel(&g, 6).starts_with("║ ├ ■ Sketch_Profile"));
    assert!(s.entity_selection.entities.is_empty());
    click_text(&mut s, "■ Sketch_Profile");
    assert!(panel(&s.compose().unwrap(), 6).starts_with("║ ├ □ Sketch_Profile"));
    // A row without a checkbox has none to flip.
    let before = s.projection.clone();
    click_text(&mut s, "Fillet_Inner");
    assert_eq!(before, s.projection);
}

#[test]
fn selection_uses_canonical_hierarchy_while_focus_keeps_the_same_glyphs() {
    let mut s = ref_shell();
    let glyphs = |s: &ShellState| {
        let g = s.compose().unwrap();
        (0..g.rows()).map(|r| text(&g, r)).collect::<Vec<_>>()
    };
    let base = glyphs(&s);
    click_text(&mut s, "Rim_Body");
    let selected = glyphs(&s);
    assert_ne!(selected, base);
    assert!(selected.iter().any(|row| row.contains("┣ ■ Rim_Body")));
    s.fixture_key(ShellKey::Tab);
    assert_eq!(glyphs(&s), selected);
}

#[test]
fn enter_and_space_expand_the_focused_branch_row() {
    let mut s = ref_shell();
    click_text(&mut s, "Hub_Features");
    assert_eq!(s.interaction().focused, Some(tree_id(Tree::Features, 2)));
    s.fixture_key(ShellKey::Enter);
    assert!(
        !s.expanded.contains(
            &s.projection
                .target(TreeTargetKind::Folder(OpaqueTreeId::new(1)))
        )
    );
    s.fixture_key(ShellKey::Char(' '));
    assert!(
        s.expanded.contains(
            &s.projection
                .target(TreeTargetKind::Folder(OpaqueTreeId::new(1)))
        )
    );
    // A leaf row has nothing to expand.
    click_text(&mut s, "Extrude_Base");
    let open = s.expanded.clone();
    s.fixture_key(ShellKey::Enter);
    assert_eq!(open, s.expanded);
}

// ---- Rename ----

#[test]
fn only_user_named_rows_are_rename_targets() {
    let s = ref_shell();
    for (needle, yes) in [
        ("Sketch_Profile_Ma", true),
        ("Hub_Features", true),
        ("Extrude_Base", true),
        ("Rim_Body", true),
        ("Hub_Body", true),
        ("Features", false),
        ("Elements", false),
        ("Solid Bodies", false),
        ("Surfaces(0)", false),
        ("Construct", false),
        ("[-] Hub_Features", false),
        ("□ Sketch_Profile", false),
        ("■ Rim_Body", false),
    ] {
        assert_eq!(s.is_rename_target(id_of(&s, needle)), yes, "{needle}");
    }
    let g = s.compose().unwrap();
    // The blank cells after a name and the guides are the row, not its name.
    let trailing = hit_at(&g, 20, 11).and_then(|h| h.widget_id).unwrap();
    assert!(!s.is_rename_target(trailing));
}

#[test]
fn rename_opens_centred_on_the_full_name() {
    let mut s = ref_shell();
    assert!(
        !s.begin_rename(id_of(&s, "Solid Bodies")),
        "a header is no target"
    );
    assert!(!s.rename().is_open());

    assert!(s.begin_rename(id_of(&s, "Sketch_Profile_Ma")));
    assert!(s.rename().is_open() && s.modal_open());
    assert_eq!(
        s.rename().draft(),
        "Sketch_Profile_Main",
        "not the `…` text"
    );
    let g = s.compose().unwrap();
    let want = [
        "╭─Rename──────────────────╮",
        "│[Sketch_Profile_Main▏   ]│",
        "│─────────────────────────│",
        "│     OK     │   Cancel   │",
        "╰─────────────────────────╯",
    ];
    // 82 x 28: (82 - 27) / 2 = 27, (28 - 5) / 2 = 11.
    for (i, line) in want.iter().enumerate() {
        let row = 11 + u32::try_from(i).unwrap();
        let got: String = text(&g, row).chars().skip(27).take(27).collect();
        assert_eq!(&got, line, "row {row}");
    }
    assert_eq!(s.interaction().focused, s.rename().focus());
}

#[test]
fn rename_ok_changes_the_name_and_keeps_ids_and_focus() {
    let mut s = ref_shell();
    let name_id = id_of(&s, "Hub_Body");
    let row_id = tree_id(Tree::Elements, 3);
    click_text(&mut s, "Hub_Body");
    s.begin_rename(name_id);
    type_str(&mut s, "_2");
    s.fixture_key(ShellKey::Backspace);
    type_str(&mut s, "3");
    assert!(s.fixture_key(ShellKey::Enter));
    assert!(!s.rename().is_open() && !s.modal_open());
    let g = s.compose().unwrap();
    assert!(
        panel(&g, 20).starts_with("║ ┃ ┗╸□ Hub_Body_3"),
        "{}",
        panel(&g, 20)
    );
    assert_eq!(id_of(&s, "Hub_Body_3"), name_id, "the id is stable");
    assert_eq!(s.interaction().focused, Some(row_id), "focus is restored");
    assert_eq!(s.entity_selection.entities, vec![OpaqueTreeId::new(2)]);

    // Renaming again starts from the new full name.
    s.begin_rename(name_id);
    assert_eq!(s.rename().draft(), "Hub_Body_3");
}

#[test]
fn rename_cancel_and_escape_keep_the_old_name_and_restore_focus() {
    let mut s = ref_shell();
    let name_id = id_of(&s, "Rim_Body");
    let row_id = tree_id(Tree::Elements, 2);
    click_text(&mut s, "Rim_Body");
    s.begin_rename(name_id);
    type_str(&mut s, "xx");
    s.fixture_key(ShellKey::Escape);
    assert!(!s.rename().is_open());
    assert!(has_text(&s, "Rim_Body") && !has_text(&s, "Rim_Bodyxx"));
    assert_eq!(s.interaction().focused, Some(row_id));

    s.begin_rename(name_id);
    type_str(&mut s, "yy");
    assert!(click_text(&mut s, "Cancel"));
    assert!(!s.rename().is_open());
    assert!(has_text(&s, "Rim_Body") && !has_text(&s, "Rim_Bodyyy"));
    assert_eq!(s.interaction().focused, Some(row_id));

    // The OK button works with the pointer too.
    s.begin_rename(name_id);
    type_str(&mut s, "!");
    assert!(click_text(&mut s, "OK"));
    assert!(has_text(&s, "Rim_Body!"));
}

fn has_text(s: &ShellState, needle: &str) -> bool {
    find(&s.compose().unwrap(), needle).is_some()
}

#[test]
fn a_long_new_name_is_truncated_in_the_tree_but_edited_in_full() {
    let mut s = ref_shell();
    let id = id_of(&s, "Hub_Body");
    s.begin_rename(id);
    for _ in 0.."Hub_Body".len() {
        s.fixture_key(ShellKey::Backspace);
    }
    type_str(&mut s, "A_very_long_element_name_here");
    s.fixture_key(ShellKey::Enter);
    let g = s.compose().unwrap();
    assert_eq!(panel(&g, 20), "║ │ ╰╴□ A_very_long_ele…║");
    s.begin_rename(id);
    assert_eq!(s.rename().draft(), "A_very_long_element_name_here");
}

#[test]
fn rename_is_modal_and_never_empties_a_name() {
    let mut s = ref_shell();
    let id = id_of(&s, "Hub_Body");
    s.begin_rename(id);
    let before = (s.expanded.clone(), s.entity_selection.entities.clone());
    // The base shell is blocked, for pointer and keyboard.
    let g = s.compose().unwrap();
    let (col, row) = find(&g, "Hub_Features").unwrap();
    assert_eq!(hit_at(&g, col, row).unwrap().widget_id, None);
    assert!(!s.fixture_click(hit_at(&g, col, row)));
    assert!(!s.fixture_key(ShellKey::MenuBar) && !s.fixture_key(ShellKey::Alt('f')));
    assert_eq!(
        before,
        (s.expanded.clone(), s.entity_selection.entities.clone())
    );
    assert!(!s.begin_rename(id_of_modal_free()), "no second modal");

    for _ in 0.."Hub_Body".len() {
        s.fixture_key(ShellKey::Backspace);
    }
    s.fixture_key(ShellKey::Enter);
    assert!(s.rename().is_open(), "an empty name is refused");
    type_str(&mut s, "ok");
    s.fixture_key(ShellKey::Enter);
    assert!(has_text(&s, "ok"));
}

fn id_of_modal_free() -> WidgetId {
    tree_part_id(Tree::Features, 1, TreePart::Name)
}

#[test]
fn renaming_is_per_row_and_does_not_touch_other_rows_or_trees() {
    let mut s = ref_shell();
    s.begin_rename(id_of(&s, "Extrude_Base"));
    type_str(&mut s, "_v2");
    s.fixture_key(ShellKey::Enter);
    let g = s.compose().unwrap();
    assert!(find(&g, "Extrude_Base_v2").is_some());
    assert!(find(&g, "Sketch_Hole").is_some() && find(&g, "Rim_Body").is_some());
    assert_eq!(
        s.projection
            .features
            .iter()
            .find(|item| item.id == FeatureNodeId::Operation(OpaqueTreeId::new(2)))
            .unwrap()
            .name,
        "Extrude_Base_v2"
    );
    assert_eq!(s.projection.elements, data::reference_trees().0.elements);
    // The ellipsis row of the reference is unaffected.
    assert!(panel(&g, 6).ends_with("Sketch_Profile_Ma…║"));
}

#[test]
fn tiny_grids_survive_every_tree_interaction() {
    for cols in [0, 1, 7, 20, 30, 82] {
        for rows in [0, 1, 4, 9, 14, 28] {
            let mut s = ShellState::new(cols, rows);
            for separator in [Separator::Navigation, Separator::Split, Separator::Playhead] {
                s.drag_separator(separator, 3, 5);
                s.drag_separator(separator, u32::MAX, u32::MAX);
                s.drag_separator(separator, 0, 0);
            }
            s.begin_rename(tree_part_id(Tree::Features, 1, TreePart::Name));
            s.fixture_key(ShellKey::Char('x'));
            s.fixture_key(ShellKey::Enter);
            let g = s.compose().unwrap();
            assert_eq!((g.cols(), g.rows()), (cols, rows));
        }
    }
}

#[test]
fn color_live_projection_guides_names_scrolling_and_replacement_keep_semantics() {
    use crate::InkRole::{Action, Contour, Text};
    let mut projection = live_projection(200);
    projection.features[0].name = "│[+]☰ an ordinary long operation name".into();
    projection.features[0].depth = 2;
    projection.features.insert(
        0,
        FeatureTreeItem {
            id: FeatureNodeId::Folder(OpaqueTreeId::new(900)),
            name: "nested folder".into(),
            depth: 1,
            visibility: None,
        },
    );
    projection.features.push(FeatureTreeItem {
        id: FeatureNodeId::Folder(OpaqueTreeId::new(901)),
        name: "empty folder".into(),
        depth: 1,
        visibility: None,
    });
    for (index, group) in [
        ElementGroup::Bodies,
        ElementGroup::Surfaces,
        ElementGroup::Construct,
    ]
    .into_iter()
    .enumerate()
    {
        projection.elements.push(ElementTreeItem {
            id: OpaqueTreeId::new(index as u64),
            source_feature: OpaqueTreeId::new(0),
            output_ordinal: index,
            group,
            name: format!("output_{index}"),
            visible: index != 1,
        });
    }
    let mut s = live_shell(projection.clone());
    s.view.color_enabled = true;
    s.expanded
        .insert(projection.target(TreeTargetKind::Folder(OpaqueTreeId::new(900))));
    let grid = s.compose().unwrap();
    let (x, y) = find(&grid, "│[+]☰").unwrap();
    for dx in 0..6 {
        assert_eq!(grid.get(x + dx, y).unwrap().ink, Text);
    }
    let original_target = s.node_id(Tree::Features, 1, TreePart::Name);
    assert!(
        s.tree_hit(original_target, ProjectionGeneration::new(0))
            .is_some()
    );
    assert!(grid.cells().iter().any(|c| c.ink == Action));
    assert!(grid.cells().iter().any(|c| c.ink == Contour));
    let playhead = s.playhead();
    s.scroll_tree_at(4, 8, -50);
    let scroll = s.compose().unwrap();
    for row in s.layout().features.inset(1).row()..s.layout().features.bottom() as u32 - 1 {
        for col in 1..s.layout().features.width() - 1 {
            let c = scroll.get(col, row).unwrap();
            if matches!(c.content, CellContent::Glyph('│' | '├' | '╰' | '╴' | '─')) {
                assert_eq!(c.ink, Text);
            }
        }
    }
    assert_eq!(s.playhead(), playhead);
    let mut replacement = live_projection(1);
    replacement.document = TreeDocumentId::new(2);
    replacement.features[0].name = "│[+]☰ an ordinary long operation name".into();
    s.set_tree_projection(replacement).unwrap();
    assert!(s.color_enabled());
    assert_eq!(s.theme(), ThemeChoice::Dark);
    assert_eq!(s.playhead(), PlacedHistoryGap::default());
    assert_eq!(
        s.tree_projection().features[0].name,
        "│[+]☰ an ordinary long operation name"
    );
}
