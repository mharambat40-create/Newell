# Selection-box content scrolling — implementation plan

Revision 2 — 2026-10-26 — Architect / Codex. Targeted correction complete;
production implementation awaits a human request. Intended Executor: Luna,
medium reasoning effort. Task identifier: selection-box-scrollbar.

## Contract and starting state

Work sequentially in `/Users/martin/Desktop/Newell` on `main`. Preserve all
existing work and the Git index. No branches, worktrees, source snapshots,
commits, parallel agents or automatic role transitions. Implement only this
plan when the human approves revision 2 and requests Executor.

The human explicitly authorized the Architect to edit the UI specifications,
overriding the role document's ordinary plan-only write limit for this task.
Canonical behavior is now in `docs/ui_design/README_UI.md` §§19–21, with matching
references in `docs/ui_design/Interface_Elements.txt`. Refer to §21 for the
normative state table, arithmetic, clipping and input rules; do not implement
from historical M2 plan illustrations. No architecture amendment is necessary:
§§3.0–3.6 already permit these glyphs, semantic roles, plain Controller state,
O(1) routing and existing drag mechanisms. Accepted ADRs 0003/0006 govern cell
geometry and ink; this task changes neither.

At the revision 1 inspection, the checkout was on main with no staged changes and extensive
unstaged/untracked prior work. Relevant modified inputs include
`ARCHITECTURE.md`, `README_UI.md`, `overlays/list.rs`, `overlays/selection.rs`,
`overlays/tests.rs`, `shell/mod.rs`, `shell/compose.rs`, `shell/tests.rs`,
`newell-ui/src/lib.rs`, and `newell-app/src/controller.rs` plus its integration
tests (source paths below). They include semantic ink and model-bound tree /
controller publication work. Interface_Elements.txt was clean; this plan did
not exist. Do not restore any older version or replace prior changes wholesale.
Re-read these current inputs and report a concrete conflict if they change.

Revision 2 addresses the three Challenger findings only. At this correction's
start, this plan remained untracked and the two canonical specification files
retained their revision 1 edits. The inspected source paths below still exhibit
the reported wheel/publication behavior, reserved thumb ID and eight-row
selection snapshot. This stage changes only this plan; all other existing work
and the index are preserved. The correction does not authorize implementation.

| Challenger finding | Revision 2 correction |
|---|---|
| Snapshot omitted from allowlist | Allow the exact selection-panel snapshot, limited to specification-backed list cells/styles; retain disabled automatic updates and explicit diff review. |
| Blanket wheel/key capture cancellation | Preserve consumed wheel input during capture and existing key routing; distinguish unchanged routing from keyboard scrolling that invalidates the anchor. |
| Profile fixture IDs collide with thumb | Give the internal profile row writer explicit item IDs; larger test fixtures use IDs disjoint from all controls, without changing production's nine-item allocation. |

Excluded: treeview/application-panel scrolling, global overlay layout, renderer,
GPU/shaders, glyph atlas, palettes, model/kernel, real geometry selection,
reordering/removal implementation, new gestures or keyboard commands, variable
mock-data public APIs, dependencies and unrelated cleanup. The nine-item mock
panels stay mock panels. Dynamic counts are supported/tested by list geometry,
state reconciliation and list composition, without inventing model integration.

## Inspected ownership and change map

| File / owner | Current writers/interfaces | Required change |
|---|---|---|
| `crates/newell-ui/src/overlays/list.rs` | `ScrollState::{max_offset,clamp,scroll_by,ensure_visible}`, `ScrollList`, `draw_scroll_list`, private `draw_scrollbar` | Centralize selection-list geometry and exact half-slot thumb calculation; cap logical content at five, conditionally allocate gutter, clear stale metadata; fix invalid-offset handling before deltas. Keep generic ScrollState's explicit-window API. |
| `crates/newell-ui/src/overlays/selection.rs` | `Geometry::{from_interior,of,from_action_box}`, `SelectionPanel` owns `scroll`, `draft`, `applied`, `profile_applied`, `focus`; `draw_content`, `draw_selected_items`, `draw_selected_scrollbar` | Consume same geometry/scrollbar writer for checkbox and profile rows; remove duplicated proportional thumb code. Both presentations use actual active labels/count, logical window and gutter eligibility. |
| same | `scroll_by`, `scroll_in_action_box`, `activate_in`, `move_focus`, `ring`, `settle`, `offset`, `geometry`, `thumb_geometry`, `thumb_grab_offset`, `drag_thumb` | Use consistent clamped state, enabled arrows and five-row ensure-visible range. Replace row-sized thumb derivation with the shared half-slot geometry; reconcile focus and stored offset. Preserve confirm/cancel/open behavior. |
| `crates/newell-ui/src/overlays/mod.rs`, `crates/newell-ui/src/lib.rs` | Current overlay/list exports | Export only the small Newell-owned grab value needed by app; keep geometry/arithmetic/writer crate-private. No renderer/model API. |
| `crates/newell-ui/src/shell/mod.rs` | `selection_thumb_grab_offset`, `drag_selection_thumb`, `captured_drag_available`, `scroll`, `sync`, `profile_selection_frame` | Forward new grab interface, normalize selection state before composing/syncing focus, validate current scrollbar/geometry rather than merely `is_open()`. Keep wheel/modal routing and source Action Box footprint. |
| `crates/newell-app/src/controller.rs` | `CapturedDrag::SelectionThumb`, `pointer_pressed`, `pointer_moved`, `cursor_hint`, `publish`, `FrameCause`; `scroll_panel` routes modal input | Store/forward grab anchor. Preserve own selection-drag capture when the generated list hit map moves; invalidate on external routing/offset or geometry/count changes. Preserve wheel suppression during capture and existing key routing. Retain all separator/tree generation safeguards. |
| tests listed below | Existing list, selection, shell, controller cases | Replace only specification-obsolete expectations and add targeted state/input regressions. |

`overlays/place.rs::{modal_layout,center_modal}`, `overlays/mod.rs::compose_overlays`,
`shell/compose.rs` and `widgets/hover.rs` are read-only integration references.
Their global clipping, modal blocking, composition order and hover policy do not
need changes. Selection-specific drawing can clear its own allocation without
changing global overlay composition. The renderer only consumes the flat grid.

Concrete current discrepancies: generic `draw_scroll_list` always allocates a
bar when geometry permits, even for empty/fitting lists; its thumb uses
whole-cell floor proportion. Profile `draw_selected_scrollbar` uses a different
ceil proportion and end-cap treatment. Generic `Geometry` currently exposes
all eight available body rows in its normal 30×12 modal. `thumb_geometry`
repeats both calculations. `Controller::publish` cancels non-owned captures
when ownership changes; a selection scroll changes the visible item IDs and can
therefore end its own drag. These are bounded implementation issues, not
unresolved product decisions.

## Internal design and interfaces

Use the existing `list.rs` module; add no framework or dependency. Proposed
internal names below define responsibilities and data, not a need for public
APIs beyond the grab value.

1. `SelectionListGeometry::new(allocation: GridRect, len: u32)` returns plain
   data: logical window `min(len,5)`, maximum offset `len.saturating_sub(5)`,
   content rect, optional logical five-row gutter/three-row track, and clipping
   allocation. For zero items the logical window is zero and the retained area
   is blank. Gutter presence depends on `len>5`, never on physical height.
   Width one may leave zero content width; width zero writes nothing. Logical
   track remains three cells even when clipped. Calculate coordinates with
   widened/saturating arithmetic, honoring GridRect's u64 right/bottom bounds.
2. `SelectionThumb::new(len: u32, offset: u32) -> Option<SelectionThumb>` is
   `None` at `len<=5`; otherwise returns clamped offset, `max`, `length`,
   `travel`, `position` and a three-character track value. Use integer quotient /
   remainder half-up rounding from §21.2 and u64 products. Independent test
   literals, not another copy of its glyph algorithm, are the oracle.
3. Make one crate-private scrollbar writer in `list.rs` accepting geometry,
   thumb and `up/down/thumb: WidgetId`. Both existing presentations call it.
   It writes all five bar cells, including blank track metadata. Full or partial
   thumb cells own the existing thumb ID; blank track cells never own it.
   Endpoint arrows stay drawn and logically disabled. Keep semantic ink and
   logical styles; no new hover eligibility.
4. Leave `ScrollList`'s public field shape and `draw_scroll_list` signature
   unchanged. Its supplied rect is the available allocation, not the desired
   visible-row count. Clear that whole allocation before composing at most five
   logical item rows; intersect writes with allocation/grid. Preserve item IDs
   from `ListItem`. Profile drawing keeps its existing row grammar and calls
   the same geometry/writer after clearing its list allocation. Factor its
   internal row-composition helper to accept borrowed `(WidgetId, &str)` rows;
   use each supplied ID directly, never derive it from a visible row or label.
   Production supplies the existing nine `(self.item_id(index), label)` pairs.
   Larger test fixtures supply distinct item IDs outside every surface/arrow/
   thumb/button ID. Do not extend production's fixed ID allocation: index 9
   would otherwise alias `THUMB` because `FIRST_ITEM + 9 == 14`.
5. `ScrollState` remains reusable with an explicit `window`; do not hard-code
   five inside it. Clamp before applying a delta so a stale offset `MAX` then
   `-1` gives `M-1`, rather than simply `M`. Saturating unsigned magnitude
   handles `i32::MIN`. Also remove overflow from shell's current `-steps`
   wheel conversion: panel adds crate-private
   `scroll_wheel(steps: i32, bounds: GridRect, profile_frame: GridRect) -> bool`,
   selects its existing geometry, and negates `i64::from(steps)` into an internal
   wide-delta helper. Keep public i32 scroll entrypoints as wrappers; share the
   clamped/saturating mutation through crate-private `scroll_by_wide(i64, len,
   window)` on ScrollState, using u64 magnitude before the final bounded cast.
   Shell forwards original signed steps once; it does not negate i32::MIN.
   Selection callers pass the logical window (zero for an
   empty list); empty offsets normalize to zero. Composition defensively clamps
   but mutating paths store the normalized result. Pure draw stays immutable.
6. Selection geometry accepts the active item count; use label count instead of
   independent `ITEMS`/`ITEM_COUNT` assumptions for scrolling. Current arrays and
   fixed item IDs need no dynamic rewrite. Centralize `reconcile(len, geometry)`
   within the panel's existing settle/sync path: clamp offset; remove unavailable
   arrows from focus ring, fallback to OK; preserve surviving item focus and all
   draft/applied flags. Don't auto-scroll focused items merely because wheel
   changed offset; only keyboard item navigation ensures visibility. Resolve
   count/geometry before all rendering, activation and drag paths.
7. Keep generic `SIZE=(30,12)`, profile footprint `(31,11)`, title/footer and
   frames in their current placement. Use only the top logical content rows of
   the existing body; remaining generic body rows are inert blanks. No outer
   resize on scrolling or count changes. The §19 compact artwork is a content
   reference; this task does not replace the generic modal's existing chrome.
   Clear before early returns for narrow/zero content and when gutter vanishes.

### Grab and capture contract

Introduce `SelectionThumbGrab` as a public, opaque, Copy plain-data value in
`selection.rs`, re-exported through `overlays` and `newell-ui`. Private fields:
initial pointer row (`u32`), initial exact offset (`u32`), initial half-slot
position (`u32`), item count (`u32`), and list allocation/track geometry needed
to validate the anchor. It contains no pointer, model identity or winit type.

Replace the existing grab-offset methods with:

- panel `thumb_grab(id, row, bounds, profile_frame) -> Option<SelectionThumbGrab>`;
- panel `drag_thumb(row, grab, bounds, profile_frame) -> bool`;
- shell `selection_thumb_grab(&HitTestResult) -> Option<SelectionThumbGrab>`;
- shell `drag_selection_thumb(row: u32, grab: SelectionThumbGrab) -> bool`;
- shell `selection_thumb_grab_valid(grab: SelectionThumbGrab) -> bool`.

The grab starts only on an owned thumb cell in the currently open eligible
selection overlay. Keep whole-cell hit targets, including the uninked half of
a partial glyph. Subsequent rows map exactly by §21.3's anchored inverse; use
i64 for signed row difference / `p0+2d`, u64 for `M*t`, and clamp before casts.
No movement must preserve an exact offset in a quantization bucket. A clamped
drag beyond either end reaches exact endpoints, even for a five-half-slot thumb.
Validate unchanged count, kind, body position/size and open overlay before use.
Preserve `Controller::scroll_panel`'s early return while a drag is active:
wheel/trackpad input is consumed (`handled=true`) without scrolling, publishing,
falling through to scene navigation or cancelling capture. Outside capture it
uses the existing modal route. Keyboard input retains its existing routing:
Space/Enter on a checkbox may change draft state without changing cell ownership,
and focus movement within the current window need not change ownership either;
these inputs retain a still-valid grab. Keyboard scrolling that changes the
content offset/visible item routes invalidates the old anchor through external
publication; dismissal or external geometry/count change also invalidates it.
Do not add unconditional wheel/key cancellation. An owned pointer scroll
preserves capture across its own changed item routes.

In controller, replace `SelectionThumb { grab_offset }` with `{ grab }`. Continue
passing physical pointer y through the existing metrics-derived whole-row
conversion; no new pixel precision or gesture. Extend `FrameCause` narrowly with
an owned-selection-drag cause carrying the matching grab anchor. `publish`
recognizes this cause only for the current same grab, unchanged selection
geometry/count and eligible modal; it retains that capture across the resulting
item/bar hit-map changes. All external causes retain their cancellation rules;
never mark an arbitrary external publication as owned, and do not relax tree
route/generation invalidation. A failed publication restores the previous state
and does not certify a new capture. `cursor_hint` validates the grab for
Grabbing; fresh Grab still comes from the current O(1) thumb hit. Disappearance
must clear capture even when an update leaves the rest of the hit map equal.
Existing release/exit/focus-loss/resize/scale cancellation continues to apply.

## Exact Executor write allowlist

Production files (only the functions/interfaces above and direct callers):

- `crates/newell-ui/src/overlays/list.rs`
- `crates/newell-ui/src/overlays/selection.rs`
- `crates/newell-ui/src/overlays/mod.rs`
- `crates/newell-ui/src/lib.rs`
- `crates/newell-ui/src/shell/mod.rs`
- `crates/newell-app/src/controller.rs`

Test files:

- Existing inline tests in `overlays/list.rs`, `overlays/selection.rs` and
  `newell-app/src/controller.rs` (full paths above).
- `crates/newell-ui/src/overlays/tests.rs`
- `crates/newell-ui/src/shell/tests.rs`
- `crates/newell-app/tests/it/controller.rs`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__selection_panel_open.snap`

The one allowed snapshot change is the generic selection list's five-row
content/bar output, cleared surplus body rows and directly affected logical
style cells, as required by README_UI §21. Preserve outer chrome, underlying
shell/source-panel content, unrelated glyphs/styles and snapshot metadata.
Keep `crates/newell-ui/tests/it/snapshots.rs` read-only. Its
`selection_panel_open` assertion already exercises this output; do not weaken,
skip or normalize it. Review the expected-cell diff explicitly and edit the
allowed snapshot deliberately with automatic snapshot updates disabled. If
existing startup/tree/menu differences prevent the snapshot test reaching or
matching this state, record them as baseline issues; do not absorb them into
this task's snapshot update or alter the harness outside the allowlist.

Executor may append implementation/check results to this plan. The two updated
specification files are read-only inputs for Executor. All other files,
including all other snapshots, Cargo files, architecture, accepted ADRs, tree/controller
integration outside the identified selection path and unrelated formatting are
outside the write scope. Additional required snapshot changes or a structural conflict
must be reported with the exact affected file; do not approve them implicitly.
Use existing Rust 1.88 / edition 2024 and AGPL headers; no dependency/MSRV/license
change is needed.

## Ordered implementation checklist

1. Read current instructions and revision 2; inspect scoped dirty changes and
   index without altering either. Record verification baseline required by
   `docs/agents/README.md`, with snapshot auto-updates disabled. Separate prior
   failures from new regressions.
2. Add the shared geometry, half-slot model and deterministic arithmetic in
   list.rs, with literal-state and boundary tests below. Update scroll mutation
   clamping and retain generic explicit-window behavior.
3. Change checkbox list composition and the profile writer to consume it. Clear
   allocations and metadata first; constrain content to its region. Update
   selection geometry, all offset/arrow/focus/ensure-visible paths and private
   reconciliation; keep selection identities and current panel footprints.
4. Add/forward the grab value and anchored inverse. Make the scoped controller
   publication/cursor eligibility adjustment so repeated drag updates work.
   Keep separator generations, cancellation and modal consumption intact.
5. Replace obsolete tests (full-cell thumbs, empty/fitting arrows, generic eight
   rows, profile ceil end caps) with canonical expectations. Add meaningful
   interaction and lifecycle regressions. Update inline docs/comments with the
   changed contracts. Review and deliberately update only the allowed selection
   snapshot's specification-backed cells/styles, keeping automatic updates
   disabled; do not rewrite unrelated assertions or snapshots.
6. Run focused tests, then the required workspace checks below. Format assigned
   Rust files only. Review final diff for scope, glyph/ink/ID clearing, unchanged
   item identity, no palette/render changes and no unintended index changes.
7. Report results and pending manual checks. Do not commit, publish or start
   another role.

## Specified tests and measurable acceptance

These are requirements for Executor, not tests run by Architect.

| Coverage | Required evidence |
|---|---|
| Every supplied state | Literal table-driven cases for all 20 offsets of N=6…10 from README_UI §21.2; assert all three track chars including spaces, L/p, five content rows, arrow coordinates, no sixth content row and correct first/last item IDs. Exercise shared checkbox writer and profile path, not only arithmetic. |
| N=0…5 | Separate cases for each N: all N items, offset 0, no arrow/thumb glyph or ID; blank empty area and chrome retained. Seed dirty grid metadata and verify removed gutter fully cleared, content reclaims its column. |
| N=11 | Offsets 0…6 give positions `[0,1,2,3,3,4,5]`; exact strings `["╹  ","╻  "," ╹ "," ╻ "," ╻ ","  ╹","  ╻"]`. Offsets 3 and 4 show different content despite identical thumb; wheel/key/arrow moves must still report/recompose the content change. |
| Large list | N=1000, walk every offset 0…995 with monotone p, only six thumb positions, exact endpoints, every one-row content advance preserved. Use u32::MAX counts/offsets in pure arithmetic without allocating a huge list, and compare against independent u128 rational reference. |
| Rounding | Half-up ties at N=15/M=10, s=1 gives p=1 (0.5), s=3 gives p=2 (1.5); test just below/above ties, both endpoints, inverse ties (e.g. M=10,T=4,t=1 gives 3), no float or multiplication overflow. Synthetic inverse tie tests may call rounding helper; production T/M pairs remain canonical. |
| Invalid offsets/deltas | Stale u32::MAX clamped before draw and mutation; stale then -1 gives M-1. i32::MIN/MAX and repeated boundary input saturate. N=0 never underflows/divides by zero. `ensure_visible` empty-list/window behavior and generic explicit-window contract retained. |
| Growth/shrink | Reuse same dirty grid and state: 5→6, 6→5, 9 at s=4→6 at s=1→0 at s=0; 11→1000 retains numeric offset. Surviving IDs/check flags unchanged, no stale gutter ownership/shared border, unavailable arrow focus falls to OK and old grab fails validation. |
| Clipping | Width/height 0, width 1, heights 1…4, surplus height, long labels, right/bottom off-grid allocation and huge rects. Logical M always N-5; logical ▲/track/▼ positions never rescaled into shorter areas. Outside allocation/grid unchanged; no label writes into gutter/control/frame; no invisible target. |
| Pointer/keyboard/wheel | Both centered and profile panels: arrow press moves one; endpoint arrows refuse; wheel/trackpad direction and modal consumption unchanged; Tab/BackTab/Up/Down focus ring and minimal ensure-visible follow five rows; Space/Enter edit same item's draft; OK applies, Cancel/Escape discard and restore prior focus. No track-click action. |
| Drag/capture | Grab every full/partial occupied cell, press/no-motion no offset jump, negative displacement above track, both endpoints, anchored inverse and interior offset. At least two consecutive drag moves must survive own hit-map publications. Release/exit/focus loss/overlay close/layout or count change cancels; old IDs/grab cannot restart. Grab/Grabbing cursor falls back when bar disappears. |
| Inputs during capture | Wheel/trackpad input is consumed with no content change, scene fallthrough or cancellation. Space on a focused checkbox changes only its draft flag and retains a valid grab; focus movement within the visible window also retains it when routing is unchanged. Keyboard movement across the five-row window changes offset/routes and cancels the old grab before another pointer move can reuse it. Exercise both selection presentations. |
| Selection snapshot | Review the exact allowed snapshot diff against §21: first five items, canonical N=9 top track, downward arrow on the fifth row, cleared surplus rows and affected styles only. Run the existing `selection_panel_open` assertion with `INSTA_UPDATE=no`; report any independently existing setup/output failure separately. No assertion weakening or unrelated expected-output changes. |
| Identity/ink/hover | Duplicate profile names retain distinct IDs; scroll down/up preserves draft/applied state and order. Labels Text, arrows/partial/full thumb Action, frames Contour, blank track no thumb metadata. Exercise Light/Dark with Color on/off; hover does not alter track or ink. Existing modal and separator/tree routing regression tests stay valid. |

Current panels have fixed nine-entry arrays. Cover variable counts through real
`ScrollList` fixtures and the count-parameterized geometry/reconciliation/grab
validation helpers; use private unit tests for these helpers. Exercise the
actual nine-item generic/profile panels end to end. Do not add public setters
or implement list editing merely to inject test data. For profile writer table
coverage, the internal helper accepts borrowed `(WidgetId, &str)` rows, not just
labels/count. Tests supply N=0…11 with stable distinct IDs disjoint from surface,
up/down/thumb and OK/Cancel IDs. At N=10 and N=11, assert that every visible item
retains its supplied ID at every offset, and that only occupied gutter cells
carry the thumb ID; item cells can never be classified as thumb targets.
These fixtures exercise composition/hit ownership, not the nine-item panel's
fixed slot decoder. Production still supplies its existing nine IDs and labels.
Count-change capture tests can validate the private anchor
against changed helper geometry; controller tests cover actual geometry/close
cancellation and successive owned drag frames.

Acceptance is all cases above passing, no new workspace failures, exactly five
logical content rows and one-column/five-row gutter iff N>5, all small lists
fully visible, preserved empty chrome, unchanged identity/input semantics and
no production changes outside the allowlist. A previously failing baseline is
reported explicitly and is not called green. No automated check alone proves
physical-screen or OS-device acceptance.

## Verification handoff and actual Architect checks

Executor runs the baseline/final commands from `docs/agents/README.md`:
`cargo check --workspace --locked`, workspace fmt check (without formatting
unassigned files), Clippy all targets/features locked with warnings denied,
`INSTA_UPDATE=no cargo nextest run --workspace --all-features --locked`, and
`cargo test --workspace --doc --locked`. Use the documented cargo-test fallback
if nextest is unavailable. Run focused newell-ui overlay/shell, the existing
selection snapshot assertion, and newell-app controller cases first during
implementation. Existing dirty-file formatting
or other baseline failures must not trigger unrelated cleanup. No GPU/atlas
changes are planned, so ignored GPU tests need no new run for this task alone.

Pending manual acceptance: inspect both selection presentations with a mouse,
wheel/trackpad and keyboard; verify repeated thumb drag, endpoints, clipping,
no added rows and no scrollbar hover effects across existing Theme/Color modes.
Record visual/device checks as pending until actually performed.

For revision 1, Architect performed source/specification inspection, manual substitution of the
half-slot model for all supplied states, documentation consistency review and
scoped whitespace checking only. `git diff --check` passed for both edited
specifications. `git diff --no-index --check /dev/null` against the new plan
reported no whitespace diagnostics (exit 1 reflects the new-file difference).
The index remained empty. No builds, tests or application commands were
run; no production code/tests/index were changed in this stage. Old plan M2 UI
illustrations are historical and subordinate to the newly amended canonical
specification. No concrete unresolved architectural/product conflict remains.

Revision 2 actual checks: targeted source/specification inspection and plan
consistency review only; `git diff --no-index --check /dev/null` against this
untracked plan reported no whitespace diagnostics (exit 1 denotes a new-file
difference). The index remained empty. All three findings are addressed in the
plan; implementation, snapshot execution and input/device acceptance remain
unverified. No code, tests, specifications, snapshot files or index entries were
changed, and no builds, tests or application commands were run in this
correction. No unresolved product or architectural decision is introduced.
Stop here. Challenger and Executor require a separate human instruction.


## Executor implementation report — revision 2 — 2026-10-26

Executor / Codex. The human approved revision 2 and requested implementation on
main. The selection-box implementation and its automated checks are complete;
the workspace is not green, and physical-device acceptance remains pending.
No other role was started.

### Implemented behavior and files

- `crates/newell-ui/src/overlays/list.rs`: shared five-row list geometry and
  conditional one-column gutter; exact six-half-slot thumb calculation with
  half-up integer rounding; fully clipped glyph/ownership writes and cleared
  surplus/gutter cells; stale offsets clamped before signed deltas. Generic
  ScrollState retains its explicit-window API and gets an internal wide-delta
  path for overflow-safe wheel direction reversal.
- `crates/newell-ui/src/overlays/selection.rs`: both presentations consume the
  shared geometry and scrollbar writer. Profile composition accepts explicit
  borrowed item IDs/labels; nine-item production IDs remain unchanged. Offset,
  focus and arrows reconcile consistently. The anchored whole-row inverse
  preserves quantized offsets on press and exact endpoints. The opaque grab
  also records its panel identity and opening epoch, rejecting grabs after
  close/reopen or from another panel; epoch exhaustion safely disables new
  captures instead of recycling identity.
- `crates/newell-ui/src/overlays/mod.rs` and `crates/newell-ui/src/lib.rs`:
  re-export the Newell-owned opaque grab value.
- `crates/newell-ui/src/shell/mod.rs`: forward and validate anchors, reconcile
  selection state during sync, preserve modal routing, and negate wheel steps
  only after widening. No wheel action is admitted while controller capture
  consumes the event.
- `crates/newell-app/src/controller.rs`: certify only a matching selection
  drag's own publication so moving list hit targets do not cancel that drag;
  retain external invalidation, separator safeguards and current OS cursors.
- Inline tests in list.rs/selection.rs,
  `crates/newell-ui/src/overlays/tests.rs`,
  `crates/newell-ui/src/shell/tests.rs`, and
  `crates/newell-app/tests/it/controller.rs`: canonical states, fitting/empty
  lists, N=11 quantization, large/extreme counts, rounding, invalid offsets,
  growth/shrink, clipping, IDs, ink/theme/hover, arrows/keyboard/wheel and capture
  lifecycle. Old eight-row/full-cell assertions were replaced only where
  superseded by §21.
- `crates/newell-ui/tests/it/snapshots/it__snapshots__selection_panel_open.snap`:
  manually changed the approved list area to five rows and blank surplus rows,
  with automatic updates disabled. No unrelated snapshot cells were changed by
  this Executor. A later external tree-stroke update to the same snapshot was
  preserved. The existing snapshot assertion passes.

The title/footer, generic 30x12 and profile 31x11 footprints, selection identity,
draft/apply/cancel semantics, palette, glyph thickness and input gestures are
preserved. No dependency, renderer, model or kernel change was made for this
implementation. The current report is the only plan amendment in this stage;
the functional specification remains revision 2's read-only input.

### Starting state and concurrent external changes

Execution began on main at HEAD
`1eb4a9a4b9fb45a4bbfaa223e348c0e5e9a69f1b`, with the extensive dirty/untracked
inputs recorded above and an empty index. Other work changed sources and HEAD
while this task was running: HEAD subsequently became
`b0d86850f6c78f8dcff5b20c3fef3caee744fa56`, then
`a2d457292fd612de56651ddfd8b07f120c06fda3`. That external commit included much of
the in-progress selection implementation along with other UI/tree/camera work.
This Executor did not commit, stage, reset, restore, stash, create a branch or
worktree, or dispatch agents. The index remained empty when inspected at the
end; externally committed/modified work was preserved.

In particular, external UI Scale/tree edits temporarily caused compilation
failures and removed/reintroduced an untracked Interface_Exemple.txt. Those
sources were not repaired or restored by this task. The human's final
clarification supersedes the earlier mistaken answer: five UI Scale values are
intended, and Interface_Exemple.txt is no longer authoritative. Existing legacy
tests were executed as required, but that document was not used to define this
implementation. Current out-of-scope scale code/architecture still describe four
labels; that separate inconsistency is not resolved by selection scrolling.

Because the checkout changed during execution, the final global failures cannot
all be classified as the original baseline failures. No claim of an unchanged
workspace baseline or a green workspace is made.

### Checks actually run

All test runs used `INSTA_UPDATE=no`; no automatic snapshot regeneration was
used. Logs are local diagnostic text in `/tmp/newell-scroll-*.log`, not source
snapshots.

| Check | Initial baseline | Final observed result |
|---|---|---|
| `cargo check --workspace --locked` | Passed | Passed after the external missing-method compilation errors were resolved externally. Log: `/tmp/newell-scroll-resume-check.log`. |
| `cargo fmt --all -- --check` | Passed | Failed on externally changed tree/hover/reference code, including tree-related sections of controller.rs and shell/tests.rs. No unrelated formatting was applied. Log: `/tmp/newell-scroll-final-fmt2.log`. |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Passed | Passed after fixing one collapsible-if warning in the new clipping test. Log: `/tmp/newell-scroll-final-clippy2.log`. |
| `cargo nextest run --workspace --all-features --locked` | First run stopped after two glyph failures | Re-run with `--no-fail-fast`: 852 run, 818 passed, 34 failed, 13 skipped. Log: `/tmp/newell-scroll-final-workspace.log`. |
| Baseline nextest with `--no-fail-fast` | 831 run, 826 passed, 5 failed, 13 skipped | Baseline failures were three glyph-reference/atlas checks and two tree/reference comparisons. Log: `/tmp/newell-scroll-baseline-all.log`. |
| Focused overlay/selection/controller/profile tests and existing selection snapshot | Existing selection snapshot passed | **107 passed, 0 failed**, 678 filtered/skipped. Exact filter and command are below. Log: `/tmp/newell-scroll-final-selection.log`. |
| `cargo test --workspace --doc --locked` | Passed | Passed; all crates currently report zero doctests. Log: `/tmp/newell-scroll-final-doc.log`. |
| Scoped `git diff --check` | Documentation stage recorded separately | Passed for the implementation files and approved snapshot. |

Focused command:

```sh
INSTA_UPDATE=no cargo nextest run -p newell-ui -p newell-app --all-features --locked --no-fail-fast -E 'test(overlays::) | test(shell::tests::selection_) | test(shell::tests::profile_) | test(shell::tests::every_profile_) | test(shell::tests::centered_selection_) | test(controller::selection_) | test(snapshots::selection_panel_open)'
```

The final full suite's 34 failures concern glyph reference coverage, tree
selection/hover/playhead and navigation-width/drag contracts, their app-level
consequences, stale command-bar/reference expectations, and UI Scale tests.
The earlier broad targeted filter also admitted a tree-selection test; its
failure was retained in the report, not hidden by weakening any assertion.
The final focused filter isolates the approved selection-box paths; the complete
workspace run above still reports every failure. Every new selection/list test
and the existing selection snapshot passed in the complete run as well.

### Remaining limits and handoff

Manual pointer/wheel/trackpad/keyboard inspection and physical-screen visual
acceptance were not performed; they remain pending as specified above. Ignored
GPU tests were not run because no rendering/atlas code was changed by this task.
The unrelated global failures, formatting differences and five-versus-four scale
inconsistency remain for their respective work. This implementation is ready
for human inspection of the scoped result, with these verification limits.
No commit, external publication or subsequent role is authorized by this report.

## Executor follow-up — green workspace — 2026-10-26

The human explicitly expanded the Executor scope to repair the remaining global
failures and formatting, preserving existing work and the index on main, with
no commit or role transition. This supersedes the earlier selection-only write
allowlist for this follow-up. The preceding report remains a historical record
of that implementation's checks; its red-workspace conclusion is superseded by
the results below.

The human resolved the scale question by choosing the four currently documented
values (80/100/120/140); the earlier five-value clarification is superseded.
Interface_Exemple.txt remains retired and is not used as an expected UI reference.
No architecture or accepted ADR was edited by this follow-up.

### Repairs and preserved work

The checkout began at HEAD
`a2d457292fd612de56651ddfd8b07f120c06fda3` with an empty index and extensive
existing UI/tree/shared-selection/renderer edits. Work continued to change in
place during the first checks, including the selection migration from Copy
arrays/objects to identity-bearing dynamic data. The initial check compiled
production but the test build exposed stale field/Copy assumptions; after those
interfaces were reconciled, successive complete runs had 15, then 4, then 2
failures, then none. Existing implementation work was preserved throughout.

Targeted follow-up changes include:

- `crates/newell-ui/tests/it/action_boxes.rs`: clone the Sweep state for the
  before/after comparison instead of attempting to copy a dynamic ActionState.
- `crates/newell-ui/src/window.rs`: the unknown-monitor extreme-input test now
  checks the documented maximum factor 120 (k=6), rather than the removed 140
  factor (k=7). All four bounded scale states and their original factors remain
  covered by the menu tests.
- `crates/newell-render/tests/it/glyph.rs`: required-symbol scans exclude the
  retired shell example. Current-reference equality, fixed glyph count, font
  coverage and atlas coverage checks remain enforced; no atlas glyph was
  removed or added to conceal a failure.
- `docs/ui_design/Command_Bar.txt`: restore the README_UI §28 canonical Datum /
  Utilities header and their missing popup reference blocks, retaining the
  existing commands and fixed width. The implementation already used those
  higher-priority specified names.
- `docs/ui_design/Application_Menu_Bar.txt`, `docs/ui_design/README_UI.md`, and
  `crates/newell-ui/tests/it/menu_bar.rs`: retain the blank disabled arrow cell
  at the maximum scale state, correcting its one-cell-short textual reference.
  The selector never changes popup width at an endpoint.
- New test helper `crates/newell-ui/tests/support/reference_shell.rs`: read the
  existing golden base-shell glyph fixture, require exactly 30 rows of 82 cells,
  and explicitly verify/remove only the two documented blank rows for 28-row
  comparisons. It does not derive expected output from the implementation.
- `crates/newell-ui/tests/it/status.rs` and
  `crates/newell-ui/src/shell/tree_tests.rs`: retain full cell-by-cell shell /
  tree comparisons using that existing golden fixture, with no dependency on
  the retired document.
- `crates/newell-ui/src/shell/tests.rs`: use the canonical Extrude reference
  directly for its 16-row box; assert the playhead stroke through R-1, matching
  the current tree specification.
- `crates/newell-ui/tests/it/hover.rs`: retain the exact canonical Starting
  Offset hover assertion and unchanged ownership/ink checks without reading the
  retired shell example.
- Tree tests explicitly establish the expanded fixture when testing collapse,
  retain both logical anchor and structural gap location, route marker input
  to the intended semantic folder, and expect the documented selected hierarchy
  after Rename. Truncated labels do not substitute for semantic identities.
- `crates/newell-app/src/app.rs` test input helper: require a matching text cell
  to have current routing ownership before clicking, so an occluded underlying
  OK button cannot steal a modal test click. The multi-output selection fixture
  uses a name that fits its narrow pane; separate long-name round-trip coverage
  remains intact. The field transaction still asserts draft isolation, exact
  committed entity identity, unchanged command session and untouched unrelated
  selection/inputs.
- Rust formatting was applied across the workspace as explicitly requested.
  Existing shared-selection, tree, camera and renderer changes and their tests
  were retained. No lint was disabled, no test was skipped to hide a failure,
  and no snapshot regeneration command or automatic snapshot update was used.

### Final executed checks

| Command | Result | Diagnostic log |
|---|---|---|
| `cargo check --workspace --locked` | Passed | `/tmp/newell-green-final-check.log` |
| `cargo fmt --all -- --check` | Passed | `/tmp/newell-green-final-format.log` |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Passed | `/tmp/newell-green-final-clippy.log` |
| `INSTA_UPDATE=no cargo nextest run --workspace --all-features --locked --no-fail-fast` | **870 passed, 0 failed; 14 GPU tests excluded from this normal run** | `/tmp/newell-green-final-tests.log` |
| `cargo test --workspace --doc --locked` | Passed (currently zero doctests) | `/tmp/newell-green-final-doc.log` |
| `cargo test -p newell-render --locked -- --ignored` | **14 GPU tests passed**: 4 unit + 10 integration tests | `/tmp/newell-green-final-gpu.log` |
| `git diff --check` | Passed | Command output empty |

All 884 executed tests passed across the normal and explicit GPU runs. No
required check remains failing. GPU tests include selection brightness across
display modes and UI palette/role/rendering checks; they do not constitute
physical-device manual interaction or visual acceptance.

At final inspection HEAD was unchanged from this follow-up's starting HEAD and
the index was still empty. No commit, branch, worktree, source snapshot,
publication or subsequent role was created by this Executor. Existing dirty and
untracked work remains available for the human's integration.
