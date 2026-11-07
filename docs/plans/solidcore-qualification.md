# SolidCore qualification against the accepted M4 prototype

> Historical pre-cutover evidence. Geometry-specific source and package references
> below are archived context; ADR 0010 and the current migration record govern
> the present checkout. Independent rank and `faer` work remains retained.

> Production migration update — 2026-10-30: the human has decided adoption and
> authorized ADR 0010 and canonical amendments. [Production migration revision 1](solidcore-production-migration.md)
> is now the implementation authority to approve; no standalone qualification
> project is a prerequisite. Revision 2 below is preserved as historical planning.
> Its F01–F20 fixtures, invariants, tolerance ledger and measurement recipes supply
> migration acceptance evidence. Its qualification-only routing/file scope and
> proposed architecture exception are superseded, and its 50h cap applies only to
> the original Q0–Q4 qualification, not the production migration. Neither plan's
> existence launches Executor. No qualification results are invented by adoption.

Revision 2 — 2026-10-30. Architect / Codex. **Proposed; planning only.**

One targeted correction of revision 1 following the human-requested Challenger
review. Only this plan and Draft ADR 0010 change. Candidate revision, prototype
scope, 50-person-hour total, package caps and investigation limits are retained;
this revision grants no execution or architecture authority.

## 1. Contract, authority and inspected state

Decide whether replacing SolidCore would benefit Newell, using reproducible evidence
from M4.5–M4.10's accepted **prototype scope**. This document authorizes nothing by
itself. No migration, production selection, dependency change or architecture
amendment has been made. The only outputs of this stage are this plan and
[Draft ADR 0010](../adr/0010-solidcore-qualification.md).

Inputs: `AGENTS.md`, `docs/agents/{README,architect}.md`, `ARCHITECTURE.md`, accepted
ADRs 0001/0004/0009, `Cargo.toml`, kernel/app manifests, `deny.toml`, kernel
`src/{lib,extrude,profile,surface,termination,boolean}.rs`, model
`src/geometry.rs`, app `src/{geometry_worker,extrude_session,app}.rs`, kernel
`tests/it/extrude_feasibility.rs`, M4 plan/validation, and relevant M5 revision-6
plan/validation and current worker/rank interfaces. Upstream inspection used
official GitHub source only; no upstream checkout was created.

Starting checkout: `/Users/martin/Desktop/Newell`, `main`, HEAD
`29321e9efc1a18753e18a081de598b5ac2f9f70b`. Index empty. Existing unstaged work:

- `Cargo.lock`; app `Cargo.toml`, `src/{app,controller,geometry_worker,navigation}.rs`;
  kernel `Cargo.toml`, `benches/sketch.rs`, `src/lib.rs`,
  `src/sketch/{mod,rank}.rs`, `tests/it/{sketch_curves,sketch_solver}.rs`.
- `docs/{backlog.md,plans/M5-sketcher.md,validation/M5-sketcher.md}`;
  `vendor/manifest.toml`; all three approved vendor `NEWELL_CHANGES.md` ledgers;
  SolidCore math manifest and `src/nurbs/{certified_intersection,certified_overlap,mod}.rs`;
  operations `src/certified_offset.rs`; faer `src/sparse/linalg/{qr,rank}.rs`.
- Untracked kernel `src/sketch/feasibility.rs`, math
  `src/nurbs/{certified_arithmetic,certified_polynomial}.rs`, faer
  `src/sparse/linalg/rank_front.rs`.

Revision-2 entry reinspection found the same main/HEAD, empty index and inherited
changes listed above, plus this plan and Draft ADR 0010 as untracked documents.
Additional correction inputs are kernel `extrude.rs` key privacy, worker
`modifier_crossings` and retention fixture, M4 §6.1 tolerance policy, and pinned
SolidCore kernel/context/tolerance, planar-face and trim implementations. These
are source inspections, not runtime verification.

During this documentation pass, status additionally showed changes outside this
task in `ARCHITECTURE.md`, renderer `src/{lib,renderer,scene}.rs`,
`docs/ui_design/README_UI.md`, and new renderer `src/{grid.rs,grid.wgsl}`;
the already-dirty app path also acquired world-grid redraw wiring. Read-only
inspection of the architecture delta found an adaptive world-grid exception,
with no change to kernel §§5–5.2 or licensing. This leaves C1–C4's inspected key,
retention, tolerance and modifier contracts unchanged; later rendering/performance
baseline evidence must describe the actual grid/render state of that checkout.
These intervening changes were preserved, not written by this Architect stage.

These are live M5 experimental inputs, not a clean published SolidCore baseline.
Preserve every input and the Git index. No branches, worktrees, snapshots, agents,
builds, installations, tests or benchmarks were used in this Architect stage.
Historical validation is evidence of its recorded checkpoint, not proof that
today's dirty checkout passes. Future execution records material intervening
changes by path; no snapshot or commit is a prerequisite.

### Binding conflict and proposed resolution

Architecture §5 permits SolidCore exclusively, §5.1 isolates its dependencies,
ADR 0001 forbids a fallback, and ADR 0009 explicitly excludes another kernel.
Executing SolidCore now would violate those rules. The precise proposed exception
is in §8 and Draft ADR 0010. **Before Q0 execution, the human must explicitly
accept the exception and request Executor work on this identified plan revision.**
The human must authorize applying its architecture text; approval of a test plan
alone is not an implicit override. No accepted document is edited in this stage.

### Revision-2 Challenger disposition

| Finding | Verified evidence and smallest correction | Affected scope and future acceptance |
|---|---|---|
| C1 blocking — inaccessible key allocation | `extrude.rs` owns both the private `NewellShapeKey(u64)` field and private `reserve_key`; the sibling adapter cannot use either. Specify one feature-gated crate-private key issuer; preserve production allocation unchanged | §7 adds only that seam and its local tests to future `extrude.rs` scope; Q1 proves candidate-only allocation, exhaustion atomicity and F05 outputs on Rust1.88 |
| C2 blocking — unavailable Undo/Redo | Worker `revision6_completion_worker_native_retention_1000_cycles` explicitly uses inverse/reapplied edits, not an Undo journal. Replace the ambiguous 250×four-action wording with 20 complete warmup and 1,000 complete measured edit/inverse/reapply/Cancel iterations | §6 fixes recipes and exact action/commit/cancel counts; Q3 demonstrates retention through existing APIs and labels actual Undo/Redo unqualified; no document/model scope added |
| C3 important — effective tolerance mapping | Kernel defaults are coarser than Newell's policy; `Kernel::new` leaves context defaults independent. Trim wrapper passes 0.01; planar-face lower routine ignores that argument and uses B-Rep tolerance, so the constant alone proves no failure | §3.1 defines units, controls, overrides and a bounded public-route Q1 gate; F19-T adds one planar narrow-slot witness, reused in Q2; no upstream edits or claim of runtime failure |
| C4 important — missing collapsed second endpoint | Existing rejection is `side==6 && start==-2`; selected `(1,1,6)` starts at zero. Add verified `(0,8,6)` without replacing existing coverage | F13 becomes 54 unique preparations per kernel; extra collapse attempts, categories and atomicity counts are specified below; Q2 uses the existing interval/diagnostic oracle |

All four corrections are charged to Q1/Q2/Q3's existing caps. No additional
package, dependency, supported geometry class or automatic review is introduced.

## 2. Comparison baseline and present integration map

The controlling baseline is [M4 plan §12](M4.5-M4.10-extrude-pipeline.md#12-prototype-scope-amendment--2026-10-29),
including its retained obligations, and the
[prototype closure](../validation/M4.5-M4.10-extrude-pipeline.md#prototype-closure--2026-10-29).
Do not restore the original revision-2 advanced geometry as migration prerequisites.

Recorded implementation evidence includes actual Solid/Surface New/Add/Remove,
bounded planar UpTo, supported ruled/cylindrical ThroughAll, crossing generated
domains, endpoint pinches, invalid-interior-pinch rejection, active modifiers,
All/explicit scopes, replay/identity/rollback/retry/cancel and exactly-once
acceptance. The record reports 120 application cases and 7,770 modifier scenarios
inside enclosing tests; these are not 7,890 independent top-level tests. Its
1,078 workspace and 20 explicit GPU passes are historical. The finite synthetic
desktop checklist is completed tool observation. **Physical mouse/trackpad,
display/DPI readability/alignment, and perceived responsiveness/Cancel/restart
acceptance remain human-only pending items.** Do not close M4 checkboxes here.

M4.D1 nonlinear support correspondence and M4.D2 general trimmed rational targets
remain deferred. Unsupported requests must still reject safely without guessed
references, approximate replacements or partial mutation.

| Boundary | Current dependency/behavior | Qualification obligation |
|---|---|---|
| Workspace | Five crates, edition 2024, Rust 1.88, AGPL-3.0-or-later, `unsafe_code=forbid` | Preserve all four; no runtime FFI, C/C++ or BLAS |
| `newell-kernel/Cargo.toml` | Only direct SolidCore declarations: math/operations/topology `=4.2.2`; optional faer `=0.22.6` | Candidate dependency stays here only; no native types escape |
| Root source resolution | Math/operations/faer local patches under ADR 0009; topology registry resolution | Label baseline “current Newell retired kernel version 4.2.2 + local experimental sources”, not stock registry 4.2.2 |
| Kernel ACL | `NewellGeometry`, private `NewellShapeKey`, owned profiles/results/mesh/error | Experimental adapter uses Newell-owned DTOs and error translation; preserve production API |
| Geometry implementation | `extrude.rs` retains arena, exact sweep/domain recipes, provenance and cancellation; `surface/boolean/termination` contain significant Newell geometry logic | Measure adapter effort and any required port separately from upstream operation capability |
| Model | `geometry.rs`: `FaceRef` is entity/generation plus `u32` face; wire/edge are qualified occurrences, not the earlier plan's hypothetical key structs | Preserve actual contract, producer/insertion validation and unique construction provenance |
| Replay | Worker `replayed_face` uses unique non-Unknown construction provenance, otherwise full descriptor equality; ambiguity fails | Do not substitute native handle equality, enumeration ordinal matching or spatial resemblance |
| Worker | One worker owns native state, replay and staged/committed generations; messages contain recipes, finalized `PreparedDocument`, integer token and `NewellMesh` | Same authority, retention, bounded inbox and stale rejection with candidate geometry |
| Application | `ExtrudeGeometrySession`, `DocumentSession::accept_prepared`, controller input and scene publication | Preserve real prepare/finalize/accept path, Formula source, Properties and delivery-only retries |
| Renderer | `NewellMesh`: positions/normals, triangle indices, face offsets, independent feature-edge polylines; `SceneSnapshot` binds entity/generation | Rendering/picking stays kernel-independent; mesh precision loss remains a typed failure |

Default M4 comparison runs exclude `m50-feasibility`; separately verify that the
candidate feature can coexist with current M5 experiments. Do not remove local
patches, edit installed registries or switch dependencies to manufacture a stock
baseline. An upstream-only baseline, if later desired, requires separate scope.

## 3. Immutable candidate and verified source facts

Planning reference selected from the official `commits/main` endpoint on
2026-10-30: **`d4cb45e1f426a45467d8108ec076f66286f2c348`**, commit timestamp
2026-10-30T11:17:18Z. This is a source reference for the proposed experiment,
**not an adopted Newell dependency pin**. Never qualify moving `main`.

All following links are immutable official source for that revision:

- [Commit](https://github.com/tangency-labs/solidcore/commit/d4cb45e1f426a45467d8108ec076f66286f2c348),
  [root manifest](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/Cargo.toml),
  [kernel manifest](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/Cargo.toml),
  [toolchain](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/rust-toolchain.toml).
  Kernel package `solidcore` is 0.0.1, edition 2021, declared Rust 1.85; root
  development toolchain is **1.98.0**. The kernel manifest has only `thiserror = 2`
  as a direct dependency. This declaration does not establish actual Rust 1.88
  compatibility; Q0 must compile the selected dependency closure on 1.88.
- [Library](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/lib.rs)
  forbids unsafe and exports `Kernel`, `Brep`, typed IDs and `SolidError`.
  [Kernel](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/kernel.rs)
  owns tolerances, deterministic RNG, caches and context; it has interior mutable
  state, so no Send/Sync inference is justified. `Kernel::new(&KernelOptions)`
  initializes `UpstreamFixes::default()` with fixes off. Explicit construction
  does not prove independent caches or cancellation behavior.
- [LICENSE](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/LICENSE),
  [NOTICE](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/NOTICE),
  [ATTRIBUTION](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/ATTRIBUTION.md):
  Apache-2.0, with contributor and NVIDIA derivative-work notices. Preserve license,
  notice, origin and modification attribution for any redistribution; audit the
  actual resolved Rust closure independently. Apache-2.0 is already allowed by
  Newell's license policy; this proposes no Newell relicensing. README/library
  origin comments do not all name the same upstream commit; attribution records
  the later USD BRep v1.0.1 origin. Record that documentation discrepancy, not an
  inferred provenance guarantee.
- [README](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/README.md)
  separates the native library from `solidcore-ffi`, `difftest` and `harness/cpp`.
  Those optional comparison tools do not make C++ a dependency of the selected
  native kernel. Exclude them, external corpora, STEP/SSI siblings and WebAssembly
  targets from the experiment. Native desktop is Newell's acceptance target.

### Source-confirmed API inventory, not demonstrated capability

Use fully qualified modules below relative to `solidcore`; methods exist at this
revision, but none was executed in this stage.

| Adapter need | Actual public source surface | Unproven integration/gap |
|---|---|---|
| Seed/support/profile | [`primitive::api_primitives`](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/primitive/api_primitives.rs): `create_box`, `create_plane`, `create_planar_circle`; [`primitive::api_curves`](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/primitive/api_curves.rs): `create_line_segment`, `create_circle`, `create_analytic_circle`; [`primitive::api_trim_ops`](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/primitive/api_trim_ops.rs): `create_planar_faces`, `trim_surface_with_3d_curves` | Hole orientation, exact cubic data ingestion, supporting occurrence extraction and conservative eligibility require adapter proof; `create_curve` is not assumed to mean control-point construction |
| Linear solids/sheets | [`sweep::api_brep`](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/sweep/api_brep.rs): `create_linear_sweep(&mut Kernel, &[Option<&Curve>], Vec3, f64, bool) -> Result<Brep>` | Caps/holes/open chains, distinct outputs and provenance need tests; native API is not a Newell ExtrudeSpec interpreter |
| Exact varying ends | [`sweep::api_surfaces`](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/sweep/api_surfaces.rs): `create_extrude`, `create_ruled_surface`; trim APIs above | No inspected one-call Newell finite UpTo/ThroughAll implementation. Reproduce existing bounded station/domain logic through candidate evaluation, exact trims and predicates; never use a box/mesh as final target |
| Booleans | [`boolean::api_brep`](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/boolean/api_brep.rs): `boolean`, `boolean_union`, `boolean_difference`, `boolean_2d`, `boolean_with_options`, `tessellate_boundaries` | `BooleanOutcome` holds result `Brep` and optional other, not Newell's connected output vector. `Boolean2dOutcome::Unassigned` is not proof of empty geometry. Separate connected material/sheet components, no-op and full removal explicitly |
| Validity/measure | [`topology::integrity::check`](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/topology/integrity.rs); [`query::api_queries`](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/query/api_queries.rs): `brep_is_manifold_solid`, `face_get_boundary_loops`, `face_classify_uv`, `brep_material_census`; [`mass::integrator::properties`](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/mass/integrator/properties.rs): `brep_volume`, `brep_area`, `face_area` | Mass-property queries are fallible, tolerance-dependent computations. Native checks plus independent geometry/incidence checks are required |
| Lineage | [`topology::topology`](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/topology/topology.rs): `TrackTopologyChanges`, `TopologyChangeLists`, callbacks; [`topology::Brep`](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/topology.rs): arenas/attributes | Notifications are candidate evidence for lineage, not a persistent naming guarantee. Copies, splits, merges and removed occurrences need explicit proof |
| Meshing | [`query::tess`](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/query/tess.rs): `TessSrfCache`, `SurfaceTessDriver`, `build_tree`, `build_tree_with_subdivision`, trimmed/poly cache access; boundary tessellation above | A complete public B-Rep-to-NewellMesh bridge is **not established**. Scoped Q1 must identify callable face triangulation, trimmed holes, orientation, normals and back references. Boundary polylines alone are insufficient |

The sibling [nurbs-grid-mesh](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/nurbs-grid-mesh/src/lib.rs)
exports `mesh_trimmed`, `triangulate_planar` and `deviation`. It needs supplied
surface/trim data, is not the native kernel's dependency, and describes measured
deviation rather than a general proof. It is **not authorized as a second mesher
dependency in this plan**. If native meshing cannot meet Q1, report this as a
separately estimated alternative, then stop dependent work. Likewise do not add
`solidcore-ssi`/`ssi` to repair a failing Boolean.

The upstream README calls integration incomplete and the API experimental.
Port percentages, C++ differential totals, ancestry and safe-Rust declarations
are upstream claims/context, not operational maturity or Newell acceptance.
Exact analytic/NURBS representation does not certify intersection completeness,
tangency, overlap classification or topology. A short experiment cannot establish
long-term API stability, maintenance continuity or all geometric inputs.

### 3.1 Effective tolerance configuration and Q1 gate (C3)

Use identity length conversion: **one native coordinate unit = one mm**; native
areas/volumes therefore convert with factors 1 and 1. Newell angles remain radians;
convert only degree-valued API fields using `degrees=radians*180/pi`. UI rounding
and display deflection never set modeling tolerance. For each job compute S from
the exact source coordinates and spans involved, and freeze
`eps=max(1e-7,1e-10*S)` before native construction. Relative coefficient 1e-10 and
angular threshold 1e-12 rad stay Newell-owned; do not infer a native relative
tolerance control that has not been established.

| Layer/control | Required mapping and evidence |
|---|---|
| `KernelOptions::tolerances` | Set `zone_tol_3d=eps/2`, `xsect_tol_3d=eps`, `approx_tol_3d=eps/2`, `ang_tol_deg=1e-12*180/pi`, `tol_gap_gain=1.0`, truthful `model_size_estimate=max source span`, and fixed `large_small_size_ratio=1000`. Half-eps zones reserve room for a two-entity summed intersection tolerance <=eps. Record requested and effective values; these settings alone do not prove all call sites honor them |
| Context | `Kernel::new` constructs `Context::default()` separately. Initialize its model size from the same truthful span and its ratio to 1000 through public setters; read `this_zone_tol_3d()`. `geom::tol::context_zone_tol_3d` can use that independently derived value, with approximation/intersection values half/twice it. If a required call reads a coarser context value, Q1 must establish an existing explicit-tolerance route or report the gap; never falsify model size merely to select a smaller tolerance bucket |
| B-Rep/entities | Initialize an empty B-Rep's tolerance before face/edge construction where the API permits; use <=eps/2 zones for paths that sum operand zones. Inspect resulting face/edge/vertex tolerances and curve/surface ownership values after construction, copy, sweep and Boolean. A tighter label applied after a coarse operation cannot undo its decisions and is not proof; record cascades and geometric gaps instead of assuming a B-Rep setter updates all boundaries |
| Planar-face wrapper | `primitive::api_trim_ops::create_planar_faces` passes 0.01, but `topology::brep::p8::create_planar_faces_with_3d_curves` ignores its `tol_3d` argument and reads `b.tolerance()`. Q1 may use the public lower routine on a correctly initialized empty B-Rep with `FaceBuildEnv`, or retain the wrapper only after proving its effective B-Rep/entity tolerance meets this policy |
| Trim wrapper | `trim_surface_with_3d_curves` forwards fixed 0.01 to trimming. The proposed existing alternative is public `topology::trimming_tools::trim_surface_with_model_space_curves`, using `TrimInput`, explicit `tol_3d<=eps`, an initialized B-Rep and notification environment. Signature availability is source-confirmed; completeness of this route and its internal tolerance use remain Q1 evidence, not an implemented adapter |
| Other local overrides | Inventory the actual selected construction/sweep/Boolean/classification paths for context/entity defaults, gap gains, `USE_DEFAULT` and constant angular/linear tests (including `geom::tol::ang_tol_rad`, which is not the configurable kernel field). Explicit Newell preflight retains M4 angle/degeneracy policy. If a native decision can still erase or invent required geometry at a coarser tolerance, preflight alone does not cure it: report a gap unless one existing public route satisfies the contract |

Pinned primary evidence:
[kernel initialization/defaults](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/kernel.rs),
[context setters/getters](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/geom/context.rs),
[tolerance dispatch](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/geom/tol.rs),
[planar-face implementation](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/topology/brep/p8.rs),
[trim wrapper](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/primitive/api_trim_ops.rs),
[explicit trim route](https://github.com/tangency-labs/solidcore/blob/d4cb45e1f426a45467d8108ec076f66286f2c348/crates/solidcore/src/topology/trimming_tools.rs).

Q1 must record a callable source-to-output path for planar and trimmed profiles,
its effective tolerance ledger, and F19-T's actual baseline/candidate results
before progressing. Inspection of this route uses **one** of the four existing
missing-capability investigations, at most 90 minutes including the alternative
route, within Q1's 12h cap. Other missing capabilities still consume the remaining
investigations. No public route established within the cap means a concrete
tolerance/API gap and stop of dependent qualification, not permission to edit
SolidCore, raise eps, rescale units secretly or claim a native failure without a
reproducer. The mapped kernel values and fixed flag configuration are used for
all subsequent tests; per-job eps follows the declared policy, not fixture tuning.

## 4. Finite qualification matrix

### Shared measurement rules

Each matrix row is a finite test group; run **all assertions of each named fixture**
unless a specific tuple subset is stated. Assertions tied solely to SolidCore's
native face partition/count/ordinal must be translated into the same geometric
and semantic coverage, not imposed as cross-kernel numbering equality; record
each such translation. Port the fixture recipe and independent
oracle, never its SolidCore handles or precomputed native result. Existing fixture
functions may need data-only extraction within §7's allowed test sections; do not
alter their assertions. Record every subcase with its own recipe/settings and
result. Run baseline and candidate from the same checkout, machine and fixture
data; preserve failures on either side. Candidate success cannot be inferred from
a baseline passing diagnostic test.

References used below:

- **K** = `crates/newell-kernel/tests/it/extrude_feasibility.rs`.
- **T/S** = unit tests in kernel `src/termination.rs` / `src/surface.rs`.
- **W** = unit tests in app `src/geometry_worker.rs`.
- **A** = unit tests in app `src/app.rs`; **E** = `src/extrude_session.rs`.
- **G** = same canonical mm/radian policy as M4 §6.1:
  `eps=max(1e-7,1e-10*S)`, S=max absolute coordinate/span involved; angular
  tolerance 1e-12 rad. Scalar volume/area error <=`max(1e-6,abs(expected)*1e-6)`
  in mm³/mm², or a fixture's tighter explicit bound. Preserve tighter station,
  residual and topology assertions in fixtures. Require finite exact geometry,
  expected components, hole/boundary counts, oriented incidence and vertex links;
  solids closed/manifold, sheets manifold with intended free boundaries and no
  fake caps, overlaps or T-junctions. Compare to analytic integrals/interval
  oracle and source curves, not just kernel agreement. No native face-count
  equality where a different valid face partition represents the same shape.
- **D** = document/registry/feature/entity/generation/reference/scene invariants
  in §§5–6; success exactly one commit/revision/event, failure zero. Full removal
  is a successful zero-output feature; unchanged Boolean is the existing typed
  failure. Repeat rejection 10 times in fresh candidate scratch, compare typed
  outcome and unchanged accepted state; diagnostic text need not match upstream.
- **M** = mesh contract in §5.2, checked at deflections 0.05 and 0.01 mm.
- **B/SW/LC** below name adapter work, not additional dependencies:
  B=Boolean/component conversion; SW=exact profile/sweep/station/trim adapter;
  LC=existing worker/model/session lifecycle with candidate-native operations.

| ID / M4 requirement and reusable fixture | SolidCore baseline behavior | SolidCore API / experimental adapter | Measurement and pass threshold | Failure consequence |
|---|---|---|---|---|
| F01 M4.6 holed Solid profile: K `holed_planar_profile_has_exact_volume_and_valid_manifold` | 10×6 minus 2×2 hole, depth 5: V=280; oblique d=(.6,0,.8): V=224 | Planar-face/trim APIs + linear sweep/SW | G+M; hole remains through, closed solid; verify exact loops and both volumes | Mandatory profile blocker |
| F02 M4.6 curved closed profiles: K `circular_profile_uses_a_true_circle_not_polygon_chords`, `nonconic_cubic_profile_and_extrusion_match_independent_integrals` | r=2, depth3: V=12π; specified cubic/chord area7.5, depth4: V=30 | Circle/NURBS construction, planar faces, sweep/SW | G+M; inspect exact analytic/rational support/control data, evaluate same curve; no polygon substitution | Mandatory curved-profile blocker |
| F03 M4.6 Surface face/wire/edge and holes: K `real_face_wire_and_edge_surface_sweeps_are_uncapped_exact_sheets`; S `face_sweep_keeps_each_hole_wire_identity_and_connected_mesh` | True support-qualified face, wire and open edge yield uncapped sheets; inner-wire identities survive | Boundary-loop queries, un-capped linear sweep/SW | G+D+M; exact expected area, connected outputs and free boundaries from fixtures; shared edge on two supports remains two occurrences | Mandatory Surface/profile-reference blocker |
| F04 M4.6 curved open sheets: K `straight_and_circular_sweeps_supply_real_uncapped_faces`, `nonconic_sheet_overlap_retains_exact_support_and_uncapped_boundaries` | Length10×5 gives area50; r2×3 cylinder area12π/two rims; cubic station Add/Remove areas4L/L | Sweep/ruled/trim APIs + B/SW | G+M; L from independent bounded integration in K, retain cubic and circular boundaries | Mandatory curved Surface blocker |
| F05 M4.9 Solid Add/Remove/scopes/multiple/empty: K `body_results_distinguish_split_full_removal_and_unchanged_targets` | Slab cuts cube into V400+400; full removal0 outputs; miss rejects; union1500 changes only selected target | `boolean*`, material census, connected-component B | G+D+M on each nonempty output; compare changed target set, total measure, components and retained unaffected geometry | Mandatory Boolean/output blocker |
| F06 M4.9 planar Surface Add/Remove: K `planar_sheet_union_difference_weld_and_split_real_uncapped_faces`, `sheet_booleans_preserve_existing_holes_and_reject_transverse_area_removal` | Union40, cut10; split two sheets area10 each; full removal empty; holes/transverse cases retain specified behavior | `boolean_2d` or exact trim B; classify Unassigned separately | G+D+M; boundary graph/vertex links; transverse zero-area cut must not delete material, unchanged result handled as fixture specifies | Mandatory Surface Boolean blocker |
| F07 M4.9 supported curved domain normalization: S `common_support_proof_handles_another_source_knot_refinement_and_degree_elevation`, `production_common_domain_keeps_cubic_boundaries_for_union_and_split` | Affine normalization/refinement/elevation retain exact support, union/split and unique lineage | SW exact support normalization + B; do not assume generic boolean supports coincident sheets | G+D+M with original-domain correspondence; do not require nonlinear correspondence | Mandatory supported-domain blocker |
| F08 M4.6/M4.9 crossing/pinch: W `curved_crossing_and_pinched_scopes_use_real_preparation_identity_and_atomic_rejection` | Both All/explicit, supported crossing and endpoint pinches succeed; invalid interior pinch rejects atomically | SW piecewise generated domains + B + LC | Existing independent measures/component/identity assertions, G+D+M; 10 repeated invalid cases | Mandatory retained prototype blocker; cannot relabel deferred |
| F09 M4.7 finite planar UpTo: K `finite_up_to_preserves_profile_and_retracts_clearance`; T `tilted_target_keeps_its_plane_and_rejects_an_interior_hole` | V24 then21 at .5 clearance; wrong finite trim/collapsed or parallel/backward target rejects; no hit through hole | Plane station + exact finite trim SW; no named one-call API established | G+D+M; complete projected region/chain coverage, same clearance direction; no vertex-only coverage test | Mandatory termination blocker |
| F10 M4.7 curved profile/tilted end: K `nonconic_tilted_cap_measures_actual_brep_and_is_manifold`; T `cubic_open_sheet_terminates_at_a_tilted_trim_with_exact_lifted_boundary` | Cubic solid at z=5+.1x gives V39.66; open sheet has exact lifted boundary | Ruled/extrude surfaces, trim SW | G+M; exact cap plane and original/lifted curve consistency, fixture trim rejection | Mandatory bounded tilted-end blocker |
| F11 M4.7 ThroughAll: K `through_all_uses_actual_multiple_hits_and_ignores_a_distant_miss`; W `partial_curved_through_all_real_worker_replays_both_directions_and_rolls_back_suffix` | Multi-hit station9.000001 (fixture origin); distant miss ignored; supported partial ruled/cylinder both directions and suffix rollback | Exact finite intersection/extrema SW + LC | G+D+M; station abs error<1e-9 in K; forward no-hit typed failure; hidden active targets count, future/preview targets do not; scope-independent hit set | Mandatory supported finite-hit blocker; no general trimmed target requirement |
| F12 M4.8 oblique/symmetry/starting plane: K `symmetric_oblique_sweep_reflects_stations_about_one_displaced_section` | Main/second V24 each, union48; start plane station2.5 | Sweep/transform/union SW+B | G+M, station error<1e-10; symmetry about displaced section, not original support | Mandatory modifier blocker |
| F13 M4.8 selected active branch crossings: W `modifier_crossings`, exact tuples below | Analytic interval oracle determines volumes/areas/components; collapsed endpoints/unchanged Add typed failure | SW+B+LC for all six Solid/Surface New/Add/Remove paths | G+D+M, all 54 unique preparations below, plus specified rejection repeats; inactive cache values cannot affect recipe; no copying the full 7,770 product | Mandatory branch/recipe blocker |
| F14 M4.5/M4.10 Formula/preview/OK/Cancel: A `real_face_formula_preview_cancel_and_exactly_once_confirmation`, `cancel_after_real_preview_preserves_unrelated_accepted_metadata_and_formula_source` | Exact `#Depth := 3.25mm` source retained; real preview; Cancel preserves unrelated rename/latest committed scene; OK once | LC with actual candidate geometry, same Formula/model APIs | D+M; compare full accepted document/registry, feature counts, events and source string; double OK/late preview cannot commit twice | Mandatory application/transaction blocker |
| F15 M4.9/M4.10 Properties/replay/reference loss: A `properties_replays_suffix_and_remaps_a_unique_cap_atomically`, `properties_can_reopen_zero_output_remove_and_cancel_is_read_only`; W `cap_edge_replay_keeps_unique_identities_and_lost_suffix_is_atomic` | Unique cap/wire/edge remap, stable operation/output identities; zero-output editing; lost suffix rolls back | TrackTopologyChanges/explicit construction lineage + LC | D+M; resolve all downstream references to proven source occurrence; deliberately ambiguous split must reject, never nearest face | Mandatory stable-reference blocker |
| F16 M4.10 stale results and worker errors: E `every_job_identity_field_rejects_stale_errors_and_transport_failure_consumes_ok_once`, `newer_ready_wins_and_late_ready_is_released_without_replacing_the_preview`; A publication retry tests named in §6 | Every identity field checked, one delivery retry then suspension/rearm; no repeated model work | LC, SolidError→Newell error conversion and existing worker tokens | D; run old success AND old failure arrivals, transport death/restart, document replacement and recovery; exact token release accounting | Mandatory authority/recovery blocker |
| F17 M4.10 cancellation/service/memory: W `cancellation_during_native_boolean_returns_promptly_rejects_late_work_and_recovers` (128 operands), plus §6 cycle schedule | Cooperative native operand/stage boundary, preserved commit, same worker recovers; historical timings are not all-phase bounds | LC, worker-local Kernel/Brep, checks between native calls | D; instrument phases, cancel ack and actual termination/drop independently; thresholds in §6; no forced thread kill | Mandatory lifecycle gate; opaque slow call is a concrete gap |
| F18 M4.6 rendering/picking: K `actual_cube_faces_keep_mesh_ordinals_frames_and_historical_geometry`; T `renderer_conversion_rejects_lost_body_and_sheet_detail_without_mutating_sources` | Grouped mesh, normals/edge data and semantic face mapping; f32 detail loss rejects | Native tess cache/face-to-mesh bridge | M+D; compare per-kernel semantic maps, never cross-kernel ordinal equality; source unchanged on failed conversion | Mandatory meshing/selection blocker |
| F19 all retained workflows, numerical policy | Baseline tolerance and deterministic outcomes; narrow planar split expected from existing supported class, not yet executed | §3.1 effective tolerance mapping, all above APIs | Repeat F01,F02,F05,F09 at uniform scales .1,1,10 and translations (0,0,0)/(1e6,-1e6,1e6), same fixture-specific topology/measure tolerances scaled by dimension and coordinate eps; 10 fresh contexts each; ambiguous/unrepresentable cases reject deterministically. Also run finite F19-T below | Mandatory numeric/reproducibility gap; do not enlarge tolerance to pass |
| F20 M4.6/M4.9 invalid inputs and scope preflight: existing kernel `box_solid`/`tessellate` validation and model `EntityGeometry::resolve_profile`, plus W insertion/scope preflight | Nonfinite/nonpositive dimensions or deflection, missing/invalid profile and stale supporting occurrence reject; duplicate merge operands are deduplicated while references are still validated | ACL input checks before native calls, same model preflight + LC | Exactly eight invalid cases: NaN box dimension, zero box dimension, infinite deflection, zero deflection, absent profile, open Edge used as Solid profile, stale support generation, absent explicit merge entity. Each repeats 10 times with D and no native/mesh publication. One valid F05 scope repeated twice in its operand list must give the same geometric result/one commit as the deduplicated scope | Mandatory input/atomicity blocker; no panic or success with fabricated geometry |
| O01 optional M4.D1 | S `nonlinear_common_support_records_an_unfinished_required_case` safely rejects; Add/Remove success not established | Candidate correspondence/Boolean probe only | Existing exact C((s+s²)/2) fixture; record actual geometry, correspondence and rejection invariants; no new algorithm | Comparative evidence only; never migration prerequisite |
| O02 optional M4.D2 | Specified trim-hole target in M4 validation; general required operations unestablished | Candidate finite trim/intersection API probe only | Translate only the already specified target recipe if constructible within optional cap; actual trim incl. hole, no bbox hit; otherwise “not constructed”, not pass | Comparative evidence only; never migration prerequisite |

F13 selects the existing worker loop by `(family,variant,side)` indices, retaining
its exact recipe constructor and independent interval oracle. Family 0=Blind,
1=UpTo, 2=ThroughAll. Pick exactly these nine triples:

`(0,0,0), (0,7,1), (0,24,2), (0,48,3), (0,72,4), (0,96,5), (1,1,6), (2,7,1), (0,8,6)`.

For each triple run Solid and Surface × New/All, Add/explicit, Remove/explicit
=54 unique preparations per kernel. This samples positive/negative/UpTo/zero start, positive/
negative/UpTo/zero end, real Along/reversal, symmetry, both Blind second-side
directions, second UpTo and nested clearance/replacement, plus Invert and UpTo
clearance. F05/F08 separately test All versus explicit Boolean scope; F09 tests
UpTo clearance off/on, F11 tests both ThroughAll directions. Fixture indices are
part of the plan: changes to loop ordering require translating these same semantic
recipes in the dossier, not choosing easier cases. The unchanged full baseline
regressions remain required workspace checks, but need not be duplicated for
SolidCore beyond this finite set.

The added `(0,8,6)` has Blind start_kind=2, start=-2, and second nested UpTo
endpoint=-2. All six geometry/result combinations must reject **before commit**,
including New. Preserve the existing diagnostic categories:
`NewellKernelError::InvalidParameter` with parameter
`"target does not lie beyond the entire start section"` for Solid and
`"target is not beyond the displaced start"` for Surface. Translate native errors
at the ACL where needed; this is Newell's existing domain diagnostic, not a demand
for native SolidError text equality. For each of these six recipes run the first
attempt plus nine fresh-scratch repeats: 60 collapse rejections, 54 additional
attempts beyond the 54 unique preparations, or **108 F13 attempts per kernel**
before any other §4 rejection repeats. Each preserves the full accepted document,
registry, identities, scene bindings and editable draft; revision/events/commits
remain zero and no preview replaces committed geometry.

**F19-T, one finite tolerance witness:** reuse K's `planar_sheet` helper and the
planar Surface Remove construction in
`sheet_booleans_preserve_existing_holes_and_reject_transverse_area_removal`.
Target is the exact sheet x=[0,10], z=[0,10], y=0; cutter is x=[5,5+g], z=[0,10],
y=0 with **g=4e-6 mm**. Both are constructed from their genuine planar supports
as in the helper. Here S=10 and eps=1e-7: the slot is 40eps wide, below the native
default zone 1e-5 and the wrapper constant 0.01. Expected exact Remove result is
two uncapped manifold connected sheets, areas 50 and `50-10g=49.99996 mm²`,
total99.99996, each with its rectangular free boundary and no artificial bridge.
Derive the oracle from these axis intervals, not native face numbering or agreement
between kernels. Require area error<=1e-7 mm² for each component/total, exact
component count2, retained boundary separation g within eps, unchanged source,
and G+M (without cross-kernel face/triangle equality). A no-op, weld across the
slot, silent widening, fabricated full removal or rejection of this required
supported geometry does not pass. Run 10 fresh contexts per kernel at the origin
only, with the same frozen configuration; Q1's runs are reused in Q2 if unchanged.
No new curve family, advanced trim class or Cartesian product is added. These
are **proposed expected results**, not reported baseline/candidate passes. If the
baseline or candidate fails, retain the reproducer and typed outcome and apply
the gap/stop rules; do not weaken the topology or tolerance criterion.

## 5. Critical geometric integration contracts

### 5.1 Stable references and outputs

The model remains the source of EntityId, FeatureId, parameter identity and
GeometryGeneration. Candidate native IDs remain inside a worker-local map. Store
proven construction roles (Base, StartCap, EndCap, Wall with source wire/edge),
oriented wire/edge occurrences and unique source-component lineage. Capture native
split/merge/copy notifications when available; test that they describe actual
geometry. Attribute propagation alone is not proof. A valid identity may survive
different face partitioning, but only a unique semantic remap may satisfy a
downstream reference. Otherwise reject the entire edit.

F03/F05/F08/F15 must prove: unaffected IDs and historical geometry retained;
replacement outputs consume only actual changed targets; unique surviving output
IDs retained on Properties edit; new/split components allocated only in candidate
state; no IDs consumed by failed work. Alter native insertion/enumeration order
in two fixture constructions and re-tessellate at both M deflections. Repeat replay
10 times. Face/wire/edge references must still name the original producer,
supporting face and oriented occurrence; a stale generation or ambiguous seam
rejects. Stable geometry under a different native index is not enough without
lineage. No nearest-point/name/normal/area heuristic is an accepted remap.

### 5.2 `NewellMesh` and scene contract (M)

Q1 must locate and exercise native trimmed face triangulation, not just list a
tessellation module. Build the adapter from source face identity through native
poly/cache records to Newell's contiguous face ranges. Feature edges come from
actual B-Rep edges, with shared boundaries sampled consistently, not triangle
edges. No mesh feeds an exact Boolean or finite-target decision.

For F01/F02/F03/F05/F10/F18, at 0.05 and 0.01 mm, require finite arrays, matched
normal/position counts, in-range indices divisible by three, monotone complete
face ranges ending at index length, valid feature-edge offsets, no unintended
zero-area/reversed triangles, and finite near-unit normals (length error<=1e-5).
Require triangle winding agrees with oriented surface normal (positive dot away
from singularities), outward solid orientation and consistent sheet orientation.
Check holes remain empty, adjacent face boundaries meet within eps, and sampled
edge chord deviation<=requested deflection. Each represented face maps to its
own semantic descriptor and picking result; triangle count/layout may differ.

Check deviation independently at triangle edge midpoints, centroid and a fixed
5×5 barycentric grid against exact source surfaces/trims, plus extrema available
for analytic fixtures. Require deviation<=deflection+eps and refinement cannot
increase maximum measured deviation by more than eps. Label this measured mesh
evidence, not a certified global bound for arbitrary rational faces. For planar/
circle/cylinder fixtures use analytic plane/radial distance as well. Preserve
existing f64→f32 rejection tests; never inflate tolerance to hide lost detail.
Render the accepted fixture scene through unchanged Newell buffers; run the
existing applicable explicit GPU checks and retain a finite screenshot/pick trace
for solid, holed, curved sheet, split result and selected output. No identical
face numbering or triangles between kernels are required.

## 6. Ownership, application lifecycle and performance protocol

Construct **one Kernel and its Breps on the existing worker thread**, retain/drop
there, and send only Newell DTOs. Compile-check Send on actual message DTOs;
neither require native Send/Sync nor add unsafe implementations. Inspect callbacks,
cache ownership/copy semantics, context-global state, panic/abort paths and actual
dependency build scripts. Failure recovery must discard poisoned scratch and
replay accepted recipes in a fresh context; never continue a partly changed arena.

Use existing GeometryJobId fields (worker epoch, document, session, base/draft
revision, serial, parameter epoch, source generation), captured insertion anchor,
token association and finalized manifest preflight. Test each stale field on
success/error replies. Preview never mutates accepted state. Cancel leaves the
latest committed scene including unrelated accepted metadata. An explicit retry
uses a fresh lifetime; two confirmations cannot accept the same prepared authority.
Properties on zero outputs remains addressable by FeatureId. Publication tests:
A `local_publication_retries_without_a_document_event_and_replacement_invalidates_every_session`
and `failed_local_publication_suspends_after_one_retry_then_rearms_without_events`.
Inject one then two publication/upload failures: one +100ms delivery retry, then
suspension/rearm, with zero extra prepares/commits and no old mesh under new IDs.

Cancellation is cooperative: poll before/after native calls and between replay,
operand, mesh-face and conversion stages. Q1 inventories native progress hooks;
Q3 times the actual opaque call remainder. UI flag acknowledgement is not compute
termination. Test cancel during replay, sweep, Boolean, meshing, conversion and
cleanup; no sleeps simulating computation. If a phase finishes too early to
inject, mark that trial invalid and repeat at most twice with the same finite
128-operand fixture; do not invent a larger workload. Submit the same ordinary
ParameterDefinitions prepare and another window/session request immediately after
cancel; measure queue-to-start and queue-to-ready separately. No GUI-thread join.
Recovery from transport death must preserve the committed document and allow an
explicit retry with new worker epoch.

### Timing and memory procedure

Q3 runs release builds, Rust 1.88, identical fixtures/OS/power settings, one worker,
no nested numerical pool. Record compiler/target, CPU/RAM/GPU/OS, native options,
dependency source and lock resolution, and default versus experimental features.
Build each variant in the same checkout with separate build-output directories
only (these are not separate execution checkouts). Alternate baseline/candidate
blocks to limit thermal bias. Use 20 untimed warmups then **500 measured jobs per
fixture** for F01 normal, F02 cubic, F05 split, F06 sheet split, F10 tilted cubic,
F11 supported curved ThroughAll and F14 preview→OK. Complete correctness first;
do not benchmark a missing/rejecting mandatory capability as if it succeeded.

Report p50/p95/p99/max for dispatch, queue, geometry/replay, meshing, finalization,
publication and input-to-visible result; nearest-rank quantiles and raw ordered
samples. Instrument cancel acknowledgement, actual termination, obsolete release,
maximum indivisible call, and next-session service independently. Include allocator
live/peak bytes, copies, native arena/cache counts, retained token/generation
counts and process peak RSS. RSS alone cannot prove release.

For memory, use the existing preparation/acceptance and Properties/parameter-edit
APIs on F14's accepted Formula-backed Extrude, initialized at
`#Depth := 3.25mm`. Initial document construction and bootstrap are recorded
separately and excluded from the following counters. Perform **20 complete warmup
iterations**, then **1,000 complete measured iterations**, each with four actions:

1. Let a be the currently accepted depth and b the other member of
   {3.25mm,4.25mm}. Edit to b and accept one real prepared result.
2. Inverse-edit to a and accept one real prepared result.
3. Reapply b and accept one real prepared result. It becomes the next iteration's a,
   so every accepted edit changes geometry rather than repeating a no-op value.
4. Prepare and display a real 3.75mm preview, then Cancel without accepting it;
   drain/release its token and preserve the latest committed b scene and Formula.

Use one explicit OK preparation per accepted edit (no extra preview preparations
for actions1–3), and one preview preparation for action4. Each iteration therefore
has four actions/preparations, **three commits/revision increments/events and one
cancelled draft with zero commits/events**. Warmups total80 actions/preparations,
60 commits and20 cancellations; measurements total4,000 actions/preparations,
3,000 commits and1,000 cancellations; combined totals4,080 preparations,
3,060 commits and1,020 cancellations. Count only geometry document-change events
in these totals; record lifecycle/release messages separately. Verify counters
per kernel, not inferred totals. Preserve operation/entity/parameter identity
where unique and verify the latest committed scene after every Cancel.

After draining warmup work, retain that committed native generation deliberately
through all measured iterations while releasing each superseded preview/current
generation as appropriate. Record warm settled memory/current-only owners first,
then the retained-old-plus-current plateau during measurement; after iteration
1,000 release the deliberate old generation and drain controls. Compare final
settled live memory and generation counts against the equivalent warm
current-only checkpoint (both even iteration counts restore the initial depth),
using the existing <=5% growth and baseline-generation-count requirements. Report
allocator peaks/copies separately. A missing allocation profiler is unverified
memory evidence, not permission to infer release from RSS alone.

This is **edit/inverse-edit/reapply/Cancel native-retention evidence**, reusing
the mechanism of worker `revision6_completion_worker_native_retention_1000_cycles`.
It does not implement or qualify an Undo journal, Undo/Redo commands, or their
retention policy. Those remain outside this experiment. F17's separate in-flight
cancellation trial still checks actual computation termination; a cancelled
ready-preview draft here is not evidence of native preemption.

Applicable existing budgets: M4's real Boolean cancellation test asserts reducer
latency<100ms but offers no universal geometry completion deadline. Preserve that
regression. Reuse M5 §10's stronger service goals as **proposed qualification
thresholds for this finite corpus**, not a claim that M4 already guarantees them:
ack<=16ms; actual obsolete-work termination/release<=100ms on accepted jobs;
settled live-memory growth<=5% after warmup and retained generations return to
baseline. On reference A, input p95<=4ms/p99<=8ms, scene navigation frame
p95<=16.7ms; B input p95<=8ms/frame p95<=33.3ms. Local-result goals A<=100ms /
B<=150ms apply to the F14 local preview edit; slow full Boolean work remains
separately reported. Do not apply Sketch S1/S2/S3/C1 solver/count budgets to a
small Extrude corpus or claim Sketch qualification from it.

Additional proposed relative limits: candidate p95 whole-job latency<=1.25×
baseline per timed fixture, peak live bytes<=1.25×baseline per fixture, no
unbounded retained growth, and no service starvation. If baseline is below clock
resolution, aggregate 100 identical jobs and divide before comparing. A baseline
that fails a correctness/budget gate is recorded as such, not used to waive the
candidate gate. These new comparative limits require acceptance with this plan.

Reference A remains M1-class/8GiB/1920×1080 and B Pi5/8GiB/64-bit Linux/supported
driver as in M5; an available M4 Mac is host characterization. Record unavailable
reference devices as **unverified**. Complete available-host evidence; do not
substitute it for reference qualification. No browser/Wasm acceptance is needed.
Retain the two pending human/device checks from §2; synthetic input/GPU results
do not supply human confirmation or prove long-term stability.

## 7. Ordered packages, exact future file scope and hard bounds

All budgets below are **proposed execution budgets**, inclusive of inspection,
coding, corrective attempts, execution and reporting. They are not performance
measurements. One person, sequentially in this checkout on main; maximum **50
person-hours across Q0–Q4**, at most seven working days of active investigation.
No clock runs while waiting for unavailable hardware, but missing hardware never
extends code investigation. A cap expires into a concrete gap report. No package
borrows unused time to extend a failing capability without human scope authority.

| Package / cap | Work and permitted progression | Evidence required before next package |
|---|---|---|
| Q0 authority/source/compatibility, 4h | Apply only human-accepted §8 text/ADR status; record candidate source, Rust closure, licenses/notices, advisories, native targets and Rust1.88 compile; record fresh baseline checks | Explicit exception authority; no FFI/native build requirement; source/lock/license/MSRV evidence; baseline failures separated. Unsupported compiler/security/license constraint stops dependency work |
| Q1 minimum ACL and early blockers, 12h | Candidate seed, exact holed/cubic profiles, sweep, Boolean split/empty, mesh bridge and lineage smoke: F01,F02,F03,F05,F18; C1 key seam/tests; §3.1 tolerance/public-route ledger and F19-T. Meshing bridge work<=6h, tolerance route investigation<=90min, all within cap | Rust1.88 key allocation/exhaustion and candidate-only F05 evidence; effective tolerance route and narrow-slot witness; actual NewellMesh with holes/normals/selection mapping, one split/empty result and replayable cap+edge reference. Missing any ends dependent packages with gap assessment |
| Q2 bounded prototype geometry, 16h | Complete F04,F06–F13,F19–F20, including 54 unique F13 preparations and collapse repeats; reuse unchanged Q1 F19-T evidence; port only existing supported exact station/domain construction needed by these fixtures; classify native versus Newell adapter work | All mandatory geometry groups pass or each failure has smallest fixture/API/options/error and estimate. Do not enter costly performance work for failed capabilities |
| Q3 real worker/lifecycle/performance, 12h | F14–F17, real application handler route, meshing/render checks, cancellation/service/retention with §6's 20 warmup+1,000 measured inverse-edit iterations and exact counters, plus paired release measurements | Raw correctness/transaction traces, paired distributions, ownership accounting and explicit device limitations; no Undo/Redo qualification claim, fake finalized manifests or mock native jobs |
| Q4 decision dossier, 6h | Reconcile default/all-feature checks, maintenance and removal inventory, outcome; optional O01/O02 together<=2h within this cap | Complete row-by-row dispositions, benefit/cost analysis, remaining gaps and human gates; no migration implementation |

Missing API investigation: **at most 90 minutes per distinct missing capability,
four such investigations total**, included in the package caps. Inspect pinned
source/docs and at most one alternative existing public API route; allow at most
two adapter correction attempts for a concrete failure. Record consulted symbols,
minimal reproducer, actual error or absent entry point, required Newell work,
estimated effort and whether upstream change is needed. No upstream patch, kernel
algorithm invention, extra mesher/SSI engine, revision hop, general rational-target
implementation or “extend until green” is authorized. If more than four missing
capabilities arise, stop qualification and summarize the remaining gaps.

Freeze candidate options after Q1: first record upstream-fixes-off smoke using
§3.1's required tolerance mapping ("default" here refers only to fix flags, not
`KernelOptions::default()` or default context tolerance).
One predeclared corrected configuration may additionally enable only existing
flags `keep_operand_uv_trims`, `planar_path_uncapped_open_profiles`,
`sheet_sweep_joins_profile_wires`, `precise_properties_body_length_scale`,
`precise_properties_split_at_surface_knots`, `precise_properties_trim_base_v`,
`brep_area_between_solid_and_void`. Record both smoke results, then use that one
configuration consistently for the full candidate corpus. No per-fixture flag
tuning; if another fix is required, name the gap. Native memo setting and seed
must also be fixed in the dossier. No source modification to SolidCore.

### Exact allowed files for the proposed Executor

No other path is writable without an explicit scope revision:

| Paths | Allowed change |
|---|---|
| `ARCHITECTURE.md`, `docs/adr/0010-solidcore-qualification.md` | Only accepted §8 exception insertion and Draft→Accepted/authority record; no other accepted ADR edits |
| `crates/newell-kernel/Cargo.toml`, `crates/newell-app/Cargo.toml`, `Cargo.lock` | Non-default `solidcore-qualification` feature; optional kernel-only Git dependency on exactly the revision in §3; app forwards kernel feature/test-support; lock only necessary closure, no unrelated upgrades |
| `deny.toml` | Allow only official `https://github.com/tangency-labs/solidcore` source as necessary for the approved exact Git resolution; retain unknown-git deny and every existing license/advisory/ban rule |
| `crates/newell-kernel/src/lib.rs` | Feature-gated module/Newell-only exports and internal constructor access for existing mesh DTO; production signatures/behavior unchanged |
| `crates/newell-kernel/src/extrude.rs` | Only the `solidcore-qualification`-gated crate-private NewellShapeKey issuer and its gated local unit tests specified below; do not change existing `reserve_key`, geometry algorithms, fields or default routing |
| `crates/newell-kernel/src/solidcore_qualification.rs` | New temporary private-native adapter, source/options ledger, fixture construction and instrumented counts; no upstream type re-export |
| `crates/newell-kernel/tests/it/{main,extrude_feasibility}.rs` | Test module registration and data-only extraction/reuse; retain baseline assertions |
| `crates/newell-kernel/tests/it/solidcore_qualification.rs` | New finite F01–F13/F18–F20/O01/O02 native/mesh probes and release measurement entry |
| `crates/newell-kernel/src/{surface,termination}.rs` | Only feature/test-gated reuse of the named existing fixture recipes/oracles; no production geometry edits |
| `crates/newell-app/src/{lib,geometry_worker,app,extrude_session}.rs` | Temporary test+feature-only candidate constructor/forwarding seam, fixture/event harness access and instrumentation; same real prepare/accept/lifetime loop, no default selector or UI addition |
| `crates/newell-app/src/solidcore_qualification.rs` | New finite worker/app harness, release timing and retention trace runner; Newell DTOs only |
| `docs/validation/solidcore-qualification.md` | Single new results dossier: source/options, commands, fixture index, measurements, failures, dependency notices, costs and decision; append dispositions without rewriting M4/M5 |

Raw logs/JSON/CSV/screenshots go to `/tmp/newell-solidcore-qualification/`, with
exact command, settings, units, fixture IDs and exit codes. The dossier embeds
the decisive tables and recipes so a missing temporary log does not erase the
conclusion. Do not commit raw generated geometry or fixture snapshots. Cargo
build outputs/caches are permitted only during authorized execution, not source
scope additions. Do not copy external corpora or third-party source into Newell.
License/notice review records immutable originals in the dossier; any later
redistribution must package those notices separately under its own approved scope.

### Minimum experimental adapter and execution entry

Inside the kernel feature, define one concrete `SolidCoreQualificationGeometry`
owning `Kernel`, Breps, private shape map and construction provenance. Expose only
the finite operations required by the current `NewellGeometry` worker calls and
Newell-owned descriptors/results/meshes/errors. Reuse `NewellShapeKey` through
exactly one new crate-private associated function defined in its owning
`extrude.rs` module:
`NewellShapeKey::allocate_for_qualification(next: &mut u64) -> Result<Self, NewellKernelError>`,
compiled only with `feature="solidcore-qualification"`. The adapter owns its
counter, initialized to0. Mirror existing allocation semantics: first check
`next.checked_add(1)`; on success issue the key for the old counter and update it;
on exhaustion return `Operation` with `"geometry identity exhausted"` and leave
the counter/maps unchanged. The tuple field remains private. Existing production
`NewellGeometry::reserve_key`, counter and call sites remain unchanged.

Define local `cfg(all(test, feature="solidcore-qualification"))` unit tests in
`extrude.rs` for keys0/1, counter advancement, last issuable key `u64::MAX-1`, and
repeated rejection at `u64::MAX` without mutation. Through F05, verify the concrete
candidate owner allocates/maps its real split/empty outputs and calls no SolidCore
geometry operation. Candidate and baseline have separate local maps/counters;
equal raw key values in different owners are never interchangeable. The worker
selects its owner only at test construction and keeps all keys with that owner
through the existing job/token/manifest authority. No global key registry or
public constructor/backend trait is introduced. Do not instantiate a SolidCore
solid or arena as a key issuer. Q1 must compile the seam and forwarding path on
Rust1.88 and run these gated unit tests together with F05; the unique
`solidcore_qualification_` test prefix below includes these tests.

Convert errors at the ACL, retaining operation/context and deterministic failure
category without exposing `SolidError`.

The app test build may use a private worker-native enum containing existing
`NewellGeometry` and the experimental concrete owner, with explicit forwarding of
only methods used by `prepare`. Candidate construction exists only under
`cfg(all(test, feature="solidcore-qualification"))`; normal builds keep their
existing SolidCore construction. Both variants execute the same worker loop,
`prepare`, model finalize and app accept/publication path. Keep this small local
switch removable; no public backend trait, runtime kernel preference, serialized
backend tag or permanent multi-kernel framework. Native data never moves across
threads. All-feature non-test app builds must still select SolidCore.

Register candidate tests with unique `solidcore_qualification_` prefixes; fixture
IDs are printed in every result. Proposed execution commands after authorization:

```sh
# Q0 and final Q4: shared workflow required baseline/final checks
cargo check --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
INSTA_UPDATE=no cargo nextest run --workspace --all-features --locked
cargo test --workspace --doc --locked
cargo +1.88.0 check --workspace --all-targets --all-features --locked
cargo deny --all-features check

# Q1/Q2 finite tests (same test entry runs both kernels and prints row IDs)
INSTA_UPDATE=no cargo test -p newell-kernel --features solidcore-qualification,test-support --locked solidcore_qualification_ -- --nocapture

# Q3 actual app/worker tests and release measurements
INSTA_UPDATE=no cargo test -p newell-app --features solidcore-qualification --locked solidcore_qualification_ -- --nocapture
INSTA_UPDATE=no cargo +1.88.0 test -p newell-kernel --release --features solidcore-qualification,test-support --locked solidcore_qualification_measure -- --nocapture --test-threads=1
INSTA_UPDATE=no cargo +1.88.0 test -p newell-app --release --features solidcore-qualification --locked solidcore_qualification_measure -- --nocapture --test-threads=1
cargo test -p newell-render --locked -- --ignored
```

Dependency resolution/lock update occurs once in Q0 after authorization, with
diff review; subsequent commands use `--locked`. Do not install tools without
scope authority: use shared-workflow Cargo fallback if nextest unavailable;
missing Rust1.88/audit/device tools are explicit unverified gates. Two compilation
repair attempts maximum under shared workflow, then stop with concrete errors.
No snapshot updates, assertion weakening, ignored failures or mass formatting of
dirty inputs. Baseline failures stay separate from candidate regressions.

## 8. Precise proposed architecture exception

Only after explicit human acceptance, append the following as Architecture §5.3
and accept Draft ADR 0010; retain §§5/5.1/5.2 and all accepted ADRs verbatim:

> **5.3 Isolated SolidCore qualification (temporary experimental exception)**
>
> SolidCore remains Newell's exclusive production geometric kernel and default
> application route. Solely for the finite qualification in
> `docs/plans/solidcore-qualification.md` revision 2, permit SolidCore revision
> `d4cb45e1f426a45467d8108ec076f66286f2c348` as an optional, non-default dependency
> declared only in `crates/newell-kernel/Cargo.toml`. This supersedes §5's kernel
> exclusivity, §5.1's SolidCore-specific dependency restriction and ADR 0001's
> single-kernel/no-fallback decision only for that isolated experimental path;
> ADR 0009 supplies no broader authority. It authorizes neither production
> fallback nor runtime kernel selection.
>
> All native SolidCore state stays inside a worker-owned kernel adapter; public
> interfaces expose only Newell-owned types and translated errors. Model/View/
> Controller ownership, Rust 1.88, safe Newell code without unsafe or FFI, native
> Rust runtime, exact-geometry and transaction correctness, dependency isolation,
> provenance, licensing/notices, advisory audits and regression controls remain
> binding. Optional C++/FFI comparison tools and sibling geometry/mesher engines
> are excluded. The default and all-feature non-test application routes retain
> SolidCore. No SolidCore source modifications are authorized.
>
> The exception ends with qualification or its hard stop. Experimental artifacts
> may remain inert for review, but further execution/expansion, pin changes or
> adoption require explicit human authority. Production replacement requires a
> separate migration plan, architectural decision, maintenance ownership,
> applicable qualification and human acceptance; this exception does not grant
> them. Project licensing and accepted historical ADR text are unchanged.

The execution budget proposes source-resolution permission only for this exact
optional Git dependency and its audited Rust closure. If it requires a new
compiler, sibling engine, license constraint or upstream patch, stop and return
that specific decision rather than interpreting the exception broadly.

## 9. Decision record, benefit and stop outcomes

The Q4 dossier must contain one row per F/O group and per integration contract:
Demonstrated/pass, demonstrated/fail, upstream claim, assumption, or unverified.
Attach minimal reproduction, both kernel results/options, independent oracle,
raw measurements location, gap cost (Newell adapter versus upstream maintenance),
affected files and decision consequence. A failing diagnostic that safely rejects
a mandatory supported input is a capability failure, not acceptance.

Also inventory the SolidCore-specific custom code/patches potentially removed,
SolidCore-specific code introduced, notice/dependency obligations, release/API
change exposure and supported native targets. Do not infer lower maintenance
from smaller dependency count or the upstream port history. Assign a named human
maintenance owner before production adoption, including pin/advisory monitoring,
rebase/upgrade regression and failure triage responsibility; do not invent one.

- **Proceed (to a separately authorized migration plan):** every mandatory F row
  and stable-reference/mesh/worker/numeric contract demonstrated; applicable
  absolute and relative budgets pass, native/toolchain/license/security evidence
  complete, reference/device acceptance complete for the proposed supported
  targets, and migration scope/cost understood. Require a measured benefit:
  either at least 20% improvement in geometric mean of p95 whole-job times over
  the seven fixed timing fixtures without any >25% fixture regression, or removal
  of a specifically inventoried SolidCore-only maintenance patch/path with all its
  supported behavior demonstrated through a smaller auditable candidate adapter.
  Record actual code/maintenance evidence for the second route; predictions alone
  cannot satisfy it. Passing equivalence without a benefit does not justify migration.
- **Proceed conditionally:** list each exact bounded gap, allowed fix, owner,
  estimate, fixture/command and closing evidence. Examples: unavailable reference
  A/B or pending human checks; a measured mesh conversion defect with a bounded
  Newell-only fix. This permits planning the fixes only; **production replacement
  remains blocked until every mandatory gap closes**. Missing mandatory geometry
  is not waived under this label; a fix beyond this experiment needs separate scope.
- **Do not migrate yet:** mandatory supported geometry unavailable, wrong result/
  topology, heuristic references, transaction/retention failure, unacceptable
  compiler/FFI/license/security requirement, repeated budget/service failure,
  unbounded upstream/adapter work, exhausted investigation, maintenance burden
  outweighing demonstrated benefit, or no credible net benefit. Name blockers
  and the smallest evidence that could change the outcome. Keep SolidCore production.

No present migration recommendation is supported: APIs have been inspected, but
no SolidCore workflow, timing, memory, platform or UI qualification was executed.

### What survives from M5.0

Retain the independent solver/rank problem, sparse faer bridge, Foster–Davis
rank protocol, original-matrix residual/operator tests, exact arithmetic methods,
independent analytic oracles, rank-versus-finite-mobility distinction, resource
budgets and source/version identity requirements. A kernel change does not
invalidate the mathematical questions or complete their pending qualification.

SolidCore-specific items include vendor math certified intersection/overlap and
operations certified offset, native curve/control/knot conversions, arrangement
and native-profile construction, topology validity/provenance, and error mapping.
They remain useful reproducible probes and negative examples; do not port or
delete them during this experiment. The current M5 dossier has pending
revision-6 completion rows and partial/failed general-geometry evidence. Neither
historical passing tests nor source now containing continuation modules proves
those pending gates passed.

After a successful M4 qualification, separately requalify all M5 curve-family
contacts/tangencies/periodic seams, original-parameter intervals and residuals,
non-affine overlaps, exact versus distance-certified offsets, regions/holes and
downstream extrusion/replay. These are M5 obligations, not new M4 migration-entry
requirements. Also rerun native conversion/meshing cost, copied/live memory,
all-phase worker cancellation/service/publication, retained-generation cycles,
S1/S2/S3/C1/X1 whole-job timings and reference-device/manual checks on the final
backend. Independent solver phase measurements may remain historical evidence;
whole-job/native-worker measurements cannot transfer unchanged across kernels.
Keep `docs/plans/M5-sketcher.md` and its single validation dossier intact.

## 10. Migration outline only if qualification succeeds

A later Architect request would use the Q4 evidence to scope: final candidate
pin/options and dependency provenance; replacement of SolidCore internals behind
existing Newell-owned types; minimal ports of proven exact station/domain logic;
explicit reference/lineage mapping and ambiguity rejection; mesh/edge conversion;
worker retention/error/cancel integration; full default regression and native
platform/performance acceptance; dependency/notice and maintenance transition;
and removal of temporary comparison routing after human-reviewed replacement.
Parametric recipes/IDs remain authoritative. If persisted native state or
cross-version references are discovered, design and qualify their conversion
before adoption; do not assume a nonexistent storage migration requirement.

No permanent dual-kernel path, production fallback, M5 rewrite or automatic
adoption follows. The actual unresolved decision at this planning handoff is
human acceptance of §8's narrow architectural exception and the proposed bounded
execution contract. This stage stops with the two documents; it starts no review,
Executor, dependency installation or benchmark.
