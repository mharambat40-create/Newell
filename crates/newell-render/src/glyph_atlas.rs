// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! CPU-side Fira Code rasterisation and the deterministic glyph atlas.
//!
//! Every glyph is rasterised into one raster cell of [`GLYPH_CELL_WIDTH`] ×
//! [`GLYPH_CELL_HEIGHT`] pixels, which is the 3:5 grid cell (`ARCHITECTURE.md`
//! §3.1). The horizontal scale maps the monospace advance onto the cell width,
//! and the vertical scale maps the font's ascender-to-descender height onto the
//! cell height. Every glyph shares one baseline, so box-drawing strokes meet at
//! cell edges. The atlas is a single-channel coverage texture laid out in
//! [`canonical_glyphs`] order.

use ab_glyph::{Font, FontVec, PxScale, ScaleFont, point};

use crate::RenderError;

/// Raster width of one grid cell in pixels (3 of the 3:5 ratio).
pub const GLYPH_CELL_WIDTH: u32 = 24;
/// Raster height of one grid cell in pixels (5 of the 3:5 ratio).
pub const GLYPH_CELL_HEIGHT: u32 = 40;

/// Glyphs per atlas row. Fixed so the atlas layout does not depend on the glyph count.
const ATLAS_COLUMNS: u32 = 16;

/// Non-ASCII glyphs that appear in `docs/ui_design/*.txt` (the `Special_ASCII.txt` palette included).
/// Prose-only characters in `docs/ui_design/README.md` are excluded.
const UI_SYMBOLS: &str = "─━│┃┄┆┈┊┌┏┐┓└┗┘┛├┣┤┫┬┳┴┻┼╋═║╒╓╔╕╖╗╘╙╚╛╜╝╞╟╠╡╢╣╤╥╦╧╩╪╬╭╮╯╰╴╵╶╷█░▒▓■□▲▶▼◀☰";

/// The canonical glyph set: printable ASCII U+0020..=U+007E plus `UI_SYMBOLS`, sorted by code point.
pub fn canonical_glyphs() -> Vec<char> {
    let mut glyphs: Vec<char> = (0x20u8..=0x7E)
        .map(char::from)
        .chain(UI_SYMBOLS.chars())
        .collect();
    glyphs.sort_unstable();
    glyphs.dedup();
    glyphs
}

/// Parses an owned font file. The bytes are not copied again.
pub fn load_font(data: Vec<u8>) -> Result<FontVec, RenderError> {
    Ok(FontVec::try_from_vec(data)?)
}

/// A pixel rectangle inside the atlas. The origin is the top-left corner of the atlas, `x` grows right and
/// `y` grows down. `x`/`y` are the first pixel of the rectangle and `width`/`height` are exclusive extents.
///
/// Every rectangle is a whole raster cell. Glyph ink is positioned inside it with the shared baseline, so no
/// per-glyph bearing or advance is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// One atlas entry: the character and the rectangle that holds its raster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphInfo {
    pub ch: char,
    pub rect: GlyphRect,
}

/// Single-channel coverage atlas for the canonical glyph set.
///
/// Slots are packed edge to edge with no padding, and box-drawing ink reaches the slot edges on purpose.
/// Sampling must therefore be nearest-neighbour with UVs on slot boundaries; linear filtering would bleed
/// the neighbouring slot into the glyph. Pixels outside the glyph slots are zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphAtlas {
    width: u32,
    height: u32,
    glyphs: Vec<GlyphInfo>,
    coverage: Vec<u8>,
}

impl GlyphAtlas {
    /// Rasterises every glyph in [`canonical_glyphs`] into a power-of-two atlas.
    ///
    /// Fails with [`RenderError::MissingGlyph`] if the font lacks one of them.
    pub fn build(font: &FontVec) -> Result<Self, RenderError> {
        let chars = canonical_glyphs();
        let rows = (chars.len() as u32).div_ceil(ATLAS_COLUMNS);
        let width = (ATLAS_COLUMNS * GLYPH_CELL_WIDTH).next_power_of_two();
        let height = (rows * GLYPH_CELL_HEIGHT).next_power_of_two();

        let mut coverage = vec![0u8; (width * height) as usize];
        let mut glyphs = Vec::with_capacity(chars.len());
        for (index, ch) in chars.into_iter().enumerate() {
            let index = index as u32;
            let x = (index % ATLAS_COLUMNS) * GLYPH_CELL_WIDTH;
            let y = (index / ATLAS_COLUMNS) * GLYPH_CELL_HEIGHT;
            let raster = rasterise(font, ch)?;
            for row in 0..GLYPH_CELL_HEIGHT {
                let src = (row * GLYPH_CELL_WIDTH) as usize;
                let dst = ((y + row) * width + x) as usize;
                let len = GLYPH_CELL_WIDTH as usize;
                coverage[dst..dst + len].copy_from_slice(&raster[src..src + len]);
            }
            glyphs.push(GlyphInfo {
                ch,
                rect: GlyphRect {
                    x,
                    y,
                    width: GLYPH_CELL_WIDTH,
                    height: GLYPH_CELL_HEIGHT,
                },
            });
        }

        Ok(Self {
            width,
            height,
            glyphs,
            coverage,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Row-major coverage bytes, `width() * height()` long. `0` is empty and `255` is full.
    pub fn coverage(&self) -> &[u8] {
        &self.coverage
    }

    /// All entries, sorted by character.
    pub fn glyphs(&self) -> &[GlyphInfo] {
        &self.glyphs
    }

    pub fn get(&self, ch: char) -> Option<&GlyphInfo> {
        self.glyphs
            .binary_search_by_key(&ch, |info| info.ch)
            .ok()
            .and_then(|index| self.glyphs.get(index))
    }
}

/// Scale that maps the monospace advance to the cell width and the ascender-to-descender height to the cell height.
///
/// `ab_glyph` divides both axes by the font's unscaled height, so `x` is set from the advance ratio.
fn cell_scale(font: &FontVec) -> PxScale {
    let height = font.height_unscaled();
    let advance = font.h_advance_unscaled(font.glyph_id('0'));
    PxScale {
        x: GLYPH_CELL_WIDTH as f32 * height / advance,
        y: GLYPH_CELL_HEIGHT as f32,
    }
}

/// Rasterises one glyph into a `GLYPH_CELL_WIDTH` × `GLYPH_CELL_HEIGHT` coverage buffer.
fn rasterise(font: &FontVec, ch: char) -> Result<Vec<u8>, RenderError> {
    let id = font.glyph_id(ch);
    if id.0 == 0 {
        return Err(RenderError::MissingGlyph(ch));
    }

    let scale = cell_scale(font);
    let baseline = font.as_scaled(scale).ascent().round();
    let mut coverage = vec![0u8; (GLYPH_CELL_WIDTH * GLYPH_CELL_HEIGHT) as usize];
    if let Some(outlined) =
        font.outline_glyph(id.with_scale_and_position(scale, point(0.0, baseline)))
    {
        let origin = outlined.px_bounds().min;
        outlined.draw(|x, y, c| {
            let px = origin.x as i32 + x as i32;
            let py = origin.y as i32 + y as i32;
            let inside = (0..GLYPH_CELL_WIDTH as i32).contains(&px)
                && (0..GLYPH_CELL_HEIGHT as i32).contains(&py);
            if inside {
                let index = py as u32 * GLYPH_CELL_WIDTH + px as u32;
                coverage[index as usize] = (c.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        });
    }
    Ok(coverage)
}
