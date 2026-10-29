# faer-0.22.6 experimental source ledger

Baseline: published Cargo archive; source and SHA-256 in ../manifest.toml.
Original MIT notices, version, upstream source and Cargo.toml.orig retained.
All shipped files remain present. No registry edits or production adoption.

## M50-FAER-001 — non-default rank candidate (M5 revision 5)

Changed files: `Cargo.toml` (newell-m50 implies std),
`src/sparse/linalg/mod.rs` (conditional module registration),
`src/sparse/linalg/qr.rs` (additive conditional safe R-entry visitor).
Added `src/sparse/linalg/rank.rs` (safe f64/usize candidate).
No pre-existing unsafe block/function body changed; no new unsafe, FFI or BLAS.
Default factorization/solve paths are unchanged.

The rank module implements the Foster–Davis §2.1 column-rejection contract,
SPQR_SSI and the rank-assessment portion of SPQR_BASIC. New implementation from
the accessible paper, not copied MATLAB/C code. Safe faer thin-SVD kernels are
reused. A left-looking fallback stores compact Householder vectors; safe R-only
imports reuse existing QR for full columns or a checked terminal rejected block.
R-only imports deliberately do not expose apply_q. Optional basic-solution,
complete null-space and SPQR_SSP convenience paths are not implemented.

Method: [Foster–Davis author manuscript](https://people.engr.tamu.edu/davis/publications_files/Reliable_Calculation_of_Numerical_Rank_Null_Space_Bases_Pseudoinverse_Solutions_and_Basic_Solutions_using_SuiteSparseQR.pdf),
§§2.1–2.4 and appendix read for this execution. Reliability remains estimated.
Differences: left-looking factor bridge rather than multifrontal SPQR; conservative
Frobenius stopping norms; both singular-triplet residual halves checked; explicit
zero-rank endpoint, deterministic starts, bounded width/work/storage/cancellation;
no alternate threshold adopted. Default SSI sizes follow the paper.

Evidence and limitations: docs/validation/M5-sketcher.md revision-5 section.
Focused tests live in Newell kernel Sketch probes; no imported dev-dependency
expansion. General fallback grid performance, complete implicit null-space APIs,
actual-worker qualification and reference-hardware acceptance remain open.

Maintenance owner: unassigned; must be assigned before production adoption.
Requalification must cover default parity, permutations, original-matrix residuals,
rank reliability, cancellation, workspace, MSRV, licenses and advisory changes.
