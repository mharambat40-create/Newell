// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Application entry point: window manager and event loop (ARCHITECTURE.md §1, §4.3).

use std::process::ExitCode;

fn main() -> ExitCode {
    match newell_app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("newell: {error}");
            ExitCode::FAILURE
        }
    }
}
