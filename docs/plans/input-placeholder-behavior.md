# Input placeholder behavior — implementation plan

Revision 1 — 2026-10-27. Role/provider: Architect / Codex. Status: ready for
human implementation approval; documentation amendments authorized and applied.
Target implementer: Executor using Luna with medium reasoning. Perform steps in
order in `/Users/martin/Desktop/Newell` on `main`; do not dispatch agents, create
branches/worktrees/snapshots, stage, commit, publish or advance another role.

## Task contract and starting state

Implement only the placeholder presentation/entry behavior in canonical
`README_UI.md` §5.13 and the Validation Operation Box in §23. The user explicitly
authorized this Architect to amend architecture and canonical UI documentation,
overriding the Architect role's ordinary prohibition on those amendments.
Production code, tests and builds are outside this documentation stage.

The checkout is on `main`. The initial index has no staged diff. There is
substantial existing unstaged and untracked work; preserve it. Relevant modified
inputs include `ARCHITECTURE.md`, `docs/ui_design/README_UI.md`,
`docs/ui_design/Interface_Elements.txt`, `crates/newell-app/src/{app,controller,
parameters}.rs`, `crates/newell-ui/src/{lib,menu/draw,menu/mod,menu/tests,
shell/compose,shell/data,shell/mod,shell/tests}.rs`, and
`crates/newell-render/src/{lib,renderer,scene}.rs`. Relevant untracked context
includes `crates/newell-app/src/extrude_session.rs`, navigation-widget sources,
`docs/adr/0007-viewport-navigation-widgets.md`, and
`docs/plans/selection-formula-reference-alignment.md` plus
`docs/plans/viewport-navigation-gizmos.md`. These include navigation rendering,
scene/selection work and canonical amendments already in progress. Other app,
model, kernel and test changes exist as shown by `git status --short`; they are
not permission to edit those areas. No source snapshot/hash is required.

Inputs inspected: shared README, Architect role, architecture; applicable ADR
0002 (single ASCII draw), ADR 0003 D7/D8 (ink/state constraints), ADR 0005
(startup), ADR 0006 (semantic palette); canonical README §§3, 5.4–5.6,
5.10–5.12, 22–23, 28.3, 36–37; `Interface_Elements.txt` numeric/Formula/Validation
Operation Box artwork; the source files identified below. Historical M4.4
behavior is corroborated by current editing/transaction code and its existing
test bodies, not assumed green from a prior report.

## 1. Components and current behavior

### Numeric inputs

- `newell-ui/src/numeric.rs`: 15 semantic `NumericFieldDescriptor`s, across
  Extrude, Revolve, Loft, Sweep and Boolean. All currently default to `0.0`;
  length/angle/unitless and directional differences must remain. `NumericEdit`,
  `NumericFieldView`, `NumericProjection` and qualified transaction types are
  the existing app/UI boundary.
- `widgets/numeric.rs`: `NumericFields` owns per-operation presentation entries,
  saved entries, one `Active` `TextEdit` and pending edits. `begin` currently
  seeds the editor from accepted rounded text or the descriptor default; it
  does not clear an initial placeholder. `commit` compares the raw draft with
  its initial rounded view to avoid reconstructing precise quantities; changed
  literals normalize to one decimal, direction changes use Direction, and empty
  or `.` becomes Incomplete. `install` refreshes accepted cache without
  replacing the active raw editor. Entries have no placeholder provenance.
- `action_box.rs` selects `NumericView` or `default_view`; `widgets/field.rs`
  composes fixed prefix/body/unit/caret and separate Formula ownership. Numeric
  `☰` and closing `]` belong to the existing Formula target; numeric value and
  opening bracket belong to the field. Direction hit testing intercepts only
  the sign. Preserve those targets and all existing allocations.
- `shell/mod.rs`: pointer click commits a different active field before routing;
  `activate_widget` branches Formula before numeric. Enter accepts the active
  field; Escape discards; Tab/BackTab commits and steps the ring. Today ring
  navigation merely focuses the next numeric field; Enter/Space starts editing.
  `numeric_focus_lost` resolves a field; modal/menu paths also resolve it.
  Deferred Formula opening, numeric projection rejection, exact OK attempts and
  Formula application acknowledgement live here.
- `newell-app/src/parameters.rs`: `CommandNumericInputs` owns real `Quantity`,
  direction, accepted source, bindings and `ParameterDraft`. All descriptors are
  initialized as actual zero quantities. Projection rounds only for display;
  it currently cannot distinguish default from explicitly accepted zero.
  `apply` clones before validation and preserves accepted values on failure;
  Formula uses a candidate transaction. `app.rs` manages sessions, numeric cache,
  accepted projections, Formula deliveries, geometry preparation and command OK.
  Cached values are cloned across operation sessions; source is cleared under
  the existing new-lifetime policy. Do not broaden/change that policy.

### Validation Operation Box

`Interface_Elements.txt` names two references “Validation Operation Box”:
Enter Name `[none …]` and Rename `[Rim_Body …]`; canonical README §23 agrees.
The actual component is `overlays/rename.rs::RenameModal`, 27×5 with a 23-cell
inner name span and shared OK/Cancel border. It currently implements **Rename
only**: fixed `Rename` title, `open(inter, name)` copies the complete name to
`applied` and `draft` and automatically focuses text. It appends/removes at the
end and composes the normal caret; it does not yet use `TextEdit` navigation.
Empty-after-trim OK is refused without closing; Cancel restores `applied`.
The shell opens it on a tree-name double-click and applies only a confirmed
rename. There is no current live Enter Name opening path and no literal `none`
placeholder in this component. Do not mistake selection fields containing
`none`, modal titles, status diagnostics or operation names for the target.

Add the Enter Name presentation mode to this shared modal and exercise it
through its public opening interface/tests. Do not add an operation-creation
workflow or new menu entry merely to expose it. Real Rename behavior stays as
it is except for ensuring empty names cannot be misclassified as placeholders.

### Renderer and specification conflict

`cell.rs::InkRole` currently has Text=0, Contour=1, Action=2.
`ui/gpu_cell.rs` serializes glyph + role as two u32s (8 bytes).
`ui/theme.rs::UiPalette` resolves three source-sRGB inks; Color off returns the
same black/white foreground for every role. `ui/pipeline.rs` writes a 96-byte
uniform with inks starting at byte 48. `ui/shader.wgsl` uses a three-entry array
and clamps invalid role indices to Text. It preserves glyph coverage as alpha,
converts source sRGB on sRGB targets and renders one instanced ASCII draw.

The previous blanket no-brightness and three-role rules forbid the requested
appearance. Authorized architecture §3.6.1 now explicitly supersedes those
restrictions, including the narrow conflicting parts of ADR 0003 D7/D8 and
ADR 0006, without editing the accepted ADR files. No other exception is implied.

## 2. Exact visual and interaction contract

Implement canonical README §5.13 and §23; the following restates its critical
acceptance conditions for the Executor.

Only inactive initial magnitude digits/decimal and Enter Name's inactive `none`
characters receive Placeholder ink. Prefix, unit, brackets, Formula control,
caret, padding, titles, buttons, labels, diagnostics, accepted text and all
active drafts retain their existing roles/ink. Fixed source-sRGB values:

| Theme | Color off | Color on |
|---|---|---|
| Dark | `#CCCCCC` | `#CDD0D3` |
| Light | `#333333` | `#414652` |

Per 8-bit channel these equal `floor((4*Text + Background)/5 + 0.5)`; compute
before target conversion. Use a fourth semantic role, not style-dependent alpha
or a general opacity setting. Keep alpha equal to glyph coverage. All four
inks keep strong full-coverage contrast (require at least 7:1 in palette tests);
actual thin-glyph legibility still needs physical-display acceptance at k=2.
Caret uses normal Text and never inherits Placeholder. Theme/Color changes
update the palette without resetting an edit or changing provenance. No layout,
frame, hover, selection, atlas, draw-count or routing change is authorized.

### Interaction decisions

1. Placeholder identity is explicit. The underlying app-owned default quantity
   remains zero. An accepted zero, nonzero rounded to `0.0`, cached accepted
   value or Formula result is real data; no text/quantity equality heuristic.
2. Numeric pointer value activation or keyboard Tab/BackTab arrival starts
   editing. Placeholder-origin drafts start with `TextEdit::new("")`, caret=0;
   accepted values start with their current rounded display, as before. Existing
   Enter/Space activation on a merely focused numeric value does the same.
   Repeated click/activation of the active field is idempotent. A navigation or
   focus action never inserts a space or digit. Modal text focus, including
   automatic opening focus, follows the same rule. OS focus gain alone and
   Formula/Enter focus restoration do not automatically reenter numeric edit.
3. Empty untouched numeric blur/Enter ends editing and restores the placeholder,
   emitting **no NumericRequest**. Enter does not confirm the command. A text
   mutation means a successful content change, not navigation, selection,
   rejected input, placeholder hiding or a sign change. Typing then deleting all
   text is an attempted edit and follows Incomplete rejection. Escape restores
   accepted presentation/provenance and emits no edit for the cancelled draft.
4. Direction-only changes preserve quantity/source/provenance, including from
   an otherwise untouched empty editor. Emptying a real accepted value is an
   invalid edit; rejection restores that value, never the placeholder. Invalid
   literals never replace accepted quantities, bindings or expressions. Numeric
   diagnostics and confirmation suspension retain their existing behavior.
5. Enter Name keeps empty applied/draft strings; `none` is display-only. It
   opens auto-focused with an empty editor/caret, rather than a flash of `none`.
   On text blur exactly empty draft shows `none`; nonempty/whitespace drafts stay
   visible normally. Reentering never erases a nonempty draft. Rename always
   seeds a real existing name, including a literal `none`, and never substitutes
   a placeholder after deleting it. No generic string-empty clearing logic.
6. Name blur does not commit. Enter/OK with trimmed-empty draft keeps the modal
   open and applied name unchanged, preserving current validation behavior;
   don't introduce new diagnostics. Cancel/Escape closes and restores applied
   text. Reopening empty Enter Name repeats placeholder/auto-focus behavior;
   reopening Rename restores the actual full name. No placeholder in payloads.
7. Direct Formula activation never calls numeric `begin`. An active editor first
   resolves via existing blur acceptance/rejection, then Formula opens through
   its existing sequence/barrier and accepted-source projection. Opening,
   closing and Cancel preserve placeholder provenance. Nonempty successful
   Formula OK marks evaluated values accepted even when zero. Rejected Formula
   OK retains source/caret/selection/diagnostic and the prior accepted value.
   Empty Formula OK keeps its existing literal-retention/expression-removal
   meaning and preserves provenance. Formula close restores its originating
   Formula control, without starting a numeric empty draft. Command OK still
   commits before exact acknowledgement closes the panel; cancellation,
   retry/one-shot delivery and stale-namespace guards remain intact.

## 3. State transitions

Here P means explicit initial-placeholder provenance, A accepted provenance,
and raw means window-local editor text. Provenance survives blur/reprojection.

| State/event | Display / next state | Acceptance and side effects |
|---|---|---|
| P inactive | Dim `0.0` or Enter Name `none` | No raw placeholder characters in payload |
| P + value/text focus | Focused-empty, normal caret | Raw empty; no candidate/model/dirty change |
| Focused-empty + navigation/rejected key | Focused-empty | Not a content mutation |
| Focused-empty + type | Edited raw, normal Text | Local draft only |
| P focused-empty + blur/Enter | Numeric: P inactive; name: rules below | Numeric emits nothing; name Enter is invalid |
| Edited + numeric valid acceptance | A (possibly optimistic pending projection) | Existing qualified literal validation; accepted zero stays A |
| A + focus / repeated click | A raw editor, current text retained | No replacement/rounding commit just from focus |
| Any active numeric + Escape | Cancelled → previous A or P | No new candidate; accepted expression retained |
| Edited numeric empty/`.` + acceptance | Invalid → accepted projection A/P | Existing Incomplete rejection; no accepted-value loss |
| Edited name + blur/refocus | Raw retained; caret only with text focus | No name commit; exactly empty Enter Name shows P when inactive |
| Name empty/whitespace + Enter/OK | Invalid, modal open | Applied name retained; no rename publication |
| Valid name + Enter/OK | Accepted, modal confirmed | Existing name validation/application path |
| Name + Cancel/Escape | Cancelled, modal closed | Applied name restored; no new name publication |
| P/A + Formula open/Cancel | P/A preserved | Existing deferred open and focus return |
| Formula valid nonempty OK | A, modal closes after acknowledgement | App accepts source/value before close |
| Formula invalid OK | Invalid modal raw retained; prior A/P retained | No accepted replacement; modal remains open |

An explicitly entered magnitude zero is not equal to an untouched placeholder
for this state machine, even if their displayed text and quantity match.

## 4. Concrete interfaces and ownership

Use the existing architecture and request types; add no dependency, model field,
serialization format, new crate, service or parallel validation path.

### App-owned numeric provenance

- Add `initial_placeholder: bool` to `parameters.rs::NumericDraftValue` and to
  public `numeric.rs::NumericFieldView`. Fresh descriptor initialization sets
  true; projection copies it. It is command-input Controller metadata, not
  `newell-model` state. Existing-data initialization must set false regardless
  of value; no new Properties/loading workflow is required by this plan.
- Successful Literal acceptance sets false **after validation**, even for `0`
  or when quantity equality says unchanged. Successful nonempty Formula sets
  false on its field and other binding-derived projected values. Failed
  acceptance preserves prior flag. Direction and empty Formula acceptance
  preserve the previous flag; newly computed values bound to expressions are
  real accepted values. Cache clones carry the flag with each value, including
  hidden branches and operation switches; session invalidation drops it with
  the existing owning state. Do not reinterpret a cleared formula cache as a
  new placeholder.
- Separate presentation provenance from semantic geometry change detection:
  transitioning default-zero → accepted-zero must be projected even if numeric
  equality is unchanged, but must not become an independent model command or
  geometry change. Keep `apply`'s change result based on its existing semantic
  quantity/direction/source comparison, explicitly excluding the new flag;
  assign validated provenance even if that comparison is false. Formula
  transaction candidate-cloning must include provenance and rollback it on error.
  Existing projection scheduling must install the flag after successful requests
  regardless of semantic equality; adjust only if inspection shows it skips it.

### UI raw editing and cache

- Add the same flag to `widgets/numeric.rs::Entry`, plus
  `is_placeholder: bool` to `NumericView`. `default_view` is placeholder;
  `install` copies the app-supplied flag. View returns Placeholder only when
  inactive and explicitly initial, never when raw editing is active.
- Extend `Active` with `initial_placeholder` and `text_mutated`. Retain the
  existing rounded initial accepted magnitude/direction for M4.4 comparison.
  Seed raw empty only for initial placeholders. Track successful content
  mutation in typed/backspace/Delete/selection replacement, separately from
  movement/selection/sign. `begin` on the already active field does nothing.
- Order `commit` decisions: initial-placeholder + no text mutation → no edit
  unless direction changed, in which case Direction only; otherwise preserve
  existing comparison for real untouched accepted values, normalize valid raw
  literals, emit Incomplete for empty/`.` attempted edits. A placeholder-origin
  valid literal is explicit even if equal to `default_magnitude`. An accepted
  untouched rounded value must never be reconstructed as a literal. Update the
  optimistic flag for explicit literal candidates only; accepted projection
  restores both flag and value on rejection. `saved`, `confirm`, `cancel`,
  `discard` and `reset` must carry/restore the flag with the existing cache.
- In `shell/mod.rs::step_ring`, after setting a genuinely different focused
  target, begin editing only if it is a visible numeric value and no modal/menu
  owns input or delivery is pending. Reuse `numeric_descriptor` and
  `NumericFields::begin`; do not call generic activation on a Formula, sign,
  selection or button target. Handle Tab between fields with the current
  pre-input routing certificate; preserve idempotence and normal activation.
  Do not add auto-begin to `sync`/projection install/focus restoration: that
  would reopen on Enter and interfere with Formula. Ensure hiding an active
  field resolves it through the existing input event before its certificate is
  lost, preserving accepted state and existing hidden-field safeguards.
- `field.rs::draw_numeric_view` marks only the visible inactive placeholder
  body span, bounded by `body.len()` and allocation clipping, as Placeholder.
  Never recolor the suffix, prefix, caret, Formula or bracket cells, and never
  infer role from glyph contents. Existing `Canvas::ink_at` suffices.

### Shared name modal

Keep `RenameModal` and its existing `open(inter, name)` API as real-name Rename.
Add `open_enter_name(&mut self, inter: &mut InteractionState)` for an explicitly
empty Enter Name instance. Store a small local mode enum `Rename | EnterName`;
select `Rename`/`Enter Name` title and initial applied/draft from that mode.
Reuse the same size, IDs, overlay, field-window, focus ring, append/backspace,
validation, confirmation and focus restoration. Render `none` with Placeholder
only for mode EnterName + draft exactly empty + text not focused; focused empty
renders the ordinary caret. This derives placeholder visibility without a
second accepted-name store or a string-sentinel comparison. Do not migrate this
modal to a new generic editing engine or add shortcuts/clipboard behavior.

The live tree rename entry and `finish_rename` continue using the real-name API.
The new Enter Name API is directly testable without wiring a new shell command.
For manual testing it needs a component harness; explicitly report that the
application has no Enter Name trigger rather than claiming end-to-end coverage.

### Palette interface

Add `InkRole::Placeholder = 3`; preserve indices 0–2 and the 8-byte GPU cell.
`UiPalette::foreground` handles Placeholder before the Color-off early return.
Use exact canonical source colors; existing Text/Contour/Action APIs are
unchanged. Expand uniform inks to four, total bytes 112 (48-byte existing header
+ four aligned vec4s); update allocation/min-binding size/tests consistently.
WGSL uses `array<vec4<f32>, 4>` and valid index `<4u`, with invalid input still
falling back to Text. No alpha multiplier, style lookup or second draw. Existing
Contour-only glyph conversion and full-cell overwrite must preserve Placeholder
metadata. Existing source/target sRGB conversion stays unchanged.

## 5. Ordered implementation and explicit file scope

Before editing, reread the current approved revision and record status/index and
the standard baseline in `docs/agents/README.md` with snapshot updates disabled.
An existing failure is recorded separately; do not overwrite other work or
silently weaken checks. Implement sequentially:

1. Add provenance to the numeric exchange and app draft; update all in-scope
   construction fixtures to set it explicitly. Test accepted zero, failed
   rollback, projection and cache preservation before wiring editor behavior.
2. Add raw-editor origin/mutation logic; keep accepted-value precision/direction
   rules. Add focused-empty and Escape/Incomplete unit tests.
3. Wire keyboard entry and preserve pointer/Formula ordering, pending-delivery
   guards, hidden-field and OS focus-loss behavior in the existing shell.
4. Add shared Enter Name mode/API and placeholder rendering. Keep live Rename
   entry and validation unchanged. Test both modes and an actual name `none`.
5. Extend the semantic ink and GPU uniform/palette contract together; annotate
   only placeholder body characters. Verify unchanged ownership/geometry and
   caret ink across four palette choices.
6. Add end-to-end numeric regressions in existing app test modules; run focused
   checks, required workspace verification, GPU checks and manual acceptance.
   Report baseline/new failures, actual command results and physical checks
   independently. Stop after Executor; no automatic Auditor or review.

Allowed production files, all under `crates/`:

| File | Permitted change |
|---|---|
| `newell-ui/src/numeric.rs` | View provenance and related fixtures |
| `newell-ui/src/widgets/numeric.rs` | Cache, raw-origin/mutation, view, exit behavior; local tests |
| `newell-ui/src/widgets/field.rs` | Exact placeholder-body ink; local geometry/ink tests |
| `newell-ui/src/cell.rs` | Fourth role and stable-index test |
| `newell-ui/src/shell/mod.rs` | Numeric focus entry/reconciliation only |
| `newell-ui/src/overlays/rename.rs` | Explicit Enter Name mode/API, local placeholder tests |
| `newell-app/src/parameters.rs` | Validated provenance and projection; local tests |
| `newell-app/src/app.rs` | Existing session/cache initialization/projection handling if needed; existing numeric tests |
| `newell-render/src/ui/theme.rs` | Placeholder palette and contrast tests |
| `newell-render/src/ui/pipeline.rs` | Four-ink uniform and CPU/GPU tests |
| `newell-render/src/ui/shader.wgsl` | Four palette entries and bounds |

Allowed supporting test files (only where required for interface fixtures and
regressions): `newell-ui/src/{action_box.rs,shell/tests.rs,shell/tree_tests.rs}`,
`newell-render/src/ui/gpu_cell.rs`, `newell-app/src/controller.rs`,
`newell-app/tests/it/input.rs`. Inspect `NumericFieldView`/`NumericView` literals
within these affected crates to update any additional test-only construction
sites; report their paths. No broad formatting, unrelated refactor or snapshot
regeneration. Production changes outside the table require a concrete scope
reason, not convenience. `action_box.rs` composition already receives
`NumericView`; no production change there is expected.

No changes to `newell-model`, kernel, document serialization, geometry workers,
scene/nav widgets, menus, feature creation, command payload types, dependencies,
Cargo.lock, MSRV or licensing. Existing Rust 1.88 and AGPL headers remain.
This Architect changed only this plan and the three canonical documents below.

## 6. Targeted regression evidence

Tests must exercise behavior and transaction outcomes, not merely enum fields.
Reuse current helpers and existing M4.4 tests rather than a duplicate harness.

| Layer / location | Required cases and assertions |
|---|---|
| `widgets/numeric.rs` | Default focus produces empty raw/caret=0; click again retains typed raw; empty untouched Enter/blur/Escape emits no candidate; text then delete emits Incomplete; `0`, `0.0`, `.0` become explicit real zero on successful acceptance; reenter keeps zero. Normal accepted untouched value emits nothing. Navigation/rejected input is not mutation. Direction-only from empty preserves source/flag. |
| `parameters.rs` | All 15 defaults project initial; accepted zero changes provenance without a new semantic quantity change; literal failure preserves entire accepted value/flag/source; Direction preserves exact quantity/expression/provenance; successful nonempty Formula evaluating zero is accepted; empty Formula OK follows literal retention and prior provenance; invalid Formula rolls back flag/bindings/registry. |
| `shell/tests.rs` | Pointer body vs Formula and sign targets; Tab/BackTab arrival clears only P; leaving empty returns P; Enter ends only field, no auto-begin loop; Escape restores P/A; repeated clicks preserve drafts; blur/hiding/menu/OS loss resolves without fake literal; projections preserve same-session active raw text and new-session flags. |
| `overlays/rename.rs` | Enter Name auto-focus empty/caret, blur `none` dim, refocus empty; type name/blur/refocus preserves text; empty/whitespace Enter and OK stay open and leave applied unchanged; Cancel restores and returns focus; accepted/reopened name `none` and existing `Rim_Body` never dim/clear; deleting real name never shows placeholder; exact frame/title/field width/IDs/shared border preserved. |
| `field.rs`, cell/GPU packing, palette/pipeline | Only placeholder digits or `none` use role=3; unit/prefix/brackets/Formula/caret/labels retain exact roles. Clipped/overwritten overlays retain no underlying placeholder ink. All four exact colors, >=7:1 full-coverage contrast, unchanged normal roles, 112-byte uniform offsets, role index=3 preserved by 8-byte GPU packing, invalid role fallback. GPU smoke for both sRGB/non-sRGB targets in existing ignored-device test; same one ASCII draw count. |
| `app.rs` numeric tests | Capture document version/dirty state, accepted quantity/direction/source/binding/parameter draft, command-history/commit evidence and numeric request/sequence before value focus → empty blur/Enter/Escape. Assert no model or parameter mutation, no numeric candidate or command and no sequence consumption for that transition. Explicit zero must survive projection/reentry/cache/branch switches. An accepted high-precision quantity displayed rounded must retain its exact quantity and expression on untouched focus/blur. |
| `app.rs` Formula/confirmation tests | Direct Formula click from P produces no numeric edit and initializes accepted source; active changed valid numeric blur is accepted before opening; invalid blur restores last accepted source before opening; empty untouched blur doesn't reject or block opening. Formula Cancel/invalid OK preserves P/A and accepted source. Valid formula zero never clears on later focus. Preserve one-shot retry/stale IDs, Formula application-before-close and exact command OK acknowledgement; empty untouched focus before explicit command OK must not create an Incomplete failure or an extra command. |

Use the existing app precision/Formula/confirmation tests (near the current
numeric helpers and `invalid_formula_stays_open_and_cancel_preserves_the_accepted_offset`)
as regression anchors. Extend controller OS-focus-loss assertions where input
crosses the published-shell copy. Provenance-only zero acceptance must not make
these wrappers discard a projection on their semantic no-change fast path.

Executor commands: focused `cargo test -p newell-ui --locked`,
`cargo test -p newell-app --locked`, `cargo test -p newell-render --locked`
with `INSTA_UPDATE=no`, followed by all shared workflow required workspace
checks (check, fmt --check, clippy, nextest or documented fallback, doc tests).
GPU work is relevant: run the shared ignored-device render tests when a device
is available; record unavailability rather than claiming GPU success. Format
only assigned touched files in this dirty checkout. Do not regenerate snapshots
just to pass; any needed reference change must correspond exactly to §5.13.

## 7. Manual acceptance matrix

Repeat the following in **Dark/off, Light/off, Dark/on, Light/on**, both at the
smallest k=2 on a physical display and at normal working scale. Record actual
Theme/Color and scale/display used; automated contrast tests are not visual QA.

- New numeric initial `0.0` is subtly dim and readable; label/unit/sign/frame/☰
  have ordinary palette ink and unchanged cell positions. Click the editable
  body: placeholder vanishes immediately, normal caret appears in an empty
  magnitude. No model/dirty/command change. Click again after typing: retain it.
- Tab/BackTab onto a numeric value has the same entry behavior. Enter/blur of
  untouched empty restores P without an error; Escape after editing restores
  the prior P/A. Type and accept `0`: it becomes normal text and survives every
  later click, keyboard focus, hide/show and operation-cache restore. Repeat
  with nonzero, angle, unitless, directional and unsigned fields.
- For an accepted precise/Formula-driven value, untouched focus/blur retains
  exact quantity and source. Delete to empty/enter `.` and accept: reject and
  restore accepted value. Toggle direction before/while editing; no magnitude
  or binding loss. Test OS focus loss and branch hiding as well as ordinary blur.
- Direct ☰ click opens Formula, without a temporary empty numeric editor.
  Test valid numeric draft → ☰ and invalid numeric draft → ☰; opening uses the
  resulting accepted source. Formula Cancel keeps original state; invalid OK
  stays open; valid zero result stays visible normally; empty OK retains literal
  according to existing semantics. Exercise explicit command OK/retry and
  Cancel, checking commit/close order and absence of extra commands.
- In the shared modal's Enter Name component harness, opening auto-focuses an
  empty field with normal caret; Tab to OK shows dim `none`; click/keyboard
  return hides it immediately. Type a name, move focus away/back: keep text.
  Empty/whitespace OK refuses to close; Cancel restores applied state/focus.
  Confirm a name, then open it through real Rename: normal text. In the live
  application double-click an existing operation named `none` and another named
  `Rim_Body`: neither dims or clears. Delete name, attempt empty OK, Cancel:
  original name preserved. Enter Name currently has no live app trigger; report
  harness acceptance separately and do not add a trigger in this change.
- Switch Theme/Color while P is visible and while a raw edit exists: correct
  colors, unchanged text/caret/state. Hover controls and inspect selection:
  existing glyph-only effects stay unchanged. Check ordinary labels, validation
  messages, buttons and names never use placeholder ink. Check modal overlays
  leave no dim text behind after closing, and a second window's state is separate.

## 8. Canonical amendments, verification and handoff

Applied in this Architect stage:

- `ARCHITECTURE.md` §3.6.1: explicit human-authorized exception to blanket
  brightness/black-white/three-role restrictions, fourth Placeholder role,
  exact palette/derivation, scope exclusions, metadata ownership and transaction
  invariants; explicit narrow precedence over conflicting accepted ADR wording.
  Existing navigation §3.7 and prior changes are retained.
- `docs/ui_design/README_UI.md`: §3 disambiguates default `0.0`; §5.10 links the
  placeholder exception; new §5.13 defines ink and full lifecycle; §28.3 links the palette exception;
  §23 identifies
  Validation Operation Box and display-only `none`; §36 includes the fourth
  role. All ASCII frames and existing Formula/caret/hover rules are retained.
- `docs/ui_design/Interface_Elements.txt`: annotation at Validation Operation Box
  identifies the inactive placeholder, focus behavior and numeric provenance;
  artwork is unchanged. Accepted ADRs are not edited.

No unresolved product decisions. Enter Name's absent application entry is a
known current implementation boundary, resolved by a shared explicit mode/API
and component coverage rather than new workflow scope. Executor may resolve
local implementation details within this contract without widening it.

Architect verification: source/spec reading and documentation diff inspection
only. No builds, tests, application or GPU commands run; no runtime or physical
acceptance claimed. Implementation and all acceptance checks above remain
pending. Recommended next stage: human approval of revision 1 and an explicit
Executor implementation request, optionally preceded by a human-requested
Challenger review. Neither stage is started by this handoff.
