// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M2.9 menu tests: state machine, ids, pointer through `hit_test`, overlay composition.

use super::*;
use crate::widgets::testing::{ctx, grid, text_row};
use crate::{
    Cell, CellContent, CellGrid, GridMetrics, UiPoint, WidgetStyles, compose_overlays, hit_test,
};

const fn tok(mn: Option<&'static str>) -> Option<Mnemonic> {
    match mn {
        Some(s) => Some(Mnemonic::new(s)),
        None => None,
    }
}

fn mn(s: &str) -> MenuKey {
    MenuKey::Mnemonic(Mnemonic::new(s))
}

const fn item(
    action: u32,
    label: &'static str,
    mn: Option<&'static str>,
    enabled: bool,
) -> MenuEntry<'static> {
    MenuEntry::Item(MenuItem {
        action_offsets: &[],
        action: MenuAction::new(action),
        label,
        mnemonic: tok(mn),
        enabled,
    })
}

// File: 0 New, 1 ---, 2 Open (disabled), 3 Save, 4 Save As, 5 Export
static FILE: [MenuEntry<'static>; 6] = [
    item(1, "New", Some("n"), true),
    MenuEntry::Separator,
    item(2, "Open", Some("o"), false),
    item(3, "Save", Some("s"), true),
    item(4, "Save As", Some("s"), true),
    item(5, "Export", Some("e"), true),
];
static EDIT: [MenuEntry<'static>; 2] = [
    item(10, "Undo", Some("u"), true),
    item(11, "Redo", Some("r"), true),
];
static DISABLED: [MenuEntry<'static>; 3] = [
    item(20, "A", None, false),
    MenuEntry::Separator,
    item(21, "B", Some("b"), false),
];

fn menus() -> [MenuDef<'static>; 4] {
    [
        MenuDef {
            label: "File",
            mnemonic: tok(Some("f")),
            entries: &FILE,
        },
        MenuDef {
            label: "Edit",
            mnemonic: tok(Some("e")),
            entries: &EDIT,
        },
        MenuDef {
            label: "Empty",
            mnemonic: None,
            entries: &[],
        },
        MenuDef {
            label: "Dis",
            mnemonic: tok(Some("d")),
            entries: &DISABLED,
        },
    ]
}

const IDS: MenuIds = MenuIds::new(1000);
const BAR: (u32, u32, u32, u32) = (0, 0, 40, 1);

fn key(s: &mut MenuState, k: MenuKey) -> MenuResult {
    s.handle_key(&menus(), k)
}

fn opened(i: usize) -> MenuState {
    let mut s = MenuState::default();
    assert!(s.open(&menus(), i));
    s
}

fn bar() -> GridRect {
    GridRect::new(BAR.0, BAR.1, BAR.2, BAR.3)
}

/// Bar + overlays composed into a 40x12 grid whose background is owned by id 99.
fn frame(state: &MenuState, inter: &InteractionState) -> (CellGrid, OverlayStack) {
    let styles = WidgetStyles::default();
    let c = ctx(&styles, inter);
    let mut g = grid(40, 12);
    for row in 0..12 {
        for col in 0..40 {
            if let Some(cell) = g.get_mut(col, row) {
                *cell = Cell {
                    ink: crate::InkRole::Text,
                    content: CellContent::Glyph('x'),
                    widget_id: Some(WidgetId::new(99)),
                    ..Cell::default()
                };
            }
        }
    }
    let m = menus();
    draw_menu_bar(&mut g, &c, bar(), IDS, &m, state);
    let mut stack = OverlayStack::default();
    state.sync_overlay(&mut stack, &m, IDS, bar(), GridRect::from_grid(&g));
    let bounds = GridRect::from_grid(&g);
    compose_overlays(&mut g, &c, &stack, bounds, |g, ov, inner| {
        if ov.id == IDS.popup() {
            draw_menu_popup(g, &c, inner, IDS, &m, state);
        }
    });
    (g, stack)
}

fn click(state: &mut MenuState, g: &CellGrid, col: u32, row: u32) -> MenuResult {
    let metrics = GridMetrics::new(6, 10).unwrap();
    let hit = hit_test(
        UiPoint {
            x: col * 6 + 1,
            y: row * 10 + 1,
        },
        metrics,
        g,
    );
    state.handle_click(&menus(), IDS, hit)
}

#[test]
fn initial_state_is_closed() {
    let s = MenuState::default();
    assert!(!s.is_active());
    assert_eq!(
        (s.current_menu(), s.open_menu(), s.highlighted()),
        (None, None, None)
    );
    assert_eq!(
        key(&mut MenuState::default(), MenuKey::Down),
        MenuResult::Ignored
    );
    assert_eq!(
        key(&mut MenuState::default(), MenuKey::Escape),
        MenuResult::Ignored
    );
    let (_, stack) = frame(&s, &InteractionState::default());
    assert!(stack.is_empty());
}

#[test]
fn open_and_switch_top_level() {
    let mut s = MenuState::default();
    assert_eq!(key(&mut s, mn("f")), MenuResult::Handled);
    assert_eq!((s.open_menu(), s.highlighted()), (Some(0), Some(0)));
    assert_eq!(key(&mut s, MenuKey::Right), MenuResult::Handled);
    assert_eq!((s.open_menu(), s.highlighted()), (Some(1), Some(0)));
    key(&mut s, MenuKey::Left);
    key(&mut s, MenuKey::Left); // wraps to the last menu
    assert_eq!(s.open_menu(), Some(3));
    assert_eq!(s.highlighted(), None); // nothing enabled
    key(&mut s, MenuKey::Right);
    assert_eq!(s.open_menu(), Some(0)); // wraps forward
}

#[test]
fn bar_mode_navigation() {
    let mut s = MenuState::default();
    assert!(s.begin(&menus()));
    assert_eq!((s.current_menu(), s.open_menu()), (Some(0), None));
    key(&mut s, MenuKey::Right);
    assert_eq!((s.current_menu(), s.open_menu()), (Some(1), None));
    key(&mut s, MenuKey::Down);
    assert_eq!((s.open_menu(), s.highlighted()), (Some(1), Some(0)));
    let mut s = MenuState::default();
    s.begin(&menus());
    key(&mut s, MenuKey::Up); // opens on the last enabled item
    assert_eq!(s.highlighted(), Some(5));
    let mut s = MenuState::default();
    s.begin(&menus());
    assert_eq!(key(&mut s, MenuKey::Enter), MenuResult::Handled);
    assert_eq!((s.open_menu(), s.highlighted()), (Some(0), Some(0)));
    assert!(!MenuState::default().begin(&[]));
}

#[test]
fn up_down_skip_separators_and_disabled_and_wrap() {
    let mut s = opened(0);
    let mut seen = vec![s.highlighted().unwrap()];
    for _ in 0..4 {
        key(&mut s, MenuKey::Down);
        seen.push(s.highlighted().unwrap());
    }
    assert_eq!(seen, [0, 3, 4, 5, 0]); // separator (1) and disabled (2) skipped, wrapped
    key(&mut s, MenuKey::Up);
    assert_eq!(s.highlighted(), Some(5)); // wraps backwards
    key(&mut s, MenuKey::Up);
    key(&mut s, MenuKey::Up);
    key(&mut s, MenuKey::Up);
    assert_eq!(s.highlighted(), Some(0));
}

#[test]
fn enter_and_space_activate_then_close() {
    for k in [MenuKey::Enter, MenuKey::Space] {
        let mut s = opened(0);
        key(&mut s, MenuKey::Down);
        assert_eq!(key(&mut s, k), MenuResult::Activated(MenuAction::new(3)));
        assert_eq!(s, MenuState::default());
    }
}

#[test]
fn activation_without_enabled_item_is_noop() {
    let mut s = opened(3);
    assert_eq!(key(&mut s, MenuKey::Enter), MenuResult::Handled);
    assert_eq!(s.open_menu(), Some(3));
    let mut s = opened(2);
    assert_eq!(key(&mut s, MenuKey::Down), MenuResult::Handled);
    assert_eq!(key(&mut s, MenuKey::Up), MenuResult::Handled);
    assert_eq!(key(&mut s, MenuKey::Space), MenuResult::Handled);
    assert_eq!(s.highlighted(), None);
}

#[test]
fn escape_dismisses_from_any_state() {
    let mut s = opened(0);
    assert_eq!(key(&mut s, MenuKey::Escape), MenuResult::Handled);
    assert_eq!(s, MenuState::default());
    s.begin(&menus());
    key(&mut s, MenuKey::Escape);
    assert_eq!(s, MenuState::default());
}

#[test]
fn mnemonics() {
    let mut s = opened(0);
    // Unique enabled match activates.
    assert_eq!(
        key(&mut s, mn("N")),
        MenuResult::Activated(MenuAction::new(1))
    );
    assert!(!s.is_active());
    // Item mnemonic wins over a top-level one ('e' = Export vs Edit).
    let mut s = opened(0);
    assert_eq!(
        key(&mut s, mn("e")),
        MenuResult::Activated(MenuAction::new(5))
    );
    // Closed: 'e' opens Edit.
    let mut s = MenuState::default();
    key(&mut s, mn("e"));
    assert_eq!(s.open_menu(), Some(1));
    // Ambiguous 's' moves between Save and Save As without activating.
    let mut s = opened(0);
    assert_eq!(key(&mut s, mn("s")), MenuResult::Handled);
    assert_eq!(s.highlighted(), Some(3));
    key(&mut s, mn("s"));
    assert_eq!(s.highlighted(), Some(4));
    key(&mut s, mn("s"));
    assert_eq!(s.highlighted(), Some(3));
    // Disabled item's mnemonic is ignored; unknown keys are ignored.
    let mut s = opened(0);
    assert_eq!(key(&mut s, mn("o")), MenuResult::Ignored);
    assert_eq!(key(&mut s, mn("z")), MenuResult::Ignored);
    assert_eq!(s.open_menu(), Some(0));
    // Top-level mnemonic switches menus while open.
    key(&mut s, mn("d"));
    assert_eq!(s.open_menu(), Some(3));
}

#[test]
fn widget_ids_are_stable_and_decode() {
    assert_eq!(IDS.title(1), Some(WidgetId::new(1001)));
    assert_eq!(IDS, MenuIds::new(1000));
    assert_eq!(IDS.item(2, 3), IDS.item(2, 3));
    assert_ne!(IDS.item(0, 1), IDS.item(1, 0));
    assert_eq!(IDS.title(MAX_MENUS), None);
    assert_eq!(IDS.item(0, MAX_ENTRIES), None);
    assert_eq!(IDS.item(MAX_MENUS, 0), None);
    assert_eq!(IDS.decode(WidgetId::new(999)), None);
    assert_eq!(IDS.decode(WidgetId::new(1000 + MenuIds::SPAN)), None);
    assert_eq!(IDS.decode(IDS.popup()), Some(MenuTarget::Popup));
    for (m, e) in [(0, 0), (0, 255), (3, 7), (63, 255)] {
        let id = IDS.item(m, e).unwrap();
        assert_eq!(IDS.decode(id), Some(MenuTarget::Item { menu: m, entry: e }));
    }
    assert_eq!(
        IDS.decode(IDS.title(5).unwrap()),
        Some(MenuTarget::Title(5))
    );
    // Extreme base is clamped, ids still decode.
    let high = MenuIds::new(u32::MAX);
    let last = high.item(MAX_MENUS - 1, MAX_ENTRIES - 1).unwrap();
    assert_eq!(
        high.decode(last),
        Some(MenuTarget::Item {
            menu: MAX_MENUS - 1,
            entry: MAX_ENTRIES - 1
        })
    );
    // Ids do not depend on state.
    let (g1, _) = frame(&opened(0), &InteractionState::default());
    let (g2, _) = frame(&opened(1), &InteractionState::default());
    assert_eq!(
        g1.get(1, 0).unwrap().widget_id,
        g2.get(1, 0).unwrap().widget_id
    );
}

#[test]
fn pointer_click_through_real_hit_test() {
    let inter = InteractionState::default();
    let mut s = MenuState::default();
    let (g, _) = frame(&s, &inter);
    // Click "Edit" title (cols 6..12).
    assert_eq!(click(&mut s, &g, 7, 0), MenuResult::Handled);
    assert_eq!((s.open_menu(), s.highlighted()), (Some(1), None));
    // Switch to File.
    assert_eq!(click(&mut s, &g, 1, 0), MenuResult::Handled);
    assert_eq!(s.open_menu(), Some(0));
    let (g, _) = frame(&s, &inter);
    // Disabled "Open" row (entry 2 -> row 4): consumed, still open.
    assert_eq!(click(&mut s, &g, 3, 4), MenuResult::Handled);
    assert_eq!(s.open_menu(), Some(0));
    // Separator (row 3) and border (col 0, row 3): popup surface, still open.
    assert_eq!(click(&mut s, &g, 3, 3), MenuResult::Handled);
    assert_eq!(click(&mut s, &g, 0, 3), MenuResult::Handled);
    assert_eq!(s.open_menu(), Some(0));
    // "Save As" (entry 4 -> row 6) activates and closes.
    assert_eq!(
        click(&mut s, &g, 3, 6),
        MenuResult::Activated(MenuAction::new(4))
    );
    assert!(!s.is_active());
    // Clicking the open title again toggles it closed.
    let mut s = opened(0);
    let (g, _) = frame(&s, &inter);
    click(&mut s, &g, 1, 0);
    assert!(!s.is_active());
}

#[test]
fn click_outside_dismisses_and_idle_click_is_ignored() {
    let inter = InteractionState::default();
    let mut s = opened(0);
    let (g, stack) = frame(&s, &inter);
    assert!(stack.is_open(IDS.popup()));
    assert_eq!(click(&mut s, &g, 30, 8), MenuResult::Handled); // background widget 99
    assert_eq!(s, MenuState::default());
    let (_, stack) = frame(&s, &inter);
    assert!(!stack.is_open(IDS.popup()));
    assert_eq!(click(&mut s, &g, 30, 8), MenuResult::Ignored);
    // Click outside the grid (no hit) also dismisses.
    let mut s = opened(0);
    assert_eq!(s.handle_click(&menus(), IDS, None), MenuResult::Handled);
    assert!(!s.is_active());
}

#[test]
fn overlay_composition_covers_background_and_clips() {
    let (g, stack) = frame(&opened(0), &InteractionState::default());
    let top = stack.top().unwrap();
    assert_eq!((top.kind, top.id), (OverlayKind::Popup, IDS.popup()));
    assert_eq!(top.rect, GridRect::new(0, 1, 13, 8));
    assert_eq!(
        text_row(&g, 0).get(..5),
        Some(" File"[..].replace(' ', ".").as_str())
    );
    assert!(text_row(&g, 2).starts_with("│.New"));
    assert!(text_row(&g, 3).starts_with("│───────────│"));
    // Cells under the popup lost the background owner; cells beside it keep it.
    assert_ne!(g.get(3, 2).unwrap().widget_id, Some(WidgetId::new(99)));
    assert_eq!(g.get(13, 2).unwrap().widget_id, Some(WidgetId::new(99)));
    assert_eq!(g.get(3, 9).unwrap().widget_id, Some(WidgetId::new(99)));
    // Items own their rows.
    assert_eq!(g.get(3, 2).unwrap().widget_id, IDS.item(0, 0));
    assert_eq!(g.get(9, 6).unwrap().widget_id, IDS.item(0, 4));

    // A small grid shrinks the popup to fit bounds.
    let styles = WidgetStyles::default();
    let inter = InteractionState::default();
    let c = ctx(&styles, &inter);
    let s = opened(0);
    let m = menus();
    let mut small = grid(8, 4);
    let bounds = GridRect::from_grid(&small);
    let mut stack = OverlayStack::default();
    s.sync_overlay(&mut stack, &m, IDS, GridRect::new(0, 0, 8, 1), bounds);
    assert!(bounds.intersection(&stack.top().unwrap().rect) == stack.top().unwrap().rect);
    compose_overlays(&mut small, &c, &stack, bounds, |g, _, inner| {
        draw_menu_popup(g, &c, inner, IDS, &m, &s);
    });
    assert_eq!(stack.top().unwrap().rect.height(), 3);
}

#[test]
fn highlight_uses_interaction_focus() {
    let styles = WidgetStyles::default();
    let mut inter = InteractionState::default();
    let mut s = opened(0);
    key(&mut s, MenuKey::Down);
    s.sync_focus(&mut inter, IDS);
    assert_eq!(inter.focused, IDS.item(0, 3));
    let (g, _) = frame(&s, &inter);
    assert_eq!(g.get(3, 5).unwrap().style, styles.focused);
    assert_eq!(g.get(3, 2).unwrap().style, styles.normal);
    assert_eq!(g.get(3, 4).unwrap().style, styles.disabled); // Open
    // Open title is drawn selected.
    assert_eq!(g.get(1, 0).unwrap().style, styles.selected);
    // Closing clears our focus but leaves foreign focus alone.
    s.close();
    s.sync_focus(&mut inter, IDS);
    assert_eq!(inter.focused, None);
    inter.set_focused(Some(WidgetId::new(7)));
    s.sync_focus(&mut inter, IDS);
    assert_eq!(inter.focused, Some(WidgetId::new(7)));
    // Bar mode focuses the title.
    s.begin(&menus());
    s.sync_focus(&mut inter, IDS);
    assert_eq!(inter.focused, IDS.title(0));
}

#[test]
fn output_is_deterministic() {
    let mut s = opened(0);
    key(&mut s, MenuKey::Down);
    let inter = InteractionState::default();
    let (a, sa) = frame(&s, &inter);
    let (b, sb) = frame(&s, &inter);
    assert_eq!(a.cells(), b.cells());
    assert_eq!(sa, sb);
}

#[test]
fn tiny_and_degenerate_grids_do_not_panic() {
    let styles = WidgetStyles::default();
    let inter = InteractionState::default();
    let c = ctx(&styles, &inter);
    let m = menus();
    let s = opened(0);
    for (cols, rows) in [(1, 1), (2, 2), (5, 1), (1, 9)] {
        let mut g = grid(cols, rows);
        let bounds = GridRect::from_grid(&g);
        let bar = GridRect::new(0, 0, cols, 1);
        draw_menu_bar(&mut g, &c, bar, IDS, &m, &s);
        let mut stack = OverlayStack::default();
        s.sync_overlay(&mut stack, &m, IDS, bar, bounds);
        compose_overlays(&mut g, &c, &stack, bounds, |g, _, inner| {
            draw_menu_popup(g, &c, inner, IDS, &m, &s);
        });
    }
    // Empty rects and rects far outside the grid.
    let mut g = grid(4, 4);
    for r in [
        GridRect::new(0, 0, 0, 0),
        GridRect::new(u32::MAX, u32::MAX, 5, 5),
    ] {
        draw_menu_bar(&mut g, &c, r, IDS, &m, &s);
        draw_menu_popup(&mut g, &c, r, IDS, &m, &s);
        assert!(title_rect(r, &m, 0).is_some());
    }
}

#[test]
fn empty_and_disabled_menus_are_safe() {
    // No menus at all.
    let mut s = MenuState::default();
    for k in [MenuKey::Down, MenuKey::Enter, MenuKey::Left, mn("f")] {
        assert_eq!(s.handle_key(&[], k), MenuResult::Ignored);
    }
    assert!(!s.open(&[], 0));
    assert_eq!(s.handle_click(&[], IDS, None), MenuResult::Ignored);
    // Empty entries.
    let mut s = opened(2);
    for k in [MenuKey::Down, MenuKey::Up, MenuKey::Enter, MenuKey::Space] {
        assert_eq!(key(&mut s, k), MenuResult::Handled);
        assert_eq!(s.highlighted(), None);
    }
    let (g, _) = frame(&s, &InteractionState::default());
    assert_eq!(text_row(&g, 2).chars().nth(14), Some('└'));
    // All disabled.
    let mut s = opened(3);
    key(&mut s, MenuKey::Down);
    assert_eq!(s.highlighted(), None);
    assert_eq!(key(&mut s, mn("b")), MenuResult::Ignored);
    assert!(s.is_active());
}

#[test]
fn extreme_state_does_not_panic() {
    let m = menus();
    let extreme = MenuState {
        active: true,
        menu: usize::MAX,
        open: true,
        item: Some(usize::MAX),
    };
    for k in [
        MenuKey::Left,
        MenuKey::Right,
        MenuKey::Up,
        MenuKey::Down,
        MenuKey::Enter,
        MenuKey::Space,
        MenuKey::Escape,
        mn("s"),
    ] {
        let mut s = extreme;
        s.handle_key(&m, k);
        assert!(s == MenuState::default() || s.menu < m.len());
    }
    // Stale item index in a valid menu.
    for k in [MenuKey::Up, MenuKey::Down, MenuKey::Enter, mn("s")] {
        let mut s = MenuState {
            active: true,
            menu: 1,
            open: true,
            item: Some(usize::MAX),
        };
        s.handle_key(&m, k);
    }
    // Stale ids / out-of-range targets in clicks.
    let g = grid(2, 2);
    let mut s = extreme;
    let hit = crate::HitTestResult {
        col: 0,
        row: 0,
        widget_id: IDS.item(63, 255),
    };
    assert_eq!(s.handle_click(&m, IDS, Some(hit)), MenuResult::Handled);
    let s = extreme;
    s.sync_overlay(
        &mut OverlayStack::default(),
        &m,
        IDS,
        bar(),
        GridRect::from_grid(&g),
    );
    assert_eq!(s.focus_id(IDS), IDS.title(usize::MAX));
    // Capacity: menus past MAX_MENUS are ignored, never aliased.
    static LIST: [MenuDef<'static>; MAX_MENUS + 3] = [MenuDef {
        label: "m",
        mnemonic: None,
        entries: &EDIT,
    }; MAX_MENUS + 3];
    let mut s = MenuState::default();
    assert!(!s.open(&LIST, MAX_MENUS));
    assert!(s.open(&LIST, MAX_MENUS - 1));
    key_on(&mut s, &LIST, MenuKey::Right);
    assert_eq!(s.open_menu(), Some(0));
}

fn key_on(s: &mut MenuState, m: &[MenuDef<'_>], k: MenuKey) {
    s.handle_key(m, k);
}

// ---- View-style menu: cyclic and checkbox rows, multi-character mnemonics ----

const fn cmd(a: u32, label: &'static str, m: &'static str) -> MenuEntry<'static> {
    item(a, label, Some(m), true)
}

static VALUES: [&str; 2] = ["Perspective", "Orthographic"];
static SHADES: [&str; 2] = ["Shaded", "Wireframe"];

fn view_entries(checked: bool, persp: usize) -> [MenuEntry<'static>; 13] {
    let cb = |action, label, checked, enabled| {
        MenuEntry::Checkbox(MenuCheckbox {
            action: MenuAction::new(action),
            label,
            checked,
            mnemonic: tok(Some("t")),
            enabled,
        })
    };
    [
        MenuEntry::Cyclic(MenuCycle {
            label: "",
            action: MenuAction::new(30),
            values: &VALUES,
            current: persp,
            bounded: false,
            enabled: true,
        }),
        MenuEntry::Cyclic(MenuCycle {
            label: "",
            action: MenuAction::new(31),
            values: &SHADES,
            current: 0,
            bounded: false,
            enabled: false,
        }),
        cb(32, "Tree panel", checked, true),
        cb(33, "Grid Overlay", false, false),
        MenuEntry::Separator,
        cmd(34, "[N]ormal to", "n"),
        cmd(35, "[Mi]rror", "Mi"),
        cmd(36, "[M]ove", "M"),
        cmd(37, "[Li]near", "Li"),
        cmd(38, "[Ci]rcular", "Ci"),
        cmd(39, "[Cu]rve", "Cu"),
        cmd(40, "[Pl]ane", "Pl"),
        cmd(41, "[Mi]dpoint", "mI"),
    ]
}

fn vmenus<'a>(entries: &'a [MenuEntry<'static>]) -> [MenuDef<'a>; 1] {
    [MenuDef {
        label: "View",
        mnemonic: tok(Some("v")),
        entries,
    }]
}

fn vopen() -> MenuState {
    let mut s = MenuState::default();
    assert!(s.open(&vmenus(&view_entries(true, 0)), 0));
    s
}

fn vkey(s: &mut MenuState, k: MenuKey) -> MenuResult {
    s.handle_key(&vmenus(&view_entries(true, 0)), k)
}

fn vframe(s: &MenuState, inter: &InteractionState, e: &[MenuEntry<'static>]) -> CellGrid {
    let styles = WidgetStyles::default();
    let c = ctx(&styles, inter);
    let mut g = grid(40, 20);
    let m = vmenus(e);
    draw_menu_bar(&mut g, &c, bar(), IDS, &m, s);
    let mut stack = OverlayStack::default();
    let bounds = GridRect::from_grid(&g);
    s.sync_overlay(&mut stack, &m, IDS, bar(), bounds);
    compose_overlays(&mut g, &c, &stack, bounds, |g, _, inner| {
        draw_menu_popup(g, &c, inner, IDS, &m, s);
    });
    g
}

fn vclick(s: &mut MenuState, g: &CellGrid, col: u32, row: u32) -> MenuResult {
    let hit = hit_test(
        UiPoint {
            x: col * 6 + 1,
            y: row * 10 + 1,
        },
        GridMetrics::new(6, 10).unwrap(),
        g,
    );
    s.handle_click(&vmenus(&view_entries(true, 0)), IDS, hit)
}

fn act(a: u32) -> MenuResult {
    MenuResult::Activated(MenuAction::new(a))
}

#[test]
fn mnemonic_tokens_are_normalised() {
    assert_eq!(Mnemonic::new("Mi"), Mnemonic::new("mI"));
    assert_ne!(Mnemonic::new("M"), Mnemonic::new("Mi"));
    assert_eq!(Mnemonic::from_char('M'), Mnemonic::new("m"));
    for bad in ["", "abcde", "é", "a b", "-"] {
        assert!(Mnemonic::new(bad).is_empty(), "{bad:?}");
    }
    assert!(Mnemonic::from_char('é').is_empty());
    // An empty token never matches, not even another empty one.
    let mut s = vopen();
    assert_eq!(vkey(&mut s, mn("")), MenuResult::Ignored);
}

#[test]
fn multi_character_mnemonics_activate_exactly() {
    // Exact token: `M` is the Move item only, never Mirror (`Mi`).
    for (token, action) in [
        ("M", 36),
        ("n", 34),
        ("Li", 37),
        ("Ci", 38),
        ("Cu", 39),
        ("Pl", 40),
    ] {
        let mut s = vopen();
        assert_eq!(vkey(&mut s, mn(token)), act(action), "{token}");
        assert!(!s.is_active());
    }
    // Case-insensitive.
    for (token, action) in [
        ("m", 36),
        ("li", 37),
        ("LI", 37),
        ("cI", 38),
        ("CU", 39),
        ("pL", 40),
    ] {
        let mut s = vopen();
        assert_eq!(vkey(&mut s, mn(token)), act(action), "{token}");
    }
    // Prefix and unknown tokens do nothing.
    let mut s = vopen();
    assert_eq!(vkey(&mut s, mn("L")), MenuResult::Ignored);
    assert_eq!(vkey(&mut s, mn("Pla")), MenuResult::Ignored);
    assert!(s.is_active());
}

#[test]
fn ambiguous_multi_character_mnemonic_cycles() {
    // `Mi` is Mirror (6) and Midpoint (12), in any case.
    let mut s = vopen();
    let mut seen = Vec::new();
    for t in ["Mi", "MI", "mi", "mI"] {
        assert_eq!(vkey(&mut s, mn(t)), MenuResult::Handled);
        seen.push(s.highlighted().unwrap());
    }
    assert_eq!(seen, [6, 12, 6, 12]);
    // Enter then activates the highlighted one.
    assert_eq!(vkey(&mut s, MenuKey::Enter), act(41));
}

#[test]
fn checkbox_row_renders_and_toggles() {
    let e = view_entries(true, 0);
    let inter = InteractionState::default();
    let g = vframe(&vopen(), &inter, &e);
    // Row of entry 2 is row 4 (frame row 1, border row 1... inner starts at row 2).
    assert!(
        text_row(&g, 4).starts_with("│.■.Tree.panel"),
        "{}",
        text_row(&g, 4)
    );
    assert!(
        text_row(&g, 5).starts_with("│.□.Grid.Overlay"),
        "{}",
        text_row(&g, 5)
    );
    let unchecked = view_entries(false, 0);
    let g2 = vframe(&vopen(), &inter, &unchecked);
    assert!(text_row(&g2, 4).starts_with("│.□.Tree.panel"));
    // Whole row is owned by the stable item id.
    assert_eq!(g.get(3, 4).unwrap().widget_id, IDS.item(0, 2));
    assert_eq!(g.get(15, 4).unwrap().widget_id, IDS.item(0, 2));
    // Keyboard: Enter / Space toggle without closing or mutating anything.
    for k in [MenuKey::Enter, MenuKey::Space] {
        let mut s = vopen();
        vkey(&mut s, MenuKey::Down); // skips the disabled cyclic row
        assert_eq!(s.highlighted(), Some(2));
        assert_eq!(vkey(&mut s, k), MenuResult::Toggled(MenuAction::new(32)));
        assert_eq!((s.open_menu(), s.highlighted()), (Some(0), Some(2)));
    }
    // Mnemonic toggles too.
    let mut s = vopen();
    assert_eq!(
        vkey(&mut s, mn("T")),
        MenuResult::Toggled(MenuAction::new(32))
    );
    // Pointer through hit_test.
    let mut s = vopen();
    assert_eq!(
        vclick(&mut s, &g, 4, 4),
        MenuResult::Toggled(MenuAction::new(32))
    );
    assert_eq!(s.highlighted(), Some(2));
    // Up/Down skip the disabled checkbox (3) and separator (4).
    vkey(&mut s, MenuKey::Down);
    assert_eq!(s.highlighted(), Some(5));
}

#[test]
fn disabled_checkbox_and_cyclic_rows_are_inert() {
    let e = view_entries(true, 0);
    let styles = WidgetStyles::default();
    let g = vframe(&vopen(), &InteractionState::default(), &e);
    // Disabled cyclic (entry 1, row 3) and checkbox (entry 3, row 5).
    assert_eq!(g.get(4, 3).unwrap().style, styles.disabled);
    assert_eq!(g.get(4, 5).unwrap().style, styles.disabled);
    assert_eq!(g.get(2, 3).unwrap().widget_id, IDS.cycle_prev(0, 1));
    for (col, row) in [(4, 5), (4, 3), (2, 3), (18, 3)] {
        let mut s = vopen();
        assert_eq!(vclick(&mut s, &g, col, row), MenuResult::Handled);
        assert_eq!(s.open_menu(), Some(0), "({col},{row})");
    }
    // Keyboard never lands on them.
    let mut s = vopen();
    let mut seen = Vec::new();
    for _ in 0..4 {
        seen.push(s.highlighted().unwrap());
        vkey(&mut s, MenuKey::Down);
    }
    assert_eq!(seen, [0, 2, 5, 6]);
}

#[test]
fn cyclic_row_shows_only_current_value() {
    let inter = InteractionState::default();
    let e = view_entries(true, 0);
    let g = vframe(&vopen(), &inter, &e);
    let row = text_row(&g, 2);
    assert!(row.contains("◀") && row.contains("Perspective") && row.contains('▶'));
    assert!(!row.contains("Orthographic"));
    let e1 = view_entries(true, 1);
    let g1 = vframe(&vopen(), &inter, &e1);
    let row = text_row(&g1, 2);
    assert!(row.contains("Orthographic") && !row.contains("Perspective"));
    // Controls and value area have stable, distinct owners.
    assert_eq!(g.get(2, 2).unwrap().content, CellContent::Glyph('◀'));
    assert_eq!(g.get(2, 2).unwrap().widget_id, IDS.cycle_prev(0, 0));
    assert_eq!(g.get(18, 2).unwrap().content, CellContent::Glyph('▶'));
    assert_eq!(g.get(18, 2).unwrap().widget_id, IDS.cycle_next(0, 0));
    assert_eq!(g.get(8, 2).unwrap().widget_id, IDS.item(0, 0));
    assert_ne!(IDS.cycle_prev(0, 0), IDS.cycle_next(0, 0));
    assert_ne!(IDS.item(0, 0), IDS.cycle_prev(0, 0));
    // Highlighted row shows through the style.
    let s = vopen();
    let mut inter = InteractionState::default();
    s.sync_focus(&mut inter, IDS);
    let g = vframe(&s, &inter, &e);
    assert_eq!(g.get(8, 2).unwrap().style, WidgetStyles::default().focused);
}

#[test]
fn cyclic_keyboard_requests_previous_next_and_ordinary_rows_switch_menus() {
    let mut s = vopen();
    let a = MenuAction::new(30);
    assert_eq!(
        vkey(&mut s, MenuKey::Right),
        MenuResult::Cycled(a, CycleStep::Next)
    );
    assert_eq!(
        vkey(&mut s, MenuKey::Left),
        MenuResult::Cycled(a, CycleStep::Previous)
    );
    // State machine unchanged: still open on the same row.
    assert_eq!((s.open_menu(), s.highlighted()), (Some(0), Some(0)));
    // Enter / Space on a cyclic row: consumed, no request.
    assert_eq!(vkey(&mut s, MenuKey::Enter), MenuResult::Handled);
    assert_eq!(vkey(&mut s, MenuKey::Space), MenuResult::Handled);
    // On an ordinary row Right switches top-level menu (single menu: wraps to itself,
    // re-highlighting the first enabled row).
    vkey(&mut s, MenuKey::Down);
    assert_eq!(s.highlighted(), Some(2));
    assert_eq!(vkey(&mut s, MenuKey::Right), MenuResult::Handled);
    assert_eq!(s.highlighted(), Some(0));
    // The caller applies the step with the existing helpers.
    assert_eq!(crate::cycle_next(1, VALUES.len()), 0);
    assert_eq!(crate::cycle_prev(0, VALUES.len()), 1);
}

#[test]
fn cyclic_pointer_uses_real_hit_test() {
    let e = view_entries(true, 0);
    let g = vframe(&vopen(), &InteractionState::default(), &e);
    let a = MenuAction::new(30);
    let mut s = vopen();
    vkey(&mut s, MenuKey::Down);
    assert_eq!(s.highlighted(), Some(2));
    assert_eq!(
        vclick(&mut s, &g, 18, 2),
        MenuResult::Cycled(a, CycleStep::Next)
    );
    assert_eq!(s.highlighted(), Some(0));
    assert_eq!(
        vclick(&mut s, &g, 2, 2),
        MenuResult::Cycled(a, CycleStep::Previous)
    );
    // Clicking the value text selects nothing and keeps the popup open.
    assert_eq!(vclick(&mut s, &g, 9, 2), MenuResult::Handled);
    assert_eq!(s.open_menu(), Some(0));
    // A cycle control click is ignored when its menu is not open (dismisses instead).
    let mut closed = MenuState::default();
    assert_eq!(vclick(&mut closed, &g, 19, 2), MenuResult::Ignored);
}

#[test]
fn view_ids_decode_and_menu_is_deterministic() {
    for (m, e) in [(0, 0), (0, 255), (63, 255), (5, 7)] {
        assert_eq!(
            IDS.decode(IDS.item(m, e).unwrap()),
            Some(MenuTarget::Item { menu: m, entry: e })
        );
        assert_eq!(
            IDS.decode(IDS.cycle_prev(m, e).unwrap()),
            Some(MenuTarget::CyclePrev { menu: m, entry: e })
        );
        assert_eq!(
            IDS.decode(IDS.cycle_next(m, e).unwrap()),
            Some(MenuTarget::CycleNext { menu: m, entry: e })
        );
    }
    assert_eq!(IDS.cycle_prev(0, MAX_ENTRIES), None);
    assert_eq!(IDS.decode(WidgetId::new(1000 + MenuIds::SPAN)), None);
    let e = view_entries(true, 0);
    let inter = InteractionState::default();
    let s = vopen();
    assert_eq!(
        vframe(&s, &inter, &e).cells(),
        vframe(&s, &inter, &e).cells()
    );
}

#[test]
fn command_items_unchanged_in_mixed_menu() {
    let e = view_entries(true, 0);
    let g = vframe(&vopen(), &InteractionState::default(), &e);
    // "Normal to" is entry 5 -> row 7.
    let mut s = vopen();
    assert_eq!(vclick(&mut s, &g, 4, 7), act(34));
    assert!(!s.is_active());
    let mut s = vopen();
    for _ in 0..2 {
        vkey(&mut s, MenuKey::Down);
    }
    assert_eq!(s.highlighted(), Some(5));
    assert_eq!(vkey(&mut s, MenuKey::Enter), act(34));
}

#[test]
fn cyclic_with_empty_values_or_bad_current_is_safe() {
    let e = [
        MenuEntry::Cyclic(MenuCycle {
            label: "",
            action: MenuAction::new(50),
            values: &[],
            current: 7,
            bounded: false,
            enabled: true,
        }),
        MenuEntry::Cyclic(MenuCycle {
            label: "",
            action: MenuAction::new(51),
            values: &VALUES,
            current: usize::MAX,
            bounded: false,
            enabled: true,
        }),
    ];
    let m = vmenus(&e);
    let mut s = MenuState::default();
    assert!(s.open(&m, 0));
    let g = vframe(&s, &InteractionState::default(), &e);
    assert_eq!(g.get(2, 2).unwrap().widget_id, IDS.cycle_prev(0, 0));
    let a = MenuAction::new(50);
    assert_eq!(
        s.handle_key(&m, MenuKey::Right),
        MenuResult::Cycled(a, CycleStep::Next)
    );
    s.handle_key(&m, MenuKey::Down);
    assert_eq!(s.highlighted(), Some(1));
    let _ = popup_size(&m[0]);
}
