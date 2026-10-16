// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! The reference application shell (M2.13): plain state, input routing and composition.
//!
//! [`ShellState`] owns the UI-only state of one window: the MenuBar and CommandBar
//! [`MenuState`]s (M2.9), the mock tree expansion, the Extrude panel (M2.11), the
//! SelectionPanel and FormulaEditor (M2.12), the interaction state and the overlay stack.
//! [`ShellState::compose`] regenerates the one flat [`crate::CellGrid`] from it. There is no widget
//! tree, no second interaction system and no geometry: every control routes to a primitive
//! that already exists, by [`WidgetId`].
//!
//! # Regions
//!
//! `MenuBar`, `CommandBar`, `NavigationPanel` (Features over Elements), `Viewport3D` (the
//! clear colour shows through its empty cells, plus a static NavCube frame), the Extrude
//! `ActionPanel` and the `StatusBar`; see `docs/ui_design/Interface_Exemple.txt`.
//!
//! # Input routing
//!
//! Pointer and keyboard input go to exactly one owner, in this order:
//!
//! 1. an open **modal** (SelectionPanel or FormulaEditor): it takes every key, and a click
//!    reaches only its own controls, because composition already cleared the ids of every
//!    covered cell (M2.8). Closing restores the focus held before it opened (M2.12);
//! 2. an active **menu** (MenuBar or CommandBar): clicks on its own ids go to it, any other
//!    click closes it and is consumed; keys feed [`MenuState::handle_key`];
//! 3. the **base shell**: tree rows toggle, Extrude controls activate, `☰` opens the
//!    FormulaEditor and a selection field opens the SelectionPanel. Tab / Shift+Tab walk the
//!    focus ring and Enter / Space activate the focused id, through the same
//!    [`ShellState::activate_widget`] as a click.
//!
//! A modal and a menu popup are never open together: opening a modal closes the menus.
//!
//! # Widget ids
//!
//! Fixed bases, independent of layout and state (the offsets and spans are in the `*_BASE`
//! constants below): MenuBar, CommandBar, tree rows (64 per tree), Extrude panel, SelectionPanel,
//! FormulaEditor, window controls. The NavCube, viewport and status text own no id.
//!
//! # Mock state
//!
//! Everything is deterministic mock data. The View menu's rows, the tree markers and the
//! FormulaEditor text change only what is displayed. No selection, formula or command is
//! evaluated, and nothing here refers to the model or the kernel.

mod compose;
mod data;
mod key;

use data::{
    ACT_BOOLEAN, ACT_EXTRUDE, ACT_LOFT, ACT_REVOLVE, ACT_SWEEP, ACT_VIEW_CHECK,
    ACT_VIEW_FULLSCREEN, ACT_VIEW_PROJECTION, ACT_VIEW_SCALE, ACT_VIEW_SHADING, ACT_VIEW_THEME,
    ACT_WORKBENCH, COMMAND_MENUS, DEFAULT_PLAYHEAD, ELEMENTS, FEATURES, Line, MAX_NODES, NodeKind,
    PROJECTION, SHADING, ViewMock, clamp_playhead, full_label, has_children, initial_checks,
    initial_open, top_level_count, visible_lines, with_bar_menus,
};

pub use data::ThemeChoice;
pub use key::ShellKey;

use crate::create_box::{ActionBoxUi, ID_SPAN, K_ACTIONS};
use crate::overlays::step_focus;
use crate::widgets::NumericFields;
use crate::{
    BooleanUi, CycleStep, EditKey, ExtrudeUi, FormulaEditor, GridRect, HitTestResult,
    InteractionState, LoftUi, MAX_MNEMONIC, MenuIds, MenuKey, MenuResult, MenuState, MenuTarget,
    Mnemonic, ModalKey, OverlayStack, RenameModal, RevolveUi, SCALE_PERCENTS, SelectionPanel,
    SweepUi, WidgetId, WindowAction, cycle_next, cycle_prev,
};
use compose::{PanelWidths, Split, TOP_ROWS, features_rows, nav_width};

pub use compose::Separator;

const MENU_BAR_BASE: u32 = 1_000_000;
const COMMAND_BAR_BASE: u32 = MENU_BAR_BASE + MenuIds::SPAN;
const TREE_BASE: u32 = COMMAND_BAR_BASE + MenuIds::SPAN;
/// Ids per tree node: the row itself, its marker, its checkbox and its name (see [`TreePart`]).
const TREE_STRIDE: u32 = 4;
/// Ids of the node rows: features first, then elements; the playhead id follows them.
const NODE_IDS: u32 = 2 * MAX_NODES as u32 * TREE_STRIDE;
/// The playhead separator row of the Features tree.
const PLAYHEAD_ID: u32 = TREE_BASE + NODE_IDS;
/// Ids reserved for the two trees and the playhead.
const TREE_SPAN: u32 = NODE_IDS + 1;
/// First id of the Action Box. The five boxes share this range: only one is shown at a time.
const ACTION_BASE: u32 = TREE_BASE + TREE_SPAN;
const SELECTION_BASE: u32 = ACTION_BASE + ID_SPAN;
/// The SelectionPanel uses 14 ids from its base.
const FORMULA_BASE: u32 = SELECTION_BASE + 16;
/// The Rename modal uses 4 ids from its base.
const RENAME_BASE: u32 = FORMULA_BASE + 16;
/// The three window controls `[-] [□] [X]`, in that order.
const WINDOW_BASE: u32 = RENAME_BASE + 16;
const WINDOW_ACTIONS: [WindowAction; 3] = [
    WindowAction::Minimize,
    WindowAction::ToggleMaximize,
    WindowAction::Close,
];

const BAR_IDS: MenuIds = MenuIds::new(MENU_BAR_BASE);
const COMMAND_IDS: MenuIds = MenuIds::new(COMMAND_BAR_BASE);

/// The operation whose Action Box is shown in the ActionPanel (`Create` and `Modify` menus).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
    const fn nodes(self) -> &'static [data::Node] {
        match self {
            Self::Features => &FEATURES,
            Self::Elements => &ELEMENTS,
        }
    }

    const fn offset(self) -> u32 {
        match self {
            Self::Features => 0,
            Self::Elements => MAX_NODES as u32,
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

/// The part of a tree row an id belongs to. Every cell of a row is owned by one of them, so
/// a press tells the expansion marker, the visibility checkbox and the name apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TreePart {
    /// Guides, spacing and the blanks after the name: selects the row.
    Row = 0,
    /// `[-]` / `[+]`: expands or collapses.
    Marker = 1,
    /// `□` / `■`: toggles the visibility.
    Check = 2,
    /// The name: selects the row; a double-click renames a named row.
    Name = 3,
}

impl TreePart {
    const ALL: [Self; 4] = [Self::Row, Self::Marker, Self::Check, Self::Name];
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
    (index < tree.nodes().len()).then_some((tree, index, part))
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
    workbench: usize,
    features_open: [bool; MAX_NODES],
    elements_open: [bool; MAX_NODES],
    /// Visibility checkboxes, per tree and node (meaningful for rows that have one).
    checks: [[bool; MAX_NODES]; 2],
    /// Names the user gave, per tree and node; `None` keeps the mock name.
    names: [[Option<String>; MAX_NODES]; 2],
    /// The row last pressed. Logical state only: selection has no visual (`README_UI.md` §5.10).
    selected: Option<(Tree, usize)>,
    /// Top-level features before the playhead separator (see [`data::clamp_playhead`]).
    playhead: usize,
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
    formula_target: Option<WidgetId>,
    rename: RenameModal,
    /// The row the open Rename modal edits.
    rename_target: Option<(Tree, usize)>,
    interaction: InteractionState,
    overlays: OverlayStack,
}

impl ShellState {
    /// The reference shell for a grid of `cols` x `rows` cells: Extrude panel open, trees
    /// as in the reference, no menu or modal open.
    #[must_use]
    pub fn new(cols: u32, rows: u32) -> Self {
        let mut state = Self {
            cols,
            rows,
            widths: PanelWidths::default(),
            menu_bar: MenuState::default(),
            command_bar: MenuState::default(),
            typed: String::new(),
            view: ViewMock::default(),
            fullscreen_request: false,
            workbench: 0,
            features_open: initial_open(&FEATURES),
            elements_open: initial_open(&ELEMENTS),
            checks: [initial_checks(&FEATURES), initial_checks(&ELEMENTS)],
            names: std::array::from_fn(|_| std::array::from_fn(|_| None)),
            selected: None,
            playhead: DEFAULT_PLAYHEAD,
            operation: ActionOperation::Extrude,
            extrude: ExtrudeUi::new(ACTION_BASE),
            revolve: RevolveUi::new(ACTION_BASE),
            loft: LoftUi::new(ACTION_BASE),
            sweep: SweepUi::new(ACTION_BASE),
            boolean: BooleanUi::new(ACTION_BASE),
            selection: SelectionPanel::new(SELECTION_BASE),
            formula: FormulaEditor::new(FORMULA_BASE),
            numeric: Default::default(),
            formula_target: None,
            rename: RenameModal::new(RENAME_BASE),
            rename_target: None,
            interaction: InteractionState::default(),
            overlays: OverlayStack::default(),
        };
        state.sync();
        state
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
        self.cols = cols;
        self.rows = rows;
        self.sync();
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

    /// Asks for a NavigationPanel `wanted` cells wide; the width is clamped to the grid
    /// (at least 8 cells, at most half the grid). Returns `true` if the laid-out width changed.
    pub fn set_nav_width(&mut self, wanted: u32) -> bool {
        let before = self.nav_width();
        self.widths.nav = nav_width(self.cols, wanted);
        self.nav_width() != before
    }

    /// `true` while no menu popup or modal owns input, so panels can be dragged and the
    /// window moved.
    fn input_free(&self) -> bool {
        !(self.modal_open() || self.menu_bar.is_active() || self.command_bar.is_active())
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
        let before = self.layout().features.height();
        let body = self.layout().viewport.height();
        let wanted = row.saturating_add(1).saturating_sub(TOP_ROWS);
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

    /// Moves the playhead separator to directly after the top-level feature (with its
    /// subtree) under the pointer's `row`, clamped to between two top-level features, so the
    /// nesting of the tree is never touched. A row above the tree counts as the first feature,
    /// one below as the last. Returns `true` if the position changed.
    fn drag_playhead(&mut self, row: u32) -> bool {
        let nodes = Tree::Features.nodes();
        let top = top_level_count(nodes);
        let current = clamp_playhead(self.playhead, top);
        let lines = visible_lines(nodes, &self.features_open, current);
        let first = self.layout().features.inset(1).row();
        let gap = match row.checked_sub(first).map(|d| d as usize) {
            None => 0,
            Some(i) => match lines.get(i) {
                Some(Line::Playhead) => return false,
                Some(_) => lines[..=i]
                    .iter()
                    .filter(|l| {
                        matches!(l, Line::Node(r) if nodes.get(r.index).is_some_and(|n| n.depth == 1))
                    })
                    .count(),
                None => top,
            },
        };
        let Some(new) = clamp_playhead(gap, top) else {
            return false;
        };
        self.playhead = new;
        current != Some(new)
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
        if hit.widget_id.is_some() {
            return hit.widget_id;
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
        !(tree_decode(id).is_some()
            || id.get() == PLAYHEAD_ID
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
    pub fn is_rename_target(id: WidgetId) -> bool {
        tree_decode(id).is_some_and(|(tree, index, part)| {
            part == TreePart::Name
                && tree
                    .nodes()
                    .get(index)
                    .is_some_and(|n| n.kind == NodeKind::Named)
        })
    }

    /// Opens the Rename modal on the row named by `id`, starting from its full stored name.
    /// Returns `false` (and does nothing) if `id` is not a rename target or a modal is open.
    pub fn begin_rename(&mut self, id: WidgetId) -> bool {
        let Some((tree, index, _)) = tree_decode(id).filter(|_| Self::is_rename_target(id)) else {
            return false;
        };
        if self.modal_open() {
            return false;
        }
        let name = full_label(
            tree.nodes(),
            index,
            self.names[tree.slot()][index].as_deref(),
        );
        self.rename_target = Some((tree, index));
        // The row keeps the focus; the modal remembers it and gives it back.
        self.interaction.set_focused(Some(tree_id(tree, index)));
        self.rename.open(&mut self.interaction, &name);
        self.sync();
        true
    }

    /// The UI theme chosen in the View menu. Light for every new shell.
    #[must_use]
    pub const fn theme(&self) -> ThemeChoice {
        self.view.theme
    }

    /// The stored UI Scale preference (60/80/100/120), as the View menu shows it. It is
    /// never a derived value: the shell holds no `k`, monitor or window-fit logic.
    #[must_use]
    pub fn preferred_scale_percent(&self) -> u32 {
        self.view.scale_percent()
    }

    /// `true` once after the Fullscreen item was activated; the application owns the window
    /// and decides (toggle, refusal).
    pub fn take_fullscreen_request(&mut self) -> bool {
        std::mem::take(&mut self.fullscreen_request)
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

    fn bounds(&self) -> GridRect {
        GridRect::new(0, 0, self.cols, self.rows)
    }

    /// Applies a pointer activation at `hit`, the hit-test result of the composed grid.
    /// Returns `true` if the click was acted on (recompose the grid); a click on nothing, or
    /// outside an open modal, returns `false`.
    pub fn click(&mut self, hit: Option<HitTestResult>) -> bool {
        let id = hit.and_then(|h| h.widget_id);
        let bounds = self.bounds();
        if self.editing_field().is_some_and(|f| Some(f) != id) {
            self.numeric[self.operation.slot()].commit();
        }
        let changed = if self.selection.is_open() {
            id.is_some_and(|id| self.selection.activate(id, bounds))
        } else if self.formula.is_open() {
            id.is_some_and(|id| self.formula.activate(id))
        } else if self.rename.is_open() {
            id.is_some_and(|id| self.rename.activate(id))
        } else if let Some(handled) = self.click_menu(id, hit) {
            handled
        } else {
            id.is_some_and(|id| self.activate_widget(id))
        };
        self.sync();
        changed
    }

    /// Applies a translated key. Returns `true` if the key was acted on (recompose the
    /// grid). Keys with an unusable character (control characters) are ignored.
    pub fn key(&mut self, key: ShellKey) -> bool {
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

    /// Brings overlays and focus in line with the state: a modal closes the menus; popups
    /// and modals are opened or dismissed; focus follows the active owner and is restored
    /// when a modal closes. Called after every input.
    fn sync(&mut self) {
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
        self.selection.sync_overlay(&mut self.overlays, bounds);
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
    }

    /// Applies the name of a Rename that was confirmed, once the modal has closed; a cancelled
    /// one changes nothing.
    fn finish_rename(&mut self) {
        if self.rename.is_open() {
            return;
        }
        if let Some((tree, index)) = self.rename_target.take() {
            if self.rename.status() == crate::ModalStatus::Confirmed {
                self.names[tree.slot()][index] = Some(self.rename.text().to_owned());
            }
        }
    }

    /// Keeps the formula text of a FormulaEditor that was confirmed, for the numeric field it
    /// was opened from; a cancelled one changes nothing.
    fn finish_formula(&mut self) {
        if self.formula.is_open() {
            return;
        }
        let Some(field) = self.formula_target.take() else {
            return;
        };
        if self.formula.status() != crate::ModalStatus::Confirmed {
            return;
        }
        if let Some(mock) = self.numeric_mock(field) {
            self.numeric[self.operation.slot()].set_formula(field, &mock, self.formula.text());
        }
    }

    /// The mock text (`+|0.0 mm`) of numeric field `id` in the shown Action Box.
    fn numeric_mock(&self, id: WidgetId) -> Option<String> {
        let panel = self.panel().compose_box()?;
        panel.numeric_text(id).map(str::to_owned)
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
        if let Some((tree, index, part)) = tree_decode(id) {
            self.activate_tree(tree, index, part);
            return true;
        }
        if !in_action_panel(id) {
            return false;
        }
        // The focus is set first so a modal opened below remembers this control.
        self.interaction.set_focused(Some(id));
        let slot = self.operation.slot();
        if self.panel().is_formula_button(id) {
            // The `☰` is slot 1 of the field it belongs to (slot 0).
            let field = WidgetId::new(id.get() - 1);
            let text = self.numeric[slot].formula(field).to_owned();
            self.formula_target = Some(field);
            self.formula.open(&mut self.interaction, &text);
        } else if self.panel().is_numeric_field(id) {
            if let Some(mock) = self.numeric_mock(id) {
                self.numeric[slot].begin(id, &mock);
            }
        } else if self.panel().is_selection_field(id) {
            self.selection.open(&mut self.interaction);
        } else {
            let button = self
                .panel()
                .live_control(id)
                .filter(|&(key, slot)| key == K_ACTIONS && slot < 2)
                .map(|(_, slot)| slot);
            self.panel_mut().activate_id(id);
            match button {
                Some(0) => self.numeric[slot].confirm(),
                Some(_) => self.numeric[slot].cancel(),
                None => {}
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
        let nodes = tree.nodes();
        match part {
            TreePart::Marker => {
                let open = match tree {
                    Tree::Features => &mut self.features_open,
                    Tree::Elements => &mut self.elements_open,
                };
                if let (true, Some(flag)) = (has_children(nodes, index), open.get_mut(index)) {
                    *flag = !*flag;
                }
            }
            TreePart::Check => {
                if let (Some(_), Some(flag)) = (
                    nodes.get(index).and_then(|n| n.check),
                    self.checks[tree.slot()].get_mut(index),
                ) {
                    *flag = !*flag;
                }
            }
            TreePart::Row | TreePart::Name => self.selected = Some((tree, index)),
        }
        self.interaction.set_focused(Some(tree_id(tree, index)));
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
                let check = action
                    .get()
                    .checked_sub(ACT_VIEW_CHECK)
                    .and_then(|i| usize::try_from(i).ok())
                    .and_then(|i| self.view.checks.get_mut(i));
                if let Some(flag) = check {
                    *flag = !*flag;
                }
                true
            }
            MenuResult::Cycled(action, step) => {
                let (slot, len) = match action.get() {
                    ACT_VIEW_PROJECTION => (&mut self.view.projection, PROJECTION.len()),
                    ACT_VIEW_SHADING => (&mut self.view.shading, SHADING.len()),
                    ACT_VIEW_SCALE => (&mut self.view.scale, SCALE_PERCENTS.len()),
                    _ => return true,
                };
                *slot = match step {
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
        } else if action == ACT_VIEW_FULLSCREEN {
            self.fullscreen_request = true;
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
        self.numeric[self.operation.slot()].commit();
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
            self.selection.key(modal, self.bounds())
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
                if let Some((tree, index, TreePart::Row)) = tree_decode(id) {
                    self.activate_tree(tree, index, TreePart::Marker);
                    self.selected = Some((tree, index));
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
        self.interaction.set_focused(next)
    }
}

#[cfg(test)]
mod resize_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tree_tests;
