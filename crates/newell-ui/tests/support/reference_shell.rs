// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Existing golden shell fixture, independent of the retired UI design example.

pub fn rows(height: u32) -> Vec<String> {
    let mut rows: Vec<_> = include_str!("../it/snapshots/it__snapshots__base_shell.snap")
        .lines()
        .skip_while(|line| !line.starts_with('╔'))
        .take(30)
        .map(str::to_owned)
        .collect();
    assert_eq!(rows.len(), 30);
    assert!(rows.iter().all(|row| row.chars().count() == 82));
    assert!(matches!(height, 28 | 30));
    if height == 28 {
        let blank = format!("║{}║{}║", " ".repeat(23), " ".repeat(56));
        assert_eq!(rows[25], blank);
        assert_eq!(rows[26], blank);
        rows.drain(25..27);
    }
    rows
}
