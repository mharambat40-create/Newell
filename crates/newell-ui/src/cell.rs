// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! A single UI grid cell.

use crate::{StyleId, WidgetId};

/// What a cell displays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellContent {
    /// Nothing drawn.
    #[default]
    Empty,
    /// A single Unicode scalar value.
    Glyph(char),
}

/// One cell of the flat UI grid: content, style, and owning widget (if any).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cell {
    pub content: CellContent,
    pub style: StyleId,
    pub widget_id: Option<WidgetId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cell_is_neutral() {
        let c = Cell::default();
        assert_eq!(c.content, CellContent::Empty);
        assert_eq!(c.style, StyleId::DEFAULT);
        assert_eq!(c.widget_id, None);
    }

    #[test]
    fn cell_with_and_without_widget() {
        let with = Cell {
            content: CellContent::Glyph('A'),
            style: StyleId::new(3),
            widget_id: Some(WidgetId::new(9)),
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
