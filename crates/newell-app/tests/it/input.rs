// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use newell_app::input::{translate_key, ui_point};
use newell_ui::{ShellKey, UiPoint};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, NativeKeyCode, PhysicalKey};

const NONE: ModifiersState = ModifiersState::empty();

fn unknown() -> PhysicalKey {
    PhysicalKey::Unidentified(NativeKeyCode::Unidentified)
}

fn ch(s: &str) -> Key {
    Key::Character(s.into())
}

fn named(key: NamedKey, mods: ModifiersState) -> Vec<ShellKey> {
    translate_key(&Key::Named(key), unknown(), None, mods)
}

#[test]
fn pointer_positions_are_floored_and_validated() {
    assert_eq!(ui_point(0.0, 0.0), Some(UiPoint { x: 0, y: 0 }));
    assert_eq!(ui_point(12.9, 7.2), Some(UiPoint { x: 12, y: 7 }));
    assert_eq!(ui_point(-0.5, 3.0), None);
    assert_eq!(ui_point(3.0, -1.0), None);
    assert_eq!(ui_point(f64::NAN, 1.0), None);
    assert_eq!(ui_point(1.0, f64::INFINITY), None);
    assert_eq!(ui_point(1e20, 0.0), Some(UiPoint { x: u32::MAX, y: 0 }));
}

#[test]
fn named_keys_map_to_shell_keys() {
    let cases = [
        (NamedKey::ArrowLeft, ShellKey::Left),
        (NamedKey::ArrowRight, ShellKey::Right),
        (NamedKey::ArrowUp, ShellKey::Up),
        (NamedKey::ArrowDown, ShellKey::Down),
        (NamedKey::Tab, ShellKey::Tab),
        (NamedKey::Enter, ShellKey::Enter),
        (NamedKey::Escape, ShellKey::Escape),
        (NamedKey::Backspace, ShellKey::Backspace),
        (NamedKey::Space, ShellKey::Char(' ')),
        (NamedKey::F10, ShellKey::MenuBar),
        (NamedKey::F6, ShellKey::CommandBar),
    ];
    for (key, want) in cases {
        assert_eq!(named(key, NONE), vec![want], "{key:?}");
    }
    assert_eq!(
        named(NamedKey::Tab, ModifiersState::SHIFT),
        vec![ShellKey::BackTab]
    );
    assert!(named(NamedKey::F1, NONE).is_empty());
    assert!(named(NamedKey::Shift, NONE).is_empty());
}

#[test]
fn typed_text_becomes_chars_without_control_characters() {
    let keys = |logical: &Key, text: Option<&str>| translate_key(logical, unknown(), text, NONE);
    assert_eq!(keys(&ch("a"), Some("a")), vec![ShellKey::Char('a')]);
    assert_eq!(keys(&ch("a"), None), vec![ShellKey::Char('a')]);
    assert_eq!(
        keys(&ch("é"), Some("é")),
        vec![ShellKey::Char('é')],
        "non-ASCII text is typed"
    );
    // IME / paste-like text with embedded controls: only printable characters survive.
    assert_eq!(
        keys(&ch("a"), Some("a\u{1b}\r\tb\u{7f}")),
        vec![ShellKey::Char('a'), ShellKey::Char('b')]
    );
    assert!(keys(&ch("\u{7f}"), None).is_empty());
    assert!(keys(&ch("\r"), Some("\r")).is_empty());
    assert!(keys(&Key::Dead(None), None).is_empty());
    assert!(keys(&Key::Unidentified(NativeKeyCode::Unidentified.into()), None).is_empty());
}

#[test]
fn ctrl_and_super_chords_are_not_text() {
    for mods in [ModifiersState::CONTROL, ModifiersState::SUPER] {
        assert!(translate_key(&ch("c"), unknown(), Some("c"), mods).is_empty());
    }
}

#[test]
fn alt_chords_use_the_logical_key_then_the_physical_key() {
    let alt = ModifiersState::ALT;
    let f = PhysicalKey::Code(KeyCode::KeyF);
    assert_eq!(
        translate_key(&ch("v"), unknown(), Some("v"), alt),
        vec![ShellKey::Alt('v')]
    );
    // macOS: Alt+F composes to `ƒ`; the physical key still says F.
    assert_eq!(
        translate_key(&ch("ƒ"), f, Some("ƒ"), alt),
        vec![ShellKey::Alt('f')]
    );
    assert_eq!(
        translate_key(&ch("ƒ"), PhysicalKey::Code(KeyCode::Digit3), None, alt),
        vec![ShellKey::Alt('3')]
    );
    assert!(translate_key(&ch("ƒ"), unknown(), Some("ƒ"), alt).is_empty());
    assert!(translate_key(&ch("ƒ"), PhysicalKey::Code(KeyCode::Space), None, alt).is_empty());
}
