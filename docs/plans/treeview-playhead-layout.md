# Features-tree playhead, layout and row glyphs

Revision 1 — 2026-10-26 — Architect / Codex — approved with the human’s four-part Executor addendum.
Documentation amendments explicitly authorized; production implementation is
not authorized by this stage. Work is sequential in `/Users/martin/Desktop/Newell`
on `main`. Intended handoff: Luna with medium reasoning effort. No other agent
or role has been started.

## Scope and starting state

Formalize Features-only playhead geometry, collapse presentation, shared tree
width cap and exact hover/selected glyphs. Canonical behavior lives in
`docs/ui_design/README_UI.md` §26.1–§26.5; do not duplicate a competing contract
in code or old plans. No viewport redesign, selection semantics, kernel changes,
new dependencies, production code or tests in this documentation stage.

The starting index was empty. The checkout had extensive existing unstaged and
untracked work, including the M4.2 implementation, View > Color and workflow
amendments. Relevant modified inputs: `ARCHITECTURE.md`, `docs/ui_design/README_UI.md`,
`Interface_Elements.txt`, `Interface_Exemple.txt`, UI shell/tree widget, app
controller, render glyph atlas and their tests. Relevant untracked inputs:
`crates/newell-ui/src/shell/tree.rs`, `crates/newell-app/src/document.rs`,
`docs/plans/M4.2-model-bound-trees.md`, ADRs 0005/0006 and the View > Color plan.
All were read in place. No source snapshot, index operation, commit, branch,
worktree, application command, build or test was performed.

Inspected: entry/shared/Architect instructions, architecture including palette,
ADRs 0005/0006, canonical UI §§5.10–5.12, 18–26 and View palette rules,
`Interface_Elements.txt`, `Interface_Exemple.txt`, `Special_ASCII.txt`, M4.2 plan,
M4.3 backlog entry, M2 hover planning, tree composition/routing/drag source,
model Selection and app document/session glue, relevant UI/app tests and glyph
coverage tests/configuration. This is current-source inspection, not evidence
that this dirty checkout compiles or passes tests.

## Concrete findings and resolved conflicts

- `shell/mod.rs::activate_tree` currently replaces the cursor with
  AfterSubtree(folder) on collapse. Remove that branch: it loses the internal
  insertion anchor. `shell/tree.rs::set_playhead` reveals ancestors, and
  reconciliation unconditionally calls it; split explicit placement/reveal from
  retention so refresh cannot undo collapse.
- `compose.rs::draw_playhead` uses `W-(S+1)` dashes, leaving one terminal blank.
  Change to `W-S`; derive gap guides from boundary structure rather than copying
  the node row's trunk. Current entire-row PLAYHEAD_ID ownership must narrow to
  stroke cells, including the new inline variant.
- `nav_width(cols,wanted)` only enforces 14..cols/2; it has no content cap.
  `set_nav_width` currently clamps the stored wish to window width. Separate the
  stored content-clamped preference from temporary window clamping.
- `may_hover` excludes every tree id; composition does not report tree hover in
  its `lit` result. Row drawing has selected StyleId metadata but no selected
  hierarchy glyphs. Implement the canonical table without changing selection.
- Color normalization currently runs after hover and maps every Contour `╣`
  to `┤`. Apply tree right-border hover after normalization, emitting `╣` for
  Color off and `╡` for Color on, preserving frame role and routing metadata.
- Old M2/M4.2 exclusions and ADR 0006's generic `╰╴→╘ ` description conflict
  with the requested tree `╘╸`. The explicitly authorized architecture amendment
  now supersedes those tree-specific clauses. The accepted ADR is retained as
  historical evidence; its palette and non-tree hover rules remain applicable.
  M4.2 now carries an explicit supersession notice. No unresolved binding conflict.
- M4.3 revision 1 now exists in `docs/plans/M4.3-shared-selection.md`.
  This task uses current tree-local selection only. M4.3 later replaces that
  source with its entity/output membership projection while preserving §26.4
  glyphs; no second selection authority or M4.3 implementation is added here.


## Canonical decisions and arithmetic checkpoints

See README_UI §26 for the normative formulas and four-state glyph table.

- `O` includes both frame cells; `W=O-2` does not. At left border L, the stroke
  ends at `L+O-2`, never `L+O-1`. Proposed start is `L+1+(2*d+1)` for d>=1.
  At O=25, d=1 yields 20 dashes, d=2 yields 18. EndOfTree has no continuing
  child guide. A proposed endpoint is explicitly distinguished from supplied
  requirements because the original endpoint sentence was incomplete.
- Collapse retains both members of PlacedHistoryGap. Derive presentation as
  SeparateGap, InlineAt(visible collapsed ancestor), or Hidden. An internal gap
  after the last child is identified by the display node's strict ancestors,
  not solely a numeric operation boundary equal to the folder's end.
- For inline folder prefix P and full name length N, label budget is W-P-4;
  display min(N,budget) label cells, one blank, then at least three dashes.
  Do not render an inline stroke at all when W<P+5. This resolves impossible
  narrow/deep layouts without corrupting controls or weakening a shown stroke.
- Both complete trees, including collapsed descendants and headings, determine
  M. Maximum is M+2 interior, M+4 outer. Reference M=24 gives 26 interior and
  28 outer. Initial outer preference remains 25 where content permits.
  Content shrink clamps preference permanently; growth retains it. Window shrink
  is temporary. Expansion does not make the cap jump.
- Selected hierarchy wins over hover hierarchy; hover still marks right frame.
  This preserves selection under pointer movement without adding glyphs.
  Selection/focus/visibility/expansion remain distinct states.

## Ownership and interfaces

`newell-ui` owns semantic UI projection, presentation/layout and cell ownership;
`newell-app` owns pointer lifecycle, published generations and owned captures.
`newell-model` keeps existing history/insertion/Selection APIs and is untouched.
`newell-render` draws immutable glyph/ink cells in the same one batched draw;
no shaders, frame metrics or raster stroke widths change.

Keep public `HistoryGap`, `GapDisplayLocation`, `PlacedHistoryGap`, TreeTarget,
TreeHitToken and TreeIntent formats unchanged. No stored duplicate cursor.
Add only private helpers/types, with these responsibilities (names may follow
local conventions):

- `TreeRowGeometry { prefix_cells, label_start, natural_cells }`: use one
  checked/saturating geometry calculation in `widgets/tree.rs` for measurement
  and drawing. Compute from full label/controls and depth, never a clipped row.
  Widen depth arithmetic before multiplication; bound visible guide iteration
  by available cells even for u32::MAX depth.
- `PlayheadPresentation::{Hidden, Separate { depth, guides }, Inline { host }}`
  derived in `shell/tree.rs` from the unchanged placed pair and expansion.
  `host` is TreeTarget. Resolve ancestry using projection preorder/depth; no
  retained widget tree. Linear composition/projection work is allowed.
- `tree_content_outer_max()` measured from both node arrays/full labels, cached
  only if necessary to avoid repeated layout measurement in one composition.
  A projection-derived scalar cap in ShellState is acceptable; update it in
  install/reconcile before layout/slot publication. Do not cache truncated text.
- Extend private layout constructor/`nav_width` with the content cap, and use
  the same calculation in layout, resizing and accessors. Keep desired width in
  PanelWidths; apply the window ceiling only to effective layout.
- Private inline draw input carrying the host's stroke span, no new semantic
  row. Compute spans once for both glyphs and ownership. `Line::Node` may carry
  optional inline presentation, or derive it by semantic host comparison.
- Constant-time hover normalization: decode hit tree part/visible slot to Row
  WidgetId, validating eligibility against the current published map. Reuse this
  in pointer acquisition and post-compose validation. Current slot vector lookup
  is O(1); do not call a helper using `.position()` in the pointer hot path.
  Compare old/candidate semantic TreeTarget (including document) across
  publication before retaining hover; reused slot ids alone are insufficient.
  No new id range or second stored hover authority is needed.
- Have chrome/tree composition contribute `lit` to compose_hovered, and record
  the eligible hovered row's right-frame cell during row composition. Apply
  its border substitution after `single_contours` with metadata unchanged.
  Draw hierarchy glyphs from selected/hovered booleans directly. Do not extend
  the generic Action Box hover mapper to rewrite tree glyphs indiscriminately.

Name/Marker/Check retain their click owners. Only horizontal playhead stroke
cells receive PLAYHEAD_ID; guide/separator blanks are inert. Inline separator
blank stays Row, name includes any displayed ellipsis and remains Name. Root
collapse hides cursor; selection and focus restoration retain M4.2 behavior.
Hover is normalized for visuals only, never substituted into a click token.

Preserve `CapturedDrag::Separator` and `FrameCause::OwnedDrag`: matching moves
rebind bound generation while retaining capture id/origin/document. An inline
host no-op does not resolve AfterSubtree(host). Leaving it permits existing
Features drag snapping. Unrelated ownership/width/document/overlay changes
cancel through existing publication rules. Never start a playhead drag from
Elements, including empty space or a reused row slot.

## Exact implementation file scope

Production changes, only upon a later implementation request:

| File | Change |
|---|---|
| `crates/newell-ui/src/widgets/tree.rs` | Shared natural geometry, inline name budget, exact hierarchy states |
| `crates/newell-ui/src/shell/tree.rs` | Presentation/ancestry, cap derivation, retain-versus-reveal reconciliation |
| `crates/newell-ui/src/shell/data.rs` | Minimal private line/row presentation adjustment if required |
| `crates/newell-ui/src/shell/compose.rs` | Content-clamped layout, guides/stroke span, inline composition, hover lit and post-Color border |
| `crates/newell-ui/src/shell/mod.rs` | Cap preference lifecycle, collapse fix, stroke drag/no-op and O(1) hover normalization |
| `crates/newell-app/src/controller.rs` | Normalize hover validation; preserve captures and atomic publication |

Tests to modify/add:
`crates/newell-ui/src/widgets/tree.rs` unit module,
`crates/newell-ui/src/shell/{tests,tree_tests}.rs`,
`crates/newell-ui/tests/it/snapshots.rs`,
`crates/newell-ui/tests/it/snapshots/it__snapshots__{features_tree_collapsed,narrow_layout,wide_layout}.snap`,
`crates/newell-app/tests/it/{tree,hover,controller}.rs`, and
`crates/newell-render/tests/it/glyph.rs` for explicit required-glyph assertions.
Any new named tree-only snapshot must stay in that same snapshots directory.
Inspect expectations first; update only specification-backed tree differences.
The approved addendum permits specification-backed tree-cell changes in every
affected existing snapshot, including base_shell, menu and palette snapshots.
Preserve every unrelated cell; inspect each diff. Do not bulk regenerate.

`crates/newell-render/src/glyph_atlas.rs` requires no production change:
static inspection of UI_SYMBOLS and font cmap confirmed every required glyph,
including ╸/┃/┣/┗/╘/╞/╣/╡/…, already exists. Keep the existing all-reference
set and all-k coverage checks. No dependency, MSRV, font license or atlas-layout
change is required; project AGPL and current Fira Code license remain intact.
No model/kernel/document/app wiring source changes are planned.

## Ordered execution steps

1. Read this revision and amended canonical specs in the existing checkout;
   inspect new local changes. Record Executor baseline with automatic snapshot
   updates disabled using the shared workflow checks. Separate existing failures.
2. Add shared natural row geometry and width-cap derivation. Wire cap to every
   layout path, stored preference and projection/label reconciliation. Verify
   fixed sample arithmetic before changing snapshots.
3. Remove collapse anchor relocation and separate retained refresh from explicit
   cursor reveal. Derive visible collapsed host and boundary guide continuation.
   Render one separate gap or one inline host, never both.
4. Use exact stroke spans for rendering/hits, inline ellipsis and narrow fallback.
   Preserve root collapse, empty-history End and existing admitted-gap policy.
5. Compose selected/hovered hierarchy from semantic row state; normalize hover
   via slots, report lit and apply right-frame glyph after Color mapping. Ensure
   hover alone leaves all routing metadata and generations identical.
6. Exercise owned-drag continuation from inline and ordinary stroke spans;
   keep host-row movements inert and preserve existing capture cancellations.
7. Add focused cases below; migrate only affected reference assertions/snapshots.
   Run focused checks, then required shared workspace verification. Do not claim
   manual visual results from automated coverage. Record pending manual checks.

## Test matrix and acceptance criteria

| Area | Required deterministic assertions |
|---|---|
| Endpoints | At O=25, d=1/2 start offsets 3/5, counts 20/18, last stroke R-1; actual border R unchanged; nonzero L and several widths |
| Hierarchy | Depth 1/2/3, ending versus continuing ancestors, before/after folder boundary, last-child internal gap and EndOfTree; no horizontal/vertical cell collision |
| Collapse | Before operation, after internal last child and End-anchored internal placement; collapse leaves exact insertion/display pair and document/event count unchanged; expand restores location |
| Nesting | Collapse inner then outer, expand outer then inner; nearest visible collapsed host shown once; root collapse hides it; scroll away/back retains anchor; refresh does not auto-expand it |
| Inline allocation | W=P+5 yields one-cell ellipsis when required, blank, exactly three dashes; shorter/exact/long names; full-name preservation and one-cell ellipsis; W=P+4 suppresses all stroke/hits |
| Width | Complete Features and Elements prefixes/controls/counts/headings and hidden descendants determine M; reference M=24/cap=28 outer; padding two interior cells; no border/filler/hover inflation |
| Width lifecycle | Drag beyond cap is stationary; smaller user width retained on label growth; content shrink clamps preference; rename, count digit transition, insert/remove/reparent and document replacement update cap before map publication |
| Small layout | Empty history/entities, collapsed roots, hidden panel, zero synthetic rows/cols, depth u32::MAX; half-window ceiling, 14 outer minimum where feasible, temporary shrink/restore; no overflow or leaked hits |
| Glyph states | All slots in §26.4, last leaf/branch, checkbox row, root, both trees and all palettes; exact ╘╸, selected-only row guides, unchanged labels/handles and unrelated frames/rows |
| Restoration | Enter via each Row/Name/Marker/Check part; exit, deselect while hovered, focus-only, check/expand without selection changes; same normalized target; correct lit result; no redraw on same-row motion |
| Hover lifecycle | Slot reuse after scroll, deletion, menus/modals, resize and Color toggle; stale target cleared; no acquisition without motion; Color right border ╡ rather than ┤ |
| Hit contracts | Normal/selected/hover combinations have identical widget/shared-border metadata; inline name only renames, handle only expands, square only changes visibility; separator blank never drags; border retains resize routing |
| Drag | Press inline clears Rename sequence, no selection mutation; own host movement preserves anchor; two or more moves across map generations keep capture id/origin and current bound generation; release/loss/overlay/scale/document changes cancel |
| Elements absence | All expansion/scroll/selection/width/empty states contain no PLAYHEAD_ID or playhead line/action; pressing any Elements row/blank cannot initiate insertion; captured Features drag retains its existing clamping behavior |
| Glyph coverage | Explicit required characters present in font, canonical set and atlas at every k=2…7; keep existing unknown-glyph fallback and reference coverage assertions |

Acceptance requires every applicable automated check green (or accurately
reported pre-existing failure), all canonical formulas/tables covered, unchanged
insertion/history semantics except removal of collapse relocation, full names
and identities preserved, O(1) pointer routing and one UI draw unchanged.
Selected row semantics remain exactly those supplied by existing state.

## Separate manual visual checks

On physical displays at k=2 and a Retina scale, then representative remaining
k values: inspect guide/stroke separation, right-edge contact, regular and heavy
hierarchy joints, exact ╘╸ tail, inline ellipsis with three dashes, both themes
and Color states. Check Text guides/strokes versus Contour border and Action
controls by eye. Confirm cursor Grab/Grabbing only over the stroke, Rename on
name, expansion on handle, divider resize and continuous drag across two or
more moves. Resize to the content cap and narrow window; rename longest rows;
collapse/expand nested folders and verify the same next insertion location.
Record results separately; do not infer visual acceptance from glyph coverage.

## Unresolved product decisions

None blocks this scope. M4.3's future mapping of entities/features/folders and
modifier/multiple-selection behavior remains owned by that milestone; this plan
consumes existing row selection and defines only its visual representation.
The proposed depth endpoint and narrow/deep suppression are explicit routine
choices of this revision, reviewable with the implementation plan.

## Documentation-stage verification and stop

Architect handoff reconciliation: the current checkout retained the architecture,
README_UI and M4.2 notice, but Interface_Elements.txt still contained the older
artwork and Interface_Exemple.txt was absent. The Executor addendum authorizes
restoring the latter from Git and reconciling both; the earlier delivery record
must not be treated as evidence those reference edits were present at execution.
The human's explicit authorization overrides Architect's usual plan-only write
scope for these targeted amendments. Accepted ADRs and all production/tests
remain untouched. Verification is limited to documentation consistency, static
font/atlas-set inspection and changed-document whitespace. Tracked-document `git diff --check` passed;
the new plan has no trailing whitespace and all newly framed tree reference
rows have matching widths. The pre-existing Markdown hard breaks in the
untracked M4.2 plan metadata are retained. No build, test,
rasterization or application was run. Implementation and manual checks above
remain pending. Stop after this Architect stage; no automatic next role.


## Approved Executor addendum and execution record

The human approved revision 1 with four corrections: recover/reconcile references,
allow targeted tree changes in all affected existing snapshots, coordinate M4.3,
and validate semantic hover across publication. No further review is required.
HEAD at entry: b0d86850f6c78f8dcff5b20c3fef3caee744fa56; index empty; extensive
existing unstaged/untracked inputs preserved. UI-scale work has changed since
Architect/Challenger inspection; it is outside this implementation.

Reference recovery: 42303b2 deleted only the 28-line complete reference, with no
replacement or reason beyond its deletion message. Recover its exact parent
version (same reference as 022f335), then amend only the terminal playhead blank.
The result remains 82×28; unrelated historical labels/menu artwork stay intact.

Baseline (INSTA_UPDATE=no): cargo check --workspace --locked passed; workspace
format check failed on existing formatting; Clippy and nextest failed to compile
because of the absent reference and an inherited [u32;4] versus five-value
assertion in newell-ui/src/window.rs:573. Workspace doc tests passed. Logs:
/tmp/newell-tree-baseline-{0,1,2,3,4}.log. This is not a green baseline.
