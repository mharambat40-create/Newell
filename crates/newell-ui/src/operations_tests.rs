// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M2.22 tests: the composed Revolve, Loft, Sweep and Boolean Action Boxes against
//! `Action_Box_Revolve/Loft/Sweep/Boolean.txt` and `Interface_Elements.txt`, character for
//! character. The reference files are read, never written. Controls are found by their text
//! on the composed grid and activated through their widget id, as a click does.

use std::collections::HashSet;

use crate::action_box::BOX_WIDTH;
use crate::create_box::{ActionBoxUi, ID_SPAN};
use crate::extrude::ExtrudeStatus;
use crate::{
    BooleanOperation, BooleanUi, CellContent, CellGrid, ExtrudeGeometryType, ExtrudeResultMode,
    GridMetrics, GridRect, InteractionState, LoftEndType, LoftUi, MockSelection, RevolveUi,
    SweepUi, UiPoint, WidgetContext, WidgetId, WidgetStyles, hit_test,
};

const BASE: u32 = 2000;
const REVOLVE_TXT: &str = include_str!("../../../docs/ui_design/Action_Box_Revolve.txt");
const LOFT_TXT: &str = include_str!("../../../docs/ui_design/Action_Box_Loft.txt");
const SWEEP_TXT: &str = include_str!("../../../docs/ui_design/Action_Box_Sweep.txt");
const BOOLEAN_TXT: &str = include_str!("../../../docs/ui_design/Action_Box_Boolean.txt");
const ELEMENTS_TXT: &str = include_str!("../../../docs/ui_design/Interface_Elements.txt");

// ---- helpers ----

fn grid_of(u: &dyn ActionBoxUi) -> Option<CellGrid> {
    let b = u.compose_box()?;
    let h = u32::try_from(b.height()).unwrap();
    let styles = WidgetStyles::default();
    let inter = InteractionState::default();
    let ctx = WidgetContext {
        styles: &styles,
        interaction: &inter,
    };
    let mut g = CellGrid::new(BOX_WIDTH, h).unwrap();
    assert_eq!(
        b.draw(&mut g, &ctx, GridRect::new(0, 0, BOX_WIDTH, h)),
        h as usize
    );
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

fn rows(u: &dyn ActionBoxUi) -> Vec<String> {
    rows_of(&grid_of(u).unwrap())
}

fn has(u: &dyn ActionBoxUi, needle: &str) -> bool {
    rows(u).iter().any(|r| r.contains(needle))
}

/// The `╭`..`╰` boxes of a reference file, in order.
fn base_boxes(file: &str) -> Vec<Vec<String>> {
    let (mut out, mut cur) = (Vec::new(), Vec::new());
    for line in file.lines() {
        if line.starts_with('╭') {
            cur.clear();
        }
        if line.starts_with('╭') || !cur.is_empty() {
            cur.push(line.to_owned());
        }
        if line.starts_with('╰') {
            out.push(std::mem::take(&mut cur));
        }
    }
    out
}

/// The blocks (runs of non-blank lines) under the heading `"name"` of `file`.
fn blocks(file: &str, name: &str) -> Vec<Vec<String>> {
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
            cur.push(line.to_owned());
        }
    }
    if !cur.is_empty() {
        out.push(cur);
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

/// `needle` appears as consecutive rows of the composed box.
fn assert_block(u: &dyn ActionBoxUi, needle: &[String]) {
    let all = rows(u);
    let found = all.windows(needle.len()).any(|w| w == needle);
    assert!(found, "block {needle:#?} not found in\n{}", all.join("\n"));
}

/// `(col, row)` of the first occurrence of `needle` in the composed box.
fn locate(u: &dyn ActionBoxUi, needle: &str) -> (u32, u32) {
    let found = rows(u).iter().enumerate().find_map(|(r, line)| {
        line.find(needle).map(|byte| {
            (
                u32::try_from(line[..byte].chars().count()).unwrap(),
                u32::try_from(r).unwrap(),
            )
        })
    });
    assert!(found.is_some(), "{needle:?} not shown");
    found.unwrap()
}

fn id_at(u: &dyn ActionBoxUi, col: u32, row: u32) -> Option<WidgetId> {
    grid_of(u).unwrap().get(col, row).and_then(|c| c.widget_id)
}

/// The id of the cell where `needle` starts.
fn id_of(u: &dyn ActionBoxUi, needle: &str) -> WidgetId {
    let (c, r) = locate(u, needle);
    let id = id_at(u, c, r);
    assert!(id.is_some(), "{needle:?} owns no widget id");
    id.unwrap()
}

/// Activates the control whose text starts with `needle`.
fn press(u: &mut dyn ActionBoxUi, needle: &str) -> bool {
    let id = id_of(u, needle);
    u.activate_id(id)
}

/// Activates the `glyph` (`◀` / `▶`) of the row that contains `row_needle`.
fn arrow(u: &mut dyn ActionBoxUi, row_needle: &str, glyph: char) -> bool {
    let (_, r) = locate(u, row_needle);
    let line = rows(u)[r as usize].clone();
    let col = line.chars().position(|c| c == glyph).unwrap();
    let id = id_at(u, u32::try_from(col).unwrap(), r).unwrap();
    u.activate_id(id)
}

/// The id of the field `[...]` of the row that contains `row_needle`.
fn field_id(u: &dyn ActionBoxUi, row_needle: &str) -> WidgetId {
    let (_, r) = locate(u, row_needle);
    let line = rows(u)[r as usize].clone();
    let col = line.chars().position(|c| c == '[').unwrap();
    id_at(u, u32::try_from(col).unwrap(), r).unwrap()
}

/// Activates the field `[...]` of the row that contains `row_needle`.
fn field(u: &mut dyn ActionBoxUi, row_needle: &str) -> bool {
    let id = field_id(u, row_needle);
    u.activate_id(id)
}

/// Every row of a composed box is 31 cells wide with a border at both ends.
fn assert_framed(u: &dyn ActionBoxUi) {
    for row in rows(u) {
        let chars: Vec<char> = row.chars().collect();
        assert_eq!(chars.len(), 31, "{row}");
        assert!("╭├│╰".contains(chars[0]), "left border of {row}");
        assert!("╮┤│╯".contains(chars[30]), "right border of {row}");
    }
}

const RESULTS: [ExtrudeResultMode; 3] = [
    ExtrudeResultMode::New,
    ExtrudeResultMode::Add,
    ExtrudeResultMode::Remove,
];

fn revolve() -> RevolveUi {
    RevolveUi::new(BASE)
}
fn loft() -> LoftUi {
    LoftUi::new(BASE)
}
fn sweep() -> SweepUi {
    SweepUi::new(BASE)
}
fn boolean() -> BooleanUi {
    BooleanUi::new(BASE)
}

/// The Solid boxes equal the reference row for row; the Surface boxes equal the Solid body under
/// the `Interface_Elements.txt` Surface header ("Same for Surfaces").
fn assert_create_boxes(
    file: &str,
    mut make: impl FnMut(ExtrudeGeometryType, ExtrudeResultMode) -> Vec<String>,
) {
    let boxes = base_boxes(file);
    assert_eq!(boxes.len(), 3);
    for (i, result) in RESULTS.into_iter().enumerate() {
        assert_eq!(
            make(ExtrudeGeometryType::Solid, result),
            boxes[i],
            "Solid / {result:?}"
        );
        let surface = make(ExtrudeGeometryType::Surface, result);
        assert_eq!(surface[0], boxes[i][0]);
        assert_eq!(surface[1..5], head_variant(3 + i)[..], "Surface head");
        assert_eq!(surface[5..], boxes[i][5..], "Surface / {result:?} body");
    }
}

// ---- Revolve ----

#[test]
fn revolve_complete_boxes_match_the_reference() {
    assert_create_boxes(REVOLVE_TXT, |geometry, result| {
        let mut u = revolve();
        u.values.geometry = geometry;
        u.values.result = result;
        rows(&u)
    });
}

#[test]
fn revolve_full_revolve_variants() {
    let v = blocks(REVOLVE_TXT, "Full Revolve variants");
    assert_eq!(v.len(), 3);
    assert_block(&revolve(), &v[0]);
    let mut u = revolve();
    u.values.full = false;
    assert_block(&u, &v[1]);
    u.values.mode = crate::ExtrudeOffsetMode::UpTo;
    assert_block(&u, &v[2]);
}

#[test]
fn revolve_offset_variants() {
    let start = blocks(REVOLVE_TXT, "Start Offset variants");
    let end1 = blocks(REVOLVE_TXT, "End Offset variants 1");
    assert_eq!((start.len(), end1.len()), (3, 3));
    let mut u = revolve();
    u.values.full = false;
    u.values.mode = crate::ExtrudeOffsetMode::UpTo;
    assert_block(&u, &start[0]);
    assert_block(&u, &end1[0]);
    u.values.start_offset = true;
    u.values.end_offset = true;
    for (i, mode) in [
        crate::ExtrudeOffsetMode::Blind,
        crate::ExtrudeOffsetMode::UpTo,
    ]
    .into_iter()
    .enumerate()
    {
        u.values.start_mode = mode;
        u.values.end_mode = mode;
        assert_block(&u, &start[1 + i]);
        assert_block(&u, &end1[1 + i]);
        assert_framed(&u);
    }
    // `End Offset` exists only for `Up to`; `Start Offset` for both.
    let mut blind = revolve();
    blind.values.full = false;
    blind.values.end_offset = true;
    assert!(!has(&blind, "End Offset"));
    assert!(has(&blind, "Start Offset"));
}

#[test]
fn revolve_second_side_symmetric_and_nested_variants() {
    let second = blocks(REVOLVE_TXT, "2nd side variants");
    let nested = blocks(REVOLVE_TXT, "End Offset variants 2");
    let symmetric = blocks(REVOLVE_TXT, "Symetric variants");
    assert_eq!((second.len(), nested.len(), symmetric.len()), (3, 3, 2));
    let mut u = revolve();
    u.values.full = false;
    assert_block(&u, &symmetric[0]);
    assert_block(&u, &second[0]);
    u.values.second_side = true;
    assert_block(&u, &second[1]);
    u.values.second_mode = crate::ExtrudeOffsetMode::UpTo;
    assert_block(&u, &second[2][..2]);
    assert_block(&u, &nested[0]);
    u.values.nested_end = true;
    for (i, mode) in [
        crate::ExtrudeOffsetMode::Blind,
        crate::ExtrudeOffsetMode::UpTo,
    ]
    .into_iter()
    .enumerate()
    {
        u.values.nested_mode = mode;
        assert_block(&u, &nested[1 + i]);
    }
    // An active `Symetric` removes `2nd side` and becomes the last child.
    u.values.symmetric = true;
    assert_block(&u, &symmetric[1]);
    assert!(!has(&u, "2nd side"));
    assert_framed(&u);
}

#[test]
fn revolve_merge_variants_and_siblings() {
    let merge = blocks(REVOLVE_TXT, "Merge with all variants");
    let mut u = revolve();
    u.values.result = ExtrudeResultMode::Add;
    assert_block(&u, &merge[0]);
    u.values.merge_with_all = false;
    assert_block(&u, &merge[1]);
    u.values.result = ExtrudeResultMode::New;
    assert!(!has(&u, "Merge with all"));
    // Every state keeps Profile, Axis, Full Revolve and OK / Cancel.
    for full in [true, false] {
        for mode in [
            crate::ExtrudeOffsetMode::Blind,
            crate::ExtrudeOffsetMode::UpTo,
        ] {
            u.values.full = full;
            u.values.mode = mode;
            for needle in ["Profile", "Axis", "Full Revolve", "OK", "Cancel"] {
                assert!(has(&u, needle), "{needle} with full={full} {mode:?}");
            }
            assert_framed(&u);
        }
    }
}

#[test]
fn revolve_interactions() {
    let mut u = revolve();
    assert!(!has(&u, "Angle"));
    assert!(press(&mut u, "Full Revolve"));
    assert!(!u.values.full && has(&u, "Angle   [+|0.0 deg      ☰]"));
    // Either arrow flips Blind / Up to.
    assert!(arrow(&mut u, "Full Revolve", '▶'));
    assert!(has(&u, "Target    [Select…       ]") && has(&u, "End Offset"));
    assert!(arrow(&mut u, "Full Revolve", '◀'));
    assert!(has(&u, "Angle") && !has(&u, "End Offset"));
    // Selection fields cycle None -> 1 Element -> N Elements -> None.
    assert!(field(&mut u, "Profile"));
    assert!(has(&u, "Profile     [1 Element    ☰]"));
    assert!(field(&mut u, "Profile"));
    assert!(has(&u, "Profile     [3 Elements   ☰]"));
    assert!(field(&mut u, "Profile"));
    assert!(has(&u, "Profile     [Select…       ]"));
    // Symmetric hides 2nd side; Cancel restores the opening values.
    assert!(press(&mut u, "□ Symetric"));
    assert!(!has(&u, "2nd side") && has(&u, "■ Symetric"));
    assert!(press(&mut u, "Cancel"));
    assert_eq!(u.status, ExtrudeStatus::Cancelled);
    assert!(u.values == crate::RevolveValues::default());
    assert!(u.compose_box().is_none() && !press_closed(&mut u));
}

/// A closed box ignores every id.
fn press_closed(u: &mut dyn ActionBoxUi) -> bool {
    (0..ID_SPAN).any(|k| u.activate_id(WidgetId::new(BASE + k)))
}

// ---- Loft ----

#[test]
fn loft_complete_boxes_match_the_reference() {
    assert_create_boxes(LOFT_TXT, |geometry, result| {
        let mut u = loft();
        u.values.geometry = geometry;
        u.values.result = result;
        rows(&u)
    });
}

#[test]
fn loft_variants() {
    let start = blocks(LOFT_TXT, "Start Type variants");
    let end = blocks(LOFT_TXT, "End Type variants");
    let guide = blocks(LOFT_TXT, "Guide variants");
    let path = blocks(LOFT_TXT, "Path variants");
    let connection = blocks(LOFT_TXT, "Connection variants");
    let merge = blocks(LOFT_TXT, "Merge with all variants");
    assert_eq!(
        (
            start.len(),
            end.len(),
            guide.len(),
            path.len(),
            connection.len(),
            merge.len()
        ),
        (4, 4, 2, 2, 2, 2)
    );
    let mut u = loft();
    for b in [&start[0], &end[0], &guide[0], &path[0], &connection[0]] {
        assert_block(&u, b);
    }
    u.values.start_type = true;
    u.values.end_type = true;
    for (i, kind) in [
        LoftEndType::Normal,
        LoftEndType::Tangent,
        LoftEndType::Match,
    ]
    .into_iter()
    .enumerate()
    {
        u.values.start_kind = kind;
        u.values.end_kind = kind;
        assert_block(&u, &start[1 + i]);
        assert_block(&u, &end[1 + i]);
        assert_framed(&u);
    }
    u.values.guide = true;
    u.values.path = true;
    u.values.connection = true;
    for b in [&guide[1], &path[1], &connection[1]] {
        assert_block(&u, b);
    }
    u.values.result = ExtrudeResultMode::Add;
    assert_block(&u, &merge[0]);
    u.values.merge_with_all = false;
    assert_block(&u, &merge[1]);
    assert_framed(&u);
}

#[test]
fn loft_interactions() {
    let mut u = loft();
    assert!(press(&mut u, "□ Start type"));
    assert!(has(&u, "◀ Normal  ▶") && has(&u, "Magnitude [0.0          ☰]"));
    // Cycling wraps both ways: Normal <- Match and Match -> Normal.
    assert!(arrow(&mut u, "Start type", '◀'));
    assert_eq!(u.values.start_kind, LoftEndType::Match);
    assert!(has(&u, "Entity    [Select…       ]") && !has(&u, "Magnitude"));
    assert!(arrow(&mut u, "Start type", '▶'));
    assert_eq!(u.values.start_kind, LoftEndType::Normal);
    // Start and End are independent.
    assert!(press(&mut u, "□ End type"));
    assert!(arrow(&mut u, "End type", '▶'));
    assert_eq!(
        (u.values.start_kind, u.values.end_kind),
        (LoftEndType::Normal, LoftEndType::Tangent)
    );
    // Guide / Path fields are live selection fields.
    assert!(press(&mut u, "□ Guide"));
    assert!(field(&mut u, "Guide"));
    assert!(has(&u, "■ Guide     [1 Element    ☰]"));
    assert!(press(&mut u, "□ Connection"));
    assert!(field(&mut u, "Start     ["));
    assert!(field(&mut u, "End       ["));
    assert_eq!(u.values.connection_start, MockSelection::One);
    assert_eq!(u.values.connection_end, MockSelection::One);
    assert!(press(&mut u, "OK"));
    assert_eq!(u.status, ExtrudeStatus::Confirmed);
    assert!(u.compose_box().is_none());
}

// ---- Sweep ----

#[test]
fn sweep_complete_boxes_match_the_reference() {
    assert_create_boxes(SWEEP_TXT, |geometry, result| {
        let mut u = sweep();
        u.values.geometry = geometry;
        u.values.result = result;
        rows(&u)
    });
}

#[test]
fn sweep_variants() {
    let full = blocks(SWEEP_TXT, "Full path variants");
    let keep = blocks(SWEEP_TXT, "Keep orientation variants");
    let twist = blocks(SWEEP_TXT, "Twist variants");
    let scale = blocks(SWEEP_TXT, "Scale variants");
    let merge = blocks(SWEEP_TXT, "Merge with all variants");
    assert_eq!(
        (
            full.len(),
            keep.len(),
            twist.len(),
            scale.len(),
            merge.len()
        ),
        (2, 2, 2, 2, 2)
    );
    let mut u = sweep();
    for b in [&full[0], &keep[0], &scale[0]] {
        assert_block(&u, b);
    }
    u.values.full_path = true;
    u.values.scale = true;
    assert_block(&u, &full[1]);
    assert_block(&u, &scale[1]);
    u.values.keep_orientation = true;
    assert_block(&u, &keep[1]);
    assert_block(&u, &twist[0]);
    u.values.twist = true;
    assert_block(&u, &twist[1]);
    u.values.result = ExtrudeResultMode::Remove;
    assert_block(&u, &merge[0]);
    u.values.merge_with_all = false;
    assert_block(&u, &merge[1]);
    assert_framed(&u);
}

#[test]
fn sweep_interactions() {
    let mut u = sweep();
    assert!(!has(&u, "Twist"));
    assert!(press(&mut u, "□ Keep orientation"));
    assert!(press(&mut u, "□ Twist"));
    assert!(has(&u, "■ Twist [+|0.0 deg      ☰]"));
    // Closing `Keep orientation` hides `Twist` but keeps its state.
    assert!(press(&mut u, "■ Keep orientation"));
    assert!(!has(&u, "Twist") && u.values.twist);
    assert!(press(&mut u, "□ Scale"));
    assert!(has(&u, "■ Scale     [0.0          ☰]"));
    assert!(press(&mut u, "□ Full path"));
    assert!(field(&mut u, "Path"));
    assert!(has(&u, "Path        [1 Element    ☰]"));
    assert!(press(&mut u, "Cancel"));
    assert!(u.values == crate::SweepValues::default());
}

// ---- Boolean ----

#[test]
fn boolean_complete_boxes_match_the_reference() {
    let boxes = base_boxes(BOOLEAN_TXT);
    assert_eq!(boxes.len(), 3);
    for (i, operation) in [
        BooleanOperation::Add,
        BooleanOperation::Cut,
        BooleanOperation::Inter,
    ]
    .into_iter()
    .enumerate()
    {
        let mut u = boolean();
        u.values.operation = operation;
        assert_eq!(rows(&u), boxes[i], "{operation:?}");
        assert_framed(&u);
    }
}

#[test]
fn boolean_footer_is_the_canonical_31_cell_row() {
    for operation in [
        BooleanOperation::Add,
        BooleanOperation::Cut,
        BooleanOperation::Inter,
    ] {
        let mut u = boolean();
        u.values.operation = operation;
        let r = rows(&u);
        assert_eq!(r[r.len() - 2], "│      OK      │    Cancel    │");
        assert_eq!(r[r.len() - 2].chars().count(), 31);
    }
}

#[test]
fn boolean_offset_and_keep_tools_variants() {
    let offset = blocks(BOOLEAN_TXT, "Offset variants");
    let all = blocks(BOOLEAN_TXT, "Offset All variants");
    let keep = blocks(BOOLEAN_TXT, "Keep tools variants");
    assert_eq!((offset.len(), all.len(), keep.len()), (2, 2, 1));
    let mut u = boolean();
    u.values.operation = BooleanOperation::Cut;
    assert_block(&u, &offset[0]);
    u.values.offset = true;
    assert_block(&u, &offset[1]);
    assert_block(&u, &all[0]);
    u.values.offset_all = false;
    // README_UI §16.8 outranks the older Offset All snippet in the .txt file.
    // Faces owns the last-child connector; Dist retains its documented indentation.
    assert_block(
        &u,
        &[
            "│ ╰╴□ Offset all              │".to_owned(),
            "│   ╰╴Faces   [none          ]│".to_owned(),
            "│     Dist  [+|0.0 mm       ☰]│".to_owned(),
        ],
    );
    assert_framed(&u);
    assert_block(&u, &keep[0][..1]);
    u.values.keep_tools = true;
    assert_block(&u, &keep[0][1..]);
}

#[test]
fn boolean_offset_is_cut_only_and_survives_switching() {
    let mut u = boolean();
    u.values.offset = true;
    for operation in [BooleanOperation::Add, BooleanOperation::Inter] {
        u.values.operation = operation;
        assert!(!has(&u, "Offset"), "{operation:?}");
    }
    u.values.operation = BooleanOperation::Cut;
    assert!(has(&u, "■ Offset") && has(&u, "Offset all"));
}

#[test]
fn boolean_interactions() {
    let mut u = boolean();
    assert!(press(&mut u, "Cut"));
    assert_eq!(u.values.operation, BooleanOperation::Cut);
    assert!(press(&mut u, "□ Offset"));
    assert!(press(&mut u, "■ Offset all"));
    assert!(has(&u, "Faces   [none          ]"));
    // `Faces` is a live selection field: `none` becomes `1 Element`.
    assert!(field(&mut u, "Faces"));
    assert!(has(&u, "Faces   [1 Element    ☰]"));
    assert!(press(&mut u, "Inter"));
    assert!(press(&mut u, "Add"));
    assert_eq!(u.values.operation, BooleanOperation::Add);
    assert!(press(&mut u, "□ Keep tools"));
    assert!(has(&u, "■ Keep tools"));
    assert!(field(&mut u, "Target"));
    assert!(field(&mut u, "Tools"));
    assert_eq!(
        (u.values.target, u.values.tools),
        (MockSelection::One, MockSelection::One)
    );
    assert!(press(&mut u, "OK"));
    assert!(!u.is_open() && u.compose_box().is_none());
}

// ---- shared behaviour ----

/// Switching `Solid | Surface` and `New | Add | Remove` keeps every other value.
#[test]
fn header_changes_preserve_state() {
    let mut u = revolve();
    u.values.full = false;
    u.values.start_offset = true;
    u.values.axis = MockSelection::Many;
    let before = u.values;
    for needle in ["Surface", "Add", "Remove", "New", "Solid"] {
        assert!(press(&mut u, needle), "{needle}");
    }
    assert_eq!(
        (u.values.geometry, u.values.result),
        (ExtrudeGeometryType::Solid, ExtrudeResultMode::New)
    );
    assert_eq!(
        (u.values.full, u.values.start_offset, u.values.axis),
        (before.full, before.start_offset, before.axis)
    );
}

#[test]
fn ids_are_stable_across_conditional_rows() {
    // The id of OK / Cancel / Profile never depends on which rows are shown.
    let mut u = revolve();
    let ids = |u: &RevolveUi| {
        [
            field_id(u, "Profile").get(),
            field_id(u, "Axis").get(),
            id_of(u, "OK").get(),
            id_of(u, "Cancel").get(),
        ]
    };
    let before = ids(&u);
    u.values.full = false;
    u.values.start_offset = true;
    u.values.result = ExtrudeResultMode::Add;
    assert_eq!(ids(&u), before);
    // All ids lie in the shared range, and no two controls of one box collide.
    let g = grid_of(&u).unwrap();
    let mut seen = HashSet::new();
    for r in 0..g.rows() {
        for c in 0..g.cols() {
            if let Some(id) = g.get(c, r).and_then(|x| x.widget_id) {
                assert!(id.get() >= BASE && id.get() < BASE + ID_SPAN);
                seen.insert(id);
            }
        }
    }
    assert!(seen.len() > 10);
}

#[test]
fn hit_testing_maps_a_pixel_to_the_control_in_o1() {
    let u = boolean();
    let g = grid_of(&u).unwrap();
    let metrics = GridMetrics::new(8, 18).unwrap();
    let (col, row) = locate(&u, "Cut");
    let hit = hit_test(
        UiPoint {
            x: col * 8 + 3,
            y: row * 18 + 9,
        },
        metrics,
        &g,
    )
    .unwrap();
    assert_eq!(hit.widget_id, id_at(&u, col, row));
    assert!(hit.widget_id.is_some());
}

#[test]
fn closed_boxes_ignore_every_id_and_reopen_with_defaults() {
    let mut u = loft();
    assert!(press(&mut u, "OK"));
    assert!(!press_closed(&mut u));
    u.reopen();
    assert!(u.is_open() && u.values == crate::LoftValues::default());
}

#[test]
fn selection_and_formula_buttons_are_classified_per_box() {
    let mut u = sweep();
    u.values.scale = true;
    u.values.keep_orientation = true;
    u.values.twist = true;
    // `☰]` of the twist and scale fields are formula buttons; the fields are not.
    let (tc, tr) = locate(&u, "■ Twist");
    let line = rows(&u)[tr as usize].clone();
    let burger = u32::try_from(line.chars().position(|c| c == '☰').unwrap()).unwrap();
    assert!(u.is_formula_button(id_at(&u, burger, tr).unwrap()));
    assert!(!u.is_formula_button(id_at(&u, tc + 8, tr).unwrap()));
    assert!(!u.is_formula_button(field_id(&u, "Profile")));
    assert!(u.is_selection_field(field_id(&u, "Profile")));
    assert!(!u.is_selection_field(id_of(&u, "OK")));
}
