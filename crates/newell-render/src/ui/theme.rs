// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Strictly monochrome UI themes (`ARCHITECTURE.md` §3.6).

/// The two UI themes. A theme is one foreground/background pair; switching themes exchanges
/// the two and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// White background, black foreground.
    #[default]
    Light,
    /// Black background, white foreground.
    Dark,
}

impl Theme {
    /// Foreground colour, RGB in `0.0..=1.0`. Always pure black or pure white.
    #[must_use]
    pub const fn foreground(self) -> [f32; 3] {
        match self {
            Self::Light => [0.0; 3],
            Self::Dark => [1.0; 3],
        }
    }

    /// Background colour, RGB in `0.0..=1.0`. Always the inverse of [`Self::foreground`].
    #[must_use]
    pub const fn background(self) -> [f32; 3] {
        match self {
            Self::Light => [1.0; 3],
            Self::Dark => [0.0; 3],
        }
    }

    /// The other theme.
    #[must_use]
    pub const fn inverted(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn themes_are_pure_black_and_white() {
        for theme in [Theme::Light, Theme::Dark] {
            for c in theme.foreground().into_iter().chain(theme.background()) {
                assert!(c == 0.0 || c == 1.0, "{theme:?}: {c}");
            }
        }
        assert_eq!(Theme::Light.foreground(), [0.0; 3]);
        assert_eq!(Theme::Light.background(), [1.0; 3]);
        assert_eq!(Theme::Dark.foreground(), [1.0; 3]);
        assert_eq!(Theme::Dark.background(), [0.0; 3]);
    }

    #[test]
    fn light_is_the_default_theme() {
        assert_eq!(Theme::default(), Theme::Light);
    }

    #[test]
    fn switching_exchanges_foreground_and_background() {
        for theme in [Theme::Light, Theme::Dark] {
            assert_eq!(theme.inverted().foreground(), theme.background());
            assert_eq!(theme.inverted().background(), theme.foreground());
            assert_eq!(theme.inverted().inverted(), theme);
        }
    }
}
