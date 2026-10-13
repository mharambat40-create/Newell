// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! CPU-side preparation of the GPU cell buffer. No GPU access, so it is testable without a device.

use newell_ui::{Cell, CellContent, CellGrid, StyleId};

use crate::glyph_atlas::{GlyphAtlas, GlyphRect};

/// `GpuCell::glyph` value for a cell that draws nothing.
pub const EMPTY_GLYPH: u32 = u32::MAX;

/// Size of one serialised [`GpuCell`]: two little-endian `u32`s.
pub const GPU_CELL_BYTES: usize = 8;

/// Character drawn for a glyph that is not in the atlas.
const FALLBACK_GLYPH: char = '?';

/// Placeholder text palette (sRGB, RGBA). Theming is not in scope for M2.4.
const PALETTE: [[u8; 4]; 4] = [
    [0xD8, 0xD8, 0xDC, 0xFF], // 0: default text
    [0xFF, 0xFF, 0xFF, 0xFF], // 1: bright
    [0x80, 0x80, 0x88, 0xFF], // 2: dim
    [0x4F, 0xC1, 0xE9, 0xFF], // 3: accent
];

/// Text colour for a style. Styles outside the palette use the default colour.
pub fn resolve_style(style: StyleId) -> [u8; 4] {
    PALETTE
        .get(usize::from(style.get()))
        .copied()
        .unwrap_or(PALETTE[0])
}

/// What the shader needs for one cell: where its glyph lives in the atlas and its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuCell {
    /// Atlas slot origin in pixels, `x | y << 16`, or [`EMPTY_GLYPH`].
    pub glyph: u32,
    /// sRGB RGBA8, red in the lowest byte.
    pub color: u32,
}

fn pack_origin(rect: &GlyphRect) -> Option<u32> {
    let x = u16::try_from(rect.x).ok()?;
    let y = u16::try_from(rect.y).ok()?;
    Some(u32::from(x) | (u32::from(y) << 16))
}

impl GpuCell {
    pub fn from_cell(cell: &Cell, atlas: &GlyphAtlas) -> Self {
        let glyph = match cell.content {
            CellContent::Empty => EMPTY_GLYPH,
            CellContent::Glyph(ch) => atlas
                .get(ch)
                .or_else(|| atlas.get(FALLBACK_GLYPH))
                .and_then(|info| pack_origin(&info.rect))
                .unwrap_or(EMPTY_GLYPH),
        };
        Self {
            glyph,
            color: u32::from_le_bytes(resolve_style(cell.style)),
        }
    }

    pub fn to_bytes(self) -> [u8; GPU_CELL_BYTES] {
        let mut out = [0u8; GPU_CELL_BYTES];
        let (glyph, color) = out.split_at_mut(4);
        glyph.copy_from_slice(&self.glyph.to_le_bytes());
        color.copy_from_slice(&self.color.to_le_bytes());
        out
    }
}

/// Serialises the whole grid, row-major, `GPU_CELL_BYTES` per cell. Linear in the cell count.
pub fn cells_to_bytes(grid: &CellGrid, atlas: &GlyphAtlas) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(grid.cells().len() * GPU_CELL_BYTES);
    for cell in grid.cells() {
        bytes.extend_from_slice(&GpuCell::from_cell(cell, atlas).to_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph_atlas::{GLYPH_CELL_HEIGHT, GLYPH_CELL_WIDTH, load_font};

    fn atlas() -> GlyphAtlas {
        let font = load_font(include_bytes!("../../../../assets/FiraCode-Regular.ttf").to_vec());
        GlyphAtlas::build(&font.unwrap()).unwrap()
    }

    fn origin(packed: u32) -> (u32, u32) {
        (packed & 0xFFFF, packed >> 16)
    }

    fn cell(ch: char, style: u16) -> Cell {
        Cell {
            content: CellContent::Glyph(ch),
            style: StyleId::new(style),
            widget_id: None,
        }
    }

    #[test]
    fn empty_cell_is_transparent_sentinel() {
        let g = GpuCell::from_cell(&Cell::default(), &atlas());
        assert_eq!(g.glyph, EMPTY_GLYPH);
    }

    #[test]
    fn glyph_cell_points_at_its_atlas_slot() {
        let atlas = atlas();
        for ch in ['A', 'a', '0', ' ', '─', '╬'] {
            let g = GpuCell::from_cell(&cell(ch, 0), &atlas);
            let rect = atlas.get(ch).unwrap().rect;
            assert_eq!(origin(g.glyph), (rect.x, rect.y), "{ch:?}");
        }
    }

    #[test]
    fn every_slot_is_inside_the_atlas_and_packs_losslessly() {
        let atlas = atlas();
        for info in atlas.glyphs() {
            let g = GpuCell::from_cell(&cell(info.ch, 0), &atlas);
            let (x, y) = origin(g.glyph);
            assert_ne!(g.glyph, EMPTY_GLYPH);
            assert!(x + GLYPH_CELL_WIDTH <= atlas.width(), "{:?}", info.ch);
            assert!(y + GLYPH_CELL_HEIGHT <= atlas.height(), "{:?}", info.ch);
        }
    }

    #[test]
    fn unknown_glyph_falls_back_to_question_mark() {
        let atlas = atlas();
        let g = GpuCell::from_cell(&cell('\u{1F600}', 0), &atlas);
        assert_eq!(g, GpuCell::from_cell(&cell('?', 0), &atlas));
    }

    #[test]
    fn style_mapping_is_deterministic_with_default_fallback() {
        assert_eq!(resolve_style(StyleId::DEFAULT), PALETTE[0]);
        assert_eq!(resolve_style(StyleId::new(1)), PALETTE[1]);
        assert_eq!(resolve_style(StyleId::new(3)), PALETTE[3]);
        assert_eq!(resolve_style(StyleId::new(4)), PALETTE[0]);
        assert_eq!(resolve_style(StyleId::new(u16::MAX)), PALETTE[0]);
        let g = GpuCell::from_cell(&cell('A', 3), &atlas());
        assert_eq!(g.color.to_le_bytes(), PALETTE[3]);
    }

    #[test]
    fn bytes_are_little_endian_glyph_then_color() {
        let g = GpuCell {
            glyph: 0x0028_0018,
            color: 0x4433_2211,
        };
        assert_eq!(
            g.to_bytes(),
            [0x18, 0x00, 0x28, 0x00, 0x11, 0x22, 0x33, 0x44]
        );
    }

    #[test]
    fn buffer_length_and_determinism() {
        let atlas = atlas();
        let mut grid = CellGrid::new(5, 3).unwrap();
        if let Some(c) = grid.get_mut(1, 1) {
            *c = cell('┌', 2);
        }
        let a = cells_to_bytes(&grid, &atlas);
        assert_eq!(a.len(), 15 * GPU_CELL_BYTES);
        assert_eq!(a, cells_to_bytes(&grid, &atlas));
        // Row-major: (1, 1) is cell 6; its neighbours stay empty.
        let at = |i: usize| a[i * GPU_CELL_BYTES..(i + 1) * GPU_CELL_BYTES].to_vec();
        assert_eq!(at(6), GpuCell::from_cell(&cell('┌', 2), &atlas).to_bytes());
        assert_eq!(
            at(5),
            GpuCell::from_cell(&Cell::default(), &atlas).to_bytes()
        );
    }

    #[test]
    fn zero_sized_grid_gives_empty_buffer() {
        let atlas = atlas();
        for (c, r) in [(0, 0), (0, 4), (4, 0)] {
            assert!(cells_to_bytes(&CellGrid::new(c, r).unwrap(), &atlas).is_empty());
        }
    }
}
