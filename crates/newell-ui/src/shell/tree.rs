// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Model-independent document projections and semantic tree presentation.

use crate::UiError;
use crate::create_box::ActionBoxUi;
use std::collections::{BTreeMap, BTreeSet};

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
    /// Historical fixture state or the aggregate visibility of a live Base operation's outputs.
    pub visibility: Option<bool>,
}
/// One entity in the ordered operation/output stream, before Elements grouping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementTreeItem {
    pub id: OpaqueTreeId,
    pub source_feature: OpaqueTreeId,
    /// Whether the producer is seeded geometry outside operation history.
    pub seeded: bool,
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
/// Built-in world references, distinct from operation and model-entity identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OriginReference {
    O,
    X,
    Y,
    Z,
    XY,
    XZ,
    YZ,
}
impl OriginReference {
    pub const ALL: [Self; 7] = [
        Self::O,
        Self::X,
        Self::Y,
        Self::Z,
        Self::XY,
        Self::XZ,
        Self::YZ,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            Self::O => "O",
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
            Self::XY => "XY",
            Self::XZ => "XZ",
            Self::YZ => "YZ",
        }
    }
}
/// Semantic target kind, independent of visible cell positions and reusable widget ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TreeTargetKind {
    FeaturesRoot,
    OriginFolder,
    OriginReference(OriginReference),
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
        let nodes = super::data::projection_nodes(projection, true);
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
    Select {
        edit: crate::SelectionEdit,
    },
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
                || (!item.seeded && !seen.contains(&FeatureNodeId::Operation(item.source_feature)))
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
    pub fn command_open(&self) -> bool {
        self.panel().showing()
    }
    pub fn take_command_start(&mut self) -> Option<super::ActionOperation> {
        std::mem::take(&mut self.command_started).then_some(self.operation)
    }
    pub fn field_descriptors(&self) -> Vec<crate::SelectionFieldDescriptor> {
        let panels: [&dyn crate::create_box::ActionBoxUi; 5] = [
            &self.extrude,
            &self.revolve,
            &self.loft,
            &self.sweep,
            &self.boolean,
        ];
        panels
            .into_iter()
            .flat_map(|panel| {
                panel
                    .select_keys()
                    .iter()
                    .filter_map(|key| panel.selection_descriptor(*key))
            })
            .collect()
    }
    pub fn selection_field_visible(&self, field: crate::SelectionFieldKey) -> bool {
        field.operation == self.operation
            && self.panel().showing()
            && self.panel().shows(field.key)
            && self.panel().selection_descriptor(field.key).is_some()
    }
    pub fn set_selection_fields(
        &mut self,
        fields: &[crate::SelectionFieldCount],
    ) -> Result<bool, UiError> {
        let descriptors = self.field_descriptors();
        let mut values = std::collections::BTreeMap::new();
        for item in fields {
            if !descriptors.iter().any(|d| d.field == item.field)
                || values.insert(item.field, item.count).is_some()
            {
                return Err(UiError::InvalidSelection);
            }
        }
        let changed = values != self.selection_field_counts;
        self.reference_fixture = false;
        self.selection_field_counts = values;
        for operation in [
            super::ActionOperation::Extrude,
            super::ActionOperation::Revolve,
            super::ActionOperation::Loft,
            super::ActionOperation::Sweep,
            super::ActionOperation::Boolean,
        ] {
            let counts = fields
                .iter()
                .filter(|f| f.field.operation == operation)
                .map(|f| (f.field.key, f.count))
                .collect();
            match operation {
                super::ActionOperation::Extrude => self.extrude.set_selection_counts(&counts),
                super::ActionOperation::Revolve => self.revolve.set_selection_counts(&counts),
                super::ActionOperation::Loft => self.loft.set_selection_counts(&counts),
                super::ActionOperation::Sweep => self.sweep.set_selection_counts(&counts),
                super::ActionOperation::Boolean => self.boolean.set_selection_counts(&counts),
            }
        }
        Ok(changed)
    }
    pub fn take_selection_request(&mut self) -> Option<crate::SelectionUiRequest> {
        self.selection_request.take().or_else(|| {
            self.selection
                .take_completion()
                .map(crate::SelectionUiRequest::Complete)
        })
    }
    pub fn open_selection(&mut self, request: crate::SelectionOpen) -> Result<(), UiError> {
        let qualified = matches!(request.destination,crate::SelectionDestination::Field{session,field} if field.geometry_qualified() && self.numeric_identity==Some((request.document,session,field.operation)) && self.selection_field_visible(field));
        if request.document != self.projection.document
            || (!qualified
                && request
                    .rows
                    .iter()
                    .any(|r| !self.projection.elements.iter().any(|e| e.id == r.entity)))
        {
            return Err(UiError::InvalidSelection);
        }
        self.selection.open_with(request, &mut self.interaction)?;
        self.sync();
        Ok(())
    }
    pub fn reconcile_selection_rows(
        &mut self,
        rows: Vec<crate::SelectionRow>,
    ) -> Result<bool, UiError> {
        let changed = self.selection.reconcile_rows(rows)?;
        self.sync();
        Ok(changed)
    }
    pub fn entity_selection(&self) -> &crate::EntitySelectionView {
        &self.entity_selection
    }
    pub fn origin_selection(&self) -> &[OriginReference] {
        &self.origin_selection
    }
    pub fn selection_field_count(&self, field: crate::SelectionFieldKey) -> usize {
        self.selection_field_counts
            .get(&field)
            .copied()
            .unwrap_or(0)
    }
    pub fn set_entity_selection(
        &mut self,
        view: crate::EntitySelectionView,
    ) -> Result<bool, UiError> {
        view.validate(&self.projection)?;
        let changed = self.entity_selection != view;
        self.entity_selection = view;
        Ok(changed)
    }
    pub fn set_origin_selection(
        &mut self,
        references: Vec<OriginReference>,
    ) -> Result<bool, UiError> {
        let unique: BTreeSet<_> = references.iter().copied().collect();
        if unique.len() != references.len() {
            return Err(UiError::InvalidSelection);
        }
        let mut changed = self.origin_selection != references;
        self.origin_selection = references;
        if !self.origin_selection.is_empty() {
            let expanded_before = self.expanded.clone();
            let scroll_before = self.tree_scroll[0];
            let folder = self.projection.target(TreeTargetKind::OriginFolder);
            self.expanded.insert(folder);
            self.rebuild_tree_slots();
            let inner = self.layout().features.inset(1);
            let height = inner.height() as usize;
            for reference in &self.origin_selection {
                let target = self
                    .projection
                    .target(TreeTargetKind::OriginReference(*reference));
                let Some(index) = self.nodes[0].iter().position(|node| node.target == target)
                else {
                    continue;
                };
                let Some(line) = self.tree_lines(super::Tree::Features).iter().position(
                    |line| matches!(line, super::data::Line::Node(row) if row.index == index),
                ) else {
                    continue;
                };
                let offset = &mut self.tree_scroll[0];
                if line < *offset {
                    *offset = line;
                } else if height > 0 && line >= offset.saturating_add(height) {
                    *offset = line.saturating_add(1).saturating_sub(height);
                }
            }
            changed |= self.expanded != expanded_before || self.tree_scroll[0] != scroll_before;
        }
        if changed {
            self.sync();
        }
        Ok(changed)
    }
    pub fn operation_selection(&self, id: OpaqueTreeId) -> (usize, usize) {
        let outputs: Vec<_> = self
            .projection
            .elements
            .iter()
            .filter(|e| e.source_feature == id)
            .collect();
        (
            outputs
                .iter()
                .filter(|e| self.entity_selection.entities.contains(&e.id))
                .count(),
            outputs.len(),
        )
    }
    pub fn is_entity_target_selected(&self, target: TreeTarget) -> bool {
        if target.document != self.entity_selection.document {
            return false;
        }
        match target.kind {
            TreeTargetKind::Entity(id) => self.entity_selection.entities.contains(&id),
            TreeTargetKind::Operation(id) => {
                let (selected, total) = self.operation_selection(id);
                total > 0 && selected == total
            }
            TreeTargetKind::OriginReference(reference) => {
                self.origin_selection.contains(&reference)
            }
            _ => false,
        }
    }
    pub fn set_selection_modifiers(&mut self, shift: bool, toggle: bool) {
        self.selection_edit = crate::SelectionEdit::modifiers(shift, toggle);
    }
    pub(super) fn request_entity_selection(&mut self, target: TreeTarget) {
        if !matches!(
            target.kind,
            TreeTargetKind::Entity(_)
                | TreeTargetKind::Operation(_)
                | TreeTargetKind::OriginReference(_)
        ) {
            return;
        }
        self.pending_tree_edit = Some((
            target,
            TreeIntentKind::Select {
                edit: self.selection_edit,
            },
        ));
        if let TreeTargetKind::OriginReference(reference) = target.kind
            && self.reference_fixture
        {
            let mut selected = self.origin_selection.clone();
            match self.selection_edit {
                crate::SelectionEdit::Replace => selected = vec![reference],
                crate::SelectionEdit::Add => {
                    if !selected.contains(&reference) {
                        selected.push(reference);
                    }
                }
                crate::SelectionEdit::Toggle => {
                    if let Some(index) = selected.iter().position(|value| *value == reference) {
                        selected.remove(index);
                    } else {
                        selected.push(reference);
                    }
                }
                crate::SelectionEdit::Remove => selected.retain(|value| *value != reference),
                crate::SelectionEdit::Clear => selected.clear(),
            }
            let _ = self.set_origin_selection(selected);
            return;
        }
        // The explicit artwork fixture has its own fake document responder. Live windows
        // only receive a copy from their application owner after consuming this intent.
        if self.reference_fixture {
            let ids: Vec<_> = self
                .projection
                .elements
                .iter()
                .filter(|e| match target.kind {
                    TreeTargetKind::Entity(id) => e.id == id,
                    TreeTargetKind::Operation(id) => e.source_feature == id,
                    _ => false,
                })
                .map(|e| e.id)
                .collect();
            let selected = &mut self.entity_selection.entities;
            match self.selection_edit {
                crate::SelectionEdit::Replace => *selected = ids,
                crate::SelectionEdit::Add => {
                    for id in ids {
                        if !selected.contains(&id) {
                            selected.push(id);
                        }
                    }
                }
                crate::SelectionEdit::Toggle => {
                    for id in ids {
                        if let Some(i) = selected.iter().position(|v| *v == id) {
                            selected.remove(i);
                        } else {
                            selected.push(id);
                        }
                    }
                }
                crate::SelectionEdit::Remove => selected.retain(|id| !ids.contains(id)),
                crate::SelectionEdit::Clear => selected.clear(),
            }
        }
    }
    pub(super) fn history_index(&self, target: TreeTarget) -> Option<usize> {
        let id = match target.kind {
            TreeTargetKind::Operation(id) => FeatureNodeId::Operation(id),
            TreeTargetKind::Folder(id) => FeatureNodeId::Folder(id),
            _ => return None,
        };
        self.projection
            .features
            .iter()
            .position(|item| item.id == id)
    }
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
            while ancestors
                .last()
                .is_some_and(|i| items[*i].depth >= item.depth)
            {
                ancestors.pop();
            }
            if item.id == id {
                ancestors.push(index);
                return ancestors
                    .iter()
                    .map(|i| {
                        (*i == index && before)
                            || items[*i + 1..]
                                .iter()
                                .find(|next| next.depth <= items[*i].depth)
                                .is_some_and(|next| next.depth == items[*i].depth)
                    })
                    .collect();
            }
            ancestors.push(index);
        }
        Vec::new()
    }

    /// The outermost collapsed strict ancestor is the only visible inline host.
    pub(super) fn inline_playhead_host(&self) -> Option<TreeTarget> {
        if !self
            .expanded
            .contains(&self.projection.target(TreeTargetKind::FeaturesRoot))
        {
            return None;
        }
        let id = match self.playhead.display {
            GapDisplayLocation::BeforeNode(id) | GapDisplayLocation::AfterSubtree(id) => id,
            GapDisplayLocation::EndOfTree => return None,
        };
        let mut ancestors: Vec<&FeatureTreeItem> = Vec::new();
        for item in &self.projection.features {
            while ancestors
                .last()
                .is_some_and(|parent| parent.depth >= item.depth)
            {
                ancestors.pop();
            }
            if item.id == id {
                return ancestors.into_iter().find_map(|parent| {
                    let FeatureNodeId::Folder(folder) = parent.id else {
                        return None;
                    };
                    let target = self.projection.target(TreeTargetKind::Folder(folder));
                    (!self.expanded.contains(&target)).then_some(target)
                });
            }
            ancestors.push(item);
        }
        None
    }

    pub(super) fn update_tree_content_max(&mut self) {
        let natural = self
            .nodes
            .iter()
            .map(|nodes| {
                nodes
                    .iter()
                    .enumerate()
                    .map(|(index, node)| {
                        let kind = if super::data::has_children(nodes, index) {
                            crate::TreeNodeKind::Branch
                        } else {
                            crate::TreeNodeKind::Leaf
                        };
                        let prefix = crate::TreeRow::prefix_width(
                            node.depth,
                            kind,
                            node.kind == super::data::NodeKind::Named,
                            node.check.is_some(),
                        );
                        prefix.saturating_add(crate::widgets::text_len(&super::data::full_label(
                            nodes, index, None,
                        )))
                    })
                    .max()
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0);
        self.tree_content_max = natural.saturating_add(4);
        self.widths.nav = self.widths.nav.min(self.tree_content_max);
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
        self.nodes = super::data::projection_nodes(&projection, !self.reference_fixture);
        self.update_tree_content_max();
        self.projection = projection;
        self.expanded = initial.expanded;
        self.playhead = initial.playhead;
        self.gap_policy = initial.gap_policy;
        self.entity_selection = crate::EntitySelectionView {
            document: self.projection.document,
            entities: Vec::new(),
        };
        self.origin_selection.clear();
        if !self.reference_fixture && self.panel().showing() {
            let cancel = crate::WidgetId::new(
                super::ACTION_BASE
                    + crate::create_box::K_ACTIONS * crate::action_box::KEY_SLOTS
                    + 1,
            );
            self.panel_mut().activate_id(cancel);
            self.numeric[self.operation.slot()].reset();
        }
        self.selection.dismiss();
        self.selection_request = None;
        self.tree_focus = None;
        self.properties_request = None;
        self.properties_origin = None;
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
        if let Some(request) = candidate.selection.request().cloned()
            && candidate.selection.is_open()
        {
            if request.document != candidate.projection.document
                || matches!(request.destination, crate::SelectionDestination::Field { field, .. } if field.geometry_qualified() || !candidate.selection_field_visible(field))
            {
                candidate.selection.dismiss();
                candidate.sync();
            } else {
                let rows = candidate
                    .projection
                    .elements
                    .iter()
                    .filter(|e| {
                        matches!(request.destination, crate::SelectionDestination::Window)
                            || request.rows.iter().any(|r| r.entity == e.id)
                    })
                    .map(|e| crate::SelectionRow {
                        entity: e.id,
                        label: e.name.clone(),
                    })
                    .collect();
                candidate.reconcile_selection_rows(rows)?;
            }
        }
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
            self.nodes = super::data::projection_nodes(&projection, !self.reference_fixture);
            self.update_tree_content_max();
            self.projection = projection;
            let targets: BTreeSet<_> = self
                .nodes
                .iter()
                .flatten()
                .map(|node| node.target)
                .collect();
            self.expanded.retain(|target| targets.contains(target));
            self.entity_selection
                .entities
                .retain(|id| self.projection.elements.iter().any(|e| e.id == *id));
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
                    // A surviving semantic row can now be hidden by a collapsed folder.
                    // Only visible rows can anchor the new frame; otherwise continue in
                    // the old visible order before falling back to a predecessor.
                    let visible_positions: BTreeMap<_, _> = self
                        .tree_lines(tree)
                        .iter()
                        .enumerate()
                        .filter_map(|(position, line)| match line {
                            super::data::Line::Node(row) => {
                                Some((self.nodes[slot][row.index].target, position))
                            }
                            _ => None,
                        })
                        .collect();
                    self.tree_scroll[slot] = old[index..]
                        .iter()
                        .chain(old[..index].iter().rev())
                        .find_map(|target| visible_positions.get(target).copied())
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
            if keyboard
                && part == TreePart::Row
                && !matches!(
                    token.target.kind,
                    TreeTargetKind::Operation(_) | TreeTargetKind::Entity(_)
                )
            {
                TreePart::Marker
            } else {
                part
            },
        );
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

#[cfg(test)]
mod origin_reference_tests {
    use super::*;
    use crate::shell::data::Line;
    use crate::shell::{ShellState, Tree};

    fn shell() -> ShellState {
        let mut shell = ShellState::new_application(100, 60);
        let mut p = ModelTreeProjection::empty(TreeDocumentId::new(9));
        p.features.push(FeatureTreeItem {
            id: FeatureNodeId::Operation(OpaqueTreeId::new(1)),
            name: "Operation".into(),
            depth: 1,
            visibility: None,
        });
        shell.set_tree_projection(p).unwrap();
        shell
    }
    fn visible(shell: &ShellState) -> Vec<TreeTargetKind> {
        shell
            .tree_lines(Tree::Features)
            .iter()
            .filter_map(|l| match l {
                Line::Node(row) => Some(shell.nodes[0][row.index].target.kind),
                _ => None,
            })
            .collect()
    }
    #[test]
    fn seed_only_document_has_origin_then_end_without_operation_rows() {
        let mut s = ShellState::new_application(100, 60);
        let mut p = ModelTreeProjection::empty(TreeDocumentId::new(9));
        p.elements.push(ElementTreeItem {
            id: OpaqueTreeId::new(1),
            source_feature: OpaqueTreeId::new(0),
            seeded: true,
            output_ordinal: 0,
            group: ElementGroup::Bodies,
            name: "Base Solid".into(),
            visible: true,
        });
        s.set_tree_projection(p).unwrap();
        assert_eq!(
            visible(&s),
            vec![TreeTargetKind::FeaturesRoot, TreeTargetKind::OriginFolder]
        );
        assert_eq!(s.playhead, PlacedHistoryGap::default());
        assert_eq!(s.projection.elements.len(), 1);
        assert!(s.projection.features.is_empty());
    }

    #[test]
    fn origin_reference_folder_order_identity_and_expansion_are_presentation_only() {
        let mut s = shell();
        let p = s.projection.clone();
        let gap = s.playhead;
        assert_eq!(
            visible(&s),
            vec![
                TreeTargetKind::FeaturesRoot,
                TreeTargetKind::OriginFolder,
                TreeTargetKind::Operation(OpaqueTreeId::new(1))
            ]
        );
        assert_eq!(s.nodes[0][1].label, "Origin");
        assert_eq!(
            s.nodes[0][2..9]
                .iter()
                .map(|n| n.label.as_str())
                .collect::<Vec<_>>(),
            vec!["O", "X", "Y", "Z", "XY", "XZ", "YZ"]
        );
        let targets = s.nodes[0][1..9]
            .iter()
            .map(|n| n.target)
            .collect::<Vec<_>>();
        assert_eq!(targets.iter().collect::<BTreeSet<_>>().len(), 8);
        assert!(
            targets
                .iter()
                .all(|t| s.history_index(*t).is_none() && !s.rename_eligible(*t))
        );
        assert!(s.nodes[0][1..9].iter().all(|n| n.check.is_none()));
        s.activate_tree(Tree::Features, 1, TreePart::Marker);
        assert_eq!(visible(&s).len(), 10);
        for reference in OriginReference::ALL {
            assert!(visible(&s).contains(&TreeTargetKind::OriginReference(reference)));
        }
        for i in 1..9 {
            s.activate_tree(Tree::Features, i, TreePart::Row);
            let target = s.nodes[0][i].target;
            if let TreeTargetKind::OriginReference(reference) = target.kind {
                assert!(matches!(
                    s.take_tree_edit(),
                    Some((same, TreeIntentKind::Select { .. })) if same == target
                ));
                s.set_origin_selection(vec![reference]).unwrap();
                assert!(s.is_entity_target_selected(target));
                s.set_origin_selection(Vec::new()).unwrap();
            } else {
                assert!(s.take_tree_edit().is_none());
                assert!(!s.is_entity_target_selected(target));
            }
            s.activate_tree(Tree::Features, i, TreePart::Check);
            assert!(s.take_tree_edit().is_none());
        }
        assert_eq!(s.playhead, gap);
        assert_eq!(s.projection, p);
        assert!(!s.nodes[1].iter().any(|n| targets.contains(&n.target)));
        s.activate_tree(Tree::Features, 1, TreePart::Marker);
        assert_eq!(visible(&s).len(), 3);
        let mut updated = p.clone();
        updated.features[0].name = "Renamed operation".into();
        s.set_tree_projection(updated).unwrap();
        assert_eq!(
            targets,
            s.nodes[0][1..9]
                .iter()
                .map(|n| n.target)
                .collect::<Vec<_>>()
        );
        assert_eq!(s.projection.elements, p.elements);
    }

    #[test]
    fn origin_selection_expands_and_minimally_scrolls_the_selected_reference_into_view() {
        let mut s = ShellState::new_application(54, 15);
        s.set_tree_projection(ModelTreeProjection::empty(TreeDocumentId::new(9)))
            .unwrap();
        s.set_origin_selection(vec![OriginReference::YZ]).unwrap();
        let target = s
            .projection
            .target(TreeTargetKind::OriginReference(OriginReference::YZ));
        assert!(
            s.expanded
                .contains(&s.projection.target(TreeTargetKind::OriginFolder))
        );
        assert!(s.is_entity_target_selected(target));
        let row_index = s.nodes[0]
            .iter()
            .position(|node| node.target == target)
            .unwrap();
        let row_line = s
            .tree_lines(Tree::Features)
            .iter()
            .position(|line| matches!(line, Line::Node(row) if row.index == row_index))
            .unwrap();
        let height = s.layout().features.inset(1).height() as usize;
        assert!(row_line >= s.tree_scroll[Tree::Features.slot()]);
        assert!(row_line < s.tree_scroll[Tree::Features.slot()] + height);
        assert_eq!(s.origin_selection(), &[OriginReference::YZ]);
    }
    #[test]
    fn origin_reference_rows_never_host_or_move_operation_insertion() {
        let mut s = shell();
        let gap = s
            .projection
            .placed_gap(GapDisplayLocation::BeforeNode(s.projection.features[0].id))
            .unwrap();
        s.set_playhead(gap).unwrap();
        for expanded in [false, true] {
            if expanded {
                s.activate_tree(Tree::Features, 1, TreePart::Marker);
            }
            let lines = s.tree_lines(Tree::Features);
            let playhead = lines
                .iter()
                .position(|l| matches!(l, Line::Playhead))
                .unwrap();
            let operation=lines.iter().position(|l|matches!(l,Line::Node(r) if s.nodes[0][r.index].target.kind==TreeTargetKind::Operation(OpaqueTreeId::new(1)))).unwrap();
            assert_eq!(playhead + 1, operation);
            let first = s.layout().features.inset(1).row();
            for (row, line) in lines.iter().enumerate() {
                if matches!(line,Line::Node(r) if matches!(s.nodes[0][r.index].target.kind,TreeTargetKind::OriginFolder|TreeTargetKind::OriginReference(_)))
                {
                    assert!(!s.drag_playhead(first + row as u32));
                    assert_eq!(s.playhead, gap);
                }
            }
            assert!(s.inline_playhead_host().is_none());
        }
        let empty = ModelTreeProjection::empty(TreeDocumentId::new(10));
        s.set_tree_projection(empty).unwrap();
        assert!(matches!(
            s.tree_lines(Tree::Features).last(),
            Some(Line::Playhead)
        ));
        assert_eq!(s.playhead.insertion, HistoryGap::End);
    }
}
