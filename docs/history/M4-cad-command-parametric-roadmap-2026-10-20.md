> Historical roadmap recovered during Desktop cleanup on 2026-10-26.
> This unapproved proposal predates the current M4.2/M4.3 plans and workflow.
> Current architecture, UI specifications and individually approved milestone
> plans take precedence. No implementation or historical approval gate is
> authorized by this recovered document. Original text follows unchanged.

# Milestone 4 — CAD Command & Parametric Interaction

- **Revision:** 2
- **Status:** Proposed — awaiting human review.
- **Role/provider:** Architect / OpenAI Codex.
- **Date:** 2026-10-20 (Europe/Paris).
- **Task:** dependency-ordered delivery roadmap for backlog M4.1–M4.10.
- **Source checkout:** `/Users/martin/Desktop/Newell`, branch `main`.
- **Verified HEAD/common baseline:** `1eb4a9a4b9fb45a4bbfaa223e348c0e5e9a69f1b` (matches the supplied last-known commit).
- **Writing checkout:** `/Users/martin/Desktop/Newell-m4-roadmap-codex`, branch `codex/m4-roadmap-r1`, at that HEAD. This existing separate worktree was reused; the source checkout remains on `main`.
- **Only deliverable:** `docs/plans/M4-cad-command-parametric-roadmap-codex.md`. Revision 2 answers the Challenger findings on face selection, Surface feasibility, expression syntax and renames, and worker fairness. The plan remains untracked in the separate writing checkout; no copy exists in the source checkout or baseline.
- **Input snapshot:** the revision-1 technical audit used the baseline plus the nine uncommitted source-checkout files identified by SHA-256 in Appendix A/A.2. Revision 2 uses that audit and the read-only Challenger findings. Appendix A.3 records later source-checkout drift, which has **not** been incorporated into the technical audit. A future Executor needs a human-selected baseline/snapshot including whichever local changes are approved; the writing checkout alone does not reproduce the audited inputs.
- **Actions excluded:** implementation, tests/builds/application runs, changes to canonical documents or Git index, commit/push/integration, other agents or workflow stages. None are authorized by this roadmap.

## 1. Authority, scope and gates

Binding sources: `ARCHITECTURE.md` §§1, 2.3, 3–5; `docs/agents/README.md`; `docs/agents/architect.md`; accepted/approved ADRs 0001–0004. ADR 0004's exact 4.2.2 pins and Rust 1.88 govern over ADR 0001's older “latest version” wording. ADR 0003 D8's permanent hover removal conflicts with the later explicit M2.25 exception in architecture §3.5; preserve that exception and report the stale ADR wording, do not remove current border hover.

UI authority is `docs/ui_design/README_UI.md`, particularly §§2.2–2.6, 5, 7–8, 18–23, 25–26, 28–34, 36–38, then `Action_Box_Extrude.txt`, `Interface_Elements.txt`, `Interface_Exemple.txt`, `Application_Menu_Bar.txt`, `Command_Bar.txt`, and `Special_ASCII.txt`. References describe the UI; the geometric contracts proposed here fill unspecified modeling behavior and require plan review. This document grants no specification amendment.

M4 includes usable document profiles, real entity results, parameter recomputation, and a safe geometry transaction. It does not require a constraint-based sketch editor, file import/export, arbitrary future Create commands, durable on-disk document format, an undo/redo system, a general job platform or persistent topological naming across arbitrary edits. These omissions cannot be used to omit the minimal prerequisites below.

### 1.1 Explicit human-resolution gates

| Gate | Finding and recommended disposition | Affected work |
|---|---|---|
| G1: local default conflict | Dirty `README_UI.md` §28.3, menu artwork, shell and renderer change defaults to Dark/Wireframe; architecture §§3.1.1, 3.6, 4.2 and ADR 0003 D7/D9 still require Light. Preserve the local work. Obtain authoritative human resolution before implementation that would depend on the default; this plan does not approve Dark. Wireframe alone is not prohibited by that theme rule. The local Theme-row toggle retaining its popup is also an input to preserve. During drafting, `Application_Menu_Bar.txt` additionally acquired a `□ Color` row and lost its terminal newline; re-read at handoff. The added row is absent from the inspected README/menu implementation and has no defined semantics. Preserve it as external work and request UI specification reconciliation; it does not authorize color in the monochrome ASCII overlay. | UI/regression baseline, not geometric computation |
| G2: profile entry prerequisite | No document profile producer exists. Recommend a small Newell-owned profile-definition API plus an explicit bundled starter document containing real profiles and bodies; start from it through the existing File/New action. This enables manual M4 use without claiming a sketch editor. Human review must accept the starter workflow or assign a real profile creation/import prerequisite. Fixture-only profiles cannot satisfy interactive acceptance. | S2 and all manual extrusion acceptance |
| G3: surface feasibility stop gate | “Same for Surfaces” and M4.6 require Surface/New, Surface/Add and Surface/Remove. Existing solid boolean APIs do not prove sheet booleans. S0 must deliver the decision package defined in §8 before S9 starts: exact operations, target classes, fixtures, measured outcomes, pinned APIs and a proposed implementation route. Human review must approve that route. If no exact route exists under the pin, stop S9 and revise scope or the kernel policy in a new reviewed plan. An `Unsupported` result cannot close this gate or complete M4. | Surface combinations and full M4 acceptance |
| G4: extent ambiguities | Up-to `Offset` has no expanded artwork (§2.6); symmetry does not specify half-depth versus per-side; arbitrary target surfaces are unspecified. Proposed geometric choices and amendment requests are in §§6, 9. Approve semantics and any exact UI amendments before affected UI work. | Offset, symmetry, curved targets |
| G5: selectable face targets | Main/Start/End/Second Up-to requires a particular live face or datum, not an arbitrary body. The Selection panel and viewport need the typed option, visible label and generation-bound mapping in §4.3. S5 and S8 cannot pass their selection acceptance until face pick, modal choice, stored target and stale invalidation round-trip by identity. | Up-to UI/data contract |

Ordinary choices made here: canonical millimetres/radians; typed checked IDs; conservative invalidation; a bounded expression parser; one geometry worker; immutable committed snapshots; typed command intents instead of polling widget status. None needs a new architectural boundary. No new crate or third-party package is proposed. If S0 proves a dependency change necessary, it requires a revised plan, exact version/MSRV/license review and, for BrepKIT, the ADR/golden-file procedure. Merely returning `Unsupported` is safe during delivery but is not completion of an in-scope valid operation.

## 2. Current-state audit (source snapshot, not a passing-test report)

| Backlog item | Concrete existing implementation/evidence | Missing functional behavior and test limits |
|---|---|---|
| M4.1 command/controller | `newell-model/src/lib.rs`: `CommandState`, `CommandController`, `ExtrudeSpec`; `newell-ui/src/extrude.rs`: `ExtrudeUi`, `ExtrudeValues`, `ExtrudeStatus`; `newell-app/src/app.rs::sync_extrude_command`. | Model command enum only covers Extrude, with no session/request revision or typed field intents. App polls mutable UI values after general input. Extrude panel initially exists as mock state; a non-command key can initiate/edit it. Confirmed status is polled repeatedly: there is no consumed-once commit intent. Other Create panels remain mocks. Existing UI/controller tests verify controls and routing, not one atomic geometry-backed command. |
| M4.2 trees | `shell/data.rs::FEATURES`/`ELEMENTS` fixed arrays; `shell/mod.rs::set_model_tree_labels`; `app.rs` startup copies feature/entity labels. `widgets/tree.rs` supplies row drawing; `shell/tree_tests.rs`, `tests/it/tree.rs` cover expansion, visibility, selection, rename, playhead/dividers. | Relabeling fixed named slots leaves mock topology/cardinality and may fall back to mock labels for absent names. No arbitrary model-sized row projection, ID binding or post-commit refresh; no real geometry catalogue. Tree-local rename/visibility is not automatically a model mutation. Playhead is a separator, not implemented history rollback. |
| M4.3 selection | `newell-model::Selection` deduplicates/toggles entity IDs; `app.rs` viewport pick sets `scene_entity`; `picking.rs::PickHit` has face ordinal and mesh revision. `overlays/selection.rs` has generic and ordered-profile render modes. | App has one window/one scene body, no face-to-entity map. Tree selection and modal's fixed nine `MOCK_ITEMS`/`PROFILE_ITEMS` are disconnected from model selection. Selection is not yet filtered by field, document or topology revision. Existing navigation tests exercise triangle queries, not document identities or cross-surface selection. |
| M4.4 units/fields | Model `Length` is finite f64 millimetres; `NumericFields` in `widgets/numeric.rs` edits magnitudes, direction prefixes and unit suffixes. `shell/mod.rs::extrude_depth_source` exposes one field's source. | No unit conversion/dimension system or parameter binding for all fields. Numeric UI caps one fractional digit and uses mock-derived units. App inserts zero strings for enabled offsets/second end; secondary numeric edits are ignored. Direction prefix currently becomes a negative source while model rejects nonpositive Blind depth, contradicting magnitude/orientation semantics (§5.4). |
| M4.5 expressions | Model `ParameterValue::{Literal,Expression}` contains strings; `evaluate_length` parses a number or whitespace-separated addition/subtraction; parameters are `BTreeMap<String, Length>`. Formula editor stores raw text and confirms it back through `NumericFields::set_formula`. | No AST, dimensional analysis, stable references, multiplication/division, parentheses, `#` references, functions, dependency graph or recomputation. Parser can accept empty/only-operator text as zero or adjacent operands as implicit addition. The reference formula is not supported. Three model unit tests cover a simple expression/commit, cancel and missing merge scope; they do not prove evaluator correctness. |
| M4.6 real Extrude | Model enums represent Solid/Surface and New/Add/Remove. `newell-kernel::NewellSolid` wraps `Topology`/`SolidId`, `box_solid`, `tessellate`; `NewellMesh` exposes positions/normals/indices/face offsets. | No profile definition, wire/face wrapper, profile-based extrusion, boolean wrapper, sheet type, multi-body scene or persisted geometric result. `Feature::BaseSolid` is a label/entity and Extrude commit only appends metadata. Kernel integration tests cover a box's six face ranges and invalid dimensions/deflection. |
| M4.7 termination | Enum exists; validator checks positive Blind depth and mistakes `profile.is_some()` for an Up-to target. | No distinct target, direction resolver, hit/trim or Through-all bound. All modes still feed the same box preview from depth; no actual termination tests. |
| M4.8 modifiers | UI has signed numeric fields, Along, start/end mode/target branches, symmetry, second Blind/Up-to and nested End Offset. Model stores some booleans and optional lengths. | Adapter loses Along and all targets/mode details, uses zero offsets/second depth; kernel ignores modifiers. Model cannot represent full UI combinations. No geometric tests. |
| M4.9 merge | Model has `merge_with_all` and scope vector, validator rejects empty explicit scope for Add/Remove. | No compatible target search, geometry intersection, actual fuse/cut, consumed entities or revision validation. App reuses global selection for both profile and merge scope. No multi-target correctness tests. |
| M4.10 safety | Model draft/preview specs are separate from history, `cancel` clears them. UI independently saves/restores mock/numeric state. | No retained committed geometry, worker, async results, atomic model/scene commit, invalid-preview state or stale suppression. App ignores modeling errors. Cancel is geometric replacement with a fixed cube, not restoration. See detailed path below. |

### 2.1 Actual preview/transaction path

`App::sync_extrude_command` reads the UI after primary presses and translated keyboard input. Only Depth uses edited numeric/formula source; most other semantic fields are hardcoded or inferred from one global selection. A valid spec goes to `set_preview_depth`, which computes `(depth_mm / 10).clamp(0.01,100)` and calls `NewellSolid::box_solid(1,1,scaled)` then tessellates **on the event-loop thread**. It replaces both the renderer mesh and navigation cache. It neither uses a profile nor preserves other bodies. Solid/Surface, Add/Remove, termination and modifiers do not alter that construction.

OK calls `CommandController::confirm` to append a new feature/entity after partial validation; no kernel result is coupled to it, and errors are discarded. There is no exactly-once confirmation token. Cancel calls `set_preview_depth(10)` and restores a unit box irrespective of committed geometry. Startup makes a unit box and separately appends `Base Solid` metadata, then sets tree labels once. `App` owns one `Option<Window>`, renderer, shell, document and selection; architecture §4's shared-document/multiple-window behavior is not implemented.

`newell-render/src/renderer.rs::set_scene_mesh` and `scene.rs::set_mesh` replace one mesh. `Navigation::set_mesh` cancels capture and replaces `PickingScene`; its revision is local mesh replacement count, not a document revision. `face_offsets` are tessellation ranges, not stable face IDs. Keep the useful M3 mechanics but replace the document/scene coupling.

### 2.2 Verification evidence boundary

Inspected model inline tests; kernel `tests/it/kernel.rs`; UI numeric/formula/selection/extrude/operations and shell tree tests; app controller/tree/navigation tests; renderer scene/GPU interfaces and local changed GPU test assertions. Many M2/M3 tests cover ASCII and camera behavior well. They establish intended regressions to protect, not M4 geometry correctness. No current checkout check, build, test, GPU run or application/manual scenario was executed for this Architect stage. Historical reports are not evidence that this snapshot passes.

## 3. Architecture and data contracts

### 3.1 Ownership and dependencies

| Owner | Contract |
|---|---|
| `newell-model` | Feature/history records, profile definitions, parameter graph, semantic validation, document identity/revision, entity catalogue and geometry bindings. No UI, winit or GPU. Add only a dependency on Newell's `newell-kernel` wrapper if needed to retain immutable geometry in the document snapshot; no BrepKIT imports. |
| `newell-kernel` | Every mutable BrepKIT arena/handle, construction/measurement/trim/boolean/tessellation operation and native error conversion. Newell-owned immutable `GeometrySnapshot`, shape/face keys, request/result DTOs. Never imports UI, app or model types; model IDs are mapped to kernel keys by the model/evaluation adapter. |
| `newell-app` | Event-loop/window manager, one shared document session, per-window command/selection, geometry-worker orchestration, intent adapters, scene/picking packet assembly and document change fan-out. No direct upstream topology or geometry algorithms. |
| `newell-ui` | ASCII composition, input editing/focus/overlay state and typed UI intents/projections. UI DTO row/field tokens are opaque app-mapped values; no dependency on BrepKIT, renderer or necessarily model. Never the semantic authority for a committed feature. |
| `newell-render` | Immutable scene packets, GPU resource preparation/presentation and flat ASCII grid. It owns no command, selection mutation, hit test, feature evaluation or camera gesture. |

Use plain structs/enums, not a generic framework or object-safe command plugin registry. Splitting existing `lib.rs` files is useful only for the concrete modules named in §8. `Selection` can remain a model-owned pure value type; its **instance** belongs to each window. Central document geometry is immutable/shared. Parameter or feature edits create a replacement document snapshot, never mutate render resources from the model. Prefer `Arc<GeometrySnapshot>` around a private cloned candidate arena, with mutation confined to worker-local scratch values. Future compile-time tests must establish Send/Sync for the concrete shared snapshots and Send for job payloads under the pin; do not add unsafe implementations. If upstream prevents sharing, keep arenas entirely worker-owned and use immutable Newell-owned geometry tokens/CPU result packets, documenting that adjustment before integration.

### 3.2 Identity, revisions and topology

Introduce checked Newell-owned `DocumentId`, `DocumentRevision`, `FeatureId`, `EntityId`, `ParameterId`, `ProfileId` (or typed profile entity ID), `GeometryGeneration`, `CommandId`, `DraftRevision` and `RequestId`. IDs are scoped to DocumentId, monotonically allocated without reuse; overflow is an error, not current saturating/wrapping allocation. Allocation happens in a candidate clone; unsuccessful/cancelled candidates do not change committed counters. Request/session IDs have independent allocators, and need not roll back.

`Document` stores feature records in deterministic history order; each has stable ID, name, definition, input references, owned parameters, and **zero/many** output entity IDs. Replace `Feature::entity()`'s one-output assumption. Entity catalogue records kind (`Profile`, `SolidBody`, `SurfaceBody`, `DatumPlane`, `Direction`), producer feature, display name and geometry binding. Add/Remove consumes active predecessor entities in the evaluated history projection; keep their feature records and upstream geometry for recomputation, but do not show both old and replacement bodies as current Elements.

`EntityRef { document_id, entity_id }`, `EntityBinding { entity_id, generation, shape_key }` and `TopologyRef { entity_id, generation, kind, local_key }` wrap geometry identities; a TopologyRef is always carried with its document context. Kernel `ShapeKey`/`FaceKey`/`EdgeKey` are Newell-owned keys valid only in a particular immutable GeometrySnapshot, not BrepKIT IDs. Kernel snapshots hide arenas entirely. Triangle/face ranges carry a parallel map to topology refs; aggregate mesh order cannot be treated as model identity. A scene packet includes DocumentId, committed revision, scene generation and range bindings. Preview keys are tagged with CommandId/RequestId and cannot become document selection IDs.

Keep entity IDs stable for an unchanged logical output. A one-to-one edited output may keep its EntityId but increments GeometryGeneration. For split/merged bodies allocate new IDs with explicit predecessor lineage; delete old active bindings. Choose deterministic output ordering by source feature/profile/target order plus geometric component order. Never use floating-point centroid equality alone as permanent identity. Newell can reuse semantic extrusion-face roles (start/end cap, side of profile segment/hole) only where their lineage is exact. Use boolean evolution as advisory provenance, not guaranteed persistent naming: 4.2.2 can fall back to a normal/centroid heuristic. On ambiguous split/merge/remesh, invalidate subentity selections and dependent target refs; require reselection instead of silently selecting an unrelated face. Entity-level refs may survive a generation change only through explicit model lineage validation.

All semantic mutations increment DocumentRevision once on successful publication. Parameter edits, feature edits and profile replacements use the same transaction path as new Extrude. Cosmetic per-window expansion/selection/camera/theme changes do not increment it. A target must belong to the same document and be current at the evaluation revision; future/self feature references are rejected. Conservative invalidation may reject a recompute; it must preserve the last valid committed document.

### 3.3 Proposed core interfaces (names define responsibilities, not upstream APIs)

- `Document::prepare_edit(Edit) -> Result<CandidateDefinition, ModelError>`: pure candidate history/parameter update, checked ID allocation, dependency validation. No committed mutation yet.
- `evaluate_parameters(&CandidateDefinition) -> Result<EvaluatedParameters, ModelError>` and `resolve_extrude(...) -> Result<ResolvedExtrude, ModelError>`: unit-checked values and IDs become Newell kernel DTOs through the evaluation adapter.
- `GeometrySnapshot::evaluate(&KernelRequest, &Cancellation) -> Result<KernelResult, NewellKernelError>`: worker-only calculation using private scratch topology. `KernelRequest` carries Newell-owned profile geometry, shape bindings, direction, extent and target keys; no UI/model types.
- `PreparedDocument { base_revision, document, geometry, entity_changes, scene_data }`: fully validated model+geometry candidate. App publication is compare-and-replace against the same base revision, with no fallible geometry work at that point.
- `DocumentChanged { document_id, old_revision, new_revision, changed_features, changed_entities, removed_entities, invalidated_topology }`: model-only event payload; no WindowId/GPU types. App fans it to every subscribed window; UI/render subscribers are app-owned.

Domain errors are structured: invalid/nonfinite quantity, unit/dimension mismatch, unknown reference/cycle, invalid profile, missing/ambiguous/stale target, degenerate direction, empty result/no effect, unsupported capability, cancelled, stale base and kernel failure. `NewellKernelError` uses `thiserror`; translate every native topology/math/operation error inside the kernel. UI gets a short actionable message and typed field/selection association, with detailed diagnostics available through existing logging. Production code adds no `unsafe`, unwrap/expect/panic.

### 3.4 Shared document and affected windows

Refactor the current single-window fields to `DocumentSession` plus `BTreeMap<WindowId, WindowContext>`. Each WindowContext owns shell/controller, command session, selection, expansion/visibility, navigation, scale/theme, renderer and scene installation state. One document session and worker suffice for M4; no general multi-document manager is required. Two windows can subscribe to the same session. Use existing File/New for the approved starter document and File/Open only if a separately approved input path exists; do not pretend persistence exists. Provide a testable second-window creation path through the existing window manager; any new visible menu action needs UI approval.

Publishing a candidate emits DocumentChanged once. All subscribed windows rebuild their tree/entity projections, prune invalid selection, cancel navigation capture on geometry replacement and request redraw. Update all subscribers initially; entity-level subscription indexing is unnecessary at M4 scale. Other windows keep their independent camera/selection/theme. A window with an active draft becomes explicitly stale on another commit: remove its preview, disable confirmation, and re-resolve/recompute against the new revision if all references remain valid; otherwise require correction. Never overwrite the other commit by restoring an old snapshot on Cancel.

## 4. Command lifecycle, trees and selection

### 4.1 Generic controller with an Extrude payload

Put runtime command state in app `command.rs`, keeping semantic Extrude definitions in model. Use `CommandSession { command_id, document_id, base_revision, draft_revision, draft, active_selection_slot, preview_state }`. Lifecycle: Idle → Editing (invalid/valid/computing/ready/error) → ConfirmPending → Idle, with Cancel available before publication. `ConfirmPending` captures one exact draft revision; repeated OK is ignored. No feature is created on menu open, a general key or preview completion.

UI emits owned, consumed-once intents: `BeginCommand`, `SetChoice`, `SetDirection`, `EditFieldText`, `ApplyFormula`, `ActivateSelectionSlot`, `ApplySelection`, `Confirm`, `Cancel`, and model tree rename/visibility intents. Define stable `CommandKind`, semantic `FieldKey`, `SelectionSlot` (Profiles, Along, MainLimit, StartLimit, EndLimit, SecondLimit, NestedEndLimit, MergeScope). Map WidgetIds to these in O(1) using current fixed Action Box key slots/direct row-token maps. A typed enum is the generic interface; only Extrude gains real geometry in M4. Preserve the selected FeatureId as local selection provenance in addition to projected output refs: invoking Extrude with a single committed Extrude feature selected loads an edit draft for that feature, while invocation with profile/entity selection creates a new draft. This uses the existing command action and preserves double-click for Rename; do not invent a competing double-click editor. Confirm replaces an edited definition and recomputes descendants rather than appending a second feature. Other panels preserve their existing UI demonstrations and cannot accidentally commit an Extrude.

`ExtrudeDraft` retains raw field edit state and hidden branch drafts. `ExtrudeDefinition` stores only validated, active semantics. Use `Extent::{Blind{magnitude,direction},UpTo{target,offset},ThroughAll{invert}}`, `StartPosition::{Profile,Distance,UpTo}`, `EndAdjustment::{None,Distance,UpTo}`, `Sides::{One,Symmetric,Second{extent,nested_end}}`, geometry/result mode and independent merge scope. Include actual Along ref, all offset modes/targets, and multiple ordered profiles. Switching branches preserves hidden drafts for restoration but does not evaluate hidden values or capture hidden dependencies. Symmetry suppresses independent second-side computation while retaining its UI draft. Command reopen starts from defaults or a selected committed feature definition, with a new CommandId.

Formula modal and Selection panel are child transactions. Their Cancel restores the parent draft field/selection slot, not the whole document; command Cancel discards all children. Numeric Enter finishes its local edit first, modal Enter handles its own transaction, and only a subsequent explicit command OK confirms. Preserve UI input precedence and no click-through. Failed validation keeps the panel editable; UI must not close/save NumericFields merely because a footer was pressed before app acknowledgement.

### 4.2 Tree projections

Build flat `FeatureRow`/`ElementRow` DTO vectors from committed history/entity catalogue, with stable opaque row tokens mapped to FeatureId/EntityId. Features include real profile/base/extrude records; Elements groups active solids, surfaces and construction/profile entities, with actual counts and no mock filler rows. Draw the existing branch/checkbox/expansion/truncation grammar. Dynamic model hierarchy is plain domain data, not a retained widget tree. Flatten on document/projection changes, compose cells on redraw, and retain direct row-index/token lookup for pointer routing. Scroll/virtualize rows without allocating WidgetIds by unchecked arithmetic or scanning bounding boxes. Keep playhead an existing nonselectable UI separator unless actual history editing is separately approved.

Expansion, row focus and visibility are per-window maps keyed by real IDs. Entity visibility controls affect scene construction/picking in that window; tree checkbox never means selected. Rename updates a document feature/entity through a validated metadata transaction and propagates to every window; renaming a parameter preserves its ParameterId. Do not regenerate all row tokens on rename or collapse. A removed token is rejected even if a stale event arrives. Mock arrays remain only in explicit reference/test fixtures, not production document projections.

### 4.3 Selection consistency and filters

`WindowSelection` contains DocumentId and ordered unique entity/topology refs. Viewport, Features, Elements and Selection panel all use the same reducer. A feature-row select projects its current output entities (profiles included); an entity row selects its entity. Preserve existing single-select and modifier behavior; specify/test additive toggle with Ctrl/Cmd and empty exposed-viewport click clearing. No invented selection glyph/color is added. State can be consistent without a new highlight until a viewport presentation is approved.

Selection-slot filters validate kind, document, generation and cardinality: Profiles = one/many planar usable profile entities (Solid requires closed regions; Surface may accept open wires), Along = one direction/straight edge or planar-normal reference, Main/Start/End/Second limits = one valid terminating face/datum, MergeScope = one/many compatible live bodies. Keep Profiles separate from MergeScope. Invalid picks are rejected with a reason; never reinterpret a solid body as a profile or an arbitrary body as an Up-to limit. Selection field counts derive from their slot's real IDs.

The app owns a flat `SelectionOption { row_token, selection_ref, label, eligible_slots }` list and passes only opaque row tokens, labels and enabled state to the existing generic Selection panel. `selection_ref` stays app/model-side: it is a checked `EntityRef` for whole profile/body/datum entities or a generation-bound `TopologyRef` with `kind = Face` for a face. The app maps a returned row token directly to that exact reference; it is never reconstructed from a displayed name or tessellation ordinal. For limit slots, include eligible datum planes and individual eligible faces of live entities, with labels such as `Body name / End face` or `Body name / Face 3`. These are dynamic data labels within existing rows, not new artwork. Duplicate names remain distinct by token. If a face cannot be named unambiguously across the current generation, present a generation-local ordinal and invalidate it on remesh; do not promise persistent topological naming. The viewport's face-range map returns the same `TopologyRef` as the corresponding modal row. Entity rows in the tree still select whole entities; a body row alone does not fill a face limit.

The generic modal edits a temporary set of these typed options. Its OK applies checked references to the active slot and projects that same set to the window selection; Cancel restores the opening local set. Only one compatible face or datum may fill each limit field; a whole-body pick produces a field-specific rejection. The ordered Profiles panel edits real profile IDs, removes by ID and reorders through its documented handle, retaining duplicate **names** without conflating identities. While modal, it intercepts background viewport input; pick directly into a slot when its field is active without a modal. Do not let a modal's draft selection mutate committed geometry. On DocumentChanged, reconcile window and open panel drafts before drawing; invalidate missing/generation-changed subentities and mark dependent limits for reselection. Picking uses the scene packet's map and rejects mismatched revision/generation even if face ordinals match.

## 5. Parameters and expressions

### 5.1 Quantities, ownership and editor state

Canonical quantities use f64 millimetres and radians, plus dimensionless scalars. Supported explicit length units: `mm`, `cm`, `m`, `um`, `in`, `ft`; angles: `deg`, `rad`; bare numbers are dimensionless in formulas. Store dimensions as a small exponent pair (length, angle), bounding exponents; addition/subtraction require equal dimensions, multiplication/division combine them. Field validation requires the final declared dimension. Do not add mass/time or general unit-conversion infrastructure in M4. Literal numeric fields use the field's selected/document display unit when suffix is omitted; do not apply contextual unit guessing to arbitrary formula operands. Accept `10` as a literal depth in mm, but `#wall + 10` is a dimensional error if wall is length; write `#wall + 10mm`.

Separate `raw_editor_text`, parsed AST with resolved ParameterIds/source spans, and evaluated finite quantity. Display rounding (canonical fixed ASCII field width and one fractional digit) never overwrites full internal precision or formula text. Direct numeric field edits remain bounded by existing editing rules; Formula provides additional precision/units. Direction is a separate enum, never encoded into a negative magnitude or the literal `+|` artwork. Signed scalar offset results and directional prefixes are converted once to an explicit coordinate convention; negative depth magnitude is rejected with a useful diagnostic. An expression cannot smuggle a sign past direction validation.

Each feature owns ParameterIds for its magnitude/offset fields; the document has a named parameter table for references, including named feature parameters. `ParameterRecord { id, owner, unique_name, declared_dimension, source, parsed, evaluated }`. UI widgets reference field→ParameterId mappings from the adapter, never own committed values. A draft allocates provisional candidate parameters, published only at commit. A rename preserves ParameterId and atomically rewrites every committed parsed source span bound to that ID to the new `#Name`, then reparses/resolves those sources in the same candidate transaction. Preserve all unrelated source text and spacing; reject the rename if a rewritten source exceeds the editor cap or fails evaluation. Formula reopen shows the new name and the original binding. An invalid, uncommitted editor draft remains its user's raw text and must be revalidated before confirmation; it is never silently rebound by spelling. Unknown names or deleted parameters make candidate evaluation invalid. Expose deterministic unique names for feature fields, for example `Extrude_1_Depth`, in their Formula source/reference context; the starter document supplies named examples such as `EntraxeA`, `Angle1` and `Angle2`. Existing-feature edit invocation from §4.1 changes a bound literal/formula and recomputes descendants, providing an actual user parameter-setting path. Direct numeric edits explicitly replace the prior formula with a literal on acknowledgement; reopening Formula still shows its saved source until that replacement is acknowledged. A standalone parameter table UI is not required.

### 5.2 Bounded syntax and evaluation

Choose a small hand-written tokenizer plus precedence parser in model, because this is a bounded CAD quantity language with source-span errors and typed references, not general scripting or standard job infrastructure. No new expression dependency. Grammar: optional leading `Fx =`, decimal/scientific finite numbers with optional unit suffix, `#Identifier`, parentheses, unary `+/-`, binary `+ - * /`, and calls to `sin`, `cos`, `tan`, `abs`, `min`, `max`. Tokenize `,` only as a function-argument separator; `call := identifier '(' expression (',' expression)* ')'`. `sin`, `cos`, `tan`, `abs` require exactly one argument; `min` and `max` require at least two. A missing/extra/trailing argument is a source-span error. ASCII identifiers match `[A-Za-z_][A-Za-z0-9_]*`; support existing bare parameter names as a migration alias only when unambiguous, serialize/display preferred references with `#`. No assignment after the optional display prefix, implicit multiplication, exponent operator, scripts, filesystem/network evaluation or arbitrary functions. `2mm`, `2 mm` both tokenize as a quantity. The two visible Formula rows are one logical source; no newline multiplication is implied.

Trigonometric inputs must be angles and return dimensionless values; `abs` preserves dimension, min/max require compatible dimensions. Reject division by zero, nonfinite intermediate/results, malformed/trailing tokens, empty input, unknown units/functions, wrong arity and trig singularities within angular tolerance. Limit source to the existing 256-character editor cap, AST nodes to 256 and nesting to 32; return a bounded diagnostic rather than recursion exhaustion. The exact `Interface_Elements.txt` sample `Fx = (#EntraxeA - 2mm) / 5 * 3 * sin(#Angle1 - #Angle2 + 3deg)` must evaluate with typed parameters.

Extract graph edges from resolved references, detect self/indirect cycles with a DFS/topological sort that reports the cycle path, then evaluate in deterministic ParameterId order for equal-rank nodes. Evaluate all active parameters for M4 (small graph); keep dependency edges for diagnostics and downstream feature order. No incremental scheduler needed. Reject forward geometry dependencies; named parameter dependencies may be defined in any declaration order if acyclic. Changes recompute affected history from the earliest dependent feature (full replay of the small history is acceptable initially), off-thread, through the same candidate transaction. A failure in any downstream feature retains the entire previous committed document/geometry/parameter values; retain raw candidate text for correction.

Invalid text is kept verbatim in an editable draft with diagnostics. It never replaces evaluated committed data, creates a preview labeled valid, or enables OK. Formula OK validates before it is acknowledged; Cancel restores the original raw source and prior parse/evaluated draft state. Empty Formula may explicitly revert to the last literal value only through an agreed clear-formula action; it must not silently evaluate as zero. Use existing text/status facilities for errors; an exact new error artwork or disabled-footer visual requires the amendment gate in §9.

## 6. Real extrusion contract and verified kernel capability

### 6.1 Profiles before extrusion

Implement `ProfileDefinition { frame(origin,u,v,normal), ordered_wires, source_refs }`, with Newell-owned line segments and circular arcs, closed outer regions and holes, or open wires for surfaces. Validate finite orthonormal frame, coplanarity, edge continuity, unique/nonzero edges, closure, nonzero area, orientation, no self-intersections, no touching/outside/nested-invalid holes, no overlapping selected regions. Separate disjoint profiles produce separate candidate entities; successful Add may unite them. Preserve deterministic wire/segment identity. Support a polygon with a hole and a circular profile in acceptance; arbitrary spline editing is not a prerequisite and is not silently approximated by a box.

Kernel builds actual vertices/edges/wires/planar faces in its arena, normalizes winding, and exposes wrappers; model stores definitions, not upstream objects. Planar existing faces may produce profiles by copying boundary geometry through a kernel wrapper with source generation binding. Never modify a body face in place to obtain a profile: `extrude` itself normalizes profile wire curves, so always use a scratch/copy. Profile entities need construction-line scene/pick data or selectable tree rows; selecting only through the tree is acceptable for the starter workflow, provided it is real and discoverable.

S2 must provide an approved interactive entry path. Recommended File/New starter model: named rectangle, holed region, circle and open chain on datum planes, plus two target solids, a target datum plane and surface examples. These are real editable definitions owned by the document, measured in mm, shared by windows; keep deterministic generation helpers separate from test-only mocks. Tests can construct arbitrary ProfileDefinitions directly. This is profile infrastructure and a starter document, not a sketch solver/editor or file importer. If the human declines this bootstrap, assign another profile producer before M4.6 can pass.

### 6.2 Local BrepKIT 4.2.2 source verification

Source root inspected: `/Users/martin/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`. All following paths are relative to `brepkit-operations-4.2.2/src/` unless noted. API presence is verified by reading source; numerical robustness has **not** been tested in this stage.

| Capability | Verified API/behavior | Newell work or limit |
|---|---|---|
| Profile solid extrusion | `extrude.rs:822`: `extrude(&mut Topology, FaceId, Vec3, f64) -> Result<SolidId, OperationsError>`. Code propagates inner wires; direction multiplied by distance, so normalize in ACL. It supports more than its stale planar-only error doc suggests. | Build/copy valid planar profile faces; reject zero/tangential extrusion, validate manifoldness and volume. No upstream `extrude_profile` or `extrude_surface` API is assumed. |
| Topology construction | `brepkit-topology-4.2.2/src/{topology,vertex,edge,wire,face,shell}.rs`: public constructors and `add_*`; `Wire::new(edges, closed)`, `Face::new(...)`, `Shell::new(...)`; Topology derives Clone. | Newell can construct planar swept sheet faces from segments inside the ACL, keep an open shell/face set without pretending it is a Solid. Initial exact line/arc routes require proofs in S0. |
| Solid fuse/cut | `boolean/mod.rs:100` `boolean(... BooleanOp, SolidId, SolidId)`. `boolean_with_evolution` at 1361 returns `(SolidId, EvolutionMap)`. `compound_cut` at 965 exists. | Operands must be in a common topology; current per-solid arenas cannot be combined just by passing IDs. Rebuild/copy into a candidate common arena. Split/empty cases need explicit component enumeration/EmptyResult handling. |
| Empty and disconnected results | `brepkit-topology-4.2.2/src/topology.rs:277,286` exposes `add_empty_solid` and `is_empty_solid`. `boolean/mod.rs:143,158` returns typed `EmptyResult` for identical/contained Cut, while intersection paths can return an empty-solid sentinel. `boolean/assembly.rs:851` has `face_components`, but it is `pub(super)`, not callable by Newell. | Translate both legitimate empty representations inside the ACL. Implement component traversal using public topology accessors; account for outer components and inner cavity shells, orient/reassociate cavities by containment, and validate the resulting solids. Never import the private helper or treat each disconnected boundary shell as an independent positive-volume body. |
| Provenance and quality | `boolean_with_evolution` uses faithful GFA provenance when possible and heuristic fallback otherwise. `mesh_fallback_count()` at 45 detects delivered mesh fallback. | Do not guarantee stable face names from evolution. Require valid B-Rep results; reject unapproved faceted fallback for real CAD operations. Single geometry worker allows a reliable counter delta around the chain within Newell. |
| Transform/copy | `transform.rs::transform_face` at 672, `transform_solid` at 41; `copy.rs::{copy_face,copy_solid,copy_wire}`. | Shift copied input to start plane, reverse/unit direction as required; avoid mutations of shared source topology. |
| Tessellation | `tessellate/mod.rs:113` `tessellate(Topology, FaceId, deflection)`, 129 tolerance version; grouped solid tessellation already wrapped. | Aggregate real sheet faces with explicit face keys/ranges; tessellation is visualization, never the definition of the operation. |
| Measurements/validation | `measure/{bounding_box,volume,area}.rs`, `validate.rs::{validate_solid,validate_solid_relaxed...}`. Bounds implementation treats analytic curvature as well as vertices. | Select validation appropriate to construction; do not switch to relaxed checks merely to hide invalid output. Exact/conservative geometry bounds for termination need verification, not just mesh bounds. |
| Plane splitting | `split.rs:41` `split(topo, solid, plane_point, plane_normal) -> SplitResult {positive,negative}`. Source documents restrictions including NURBS and uses polygon clipping. | Not an arbitrary curved-face trim nor automatically safe for every hole/analytic case. Prove applicable planar cases before using; otherwise use an exact half-space solid cutter and proven booleans. |
| Sewing | `sew.rs:37` `sew_faces(...) -> SolidId`, documented minimum two faces and no NURBS. | Does not establish sheet Boolean support or a general open-shell sew API. Do not call it as if it did. |
| Gaps | Inspected exported operation modules do not supply a general sheet fuse/cut or direct arbitrary target-surface termination API. | S0/S9 exact capability work and human decision are mandatory for all required combinations; neither thin-solid thickening nor CPU mesh clipping substitutes for sheet B-Rep semantics. |

### 6.3 Coordinates, direction, tolerance and extent calculation

All geometry computation is f64 in canonical mm; renderer converts checked finite coordinates/normals to f32 only at packet creation. Kernel tolerance policy mirrors the pinned defaults: linear `1e-7 mm`, relative `1e-10`, angular `1e-12 rad`; comparisons use `max(linear,relative*scale)`. Separate topology tolerance from display deflection (initially `0.05 mm`, configurable within kernel request). Do not inflate tolerances to fix a failing boolean. Because upstream extrusion compares squared direction length to its tolerance, Newell always supplies a normalized unit vector.

Let profile frame origin be O, normal n, unit extrusion direction d; validate `abs(n·d)` above a documented angular/conditioning threshold so a solid sweep has nonzero volume. Default d=n. Blind Along may use a straight edge/datum direction (or planar normal) with an explicit orientation. A curve tangent chosen from an arbitrary click is ambiguous and rejected unless an approved parameterization exists. `+|/-|` flips d independently of nonnegative magnitude. Through-all Invert flips d; never add an Invert row to Blind/Up-to. UI hides Along under other branches: those modes use profile normal, without secretly inheriting a hidden Along override.

Parameterize swept points as `P(u,v)+t*d`. An extent produces a start boundary s and end boundary e (constant or surface-dependent). Translation to the start is real geometry; bounds/camera scaling never changes physical depth. Require `e-s > tolerance` over the complete active region, not just at centroid/vertices if curved. Side surfaces connect the true boundaries and caps close solids. Solid volume on a planar constant extent is profile area times `(e-s)*abs(n·d)`.

- **Blind:** default s=0, e=L; L positive. Starting Offset Distance sets s to signed a. Proposed interpretation: main Depth remains the extrusion length, so e=s+L. Main End Offset Distance adjusts e by signed b; End Offset Up-to replaces the end boundary with the selected target. Record this interpretation for review; do not mix length-from-start and length-from-original-plane between code paths.
- **Start Offset Up-to:** target defines a new start boundary. A plane parallel to profile gives constant s; oblique/curved start requires actual trimming and cannot be reduced to one representative distance. Preserve the selected generation. Main blind depth translates that start boundary along d by L before end adjustment. Through-all instead resolves its outer end independently.
- **Up to:** target is independent of Profile. For a planar target with equation N·X=c, solve `t(P)=(c-N·P)/(N·d)`. Reject parallel/coincident ambiguity, target behind the chosen direction, incomplete profile coverage, disconnected/multiple limiting patches or zero/reversed interval. An oblique plane needs an oblique cap (trim), not extrusion by centroid distance. Finite face targets must cover the swept section; a datum plane can be infinite. A selected body must resolve to one explicit face rather than an arbitrary nearest face.
- **Through all:** geometric scope is every applicable document entity along d as §8.2 states, independent of per-window visibility and distinct from explicit merge scope. Resolve conservative projection extrema of the applicable committed geometry along d and extend beyond the maximum by `max(10*linear_tol, relative_tol*scale*10)`; include start/profile extrema and verify that every relevant bounding interval is passed. Include finite committed solid, sheet and profile geometry regardless of the output type; construction datum planes/directions have no finite outer bound and do not define the end. Exclude the active source profiles and transient candidate geometry from the search for a forward limiting entity. This avoids shortening a Solid extrusion because its furthest limiting entity is a sheet, or vice versa. No eligible forward entity gives MissingTerminationGeometry, including New; do not invent an enormous fixed depth. Do not use the current camera/frustum, visible tessellation, or only merge-selected bodies to shorten “all.” Invert changes which side is forward. Reject a start already beyond all applicable geometry if no forward interval remains. Symmetric Through-all computes both directions independently and chooses the larger required span.

The Up-to Offset checkbox is semantically required but its magnitude/direction UI is unspecified. Proposed amendment: disclose a signed formula-capable distance child, shifting the chosen end boundary by that distance **along d** (not surface-normal offset). Default zero changes no geometry; a checked checkbox with no controllable value is not sufficient acceptance. This exact child artwork must be approved in the canonical files before implementation. Preserve existing collapsed form meanwhile. No silent fixed offset.

Curved targets require an exact swept-volume/surface intersection and trimming route, with branch/coverage rules above. Analytic ray/face intersections can help resolve feasibility, but tessellation hits cannot define the cap. Initial delivery may land planar Up-to first; curved targets remain a declared capability gate, not a passing “Up to” checkbox. S0 must identify the supported target classes and either demonstrate the pinned BrepKIT route or request a human-approved supported-domain amendment. No kernel upgrade is implied.

### 6.4 Symmetry, second side and offsets

Proposed Blind symmetry meaning: Depth is total span, hence default boundaries `[-L/2,+L/2]` about the start reference. Starting Offset moves the center. With Up-to, resolve the complete forward boundary then reflect it about the profile/start reference; Through-all uses equal spans large enough to cover applicable geometry in both directions. No independent second-side geometry/dependency while Symetric is on. Main end adjustment is mirrored as part of the resolved half-extent. Surface and Solid share these coordinate rules. The half-depth choice and translated symmetry center require explicit plan review under G4 because the UI reference does not settle them.

With independent second side, resolve main extent along d and second extent along -d, each with its own positive magnitude or target. The resulting sweep joins through the common profile/start boundary without duplicate internal caps; for offset starts, use the same shifted join boundary. A second-side direction prefix reverses its local direction; if it makes both extents point to the same side or invert the valid interval, report InvalidSideDirections rather than taking an absolute value. A two-sided extrusion must not leave an unannounced central gap.

Second Up-to exposes its nested End Offset exactly as documented. Its Distance is unsigned in the reference; propose “shorten from the second target toward the join by this amount,” with nonnegative value. Its Target replaces that second-end limit with another selected target. Distinguish this from signed main End Offset. Retain all mode/target identities and recompute from active branches; do not convert second-side Up-to to an optional scalar depth. Main Up-to/Through-all do not grow an End Offset row absent a spec amendment. Combining symmetry/start/up-to or second-side/nested offsets uses the same boundary resolver; a combination is rejected only for an explicit geometric invalidity or an identified pending capability, never silently ignored.

### 6.5 Result modes, surfaces and merge scope

| Geometry/result | Required behavior | Validity/implementation condition |
|---|---|---|
| Solid/New | Extrude closed region(s), with holes, into independently identified manifold body components. Leave prior bodies intact. | Open wire invalid; no targets needed for Blind. Up-to/Through-all still need their limiting geometry. |
| Solid/Add | Fuse swept volume with all applicable touching/intersecting targets or selected compatible solids. | No target/no actual merge is a reported no-effect error, not disguised New. Handle multiple connected output components and lineage. |
| Solid/Remove | Subtract swept volume from applicable target solids; return remaining connected bodies. No tool body persists. | Empty explicit scope invalid. Valid complete removal consumes the entity and records zero outputs. A nonintersecting target is unchanged; if all targets unchanged, no-effect error. |
| Surface/New | Sweep open wire(s) or boundaries of a closed region into uncapped exact sheets; holes add inner walls. | Use actual kernel face/shell representations, area and boundary validation; not a transparent solid. |
| Surface/Add | Fuse/join generated sheet faces with compatible surface targets, split intersections and remove duplicate overlapping regions, preserving an open sheet result. | Requires proved sheet topology/intersection/sewing route. Mere mesh concatenation or a solid boolean on an open shell fails acceptance. |
| Surface/Remove | Remove target sheet regions covered/cut by the swept sheet under an explicitly reviewed trim convention; preserve complementary patches. | Transverse sheets have intersection curves but no volumetric “inside”; this semantic decision and exact trimming capability are G3, not assumed from Solid Cut. |

For all modes: multiple profiles, holes/open chains where appropriate, both directions, every extent and active modifier remain in the acceptance matrix. Reject self-intersecting/degenerate sweeps and nonmanifold results. Solid and surface targets are separate compatible-kind sets; no accidental solid tool applied as a surface cut or implicit thickening.

Automatic Merge with all resolves applicable targets in the base committed revision, deterministically ordered by EntityId. For Add/Remove, broad phase uses exact/conservative world bounds; narrow phase establishes actual geometric interaction through kernel operations. Through-all limiting geometry is resolved independently as above. Visibility/selection in other windows has no effect on model targets. Explicit MergeScope deduplicates selected live compatible entity IDs, validates revisions, excludes profile/tool/transient preview and future descendants, and affects only those targets. Explicit disjoint targets may remain unchanged, but at least one target must change. New ignores dormant merge settings and stores no merge dependencies.

Perform every target operation in private candidate topology. If any intended target fails, discard the entire command rather than publishing partial changes. Preserve unaffected entities and geometry bindings; replace/consume affected ones according to lineage. Splitting into multiple bodies and fully consumed targets must work despite the current one-feature/one-entity API and upstream SolidId result. Enumerate output connected components and validate them, preserving cavity-shell orientation and ownership; do not infer successful whole-body removal from a generic Operation error. Upstream typed EmptyResult may be translated to a valid zero-output Remove only when that specific boolean outcome is established; also normalize a successful empty-solid sentinel through the verified topology query. Keep either representation internal to the ACL.

Boolean fallback quality gate: the verified `mesh_fallback_count` documents loss of analytic types and no watertight guarantee. For this roadmap's real B-Rep operations, reject such a fallback unless a future explicit accuracy policy permits it; retain valid state and identify the unsupported fixture. A runtime error on an exceptional degenerate input is acceptable; routine required matrix cells failing due to kernel capability keep that scope incomplete.

## 7. Preview, workers, commit and rollback

Use one dedicated geometry thread for the shared document with a small queue: at most one running calculation and one newest pending preview per window, plus explicit commit work. A ready commit has priority over pending previews; commits run FIFO by accepted Confirm intent. After commits, choose one window's latest preview per turn in round-robin window order, then advance the cursor. Replace any older pending preview for that window immediately and discard it without work. A running BrepKIT call cannot be preempted, but its stale result is discarded and the next job follows this priority rule. Thus preview bursts cannot starve a commit and one active window cannot starve another's preview. Standard library channels/cancellation flags and `winit::EventLoopProxy<AppEvent>` suffice; no async runtime/thread pool dependency. A queue mutex may guard replacing a tiny pending request but never kernel work or a topology clone; event-loop enqueue cannot wait for queue capacity. Build/copy topology, parse/evaluate/replay history, booleans, bounds and tessellation in the worker. Keep GPU calls on the event-loop thread. Validate only trivial field bounds immediately; no geometric queries/parameter graph traversal on every pointer move.

### 7.1 Request/result protocol

`JobKey = (DocumentId, base DocumentRevision, WindowId/window_generation, CommandId, DraftRevision, RequestId, purpose)`. A job captures immutable committed input plus a frozen draft; monotonically increasing IDs distinguish identical text submitted later. Parsed/semantic failures invalidate preview just as kernel failures do. Coalesce edit bursts at an event-loop idle boundary; no polling/continuous redraw loop or decorative progress animation. Existing status/progress demo behavior remains independent.

Worker result is `JobFinished { key, Result<PreparedCandidate, CommandError> }`. Result delivery posts a user event waking the event loop; no blocking receive/join on input. Check cancellation between parse/evaluation, feature steps, boolean targets and tessellation; BrepKIT calls need not be forcibly interruptible. Once cancelled, a running native call may finish, but its result is discarded. Do not invent a kernel cancellation API or rely on stopping a Rust thread. Retain/release topology and prepared candidates on the worker where practical so obsolete large geometry destruction does not stall input.

On receipt accept only if document/session/window/draft/base/request/purpose all still match. Drop old errors as well as old successes. A result for a closed window, cancelled command, another document or earlier draft must not change scene, UI or model. Shared-session shutdown closes submission and signals cancellation; thread finishes without the UI waiting for noninterruptible work. Closing one window cancels only that window's jobs; a document or another window remains live.

### 7.2 Preview scene

Retain an immutable committed scene packet in each window. A preview is a separate transient candidate scene of the **whole predicted result**: keep unaffected bodies and replace affected bodies in that candidate only. Other windows continue rendering committed geometry. Explicitly label state through existing approved text facilities; no new colors/opacity/highlight or UI pass. Rendering may use additional existing 3D batches for multiple bodies/sheets, but exactly one draw call still renders the ASCII overlay.

Renderer API becomes value-based `prepare_scene(&ScenePacket)` and `install_scene(PreparedScene)`/`clear_preview`; packet contains meshes, optional profile lines, range bindings, bounds and generation. `PickingScene` is built from the same candidate mesh packet and installed atomically with it. Preview output is not eligible as a persistent selection/target; allow picks on unchanged committed entities whose maps survive. Navigation can use the displayed candidate geometry for contextual pivots, retaining M3's no-reframe mesh replacement. Every replacement/cancel invalidates capture, bounds cache and stale PickHit IDs while retaining camera pose.

Invalid new input immediately removes a previous valid preview and restores the retained **current** committed scene, keeping diagnostics and draft text. Never show an old valid preview as if it represents an invalid draft. No geometry upload on an unchanged draft/request; results drive redraws, not an idle loop.

### 7.3 Confirmation and failure atomicity

1. Consume one explicit Confirm intent; finish active child edit, validate/freeze its exact draft and mark ConfirmPending. Keep editing session open until success; edits during pending confirmation cancel that confirmation and issue a new draft revision.
2. Reuse a ready candidate only for the identical base revision and draft fingerprint; otherwise submit a commit-purpose job. No UI-thread geometry or metadata-first commit. Worker returns a complete model/parameter/geometry/entity candidate and CPU scene/picking data.
3. Prepare the origin window's GPU resources off the live scene. If resource preparation fails, publish nothing, keep committed document/scene and retain editable draft with error. Other windows independently prepare their committed rendering after notification; a failed window displays its last coherent revision with an explicit stale/error state and cannot pick it as current.
4. In one event-loop turn recheck base revision/command key, then replace the shared committed snapshot and install the matching prepared origin scene/picking packet. Publication and installation are infallible swaps after preparation. Increment DocumentRevision exactly once, emit DocumentChanged, acknowledge UI confirmation and clear the draft/preview. No repeated footer state can create another feature.
5. Fan out to other subscribed windows. They may install later but must never combine a new mesh with old identity maps. Disable picks/command commits on stale window packets until synchronized. If another commit wins during preparation, discard/recompute; never partially append metadata or advance allocators.

Candidate validation includes profile/parameter/model semantics, geometric validity, result quality, entity maps and tessellation. Any failure discards all prepared mutations. Cancelling before publication advances command/request generation, invalidates pending work, drops preview/child editors and reinstalls the retained committed packet without recomputing a cube or replaying history. With no intervening external commit, document contents, counters, geometry, scene mesh/map and revision equal the opening committed snapshot exactly. If another window committed, Cancel shows that latest committed snapshot; it does not roll back another window's valid work. Camera navigation performed during editing is per-window interaction and is retained, not undone as geometry rollback.

## 8. Dependency-ordered delivery slices

File paths below are repository-relative and are exact proposed future Executor scope. “Modify lib/main” includes module/test registration only as needed. Every slice changes its listed files/tests only, after the approved revision and current baseline are established. No slice is authorized for execution now. Tests marked new are to be created by a future Executor.

### S0 — Capability proof and semantic gates

**Prerequisites:** human review of revision 2; reproducible input baseline including chosen local changes. **Owner:** kernel implementer with Architect/human decisions at G2–G4.

**Future files:** `crates/newell-kernel/tests/it/extrude_capabilities.rs` (new), `tests/it/main.rs`; focused ACL additions in `src/lib.rs` and new `src/{profile,extrude,surface}.rs` only as approved. Capability outcomes update this plan in a separately authorized revision, not canonical documents by implication.

**Work/interfaces:** prove in pinned 4.2.2: closed polygon/hole/circle extrusion, open-wire sheet construction, oblique plane clipping without hole loss, multi-component/empty booleans, face evolution quality and fallback detection. For Surface/New, Add and Remove, produce a decision package naming each exact sheet construction, intersection, split, join, duplicate-region removal and target-trim operation; distinguish coplanar overlap, transverse intersection, disconnected targets, holes and complete removal; state the compatible target classes and Surface Remove convention. For each, include a minimal input fixture, expected topology/area/boundary result, measured result or explicit failure, and the precise public 4.2.2 API/Newell ACL algorithm that would implement it. Do the same for claimed curved Up-to target classes. A source-level possibility alone is not proof of a successful operation. Decide the approved target domain, half-depth and Up-to Offset disclosure before affected UI implementation. Inspect relevant upstream algorithms further as necessary, not APIs from other versions.

**Tests/exit:** measured volume/area, manifold/boundary checks and real component counts, not only screenshots/nonempty meshes. Record each matrix category as proved, unsupported or unresolved with fixture. Present the Surface decision package for human review before S9 starts. If any required sheet operation lacks an exact route, stop S9: the next action is a new reviewed plan choosing a permitted kernel enhancement/version or an explicit M4 scope amendment, with affected tests and acceptance rewritten. S0 can unblock ordinary Solid/Blind infrastructure while Surface/curved gates remain open, but full M4 completion cannot bypass them.

### S1 — Document identities, immutable candidates and entity catalogue

**Prerequisites:** reviewed data contracts; does not depend on difficult geometry proof. **Owner:** model.

**Future files:** modify `crates/newell-model/{Cargo.toml,src/lib.rs}`; create `src/{ids,document,entity,feature,selection}.rs`; create `tests/it/document.rs`, `tests/it/selection.rs`; register in `tests/it/main.rs`.

**Work/interfaces:** checked typed IDs/revisions, zero/many feature outputs, current entities/lineage, pure candidate edits, geometry generation and DocumentChanged payload; add only Newell kernel dependency for immutable snapshot ownership. Preserve public compatibility deliberately while removing false one-entity assumptions. Separate semantic selected refs from UI instance ownership.

**Tests/exit:** stable IDs through rename and unaffected edits; cross-document/stale rejection; zero/multi-output catalogue; candidate failure leaves exact committed counters/bytes/revision; overflow returns error. No UI/GPU/upstream leakage. Existing simple transaction tests are adapted to the approved semantics, not retained as claims of real extrusion.

### S2 — Real profile infrastructure and usable starter input

**Prerequisites:** S1, S0 basic profile proof, G2. **Owner:** model/kernel; app integration owned by app.

**Future files:** create `crates/newell-model/src/profile.rs`, `crates/newell-kernel/src/profile.rs`, `crates/newell-app/src/starter_document.rs`; modify corresponding `src/lib.rs`, kernel `src/lib.rs`, app `src/app.rs`, `crates/newell-ui/src/shell/data.rs` only for mapping the existing File/New action (no new artwork). Create model `tests/it/profiles.rs`, kernel `tests/it/profiles.rs`, app `tests/it/starter_document.rs`; register all.

**Work/interfaces:** ProfileDefinition and validation, ACL face/wire construction/copy, document registration, real bootstrap profiles/targets and approved File/New loading. Keep test fixture data separate from production starter definitions. Add profile drawing/picking through S5/S7; tree selection suffices until then.

**Tests/exit:** closed polygon, hole and circle are genuine geometric profiles; open chain valid for Surface only; nonplanar/self-crossing/zero-length/invalid holes rejected. A user can obtain and select a real profile before invoking Extrude. A seeded box body is not itself a profile.

### S3 — Units, parameter graph and bounded expressions

**Prerequisites:** S1. **Owner:** model. May proceed separately from kernel S2 after contracts freeze.

**Future files:** new `crates/newell-model/src/{quantity,parameter,expression,recompute}.rs`, modify `src/{lib,feature,document}.rs`; new `tests/it/{units,expressions,recompute}.rs` and registration. No added third-party dependency.

**Work/interfaces:** §5 quantities/AST/binding/DAG and immutable evaluated candidates, active-field dependency extraction, feature parameter ownership and deterministic recomputation ordering. Model errors carry source spans/owner fields.

**Tests/exit:** conversions, dimension algebra, canonical formula example, precedence/unary/scientific numbers, `min(1mm, 2mm)`/`max(1mm, 2mm, 3mm)` and comma/arity failures, missing names/cycles/division/nonfinite/nesting bounds, hidden invalid field exclusion. Rename tests cover committed `#old` and bare migration references, source-span rewriting, Formula reopen, duplicate-name rejection, over-cap rollback and a concurrent invalid draft that stays raw and requires revalidation. Dependent features update together or all roll back. Raw text round trips independently of displayed one-decimal values.

### S4 — Typed command intents and parameter-bound Action Panel

**Prerequisites:** S1/S3 and semantic enum contract; S2 needed only for successful interactive geometry. **Owner:** app/UI.

**Future files:** new `crates/newell-app/src/command.rs`, `crates/newell-ui/src/command.rs`; modify app `src/{lib,app,controller}.rs`; UI `src/{lib,extrude,create_box,action_box}.rs`, `src/shell/{mod,key,compose}.rs`, `src/widgets/numeric.rs`, `src/overlays/formula.rs`; new app `tests/it/commands.rs`, UI `tests/it/command_binding.rs`, respective `tests/it/main.rs`; extend `src/extrude_tests.rs` and app `tests/it/controller.rs` only with specification-backed assertions.

**Work/interfaces:** §4 lifecycle/intents, all field keys mapped independently, consumed-once OK/Cancel, modal child transaction acknowledgements, editor raw/parse/value separation, separate direction flags. Eliminate `sync_extrude_command` polling and zero-filled adapter paths. Other command panels continue their UI behavior without generic framework expansion.

**Tests/exit:** no command begins from unrelated key; each edit affects the intended draft parameter; formula errors cannot close the editor/commit; mode switch preserves inactive drafts without evaluation; repeated OK emits one request; Cancel never submits commit. ASCII dimensions and hover/precedence tests unchanged except dynamic semantic data.

### S5 — Real tree projections and per-window selection

**Prerequisites:** S1/S2; S4 selection-slot contract. **Owner:** app/UI. Renderer packet contract can be developed from S7 interfaces first.

**Future files:** new app `src/{projection,selection}.rs`, UI `src/shell/tree.rs`; modify app `src/{lib,app,controller,picking,navigation}.rs`; UI `src/shell/{data,mod,key,compose}.rs`, `src/widgets/tree.rs`, `src/overlays/selection.rs`; new app `tests/it/selection.rs`, UI `tests/it/document_tree.rs`; extend app `tests/it/tree.rs`, UI `src/shell/tree_tests.rs` and test registration.

**Work/interfaces:** dynamic flat row DTOs and token→ID lookup, actual output counts, rename/visibility, filtered generic/profile panels, common selection reducer, scene range→entity/topology mapping. Remove mock arrays from real document paths; keep canonical reference harnesses explicit.

**Tests/exit:** empty/large/multiple-result document, duplicate labels, scroll/collapse/reorder/delete, checkbox versus selection distinction, cross-document/generation invalidation, field filtering, profile versus merge scope. For each limit slot, a face picked in the viewport and its modal row resolve the identical generation-bound TopologyRef; a datum resolves its EntityRef; a whole-body row is rejected as a limit. Stored targets round-trip through draft reopen and stale face generations require reselection. Route UI pointer target in O(1); CPU geometry query complexity remains separately permitted.

### S6 — Solid/Blind extrusion and explicit boolean scope

**Prerequisites:** S0 Solid capability proof, S1/S2/S3. S4/S5 for end-to-end UI acceptance. **Owner:** kernel/model.

**Future files:** new kernel `src/{geometry,extrude,boolean,error}.rs`; modify kernel `src/lib.rs`; new model `src/extrude.rs`, modify `src/{feature,recompute}.rs`; new kernel `tests/it/{extrude,boolean}.rs`, model `tests/it/extrude.rs` and registrations.

**Work/interfaces:** GeometrySnapshot/scratch arena, ResolvedExtrude, profile-based Solid/New, real fuse/cut with automatic/explicit targets, multi-output/empty removal and lineage. Do not infer geometry from displayed mock strings. Measurements/validation and fallback errors are ACL-owned.

**Tests/exit:** correct dimensions and volumes for rectangle/circle/hole; New retains other bodies; Add physically fuses; Remove physically cuts/splits/consumes; hollow-body cavity shells stay cavities after component enumeration; typed empty-Cut and empty-sentinel results normalize correctly; untouched target IDs and shapes remain; one target failing aborts all; disjoint scope no effect diagnosed; invalid open/degenerate profiles fail without mutation. No fixed box scaling remains in this path.

### S7 — Asynchronous geometry, revisioned scenes and atomic transactions

**Prerequisites:** S1/S4 and S6 working kernel result; S5 identity maps. Can scaffold worker protocol before S6 without claiming geometry acceptance. **Owner:** app, renderer, kernel snapshot boundary.

**Future files:** new app `src/{geometry_worker,document_session,scene}.rs`; modify app `src/{app,lib,window,picking,navigation}.rs`; modify model `src/{document,recompute}.rs`, kernel `src/geometry.rs`; modify renderer `src/{lib,renderer,scene}.rs` and `src/scene.wgsl` only if multi-body/sheet drawing needs it. New app `tests/it/{geometry_worker,transactions,scene_updates}.rs`, renderer `tests/it/scene_updates.rs` and registrations; extend existing navigation tests.

**Work/interfaces:** §7 JobKey, cancellation/coalescing, immutable geometry result, EventLoopProxy completion, retained committed/candidate packets, two-phase GPU prepare/install, exactly-once publication, automatic recomputation through same path. Preserve navigation camera pose and event-driven redraws.

**Tests/exit:** deterministic fake worker delivers successes/errors out of order, blocked kernel does not block processing a Cancel/input, stale revision never commits, closing a window never resurrects it, exact rollback after multiple successful prior features, GPU preparation failure leaves live state, parameter recomputation failure rolls back. A sustained stream of preview edits cannot delay a queued commit after the running call; distinct windows receive preview turns in round-robin order and replaced previews never run. A real Solid/Blind command traverses this worker/preview/commit path. Do not use timing sleeps as a concurrency oracle; use barriers/manual completion channels.

### S8 — Full extent/modifier resolver

**Prerequisites:** S0 extent proof/G4, S3/S6; integrate through S7. **Owner:** kernel/model with app/UI adapters.

**Future files:** new kernel `src/extent.rs`, model `src/extent.rs`; modify kernel/model `src/{lib,extrude}.rs`, app `src/command.rs`, UI `src/extrude.rs` for approved data bindings only; new kernel `tests/it/extents.rs`, app `tests/it/extrude_matrix.rs` and registrations. Any new Up-to Offset child awaits a separately approved canonical amendment, then explicit Executor file scope revision.

**Work/interfaces:** independent typed targets, exact planar then proven curved Up-to, geometric Through-all bounds, Along, all offset modes, symmetry and independent second side/nested offsets, coherent combinations. One boundary resolver drives preview and committed geometry.

**Tests/exit:** §10 matrix with numerical boundaries/volume, not only presence of flags; viewport-face/modal-row/field-target identity round trips for every limit slot, whole-body rejection, stale/deleted face reselection; target behind/parallel/partial coverage errors; Through-all beyond hidden/unselected bodies; inversion, offset start/end, symmetric and second-side targets at both ends; no leakage of inactive branch settings. Unproved curved targets stay G4-open and cannot be silently called complete.

### S9 — Surface/New/Add/Remove through the same transaction

**Prerequisites:** S0/G3 decision package with human-approved exact route and semantics, S2/S6/S8/S7. **Owner:** kernel/model; app/renderer integration.

**Future files:** new/complete kernel `src/surface.rs`; modify `src/{geometry,extrude,extent}.rs`, model `src/{extrude,entity,recompute}.rs`, app `src/{scene,projection,selection}.rs`, renderer `src/scene.rs`; new kernel `tests/it/surfaces.rs`, app `tests/it/surface_transactions.rs`, renderer `tests/it/surfaces.rs` and registrations.

**Work/interfaces:** actual open/closed-boundary sheet sweeps, compatible target fusion/trimming, retained analytic surfaces and boundaries, surface area/components/identity maps; no caps or implicit thickening. Every termination/modifier/scope combination uses the same validated semantics.

**Tests/exit:** zero enclosed volume/open boundary where expected, measured area, holes, oblique/curved approved limits, coplanar overlap join/remove and reviewed transverse intersection behavior, rejected nonmanifold outcomes, multi-surface scope and rollback. If pinned capabilities cannot implement these, halt this slice with exact fixtures/API evidence; human must select a permitted kernel enhancement/version procedure or scope amendment. Safe runtime “unsupported” behavior is not a green M4.6 report.

### S10 — Shared windows and integrated regression acceptance

**Prerequisites:** S5/S7, then S8/S9 for full M4 acceptance; G1 resolution for default-dependent checks. **Owner:** app/integration.

**Future files:** complete app `src/{app,window,document_session,lib}.rs` (WindowContext map and user events), modify `src/{controller,projection,selection,command}.rs`; new app `tests/it/{multi_window,m4_acceptance}.rs`, renderer `tests/it/m4_gpu.rs` and registrations; extend UI `tests/it/{action_boxes,command_bar}.rs` only for approved bindings/actual data fixtures. Do not overwrite another task's theme/menu tests.

**Work/interfaces:** connect shared document subscriptions, independent window states, stale draft revalidation and close/shutdown; exercise complete interactive workflow, no scene-only success path. Settle G2's actual second-window/manual entry surface within approved UI scope.

**Tests/exit:** two windows see same committed geometry/tree revisions and independently selected entities/cameras/themes; Cancel in one preserves other's commit; close during computation affects only owner; command preview stays local; all automated and manual scenarios in §10 recorded with material limitations. No automatic workflow transition or merge.

### 8.1 Coverage, critical path and separately assignable boundaries

| Requirement | Primary delivery | Supporting slices |
|---|---|---|
| M4.1 command interface | S4 | S1, S7, S10 |
| M4.2 real Features/Elements | S5 | S1, S2, S6, S10 |
| M4.3 consistent selection | S5 | S1, S4, S7, S10 |
| M4.4 units/bound numeric values | S3, S4 | S8 |
| M4.5 formulas/recomputation | S3, S4 | S7 |
| M4.6 Solid/Surface × New/Add/Remove | S6, S9 | S0, S2, S5, S7 |
| M4.7 Blind/Up-to/Through-all | S6, S8, S9 | S0, S7 |
| M4.8 Invert/offset/symmetric/second end | S8, S9 | S3, S4 |
| M4.9 all/explicit Merge scope | S6, S9 | S5, S8 |
| M4.10 preview/confirm/cancel/rollback | S7, S10 | every geometric slice and S4 |

Critical path: review/gates → S1 → S2/S3 → S4/S5 + S6 → S7 → S8 → S9 → S10 full acceptance. S0's surface/target proof can become the controlling critical path; do it early, not after calling Solid-only work M4 complete. S10's WindowContext/session scaffolding can be prepared after S1/S7 contracts instead of postponing ownership problems to the end.

If the human later assigns parallel work, freeze plan revision and interface DTOs first. Candidate boundaries: S3 parameters versus S2 profile ACL; S4 UI adapter versus S6 kernel; scene packet renderer work versus model expression work. `app.rs`, `lib.rs`, manifests and shared tests are integration hotspots: assign one writer per integration task. Each writer needs a separate checkout/branch at the same identified baseline plus explicitly authorized dirty inputs. No writer changes another checkout/index or merges into main. No parallel work or agent is started by this document.

## 9. UI/regression protection and proposed amendments

Preserve the canonical 31-cell Action Box, fixed field columns, all complete base siblings, branch-specific progressive disclosure, exact labels (`Symetric`, `Profils`, `Starting Offset`/`Start Offset`, `Second end position`/`2nd side`) and nested offset glyphs, as specified in §§2.2–2.6, 7–8, 18–22, 32–34. Bind real counts/values without rewriting the ASCII grammar. Preserve one-character/one-cell caret editing, focus restoration, static border-only M2.25 hover and the flat CellGrid. Keep direct division/subcell modulo routing and direct WidgetId dispatch; dynamic trees must not introduce bounding-box hit scans. Exactly one UI draw call; no UI DOM, additional UI pass or decorative animation.

Preserve menu/modal/resize/captured-gesture precedence, no click-through, physical cell geometry for all k=2…7 and per-monitor scale transitions. M3 camera contract remains `docs/plans/M3-camera-mechanics.md` and architecture §4.4: zero roll, pivot on press, signed pole clamp, anchor-preserving zoom, contextual pan, no reframe on mesh change, fit only via appropriate Home/reset and cancellation of captured drags on mesh/layout/projection changes. Preview replacement changes geometry bounds/maps but never automatically fits the camera.

**Proposed amendments, not approved/applied here:** (a) resolve Light-versus-Dark default authority and retain local Theme popup behavior; (b) exact Up-to Offset expanded magnitude/direction artwork; (c) documented use of existing status text for validation/pending/stale states or exact new error/disabled-control artwork if existing facilities are inadequate; (d) agreed Surface Remove and curved target support domain; (e) profile bootstrap and second-window manual entry if new visible actions are required. Geometric modeling defaults with no visual change can be accepted through plan review; changes to canonical UI need the canonical files amended by a separately authorized task. Keep existing reference artwork/snapshots until there is an approved reason to change them. Real document-data snapshots should be additional explicit fixtures, not a mass regeneration of reference snapshots.

## 10. Verification and acceptance plan for a future Executor

### 10.1 Layer tests and geometric oracles

| Layer | Meaningful tests |
|---|---|
| Model | Checked ID allocation, zero/many outputs/lineage, canonical units/dimensions, actual reference formula, function separators/arity and parsing failures/spans, source-preserving rename/reopen/deletion, active dependency graph, candidate parameter/history replay and all-or-nothing failure. |
| Kernel | Exact profile validation and true extrusion bounds; solid manifold/orientation/volume and sheet boundary/area; circle and holed region; degenerate/self-intersecting/tangent profiles; exact target clipping and tolerances; component splits/complete removal, boolean fallback detection and error translation; no upstream types in public APIs/dependencies elsewhere. |
| Controller | Typed field/selection intents, exact face/datum option mapping and stale revalidation, no status polling/duplicate confirm, raw/parsed/evaluated separation, first-class direction, pending edit cancellation, child modal Cancel, commit priority/preview fairness, delayed/stale errors/results, closes, external commits and exact restoration after multiple real prior features. |
| UI | Reference geometry, all six Solid/Surface × result headers and branches, active row binding, dynamic labels/counts, duplicate names, truncation, scrolling, focus and hover, field magnitude versus direction, formula cap, no overlay click-through, all supported scales and themed palettes under the resolved authority. |
| Integration | Starter profile→selection→draft→worker→real preview→one commit→tree/entity refresh; edit parameter recomputes downstream geometry; Add/Remove true target effects; multiple windows with local preview/selection/camera; stale scene pick rejection and atomic model/geometry publication; invalid inputs retain last good state. |
| GPU | Upload/install multiple bodies and sheets with matching maps; retained committed scene restored after Cancel; prepared-scene failure preserves current scene; UI draw remains one batch and atlas/theme behavior stable; explicit ignored/device tests. Manual physical-screen acceptance remains separate. |

Use numerical geometry oracles rather than byte-identical tessellation: rectangle area × span × direction projection, circle `pi*r*r*span`, holed area subtraction, independently calculated boolean volumes, plane residuals at caps, directed Through-all bounds and sheet area/boundary counts. For example a 20×10 mm rectangle swept 5 mm normally gives 1000 mm³ and exact 5 mm span. Rounding text never becomes the geometric expected value. Check near-tolerance inputs on both sides of thresholds; absolute+relative comparison tolerances must be stated per fixture. Golden files can be added as approved simple geometry regression references; never bulk accept changed goldens/snapshots to get a passing suite.

### 10.2 Required extrusion combination matrix

Generate table-driven tests from explicit semantic states, not arbitrary booleans with impossible UI rows. Every geometry/result row below crosses each main termination and each side configuration; offset submodes/targets and merge scopes refine the cells. Invalid cells have typed errors and unchanged committed-state assertions. Pending capability cells are tracked as blocked acceptance, not skipped tests silently counted as passes.

| Dimension | Required values |
|---|---|
| Geometry × result | Solid/New, Solid/Add, Solid/Remove, Surface/New, Surface/Add, Surface/Remove |
| Main termination | Blind, Up-to planar parallel, Up-to oblique, approved curved target classes, Through-all; every Up-to target is a specific face TopologyRef or datum EntityRef |
| Side configuration | One side; Symetric; Second Blind; Second Up-to |
| Direction | Blind positive/negative prefix; Along absent/straight reference; Through-all Invert off/on; second-side orientation conflicts explicitly rejected |
| Start | Off; signed Blind distance; Up-to target where reference exposes Start Offset (Blind and Through-all) |
| Main end | Off; signed Blind distance; Up-to target under Blind; main Up-to Offset off/on after approved disclosure; no invented main End Offset under Up-to/Through-all |
| Second end | Blind positive magnitude; Up-to target with nested End Offset off/unsigned Distance/Target; hidden entirely during symmetry |
| Merge | Dormant for New; automatic all for Add/Remove; explicit one/many; wrong-kind/missing/stale/disjoint targets; partial/no-effect and all-target failure rollback |
| Profile | Rectangle, nonconvex polygon, circle, region with hole, multiple disjoint profiles; open chain for Surface; invalid open Solid, self-crossing, nonplanar, touching holes/zero area |
| Environment | Empty scene, multiple bodies/sheets, hidden limiting entity, duplicate face labels, stale face generation, other-window commit, closed window, out-of-order result, failed GPU preparation |

At minimum run all 6 × 3 main termination classes × 4 side configurations with default valid inputs (72 semantic cases before oblique/curved refinements); every supported mode must have geometric success cases. Add explicit cross-product tests for modifier interactions: symmetry+start+end; second Up-to+nested offset+main start; invert+Through-all+explicit scope; main Up-to+second Up-to+formula-driven limits; holes/multiple profiles+Add/Remove. Cover each active modifier submode against every geometry/result/termination combination where the canonical UI allows it; pairwise coverage alone does not replace these required crossings. Invalid/pending combinations are listed by reason, not removed from generated cases. Full Cartesian parser and geometric-degeneracy fuzzing is unnecessary; property tests should target invariants, not mirror field assignment code.

### 10.3 Required workspace checks (not run by Architect)

Record baseline with snapshot updates disabled, then focused checks per slice and final full checks in the approved Executor checkout:

```sh
cargo check --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
INSTA_UPDATE=no cargo nextest run --workspace --all-features --locked
cargo test --workspace --doc --locked
```

Without nextest use `INSTA_UPDATE=no cargo test --workspace --all-features --locked --no-fail-fast`. Relevant additional checks from current `.github/workflows/ci.yml` and architecture:

```sh
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --document-private-items --all-features --locked
cargo deny check
cargo +1.88 check --workspace --locked
cargo check --workspace --target aarch64-unknown-linux-gnu --locked
cargo test -p newell-render --locked -- --ignored
```

Run target/MSRV/GPU checks where toolchain, target and physical/device support exist; report unavailable checks explicitly. Check every new Rust file's required AGPL header; exact BrepKIT pins and dependency isolation, f64→f32 bounds, no production panic/unsafe. CI covers Linux/macOS/Windows; local success alone does not prove all three. Keep build outputs local and snapshot updates off; classify pre-existing failures separately. Two unsuccessful compilation repair attempts require the workflow's stop/help report. No weakened lint/test or unapproved reference update.

### 10.4 Manual CAD scenarios

1. Load approved starter document; inspect actual Features/Elements counts, select each real profile through tree/panel, verify visible construction/body identity and field counts. Create the 20×10×5 mm rectangle extrusion; confirm one feature/body and correct geometry, then create a second feature. Duplicate names must remain independently selectable.
2. Exercise Solid/New/Add/Remove against one and multiple targets, including a hole, disjoint scopes, split result and whole-body removal. Toggle Merge with all/explicit scope; untouched bodies remain. Invalid profile/target produces a useful error and preserves both prior features.
3. Use mm/in conversion and the reference formula; confirm, edit an upstream value and observe real downstream recomputation. Try unknown reference, cycle, wrong dimension, divide by zero and invalid text; correct or Cancel without corrupting geometry.
4. For both geometry types test Blind with positive/negative direction and Along, parallel/oblique and approved curved Up-to with actual target coverage, Through-all with a hidden/unselected far entity, Invert, both start modes, main end modes, symmetry and second Up-to/nested end. For each Up-to field choose a face in the viewport and the matching named row in the Selection panel; verify identical stored identity, body-row rejection and reselection after a stale generation. Compare modeled dimensions and boundaries, not just visual plausibility.
5. Exercise Surface/New/Add/Remove with open chain and closed boundary/hole, automatic and explicit sheet targets; inspect uncapped boundaries and the approved trimming convention. An unsupported required operation is a failed acceptance item, not a visual substitute.
6. Rapidly edit values/modes while a deliberately delayed calculation runs; Cancel and close a window during computation. No late scene/document change; after Cancel geometry matches the last committed multiple-feature model exactly. Confirm once during computation; no duplicate feature from repeated Enter/OK.
7. Open two views of the shared document through the approved path. Keep different selections/cameras/theme/expansion, preview in one only, commit in the other and verify draft invalidation/recompute, then Cancel. Close one view with work pending; the other retains the document.
8. Orbit/pan/zoom/Home around candidate geometry; open menus/modals and resize/scale between k=2…7, with Retina and external Full HD if available. No camera jump on preview/Cancel, click-through, broken ASCII joins/field columns, selection/visibility confusion or theme-default regression. Human screen/device acceptance is recorded separately from GPU tests.

## 11. Completion, unresolved decisions and handoff

**Milestone 4 is complete only when** every backlog item M4.1–M4.10 maps to delivered, tested behavior: a user obtains real profiles, edits typed/unit-aware formula-backed parameters through existing panels, selects consistent real entities across viewport/trees/panels in each window, computes every approved valid Solid/Surface × New/Add/Remove extent/modifier/scope combination in BrepKIT, previews asynchronously, confirms once into shared model geometry, and cancels/fails without losing any valid committed state. Parameter edits recompute dependent history atomically. Trees represent real zero/many outputs. Required automated checks and manual scenarios have reported outcomes; accepted exceptions need explicit human scope approval. Required surface/curved/offset cells cannot be marked complete merely because a safe Unsupported error exists. No UI/MVC/kernel boundary or M3 navigation regression is introduced.

Unresolved: G1 default conflict; G2 approved interactive profile/second-window entry; G3 exact Surface Add/Remove semantics and pinned capability decision; G4 Up-to Offset artwork and curved target domain, plus symmetry/offset coordinate conventions for human review; G5 review of the proposed face/datum selection contract before dependent UI work. Known prerequisites: profile producer absent, single-window runtime, no sheet/real extrusion wrapper, no parameter graph, no atomic geometry-backed command. A full sketch editor and file persistence remain outside M4, but their absence must not make the promised command unusable.

**Recommended Challenger scope:** review this revision's G3 stop gate and G5 face/datum option contract against the canonical Selection panel; verify parser commas/arity and rename source behavior, plus commit priority/window fairness. Scrutinize common-arena ownership/copies and Send/Sync of opaque snapshots, model↔kernel dependency direction, checked identities and face-evolution conservatism, two-phase GPU/model publication and stale-window picks, cancellation/shutdown without event-loop waits, complete-remove/component lineage, units/direction/symmetric offset mathematics, child-edit precedence and retained committed scene restoration. Challenge the approved profile entry workflow and matrix completeness before approving implementation. Review this exact revision and Appendix A audit snapshot; re-review affected inputs if source hashes change.

Architect handoff: only this proposed roadmap was written, in its separate worktree. Source changes and Git index were preserved. No implementation checks were run; all test/build/manual work above is prospective. Human review, then an explicitly assigned Challenger review, is the recommended next stage. No stage or agent is started automatically.

## Appendix A. Identifiable uncommitted input snapshot

Source status was nine unstaged modified tracked files, no staged changes or untracked files at initial inspection. SHA-256 hashes below identify the initial full working-file bytes consumed. A handoff comparison found one externally changed input, `docs/ui_design/Application_Menu_Bar.txt`; all other scoped input hashes were unchanged. Its new Color row was re-read and assessed under G1. Both input snapshots are identified below; this is not a claim of a stable source checkout throughout drafting. No dirty source files were copied, stashed, reset, staged or committed.

| Working-file path | SHA-256 |
|---|---|
| `crates/newell-render/src/scene.rs` | `8c8357dc3276f6db28e3886544e2008ab61d43dd5b261266c19294acdff9c5be` |
| `crates/newell-render/src/ui/theme.rs` | `fa45f7911ec42f77e061d685de580cda8da3103b01ee8795d8606c332d2276e8` |
| `crates/newell-render/tests/it/ui_gpu.rs` | `1f8c02ee5f3f5454a4fff8bacf6e32b8737d77a7d149a4393a4875118a083cfe` |
| `crates/newell-ui/src/shell/data.rs` | `371a34d7839f30756592c29dc38af7ad0069fc0d8801281518d106c2805e9bee` |
| `crates/newell-ui/src/shell/mod.rs` | `08af6da5057bf20f5f12e1b0cac6002e56babefa316d2d42e416bd61a80bb73e` |
| `crates/newell-ui/src/shell/tests.rs` | `099e51c205a8e75eafa48ec3e74ae0c91cb997acbdc992ad64b4483901b5e54c` |
| `crates/newell-ui/tests/it/menu_bar.rs` | `ce3d0b236e2a238fc89418ed587bd7eddc2cb0b02d6c14dbde5121c04a95b82a` |
| `docs/ui_design/Application_Menu_Bar.txt` | `02ab8b5041a47e5e2459d9438879bbd83fbca6cd05dfd3e6f3ebad8c9d045672` |
| `docs/ui_design/README_UI.md` | `7af81339b1cc3eb119cbf710611bb63467fdc83a793e39a40f95d70e4bf9b811` |

The baseline-to-working-tree binary diff (`git diff --binary HEAD` in the source checkout) has SHA-256 `0e5c11969a9eae324e58f178b53b512f5765eca6cc961a9012952b645f017e89`.

The initial scoped 139-file input manifest has SHA-256 `13341511c2de7d37fd3878e3d8a2d1176d54ed98d6f405438e58b1a8a84da29c`. Reproduction: collect all files with extensions `.rs`, `.toml`, `.wgsl` recursively under the five named `crates/newell-*` directories, plus `AGENTS.md`, `ARCHITECTURE.md`, `Cargo.toml`, `Cargo.lock`, `docs/agents/README.md`, `docs/agents/architect.md`, `docs/backlog.md`, `docs/ui_design/README_UI.md`, `docs/plans/M3-camera-mechanics.md`, `docs/adr/*.md` and `docs/ui_design/*.txt`. Deduplicate paths, sort repository-relative POSIX paths lexically; each UTF-8 manifest line is the lowercase SHA-256 of file bytes, two spaces, the path, then LF. Hash their concatenation. Hashing identifies input contents; it is not a build/test result. Extra context inspected: `.github/workflows/ci.yml`; local 4.2.2 upstream sources named in §6.2 and their tolerance/evolution/topology definitions. No target output or binary fixture body was read.

### A.1 Input change observed during drafting

The later menu-reference SHA-256 is `eb66c96e8f55b0205dca7abbe0d91326167185bdf2d005dc50d4689948bc75ea`; its initial hash is retained in the table above. The later baseline-to-working-tree binary diff SHA-256 is `4fcf12fa3b3bb25292f7bef13a5a3d01e4ac4a15a52688e1b28633aa5c715a5a`. The only scoped input change detected between the two snapshots was the added `│ □ Color           │` row and terminal-newline removal in that reference. The roadmap's G1 assessment was updated after reading it. Future reviews must check for subsequent changes as well.

### A.2 Revision-1 verification pass and preservation of the existing draft

On 2026-10-20 the source checkout and existing writing checkout were independently verified at `1eb4a9a4b9fb45a4bbfaa223e348c0e5e9a69f1b`, on `main` and `codex/m4-roadmap-r1` respectively. The writing checkout contained only this untracked plan. Its opening SHA-256 was `827484984eebfb37de436ee7626f0b62bb4fec0f72087c2c1136ad2744c9266c`. Existing sections and historical snapshot records were retained; this pass clarified checkout provenance, public/private upstream APIs, empty/component/cavity handling, shared-snapshot constraints and Through-all coverage across entity kinds. This remains proposed revision 1, without human approval or a workflow transition.

At that pass the source snapshot was HEAD plus the nine paths in Appendix A, with `Application_Menu_Bar.txt` at the **later** A.1 hash and all other eight hashes unchanged. The then-current binary diff SHA-256 was `4fcf12fa3b3bb25292f7bef13a5a3d01e4ac4a15a52688e1b28633aa5c715a5a`; the then-current 139-file manifest using Appendix A's algorithm was `d4d6bf40e3e9def887a6067db9870f8b2448de5a4793461a7ae5844dd36815f5`. The initial source Git-index bytes hash was `1c8c97d509694baff6b24f888e30e68f7e6075bac7d3b9f0995bb5252e124948`; no index operation was performed. Hash computation and source inspection were the only verification performed there, not implementation checks.

Re-inspection covered the shared role/architecture/backlog and ADRs 0001–0004; both core `lib.rs` implementations and manifests; the app's command adapter, startup, controller, picking, navigation interfaces and M3 camera contract; Extrude/UI numeric editing, Formula and Selection state, shell tree projection/routing, canonical Extrude/Interface artwork and relevant README semantics; renderer mesh replacement/UI draw interfaces; representative model/kernel/UI/controller/tree tests and current CI requirements. The exact locally installed 4.2.2 extrusion/boolean/copy/transform/topology/tessellation/splitting/sewing/tolerance interfaces were checked. `section.rs` produces solid/plane cross-sections and `offset_trim.rs` handles offset-surface self-intersection: neither establishes the missing general sheet fuse/cut operation. Numerical kernel behavior, worker trait bounds, current suite health and physical-screen acceptance remain untested and assigned to the future Executor.

### A.3 Revision-2 source drift observation

Revision 2 edits only this plan, using the revision-1 audit and Challenger findings. During this pass the source checkout remained on `main` at the same HEAD, but other local work expanded the dirty tree to 17 modified tracked files, three untracked UI reference files and an untracked `.primary-orbit-work/` directory. The observed baseline-to-working-tree binary diff SHA-256 was `d0c410b0b8f1613f2f2bcf79d0dd060f4f6c32ba6c4a608384b379558b9cf92a`. The seven unchanged modified paths from Appendix A/A.1 retained their earlier final hashes. The ten additional or changed tracked paths, and the three untracked references, had these full-file SHA-256 values at observation:

| Path | SHA-256 |
|---|---|
| `crates/newell-app/src/app.rs` | `fc906a658f412cdfbcf5026bfb22ba9f1920815c9210e2b57c21e4269ac4859a` |
| `crates/newell-app/src/navigation.rs` | `94e1a4622067fcc6377a80cdafaf01e350eb676e79c33ee465166c1a47f9987e` |
| `crates/newell-app/tests/it/controller.rs` | `64369dce3638222342aae29898b51c6429aef3c4f5f9a0718a046e1b2cde2131` |
| `crates/newell-app/tests/it/navigation.rs` | `1ff38898defe0404add6381bec5733f13127454ba6b1ec89664df49c534d5987` |
| `crates/newell-ui/src/create_box.rs` | `76d94b810795ff5a670a9cdf2024d83f790fe585a94e3366005a2a8ae4f36fdf` |
| `crates/newell-ui/src/overlays/selection.rs` | `881d1ff0f0e64acb8d06dadbedb407770606dcf3ad9864454b68ea290a6a433b` |
| `crates/newell-ui/src/shell/compose.rs` | `2845916d410b37141f1800e0ca634a0861f875441f2168f84da3adfd94bf3ed5` |
| `crates/newell-ui/src/shell/mod.rs` | `3500793f52cc246c3895fb1dc579fde0aae7a5779bc89e1e04cad5feb436e385` |
| `crates/newell-ui/src/shell/tests.rs` | `e6749e4d3f2e3af4e142f38e08c09cf5a90e4c0241104ec054ce621f384eecae` |
| `crates/newell-ui/tests/it/snapshots/it__snapshots__selection_panel_open.snap` | `a4699756818b84c30cceb328f8b1f9fd8e09e86aa60e85b73c0268743ce7470e` |
| `docs/ui_design/Action_Box_Axis.txt` (untracked) | `902c43c37f6ee8a2ce1a13751525740fc95c32fbae37b6b9a56a391a3cdae6a0` |
| `docs/ui_design/Action_Box_Construction.txt` (untracked) | `38cf00a81e8cf78affa045ce65eaa21696e3b813e3cc06abc8466c147fac8928` |
| `docs/ui_design/Action_Box_Plan.txt` (untracked) | `7b19f5119a261c2752471ef9b78741e8d1bf5465817ecb61651e519494374ba4` |

These newer files were observed for provenance, not reviewed as M4 technical inputs or copied into the writing checkout. `.primary-orbit-work/` was not inspected or hashed. Any implementation handoff must choose and record an updated source baseline, inspect applicable changes and reconcile affected plan assumptions before execution. No source checkout file or Git index was changed for this revision.

A later status observation during this same plan edit showed further unrelated local turnover: the selection snapshot path and `.primary-orbit-work/` were no longer reported, while modified `crates/newell-ui/tests/it/snapshots.rs` (SHA-256 `1013cf26061828b9f5f56dcebd2dd24fe0251aa1d22da5581a16b61824f1c23a`) and untracked `docs/validation/M3-primary-orbit-2026-10-20.md` (SHA-256 `1e30a1dca0164403819c84d24def61f2223edaebe67062cac5d0fc107158793f`) appeared. The then-current tracked binary diff SHA-256 was `c3bedbe561267eecfb9f7628c193cc092506e352075ccbfc740b505c450d16b5`. These are successive observations of external work, not a new audited M4 input snapshot; the future handoff must capture its own final state.


