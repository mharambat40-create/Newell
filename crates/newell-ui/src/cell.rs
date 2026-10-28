// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! A single UI grid cell.

use crate::{SharedBorder, StyleId, WidgetId};

/// What a cell displays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellContent {
    /// Nothing drawn.
    #[default]
    Empty,
    /// A single Unicode scalar value.
    Glyph(char),
}

/// Semantic ink, independent of a glyph's shape, interaction style or hit-test owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum InkRole {
    /// Ordinary text, values, titles, caret and tree hierarchy guides.
    #[default]
    Text = 0,
    /// Structural outlines, separators and shared frame junctions.
    Contour = 1,
    /// Explicit control affordances, independent of their interaction state.
    Action = 2,
    /// Initial input placeholder text only; never a focus or disabled style.
    Placeholder = 3,
}

impl InkRole {
    /// Stable GPU palette index: Text = 0, Contour = 1, Action = 2, Placeholder = 3.
    #[must_use]
    pub const fn index(self) -> u32 {
        self as u32
    }
}

/// One cell of the flat UI grid: content, semantic ink, logical style, owning widget, and
/// optional shared-border routing targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cell {
    pub content: CellContent,
    pub style: StyleId,
    /// Component-assigned ink, preserved by hover and ignored by hit testing.
    pub ink: InkRole,
    pub widget_id: Option<WidgetId>,
    /// When set, this cell is a separator shared by two adjacent regions and
    /// hit-testing routes by sub-cell position instead of using `widget_id`.
    pub shared_border: Option<SharedBorder>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cell_is_neutral() {
        let c = Cell::default();
        assert_eq!(c.content, CellContent::Empty);
        assert_eq!(c.style, StyleId::DEFAULT);
        assert_eq!(c.ink, InkRole::Text);
        assert_eq!(c.widget_id, None);
        assert_eq!(c.shared_border, None);
    }

    #[test]
    fn cell_with_and_without_widget() {
        let with = Cell {
            ink: crate::InkRole::Text,
            content: CellContent::Glyph('A'),
            style: StyleId::new(3),
            widget_id: Some(WidgetId::new(9)),
            shared_border: None,
        };
        let without = Cell {
            widget_id: None,
            ..with
        };
        assert_eq!(with.widget_id, Some(WidgetId::new(9)));
        assert_eq!(without.widget_id, None);
        assert_ne!(with, without);
    }
}
