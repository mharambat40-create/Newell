// SPDX-License-Identifier: MIT
//! Experimental safe-Rust factor bridge for Foster–Davis SPQR_RANK.
//!
//! Implements the column-rejection contract of §2.1, independently of the
//! existing QR path. `P[k]` is the original column at position k; accepted
//! columns precede deferred columns. `A P = Q [R1; 0] + E`, with R11 upper
//! triangular and `||E||_F = hypot(dropped column-tail norms)` in exact
//! arithmetic. Householder vectors are sparse; Q is never materialized.
//!
//! This left-looking bridge is not SuiteSparseQR's multifrontal implementation.
//! A completed bridge alone does NOT establish numerical rank. The separate
//! reliability stage must check R11 and the original undamped, scaled matrix.
//! Source: Foster & Davis, “Reliable Calculation of Numerical Rank ... using
//! SuiteSparseQR”, author manuscript §§2.1–2.4 (M5 plan revision 5).
use crate::sparse::SparseColMatRef;
use alloc::vec::Vec;
use core::cmp::Reverse;
use std::collections::BinaryHeap;

#[path = "rank_front.rs"]
pub mod front;

/// Explicit resource and arithmetic failures. No partial factor is consumable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankQrError {
    /// Invalid dimensions, pattern, permutation, threshold or input value.
    InvalidInput,
    /// Workspace or symbolic/fill admission failed.
    WorkspaceLimit,
    /// Arithmetic-work admission failed.
    WorkLimit,
    /// Cancellation requested; restart the numeric job before reusing it.
    Cancelled,
    /// Floating-point overflow/underflow or invalid numerical operation.
    NumericalFailure,
    /// Factorization has not completed successfully.
    Incomplete,
}
impl core::fmt::Display for RankQrError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl core::error::Error for RankQrError {}

/// Admission limits; all new storage and sparse fill is charged before growth.
#[derive(Debug, Clone, Copy)]
pub struct RankQrLimits {
    /// Bytes of owned vector capacities (including nested-vector metadata).
    pub max_bytes: usize,
    /// Maximum counted elementary sparse work units per numeric job.
    pub max_work: usize,
}

/// Caller-selected pattern and fill-reducing original-column order.
///
/// Construction uses only the pattern. Any bijective order is admissible; faer's
/// existing COLAMD order may be supplied. Values/scaling are numeric-job inputs.
#[derive(Debug)]
pub struct RankQrSymbolic {
    rows: usize,
    cols: usize,
    order: Vec<usize>,
    row_order: Vec<usize>,
    row_inverse: Vec<usize>,
    pattern: Vec<Vec<usize>>,
}
impl RankQrSymbolic {
    /// Capture a pattern, after bounded admission, without reading its values.
    pub fn new(
        a: SparseColMatRef<'_, usize, f64>,
        order: &[usize],
        max_bytes: usize,
    ) -> Result<Self, RankQrError> {
        let mut bytes = 0;
        let mut seen = Vec::new();
        reserve(&mut seen, a.ncols(), &mut bytes, max_bytes)?;
        seen.resize(a.ncols(), false);
        if order.len() != a.ncols() {
            return Err(RankQrError::InvalidInput);
        }
        for &j in order {
            if j >= a.ncols() || seen[j] {
                return Err(RankQrError::InvalidInput);
            }
            seen[j] = true;
        }
        let mut permutation = Vec::new();
        reserve(&mut permutation, order.len(), &mut bytes, max_bytes)?;
        permutation.extend_from_slice(order);
        let mut pattern = Vec::new();
        reserve(&mut pattern, a.ncols(), &mut bytes, max_bytes)?;
        for j in 0..a.ncols() {
            let mut col = Vec::new();
            reserve(&mut col, a.row_idx_of_col(j).count(), &mut bytes, max_bytes)?;
            col.extend(a.row_idx_of_col(j));
            pattern.push(col);
        }
        // A structural row transversal keeps the leading pivot rows local to
        // the column order. It is ordering only, never numerical rank evidence.
        let mut row_inverse = Vec::new();
        reserve(&mut row_inverse, a.nrows(), &mut bytes, max_bytes)?;
        row_inverse.resize(a.nrows(), usize::MAX);
        let mut row_order = Vec::new();
        reserve(&mut row_order, a.nrows(), &mut bytes, max_bytes)?;
        for &j in order {
            if let Some(i) = a.row_idx_of_col(j).find(|&i| row_inverse[i] == usize::MAX) {
                row_inverse[i] = row_order.len();
                row_order.push(i);
            }
        }
        for (i, inverse) in row_inverse.iter_mut().enumerate() {
            if *inverse == usize::MAX {
                *inverse = row_order.len();
                row_order.push(i);
            }
        }
        Ok(Self {
            rows: a.nrows(),
            cols: a.ncols(),
            order: permutation,
            row_order,
            row_inverse,
            pattern,
        })
    }
    /// Resident symbolic capacity, separate from the numeric workspace.
    pub fn bytes(&self) -> usize {
        (self.order.capacity() + self.row_order.capacity() + self.row_inverse.capacity())
            * core::mem::size_of::<usize>()
            + self.pattern.capacity() * core::mem::size_of::<Vec<usize>>()
            + self
                .pattern
                .iter()
                .map(|c| c.capacity() * core::mem::size_of::<usize>())
                .sum::<usize>()
    }
}

#[derive(Debug)]
struct Reflector {
    v: Vec<(usize, f64)>,
    tau: f64,
}

/// Factor-contract progress. `Complete` is not a numerical-rank certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankQrProgress {
    /// More original columns remain.
    Pending,
    /// Factor bridge complete; numerical-rank validation is separate.
    Complete,
}

/// Reusable, caller-owned numeric state. Successful restart invalidates all
/// previous factors. Error/cancellation also invalidates partial factors.
#[derive(Debug)]
pub struct RankQrWorkspace<'a> {
    symbolic: &'a RankQrSymbolic,
    rows: usize,
    cols: usize,
    limits: RankQrLimits,
    bytes: usize,
    work: usize,
    next: usize,
    threshold: f64,
    valid: bool,
    running: bool,
    scratch: Vec<f64>,
    touched: Vec<usize>,
    marked: Vec<bool>,
    queued: Vec<bool>,
    heap: BinaryHeap<Reverse<usize>>,
    incidence: Vec<Vec<usize>>,
    reflectors: Vec<Reflector>,
    columns: Vec<Vec<(usize, f64)>>,
    accepted: Vec<usize>,
    suggested_deferrals: Vec<usize>,
    deferred: Vec<usize>,
    dropped: Vec<f64>,
    imported_r: bool,
    max_svd_ns: u128,
    norm_a: f64,
    perturbation: f64,
}

fn reserve<T>(
    v: &mut Vec<T>,
    len: usize,
    bytes: &mut usize,
    max: usize,
) -> Result<(), RankQrError> {
    if len <= v.capacity() {
        return Ok(());
    }
    let extra = (len - v.capacity())
        .checked_mul(core::mem::size_of::<T>())
        .ok_or(RankQrError::WorkspaceLimit)?;
    let next = bytes
        .checked_add(extra)
        .ok_or(RankQrError::WorkspaceLimit)?;
    if next > max {
        return Err(RankQrError::WorkspaceLimit);
    }
    let old = v.capacity();
    v.try_reserve_exact(len - v.len())
        .map_err(|_| RankQrError::WorkspaceLimit)?;
    *bytes = bytes
        .checked_add((v.capacity() - old) * core::mem::size_of::<T>())
        .ok_or(RankQrError::WorkspaceLimit)?;
    if *bytes > max {
        return Err(RankQrError::WorkspaceLimit);
    }
    Ok(())
}

impl<'a> RankQrWorkspace<'a> {
    /// Allocate O(m+n) scratch, admitting it together with resident symbolic data.
    pub fn new(symbolic: &'a RankQrSymbolic, limits: RankQrLimits) -> Result<Self, RankQrError> {
        let mut w = Self {
            symbolic,
            rows: symbolic.rows,
            cols: symbolic.cols,
            limits,
            bytes: symbolic.bytes(),
            work: 0,
            next: 0,
            threshold: 0.,
            valid: false,
            running: false,
            scratch: Vec::new(),
            touched: Vec::new(),
            marked: Vec::new(),
            queued: Vec::new(),
            heap: BinaryHeap::new(),
            incidence: Vec::new(),
            reflectors: Vec::new(),
            columns: Vec::new(),
            accepted: Vec::new(),
            suggested_deferrals: Vec::new(),
            deferred: Vec::new(),
            dropped: Vec::new(),
            imported_r: false,
            max_svd_ns: 0,
            norm_a: 0.,
            perturbation: 0.,
        };
        let max = limits.max_bytes;
        reserve(&mut w.scratch, w.rows, &mut w.bytes, max)?;
        w.scratch.resize(w.rows, 0.);
        reserve(&mut w.touched, w.rows, &mut w.bytes, max)?;
        reserve(&mut w.marked, w.rows, &mut w.bytes, max)?;
        w.marked.resize(w.rows, false);
        let rank_max = w.rows.min(w.cols);
        reserve(&mut w.queued, rank_max, &mut w.bytes, max)?;
        w.queued.resize(rank_max, false);
        let heap_bytes = rank_max
            .checked_mul(core::mem::size_of::<usize>())
            .ok_or(RankQrError::WorkspaceLimit)?;
        if w.bytes.checked_add(heap_bytes).is_none_or(|b| b > max) {
            return Err(RankQrError::WorkspaceLimit);
        }
        w.heap
            .try_reserve_exact(rank_max)
            .map_err(|_| RankQrError::WorkspaceLimit)?;
        w.bytes += w.heap.capacity() * core::mem::size_of::<usize>();
        reserve(&mut w.incidence, w.rows, &mut w.bytes, max)?;
        w.incidence.resize_with(w.rows, Vec::new);
        reserve(&mut w.reflectors, rank_max, &mut w.bytes, max)?;
        w.reflectors.resize_with(rank_max, || Reflector {
            v: Vec::new(),
            tau: 0.,
        });
        reserve(&mut w.columns, w.cols, &mut w.bytes, max)?;
        w.columns.resize_with(w.cols, Vec::new);
        reserve(&mut w.accepted, w.cols, &mut w.bytes, max)?;
        reserve(&mut w.suggested_deferrals, w.cols, &mut w.bytes, max)?;
        reserve(&mut w.deferred, w.cols, &mut w.bytes, max)?;
        reserve(&mut w.dropped, w.cols, &mut w.bytes, max)?;
        w.dropped.resize(w.cols, 0.);
        if w.bytes > max {
            return Err(RankQrError::WorkspaceLimit);
        }
        Ok(w)
    }
    /// Reset a numeric job. The caller's matrix is already scaled; tau is absolute
    /// in those units and is never silently changed by the numerical backend.
    pub fn start(&mut self, threshold: f64) -> Result<(), RankQrError> {
        self.valid = false;
        self.running = false;
        if !threshold.is_finite() || threshold <= 0. {
            return Err(RankQrError::InvalidInput);
        }
        self.imported_r = false;
        self.max_svd_ns = 0;
        self.threshold = threshold;
        self.work = 0;
        self.next = 0;
        self.norm_a = 0.;
        self.perturbation = 0.;
        self.scratch.fill(0.);
        self.marked.fill(false);
        self.queued.fill(false);
        self.touched.clear();
        self.heap.clear();
        self.accepted.clear();
        self.suggested_deferrals.clear();
        self.deferred.clear();
        self.dropped.fill(0.);
        // Keep inner capacities for subsequent numeric jobs.
        for c in &mut self.columns {
            c.clear();
        }
        for r in &mut self.incidence {
            r.clear();
        }
        // Retain sparse reflector capacities for subsequent numeric jobs.
        for r in &mut self.reflectors {
            r.v.clear();
            r.tau = 0.;
        }
        self.running = true;
        Ok(())
    }
    fn tick(
        &mut self,
        units: usize,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<(), RankQrError> {
        let old = self.work;
        self.work = self.work.checked_add(units).ok_or(RankQrError::WorkLimit)?;
        if self.work > self.limits.max_work {
            return Err(RankQrError::WorkLimit);
        }
        if old / 256 != self.work / 256 && cancelled() {
            return Err(RankQrError::Cancelled);
        }
        Ok(())
    }
    fn touch(&mut self, row: usize) {
        if !self.marked[row] {
            self.marked[row] = true;
            self.touched.push(row);
        }
    }
    fn enqueue_row(&mut self, row: usize, after: Option<usize>) {
        for &h in &self.incidence[row] {
            if after.is_none_or(|k| h > k) && !self.queued[h] {
                self.queued[h] = true;
                self.heap.push(Reverse(h));
            }
        }
    }
    /// Advance at most `columns` original columns, with cancellation polling
    /// during sparse inner loops. The same symbolic pattern/order must be used
    /// throughout a job; values must not change until it completes.
    pub fn advance(
        &mut self,
        a: SparseColMatRef<'_, usize, f64>,
        columns: usize,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<RankQrProgress, RankQrError> {
        let result = self.advance_inner(a, columns, &mut cancelled);
        if result.is_err() {
            self.running = false;
            self.valid = false;
        }
        result
    }
    fn advance_inner(
        &mut self,
        a: SparseColMatRef<'_, usize, f64>,
        columns: usize,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<RankQrProgress, RankQrError> {
        let symbolic = self.symbolic;
        if !self.running {
            return Err(RankQrError::Incomplete);
        }
        if a.nrows() != self.rows
            || a.ncols() != self.cols
            || symbolic.rows != self.rows
            || symbolic.cols != self.cols
        {
            return Err(RankQrError::InvalidInput);
        }
        if cancelled() {
            return Err(RankQrError::Cancelled);
        }
        let end = self.cols.min(self.next.saturating_add(columns));
        while self.next < end {
            let col = symbolic.order[self.next];
            if !a
                .row_idx_of_col(col)
                .eq(symbolic.pattern[col].iter().copied())
            {
                return Err(RankQrError::InvalidInput);
            }
            self.touched.clear();
            self.heap.clear();
            for (original_row, &value) in a.row_idx_of_col(col).zip(a.val_of_col(col)) {
                let i = symbolic.row_inverse[original_row];
                self.tick(1, cancelled)?;
                if !value.is_finite() {
                    return Err(RankQrError::InvalidInput);
                }
                self.norm_a = self.norm_a.hypot(value);
                if !self.norm_a.is_finite() {
                    return Err(RankQrError::NumericalFailure);
                }
                self.touch(i);
                self.scratch[i] = value;
                self.enqueue_row(i, None);
            }
            while let Some(Reverse(h)) = self.heap.pop() {
                self.queued[h] = false;
                let mut dot = 0.;
                for t in 0..self.reflectors[h].v.len() {
                    self.tick(1, cancelled)?;
                    let (i, v) = self.reflectors[h].v[t];
                    dot += v * self.scratch[i];
                }
                dot *= self.reflectors[h].tau;
                if !dot.is_finite() {
                    return Err(RankQrError::NumericalFailure);
                }
                if dot != 0. {
                    for t in 0..self.reflectors[h].v.len() {
                        self.tick(1, cancelled)?;
                        let (i, v) = self.reflectors[h].v[t];
                        self.touch(i);
                        self.scratch[i] -= v * dot;
                        if !self.scratch[i].is_finite() {
                            return Err(RankQrError::NumericalFailure);
                        }
                        self.enqueue_row(i, Some(h));
                    }
                }
                // Exact Householder arithmetic zeros this pivot tail, but do
                // not drop roundoff from other rows or call it certified error.
            }
            let pivot = self.accepted.len();
            let mut norm = 0_f64;
            for t in 0..self.touched.len() {
                self.tick(1, cancelled)?;
                let i = self.touched[t];
                if i >= pivot {
                    norm = norm.hypot(self.scratch[i]);
                }
            }
            let accept = pivot < self.rows && norm > self.threshold;
            let top_count = self
                .touched
                .iter()
                .filter(|&&i| i < pivot && self.scratch[i] != 0.)
                .count();
            reserve(
                &mut self.columns[col],
                top_count + usize::from(accept),
                &mut self.bytes,
                self.limits.max_bytes,
            )?;
            for t in 0..self.touched.len() {
                self.tick(1, cancelled)?;
                let i = self.touched[t];
                if i < pivot && self.scratch[i] != 0. {
                    self.columns[col].push((i, self.scratch[i]));
                }
            }
            if accept {
                let x0 = self.scratch[pivot];
                let beta = -norm.copysign(x0);
                let denominator = x0 - beta;
                let tau = (beta - x0) / beta;
                if !denominator.is_finite() || denominator == 0. || !tau.is_finite() {
                    return Err(RankQrError::NumericalFailure);
                }
                let tail_count = self
                    .touched
                    .iter()
                    .filter(|&&i| i > pivot && self.scratch[i] != 0.)
                    .count();
                reserve(
                    &mut self.reflectors[pivot].v,
                    1 + tail_count,
                    &mut self.bytes,
                    self.limits.max_bytes,
                )?;
                self.reflectors[pivot].v.push((pivot, 1.));
                for t in 0..self.touched.len() {
                    self.tick(1, cancelled)?;
                    let i = self.touched[t];
                    if i > pivot && self.scratch[i] != 0. {
                        self.reflectors[pivot]
                            .v
                            .push((i, self.scratch[i] / denominator));
                    }
                }
                for t in 0..self.reflectors[pivot].v.len() {
                    let i = self.reflectors[pivot].v[t].0;
                    self.tick(1, cancelled)?;
                    let len = self.incidence[i].len();
                    reserve(
                        &mut self.incidence[i],
                        len + 1,
                        &mut self.bytes,
                        self.limits.max_bytes,
                    )?;
                    self.incidence[i].push(pivot);
                }
                self.reflectors[pivot].tau = tau;
                self.columns[col].push((pivot, beta));
                self.accepted.push(col);
            } else {
                self.dropped[col] = norm;
                self.perturbation = self.perturbation.hypot(norm);
                self.deferred.push(col);
            }
            // No sorting is needed: sparse triangular operations use explicit
            // row indices. Rejected columns retain their earlier R entries.
            for &i in &self.touched {
                self.scratch[i] = 0.;
                self.marked[i] = false;
            }
            self.next += 1;
            if cancelled() {
                return Err(RankQrError::Cancelled);
            }
        }
        if self.next == self.cols {
            self.valid = true;
            self.running = false;
            Ok(RankQrProgress::Complete)
        } else {
            Ok(RankQrProgress::Pending)
        }
    }
    fn require_complete(&self) -> Result<(), RankQrError> {
        if self.valid {
            Ok(())
        } else {
            Err(RankQrError::Incomplete)
        }
    }
    /// Original-column permutation, accepted columns first.
    pub fn permutation(&self) -> Result<impl Iterator<Item = usize> + '_, RankQrError> {
        self.require_complete()?;
        Ok(self.accepted.iter().chain(&self.deferred).copied())
    }
    /// Preliminary accepted-column count; not validated numerical rank.
    pub fn preliminary_rank(&self) -> Result<usize, RankQrError> {
        self.require_complete()?;
        Ok(self.accepted.len())
    }
    /// Sparse R1 column in original-column coordinates.
    pub fn r_column(&self, original: usize) -> Result<&[(usize, f64)], RankQrError> {
        self.require_complete()?;
        self.columns
            .get(original)
            .map(Vec::as_slice)
            .ok_or(RankQrError::InvalidInput)
    }
    /// Dropped-tail norm per original column and aggregate Frobenius estimate.
    /// These account for deliberate rejection, not floating-point roundoff.
    pub fn perturbation(&self) -> Result<(&[f64], f64), RankQrError> {
        self.require_complete()?;
        Ok((&self.dropped, self.perturbation))
    }
    /// Resident capacity including the symbolic object, and work used so far.
    pub fn usage(&self) -> (usize, usize) {
        (self.bytes, self.work)
    }
    /// Apply Q or Q^T to one vector without forming Q.
    pub fn apply_q(
        &self,
        x: &mut [f64],
        transpose: bool,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<(), RankQrError> {
        self.require_complete()?;
        if self.imported_r {
            return Err(RankQrError::Incomplete);
        }
        if x.len() != self.rows || x.iter().any(|v| !v.is_finite()) {
            return Err(RankQrError::InvalidInput);
        }
        let mut mapped = Vec::new();
        let mut bytes = self.bytes;
        reserve(&mut mapped, self.rows, &mut bytes, self.limits.max_bytes)?;
        if transpose {
            for &i in &self.symbolic.row_order {
                mapped.push(x[i]);
            }
            x.copy_from_slice(&mapped);
        }
        let mut count = 0;
        for step in 0..self.accepted.len() {
            if cancelled() {
                return Err(RankQrError::Cancelled);
            }
            let k = if transpose {
                step
            } else {
                self.accepted.len() - 1 - step
            };
            let h = &self.reflectors[k];
            let mut dot = 0.;
            for &(i, v) in &h.v {
                dot += x[i] * v;
                count += 1;
                if count % 256 == 0 && cancelled() {
                    return Err(RankQrError::Cancelled);
                }
            }
            dot *= h.tau;
            for &(i, v) in &h.v {
                x[i] -= dot * v;
                count += 1;
                if !x[i].is_finite() {
                    return Err(RankQrError::NumericalFailure);
                }
                if count % 256 == 0 && cancelled() {
                    return Err(RankQrError::Cancelled);
                }
            }
        }
        if !transpose {
            mapped.extend_from_slice(x);
            for (k, &i) in self.symbolic.row_order.iter().enumerate() {
                x[i] = mapped[k];
            }
        }
        Ok(())
    }
}

/// Reliability of an estimate; neither outcome is a rigorous rank certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankQrReliability {
    /// Estimated bounds straddle the caller's threshold with the required gap.
    EstimatedReliable,
    /// No accepted separation, or iteration/block admission was exhausted.
    Uncertain,
}
/// Controls for SPQR_SSI. No threshold override is permitted.
#[derive(Debug, Clone, Copy)]
pub struct RankQrOptions {
    /// Initial subspace width (positive, at most 256).
    pub initial_block: usize,
    /// Width added when the current block contains no above-threshold value.
    pub block_increment: usize,
    /// Maximum width, at most 256; tall-thin work counts against workspace.
    pub max_block: usize,
    /// Maximum inverse iterations, including width expansions.
    pub max_iterations: usize,
    /// Deterministic pseudo-random initialization; record independent restarts.
    pub seed: u64,
}
impl Default for RankQrOptions {
    fn default() -> Self {
        Self {
            initial_block: 3,
            block_increment: 5,
            max_block: 10,
            max_iterations: 100,
            seed: 0x8d8f_2703_541b_2a19,
        }
    }
}
/// Rank estimate for the original, caller-scaled matrix, not an augmented solve.
#[derive(Debug, Clone, Copy)]
pub struct RankQrReport {
    /// Numerical estimate (only publish when reliability permits).
    pub rank: usize,
    /// Accepted-column count before inverse-iteration correction.
    pub preliminary_rank: usize,
    /// Absolute caller threshold, unchanged.
    pub threshold: f64,
    /// Reliability of the numerical estimate.
    pub reliability: RankQrReliability,
    /// Estimated lower bound at the last retained singular value.
    pub lower: f64,
    /// Estimated upper bound at the first discarded singular value.
    pub upper: f64,
    /// Deliberately dropped-tail Frobenius estimate, excluding roundoff.
    pub perturbation: f64,
    /// Boundary singular-triplet residual.
    pub boundary_residual: f64,
    /// Largest checked residual norm of the small candidate null subspaces.
    pub null_residual: f64,
    /// Inverse iterations actually performed.
    pub iterations: usize,
    /// Width actually used.
    pub block: usize,
    /// Initialization provenance.
    pub seed: u64,
    /// Conservative peak storage admission, including faer dense-kernel scratch.
    pub admitted_bytes: usize,
}

impl RankQrWorkspace<'_> {
    /// Import R from an existing faer QR only when every diagonal is above
    /// tau. This is the full-column bridge; rank still requires SSI validation.
    /// A rejected diagonal returns false and leaves no consumable result.
    ///
    /// Q stays owned by the caller's QR; `apply_q` is unavailable for this
    /// R-only import. The caller must validate the original QR reconstruction
    /// and account for its separate storage/scratch in end-to-end measurements.
    /// Import an existing QR with no numerical column rejections. False means
    /// that the caller must use the general column-deferral bridge instead.
    pub fn import_full_column_factor(
        &mut self,
        qr: super::qr::QrRef<'_, usize, f64>,
        threshold: f64,
        cancelled: impl FnMut() -> bool,
    ) -> Result<bool, RankQrError> {
        self.import_factor(qr, threshold, false, cancelled)
    }
    /// Import an existing QR whose deferred columns form a terminal block.
    /// Every transformed tail must have norm <= tau; its discarded norm is
    /// included in the perturbation. Nonterminal dependence returns false.
    /// Q remains caller-owned; `apply_q` is unavailable for imported factors.
    pub fn import_column_factor(
        &mut self,
        qr: super::qr::QrRef<'_, usize, f64>,
        threshold: f64,
        cancelled: impl FnMut() -> bool,
    ) -> Result<bool, RankQrError> {
        self.import_factor(qr, threshold, true, cancelled)
    }

    fn import_factor(
        &mut self,
        qr: super::qr::QrRef<'_, usize, f64>,
        threshold: f64,
        allow_tail: bool,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<bool, RankQrError> {
        self.start(threshold)?;
        self.running = false;
        if qr.symbolic().nrows() != self.rows
            || qr.symbolic().ncols() != self.cols
            || self.rows < self.cols
        {
            return Err(RankQrError::InvalidInput);
        }
        let (perm, _) = qr.symbolic().col_perm().arrays();
        self.accepted.extend_from_slice(perm);
        let imported = qr.visit_r_entries(|i, j, value| {
            self.tick(1, &mut cancelled)?;
            if i > j || !value.is_finite() {
                return Err(RankQrError::NumericalFailure);
            }
            let col = perm[j];
            if value != 0. {
                let len = self.columns[col].len();
                reserve(
                    &mut self.columns[col],
                    len + 1,
                    &mut self.bytes,
                    self.limits.max_bytes,
                )?;
                self.columns[col].push((i, value));
            }
            Ok(())
        });
        imported?;
        let mut accepted = self.cols;
        for (k, &col) in perm.iter().enumerate() {
            if !self.columns[col]
                .iter()
                .any(|&(i, v)| i == k && v.abs() > threshold)
            {
                accepted = k;
                break;
            }
        }
        if accepted < self.cols {
            if !allow_tail {
                return Ok(false);
            }
            // Deferral is allowed only for an entire terminal transformed tail
            // bounded per original column. This is not diagonal counting.
            self.scratch.fill(0.);
            let mut tail_one_norm = 0_f64;
            for &col in &perm[accepted..] {
                let mut norm = 0_f64;
                let mut column_sum = 0_f64;
                for t in 0..self.columns[col].len() {
                    self.tick(1, &mut cancelled)?;
                    let (i, v) = self.columns[col][t];
                    if i >= accepted {
                        norm = norm.hypot(v);
                        column_sum = (column_sum + v.abs()).next_up();
                        self.scratch[i] = (self.scratch[i] + v.abs()).next_up();
                    }
                }
                if norm > threshold {
                    return Ok(false);
                }
                self.dropped[col] = norm;
                self.perturbation = self.perturbation.hypot(norm);
                tail_one_norm = tail_one_norm.max(column_sum);
                self.columns[col].retain(|&(i, _)| i < accepted);
            }
            // ||E||2 <= sqrt(||E||1 ||E||inf). This whole-tail bound can be
            // sharper than Frobenius for orthogonal rejected directions. It
            // still rejects many small columns aligned in the same direction.
            let tail_infinity_norm = self.scratch.iter().copied().fold(0_f64, f64::max);
            if tail_one_norm == 0. || tail_infinity_norm == 0. {
                self.perturbation = 0.;
            } else {
                let bound = (tail_one_norm.sqrt().next_up() * tail_infinity_norm.sqrt().next_up())
                    .next_up();
                self.perturbation = self.perturbation.min(bound);
            }
            self.accepted.truncate(accepted);
            self.deferred.extend_from_slice(&perm[accepted..]);
        }
        if cancelled() {
            return Err(RankQrError::Cancelled);
        }
        self.imported_r = true;
        self.valid = true;
        Ok(true)
    }

    /// Original columns proposed from small right singular-vector supports.
    /// A proposal is never a numerical-rank certificate; a refactorization must
    /// validate the retained and entire rejected blocks on the original matrix.
    pub fn suggested_deferrals(&self) -> &[usize] {
        &self.suggested_deferrals
    }

    fn triangular_solve(
        &mut self,
        rhs: &mut crate::Mat<f64>,
        transpose: bool,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<(), RankQrError> {
        let n = self.accepted.len();
        for j in 0..rhs.ncols() {
            for step in 0..n {
                let k = if transpose { step } else { n - 1 - step };
                let col = self.accepted[k];
                let mut diag = 0.;
                for t in 0..self.columns[col].len() {
                    self.tick(1, cancelled)?;
                    let (i, value) = self.columns[col][t];
                    if i == k {
                        diag = value;
                    } else if transpose {
                        rhs[(k, j)] -= value * rhs[(i, j)];
                    }
                }
                if diag == 0. {
                    return Err(RankQrError::NumericalFailure);
                }
                rhs[(k, j)] /= diag;
                if !rhs[(k, j)].is_finite() {
                    return Err(RankQrError::NumericalFailure);
                }
                if !transpose {
                    for t in 0..self.columns[col].len() {
                        self.tick(1, cancelled)?;
                        let (i, value) = self.columns[col][t];
                        if i < k {
                            rhs[(i, j)] -= value * rhs[(k, j)];
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn triangular_mul(
        &mut self,
        x: &crate::Mat<f64>,
        transpose: bool,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<crate::Mat<f64>, RankQrError> {
        let mut out = crate::Mat::zeros(self.accepted.len(), x.ncols());
        for k in 0..self.accepted.len() {
            let col = self.accepted[k];
            for t in 0..self.columns[col].len() {
                let (i, value) = self.columns[col][t];
                self.tick(x.ncols(), cancelled)?;
                for j in 0..x.ncols() {
                    if transpose {
                        out[(k, j)] += value * x[(i, j)];
                    } else {
                        out[(i, j)] += value * x[(k, j)];
                    }
                }
            }
        }
        Ok(out)
    }
    fn thin_svd(
        &mut self,
        a: &crate::Mat<f64>,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<crate::linalg::solvers::Svd<f64>, RankQrError> {
        if a.ncols() > 256 || a.ncols() > a.nrows() || !a.as_ref().is_all_finite() {
            return Err(RankQrError::NumericalFailure);
        }
        if cancelled() {
            return Err(RankQrError::Cancelled);
        }
        self.tick(
            a.nrows()
                .saturating_mul(a.ncols())
                .saturating_mul(a.ncols()),
            cancelled,
        )?;
        let began = std::time::Instant::now();
        let svd = a.thin_svd().map_err(|_| RankQrError::NumericalFailure)?;
        self.max_svd_ns = self.max_svd_ns.max(began.elapsed().as_nanos());
        if cancelled() {
            return Err(RankQrError::Cancelled);
        }
        Ok(svd)
    }
    /// Validate the bridge with the SPQR_SSI/BASIC rank protocol. Full-rank and
    /// zero-rank endpoints are explicit. Estimated bounds remain estimates.
    ///
    /// Thin SVDs are existing faer kernels, not global dense factorizations.
    /// Their indivisible latency still requires workload-specific qualification.
    /// A cancelled/failed analysis returns no report and preserves the completed
    /// factor bridge; it cannot publish a stale rank report.
    pub fn analyze_rank(
        &mut self,
        options: RankQrOptions,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<RankQrReport, RankQrError> {
        self.require_complete()?;
        if options.initial_block == 0
            || options.block_increment == 0
            || options.max_block == 0
            || options.max_block > 256
            || options.initial_block > options.max_block
        {
            return Err(RankQrError::InvalidInput);
        }
        if cancelled() {
            return Err(RankQrError::Cancelled);
        }
        let n = self.accepted.len();
        let max_b = options.max_block.min(n);
        // All simultaneously live thin matrices, copies and conservative faer
        // kernel scratch. Checked before any dense-kernel allocation. This is
        // an admission envelope, not a measured allocator high-water mark.
        let dense_bytes = self
            .rows
            .max(self.cols)
            .checked_mul(max_b)
            .and_then(|v| v.checked_add(max_b.checked_mul(max_b)?))
            .and_then(|v| v.checked_mul(8 * 40))
            .ok_or(RankQrError::WorkspaceLimit)?;
        let peak = self
            .bytes
            .checked_add(dense_bytes)
            .ok_or(RankQrError::WorkspaceLimit)?;
        if peak > self.limits.max_bytes {
            return Err(RankQrError::WorkspaceLimit);
        }
        let mut report = RankQrReport {
            rank: n,
            preliminary_rank: n,
            threshold: self.threshold,
            reliability: RankQrReliability::Uncertain,
            lower: 0.,
            upper: self.perturbation,
            perturbation: self.perturbation,
            boundary_residual: f64::INFINITY,
            null_residual: f64::INFINITY,
            iterations: 0,
            block: 0,
            seed: options.seed,
            admitted_bytes: peak,
        };
        if n == 0 {
            // Frobenius bounds spectral norm. A large aggregate of individually
            // small dropped columns must not be reported as zero numerical rank.
            if self.perturbation <= self.threshold {
                report.reliability = RankQrReliability::EstimatedReliable;
                report.boundary_residual = 0.;
                report.null_residual = self.perturbation;
            }
            return Ok(report);
        }
        let mut state = options.seed;
        let random = |state: &mut u64| {
            *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut x = *state;
            x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            let x = x ^ (x >> 31);
            ((x >> 11) as f64) / ((1_u64 << 53) as f64) - 0.5
        };
        let mut b = options.initial_block.min(n);
        let mut initial = crate::Mat::from_fn(n, b, |_, _| random(&mut state));
        let mut u = self.thin_svd(&initial, &mut cancelled)?.U().to_owned();
        for iteration in 0..options.max_iterations {
            report.iterations = iteration + 1;
            report.block = b;
            let mut inverse = u.clone();
            self.triangular_solve(&mut inverse, false, &mut cancelled)?;
            let first = self.thin_svd(&inverse, &mut cancelled)?;
            let v = first.U().to_owned();
            let mut inverse_t = v.clone();
            self.triangular_solve(&mut inverse_t, true, &mut cancelled)?;
            let second = self.thin_svd(&inverse_t, &mut cancelled)?;
            u = second.U().to_owned();
            let mut q = 0;
            while q < b && 1. / second.S()[q] <= self.threshold {
                q += 1;
            }
            if q == b {
                if b == max_b {
                    // The bounded SSI subspace is saturated. Keep uncertainty,
                    // but expose right-vector supports as proposals so the
                    // bounded refactoring route can test a new terminal order.
                    for j in 0..q {
                        let mut chosen = None;
                        for k in 0..n {
                            self.tick(b, &mut cancelled)?;
                            let weight = (0..b)
                                .map(|t| v[(k, t)] * second.V()[(t, j)])
                                .sum::<f64>()
                                .abs();
                            let original = self.accepted[k];
                            if !self.suggested_deferrals.contains(&original)
                                && chosen.is_none_or(|(old_weight, old_col)| {
                                    weight > old_weight
                                        || weight == old_weight && original < old_col
                                })
                            {
                                chosen = Some((weight, original));
                            }
                        }
                        if let Some((_, col)) = chosen {
                            self.suggested_deferrals.push(col);
                        }
                    }
                    return Ok(report);
                }
                let next_b = max_b.min(b.saturating_add(options.block_increment));
                // Project new starts twice against U, then orthonormalize the
                // expanded span. Existing converged directions are retained.
                initial = crate::Mat::from_fn(n, next_b, |i, j| {
                    if j < b {
                        u[(i, j)]
                    } else {
                        random(&mut state)
                    }
                });
                for j in b..next_b {
                    for _ in 0..2 {
                        for k in 0..j {
                            let mut dot = 0.;
                            for i in 0..n {
                                self.tick(1, &mut cancelled)?;
                                dot += initial[(i, k)] * initial[(i, j)];
                            }
                            for i in 0..n {
                                initial[(i, j)] -= dot * initial[(i, k)];
                            }
                        }
                    }
                    let norm = (0..n).fold(0_f64, |v, i| v.hypot(initial[(i, j)]));
                    if norm == 0. || !norm.is_finite() {
                        return Err(RankQrError::NumericalFailure);
                    }
                    for i in 0..n {
                        initial[(i, j)] /= norm;
                    }
                }
                b = next_b;
                u = initial.clone();
                continue;
            }
            let sigma = 1. / second.S()[q];
            if !sigma.is_finite() || sigma <= 0. {
                return Err(RankQrError::NumericalFailure);
            }
            let v_rotated = crate::Mat::from_fn(n, q + 1, |i, j| {
                (0..b).map(|k| v[(i, k)] * second.V()[(k, j)]).sum::<f64>()
            });
            let rv = self.triangular_mul(&v_rotated, false, &mut cancelled)?;
            let small_u = crate::Mat::from_fn(n, q + 1, |i, j| u[(i, j)]);
            let rtu = self.triangular_mul(&small_u, true, &mut cancelled)?;
            let mut residual = 0_f64;
            for i in 0..n {
                self.tick(1, &mut cancelled)?;
                residual = residual.hypot(rv[(i, q)] - sigma * u[(i, q)]);
                residual = residual.hypot(rtu[(i, q)] - sigma * v_rotated[(i, q)]);
            }
            residual /= core::f64::consts::SQRT_2;
            report.boundary_residual = residual;
            report.rank = n - q;
            report.lower = sigma - residual;
            let mut null_norm = 0_f64;
            // Frobenius norms are conservative upper bounds on each spectral
            // norm in SSI's small-null-subspace stopping condition.
            for mat in [&rv, &rtu] {
                let mut value = 0_f64;
                for j in 0..q {
                    for i in 0..n {
                        self.tick(1, &mut cancelled)?;
                        value = value.hypot(mat[(i, j)]);
                    }
                }
                null_norm = null_norm.max(value);
            }
            report.null_residual = null_norm;
            if residual <= 0.1 * (sigma - self.threshold)
                && residual <= 0.1 * sigma
                && null_norm <= self.threshold
            {
                // BASIC step 5: U2^T R1, including all deferred columns, not R11
                // alone. Its spectral norm bounds the first discarded value.
                let mut upper = self.perturbation;
                if q > 0 {
                    let mut projected = crate::Mat::zeros(self.cols, q);
                    for col in 0..self.cols {
                        for t in 0..self.columns[col].len() {
                            let (i, value) = self.columns[col][t];
                            self.tick(q, &mut cancelled)?;
                            for j in 0..q {
                                projected[(col, j)] += value * u[(i, j)];
                            }
                        }
                    }
                    upper += self.thin_svd(&projected, &mut cancelled)?.S()[0];
                }
                report.upper = upper;
                if report.lower > self.threshold && upper <= self.threshold {
                    report.reliability = RankQrReliability::EstimatedReliable;
                    for j in 0..q {
                        let mut chosen = None;
                        for k in 0..n {
                            self.tick(1, &mut cancelled)?;
                            let original = self.accepted[k];
                            let weight = v_rotated[(k, j)].abs();
                            if !self.suggested_deferrals.contains(&original)
                                && chosen.is_none_or(|(old_weight, old_col)| {
                                    weight > old_weight
                                        || weight == old_weight && original < old_col
                                })
                            {
                                chosen = Some((weight, original));
                            }
                        }
                        if let Some((_, col)) = chosen {
                            self.suggested_deferrals.push(col);
                        }
                    }
                }
                return Ok(report);
            }
        }
        Ok(report)
    }
}

/// Coordinate domain of a recorded permutation, including transpose jobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankPermutationAxis {
    /// Original Jacobian variable columns.
    OriginalColumns,
    /// Original Jacobian equation rows, when its transpose was factored.
    OriginalRows,
}
/// Result of the bounded initial-plus-two-corrective-pass route.
#[derive(Debug)]
pub struct RankRefactorReport {
    /// Numerical report, absent when no admissible bridge was found.
    pub rank: Option<RankQrReport>,
    /// Original-column order used by the final attempted factorization.
    pub permutation: Vec<usize>,
    /// Whether this permutation addresses original equations or variables.
    pub permutation_axis: RankPermutationAxis,
    /// All numeric factorizations, never more than three.
    pub factorizations: usize,
    /// Largest admitted numeric arrays/scratch plus rank-workspace envelope.
    /// This excludes caller input and is not a measured process peak.
    pub numeric_admitted_bytes: usize,
    /// Sparse bridge/SSI work counted across passes (opaque QR work is timed
    /// separately by the caller, not misrepresented as counted arithmetic).
    pub counted_work: usize,
    /// Worst observed indivisible native numeric QR call, nanoseconds.
    pub native_qr_max_ns: u128,
    /// Worst observed indivisible faer thin-SVD call, nanoseconds.
    pub dense_svd_max_ns: u128,
    /// Conservative upper bound across each whole symbolic call (which polls
    /// between its internal phases), not an exact indivisible duration.
    pub symbolic_call_upper_ns: u128,
}
/// Bounded refactorization of the ORIGINAL scaled, undamped matrix. No columns
/// or equations are removed. Proposed dependent columns are moved terminally;
/// explicit symbolic ordering prevents COLAMD from undoing that partition.
pub fn refactorize_rank(
    a: SparseColMatRef<'_, usize, f64>,
    initial_order: &[usize],
    threshold: f64,
    limits: RankQrLimits,
    options: RankQrOptions,
    mut cancelled: impl FnMut() -> bool,
) -> Result<RankRefactorReport, RankQrError> {
    if !threshold.is_finite() || threshold <= 0. {
        return Err(RankQrError::InvalidInput);
    }
    let mut inspected = 0usize;
    for j in 0..a.ncols() {
        for &value in a.val_of_col(j) {
            inspected += 1;
            if !value.is_finite() {
                return Err(RankQrError::InvalidInput);
            }
            if inspected % 256 == 0 && cancelled() {
                return Err(RankQrError::Cancelled);
            }
        }
    }
    if a.nrows() < a.ncols() {
        // Validate the caller's original-variable order before changing axes.
        let validated = RankQrSymbolic::new(a, initial_order, limits.max_bytes)?;
        drop(validated);
        let transpose_bytes = a
            .compute_nnz()
            .checked_mul(16)
            .and_then(|b| b.checked_add(a.nrows().checked_add(1)?.checked_mul(8)?))
            .ok_or(RankQrError::WorkspaceLimit)?;
        let child_limits = RankQrLimits {
            max_bytes: limits
                .max_bytes
                .checked_sub(transpose_bytes)
                .ok_or(RankQrError::WorkspaceLimit)?,
            ..limits
        };
        if cancelled() {
            return Err(RankQrError::Cancelled);
        }
        let at = a
            .transpose()
            .to_col_major()
            .map_err(|_| RankQrError::WorkspaceLimit)?;
        let mut order = Vec::new();
        order
            .try_reserve_exact(at.ncols())
            .map_err(|_| RankQrError::WorkspaceLimit)?;
        order.extend(0..at.ncols());
        let callback: &mut dyn FnMut() -> bool = &mut cancelled;
        let mut result = refactorize_rank(
            at.as_ref(),
            &order,
            threshold,
            child_limits,
            options,
            callback,
        )?;
        result.permutation_axis = RankPermutationAxis::OriginalRows;
        result.numeric_admitted_bytes = result
            .numeric_admitted_bytes
            .checked_add(transpose_bytes)
            .ok_or(RankQrError::WorkspaceLimit)?;
        return Ok(result);
    }
    let pattern = RankQrSymbolic::new(a, initial_order, limits.max_bytes)?;
    let mut workspace = RankQrWorkspace::new(&pattern, limits)?;
    let mut order = Vec::new();
    order
        .try_reserve_exact(initial_order.len())
        .map_err(|_| RankQrError::WorkspaceLimit)?;
    order.extend_from_slice(initial_order);
    let mut report = RankRefactorReport {
        rank: None,
        permutation: Vec::new(),
        permutation_axis: RankPermutationAxis::OriginalColumns,
        factorizations: 0,
        numeric_admitted_bytes: 0,
        counted_work: 0,
        native_qr_max_ns: 0,
        dense_svd_max_ns: 0,
        symbolic_call_upper_ns: 0,
    };
    let mut defer = Vec::new();
    defer
        .try_reserve_exact(a.ncols())
        .map_err(|_| RankQrError::WorkspaceLimit)?;
    let mut diagonal = Vec::new();
    diagonal
        .try_reserve_exact(a.ncols())
        .map_err(|_| RankQrError::WorkspaceLimit)?;
    diagonal.resize(a.ncols(), 0.);
    let mut flags = Vec::new();
    flags
        .try_reserve_exact(a.ncols())
        .map_err(|_| RankQrError::WorkspaceLimit)?;
    flags.resize(a.ncols(), false);
    for pass in 0..3 {
        if cancelled() {
            return Err(RankQrError::Cancelled);
        }
        let symbolic_began = std::time::Instant::now();
        let symbolic = super::qr::factorize_symbolic_qr_ordered(
            a.symbolic(),
            &order,
            super::qr::QrSymbolicParams {
                supernodal_flop_ratio_threshold: super::SupernodalThreshold::FORCE_SUPERNODAL,
                ..Default::default()
            },
            limits.max_bytes,
            &mut cancelled,
        )?;
        report.symbolic_call_upper_ns = report
            .symbolic_call_upper_ns
            .max(symbolic_began.elapsed().as_nanos());
        let req = symbolic.factorize_numeric_qr_scratch::<f64>(crate::Par::Seq, Default::default());
        let numeric = (symbolic
            .len_idx()
            .checked_add(symbolic.len_val())
            .and_then(|n| n.checked_mul(8)))
        .and_then(|b| b.checked_add(req.size_bytes()))
        .ok_or(RankQrError::WorkspaceLimit)?;
        // Caller-owned original matrix stays borrowed; account helper vectors and
        // a conservative symbolic envelope separately from numeric buffers.
        let overhead = a
            .ncols()
            .checked_mul(8 * 12)
            .and_then(|b| b.checked_add(a.nrows().checked_mul(8 * 4)?))
            .ok_or(RankQrError::WorkspaceLimit)?;
        let base = numeric
            .checked_add(overhead)
            .and_then(|b| b.checked_add(workspace.usage().0))
            .ok_or(RankQrError::WorkspaceLimit)?;
        if base > limits.max_bytes {
            return Err(RankQrError::WorkspaceLimit);
        }
        let mut indices = Vec::new();
        indices
            .try_reserve_exact(symbolic.len_idx())
            .map_err(|_| RankQrError::WorkspaceLimit)?;
        indices.resize(symbolic.len_idx(), 0);
        let mut values = Vec::new();
        values
            .try_reserve_exact(symbolic.len_val())
            .map_err(|_| RankQrError::WorkspaceLimit)?;
        values.resize(symbolic.len_val(), 0.);
        let mut memory =
            crate::dyn_stack::MemBuffer::try_new(req).map_err(|_| RankQrError::WorkspaceLimit)?;
        let native_began = std::time::Instant::now();
        let qr = symbolic.factorize_numeric_qr(
            &mut indices,
            &mut values,
            a,
            crate::Par::Seq,
            crate::dyn_stack::MemStack::new(&mut memory),
            Default::default(),
        );
        report.native_qr_max_ns = report
            .native_qr_max_ns
            .max(native_began.elapsed().as_nanos());
        report.factorizations = pass + 1;
        if cancelled() {
            return Err(RankQrError::Cancelled);
        }
        diagonal.fill(0.);
        qr.visit_r_entries(|i, j, v| {
            if !v.is_finite() {
                return Err(RankQrError::NumericalFailure);
            }
            if i == j {
                diagonal[j] = v.abs();
            }
            Ok(())
        })?;
        workspace.limits.max_bytes = limits
            .max_bytes
            .checked_sub(numeric + overhead)
            .ok_or(RankQrError::WorkspaceLimit)?;
        workspace.limits.max_work = limits
            .max_work
            .checked_sub(report.counted_work)
            .ok_or(RankQrError::WorkLimit)?;
        let admitted = workspace.import_column_factor(qr, threshold, &mut cancelled)?;
        defer.clear();
        if admitted {
            let rank = workspace.analyze_rank(options, &mut cancelled)?;
            report.dense_svd_max_ns = report.dense_svd_max_ns.max(workspace.max_svd_ns);
            report.numeric_admitted_bytes = report
                .numeric_admitted_bytes
                .max(numeric + overhead + rank.admitted_bytes);
            if report.numeric_admitted_bytes > limits.max_bytes {
                return Err(RankQrError::WorkspaceLimit);
            }
            let corrected = rank.rank < rank.preliminary_rank;
            report.rank = Some(rank);
            if rank.reliability == RankQrReliability::EstimatedReliable && !corrected {
                report.counted_work = report
                    .counted_work
                    .checked_add(workspace.usage().1)
                    .ok_or(RankQrError::WorkLimit)?;
                report.permutation = order;
                return Ok(report);
            }
            defer.extend_from_slice(workspace.suggested_deferrals());
        } else {
            report.rank = None;
        }
        report.counted_work = report
            .counted_work
            .checked_add(workspace.usage().1)
            .ok_or(RankQrError::WorkLimit)?;
        if report.counted_work > limits.max_work {
            return Err(RankQrError::WorkLimit);
        }
        if defer.is_empty() {
            for (k, &d) in diagonal.iter().enumerate() {
                if d <= threshold {
                    defer.push(order[k]);
                }
            }
        }
        if pass == 2 || defer.is_empty() {
            break;
        }
        flags.fill(false);
        for &j in &defer {
            flags[j] = true;
        }
        // Stable in-partition ordering; original fill order is retained.
        let mut next = Vec::new();
        next.try_reserve_exact(order.len())
            .map_err(|_| RankQrError::WorkspaceLimit)?;
        next.extend(order.iter().copied().filter(|&j| !flags[j]));
        next.extend(order.iter().copied().filter(|&j| flags[j]));
        if next == order {
            break;
        }
        order = next;
        if cancelled() {
            return Err(RankQrError::Cancelled);
        }
    }
    report.permutation = order;
    Ok(report)
}

/// Owned compact native orthogonal operator in original row coordinates. The
/// row order aligns the first n output coordinates with R, retaining complement.
#[derive(Debug)]
pub struct RankQrOrthogonal {
    pub(crate) rows: usize,
    pub(crate) order: Vec<usize>,
    pub(crate) reflectors: Vec<Vec<(usize, f64)>>,
    pub(crate) beta: Vec<f64>,
    pub(crate) bytes: usize,
    pub(crate) max_bytes: usize,
    pub(crate) column_shift: usize,
}
impl RankQrOrthogonal {
    /// Actual resident vector capacities, including nested sparse reflectors.
    /// Does not include caller input/output or any other factor owners.
    pub fn owned_bytes(&self) -> usize {
        self.order.capacity() * core::mem::size_of::<usize>()
            + self.beta.capacity() * core::mem::size_of::<f64>()
            + self.reflectors.capacity() * core::mem::size_of::<Vec<(usize, f64)>>()
            + self
                .reflectors
                .iter()
                .map(|v| v.capacity() * core::mem::size_of::<(usize, f64)>())
                .sum::<usize>()
    }
    pub(crate) fn new(rows: usize, max_bytes: usize) -> Result<Self, RankQrError> {
        let mut result = Self {
            rows,
            order: Vec::new(),
            reflectors: Vec::new(),
            beta: Vec::new(),
            bytes: 0,
            max_bytes,
            column_shift: 0,
        };
        reserve(&mut result.order, rows, &mut result.bytes, max_bytes)?;
        result.order.resize(rows, usize::MAX);
        Ok(result)
    }
    pub(crate) fn push(&mut self, values: Vec<(usize, f64)>, beta: f64) -> Result<(), RankQrError> {
        if !beta.is_finite()
            || values
                .iter()
                .any(|&(i, v)| i >= self.rows || !v.is_finite())
        {
            return Err(RankQrError::NumericalFailure);
        }
        let n = self.reflectors.len() + 1;
        reserve(&mut self.reflectors, n, &mut self.bytes, self.max_bytes)?;
        reserve(&mut self.beta, n, &mut self.bytes, self.max_bytes)?;
        self.reflectors.push(values);
        self.beta.push(beta);
        Ok(())
    }
    pub(crate) fn vector(&mut self, len: usize) -> Result<Vec<(usize, f64)>, RankQrError> {
        let mut v = Vec::new();
        reserve(&mut v, len, &mut self.bytes, self.max_bytes)?;
        Ok(v)
    }
    pub(crate) fn finish(&mut self) -> Result<(), RankQrError> {
        let mut seen = Vec::new();
        reserve(&mut seen, self.rows, &mut self.bytes, self.max_bytes)?;
        seen.resize(self.rows, false);
        for &i in &self.order {
            if i != usize::MAX {
                if i >= self.rows || seen[i] {
                    return Err(RankQrError::NumericalFailure);
                }
                seen[i] = true;
            }
        }
        let mut next = 0;
        for i in &mut self.order {
            if *i == usize::MAX {
                while seen[next] {
                    next += 1;
                }
                *i = next;
                seen[next] = true;
            }
        }
        Ok(())
    }
    /// Resident vector capacity admission; caller-owned source factor is separate.
    pub fn admitted_bytes(&self) -> usize {
        self.bytes
    }
    /// Apply Q or Q^T, preserving the full orthogonal complement. Cancellation
    /// invalidates this output vector; the caller must not publish it.
    pub fn apply(
        &self,
        x: &mut [f64],
        transpose: bool,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<(), RankQrError> {
        if x.len() != self.rows || x.iter().any(|v| !v.is_finite()) {
            return Err(RankQrError::InvalidInput);
        }
        let mut temp = Vec::new();
        let mut bytes = self.bytes;
        reserve(&mut temp, self.rows, &mut bytes, self.max_bytes)?;
        temp.resize(self.rows, 0.);
        if !transpose {
            x.rotate_right(self.column_shift);
            for (k, &row) in self.order.iter().enumerate() {
                temp[row] = x[k];
            }
            x.copy_from_slice(&temp);
        }
        let mut count = 0;
        for step in 0..self.reflectors.len() {
            if cancelled() {
                return Err(RankQrError::Cancelled);
            }
            let k = if transpose {
                step
            } else {
                self.reflectors.len() - 1 - step
            };
            let mut dot = 0.;
            for &(i, v) in &self.reflectors[k] {
                dot += v * x[i];
                count += 1;
                if count % 256 == 0 && cancelled() {
                    return Err(RankQrError::Cancelled);
                }
            }
            dot *= self.beta[k];
            for &(i, v) in &self.reflectors[k] {
                x[i] -= v * dot;
                count += 1;
                if !x[i].is_finite() {
                    return Err(RankQrError::NumericalFailure);
                }
                if count % 256 == 0 && cancelled() {
                    return Err(RankQrError::Cancelled);
                }
            }
        }
        if transpose {
            for (k, &row) in self.order.iter().enumerate() {
                temp[k] = x[row];
            }
            x.copy_from_slice(&temp);
            x.rotate_left(self.column_shift);
        }
        Ok(())
    }
}

impl RankQrOrthogonal {
    /// Apply the trailing-column operator Q[:, rank..] or its adjoint without
    /// materializing a basis. The caller must link the supplied rank and factor
    /// to the original matrix and validate numerical-null-space residuals.
    pub fn apply_null(
        &self,
        rank: usize,
        input: &[f64],
        output: &mut [f64],
        transpose: bool,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<(), RankQrError> {
        if rank > self.rows {
            return Err(RankQrError::InvalidInput);
        }
        let nullity = self.rows - rank;
        if (!transpose && (input.len() != nullity || output.len() != self.rows))
            || (transpose && (input.len() != self.rows || output.len() != nullity))
        {
            return Err(RankQrError::InvalidInput);
        }
        let mut full = Vec::new();
        let mut bytes = self.bytes;
        reserve(
            &mut full,
            self.rows
                .checked_mul(2)
                .ok_or(RankQrError::WorkspaceLimit)?,
            &mut bytes,
            self.max_bytes,
        )?;
        full.resize(self.rows, 0.);
        if transpose {
            full.copy_from_slice(input);
            self.apply(&mut full, true, &mut cancelled)?;
            output.copy_from_slice(&full[rank..]);
        } else {
            full[rank..].copy_from_slice(input);
            self.apply(&mut full, false, &mut cancelled)?;
            output.copy_from_slice(&full);
        }
        Ok(())
    }
}
