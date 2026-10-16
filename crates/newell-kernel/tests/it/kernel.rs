// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use newell_kernel::{NewellKernelError, NewellSolid};

#[test]
fn box_tessellation_is_renderer_owned_data_with_six_face_ranges() {
    let solid = NewellSolid::box_solid(1.0, 2.0, 3.0).unwrap();
    let mesh = solid.tessellate(0.05).unwrap();

    assert_eq!(mesh.positions().len(), mesh.normals().len());
    assert_eq!(mesh.indices().len() % 3, 0);
    assert_eq!(mesh.face_count(), 6);
    assert_eq!(mesh.face_offsets().first(), Some(&0));
    assert_eq!(
        mesh.face_offsets().last(),
        Some(&(mesh.indices().len() as u32))
    );
    assert!(
        mesh.indices()
            .iter()
            .all(|&index| (index as usize) < mesh.positions().len())
    );
    assert!(
        mesh.positions()
            .iter()
            .flatten()
            .all(|coordinate| coordinate.is_finite())
    );
}

#[test]
fn invalid_kernel_inputs_are_translated_without_upstream_types() {
    assert!(matches!(
        NewellSolid::box_solid(f64::NAN, 1.0, 1.0),
        Err(NewellKernelError::InvalidParameter { parameter: "dx" })
    ));
    let solid = NewellSolid::box_solid(1.0, 1.0, 1.0).unwrap();
    assert!(matches!(
        solid.tessellate(0.0),
        Err(NewellKernelError::InvalidParameter {
            parameter: "deflection"
        })
    ));
}
