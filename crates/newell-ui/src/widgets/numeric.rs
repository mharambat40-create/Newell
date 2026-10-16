// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Editable numeric fields of an Action Box (M2.23): value, direction, limits and the edit
//! session. Values are text; nothing is parsed into a CAD quantity or evaluated.
//!
//! A field's mock text (`+|0.0 mm`, `0.0 deg`, `0.0`) gives its direction prefix, magnitude
//! and unit. The magnitude is `digits[.digit]` with at most 8 integer digits (7 for `deg`),
//! so `[+|12345678.9 mm☰]` and `[+|1234567.8 deg☰]` fill the 13-cell text area exactly.

use super::edit::{EditKey, TextEdit};
use crate::WidgetId;

/// What a field shows, split into the parts that are drawn separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NumericView {
    /// `+|`, `-|` or empty (field without direction).
    pub(crate) prefix: &'static str,
    pub(crate) body: Vec<char>,
    /// Static text after the value (` mm`), or empty.
    pub(crate) suffix: String,
    /// Caret index into `body` while the field is edited.
    pub(crate) caret: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Spec {
    prefixed: bool,
    negative: bool,
    magnitude: String,
    suffix: String,
    int_digits: usize,
}

impl Spec {
    fn parse(mock: &str) -> Self {
        let (prefixed, negative, rest) = if let Some(rest) = mock.strip_prefix("+|") {
            (true, false, rest)
        } else if let Some(rest) = mock.strip_prefix("-|") {
            (true, true, rest)
        } else {
            (false, false, mock)
        };
        let (magnitude, unit) = rest.split_once(' ').unwrap_or((rest, ""));
        Self {
            prefixed,
            negative,
            magnitude: magnitude.to_owned(),
            suffix: if unit.is_empty() {
                String::new()
            } else {
                format!(" {unit}")
            },
            int_digits: if unit == "deg" { 7 } else { 8 },
        }
    }

    fn accepts(&self, text: &str) -> bool {
        let (int, frac) = match text.split_once('.') {
            Some((i, f)) => (i, f),
            None => (text, ""),
        };
        let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
        digits(int) && digits(frac) && int.len() <= self.int_digits && frac.len() <= 1
    }
}

/// `5` → `5.0`, `.5` → `0.5`, `007.5` → `7.5`; `None` for no digit at all.
fn normalize(text: &str) -> Option<String> {
    let (int, frac) = text.split_once('.').unwrap_or((text, ""));
    if int.is_empty() && frac.is_empty() {
        return None;
    }
    let int = int.trim_start_matches('0');
    let int = if int.is_empty() { "0" } else { int };
    let frac = if frac.is_empty() { "0" } else { frac };
    Some(format!("{int}.{frac}"))
}

fn prefix_of(spec: &Spec, negative: bool) -> &'static str {
    match (spec.prefixed, negative) {
        (false, _) => "",
        (true, false) => "+|",
        (true, true) => "-|",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    id: u32,
    negative: bool,
    magnitude: String,
    formula: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Active {
    id: u32,
    spec: Spec,
    negative: bool,
    edit: TextEdit,
}

/// The edited values and formula drafts of one Action Box, keyed by field id, plus the edit
/// session. `saved` is what OK last kept, so Cancel can return to it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct NumericFields {
    entries: Vec<Entry>,
    saved: Vec<Entry>,
    active: Option<Active>,
}

impl NumericFields {
    pub(crate) fn editing(&self) -> Option<WidgetId> {
        self.active.as_ref().map(|a| WidgetId::new(a.id))
    }

    fn entry(&self, id: u32) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    fn entry_mut(&mut self, id: u32, spec: &Spec) -> &mut Entry {
        let at = self
            .entries
            .iter()
            .position(|e| e.id == id)
            .unwrap_or_else(|| {
                self.entries.push(Entry {
                    id,
                    negative: spec.negative,
                    magnitude: spec.magnitude.clone(),
                    formula: String::new(),
                });
                self.entries.len() - 1
            });
        &mut self.entries[at]
    }

    /// Starts editing field `id` (caret at the end). `mock` is the field's default text.
    /// Ends any other edit first; editing the same field again changes nothing.
    pub(crate) fn begin(&mut self, id: WidgetId, mock: &str) {
        if self.editing() == Some(id) {
            return;
        }
        self.commit();
        let spec = Spec::parse(mock);
        let (negative, magnitude) = self.entry(id.get()).map_or_else(
            || (spec.negative, spec.magnitude.clone()),
            |e| (e.negative, e.magnitude.clone()),
        );
        self.active = Some(Active {
            id: id.get(),
            edit: TextEdit::new(&magnitude),
            negative,
            spec,
        });
    }

    /// Keeps the edit: the text is normalised (`5` → `5.0`); text with no digit restores the
    /// previous value.
    pub(crate) fn commit(&mut self) {
        let Some(active) = self.active.take() else {
            return;
        };
        let entry = self.entry_mut(active.id, &active.spec);
        entry.negative = active.negative;
        if let Some(text) = normalize(&active.edit.string()) {
            entry.magnitude = text;
        }
    }

    /// Ends the edit and keeps the previous value.
    pub(crate) fn discard(&mut self) {
        self.active = None;
    }

    /// A typed character: a digit or `.` edits the magnitude; `+` / `-` set the direction of a
    /// field that has one. Everything else, and anything over the limits, is refused.
    pub(crate) fn typed(&mut self, c: char) -> bool {
        let Some(a) = self.active.as_mut() else {
            return false;
        };
        match c {
            '+' | '-' if a.spec.prefixed => {
                let negative = c == '-';
                std::mem::replace(&mut a.negative, negative) != negative
            }
            '0'..='9' | '.' => {
                let spec = &a.spec;
                a.edit.insert(c, |s| spec.accepts(s))
            }
            _ => false,
        }
    }

    pub(crate) fn backspace(&mut self) -> bool {
        let Some(a) = self.active.as_mut() else {
            return false;
        };
        let spec = &a.spec;
        a.edit.backspace(|s| spec.accepts(s))
    }

    pub(crate) fn edit_key(&mut self, key: EditKey) -> bool {
        let Some(a) = self.active.as_mut() else {
            return false;
        };
        let spec = &a.spec;
        a.edit.apply(key, |s| spec.accepts(s))
    }

    /// What field `id` shows, or `None` while it was never edited (the mock text applies).
    pub(crate) fn view(&self, id: WidgetId, mock: &str) -> Option<NumericView> {
        if let Some(a) = self.active.as_ref().filter(|a| a.id == id.get()) {
            return Some(NumericView {
                prefix: prefix_of(&a.spec, a.negative),
                body: a.edit.chars().to_vec(),
                suffix: a.spec.suffix.clone(),
                caret: Some(a.edit.caret()),
            });
        }
        let e = self.entry(id.get())?;
        let spec = Spec::parse(mock);
        Some(NumericView {
            prefix: prefix_of(&spec, e.negative),
            body: e.magnitude.chars().collect(),
            suffix: spec.suffix,
            caret: None,
        })
    }

    /// The formula draft kept for field `id` (empty if none).
    pub(crate) fn formula(&self, id: WidgetId) -> &str {
        self.entry(id.get()).map_or("", |e| e.formula.as_str())
    }

    pub(crate) fn set_formula(&mut self, id: WidgetId, mock: &str, text: &str) {
        let spec = Spec::parse(mock);
        self.entry_mut(id.get(), &spec).formula = text.to_owned();
    }

    /// OK: the values become the ones Cancel returns to.
    pub(crate) fn confirm(&mut self) {
        self.commit();
        self.saved = self.entries.clone();
    }

    /// Cancel: back to the values of the last OK (or the defaults).
    pub(crate) fn cancel(&mut self) {
        self.active = None;
        self.entries = self.saved.clone();
    }

    /// The box reopens with its defaults.
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: &str = "+|0.0 mm";
    const DEG: &str = "-|0.0 deg";
    const PLAIN: &str = "0.0";

    fn id(v: u32) -> WidgetId {
        WidgetId::new(v)
    }

    fn type_str(f: &mut NumericFields, s: &str) -> Vec<bool> {
        s.chars().map(|c| f.typed(c)).collect()
    }

    fn shown(f: &NumericFields, at: u32, mock: &str) -> String {
        let v = f.view(id(at), mock).unwrap();
        format!(
            "{}{}{}",
            v.prefix,
            v.body.iter().collect::<String>(),
            v.suffix
        )
    }

    fn select_all_and_type(f: &mut NumericFields, s: &str) {
        f.edit_key(EditKey::SelectAll);
        type_str(f, s);
    }

    #[test]
    fn mm_accepts_exactly_the_reference_limit() {
        let mut f = NumericFields::default();
        f.begin(id(1), MM);
        select_all_and_type(&mut f, "12345678.9");
        assert_eq!(f.view(id(1), MM).unwrap().body.len(), 10);
        assert!(!f.typed('1'), "no second decimal");
        f.commit();
        assert_eq!(shown(&f, 1, MM), "+|12345678.9 mm");
        f.begin(id(1), MM);
        f.edit_key(EditKey::SelectAll);
        assert!(
            type_str(&mut f, "123456789").last() == Some(&false),
            "9th integer digit"
        );
        assert_eq!(
            f.view(id(1), MM).unwrap().body.iter().collect::<String>(),
            "12345678"
        );
    }

    #[test]
    fn deg_limit_is_seven_integer_digits() {
        let mut f = NumericFields::default();
        f.begin(id(2), DEG);
        select_all_and_type(&mut f, "1234567.8");
        f.commit();
        assert_eq!(shown(&f, 2, DEG), "-|1234567.8 deg");
        f.begin(id(2), DEG);
        f.edit_key(EditKey::SelectAll);
        assert_eq!(type_str(&mut f, "12345678").last(), Some(&false));
    }

    #[test]
    fn only_digits_and_one_decimal_point_are_accepted() {
        let mut f = NumericFields::default();
        f.begin(id(1), PLAIN);
        f.edit_key(EditKey::SelectAll);
        assert_eq!(
            type_str(&mut f, "1a,2e-.3."),
            [true, false, false, true, false, false, true, true, false]
        );
        assert_eq!(
            f.view(id(1), PLAIN)
                .unwrap()
                .body
                .iter()
                .collect::<String>(),
            "12.3"
        );
        // The mock `0.0` selected and replaced by `3` normalises on commit.
        f.edit_key(EditKey::SelectAll);
        f.typed('3');
        f.commit();
        assert_eq!(shown(&f, 1, PLAIN), "3.0");
    }

    #[test]
    fn sign_changes_direction_not_the_magnitude() {
        let mut f = NumericFields::default();
        f.begin(id(1), MM);
        assert!(f.typed('-'));
        assert!(!f.typed('-'), "already negative");
        assert_eq!(shown(&f, 1, MM), "-|0.0 mm");
        assert!(f.typed('+'));
        f.commit();
        assert_eq!(shown(&f, 1, MM), "+|0.0 mm");
        // A field without a direction refuses signs.
        f.begin(id(3), PLAIN);
        assert!(!f.typed('-') && !f.typed('+'));
    }

    #[test]
    fn decimal_commit_normalises_and_empty_restores() {
        let mut f = NumericFields::default();
        for (typed, shown_after) in [("5", "5.0"), (".5", "0.5"), ("5.", "5.0"), ("007", "7.0")] {
            f.begin(id(1), MM);
            select_all_and_type(&mut f, typed);
            f.commit();
            assert_eq!(shown(&f, 1, MM), format!("+|{shown_after} mm"));
        }
        f.begin(id(1), MM);
        f.edit_key(EditKey::SelectAll);
        assert!(f.backspace());
        f.commit();
        assert_eq!(shown(&f, 1, MM), "+|7.0 mm", "no digits: previous value");
    }

    #[test]
    fn caret_and_editing_keys_work_inside_the_text() {
        let mut f = NumericFields::default();
        f.begin(id(1), MM);
        f.edit_key(EditKey::SelectAll);
        type_str(&mut f, "123");
        f.edit_key(EditKey::Left);
        assert_eq!(f.view(id(1), MM).unwrap().caret, Some(2));
        assert!(f.typed('.'));
        assert_eq!(
            f.view(id(1), MM).unwrap().body.iter().collect::<String>(),
            "12.3"
        );
        assert!(!f.typed('.'), "one point only");
        f.edit_key(EditKey::Home);
        assert!(f.edit_key(EditKey::Delete));
        assert!(!f.backspace(), "nothing before the caret");
        f.edit_key(EditKey::End);
        assert!(f.backspace());
        assert_eq!(
            f.view(id(1), MM).unwrap().body.iter().collect::<String>(),
            "2."
        );
    }

    #[test]
    fn discard_cancel_confirm_and_reset() {
        let mut f = NumericFields::default();
        f.begin(id(1), MM);
        select_all_and_type(&mut f, "9");
        f.discard();
        assert!(f.view(id(1), MM).is_none(), "never committed");
        f.begin(id(1), MM);
        select_all_and_type(&mut f, "4");
        f.confirm();
        f.begin(id(1), MM);
        select_all_and_type(&mut f, "8");
        f.commit();
        assert_eq!(shown(&f, 1, MM), "+|8.0 mm");
        f.cancel();
        assert_eq!(shown(&f, 1, MM), "+|4.0 mm");
        f.reset();
        assert_eq!(f, NumericFields::default());
    }

    #[test]
    fn formula_drafts_are_independent_per_field() {
        let mut f = NumericFields::default();
        f.set_formula(id(1), MM, "a");
        f.set_formula(id(2), DEG, "b");
        assert_eq!(
            (f.formula(id(1)), f.formula(id(2)), f.formula(id(3))),
            ("a", "b", "")
        );
        assert!(f.view(id(1), MM).is_some(), "defaults keep the mock look");
        assert_eq!(shown(&f, 1, MM), "+|0.0 mm");
        f.cancel();
        assert_eq!(f.formula(id(1)), "");
    }

    #[test]
    fn beginning_another_field_commits_the_first() {
        let mut f = NumericFields::default();
        f.begin(id(1), MM);
        select_all_and_type(&mut f, "6");
        f.begin(id(2), MM);
        assert_eq!(f.editing(), Some(id(2)));
        assert_eq!(shown(&f, 1, MM), "+|6.0 mm");
        assert!(f.view(id(2), MM).unwrap().caret.is_some());
        assert!(f.view(id(1), MM).unwrap().caret.is_none());
    }
}
