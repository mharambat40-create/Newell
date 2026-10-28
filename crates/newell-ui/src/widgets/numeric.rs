// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Compact numeric text editing. Accepted quantities belong to the application.
use super::edit::{EditKey, TextEdit};
use crate::{
    NumericDirection, NumericEdit, NumericFieldDescriptor, NumericFieldView, NumericUnit, WidgetId,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NumericView {
    pub(crate) is_placeholder: bool,
    pub(crate) prefix: &'static str,
    pub(crate) body: Vec<char>,
    pub(crate) suffix: String,
    pub(crate) caret: Option<usize>,
}
fn suffix(d: NumericFieldDescriptor) -> &'static str {
    match d.unit {
        NumericUnit::Millimetres => " mm",
        NumericUnit::Degrees => " deg",
        NumericUnit::Unitless => "",
    }
}
pub(crate) fn default_view(d: NumericFieldDescriptor) -> NumericView {
    NumericView {
        is_placeholder: true,
        prefix: if d.directional { "+|" } else { "" },
        body: d.default_magnitude.chars().collect(),
        suffix: suffix(d).to_owned(),
        caret: None,
    }
}
fn accepts(d: NumericFieldDescriptor, text: &str) -> bool {
    let (i, f) = text.split_once('.').unwrap_or((text, ""));
    i.bytes().all(|c| c.is_ascii_digit())
        && f.bytes().all(|c| c.is_ascii_digit())
        && i.len() <= usize::from(d.integer_digits)
        && f.len() <= 1
}
fn normalize(text: &str) -> Option<String> {
    let (i, f) = text.split_once('.').unwrap_or((text, ""));
    if i.is_empty() && f.is_empty() {
        return None;
    }
    let i = i.trim_start_matches('0');
    Some(format!(
        "{}.{}",
        if i.is_empty() { "0" } else { i },
        if f.is_empty() { "0" } else { f }
    ))
}
fn direction(reverse: bool) -> NumericDirection {
    if reverse {
        NumericDirection::Reverse
    } else {
        NumericDirection::Forward
    }
}
/// Presentation cache only, including an optimistic view while an emitted candidate resolves.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    initial_placeholder: bool,
    id: u32,
    negative: bool,
    magnitude: String,
    formula: String,
}
/// Raw editor and its initial rounded view, used solely to distinguish explicit changes.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Active {
    initial_placeholder: bool,
    text_mutated: bool,
    id: u32,
    descriptor: NumericFieldDescriptor,
    negative: bool,
    initial_negative: bool,
    initial: String,
    edit: TextEdit,
}
/// Per-operation text state and candidate queue. The shell supplies session identities;
/// canonical quantities and accepted draft authority remain in the application.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct NumericFields {
    entries: Vec<Entry>,
    saved: Vec<Entry>,
    active: Option<Active>,
    pending: Vec<(WidgetId, NumericEdit)>,
}
impl NumericFields {
    pub(crate) fn editing(&self) -> Option<WidgetId> {
        self.active.as_ref().map(|a| WidgetId::new(a.id))
    }
    fn entry(&self, id: u32) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }
    fn entry_mut(&mut self, id: u32, d: NumericFieldDescriptor) -> &mut Entry {
        let at = match self.entries.iter().position(|e| e.id == id) {
            Some(at) => at,
            None => {
                self.entries.push(Entry {
                    initial_placeholder: true,
                    id,
                    negative: false,
                    magnitude: d.default_magnitude.to_owned(),
                    formula: String::new(),
                });
                self.entries.len() - 1
            }
        };
        &mut self.entries[at]
    }
    pub(crate) fn begin(&mut self, id: WidgetId, d: NumericFieldDescriptor) {
        if self.editing() == Some(id) {
            return;
        }
        self.commit();
        let (negative, magnitude) = self.entry(id.get()).map_or_else(
            || (false, d.default_magnitude.to_owned()),
            |e| (e.negative, e.magnitude.clone()),
        );
        let initial_placeholder = self.entry(id.get()).is_none_or(|e| e.initial_placeholder);
        self.active = Some(Active {
            initial_placeholder,
            text_mutated: false,
            id: id.get(),
            descriptor: d,
            negative,
            initial_negative: negative,
            initial: magnitude.clone(),
            edit: TextEdit::new(if initial_placeholder { "" } else { &magnitude }),
        });
    }
    pub(crate) fn toggle_sign(&mut self, id: WidgetId, d: NumericFieldDescriptor) -> bool {
        if !d.directional {
            return false;
        }
        if let Some(a) = self.active.as_mut().filter(|a| a.id == id.get()) {
            a.negative = !a.negative;
            return true;
        }
        let e = self.entry_mut(id.get(), d);
        e.negative = !e.negative;
        let reverse = e.negative;
        self.pending
            .push((id, NumericEdit::Direction(direction(reverse))));
        true
    }
    /// Ends the raw edit and queues only an explicit change or incomplete acceptance attempt.
    /// Lexical normalization here never reconstructs an untouched canonical quantity.
    pub(crate) fn commit(&mut self) {
        let Some(a) = self.active.take() else { return };
        let raw = a.edit.string();
        let unchanged = if a.initial_placeholder {
            !a.text_mutated
        } else {
            raw == a.initial
        };
        if unchanged && a.negative == a.initial_negative {
            self.entry_mut(a.id, a.descriptor);
            return;
        }
        let edit = if unchanged {
            NumericEdit::Direction(direction(a.negative))
        } else if let Some(text) = normalize(&raw) {
            NumericEdit::Literal {
                magnitude: text,
                direction: direction(a.negative),
            }
        } else {
            NumericEdit::Incomplete(raw)
        };
        match &edit {
            NumericEdit::Literal { magnitude, .. } => {
                let e = self.entry_mut(a.id, a.descriptor);
                e.magnitude = magnitude.clone();
                e.negative = a.negative;
                e.formula.clear();
                e.initial_placeholder = false;
            }
            NumericEdit::Direction(_) => self.entry_mut(a.id, a.descriptor).negative = a.negative,
            _ => {}
        }
        self.pending.push((WidgetId::new(a.id), edit));
    }
    pub(crate) fn discard(&mut self) {
        self.active = None;
    }
    pub(crate) fn typed(&mut self, c: char) -> bool {
        let Some(a) = self.active.as_mut() else {
            return false;
        };
        match c {
            '+' | '-' if a.descriptor.directional => {
                let reverse = c == '-';
                std::mem::replace(&mut a.negative, reverse) != reverse
            }
            '0'..='9' | '.' => {
                let d = a.descriptor;
                let before = a.edit.chars().to_vec();
                let changed = a.edit.insert(c, |s| accepts(d, s));
                a.text_mutated |= a.edit.chars() != before;
                changed
            }
            _ => false,
        }
    }
    pub(crate) fn backspace(&mut self) -> bool {
        let Some(a) = self.active.as_mut() else {
            return false;
        };
        let d = a.descriptor;
        let before = a.edit.chars().to_vec();
        let changed = a.edit.backspace(|s| accepts(d, s));
        a.text_mutated |= a.edit.chars() != before;
        changed
    }
    pub(crate) fn edit_key(&mut self, k: EditKey) -> bool {
        let Some(a) = self.active.as_mut() else {
            return false;
        };
        let d = a.descriptor;
        let before = a.edit.chars().to_vec();
        let changed = a.edit.apply(k, |s| accepts(d, s));
        a.text_mutated |= a.edit.chars() != before;
        changed
    }
    pub(crate) fn view(&self, id: WidgetId, d: NumericFieldDescriptor) -> Option<NumericView> {
        let e = self.entry(id.get());
        let a = self.active.as_ref().filter(|a| a.id == id.get());
        if e.is_none() && a.is_none() {
            return None;
        }
        let reverse = a.map_or_else(|| e.is_some_and(|e| e.negative), |a| a.negative);
        Some(NumericView {
            is_placeholder: a.is_none() && e.is_none_or(|e| e.initial_placeholder),
            prefix: if !d.directional {
                ""
            } else if reverse {
                "-|"
            } else {
                "+|"
            },
            body: a.map_or_else(
                || {
                    e.map_or(d.default_magnitude, |e| e.magnitude.as_str())
                        .chars()
                        .collect()
                },
                |a| a.edit.chars().to_vec(),
            ),
            suffix: suffix(d).to_owned(),
            caret: a.map(|a| a.edit.caret()),
        })
    }
    pub(crate) fn formula(&self, id: WidgetId) -> &str {
        self.entry(id.get()).map_or("", |e| e.formula.as_str())
    }
    pub(crate) fn set_formula(&mut self, id: WidgetId, d: NumericFieldDescriptor, text: &str) {
        let e = self.entry_mut(id.get(), d);
        e.formula = text.to_owned();
        if !text.trim().is_empty() {
            e.initial_placeholder = false;
        }
        self.pending
            .push((id, NumericEdit::Formula(text.to_owned())));
    }
    /// Refreshes the cached accepted view without replacing an active raw editor.
    pub(crate) fn install(
        &mut self,
        id: WidgetId,
        d: NumericFieldDescriptor,
        v: &NumericFieldView,
    ) {
        let e = self.entry_mut(id.get(), d);
        e.magnitude = v.magnitude.clone();
        e.negative = v.direction == NumericDirection::Reverse;
        e.formula = v.formula.clone();
        e.initial_placeholder = v.initial_placeholder;
    }
    pub(crate) fn take_edits(&mut self) -> Vec<(WidgetId, NumericEdit)> {
        std::mem::take(&mut self.pending)
    }
    pub(crate) fn confirm(&mut self) {
        self.commit();
        self.saved = self.entries.clone();
    }
    pub(crate) fn cancel(&mut self) {
        self.active = None;
        self.entries = self.saved.clone();
        self.pending.clear();
    }
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    const MM: NumericFieldDescriptor = crate::numeric::field(crate::ActionOperation::Extrude, 4);
    const DEG: NumericFieldDescriptor = crate::numeric::field(crate::ActionOperation::Revolve, 6);
    const PLAIN: NumericFieldDescriptor = crate::numeric::field(crate::ActionOperation::Loft, 5);

    fn id(v: u32) -> WidgetId {
        WidgetId::new(v)
    }

    fn type_str(f: &mut NumericFields, s: &str) -> Vec<bool> {
        s.chars().map(|c| f.typed(c)).collect()
    }

    fn shown(f: &NumericFields, at: u32, mock: NumericFieldDescriptor) -> String {
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
        assert_eq!(shown(&f, 2, DEG), "+|1234567.8 deg");
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
        assert_eq!(shown(&f, 1, MM), "-| mm");
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
    #[test]
    fn placeholder_entry_is_empty_idempotent_and_never_a_candidate() {
        let mut f = NumericFields::default();
        for _ in 0..2 {
            f.begin(id(1), MM);
            let v = f.view(id(1), MM).unwrap();
            assert!(v.body.is_empty());
            assert_eq!(v.caret, Some(0));
            assert!(!v.is_placeholder);
            f.edit_key(EditKey::SelectAll);
            f.edit_key(EditKey::Left);
            assert!(!f.typed('x'));
            f.begin(id(1), MM);
            f.commit();
            assert!(f.take_edits().is_empty());
            assert!(f.view(id(1), MM).unwrap().is_placeholder);
        }
        f.begin(id(1), MM);
        f.typed('7');
        f.begin(id(1), MM);
        assert_eq!(shown(&f, 1, MM), "+|7 mm");
        f.discard();
        assert!(f.view(id(1), MM).unwrap().is_placeholder);
    }

    #[test]
    fn explicit_zero_is_real_but_deleted_draft_is_incomplete() {
        for zero in ["0", "0.0", ".0"] {
            let mut f = NumericFields::default();
            f.begin(id(1), MM);
            type_str(&mut f, zero);
            f.commit();
            assert!(
                matches!(&f.take_edits()[0].1, NumericEdit::Literal { magnitude, .. } if magnitude == "0.0")
            );
            assert!(!f.view(id(1), MM).unwrap().is_placeholder);
            f.begin(id(1), MM);
            assert_eq!(shown(&f, 1, MM), "+|0.0 mm");
            f.commit();
            assert!(f.take_edits().is_empty());
            f.begin(id(1), MM);
            f.edit_key(EditKey::SelectAll);
            f.backspace();
            f.commit();
            assert_eq!(f.take_edits()[0].1, NumericEdit::Incomplete(String::new()));
            assert_eq!(shown(&f, 1, MM), "+|0.0 mm");
        }
        let mut f = NumericFields::default();
        f.begin(id(1), MM);
        f.typed('1');
        f.backspace();
        f.commit();
        assert_eq!(f.take_edits()[0].1, NumericEdit::Incomplete(String::new()));
    }

    #[test]
    fn direction_only_and_formula_preserve_placeholder_provenance() {
        let mut f = NumericFields::default();
        f.begin(id(1), MM);
        f.typed('-');
        f.commit();
        assert_eq!(
            f.take_edits()[0].1,
            NumericEdit::Direction(NumericDirection::Reverse)
        );
        assert!(f.view(id(1), MM).unwrap().is_placeholder);
        f.set_formula(id(1), MM, "0mm");
        assert!(!f.view(id(1), MM).unwrap().is_placeholder);
        f.take_edits();
        f.begin(id(1), MM);
        f.commit();
        assert!(f.take_edits().is_empty());
        assert_eq!(f.formula(id(1)), "0mm");
    }
}
