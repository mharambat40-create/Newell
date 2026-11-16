// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Test-only native migration probes; no production routing or public native types.
use solidcore::kernel::{Kernel, KernelOptions, Tolerances};

// One numerical integration policy for all candidate property calls, independent
// of display deflection and the M4 modeling tolerance. Exact oracles stay fixed.
const PROPERTY_RELATIVE_ACCURACY: f64 = 1e-8;

fn trim_cubic_controls(controls: [[f64; 3]; 4], lo: f64, hi: f64) -> Option<[[f64; 3]; 4]> {
    if !(0.0..=1.0).contains(&lo) || !(0.0..=1.0).contains(&hi) || hi <= lo {
        return None;
    }
    let split = |[a, b, c, d]: [[f64; 3]; 4], t: f64| {
        let lerp = |p: [f64; 3], q: [f64; 3]| std::array::from_fn(|i| (1.0 - t) * p[i] + t * q[i]);
        let (ab, bc, cd) = (lerp(a, b), lerp(b, c), lerp(c, d));
        let (abc, bcd) = (lerp(ab, bc), lerp(bc, cd));
        let center = lerp(abc, bcd);
        ([a, ab, abc, center], [center, bcd, cd, d])
    };
    Some(split(split(controls, hi).0, lo / hi).1)
}

fn mapped_kernel(span: f64) -> Kernel {
    let eps = 1e-7_f64.max(1e-10 * span);
    let mut kernel = Kernel::new(&KernelOptions {
        tolerances: Tolerances {
            zone_tol_3d: eps / 2.0,
            xsect_tol_3d: eps,
            approx_tol_3d: eps / 2.0,
            ang_tol_deg: 1e-12 * 180.0 / std::f64::consts::PI,
            tol_gap_gain: 1.0,
            model_size_estimate: span,
            large_small_size_ratio: 1000.0,
        },
        ..KernelOptions::default()
    });
    kernel.context.set_this_model_size_estimate(span);
    kernel.context.set_this_large_small_size_ratio(1000.0);
    // Qualification revision 2 predeclares this single corrected configuration.
    // Default-off smoke is retained in the execution dossier. No fixture changes
    // these flags; all unlisted upstream fixes remain off.
    kernel.fixes.keep_operand_uv_trims = true;
    kernel.fixes.planar_path_uncapped_open_profiles = true;
    kernel.fixes.sheet_sweep_joins_profile_wires = true;
    kernel.fixes.precise_properties_body_length_scale = true;
    // Both integration corrections are unconditional in SolidCore 1.0.1.
    kernel.fixes.brep_area_between_solid_and_void = true;
    kernel
}

#[cfg(feature = "test-support")]
fn native_fixture_matrix(
    transform: [[f64; 4]; 4],
) -> Result<[[f64; 4]; 4], crate::NewellKernelError> {
    if !transform.iter().flatten().all(|value| value.is_finite())
        || transform[3] != [0., 0., 0., 1.]
    {
        return Err(crate::operation_error("invalid fixture transform"));
    }
    let a = transform;
    let determinant = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
        - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
        + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);
    let scale = a[..3]
        .iter()
        .flat_map(|row| &row[..3])
        .fold(0_f64, |size, value| size.max(value.abs()));
    if !determinant.is_finite() || !scale.is_finite() || determinant.abs() <= 1e-14 * scale.powi(3)
    {
        return Err(crate::operation_error("singular fixture transform"));
    }
    // Newell recipes use column vectors; the native matrix API uses row vectors.
    Ok(std::array::from_fn(|row| {
        std::array::from_fn(|column| transform[column][row])
    }))
}

#[test]
fn solidcore_migration_native_box_and_boundary_probe() -> Result<(), Box<dyn std::error::Error>> {
    let mut kernel = mapped_kernel(10.0);
    let body = solidcore::primitive::api_primitives::create_box(
        &mut kernel,
        solidcore::math::Point3::ZERO,
        10.0,
        6.0,
        5.0,
    )?;
    assert!(solidcore::query::api_queries::brep_is_manifold_solid(
        &body
    )?);
    let boundaries = solidcore::boolean::api_brep::tessellate_boundaries(&kernel, &body, 5.0)?;
    assert!(boundaries.complete);
    assert_eq!(boundaries.edges.len(), 12);
    assert_eq!(boundaries.vertex_counts, vec![2; 12]);
    assert_eq!(boundaries.points.len(), 24);
    println!(
        "NATIVE_BOX,kernel_zone={},context_zone={},body_zone={},faces={},edges={}",
        kernel.tolerances.zone_tol_3d,
        kernel.context.this_zone_tol_3d(),
        body.tolerance(),
        body.face_list().len(),
        boundaries.edges.len(),
    );
    for deflection in [0.05, 0.01] {
        let mut slots = body.face_list().to_vec();
        slots.reverse();
        let mesh = crate::solidcore_mesh::planar_native(&kernel, &body, &slots, deflection)?;
        assert_eq!(mesh.face_count(), slots.len());
        assert_eq!(mesh.feature_edges().offsets().len(), 13);
        for (slot, &face) in slots.iter().enumerate() {
            let single = crate::solidcore_mesh::planar_native(&kernel, &body, &[face], deflection)?;
            let start = mesh.face_offsets()[slot] as usize;
            let end = mesh.face_offsets()[slot + 1] as usize;
            assert_eq!(end - start, single.indices().len());
            for (&composed, &local) in mesh.indices()[start..end].iter().zip(single.indices()) {
                assert_eq!(
                    mesh.positions()[composed as usize],
                    single.positions()[local as usize]
                );
                assert_eq!(
                    mesh.normals()[composed as usize],
                    single.normals()[local as usize]
                );
            }
        }
    }
    Ok(())
}

#[test]
fn solidcore_migration_context_tolerance_ledger() {
    let kernel = mapped_kernel(10.0);
    assert_eq!(kernel.tolerances.zone_tol_3d, 5e-8);
    // Characterizes a native option/context mismatch; this is not a tolerance gate pass.
    assert_eq!(kernel.context.this_zone_tol_3d(), 1e-5);
}

#[test]
fn solidcore_migration_explicit_tolerance_holed_profile_probe()
-> Result<(), Box<dyn std::error::Error>> {
    use solidcore::curve::{curve::Curve, line::Line};
    use solidcore::error::EntityRef;
    use solidcore::math::Point3;
    use solidcore::topology::brep::{p2::ArenaNotifyState, p8::FaceBuildEnv};
    use solidcore::topology::{FaceId, topology_traverser::Marks};
    let mut kernel = mapped_kernel(10.0);
    let mut body = solidcore::topology::brep::p3::new_brep(&mut kernel, 10.0, 1000.0)?;
    // Set the empty native owner before constructing geometry, never relabel afterward.
    body.set_tolerance(5e-8);
    let loops = [
        [[0., 0.], [10., 0.], [10., 6.], [0., 6.]],
        [[2., 2.], [2., 4.], [4., 4.], [4., 2.]],
    ];
    let mut curves = Vec::new();
    for points in loops {
        for i in 0..4 {
            let a = points[i];
            let b = points[(i + 1) % 4];
            curves.push(Curve::from(Line::create_line_segment(
                3,
                Point3::new(a[0], a[1], 0.),
                Point3::new(b[0], b[1], 0.),
                None,
            )?));
        }
    }
    let mut state = ArenaNotifyState::new(&body);
    let mut env = state.env(None);
    let mut marks = Marks::default();
    let key_of = env.key_of;
    let this_face = |face: FaceId| key_of(EntityRef::Face(face));
    let mut face_env = FaceBuildEnv {
        env: &mut env,
        marks: &mut marks,
        this_face: &this_face,
    };
    let region = body.infinite_region();
    let faces = solidcore::topology::brep::p8::create_planar_faces_with_3d_curves(
        &mut kernel,
        &mut body,
        region,
        &curves,
        1e-7,
        &mut face_env,
    )?;
    assert_eq!(faces.len(), 1);
    let boundary = solidcore::query::api_queries::face_get_boundary_loops(&body, faces[0])?;
    assert_eq!(boundary.edge_counts, vec![4, 4]);
    println!(
        "F01_PROFILE,body_zone={},loops={:?}",
        body.tolerance(),
        boundary.edge_counts
    );
    for deflection in [0.05, 0.01] {
        let mesh = crate::solidcore_mesh::planar_native(&kernel, &body, &faces, deflection)?;
        assert_eq!(mesh.face_count(), 1);
        assert_eq!(mesh.feature_edges().offsets().len(), 9);
        let mut area = 0.;
        for triangle in mesh.indices().as_chunks::<3>().0 {
            let a = mesh.positions()[triangle[0] as usize];
            let b = mesh.positions()[triangle[1] as usize];
            let c = mesh.positions()[triangle[2] as usize];
            let cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            assert!(cross > 0.);
            area += f64::from(cross) * 0.5;
            let center = [(a[0] + b[0] + c[0]) / 3., (a[1] + b[1] + c[1]) / 3.];
            assert!(!(center[0] > 2. && center[0] < 4. && center[1] > 2. && center[1] < 4.));
        }
        assert!((area - 56.).abs() < 1e-6);
    }
    let occurrences = initial_boundary_occurrences(&body, faces[0])?;
    for (direction, expected) in [
        (solidcore::math::Vec3::new(0., 0., 1.), 280.),
        (solidcore::math::Vec3::new(0.6, 0., 0.8), 224.),
    ] {
        let result = capped_sweep_with_lineage(
            &mut kernel,
            &body,
            faces[0],
            &occurrences,
            [5. * direction.x, 5. * direction.y, 5. * direction.z],
        )?;
        let mut swept = result.body;
        assert!(solidcore::query::api_queries::brep_is_manifold_solid(
            &swept
        )?);
        let store = solidcore::geom::attribute::AttributeStore::default();
        // This probe has no optional mass attributes; no persistent identity is inferred.
        let key_of = |_: EntityRef| solidcore::geom::attribute::ObjectKey(1);
        let attrs = solidcore::mass::integrator::properties::PropertyAttributes {
            store: &store,
            key_of: &key_of,
        };
        let volume = solidcore::mass::integrator::properties::brep_volume(
            &kernel,
            &mut swept,
            PROPERTY_RELATIVE_ACCURACY,
            &attrs,
        )?;
        println!(
            "F01_SWEEP,expected={expected},volume={volume},body_zone={}",
            swept.tolerance()
        );
        assert!((volume - expected).abs() <= 1e-6);
        for deflection in [0.05, 0.01] {
            let mesh =
                crate::solidcore_mesh::native_body(&kernel, &swept, swept.face_list(), deflection)?;
            assert!((oriented_mesh_volume(&mesh)? - expected).abs() < 1e-3);
        }
    }

    let (mut sheets, _) =
        uncapped_sweep_with_lineage(&mut kernel, &body, &occurrences, [0., 0., 5.])?;
    assert_eq!(sheet_component_count(&sheets)?, 2);
    let area = solidcore::mass::integrator::properties::brep_area(
        &kernel,
        &mut sheets,
        PROPERTY_RELATIVE_ACCURACY,
    )?;
    assert!((area - 200.).abs() < 1e-6);
    let mesh = crate::solidcore_mesh::native_body(&kernel, &sheets, sheets.face_list(), 0.01)?;
    assert!(!mesh.indices().is_empty());
    Ok(())
}

#[cfg(test)]
fn planar_rectangle(
    kernel: &mut Kernel,
    x0: f64,
    x1: f64,
) -> Result<solidcore::Brep, solidcore::SolidError> {
    use solidcore::curve::{curve::Curve, line::Line};
    use solidcore::error::EntityRef;
    use solidcore::math::Point3;
    use solidcore::topology::brep::{p2::ArenaNotifyState, p8::FaceBuildEnv};
    use solidcore::topology::{FaceId, topology_traverser::Marks};
    let points = [[x0, 0.], [x1, 0.], [x1, 10.], [x0, 10.]];
    let mut curves = Vec::new();
    for i in 0..4 {
        let a = points[i];
        let b = points[(i + 1) % 4];
        curves.push(Curve::from(Line::create_line_segment(
            3,
            Point3::new(a[0], 0., a[1]),
            Point3::new(b[0], 0., b[1]),
            None,
        )?));
    }
    let mut body = solidcore::topology::brep::p3::new_brep(kernel, 10., 1000.)?;
    body.set_tolerance(5e-8);
    let mut state = ArenaNotifyState::new(&body);
    let mut env = state.env(None);
    let mut marks = Marks::default();
    let key_of = env.key_of;
    let this_face = |face: FaceId| key_of(EntityRef::Face(face));
    let mut face_env = FaceBuildEnv {
        env: &mut env,
        marks: &mut marks,
        this_face: &this_face,
    };
    let region = body.infinite_region();
    let faces = solidcore::topology::brep::p8::create_planar_faces_with_3d_curves(
        kernel,
        &mut body,
        region,
        &curves,
        1e-7,
        &mut face_env,
    )?;
    assert_eq!(faces.len(), 1);
    Ok(body)
}

#[test]
fn solidcore_migration_f19_t_narrow_slot() -> Result<(), Box<dyn std::error::Error>> {
    for _ in 0..10 {
        narrow_slot_witness()?;
    }
    Ok(())
}

#[cfg(test)]
fn narrow_slot_witness() -> Result<(), Box<dyn std::error::Error>> {
    use solidcore::boolean::api_brep::{Boolean2d, Boolean2dOutcome, boolean_2d};
    let mut kernel = mapped_kernel(10.);
    let sheet = planar_rectangle(&mut kernel, 0., 10.)?;
    let cutter = planar_rectangle(&mut kernel, 5., 5.000004)?;
    let mut result = match boolean_2d(&mut kernel, sheet, cutter, Boolean2d::Difference)? {
        Boolean2dOutcome::Result(body) => body,
        Boolean2dOutcome::Unassigned { .. } => return Err("F19-T: no Boolean result".into()),
    };
    let area = solidcore::mass::integrator::properties::brep_area(
        &kernel,
        &mut result,
        PROPERTY_RELATIVE_ACCURACY,
    )?;
    println!(
        "F19_T,area={area},expected=99.99996,faces={},body_zone={}",
        result.face_list().len(),
        result.tolerance()
    );
    assert!((area - 99.99996).abs() <= 1e-7);
    // Count connected sheet components through shared exact topology, allowing
    // different valid native face partitions.
    let mut pending = result.face_list().to_vec();
    let mut components = 0;
    let mut component_areas = Vec::new();
    while let Some(face) = pending.pop() {
        components += 1;
        let mut component = vec![face];
        let mut edges: std::collections::BTreeSet<_> =
            solidcore::query::api_queries::face_get_boundary_loops(&result, face)?
                .edges
                .into_iter()
                .collect();
        loop {
            let mut joined = false;
            let mut index = 0;
            while index < pending.len() {
                let boundary = solidcore::query::api_queries::face_get_boundary_loops(
                    &result,
                    pending[index],
                )?;
                if boundary.edges.iter().any(|edge| edges.contains(edge)) {
                    edges.extend(boundary.edges);
                    component.push(pending.swap_remove(index));
                    joined = true;
                } else {
                    index += 1;
                }
            }
            if !joined {
                break;
            }
        }
        let mut component_area = 0.;
        for face in component {
            component_area += solidcore::mass::integrator::properties::face_area(
                &kernel,
                &mut result,
                face,
                PROPERTY_RELATIVE_ACCURACY,
            )?;
        }
        component_areas.push(component_area);
    }
    assert_eq!(components, 2);
    component_areas.sort_by(f64::total_cmp);
    for (actual, expected) in component_areas.into_iter().zip([49.99996, 50.]) {
        assert!((actual - expected).abs() <= 1e-7);
    }
    assert_native_tolerances(&result, 5e-8)?;
    for deflection in [0.05, 0.01] {
        let mesh =
            crate::solidcore_mesh::native_body(&kernel, &result, result.face_list(), deflection)?;
        for t in mesh.indices().as_chunks::<3>().0 {
            let x = t
                .iter()
                .map(|&i| f64::from(mesh.positions()[i as usize][0]))
                .sum::<f64>()
                / 3.;
            assert!(x <= 5. || x >= 5.000004, "trimmed slot filled by mesh");
        }
    }
    Ok(())
}

#[test]
fn solidcore_migration_f05_native_split_and_empty() -> Result<(), Box<dyn std::error::Error>> {
    let mut owner = MigrationGeometry::new(20.);
    let a = owner.box_shape([10., 10., 10.], [0.; 3])?;
    let b = owner.box_shape([2., 20., 20.], [4., -5., -5.])?;
    let box_mesh = owner.tessellate(a, 0.01)?;
    assert!((oriented_mesh_volume(&box_mesh)? - 1000.).abs() < 1e-5);
    let target = independent_native_copy(
        &mut owner.kernel,
        owner.objects.get(&a).ok_or("missing target")?,
    )?;
    let cutter = independent_native_copy(
        &mut owner.kernel,
        owner.objects.get(&b).ok_or("missing cutter")?,
    )?;
    let kernel = &mut owner.kernel;
    let mut split = solidcore::boolean::api_brep::boolean_difference(kernel, target, cutter)?;
    assert_native_tolerances(&split, 5e-8)?;
    let (materials, voids) = solidcore::query::api_queries::brep_material_census(&split)?;
    assert_eq!((materials, voids), (2, 0));
    assert!(solidcore::query::api_queries::brep_is_manifold_solid(
        &split
    )?);
    let store = solidcore::geom::attribute::AttributeStore::default();
    let key_of = |_: solidcore::error::EntityRef| solidcore::geom::attribute::ObjectKey(1);
    let attrs = solidcore::mass::integrator::properties::PropertyAttributes {
        store: &store,
        key_of: &key_of,
    };
    let volume = solidcore::mass::integrator::properties::brep_volume(
        kernel,
        &mut split,
        PROPERTY_RELATIVE_ACCURACY,
        &attrs,
    )?;
    assert!((volume - 800.).abs() < 1e-6, "split volume {volume}");
    let components = native_material_components(kernel, &mut split)?;
    assert_eq!(components.len(), 2);
    for mut body in components {
        assert_native_tolerances(&body, 5e-8)?;
        assert!(solidcore::query::api_queries::brep_is_manifold_solid(
            &body
        )?);
        assert_eq!(
            solidcore::query::api_queries::brep_material_census(&body)?.0,
            1
        );
        let volume = solidcore::mass::integrator::properties::brep_volume(
            kernel,
            &mut body,
            PROPERTY_RELATIVE_ACCURACY,
            &attrs,
        )?;
        assert!((volume - 400.).abs() < 1e-6, "component volume {volume}");
    }
    let a = owner.box_shape([10., 10., 10.], [0.; 3])?;
    let b = owner.box_shape([12., 12., 12.], [-1.; 3])?;
    let target = independent_native_copy(
        &mut owner.kernel,
        owner.objects.get(&a).ok_or("missing removal target")?,
    )?;
    let cutter = independent_native_copy(
        &mut owner.kernel,
        owner.objects.get(&b).ok_or("missing removal cutter")?,
    )?;
    assert!(native_material_at(
        &owner.kernel,
        &cutter,
        [5., 5., 5.],
        5e-8
    )?);
    assert!(!native_material_at(
        &owner.kernel,
        &cutter,
        [15., 5., 5.],
        5e-8
    )?);
    let cutter_mesh = owner.tessellate(b, 0.01)?;
    assert!((oriented_mesh_volume(&cutter_mesh)? - 1728.).abs() < 1e-5);
    let empty =
        solidcore::boolean::api_brep::boolean_difference(&mut owner.kernel, target, cutter)?;
    assert!(empty.face_list().is_empty());
    assert!(empty.edge_list().is_empty());
    assert!(empty.vertex_list().is_empty());
    // Scratch operand copies leave the original retained owner usable.
    assert!((oriented_mesh_volume(&owner.tessellate(a, 0.01)?)? - 1000.).abs() < 1e-5);
    assert_eq!(
        solidcore::query::api_queries::brep_material_census(&empty)?.0,
        0
    );
    println!("F05_NATIVE,materials={materials},volume={volume},empty_materials=0");
    Ok(())
}

struct MigrationGeometry {
    cancellation: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    kernel: Kernel,
    objects: std::collections::BTreeMap<crate::NewellShapeKey, solidcore::Brep>,
    next: u64,
    roles: std::collections::BTreeMap<
        crate::NewellShapeKey,
        std::collections::BTreeMap<crate::NewellFaceProvenance, solidcore::FaceId>,
    >,
    // Keys belong to this owner and source face. They are constructor occurrences,
    // not native boundary traversal indices or cross-generation geometry matches.
    occurrences: std::collections::BTreeMap<
        crate::NewellShapeKey,
        std::collections::BTreeMap<(crate::NewellFaceProvenance, u32, u32), solidcore::EdgeId>,
    >,
    anonymous_faces: std::collections::BTreeMap<crate::NewellShapeKey, Vec<solidcore::FaceId>>,
    sheet_keys: std::collections::BTreeSet<crate::NewellShapeKey>,
    source_wires: std::collections::BTreeMap<crate::NewellShapeKey, u32>,
    extrusion_sources:
        std::collections::BTreeMap<crate::NewellShapeKey, (crate::NewellShapeKey, u32)>,
}
impl MigrationGeometry {
    fn new(span: f64) -> Self {
        Self {
            cancellation: None,
            kernel: mapped_kernel(span),
            objects: Default::default(),
            next: 0,
            roles: Default::default(),
            occurrences: Default::default(),
            anonymous_faces: Default::default(),
            sheet_keys: Default::default(),
            source_wires: Default::default(),
            extrusion_sources: Default::default(),
        }
    }
    fn set_cancellation_flag(&mut self, flag: std::sync::Arc<std::sync::atomic::AtomicBool>) {
        self.cancellation = Some(flag);
    }
    fn check_cancelled(&self) -> Result<(), crate::NewellKernelError> {
        check_native_cancelled(self.cancellation.as_deref())
    }
    fn insert(
        &mut self,
        body: solidcore::Brep,
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        let key = crate::NewellShapeKey::allocate(&mut self.next)?;
        self.objects.insert(key, body);
        Ok(key)
    }
}

/// Worker-local SolidCore owner with Newell-owned inputs, results and identities.
/// The worker constructs it on its own thread and discards it after a failed job.
pub struct SolidCoreGeometry(MigrationGeometry);

impl SolidCoreGeometry {
    /// Creates an empty geometry owner. Each operation maps its own actual span.
    #[must_use]
    pub fn new() -> Self {
        Self(MigrationGeometry::new(1.))
    }

    /// Installs the worker request's cooperative cancellation flag.
    pub fn set_cancellation_flag(&mut self, flag: std::sync::Arc<std::sync::atomic::AtomicBool>) {
        self.0.set_cancellation_flag(flag);
    }

    /// Constructs a box with generation-local face and edge roles.
    pub fn box_shape(
        &mut self,
        size: [f64; 3],
        origin: [f64; 3],
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        self.0.box_shape(size, origin)
    }

    /// Constructs an exact curved base fixture for application regressions.
    #[cfg(feature = "test-support")]
    pub fn cubic_shape_for_test(
        &mut self,
        control: [[f64; 2]; 4],
        depth: f64,
        transform: [[f64; 4]; 4],
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        self.0.cubic_shape_for_test(control, depth, transform)
    }

    /// Applies a fixture affine map before the model issues geometry references.
    #[cfg(feature = "test-support")]
    pub fn transform_shape_for_test(
        &mut self,
        key: crate::NewellShapeKey,
        transform: [[f64; 4]; 4],
    ) -> Result<(), crate::NewellKernelError> {
        self.0.transform_shape_for_test(key, transform)
    }

    /// Resolves a proven source wire for a sheet output.
    #[must_use]
    pub fn source_wire(&self, key: crate::NewellShapeKey) -> Option<u32> {
        self.0.source_wire(key)
    }

    /// Returns planar face descriptors in the mesh's face-slot order.
    pub fn profile_faces(
        &self,
        key: crate::NewellShapeKey,
    ) -> Result<Vec<crate::NewellProfileFace>, crate::NewellKernelError> {
        self.0.profile_faces(key)
    }

    /// Resolves a straight oriented edge as an Along direction.
    pub fn direction(
        &self,
        key: crate::NewellShapeKey,
        face: u32,
        wire: u32,
        edge: u32,
    ) -> Result<[f64; 3], crate::NewellKernelError> {
        self.0.direction(key, face, wire, edge)
    }

    /// Measures the filled source's supported planar starting target.
    pub fn starting_station(
        &mut self,
        profile: (crate::NewellShapeKey, u32),
        direction: [f64; 3],
        target: (crate::NewellShapeKey, u32),
    ) -> Result<f64, crate::NewellKernelError> {
        self.0.starting_station(profile, direction, target)
    }

    /// Resolves an exact surface extrusion request.
    pub fn extrude_surface(
        &mut self,
        request: crate::NewellSurfaceExtrude,
    ) -> Result<Vec<crate::NewellShapeKey>, crate::NewellKernelError> {
        self.0.extrude_surface(request)
    }

    /// Creates a capped solid sweep from a selected planar face.
    pub fn extrude_face(
        &mut self,
        key: crate::NewellShapeKey,
        face: u32,
        along: Option<[f64; 3]>,
        start: f64,
        end: f64,
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        self.0.extrude_face(key, face, along, start, end)
    }

    /// Creates a bounded solid sweep to a proven planar target face.
    pub fn extrude_to_face(
        &mut self,
        profile: (crate::NewellShapeKey, u32),
        direction: [f64; 3],
        start: f64,
        target: (crate::NewellShapeKey, u32),
        clearance: f64,
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        self.0
            .extrude_to_face(profile, direction, start, target, clearance)
    }

    /// Finds the last exact forward hit with supported generated targets.
    pub fn through_all_station(
        &mut self,
        profile: (crate::NewellShapeKey, u32),
        direction: [f64; 3],
        start: f64,
        targets: &[crate::NewellShapeKey],
    ) -> Result<f64, crate::NewellKernelError> {
        self.0
            .through_all_station(profile, direction, start, targets)
    }

    /// Reflects a constructed sweep with reciprocal native lineage.
    pub fn mirror_sweep(
        &mut self,
        shape: crate::NewellShapeKey,
        profile: (crate::NewellShapeKey, u32),
        direction: [f64; 3],
        start: f64,
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        self.0.mirror_sweep(shape, profile, direction, start)
    }

    /// Joins two coaxial sweeps with a shared start, preserving the original profile roles.
    pub fn join_extrusion_sides(
        &mut self,
        first: crate::NewellShapeKey,
        second: crate::NewellShapeKey,
        profile: (crate::NewellShapeKey, u32),
        direction: [f64; 3],
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        self.0
            .join_extrusion_sides(first, second, profile, direction)
    }

    /// Executes an exact solid Boolean; an empty cut has no output.
    pub fn boolean(
        &mut self,
        target: crate::NewellShapeKey,
        tool: crate::NewellShapeKey,
        mode: crate::NewellBooleanMode,
    ) -> Result<Option<crate::NewellShapeKey>, crate::NewellKernelError> {
        self.0.boolean(target, tool, mode)
    }

    /// Separates disconnected material components while retaining cavities.
    pub fn body_components(
        &mut self,
        key: crate::NewellShapeKey,
    ) -> Result<Vec<crate::NewellShapeKey>, crate::NewellKernelError> {
        self.0.body_components(key)
    }

    /// Combines the tool with the selected material targets atomically.
    pub fn combine_bodies_cancellable(
        &mut self,
        tool: crate::NewellShapeKey,
        targets: &[crate::NewellShapeKey],
        mode: crate::NewellBooleanMode,
        cancelled: impl FnMut() -> bool,
    ) -> Result<crate::NewellBodyResult, crate::NewellKernelError> {
        self.0
            .combine_bodies_cancellable(tool, targets, mode, cancelled)
    }

    /// Combines sheet tools with selected sheet targets atomically.
    pub fn combine_surfaces(
        &mut self,
        tools: &[crate::NewellShapeKey],
        targets: &[crate::NewellShapeKey],
        mode: crate::NewellBooleanMode,
    ) -> Result<crate::NewellSurfaceResult, crate::NewellKernelError> {
        self.0.combine_surfaces(tools, targets, mode)
    }

    /// Verifies that output face roles come from the named profile construction.
    pub fn tag_extrude(
        &mut self,
        key: crate::NewellShapeKey,
        profile: (crate::NewellShapeKey, u32),
        direction: [f64; 3],
    ) -> Result<(), crate::NewellKernelError> {
        self.0.tag_extrude(key, profile, direction)
    }

    /// Reports whether the key owns an uncapped sheet.
    #[must_use]
    pub fn is_surface(&self, key: crate::NewellShapeKey) -> bool {
        self.0.is_surface(key)
    }

    /// Counts the complete native face set.
    pub fn face_count(
        &self,
        key: crate::NewellShapeKey,
    ) -> Result<usize, crate::NewellKernelError> {
        self.0.face_count(key)
    }

    /// Measures a closed material body independently of its display mesh.
    pub fn volume(&self, key: crate::NewellShapeKey) -> Result<f64, crate::NewellKernelError> {
        self.0.volume(key)
    }

    /// Measures a sheet independently of its display mesh.
    pub fn surface_area(
        &self,
        key: crate::NewellShapeKey,
    ) -> Result<f64, crate::NewellKernelError> {
        self.0.surface_area(key)
    }

    /// Tessellates exact faces and edges with face-slot picking ranges.
    pub fn tessellate(
        &self,
        key: crate::NewellShapeKey,
        deflection: f64,
    ) -> Result<crate::NewellMesh, crate::NewellKernelError> {
        self.0.tessellate(key, deflection)
    }
}

impl Default for SolidCoreGeometry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "test-support")]
impl MigrationGeometry {
    fn insert_base_body(
        &mut self,
        body: solidcore::Brep,
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        use crate::{NewellFaceProvenance as Role, operation_error};
        let mut roles = std::collections::BTreeMap::new();
        let mut occurrences = NativeOccurrences::new();
        for (index, &face) in body.face_list().iter().enumerate() {
            let role = Role::Base(u32::try_from(index).map_err(operation_error)?);
            roles.insert(role, face);
            for ((wire, edge), native) in initial_boundary_occurrences(&body, face)? {
                occurrences.insert((role, wire, edge), native);
            }
        }
        self.check_cancelled()?;
        let key = self.insert(body)?;
        self.roles.insert(key, roles);
        self.occurrences.insert(key, occurrences);
        Ok(key)
    }

    fn cubic_shape_for_test(
        &mut self,
        control: [[f64; 2]; 4],
        depth: f64,
        transform: [[f64; 4]; 4],
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        use crate::operation_error;
        use solidcore::{
            curve::{curve::Curve, line::Line},
            math::Point3,
            nurbs::{b_spline_curve::BSplineCurve, crv_basic::Curve as NurbsCurve, math::CPoint},
        };
        self.check_cancelled()?;
        if !depth.is_finite() || depth <= 1e-7 || !control.iter().flatten().all(|v| v.is_finite()) {
            return Err(operation_error("invalid cubic fixture"));
        }
        let matrix = native_fixture_matrix(transform)?;
        let controls = control
            .map(|point| CPoint::from_point(Point3::new(point[0], point[1], 0.)))
            .to_vec();
        let curve = Curve::from(BSplineCurve::from_nurb(
            NurbsCurve::new(controls, 3, vec![0., 0., 0., 0., 1., 1., 1., 1.])
                .map_err(operation_error)?,
            3,
            false,
        ));
        let chord = Curve::from(
            Line::create_line_segment(
                3,
                Point3::new(control[3][0], control[3][1], 0.),
                Point3::new(control[0][0], control[0][1], 0.),
                None,
            )
            .map_err(operation_error)?,
        );
        let (profile, face) = exact_planar_profile(&mut self.kernel, &[curve, chord])?;
        let occurrences = initial_boundary_occurrences(&profile, face)?;
        let mut body = capped_sweep_with_lineage(
            &mut self.kernel,
            &profile,
            face,
            &occurrences,
            [0., 0., depth],
        )?
        .body;
        solidcore::topology::brep::mirror::transform_matrix(
            &mut self.kernel,
            &mut body,
            &matrix,
            false,
        )
        .map_err(operation_error)?;
        body.refresh_cached_topology();
        if !solidcore::query::api_queries::brep_is_manifold_solid(&body).map_err(operation_error)? {
            return Err(operation_error("transformed cubic fixture is not manifold"));
        }
        self.insert_base_body(body)
    }

    fn transform_shape_for_test(
        &mut self,
        key: crate::NewellShapeKey,
        transform: [[f64; 4]; 4],
    ) -> Result<(), crate::NewellKernelError> {
        use crate::operation_error;
        self.check_cancelled()?;
        let matrix = native_fixture_matrix(transform)?;
        let source = self
            .objects
            .get(&key)
            .ok_or_else(|| operation_error("missing fixture owner"))?;
        let mut scratch = source.clone_standing_for_self().map_err(operation_error)?;
        solidcore::topology::brep::mirror::transform_matrix(
            &mut self.kernel,
            &mut scratch,
            &matrix,
            false,
        )
        .map_err(operation_error)?;
        scratch.refresh_cached_topology();
        for &face in self
            .roles
            .get(&key)
            .into_iter()
            .flat_map(|roles| roles.values())
        {
            scratch.face(face).map_err(operation_error)?;
        }
        for &edge in self
            .occurrences
            .get(&key)
            .into_iter()
            .flat_map(|edges| edges.values())
        {
            scratch.edge(edge).map_err(operation_error)?;
        }
        if !solidcore::query::api_queries::brep_is_manifold_solid(&scratch)
            .map_err(operation_error)?
        {
            return Err(operation_error("transformed fixture is not manifold"));
        }
        self.check_cancelled()?;
        self.objects.insert(key, scratch);
        Ok(())
    }
}

#[test]
fn solidcore_migration_owner_key_exhaustion_is_atomic() -> Result<(), Box<dyn std::error::Error>> {
    let mut owner = MigrationGeometry::new(10.);
    let body = solidcore::primitive::api_primitives::create_box(
        &mut owner.kernel,
        solidcore::math::Point3::ZERO,
        1.,
        1.,
        1.,
    )?;
    let first = owner.insert(body.clone_standing_for_self()?)?;
    let second = owner.insert(body.clone_standing_for_self()?)?;
    assert_ne!(first, second);
    assert_eq!(owner.next, 2);
    assert_eq!(owner.objects.len(), 2);
    owner.next = u64::MAX - 1;
    owner.insert(body.clone_standing_for_self()?)?;
    let count = owner.objects.len();
    for _ in 0..2 {
        assert!(owner.insert(body.clone_standing_for_self()?).is_err());
        assert_eq!(owner.next, u64::MAX);
        assert_eq!(owner.objects.len(), count);
    }
    Ok(())
}

fn native_material_components(
    kernel: &mut Kernel,
    source: &mut solidcore::Brep,
) -> Result<Vec<solidcore::Brep>, solidcore::SolidError> {
    use solidcore::error::EntityRef;
    use solidcore::topology::brep::{
        p2::ArenaNotifyState,
        p3::new_brep,
        p5::{StitchAndOrientOptions, stitch_and_orient},
        p7::copy_faces,
    };
    let regions = solidcore::topology::brep::p1::regions(source, None)?;
    let mut results = Vec::new();
    let span = solidcore::query::brep::calculate_bounding_box(
        kernel,
        &*source,
        solidcore::query::brep::BoundingBoxOptions {
            approx: false,
            tight: true,
            expand_by_zone_tol_3d: false,
        },
    )?
    .max_length();
    for region in regions {
        let region = source.region(region)?;
        if region.is_void != 0 {
            continue;
        }
        let mut faces = std::collections::BTreeSet::new();
        for &shell in &region.shells {
            faces.extend(source.shell(shell)?.faces.iter().copied());
        }
        let faces: Vec<_> = faces.into_iter().collect();
        let mut dest = new_brep(kernel, span, 1000.)?;
        dest.set_tolerance(source.tolerance());
        let mut state = ArenaNotifyState::new(source);
        state.seed(&dest);
        let mut env = state.env(None);
        let key_of = env.key_of;
        let this_face = |face| key_of(EntityRef::Face(face));
        let (_, ok) = copy_faces(
            kernel, source, &faces, &mut dest, true, &this_face, &mut env,
        )?;
        if !ok {
            return Err(solidcore::SolidError::invalid_input(
                "migration components",
                "face copy incomplete",
            ));
        }
        stitch_and_orient(kernel, &mut dest, &StitchAndOrientOptions::default())?;
        dest.refresh_cached_topology();
        results.push(dest);
    }
    Ok(results)
}

impl MigrationGeometry {
    fn box_shape(
        &mut self,
        size: [f64; 3],
        origin: [f64; 3],
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        self.box_shape_in_order(size, origin, [0, 1, 2, 3, 4, 5])
    }
    fn box_shape_in_order(
        &mut self,
        size: [f64; 3],
        origin: [f64; 3],
        order: [usize; 6],
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        use crate::operation_error;
        use solidcore::curve::{curve::Curve, line::Line};
        use solidcore::math::Point3;
        if size.iter().any(|x| !x.is_finite() || *x <= 1e-7)
            || origin.iter().any(|x| !x.is_finite())
        {
            return Err(crate::NewellKernelError::InvalidParameter {
                parameter: "box recipe",
            });
        }
        self.check_cancelled()?;
        let vertices = [
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [1., 0., 1.],
            [1., 1., 1.],
            [0., 1., 1.],
        ]
        .map(|p| std::array::from_fn::<_, 3, _>(|i| origin[i] + p[i] * size[i]));
        let faces = [
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
            [1, 2, 6, 5],
        ];
        let span = size.into_iter().fold(0_f64, f64::max);
        let scale = vertices.iter().flatten().fold(span, |s, x| s.max(x.abs()));
        let eps = configure_native_policy(&mut self.kernel, span, scale)?;
        let mut body = solidcore::topology::brep::p3::new_brep(&mut self.kernel, span, 1000.)
            .map_err(operation_error)?;
        body.set_tolerance(native_construction_zone(eps, scale)?);
        let mut roles = std::collections::BTreeMap::new();
        let mut occurrences = NativeOccurrences::new();
        for role in order {
            self.check_cancelled()?;
            let corners = faces[role];
            let mut curves = Vec::new();
            for i in 0..4 {
                let a = vertices[corners[i]];
                let b = vertices[corners[(i + 1) % 4]];
                curves.push(Curve::from(
                    Line::create_line_segment(
                        3,
                        Point3::new(a[0], a[1], a[2]),
                        Point3::new(b[0], b[1], b[2]),
                        None,
                    )
                    .map_err(operation_error)?,
                ));
            }
            let (face, owners) =
                construct_planar_face_owners(&mut self.kernel, &mut body, curves, eps)?;
            let role =
                crate::NewellFaceProvenance::Base(u32::try_from(role).map_err(operation_error)?);
            roles.insert(role, face);
            for (edge, native) in owners.into_iter().enumerate() {
                occurrences.insert(
                    (role, 0, u32::try_from(edge).map_err(operation_error)?),
                    native,
                );
            }
        }
        stitch_constructed_occurrences(&mut self.kernel, &mut body, &roles, &mut occurrences)?;
        body.refresh_cached_topology();
        if !solidcore::query::api_queries::brep_is_manifold_solid(&body).map_err(operation_error)? {
            return Err(operation_error("box is not manifold"));
        }
        self.check_cancelled()?;
        let key = self.insert(body)?;
        self.roles.insert(key, roles);
        self.occurrences.insert(key, occurrences);
        Ok(key)
    }
}

#[test]
fn solidcore_migration_constructive_base_identity() -> Result<(), Box<dyn std::error::Error>> {
    let mut first = MigrationGeometry::new(10.);
    let mut replay = MigrationGeometry::new(20.);
    let a = first.box_shape([10., 6., 5.], [0.; 3])?;
    let b = replay.box_shape_in_order([20., 7., 8.], [3., 4., 5.], [5, 4, 3, 2, 1, 0])?;
    let old = first.roles.get(&a).ok_or("missing first roles")?;
    let new = replay.roles.get(&b).ok_or("missing replay roles")?;
    assert_eq!(
        old.keys().collect::<Vec<_>>(),
        new.keys().collect::<Vec<_>>()
    );
    assert_eq!(old.len(), 6);
    assert_ne!(
        old.get(&crate::NewellFaceProvenance::Base(0)),
        new.get(&crate::NewellFaceProvenance::Base(0))
    );
    let body = first.objects.get(&a).ok_or("missing native owner")?;
    let slots: Vec<_> = old.values().copied().collect();
    let mesh = crate::solidcore_mesh::planar_native(&first.kernel, body, &slots, 0.01)?;
    assert_eq!(mesh.face_count(), 6);
    assert_eq!(mesh.feature_edges().offsets().len(), 13);
    assert!(body.tolerance() > 0. && body.tolerance() <= 5e-8);
    Ok(())
}

#[test]
fn solidcore_migration_native_cubic_body_mesh() -> Result<(), Box<dyn std::error::Error>> {
    use solidcore::curve::{curve::Curve, line::Line};
    use solidcore::math::Point3;
    use solidcore::nurbs::{
        b_spline_curve::BSplineCurve, crv_basic::Curve as NurbsCurve, math::CPoint,
    };
    let mut kernel = mapped_kernel(5.);
    let controls = [[0., 0., 0.], [0., 2.5, 0.], [5., 2.5, 0.], [5., 0., 0.]]
        .map(|p| CPoint::from_point(Point3::new(p[0], p[1], p[2])));
    let nurb = NurbsCurve::new(controls.to_vec(), 3, vec![0., 0., 0., 0., 1., 1., 1., 1.])?;
    let curve = Curve::from(BSplineCurve::from_nurb(nurb, 3, false));
    let chord = Curve::from(Line::create_line_segment(
        3,
        Point3::new(5., 0., 0.),
        Point3::ZERO,
        None,
    )?);
    let (profile, face) = exact_planar_profile(&mut kernel, &[curve, chord])?;
    let occurrences = initial_boundary_occurrences(&profile, face)?;
    let body =
        capped_sweep_with_lineage(&mut kernel, &profile, face, &occurrences, [0., 0., 4.])?.body;
    let mut measured = body.clone_standing_for_self()?;
    let attrs_store = solidcore::geom::attribute::AttributeStore::default();
    let attr_key = |_: solidcore::error::EntityRef| solidcore::geom::attribute::ObjectKey(1);
    let attrs = solidcore::mass::integrator::properties::PropertyAttributes {
        store: &attrs_store,
        key_of: &attr_key,
    };
    let exact_volume = solidcore::mass::integrator::properties::brep_volume(
        &kernel,
        &mut measured,
        PROPERTY_RELATIVE_ACCURACY,
        &attrs,
    )?;
    assert!((exact_volume - 30.).abs() < 1e-6);
    let cubic_edges:std::collections::BTreeMap<_,_>=occurrences.iter().filter_map(|(&key,&id)| {
        let curve=profile.edge(id).ok()?.curve.as_ref()?;
        matches!(curve.kind(),solidcore::curve::curve::CurveKind::BSpline(c) if c.gw_nurb().is_some_and(|n|n.degree()==3)).then_some((key,id))
    }).collect();
    assert_eq!(cubic_edges.len(), 1);
    let (mut sheet, _) =
        uncapped_sweep_with_lineage(&mut kernel, &profile, &cubic_edges, [0., 0., 4.])?;
    let area = solidcore::mass::integrator::properties::brep_area(
        &kernel,
        &mut sheet,
        PROPERTY_RELATIVE_ACCURACY,
    )?;
    let speed = |t: f64| ((30. * t * (1. - t)).powi(2) + (7.5 * (1. - 2. * t)).powi(2)).sqrt();
    let n = 4096;
    let mut integral = speed(0.) + speed(1.);
    for i in 1..n {
        integral += if i % 2 == 0 { 2. } else { 4. } * speed(f64::from(i) / f64::from(n));
    }
    let length = integral / (3. * f64::from(n));
    assert!(
        (area - 4. * length).abs() < 1e-6,
        "cubic sheet area {area}, expected {}",
        4. * length
    );
    assert_eq!(free_boundary_cycles(&sheet)?, 1);
    let sheet_mesh = crate::solidcore_mesh::native_body(&kernel, &sheet, sheet.face_list(), 0.01)?;
    assert!(!sheet_mesh.indices().is_empty());
    println!("F04_NATIVE,kind=cubic,area={area},length={length}");
    // Mesh volume is approximate:75 mm² bounds cap+side area from the cubic's
    // control-polygon length. Exact native geometry keeps its1e-6 mm³ oracle.
    for deflection in [0.05, 0.01] {
        let mesh =
            crate::solidcore_mesh::native_body(&kernel, &body, body.face_list(), deflection)?;
        assert_eq!(mesh.face_count(), body.face_list().len());
        assert!(!mesh.indices().is_empty());
        let signed = oriented_mesh_volume(&mesh)?;
        assert!(
            (signed - 30.).abs() < 75. * deflection,
            "cubic signed volume {signed}"
        );
        assert_eq!(mesh.normals().len(), mesh.positions().len());
        assert!(!mesh.feature_edges().positions().is_empty());
        println!(
            "NATIVE_CUBIC_MESH,deflection={deflection},faces={},triangles={}",
            mesh.face_count(),
            mesh.indices().len() / 3
        );
    }
    Ok(())
}

#[test]
fn solidcore_migration_native_cap_provenance_route() -> Result<(), Box<dyn std::error::Error>> {
    use solidcore::curve::{curve::Curve, line::Line};
    use solidcore::math::Point3;
    let mut owner = MigrationGeometry::new(10.);
    let key = owner.box_shape([10., 6., 5.], [0.; 3])?;
    let face = *owner
        .roles
        .get(&key)
        .and_then(|r| r.get(&crate::NewellFaceProvenance::Base(1)))
        .ok_or("missing semantic top")?;
    let source = owner.objects.get(&key).ok_or("missing source")?;
    let path = Curve::from(Line::create_line_segment(
        3,
        Point3::new(0., 0., 5.),
        Point3::new(0., 0., 8.),
        None,
    )?);
    let options = solidcore::primitive::primitive_creation::SweepOptions {
        cap_ends: true,
        translational_sweep: true,
        profile_at_path_start: true,
        ..Default::default()
    };
    let sweep = solidcore::sweep::api_brep::create_curve_sweep_from_faces(
        &mut owner.kernel,
        &[(source, face)],
        Some(&path),
        None,
        None,
        Some(&options),
    )?;
    assert_eq!(sweep.start_faces.len(), 1);
    assert_eq!(sweep.end_faces.len(), 1);
    assert_ne!(sweep.start_faces, sweep.end_faces);
    for &cap in sweep.start_faces.iter().chain(&sweep.end_faces) {
        let loops = solidcore::query::api_queries::face_get_boundary_loops(&sweep.brep, cap)?;
        assert_eq!(loops.edge_counts, vec![4]);
    }
    let mesh = crate::solidcore_mesh::native_body(
        &owner.kernel,
        &sweep.brep,
        sweep.brep.face_list(),
        0.01,
    )?;
    assert!(!mesh.indices().is_empty());
    Ok(())
}

#[test]
fn solidcore_migration_explicit_owner_sweep_tolerance() -> Result<(), Box<dyn std::error::Error>> {
    use solidcore::nurbs::{
        b_spline_curve::BSplineCurve, crv_basic::Curve as NurbsCurve, math::CPoint,
    };
    use solidcore::primitive::primitive_creation::{
        SweepOptions, model::PrimitiveModel, p1::PrimitiveCreation,
    };
    let mut owner = MigrationGeometry::new(10.);
    let key = owner.box_shape([10., 6., 5.], [0.; 3])?;
    let face = *owner
        .roles
        .get(&key)
        .and_then(|r| r.get(&crate::NewellFaceProvenance::Base(1)))
        .ok_or("missing semantic top")?;
    let source = owner.objects.get(&key).ok_or("missing source")?;
    let mut output = solidcore::topology::brep::p3::new_brep(&mut owner.kernel, 10., 1000.)?;
    output.set_tolerance(5e-8);
    let mut pc = PrimitiveCreation::new(output.infinite_region(), 0, true);
    let path = BSplineCurve::from_nurb(
        NurbsCurve::new(
            vec![
                CPoint::from_point(solidcore::math::Point3::new(0., 0., 5.)),
                CPoint::from_point(solidcore::math::Point3::new(0., 0., 8.)),
            ],
            1,
            vec![0., 0., 1., 1.],
        )?,
        3,
        false,
    );
    let mut model = PrimitiveModel::new(&mut owner.kernel, output);
    let input = model.add_owner_breps(&[(source, face)])?;
    let mut start = Vec::new();
    let mut sides = Vec::new();
    let mut end = Vec::new();
    pc.create_curve_sweep_from_faces(
        &mut model,
        PrimitiveModel::MAIN,
        &input,
        &path,
        None,
        None,
        5e-8,
        SweepOptions {
            cap_ends: true,
            translational_sweep: true,
            profile_at_path_start: true,
            ..Default::default()
        },
        &mut start,
        &mut sides,
        &mut end,
    )?;
    let output = model.into_brep()?;
    let mut largest = output.tolerance();
    for f in output.face_list() {
        largest = largest.max(solidcore::topology::face::p2::tolerance(&output, *f)?);
    }
    for e in output.edge_list() {
        largest = largest.max(output.edge(*e)?.tolerance);
    }
    for v in output.vertex_list() {
        largest = largest.max(output.vertex(*v)?.tolerance);
    }
    println!(
        "EXPLICIT_OWNER_SWEEP,body_zone={},largest_entity_zone={largest},context_zone={}",
        output.tolerance(),
        owner.kernel.context.this_zone_tol_3d()
    );
    assert!(
        largest <= 5e-8,
        "coarse effective sweep tolerance {largest}"
    );
    assert_eq!(start.len(), 1);
    assert_eq!(end.len(), 1);
    Ok(())
}

/// Bind an initial box recipe's straight source segment within its explicitly
/// constructed face. This creates the source occurrence map in a fresh owner;
/// copied/replayed occurrences use TopologySweep's actual lineage maps instead.
#[cfg(test)]
fn constructed_linear_occurrence(
    body: &solidcore::Brep,
    cap: solidcore::FaceId,
    expected: [[f64; 3]; 2],
) -> Result<(u32, u32, solidcore::EdgeId), crate::NewellKernelError> {
    use crate::operation_error;
    let loops = solidcore::query::api_queries::face_get_boundary_loops(body, cap)
        .map_err(operation_error)?;
    let mut offset = 0;
    let mut matches = Vec::new();
    for (wire, &count) in loops.edge_counts.iter().enumerate() {
        for edge in 0..count {
            let native = *loops
                .edges
                .get(offset + edge)
                .ok_or_else(|| operation_error("missing cap occurrence"))?;
            let record = body.edge(native).map_err(operation_error)?;
            let a = body
                .vertex(record.vertices[0])
                .map_err(operation_error)?
                .point;
            let b = body
                .vertex(record.vertices[1])
                .map_err(operation_error)?
                .point;
            let a = [a.x, a.y, a.z];
            let b = [b.x, b.y, b.z];
            if (a == expected[0] && b == expected[1]) || (a == expected[1] && b == expected[0]) {
                // A line is determined by its two distinct endpoints; use only
                // exact input-copy values, never proximity or a similar curve.
                if record.curve.as_ref().is_some_and(|c| match c.kind() {
                    solidcore::curve::curve::CurveKind::Line(_) => true,
                    solidcore::curve::curve::CurveKind::BSpline(c) => {
                        c.gw_nurb().is_some_and(|n| n.degree() == 1 && n.n() == 1)
                    }
                    _ => false,
                }) {
                    matches.push((
                        u32::try_from(wire).map_err(operation_error)?,
                        u32::try_from(edge).map_err(operation_error)?,
                        native,
                    ));
                }
            }
        }
        offset += count;
    }
    match matches.as_slice() {
        [one] => Ok(*one),
        _ => Err(operation_error("lost or ambiguous constructive cap edge")),
    }
}

#[test]
fn solidcore_migration_native_circle_body_mesh() -> Result<(), Box<dyn std::error::Error>> {
    use solidcore::curve::curve::Curve;
    use solidcore::nurbs::{
        b_spline_curve::BSplineCurve, crv_basic::Curve as NurbsCurve, math::CPoint,
    };
    let mut kernel = mapped_kernel(4.);
    let xy = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
    ];
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let weights = [1., w, 1., w, 1., w, 1., w, 1.];
    let controls = xy
        .into_iter()
        .zip(weights)
        .map(|(p, w)| CPoint::new(2. * p[0] * w, 2. * p[1] * w, 0., w))
        .collect();
    let circle = Curve::from(BSplineCurve::from_nurb(
        NurbsCurve::new(
            controls,
            2,
            vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
        )?,
        3,
        false,
    ));
    let (profile, face) = exact_planar_profile(&mut kernel, &[circle])?;
    let occurrences = initial_boundary_occurrences(&profile, face)?;
    let mut sweep =
        capped_sweep_with_lineage(&mut kernel, &profile, face, &occurrences, [0., 0., 3.])?;
    let store = solidcore::geom::attribute::AttributeStore::default();
    let key_of = |_: solidcore::error::EntityRef| solidcore::geom::attribute::ObjectKey(1);
    let attributes = solidcore::mass::integrator::properties::PropertyAttributes {
        store: &store,
        key_of: &key_of,
    };
    let volume = solidcore::mass::integrator::properties::brep_volume(
        &kernel,
        &mut sweep.body,
        PROPERTY_RELATIVE_ACCURACY,
        &attributes,
    )?;
    assert!((volume - 12. * std::f64::consts::PI).abs() < 1e-6);
    let (mut sheet, _) =
        uncapped_sweep_with_lineage(&mut kernel, &profile, &occurrences, [0., 0., 3.])?;
    let area = solidcore::mass::integrator::properties::brep_area(
        &kernel,
        &mut sheet,
        PROPERTY_RELATIVE_ACCURACY,
    )?;
    assert!(
        (area - 12. * std::f64::consts::PI).abs() < 1e-6,
        "uncapped circle area {area}"
    );
    assert_eq!(free_boundary_cycles(&sheet)?, 2);
    let sheet_mesh = crate::solidcore_mesh::native_body(&kernel, &sheet, sheet.face_list(), 0.01)?;
    assert_eq!(sheet_mesh.feature_edges().offsets().len(), 3);
    println!("F04_NATIVE,kind=circle,area={area},boundary_cycles=2");
    let (mut longer, _) =
        uncapped_sweep_with_lineage(&mut kernel, &profile, &occurrences, [0., 0., 4.])?;
    let longer_area = solidcore::mass::integrator::properties::brep_area(
        &kernel,
        &mut longer,
        PROPERTY_RELATIVE_ACCURACY,
    )?;
    assert!(
        (longer_area - 16. * std::f64::consts::PI).abs() < 1e-8,
        "retained circular area {longer_area}"
    );
    for deflection in [0.05, 0.01] {
        let mesh = crate::solidcore_mesh::native_body(
            &kernel,
            &sweep.body,
            sweep.body.face_list(),
            deflection,
        )?;
        assert_eq!(mesh.face_count(), sweep.body.face_list().len());
        assert!(!mesh.indices().is_empty());
        assert_eq!(
            mesh.feature_edges().offsets().len(),
            3,
            "only the two cap rings are display edges"
        );
        for pair in mesh.feature_edges().offsets().windows(2) {
            let edge = &mesh.feature_edges().positions()[pair[0] as usize..pair[1] as usize];
            assert!(edge.iter().all(|p| p[2] == 0.) || edge.iter().all(|p| p[2] == 3.));
        }
        let signed = oriented_mesh_volume(&mesh)?;
        assert!(
            (signed - 12. * std::f64::consts::PI).abs() < 20. * std::f64::consts::PI * deflection,
            "circle signed volume {signed}"
        );
        println!(
            "NATIVE_CIRCLE_MESH,deflection={deflection},volume={volume},triangles={}",
            mesh.indices().len() / 3
        );
    }
    Ok(())
}

#[test]
fn solidcore_migration_f03_uncapped_face_wire_edge() -> Result<(), Box<dyn std::error::Error>> {
    let mut owner = MigrationGeometry::new(10.);
    let key = owner.box_shape([10., 6., 5.], [0.; 3])?;
    let source = owner.objects.get(&key).ok_or("missing source")?;
    let full: std::collections::BTreeMap<_, _> = owner
        .occurrences
        .get(&key)
        .ok_or("missing occurrences")?
        .iter()
        .filter_map(|(&(role, w, e), &native)| {
            (role == crate::NewellFaceProvenance::Base(1)).then_some(((w, e), native))
        })
        .collect();
    let selected = full
        .iter()
        .filter_map(|(&key, &native)| (key == (0, 0)).then_some((key, native)))
        .collect();
    for (name, input, expected, free_expected) in [
        ("face", &full, 96., 8),
        ("wire", &full, 96., 8),
        ("edge", &selected, 30., 4),
    ] {
        let (mut sheet, roles) =
            uncapped_sweep_with_lineage(&mut owner.kernel, source, input, [0., 0., 3.])?;
        assert_eq!(roles.len(), input.len());
        let area = solidcore::mass::integrator::properties::brep_area(
            &owner.kernel,
            &mut sheet,
            PROPERTY_RELATIVE_ACCURACY,
        )?;
        assert!((area - expected).abs() < 1e-6, "{name} sheet area {area}");
        assert_eq!(
            solidcore::query::api_queries::brep_material_census(&sheet)?.0,
            0
        );
        let mut free = 0;
        for e in sheet.edge_list() {
            let adjacent = solidcore::topology::edge::faces(&sheet, *e, None)?.len();
            assert!((1..=2).contains(&adjacent));
            if adjacent == 1 {
                free += 1;
            }
        }
        assert_eq!(free, free_expected);
        assert_eq!(sheet_component_count(&sheet)?, 1);
        let mesh =
            crate::solidcore_mesh::native_body(&owner.kernel, &sheet, sheet.face_list(), 0.01)?;
        assert!(!mesh.indices().is_empty());
        for n in mesh.normals() {
            assert!(n[2].abs() < 1e-6, "fake cap normal {n:?}");
        }
        println!("F03_NATIVE,boundary={name},area={area},free_edges={free}");
    }
    Ok(())
}

#[cfg(test)]
fn oriented_mesh_volume(mesh: &crate::NewellMesh) -> Result<f64, Box<dyn std::error::Error>> {
    use solidcore::math::Vec3;
    let point = |i: u32| -> Result<Vec3, Box<dyn std::error::Error>> {
        let p = mesh.positions().get(i as usize).ok_or("mesh point index")?;
        Ok(Vec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2])))
    };
    let origin = point(*mesh.indices().first().ok_or("empty mesh")?)?;
    let mut volume = 0.;
    for t in mesh.indices().as_chunks::<3>().0 {
        let a = point(t[0])?;
        let b = point(t[1])?;
        let c = point(t[2])?;
        let area = (b - a).cross(c - a);
        assert!(area.length_squared() > 0.);
        for &i in t {
            let n = mesh.normals().get(i as usize).ok_or("mesh normal index")?;
            let n = Vec3::new(f64::from(n[0]), f64::from(n[1]), f64::from(n[2]));
            assert!((n.length_squared() - 1.).abs() < 1e-5);
            assert!(area.dot(n) > 0., "triangle opposes face normal");
        }
        volume += (a - origin).dot((b - origin).cross(c - origin)) / 6.;
    }
    Ok(volume)
}

#[test]
fn solidcore_migration_real_precision_loss_still_rejects() {
    // A genuine source feature is not an inserted trim-grid artifact.
    let exact = [[1e6, 0., 0.], [1e6 + 1e-5, 0., 0.], [1e6, 1., 0.]];
    let mesh = crate::solidcore_mesh::precision_probe(exact.to_vec());
    assert!(matches!(
        mesh,
        Err(crate::NewellKernelError::MeshPrecisionLoss)
    ));
}

fn sheet_component_count(body: &solidcore::Brep) -> Result<usize, solidcore::SolidError> {
    let mut pending = body.face_list().to_vec();
    let mut components = 0;
    while let Some(face) = pending.pop() {
        components += 1;
        let mut edges: std::collections::BTreeSet<_> =
            solidcore::query::api_queries::face_get_boundary_loops(body, face)?
                .edges
                .into_iter()
                .collect();
        loop {
            let mut joined = false;
            let mut i = 0;
            while i < pending.len() {
                let boundary =
                    solidcore::query::api_queries::face_get_boundary_loops(body, pending[i])?;
                if boundary.edges.iter().any(|edge| edges.contains(edge)) {
                    edges.extend(boundary.edges);
                    pending.swap_remove(i);
                    joined = true;
                } else {
                    i += 1;
                }
            }
            if !joined {
                break;
            }
        }
    }
    Ok(components)
}

fn native_sheet_components(
    kernel: &mut Kernel,
    source: &mut solidcore::Brep,
) -> Result<Vec<solidcore::Brep>, crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::{
        error::EntityRef,
        topology::brep::{
            p2::ArenaNotifyState,
            p3::new_brep,
            p5::{StitchAndOrientOptions, stitch_and_orient},
            p7::copy_faces,
        },
    };
    let mut pending = source.face_list().to_vec();
    let mut groups = Vec::new();
    while let Some(face) = pending.pop() {
        let mut group = vec![face];
        let mut edges: std::collections::BTreeSet<_> =
            solidcore::query::api_queries::face_get_boundary_loops(source, face)
                .map_err(operation_error)?
                .edges
                .into_iter()
                .collect();
        loop {
            let mut joined = false;
            let mut index = 0;
            while index < pending.len() {
                let boundary =
                    solidcore::query::api_queries::face_get_boundary_loops(source, pending[index])
                        .map_err(operation_error)?;
                if boundary.edges.iter().any(|edge| edges.contains(edge)) {
                    edges.extend(boundary.edges);
                    group.push(pending.swap_remove(index));
                    joined = true;
                } else {
                    index += 1;
                }
            }
            if !joined {
                break;
            }
        }
        groups.push(group);
    }
    if groups.len() <= 1 {
        return Ok(if groups.is_empty() {
            Vec::new()
        } else {
            vec![source.clone_standing_for_self().map_err(operation_error)?]
        });
    }
    let bounds = solidcore::query::brep::calculate_bounding_box(
        kernel,
        &*source,
        solidcore::query::brep::BoundingBoxOptions {
            approx: false,
            tight: true,
            expand_by_zone_tol_3d: false,
        },
    )
    .map_err(operation_error)?;
    let span = bounds.max_length().max(1e-7);
    let mut results = Vec::new();
    for faces in groups {
        let mut dest = new_brep(kernel, span, 1000.).map_err(operation_error)?;
        dest.set_tolerance(source.tolerance());
        let mut state = ArenaNotifyState::new(source);
        state.seed(&dest);
        let mut env = state.env(None);
        let key_of = env.key_of;
        let this_face = |face| key_of(EntityRef::Face(face));
        let (_, complete) = copy_faces(
            kernel, source, &faces, &mut dest, true, &this_face, &mut env,
        )
        .map_err(operation_error)?;
        if !complete {
            return Err(operation_error("sheet component copy incomplete"));
        }
        stitch_and_orient(kernel, &mut dest, &StitchAndOrientOptions::default())
            .map_err(operation_error)?;
        dest.refresh_cached_topology();
        if sheet_component_count(&dest).map_err(operation_error)? != 1 {
            return Err(operation_error("sheet component copy disconnected"));
        }
        validate_native_sheet_links(&dest)?;
        results.push(dest);
    }
    Ok(results)
}

fn native_sheet_area(
    kernel: &Kernel,
    source: &solidcore::Brep,
) -> Result<f64, crate::NewellKernelError> {
    let mut scratch = source
        .clone_standing_for_self()
        .map_err(crate::operation_error)?;
    solidcore::mass::integrator::properties::brep_area(
        kernel,
        &mut scratch,
        PROPERTY_RELATIVE_ACCURACY,
    )
    .map_err(crate::operation_error)
}

#[test]
fn solidcore_migration_topology_sweep_reports_real_edge_lineage()
-> Result<(), Box<dyn std::error::Error>> {
    use solidcore::primitive::primitive_creation::model::PrimitiveModel;
    use solidcore::sweep::{
        topology_sweep::{
            SweepEntity, TopologySweep, access::SweepAccess, engine::SweepTargets,
            geometry::SweepGeometry,
        },
        trans_sweep_geometry::TranslationalSweepGeometry,
    };
    let mut owner = MigrationGeometry::new(10.);
    let key = owner.box_shape([10., 6., 5.], [0.; 3])?;
    let face = *owner
        .roles
        .get(&key)
        .and_then(|r| r.get(&crate::NewellFaceProvenance::Base(1)))
        .ok_or("missing source face")?;
    let source = owner.objects.get(&key).ok_or("missing source body")?;
    let (_, _, edge) = constructed_linear_occurrence(source, face, [[0., 0., 5.], [10., 0., 5.]])?;
    let mut output = solidcore::topology::brep::p3::new_brep(&mut owner.kernel, 10., 1000.)?;
    output.set_tolerance(5e-8);
    let destination_region = output.infinite_region().ok_or("missing target region")?;
    let mut model = PrimitiveModel::new(&mut owner.kernel, output);
    let source_id = model.add_brep(source.clone_standing_for_self()?)?;
    let mut targets = SweepTargets {
        faces_higher: Some(vec![Some((source_id, face))]),
        ..Default::default()
    };
    let mut geometry = TranslationalSweepGeometry::new(solidcore::math::Vec3::new(0., 0., 3.));
    geometry.base.eps = 5e-8;
    let mut sweep = TopologySweep::default();
    sweep.do_sweep(
        &mut model,
        &SweepGeometry::Translation(geometry),
        Some(source_id),
        Some((PrimitiveModel::MAIN, destination_region)),
        &mut targets,
    )?;
    let copied = sweep
        .get_to_same(Some(SweepEntity::Edge(source_id, edge)))
        .ok_or("missing copied edge lineage")?;
    let higher = sweep
        .get_to_higher(Some(SweepEntity::Edge(source_id, edge)))
        .ok_or("missing wall lineage")?;
    let SweepEntity::Edge(body, new_edge) = copied else {
        return Err("wrong copied dimension".into());
    };
    assert_eq!(
        sweep.get_from_same(Some(copied)),
        Some(SweepEntity::Edge(source_id, edge))
    );
    let SweepEntity::Face(wall_body, wall) = higher else {
        return Err("wrong wall dimension".into());
    };
    assert_eq!(body, PrimitiveModel::MAIN);
    assert_eq!(wall_body, body);
    model.brep(body)?.edge(new_edge)?;
    model.brep(body)?.face(wall)?;
    println!("TOPOLOGY_LINEAGE,source={source_id:?}/{edge:?},copied={copied:?},wall={higher:?}");
    Ok(())
}

#[test]
fn solidcore_migration_cap_edge_replay_uses_lineage_not_native_indices()
-> Result<(), Box<dyn std::error::Error>> {
    use solidcore::primitive::primitive_creation::model::PrimitiveModel;
    use solidcore::sweep::{
        topology_sweep::{
            SweepEntity, TopologySweep, access::SweepAccess, engine::SweepTargets,
            geometry::SweepGeometry,
        },
        trans_sweep_geometry::TranslationalSweepGeometry,
    };
    let mut native_faces = Vec::new();
    for (size, origin, order) in [
        ([10., 6., 5.], [0.; 3], [0, 1, 2, 3, 4, 5]),
        ([20., 7., 8.], [3., 4., 5.], [5, 4, 3, 2, 1, 0]),
    ] {
        let mut owner = MigrationGeometry::new(20.);
        let key = owner.box_shape_in_order(size, origin, order)?;
        let face = *owner
            .roles
            .get(&key)
            .and_then(|r| r.get(&crate::NewellFaceProvenance::Base(1)))
            .ok_or("missing semantic source top")?;
        native_faces.push(face);
        let source = owner.objects.get(&key).ok_or("missing source body")?;
        let z = origin[2] + size[2];
        let (_, _, edge) = constructed_linear_occurrence(
            source,
            face,
            [
                [origin[0], origin[1], z],
                [origin[0] + size[0], origin[1], z],
            ],
        )?;
        let mut output = solidcore::topology::brep::p3::new_brep(&mut owner.kernel, 20., 1000.)?;
        output.set_tolerance(5e-8);
        let region = output
            .infinite_region()
            .ok_or("missing destination region")?;
        let mut model = PrimitiveModel::new(&mut owner.kernel, output);
        let source_id = model.add_brep(source.clone_standing_for_self()?)?;
        let mut targets = SweepTargets {
            faces_higher: Some(vec![Some((source_id, face))]),
            ..Default::default()
        };
        let mut geometry = TranslationalSweepGeometry::new(solidcore::math::Vec3::new(0., 0., 3.));
        geometry.base.eps = 5e-8;
        let mut sweep = TopologySweep::default();
        sweep.do_sweep(
            &mut model,
            &SweepGeometry::Translation(geometry),
            Some(source_id),
            Some((PrimitiveModel::MAIN, region)),
            &mut targets,
        )?;
        let copied_face = sweep
            .get_to_same(Some(SweepEntity::Face(source_id, face)))
            .ok_or("missing copied cap provenance")?;
        let copied_edge = sweep
            .get_to_same(Some(SweepEntity::Edge(source_id, edge)))
            .ok_or("missing copied edge provenance")?;
        let SweepEntity::Face(destination, cap) = copied_face else {
            return Err("wrong copied cap dimension".into());
        };
        let SweepEntity::Edge(edge_destination, edge) = copied_edge else {
            return Err("wrong copied edge dimension".into());
        };
        assert_eq!(destination, edge_destination);
        let loops =
            solidcore::query::api_queries::face_get_boundary_loops(model.brep(destination)?, cap)?;
        assert_eq!(loops.edges.iter().filter(|&&e| e == edge).count(), 1);
        assert_eq!(
            sweep.get_from_same(Some(copied_face)),
            Some(SweepEntity::Face(source_id, face))
        );
        // No coordinate/normal/descriptor/native ordinal chooses either copied
        // entity. Both come from the original owner-qualified construction nodes.
    }
    assert_ne!(native_faces[0], native_faces[1]);
    Ok(())
}

/// Copy one actual filled face, retaining the native operand-qualified mate map.
/// The returned map is used only during this construction, never as saved identity.
fn copy_profile_with_lineage(
    kernel: &mut Kernel,
    source: &solidcore::Brep,
    face: solidcore::FaceId,
    span: f64,
    tolerance: f64,
) -> Result<
    (
        solidcore::Brep,
        solidcore::FaceId,
        solidcore::boolean::topology_intersector::p1::Relations,
    ),
    crate::NewellKernelError,
> {
    use crate::operation_error;
    use solidcore::boolean::merge::{
        p1::VertexCopy,
        p2::{MergeContext, TopologyCopy},
    };
    use solidcore::boolean::topology_intersector::p1::Relations;
    use solidcore::error::EntityRef;
    use solidcore::topology::brep::p2::ArenaNotifyState;
    let mut input = source.clone_standing_for_self().map_err(operation_error)?;
    let mut output =
        solidcore::topology::brep::p3::new_brep(kernel, span, 1000.).map_err(operation_error)?;
    output.set_tolerance(tolerance);
    let mut state = ArenaNotifyState::new(&input);
    let env = state.env(None);
    let source_key = env.key_of;
    // Native arena ids occupy the low 32 bits of these per-kind keys. A separate
    // operand namespace prevents attributes/notifications aliasing across owners.
    let destination_key = |entity| {
        let mut key = source_key(entity);
        key.key.0 |= 1_u64 << 48;
        key
    };
    let mut copy = TopologyCopy {
        vertices: VertexCopy {
            records: env.records,
            attributes: env.attributes,
            source_key: &|v| source_key(EntityRef::Vertex(v)),
            destination_key: &destination_key,
        },
        edge_source_key: &|e| source_key(EntityRef::Edge(e)),
        edge_destination_key: &|e| destination_key(EntityRef::Edge(e)),
        face_source_key: &|f| source_key(EntityRef::Face(f)),
        face_destination_key: &|f| destination_key(EntityRef::Face(f)),
        surface_attributes: false,
    };
    let mut relations = Relations::default();
    let source_use = solidcore::topology::face::p2::upward_faceuse(&input, face)
        .map_err(operation_error)?
        .ok_or_else(|| operation_error("missing source face use"))?;
    let region = output.infinite_region();
    let mut made = Vec::new();
    // The wrapper classifies into an empty owner and silently ignores a soft
    // classification failure. Its public lower route accepts the known destination
    // infinite region directly and propagates the actual copy status.
    solidcore::boolean::merge::p2::make_faceuse_with(
        MergeContext::Split(kernel, None),
        &mut output,
        &mut input,
        source_use,
        region,
        &mut relations,
        &mut copy,
        Some(&mut made),
    )
    .map_err(operation_error)?;
    let [copied] = made.as_slice() else {
        return Err(operation_error("filled profile copy is not unique"));
    };
    if relations.faces.primary(face) != Some(*copied)
        || relations.faces.other(*copied) != Some(face)
    {
        return Err(operation_error(
            "missing reciprocal profile copy provenance",
        ));
    }
    Ok((output, *copied, relations))
}

type NativeBoundary = std::collections::BTreeMap<(u32, u32), solidcore::EdgeId>;
type ResolvedNativeFace = (solidcore::FaceId, NativeBoundary);

type NativeOccurrences =
    std::collections::BTreeMap<(crate::NewellFaceProvenance, u32, u32), solidcore::EdgeId>;
struct NativeNamedBody {
    body: solidcore::Brep,
    roles: std::collections::BTreeMap<crate::NewellFaceProvenance, solidcore::FaceId>,
    occurrence_map: NativeOccurrences,
}

#[cfg(any(test, feature = "test-support"))]
fn capped_sweep_with_lineage(
    kernel: &mut Kernel,
    source: &solidcore::Brep,
    face: solidcore::FaceId,
    occurrences: &std::collections::BTreeMap<(u32, u32), solidcore::EdgeId>,
    delta: [f64; 3],
) -> Result<NativeNamedBody, crate::NewellKernelError> {
    capped_sweep_between_with_lineage(kernel, source, face, occurrences, [0.; 3], delta)
}

fn capped_sweep_between_with_lineage(
    kernel: &mut Kernel,
    source: &solidcore::Brep,
    face: solidcore::FaceId,
    occurrences: &std::collections::BTreeMap<(u32, u32), solidcore::EdgeId>,
    offset: [f64; 3],
    delta: [f64; 3],
) -> Result<NativeNamedBody, crate::NewellKernelError> {
    use crate::{NewellFaceProvenance as P, operation_error};
    use solidcore::primitive::primitive_creation::model::PrimitiveModel;
    use solidcore::sweep::{
        topology_sweep::{
            SweepEntity, TopologySweep, access::SweepAccess, engine::SweepTargets,
            geometry::SweepGeometry,
        },
        trans_sweep_geometry::TranslationalSweepGeometry,
    };
    if offset.iter().chain(delta.iter()).any(|v| !v.is_finite())
        || delta.iter().map(|v| v * v).sum::<f64>().sqrt() <= 2. * source.tolerance()
    {
        return Err(operation_error("invalid capped sweep displacement"));
    }
    let face_bounds =
        solidcore::query::face::p1::calculate_bounding_box(&*kernel, source, face, true)
            .map_err(operation_error)?;
    let shifted_bounds = solidcore::geom::extent3d::Extent3d::from_bounds(
        face_bounds.min_at(0) + offset[0],
        face_bounds.min_at(1) + offset[1],
        face_bounds.min_at(2) + offset[2],
        face_bounds.max_at(0) + offset[0],
        face_bounds.max_at(1) + offset[1],
        face_bounds.max_at(2) + offset[2],
    );
    let (span, eps, zone) = sweep_native_policy(kernel, source, &shifted_bounds, delta)?;
    if source.tolerance() > eps / 2. {
        return Err(operation_error(
            "source precision exceeds mapped sweep policy",
        ));
    }
    let (mut output, start, copied) = copy_profile_with_lineage(kernel, source, face, span, zone)?;
    if offset.iter().any(|&v| v != 0.) {
        let placement = solidcore::geom::axis2_placement::Axis2Placement::new(
            solidcore::math::Point3::new(offset[0], offset[1], offset[2]),
            solidcore::math::Vec3::X,
            solidcore::math::Vec3::Y,
        );
        solidcore::topology::brep::p5::transform(kernel, &mut output, &placement, None, false)
            .map_err(operation_error)?;
        assert_native_tolerances(&output, eps / 2.)?;
    }
    let region = output
        .infinite_region()
        .ok_or_else(|| operation_error("missing sweep region"))?;
    let tolerance = output.tolerance();
    let mut model = PrimitiveModel::new(kernel, output);
    let main = PrimitiveModel::MAIN;
    let mut targets = SweepTargets {
        faces_higher: Some(vec![Some((main, start))]),
        ..Default::default()
    };
    let mut geometry =
        TranslationalSweepGeometry::new(solidcore::math::Vec3::new(delta[0], delta[1], delta[2]));
    geometry.base.eps = tolerance;
    let mut sweep = TopologySweep::default();
    sweep
        .do_sweep(
            &mut model,
            &SweepGeometry::Translation(geometry),
            Some(main),
            Some((main, region)),
            &mut targets,
        )
        .map_err(operation_error)?;
    // TopologySweep unifies its result; a material needs outward orientation too.
    // Keep this on the exact native B-Rep, never repair renderer normals alone.
    model.orient(main, true, false).map_err(operation_error)?;
    let end = sweep
        .get_to_same(Some(SweepEntity::Face(main, start)))
        .ok_or_else(|| operation_error("missing copied cap lineage"))?;
    let SweepEntity::Face(end_owner, end_face) = end else {
        return Err(operation_error("invalid cap lineage dimension"));
    };
    if end_owner != main || sweep.get_from_same(Some(end)) != Some(SweepEntity::Face(main, start)) {
        return Err(operation_error("nonreciprocal cap lineage"));
    }
    let mut roles = std::collections::BTreeMap::from([(P::StartCap, start), (P::EndCap, end_face)]);
    let mut occurrence_map = NativeOccurrences::new();
    for (&(wire, edge), &original) in occurrences {
        let native = copied
            .edges
            .primary(original)
            .ok_or_else(|| operation_error("lost source edge copy"))?;
        if copied.edges.other(native) != Some(original) {
            return Err(operation_error("nonreciprocal source edge copy"));
        }
        let source_edge = SweepEntity::Edge(main, native);
        let wall = sweep
            .get_to_higher(Some(source_edge))
            .ok_or_else(|| operation_error("missing wall lineage"))?;
        let SweepEntity::Face(wall_owner, wall_face) = wall else {
            return Err(operation_error("invalid wall lineage dimension"));
        };
        if wall_owner != main || sweep.get_from_lower(Some(wall)) != Some(source_edge) {
            return Err(operation_error("nonreciprocal wall lineage"));
        }
        roles.insert(P::Wall { wire, edge }, wall_face);
        let end = sweep
            .get_to_same(Some(source_edge))
            .ok_or_else(|| operation_error("missing cap edge lineage"))?;
        let SweepEntity::Edge(end_owner, end_edge) = end else {
            return Err(operation_error("invalid cap edge lineage dimension"));
        };
        if end_owner != main || sweep.get_from_same(Some(end)) != Some(source_edge) {
            return Err(operation_error("nonreciprocal cap edge lineage"));
        }
        occurrence_map.insert((P::StartCap, wire, edge), native);
        occurrence_map.insert((P::EndCap, wire, edge), end_edge);
        let role = P::Wall { wire, edge };
        occurrence_map.insert((role, 0, 0), native);
        occurrence_map.insert((role, 0, 2), end_edge);
        let source_boundary = solidcore::query::api_queries::face_get_boundary_loops(source, face)
            .map_err(operation_error)?;
        let positions: Vec<_> = source_boundary
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, &e)| (e == original).then_some(i))
            .collect();
        let [position] = positions.as_slice() else {
            return Err(operation_error("source edge occurrence is ambiguous"));
        };
        let source_record = source.edge(original).map_err(operation_error)?;
        let mut vertices = source_record.vertices;
        if source_boundary.orientations[*position]
            == solidcore::nurbs::b_spline_surface_n_lib::p2::OrientType::Opposite
        {
            vertices.reverse();
        }
        for (endpoint, key) in [(vertices[1], 1), (vertices[0], 3)] {
            let copied_vertex = copied
                .vertices
                .primary(endpoint)
                .ok_or_else(|| operation_error("missing copied endpoint lineage"))?;
            if copied.vertices.other(copied_vertex) != Some(endpoint) {
                return Err(operation_error("nonreciprocal copied endpoint"));
            }
            let entity = SweepEntity::Vertex(main, copied_vertex);
            let side = sweep
                .get_to_higher(Some(entity))
                .ok_or_else(|| operation_error("missing longitudinal edge lineage"))?;
            let SweepEntity::Edge(side_owner, side_edge) = side else {
                return Err(operation_error("invalid longitudinal lineage dimension"));
            };
            if side_owner != main || sweep.get_from_lower(Some(side)) != Some(entity) {
                return Err(operation_error("nonreciprocal longitudinal lineage"));
            }
            occurrence_map.insert((role, 0, key), side_edge);
        }
    }
    let mut body = model.into_brep().map_err(operation_error)?;
    body.refresh_cached_topology();
    let unique: std::collections::BTreeSet<_> = roles.values().copied().collect();
    if unique.len() != roles.len()
        || body.face_list().len() != roles.len()
        || !solidcore::query::api_queries::brep_is_manifold_solid(&body).map_err(operation_error)?
    {
        return Err(operation_error(
            "sweep does not have uniquely named manifold faces",
        ));
    }
    for &f in roles.values() {
        body.face(f).map_err(operation_error)?;
    }
    for (&(role, _, _), &e) in &occurrence_map {
        let face = roles
            .get(&role)
            .ok_or_else(|| operation_error("missing cap role"))?;
        let boundary = solidcore::query::api_queries::face_get_boundary_loops(&body, *face)
            .map_err(operation_error)?;
        let named_count = occurrence_map
            .iter()
            .filter(|((r, _, _), native)| *r == role && **native == e)
            .count();
        if boundary.edges.iter().filter(|&&id| id == e).count() != named_count {
            return Err(operation_error(
                "lost or ambiguous constructed occurrence after stitching",
            ));
        }
    }
    assert_native_tolerances(&body, eps / 2.)?;
    Ok(NativeNamedBody {
        body,
        roles,
        occurrence_map,
    })
}

impl MigrationGeometry {
    #[cfg(test)]
    fn sweep_face_delta(
        &mut self,
        key: crate::NewellShapeKey,
        slot: u32,
        delta: [f64; 3],
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        let length = delta.iter().map(|v| v * v).sum::<f64>().sqrt();
        self.extrude_face(key, slot, Some(delta), 0., length)
    }
}

#[test]
fn solidcore_migration_capped_sweep_preserves_native_lineage_and_geometry()
-> Result<(), Box<dyn std::error::Error>> {
    let mut owner = MigrationGeometry::new(10.);
    let source = owner.box_shape([10., 6., 5.], [0.; 3])?;
    let key = owner.sweep_face_delta(source, 1, [0., 0., 3.])?;
    owner.tag_extrude(key, (source, 1), [0., 0., 1.])?;
    assert!(owner.tag_extrude(key, (source, 0), [0., 0., 1.]).is_err());
    assert!(owner.tag_extrude(key, (source, 1), [1., 0., 0.]).is_err());
    let body = owner.objects.get_mut(&key).ok_or("missing swept owner")?;
    let attrs = solidcore::geom::attribute::AttributeStore::default();
    let key_of = |_: solidcore::error::EntityRef| solidcore::geom::attribute::ObjectKey(1);
    let attrs = solidcore::mass::integrator::properties::PropertyAttributes {
        store: &attrs,
        key_of: &key_of,
    };
    let volume = solidcore::mass::integrator::properties::brep_volume(
        &owner.kernel,
        body,
        PROPERTY_RELATIVE_ACCURACY,
        &attrs,
    )?;
    assert!((volume - 180.).abs() < 1e-6, "native volume={volume}");
    let mesh = owner.tessellate(key, 0.01)?;
    let mesh_volume = oriented_mesh_volume(&mesh)?;
    assert!(
        (mesh_volume - 180.).abs() < 1e-5,
        "mesh volume={mesh_volume}"
    );
    assert_eq!(mesh.face_count(), 6);
    assert_eq!(owner.objects.len(), 2);
    let metadata = owner.profile_faces(key)?;
    assert_eq!(metadata.len(), 6);
    assert!(
        metadata
            .iter()
            .all(|f| f.wires.len() == 1 && f.wires[0].edges.len() == 4)
    );
    let end_slot = metadata
        .iter()
        .find(|f| f.provenance == crate::NewellFaceProvenance::EndCap)
        .ok_or("missing end descriptor")?
        .face;
    let nested = owner.sweep_face_delta(key, end_slot, [0., 0., 2.])?;
    let nested_mesh = owner.tessellate(nested, 0.01)?;
    assert!((oriented_mesh_volume(&nested_mesh)? - 120.).abs() < 1e-5);
    assert_eq!(owner.profile_faces(nested)?.len(), 6);
    let count = owner.objects.len();
    let next = owner.next;
    assert!(owner.sweep_face_delta(source, 99, [0., 0., 3.]).is_err());
    assert!(owner.sweep_face_delta(source, 1, [0.; 3]).is_err());
    assert_eq!(owner.objects.len(), count);
    assert_eq!(owner.next, next);
    Ok(())
}

/// Display a material boundary from its unique void-side use. Native reflection
/// exchanges shells without changing the primary use. Sheets have the same
/// region on both sides and retain the constructor's primary orientation.
pub(super) fn native_display_faceuse(
    body: &solidcore::Brep,
    face: solidcore::FaceId,
) -> Result<solidcore::topology::FaceuseId, crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::topology::edgeuse::p1::faceuse_region;
    let (primary, mate) =
        solidcore::topology::face::p2::faceuses(body, face).map_err(operation_error)?;
    let primary = primary.ok_or_else(|| operation_error("missing primary face use"))?;
    let mate = mate.ok_or_else(|| operation_error("missing mate face use"))?;
    let region = |fu| {
        faceuse_region(body, fu)
            .map_err(operation_error)?
            .ok_or_else(|| operation_error("missing face use region"))
    };
    let a = region(primary)?;
    let b = region(mate)?;
    if a == b {
        return Ok(primary);
    }
    let a_void = body.region(a).map_err(operation_error)?.is_void;
    let b_void = body.region(b).map_err(operation_error)?.is_void;
    match (a_void, b_void) {
        (1, 0) | (0, 0) | (1, 1) => Ok(primary),
        (0, 1) => Ok(mate),
        _ => Err(operation_error(
            "uncertified face use material classification",
        )),
    }
}

fn native_planar_frame(
    kernel: &Kernel,
    body: &solidcore::Brep,
    face: solidcore::FaceId,
) -> Result<crate::NewellProfileFrame, crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::surface::surface::SurfaceKind;
    let surface = solidcore::topology::face::p2::surface(body, face)
        .map_err(operation_error)?
        .ok_or_else(|| operation_error("missing face surface"))?;
    let planar = surface
        .is_planar(kernel, body.tolerance())
        .map_err(operation_error)?;
    if !planar.is_planar || planar.deviation > body.tolerance() {
        return Err(operation_error("profile support is not planar"));
    }
    let (point, normal) = if let SurfaceKind::Plane(plane) = surface.kind() {
        (plane.position().origin(), plane.evaluate_normal())
    } else {
        let uv = surface
            .natural_uv_domain_with_nurb(kernel)
            .map_err(operation_error)?
            .mid();
        (
            planar.point,
            surface
                .evaluate_normal(
                    kernel,
                    uv,
                    solidcore::nurbs::knots::Side::Left,
                    solidcore::nurbs::knots::Side::Left,
                )
                .map_err(operation_error)?,
        )
    };
    let fu = native_display_faceuse(body, face)?;
    let sign = if body.faceuse(fu).map_err(operation_error)?.orientation
        == solidcore::nurbs::b_spline_surface_n_lib::p2::OrientType::Opposite
    {
        -1.
    } else {
        1.
    };
    let length = normal.length();
    if !length.is_finite() || length <= 1e-12 {
        return Err(operation_error("invalid plane normal"));
    }
    let n = [normal.x, normal.y, normal.z].map(|v| v * sign / length);
    let h = n[0] * point.x + n[1] * point.y + n[2] * point.z;
    let mut axis = 0;
    for i in 1..3 {
        if n[i].abs() < n[axis].abs() {
            axis = i;
        }
    }
    let mut u = n.map(|x| -x * n[axis]);
    u[axis] += 1.;
    let length = u.iter().map(|x| x * x).sum::<f64>().sqrt();
    let u = u.map(|x| x / length);
    let v = [
        n[1] * u[2] - n[2] * u[1],
        n[2] * u[0] - n[0] * u[2],
        n[0] * u[1] - n[1] * u[0],
    ];
    Ok(crate::NewellProfileFrame {
        origin: n.map(|x| x * h),
        normal: n,
        u,
        v,
    })
}

impl MigrationGeometry {
    fn source_wire(&self, key: crate::NewellShapeKey) -> Option<u32> {
        self.source_wires.get(&key).copied()
    }

    fn fixed_surface_extent(
        &mut self,
        profile: (crate::NewellShapeKey, u32),
        boundary: crate::NewellBoundary,
        direction: [f64; 3],
        start: f64,
        extent: &crate::NewellSurfaceExtent,
        sign: f64,
    ) -> Result<f64, crate::NewellKernelError> {
        use crate::{NewellKernelError, NewellSurfaceExtent as Extent, operation_error};
        let signed_direction = direction.map(|value| value * sign);
        let station = match extent {
            Extent::Blind(depth) if depth.is_finite() && *depth > 1e-7 => start + sign * depth,
            Extent::Blind(_) => return Err(operation_error("invalid surface depth")),
            Extent::UpTo(target) => {
                if !target.clearance.is_finite() {
                    return Err(operation_error("invalid target clearance"));
                }
                sign * (self.starting_station(
                    profile,
                    signed_direction,
                    (target.shape, target.face),
                )? - target.clearance)
            }
            Extent::ThroughAll(targets) => {
                sign * self.through_all_surface_station(
                    profile,
                    boundary,
                    signed_direction,
                    sign * start,
                    targets,
                )?
            }
        };
        if !station.is_finite() || sign * (station - start) <= 1e-7 {
            return Err(NewellKernelError::InvalidParameter {
                parameter: "target is not beyond the displaced start",
            });
        }
        Ok(station)
    }

    fn extrude_surface(
        &mut self,
        request: crate::NewellSurfaceExtrude,
    ) -> Result<Vec<crate::NewellShapeKey>, crate::NewellKernelError> {
        use crate::{NewellSurfaceExtent as Extent, NewellSurfaceSides as Sides};
        self.check_cancelled()?;
        let start = match request.start {
            crate::NewellSurfaceStart::Distance(value) if value.is_finite() => value,
            crate::NewellSurfaceStart::Distance(_) => {
                return Err(crate::operation_error("invalid surface start"));
            }
            crate::NewellSurfaceStart::UpTo(target) => self.starting_station(
                request.profile,
                request.direction,
                (target.shape, target.face),
            )?,
        };
        match (request.sides, request.extent) {
            (Sides::Symmetric, extent) => {
                let end = self.fixed_surface_extent(
                    request.profile,
                    request.boundary,
                    request.direction,
                    start,
                    &extent,
                    1.,
                )?;
                self.sweep_surface(
                    request.profile,
                    request.boundary,
                    request.direction,
                    2. * start - end,
                    end,
                )
            }
            (
                Sides::Second {
                    extent: Extent::UpTo(lower),
                    reverse: false,
                },
                Extent::UpTo(upper),
            ) if self.surface_end_is_tilted(request.profile, lower)?
                || self.surface_end_is_tilted(request.profile, upper)? =>
            {
                self.sweep_surface_between(
                    request.profile,
                    request.boundary,
                    request.direction,
                    SheetStart::Face(lower),
                    upper,
                )
            }
            (Sides::Second { extent, reverse }, main) => {
                let main = self.fixed_surface_extent(
                    request.profile,
                    request.boundary,
                    request.direction,
                    start,
                    &main,
                    1.,
                )?;
                let second = self.fixed_surface_extent(
                    request.profile,
                    request.boundary,
                    request.direction,
                    start,
                    &extent,
                    if reverse { 1. } else { -1. },
                )?;
                let (low, high) = if reverse {
                    (start, main.max(second))
                } else {
                    (second, main)
                };
                self.sweep_surface(
                    request.profile,
                    request.boundary,
                    request.direction,
                    low,
                    high,
                )
            }
            (Sides::One, Extent::Blind(depth)) => {
                let end = self.fixed_surface_extent(
                    request.profile,
                    request.boundary,
                    request.direction,
                    start,
                    &Extent::Blind(depth),
                    1.,
                )?;
                self.sweep_surface(
                    request.profile,
                    request.boundary,
                    request.direction,
                    start,
                    end,
                )
            }
            (Sides::One, Extent::UpTo(target)) => self.sweep_surface_to_face(
                request.profile,
                request.boundary,
                request.direction,
                start,
                (target.shape, target.face),
                target.clearance,
            ),
            (Sides::One, Extent::ThroughAll(targets)) => {
                let end = self.through_all_surface_station(
                    request.profile,
                    request.boundary,
                    request.direction,
                    start,
                    &targets,
                )?;
                self.sweep_surface(
                    request.profile,
                    request.boundary,
                    request.direction,
                    start,
                    end,
                )
            }
        }
    }

    fn tag_extrude(
        &self,
        key: crate::NewellShapeKey,
        profile: (crate::NewellShapeKey, u32),
        direction: [f64; 3],
    ) -> Result<(), crate::NewellKernelError> {
        use crate::{NewellFaceProvenance as Role, operation_error};
        self.check_cancelled()?;
        let source = self
            .profile_faces(profile.0)?
            .into_iter()
            .find(|face| face.face == profile.1)
            .ok_or_else(|| operation_error("missing extrusion profile"))?;
        let length = direction
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        if !length.is_finite()
            || length <= 0.
            || source
                .frame
                .normal
                .into_iter()
                .zip(direction)
                .map(|(a, b)| a * b)
                .sum::<f64>()
                .abs()
                <= length * 1e-12
        {
            return Err(operation_error("invalid extrusion direction"));
        }
        let output = self
            .objects
            .get(&key)
            .ok_or_else(|| operation_error("missing extrusion output"))?;
        if self.extrusion_sources.get(&key).copied() != Some(profile) {
            return Err(operation_error("extrusion source identity is unproven"));
        }
        let roles = self
            .roles
            .get(&key)
            .ok_or_else(|| operation_error("extrusion has no proven face lineage"))?;
        let source_edges: std::collections::BTreeSet<_> = source
            .wires
            .iter()
            .flat_map(|wire| wire.edges.iter().map(move |edge| (wire.wire, edge.edge)))
            .collect();
        let walls: std::collections::BTreeSet<_> = roles
            .keys()
            .filter_map(|role| match role {
                Role::Wall { wire, edge } => Some((*wire, *edge)),
                _ => None,
            })
            .collect();
        if walls.is_empty()
            || !walls.is_subset(&source_edges)
            || (!self.is_surface(key)
                && (!roles.contains_key(&Role::StartCap)
                    || !roles.contains_key(&Role::EndCap)
                    || walls != source_edges))
        {
            return Err(operation_error("incomplete extrusion face lineage"));
        }
        for face in roles.values() {
            output.face(*face).map_err(operation_error)?;
        }
        Ok(())
    }

    fn direction(
        &self,
        key: crate::NewellShapeKey,
        face: u32,
        wire: u32,
        edge: u32,
    ) -> Result<[f64; 3], crate::NewellKernelError> {
        let support = self
            .profile_faces(key)?
            .into_iter()
            .find(|candidate| candidate.face == face)
            .ok_or_else(|| crate::operation_error("direction support"))?;
        let edge = support
            .wires
            .iter()
            .find(|candidate| candidate.wire == wire)
            .and_then(|wire| {
                wire.edges
                    .iter()
                    .find(|candidate| candidate.edge == edge && candidate.straight)
            })
            .ok_or_else(|| crate::operation_error("direction edge"))?;
        let delta = std::array::from_fn::<_, 3, _>(|axis| edge.end[axis] - edge.start[axis]);
        let length = delta
            .iter()
            .map(|component| component * component)
            .sum::<f64>()
            .sqrt();
        if !length.is_finite() || length <= 0. {
            return Err(crate::operation_error("degenerate direction edge"));
        }
        Ok(delta.map(|component| component / length))
    }

    fn slots(
        &self,
        key: crate::NewellShapeKey,
    ) -> Result<Vec<solidcore::FaceId>, crate::NewellKernelError> {
        self.roles
            .get(&key)
            .map(|r| r.values().copied().collect())
            .or_else(|| self.anonymous_faces.get(&key).cloned())
            .ok_or_else(|| crate::operation_error("missing native face-slot map"))
    }
    fn tessellate(
        &self,
        key: crate::NewellShapeKey,
        deflection: f64,
    ) -> Result<crate::NewellMesh, crate::NewellKernelError> {
        let body = self
            .objects
            .get(&key)
            .ok_or_else(|| crate::operation_error("missing native owner"))?;
        crate::solidcore_mesh::native_body_cancellable(
            &self.kernel,
            body,
            &self.slots(key)?,
            deflection,
            &|| self.check_cancelled(),
        )
    }
    fn profile_faces(
        &self,
        key: crate::NewellShapeKey,
    ) -> Result<Vec<crate::NewellProfileFace>, crate::NewellKernelError> {
        use crate::operation_error;
        let body = self
            .objects
            .get(&key)
            .ok_or_else(|| operation_error("missing native owner"))?;
        let slots = self.slots(key)?;
        let mut result = Vec::new();
        for (slot, face) in slots.into_iter().enumerate() {
            self.check_cancelled()?;
            let role = self
                .roles
                .get(&key)
                .and_then(|r| {
                    r.iter()
                        .find_map(|(&role, &id)| (id == face).then_some(role))
                })
                .unwrap_or(crate::NewellFaceProvenance::Unknown);
            let occurrences = if role == crate::NewellFaceProvenance::Unknown {
                initial_boundary_occurrences(body, face)?
            } else {
                self.occurrences
                    .get(&key)
                    .map(|all| {
                        all.iter()
                            .filter_map(|(&(r, w, e), &id)| (r == role).then_some(((w, e), id)))
                            .collect()
                    })
                    .ok_or_else(|| operation_error("missing occurrence map"))?
            };
            let surface = solidcore::topology::face::p2::surface(body, face)
                .map_err(operation_error)?
                .ok_or_else(|| operation_error("missing profile surface"))?;
            if !surface
                .is_planar(&self.kernel, body.tolerance())
                .map_err(operation_error)?
                .is_planar
            {
                continue;
            }
            let boundary = solidcore::query::api_queries::face_get_boundary_loops(body, face)
                .map_err(operation_error)?;
            let mut wires = Vec::new();
            let mut offset = 0;
            for (wire, &count) in boundary.edge_counts.iter().enumerate() {
                let mut edges = Vec::new();
                for occurrence in offset..offset + count {
                    let id = boundary.edges[occurrence];
                    let names: Vec<_> = occurrences
                        .iter()
                        .filter_map(|(&(w, e), &native)| (native == id).then_some((w, e)))
                        .collect();
                    let [(wire_key, edge_key)] = names.as_slice() else {
                        return Err(operation_error("missing or ambiguous edge occurrence"));
                    };
                    if usize::try_from(*wire_key).map_err(operation_error)? != wire {
                        return Err(operation_error("wire correspondence changed"));
                    }
                    let edge = body.edge(id).map_err(operation_error)?;
                    let a = body
                        .vertex(edge.vertices[0])
                        .map_err(operation_error)?
                        .point;
                    let b = body
                        .vertex(edge.vertices[1])
                        .map_err(operation_error)?
                        .point;
                    let (a, b) = if boundary.orientations[occurrence]
                        == solidcore::nurbs::b_spline_surface_n_lib::p2::OrientType::Same
                    {
                        (a, b)
                    } else {
                        (b, a)
                    };
                    let straight = edge.curve.as_ref().is_some_and(|c| match c.kind() {
                        solidcore::curve::curve::CurveKind::Line(_) => true,
                        solidcore::curve::curve::CurveKind::BSpline(c) => {
                            c.gw_nurb().is_some_and(|n| n.degree() == 1 && n.n() == 1)
                        }
                        _ => false,
                    });
                    let cubic = edge.curve.as_ref().and_then(|curve| {
                        let solidcore::curve::curve::CurveKind::BSpline(spline) = curve.kind()
                        else {
                            return None;
                        };
                        let native = spline.gw_nurb()?;
                        let &[lo, lo1, lo2, lo3, hi, hi1, hi2, hi3] = native.knots() else {
                            return None;
                        };
                        if native.degree() != 3
                            || lo >= hi
                            || [lo1, lo2, lo3] != [lo; 3]
                            || [hi1, hi2, hi3] != [hi; 3]
                            || edge.interval.min() < lo
                            || edge.interval.max() > hi
                        {
                            return None;
                        }
                        let (points, weights) = spline.control_polygon().ok()?;
                        let [p0, p1, p2, p3] = points.as_slice() else {
                            return None;
                        };
                        if !weights.is_empty() {
                            let &[w0, w1, w2, w3] = weights.as_slice() else {
                                return None;
                            };
                            if w0 <= 0.0 || !w0.is_finite() || [w1, w2, w3] != [w0; 3] {
                                return None;
                            }
                        }
                        let controls = [p0, p1, p2, p3].map(|p| [p.x, p.y, p.z]);
                        let t0 = (edge.interval.min() - lo) / (hi - lo);
                        let t1 = (edge.interval.max() - lo) / (hi - lo);
                        let controls = trim_cubic_controls(controls, t0, t1)?;
                        let distance = |p: [f64; 3], q: solidcore::math::Point3| {
                            (p[0] - q.x).hypot(p[1] - q.y).hypot(p[2] - q.z)
                        };
                        let tol = body.tolerance();
                        if distance(controls[0], a) <= tol && distance(controls[3], b) <= tol {
                            Some(controls)
                        } else if distance(controls[3], a) <= tol && distance(controls[0], b) <= tol
                        {
                            Some([controls[3], controls[2], controls[1], controls[0]])
                        } else {
                            None
                        }
                    });
                    edges.push(crate::NewellProfileEdge {
                        edge: *edge_key,
                        start: [a.x, a.y, a.z],
                        end: [b.x, b.y, b.z],
                        straight,
                        cubic,
                    });
                }
                // Constructor occurrence order is stable when native loop heads move.
                // This order is the explicit box construction cycle, not a geometry sort.
                edges.sort_by_key(|e| e.edge);
                if edges
                    .iter()
                    .zip(edges.iter().cycle().skip(1))
                    .any(|(a, b)| a.end != b.start)
                {
                    edges.reverse();
                }
                if edges
                    .iter()
                    .zip(edges.iter().cycle().skip(1))
                    .any(|(a, b)| a.end != b.start)
                {
                    return Err(operation_error(
                        "constructed boundary is not a closed oriented cycle",
                    ));
                }
                wires.push(crate::NewellProfileWire {
                    wire: u32::try_from(wire).map_err(operation_error)?,
                    closed: true,
                    edges,
                });
                offset += count;
            }
            result.push(crate::NewellProfileFace {
                face: u32::try_from(slot).map_err(operation_error)?,
                provenance: role,
                frame: native_planar_frame(&self.kernel, body, face)?,
                wires,
            });
        }
        Ok(result)
    }
}

#[test]
fn solidcore_migration_profile_metadata_matches_picking_slots()
-> Result<(), Box<dyn std::error::Error>> {
    let mut owners = [MigrationGeometry::new(20.), MigrationGeometry::new(20.)];
    let mut profiles = Vec::new();
    for (owner, order) in owners
        .iter_mut()
        .zip([[0, 1, 2, 3, 4, 5], [5, 4, 3, 2, 1, 0]])
    {
        let key = owner.box_shape_in_order([10., 6., 5.], [3., 4., 5.], order)?;
        let faces = owner.profile_faces(key)?;
        assert_eq!(faces.len(), 6);
        for deflection in [0.05, 0.01] {
            let mesh = owner.tessellate(key, deflection)?;
            assert_eq!(mesh.face_count(), faces.len());
            for face in &faces {
                let slot = usize::try_from(face.face)?;
                let start = usize::try_from(mesh.face_offsets()[slot])?;
                let end = usize::try_from(mesh.face_offsets()[slot + 1])?;
                assert!(start < end);
                for &index in &mesh.indices()[start..end] {
                    let p = mesh.positions()[usize::try_from(index)?].map(f64::from);
                    let d: f64 = (0..3)
                        .map(|i| (p[i] - face.frame.origin[i]) * face.frame.normal[i])
                        .sum();
                    assert!(d.abs() < 1e-6);
                    let n = mesh.normals()[usize::try_from(index)?].map(f64::from);
                    assert!((0..3).map(|i| n[i] * face.frame.normal[i]).sum::<f64>() > 0.99999);
                }
                assert_eq!(face.wires.len(), 1);
                assert_eq!(face.wires[0].edges.len(), 4);
                assert!(face.wires[0].edges.iter().all(|e| e.straight));
                for edge in &face.wires[0].edges {
                    let direction =
                        owner.direction(key, face.face, face.wires[0].wire, edge.edge)?;
                    let delta =
                        std::array::from_fn::<_, 3, _>(|axis| edge.end[axis] - edge.start[axis]);
                    let projection: f64 =
                        direction.into_iter().zip(delta).map(|(a, b)| a * b).sum();
                    assert!(projection > 0.);
                    assert!((direction.into_iter().map(|x| x * x).sum::<f64>() - 1.).abs() < 1e-12);
                }
            }
        }
        profiles.push(faces);
    }
    // Exact recipe and provenance give identical Newell descriptors despite
    // different native face ids, loop heads and shared-edge construction order.
    assert_eq!(profiles[0], profiles[1]);
    Ok(())
}

/// Assign fresh generation-local occurrences to an exact newly constructed input.
/// These names are not a cross-generation proof: copy/sweep output uses reciprocal
/// native maps and recipe roles. The native loop's repeated edges are retained.
fn initial_boundary_occurrences(
    body: &solidcore::Brep,
    face: solidcore::FaceId,
) -> Result<std::collections::BTreeMap<(u32, u32), solidcore::EdgeId>, crate::NewellKernelError> {
    let boundary = solidcore::query::api_queries::face_get_boundary_loops(body, face)
        .map_err(crate::operation_error)?;
    let mut map = std::collections::BTreeMap::new();
    let mut offset = 0;
    for (wire, &count) in boundary.edge_counts.iter().enumerate() {
        for edge in 0..count {
            map.insert(
                (
                    u32::try_from(wire).map_err(crate::operation_error)?,
                    u32::try_from(edge).map_err(crate::operation_error)?,
                ),
                boundary.edges[offset + edge],
            );
        }
        offset += count;
    }
    Ok(map)
}

fn exact_planar_profile(
    kernel: &mut Kernel,
    curves: &[solidcore::curve::curve::Curve],
) -> Result<(solidcore::Brep, solidcore::FaceId), crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::error::EntityRef;
    use solidcore::topology::brep::{p2::ArenaNotifyState, p8::FaceBuildEnv};
    let mut bounds = None;
    for curve in curves {
        let next = curve
            .calculate_tight_bounding_box(kernel, &curve.natural_interval())
            .map_err(operation_error)?;
        bounds = Some(
            bounds.map_or(next, |b: solidcore::geom::extent3d::Extent3d| {
                b.union(&next)
            }),
        );
    }
    let bounds = bounds.ok_or_else(|| operation_error("empty planar profile"))?;
    let span = bounds.max_length();
    let scale = (0..3)
        .flat_map(|i| [bounds.min_at(i).abs(), bounds.max_at(i).abs()])
        .fold(span, f64::max);
    let eps = configure_native_policy(kernel, span, scale)?;
    let tolerance = eps / 2.;
    let mut body =
        solidcore::topology::brep::p3::new_brep(kernel, span, 1000.).map_err(operation_error)?;
    body.set_tolerance(native_construction_zone(eps, scale)?);
    let mut state = ArenaNotifyState::new(&body);
    let mut env = state.env(None);
    let key_of = env.key_of;
    let this_face = |f| key_of(EntityRef::Face(f));
    let mut marks = solidcore::topology::topology_traverser::Marks::default();
    let mut face_env = FaceBuildEnv {
        env: &mut env,
        marks: &mut marks,
        this_face: &this_face,
    };
    let region = body.infinite_region();
    let made = solidcore::topology::brep::p8::create_planar_faces_with_3d_curves(
        kernel,
        &mut body,
        region,
        curves,
        tolerance,
        &mut face_env,
    )
    .map_err(operation_error)?;
    let [face] = made.as_slice() else {
        return Err(operation_error(
            "profile construction must yield exactly one filled face",
        ));
    };
    let face = *face;
    Ok((body, face))
}

fn assert_native_tolerances(
    body: &solidcore::Brep,
    maximum: f64,
) -> Result<(), crate::NewellKernelError> {
    use crate::operation_error;
    let valid = |v: f64| v.is_finite() && v > 0. && v <= maximum;
    if !valid(body.tolerance()) {
        return Err(operation_error("invalid effective owner tolerance"));
    }
    for &f in body.face_list() {
        if !valid(solidcore::topology::face::p2::tolerance(body, f).map_err(operation_error)?) {
            return Err(operation_error("invalid effective face tolerance"));
        }
    }
    for &e in body.edge_list() {
        if !valid(body.edge(e).map_err(operation_error)?.tolerance) {
            return Err(operation_error("invalid effective edge tolerance"));
        }
    }
    for &v in body.vertex_list() {
        if !valid(body.vertex(v).map_err(operation_error)?.tolerance) {
            return Err(operation_error(format!(
                "invalid effective vertex tolerance {} (maximum {})",
                body.vertex(v).map_err(operation_error)?.tolerance,
                maximum
            )));
        }
    }
    Ok(())
}

/// An editable independent native body must own its copied curves and surfaces.
/// Brep::clone resets geometric owners and is not suitable for Boolean scratch.
fn independent_native_copy(
    kernel: &mut Kernel,
    source: &solidcore::Brep,
) -> Result<solidcore::Brep, crate::NewellKernelError> {
    use solidcore::topology::brep::p2::ArenaNotifyState;
    let mut state = ArenaNotifyState::new(source);
    let mut env = state.env(None);
    let source_key = env.key_of;
    let destination_key = |e| {
        let mut key = source_key(e);
        key.key.0 |= 1_u64 << 48;
        key
    };
    env.key_of = &destination_key;
    solidcore::topology::brep::p3::api_brep_copy(kernel, &mut env, Some(source), source_key)
        .map_err(crate::operation_error)
}

type NativeCopyMap =
    solidcore::topology::proto_topology::p1::CopyBrepMap<solidcore::error::EntityRef>;

#[test]
fn solidcore_migration_whole_body_copy_has_reciprocal_occurrence_lineage()
-> Result<(), Box<dyn std::error::Error>> {
    use solidcore::error::EntityRef;
    let mut owner = MigrationGeometry::new(20.);
    let source = owner.box_shape([2., 3., 1.], [3., 4., 5.])?;
    let swept = owner.extrude_face(source, 1, Some([0.6, -0.3, 1.]), -1., 2.)?;
    for key in [source, swept] {
        let input = owner.objects.get(&key).ok_or("source")?;
        let (output, backward, forward) = native_copy_with_lineage(&mut owner.kernel, input)?;
        solidcore::topology::integrity::check(&output)?;
        assert!(solidcore::query::api_queries::brep_is_manifold_solid(
            &output
        )?);
        assert!(
            (corpus_volume(&owner.kernel, &output)? - corpus_volume(&owner.kernel, input)?).abs()
                < 1e-8
        );
        let old_entities = input
            .vertex_list()
            .iter()
            .copied()
            .map(EntityRef::Vertex)
            .chain(input.edge_list().iter().copied().map(EntityRef::Edge))
            .chain(input.face_list().iter().copied().map(EntityRef::Face));
        let mut unique = std::collections::BTreeSet::new();
        for old in old_entities {
            let new = forward.get_at(Some(old))?.ok_or("missing forward copy")?;
            assert_eq!(backward.get_at(Some(new))?, Some(old));
            assert!(unique.insert(new));
            match (old, new) {
                (EntityRef::Vertex(a), EntityRef::Vertex(b)) => {
                    assert_eq!(input.vertex(a)?.point, output.vertex(b)?.point)
                }
                (EntityRef::Edge(a), EntityRef::Edge(b)) => {
                    let expected = input
                        .edge(a)?
                        .vertices
                        .map(|v| forward.get_at(Some(EntityRef::Vertex(v))));
                    for (expected, vertex) in expected.into_iter().zip(output.edge(b)?.vertices) {
                        assert_eq!(expected?, Some(EntityRef::Vertex(vertex)));
                    }
                }
                (EntityRef::Face(a), EntityRef::Face(b)) => {
                    let before = solidcore::query::api_queries::face_get_boundary_loops(input, a)?;
                    let after = solidcore::query::api_queries::face_get_boundary_loops(&output, b)?;
                    assert_eq!(before.edge_counts, after.edge_counts);
                    // Native loop heads may move; compare proven copied edge
                    // occurrences with multiplicity, never spatial resemblance.
                    let mut expected = std::collections::BTreeMap::<_, usize>::new();
                    let mut actual = std::collections::BTreeMap::<_, usize>::new();
                    for e in before.edges {
                        *expected
                            .entry(
                                forward
                                    .get_at(Some(EntityRef::Edge(e)))?
                                    .ok_or("edge copy")?,
                            )
                            .or_default() += 1;
                    }
                    for e in after.edges {
                        *actual.entry(EntityRef::Edge(e)).or_default() += 1;
                    }
                    assert_eq!(expected, actual);
                }
                _ => return Err("copied entity dimension changed".into()),
            }
        }
        assert_eq!(
            unique.len(),
            output.vertex_list().len() + output.edge_list().len() + output.face_list().len()
        );
    }
    Ok(())
}

/// Both maps are emitted by the native copy operation, rather than reconstructed
/// from its arena order. Callers must check reciprocal entries before naming.
fn native_copy_with_lineage(
    kernel: &mut Kernel,
    source: &solidcore::Brep,
) -> Result<(solidcore::Brep, NativeCopyMap, NativeCopyMap), crate::NewellKernelError> {
    use solidcore::topology::brep::p2::ArenaNotifyState;
    let mut state = ArenaNotifyState::new(source);
    let mut env = state.env(None);
    let source_key = env.key_of;
    let destination_key = |e| {
        let mut key = source_key(e);
        key.key.0 |= 1_u64 << 48;
        key
    };
    env.key_of = &destination_key;
    let mut copied_to_source = NativeCopyMap::new();
    let mut source_to_copied = NativeCopyMap::new();
    let mut body = solidcore::topology::brep::p3::copy_brep(
        kernel,
        &mut env,
        source,
        source_key,
        Some(&mut copied_to_source),
        Some(&mut source_to_copied),
        true,
        false,
    )
    .map_err(crate::operation_error)?;
    // This lower copy route leaves derived traversal caches for its caller.
    body.refresh_cached_topology();
    Ok((body, copied_to_source, source_to_copied))
}

/// Factor the literal oblique station reflection into two rigid placements and
/// one diagonal scale. The native placement API is rigid; a single placement
/// cannot express I - 2 d n^T/(n.d). This analytic 2D SVD adds no numerical backend.
fn native_station_reflection(
    frame: crate::NewellProfileFrame,
    direction: [f64; 3],
    start: f64,
) -> Result<
    (
        solidcore::geom::axis2_placement::Axis2Placement,
        solidcore::geom::axis2_placement::Axis2Placement,
        solidcore::math::Vec3,
    ),
    crate::NewellKernelError,
> {
    use crate::operation_error;
    use solidcore::{
        geom::axis2_placement::Axis2Placement,
        math::{Point3, Vec3},
    };
    let dot = |a: [f64; 3], b: [f64; 3]| (0..3).map(|i| a[i] * b[i]).sum::<f64>();
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let norm = |a| dot(a, a).sqrt();
    let length = norm(direction);
    if !start.is_finite() || !length.is_finite() || length <= 1e-12 {
        return Err(operation_error("symmetric frame"));
    }
    let d = direction.map(|v| v / length);
    let n = frame.normal;
    let b = dot(n, d);
    if b.abs() <= 1e-12 {
        return Err(operation_error("symmetric frame"));
    }
    let tangent: [f64; 3] = std::array::from_fn(|i| d[i] - b * n[i]);
    let a = norm(tangent);
    let x = if a > 1e-15 {
        tangent.map(|v| v / a)
    } else {
        frame.u
    };
    let y = cross(n, x);
    let k = a / b;
    let s = 1_f64.hypot(k) + k.abs();
    let theta = if k == 0. {
        0.
    } else {
        0.5 * (-4. * k).atan2(-4. * k * k)
    };
    let (sin, cos) = theta.sin_cos();
    let vx: [f64; 3] = std::array::from_fn(|i| cos * x[i] + sin * n[i]);
    let ux: [f64; 3] = std::array::from_fn(|i| (vx[i] - 2. * d[i] * dot(n, vx) / b) / s);
    let p: [f64; 3] = std::array::from_fn(|i| frame.origin[i] + d[i] * start);
    // Reject unrepresentable/ill-conditioned placements before the native
    // constructor (which otherwise ignores its canonicalization error).
    if p.iter()
        .chain(vx.iter())
        .chain(ux.iter())
        .chain(y.iter())
        .any(|v| !v.is_finite())
        || (norm(vx) - 1.).abs() > 1e-10
        || (norm(ux) - 1.).abs() > 1e-10
        || (norm(y) - 1.).abs() > 1e-10
        || dot(vx, y).abs() > 1e-10
        || dot(ux, y).abs() > 1e-10
        || !s.is_finite()
    {
        return Err(operation_error("unrepresentable symmetric frame"));
    }
    let vec = |v: [f64; 3]| Vec3::new(v[0], v[1], v[2]);
    let origin = Point3::new(p[0], p[1], p[2]);
    let first = Axis2Placement::new(origin, vec(vx), vec(y)).invert();
    let second = Axis2Placement::new(origin, vec(ux), vec(y));
    Ok((first, second, Vec3::new(s, 1., -1. / s)))
}

#[cfg(test)]
fn native_material_at(
    kernel: &Kernel,
    body: &solidcore::Brep,
    point: [f64; 3],
    tolerance: f64,
) -> Result<bool, crate::NewellKernelError> {
    let mut scratch = body
        .clone_standing_for_self()
        .map_err(crate::operation_error)?;
    let query = solidcore::query::brep::BrepQuery::of_brep(&scratch);
    let result = query
        .point_3d_classify(
            kernel,
            &mut scratch,
            solidcore::math::Point3::new(point[0], point[1], point[2]),
            tolerance,
            false,
        )
        .map_err(crate::operation_error)?;
    let Some(solidcore::error::EntityRef::Region(region)) = result.object() else {
        return Err(crate::operation_error(
            "point did not classify uniquely to a region",
        ));
    };
    Ok(scratch
        .region(region)
        .map_err(crate::operation_error)?
        .is_void
        == 0)
}

/// M4's coordinate-based policy is independent of display deflection. Native
/// context still reports its size-derived default separately; selected lower
/// routes receive explicit owner/entity tolerances and their outputs are checked.
fn configure_native_policy(
    kernel: &mut Kernel,
    span: f64,
    scale: f64,
) -> Result<f64, crate::NewellKernelError> {
    if !span.is_finite() || span <= 0. || !scale.is_finite() || scale < span {
        return Err(crate::operation_error("invalid modeling extent"));
    }
    let eps = 1e-7_f64.max(1e-10 * scale);
    kernel.tolerances = Tolerances {
        zone_tol_3d: eps / 2.,
        approx_tol_3d: eps / 2.,
        xsect_tol_3d: eps,
        ang_tol_deg: 1e-12 * 180. / std::f64::consts::PI,
        tol_gap_gain: 1.,
        model_size_estimate: span,
        large_small_size_ratio: 1000.,
    };
    kernel.context.set_this_model_size_estimate(span);
    kernel.context.set_this_large_small_size_ratio(1000.);
    Ok(eps)
}

fn sweep_native_policy(
    kernel: &mut Kernel,
    source: &solidcore::Brep,
    selection: &solidcore::geom::extent3d::Extent3d,
    delta: [f64; 3],
) -> Result<(f64, f64, f64), crate::NewellKernelError> {
    let source_bounds = solidcore::query::brep::calculate_bounding_box(
        &*kernel,
        source,
        solidcore::query::brep::BoundingBoxOptions {
            approx: false,
            tight: true,
            expand_by_zone_tol_3d: false,
        },
    )
    .map_err(crate::operation_error)?;
    let span = (0..3)
        .map(|i| selection.max_at(i) - selection.min_at(i) + delta[i].abs())
        .fold(0_f64, f64::max);
    let mut scale = span;
    for (i, &d) in delta.iter().enumerate() {
        for x in [
            source_bounds.min_at(i),
            source_bounds.max_at(i),
            selection.min_at(i) + d,
            selection.max_at(i) + d,
        ] {
            scale = scale.max(x.abs());
        }
    }
    let eps = configure_native_policy(kernel, span, scale)?;
    Ok((span, eps, native_construction_zone(eps, scale)?))
}

#[cfg(test)]
fn uncapped_sweep_with_lineage(
    kernel: &mut Kernel,
    source: &solidcore::Brep,
    occurrences: &std::collections::BTreeMap<(u32, u32), solidcore::EdgeId>,
    delta: [f64; 3],
) -> Result<
    (
        solidcore::Brep,
        std::collections::BTreeMap<crate::NewellFaceProvenance, solidcore::FaceId>,
    ),
    crate::NewellKernelError,
> {
    uncapped_sweep_between_with_lineage(kernel, source, occurrences, [0.; 3], delta)
}

#[cfg(test)]
fn uncapped_sweep_between_with_lineage(
    kernel: &mut Kernel,
    source: &solidcore::Brep,
    occurrences: &std::collections::BTreeMap<(u32, u32), solidcore::EdgeId>,
    offset: [f64; 3],
    delta: [f64; 3],
) -> Result<
    (
        solidcore::Brep,
        std::collections::BTreeMap<crate::NewellFaceProvenance, solidcore::FaceId>,
    ),
    crate::NewellKernelError,
> {
    use crate::{NewellFaceProvenance as P, operation_error};
    use solidcore::primitive::primitive_creation::model::PrimitiveModel;
    use solidcore::sweep::{
        topology_sweep::{
            SweepEntity, TopologySweep, access::SweepAccess, engine::SweepTargets,
            geometry::SweepGeometry,
        },
        trans_sweep_geometry::TranslationalSweepGeometry,
    };
    if occurrences.is_empty()
        || offset.iter().chain(delta.iter()).any(|v| !v.is_finite())
        || delta.iter().map(|v| v * v).sum::<f64>().sqrt() <= 2. * source.tolerance()
    {
        return Err(operation_error("invalid sheet sweep input"));
    }
    let mut bounds = solidcore::geom::extent3d::Extent3d::new();
    for &edge in occurrences.values() {
        let edge_bounds = solidcore::query::edge::calculate_bounding_box(
            kernel,
            source,
            edge,
            solidcore::nurbs::b_spline_curve::BoundingBoxRequest {
                normal: true,
                pseudo: false,
                polar: false,
            },
            true,
        )
        .map_err(operation_error)?
        .normal
        .ok_or_else(|| operation_error("missing exact edge bounds"))?;
        bounds = bounds.union(&edge_bounds);
    }
    let shifted = solidcore::geom::extent3d::Extent3d::from_bounds(
        bounds.min_at(0) + offset[0],
        bounds.min_at(1) + offset[1],
        bounds.min_at(2) + offset[2],
        bounds.max_at(0) + offset[0],
        bounds.max_at(1) + offset[1],
        bounds.max_at(2) + offset[2],
    );
    let (span, eps, zone) = sweep_native_policy(kernel, source, &shifted, delta)?;
    if source.tolerance() > eps / 2. {
        return Err(operation_error("source precision exceeds sheet policy"));
    }
    let mut output =
        solidcore::topology::brep::p3::new_brep(kernel, span, 1000.).map_err(operation_error)?;
    output.set_tolerance(zone);
    let region = output
        .infinite_region()
        .ok_or_else(|| operation_error("missing sheet region"))?;
    let mut scratch = source.clone_standing_for_self().map_err(operation_error)?;
    if offset.iter().any(|&v| v != 0.) {
        let placement = solidcore::geom::axis2_placement::Axis2Placement::new(
            solidcore::math::Point3::new(offset[0], offset[1], offset[2]),
            solidcore::math::Vec3::X,
            solidcore::math::Vec3::Y,
        );
        solidcore::topology::brep::p5::transform(kernel, &mut scratch, &placement, None, false)
            .map_err(operation_error)?;
        assert_native_tolerances(&scratch, eps / 2.)?;
    }
    let mut model = PrimitiveModel::new(kernel, output);
    let input = model.add_brep(scratch).map_err(operation_error)?;
    let main = PrimitiveModel::MAIN;
    let mut targets = SweepTargets {
        edges_higher: Some(occurrences.values().map(|&e| Some((input, e))).collect()),
        ..Default::default()
    };
    let mut geometry =
        TranslationalSweepGeometry::new(solidcore::math::Vec3::new(delta[0], delta[1], delta[2]));
    geometry.base.eps = zone;
    let mut sweep = TopologySweep {
        do_stitching: false,
        ..Default::default()
    };
    sweep
        .do_sweep(
            &mut model,
            &SweepGeometry::Translation(geometry),
            Some(input),
            Some((main, region)),
            &mut targets,
        )
        .map_err(operation_error)?;
    model
        .stitch(main, &mut sweep, zone, false)
        .map_err(operation_error)?;
    model.orient(main, false, true).map_err(operation_error)?;
    let mut roles = std::collections::BTreeMap::new();
    for (&(wire, edge), &source) in occurrences {
        let original = SweepEntity::Edge(input, source);
        let mapped = sweep
            .get_to_higher(Some(original))
            .ok_or_else(|| operation_error("missing sheet wall lineage"))?;
        let SweepEntity::Face(owner, face) = mapped else {
            return Err(operation_error("invalid sheet wall lineage dimension"));
        };
        if owner != main || sweep.get_from_lower(Some(mapped)) != Some(original) {
            return Err(operation_error("nonreciprocal sheet wall lineage"));
        }
        roles.insert(P::Wall { wire, edge }, face);
    }
    let mut body = model.into_brep().map_err(operation_error)?;
    body.refresh_cached_topology();
    let unique: std::collections::BTreeSet<_> = roles.values().copied().collect();
    if unique.len() != roles.len() || body.face_list().len() != roles.len() {
        return Err(operation_error("ambiguous sheet face construction"));
    }
    assert_native_tolerances(&body, eps / 2.)?;
    if solidcore::query::api_queries::brep_material_census(&body)
        .map_err(operation_error)?
        .0
        != 0
    {
        return Err(operation_error(
            "uncapped sweep unexpectedly produced material",
        ));
    }
    Ok((body, roles))
}

#[test]
fn solidcore_migration_scale_translation_policy_and_repeatability()
-> Result<(), Box<dyn std::error::Error>> {
    for scale in [0.1, 1., 10.] {
        for origin in [[0.; 3], [1e6, -1e6, 1e6]] {
            let mut previous = None;
            for _ in 0..10 {
                let mut owner = MigrationGeometry::new(10. * scale);
                let source = owner.box_shape([10. * scale, 6. * scale, 5. * scale], origin)?;
                let key = owner.sweep_face_delta(source, 1, [0., 0., 3. * scale])?;
                let expected_span = 10. * scale;
                assert!((owner.kernel.tolerances.model_size_estimate - expected_span).abs() < 1e-7);
                assert_eq!(owner.kernel.tolerances.large_small_size_ratio, 1000.);
                assert_eq!(owner.kernel.tolerances.tol_gap_gain, 1.);
                let eps = 1e-7_f64.max(
                    1e-10
                        * (origin[0] + 10. * scale)
                            .abs()
                            .max(origin[1].abs())
                            .max((origin[2] + 8. * scale).abs())
                            .max(expected_span),
                );
                let effective = owner.kernel.tolerances.zone_tol_3d;
                assert!((effective - eps / 2.).abs() < 1e-14);
                let body = owner.objects.get(&key).ok_or("missing scaled owner")?;
                assert_native_tolerances(body, eps / 2.)?;
                assert!(native_material_at(
                    &owner.kernel,
                    body,
                    [
                        origin[0] + 5. * scale,
                        origin[1] + 3. * scale,
                        origin[2] + 6.5 * scale
                    ],
                    eps / 2.
                )?);
                let descriptors = owner.profile_faces(key)?;
                let counts = descriptors
                    .iter()
                    .map(|f| {
                        (
                            f.provenance,
                            f.wires
                                .iter()
                                .map(|w| w.edges.iter().map(|e| e.edge).collect::<Vec<_>>())
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect::<Vec<_>>();
                if let Some(ref previous) = previous {
                    assert_eq!(&counts, previous);
                }
                previous = Some(counts);
                let mut native = body.clone_standing_for_self()?;
                let store = solidcore::geom::attribute::AttributeStore::default();
                let key_of =
                    |_: solidcore::error::EntityRef| solidcore::geom::attribute::ObjectKey(1);
                let attrs = solidcore::mass::integrator::properties::PropertyAttributes {
                    store: &store,
                    key_of: &key_of,
                };
                let volume = solidcore::mass::integrator::properties::brep_volume(
                    &owner.kernel,
                    &mut native,
                    PROPERTY_RELATIVE_ACCURACY,
                    &attrs,
                )?;
                let expected = 180. * scale * scale * scale;
                let bound = (1e-6 * scale * scale * scale).max(eps * 216. * scale * scale);
                assert!(
                    (volume - expected).abs() < bound,
                    "scale={scale},origin={origin:?},volume={volume},bound={bound}"
                );
            }
        }
    }
    Ok(())
}

fn native_boolean_copies(
    kernel: &mut Kernel,
    target: &solidcore::Brep,
    tool: &solidcore::Brep,
    mode: crate::NewellBooleanMode,
) -> Result<solidcore::Brep, crate::NewellKernelError> {
    let operation = match mode {
        crate::NewellBooleanMode::Add => solidcore::boolean::api_brep::boolean_union,
        crate::NewellBooleanMode::Remove => solidcore::boolean::api_brep::boolean_difference,
    };
    native_operation_copies(kernel, target, tool, operation)
}

fn native_operation_copies(
    kernel: &mut Kernel,
    target: &solidcore::Brep,
    tool: &solidcore::Brep,
    operation: fn(
        &mut Kernel,
        solidcore::Brep,
        solidcore::Brep,
    ) -> Result<solidcore::Brep, solidcore::SolidError>,
) -> Result<solidcore::Brep, crate::NewellKernelError> {
    let options = solidcore::query::brep::BoundingBoxOptions {
        approx: false,
        tight: true,
        expand_by_zone_tol_3d: false,
    };
    let target_bounds = solidcore::query::brep::calculate_bounding_box(&*kernel, target, options)
        .map_err(crate::operation_error)?;
    let tool_bounds = solidcore::query::brep::calculate_bounding_box(&*kernel, tool, options)
        .map_err(crate::operation_error)?;
    let bounds = target_bounds.union(&tool_bounds);
    let span = bounds.max_length();
    let scale = (0..3)
        .flat_map(|i| [bounds.min_at(i).abs(), bounds.max_at(i).abs()])
        .fold(span, f64::max);
    let eps = configure_native_policy(kernel, span, scale)?;
    assert_native_tolerances(target, eps / 2.)?;
    assert_native_tolerances(tool, eps / 2.)?;
    let target = independent_native_copy(kernel, target)?;
    let tool = independent_native_copy(kernel, tool)?;
    let result = operation(kernel, target, tool).map_err(crate::operation_error)?;
    if !result.face_list().is_empty()
        && solidcore::query::api_queries::brep_material_census(&result)
            .map_err(crate::operation_error)?
            .0
            == 0
    {
        validate_native_sheet_links(&result)?;
    }
    assert_native_tolerances(&result, eps / 2.)?;
    Ok(result)
}

#[test]
fn solidcore_migration_f06_planar_sheet_union_difference_and_split()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::NewellBooleanMode::{Add, Remove};
    let mut owner = MigrationGeometry::new(10.);
    let mut sheets = Vec::new();
    for (z, depth) in [(0., 3.), (1., 3.), (0., 4.), (1., 2.)] {
        let key = owner.box_shape([2., 3., 1.], [0., 0., z])?;
        let source = owner.objects.get(&key).ok_or("missing F06 source")?;
        let occurrences = owner
            .occurrences
            .get(&key)
            .ok_or("missing F06 source edges")?
            .iter()
            .filter_map(|(&(r, w, e), &native)| {
                (r == crate::NewellFaceProvenance::Base(1)).then_some(((w, e), native))
            })
            .collect();
        sheets.push(
            uncapped_sweep_with_lineage(&mut owner.kernel, source, &occurrences, [0., 0., depth])?
                .0,
        );
    }
    for (name, target, tool, mode, expected, components) in [
        ("union", 0, 1, Add, 40., 1),
        ("cut", 0, 1, Remove, 10., 1),
        ("split", 2, 3, Remove, 20., 2),
        ("empty", 2, 2, Remove, 0., 0),
    ] {
        let mut result =
            planar_surface_boolean_copies(&mut owner.kernel, &sheets[target], &sheets[tool], mode)?;
        let area = solidcore::mass::integrator::properties::brep_area(
            &owner.kernel,
            &mut result,
            PROPERTY_RELATIVE_ACCURACY,
        )?;
        assert!((area - expected).abs() < 1e-6, "F06 {name}: area={area}");
        assert_eq!(
            sheet_component_count(&result)?,
            components,
            "F06 {name} components"
        );
        let parts = native_sheet_components(&mut owner.kernel, &mut result)?;
        assert_eq!(parts.len(), components);
        let mut parted_area = 0.;
        for mut part in parts {
            parted_area += solidcore::mass::integrator::properties::brep_area(
                &owner.kernel,
                &mut part,
                PROPERTY_RELATIVE_ACCURACY,
            )?;
        }
        assert!((parted_area - expected).abs() < 1e-6);
        assert_eq!(
            solidcore::query::api_queries::brep_material_census(&result)?.0,
            0
        );
        assert_eq!(free_boundary_cycles(&result)?, 2 * components);
        if components > 0 {
            assert_native_tolerances(&result, 5e-8)?;
            let mesh = crate::solidcore_mesh::native_body(
                &owner.kernel,
                &result,
                result.face_list(),
                0.01,
            )?;
            assert!(!mesh.indices().is_empty());
        } else {
            assert!(result.face_list().is_empty());
        }
        println!("F06_NATIVE,case={name},area={area},components={components}");
    }
    Ok(())
}

/// Count actual free boundary cycles by topology, independent of face partition
/// and native loop heads. A branch, open chain or nonmanifold edge is an error.
#[cfg(test)]
fn free_boundary_cycles(body: &solidcore::Brep) -> Result<usize, crate::NewellKernelError> {
    use crate::operation_error;
    let mut adjacency =
        std::collections::BTreeMap::<solidcore::VertexId, Vec<solidcore::VertexId>>::new();
    for &id in body.edge_list() {
        let faces = solidcore::topology::edge::faces(body, id, None).map_err(operation_error)?;
        if faces.is_empty() || faces.len() > 2 {
            return Err(operation_error("nonmanifold sheet edge"));
        }
        if faces.len() == 1 {
            // A periodic seam has two occurrences of the same face; it is not a
            // free rim. This is topological incidence, not proximity of samples.
            if solidcore::topology::edge::number_face_occurances(body, id, Some(faces[0]))
                .map_err(operation_error)?
                > 1
            {
                continue;
            }
            let edge = body.edge(id).map_err(operation_error)?;
            let [a, b] = edge.vertices;
            adjacency.entry(a).or_default().push(b);
            adjacency.entry(b).or_default().push(a);
        }
    }
    if adjacency.values().any(|neighbors| neighbors.len() != 2) {
        return Err(operation_error("free boundary is branched or open"));
    }
    let mut remaining: std::collections::BTreeSet<_> = adjacency.keys().copied().collect();
    let mut cycles = 0;
    while let Some(start) = remaining.pop_first() {
        cycles += 1;
        let mut pending = vec![start];
        while let Some(vertex) = pending.pop() {
            for &next in adjacency
                .get(&vertex)
                .ok_or_else(|| operation_error("missing boundary vertex"))?
            {
                if remaining.remove(&next) {
                    pending.push(next);
                }
            }
        }
    }
    Ok(cycles)
}

#[cfg(test)]
fn planar_patch(
    kernel: &mut Kernel,
    x: f64,
    z: f64,
    width: f64,
    height: f64,
) -> Result<solidcore::Brep, crate::NewellKernelError> {
    use solidcore::{
        curve::{curve::Curve, line::Line},
        math::Point3,
    };
    let corners = [
        [x, 0., z],
        [x + width, 0., z],
        [x + width, 0., z + height],
        [x, 0., z + height],
    ];
    let mut curves = Vec::new();
    for i in 0..4 {
        let a = corners[i];
        let b = corners[(i + 1) % 4];
        curves.push(Curve::from(
            Line::create_line_segment(
                3,
                Point3::new(a[0], a[1], a[2]),
                Point3::new(b[0], b[1], b[2]),
                None,
            )
            .map_err(crate::operation_error)?,
        ));
    }
    exact_planar_profile(kernel, &curves).map(|(body, _)| body)
}

#[test]
fn solidcore_migration_f06_sheet_hole_partial_fill_and_transverse_rejection()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::NewellBooleanMode::{Add, Remove};
    let mut kernel = mapped_kernel(10.);
    let target = planar_patch(&mut kernel, 0., 0., 10., 10.)?;
    let cutter = planar_patch(&mut kernel, 4., 4., 2., 2.)?;
    let mut holed = planar_surface_boolean_copies(&mut kernel, &target, &cutter, Remove)?;
    assert!(
        (solidcore::mass::integrator::properties::brep_area(
            &kernel,
            &mut holed,
            PROPERTY_RELATIVE_ACCURACY
        )? - 96.)
            .abs()
            < 1e-6
    );
    assert_eq!(free_boundary_cycles(&holed)?, 2);
    let fill = planar_patch(&mut kernel, 5., 4., 2., 2.)?;
    let mut partial = planar_surface_boolean_copies(&mut kernel, &holed, &fill, Add)?;
    assert!(
        (solidcore::mass::integrator::properties::brep_area(
            &kernel,
            &mut partial,
            PROPERTY_RELATIVE_ACCURACY
        )? - 98.)
            .abs()
            < 1e-6
    );
    assert_eq!(free_boundary_cycles(&partial)?, 2);
    assert_native_tolerances(&partial, 5e-8)?;
    for deflection in [0.05, 0.01] {
        let mesh =
            crate::solidcore_mesh::native_body(&kernel, &partial, partial.face_list(), deflection)?;
        assert!(!mesh.indices().is_empty());
    }
    let target = planar_patch(&mut kernel, 0., 0., 1., 1.)?;
    let corners = [
        [0.5, -0.5, 0.],
        [0.5, 0.5, 0.],
        [0.5, 0.5, 1.],
        [0.5, -0.5, 1.],
    ];
    let mut curves = Vec::new();
    for i in 0..4 {
        let a = corners[i];
        let b = corners[(i + 1) % 4];
        curves.push(solidcore::curve::curve::Curve::from(
            solidcore::curve::line::Line::create_line_segment(
                3,
                solidcore::math::Point3::new(a[0], a[1], a[2]),
                solidcore::math::Point3::new(b[0], b[1], b[2]),
                None,
            )?,
        ));
    }
    let (transverse, _) = exact_planar_profile(&mut kernel, &curves)?;
    for mode in [Add, Remove] {
        assert!(planar_surface_boolean_copies(&mut kernel, &target, &transverse, mode).is_err());
    }
    let mut target = target.clone_standing_for_self()?;
    assert!(
        (solidcore::mass::integrator::properties::brep_area(
            &kernel,
            &mut target,
            PROPERTY_RELATIVE_ACCURACY
        )? - 1.)
            .abs()
            < 1e-6
    );
    println!("F06_NATIVE,hole_area=96,partial_fill_area=98,boundary_cycles=2");
    Ok(())
}

fn planar_surface_boolean_copies(
    kernel: &mut Kernel,
    target: &solidcore::Brep,
    tool: &solidcore::Brep,
    mode: crate::NewellBooleanMode,
) -> Result<solidcore::Brep, crate::NewellKernelError> {
    use crate::operation_error;
    if target.face_list().is_empty() || tool.face_list().is_empty() {
        return Err(operation_error("empty surface operand"));
    }
    let supports = target
        .face_list()
        .iter()
        .map(|&face| native_planar_frame(kernel, target, face))
        .collect::<Result<Vec<_>, _>>()?;
    for &face in tool.face_list() {
        let frame = native_planar_frame(kernel, tool, face)?;
        let same_support = supports.iter().any(|support| {
            let a = support.normal;
            let b = frame.normal;
            let cross = [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ];
            let distance = (0..3)
                .map(|i| (support.origin[i] - frame.origin[i]) * a[i])
                .sum::<f64>()
                .abs();
            cross.iter().map(|x| x * x).sum::<f64>().sqrt() <= 1e-12
                && distance <= kernel.tolerances.xsect_tol_3d
        });
        if !same_support {
            return Err(operation_error(
                "transverse surface area operation is unsupported",
            ));
        }
    }
    native_boolean_copies(kernel, target, tool, mode)
}

#[cfg(test)]
fn exact_cubic_sheet(
    kernel: &mut Kernel,
    control: [[f64; 2]; 4],
    start: f64,
    end: f64,
) -> Result<solidcore::Brep, crate::NewellKernelError> {
    exact_bspline_sheet(
        kernel,
        &control,
        3,
        &[0., 0., 0., 0., 1., 1., 1., 1.],
        [start, end],
    )
}

#[cfg(test)]
fn exact_bspline_sheet(
    kernel: &mut Kernel,
    control: &[[f64; 2]],
    degree: usize,
    knots: &[f64],
    stations: [f64; 2],
) -> Result<solidcore::Brep, crate::NewellKernelError> {
    use solidcore::{
        curve::{curve::Curve, line::Line},
        math::Point3,
        nurbs::{b_spline_curve::BSplineCurve, crv_basic::Curve as NurbsCurve, math::CPoint},
    };
    let first = control
        .first()
        .ok_or_else(|| crate::operation_error("empty spline controls"))?;
    let last = control
        .last()
        .ok_or_else(|| crate::operation_error("empty spline controls"))?;
    let controls = control
        .iter()
        .map(|p| CPoint::from_point(Point3::new(p[0], p[1], stations[0])))
        .collect();
    let curve = Curve::from(BSplineCurve::from_nurb(
        NurbsCurve::new(controls, degree, knots.to_vec()).map_err(crate::operation_error)?,
        3,
        false,
    ));
    let chord = Curve::from(
        Line::create_line_segment(
            3,
            Point3::new(last[0], last[1], stations[0]),
            Point3::new(first[0], first[1], stations[0]),
            None,
        )
        .map_err(crate::operation_error)?,
    );
    let (profile, face) = exact_planar_profile(kernel, &[curve, chord])?;
    let candidates = initial_boundary_occurrences(&profile, face)?;
    let mut selected = std::collections::BTreeMap::new();
    for (key, id) in candidates {
        let curve = profile
            .edge(id)
            .map_err(crate::operation_error)?
            .curve
            .as_ref()
            .ok_or_else(|| crate::operation_error("missing source curve"))?;
        if matches!(curve.kind(),solidcore::curve::curve::CurveKind::BSpline(c) if c.gw_nurb().is_some_and(|n|n.degree()==degree))
        {
            selected.insert(key, id);
        }
    }
    if selected.len() != 1 {
        return Err(crate::operation_error("missing unique constructed spline"));
    }
    uncapped_sweep_with_lineage(
        kernel,
        &profile,
        &selected,
        [0., 0., stations[1] - stations[0]],
    )
    .map(|(body, _)| body)
}

#[test]
fn solidcore_migration_f04_canonical_cubic_overlap_keeps_exact_uncapped_support()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::NewellBooleanMode::{Add, Remove};
    let mut kernel = mapped_kernel(6.);
    let control = [[0., 0.], [2., 3.], [4., 2.], [6., 0.]];
    let mut a = exact_cubic_sheet(&mut kernel, control, 0., 3.)?;
    let b = exact_cubic_sheet(&mut kernel, control, 1., 4.)?;
    // Same independent polynomial-speed Simpson oracle and truncation bound as
    // the retained fixture; neither source curves nor topology are precomputed.
    let n = 16384;
    let speed = |t: f64| (36. + (9. - 24. * t + 9. * t * t).powi(2)).sqrt();
    let mut sum = speed(0.) + speed(1.);
    for i in 1..n {
        sum += if i % 2 == 0 { 2. } else { 4. } * speed(f64::from(i) / f64::from(n));
    }
    let length = sum / (3. * f64::from(n));
    assert!(250_000. / (180. * f64::from(n).powi(4)) + 1e-10 < 1e-9);
    let area = solidcore::mass::integrator::properties::brep_area(
        &kernel,
        &mut a,
        PROPERTY_RELATIVE_ACCURACY,
    )?;
    assert!((area - 3. * length).abs() < 1e-6);
    assert_eq!(free_boundary_cycles(&a)?, 1);
    for (mode, expected) in [(Add, 4. * length), (Remove, length)] {
        let mut result = native_boolean_copies(&mut kernel, &a, &b, mode)?;
        let area = solidcore::mass::integrator::properties::brep_area(
            &kernel,
            &mut result,
            PROPERTY_RELATIVE_ACCURACY,
        )?;
        assert!(
            (area - expected).abs() < 1e-6,
            "F04 {mode:?}: area={area}, expected={expected}"
        );
        assert_eq!(sheet_component_count(&result)?, 1);
        assert_eq!(free_boundary_cycles(&result)?, 1);
        assert_native_tolerances(&result, 5e-8)?;
        for deflection in [0.05, 0.01] {
            let mesh = crate::solidcore_mesh::native_body(
                &kernel,
                &result,
                result.face_list(),
                deflection,
            )?;
            assert!(!mesh.indices().is_empty());
        }
        println!("F04_CUBIC_NATIVE,mode={mode:?},area={area},expected={expected}");
    }
    Ok(())
}

#[test]
fn solidcore_migration_f07_degree_elevated_refined_affine_source_overlap()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::NewellBooleanMode::{Add, Remove};
    let mut kernel = mapped_kernel(6.);
    let source = [[0., 0.], [2., 3.], [4., 2.], [6., 0.]];
    let a = exact_cubic_sheet(&mut kernel, source, 0., 3.)?;
    let elevated = [
        source[0],
        std::array::from_fn(|i| 0.25 * source[0][i] + 0.75 * source[1][i]),
        std::array::from_fn(|i| 0.5 * source[1][i] + 0.5 * source[2][i]),
        std::array::from_fn(|i| 0.75 * source[2][i] + 0.25 * source[3][i]),
        source[3],
    ];
    // Exact one-knot insertion of this degree-four Bezier at its midpoint,
    // followed by the retained affine [0,1] -> [2,5] parameter mapping.
    let mut refined = vec![elevated[0]];
    for pair in elevated.windows(2) {
        refined.push(std::array::from_fn(|i| 0.5 * (pair[0][i] + pair[1][i])));
    }
    refined.push(elevated[4]);
    let b = exact_bspline_sheet(
        &mut kernel,
        &refined,
        4,
        &[2., 2., 2., 2., 2., 3.5, 5., 5., 5., 5., 5.],
        [1., 4.],
    )?;
    let n = 16384;
    let speed = |t: f64| (36. + (9. - 24. * t + 9. * t * t).powi(2)).sqrt();
    let mut sum = speed(0.) + speed(1.);
    for i in 1..n {
        sum += if i % 2 == 0 { 2. } else { 4. } * speed(f64::from(i) / f64::from(n));
    }
    let length = sum / (3. * f64::from(n));
    for (mode, expected) in [(Add, 4. * length), (Remove, length)] {
        let mut result = native_boolean_copies(&mut kernel, &a, &b, mode)?;
        let area = solidcore::mass::integrator::properties::brep_area(
            &kernel,
            &mut result,
            PROPERTY_RELATIVE_ACCURACY,
        )?;
        assert!(
            (area - expected).abs() < 1e-6,
            "F07 {mode:?}: area={area}, expected={expected}"
        );
        assert_eq!(sheet_component_count(&result)?, 1);
        assert_eq!(free_boundary_cycles(&result)?, 1);
        assert_native_tolerances(&result, 5e-8)?;
        let mesh = crate::solidcore_mesh::native_body(&kernel, &result, result.face_list(), 0.02)?;
        assert!(!mesh.indices().is_empty());
        println!("F07_NATIVE,mode={mode:?},area={area},expected={expected}");
    }
    Ok(())
}

impl MigrationGeometry {
    fn extrude_face(
        &mut self,
        key: crate::NewellShapeKey,
        slot: u32,
        along: Option<[f64; 3]>,
        start: f64,
        end: f64,
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        use crate::operation_error;
        self.check_cancelled()?;
        if !start.is_finite()
            || !end.is_finite()
            || !(end - start).is_finite()
            || end - start <= 1e-7
        {
            return Err(operation_error("invalid extrusion interval"));
        }
        let (face, occurrences) = self.resolve_source_face(key, slot)?;
        let body = self
            .objects
            .get(&key)
            .ok_or_else(|| operation_error("missing source owner"))?;
        let frame = native_planar_frame(&self.kernel, body, face)?;
        let direction = along.unwrap_or(frame.normal);
        if direction.iter().any(|v| !v.is_finite()) {
            return Err(operation_error("invalid Along"));
        }
        let length = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
        if !length.is_finite() || length <= 0. {
            return Err(operation_error("degenerate Along"));
        }
        let direction = direction.map(|v| v / length);
        if direction
            .iter()
            .zip(frame.normal)
            .map(|(a, b)| a * b)
            .sum::<f64>()
            .abs()
            <= 1e-12
        {
            return Err(operation_error("tangential Along"));
        }
        let result = capped_sweep_between_with_lineage(
            &mut self.kernel,
            body,
            face,
            &occurrences,
            direction.map(|v| v * start),
            direction.map(|v| v * (end - start)),
        )?;
        self.check_cancelled()?;
        let output = self.insert(result.body)?;
        self.roles.insert(output, result.roles);
        self.occurrences.insert(output, result.occurrence_map);
        self.extrusion_sources.insert(output, (key, slot));
        Ok(output)
    }
}

#[test]
fn solidcore_two_sided_union_loses_extrusion_lineage() -> Result<(), Box<dyn std::error::Error>> {
    let mut owner = MigrationGeometry::new(10.);
    let source = owner.box_shape([1., 1., 1.], [0.; 3])?;
    let profile = (source, 1);
    let direction = owner
        .profile_faces(source)?
        .into_iter()
        .find(|face| face.face == profile.1)
        .ok_or("profile face")?
        .frame
        .normal;
    let first = owner.extrude_face(source, profile.1, Some(direction), 0., 3.)?;
    let second = owner.mirror_sweep(first, profile, direction, 0.)?;
    for (side, key) in [(0_i64, first), (1, second)] {
        let roles = owner.roles.get(&key).ok_or("missing sweep roles")?.clone();
        let body = owner.objects.get_mut(&key).ok_or("missing sweep body")?;
        for (ordinal, face) in roles.values().enumerate() {
            body.add_attribute(
                solidcore::error::EntityRef::Face(*face),
                solidcore::geom::attribute::Attribute::long(
                    10001,
                    side * 100 + i64::try_from(ordinal)?,
                    solidcore::geom::attribute::Behavior::Copy,
                ),
            )?;
        }
        let occurrences = owner
            .occurrences
            .get(&key)
            .ok_or("missing sweep edges")?
            .clone();
        for (ordinal, edge) in occurrences.values().enumerate() {
            body.add_attribute(
                solidcore::error::EntityRef::Edge(*edge),
                solidcore::geom::attribute::Attribute::long(
                    10002,
                    side * 1000 + i64::try_from(ordinal)?,
                    solidcore::geom::attribute::Behavior::Copy,
                ),
            )?;
        }
    }
    let joined = owner
        .boolean(first, second, crate::NewellBooleanMode::Add)?
        .ok_or("joined body")?;
    let body = owner.objects.get(&joined).ok_or("missing joined body")?;
    let tags: Vec<_> = body
        .face_list()
        .iter()
        .map(|face| {
            body.entity_attributes(solidcore::error::EntityRef::Face(*face))
                .into_iter()
                .filter_map(|(_, attr)| match attr.value() {
                    solidcore::geom::attribute::AttributeValue::Long(value) => Some(*value),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(tags.len(), 6);
    assert!(tags.iter().all(|face| face.len() == 1));
    let surviving: std::collections::BTreeSet<_> = tags.into_iter().flatten().collect();
    assert_eq!(surviving, [1, 2, 5, 101, 103, 104].into());
    let edge_tags: Vec<_> = body
        .edge_list()
        .iter()
        .map(|edge| {
            body.entity_attributes(solidcore::error::EntityRef::Edge(*edge))
                .into_iter()
                .filter_map(|(_, attr)| match attr.value() {
                    solidcore::geom::attribute::AttributeValue::Long(value)
                        if attr.attribute_id() == 10002 =>
                    {
                        Some(*value)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(edge_tags.len(), 12);
    assert!(edge_tags.iter().all(|edge| edge.len() == 1));
    assert_eq!(
        edge_tags
            .iter()
            .flatten()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        12
    );
    assert!(owner.tag_extrude(joined, profile, direction).is_err());
    Ok(())
}

#[test]
fn solidcore_migration_displaced_symmetric_oblique_extents_keep_source()
-> Result<(), Box<dyn std::error::Error>> {
    let mut owner = MigrationGeometry::new(10.);
    let source = owner.box_shape([10., 6., 5.], [0.; 3])?;
    let original = owner.tessellate(source, 0.01)?;
    for (along, start, end, expected, z_min, z_max) in [
        (None, 2., 5., 180., 7., 10.),
        (None, -2., 2., 240., 3., 7.),
        (Some([0.6, 0., 0.8]), 2., 5., 144., 6.6, 9.),
    ] {
        let key = owner.extrude_face(source, 1, along, start, end)?;
        let mesh = owner.tessellate(key, 0.01)?;
        assert!((oriented_mesh_volume(&mesh)? - expected).abs() < 1e-3);
        let low = mesh
            .positions()
            .iter()
            .map(|p| f64::from(p[2]))
            .fold(f64::INFINITY, f64::min);
        let high = mesh
            .positions()
            .iter()
            .map(|p| f64::from(p[2]))
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((low - z_min).abs() < 1e-6);
        assert!((high - z_max).abs() < 1e-6);
        assert_eq!(owner.profile_faces(key)?.len(), 6);
        assert_eq!(owner.tessellate(source, 0.01)?, original);
    }
    let next = owner.next;
    let count = owner.objects.len();
    for (along, start, end) in [
        (Some([1., 0., 0.]), 0., 3.),
        (Some([f64::NAN, 0., 1.]), 0., 3.),
        (None, 3., 3.),
    ] {
        assert!(owner.extrude_face(source, 1, along, start, end).is_err());
        assert_eq!(owner.next, next);
        assert_eq!(owner.objects.len(), count);
    }
    Ok(())
}

/// Initialize below the permitted half-epsilon zone, leaving a small arithmetic
/// reserve for native curve/vertex reconciliation after affine transformations.
/// The final limit stays eps/2 and kernel options stay eps/2; no completed
/// geometry is relabeled and no modeling tolerance is enlarged.
fn native_construction_zone(eps: f64, scale: f64) -> Result<f64, crate::NewellKernelError> {
    let zone = eps / 2. - 128. * f64::EPSILON * scale;
    if !zone.is_finite() || zone <= 0. || zone > eps / 2. {
        return Err(crate::operation_error(
            "no arithmetic room within modeling zone",
        ));
    }
    Ok(zone)
}

impl MigrationGeometry {
    fn resolve_source_face(
        &self,
        key: crate::NewellShapeKey,
        slot: u32,
    ) -> Result<ResolvedNativeFace, crate::NewellKernelError> {
        use crate::operation_error;
        let face = *self
            .slots(key)?
            .get(usize::try_from(slot).map_err(operation_error)?)
            .ok_or_else(|| operation_error("missing source face slot"))?;
        let body = self
            .objects
            .get(&key)
            .ok_or_else(|| operation_error("missing source owner"))?;
        native_planar_frame(&self.kernel, body, face)?;
        let role = self
            .roles
            .get(&key)
            .and_then(|r| {
                r.iter()
                    .find_map(|(&role, &id)| (id == face).then_some(role))
            })
            .unwrap_or(crate::NewellFaceProvenance::Unknown);
        let occurrences = if role == crate::NewellFaceProvenance::Unknown {
            initial_boundary_occurrences(body, face)?
        } else {
            self.occurrences
                .get(&key)
                .map(|all| {
                    all.iter()
                        .filter_map(|(&(r, w, e), &id)| (r == role).then_some(((w, e), id)))
                        .collect()
                })
                .ok_or_else(|| operation_error("missing source occurrences"))?
        };
        Ok((face, occurrences))
    }
    fn insert_anonymous(
        &mut self,
        body: solidcore::Brep,
        surface: bool,
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        let faces = body.face_list().to_vec();
        u32::try_from(faces.len()).map_err(crate::operation_error)?;
        let key = self.insert(body)?;
        self.anonymous_faces.insert(key, faces);
        if surface {
            self.sheet_keys.insert(key);
        }
        Ok(key)
    }
    fn is_surface(&self, key: crate::NewellShapeKey) -> bool {
        self.sheet_keys.contains(&key)
    }
    fn face_count(&self, key: crate::NewellShapeKey) -> Result<usize, crate::NewellKernelError> {
        self.slots(key).map(|s| s.len())
    }
    fn volume(&self, key: crate::NewellShapeKey) -> Result<f64, crate::NewellKernelError> {
        if self.is_surface(key) {
            return Err(crate::operation_error("a sheet has no material volume"));
        }
        let body = self
            .objects
            .get(&key)
            .ok_or_else(|| crate::operation_error("missing volume owner"))?;
        let mut scratch = body
            .clone_standing_for_self()
            .map_err(crate::operation_error)?;
        let store = solidcore::geom::attribute::AttributeStore::default();
        let key_of = |_: solidcore::error::EntityRef| solidcore::geom::attribute::ObjectKey(1);
        let attrs = solidcore::mass::integrator::properties::PropertyAttributes {
            store: &store,
            key_of: &key_of,
        };
        solidcore::mass::integrator::properties::brep_volume(
            &self.kernel,
            &mut scratch,
            PROPERTY_RELATIVE_ACCURACY,
            &attrs,
        )
        .map_err(crate::operation_error)
    }
    fn surface_area(&self, key: crate::NewellShapeKey) -> Result<f64, crate::NewellKernelError> {
        if !self.is_surface(key) {
            return Err(crate::operation_error("not a sheet owner"));
        }
        let body = self
            .objects
            .get(&key)
            .ok_or_else(|| crate::operation_error("missing surface owner"))?;
        let mut scratch = body
            .clone_standing_for_self()
            .map_err(crate::operation_error)?;
        solidcore::mass::integrator::properties::brep_area(
            &self.kernel,
            &mut scratch,
            PROPERTY_RELATIVE_ACCURACY,
        )
        .map_err(crate::operation_error)
    }
    fn join_extrusion_sides(
        &mut self,
        first: crate::NewellShapeKey,
        second: crate::NewellShapeKey,
        profile: (crate::NewellShapeKey, u32),
        direction: [f64; 3],
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        use crate::{NewellFaceProvenance as Role, operation_error};
        self.check_cancelled()?;
        let (face, _) = self.resolve_source_face(profile.0, profile.1)?;
        let source = native_planar_frame(&self.kernel, &self.objects[&profile.0], face)?;
        let dot = |a: [f64; 3], b: [f64; 3]| a.into_iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
        let denominator = dot(source.normal, direction);
        let stations = |key| {
            let body = self.objects.get(&key)?;
            let roles = self.roles.get(&key)?;
            if self.extrusion_sources.get(&key) != Some(&profile) {
                return None;
            }
            let station = |role| {
                let cap = native_planar_frame(&self.kernel, body, *roles.get(&role)?).ok()?;
                if (dot(source.normal, cap.normal).abs() - 1.).abs() > 1e-12 {
                    return None;
                }
                Some(
                    dot(
                        std::array::from_fn(|i| cap.origin[i] - source.origin[i]),
                        source.normal,
                    ) / denominator,
                )
            };
            Some([station(Role::StartCap)?, station(Role::EndCap)?])
        };
        if let (Some(a), Some(b)) = (stations(first), stations(second))
            && denominator.abs() > 1e-12
            && (a[0] - b[0]).abs() <= 64. * f64::EPSILON * a[0].abs().max(1.)
        {
            return self.extrude_face(
                profile.0,
                profile.1,
                Some(direction),
                a[0].min(a[1]).min(b[1]),
                a[0].max(a[1]).max(b[1]),
            );
        }
        self.boolean(first, second, crate::NewellBooleanMode::Add)?
            .ok_or_else(|| operation_error("joined extrusion is empty"))
    }

    fn boolean(
        &mut self,
        target: crate::NewellShapeKey,
        tool: crate::NewellShapeKey,
        mode: crate::NewellBooleanMode,
    ) -> Result<Option<crate::NewellShapeKey>, crate::NewellKernelError> {
        use crate::operation_error;
        self.check_cancelled()?;
        if self.is_surface(target) || self.is_surface(tool) {
            return Err(operation_error(
                "solid operation requires material operands",
            ));
        }
        let target = self
            .objects
            .get(&target)
            .ok_or_else(|| operation_error("missing boolean target"))?;
        let tool = self
            .objects
            .get(&tool)
            .ok_or_else(|| operation_error("missing boolean tool"))?;
        let mut result = native_boolean_copies(&mut self.kernel, target, tool, mode)?;
        self.check_cancelled()?;
        result.refresh_cached_topology();
        let materials = solidcore::query::api_queries::brep_material_census(&result)
            .map_err(operation_error)?
            .0;
        if result.face_list().is_empty()
            && materials == 0
            && mode == crate::NewellBooleanMode::Remove
        {
            return Ok(None);
        }
        if materials == 0
            || !solidcore::query::api_queries::brep_is_manifold_solid(&result)
                .map_err(operation_error)?
        {
            return Err(operation_error("boolean result is not manifold material"));
        }
        // Native booleans may return NURBS faces without their UV trim cache.
        // Prepare it on the unpublished result, preserving modeling tolerances.
        for face in result.face_list().to_vec() {
            self.check_cancelled()?;
            solidcore::topology::face::p3::create_uv_trim_curves(
                &self.kernel,
                &mut result,
                face,
                false,
                None,
            )
            .map_err(operation_error)?;
        }
        self.check_cancelled()?;
        self.insert_anonymous(result, false).map(Some)
    }
    fn body_components(
        &mut self,
        key: crate::NewellShapeKey,
    ) -> Result<Vec<crate::NewellShapeKey>, crate::NewellKernelError> {
        use crate::operation_error;
        self.check_cancelled()?;
        if self.is_surface(key) {
            return Err(operation_error("not a material owner"));
        }
        let source = self
            .objects
            .get(&key)
            .ok_or_else(|| operation_error("missing material owner"))?;
        let count = solidcore::query::api_queries::brep_material_census(source)
            .map_err(operation_error)?
            .0;
        if count == 1 {
            return Ok(vec![key]);
        }
        let mut scratch = source.clone_standing_for_self().map_err(operation_error)?;
        let parts =
            native_material_components(&mut self.kernel, &mut scratch).map_err(operation_error)?;
        self.next
            .checked_add(u64::try_from(parts.len()).map_err(operation_error)?)
            .ok_or_else(|| operation_error("geometry identity exhausted"))?;
        self.check_cancelled()?;
        let mut results = Vec::new();
        for part in parts {
            results.push(self.insert_anonymous(part, false)?);
        }
        Ok(results)
    }

    fn combine_bodies_cancellable(
        &mut self,
        tool: crate::NewellShapeKey,
        targets: &[crate::NewellShapeKey],
        mode: crate::NewellBooleanMode,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<crate::NewellBodyResult, crate::NewellKernelError> {
        use crate::{NewellBooleanMode as B, operation_error};
        let flag = self.cancellation.clone();
        let mut check = || {
            check_native_cancelled(flag.as_deref())?;
            if cancelled() {
                return Err(operation_error("geometry request cancelled"));
            }
            Ok(())
        };
        check()?;
        let tool_body = self
            .objects
            .get(&tool)
            .ok_or_else(|| operation_error("missing combine tool"))?;
        if self.is_surface(tool) {
            return Err(operation_error(
                "solid operation requires material operands",
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut scope = Vec::new();
        for (index, &key) in targets.iter().enumerate() {
            check()?;
            let body = self
                .objects
                .get(&key)
                .ok_or_else(|| operation_error("missing combine target"))?;
            if self.is_surface(key) {
                return Err(operation_error(
                    "solid operation requires material operands",
                ));
            }
            if seen.insert(key) {
                scope.push((index, body));
            }
        }
        let mut changed = Vec::new();
        let mut outputs = Vec::new();
        let mut accepted_zone = 0_f64;
        match mode {
            B::Add => {
                let mut union = independent_native_copy(&mut self.kernel, tool_body)?;
                let mut pending = scope.clone();
                loop {
                    let mut progress = false;
                    let mut remaining = Vec::new();
                    for (index, target) in pending {
                        check()?;
                        let result =
                            native_boolean_copies(&mut self.kernel, &union, target, B::Add)?;
                        accepted_zone = accepted_zone.max(self.kernel.tolerances.zone_tol_3d);
                        let count = solidcore::query::api_queries::brep_material_census(&result)
                            .map_err(operation_error)?
                            .0;
                        if count == 1 {
                            union = result;
                            changed.push(index);
                            progress = true;
                        } else {
                            remaining.push((index, target));
                        }
                    }
                    if !progress || remaining.is_empty() {
                        break;
                    }
                    pending = remaining;
                }
                changed.sort_unstable();
                if changed.len() == 1 {
                    let target = scope
                        .iter()
                        .find_map(|&(index, body)| (index == changed[0]).then_some(body))
                        .ok_or_else(|| operation_error("lost combine target"))?;
                    let old = corpus_volume(&self.kernel, target)?;
                    if corpus_volume(&self.kernel, &union)? <= old + 1e-9_f64.max(old.abs() * 1e-10)
                    {
                        return Err(operation_error("NoTargetIntersection"));
                    }
                }
                outputs.push(union);
            }
            B::Remove => {
                for (index, target) in scope {
                    check()?;
                    let old = corpus_volume(&self.kernel, target)?;
                    let mut result =
                        native_boolean_copies(&mut self.kernel, target, tool_body, B::Remove)?;
                    accepted_zone = accepted_zone.max(self.kernel.tolerances.zone_tol_3d);
                    let count = solidcore::query::api_queries::brep_material_census(&result)
                        .map_err(operation_error)?
                        .0;
                    let measure = if count == 0 && result.face_list().is_empty() {
                        0.
                    } else {
                        corpus_volume(&self.kernel, &result)?
                    };
                    let eps = 1e-9_f64.max(old.abs() * 1e-10);
                    if measure > old + eps {
                        return Err(operation_error("difference increased the target volume"));
                    }
                    if (old - measure).abs() <= eps {
                        continue;
                    }
                    changed.push(index);
                    match count {
                        0 if result.face_list().is_empty() => {}
                        1 => outputs.push(result),
                        0 => return Err(operation_error("difference returned nonmaterial faces")),
                        _ => outputs.extend(
                            native_material_components(&mut self.kernel, &mut result)
                                .map_err(operation_error)?,
                        ),
                    }
                }
            }
        }
        if changed.is_empty() {
            return Err(operation_error("NoTargetIntersection"));
        }
        // All native work and admission checks precede publication of any key.
        for body in &mut outputs {
            check()?;
            body.refresh_cached_topology();
            assert_native_tolerances(body, accepted_zone)?;
            u32::try_from(body.face_list().len()).map_err(operation_error)?;
            if !solidcore::query::api_queries::brep_is_manifold_solid(body)
                .map_err(operation_error)?
            {
                return Err(operation_error("combine result is not manifold material"));
            }
            for face in body.face_list().to_vec() {
                check()?;
                solidcore::topology::face::p3::create_uv_trim_curves(
                    &self.kernel,
                    body,
                    face,
                    false,
                    None,
                )
                .map_err(operation_error)?;
            }
        }
        self.next
            .checked_add(u64::try_from(outputs.len()).map_err(operation_error)?)
            .ok_or_else(|| operation_error("geometry identity exhausted"))?;
        check()?;
        let mut keys = Vec::new();
        for body in outputs {
            keys.push(self.insert_anonymous(body, false)?);
        }
        Ok(crate::NewellBodyResult {
            outputs: keys,
            changed_targets: changed,
        })
    }

    fn combine_surfaces(
        &mut self,
        tools: &[crate::NewellShapeKey],
        targets: &[crate::NewellShapeKey],
        mode: crate::NewellBooleanMode,
    ) -> Result<crate::NewellSurfaceResult, crate::NewellKernelError> {
        use crate::{NewellBooleanMode as B, operation_error};
        self.check_cancelled()?;
        let mut tool = None;
        for &key in tools {
            self.check_cancelled()?;
            if !self.is_surface(key) {
                return Err(operation_error("surface operation requires sheet tools"));
            }
            let body = self
                .objects
                .get(&key)
                .ok_or_else(|| operation_error("missing surface tool"))?;
            tool = Some(match tool {
                None => independent_native_copy(&mut self.kernel, body)?,
                Some(prior) => native_boolean_copies(&mut self.kernel, &prior, body, B::Add)?,
            });
        }
        let tool = tool.ok_or_else(|| operation_error("empty surface tool scope"))?;
        let mut seen = std::collections::BTreeSet::new();
        let mut scope = Vec::new();
        for (index, &key) in targets.iter().enumerate() {
            self.check_cancelled()?;
            if !self.is_surface(key) {
                return Err(operation_error("surface operation requires sheet targets"));
            }
            let body = self
                .objects
                .get(&key)
                .ok_or_else(|| operation_error("missing surface target"))?;
            if seen.insert(key) {
                scope.push((index, body));
            }
        }
        let mut changed = Vec::new();
        let mut outputs = Vec::new();
        match mode {
            B::Add => {
                let mut union = tool;
                let mut pending = scope.clone();
                loop {
                    let mut progress = false;
                    let mut remaining = Vec::new();
                    for (index, target) in pending {
                        self.check_cancelled()?;
                        let mut candidate =
                            native_boolean_copies(&mut self.kernel, &union, target, B::Add)?;
                        self.check_cancelled()?;
                        let separate = sheet_component_count(&union).map_err(operation_error)?
                            + sheet_component_count(target).map_err(operation_error)?;
                        let connected =
                            sheet_component_count(&candidate).map_err(operation_error)? < separate;
                        let old_area = native_sheet_area(&self.kernel, target)?;
                        let new_area = native_sheet_area(&self.kernel, &candidate)?;
                        if connected && new_area > old_area + 1e-9_f64.max(old_area * 1e-10) {
                            candidate.refresh_cached_topology();
                            union = candidate;
                            changed.push(index);
                            progress = true;
                        } else {
                            remaining.push((index, target));
                        }
                    }
                    if !progress || remaining.is_empty() {
                        break;
                    }
                    pending = remaining;
                }
                if !changed.is_empty() {
                    outputs = native_sheet_components(&mut self.kernel, &mut union)?;
                }
            }
            B::Remove => {
                for (index, target) in scope {
                    self.check_cancelled()?;
                    let old_area = native_sheet_area(&self.kernel, target)?;
                    let mut result =
                        native_boolean_copies(&mut self.kernel, target, &tool, B::Remove)?;
                    self.check_cancelled()?;
                    let new_area = if result.face_list().is_empty() {
                        0.
                    } else {
                        native_sheet_area(&self.kernel, &result)?
                    };
                    if new_area > old_area + 1e-9_f64.max(old_area * 1e-10) {
                        return Err(operation_error("surface difference increased area"));
                    }
                    if old_area - new_area <= 1e-9_f64.max(old_area * 1e-10) {
                        continue;
                    }
                    changed.push(index);
                    outputs.extend(native_sheet_components(&mut self.kernel, &mut result)?);
                }
            }
        }
        if changed.is_empty() {
            return Err(operation_error("NoTargetIntersection"));
        }
        for body in &outputs {
            self.check_cancelled()?;
            if solidcore::query::api_queries::brep_material_census(body)
                .map_err(operation_error)?
                .0
                != 0
                || sheet_component_count(body).map_err(operation_error)? != 1
            {
                return Err(operation_error("invalid connected sheet result"));
            }
            u32::try_from(body.face_list().len()).map_err(operation_error)?;
            validate_native_sheet_links(body)?;
        }
        self.next
            .checked_add(u64::try_from(outputs.len()).map_err(operation_error)?)
            .ok_or_else(|| operation_error("geometry identity exhausted"))?;
        self.check_cancelled()?;
        changed.sort_unstable();
        let source = |key| {
            self.extrusion_sources
                .get(&key)
                .copied()
                .zip(self.source_wires.get(&key).copied())
        };
        let wire = tools
            .first()
            .and_then(|&key| source(key))
            .filter(|&lineage| {
                tools
                    .iter()
                    .copied()
                    .chain(changed.iter().map(|&index| targets[index]))
                    .all(|key| source(key) == Some(lineage))
            });
        let mut keys = Vec::new();
        for body in outputs {
            let key = self.insert_anonymous(body, true)?;
            if let Some((profile, wire)) = wire {
                self.extrusion_sources.insert(key, profile);
                self.source_wires.insert(key, wire);
            }
            keys.push(key);
        }
        Ok(crate::NewellSurfaceResult {
            outputs: keys,
            changed_targets: changed,
        })
    }

    fn sweep_surface(
        &mut self,
        profile: (crate::NewellShapeKey, u32),
        boundary: crate::NewellBoundary,
        direction: [f64; 3],
        start: f64,
        end: f64,
    ) -> Result<Vec<crate::NewellShapeKey>, crate::NewellKernelError> {
        use crate::{NewellBoundary as B, operation_error};
        self.check_cancelled()?;
        if !start.is_finite()
            || !end.is_finite()
            || !(end - start).is_finite()
            || end - start <= 1e-7
            || direction.iter().any(|v| !v.is_finite())
        {
            return Err(operation_error(
                "invalid surface sweep interval or direction",
            ));
        }
        let (face, occurrences) = self.resolve_source_face(profile.0, profile.1)?;
        let body = self
            .objects
            .get(&profile.0)
            .ok_or_else(|| operation_error("missing surface source"))?;
        let frame = native_planar_frame(&self.kernel, body, face)?;
        let length = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
        if !length.is_finite() || length <= 0. {
            return Err(operation_error("invalid surface direction"));
        }
        let direction = direction.map(|v| v / length);
        if direction
            .iter()
            .zip(frame.normal)
            .map(|(a, b)| a * b)
            .sum::<f64>()
            .abs()
            <= 1e-12
        {
            return Err(operation_error("tangential Along"));
        }
        let mut groups = std::collections::BTreeMap::<
            u32,
            std::collections::BTreeMap<(u32, u32), solidcore::EdgeId>,
        >::new();
        for ((wire, edge), native) in occurrences {
            let selected = match boundary {
                B::Face => true,
                B::Wire(w) => wire == w,
                B::Edge { wire: w, edge: e } => wire == w && edge == e,
            };
            if selected {
                groups.entry(wire).or_default().insert((wire, edge), native);
            }
        }
        if groups.is_empty() {
            return Err(operation_error("missing surface boundary occurrence"));
        }
        let mut staged = Vec::new();
        for (wire, occurrences) in groups {
            check_native_cancelled(self.cancellation.as_deref())?;
            let output = constructed_sheet_sweep(
                &mut self.kernel,
                (body, face),
                &occurrences,
                direction.map(|v| v * start),
                direction.map(|v| v * (end - start)),
                self.cancellation.as_deref(),
            )?;
            staged.push((output.body, output.roles, output.occurrence_map, wire));
        }
        self.next
            .checked_add(u64::try_from(staged.len()).map_err(operation_error)?)
            .ok_or_else(|| operation_error("geometry identity exhausted"))?;
        self.check_cancelled()?;
        let mut results = Vec::new();
        for (body, roles, names, wire) in staged {
            let key = self.insert(body)?;
            self.roles.insert(key, roles);
            self.occurrences.insert(key, names);
            self.sheet_keys.insert(key);
            self.source_wires.insert(key, wire);
            self.extrusion_sources.insert(key, profile);
            results.push(key);
        }
        Ok(results)
    }
}

#[test]
fn solidcore_migration_owner_material_scopes_deduplicate_bridge_split_empty_and_cancel()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::NewellBooleanMode as B;
    let mut owner = MigrationGeometry::new(20.);
    let left = owner.box_shape([2., 2., 2.], [0.; 3])?;
    let right = owner.box_shape([2., 2., 2.], [4., 0., 0.])?;
    let bridge = owner.box_shape([4., 2., 2.], [1., 0., 0.])?;
    let miss = owner.box_shape([1.; 3], [20.; 3])?;
    let baseline = owner.tessellate(left, 0.01)?;
    for scope in [vec![right, left], vec![right, left, right, left]] {
        let result = owner.combine_bodies_cancellable(bridge, &scope, B::Add, || false)?;
        assert_eq!(result.changed_targets, vec![0, 1]);
        assert_eq!(result.outputs.len(), 1);
        assert!((owner.volume(result.outputs[0])? - 24.).abs() < 1e-7);
    }
    // The first target is separated from the tool until the right body joins.
    let far = owner.box_shape([2., 2., 2.], [5.5, 0., 0.])?;
    let result = owner.combine_bodies_cancellable(bridge, &[far, right, left], B::Add, || false)?;
    assert_eq!(result.changed_targets, vec![0, 1, 2]);
    assert_eq!(result.outputs.len(), 1);
    assert!((owner.volume(result.outputs[0])? - 30.).abs() < 1e-7);
    let target = owner.box_shape([10., 2., 2.], [0.; 3])?;
    let cutter = owner.box_shape([2., 4., 4.], [4., -1., -1.])?;
    for scope in [vec![target, miss], vec![target, miss, target]] {
        let result = owner.combine_bodies_cancellable(cutter, &scope, B::Remove, || false)?;
        assert_eq!(result.changed_targets, vec![0]);
        assert_eq!(result.outputs.len(), 2);
        for key in result.outputs {
            assert!((owner.volume(key)? - 16.).abs() < 1e-7);
            assert!((oriented_mesh_volume(&owner.tessellate(key, 0.01)?)? - 16.).abs() < 1e-5);
        }
    }
    let cover = owner.box_shape([20.; 3], [-5.; 3])?;
    let count = owner.objects.len();
    let next = owner.next;
    let empty = owner.combine_bodies_cancellable(cover, &[target, target], B::Remove, || false)?;
    assert!(empty.outputs.is_empty());
    assert_eq!(empty.changed_targets, vec![0]);
    assert_eq!(owner.objects.len(), count);
    assert_eq!(owner.next, next);
    for (tool, scope, mode) in [
        (left, vec![left], B::Add),
        (miss, vec![left], B::Add),
        (miss, vec![left], B::Remove),
    ] {
        assert!(
            matches!(owner.combine_bodies_cancellable(tool,&scope,mode,||false),
            Err(crate::NewellKernelError::Operation{message}) if message=="NoTargetIntersection")
        );
        assert_eq!(owner.next, next);
    }
    let mut missing_next = owner.next;
    let missing = crate::NewellShapeKey::allocate(&mut missing_next)?;
    assert!(
        owner
            .combine_bodies_cancellable(bridge, &[left, left, missing], B::Add, || false)
            .is_err()
    );
    assert_eq!(owner.next, next);
    let mut polls = 0;
    assert!(
        owner
            .combine_bodies_cancellable(bridge, &[right, left], B::Add, || {
                polls += 1;
                polls == 5
            })
            .is_err()
    );
    assert_eq!(owner.next, next);
    assert_eq!(owner.objects.len(), count);
    assert_eq!(owner.tessellate(left, 0.01)?, baseline);
    owner.next = u64::MAX - 1;
    assert!(
        owner
            .combine_bodies_cancellable(cutter, &[target], B::Remove, || false)
            .is_err()
    );
    assert_eq!(owner.next, u64::MAX - 1);
    assert_eq!(owner.objects.len(), count);
    Ok(())
}

#[test]
fn solidcore_migration_owner_solid_and_sheet_results_are_real_and_atomic()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{
        NewellBooleanMode::{Add, Remove},
        NewellBoundary,
    };
    let mut owner = MigrationGeometry::new(20.);
    let source = owner.box_shape([10.; 3], [0.; 3])?;
    let cutter = owner.box_shape([2., 20., 20.], [4., -5., -5.])?;
    let split = owner
        .boolean(source, cutter, Remove)?
        .ok_or("missing split")?;
    let parts = owner.body_components(split)?;
    assert_eq!(parts.len(), 2);
    for part in parts {
        assert!((owner.volume(part)? - 400.).abs() < 1e-6);
        assert!(!owner.profile_faces(part)?.is_empty());
        let mesh = owner.tessellate(part, 0.01)?;
        assert_eq!(mesh.face_count(), owner.face_count(part)?);
        assert!(
            owner
                .profile_faces(part)?
                .iter()
                .all(|f| f.provenance == crate::NewellFaceProvenance::Unknown)
        );
    }
    let cutter = owner.box_shape([12.; 3], [-1.; 3])?;
    let before = owner.next;
    assert_eq!(owner.boolean(source, cutter, Remove)?, None);
    assert_eq!(owner.next, before);
    let sheets = owner.extrude_surface(crate::NewellSurfaceExtrude {
        profile: (source, 1),
        boundary: NewellBoundary::Edge { wire: 0, edge: 0 },
        direction: [0., 0., 1.],
        start: crate::NewellSurfaceStart::Distance(2.),
        extent: crate::NewellSurfaceExtent::Blind(3.),
        sides: crate::NewellSurfaceSides::One,
    })?;
    assert_eq!(sheets.len(), 1);
    let sheet = sheets[0];
    owner.tag_extrude(sheet, (source, 1), [0., 0., 1.])?;
    assert!(owner.is_surface(sheet));
    assert!((owner.surface_area(sheet)? - 30.).abs() < 1e-6);
    assert!(owner.volume(sheet).is_err());
    assert_eq!(owner.source_wire(sheet), Some(0));
    assert!(!owner.tessellate(sheet, 0.01)?.indices().is_empty());
    let tool = owner.sweep_surface(
        (source, 1),
        NewellBoundary::Edge { wire: 0, edge: 0 },
        [0., 0., 1.],
        4.,
        7.,
    )?[0];
    let added = owner.combine_surfaces(&[tool], &[sheet], Add)?;
    assert_eq!(added.changed_targets, [0]);
    assert_eq!(added.outputs.len(), 1);
    assert!((owner.surface_area(added.outputs[0])? - 50.).abs() < 1e-6);
    let removed = owner.combine_surfaces(&[tool], &[sheet], Remove)?;
    assert_eq!(removed.changed_targets, [0]);
    assert_eq!(removed.outputs.len(), 1);
    assert!((owner.surface_area(removed.outputs[0])? - 20.).abs() < 1e-6);
    let before = owner.next;
    let count = owner.objects.len();
    assert!(
        owner
            .sweep_surface((source, 1), NewellBoundary::Wire(99), [0., 0., 1.], 0., 3.)
            .is_err()
    );
    assert!(owner.boolean(source, sheet, Remove).is_err());
    assert_eq!(owner.next, before);
    assert_eq!(owner.objects.len(), count);
    assert!((owner.volume(source)? - 1000.).abs() < 1e-6);
    Ok(())
}

#[test]
fn solidcore_migration_surface_side_dto_preserves_fixed_extents()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{NewellBoundary, NewellSurfaceExtent as Extent, NewellSurfaceSides as Sides};
    let mut owner = MigrationGeometry::new(20.);
    let source = owner.box_shape([10.; 3], [0.; 3])?;
    let before = owner.tessellate(source, 0.01)?;
    for (sides, expected_area) in [
        (Sides::Symmetric, 60.),
        (
            Sides::Second {
                extent: Extent::Blind(4.),
                reverse: false,
            },
            70.,
        ),
        (
            Sides::Second {
                extent: Extent::Blind(4.),
                reverse: true,
            },
            40.,
        ),
    ] {
        let output = owner.extrude_surface(crate::NewellSurfaceExtrude {
            profile: (source, 1),
            boundary: NewellBoundary::Edge { wire: 0, edge: 0 },
            direction: [0., 0., 1.],
            start: crate::NewellSurfaceStart::Distance(2.),
            extent: Extent::Blind(3.),
            sides,
        })?;
        assert_eq!(output.len(), 1);
        owner.tag_extrude(output[0], (source, 1), [0., 0., 1.])?;
        assert!((owner.surface_area(output[0])? - expected_area).abs() < 1e-6);
        assert!(!owner.tessellate(output[0], 0.01)?.indices().is_empty());
    }
    assert_eq!(owner.tessellate(source, 0.01)?, before);
    Ok(())
}

fn check_native_cancelled(
    flag: Option<&std::sync::atomic::AtomicBool>,
) -> Result<(), crate::NewellKernelError> {
    if flag.is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Acquire)) {
        return Err(crate::operation_error("geometry request cancelled"));
    }
    Ok(())
}

#[test]
fn solidcore_migration_cancelled_owner_preserves_sources_and_recovers()
-> Result<(), Box<dyn std::error::Error>> {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let mut owner = MigrationGeometry::new(10.);
    let source = owner.box_shape([10.; 3], [0.; 3])?;
    let tool = owner.box_shape([2., 10., 10.], [4., 0., 0.])?;
    let original = owner.tessellate(source, 0.01)?;
    let flag = Arc::new(AtomicBool::new(true));
    owner.set_cancellation_flag(Arc::clone(&flag));
    let next = owner.next;
    let keys: Vec<_> = owner.objects.keys().copied().collect();
    for _ in 0..10 {
        assert!(owner.box_shape([1.; 3], [0.; 3]).is_err());
        assert!(owner.extrude_face(source, 1, None, 0., 3.).is_err());
        assert!(
            owner
                .sweep_surface(
                    (source, 1),
                    crate::NewellBoundary::Face,
                    [0., 0., 1.],
                    0.,
                    3.
                )
                .is_err()
        );
        assert!(
            owner
                .boolean(source, tool, crate::NewellBooleanMode::Remove)
                .is_err()
        );
        assert!(owner.body_components(source).is_err());
        assert!(owner.tessellate(source, 0.01).is_err());
        assert!(owner.profile_faces(source).is_err());
        assert_eq!(owner.next, next);
        assert_eq!(owner.objects.keys().copied().collect::<Vec<_>>(), keys);
    }
    // Each real request installs its own flag; the previous cancelled flag stays set.
    owner.set_cancellation_flag(Arc::new(AtomicBool::new(false)));
    assert!(flag.load(Ordering::Acquire));
    assert_eq!(owner.tessellate(source, 0.01)?, original);
    let cut = owner
        .boolean(source, tool, crate::NewellBooleanMode::Remove)?
        .ok_or("missing cut")?;
    assert!((owner.volume(cut)? - 800.).abs() < 1e-6);
    assert_eq!(owner.body_components(cut)?.len(), 2);
    Ok(())
}

#[test]
fn solidcore_migration_invalid_adapter_inputs_are_atomic() -> Result<(), Box<dyn std::error::Error>>
{
    let mut owner = MigrationGeometry::new(10.);
    let source = owner.box_shape([10.; 3], [0.; 3])?;
    let original = owner.tessellate(source, 0.01)?;
    let next = owner.next;
    for _ in 0..10 {
        for size in [[f64::NAN, 1., 1.], [0., 1., 1.]] {
            assert!(matches!(
                owner.box_shape(size, [0.; 3]),
                Err(crate::NewellKernelError::InvalidParameter {
                    parameter: "box recipe"
                })
            ));
        }
        for deflection in [f64::INFINITY, 0., f64::NAN, -1.] {
            assert!(matches!(
                owner.tessellate(source, deflection),
                Err(crate::NewellKernelError::InvalidParameter {
                    parameter: "deflection"
                })
            ));
        }
        assert!(owner.extrude_face(source, u32::MAX, None, 0., 3.).is_err());
        assert!(
            owner
                .extrude_face(source, 1, None, -f64::MAX, f64::MAX)
                .is_err()
        );
        assert!(
            owner
                .sweep_surface(
                    (source, 1),
                    crate::NewellBoundary::Face,
                    [0., 0., 1.],
                    -f64::MAX,
                    f64::MAX
                )
                .is_err()
        );
        assert_eq!(owner.next, next);
        assert_eq!(owner.objects.len(), 1);
        assert_eq!(owner.tessellate(source, 0.01)?, original);
    }
    Ok(())
}

#[test]
fn solidcore_migration_mesh_cancellation_discards_partial_conversion()
-> Result<(), Box<dyn std::error::Error>> {
    use std::cell::Cell;
    let mut owner = MigrationGeometry::new(10.);
    let key = owner.box_shape([10.; 3], [0.; 3])?;
    let body = owner.objects.get(&key).ok_or("missing body")?;
    let slots = owner.slots(key)?;
    let polls = Cell::new(0_usize);
    let count = || {
        polls.set(polls.get() + 1);
        Ok(())
    };
    let original =
        crate::solidcore_mesh::native_body_cancellable(&owner.kernel, body, &slots, 0.01, &count)?;
    let total = polls.get();
    assert!(total > slots.len());
    for boundary in [1, 2, total / 2, total - 1, total] {
        polls.set(0);
        let cancel = || {
            polls.set(polls.get() + 1);
            if polls.get() == boundary {
                Err(crate::operation_error("geometry request cancelled"))
            } else {
                Ok(())
            }
        };
        let error = crate::solidcore_mesh::native_body_cancellable(
            &owner.kernel,
            body,
            &slots,
            0.01,
            &cancel,
        )
        .err()
        .ok_or("cancelled mesh was published")?;
        assert!(
            matches!(error, crate::NewellKernelError::Operation { message } if message == "geometry request cancelled")
        );
        assert_eq!(polls.get(), boundary);
        assert_eq!(owner.tessellate(key, 0.01)?, original);
    }
    Ok(())
}

#[cfg(test)]
fn corpus_profile(
    kernel: &mut Kernel,
    kind: u8,
    scale: f64,
    origin: [f64; 3],
) -> Result<(solidcore::Brep, solidcore::FaceId), crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::{
        curve::{curve::Curve, line::Line},
        math::Point3,
        nurbs::{b_spline_curve::BSplineCurve, crv_basic::Curve as NurbsCurve, math::CPoint},
    };
    let point = |p: [f64; 3]| {
        Point3::new(
            origin[0] + scale * p[0],
            origin[1] + scale * p[1],
            origin[2] + scale * p[2],
        )
    };
    let mut curves = Vec::new();
    match kind {
        0 => {
            for ring in [
                [[0., 0., 0.], [10., 0., 0.], [10., 6., 0.], [0., 6., 0.]],
                [[2., 2., 0.], [2., 4., 0.], [4., 4., 0.], [4., 2., 0.]],
            ] {
                for i in 0..4 {
                    curves.push(Curve::from(
                        Line::create_line_segment(
                            3,
                            point(ring[i]),
                            point(ring[(i + 1) % 4]),
                            None,
                        )
                        .map_err(operation_error)?,
                    ));
                }
            }
        }
        1 => {
            let controls = [[0., 0., 0.], [2., 3., 0.], [4., 2., 0.], [6., 0., 0.]]
                .map(|p| CPoint::from_point(point(p)))
                .to_vec();
            curves.push(Curve::from(BSplineCurve::from_nurb(
                NurbsCurve::new(controls, 3, vec![0., 0., 0., 0., 1., 1., 1., 1.])
                    .map_err(operation_error)?,
                3,
                false,
            )));
            curves.push(Curve::from(
                Line::create_line_segment(3, point([6., 0., 0.]), point([0.; 3]), None)
                    .map_err(operation_error)?,
            ));
        }
        2 => {
            let w = std::f64::consts::FRAC_1_SQRT_2;
            let controls = [
                [1., 0.],
                [1., 1.],
                [0., 1.],
                [-1., 1.],
                [-1., 0.],
                [-1., -1.],
                [0., -1.],
                [1., -1.],
                [1., 0.],
            ]
            .into_iter()
            .zip([1., w, 1., w, 1., w, 1., w, 1.])
            .map(|(p, w)| {
                let p = point([2. * p[0], 2. * p[1], 0.]);
                CPoint::new(p.x * w, p.y * w, p.z * w, w)
            })
            .collect();
            curves.push(Curve::from(BSplineCurve::from_nurb(
                NurbsCurve::new(
                    controls,
                    2,
                    vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
                )
                .map_err(operation_error)?,
                3,
                false,
            )));
        }
        _ => return Err(operation_error("unknown numeric corpus profile")),
    }
    exact_planar_profile(kernel, &curves)
}

fn corpus_volume(kernel: &Kernel, body: &solidcore::Brep) -> Result<f64, crate::NewellKernelError> {
    use crate::operation_error;
    let mut measured = body.clone_standing_for_self().map_err(operation_error)?;
    let store = solidcore::geom::attribute::AttributeStore::default();
    let key_of = |_: solidcore::error::EntityRef| solidcore::geom::attribute::ObjectKey(1);
    let attrs = solidcore::mass::integrator::properties::PropertyAttributes {
        store: &store,
        key_of: &key_of,
    };
    solidcore::mass::integrator::properties::brep_volume(
        kernel,
        &mut measured,
        PROPERTY_RELATIVE_ACCURACY,
        &attrs,
    )
    .map_err(operation_error)
}

#[test]
fn solidcore_migration_f19_profile_scale_translation_corpus()
-> Result<(), Box<dyn std::error::Error>> {
    for scale in [0.1_f64, 1., 10.] {
        for origin in [[0.; 3], [1e6, -1e6, 1e6]] {
            for repeat in 0..10 {
                for (kind, depth, expected) in [
                    (0, 5., 280.),
                    (1, 4., 30.),
                    (2, 3., 12. * std::f64::consts::PI),
                ] {
                    println!(
                        "F19_PROFILE_BEGIN,kind={kind},scale={scale},origin={origin:?},repeat={repeat}"
                    );
                    let phase = std::time::Instant::now();
                    let mut kernel = mapped_kernel(10. * scale);
                    let (profile, face) = corpus_profile(&mut kernel, kind, scale, origin)?;
                    let occurrences = initial_boundary_occurrences(&profile, face)?;
                    let body = capped_sweep_with_lineage(
                        &mut kernel,
                        &profile,
                        face,
                        &occurrences,
                        [0., 0., depth * scale],
                    )?
                    .body;
                    assert!(solidcore::query::api_queries::brep_is_manifold_solid(
                        &body
                    )?);
                    println!("F19_PROFILE_SWEEP,ns={}", phase.elapsed().as_nanos());
                    let phase = std::time::Instant::now();
                    let actual = corpus_volume(&kernel, &body)?;
                    println!("F19_PROFILE_MEASURE,ns={}", phase.elapsed().as_nanos());
                    let bound = 1e-6_f64.max(expected.abs() * 1e-6) * scale.powi(3);
                    let expected = expected * scale.powi(3);
                    assert!(
                        (actual - expected).abs() <= bound,
                        "F19 kind={kind} scale={scale} origin={origin:?} repeat={repeat} actual={actual} expected={expected}"
                    );
                    assert_native_tolerances(&body, kernel.tolerances.zone_tol_3d)?;
                    for deflection in [0.05, 0.01] {
                        println!("F19_PROFILE_MESH_BEGIN,deflection={deflection}");
                        let phase = std::time::Instant::now();
                        match crate::solidcore_mesh::native_body(
                            &kernel,
                            &body,
                            body.face_list(),
                            deflection,
                        ) {
                            Ok(mesh) => {
                                assert_eq!(mesh.face_count(), body.face_list().len());
                                assert!(!mesh.indices().is_empty());
                            }
                            Err(crate::NewellKernelError::MeshPrecisionLoss)
                                if origin != [0.; 3] =>
                            {
                                // The explicit F19 unrepresentable-detail rejection is permitted;
                                // exact construction and measure above must still satisfy their oracle.
                            }
                            Err(error) => return Err(format!("F19 kind={kind} scale={scale} origin={origin:?} repeat={repeat} deflection={deflection}: {error}").into()),
                        }
                        println!("F19_PROFILE_MESH_END,ns={}", phase.elapsed().as_nanos());
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn solidcore_migration_f19_split_empty_scale_translation_corpus()
-> Result<(), Box<dyn std::error::Error>> {
    for scale in [0.1_f64, 1., 10.] {
        for origin in [[0.; 3], [1e6, -1e6, 1e6]] {
            for repeat in 0..10 {
                let mut owner = MigrationGeometry::new(10. * scale);
                let source = owner.box_shape([10. * scale; 3], origin)?;
                let slab = owner.box_shape(
                    [2. * scale, 10. * scale, 10. * scale],
                    [origin[0] + 4. * scale, origin[1], origin[2]],
                )?;
                let cut = owner
                    .boolean(source, slab, crate::NewellBooleanMode::Remove)?
                    .ok_or("missing F19 split")?;
                let parts = owner.body_components(cut)?;
                assert_eq!(parts.len(), 2);
                for part in parts {
                    let actual = owner.volume(part)?;
                    let expected = 400. * scale.powi(3);
                    assert!(
                        (actual - expected).abs() <= 400. * 1e-6 * scale.powi(3),
                        "F19 split scale={scale} origin={origin:?} repeat={repeat} actual={actual} expected={expected}"
                    );
                    let body = owner.objects.get(&part).ok_or("missing component")?;
                    assert_native_tolerances(body, owner.kernel.tolerances.zone_tol_3d)?;
                }
                let covering = owner.box_shape([12. * scale; 3], origin.map(|x| x - scale))?;
                let before = owner.next;
                assert_eq!(
                    owner.boolean(source, covering, crate::NewellBooleanMode::Remove)?,
                    None
                );
                assert_eq!(owner.next, before);
                assert!(
                    (owner.volume(source)? - 1000. * scale.powi(3)).abs()
                        <= 1000. * 1e-6 * scale.powi(3)
                );
            }
        }
    }
    Ok(())
}

/// Exact affine lifting to a plane, preserving homogeneous control coordinates.
/// This is the bounded M4 planar-end construction, not a sampled cap.
fn lift_native_curve_to_plane(
    curve: &solidcore::nurbs::b_spline_curve::BSplineCurve,
    direction: [f64; 3],
    plane: ([f64; 3], f64),
) -> Result<solidcore::nurbs::b_spline_curve::BSplineCurve, crate::NewellKernelError> {
    use crate::operation_error;
    let source = curve
        .gw_nurb()
        .ok_or_else(|| operation_error("missing exact source NURBS"))?;
    let denominator: f64 = plane.0.iter().zip(direction).map(|(a, b)| a * b).sum();
    if !denominator.is_finite() || denominator.abs() <= 1e-12 {
        return Err(operation_error("parallel planar end"));
    }
    let controls = source
        .cpts()
        .iter()
        .map(|p| {
            // Non-rational points carry NLib's NOW sentinel, not a weight.
            let weight = if source.is_rational() { p.w } else { 1. };
            let station =
                (plane.1 * weight - plane.0[0] * p.x - plane.0[1] * p.y - plane.0[2] * p.z)
                    / denominator;
            solidcore::nurbs::math::CPoint::new(
                p.x + direction[0] * station,
                p.y + direction[1] * station,
                p.z + direction[2] * station,
                p.w,
            )
        })
        .collect();
    let lifted =
        solidcore::nurbs::crv_basic::Curve::new(controls, source.degree(), source.knots().to_vec())
            .map_err(operation_error)?;
    Ok(solidcore::nurbs::b_spline_curve::BSplineCurve::from_nurb(
        lifted, 3, false,
    ))
}

fn native_occurrence_spline(
    kernel: &Kernel,
    body: &solidcore::Brep,
    edge: solidcore::EdgeId,
    opposite: bool,
) -> Result<solidcore::nurbs::b_spline_curve::BSplineCurve, crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::{
        curve::curve::CurveKind,
        nurbs::{b_spline_curve::BSplineCurve, crv_basic::Curve as NCurve, math::CPoint},
    };
    let edge = body.edge(edge).map_err(operation_error)?;
    let curve = edge
        .curve
        .as_ref()
        .ok_or_else(|| operation_error("missing occurrence curve"))?;
    let mut owned = curve.copy(kernel).map_err(operation_error)?;
    let mut interval = edge.interval;
    owned
        .trim(
            kernel,
            &mut interval,
            solidcore::nurbs::b_spline_curve_n_lib::TrimMode::Notify,
        )
        .map_err(operation_error)?;
    let mut result = match owned.kind() {
        CurveKind::BSpline(c) => BSplineCurve::from_nurb(
            c.gw_nurb()
                .ok_or_else(|| operation_error("missing trimmed NURBS"))?
                .clone(),
            3,
            false,
        ),
        CurveKind::Line(_) => {
            let a = owned
                .evaluate_point(kernel, interval.min())
                .map_err(operation_error)?;
            let b = owned
                .evaluate_point(kernel, interval.max())
                .map_err(operation_error)?;
            BSplineCurve::from_nurb(
                NCurve::new(
                    vec![CPoint::from_point(a), CPoint::from_point(b)],
                    1,
                    vec![0., 0., 1., 1.],
                )
                .map_err(operation_error)?,
                3,
                false,
            )
        }
        _ => return Err(operation_error("unsupported exact lifted occurrence")),
    };
    if opposite {
        let (lo, hi) = result
            .gw_nurb()
            .ok_or_else(|| operation_error("missing lifted NURBS"))?
            .param_bounds();
        result
            .reverse_parameterization(&solidcore::geom::extent1d::Extent1d::from_min_max(lo, hi))
            .map_err(operation_error)?;
    }
    Ok(result)
}

fn translate_native_curve(
    curve: &solidcore::nurbs::b_spline_curve::BSplineCurve,
    delta: [f64; 3],
) -> Result<solidcore::nurbs::b_spline_curve::BSplineCurve, crate::NewellKernelError> {
    use crate::operation_error;
    let source = curve
        .gw_nurb()
        .ok_or_else(|| operation_error("missing translated source curve"))?;
    let mut controls = source.cpts().to_vec();
    for p in &mut controls {
        let w = if source.is_rational() { p.w } else { 1. };
        if !w.is_finite() || w <= 0. {
            return Err(operation_error("invalid translated curve weight"));
        }
        p.x += delta[0] * w;
        p.y += delta[1] * w;
        p.z += delta[2] * w;
        if !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite() {
            return Err(operation_error("nonfinite translated curve"));
        }
    }
    let data =
        solidcore::nurbs::crv_basic::Curve::new(controls, source.degree(), source.knots().to_vec())
            .map_err(operation_error)?;
    Ok(solidcore::nurbs::b_spline_curve::BSplineCurve::from_nurb(
        data, 3, false,
    ))
}

fn native_planar_end_body(
    kernel: &mut Kernel,
    source: &solidcore::Brep,
    face: solidcore::FaceId,
    direction: [f64; 3],
    plane: NativeStation,
) -> Result<solidcore::Brep, crate::NewellKernelError> {
    let occurrences = initial_boundary_occurrences(source, face)?;
    native_planar_end_with_lineage(
        kernel,
        (source, face),
        &occurrences,
        direction,
        0.,
        plane,
        None,
    )
    .map(|output| output.body)
}

fn native_planar_end_with_lineage(
    kernel: &mut Kernel,
    source: (&solidcore::Brep, solidcore::FaceId),
    occurrences: &std::collections::BTreeMap<(u32, u32), solidcore::EdgeId>,
    direction: [f64; 3],
    start_distance: f64,
    plane: NativeStation,
    cancellation: Option<&std::sync::atomic::AtomicBool>,
) -> Result<NativeNamedBody, crate::NewellKernelError> {
    use crate::{NewellFaceProvenance as P, operation_error};
    use solidcore::{curve::curve::Curve, nurbs::b_spline_surface_n_lib::p2::OrientType};
    check_native_cancelled(cancellation)?;
    if direction
        .iter()
        .chain(plane.0.iter())
        .chain([&plane.1])
        .chain([&start_distance])
        .any(|v| !v.is_finite())
    {
        return Err(operation_error("nonfinite planar end"));
    }
    let length = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !length.is_finite() || length <= 1e-12 {
        return Err(operation_error("invalid planar-end direction"));
    }
    let direction = direction.map(|v| v / length);
    let denominator = (0..3).map(|i| plane.0[i] * direction[i]).sum::<f64>();
    if denominator.abs() <= 1e-12 {
        return Err(operation_error("parallel planar end"));
    }
    let boundary = solidcore::query::api_queries::face_get_boundary_loops(source.0, source.1)
        .map_err(operation_error)?;
    if boundary.edges.len() != occurrences.len() {
        return Err(operation_error("incomplete planar-end source occurrences"));
    }
    let mut names = Vec::new();
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    let mut bounds = None;
    for (&edge, &orientation) in boundary.edges.iter().zip(&boundary.orientations) {
        check_native_cancelled(cancellation)?;
        let keys: Vec<_> = occurrences
            .iter()
            .filter_map(|(&key, &native)| (native == edge).then_some(key))
            .collect();
        let [key] = keys.as_slice() else {
            return Err(operation_error("ambiguous planar-end source occurrence"));
        };
        names.push((*key, edge));
        let original =
            native_occurrence_spline(kernel, source.0, edge, orientation == OrientType::Opposite)?;
        let end = lift_native_curve_to_plane(&original, direction, plane)?;
        let start = if start_distance == 0. {
            original
        } else {
            translate_native_curve(&original, direction.map(|v| v * start_distance))?
        };
        for curve in [&start, &end] {
            let curve = Curve::from(curve.clone());
            let next = curve
                .calculate_tight_bounding_box(kernel, &curve.natural_interval())
                .map_err(operation_error)?;
            bounds = Some(
                bounds.map_or(next, |b: solidcore::geom::extent3d::Extent3d| {
                    b.union(&next)
                }),
            );
        }
        starts.push(start);
        ends.push(end);
    }
    let bounds = bounds.ok_or_else(|| operation_error("empty planar-end source"))?;
    let span = bounds.max_length();
    let scale = (0..3)
        .flat_map(|i| [bounds.min_at(i).abs(), bounds.max_at(i).abs()])
        .fold(span, f64::max);
    let eps = configure_native_policy(kernel, span, scale)?;
    for curve in &starts {
        let minimum = -native_curve_station_max_cancellable(
            curve.clone(),
            (plane.0.map(|v| v / denominator), -plane.1 / denominator),
            &|| check_native_cancelled(cancellation),
        )?;
        if !minimum.is_finite() || minimum <= eps {
            return Err(crate::NewellKernelError::InvalidParameter {
                parameter: "target does not lie beyond the entire start section",
            });
        }
    }
    let mut body =
        solidcore::topology::brep::p3::new_brep(kernel, span, 1000.).map_err(operation_error)?;
    body.set_tolerance(native_construction_zone(eps, scale)?);
    let mut roles = std::collections::BTreeMap::new();
    let mut occurrence_map = NativeOccurrences::new();
    for (role, curves) in [(P::StartCap, &starts), (P::EndCap, &ends)] {
        check_native_cancelled(cancellation)?;
        let curves = curves.iter().cloned().map(Curve::from).collect();
        let (face, owners) = construct_planar_face_owners(kernel, &mut body, curves, eps)?;
        roles.insert(role, face);
        for (&((wire, edge), _), native) in names.iter().zip(owners) {
            occurrence_map.insert((role, wire, edge), native);
        }
    }
    for (((wire, edge), original), (start, end)) in
        names.into_iter().zip(starts.into_iter().zip(ends))
    {
        check_native_cancelled(cancellation)?;
        let vertices = source.0.edge(original).map_err(operation_error)?.vertices;
        let (face, owners) =
            construct_owned_ruled_wall(kernel, &mut body, start, end, vertices[0] == vertices[1])?;
        let role = P::Wall { wire, edge };
        roles.insert(role, face);
        for (slot, native) in owners.into_iter().enumerate() {
            occurrence_map.insert(
                (role, 0, u32::try_from(slot).map_err(operation_error)?),
                native,
            );
        }
    }
    stitch_constructed_occurrences(kernel, &mut body, &roles, &mut occurrence_map)?;
    check_native_cancelled(cancellation)?;
    body.refresh_cached_topology();
    let unique: std::collections::BTreeSet<_> = roles.values().copied().collect();
    if unique.len() != roles.len()
        || roles.len() != body.face_list().len()
        || !solidcore::query::api_queries::brep_is_manifold_solid(&body).map_err(operation_error)?
    {
        return Err(operation_error(
            "planar-end construction lacks uniquely named manifold faces",
        ));
    }
    for (&role, &face) in &roles {
        let boundary = solidcore::query::api_queries::face_get_boundary_loops(&body, face)
            .map_err(operation_error)?;
        let names: Vec<_> = occurrence_map
            .iter()
            .filter_map(|(&(r, _, _), &e)| (r == role).then_some(e))
            .collect();
        if names.len() != boundary.edges.len()
            || names.iter().any(|id| {
                names.iter().filter(|e| *e == id).count()
                    != boundary.edges.iter().filter(|e| *e == id).count()
            })
        {
            return Err(operation_error("lost or ambiguous planar-end occurrence"));
        }
    }
    for &face in roles.values() {
        check_native_cancelled(cancellation)?;
        solidcore::topology::face::p3::create_uv_trim_curves(kernel, &mut body, face, false, None)
            .map_err(operation_error)?;
    }
    check_native_cancelled(cancellation)?;
    assert_native_tolerances(&body, eps / 2.)?;
    Ok(NativeNamedBody {
        body,
        roles,
        occurrence_map,
    })
}

#[test]
fn solidcore_migration_planar_end_constructor_preserves_input_occurrences()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::NewellFaceProvenance as P;
    for (kind, expected) in [(0, 308.8), (1, 39.66), (2, 20. * std::f64::consts::PI)] {
        let mut owner = MigrationGeometry::new(10.);
        let (source, face) = corpus_profile(&mut owner.kernel, kind, 1., [0.; 3])?;
        let mut occurrences = initial_boundary_occurrences(&source, face)?;
        // Rotate each explicit semantic input cycle. This breaks agreement with
        // native loop heads without changing the source geometry or identity.
        let original = occurrences.clone();
        for (&(wire, edge), &native) in &original {
            let count = u32::try_from(original.keys().filter(|(w, _)| *w == wire).count())?;
            occurrences.insert((wire, (edge + 1) % count), native);
        }
        let output = native_planar_end_with_lineage(
            &mut owner.kernel,
            (&source, face),
            &occurrences,
            [0., 0., 1.],
            0.,
            ([-0.1, 0., 1.], 5.),
            None,
        )
        .map_err(|error| format!("kind={kind}: {error}"))?;
        assert_eq!(output.roles.len(), occurrences.len() + 2);
        let volume = corpus_volume(&owner.kernel, &output.body)?;
        assert!(
            (volume - expected).abs() < 1e-6,
            "kind={kind},volume={volume},expected={expected}"
        );
        let key = owner.insert(output.body)?;
        owner.roles.insert(key, output.roles);
        owner.occurrences.insert(key, output.occurrence_map);
        let faces = owner.profile_faces(key)?;
        for role in [P::StartCap, P::EndCap] {
            let cap = faces
                .iter()
                .find(|f| f.provenance == role)
                .ok_or("missing cap metadata")?;
            let keys: std::collections::BTreeSet<_> = cap
                .wires
                .iter()
                .flat_map(|w| w.edges.iter().map(|e| (w.wire, e.edge)))
                .collect();
            assert_eq!(keys, occurrences.keys().copied().collect());
        }
        for deflection in [0.05, 0.01] {
            assert!(!owner.tessellate(key, deflection)?.indices().is_empty());
        }
    }
    Ok(())
}

#[test]
fn solidcore_migration_owner_oblique_displaced_symmetry() -> Result<(), Box<dyn std::error::Error>>
{
    use solidcore::math::Point3;
    for direction in [[0., 0., 2.], [0.6, -0.3, 1.], [0.6, -0.3, -1.]] {
        for start in [-1., 0., 1.5] {
            let mut owner = MigrationGeometry::new(20.);
            let source = owner.box_shape([2., 3., 1.], [3., 4., 5.])?;
            let original = owner.tessellate(source, 0.01)?;
            let key = owner.extrude_face(source, 1, Some(direction), start, start + 2.)?;
            let volume = owner.volume(key)?;
            let frame = owner
                .profile_faces(source)?
                .into_iter()
                .find(|f| f.face == 1)
                .ok_or("source face")?
                .frame;
            let (first, second, scale) = native_station_reflection(frame, direction, start)?;
            let length = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
            let d = direction.map(|v| v / length);
            let den = (0..3).map(|i| d[i] * frame.normal[i]).sum::<f64>();
            let reflect = |p: [f64; 3]| {
                let station = (0..3)
                    .map(|i| frame.normal[i] * (p[i] - frame.origin[i]))
                    .sum::<f64>()
                    / den;
                std::array::from_fn::<_, 3, _>(|i| p[i] - 2. * d[i] * (station - start))
            };
            for p in [[0., 0., 0.], [3., 4., 6.], [5., 7., 9.], [-10., 20., -30.]] {
                let q = first.transform_point(Point3::new(p[0], p[1], p[2]));
                let q = second.transform_point(Point3::new(
                    q.x * scale.x,
                    q.y * scale.y,
                    q.z * scale.z,
                ));
                let expected = reflect(p);
                assert!(
                    ([q.x, q.y, q.z]
                        .iter()
                        .zip(expected)
                        .map(|(a, b)| (a - b).abs())
                        .fold(0., f64::max))
                        < 1e-10
                );
            }
            let mirror = owner.mirror_sweep(key, (source, 1), direction, start)?;
            assert!((owner.volume(mirror)? - volume).abs() < 1e-6);
            let body = owner.objects.get(&mirror).ok_or("mirror")?;
            solidcore::topology::integrity::check(body)?;
            assert!(solidcore::query::api_queries::brep_is_manifold_solid(body)?);
            assert_eq!(
                owner
                    .roles
                    .get(&mirror)
                    .ok_or("roles")?
                    .keys()
                    .collect::<Vec<_>>(),
                owner
                    .roles
                    .get(&key)
                    .ok_or("roles")?
                    .keys()
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                owner
                    .occurrences
                    .get(&mirror)
                    .ok_or("occurrences")?
                    .keys()
                    .collect::<Vec<_>>(),
                owner
                    .occurrences
                    .get(&key)
                    .ok_or("occurrences")?
                    .keys()
                    .collect::<Vec<_>>()
            );
            let input = owner.objects.get(&key).ok_or("input")?;
            for &vertex in input.vertex_list() {
                let p = input.vertex(vertex)?.point;
                let expected = reflect([p.x, p.y, p.z]);
                assert!(
                    body.vertex_list()
                        .iter()
                        .any(|&v| body.vertex(v).is_ok_and(|v| {
                            [v.point.x, v.point.y, v.point.z]
                                .iter()
                                .zip(expected)
                                .all(|(a, b)| (a - b).abs() < 1e-9)
                        }))
                );
            }
            for deflection in [0.05, 0.01] {
                let mesh = owner.tessellate(mirror, deflection)?;
                for face in owner.profile_faces(mirror)? {
                    let slot = usize::try_from(face.face)?;
                    let range = usize::try_from(mesh.face_offsets()[slot])?
                        ..usize::try_from(mesh.face_offsets()[slot + 1])?;
                    for &vertex in &mesh.indices()[range] {
                        let normal = mesh.normals()[usize::try_from(vertex)?];
                        assert!(
                            (0..3)
                                .map(|i| f64::from(normal[i]) * face.frame.normal[i])
                                .sum::<f64>()
                                > 0.99999
                        );
                    }
                }
                let mesh_volume = oriented_mesh_volume(&mesh)?;
                assert!(
                    (mesh_volume - volume).abs() < 1e-4,
                    "direction={direction:?},start={start},mesh_volume={mesh_volume},volume={volume}"
                );
            }
            let again = owner.mirror_sweep(mirror, (source, 1), direction, start)?;
            assert!((owner.volume(again)? - volume).abs() < 1e-6);
            assert_eq!(owner.tessellate(source, 0.01)?, original);
            let next = owner.next;
            for (d, s) in [
                ([1., 0., 0.], start),
                ([0.; 3], start),
                (direction, f64::NAN),
            ] {
                assert!(owner.mirror_sweep(key, (source, 1), d, s).is_err());
                assert_eq!(owner.next, next);
            }
        }
    }
    Ok(())
}

#[test]
fn solidcore_migration_reflection_preserves_holes_cubic_and_periodic_caps()
-> Result<(), Box<dyn std::error::Error>> {
    for kind in 0..3 {
        let mut owner = MigrationGeometry::new(20.);
        let (profile, _) = corpus_profile(&mut owner.kernel, kind, 1., [0.; 3])?;
        let profile = owner.insert_anonymous(profile, false)?;
        let input = owner.extrude_face(profile, 0, Some([0.6, 0., 0.8]), 2., 5.)?;
        let volume = owner.volume(input)?;
        let mirror = owner.mirror_sweep(input, (profile, 0), [0.6, 0., 0.8], 2.)?;
        assert!((owner.volume(mirror)? - volume).abs() < 1e-6);
        let input_body = owner.objects.get(&input).ok_or("input")?;
        let mirrored_body = owner.objects.get(&mirror).ok_or("mirror")?;
        assert_eq!(
            input_body.face_list().len(),
            mirrored_body.face_list().len()
        );
        assert!(solidcore::query::api_queries::brep_is_manifold_solid(
            mirrored_body
        )?);
        for deflection in [0.05, 0.01] {
            let original = owner.tessellate(input, deflection)?;
            let reflected = owner.tessellate(mirror, deflection)?;
            let a = oriented_mesh_volume(&original)?;
            let b = oriented_mesh_volume(&reflected)?;
            assert!(a > 0. && b > 0.);
            assert!((a - b).abs() < 1e-4, "kind={kind},a={a},b={b}");
        }
    }
    Ok(())
}

#[test]
fn solidcore_migration_owner_surface_tilt_uses_chain_coverage_and_atomic_failure()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::NewellBoundary as B;
    use solidcore::{
        curve::{curve::Curve, line::Line},
        math::Point3,
    };
    let mut owner = MigrationGeometry::new(20.);
    let source = owner.box_shape([2., 3., 1.], [0.; 3])?;
    let original = owner.tessellate(source, 0.01)?;
    let make = |owner: &mut MigrationGeometry,
                hole: [[f64; 2]; 4]|
     -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        let mut curves = Vec::new();
        for ring in [[[-1., -1.], [3., -1.], [3., 4.], [-1., 4.]], hole] {
            for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
                curves.push(Curve::from(
                    Line::create_line_segment(
                        3,
                        Point3::new(a[0], a[1], 5. + 0.1 * a[0]),
                        Point3::new(b[0], b[1], 5. + 0.1 * b[0]),
                        None,
                    )
                    .map_err(crate::operation_error)?,
                ));
            }
        }
        let (body, _) = exact_planar_profile(&mut owner.kernel, &curves)?;
        owner.insert_anonymous(body, false)
    };
    let target = make(&mut owner, [[0.75, 1.], [0.75, 2.], [1.25, 2.], [1.25, 1.]])?;
    for (start, clearance, expected) in [(0., 0., 41.), (1., 0.5, 26.)] {
        let keys = owner.sweep_surface_to_face(
            (source, 1),
            B::Face,
            [0., 0., 1.],
            start,
            (target, 0),
            clearance,
        )?;
        assert_eq!(keys.len(), 1);
        assert!((owner.surface_area(keys[0])? - expected).abs() < 1e-6);
        assert_eq!(owner.face_count(keys[0])?, 4);
        for deflection in [0.05, 0.01] {
            assert!(!owner.tessellate(keys[0], deflection)?.indices().is_empty());
        }
    }
    assert!(
        owner
            .extrude_to_face((source, 1), [0., 0., 1.], 0., (target, 0), 0.)
            .is_err()
    );
    let crossing = make(
        &mut owner,
        [[0.75, -0.5], [0.75, 0.5], [1.25, 0.5], [1.25, -0.5]],
    )?;
    let count = owner.objects.len();
    let next = owner.next;
    assert!(
        matches!(owner.sweep_surface_to_face((source,1),B::Face,[0.,0.,1.],0.,(crossing,0),0.),
        Err(crate::NewellKernelError::Operation{message}) if message=="target trim does not cover selected chain")
    );
    assert_eq!(owner.objects.len(), count);
    assert_eq!(owner.next, next);
    owner.set_cancellation_flag(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
        true,
    )));
    assert!(
        owner
            .sweep_surface_to_face((source, 1), B::Face, [0., 0., 1.], 0., (target, 0), 0.)
            .is_err()
    );
    owner.set_cancellation_flag(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
        false,
    )));
    assert_eq!(owner.tessellate(source, 0.01)?, original);
    assert_eq!(owner.objects.len(), count);
    assert_eq!(owner.next, next);
    Ok(())
}

#[test]
fn solidcore_migration_owner_surface_up_to_named_curves_and_atomic_batch()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::NewellBoundary as B;
    let mut owner = MigrationGeometry::new(20.);
    let source = owner.box_shape([2., 3., 1.], [0.; 3])?;
    let target = owner.box_shape([20., 20., 1.], [-5., -5., 5.])?;
    let original = owner.tessellate(source, 0.01)?;
    for (boundary, start, clearance, expected) in [
        (B::Face, 0., 0., 40.),
        (B::Wire(0), 1., 0.5, 25.),
        (B::Edge { wire: 0, edge: 0 }, -1., 0., 10.),
    ] {
        let keys = owner.extrude_surface(crate::NewellSurfaceExtrude {
            profile: (source, 1),
            boundary,
            direction: [0., 0., 2.],
            start: crate::NewellSurfaceStart::Distance(start),
            extent: crate::NewellSurfaceExtent::UpTo(crate::NewellFaceTarget {
                shape: target,
                face: 0,
                clearance,
            }),
            sides: crate::NewellSurfaceSides::One,
        })?;
        assert_eq!(keys.len(), 1);
        assert!((owner.surface_area(keys[0])? - expected).abs() < 1e-6);
        for deflection in [0.05, 0.01] {
            assert!(!owner.tessellate(keys[0], deflection)?.indices().is_empty());
        }
    }
    let n = 4096;
    let speed = |i: usize| {
        let t = i as f64 / n as f64;
        6_f64.hypot(9. - 24. * t + 9. * t * t)
    };
    let length = (0..=n)
        .map(|i| {
            speed(i)
                * if i == 0 || i == n {
                    1.
                } else if i % 2 == 0 {
                    2.
                } else {
                    4.
                }
        })
        .sum::<f64>()
        / (3. * n as f64);
    for (kind, expected, count) in [
        (0, 200., 2),
        (1, 5. * (length + 6.), 1),
        (2, 20. * std::f64::consts::PI, 1),
    ] {
        let (profile, _) = corpus_profile(&mut owner.kernel, kind, 1., [0.; 3])?;
        let key = owner.insert_anonymous(profile, false)?;
        let keys =
            owner.sweep_surface_to_face((key, 0), B::Face, [0., 0., 1.], 0., (target, 0), 0.)?;
        assert_eq!(keys.len(), count);
        let area = keys
            .iter()
            .map(|&k| owner.surface_area(k))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .sum::<f64>();
        assert!(
            (area - expected).abs() < 1e-6,
            "kind={kind},area={area},expected={expected}"
        );
        for key in keys {
            assert!(!owner.tessellate(key, 0.01)?.indices().is_empty());
        }
    }
    assert_eq!(owner.tessellate(source, 0.01)?, original);
    let (profile, _) = corpus_profile(&mut owner.kernel, 0, 1., [0.; 3])?;
    let profile = owner.insert_anonymous(profile, false)?;
    owner.next = u64::MAX - 1;
    let count = owner.objects.len();
    assert!(
        owner
            .sweep_surface_to_face((profile, 0), B::Face, [0., 0., 1.], 0., (target, 0), 0.)
            .is_err()
    );
    assert_eq!(owner.next, u64::MAX - 1);
    assert_eq!(owner.objects.len(), count);
    assert!(
        owner
            .sweep_surface_to_face(
                (source, 1),
                B::Wire(u32::MAX),
                [0., 0., 1.],
                0.,
                (target, 0),
                0.
            )
            .is_err()
    );
    assert!(
        owner
            .sweep_surface_to_face((source, 1), B::Face, [0., 0., 1.], 4., (target, 0), 0.)
            .is_err()
    );
    assert_eq!(owner.objects.len(), count);
    Ok(())
}

#[test]
fn solidcore_migration_f12_retained_symmetry_union_and_start_target()
-> Result<(), Box<dyn std::error::Error>> {
    let mut owner = MigrationGeometry::new(20.);
    let source = owner.box_shape([2., 3., 1.], [0.; 3])?;
    let direction = [0.6, 0., 0.8];
    let main = owner.extrude_face(source, 1, Some(direction), 2., 7.)?;
    let second = owner.mirror_sweep(main, (source, 1), direction, 2.)?;
    for key in [main, second] {
        assert!((owner.volume(key)? - 24.).abs() < 1e-7);
    }
    let combined = owner
        .boolean(main, second, crate::NewellBooleanMode::Add)?
        .ok_or("empty union")?;
    assert!((owner.volume(combined)? - 48.).abs() < 1e-7);
    for deflection in [0.05, 0.01] {
        let mesh = owner.tessellate(combined, deflection)?;
        assert!((oriented_mesh_volume(&mesh)? - 48.).abs() < 1e-4);
    }
    let target = owner.box_shape([20., 20., 1.], [-10., -10., 3.])?;
    let station = owner.starting_station((source, 1), direction, (target, 0))?;
    assert!((station - 2.5).abs() < 1e-10);
    Ok(())
}

#[test]
fn solidcore_migration_owner_planar_up_to_start_clearance_and_through_all()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::NewellFaceProvenance as P;
    let mut owner = MigrationGeometry::new(10.);
    let source = owner.box_shape([2., 3., 1.], [0.; 3])?;
    let target = owner.box_shape([4., 5., 1.], [-1., -1., 5.])?;
    let original = owner.tessellate(source, 0.01)?;
    let before_start = owner.next;
    for (direction, expected) in [
        ([0., 0., 2.], 4.),
        ([0., 0., -3.], -4.),
        ([0.1, 0., 1.], 4. * 1.01_f64.sqrt()),
    ] {
        let station = owner.starting_station((source, 1), direction, (target, 0))?;
        assert!(
            (station - expected).abs() < 1e-9,
            "station={station}, expected={expected}"
        );
    }
    assert_eq!(
        owner.starting_station((source, 1), [0., 0., 1.], (source, 1))?,
        0.
    );
    assert_eq!(owner.next, before_start);
    for (start, clearance, expected) in
        [(0., 0., 24.), (0., 0.5, 21.), (1., 0., 18.), (-1., 0., 30.)]
    {
        let key =
            owner.extrude_to_face((source, 1), [0., 0., 2.], start, (target, 0), clearance)?;
        assert!((owner.volume(key)? - expected).abs() < 1e-6);
        let faces = owner.profile_faces(key)?;
        let base = faces
            .iter()
            .find(|f| f.provenance == P::StartCap)
            .ok_or("start cap")?;
        let end = faces
            .iter()
            .find(|f| f.provenance == P::EndCap)
            .ok_or("end cap")?;
        assert!((base.frame.origin[2] - (1. + start)).abs() < 1e-9);
        assert!((end.frame.origin[2] - (5. - clearance)).abs() < 1e-9);
        assert_eq!(base.wires[0].edges.len(), 4);
        assert_eq!(end.wires[0].edges.len(), 4);
        assert!(!owner.tessellate(key, 0.01)?.indices().is_empty());
        assert_eq!(owner.tessellate(source, 0.01)?, original);
    }
    let next = owner.next;
    let count = owner.objects.len();
    for (start, clearance, slot, direction) in [
        (f64::NAN, 0., 0, [0., 0., 1.]),
        (4.1, 0., 0, [0., 0., 1.]),
        (0., 4., 0, [0., 0., 1.]),
        (0., 0., u32::MAX, [0., 0., 1.]),
        (0., 0., 0, [1., 0., 0.]),
    ] {
        assert!(
            owner
                .extrude_to_face((source, 1), direction, start, (target, slot), clearance)
                .is_err()
        );
        assert_eq!(owner.next, next);
        assert_eq!(owner.objects.len(), count);
    }
    let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    owner.set_cancellation_flag(flag);
    assert!(
        owner
            .starting_station((source, 1), [0., 0., 1.], (target, 0))
            .is_err()
    );
    assert!(
        owner
            .extrude_to_face((source, 1), [0., 0., 1.], 0., (target, 0), 0.)
            .is_err()
    );
    assert!(
        owner
            .through_all_station((source, 1), [0., 0., 1.], 0., &[target])
            .is_err()
    );
    assert_eq!(owner.next, next);
    owner.set_cancellation_flag(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
        false,
    )));
    let reach =
        owner.through_all_station((source, 1), [0., 0., 1.], 0., &[source, target, target])?;
    assert!((reach - 5.000001).abs() < 1e-9, "owner ThroughAll={reach}");
    let reverse = owner.through_all_station((source, 1), [0., 0., -1.], 0., &[source])?;
    assert!(
        (reverse - 1.000001).abs() < 1e-9,
        "self-target reverse reach={reverse}"
    );
    let displaced = owner.through_all_station((source, 1), [0., 0., 1.], -0.5, &[source])?;
    assert!(
        (displaced - 1e-6).abs() < 1e-9,
        "self-target displaced reach={displaced}"
    );
    assert_eq!(owner.next, next);
    assert_eq!(owner.objects.len(), count);
    assert_eq!(owner.tessellate(source, 0.01)?, original);
    Ok(())
}

/// Finite planar UpTo uses an exact filled projected cap. Subtracting the
/// selected target trim proves coverage of the entire region, including holes
/// wholly inside the cap and concavities between its boundary vertices.
#[cfg(test)]
fn native_finite_planar_end(
    kernel: &mut Kernel,
    source: (&solidcore::Brep, solidcore::FaceId),
    target: (&solidcore::Brep, solidcore::FaceId),
    direction: [f64; 3],
    clearance: f64,
) -> Result<solidcore::Brep, crate::NewellKernelError> {
    let (direction, plane) = native_finite_planar_end_parameters(
        kernel, source, target, direction, 0., clearance, None,
    )?;
    native_planar_end_body(kernel, source.0, source.1, direction, plane)
}

/// Resolve a parallel finite start in either direction, including a coincident
/// section. Coverage is proved on the exact filled projected profile, so a hole
/// entirely inside that profile cannot be hidden by boundary-only tests.
fn native_starting_station(
    kernel: &mut Kernel,
    source: (&solidcore::Brep, solidcore::FaceId),
    target: (&solidcore::Brep, solidcore::FaceId),
    direction: [f64; 3],
    cancellation: Option<&std::sync::atomic::AtomicBool>,
) -> Result<f64, crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::nurbs::b_spline_surface_n_lib::p2::OrientType;
    check_native_cancelled(cancellation)?;
    let length = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !length.is_finite() || length <= 1e-12 {
        return Err(operation_error("invalid starting direction"));
    }
    let direction = direction.map(|v| v / length);
    let options = solidcore::query::brep::BoundingBoxOptions {
        approx: false,
        tight: true,
        expand_by_zone_tol_3d: false,
    };
    let bounds = solidcore::query::brep::calculate_bounding_box(&*kernel, source.0, options)
        .map_err(operation_error)?
        .union(
            &solidcore::query::brep::calculate_bounding_box(&*kernel, target.0, options)
                .map_err(operation_error)?,
        );
    let span = bounds.max_length();
    let scale = (0..3)
        .flat_map(|i| [bounds.min_at(i).abs(), bounds.max_at(i).abs()])
        .fold(span, f64::max);
    let eps = configure_native_policy(kernel, span, scale)?;
    assert_native_tolerances(source.0, eps / 2.)?;
    assert_native_tolerances(target.0, eps / 2.)?;
    let p = native_planar_frame(kernel, source.0, source.1)?;
    let t = native_planar_frame(kernel, target.0, target.1)?;
    let cross = [
        p.normal[1] * t.normal[2] - p.normal[2] * t.normal[1],
        p.normal[2] * t.normal[0] - p.normal[0] * t.normal[2],
        p.normal[0] * t.normal[1] - p.normal[1] * t.normal[0],
    ];
    if cross.iter().map(|v| v * v).sum::<f64>().sqrt() > 1e-10 {
        return Err(operation_error("tilted starting target"));
    }
    let denominator = (0..3).map(|i| p.normal[i] * direction[i]).sum::<f64>();
    if denominator.abs() <= 1e-12 {
        return Err(operation_error("tangential Along"));
    }
    let station = (0..3)
        .map(|i| p.normal[i] * (t.origin[i] - p.origin[i]))
        .sum::<f64>()
        / denominator;
    if !station.is_finite() {
        return Err(operation_error("starting station"));
    }
    let plane = (t.normal, (0..3).map(|i| t.normal[i] * t.origin[i]).sum());
    let boundary = solidcore::query::api_queries::face_get_boundary_loops(source.0, source.1)
        .map_err(operation_error)?;
    let mut projected = Vec::new();
    for (&edge, &orientation) in boundary.edges.iter().zip(&boundary.orientations) {
        check_native_cancelled(cancellation)?;
        let curve =
            native_occurrence_spline(kernel, source.0, edge, orientation == OrientType::Opposite)?;
        projected.push(solidcore::curve::curve::Curve::from(
            lift_native_curve_to_plane(&curve, direction, plane)?,
        ));
    }
    let (projected, _) = exact_planar_profile(kernel, &projected)?;
    let selected = copy_selected_native_face(kernel, target.0, target.1)?;
    let uncovered = planar_surface_boolean_copies(
        kernel,
        &projected,
        &selected,
        crate::NewellBooleanMode::Remove,
    )?;
    check_native_cancelled(cancellation)?;
    if !uncovered.face_list().is_empty() {
        return Err(operation_error("starting target trim coverage"));
    }
    Ok(station)
}

fn native_finite_planar_end_parameters(
    kernel: &mut Kernel,
    source: (&solidcore::Brep, solidcore::FaceId),
    target: (&solidcore::Brep, solidcore::FaceId),
    direction: [f64; 3],
    start: f64,
    clearance: f64,
    cancellation: Option<&std::sync::atomic::AtomicBool>,
) -> Result<([f64; 3], NativeStation), crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::nurbs::b_spline_surface_n_lib::p2::OrientType;
    check_native_cancelled(cancellation)?;
    if !clearance.is_finite() || clearance < 0. || direction.iter().any(|v| !v.is_finite()) {
        return Err(operation_error("invalid UpTo extent"));
    }
    let length = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !length.is_finite() || length <= 1e-12 {
        return Err(operation_error("invalid UpTo direction"));
    }
    let direction = direction.map(|v| v / length);
    let options = solidcore::query::brep::BoundingBoxOptions {
        approx: false,
        tight: true,
        expand_by_zone_tol_3d: false,
    };
    let bounds = solidcore::query::brep::calculate_bounding_box(&*kernel, source.0, options)
        .map_err(operation_error)?
        .union(
            &solidcore::query::brep::calculate_bounding_box(&*kernel, target.0, options)
                .map_err(operation_error)?,
        );
    let span = bounds.max_length();
    let scale = (0..3)
        .flat_map(|i| [bounds.min_at(i).abs(), bounds.max_at(i).abs()])
        .fold(span, f64::max);
    let operation_eps = configure_native_policy(kernel, span, scale)?;
    assert_native_tolerances(source.0, operation_eps / 2.)?;
    assert_native_tolerances(target.0, operation_eps / 2.)?;
    let frame = native_planar_frame(kernel, target.0, target.1)?;
    let plane = (
        frame.normal,
        (0..3)
            .map(|i| frame.normal[i] * frame.origin[i])
            .sum::<f64>(),
    );
    let denominator = (0..3).map(|i| plane.0[i] * direction[i]).sum::<f64>();
    if denominator.abs() <= 1e-12 {
        return Err(operation_error("target parallel to sweep"));
    }
    let boundary = solidcore::query::api_queries::face_get_boundary_loops(source.0, source.1)
        .map_err(operation_error)?;
    let mut projected = Vec::new();
    for (&edge, &orientation) in boundary.edges.iter().zip(&boundary.orientations) {
        check_native_cancelled(cancellation)?;
        let curve =
            native_occurrence_spline(kernel, source.0, edge, orientation == OrientType::Opposite)?;
        // Negated station maximum gives a certified complete-curve minimum.
        // Exact knot splitting resolves hull overshoot; no vertex-only test or
        // conservative control-polygon rejection replaces the curve extremum.
        let minimum = -native_curve_station_max_cancellable(
            curve.clone(),
            (plane.0.map(|n| n / denominator), -plane.1 / denominator),
            &|| check_native_cancelled(cancellation),
        )?;
        if !minimum.is_finite() || minimum - start - clearance <= operation_eps {
            return Err(crate::NewellKernelError::InvalidParameter {
                parameter: "target does not lie beyond the entire start section",
            });
        }
        projected.push(solidcore::curve::curve::Curve::from(
            lift_native_curve_to_plane(&curve, direction, plane)?,
        ));
    }
    let (projected, _) = exact_planar_profile(kernel, &projected)?;
    // Copy exactly the selected occurrence; other target faces are not coverage.
    let selected = copy_selected_native_face(kernel, target.0, target.1)?;
    let uncovered = planar_surface_boolean_copies(
        kernel,
        &projected,
        &selected,
        crate::NewellBooleanMode::Remove,
    )?;
    check_native_cancelled(cancellation)?;
    if !uncovered.face_list().is_empty() {
        return Err(operation_error(
            "target trim does not cover the entire profile",
        ));
    }
    Ok((direction, (plane.0, plane.1 - clearance * denominator)))
}

impl MigrationGeometry {
    fn starting_station(
        &mut self,
        profile: (crate::NewellShapeKey, u32),
        direction: [f64; 3],
        target: (crate::NewellShapeKey, u32),
    ) -> Result<f64, crate::NewellKernelError> {
        self.check_cancelled()?;
        let (source_face, _) = self.resolve_source_face(profile.0, profile.1)?;
        let (target_face, _) = self.resolve_source_face(target.0, target.1)?;
        let source = self
            .objects
            .get(&profile.0)
            .ok_or_else(|| crate::operation_error("missing starting source"))?;
        let target = self
            .objects
            .get(&target.0)
            .ok_or_else(|| crate::operation_error("missing starting target"))?;
        native_starting_station(
            &mut self.kernel,
            (source, source_face),
            (target, target_face),
            direction,
            self.cancellation.as_deref(),
        )
    }

    fn surface_end_is_tilted(
        &self,
        profile: (crate::NewellShapeKey, u32),
        target: crate::NewellFaceTarget,
    ) -> Result<bool, crate::NewellKernelError> {
        let frame = |key, slot| {
            let (face, _) = self.resolve_source_face(key, slot)?;
            native_planar_frame(&self.kernel, &self.objects[&key], face)
        };
        let a = frame(profile.0, profile.1)?.normal;
        let b = frame(target.shape, target.face)?.normal;
        Ok((0..3).any(|i| {
            let j = (i + 1) % 3;
            let k = (i + 2) % 3;
            (a[j] * b[k] - a[k] * b[j]).abs() > 1e-10
        }))
    }

    fn sweep_surface_to_face(
        &mut self,
        profile: (crate::NewellShapeKey, u32),
        boundary: crate::NewellBoundary,
        direction: [f64; 3],
        start: f64,
        target: (crate::NewellShapeKey, u32),
        clearance: f64,
    ) -> Result<Vec<crate::NewellShapeKey>, crate::NewellKernelError> {
        self.sweep_surface_between(
            profile,
            boundary,
            direction,
            SheetStart::Station(start),
            crate::NewellFaceTarget {
                shape: target.0,
                face: target.1,
                clearance,
            },
        )
    }

    fn sweep_surface_between(
        &mut self,
        profile: (crate::NewellShapeKey, u32),
        boundary: crate::NewellBoundary,
        direction: [f64; 3],
        start: SheetStart<crate::NewellFaceTarget>,
        target: crate::NewellFaceTarget,
    ) -> Result<Vec<crate::NewellShapeKey>, crate::NewellKernelError> {
        use crate::{NewellBoundary as B, operation_error};
        self.check_cancelled()?;
        let (face, occurrences) = self.resolve_source_face(profile.0, profile.1)?;
        let (target_face, _) = self.resolve_source_face(target.shape, target.face)?;
        let target = (&self.objects[&target.shape], target_face, target.clearance);
        let start = match start {
            SheetStart::Station(station) => SheetStart::Station(station),
            SheetStart::Face(lower) => {
                let (face, _) = self.resolve_source_face(lower.shape, lower.face)?;
                SheetStart::Face((&self.objects[&lower.shape], face, lower.clearance))
            }
        };
        let source = self
            .objects
            .get(&profile.0)
            .ok_or_else(|| operation_error("missing surface source"))?;
        let mut groups = std::collections::BTreeMap::<u32, NativeBoundary>::new();
        for ((wire, edge), native) in occurrences {
            if match boundary {
                B::Face => true,
                B::Wire(w) => wire == w,
                B::Edge { wire: w, edge: e } => wire == w && edge == e,
            } {
                groups.entry(wire).or_default().insert((wire, edge), native);
            }
        }
        if groups.is_empty() {
            return Err(operation_error("missing surface boundary occurrence"));
        }
        let mut staged = Vec::new();
        for (wire, selected) in groups {
            staged.push((
                wire,
                constructed_sheet_to_face(
                    &mut self.kernel,
                    (source, face),
                    &selected,
                    direction,
                    start,
                    target,
                    self.cancellation.as_deref(),
                )?,
            ));
        }
        self.next
            .checked_add(u64::try_from(staged.len()).map_err(operation_error)?)
            .ok_or_else(|| operation_error("geometry identity exhausted"))?;
        self.check_cancelled()?;
        let mut keys = Vec::new();
        for (wire, output) in staged {
            let key = self.insert(output.body)?;
            self.roles.insert(key, output.roles);
            self.occurrences.insert(key, output.occurrence_map);
            self.source_wires.insert(key, wire);
            self.sheet_keys.insert(key);
            self.extrusion_sources.insert(key, profile);
            keys.push(key);
        }
        Ok(keys)
    }

    fn mirror_sweep(
        &mut self,
        shape: crate::NewellShapeKey,
        profile: (crate::NewellShapeKey, u32),
        direction: [f64; 3],
        start: f64,
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        use crate::operation_error;
        use solidcore::error::EntityRef;
        self.check_cancelled()?;
        let (face, _) = self.resolve_source_face(profile.0, profile.1)?;
        let profile_body = self
            .objects
            .get(&profile.0)
            .ok_or_else(|| operation_error("missing symmetric profile"))?;
        let frame = native_planar_frame(&self.kernel, profile_body, face)?;
        let (first, second, scale) = native_station_reflection(frame, direction, start)?;
        let source = self
            .objects
            .get(&shape)
            .ok_or_else(|| operation_error("missing symmetric shape"))?;
        let (mut output, backward, forward) = native_copy_with_lineage(&mut self.kernel, source)?;
        let corresponding = |old| {
            let new = forward
                .get_at(Some(old))
                .map_err(operation_error)?
                .ok_or_else(|| operation_error("missing symmetric copy lineage"))?;
            if backward.get_at(Some(new)).map_err(operation_error)? != Some(old) {
                return Err(operation_error("nonreciprocal symmetric copy lineage"));
            }
            Ok(new)
        };
        let mut roles = std::collections::BTreeMap::new();
        for (&role, &old) in self.roles.get(&shape).into_iter().flatten() {
            let EntityRef::Face(new) = corresponding(EntityRef::Face(old))? else {
                return Err(operation_error("symmetric face copy dimension"));
            };
            roles.insert(role, new);
        }
        let mut occurrences = std::collections::BTreeMap::new();
        for (&name, &old) in self.occurrences.get(&shape).into_iter().flatten() {
            let EntityRef::Edge(new) = corresponding(EntityRef::Edge(old))? else {
                return Err(operation_error("symmetric edge copy dimension"));
            };
            occurrences.insert(name, new);
        }
        let mut anonymous = Vec::new();
        for &old in self.anonymous_faces.get(&shape).into_iter().flatten() {
            let EntityRef::Face(new) = corresponding(EntityRef::Face(old))? else {
                return Err(operation_error("symmetric anonymous copy dimension"));
            };
            anonymous.push(new);
        }
        // Every edit is made on the independent copy. Preserve the existing
        // tolerances; false prevents the transform from choosing coarser ones.
        solidcore::topology::brep::p5::transform(
            &mut self.kernel,
            &mut output,
            &first,
            None,
            false,
        )
        .map_err(operation_error)?;
        self.check_cancelled()?;
        solidcore::topology::brep::p5::transform(
            &mut self.kernel,
            &mut output,
            &second,
            Some(scale),
            false,
        )
        .map_err(operation_error)?;
        output.refresh_cached_topology();
        for face in output.face_list().to_vec() {
            self.check_cancelled()?;
            solidcore::topology::face::p3::create_uv_trim_curves(
                &self.kernel,
                &mut output,
                face,
                false,
                None,
            )
            .map_err(operation_error)?;
        }
        let options = solidcore::query::brep::BoundingBoxOptions {
            approx: false,
            tight: true,
            expand_by_zone_tol_3d: false,
        };
        let bounds = solidcore::query::brep::calculate_bounding_box(&self.kernel, source, options)
            .map_err(operation_error)?
            .union(
                &solidcore::query::brep::calculate_bounding_box(&self.kernel, &output, options)
                    .map_err(operation_error)?,
            );
        let span = bounds.max_length();
        let size = (0..3)
            .flat_map(|i| [bounds.min_at(i).abs(), bounds.max_at(i).abs()])
            .fold(span, f64::max);
        let eps = configure_native_policy(&mut self.kernel, span, size)?;
        assert_native_tolerances(&output, eps / 2.)?;
        solidcore::topology::integrity::check(&output).map_err(operation_error)?;
        validate_native_sheet_links(&output)?;
        self.check_cancelled()?;
        let sheet = self.sheet_keys.contains(&shape);
        let wires = self.source_wires.get(&shape).copied();
        let key = self.insert(output)?;
        self.roles.insert(key, roles);
        self.occurrences.insert(key, occurrences);
        if !anonymous.is_empty() {
            self.anonymous_faces.insert(key, anonymous);
        }
        if sheet {
            self.sheet_keys.insert(key);
        }
        if let Some(wires) = wires {
            self.source_wires.insert(key, wires);
        }
        if let Some(source) = self.extrusion_sources.get(&shape).copied() {
            self.extrusion_sources.insert(key, source);
        }
        Ok(key)
    }

    fn extrude_to_face(
        &mut self,
        profile: (crate::NewellShapeKey, u32),
        direction: [f64; 3],
        start: f64,
        target: (crate::NewellShapeKey, u32),
        clearance: f64,
    ) -> Result<crate::NewellShapeKey, crate::NewellKernelError> {
        use crate::operation_error;
        self.check_cancelled()?;
        if !start.is_finite() {
            return Err(operation_error("invalid UpTo start"));
        }
        let (source_face, occurrences) = self.resolve_source_face(profile.0, profile.1)?;
        let (target_face, _) = self.resolve_source_face(target.0, target.1)?;
        let source = self
            .objects
            .get(&profile.0)
            .ok_or_else(|| operation_error("missing UpTo source"))?;
        let target = self
            .objects
            .get(&target.0)
            .ok_or_else(|| operation_error("missing UpTo target"))?;
        let (direction, plane) = native_finite_planar_end_parameters(
            &mut self.kernel,
            (source, source_face),
            (target, target_face),
            direction,
            start,
            clearance,
            self.cancellation.as_deref(),
        )?;
        check_native_cancelled(self.cancellation.as_deref())?;
        let output = native_planar_end_with_lineage(
            &mut self.kernel,
            (source, source_face),
            &occurrences,
            direction,
            start,
            plane,
            self.cancellation.as_deref(),
        )?;
        self.check_cancelled()?;
        let key = self.insert(output.body)?;
        self.roles.insert(key, output.roles);
        self.occurrences.insert(key, output.occurrence_map);
        self.extrusion_sources.insert(key, profile);
        Ok(key)
    }

    fn through_all_surface_station(
        &mut self,
        profile: (crate::NewellShapeKey, u32),
        boundary: crate::NewellBoundary,
        direction: [f64; 3],
        start: f64,
        targets: &[crate::NewellShapeKey],
    ) -> Result<f64, crate::NewellKernelError> {
        use crate::{NewellBoundary as B, operation_error};
        self.check_cancelled()?;
        let (face, occurrences) = self.resolve_source_face(profile.0, profile.1)?;
        let selected = occurrences
            .into_iter()
            .filter(|((wire, edge), _)| match boundary {
                B::Face => true,
                B::Wire(w) => *wire == w,
                B::Edge { wire: w, edge: e } => *wire == w && *edge == e,
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        if selected.is_empty() {
            return Err(operation_error("missing surface boundary occurrence"));
        }
        let source = self
            .objects
            .get(&profile.0)
            .ok_or_else(|| operation_error("missing ThroughAll source"))?;
        let keys: std::collections::BTreeSet<_> = targets.iter().copied().collect();
        let bodies = keys
            .iter()
            .map(|key| {
                self.objects
                    .get(key)
                    .ok_or_else(|| operation_error("missing ThroughAll target"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let intersection = native_generated_through_all_with_boundary(
            &mut self.kernel,
            (source, face),
            direction,
            start,
            &bodies,
            Some(&selected),
            self.cancellation.as_deref(),
        );
        let mut end = match intersection {
            Ok(end) => end,
            Err(crate::NewellKernelError::Operation { message })
                if message == "NoThroughAllHit" =>
            {
                f64::NEG_INFINITY
            }
            Err(error) => return Err(error),
        };
        // The intersection can omit contact where a sweep wall lies exactly on a
        // material boundary. Independently certify the cap's complete trim coverage.
        use crate::NewellFaceProvenance::{EndCap, StartCap};
        for key in keys {
            if self.is_surface(key) || self.extrusion_sources.get(&key) != Some(&profile) {
                continue;
            }
            for role in [StartCap, EndCap] {
                self.check_cancelled()?;
                let Some(&cap) = self.roles.get(&key).and_then(|roles| roles.get(&role)) else {
                    continue;
                };
                if let Ok(station) = native_starting_station(
                    &mut self.kernel,
                    (source, face),
                    (&self.objects[&key], cap),
                    direction,
                    self.cancellation.as_deref(),
                ) {
                    let eps = self.kernel.tolerances.xsect_tol_3d;
                    if station > start + eps {
                        end = end.max(station + (10. * eps).max(1e-6));
                    }
                }
            }
        }
        self.check_cancelled()?;
        if !end.is_finite() {
            return Err(operation_error("NoThroughAllHit"));
        }
        Ok(end)
    }

    fn through_all_station(
        &mut self,
        profile: (crate::NewellShapeKey, u32),
        direction: [f64; 3],
        start: f64,
        targets: &[crate::NewellShapeKey],
    ) -> Result<f64, crate::NewellKernelError> {
        self.check_cancelled()?;
        let (face, _) = self.resolve_source_face(profile.0, profile.1)?;
        let source = self
            .objects
            .get(&profile.0)
            .ok_or_else(|| crate::operation_error("missing ThroughAll source"))?;
        let keys: std::collections::BTreeSet<_> = targets.iter().copied().collect();
        let targets = keys
            .into_iter()
            .map(|key| {
                self.objects
                    .get(&key)
                    .ok_or_else(|| crate::operation_error("missing ThroughAll target"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        native_generated_through_all_station(
            &mut self.kernel,
            (source, face),
            direction,
            start,
            &targets,
            self.cancellation.as_deref(),
        )
    }
}

fn copy_selected_native_face(
    kernel: &mut Kernel,
    source: &solidcore::Brep,
    face: solidcore::FaceId,
) -> Result<solidcore::Brep, crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::topology::brep::{p2::ArenaNotifyState, p7::copy_faces};
    // Standing clone retains the same topology arena/handles while isolating
    // lazy source caches. copy_faces then creates independently owned geometry.
    let mut source = source.clone_standing_for_self().map_err(operation_error)?;
    let span = solidcore::query::api_queries::face_bounding_box(kernel, &source, face, true)
        .map_err(operation_error)?
        .max_length();
    let mut dest =
        solidcore::topology::brep::p3::new_brep(kernel, span, 1000.).map_err(operation_error)?;
    dest.set_tolerance(source.tolerance());
    let mut state = ArenaNotifyState::new(&source);
    state.seed(&dest);
    let mut env = state.env(None);
    let key_of = env.key_of;
    let this_face = |face| key_of(solidcore::error::EntityRef::Face(face));
    let (_, complete) = copy_faces(
        kernel,
        &mut source,
        &[face],
        &mut dest,
        true,
        &this_face,
        &mut env,
    )
    .map_err(operation_error)?;
    if !complete || dest.face_list().len() != 1 {
        return Err(operation_error("incomplete selected face copy"));
    }
    Ok(dest)
}

#[test]
fn solidcore_migration_f09_finite_filled_planar_target() -> Result<(), Box<dyn std::error::Error>> {
    use solidcore::{
        curve::{curve::Curve, line::Line},
        math::Point3,
    };
    let mut kernel = mapped_kernel(20.);
    let make = |kernel: &mut Kernel, rings: Vec<Vec<[f64; 3]>>| {
        let mut curves = Vec::new();
        for ring in rings {
            for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
                curves.push(Curve::from(
                    Line::create_line_segment(
                        3,
                        Point3::new(a[0], a[1], a[2]),
                        Point3::new(b[0], b[1], b[2]),
                        None,
                    )
                    .map_err(crate::operation_error)?,
                ));
            }
        }
        exact_planar_profile(kernel, &curves)
    };
    let (source, source_face) = make(
        &mut kernel,
        vec![vec![[0., 0., 1.], [2., 0., 1.], [2., 3., 1.], [0., 3., 1.]]],
    )?;
    let ring = |z: f64, tilt: f64| {
        vec![
            [-1., -1., z - tilt],
            [3., -1., z + 3. * tilt],
            [3., 4., z + 3. * tilt],
            [-1., 4., z - tilt],
        ]
    };
    for z in [-3., 1., 5.] {
        let (target, target_face) = make(&mut kernel, vec![ring(z, 0.)])?;
        for sign in [-1., 1.] {
            let station = native_starting_station(
                &mut kernel,
                (&source, source_face),
                (&target, target_face),
                [0., 0., sign * 2.],
                None,
            )?;
            assert!((station - sign * (z - 1.)).abs() < 1e-9);
        }
    }
    let (tilted, tilted_face) = make(&mut kernel, vec![ring(5., 0.1)])?;
    assert!(matches!(native_starting_station(
        &mut kernel, (&source, source_face), (&tilted, tilted_face), [0., 0., 1.], None,
    ), Err(crate::NewellKernelError::Operation {message}) if message == "tilted starting target"));
    for (tilt, clearance, expected) in [(0., 0., 24.), (0., 0.5, 21.), (0.1, 0., 24.6)] {
        let (target, target_face) = make(&mut kernel, vec![ring(5., tilt)])?;
        let result = native_finite_planar_end(
            &mut kernel,
            (&source, source_face),
            (&target, target_face),
            [0., 0., 1.],
            clearance,
        )?;
        let volume = corpus_volume(&kernel, &result)?;
        assert!(
            (volume - expected).abs() < 1e-6,
            "F09 volume={volume},expected={expected}"
        );
        for deflection in [0.05, 0.01] {
            let mesh = crate::solidcore_mesh::native_body(
                &kernel,
                &result,
                result.face_list(),
                deflection,
            )?;
            assert!(!mesh.indices().is_empty());
        }
    }
    let hole = vec![
        [0.75, 1., 5.],
        [0.75, 2., 5.],
        [1.25, 2., 5.],
        [1.25, 1., 5.],
    ];
    let (target, target_face) = make(&mut kernel, vec![ring(5., 0.), hole])?;
    assert!(matches!(native_starting_station(
        &mut kernel, (&source, source_face), (&target, target_face), [0., 0., 1.], None,
    ), Err(crate::NewellKernelError::Operation {message}) if message == "starting target trim coverage"));
    let error = native_finite_planar_end(
        &mut kernel,
        (&source, source_face),
        (&target, target_face),
        [0., 0., 1.],
        0.,
    );
    assert!(
        matches!(error,Err(crate::NewellKernelError::Operation{message}) if message=="target trim does not cover the entire profile")
    );
    for (z, clearance, direction) in [
        (0., 0., [0., 0., 1.]),
        (5., 4., [0., 0., 1.]),
        (5., 0., [1., 0., 0.]),
    ] {
        let (target, target_face) = make(&mut kernel, vec![ring(z, 0.)])?;
        assert!(
            native_finite_planar_end(
                &mut kernel,
                (&source, source_face),
                (&target, target_face),
                direction,
                clearance
            )
            .is_err()
        );
    }
    Ok(())
}

#[test]
fn solidcore_migration_f19_finite_up_to_scale_translation() -> Result<(), Box<dyn std::error::Error>>
{
    use solidcore::{
        curve::{curve::Curve, line::Line},
        math::Point3,
    };
    for scale in [0.1_f64, 1., 10.] {
        for origin in [[0.; 3], [1e6, -1e6, 1e6]] {
            for repeat in 0..10 {
                let mut kernel = mapped_kernel(20. * scale);
                let mut make = |ring: [[f64; 3]; 4]| {
                    let points = ring.map(|p| {
                        Point3::new(
                            origin[0] + scale * p[0],
                            origin[1] + scale * p[1],
                            origin[2] + scale * p[2],
                        )
                    });
                    let mut curves = Vec::new();
                    for i in 0..4 {
                        curves.push(Curve::from(
                            Line::create_line_segment(3, points[i], points[(i + 1) % 4], None)
                                .map_err(crate::operation_error)?,
                        ));
                    }
                    exact_planar_profile(&mut kernel, &curves)
                };
                let (source, source_face) =
                    make([[0., 0., 1.], [2., 0., 1.], [2., 3., 1.], [0., 3., 1.]])?;
                let (target, target_face) =
                    make([[-1., -1., 5.], [3., -1., 5.], [3., 4., 5.], [-1., 4., 5.]])?;
                for clearance in [0., 0.5 * scale] {
                    let body = native_finite_planar_end(
                        &mut kernel,
                        (&source, source_face),
                        (&target, target_face),
                        [0., 0., 1.],
                        clearance,
                    )?;
                    let volume = corpus_volume(&kernel, &body)?;
                    let expected = if clearance == 0. { 24. } else { 21. } * scale.powi(3);
                    assert!(
                        (volume - expected).abs() <= expected * 1e-6,
                        "F19 UpTo volume={volume},expected={expected},scale={scale},origin={origin:?},repeat={repeat}"
                    );
                    assert_native_tolerances(&body, kernel.tolerances.zone_tol_3d)?;
                }
            }
        }
    }
    Ok(())
}

#[test]
fn solidcore_migration_f10_exact_cubic_planar_end() -> Result<(), Box<dyn std::error::Error>> {
    let mut kernel = mapped_kernel(6.);
    let (profile, face) = corpus_profile(&mut kernel, 1, 1., [0.; 3])?;
    let body = native_planar_end_body(
        &mut kernel,
        &profile,
        face,
        [0., 0., 1.],
        ([-0.1, 0., 1.], 5.),
    )?;
    let volume = corpus_volume(&kernel, &body)?;
    assert!((volume - 39.66).abs() < 1e-6, "F10 exact volume {volume}");
    for deflection in [0.05, 0.01] {
        let mesh =
            crate::solidcore_mesh::native_body(&kernel, &body, body.face_list(), deflection)?;
        assert!(!mesh.indices().is_empty());
    }
    Ok(())
}

/// The native constructor returns each input curve's owner. Bind semantic input
/// occurrences through that explicit result, never endpoint resemblance.
fn construct_planar_face_owners(
    kernel: &mut Kernel,
    body: &mut solidcore::Brep,
    curves: Vec<solidcore::curve::curve::Curve>,
    tolerance: f64,
) -> Result<(solidcore::FaceId, Vec<solidcore::EdgeId>), crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::nurbs::b_spline_surface_n_lib::p2::OrientType;
    use solidcore::topology::brep::{
        p2::ArenaNotifyState,
        p7::{FaceCurvesInput, make_face_with_curve_owners},
    };
    let plane = solidcore::surface::surface::Surface::create_plane_with_3d_curves(
        kernel, &curves, tolerance,
    )
    .map_err(operation_error)?;
    if plane.outer_loop_counts.len() != 1
        || plane.outer_loop_counts[0] != plane.loop_counts.len()
        || plane.loop_counts.iter().sum::<usize>() != curves.len()
        || plane.ordered_curves.len() != curves.len()
    {
        return Err(operation_error("ambiguous constructor loop correspondence"));
    }
    let ordered = plane.ordered_curves;
    let curve_count = curves.len();
    let mut inputs: Vec<_> = curves.into_iter().map(Some).collect();
    let input = FaceCurvesInput {
        loops: plane.loop_counts,
        curves_3d: Some(
            ordered
                .iter()
                .map(|&i| {
                    inputs
                        .get_mut(i)
                        .and_then(Option::take)
                        .map(Some)
                        .ok_or_else(|| {
                            operation_error("invalid or duplicate constructor curve slot")
                        })
                })
                .collect::<Result<_, _>>()?,
        ),
        uv_curves: None,
        orientations: plane.orients,
        loop_points: vec![],
    };
    let domain = plane
        .plane
        .natural_uv_domain_with_nurb(kernel)
        .map_err(operation_error)?;
    let region = body.infinite_region();
    let mut state = ArenaNotifyState::new(body);
    let mut env = state.env(None);
    let key_of = env.key_of;
    let this_face = |face| key_of(solidcore::error::EntityRef::Face(face));
    let mut native_owners = Vec::new();
    let made = make_face_with_curve_owners(
        kernel,
        body,
        region,
        input,
        plane.plane,
        &domain,
        OrientType::Same,
        None,
        &this_face,
        &mut native_owners,
        &mut env,
    )
    .map_err(operation_error)?;
    if native_owners.len() != ordered.len() {
        return Err(operation_error("incomplete constructor owner map"));
    }
    let mut owners = vec![None; curve_count];
    for (input_slot, native) in ordered.into_iter().zip(native_owners) {
        let output = owners
            .get_mut(input_slot)
            .ok_or_else(|| operation_error("invalid constructor source slot"))?;
        if output.is_some() {
            return Err(operation_error("duplicate constructor source slot"));
        }
        *output = Some(native.ok_or_else(|| operation_error("lost constructor source curve"))?);
    }
    let owners = owners
        .into_iter()
        .map(|e| e.ok_or_else(|| operation_error("missing constructor source occurrence")))
        .collect::<Result<_, _>>()?;
    Ok((
        made.face
            .ok_or_else(|| operation_error("missing constructed face"))?,
        owners,
    ))
}

/// Explicit owners from the native face constructor, preserving every input
/// slot even when a periodic seam uses one native edge more than once.
fn construct_owned_ruled_wall(
    kernel: &mut Kernel,
    body: &mut solidcore::Brep,
    start: solidcore::nurbs::b_spline_curve::BSplineCurve,
    end: solidcore::nurbs::b_spline_curve::BSplineCurve,
    closed_source: bool,
) -> Result<(solidcore::FaceId, [solidcore::EdgeId; 4]), crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::{
        curve::{curve::Curve, line::Line},
        nurbs::b_spline_surface_n_lib::p2::OrientType,
        topology::trimming_tools::TrimInput,
    };
    let surface = solidcore::surface::surface::Surface::from(
        solidcore::sweep::api_surfaces::create_ruled_surface(&start, &end)
            .map_err(operation_error)?,
    );
    let uv_domain = surface
        .natural_uv_domain_with_nurb(kernel)
        .map_err(operation_error)?;
    let start = Curve::from(start);
    let end = Curve::from(end);
    let a = start
        .evaluate_point(kernel, start.natural_interval().min())
        .map_err(operation_error)?;
    let b = start
        .evaluate_point(kernel, start.natural_interval().max())
        .map_err(operation_error)?;
    let c = end
        .evaluate_point(kernel, end.natural_interval().min())
        .map_err(operation_error)?;
    let d = end
        .evaluate_point(kernel, end.natural_interval().max())
        .map_err(operation_error)?;
    let input = TrimInput {
        loop_counts: vec![4],
        curves: vec![
            start,
            Curve::from(Line::create_line_segment(3, b, d, None).map_err(operation_error)?),
            end,
            Curve::from(Line::create_line_segment(3, a, c, None).map_err(operation_error)?),
        ],
        orientations: vec![
            OrientType::Same,
            OrientType::Same,
            OrientType::Opposite,
            OrientType::Opposite,
        ],
        loop_points: vec![],
        surface,
        uv_domain,
        surface_orientation: OrientType::Same,
    };
    let (face, owners) = construct_surface_face_owners(kernel, body, input)?;
    let owners: [Option<solidcore::EdgeId>; 4] = owners
        .try_into()
        .map_err(|_| operation_error("incomplete ruled-wall owners"))?;
    let owner = |slot: usize, other: usize| {
        owners[slot]
            .or(if closed_source { owners[other] } else { None })
            .ok_or_else(|| operation_error("lost ruled-wall occurrence"))
    };
    Ok((
        face,
        [owner(0, 0)?, owner(1, 3)?, owner(2, 2)?, owner(3, 1)?],
    ))
}

fn constructed_sheet_sweep(
    kernel: &mut Kernel,
    source: (&solidcore::Brep, solidcore::FaceId),
    selected: &std::collections::BTreeMap<(u32, u32), solidcore::EdgeId>,
    offset: [f64; 3],
    delta: [f64; 3],
    cancellation: Option<&std::sync::atomic::AtomicBool>,
) -> Result<NativeNamedBody, crate::NewellKernelError> {
    use crate::{NewellFaceProvenance as P, operation_error};
    use solidcore::nurbs::b_spline_surface_n_lib::p2::OrientType;
    check_native_cancelled(cancellation)?;
    if selected.is_empty() || offset.iter().chain(delta.iter()).any(|v| !v.is_finite()) {
        return Err(operation_error("invalid named sheet sweep"));
    }
    let frame = native_planar_frame(kernel, source.0, source.1)?;
    let length = delta.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !length.is_finite()
        || length <= 0.
        || (0..3)
            .map(|i| frame.normal[i] * delta[i] / length)
            .sum::<f64>()
            .abs()
            <= 1e-12
    {
        return Err(operation_error("tangential or invalid sheet sweep"));
    }
    let bounds = solidcore::query::brep::calculate_bounding_box(
        &*kernel,
        source.0,
        solidcore::query::brep::BoundingBoxOptions {
            approx: false,
            tight: true,
            expand_by_zone_tol_3d: false,
        },
    )
    .map_err(operation_error)?;
    let shifted = solidcore::geom::extent3d::Extent3d::from_bounds(
        bounds.min_at(0) + offset[0],
        bounds.min_at(1) + offset[1],
        bounds.min_at(2) + offset[2],
        bounds.max_at(0) + offset[0],
        bounds.max_at(1) + offset[1],
        bounds.max_at(2) + offset[2],
    );
    let (span, eps, zone) = sweep_native_policy(kernel, source.0, &shifted, delta)?;
    assert_native_tolerances(source.0, eps / 2.)?;
    if length <= eps {
        return Err(operation_error("nonpositive sheet sweep"));
    }
    let boundary = solidcore::query::api_queries::face_get_boundary_loops(source.0, source.1)
        .map_err(operation_error)?;
    let mut body =
        solidcore::topology::brep::p3::new_brep(kernel, span, 1000.).map_err(operation_error)?;
    body.set_tolerance(zone);
    let mut roles = std::collections::BTreeMap::new();
    let mut occurrence_map = NativeOccurrences::new();
    for (&(wire, edge), &native) in selected {
        check_native_cancelled(cancellation)?;
        let positions: Vec<_> = boundary
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, &e)| (e == native).then_some(i))
            .collect();
        let [position] = positions.as_slice() else {
            return Err(operation_error("ambiguous named sheet source occurrence"));
        };
        let curve = native_occurrence_spline(
            kernel,
            source.0,
            native,
            boundary.orientations[*position] == OrientType::Opposite,
        )?;
        let start = translate_native_curve(&curve, offset)?;
        let end = translate_native_curve(&start, delta)?;
        let vertices = source.0.edge(native).map_err(operation_error)?.vertices;
        let (face, owners) =
            construct_owned_ruled_wall(kernel, &mut body, start, end, vertices[0] == vertices[1])?;
        let role = P::Wall { wire, edge };
        roles.insert(role, face);
        for (slot, native) in owners.into_iter().enumerate() {
            occurrence_map.insert(
                (role, 0, u32::try_from(slot).map_err(operation_error)?),
                native,
            );
        }
    }
    finish_constructed_sheet(kernel, body, roles, occurrence_map, eps, cancellation)
}

#[derive(Clone, Copy)]
enum SheetStart<T> {
    Station(f64),
    Face(T),
}

fn constructed_sheet_to_face(
    kernel: &mut Kernel,
    source: (&solidcore::Brep, solidcore::FaceId),
    selected: &NativeBoundary,
    direction: [f64; 3],
    start: SheetStart<(&solidcore::Brep, solidcore::FaceId, f64)>,
    target: (&solidcore::Brep, solidcore::FaceId, f64),
    cancellation: Option<&std::sync::atomic::AtomicBool>,
) -> Result<NativeNamedBody, crate::NewellKernelError> {
    use crate::{NewellFaceProvenance as P, operation_error};
    use solidcore::nurbs::b_spline_surface_n_lib::p2::OrientType;
    let (target_body, target_face, clearance) = target;
    let target = (target_body, target_face);
    check_native_cancelled(cancellation)?;
    let length = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
    if selected.is_empty()
        || matches!(start, SheetStart::Station(station) if !station.is_finite())
        || !length.is_finite()
        || length <= 1e-12
    {
        return Err(operation_error("invalid named UpTo sheet"));
    }
    let direction = direction.map(|v| v / length);
    let frame = native_planar_frame(kernel, source.0, source.1)?;
    if (0..3)
        .map(|i| frame.normal[i] * direction[i])
        .sum::<f64>()
        .abs()
        <= 1e-12
    {
        return Err(operation_error("tangential Along"));
    }
    let boundary = solidcore::query::api_queries::face_get_boundary_loops(source.0, source.1)
        .map_err(operation_error)?;
    let mut bounds = solidcore::query::brep::calculate_bounding_box(
        &*kernel,
        source.0,
        solidcore::query::brep::BoundingBoxOptions {
            approx: false,
            tight: true,
            expand_by_zone_tol_3d: false,
        },
    )
    .map_err(operation_error)?;
    let mut curves = Vec::new();
    for (&name, &native) in selected {
        check_native_cancelled(cancellation)?;
        let positions = boundary
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, &e)| (e == native).then_some(i))
            .collect::<Vec<_>>();
        let [position] = positions.as_slice() else {
            return Err(operation_error("ambiguous named UpTo source occurrence"));
        };
        let curve = native_occurrence_spline(
            kernel,
            source.0,
            native,
            boundary.orientations[*position] == OrientType::Opposite,
        )?;
        let start_curve = match start {
            SheetStart::Station(station) => {
                translate_native_curve(&curve, direction.map(|v| v * station))?
            }
            SheetStart::Face((body, face, clearance)) => native_finite_curve_planar_end_curve(
                kernel,
                &curve,
                (body, face),
                direction.map(|v| -v),
                clearance,
                false,
                cancellation,
            )?,
        };
        let end_curve = native_finite_curve_planar_end_curve(
            kernel,
            &start_curve,
            target,
            direction,
            clearance,
            true,
            cancellation,
        )?;
        for curve in [&start_curve, &end_curve] {
            let curve = solidcore::curve::curve::Curve::from(curve.clone());
            bounds = bounds.union(
                &curve
                    .calculate_tight_bounding_box(kernel, &curve.natural_interval())
                    .map_err(operation_error)?,
            );
        }
        let vertices = source.0.edge(native).map_err(operation_error)?.vertices;
        curves.push((name, start_curve, end_curve, vertices[0] == vertices[1]));
    }
    let span = bounds.max_length();
    let scale = (0..3)
        .flat_map(|i| [bounds.min_at(i).abs(), bounds.max_at(i).abs()])
        .fold(span, f64::max);
    let eps = configure_native_policy(kernel, span, scale)?;
    assert_native_tolerances(source.0, eps / 2.)?;
    let mut body =
        solidcore::topology::brep::p3::new_brep(kernel, span, 1000.).map_err(operation_error)?;
    body.set_tolerance(native_construction_zone(eps, scale)?);
    let mut roles = std::collections::BTreeMap::new();
    let mut names = NativeOccurrences::new();
    for ((wire, edge), a, b, closed) in curves {
        check_native_cancelled(cancellation)?;
        let (face, owners) = construct_owned_ruled_wall(kernel, &mut body, a, b, closed)?;
        let role = P::Wall { wire, edge };
        roles.insert(role, face);
        for (slot, native) in owners.into_iter().enumerate() {
            names.insert(
                (role, 0, u32::try_from(slot).map_err(operation_error)?),
                native,
            );
        }
    }
    finish_constructed_sheet(kernel, body, roles, names, eps, cancellation)
}

fn finish_constructed_sheet(
    kernel: &mut Kernel,
    mut body: solidcore::Brep,
    roles: std::collections::BTreeMap<crate::NewellFaceProvenance, solidcore::FaceId>,
    mut occurrence_map: NativeOccurrences,
    eps: f64,
    cancellation: Option<&std::sync::atomic::AtomicBool>,
) -> Result<NativeNamedBody, crate::NewellKernelError> {
    use crate::operation_error;
    stitch_constructed_occurrences(kernel, &mut body, &roles, &mut occurrence_map)?;
    check_native_cancelled(cancellation)?;
    body.refresh_cached_topology();
    let unique: std::collections::BTreeSet<_> = roles.values().copied().collect();
    if unique.len() != roles.len()
        || roles.len() != body.face_list().len()
        || solidcore::query::api_queries::brep_material_census(&body)
            .map_err(operation_error)?
            .0
            != 0
    {
        return Err(operation_error(
            "named sheet construction changed its face or material class",
        ));
    }
    validate_native_sheet_links(&body)?;
    for (&role, &face) in &roles {
        let boundary = solidcore::query::api_queries::face_get_boundary_loops(&body, face)
            .map_err(operation_error)?;
        let names: Vec<_> = occurrence_map
            .iter()
            .filter_map(|(&(r, _, _), &e)| (r == role).then_some(e))
            .collect();
        if names.len() != boundary.edges.len()
            || names.iter().any(|id| {
                names.iter().filter(|e| *e == id).count()
                    != boundary.edges.iter().filter(|e| *e == id).count()
            })
        {
            return Err(operation_error("lost or ambiguous named sheet occurrence"));
        }
    }
    for &face in roles.values() {
        check_native_cancelled(cancellation)?;
        solidcore::topology::face::p3::create_uv_trim_curves(kernel, &mut body, face, false, None)
            .map_err(operation_error)?;
    }
    check_native_cancelled(cancellation)?;
    assert_native_tolerances(&body, eps / 2.)?;
    Ok(NativeNamedBody {
        body,
        roles,
        occurrence_map,
    })
}

#[test]
fn solidcore_migration_named_sheet_input_occurrences_and_curved_loops()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::NewellFaceProvenance as P;
    let mut owner = MigrationGeometry::new(10.);
    let source = owner.box_shape([10., 6., 5.], [0.; 3])?;
    let source_meta = owner
        .profile_faces(source)?
        .into_iter()
        .find(|f| f.provenance == P::Base(1))
        .ok_or("source metadata")?;
    let (face, original) = owner.resolve_source_face(source, 1)?;
    let selected = original
        .iter()
        .map(|(&(w, e), &id)| ((w, (e + 1) % 4), id))
        .collect();
    let output = constructed_sheet_sweep(
        &mut owner.kernel,
        (owner.objects.get(&source).ok_or("source")?, face),
        &selected,
        [0., 0., 2.],
        [0., 0., 3.],
        None,
    )?;
    let key = owner.insert(output.body)?;
    owner.roles.insert(key, output.roles);
    owner.occurrences.insert(key, output.occurrence_map);
    owner.sheet_keys.insert(key);
    assert!((owner.surface_area(key)? - 96.).abs() < 1e-6);
    let faces = owner.profile_faces(key)?;
    assert_eq!(faces.len(), 4);
    for face in faces {
        let P::Wall { wire: 0, edge } = face.provenance else {
            return Err("unproven sheet wall".into());
        };
        let source_edge = source_meta.wires[0]
            .edges
            .iter()
            .find(|e| e.edge == (edge + 3) % 4)
            .ok_or("source edge")?;
        let bottom = face.wires[0]
            .edges
            .iter()
            .find(|e| e.edge == 0)
            .ok_or("bottom edge")?;
        let shifted = |p: [f64; 3]| [p[0], p[1], p[2] + 2.];
        assert_eq!(bottom.start, shifted(source_edge.start));
        assert_eq!(bottom.end, shifted(source_edge.end));
        assert_eq!(face.wires[0].edges.len(), 4);
    }
    let n = 16384;
    let speed = |i| {
        let t = f64::from(i) / f64::from(n);
        (36. + (9. - 24. * t + 9. * t * t).powi(2)).sqrt()
    };
    let mut length = speed(0) + speed(n);
    for i in 1..n {
        length += if i % 2 == 0 { 2. } else { 4. } * speed(i);
    }
    length /= 3. * f64::from(n);
    for (kind, area, components, cycles) in [
        (0, 120., 2, 4),
        (1, 3. * (length + 6.), 1, 2),
        (2, 12. * std::f64::consts::PI, 1, 2),
    ] {
        let (source, face) = corpus_profile(&mut owner.kernel, kind, 1., [0.; 3])?;
        let selected = initial_boundary_occurrences(&source, face)?;
        let output = constructed_sheet_sweep(
            &mut owner.kernel,
            (&source, face),
            &selected,
            [0., 0., 1.],
            [0., 0., 3.],
            None,
        )
        .map_err(|error| format!("kind={kind}: {error}"))?;
        assert_eq!(sheet_component_count(&output.body)?, components);
        assert_eq!(free_boundary_cycles(&output.body)?, cycles);
        let key = owner.insert(output.body)?;
        owner.roles.insert(key, output.roles);
        owner.occurrences.insert(key, output.occurrence_map);
        owner.sheet_keys.insert(key);
        let measured = owner.surface_area(key)?;
        assert!(
            (measured - area).abs() < 1e-6,
            "kind={kind},area={measured},expected={area}"
        );
        for deflection in [0.05, 0.01] {
            assert!(!owner.tessellate(key, deflection)?.indices().is_empty());
        }
    }
    Ok(())
}

fn construct_surface_face_owners(
    kernel: &mut Kernel,
    body: &mut solidcore::Brep,
    input: solidcore::topology::trimming_tools::TrimInput,
) -> Result<(solidcore::FaceId, Vec<Option<solidcore::EdgeId>>), crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::topology::brep::{
        p2::ArenaNotifyState,
        p7::{FaceCurvesInput, make_face_with_curve_owners},
    };
    let region = body.infinite_region();
    let mut state = ArenaNotifyState::new(body);
    let mut env = state.env(None);
    let key_of = env.key_of;
    let this_face = |face| key_of(solidcore::error::EntityRef::Face(face));
    let curves = FaceCurvesInput {
        loops: input.loop_counts,
        curves_3d: Some(input.curves.into_iter().map(Some).collect()),
        uv_curves: None,
        orientations: input.orientations,
        loop_points: input.loop_points,
    };
    let mut owners = Vec::new();
    let made = make_face_with_curve_owners(
        kernel,
        body,
        region,
        curves,
        input.surface,
        &input.uv_domain,
        input.surface_orientation,
        None,
        &this_face,
        &mut owners,
        &mut env,
    )
    .map_err(operation_error)?;
    Ok((
        made.face
            .ok_or_else(|| operation_error("missing constructed wall"))?,
        owners,
    ))
}

fn stitch_constructed_occurrences(
    kernel: &mut Kernel,
    body: &mut solidcore::Brep,
    roles: &std::collections::BTreeMap<crate::NewellFaceProvenance, solidcore::FaceId>,
    occurrences: &mut NativeOccurrences,
) -> Result<(), crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::{
        error::EntityRef,
        geom::object::NotifyOperation,
        topology::{
            brep::p3::{notify_serial, topology_ref},
            topology::TrackTopologyChanges,
        },
    };
    if kernel.context.sys_notify_callback().is_some() {
        return Err(operation_error(
            "constructor notification callback already owned",
        ));
    }
    let serial = notify_serial(&mut kernel.context, body).map_err(operation_error)?;
    let brep_id = solidcore::query::brep::BrepIdentity::of_brep(body)
        .identity()
        .brep_id;
    // Face construction uses the documented arena notification names; the
    // stitch's context-owned vertex edits use issued names. Resolve both by
    // explicit forward maps of this body's actual native handles, not by
    // decoding a guessed arena index or an enumeration position.
    let mut named_edges = std::collections::BTreeMap::new();
    {
        let proven = solidcore::query::brep::BrepIdentity::of_brep(body);
        let identity = proven.identity();
        for (edge, _) in body.edges().iter() {
            named_edges.insert(
                (identity.keys)(solidcore::topology::brep_cache::CacheObject::Edge(edge)),
                edge,
            );
            named_edges.insert(topology_ref(serial, EntityRef::Edge(edge)), edge);
        }
    }
    let mut tracker = TrackTopologyChanges::new(Some(brep_id), 1, true, Some(&mut kernel.context));
    let result =
        solidcore::topology::brep::p5::stitch_and_orient(kernel, body, &Default::default());
    tracker.stop_tracking(Some(&mut kernel.context));
    result.map_err(operation_error)?;
    if let Some(error) = tracker.failure() {
        return Err(operation_error(error));
    }
    {
        let proven = solidcore::query::brep::BrepIdentity::of_brep(body);
        let identity = proven.identity();
        for (edge, _) in body.edges().iter() {
            named_edges.insert(
                (identity.keys)(solidcore::topology::brep_cache::CacheObject::Edge(edge)),
                edge,
            );
            named_edges.insert(topology_ref(serial, EntityRef::Edge(edge)), edge);
        }
    }
    let mut merged = std::collections::BTreeMap::new();
    for event in tracker.lists().notify_events {
        if event.operation == NotifyOperation::MergeInBrep
            && let (Some(survivor), Some(deleted)) = (event.data1, event.data2)
            && let (Some(&survivor), Some(&deleted)) =
                (named_edges.get(&survivor), named_edges.get(&deleted))
            && merged
                .insert(deleted, survivor)
                .is_some_and(|previous| previous != survivor)
        {
            return Err(operation_error("ambiguous constructor merge lineage"));
        }
    }
    for (&(role, _, _), native) in occurrences.iter_mut() {
        let mut seen = std::collections::BTreeSet::new();
        while let Some(&survivor) = merged.get(native) {
            if !seen.insert(*native) {
                return Err(operation_error("cyclic constructor merge lineage"));
            }
            *native = survivor;
        }
        let face = *roles
            .get(&role)
            .ok_or_else(|| operation_error("missing constructed face role"))?;
        let boundary = solidcore::query::api_queries::face_get_boundary_loops(body, face)
            .map_err(operation_error)?;
        if !boundary.edges.contains(native) {
            println!(
                "CONSTRUCTOR_LINEAGE_MISSING,serial={serial},role={role:?},native={native:?},events={:#?}",
                tracker.lists().notify_events
            );
            return Err(operation_error(format!(
                "lost constructor edge lineage: key={:?}",
                topology_ref(serial, EntityRef::Edge(*native)).key
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
fn native_ruled_sheet(
    kernel: &mut Kernel,
    start: solidcore::nurbs::b_spline_curve::BSplineCurve,
    end: solidcore::nurbs::b_spline_curve::BSplineCurve,
) -> Result<solidcore::Brep, crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::{
        curve::{curve::Curve, line::Line},
        nurbs::b_spline_surface_n_lib::p2::OrientType,
        topology::{
            brep::p2::ArenaNotifyState,
            trimming_tools::{TrimInput, trim_surface_with_model_space_curves},
        },
    };
    let surface = solidcore::surface::surface::Surface::from(
        solidcore::sweep::api_surfaces::create_ruled_surface(&start, &end)
            .map_err(operation_error)?,
    );
    let uv_domain = surface
        .natural_uv_domain_with_nurb(kernel)
        .map_err(operation_error)?;
    let start = Curve::from(start);
    let end = Curve::from(end);
    let a = start
        .evaluate_point(kernel, start.natural_interval().min())
        .map_err(operation_error)?;
    let b = start
        .evaluate_point(kernel, start.natural_interval().max())
        .map_err(operation_error)?;
    let c = end
        .evaluate_point(kernel, end.natural_interval().min())
        .map_err(operation_error)?;
    let d = end
        .evaluate_point(kernel, end.natural_interval().max())
        .map_err(operation_error)?;
    let bounds = start
        .calculate_tight_bounding_box(kernel, &start.natural_interval())
        .map_err(operation_error)?
        .union(
            &end.calculate_tight_bounding_box(kernel, &end.natural_interval())
                .map_err(operation_error)?,
        );
    let span = bounds.max_length();
    let scale = (0..3)
        .flat_map(|i| [bounds.min_at(i).abs(), bounds.max_at(i).abs()])
        .fold(span, f64::max);
    let eps = configure_native_policy(kernel, span, scale)?;
    let zone = native_construction_zone(eps, scale)?;
    let mut body =
        solidcore::topology::brep::p3::new_brep(kernel, span, 1000.).map_err(operation_error)?;
    body.set_tolerance(zone);
    let mut curves = vec![start];
    let mut orientations = vec![OrientType::Same];
    if b != d {
        curves.push(Curve::from(
            Line::create_line_segment(3, b, d, None).map_err(operation_error)?,
        ));
        orientations.push(OrientType::Same);
    }
    curves.push(end);
    orientations.push(OrientType::Opposite);
    if a != c {
        curves.push(Curve::from(
            Line::create_line_segment(3, a, c, None).map_err(operation_error)?,
        ));
        orientations.push(OrientType::Opposite);
    }
    let input = TrimInput {
        loop_counts: vec![curves.len()],
        curves,
        orientations,
        loop_points: vec![],
        surface,
        uv_domain,
        surface_orientation: OrientType::Same,
    };
    let region = body.infinite_region();
    let mut state = ArenaNotifyState::new(&body);
    let mut env = state.env(None);
    let faces = trim_surface_with_model_space_curves(
        kernel, &mut body, region, zone, input, true, true, true, &mut env,
    )
    .map_err(operation_error)?;
    if faces.len() != 1 {
        return Err(operation_error("ambiguous ruled sheet construction"));
    }
    body.refresh_cached_topology();
    assert_native_tolerances(&body, eps / 2.)?;
    Ok(body)
}

#[test]
fn solidcore_migration_f08_exact_endpoint_pinched_sheet() -> Result<(), Box<dyn std::error::Error>>
{
    use solidcore::{
        math::Point3,
        nurbs::{b_spline_curve::BSplineCurve, crv_basic::Curve as NCurve, math::CPoint},
    };
    let mut kernel = mapped_kernel(6.);
    let start = BSplineCurve::from_nurb(
        NCurve::new(
            [[0., 0.], [2., 3.], [4., 2.], [6., 0.]]
                .map(|p| CPoint::from_point(Point3::new(p[0], p[1], 0.)))
                .to_vec(),
            3,
            vec![0., 0., 0., 0., 1., 1., 1., 1.],
        )?,
        3,
        false,
    );
    let end = lift_native_curve_to_plane(&start, [0., 0., 1.], ([0., -0.1, 1.], 0.))?;
    let mut sheet = native_ruled_sheet(&mut kernel, start, end)?;
    assert_eq!(sheet_component_count(&sheet)?, 1);
    assert_eq!(free_boundary_cycles(&sheet)?, 1);
    assert_eq!(
        solidcore::query::api_queries::brep_material_census(&sheet)?.0,
        0
    );
    let n = 16384;
    let integrand = |t: f64| {
        let y = 9. * t - 12. * t * t + 3. * t * t * t;
        (36. + (9. - 24. * t + 9. * t * t).powi(2)).sqrt() * 0.1 * y
    };
    let mut sum = integrand(0.) + integrand(1.);
    for i in 1..n {
        sum += if i % 2 == 0 { 2. } else { 4. } * integrand(f64::from(i) / f64::from(n));
    }
    let expected = sum / (3. * f64::from(n));
    let area = solidcore::mass::integrator::properties::brep_area(
        &kernel,
        &mut sheet,
        PROPERTY_RELATIVE_ACCURACY,
    )?;
    assert!(
        (area - expected).abs() < 1e-6,
        "F08 pinched area={area},expected={expected}"
    );
    println!(
        "F08_NATIVE,area={area},expected={expected},zone={},components=1,free_cycles=1,materials=0",
        sheet.tolerance()
    );
    let face = *sheet.face_list().first().ok_or("missing pinched face")?;
    let surface =
        solidcore::topology::face::p2::surface(&sheet, face)?.ok_or("missing pinched surface")?;
    let domain = surface.natural_uv_domain_with_nurb(&kernel)?;
    for (u, side, tangent_y) in [
        (domain.min().x, solidcore::nurbs::knots::Side::Right, 9.),
        (domain.max().x, solidcore::nurbs::knots::Side::Left, -6.),
    ] {
        let normal = surface.evaluate_normal(
            &kernel,
            solidcore::geom::vector2d::Point2::new(u, (domain.min().y + domain.max().y) * 0.5),
            side,
            solidcore::nurbs::knots::Side::Right,
        )?;
        assert!(normal.x.is_finite() && normal.y.is_finite() && normal.z.is_finite());
        assert!(
            (normal.x * normal.x + normal.y * normal.y + normal.z * normal.z - 1.).abs() < 1e-10
        );
        assert!(normal.z.abs() < 1e-10);
        assert!((6. * normal.x + tangent_y * normal.y).abs() < 1e-8);
    }
    for deflection in [0.05, 0.01] {
        let mesh =
            crate::solidcore_mesh::native_body(&kernel, &sheet, sheet.face_list(), deflection)?;
        assert!(!mesh.indices().is_empty());
        for (&position, &normal) in mesh.positions().iter().zip(mesh.normals()) {
            let t = f64::from(position[0]) / 6.;
            let tangent_y = 9. - 24. * t + 9. * t * t;
            assert!(f64::from(normal[2]).abs() < 1e-6);
            assert!((6. * f64::from(normal[0]) + tangent_y * f64::from(normal[1])).abs() < 1e-5);
            assert!(position[2] >= -1e-7 && position[2] <= 0.1 * position[1] + 1e-7);
        }
        for triangle in mesh.indices().as_chunks::<3>().0 {
            let [a, b, c] = triangle.map(|i| mesh.positions()[i as usize].map(f64::from));
            let u = std::array::from_fn::<_, 3, _>(|i| b[i] - a[i]);
            let v = std::array::from_fn::<_, 3, _>(|i| c[i] - a[i]);
            let geometric = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            for &index in triangle {
                assert!(
                    geometric
                        .into_iter()
                        .zip(mesh.normals()[index as usize])
                        .map(|(a, b)| a * f64::from(b))
                        .sum::<f64>()
                        > 0.
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
fn native_cubic_station_sheet(
    kernel: &mut Kernel,
    low: ([f64; 3], f64),
    high: ([f64; 3], f64),
) -> Result<solidcore::Brep, crate::NewellKernelError> {
    use solidcore::{
        math::Point3,
        nurbs::{b_spline_curve::BSplineCurve, crv_basic::Curve as NCurve, math::CPoint},
    };
    let source = BSplineCurve::from_nurb(
        NCurve::new(
            [[0., 0.], [2., 3.], [4., 2.], [6., 0.]]
                .map(|p| CPoint::from_point(Point3::new(p[0], p[1], 0.)))
                .to_vec(),
            3,
            vec![0., 0., 0., 0., 1., 1., 1., 1.],
        )
        .map_err(crate::operation_error)?,
        3,
        false,
    );
    let plane =
        |(gradient, constant): ([f64; 3], f64)| ([-gradient[0], -gradient[1], 1.], constant);
    let start = lift_native_curve_to_plane(&source, [0., 0., 1.], plane(low))?;
    let end = lift_native_curve_to_plane(&source, [0., 0., 1.], plane(high))?;
    native_ruled_sheet(kernel, start, end)
}

#[test]
fn solidcore_migration_f08_crossing_and_endpoint_boolean_geometry()
-> Result<(), Box<dyn std::error::Error>> {
    let mut kernel = mapped_kernel(6.);
    let target = native_cubic_station_sheet(&mut kernel, ([0.; 3], 0.), ([0.1, 0., 0.], 3.))?;
    let crossing = native_cubic_station_sheet(&mut kernel, ([0.; 3], 1.), ([-0.1, 0., 0.], 3.6))?;
    let pinched =
        native_cubic_station_sheet(&mut kernel, ([0., 0.1, 0.], 0.), ([0.2, 0., 0.], 4.))?;
    validate_cubic_remove_domain(
        [[0., 0., 0.], [2., 3., 0.], [4., 2., 0.], [6., 0., 0.]],
        [([0.; 3], 0.), ([0.1, 0., 0.], 3.)],
        [([0., 0.1, 0.], 0.), ([0.2, 0., 0.], 4.)],
    )?;
    let n = 16384;
    let integrate = |height: &dyn Fn(f64, f64) -> f64| {
        let at = |i| {
            let t = f64::from(i) / f64::from(n);
            let y = 9. * t - 12. * t * t + 3. * t * t * t;
            (36. + (9. - 24. * t + 9. * t * t).powi(2)).sqrt() * height(t, y)
        };
        let mut sum = at(0) + at(n);
        for i in 1..n {
            sum += if i % 2 == 0 { 2. } else { 4. } * at(i);
        }
        sum / (3. * f64::from(n))
    };
    let crossed = integrate(&|t, _| (3. + 0.6 * t).max(3.6 - 0.6 * t));
    let lens = integrate(&|_, y| 0.1 * y);
    for (mode, tool, expected) in [
        (crate::NewellBooleanMode::Add, &crossing, crossed),
        (crate::NewellBooleanMode::Remove, &pinched, lens),
    ] {
        let mut result = native_boolean_copies(&mut kernel, &target, tool, mode)?;
        let area = solidcore::mass::integrator::properties::brep_area(
            &kernel,
            &mut result,
            PROPERTY_RELATIVE_ACCURACY,
        )?;
        println!("F08_BOOLEAN,mode={mode:?},area={area},expected={expected}");
        assert!((area - expected).abs() < 1e-6);
        assert_eq!(sheet_component_count(&result)?, 1);
        assert_eq!(free_boundary_cycles(&result)?, 1);
        for deflection in [0.05, 0.01] {
            let mesh = crate::solidcore_mesh::native_body(
                &kernel,
                &result,
                result.face_list(),
                deflection,
            )?;
            assert!(!mesh.indices().is_empty());
        }
    }
    Ok(())
}

type NativeEdgeGerm = (solidcore::EdgeId, usize);
type NativeVertexLink = std::collections::BTreeMap<NativeEdgeGerm, Vec<NativeEdgeGerm>>;

type NativeStation = ([f64; 3], f64);

/// Complete curve/trim partition, using explicit modeling tolerance rather than
/// the API intersector's fixed0.001 wrapper or a midpoint-only overlap test.
fn native_curve_covered_by_planar_face(
    kernel: &Kernel,
    curve: &solidcore::curve::curve::Curve,
    target: (&solidcore::Brep, solidcore::FaceId),
) -> Result<bool, crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::{
        geom::solution_array::SolutionType,
        intersect::point_class::{PointClassType, PointClassification},
    };
    let interval = curve.natural_interval();
    let (lo, hi) = (interval.min(), interval.max());
    let mut breaks = vec![lo, hi];
    let boundary = solidcore::query::api_queries::face_get_boundary_loops(target.0, target.1)
        .map_err(operation_error)?;
    for &edge in &boundary.edges {
        let bound = solidcore::curve::curve::Curve::from(native_occurrence_spline(
            kernel, target.0, edge, false,
        )?);
        let hits = curve
            .global_curve_intersect::<solidcore::topology::ObjectHandle>(
                kernel,
                &interval,
                &bound,
                &bound.natural_interval(),
                kernel.tolerances.xsect_tol_3d,
                &Default::default(),
            )
            .map_err(operation_error)?;
        for hit in hits.as_slice() {
            match hit.solution_type {
                SolutionType::SingleValue | SolutionType::PolySingleValue => {
                    breaks.push(hit.start.param(0).map_err(operation_error)?)
                }
                SolutionType::RangeOfValues => {
                    breaks.push(hit.start.param(0).map_err(operation_error)?);
                    breaks.push(hit.end.param(0).map_err(operation_error)?);
                }
                _ => return Err(operation_error("uncertified target trim intersection")),
            }
        }
    }
    if breaks.iter().any(|t| !t.is_finite() || *t < lo || *t > hi) {
        return Err(operation_error(
            "uncertified target trim intersection parameter",
        ));
    }
    breaks.sort_by(f64::total_cmp);
    breaks.dedup_by(|a, b| (*a - *b).abs() <= 1e-10 * (hi - lo));
    let mut samples = vec![lo, hi];
    samples.extend(
        breaks
            .windows(2)
            .map(|pair| pair[0] + (pair[1] - pair[0]) * 0.5),
    );
    let surface = solidcore::topology::face::p2::surface(target.0, target.1)
        .map_err(operation_error)?
        .ok_or_else(|| operation_error("missing target surface"))?;
    let mut scratch = target
        .0
        .clone_standing_for_self()
        .map_err(operation_error)?;
    for t in samples {
        let point = curve.evaluate_point(kernel, t).map_err(operation_error)?;
        let closest = solidcore::query::api_queries::surface_closest_point(kernel, surface, point)
            .map_err(operation_error)?;
        if (closest.closest.point - point).length() > kernel.tolerances.zone_tol_3d {
            return Err(operation_error("projected curve is off target plane"));
        }
        let uv = *closest
            .uvs
            .first()
            .ok_or_else(|| operation_error("missing target UV"))?;
        let mut class = PointClassification::new(kernel.tolerances.zone_tol_3d);
        solidcore::query::face::p2::point_classify(
            kernel,
            &mut scratch,
            target.1,
            uv,
            &mut class,
            Default::default(),
        )
        .map_err(operation_error)?;
        match class.point_class() {
            PointClassType::Face | PointClassType::Edge | PointClassType::Vertex => {}
            // A successful native face classification uses Unknown for proved
            // Outside (including inside an inner loop), not for solver failure.
            // Every failed classification above still propagates its error.
            PointClassType::Unknown | PointClassType::Nothing => return Ok(false),
            _ => return Err(operation_error("uncertified target trim classification")),
        }
    }
    assert_native_tolerances(&scratch, kernel.tolerances.zone_tol_3d)?;
    Ok(true)
}

#[cfg(test)]
fn native_finite_curve_planar_end(
    kernel: &mut Kernel,
    start: solidcore::nurbs::b_spline_curve::BSplineCurve,
    target: (&solidcore::Brep, solidcore::FaceId),
    direction: [f64; 3],
    clearance: f64,
) -> Result<solidcore::Brep, crate::NewellKernelError> {
    let end = native_finite_curve_planar_end_curve(
        kernel, &start, target, direction, clearance, true, None,
    )?;
    native_ruled_sheet(kernel, start, end)
}

fn native_finite_curve_planar_end_curve(
    kernel: &mut Kernel,
    start: &solidcore::nurbs::b_spline_curve::BSplineCurve,
    target: (&solidcore::Brep, solidcore::FaceId),
    direction: [f64; 3],
    clearance: f64,
    require_forward: bool,
    cancellation: Option<&std::sync::atomic::AtomicBool>,
) -> Result<solidcore::nurbs::b_spline_curve::BSplineCurve, crate::NewellKernelError> {
    use crate::operation_error;
    check_native_cancelled(cancellation)?;
    if !clearance.is_finite() || clearance < 0. || direction.iter().any(|v| !v.is_finite()) {
        return Err(operation_error("invalid UpTo extent"));
    }
    let length = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !length.is_finite() || length <= 1e-12 {
        return Err(operation_error("invalid UpTo direction"));
    }
    let direction = direction.map(|v| v / length);
    let frame = native_planar_frame(kernel, target.0, target.1)?;
    let plane = (
        frame.normal,
        (0..3)
            .map(|i| frame.normal[i] * frame.origin[i])
            .sum::<f64>(),
    );
    let denominator = (0..3).map(|i| plane.0[i] * direction[i]).sum::<f64>();
    if denominator.abs() <= 1e-12 {
        return Err(operation_error("target parallel to sweep"));
    }
    let lifted = lift_native_curve_to_plane(start, direction, plane)?;
    let curve = solidcore::curve::curve::Curve::from(lifted);
    let bounds = curve
        .calculate_tight_bounding_box(kernel, &curve.natural_interval())
        .map_err(operation_error)?;
    let source_curve = solidcore::curve::curve::Curve::from(start.clone());
    let bounds = bounds
        .union(
            &source_curve
                .calculate_tight_bounding_box(kernel, &source_curve.natural_interval())
                .map_err(operation_error)?,
        )
        .union(
            &solidcore::query::brep::calculate_bounding_box(
                &*kernel,
                target.0,
                solidcore::query::brep::BoundingBoxOptions {
                    approx: false,
                    tight: true,
                    expand_by_zone_tol_3d: false,
                },
            )
            .map_err(operation_error)?,
        );
    let span = bounds.max_length();
    let scale = (0..3)
        .flat_map(|i| [bounds.min_at(i).abs(), bounds.max_at(i).abs()])
        .fold(span, f64::max);
    let eps = configure_native_policy(kernel, span, scale)?;
    assert_native_tolerances(target.0, eps / 2.)?;
    if require_forward {
        let minimum = -native_curve_station_max_cancellable(
            start.clone(),
            (plane.0.map(|v| v / denominator), -plane.1 / denominator),
            &|| check_native_cancelled(cancellation),
        )?;
        if !minimum.is_finite() || minimum - clearance <= eps {
            return Err(operation_error(
                "target does not lie beyond the entire start section",
            ));
        }
    }
    if !native_curve_covered_by_planar_face(kernel, &curve, target)? {
        return Err(operation_error("target trim does not cover selected chain"));
    }
    let end = lift_native_curve_to_plane(
        start,
        direction,
        (plane.0, plane.1 - clearance * denominator),
    )?;
    check_native_cancelled(cancellation)?;
    Ok(end)
}

/// Provider port of termination::curve_station_max: positive-weight exact
/// control hulls plus native knot splitting certify the affine extremum.
#[cfg(test)]
fn native_curve_station_max(
    curve: solidcore::nurbs::b_spline_curve::BSplineCurve,
    station: NativeStation,
) -> Result<f64, crate::NewellKernelError> {
    native_curve_station_max_cancellable(curve, station, &|| Ok(()))
}

fn native_curve_station_max_cancellable(
    curve: solidcore::nurbs::b_spline_curve::BSplineCurve,
    station: NativeStation,
    check_cancelled: &dyn Fn() -> Result<(), crate::NewellKernelError>,
) -> Result<f64, crate::NewellKernelError> {
    use crate::operation_error;
    check_cancelled()?;
    let project = |p: solidcore::math::Point3| {
        station.1 + station.0[0] * p.x + station.0[1] * p.y + station.0[2] * p.z
    };
    let mut work = vec![(curve, 0u32)];
    let mut best = f64::NEG_INFINITY;
    let mut certified = f64::NEG_INFINITY;
    let mut visits = 0;
    while let Some((curve, depth)) = work.pop() {
        check_cancelled()?;
        visits += 1;
        if visits > 65536 || depth > 64 {
            return Err(operation_error("curve extremum complexity"));
        }
        let nurb = curve
            .gw_nurb()
            .ok_or_else(|| operation_error("missing extremum curve"))?;
        let mut upper = f64::NEG_INFINITY;
        for p in nurb.cpts() {
            check_cancelled()?;
            let w = if nurb.is_rational() { p.w } else { 1. };
            if !w.is_finite() || w <= 0. {
                return Err(operation_error("rational curve bound"));
            }
            upper = upper.max(project(solidcore::math::Point3::new(
                p.x / w,
                p.y / w,
                p.z / w,
            )));
        }
        let (lo, hi) = nurb.param_bounds();
        let mid = lo + (hi - lo) * 0.5;
        for t in [lo, mid, hi] {
            best = best.max(project(curve.evaluate_point(t).map_err(operation_error)?));
        }
        if upper <= best + 1e-9 {
            certified = certified.max(upper);
            continue;
        }
        let (left, right) =
            solidcore::nurbs::nurbs_crv::crv_split(nurb, mid).map_err(operation_error)?;
        for part in [left, right] {
            work.push((
                solidcore::nurbs::b_spline_curve::BSplineCurve::from_nurb(part, 3, false),
                depth + 1,
            ));
        }
    }
    check_cancelled()?;
    Ok(certified.max(best))
}

fn native_projected_face(
    kernel: &mut Kernel,
    source: (&solidcore::Brep, solidcore::FaceId),
    direction: [f64; 3],
    plane: NativeStation,
) -> Result<solidcore::Brep, crate::NewellKernelError> {
    use crate::operation_error;
    let boundary = solidcore::query::api_queries::face_get_boundary_loops(source.0, source.1)
        .map_err(operation_error)?;
    let mut curves = Vec::new();
    for (&edge, &orientation) in boundary.edges.iter().zip(&boundary.orientations) {
        let curve = native_occurrence_spline(
            kernel,
            source.0,
            edge,
            orientation == solidcore::nurbs::b_spline_surface_n_lib::p2::OrientType::Opposite,
        )?;
        curves.push(solidcore::curve::curve::Curve::from(
            lift_native_curve_to_plane(&curve, direction, plane)?,
        ));
    }
    exact_planar_profile(kernel, &curves).map(|(body, _)| body)
}

#[test]
fn solidcore_migration_curve_extremum_cancellation_discards_partial_certificate()
-> Result<(), Box<dyn std::error::Error>> {
    use std::cell::Cell;
    let mut kernel = mapped_kernel(6.);
    let (source, face) = corpus_profile(&mut kernel, 1, 1., [0.; 3])?;
    let mut curves = Vec::new();
    for &edge in &solidcore::query::api_queries::face_get_boundary_loops(&source, face)?.edges {
        let curve = native_occurrence_spline(&kernel, &source, edge, false)?;
        if curve.gw_nurb().is_some_and(|data| data.degree() == 3) {
            curves.push(curve);
        }
    }
    let [curve] = curves.as_slice() else {
        return Err("missing unique cubic witness".into());
    };
    let original = [0., 0.25, 0.5, 0.75, 1.]
        .map(|t| curve.evaluate_point(t))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let polls = Cell::new(0usize);
    let maximum = native_curve_station_max_cancellable(curve.clone(), ([0., 1., 0.], 0.), &|| {
        polls.set(polls.get() + 1);
        Ok(())
    })?;
    let t = (24. - 252_f64.sqrt()) / 18.;
    let expected = 9. * t - 12. * t * t + 3. * t * t * t;
    assert!((maximum - expected).abs() <= 1e-9);
    let total = polls.get();
    assert!(total > 20);
    for stop in [1, 2, total / 2, total - 1, total] {
        polls.set(0);
        let error =
            native_curve_station_max_cancellable(curve.clone(), ([0., 1., 0.], 0.), &|| {
                polls.set(polls.get() + 1);
                if polls.get() == stop {
                    Err(crate::operation_error("geometry request cancelled"))
                } else {
                    Ok(())
                }
            });
        assert!(
            matches!(error,Err(crate::NewellKernelError::Operation{message}) if message=="geometry request cancelled")
        );
        assert_eq!(polls.get(), stop);
        let after = [0., 0.25, 0.5, 0.75, 1.]
            .map(|t| curve.evaluate_point(t))
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(after, original);
    }
    Ok(())
}

/// Bounded planar-material ThroughAll route: exact filled overlap first, then
/// affine extrema over the actual clipped curves. Curved targets retain their
/// separate generated-domain route; they are never replaced by support bounds.
#[cfg(test)]
fn native_planar_through_all_station(
    kernel: &mut Kernel,
    source: (&solidcore::Brep, solidcore::FaceId),
    direction: [f64; 3],
    start: f64,
    targets: &[&solidcore::Brep],
) -> Result<f64, crate::NewellKernelError> {
    use crate::operation_error;
    if !start.is_finite() || direction.iter().any(|v| !v.is_finite()) {
        return Err(operation_error("invalid ThroughAll extent"));
    }
    let length = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !length.is_finite() || length <= 1e-12 {
        return Err(operation_error("invalid ThroughAll direction"));
    }
    let direction = direction.map(|v| v / length);
    let frame = native_planar_frame(kernel, source.0, source.1)?;
    let source_plane = (
        frame.normal,
        (0..3)
            .map(|i| frame.normal[i] * frame.origin[i])
            .sum::<f64>(),
    );
    if (0..3)
        .map(|i| frame.normal[i] * direction[i])
        .sum::<f64>()
        .abs()
        <= 1e-12
    {
        return Err(operation_error("tangential Along"));
    }
    let options = solidcore::query::brep::BoundingBoxOptions {
        approx: false,
        tight: true,
        expand_by_zone_tol_3d: false,
    };
    let mut operation_bounds =
        solidcore::query::brep::calculate_bounding_box(&*kernel, source.0, options)
            .map_err(operation_error)?;
    for &target in targets {
        operation_bounds = operation_bounds.union(
            &solidcore::query::brep::calculate_bounding_box(&*kernel, target, options)
                .map_err(operation_error)?,
        );
    }
    let operation_span = operation_bounds.max_length();
    let operation_scale = (0..3)
        .flat_map(|i| {
            [
                operation_bounds.min_at(i).abs(),
                operation_bounds.max_at(i).abs(),
            ]
        })
        .fold(operation_span.max(start.abs()), f64::max);
    let eps = configure_native_policy(kernel, operation_span, operation_scale)?;
    assert_native_tolerances(source.0, eps / 2.)?;
    for &target in targets {
        assert_native_tolerances(target, eps / 2.)?;
    }
    let section = copy_selected_native_face(kernel, source.0, source.1)?;
    let mut farthest = f64::NEG_INFINITY;
    for &target in targets {
        if !solidcore::query::api_queries::brep_is_manifold_solid(target)
            .map_err(operation_error)?
        {
            return Err(operation_error(
                "ThroughAll target needs generated-domain route",
            ));
        }
        for &face in target.face_list() {
            let frame = native_planar_frame(kernel, target, face)?;
            let denominator = (0..3).map(|i| frame.normal[i] * direction[i]).sum::<f64>();
            if denominator.abs() <= 1e-12 {
                continue;
            }
            let projected = native_projected_face(kernel, (target, face), direction, source_plane)?;
            let outside = planar_surface_boolean_copies(
                kernel,
                &section,
                &projected,
                crate::NewellBooleanMode::Remove,
            )?;
            let overlap = if outside.face_list().is_empty() {
                independent_native_copy(kernel, &section)?
            } else {
                planar_surface_boolean_copies(
                    kernel,
                    &section,
                    &outside,
                    crate::NewellBooleanMode::Remove,
                )?
            };
            let station = (
                frame.normal.map(|n| -n / denominator),
                (0..3)
                    .map(|i| frame.normal[i] * frame.origin[i])
                    .sum::<f64>()
                    / denominator,
            );
            for &edge in overlap.edge_list() {
                farthest = farthest.max(native_curve_station_max(
                    native_occurrence_spline(kernel, &overlap, edge, false)?,
                    station,
                )?);
            }
        }
    }
    if !farthest.is_finite() || farthest <= start + eps {
        return Err(operation_error("NoThroughAllHit"));
    }
    let end = farthest + (10. * eps).max(1e-6);
    if !end.is_finite() {
        return Err(operation_error("ThroughAll bound"));
    }
    Ok(end)
}

#[test]
fn solidcore_migration_f11_planar_actual_hits_both_directions()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::NewellFaceProvenance::Base;
    let mut owner = MigrationGeometry::new(110.);
    let source = owner.box_shape([2., 3., 1.], [0.; 3])?;
    let near = owner.box_shape([2., 3., 1.], [0., 0., 5.])?;
    let far = owner.box_shape([2., 3., 1.], [0., 0., 9.])?;
    let miss = owner.box_shape([2., 3., 1.], [100., 100., 100.])?;
    let upper = owner.box_shape([2., 3., 1.], [0., 0., 15.])?;
    let partial = owner.box_shape([1., 1., 1.], [0.5, 1., 9.])?;
    let source_face = *owner
        .roles
        .get(&source)
        .and_then(|roles| roles.get(&Base(1)))
        .ok_or("source face")?;
    let upper_face = *owner
        .roles
        .get(&upper)
        .and_then(|roles| roles.get(&Base(0)))
        .ok_or("upper face")?;
    let body = |key| owner.objects.get(&key).ok_or("missing body");
    let section = body(source)?;
    let end = native_planar_through_all_station(
        &mut owner.kernel,
        (section, source_face),
        [0., 0., 1.],
        0.,
        &[section, body(near)?, body(far)?, body(miss)?],
    )?;
    assert!((end - 9.000001).abs() < 1e-9, "F11 end={end}");
    assert!(
        matches!(native_planar_through_all_station(&mut owner.kernel,(section,source_face),[0.,0.,1.],0.,&[section,body(miss)?]),Err(crate::NewellKernelError::Operation{message}) if message=="NoThroughAllHit")
    );
    let end = native_planar_through_all_station(
        &mut owner.kernel,
        (section, source_face),
        [0., 0., 1.],
        0.,
        &[body(partial)?],
    )?;
    assert!((end - 9.000001).abs() < 1e-9, "F11 partial end={end}");
    let end = native_planar_through_all_station(
        &mut owner.kernel,
        (body(upper)?, upper_face),
        [0., 0., -1.],
        0.,
        &[body(far)?],
    )?;
    assert!((end - 6.000001).abs() < 1e-9, "F11 reverse end={end}");
    Ok(())
}

#[test]
fn solidcore_migration_f10_finite_cubic_hull_overshoot() -> Result<(), Box<dyn std::error::Error>> {
    use solidcore::{
        curve::{curve::Curve, line::Line},
        math::Point3,
    };
    let mut kernel = mapped_kernel(10.);
    let (source, source_face) = corpus_profile(&mut kernel, 1, 1., [0.; 3])?;
    let points = [[-1., -1., 3.], [7., -1., 3.], [7., 4., -2.], [-1., 4., -2.]]
        .map(|p| Point3::new(p[0], p[1], p[2]));
    let mut curves = Vec::new();
    for i in 0..4 {
        curves.push(Curve::from(Line::create_line_segment(
            3,
            points[i],
            points[(i + 1) % 4],
            None,
        )?));
    }
    let (target, target_face) = exact_planar_profile(&mut kernel, &curves)?;
    // z=2-y is positive everywhere on the exact filled cubic region, although
    // its affine image has one negative-height control point. A control-hull
    // rejection would discard this supported exact planar-end construction.
    let body = native_finite_planar_end(
        &mut kernel,
        (&source, source_face),
        (&target, target_face),
        [0., 0., 1.],
        0.,
    )?;
    let volume = corpus_volume(&kernel, &body)?;
    assert!(
        (volume - 327. / 35.).abs() < 1e-6,
        "F10 hull overshoot volume={volume}"
    );
    for deflection in [0.05, 0.01] {
        let mesh =
            crate::solidcore_mesh::native_body(&kernel, &body, body.face_list(), deflection)?;
        assert!(!mesh.indices().is_empty());
    }
    Ok(())
}

#[test]
fn solidcore_migration_f10_exact_circular_and_holed_planar_ends()
-> Result<(), Box<dyn std::error::Error>> {
    for (kind, expected) in [(0, 308.8), (2, 20. * std::f64::consts::PI)] {
        let mut kernel = mapped_kernel(10.);
        let (source, source_face) = corpus_profile(&mut kernel, kind, 1., [0.; 3])?;
        let body = native_planar_end_body(
            &mut kernel,
            &source,
            source_face,
            [0., 0., 1.],
            ([-0.1, 0., 1.], 5.),
        )?;
        let volume = corpus_volume(&kernel, &body)?;
        assert!(
            (volume - expected).abs() < 1e-6,
            "F10 kind={kind},volume={volume},expected={expected}"
        );
        for deflection in [0.05, 0.01] {
            let mesh =
                crate::solidcore_mesh::native_body(&kernel, &body, body.face_list(), deflection)?;
            assert!(!mesh.indices().is_empty());
        }
    }
    Ok(())
}

#[test]
fn solidcore_migration_f10_open_cubic_finite_trim_coverage()
-> Result<(), Box<dyn std::error::Error>> {
    use solidcore::{
        curve::{curve::Curve, line::Line},
        math::Point3,
        nurbs::{b_spline_curve::BSplineCurve, crv_basic::Curve as NCurve, math::CPoint},
    };
    let mut kernel = mapped_kernel(10.);
    let start = BSplineCurve::from_nurb(
        NCurve::new(
            [[0., 0.], [2., 3.], [4., 2.], [6., 0.]]
                .map(|p| CPoint::from_point(Point3::new(p[0], p[1], 0.)))
                .to_vec(),
            3,
            vec![0., 0., 0., 0., 1., 1., 1., 1.],
        )?,
        3,
        false,
    );
    let make = |kernel: &mut Kernel, hole: Option<Vec<[f64; 2]>>| {
        let mut rings = vec![vec![[-1., -1.], [7., -1.], [7., 4.], [-1., 4.]]];
        rings.extend(hole);
        let mut curves = Vec::new();
        for ring in rings {
            let points: Vec<_> = ring
                .into_iter()
                .map(|p| Point3::new(p[0], p[1], 5. + 0.1 * p[0]))
                .collect();
            for (a, b) in points.iter().zip(points.iter().cycle().skip(1)) {
                curves.push(Curve::from(
                    Line::create_line_segment(3, *a, *b, None).map_err(crate::operation_error)?,
                ));
            }
        }
        exact_planar_profile(kernel, &curves)
    };
    let n = 16384;
    let integrand = |i| {
        let t = f64::from(i) / f64::from(n);
        (36. + (9. - 24. * t + 9. * t * t).powi(2)).sqrt() * (5. + 0.6 * t)
    };
    let mut expected = integrand(0) + integrand(n);
    for i in 1..n {
        expected += if i % 2 == 0 { 2. } else { 4. } * integrand(i);
    }
    expected /= 3. * f64::from(n);
    for hole in [
        None,
        Some(vec![[1., 0.2], [1., 0.4], [1.5, 0.4], [1.5, 0.2]]),
    ] {
        let (target, face) = make(&mut kernel, hole)?;
        let mut sheet = native_finite_curve_planar_end(
            &mut kernel,
            start.clone(),
            (&target, face),
            [0., 0., 1.],
            0.,
        )?;
        let area = solidcore::mass::integrator::properties::brep_area(
            &kernel,
            &mut sheet,
            PROPERTY_RELATIVE_ACCURACY,
        )?;
        assert!(
            (area - expected).abs() < 1e-6,
            "F10 sheet area={area},expected={expected}"
        );
        assert_eq!(sheet_component_count(&sheet)?, 1);
        assert_eq!(free_boundary_cycles(&sheet)?, 1);
        for deflection in [0.05, 0.01] {
            let mesh =
                crate::solidcore_mesh::native_body(&kernel, &sheet, sheet.face_list(), deflection)?;
            assert!(!mesh.indices().is_empty());
        }
    }
    let (target, face) = make(
        &mut kernel,
        Some(vec![[2., 1.], [2., 2.], [4., 2.], [4., 1.]]),
    )?;
    let error =
        native_finite_curve_planar_end(&mut kernel, start, (&target, face), [0., 0., 1.], 0.);
    assert!(
        matches!(error,Err(crate::NewellKernelError::Operation{message}) if message=="target trim does not cover selected chain")
    );
    Ok(())
}

#[test]
fn solidcore_migration_f11_partial_nonparallel_ruled_native_intersection()
-> Result<(), Box<dyn std::error::Error>> {
    use solidcore::{
        math::Point3,
        nurbs::{b_spline_curve::BSplineCurve, crv_basic::Curve as NCurve, math::CPoint},
    };
    let mut owner = MigrationGeometry::new(10.);
    let beam = owner.box_shape([1., 1., 10.], [2., 1., 0.])?;
    let partial = owner.box_shape([1.; 3], [2., 1., 0.])?;
    let reverse_source = owner.box_shape([1.; 3], [2., 1., 10.])?;
    let full = owner.box_shape([20., 20., 1.], [-5., -5., 0.])?;
    let miss = owner.box_shape([1.; 3], [100., 100., 0.])?;
    let make = |offset: [f64; 3]| -> Result<_, solidcore::SolidError> {
        let controls = [[0., 0., 4.], [2., 3., 7.], [4., 2., 6.], [6., 0., 4.]]
            .map(|p| {
                CPoint::from_point(Point3::new(
                    p[0] + offset[0],
                    p[1] + offset[1],
                    p[2] + offset[2],
                ))
            })
            .to_vec();
        Ok(BSplineCurve::from_nurb(
            NCurve::new(controls, 3, vec![0., 0., 0., 0., 1., 1., 1., 1.])?,
            3,
            false,
        ))
    };
    let sheet = native_ruled_sheet(&mut owner.kernel, make([0.; 3])?, make([1.2, 0., 1.6])?)?;
    let beam = independent_native_copy(&mut owner.kernel, owner.objects.get(&beam).ok_or("beam")?)?;
    let sheet_copy = independent_native_copy(&mut owner.kernel, &sheet)?;
    let clipped =
        solidcore::boolean::api_brep::boolean_intersection(&mut owner.kernel, sheet_copy, beam)?;
    assert_native_tolerances(&clipped, 5e-8)?;
    assert_eq!(sheet_component_count(&clipped)?, 1);
    let mut forward = f64::NEG_INFINITY;
    let mut reverse = f64::NEG_INFINITY;
    for &edge in clipped.edge_list() {
        let curve = native_occurrence_spline(&owner.kernel, &clipped, edge, false)?;
        forward = forward.max(native_curve_station_max(
            curve.clone(),
            ([0., 0., 1.], -1.),
        )?);
        reverse = reverse.max(native_curve_station_max(curve, ([0., 0., -1.], 10.))?);
    }
    assert!(
        (forward - 6.301).abs() < 2e-9,
        "F11 partial ruled forward={forward}"
    );
    assert!(
        (reverse - 38. / 9.).abs() < 2e-9,
        "F11 partial ruled reverse={reverse}"
    );
    for deflection in [0.05, 0.01] {
        let mesh = crate::solidcore_mesh::native_body(
            &owner.kernel,
            &clipped,
            clipped.face_list(),
            deflection,
        )?;
        assert!(!mesh.indices().is_empty());
    }
    let face = |key, role| {
        owner
            .roles
            .get(&key)
            .and_then(|r| r.get(&crate::NewellFaceProvenance::Base(role)))
            .copied()
            .ok_or("source face")
    };
    let body = |key| owner.objects.get(&key).ok_or("source body");
    let t = (24. - 252_f64.sqrt()) / 18.;
    let ymax = 9. * t - 12. * t * t + 3. * t * t * t;
    for (key, role, direction, expected) in [
        (partial, 1, [0., 0., 1.], 6.301001),
        (reverse_source, 0, [0., 0., -1.], 38. / 9. + 1e-6),
        (full, 1, [0., 0., 1.], 4.6 + ymax + 1e-6),
    ] {
        let reach = native_generated_through_all_station(
            &mut owner.kernel,
            (body(key)?, face(key, role)?),
            direction,
            0.,
            &[&sheet],
            None,
        )?;
        assert!(
            (reach - expected).abs() < 2e-9,
            "F11 generated reach={reach},expected={expected}"
        );
    }
    let (source_face, occurrences) = owner.resolve_source_face(partial, 1)?;
    for (edge, expected) in [(0, 5.600001), (1, 6.301001)] {
        let native = *occurrences.get(&(0, edge)).ok_or("selected edge")?;
        let selected = std::collections::BTreeMap::from([((0, edge), native)]);
        let reach = native_generated_through_all_with_boundary(
            &mut owner.kernel,
            (body(partial)?, source_face),
            [0., 0., 1.],
            0.,
            &[&sheet],
            Some(&selected),
            None,
        )?;
        assert!(
            (reach - expected).abs() < 2e-9,
            "F11 selected edge={edge},reach={reach},expected={expected}"
        );
    }
    assert!(
        matches!(native_generated_through_all_station(&mut owner.kernel,(body(miss)?,face(miss,1)?),[0.,0.,1.],0.,&[&sheet],None),Err(crate::NewellKernelError::Operation{message}) if message=="NoThroughAllHit")
    );
    Ok(())
}

#[test]
fn solidcore_migration_f11_partial_nonparallel_cylinder() -> Result<(), Box<dyn std::error::Error>>
{
    use solidcore::{
        curve::curve::Curve,
        nurbs::{b_spline_curve::BSplineCurve, crv_basic::Curve as NCurve, math::CPoint},
    };
    let mut owner = MigrationGeometry::new(10.);
    let source = owner.box_shape([1., 0.25, 1.], [0.5, 0.5, 0.])?;
    let reverse_source = owner.box_shape([1., 0.25, 1.], [0.5, 0.5, 10.])?;
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let controls = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
    ]
    .into_iter()
    .zip([1., w, 1., w, 1., w, 1., w, 1.])
    .map(|(p, w)| CPoint::new(0., p[0] * w, (4. + p[1]) * w, w))
    .collect();
    let circle = Curve::from(BSplineCurve::from_nurb(
        NCurve::new(
            controls,
            2,
            vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
        )?,
        3,
        false,
    ));
    let (profile, face) = exact_planar_profile(&mut owner.kernel, &[circle])?;
    let occurrences = initial_boundary_occurrences(&profile, face)?;
    let target = capped_sweep_with_lineage(
        &mut owner.kernel,
        &profile,
        face,
        &occurrences,
        [2., 0., 0.],
    )?
    .body;
    for (key, role, direction, expected) in [
        (source, 1, [0., 0., 1.], 3. + 0.75_f64.sqrt() + 1e-6),
        (
            reverse_source,
            0,
            [0., 0., -1.],
            6. + 0.75_f64.sqrt() + 1e-6,
        ),
    ] {
        let face = *owner
            .roles
            .get(&key)
            .and_then(|r| r.get(&crate::NewellFaceProvenance::Base(role)))
            .ok_or("source face")?;
        let source = owner.objects.get(&key).ok_or("source body")?;
        let reach = native_generated_through_all_station(
            &mut owner.kernel,
            (source, face),
            direction,
            0.,
            &[&target],
            None,
        )?;
        assert!(
            (reach - expected).abs() < 2e-9,
            "F11 cylinder reach={reach},expected={expected}"
        );
    }
    let (face, occurrences) = owner.resolve_source_face(source, 1)?;
    for (edge, y) in [(0, 0.5_f64), (2, 0.75)] {
        let native = *occurrences.get(&(0, edge)).ok_or("selected source edge")?;
        let selected = std::collections::BTreeMap::from([((0, edge), native)]);
        let reach = native_generated_through_all_with_boundary(
            &mut owner.kernel,
            (owner.objects.get(&source).ok_or("source body")?, face),
            [0., 0., 1.],
            0.,
            &[&target],
            Some(&selected),
            None,
        )?;
        let expected = 3. + (1. - y * y).sqrt() + 1e-6;
        assert!(
            (reach - expected).abs() < 2e-9,
            "selected y={y},reach={reach},expected={expected}"
        );
    }
    let target = owner.insert_anonymous(target, false)?;
    let next = owner.next;
    let count = owner.objects.len();
    let reach = owner.through_all_surface_station(
        (source, 1),
        crate::NewellBoundary::Edge { wire: 0, edge: 2 },
        [0., 0., 1.],
        0.,
        &[target, target],
    )?;
    assert!((reach - (3. + (1. - 0.75_f64.powi(2)).sqrt() + 1e-6)).abs() < 2e-9);
    assert_eq!(owner.next, next);
    assert_eq!(owner.objects.len(), count);
    let sheet = owner.extrude_surface(crate::NewellSurfaceExtrude {
        profile: (source, 1),
        boundary: crate::NewellBoundary::Edge { wire: 0, edge: 2 },
        direction: [0., 0., 1.],
        start: crate::NewellSurfaceStart::Distance(0.),
        extent: crate::NewellSurfaceExtent::ThroughAll(vec![target, target]),
        sides: crate::NewellSurfaceSides::One,
    })?;
    assert_eq!(sheet.len(), 1);
    let face = owner
        .profile_faces(source)?
        .into_iter()
        .find(|face| face.face == 1)
        .ok_or("source face")?;
    let edge = face.wires[0]
        .edges
        .iter()
        .find(|edge| edge.edge == 2)
        .ok_or("selected edge")?;
    let length = edge
        .start
        .into_iter()
        .zip(edge.end)
        .map(|(a, b)| (b - a).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!((owner.surface_area(sheet[0])? - length * reach).abs() < 1e-5);
    assert!(!owner.tessellate(sheet[0], 0.02)?.indices().is_empty());
    Ok(())
}

/// Actual finite beam intersection for the retained generated ruled/cylindrical
/// and planar target classes. Bounds only close the temporary exact prism;
/// actual native common geometry supplies every accepted hit and extremum.
fn native_generated_through_all_station(
    kernel: &mut Kernel,
    source: (&solidcore::Brep, solidcore::FaceId),
    direction: [f64; 3],
    start: f64,
    targets: &[&solidcore::Brep],
    cancellation: Option<&std::sync::atomic::AtomicBool>,
) -> Result<f64, crate::NewellKernelError> {
    native_generated_through_all_with_boundary(
        kernel,
        source,
        direction,
        start,
        targets,
        None,
        cancellation,
    )
}

fn native_generated_through_all_with_boundary(
    kernel: &mut Kernel,
    source: (&solidcore::Brep, solidcore::FaceId),
    direction: [f64; 3],
    start: f64,
    targets: &[&solidcore::Brep],
    boundary: Option<&std::collections::BTreeMap<(u32, u32), solidcore::EdgeId>>,
    cancellation: Option<&std::sync::atomic::AtomicBool>,
) -> Result<f64, crate::NewellKernelError> {
    use crate::operation_error;
    check_native_cancelled(cancellation)?;
    if !start.is_finite() || direction.iter().any(|v| !v.is_finite()) {
        return Err(operation_error("invalid ThroughAll extent"));
    }
    let length = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !length.is_finite() || length <= 1e-12 {
        return Err(operation_error("invalid ThroughAll direction"));
    }
    let direction = direction.map(|v| v / length);
    let frame = native_planar_frame(kernel, source.0, source.1)?;
    let denominator = (0..3).map(|i| frame.normal[i] * direction[i]).sum::<f64>();
    if denominator.abs() <= 1e-12 {
        return Err(operation_error("tangential Along"));
    }
    let h = (0..3)
        .map(|i| frame.normal[i] * frame.origin[i])
        .sum::<f64>();
    let axis = frame.normal.map(|v| v / denominator);
    let options = solidcore::query::brep::BoundingBoxOptions {
        approx: false,
        tight: true,
        expand_by_zone_tol_3d: false,
    };
    let mut operation_bounds =
        solidcore::query::brep::calculate_bounding_box(&*kernel, source.0, options)
            .map_err(operation_error)?;
    let mut bound = start;
    for &target in targets {
        check_native_cancelled(cancellation)?;
        let bounds = solidcore::query::brep::calculate_bounding_box(&*kernel, target, options)
            .map_err(operation_error)?;
        operation_bounds = operation_bounds.union(&bounds);
        for x in [bounds.min_at(0), bounds.max_at(0)] {
            for y in [bounds.min_at(1), bounds.max_at(1)] {
                for z in [bounds.min_at(2), bounds.max_at(2)] {
                    bound = bound.max(axis[0] * x + axis[1] * y + axis[2] * z - h / denominator);
                }
            }
        }
    }
    let scale = (0..3)
        .flat_map(|i| {
            [
                operation_bounds.min_at(i).abs(),
                operation_bounds.max_at(i).abs(),
            ]
        })
        .fold(operation_bounds.max_length().max(start.abs()), f64::max);
    let eps = configure_native_policy(kernel, operation_bounds.max_length(), scale)?;
    if !bound.is_finite() || bound <= start + eps {
        return Err(operation_error("NoThroughAllHit"));
    }
    let bound = bound + (bound - start).max(1.);
    if !bound.is_finite() {
        return Err(operation_error("ThroughAll bound"));
    }
    let beam = if let Some(boundary) = boundary {
        constructed_sheet_sweep(
            kernel,
            source,
            boundary,
            direction.map(|v| v * start),
            direction.map(|v| v * (bound - start)),
            cancellation,
        )?
        .body
    } else {
        let starter = native_projected_face(
            kernel,
            source,
            direction,
            (frame.normal, h + denominator * start),
        )?;
        let [start_face] = starter.face_list() else {
            return Err(operation_error("ambiguous ThroughAll source cap"));
        };
        native_planar_end_body(
            kernel,
            &starter,
            *start_face,
            direction,
            (frame.normal, h + denominator * bound),
        )?
    };
    let operation_bounds = operation_bounds.union(
        &solidcore::query::brep::calculate_bounding_box(&*kernel, &beam, options)
            .map_err(operation_error)?,
    );
    let scale = (0..3)
        .flat_map(|i| {
            [
                operation_bounds.min_at(i).abs(),
                operation_bounds.max_at(i).abs(),
            ]
        })
        .fold(operation_bounds.max_length(), f64::max);
    let eps = configure_native_policy(kernel, operation_bounds.max_length(), scale)?;
    let station = (axis, -h / denominator);
    let mut farthest = f64::NEG_INFINITY;
    for &target in targets {
        check_native_cancelled(cancellation)?;
        let common = native_operation_copies(
            kernel,
            target,
            &beam,
            solidcore::boolean::api_brep::boolean_intersection,
        )?;
        check_native_cancelled(cancellation)?;
        if common.edge_list().is_empty() && !common.face_list().is_empty() {
            return Err(operation_error(
                "ThroughAll target lacks a certified boundary extremum",
            ));
        }
        for &edge in common.edge_list() {
            check_native_cancelled(cancellation)?;
            farthest = farthest.max(native_curve_station_max_cancellable(
                native_occurrence_spline(kernel, &common, edge, false)?,
                station,
                &|| check_native_cancelled(cancellation),
            )?);
        }
    }
    check_native_cancelled(cancellation)?;
    if !farthest.is_finite() || farthest <= start + eps {
        return Err(operation_error("NoThroughAllHit"));
    }
    let end = farthest + (10. * eps).max(1e-6);
    if !end.is_finite() {
        return Err(operation_error("ThroughAll bound"));
    }
    Ok(end)
}

/// Exact common-source cubic domain preflight. The caller supplies construction
/// correspondence, never inferred coincidence of independently created curves.
/// Retained bands which disappear at an interior contact and resume on both
/// sides have disconnected vertex fans. Endpoint collapses remain valid disks.
#[cfg(test)]
fn station_bernstein_roots(values: &[f64; 4]) -> Vec<f64> {
    let [p0, p1, p2, p3] = *values;
    let coefficients = [
        p0,
        3. * (p1 - p0),
        3. * (p2 - 2. * p1 + p0),
        p3 - 3. * p2 + 3. * p1 - p0,
    ];
    let evaluate = |t: f64| {
        coefficients[0] + t * (coefficients[1] + t * (coefficients[2] + t * coefficients[3]))
    };
    let mut knots = vec![0., 1.];
    let [_, c, b, a] = coefficients;
    let discriminant = b * b - 3. * a * c;
    if a != 0. && discriminant >= 0. {
        for root in [
            (-b - discriminant.sqrt()) / (3. * a),
            (-b + discriminant.sqrt()) / (3. * a),
        ] {
            if root > 0. && root < 1. {
                knots.push(root);
            }
        }
    } else if b != 0. {
        let root = -c / (2. * b);
        if root > 0. && root < 1. {
            knots.push(root);
        }
    }
    knots.sort_by(f64::total_cmp);
    let mut roots: Vec<f64> = knots
        .iter()
        .copied()
        .filter(|&t| evaluate(t).abs() < 1e-12)
        .collect();
    for pair in knots.windows(2) {
        let (mut low, mut high) = (pair[0], pair[1]);
        let left = evaluate(low);
        if left.abs() < 1e-12
            || evaluate(high).abs() < 1e-12
            || left.signum() == evaluate(high).signum()
        {
            continue;
        }
        for _ in 0..60 {
            let mid = (low + high) * 0.5;
            if evaluate(mid).signum() == left.signum() {
                low = mid;
            } else {
                high = mid;
            }
        }
        roots.push((low + high) * 0.5);
    }
    roots.retain(|&t| t > 0. && t < 1.);
    roots
}

#[cfg(test)]
fn validate_cubic_remove_domain(
    controls: [[f64; 3]; 4],
    target: [NativeStation; 2],
    tool: [NativeStation; 2],
) -> Result<(), crate::NewellKernelError> {
    let stations = [target[0], target[1], tool[0], tool[1]];
    if controls.iter().flatten().any(|v| !v.is_finite())
        || stations
            .iter()
            .any(|(g, c)| !c.is_finite() || g.iter().any(|v| !v.is_finite()))
    {
        return Err(crate::operation_error("invalid cubic station domain"));
    }
    let coefficients =
        stations.map(|(g, c)| controls.map(|p| c + (0..3).map(|i| g[i] * p[i]).sum::<f64>()));
    let value = |c: [f64; 4], t: f64| {
        let s = 1. - t;
        s * s * s * c[0] + 3. * s * s * t * c[1] + 3. * s * t * t * c[2] + t * t * t * c[3]
    };
    let mut breaks = vec![0., 1.];
    for i in 0..4 {
        for j in i + 1..4 {
            let diff = std::array::from_fn::<_, 4, _>(|k| coefficients[i][k] - coefficients[j][k]);
            breaks.extend(station_bernstein_roots(&diff));
        }
    }
    breaks.sort_by(f64::total_cmp);
    breaks.dedup_by(|a, b| (*a - *b).abs() < 1e-11);
    let retained = |t| {
        let [a, b, c, d] = coefficients.map(|p| value(p, t));
        (b.min(c) - a).max(0.) + (b - a.max(d)).max(0.)
    };
    for triple in breaks.windows(3) {
        let [left, contact, right] = [triple[0], triple[1], triple[2]];
        if retained(contact) <= 1e-10
            && retained((left + contact) * 0.5) > 1e-10
            && retained((contact + right) * 0.5) > 1e-10
        {
            return Err(crate::operation_error("nonmanifold pinched sheet vertex"));
        }
    }
    Ok(())
}

/// Oriented native incidence and vertex-link validation, independent of display
/// triangles. A boundary fan is one path; an interior fan is one cycle.
fn validate_native_sheet_links(body: &solidcore::Brep) -> Result<(), crate::NewellKernelError> {
    use crate::operation_error;
    use solidcore::nurbs::b_spline_surface_n_lib::p2::OrientType;
    use std::collections::{BTreeMap, BTreeSet};
    let mut uses: BTreeMap<solidcore::EdgeId, Vec<OrientType>> = BTreeMap::new();
    let mut links: BTreeMap<solidcore::VertexId, NativeVertexLink> = BTreeMap::new();
    for &face in body.face_list() {
        let boundary = solidcore::query::api_queries::face_get_boundary_loops(body, face)
            .map_err(operation_error)?;
        let mut offset = 0;
        for count in boundary.edge_counts {
            if count == 0 {
                return Err(operation_error("empty sheet boundary"));
            }
            let mut occurrences = Vec::new();
            for i in offset..offset + count {
                let edge = *boundary
                    .edges
                    .get(i)
                    .ok_or_else(|| operation_error("sheet edge occurrence"))?;
                let orientation = *boundary
                    .orientations
                    .get(i)
                    .ok_or_else(|| operation_error("sheet edge orientation"))?;
                uses.entry(edge).or_default().push(orientation);
                let endpoints = body.edge(edge).map_err(operation_error)?.vertices;
                let (start, end) = if orientation == OrientType::Same {
                    (0, 1)
                } else {
                    (1, 0)
                };
                occurrences.push((edge, (endpoints[start], start), (endpoints[end], end)));
            }
            for (incoming, outgoing) in occurrences.iter().zip(occurrences.iter().cycle().skip(1)) {
                if incoming.2.0 != outgoing.1.0 {
                    return Err(operation_error("sheet boundary incidence is not closed"));
                }
                let vertex = incoming.2.0;
                let a = (incoming.0, incoming.2.1);
                let b = (outgoing.0, outgoing.1.1);
                if a == b {
                    return Err(operation_error("degenerate sheet vertex link"));
                }
                let graph = links.entry(vertex).or_default();
                graph.entry(a).or_default().push(b);
                graph.entry(b).or_default().push(a);
            }
            offset += count;
        }
    }
    for orientations in uses.values() {
        if orientations.len() > 2 || orientations.len() == 2 && orientations[0] == orientations[1] {
            return Err(operation_error("nonmanifold sheet edge incidence"));
        }
    }
    for graph in links.values() {
        let Some(&first) = graph.keys().next() else {
            return Err(operation_error("empty sheet vertex link"));
        };
        let mut reached = BTreeSet::new();
        let mut work = vec![first];
        while let Some(node) = work.pop() {
            if reached.insert(node) {
                work.extend(
                    graph
                        .get(&node)
                        .ok_or_else(|| operation_error("missing sheet vertex link"))?
                        .iter()
                        .copied(),
                );
            }
        }
        let ends = graph
            .values()
            .filter(|adjacent| adjacent.len() == 1)
            .count();
        if reached.len() != graph.len()
            || !matches!(ends, 0 | 2)
            || graph
                .values()
                .any(|adjacent| adjacent.is_empty() || adjacent.len() > 2)
        {
            return Err(operation_error("nonmanifold pinched sheet vertex"));
        }
    }
    Ok(())
}

#[test]
fn solidcore_migration_f08_interior_pinch_rejects_exact_topology()
-> Result<(), Box<dyn std::error::Error>> {
    let mut kernel = mapped_kernel(6.);
    let target = native_cubic_station_sheet(&mut kernel, ([0.; 3], 0.), ([0.1, 0., 0.], 3.))?;
    let t = (24. - 252_f64.sqrt()) / 18.;
    let ymax = 9. * t - 12. * t * t + 3. * t * t * t;
    let tool = native_cubic_station_sheet(&mut kernel, ([0., -1., 0.], ymax), ([0.2, 0., 0.], 4.))?;
    let before = solidcore::mass::integrator::properties::brep_area(
        &kernel,
        &mut target.clone_standing_for_self()?,
        PROPERTY_RELATIVE_ACCURACY,
    )?;
    let outcome = validate_cubic_remove_domain(
        [[0., 0., 0.], [2., 3., 0.], [4., 2., 0.], [6., 0., 0.]],
        [([0.; 3], 0.), ([0.1, 0., 0.], 3.)],
        [([0., -1., 0.], ymax), ([0.2, 0., 0.], 4.)],
    )
    .and_then(|()| {
        native_boolean_copies(
            &mut kernel,
            &target,
            &tool,
            crate::NewellBooleanMode::Remove,
        )
    });
    assert!(
        matches!(outcome,Err(crate::NewellKernelError::Operation {message}) if message=="nonmanifold pinched sheet vertex")
    );
    let after = solidcore::mass::integrator::properties::brep_area(
        &kernel,
        &mut target.clone_standing_for_self()?,
        PROPERTY_RELATIVE_ACCURACY,
    )?;
    assert_eq!(before, after);
    Ok(())
}
