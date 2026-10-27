// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! M2.24 complete-shell fidelity against the actual ASCII reference.
#![allow(clippy::unwrap_used)]
use newell_ui::{
    CellContent, CellGrid, GridMetrics, ShellKey, ShellState, ThemeChoice, UiPoint, hit_test,
};

#[path = "../support/reference_shell.rs"]
mod reference_shell;

fn rows(g: &CellGrid) -> Vec<String> {
    (0..g.rows())
        .map(|row| {
            (0..g.cols())
                .map(|col| match g.get(col, row).unwrap().content {
                    CellContent::Glyph(c) => c,
                    _ => ' ',
                })
                .collect()
        })
        .collect()
}

#[test]
fn complete_shell_matches_reference_in_both_themes_and_both_heights() {
    let reference = reference_shell::rows(28);
    assert_eq!(reference.len(), 28);
    for height in [28, 30] {
        for dark in [false, true] {
            let mut s = ShellState::new(82, height);
            s.set_status(98.0, 65);
            let desired_theme = if dark {
                ThemeChoice::Dark
            } else {
                ThemeChoice::Light
            };
            if s.theme() != desired_theme {
                s.key(ShellKey::Alt('v'));
                let g = s.compose().unwrap();
                let (col, row) = rows(&g)
                    .iter()
                    .enumerate()
                    .find_map(|(row, line)| {
                        if !line.contains("Theme") {
                            return None;
                        }
                        let byte = line.find('[')?;
                        Some((line[..byte].chars().count() as u32, row as u32))
                    })
                    .unwrap();
                s.click(hit_test(
                    UiPoint {
                        x: col * 8 + 3,
                        y: row * 18 + 9,
                    },
                    GridMetrics::new(8, 18).unwrap(),
                    &g,
                ));
                s.key(ShellKey::Escape);
            }
            assert_eq!(s.theme(), desired_theme);
            let mut actual = rows(&s.compose().unwrap());
            if height == 30 {
                let blank = format!("║{}║{}║", " ".repeat(23), " ".repeat(56));
                assert_eq!(actual[25], blank);
                assert_eq!(actual[26], blank);
                actual.drain(25..27);
            }
            for (index, (got, want)) in actual.iter().zip(&reference).enumerate() {
                assert_eq!(
                    got,
                    want,
                    "row {} at height {height}, dark={dark}",
                    index + 1
                );
            }
            assert_eq!(actual.len(), reference.len());
        }
    }
}
