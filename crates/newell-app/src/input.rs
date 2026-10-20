// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Translation of `winit` input into `newell-ui` input.
//!
//! Pure functions, so they are testable without a window. Control characters are dropped
//! here, before they can reach a text area or the menu mnemonic matcher.

use newell_ui::{EditKey, ShellKey, UiPoint};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

/// A physical pointer position as a [`UiPoint`]. Non-finite and negative coordinates are
/// outside the grid by definition (`None`); fractions are floored.
#[must_use]
pub fn ui_point(x: f64, y: f64) -> Option<UiPoint> {
    let valid = |v: f64| v.is_finite() && v >= 0.0;
    // `as` saturates for floats, so a huge coordinate lands past the grid.
    (valid(x) && valid(y)).then_some(UiPoint {
        x: x as u32,
        y: y as u32,
    })
}

/// Translates a key press into zero or more [`ShellKey`]s.
///
/// - Named keys: arrows (Shift extends the selection), Home, End, Delete, Tab (Shift+Tab is `BackTab`), Enter, Escape, Backspace, Space,
///   F10 (MenuBar) and F6 (CommandBar). Other named keys give nothing.
/// - Alt plus a letter or digit is an `Alt` mnemonic. The logical key is used when it is
///   ASCII; otherwise (macOS composes Alt+F into `ƒ`) the physical key decides.
/// - Ctrl and Super chords are not text and give nothing.
/// - Otherwise the typed `text` (or, without it, the logical character) becomes `Char`s,
///   with every control character removed.
#[must_use]
pub fn translate_key(
    logical: &Key,
    physical: PhysicalKey,
    text: Option<&str>,
    modifiers: ModifiersState,
) -> Vec<ShellKey> {
    if let Key::Named(named) = logical {
        return named_key(*named, modifiers).into_iter().collect();
    }
    if modifiers.control_key() || modifiers.super_key() {
        // Ctrl/Cmd+A selects all in the field being edited; other chords are not text.
        let select_all = matches!(logical, Key::Character(s) if s.eq_ignore_ascii_case("a"));
        return if select_all && !modifiers.alt_key() {
            vec![ShellKey::Edit(EditKey::SelectAll)]
        } else {
            Vec::new()
        };
    }
    if modifiers.alt_key() {
        let logical_char = match logical {
            Key::Character(s) => s.chars().next().filter(char::is_ascii_alphanumeric),
            _ => None,
        };
        let c = logical_char.or_else(|| physical_char(physical));
        return c.and_then(ShellKey::alt).into_iter().collect();
    }
    let typed = match (text, logical) {
        (Some(t), _) => t,
        (None, Key::Character(s)) => s.as_str(),
        _ => "",
    };
    typed.chars().filter_map(ShellKey::character).collect()
}

fn named_key(key: NamedKey, modifiers: ModifiersState) -> Option<ShellKey> {
    Some(match key {
        NamedKey::ArrowLeft if modifiers.shift_key() => ShellKey::Edit(EditKey::SelectLeft),
        NamedKey::ArrowRight if modifiers.shift_key() => ShellKey::Edit(EditKey::SelectRight),
        NamedKey::Home if modifiers.shift_key() => ShellKey::Edit(EditKey::SelectHome),
        NamedKey::End if modifiers.shift_key() => ShellKey::Edit(EditKey::SelectEnd),
        NamedKey::Home => ShellKey::Edit(EditKey::Home),
        NamedKey::End => ShellKey::Edit(EditKey::End),
        NamedKey::Delete => ShellKey::Edit(EditKey::Delete),
        NamedKey::ArrowLeft => ShellKey::Left,
        NamedKey::ArrowRight => ShellKey::Right,
        NamedKey::ArrowUp => ShellKey::Up,
        NamedKey::ArrowDown => ShellKey::Down,
        NamedKey::Tab if modifiers.shift_key() => ShellKey::BackTab,
        NamedKey::Tab => ShellKey::Tab,
        NamedKey::Enter => ShellKey::Enter,
        NamedKey::Escape => ShellKey::Escape,
        NamedKey::Backspace => ShellKey::Backspace,
        NamedKey::Space => ShellKey::Char(' '),
        NamedKey::F10 => ShellKey::MenuBar,
        NamedKey::F6 => ShellKey::CommandBar,
        _ => return None,
    })
}

/// The ASCII letter or digit printed on a physical key, for Alt chords.
fn physical_char(key: PhysicalKey) -> Option<char> {
    let PhysicalKey::Code(code) = key else {
        return None;
    };
    let letters = [
        (KeyCode::KeyA, 'a'),
        (KeyCode::KeyB, 'b'),
        (KeyCode::KeyC, 'c'),
        (KeyCode::KeyD, 'd'),
        (KeyCode::KeyE, 'e'),
        (KeyCode::KeyF, 'f'),
        (KeyCode::KeyG, 'g'),
        (KeyCode::KeyH, 'h'),
        (KeyCode::KeyI, 'i'),
        (KeyCode::KeyJ, 'j'),
        (KeyCode::KeyK, 'k'),
        (KeyCode::KeyL, 'l'),
        (KeyCode::KeyM, 'm'),
        (KeyCode::KeyN, 'n'),
        (KeyCode::KeyO, 'o'),
        (KeyCode::KeyP, 'p'),
        (KeyCode::KeyQ, 'q'),
        (KeyCode::KeyR, 'r'),
        (KeyCode::KeyS, 's'),
        (KeyCode::KeyT, 't'),
        (KeyCode::KeyU, 'u'),
        (KeyCode::KeyV, 'v'),
        (KeyCode::KeyW, 'w'),
        (KeyCode::KeyX, 'x'),
        (KeyCode::KeyY, 'y'),
        (KeyCode::KeyZ, 'z'),
        (KeyCode::Digit0, '0'),
        (KeyCode::Digit1, '1'),
        (KeyCode::Digit2, '2'),
        (KeyCode::Digit3, '3'),
        (KeyCode::Digit4, '4'),
        (KeyCode::Digit5, '5'),
        (KeyCode::Digit6, '6'),
        (KeyCode::Digit7, '7'),
        (KeyCode::Digit8, '8'),
        (KeyCode::Digit9, '9'),
    ];
    letters.iter().find(|(k, _)| *k == code).map(|&(_, c)| c)
}

/// Wheel steps for exponential zoom. Pixel deltas are physical pixels; normalize
/// to logical pixels so Retina and standard displays have identical sensitivity.
#[must_use]
pub fn zoom_steps(delta: winit::event::MouseScrollDelta, scale_factor: f64) -> f32 {
    let steps = match delta {
        winit::event::MouseScrollDelta::LineDelta(_, y) => f64::from(y),
        winit::event::MouseScrollDelta::PixelDelta(p) => p.y / scale_factor.max(1.0) / 100.0,
    };
    if steps.is_finite() {
        steps.clamp(-100.0, 100.0) as f32
    } else {
        0.0
    }
}

/// Native pinch deltas are fractional magnification, independent of display DPI.
#[must_use]
pub fn pinch_steps(delta: f64) -> f32 {
    if delta.is_finite() {
        (delta.clamp(-1.0, 1.0) / 0.12) as f32
    } else {
        0.0
    }
}

/// Tracks ownership of a phased trackpad gesture, including momentum events.
/// A gesture rejected at its start cannot leak into the scene when the pointer moves.
#[derive(Debug, Default)]
pub struct NavigationGesture {
    admitted: Option<bool>,
}

impl NavigationGesture {
    /// Rejects the remainder of the current stream until a new start or an end.
    pub fn cancel(&mut self) {
        if self.admitted.is_some() {
            self.admitted = Some(false);
        }
    }

    /// Claims a new gesture only over visible scene pixels. Crossing UI cancels it
    /// until a new start (or end), while platforms without Started can begin on Moved.
    pub fn update(&mut self, phase: winit::event::TouchPhase, visible: bool) -> bool {
        use winit::event::TouchPhase;
        if phase == TouchPhase::Cancelled {
            self.admitted = None;
            return false;
        }
        if phase == TouchPhase::Started {
            self.admitted = Some(visible);
        }
        let allowed = self.admitted.get_or_insert(visible);
        *allowed &= visible;
        let result = *allowed;
        if phase == TouchPhase::Ended {
            self.admitted = None;
        }
        result
    }
}
