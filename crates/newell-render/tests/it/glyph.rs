// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Test helpers call `expect` for clear failure messages. The `clippy.toml` allowance only covers `#[test]` bodies.
#![allow(clippy::expect_used)]

use ab_glyph::{Font, PxScale, point};
use newell_render::{CellLayout, GlyphAtlas, canonical_glyphs, load_font};
use newell_ui::{DEFAULT_K, GridMetrics};

const FONT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/FiraCode-Regular.ttf"
);
const UI_DESIGN_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/ui_design");

/// Every permitted `k` (2…7: Full HD 100 % is 2, Retina 100 % is 5).
const KS: [u32; 6] = [2, 3, 4, 5, 6, 7];

/// Coverage at or above this value counts as ink.
const ON: u8 = 128;

/// Arms as (top, right, bottom, left).
type Arms = (char, [bool; 4]);

const SINGLE: [Arms; 7] = [
    ('─', [false, true, false, true]),
    ('│', [true, false, true, false]),
    ('┌', [false, true, true, false]),
    ('┐', [false, false, true, true]),
    ('└', [true, true, false, false]),
    ('┘', [true, false, false, true]),
    ('┼', [true, true, true, true]),
];

const DOUBLE: [Arms; 11] = [
    ('═', [false, true, false, true]),
    ('║', [true, false, true, false]),
    ('╔', [false, true, true, false]),
    ('╗', [false, false, true, true]),
    ('╚', [true, true, false, false]),
    ('╝', [true, false, false, true]),
    ('╬', [true, true, true, true]),
    ('╠', [true, true, true, false]),
    ('╣', [true, false, true, true]),
    ('╦', [false, true, true, true]),
    ('╩', [true, true, false, true]),
];

fn font_bytes() -> Vec<u8> {
    std::fs::read(FONT_PATH).expect("assets/FiraCode-Regular.ttf must exist")
}

fn metrics(k: u32) -> GridMetrics {
    GridMetrics::from_k(k).expect("k is valid")
}

fn build_atlas(k: u32) -> GlyphAtlas {
    let font = load_font(font_bytes()).expect("Fira Code must parse");
    GlyphAtlas::build(&font, metrics(k)).expect("every canonical glyph must rasterise")
}

fn cell_w(atlas: &GlyphAtlas) -> u32 {
    atlas.layout().cell_width
}

fn cell_h(atlas: &GlyphAtlas) -> u32 {
    atlas.layout().cell_height
}

fn pixel(atlas: &GlyphAtlas, ch: char, x: u32, y: u32) -> u8 {
    let rect = atlas.get(ch).expect("glyph is in the atlas").rect;
    atlas.coverage()[((rect.y + y) * atlas.width() + rect.x + x) as usize]
}

fn ink_rows_at_column(atlas: &GlyphAtlas, ch: char, x: u32) -> Vec<u32> {
    (0..cell_h(atlas))
        .filter(|&y| pixel(atlas, ch, x, y) >= ON)
        .collect()
}

fn ink_columns_at_row(atlas: &GlyphAtlas, ch: char, y: u32) -> Vec<u32> {
    (0..cell_w(atlas))
        .filter(|&x| pixel(atlas, ch, x, y) >= ON)
        .collect()
}

/// Number of contiguous ink runs in a sorted index list.
fn runs(indices: &[u32]) -> usize {
    if indices.is_empty() {
        return 0;
    }
    1 + indices
        .windows(2)
        .filter(|pair| pair[1] != pair[0] + 1)
        .count()
}

/// Ink on each cell edge.
struct Edges {
    top: Vec<u32>,
    right: Vec<u32>,
    bottom: Vec<u32>,
    left: Vec<u32>,
}

fn edges(atlas: &GlyphAtlas, ch: char) -> Edges {
    Edges {
        top: ink_columns_at_row(atlas, ch, 0),
        right: ink_rows_at_column(atlas, ch, cell_w(atlas) - 1),
        bottom: ink_columns_at_row(atlas, ch, cell_h(atlas) - 1),
        left: ink_rows_at_column(atlas, ch, 0),
    }
}

fn is_box_or_block(ch: char) -> bool {
    matches!(ch, '\u{2500}'..='\u{259F}')
}

/// Inclusive `(first, last)` columns holding any ink, or `None` for a blank glyph.
fn ink_columns(atlas: &GlyphAtlas, ch: char) -> Option<(u32, u32)> {
    let ink = |x| (0..cell_h(atlas)).any(|y| pixel(atlas, ch, x, y) > 0);
    let first = (0..cell_w(atlas)).find(|&x| ink(x))?;
    let last = (0..cell_w(atlas)).rev().find(|&x| ink(x))?;
    Some((first, last))
}

/// The characters of every `docs/ui_design/*.txt` outside ASCII.
fn reference_symbols() -> std::collections::BTreeSet<char> {
    let mut scanned = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(UI_DESIGN_DIR).expect("docs/ui_design must exist") {
        let path = entry.expect("readable directory entry").path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("txt") {
            let text = std::fs::read_to_string(&path).expect("reference file is UTF-8");
            scanned.extend(text.chars().filter(|ch| !ch.is_ascii()));
        }
    }
    scanned
}

#[test]
fn font_asset_loads() {
    assert!(load_font(font_bytes()).is_ok());
}

#[test]
fn invalid_font_bytes_are_rejected() {
    assert!(load_font(vec![0, 1, 2, 3]).is_err());
}

#[test]
fn layout_matches_the_architecture_table() {
    // (k, W, H, L, top, baseline) from ARCHITECTURE.md §3.1.
    let table = [
        (2, 8, 18, 16, 1, 13),
        (3, 12, 27, 24, 1, 19),
        (4, 16, 36, 32, 2, 26),
        (5, 20, 45, 40, 2, 32),
        (6, 24, 54, 48, 3, 39),
        (7, 28, 63, 56, 3, 45),
    ];
    let font = load_font(font_bytes()).expect("Fira Code must parse");
    for (k, w, h, l, top, baseline) in table {
        let layout = CellLayout::new(&font, metrics(k)).expect("line box fits");
        assert_eq!((layout.cell_width, layout.cell_height), (w, h), "k = {k}");
        assert_eq!(layout.line_box, l, "k = {k}");
        assert_eq!(layout.top, top, "k = {k}");
        assert_eq!(layout.baseline, baseline, "k = {k}");
        assert!((layout.scale - k as f32 / 300.0).abs() < 1e-6, "k = {k}");
    }
}

#[test]
fn fira_code_has_its_natural_one_to_two_proportions() {
    // advance 1200, ascender 1800, descender -600, line gap 0, upem 1950.
    let font = load_font(font_bytes()).expect("Fira Code must parse");
    assert_eq!(font.h_advance_unscaled(font.glyph_id('0')), 1200.0);
    assert_eq!(font.ascent_unscaled(), 1800.0);
    assert_eq!(font.descent_unscaled(), -600.0);
    assert_eq!(font.line_gap_unscaled(), 0.0);
}

#[test]
fn a_line_box_taller_than_the_cell_is_refused() {
    let font = load_font(font_bytes()).expect("Fira Code must parse");
    // A 1:2 cell holds the line box; a squat 2:1 cell is too short for it.
    let cell = GridMetrics::new(10, 20).expect("non-zero");
    assert!(CellLayout::new(&font, cell).is_ok());
    let squat = GridMetrics::new(40, 20).expect("non-zero");
    assert!(CellLayout::new(&font, squat).is_err());
}

#[test]
fn glyphs_use_one_uniform_scale_at_the_shared_baseline() {
    // Independent reference: ab_glyph at the same scale on both axes (8k px for the 2400-unit
    // height, i.e. s = k / 300), pen origin at x = 0, baseline from the architecture table.
    let font = load_font(font_bytes()).expect("Fira Code must parse");
    for k in KS {
        let atlas = build_atlas(k);
        let baseline = atlas.layout().baseline as f32;
        let (w, h) = (cell_w(&atlas), cell_h(&atlas));
        for ch in ['A', 'g', 'M', 'W', '@', '0', '?', '…', '◐', '─', '█'] {
            let glyph = font
                .glyph_id(ch)
                .with_scale_and_position(PxScale::from(8.0 * k as f32), point(0.0, baseline));
            let mut expected = vec![0u8; (w * h) as usize];
            if let Some(outlined) = font.outline_glyph(glyph) {
                let origin = outlined.px_bounds().min;
                outlined.draw(|x, y, c| {
                    let (px, py) = (origin.x as i32 + x as i32, origin.y as i32 + y as i32);
                    if (0..w as i32).contains(&px) && (0..h as i32).contains(&py) {
                        expected[(py as u32 * w + px as u32) as usize] =
                            (c.clamp(0.0, 1.0) * 255.0).round() as u8;
                    }
                });
            }
            let rect = atlas.get(ch).expect("glyph is in the atlas").rect;
            for y in 0..h {
                for x in 0..w {
                    assert_eq!(
                        pixel(&atlas, ch, x, y),
                        expected[(y * w + x) as usize],
                        "k = {k}, {ch:?} at ({x}, {y}), slot {rect:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn representative_glyph_rasterises() {
    for k in KS {
        let atlas = build_atlas(k);
        assert!(ink_columns(&atlas, 'A').is_some(), "k = {k}");
        let peak = (0..cell_h(&atlas))
            .flat_map(|y| (0..cell_w(&atlas)).map(move |x| (x, y)))
            .map(|(x, y)| pixel(&atlas, 'A', x, y))
            .max();
        assert!(peak.is_some_and(|peak| peak >= ON), "k = {k}");
    }
}

#[test]
fn canonical_set_is_sorted_and_unique() {
    let glyphs = canonical_glyphs();
    assert!(glyphs.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn canonical_set_is_printable_ascii_plus_ui_symbols() {
    let glyphs = canonical_glyphs();
    assert!((0x20u8..=0x7E).all(|byte| glyphs.contains(&char::from(byte))));
    // 95 printable ASCII plus 83 reference symbols, including Ø, à, ╸ and ╺.
    assert_eq!(glyphs.len(), 178);
    let required: std::collections::BTreeSet<char> = (0x20u8..=0x7E)
        .map(char::from)
        .chain(reference_symbols())
        .chain(std::iter::once('▏'))
        .collect();
    assert_eq!(glyphs, required.into_iter().collect::<Vec<_>>());
}

#[test]
fn canonical_set_covers_every_ui_reference_symbol() {
    let glyphs = canonical_glyphs();
    for entry in std::fs::read_dir(UI_DESIGN_DIR).expect("docs/ui_design must exist") {
        let path = entry.expect("readable directory entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("txt") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("reference file is UTF-8");
        for ch in text.chars().filter(|ch| !ch.is_ascii()) {
            assert!(
                glyphs.contains(&ch),
                "{ch:?} from {path:?} is not in the glyph set"
            );
        }
    }
}

#[test]
fn required_set_has_font_coverage() {
    let font = load_font(font_bytes()).expect("Fira Code must parse");
    for ch in canonical_glyphs() {
        assert_ne!(font.glyph_id(ch).0, 0, "font lacks {ch:?}");
    }
}

#[test]
fn every_required_glyph_is_in_the_atlas_for_every_k() {
    for k in KS {
        let atlas = build_atlas(k);
        for ch in canonical_glyphs() {
            assert!(atlas.get(ch).is_some(), "k = {k}: {ch:?}");
        }
        for ch in reference_symbols() {
            assert!(atlas.get(ch).is_some(), "k = {k}: {ch:?}");
        }
        for ch in ['…', '◐', '◑', '▏', '╻', '╹', '×', '?'] {
            assert!(ink_columns(&atlas, ch).is_some(), "k = {k}: {ch:?} has ink");
        }
    }
}

#[test]
fn characters_outside_the_required_set_have_no_slot() {
    // `é` is in the font but not in the required set: the atlas has no slot, so the cell draws `?`.
    let font = load_font(font_bytes()).expect("Fira Code must parse");
    assert_ne!(font.glyph_id('é').0, 0);
    let atlas = build_atlas(DEFAULT_K);
    assert!(atlas.get('é').is_none());
    assert!(atlas.get('\u{1F600}').is_none());
}

#[test]
fn atlas_is_power_of_two_and_fully_sized() {
    for k in KS {
        let atlas = build_atlas(k);
        assert!(atlas.width().is_power_of_two(), "k = {k}");
        assert!(atlas.height().is_power_of_two(), "k = {k}");
        assert_eq!(
            atlas.coverage().len(),
            (atlas.width() * atlas.height()) as usize
        );
    }
}

#[test]
fn atlas_places_glyphs_in_canonical_order() {
    const COLUMNS: u32 = 16;
    for k in KS {
        let atlas = build_atlas(k);
        let (w, h) = (cell_w(&atlas), cell_h(&atlas));
        let glyphs = canonical_glyphs();
        assert_eq!(atlas.glyphs().len(), glyphs.len());
        for (index, (info, ch)) in atlas.glyphs().iter().zip(glyphs).enumerate() {
            let index = index as u32;
            assert_eq!(info.ch, ch);
            assert_eq!(info.rect.x, (index % COLUMNS) * w);
            assert_eq!(info.rect.y, (index / COLUMNS) * h);
            assert_eq!((info.rect.width, info.rect.height), (w, h));
            assert!(info.rect.x + info.rect.width <= atlas.width());
            assert!(info.rect.y + info.rect.height <= atlas.height());
        }
    }
}

#[test]
fn atlas_build_is_deterministic() {
    for k in KS {
        assert_eq!(build_atlas(k), build_atlas(k), "k = {k}");
    }
}

#[test]
fn default_cell_is_twenty_by_forty_five() {
    let atlas = build_atlas(DEFAULT_K);
    assert_eq!((cell_w(&atlas), cell_h(&atlas)), (20, 45));
    assert_eq!(cell_w(&atlas) * 9, cell_h(&atlas) * 4);
}

#[test]
fn atlas_dimensions_cover_exactly_the_packed_region() {
    let atlas = build_atlas(DEFAULT_K);
    // 178 glyphs in 16 columns: 12 rows of 20 x 45 slots, 320 x 540 used, rounded up to 512 x 1024.
    assert_eq!((atlas.width(), atlas.height()), (512, 1024));
    let used_width = 16 * cell_w(&atlas);
    let used_height = 12 * cell_h(&atlas);
    for y in 0..atlas.height() {
        for x in 0..atlas.width() {
            if x >= used_width || y >= used_height {
                assert_eq!(atlas.coverage()[(y * atlas.width() + x) as usize], 0);
            }
        }
    }
}

#[test]
fn space_is_a_present_blank_glyph() {
    let atlas = build_atlas(DEFAULT_K);
    assert!(atlas.get(' ').is_some());
    let blank = (0..cell_h(&atlas))
        .flat_map(|y| (0..cell_w(&atlas)).map(move |x| (x, y)))
        .all(|(x, y)| pixel(&atlas, ' ', x, y) == 0);
    assert!(blank);
}

#[test]
fn no_text_glyph_is_clipped() {
    // Only box drawing and block elements may overshoot the cell (removed by clipping).
    for k in KS {
        let atlas = build_atlas(k);
        for info in atlas.glyphs() {
            if !is_box_or_block(info.ch) {
                assert!(!info.clipped, "k = {k}: {:?} is clipped", info.ch);
            }
        }
    }
}

#[test]
fn wide_text_glyphs_stay_inside_the_cell() {
    for k in KS {
        let atlas = build_atlas(k);
        for ch in "MWmw@O%&Q…".chars() {
            let (first, last) = ink_columns(&atlas, ch).expect("glyph has ink");
            assert!(last < cell_w(&atlas) && last >= first, "k = {k}: {ch:?}");
        }
    }
}

#[test]
fn box_drawing_arms_reach_cell_edges() {
    for k in KS {
        let atlas = build_atlas(k);
        for (arms_table, strokes) in [(SINGLE.as_slice(), 1), (DOUBLE.as_slice(), 2)] {
            for &(ch, [top, right, bottom, left]) in arms_table {
                let found = edges(&atlas, ch);
                for (arm, span) in [
                    (top, &found.top),
                    (right, &found.right),
                    (bottom, &found.bottom),
                    (left, &found.left),
                ] {
                    assert_eq!(!span.is_empty(), arm, "k = {k}: {ch:?} arm presence");
                    if arm {
                        assert_eq!(runs(span), strokes, "k = {k}: {ch:?} stroke count at edge");
                    }
                }
            }
        }
    }
}

#[test]
fn box_drawing_neighbours_join_across_cell_edges() {
    for k in KS {
        let atlas = build_atlas(k);
        for arms_table in [SINGLE.as_slice(), DOUBLE.as_slice()] {
            for &(a, [_, a_right, a_bottom, _]) in arms_table {
                for &(b, [b_top, _, _, b_left]) in arms_table {
                    let a_edges = edges(&atlas, a);
                    let b_edges = edges(&atlas, b);
                    if a_right && b_left {
                        assert_eq!(
                            a_edges.right, b_edges.left,
                            "k = {k}: {a:?} right into {b:?} left"
                        );
                    }
                    if a_bottom && b_top {
                        assert_eq!(
                            a_edges.bottom, b_edges.top,
                            "k = {k}: {a:?} bottom into {b:?} top"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn straight_box_lines_are_continuous_over_five_cells() {
    // Five identical cells side by side (and stacked) leave no gap in a straight stroke.
    for k in KS {
        let atlas = build_atlas(k);
        for (ch, double) in [('─', false), ('═', true)] {
            let left = ink_rows_at_column(&atlas, ch, 0);
            let right = ink_rows_at_column(&atlas, ch, cell_w(&atlas) - 1);
            assert!(!left.is_empty(), "k = {k}: {ch:?}");
            assert_eq!(left, right, "k = {k}: {ch:?} rows match across the join");
            assert_eq!(runs(&left), if double { 2 } else { 1 });
        }
        for (ch, double) in [('│', false), ('║', true)] {
            let top = ink_columns_at_row(&atlas, ch, 0);
            let bottom = ink_columns_at_row(&atlas, ch, cell_h(&atlas) - 1);
            assert!(!top.is_empty(), "k = {k}: {ch:?}");
            assert_eq!(top, bottom, "k = {k}: {ch:?} columns match across the join");
            assert_eq!(runs(&top), if double { 2 } else { 1 });
        }
    }
}

#[test]
fn box_drawing_strokes_are_centred_in_the_cell() {
    for k in KS {
        let atlas = build_atlas(k);
        let horizontal = ink_rows_at_column(&atlas, '─', cell_w(&atlas) / 2);
        let vertical = ink_columns_at_row(&atlas, '│', cell_h(&atlas) / 2);
        let centre_row = i64::from(cell_h(&atlas) / 2);
        let centre_column = i64::from(cell_w(&atlas) / 2);
        let row_mid =
            horizontal.iter().map(|&row| i64::from(row)).sum::<i64>() / horizontal.len() as i64;
        let column_mid =
            vertical.iter().map(|&col| i64::from(col)).sum::<i64>() / vertical.len() as i64;
        let tolerance = i64::from(k) / 2 + 1;
        assert!(
            (row_mid - centre_row).abs() <= tolerance,
            "k = {k}: horizontal stroke at row {row_mid}"
        );
        assert!(
            (column_mid - centre_column).abs() <= tolerance,
            "k = {k}: vertical stroke at column {column_mid}"
        );
    }
}

#[test]
fn block_elements_tile_horizontally_and_cover_the_line_box() {
    for k in KS {
        let atlas = build_atlas(k);
        let layout = atlas.layout();
        // `█` has ink in every column, so five cells in a row show no gap.
        for x in 0..cell_w(&atlas) {
            assert!(
                (0..cell_h(&atlas)).any(|y| pixel(&atlas, '█', x, y) > 0),
                "k = {k}: █ column {x} is empty"
            );
        }
        // The shades are the font's own dither patterns: they span the whole advance, with
        // ink within k / 2 pixels of each vertical cell edge (the dots scale with k).
        for ch in ['░', '▒', '▓'] {
            let (first, last) = ink_columns(&atlas, ch).expect("shade has ink");
            assert!(first <= k / 2, "k = {k}: {ch:?} starts at column {first}");
            assert!(
                last + k / 2 + 1 >= cell_w(&atlas),
                "k = {k}: {ch:?} ends at column {last}"
            );
        }
        // `█` is solid over exactly the line box L and empty elsewhere.
        for y in 0..cell_h(&atlas) {
            let inside = (layout.top..layout.top + layout.line_box).contains(&y);
            for x in 0..cell_w(&atlas) {
                let expected = if inside { 255 } else { 0 };
                assert_eq!(
                    pixel(&atlas, '█', x, y),
                    expected,
                    "k = {k}: █ at ({x}, {y})"
                );
            }
        }
    }
}
