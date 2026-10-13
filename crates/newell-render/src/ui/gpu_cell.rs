// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! CPU-side preparation of the GPU cell buffer. No GPU access, so it is testable without a device.

use newell_ui::{Cell, CellContent, CellGrid, StyleId, WidgetId};

use crate::glyph_atlas::{GlyphAtlas, GlyphRect};

/// `GpuCell::glyph` value for a cell that draws nothing.
pub const EMPTY_GLYPH: u32 = u32::MAX;

/// Size of one serialised [`GpuCell`]: four little-endian `u32`s.
pub const GPU_CELL_BYTES: usize = 16;

/// `GpuCell::flags` / hover-uniform flag: the `widget` field holds a real `WidgetId`.
///
/// `WidgetId` uses the whole `u32` range, so "no widget" cannot be a reserved ID value; it is
/// the absence of this flag instead (and then `widget` is always 0).
pub const WIDGET_PRESENT: u32 = 1;
/// `GpuCell::flags`: the cell is the first cell of its widget on its row (the cell to its left
/// has a different owner, or there is none). The hover left-border stripe is drawn only here.
pub const WIDGET_LEFT_EDGE: u32 = 2;

/// Encodes an optional `WidgetId` as `(widget, flags)`; `None` is `(0, 0)`.
pub const fn encode_widget(widget: Option<WidgetId>) -> (u32, u32) {
    match widget {
        Some(id) => (id.get(), WIDGET_PRESENT),
        None => (0, 0),
    }
}

/// The widget a cell is rendered as belonging to. A shared-border cell is structural, so it
/// has no GPU owner: it never shows hover (routing for the pointer is still `hit_test`'s job).
fn gpu_owner(cell: &Cell) -> Option<WidgetId> {
    if cell.shared_border.is_some() {
        None
    } else {
        cell.widget_id
    }
}

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
    /// Raw `WidgetId`, meaningful only if [`WIDGET_PRESENT`] is set in `flags`; otherwise 0.
    pub widget: u32,
    /// [`WIDGET_PRESENT`] | [`WIDGET_LEFT_EDGE`].
    pub flags: u32,
}

fn pack_origin(rect: &GlyphRect) -> Option<u32> {
    let x = u16::try_from(rect.x).ok()?;
    let y = u16::try_from(rect.y).ok()?;
    Some(u32::from(x) | (u32::from(y) << 16))
}

impl GpuCell {
    /// `left` is the cell immediately to the left, if any (used only for the left-edge flag).
    pub fn from_cell(cell: &Cell, left: Option<&Cell>, atlas: &GlyphAtlas) -> Self {
        let glyph = match cell.content {
            CellContent::Empty => EMPTY_GLYPH,
            CellContent::Glyph(ch) => atlas
                .get(ch)
                .or_else(|| atlas.get(FALLBACK_GLYPH))
                .and_then(|info| pack_origin(&info.rect))
                .unwrap_or(EMPTY_GLYPH),
        };
        let owner = gpu_owner(cell);
        let (widget, mut flags) = encode_widget(owner);
        if owner.is_some() && left.and_then(gpu_owner) != owner {
            flags |= WIDGET_LEFT_EDGE;
        }
        Self {
            glyph,
            color: u32::from_le_bytes(resolve_style(cell.style)),
            widget,
            flags,
        }
    }

    pub fn to_bytes(self) -> [u8; GPU_CELL_BYTES] {
        let mut out = [0u8; GPU_CELL_BYTES];
        for (chunk, value) in
            out.chunks_exact_mut(4)
                .zip([self.glyph, self.color, self.widget, self.flags])
        {
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        out
    }
}

/// Serialises the whole grid, row-major, `GPU_CELL_BYTES` per cell. Linear in the cell count.
pub fn cells_to_bytes(grid: &CellGrid, atlas: &GlyphAtlas) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(grid.cells().len() * GPU_CELL_BYTES);
    let cols = grid.cols().max(1) as usize;
    for (i, cell) in grid.cells().iter().enumerate() {
        let left = if i % cols == 0 {
            None
        } else {
            grid.cells().get(i - 1)
        };
        bytes.extend_from_slice(&GpuCell::from_cell(cell, left, atlas).to_bytes());
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
            shared_border: None,
        }
    }

    #[test]
    fn empty_cell_is_transparent_sentinel() {
        let g = GpuCell::from_cell(&Cell::default(), None, &atlas());
        assert_eq!(g.glyph, EMPTY_GLYPH);
    }

    #[test]
    fn glyph_cell_points_at_its_atlas_slot() {
        let atlas = atlas();
        for ch in ['A', 'a', '0', ' ', '─', '╬'] {
            let g = GpuCell::from_cell(&cell(ch, 0), None, &atlas);
            let rect = atlas.get(ch).unwrap().rect;
            assert_eq!(origin(g.glyph), (rect.x, rect.y), "{ch:?}");
        }
    }

    #[test]
    fn every_slot_is_inside_the_atlas_and_packs_losslessly() {
        let atlas = atlas();
        for info in atlas.glyphs() {
            let g = GpuCell::from_cell(&cell(info.ch, 0), None, &atlas);
            let (x, y) = origin(g.glyph);
            assert_ne!(g.glyph, EMPTY_GLYPH);
            assert!(x + GLYPH_CELL_WIDTH <= atlas.width(), "{:?}", info.ch);
            assert!(y + GLYPH_CELL_HEIGHT <= atlas.height(), "{:?}", info.ch);
        }
    }

    #[test]
    fn unknown_glyph_falls_back_to_question_mark() {
        let atlas = atlas();
        let g = GpuCell::from_cell(&cell('\u{1F600}', 0), None, &atlas);
        assert_eq!(g, GpuCell::from_cell(&cell('?', 0), None, &atlas));
    }

    #[test]
    fn style_mapping_is_deterministic_with_default_fallback() {
        assert_eq!(resolve_style(StyleId::DEFAULT), PALETTE[0]);
        assert_eq!(resolve_style(StyleId::new(1)), PALETTE[1]);
        assert_eq!(resolve_style(StyleId::new(3)), PALETTE[3]);
        assert_eq!(resolve_style(StyleId::new(4)), PALETTE[0]);
        assert_eq!(resolve_style(StyleId::new(u16::MAX)), PALETTE[0]);
        let g = GpuCell::from_cell(&cell('A', 3), None, &atlas());
        assert_eq!(g.color.to_le_bytes(), PALETTE[3]);
    }

    #[test]
    fn bytes_are_little_endian_glyph_then_color() {
        let g = GpuCell {
            glyph: 0x0028_0018,
            color: 0x4433_2211,
            widget: 0x0000_00AB,
            flags: 3,
        };
        assert_eq!(
            g.to_bytes(),
            [
                0x18, 0x00, 0x28, 0x00, 0x11, 0x22, 0x33, 0x44, 0xAB, 0, 0, 0, 3, 0, 0, 0
            ]
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
        assert_eq!(
            at(6),
            GpuCell::from_cell(&cell('┌', 2), None, &atlas).to_bytes()
        );
        assert_eq!(
            at(5),
            GpuCell::from_cell(&Cell::default(), None, &atlas).to_bytes()
        );
    }

    #[test]
    fn zero_sized_grid_gives_empty_buffer() {
        let atlas = atlas();
        for (c, r) in [(0, 0), (0, 4), (4, 0)] {
            assert!(cells_to_bytes(&CellGrid::new(c, r).unwrap(), &atlas).is_empty());
        }
    }

    fn owned(ch: char, id: u32) -> Cell {
        Cell {
            widget_id: Some(WidgetId::new(id)),
            ..cell(ch, 0)
        }
    }

    #[test]
    fn none_widget_encoding_is_explicit_zero() {
        assert_eq!(encode_widget(None), (0, 0));
        let g = GpuCell::from_cell(&cell('A', 0), None, &atlas());
        assert_eq!((g.widget, g.flags), (0, 0));
    }

    #[test]
    fn every_widget_id_is_distinct_from_none() {
        for raw in [0, 1, 7, u32::MAX - 1, u32::MAX] {
            let (widget, flags) = encode_widget(Some(WidgetId::new(raw)));
            assert_eq!((widget, flags), (raw, WIDGET_PRESENT));
            assert_ne!(flags, encode_widget(None).1, "id {raw} collides with None");
        }
    }

    #[test]
    fn left_edge_marks_first_cell_of_each_widget_run() {
        let atlas = atlas();
        let mut grid = CellGrid::new(6, 1).unwrap();
        for (col, id) in [(0, 1), (1, 1), (2, 2), (4, 1), (5, 1)] {
            *grid.get_mut(col, 0).unwrap() = owned('x', id);
        }
        let bytes = cells_to_bytes(&grid, &atlas);
        let flags = |col: usize| {
            let o = col * GPU_CELL_BYTES + 12;
            u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap())
        };
        let edge = WIDGET_PRESENT | WIDGET_LEFT_EDGE;
        assert_eq!(
            [flags(0), flags(1), flags(2), flags(3), flags(4), flags(5)],
            [edge, WIDGET_PRESENT, edge, 0, edge, WIDGET_PRESENT]
        );
    }

    #[test]
    fn left_edge_does_not_wrap_across_rows() {
        let atlas = atlas();
        let mut grid = CellGrid::new(2, 2).unwrap();
        for (col, row) in [(1, 0), (0, 1)] {
            *grid.get_mut(col, row).unwrap() = owned('x', 5);
        }
        let g = GpuCell::from_cell(grid.get(0, 1).unwrap(), None, &atlas);
        assert_eq!(g.flags, WIDGET_PRESENT | WIDGET_LEFT_EDGE);
        let bytes = cells_to_bytes(&grid, &atlas);
        let o = 2 * GPU_CELL_BYTES + 12; // cell (0, 1)
        assert_eq!(
            u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap()),
            WIDGET_PRESENT | WIDGET_LEFT_EDGE
        );
    }

    #[test]
    fn shared_border_cell_has_no_gpu_owner() {
        use newell_ui::{BorderAxis, SharedBorder};
        let mut c = owned('│', 9);
        c.shared_border = Some(SharedBorder {
            axis: BorderAxis::Vertical,
            low: Some(WidgetId::new(1)),
            high: Some(WidgetId::new(2)),
        });
        let g = GpuCell::from_cell(&c, None, &atlas());
        assert_eq!((g.widget, g.flags), (0, 0));
    }
}
