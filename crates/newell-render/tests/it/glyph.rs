// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Test helpers call `expect` for clear failure messages. The `clippy.toml` allowance only covers `#[test]` bodies.
#![allow(clippy::expect_used)]

use newell_render::{GLYPH_CELL_HEIGHT, GLYPH_CELL_WIDTH, GlyphAtlas, canonical_glyphs, load_font};

const FONT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/FiraCode-Regular.ttf"
);
const UI_DESIGN_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/ui_design");

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

fn build_atlas() -> GlyphAtlas {
    let font = load_font(font_bytes()).expect("Fira Code must parse");
    GlyphAtlas::build(&font).expect("every canonical glyph must rasterise")
}

fn pixel(atlas: &GlyphAtlas, ch: char, x: u32, y: u32) -> u8 {
    let rect = atlas.get(ch).expect("glyph is in the atlas").rect;
    atlas.coverage()[((rect.y + y) * atlas.width() + rect.x + x) as usize]
}

fn ink_rows_at_column(atlas: &GlyphAtlas, ch: char, x: u32) -> Vec<u32> {
    (0..GLYPH_CELL_HEIGHT)
        .filter(|&y| pixel(atlas, ch, x, y) >= ON)
        .collect()
}

fn ink_columns_at_row(atlas: &GlyphAtlas, ch: char, y: u32) -> Vec<u32> {
    (0..GLYPH_CELL_WIDTH)
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
    let last_x = GLYPH_CELL_WIDTH - 1;
    let last_y = GLYPH_CELL_HEIGHT - 1;
    Edges {
        top: ink_columns_at_row(atlas, ch, 0),
        right: ink_rows_at_column(atlas, ch, last_x),
        bottom: ink_columns_at_row(atlas, ch, last_y),
        left: ink_rows_at_column(atlas, ch, 0),
    }
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
fn representative_glyph_rasterises() {
    let atlas = build_atlas();
    let peak = (0..GLYPH_CELL_HEIGHT)
        .flat_map(|y| (0..GLYPH_CELL_WIDTH).map(move |x| (x, y)))
        .map(|(x, y)| pixel(&atlas, 'A', x, y))
        .max();
    assert!(peak.is_some_and(|peak| peak >= ON));
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
    // 95 printable ASCII plus 72 UI symbols.
    assert_eq!(glyphs.len(), 167);
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
fn atlas_is_power_of_two_and_fully_sized() {
    let atlas = build_atlas();
    assert!(atlas.width().is_power_of_two());
    assert!(atlas.height().is_power_of_two());
    assert_eq!(
        atlas.coverage().len(),
        (atlas.width() * atlas.height()) as usize
    );
}

#[test]
fn atlas_places_glyphs_in_canonical_order() {
    const COLUMNS: u32 = 16;
    let atlas = build_atlas();
    let glyphs = canonical_glyphs();
    assert_eq!(atlas.glyphs().len(), glyphs.len());
    for (index, (info, ch)) in atlas.glyphs().iter().zip(glyphs).enumerate() {
        let index = index as u32;
        assert_eq!(info.ch, ch);
        assert_eq!(info.rect.x, (index % COLUMNS) * GLYPH_CELL_WIDTH);
        assert_eq!(info.rect.y, (index / COLUMNS) * GLYPH_CELL_HEIGHT);
        assert_eq!(info.rect.width, GLYPH_CELL_WIDTH);
        assert_eq!(info.rect.height, GLYPH_CELL_HEIGHT);
        assert!(info.rect.x + info.rect.width <= atlas.width());
        assert!(info.rect.y + info.rect.height <= atlas.height());
    }
}

#[test]
fn atlas_build_is_deterministic() {
    assert_eq!(build_atlas(), build_atlas());
}

#[test]
fn raster_cell_is_three_by_five() {
    assert_eq!(GLYPH_CELL_WIDTH * 5, GLYPH_CELL_HEIGHT * 3);
}

#[test]
fn box_drawing_arms_reach_cell_edges() {
    let atlas = build_atlas();
    for (arms_table, strokes) in [(SINGLE.as_slice(), 1), (DOUBLE.as_slice(), 2)] {
        for &(ch, [top, right, bottom, left]) in arms_table {
            let found = edges(&atlas, ch);
            for (arm, span) in [
                (top, &found.top),
                (right, &found.right),
                (bottom, &found.bottom),
                (left, &found.left),
            ] {
                assert_eq!(!span.is_empty(), arm, "{ch:?} arm presence");
                if arm {
                    assert_eq!(runs(span), strokes, "{ch:?} stroke count at edge");
                }
            }
        }
    }
}

#[test]
fn box_drawing_neighbours_join_across_cell_edges() {
    let atlas = build_atlas();
    for arms_table in [SINGLE.as_slice(), DOUBLE.as_slice()] {
        for &(a, [_, a_right, a_bottom, _]) in arms_table {
            for &(b, [b_top, _, _, b_left]) in arms_table {
                let a_edges = edges(&atlas, a);
                let b_edges = edges(&atlas, b);
                if a_right && b_left {
                    assert_eq!(a_edges.right, b_edges.left, "{a:?} right into {b:?} left");
                }
                if a_bottom && b_top {
                    assert_eq!(a_edges.bottom, b_edges.top, "{a:?} bottom into {b:?} top");
                }
            }
        }
    }
}

#[test]
fn box_drawing_strokes_are_centred_in_the_cell() {
    let atlas = build_atlas();
    let horizontal = ink_rows_at_column(&atlas, '─', GLYPH_CELL_WIDTH / 2);
    let vertical = ink_columns_at_row(&atlas, '│', GLYPH_CELL_HEIGHT / 2);
    let centre_row = (GLYPH_CELL_HEIGHT / 2) as i64;
    let centre_column = (GLYPH_CELL_WIDTH / 2) as i64;
    let row_mid = horizontal.iter().map(|&row| row as i64).sum::<i64>() / horizontal.len() as i64;
    let column_mid = vertical.iter().map(|&col| col as i64).sum::<i64>() / vertical.len() as i64;
    assert!(
        (row_mid - centre_row).abs() <= 2,
        "horizontal stroke at row {row_mid}"
    );
    assert!(
        (column_mid - centre_column).abs() <= 2,
        "vertical stroke at column {column_mid}"
    );
}

#[test]
fn atlas_dimensions_cover_exactly_the_packed_region() {
    let atlas = build_atlas();
    // 167 glyphs in 16 columns: 11 rows of 24 x 40 slots, 384 x 440 used, rounded up to 512 x 512.
    assert_eq!((atlas.width(), atlas.height()), (512, 512));
    let used_width = 16 * GLYPH_CELL_WIDTH;
    let used_height = 11 * GLYPH_CELL_HEIGHT;
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
    let atlas = build_atlas();
    assert!(atlas.get(' ').is_some());
    let blank = (0..GLYPH_CELL_HEIGHT)
        .flat_map(|y| (0..GLYPH_CELL_WIDTH).map(move |x| (x, y)))
        .all(|(x, y)| pixel(&atlas, ' ', x, y) == 0);
    assert!(blank);
}
