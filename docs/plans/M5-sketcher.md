# M5 — Complete, responsive parametric Sketcher

Revision 5 — accessible sparse-rank method and execution boundary, 2026-10-29.
Role/provider: Architect / Codex.

The human interrupted revision-4 execution and requested an English Architect
revision comparing solver/rank alternatives and selecting an accessible,
documented method. This revision selects a safe-Rust adaptation of Foster–Davis
SPQR_RANK inside vendored faer, retaining faer's solve baseline. It replaces the
inaccessible Pierce–Lewis implementation prerequisite; it does not declare the
new method implemented or qualified. Rust without FFI, BrepKIT alone, every
Sketcher requirement and S1/S2/S3/C1/X1 budget remain unchanged.

This request authorizes **this plan only**. Preserve partial V0–V4 work;
no further implementation, dependency change, test execution, production
adoption, M5.1, agent dispatch or publication occurs in this Architect stage.

## 1. Task contract and inspected state

Deliver a complete planar parametric sketching workflow: draw, constrain, edit,
diagnose, select bounded regions, consume profiles in CAD operations, and edit
the sketch again without losing design intent. Large-sketch responsiveness is
a design requirement from the first slice, not a final optimization exercise.

This document is a proposal, not implementation authorization. The human owns
approval, role transitions, integration and publication. Do not start another
agent or role. Work sequentially in the existing Newell checkout on `main`;
no branches, worktrees, separate checkouts, commits or publication.

Architect output scope: this file only. No production, test, accepted ADR,
architecture, UI-reference, backlog or Description changes in this stage.
No builds, tests, application launches, numerical experiments or benchmarks
were run. All acceptance checks below are future Executor work.

Inspected HEAD: `086fc097c476dcd097aa1a05af86a273ca0fcdec`.
Revision 1 entry working tree: 93 status entries, including 15 untracked entries; index
empty. Existing changes include architecture and canonical UI semantics;
app/controller/document/geometry-worker/parameter/selection/navigation code;
kernel extrusion/profile/surface/boolean/termination code; model geometry and
parameter code; renderer scene, navigation and ASCII pipelines; UI panels,
menus, numeric editing, trees and snapshots. Untracked inputs include
`crates/newell-app/src/{extrude_session,navigation_widgets}.rs`,
`crates/newell-kernel/src/{boolean,termination}.rs`, renderer navigation files,
`docs/adr/0007-viewport-navigation-widgets.md` and existing UI/navigation plans.
Preserve all of them and the index. Reinspect materially changed inputs before
implementation; no snapshot or commit is a prerequisite.

Correction-pass entry: same HEAD and `main`, 98 status entries, including 16
untracked entries; index empty. This plan is an existing untracked input.
Reinspected the worker and parameter-draft paths, pinned BrepKIT offset and
tolerance sources, and current architecture/UI selection and navigation rules.
The worker now also carries prepared-result authority and liveness state;
reuse those safeguards. Navigation specifications have changed since revision
1: transparent tile allocations do not occlude the scene; only actual semantic
targets intercept input. References below defer to that current contract.
Existing source, UI, architecture and navigation changes remain untouched.

### Revision 5 inspected state and authority

Same `main` checkout and HEAD `7019c300ff12ff6a1aeee974ca59f865e00b045e`;
index empty. The revision-4 plan and earlier source/document changes remain
unstaged. Additional inputs now include architecture §§5.2/6 and accepted
ADR0009 (V0), three untracked vendor package trees and their provenance files,
root patch/exclusion/lock changes, candidate kernel feature wiring and partial
BrepKIT candidate curve/offset APIs with focused test additions. Preserve all
of these, including unrelated work; only this plan changes in this stage.

V0 is already applied under explicit human authority. V1 imported the published
baselines; earlier execution reported 1,082 parity tests passing, 21 skipped,
and three focused candidate tests passing. These are historical execution
results, not checks rerun here. V2 has no rank implementation; an empty feature
is not an implemented API. V3's restricted analytic/affine probes do not cover
the complete curve-family corpus. V4 remains incomplete. Vendor patch ledgers
must be reconciled with the actual partial edits at the next Executor stage.

Read the shared workflow, architecture, Architect role, ADR0009, current plan,
manifest/source seams and accessible primary numerical references. The former
Pierce–Lewis full-text prerequisite could not be fulfilled during execution.
§9.5 V2 now selects an accessible alternative and states the adapter obligations
explicitly. This removes a documentation-access blocker, not the outstanding
numerical, geometry, latency or physical-device qualification gates.

### Revision 4 inspected state and authority

Same main checkout and HEAD `7019c300ff12ff6a1aeee974ca59f865e00b045e`;
index empty. This plan is an existing unstaged input. Preserve the revision-3
entry paths below, the new test-only kernel `src/sketch/{mod,curve}.rs`,
nonlinear chain benchmark/test additions, actual-worker recovery test and
updated M5 validation record. At that historical revision-4 entry no vendor
tree existed; revision-5 entry above records the subsequent partial execution.
Reinspected workspace/kernel manifests, ADR 0004, architecture §§5–6, the
published faer0.22.6 and BrepKIT4.2.2 manifests and relevant local source seams.
The existing dossier reports1,082 passing tests and remaining feasibility gaps;
that is historical evidence, not a new check run by this Architect.

The human's approved direction now includes developing controlled third-party
Rust sources, rather than only searching for an already qualified library.
§9.5 supersedes revision-3 clauses requiring another generic permission merely
to develop those **named** sources. Concrete unrelated dependencies, changed
upstream baselines, licensing terms, FFI or production adoption remain outside
that scope. Existing D1–D3 Sketch specification prerequisites are unchanged.

### Revision 3 inspected state and decision authority

Current checkout: `main`, HEAD `7019c300ff12ff6a1aeee974ca59f865e00b045e`;
index empty. The plan was clean at entry. Preserve existing unstaged changes in
architecture, Cargo files, Description, app camera/controller/navigation/
geometry/session/parameter code, kernel geometry/test-support, model geometry,
renderer scene, UI selection/tree/menu files, backlog and M3/M4/Selection plans.
Existing untracked inputs include the M5 benchmark, three Sketch test modules,
and M4/M5 validation records. No snapshot, branch or checkout is required.

Read the shared workflow/Architect role, current architecture, ADR 0004 and
relevant ADR status/boundaries, current manifests, M5 probe/worker seams and
`docs/validation/M5-sketcher.md`. That dossier supplies historical measurements;
this Architect turn runs no builds, tests, application or numerical experiments.
`docs/validation/M4.5-M4.10-extrude-pipeline.md` now records prototype-scope
implementation complete, acceptance pending, with explicit M4.D1/M4.D2 advanced
geometry deferrals. Neither those deferrals nor historical green checks reduce
M5's exact-curve/profile requirements or establish current M4 acceptance.

| Authority / finding | Revision-3 disposition |
|---|---|
| Human approved Rust without FFI as priority, one BrepKIT kernel, unchanged requirements | Binding direction for this revision. No FFI fallback is authorized; absence of a viable Rust method yields an explicit blocked package. |
| D1–D5 approved as the revision-2 M5.0 requirements baseline | Preserve that approval. D1–D3 authoritative Sketch amendments remain a separate prerequisite before dependent production UI/camera work; this plan does not edit binding specifications. |
| Forced-supernodal faer with reusable workspace improved measured chain/grid phases | Preferred solve candidate; not proof of large numerical rank, nonlinear convergence or actual app responsiveness. |
| Large-block numerical rank unavailable in the investigated contract | §9.3 B requires an established method/API and independent evidence before adoption; no custom factorization or dense global fallback. |
| Pinned contact/overlap/offset failures | §9.3 C–D plan targeted BrepKIT corrections and certified algorithms, not a general untested kernel upgrade. |
| Scalar fixtures and service surrogates leave integration evidence missing | §9.3 E–F require a real-worker feasibility harness and mixed geometry traces; no production Sketch shell is needed to run those proofs. |

### Revision 2 correction traceability

| Challenger finding | Revision 2 correction | Required acceptance evidence |
|---|---|---|
| 1 — Cancel can erase another window's commit | §8 discards only session-private state | Relevant and unrelated concurrent commits survive Cancel |
| 2 — Large-block rank/DOF method unproven | §6.1 separates regular numerical rank, singularity and bounded mobility; §9.2 requires a named proven method | Dense/analytic comparisons, singular and large coupled cases |
| 3 — Cancellation does not release the worker | §6.2 defines service classes, yielding and indivisible-call evidence | Cancel in every expensive phase, then service a click and another window |
| 4 — Certified advanced curve operations unproven | §§5, 7.1 and 9.2 require operation-family and tolerance evidence through extrusion | Contacts/overlaps/offsets, nesting and translated small-detail fixtures |
| 5 — Copies outside entity storage break incrementality | §§3.2, 3.3 and 10 cover parameter drafts, messages, indexes and retained generations | S3 geometry/formula edit allocation and traversal counters |
| 6 — Split/merge leaves no repair path | §7.4 defines candidate-local Extrude profile repair | Repair/Cancel/atomic commit and failed downstream rebuild |

M5 planning is authorized now. Production execution follows human acceptance
of M4's real command/parameter/geometry pipeline. Do not treat backlog boxes,
an old validation report, or this plan as evidence that M4 is currently green.

### 1.1 Evidence and source precedence

| Source inspected | Consequence for this plan |
|---|---|
| `docs/agents/{README,architect}.md`, `ARCHITECTURE.md` | Five existing crates, strict MVC, safe Rust, one ASCII draw, Controller input ownership, kernel boundary, sequential work |
| ADRs 0001–0006 and current §3.7/ADR 0007 | BrepKIT 4.2.2, Rust 1.88, AGPL project; navigation exception is narrowly scoped and cannot silently authorize Sketch graphics |
| `docs/ui_design/README_UI.md` §§2–5, 18–23, 25–29, 34–40, 41 | Fixed-cell controls, numeric/formula lifecycle, modal precedence, trees, current Sketch command is a mock; current navigation semantic targets retain input precedence without rectangular scene occlusion |
| `docs/ui_design/{Action_Box_Sketch,Sketch_Bar,Command_Bar,Application_Menu_Bar,Interface_Elements}.txt` | Existing Sketch artwork is real input, with unresolved semantics and inconsistencies listed in §2 |
| `docs/backlog.md`, `Description.md` §10 | Current M4 is CAD Command & Parametric Interaction; Description's older M4 Sketch / M5 Parametric Core numbering is stale |
| `crates/newell-model/src/{lib,geometry,quantity,parameters,formula}.rs` | Real units, typed expression graph, stable feature/entity IDs and prepared-document protocol exist; no real Sketch feature/entity model or sketch journal exists |
| `crates/newell-app/src/{document,geometry_worker,extrude_session,parameters,selection,navigation}.rs` | Reuse document-instance/version certificates, worker-owned exact geometry, bounded latest-work transport, terminal delivery and model-selection seams |
| `crates/newell-kernel/src/{lib,profile,extrude,surface}.rs` | Exact profiles and B-Rep wrappers exist; no general Sketch region arrangement or scalable sketch solver wrapper is exposed |
| `crates/newell-render/src/{renderer,scene,navigation_widgets}.rs`, current scene/reference shaders | Immutable packets and separate viewport graphics are possible; retain one flat ASCII draw and account for additional Sketch draws explicitly |

Pinned-source inspection used the local Cargo registry under
`/Users/martin/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`:

- `brepkit-operations-4.2.2/src/sketch.rs` re-exports `GcsSystem`,
  `GcsConstraint`, points, lines, circles, arcs and rank/solve results.
- `brepkit-sketch-4.2.2/src/gcs/{system,constraint,solver}.rs` supplies a useful
  basic-entity reference solver. DogLeg builds and clones a dense row-major
  Jacobian before QR. Its public solve has an iteration limit but no
  cooperative-cancellation callback. The observed public constraint family
  does not cover the full proposed spline/conic/curvature toolset.
- `ab_glyph-0.2.32/src/outlined.rs` exposes line/quadratic/cubic outline data.
  This is a possible text-outline seam, not proof that its floating-point
  outline precision meets every Sketch scale requirement.

API presence is not correctness, thread-safety or performance evidence.
Never modify registry sources, upgrade the pinned kernel silently, or use
display polylines as exact CAD boundaries.

## 2. Proposed product decisions and specification amendments

The choices below resolve the design for review. They require the human's
explicit approval where they amend binding specifications. This Architect
stage does not apply those amendments. An implementation request alone must
not be interpreted as permission to ignore unchanged conflicting rules.

### D1 — A scoped Sketch viewport graphics/input contract

Propose a dedicated architecture subsection and canonical UI §42, restricted
to the active planar Sketch session:

- Analytic sketch curves, construction/projected curves, dimension leaders,
  points, control polygons, selection handles, snap proposals and box selection
  are viewport geometry. They are not ASCII widgets or B-Rep model selection.
- The app owns selection, snapping, tool state, hit queries and camera changes;
  the renderer receives immutable derived packets and performs no picking.
- Geometry candidates may be queried through a spatial index after O(1)
  ASCII/overlay routing. §3.4's prohibition on widget bounding-box scans remains
  unchanged; it does not become a prohibition on indexed geometric queries.
- Use the current theme's ordinary Text ink for Sketch graphics. No solver
  state colors, red/green DOF coloring, hover glow, fading or animation.
  Ordinary curves use a 1.5 logical-pixel stroke; selected curves use 2.5.
  Construction uses 6/4 logical-pixel dashes; projected references use 10/3/2/3
  dash/dot intervals. Dashes follow curve arc length and do not animate.
- Visible points use 3-logical-pixel squares; selected editable handles use
  6-logical-pixel unfilled squares. The active snap proposal is one
  8-logical-pixel diamond and an ASCII status description. It is a tool
  proposal, never an automatically committed constraint. At most one proposal
  is displayed, with deterministic cycling through ambiguous candidates.
- Selected-region boundaries use the selected stroke. No translucent region
  fill is required. Selection rectangles use a fixed dashed outline, with no
  background inversion. Constraint glyphs/labels use documented ASCII names
  and existing-font cells, not a new font or pictogram dependency.
- Graphics dimensions convert logical pixels once using the window scale.
  This does not change ASCII cell metrics or model-space tolerances.
- Add one batched Sketch stroke/handle pipeline, at most two draw calls when
  active; labels remain in the single ASCII overlay. Navigation-widget draws
  retain their own existing accounting. Suppressed Sketch graphics issue zero
  Sketch draws. No per-entity draw call or GPU picking/readback.
- Curves and handles are exposed only inside unobstructed viewport pixels.
  Crop label cells and strokes against Action Boxes, trees and overlays;
  preserve current §41 navigation composition and semantic-target input
  precedence. Do not turn a transparent navigation tile into an occluder.
  Snap proposals cannot appear through an occluder.

This is an explicit proposed exception to §§1, 3.1, 3.4–3.6 and related ADR
restrictions where necessary. §3.7 grants no implicit Sketch exception.
Preserve all existing ASCII hover and ordinary model-rendering rules.

### D2 — Canonical Sketch controls and reference corrections

Keep `Action_Box_Sketch.txt` at its actual **27-cell** external width. Existing
31-cell Extrude/Revolve/Loft/Sweep/Boolean boxes do not establish a 31-cell
Sketch requirement. Preserve its existing rows; specify their semantics:

| Existing row | Proposed meaning/default |
|---|---|
| Plane | Principal XY/XZ/YZ plane, a real datum plane, or a certified planar face; no fabricated nonplanar support |
| First Construction, before Slice | Creation mode: new entities are construction-only; initially unchecked; changing it does not convert existing geometry |
| Slice | Window-local clipping of background model geometry to the Sketch plane's far half-space; unchecked; no caps or model mutation |
| Sketch Grid | Window-local adaptive grid visibility; unchecked; independent of snap-to-grid |
| Profile | Show selectable bounded-region boundaries; checked |
| Point | Show entity points; checked |
| Dimension | Show visible driving/reference dimensions; checked |
| Constraints | Show contextual geometric constraint labels; checked |
| Projected Geometries | Show reference geometry; checked |
| Second Construction, in display section | Show construction geometry; checked; does not change entity roles |
| OK / Cancel | Commit the entire edit / discard the session; never implicit per-primitive confirmation |

Both Construction labels keep their positions; the divider separates tool mode
from visibility. Their accessible descriptions and contextual status explicitly
say `Create construction geometry` and `Show construction geometry`.

The Sketch context's CommandBar is
`Workbench | Draw | Curve | Modify | Pattern | Reference | Constraints`, using
the existing whole-cell layout and centered-popup policy (§29.1). It is a
temporary editing context, not a new persistent workbench or a new global UI.
The previously active workbench returns on exit.

Normalize `Sketch_Bar.txt` only in the approved amendment: remove the duplicate
second `Tangent Arc` row; change `Beziez` to `Bezier`, `Perpendicu` to
`Perpendicular`; remove the trailing space outside the `3-Pts Circle` frame;
replace its inconsistent header with the context groups above. Existing popup
rows have 22 cells including borders. Recompute only the Curve popup height
after removing the duplicate. Keep exact spacing of unaffected rows.

Add canonical reference states for Sketch tool input, constraint/dimension
editing, diagnostic lists and profile selection, using existing 31-cell tool
panel and Selection/Formula/Enter Name grammar. No unrelated menu rewrite.
Their field schema and interaction rules are in §§4–5; Executor must make the
approved references/semantics executable before composing new production UI.
`Sketch_Diagnostics.txt` and `Sketch_Profiles.txt` must also describe the
candidate-local dependency repair states and focus return of §7.4, using the
existing Selection grammar (§§19–21), without changing Extrude artwork.
Existing `Action_Box_Extrude.txt` is not an output file.

### D3 — Camera and input during Sketch editing

On entry, save the current camera/workbench and align the one authoritative
camera orthographically to the support plane, with its local U right and V up.
This requires a narrow §4.4 amendment allowing plane-relative camera up during
Sketch editing; ordinary world-Z navigation resumes on exit. Do not create a
second mutable camera in the renderer.

Primary click draws/selects; primary drag manipulates geometry or performs
box selection in Select mode. Secondary/middle drag pans, wheel and pinch
zoom, and phased two-finger pan retain their established lifecycle rules.
Alt+primary drag or the View Navigator can inspect the sketch in 3D. Return
`Normal to` to the active support. When ray/plane intersection is ill-conditioned
or behind the eye, decline drawing/dragging with `View nearly parallel to sketch`
instead of placing distant or nonfinite points. Navigation remains available.

On exit restore the entry camera/workbench, resized to the current viewport;
do not undo unrelated per-window theme, layout, UI-scale or visibility choices.
Scale/layout changes cancel the current gesture, preserving the session draft.
Document replacement invalidates the entire session.

### D4 — Meaning of a complete M5

All commands in the corrected Sketch Bar are functional targets of M5,
including conics, splines, text, references and patterns. They may arrive in
successive slices, but a permanently disabled or mock advertised command does
not satisfy completion. Ordinary planar Sketch editing and the named test
classes must meet §10; no universal interactive guarantee is made for arbitrary
dense nonlinear systems or arrangements with quadratic output.

Explicit exclusions: free-space 3D sketches, surface-attached nonplanar sketches,
cloud/multi-user editing, a new native file format, general SVG/DXF import,
font management, and implementing the full Revolve/Loft/Sweep commands.
`Insert` means copying supported geometry from another sketch in this document;
external-format import belongs to the Files milestone. Publish open-wire and
region interfaces usable by those later CAD commands.

Text uses the bundled Fira Code font. Printable ASCII is the initial supported
text repertoire; reject unsupported input explicitly. Bidirectional shaping,
font substitution and arbitrary Unicode typography are outside this planar
mechanical sketching milestone. This limitation must be visible before entry,
not discovered after an apparently successful commit.

Document-native save/load remains M6. M5 must preserve complete model recipes,
provide a versioned Sketch DTO round-trip test, and support local and document
undo/redo. Do not claim disk persistence or crash recovery from an in-memory
round-trip test.

### D5 — Solver strategy and numerical dependencies

Use a Newell-owned Sketch equation/graph layer behind `newell-kernel`, with a
single production solver contract for all supported entities. Use established
sparse linear algebra, rather than implementing QR/factorization internally.
Preferred numerical dependency: `faer = "=0.22.6"`, default features disabled,
with only `std` and `sparse-linalg` enabled initially. This published version
declares Rust 1.84 and MIT. Its exact usable API, transitive MSRV, allocation
behavior and cancellation granularity must be proven in M5.0 before adoption.
No BLAS/FFI integration, dependency upgrade or unsafe adapter is authorized.

Use the existing `brepkit_operations::sketch` re-exports as a bounded test
oracle for common constraints. Do not add a direct `brepkit-sketch` dependency,
copy private upstream internals, or present its dense solve as the scalable
production implementation. Custom code is restricted to Sketch semantics,
incremental dependency handling, residual/derivative assembly, trust-region
orchestration and diagnostics; this is justified by the observed dense backend,
missing cancellation hook and advanced-curve coverage gaps. The established
library supplies the numerical infrastructure.

Use `rstar = "=0.12.2"` for indexed geometric candidate search, restricted to
kernel/query modules; it declares Rust 1.63 and MIT OR Apache-2.0. Use the MIT
option in the dependency inventory. Do not implement another general R-tree.
Text may reuse `ab_glyph = "0.2.32"` inside the kernel if its contour extraction
passes the precision probe; prefer a checked parser with preserved font units
if that probe establishes a precision defect. A different parser needs the
same MSRV/licensing/security review, not an unapproved font-system expansion.

Revision-2 candidates now exist as development dependencies in the inspected
checkout; their historical probes are not production adoption. Revision 3 prefers
faer 0.22.6's safe public forced-supernodal QR and reusable numerical workspace
for solving, while keeping undamped rank analysis separate. Research compatible
established pure-Rust rank methods under §9.3 B. A newly selected library/version
requires an exact candidate inventory and explicit adoption decision first.
No FFI, BLAS or unsafe Newell adapter is authorized. Revision4 §9.5 permits
an algorithmically specified extension inside the controlled third-party faer
source, not a private factorization in Newell or an automatic fallback.
BrepKIT corrections follow §9.3 C's patch/pin decision procedure; the currently
accepted 4.2.2 pin remains authoritative until that procedure is approved.
Executor verifies the full resolved graph, advisories, licenses and Rust 1.88
on supported architectures before merging any dependency change. If a candidate
cannot meet the contract, stop its dependent work with the concrete evidence;
do not silently replace the solver with an unbounded dense fallback.

## 3. Ownership, identities and interfaces

### 3.1 Existing crate boundaries

| Owner | Responsibility |
|---|---|
| `newell-model` | Document Sketch recipes, entity/constraint IDs, parameter ownership, reference DAG, journal actions, validity/commit policies and stable logical outputs; no UI, GPU or native kernel handles |
| `newell-kernel` | Numerical Sketch evaluation, exact curve queries/intersections/arrangements, exact profile construction and BrepKIT topology; Newell-owned public DTOs only |
| `newell-app` | Window-local Sketch sessions, tool/gesture ownership, certificates, job scheduling, app-to-model/kernel adapters, atomic commit, document notifications and frame publication |
| `newell-ui` | Fixed-cell panels/menus, numeric/formula editors, diagnostic/selection lists and immutable view projections; never authoritative geometry or constraint solving |
| `newell-render` | Immutable curve/stroke/point packets and existing-font labels, viewport clipping and GPU uploads; no solve, geometric query, selection or camera mutation |

No `newell-sketch` crate or second B-Rep kernel. Model and kernel input DTOs are
mapped in the app/worker so the kernel never imports model IDs or the model
imports BrepKIT. Pure semantic validation stays in Model; geometric and
numerical validation stays in kernel.

### 3.2 Model additions

- `SketchId`, `SketchEntityId`, `SketchConstraintId`, `SketchGroupId` and
  `SketchRevision`: checked document-scoped IDs, never reused; separate from
  EntityId, WidgetId, arena positions and solver variable columns.
- `Feature::Sketch(SketchFeature)` retaining FeatureId/name/history placement;
  extend EntityKind with explicit Sketch/SketchProfile categories rather than
  falsely describing all sketch entities as solid bodies.
- `SketchSupport`: PrincipalPlane, certified DatumPlane reference, or a
  generation-qualified planar-face reference. Store a validated right-handed
  `SketchFrame { origin, u, v, normal }` and support provenance separately.
- `SketchGeometry`: analytic definitions for points, finite segments, circles,
  oriented arcs, ellipses/arcs, rational quadratic conics, Bézier and rational
  B-splines; definition/control geometry and derived display are separate.
- `SketchEntityRole`: Profile, Construction, or ExternalReference. Shape and
  role are independent. Visibility never changes profile membership.
- `SketchConstraint`: typed entity/subentity references and optional
  ParameterId, source/branch qualifiers, driving/reference/suppressed state.
- `SketchDelta`: touched entities/constraints/groups/parameters and inverse
  edit data; one completed gesture produces one undo action.
- `SketchProfileRef`: SketchId, region generation and region lineage ID.
  `SketchWireRef`: an explicitly selected ordered open/closed boundary chain.

Store large entity/constraint tables in immutable shareable chunks with
copy-on-write updates. A pointer move must not clone the whole Document,
parameter registry, curve list or exact topology. Use stable dense slot tables
plus ID-to-slot maps and tombstones as needed; slot compaction cannot change
identity. Cache data is derived and absent from document equality/persistence.

Reuse M4's Quantity/ParameterRegistry/Formula DAG, owner restrictions, precision,
origin certificates and terminal-delivery semantics. A Sketch dimension's
ParameterId belongs to its Sketch feature. Reference dimensions are computed
readouts and cannot feed a cycle back into their own solving component.

Reuse M4 semantics, not its current deep-copy costs. In the already scoped
`newell-model/src/{lib,geometry,parameters,formula}.rs` and app document,
parameter and worker adapters, extend shared immutable storage to the committed
registry, formula definitions, reverse dependency caches and source document
roots. A Sketch parameter draft retains a shared certified base plus touched
definition/name/binding edits and inverse data. Updating a field traverses its
affected formula dependencies; it cannot clone the complete registry or scan
all committed definitions merely to rebuild an active-root set. Global formula
dependencies may require global evaluation and must be measured separately.

Adapt `ParameterDraft`/`begin_parameter_draft`, prepared candidates and Sketch
messages so source capture and handoff share roots/chunks rather than copying
all document/registry/mesh data. Preserve owner restrictions, stable IDs,
validation equality, prepared authority and M4 behavior; cache identity is not
semantic equality. Use the same bounded-block approach already required here,
not a new general storage framework or persistence layer.

### 3.3 Worker/query contracts

Define these Newell-owned contracts; signatures may follow existing error
conventions without changing their ownership or semantics:

| Contract | Input/output |
|---|---|
| `SketchEvaluationInput` | Canonical-unit definitions, changed-ID set, source certificate, last accepted solution and parameter values |
| `SketchEvaluationBudget` | Iteration/workspace limits, deadline, cooperative cancellation and interactive/final purpose |
| `SketchEvaluationResult` | Solved coordinates, residuals, rank/DOF certification state, diagnostics, changed IDs and exact source certificate |
| `SketchQuerySnapshot` | Shared immutable analytic descriptor chunks and a revision-qualified worker index key for the acknowledged SketchRevision; no native handles or whole-tree clone |
| `query_sketch` | Local-plane point/rectangle and tolerance -> deterministic candidates with curve parameters and source IDs |
| `extract_sketch_regions` | Solved geometry and affected area -> bounded faces, open wires, overlaps/gap diagnostics and provenance |
| `prepare_sketch_commit` | Final solved definition + region results + dependent history -> existing PreparedDocument-style authority and retained worker geometry token |

Define kernel-owned keys and map them to model IDs explicitly. Query results
carry revision and entity role; renderer vertices/triangle ordinals are not IDs.
Index construction and heavy queries run off the event loop. Pointer queries
use a newest-only bounded request; consume a result only while its pointer,
camera, layout, source and tool certificate remain current. A press may wait
for its certified query with visible status; it must not select a stale target.
Motion-query freshness and press-query authority are distinct: freeze the
press pixel and source certificate when the button goes down, retain movement
and the click/drag threshold while its query is pending, then apply accumulated
movement to that certified target. A later pointer position does not retarget
the press or erase it. A camera/layout/source/modal change cancels it and
consumes the corresponding release, as for existing certified scene clicks.

The mutable R-tree stays worker-owned. Update changed entries in bounded batches
and publish its revision only when the batch is complete; never query a mixed
revision. Retained display snapshots share descriptor/packet chunks, not a
mutable tree. An obsolete index key cannot authorize a query: cancel/reissue
against the current certified revision. Keeping an old display packet alive
must not force `Arc::make_mut` of a complete R-tree or entity/parameter table.

## 4. Functional coverage and tool contracts

### 4.1 Tool state shared by every command

One finite state machine per window:
`Idle -> SelectingSupport -> Editing/Select -> ToolCollecting -> ToolReady`,
with explicit `Solving`, `Diagnosing`, `Confirming`, `Conflict` and `Closed`
substates. These are domain/session states, not retained widget objects.

- Click-click construction is the default; a tool supplies prompts for each
  required point and a live, explicitly provisional preview.
- A valid primitive confirms on its final click or explicit tool acceptance,
  remains in the session, and repeats the same drawing tool. It does not commit
  the document. Multi-segment Line continues from its previous endpoint.
- Tab cycles numeric inputs using existing editing rules. Enter first accepts
  an active field, then a ready primitive; never commits the entire sketch.
- Escape cancels the active gesture/partial primitive first, then returns the
  tool to Select. In Select it clears the selection. Entire-session discard
  uses the explicit Sketch Cancel button; accidental Escape does not lose work.
- Undo/redo while editing targets the local journal, with a whole drag, trim,
  pattern or dimension edit as one action. Redo is invalidated by a new edit.
- Tool popup mnemonics are scoped as in §29; no global single-letter action
  fires while editing text or while a modal owns input.
- Manipulation uses temporary drag objectives, not persistent Fix constraints.
  Preserve solution branch/handedness unless the user explicitly changes it.
- Confirming invalid inputs keeps the editor open with source-spanned or
  entity-qualified diagnostics. No silent rounding, repair or entity deletion.

Tool numeric-input projections use semantic field IDs (never row numbers),
existing 31-cell input rows, progressive disclosure and Formula controls.
Required fields follow the command table below; coordinates and distances are
Length, angles Angle, counts integer Scalar, rational weights Scalar. Directions
remain separate from unsigned magnitudes where M4 requires that separation.

Only one contextual Action Box is visible: Select/support editing uses the
27-cell Sketch reference; an active construction/modification tool replaces it
at the same top-right anchor with its 31-cell tool panel. Do not stack two
panels beyond the 82x30 minimum body. The tool panel contains its title, inert
support name, required semantic numeric/selection fields and `Apply` /
`Close Tool` controls. Apply completes one ready primitive/compound edit and
repeats the tool; Close Tool discards its unfinished primitive and returns to
the Sketch panel without discarding completed draft actions. Escape follows
the same tool-cancellation rule. Whole-session OK/Cancel are available in the
Sketch panel after returning to Select. A tool cannot interpret Apply as a
Sketch commit. Long field groups scroll within the body's available whole
rows, keeping the title, active field and tool controls accessible; the
canonical reference documents the viewport/gutter without resizing fields.

### 4.2 Drawing and advanced curves

| Tool family | Required behavior and inputs |
|---|---|
| Point / Line | Local X/Y entry; connected polyline; close to first point by an explicit accepted coincident proposal; construction axes available as references |
| Rectangle | Two corners; Center Rectangle uses center/half extents; 3-Points Rectangle uses base endpoints and perpendicular width; create editable ordinary segments plus the defining constraints |
| Polygon | Center, circumradius, integer sides >=3, rotation; ordinary segments/constraints with source group; cap expansion by the admitted workspace budget before allocation |
| Circle | Center/radius or three noncollinear points; exact analytic circle |
| Slot | Centerline endpoints and width; two sides plus tangent semicircular caps, editable as a defining group; reject zero width/length |
| Arc | Three-point, center/start/end and tangent-at-existing-endpoint modes; retain explicit orientation and sweep, including sweeps above pi; reject degenerate input |
| Ellipse / Elliptical Arc | Center, axes, rotation, then ordered parameter interval for arcs; reject nonpositive minor axis and invalid intervals |
| Conic | Bounded rational quadratic defined by endpoints, tangent-intersection control and positive weight; show ellipse/parabola/hyperbola classification; only bounded finite intervals can become profiles |
| Bezier | Editable control polygon; default cubic; degree elevation and control-point insertion must preserve the represented curve where mathematically possible |
| Spline | Interpolating cubic through fit points, with explicit editable tangent conditions; expose its equivalent B-spline representation |
| Add Control Points | Exact knot insertion for B-splines; split/degree elevation for Bézier; do not claim arbitrary added Bézier controls preserve degree and shape |
| Text | String, baseline placement, height, rotation and spacing; retain text recipe plus bundled font identity; derive oriented line/quadratic/cubic contours with holes, as one rigid editable group |

Represent ellipses/conics/Bézier/splines exactly as analytic/rational curves in
kernel topology. Display tessellation is not the representation used for
intersection, trim, profile identity, measurement or extrusion.

### 4.3 Modification, patterns and references

| Family | Required behavior |
|---|---|
| Trim / Extend / Split | Pick a certified interval; trim to neighboring exact intersections, extend finite geometry within its mathematical domain, split at a specified point; map surviving constraints by provenance |
| Fillet / Chamfer | Radius or two setbacks; explicit target corners; construct tangent arcs or connecting segments atomically with trim of source curves |
| Offset | Signed side plus unsigned distance, single-chain/region selection, join policy Round/Miter/Bevel; resolve self-intersections into selectable outputs; exact curves or a certified bounded-error curve representation, never screen chords |
| Transform | Copy/move/rotate/scale about a chosen point; uniform positive scale; reject incompatible driving dimensions rather than scaling their meaning silently |
| Mirror | Choose axis; independent copied geometry by default; explicit associative option adds source/group relations |
| Linear / Circular Pattern | Count, spacing or angle, direction/center; retain an associative pattern recipe by default; instances share master definitions and transform parameters rather than duplicating every master solve variable |
| Projected | Orthogonal projection along support normal; choose linked reference (default), independent copy, or explicit conversion into profile geometry |
| Intersect | Intersection of supported upstream faces/curves with the Sketch plane, through BrepKIT; retain upstream generation and return exact planar curves |
| Insert | Select another same-document sketch and copy supported entities into the current frame using an explicit rigid transform; reject noncoplanar copies unless Projected is selected |

An associative pattern instance is edited via the master or detached explicitly
into independent entities. Constraints to an instance refer to its analytic
transform and master variables; shared definitions do not create fake
independence. Transform/trim/delete previews list removed, retained and
invalidated constraints; never silently leave dangling references. Redundant
exact duplicates can be repaired only through an explicit previewed operation.

### 4.4 Constraint and dimension coverage

Support Coincident; point on finite curve with a bounded curve parameter;
Horizontal/Vertical; Parallel/Perpendicular; Collinear; Concentric; Midpoint;
Tangent; Equal length/radius; Symmetric; Fix; rigid group; and dimensions for
length, X/Y/direct distance, angle, radius, diameter and arc length.
`Normal` means a selected segment normal to a selected curve at an explicitly
coincident attachment point; it is not an undefined synonym for Perpendicular.
`Curvature` constrains signed planar curvature at the chosen joined endpoints
after positional/tangent continuity; zero derivative is invalid, not NaN.

Define applicability per entity pair in a typed capability table. An invalid
pair returns an actionable selection requirement. Under-constrained is a valid
state. Redundant-consistent, inconsistent, degenerate, nonconverged,
budget-exhausted, cancelled and reference-broken are distinct outcomes.
Do not classify failure to converge as proof of inconsistency.

The minimum applicability matrix is:

| Relation | Admitted references and branch policy |
|---|---|
| Coincident / Midpoint | Two point references / point and finite-segment midpoint; endpoint/center/control-point references are explicit |
| Point on curve | Point and bounded segment/arc/conic/spline interval; a solved auxiliary parameter cannot escape the admitted interval |
| Horizontal / Vertical | Finite segment or two selected points, in the Sketch frame |
| Parallel / Perpendicular / Collinear | Two nondegenerate segments or construction axes; Collinear includes positional incidence |
| Concentric | Circle/arc/ellipse centers; does not impose equal radius or equal ellipse axes |
| Tangent | Line/curve or curve/curve, at named joined endpoints or explicitly chosen contact parameters; internal/external circle tangency branches are retained |
| Equal | Segment length, circle/arc radius or corresponding ellipse axes; no ambiguous equality of two arbitrary shapes |
| Symmetric | Two point references about a line/axis or center point; grouped geometry expands through its defining points and checked curve parameters |
| Fix / Rigid group | Fix an entity's independent parameters, or retain a group's internal shape with its planar rigid-transform freedom; neither is silently added during drag |
| Normal / Curvature | The attached segment/curve and joined-endpoint cases defined above; require finite nonzero tangent derivatives |
| Dimension | Entity length/radius/diameter/arc length, point-to-point or point-to-curve distance, X/Y distance, segment angle or angle between segments; explicit signed/unsigned branch and units |

Spline length and point-to-curve distance use kernel evaluation with a checked
error bound and derivative policy. Reference measurements never become
driving equations implicitly. Suppressed constraints retain identities and
definitions but contribute no equations; restoring them triggers full affected
component validation before acceptance.

Dimension editor fields: measured type, driving/reference mode, value/formula,
optional parameter name and explicit branch/direction where relevant. Constraint
editor fields: selected relation, participating entities, numeric parameter
when applicable, suppression and deletion. Diagnostics list shows stable IDs,
plain-language reason, residual/units and `Select affected` action.

## 5. Selection, snapping, annotations and numerical policy

Select mode supports point selection, Shift add, existing toggle modifier,
rectangle-contained left-to-right selection and crossing right-to-left
selection. Filter points/curves/profiles/constraints/dimensions/references.
Cycle overlapping candidates with Tab when no numeric editor is active.
Tie-break by role/selection priority, screen distance and stable ID; never by
hash iteration or transient solver column.

Snap order: explicit selected target, endpoint/origin, intersection, midpoint,
center, then nearest curve/grid. Default radius 8 logical pixels with a
12-pixel release radius for hysteresis; cycle candidates and allow temporary
suppression with Ctrl while drawing. Auto-constraints are on by default but
the active inferred relation is named before click. Coincidence at accepted
endpoints is persistent; merely hovering/snapping a preview creates nothing.
Proximity does not merge model identities. Projected references are read-only
until explicitly copied or converted.

Keep finite f64 model coordinates in canonical millimetres/radians. Normalize
solver coordinates around a local origin and characteristic length; scale
angle and length residuals separately. Normalize tangent/parallel equations
and guard zero derivatives. A document tolerance policy contains distinct
geometric, intersection, solve and rendering tolerances. Initial proposal:
geometric absolute tolerance `1e-7 mm`, relative term `1e-12 * local_extent`,
with a `1e-4 mm` maximum automatic-coincidence tolerance. If a model cannot
distinguish required details within that policy, report resolution/conditioning
failure; do not silently increase the tolerance. Final scaled solver residual
must satisfy the corresponding dimensional tolerance, not just a raw norm.

This proposed policy must be reconciled with the pinned kernel's actual
tolerances in M5.0. Interactive snap distance never changes exact closure.
Record each construction/intersection/validation/extrusion call's actual
absolute, relative, angular and parameter-space thresholds. BrepKIT 4.2.2
`Tolerance::new()` uses linear `1e-7`, angular `1e-12` and relative `1e-10`,
with relative comparisons based on operand magnitude rather than Sketch local
extent. Do not treat these policies as interchangeable. Keep planar predicates
and profile construction in the validated local frame where supported, and
certify the conversion to support/world coordinates through downstream calls.
If an API fixes an incompatible tolerance internally, report that operation
as unsupported at the affected scale until a compliant path is proven; do not
silently enlarge closure tolerances or patch registry sources. M5.0 must fix
the compatible thresholds and admitted scale limits in its capability report
before dependent implementation; changing product tolerances beyond this
proposal requires an explicit human decision.
Do not infer coincidence from decimal display, grid rounding or mesh vertices.
Treat near-equality as a checked geometric predicate, not a transitive global
equivalence relation: A near B and B near C does not authorize merging A and C.

Dimensions/constraint labels use current-size existing-font ASCII cells,
projected then snapped to whole cells. Active edit and selected labels have
priority, followed by stable-ID order. Use a cell occupancy map and retained
user offsets; do not compare every label with every other label.
Default visible annotation budget: 200 labels. Show a textual `Labels X/Y`
count when visible eligible labels exceed it; all remain searchable/selectable
through the virtualized list. No silent cap on model entities or constraints.
All visibility toggles are window-local, and hidden geometry remains solved.

## 6. Incremental solving and scalable interaction

### 6.1 Component graph and production numerical path

Build a dependency graph of unknown blocks and constraint residuals. Grounded
constants and shared numeric parameter values do not couple otherwise
independent components. A genuinely shared free master or group transform does.
Maintain entity-to-constraint reverse adjacency, source-to-derived dependencies
and graph revisions. Additions can union components; deletions must support
splitting, not leave a permanently growing union-find component.

Dirty a complete affected component, not merely a cursor neighborhood. Within
it, eliminate direct substitutions and fixed coordinates, and detect rigid
substructure where equivalence is certified. A solve must not modify independent
components. Maintain sparse Jacobian blocks with analytic derivatives for basic
geometry and checked local derivative generation for advanced curves. Validate
every derivative family against numerical differences in tests; production
must not finite-difference the entire sketch at every iteration.

Use a scaled damped trust-region least-squares step. Solve the augmented sparse
system `[J; sqrt(lambda)*D] dx = [-r; 0]` using the vetted numerical backend;
do not assume sparse QR is rank-revealing, or use normal equations as an
unqualified rank/DOF test. Preserve symbolic factorization and workspace while
the variable/incidence pattern is unchanged. Preserve solution guesses and
branch certificates across numerical updates; invalidate on structural edits.

Undamped rank/DOF certification is a separate job. For reduced bounded blocks
(at most 256 unknowns), use a vetted dense SVD rank analysis as the reference
path, including the effects of certified eliminations. M5.0 must identify its
actual backend API and record scaling, absolute/relative singular-value
thresholds, residual checks and sensitivity to threshold perturbation. Rank
close to the decision threshold is uncertain, not a rounded integer assertion.
For large coupled blocks, M5.0 must identify and demonstrate a named numerical
rank method and its concrete API; sparse incidence/matching or the existence
of a sparse QR solve is not such evidence. No method for that case is claimed
proven in this plan. Do not densify an unbounded block or implement a custom
factorization to fill the gap. An unavailable compatible method stops that
dependent path, as specified in §9.2.

Report `Regular / Certified`, `Singular / Not certified`, `Pending` and
`Unavailable` separately from convergence and residual validity. At a regular
interior solution, remaining unknowns minus certified undamped rank gives local
DOF. At a singular configuration, linearized nullity is only an infinitesimal
quantity, not proof of a finite motion: a point on two fixed tangent circles
is a required counterexample. Retained branch conditions and active bounds on
curve/contact parameters also restrict feasible motion. Detect boundary cases,
report active bounds and do not equate equality-Jacobian nullity with feasible
mobility unless that case is certified. Singular/uncertain cases remain editable
and may have valid profiles, but cannot display `Fully constrained` merely from
equation count or an apparently full-rank damped system.

Certification on the regular accepted S/C fixtures, and correct explicit
singular/boundary classification on the declared difficult fixtures, are
completion gates in M5.0/M5.15. This clarifies the numerical meaning of DOF;
it does not claim exact symbolic algebra or remove large-block diagnosis.

Conflict diagnostics first return a small verified conflicting/redundant set
and its affected entities. Finding a globally minimum conflicting set is not
required. Expensive diagnosis runs on demand with cancellation and its own
budget; it cannot rerun a full rank solve on each pointer move.

### 6.2 Scheduling, memory and publication

Reuse the existing geometry worker's owned-kernel lifetime; extend its message
enum with Sketch evaluate/query/regions/prepare requests. Replace the single
newest-waiting slot for Sketch service with bounded per-session work classes:
one frozen press query, one explicit final-validation/prepare continuation,
one latest draft evaluation/region continuation and one latest motion query.
Repeated motion replaces motion only; it cannot erase a press, cancel a valid
final confirmation or restart unrelated work. A newer draft supersedes its old
interactive solve; confirmation freezes the draft being validated. Keep input
capture immediate and preserve the pending-press contract of §3.3.

At each cooperative yield, process cancellation, release and invalidation
controls first. Visit ready classes in the order press, final, interactive,
motion, executing at most one bounded work slice per class before repeating;
round-robin sessions/windows within each class. Thus continuous presses or
motion cannot monopolize the worker or starve final validation. Long final,
region and diagnostic jobs retain continuations and yield, rather than owning
the worker until their whole job finishes. Queue capacity is bounded per live
session; session closure releases its queued work and continuations. Retain
the current prepared-authority/liveness checks. No additional per-event OS
thread, unbounded queue or nested numerical pool is introduced.

Every job/result certificate contains document instance, worker epoch, session,
base SketchRevision, draft revision, parameter epoch, support/reference
generations, purpose and request serial. Queries additionally contain camera,
layout, pointer and tool sequence. Reject stale results before any geometry,
selection, label, profile or document publication.

Check cancellation between bounded assembly batches and solver iterations and
before publishing. Prove that every noninterruptible numerical/kernel call
fits the cancellation budgets for accepted workloads. If it cannot, choose
an interruptible backend/algorithm in M5.0; merely ignoring a late result does
not prove that the worker can serve the next interaction promptly.
The M5.0 report must separately measure worst observed indivisible duration and
workspace for symbolic analysis, numeric factorization, solve, rank analysis,
intersection/root isolation, arrangement traversal, wire/face construction and
dependent extrusion. Record where cancellation/yield can actually occur and
reserve headroom for release/cleanup within the 100 ms cancellation budget.
Measure complete queue-to-service latency too: many short calls may still
starve a click. A five-second C1 solve cannot be one five-second worker call.
If a required pinned call cannot yield or fit the budget, stop its affected
path with evidence; another backend/dependency outside D5 needs a human decision.

Input capture and cancellation are immediate on the app thread; it never waits
on worker joins, factorization, full arrangement rebuilds or GPU readback.
Work budgets yield honest `Solving`/`Checking profiles` text with the last
valid geometry. A cursor target/primitive ghost is clearly provisional.
Final confirmation waits asynchronously for full validation; approximate
interactive geometry cannot become a committed accepted result.

Keep draft/journal updates proportional to touched blocks. Reuse equation,
spatial and render buffers. Bound caches and worker-owned retained generations;
release tokens on Cancel, undo, document replacement and failed publication.
Memory pressure returns a typed resource-budget outcome before large
allocation. Do not silently drop constraints, reduce geometric precision or
truncate profile output to obtain responsiveness.

### 6.3 Indexed queries and rendering

Use world/plane-space conservative analytic bounds in an R-tree; update changed
entries only. Expand a query by the screen tolerance converted through the
actual projection, then perform exact curve-distance/parameter checks on
candidates. Perspective/oblique views need a conservative projected envelope;
test their completeness, not just center points. CPU curve queries and region
analysis are separate from the ASCII grid's O(1) lookup.

Bulk-load on import/initialization, maintain local edits incrementally, and
rebuild only when measured degradation warrants it. Record candidate counts;
coincident boxes can force linear queries. No O(1) geometric-query claim.

Display packets preserve analytic IDs and use view-error-bounded tessellation,
with at most 0.25 physical-pixel curve deviation in the admitted camera range.
Cache by curve revision and camera/scale class, not by unrelated UI state.
Packet sharing and dirty-range uploads avoid O(N) work on the event loop.
Cull offscreen geometry and annotations. Preserve event-driven redraw: no
continuous idle loop just because an active Sketch exists.

## 7. Exact regions, open wires and profile identity

Closure, constraint completeness and profile validity are separate properties.
An under-constrained sketch can contain a valid closed region; a fully
constrained sketch can remain open. A closed region can coexist with open
branches elsewhere. Construction is excluded; ExternalReference is excluded
unless explicitly converted into participating profile geometry.

### 7.1 Arrangement pipeline

1. Validate participating curves, parameter intervals and finite support frame.
2. Query candidate intersections through conservative analytic bounds.
3. Compute geometric intersections, tangencies and coincident intervals through
   kernel-owned curve algorithms; classify multiplicity and uncertainty.
4. Construct virtual curve pieces at intersections with original entity ID,
   oriented parameter interval and contact provenance. Do not rewrite the
   user's entity definitions or dimensions just to construct a face.
5. Build a plane embedding/half-edge representation and sort incident directions
   with robust predicates and deterministic ties. A degree-two-endpoint graph
   alone is insufficient at T/X junctions, circles and tangencies.
6. Extract bounded cells and ordered boundary components; discard the unbounded
   exterior. Track nesting and material-side semantics for holes/islands.
7. Validate chosen cells, construct BrepKIT wires/faces with correct orientation,
   and map topology/region generations back to the source entities.

Retain analytic arcs/conics/NURBS throughout. Polygonal display chords may help
broad-phase queries but cannot decide exact topology. Certified subdivision
with error bounds/root isolation is permitted inside kernel algorithms; if
precision cannot classify a near-degenerate contact, report uncertainty and
block only the affected profile. Never silently snap or heal that contact.

Before depending on those operations, M5.0 records a capability matrix for all
unordered intersection pairs among segment, circle/arc, ellipse/arc, rational
quadratic conic and Bezier/B-spline/NURBS families, plus self-intersections and
partial/full coincident intervals. Separate transverse roots, tangency,
periodic seams and overlaps: testing only ordinary crossings is insufficient.
For offsets, cover each admitted curve family, open/closed chains, each join
policy, holes and self-intersection cleanup. For topology, cover ordered wires,
oriented faces with holes and their actual M4 extrusion. Text contours use the
same line/quadratic/cubic cases; this adds no new tool or curve family.

Each matrix entry names the pinned API or the required complementary algorithm
inside the scoped kernel Sketch modules; its error/root-isolation guarantee,
tolerance/frame conversion, cancellation points, workspace bound and adversarial
fixture; and status `Proven`, `Unproven` or `Unsupported`. The existing BrepKIT
`offset_wire_with_join` calls `face_polygon`; that path alone is not evidence
of certified curved offsets. A proposed complementary algorithm is not green
until independently checked. Missing capabilities stop only dependent slices;
they do not authorize screen-chord faces, a second kernel, an upgrade or silently
dropping advertised curve tools. See §9.2 for the required evidence disposition.

Cache intersections by source IDs/revisions and bounds. Editing one curve
invalidates its old/new contact neighborhood and dependent regions; region
connectivity may propagate farther than the edited bounding box. Fall back
to a worker-side complete rebuild when invalidation cannot be proven local.
Record that rebuild and keep the app responsive. Intersections can be O(N²)
in number; no spatial index can remove the cost of materializing that output.

### 7.2 Required semantics for difficult cases

| Case | Required result |
|---|---|
| Nearly closed gap | Open with localized endpoints/gap measurement; repair requires explicit accepted coincidence/heal action |
| Zero-length line, zero-radius circle, collapsed spline derivative | Entity-specific degeneracy; prevent invalid face, retain editable source |
| Coincident/overlapping intervals | Preserve all source provenance; collapse identical coverage only in derived topology; never erase source constraints silently |
| X/T junction | Derive all finite bounded cells and retain unrelated open branches |
| Circle without explicit endpoint | Valid periodic loop with a virtual seam that is not a new user point |
| Tangency/contact at one point | Distinguish adjacent regions from a self-touching nonmanifold boundary; do not manufacture a bridge |
| Figure-eight/self-intersection | Select individual bounded arrangement cells; reject a requested self-intersecting face boundary |
| Nested loops | Default minimal bounded cells are separately selectable; outer cell contains its immediate holes; islands remain separate cells |
| Adjacent selected cells | Derive the regular union, removing internal shared boundaries; retain only valid connected faces or explicit multiple outputs |
| Point-touching selected cells | Keep separate outputs if each is valid; reject a single requested nonmanifold face |
| Tiny details at large coordinates | Local normalization plus dimensional predicates; report unresolved tolerance instead of dropping a hole |
| Advanced curve near-tangency or overlap | Certified classification or an explicit uncertain-profile diagnostic; never an approximate committed face |

### 7.3 Lineage and downstream CAD

Preserve a RegionId only when there is a unique, verified correspondence of
oriented source boundaries and contact topology. A split/merge yields new IDs
with predecessor lineage. Spatial area/centroid or region-list position alone
is not persistent naming. Ambiguous downstream references require reselection.

Extend M4 ProfileRef with explicit Sketch region/wire references and preserve
existing face/wire/edge behavior. Closed selected regions feed Solid Extrude;
open chains and region boundaries feed Surface Extrude according to its exact
profile contract. A Sketch feature is not a fabricated solid or an arbitrary
thin sheet. Exposed scene mesh is a display projection, never profile authority.

On Sketch edit, recompute dependent history in a private candidate. A failed
Extrude/reference update cannot commit half a Sketch/history change. OK either
publishes the complete validated candidate or leaves the draft and previous
document intact with the affected dependency diagnostic.

### 7.4 Candidate-local repair of dependent Extrude profiles

A split/merge or ambiguous lineage opens a repairable diagnostic in the Sketch
session; it must not force the user to discard the Sketch to edit the committed
Extrude. The app retains one private candidate containing the Sketch changes,
dependent replay results and explicit reference substitutions. The model owns
the substitutions (dependent FeatureId/field, old qualified reference, new
candidate-qualified reference); the worker validates their geometry and history
order. The committed document and its selection IDs remain unchanged.

1. Sketch OK runs final validation/replay. On a broken profile, show the
   dependent operation/field and reason in the diagnostics list, with
   `Repair profile`, `Return to Sketch` and the whole-session Cancel action.
   Do not close the session or publish a partial candidate.
2. `Repair profile` opens the existing single-reference Selection grammar
   against the candidate's certified eligible regions/wires. Bind the request
   to session, candidate revision, dependent FeatureId and field. Respect the
   current M4 cardinality, solid/surface rules and history ordering: no new
   multi-profile Extrude or arbitrary downstream-operation editor is added.
   Never choose a split successor automatically. A valid regular union is
   eligible only when it meets that existing profile contract.
3. Selection OK stages the explicit replacement in this private candidate and
   replays affected dependents; it never accepts a document or closes Sketch.
   Selection Cancel/Escape retains the prior repair state and returns focus
   to the originating diagnostic. A later Sketch edit invalidates stale
   candidate selections and revalidates retained substitutions by identity.
4. If rebuilding still fails, retain the Sketch and repair choices with the
   specific failing dependency diagnostic. Return to Sketch allows further
   correction. A concurrent relevant commit invalidates the whole preparation
   and uses §8's conflict/rebase policy; repairs cannot overwrite newer history.
5. Once all dependencies validate, explicit Sketch OK prepares/accepts the
   entire candidate through the existing one-shot authority/token handshake.
   One document journal transaction includes Sketch edits and all accepted
   profile substitutions. Undo/redo restores them together with fresh result
   generations. Whole-session Cancel discards both, following §8.

Document these states in the already planned Sketch diagnostics/profile
references under D2; reuse their list/Selection controls and focus semantics
from README_UI §§19–21. No changes to Extrude artwork or additional panel
framework are required. Acceptance covers split, merge, hole deletion, cancelled
reselection, a failed rebuild after reselection and concurrent invalidation.

## 8. Transactions, multi-window behavior and history

Entering Sketch creates a window-local draft and captures the insertion/edit
target and its source generations. Only one terminal owner accepts OK/Cancel.
Use the existing prepared-document/worker-token handshake: commit exactly once,
then deliver/acknowledge UI closure; publication retries do not repeat a solve
or Model mutation. Preview IDs are never promoted into committed selection IDs.

Local undo stores SketchDelta transactions, not full-scene snapshots. A drag
coalesces pointer updates into one delta. Compound tools create geometry,
constraints and parameter definitions atomically. Whole-session Cancel discards
only the session-private geometry, parameter definitions, deltas, repair choices,
queued work and retained preview tokens. Cancel never replaces the global
document/registry, publishes inverse edits or resets their revision/ID counters.
Every change accepted meanwhile in another window (relevant or unrelated)
remains intact. If no external commit occurred, the committed document/registry
equal the entry values because this session never mutated them. Otherwise the
closing window projects the current committed document, not its saved entry
copy. Late results cannot revive the closed session or release another session's
committed geometry. Existing camera/workbench restoration follows D3 only.

Document undo/redo must cover accepted Sketch insert/replace/delete and every
other mutation that can occur between them; never skip intervening M4 geometry
or tree/parameter actions. Introduce a typed model journal using inverse edits
and shared immutable recipe blocks. Geometry re-evaluation is worker-side and
accepts as a new document revision; old geometry/request tokens do not become
valid again when undo restores logical IDs. Preserve checked ID high-water
marks so a later new operation cannot reuse an undone entity's identity.

Two windows may hold drafts. A relevant source Sketch/support/parameter/history
change invalidates confirmation in the other window, preserving its draft with
`Source changed`. Offer explicit Reload, Rebase or Cancel using documented
dialog controls. Rebase replays the local delta log into the current source,
validates every identity/reference and reruns final evaluation; conflicts are
reported, never resolved by last-writer-wins replacement. Candidate-local repair
substitutions are replayed and revalidated too; an ambiguous successor returns
to §7.4 instead of retaining an obsolete selection. Unrelated metadata
updates may be recertified only after explicit source-dependency validation.
Draft geometry, active tool, selection, visibility and annotation layout are
not broadcast as another window's live editing state.

Deleting a referenced entity/constraint, changing support, suppression and
conversion from linked to independent geometry require previewed dependency
effects and one undoable operation. Deleting a Sketch with dependents is
blocked unless the user explicitly accepts a listed dependent deletion set.

## 9. Exact implementation scope and delivery sequence

The lists below are the permitted future implementation scope after approval.
File splitting within these named modules is routine; a new crate, UI framework,
kernel version or ownership boundary is not.

| Area | Existing files to extend | New modules/tests |
|---|---|---|
| Model | `crates/newell-model/src/{lib,geometry,parameters,formula}.rs` | `src/sketch/{mod,entity,constraint,dependency,delta,profile}.rs`, `src/history.rs`; `tests/it/{sketch,sketch_history}.rs`, register in existing `tests/it/main.rs` |
| Kernel | `crates/newell-kernel/{Cargo.toml,src/lib.rs,src/profile.rs}` | `src/sketch/{mod,curve,solver,rank,query,regions,text}.rs`; `tests/it/{sketch_solver,sketch_regions,sketch_curves}.rs`, register in main |
| App | `crates/newell-app/src/{app,lib,document,geometry_worker,parameters,selection,navigation}.rs` | `src/sketch/{mod,session,tools,inputs,publication}.rs`; `tests/it/{sketch,sketch_sessions}.rs`, register in main |
| UI | `crates/newell-ui/src/{lib,action_box,selection,numeric,menu/mod,shell/mod,shell/data,shell/compose}.rs` | `src/sketch/{mod,panel,tools,diagnostics}.rs`; `tests/it/sketch.rs`, reference-backed snapshots and main registration |
| Renderer | `crates/newell-render/src/{lib,renderer,scene}.rs` | `src/sketch.rs`, `src/sketch.wgsl`; `tests/it/sketch_gpu.rs`, main registration |
| Dependency integration | Existing workspace/member manifests, `Cargo.lock`, `deny.toml` only if an audited compatible dependency needs its exact existing-policy inventory entry | No weakening of license/advisory policy; no new build-time geometry generation |
| Approved specification alignment | `ARCHITECTURE.md`, `docs/ui_design/README_UI.md`, `Sketch_Bar.txt`, `Action_Box_Sketch.txt`; `Description.md` roadmap, `docs/backlog.md` | Future ADR `0008-planar-sketcher.md`; `docs/ui_design/{Sketch_Tool_Inputs,Sketch_Constraints,Sketch_Diagnostics,Sketch_Profiles}.txt` |
| Performance/validation | Existing manifest benchmark registration where needed | `crates/newell-kernel/benches/sketch.rs`, `crates/newell-app/benches/sketch_interaction.rs`, generated deterministic fixture definitions, `docs/validation/M5-sketcher.md` |

The specification-alignment row requires explicit D1–D3 approval and an
authorized documentation amendment stage before dependent production work.
Architect does not modify these authoritative inputs under this task.
Do not edit unrelated snapshots, Extrude artwork, accepted ADRs, assets or
earlier milestone plans to make M5 appear compatible.

### 9.1 Sequential slices

| Slice | Implementation deliverable | Exit evidence |
|---|---|---|
| M5.0 — Feasibility and requirements | Preserve revision-2 dossier; execute separately authorized recovery packages §9.3 A–G to complete §9.2 evidence | All mandatory feasibility gates proven for their admitted workloads; negative investigation recorded without checking completion; no automatic later slice |
| M5.1 — Model/session/journal | IDs, shared document/parameter/formula chunks, support/frame, Sketch feature, typed edits, local/global history and versioned DTO | Concurrent-commit preservation on Cancel, identity, undo/redo, round-trip and source-certificate tests; bounded copies with retained generations |
| M5.2 — Incremental numerical foundation | Dependency graph, sparse residual blocks, small/large basic-geometry solves, scaling, bounded service classes and cancellation | Analytic-derivative tests, independent-component invariants, dense-oracle comparisons, phase cancellation/fairness and first S1/S2 results |
| M5.3 — Editing shell and navigation | Canonical panels, support picker, plane camera, input arbitration, Select, numeric/formula tools, basic indexed queries | Reference snapshots and real event-sequence tests including scale/layout/modal cancellation |
| M5.4 — Basic draw and constraints | Points, lines, rectangles, polygons, circles/arcs/slots; standard dimensional/geometric relations | Draw/drag/cote workflow; no stale captures, branch flips or auto-constraint duplication |
| M5.5 — Regions and selection | Exact arrangements, holes/islands, open chains, selected-cell unions and repair diagnostics | Full degeneracy corpus, deterministic provenance and incremental-vs-full extraction equivalence |
| M5.6 — Real CAD integration | Prepared Sketch commit and Sketch region/wire consumption in M4 Extrude; downstream replay and candidate-local profile repair | Draw -> constrain -> extrude -> edit -> repair -> regenerate; one atomic journal action, preserved draft on failure and complete private-state discard on Cancel |
| M5.7 — Modify | Trim/extend/split/fillet/chamfer/offset and relation transfer | Exact/certified curve cases, invalidation preview and atomic undo |
| M5.8 — Patterns/transforms | Copy/move/rotate/scale, mirror and associative linear/circular patterns | Master/instance relations, detach, bulk operations and large-pattern memory behavior |
| M5.9 — Advanced curves | Ellipse/arcs, bounded conics, Bézier, fit spline and exact control refinement | Exact geometry fixtures, parameterization stability and profile construction without display chords |
| M5.10 — Advanced constraints/diagnostics | Normal/curvature, spline tangency, rank/DOF certification, redundant/conflicting set diagnostics | Independently checked Jacobians, singular cases, invalid/nonconverged distinctions and large-component rank cases |
| M5.11 — References | Projected/Intersect/Insert, linked/copy modes and dependency repair | Upstream edits, cycles, missing-generation handling and plane transformations |
| M5.12 — Text | Supported text entry, exact font contours, rigid placement/scale and conversion | Holes such as O/B/8, unsupported repertoire diagnostics and text-profile extrusion |
| M5.13 — Large-sketch usability | Filtered/virtual lists, label placement, search/isolation, explicit label counts and slice/grid controls | Dense view, overlapping targets, keyboard accessibility and both themes at all k values |
| M5.14 — Multi-window and stress | Conflicts/rebase, late completions, history/parameter integration, memory and worker fairness | Adversarial scheduling tests and cross-window manual acceptance |
| M5.15 — Milestone acceptance | All advertised tools, benchmark classes, required workspace checks, GPU checks and physical-device review | Signed capability/performance report; no mocks or deferred full-scope tools presented as complete |

Every slice includes its own tests and performance observations. Later slices
do not authorize postponing incremental storage or cancellation from M5.1–M5.2.
If M5.0 exposes an upstream missing operation or unavailable sparse rank method,
continue independent explicitly authorized M5.0 packages but stop the affected path.
No permission to start M5.1–M5.15 follows from a completed investigation. Report the exact
required upstream/API change for a human scope/dependency decision. No kernel
upgrade, alternate B-Rep engine or loss of accepted tool coverage is automatic.

### 9.2 Required M5.0 evidence and dependent-path disposition

M5.0 has an executed revision-2 dossier with unsatisfied exit criteria.
Revision 3 plans the remaining Executor recovery work; no experiment is run
or authorized for execution by this Architect-only request. Record its evidence in the already
scoped `docs/validation/M5-sketcher.md`, with the existing benchmark/test modules
and deterministic fixture definitions. It must contain:

- A numerical dossier naming the bounded dense rank reference and large-block
  rank algorithm/API, thresholds and regular/singular/boundary semantics from
  §6.1. Compare known analytic solutions and dense reference results on
  overlapping small cases; include duplicated equations, rigid freedoms,
  toggle configurations, a point on two tangent fixed circles, active contact
  bounds, and the C1 chain and grid as separately recorded traces.
- A worker dossier with indivisible-phase durations, workspace/cleanup costs
  and queue-to-service measurements from §6.2. Inject Cancel in every listed
  expensive phase, immediately submit a frozen click and an edit in another
  window, and record cancellation acknowledgement, release and both subsequent
  service times. Run sustained motion plus final confirmation to prove neither
  frozen presses nor confirmation starve. Fake scheduling tests prove ordering;
  they do not replace real-backend latency measurements.
- The operation-family matrix of §7.1, including tolerance mapping from §5,
  parameter/orientation provenance and independent expected topology/error
  checks. Include near/exact tangencies, partial overlaps, periodic seams,
  open branches, three nested loop levels, curved offsets with self-intersections
  and a small hole. Repeat at the origin and translated to large representable
  coordinates (including `1e9 mm`), recording resolution limits rather than
  inventing a universal scale guarantee. Validate face construction and actual
  extrusion, not merely intersection points or a displayed contour.

For each required capability, record `Proven`, `Unproven` or `Unsupported`, its
dependent slices and the concrete remedy. Availability of an API/feature flag
is not proof. Compatible local implementation details within the existing
scope remain Executor choices; a missing large-rank method, incompatible
indivisible call or missing certified curve operation requiring another
dependency, kernel change or specification exception is a precise human
decision. Stop that dependent path and continue independent authorized work;
do not substitute dense global solving, polygonal profiles or weaker budgets.
M5 completion remains contingent on all originally advertised coverage and
accepted classes; this report does not redefine a failed class as accepted.

### 9.3 Revision-3 M5.0 recovery packages

These are sequential packages inside M5.0, not new milestones or permission to
implement the complete Sketcher. Future Executor may resolve ordinary choices
within a specifically authorized package. Record every run/configuration and
negative result in the existing validation report, preserving earlier evidence.
Use `Proven / Unproven / Unsupported` per capability and workload. A failed
mandatory gate leaves M5.0 open even if the investigation itself is complete.

#### A — Reestablish the evidence baseline

Inspect staged/unstaged/untracked inputs on main and preserve them. Reconcile
historical failure reports with current code; run §11 checks with snapshot
updates disabled only as Executor. Separate unrelated failures and M4 prototype
acceptance from M5 evidence. Do not repair unrelated navigation/UI work merely
to obtain green checks. Record M4.D1/M4.D2 intersections with each M5 profile
fixture: supported M4 paths must actually work, while any required M5 path
falling into a deferred class remains blocked until proven, not exempted.

Record actual CPU/RAM/OS/compiler/profile/power mode, worker/numerical thread
counts, fixtures, parameters, warm-up, percentiles and memory methodology.
Preserve the older M4/Low-Power/Rust-1.99 observations as characterization.
Acceptance timings require §10 reference conditions/Rust 1.88; Pi 5 results
are separate. Missing hardware yields missing evidence, not a new weaker budget.

#### B — Establish the numerical method and rank contract

1. Retain faer 0.22.6 as the solve baseline. Compare safe public AUTO,
   FORCE_SIMPLICIAL and FORCE_SUPERNODAL paths; prefer the demonstrated
   forced-supernodal configuration with preallocated index/value/scratch
   storage. Measure assembly, symbolic, factorization, solve, rank, cleanup
   and full job separately. Include the undamped and augmented systems and
   structural changes, not just one well-conditioned linear right-hand side.
2. Keep dense SVD at <=256 reduced unknowns as reference, with the §6.1
   dimensional residual/scaling and threshold-sensitivity policy. Recheck
   duplicated equations, contradictions, rigid freedoms, toggles, tangent fixed
   circles and active bounds. Preserve uncertain/singular/boundary outcomes.
3. Inventory established pure-Rust large-rank candidates or an upstream faer
   extension: exact package/release or proposed upstream API, numerical method,
   rank threshold/error or reliability assessment, rank-deficiency handling,
   workspace, cancellation boundaries, maintenance, licenses, transitive MSRV
   and supported-platform feasibility. Use the revision-5 comparison and selected
   method in §9.5 V2; do not restart an open-ended library search. A contribution
   to an established numerical library is research, not permission to write a private Newell factorization.
4. For the named faer0.22.6 vendored extension, follow §9.5 V1–V3 after
   the specified documentation prerequisites and explicit execution request;
   do not ask again for generic permission to develop this approved direction.
   Additional numerical crates, upstream baseline/version changes or different
   algorithms outside that package need a concrete revised scope. Failure of
   the Rust method keeps the package blocked; no FFI/SuiteSparse fallback.
5. Compare candidate rank/null-space residuals against analytic cases and
   bounded dense SVD on overlapping chain/grid cases, including scaled and
   nearly dependent variants. Then exercise both C1 chain and grid with at
   least 10,000 free unknowns, plus rank-deficient and X1 admission cases.
   Structural rank, damped rank and a solve success flag cannot certify DOF.

Exit: a named vetted method/API with demonstrated regular rank reliability,
explicit uncertainty/singularity/bound handling, bounded workspace and §10
latency/cancellation evidence. Fast QR solves alone cannot close this package.

#### C — Correct BrepKIT contacts, overlaps and parameter provenance

Use the existing exact NURBS corpus as executable failures, not API names as
support: missed parabola tangency/near crossings, periodic seam, figure-eight
nonadjacent crossing, incorrect returned parameters and partial/reversed overlap
watchdogs. Add independent expected roots/intervals, both input orders,
orientation reversal, reparameterized/subdivided coincident spans, near-but-
distinct curves, multi-span knots and origin/1e9-translated cases. Reevaluate
returned parameters on both original curves; validate multiplicity/contact and
interval maps as well as positions. Cover all 15 unordered family pairs of §7.1.

Plan targeted upstream corrections with typed uncertainty, checked geometric
versus parameter error, work/workspace bounds and resumable cancellation or
proven bounded indivisible calls. Sampled proximity cannot certify overlap;
recursion exhaustion must not manufacture a hit. Independent root isolation or
analytic proofs are required where the current heuristic is insufficient.

**Patch/pin procedure:** §9.5 now specifies the named BrepKIT4.2.2 vendor
source and local correction process. Keep installed registry sources untouched;
prepare minimal failures and guarantees in the existing validation report.
The earlier revision-4 request authorized development/testing of this exact
source scope and V0 is applied; resumption now follows §12. No unknown
upstream revision is silently adopted. Normal application integration remains
separate from the non-default candidate experiment. External issues, pull
requests, publication and commits require separate human authorization.

Exit: corrected operations pass independently checked contact/overlap/provenance
cases, through exact wires/faces/extrusion, with actual worker cancellation
budgets. An untested upstream release or a killed subprocess is not evidence.

#### D — Prove exact/certified offsets, regions and scale conversion

Treat this as substantive geometric algorithm work, not a wrapper around
`face_polygon`. Implement/prototype within the scoped kernel Sketch modules:
analytic offsets where exact; otherwise bounded-error curve representations
with a proven geometric deviation bound and explicit unresolved cases. Curved
offsets need not remain in the same polynomial degree/curve family; returned
geometry must stay within the permitted exact/certified contract. No display
chords or unbounded sampled fit may become profile authority.

Test each admitted family, signed sides, all three joins, open/closed chains,
small holes, offset singularities/cusps and self-intersection cleanup. Then
construct exact arrangements with T/X branches, point contacts, seams, overlaps,
three nested levels and regular selected-cell unions; preserve oriented source
parameter provenance. A caller-preordered face with holes proves extrusion,
not automatic arrangement. Complementary algorithms stay kernel-owned and
use BrepKIT topology; no second B-Rep engine or new generic geometry dependency
is implied. An additional library requires its own concrete adoption decision.

Retain §5's absolute/relative/angular/parameter distinctions. Compute near the
support origin and record conservative conversion/rounding error through every
world-space call. Repeat representable detail fixtures at 1e9 mm, including the
known .1 mm hole; explicitly reject unresolvable details instead of widening
tolerances. f64 world resolution is not repaired merely by translating input
back after an operation. Document the proven admitted domain and downstream
conversion limits. A representation change beyond existing local Sketch frame
plus checked world output is an architectural decision, not an automatic fix.

Carry representative exact profiles, including curved/text holes, oriented
faces and translated details through the real M4 arena and worker replay.
Separate geometric error, topology validity and approximate volume/mesh error.
Expand O/B/8 evidence to the advertised printable ASCII and transforms needed
for M5.12, preserving raw font units before f64 scaling.

Exit: §7.1 operation matrix and §9.2 corpus are proven on their stated domain,
with independently checked topology/error, cancellation and real M4 consumption.
Any remaining required family/path prevents dependent acceptance.

#### E — Real-worker responsiveness feasibility harness

Add a minimal non-default/test-only harness using the actual geometry worker,
Controller input routing and real backend operations. Reuse current cancellation
and test-support seams where applicable. It may exercise synthetic Sketch input
DTOs/continuations, but must not create a production Sketch session, document
feature, menu, renderer pipeline or persistent journal. Keep Newell-owned inputs
and native topology inside the existing kernel boundary.

Probe a bounded per-session service pump with frozen press, final, latest draft
and latest motion slots, §6.2 priority/yield order and round-robin windows. The
probe seam must exercise the same worker execution/cleanup controls, not an
independent toy thread. Default runtime behavior remains unchanged; production
adoption belongs to an explicitly authorized later slice. If a representative
phase cannot yet be executed, record it missing rather than substituting a tiny
SVD. New test-only kernel hooks may observe entry/return, never fake cancellation.

Inject Cancel during real assembly, symbolic/numeric/solve/rank, root isolation,
arrangement traversal, construction and dependent extrusion. Immediately enqueue
an actual frozen input request and a second-window edit. Timestamp event receipt,
acknowledgement, cancellation poll, call return, cleanup, queue admission,
service start, completion and publication separately. Include request races,
workspace exhaustion, long final checks and continuous motion; assert bounded
queues, eventual press/final service and correct stale-result rejection.

Exit: accepted-case acknowledgement <=16 ms, worker release <=100 ms including
cleanup, and §10 query/input/visible-result budgets on applicable real paths.
Any noninterruptible call exceeding its budget requires a different bounded
algorithm/API inside the approved scope, not extra per-event threads, late-result
suppression or process killing. Fake scheduling tests complement, not replace,
real timings. Full Sketch GPU/visual acceptance remains M5.15 and is not claimed
from the harness; missing applicable M5.0 service evidence stays Unproven.

#### F — Mixed workloads and evidence reconciliation

Instantiate actual mixed geometry/constraints for S1/S2/S3 counts and real
connected nonlinear C1 chain/grid cases; keep the old scalar traces clearly
labelled. Record unknown/equation/block/nonzero/intersection/region/visible counts,
500 warmed updates, all §10 percentiles, residuals and workspace/allocation data.
Separate genuinely global dependencies and pathological quadratic output.
Measure setup, independent phases, full worker path and retained result cleanup.
Use analytic/topological references and bounded dense comparisons; retained
source/render/parameter copy checks keep their §10 M5.1–M5.2 adoption gates.
Do not require an entire production UI merely to obtain M5.0 backend evidence,
and do not relabel unimplemented full-path/camera/GPU checks as passed.

#### G — Close or explicitly leave M5.0 open

Update only the validation dossier and, only if all M5.0 exit gates pass, its
single backlog item. Required evidence includes numerical rank, exact operation
families/scale conversion through extrusion and real cancellation/service proof.
List every still-unproven mandatory capability with an exact blocker/remedy;
a finished negative investigation is not successful M5.0 completion. Preserve
M5.15's complete hardware/UI/GPU acceptance obligations and M4 prerequisites.
No later slice, new agent, commit or publication starts automatically.

### 9.4 Recovery file scope and interfaces

A future explicitly requested Executor recovery uses the existing §9 scope,
restricted as follows; this Architect revision writes **this plan only**.

| Package | Permitted future files / ownership |
|---|---|
| A/B/F | Kernel `Cargo.toml`, `benches/sketch.rs`, `tests/it/{main,sketch_solver}.rs`; bounded prototypes in `src/sketch/{mod,solver,rank}.rs`, registration in `src/lib.rs`; validation report. Named vendor/feature/lock changes follow §9.5; unrelated candidates need exact adoption approval. |
| C/D | `src/sketch/{mod,curve,query,regions,text}.rs`, `src/{lib,profile}.rs` for non-default/test-only seams; `tests/it/{main,sketch_curves,sketch_regions}.rs`; kernel benchmark/report. No registry, accepted ADR, asset or unrelated golden edit. |
| E/F | App `Cargo.toml` benchmark/test registration, `benches/sketch_interaction.rs`, `tests/it/{main,sketch_sessions}.rs`, test-only seams in `src/{lib,geometry_worker,app}.rs`; kernel test-support seams in existing modules. No default application behavior or production Sketch UI. |
| G | `docs/validation/M5-sketcher.md`, only M5.0 status in `docs/backlog.md` when justified. |

Prototype contracts: kernel-owned numerical input/result includes scaled residuals,
rank reliability/state, source parameter intervals, typed uncertainty/resource/
cancel outcomes and workspace requirements; app-owned probe jobs carry the same
session/source/purpose certificates as §3.3. A resumable operation advances one
bounded work unit and returns progress or terminal output. Native backend types
never enter public ACL signatures. Reuse these minimal seams when later approved;
do not introduce a second solver architecture or generic job framework.

The named third-party source changes and Cargo overrides are explicitly
scoped in §9.5. Other dependencies, baselines or architectural changes require
an exact additional scope; they are not hidden inside this table.

### 9.5 Controlled vendoring and dependency development (revisions 4–5)

#### V0 — Authority and compatibility prerequisites

**Revision-5 status:** already applied by the explicitly authorized V0 stage:
architecture §§5.2/6 and accepted ADR0009 now establish the boundary below.
The following text records the original prerequisite; do not repeat the amendment
or ask for its authority again. This Architect stage changes neither file.

This Architect turn edits only this plan. Before any source import/override,
a separately authorized documentation stage must establish the following narrow
rules in `ARCHITECTURE.md` and a **new** ADR
`docs/adr/0009-m50-controlled-vendoring.md`. Preserve accepted ADR0004 verbatim
as the original baseline; the new accepted ADR must explicitly supersede only
its source-selection clause for the named local candidate. Do not silently
pretend that a path override of4.2.2 is identical to the registry implementation.
A future human instruction may authorize the documentation amendment and bounded
Executor stage together; an extra review cycle is not required.

- Architecture §5 forbids `unsafe` in Newell's codebase; §6 requires the Newell
  AGPL header/package license. Existing faer dependency sources contain upstream
  unsafe and MIT notices. Define `vendor/` as third-party dependency source,
  excluded from **Newell-authored** header/licensing and unsafe-code rules,
  while retaining original notices, provenance and dependency policy review.
  This is a scoped source-boundary clarification, not permission to add unsafe
  Newell code or remove upstream notices. New extension/correction code is safe
  Rust; no new unsafe block, FFI, C/Fortran build or BLAS dependency is permitted.
  Existing upstream unsafe changes require a separate reviewed scope; do not
  disguise them as formatting. The five Newell crates retain `unsafe_code=forbid`.
- Exact versions, kernel-only dependency declarations, Newell-owned public ACL
  types, error translation and golden regressions remain binding. Root Cargo
  patch entries are resolution directives only, never workspace geometry deps.
- Vendor packages retain their actual licenses (faer MIT; selected BrepKIT
  AGPL-3.0-only), copyright, license files and attribution. Do not relicense
  third-party Rust files or apply Newell's headers to them. Newell's own package
  licenses remain AGPL-3.0-or-later. Do not weaken `deny.toml` to pass an audit.

Until these authoritative changes exist, leave the dependent import/override
unexecuted and report the exact conflict. Approval of a plan alone cannot
rewrite binding architecture. This prerequisite does not reopen the already
approved Rust/no-FFI/BrepKIT-only direction or make an optional Challenger mandatory.

#### V1 — Import, identify and isolate the named candidates

The source baselines are the **published Cargo packages** faer0.22.6,
brepkit-math4.2.2 and brepkit-operations4.2.2. Import them into
`vendor/faer-0.22.6/`, `vendor/brepkit-math-4.2.2/` and
`vendor/brepkit-operations-4.2.2/` in the existing main checkout. Keep topology
and other BrepKIT crates on their current exact registry pins unless a later
concrete change is separately approved. No Git worktree, task branch, embedded
Git repository or separate execution checkout. Do not edit Cargo registry files.

For each imported package retain its shipped manifest/source/tests/license files;
use the published normalized manifest when original sibling workspace paths
would otherwise escape the package. Limit manifest edits to documented feature,
path and test registration changes. Imported benchmarks/dev dependencies are
not automatic permission for a new unreviewed dependency graph.

Add `vendor/README.md`, `vendor/manifest.toml` and per-package
`NEWELL_CHANGES.md`: record upstream package/version/download provenance,
published archive integrity, local patch IDs and file list, license/MSRV,
feature/build requirements, tests and maintenance owner responsibilities. This
is dependency supply-chain provenance, not a snapshot of the dirty Newell tree.
Keep original version numbers; local modifications are identified by the checked
in source plus manifest/patch ledger, never misrepresented as an upstream release.

Root `Cargo.toml` explicitly excludes the vendor directories from the workspace
and adds `[patch.crates-io]` paths for the three named packages. It must not add
BrepKIT to `[workspace.dependencies]` or another Newell crate. Kernel direct
BrepKIT pins remain `=4.2.2`; the ledger/ADR identifies their patched resolution.
faer remains `=0.22.6` with defaults off and `std,sparse-linalg` only initially.
Update Cargo.lock normally; retain all unrelated lock entries. Inspect metadata
for one consistent math/operations/topology type graph and no accidental
registry duplicate, source escape or changed transitive platform requirement.

First import unmodified sources and prove behavioral parity/golden equivalence.
Then add a non-default `newell-m50` candidate feature in the necessary vendor
packages and explicit `m50-feasibility` wiring in newell-kernel (and test/bench
app wiring only where needed). Candidate APIs/algorithms are additive, feature
isolated and not selected by normal application call sites. Cargo all-feature
unification must be tested; no reliance on a feature remaining disabled in tests.
Verify both default and candidate builds. A global path patch is not by itself
feature isolation: all default-call behavior must remain parity-tested.

#### V2 — Selected sparse numerical-rank method and qualification

**Decision:** retain faer0.22.6 forced-supernodal QR and reusable workspace for
linear solves inside the existing nonlinear orchestration. Develop a separate
safe-Rust **Foster–Davis SPQR_RANK adaptation**, using the accessible full paper
linked in §13, inside the named faer vendor package. Do not link SuiteSparse,
MATLAB or a native backend. This is numerical-library development, not a claim
that a drop-in pure-Rust sparse-rank package has been found.

##### Comparison and selection rationale

| Existing solution / method | Solve and rank capability | M5 disposition |
|---|---|---|
| Pinned faer0.22.6 sparse QR | Existing sparse solve baseline; inspected QR does not supply the required numerical-rank/reliability contract | Retain solve path and extend the numerical library; solve timing cannot stand in for rank timing |
| BrepKIT4.2.2 GCS DogLeg | Existing common-constraint reference; dense Jacobian and no cooperative cancellation in the inspected API | Bounded semantic oracle; insufficient for scalable advanced Sketch constraints |
| faer dense SVD / column-pivoted QR | Bounded dense numerical tools; global dense allocation defeats C1/S3 | SVD reference for <=256 reduced unknowns; never densify a large global Jacobian |
| [rsparse1.2.1](https://docs.rs/rsparse/1.2.1/rsparse/) | Documented sparse QR/least-squares and triangular solves; no demonstrated M5 rank-reliability/cancellation contract | No dependency switch: another solve API does not resolve the missing evidence; full dependency audit not performed |
| Structural matching, normal equations or iterative least-squares alone | Structural sparsity or a step/residual is insufficient to distinguish numerical dependence at the chosen threshold | May support ordering/diagnostics; cannot certify DOF or replace undamped numerical-rank analysis |
| Gu–Eisenstat strong RRQR | Accessible full paper with rank-revealing bounds; presented dense algorithms have worst-case O(m n²) work | Valid bounded reference/alternative research, but choosing it globally would leave a separate sparse algorithm and fill-control problem |
| Native SuiteSparseQR / SPQR_RANK | Existing sparse solver/rank research implementation; native dependency does not satisfy the approved implementation boundary | Algorithmic reference only; no FFI, build dependency or native runtime/oracle prerequisite |
| Foster–Davis sparse rank protocol ported to faer | Sparse-specific method with explicit reliability diagnostics; faer adapter and safe Rust implementation still required | **Selected development route**, subject to the unchanged gates below |
| Pierce–Lewis multifrontal RRQR | Formerly selected; full method was unavailable through the investigated access routes | Historical alternative only; obtaining this paper is no longer an execution prerequisite |

This choice preserves the measured solve investment and uses a sparse-specific
rank protocol rather than inventing a Newell factorization. Accessibility removes
the reading obstacle. It does not remove the development risk or establish a
performance guarantee. No listed replacement is asserted to satisfy all M5 gates.

##### Published method boundary

The accessible Foster–Davis paper specifies a rank-estimating sparse QR contract
(§2.1), subspace inverse iteration SPQR_SSI (§2.2), SPQR_BASIC reliability
assessment (§2.3) and implicit null-space construction SPQR_NULL (§2.4).
Its singular-value bounds include estimates: reported success is not an
unconditional mathematical guarantee. Use this protocol, retaining uncertainty
and failure outcomes. The full text, pseudocode and appendix are available at
the author-hosted link; Executor reads the relevant complete algorithms before
coding. These names identify methods, not Rust APIs already present in faer.

##### Newell implementation decisions and exact output boundary

1. Implement an additive `vendor/faer-0.22.6/src/sparse/linalg/rank.rs`, module
   registration and the smallest safe `qr.rs`/solver-interface changes needed
   for the contract. Replace revision4's proposed `rrqr.rs` destination; do not
   maintain two numerical implementations. Use existing safe kernels, storage,
   ordering and caller-owned scratch. Keep normal QR behavior unchanged.
2. **First prove the factor bridge.** Specify permutation conventions, compact
   orthogonal transforms, triangular/trapezoidal blocks, numerical column
   rejection and perturbation accounting required by the cited protocol. The
   existing unpivoted faer QR output is not automatically equivalent. Add safe
   numerical column deferral/refactorization support where necessary and test
   the decomposition on the original scaled matrix. Do not merely count R's
   diagonals or pass a damped augmented matrix as the constraint Jacobian.
   Reconcile any changed symbolic pattern with workspace admission before
   allocation. Missing safe access is a concrete implementation blocker.
3. Expose `RankQrSymbolic`, reusable `RankQrWorkspace`, bounded `advance`/cancel
   controls and `RankQrReport` (names may follow faer conventions). Report
   scaling/threshold, rank estimate, reliability classification, residuals,
   perturbation estimates, iteration/restart provenance and workspace usage.
   Implement rank validation before optional solution/null-space convenience
   APIs. No algorithm or code license is inferred from free reading access;
   retain attribution and independently check any reused source license.
4. `crates/newell-kernel/src/sketch/rank.rs` owns only threshold policy,
   translation to Newell-owned results and constraint/DOF semantics. No public
   vendor type escapes the ACL. Preserve §6.1 singularity, finite mobility and
   active-bound distinctions. Reliability estimates must never be relabeled as
   rigorous certificates. Existing certification states must retain that
   distinction; an ambiguous result cannot mark a sketch fully constrained.
5. Handle zero/all-zero rank, full row/column rank and both rectangular shapes
   explicitly; validate transpose and original-equation mappings. Retain
   null spaces as operators where materializing them would exceed the budget.
   Never allocate a global dense Q, J, R or full null-space basis. Dense SVD
   remains bounded to <=256 reduced unknowns; tall thin iteration work arrays
   may have a bounded block width, with their actual bytes charged to workspace.
6. Set work/block/restart/fill limits from the unchanged workload budgets;
   record deterministic initialization and independent restarts in probes.
   Exhaustion, nonfinite arithmetic, threshold ambiguity or unreliable estimates
   return typed uncertainty/failure. Do not change the semantic threshold to
   make a rank estimate pass. Regular accepted cases must actually succeed;
   universal uncertainty or premature admission rejection cannot close M5.0.

##### Qualification and performance, without relaxed requirements

- First validate the factor bridge independently: reconstruction/orthogonality,
  dropped-column perturbation accounting, permutations and safe workspace
  behavior. Then compare rank and null-space action with analytic and <=256
  dense-SVD references: duplicates, zeros, near dependence, rectangular systems,
  large/small nullities, scaling, column/row permutations and threshold sweeps.
  Include adversarial cases where diagonal counting gives the wrong answer;
  test reliability failures as well as successful rank reports.
- Next run regular and rank-deficient C1 **chain and grid**, each with >=10,000
  free unknowns after documented eliminations. Test nonlinear solve and undamped
  rank separately and together. Preserve actual S1/S2/S3 mixed workloads,
  original entity/equation counts, residual tolerances and all §10 budgets.
- Report assembly, symbolic changes, factorization, rank iteration, nonlinear
  iterations, cleanup and end-to-end worker latency separately. Reuse valid
  symbolic/storage state, not a rank report from a different Jacobian/version.
  Include rank work in the final-solve budget and in interactive work whenever
  its result is required for the published interactive diagnostic.
- Enforce C1 <=5 s, obsolete release <=100 ms, cancellation acknowledgement
  <=16 ms, and every S-class time/memory limit. Bound indivisible QR and rank
  work by evidence or make it resumable; cancellable outer iteration cannot
  hide an uninterruptible expensive factorization. Keep 500 warmed samples,
  actual worker fairness, hardware and retained-generation acceptance unchanged.

V2 is complete only with a qualified safe Rust implementation and reproducible
independent evidence. The selected port is not yet implemented. Excess fill,
reliability failure, unsafe-only access or missed timing keeps M5.0 open; return
that evidence rather than lower counts, omit rank work, add FFI or claim that
an accessible paper guarantees a feasible schedule. V3 geometry corrections
remain independent and mandatory; this method change does not qualify them.

#### V3 — BrepKIT corrections and certified geometry

Primary edit scope: `vendor/brepkit-math-4.2.2/src/nurbs/{bezier_clip.rs,knot_ops.rs}`
and new `src/nurbs/{certified_intersection.rs,certified_overlap.rs}` with module
registration; `vendor/brepkit-operations-4.2.2/src/{offset_wire.rs,offset_trim.rs}`
and new `src/certified_offset.rs` with module registration. Add focused tests in
those packages and Newell's already scoped curve/region probes. Do not change
boolean/meshing/tolerance defaults or unrelated APIs as incidental cleanup.
If a correction proves necessary outside these files, present its exact file,
failure and scope before extending this list.

Implement first the reproduced parameter-map/order, subdivision, seam and affine
interval faults with independent expected values, both operand orders and
orientation reversal. Add candidate certified APIs rather than globally replacing
normal application calls before qualification. General roots require analytic
proofs or conservative enclosures/root isolation with explicit multiplicity,
parameter maps and uncertainty. Overlap needs verified common geometry with a
parameter correspondence, not proximity samples. Depth/work exhaustion returns
unresolved/resource/cancel status and must not manufacture accepted topology.

Exact/certified offsets retain analytic line/circle cases; general conic/spline
results may change degree/representation but require a bounded geometric error
and checked topology of joins/self intersections. Candidate arrangement traversal
stays Newell kernel-owned in `src/sketch/regions.rs` and constructs BrepKIT
wires/faces. A geometry algorithm library is not automatically approved. If
higher precision/exact arithmetic is required, identify the exact pure-Rust
candidate/version/license/MSRV/feature graph and request that concrete addition;
no arbitrary new dependency is preapproved and no native arithmetic library/FFI.

Use existing C/D matrices without reducing families. Repeat contact/overlap,
periodic/self-intersection, all joins/open/closed, small holes/nesting/union and
translated representable details. Independently bound physical/parameter errors
and local/world conversion; preserve §5 and the known1e9 resolution limits.
Validate actual exact M4 extrusion/replay, supported M4 histories and transaction
atomicity. M4.D1/D2 deferrals remain blocked intersections of scope, not exceptions
for a successful Sketcher. Corrected root points alone cannot close C/D.

Every expensive phase exposes a bounded continuation or a measured indivisible
bound leaving cleanup headroom under100 ms. Do not replace this requirement with
extra threads or subprocess termination. Preserve cancellation and uncertainty
through the vendor API, ACL, worker certificates and final acceptance path.

#### V4 — Qualification, adoption and maintenance

V2 and V3 may progress independently within explicitly authorized scope while
one capability remains blocked; do not wait to build E's real-worker harness.
Exercise candidate operations and the bounded service pump on the **actual**
worker/input routes, including frozen queries, final confirmation and continuous
motion. Use partial backends only for the phases they really implement; list
missing phases. The recent preparation-only second-session test is a baseline,
not the required press-query/fairness proof. Preserve F's500 warmed mixed-workload
and retained-generation requirements and separate A/B hardware acceptance.

Produce one adoption dossier in `docs/validation/M5-sketcher.md`: exact vendor
ledger/patch IDs, reproducible build/feature commands, method/API contracts,
licenses/MSRV/advisories, upstream parity and patch regressions, golden geometry,
reference-hardware latency/cancellation/workspace, independent rank/error/topology
comparisons, coverage and every unresolved failure. A qualified candidate may be
accepted only for its demonstrated capabilities; any mandatory missing capability
keeps M5.0 open and blocks dependent production use.

V0 and the original experimental execution were explicitly authorized under
revision4; the current role switch and revision-5 resumption boundary are in §12.
Production switching/default feature changes require
a separate explicit adoption request against the concrete dossier, not renewed
permission for already authorized test-only work. Do not start M5.1 or mark
Sketcher implemented as part of dependency qualification. A new upstream baseline,
additional vendored package, new unsafe/FFI need or material license/MSRV change
requires an explicit new decision; no silent expansion.

Keep source deltas minimal and attributable; retain upstream regression tests
and add adversarial independent ones. Nominate maintenance ownership at adoption:
who tracks advisories/releases, rebases patches, maintains algorithm evidence and
supported platforms. Removing a local fork requires upstream parity of the actual
fixes and rerunning the corpus, not merely a newer release number. No automatic
upstream issue/PR, commit, push or publication. Proposals/patch descriptions can
be prepared locally; sending them still needs explicit authorization.

#### Additional exact future output scope

- `vendor/{README.md,manifest.toml}`; the three package trees above with original
  notices, `NEWELL_CHANGES.md`, focused tests and only V2/V3 algorithm edits.
- Root `Cargo.toml`, `Cargo.lock`; existing kernel/app manifests solely for
  exclusion, patch resolution and non-default feasibility feature/test wiring.
- Existing §9.4 kernel/app seams, tests/benchmarks and validation record; only
  the M5.0 backlog item if all exit gates truly pass.
- **V0 documentation already applied:** `ARCHITECTURE.md` scoped third-party
  boundary and accepted `docs/adr/0009-m50-controlled-vendoring.md`. Neither
  is modified by this plan-only stage; accepted ADR0004 is preserved.

All five Newell crates, MVC/ACL ownership, safety rules for Newell-authored code,
project licensing, tool coverage and original performance/accuracy gates persist.


## 10. Measurable responsiveness and scale acceptance

These are proposed acceptance budgets, not current benchmark results. Reference
A: Apple M1-class CPU, 8 GiB RAM, integrated GPU, 1920x1080 viewport, release
build, Rust 1.88, one geometry worker and no nested numerical thread pool.
Record actual model/OS/compiler/GPU/power mode. Reference B: Raspberry Pi 5,
8 GiB, 64-bit Linux and supported driver. Report B separately; do not claim
desktop-scale counts from desktop results alone.

Fixtures must specify entity/unknown/equation counts, largest connected block,
Jacobian nonzeros, intersection count, region count and visible geometry. A
headline entity count alone is not a result.

| Class | Required workload | Proposed A acceptance |
|---|---|---|
| S1 | 1,000 independent/mixed entities, about 3,000 scalar equations, largest free block <=128 unknowns | p95 incremental numerical update <=16 ms; final validation <=250 ms |
| S2 | 10,000 entities, about 30,000 equations, largest free block <=256 unknowns | p95 update <=30 ms; whole-sketch validation <=2 s |
| S3 | 100,000 entities, about 200,000 equations, bounded components <=256 unknowns | p95 local update <=50 ms; initial/final whole-sketch validation <=15 s; incremental Sketch peak memory <=1 GiB |
| C1 | 10,000 entities in one connected sparse chain/grid, at least 10,000 free unknowns | Whole solve <=5 s; interface remains interactive; obsolete-job worker release <=100 ms on accepted runs |
| X1 | Dense coupling, coincident bounds, many intersections, near-singular geometry | Enforce declared workspace budget, cancellable progress and explicit typed failure; no universal solve-time promise |

All measured updates include applying the edit and invalidating dependent
numeric caches; separately report packet preparation, query, region update,
upload and end-to-end input-to-visible-result latency. S-class local edits
touch a bounded component and contact neighborhood; separately measure edits
with global dependencies instead of assigning them the local budget.

Additional A requirements for the accepted classes:

- Event-loop input handling p95 <=4 ms and p99 <=8 ms; worker scheduling and
  snapshot handoff are included, expensive geometry is not on this thread.
- Certified selection-query p95 <=5 ms on nonpathological S1–S3 views; measure
  crowded candidate lists separately and expose ambiguity rather than pick an
  arbitrary item.
- Event-to-visible local feedback p95 <=100 ms for S1–S3, including worker
  queuing, query/solve, packet preparation and presentation.
- At least 60 FPS during navigation/interaction on the defined A fixtures;
  report p95 frame time <=16.7 ms, visible count, segment count and draw calls.
  Idle remains event-driven; a busy-solver status does not create animation.
- Logical cancel acknowledgement <=16 ms; worker release of superseded work
  <=100 ms where the accepted workload permits the operation. M5.0 must reject
  any backend that cannot meet these accepted-case cancellation requirements.
- After 1,000 edit/undo/redo/cancel cycles, live retained-generation count returns
  to baseline; settled memory growth <=5% after warm-up, with allocator peaks
  reported separately. Journals have explicit bounded retention accounting.
- A local edit of one S3 component must not evaluate residuals of independent
  components, clone all entity tables, or rebuild every spatial/render buffer.
  Verify with deterministic work counters, not wall-clock timing alone.

For the last requirement, run both a geometric drag and a formula-backed
dimension edit with local dependencies, then repeat while an older published
generation is retained. Count document/entity/constraint/parameter/formula
entries copied or visited, index entries changed, packet bytes prepared,
handoff bytes and allocations through visible publication. Include source
capture, draft field acceptance, message submission, certificate validation and
retained-generation release; solver-only timing cannot satisfy this check.
Work must follow touched blocks and actual dependents, not total S3 size.
Record separate traces for a genuinely global formula edit and a camera change.
These checks start in M5.1–M5.2 and cannot wait for the final stress slice.

Reference B initially gates S1 responsiveness and cancel/correctness using
separately recorded budgets; require p95 input <=8 ms, p95 local result <=150 ms
and interactive frame p95 <=33.3 ms on S1. Do not call this 60 FPS compliance
with the vision's aspirational low-end target; record the gap explicitly and
require human acceptance of any release-target exception. S2/S3 B measurements
are mandatory characterization, not a hidden promise of interactive solving.

Use at least 500 warmed interactions per latency fixture and report p50/p95/p99,
maximum, allocation bytes and full-solve convergence/residuals. Seeds and edit
traces are versioned. CI gates correctness/work counters and dedicated pinned
performance hardware gates time; do not use noisy shared CI timings as proof.

## 11. Test and acceptance matrix

| Area | Required independent evidence |
|---|---|
| Numeric correctness | Residuals in physical units, known analytical solutions, derivative finite-difference checks over multiple scales and orientations; bounded upstream GCS comparisons |
| Rank/diagnosis | Named methods/thresholds; rigid motions, duplicate equations, redundant-consistent sets, true contradictions, singular/toggle configurations, tangent-circle point and active bounds; distinguish infinitesimal nullity from certified regular DOF |
| Exact profiles | §7.1 capability matrix through face construction and extrusion; segments/arcs/circles/ellipses/conics/NURBS, holes, three-level nesting, overlaps, X/T contacts, tangencies, periodic seams, self-intersections, open branches and translated small-detail fixtures |
| Incremental equivalence | Same accepted solution/residual and same region topology as a deterministic fresh solve/extraction for seeded edit sequences; branch-equivalent solutions compared appropriately |
| Curve operations | Shape-preserving refinement, bounded-error offsets with certified limits, trim parameter provenance, curvature at joined endpoints and degenerate derivative rejection |
| Interaction | Real click-click, drag threshold, pending query, pointer exit, focus loss, resize, UI-scale change, menu/modal interception, Escape, Tab, Enter and native scroll/pinch event sequences |
| Transactions | OK once, candidate-local profile repair after split/merge/hole deletion, cancelled repair, failure after repair, atomic Sketch+repair undo/redo; Cancel discards private state without restoring global snapshots; transport failure, publication retry, late completion, duplicate callback, source replacement and checked-ID high-water preservation |
| Multi-window | Relevant and unrelated commits including parameter edits survive another window's Cancel; unrelated edit recertification, relevant-source conflict, successful/conflicting explicit rebase, repair-candidate invalidation, window close during work and no cross-window draft/visibility leakage |
| UI | Exact approved .txt snapshots; minimum 82x30 and larger grids, k=2..7, four theme/Color combinations, clipping and accessible text diagnostics |
| GPU | Offscreen/on-device curve/handle/dash antialiasing and clipping, occlusion, unchanged ASCII draw count and declared Sketch/navigation draw budgets |
| Scale | S1/S2/S3/C1/X1 traces, full-path geometric/formula edit counters with an older generation retained, memory ceilings, phase cancellation then click/other-window service, confirmation fairness and no idle redraw loop |

Required Executor baseline and final checks, snapshots never auto-updated:

```sh
cargo check --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
INSTA_UPDATE=no cargo nextest run --workspace --all-features --locked
cargo test --workspace --doc --locked
cargo test -p newell-render --locked -- --ignored
```

Use the documented cargo-test fallback when nextest is unavailable. Verify Rust
1.88, resolved dependency licensing/advisories and golden geometry for any
explicitly approved kernel change. Existing snapshots change only for the
approved Sketch/spec additions; no unrelated regeneration. After two failed
compilation repair attempts, stop and report the concrete error and attempts.

Manual acceptance, separately recorded by the human:

1. Starting with no external geometry, create a dimensioned rectangle with a
   circular hole; select the correct region and extrude it through M4.
2. Reopen the sketch, change a formula-backed dimension, inspect the regenerated
   body, then undo/redo both local and committed edits.
3. Draw and repair a nearly closed profile; distinguish an open chain, a closed
   under-constrained region and an inconsistent constraint system.
4. Exercise every advertised tool including curved offsets, spline refinement,
   curvature, linked projections, patterns and text holes.
5. Edit an S3 local component while other components remain unchanged; navigate,
   search constraints, cancel a C1 solve and immediately perform another edit.
6. Cross a monitor/scale boundary, use trackpad/pinch/mouse and test input near
   Action Boxes/navigation widgets. No gesture may steal another's capture.
7. Edit the same sketch in two windows, verify conflict/rebase, and close one
   while a result is pending. Cancel one draft after relevant and unrelated
   changes accepted by the other, including parameters: all commits survive.
   No stale completion changes the other draft or committed geometry.
8. Split/merge an extruded region and remove a hole; explicitly repair the
   dependent profile inside the private candidate. Exercise Selection Cancel,
   whole-session Cancel, failed replay and successful atomic OK/undo/redo.

Milestone completion requires all advertised tools, the accepted geometry and
performance classes, honest unresolved-failure reporting and physical-device
acceptance. Passing CPU tests alone is not visual or latency acceptance.

## 12. Decisions, remaining gates and Architect handoff

Revision5 selects faer for sparse solves and a Foster–Davis SPQR_RANK adaptation
inside that library for rank, replacing the unavailable Pierce–Lewis reading
prerequisite. §9.5 V2 compares the alternatives and fixes the method, factor
bridge, exact files, result semantics and qualification. No new package, FFI,
unsafe extension or second geometry kernel is selected. All Sketcher goals and
performance/accuracy gates remain unchanged; no success date is asserted.

V0 is already applied and accepted. V1 import/parity and V3 candidate probes are
partial existing work; preserve them and reconcile the patch provenance at
resumption. V2 is not implemented; V3 coverage and V4 qualification remain
incomplete. M5.0 is still open. Existing D1–D3 authoritative Sketch amendments
and M4 acceptance prerequisites persist. The present role switch suspends
Executor work; the earlier revision-4 execution request does not authorize
continuing implementation during this Architect stage.

There is no unresolved new product choice within the selected method. The human
may approve revision5 and explicitly resume the bounded Executor work, beginning
with the V2 factor-contract proof while completing V1/V3 evidence and V4 gates.
That is a recommended next stage only. An optional Challenger is not required.
A concrete failure requiring new unsafe, another baseline/package, architectural
change or altered scope must return with evidence and its specific decision.
Production adoption requires the V4 dossier and separate explicit authorization.
Do not begin M5.1.

Only `docs/plans/M5-sketcher.md` changed during this Architect revision. No build,
test, benchmark, numerical experiment, source import, application, agent,
commit or publication was run. Historical checks above are not current
verification. The dirty checkout and index were preserved; textual checks do
not qualify implementation. Stop after delivering this plan.

## 13. External primary references

These support dependency/API investigation, not benchmark or integration claims:

- [faer 0.22.6 manifest](https://docs.rs/crate/faer/0.22.6/source/Cargo.toml.orig):
  published features, license and compiler baseline.
- [rstar 0.12.2 manifest](https://docs.rs/crate/rstar/0.12.2/source/Cargo.toml.orig)
  and [RTree API](https://docs.rs/rstar/0.12.2/rstar/struct.RTree.html): candidate
  indexing dependency; distributions with overlapping bounds degrade queries.
- [SolveSpace solver description](https://solvespace.com/tech.pl): nonlinear
  equations, rank/under-constraint considerations and dependence on initial state.
- [CGAL arrangements manual](https://doc.cgal.org/latest/Arrangement_on_surface_2/index.html):
  algorithmic vocabulary for intersections, half-edges and bounded faces only;
  no CGAL dependency or second geometric kernel is proposed.

- [Foster–Davis SPQR_RANK, full author-hosted paper](https://people.engr.tamu.edu/davis/publications_files/Reliable_Calculation_of_Numerical_Rank_Null_Space_Bases_Pseudoinverse_Solutions_and_Basic_Solutions_using_SuiteSparseQR.pdf):
  selected V2 method; §§2.1–2.4 and appendix supply the factor contract,
  iteration, reliability and null-space algorithms. Accessible on 2026-10-29;
  this author manuscript carries placeholder publication fields, not a Rust release.
- [Gu–Eisenstat strong RRQR, full author-hosted paper](https://math.berkeley.edu/~mgu/MA273/Strong_RRQR.pdf):
  SIAM J. Sci. Comput.17(4),848–869,1996; compared dense alternative.
- [rsparse1.2.1 API](https://docs.rs/rsparse/1.2.1/rsparse/): direct sparse solver
  inventory, not evidence of M5 rank or cancellation qualification.
- [Pierce–Lewis sparse multifrontal RRQR](https://doi.org/10.1137/S0895479893244353):
  revision4 historical selection; full text unavailable in that investigation,
  superseded by revision5 and no longer required for execution.
