// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Per-window shell state, input routing and pure flat-grid composition.
//!
//! Features and Elements consume a model-independent projection; names and visibility
//! come only from that projection. Expansion, selection, semantic focus, scrolling and
//! insertion-cursor placement belong to this window. Visible slots reuse widget ids.
//! The application controller publishes the grid and semantic routing with a generation;
//! tree actions validate a captured semantic token before activation.
//!
//! Modal input precedes menu input, then base-shell controls. A tree cannot receive input
//! behind either overlay owner. The existing action panels, numeric editors, focus ring,
//! ASCII row renderer and shared-border hit routing are retained.
//!
//! ShellState::new supplies the historical reference fixture for artwork tests.
//! ShellState::new_application starts with empty trees until the app publishes a document.
//! No model or geometric-kernel type enters this crate.

mod compose;
mod data;
mod key;
mod tree;
pub use tree::*;

use data::{
    ACT_BOOLEAN, ACT_EXTRUDE, ACT_LOFT, ACT_REVOLVE, ACT_SWEEP, ACT_VIEW_CHECK, ACT_VIEW_COLOR,
    ACT_VIEW_FULLSCREEN, ACT_VIEW_PROJECTION, ACT_VIEW_SCALE, ACT_VIEW_SHADING, ACT_VIEW_THEME,
    ACT_WORKBENCH, COMMAND_MENUS, Line, PROJECTION, SHADING, ViewMock, has_children,
    with_bar_menus,
};

pub use data::ThemeChoice;
pub use key::ShellKey;

use crate::create_box::{ActionBoxUi, ID_SPAN, K_ACTIONS};
use crate::overlays::step_focus;
use crate::widgets::NumericFields;
use crate::{
    BooleanUi, CycleStep, EditKey, ExtrudeStatus, ExtrudeUi, FormulaEditor, GridRect,
    HitTestResult, InteractionState, LoftUi, MAX_MNEMONIC, MenuIds, MenuKey, MenuResult, MenuState,
    MenuTarget, Mnemonic, ModalKey, OverlayStack, RenameModal, RevolveUi, SCALE_FACTORS_PERCENT,
    SelectionPanel, SelectionThumbGrab, SweepUi, WidgetId, WindowAction, cycle_next, cycle_prev,
};
use compose::{PanelWidths, Split, features_rows};
use std::collections::BTreeSet;

pub use compose::Separator;

const MENU_BAR_BASE: u32 = 1_000_000;
const COMMAND_BAR_BASE: u32 = MENU_BAR_BASE + MenuIds::SPAN;
const TREE_BASE: u32 = COMMAND_BAR_BASE + MenuIds::SPAN;
/// Ids per tree node: the row itself, its marker, its checkbox and its name (see [`TreePart`]).
const TREE_STRIDE: u32 = 4;
/// Ids of the node rows: features first, then elements; the playhead id follows them.
const TREE_SLOT_CAP: u32 = crate::MAX_CELLS;
const NODE_IDS: u32 = 2 * TREE_SLOT_CAP * TREE_STRIDE;
/// The playhead separator row of the Features tree.
const PLAYHEAD_ID: u32 = TREE_BASE + NODE_IDS;
/// Ids reserved for the two trees and the playhead.
const TREE_SPAN: u32 = NODE_IDS + 1;
/// First id of the Action Box. The five boxes share this range: only one is shown at a time.
const ACTION_BASE: u32 = TREE_BASE + TREE_SPAN;
const SELECTION_BASE: u32 = ACTION_BASE + ID_SPAN;
/// The SelectionPanel uses 14 ids from its base.
const FORMULA_BASE: u32 = SELECTION_BASE + SelectionPanel::ID_SPAN;
/// The Rename modal uses 4 ids from its base.
const RENAME_BASE: u32 = FORMULA_BASE + 16;
/// The three window controls `[-] [□] [X]`, in that order.
const WINDOW_BASE: u32 = RENAME_BASE + 16;
const PROGRESS_ID: u32 = WINDOW_BASE + 3;
const WINDOW_ACTIONS: [WindowAction; 3] = [
    WindowAction::Minimize,
    WindowAction::ToggleMaximize,
    WindowAction::Close,
];

const BAR_IDS: MenuIds = MenuIds::new(MENU_BAR_BASE);
const COMMAND_IDS: MenuIds = MenuIds::new(COMMAND_BAR_BASE);

/// The operation whose Action Box is shown in the ActionPanel (`Create` and `Modify` menus).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ActionOperation {
    #[default]
    Extrude,
    Revolve,
    Loft,
    Sweep,
    Boolean,
}

impl ActionOperation {
    /// Index into the per-operation state arrays.
    const fn slot(self) -> usize {
        self as usize
    }

    /// The operation a CommandBar action opens, if it is one of the five.
    const fn from_action(action: u32) -> Option<Self> {
        match action {
            ACT_EXTRUDE => Some(Self::Extrude),
            ACT_REVOLVE => Some(Self::Revolve),
            ACT_LOFT => Some(Self::Loft),
            ACT_SWEEP => Some(Self::Sweep),
            ACT_BOOLEAN => Some(Self::Boolean),
            _ => None,
        }
    }
}

/// Which mock tree a row belongs to. They are different data sources (README §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tree {
    Features,
    Elements,
}

impl Tree {
    const fn offset(self) -> u32 {
        match self {
            Self::Features => 0,
            Self::Elements => TREE_SLOT_CAP,
        }
    }

    /// Index into the per-tree state arrays.
    const fn slot(self) -> usize {
        match self {
            Self::Features => 0,
            Self::Elements => 1,
        }
    }
}

/// `true` if `id` lies in the Action Box id range.
fn in_action_panel(id: WidgetId) -> bool {
    id.get()
        .checked_sub(ACTION_BASE)
        .is_some_and(|rel| rel < ID_SPAN)
}

/// Stable id of `part` of tree node `index`, independent of which rows are visible.
fn tree_part_id(tree: Tree, index: usize, part: TreePart) -> WidgetId {
    let node = tree.offset() + u32::try_from(index).unwrap_or(0);
    WidgetId::new(TREE_BASE + node * TREE_STRIDE + part as u32)
}

/// The row id of tree node `index`: the keyboard focus target and the activation target of
/// Enter / Space.
fn tree_id(tree: Tree, index: usize) -> WidgetId {
    tree_part_id(tree, index, TreePart::Row)
}

/// Inverse of [`tree_part_id`] for nodes that exist.
fn tree_decode(id: WidgetId) -> Option<(Tree, usize, TreePart)> {
    let rel = id.get().checked_sub(TREE_BASE)?;
    if rel >= NODE_IDS {
        return None;
    }
    let part = TreePart::ALL[(rel % TREE_STRIDE) as usize];
    let node = rel / TREE_STRIDE;
    let (tree, index) = if node < Tree::Elements.offset() {
        (Tree::Features, node)
    } else {
        (Tree::Elements, node - Tree::Elements.offset())
    };
    let index = usize::try_from(index).ok()?;
    Some((tree, index, part))
}

/// The UI state of one application window. See the module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellState {
    cols: u32,
    rows: u32,
    widths: PanelWidths,
    menu_bar: MenuState,
    command_bar: MenuState,
    /// Mnemonic token typed so far in an open menu (see [`ShellState::typed_char`]).
    typed: String,
    view: ViewMock,
    /// The Fullscreen item was activated and the application has not taken the request yet.
    fullscreen_request: bool,
    camera_reset_request: bool,
    normal_view_request: bool,
    workbench: usize,
    fps: u32,
    progress: u32,
    projection: ModelTreeProjection,
    nodes: [Vec<data::Node>; 2],
    tree_content_max: u32,
    expanded: BTreeSet<TreeTarget>,
    entity_selection: crate::EntitySelectionView,
    reference_fixture: bool,
    selection_edit: crate::SelectionEdit,
    selection_request: Option<crate::SelectionUiRequest>,
    command_started: bool,
    selection_field_counts: std::collections::BTreeMap<crate::SelectionFieldKey, usize>,
    tree_focus: Option<TreeTarget>,
    properties_request: Option<crate::PropertiesOpenRequest>,
    properties_origin: Option<crate::ProjectionGeneration>,
    next_properties_id: u64,
    playhead: PlacedHistoryGap,
    gap_policy: HistoryGapPolicy,
    tree_scroll: [usize; 2],
    tree_slots: [Vec<usize>; 2],
    pending_tree_edit: Option<(TreeTarget, TreeIntentKind)>,
    /// The Action Box shown. The five boxes keep their own state, so switching operations
    /// restores what each one held.
    operation: ActionOperation,
    extrude: ExtrudeUi,
    revolve: RevolveUi,
    loft: LoftUi,
    sweep: SweepUi,
    boolean: BooleanUi,
    selection: SelectionPanel,
    formula: FormulaEditor,
    /// Edited numeric fields and formula drafts of each Action Box, by [`ActionOperation::slot`].
    numeric: [NumericFields; 5],
    /// The numeric field the open FormulaEditor belongs to.
    formula_target: Option<(WidgetId, Option<crate::NumericDestination>)>,
    numeric_retired_session: Option<(TreeDocumentId, crate::CommandInputSessionId)>,
    numeric_identity: Option<(
        TreeDocumentId,
        crate::CommandInputSessionId,
        ActionOperation,
    )>,
    numeric_origin: ProjectionGeneration,
    numeric_sequence: u64,
    numeric_accepted_sequence: u64,
    numeric_attempt: u64,
    numeric_requests: Vec<crate::NumericRequest>,
    numeric_last_edits: std::collections::BTreeMap<crate::NumericFieldKey, u64>,
    numeric_confirm: Option<crate::NumericConfirmRequest>,
    numeric_confirm_outbound: bool,
    numeric_formula_open: Option<crate::NumericFormulaOpenRequest>,
    numeric_formula_outbound: bool,
    formula_modal_session: Option<u64>,
    formula_apply: Option<crate::FormulaApplyRequest>,
    formula_apply_outbound: bool,
    formula_diagnostic: Option<String>,
    numeric_error: Option<crate::UiError>,
    rename: RenameModal,
    /// The row the open Rename modal edits.
    rename_target: Option<(TreeTarget, String)>,
    interaction: InteractionState,
    overlays: OverlayStack,
}

impl ShellState {
    /// The reference shell for a grid of `cols` x `rows` cells: Extrude panel open, trees
    /// as in the reference, no menu or modal open.
    #[must_use]
    pub fn new(cols: u32, rows: u32) -> Self {
        let (projection, initial) = data::reference_trees();
        let nodes = data::projection_nodes(&projection, false);
        let selection_document = projection.document;
        let mut state = Self {
            cols,
            rows,
            widths: PanelWidths::default(),
            menu_bar: MenuState::default(),
            command_bar: MenuState::default(),
            typed: String::new(),
            view: ViewMock::default(),
            fullscreen_request: false,
            camera_reset_request: false,
            normal_view_request: false,
            workbench: 0,
            fps: 0,
            progress: 0,
            projection,
            nodes,
            tree_content_max: u32::MAX,
            expanded: initial.expanded,
            entity_selection: crate::EntitySelectionView {
                document: selection_document,
                entities: Vec::new(),
            },
            reference_fixture: true,
            selection_edit: crate::SelectionEdit::Replace,
            selection_request: None,
            command_started: false,
            selection_field_counts: Default::default(),
            tree_focus: None,
            properties_request: None,
            properties_origin: None,
            next_properties_id: 0,
            playhead: initial.playhead,
            gap_policy: initial.gap_policy,
            tree_scroll: [0, 0],
            tree_slots: Default::default(),
            pending_tree_edit: None,
            operation: ActionOperation::Extrude,
            extrude: ExtrudeUi::new(ACTION_BASE),
            revolve: RevolveUi::new(ACTION_BASE),
            loft: LoftUi::new(ACTION_BASE),
            sweep: SweepUi::new(ACTION_BASE),
            boolean: BooleanUi::new(ACTION_BASE),
            selection: SelectionPanel::reference(SELECTION_BASE),
            formula: FormulaEditor::new(FORMULA_BASE),
            numeric: Default::default(),
            formula_target: None,
            numeric_retired_session: None,
            numeric_identity: None,
            numeric_origin: ProjectionGeneration::new(0),
            numeric_sequence: 0,
            numeric_accepted_sequence: 0,
            numeric_attempt: 0,
            numeric_requests: Vec::new(),
            numeric_last_edits: std::collections::BTreeMap::new(),
            numeric_confirm: None,
            numeric_confirm_outbound: false,
            numeric_formula_open: None,
            numeric_formula_outbound: false,
            formula_modal_session: None,
            formula_apply: None,
            formula_apply_outbound: false,
            formula_diagnostic: None,
            numeric_error: None,
            rename: RenameModal::new(RENAME_BASE),
            rename_target: None,
            interaction: InteractionState::default(),
            overlays: OverlayStack::default(),
        };
        state.sync();
        state
    }

    /// The actual application startup shell: unlike the example composition in `new`, it has
    /// no contextual Action Box open.
    #[must_use]
    pub fn new_application(cols: u32, rows: u32) -> Self {
        let mut state = Self::new(cols, rows);
        state.close_startup_action_box();
        state.reference_fixture = false;
        state.selection = SelectionPanel::new(SELECTION_BASE);
        let projection = ModelTreeProjection::empty(TreeDocumentId::new(0));
        let initial = projection.initial_state();
        state.install_projection(projection, initial);
        state.sync();
        state
    }

    /// Closes the reference Extrude box for an actual application launch.
    fn close_startup_action_box(&mut self) {
        self.extrude.status = ExtrudeStatus::Cancelled;
    }

    /// Grid size `(cols, rows)` the shell is composed for.
    #[must_use]
    pub const fn size(&self) -> (u32, u32) {
        (self.cols, self.rows)
    }

    /// Re-targets the shell to a grid of `cols` x `rows` cells (the whole-cell size of the OS
    /// window). Only the layout changes: every other piece of state, and the panel widths the
    /// user chose, are kept, and open menus and modals are re-placed for the new bounds.
    /// Returns `true` if the size changed (recompose the grid). Any size is valid, zero included.
    pub fn resize(&mut self, cols: u32, rows: u32) -> bool {
        if (cols, rows) == (self.cols, self.rows) {
            return false;
        }
        let focused = self.interaction.focused;
        self.cols = cols;
        self.rows = rows;
        self.sync();
        // Preserve the established resize focus memory even if the pane is temporarily
        // too short to compose any row. Activation still validates semantic focus below.
        if self.tree_focus.is_some() && self.interaction.focused.is_none() {
            self.interaction.set_focused(focused);
        }
        true
    }

    /// NavigationPanel width in cells, frame included, as laid out on the current grid.
    #[must_use]
    pub fn nav_width(&self) -> u32 {
        self.layout().nav
    }

    /// Action Box width in cells, frame included, as laid out on the current grid: always
    /// 31 (`README_UI.md` §2.2), or the viewport interior if that is narrower (the box is then
    /// cropped at its right edge, never squeezed).
    #[must_use]
    pub fn action_width(&self) -> u32 {
        self.layout().action
    }

    /// Selected navigation representation, independent of its visibility checkbox.
    pub fn navigator_kind(&self) -> crate::navigation_widgets::NavigatorKind {
        self.view.navigator_kind
    }

    /// The full viewport interior reserved for the scene renderer.
    ///
    /// The Action Box and NavCube retain their reference positions on the viewport's right;
    /// this rect is expressed in whole grid cells so renderers can map it exactly to physical
    /// pixels.
    #[must_use]
    pub fn scene_viewport(&self) -> GridRect {
        let layout = self.layout();
        layout.viewport.inset(1)
    }

    /// Opaque footprints which must hide the 3D scene below them.
    #[must_use]
    pub fn scene_occlusions(&self) -> Vec<GridRect> {
        let scene = self.scene_viewport();
        let mut rects = Vec::with_capacity(2);
        if let Some(rect) = self.action_panel_rect() {
            rects.push(rect.intersection(&scene));
        }
        if let Some(overlay) = self.overlays.top() {
            rects.push(overlay.rect.intersection(&scene));
        }
        rects.retain(|rect| rect.width() > 0 && rect.height() > 0);
        rects
    }

    /// Asks for a NavigationPanel `wanted` cells wide; the width is clamped to the grid
    /// (at least 8 cells, at most half the grid). Returns `true` if the laid-out width changed.
    pub fn set_nav_width(&mut self, wanted: u32) -> bool {
        let before = (self.nav_width(), self.widths.nav);
        self.widths.nav = wanted.clamp(14.min(self.tree_content_max), self.tree_content_max);
        (self.nav_width(), self.widths.nav) != before
    }

    /// `true` while no menu popup or modal owns input, so panels can be dragged and the
    /// window moved.
    fn input_free(&self) -> bool {
        !(self.modal_open() || self.menu_bar.is_active() || self.command_bar.is_active())
    }

    /// `true` when a viewport interaction is not covered by a menu or modal.
    #[must_use]
    pub fn scene_input_enabled(&self) -> bool {
        self.input_free()
    }

    /// The layout separator under cell `(col, row)`, if one can be dragged now: not while a
    /// modal or a menu owns input.
    #[must_use]
    pub fn separator_at(&self, col: u32, row: u32) -> Option<Separator> {
        if !self.input_free() {
            return None;
        }
        self.layout().separator_at(col, row)
    }

    /// What a press on `hit` starts dragging: a layout separator ([`ShellState::separator_at`],
    /// by arithmetic on the layout) or the playhead separator row (by the id of the cell).
    #[must_use]
    pub fn drag_at(&self, hit: &HitTestResult) -> Option<Separator> {
        self.separator_at(hit.col, hit.row).or_else(|| {
            (self.input_free() && hit.widget_id == Some(WidgetId::new(PLAYHEAD_ID)))
                .then_some(Separator::Playhead)
        })
    }

    /// Captures the exact content offset and pointer row when `hit` is the
    /// actionable thumb in the topmost SelectionPanel.
    #[must_use]
    pub fn selection_thumb_grab(&self, hit: &HitTestResult) -> Option<SelectionThumbGrab> {
        let id = hit.widget_id?;
        let bounds = GridRect::new(0, 0, self.cols, self.rows);
        self.selection
            .thumb_grab(id, hit.row, bounds, self.profile_selection_frame())
    }

    /// Whether the selection drag anchor still refers to the current list allocation.
    #[must_use]
    pub fn selection_thumb_grab_valid(&self, grab: SelectionThumbGrab) -> bool {
        self.selection.thumb_grab_valid(
            grab,
            GridRect::new(0, 0, self.cols, self.rows),
            self.profile_selection_frame(),
        )
    }

    /// Whether a captured drag's handle still exists under the current overlay state.
    #[must_use]
    pub fn captured_drag_available(&self, selection_thumb: bool) -> bool {
        if selection_thumb {
            self.selection.has_scrollbar(
                GridRect::new(0, 0, self.cols, self.rows),
                self.profile_selection_frame(),
            )
        } else {
            self.input_free()
        }
    }

    /// Moves a captured SelectionPanel scrollbar thumb to the pointer row.
    pub fn drag_selection_thumb(&mut self, row: u32, grab: SelectionThumbGrab) -> bool {
        let bounds = GridRect::new(0, 0, self.cols, self.rows);
        let changed = self
            .selection
            .drag_thumb(row, grab, bounds, self.profile_selection_frame());
        if changed {
            self.sync();
        }
        changed
    }

    /// Drags `separator` to the pointer cell `(col, row)` (any cell, even off the grid: the
    /// result is clamped). `Navigation` reads the column, `Split` and `Playhead` the row. Returns `true` if the layout or the playhead changed.
    pub fn drag_separator(&mut self, separator: Separator, col: u32, row: u32) -> bool {
        match separator {
            Separator::Navigation => self.set_nav_width(col.saturating_add(1)),
            Separator::Split => self.set_split(row),
            Separator::Playhead => self.drag_playhead(row),
        }
    }

    /// Moves the Features / Elements divider to `row` (the row of the shared border). The pane
    /// heights are clamped to three rows each. The proportion is remembered, so a later window
    /// resize keeps it. Returns `true` if the laid-out heights changed.
    fn set_split(&mut self, row: u32) -> bool {
        let layout = self.layout();
        let before = layout.features.height();
        let body = layout.viewport.height();
        let wanted = row.saturating_add(1).saturating_sub(layout.features.row());
        let features = features_rows(
            body,
            Some(Split {
                features: wanted,
                body,
            }),
        );
        self.widths.split = Some(Split { features, body });
        self.layout().features.height() != before
    }

    fn drag_playhead(&mut self, row: u32) -> bool {
        let first = self.layout().features.inset(1).row();
        let lines = self.tree_lines(Tree::Features);
        let index = self.tree_scroll[0].saturating_add(row.saturating_sub(first) as usize);
        let display = if row < first {
            self.projection
                .features
                .first()
                .map_or(GapDisplayLocation::EndOfTree, |item| {
                    GapDisplayLocation::BeforeNode(item.id)
                })
        } else {
            match lines.get(index) {
                Some(Line::Playhead) => return false,
                Some(Line::Node(shown)) if shown.index == 0 => self
                    .projection
                    .features
                    .first()
                    .map_or(GapDisplayLocation::EndOfTree, |item| {
                        GapDisplayLocation::BeforeNode(item.id)
                    }),
                Some(Line::Node(shown))
                    if self.inline_playhead_host()
                        == Some(self.nodes[Tree::Features.slot()][shown.index].target) =>
                {
                    return false;
                }
                Some(Line::Node(shown)) if shown.index > 0 => {
                    let Some(history_index) =
                        self.history_index(self.nodes[Tree::Features.slot()][shown.index].target)
                    else {
                        return false;
                    };
                    let item = &self.projection.features[history_index];
                    GapDisplayLocation::AfterSubtree(item.id)
                }
                _ => GapDisplayLocation::EndOfTree,
            }
        };
        let Ok(mut gap) = self.projection.placed_gap(display) else {
            return false;
        };
        if let HistoryGapPolicy::ExplicitGaps(allowed) = &self.gap_policy {
            let history_index = lines.get(index).and_then(|line| match line {
                Line::Node(shown) => {
                    self.history_index(self.nodes[Tree::Features.slot()][shown.index].target)
                }
                _ => None,
            });
            let reference_display = history_index
                .filter(|i| self.projection.features[*i].depth > 1)
                .and_then(|i| {
                    self.projection.features[..=i]
                        .iter()
                        .rev()
                        .find(|item| item.depth == 1)
                })
                .map_or(display, |item| GapDisplayLocation::AfterSubtree(item.id));
            let boundary = self
                .projection
                .display_boundary(reference_display)
                .unwrap_or(0);
            gap = allowed
                .iter()
                .min_by_key(|candidate| {
                    self.projection
                        .display_boundary(candidate.display)
                        .unwrap_or(0)
                        .abs_diff(boundary)
                })
                .copied()
                .unwrap_or(gap);
        }
        self.set_playhead(gap).unwrap_or(false)
    }

    /// The window control that owns widget `id`, if it is one of `[-] [□] [X]`.
    #[must_use]
    pub fn window_action(id: WidgetId) -> Option<WindowAction> {
        let rel = id.get().checked_sub(WINDOW_BASE)?;
        WINDOW_ACTIONS.get(usize::try_from(rel).ok()?).copied()
    }

    /// Resolves hover separately from clicks. Inert Action Box labels and cyclic values
    /// inherit a region id from their row without changing the grid's click routing.
    /// The fallback reads at most the fixed 31 Action Box cells, independent of scene size.
    #[must_use]
    pub fn hover_target(&self, hit: HitTestResult, grid: &crate::CellGrid) -> Option<WidgetId> {
        if let Some(id) = hit.widget_id {
            if let Some((tree, slot, _)) = tree_decode(id) {
                return (self.input_free() && self.tree_slots[tree.slot()].get(slot).is_some())
                    .then(|| tree_part_id(tree, slot, TreePart::Row));
            }
            return Some(id);
        }
        if !self.input_free() {
            return None;
        }
        let rect = self.layout().action_panel(1);
        let end = rect.col().saturating_add(rect.width().min(31));
        if hit.col <= rect.col() || hit.col >= end.saturating_sub(1) {
            return None;
        }
        let cell = grid.get(hit.col, hit.row)?;
        if matches!(
            cell.content,
            crate::CellContent::Glyph('│' | '╞' | '╡' | '╰' | '╴' | '┆' | '─')
        ) {
            return None;
        }
        (rect.col()..end).find_map(|col| {
            let id = grid.get(col, hit.row)?.widget_id?;
            self.panel().decode(id).map(|_| id)
        })
    }

    /// `false` if widget `id` can never be an ASCII hover target (M2.25): a tree row or part,
    /// the playhead, a window control, a MenuBar or CommandBar title, a popup surface. The
    /// Controller then needs no recomposition to know that nothing is hovered. `true` only
    /// means "maybe": the composer decides the eligibility.
    #[must_use]
    pub fn may_hover(id: WidgetId) -> bool {
        let bar_chrome = [BAR_IDS, COMMAND_IDS].into_iter().any(|ids| {
            matches!(
                ids.decode(id),
                Some(MenuTarget::Title(_) | MenuTarget::Popup)
            )
        });
        !(id.get() == PLAYHEAD_ID
            || id.get() == PROGRESS_ID
            || Self::window_action(id).is_some()
            || bar_chrome)
    }

    /// `true` if the pointer at cell `(col, row)` of `grid` is on unused MenuBar space, where a
    /// press drags the window: the MenuBar row, inside its frame, on a cell that carries no
    /// widget and no shared border, and no menu popup or modal is open.
    #[must_use]
    pub fn window_drag_at(&self, col: u32, row: u32, grid: &crate::CellGrid) -> bool {
        if !self.input_free() {
            return false;
        }
        let bar = self.layout().menu_bar();
        let inside = row == bar.row() && col >= bar.col() && u64::from(col) < bar.right();
        inside
            && grid
                .get(col, row)
                .is_some_and(|c| c.widget_id.is_none() && c.shared_border.is_none())
    }

    /// Focus and press state.
    #[must_use]
    pub const fn interaction(&self) -> &InteractionState {
        &self.interaction
    }

    /// The Extrude panel's mock state.
    #[must_use]
    pub const fn extrude(&self) -> &ExtrudeUi {
        &self.extrude
    }

    /// The Revolve panel's mock state.
    #[must_use]
    pub const fn revolve(&self) -> &RevolveUi {
        &self.revolve
    }

    /// The Loft panel's mock state.
    #[must_use]
    pub const fn loft(&self) -> &LoftUi {
        &self.loft
    }

    /// The Sweep panel's mock state.
    #[must_use]
    pub const fn sweep(&self) -> &SweepUi {
        &self.sweep
    }

    /// The Boolean panel's mock state.
    #[must_use]
    pub const fn boolean(&self) -> &BooleanUi {
        &self.boolean
    }

    /// The operation whose Action Box the ActionPanel shows.
    #[must_use]
    pub const fn operation(&self) -> ActionOperation {
        self.operation
    }

    /// Explicit numeric contracts for every field in the current operation, including hidden branches.
    #[must_use]
    pub fn numeric_descriptors(&self) -> Vec<crate::NumericFieldDescriptor> {
        self.panel().numeric_descriptors()
    }

    /// Identity installed by the current command's accepted numeric projection.
    pub fn numeric_session(
        &self,
    ) -> Option<(
        TreeDocumentId,
        crate::CommandInputSessionId,
        ActionOperation,
    )> {
        self.numeric_identity
    }
    /// Whether unaccepted numeric text currently owns keyboard input.
    #[must_use]
    pub fn numeric_editing(&self) -> bool {
        self.editing_field().is_some()
    }
    /// Controller-provided published origin for an application action.
    pub fn set_action_origin(&mut self, origin: crate::ProjectionGeneration) {
        self.properties_origin = Some(origin);
    }
    /// Semantic focused operation; opening a menu does not replace this context.
    #[must_use]
    pub fn focused_features_operation(&self) -> Option<TreeTarget> {
        self.tree_focus.filter(|target| {
            matches!(target.kind, TreeTargetKind::Operation(_))
                && self.nodes[Tree::Features.slot()]
                    .iter()
                    .any(|node| node.target == *target)
        })
    }
    #[must_use]
    pub fn properties_open(&self) -> Option<crate::PropertiesOpenRequest> {
        self.properties_request
    }
    pub fn take_properties_open(&mut self) -> Option<crate::PropertiesOpenRequest> {
        self.properties_request.take()
    }
    /// Keeps one-shot action ids consumed even if a candidate publication rolls back.
    pub fn retain_properties_counter(&mut self, id: crate::PropertiesRequestId) {
        self.next_properties_id = self.next_properties_id.max(id.0);
    }
    /// Hydrates an existing operation after the app has validated its certified request.
    pub fn open_extrude_edit(&mut self, values: crate::ExtrudeValues) {
        self.show_operation(ActionOperation::Extrude);
        self.command_started = false;
        self.extrude.values = values;
        self.extrude.status = crate::ExtrudeStatus::Editing;
        self.numeric[ActionOperation::Extrude.slot()].reset();
        self.sync();
    }
    /// Ordinary contextual diagnostic, drawn in the existing free-text status segment.
    pub fn set_contextual_status(&mut self, message: Option<String>) -> bool {
        if self.formula_diagnostic == message {
            return false;
        }
        self.formula_diagnostic = message;
        true
    }
    /// Tests active eligibility from semantic branch state, excluding closed/other panels.
    pub fn numeric_field_visible(&self, field: crate::NumericFieldKey) -> bool {
        if field.operation != self.operation || !self.panel().showing() {
            return false;
        }
        use crate::{ExtrudeOffsetMode as Mode, ExtrudeTermination as Term};
        let active = match self.operation {
            ActionOperation::Extrude => {
                let v = &self.extrude.values;
                match field.key {
                    4 => v.termination == Term::Blind,
                    29 => v.termination == Term::UpTo && v.offset,
                    11 => {
                        matches!(v.termination, Term::Blind | Term::ThroughAll)
                            && v.starting_offset
                            && v.start_mode == Mode::Blind
                    }
                    15 => v.termination == Term::Blind && v.end_offset && v.end_mode == Mode::Blind,
                    20 => !v.symmetric && v.second_side && v.second_mode == Mode::Blind,
                    24 => {
                        !v.symmetric
                            && v.second_side
                            && v.second_mode == Mode::UpTo
                            && v.nested_end
                            && v.nested_mode == Mode::Blind
                    }
                    _ => false,
                }
            }
            ActionOperation::Revolve => {
                let v = &self.revolve.values;
                !v.full
                    && match field.key {
                        6 => v.mode == Mode::Blind,
                        10 => v.start_offset && v.start_mode == Mode::Blind,
                        14 => v.mode == Mode::UpTo && v.end_offset && v.end_mode == Mode::Blind,
                        19 => !v.symmetric && v.second_side && v.second_mode == Mode::Blind,
                        24 => {
                            !v.symmetric
                                && v.second_side
                                && v.second_mode == Mode::UpTo
                                && v.nested_end
                                && v.nested_mode == Mode::Blind
                        }
                        _ => false,
                    }
            }
            ActionOperation::Loft => {
                let v = &self.loft.values;
                match field.key {
                    5 => v.start_type && v.start_kind != crate::LoftEndType::Match,
                    9 => v.end_type && v.end_kind != crate::LoftEndType::Match,
                    _ => false,
                }
            }
            ActionOperation::Sweep => {
                let v = &self.sweep.values;
                match field.key {
                    7 => v.keep_orientation && v.twist,
                    9 => v.scale,
                    _ => false,
                }
            }
            ActionOperation::Boolean => {
                field.key == 6
                    && self.boolean.values.operation == crate::BooleanOperation::Cut
                    && self.boolean.values.offset
            }
        };
        debug_assert_eq!(
            active,
            self.panel().numeric_keys().contains(&field.key) && self.panel().shows(field.key)
        );
        active
    }
    /// Stamps subsequent candidates with the controller's pre-input published generation.
    pub fn set_numeric_origin(&mut self, origin: ProjectionGeneration) {
        self.numeric_origin = origin;
    }
    /// Whether this exact OK attempt still awaits acknowledgement, even after outbound drain.
    pub fn numeric_confirm_pending(&self, request: crate::NumericConfirmRequest) -> bool {
        self.numeric_confirm == Some(request)
    }
    /// Whether this exact Formula opening still awaits its accepted projection.
    pub fn numeric_formula_pending(&self, request: crate::NumericFormulaOpenRequest) -> bool {
        self.numeric_formula_open == Some(request)
    }
    /// Whether a terminal acknowledgement or deferred Formula opening currently gates input.
    pub fn numeric_delivery_pending(&self) -> bool {
        self.numeric_confirm.is_some()
            || self.numeric_formula_open.is_some()
            || self.formula_apply.is_some()
    }
    fn next_numeric_sequence(&mut self) -> Result<u64, crate::UiError> {
        self.numeric_sequence = self
            .numeric_sequence
            .checked_add(1)
            .ok_or(crate::UiError::InteractionIdentityExhausted)?;
        Ok(self.numeric_sequence)
    }
    fn queue_numeric_edits(&mut self) {
        let operations = [
            ActionOperation::Extrude,
            ActionOperation::Revolve,
            ActionOperation::Loft,
            ActionOperation::Sweep,
            ActionOperation::Boolean,
        ];
        for operation in operations {
            for (id, edit) in self.numeric[operation.slot()].take_edits() {
                let Some((document, session, owner)) = self.numeric_identity else {
                    continue;
                };
                if owner != operation {
                    continue;
                }
                let Some(key) = id
                    .get()
                    .checked_sub(ACTION_BASE)
                    .map(|n| n / crate::action_box::KEY_SLOTS)
                else {
                    continue;
                };
                match self.next_numeric_sequence() {
                    Ok(sequence) => {
                        self.numeric_last_edits
                            .insert(crate::NumericFieldKey { operation, key }, sequence);
                        self.numeric_requests.push(crate::NumericRequest {
                            destination: crate::NumericDestination {
                                document,
                                session,
                                field: crate::NumericFieldKey { operation, key },
                            },
                            sequence,
                            origin: self.numeric_origin,
                            edit,
                        });
                    }
                    Err(error) => self.numeric_error = Some(error),
                }
            }
        }
    }
    /// Drains the last checked interaction-identity allocation failure once.
    pub fn take_numeric_error(&mut self) -> Option<crate::UiError> {
        self.numeric_error.take()
    }
    /// Qualifies queued editor candidates with the current identity and sequence, then drains.
    pub fn take_numeric_requests(&mut self) -> Vec<crate::NumericRequest> {
        self.queue_numeric_edits();
        std::mem::take(&mut self.numeric_requests)
    }
    /// Drains outbound OK once, retaining the exact pending record for acknowledgement.
    pub fn take_numeric_confirm(&mut self) -> Option<crate::NumericConfirmRequest> {
        if std::mem::take(&mut self.numeric_confirm_outbound) {
            self.numeric_confirm
        } else {
            None
        }
    }
    /// Drains outbound Formula opening once, retaining its pending delivery record.
    pub fn take_numeric_formula_open(&mut self) -> Option<crate::NumericFormulaOpenRequest> {
        if std::mem::take(&mut self.numeric_formula_outbound) {
            self.numeric_formula_open
        } else {
            None
        }
    }
    /// Drains one explicit Formula application, retaining its exact pending identity.
    pub fn take_formula_apply(&mut self) -> Option<crate::FormulaApplyRequest> {
        if std::mem::take(&mut self.formula_apply_outbound) {
            self.formula_apply.clone()
        } else {
            None
        }
    }
    /// Whether this exact modal application still owns its destination.
    #[must_use]
    pub fn formula_apply_pending(&self, request: &crate::FormulaApplyRequest) -> bool {
        self.formula_apply.as_ref() == Some(request)
            && self.formula_modal_session == Some(request.modal_session)
    }
    /// Publishes a validated Formula result; failure retains editable source and diagnostic.
    ///
    /// # Errors
    /// Rejects an inconsistent accepted field projection.
    pub fn finish_formula_apply(
        &mut self,
        request: &crate::FormulaApplyRequest,
        value: crate::NumericProjection,
        diagnostic: Option<String>,
    ) -> Result<bool, crate::UiError> {
        if !self.formula_apply_pending(request) {
            return Ok(false);
        }
        if value.accepted_sequence != request.attempt.0 {
            return Err(crate::UiError::InvalidNumericProjection);
        }
        self.set_numeric_projection(value)?;
        self.formula_apply = None;
        self.formula_apply_outbound = false;
        self.formula_diagnostic = diagnostic;
        self.formula
            .finish_application(self.formula_diagnostic.is_none());
        if self.formula_diagnostic.is_none() {
            self.formula_target = None;
            self.formula_modal_session = None;
        }
        self.sync();
        Ok(true)
    }
    /// Installs complete accepted views without emitting edits or replacing same-session raw text.
    /// A new session drops the old raw edit and resets its interaction identities.
    ///
    /// Returns `false` for a stale session/sequence. Invalid namespaces, operations, or field
    /// inventories return [`crate::UiError::InvalidNumericProjection`].
    pub fn set_numeric_projection(
        &mut self,
        value: crate::NumericProjection,
    ) -> Result<bool, crate::UiError> {
        if value.document != self.projection.document || value.operation != self.operation {
            return Err(crate::UiError::InvalidNumericProjection);
        }
        if let Some((document, session, _)) = self.numeric_identity
            && document == value.document
            && value.session.0 < session.0
        {
            return Ok(false);
        }
        if self.numeric_identity == Some((value.document, value.session, value.operation))
            && value.accepted_sequence < self.numeric_accepted_sequence
        {
            return Ok(false);
        }
        if self
            .numeric_retired_session
            .is_some_and(|(document, session)| {
                document == value.document && value.session.0 <= session.0
            })
        {
            return Ok(false);
        }
        let descriptors = crate::numeric::descriptors(value.operation);
        let keys: BTreeSet<_> = value.fields.iter().map(|v| v.field).collect();
        if keys.len() != value.fields.len() || keys != descriptors.iter().map(|d| d.field).collect()
        {
            return Err(crate::UiError::InvalidNumericProjection);
        }
        let identity = Some((value.document, value.session, value.operation));
        if self.numeric_identity != identity {
            self.numeric_identity = identity;
            self.numeric_sequence = value.accepted_sequence;
            self.numeric_attempt = 0;
            self.numeric_confirm = None;
            self.numeric_confirm_outbound = false;
            self.numeric_formula_open = None;
            self.numeric_formula_outbound = false;
            self.formula = FormulaEditor::new(FORMULA_BASE);
            self.formula_target = None;
            self.formula_apply = None;
            self.formula_apply_outbound = false;
            self.formula_modal_session = None;
            self.formula_diagnostic = None;
            self.numeric_requests.clear();
            self.numeric_last_edits.clear();
            self.numeric[value.operation.slot()].discard();
        }
        self.numeric_sequence = self.numeric_sequence.max(value.accepted_sequence);
        self.numeric_accepted_sequence = value.accepted_sequence;
        self.numeric_last_edits
            .retain(|_, sequence| *sequence > value.accepted_sequence);
        for view in &value.fields {
            let Some(descriptor) = descriptors.iter().find(|d| d.field == view.field) else {
                return Err(crate::UiError::InvalidNumericProjection);
            };
            let id = WidgetId::new(ACTION_BASE + view.field.key * crate::action_box::KEY_SLOTS);
            self.numeric[value.operation.slot()].install(id, *descriptor, view);
        }
        Ok(true)
    }
    /// Ends only the matching outstanding edit, ignoring stale destinations and sequences.
    /// The application supplies the accepted projection separately to restore presentation.
    pub fn reject_numeric_edit(
        &mut self,
        destination: crate::NumericDestination,
        sequence: u64,
    ) -> Result<bool, crate::UiError> {
        if self.numeric_identity
            != Some((
                destination.document,
                destination.session,
                destination.field.operation,
            ))
            || self.numeric_last_edits.get(&destination.field) != Some(&sequence)
        {
            return Ok(false);
        }
        self.numeric_last_edits.remove(&destination.field);
        let id = WidgetId::new(ACTION_BASE + destination.field.key * crate::action_box::KEY_SLOTS);
        if self.editing_field() == Some(id) {
            self.numeric[destination.field.operation.slot()].discard();
        }
        Ok(true)
    }
    /// Acknowledges one exact resolved OK attempt; this method performs no model commit.
    /// Acceptance closes the panel once; rejection releases the pending attempt and keeps it
    /// Editing. Mismatched or duplicate acknowledgements return `false` without changing it.
    pub fn finish_numeric_confirm(
        &mut self,
        request: crate::NumericConfirmRequest,
        accepted: bool,
    ) -> Result<bool, crate::UiError> {
        if self.numeric_confirm != Some(request)
            || self.numeric_identity != Some((request.document, request.session, request.operation))
        {
            return Ok(false);
        }
        self.numeric_confirm = None;
        self.numeric_confirm_outbound = false;
        if accepted {
            self.panel_mut().activate_id(WidgetId::new(
                ACTION_BASE + K_ACTIONS * crate::action_box::KEY_SLOTS,
            ));
            self.numeric[request.operation.slot()].confirm();
            self.interaction.set_focused(None);
        }
        self.sync();
        Ok(true)
    }
    /// Installs the resulting accepted views and opens Formula once from that exact source.
    ///
    /// Ignores stale/duplicate opening identities. A mismatched accepted projection returns
    /// [`crate::UiError::InvalidNumericProjection`].
    pub fn open_numeric_formula(
        &mut self,
        request: crate::NumericFormulaOpenRequest,
        value: crate::NumericProjection,
    ) -> Result<bool, crate::UiError> {
        if self.numeric_formula_open != Some(request)
            || self.numeric_identity
                != Some((
                    request.destination.document,
                    request.destination.session,
                    request.destination.field.operation,
                ))
        {
            return Ok(false);
        }
        if value.accepted_sequence != request.sequence
            || value.accepted_sequence < self.numeric_accepted_sequence
            || value.document != request.destination.document
            || value.session != request.destination.session
            || value.operation != request.destination.field.operation
        {
            return Err(crate::UiError::InvalidNumericProjection);
        }
        let source = value
            .fields
            .iter()
            .find(|v| v.field == request.destination.field)
            .ok_or(crate::UiError::InvalidNumericProjection)?
            .formula
            .clone();
        self.set_numeric_projection(value)?;
        let field = WidgetId::new(
            ACTION_BASE + request.destination.field.key * crate::action_box::KEY_SLOTS,
        );
        self.formula_target = Some((field, Some(request.destination)));
        self.numeric_formula_open = None;
        self.numeric_formula_outbound = false;
        self.formula.open(&mut self.interaction, &source);
        self.formula.defer_application();
        self.formula_modal_session = Some(request.sequence);
        self.formula_diagnostic = None;
        self.sync();
        Ok(true)
    }
    /// Resolves the raw field by the ordinary acceptance rule, without confirming the command.
    /// Candidates still require application validation; returns whether an edit was active.
    pub fn numeric_focus_lost(&mut self) -> bool {
        let editing = self.editing_field().is_some();
        self.numeric[self.operation.slot()].commit();
        editing
    }
    /// Drops numeric destinations, pending deliveries, Formula state, and presentation caches
    /// when the document/session owner tears down its interaction namespace.
    pub fn invalidate_numeric_session(&mut self) {
        self.formula = FormulaEditor::new(FORMULA_BASE);
        self.numeric_identity = None;
        self.numeric_retired_session = None;
        self.numeric_confirm = None;
        self.numeric_formula_open = None;
        self.numeric_requests.clear();
        self.numeric_last_edits.clear();
        self.formula_target = None;
        self.formula_apply = None;
        self.formula_apply_outbound = false;
        self.formula_modal_session = None;
        self.formula_diagnostic = None;
        for fields in &mut self.numeric {
            fields.reset();
        }
        self.sync();
    }

    /// The shown Action Box, whichever operation it belongs to.
    fn panel(&self) -> &dyn ActionBoxUi {
        match self.operation {
            ActionOperation::Extrude => &self.extrude,
            ActionOperation::Revolve => &self.revolve,
            ActionOperation::Loft => &self.loft,
            ActionOperation::Sweep => &self.sweep,
            ActionOperation::Boolean => &self.boolean,
        }
    }

    fn panel_mut(&mut self) -> &mut dyn ActionBoxUi {
        match self.operation {
            ActionOperation::Extrude => &mut self.extrude,
            ActionOperation::Revolve => &mut self.revolve,
            ActionOperation::Loft => &mut self.loft,
            ActionOperation::Sweep => &mut self.sweep,
            ActionOperation::Boolean => &mut self.boolean,
        }
    }

    /// The SelectionPanel.
    #[must_use]
    pub const fn selection(&self) -> &SelectionPanel {
        &self.selection
    }

    /// The numeric fields of the shown Action Box.
    pub(crate) fn fields(&self) -> &NumericFields {
        &self.numeric[self.operation.slot()]
    }

    /// The numeric field being edited, if any.
    #[must_use]
    pub fn editing_field(&self) -> Option<WidgetId> {
        self.fields().editing()
    }

    /// The FormulaEditor.
    #[must_use]
    pub const fn formula(&self) -> &FormulaEditor {
        &self.formula
    }

    /// The Rename modal.
    #[must_use]
    pub const fn rename(&self) -> &RenameModal {
        &self.rename
    }

    /// `true` if `id` is the name of a user-named Feature or Element row: the only target of a
    /// double-click Rename (`README_UI.md` §23). Markers, checkboxes, category headers and the
    /// playhead are not.
    #[must_use]
    pub fn is_rename_target(&self, id: WidgetId) -> bool {
        self.decode_tree(id).is_some_and(|(tree, index, part)| {
            part == TreePart::Name && self.rename_eligible(self.nodes[tree.slot()][index].target)
        })
    }
    fn rename_eligible(&self, target: TreeTarget) -> bool {
        match target.kind {
            TreeTargetKind::Operation(_) | TreeTargetKind::Folder(_) => true,
            TreeTargetKind::Entity(id) => self
                .projection
                .elements
                .iter()
                .any(|item| item.id == id && item.group == ElementGroup::Bodies),
            _ => false,
        }
    }
    pub fn begin_rename(&mut self, id: WidgetId) -> bool {
        let Some((tree, index, _)) = self.decode_tree(id).filter(|_| self.is_rename_target(id))
        else {
            return false;
        };
        if !self.input_free() {
            return false;
        }
        let node = &self.nodes[tree.slot()][index];
        let target = node.target;
        let name = node.label.clone();
        self.rename_target = Some((target, name.clone()));
        self.tree_focus = Some(target);
        self.interaction
            .set_focused(Some(self.node_id(tree, index, TreePart::Row)));
        self.rename.open(&mut self.interaction, &name);
        self.sync();
        true
    }

    /// Whether this window uses the accepted semantic palette.
    #[must_use]
    pub const fn color_enabled(&self) -> bool {
        self.view.color_enabled
    }

    /// The UI theme chosen in the View menu. Dark for every new shell.
    #[must_use]
    pub const fn theme(&self) -> ThemeChoice {
        self.view.theme
    }

    /// Selected View > Shading variant, in the documented menu order.
    #[must_use]
    pub const fn shading(&self) -> usize {
        self.view.shading
    }

    /// `true` for Perspective and `false` for Orthographic projection.
    #[must_use]
    pub const fn perspective(&self) -> bool {
        self.view.projection == 0
    }

    /// Whether one of the View visibility toggles is enabled.
    #[must_use]
    pub fn view_panel_visible(&self, index: usize) -> bool {
        self.view.checks.get(index).copied().unwrap_or(false)
    }

    /// Consumes a requested camera reset from the View menu.
    pub fn take_camera_reset(&mut self) -> bool {
        std::mem::take(&mut self.camera_reset_request)
    }

    /// Consumes a requested closest principal "Normal to" view from the View menu.
    pub fn take_normal_view(&mut self) -> bool {
        std::mem::take(&mut self.normal_view_request)
    }

    /// The rendering multiplier paired with the selected View-menu label. The shell holds no
    /// `k`, monitor or window-fit logic.
    #[must_use]
    pub fn scale_factor_percent(&self) -> u32 {
        self.view.scale_factor_percent()
    }

    /// `true` once after the Fullscreen item was activated; the application owns the window
    /// and decides (toggle, refusal).
    pub fn take_fullscreen_request(&mut self) -> bool {
        std::mem::take(&mut self.fullscreen_request)
    }

    /// Inject runtime status (or deterministic snapshot values) without changing layout.
    pub fn set_status(&mut self, fps: f64, progress: u32) -> bool {
        let fps = if fps.is_finite() && fps >= 0.0 {
            fps.round().min(999.0) as u32
        } else {
            0
        };
        let progress = progress.min(100);
        let changed = self.fps != fps || self.progress != progress;
        self.fps = fps;
        self.progress = progress;
        changed
    }

    /// A progress press is eligible only without any menu/modal above the shell.
    #[must_use]
    pub fn progress_press(&self, id: WidgetId) -> bool {
        id.get() == PROGRESS_ID && self.input_free()
    }

    /// Index of the active mock workbench, shown in the StatusBar.
    #[must_use]
    pub const fn workbench(&self) -> usize {
        self.workbench
    }

    /// `true` while a modal overlay owns input.
    #[must_use]
    pub fn modal_open(&self) -> bool {
        self.selection.is_open() || self.formula.is_open() || self.rename.is_open()
    }

    /// Applies a pointer activation at `hit`, the hit-test result of the composed grid.
    /// Returns `true` if the click was acted on (recompose the grid); a click on nothing, or
    /// outside an open modal, returns `false`.
    pub fn click(&mut self, hit: Option<HitTestResult>) -> bool {
        if self.numeric_delivery_pending() {
            return false;
        }
        let bounds = GridRect::new(0, 0, self.cols, self.rows);
        let id = hit.and_then(|h| h.widget_id);
        if self.editing_field().is_some_and(|f| Some(f) != id) {
            self.numeric[self.operation.slot()].commit();
        }
        let changed = if self.selection.is_open() {
            id.is_some_and(|id| {
                if self.selection.is_profile_picker() {
                    self.selection
                        .activate_in_action_box(id, self.profile_selection_frame())
                } else {
                    self.selection.activate(id, bounds)
                }
            })
        } else if self.formula.is_open() {
            id.is_some_and(|id| self.formula.activate(id))
        } else if self.rename.is_open() {
            id.is_some_and(|id| self.rename.activate(id))
        } else if let Some(handled) = self.click_menu(id, hit) {
            handled
        } else {
            id.is_some_and(|id| {
                self.toggle_numeric_sign(id, hit)
                    .unwrap_or_else(|| self.activate_widget(id))
            })
        };
        self.sync();
        changed
    }

    /// Applies a translated key. Returns `true` if the key was acted on (recompose the
    /// grid). Keys with an unusable character (control characters) are ignored.
    pub fn key(&mut self, key: ShellKey) -> bool {
        if self.numeric_delivery_pending() {
            return false;
        }
        if !key.is_valid() {
            return false;
        }
        let changed = if self.modal_open() {
            self.modal_key(key)
        } else if let Some(handled) = self.numeric_key(key) {
            handled
        } else {
            self.menu_or_base_key(key)
        };
        self.sync();
        changed
    }

    /// Routes wheel or trackpad scroll steps to the modal selection list. Positive steps
    /// move the list upwards, matching native wheel conventions.
    pub fn scroll(&mut self, steps: i32) -> bool {
        let changed = self.selection.scroll_wheel(
            steps,
            GridRect::new(0, 0, self.cols, self.rows),
            self.profile_selection_frame(),
        );
        if changed {
            self.sync();
        }
        changed
    }

    /// Brings overlays and focus in line with the state: a modal closes the menus; popups
    /// and modals are opened or dismissed; focus follows the active owner and is restored
    /// when a modal closes. Called after every input.
    fn sync(&mut self) {
        self.update_tree_content_max();
        let restoring_rename = !self.rename.is_open()
            && (self.rename_target.is_some()
                || self
                    .interaction
                    .focused
                    .is_some_and(|id| (RENAME_BASE..RENAME_BASE + 4).contains(&id.get())));
        self.rebuild_tree_slots();
        self.finish_rename();
        self.finish_formula();
        let layout = self.layout();
        let bounds = layout.grid;
        if self.modal_open() {
            self.numeric[self.operation.slot()].commit();
            self.menu_bar.close();
            self.command_bar.close();
            self.typed.clear();
        }
        if !self.view_panel_visible(1) {
            self.command_bar.close();
            self.typed.clear();
        }
        let profile_frame = self.profile_selection_frame();
        self.selection.reconcile(bounds, profile_frame);
        self.selection
            .sync_overlay_with_profile_frame(&mut self.overlays, bounds, profile_frame);
        self.formula.sync_overlay(&mut self.overlays, bounds);
        self.rename.sync_overlay(&mut self.overlays, bounds);
        let bar = layout.menu_bar();
        with_bar_menus(&self.view, |menus| {
            self.menu_bar
                .sync_overlay(&mut self.overlays, menus, BAR_IDS, bar, bounds);
        });
        self.command_bar.sync_centered_overlay(
            &mut self.overlays,
            &COMMAND_MENUS,
            COMMAND_IDS,
            bounds,
            data::COMMAND_POPUP_WIDTH,
        );
        self.menu_bar.sync_focus(&mut self.interaction, BAR_IDS);
        self.command_bar
            .sync_focus(&mut self.interaction, COMMAND_IDS);
        self.selection.sync_focus(&mut self.interaction);
        self.formula.sync_focus(&mut self.interaction);
        self.rename.sync_focus(&mut self.interaction);
        if !self.modal_open()
            && self
                .interaction
                .focused
                .is_some_and(|id| in_action_panel(id) && self.panel().live_control(id).is_none())
        {
            self.interaction.set_focused(None);
        }
        if restoring_rename {
            if self.tree_focus.is_none() {
                self.interaction.set_focused(None);
            } else {
                self.rebuild_tree_slots();
            }
        }
    }

    /// Applies the name of a Rename that was confirmed, once the modal has closed; a cancelled
    /// one changes nothing.
    fn finish_rename(&mut self) {
        if self.rename.is_open() {
            return;
        }
        if let Some((target, source_name)) = self.rename_target.take()
            && self.rename.status() == crate::ModalStatus::Confirmed
        {
            self.pending_tree_edit = Some((
                target,
                TreeIntentKind::Rename {
                    source_name,
                    name: self.rename.text().to_owned(),
                },
            ));
        }
    }

    /// Keeps the formula text of a FormulaEditor that was confirmed, for the numeric field it
    /// was opened from; a cancelled one changes nothing.
    fn finish_formula(&mut self) {
        if self.formula.application_pending()
            && self.formula_apply.is_none()
            && let (Some((_, Some(destination))), Some(modal_session)) =
                (self.formula_target, self.formula_modal_session)
        {
            let through_sequence = self.numeric_sequence;
            match self.next_numeric_sequence() {
                Ok(attempt) => {
                    self.formula_apply = Some(crate::FormulaApplyRequest {
                        destination,
                        modal_session,
                        attempt: crate::FormulaAttemptId(attempt),
                        through_sequence,
                        origin: self.numeric_origin,
                        source: self.formula.draft(),
                        source_origins: self.formula.source_origins().to_vec(),
                    });
                    self.formula_apply_outbound = true;
                }
                Err(error) => {
                    self.numeric_error = Some(error);
                    self.formula.finish_application(false);
                }
            }
        }
        if self.formula.is_open() {
            return;
        }
        let Some((field, destination)) = self.formula_target.take() else {
            return;
        };
        if destination.is_some_and(|d| {
            self.numeric_identity != Some((d.document, d.session, d.field.operation))
        }) {
            return;
        }
        if self.formula.status() != crate::ModalStatus::Confirmed {
            return;
        }
        if let Some(mock) = self.numeric_descriptor(field) {
            self.numeric[self.operation.slot()].set_formula(field, mock, self.formula.text());
        }
    }

    fn numeric_descriptor(&self, id: WidgetId) -> Option<crate::NumericFieldDescriptor> {
        let (key, _) = self.panel().decode(id)?;
        self.panel()
            .shows(key)
            .then(|| self.panel().numeric_descriptor(key))
            .flatten()
    }

    /// Source Action Box footprint; centered modals have separate overlay rectangles.
    pub fn action_panel_rect(&self) -> Option<GridRect> {
        let panel = self.panel().compose_box()?;
        Some(self.layout().action_panel(panel.height()))
    }

    /// Frame occupied by the Profile Selection box while it replaces an Action Box.
    fn profile_selection_frame(&self) -> GridRect {
        self.layout().action_panel(self.selection.profile_height())
    }

    /// Returns `Some` only when this click is on the `+` / `-` marker of a directional
    /// numeric field.  The rest of the field retains normal edit behaviour.
    fn toggle_numeric_sign(&mut self, id: WidgetId, hit: Option<HitTestResult>) -> Option<bool> {
        let hit = hit?;
        let frame = self.action_panel_rect()?;
        let col = hit.col.checked_sub(frame.col())?;
        let row = hit.row.checked_sub(frame.row())?;
        let panel = self.panel().compose_box()?;
        if !panel.numeric_sign_at(id, col, row) {
            return None;
        }
        let mock = self.numeric_descriptor(id)?;
        Some(self.numeric[self.operation.slot()].toggle_sign(id, mock))
    }

    /// Routes a key to the numeric field being edited. `None` if no field is edited, or for
    /// the keys that open a menu (the edit is kept first) and then follow their normal path.
    fn numeric_key(&mut self, key: ShellKey) -> Option<bool> {
        self.editing_field()?;
        let slot = self.operation.slot();
        let fields = &mut self.numeric[slot];
        let handled = match key {
            ShellKey::Char(c) => fields.typed(c),
            ShellKey::Backspace => fields.backspace(),
            ShellKey::Left => fields.edit_key(EditKey::Left),
            ShellKey::Right => fields.edit_key(EditKey::Right),
            ShellKey::Edit(k) => fields.edit_key(k),
            ShellKey::Up | ShellKey::Down => false,
            ShellKey::Enter => {
                fields.commit();
                true
            }
            ShellKey::Escape => {
                fields.discard();
                true
            }
            ShellKey::Tab | ShellKey::BackTab => {
                fields.commit();
                self.step_ring(key == ShellKey::Tab);
                return Some(true);
            }
            ShellKey::MenuBar | ShellKey::CommandBar | ShellKey::Alt(_) => {
                fields.commit();
                return None;
            }
        };
        Some(handled)
    }

    // ---- pointer ----

    /// Routes a click to the menu that owns the hit id; any other click while a menu is
    /// active closes it. `None` if no menu is involved.
    fn click_menu(&mut self, id: Option<WidgetId>, hit: Option<HitTestResult>) -> Option<bool> {
        let on_bar = id.is_some_and(|i| BAR_IDS.decode(i).is_some());
        let on_command = id.is_some_and(|i| COMMAND_IDS.decode(i).is_some());
        if on_bar {
            self.command_bar.close();
            self.typed.clear();
            let result = with_bar_menus(&self.view, |menus| {
                self.menu_bar.handle_click(menus, BAR_IDS, hit)
            });
            Some(self.apply_menu(result))
        } else if on_command {
            self.menu_bar.close();
            self.typed.clear();
            let result = self
                .command_bar
                .handle_click(&COMMAND_MENUS, COMMAND_IDS, hit);
            Some(self.apply_menu(result))
        } else if self.menu_bar.is_active() || self.command_bar.is_active() {
            self.menu_bar.close();
            self.command_bar.close();
            self.typed.clear();
            Some(true)
        } else {
            None
        }
    }

    /// Activates the base-shell control `id`: the single path of pointer clicks and of
    /// Enter / Space on the focused widget. Returns `true` if `id` belongs to the shell.
    fn activate_widget(&mut self, id: WidgetId) -> bool {
        if let Some((tree, index, part)) = self.decode_tree(id) {
            self.activate_tree(tree, index, part);
            return true;
        }
        if !in_action_panel(id) {
            return false;
        }
        // The focus is set first so a modal opened below remembers this control.
        self.tree_focus = None;
        self.interaction.set_focused(Some(id));
        let slot = self.operation.slot();
        if self.panel().is_formula_button(id) {
            // The `☰` is slot 1 of the field it belongs to (slot 0).
            let field = WidgetId::new(id.get() - 1);
            if let Some((document, session, operation)) = self.numeric_identity {
                self.queue_numeric_edits();
                let through_sequence = self.numeric_sequence;
                match self.next_numeric_sequence() {
                    Ok(sequence) => {
                        self.numeric_formula_open = Some(crate::NumericFormulaOpenRequest {
                            destination: crate::NumericDestination {
                                document,
                                session,
                                field: crate::NumericFieldKey {
                                    operation,
                                    key: (field.get() - ACTION_BASE) / crate::action_box::KEY_SLOTS,
                                },
                            },
                            sequence,
                            through_sequence,
                            origin: self.numeric_origin,
                        });
                        self.numeric_formula_outbound = true;
                    }
                    Err(error) => self.numeric_error = Some(error),
                }
            } else if self.reference_fixture {
                let text = self.numeric[slot].formula(field).to_owned();
                self.formula_target = Some((field, None));
                self.formula.open(&mut self.interaction, &text);
            }
        } else if self.panel().is_numeric_field(id) {
            if let Some(mock) = self.numeric_descriptor(id) {
                self.numeric_last_edits.remove(&mock.field);
                self.numeric[slot].begin(id, mock);
            }
        } else if self.panel().is_selection_field(id) {
            if !self.reference_fixture {
                if let Some((key, _)) = self.panel().decode(id)
                    && let Some(descriptor) = self.panel().selection_descriptor(key)
                {
                    self.selection_request = Some(crate::SelectionUiRequest::OpenField(descriptor));
                }
            } else if self.panel().is_profile_selection_field(id) {
                self.selection.open_profiles(&mut self.interaction);
            } else {
                self.selection.open(&mut self.interaction);
            }
        } else {
            let button = self
                .panel()
                .live_control(id)
                .filter(|&(key, slot)| key == K_ACTIONS && slot < 2)
                .map(|(_, slot)| slot);
            if button == Some(0) && self.numeric_identity.is_some() {
                self.queue_numeric_edits();
                if self.numeric_confirm.is_none() {
                    match self.numeric_attempt.checked_add(1) {
                        Some(attempt) => {
                            self.numeric_attempt = attempt;
                            if let Some((document, session, operation)) = self.numeric_identity {
                                self.numeric_confirm = Some(crate::NumericConfirmRequest {
                                    document,
                                    session,
                                    operation,
                                    attempt: crate::NumericConfirmAttemptId(attempt),
                                    through_sequence: self.numeric_sequence,
                                });
                                self.numeric_confirm_outbound = true;
                            }
                        }
                        None => {
                            self.numeric_error = Some(crate::UiError::InteractionIdentityExhausted)
                        }
                    }
                }
            } else {
                if button == Some(0) && !self.reference_fixture && self.numeric_identity.is_none() {
                    return false;
                }
                self.panel_mut().activate_id(id);
                match button {
                    Some(0) => self.numeric[slot].confirm(),
                    Some(_) => {
                        self.numeric[slot].cancel();
                        self.numeric_confirm = None;
                        self.numeric_formula_open = None;
                    }
                    None => {}
                }
            }
            if !self.panel().showing() {
                self.interaction.set_focused(None);
            }
        }
        true
    }

    /// Applies a press on `part` of tree node `index`. The marker expands or collapses a
    /// branch, the checkbox flips the visibility, every other part selects the row. The row
    /// takes the keyboard focus in all cases. No geometry or model is touched.
    fn activate_tree(&mut self, tree: Tree, index: usize, part: TreePart) {
        let target = self.nodes[tree.slot()][index].target;
        match part {
            TreePart::Marker if has_children(&self.nodes[tree.slot()], index) => {
                if !self.expanded.remove(&target) {
                    self.expanded.insert(target);
                }
            }
            TreePart::Check => {
                if let Some(expected_visible) = self.nodes[tree.slot()][index].check {
                    self.pending_tree_edit = Some((
                        target,
                        TreeIntentKind::SetVisibility {
                            expected_visible,
                            visible: !expected_visible,
                        },
                    ));
                }
            }
            TreePart::Row | TreePart::Name => self.request_entity_selection(target),
            _ => {}
        }
        self.tree_focus = Some(target);
        self.rebuild_tree_slots();
    }

    // ---- menus ----

    /// Applies the outcome of a menu input. Returns `true` unless it was ignored.
    fn apply_menu(&mut self, result: MenuResult) -> bool {
        match result {
            MenuResult::Ignored => false,
            MenuResult::Handled => true,
            MenuResult::Activated(action) => {
                self.run_action(action.get());
                true
            }
            MenuResult::Toggled(action) => {
                if action.get() == ACT_VIEW_COLOR {
                    self.view.color_enabled = !self.view.color_enabled;
                    return true;
                }
                let index = action
                    .get()
                    .checked_sub(ACT_VIEW_CHECK)
                    .and_then(|i| usize::try_from(i).ok());
                if let Some(flag) = index.and_then(|i| self.view.checks.get_mut(i)) {
                    *flag = !*flag;
                    if index == Some(1) && !*flag {
                        self.command_bar.close();
                        self.typed.clear();
                    }
                }
                true
            }
            MenuResult::Cycled(action, step) => {
                if action.get() == data::ACT_VIEW_NAVIGATOR_KIND {
                    if self.view.checks[3] {
                        self.view.navigator_kind = match self.view.navigator_kind {
                            crate::navigation_widgets::NavigatorKind::Gizmo => {
                                crate::navigation_widgets::NavigatorKind::Cube
                            }
                            crate::navigation_widgets::NavigatorKind::Cube => {
                                crate::navigation_widgets::NavigatorKind::Gizmo
                            }
                        };
                    }
                    return true;
                }
                let (slot, len) = match action.get() {
                    ACT_VIEW_PROJECTION => (&mut self.view.projection, PROJECTION.len()),
                    ACT_VIEW_SHADING => (&mut self.view.shading, SHADING.len()),
                    ACT_VIEW_SCALE => (&mut self.view.scale, SCALE_FACTORS_PERCENT.len()),
                    _ => return true,
                };
                *slot = match step {
                    CycleStep::Next if action.get() == ACT_VIEW_SCALE => {
                        slot.saturating_add(1).min(len.saturating_sub(1))
                    }
                    CycleStep::Previous if action.get() == ACT_VIEW_SCALE => slot.saturating_sub(1),
                    CycleStep::Next => cycle_next(*slot, len),
                    CycleStep::Previous => cycle_prev(*slot, len),
                };
                true
            }
        }
    }

    /// Runs an activated menu command. Workbench rows set the StatusBar's workbench,
    /// Create > Extrude / Revolve / Loft / Sweep and Modify > Boolean show that Action Box
    /// (reopening it if OK / Cancel closed it), View > Theme switches the theme and
    /// View > Fullscreen leaves a request for the application.
    fn run_action(&mut self, action: u32) {
        if action == ACT_VIEW_THEME {
            self.view.theme = self.view.theme.toggled();
            // MenuState closes activated items by default. Theme is the one View item
            // whose specification keeps the popup open, with selection on its own row.
            with_bar_menus(&self.view, |menus| {
                self.menu_bar.open(menus, 2);
                for _ in 0..3 {
                    let _ = self.menu_bar.handle_key(menus, MenuKey::Down);
                }
            });
        } else if action == 353 {
            if let (Some(target), Some(origin), Some(id)) = (
                self.focused_features_operation(),
                self.properties_origin,
                self.next_properties_id.checked_add(1),
            ) {
                self.next_properties_id = id;
                self.properties_request = Some(crate::PropertiesOpenRequest {
                    id: crate::PropertiesRequestId(id),
                    target,
                    origin,
                });
            } else {
                self.formula_diagnostic = Some("Focus an Extrude in Features".into());
            }
        } else if action == ACT_VIEW_FULLSCREEN {
            self.fullscreen_request = true;
        } else if action == 220 {
            self.normal_view_request = true;
        } else if action == 221 {
            self.camera_reset_request = true;
        } else if let Some(operation) = ActionOperation::from_action(action) {
            self.show_operation(operation);
        } else if let Some(i) = action
            .checked_sub(ACT_WORKBENCH)
            .and_then(|i| usize::try_from(i).ok())
            .filter(|&i| i < data::WORKBENCHES.len())
        {
            self.workbench = i;
        }
    }

    /// Makes `operation` the shown Action Box. A box still being edited keeps its state; one
    /// closed by OK / Cancel reopens with defaults. The boxes share one id range, so a focus
    /// held in the panel is dropped when the operation changes.
    fn show_operation(&mut self, operation: ActionOperation) {
        self.command_started = true;
        self.numeric[self.operation.slot()].commit();
        self.queue_numeric_edits();
        if let Some((document, session, _)) = self.numeric_identity.take() {
            self.numeric_retired_session = Some((document, session));
        }
        self.numeric_confirm = None;
        self.numeric_confirm_outbound = false;
        self.numeric_formula_open = None;
        self.numeric_formula_outbound = false;
        self.formula_target = None;
        self.formula_apply = None;
        self.formula_apply_outbound = false;
        self.formula_modal_session = None;
        self.formula_diagnostic = None;
        if self.operation != operation {
            if self.interaction.focused.is_some_and(in_action_panel) {
                self.interaction.set_focused(None);
            }
            self.operation = operation;
        }
        if !self.panel().showing() {
            self.panel_mut().restart();
            self.numeric[operation.slot()].reset();
        }
    }

    fn menu_send(&mut self, bar: bool, key: MenuKey) -> MenuResult {
        if bar {
            with_bar_menus(&self.view, |menus| self.menu_bar.handle_key(menus, key))
        } else {
            self.command_bar.handle_key(&COMMAND_MENUS, key)
        }
    }

    // ---- keyboard ----

    fn modal_key(&mut self, key: ShellKey) -> bool {
        let modal = match key {
            ShellKey::Tab => ModalKey::Tab,
            ShellKey::BackTab => ModalKey::BackTab,
            ShellKey::Up => ModalKey::Up,
            ShellKey::Down => ModalKey::Down,
            ShellKey::Enter => ModalKey::Enter,
            ShellKey::Escape => ModalKey::Escape,
            ShellKey::Backspace => ModalKey::Backspace,
            ShellKey::Char(c) => ModalKey::Char(c),
            ShellKey::Left => ModalKey::Edit(EditKey::Left),
            ShellKey::Right => ModalKey::Edit(EditKey::Right),
            ShellKey::Edit(k) => ModalKey::Edit(k),
            ShellKey::MenuBar | ShellKey::CommandBar | ShellKey::Alt(_) => return false,
        };
        if self.selection.is_open() {
            if self.selection.is_profile_picker() {
                self.selection
                    .key_in_action_box(modal, self.profile_selection_frame())
            } else {
                self.selection
                    .key(modal, GridRect::new(0, 0, self.cols, self.rows))
            }
        } else if self.rename.is_open() {
            self.rename.key(modal)
        } else {
            self.formula.key(modal)
        }
    }

    fn menu_or_base_key(&mut self, key: ShellKey) -> bool {
        match key {
            ShellKey::MenuBar => return self.toggle_menu(true),
            ShellKey::CommandBar => return self.toggle_menu(false),
            ShellKey::Alt(c) => return self.alt(c),
            _ => {}
        }
        if self.menu_bar.is_active() {
            self.menu_key(true, key)
        } else if self.command_bar.is_active() {
            self.menu_key(false, key)
        } else {
            self.base_key(key)
        }
    }

    /// F10 / F6: leaves menu navigation if that bar is active, else starts it on that bar.
    fn toggle_menu(&mut self, bar: bool) -> bool {
        if !bar && !self.view_panel_visible(1) {
            self.command_bar.close();
            return true;
        }
        let was_active = if bar {
            self.menu_bar.is_active()
        } else {
            self.command_bar.is_active()
        };
        self.menu_bar.close();
        self.command_bar.close();
        self.typed.clear();
        if !was_active {
            if bar {
                with_bar_menus(&self.view, |menus| self.menu_bar.begin(menus));
            } else {
                self.command_bar.begin(&COMMAND_MENUS);
            }
        }
        true
    }

    /// Alt + letter: a top-level MenuBar mnemonic.
    fn alt(&mut self, c: char) -> bool {
        self.command_bar.close();
        self.typed.clear();
        let result = self.menu_send(true, MenuKey::Mnemonic(Mnemonic::from_char(c)));
        self.apply_menu(result)
    }

    fn menu_key(&mut self, bar: bool, key: ShellKey) -> bool {
        let menu = match key {
            ShellKey::Left => MenuKey::Left,
            ShellKey::Right => MenuKey::Right,
            ShellKey::Up => MenuKey::Up,
            ShellKey::Down => MenuKey::Down,
            ShellKey::Enter | ShellKey::Char(' ') => MenuKey::Enter,
            ShellKey::Escape => MenuKey::Escape,
            ShellKey::Char(c) => return self.typed_char(bar, c),
            ShellKey::Tab
            | ShellKey::BackTab
            | ShellKey::Backspace
            | ShellKey::Edit(_)
            | ShellKey::MenuBar
            | ShellKey::CommandBar
            | ShellKey::Alt(_) => return false,
        };
        self.typed.clear();
        let result = self.menu_send(bar, menu);
        self.apply_menu(result)
    }

    /// Assembles a mnemonic token from typed characters, as [`MenuState`] expects the
    /// controller to. `Mi` is reached by typing `M`, `i`: while a prefix matches nothing it is
    /// kept, up to [`MAX_MNEMONIC`] characters. A character that breaks the token restarts it.
    fn typed_char(&mut self, bar: bool, c: char) -> bool {
        if !c.is_ascii_alphanumeric() {
            self.typed.clear();
            return false;
        }
        if self.typed.chars().count() >= MAX_MNEMONIC {
            self.typed.clear();
        }
        self.typed.push(c);
        let mut result = self.send_token(bar);
        if result == MenuResult::Ignored && self.typed.chars().count() > 1 {
            self.typed = c.to_string();
            result = self.send_token(bar);
        }
        if result != MenuResult::Ignored {
            self.typed.clear();
        }
        self.apply_menu(result)
    }

    fn send_token(&mut self, bar: bool) -> MenuResult {
        let token = Mnemonic::new(&self.typed);
        self.menu_send(bar, MenuKey::Mnemonic(token))
    }

    fn base_key(&mut self, key: ShellKey) -> bool {
        match key {
            ShellKey::Tab => self.step_ring(true),
            ShellKey::BackTab => self.step_ring(false),
            ShellKey::Enter | ShellKey::Char(' ') => {
                let Some(id) = self.interaction.focused else {
                    return false;
                };
                // Enter / Space on a focused row expand or collapse it, like its marker.
                if let Some((tree, index, TreePart::Row)) = self.decode_tree(id) {
                    if self.tree_focus != Some(self.nodes[tree.slot()][index].target) {
                        return false;
                    }
                    let part = if matches!(
                        self.nodes[tree.slot()][index].target.kind,
                        TreeTargetKind::Operation(_) | TreeTargetKind::Entity(_)
                    ) {
                        TreePart::Row
                    } else {
                        TreePart::Marker
                    };
                    self.activate_tree(tree, index, part);
                    return true;
                }
                self.activate_widget(id)
            }
            _ => false,
        }
    }

    fn step_ring(&mut self, forward: bool) -> bool {
        let Ok(grid) = self.compose() else {
            return false;
        };
        let ring = self.focus_ring(&grid);
        let next = step_focus(&ring, self.interaction.focused, forward);
        self.tree_focus = next.and_then(|id| {
            self.decode_tree(id)
                .map(|(tree, index, _)| self.nodes[tree.slot()][index].target)
        });
        let changed = self.interaction.set_focused(next);
        if changed
            && !self.modal_open()
            && !self.numeric_delivery_pending()
            && let Some(id) = next
            && self.panel().is_numeric_field(id)
            && let Some(descriptor) = self.numeric_descriptor(id)
            && self.numeric_field_visible(descriptor.field)
        {
            self.numeric[self.operation.slot()].begin(id, descriptor);
        }
        changed
    }
}

#[cfg(test)]
mod resize_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tree_tests;
