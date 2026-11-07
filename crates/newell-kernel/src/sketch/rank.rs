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
pub enum RankAssessment {
    Estimated {
        rank: usize,
        infinitesimal_nullity: usize,
    },
    Uncertain,
}

/// The input report must concern the original undamped equality Jacobian after
/// documented scaling/elimination. No vendor success flag certifies finite DOF,
/// constraint consistency, singular mobility or feasibility at active bounds.
pub fn translate(report: &RankQrReport, rows: usize, unknowns: usize) -> RankAssessment {
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

/// Feature-only identity of one undamped equality matrix, including its inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankJobIdentity {
    pub source: u64,
    pub source_revision: u64,
    pub parameter_revision: u64,
    pub matrix_revision: u64,
    pub scaling_revision: u64,
}
/// Exact affine reconstruction of one eliminated original variable.
#[derive(Debug, Clone, PartialEq)]
pub struct RankElimination {
    pub original_index: usize,
    pub offset: f64,
    pub reduced_terms: Vec<(usize, f64)>,
}
/// Newell-owned provenance, kept with the report rather than a lossy hash.
#[derive(Debug, Clone, PartialEq)]
pub struct RankInputProvenance {
    pub identity: RankJobIdentity,
    pub original_variable_ids: Vec<u64>,
    pub equation_ids: Vec<u64>,
    pub retained_variables: Vec<usize>,
    pub eliminated: Vec<RankElimination>,
    pub variable_scaling: Vec<f64>,
    pub equation_scaling: Vec<f64>,
    pub threshold: f64,
    pub undamped_equalities: bool,
}
impl RankInputProvenance {
    /// Identity maps for a named, uneliminated experiment. IDs are fixture-local
    /// under identity.source, never indices into a mutable production document.
    pub fn uneliminated(
        identity: RankJobIdentity,
        rows: usize,
        columns: usize,
        threshold: f64,
    ) -> Self {
        Self {
            identity,
            original_variable_ids: (0..columns as u64).collect(),
            equation_ids: (0..rows as u64).collect(),
            retained_variables: (0..columns).collect(),
            eliminated: Vec::new(),
            variable_scaling: vec![1.; columns],
            equation_scaling: vec![1.; rows],
            threshold,
            undamped_equalities: true,
        }
    }
    /// Validate all shape/scaling/reconstruction maps before numeric consumption.
    pub fn valid_for_shape(&self, rows: usize, columns: usize) -> bool {
        let unique = |ids: &[u64]| {
            let set: std::collections::BTreeSet<_> = ids.iter().collect();
            set.len() == ids.len()
        };
        if !self.undamped_equalities
            || self.identity.source == 0
            || !self.threshold.is_finite()
            || self.threshold <= 0.
            || self.retained_variables.len() != columns
            || self.equation_ids.len() != rows
            || self.variable_scaling.len() != columns
            || self.equation_scaling.len() != rows
            || !unique(&self.original_variable_ids)
            || !unique(&self.equation_ids)
            || self
                .variable_scaling
                .iter()
                .chain(&self.equation_scaling)
                .any(|s| !s.is_finite() || *s <= 0.)
        {
            return false;
        }
        let mut covered = vec![false; self.original_variable_ids.len()];
        for &i in &self.retained_variables {
            if i >= covered.len() || covered[i] {
                return false;
            }
            covered[i] = true;
        }
        for eliminated in &self.eliminated {
            if eliminated.original_index >= covered.len()
                || covered[eliminated.original_index]
                || !eliminated.offset.is_finite()
                || eliminated
                    .reduced_terms
                    .iter()
                    .any(|&(j, c)| j >= columns || !c.is_finite())
            {
                return false;
            }
            covered[eliminated.original_index] = true;
        }
        covered.into_iter().all(|v| v)
    }
    /// Any source/parameter/matrix/scaling/map/threshold change invalidates reuse.
    pub fn is_current_for(&self, current: &Self) -> bool {
        self == current
            && current.valid_for_shape(current.equation_ids.len(), current.retained_variables.len())
    }
    /// Restore an infinitesimal direction to original physical variables.
    /// Affine offsets do not participate in a derivative/null direction.
    /// Orthonormality in scaled coordinates is not claimed after this mapping.
    pub fn original_null_direction(
        &self,
        scaled: &[f64],
    ) -> Result<Vec<f64>, crate::NewellKernelError> {
        if !self.valid_for_shape(self.equation_ids.len(), scaled.len())
            || scaled.iter().any(|x| !x.is_finite())
        {
            return Err(crate::NewellKernelError::InvalidParameter {
                parameter: "rank null provenance",
            });
        }
        let physical: Vec<_> = scaled
            .iter()
            .zip(&self.variable_scaling)
            .map(|(&x, &s)| x * s)
            .collect();
        let mut original = vec![0.; self.original_variable_ids.len()];
        for (j, &i) in self.retained_variables.iter().enumerate() {
            original[i] = physical[j];
        }
        for eliminated in &self.eliminated {
            original[eliminated.original_index] = eliminated
                .reduced_terms
                .iter()
                .map(|&(j, c)| c * physical[j])
                .sum();
        }
        if original.iter().any(|x| !x.is_finite()) {
            return Err(crate::NewellKernelError::InvalidParameter {
                parameter: "rank null overflow",
            });
        }
        Ok(original)
    }
    /// Map a left-null witness through equation scaling to original equations.
    pub fn original_left_direction(
        &self,
        scaled: &[f64],
    ) -> Result<Vec<f64>, crate::NewellKernelError> {
        if !self.valid_for_shape(scaled.len(), self.retained_variables.len())
            || scaled.iter().any(|x| !x.is_finite())
        {
            return Err(crate::NewellKernelError::InvalidParameter {
                parameter: "rank left provenance",
            });
        }
        let original: Vec<_> = scaled
            .iter()
            .zip(&self.equation_scaling)
            .map(|(&x, &s)| x * s)
            .collect();
        if original.iter().any(|x| !x.is_finite()) {
            return Err(crate::NewellKernelError::InvalidParameter {
                parameter: "rank left overflow",
            });
        }
        Ok(original)
    }
    /// Resident owned capacity; does not include temporary validation allocations.
    pub fn owned_bytes(&self) -> usize {
        (self.original_variable_ids.capacity()
            + self.equation_ids.capacity()
            + self.retained_variables.capacity()
            + self.variable_scaling.capacity()
            + self.equation_scaling.capacity())
            * 8
            + self.eliminated.capacity() * std::mem::size_of::<RankElimination>()
            + self
                .eliminated
                .iter()
                .map(|e| e.reduced_terms.capacity() * std::mem::size_of::<(usize, f64)>())
                .sum::<usize>()
    }
}

#[cfg(test)]
mod completion_tests {
    use super::*;
    #[test]
    fn revision6_completion_rank_provenance_invalidates_every_semantic_change() {
        let identity = RankJobIdentity {
            source: 1,
            source_revision: 2,
            parameter_revision: 3,
            matrix_revision: 4,
            scaling_revision: 5,
        };
        let p = RankInputProvenance::uneliminated(identity, 3, 2, 1e-9);
        assert!(p.valid_for_shape(3, 2));
        assert!(p.is_current_for(&p));
        for change in 0..10 {
            let mut current = p.clone();
            match change {
                0 => current.identity.source += 1,
                1 => current.identity.source_revision += 1,
                2 => current.identity.parameter_revision += 1,
                3 => current.identity.matrix_revision += 1,
                4 => current.identity.scaling_revision += 1,
                5 => current.variable_scaling[0] = 2.,
                6 => current.equation_scaling[0] = 2.,
                7 => current.retained_variables.swap(0, 1),
                8 => current.threshold *= 2.,
                _ => current.undamped_equalities = false,
            }
            assert!(!p.is_current_for(&current), "change={change}");
        }
        let mut eliminated = p.clone();
        eliminated.original_variable_ids.push(42);
        eliminated.eliminated.push(RankElimination {
            original_index: 2,
            offset: 1.,
            reduced_terms: vec![(0, 2.), (1, -1.)],
        });
        assert!(eliminated.valid_for_shape(3, 2));
        eliminated.variable_scaling = vec![2., 0.5];
        eliminated.equation_scaling = vec![3., 4., 5.];
        assert_eq!(
            eliminated.original_null_direction(&[1., 2.]).unwrap(),
            vec![2., 1., 3.]
        );
        assert_eq!(
            eliminated.original_left_direction(&[1., 2., 3.]).unwrap(),
            vec![3., 8., 15.]
        );
        assert!(!p.is_current_for(&eliminated));
        eliminated.eliminated[0].reduced_terms[0].0 = 2;
        assert!(!eliminated.valid_for_shape(3, 2));
    }
}
