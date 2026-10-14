// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Keys the application shell reacts to (already translated from window events).

/// A translated keyboard input. The shell routes it to the modal, the menus or the base
/// shell; see [`crate::ShellState::key`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKey {
    Left,
    Right,
    Up,
    Down,
    /// Moves focus forward.
    Tab,
    /// Moves focus backward.
    BackTab,
    Enter,
    Escape,
    Backspace,
    /// Starts or leaves keyboard navigation of the MenuBar (F10).
    MenuBar,
    /// Starts or leaves keyboard navigation of the CommandBar (F6).
    CommandBar,
    /// Alt plus an ASCII letter or digit: a top-level mnemonic.
    Alt(char),
    /// A typed, printable character (a space is `Char(' ')`).
    Char(char),
}

impl ShellKey {
    /// A typed character, or `None` for a control character (so `\r`, `\t`, DEL and the
    /// like never reach a text area or the mnemonic matcher).
    #[must_use]
    pub fn character(c: char) -> Option<Self> {
        (!c.is_control()).then_some(Self::Char(c))
    }

    /// An Alt chord, or `None` unless `c` is an ASCII letter or digit.
    #[must_use]
    pub fn alt(c: char) -> Option<Self> {
        c.is_ascii_alphanumeric().then_some(Self::Alt(c))
    }

    /// `false` for keys that carry an unusable character (constructed directly rather than
    /// through [`ShellKey::character`] / [`ShellKey::alt`]).
    pub(super) fn is_valid(self) -> bool {
        match self {
            Self::Char(c) => !c.is_control(),
            Self::Alt(c) => c.is_ascii_alphanumeric(),
            _ => true,
        }
    }
}
