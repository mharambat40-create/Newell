# Selection and Formula reference alignment

Revision 3 — proposed implementation plan, 2026-10-28. Architect / Codex.
Task: selection-formula-reference-alignment, production Selection layout correction.
Intended Executor: Luna with medium reasoning, after explicit human approval of
revision 3 and an implementation request. No other role or agent is started.

This remains the single active plan; revision 3 supersedes revisions 1–2. The
current request authorizes the Selection diagnosis/plan and narrow canonical
documentation amendments only. Formula §5 and Formula-specific tests below
are retained verbatim from revision 2 as pending work, not newly implemented or
expanded. A Selection-only execution of this revision excludes Formula source,
Formula snapshot changes and Formula implementation step 5; a request to execute
the entire combined plan includes the unchanged Formula work. Selection work
must preserve current Formula behavior in either case.

## 1. Contract, current state and authority

Checkout: `/Users/martin/Desktop/Newell`, branch `main`, no staged entries.
At entry these existing unstaged inputs were present: app `geometry_worker.rs`,
kernel `extrude.rs` and `surface.rs`, model `geometry.rs`. During read-only
inspection kernel `boolean.rs` and `termination.rs`, then `Description.md` and
`docs/backlog.md`, also became modified.
These geometry/context changes are external work and are preserved; no affected UI or
plan write conflict was found. No source snapshot, branch, worktree, index
mutation, commit, build, test, application launch or agent dispatch is permitted
in this Architect stage. Executor must reread current diffs before writing.

Primary sources inspected: AGENTS.md, shared README, Architect role,
ARCHITECTURE §§2–3.6/4; accepted ADRs 0002, 0003 and 0006; README_UI §§5.5–5.12,
12.1,14.1–14.2,18–22,25/25.1,28.3; Interface_Elements Selection/Profile and scroll
references; current UI/app implementation and tests identified in §2. The
attachment is text only: the reported screenshot is not independently visible.
No screenshot/executable provenance or present test-suite pass is claimed.

The human explicitly authorizes narrow canonical UI/architecture amendments,
overriding Architect's ordinary plan-only documentation restriction for this
task. This stage changes only this plan, README_UI and Interface_Elements.
Architecture already delegates UI contracts to README_UI (§3.0); its modal,
hover and semantic-palette rules do not require legacy Selection chrome. No
architecture or ADR amendment is needed. README_UI now explicitly binds every
Selection production path to §§19–21, clarifies purpose mapping and placement,
and cross-references contextual selection `☰` from §5.6. The second duplicate
Profile artwork in Interface_Elements now points to its first occurrence and
the canonical rules. No Formula prose/artwork is changed.

Excluded: geometry repair, model transactions, candidate eligibility/cardinality
changes, new CAD capabilities, unrelated controls/navigation, renderer/shaders,
fonts, dependencies, licensing and public integration. Existing Rust 1.88/MSRV,
AGPL-3.0-or-later, MVC and one batched ASCII UI draw remain unchanged.

## 2. Confirmed diagnosis and affected paths

The prior plan's Selection correction is **not implemented in this checkout**.
Shared selection/session/scroll infrastructure is present from earlier work,
but the required dynamic chrome, presentation payload, ordered subcontrols and
command-path composed-cell tests are absent. The plan's existence is not
implementation evidence or approval. Formula's requested revision2 changes also
remain absent: overlays/formula.rs still has AREA_CELLS=2*ROW_CELLS (62), calls
caret_window without a prefix, and open truncates at MAX_LEN. This static status
observation does not authorize Formula changes in the Selection-only task.
No evidence proves that anyone intentionally
bypassed an approved plan; history/authorization is not inferred from source.

| File and symbol | Confirmed production behavior / implication |
|---|---|
| `newell-ui/src/shell/mod.rs::activate_widget` | Model-backed activation emits `OpenField(descriptor)` for both field slots. Its `open_profiles` branch is in the mock fallback, not the live model path. |
| `newell-app/src/app.rs::finish_shell_input`, `handle_selection_ui`, `open_field_selection` | Real field intent creates application-supplied candidates/applied membership and opens `SelectionOpen` through the controller. There is no purpose/presentation field. |
| `newell-app/src/controller.rs::open_selection`, `newell-ui/src/shell/tree.rs::open_selection` | Controller publishes/certifies the modal; shell validates document and qualified geometry rows, then calls `SelectionPanel::open_with`. Preserve these boundaries. |
| `newell-ui/src/overlays/selection.rs::open_with` | Unconditionally `open_kind(..., SelectionKind::Generic)` for every runtime request. Thus typed Extrude, other command fields and Window API all reach the legacy Generic writer. |
| Same file `SIZE`, `Geometry::of`, `Geometry::from_interior`, `draw_content` | Fixed 30×12; `modal_layout` reserves a title body row, then a generic checkbox list and footer. No inner list frame or separator is drawn. Even zero/one/three candidates retain excess empty rows. |
| `newell-ui/src/overlays/mod.rs::draw_frame` | `OverlayKind::Modal` supplies a double border. Generic content does not overwrite it. Color mode only maps same-cell contour strokes; it cannot add the missing frame/title/separator or fix height. |
| `overlays/selection.rs::PROFILE_SIZE`, `draw_in_action_box`, `draw_action_content`, `draw_selected_items`, `draw_profile_rows` | A second 31×11 fixture path has rounded title/list/separator. It stays fixed-height, derives logical art after clipping, and gives ☰/× the same item ID and toggle semantics. Copying its art alone cannot provide runtime ordering/removal. |
| `shell/mod.rs::profile_selection_frame`, `sync`, `activate_modal`, `route_modal`; `shell/compose.rs::compose`, `draw_overlays` | Placement, key/activation, scroll/thumb geometry and suppression of underlying Action Box depend on the legacy profile-kind split. They must consume the same new layout/projection, not receive an isolated drawing fix. |
| `create_box.rs::is_profile_selection_field` | Classifies every shared K_PROFILE as ordered in the fixture branch, including single-reference Extrude/Revolve. Remove this inference. All five live descriptor implementations currently say ordered=false. |
| `overlays/selection.rs::len`, `labels`, `activate_in`, `ring`, `reconcile_rows` | Runtime dataset is all candidates; draft_order is separately retained. Current item activation toggles membership; no distinct remove/handle routes exist. Candidate count must not size an ordered selected-list manager. |

Repository-root paths above are under `crates/`. This chain explains all five
reported chrome defects without a geometry failure: construction selects legacy
Generic, geometry reserves the old size/title, overlay adds the double frame,
content lacks list/separator, and interaction uses the matching old geometry.
The source predicts the screenshot's appearance; whether a stale installed app
was also launched is unverified. Do not diagnose application provenance from art.

### 2.1 Tests currently protecting the wrong path

- `newell-ui/tests/it/snapshots.rs::selection_panel_open` uses `ShellState::new`
  fixture data. Its committed `it__snapshots__selection_panel_open.snap` explicitly
  stores the large double frame/body title/blank area. It must change against
  canonical §19, not be treated as authority.
- `shell/tests.rs::profile_field_opens_the_exact_profile_selection_box` and
  `every_profile_field_opens_the_profile_selection_box` inspect exact cells but
  use the fixture shell and incorrectly demand Profiles for every Profile field.
- `overlays/selection.rs` has useful transaction/reconciliation/scroll tests;
  `profile_writer_matches_every_state...` directly exercises the alternate
  writer. Neither proves that a command opens that writer in production.
- `app.rs::shared_selection_tests::field_transaction_latches_and_updates_only_originating_input`
  starts a command but calls `app.open_field_selection(d)` directly, asserting
  rows/counts/transactions rather than exact composed chrome. Its neighboring
  Window test fabricates a `SelectionOpen`. Retain their behavioral coverage;
  add the full activation-to-composition tests in §7.1.

No higher-precedence Selection prose currently mandates obsolete art: README_UI
§§19–21 already require the requested shared frame. The contradiction found is
**semantic**: revision 2/§19.2 assigned ordered managers to Sweep Profile/Path
although §§14.1–14.2 define one section/trajectory and source limits both to one.
Revision 3 corrects that overreach; only Loft Profile has the ordered-set contract
(§12.1). Loft's existing max1 implementation limitation remains explicit.

The diagnostic `invalid or stale geometry reference` is the display string of
`newell-model/src/lib.rs::ModelError::InvalidGeometryReference`. It is a separate model/typed-reference
validation matter. No causal link from it to the chrome was found. Preserve
GeometryPicker version/session/field/reference validation; do not fix or suppress
that error in this task. Geometry files already dirty are outside this scope.

## 3. Reference geometry and placement

README_UI §§19.1/20 are the only normative cell-coordinate contract; §21
owns scrollbar math. Use those tables directly, including width31, heightV+6,
V=max(1,min(N,5)), divider15 and rounded chrome in all four Theme/Color states.
Expected footprints for counts0,1,3,5,6+ are respectively31×7,31×7,31×9,31×11,
31×11. Titles/content/control positions and ink roles come from those canonical
sections, not copies of modal_layout or a snapshot. Formula geometry remains
unchanged by Selection execution; its pending work is in §5.

Create one private `SelectionLayout` value in overlays/selection.rs, replacing
Selection-specific use of Geometry::of/from_interior/from_action_box and both
size constants. Inputs: logical origin, clip allocation, active projection count
and presentation. Value contains logical outer/title/inner/content/separator/
footer/OK/SharedBorder/Cancel/gutter rects plus clipped allocation and existing
SelectionListGeometry. Compute with full logical dimensions before intersecting.
All draw, sync_overlay, keyboard/ensure-visible, wheel/arrows, thumb/item-grab
validation and drag-target methods consume this value (or accessors on it).
An empty list reserves one **visual** row but has zero items/focus/item targets;
do not change shared SelectionListGeometry's zero-item window to a fake item.

Keep existing center_modal origin/allocation for candidate/window choosers.
Ordered management uses existing Layout::action_panel at its top/right anchor
with dynamic height. Keep logical width31 even if that allocation is cropped.
Use the returned origin and separately retain the full logical layout; never
infer footer/right edge from cropped width/height. No global place.rs change.

One shared chrome writer clears the clipped allocation to surface-owned blank
cells, writes rounded outer frame, embedded title, inner frame, separator and
footer at the logical positions, then variant rows. `draw` and retained public
`draw_in_action_box` are thin placement adapters calling that writer. Remove
legacy Selection `draw_content`/`draw_profile_rows` chrome alternatives; a fixture
may supply explicit data but must use the same frame and variant row writer.
No second independent selection-frame implementation may survive.

Continue compose_overlays modal blocking. Its callback must overwrite the
**whole Selection frame**, including the default double border, and report footer
hover consistently with Formula. Do not alter OverlayKind::Modal globally or
change unrelated dialogs. Canvas writes must be intersected with allocation,
bounds and grid; inert inner frames/empty rows remain surface-owned. Full shell
recomposition clears obsolete glyphs/roles/IDs/shared borders after shrink/move.
Suppression of the underlying Action Box follows ordered-manager placement;
candidate choosers retain the command box beneath the modal as today.

Keep draw_ok_cancel's existing button styling and SharedBorder semantics, but
write the Selection footer through the shared frame writer's clipped Canvas,
using existing owned_blank/shared cell constructors and the exact §19.1 literal.
Its labels/ownership use the full logical layout; do not pass a cropped rect to
draw_ok_cancel, which would recenter captions and recompute the divider. No
change to the global footer primitive is needed. Rendering, hover and hit
testing share these spans:
column14 remains OK padding, divider15 routes vertical modulo low=OK/high=Cancel,
outer boundaries remain surface-owned. Roles are semantic: title/name/caption
Text; frames/separator/divider Contour; checkbox, handle, remove, arrows/thumb
Action. No focus tint, new hover visual, palette/shader/atlas change or UI draw.

Reuse §21.2 arithmetic unchanged through existing SelectionThumb and
round_half_up: six half slots, current glyphs, saturation and widened arithmetic.
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

`SelectionFieldDescriptor.ordered` becomes true only for Loft K_PROFILE.
All other descriptor flags, roles, max_count values and filters remain unchanged.
Current manager requests use OrderedItems { profiles: true }; retain the explicit
false case for an unrestricted non-profile ordered UI request, not an invented
command route. README_UI §19.2's table is the exhaustive production-purpose map.
Sweep Profile/Path remain single-reference candidates. Do not raise any limit.
Only an ordered descriptor may request ManageSelected, enforced shell/app-side.
The accepted singleton Loft manager may remove; its handle starts no effective
move. A single-reference chooser never displays handle/×, whatever its label.

In `ShellState::activate_widget`, existing decoded slot0 (body) requests Candidates;
slot1 (populated ☰) requests ManageSelected only for descriptor.ordered, otherwise
Candidates. Empty fields have no manager target. `Row::select` already provides
two slots: preserve its dimensions, glyphs and activation. Keep historical mock
shell tests explicit; live `ShellState::with_model` always uses SelectionOpen,
never SelectionPanel::reference or fallback labels. Remove the universal profile
helper from runtime and fixture decision-making. Reference fixtures must choose
presentation explicitly; remove/update tests that infer Profiles from every
K_PROFILE. Fixtures share the runtime writer and may not preserve an obsolete
private rendering branch.

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
Formula-specific rows apply only to an explicit combined-plan execution (revision scope above);
they are excluded from the Selection-only correction.
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
| `crates/newell-ui/src/create_box.rs` | remove universal Profile presentation inference from runtime/fixture dispatch; no field geometry changes |
| `crates/newell-ui/src/loft.rs` | ordered=true only for K_PROFILE, matching embedded expectations |
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
composition suffice. Current SelectionOpen literals are in app.rs and overlays/selection.rs, both
allowed. Audit all OpenField matches and request constructors in these files
when changing the payload; do not leave a default Generic runtime branch.
No wildcard snapshot allowlist and no new bulk snapshot files. Additional
states use explicit cell assertions/inline expected literals in listed tests.

Small implementation increments, each complete before the next:

1. Record required verification baseline without snapshot updates. Re-read this
   revision/current spec/source diff. Confirm no incompatible concurrent edits.
2. Add purpose/presentation types and update listed constructors/matches.
   Set only Loft K_PROFILE ordered=true; Sweep/Extrude/Revolve stay false.
   Keep app candidates, destination/session, cardinality and typed-reference contracts. Test data projection/transactions.
3. Unify Selection logical geometry/chrome for candidate and manager, add dynamic
   height/placement and stable subslots. Use divider15 and the corrected hover
   boundaries; verify empty/short/long cells and roles.
4. Add manager remove and then controller handle capture/reorder. Reconcile scroll,
   focus and route certificates, including the last-item close/count-publication
   repair and stale management-open guard. Verify failed publication and cancellation.
5. Add fixed Formula prefix/57-slot mapping and source-preserving opening/edit
   predicate. Verify source-origin and transaction regression tests before any
   snapshot adjustment.
6. Review the Selection snapshot against §§19–21 (plus the Formula snapshot
   against §22 only for combined execution); deliberately edit
   their expected cells after inspecting actual output. Run focused then required
   workspace checks. Report manual checks separately; do not claim them from tests.

No stage starts another agent/role; this is a work sequence for one Executor.

## 7. Required evidence and acceptance tests (future Executor)

### 7.1 Required production-command composed-cell regression

Extend app.rs::shared_selection_tests with its existing document fixture and
input helpers (click/key call the controller and finish_shell_input). Create
**real document outputs** with distinct identities/names; do not populate mock
MOCK_ITEMS/PROFILE_ITEMS or bypass the app with a selection-frame-only test.
For an Extrude merge test, drive Create → [E]xtrude → Add → disable Merge with
all → hit-test/click the actual Targets field. Finish normal request delivery;
assert the published controller grid equals last_ui_publication and inspect the
entire popup rectangle from that grid, including glyph/role/routing metadata.
Do not call open_field_selection directly in this new test.

Run this path with0,1,3,5,6,9 eligible bodies from document data; zero selected
with candidates must still show every eligible candidate. Compare independent
literal rows and exact height/title/list/separator/footer, not production layout
helper output. Scroll N=9 to offset1 to check ╻╹ , then to both endpoints.
Hit-test arrow, thumb, blank track, content, footer column14, both pixel halves
of divider15, and Cancel; assert matching draft/completion/session effects.
Repeat representative empty/three/long command states across Light/Dark × Color
off/on, checking canonical semantic roles and that Color cannot mask bad chrome.

Add command-route cases for typed Extrude Profile (real kernel-backed source via
existing Extrude fixture helpers, retaining captured support references),
Revolve Profile, Loft Profile body and populated ☰, Sweep Profile and Path,
Boolean Target/Tools/Faces, plus optional-field activation for each descriptor
family. Table-drive all current field_descriptors per operation by making each
conditional field visible, clicking each live slot, then cancelling. Compare
presentation/count/cardinality against README_UI §19.2. No field may silently
fall back to the old writer. Unsupported Boolean Faces is an actual empty case.
Use valid document data for eligible entities; don't weaken candidate filters.

Current max1 Loft means real-command manager counts are1 and0 after removal;
cover both and focus repair through normal app publication. For manager counts
3/5/6/9, supplement with unrestricted explicit SelectionOpen containing real
document identities through ShellController::open_selection and its composed
published grid. Cover profile and non-profile ordered content/title there,
reorder/removal and Cancel/OK. This is runtime UI-contract coverage, clearly
labelled **not** evidence of a multi-profile command. Do not increase a command's
cardinality to manufacture command coverage. Window destination has no observed
menu opener; test its existing controller API with real data and report that
limit instead of inventing a command.

Regression sensitivity: assert the command-generated request has explicit
presentation; the legacy forced Generic path, either old size constant, a
missing inner frame/separator, wrong dataset, or private fixture writer must
fail these tests even if direct writer unit tests still pass. Existing mock
snapshots remain supplemental. The actual-command matrix is required acceptance,
not replaceable by updating the old snapshot or opening a fabricated request.

### 7.2 Additional state/geometry coverage

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
| Purpose types | Merge fields always chooser; Loft Profile body chooser vs populated ☰ manager; Extrude/Revolve/Sweep Profile and Sweep Path stay single-candidate selection; Boolean tools remains unordered membership |
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

Executor/manual tester must build and launch the current executable from this
checkout, not an installed app chosen by name. From `/Users/martin/Desktop/Newell`
use `cargo build -p newell-app --bin newell --locked`, then launch the absolute
resulting executable (normally `/Users/martin/Desktop/Newell/target/debug/newell`;
if CARGO_TARGET_DIR is set, resolve and record the actual output first). Record
source checkout, build result/time and launched absolute path. Never use
`open -a Newell`, Spotlight or the obsolete OneDrive application. This Architect
has not built or launched either executable.

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
- Open Loft Profile selected manager via ☰ and chooser via body; verify title,
  candidate/selected distinction, singleton limitation and no indiscriminate
  profile checkboxes. Multi-item reordering may require a controlled UI test
  fixture because the current Loft descriptor retains max1: label that evidence
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

## 9. Handoff, approval and remaining limits

Revision 3 is implementation-ready for Selection. Its canonical purpose mapping
supersedes revision2's Sweep ordered-manager proposal; placement retains centered
choosers versus anchored ordered management. Formula work is unchanged/pending
as described at the top. No unresolved architecture decision or additional
review gate is introduced. Human approval of revision3 plus an implementation
request authorizes Executor; Challenger remains optional and human-initiated.
This Architect stage stops after delivering these documents.

No candidate filter, identity, cardinality, supporting-reference lifetime or
model transaction is changed by this plan. Loft max1 and lack of a standalone
window-selection menu entry remain limitations, with the exact test distinction
in §7.1. The stale-geometry diagnostic and installed-app provenance remain
uninvestigated beyond the source observations in §2. Physical display and
running executable acceptance are pending.

Completed documentation verification: canonical Selection/Formula reference
blocks match Interface_Elements verbatim and retain31/35-cell widths; Formula
§5 and Formula-specific tests equal HEAD verbatim, as does README_UI §22;
duplicate reference removed; all allowlisted paths exist; document code fences
balance. Scoped `git diff --check` passed. `git diff --name-only --cached` remains
empty. These checks
cannot establish build, test, runtime, GPU or manual acceptance. No production
code/tests were edited, no implementation checks were run, and the index and
external geometry changes are preserved.
