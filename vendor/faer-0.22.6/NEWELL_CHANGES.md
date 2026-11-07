# faer-0.22.6 experimental source ledger

## FAER-COMPAT-004 — approved stable compiler compatibility (2026-11-06)

Human approved the exact twelve-file patch. Martin Harambat remains maintenance
owner. The patch makes 25 existing borrow lifetimes explicit, qualifies macro
reexports, removes redundant imports and parentheses, selects the existing
typed unit value in two GEMM expressions, and removes the unused nrow2 counter
and private Ptr::null method. The approval explicitly covers these edits within
existing unsafe scopes. No unsafe operation is added. The faer version, Rust
1.88 baseline, enabled features, numerical thresholds, and MIT notices remain
unchanged. Verification is recorded in the SolidCore migration dossier.

## FAER-COMPAT-003 — approved feature-conditional warning correction (2026-11-05)

Human approved the exact proposed patch, including compilation conditions around
existing unsafe declarations whose bodies remain unchanged. Maintenance owner:
Martin Harambat. Version0.22.6, Rust1.88, enabled Newell features, numerical
algorithms, thresholds and MIT notices remain unchanged. No lint is disabled.

Changed: src/lib.rs; src/{mat/matmut,col/colmut,row/rowmut,col/colref}.rs;
src/linalg/{matmul/mod,lu/full_pivoting/factor,householder}.rs; src/io.rs;
src/sparse/linalg/rank_front.rs. Rayon/npy-only imports and declarations receive
matching cfg conditions; the Householder temporary is mutable only under Rayon;
ColRef::cloned uses its existing equivalent read method; FrontProgress variants
receive documentation. No existing unsafe body or numerical routine is rewritten,
and no new unsafe, FFI, BLAS or dependency is introduced. Earlier ledger entries
below describe historical approval and ownership at their dates; this entry
supersedes unassigned faer maintenance ownership, not experimental adoption limits.

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

## M50-FAER-002 — bounded numerical column deferral (M5 revision 6)

Changed: `src/sparse/linalg/{qr,rank}.rs`. Additive safe, non-default
`newell-m50` interfaces accept a checked explicit symbolic permutation, extract
compact full Q (including its complement), and apply left/right null operators
and adjoints without a global dense basis. Right-null extraction additionally
factorizes the retained R transpose in original-variable coordinates; callers
must validate the original-matrix residual. This auxiliary work is not free.
No upstream unsafe body changed; no new unsafe, dependency, FFI or BLAS.

Refactorization uses the unchanged original matrix and threshold, moves stable
SSI suspect-column proposals to a terminal partition, and admits at most two
corrective passes after the initial QR. Each pass checks the entire rejected
tail and SSI separation. Proposals alone never certify rank. Rectangular
transpose analysis explicitly labels the permutation axis. Workspace/work
admission is cumulative across passes and measured opaque QR/SVD times are
reported separately from symbolic-call upper bounds.

This is a bounded feasibility extension, not complete rank-job provenance,
resumable fronts, rigorous singular mobility certification or whole-process
memory/cancellation qualification. `rank_front.rs` is not added: the tested
V2.1 chain/grid dependencies do not trigger its conditional escalation. Existing
upstream kernels remain opaque. See the current revision-6 validation section
for exact fixtures, timing results, unresolved requirements and reproduction.
Maintenance ownership remains unassigned; production adoption is not authorized.
