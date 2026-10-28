# Selection and Formula reference alignment

Revision 2 — proposed, 2026-10-27. Architect / Codex.
Task: selection-formula-reference-alignment. Intended Executor: Luna with medium
reasoning effort, only after human approval of this revision and an implementation
request. This stage does not start another role.

This is the single targeted Architect correction of revision 1 for the two
human-requested Challenger findings. C1 corrects the Selection footer divider,
button spans and hover boundaries; C2 specifies focus restoration when the
management control disappears and rejects stale empty-field management opens.
No other behavior or product policy is redesigned. Revision 2 is not approved
for implementation by this documentation correction.

## 1. Contract, evidence and boundaries

Work sequentially in `/Users/martin/Desktop/Newell` on `main`. Preserve existing
work and the Git index. No branches, worktrees, checkouts, source snapshots,
commits, pulls, publication or additional agents. The human explicitly permits
targeted README_UI amendments for this task, overriding Architect's ordinary
plan-only write scope. This stage changes only this plan and those amendments.

Primary evidence: current `Interface_Elements.txt` Selection, Profile Selection
and “☰ variants” sections and their unstaged diff; canonical README_UI §§5.5–5.12,
12.1, 14.1–14.2, 18–22, 25/25.1 and 36; architecture §§1,3.0–3.6,4; accepted ADRs
0002,0003,0006; current source paths below. Historical contracts: M4.3 shared
selection; M4.4 units/parameter binding, especially §§6–7; selection-box-scrollbar
revision 2; M4.5–M4.10 Extrude pipeline revision 2, especially I3/I4 and §8.
Current source takes priority over historical implementation descriptions.

No screenshot image is accessible in the supplied attachment: it contains the
request text only. The reported oversized dialog is corroborated by source;
no claim of screenshot inspection or screen-position measurement is made.
Unrelated prose at the end of Interface_Elements, especially navigation, was
excluded from design authority for this task.

Starting state: main; no staged entries. Existing unstaged files:
`ARCHITECTURE.md`; app `app.rs`, `geometry_worker.rs`, `lib.rs`, `parameters.rs`,
`selection.rs`; kernel `lib.rs`; model `geometry.rs`, `lib.rs`, `parameters.rs`;
render `lib.rs`, `renderer.rs`, `scene.rs`; UI docs `Application_Menu_Bar.txt`,
`Interface_Elements.txt`, `README_UI.md`. Untracked inputs:
`crates/newell-app/src/extrude_session.rs`,
`docs/adr/0007-viewport-navigation-widgets.md`,
`docs/plans/viewport-navigation-gizmos.md`.
Relevant existing app changes add typed Extrude candidate/reference plumbing;
README changes concern View Navigator/§41. Preserve them intact. Do not roll
back to pre-M4.5 app or spec files. No concrete write conflict was found for
this documentation stage. During inspection, other work changed shared inputs:
app app/controller/document/navigation/camera and camera tests; UI lib/menu and
shell compose/data/mod/tests; render error/lib/renderer; new app/UI/render
navigation-widget files/shaders. App start-command now calls begin_geometry_command,
and selection opening also cancels nav_interaction. Rereading panel_rows,
open_field_selection, handle_selection_ui, controller publish, shell composition
and the existing two-slot field writer confirmed the selection contracts used
here still apply. Preserve those new lifecycle calls; the allowlist grants no
navigation rewrite. This is a moving-checkout limitation: Executor must reread
all shared affected paths when this stage has exclusive access. No parallel
agent was started by this Architect, and no external work was modified.

Revision-2 entry: still main with an empty index; revision 1 is the untracked
assigned plan. Relevant existing modified inputs include README_UI (also its
unrelated placeholder and navigation amendments), Interface_Elements, app
app/controller and UI shell mod/compose/tests/lib. Other existing dirty work
includes geometry/kernel/model/render, navigation files/tests and snapshots;
input-placeholder-behavior.md is also untracked. Re-inspected the exact footer
reference, draw_ok_cancel, SelectionPanel::sync_focus, ActionBoxUi::live_control,
ShellState::sync and set_selection_fields. The reported paths still apply.
Only this plan and README §§19.1–19.2 are changed in this correction; preserve
all unrelated work and existing reference artwork verbatim.

Excluded: new CAD commands/topology, geometry/kernel/model changes, expression
language/parser/units, navigation widgets, tree scrolling, unrelated modals,
renderer/palette/shader/font changes, dependencies, public publication/integration.
No build, test or application command was run. Static document/whitespace checks
are the only completed verification; all test cases below are future work.

## 2. Mismatch inventory and canonical amendments

| Current evidence | Required correction / canonical authority |
|---|---|
| `overlays/selection.rs` generic SIZE=(30,12), `Geometry::of`, `draw_content`: double frame, separate title row, surplus body | 31×(V+6) rounded titled outer frame, framed list, separator and shared footer; empty 31×7; README §19.1 |
| Profile fixture PROFILE_SIZE=(31,11) and Action Box art already closer; fixed height | Same variable compact height for all presentations, based on rendered dataset, not candidate/selected count interchangeably |
| `SelectionPanel::open_with` always chooses Generic | Explicit presentation supplied by app; purpose derived from semantic descriptor and field part, not label or universal K_PROFILE shortcut |
| Runtime draws every candidate as a checkbox; fixture draws handles/× but both share item activation | Candidate chooser versus selected ordered manager; separate stable row/handle/remove targets; real draft reorder/remove |
| All five panel descriptors currently have ordered=false; Loft Profile/Sweep Profile/Path max_count=Some(1) | Enable ordered management only for those semantic inputs; preserve current max_count and all filters. A singleton handle cannot reorder anything |
| `SelectionOpen.rows` contains eligible candidates; `draft_order` contains selected ids | Retain both: manager iterates draft_order; chooser iterates candidate rows. Empty selected set does not blank a chooser with candidates |
| §20 formerly depicted N=9 offset=0 `┃  ` | Newly edited exact artwork has `╻╹ `, N=9 offset=1; copy it without changing significant spaces |
| §21 says retain old footprint and excludes reorder/remove | Compact §19/20 footprint; unchanged half-slot scroll model; scoped §20 reorder/remove |
| Formula 35×8 already correct, but `draw_art` calls caret_window with 62 cells and no prefix | Compose fixed five-cell prefix plus 57 expression/caret slots; README §22.1 |
| `FormulaEditor::open` takes only first 256 chars | Preserve supplied source verbatim; keep typed-growth cap, allow non-growing edits of longer supplied source |
| M4.4 deferred-open/apply and controller publication already present | Preserve those paths; prefix is composed display only, never an input transformation |

Amendments applied to README_UI in this stage: §§19–20 (exact art, geometry,
purpose/dataset, proposed placement, ordered input), §21.1/21.3 (compact empty
height, runtime data, narrowly scoped reorder exception), §22.1 (empty art,
fixed prefix and exact viewport/editing/transaction rules). Other sections and
Interface_Elements were not edited. §§18 and 5.5–5.12 remain authoritative for
field dimensions, contextual controls, existing keys, roles and hover.

No architecture amendment is needed. Architecture §3.5 already permits documented
ASCII state structures, plain Controller state and text caret. Fixed prefix slots
are equivalent to numeric direction-prefix exclusion; the caret still uses its
own cell and the same linear scrolling formula. O(1) routing remains cell-ID
routing, never a scan of row bounding boxes. Exactly one ASCII draw remains.

## 3. Reference geometry and placement

Implement the normative spans in README §§19.1,20,22.1, using zero-based local
coordinates and half-open ranges. Summary for Executor's arithmetic checks:

- Selection N rendered rows; V=max(1,min(N,5)); width31, heightV+6.
  Title Selection at x2..11, or Profils x2..9, y0. Side frames x0/30.
  Inner frame x1/29, top1, bottomV+2. List x[2,29), y[2,V+2).
  Separator yV+3. Footer yV+4: OK [1,15), shared15, Cancel[16,30).
  Column14 is OK padding. OK hover boundaries are x0/15; Cancel x15/30.
  Bottom yV+5. Empty 7 rows; 3 items 9 rows; 5 or more 11 rows.
- Membership row: padding2, checkbox3, gap4, name[5,29) with no bar;
  with bar name[5,28), gutter28. Widths24/23 respectively.
- Ordered row: padding2, handle3, gap4; with bar name[5,26), remove26,
  gap27, gutter28; without bar name[5,27), remove27, gap28.
  Widths21/22 respectively. Never truncate controls to fit names.
- Formula width35, height8; outer0/34; title[2,9), inner1/33; text rows2/3;
  prefix[2,7) on row2; editable slots[7,33) row2 and[2,33) row3.
  Separator5; footer6: OK[1,17), shared17, Cancel[18,34); bottom7.

The new artwork gives no global coordinates. Proposed policy P1 preserves the
current modal/Action Box split: candidates centered within current shell overlay
bounds, ordered manager at the originating Action Box's existing action_panel
anchor, Formula centered. `profile_selection_frame` must become presentation-
aware and use dynamic height, not all fields named Profile. This is an explicit
proposal awaiting approval with this revision, not evidence from a screenshot.

Keep existing center_modal allocation behavior and modal outside-input blocking.
Derive logical art before clipping: place a fixed logical frame at the helper's
origin, then intersect writes with allocated rect/grid/surface. Never derive a
new footer/right border from a constrained rect's width/height. Even if the
right border is cropped, the logical gutter does not slide left onto a name.
Use clipped Canvas writes and the existing overlay callback to overwrite the
initial generic frame, as Formula already does. Do not change global placement
or OverlayKind to fix Selection-specific artwork. Full shell composition clears
old/new footprint and invalidated controls. Empty inner cells own the surface;
inner frame/chrome own surface; actual item spans own their stable row controls.
Formula inner outline and fixed prefix keep the existing text target. Shared
footer strokes keep Contour ink and use vertical sub-cell modulo routing.

§21 arithmetic is unchanged: N>5 uses M=N-5, L=max(1,11-N), T=6-L,
p=round_half_up(T*s/M), six half slots, existing glyphs and u64 arithmetic.
Chooser N=eligible rows; manager N=draft_order.len(). Offset clamps after changes;
N<=5 removes all bar routing. Initial offset0; no implicit jump after growth.
A captured thumb survives its own scroll publication and is invalidated by
external geometry/count/identity/presentation changes. Preserve arrows, wheel
translation, focus ensure-visible and cancellation/release consumption.

## 4. Minimal data and request design

All new types are model-free Controller values in existing UI modules. No model
or kernel imports in UI; no mutable behavior in renderer. Use these concrete
interfaces (derives like neighboring types):

```rust
// newell-ui/src/selection.rs, exported from lib.rs
pub enum SelectionOpenPurpose { Candidates, ManageSelected }
pub enum SelectionPresentation { Candidates, OrderedItems { profiles: bool } }
// Add `presentation: SelectionPresentation` to SelectionOpen.
// Replace OpenField(descriptor) with:
SelectionUiRequest::OpenField {
    descriptor: SelectionFieldDescriptor,
    purpose: SelectionOpenPurpose,
}
// Existing open/completion payload identities and outcome stay unchanged.
```

`SelectionFieldDescriptor.ordered` becomes true only for Loft K_PROFILE and Sweep
K_PROFILE/K_PATH. All other roles, max_count values and candidate filters remain
unchanged. Profiles=true only for the two Profile fields. Sweep Path uses title
Selection. This exposes selected-item management without falsely making Extrude
or Revolve single geometry profiles into a reorderable multi-profile command.
Do not raise Loft or Sweep cardinality here; record their existing limitation.

In `ShellState::activate_widget`, existing decoded slot0 (body) requests Candidates;
slot1 (populated ☰) requests ManageSelected only for descriptor.ordered, otherwise
Candidates. Empty fields have no manager target. `Row::select` already provides
two slots: preserve its dimensions, glyphs and activation. Keep historical mock
shell tests explicit; live `ShellState::with_model` always uses SelectionOpen,
never SelectionPanel::reference or fallback labels. Remove the universal profile
helper from runtime decision-making; retain legacy fixture behavior only where
its test explicitly asks for it.

For C2, normalize restored selection-field focus in
`ShellState::reconcile_selection_focus(&mut self)` (pub(super), shell/mod.rs).
Call it after modal focus restoration in sync and after accepted selection
counts are installed by set_selection_fields in shell/tree.rs. When a restored
slot1 no longer exists because the field count is zero, focus the same live
field's slot0 body. Do not use live_control's key-only visibility check as proof
that slot1 exists. Preserve the original control when it survives; clear stale
focus if the originating field/session has disappeared, never rebind it to a
new command lifetime. Leave modal-local focus alone while the modal is open.
The count-publication call is necessary: the modal closes before app acceptance
publishes the new field count, so the first close-time sync can still see the
old nonzero count. A successful later count publication must repair focus in
the same controller candidate before composition/publication; retries repair
presentation only and do not replay the accepted removal.

App `open_field_selection(descriptor,purpose) -> Update` revalidates the live
field descriptor, document and command session, then assigns presentation. It
also requires current accepted membership to be nonempty for ManageSelected,
before allocating a modal identity or changing active_selection. At shell
activation, an absent selection slot1 is inert and focus is normalized; do not
emit a request from the stale id. App rejection of a stale ManageSelected
request opens nothing and mutates no destination. This opening guard does not
forbid confirming an existing manager whose draft has subsequently become empty.
The application supplies **all eligible candidate rows** and the actual applied identity vector
in both presentations. Manager drawing selects rows from applied/draft order;
keeping the full candidate universe permits validation/reconciliation without
changing SelectionOpen's applied-subset invariant. Manager OK cannot introduce
new candidate membership; only permute/remove ids admitted at opening (minus
later invalidations). Check that constraint app-side as well as UI-side.

Dataset sources:

| Field/purpose | Real source |
|---|---|
| MergeScope | app candidate_rows with semantic role and insertion boundary; existing geometry-kind eligibility retained |
| Ordinary entity/axis/direction/path/body chooser | existing candidate_rows, filtered by current descriptor; document feature/output order |
| Typed Extrude Profile/Along/termination | existing geometry_field/geometry_candidates and GeometryPicker table, document/version/support-frame qualified |
| Loft Profile / Sweep Profile, Path chooser | existing descriptor-filtered candidates; no new topology source or artificial selected entries |
| Ordered manager | CommandSelectionInputs.values[field].entities() in stored order; resolve labels through candidates, retain equal labels with distinct ids |
| Window destination (existing API/tests) | existing WindowSelection and candidate_rows(None), unchanged membership chooser |

`panel_rows` and reconciliation return the eligible universe, not only the rows
currently rendered by a manager. Prune invalid identities according to existing
reconciliation, preserving survivors/order; do not silently restore removed
items. Distinct candidate generation/reference tables remain app-owned.
`geometry_values` and GeometryPicker are never reconstructed by parsing labels
or treating temporary picker ids as persistent entity ids. Existing typed
completion checks (version, session, field, reference table) remain intact.

### 4.1 Stable controls and reorder capture

Keep SelectionPanel's `rows`, `draft_order`, `runtime`, `completion`, scroll,
focus and return_focus. Add a separate bounded subslot for each row, handle and
remove control: `FIRST_ITEM + 3*stable_slot + part` (part=0 row,1 handle,2 remove).
Reserve `FIRST_ITEM + 3*MAX_CELLS` IDs; checked base/span arithmetic must exclude
Formula/Rename/window/progress ranges, including dynamically reconciled tombstone
slots. Update stale comments about 14 ids. Decode arithmetically into existing
`slot_rows`; never use visible-row indices as identities. Slots remain stable
through reorder and candidate reconciliation. Removed slot routes are inert;
new rows do not reuse tombstones during the session. Capacity exhaustion returns
existing UiError::InvalidSelection without partially replacing live state.

Add Slot::Handle(usize), Slot::Remove(usize) alongside Item. Candidate row
activation retains existing toggle and max_count=1 replacement. Ordered row
activation only focuses; Remove edits draft_order by identity, exactly once.
Build `visible_items()` from draft_order for managers, from rows for candidates;
use that same projection for len, scroll, focus ensure-visible and composition.
This projection may iterate data during composition; pointer hit testing cannot.
The focus ring includes each manager row then its remove control, enabled
arrows, OK, Cancel. Handles are pointer drag controls, not a new keyboard move
command; row focus has no invented visual. Keyboard activation of remove uses
existing Enter/Space. Return removed-control focus to OK.

Concrete internal/public boundary:

```rust
// overlays/selection.rs; export grab through overlays/mod.rs and lib.rs
pub struct SelectionItemGrab { /* private Copy fields: session/route epoch,
    entity, source index, logical content geometry */ }
impl SelectionPanel {
    pub(crate) fn item_grab(&self, id: WidgetId) -> Option<SelectionItemGrab>;
    pub(crate) fn item_grab_valid(&self, grab: &SelectionItemGrab) -> bool;
    pub(crate) fn finish_item_drag(&mut self, grab: &SelectionItemGrab,
        target: WidgetId) -> bool;
}
// Shell mirrors these with its current overlay geometry and semantic hit checks.
// ShellController's existing pointer entry points remain source-compatible.
```

`finish_item_drag` requires a target in the current visible ordered content,
same modal/session/route epoch, and unchanged draft-mutation epoch. Increment
that epoch on removal/reconciliation/reorder; retain the Copy grab representation
used by the existing CapturedDrag enum instead of storing a vector in it. It resolves
source/destination identities from stable slots and performs one remove+insert;
moving down inserts after target, moving up before target. No-op on source row,
inert/scrollbar/footer/outside target or expired grab. Resolve handle/row/remove
parts of a target to its identity without activating that target's remove.

Add `CapturedDrag::SelectionItem { grab, start_row, target }` in controller.rs.
Only primary handle press creates it, ahead of ordinary activation; movement
across a different visible content row arms reorder. Resolve target from the
published cell hit map, using direct cell reads; no row-rectangle search. Do not
mutate draft during motion; consume initiating release and apply once if armed
and valid. Reuse capture suppression of wheel and scene admission. Cancel on
pointer exit, focus loss, resize/scale, overlay dismissal, external list/route
change or Escape; cancelled release cannot activate a new control. A press on
× follows existing press activation once and consumes release, even when removal
compacts the list under the pointer. No auto-scroll, drag preview glyph, new OS
cursor style or animation. Thumb Grab/Grabbing behavior is retained separately.

Publication must preserve a valid item capture while only its target changes;
external ownership/draft/geometry changes invalidate it. Extend `same_routes`
with presentation and selected ordered mapping as well as candidate slots so
reordering cannot leave a certified control targeting its former identity.
Controller clone/compose/publish failure rolls back UI draft and input routing
as today; never emit completion from an unpublished close.

### 4.2 Request/completion and app transaction path

Retain this pipeline:

1. Semantic field activation -> ShellState selection request -> successful
   ShellController refresh/publication -> take_selection_request.
2. App::handle_selection_ui -> open_field_selection -> open SelectionOpen through
   ShellController::open_selection -> ShellState::open_selection -> open_with.
   Failed publication neither consumes a new active transaction nor substitutes
   mock state. Modal input blocks scene selection/navigation as today.
3. Candidate toggles or manager reorder/remove affect only SelectionPanel's
   draft. No CommandSelectionInputs/WindowSelection mutation yet.
4. OK -> confirm -> one SelectionCompletion(session,document,destination,
   Confirmed(ids)); Cancel/Escape -> Cancelled. Controller certifies close
   against its current selection_session/generation after successful publication.
5. App::handle_selection_ui checks the exact active tuple, consumes it once,
   validates uniqueness, eligibility, cardinality, live destination and, for
   manager, the admitted identity subset. Then replace only originating field
   or WindowSelection. GeometryPicker completion keeps version/reference checks.
6. Existing publish_selection installs counts/membership/rows, retains bounded
   delivery retry and never replays a mutation. Duplicate/stale close is inert.
   Selection OK creates no feature or persistent document notification.

Keep M4.4 input ordering in finish_shell_input, all retry/suspension machinery,
command lifetime replacement and command commit latch. Do not move selection
completion after a newly allocated command lifetime. Cancel manager changes
neither field counts nor window selection; restoring a command field's accepted
vector is not a global window-selection replacement.

## 5. Formula implementation and transaction preservation

Only Formula's local display needs the prefix. In overlays/formula.rs define
`PREFIX: &str = "Fx = "`, PREFIX_CELLS=5, FIRST_ROW_CELLS=26,
AREA_CELLS=57, keeping ROW_CELLS=31, SIZE=(35,8). Reuse
`caret_window(self.edit.chars(), "", caret,57)`; map its first 26 cells to row2
x7 onward, the remaining 31 to row3 x2 onward. Independently write prefix at
row2 x2. Fill the whole inner text allocation before writing; no old suffix
cells or metadata survive shorter text. A private row-mapping helper in this
file suffices; do not redesign shared TextEdit/caret_window or numeric editing.

README §22.1 is normative: c=25 puts caret at row2 x32; c=26 puts it at row3
x2; c=56 puts it at row3 x32; c=57 starts s=1 with caret still row3 x32.
With focus on OK/Cancel, draw prefix plus E with no caret and s=0, just as
existing inactive caret_window does. No pointer caret placement is added.
Home/End mean whole expression boundaries; selection anchors and source origins
are expression-relative. The five prefix cells have Text ink and the existing
TEXT target; inner frame keeps Contour. Footer-only hover remains unchanged.

`open(&mut self, inter: &mut InteractionState, text: &str)` keeps its signature:
replace the current `.chars().take(MAX_LEN)` initialization with the exact
supplied String, then initialize TextEdit and byte-origin map from that source.
Keep 256 as typed growth cap. Edit acceptance must allow resulting length
<=256 OR <=the length before this edit, allowing deletion/replacement without
growth of longer supplied sources. Navigation never fails on length. This
allows verbatim opening without redefining parser bounds. Parser rejection
still follows its existing contract; no evaluator changes are authorized.

No production string concatenation of PREFIX and source, prefix stripping,
whitespace normalization or newline insertion. `draft`, `text`, `origins`,
FormulaApplyRequest.source/source_origins, diagnostics and accepted numeric
projection continue to hold only the actual expression. If a supplied expression
itself begins `Fx = `, keep it verbatim as user source and let current validation
decide; don't remove it because it resembles the presentation prefix.

Preserve, and regression-test through current seams:

- click numeric ☰ -> NumericFormulaOpenRequest(destination,sequence,
  through_sequence,origin); prior blur batch resolves first;
- App::complete_numeric_formula supplies resulting accepted NumericProjection,
  then ShellController::open_numeric_formula installs view and opens once;
  changed magnitude clears old expression, rejected/incomplete magnitude retains
  accepted expression, direction-only/no-op retains it;
- failed opening delivery retries delivery only; duplicate opening cannot reset
  a draft that the user already edited;
- Formula Enter/OK -> application_pending -> ShellState::finish_formula creates
  FormulaApplyRequest with exact destination/modal session/distinct attempt and
  source-origin provenance -> app validates/applies -> finish_formula_apply;
- rejection leaves exact draft, caret/selection and diagnostic editable; success
  acknowledges application before modal close. Empty source returns to retained
  literal. Escape/Cancel discard only modal edits and restore field focus;
- command OK still drains required input/apply requests and commits before
  command closure; accepted commit with delayed delivery cannot commit again.

Do not modify numeric.rs, app parameters/evaluator, or command commit semantics
just to display a prefix. App/controller test additions are allowed to prove
that no prefixed payload crosses existing boundaries.

## 6. Exact Executor allowlist and ordered work

Only the following paths may change; existing unrelated hunks must survive.
No new production file/dependency is needed. New test functions belong in the
listed existing files. Changes to other paths require a concrete scope issue
reported to the human, not an automatic wider refactor.

| Files | Responsibility |
|---|---|
| `crates/newell-ui/src/selection.rs` | purpose/presentation values, OpenField request, SelectionOpen validation/documentation |
| `crates/newell-ui/src/lib.rs`, `crates/newell-ui/src/overlays/mod.rs` | narrow type/grab exports |
| `crates/newell-ui/src/overlays/selection.rs` | unified logical art, purpose-aware projection, stable subslots, draft remove/reorder, focus/scroll, embedded tests |
| `crates/newell-ui/src/overlays/formula.rs` | prefix layout, exact opening/source-preserving edit cap, embedded tests |
| `crates/newell-ui/src/shell/mod.rs` | slot-aware open intent, dynamic manager frame, forwarding item capture, request fixtures |
| `crates/newell-ui/src/shell/tree.rs` | C2 only: invoke selection-focus normalization after accepted field counts are installed; no tree behavior changes |
| `crates/newell-ui/src/shell/compose.rs` | propagate Selection footer hover result consistently with Formula; preserve overlay precedence |
| `crates/newell-ui/src/create_box.rs` | replace misleading universal Profile presentation helper with semantic/purpose dispatch if needed; no field geometry changes |
| `crates/newell-ui/src/loft.rs`, `crates/newell-ui/src/sweep.rs` | ordered descriptor flags only, matching embedded expectations |
| `crates/newell-app/src/controller.rs` | certified purpose requests, item capture, generation/publication lifecycle and embedded tests |
| `crates/newell-app/src/app.rs` | purpose-aware open, nonempty accepted-membership guard for ManageSelected, presentation selection, manager-subset validation and embedded app/Formula regression tests; preserve existing geometry candidate hunks |
| `crates/newell-ui/src/shell/tests.rs` | actual chooser/manager paths, selection fixtures and formula/hover integration |
| `crates/newell-ui/src/overlays/tests.rs` | affected reference expectations only if present for these components |
| `crates/newell-app/tests/it/controller.rs` | pointer capture, shared borders, publication rollback/retry and Formula payload checks |
| `crates/newell-ui/tests/it/action_boxes.rs` | exact descriptor semantics/cardinality and field-body/☰ ownership checks |
| `crates/newell-ui/tests/it/snapshots.rs` | narrowly scoped snapshot setup and added local cell assertions |
| `crates/newell-ui/tests/it/snapshots/it__snapshots__selection_panel_open.snap` | only changed modal footprint/content and revealed underlying cells |
| `crates/newell-ui/tests/it/snapshots/it__snapshots__formula_editor_open.snap` | only prefix and expression/caret viewport cells |

`overlays/list.rs`, `overlays/place.rs`, `widgets/edit.rs`, `action_box.rs`,
app `selection.rs`, `parameters.rs`, `navigation.rs` and M4.5 plans are integration
references, not edit targets. Existing scroll geometry/math and field two-slot
composition suffice. If a literal request fixture elsewhere now needs the added
presentation field, report that exact compile-only path before widening scope.
No wildcard snapshot allowlist and no new bulk snapshot files. Additional
states use explicit cell assertions/inline expected literals in listed tests.

Small implementation increments, each complete before the next:

1. Record required verification baseline without snapshot updates. Re-read this
   revision/current spec/source diff. Confirm no incompatible concurrent edits.
2. Add purpose/presentation types and update listed constructors/matches.
   Set only the three ordered flags. Keep app candidates, destination/session,
   cardinality and typed-reference contracts. Test data projection/transactions.
3. Unify Selection logical geometry/chrome for candidate and manager, add dynamic
   height/placement and stable subslots. Use divider15 and the corrected hover
   boundaries; verify empty/short/long cells and roles.
4. Add manager remove and then controller handle capture/reorder. Reconcile scroll,
   focus and route certificates, including the last-item close/count-publication
   repair and stale management-open guard. Verify failed publication and cancellation.
5. Add fixed Formula prefix/57-slot mapping and source-preserving opening/edit
   predicate. Verify source-origin and transaction regression tests before any
   snapshot adjustment.
6. Review only the two allowed snapshot diffs against §§19–22; deliberately edit
   their expected cells after inspecting actual output. Run focused then required
   workspace checks. Report manual checks separately; do not claim them from tests.

No stage starts another agent/role; this is a work sequence for one Executor.

## 7. Required evidence and acceptance tests (future Executor)

Use independent expected literals from the canonical reference, not the production
geometry/formatting helper to compute test expectations. Verify glyph, ink,
WidgetId/shared-border and state invariants in addition to text snapshots.

| Cases | Required assertions |
|---|---|
| Empty chooser, no candidates | 31×7, one 27-space inner row, no mock/placeholder/arrow/item target; OK/Cancel valid |
| Zero selected, nonempty candidate universe | Eligible real rows still visible, all unchecked; adding updates only modal draft |
| Empty manager, nonempty eligible universe | Exactly one blank row, no checkbox candidates; OK empty vector or Cancel unchanged applied vector |
| N=1,3,5,6,9,10,>10 | Heights7,9,11,11,11,11,11; no bar at <=5; five-row viewport otherwise. Retain full §21 offset/glyph table tests |
| New profile reference | N=9 offset1 gives track `╻╹ `; every row31 cells, handle3, remove26, gutter28. Exact empty/membership art matches byte-for-byte spaces |
| Purpose types | Merge fields always chooser; Loft Profile, Sweep Profile/Path body chooser vs populated ☰ manager; Extrude/Revolve profile stays single-candidate selection; Boolean tools remains unordered membership |
| Cardinality/filter | Existing max_count=1 replacement still works; unrestricted manager fixtures preserve supplied order; role filters/insertion boundaries unchanged; unsupported/absent data produces no fake rows |
| Stable identity | Two identical names with distinct ids: reorder second past another row, remove exactly first, retain correct survivors. Labels never identify rows. Change labels without changing ids; stale removed slot does nothing |
| Transaction | Before OK field/window/count unchanged; OK replaces exact owning destination once; duplicate close does nothing; Cancel/Escape after combined reorder/remove restore accepted vector and focus; no feature/revision/notification |
| Last-item focus (C2) | Open a singleton manager through populated ☰, remove the item and OK through app/controller; accepted zero count removes ☰ and focuses the same visible body. Enter/Space opens its candidate chooser. Cancel instead retains membership and restores the surviving original ☰ |
| Stale manager open (C2) | Submit ManageSelected after accepted membership becomes empty: shell absent-slot activation emits nothing; app independently rejects a stale request before modal allocation/open or destination mutation. An already open manager may still confirm its empty draft |
| Focus publication/lifetime (C2) | Failed zero-count publication retains prior coherent UI; successful retry publishes empty field and body focus together, with no replayed removal. Field/session disappearance clears stale focus rather than moving it into a replacement command |
| Dataset reconciliation | Disappearing candidate pruned by existing policy; survivors retain slots/order; manager removal does not remove candidate from universe; change from6 to5 clears bar/capture, offsets clamp; growth retains valid offset |
| Capture | Handle press no mutation, row movement+valid release one reorder, same-row no-op; outside/footer/gutter drop cancels; wrong button does not complete; pointer exit/focus loss/Escape/scale/resize/session replacement cancels; × release cannot remove newly shifted row |
| Publication | Failed compose/publish leaves prior certified grid/draft/destination; repeated valid capture moves retained; changed routes/identity/count invalidate; failed close emits no completion; bounded retry does not apply twice |
| Clipping | Width/height0 and1, narrow rect, partially off-grid, ordinary82×30, larger grid; no underflow/OOB, logical gutter/control positions fixed, clipped cells not targets, no old shared borders/roles after shrink |
| Shared footer/hover (C1) | Exact footer literal; column14 belongs to OK, column15 is vertical SharedBorder(low OK/high Cancel), column16 belongs to Cancel. Test both halves at column15; OK hover substitutes boundaries0/15, Cancel15/30, preserving labels/roles/ownership; modal background and scene cannot activate |
| Scroll | Existing arrow/wheel/keyboard exact offsets, half-slot endpoint and quantization cases, invalid offsets saturated, thumb own-scroll capture persists, external layout/count cancellation and release consumption preserved |

Formula-specific tests:

1. Empty open has exact prefix at (2..6,2), caret (7,2); focus on buttons hides
   only caret. SelectAll then Backspace empties E while prefix remains. Home then
   repeated Backspace, End then Delete, selection deletion/replacement never
   modifies prefix. Prefix click focuses text without pointer caret positioning.
2. Independent boundary literals for caret indices0,25,26,56,57,58,256; verify
   asymmetric wrap, s, last-cell caret, clipping, unchanged35×8 frame and footer.
   A long inactive expression shows first57 source chars; active shows56+caret.
   First row never overwrites prefix; second row starts atx2. Small allocation
   crops original coordinates instead of relocating controls.
3. Exact whitespace/Unicode scalar source preservation through open, no-op Enter,
   Cancel/reopen, prefix-looking user source and source lengths255/256/257.
   Supplied257 source is not truncated; further growth rejected, shortening
   allowed; normal input stops at256. Display substitution for unsupported glyphs
   never changes source. Source-origin byte offsets remain expression-only.
4. Assert FormulaEditor.draft/text and emitted FormulaApplyRequest.source equal
   E, including empty string; origin vector length matches E's chars, not prefix
   length. Feed captured E through current app validation path and inspect exact
   source, so a display-only assertion cannot hide prefixed evaluator input.
   No new clipboard shortcut is required: any existing copied/source payload
   must originate in TextEdit/E, never composed cells.
5. Existing same-event changed literal -> empty accepted expression; no-op or
   direction-only -> retained expression; rejected/incomplete literal -> last
   accepted expression. All show prefix regardless of source. Delayed opening
   retry opens once, does not replay numeric edits or reset typed draft.
6. Invalid Formula OK keeps exact draft/caret/selection and diagnostic open;
   explicit corrected attempt works once. Empty OK clears formula only; Cancel
   leaves accepted literal/formula intact. Duplicate/stale apply/session or
   namespace replacement cannot update a different destination. Failed apply
   delivery retries stored outcome, not parsing/mutation. Successful app apply
   precedes modal close; command commit precedes command close.

Run focused UI overlay/shell, app selection/Formula/controller tests first,
then the workspace gates required by docs/agents/README:

```sh
cargo check --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
INSTA_UPDATE=no cargo nextest run --workspace --all-features --locked
cargo test --workspace --doc --locked
```

If nextest is unavailable, use the documented
`INSTA_UPDATE=no cargo test --workspace --all-features --locked --no-fail-fast`.
Run baseline before edits; distinguish existing failures from regressions. Do
not format unrelated dirty files, weaken tests/lints or auto-update snapshots.
Read each permitted snapshot diff cell-by-cell; no bulk regeneration. Geometry,
GPU/MSRV/dependency/license checks need no new scope here; follow repository
requirements if a real affected-path issue arises. Do not claim current suite
passes: Architect ran none of these commands.

## 8. Separate manual acceptance checklist (not yet performed)

Record device, display and actual effective_k as well as the UI Scale label.
Repeat representative empty, short, long and Formula states in Light/Dark,
Color off/on, UI Scale80/100/120/140, on Retina and Full HD when available.
Labels sharing the same k are expected. Exercise all effective k2…7 where
available; report unreachable sizes rather than claiming physical coverage.

- Open empty runtime selection: compact one-row blank31×7 with rounded title,
  inner list and shared footer. No invented profiles; candidate rows appear
  where eligible data exists despite zero selection. Check centered placement.
- Open actual merge scope: checkbox states reflect draft membership; Cancel
  leaves targets/count unchanged; OK affects the originating field only.
- Open Loft/Sweep selected manager via ☰ and chooser via body; verify title,
  candidate/selected distinction, singleton limitation and no indiscriminate
  profile checkboxes. Multi-item reordering may require a controlled UI test
  fixture because current runtime descriptors retain max1: label that evidence
  explicitly, never present mock data as real runtime geometry.
- Remove the singleton manager's last item and OK: the accepted empty field
  restores focus to its visible body and Enter opens candidate selection. Repeat
  with Cancel: membership and the original populated ☰ focus survive.
- Inspect duplicate labels, remove/reorder and five-row scrolling; thumb remains
  grabbed during motion, then cancels correctly on resize/scale/focus exit.
  Check rows/controls at clipped edges, count shrink, and revealed background.
- Empty Formula visibly retains `Fx = ` including space. Type through both wrap
  boundaries, Home/End/select/delete, focus buttons, scroll long source, Cancel
  and reopen. Caret never occupies prefix or frame and does not blink.
- Click ☰ during valid/invalid numeric edits; verify resulting accepted source,
  rejected Formula recovery, empty OK, Cancel, focus return, and normal command
  commit-before-close. Check retained precision/source in actual workflows.
- Both footer divider halves at column15 activate the intended button; column14
  remains OK padding. Hover only substitutes boundaries0/15 or15/30 with
  unchanged ink. Check Text/Contour/Action across four palettes;
  no focus tint, generic inversion or changed drag/scene behavior under modals.

## 9. Proposed policy and concrete limitations

One product policy remains for approval with this plan: **P1 placement**, as
specified in §3/README §19.1. The artwork alone does not decide global position.
The body-versus-☰ distinction is the concrete proposed implementation of the
existing contextual field semantics, using already separate targets so candidate
admission and ordered management both remain reachable. Approval of revision2
includes that explicit behavior; do not silently choose a different interaction.

No cardinality expansion is proposed. Current Loft Profile and Sweep Profile/Path
are max1 in source despite Loft's canonical ordered-set semantics. This plan
makes their management presentation correct while preserving that limit; it
does not claim a runtime multi-profile Loft/Sweep pipeline. Relaxing it or adding
typed topology for those commands requires a separate functional request. Tests
with explicit unrestricted SelectionOpen values prove the generic ordered editor
without implying those unsupported CAD operations exist.

Reference-only names are not source data, the screenshot is unavailable, and
runtime/visual/physical-device acceptance remains pending. Architecture and ADRs
were not amended. This documentation stage ends here. Recommended next stage:
human approval of this revision (including P1), then Executor if requested;
Challenger review is optional and only human-initiated.

Completed Architect verification: exact empty Selection, membership, ordered,
empty Formula and populated Formula blocks in README match Interface_Elements
character-for-character; row widths are31/35 as applicable. Checked actual
ordered remove/gutter columns26/28, five-character prefix and26+31=57 capacity,
balanced document code fences and existence of cited explicit crate paths.
`git diff --check -- docs/ui_design/README_UI.md` reported no whitespace errors;
Git index remains empty/unchanged. These are documentation checks only.

Revision-2 verification is limited to documentation consistency and whitespace:
the footer literal and both button spans agree with divider15; hover boundaries
and C1 acceptance assertions match. C2 specifies close-time and accepted-count
focus repair, nonempty app admission, Cancel preservation and publication/lifetime
regressions within the explicit allowlist. Reference artwork and Formula rules
are unchanged. No builds, tests, application runs, code edits or agent dispatch
were performed. P1 and the prior runtime/manual limitations remain pending.
