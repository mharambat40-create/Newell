// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Text-editing core shared by the numeric fields and the FormulaEditor (M2.23).
//!
//! [`TextEdit`] holds characters, a caret and a selection anchor; it never contains the caret
//! glyph. [`caret_window`] composes the visible cells with the caret `▏` (`README_UI.md` §5.11).

/// The text-editing caret (`README_UI.md` §5.11).
pub(crate) const CARET: char = '▏';

/// Editing keys besides typed characters and Backspace. `Select*` extend the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKey {
    Left,
    Right,
    Home,
    End,
    /// Deletes the character after the caret.
    Delete,
    SelectAll,
    SelectLeft,
    SelectRight,
    SelectHome,
    SelectEnd,
}

/// Characters, caret index `0..=len` and selection anchor (`anchor == caret`: no selection).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TextEdit {
    text: Vec<char>,
    caret: usize,
    anchor: usize,
}

impl TextEdit {
    /// `text` with the caret at its end.
    pub(crate) fn new(text: &str) -> Self {
        let text: Vec<char> = text.chars().collect();
        let end = text.len();
        Self {
            text,
            caret: end,
            anchor: end,
        }
    }

    pub(crate) fn chars(&self) -> &[char] {
        &self.text
    }

    pub(crate) fn string(&self) -> String {
        self.text.iter().collect()
    }

    pub(crate) const fn caret(&self) -> usize {
        self.caret
    }

    /// Selected range `lo..hi` (empty without a selection).
    pub(crate) fn selection(&self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }

    /// Replaces the selection (or inserts at the caret) with `c` if `accept` takes the result.
    pub(crate) fn insert(&mut self, c: char, accept: impl Fn(&str) -> bool) -> bool {
        self.replace(Some(c), accept)
    }

    /// Deletes the selection, else the character before the caret.
    pub(crate) fn backspace(&mut self, accept: impl Fn(&str) -> bool) -> bool {
        if self.selection().0 == self.selection().1 && self.caret > 0 {
            self.anchor = self.caret - 1;
        }
        self.replace(None, accept)
    }

    /// Applies a navigation or `Delete` key. Returns `true` if anything changed.
    pub(crate) fn apply(&mut self, key: EditKey, accept: impl Fn(&str) -> bool) -> bool {
        let before = self.clone();
        let (lo, hi) = self.selection();
        let len = self.text.len();
        match key {
            EditKey::Left if lo != hi => self.caret = lo,
            EditKey::Right if lo != hi => self.caret = hi,
            EditKey::Left | EditKey::SelectLeft => {
                self.caret = self.caret.saturating_sub(1);
            }
            EditKey::Right | EditKey::SelectRight => self.caret = (self.caret + 1).min(len),
            EditKey::Home | EditKey::SelectHome => self.caret = 0,
            EditKey::End | EditKey::SelectEnd => self.caret = len,
            EditKey::SelectAll => {
                self.anchor = 0;
                self.caret = len;
                return *self != before;
            }
            EditKey::Delete => {
                if lo == hi && self.caret < len {
                    self.anchor = self.caret + 1;
                }
                return self.replace(None, accept);
            }
        }
        if !matches!(
            key,
            EditKey::SelectLeft | EditKey::SelectRight | EditKey::SelectHome | EditKey::SelectEnd
        ) {
            self.anchor = self.caret;
        }
        *self != before
    }

    fn replace(&mut self, with: Option<char>, accept: impl Fn(&str) -> bool) -> bool {
        let (lo, hi) = self.selection();
        let mut next = self.text.clone();
        next.splice(lo..hi, with);
        if !accept(&next.iter().collect::<String>()) {
            self.anchor = self.caret;
            return false;
        }
        let changed = next != self.text || self.caret != self.anchor;
        self.caret = lo + usize::from(with.is_some());
        self.anchor = self.caret;
        self.text = next;
        changed
    }
}

/// The `n` visible cells of `body` followed by the static `suffix`. With a `caret` index the
/// caret `▏` takes its own cell and the window starts at `max(0, c + 1 − n)` (§5.11): the
/// suffix is clipped before the text, the text before the caret, whole cells only.
pub(crate) fn caret_window(body: &[char], suffix: &str, caret: Option<usize>, n: usize) -> String {
    let mut shown: Vec<char> = Vec::with_capacity(body.len() + 1);
    let start = match caret {
        Some(c) => {
            let c = c.min(body.len());
            shown.extend(&body[..c]);
            shown.push(CARET);
            shown.extend(&body[c..]);
            (c + 1).saturating_sub(n)
        }
        None => {
            shown.extend(body);
            0
        }
    };
    shown.extend(suffix.chars());
    shown.into_iter().skip(start).take(n).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn any(_: &str) -> bool {
        true
    }

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn window_empty_middle_and_suffix_clipping() {
        assert_eq!(caret_window(&[], " mm", Some(0), 13), "▏ mm");
        assert_eq!(caret_window(&chars("12"), " mm", Some(1), 13), "1▏2 mm");
        assert_eq!(caret_window(&chars("12"), " mm", None, 13), "12 mm");
        // 10 characters + caret + " mm" > 13: the suffix goes first.
        assert_eq!(
            caret_window(&chars("12345678.9"), " mm", Some(0), 13),
            "▏12345678.9 m"
        );
        assert_eq!(
            caret_window(&chars("12345678.9"), " mm", Some(10), 13),
            "12345678.9▏ m"
        );
    }

    #[test]
    fn window_scrolls_for_n_minus_1_n_and_n_plus_1_characters() {
        let n = 5;
        for len in [n - 1, n, n + 1] {
            let body: Vec<char> = "abcdefgh".chars().take(len).collect();
            let at_start = caret_window(&body, "", Some(0), n);
            assert_eq!(at_start.chars().next(), Some(CARET));
            assert_eq!(at_start.chars().count(), n);
            let at_end = caret_window(&body, "", Some(len), n);
            assert_eq!(at_end.chars().filter(|&c| c == CARET).count(), 1);
            assert_eq!(at_end.chars().last(), Some(CARET));
        }
        assert_eq!(caret_window(&chars("abcd"), "", Some(4), 5), "abcd▏");
        assert_eq!(caret_window(&chars("abcde"), "", Some(5), 5), "bcde▏");
        assert_eq!(caret_window(&chars("abcdef"), "", Some(6), 5), "cdef▏");
        assert_eq!(caret_window(&chars("abcdef"), "", Some(0), 5), "▏abcd");
    }

    #[test]
    fn typing_navigation_and_selection() {
        let mut t = TextEdit::new("ab");
        assert!(t.insert('c', any));
        assert_eq!((t.string().as_str(), t.caret()), ("abc", 3));
        assert!(t.apply(EditKey::Home, any));
        assert!(!t.apply(EditKey::Left, any), "already at the start");
        assert!(t.apply(EditKey::Delete, any));
        assert_eq!(t.string(), "bc");
        assert!(t.apply(EditKey::SelectEnd, any));
        assert_eq!(t.selection(), (0, 2));
        assert!(t.insert('x', any));
        assert_eq!((t.string().as_str(), t.caret()), ("x", 1));
        assert!(t.apply(EditKey::SelectAll, any));
        assert!(t.backspace(any));
        assert_eq!((t.string().as_str(), t.caret()), ("", 0));
        assert!(!t.backspace(any));
    }

    #[test]
    fn selection_collapses_and_rejected_edits_change_nothing() {
        let mut t = TextEdit::new("abc");
        t.apply(EditKey::SelectLeft, any);
        t.apply(EditKey::SelectLeft, any);
        assert_eq!(t.selection(), (1, 3));
        assert!(t.apply(EditKey::Left, any));
        assert_eq!((t.caret(), t.selection()), (1, (1, 1)));
        assert!(!t.insert('z', |s| s.len() <= 3));
        assert_eq!(t.string(), "abc");
        assert!(t.apply(EditKey::SelectAll, any));
        assert!(!t.apply(EditKey::SelectAll, any));
        assert!(t.apply(EditKey::Right, any));
        assert_eq!(t.caret(), 3);
    }
}
