// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use proptest::prelude::*;

#[test]
fn insta_snapshot_is_wired() {
    insta::assert_debug_snapshot!((1u32, "canary"));
}

proptest! {
    #[test]
    fn proptest_is_wired(x in any::<i64>()) {
        prop_assert_eq!(x.to_string().parse::<i64>(), Ok(x));
    }
}

#[test]
fn unwrap_allowed_in_tests() {
    assert_eq!("1".parse::<u8>().unwrap(), 1);
}
