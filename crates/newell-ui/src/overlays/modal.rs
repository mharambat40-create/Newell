// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Shared vocabulary of the modal overlays (M2.12): keys, lifecycle and focus stepping.

use crate::WidgetId;

/// Keys a modal reacts to (already translated from window events).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalKey {
    /// Moves focus forward.
    Tab,
    /// Moves focus backward.
    BackTab,
    /// Moves focus backward (list navigation).
    Up,
    /// Moves focus forward (list navigation).
    Down,
    /// Activates the focused control.
    Enter,
    /// Cancels the modal.
    Escape,
    /// Deletes the character before the caret.
    Backspace,
    /// A typed character.
    Char(char),
}

/// Lifecycle of a modal. `Confirmed` and `Cancelled` record the last outcome until the modal
/// is opened again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModalStatus {
    /// Never opened, or dismissed without an outcome.
    #[default]
    Closed,
    /// Shown and taking input.
    Open,
    /// OK was activated; the draft became the applied value.
    Confirmed,
    /// Cancel or Escape was activated; the draft was discarded.
    Cancelled,
}

/// The control after `current` in `ring` (or before it when `forward` is `false`), wrapping
/// at both ends. A `current` that is not in the ring starts from the first entry going
/// forward and from the last going backward. An empty ring yields `None`.
pub(crate) fn step_focus(
    ring: &[WidgetId],
    current: Option<WidgetId>,
    forward: bool,
) -> Option<WidgetId> {
    let len = ring.len();
    let at = current.and_then(|c| ring.iter().position(|&r| r == c));
    let index = match (at, forward) {
        (Some(i), true) => (i + 1) % len,
        (Some(i), false) => (i + len - 1) % len,
        (None, true) => 0,
        (None, false) => len.checked_sub(1)?,
    };
    ring.get(index).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(v: u32) -> WidgetId {
        WidgetId::new(v)
    }

    #[test]
    fn forward_and_backward_wrap_at_both_ends() {
        let ring = [id(1), id(2), id(3)];
        assert_eq!(step_focus(&ring, Some(id(1)), true), Some(id(2)));
        assert_eq!(step_focus(&ring, Some(id(3)), true), Some(id(1)));
        assert_eq!(step_focus(&ring, Some(id(1)), false), Some(id(3)));
        assert_eq!(step_focus(&ring, Some(id(2)), false), Some(id(1)));
    }

    #[test]
    fn unknown_or_missing_focus_starts_at_the_matching_end() {
        let ring = [id(1), id(2)];
        assert_eq!(step_focus(&ring, None, true), Some(id(1)));
        assert_eq!(step_focus(&ring, None, false), Some(id(2)));
        assert_eq!(step_focus(&ring, Some(id(9)), true), Some(id(1)));
        assert_eq!(step_focus(&ring, Some(id(9)), false), Some(id(2)));
    }

    #[test]
    fn single_and_empty_rings() {
        assert_eq!(step_focus(&[id(7)], Some(id(7)), true), Some(id(7)));
        assert_eq!(step_focus(&[id(7)], Some(id(7)), false), Some(id(7)));
        assert_eq!(step_focus(&[], None, true), None);
        assert_eq!(step_focus(&[], Some(id(1)), false), None);
    }
}
