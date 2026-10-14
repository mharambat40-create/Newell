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
//! [`ShellState::compose`] regenerates the one flat [`CellGrid`] from it. There is no widget
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
//! FormulaEditor. The NavCube, viewport, status text and window controls own no id.
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
    ACT_EXTRUDE, ACT_VIEW_CHECK, ACT_VIEW_PROJECTION, ACT_VIEW_SHADING, ACT_WORKBENCH,
    COMMAND_MENUS, ELEMENTS, FEATURES, MAX_NODES, PROJECTION, SHADING, ViewMock, initial_open,
    with_bar_menus,
};

pub use key::ShellKey;

use crate::overlays::step_focus;
use crate::{
    CycleStep, ExtrudeUi, FormulaEditor, GridRect, HitTestResult, InteractionState, MAX_MNEMONIC,
    MenuIds, MenuKey, MenuResult, MenuState, Mnemonic, ModalKey, OverlayStack, PanelRowKind,
    ROW_ID_STRIDE, SelectionPanel, WidgetId, cycle_next, cycle_prev,
};
use compose::Layout;

const MENU_BAR_BASE: u32 = 1_000_000;
const COMMAND_BAR_BASE: u32 = MENU_BAR_BASE + MenuIds::SPAN;
const TREE_BASE: u32 = COMMAND_BAR_BASE + MenuIds::SPAN;
/// Ids reserved for the two trees: features first, then elements.
const TREE_SPAN: u32 = 2 * MAX_NODES as u32;
const EXTRUDE_BASE: u32 = TREE_BASE + TREE_SPAN;
/// Ids reserved for the Extrude panel: 16 row keys of [`ROW_ID_STRIDE`] slots.
const EXTRUDE_SPAN: u32 = 16 * ROW_ID_STRIDE;
const SELECTION_BASE: u32 = EXTRUDE_BASE + EXTRUDE_SPAN;
/// The SelectionPanel uses 14 ids from its base.
const FORMULA_BASE: u32 = SELECTION_BASE + 16;

const BAR_IDS: MenuIds = MenuIds::new(MENU_BAR_BASE);
const COMMAND_IDS: MenuIds = MenuIds::new(COMMAND_BAR_BASE);

/// Mock formula shown by the FormulaEditor (`docs/ui_design/Interface_Elements.txt`).
const MOCK_FORMULA: &str = "Fx = (#EntraxeA - 2mm) / 5 * 3 * sin(#Angle1 - #Angle2 + 3deg)";

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
}

/// Stable id of tree node `index`, independent of which rows are visible.
fn tree_id(tree: Tree, index: usize) -> WidgetId {
    WidgetId::new(TREE_BASE + tree.offset() + u32::try_from(index).unwrap_or(0))
}

/// Inverse of [`tree_id`] for nodes that exist.
fn tree_decode(id: WidgetId) -> Option<(Tree, usize)> {
    let rel = id.get().checked_sub(TREE_BASE)?;
    let (tree, index) = if rel < Tree::Elements.offset() {
        (Tree::Features, rel)
    } else {
        (Tree::Elements, rel - Tree::Elements.offset())
    };
    let index = usize::try_from(index).ok()?;
    (index < tree.nodes().len()).then_some((tree, index))
}

/// The UI state of one application window. See the module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellState {
    cols: u32,
    rows: u32,
    menu_bar: MenuState,
    command_bar: MenuState,
    /// Mnemonic token typed so far in an open menu (see [`ShellState::typed_char`]).
    typed: String,
    view: ViewMock,
    workbench: usize,
    features_open: [bool; MAX_NODES],
    elements_open: [bool; MAX_NODES],
    extrude: ExtrudeUi,
    selection: SelectionPanel,
    formula: FormulaEditor,
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
            menu_bar: MenuState::default(),
            command_bar: MenuState::default(),
            typed: String::new(),
            view: ViewMock::default(),
            workbench: 0,
            features_open: initial_open(&FEATURES),
            elements_open: initial_open(&ELEMENTS),
            extrude: ExtrudeUi::new(EXTRUDE_BASE),
            selection: SelectionPanel::new(SELECTION_BASE),
            formula: FormulaEditor::new(FORMULA_BASE, MOCK_FORMULA),
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

    /// Hover, focus and press state (hover is applied by the shader, not composed).
    #[must_use]
    pub const fn interaction(&self) -> &InteractionState {
        &self.interaction
    }

    /// The Extrude panel's mock state.
    #[must_use]
    pub const fn extrude(&self) -> &ExtrudeUi {
        &self.extrude
    }

    /// The SelectionPanel.
    #[must_use]
    pub const fn selection(&self) -> &SelectionPanel {
        &self.selection
    }

    /// The FormulaEditor.
    #[must_use]
    pub const fn formula(&self) -> &FormulaEditor {
        &self.formula
    }

    /// Index of the active mock workbench, shown in the StatusBar.
    #[must_use]
    pub const fn workbench(&self) -> usize {
        self.workbench
    }

    /// `true` while a modal overlay owns input.
    #[must_use]
    pub fn modal_open(&self) -> bool {
        self.selection.is_open() || self.formula.is_open()
    }

    fn bounds(&self) -> GridRect {
        GridRect::new(0, 0, self.cols, self.rows)
    }

    /// Sets hover from a pointer hit-test of the composed grid (`None` when the pointer left
    /// the grid). Returns `true` if the hovered widget changed. Hover does not change the
    /// composed cells.
    pub fn pointer_moved(&mut self, hit: Option<HitTestResult>) -> bool {
        self.interaction.set_hovered_from_hit(hit)
    }

    /// Applies a pointer activation at `hit`, the hit-test result of the composed grid.
    /// Returns `true` if the click was acted on (recompose the grid); a click on nothing, or
    /// outside an open modal, returns `false`.
    pub fn click(&mut self, hit: Option<HitTestResult>) -> bool {
        let id = hit.and_then(|h| h.widget_id);
        let bounds = self.bounds();
        let changed = if self.selection.is_open() {
            id.is_some_and(|id| self.selection.activate(id, bounds))
        } else if self.formula.is_open() {
            id.is_some_and(|id| self.formula.activate(id))
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
        let layout = Layout::new(self.cols, self.rows);
        let bounds = layout.grid;
        if self.modal_open() {
            self.menu_bar.close();
            self.command_bar.close();
            self.typed.clear();
        }
        self.selection.sync_overlay(&mut self.overlays, bounds);
        self.formula.sync_overlay(&mut self.overlays, bounds);
        let (bar, command) = (layout.menu_bar(), layout.command_bar());
        with_bar_menus(&self.view, |menus| {
            self.menu_bar
                .sync_overlay(&mut self.overlays, menus, BAR_IDS, bar, bounds);
        });
        self.command_bar.sync_overlay(
            &mut self.overlays,
            &COMMAND_MENUS,
            COMMAND_IDS,
            command,
            bounds,
        );
        self.menu_bar.sync_focus(&mut self.interaction, BAR_IDS);
        self.command_bar
            .sync_focus(&mut self.interaction, COMMAND_IDS);
        self.selection.sync_focus(&mut self.interaction);
        self.formula.sync_focus(&mut self.interaction);
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
        if let Some((tree, index)) = tree_decode(id) {
            let open = match tree {
                Tree::Features => &mut self.features_open,
                Tree::Elements => &mut self.elements_open,
            };
            if let (Some(flag), Some(node)) = (open.get_mut(index), tree.nodes().get(index)) {
                if node.branch {
                    *flag = !*flag;
                }
            }
            self.interaction.set_focused(Some(id));
            return true;
        }
        let in_extrude = id
            .get()
            .checked_sub(EXTRUDE_BASE)
            .is_some_and(|rel| rel < EXTRUDE_SPAN);
        if !in_extrude {
            return false;
        }
        // The focus is set first so a modal opened below remembers this control.
        self.interaction.set_focused(Some(id));
        if self.is_formula_button(id) {
            self.formula.open(&mut self.interaction);
        } else if self.is_selection_field(id) {
            self.selection.open(&mut self.interaction);
        } else {
            self.extrude.activate(id);
            if !self.extrude.is_open() {
                self.interaction.set_focused(None);
            }
        }
        true
    }

    /// `true` if `id` is the `☰` button of a numeric row of the open Extrude panel.
    fn is_formula_button(&self, id: WidgetId) -> bool {
        self.extrude.compose().is_some_and(|panel| {
            panel.rows.iter().any(|row| {
                matches!(row.kind, PanelRowKind::Numeric { .. })
                    && panel.widget_id(row.key, 1) == Some(id)
            })
        })
    }

    /// `true` if `id` is a selection field (a text row) of the open Extrude panel.
    fn is_selection_field(&self, id: WidgetId) -> bool {
        self.extrude.compose().is_some_and(|panel| {
            panel.rows.iter().any(|row| {
                matches!(row.kind, PanelRowKind::Text { .. })
                    && panel.widget_id(row.key, 0) == Some(id)
            })
        })
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

    /// Runs an activated menu command. Only two have an effect in the mock: Workbench rows
    /// set the StatusBar's workbench, and Create > Extrude reopens a closed Extrude panel.
    fn run_action(&mut self, action: u32) {
        if action == ACT_EXTRUDE {
            if !self.extrude.is_open() {
                self.extrude.reopen();
            }
        } else if let Some(i) = action
            .checked_sub(ACT_WORKBENCH)
            .and_then(|i| usize::try_from(i).ok())
            .filter(|&i| i < data::WORKBENCHES.len())
        {
            self.workbench = i;
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
            ShellKey::Left
            | ShellKey::Right
            | ShellKey::MenuBar
            | ShellKey::CommandBar
            | ShellKey::Alt(_) => return false,
        };
        if self.selection.is_open() {
            self.selection.key(modal, self.bounds())
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
                let focused = self.interaction.focused;
                focused.is_some_and(|id| self.activate_widget(id))
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
mod tests;
