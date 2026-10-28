// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! UI themes and the fixed semantic palette (`ARCHITECTURE.md` §3.6).

/// The two UI Theme choices. These foreground/background accessors retain the exact
/// monochrome pair; [`UiPalette`] resolves the independent Theme/Color combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// White background, black foreground.
    Light,
    /// Black background, white foreground.
    #[default]
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

/// Per-window Theme/Color choices; source colours are normalized sRGB components.
///
/// Defaults to Dark with Color unchecked. Palette selection never changes the atlas,
/// cell geometry or interaction styling; callers handle any changed glyph grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiPalette {
    theme: Theme,
    color_enabled: bool,
}

impl UiPalette {
    /// Selects a Theme and its independent Color checkbox state.
    #[must_use]
    pub const fn new(theme: Theme, color_enabled: bool) -> Self {
        Self {
            theme,
            color_enabled,
        }
    }
    /// The selected Light/Dark Theme.
    #[must_use]
    pub const fn theme(self) -> Theme {
        self.theme
    }
    /// Whether the three semantic inks are visible instead of the monochrome ink.
    #[must_use]
    pub const fn color_enabled(self) -> bool {
        self.color_enabled
    }
    /// Changes Theme while retaining this window's Color checkbox state.
    #[must_use]
    pub const fn with_theme(self, theme: Theme) -> Self {
        Self { theme, ..self }
    }
    /// Source sRGB ink in `0.0..=1.0`; ordinary roles share the same ink when Color is off.
    /// Target-format conversion and glyph-coverage blending belong to the renderer.
    #[must_use]
    pub fn foreground(self, ink: newell_ui::InkRole) -> [f32; 3] {
        use newell_ui::InkRole::{Action, Contour, Placeholder, Text};
        if ink != Placeholder && !self.color_enabled {
            return self.theme.foreground();
        }
        let rgb = match (self.theme, ink) {
            (Theme::Dark, Text) => [255, 255, 255],
            (Theme::Dark, Contour) => [65, 180, 229],
            (Theme::Dark, Action) => [255, 162, 74],
            (Theme::Light, Text) => [17, 24, 39],
            (Theme::Light, Contour) => [9, 105, 160],
            (Theme::Light, Action) => [154, 77, 0],
            (_, Placeholder) => {
                let text = self.foreground(Text);
                let background = self.background();
                return std::array::from_fn(|i| {
                    ((4.0 * text[i] + background[i]) * 255.0 / 5.0).round() / 255.0
                });
            }
        };
        rgb.map(|v| v as f32 / 255.0)
    }
    /// Source sRGB background in `0.0..=1.0`, also used for the viewport clear.
    #[must_use]
    pub fn background(self) -> [f32; 3] {
        if self.color_enabled && self.theme == Theme::Dark {
            [7.0 / 255.0, 21.0 / 255.0, 33.0 / 255.0]
        } else {
            self.theme.background()
        }
    }
}

/// Mirrors the UI shader's target-format conversion for frame clears.
pub(crate) fn target_rgb(rgb: [f32; 3], srgb: bool) -> [f32; 3] {
    rgb.map(|c| {
        if !srgb {
            c
        } else if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    })
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
    fn dark_is_the_default_theme() {
        assert_eq!(Theme::default(), Theme::Dark);
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

#[cfg(test)]
mod color_tests {
    use super::*;
    use newell_ui::InkRole::{Action, Contour, Text};
    #[test]
    fn exact_four_palettes_defaults_independent_theme_and_light_contrast() {
        assert_eq!(UiPalette::default(), UiPalette::new(Theme::Dark, false));
        for theme in [Theme::Light, Theme::Dark] {
            for ink in [Text, Contour, Action] {
                assert_eq!(
                    UiPalette::new(theme, false).foreground(ink),
                    theme.foreground()
                );
            }
            let on = UiPalette::new(theme, true);
            let expected = if theme == Theme::Dark {
                [[255, 255, 255], [65, 180, 229], [255, 162, 74]]
            } else {
                [[17, 24, 39], [9, 105, 160], [154, 77, 0]]
            };
            for (ink, rgb) in [Text, Contour, Action].into_iter().zip(expected) {
                assert_eq!(on.foreground(ink), rgb.map(|v| v as f32 / 255.0));
                if theme == Theme::Light {
                    let linear = target_rgb(on.foreground(ink), true);
                    let luminance = linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722;
                    assert!(1.05 / (luminance + 0.05) >= 4.5);
                }
            }
            assert!(on.with_theme(theme.inverted()).color_enabled());
            assert_eq!(on.with_theme(theme.inverted()).theme(), theme.inverted());
        }
        assert_eq!(
            UiPalette::new(Theme::Dark, true).background(),
            [7.0 / 255.0, 21.0 / 255.0, 33.0 / 255.0]
        );
        assert_eq!(UiPalette::new(Theme::Light, true).background(), [1.0; 3]);
    }
}

#[cfg(test)]
mod placeholder_tests {
    use super::*;
    use newell_ui::InkRole;
    #[test]
    fn placeholder_inks_are_exact_readable_and_independent_of_ordinary_roles() {
        assert_eq!(InkRole::Placeholder.index(), 3);
        for (theme, color, expected) in [
            (Theme::Dark, false, [204, 204, 204]),
            (Theme::Light, false, [51, 51, 51]),
            (Theme::Dark, true, [205, 208, 211]),
            (Theme::Light, true, [65, 70, 82]),
        ] {
            let palette = UiPalette::new(theme, color);
            let ink = palette.foreground(InkRole::Placeholder);
            assert_eq!(ink, expected.map(|c| c as f32 / 255.0));
            let luminance = |rgb| {
                let c = target_rgb(rgb, true);
                c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722
            };
            let a = luminance(ink);
            let b = luminance(palette.background());
            assert!((a.max(b) + 0.05) / (a.min(b) + 0.05) >= 7.0);
            assert_ne!(ink, palette.foreground(InkRole::Text));
        }
    }
}
