// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! CPU-side preparation of the GPU cell buffer. No GPU access, so it is testable without a device.

use newell_ui::{Cell, CellContent, CellGrid};

use crate::glyph_atlas::GlyphAtlas;

/// `GpuCell::glyph` value for a cell that draws nothing.
pub const EMPTY_GLYPH: u32 = u32::MAX;

/// Size of one serialised [`GpuCell`]: two little-endian `u32`s (glyph, semantic ink).
pub const GPU_CELL_BYTES: usize = 8;

/// Character drawn for a glyph that is not in the atlas.
const FALLBACK_GLYPH: char = '?';

/// One glyph atlas origin and its semantic ink index, consumed by the UI shader.
///
/// Logical styles and hit-test routing remain CPU state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuCell {
    /// Atlas slot origin in pixels, `x | y << 16`, or [`EMPTY_GLYPH`].
    pub glyph: u32,
    /// `InkRole::index()` in palette order; the shader resolves invalid raw indices to Text.
    pub ink: u32,
}

fn pack_origin(rect: &crate::glyph_atlas::GlyphRect) -> Option<u32> {
    let x = u16::try_from(rect.x).ok()?;
    let y = u16::try_from(rect.y).ok()?;
    Some(u32::from(x) | (u32::from(y) << 16))
}

impl GpuCell {
    /// Resolves the atlas origin, falling back to `?`, and retains the cell's semantic ink.
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
            ink: cell.ink.index(),
        }
    }

    /// Eight bytes: little-endian glyph origin followed by little-endian ink index.
    pub fn to_bytes(self) -> [u8; GPU_CELL_BYTES] {
        let mut bytes = [0; GPU_CELL_BYTES];
        bytes[..4].copy_from_slice(&self.glyph.to_le_bytes());
        bytes[4..].copy_from_slice(&self.ink.to_le_bytes());
        bytes
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
    use crate::glyph_atlas::load_font;
    use newell_ui::{DEFAULT_K, GridMetrics, StyleId, WidgetId};

    fn atlas() -> GlyphAtlas {
        let font = load_font(include_bytes!("../../../../assets/FiraCode-Regular.ttf").to_vec());
        GlyphAtlas::build(&font.unwrap(), GridMetrics::from_k(DEFAULT_K).unwrap()).unwrap()
    }

    fn origin(packed: u32) -> (u32, u32) {
        (packed & 0xFFFF, packed >> 16)
    }

    fn cell(ch: char, style: u16) -> Cell {
        Cell {
            ink: newell_ui::InkRole::Text,
            content: CellContent::Glyph(ch),
            style: StyleId::new(style),
            widget_id: None,
            shared_border: None,
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
            let layout = atlas.layout();
            assert!(x + layout.cell_width <= atlas.width(), "{:?}", info.ch);
            assert!(y + layout.cell_height <= atlas.height(), "{:?}", info.ch);
        }
    }

    #[test]
    fn unknown_glyph_falls_back_to_question_mark() {
        let atlas = atlas();
        let g = GpuCell::from_cell(&cell('\u{1F600}', 0), &atlas);
        assert_eq!(g, GpuCell::from_cell(&cell('?', 0), &atlas));
    }

    #[test]
    fn unknown_text_leaves_the_atlas_unchanged() {
        // `é` is in the font but not in the required set: it is drawn as `?`, and building the
        // cell buffer only reads the atlas (bytes, size and slot map stay identical).
        let atlas = atlas();
        let before = atlas.clone();
        let g = GpuCell::from_cell(&cell('é', 0), &atlas);
        assert_eq!(g, GpuCell::from_cell(&cell('?', 0), &atlas));
        assert_eq!(atlas, before);
    }

    #[test]
    fn styles_never_reach_the_gpu() {
        // Logical style never changes the semantic ink word.
        let atlas = atlas();
        let plain = GpuCell::from_cell(&cell('A', 0), &atlas);
        for style in [1, 2, 3, 4, u16::MAX] {
            assert_eq!(GpuCell::from_cell(&cell('A', style), &atlas), plain);
        }
    }

    #[test]
    fn widget_ownership_never_reaches_the_gpu() {
        // Pointer routing is a CPU concern: the same glyph with and without an owner
        // serialises identically.
        let atlas = atlas();
        let mut owned = cell('x', 0);
        owned.widget_id = Some(WidgetId::new(7));
        assert_eq!(
            GpuCell::from_cell(&owned, &atlas),
            GpuCell::from_cell(&cell('x', 0), &atlas)
        );
    }

    #[test]
    fn bytes_are_the_little_endian_glyph_and_ink() {
        let g = GpuCell {
            glyph: 0x0028_0018,
            ink: 2,
        };
        assert_eq!(g.to_bytes(), [0x18, 0x00, 0x28, 0x00, 2, 0, 0, 0]);
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

#[cfg(test)]
mod color_tests {
    use super::*;
    use newell_ui::{InkRole, StyleId, WidgetId};
    #[test]
    fn ink_word_preserves_role_through_fallback_and_omits_routing_style() {
        let font = crate::glyph_atlas::load_font(
            include_bytes!("../../../../assets/FiraCode-Regular.ttf").to_vec(),
        )
        .unwrap();
        let atlas = GlyphAtlas::build(&font, newell_ui::GridMetrics::from_k(5).unwrap()).unwrap();
        for ink in [InkRole::Text, InkRole::Contour, InkRole::Action] {
            let mut c = Cell {
                content: CellContent::Glyph('é'),
                ink,
                ..Cell::default()
            };
            let raw = GpuCell::from_cell(&c, &atlas);
            assert_eq!(raw.ink, ink.index());
            assert_eq!(&raw.to_bytes()[4..], &ink.index().to_le_bytes());
            c.content = CellContent::Glyph('?');
            assert_eq!(GpuCell::from_cell(&c, &atlas), raw);
            c.style = StyleId::new(4);
            c.widget_id = Some(WidgetId::new(5));
            assert_eq!(GpuCell::from_cell(&c, &atlas), raw);
        }
    }
}
