// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! M5.0 reproducible backend probes. No production Sketch implementation.
#[path = "../tests/it/sketch_curves.rs"]
mod sketch_curves;
#[path = "../tests/it/sketch_regions.rs"]
mod sketch_regions;
#[path = "../tests/it/sketch_solver.rs"]
mod sketch_solver;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("NEWELL_M50_CONTACT").is_some() {
        return sketch_curves::opaque_contact();
    }
    faer::set_global_parallelism(faer::Par::Seq);
    println!("M5.0,seed=0,iterations=500,parallelism=Seq,units=mm");
    #[cfg(feature = "m50-feasibility")]
    if std::env::var("NEWELL_M50_PHASE").as_deref() == Ok("rank") {
        let runs = std::env::var("NEWELL_M50_RANK_RUNS")
            .ok()
            .map(|s| s.parse())
            .transpose()?
            .unwrap_or(500);
        return sketch_solver::rank_large(runs, 10_000);
    }
    if std::env::var("NEWELL_M50_PHASE").as_deref() == Ok("regions") {
        return sketch_regions::run(500);
    }

    if std::env::var("NEWELL_M50_PHASE").as_deref() == Ok("cancellation") {
        return sketch_solver::cancellation();
    }
    if std::env::var("NEWELL_M50_PHASE").as_deref() == Ok("scale") {
        return sketch_solver::scale_workloads(500);
    }
    if std::env::var("NEWELL_M50_PHASE").as_deref() == Ok("curves") {
        return sketch_curves::run(500);
    }
    if std::env::var("NEWELL_M50_PHASE").as_deref() == Ok("qr-config") {
        let runs = std::env::var("NEWELL_M50_QR_RUNS")
            .ok()
            .map(|s| s.parse())
            .transpose()?
            .unwrap_or(500);
        return sketch_solver::configured_qr(runs, 10_000);
    }
    if std::env::var("NEWELL_M50_PHASE").as_deref() == Ok("geometric") {
        let mut samples = Vec::new();
        for i in 0..=500 {
            let began = std::time::Instant::now();
            sketch_solver::geometric_chain(10_002)?;
            if i > 0 {
                samples.push(began.elapsed().as_secs_f64() * 1e6);
            }
        }
        sketch_solver::distribution("r3_geometric_chain_full_job", samples);
        return Ok(());
    }
    sketch_solver::scale_workloads(500)?;
    sketch_solver::dense_cases(500)?;
    sketch_solver::gcs_reference(500)?;
    sketch_solver::sparse_cases(500, true)?;
    sketch_solver::cancellation()?;
    sketch_solver::spatial(500)?;
    sketch_curves::run(500)?;
    sketch_regions::run(500)?;
    Ok(())
}
