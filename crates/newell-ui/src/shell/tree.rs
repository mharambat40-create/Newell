// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Model-independent document projections and semantic tree presentation.

use crate::UiError;
use std::collections::BTreeSet;

macro_rules! identity {
    ($name:ident, $documentation:literal) => {
        #[doc = $documentation]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);
        impl $name {
            /// Wraps a caller-supplied value; this does not allocate an identity.
            #[must_use]
            pub const fn new(value: u64) -> Self {
                Self(value)
            }
            /// Returns the underlying value for the owning adapter or controller.
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }
        }
    };
}
identity!(
    TreeDocumentId,
    "Document namespace supplied by the application; replacement uses a fresh value."
);
identity!(
    OpaqueTreeId,
    "Stable model identity supplied by the adapter, interpreted only within its semantic kind."
);
identity!(
    ProjectionGeneration,
    "Published routing-map generation owned by the application controller."
);
/// Fixed Elements group order; Construct includes all construction entity kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ElementGroup {
    Bodies,
    Surfaces,
    Construct,
}
/// Distinguishes operation and folder ids even when their raw values are equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FeatureNodeId {
    Operation(OpaqueTreeId),
    Folder(OpaqueTreeId),
}
/// One operation or folder in Features preorder, with an untruncated name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureTreeItem {
    pub id: FeatureNodeId,
    pub name: String,
    /// Root children have depth one; only folders can parent deeper items.
    pub depth: u32,
    /// Legacy reference squares only; live operations and folders supply `None`.
    pub visibility: Option<bool>,
}
/// One entity in the ordered operation/output stream, before Elements grouping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementTreeItem {
    pub id: OpaqueTreeId,
    pub source_feature: OpaqueTreeId,
    /// Zero-based position within the producing operation's stored outputs.
    pub output_ordinal: usize,
    pub group: ElementGroup,
    pub name: String,
    pub visible: bool,
}
/// Group labels; only the historical fixture overrides Bodies with Solid Bodies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeLabels {
    pub bodies: String,
    pub surfaces: String,
    pub construct: String,
}
impl Default for TreeLabels {
    fn default() -> Self {
        Self {
            bodies: "Bodies".into(),
            surfaces: "Surfaces".into(),
            construct: "Construct".into(),
        }
    }
}
/// Complete model-independent tree data supplied by the application adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelTreeProjection {
    pub document: TreeDocumentId,
    pub features: Vec<FeatureTreeItem>,
    pub elements: Vec<ElementTreeItem>,
    pub labels: TreeLabels,
}
/// Operation insertion boundary, independent of where the separator is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryGap {
    Before(OpaqueTreeId),
    End,
}
/// Structural separator placement, including the headers and complete subtrees of folders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapDisplayLocation {
    BeforeNode(FeatureNodeId),
    AfterSubtree(FeatureNodeId),
    EndOfTree,
}
/// Insertion boundary paired with a structural location that resolves to that boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedHistoryGap {
    pub insertion: HistoryGap,
    pub display: GapDisplayLocation,
}
impl Default for PlacedHistoryGap {
    fn default() -> Self {
        Self {
            insertion: HistoryGap::End,
            display: GapDisplayLocation::EndOfTree,
        }
    }
}
/// Semantic target kind, independent of visible cell positions and reusable widget ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TreeTargetKind {
    FeaturesRoot,
    Operation(OpaqueTreeId),
    Folder(OpaqueTreeId),
    ElementsRoot,
    Group(ElementGroup),
    Entity(OpaqueTreeId),
    Playhead,
}
/// Tagged semantic identity inside a single document namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TreeTarget {
    pub document: TreeDocumentId,
    pub kind: TreeTargetKind,
}
/// Live cursor admission or the historical reference's explicitly permitted placements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryGapPolicy {
    AllOperationGaps,
    ExplicitGaps(Vec<PlacedHistoryGap>),
}
/// Presentation defaults applied on initialization, separately from projected model data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeInitialState {
    pub expanded: BTreeSet<TreeTarget>,
    pub playhead: PlacedHistoryGap,
    pub gap_policy: HistoryGapPolicy,
}
impl TreeInitialState {
    /// Checks expanded targets, the initial cursor pair and every explicitly permitted gap.
    pub fn validate(&self, projection: &ModelTreeProjection) -> Result<(), UiError> {
        projection.validate()?;
        if projection.placed_gap(self.playhead.display)? != self.playhead {
            return Err(UiError::InvalidTreeProjection);
        }
        if let HistoryGapPolicy::ExplicitGaps(gaps) = &self.gap_policy {
            if !gaps.contains(&self.playhead) {
                return Err(UiError::InvalidTreeProjection);
            }
            for gap in gaps {
                if projection.placed_gap(gap.display)? != *gap {
                    return Err(UiError::InvalidTreeProjection);
                }
            }
        }
        let nodes = super::data::projection_nodes(projection);
        if self
            .expanded
            .iter()
            .any(|target| !nodes.iter().flatten().any(|node| node.target == *target))
        {
            return Err(UiError::InvalidTreeProjection);
        }
        Ok(())
    }
}
/// Routing part of a row; name and padding have different double-click behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreePart {
    Row = 0,
    Marker = 1,
    Check = 2,
    Name = 3,
}
impl TreePart {
    pub(super) const ALL: [Self; 4] = [Self::Row, Self::Marker, Self::Check, Self::Name];
}
/// Captured semantic hit to be validated against the currently published generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeHitToken {
    pub generation: ProjectionGeneration,
    pub target: TreeTarget,
    pub part: TreePart,
}
/// Model edit request with the source value needed to reject stale changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeIntentKind {
    Rename {
        source_name: String,
        name: String,
    },
    SetVisibility {
        expected_visible: bool,
        visible: bool,
    },
}
/// One model edit authorized by the controller's published frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeIntent {
    pub(super) generation: ProjectionGeneration,
    pub(super) origin_generation: ProjectionGeneration,
    pub(super) target: TreeTarget,
    pub(super) kind: TreeIntentKind,
}
impl TreeIntent {
    /// Frame at which this one-shot intent may currently be delivered.
    #[must_use]
    pub const fn generation(&self) -> ProjectionGeneration {
        self.generation
    }
    /// Original hit generation, retained across Rename's certified modal close.
    #[must_use]
    pub const fn origin_generation(&self) -> ProjectionGeneration {
        self.origin_generation
    }
    /// Semantic target, including its document namespace.
    #[must_use]
    pub const fn target(&self) -> TreeTarget {
        self.target
    }
    /// Requested edit and its expected source value.
    #[must_use]
    pub const fn kind(&self) -> &TreeIntentKind {
        &self.kind
    }
}

impl ModelTreeProjection {
    /// Empty live-style projection with all three Elements group labels.
    #[must_use]
    pub fn empty(document: TreeDocumentId) -> Self {
        Self {
            document,
            features: Vec::new(),
            elements: Vec::new(),
            labels: TreeLabels::default(),
        }
    }
    /// Qualifies a semantic target kind with this projection's namespace.
    #[must_use]
    pub const fn target(&self, kind: TreeTargetKind) -> TreeTarget {
        TreeTarget {
            document: self.document,
            kind,
        }
    }
    /// Validates unique identities, folder-only nesting and operation/output references.
    pub fn validate(&self) -> Result<(), UiError> {
        let mut seen = BTreeSet::new();
        let mut previous: Option<&FeatureTreeItem> = None;
        for item in &self.features {
            if !seen.insert(item.id) || item.depth == 0 || previous.is_none() && item.depth != 1 {
                return Err(UiError::InvalidTreeProjection);
            }
            if let Some(prev) = previous
                && item.depth > prev.depth
                && (item.depth.checked_sub(prev.depth) != Some(1)
                    || !matches!(prev.id, FeatureNodeId::Folder(_)))
            {
                return Err(UiError::InvalidTreeProjection);
            }
            previous = Some(item);
        }
        let mut entities = BTreeSet::new();
        let mut outputs = BTreeSet::new();
        for item in &self.elements {
            if !entities.insert(item.id)
                || !outputs.insert((item.source_feature, item.output_ordinal))
                || !seen.contains(&FeatureNodeId::Operation(item.source_feature))
            {
                return Err(UiError::InvalidTreeProjection);
            }
        }
        Ok(())
    }
    /// Resolves a structural display location to its operation insertion boundary.
    pub fn placed_gap(&self, display: GapDisplayLocation) -> Result<PlacedHistoryGap, UiError> {
        let start = self.display_boundary(display)?;
        let insertion = self.features[start..]
            .iter()
            .find_map(|item| match item.id {
                FeatureNodeId::Operation(id) => Some(HistoryGap::Before(id)),
                FeatureNodeId::Folder(_) => None,
            })
            .unwrap_or(HistoryGap::End);
        Ok(PlacedHistoryGap { insertion, display })
    }
    pub(super) fn display_boundary(&self, display: GapDisplayLocation) -> Result<usize, UiError> {
        let (id, after) = match display {
            GapDisplayLocation::EndOfTree => return Ok(self.features.len()),
            GapDisplayLocation::BeforeNode(id) => (id, false),
            GapDisplayLocation::AfterSubtree(id) => (id, true),
        };
        let index = self
            .features
            .iter()
            .position(|item| item.id == id)
            .ok_or(UiError::InvalidTreeProjection)?;
        if !after {
            return Ok(index);
        }
        Ok(index
            + 1
            + self.features[index + 1..]
                .iter()
                .take_while(|item| item.depth > self.features[index].depth)
                .count())
    }
    /// Live defaults: roots and Bodies expanded, folders collapsed, cursor at End.
    #[must_use]
    pub fn initial_state(&self) -> TreeInitialState {
        TreeInitialState {
            expanded: [
                TreeTargetKind::FeaturesRoot,
                TreeTargetKind::ElementsRoot,
                TreeTargetKind::Group(ElementGroup::Bodies),
            ]
            .into_iter()
            .map(|kind| self.target(kind))
            .collect(),
            playhead: PlacedHistoryGap::default(),
            gap_policy: HistoryGapPolicy::AllOperationGaps,
        }
    }
}

impl super::ShellState {
    pub(super) fn playhead_depth(&self) -> u32 {
        match self.playhead.display {
            GapDisplayLocation::BeforeNode(id) | GapDisplayLocation::AfterSubtree(id) => self
                .projection
                .features
                .iter()
                .find(|item| item.id == id)
                .map_or(1, |item| item.depth),
            GapDisplayLocation::EndOfTree => 1,
        }
    }
    /// Guides belong to the structural boundary, not to the previous visible row.
    pub(super) fn playhead_trunk(&self) -> Vec<bool> {
        let (id, before) = match self.playhead.display {
            GapDisplayLocation::BeforeNode(id) => (id, true),
            GapDisplayLocation::AfterSubtree(id) => (id, false),
            GapDisplayLocation::EndOfTree => return Vec::new(),
        };
        let items = &self.projection.features;
        let mut ancestors: Vec<usize> = Vec::new();
        for (index, item) in items.iter().enumerate() {
            while ancestors.last().is_some_and(|i| items[*i].depth >= item.depth) {
                ancestors.pop();
            }
            if item.id == id {
                ancestors.push(index);
                return ancestors.iter().map(|i| {
                    (*i == index && before) || items[*i + 1..].iter()
                        .find(|next| next.depth <= items[*i].depth)
                        .is_some_and(|next| next.depth == items[*i].depth)
                }).collect();
            }
            ancestors.push(index);
        }
        Vec::new()
    }

    /// The outermost collapsed strict ancestor is the only visible inline host.
    pub(super) fn inline_playhead_host(&self) -> Option<TreeTarget> {
        if !self.expanded.contains(&self.projection.target(TreeTargetKind::FeaturesRoot)) {
            return None;
        }
        let id = match self.playhead.display {
            GapDisplayLocation::BeforeNode(id) | GapDisplayLocation::AfterSubtree(id) => id,
            GapDisplayLocation::EndOfTree => return None,
        };
        let mut ancestors: Vec<&FeatureTreeItem> = Vec::new();
        for item in &self.projection.features {
            while ancestors.last().is_some_and(|parent| parent.depth >= item.depth) {
                ancestors.pop();
            }
            if item.id == id {
                return ancestors.into_iter().find_map(|parent| {
                    let FeatureNodeId::Folder(folder) = parent.id else { return None; };
                    let target = self.projection.target(TreeTargetKind::Folder(folder));
                    (!self.expanded.contains(&target)).then_some(target)
                });
            }
            ancestors.push(item);
        }
        None
    }

    pub(super) fn update_tree_content_max(&mut self) {
        let natural = self.nodes.iter().map(|nodes| {
            nodes.iter().enumerate().map(|(index, node)| {
                let kind = if super::data::has_children(nodes, index) {
                    crate::TreeNodeKind::Branch
                } else { crate::TreeNodeKind::Leaf };
                let prefix = crate::TreeRow::prefix_width(node.depth, kind,
                    node.kind == super::data::NodeKind::Named, node.check.is_some());
                prefix.saturating_add(crate::widgets::text_len(&super::data::full_label(nodes, index, None)))
            }).max().unwrap_or(0)
        }).max().unwrap_or(0);
        self.tree_content_max = natural.saturating_add(4);
        self.widths.nav = self.widths.nav.min(self.tree_content_max);
    }

    /// Resolves a reusable slot to semantic identity in constant time for publication checks.
    #[must_use]
    pub fn tree_hover_target(&self, id: crate::WidgetId) -> Option<TreeTarget> {
        let (tree, index, _) = self.decode_tree(id)?;
        self.input_free().then_some(self.nodes[tree.slot()][index].target)
    }

    #[must_use]
    pub const fn tree_projection(&self) -> &ModelTreeProjection {
        &self.projection
    }
    #[must_use]
    pub const fn playhead(&self) -> PlacedHistoryGap {
        self.playhead
    }
    pub fn set_playhead(&mut self, gap: PlacedHistoryGap) -> Result<bool, UiError> {
        if self.projection.placed_gap(gap.display)? != gap {
            return Err(UiError::InvalidTreeProjection);
        }
        if let HistoryGapPolicy::ExplicitGaps(allowed) = &self.gap_policy
            && !allowed.contains(&gap)
        {
            return Err(UiError::InvalidTreeProjection);
        }
        let changed = gap != self.playhead;
        let expanded_before = self.expanded.clone();
        if let GapDisplayLocation::BeforeNode(FeatureNodeId::Operation(id)) = gap.display {
            let mut ancestors: Vec<&FeatureTreeItem> = Vec::new();
            for item in &self.projection.features {
                while ancestors
                    .last()
                    .is_some_and(|parent| parent.depth >= item.depth)
                {
                    ancestors.pop();
                }
                if item.id == FeatureNodeId::Operation(id) {
                    for parent in ancestors {
                        if let FeatureNodeId::Folder(folder) = parent.id {
                            self.expanded
                                .insert(self.projection.target(TreeTargetKind::Folder(folder)));
                        }
                    }
                    break;
                }
                ancestors.push(item);
            }
        }
        self.playhead = gap;
        self.rebuild_tree_slots();
        Ok(changed || self.expanded != expanded_before)
    }
    pub(super) fn install_projection(
        &mut self,
        projection: ModelTreeProjection,
        initial: TreeInitialState,
    ) {
        if self.interaction.focused.is_some_and(|id| {
            super::tree_decode(id).is_some()
                || (super::RENAME_BASE..super::RENAME_BASE + 4).contains(&id.get())
        }) {
            self.interaction.set_focused(None);
        }
        self.nodes = super::data::projection_nodes(&projection);
        self.update_tree_content_max();
        self.projection = projection;
        self.expanded = initial.expanded;
        self.playhead = initial.playhead;
        self.gap_policy = initial.gap_policy;
        self.selected = None;
        self.tree_focus = None;
        self.tree_scroll = [0, 0];
        self.rename.cancel();
        self.rename_target = None;
        self.pending_tree_edit = None;
    }
    pub fn set_tree_projection(
        &mut self,
        projection: ModelTreeProjection,
    ) -> Result<bool, UiError> {
        projection.validate()?;
        if projection == self.projection {
            return Ok(false);
        }
        let mut candidate = self.clone();
        candidate.reconcile_tree_projection(projection)?;
        *self = candidate;
        Ok(true)
    }
    fn reconcile_tree_projection(
        &mut self,
        projection: ModelTreeProjection,
    ) -> Result<(), UiError> {
        if projection.document != self.projection.document {
            let initial = projection.initial_state();
            initial.validate(&projection)?;
            self.install_projection(projection, initial);
        } else {
            let old_visible = [super::Tree::Features, super::Tree::Elements].map(|tree| {
                self.tree_lines(tree)
                    .iter()
                    .filter_map(|line| match line {
                        super::data::Line::Node(row) => {
                            Some(self.nodes[tree.slot()][row.index].target)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            });
            let anchors = [super::Tree::Features, super::Tree::Elements].map(|tree| {
                self.tree_lines(tree)
                    .iter()
                    .skip(self.tree_scroll[tree.slot()])
                    .find_map(|line| match line {
                        super::data::Line::Node(row) => {
                            old_visible[tree.slot()].iter().position(|target| {
                                *target == self.nodes[tree.slot()][row.index].target
                            })
                        }
                        _ => None,
                    })
            });
            let old_features = self.projection.features.clone();
            self.nodes = super::data::projection_nodes(&projection);
            self.update_tree_content_max();
            self.projection = projection;
            let targets: BTreeSet<_> = self
                .nodes
                .iter()
                .flatten()
                .map(|node| node.target)
                .collect();
            self.expanded.retain(|target| targets.contains(target));
            self.selected = self.selected.filter(|target| targets.contains(target));
            let previous_focus = self.tree_focus;
            self.tree_focus = self.tree_focus.filter(|target| targets.contains(target));
            if previous_focus.is_some() && self.tree_focus.is_none() {
                self.interaction.set_focused(None);
            }
            if let Some((target, source)) = &self.rename_target
                && !self
                    .nodes
                    .iter()
                    .flatten()
                    .any(|node| node.target == *target && node.label == *source)
            {
                self.rename.cancel();
                self.rename_target = None;
            }
            if self.projection.placed_gap(self.playhead.display).ok() != Some(self.playhead) {
                let anchor = match self.playhead.insertion {
                    HistoryGap::Before(id) => {
                        let index = old_features
                            .iter()
                            .position(|item| item.id == FeatureNodeId::Operation(id))
                            .unwrap_or(old_features.len());
                        old_features[index..].iter().find_map(|item| match item.id {
                            FeatureNodeId::Operation(id)
                                if self
                                    .projection
                                    .features
                                    .iter()
                                    .any(|candidate| candidate.id == item.id) =>
                            {
                                Some(id)
                            }
                            _ => None,
                        })
                    }
                    HistoryGap::End => None,
                };
                self.playhead = anchor.map_or(PlacedHistoryGap::default(), |id| PlacedHistoryGap {
                    insertion: HistoryGap::Before(id),
                    display: GapDisplayLocation::BeforeNode(FeatureNodeId::Operation(id)),
                });
            }
            for tree in [super::Tree::Features, super::Tree::Elements] {
                let slot = tree.slot();
                if let Some(index) = anchors[slot] {
                    let old = &old_visible[slot];
                    let target = old[index..]
                        .iter()
                        .chain(old[..index].iter().rev())
                        .find(|target| targets.contains(target))
                        .copied();
                    self.tree_scroll[slot] = self
                        .tree_lines(tree)
                        .iter()
                        .position(|line| match line {
                            super::data::Line::Node(row) => {
                                Some(self.nodes[slot][row.index].target) == target
                            }
                            _ => false,
                        })
                        .unwrap_or(0);
                }
            }
        }
        // Retention validates the pair but never runs explicit-placement reveal behavior.
        if self.projection.placed_gap(self.playhead.display)? != self.playhead {
            return Err(UiError::InvalidTreeProjection);
        }
        self.rebuild_tree_slots();
        if let Some(target) = self.tree_focus
            && let Some((tree, index)) = [super::Tree::Features, super::Tree::Elements]
                .into_iter()
                .find_map(|tree| {
                    self.nodes[tree.slot()]
                        .iter()
                        .position(|node| node.target == target)
                        .map(|index| (tree, index))
                })
        {
            let nodes = &self.nodes[tree.slot()];
            let mut depth = nodes[index].depth;
            for parent in nodes[..index].iter().rev() {
                if parent.depth < depth {
                    self.expanded.insert(parent.target);
                    depth = parent.depth;
                }
            }
            let lines = self.tree_lines(tree);
            if let Some(line) = lines
                .iter()
                .position(|line| matches!(line, super::data::Line::Node(row) if row.index == index))
            {
                let height = match tree {
                    super::Tree::Features => self.layout().features,
                    super::Tree::Elements => self.layout().elements,
                }
                .inset(1)
                .height() as usize;
                let offset = &mut self.tree_scroll[tree.slot()];
                if line < *offset {
                    *offset = line;
                } else if line >= offset.saturating_add(height) {
                    *offset = line.saturating_add(1).saturating_sub(height);
                }
            }
        }
        self.sync();
        Ok(())
    }
    pub(super) fn decode_tree(
        &self,
        id: crate::WidgetId,
    ) -> Option<(super::Tree, usize, TreePart)> {
        let (tree, slot, part) = super::tree_decode(id)?;
        self.tree_slots[tree.slot()]
            .get(slot)
            .copied()
            .map(|index| (tree, index, part))
    }
    pub(super) fn node_id(
        &self,
        tree: super::Tree,
        index: usize,
        part: TreePart,
    ) -> crate::WidgetId {
        let slot = self.tree_slots[tree.slot()]
            .iter()
            .position(|current| *current == index)
            .unwrap_or(super::TREE_SLOT_CAP as usize);
        super::tree_part_id(tree, slot, part)
    }
    pub(super) fn rebuild_tree_slots(&mut self) {
        for (tree, frame) in [
            (super::Tree::Features, self.layout().features),
            (super::Tree::Elements, self.layout().elements),
        ] {
            let lines = self.tree_lines(tree);
            let inner = frame.inset(1);
            let height = if inner.width() == 0 {
                0
            } else {
                inner.height() as usize
            };
            self.tree_scroll[tree.slot()] =
                self.tree_scroll[tree.slot()].min(lines.len().saturating_sub(height));
            self.tree_slots[tree.slot()] = lines
                .iter()
                .skip(self.tree_scroll[tree.slot()])
                .take(height)
                .filter_map(|line| match line {
                    super::data::Line::Node(row) => Some(row.index),
                    _ => None,
                })
                .take(super::TREE_SLOT_CAP as usize)
                .collect();
        }
        if !self.modal_open()
            && let Some(target) = self.tree_focus
        {
            let focus = [super::Tree::Features, super::Tree::Elements]
                .into_iter()
                .find_map(|tree| {
                    self.tree_slots[tree.slot()]
                        .iter()
                        .position(|index| self.nodes[tree.slot()][*index].target == target)
                        .map(|slot| super::tree_id(tree, slot))
                });
            self.interaction.set_focused(focus);
        }
    }
    #[must_use]
    pub fn tree_hit(
        &self,
        id: crate::WidgetId,
        generation: ProjectionGeneration,
    ) -> Option<TreeHitToken> {
        if !self.input_free() {
            return None;
        }
        let (tree, index, part) = self.decode_tree(id)?;
        Some(TreeHitToken {
            generation,
            target: self.nodes[tree.slot()][index].target,
            part,
        })
    }
    #[must_use]
    pub fn validate_tree_hit(
        &self,
        id: crate::WidgetId,
        token: TreeHitToken,
        generation: ProjectionGeneration,
    ) -> bool {
        token.generation == generation && self.tree_hit(id, generation) == Some(token)
    }
    pub fn activate_tree_hit(
        &mut self,
        id: crate::WidgetId,
        token: TreeHitToken,
        generation: ProjectionGeneration,
        keyboard: bool,
    ) -> bool {
        if !self.validate_tree_hit(id, token, generation) {
            return false;
        }
        if keyboard && self.tree_focus != Some(token.target) {
            return false;
        }
        let Some((tree, index, part)) = self.decode_tree(id) else {
            return false;
        };
        if self.editing_field().is_some() {
            self.numeric[self.operation.slot()].commit();
        }
        self.activate_tree(
            tree,
            index,
            if keyboard && part == TreePart::Row {
                TreePart::Marker
            } else {
                part
            },
        );
        if keyboard {
            self.selected = Some(token.target);
        }
        self.sync();
        true
    }
    #[must_use]
    pub fn rename_source(&self) -> Option<(TreeTarget, &str)> {
        self.rename_target
            .as_ref()
            .map(|(target, source)| (*target, source.as_str()))
    }
    pub fn take_tree_edit(&mut self) -> Option<(TreeTarget, TreeIntentKind)> {
        self.pending_tree_edit.take()
    }
    pub fn take_tree_intent(
        &mut self,
        generation: ProjectionGeneration,
        origin_generation: ProjectionGeneration,
    ) -> Option<TreeIntent> {
        self.pending_tree_edit
            .take()
            .map(|(target, kind)| TreeIntent {
                generation,
                origin_generation,
                target,
                kind,
            })
    }
    #[must_use]
    pub fn same_tree_routes(&self, other: &Self) -> bool {
        self.input_free() == other.input_free()
            && self.layout().features == other.layout().features
            && self.layout().elements == other.layout().elements
            && self.view_panel_visible(0) == other.view_panel_visible(0)
            && (0..2).all(|tree| {
                self.tree_slots[tree]
                    .iter()
                    .map(|index| self.nodes[tree][*index].target)
                    .eq(other.tree_slots[tree]
                        .iter()
                        .map(|index| other.nodes[tree][*index].target))
            })
    }
    pub fn cancel_tree_interactions(&mut self) {
        self.rename.cancel();
        self.rename_target = None;
        self.pending_tree_edit = None;
        self.sync();
    }
    pub fn scroll_tree_at(&mut self, col: u32, row: u32, steps: i32) -> Option<bool> {
        if !self.input_free() {
            return Some(false);
        }
        let tree = [
            (super::Tree::Features, self.layout().features),
            (super::Tree::Elements, self.layout().elements),
        ]
        .into_iter()
        .find_map(|(tree, frame)| {
            (col >= frame.col()
                && col < frame.col().saturating_add(frame.width())
                && row >= frame.row()
                && row < frame.row().saturating_add(frame.height()))
            .then_some(tree)
        })?;
        let old = self.tree_scroll[tree.slot()];
        let magnitude = steps.unsigned_abs() as usize;
        self.tree_scroll[tree.slot()] = if steps >= 0 {
            old.saturating_sub(magnitude)
        } else {
            old.saturating_add(magnitude)
        };
        self.rebuild_tree_slots();
        Some(old != self.tree_scroll[tree.slot()])
    }
}
