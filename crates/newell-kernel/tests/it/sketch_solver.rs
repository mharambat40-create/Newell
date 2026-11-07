// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M5.0: independently known ranks and the singular-mobility counterexample.
use faer::Mat;
#[test]
fn tangent_circle_nullity_does_not_prove_finite_mobility() -> Result<(), Box<dyn std::error::Error>>
{
    let jacobian = Mat::from_fn(2, 2, |i, j| {
        if j == 0 {
            if i == 0 { 2. } else { -2. }
        } else {
            0.
        }
    });
    let values = jacobian
        .singular_values()
        .map_err(|e| format!("SVD: {e:?}"))?;
    assert_eq!(values.iter().filter(|&&v| v > 1e-10).count(), 1);
    // (x+1)^2+y^2=1 and (x-1)^2+y^2=1 imply x=y=0.
    // Subtract to get x=0; substitute to get y^2=0. Nullity is one,
    // but the real solution is isolated. A solver success flag cannot certify DOF.
    for y in [0.001_f64, -0.001] {
        assert!(((1. + y * y) - 1.).abs() > 1e-7);
    }
    Ok(())
}
#[test]
fn duplicated_equations_and_rigid_freedoms_have_analytic_rank()
-> Result<(), Box<dyn std::error::Error>> {
    let duplicate = Mat::from_fn(3, 2, |i, j| {
        if (i < 2 && j == 0) || (i == 2 && j == 1) {
            1.
        } else {
            0.
        }
    });
    assert_eq!(
        duplicate
            .singular_values()
            .map_err(|e| format!("SVD: {e:?}"))?
            .iter()
            .filter(|&&v| v > 1e-10)
            .count(),
        2
    );
    let segment = Mat::from_fn(1, 4, |_, j| match j {
        0 => -1.,
        2 => 1.,
        _ => 0.,
    });
    assert_eq!(
        segment
            .singular_values()
            .map_err(|e| format!("SVD: {e:?}"))?
            .iter()
            .filter(|&&v| v > 1e-10)
            .count(),
        1
    );
    Ok(())
}

use faer::{
    Par,
    prelude::SolveLstsq,
    sparse::{
        SparseColMat, Triplet,
        linalg::solvers::{Qr, SymbolicQr},
    },
};
use std::{
    hint::black_box,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Instant,
};
pub(crate) type ProbeResult = Result<(), Box<dyn std::error::Error>>;
pub(crate) fn distribution(name: &str, mut us: Vec<f64>) {
    us.sort_by(f64::total_cmp);
    let n = us.len();
    if n == 0 {
        return;
    }
    let at = |p: usize| us[(n * p).div_ceil(100).saturating_sub(1).min(n - 1)];
    println!(
        "LATENCY,{name},n={n},p50_us={:.3},p95_us={:.3},p99_us={:.3},max_us={:.3}",
        at(50),
        at(95),
        at(99),
        us[n - 1]
    );
}
pub(crate) fn dense_cases(iterations: usize) -> ProbeResult {
    let cases: Vec<(&str, Mat<f64>, usize)> = vec![
        ("rigid_segment", faer::mat![[-2., 0., 2., 0.]], 1),
        ("duplicated_equations", faer::mat![[1., -1.], [2., -2.]], 1),
        ("toggle", faer::mat![[1., 0.], [0., 0.]], 1),
        ("tangent_fixed_circles", faer::mat![[2., 0.], [-2., 0.]], 1),
        ("active_parameter_bound", faer::mat![[1., 0.]], 1),
        ("regular_grounded", faer::mat![[1., 0.], [0., 1.]], 2),
    ];
    for (name, a, rank) in cases {
        let mut times = Vec::new();
        black_box(a.singular_values().map_err(|e| format!("SVD: {e:?}"))?);
        for _ in 0..iterations {
            let start = Instant::now();
            let values = a.singular_values().map_err(|e| format!("SVD: {e:?}"))?;
            times.push(start.elapsed().as_secs_f64() * 1e6);
            let largest = values.iter().copied().fold(0., f64::max);
            for factor in [0.1, 1., 10.] {
                assert_eq!(
                    values
                        .iter()
                        .filter(|v| **v > factor * 1e-10_f64.max(1e-9 * largest))
                        .count(),
                    rank
                );
            }
            black_box(values);
        }
        println!(
            "RANK,{name},rows={},cols={},rank={rank},linearized_nullity={},mobility={}",
            a.nrows(),
            a.ncols(),
            a.ncols() - rank,
            if name == "tangent_fixed_circles" || name == "toggle" {
                "Singular_NotCertified"
            } else if name == "active_parameter_bound" {
                "Boundary_NotCertified"
            } else {
                "Regular_analytic_fixture"
            }
        );
        distribution(&format!("dense_svd_{name}"), times);
    }
    let bounded = Mat::from_fn(256, 256, |i, j| {
        if i == j {
            2.
        } else if i.abs_diff(j) == 1 {
            -1.
        } else {
            0.
        }
    });
    black_box(
        bounded
            .singular_values()
            .map_err(|e| format!("SVD: {e:?}"))?,
    );
    let mut times = Vec::new();
    for _ in 0..iterations {
        let t = Instant::now();
        let values = bounded
            .singular_values()
            .map_err(|e| format!("SVD: {e:?}"))?;
        assert_eq!(
            values.iter().filter(|&&v| v > 1e-9 * values[0]).count(),
            256
        );
        times.push(t.elapsed().as_secs_f64() * 1e6);
    }
    distribution("dense_svd_256", times);
    for scale in [1e-9, 1., 1e9] {
        let a = faer::mat![[scale, 0.], [0., scale * 1e-10]];
        let values = a.singular_values().map_err(|e| format!("SVD: {e:?}"))?;
        let ranks: Vec<_> = [0.1, 1., 10.]
            .iter()
            .map(|f| {
                values
                    .iter()
                    .filter(|v| **v > f * 1e-10_f64.max(1e-9 * values[0]))
                    .count()
            })
            .collect();
        println!(
            "THRESHOLD,scale={scale:e},singular_values={values:?},ranks={ranks:?},classification=Uncertain"
        );
    }
    Ok(())
}
type Sparse = SparseColMat<usize, f64>;
pub(crate) fn fixture(
    n: usize,
    grid: bool,
) -> Result<(Sparse, Mat<f64>, usize), Box<dyn std::error::Error>> {
    let width = if grid { n.isqrt() } else { n };
    let mut entries = Vec::new();
    let mut rhs = Vec::new();
    entries.push(Triplet::new(0, 0, 1.));
    rhs.push(0.);
    for i in 0..n {
        if i % width + 1 < width && i + 1 < n {
            let r = rhs.len();
            entries.push(Triplet::new(r, i, -1.));
            entries.push(Triplet::new(r, i + 1, 1.));
            rhs.push(0.01);
        }
        if grid && i + width < n {
            let r = rhs.len();
            entries.push(Triplet::new(r, i, -1.));
            entries.push(Triplet::new(r, i + width, 1.));
            rhs.push(width as f64 * 0.01);
        }
    }
    let nnz = entries.len();
    let rows = rhs.len();
    Ok((
        SparseColMat::try_new_from_triplets(rows, n, &entries)?,
        Mat::from_fn(rows, 1, |i, _| rhs[i]),
        nnz,
    ))
}
pub(crate) fn sparse_cases(iterations: usize, large: bool) -> ProbeResult {
    faer::set_global_parallelism(Par::Seq);
    let cases = if large {
        vec![
            ("S1_component", 128, false),
            ("S2_component", 256, false),
            ("C1_chain", 10_000, false),
            ("C1_grid", 10_000, true),
        ]
    } else {
        vec![("small_chain", 32, false)]
    };
    for (name, n, grid) in cases {
        let setup = Instant::now();
        let (a, b, nnz) = fixture(n, grid)?;
        println!(
            "FIXTURE,{name},unknowns={n},equations={},nnz={nnz},setup_us={:.3},input_csc_lower_bound_bytes={}",
            a.nrows(),
            setup.elapsed().as_secs_f64() * 1e6,
            nnz * 16 + (n + 1) * 8 + b.nrows() * 8
        );
        let symbolic_workspace =
            faer::sparse::linalg::qr::factorize_symbolic_qr(a.symbolic(), Default::default())?;
        let scratch =
            symbolic_workspace.factorize_numeric_qr_scratch::<f64>(Par::Seq, Default::default());
        println!(
            "WORKSPACE,{name},numeric_values_bytes={},numeric_indices_bytes={},factor_scratch_bytes={},solve_scratch_bytes={},symbolic_heap_bytes=Unmeasured",
            symbolic_workspace.len_val() * 8,
            symbolic_workspace.len_idx() * 8,
            scratch.size_bytes(),
            symbolic_workspace
                .solve_in_place_scratch::<f64>(1, Par::Seq)
                .size_bytes()
        );
        drop(symbolic_workspace);
        let mut symbols = Vec::new();
        let mut numerics = Vec::new();
        let mut solves = Vec::new();
        let mut cleanups = Vec::new();
        let mut end = Vec::new();
        let mut max_error = 0_f64;
        for k in 0..=iterations {
            let total = Instant::now();
            let t = Instant::now();
            let symbolic = SymbolicQr::try_new(a.symbolic())?;
            let sym = t.elapsed().as_secs_f64() * 1e6;
            let t = Instant::now();
            let qr = Qr::try_new_with_symbolic(symbolic, a.as_ref())?;
            let num = t.elapsed().as_secs_f64() * 1e6;
            let t = Instant::now();
            let x = qr.solve_lstsq(&b);
            let sol = t.elapsed().as_secs_f64() * 1e6;
            for i in 0..n {
                max_error = max_error.max((x[(i, 0)] - i as f64 * 0.01).abs());
            }
            assert!(max_error < 1e-6, "sparse solution error {max_error}");
            let t = Instant::now();
            drop(qr);
            drop(x);
            let cleanup = t.elapsed().as_secs_f64() * 1e6;
            if k > 0 {
                symbols.push(sym);
                numerics.push(num);
                solves.push(sol);
                cleanups.push(cleanup);
                end.push(total.elapsed().as_secs_f64() * 1e6);
            }
        }
        println!(
            "SOLVE,{name},analytic_solution_max_error={max_error:e},generic_large_numerical_rank=Unavailable"
        );
        for (phase, t) in [
            ("symbolic", symbols),
            ("factor", numerics),
            ("solve", solves),
            ("cleanup", cleanups),
            ("backend_total", end),
        ] {
            distribution(&format!("{name}_{phase}"), t);
        }
    }
    Ok(())
}
/// A backend-boundary probe, not production Sketch scheduler acceptance.
/// Cancel is sent after entry notification; in_flight distinguishes short-call races.
pub(crate) fn cancel_boundary<F>(name: &str, operation: F) -> ProbeResult
where
    F: FnOnce() + Send,
{
    let cancelled = AtomicBool::new(false);
    let (started_tx, started_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    std::thread::scope(|scope| -> ProbeResult {
        let cancel_ref = &cancelled;
        scope.spawn(move || {
            let call = Instant::now();
            started_tx.send(call).ok();
            operation();
            let indivisible = call.elapsed();
            let released = Instant::now();
            black_box(cancel_ref.load(Ordering::Acquire));
            black_box(faer::mat![[1_f64, 0.], [0., 1.]].singular_values()).ok();
            let click_done = Instant::now();
            black_box(faer::mat![[2_f64, 0.], [0., 3.]].singular_values()).ok();
            done_tx
                .send((indivisible, released, click_done, Instant::now()))
                .ok();
        });
        let began = started_rx.recv()?;
        let requested = Instant::now();
        cancelled.store(true, Ordering::Release);
        let ack = requested.elapsed();
        let (call, release, click, other) = done_rx.recv()?;
        println!(
            "CANCEL,{name},atomic_latch_us={:.3},indivisible_us={:.3},worker_release_us={:.3},frozen_click_surrogate_us={:.3},other_window_surrogate_us={:.3},in_flight={}",
            ack.as_secs_f64() * 1e6,
            call.as_secs_f64() * 1e6,
            release.duration_since(requested).as_secs_f64() * 1e6,
            click.duration_since(requested).as_secs_f64() * 1e6,
            other.duration_since(requested).as_secs_f64() * 1e6,
            began <= requested && requested <= release
        );
        Ok(())
    })
}
pub(crate) fn cancellation() -> ProbeResult {
    let (chain, _, _) = fixture(10_000, false)?;
    let chain_symbolic = SymbolicQr::try_new(chain.symbolic())?;
    cancel_boundary("sparse_numeric_C1_chain", || {
        black_box(Qr::try_new_with_symbolic(chain_symbolic, chain.as_ref())).ok();
    })?;
    let (a, b, _) = fixture(10_000, true)?;
    cancel_boundary("sparse_symbolic_C1_grid", || {
        black_box(SymbolicQr::try_new(a.symbolic())).ok();
    })?;
    let symbolic = SymbolicQr::try_new(a.symbolic())?;
    cancel_boundary("sparse_numeric_C1_grid", || {
        black_box(Qr::try_new_with_symbolic(symbolic, a.as_ref())).ok();
    })?;
    let qr = a.sp_qr()?;
    cancel_boundary("sparse_solve_C1_grid", || {
        black_box(qr.solve_lstsq(&b));
    })?;
    let dense = Mat::from_fn(256, 256, |i, j| {
        if i == j {
            2.
        } else if i.abs_diff(j) == 1 {
            -1.
        } else {
            0.
        }
    });
    cancel_boundary("dense_rank_256", || {
        black_box(dense.singular_values()).ok();
    })?;
    Ok(())
}
#[test]
fn analytic_rank_distinguishes_singular_and_boundary_cases() -> ProbeResult {
    dense_cases(1)
}
#[test]
fn sparse_backend_solves_independent_analytic_chain() -> ProbeResult {
    sparse_cases(1, false)
}

#[test]
fn pinned_backend_cancellation_boundaries_are_measurable() -> ProbeResult {
    cancellation()
}

pub(crate) fn spatial(iterations: usize) -> ProbeResult {
    for count in [1000, 10_000, 100_000] {
        let points: Vec<[f64; 2]> = (0..count).map(|i| [i as f64, 0.]).collect();
        let t = Instant::now();
        let mut tree = rstar::RTree::bulk_load(points);
        println!(
            "INDEX,count={count},setup_us={:.3},input_payload_bytes={}",
            t.elapsed().as_secs_f64() * 1e6,
            count * 16
        );
        let mut times = Vec::new();
        for i in 0..iterations {
            let x = (i * 97 % count) as f64;
            let t = Instant::now();
            let query = rstar::AABB::from_corners([x - 0.1, -1.], [x + 0.1, 1.]);
            let found = tree.locate_in_envelope_intersecting(&query).count();
            assert_eq!(found, 1);
            times.push(t.elapsed().as_secs_f64() * 1e6);
        }
        let t = Instant::now();
        assert!(tree.remove(&[0., 0.]).is_some());
        tree.insert([0., 0.5]);
        println!(
            "INDEX_EDIT,count={count},remove_insert_us={:.3},explicit_entries_changed=1,internal_visits_and_allocations=Unmeasured",
            t.elapsed().as_secs_f64() * 1e6
        );
        distribution(&format!("rstar_query_{count}"), times);
    }
    Ok(())
}
#[test]
fn spatial_fixture_queries_match_known_points() -> ProbeResult {
    spatial(1)
}

pub(crate) fn scale_workloads(iterations: usize) -> ProbeResult {
    // Complete class-sized linear surrogate. No model/UI integration is implied.
    for (name, entities, unknowns, block_size) in [
        ("S1", 1000, 3000, 128),
        ("S2", 10_000, 30_000, 256),
        ("S3", 100_000, 200_000, 256),
    ] {
        let setup = Instant::now();
        let mut blocks = Vec::new();
        let mut left = unknowns;
        while left > 0 {
            let count = left.min(block_size);
            blocks.push(fixture(count, false)?);
            left -= count;
        }
        let nnz: usize = blocks.iter().map(|b| b.2).sum();
        println!(
            "SCALE_FIXTURE,{name},entities={entities},unknowns={unknowns},equations={unknowns},largest_block={block_size},nnz={nnz},blocks={},setup_us={:.3},scope=linear_backend_surrogate",
            blocks.len(),
            setup.elapsed().as_secs_f64() * 1e6
        );
        let t = Instant::now();
        let mut max_error = 0_f64;
        for (a, b, _) in &blocks {
            let qr = a.sp_qr()?;
            let x = qr.solve_lstsq(b);
            for i in 0..a.ncols() {
                max_error = max_error.max((x[(i, 0)] - i as f64 * 0.01).abs());
            }
        }
        assert!(max_error < 1e-7);
        println!(
            "SCALE_FINAL,{name},all_blocks_us={:.3},max_solution_error={max_error:e},integrated_final_validation=Unproven",
            t.elapsed().as_secs_f64() * 1e6
        );
        let (a, b, _) = &mut blocks[0];
        let symbolic = SymbolicQr::try_new(a.symbolic())?;
        let mut times = Vec::new();
        for i in 0..iterations {
            let t = Instant::now();
            b[(0, 0)] = i as f64 * 1e-6;
            let qr = Qr::try_new_with_symbolic(symbolic.clone(), a.as_ref())?;
            let x = qr.solve_lstsq(&*b);
            assert!((x[(0, 0)] - b[(0, 0)]).abs() < 1e-7);
            times.push(t.elapsed().as_secs_f64() * 1e6);
        }
        println!(
            "SCALE_COUNTER,{name},explicit_components_evaluated_per_local_edit=1,other_components_evaluated=0,full_path_copy_allocations=Unmeasured,old_published_generation=NotIntegrated"
        );
        distribution(&format!("{name}_linear_component_edit"), times);
    }
    Ok(())
}
#[test]
fn full_class_sized_linear_surrogates_preserve_known_solution() -> ProbeResult {
    scale_workloads(1)
}

#[test]
fn sparse_chain_and_grid_agree_with_bounded_dense_svd_reference() -> ProbeResult {
    for grid in [false, true] {
        let (a, b, nnz) = fixture(64, grid)?;
        assert_eq!((a.nrows(), nnz), if grid { (113, 225) } else { (64, 127) });
        let dense = a.to_dense();
        let values = dense.singular_values().map_err(|e| format!("SVD: {e:?}"))?;
        assert_eq!(values.iter().filter(|&&s| s > 1e-9 * values[0]).count(), 64);
        let sparse = a.sp_qr()?.solve_lstsq(&b);
        let reference = dense.col_piv_qr().solve_lstsq(&b);
        assert!((&sparse - &reference).norm_max() < 1e-9);
        assert!((&a * &sparse - &b).norm_max() < 1e-9);
        println!(
            "DENSE_REFERENCE,grid={grid},unknowns=64,rank=64,max_solution_difference={:e},max_residual={:e}",
            (&sparse - &reference).norm_max(),
            (&a * &sparse - &b).norm_max()
        );
    }
    Ok(())
}

/// Public, safe lower-level API: investigate whether backend configuration can
/// reduce an opaque call without changing the equation semantics or dependency.
pub(crate) fn configured_qr(iterations: usize, unknowns: usize) -> ProbeResult {
    use faer::{
        Conj,
        dyn_stack::{MemBuffer, MemStack},
        sparse::linalg::{
            SupernodalThreshold,
            qr::{QrSymbolicParams, factorize_symbolic_qr},
        },
    };
    for (name, threshold) in [
        ("simplicial", SupernodalThreshold::FORCE_SIMPLICIAL),
        ("supernodal", SupernodalThreshold::FORCE_SUPERNODAL),
    ] {
        if std::env::var("NEWELL_M50_QR_MODE")
            .ok()
            .is_some_and(|mode| mode != name)
        {
            continue;
        }
        for grid in [false, true] {
            let (a, b, _) = fixture(unknowns, grid)?;
            let setup = Instant::now();
            let symbolic = factorize_symbolic_qr(
                a.symbolic(),
                QrSymbolicParams {
                    supernodal_flop_ratio_threshold: threshold,
                    ..Default::default()
                },
            )?;
            let symbolic_setup_us = setup.elapsed().as_secs_f64() * 1e6;
            let mut symbolic_times = Vec::new();
            for _ in 0..iterations {
                let t = Instant::now();
                black_box(factorize_symbolic_qr(
                    a.symbolic(),
                    QrSymbolicParams {
                        supernodal_flop_ratio_threshold: threshold,
                        ..Default::default()
                    },
                )?);
                symbolic_times.push(t.elapsed().as_secs_f64() * 1e6);
            }
            distribution(
                &format!("configured_{name}_grid_{grid}_symbolic"),
                symbolic_times,
            );
            cancel_boundary(&format!("configured_{name}_grid_{grid}_symbolic"), || {
                black_box(factorize_symbolic_qr(
                    a.symbolic(),
                    QrSymbolicParams {
                        supernodal_flop_ratio_threshold: threshold,
                        ..Default::default()
                    },
                ))
                .ok();
            })?;
            let buffers_setup = Instant::now();
            let mut indices = vec![0; symbolic.len_idx()];
            let mut values = vec![0.; symbolic.len_val()];
            let mut numeric_mem = MemBuffer::new(
                symbolic.factorize_numeric_qr_scratch::<f64>(Par::Seq, Default::default()),
            );
            let mut solve_mem = MemBuffer::new(symbolic.solve_in_place_scratch::<f64>(1, Par::Seq));
            println!(
                "QR_CONFIG,{name},grid={grid},setup_us={:.3},reused_workspace_bytes={},rank_certification=Unavailable",
                symbolic_setup_us + buffers_setup.elapsed().as_secs_f64() * 1e6,
                symbolic.len_val() * 8
                    + symbolic.len_idx() * 8
                    + numeric_mem.len()
                    + solve_mem.len()
            );
            let mut times = Vec::new();
            let mut solve_times = Vec::new();
            for k in 0..=iterations {
                let t = Instant::now();
                let qr = symbolic.factorize_numeric_qr(
                    &mut indices,
                    &mut values,
                    a.as_ref(),
                    Par::Seq,
                    MemStack::new(&mut numeric_mem),
                    Default::default(),
                );
                let numeric = t.elapsed().as_secs_f64() * 1e6;
                let mut x = b.clone();
                let t = Instant::now();
                qr.solve_in_place_with_conj(
                    Conj::No,
                    x.as_mut(),
                    Par::Seq,
                    MemStack::new(&mut solve_mem),
                );
                let solve = t.elapsed().as_secs_f64() * 1e6;
                assert!((0..unknowns).all(|i| (x[(i, 0)] - i as f64 * 0.01).abs() < 1e-6));
                if k > 0 {
                    times.push(numeric);
                    solve_times.push(solve);
                }
            }
            distribution(&format!("configured_{name}_grid_{grid}_numeric"), times);
            distribution(&format!("configured_{name}_grid_{grid}_solve"), solve_times);
            cancel_boundary(&format!("configured_{name}_grid_{grid}_numeric"), || {
                black_box(symbolic.factorize_numeric_qr(
                    &mut indices,
                    &mut values,
                    a.as_ref(),
                    Par::Seq,
                    MemStack::new(&mut numeric_mem),
                    Default::default(),
                ));
            })?;
        }
    }
    Ok(())
}

#[test]
fn configured_sparse_methods_preserve_analytic_solution() -> ProbeResult {
    configured_qr(1, 64)
}

/// Revision 3: actual point/segment equations, not scalar incidence labels.
/// y coordinates are constrained to zero; unit distances connect every point.
/// Off-axis initial guesses exercise nonlinear length residuals and Jacobians.
pub(crate) fn geometric_chain(points: usize) -> ProbeResult {
    geometric_chain_ranked(points, false)
}

pub(crate) fn geometric_chain_ranked(points: usize, include_rank: bool) -> ProbeResult {
    use faer::{
        Conj,
        dyn_stack::{MemBuffer, MemStack},
        sparse::linalg::{
            SupernodalThreshold,
            qr::{QrSymbolicParams, factorize_symbolic_qr},
        },
    };
    let began = Instant::now();
    let mut xy: Vec<_> = (0..points)
        .map(|i| [i as f64 + 0.03 * (i as f64).sin(), 0.02 * (i as f64).cos()])
        .collect();
    let mut symbolic = None;
    let mut max_residual = 0_f64;
    let mut phases = Vec::new();
    let mut converged = false;
    let mut iterations = 0;
    for iteration in 0..10 {
        let assembling = Instant::now();
        let mut entries = Vec::with_capacity(points * 5);
        let mut rhs = Mat::zeros(2 * points, 1);
        entries.push(Triplet::new(0, 0, 1.));
        rhs[(0, 0)] = -xy[0][0];
        for i in 0..points {
            entries.push(Triplet::new(i + 1, 2 * i + 1, 1.));
            rhs[(i + 1, 0)] = -xy[i][1];
            if i > 0 {
                let dx = xy[i][0] - xy[i - 1][0];
                let dy = xy[i][1] - xy[i - 1][1];
                let length = dx.hypot(dy);
                if length < 1e-12 {
                    return Err("collapsed segment".into());
                }
                let row = points + i;
                for (col, value) in [
                    (2 * i, dx / length),
                    (2 * i + 1, dy / length),
                    (2 * i - 2, -dx / length),
                    (2 * i - 1, -dy / length),
                ] {
                    entries.push(Triplet::new(row, col, value));
                }
                rhs[(row, 0)] = 1. - length;
            }
        }
        max_residual = rhs.norm_max();
        if max_residual < 1e-9 {
            converged = true;
            break;
        }
        let a =
            SparseColMat::<usize, f64>::try_new_from_triplets(2 * points, 2 * points, &entries)?;
        let assembly_us = assembling.elapsed().as_secs_f64() * 1e6;
        let symbolic_start = Instant::now();
        if symbolic.is_none() {
            symbolic = Some(factorize_symbolic_qr(
                a.symbolic(),
                QrSymbolicParams {
                    supernodal_flop_ratio_threshold: SupernodalThreshold::FORCE_SUPERNODAL,
                    ..Default::default()
                },
            )?);
        }
        let symbol = symbolic.as_ref().ok_or("missing symbolic")?;
        let symbolic_us = symbolic_start.elapsed().as_secs_f64() * 1e6;
        let mut indices = vec![0; symbol.len_idx()];
        let mut values = vec![0.; symbol.len_val()];
        let mut scratch = MemBuffer::new(
            symbol.factorize_numeric_qr_scratch::<f64>(Par::Seq, Default::default()),
        );
        let factor_start = Instant::now();
        let qr = symbol.factorize_numeric_qr(
            &mut indices,
            &mut values,
            a.as_ref(),
            Par::Seq,
            MemStack::new(&mut scratch),
            Default::default(),
        );
        let factor_us = factor_start.elapsed().as_secs_f64() * 1e6;
        if include_rank {
            #[cfg(feature = "rank-research")]
            {
                use faer::sparse::linalg::rank::{
                    RankQrLimits, RankQrOptions, RankQrReliability, refactorize_rank,
                };
                let started = Instant::now();
                let report = refactorize_rank(
                    a.as_ref(),
                    symbol.col_perm().arrays().0,
                    1e-9,
                    RankQrLimits {
                        max_bytes: 1 << 30,
                        max_work: 400_000_000,
                    },
                    RankQrOptions::default(),
                    || false,
                )?;
                let rank = report.rank.ok_or("missing nonlinear rank")?;
                assert_eq!(rank.reliability, RankQrReliability::EstimatedReliable);
                assert_eq!(rank.rank, 2 * points);
                println!(
                    "R6_GEOMETRIC_RANK,iteration={iteration},unknowns={},rank={},factorizations={},us={},native_qr_max_ns={},dense_svd_max_ns={}",
                    2 * points,
                    rank.rank,
                    report.factorizations,
                    started.elapsed().as_secs_f64() * 1e6,
                    report.native_qr_max_ns,
                    report.dense_svd_max_ns
                );
            }
            #[cfg(not(feature = "rank-research"))]
            return Err("ranked nonlinear probe requires m50-feasibility".into());
        }

        let mut scratch = MemBuffer::new(symbol.solve_in_place_scratch::<f64>(1, Par::Seq));
        let solve_start = Instant::now();
        qr.solve_in_place_with_conj(
            Conj::No,
            rhs.as_mut(),
            Par::Seq,
            MemStack::new(&mut scratch),
        );
        let solve_us = solve_start.elapsed().as_secs_f64() * 1e6;
        for i in 0..points {
            xy[i][0] += rhs[(2 * i, 0)];
            xy[i][1] += rhs[(2 * i + 1, 0)];
        }
        phases.push((assembly_us, symbolic_us, factor_us, solve_us));
        iterations = iteration + 1;
    }
    let error = xy
        .iter()
        .enumerate()
        .map(|(i, p)| (p[0] - i as f64).abs().max(p[1].abs()))
        .fold(0., f64::max);
    assert!(converged && max_residual < 1e-9 && error < 1e-6);
    println!(
        "GEOMETRIC_CHAIN,points={points},segments={},entities={},unknowns={},equations={},nnz={},blocks=1,intersections=0,regions=0,visible=0,iterations={iterations},residual_mm={max_residual:e},solution_error_mm={error:e},total_us={:.3},rank_checked={include_rank},workspace_reuse=not_yet",
        points - 1,
        2 * points - 1,
        2 * points,
        2 * points,
        5 * points - 3,
        began.elapsed().as_secs_f64() * 1e6
    );
    for (i, (assembly, symbolic, factor, solve)) in phases.into_iter().enumerate() {
        println!(
            "GEOMETRIC_PHASE,iteration={i},assembly_us={assembly:.3},symbolic_us={symbolic:.3},factor_us={factor:.3},solve_us={solve:.3}"
        );
    }
    Ok(())
}
#[test]
fn nonlinear_point_segment_chain_matches_analytic_solution() -> ProbeResult {
    geometric_chain(32)
}

#[cfg(feature = "rank-research")]
#[test]
fn m50_rank_factor_bridge_preserves_original_columns_and_rejection_error()
-> Result<(), Box<dyn std::error::Error>> {
    use faer::sparse::linalg::rank::{
        RankQrLimits, RankQrProgress, RankQrSymbolic, RankQrWorkspace,
    };
    let cases = [
        faer::mat![[0., 1., 2., 0.], [0., 2., 4., 1.], [0., 3., 6., 0.]],
        faer::mat![[1., 1., 0.], [0., 1e-12, 1.], [0., 0., 1.], [0., 0., 0.]],
        faer::mat![[0., 1., 2., 3.]],
        faer::mat![[0., 0.], [0., 0.]],
        faer::mat![[1., 2.], [3., 4.]],
    ];
    for (case, dense) in cases.iter().enumerate() {
        for reverse in [false, true] {
            let entries: Vec<_> = (0..dense.ncols())
                .flat_map(|j| {
                    (0..dense.nrows()).filter_map(move |i| {
                        (dense[(i, j)] != 0.).then_some(Triplet::new(i, j, dense[(i, j)]))
                    })
                })
                .collect();
            let a = SparseColMat::<usize, f64>::try_new_from_triplets(
                dense.nrows(),
                dense.ncols(),
                &entries,
            )?;
            let mut order: Vec<_> = (0..dense.ncols()).collect();
            if reverse {
                order.reverse();
            }
            let symbolic = RankQrSymbolic::new(a.as_ref(), &order, 1 << 20)?;
            let limits = RankQrLimits {
                max_bytes: 1 << 20,
                max_work: 1_000_000,
            };
            let mut work = RankQrWorkspace::new(&symbolic, limits)?;
            for _repeat in 0..2 {
                work.start(1e-10)?;
                while work.advance(a.as_ref(), 1, || false)? == RankQrProgress::Pending {}
                let rank = work.preliminary_rank()?;
                let expected = dense
                    .singular_values()
                    .map_err(|e| format!("SVD {e:?}"))?
                    .iter()
                    .filter(|&&s| s > 1e-10)
                    .count();
                assert_eq!(rank, expected, "case {case}, reverse={reverse}");
                let perm: Vec<_> = work.permutation()?.collect();
                assert_eq!(perm.len(), dense.ncols());
                let mut sorted = perm.clone();
                sorted.sort_unstable();
                assert_eq!(sorted, (0..dense.ncols()).collect::<Vec<_>>());
                let (dropped, norm_e) = work.perturbation()?;
                let mut observed = 0_f64;
                for j in 0..dense.ncols() {
                    let mut x: Vec<_> = (0..dense.nrows()).map(|i| dense[(i, j)]).collect();
                    work.apply_q(&mut x, true, || false)?;
                    for &(i, r) in work.r_column(j)? {
                        x[i] -= r;
                    }
                    let residual = x.iter().fold(0_f64, |n, &v| n.hypot(v));
                    assert!(
                        (residual - dropped[j]).abs() < 2e-14,
                        "case {case}: error accounting {residual:e} vs {:e}",
                        dropped[j]
                    );
                    observed = observed.hypot(residual);
                }
                assert!((observed - norm_e).abs() < 5e-14);
                for j in 0..dense.nrows() {
                    let mut x = vec![0.; dense.nrows()];
                    x[j] = 1.;
                    work.apply_q(&mut x, false, || false)?;
                    work.apply_q(&mut x, true, || false)?;
                    for (i, value) in x.iter().enumerate() {
                        assert!((value - f64::from(i == j)).abs() < 1e-14);
                    }
                }
                // R11 is triangular in accepted-column coordinates even when
                // zero/dependent original columns were deferred in between.
                for (k, &j) in perm.iter().take(rank).enumerate() {
                    assert!(work.r_column(j)?.iter().all(|&(i, _)| i <= k));
                    assert!(
                        work.r_column(j)?
                            .iter()
                            .any(|&(i, v)| i == k && v.abs() > 1e-10)
                    );
                }
                println!(
                    "FACTOR_BRIDGE,case={case},reverse={reverse},preliminary_rank={rank},perturbation={norm_e:e},bytes={},work={}",
                    work.usage().0,
                    work.usage().1
                );
            }
        }
    }
    Ok(())
}

#[cfg(feature = "rank-research")]
#[test]
fn m50_rank_factor_bridge_invalidates_partial_cancelled_and_over_budget_jobs()
-> Result<(), Box<dyn std::error::Error>> {
    use faer::sparse::linalg::rank::{
        RankQrError, RankQrLimits, RankQrProgress, RankQrSymbolic, RankQrWorkspace,
    };
    let entries = [Triplet::new(0, 0, 1.), Triplet::new(1, 1, 1.)];
    let a = SparseColMat::<usize, f64>::try_new_from_triplets(2, 2, &entries)?;
    assert!(matches!(
        RankQrSymbolic::new(a.as_ref(), &[0, 0], 1 << 20),
        Err(RankQrError::InvalidInput)
    ));
    assert!(matches!(
        RankQrSymbolic::new(a.as_ref(), &[0, 1], 1),
        Err(RankQrError::WorkspaceLimit)
    ));
    let symbolic = RankQrSymbolic::new(a.as_ref(), &[0, 1], 1 << 20)?;
    let mut work = RankQrWorkspace::new(
        &symbolic,
        RankQrLimits {
            max_bytes: 1 << 20,
            max_work: 1000,
        },
    )?;
    work.start(1e-10)?;
    assert_eq!(
        work.advance(a.as_ref(), 1, || false)?,
        RankQrProgress::Pending
    );
    assert!(matches!(
        work.preliminary_rank(),
        Err(RankQrError::Incomplete)
    ));
    assert_eq!(
        work.advance(a.as_ref(), 1, || true),
        Err(RankQrError::Cancelled)
    );
    assert!(matches!(work.permutation(), Err(RankQrError::Incomplete)));
    work.start(1e-10)?;
    assert_eq!(
        work.advance(a.as_ref(), 2, || false)?,
        RankQrProgress::Complete
    );
    assert_eq!(work.preliminary_rank()?, 2);
    let mut tiny = RankQrWorkspace::new(
        &symbolic,
        RankQrLimits {
            max_bytes: 1 << 20,
            max_work: 0,
        },
    )?;
    tiny.start(1e-10)?;
    assert_eq!(
        tiny.advance(a.as_ref(), 2, || false),
        Err(RankQrError::WorkLimit)
    );
    assert!(matches!(
        tiny.preliminary_rank(),
        Err(RankQrError::Incomplete)
    ));
    assert_eq!(work.start(f64::NAN), Err(RankQrError::InvalidInput));
    assert!(matches!(
        work.preliminary_rank(),
        Err(RankQrError::Incomplete)
    ));
    Ok(())
}

#[cfg(feature = "rank-research")]
fn rank_protocol_probe(
    dense: &Mat<f64>,
    tau: f64,
    seed: u64,
) -> Result<faer::sparse::linalg::rank::RankQrReport, Box<dyn std::error::Error>> {
    use faer::sparse::linalg::rank::{
        RankQrLimits, RankQrOptions, RankQrProgress, RankQrSymbolic, RankQrWorkspace,
    };
    let entries: Vec<_> = (0..dense.ncols())
        .flat_map(|j| {
            (0..dense.nrows()).filter_map(move |i| {
                (dense[(i, j)] != 0.).then_some(Triplet::new(i, j, dense[(i, j)]))
            })
        })
        .collect();
    let a =
        SparseColMat::<usize, f64>::try_new_from_triplets(dense.nrows(), dense.ncols(), &entries)?;
    let symbolic =
        RankQrSymbolic::new(a.as_ref(), &(0..dense.ncols()).collect::<Vec<_>>(), 1 << 24)?;
    let mut work = RankQrWorkspace::new(
        &symbolic,
        RankQrLimits {
            max_bytes: 1 << 24,
            max_work: 100_000_000,
        },
    )?;
    work.start(tau)?;
    while work.advance(a.as_ref(), 3, || false)? == RankQrProgress::Pending {}
    Ok(work.analyze_rank(
        RankQrOptions {
            seed,
            max_block: 32,
            ..Default::default()
        },
        || false,
    )?)
}

#[cfg(feature = "rank-research")]
#[test]
fn m50_rank_protocol_corrects_diagonal_count_and_checks_threshold_separation() -> ProbeResult {
    use faer::sparse::linalg::rank::RankQrReliability;
    for scale in [1e-6, 1., 1e6] {
        let examples = [
            (
                "diagonal_trap",
                faer::mat![[scale, scale * 1e8], [0., scale]],
                1,
                2,
            ),
            ("regular", faer::mat![[scale, 0.], [0., scale]], 2, 2),
            (
                "duplicate",
                faer::mat![[scale, scale, 0.], [0., 0., scale]],
                2,
                2,
            ),
            (
                "tall",
                faer::mat![[scale, 0.], [0., scale], [scale, scale]],
                2,
                2,
            ),
            ("zero", Mat::zeros(2, 4), 0, 0),
        ];
        for (name, a, expected, preliminary) in examples {
            let tau = scale * 1e-6;
            let svd_rank = a
                .singular_values()
                .map_err(|e| format!("SVD {e:?}"))?
                .iter()
                .filter(|&&s| s > tau)
                .count();
            assert_eq!(svd_rank, expected);
            for seed in [1, 7, 1209] {
                let report = rank_protocol_probe(&a, tau, seed)?;
                println!("RANK_PROTOCOL,{name},scale={scale:e},{report:?}");
                assert_eq!(report.preliminary_rank, preliminary);
                assert_eq!(
                    report.reliability,
                    RankQrReliability::EstimatedReliable,
                    "{name} {report:?}"
                );
                assert_eq!(report.rank, expected);
                assert!(report.upper <= tau);
                assert!(expected == 0 || report.lower > tau);
            }
        }
    }
    // Each rejected column is smaller than tau, but their aggregate has a
    // singular value above tau. The bridge must not certify zero rank.
    let a = faer::mat![[0.8, 0.8]];
    let report = rank_protocol_probe(&a, 1., 1)?;
    assert_eq!(report.reliability, RankQrReliability::Uncertain);
    assert_eq!(report.preliminary_rank, 0);
    // Semantic tau is never raised to the alternate value suggested by a method.
    let report = rank_protocol_probe(&faer::mat![[1.00000001, 0.], [0., 0.99999999]], 1., 7)?;
    assert_eq!(report.threshold, 1.);
    if report.reliability == RankQrReliability::EstimatedReliable {
        assert_eq!(report.rank, 1);
    }
    Ok(())
}

#[cfg(feature = "rank-research")]
pub(crate) fn rank_large(iterations: usize, n: usize) -> ProbeResult {
    use faer::dyn_stack::{MemBuffer, MemStack};
    use faer::sparse::linalg::{
        colamd,
        rank::{RankQrLimits, RankQrOptions, RankQrProgress, RankQrSymbolic, RankQrWorkspace},
    };
    let reference = rank_protocol_probe(&faer::mat![[1., 1e8], [0., 1.]], 1e-6, 1)?;
    assert_eq!(reference.rank, 1);
    for (grid, deficient) in [(false, false), (false, true), (true, false), (true, true)] {
        let (original, _, _) = fixture(n, grid)?;
        let entries: Vec<_> = (0..n)
            .filter(|&j| !deficient || j != n - 1)
            .flat_map(|j| {
                original
                    .row_idx_of_col(j)
                    .zip(original.val_of_col(j))
                    .map(move |(i, &v)| Triplet::new(i, j, v))
            })
            .collect();
        let a = SparseColMat::<usize, f64>::try_new_from_triplets(original.nrows(), n, &entries)?;
        let mut order = vec![0; n];
        let mut inverse = vec![0; n];
        let mut memory = MemBuffer::new(colamd::order_scratch::<usize>(
            a.nrows(),
            n,
            a.compute_nnz(),
        ));
        colamd::order(
            &mut order,
            &mut inverse,
            a.symbolic(),
            Default::default(),
            MemStack::new(&mut memory),
        )?;
        let symbolic = RankQrSymbolic::new(a.as_ref(), &order, 1 << 30)?;
        let mut work = RankQrWorkspace::new(
            &symbolic,
            RankQrLimits {
                max_bytes: 1 << 30,
                max_work: 400_000_000,
            },
        )?;
        let qr_symbolic = faer::sparse::linalg::qr::factorize_symbolic_qr(
            a.symbolic(),
            faer::sparse::linalg::qr::QrSymbolicParams {
                supernodal_flop_ratio_threshold:
                    faer::sparse::linalg::SupernodalThreshold::FORCE_SUPERNODAL,
                ..Default::default()
            },
        )?;
        let requirement =
            qr_symbolic.factorize_numeric_qr_scratch::<f64>(Par::Seq, Default::default());
        assert!(
            requirement.size_bytes() + (qr_symbolic.len_idx() + qr_symbolic.len_val()) * 8
                < (1 << 30)
        );
        let mut indices = vec![0; qr_symbolic.len_idx()];
        let mut values = vec![0.; qr_symbolic.len_val()];
        let mut qr_memory = MemBuffer::new(requirement);
        let mut factor_us = Vec::new();
        let mut rank_us = Vec::new();
        for sample in 0..=iterations {
            work.start(1e-9)?;
            let began = Instant::now();
            let qr = qr_symbolic.factorize_numeric_qr(
                &mut indices,
                &mut values,
                a.as_ref(),
                Par::Seq,
                MemStack::new(&mut qr_memory),
                Default::default(),
            );
            let outcome = match work.import_column_factor(qr, 1e-9, || false) {
                Ok(true) => Ok(RankQrProgress::Complete),
                Ok(false) => {
                    work.start(1e-9)?;
                    loop {
                        match work.advance(a.as_ref(), 16, || false) {
                            Ok(RankQrProgress::Pending) => {}
                            other => break other,
                        }
                    }
                }
                Err(error) => Err(error),
            };
            let elapsed = began.elapsed().as_secs_f64() * 1e6;
            println!(
                "RANK_C1,grid={grid},deficient={deficient},n={n},sample={sample},factor={outcome:?},factor_us={elapsed},bytes={},work={}",
                work.usage().0,
                work.usage().1
            );
            if outcome.is_err() {
                break;
            }
            let began = Instant::now();
            let report = work.analyze_rank(RankQrOptions::default(), || false);
            let rank_elapsed = began.elapsed().as_secs_f64() * 1e6;
            println!(
                "RANK_C1_REPORT,grid={grid},deficient={deficient},n={n},sample={sample},{report:?},rank_us={}",
                rank_elapsed
            );
            if sample > 0 {
                factor_us.push(elapsed);
                rank_us.push(rank_elapsed);
            }
            if let Ok(report) = report
                && report.reliability
                    == faer::sparse::linalg::rank::RankQrReliability::EstimatedReliable
            {
                assert_eq!(report.rank, n - usize::from(deficient));
            }
        }
        distribution(
            if grid {
                if deficient {
                    "rank_grid_deficient_factor"
                } else {
                    "rank_grid_factor"
                }
            } else {
                if deficient {
                    "rank_chain_deficient_factor"
                } else {
                    "rank_chain_factor"
                }
            },
            factor_us,
        );
        distribution(
            if grid {
                if deficient {
                    "rank_grid_deficient_analysis"
                } else {
                    "rank_grid_analysis"
                }
            } else {
                if deficient {
                    "rank_chain_deficient_analysis"
                } else {
                    "rank_chain_analysis"
                }
            },
            rank_us,
        );
    }
    Ok(())
}

#[cfg(feature = "rank-research")]
#[test]
fn m50_rank_sparse_chain_and_grid_characterization() -> ProbeResult {
    rank_large(0, 64)
}

#[cfg(feature = "rank-research")]
#[test]
fn m50_rank_imported_r_preserves_gram_permutation_and_reliability() -> ProbeResult {
    use faer::dyn_stack::{MemBuffer, MemStack};
    use faer::sparse::linalg::{
        SupernodalThreshold,
        qr::{QrSymbolicParams, factorize_symbolic_qr},
        rank::{RankQrLimits, RankQrOptions, RankQrSymbolic, RankQrWorkspace},
    };
    for grid in [false, true] {
        let (a, _, _) = fixture(64, grid)?;
        let mut gram = Mat::zeros(64, 64);
        let mut dense = Mat::zeros(a.nrows(), 64);
        for j in 0..64 {
            for (i, &v) in a.row_idx_of_col(j).zip(a.val_of_col(j)) {
                dense[(i, j)] = v;
            }
        }
        for i in 0..64 {
            for j in 0..64 {
                gram[(i, j)] = (0..a.nrows())
                    .map(|k| dense[(k, i)] * dense[(k, j)])
                    .sum::<f64>();
            }
        }
        for threshold in [
            SupernodalThreshold::FORCE_SIMPLICIAL,
            SupernodalThreshold::FORCE_SUPERNODAL,
        ] {
            let qr_symbolic = factorize_symbolic_qr(
                a.symbolic(),
                QrSymbolicParams {
                    supernodal_flop_ratio_threshold: threshold,
                    ..Default::default()
                },
            )?;
            let mut indices = vec![0; qr_symbolic.len_idx()];
            let mut values = vec![0.; qr_symbolic.len_val()];
            let mut memory = MemBuffer::new(
                qr_symbolic.factorize_numeric_qr_scratch::<f64>(Par::Seq, Default::default()),
            );
            let qr = qr_symbolic.factorize_numeric_qr(
                &mut indices,
                &mut values,
                a.as_ref(),
                Par::Seq,
                MemStack::new(&mut memory),
                Default::default(),
            );
            let order: Vec<_> = (0..64).collect();
            let symbolic = RankQrSymbolic::new(a.as_ref(), &order, 1 << 24)?;
            let mut work = RankQrWorkspace::new(
                &symbolic,
                RankQrLimits {
                    max_bytes: 1 << 24,
                    max_work: 100_000_000,
                },
            )?;
            assert!(work.import_full_column_factor(qr, 1e-9, || false)?);
            let perm: Vec<_> = work.permutation()?.collect();
            let mut r = Mat::zeros(64, 64);
            for (j, &col) in perm.iter().enumerate() {
                for &(i, v) in work.r_column(col)? {
                    r[(i, j)] = v;
                }
            }
            for i in 0..64 {
                for j in 0..64 {
                    let actual = (0..64).map(|k| r[(k, i)] * r[(k, j)]).sum::<f64>();
                    assert!((actual - gram[(perm[i], perm[j])]).abs() < 1e-12);
                }
            }
            let report = work.analyze_rank(RankQrOptions::default(), || false)?;
            assert_eq!(
                report.reliability,
                faer::sparse::linalg::rank::RankQrReliability::EstimatedReliable
            );
            assert_eq!(report.rank, 64);
            assert!(
                work.apply_q(&mut vec![0.; a.nrows()], true, || false)
                    .is_err()
            );
        }
    }
    Ok(())
}

#[cfg(feature = "rank-research")]
#[test]
fn m50_rank_bridge_mid_phase_cancellation_and_sparse_input_errors() -> ProbeResult {
    use faer::sparse::linalg::rank::{RankQrError, RankQrLimits, RankQrSymbolic, RankQrWorkspace};
    let entries: Vec<_> = (0..32)
        .flat_map(|j| (0..32).map(move |i| Triplet::new(i, j, if i == j { 2. } else { 0.01 })))
        .collect();
    let a = SparseColMat::<usize, f64>::try_new_from_triplets(32, 32, &entries)?;
    let order: Vec<_> = (0..32).collect();
    let symbolic = RankQrSymbolic::new(a.as_ref(), &order, 1 << 24)?;
    let mut work = RankQrWorkspace::new(
        &symbolic,
        RankQrLimits {
            max_bytes: 1 << 24,
            max_work: 1_000_000,
        },
    )?;
    work.start(1e-9)?;
    let mut polls = 0;
    assert_eq!(
        work.advance(a.as_ref(), 32, || {
            polls += 1;
            polls >= 4
        }),
        Err(RankQrError::Cancelled)
    );
    assert!(work.preliminary_rank().is_err());
    assert!(work.usage().1 > 0);
    work.start(1e-9)?;
    let invalid =
        SparseColMat::<usize, f64>::try_new_from_triplets(32, 32, &[Triplet::new(0, 0, 1.)])?;
    assert_eq!(
        work.advance(invalid.as_ref(), 1, || false),
        Err(RankQrError::InvalidInput)
    );
    let mut values: Vec<_> = entries.to_vec();
    values[0] = Triplet::new(0, 0, f64::NAN);
    let invalid = SparseColMat::<usize, f64>::try_new_from_triplets(32, 32, &values)?;
    work.start(1e-9)?;
    assert_eq!(
        work.advance(invalid.as_ref(), 1, || false),
        Err(RankQrError::InvalidInput)
    );
    Ok(())
}

/// Revision6 pre-change reproduction, forced through the existing general bridge.
#[cfg(feature = "rank-research")]
pub(crate) fn revision6_reproduce_deferral(n: usize) -> ProbeResult {
    use faer::dyn_stack::{MemBuffer, MemStack};
    use faer::sparse::linalg::{
        colamd,
        rank::{RankQrLimits, RankQrProgress, RankQrSymbolic, RankQrWorkspace},
    };
    let (a, _, _) = fixture(n, true)?;
    let mut order = vec![0; n];
    let mut inverse = vec![0; n];
    let mut memory = MemBuffer::new(colamd::order_scratch::<usize>(
        a.nrows(),
        n,
        a.compute_nnz(),
    ));
    colamd::order(
        &mut order,
        &mut inverse,
        a.symbolic(),
        Default::default(),
        MemStack::new(&mut memory),
    )?;
    let symbolic = RankQrSymbolic::new(a.as_ref(), &order, 1 << 30)?;
    let mut work = RankQrWorkspace::new(
        &symbolic,
        RankQrLimits {
            max_bytes: 1 << 30,
            max_work: 400_000_000,
        },
    )?;
    work.start(1e-9)?;
    let began = Instant::now();
    let outcome = loop {
        match work.advance(a.as_ref(), 16, || false) {
            Ok(RankQrProgress::Pending) => {}
            other => break other,
        }
    };
    println!(
        "R6_REPRO,grid=true,unknowns={n},rows={},outcome={outcome:?},elapsed_us={},bytes={},work={},budget_units=400000000,limit_bytes=1073741824",
        a.nrows(),
        began.elapsed().as_secs_f64() * 1e6,
        work.usage().0,
        work.usage().1
    );
    if n <= 256 {
        assert_eq!(outcome?, RankQrProgress::Complete);
        assert_eq!(work.preliminary_rank()?, n);
    }
    Ok(())
}
#[cfg(feature = "rank-research")]
#[test]
fn revision6_general_bridge_reproducer_has_known_small_rank() -> ProbeResult {
    revision6_reproduce_deferral(64)
}

#[cfg(feature = "rank-research")]
#[test]
fn revision6_explicit_qr_order_and_nonterminal_refactor_preserve_original_rank() -> ProbeResult {
    use faer::sparse::linalg::rank::{
        RankQrLimits, RankQrOptions, RankQrReliability, refactorize_rank,
    };
    for (dense, tau, rank) in [
        (
            faer::mat![[1., 0., 0.], [0., 1e-12, 0.], [0., 0., 1.]],
            1e-9,
            2,
        ),
        (faer::mat![[1., 1e8], [0., 1.]], 1e-6, 1),
        (
            faer::mat![[1., 1., 0.], [0., 0., 1.], [0., 0., 0.]],
            1e-9,
            2,
        ),
        (faer::mat![[1., 0.], [0., 1.], [1., 1.]], 1e-9, 2),
        (Mat::zeros(3, 2), 1e-9, 0),
        (faer::mat![[1., 0., 1.], [0., 1., 1.]], 1e-9, 2),
        (
            faer::mat![[1., 0., 1., 0.], [1., 0., 1., 0.], [0., 1., 0., 1.]],
            1e-9,
            2,
        ),
    ] {
        let entries: Vec<_> = (0..dense.ncols())
            .flat_map(|j| {
                (0..dense.nrows()).filter_map({
                    let d = &dense;
                    move |i| (d[(i, j)] != 0.).then_some(Triplet::new(i, j, d[(i, j)]))
                })
            })
            .collect();
        let a = SparseColMat::<usize, f64>::try_new_from_triplets(
            dense.nrows(),
            dense.ncols(),
            &entries,
        )?;
        let expected = dense
            .singular_values()
            .map_err(|e| format!("SVD {e:?}"))?
            .iter()
            .filter(|&&s| s > tau)
            .count();
        assert_eq!(expected, rank);
        for reverse in [false, true] {
            let mut order: Vec<_> = (0..dense.ncols()).collect();
            if reverse {
                order.reverse();
            }
            let result = refactorize_rank(
                a.as_ref(),
                &order,
                tau,
                RankQrLimits {
                    max_bytes: 1 << 24,
                    max_work: 10_000_000,
                },
                RankQrOptions::default(),
                || false,
            )?;
            println!("R6_REFACTOR,reverse={reverse},{result:?}");
            assert!(result.factorizations <= 3);
            let actual = result.rank.ok_or("no bridge")?;
            assert_eq!(actual.reliability, RankQrReliability::EstimatedReliable);
            assert_eq!(actual.rank, expected);
        }
    }
    Ok(())
}

/// Revision6 C1 bounded refactorization, regular and two distributed dependencies.
#[cfg(feature = "rank-research")]
pub(crate) fn revision6_refactor_workloads(runs: usize, n: usize) -> ProbeResult {
    use faer::dyn_stack::{MemBuffer, MemStack};
    use faer::sparse::linalg::{
        colamd,
        rank::{RankQrLimits, RankQrOptions, RankQrReliability, refactorize_rank},
    };
    for (grid, dependent) in [(false, false), (false, true), (true, false), (true, true)] {
        let build = Instant::now();
        let (original, _, _) = fixture(n, grid)?;
        let mut entries = Vec::new();
        for j in 0..n {
            let source = if dependent && j == n / 3 {
                0
            } else if dependent && j == 2 * n / 3 {
                n - 1
            } else {
                j
            };
            entries.extend(
                original
                    .row_idx_of_col(source)
                    .zip(original.val_of_col(source))
                    .map(|(i, &v)| Triplet::new(i, j, v)),
            );
        }
        let a = SparseColMat::<usize, f64>::try_new_from_triplets(original.nrows(), n, &entries)?;
        let mut order = vec![0; n];
        let mut inverse = vec![0; n];
        let mut memory = MemBuffer::new(colamd::order_scratch::<usize>(
            a.nrows(),
            n,
            a.compute_nnz(),
        ));
        colamd::order(
            &mut order,
            &mut inverse,
            a.symbolic(),
            Default::default(),
            MemStack::new(&mut memory),
        )?;
        let setup_us = build.elapsed().as_secs_f64() * 1e6;
        let mut samples = Vec::new();
        for sample in 0..=runs {
            let began = Instant::now();
            let report = refactorize_rank(
                a.as_ref(),
                &order,
                1e-9,
                RankQrLimits {
                    max_bytes: 1 << 30,
                    max_work: 400_000_000,
                },
                RankQrOptions {
                    seed: 1 + sample as u64,
                    ..Default::default()
                },
                || false,
            )?;
            let elapsed = began.elapsed().as_secs_f64() * 1e6;
            let rank = report.rank.ok_or("no admissible refactored bridge")?;
            println!(
                "R6_C1,grid={grid},dependent={dependent},n={n},rows={},nnz={},sample={sample},expected_rank={},rank={},reliability={:?},factorizations={},setup_us={setup_us},numeric_total_us={elapsed},admitted_bytes={},counted_work={},seed={},native_qr_max_ns={},dense_svd_max_ns={},symbolic_call_upper_ns={}",
                a.nrows(),
                a.compute_nnz(),
                n - 2 * usize::from(dependent),
                rank.rank,
                rank.reliability,
                report.factorizations,
                report.numeric_admitted_bytes,
                report.counted_work,
                rank.seed,
                report.native_qr_max_ns,
                report.dense_svd_max_ns,
                report.symbolic_call_upper_ns
            );
            if rank.reliability != RankQrReliability::EstimatedReliable {
                return Err("refactored rank remains uncertain".into());
            }
            assert_eq!(rank.rank, n - 2 * usize::from(dependent));
            assert!(report.factorizations <= 3);
            if sample > 0 {
                samples.push(elapsed);
            }
        }
        distribution(
            &format!("r6_refactor_grid_{grid}_dependent_{dependent}"),
            samples,
        );
    }
    Ok(())
}
#[cfg(feature = "rank-research")]
#[test]
fn revision6_distributed_dependencies_match_analytic_chain_grid_rank() -> ProbeResult {
    revision6_refactor_workloads(0, 64)
}

#[cfg(feature = "rank-research")]
#[test]
fn revision6_native_q_and_full_complement_reconstruct_original_matrix() -> ProbeResult {
    use faer::dyn_stack::{MemBuffer, MemStack};
    use faer::sparse::linalg::{
        SupernodalThreshold,
        qr::{QrSymbolicParams, factorize_symbolic_qr},
    };
    for grid in [false, true] {
        let (a, _, _) = fixture(64, grid)?;
        for threshold in [
            SupernodalThreshold::FORCE_SIMPLICIAL,
            SupernodalThreshold::FORCE_SUPERNODAL,
        ] {
            let symbolic = factorize_symbolic_qr(
                a.symbolic(),
                QrSymbolicParams {
                    supernodal_flop_ratio_threshold: threshold,
                    ..Default::default()
                },
            )?;
            let mut indices = vec![0; symbolic.len_idx()];
            let mut values = vec![0.; symbolic.len_val()];
            let mut memory = MemBuffer::new(
                symbolic.factorize_numeric_qr_scratch::<f64>(Par::Seq, Default::default()),
            );
            let qr = symbolic.factorize_numeric_qr(
                &mut indices,
                &mut values,
                a.as_ref(),
                Par::Seq,
                MemStack::new(&mut memory),
                Default::default(),
            );
            let q = qr.orthogonal_operator(1 << 24, || false)?;
            let perm = symbolic.col_perm().arrays().0;
            for (k, &j) in perm.iter().enumerate() {
                let mut x = vec![0.; a.nrows()];
                for (i, &v) in a.row_idx_of_col(j).zip(a.val_of_col(j)) {
                    x[i] = v;
                }
                q.apply(&mut x, true, || false)?;
                qr.visit_r_entries(
                    |i, col, v| -> Result<(), faer::sparse::linalg::rank::RankQrError> {
                        if col == k {
                            x[i] -= v;
                        }
                        Ok(())
                    },
                )?;
                assert!(
                    x.iter().all(|v| v.abs() < 1e-12),
                    "grid={grid},threshold={threshold:?},column={k},residual={}",
                    x.iter().copied().map(f64::abs).fold(0., f64::max)
                );
            }
            for j in [0, 63, a.nrows() - 1] {
                let mut x = vec![0.; a.nrows()];
                x[j] = 1.;
                q.apply(&mut x, false, || false)?;
                q.apply(&mut x, true, || false)?;
                for (i, v) in x.iter().enumerate() {
                    assert!((v - f64::from(i == j)).abs() < 1e-12);
                }
            }
        }
    }
    Ok(())
}

#[cfg(feature = "rank-research")]
#[test]
fn revision6_left_right_null_actions_are_orthonormal_and_annihilate_original_matrix() -> ProbeResult
{
    use faer::dyn_stack::{MemBuffer, MemStack};
    use faer::sparse::linalg::{
        SupernodalThreshold,
        qr::{QrSymbolicParams, factorize_symbolic_qr_ordered},
    };
    let dense = faer::mat![[1., 1., 0.], [0., 0., 1.], [1., 1., 1.], [2., 2., -1.]];
    let entries: Vec<_> = (0..3)
        .flat_map(|j| {
            (0..4).filter_map({
                let a = &dense;
                move |i| (a[(i, j)] != 0.).then_some(Triplet::new(i, j, a[(i, j)]))
            })
        })
        .collect();
    let a = SparseColMat::<usize, f64>::try_new_from_triplets(4, 3, &entries)?;
    let symbolic = factorize_symbolic_qr_ordered(
        a.symbolic(),
        &[0, 2, 1],
        QrSymbolicParams {
            supernodal_flop_ratio_threshold: SupernodalThreshold::FORCE_SUPERNODAL,
            ..Default::default()
        },
        1 << 24,
        || false,
    )?;
    let mut indices = vec![0; symbolic.len_idx()];
    let mut values = vec![0.; symbolic.len_val()];
    let mut memory =
        MemBuffer::new(symbolic.factorize_numeric_qr_scratch::<f64>(Par::Seq, Default::default()));
    let qr = symbolic.factorize_numeric_qr(
        &mut indices,
        &mut values,
        a.as_ref(),
        Par::Seq,
        MemStack::new(&mut memory),
        Default::default(),
    );
    let left = qr.orthogonal_operator(1 << 24, || false)?;
    let right = qr.right_null_operator(2, 1 << 24, || false)?;
    let mut previous = None;
    for j in 0..2 {
        let mut coefficients = vec![0.; 2];
        coefficients[j] = 1.;
        let mut vector = vec![0.; 4];
        left.apply_null(2, &coefficients, &mut vector, false, || false)?;
        for col in 0..3 {
            assert!(
                (0..4)
                    .map(|i| dense[(i, col)] * vector[i])
                    .sum::<f64>()
                    .abs()
                    < 1e-12
            );
        }
        let mut back = vec![0.; 2];
        left.apply_null(2, &vector, &mut back, true, || false)?;
        for (a, b) in back.iter().zip(coefficients) {
            assert!((a - b).abs() < 1e-12);
        }
        if let Some(old) = previous {
            let old: Vec<f64> = old;
            assert!(
                old.iter()
                    .zip(&vector)
                    .map(|(a, b)| a * b)
                    .sum::<f64>()
                    .abs()
                    < 1e-12
            );
        }
        previous = Some(vector);
    }
    let mut vector = vec![0.; 3];
    right.apply_null(2, &[1.], &mut vector, false, || false)?;
    for row in 0..4 {
        assert!(
            (0..3)
                .map(|j| dense[(row, j)] * vector[j])
                .sum::<f64>()
                .abs()
                < 1e-12
        );
    }
    assert!((vector.iter().map(|v| v * v).sum::<f64>() - 1.).abs() < 1e-12);
    let mut back = [0.];
    right.apply_null(2, &vector, &mut back, true, || false)?;
    assert!((back[0] - 1.).abs() < 1e-12);
    Ok(())
}

#[cfg(feature = "rank-research")]
#[test]
fn revision6_native_q_dense_front_blocking_preserves_orthogonal_complement() -> ProbeResult {
    use faer::dyn_stack::{MemBuffer, MemStack};
    use faer::sparse::linalg::{
        SupernodalThreshold,
        qr::{QrSymbolicParams, factorize_symbolic_qr_ordered},
    };
    let (m, n) = (128, 96);
    let entries: Vec<_> = (0..n)
        .flat_map(|j| {
            (0..m).map(move |i| {
                Triplet::new(
                    i,
                    j,
                    if i == j {
                        4.
                    } else {
                        ((i * 37 + j * 13 + 17) % 101) as f64 / 101. - 0.5
                    },
                )
            })
        })
        .collect();
    let a = SparseColMat::<usize, f64>::try_new_from_triplets(m, n, &entries)?;
    let order: Vec<_> = (0..n).rev().collect();
    let symbolic = factorize_symbolic_qr_ordered(
        a.symbolic(),
        &order,
        QrSymbolicParams {
            supernodal_flop_ratio_threshold: SupernodalThreshold::FORCE_SUPERNODAL,
            ..Default::default()
        },
        1 << 26,
        || false,
    )?;
    let mut indices = vec![0; symbolic.len_idx()];
    let mut values = vec![0.; symbolic.len_val()];
    let mut memory =
        MemBuffer::new(symbolic.factorize_numeric_qr_scratch::<f64>(Par::Seq, Default::default()));
    let qr = symbolic.factorize_numeric_qr(
        &mut indices,
        &mut values,
        a.as_ref(),
        Par::Seq,
        MemStack::new(&mut memory),
        Default::default(),
    );
    let q = qr.orthogonal_operator(1 << 26, || false)?;
    for k in [0, 31, 32, 63, 64, 95] {
        let mut x = vec![0.; m];
        for (i, &v) in a.row_idx_of_col(order[k]).zip(a.val_of_col(order[k])) {
            x[i] = v;
        }
        q.apply(&mut x, true, || false)?;
        qr.visit_r_entries(
            |i, j, v| -> Result<(), faer::sparse::linalg::rank::RankQrError> {
                if j == k {
                    x[i] -= v;
                }
                Ok(())
            },
        )?;
        assert!(x.iter().all(|v| v.abs() < 1e-11));
    }
    let mut x = vec![0.; m];
    x[m - 1] = 1.;
    q.apply(&mut x, false, || false)?;
    q.apply(&mut x, true, || false)?;
    for (i, v) in x.iter().enumerate() {
        assert!((v - f64::from(i == m - 1)).abs() < 1e-11);
    }
    Ok(())
}

#[cfg(feature = "rank-research")]
#[test]
fn revision6_nonlinear_point_segment_solve_includes_undamped_rank() -> ProbeResult {
    geometric_chain_ranked(32, true)
}

#[cfg(feature = "rank-research")]
#[test]
fn revision6_completion_rank_difficult_svd_corpus() -> ProbeResult {
    use faer::sparse::linalg::rank::{
        RankQrLimits, RankQrOptions, RankQrReliability, refactorize_rank,
    };
    faer::set_global_parallelism(Par::Seq);
    for (name, matrix, tau) in [
        (
            "large_nullity",
            Mat::from_fn(64, 64, |i, j| if i == j && i < 8 { 1. } else { 0. }),
            1e-9,
        ),
        ("rectangular_rigid", faer::mat![[-1., 0., 1., 0.]], 1e-9),
        (
            "duplicated_equations",
            faer::mat![[1., -1.], [2., -2.]],
            1e-9,
        ),
        (
            "tangent_fixed_circles",
            faer::mat![[2., 0.], [-2., 0.]],
            1e-9,
        ),
        ("toggle", faer::mat![[1., 0.], [0., 0.]], 1e-9),
        (
            "many_large_diagonal_dependencies",
            Mat::from_fn(32, 32, |i, j| {
                if i == j {
                    1.
                } else if j == i + 1 && i % 2 == 0 {
                    1e8
                } else {
                    0.
                }
            }),
            1e-6,
        ),
        (
            "threshold_cluster",
            Mat::from_fn(32, 32, |i, j| {
                if i == j {
                    if i < 8 {
                        1.
                    } else if i < 16 {
                        1.001e-8
                    } else if i < 24 {
                        0.999e-8
                    } else {
                        1e-12
                    }
                } else {
                    0.
                }
            }),
            1e-8,
        ),
    ] {
        for scale in [1e-3, 1., 1e3] {
            for reverse in [false, true] {
                let mut dense = matrix.clone();
                for j in 0..dense.ncols() {
                    for i in 0..dense.nrows() {
                        dense[(i, j)] *= scale;
                    }
                }
                let tau = tau * scale;
                let singular = dense
                    .singular_values()
                    .map_err(|e| format!("reference SVD {e:?}"))?;
                let expected = singular.iter().filter(|&&s| s > tau).count();
                let entries: Vec<_> = (0..dense.ncols())
                    .flat_map(|j| {
                        (0..dense.nrows()).filter_map({
                            let d = &dense;
                            move |i| (d[(i, j)] != 0.).then_some(Triplet::new(i, j, d[(i, j)]))
                        })
                    })
                    .collect();
                let a = SparseColMat::<usize, f64>::try_new_from_triplets(
                    dense.nrows(),
                    dense.ncols(),
                    &entries,
                )?;
                let mut order: Vec<_> = (0..a.ncols()).collect();
                if reverse {
                    order.reverse();
                }
                for seed in [1, 17, 12345] {
                    let result = refactorize_rank(
                        a.as_ref(),
                        &order,
                        tau,
                        RankQrLimits {
                            max_bytes: 1 << 28,
                            max_work: 400_000_000,
                        },
                        RankQrOptions {
                            seed,
                            ..Default::default()
                        },
                        || false,
                    );
                    println!(
                        "R6_DIFFICULT,name={name},scale={scale},reverse={reverse},seed={seed},expected={expected},result={result:?}"
                    );
                    let r = result?;
                    let rank = r.rank.ok_or("missing difficult-case report")?;
                    assert_eq!(
                        rank.reliability,
                        RankQrReliability::EstimatedReliable,
                        "{name}"
                    );
                    assert_eq!(rank.rank, expected, "{name}");
                    assert!(r.factorizations <= 3);
                }
            }
        }
    }
    Ok(())
}

/// Real nonlinear length constraints on a connected rectangular point grid.
#[cfg(feature = "rank-research")]
pub(crate) fn completion_geometric_grid(points: usize) -> ProbeResult {
    use faer::sparse::linalg::rank::{
        RankQrLimits, RankQrOptions, RankQrReliability, refactorize_rank,
    };
    let began = Instant::now();
    let width = points.isqrt();
    if width * width != points || width < 2 {
        return Err("square grid required".into());
    }
    let mut xy: Vec<_> = (0..points)
        .map(|i| {
            [
                (i % width) as f64 + 0.01 * (i as f64).sin(),
                (i / width) as f64 + 0.01 * (i as f64).cos(),
            ]
        })
        .collect();
    let rows = points + width + 2 * width * (width - 1);
    let mut symbolic = None;
    let mut iterations = 0;
    let mut max_residual = 0.;
    let mut converged = false;
    for iteration in 0..10 {
        let mut entries = Vec::new();
        let mut rhs = Mat::zeros(rows, 1);
        let mut row = 0;
        for (i, p) in xy.iter().enumerate() {
            entries.push(Triplet::new(row, 2 * i + 1, 1.));
            rhs[(row, 0)] = (i / width) as f64 - p[1];
            row += 1;
            if i % width == 0 {
                entries.push(Triplet::new(row, 2 * i, 1.));
                rhs[(row, 0)] = -p[0];
                row += 1;
            }
        }
        for i in 0..points {
            for j in [
                if i % width + 1 < width {
                    Some(i + 1)
                } else {
                    None
                },
                if i + width < points {
                    Some(i + width)
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            {
                let dx = xy[j][0] - xy[i][0];
                let dy = xy[j][1] - xy[i][1];
                let length = dx.hypot(dy);
                if length < 1e-12 {
                    return Err("collapsed grid edge".into());
                }
                for (column, value) in [
                    (2 * i, -dx / length),
                    (2 * i + 1, -dy / length),
                    (2 * j, dx / length),
                    (2 * j + 1, dy / length),
                ] {
                    entries.push(Triplet::new(row, column, value));
                }
                rhs[(row, 0)] = 1. - length;
                row += 1;
            }
        }
        assert_eq!(row, rows);
        max_residual = rhs.norm_max();
        if max_residual < 1e-9 {
            converged = true;
            break;
        }
        let a = SparseColMat::<usize, f64>::try_new_from_triplets(rows, 2 * points, &entries)?;
        if symbolic.is_none() {
            symbolic = Some(SymbolicQr::try_new(a.symbolic())?);
        }
        let symbol = symbolic.as_ref().ok_or("missing grid symbolic")?.clone();
        let factor = Instant::now();
        let qr = Qr::try_new_with_symbolic(symbol, a.as_ref())?;
        let factor_ns = factor.elapsed().as_nanos();
        let rank_started = Instant::now();
        let order: Vec<_> = (0..a.ncols()).collect();
        let report = refactorize_rank(
            a.as_ref(),
            &order,
            1e-9,
            RankQrLimits {
                max_bytes: 1 << 30,
                max_work: 400_000_000,
            },
            RankQrOptions::default(),
            || false,
        )?;
        let rank = report.rank.ok_or("missing nonlinear grid rank")?;
        assert_eq!(rank.reliability, RankQrReliability::EstimatedReliable);
        assert_eq!(rank.rank, 2 * points);
        let step = qr.solve_lstsq(&rhs);
        for (i, p) in xy.iter_mut().enumerate() {
            p[0] += step[(2 * i, 0)];
            p[1] += step[(2 * i + 1, 0)];
        }
        iterations = iteration + 1;
        println!(
            "R6_GRID_PHASE,iteration={iteration},equations={rows},nnz={},factor_ns={factor_ns},rank_and_solve_ns={},rank={},qr_max_ns={},svd_max_ns={}",
            a.compute_nnz(),
            rank_started.elapsed().as_nanos(),
            rank.rank,
            report.native_qr_max_ns,
            report.dense_svd_max_ns
        );
    }
    let error = xy
        .iter()
        .enumerate()
        .map(|(i, p)| {
            (p[0] - (i % width) as f64)
                .abs()
                .max((p[1] - (i / width) as f64).abs())
        })
        .fold(0_f64, f64::max);
    assert!(
        converged && max_residual < 1e-9 && error < 1e-6,
        "grid residual={max_residual},error={error}"
    );
    println!(
        "R6_GEOMETRIC_GRID,points={points},entities={},unknowns={},equations={rows},largest_free_block={},eliminations=0,iterations={iterations},residual={max_residual},solution_error={error},total_us={}",
        points + 2 * width * (width - 1),
        2 * points,
        2 * points,
        began.elapsed().as_secs_f64() * 1e6
    );
    Ok(())
}
#[cfg(feature = "rank-research")]
#[test]
fn revision6_completion_rank_nonlinear_grid() -> ProbeResult {
    completion_geometric_grid(16)
}

#[cfg(feature = "rank-research")]
pub(crate) fn completion_rank_workloads(n: usize) -> ProbeResult {
    use faer::{
        dyn_stack::{MemBuffer, MemStack},
        sparse::linalg::{
            SupernodalThreshold,
            qr::{QrSymbolicParams, factorize_symbolic_qr_ordered},
            rank::{RankQrLimits, RankQrOptions, RankQrReliability, refactorize_rank},
        },
    };
    for grid in [false, true] {
        for large_nullity in [false, true] {
            println!("R6_DIFFICULT_BEGIN,grid={grid},large_nullity={large_nullity},n={n}");
            let began = Instant::now();
            let (original, _, _) = fixture(n, grid)?;
            let mut entries = Vec::new();
            for j in 0..n {
                if large_nullity && j % 2 == 1 {
                    continue;
                }
                let source = if !large_nullity && j == n / 3 {
                    0
                } else if !large_nullity && j == 2 * n / 3 {
                    n - 1
                } else {
                    j
                };
                entries.extend(
                    original
                        .row_idx_of_col(source)
                        .zip(original.val_of_col(source))
                        .map(|(i, &v)| Triplet::new(i, j, v)),
                );
                if source != j {
                    entries.extend(
                        original
                            .row_idx_of_col(j)
                            .zip(original.val_of_col(j))
                            .map(|(i, &v)| Triplet::new(i, j, v * 1e-12)),
                    );
                }
            }
            let a =
                SparseColMat::<usize, f64>::try_new_from_triplets(original.nrows(), n, &entries)?;
            let order: Vec<_> = (0..n).rev().collect();
            let report = refactorize_rank(
                a.as_ref(),
                &order,
                1e-9,
                RankQrLimits {
                    max_bytes: 1 << 30,
                    max_work: 400_000_000,
                },
                RankQrOptions {
                    seed: 17,
                    ..Default::default()
                },
                || false,
            )?;
            let rank = report.rank.ok_or("missing difficult C1 bridge")?;
            println!(
                "R6_DIFFICULT_STAGE,stage=rank,rank={},native_qr_max_ns={},svd_max_ns={},admitted_bytes={}",
                rank.rank,
                report.native_qr_max_ns,
                report.dense_svd_max_ns,
                report.numeric_admitted_bytes
            );
            let expected = if large_nullity { n.div_ceil(2) } else { n - 2 };
            assert_eq!(rank.reliability, RankQrReliability::EstimatedReliable);
            assert_eq!(rank.rank, expected);
            let symbolic = factorize_symbolic_qr_ordered(
                a.symbolic(),
                &report.permutation,
                QrSymbolicParams {
                    supernodal_flop_ratio_threshold: SupernodalThreshold::FORCE_SUPERNODAL,
                    ..Default::default()
                },
                1 << 30,
                || false,
            )?;
            let mut indices = vec![0; symbolic.len_idx()];
            println!(
                "R6_DIFFICULT_STAGE,stage=operator_symbolic,indices={},values={}",
                symbolic.len_idx(),
                symbolic.len_val()
            );
            let mut values = vec![0.; symbolic.len_val()];
            let mut scratch = MemBuffer::new(
                symbolic.factorize_numeric_qr_scratch::<f64>(Par::Seq, Default::default()),
            );
            let qr = symbolic.factorize_numeric_qr(
                &mut indices,
                &mut values,
                a.as_ref(),
                Par::Seq,
                MemStack::new(&mut scratch),
                Default::default(),
            );
            let null = qr.right_null_operator(rank.rank, 1 << 30, || false)?;
            let input: Vec<_> = (0..n - rank.rank)
                .map(|i| (i as f64 + 0.5).sin() / ((n - rank.rank) as f64).sqrt())
                .collect();
            let mut output = vec![0.; n];
            null.apply_null(rank.rank, &input, &mut output, false, || false)?;
            let mut residual = vec![0.; a.nrows()];
            for (j, &x) in output.iter().enumerate() {
                for (i, &v) in a.row_idx_of_col(j).zip(a.val_of_col(j)) {
                    residual[i] += v * x;
                }
            }
            let error = residual.iter().fold(0_f64, |s, &v| s.hypot(v));
            assert!(error < 1e-8, "original null residual={error}");
            let mut adjoint = vec![0.; input.len()];
            null.apply_null(rank.rank, &output, &mut adjoint, true, || false)?;
            assert!(
                input
                    .iter()
                    .zip(&adjoint)
                    .all(|(&a, &b)| (a - b).abs() < 1e-10)
            );
            let null_bytes = null.owned_bytes();
            drop(null);
            drop(residual);
            drop(output);
            drop(adjoint);
            drop(scratch);
            drop(values);
            drop(indices);
            drop(symbolic);
            drop(a);
            drop(original);
            drop(entries);
            let elapsed = began.elapsed().as_secs_f64() * 1e6;
            println!(
                "R6_DIFFICULT_C1,grid={grid},large_nullity={large_nullity},n={n},rank={},null_residual={error},whole_job_us={elapsed},native_qr_max_ns={},svd_max_ns={},admitted_rank_bytes={},owned_null_bytes={null_bytes},under_5s={}",
                rank.rank,
                report.native_qr_max_ns,
                report.dense_svd_max_ns,
                report.numeric_admitted_bytes,
                elapsed <= 5e6
            );
        }
    }
    Ok(())
}
#[cfg(feature = "rank-research")]
#[test]
fn revision6_completion_rank_large_nullity_operators_small_reference() -> ProbeResult {
    completion_rank_workloads(64)
}

#[cfg(feature = "rank-research")]
#[test]
fn revision6_completion_rank_front_contract_reconstructs_original_equations() -> ProbeResult {
    use faer::sparse::linalg::rank::{
        RankQrError, RankQrLimits, RankQrOptions, RankQrReliability,
        front::{FrontProgress, RankQrContinuation},
    };
    for grid in [false, true] {
        let (a, _, _) = fixture(16, grid)?;
        let order: Vec<_> = (0..16).rev().collect();
        let mut job = RankQrContinuation::new(
            a.as_ref(),
            &order,
            1e-9,
            RankQrLimits {
                max_bytes: 1 << 27,
                max_work: 400_000_000,
            },
        )?;
        while job.advance(1, || false)? != FrontProgress::Complete {}
        let rank = job.assess(RankQrOptions::default(), || false)?;
        assert_eq!(rank.reliability, RankQrReliability::EstimatedReliable);
        assert_eq!(rank.rank, 16);
        for j in 0..16 {
            let mut x = vec![0.; a.nrows()];
            for (i, &v) in a.row_idx_of_col(j).zip(a.val_of_col(j)) {
                x[i] = v;
            }
            job.apply_q(&mut x, true, || false)?;
            job.visit_r(|i, k, v| {
                if k == j {
                    x[i] -= v;
                }
            })?;
            assert!(x.iter().all(|v| v.abs() < 1e-10), "frontal QtA-R col{j}");
        }
        let mut x: Vec<_> = (0..a.nrows()).map(|i| (i as f64).sin()).collect();
        let original = x.clone();
        job.apply_q(&mut x, true, || false)?;
        job.apply_q(&mut x, false, || false)?;
        assert!(x.iter().zip(original).all(|(&a, b)| (a - b).abs() < 1e-10));
        let mut cancelled = RankQrContinuation::new(
            a.as_ref(),
            &order,
            1e-9,
            RankQrLimits {
                max_bytes: 1 << 27,
                max_work: 400_000_000,
            },
        )?;
        assert_eq!(cancelled.advance(1, || false)?, FrontProgress::Pending);
        assert_eq!(
            cancelled.advance(1, || true).err(),
            Some(RankQrError::Cancelled)
        );
        assert!(cancelled.apply_q(&mut x, false, || false).is_err());
    }
    Ok(())
}

#[cfg(feature = "rank-research")]
#[test]
fn revision6_completion_rank_semantics_do_not_confuse_rank_convergence_or_mobility() -> ProbeResult
{
    use faer::sparse::linalg::rank::{
        RankQrLimits, RankQrOptions, RankQrReliability, refactorize_rank,
    };
    for (name, dense, expected, mobility) in [
        (
            "regular_fixed_point",
            faer::mat![[1., 0.], [0., 1.]],
            2,
            "RegularLocalDimension0",
        ),
        (
            "duplicate_equalities",
            faer::mat![[1., -1.], [2., -2.]],
            1,
            "RegularLocalDimension1",
        ),
        (
            "rigid_segment",
            faer::mat![[-1., 0., 1., 0.]],
            1,
            "RegularLocalDimension3",
        ),
        (
            "toggle_and_tangent_fixed_circles",
            faer::mat![[2., 0.], [-2., 0.]],
            1,
            "Singular_infinitesimal1_finite0_analytic",
        ),
        (
            "active_contact_bound",
            faer::mat![[0., 1.]],
            1,
            "Boundary_feasible_cone_not_regular_DOF",
        ),
        (
            "contradictory_equalities",
            faer::mat![[1.], [1.]],
            1,
            "Inconsistent_despite_full_column_rank",
        ),
    ] {
        let entries: Vec<_> = (0..dense.ncols())
            .flat_map(|j| {
                (0..dense.nrows()).filter_map({
                    let d = &dense;
                    move |i| (d[(i, j)] != 0.).then_some(Triplet::new(i, j, d[(i, j)]))
                })
            })
            .collect();
        let a = SparseColMat::<usize, f64>::try_new_from_triplets(
            dense.nrows(),
            dense.ncols(),
            &entries,
        )?;
        let order: Vec<_> = (0..dense.ncols()).collect();
        let report = refactorize_rank(
            a.as_ref(),
            &order,
            1e-9,
            RankQrLimits {
                max_bytes: 1 << 26,
                max_work: 400_000_000,
            },
            RankQrOptions::default(),
            || false,
        )?;
        let rank = report.rank.ok_or("no semantic rank report")?;
        assert_eq!(rank.reliability, RankQrReliability::EstimatedReliable);
        assert_eq!(rank.rank, expected);
        let reference = dense
            .singular_values()
            .map_err(|e| format!("semantic SVD {e:?}"))?;
        assert_eq!(reference.iter().filter(|&&s| s > 1e-9).count(), expected);
        if name == "contradictory_equalities" {
            // Exact rational witness [-1,1]^T A=0 and [-1,1]^T [0,1]=1.
            assert_eq!(-dense[(0, 0)] + dense[(1, 0)], 0.);
            let rhs = faer::mat![[0.], [1.]];
            let x = Qr::try_new_with_symbolic(SymbolicQr::try_new(a.symbolic())?, a.as_ref())?
                .solve_lstsq(&rhs);
            assert!((x[(0, 0)] - 0.5).abs() < 1e-12);
            assert!((x[(0, 0)] - rhs[(0, 0)]).abs() > 1e-7);
        }
        if name == "active_contact_bound" {
            let inward = [1., 0.];
            let outward = [-1., 0.];
            assert!(inward[0] >= 0.);
            assert!(outward[0] < 0.);
            assert_eq!(dense[(0, 0)] * outward[0] + dense[(0, 1)] * outward[1], 0.);
        }
        // Tangent unit-circle equations subtract to x=0, then y²=0;
        // the null direction therefore does not extend to a finite motion.
        println!(
            "R6_SEMANTICS,{name},rank={},infinitesimal_nullity={},classification={mobility},rank_does_not_certify_finite_mobility=true",
            rank.rank,
            dense.ncols() - rank.rank
        );
    }
    Ok(())
}

#[cfg(feature = "rank-research")]
#[test]
fn revision6_completion_rank_front_compression_keeps_q_r_and_deferral() -> ProbeResult {
    use faer::sparse::linalg::rank::{
        RankQrLimits, RankQrOptions, RankQrReliability,
        front::{FrontProgress, RankQrContinuation},
    };
    for dependent in [false, true] {
        let dense = Mat::from_fn(192, 17, |i, j| {
            if j == 0 || i == if dependent && j == 2 { 1 } else { j } {
                1.
            } else {
                0.
            }
        });
        let entries: Vec<_> = (0..17)
            .flat_map(|j| {
                (0..192).filter_map({
                    let a = &dense;
                    move |i| (a[(i, j)] != 0.).then_some(Triplet::new(i, j, a[(i, j)]))
                })
            })
            .collect();
        let a = SparseColMat::<usize, f64>::try_new_from_triplets(192, 17, &entries)?;
        let order: Vec<_> = (0..17).collect();
        let mut job = RankQrContinuation::new(
            a.as_ref(),
            &order,
            1e-9,
            RankQrLimits {
                max_bytes: 1 << 27,
                max_work: 400_000_000,
            },
        )?;
        while job.advance(1, || false)? != FrontProgress::Complete {}
        let rank = job.assess(RankQrOptions::default(), || false)?;
        assert_eq!(rank.reliability, RankQrReliability::EstimatedReliable);
        assert_eq!(rank.rank, 17 - usize::from(dependent));
        let reference = dense
            .singular_values()
            .map_err(|e| format!("compression SVD {e:?}"))?;
        assert_eq!(reference.iter().filter(|&&s| s > 1e-9).count(), rank.rank);
        for j in 0..17 {
            let mut column: Vec<_> = (0..192).map(|i| dense[(i, j)]).collect();
            job.apply_q(&mut column, true, || false)?;
            job.visit_r(|i, k, v| {
                if k == j {
                    column[i] -= v;
                }
            })?;
            assert!(
                column.iter().all(|v| v.abs() < 1e-10),
                "compression QtA-R,{dependent},{j}"
            );
        }
        let mut x: Vec<_> = (0..192).map(|i| (i as f64).cos()).collect();
        let saved = x.clone();
        job.apply_q(&mut x, true, || false)?;
        job.apply_q(&mut x, false, || false)?;
        assert!(x.iter().zip(saved).all(|(&a, b)| (a - b).abs() < 1e-10));
    }
    Ok(())
}
