# View > Color — implementation plan (revision 4)

**Role/provider:** Architect / Codex. **Date:** 2026-10-26.
**Checkout:** `/Users/martin/Desktop/Newell`, `main`.
**Status:** the human approved revision 3's functional design and ADR 0006's
architectural/UI-specification amendments in the 2026-10-26 resume instruction.
Those documentation amendments are applied and ADR 0006 is Accepted. Revision 4
adapts that approved behavior to the current M4.2 implementation. This stage is
complete with documentation; production implementation has not been requested.
No further Challenger review is a prerequisite. The human controls the next
implementation request and role transition.

## 1. Starting state and authority

Work sequentially in this existing checkout on main. Preserve existing staged,
unstaged and untracked work and the Git index. Do not pull, commit, push, create
branches/worktrees/checkouts, or create source snapshots or archives. HEAD was
`1eb4a9a4b9fb45a4bbfaa223e348c0e5e9a69f1b` when inspected; main was behind its
remote by six commits. This is context, not a request to synchronize the remote.
The index had no staged changes. Current working files, including M4.2, are the
implementation inputs; the historical commit alone does not describe them.

The reviewed migration inputs were read only from the existing
`/Users/martin/.codex/worktrees/view-color-architect/Newell` directory:

- Plan revision 3 SHA-256 `12cdcb4a091a71c7a86eefaac95eb81769205fede53f302efe0cf612db0b9500`.
- Draft ADR 0006 SHA-256 `02e119643456c72c3335b565c423e73e8b3db379b7b8c8404c5f7bf8816cba42`.

Both matched on migration. Their 2026-10-23 source hashes are historical review
provenance, not execution gates. No historical source was copied back. The
maintained versions are this plan and `docs/adr/0006-view-color-semantic-palette.md`
in main. All branch/worktree/snapshot prerequisites in revision 3 are superseded.

Relevant existing changes inspected:

- `ARCHITECTURE.md`, `AGENTS.md`, `docs/agents/{README,architect}.md`: sequential
  main workflow, bounded human-controlled stages and current rendering/navigation.
- `docs/ui_design/README_UI.md`, `Application_Menu_Bar.txt`, ADRs 0003/0005:
  existing theme/menu, hover, caret and M4.2 live/reference-tree distinctions.
- `crates/newell-app/src/{app,controller,document}.rs`: current DocumentSession,
  projection publication/retry, tree intent validation, capture and renderer upload.
  `document.rs` is untracked existing work, not a file to recreate.
- `crates/newell-ui/src/shell/{data,mod,compose,tree,tests,tree_tests}.rs`,
  `widgets/tree.rs`, overlays and their tests: dynamic trees and full-length Rename.
  `shell/tree.rs` is untracked existing work, not a mock-tree replacement target.
- Existing renderer/theme/display-mode/navigation changes and app/UI integration
  tests remain in place. Other user changes are outside this increment.

Source precedence remains ARCHITECTURE.md → README_UI.md → `.txt` references →
plan. Accepted ADR 0006 and architecture §§3.2–3.6 now authorize Color explicitly;
menu artwork is no longer the sole mention. ADR 0005 remains verbatim historical
scope and preserves Dark startup. This Architect stage writes only this plan,
ADR 0006, ARCHITECTURE.md, README_UI.md and the relevant ADR 0003 clauses under the
human's explicit exception. It runs no builds, tests or application.

## 2. Accepted behavior and boundaries

Use the exact palette in ADR 0006 / architecture §3.6:

| Theme / Color | Background | Text | Contour | Action |
|---|---|---|---|---|
| Dark / unchecked | `#000000` | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` |
| Light / unchecked | `#FFFFFF` | `#000000` | `#000000` | `#000000` |
| Dark / checked | `#071521` | `#FFFFFF` | `#41B4E5` | `#FFA24A` |
| Light / checked | `#FFFFFF` | `#111827` | `#0969A0` | `#9A4D00` |

Color is per-window presentation, initially false, independent of manually
selected Theme. It is not document data, inherited preference, OS appearance,
interaction-state styling or persistence work. Every new shell starts Dark +
unchecked. Pointer and Enter/Space toggle its View checkbox, leave the popup open
and keep that row focused. The existing Theme item retains its own activation.

Unchecked preserves monochrome pixels and glyph geometry exactly, including
original double frames and hover; only the approved new View-menu row is added.
Checked uses semantic roles and the eleven same-cell contour mappings in §4.4.
Hover still substitutes Unicode without changing a role, palette, brightness,
alpha, animation or shader behavior. Viewport clear follows the chosen background;
scene display modes/materials/selection/depth/camera do not change.

The requested blue-contour/orange-affordance allocation is accepted for its clear
structural/action hierarchy with high-contrast ordinary text. ADR 0006 retains
the two quieter alternatives and rationale: blue only on outer frames loses the
requested structural consistency; orange only on focus/hover makes it a state
indicator and conflicts with hover invariance. The btop reference supplies colour
hierarchy only. No btop layout/features, logos, new font/dependency, retained tree,
model/kernel changes or additional render pass are in scope.

## 3. Current M4.2 interfaces to preserve

### 3.1 Document and projection ownership

`newell-app/src/document.rs::DocumentSession` owns the Document, instance identity,
revision and Changed/Replaced notifications. `project_document` emits model-agnostic
`ModelTreeProjection`. `ProjectionSync::poll` drains notifications without treating
that as acknowledgment, retains pending work on failure, retries once after 100 ms,
then suspends until its existing rearm triggers. Acknowledgment occurs only after
successful controller publication. Color/Theme must neither publish a document
notification nor increment its revision, replace its identity, clear pending work,
change deadlines or bypass publication. Their ordinary UserActivation may rearm an
already pending projection through the existing path; preserve that behavior.

`ShellState` owns projection-derived nodes, local expansion/selection/focus/scroll
and `PlacedHistoryGap`. `shell/tree.rs::set_tree_projection` validates and reconciles
same-document changes by semantic identity; replacement resets tree presentation
and Rename appropriately. It must retain window Theme/Color, as it currently
retains other View choices. Add no color fields to projection/target/token/intent,
DocumentSession, model, tree node IDs or history-gap types.

`ShellState::new_application` supplies live startup; `ShellState::new` and
`data::reference_trees` support the historical reference fixture. Keep live
Bodies/Surfaces/Construct counts, empty/live End cursor, folder preorder, output
order and names. Do not reintroduce fixed mock slots or `set_model_tree_labels`.
No document.rs, model crate or shell/tree.rs production change is needed.

### 3.2 Actual tree writers

`shell/compose.rs::tree_lines` uses current derived nodes, expansion and gap display
location, then `draw_tree` scrolls/clips and constructs `TreeRow` with dynamic
`TreeRowIds` for Row/Marker/Check/Name. `widgets/tree.rs::draw_tree_row` already writes
separate guide, marker, checkbox and name pieces. Attach roles to those writes:
Text for guides/connectors, full names, counts, ellipsis and row blanks; Action
for the entire `[-]`/`[+]` marker and actual visibility checkbox. A name containing
brackets, `│`, `☰` or `+` is still Text. `draw_playhead` uses Text for its vertical
continuations and horizontal dashes, even though it has a draggable WidgetId.
Preserve all depths/trunks, clipping, bounded guide generation, slots and owners.

The panel's `draw_border` cells are Contour. They are distinct from the inner
Text guides, including at the narrow pane width and beside the shared Features/
Elements divider. Do not recolour by the tree row's ID, style or visible glyph.

### 3.3 Published frames, generations, intents and capture

`ShellController` owns `shell`, `published_shell`, `grid`, `generation`,
`CapturedDrag`, `RenameSession`, hover and pending TreeIntent. Its `publish`
compares `same_tree_routes`, grid dimensions and per-cell `(widget_id, shared_border)`.
Keep precisely that routing-based comparison: neither new InkRole nor glyph
changes belong in the generation key. Do not replace it with CellGrid equality.
Presentation-only publication must preserve generation and a valid capture.

Menu opening/closing, overlays, actual routing/layout/identity changes still
invalidate stale hits and cancel unowned captures as now. Successful separator/
playhead moves still use OwnedDrag publication and rebind only their own capture.
Keep Rename's origin/bound generation, source-name check and certified completion
at its own close. Keep failure rollback to `published_shell`, intent consumption,
document-match checks, pointer exit and scale/resize cancellation. No new capture
kind or Color generation counter is needed. Opening View already changes routing;
do not claim toggling Color can bypass a modal or survive an unrelated menu change.

### 3.4 Application and renderer path

Keep `App::finish_shell_input`: validated TreeIntent → existing session edit →
`sync_extrude_command` → poll projection → merge Update → `apply`.
Keep the consumed Extrude confirmation latch and forward-reference validation.
`App::scroll_shell_panel` must continue applying a handled wheel Update before
returning, including panel ownership at a scroll limit. Color must not route
around either path or alter navigation admission/capture.

`App::apply` calls `sync_view_choices`, then uploads `shell.grid()` when
`Update.grid` is true and updates viewport/occlusions/camera as now.
`apply_rescale` prepares an atlas plus a complete controller grid, commits metrics
and retains projection rearming. All composition paths (ordinary refresh, hover,
projection refresh, resize and prepared scale) must produce the same role-bearing
Color presentation via `ShellState::compose_hovered`; do not postprocess only an
app upload copy, which would diverge from the hit-tested published grid.

## 4. Implementation contract

### 4.1 Per-window state and menu semantics

In `shell/data.rs`, keep the existing ViewMock name and five visibility checks.
Add `color_enabled: bool`, default false, `ACT_VIEW_COLOR = 224`, and a separate
MenuCheckbox immediately after Theme. `view_entries` grows from 12 to 13; Theme
remains entry index 3, Color becomes index 4. Preserve semantic action IDs 210–214
and all their visibility effects; entry-derived WidgetIds below the inserted row
naturally move with that documented row insertion. Do not renumber action IDs.

Import ACT_VIEW_COLOR in shell/mod.rs and expose `ShellState::color_enabled()`.
In `apply_menu`, `MenuResult::Toggled(action)` first handles exact action 224 by
inverting only the Color flag and returning true. Only other toggles reach the
existing `checked_sub(ACT_VIEW_CHECK)`/visibility-array lookup. Do not put Color
in `checks`, where it would shift Tree/Task/Status/NavCube/Grid behavior.
`MenuState` already keeps checkbox rows open/focused; keep that route.
Theme's existing `run_action` reopens View with index 3 focused; keep it and
preserve Color across Theme switches.

Add `MenuItem::action_offsets: &'a [u32]`, empty for ordinary items. The View
builder gives Theme `&[6, 8, 16]`, relative to its exact 17-character label:
`Theme [ ◑ Dark  ]` or `Theme [ ◐ Light ]`. Generic `draw_menu_popup` writes the
same Text label, then changes only the role of those already drawn cells to
Action, clipped to the label/interior/grid. Add a small Canvas role-stamp helper
if needed; it must not write content or routing or reach the popup border.
Label `Theme`, value Dark/Light and spaces remain Text; outer popup delimiters
are Contour. Do not classify by action ID or glyph inside the generic renderer.
Menu command captions and mnemonic spelling such as `[N]ormal to` remain Text.

### 4.2 Flat-cell representation and write propagation

Define `#[repr(u8)] InkRole { Text = 0, Contour = 1, Action = 2 }` in cell.rs,
derive Copy/Eq/Default with Text default, and provide an explicit GPU index
conversion. Add `Cell::ink: InkRole`; export InkRole from newell-ui/lib.rs.
Keep StyleId as the logical-state marker with its current numeric values.
Role is independent of style, WidgetId and SharedBorder; it exists also when
Color is off, but all roles then resolve to the same monochrome ink.

In widgets/mod.rs keep existing `cell`, `owned_blank` and Canvas::text signatures
as Text defaults. Add explicit `cell_with_ink` and `Canvas::text_with_ink` helpers
(or identically scoped wrappers) that use the same bounds, full-cell writes and
return offsets. `shared` constructs Contour dividers. Canvas::put/fill/fill_col and
ActionBox scratch-grid copies already copy whole Cell: retain that including ink.
Defaults/clears must reset ink to Text; no stale role survives an overwritten cell.
Content-only hover writes preserve ink. Border merge writes set Contour while
retaining the exact current junction algorithm and metadata behavior.

Update full Cell literals in scoped source/unit/integration tests. Update grid.rs
unit assertions for default/clear ink, without changing grid indexing/production
behavior. Correct obsolete monochrome-only source comments in touched files;
StyleId remains invisible for every interaction state.

### 4.3 Component roles, mixed rows and ownership

| Writer / content | Ink assignment and required preservation |
|---|---|
| `draw_border`, `draw_separator`, panel/frame joins, shared dividers | Contour, including merged corners/T/cross joins and split-routing cells. Frame stroke owns a true frame/tree intersection; next tree cell stays Text. Preserve all routing halves. |
| `draw_label`, `draw_button`, `draw_checkbox`, `draw_cyclic` | Labels/button captions/value text Text; checkbox glyph and selector arrows Action; padding Text/Empty. Whole-row ownership never selects ink. |
| Text/numeric edit fields | Ordinary brackets Contour; numeric direction sign Action, adjacent `\|` Contour, formula `☰` Action; value/unit/caret Text. A closing bracket remains Contour even if owned by the formula WidgetId. |
| Selection fields | Both brackets and any explicit `☰` trigger Action; Select…/none/count/value Text. Propagate caller semantic kind for both empty and nonempty selection rather than recognizing the text or bracket glyph. |
| Dynamic tree rows and playhead | As §3.2; no role from node ordinal, raw identity, name text, selected/focused/expanded style, or draggable ID. |
| Window controls / NavCube | Entire fixed `[-] [□] [X]` control glyph groups Action; inert NavCube outline Contour and its overwritten title Text. Status/progress/viewport labels Text. |
| List/selection overlay | Check glyphs, explicit selection list `☰`/`×`, scrollbar arrows and draggable thumb affordances Action; names/counts and inert annotations Text; list frames/separators Contour. Ownership, disabled behavior, scrolling and thumb capture stay current. |

In action_box.rs, add ink to `Item::Text` and a `Row::text_with_ink`/`Row::contour`
helper while retaining `Row::text` as Text. Split each literal builder into
structural and Text spans. Example `│ │ Depth` is outer Contour `│` followed by
Text ` │ Depth`; the inner `│` is a hierarchy guide. A top border's title letters
are Text between Contour spans. Update create_box/extrude/revolve/loft/sweep/boolean,
including formatted prefix helpers and active-segment topology. Row::end writes
Contour. Do not parse arbitrary literal strings to discover frame glyphs.

Currently Row::select_or represents nonempty selection with Item::Numeric and
empty selection with Item::TextField. Add explicit `FieldKind { Edit, Selection }`
to those internal items and to role-aware internal field helpers. Row::numeric
supplies Edit; both select_or branches supply Selection. Public draw_text_field/
draw_numeric_field keep their existing signatures as Edit wrappers. Keep numeric
editing/caret/prefix logic and field/formula owner slots unchanged. The semantic
kind changes role assignment only; it does not turn a selection count into editing.

Mixed-title paths require specific writes, not whole-row Contour assignment:

- `menu/draw.rs::draw_popup_frame`: its current closure changes only content;
  corner/edge/junction writes must establish Contour, while each clipped title
  letter over the top edge becomes Text. Preserve existing owner/style/routing.
- `RenameModal::draw_art` and `FormulaEditor::draw_art`: emit `╭─`, title, fill
  and `╮` as separate spans. Edges, separator rows, bottom frame, Formula inner
  outline and button divider are Contour; field text/caret/button captions Text.
  Keep M4.2's full-length rename text and visible caret window, never truncate
  stored names or change source-name validation.
- `SelectionPanel::draw_in_action_box`: split `╭─{title}{header_fill}╮` at its
  current offsets into Contour `╭─`, Text title, Contour fill and `╮`. Use the same
  style, `Some(self.surface())`, existing clipped Canvas and full-frame overwrite.
  Bottom `╰…╯`, nested list frame, structural separator before actions and shared
  footer divider are Contour. Keep the generic SelectionPanel::draw path and its
  content-title Text rule too. Preserve current hover_action_buttons boundaries.
- `overlays/mod.rs::draw_frame`: clear full cells to Empty/Text, draw Contour
  frame, then content/title writers set their own roles. Modal input masking
  outside the modal changes only routing metadata; it must not recolour still
  visible underlying cells. A covered cell replaces glyph/role together.

Clipping never changes which cells the component owns. No title span may escape
its current Canvas or overwrite the far corner just because the text is longer.
Unknown text falls back to `?` at render time with the source role intact.

### 4.4 Presentation pass and hover ordering

At the end of `ShellState::compose_hovered`, after chrome, Action Box and topmost
overlay composition with their existing hover substitutions, run a small pass
only when `self.color_enabled()` is true. For cells with ink Contour, replace:

| Double | Single | Double | Single | Double | Single |
|---|---|---|---|---|---|
| `═` | `─` | `║` | `│` | `╔` | `┌` |
| `╗` | `┐` | `╚` | `└` | `╝` | `┘` |
| `╠` | `├` | `╣` | `┤` | `╦` | `┬` |
| `╩` | `┴` | `╬` | `┼` | — | — |

Only content changes. Role, style, coordinates, WidgetId and SharedBorder remain
identical. Single/rounded corners, `┆`, Text guides, Action cells and `╞/╡/╘`
remain unchanged. Unchecked skips the pass completely. Keep this helper in
shell/compose.rs; no new module/framework or role inference from glyphs.

Order: semantic writes → existing Unicode hover → map remaining double Contour
cells → publish/upload. This preserves current hover eligibility: a double `║`
is not a new hover slot merely because Color later maps it to `│`. The existing
slots `│`/`┆` and two-cell `╰╴` still transform as specified. `╘` inherits the old
`╰` role; the following blank retains the old `╴` metadata. No stored hover color,
new pointer scan or shader hover input. The linear composition pass does not
change O(1) hit testing.

### 4.5 Palette, GPU transfer and background

In render/ui/theme.rs keep Theme, its Dark default and monochrome foreground/
background methods for compatibility. Add `UiPalette` (Copy/Eq) containing Theme
and `color_enabled`, with constructor/default/accessors and exact sRGB normalized
`foreground(InkRole)` / `background()` from the accepted table. Default is Dark /
false. Export through ui/mod.rs and render/lib.rs. No per-cell RGB or mutable global.

Replace stored Theme in UiOverlay/Renderer with UiPalette; add set_palette to both.
Add `UiPalette::with_theme(self, Theme) -> Self`, preserving Color, and use it
in retained set_theme wrappers for both renderer and overlay. App uses set_palette, not a later set_theme that could
reset Color. Palette changes rewrite the small uniform only and never rebuild
an atlas. Color's glyph changes are uploaded through Update.grid separately.

`GpuCell` becomes `{ glyph: u32, ink: u32 }`, serialized explicitly little-endian
in that order, 8 bytes per instance. Ink is the enum index; StyleId/WidgetId/
SharedBorder remain CPU-only. Preserve EMPTY_GLYPH and unknown glyph fallback.
Pipeline attributes are locations 0 and 1 Uint32 at byte offsets 0 and 4, stride 8.
Pass ink as a flat u32 from vertex to fragment (new free output location 3).
Any raw GPU index outside 0..3 resolves to Text (0), not to the nearest high role.

Increase UNIFORM_BYTES from 64 to 96, including bind-layout min_binding_size and
buffer size. Retain current serialized header; offsets are:

| Byte offset | Value |
|---|---|
| 0, 8, 16, 24 | existing vec2 viewport/cell/glyph/atlas floats |
| 32, 36 | cols u32, srgb_target u32 |
| 40–47 | 8 zero padding bytes |
| 48, 64, 80 | Text, Contour, Action vec4<f32>, each alpha slot 1 |

WGSL uses three aligned vec4 inks (or fixed array with stride 16) after that
header and selects the role in the current fragment shader. Retain coverage-only
atlas, nearest text sampler, ALPHA_BLENDING and six-vertex instanced quad. Empty
cells remain transparent; zero-sized grid draws zero times; every nonempty valid
grid uses exactly one UI draw. Palette changes do not add textures/passes or logos.

The shader starts with normalized sRGB source hex. For an sRGB target it applies
existing srgb_to_linear before output; for a linear Unorm target it outputs the
normalized components as today. Renderer clear uses the same source background
and target-format conversion (`config.format.is_srgb()`), alpha 1. Do not send
navy hex directly as a linear clear on an sRGB surface. Retain coverage blending
in the target's existing space; antialiased pixels need not equal source hex.

### 4.6 App synchronization and uploads

In app.rs add one small pure `ui_palette(&ShellState) -> UiPalette` mapping used
by `App::sync_view_choices`; it maps ThemeChoice and shell.color_enabled only.
Call renderer.set_palette there, alongside existing separate shading/projection/
scale/camera handling. Keep apply's actual Update.grid branch. There is no need
for a second helper merely returning Update.grid or for a new generic renderer
trait. Existing app unit tests can reach App and `last_ui_publication` directly.

Color toggles recompose and upload. Both Theme directions with View open also
recompose/upload, because the Theme string changes. Do not suppress either grid
update after writing the palette. A uniform-only Theme update is safe only with
an actually unchanged glyph grid; no optimization of controller refresh is
required in this increment. Preserve palette through metrics/atlas activation,
resize, document refresh/replacement and renderer startup before first visibility.
No rendering, scheduling, projection-retry or navigation rewrite is needed.

## 5. Exact future implementation file scope

All paths below are relative to `/Users/martin/Desktop/Newell`; grouped filenames
in a directory are exhaustive, not permission to scan/change other files.

| Directory | Production files and purpose |
|---|---|
| `crates/newell-ui/src/` | `cell.rs` (InkRole/Cell), `style.rs` (clarify logical style comment only), `lib.rs` (export InkRole), `action_box.rs`, `create_box.rs`, `extrude.rs`, `revolve.rs`, `loft.rs`, `sweep.rs`, `boolean.rs` (semantic spans/field kind) |
| `crates/newell-ui/src/widgets/` | `mod.rs` (role helpers/docs), `button.rs`, `field.rs`, `selector.rs`, `tree.rs` (source role assignments); `hover.rs` role-preservation assertions/docs only unless fixing an actual metadata loss |
| `crates/newell-ui/src/layout/` | `border.rs` (Contour assignment at current merge writes) |
| `crates/newell-ui/src/menu/` | `mod.rs` (semantic offsets), `draw.rs` (role-bearing labels/titles/separators) |
| `crates/newell-ui/src/shell/` | `data.rs` (View flag/menu metadata), `mod.rs` (toggle/getter), `compose.rs` (tree/chrome roles and final presentation pass) |
| `crates/newell-ui/src/overlays/` | `mod.rs`, `list.rs`, `selection.rs`, `formula.rs`, `rename.rs` (frame/control/title roles, full-cell propagation) |
| `crates/newell-render/src/ui/` | `mod.rs`, `theme.rs`, `gpu_cell.rs`, `pipeline.rs`, `shader.wgsl` (palette/export/transfer/lookup) |
| `crates/newell-render/src/` | `renderer.rs` (palette/converted clear), `lib.rs` (UiPalette re-export) |
| `crates/newell-app/src/` | `app.rs` (palette mapping and existing upload path only) |

Test-only edits may also use `crates/newell-ui/src/grid.rs`,
`crates/newell-ui/src/menu/tests.rs`,
`crates/newell-ui/src/shell/{tests,tree_tests}.rs`,
`crates/newell-ui/src/overlays/tests.rs`,
`crates/newell-app/src/{controller,document}.rs` inside existing cfg(test) modules,
and the integration files in §7. No production edits to document.rs,
controller.rs, shell/tree.rs, model/kernel crates, navigation.rs, scene.rs or
scene shaders. No Cargo/font/reference-artwork change. Production scope above
includes updating affected existing full Cell/MenuItem literals in their unit
tests. Do not touch irrelevant files solely because they are allowed.

Existing `.snap` files remain byte-for-byte. There is currently no View-menu
snapshot to regenerate. Add only the new snapshots named in §7; if an existing
snapshot unexpectedly changes, diagnose that before any update. No blanket
snapshot regeneration. Binding documentation is already amended in this stage;
Executor does not reopen or rewrite the decision.

## 6. Ordered implementation steps

1. Inspect current scoped changes and read this revision plus accepted ADR 0006.
   Record the repository's verification baseline with automatic test-snapshot
   updates disabled, as required by docs/agents/README.md. Preserve current M4.2
   behavior; record existing failures separately. No source snapshot/hash gate.
2. Add InkRole, Cell ink, default/full-copy helpers and compile-required literal
   fields. Add role/clear/copy/border tests. Keep all styles and owners unchanged.
3. Annotate primitive, tree, Action Box and overlay writers in §4.3, including
   field semantic kind, current dynamic rows and all mixed-title paths. Add
   targeted role/owner/clipping/hover assertions before adding Color glyph mapping.
4. Add per-window state, dedicated action 224 and Theme metadata. Add menu state,
   pointer/keyboard/visibility-ID tests. Add final Contour-only mapping in
   compose_hovered and table-driven tests for all eleven joins plus exclusions.
5. Add UiPalette, 8-byte transfer, 96-byte aligned uniform, shader lookup and
   background conversion as one coherent render change. Preserve old set_theme
   callers as described. Add packing/palette/header tests, then device tests.
6. Connect app palette selection in its existing apply path. Exercise the real
   App unit seam and M4.2 controller/document contracts under Color. Never bypass
   projection polling, certified intents, capture, or atomic metrics publication.
7. Run focused tests and required repository checks, then explicit ignored GPU
   tests. Add/review only the specified new snapshots. Report actual results,
   pending visual/device checks and baseline failures. Do not commit/publish or
   dispatch another role. The human performs manual acceptance as available.

Routine local implementation details within these interfaces are Executor choices;
they do not require another Architect or Challenger pass. A concrete product,
ownership or binding-rule conflict is the reason to return, not optional polish.

## 7. Tests, ownership and acceptance evidence

Tests listed here are future Executor work. Architect ran none. Prefer extending
existing helpers/parameterized cases over duplicate frameworks.

| Exact location | Meaningful required evidence |
|---|---|
| `crates/newell-ui/src/cell.rs`, `grid.rs`, `widgets/mod.rs` unit tests | Default/clear ink Text; full-cell Canvas writes and ActionBox copies include ink; clipping cannot leak ink/ownership beyond bounds. Update stale-cell fixtures and preserve existing style assertions. |
| `crates/newell-ui/src/layout/border.rs` and `crates/newell-ui/src/widgets/{button,field,selector,tree,hover}.rs` unit tests | Border corners/joins Contour; labels/values Text; Action limited to explicit affordances; Edit vs Selection brackets differ despite identical glyphs. Tree literal `│`, bracket or `☰` inside a name stays Text. Hover `│→╞/╡`, `┆→╞/╡`, `╰╴→╘ ` preserves ink/style/WidgetId/SharedBorder for Text/Contour/Action fixtures, including blank metadata. No new eligible slot from a converted double frame. |
| `crates/newell-ui/src/{action_box,create_box,extrude,revolve,loft,sweep,boolean}.rs` unit tests | Mixed `│ │ Depth` outer Contour/inner Text, titles vs fill/corners, numeric +/− vs adjacent separator/value, selection fields for count 0/1/many, active-segment/footer dividers Contour. Scratch crop copies correct roles with the unchanged hit map. |
| `crates/newell-ui/src/menu/tests.rs`, `crates/newell-ui/src/overlays/tests.rs`, `crates/newell-ui/src/overlays/{selection,rename,formula}.rs` unit tests | Popup, Rename, Formula and SelectionPanel::draw_in_action_box title letters Text; both corners and adjacent top strokes/junctions Contour. Selection nested list frame, structural separator, bottom and shared footer divider Contour. Hover OK/Cancel leaves title/edges/separator roles unchanged and substitutes only current boundaries with inherited roles. Include cropped title and prefilled underlying ink/ownership, modal outside routing-mask preservation, full replacement, list controls/scroll thumb. |
| `crates/newell-ui/src/shell/tests.rs` and `crates/newell-ui/tests/it/menu_bar.rs` | Default unchecked; pointer, Enter and Space each update Color flag/checkmark while keeping View open/focused. Test both toggles and all four Theme/Color pairs; Theme toggles both directions keep Color. IDs 210–214 retain effects. Both Theme strings: relative offsets 6/8/16 Action, word/value Text, empty padding no ink, popup borders Contour, unchanged Theme glyph coordinates and activation owners off/on. Two independent shells prove default/state independence. |
| `crates/newell-ui/src/shell/compose.rs` unit tests | Eleven-map table at Contour, same glyphs excluded for Text/Action, single/rounded/dashed/hover glyphs unchanged, off is exact identity. Compare all style/ownership/SharedBorder and coordinates before/after; mapping only changes eligible content. |
| `crates/newell-ui/src/shell/tree_tests.rs` | Use live ModelTreeProjection with nested/empty folders, operation/output kinds, long names and enough rows to scroll; enable Color and keep full names/counts/preorder, ellipsis, roles and per-part hit tokens correct after scrolling/expand/reconciliation/replacement. Playhead guides/dashes Text next to Contour pane joins at minimum pane width; before-first/between/End semantics unchanged. Retain historical fixture comparisons when off. Window Theme/Color survive projection replacement. |
| `crates/newell-app/src/controller.rs::tree_contract_tests`; `crates/newell-app/tests/it/{controller,tree,hover}.rs` | Parameterize relevant current stale-hit/Rename/OwnedDrag cases with Color off/on. Ink/glyph-only private publish with identical routes preserves generation and capture. Actual scroll/overlay/document replacement still invalidates old TreeHitTokens. Three moves of one captured Navigation/Split/Playhead drag rebind only its own capture; release/exit/overlay/resize still cancel. Rename's certified one-time close intent and original/source-name checks remain; no color in route comparison. Color/Theme menu changes produce Update.grid and hover uses the published grid. |
| `crates/newell-app/src/app.rs` existing private unit module | Use live_app, click/key and last_ui_publication. For both Theme directions with Color off/on, actual visible row changes and published grid equals controller.grid; palette helper gives the exact pair used by sync_view_choices. Color pointer/Enter/Space reaches the same upload seam. Verify shading choice unchanged, current document identity/revision and a separately subscribed notification receiver unchanged on isolated toggles, no new TreeIntent, and M4.2's consumed Extrude latch remains one-shot. After a real rename/visibility refresh and document replacement, palette stays selected while roles reflect the new projection. Do not construct App from an external integration test: it is crate-private and owns one window. |
| `crates/newell-app/src/document.rs` cfg(test) module, reusing ProjectionSync tests | With a Color-enabled controller, inject projection failure using the existing fail_next_projection seam; previous published grid/roles stay valid, pending refresh survives, 100-ms retry succeeds with roles and chosen palette intact. Retain suspension, user/layout rearming and acknowledgment rules. A Color activation is allowed to take the existing UserActivation path, never fabricate a document change or acknowledge failure. No production API change for these tests. |
| `crates/newell-render/src/ui/{gpu_cell,theme,pipeline}.rs` and `crates/newell-render/src/renderer.rs` unit tests | Exact 8-byte row-major packing and role indices, unknown glyph retains role, styles/owners omitted, zero grid; exact four source palettes/default and contrast. Factor uniform byte assembly into a private pure function used by write_uniforms and assert 96-byte length, offsets/padding/three inks. Test clear conversion for sRGB and linear formats. Test UiPalette::with_theme preserves Color; the retained wrappers must delegate to it. |
| `crates/newell-render/tests/it/ui_gpu.rs` | Extend offscreen helper to accept UiPalette and target format Rgba8Unorm/Rgba8UnormSrgb, preserving old monochrome cases. All four palettes: Text/Contour/Action, Empty/background, one draw (zero when empty), same cell geometry/crop/atlas count. Actual Theme strings and mixed title/frame/guide fixtures render coverage-weighted role colours; full-block fixtures provide covered pixels for exact source-hex checks. Palette-only changes with the same grid work without set_grid; changed Theme strings explicitly call set_grid and show new glyphs. Keep k-switch/cache tests with selected palette and exercise UiOverlay::set_theme after enabling Color to prove its wrapper preserves the coloured output. |
| `crates/newell-render/src/ui/pipeline.rs` private ignored GPU unit test | Inject a raw role word outside 0..3 into its private cell buffer and render as Text, exercising the shader fallback. Use a local minimal device/readback fixture; do not expose raw upload or private buffers in production merely for an integration test. |
| `crates/newell-ui/tests/it/snapshots.rs` | Keep current glyph/style serialization and all existing snapshots exact. Add named cases view_menu_color_unchecked, view_menu_color_checked_dark, view_menu_color_checked_light. New files are `crates/newell-ui/tests/it/snapshots/it__snapshots__view_menu_color_unchecked.snap`, `crates/newell-ui/tests/it/snapshots/it__snapshots__view_menu_color_checked_dark.snap`, `crates/newell-ui/tests/it/snapshots/it__snapshots__view_menu_color_checked_light.snap`. Dark/Light checked glyphs may coincide; role/colour assertions live in the tests above, not inferred from a text snapshot. |

For GPU pixel expectations use atlas coverage, existing ALPHA_BLENDING and the
format's conversion; do not demand exact source RGB from a thin antialiased glyph.
Use a fully covered interior pixel of a role-tagged full block for source colour,
and explicit tolerances for rounding/conversion only. Linear-target tests retain
today's component behavior; sRGB-target tests compare after encode/decode.
UiOverlay::draw already returns the draw count; use it rather than a new profiler.
App's upload seam proves changed cells are handed off; device tests prove actual
transfer/rendering. Neither alone is a live multi-window or physical visual test.

## 8. Completion criteria and manual acceptance

Implementation is complete when the required checks pass (or pre-existing failures
are clearly separated), §7 assertions hold and the following behavior is verified:

1. Startup Dark + unchecked and exact black/white output off; Color off restores
   original glyphs immediately. Existing non-View snapshots remain exact.
2. Four exact palettes and independent Theme/Color, both Theme strings, focused
   open-popup toggles and unchanged visibility actions. No white text on Light.
3. Ordinary/tree/title ink Text; structural shared strokes Contour; explicit
   affordances Action, including Theme brackets/glyph. All mixed rows/clipping/
   overlay replacements and hover inherit roles without geometry/routing changes.
4. Eleven single-stroke mappings only when checked; hover substitutions and their
   eligibility remain current. One batched UI draw, O(1) hit routing, coverage-only
   atlas and target-format colour conversion, viewport palette background with
   scene display-mode behavior unchanged.
5. Current live M4.2 document projection, events/retry/acknowledgment, dynamic tree
   identities/scroll/rename/visibility/history cursor, generation/capture and
   one-shot Extrude confirmation retain their contracts. No presentation state in
   DocumentSession/model/projection or additional rendering/scheduling machinery.

Executor follows docs/agents/README.md's baseline and final cargo check, format,
Clippy, snapshot-disabled workspace/doc tests and explicit ignored GPU suite.
Do not weaken existing assertions or silently regenerate reference snapshots.
Architect documentation consistency/whitespace checks do not establish any of
those implementation results.

Manual acceptance after implementation: inspect all four states at reference and
compact k, Light contrast, text within controls, tree/frame/shared joins, nested
live folders/output names/counts/scroll, mixed popup/dialog/selection titles,
hover entry/exit, menu pointer and keyboard behavior, metrics/monitor transition,
viewport background with every existing model display style, and M4.2 rename/
visibility/insertion/capture flows. No physical-device or visual acceptance is
claimed in this documentation stage. Two-shell tests verify per-instance state;
live two-window independence remains a later integration/manual requirement when
the current single-window App supports that operation.

**Documentation verification (2026-10-26):** reviewed accepted palette, role,
menu, caret, hover, logo and workflow wording across the five changed documents.
Scoped `git diff --check` reported no errors; the two migrated documents have
no trailing whitespace and their Markdown tables are consistent. Existing
README_UI.md whitespace outside the edited passages was preserved. The Git index
remains unchanged; no build, test or application command was run.

**Handoff:** documentation amendments are authorized and applied; no concrete
unresolved architectural or functional conflict was found in the inspected
interfaces. This revision is the implementation handoff for the human's next
request. This stage starts neither Executor nor another review. Keep all existing
work and the index intact; make no external publication or integration action.
