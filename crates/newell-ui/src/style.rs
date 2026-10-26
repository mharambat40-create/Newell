// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Style identifier.

/// Opaque logical interaction-state identifier; it does not select a colour.
///
/// Ink is carried separately by `Cell::ink`; styles do not select colours.
/// Stored as `u16`: 65 536 logical styles are ample and keep
/// [`crate::Cell`] compact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StyleId(u16);

impl StyleId {
    /// The base style used by default cells.
    pub const DEFAULT: Self = Self(0);

    /// Creates a style ID from a raw value.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the raw value.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl Default for StyleId {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_deterministic() {
        assert_eq!(StyleId::default(), StyleId::DEFAULT);
        assert_eq!(StyleId::default().get(), 0);
    }

    #[test]
    fn round_trips_value() {
        assert_eq!(StyleId::new(513).get(), 513);
        assert_ne!(StyleId::new(1), StyleId::DEFAULT);
    }
}
