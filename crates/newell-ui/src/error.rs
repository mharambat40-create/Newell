// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Errors for the UI data structures.

use thiserror::Error;

/// Errors produced while constructing validated UI data structures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum UiError {
    #[error("invalid tree projection")]
    InvalidTreeProjection,
    #[error("projection generation exhausted")]
    ProjectionGenerationExhausted,
    #[error("interaction identity exhausted")]
    InteractionIdentityExhausted,
    /// `cols * rows` overflowed, or the cell count does not fit in `usize`.
    #[error("grid cell count overflows for {cols} x {rows}")]
    CellCountOverflow { cols: u32, rows: u32 },

    /// The requested grid exceeds [`crate::MAX_CELLS`].
    #[error("grid of {cols} x {rows} cells exceeds the maximum of {max} cells")]
    TooManyCells { cols: u32, rows: u32, max: u32 },

    /// A cell metric dimension was zero.
    #[error("invalid grid metrics {width_px} x {height_px} px: both must be non-zero")]
    InvalidMetrics { width_px: u32, height_px: u32 },
}
