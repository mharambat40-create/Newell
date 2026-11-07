// SPDX-License-Identifier: MIT
//! Bounded safe frontal experiment for the revision-6 V2.2 escalation.
//! Rows are assembled at their first active column. Fully assembled columns
//! are accepted or carried as deferred columns; no structural matching is rank.
//! Native dense QR uses <=128-row pivot columns or <=256x128 local compression. Each
//! batch is one continuation slice; completed factors still require SSI and
//! original-matrix residual validation. This prototype has no supernode update
//! supernode amalgamation beyond bounded local QR; fill/work exhaustion remains explicit.
use super::*;
use std::collections::BTreeMap;
#[derive(Debug)]
struct Row {
    id: usize,
    values: BTreeMap<usize, f64>,
}
#[derive(Debug)]
enum FrontAction {
    Reflector(Vec<(usize, f64)>, f64),
    LocalOrthogonal(Vec<usize>, crate::Mat<f64>),
}
/// Progress of one bounded batch; no partial factor is consumable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontProgress {
    /// More front batches remain; no partial factor is consumable.
    Pending,
    /// Front batches completed; factor assessment is still required.
    Complete,
}
/// Owned safe frontal continuation. No self-referential/native factor borrow.
#[derive(Debug)]
pub struct RankQrContinuation {
    symbolic: RankQrSymbolic,
    limits: RankQrLimits,
    threshold: f64,
    buckets: Vec<Vec<Row>>,
    terminal: Vec<Row>,
    current: Vec<Row>,
    pivot: Option<Row>,
    column: usize,
    rejected: Vec<bool>,
    accepted: Vec<usize>,
    r: Vec<Row>,
    actions: Vec<FrontAction>,
    work: usize,
    peak_bytes: usize,
    qr_max_ns: u128,
    complete: bool,
    failed: bool,
}
impl RankQrContinuation {
    /// Assemble original rows into ordered fronts after conservative admission.
    pub fn new(
        a: SparseColMatRef<'_, usize, f64>,
        order: &[usize],
        threshold: f64,
        limits: RankQrLimits,
    ) -> Result<Self, RankQrError> {
        if !threshold.is_finite() || threshold <= 0. {
            return Err(RankQrError::InvalidInput);
        }
        let symbolic = RankQrSymbolic::new(a, order, limits.max_bytes)?;
        let initial = a
            .compute_nnz()
            .checked_mul(96)
            .and_then(|b| b.checked_add(a.nrows().checked_mul(128)?))
            .and_then(|b| b.checked_add(a.ncols().checked_mul(64)?))
            .ok_or(RankQrError::WorkspaceLimit)?;
        if initial > limits.max_bytes {
            return Err(RankQrError::WorkspaceLimit);
        }
        let mut inverse = vec![0; order.len()];
        for (k, &j) in order.iter().enumerate() {
            inverse[j] = k;
        }
        let mut rows: Vec<_> = (0..a.nrows())
            .map(|id| Row {
                id,
                values: BTreeMap::new(),
            })
            .collect();
        for (j, &ordered) in inverse.iter().enumerate() {
            for (i, &v) in a.row_idx_of_col(j).zip(a.val_of_col(j)) {
                if !v.is_finite() {
                    return Err(RankQrError::InvalidInput);
                }
                if v != 0. {
                    rows[i].values.insert(ordered, v);
                }
            }
        }
        let mut buckets: Vec<Vec<Row>> = (0..a.ncols()).map(|_| Vec::new()).collect();
        let mut terminal = Vec::new();
        for row in rows {
            if let Some((&k, _)) = row.values.first_key_value() {
                buckets[k].push(row);
            } else {
                terminal.push(row);
            }
        }
        Ok(Self {
            symbolic,
            limits,
            threshold,
            buckets,
            terminal,
            current: Vec::new(),
            pivot: None,
            column: 0,
            rejected: vec![false; a.ncols()],
            accepted: Vec::new(),
            r: Vec::new(),
            actions: Vec::new(),
            work: 0,
            peak_bytes: initial,
            qr_max_ns: 0,
            complete: false,
            failed: false,
        })
    }
    fn tick(&mut self, units: usize, cancel: &mut impl FnMut() -> bool) -> Result<(), RankQrError> {
        self.work = self.work.checked_add(units).ok_or(RankQrError::WorkLimit)?;
        if self.work > self.limits.max_work {
            return Err(RankQrError::WorkLimit);
        }
        if cancel() {
            return Err(RankQrError::Cancelled);
        }
        Ok(())
    }
    fn live_envelope(&self) -> usize {
        let row_bytes = |row: &Row| 128 + row.values.len() * 96;
        self.buckets
            .iter()
            .flatten()
            .chain(&self.current)
            .chain(&self.terminal)
            .chain(&self.r)
            .chain(self.pivot.iter())
            .map(row_bytes)
            .sum::<usize>()
            + self
                .actions
                .iter()
                .map(|action| match action {
                    FrontAction::Reflector(v, _) => v.capacity() * 16 + 48,
                    FrontAction::LocalOrthogonal(rows, q) => {
                        rows.capacity() * 8 + q.nrows() * q.ncols() * 16 + 64
                    }
                })
                .sum::<usize>()
            + self.symbolic.bytes()
            + self.buckets.capacity() * 24
            + self.rejected.capacity()
    }
    // Compress only a bounded, fully owned contribution block. The existing
    // safe dense QR supplies its exact numerical Q/R convention; local Q is
    // never a global dense basis. Zero contribution rows retain their IDs.
    fn compress_current(&mut self, cancel: &mut impl FnMut() -> bool) -> Result<bool, RankQrError> {
        let rows = self.current.len().min(256);
        if rows <= 64 {
            return Ok(false);
        }
        let mut columns = std::collections::BTreeSet::new();
        for i in self.current.len() - rows..self.current.len() {
            self.tick(1, cancel)?;
            columns.extend(self.current[i].values.keys().copied());
        }
        let width = columns.len();
        if width == 0 || width >= rows || width > 128 {
            return Ok(false);
        }
        let columns: Vec<_> = columns.into_iter().collect();
        let extra = rows
            .checked_mul(rows + width * 4)
            .and_then(|n| n.checked_mul(32))
            .ok_or(RankQrError::WorkspaceLimit)?;
        let peak = self
            .live_envelope()
            .checked_add(extra)
            .ok_or(RankQrError::WorkspaceLimit)?;
        if peak > self.limits.max_bytes {
            return Err(RankQrError::WorkspaceLimit);
        }
        self.peak_bytes = self.peak_bytes.max(peak);
        self.tick(2 * rows * width * width + 2 * rows * rows * width, cancel)?;
        let mut batch = self.current.split_off(self.current.len() - rows);
        let matrix = crate::Mat::from_fn(rows, width, |i, j| {
            *batch[i].values.get(&columns[j]).unwrap_or(&0.)
        });
        let began = std::time::Instant::now();
        let qr = matrix.qr();
        self.qr_max_ns = self.qr_max_ns.max(began.elapsed().as_nanos());
        if cancel() {
            return Err(RankQrError::Cancelled);
        }
        let began = std::time::Instant::now();
        let q = qr.compute_Q();
        self.qr_max_ns = self.qr_max_ns.max(began.elapsed().as_nanos());
        if !q.as_ref().is_all_finite() {
            return Err(RankQrError::NumericalFailure);
        }
        let identities = batch.iter().map(|r| r.id).collect();
        self.actions
            .push(FrontAction::LocalOrthogonal(identities, q));
        for (i, row) in batch.iter_mut().enumerate() {
            self.tick(width, cancel)?;
            row.values.clear();
            if i < width {
                for (j, &column) in columns.iter().enumerate() {
                    let value = qr.R()[(i, j)];
                    if !value.is_finite() {
                        return Err(RankQrError::NumericalFailure);
                    }
                    if value != 0. {
                        row.values.insert(column, value);
                    }
                }
            }
        }
        let zero = batch.split_off(width);
        self.terminal.extend(zero);
        self.current.extend(batch);
        Ok(true)
    }
    fn scatter(&mut self, row: Row) {
        if let Some((&k, _)) = row
            .values
            .iter()
            .find(|(k, _)| !self.rejected[**k] && **k > self.column)
        {
            self.buckets[k].push(row);
        } else {
            self.terminal.push(row);
        }
    }
    /// At most allowance QR batches/front transitions. All safe inner loops
    /// poll cancellation; native QR/orthogonal compression calls are bounded locally.
    pub fn advance(
        &mut self,
        allowance: usize,
        mut cancel: impl FnMut() -> bool,
    ) -> Result<FrontProgress, RankQrError> {
        if self.failed {
            return Err(RankQrError::Incomplete);
        }
        if allowance == 0 {
            return Err(RankQrError::InvalidInput);
        }
        let result = self.advance_inner(allowance, &mut cancel);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn advance_inner(
        &mut self,
        allowance: usize,
        cancel: &mut impl FnMut() -> bool,
    ) -> Result<FrontProgress, RankQrError> {
        for _ in 0..allowance {
            self.tick(1, cancel)?;
            if self.complete {
                return Ok(FrontProgress::Complete);
            }
            if self.column == self.symbolic.cols {
                self.complete = true;
                return Ok(FrontProgress::Complete);
            }
            if self.current.is_empty() && self.pivot.is_none() {
                self.current = core::mem::take(&mut self.buckets[self.column]);
                if self.compress_current(cancel)? {
                    self.buckets[self.column] = core::mem::take(&mut self.current);
                    continue;
                }

                let mut norm = 0_f64;
                for i in 0..self.current.len() {
                    self.tick(1, cancel)?;
                    norm = norm.hypot(*self.current[i].values.get(&self.column).unwrap_or(&0.));
                }
                if norm <= self.threshold {
                    self.rejected[self.column] = true;
                    for row in core::mem::take(&mut self.current) {
                        self.scatter(row);
                    }
                    self.column += 1;
                    continue;
                }
            }
            let mut batch = Vec::new();
            if let Some(pivot) = self.pivot.take() {
                batch.push(pivot);
            }
            while batch.len() < 128 {
                if let Some(row) = self.current.pop() {
                    batch.push(row);
                } else {
                    break;
                }
            }
            let mut columns = std::collections::BTreeSet::new();
            for row in &batch {
                columns.extend(row.values.keys().copied());
            }
            let bytes = self
                .live_envelope()
                .checked_add(
                    batch
                        .iter()
                        .map(|r| 128 + r.values.len() * 96)
                        .sum::<usize>(),
                )
                .and_then(|b| {
                    b.checked_add(batch.len().checked_mul(columns.len())?.checked_mul(128)?)
                })
                .ok_or(RankQrError::WorkspaceLimit)?;
            if bytes > self.limits.max_bytes {
                return Err(RankQrError::WorkspaceLimit);
            }
            self.peak_bytes = self.peak_bytes.max(bytes);
            self.tick(
                batch.len().saturating_mul(columns.len()).saturating_mul(6),
                cancel,
            )?;
            let vector = crate::Mat::from_fn(batch.len(), 1, |i, _| {
                *batch[i].values.get(&self.column).unwrap_or(&0.)
            });
            let began = std::time::Instant::now();
            let qr = vector.qr();
            self.qr_max_ns = self.qr_max_ns.max(began.elapsed().as_nanos());
            let beta = 1. / qr.Q_coeff()[(0, 0)];
            if !beta.is_finite() {
                return Err(RankQrError::NumericalFailure);
            }
            let values: Vec<_> = (0..batch.len())
                .map(|i| if i == 0 { 1. } else { qr.Q_basis()[(i, 0)] })
                .collect();
            for column in columns {
                self.tick(batch.len() * 2, cancel)?;
                let dot = beta
                    * batch
                        .iter()
                        .zip(&values)
                        .map(|(r, &v)| v * r.values.get(&column).copied().unwrap_or(0.))
                        .sum::<f64>();
                for (i, row) in batch.iter_mut().enumerate() {
                    let value = if column == self.column {
                        if i == 0 {
                            qr.R()[(0, 0)]
                        } else {
                            0.
                        }
                    } else {
                        row.values.get(&column).copied().unwrap_or(0.) - values[i] * dot
                    };
                    if !value.is_finite() {
                        return Err(RankQrError::NumericalFailure);
                    }
                    if value == 0. {
                        row.values.remove(&column);
                    } else {
                        row.values.insert(column, value);
                    }
                }
            }
            self.actions.push(FrontAction::Reflector(
                batch.iter().zip(values).map(|(r, v)| (r.id, v)).collect(),
                beta,
            ));
            let pivot = batch.remove(0);
            for row in batch {
                self.scatter(row);
            }
            if self.current.is_empty() {
                self.accepted.push(self.symbolic.order[self.column]);
                self.r.push(pivot);
                self.column += 1;
            } else {
                self.pivot = Some(pivot);
            }
        }
        Ok(FrontProgress::Pending)
    }
    /// Counted work, conservative live admission peak and actual native max.
    pub fn usage(&self) -> (usize, usize, u128) {
        (self.peak_bytes, self.work, self.qr_max_ns)
    }
    fn require_complete(&self) -> Result<(), RankQrError> {
        if !self.complete || self.failed {
            Err(RankQrError::Incomplete)
        } else {
            Ok(())
        }
    }
    /// Apply full Q/Qt with original row identities; no dense global basis.
    pub fn apply_q(
        &self,
        x: &mut [f64],
        transpose: bool,
        mut cancel: impl FnMut() -> bool,
    ) -> Result<(), RankQrError> {
        self.require_complete()?;
        if x.len() != self.symbolic.rows {
            return Err(RankQrError::InvalidInput);
        }
        let mut row_order: Vec<_> = self.r.iter().map(|r| r.id).collect();
        let mut seen = vec![false; x.len()];
        for &i in &row_order {
            seen[i] = true;
        }
        row_order.extend((0..x.len()).filter(|&i| !seen[i]));
        let mut temp = vec![0.; x.len()];
        if !transpose {
            for (k, &i) in row_order.iter().enumerate() {
                temp[i] = x[k];
            }
            x.copy_from_slice(&temp);
        }
        for step in 0..self.actions.len() {
            if cancel() {
                return Err(RankQrError::Cancelled);
            }
            let k = if transpose {
                step
            } else {
                self.actions.len() - 1 - step
            };
            match &self.actions[k] {
                FrontAction::Reflector(v, beta) => {
                    let dot = *beta * v.iter().map(|&(i, c)| c * x[i]).sum::<f64>();
                    for &(i, c) in v {
                        x[i] -= c * dot;
                    }
                }
                FrontAction::LocalOrthogonal(rows, q) => {
                    let before: Vec<_> = rows.iter().map(|&i| x[i]).collect();
                    for (i, &row) in rows.iter().enumerate() {
                        if cancel() {
                            return Err(RankQrError::Cancelled);
                        }
                        x[row] = (0..rows.len())
                            .map(|j| {
                                if transpose {
                                    q[(j, i)] * before[j]
                                } else {
                                    q[(i, j)] * before[j]
                                }
                            })
                            .sum();
                    }
                }
            }
        }
        if transpose {
            for (k, &i) in row_order.iter().enumerate() {
                temp[k] = x[i];
            }
            x.copy_from_slice(&temp);
        }
        Ok(())
    }
    /// Original-column R entries, plus the entire rejected transformed tail.
    pub fn visit_r(&self, mut visit: impl FnMut(usize, usize, f64)) -> Result<(), RankQrError> {
        self.require_complete()?;
        for (i, row) in self.r.iter().enumerate() {
            for (&j, &v) in &row.values {
                visit(i, self.symbolic.order[j], v);
            }
        }
        let mut remaining: Vec<_> = self.terminal.iter().collect();
        remaining.sort_by_key(|r| r.id);
        for (k, row) in remaining.iter().enumerate() {
            for (&j, &v) in &row.values {
                visit(self.r.len() + k, self.symbolic.order[j], v);
            }
        }
        Ok(())
    }
    /// SSI assessment is independent of completed frontal elimination. Admission
    /// includes the retained front owner as well as the assessment workspace.
    pub fn assess(
        &self,
        options: RankQrOptions,
        cancel: impl FnMut() -> bool,
    ) -> Result<RankQrReport, RankQrError> {
        self.require_complete()?;
        let mut limits = self.limits;
        limits.max_bytes = limits
            .max_bytes
            .checked_sub(self.live_envelope())
            .ok_or(RankQrError::WorkspaceLimit)?;
        limits.max_work = limits
            .max_work
            .checked_sub(self.work)
            .ok_or(RankQrError::WorkLimit)?;
        let mut workspace = RankQrWorkspace::new(&self.symbolic, limits)?;
        workspace.start(self.threshold)?;
        workspace.accepted.extend_from_slice(&self.accepted);
        workspace.deferred.extend(
            self.symbolic
                .order
                .iter()
                .enumerate()
                .filter_map(|(k, &j)| self.rejected[k].then_some(j)),
        );
        for (i, row) in self.r.iter().enumerate() {
            for (&ordered, &v) in &row.values {
                let j = self.symbolic.order[ordered];
                let next = workspace.columns[j].len() + 1;
                reserve(
                    &mut workspace.columns[j],
                    next,
                    &mut workspace.bytes,
                    limits.max_bytes,
                )?;
                workspace.columns[j].push((i, v));
            }
        }
        let mut norm = 0_f64;
        for row in &self.terminal {
            for &v in row.values.values() {
                norm = norm.hypot(v);
            }
        }
        workspace.perturbation = norm;
        workspace.valid = true;
        workspace.running = false;
        workspace.imported_r = true;
        workspace.analyze_rank(options, cancel)
    }
}
