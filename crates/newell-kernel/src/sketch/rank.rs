// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Test-only ACL translation for the M5.0 vendor rank candidate.
use faer::sparse::linalg::rank::{RankQrReliability, RankQrReport};

/// Equality-Jacobian assessment; finite mobility remains a separate analysis.
#[derive(Debug, PartialEq)]
pub(super) enum RankAssessment {
    Estimated {
        rank: usize,
        infinitesimal_nullity: usize,
    },
    Uncertain,
}

/// The input report must concern the original undamped equality Jacobian after
/// documented scaling/elimination. No vendor success flag certifies finite DOF,
/// constraint consistency, singular mobility or feasibility at active bounds.
pub(super) fn translate(report: &RankQrReport, rows: usize, unknowns: usize) -> RankAssessment {
    let valid = report.rank <= rows.min(unknowns)
        && report.reliability == RankQrReliability::EstimatedReliable
        && report.threshold.is_finite()
        && report.threshold > 0.
        && report.upper.is_finite()
        && report.upper >= 0.
        && report.upper <= report.threshold
        && (report.rank == 0 || report.lower.is_finite() && report.lower > report.threshold);
    if valid {
        RankAssessment::Estimated {
            rank: report.rank,
            infinitesimal_nullity: unknowns - report.rank,
        }
    } else {
        RankAssessment::Uncertain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rank_translation_preserves_uncertainty_and_does_not_infer_mobility() {
        let mut report = RankQrReport {
            rank: 1,
            preliminary_rank: 1,
            threshold: 1e-9,
            reliability: RankQrReliability::EstimatedReliable,
            lower: 1.,
            upper: 0.,
            perturbation: 0.,
            boundary_residual: 0.,
            null_residual: 0.,
            iterations: 1,
            block: 1,
            seed: 1,
            admitted_bytes: 1024,
        };
        // Two fixed tangent circles: one infinitesimal freedom, isolated real
        // intersection. The ACL publishes no finite-mobility claim.
        assert_eq!(
            translate(&report, 2, 2),
            RankAssessment::Estimated {
                rank: 1,
                infinitesimal_nullity: 1
            }
        );
        report.reliability = RankQrReliability::Uncertain;
        assert_eq!(translate(&report, 2, 2), RankAssessment::Uncertain);
        report.reliability = RankQrReliability::EstimatedReliable;
        report.upper = 1e-8;
        assert_eq!(translate(&report, 2, 2), RankAssessment::Uncertain);
        report.upper = 0.;
        report.rank = 3;
        assert_eq!(translate(&report, 2, 2), RankAssessment::Uncertain);
        report.rank = 1;
        report.lower = f64::NAN;
        assert_eq!(translate(&report, 2, 2), RankAssessment::Uncertain);
    }
}
