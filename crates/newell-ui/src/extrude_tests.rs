// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M2.21 tests: the composed Extrude Action Box against `Action_Box_Extrude.txt` and
//! `Interface_Elements.txt`, character for character. The reference files are read, never
//! written.

use std::collections::HashSet;

use super::*;
use crate::{
    CellContent, CellGrid, GridMetrics, GridRect, InteractionState, UiPoint, WidgetContext,
    WidgetStyles, hit_test,
};

const BASE: u32 = 1000;
const EXTRUDE_TXT: &str = include_str!("../../../docs/ui_design/Action_Box_Extrude.txt");
const ELEMENTS_TXT: &str = include_str!("../../../docs/ui_design/Interface_Elements.txt");

// ---- helpers ----

fn ui() -> ExtrudeUi {
    ExtrudeUi::new(BASE)
}

fn with(f: impl FnOnce(&mut ExtrudeValues)) -> ExtrudeUi {
    let mut u = ui();
    f(&mut u.values);
    u
}

fn click(u: &mut ExtrudeUi, key: u32, slot: u32) -> bool {
    let id = u.id(key, slot);
    u.activate(id)
}

fn render(u: &ExtrudeUi) -> Option<CellGrid> {
    let b = u.compose()?;
    let h = u32::try_from(b.height()).unwrap();
    let styles = WidgetStyles::default();
    let inter = InteractionState::default();
    let ctx = WidgetContext {
        styles: &styles,
        interaction: &inter,
    };
    let mut g = CellGrid::new(31, h).unwrap();
    assert_eq!(b.draw(&mut g, &ctx, GridRect::new(0, 0, 31, h)), h as usize);
    Some(g)
}

fn rows_of(g: &CellGrid) -> Vec<String> {
    (0..g.rows())
        .map(|r| {
            (0..g.cols())
                .map(|c| match g.get(c, r).map(|x| x.content) {
                    Some(CellContent::Glyph(ch)) => ch,
                    _ => ' ',
                })
                .collect()
        })
        .collect()
}

fn rows(u: &ExtrudeUi) -> Vec<String> {
    rows_of(&render(u).unwrap())
}

fn has(u: &ExtrudeUi, needle: &str) -> bool {
    rows(u).iter().any(|r| r.contains(needle))
}

fn find(u: &ExtrudeUi, needle: &str) -> usize {
    let all = rows(u);
    let at = all.iter().position(|r| r.contains(needle));
    assert!(at.is_some(), "no row contains {needle:?}");
    at.unwrap_or_default()
}

fn keys_present(u: &ExtrudeUi) -> Vec<u32> {
    let b = u.compose().unwrap();
    (0..crate::create_box::ID_SPAN / KEY_SLOTS)
        .filter(|&k| b.has_key(k))
        .collect()
}

/// The `n`-th (0-based) block of lines under the heading `"name"` of `file`.
fn blocks<'a>(file: &'a str, name: &str) -> Vec<Vec<&'a str>> {
    let heading = format!("\"{name}\"");
    let (mut out, mut cur, mut inside) = (Vec::new(), Vec::new(), false);
    for line in file.lines() {
        if line.starts_with('"') {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            inside = line.trim() == heading;
            continue;
        }
        if !inside {
            continue;
        }
        if line.trim().is_empty() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(line);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// The three complete base boxes of `Action_Box_Extrude.txt`, `╭` to `╰`.
fn base_boxes() -> Vec<Vec<&'static str>> {
    let (mut out, mut cur) = (Vec::new(), Vec::new());
    for line in EXTRUDE_TXT.lines() {
        if line.starts_with('╭') {
            cur.clear();
        }
        if line.starts_with('╭') || !cur.is_empty() {
            cur.push(line);
        }
        if line.starts_with('╰') {
            out.push(std::mem::take(&mut cur));
        }
    }
    out
}

/// Rows `1..5` of variant `index` of the "Head Action Box variants" of `Interface_Elements.txt`.
fn head_variant(index: usize) -> Vec<String> {
    let lines: Vec<&str> = ELEMENTS_TXT.lines().collect();
    let top = lines
        .iter()
        .position(|l| l.starts_with("╭─Operation"))
        .unwrap();
    (1..5)
        .map(|i| {
            lines[top + i]
                .chars()
                .skip(index * 31)
                .take(31)
                .collect::<String>()
        })
        .collect()
}

/// `needle` rows appear contiguously, in order, in the composed box.
fn assert_contains_block(u: &ExtrudeUi, needle: &[&str]) {
    let all = rows(u);
    let found = all
        .windows(needle.len())
        .any(|w| w.iter().zip(needle).all(|(a, b)| a == b));
    assert!(found, "block {needle:#?} not found in\n{}", all.join("\n"));
}

/// The README §2.6 prefix transformation of an expanded last-child offset: cells 1-4
/// `│ │ ` become `│ ╰╴` in the first row, cells 1-6 `│ │ ╰╴` become `│   ╰╴` below.
fn last_child(block: &[&str]) -> Vec<String> {
    block
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let rest: String = row.chars().skip(if i == 0 { 4 } else { 6 }).collect();
            let prefix = if i == 0 { "│ ╰╴" } else { "│   ╰╴" };
            assert!(
                row.starts_with(if i == 0 { "│ │ " } else { "│ │ ╰╴" }),
                "{row}"
            );
            format!("{prefix}{rest}")
        })
        .collect()
}

fn as_strs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

// ---- complete base boxes ----

#[test]
fn the_three_solid_boxes_match_the_reference_row_for_row() {
    let boxes = base_boxes();
    assert_eq!(boxes.len(), 3);
    assert_eq!(boxes[0].len(), 16);
    for (i, result) in [
        ExtrudeResultMode::New,
        ExtrudeResultMode::Add,
        ExtrudeResultMode::Remove,
    ]
    .into_iter()
    .enumerate()
    {
        let u = with(|v| v.result = result);
        let want: Vec<String> = boxes[i].iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(rows(&u), want, "Solid / {result:?}");
    }
}

#[test]
fn surface_boxes_are_the_solid_boxes_with_the_surface_header() {
    let boxes = base_boxes();
    for (i, result) in [
        ExtrudeResultMode::New,
        ExtrudeResultMode::Add,
        ExtrudeResultMode::Remove,
    ]
    .into_iter()
    .enumerate()
    {
        let u = with(|v| {
            v.geometry = ExtrudeGeometryType::Surface;
            v.result = result;
        });
        let got = rows(&u);
        // "Same for Surfaces": everything under the header is the Solid box.
        let body: Vec<String> = boxes[i][5..].iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(got[5..], body[..], "Surface / {result:?}");
        assert_eq!(got[0], boxes[i][0]);
        assert_eq!(
            got[1..5],
            head_variant(3 + i)[..],
            "Surface / {result:?} head"
        );
    }
}

#[test]
fn every_head_variant_equals_interface_elements() {
    for (g, geometry) in [ExtrudeGeometryType::Solid, ExtrudeGeometryType::Surface]
        .into_iter()
        .enumerate()
    {
        for (r, result) in [
            ExtrudeResultMode::New,
            ExtrudeResultMode::Add,
            ExtrudeResultMode::Remove,
        ]
        .into_iter()
        .enumerate()
        {
            let u = with(|v| {
                v.geometry = geometry;
                v.result = result;
            });
            assert_eq!(rows(&u)[1..5], head_variant(g * 3 + r)[..]);
        }
    }
}

#[test]
fn title_is_part_of_the_top_border_and_the_bottom_is_closed() {
    let r = rows(&ui());
    assert_eq!(r[0], "╭─Extrude─────┬───────────────╮");
    assert_eq!(r[r.len() - 1], "╰─────────────────────────────╯");
    assert_eq!(r[r.len() - 3], "│─────────────────────────────│");
    assert_eq!(r[r.len() - 2], "│      OK      │    Cancel    │");
    assert_eq!(
        crate::create_box::top_border("A very long title"),
        "╭─A very long ┬───────────────╮"
    );
}

#[test]
fn active_borders_are_solid_and_inactive_borders_are_dotted() {
    for (g, geometry) in [ExtrudeGeometryType::Solid, ExtrudeGeometryType::Surface]
        .into_iter()
        .enumerate()
    {
        for (r, result) in [
            ExtrudeResultMode::New,
            ExtrudeResultMode::Add,
            ExtrudeResultMode::Remove,
        ]
        .into_iter()
        .enumerate()
        {
            let u = with(|v| {
                v.geometry = geometry;
                v.result = result;
            });
            let head = rows(&u);
            let chars = |row: usize| head[row].chars().collect::<Vec<_>>();
            // The ┆ divider sits beside `New` / `Remove` only when the other pair is active.
            assert_eq!(chars(3)[8] == '┆', r == 2, "{g}/{r}");
            assert_eq!(chars(3)[18] == '┆', r == 0, "{g}/{r}");
            // Solid underlines Solid, Surface underlines Surface; the other is dotted.
            let under: String = chars(2)[1..14].iter().collect();
            let under_surface: String = chars(2)[15..30].iter().collect();
            if g == 0 {
                assert!(!under.contains('╴'), "{under}");
                assert!(under_surface.contains('╴'), "{under_surface}");
            } else {
                assert!(under.contains('╴'), "{under}");
                assert!(!under_surface.contains('╴'), "{under_surface}");
            }
        }
    }
}

// ---- Extend variants ----

#[test]
fn blind_is_the_complete_base_branch_and_keeps_end_offset() {
    let u = ui();
    let r = rows(&u);
    let at = find(&u, "Extend");
    assert_eq!(r[at], "│ Extend   ◀     Blind     ▶  │");
    assert_eq!(r[at + 1], "│ │ Depth   [+|0.0 mm       ☰]│");
    assert_eq!(r[at + 2], "│ │ Along     [Select…       ]│");
    assert_eq!(r[at + 3], "│ │ □ Starting Offset         │");
    assert_eq!(r[at + 4], "│ ╰╴□ End Offset              │");
    // The snippet is a partial excerpt: its first three rows are exact.
    let snippet = &blocks(EXTRUDE_TXT, "Extend variants")[0];
    assert_contains_block(&u, &snippet[..3]);
}

#[test]
fn up_to_and_through_all_equal_their_snippets() {
    let snippets = blocks(EXTRUDE_TXT, "Extend variants");
    let up = with(|v| v.termination = ExtrudeTermination::UpTo);
    assert_contains_block(&up, &snippets[1]);
    let through = with(|v| v.termination = ExtrudeTermination::ThroughAll);
    assert_contains_block(&through, &snippets[2]);
    // They show no End Offset (open reference point 2) and no Blind rows.
    for u in [&up, &through] {
        assert!(!has(u, "End Offset") && !has(u, "Depth") && !has(u, "Along"));
    }
    // Invert exists only under Through all.
    assert!(!has(&ui(), "Invert") && !has(&up, "Invert") && has(&through, "Invert"));
    assert!(has(&through, "│ │ □ Invert                  │"));
}

#[test]
fn switching_extend_never_removes_sibling_controls() {
    for result in [ExtrudeResultMode::New, ExtrudeResultMode::Add] {
        for termination in [
            ExtrudeTermination::Blind,
            ExtrudeTermination::UpTo,
            ExtrudeTermination::ThroughAll,
        ] {
            let u = with(|v| {
                v.result = result;
                v.termination = termination;
            });
            let r = rows(&u);
            for sibling in [
                "│ Profile     [Select…       ]│",
                "│ □ Symetric                  │",
                "│ □ Second end position       │",
                "│      OK      │    Cancel    │",
            ] {
                assert!(r.iter().any(|x| x == sibling), "{termination:?}: {sibling}");
            }
            assert_eq!(
                r.iter().any(|x| x == "│ ■ Merge with all            │"),
                result == ExtrudeResultMode::Add
            );
            // Siblings keep their column: every row is 31 cells wide.
            assert!(r.iter().all(|x| x.chars().count() == 31));
        }
    }
}

// ---- offsets ----

#[test]
fn start_offset_variants_match_the_snippets() {
    let snippets = blocks(EXTRUDE_TXT, "Start Offset variants");
    assert_eq!(snippets.len(), 3);
    // Blind form (non-last child): exact.
    let blind = with(|v| v.starting_offset = true);
    assert_contains_block(&blind, &snippets[1]);
    let up_to = with(|v| {
        v.starting_offset = true;
        v.start_mode = ExtrudeOffsetMode::UpTo;
    });
    assert_contains_block(&up_to, &snippets[2]);
    // Collapsed Blind row keeps its own label (open reference point 1).
    assert!(has(&ui(), "│ │ □ Starting Offset         │"));
    assert!(!has(&ui(), snippets[0][0]));
}

#[test]
fn through_all_start_offset_uses_the_last_child_form() {
    let snippets = blocks(EXTRUDE_TXT, "Start Offset variants");
    let base = |f: fn(&mut ExtrudeValues)| {
        with(|v| {
            v.termination = ExtrudeTermination::ThroughAll;
            f(v);
        })
    };
    let collapsed = base(|_| {});
    assert_contains_block(&collapsed, &["│ ╰╴□ Start Offset            │"]);
    let blind = base(|v| v.starting_offset = true);
    assert_contains_block(&blind, &as_strs(&last_child(&snippets[1])));
    let up_to = base(|v| {
        v.starting_offset = true;
        v.start_mode = ExtrudeOffsetMode::UpTo;
    });
    assert_contains_block(&up_to, &as_strs(&last_child(&snippets[2])));
    assert_contains_block(
        &blind,
        &[
            "│ ╰╴■ Start Offset ◀ Blind ▶  │",
            "│   ╰╴Dist  [+|0.0 mm       ☰]│",
        ],
    );
}

#[test]
fn end_offset_under_blind_is_the_last_child() {
    let snippets = blocks(EXTRUDE_TXT, "End Offset variants 1");
    assert_eq!(snippets.len(), 3);
    assert_contains_block(&ui(), &["│ ╰╴□ End Offset              │"]);
    let blind = with(|v| v.end_offset = true);
    assert_contains_block(&blind, &as_strs(&last_child(&snippets[1])));
    let up_to = with(|v| {
        v.end_offset = true;
        v.end_mode = ExtrudeOffsetMode::UpTo;
    });
    assert_contains_block(&up_to, &as_strs(&last_child(&snippets[2])));
    // The non-last form differs from the last-child form only in the prefix cells.
    for (got, want) in last_child(&snippets[1]).iter().zip(snippets[1].iter()) {
        assert_eq!(got.chars().count(), want.chars().count());
        let g: Vec<char> = got.chars().collect();
        let w: Vec<char> = want.chars().collect();
        assert_eq!(g[6..], w[6..]);
    }
}

#[test]
fn up_to_offset_toggles_only_its_glyph() {
    let mut u = with(|v| v.termination = ExtrudeTermination::UpTo);
    let before = rows(&u);
    assert!(click(&mut u, K_OFFSET, 0));
    let after = rows(&u);
    assert_eq!(before.len(), after.len());
    let diff: Vec<usize> = (0..before.len())
        .filter(|&i| before[i] != after[i])
        .collect();
    assert_eq!(diff, [find(&u, "Offset")]);
    assert_eq!(after[diff[0]], "│ ╰╴■ Offset                  │");
}

// ---- Symetric, 2nd side, Merge, Invert ----

#[test]
fn symetric_removes_the_second_side_and_restores_it() {
    let mut u = ui();
    assert!(has(&u, "│ □ Symetric                  │") && has(&u, "Second end position"));
    assert!(click(&mut u, K_SYMMETRIC, 0));
    let r = rows(&u);
    assert!(r.iter().any(|x| x == "│ ■ Symetric                  │"));
    assert!(!has(&u, "Second end position") && !has(&u, "2nd side"));
    assert_eq!(r.len(), rows(&ui()).len() - 1);
    // A stale id of the removed row does nothing.
    let before = u.clone();
    assert!(!click(&mut u, K_SECOND, 0));
    assert_eq!(u, before);
    assert!(click(&mut u, K_SYMMETRIC, 0));
    assert!(has(&u, "Second end position"));
}

#[test]
fn second_side_variants_match_the_snippets() {
    let snippets = blocks(EXTRUDE_TXT, "2nd side variants");
    assert_eq!(snippets.len(), 3);
    let blind = with(|v| v.second_side = true);
    assert_contains_block(&blind, &snippets[1]);
    let up_to = with(|v| {
        v.second_side = true;
        v.second_mode = ExtrudeOffsetMode::UpTo;
    });
    assert_contains_block(&up_to, &snippets[2]);
    // Collapsed: the complete base box row (label drift, reproduced as written).
    assert!(has(&ui(), "│ □ Second end position       │"));
    let sym = blocks(EXTRUDE_TXT, "Symetric variants");
    assert_eq!(sym[1], ["│ ■ Symetric                  │"]);
    assert_eq!(sym[0][0], "│ □ Symetric                  │");
}

#[test]
fn nested_end_offset_variants_match_the_snippets() {
    let snippets = blocks(EXTRUDE_TXT, "End Offset variants 2");
    assert_eq!(snippets.len(), 3);
    let nested = |end: bool, mode: ExtrudeOffsetMode| {
        with(|v| {
            v.second_side = true;
            v.second_mode = ExtrudeOffsetMode::UpTo;
            v.nested_end = end;
            v.nested_mode = mode;
        })
    };
    assert_contains_block(&nested(false, ExtrudeOffsetMode::Blind), &snippets[0]);
    assert_contains_block(&nested(true, ExtrudeOffsetMode::Blind), &snippets[1]);
    assert_contains_block(&nested(true, ExtrudeOffsetMode::UpTo), &snippets[2]);
    // Nested End Offset only exists under `2nd side = Up to`.
    assert!(!has(&with(|v| v.second_side = true), "│   ╰╴□ End Offset"));
    let mut blind = with(|v| v.second_side = true);
    assert!(!click(&mut blind, K_NESTED, 0));
}

#[test]
fn merge_with_all_variants_match_the_snippets() {
    let snippets = blocks(EXTRUDE_TXT, "Merge with all variants");
    assert_eq!(snippets.len(), 2);
    let on = with(|v| v.result = ExtrudeResultMode::Add);
    assert_contains_block(&on, &snippets[0]);
    let off = with(|v| {
        v.result = ExtrudeResultMode::Remove;
        v.merge_with_all = false;
    });
    assert_contains_block(&off, &snippets[1]);
    // New has no merge rows at all.
    assert!(!has(&ui(), "Merge") && !has(&ui(), "Targets"));
}

#[test]
fn invert_checkbox_states_follow_the_reference() {
    let snippets = blocks(EXTRUDE_TXT, "Invert variants");
    assert_eq!(snippets[0], ["│ □ Invert                    │"]);
    assert_eq!(snippets[1], ["│ ■ Invert                    │"]);
    let mut u = with(|v| v.termination = ExtrudeTermination::ThroughAll);
    assert!(click(&mut u, K_INVERT, 0));
    assert_contains_block(&u, &["│ │ ■ Invert                  │"]);
    assert!(click(&mut u, K_INVERT, 0));
    assert_contains_block(&u, &["│ │ □ Invert                  │"]);
}

// ---- selection fields ----

#[test]
fn selection_fields_show_the_documented_texts() {
    let mut u = ui();
    let at = find(&u, "Profile");
    assert_eq!(rows(&u)[at], "│ Profile     [Select…       ]│");
    assert!(click(&mut u, K_PROFILE, 0));
    assert_eq!(rows(&u)[at], "│ Profile     [1 Element    ☰]│");
    assert!(click(&mut u, K_PROFILE, 0));
    assert_eq!(rows(&u)[at], "│ Profile     [3 Elements   ☰]│");
    assert!(click(&mut u, K_PROFILE, 0));
    assert_eq!(rows(&u)[at], "│ Profile     [Select…       ]│");
    for (ctl, label) in [(K_ALONG, "Along"), (K_SELECTION, "Selection")] {
        let mut u = with(|v| {
            v.termination = if ctl == K_ALONG {
                ExtrudeTermination::Blind
            } else {
                ExtrudeTermination::UpTo
            }
        });
        assert!(click(&mut u, ctl, 0));
        let row = &rows(&u)[find(&u, label)];
        assert!(
            row.contains("[1 Element    ☰]") && row.chars().count() == 31,
            "{row}"
        );
    }
    assert_eq!(MockSelection::One.text(), "1 Element");
    assert_eq!(MockSelection::Many.text(), "3 Elements");
    assert_eq!(MockSelection::None.text(), "Select…");
}

#[test]
fn a_populated_field_has_a_separate_menu_region() {
    let mut u = ui();
    click(&mut u, K_PROFILE, 0);
    let g = render(&u).unwrap();
    let r = find(&u, "Profile") as u32;
    let owner = |c| g.get(c, r).and_then(|x| x.widget_id).map(WidgetId::get);
    assert_eq!(owner(14), Some(u.id(K_PROFILE, 0).get()));
    assert_eq!(owner(28), Some(u.id(K_PROFILE, 1).get()));
    assert_eq!(owner(29), Some(u.id(K_PROFILE, 1).get()));
    assert!(u.is_selection_field(u.id(K_PROFILE, 0)) && u.is_selection_field(u.id(K_PROFILE, 1)));
    assert!(!u.is_formula_button(u.id(K_PROFILE, 1)));
}

// ---- interactions ----

#[test]
fn extend_cycles_in_both_directions_and_wraps() {
    let mut u = ui();
    let t = |u: &ExtrudeUi| u.values.termination;
    assert!(click(&mut u, K_EXTEND, 1));
    assert_eq!(t(&u), ExtrudeTermination::UpTo);
    assert!(has(&u, "│ Extend   ◀     Up to     ▶  │"));
    assert!(click(&mut u, K_EXTEND, 1));
    assert_eq!(t(&u), ExtrudeTermination::ThroughAll);
    assert!(has(&u, "│ Extend   ◀  Through all  ▶  │"));
    assert!(click(&mut u, K_EXTEND, 1));
    assert_eq!(t(&u), ExtrudeTermination::Blind);
    assert!(click(&mut u, K_EXTEND, 0));
    assert_eq!(t(&u), ExtrudeTermination::ThroughAll);
    assert!(click(&mut u, K_EXTEND, 0));
    assert_eq!(t(&u), ExtrudeTermination::UpTo);
}

#[test]
fn two_value_selectors_flip_with_either_arrow() {
    let mut u = with(|v| v.starting_offset = true);
    assert!(click(&mut u, K_START_MODE, 1));
    assert_eq!(u.values.start_mode, ExtrudeOffsetMode::UpTo);
    assert!(click(&mut u, K_START_MODE, 0));
    assert_eq!(u.values.start_mode, ExtrudeOffsetMode::Blind);
    assert!(
        !click(&mut u, K_START_TARGET, 0),
        "Target is absent under Blind"
    );
    // The two modes of the offset hold their own state.
    click(&mut u, K_START_MODE, 1);
    assert!(click(&mut u, K_START_TARGET, 0));
    assert_eq!(u.values.start_target, MockSelection::One);
}

#[test]
fn segments_select_exactly_one_value_each() {
    let mut u = ui();
    assert!(!click(&mut u, K_GEOMETRY, 0), "Solid is already active");
    assert!(click(&mut u, K_GEOMETRY, 1));
    assert_eq!(u.values.geometry, ExtrudeGeometryType::Surface);
    for (slot, mode) in [
        (1, ExtrudeResultMode::Add),
        (2, ExtrudeResultMode::Remove),
        (0, ExtrudeResultMode::New),
    ] {
        assert!(click(&mut u, K_RESULT, slot));
        assert_eq!(u.values.result, mode);
        assert!(!click(&mut u, K_RESULT, slot));
    }
}

#[test]
fn numeric_fields_and_foreign_ids_change_nothing() {
    let mut u = ui();
    let before = u.clone();
    assert!(!click(&mut u, K_DEPTH, 0));
    assert!(!click(&mut u, K_DEPTH, 1));
    assert!(!u.activate(WidgetId::new(0)));
    assert!(!u.activate(WidgetId::new(BASE + crate::create_box::ID_SPAN)));
    assert!(!u.activate(WidgetId::new(u32::MAX)));
    assert_eq!(u, before);
    assert!(u.is_formula_button(u.id(K_DEPTH, 1)));
    assert!(!u.is_formula_button(u.id(K_DEPTH, 0)));
    // A formula button of a row that is not shown is not a formula button.
    assert!(!u.is_formula_button(u.id(K_START_DIST, 1)));
}

#[test]
fn ok_confirms_and_cancel_restores_the_opening_values() {
    let mut ok = ui();
    click(&mut ok, K_RESULT, 2);
    click(&mut ok, K_SYMMETRIC, 0);
    assert!(click(&mut ok, K_ACTIONS, 0));
    assert_eq!(ok.status, ExtrudeStatus::Confirmed);
    assert!(!ok.is_open() && ok.compose().is_none());
    assert_eq!(ok.values.result, ExtrudeResultMode::Remove);
    assert!(!click(&mut ok, K_SYMMETRIC, 0));

    let mut cancel = ui();
    click(&mut cancel, K_RESULT, 2);
    click(&mut cancel, K_EXTEND, 1);
    assert!(click(&mut cancel, K_ACTIONS, 1));
    assert_eq!(cancel.status, ExtrudeStatus::Cancelled);
    assert_eq!(cancel.values, ExtrudeValues::default());
    assert!(cancel.compose().is_none() && render(&cancel).is_none());

    cancel.reopen();
    assert!(cancel.is_open());
    assert_eq!(cancel.values, ExtrudeValues::default());
}

#[test]
fn disclosure_adds_and_removes_physical_rows() {
    let mut u = ui();
    let closed = rows(&u).len();
    click(&mut u, K_START, 0);
    assert_eq!(rows(&u).len(), closed + 1);
    click(&mut u, K_END, 0);
    assert_eq!(rows(&u).len(), closed + 2);
    click(&mut u, K_START, 0);
    click(&mut u, K_END, 0);
    assert_eq!(rows(&u).len(), closed);
    assert_eq!(rows(&u), rows(&ui()));
}

// ---- ids, focus and hit-testing ----

#[test]
fn control_keys_are_stable_across_disclosure() {
    let owner_of = |u: &ExtrudeUi, needle: &str, col: u32| {
        let g = render(u).unwrap();
        let r = u32::try_from(find(u, needle)).unwrap();
        g.get(col, r).and_then(|x| x.widget_id).map(WidgetId::get)
    };
    let mut u = ui();
    let sym = owner_of(&u, "Symetric", 5);
    let ok = owner_of(&u, "OK", 6);
    assert_eq!(sym, Some(u.id(K_SYMMETRIC, 0).get()));
    assert_eq!(ok, Some(u.id(K_ACTIONS, 0).get()));
    click(&mut u, K_RESULT, 1);
    click(&mut u, K_MERGE, 0);
    click(&mut u, K_START, 0);
    click(&mut u, K_END, 0);
    click(&mut u, K_EXTEND, 1);
    assert_eq!(owner_of(&u, "Symetric", 5), sym);
    assert_eq!(owner_of(&u, "OK", 6), ok);
    // The ids are a pure function of base, key and slot.
    assert_eq!(u.id(K_ACTIONS, 1).get(), BASE + K_ACTIONS * KEY_SLOTS + 1);
    assert!(u.id(K_ACTIONS, 1).get() < BASE + crate::create_box::ID_SPAN);
}

#[test]
fn hidden_controls_own_no_cells() {
    let u = ui();
    let g = render(&u).unwrap();
    let present = keys_present(&u);
    for r in 0..g.rows() {
        for c in 0..g.cols() {
            let Some(id) = g.get(c, r).and_then(|x| x.widget_id) else {
                continue;
            };
            assert!(
                present.contains(&((id.get() - BASE) / KEY_SLOTS)),
                "({c},{r})"
            );
        }
    }
    assert!(!present.contains(&K_INVERT) && !present.contains(&K_MERGE));
}

#[test]
fn hit_testing_resolves_segments_dividers_and_buttons_at_every_k() {
    let u = ui();
    let g = render(&u).unwrap();
    let last = g.rows() - 1;
    for k in [2, 3, 5, 7] {
        let m = GridMetrics::new(4 * k, 9 * k).unwrap();
        let at = |col: u32, row: u32, dx: u32| {
            hit_test(
                UiPoint {
                    x: col * 4 * k + dx,
                    y: row * 9 * k + 1,
                },
                m,
                &g,
            )
            .and_then(|h| h.widget_id)
            .map(WidgetId::get)
        };
        let id = |key, slot| Some(u.id(key, slot).get());
        let (left, right) = (0, 4 * k - 1);
        // Solid | Surface: cells, and the shared divider by half.
        assert_eq!(at(5, 1, 1), id(K_GEOMETRY, 0));
        assert_eq!(at(20, 1, 1), id(K_GEOMETRY, 1));
        assert_eq!(at(14, 1, left), id(K_GEOMETRY, 0), "k={k}");
        assert_eq!(at(14, 1, right), id(K_GEOMETRY, 1), "k={k}");
        // New | Add | Remove.
        assert_eq!(at(3, 3, 1), id(K_RESULT, 0));
        assert_eq!(at(12, 3, 1), id(K_RESULT, 1));
        assert_eq!(at(24, 3, 1), id(K_RESULT, 2));
        assert_eq!(at(8, 3, left), id(K_RESULT, 0));
        assert_eq!(at(8, 3, right), id(K_RESULT, 1));
        assert_eq!(at(18, 3, left), id(K_RESULT, 1));
        assert_eq!(at(18, 3, right), id(K_RESULT, 2));
        // OK | Cancel.
        assert_eq!(at(5, last - 1, 1), id(K_ACTIONS, 0));
        assert_eq!(at(22, last - 1, 1), id(K_ACTIONS, 1));
        assert_eq!(at(15, last - 1, left), id(K_ACTIONS, 0));
        assert_eq!(at(15, last - 1, right), id(K_ACTIONS, 1));
        // Frame and labels are inert.
        assert_eq!(at(0, 5, 1), None);
        assert_eq!(at(30, 5, 1), None);
        assert_eq!(at(3, 5, 1), None, "the `Profile` label");
    }
}

#[test]
fn numeric_rows_have_separate_field_and_formula_regions() {
    let u = ui();
    let g = render(&u).unwrap();
    let r = u32::try_from(find(&u, "Depth")).unwrap();
    let owner = |c| g.get(c, r).and_then(|x| x.widget_id).map(WidgetId::get);
    assert_eq!(owner(12), Some(u.id(K_DEPTH, 0).get()));
    assert_eq!(owner(20), Some(u.id(K_DEPTH, 0).get()));
    assert_eq!(owner(28), Some(u.id(K_DEPTH, 1).get()));
    assert_eq!(owner(29), Some(u.id(K_DEPTH, 1).get()));
    assert_eq!(owner(4), None, "the `Depth` label");
}

// ---- exhaustive layout properties ----

/// Every reference row plus the derived forms the README composition rules allow.
fn allowed_rows() -> HashSet<String> {
    let mut set: HashSet<String> = HashSet::new();
    let mut add = |line: &str| {
        let mut forms = vec![line.to_owned()];
        for _ in 0..2 {
            let mut more = Vec::new();
            for f in &forms {
                more.push(f.replace('□', "■"));
                more.push(f.replace('■', "□"));
                more.push(f.replace("[Select…       ]", "[1 Element    ☰]"));
                more.push(f.replace("[Select…       ]", "[3 Elements   ☰]"));
                if f.contains("Offset") && f.starts_with("│ │ ") && !f.starts_with("│ │ ╰╴")
                {
                    more.push(format!("│ ╰╴{}", f.chars().skip(4).collect::<String>()));
                }
                if f.starts_with("│ │ ╰╴") && (f.contains("Dist") || f.contains("Target")) {
                    more.push(format!("│   ╰╴{}", f.chars().skip(6).collect::<String>()));
                }
            }
            forms.extend(more);
        }
        set.extend(forms);
    };
    for line in EXTRUDE_TXT.lines().filter(|l| l.chars().count() == 31) {
        add(line);
    }
    for i in 0..6 {
        for row in head_variant(i) {
            add(&row);
        }
    }
    set
}

#[test]
fn every_state_composes_only_documented_rows_of_31_cells() {
    let allowed = allowed_rows();
    let modes = [ExtrudeOffsetMode::Blind, ExtrudeOffsetMode::UpTo];
    let mut states = 0_u32;
    for geometry in [ExtrudeGeometryType::Solid, ExtrudeGeometryType::Surface] {
        for result in [
            ExtrudeResultMode::New,
            ExtrudeResultMode::Add,
            ExtrudeResultMode::Remove,
        ] {
            for termination in [
                ExtrudeTermination::Blind,
                ExtrudeTermination::UpTo,
                ExtrudeTermination::ThroughAll,
            ] {
                for bits in 0..(1_u32 << 9) {
                    let flag = |i: u32| bits & (1 << i) != 0;
                    let mode = |i: u32| modes[usize::from(flag(i))];
                    let u = with(|v| {
                        v.geometry = geometry;
                        v.result = result;
                        v.termination = termination;
                        v.starting_offset = flag(0);
                        v.start_mode = mode(1);
                        v.end_offset = flag(2);
                        v.end_mode = mode(3);
                        v.symmetric = flag(4);
                        v.second_side = flag(5);
                        v.second_mode = mode(6);
                        v.nested_end = flag(7);
                        v.merge_with_all = flag(8);
                        v.nested_mode = mode(1);
                    });
                    let r = rows(&u);
                    states += 1;
                    assert_eq!(r[0], "╭─Extrude─────┬───────────────╮");
                    assert_eq!(r[r.len() - 1], "╰─────────────────────────────╯");
                    for row in &r {
                        assert_eq!(row.chars().count(), 31, "{row:?}");
                        let first = row.chars().next().unwrap();
                        let last = row.chars().last().unwrap();
                        assert!(
                            matches!(first, '│' | '╭' | '╰' | '├'),
                            "left border of {row:?}"
                        );
                        assert!(
                            matches!(last, '│' | '╮' | '╯' | '┤'),
                            "right border of {row:?}"
                        );
                        assert!(allowed.contains(row), "undocumented row {row:?}");
                    }
                }
            }
        }
    }
    assert_eq!(states, 2 * 3 * 3 * 512);
}

#[test]
fn composition_is_deterministic_and_the_default_matches_the_first_box() {
    let u = ui();
    assert_eq!(u.compose(), u.compose());
    assert_eq!(render(&u), render(&u));
    let first: Vec<String> = base_boxes()[0].iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(rows(&u), first);
}

#[test]
fn the_reference_files_are_not_modified_by_these_tests() {
    // The files are embedded read-only at compile time; this documents the dependency.
    assert!(EXTRUDE_TXT.contains("\"Extrude Action Box\""));
    assert!(ELEMENTS_TXT.contains("\"Head Action Box variants\""));
}
