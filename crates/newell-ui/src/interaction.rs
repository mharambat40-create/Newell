// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Logical interaction state (ARCHITECTURE.md §3.4, §3.5).
//!
//! Plain `Copy` values owned by the Controller. There is no widget object, callback or
//! registry, and nothing here is stored in the rendered [`crate::CellGrid`].

use crate::{HitTestResult, WidgetId};

/// Transient pointer / keyboard state: at most one widget per role.
///
/// - `hovered`: the widget currently under the pointer, as resolved by [`crate::hit_test()`]
///   (so shared-border routing is already applied).
/// - `focused`: the widget that owns keyboard focus.
/// - `pressed`: the widget currently held down by pointer activation.
///
/// Changing hover never changes the rendered cell grid: it only needs the hovered ID
/// to reach the shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InteractionState {
    pub hovered: Option<WidgetId>,
    pub focused: Option<WidgetId>,
    pub pressed: Option<WidgetId>,
}

impl InteractionState {
    /// Sets (or clears, with `None`) hover. Returns `true` if the value changed.
    pub fn set_hovered(&mut self, widget: Option<WidgetId>) -> bool {
        replace(&mut self.hovered, widget)
    }

    /// Updates hover from a pointer hit-test. A pointer outside the grid (`None`) or over a
    /// cell without an owner clears hover. Returns `true` if the hovered widget changed.
    pub fn set_hovered_from_hit(&mut self, hit: Option<HitTestResult>) -> bool {
        self.set_hovered(hit.and_then(|h| h.widget_id))
    }

    /// Sets (or clears, with `None`) keyboard focus. Returns `true` if the value changed.
    pub fn set_focused(&mut self, widget: Option<WidgetId>) -> bool {
        replace(&mut self.focused, widget)
    }

    /// Starts a pointer press on `widget`. A disabled widget is not activatable: the press
    /// is refused and the state is left unchanged. Returns `true` if the press was accepted.
    pub fn press(&mut self, widget: WidgetId, state: WidgetState) -> bool {
        if !state.is_activatable() {
            return false;
        }
        self.pressed = Some(widget);
        true
    }

    /// Ends the pointer press and returns the widget that was held, if any.
    pub fn release(&mut self) -> Option<WidgetId> {
        self.pressed.take()
    }
}

fn replace(slot: &mut Option<WidgetId>, value: Option<WidgetId>) -> bool {
    let changed = *slot != value;
    *slot = value;
    changed
}

/// Persistent logical flags of one widget, owned by whoever owns that widget's data
/// (e.g. the Controller's tree or form state), not by the cell grid.
///
/// These affect composition (glyphs and styles chosen when the grid is rebuilt), not the
/// shader, so they are not sent to the GPU.
///
/// - `selected`: the item is the current selection.
/// - `disabled`: the widget is not activatable.
/// - `expanded`: a collapsible node is open.
/// - `checked`: a toggle is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WidgetState {
    pub selected: bool,
    pub disabled: bool,
    pub expanded: bool,
    pub checked: bool,
}

impl WidgetState {
    /// `true` unless the widget is disabled.
    #[must_use]
    pub const fn is_activatable(self) -> bool {
        !self.disabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BorderAxis, CellGrid, GridMetrics, SharedBorder, UiPoint, hit_test};

    fn id(v: u32) -> WidgetId {
        WidgetId::new(v)
    }

    #[test]
    fn default_state_is_empty() {
        let s = InteractionState::default();
        assert_eq!((s.hovered, s.focused, s.pressed), (None, None, None));
        assert_eq!(
            WidgetState::default(),
            WidgetState {
                selected: false,
                disabled: false,
                expanded: false,
                checked: false,
            }
        );
        assert!(WidgetState::default().is_activatable());
    }

    #[test]
    fn hover_set_clear_reports_change() {
        let mut s = InteractionState::default();
        assert!(s.set_hovered(Some(id(1))));
        assert!(!s.set_hovered(Some(id(1))));
        assert!(s.set_hovered(Some(id(2))));
        assert_eq!(s.hovered, Some(id(2)));
        assert!(s.set_hovered(None));
        assert!(!s.set_hovered(None));
        assert_eq!(s.hovered, None);
    }

    #[test]
    fn focus_set_clear_is_independent_of_hover() {
        let mut s = InteractionState::default();
        assert!(s.set_focused(Some(id(5))));
        assert!(!s.set_focused(Some(id(5))));
        s.set_hovered(Some(id(6)));
        assert_eq!((s.focused, s.hovered), (Some(id(5)), Some(id(6))));
        assert!(s.set_focused(None));
        assert_eq!((s.focused, s.hovered), (None, Some(id(6))));
    }

    #[test]
    fn press_release_cycle() {
        let mut s = InteractionState::default();
        assert!(s.press(id(3), WidgetState::default()));
        assert_eq!(s.pressed, Some(id(3)));
        assert_eq!(s.release(), Some(id(3)));
        assert_eq!(s.pressed, None);
        assert_eq!(s.release(), None);
    }

    #[test]
    fn disabled_widget_cannot_be_pressed() {
        let mut s = InteractionState::default();
        let disabled = WidgetState {
            disabled: true,
            ..WidgetState::default()
        };
        assert!(!disabled.is_activatable());
        assert!(!s.press(id(3), disabled));
        assert_eq!(s, InteractionState::default());
        // A refused press does not disturb an existing one.
        assert!(s.press(id(1), WidgetState::default()));
        assert!(!s.press(id(3), disabled));
        assert_eq!(s.pressed, Some(id(1)));
    }

    #[test]
    fn widget_flags_are_independent() {
        let base = WidgetState::default();
        let all = [
            WidgetState {
                selected: true,
                ..base
            },
            WidgetState {
                disabled: true,
                ..base
            },
            WidgetState {
                expanded: true,
                ..base
            },
            WidgetState {
                checked: true,
                ..base
            },
        ];
        for (i, a) in all.iter().enumerate() {
            assert_ne!(*a, base);
            for (j, b) in all.iter().enumerate() {
                assert_eq!(a == b, i == j);
            }
        }
        assert!(all[0].is_activatable() && all[2].is_activatable() && all[3].is_activatable());
    }

    #[test]
    fn equality_and_determinism() {
        let build = || {
            let mut s = InteractionState::default();
            s.set_hovered(Some(id(1)));
            s.set_focused(Some(id(2)));
            s.press(id(1), WidgetState::default());
            s
        };
        assert_eq!(build(), build());
        let mut other = build();
        other.set_focused(Some(id(3)));
        assert_ne!(build(), other);
    }

    fn grid() -> CellGrid {
        let mut g = CellGrid::new(3, 1).unwrap();
        g.get_mut(0, 0).unwrap().widget_id = Some(id(10));
        let c = g.get_mut(1, 0).unwrap();
        c.widget_id = Some(id(99)); // ignored: shared_border wins
        c.shared_border = Some(SharedBorder {
            axis: BorderAxis::Vertical,
            low: Some(id(10)),
            high: Some(id(20)),
        });
        g
    }

    #[test]
    fn hit_result_updates_hover() {
        let m = GridMetrics::new(6, 10).unwrap();
        let g = grid();
        let mut s = InteractionState::default();
        let at = |x| hit_test(UiPoint { x, y: 0 }, m, &g);

        assert!(s.set_hovered_from_hit(at(1)));
        assert_eq!(s.hovered, Some(id(10)));
        // Same widget from a different pixel: no change.
        assert!(!s.set_hovered_from_hit(at(5)));
        // Shared border: left half routes to 10, right half to 20 (never 99).
        assert!(!s.set_hovered_from_hit(at(6)));
        assert!(s.set_hovered_from_hit(at(9)));
        assert_eq!(s.hovered, Some(id(20)));
        // Cell without owner clears hover.
        assert!(s.set_hovered_from_hit(at(13)));
        assert_eq!(s.hovered, None);
        // Outside the grid clears hover too.
        s.set_hovered(Some(id(10)));
        assert!(s.set_hovered_from_hit(at(1000)));
        assert_eq!(s.hovered, None);
    }
}
