// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Logical widget identity.

/// Logical identity of a control, independent of screen position.
///
/// IDs are created by callers (e.g. derived from stable mock node IDs); there
/// is no allocator or registry. Absence of a widget is `Option<WidgetId>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(u32);

impl WidgetId {
    /// Creates an ID from a caller-chosen value.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality_and_inequality() {
        assert_eq!(WidgetId::new(7), WidgetId::new(7));
        assert_ne!(WidgetId::new(7), WidgetId::new(8));
    }

    #[test]
    fn zero_is_an_ordinary_id() {
        assert_eq!(WidgetId::new(0).get(), 0);
        assert_ne!(Some(WidgetId::new(0)), None);
    }

    #[test]
    fn copy_is_stable() {
        let a = WidgetId::new(42);
        let b = a;
        assert_eq!(a, b);
        assert_eq!(a.get(), 42);
    }

    #[test]
    fn usable_in_option_and_hash_set() {
        let some = Some(WidgetId::new(1));
        let none: Option<WidgetId> = None;
        assert!(some.is_some() && none.is_none());
        let set: std::collections::HashSet<_> =
            [WidgetId::new(1), WidgetId::new(1), WidgetId::new(2)].into();
        assert_eq!(set.len(), 2);
    }
}
