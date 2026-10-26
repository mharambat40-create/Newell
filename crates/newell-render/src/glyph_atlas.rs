// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! CPU-side Fira Code rasterisation and the deterministic glyph atlas (`ARCHITECTURE.md` §3.1).
//!
//! Every glyph is rasterised into one raster cell of the grid's `W` × `H` physical pixels
//! ([`GridMetrics`], 4:9). One uniform scale `s = W / advance` applies to both axes, so the font
//! keeps its natural proportions: the line box is `L = round((ascender − descender + gap) × s)`,
//! the top margin is `(H − L) div 2`, every glyph shares the baseline `top + round(ascender × s)`
//! and its pen origin is the cell's left edge. Ink outside the cell is clipped, never rescaled.
//! The atlas is a single-channel coverage texture laid out in [`canonical_glyphs`] order. It is a
//! pure function of (font, canonical set, metrics) and is never modified after [`GlyphAtlas::build`].

use ab_glyph::{Font, FontVec, PxScale, point};
use newell_ui::GridMetrics;

use crate::RenderError;

/// Glyphs per atlas row. Fixed so the atlas layout does not depend on the glyph count.
const ATLAS_COLUMNS: u32 = 16;

/// Characters of `docs/ui_design/*.txt` outside printable ASCII, plus the text caret `▏` (U+258F).
const UI_SYMBOLS: &str =
    "Øà─━│┃┄┆┈┊┌┏┐┓└┗┘┛├┣┤┫┬┳┴┻┼╋═║╒╓╔╕╖╗╘╙╚╛╜╝╞╟╠╡╢╣╤╥╦╧╩╪╬╭╮╯╰╴╵╶╷╸╹╺╻█▏░▒▓■□▲▶▼◀◐◑☰×…";

/// The required atlas set: printable ASCII U+0020..=U+007E plus `UI_SYMBOLS`, sorted by code point.
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

/// Where the font sits inside one cell, in whole pixels (`ARCHITECTURE.md` §3.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellLayout {
    /// Cell width `W` in pixels.
    pub cell_width: u32,
    /// Cell height `H` in pixels.
    pub cell_height: u32,
    /// Uniform scale `s = W / advance`, pixels per font unit, identical on both axes.
    pub scale: f32,
    /// Line box `L`.
    pub line_box: u32,
    /// Blank rows above the line box, `(H − L) div 2`.
    pub top: u32,
    /// Baseline measured from the top edge of the cell.
    pub baseline: u32,
}

impl CellLayout {
    /// Derives the layout of `font` in the cell of `metrics`.
    ///
    /// Fails with [`RenderError::LineBoxTooTall`] if `L > H`.
    pub fn new(font: &FontVec, metrics: GridMetrics) -> Result<Self, RenderError> {
        let (cell_width, cell_height) = (metrics.cell_width_px(), metrics.cell_height_px());
        let advance = font.h_advance_unscaled(font.glyph_id('0'));
        let scale = cell_width as f32 / advance;
        let ascent = font.ascent_unscaled();
        let natural = ascent - font.descent_unscaled() + font.line_gap_unscaled();
        let line_box = (natural * scale).round() as u32;
        if line_box > cell_height {
            return Err(RenderError::LineBoxTooTall {
                line_box,
                cell_height,
            });
        }
        let top = (cell_height - line_box) / 2;
        Ok(Self {
            cell_width,
            cell_height,
            scale,
            line_box,
            top,
            baseline: top + (ascent * scale).round() as u32,
        })
    }

    /// The one `ab_glyph` scale used on both axes: `s` pixels per font unit.
    fn px_scale(&self, font: &FontVec) -> PxScale {
        PxScale::from(self.scale * font.height_unscaled())
    }
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
    /// `true` if ink outside the cell was cut at its edge (expected only for box drawing and
    /// block elements, whose overshoot is removed by clipping).
    pub clipped: bool,
}

/// Single-channel coverage atlas for the canonical glyph set.
///
/// Slots are packed edge to edge with no padding, and box-drawing ink reaches the slot edges on purpose.
/// Sampling must therefore be nearest-neighbour with UVs on slot boundaries; linear filtering would bleed
/// the neighbouring slot into the glyph. Pixels outside the glyph slots are zero.
#[derive(Debug, Clone, PartialEq)]
pub struct GlyphAtlas {
    layout: CellLayout,
    width: u32,
    height: u32,
    glyphs: Vec<GlyphInfo>,
    coverage: Vec<u8>,
}

impl GlyphAtlas {
    /// Rasterises every glyph in [`canonical_glyphs`] into a power-of-two atlas of `metrics` cells.
    ///
    /// Fails with [`RenderError::MissingGlyph`] if the font lacks one of them.
    pub fn build(font: &FontVec, metrics: GridMetrics) -> Result<Self, RenderError> {
        let layout = CellLayout::new(font, metrics)?;
        let (cell_w, cell_h) = (layout.cell_width, layout.cell_height);
        let chars = canonical_glyphs();
        let rows = (chars.len() as u32).div_ceil(ATLAS_COLUMNS);
        let width = (ATLAS_COLUMNS * cell_w).next_power_of_two();
        let height = (rows * cell_h).next_power_of_two();

        let mut coverage = vec![0u8; (width * height) as usize];
        let mut glyphs = Vec::with_capacity(chars.len());
        for (index, ch) in chars.into_iter().enumerate() {
            let index = index as u32;
            let x = (index % ATLAS_COLUMNS) * cell_w;
            let y = (index / ATLAS_COLUMNS) * cell_h;
            let (raster, clipped) = rasterise(font, &layout, ch)?;
            for row in 0..cell_h {
                let src = (row * cell_w) as usize;
                let dst = ((y + row) * width + x) as usize;
                let len = cell_w as usize;
                coverage[dst..dst + len].copy_from_slice(&raster[src..src + len]);
            }
            glyphs.push(GlyphInfo {
                ch,
                rect: GlyphRect {
                    x,
                    y,
                    width: cell_w,
                    height: cell_h,
                },
                clipped,
            });
        }

        Ok(Self {
            layout,
            width,
            height,
            glyphs,
            coverage,
        })
    }

    /// The cell layout every glyph was rasterised with.
    pub fn layout(&self) -> CellLayout {
        self.layout
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

/// Rasterises one glyph into a `W` × `H` coverage buffer at the uniform scale. Returns the buffer
/// and whether any ink fell outside the cell and was clipped.
fn rasterise(
    font: &FontVec,
    layout: &CellLayout,
    ch: char,
) -> Result<(Vec<u8>, bool), RenderError> {
    let id = font.glyph_id(ch);
    if id.0 == 0 {
        return Err(RenderError::MissingGlyph(ch));
    }
    let (cell_w, cell_h) = (layout.cell_width, layout.cell_height);
    let mut coverage = vec![0u8; (cell_w * cell_h) as usize];
    let mut clipped = false;
    let glyph =
        id.with_scale_and_position(layout.px_scale(font), point(0.0, layout.baseline as f32));
    if let Some(outlined) = font.outline_glyph(glyph) {
        let origin = outlined.px_bounds().min;
        outlined.draw(|x, y, c| {
            let px = origin.x as i32 + x as i32;
            let py = origin.y as i32 + y as i32;
            let inside = (0..cell_w as i32).contains(&px) && (0..cell_h as i32).contains(&py);
            if inside {
                coverage[(py as u32 * cell_w + px as u32) as usize] =
                    (c.clamp(0.0, 1.0) * 255.0).round() as u8;
            } else if c > 0.0 {
                clipped = true;
            }
        });
    }
    Ok((coverage, clipped))
}
