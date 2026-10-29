// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Exact input curves. Measured hits do not certify completeness or arrangements.
use super::sketch_solver::{ProbeResult, cancel_boundary, distribution};
use brepkit_math::{
    nurbs::{bezier_clip::curve_curve_intersect_full, curve::NurbsCurve},
    tolerance::Tolerance,
    vec::Point3,
};
use std::{hint::black_box, time::Instant};
fn curve(
    degree: usize,
    points: &[[f64; 2]],
    weights: Vec<f64>,
    shift: f64,
) -> Result<NurbsCurve, Box<dyn std::error::Error>> {
    let knots = std::iter::repeat_n(0., degree + 1)
        .chain(std::iter::repeat_n(1., degree + 1))
        .collect();
    Ok(NurbsCurve::new(
        degree,
        knots,
        points
            .iter()
            .map(|p| Point3::new(p[0] + shift, p[1] + shift, 0.))
            .collect(),
        weights,
    )?)
}
fn families(shift: f64) -> Result<Vec<(&'static str, NurbsCurve)>, Box<dyn std::error::Error>> {
    Ok(vec![
        (
            "segment",
            curve(1, &[[-1., 0.], [1., 0.]], vec![1.; 2], shift)?,
        ),
        (
            "circle_arc",
            curve(
                2,
                &[[1., 0.], [1., 1.], [0., 1.]],
                vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                shift,
            )?,
        ),
        (
            "ellipse_arc",
            curve(
                2,
                &[[2., 0.], [2., 1.], [0., 1.]],
                vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                shift,
            )?,
        ),
        (
            "rational_conic",
            curve(
                2,
                &[[-1., 1.], [0., -1.], [1., 1.]],
                vec![1., 0.5, 1.],
                shift,
            )?,
        ),
        (
            "bezier_bspline",
            curve(
                3,
                &[[-1., -1.], [1., -1.], [-1., 1.], [1., 1.]],
                vec![1.; 4],
                shift,
            )?,
        ),
    ])
}
pub(crate) fn run(iterations: usize) -> ProbeResult {
    for shift in [0., 1e9] {
        let curves = families(shift)?;
        for (i, (an, a)) in curves.iter().enumerate() {
            for (bn, b) in curves.iter().skip(i) {
                let t = Instant::now();
                let result = curve_curve_intersect_full(a, b, 1e-7)?;
                let error = result
                    .hits
                    .iter()
                    .map(|h| (a.evaluate(h.u1) - b.evaluate(h.u2)).length())
                    .fold(0., f64::max);
                println!(
                    "PAIR,{an},{bn},shift={shift:e},hits={},overlaps={},max_residual={error:e},us={:.3},completeness=Unproven",
                    result.hits.len(),
                    result.overlaps.len(),
                    t.elapsed().as_secs_f64() * 1e6
                );
            }
        }
        // Known analytic crossing / tangent / near-contact / disjoint / overlap.
        let line = curve(1, &[[-2., 0.], [2., 0.]], vec![1.; 2], shift)?;
        let parabola = curve(2, &[[-1., 1.], [0., -1.], [1., 1.]], vec![1.; 3], shift)?;
        for (name, points, expected_hits, expected_overlap) in [
            ("cross", vec![[0., -1.], [0., 1.]], 1, 0),
            ("partial_overlap", vec![[0., 0.], [3., 0.]], 0, 1),
            ("reversed_overlap", vec![[1., 0.], [-1., 0.]], 0, 1),
            ("T_contact", vec![[0., 0.], [0., 1.]], 1, 0),
        ] {
            if name == "partial_overlap" || name == "reversed_overlap" {
                isolated_contact(name, shift)?;
                continue;
            }
            let b = curve(1, &points, vec![1.; 2], shift)?;
            let r = curve_curve_intersect_full(&line, &b, 1e-7)?;
            println!(
                "ANALYTIC,{name},shift={shift:e},hits={},overlaps={},expected_hits={expected_hits},expected_overlap={expected_overlap},matches={}",
                r.hits.len(),
                r.overlaps.len(),
                r.hits.len() == expected_hits && r.overlaps.len() == expected_overlap
            );
        }
        for delta in [0., 1e-6, -1e-6] {
            let b = curve(1, &[[-2., delta], [2., delta]], vec![1.; 2], shift)?;
            let t = Instant::now();
            let r = curve_curve_intersect_full(&parabola, &b, 1e-7)?;
            let expected = if delta == 0. {
                1
            } else if delta > 0. {
                2
            } else {
                0
            };
            println!(
                "ANALYTIC,parabola_contact,shift={shift:e},delta={delta:e},hits={},expected={expected},matches={},us={:.3}",
                r.hits.len(),
                r.hits.len() == expected,
                t.elapsed().as_secs_f64() * 1e6
            );
        }
        let mut times = Vec::new();
        for _ in 0..iterations {
            let t = Instant::now();
            black_box(curve_curve_intersect_full(&parabola, &line, 1e-7)?);
            times.push(t.elapsed().as_secs_f64() * 1e6);
        }
        distribution(&format!("intersection_tangent_shift_{shift:e}"), times);
        cancel_boundary(&format!("intersection_tangent_shift_{shift:e}"), || {
            black_box(curve_curve_intersect_full(&parabola, &line, 1e-7)).ok();
        })?;
        isolated_contact("periodic_seam", shift)?;
        isolated_contact("figure_eight", shift)?;
        let tol = Tolerance::new();
        let stored = (shift + 1e-5) - shift;
        println!(
            "TOLERANCE,shift={shift:e},linear={},angular={},relative={},tiny_gap_stored={stored:e},kernel_approx_equal={},local_policy_equal={},ulp={:e}",
            tol.linear,
            tol.angular,
            tol.relative,
            tol.approx_eq(shift, shift + 1e-5),
            stored.abs() <= 1e-7,
            f64::from_bits(shift.to_bits() + 1) - shift
        );
    }
    font_probe()?;
    Ok(())
}
fn font_probe() -> ProbeResult {
    use ab_glyph::{Font, FontRef};
    let font = FontRef::try_from_slice(include_bytes!("../../../../assets/FiraCode-Regular.ttf"))?;
    for c in ['O', 'B', '8'] {
        let outline = font
            .outline(font.glyph_id(c))
            .ok_or("missing font outline")?;
        println!(
            "FONT,{c},outline_segments={},bounds={:?},font_units_per_em={:?},contour_orientation_and_precision=Unproven",
            outline.curves.len(),
            outline.bounds,
            font.units_per_em()
        );
    }
    Ok(())
}
#[test]
fn segment_crossing_has_independent_analytic_parameters() -> ProbeResult {
    let a = curve(1, &[[-1., 0.], [1., 0.]], vec![1.; 2], 0.)?;
    let b = curve(1, &[[0., -1.], [0., 1.]], vec![1.; 2], 0.)?;
    let r = curve_curve_intersect_full(&a, &b, 1e-7)?;
    assert_eq!(r.hits.len(), 1);
    assert!(r.overlaps.is_empty());
    assert!((r.hits[0].u1 - 0.5).abs() < 1e-7 && (r.hits[0].u2 - 0.5).abs() < 1e-7);
    Ok(())
}

#[test]
fn curve_capability_dossier_executes() -> ProbeResult {
    run(1)
}

/// Process isolation bounds a feasibility experiment. Killing this child is NOT
/// cooperative cancellation and does not make the backend suitable for M5.
pub(crate) fn opaque_contact() -> ProbeResult {
    let name = std::env::var("NEWELL_M50_CONTACT")?;
    let shift = std::env::var("NEWELL_M50_SHIFT")?.parse::<f64>()?;
    if name == "figure_eight" {
        let original = curve(
            3,
            &[
                [3., -6.],
                [-7. / 3., 26. / 3.],
                [-7. / 3., -26. / 3.],
                [3., 6.],
            ],
            vec![1.; 4],
            shift,
        )?;
        let (left, right) = brepkit_math::nurbs::knot_ops::curve_split(&original, 0.5)?;
        println!("OPAQUE_START,figure_eight,shift={shift:e}");
        use std::io::Write;
        std::io::stdout().flush()?;
        let time = Instant::now();
        let r = curve_curve_intersect_full(&left, &right, 1e-7)?;
        println!(
            "SPECIAL,figure_eight,shift={shift:e},hits={},overlaps={},us={:.3},expected_nonadjacent_parameters=0.25_0.75",
            r.hits.len(),
            r.overlaps.len(),
            time.elapsed().as_secs_f64() * 1e6
        );
        for h in r.hits {
            println!(
                "SPECIAL_PARAMETERS,{},{},residual={:e}",
                h.u1,
                h.u2,
                (left.evaluate(h.u1) - right.evaluate(h.u2)).length()
            );
        }
        return Ok(());
    }
    if name == "periodic_seam" {
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
        ];
        let circle = NurbsCurve::new(
            2,
            vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
            controls
                .map(|p| Point3::new(p[0] + shift, p[1] + shift, 0.))
                .into(),
            vec![1., w, 1., w, 1., w, 1., w, 1.],
        )?;
        let line = curve(1, &[[-2., 0.], [2., 0.]], vec![1.; 2], shift)?;
        println!("OPAQUE_START,periodic_seam,shift={shift:e}");
        use std::io::Write;
        std::io::stdout().flush()?;
        let time = Instant::now();
        let r = curve_curve_intersect_full(&circle, &line, 1e-7)?;
        println!(
            "SPECIAL,periodic_seam,shift={shift:e},hits={},overlaps={},expected_distinct_geometric_points=2,us={:.3}",
            r.hits.len(),
            r.overlaps.len(),
            time.elapsed().as_secs_f64() * 1e6
        );
        return Ok(());
    }
    let a = curve(1, &[[-2., 0.], [2., 0.]], vec![1.; 2], shift)?;
    let points = if name == "partial_overlap" {
        [[0., 0.], [3., 0.]]
    } else {
        [[1., 0.], [-1., 0.]]
    };
    let b = curve(1, &points, vec![1.; 2], shift)?;
    println!("OPAQUE_START,{name},shift={shift:e}");
    use std::io::Write;
    std::io::stdout().flush()?;
    let t = Instant::now();
    let result = curve_curve_intersect_full(&a, &b, 1e-7)?;
    println!(
        "OPAQUE_RESULT,{name},hits={},overlaps={},us={:.3},expected_overlap=1",
        result.hits.len(),
        result.overlaps.len(),
        t.elapsed().as_secs_f64() * 1e6
    );
    Ok(())
}
fn isolated_contact(name: &str, shift: f64) -> ProbeResult {
    use std::{
        process::{Command, Stdio},
        time::Duration,
    };
    let exe = std::env::current_exe()?;
    let mut command = Command::new(&exe);
    if !exe
        .file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with("sketch-"))
    {
        command.args([
            "--exact",
            "sketch_curves::opaque_contact_child",
            "--ignored",
            "--nocapture",
        ]);
    }
    let t = Instant::now();
    let mut child = command
        .env("NEWELL_M50_CONTACT", name)
        .env("NEWELL_M50_SHIFT", shift.to_string())
        .stdout(Stdio::piped())
        .spawn()?;
    loop {
        if child.try_wait()?.is_some() {
            let output = child.wait_with_output()?;
            println!("{}", String::from_utf8_lossy(&output.stdout));
            return Ok(());
        }
        if t.elapsed() > Duration::from_secs(2) {
            child.kill()?;
            let output = child.wait_with_output()?;
            println!("{}", String::from_utf8_lossy(&output.stdout));
            println!(
                "WATCHDOG,{name},shift={shift:e},wall_us={:.3},status=Unsupported_uninterruptible_timeout,process_kill_is_not_cancellation=true",
                t.elapsed().as_secs_f64() * 1e6
            );
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
#[ignore = "Child entry point: called with explicit watchdog by the parent probe"]
fn opaque_contact_child() -> ProbeResult {
    opaque_contact()
}

#[cfg(feature = "m50-feasibility")]
#[test]
fn vendor_candidate_exact_affine_corpus_and_control_outcomes() -> ProbeResult {
    use brepkit_math::nurbs::{
        certified_intersection::intersect,
        certified_overlap::{Outcome, classify},
    };
    let mut times = Vec::new();
    for shift in [0., 1e9] {
        let a = curve(1, &[[-2., 0.], [2., 0.]], vec![1.; 2], shift)?;
        for points in [[[0., 0.], [3., 0.]], [[1., 0.], [-1., 0.]]] {
            let b = curve(1, &points, vec![1.; 2], shift)?;
            for (first, second) in [(&a, &b), (&b, &a)] {
                for _ in 0..500 {
                    let t = Instant::now();
                    let result = intersect(first, second, 1, false);
                    times.push(t.elapsed().as_secs_f64() * 1e6);
                    let Outcome::Overlap(ends) = result else {
                        return Err("missing exact overlap".into());
                    };
                    for end in ends {
                        let u = end.a.numerator as f64 / end.a.denominator as f64;
                        let v = end.b.numerator as f64 / end.b.denominator as f64;
                        assert!((first.evaluate(u) - second.evaluate(v)).length() < 1e-7);
                        for (value, bound) in [(u, end.a.enclosure()), (v, end.b.enclosure())] {
                            assert!(bound[0] <= value && value <= bound[1]);
                        }
                    }
                }
            }
        }
        for (b, point) in [
            ([[0., -1.], [0., 1.]], true),
            ([[2., 0.], [2., 1.]], true),
            ([[3., -1.], [3., 1.]], false),
        ] {
            let b = curve(1, &b, vec![1.; 2], shift)?;
            assert_eq!(
                matches!(intersect(&a, &b, 1, false), Outcome::Point(_)),
                point
            );
        }
        assert_eq!(intersect(&a, &a, 0, false), Outcome::BudgetExceeded);
        assert_eq!(intersect(&a, &a, 1, true), Outcome::Cancelled);
        let conic = curve(2, &[[-1., 1.], [0., -1.], [1., 1.]], vec![1.; 3], shift)?;
        assert_eq!(intersect(&a, &conic, 1, false), Outcome::Unresolved);
    }
    assert_eq!(
        classify([[0.1, 0.], [1., 0.]], [[0., 0.], [2., 0.]], 1, false),
        Outcome::Unresolved
    );
    assert_eq!(
        classify([[0., 0.], [0., 0.]], [[0., 0.], [1., 0.]], 1, false),
        Outcome::Unresolved
    );
    // Exact integer limits exercise i128 predicate and overlap mapping bounds.
    let l = (1_u64 << 40) as f64;
    assert!(matches!(
        classify([[-l, -l], [l, l]], [[-l, l], [l, -l]], 1, false),
        Outcome::Point(_)
    ));
    assert!(matches!(
        classify([[-l, 0.], [l, 0.]], [[l, 0.], [-l, 0.]], 1, false),
        Outcome::Overlap(_)
    ));
    distribution("V3_exact_affine_admitted_domain", times);
    Ok(())
}

#[cfg(feature = "m50-feasibility")]
#[test]
fn vendor_candidate_quadratic_tangency_and_near_crossings_are_enclosed() -> ProbeResult {
    use brepkit_math::nurbs::certified_intersection::{QuadraticOutcome, horizontal_quadratic};
    for shift in [0., 1e9] {
        let parabola = curve(2, &[[-1., 1.], [0., -1.], [1., 1.]], vec![1.; 3], shift)?;
        for delta in [0., 1e-6, -1e-6] {
            let line = curve(1, &[[-2., delta], [2., delta]], vec![1.; 2], shift)?;
            let actual_delta = line.control_points()[0].y() - shift;
            let result = horizontal_quadratic(&parabola, &line, 1e-7, 1, false);
            let QuadraticOutcome::Complete(hits) = result else {
                return Err(format!("unresolved test contact {shift} {delta}").into());
            };
            let expected = if delta == 0. {
                vec![0.5]
            } else if delta > 0. {
                vec![
                    (1. - actual_delta.sqrt()) / 2.,
                    (1. + actual_delta.sqrt()) / 2.,
                ]
            } else {
                vec![]
            };
            assert_eq!(hits.len(), expected.len());
            for (hit, t) in hits.iter().zip(expected) {
                assert!(hit.quadratic[0] <= t && t <= hit.quadratic[1]);
                assert_eq!(hit.multiplicity, if delta == 0. { 2 } else { 1 });
                let u = (hit.quadratic[0] + hit.quadratic[1]) / 2.;
                let v = (hit.segment[0] + hit.segment[1]) / 2.;
                assert!((parabola.evaluate(u) - line.evaluate(v)).length() < 3e-7);
            }
            println!(
                "V3_QUADRATIC,shift={shift:e},delta={delta:e},hits={hits:?},scope=horizontal_quadratic_only"
            );
        }
    }
    Ok(())
}

#[cfg(feature = "m50-feasibility")]
#[test]
fn m50_candidate_overlap_preserves_nonunit_domains_and_rational_reversal() -> ProbeResult {
    use brepkit_math::nurbs::certified_intersection::{MappedOutcome, intersect_mapped};
    for shift in [0., 1e9] {
        for degree in [1, 2, 3] {
            let controls: Vec<_> = (0..=degree)
                .map(|i| Point3::new(shift + 2. * i as f64, shift + (i * i) as f64, 0.))
                .collect();
            let weights: Vec<_> = (0..=degree).map(|i| 1. + i as f64).collect();
            let knots = |lo, hi| {
                let mut v = vec![lo; degree + 1];
                v.extend(vec![hi; degree + 1]);
                v
            };
            let a = NurbsCurve::new(degree, knots(2., 6.), controls.clone(), weights.clone())?;
            let b = NurbsCurve::new(
                degree,
                knots(-3., 5.),
                controls.into_iter().rev().collect(),
                weights.into_iter().rev().collect(),
            )?;
            for (first, second) in [(&a, &b), (&b, &a)] {
                let MappedOutcome::Overlap(map) = intersect_mapped(first, second, 100, false)
                else {
                    return Err("exact control-net reversal was not recognized".into());
                };
                assert_eq!(map[0].a[0], first.domain().0);
                assert_eq!(map[1].a[0], first.domain().1);
                assert_eq!(map[0].b[0], second.domain().1);
                for i in 0..=16 {
                    let f = i as f64 / 16.;
                    let u = (1. - f) * map[0].a[0] + f * map[1].a[0];
                    let v = (1. - f) * map[0].b[0] + f * map[1].b[0];
                    assert!((first.evaluate(u) - second.evaluate(v)).length() < 4e-7);
                }
                assert!(matches!(
                    intersect_mapped(first, second, 100, true),
                    MappedOutcome::Cancelled
                ));
                assert!(matches!(
                    intersect_mapped(first, second, 0, false),
                    MappedOutcome::BudgetExceeded
                ));
            }
        }
        let a = NurbsCurve::new(
            1,
            vec![2., 2., 6., 6.],
            vec![
                Point3::new(shift, shift, 0.),
                Point3::new(shift + 4., shift, 0.),
            ],
            vec![1.; 2],
        )?;
        let b = NurbsCurve::new(
            1,
            vec![-3., -3., 5., 5.],
            vec![
                Point3::new(shift + 6., shift, 0.),
                Point3::new(shift + 2., shift, 0.),
            ],
            vec![1.; 2],
        )?;
        let MappedOutcome::Overlap(map) = intersect_mapped(&a, &b, 4, false) else {
            return Err("affine original-domain overlap missing".into());
        };
        for (pair, (u, v)) in map.iter().zip([(4., 5.), (6., 1.)]) {
            assert!(pair.a[0] <= u && u <= pair.a[1]);
            assert!(pair.b[0] <= v && v <= pair.b[1]);
            assert!((a.evaluate(u) - b.evaluate(v)).length() < 1e-12);
        }
    }
    Ok(())
}
