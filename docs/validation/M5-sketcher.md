# M5.0 — Feasibility and requirements

2026-10-28. Executor / Codex. Input: `docs/plans/M5-sketcher.md`, revision 2,
explicitly approved by the human for **M5.0 only**. This is a requirements and
experimental dossier, not a Sketcher implementation or authorization for M5.1–M5.15.

## Revision 5 V1–V4 execution — 2026-10-29 (current disposition)

Executor / Codex, explicitly requested after the human approved plan revision 5.
Started with V2's factor contract. Rust without FFI, BrepKIT alone and all
original requirements/budgets remain binding. **M5.0 remains open.** No M5.1,
production adoption, default runtime switching, agent dispatch, commit or
publication. This section supersedes earlier current-state summaries; historical
reports below remain evidence of their respective earlier executions.

### Entry, baseline and source integrity (V1)

Existing `main`, HEAD `7019c300ff12ff6a1aeee974ca59f865e00b045e`, index empty.
Preserved existing dirty architecture, Cargo, app/kernel/model/render/UI and
planning work, accepted V0 ADR0009, vendor imports and candidate APIs/tests.
Only approved vendor sources/ledgers, test-only kernel Sketch seams, Sketch
probes/benchmark and this dossier changed. The approved revision-5 plan,
architecture, accepted ADRs, backlog and Git index were not edited.

Fresh baseline: check/fmt/strict workspace Clippy/doc tests passed; Nextest
**1,085 passed, 21 skipped**. `INSTA_UPDATE=no`; no snapshot regeneration.
The three published archive SHA-256 values were reverified against the imported
baselines. Comparing every shipped file to its archive found no missing file.
Exact local deltas are now recorded as M50-FAER-001, M50-MATH-001 and
M50-OPERATIONS-001 in `vendor/manifest.toml` and per-package `NEWELL_CHANGES.md`.
Metadata confirms five Newell workspace members, exactly the three named local
packages, and registry brepkit-topology4.2.2. No other package/version was added.

V0 remains accepted and applied. Third-party original MIT/AGPL-3.0-only notices
and Cargo.toml.orig remain present. New correction/extension code is safe Rust;
no pre-existing unsafe body was edited and no new unsafe, FFI or BLAS was added.
The faer change to qr.rs is an additive feature-gated read-only R visitor only.
Default algorithm bodies remain unchanged; all new candidate call sites are
feasibility probes/test-only. Maintenance ownership is still unassigned and is
required before any later production adoption, not waived by these experiments.

### V2 — Implemented factor bridge and rank candidate

Read the accessible Foster–Davis author manuscript §§2.1–2.4, full SSI/BASIC
pseudocode and appendix. Implemented new safe f64/usize numerical code inside
`vendor/faer-0.22.6/src/sparse/linalg/rank.rs`, not a Newell factorization or a
copy of native/MATLAB code. It is conditional on non-default `newell-m50`.

The factor contract is explicit: original-column permutation, sparse R11/R12,
compact Householder transforms, original row mapping and per-rejected-column
perturbation. Completed factorization is distinct from a reliable rank report.
A caller-owned symbolic pattern is borrowed by the workspace; changed input
patterns are rejected. Numeric restarts invalidate previous factors and retain
sparse storage capacities. Resource/cancel/numerical failures expose no partial
consumable factor. The caller must keep numeric values unchanged during a job.

Two bridges are implemented:

- General left-looking column deferral, with row-transversal ordering, sparse
  reflectors and dependency-indexed application. Tests reconstruct original
  columns and check rejection norms, Q/Q^T, rectangular/zero/dependent cases,
  permutations, restart, invalid input, cancellation inside work and budgets.
- Safe R-entry import from existing simplicial/supernodal faer QR. Full-column
  or a terminal rejected block is admitted only after the whole transformed
  rejected tail is bounded, not merely its diagonal. Nonterminal dependence
  returns inadmissible and requires the general bridge. Q remains caller-owned;
  apply_q is explicitly unavailable on an R-only import. Small independent
  Gram comparisons verify R and its permutation against the original matrix.

The SSI/BASIC rank-assessment portion reuses faer's thin SVDs and checks boundary
and small-null-subspace residuals. It reports estimated lower/upper separation,
threshold, rank, perturbation, initialization, iterations, width and conservative
storage admission. Zero/full rank endpoints and threshold failure are explicit;
no alternate threshold is silently accepted. Tests correct the adversarial
matrix [[1,1e8],[0,1]], whose two large QR diagonals hide numerical rank one at
threshold1e-6. SVD/analytic comparisons use three unit scales and three starts.
Many individually small rejected columns that collectively exceed tau produce
Uncertain, not a false zero-rank result.

Deliberate method differences are in the faer patch ledger. In particular,
reliability remains **estimated**, not a rigorous certificate; Frobenius null
residual tests are conservative; both triplet residual halves are checked.
`crates/newell-kernel/src/sketch/rank.rs` is test-only ACL translation and never
infers finite mobility/consistency from infinitesimal nullity. Tangent-circle,
uncertainty, invalid-bound and impossible-rank translations are tested.

#### C1 host characterization: 500 warmed samples per variant

Rust1.88.0, optimized bench, Par::Seq, Apple M4/24GiB/macOS27.2 (26B123); not reference
A (M1/8GiB) or B (Pi5). Each system has10,000 unknowns. Grounded incidence chain
and100x100 grid have analytically full column rank. The deficient variants zero
one original column, giving a known null vector and rank9,999. These are numerical
fixtures, not a complete mixed/nonlinear Sketch workload. Each variant has one
warmup plus500 recorded samples. All2,004 reports were EstimatedReliable with
the analytically expected rank and unchanged tau1e-9.

The table sums each sample's numeric factorization/R-import time and rank-analysis
time before calculating quantiles; it does not add phase percentiles.

| Fixture | Rank | p50 ms | p95 ms | p99 ms | Max ms |
|---|---:|---:|---:|---:|---:|
| Chain regular | 10,000 | 2.565 | 2.690 | 2.732 | 2.868 |
| Chain one zero column | 9,999 | 2.565 | 2.678 | 2.701 | 2.722 |
| Grid regular | 10,000 | 12.475 | 12.765 | 12.993 | 13.806 |
| Grid one zero column | 9,999 | 12.570 | 12.888 | 13.178 | 13.533 |

Reproduce with `NEWELL_M50_PHASE=rank NEWELL_M50_RANK_RUNS=500 cargo +1.88.0
bench -p newell-kernel --bench sketch --features m50-feasibility --locked`.
Raw isolated run: `/tmp/newell-m50-v5/rank-500-isolated.log`. The earlier500-sample
run overlapped verification and is excluded from this table. Original counts,
thresholds, sample count and time budgets were not reduced to get these results.

Storage admission envelopes were approximately34.3MB for chains and69.2MB for
grids, plus caller-owned native QR/input/symbolic allocations. These are
conservative kernel-workspace envelopes, not measured whole-process allocation
high-water marks or the S3 memory gate. The timed pair excludes assembly,
symbolic setup, nonlinear iterations, worker transport, service and cleanup;
it is not an end-to-end C1 or interactive acceptance claim.

#### V2 remaining failures and gates

The generic left-looking bridge hit400,000,001 counted work units on the10,000
unknown grid:22.798s without the row transversal,15.441s with it. Both exceed
C1's5s budget and return WorkLimit. Preserve these failures; the successful
native-R path does not qualify arbitrary nonterminal numerical column deferral.
No larger work allowance, smaller fixture or FFI workaround was used to hide it.

The complete implicit left/right null-space operators and convenience solve APIs
are not implemented. Dense thin-SVD and existing native QR calls remain indivisible;
inner sparse polling tests do not establish their <=16ms acknowledgement or
<=100ms release under every admitted workload. Source/version/scaling ownership
and prepared-result certificates remain caller responsibilities; the test-only
adapter is not a production diagnostic or worker integration. Broad adversarial
large-rank cases, singular finite-mobility/active-bound handling, nonlinear mixed
C1/S traces and hardware acceptance still require evidence. **V2 is partial.**

### V3 — BrepKIT candidates and limits

Preserved earlier feature-isolated affine/quadratic/circle APIs. Added
`intersect_mapped` with original non-unit knot-domain enclosures for the exact
bounded-integer affine domain and a whole-span identity/reversal witness for
positive-weight, clamped single-span rational control nets. Exact control/weight
comparison proves the shared representation without sampled proximity. It does
not assert equivalence for different refinements or general overlapping curves.

The new test exercises degrees1/2/3, non-unit domains, rational reversed weights,
both operand orders, origin/1e9 translation, endpoint correspondences and affine
partial-overlap maps; cancel/resource outcomes remain explicit. Numeric point
reevaluation at1e9 is a sanity check limited by f64 resolution, not a substitute
for the exact common-control-net witness or a general world-error certificate.

All supported candidate tests pass. The unchanged general Bezier clipping still
has the previously reproduced tangency, parameter-order, periodic/seam and
sampled-overlap limitations. General knot/refinement correspondence, all15 family
pairs, curved offsets with joins/self-intersections, automatic arrangements,
region/repair atomicity and M4.D1/D2 advanced paths remain unqualified. Circle
extrusion and the new exact domains do not close those requirements. **V3 is
partial; no missing family or geometry accuracy requirement was removed.**

### V4 — Checks and qualification disposition

Final checks on this source, snapshot updates disabled:

| Check | Result |
|---|---|
| cargo check --workspace --locked | Passed |
| cargo fmt --all -- --check | Passed |
| cargo clippy --workspace --all-targets --all-features --locked -- -D warnings | Passed;50 inherited faer Rust1.99 warnings remain dependency warnings, unchanged in count from baseline |
| INSTA_UPDATE=no cargo nextest run --workspace --all-features --locked | 1,093 passed,21 skipped;45.359s |
| cargo test --workspace --doc --locked | Passed |
| cargo test -p newell-render --locked -- --ignored | 14 GPU tests passed; no physical Sketch UI acceptance inferred |
| cargo +1.88.0 check --workspace --all-features --locked | Passed |
| INSTA_UPDATE=no cargo +1.88.0 test -p newell-kernel --all-features --locked m50_ | 7 focused tests passed |
| INSTA_UPDATE=no cargo nextest run -p newell-kernel --locked (default features) | 68 passed,1 skipped |
| INSTA_UPDATE=no cargo test -p newell-kernel --all-features --locked candidate -- --nocapture | 4 candidate geometry tests passed, including all earlier admitted domains |
| cargo deny check | advisories/bans/licenses/sources passed; existing policy warnings retained |

Workspace checks used rustc1.99.0; the explicit MSRV checks above used1.88.0.
No new workspace failure remains after local fixes. Intermediate issues were a
probe helper name and unused helper/conditional style warnings; corrected without
lint suppression. Existing faer warnings and block0.1.6 future incompatibility
were retained and reported, not cleaned up outside scope. Cargo-audit is not
installed; the available cargo-deny advisory check passed without policy edits.
Baseline/final logs and raw traces are under `/tmp/newell-m50-v5/`; this dossier
and vendor ledgers retain the durable results when temporary logs are removed.

| Lot | Disposition |
|---|---|
| V1 | Named import/isolation/provenance verified; original notices and baselines preserved |
| V2 | Factor contract and rank candidate implemented and tested; full qualification blocked by the general-deferral timing failure and missing gates above |
| V3 | Exact admitted domains improved; general geometry/offset/arrangement coverage remains missing |
| V4 | Workspace, MSRV, security/license and focused numerical qualification performed; mandatory full-path/hardware/geometry acceptance incomplete |

Actual-worker mixed500-sample frozen-query/confirmation/cancellation fairness,
retained-generation S3 allocation accounting, complete C/D geometry and physical
reference-device results are still missing. Existing worker preparation-only
probes remain baselines, not rank/geometry end-to-end proofs. No production call
site was switched and no completion checkbox was changed. The specific next
technical work is the general sparse deferral/performance path, complete implicit
null operators, remaining certified geometry and real-worker qualification.
There is no new request for generic vendoring permission or a reduced budget.

## Revision 3 recovery execution — 2026-10-29 (historical disposition)

Executor / Codex. The human explicitly approved revision 3 and requested M5.0
packages A–G, Rust without FFI, BrepKIT only, unchanged requirements, no M5.1.
This section supersedes earlier current-source/check summaries, preserving their
historical evidence below. **M5.0 remains open.** Investigations and bounded
probes progressed; the complete recovery exit gates have not been achieved.
No dependency adoption, source override, upstream publication or accepted ADR
amendment is inferred from approval of these packages.

### A — Fresh baseline and prerequisite

Existing main checkout/HEAD `7019c300ff12ff6a1aeee974ca59f865e00b045e`; empty
index at entry. Existing dirty architecture, Cargo, app/kernel/model/render/UI,
backlog/plans and untracked M4/M5 evidence/probes were preserved. Revision 3 is
an existing unstaged input. No other agent, checkout, branch, stash or snapshot.
Current source includes M4's cancellation/test-support additions; these were
read and reused, not overwritten. The earlier navigation/glyph failures are
absent: fresh check, formatting, strict Clippy, doc tests passed; Nextest
**1,078 passed, zero failed,21 skipped** (79.394 s). Snapshots never auto-update.
This is a fresh baseline, not an inference from the M4 report.

M4 records prototype implementation complete but human/device acceptance pending;
M4.D1 nonlinear curved-support correspondence and M4.D2 general trimmed rational
targets are explicitly deferred. They are not M5 exemptions. Ordinary face/cubic/
circular-hole extrusion fixtures use supported paths; general offset/region
Add/Remove can encounter D1, and trimmed rational target projection/termination
can encounter D2. Those paths require their original exact/provenance/atomic
criteria before M5 acceptance; the backlog was not checked or rewritten.

Host remains Apple M4/24 GiB/macOS27.2; battery with configured Low Power Mode1.
Final new release measurements use **Rust1.88.0**, `Par::Seq`, ordinary optimized
bench build, one numerical thread. They are host characterization, not reference
A/B acceptance. No Pi5/reference-A hardware or physical Sketch UI is available.

### B — Numerical method inventory and executable evidence

No numerical dependency or pin was changed. Investigated candidates:

| Candidate | Concrete API/method / compatibility evidence | Disposition |
|---|---|---|
| faer=0.22.6, MIT, MSRV1.84 | Existing pinned source: non-pivoted sparse QR; safe public supernodal factor/solve with reusable workspace; bounded dense SVD. Current Rust1.88 probes executed. | Solve candidate retained; no generic sparse numerical rank certificate. |
| rsparse1.2.1 | [Official API](https://docs.rs/rsparse/1.2.1/): `qr`, `qrsol`, `sqr`; direct sparse solves. No demonstrated rank-reliability/threshold/uncertainty API. | Not selected; no adoption/build. Full transitive MSRV/license/platform audit remains unperformed because method suitability has not passed. No unsupported compatibility assertion. |
| russell_sparse3.3.1 | [Official documentation](https://docs.rs/russell_sparse/3.3.1/): native UMFPACK/MUMPS/cuDSS and non-Rust libraries. | Incompatible with this no-FFI scope; not installed or benchmarked. |
| Upstream faer rank extension | Required proposed contract: undamped scaled CSC input, numerical rank threshold and reliability/uncertainty, null-space residual checks, explicit workspace bounds and bounded-call/continuation behavior. | Research/API gap, not an existing tested method. Requires concrete upstream source/adoption scope before integration; no private Newell factorization written. |

This shortlist does not establish that no suitable Rust library can ever exist.
It establishes that **no eligible large-rank implementation was demonstrated or
adopted in this execution**. An API name, R diagonal or incidence rank is not a
substitute. B's mandatory exit remains blocked; there is no FFI fallback.

Added `geometric_chain`: actual point/segment length residuals and analytic
Jacobian assembled from off-axis perturbed coordinates, two Newton updates using
safe faer supernodal QR. Distances are independently checked against the known
positive-x unit chain; y constraints and the x anchor are explicit equations.
There are10,002 points and10,001 finite segments (20,003 entities),20,004 scalar
unknowns/equations,50,007 Jacobian nonzeros, one connected block, zero regions/
intersections/visible packets. After direct y/anchor eliminations there would
still be10,001 free x coordinates; this probe deliberately assembles the full
system. Rank/DOF remains Unavailable; nonlinear convergence is not rank proof.

500 warmed complete jobs: p50=7.856 ms,p95=15.883 ms,p99=39.745 ms,max=109.599 ms;
max dimensional residual1.82e-12 mm, solution error1.10e-11 mm. The full-job timer
includes setup, checks and trace output; phase logs separate assembly/symbolic/
factor/solve. The maximum is **not** one indivisible call or cancellation result.
Symbolic structure is reused per solve, but numerical workspace is currently
reallocated per iteration and explicitly reported as such. This is one nonlinear
chain branch, not C1 grid, singular diagnosis, trust-region or full tool coverage.
The bounded32-point correctness test independently asserts residual/coordinates.

Current forced-supernodal500-sample chain/grid traces and their setup, scratch,
cancellation/service-surrogate records are retained below. They still do not
supply a rank method or prove the real Sketch scheduler.

### C — Pinned operation failures and precise upstream correction scope

Reexecuted all15 family pairs at origin and1e9 using the pinned exact curve API;
parabola tangency/near crossings and periodic/figure-eight roots remain missing,
parameter pairs can disagree by≈.666667 mm, partial/reversed overlaps still hit
the2-second watchdog. Watchdog termination protects this experiment only.
An independent affine overlap oracle now checks interval endpoints and parameter
maps at both translations: A(u)=-2+4u; B(v)=3v or1-2v. These prove the expected
partial/reversed intervals, not a repaired upstream implementation.

Concrete proposed upstream changes to `brepkit-math-4.2.2/src/nurbs/bezier_clip.rs`:

1. Carry original curve ordering and original parameter maps through recursive
   role swaps, subdivision and merging; return/validate both original parameters.
2. Dispatch provably affine collinear spans to exact interval clipping with
   orientation-qualified endpoint maps; separate near-collinearity/uncertainty.
3. Replace sampled overlap acceptance and midpoint/depth-exhaustion hits with
   checked root/contact/overlap classification, explicit unresolved outcome
   and work limits. Correct homogeneous rational bounds and endpoint/seam
   handling; prove tangencies and nonadjacent self contacts rather than merely
   accepting a small residual at one seed.
4. Add bounded advancement/cancellation and workspace admission; retain all
   adversarial fixtures, reversed operands and independent root expectations.

These are concrete algorithm/API requirements, **not an untested patch claimed
correct**. No installed registry source, pin or accepted ADR was altered. A
candidate upstream source revision does not yet exist in this dossier, so the
required exact patch/fork adoption decision cannot honestly be represented as
already approved. C's corrected-backend exit remains unsatisfied. The proposed
source/API scope above is the next integration work requiring explicit scope
and ADR0004 authorization; no external issue/PR was sent.

### D — Independent exact paths, text precision and remaining geometry

Added test-only kernel `src/sketch/{mod,curve}.rs`, compiled only with `cfg(test)`.
For a smooth full circle, offset is independently radius r+d; join policy has
no corner to resolve. Native circles at radius1.75/2.25, origin/1e9, extruded2 mm
have valid topology and analytic volumes19.24225500323748/31.808625617596654 mm³.
This establishes the **restricted analytic construction path**, not a fixed
`offset_wire_with_join`, arbitrary arc joins, holes or self-intersection cleanup.
The prior chordal offset failure is unchanged. General certified offsets and
arrangements depend on the unresolved C contracts; no screen polygon is accepted.

Raw outlines for all94 visible printable ASCII glyphs plus valid no-ink space
were inspected:9,146 coordinate components are integer/half font units and exact
in f32 before promotion. At10 mm/em with45-degree rotation and1e9 translation,
maximum measured coordinate round-trip error is5.96e-8 mm. This does not certify
all derivatives/intersections/closure or arbitrary transformed text. O/B/8 exact
M4 hole tests remain in the suite. Full ASCII topology/region conversion and
transformed M4 consumption are still Unproven; a precision subset is not a full
Text implementation. Existing f64 spacing/tolerance limits remain unchanged.

No general admitted scale domain or new geometry representation is claimed.
No alternative kernel, new parser, tolerance increase or disabled tool.

### E — Real worker service, and what remains absent

Extended the existing actual native-Boolean cancellation/recovery test, reusing
its entry/return hook and real `ExtrudeGeometrySession::cancel`/prepared authority.
After cancellation, the same actual worker completes an ordinary recovery job,
then serves a distinct session certificate2, serial4, with the current document
source. The additional result is released without a second document publication;
its source revision and session identity are asserted. This exercises actual
worker execution/cleanup, not the earlier tiny-SVD service surrogate.

Focused debug observation: native batch7.814 ms, cancel reducer0.583 µs,
noninterruptible remainder1.532 ms, next real preparation151.420 ms; subsequent
session2 preparation148.146 ms queue-to-completed. These are one-shot **debug**
observations, not reference-hardware timing acceptance; queue-to-completed includes
actual geometry/mesh work. Different certificates are not two physical windows.

The frozen Sketch query/input route and bounded press/final/interactive/motion
continuation pump are **not implemented or proven**. Their exact curve-query/
region/rank backends are still missing. No PrepareGeometry edit is relabelled as
a certified press query. A complete E harness/ordering and sustained-motion/final
fairness proof remains required; neither this recovery test nor late-result
rejection closes E. Production worker behavior is unchanged by this extension.

### F/G — Coverage and closure disposition

A fresh nonlinear connected chain and500 operations replace scalar-only evidence
for that specific branch. Full mixed S1/S2/S3 counts, nonlinear C1 grid, all advanced
constraints, global-dependency traces, bounded workspace/retained-generation/copy
and visible publication checks remain missing. The existing scalar S-class tests
are retained and never relabelled as full geometry acceptance. A/B hardware and
physical input evidence remain unavailable. No weakening or hardware exception.

| Package | Current outcome |
|---|---|
| A | Fresh automated baseline established; M4 human/device acceptance still pending, not inferred from backlog. |
| B | Safe solve/nonlinear probes progressed; generic large rank blocked by missing demonstrated eligible method/API. |
| C | Reproductions and independent interval maps established; corrected upstream source/adoption and certified general roots remain absent. |
| D | Analytic circle offset and printable-ASCII coordinate precision subsets pass; general certified offsets/arrangements remain unproven. |
| E | Actual worker cancellation and subsequent distinct-session preparation measured; complete real Sketch query/service pump not proven. |
| F |500 nonlinear chain jobs measured; full accepted mixed classes/grid/end-to-end evidence not complete. |
| G | Reconciled dossier delivered with negative exit assessment; **M5.0 not checked**, no M5.1 started. |

The requested A–G investigation has therefore progressed but is **not a complete
successful implementation of every recovery package**. Mandatory unresolved
contracts remain visible. No dependency/patch was adopted merely to make this
report green; no automatic Architect/Challenger/Executor cycle is started.

### Changed files and reproducible commands for this continuation

Changes: kernel `src/lib.rs` only adds `cfg(test) mod sketch`; new
`src/sketch/{mod,curve}.rs`; existing `tests/it/sketch_solver.rs` and
`benches/sketch.rs` add the nonlinear chain and driver; app `geometry_worker.rs`
extends only its existing test; this report. Existing manifests/pins and the
Git index are preserved. No production Sketch session, feature, journal or UI.

```sh
cargo test -p newell-kernel --lib sketch:: --locked -- --nocapture
cargo test -p newell-kernel --test it nonlinear_point_segment_chain_matches_analytic_solution --locked -- --nocapture
cargo test -p newell-app --lib cancellation_during_native_boolean_returns_promptly_rejects_late_work_and_recovers --locked -- --nocapture
cargo +1.88.0 bench -p newell-kernel --bench sketch --no-run --locked
# Use the executable printed by Cargo; observed on this host:
NEWELL_M50_PHASE=geometric /usr/bin/time -l target/release/deps/sketch-0955dd7c683b7b5f
NEWELL_M50_PHASE=qr-config NEWELL_M50_QR_MODE=supernodal /usr/bin/time -l target/release/deps/sketch-0955dd7c683b7b5f
NEWELL_M50_PHASE=curves target/release/deps/sketch-0955dd7c683b7b5f
```

### Revision-3 final checks and measured traces

| Check | Observed result |
|---|---|
| `cargo check --workspace --locked` | Passed. |
| `cargo fmt --all -- --check` | Passed. Only assigned source files formatted during this continuation. |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Passed; no lint disabled. |
| `INSTA_UPDATE=no cargo nextest run --workspace --all-features --locked --no-fail-fast` | **1,082 passed, zero failed,21 skipped**,41.893 s. Passing dossier tests do not certify their recorded negative capabilities. |
| `cargo test --workspace --doc --locked` | Passed. |
| `cargo +1.88.0 check --workspace --all-targets --all-features --locked` | Passed on this host; other architectures untested. |
| `git diff --check` | Passed. |

No new dependency or pin requires a changed dependency audit; no external
package was installed. Existing block0.1.6 future-compatibility notice remains
informational. GPU/application/physical acceptance was not run for test-only
CPU changes. Index preserved; no commit or M5.1 implementation. The following
are actual outputs; repeated identical chain logs are compacted, not resampled.

```text
M5.0,seed=0,iterations=500,parallelism=Seq,units=mm
LATENCY,r3_geometric_chain_full_job,n=500,p50_us=7856.208,p95_us=15882.708,p99_us=39744.958,max_us=109599.250
        5.11 real         4.19 user         0.06 sys
            25608192  maximum resident set size
            24379848  peak memory footprint
GEOMETRIC_CHAIN,points=10002,segments=10001,entities=20003,unknowns=20004,equations=20004,nnz=50007,blocks=1,intersections=0,regions=0,visible=0,iterations=2,residual_mm=1.8189894035458565e-12,solution_error_mm=1.0913936421275139e-11,total_us=12057.875,rank=Unavailable,workspace_reuse=not_yet
GEOMETRIC_PHASE,iteration=0,assembly_us=2205.792,symbolic_us=2790.833,factor_us=2122.041,solve_us=784.625
GEOMETRIC_PHASE,iteration=1,assembly_us=1066.333,symbolic_us=0.000,factor_us=2173.542,solve_us=690.791
```

```text
M5.0,seed=0,iterations=500,parallelism=Seq,units=mm
LATENCY,configured_supernodal_grid_false_symbolic,n=500,p50_us=712.791,p95_us=746.250,p99_us=762.459,max_us=784.834
CANCEL,configured_supernodal_grid_false_symbolic,atomic_latch_us=0.000,indivisible_us=743.291,worker_release_us=738.667,frozen_click_surrogate_us=750.667,other_window_surrogate_us=751.500,in_flight=true
QR_CONFIG,supernodal,grid=false,setup_us=819.542,reused_workspace_bytes=4390424,rank_certification=Unavailable
LATENCY,configured_supernodal_grid_false_numeric,n=500,p50_us=757.458,p95_us=779.625,p99_us=828.250,max_us=919.500
LATENCY,configured_supernodal_grid_false_solve,n=500,p50_us=211.750,p95_us=224.916,p99_us=242.500,max_us=262.750
CANCEL,configured_supernodal_grid_false_numeric,atomic_latch_us=0.041,indivisible_us=757.458,worker_release_us=750.291,frozen_click_surrogate_us=754.666,other_window_surrogate_us=755.416,in_flight=true
LATENCY,configured_supernodal_grid_true_symbolic,n=500,p50_us=2058.458,p95_us=2114.166,p99_us=2236.542,max_us=2372.250
CANCEL,configured_supernodal_grid_true_symbolic,atomic_latch_us=0.042,indivisible_us=2023.042,worker_release_us=2016.417,frozen_click_surrogate_us=2021.667,other_window_surrogate_us=2022.792,in_flight=true
QR_CONFIG,supernodal,grid=true,setup_us=2628.501,reused_workspace_bytes=18777952,rank_certification=Unavailable
LATENCY,configured_supernodal_grid_true_numeric,n=500,p50_us=6167.333,p95_us=6475.750,p99_us=6701.333,max_us=6839.417
LATENCY,configured_supernodal_grid_true_solve,n=500,p50_us=814.500,p95_us=878.666,p99_us=917.667,max_us=1365.209
CANCEL,configured_supernodal_grid_true_numeric,atomic_latch_us=0.000,indivisible_us=6195.167,worker_release_us=6187.166,frozen_click_surrogate_us=6191.333,other_window_surrogate_us=6191.958,in_flight=true
        5.42 real         5.40 user         0.01 sys
            30048256  maximum resident set size
                   0  average shared memory size
                   0  average unshared data size
                   0  average unshared stack size
                2023  page reclaims
                   0  page faults
                   0  swaps
                   0  block input operations
                   0  block output operations
                   0  messages sent
                   0  messages received
                   0  signals received
                   0  voluntary context switches
                 122  involuntary context switches
        123036524550  instructions retired
         21862375881  cycles elapsed
            28426672  peak memory footprint
```

```text
   Compiling newell-kernel v0.1.0 (/Users/martin/Desktop/Newell/crates/newell-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.02s
     Running unittests src/lib.rs (target/debug/deps/newell_kernel-c4db60f6278a531e)

running 3 tests
R3_OVERLAP_ORACLE,shift=0e0,direction=3,endpoints=[(0.5, 0.0), (1.0, 0.6666666666666666)]
R3_OVERLAP_ORACLE,shift=0e0,direction=-2,endpoints=[(0.25, 1.0), (0.75, -0.0)]
R3_OVERLAP_ORACLE,shift=1e9,direction=3,endpoints=[(0.5, 0.0), (1.0, 0.6666666666666666)]
R3_OVERLAP_ORACLE,shift=1e9,direction=-2,endpoints=[(0.25, 1.0), (0.75, -0.0)]
test sketch::curve::tests::affine_partial_and_reversed_overlap_have_independent_parameter_maps ... ok
R3_ANALYTIC_OFFSET,shift=0e0,distance=-0.25,radius=1.75,expected_volume=19.24225500323748,volume=19.24225500323748,elapsed_us=3985.167,scope=smooth_circle_only
R3_ANALYTIC_OFFSET,shift=0e0,distance=0.25,radius=2.25,expected_volume=31.808625617596654,volume=31.808625617596654,elapsed_us=158.583,scope=smooth_circle_only
R3_ANALYTIC_OFFSET,shift=1e9,distance=-0.25,radius=1.75,expected_volume=19.24225500323748,volume=19.24225500323748,elapsed_us=109.666,scope=smooth_circle_only
R3_ASCII,ink_glyphs=94,space=valid_no_ink,coordinates=9146,max_rotated_translation_error_mm=5.958085314006212e-8,full_font_arrangement=Unproven
test sketch::curve::tests::printable_ascii_raw_outline_precision_and_rigid_transforms ... ok
R3_ANALYTIC_OFFSET,shift=1e9,distance=0.25,radius=2.25,expected_volume=31.808625617596654,volume=31.808625617596654,elapsed_us=108.875,scope=smooth_circle_only
test sketch::curve::tests::analytic_circle_offset_remains_exact_through_native_extrusion ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 26 filtered out; finished in 0.01s

```

```text
   Compiling newell-kernel v0.1.0 (/Users/martin/Desktop/Newell/crates/newell-kernel)
   Compiling newell-render v0.1.0 (/Users/martin/Desktop/Newell/crates/newell-render)
   Compiling newell-app v0.1.0 (/Users/martin/Desktop/Newell/crates/newell-app)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.29s
warning: the following packages contain code that will be rejected by a future version of Rust: block v0.1.6
note: to see what the problems were, use the option `--future-incompat-report`, or run `cargo report future-incompatibilities --id 1`
     Running unittests src/lib.rs (target/debug/deps/newell_app-eb900c43bf0ab160)

running 1 test
native Boolean batch until cooperative boundary: 7.81425ms; cancel reducer: 583ns; uninterruptible remainder: 1.531791ms; subsequent real preparation: 151.419709ms
R3_REAL_WORKER,other_session=2,queue_to_completed_us=148146.208,committed_revision_unchanged=true,press_query_and_full_sketch_scheduler=Unproven
test geometry_worker::tests::cancellation_during_native_boolean_returns_promptly_rejects_late_work_and_recovers ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 145 filtered out; finished in 0.55s

```


## Assessment

**M5.0 exit criteria are not satisfied.** Bounded dense rank and specific exact
profiles are demonstrated. A safe public faer configuration substantially improves
large sparse solve/cancellation boundaries. However, a vetted generic large-block
numerical rank method is still unavailable in the approved backend; pinned curve
contacts, parameter provenance, overlaps and curved offsets fail mandatory cases.
General exact arrangements, admitted scale limits and real Sketch worker/UI service
remain unproven. No failed mandatory gate is marked green. The M5.0 backlog checkbox
remains unchecked; no dependent production slice is started.

`Proven` means only the specified fixture, operation and checks. `Unsupported`
means a demonstrated failure or missing required API in the investigated path,
not that no future algorithm could work. `Unproven` means insufficient evidence.
Passing a data-collection test does not certify the capabilities it records as failing.

## Authority, prerequisite and starting state

| Decision | Authority/status for this execution | Feasibility consequence |
|---|---|---|
| D1 — scoped Sketch graphics/input | Human approved revision 2 for M5.0. No Sketch specification amendment applied here. | Keep stroke/handle, clipping, input authority and draw budgets; this CPU slice grants no production rendering exception. |
| D2 — controls/reference corrections | Same approval, no Sketch artwork/UI amendment here. | Preserve complete tools and controls, 27-cell Sketch panel, existing numeric/Selection grammar. |
| D3 — support-plane camera/input | Same approval, no Sketch plane-camera amendment here. | Retain one Controller camera; a future Sketch-specific amendment remains required. Other navigation changes in this checkout are independent inputs. |
| D4 — complete M5 coverage | Approved requirements baseline for investigation. | Conics, splines, text, references, patterns, all constraints and regions remain required. Failed capabilities are not removed. |
| D5 — solver/query strategy | Approved candidates for feasibility. | Exact **dev-only** faer 0.22.6, rstar 0.12.2 and ab_glyph 0.2.32. No production adoption, BLAS/FFI, kernel upgrade, alternative B-Rep engine or unapproved dependency. |

The request explicitly authorizes this feasibility slice; it does not claim that
D1–D3 amendments have already been applied. M4 prerequisite: the records inspected
contain human acceptance of **M4.4**, not acceptance of the full current M4 command,
parameter and geometry pipeline. Real M4 implementations/tests exist. Unchecked
backlog boxes were not treated as concrete proof of missing behavior. Full M4 human
acceptance is **unverified**, rather than a demonstrated functional failure or an
excuse to stop independent non-production M5.0 experiments.

Checkout: `/Users/martin/Desktop/Newell`, `main`, HEAD
`7019c300ff12ff6a1aeee974ca59f865e00b045e`. Index empty at initial inspection.
Initial dirty paths: `Description.md`, app `geometry_worker.rs`, kernel
`{boolean,extrude,surface,termination}.rs`, model `geometry.rs`, `docs/backlog.md`,
Selection alignment plan, `Interface_Elements.txt` and `README_UI.md`.
Concurrent execution replaced the probe driver during the first run. Writes were
paused; the human subsequently confirmed the other execution stopped and explicitly
instructed this chat to continue. Its resulting probes/report were inspected and
reconciled. Further independent navigation, model, geometry and UI edits appeared
during verification, including architecture/earlier-plan amendments; they were
preserved, not attributed to M5.0. Earlier check runs are not claimed to certify
subsequently changed sources. Historical raw records are retained separately below;
the final-run sections and checks here supersede their conclusions.

No branch, worktree, snapshot, stash, commit, agent dispatch or external publication.
This slice changes no production behavior. `src/lib.rs` additions are `cfg(test)` only.

## Changed feasibility files

- `crates/newell-kernel/Cargo.toml` and `Cargo.lock`: exact development dependencies,
  default numerical features disabled, `std`/`sparse-linalg` only, benchmark registration.
- `crates/newell-kernel/benches/sketch.rs`: deterministic executable driver and phase selectors.
- `crates/newell-kernel/tests/it/{sketch_solver,sketch_curves,sketch_regions}.rs`,
  registered once in `tests/it/main.rs`: numerical, contacts, tolerance, offset,
  exact-profile, resource and cancellation experiments.
- `crates/newell-kernel/src/lib.rs`: test-only access to the actual M4 arena for
  oriented/translated holed profiles, cubic profiles and exact O/B/8 contours.
- This validation report. Existing backlog edits are preserved; M5.0 status is not changed.

## Environment and reproducibility

Observed host: Apple M4, 10 CPU cores, 24 GiB RAM, aarch64-apple-darwin;
macOS 27.2 build 26B123; integrated 10-core Apple GPU, built-in 2560×1664 display.
Battery power and configured Low Power Mode=1 were observed, not changed. This is
**neither reference A** (M1-class/8 GiB/1920×1080) **nor reference B** (Pi 5/Linux).
No viewport was rendered for these experiments. Shared-machine load and power state
are not controlled performance-hardware acceptance conditions.

The final measured executable was built with **rustc 1.99.0** (b940084d7,
LLVM 23.1.1), Cargo 1.99.0, normal optimized Cargo bench profile, no custom CPU flags.
Rust **1.88.0 compatibility is checked separately**; historical 1.88 measurements
below are not mixed into the final 1.99 distributions. faer explicitly uses `Par::Seq`;
no nested numerical pool. Cancellation tests use one scoped worker. Contact watchdog
subprocesses protect experiments only; process killing is not adopted as cooperative
production cancellation.

Commands from the checkout:

```sh
cargo bench -p newell-kernel --bench sketch --locked --no-run
# Run the executable printed by Cargo; final 1.99 path on this host:
/usr/bin/time -l /Users/martin/Desktop/Newell/target/release/deps/sketch-609353609db3eed6
NEWELL_M50_PHASE=qr-config NEWELL_M50_QR_MODE=supernodal /usr/bin/time -l /Users/martin/Desktop/Newell/target/release/deps/sketch-609353609db3eed6
# Optional exploration: both simplicial and supernodal configurations, five samples:
NEWELL_M50_PHASE=qr-config NEWELL_M50_QR_RUNS=5 /Users/martin/Desktop/Newell/target/release/deps/sketch-609353609db3eed6
INSTA_UPDATE=no cargo test -p newell-kernel --test it --locked sketch_ -- --nocapture
cargo test -p newell-kernel --lib --locked m50_ -- --nocapture
```

Selectors `scale`, `cancellation`, `curves`, `regions` isolate other dossiers.
Fixtures are deterministic (version 1, no random generator); parameter edits use
`i*1e-6`, spatial queries use `i*97 % count`. Sparse timing loops discard a warm-up;
dense 256-column SVD also warms before 500 samples. Geometry timing loops follow
construction/contact warm-up cases. Percentiles use nearest-rank selection.
One-shot setup, cancellation, adversarial cases and whole-scalar-validation timings
are labelled separately from 500-sample distributions. Units are mm, radians,
microseconds for raw timings, mm³ for volumes.

No GUI/GPU run is needed to measure these CPU/backend boundaries; no physical or
visual acceptance is claimed. Any application validation must build and launch the
current checkout's absolute executable, never the obsolete OneDrive app by name.

## Dependency and API inventory

Pinned local registry sources under
`/Users/martin/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` were inspected;
none were patched.

| Dependency | Pin / declared MSRV / license | Investigated public APIs |
|---|---|---|
| brepkit math/operations/topology | =4.2.2 / 1.88 / AGPL-3.0-only | Curve evaluation, clipping, topology builders, native extrusion, measurement/validation; sole B-Rep engine. |
| faer | =0.22.6 / 1.84 / MIT | `Mat::singular_values`, dense column-pivoted QR reference; high-level `SymbolicQr`/`Qr`; safe low-level `factorize_symbolic_qr`, `factorize_numeric_qr`, `solve_in_place_with_conj`, workspace size APIs. |
| rstar | =0.12.2 / 1.63 / MIT option | Bulk load, envelope query, remove/insert; point fixtures only. |
| ab_glyph | =0.2.32 / undeclared / Apache-2.0 | Raw `Font::outline` line/quadratic/cubic curves, not raster output. |
| Numerical transitives | faer-traits 0.22.1, pulp 0.21.5, dyn-stack 0.13.2, gemm 0.18.2, qd 0.7.7 / MIT | No direct additions beyond approved candidates; safe library interfaces used, no unsafe Newell adapter. |
| Font parser transitive | ttf-parser 0.25.1 / 1.63 / MIT OR Apache-2.0 | No alternate direct parser adopted. |

Rust 1.88 and dependency-policy check results are recorded below. Compilation on
this aarch64 Mac does not establish all supported architectures. License/advisory
policy was not weakened. fontTools 4.60.1, already present as a local inspection
tool, independently supplied bundled glyph contour counts and signed polynomial
areas; it was not added to the project or used as a production font dependency.

## Numerical rank, solve and scaling — §§6.1, 9.2

Bounded reference: **undamped dense SVD**, at most 256 columns. Threshold:
`factor * max(1e-10, 1e-9*sigma_max)`, factors 0.1, 1, 10. Stable regular analytic
fixtures have unchanged rank under these perturbations. Near-threshold/raw-scale
cases are **uncertain**, not rounded integer DOF. No damped system certifies rank.

| Case | Independent result and required interpretation |
|---|---|
| Four-coordinate fixed-length segment | Rank 1; three regular planar rigid freedoms. |
| Duplicated equations | Rank unchanged by duplication; equation count is not rank. |
| Toggle | Rank 1 at a singular configuration; finite mobility not certified. |
| Point on two fixed tangent unit circles | Rows [2,0],[-2,0]: rank 1, infinitesimal nullity 1. Subtract original equations to obtain x=0, then y²=0: isolated real solution, finite DOF 0. |
| Active parameter/contact bound | Equality rank and active inequality restrictions remain separate; feasible mobility not certified. |
| Grounded regular 2×2 | Rank 2, zero regular interior DOF. |
| Upstream GCS oracle | Distance 5, y=0, positive-x branch solves (5,0), residual checked independently. DogLeg success proves neither rank nor consistency classification generally. |
| 64-column sparse/dense overlap | Chain 64 equations/127 nonzeros; 8×8 grid 113 equations/225 nonzeros. Sparse solution agrees with dense column-pivoted QR; dense SVD rank 64; residual and solution difference <1e-9. |

Final focused comparison measured maximum sparse/dense solution differences
3.89e-15 (chain) and5.55e-16 (grid), and maximum residuals1.20e-16 and
2.86e-16 respectively. Reproduce with
`cargo test -p newell-kernel --test it sparse_chain_and_grid_agree_with_bounded_dense_svd_reference --locked -- --nocapture`.

Raw scale examples at 1e-9, 1 and 1e9 demonstrate threshold sensitivity. A complete
nonlinear normalization/analytic-derivative implementation is not inferred from
these matrices. Final SVD of a 256×256 tridiagonal reference is measured separately.

C1 backend fixtures: 10,000 scalar unknowns, one connected block. Chain:
10,000 equations, 19,999 nonzeros; 100×100 grid: 19,801 equations, 39,601 nonzeros.
Anchor 0, horizontal differences .01 and vertical differences 1 give independently
known `x_i=.01*i`. Their maximum solution errors in the final high-level run were
1.21e-9 (chain) and 2.56e-13 (grid). These are **linear backend workloads**, not
complete nonlinear Sketches. Numerical convergence/residual, regular rank,
singularity and finite mobility are not conflated.

**Generic large-block numerical rank: Unsupported by the investigated contract.**
Pinned faer sparse QR uses non-pivoted QR and exposes no vetted numerical rank
certificate. Column ordering, R diagonals, incidence matching, known analytic
fixture rank or a successful solve do not replace that API. Sparse SVD/rank-revealing
QR was not demonstrated. No unbounded block was densified and no custom factorization
was written. This remains a mandatory gate for M5.2/M5.10/M5.15.

## Cancellation, configuration, scheduling and resources — §6.2

High-level faer `Qr::try_new_with_symbolic` allocates numerical arrays/scratch on
each call and selects its default sparse strategy. On the chain, its final numeric
p50 is 147.432 ms, maximum 392.513 ms; one in-flight cancellation waits 147.657 ms
for release. **That default path fails the 100 ms accepted-case requirement.**

A compatible **local implementation choice within D5** was therefore investigated:
public `QrSymbolicParams` with `SupernodalThreshold::FORCE_SUPERNODAL`, then safe
low-level numeric/solve calls with preallocated reusable index/value/scratch buffers.
No dependency or equation changes. FORCE_SIMPLICIAL exploration (five samples) is
also retained: grid numeric operations near .95–1.17 s fail the budget; forcing
supernodal avoids that failure on both fixtures. The chosen configuration is
measured for 500 warmed operations, separately for chain and grid.

| Operation (500 samples; ms) | p50 | p95 | p99 | Maximum |
|---|---:|---:|---:|---:|
| Chain symbolic | 1.353292 | 1.379917 | 1.393333 | 1.426166 |
| Chain numeric | 1.434667 | 1.459667 | 1.476625 | 1.510208 |
| Chain solve | 0.391209 | 0.397041 | 0.402958 | 0.423625 |
| Grid symbolic | 3.806875 | 3.922958 | 4.197209 | 4.454583 |
| Grid numeric | 11.510042 | 18.975292 | 24.537375 | 32.918584 |
| Grid solve | 1.524042 | 4.172416 | 4.767333 | 22.121750 |

Maximum grid numeric duration was12.171 ms in the first configured500-sample
run and32.919 ms in the final repeat; the final repeat above is retained rather
than selecting the faster run. Observed final in-flight worker release was1.448 ms
for chain numeric and13.316 ms for grid numeric; frozen-click/other-window
surrogates completed at1.460/1.461 ms and13.326/13.327 ms respectively.
Symbolic release was1.390/4.265 ms. Actual app queue service remains unproven.
One-shot symbolic+buffer setup was1.605/5.007 ms, excluding the500 symbolic
experiments; buffer and phase timings are separated in the executable.

These are **bounded-call boundary observations**, not interruptibility inside a
factorization. The raw APIs have no cancellation callback; polling is possible
before/after each call. Known workspace is 4,390,424 bytes for the forced-supernodal
chain and 18,777,952 bytes for the grid, excluding symbolic heap, input storage,
allocator overhead and rhs/result storage. Arrays/scratch are allocated before the
numeric loop and reused. RHS is cloned outside solve timing; it is not counted as
zero-copy application handoff. Configuration does not supply large-block rank.

The cancellation probe signals phase entry, writes an atomic latch, and separately
records call duration, worker release and completion of two tiny real SVD operations
labelled frozen-click/other-window **surrogates**. The latch is not an application
cancel acknowledgement. `in_flight=false` marks short-call races; zero release
latency in those rows is not mid-call cancellation. Subsequent surrogate service
bounds are recorded independently from indivisible time.

Partial/reversed-overlap calls exceed a 2 s watchdog at origin and 1e9 translation.
Killing the isolated experiment is not cooperative cancellation or production worker
availability. Correct bounded root-isolation/arrangement continuation APIs have not
been demonstrated; their missing timing cannot be replaced with a fake phase.

**Real Sketch queue service remains Unproven:** the existing M4 worker has a single
newest waiting slot, and no Sketch press/final/interactive/motion service classes.
No real frozen click, concurrent-window edit, sustained motion/final-confirmation
fairness, event acknowledgement or visible-result latency is certified by surrogate
calls. Implementing the production M5.2 scheduler is outside this slice; ignoring
late results would not satisfy the requirement. This evidence gap keeps M5.0 open.

Final full-driver command:102.21 s wall, maximum RSS148,176,896 bytes,
reported peak footprint133,628,480 bytes. Final forced-supernodal-only
command:11.37 s wall, maximum RSS30,343,168 bytes, footprint28,819,888 bytes.
The full-driver elapsed time is500 repetitions plus watchdogs, not a C1
single-solve duration. Its process resources include all scalar/curve fixtures.

The process RSS is characterization, not incremental Sketch memory acceptance.
High-level CSC inputs and reported scratch sizes are explicit lower bounds; symbolic
heap allocations/copies are not fully instrumented. Full document/parameter/formula
capture, retained generations, publication bytes, geometric/formula local edit
counters and 1,000-cycle settled growth remain Unproven. §10 assigns their full-path
implementation checks from M5.1–M5.2, not permission to omit them.

## Exact curves and operation-family matrix — §§5, 7.1, 9.2

API: `brepkit_math::nurbs::bezier_clip::curve_curve_intersect_full`, exact clamped
NURBS input, explicit 1e-7 tolerance. Thirty representative pair observations
(15 unordered families × origin/1e9 translation) retain returned parameter intervals.
Residuals are independently reevaluated on **both original curves**. A good residual
for one root proves neither all roots nor interval/contact classification.

| Unordered families | Observed evidence | Disposition for full operation |
|---|---|---|
| Segment / segment | Known X and T contacts pass; partial/reversed overlaps exceed 2 s. | Proven isolated fixtures; overlap/cancellation path Unsupported. |
| Segment / circle or arc | Endpoint fixture within tolerance; periodic circle seam returns one of two expected geometric hits. | Required periodic path Unsupported; general contacts Unproven. |
| Segment / ellipse or arc | Representative disjoint fixture recorded. | General tangency/all-root/interval completeness Unproven. |
| Segment / rational quadratic | Parabola exact tangency: 0 instead of 1; y=+1e-6: 0 instead of 2; y=-1e-6 correctly disjoint. | Required contacts Unsupported. |
| Segment / Bézier/B-spline/NURBS | Representative crossing recorded. | Multi-span/contact/overlap completeness Unproven. |
| Circle / circle | Identical quarter-arc interval returned. | General partial overlaps, contacts and periodic classification Unproven. |
| Circle / ellipse | Interval returned for distinct arcs with only an endpoint in common. | Returned interval has no certified identity guarantee; required overlap classification unsupported by this fixture. |
| Circle / conic | Returned parameter pair evaluates ≈.666667 mm apart. | Parameter provenance Unsupported. |
| Circle / Bézier/B-spline/NURBS | Representative pair recorded. | General cases Unproven. |
| Ellipse / ellipse | Identical arc interval returned. | General partial/contact cases Unproven. |
| Ellipse / conic | Returned parameter pair evaluates ≈.666667 mm apart. | Parameter provenance Unsupported. |
| Ellipse / Bézier/B-spline/NURBS | Representative pair recorded. | General cases Unproven. |
| Conic / conic | Identical bounded rational arc interval returned. | General branches/partial intervals Unproven. |
| Conic / Bézier/B-spline/NURBS | Origin 2 hits, residual≤3.38e-8; translated 3 hits, residual up to3.77e-7. | Translated tolerance/completeness not certified. |
| Bézier/B-spline/NURBS / same | Identical cubic full interval returned; figure-eight split misses known nonadjacent crossing. | Self-intersection path Unsupported; general multi-span cases Unproven. |

Figure-eight: `x=t²−1, y=t(t²−1), t∈[-2,2]`. Exact cubic split at t=0; independent
crossing t=-1,+1 (original .25,.75). Only the adjacent split endpoint is returned.
These failures occur at both translations. Source clipping has MAX_DEPTH=50,
MAX_NEWTON=10, midpoint/fallback hits and sampled overlap tests; no all-root error,
root-isolation certificate or typed uncertainty contract is exposed. Recursive role
swapping also warrants parameter-provenance validation. No registry patch was made.

| Other family/operation | Actual candidate and evidence | Disposition |
|---|---|---|
| Segment polygon offset | `offset_wire_with_join`, Miter/Intersection, Round/Arc, Bevel/Chamfer; rectangle returns wires. | Simple execution demonstrated; topology/error/cleanup guarantees Unproven. |
| Circle/arc offset | Radius2 + .2 returns 32 lines, 32 lines+32 arcs, or64 lines respectively. Straight-chord midpoint radius error≈.00963055 mm. | Unsupported for exact/certified curved offset at1e-7, despite Ok. |
| Ellipse offset | Same API samples the periodic input via `face_polygon`. | Unsupported as an exact path; no certified error bound. |
| Rational conic offset | Quarter-arc/chord and parabolic/chord cases report fewer-than-three-vertices error under all joins. | Demonstrated failure of this path. |
| Bézier/B-spline/NURBS offset | Cubic/chord rejected under all joins; no general certified multi-span path. | Cubic path Unsupported; broader representation Unproven. |
| Open-chain offsets, hole preservation, curved self-intersection cleanup | API takes a planar face/outer polygon, not the required exact open-chain/region contract. | Unproven; no substitute polygon output accepted. |
| Shape-preserving refinement | Pinned knot insertion/splitting APIs exist; no full independent refinement corpus in this dossier. | Unproven for all advertised editing cases. |
| General length/distance/curvature and advanced constraints | Available evaluation/derivative/numerical-length APIs are not a checked error-bound and solver applicability proof. | Unproven; no silent tool reduction. |
| Projection/Intersect/Insert/references | Existing upstream operation modules alone are not evidence of exact branch/provenance/cancellation coverage. | Unproven; M5.11 scope retained. |
| Arrangement, unions, open branches, X/T cells and point-touch classification | No demonstrated full exact arrangement/continuation implementation. | Unproven; manually ordered wires do not establish it. |

### Tolerances and admitted scales

| Call/predicate | Actual threshold | Required limitation |
|---|---|---|
| Candidate Sketch policy | abs1e-7 mm; relative1e-12*local_extent; auto-coincidence cap1e-4 mm | Unchanged requirements, no silently enlarged tolerance. |
| Clipping | Explicit1e-7 also used as parameter-span epsilon; fallbacks may use10× | Parameter epsilon is not a geometric error certificate; failed cases cannot commit. |
| Wire/circle/NURBS construction | Explicit1e-7 linear | Exact input retained; builder success alone is not face/arrangement certification. |
| `Tolerance::new()` | linear1e-7; angular1e-12; relative1e-10*world operand magnitude | At1e9, representable gap≈1.00136e-5 is treated equal although local policy rejects it. |
| Parameter conversion helper | linear/tangent magnitude, clamped[1e-15,.1] | Must check each derivative/domain rather than reuse one epsilon. |
| Native extrusion/offset | Internal default tolerance, no per-call policy/cancel argument | Local-to-world compatibility remains to certify per operation. |
| Native `solid_volume` | Probe deflection1e-5 | Approximate measurement; errors do not alone prove exact boundary is wrong. |

At1e9, f64 spacing is1.1920928955078125e-7 mm. Required arbitrary1e-7 details are
therefore not universally representable. The tested .1 mm circular hole survives;
this is not a universal admitted scale interval. Local normalization plus dimensional
predicates and checked conversion are required. A compatible general scale domain
is **not established**, so dependent geometry cannot be accepted on these tests.

## Exact profiles through M4 and text

Both normal signs of a10×10 square with exact radius.1 circular hole were extruded
at origin and translated1e9: native topology validates; final volume error≤9.29e-9 mm³.
Three nested levels are represented as two explicitly ordered faces: outer-with-hole
volume128, island8, at both translations. Automatic nesting/region extraction and
open-branch/selected-union semantics remain Unproven.

Exact periodic ellipse, rational quarter arc/chord, parabola/chord and cubic/chord
reach native extrusion at both translations. All tested topology validates. Native
approximate volume errors reach≈5.3e-5 mm³ for ellipse/cubic and≈2.7e-5 for parabola;
the corresponding one-shot `ADVANCED_EXTRUDE us` combines extrusion, measurement and
validation, not isolated extrusion time. The holed-profile500-sample traces separate
setup, extrusion, measure and validation. No display chords replace exact inputs.

Test-only arena fixtures seed native exact geometry and then call the **real M4
`NewellGeometry::extrude_face`** copy/frame/displacement/extrude/validate path:
normal±Z holed profiles at both translations, and a cubic cap. Six box-face normals
also exercise this seam. The known cubic has area7.5 and volume30 at depth4;
the actual M4 arena re-extrusion at depth2 gives15. These are real kernel paths,
not a new Sketch→app selection→M4 command implementation.

O/B/8 from bundled Fira Code are retained as exact quadratic/line edges. Raw f32
outline coordinates are checked to be integer/half-font-unit values and promoted
to f64 **before** scaling. ab_glyph appends synthetic zero-length closing lines even
when a contour is already closed; the test reports and excludes those parser artifacts.
Independent fontTools glyph data confirms2/3/3 contours and areas below. Oriented
outer/inner wires are carried through the same M4 arena, checking1/2/2 holes and two
holed output caps.

| Glyph | Independent font-unit area magnitude | Area at10 mm/em | Depth2 expected volume |
|---|---:|---:|---:|
| O | 563343.5416666667 | 14.815083278544819 | 29.630166557089638 |
| B | 637555 | 16.766732412886252 | 33.533464825772504 |
| 8 | 589791.541666667 | 15.510625684856462 | 31.021251369712924 |

This subset passes exact-profile/volume checks; it does not certify every printable
ASCII glyph, arbitrary layout, transformed text, translated text precision or generic
contour nesting. No different font system/parser was adopted.

## Performance against revision-2 requirements

| Operation (500 samples; ms) | p50 | p95 | p99 | Maximum |
|---|---:|---:|---:|---:|
| S1 scalar local component | 0.036000 | 0.036500 | 0.038584 | 0.047000 |
| S2 scalar local component | 0.118792 | 0.121541 | 0.130500 | 0.137708 |
| S3 scalar local component | 0.118708 | 0.121625 | 0.130416 | 0.142417 |
| Dense rank,256 columns | 9.858333 | 9.946333 | 10.053250 | 10.485000 |
| Default chain numeric | 147.431500 | 151.891250 | 309.181959 | 392.513291 |
| Default grid numeric | 11.853542 | 12.339208 | 12.653208 | 13.964583 |
| Default chain whole backend | 148.977334 | 153.896167 | 313.284250 | 394.789708 |
| Default grid whole backend | 17.841084 | 18.481708 | 18.952125 | 20.643875 |

One-shot whole-scalar-block validation: S1=1.301 ms, S2=17.880 ms,
S3=119.719 ms. These exclude exact regions, queries, application scheduling,
source capture and visible publication. They do not satisfy the full update
or final-validation gates. Setup is0.156/0.928/5.824 ms respectively.

S1/S2/S3 scalar fixtures have respectively3,000/30,000/200,000 equations and unknowns,
24/118/782 bounded blocks (128/256/256 maximum), nonzeros5,976/59,882/399,218. Entity
counts1,000/10,000/100,000 are **logical workload labels**, not instantiated mixed
Sketch entity tables. Intersections/regions/visible geometry are0; C1 is one scalar
block. R-tree fixtures separately store1,000/10,000/100,000 explicit points, one known
candidate per query. No headline count is substituted for geometric coverage.

| Mandatory budget | Final disposition |
|---|---|
| S1/S2/S3 p95 update16/30/50 ms; final250 ms/2 s/15 s | Backend surrogates recorded; full mixed-geometry update, invalidation and final validation Unproven. |
| C1 solve≤5 s, release≤100 ms | Particular linear solve passes time characterization with forced supernodal calls; generic rank and nonlinear C1 remain Unproven. Default chain configuration fails release. No universal timing guarantee. |
| App input p95≤4/p99≤8 ms, certified query p95≤5 ms | Point-index subset only; event handling/certified curve queries Unproven. |
| App cancel ack≤16 ms; actual click/other-window/final fairness | Atomic latch/surrogates only; required actual service evidence Unproven. Overlap backend exceeds2 s. |
| Event-to-visible≤100 ms; frame p95≤16.7 ms,60 FPS | No Sketch renderer/publication integration; Unproven. |
| X1 typed workspace exhaustion/cancellation | Scratch estimates only; no complete bounded admission/typed nonlinear/arrangement outcome. Unproven. |
| S3 incremental memory≤1 GiB; settled growth≤5% after1,000 cycles | Process resources do not certify copies, retained generations or journal growth; Unproven. |
| B S1 input8 ms/result150 ms/frame33.3 ms; S2/S3 characterization | No Pi5 available; Unproven. No low-end exception assumed. |

## Dependent-path disposition and smallest remedies

| Capability | Status / affected slices | Smallest next change or decision |
|---|---|---|
| Bounded dense rank/analytic sparse solve | Proven fixtures only; M5.2/M5.10 foundation candidate | Keep scaling/threshold/branch checks; do not generalize solver success into rank. |
| Generic large numerical rank | Unsupported demonstrated API; M5.2/M5.10/M5.15 | Approve a compatible vetted sparse rank backend or a precise upstream rank API, with MSRV/license/cancel and dense/analytic comparisons. No custom factorization/global dense fallback. |
| Sparse call granularity | Forced-supernodal path improves tested chain/grid; real scheduler Unproven | Prefer the demonstrated public configuration and reusable scratch within D5; retain bounded continuation/service-class tests. No new numerical dependency needed for this local choice. |
| Contacts/overlap/parameter provenance | Concrete failures; M5.5/M5.7/M5.9/M5.11/M5.12 | Certified complementary algorithm in scoped kernel modules or explicit upstream correction with uncertainty, parameter and bounded-work contracts. Current fixed pin is not silently upgraded. |
| Certified curved offsets | Unsupported candidate; M5.7/M5.9/M5.12 | Exact/bounded-error family algorithms, all joins, open/closed chains, holes and self-intersection cleanup; no display-polyline substitution. |
| Scale/tolerance conversion | General domain Unproven; all profile/reference consumers | Certify local/world conversion per operation and reject unresolved detail. Product tolerance changes require a human decision. |
| General arrangement/text/reference integration | Subset proofs only; M5.5/M5.6/M5.11/M5.12 | Full independent topology/curve corpus through exact profiles and actual command integration. No tool removed. |
| Storage/publication/UI service/M4 acceptance | Unproven prerequisites/evidence | Preserve the requirements; no later slice starts under this M5.0 authorization. |

Independent numerical, curve, tolerance, font and extrusion investigations continued
when another path failed. No upgraded kernel, alternate engine, unapproved dependency
or reduced accuracy/tool coverage is used to disguise a gap. The delivered dossier
identifies concrete failed gates and missing evidence; it does not mark M5.0 complete.

## Verification

Initial baseline (before this execution's writes): check passed; formatting failed;
strict Clippy failed on two suspicious-assignment-formatting diagnostics in inherited
`termination.rs`; Nextest1,033 passed,5 failed,20 skipped (three glyph/reference,
two View-menu tests); doc tests passed. INSTA_UPDATE was disabled. Earlier commentary
incorrectly said four failures; the baseline log establishes five.

During resumption an inherited M5.0 test initialized private arena fields; it was
repaired using `NewellGeometry::new()` followed by the existing crate-visible topology
assignment. No visibility was widened. A new translated fixture needed borrowed/cloned
constructor inputs; that ordinary probe compilation issue was fixed. Font contour
analysis initially counted ab_glyph's synthetic zero-length closures as contours;
inspection of its builder and independent fontTools counts corrected the test adapter.

Other ongoing checkout work changed navigation/UI/geometry sources after the initial
baseline. For example an intermediate suite had one Origin-tree failure and an
intermediate Clippy run flagged navigation/app lints. These are distinguished from
M5.0 probe regressions; this slice does not repair unrelated production behavior.
The final current-source checks, below, replace earlier intermediate green/red claims.

| Command | Final observed result |
|---|---|
| `cargo check --workspace --locked` | Passed. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Failed: unrelated `navigation.rs:243` useless conversion and `app.rs:2637` collapsible if. No lint disabled. |
| `cargo clippy -p newell-kernel --all-targets --all-features --locked -- -D warnings` | Passed, including M5.0 integration probes/benchmark. |
| `INSTA_UPDATE=no cargo nextest run --workspace --all-features --locked --no-fail-fast` | 1,061 passed,1 failed,21 skipped; failing app test `origin_reference_viewport_and_tree_routes_share_one_selection_identity` belongs to independent Origin/navigation work. All M5.0 probes passed. |
| `cargo test --workspace --doc --locked` | Passed. |
| `cargo +1.88.0 check --workspace --all-targets --all-features --locked` | Passed on this aarch64 Mac; not cross-architecture execution proof. |
| `cargo test -p newell-kernel --lib m50_ --locked -- --nocapture` | 3 passed: real M4 oriented/translated holes, cubic, exact font contours. |
| `cargo deny check` | Advisories, licenses, bans and sources passed; existing warning inventory retained, no policy weakened. |
| `git diff --check` | Passed. |

The final failing Origin-route assertion selected `[O]` where `[X]` was
expected; it is outside the feasibility files changed here. The earlier five
glyph/menu failures no longer occur after independent checkout updates. The
workspace remains **not green**. The extra ignored helper is the watchdog-child
entry point, explicitly invoked by its parent; skips are not capability passes.
Existing `block0.1.6` future-compatibility notice is informational. No GPU checks
were required by changes to this CPU-only feasibility harness.

No test snapshot was regenerated. No GPU/window/physical-device acceptance is inferred.
The report stops at M5.0; no Auditor-green assertion or automatic next role/slice.

## Final reproducible traces (Rust1.99)

The following are direct outputs from the final executable and focused M4 test.
Timing is characterization, not reference-A/B acceptance.

### Full driver,500 samples

```text
M5.0,seed=0,iterations=500,parallelism=Seq,units=mm
SCALE_FIXTURE,S1,entities=1000,unknowns=3000,equations=3000,largest_block=128,nnz=5976,blocks=24,setup_us=155.917,scope=linear_backend_surrogate
SCALE_FINAL,S1,all_blocks_us=1300.625,max_solution_error=3.774758283725532e-14,integrated_final_validation=Unproven
SCALE_COUNTER,S1,explicit_components_evaluated_per_local_edit=1,other_components_evaluated=0,full_path_copy_allocations=Unmeasured,old_published_generation=NotIntegrated
LATENCY,S1_linear_component_edit,n=500,p50_us=36.000,p95_us=36.500,p99_us=38.584,max_us=47.000
SCALE_FIXTURE,S2,entities=10000,unknowns=30000,equations=30000,largest_block=256,nnz=59882,blocks=118,setup_us=927.667,scope=linear_backend_surrogate
SCALE_FINAL,S2,all_blocks_us=17880.458,max_solution_error=1.6919798895287386e-13,integrated_final_validation=Unproven
SCALE_COUNTER,S2,explicit_components_evaluated_per_local_edit=1,other_components_evaluated=0,full_path_copy_allocations=Unmeasured,old_published_generation=NotIntegrated
LATENCY,S2_linear_component_edit,n=500,p50_us=118.792,p95_us=121.541,p99_us=130.500,max_us=137.708
SCALE_FIXTURE,S3,entities=100000,unknowns=200000,equations=200000,largest_block=256,nnz=399218,blocks=782,setup_us=5824.250,scope=linear_backend_surrogate
SCALE_FINAL,S3,all_blocks_us=119719.375,max_solution_error=1.6919798895287386e-13,integrated_final_validation=Unproven
SCALE_COUNTER,S3,explicit_components_evaluated_per_local_edit=1,other_components_evaluated=0,full_path_copy_allocations=Unmeasured,old_published_generation=NotIntegrated
LATENCY,S3_linear_component_edit,n=500,p50_us=118.708,p95_us=121.625,p99_us=130.416,max_us=142.417
RANK,rigid_segment,rows=1,cols=4,rank=1,linearized_nullity=3,mobility=Regular_analytic_fixture
LATENCY,dense_svd_rigid_segment,n=500,p50_us=0.375,p95_us=0.417,p99_us=0.500,max_us=5.791
RANK,duplicated_equations,rows=2,cols=2,rank=1,linearized_nullity=1,mobility=Regular_analytic_fixture
LATENCY,dense_svd_duplicated_equations,n=500,p50_us=0.542,p95_us=0.583,p99_us=0.584,max_us=0.959
RANK,toggle,rows=2,cols=2,rank=1,linearized_nullity=1,mobility=Singular_NotCertified
LATENCY,dense_svd_toggle,n=500,p50_us=0.416,p95_us=0.417,p99_us=0.458,max_us=0.542
RANK,tangent_fixed_circles,rows=2,cols=2,rank=1,linearized_nullity=1,mobility=Singular_NotCertified
LATENCY,dense_svd_tangent_fixed_circles,n=500,p50_us=0.458,p95_us=0.459,p99_us=0.500,max_us=0.583
RANK,active_parameter_bound,rows=1,cols=2,rank=1,linearized_nullity=1,mobility=Boundary_NotCertified
LATENCY,dense_svd_active_parameter_bound,n=500,p50_us=0.334,p95_us=0.375,p99_us=0.375,max_us=0.417
RANK,regular_grounded,rows=2,cols=2,rank=2,linearized_nullity=0,mobility=Regular_analytic_fixture
LATENCY,dense_svd_regular_grounded,n=500,p50_us=0.416,p95_us=0.417,p99_us=0.500,max_us=0.583
LATENCY,dense_svd_256,n=500,p50_us=9858.333,p95_us=9946.333,p99_us=10053.250,max_us=10485.000
THRESHOLD,scale=1e-9,singular_values=[9.999999999999999e-10, 1.0000000000000001e-19],ranks=[1, 1, 0],classification=Uncertain
THRESHOLD,scale=1e0,singular_values=[1.0, 1e-10],ranks=[1, 1, 1],classification=Uncertain
THRESHOLD,scale=1e9,singular_values=[1000000000.0, 0.10000000000000002],ranks=[2, 1, 1],classification=Uncertain
GCS,bounded_oracle,max_residual=0e0,unknowns=2,equations=2,branch=positive_x
LATENCY,gcs_bounded_nonlinear_solve,n=500,p50_us=6.375,p95_us=6.833,p99_us=9.833,max_us=60.041
FIXTURE,S1_component,unknowns=128,equations=128,nnz=255,setup_us=8.834,input_csc_lower_bound_bytes=6136
WORKSPACE,S1_component,numeric_values_bytes=5104,numeric_indices_bytes=6144,factor_scratch_bytes=4096,solve_scratch_bytes=1024,symbolic_heap_bytes=Unmeasured
SOLVE,S1_component,analytic_solution_max_error=3.774758283725532e-14,generic_large_numerical_rank=Unavailable
LATENCY,S1_component_symbolic,n=500,p50_us=16.500,p95_us=16.958,p99_us=20.375,max_us=36.542
LATENCY,S1_component_factor,n=500,p50_us=33.000,p95_us=33.500,p99_us=40.250,max_us=67.708
LATENCY,S1_component_solve,n=500,p50_us=2.958,p95_us=3.084,p99_us=3.250,max_us=36.334
LATENCY,S1_component_cleanup,n=500,p50_us=0.250,p95_us=0.333,p99_us=0.375,max_us=1.625
LATENCY,S1_component_backend_total,n=500,p50_us=53.000,p95_us=54.625,p99_us=61.917,max_us=122.708
FIXTURE,S2_component,unknowns=256,equations=256,nnz=511,setup_us=19.333,input_csc_lower_bound_bytes=12280
WORKSPACE,S2_component,numeric_values_bytes=10224,numeric_indices_bytes=12288,factor_scratch_bytes=8192,solve_scratch_bytes=2048,symbolic_heap_bytes=Unmeasured
SOLVE,S2_component,analytic_solution_max_error=1.6919798895287386e-13,generic_large_numerical_rank=Unavailable
LATENCY,S2_component_symbolic,n=500,p50_us=32.833,p95_us=33.958,p99_us=42.375,max_us=88.167
LATENCY,S2_component_factor,n=500,p50_us=113.125,p95_us=115.625,p99_us=136.958,max_us=157.125
LATENCY,S2_component_solve,n=500,p50_us=5.625,p95_us=5.958,p99_us=7.000,max_us=31.500
LATENCY,S2_component_cleanup,n=500,p50_us=0.167,p95_us=0.209,p99_us=0.459,max_us=1.292
LATENCY,S2_component_backend_total,n=500,p50_us=152.125,p95_us=158.250,p99_us=185.500,max_us=210.083
FIXTURE,C1_chain,unknowns=10000,equations=10000,nnz=19999,setup_us=250.583,input_csc_lower_bound_bytes=479992
WORKSPACE,C1_chain,numeric_values_bytes=399984,numeric_indices_bytes=480000,factor_scratch_bytes=320000,solve_scratch_bytes=80000,symbolic_heap_bytes=Unmeasured
SOLVE,C1_chain,analytic_solution_max_error=1.2067999932696694e-9,generic_large_numerical_rank=Unavailable
LATENCY,C1_chain_symbolic,n=500,p50_us=1287.875,p95_us=1386.291,p99_us=2027.625,max_us=4106.417
LATENCY,C1_chain_factor,n=500,p50_us=147431.500,p95_us=151891.250,p99_us=309181.959,max_us=392513.291
LATENCY,C1_chain_solve,n=500,p50_us=226.667,p95_us=262.125,p99_us=305.625,max_us=334.666
LATENCY,C1_chain_cleanup,n=500,p50_us=2.083,p95_us=3.208,p99_us=3.875,max_us=7.542
LATENCY,C1_chain_backend_total,n=500,p50_us=148977.334,p95_us=153896.167,p99_us=313284.250,max_us=394789.708
FIXTURE,C1_grid,unknowns=10000,equations=19801,nnz=39601,setup_us=1634.875,input_csc_lower_bound_bytes=872032
WORKSPACE,C1_grid,numeric_values_bytes=15136296,numeric_indices_bytes=1730744,factor_scratch_bytes=1752192,solve_scratch_bytes=158720,symbolic_heap_bytes=Unmeasured
SOLVE,C1_grid,analytic_solution_max_error=2.5579538487363607e-13,generic_large_numerical_rank=Unavailable
LATENCY,C1_grid_symbolic,n=500,p50_us=4440.167,p95_us=4553.167,p99_us=4637.084,max_us=5078.000
LATENCY,C1_grid_factor,n=500,p50_us=11853.542,p95_us=12339.208,p99_us=12653.208,max_us=13964.583
LATENCY,C1_grid_solve,n=500,p50_us=1556.083,p95_us=1639.750,p99_us=1718.375,max_us=1798.875
LATENCY,C1_grid_cleanup,n=500,p50_us=3.125,p95_us=4.000,p99_us=4.500,max_us=12.459
LATENCY,C1_grid_backend_total,n=500,p50_us=17841.084,p95_us=18481.708,p99_us=18952.125,max_us=20643.875
CANCEL,sparse_numeric_C1_chain,atomic_latch_us=0.042,indivisible_us=147666.458,worker_release_us=147656.750,frozen_click_surrogate_us=147664.875,other_window_surrogate_us=147666.209,in_flight=true
CANCEL,sparse_symbolic_C1_grid,atomic_latch_us=0.042,indivisible_us=4553.041,worker_release_us=4544.375,frozen_click_surrogate_us=4549.917,other_window_surrogate_us=4551.375,in_flight=true
CANCEL,sparse_numeric_C1_grid,atomic_latch_us=0.000,indivisible_us=12608.625,worker_release_us=12600.791,frozen_click_surrogate_us=12605.250,other_window_surrogate_us=12606.291,in_flight=true
CANCEL,sparse_solve_C1_grid,atomic_latch_us=0.042,indivisible_us=1647.000,worker_release_us=1639.542,frozen_click_surrogate_us=1645.708,other_window_surrogate_us=1647.000,in_flight=true
CANCEL,dense_rank_256,atomic_latch_us=0.042,indivisible_us=9975.083,worker_release_us=9967.417,frozen_click_surrogate_us=9969.208,other_window_surrogate_us=9970.042,in_flight=true
INDEX,count=1000,setup_us=152.333,input_payload_bytes=16000
INDEX_EDIT,count=1000,remove_insert_us=4.167,explicit_entries_changed=1,internal_visits_and_allocations=Unmeasured
LATENCY,rstar_query_1000,n=500,p50_us=0.333,p95_us=0.500,p99_us=0.709,max_us=0.875
INDEX,count=10000,setup_us=1056.000,input_payload_bytes=160000
INDEX_EDIT,count=10000,remove_insert_us=2.500,explicit_entries_changed=1,internal_visits_and_allocations=Unmeasured
LATENCY,rstar_query_10000,n=500,p50_us=0.542,p95_us=0.792,p99_us=0.958,max_us=1.667
INDEX,count=100000,setup_us=12265.166,input_payload_bytes=1600000
INDEX_EDIT,count=100000,remove_insert_us=3.792,explicit_entries_changed=1,internal_visits_and_allocations=Unmeasured
LATENCY,rstar_query_100000,n=500,p50_us=1.208,p95_us=1.834,p99_us=2.166,max_us=3.583
PAIR,segment,segment,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=56.833,completeness=Unproven
PAIR,segment,circle_arc,shift=0e0,hits=1,overlaps=0,max_residual=4.214684887878958e-8,us=65.375,completeness=Unproven
PAIR,segment,ellipse_arc,shift=0e0,hits=0,overlaps=0,max_residual=0e0,us=4.250,completeness=Unproven
PAIR,segment,rational_conic,shift=0e0,hits=0,overlaps=0,max_residual=0e0,us=4.583,completeness=Unproven
PAIR,segment,bezier_bspline,shift=0e0,hits=1,overlaps=0,max_residual=0e0,us=671.209,completeness=Unproven
PAIR,circle_arc,circle_arc,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=4742.583,completeness=Unproven
PAIR,circle_arc,ellipse_arc,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=360.167,completeness=Unproven
PAIR,circle_arc,rational_conic,shift=0e0,hits=1,overlaps=0,max_residual=6.666666666666667e-1,us=54.209,completeness=Unproven
PAIR,circle_arc,bezier_bspline,shift=0e0,hits=0,overlaps=0,max_residual=0e0,us=47.041,completeness=Unproven
PAIR,ellipse_arc,ellipse_arc,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=4806.125,completeness=Unproven
PAIR,ellipse_arc,rational_conic,shift=0e0,hits=1,overlaps=0,max_residual=6.666666666666667e-1,us=55.916,completeness=Unproven
PAIR,ellipse_arc,bezier_bspline,shift=0e0,hits=0,overlaps=0,max_residual=0e0,us=40.125,completeness=Unproven
PAIR,rational_conic,rational_conic,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=4906.667,completeness=Unproven
PAIR,rational_conic,bezier_bspline,shift=0e0,hits=2,overlaps=0,max_residual=3.3796315915761384e-8,us=445.417,completeness=Unproven
PAIR,bezier_bspline,bezier_bspline,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=6705.459,completeness=Unproven
ANALYTIC,cross,shift=0e0,hits=1,overlaps=0,expected_hits=1,expected_overlap=0,matches=true
OPAQUE_START,partial_overlap,shift=0e0

WATCHDOG,partial_overlap,shift=0e0,wall_us=2001685.334,status=Unsupported_uninterruptible_timeout,process_kill_is_not_cancellation=true
OPAQUE_START,reversed_overlap,shift=0e0

WATCHDOG,reversed_overlap,shift=0e0,wall_us=2001633.459,status=Unsupported_uninterruptible_timeout,process_kill_is_not_cancellation=true
ANALYTIC,T_contact,shift=0e0,hits=1,overlaps=0,expected_hits=1,expected_overlap=0,matches=true
ANALYTIC,parabola_contact,shift=0e0,delta=0e0,hits=0,expected=1,matches=false,us=555.000
ANALYTIC,parabola_contact,shift=0e0,delta=1e-6,hits=0,expected=2,matches=false,us=549.709
ANALYTIC,parabola_contact,shift=0e0,delta=-1e-6,hits=0,expected=0,matches=true,us=2.250
LATENCY,intersection_tangent_shift_0e0,n=500,p50_us=536.166,p95_us=554.083,p99_us=562.666,max_us=699.583
CANCEL,intersection_tangent_shift_0e0,atomic_latch_us=0.042,indivisible_us=556.125,worker_release_us=547.042,frozen_click_surrogate_us=558.083,other_window_surrogate_us=559.458,in_flight=true
OPAQUE_START,periodic_seam,shift=0e0
SPECIAL,periodic_seam,shift=0e0,hits=1,overlaps=0,expected_distinct_geometric_points=2,us=281.625

OPAQUE_START,figure_eight,shift=0e0
SPECIAL,figure_eight,shift=0e0,hits=1,overlaps=0,us=173.292,expected_nonadjacent_parameters=0.25_0.75
SPECIAL_PARAMETERS,0.49999997519261874,0.5,residual=9.922952504126618e-8

TOLERANCE,shift=0e0,linear=0.0000001,angular=0.000000000001,relative=0.0000000001,tiny_gap_stored=1e-5,kernel_approx_equal=false,local_policy_equal=false,ulp=5e-324
PAIR,segment,segment,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=5.292,completeness=Unproven
PAIR,segment,circle_arc,shift=1e9,hits=1,overlaps=0,max_residual=0e0,us=69.625,completeness=Unproven
PAIR,segment,ellipse_arc,shift=1e9,hits=0,overlaps=0,max_residual=0e0,us=3.958,completeness=Unproven
PAIR,segment,rational_conic,shift=1e9,hits=0,overlaps=0,max_residual=0e0,us=1.542,completeness=Unproven
PAIR,segment,bezier_bspline,shift=1e9,hits=1,overlaps=0,max_residual=0e0,us=719.750,completeness=Unproven
PAIR,circle_arc,circle_arc,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=5030.250,completeness=Unproven
PAIR,circle_arc,ellipse_arc,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=382.542,completeness=Unproven
PAIR,circle_arc,rational_conic,shift=1e9,hits=1,overlaps=0,max_residual=6.666666269302368e-1,us=59.416,completeness=Unproven
PAIR,circle_arc,bezier_bspline,shift=1e9,hits=0,overlaps=0,max_residual=0e0,us=53.625,completeness=Unproven
PAIR,ellipse_arc,ellipse_arc,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=4942.875,completeness=Unproven
PAIR,ellipse_arc,rational_conic,shift=1e9,hits=1,overlaps=0,max_residual=6.666666269302368e-1,us=59.583,completeness=Unproven
PAIR,ellipse_arc,bezier_bspline,shift=1e9,hits=0,overlaps=0,max_residual=0e0,us=43.375,completeness=Unproven
PAIR,rational_conic,rational_conic,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=4868.334,completeness=Unproven
PAIR,rational_conic,bezier_bspline,shift=1e9,hits=3,overlaps=0,max_residual=3.769728732309794e-7,us=485.500,completeness=Unproven
PAIR,bezier_bspline,bezier_bspline,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=6696.625,completeness=Unproven
ANALYTIC,cross,shift=1e9,hits=1,overlaps=0,expected_hits=1,expected_overlap=0,matches=true
OPAQUE_START,partial_overlap,shift=1e9

WATCHDOG,partial_overlap,shift=1e9,wall_us=2002094.250,status=Unsupported_uninterruptible_timeout,process_kill_is_not_cancellation=true
OPAQUE_START,reversed_overlap,shift=1e9

WATCHDOG,reversed_overlap,shift=1e9,wall_us=2001501.709,status=Unsupported_uninterruptible_timeout,process_kill_is_not_cancellation=true
ANALYTIC,T_contact,shift=1e9,hits=1,overlaps=0,expected_hits=1,expected_overlap=0,matches=true
ANALYTIC,parabola_contact,shift=1e9,delta=0e0,hits=0,expected=1,matches=false,us=550.875
ANALYTIC,parabola_contact,shift=1e9,delta=1e-6,hits=0,expected=2,matches=false,us=806.583
ANALYTIC,parabola_contact,shift=1e9,delta=-1e-6,hits=0,expected=0,matches=true,us=1.625
LATENCY,intersection_tangent_shift_1e9,n=500,p50_us=544.041,p95_us=715.250,p99_us=875.209,max_us=1378.375
CANCEL,intersection_tangent_shift_1e9,atomic_latch_us=0.042,indivisible_us=636.125,worker_release_us=629.542,frozen_click_surrogate_us=636.584,other_window_surrogate_us=637.959,in_flight=true
OPAQUE_START,periodic_seam,shift=1e9
SPECIAL,periodic_seam,shift=1e9,hits=1,overlaps=0,expected_distinct_geometric_points=2,us=321.625

OPAQUE_START,figure_eight,shift=1e9
SPECIAL,figure_eight,shift=1e9,hits=1,overlaps=0,us=200.042,expected_nonadjacent_parameters=0.25_0.75
SPECIAL_PARAMETERS,0.4999999574378508,0.5,residual=1.1920928955078125e-7

TOLERANCE,shift=1e9,linear=0.0000001,angular=0.000000000001,relative=0.0000000001,tiny_gap_stored=1.0013580322265625e-5,kernel_approx_equal=true,local_policy_equal=false,ulp=1.1920928955078125e-7
FONT,O,outline_segments=22,bounds=Rect { min: point(93.0, 1399.0), max: point(1107.0, -23.0) },font_units_per_em=Some(1950.0),contour_orientation_and_precision=Unproven
FONT,B,outline_segments=36,bounds=Rect { min: point(193.0, 1377.0), max: point(1079.0, 0.0) },font_units_per_em=Some(1950.0),contour_orientation_and_precision=Unproven
FONT,8,outline_segments=42,bounds=Rect { min: point(131.0, 1399.0), max: point(1069.0, -23.0) },font_units_per_em=Some(1950.0),contour_orientation_and_precision=Unproven
OFFSET_ADVANCED,ellipse_periodic,shift=0e0,join=Intersection,result=Ok(Id(1)),certified_error_bound=Unavailable
OFFSET_ADVANCED,ellipse_periodic,shift=0e0,join=Arc,result=Ok(Id(2)),certified_error_bound=Unavailable
OFFSET_ADVANCED,ellipse_periodic,shift=0e0,join=Chamfer,result=Ok(Id(3)),certified_error_bound=Unavailable
ADVANCED_EXTRUDE,ellipse_periodic,shift=0e0,expected_volume=12.566370614359172,native_volume=12.566317699613245,error=5.291474592716838e-5,valid=true,us=893684.750
OFFSET_ADVANCED,ellipse_periodic,shift=1e9,join=Intersection,result=Ok(Id(1)),certified_error_bound=Unavailable
OFFSET_ADVANCED,ellipse_periodic,shift=1e9,join=Arc,result=Ok(Id(2)),certified_error_bound=Unavailable
OFFSET_ADVANCED,ellipse_periodic,shift=1e9,join=Chamfer,result=Ok(Id(3)),certified_error_bound=Unavailable
ADVANCED_EXTRUDE,ellipse_periodic,shift=1e9,expected_volume=12.566370614359172,native_volume=12.566317716283601,error=5.289807557140591e-5,valid=true,us=765227.583
OFFSET_ADVANCED,rational_quadratic_arc,shift=0e0,join=Intersection,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,rational_quadratic_arc,shift=0e0,join=Arc,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,rational_quadratic_arc,shift=0e0,join=Chamfer,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
ADVANCED_EXTRUDE,rational_quadratic_arc,shift=0e0,expected_volume=0.5707963267948966,native_volume=0.5707963267948961,error=4.440892098500626e-16,valid=true,us=84.375
OFFSET_ADVANCED,rational_quadratic_arc,shift=1e9,join=Intersection,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,rational_quadratic_arc,shift=1e9,join=Arc,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,rational_quadratic_arc,shift=1e9,join=Chamfer,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
ADVANCED_EXTRUDE,rational_quadratic_arc,shift=1e9,expected_volume=0.5707963267948966,native_volume=0.5707961091414785,error=2.1765341806645466e-7,valid=true,us=24.208
OFFSET_ADVANCED,parabolic_conic,shift=0e0,join=Intersection,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,parabolic_conic,shift=0e0,join=Arc,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,parabolic_conic,shift=0e0,join=Chamfer,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
ADVANCED_EXTRUDE,parabolic_conic,shift=0e0,expected_volume=2.6666666666666665,native_volume=2.666639539930557,error=2.7126736109384098e-5,valid=true,us=191826.417
OFFSET_ADVANCED,parabolic_conic,shift=1e9,join=Intersection,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,parabolic_conic,shift=1e9,join=Arc,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,parabolic_conic,shift=1e9,join=Chamfer,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
ADVANCED_EXTRUDE,parabolic_conic,shift=1e9,expected_volume=2.6666666666666665,native_volume=2.6666395376167196,error=2.712904994695009e-5,valid=true,us=192032.042
OFFSET_ADVANCED,cubic_bezier,shift=0e0,join=Intersection,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,cubic_bezier,shift=0e0,join=Arc,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,cubic_bezier,shift=0e0,join=Chamfer,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
ADVANCED_EXTRUDE,cubic_bezier,shift=0e0,expected_volume=15,native_volume=14.999947397214164,error=5.2602785835631494e-5,valid=true,us=27821.334
OFFSET_ADVANCED,cubic_bezier,shift=1e9,join=Intersection,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,cubic_bezier,shift=1e9,join=Arc,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,cubic_bezier,shift=1e9,join=Chamfer,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
ADVANCED_EXTRUDE,cubic_bezier,shift=1e9,expected_volume=15,native_volume=14.99994886955148,error=5.113044852045334e-5,valid=true,us=27291.000
NESTING,manual_exact_faces,shift=0e0,outer_expected=128,outer_volume=128,island_expected=8,island_volume=8,automatic_arrangement=Unproven
NESTING,manual_exact_faces,shift=1e9,outer_expected=128,outer_volume=128,island_expected=8,island_volume=8,automatic_arrangement=Unproven
EXTRUDE,exact_circular_hole,shift=0e0,flipped=false,volume_abs_error=1.9895196601282805e-13,invalid=0,tolerance_mm=1e-7
LATENCY,native_holed_extrude_0e0_false,n=500,p50_us=2.541,p95_us=2.709,p99_us=3.000,max_us=5.083
LATENCY,wire_face_setup_0e0_false,n=500,p50_us=0.583,p95_us=0.667,p99_us=0.750,max_us=1.375
LATENCY,native_volume_0e0_false,n=500,p50_us=89.958,p95_us=91.625,p99_us=103.792,max_us=154.708
LATENCY,native_validate_0e0_false,n=500,p50_us=25.875,p95_us=26.583,p99_us=28.708,max_us=41.042
EXTRUDE,exact_circular_hole,shift=0e0,flipped=true,volume_abs_error=1.9895196601282805e-13,invalid=0,tolerance_mm=1e-7
LATENCY,native_holed_extrude_0e0_true,n=500,p50_us=2.542,p95_us=2.709,p99_us=2.917,max_us=5.542
LATENCY,wire_face_setup_0e0_true,n=500,p50_us=0.583,p95_us=0.667,p99_us=0.750,max_us=1.333
LATENCY,native_volume_0e0_true,n=500,p50_us=89.209,p95_us=91.542,p99_us=99.709,max_us=155.208
LATENCY,native_validate_0e0_true,n=500,p50_us=25.792,p95_us=26.417,p99_us=28.084,max_us=70.458
CANCEL,dependent_extrude_0e0,atomic_latch_us=0.041,indivisible_us=6.167,worker_release_us=0.000,frozen_click_surrogate_us=5.083,other_window_surrogate_us=6.208,in_flight=false
EXTRUDE,exact_circular_hole,shift=1e9,flipped=false,volume_abs_error=9.288100955018308e-9,invalid=0,tolerance_mm=1e-7
LATENCY,native_holed_extrude_1e9_false,n=500,p50_us=2.542,p95_us=2.709,p99_us=3.208,max_us=6.500
LATENCY,wire_face_setup_1e9_false,n=500,p50_us=0.584,p95_us=0.667,p99_us=0.833,max_us=1.625
LATENCY,native_volume_1e9_false,n=500,p50_us=89.042,p95_us=91.208,p99_us=96.042,max_us=127.000
LATENCY,native_validate_1e9_false,n=500,p50_us=25.708,p95_us=26.333,p99_us=28.667,max_us=41.000
EXTRUDE,exact_circular_hole,shift=1e9,flipped=true,volume_abs_error=9.288100955018308e-9,invalid=0,tolerance_mm=1e-7
LATENCY,native_holed_extrude_1e9_true,n=500,p50_us=2.542,p95_us=2.750,p99_us=3.500,max_us=9.750
LATENCY,wire_face_setup_1e9_true,n=500,p50_us=0.583,p95_us=0.667,p99_us=0.917,max_us=3.125
LATENCY,native_volume_1e9_true,n=500,p50_us=88.584,p95_us=91.125,p99_us=117.083,max_us=167.292
LATENCY,native_validate_1e9_true,n=500,p50_us=25.667,p95_us=26.625,p99_us=29.416,max_us=61.958
CANCEL,dependent_extrude_1e9,atomic_latch_us=0.084,indivisible_us=8.375,worker_release_us=0.000,frozen_click_surrogate_us=6.542,other_window_surrogate_us=7.625,in_flight=false
CANCEL,wire_and_face_construction,atomic_latch_us=0.042,indivisible_us=22.375,worker_release_us=14.209,frozen_click_surrogate_us=17.042,other_window_surrogate_us=17.917,in_flight=true
OFFSET,holed_polygon,join=Intersection,result=Ok(Id(2)),hole_and_self_intersection_certification=Unproven
OFFSET,exact_periodic_circle,join=Intersection,result=Ok(Id(1)),analytic_expected_radius=2.2
OFFSET_ACCURACY,circle,join=Intersection,lines=32,circles=0,endpoint_radius_error=9.677144752622269e-4,chord_midpoint_radius_error=9.63054665560703e-3,certified_bound=Unavailable
OFFSET,holed_polygon,join=Arc,result=Ok(Id(2)),hole_and_self_intersection_certification=Unproven
OFFSET,exact_periodic_circle,join=Arc,result=Ok(Id(1)),analytic_expected_radius=2.2
OFFSET_ACCURACY,circle,join=Arc,lines=32,circles=32,endpoint_radius_error=8.756785170707104e-4,chord_midpoint_radius_error=9.630546655606587e-3,certified_bound=Unavailable
OFFSET,holed_polygon,join=Chamfer,result=Ok(Id(2)),hole_and_self_intersection_certification=Unproven
OFFSET,exact_periodic_circle,join=Chamfer,result=Ok(Id(1)),analytic_expected_radius=2.2
OFFSET_ACCURACY,circle,join=Chamfer,lines=64,circles=0,endpoint_radius_error=8.756785170707104e-4,chord_midpoint_radius_error=9.630546655606587e-3,certified_bound=Unavailable
M4_PROFILE,cubic,volume=29.999999999999915,expected=30,us=2273.833,full_app_path=Unproven
M4_ARENA,face=0,result=Ok(Ok(11.999999999999998))
M4_ARENA,face=1,result=Ok(Ok(11.999999999999998))
M4_ARENA,face=2,result=Ok(Ok(16.000000000000018))
M4_ARENA,face=3,result=Ok(Ok(16.000000000000018))
M4_ARENA,face=4,result=Ok(Ok(23.999999999999996))
M4_ARENA,face=5,result=Ok(Ok(23.999999999999996))
      102.21 real       100.47 user         0.39 sys
           148176896  maximum resident set size
                   0  average shared memory size
                   0  average unshared data size
                   0  average unshared stack size
               24529  page reclaims
                   0  page faults
                   0  swaps
                   0  block input operations
                   0  block output operations
                   0  messages sent
                   0  messages received
                   0  signals received
                  12  voluntary context switches
               23633  involuntary context switches
       1190678399342  instructions retired
        193840566693  cycles elapsed
           133628480  peak memory footprint
```

### Forced-supernodal configuration,500 samples

```text
M5.0,seed=0,iterations=500,parallelism=Seq,units=mm
LATENCY,configured_supernodal_grid_false_symbolic,n=500,p50_us=1353.292,p95_us=1379.917,p99_us=1393.333,max_us=1426.166
CANCEL,configured_supernodal_grid_false_symbolic,atomic_latch_us=0.042,indivisible_us=1402.542,worker_release_us=1389.833,frozen_click_surrogate_us=1456.750,other_window_surrogate_us=1458.292,in_flight=true
QR_CONFIG,supernodal,grid=false,setup_us=1605.249,reused_workspace_bytes=4390424,rank_certification=Unavailable
LATENCY,configured_supernodal_grid_false_numeric,n=500,p50_us=1434.667,p95_us=1459.667,p99_us=1476.625,max_us=1510.208
LATENCY,configured_supernodal_grid_false_solve,n=500,p50_us=391.209,p95_us=397.041,p99_us=402.958,max_us=423.625
CANCEL,configured_supernodal_grid_false_numeric,atomic_latch_us=0.042,indivisible_us=1457.375,worker_release_us=1448.000,frozen_click_surrogate_us=1459.667,other_window_surrogate_us=1461.042,in_flight=true
LATENCY,configured_supernodal_grid_true_symbolic,n=500,p50_us=3806.875,p95_us=3922.958,p99_us=4197.209,max_us=4454.583
CANCEL,configured_supernodal_grid_true_symbolic,atomic_latch_us=0.042,indivisible_us=4275.333,worker_release_us=4264.542,frozen_click_surrogate_us=4276.917,other_window_surrogate_us=4277.917,in_flight=true
QR_CONFIG,supernodal,grid=true,setup_us=5006.959,reused_workspace_bytes=18777952,rank_certification=Unavailable
LATENCY,configured_supernodal_grid_true_numeric,n=500,p50_us=11510.042,p95_us=18975.292,p99_us=24537.375,max_us=32918.584
LATENCY,configured_supernodal_grid_true_solve,n=500,p50_us=1524.042,p95_us=4172.416,p99_us=4767.333,max_us=22121.750
CANCEL,configured_supernodal_grid_true_numeric,atomic_latch_us=0.042,indivisible_us=13327.542,worker_release_us=13316.250,frozen_click_surrogate_us=13325.667,other_window_surrogate_us=13326.833,in_flight=true
       11.37 real        11.24 user         0.02 sys
            30343168  maximum resident set size
                   0  average shared memory size
                   0  average unshared data size
                   0  average unshared stack size
                2038  page reclaims
                   0  page faults
                   0  swaps
                   0  block input operations
                   0  block output operations
                   0  messages sent
                   0  messages received
                   0  signals received
                   0  voluntary context switches
                1105  involuntary context switches
        116779485169  instructions retired
         22299868963  cycles elapsed
            28819888  peak memory footprint
```

### Rejected/alternative configuration exploration,5 samples

```text
M5.0,seed=0,iterations=500,parallelism=Seq,units=mm
QR_CONFIG,simplicial,grid=false,setup_us=2178.416,reused_workspace_bytes=1279984,rank_certification=Unavailable
LATENCY,configured_simplicial_grid_false_numeric,n=5,p50_us=147822.875,p95_us=149537.375,p99_us=149537.375,max_us=149537.375
LATENCY,configured_simplicial_grid_false_solve,n=5,p50_us=206.959,p95_us=219.834,p99_us=219.834,max_us=219.834
CANCEL,configured_simplicial_grid_false_numeric,atomic_latch_us=0.041,indivisible_us=148931.208,worker_release_us=148922.291,frozen_click_surrogate_us=148963.208,other_window_surrogate_us=148964.916,in_flight=true
QR_CONFIG,simplicial,grid=true,setup_us=4026.791,reused_workspace_bytes=53809296,rank_certification=Unavailable
LATENCY,configured_simplicial_grid_true_numeric,n=5,p50_us=945648.459,p95_us=1174807.125,p99_us=1174807.125,max_us=1174807.125
LATENCY,configured_simplicial_grid_true_solve,n=5,p50_us=6832.750,p95_us=7983.875,p99_us=7983.875,max_us=7983.875
CANCEL,configured_simplicial_grid_true_numeric,atomic_latch_us=0.083,indivisible_us=956724.875,worker_release_us=956716.042,frozen_click_surrogate_us=956736.000,other_window_surrogate_us=956737.917,in_flight=true
QR_CONFIG,supernodal,grid=false,setup_us=1510.792,reused_workspace_bytes=4390424,rank_certification=Unavailable
LATENCY,configured_supernodal_grid_false_numeric,n=5,p50_us=1442.833,p95_us=1447.541,p99_us=1447.541,max_us=1447.541
LATENCY,configured_supernodal_grid_false_solve,n=5,p50_us=406.833,p95_us=415.083,p99_us=415.083,max_us=415.083
CANCEL,configured_supernodal_grid_false_numeric,atomic_latch_us=0.042,indivisible_us=1452.875,worker_release_us=1443.792,frozen_click_surrogate_us=1449.917,other_window_surrogate_us=1450.834,in_flight=true
QR_CONFIG,supernodal,grid=true,setup_us=5688.416,reused_workspace_bytes=18777952,rank_certification=Unavailable
LATENCY,configured_supernodal_grid_true_numeric,n=5,p50_us=11806.791,p95_us=12157.000,p99_us=12157.000,max_us=12157.000
LATENCY,configured_supernodal_grid_true_solve,n=5,p50_us=1571.041,p95_us=1620.000,p99_us=1620.000,max_us=1620.000
CANCEL,configured_supernodal_grid_true_numeric,atomic_latch_us=0.042,indivisible_us=11743.542,worker_release_us=11733.417,frozen_click_surrogate_us=11740.334,other_window_surrogate_us=11741.375,in_flight=true
```

### Actual M4 arena and independent font-area checks

```text
Compiling newell-kernel v0.1.0 (/Users/martin/Desktop/Newell/crates/newell-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.28s
     Running unittests src/lib.rs (target/debug/deps/newell_kernel-c4db60f6278a531e)

running 3 tests
FONT_OUTLINE,O,contours=2,synthetic_zero_length_closures=2
M4_HOLE,shift=0e0,normal=Vector([0.0, 0.0, 1.0]),volume=1.9993716814692758e2,arena_extrude_us=6259.125
M4_HOLE,shift=0e0,normal=Vector([0.0, 0.0, -1.0]),volume=1.9993716814692758e2,arena_extrude_us=4199.416
M4_HOLE,shift=1e9,normal=Vector([0.0, 0.0, 1.0]),volume=1.9993716814248077e2,arena_extrude_us=3940.167
M4_HOLE,shift=1e9,normal=Vector([0.0, 0.0, -1.0]),volume=1.9993716814248077e2,arena_extrude_us=3409.167
test sketch_feasibility::m50_real_m4_extrusion_preserves_oriented_holed_profiles ... ok
test sketch_feasibility::m50_real_m4_arena_preserves_cubic_profile ... ok
M4_FONT,O,holes=1,area=1.4815083278544815e1,volume=2.9630166557089865e1,expected_volume=2.963016655708963e1,arena_extrude_us=5604.583,coordinates=exact_half_font_units_promoted_before_scaling
FONT_OUTLINE,B,contours=3,synthetic_zero_length_closures=3
M4_FONT,B,holes=2,area=1.676673241288624e1,volume=3.353346482577224e1,expected_volume=3.353346482577248e1,arena_extrude_us=5567.041,coordinates=exact_half_font_units_promoted_before_scaling
FONT_OUTLINE,8,contours=3,synthetic_zero_length_closures=3
M4_FONT,8,holes=2,area=1.5510625684856425e1,volume=3.1021251369712836e1,expected_volume=3.102125136971285e1,arena_extrude_us=7916.084,coordinates=exact_half_font_units_promoted_before_scaling
test sketch_feasibility::m50_exact_font_contours_survive_real_m4_extrusion ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 18 filtered out; finished in 0.17s
```

## Historical earlier-execution records

These earlier recorded traces are preserved for provenance and variability. They
are superseded by the final current-run evidence above and appended below; their
compiler/conditions and source revisions must not be mixed into final percentiles.

## Raw deterministic measurement traces

All latency units are microseconds; volumes are mm³. The full run is retained
alongside the final curve/region supplements so variation and negative results
remain visible. `Unproven`/`Unavailable`, false expected-match fields and watchdog
lines are deliberate capability evidence, not successful acceptance.

### Complete release run

```text
M5.0,seed=0,iterations=500,parallelism=Seq,units=mm
SCALE_FIXTURE,S1,entities=1000,unknowns=3000,equations=3000,largest_block=128,nnz=5976,blocks=24,setup_us=313.917,scope=linear_backend_surrogate
SCALE_FINAL,S1,all_blocks_us=2233.542,max_solution_error=3.774758283725532e-14,integrated_final_validation=Unproven
SCALE_COUNTER,S1,explicit_components_evaluated_per_local_edit=1,other_components_evaluated=0,full_path_copy_allocations=Unmeasured,old_published_generation=NotIntegrated
LATENCY,S1_linear_component_edit,n=500,p50_us=43.500,p95_us=84.208,p99_us=265.583,max_us=2835.916
SCALE_FIXTURE,S2,entities=10000,unknowns=30000,equations=30000,largest_block=256,nnz=59882,blocks=118,setup_us=1517.166,scope=linear_backend_surrogate
SCALE_FINAL,S2,all_blocks_us=34226.542,max_solution_error=1.6919798895287386e-13,integrated_final_validation=Unproven
SCALE_COUNTER,S2,explicit_components_evaluated_per_local_edit=1,other_components_evaluated=0,full_path_copy_allocations=Unmeasured,old_published_generation=NotIntegrated
LATENCY,S2_linear_component_edit,n=500,p50_us=213.875,p95_us=448.541,p99_us=1505.833,max_us=3276.958
SCALE_FIXTURE,S3,entities=100000,unknowns=200000,equations=200000,largest_block=256,nnz=399218,blocks=782,setup_us=10564.708,scope=linear_backend_surrogate
SCALE_FINAL,S3,all_blocks_us=181665.708,max_solution_error=1.6919798895287386e-13,integrated_final_validation=Unproven
SCALE_COUNTER,S3,explicit_components_evaluated_per_local_edit=1,other_components_evaluated=0,full_path_copy_allocations=Unmeasured,old_published_generation=NotIntegrated
LATENCY,S3_linear_component_edit,n=500,p50_us=176.250,p95_us=241.000,p99_us=391.208,max_us=1443.208
RANK,rigid_segment,rows=1,cols=4,rank=1,linearized_nullity=3,mobility=Regular_analytic_fixture
LATENCY,dense_svd_rigid_segment,n=500,p50_us=0.542,p95_us=0.584,p99_us=0.667,max_us=60.375
RANK,duplicated_equations,rows=2,cols=2,rank=1,linearized_nullity=1,mobility=Regular_analytic_fixture
LATENCY,dense_svd_duplicated_equations,n=500,p50_us=0.709,p95_us=0.792,p99_us=0.833,max_us=4.417
RANK,toggle,rows=2,cols=2,rank=1,linearized_nullity=1,mobility=Singular_NotCertified
LATENCY,dense_svd_toggle,n=500,p50_us=0.542,p95_us=0.625,p99_us=0.625,max_us=1.792
RANK,tangent_fixed_circles,rows=2,cols=2,rank=1,linearized_nullity=1,mobility=Singular_NotCertified
LATENCY,dense_svd_tangent_fixed_circles,n=500,p50_us=0.583,p95_us=0.625,p99_us=0.667,max_us=1.041
RANK,active_parameter_bound,rows=1,cols=2,rank=1,linearized_nullity=1,mobility=Boundary_NotCertified
LATENCY,dense_svd_active_parameter_bound,n=500,p50_us=0.458,p95_us=0.500,p99_us=0.583,max_us=1.625
RANK,regular_grounded,rows=2,cols=2,rank=2,linearized_nullity=0,mobility=Regular_analytic_fixture
LATENCY,dense_svd_regular_grounded,n=500,p50_us=0.541,p95_us=0.583,p99_us=0.625,max_us=1.167
THRESHOLD,scale=1e-9,singular_values=[9.999999999999999e-10, 1.0000000000000001e-19],ranks=[1, 1, 0],classification=Uncertain
THRESHOLD,scale=1e0,singular_values=[1.0, 1e-10],ranks=[1, 1, 1],classification=Uncertain
THRESHOLD,scale=1e9,singular_values=[1000000000.0, 0.10000000000000002],ranks=[2, 1, 1],classification=Uncertain
GCS,bounded_oracle,max_residual=0e0,unknowns=2,equations=2,branch=positive_x
LATENCY,gcs_bounded_nonlinear_solve,n=500,p50_us=10.375,p95_us=18.750,p99_us=156.333,max_us=2274.708
FIXTURE,S1_component,unknowns=128,equations=128,nnz=255,setup_us=28.458,input_csc_lower_bound_bytes=6136
WORKSPACE,S1_component,numeric_values_bytes=5104,numeric_indices_bytes=6144,factor_scratch_bytes=4096,solve_scratch_bytes=1024,symbolic_heap_bytes=Unmeasured
SOLVE,S1_component,analytic_solution_max_error=3.774758283725532e-14,generic_large_numerical_rank=Unavailable
LATENCY,S1_component_symbolic,n=500,p50_us=25.125,p95_us=30.959,p99_us=45.709,max_us=167.875
LATENCY,S1_component_factor,n=500,p50_us=52.791,p95_us=69.125,p99_us=114.166,max_us=1921.291
LATENCY,S1_component_solve,n=500,p50_us=3.250,p95_us=4.458,p99_us=9.625,max_us=78.083
LATENCY,S1_component_cleanup,n=500,p50_us=0.416,p95_us=0.875,p99_us=1.750,max_us=3.209
LATENCY,S1_component_backend_total,n=500,p50_us=82.042,p95_us=108.000,p99_us=214.542,max_us=1962.708
FIXTURE,S2_component,unknowns=256,equations=256,nnz=511,setup_us=59.958,input_csc_lower_bound_bytes=12280
WORKSPACE,S2_component,numeric_values_bytes=10224,numeric_indices_bytes=12288,factor_scratch_bytes=8192,solve_scratch_bytes=2048,symbolic_heap_bytes=Unmeasured
SOLVE,S2_component,analytic_solution_max_error=1.6919798895287386e-13,generic_large_numerical_rank=Unavailable
LATENCY,S2_component_symbolic,n=500,p50_us=38.500,p95_us=58.958,p99_us=84.167,max_us=114.000
LATENCY,S2_component_factor,n=500,p50_us=143.584,p95_us=205.292,p99_us=251.334,max_us=313.334
LATENCY,S2_component_solve,n=500,p50_us=5.375,p95_us=8.375,p99_us=17.791,max_us=52.458
LATENCY,S2_component_cleanup,n=500,p50_us=0.250,p95_us=0.916,p99_us=1.709,max_us=11.583
LATENCY,S2_component_backend_total,n=500,p50_us=190.583,p95_us=274.542,p99_us=323.041,max_us=359.959
FIXTURE,C1_chain,unknowns=10000,equations=10000,nnz=19999,setup_us=386.166,input_csc_lower_bound_bytes=479992
WORKSPACE,C1_chain,numeric_values_bytes=399984,numeric_indices_bytes=480000,factor_scratch_bytes=320000,solve_scratch_bytes=80000,symbolic_heap_bytes=Unmeasured
SOLVE,C1_chain,analytic_solution_max_error=1.2067999932696694e-9,generic_large_numerical_rank=Unavailable
LATENCY,C1_chain_symbolic,n=500,p50_us=1333.625,p95_us=2372.334,p99_us=3696.458,max_us=17649.000
LATENCY,C1_chain_factor,n=500,p50_us=166629.916,p95_us=318287.083,p99_us=505617.584,max_us=982882.333
LATENCY,C1_chain_solve,n=500,p50_us=211.541,p95_us=307.166,p99_us=422.416,max_us=2577.167
LATENCY,C1_chain_cleanup,n=500,p50_us=2.125,p95_us=4.500,p99_us=6.708,max_us=8.167
LATENCY,C1_chain_backend_total,n=500,p50_us=168131.458,p95_us=321144.792,p99_us=508395.208,max_us=991295.750
FIXTURE,C1_grid,unknowns=10000,equations=19801,nnz=39601,setup_us=2668.458,input_csc_lower_bound_bytes=872032
WORKSPACE,C1_grid,numeric_values_bytes=15136296,numeric_indices_bytes=1730744,factor_scratch_bytes=1752192,solve_scratch_bytes=158720,symbolic_heap_bytes=Unmeasured
SOLVE,C1_grid,analytic_solution_max_error=2.5579538487363607e-13,generic_large_numerical_rank=Unavailable
LATENCY,C1_grid_symbolic,n=500,p50_us=6992.250,p95_us=20562.625,p99_us=39577.792,max_us=80858.709
LATENCY,C1_grid_factor,n=500,p50_us=19573.833,p95_us=48926.542,p99_us=83495.000,max_us=132208.666
LATENCY,C1_grid_solve,n=500,p50_us=3176.917,p95_us=7586.250,p99_us=15600.917,max_us=44293.209
LATENCY,C1_grid_cleanup,n=500,p50_us=5.250,p95_us=8.375,p99_us=29.625,max_us=63.916
LATENCY,C1_grid_backend_total,n=500,p50_us=29692.916,p95_us=75996.958,p99_us=121103.375,max_us=162870.792
CANCEL,sparse_numeric_C1_chain,atomic_latch_us=0.042,indivisible_us=240413.292,worker_release_us=240365.167,frozen_click_surrogate_us=240400.625,other_window_surrogate_us=240402.375,in_flight=true
CANCEL,sparse_symbolic_C1_grid,atomic_latch_us=0.042,indivisible_us=6521.334,worker_release_us=6509.542,frozen_click_surrogate_us=6516.792,other_window_surrogate_us=6518.250,in_flight=true
CANCEL,sparse_numeric_C1_grid,atomic_latch_us=0.000,indivisible_us=18842.500,worker_release_us=18832.917,frozen_click_surrogate_us=18840.209,other_window_surrogate_us=18841.667,in_flight=true
CANCEL,sparse_solve_C1_grid,atomic_latch_us=0.041,indivisible_us=3241.042,worker_release_us=3229.166,frozen_click_surrogate_us=3237.750,other_window_surrogate_us=3239.416,in_flight=true
CANCEL,dense_rank_256,atomic_latch_us=0.042,indivisible_us=15456.000,worker_release_us=15447.125,frozen_click_surrogate_us=15451.417,other_window_surrogate_us=15453.042,in_flight=true
INDEX,count=1000,setup_us=216.416,input_payload_bytes=16000
INDEX_EDIT,count=1000,remove_insert_us=3.083,explicit_entries_changed=1,internal_visits_and_allocations=Unmeasured
LATENCY,rstar_query_1000,n=500,p50_us=0.417,p95_us=0.542,p99_us=0.667,max_us=0.792
INDEX,count=10000,setup_us=1570.167,input_payload_bytes=160000
INDEX_EDIT,count=10000,remove_insert_us=2.541,explicit_entries_changed=1,internal_visits_and_allocations=Unmeasured
LATENCY,rstar_query_10000,n=500,p50_us=0.750,p95_us=1.042,p99_us=1.292,max_us=1.708
INDEX,count=100000,setup_us=17612.834,input_payload_bytes=1600000
INDEX_EDIT,count=100000,remove_insert_us=4.250,explicit_entries_changed=1,internal_visits_and_allocations=Unmeasured
LATENCY,rstar_query_100000,n=500,p50_us=1.500,p95_us=2.042,p99_us=2.458,max_us=4.459
PAIR,segment,segment,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=73.291,completeness=Unproven
PAIR,segment,circle_arc,shift=0e0,hits=1,overlaps=0,max_residual=4.214684887878958e-8,us=97.625,completeness=Unproven
PAIR,segment,ellipse_arc,shift=0e0,hits=0,overlaps=0,max_residual=0e0,us=9.333,completeness=Unproven
PAIR,segment,rational_conic,shift=0e0,hits=0,overlaps=0,max_residual=0e0,us=5.750,completeness=Unproven
PAIR,segment,bezier_bspline,shift=0e0,hits=1,overlaps=0,max_residual=0e0,us=953.416,completeness=Unproven
PAIR,circle_arc,circle_arc,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=6900.042,completeness=Unproven
PAIR,circle_arc,ellipse_arc,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=602.125,completeness=Unproven
PAIR,circle_arc,rational_conic,shift=0e0,hits=1,overlaps=0,max_residual=6.666666666666667e-1,us=91.792,completeness=Unproven
PAIR,circle_arc,bezier_bspline,shift=0e0,hits=0,overlaps=0,max_residual=0e0,us=83.042,completeness=Unproven
PAIR,ellipse_arc,ellipse_arc,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=6491.083,completeness=Unproven
PAIR,ellipse_arc,rational_conic,shift=0e0,hits=1,overlaps=0,max_residual=6.666666666666667e-1,us=102.750,completeness=Unproven
PAIR,ellipse_arc,bezier_bspline,shift=0e0,hits=0,overlaps=0,max_residual=0e0,us=61.667,completeness=Unproven
PAIR,rational_conic,rational_conic,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=7621.666,completeness=Unproven
PAIR,rational_conic,bezier_bspline,shift=0e0,hits=2,overlaps=0,max_residual=3.3796315915761384e-8,us=706.250,completeness=Unproven
PAIR,bezier_bspline,bezier_bspline,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=8770.916,completeness=Unproven
ANALYTIC,cross,shift=0e0,hits=1,overlaps=0,expected_hits=1,expected_overlap=0,matches=true
OPAQUE_START,partial_overlap,shift=0e0

WATCHDOG,partial_overlap,shift=0e0,wall_us=2001731.125,status=Unsupported_uninterruptible_timeout,process_kill_is_not_cancellation=true
OPAQUE_START,reversed_overlap,shift=0e0

WATCHDOG,reversed_overlap,shift=0e0,wall_us=2002372.709,status=Unsupported_uninterruptible_timeout,process_kill_is_not_cancellation=true
ANALYTIC,T_contact,shift=0e0,hits=1,overlaps=0,expected_hits=1,expected_overlap=0,matches=true
ANALYTIC,parabola_contact,shift=0e0,delta=0e0,hits=0,expected=1,matches=false,us=868.791
ANALYTIC,parabola_contact,shift=0e0,delta=1e-6,hits=0,expected=2,matches=false,us=750.667
ANALYTIC,parabola_contact,shift=0e0,delta=-1e-6,hits=0,expected=0,matches=true,us=3.750
LATENCY,intersection_tangent_shift_0e0,n=500,p50_us=783.959,p95_us=1208.667,p99_us=2656.250,max_us=10183.292
CANCEL,intersection_tangent_shift_0e0,atomic_latch_us=0.041,indivisible_us=718.708,worker_release_us=710.583,frozen_click_surrogate_us=719.375,other_window_surrogate_us=720.916,in_flight=true
TOLERANCE,shift=0e0,linear=0.0000001,angular=0.000000000001,relative=0.0000000001,tiny_gap_stored=1e-5,kernel_approx_equal=false,local_policy_equal=false,ulp=5e-324
PAIR,segment,segment,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=6.750,completeness=Unproven
PAIR,segment,circle_arc,shift=1e9,hits=1,overlaps=0,max_residual=0e0,us=86.000,completeness=Unproven
PAIR,segment,ellipse_arc,shift=1e9,hits=0,overlaps=0,max_residual=0e0,us=4.708,completeness=Unproven
PAIR,segment,rational_conic,shift=1e9,hits=0,overlaps=0,max_residual=0e0,us=1.917,completeness=Unproven
PAIR,segment,bezier_bspline,shift=1e9,hits=1,overlaps=0,max_residual=0e0,us=932.458,completeness=Unproven
PAIR,circle_arc,circle_arc,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=6519.042,completeness=Unproven
PAIR,circle_arc,ellipse_arc,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=517.166,completeness=Unproven
PAIR,circle_arc,rational_conic,shift=1e9,hits=1,overlaps=0,max_residual=6.666666269302368e-1,us=77.125,completeness=Unproven
PAIR,circle_arc,bezier_bspline,shift=1e9,hits=0,overlaps=0,max_residual=0e0,us=65.750,completeness=Unproven
PAIR,ellipse_arc,ellipse_arc,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=6819.375,completeness=Unproven
PAIR,ellipse_arc,rational_conic,shift=1e9,hits=1,overlaps=0,max_residual=6.666666269302368e-1,us=72.625,completeness=Unproven
PAIR,ellipse_arc,bezier_bspline,shift=1e9,hits=0,overlaps=0,max_residual=0e0,us=52.417,completeness=Unproven
PAIR,rational_conic,rational_conic,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=6352.292,completeness=Unproven
PAIR,rational_conic,bezier_bspline,shift=1e9,hits=3,overlaps=0,max_residual=3.769728732309794e-7,us=616.917,completeness=Unproven
PAIR,bezier_bspline,bezier_bspline,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=8791.250,completeness=Unproven
ANALYTIC,cross,shift=1e9,hits=1,overlaps=0,expected_hits=1,expected_overlap=0,matches=true
OPAQUE_START,partial_overlap,shift=1e9

WATCHDOG,partial_overlap,shift=1e9,wall_us=2001988.917,status=Unsupported_uninterruptible_timeout,process_kill_is_not_cancellation=true
OPAQUE_START,reversed_overlap,shift=1e9

WATCHDOG,reversed_overlap,shift=1e9,wall_us=2001489.625,status=Unsupported_uninterruptible_timeout,process_kill_is_not_cancellation=true
ANALYTIC,T_contact,shift=1e9,hits=1,overlaps=0,expected_hits=1,expected_overlap=0,matches=true
ANALYTIC,parabola_contact,shift=1e9,delta=0e0,hits=0,expected=1,matches=false,us=647.750
ANALYTIC,parabola_contact,shift=1e9,delta=1e-6,hits=0,expected=2,matches=false,us=932.834
ANALYTIC,parabola_contact,shift=1e9,delta=-1e-6,hits=0,expected=0,matches=true,us=2.166
LATENCY,intersection_tangent_shift_1e9,n=500,p50_us=541.208,p95_us=874.333,p99_us=1256.958,max_us=6586.750
CANCEL,intersection_tangent_shift_1e9,atomic_latch_us=0.042,indivisible_us=838.500,worker_release_us=825.792,frozen_click_surrogate_us=848.875,other_window_surrogate_us=850.417,in_flight=true
TOLERANCE,shift=1e9,linear=0.0000001,angular=0.000000000001,relative=0.0000000001,tiny_gap_stored=1.0013580322265625e-5,kernel_approx_equal=true,local_policy_equal=false,ulp=1.1920928955078125e-7
FONT,O,outline_segments=22,bounds=Rect { min: point(93.0, 1399.0), max: point(1107.0, -23.0) },font_units_per_em=Some(1950.0),contour_orientation_and_precision=Unproven
FONT,B,outline_segments=36,bounds=Rect { min: point(193.0, 1377.0), max: point(1079.0, 0.0) },font_units_per_em=Some(1950.0),contour_orientation_and_precision=Unproven
FONT,8,outline_segments=42,bounds=Rect { min: point(131.0, 1399.0), max: point(1069.0, -23.0) },font_units_per_em=Some(1950.0),contour_orientation_and_precision=Unproven
ADVANCED_EXTRUDE,ellipse_periodic,expected_volume=12.566370614359172,native_volume=12.566317699613245,error=5.291474592716838e-5,valid=true,us=1624753.833
ADVANCED_EXTRUDE,rational_quadratic_arc,expected_volume=0.5707963267948966,native_volume=0.5707963267948961,error=4.440892098500626e-16,valid=true,us=64.500
ADVANCED_EXTRUDE,parabolic_conic,expected_volume=2.6666666666666665,native_volume=2.666639539930557,error=2.7126736109384098e-5,valid=true,us=193454.708
ADVANCED_EXTRUDE,cubic_bezier,expected_volume=15,native_volume=14.999947397214164,error=5.2602785835631494e-5,valid=true,us=27886.917
NESTING,manual_exact_faces,outer_expected=128,outer_volume=128,island_expected=8,island_volume=8,automatic_arrangement=Unproven
EXTRUDE,exact_circular_hole,shift=0e0,flipped=false,volume_abs_error=1.9895196601282805e-13,invalid=0,tolerance_mm=1e-7
LATENCY,native_holed_extrude_0e0_false,n=500,p50_us=2.625,p95_us=2.834,p99_us=3.958,max_us=7.875
LATENCY,wire_face_setup_0e0_false,n=500,p50_us=0.583,p95_us=0.667,p99_us=0.833,max_us=2.167
LATENCY,native_volume_0e0_false,n=500,p50_us=95.417,p95_us=97.292,p99_us=134.791,max_us=188.667
LATENCY,native_validate_0e0_false,n=500,p50_us=25.625,p95_us=26.916,p99_us=32.958,max_us=50.250
EXTRUDE,exact_circular_hole,shift=0e0,flipped=true,volume_abs_error=1.9895196601282805e-13,invalid=0,tolerance_mm=1e-7
LATENCY,native_holed_extrude_0e0_true,n=500,p50_us=2.667,p95_us=2.875,p99_us=3.250,max_us=7.083
LATENCY,wire_face_setup_0e0_true,n=500,p50_us=0.584,p95_us=0.667,p99_us=0.750,max_us=1.291
LATENCY,native_volume_0e0_true,n=500,p50_us=95.584,p95_us=97.459,p99_us=104.875,max_us=147.542
LATENCY,native_validate_0e0_true,n=500,p50_us=25.542,p95_us=26.375,p99_us=31.417,max_us=37.500
CANCEL,dependent_extrude_0e0,atomic_latch_us=0.083,indivisible_us=7.042,worker_release_us=0.000,frozen_click_surrogate_us=4.000,other_window_surrogate_us=5.291,in_flight=false
EXTRUDE,exact_circular_hole,shift=1e9,flipped=false,volume_abs_error=9.288100955018308e-9,invalid=0,tolerance_mm=1e-7
LATENCY,native_holed_extrude_1e9_false,n=500,p50_us=2.667,p95_us=2.875,p99_us=3.166,max_us=4.625
LATENCY,wire_face_setup_1e9_false,n=500,p50_us=0.584,p95_us=0.667,p99_us=0.750,max_us=1.416
LATENCY,native_volume_1e9_false,n=500,p50_us=95.625,p95_us=98.250,p99_us=106.792,max_us=111.542
LATENCY,native_validate_1e9_false,n=500,p50_us=25.625,p95_us=26.500,p99_us=29.166,max_us=34.167
EXTRUDE,exact_circular_hole,shift=1e9,flipped=true,volume_abs_error=9.288100955018308e-9,invalid=0,tolerance_mm=1e-7
LATENCY,native_holed_extrude_1e9_true,n=500,p50_us=2.667,p95_us=2.875,p99_us=3.041,max_us=7.750
LATENCY,wire_face_setup_1e9_true,n=500,p50_us=0.583,p95_us=0.666,p99_us=0.709,max_us=1.834
LATENCY,native_volume_1e9_true,n=500,p50_us=95.583,p95_us=96.917,p99_us=107.042,max_us=125.666
LATENCY,native_validate_1e9_true,n=500,p50_us=25.417,p95_us=26.125,p99_us=26.917,max_us=36.792
CANCEL,dependent_extrude_1e9,atomic_latch_us=0.042,indivisible_us=6.417,worker_release_us=0.000,frozen_click_surrogate_us=3.625,other_window_surrogate_us=4.708,in_flight=false
CANCEL,wire_and_face_construction,atomic_latch_us=0.042,indivisible_us=29.542,worker_release_us=17.083,frozen_click_surrogate_us=19.958,other_window_surrogate_us=21.000,in_flight=true
OFFSET,holed_polygon,join=Intersection,result=Ok(Id(2)),hole_and_self_intersection_certification=Unproven
OFFSET,exact_periodic_circle,join=Intersection,result=Ok(Id(1)),analytic_expected_radius=2.2
OFFSET_ACCURACY,circle,join=Intersection,lines=32,circles=0,endpoint_radius_error=9.677144752622269e-4,chord_midpoint_radius_error=9.63054665560703e-3,certified_bound=Unavailable
OFFSET,holed_polygon,join=Arc,result=Ok(Id(2)),hole_and_self_intersection_certification=Unproven
OFFSET,exact_periodic_circle,join=Arc,result=Ok(Id(1)),analytic_expected_radius=2.2
OFFSET_ACCURACY,circle,join=Arc,lines=32,circles=32,endpoint_radius_error=8.756785170707104e-4,chord_midpoint_radius_error=9.630546655606587e-3,certified_bound=Unavailable
OFFSET,holed_polygon,join=Chamfer,result=Ok(Id(2)),hole_and_self_intersection_certification=Unproven
OFFSET,exact_periodic_circle,join=Chamfer,result=Ok(Id(1)),analytic_expected_radius=2.2
OFFSET_ACCURACY,circle,join=Chamfer,lines=64,circles=0,endpoint_radius_error=8.756785170707104e-4,chord_midpoint_radius_error=9.630546655606587e-3,certified_bound=Unavailable
M4_PROFILE,cubic,volume=29.999999999999915,expected=30,us=2185.500,full_app_path=Unproven
M4_ARENA,face=0,result=Ok(Ok(11.999999999999998))
M4_ARENA,face=1,result=Ok(Ok(11.999999999999998))
M4_ARENA,face=2,result=Ok(Ok(16.000000000000018))
M4_ARENA,face=3,result=Ok(Ok(16.000000000000018))
M4_ARENA,face=4,result=Ok(Ok(23.999999999999996))
M4_ARENA,face=5,result=Ok(Ok(23.999999999999996))
```

### Full-run process resources

```text
      132.56 real       117.49 user         1.31 sys
            99106816  maximum resident set size
                   0  average shared memory size
                   0  average unshared data size
                   0  average unshared stack size
               17645  page reclaims
                   2  page faults
                   0  swaps
                   0  block input operations
                   0  block output operations
                   0  messages sent
                   0  messages received
                   0  signals received
                  12  voluntary context switches
              238012  involuntary context switches
       1248097840508  instructions retired
        219623022826  cycles elapsed
            96600592  peak memory footprint
```

### Final contact/seam/self-intersection supplement

```text
M5.0,seed=0,iterations=500,parallelism=Seq,units=mm
PAIR,segment,segment,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=92.708,completeness=Unproven
PAIR,segment,circle_arc,shift=0e0,hits=1,overlaps=0,max_residual=4.214684887878958e-8,us=114.583,completeness=Unproven
PAIR,segment,ellipse_arc,shift=0e0,hits=0,overlaps=0,max_residual=0e0,us=5.167,completeness=Unproven
PAIR,segment,rational_conic,shift=0e0,hits=0,overlaps=0,max_residual=0e0,us=2.208,completeness=Unproven
PAIR,segment,bezier_bspline,shift=0e0,hits=1,overlaps=0,max_residual=0e0,us=878.500,completeness=Unproven
PAIR,circle_arc,circle_arc,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=5906.292,completeness=Unproven
PAIR,circle_arc,ellipse_arc,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=427.125,completeness=Unproven
PAIR,circle_arc,rational_conic,shift=0e0,hits=1,overlaps=0,max_residual=6.666666666666667e-1,us=65.667,completeness=Unproven
PAIR,circle_arc,bezier_bspline,shift=0e0,hits=0,overlaps=0,max_residual=0e0,us=55.166,completeness=Unproven
PAIR,ellipse_arc,ellipse_arc,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=5097.625,completeness=Unproven
PAIR,ellipse_arc,rational_conic,shift=0e0,hits=1,overlaps=0,max_residual=6.666666666666667e-1,us=56.708,completeness=Unproven
PAIR,ellipse_arc,bezier_bspline,shift=0e0,hits=0,overlaps=0,max_residual=0e0,us=38.458,completeness=Unproven
PAIR,rational_conic,rational_conic,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=4631.584,completeness=Unproven
PAIR,rational_conic,bezier_bspline,shift=0e0,hits=2,overlaps=0,max_residual=3.3796315915761384e-8,us=410.333,completeness=Unproven
PAIR,bezier_bspline,bezier_bspline,shift=0e0,hits=0,overlaps=1,max_residual=0e0,us=6278.542,completeness=Unproven
ANALYTIC,cross,shift=0e0,hits=1,overlaps=0,expected_hits=1,expected_overlap=0,matches=true
OPAQUE_START,partial_overlap,shift=0e0

WATCHDOG,partial_overlap,shift=0e0,wall_us=2001327.125,status=Unsupported_uninterruptible_timeout,process_kill_is_not_cancellation=true
OPAQUE_START,reversed_overlap,shift=0e0

WATCHDOG,reversed_overlap,shift=0e0,wall_us=2002200.417,status=Unsupported_uninterruptible_timeout,process_kill_is_not_cancellation=true
ANALYTIC,T_contact,shift=0e0,hits=1,overlaps=0,expected_hits=1,expected_overlap=0,matches=true
ANALYTIC,parabola_contact,shift=0e0,delta=0e0,hits=0,expected=1,matches=false,us=469.000
ANALYTIC,parabola_contact,shift=0e0,delta=1e-6,hits=0,expected=2,matches=false,us=463.083
ANALYTIC,parabola_contact,shift=0e0,delta=-1e-6,hits=0,expected=0,matches=true,us=1.958
LATENCY,intersection_tangent_shift_0e0,n=500,p50_us=484.125,p95_us=585.417,p99_us=664.250,max_us=747.375
CANCEL,intersection_tangent_shift_0e0,atomic_latch_us=0.042,indivisible_us=520.584,worker_release_us=480.000,frozen_click_surrogate_us=633.042,other_window_surrogate_us=635.000,in_flight=true
OPAQUE_START,periodic_seam,shift=0e0
SPECIAL,periodic_seam,shift=0e0,hits=1,overlaps=0,expected_distinct_geometric_points=2,us=294.000

OPAQUE_START,figure_eight,shift=0e0
SPECIAL,figure_eight,shift=0e0,hits=1,overlaps=0,us=172.292,expected_nonadjacent_parameters=0.25_0.75
SPECIAL_PARAMETERS,0.49999997519261874,0.5,residual=9.922952504126618e-8

TOLERANCE,shift=0e0,linear=0.0000001,angular=0.000000000001,relative=0.0000000001,tiny_gap_stored=1e-5,kernel_approx_equal=false,local_policy_equal=false,ulp=5e-324
PAIR,segment,segment,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=5.625,completeness=Unproven
PAIR,segment,circle_arc,shift=1e9,hits=1,overlaps=0,max_residual=0e0,us=65.125,completeness=Unproven
PAIR,segment,ellipse_arc,shift=1e9,hits=0,overlaps=0,max_residual=0e0,us=3.750,completeness=Unproven
PAIR,segment,rational_conic,shift=1e9,hits=0,overlaps=0,max_residual=0e0,us=1.709,completeness=Unproven
PAIR,segment,bezier_bspline,shift=1e9,hits=1,overlaps=0,max_residual=0e0,us=754.959,completeness=Unproven
PAIR,circle_arc,circle_arc,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=4752.458,completeness=Unproven
PAIR,circle_arc,ellipse_arc,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=395.583,completeness=Unproven
PAIR,circle_arc,rational_conic,shift=1e9,hits=1,overlaps=0,max_residual=6.666666269302368e-1,us=56.125,completeness=Unproven
PAIR,circle_arc,bezier_bspline,shift=1e9,hits=0,overlaps=0,max_residual=0e0,us=78.916,completeness=Unproven
PAIR,ellipse_arc,ellipse_arc,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=4807.875,completeness=Unproven
PAIR,ellipse_arc,rational_conic,shift=1e9,hits=1,overlaps=0,max_residual=6.666666269302368e-1,us=51.542,completeness=Unproven
PAIR,ellipse_arc,bezier_bspline,shift=1e9,hits=0,overlaps=0,max_residual=0e0,us=37.417,completeness=Unproven
PAIR,rational_conic,rational_conic,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=4582.792,completeness=Unproven
PAIR,rational_conic,bezier_bspline,shift=1e9,hits=3,overlaps=0,max_residual=3.769728732309794e-7,us=445.875,completeness=Unproven
PAIR,bezier_bspline,bezier_bspline,shift=1e9,hits=0,overlaps=1,max_residual=0e0,us=6384.541,completeness=Unproven
ANALYTIC,cross,shift=1e9,hits=1,overlaps=0,expected_hits=1,expected_overlap=0,matches=true
OPAQUE_START,partial_overlap,shift=1e9

WATCHDOG,partial_overlap,shift=1e9,wall_us=2007222.416,status=Unsupported_uninterruptible_timeout,process_kill_is_not_cancellation=true
OPAQUE_START,reversed_overlap,shift=1e9

WATCHDOG,reversed_overlap,shift=1e9,wall_us=2002844.333,status=Unsupported_uninterruptible_timeout,process_kill_is_not_cancellation=true
ANALYTIC,T_contact,shift=1e9,hits=1,overlaps=0,expected_hits=1,expected_overlap=0,matches=true
ANALYTIC,parabola_contact,shift=1e9,delta=0e0,hits=0,expected=1,matches=false,us=475.375
ANALYTIC,parabola_contact,shift=1e9,delta=1e-6,hits=0,expected=2,matches=false,us=730.625
ANALYTIC,parabola_contact,shift=1e9,delta=-1e-6,hits=0,expected=0,matches=true,us=3.792
LATENCY,intersection_tangent_shift_1e9,n=500,p50_us=481.750,p95_us=568.291,p99_us=661.000,max_us=759.625
CANCEL,intersection_tangent_shift_1e9,atomic_latch_us=0.041,indivisible_us=542.250,worker_release_us=532.583,frozen_click_surrogate_us=542.833,other_window_surrogate_us=544.333,in_flight=true
OPAQUE_START,periodic_seam,shift=1e9
SPECIAL,periodic_seam,shift=1e9,hits=1,overlaps=0,expected_distinct_geometric_points=2,us=268.041

OPAQUE_START,figure_eight,shift=1e9
SPECIAL,figure_eight,shift=1e9,hits=1,overlaps=0,us=261.500,expected_nonadjacent_parameters=0.25_0.75
SPECIAL_PARAMETERS,0.4999999574378508,0.5,residual=1.1920928955078125e-7

TOLERANCE,shift=1e9,linear=0.0000001,angular=0.000000000001,relative=0.0000000001,tiny_gap_stored=1.0013580322265625e-5,kernel_approx_equal=true,local_policy_equal=false,ulp=1.1920928955078125e-7
FONT,O,outline_segments=22,bounds=Rect { min: point(93.0, 1399.0), max: point(1107.0, -23.0) },font_units_per_em=Some(1950.0),contour_orientation_and_precision=Unproven
FONT,B,outline_segments=36,bounds=Rect { min: point(193.0, 1377.0), max: point(1079.0, 0.0) },font_units_per_em=Some(1950.0),contour_orientation_and_precision=Unproven
FONT,8,outline_segments=42,bounds=Rect { min: point(131.0, 1399.0), max: point(1069.0, -23.0) },font_units_per_em=Some(1950.0),contour_orientation_and_precision=Unproven
```

### Final curve/profile/offset supplement

```text
M5.0,seed=0,iterations=500,parallelism=Seq,units=mm
OFFSET_ADVANCED,ellipse_periodic,join=Intersection,result=Ok(Id(1)),certified_error_bound=Unavailable
OFFSET_ADVANCED,ellipse_periodic,join=Arc,result=Ok(Id(2)),certified_error_bound=Unavailable
OFFSET_ADVANCED,ellipse_periodic,join=Chamfer,result=Ok(Id(3)),certified_error_bound=Unavailable
ADVANCED_EXTRUDE,ellipse_periodic,expected_volume=12.566370614359172,native_volume=12.566317699613245,error=5.291474592716838e-5,valid=true,us=790167.625
OFFSET_ADVANCED,rational_quadratic_arc,join=Intersection,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,rational_quadratic_arc,join=Arc,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,rational_quadratic_arc,join=Chamfer,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
ADVANCED_EXTRUDE,rational_quadratic_arc,expected_volume=0.5707963267948966,native_volume=0.5707963267948961,error=4.440892098500626e-16,valid=true,us=115.542
OFFSET_ADVANCED,parabolic_conic,join=Intersection,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,parabolic_conic,join=Arc,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,parabolic_conic,join=Chamfer,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
ADVANCED_EXTRUDE,parabolic_conic,expected_volume=2.6666666666666665,native_volume=2.666639539930557,error=2.7126736109384098e-5,valid=true,us=202316.875
OFFSET_ADVANCED,cubic_bezier,join=Intersection,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,cubic_bezier,join=Arc,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
OFFSET_ADVANCED,cubic_bezier,join=Chamfer,result=Err(InvalidInput { reason: "wire must have at least 3 vertices" }),certified_error_bound=Unavailable
ADVANCED_EXTRUDE,cubic_bezier,expected_volume=15,native_volume=14.999947397214164,error=5.2602785835631494e-5,valid=true,us=28296.208
NESTING,manual_exact_faces,outer_expected=128,outer_volume=128,island_expected=8,island_volume=8,automatic_arrangement=Unproven
EXTRUDE,exact_circular_hole,shift=0e0,flipped=false,volume_abs_error=1.9895196601282805e-13,invalid=0,tolerance_mm=1e-7
LATENCY,native_holed_extrude_0e0_false,n=500,p50_us=2.583,p95_us=3.000,p99_us=5.916,max_us=15.166
LATENCY,wire_face_setup_0e0_false,n=500,p50_us=0.583,p95_us=0.667,p99_us=1.167,max_us=9.583
LATENCY,native_volume_0e0_false,n=500,p50_us=95.042,p95_us=100.250,p99_us=114.708,max_us=197.250
LATENCY,native_validate_0e0_false,n=500,p50_us=25.333,p95_us=26.625,p99_us=39.208,max_us=53.333
EXTRUDE,exact_circular_hole,shift=0e0,flipped=true,volume_abs_error=1.9895196601282805e-13,invalid=0,tolerance_mm=1e-7
LATENCY,native_holed_extrude_0e0_true,n=500,p50_us=2.666,p95_us=2.833,p99_us=4.250,max_us=7.542
LATENCY,wire_face_setup_0e0_true,n=500,p50_us=0.583,p95_us=0.625,p99_us=0.750,max_us=2.375
LATENCY,native_volume_0e0_true,n=500,p50_us=95.125,p95_us=100.542,p99_us=109.541,max_us=144.459
LATENCY,native_validate_0e0_true,n=500,p50_us=25.417,p95_us=26.417,p99_us=33.792,max_us=45.542
CANCEL,dependent_extrude_0e0,atomic_latch_us=0.041,indivisible_us=10.041,worker_release_us=0.000,frozen_click_surrogate_us=25.000,other_window_surrogate_us=26.708,in_flight=false
EXTRUDE,exact_circular_hole,shift=1e9,flipped=false,volume_abs_error=9.288100955018308e-9,invalid=0,tolerance_mm=1e-7
LATENCY,native_holed_extrude_1e9_false,n=500,p50_us=2.625,p95_us=2.834,p99_us=3.875,max_us=9.542
LATENCY,wire_face_setup_1e9_false,n=500,p50_us=0.583,p95_us=0.667,p99_us=1.042,max_us=2.542
LATENCY,native_volume_1e9_false,n=500,p50_us=95.209,p95_us=98.417,p99_us=109.500,max_us=136.666
LATENCY,native_validate_1e9_false,n=500,p50_us=25.416,p95_us=26.500,p99_us=38.041,max_us=85.167
EXTRUDE,exact_circular_hole,shift=1e9,flipped=true,volume_abs_error=9.288100955018308e-9,invalid=0,tolerance_mm=1e-7
LATENCY,native_holed_extrude_1e9_true,n=500,p50_us=2.667,p95_us=2.875,p99_us=3.083,max_us=3.708
LATENCY,wire_face_setup_1e9_true,n=500,p50_us=0.583,p95_us=0.625,p99_us=0.791,max_us=1.209
LATENCY,native_volume_1e9_true,n=500,p50_us=95.417,p95_us=98.500,p99_us=112.709,max_us=139.875
LATENCY,native_validate_1e9_true,n=500,p50_us=25.416,p95_us=26.167,p99_us=33.958,max_us=88.208
CANCEL,dependent_extrude_1e9,atomic_latch_us=0.042,indivisible_us=12.292,worker_release_us=2.584,frozen_click_surrogate_us=14.917,other_window_surrogate_us=16.292,in_flight=true
CANCEL,wire_and_face_construction,atomic_latch_us=0.042,indivisible_us=14.500,worker_release_us=5.792,frozen_click_surrogate_us=8.959,other_window_surrogate_us=10.125,in_flight=true
OFFSET,holed_polygon,join=Intersection,result=Ok(Id(2)),hole_and_self_intersection_certification=Unproven
OFFSET,exact_periodic_circle,join=Intersection,result=Ok(Id(1)),analytic_expected_radius=2.2
OFFSET_ACCURACY,circle,join=Intersection,lines=32,circles=0,endpoint_radius_error=9.677144752622269e-4,chord_midpoint_radius_error=9.63054665560703e-3,certified_bound=Unavailable
OFFSET,holed_polygon,join=Arc,result=Ok(Id(2)),hole_and_self_intersection_certification=Unproven
OFFSET,exact_periodic_circle,join=Arc,result=Ok(Id(1)),analytic_expected_radius=2.2
OFFSET_ACCURACY,circle,join=Arc,lines=32,circles=32,endpoint_radius_error=8.756785170707104e-4,chord_midpoint_radius_error=9.630546655606587e-3,certified_bound=Unavailable
OFFSET,holed_polygon,join=Chamfer,result=Ok(Id(2)),hole_and_self_intersection_certification=Unproven
OFFSET,exact_periodic_circle,join=Chamfer,result=Ok(Id(1)),analytic_expected_radius=2.2
OFFSET_ACCURACY,circle,join=Chamfer,lines=64,circles=0,endpoint_radius_error=8.756785170707104e-4,chord_midpoint_radius_error=9.630546655606587e-3,certified_bound=Unavailable
M4_PROFILE,cubic,volume=29.999999999999915,expected=30,us=2240.625,full_app_path=Unproven
M4_ARENA,face=0,result=Ok(Ok(11.999999999999998))
M4_ARENA,face=1,result=Ok(Ok(11.999999999999998))
M4_ARENA,face=2,result=Ok(Ok(16.000000000000018))
M4_ARENA,face=3,result=Ok(Ok(16.000000000000018))
M4_ARENA,face=4,result=Ok(Ok(23.999999999999996))
M4_ARENA,face=5,result=Ok(Ok(23.999999999999996))
```

### Subsequent Rust 1.88 M4 font subset test

```text
   Compiling newell-kernel v0.1.0 (/Users/martin/Desktop/Newell/crates/newell-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.20s
     Running unittests src/lib.rs (target/debug/deps/newell_kernel-d9a2f0c3100617b1)

running 3 tests
FONT_OUTLINE,O,contours=2,synthetic_zero_length_closures=2
test sketch_feasibility::m50_real_m4_extrusion_preserves_oriented_holed_profiles ... ok
M4_FONT,O,holes=1,area=1.4815083278544815e1,volume=2.9630166557089865e1,expected_volume=2.963016655708963e1,arena_extrude_us=6191.959,coordinates=exact_half_font_units_promoted_before_scaling
FONT_OUTLINE,B,contours=3,synthetic_zero_length_closures=3
test sketch_feasibility::m50_real_m4_arena_preserves_cubic_profile ... ok
M4_FONT,B,holes=2,area=1.676673241288624e1,volume=3.353346482577224e1,expected_volume=3.353346482577248e1,arena_extrude_us=6156.500,coordinates=exact_half_font_units_promoted_before_scaling
FONT_OUTLINE,8,contours=3,synthetic_zero_length_closures=3
M4_FONT,8,holes=2,area=1.5510625684856425e1,volume=3.1021251369712836e1,expected_volume=3.102125136971285e1,arena_extrude_us=8809.458,coordinates=exact_half_font_units_promoted_before_scaling
test sketch_feasibility::m50_exact_font_contours_survive_real_m4_extrusion ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 18 filtered out; finished in 0.21s
```
