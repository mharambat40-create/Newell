# Newell — Milestone 2 UI Framework Plan

**Milestone:** M2 — ASCII UI Framework & Application Shell  
**Status:** M2.1–M2.16 and M2.19–M2.25 validated locally; M2.17–M2.18 retain native checks (see `docs/validation/M2.24-2026-10-16.md`).
**Implementation scope:** `newell-ui`, `newell-render`, `newell-app`  
**Architecture authority:** `ARCHITECTURE.md`

This document is an implementation plan. It does not override `ARCHITECTURE.md` or the UI design specification.

> **Amendment (M2.16–M2.24, 2026-10-15).** Sections of this plan written for
> M2.1–M2.15 are historical and are intentionally not rewritten. Where they
> mention hover (`InteractionState.hovered`, hover uniform, hover stripe, §4.5,
> §6.6, §7.2, §10.3, §11.3, §13.5, §16.6, M2.5–M2.7), a 3:5 cell, the
> `║` active-segment separator or palette/muted style slots, they are
> **superseded** by `ARCHITECTURE.md` §3.1, §3.1.1, §3.5, §3.6, ADR 0003 and the
> M2.16–M2.24 sections below. Do not implement the superseded text. The one
> exception is the M2.7 `║` active-segment separator, which stays as a
> **temporary accepted deviation** until M2.21 (`ARCHITECTURE.md` §3.5).
>
> **Amendment (M2.25).** `ARCHITECTURE.md` §3.5 (*ASCII Hover Highlight*) now
> authorises a discrete, text-only hover highlight. The M2.16–M2.24 sentences
> "no hover", "hover removal" and "no hover state exists" describe those
> milestones and stay as written; they do **not** forbid M2.25. The hover
> removed in M2.16 (uniform, shader stripe, colour, `WidgetState::Hovered`)
> stays removed and is not reintroduced; M2.25 is a different mechanism
> (glyph substitution at composition time).

---

## 0. Specification Sources and Precedence

Milestone 2 uses the following source order:

1. `ARCHITECTURE.md`
2. `docs/ui_design/README_UI.md`
3. Relevant ASCII reference states under `docs/ui_design/`
4. This implementation plan
5. Existing implementation details, where they do not conflict with the above

If an ASCII reference conflicts with `docs/ui_design/README_UI.md`, the README wins.

If any UI document conflicts with `ARCHITECTURE.md`, `ARCHITECTURE.md` wins.

The deleted legacy `ui_design.md` must not be referenced.

### Reference-state policy

Files under `docs/ui_design/` are design references, not independent architecture documents.

They may describe:
- complete application layouts;
- command states;
- menus;
- overlays;
- user-action sequences;
- state transitions.

Shared component semantics belong in `docs/ui_design/README_UI.md`, not duplicated across every ASCII mock-up.

---

## 1. Current Repository State Relevant to M2

Milestone 1 established the application/runtime skeleton.

Relevant existing state:

- `newell-app`
  - owns the `winit` application/event loop;
  - owns the application window;
  - owns the frame-pacing bootstrap;
  - routes window events;
  - currently runs one window.

- `newell-render`
  - owns `wgpu` initialization;
  - owns the surface/device/queue;
  - renders the current clear-colour frame;
  - handles resize, minimise, surface loss, GPU errors and presentation.

- `newell-ui`
  - exists as a workspace crate but does not yet own the complete UI grammar required by M2.

Known versions from M1.3:

- `wgpu = 26.0.1`
- `winit = 0.30.13`
- `raw-window-handle = 0.6.x`
- workspace MSRV: Rust `1.85`

Milestone 2 builds on the M1.3 window and surface lifecycle. It must not reimplement that infrastructure.

---

## 2. Milestone 2 Goal

By the end of M2, Newell must display and interact with a deterministic UI-only application shell matching the intended ASCII-grid design.

The reference shell contains:

- `MenuBar`
  - File
  - Edit
  - View
  - Help

- `CommandBar`
  - Workbench
  - Sketch
  - Create
  - Operate
  - Pattern
  - Refine
  - Construct
  - Inspect

- `NavigationPanel`
  - Features tree
  - Elements tree

- `Viewport3D`
  - existing clear-colour viewport
  - static NavCube placeholder only

- contextual `ActionPanel`
  - reference Extrude UI mock

- `StatusBar`

The complete reference state uses deterministic mock data.

The goal of M2 is to establish Newell's UI grammar and interaction mechanics, not CAD semantics.

---

## 3. Explicit Non-Goals

M2 must not implement:

- BrepKIT integration;
- real geometry;
- real parametric features;
- Boolean geometry;
- actual Extrude execution;
- model recomputation;
- expression evaluation;
- parameter dependency graphs;
- topological naming;
- real geometric selection;
- functional NavCube camera control;
- functional viewport display modes;
- multi-window synchronisation;
- shared document event bus;
- undo/redo architecture;
- final CAD command architecture;
- logo rendering unless a later M2 sub-step explicitly requires it.

M2 may display CAD-like concepts using deterministic mock state, but those concepts must remain UI-only.

---

## 4. Architectural Decisions

### 4.1 Immediate-mode composition into a retained flat render buffer

Newell does **not** use a retained widget tree.

High-level UI state is stored as plain Newell-owned structs/enums.

When relevant state changes, pure composition/layout functions regenerate the logical UI into one flat cell buffer:

```text
Persistent UI state
Temporary interaction state
        │
        ▼
pure layout / composition
        │
        ▼
Vec<Cell>
        │
        ▼
newell-render
```

There is:

- no DOM;
- no retained widget object hierarchy;
- no generic scene graph;
- no diffing engine;
- no widget bounding-box scan for normal hit-testing.

### 4.2 State categories remain separate

Five concerns remain distinct:

1. persistent UI state;
2. temporary interaction state;
3. pure layout/composition;
4. flattened render cells;
5. widget identity/event routing.

A `Cell` must not own high-level command state.

### 4.3 Full logical rebuilds are acceptable in M2

UI state changes may regenerate the complete logical cell buffer.

This is deliberately simpler than introducing incremental diffing.

M2 should measure before introducing partial rebuild machinery.

### 4.4 Single UI draw call remains binding

The complete ASCII UI overlay must be rendered in one `wgpu` draw call.

The 3D viewport remains a separate pass rendered below the overlay.

Popup menus and modal overlays are flattened into the same logical cell buffer and do not create extra UI draw calls.

### 4.5 Interaction-state updates should avoid unnecessary cell regeneration where practical

Hover/focus/pressed state may be passed to the shader separately when this remains simple and compatible with the single-draw architecture.

The preferred M2 design is:

- cell data identifies `WidgetId`;
- a compact interaction uniform contains current hovered/focused/pressed IDs;
- the shader compares cell/widget identity with those IDs;
- hover/focus colour treatment remains shader-side.

This avoids regenerating the full grid solely for transient pointer movement.

If implementation evidence shows that this complicates the pipeline disproportionately, the Challenger may recommend a simpler M2-only alternative, provided `ARCHITECTURE.md` remains satisfied.

### 4.6 No global UI singleton

All UI state must remain instantiable per application window.

M2 implements one window, but no type or API may make future per-window ownership impossible.

---

## 5. Proposed Module Layout

The exact filenames may be adjusted if current crate structure requires it, but the responsibility split should remain equivalent.

### `crates/newell-ui`

Suggested logical modules:

```text
src/
├── lib.rs
├── cell.rs
├── grid.rs
├── metrics.rs
├── widget_id.rs
├── style.rs
├── interaction.rs
├── hit_test.rs
├── layout/
│   ├── mod.rs
│   ├── rect.rs
│   ├── border.rs
│   └── shell.rs
├── widgets/
│   ├── mod.rs
│   ├── label.rs
│   ├── button.rs
│   ├── checkbox.rs
│   ├── segmented.rs
│   ├── cyclic.rs
│   ├── field.rs
│   └── tree.rs
├── overlays/
│   ├── mod.rs
│   ├── menu.rs
│   ├── selection.rs
│   └── formula.rs
└── reference/
    ├── mod.rs
    ├── app_state.rs
    └── extrude.rs
```

Do not create all modules eagerly if a sub-milestone does not need them yet.

### `crates/newell-render`

Suggested additions as needed:

```text
src/
├── ...
├── ui/
│   ├── mod.rs
│   ├── gpu_cell.rs
│   ├── glyph_atlas.rs
│   ├── pipeline.rs
│   └── shader.wgsl
```

### `crates/newell-app`

The application owns orchestration:

- `winit` events;
- pointer/keyboard input;
- per-window UI state;
- calling `newell-ui` composition;
- handing cells to `newell-render`.

Avoid putting reusable UI composition logic in `newell-app`.

---

## 6. Proposed Public Interfaces / Signatures

Signatures below are architectural sketches, not production code.

### 6.1 Cell

```rust
pub struct Cell {
    pub content: CellContent,
    pub style: StyleId,
    pub flags: CellFlags,
    pub widget_id: WidgetId,
}
```

`Cell` contains only data needed by rendering and O(1) event routing.

Possible content representation:

```rust
pub enum CellContent {
    Glyph(char),
    Logo(LogoSliceId),
    Empty,
}
```

If this representation prevents the desired compact size, a compact tagged integer representation may be used instead.

### 6.2 Widget identity

```rust
pub struct WidgetId(u32);
```

`WidgetId` must identify what a control logically is, not its screen position.

Examples:

- File menu entry;
- Create menu;
- Extrude Depth field;
- Extrude Starting Offset checkbox;
- tree node toggle for a specific mock node.

Dynamic layout changes must not renumber unrelated controls.

### 6.3 Grid

```rust
pub struct CellGrid {
    cols: u32,
    rows: u32,
    cells: Vec<Cell>,
}
```

Expected methods:

```rust
pub fn new(cols: u32, rows: u32) -> Result<Self, UiError>;
pub fn cols(&self) -> u32;
pub fn rows(&self) -> u32;
pub fn index(&self, col: u32, row: u32) -> Option<usize>;
pub fn get(&self, col: u32, row: u32) -> Option<&Cell>;
pub fn get_mut(&mut self, col: u32, row: u32) -> Option<&mut Cell>;
```

No unchecked indexing in production code.

### 6.4 Grid metrics

```rust
pub struct GridMetrics {
    pub cell_width_px: u32,
    pub cell_height_px: u32,
}
```

The cell ratio is 4:9 (`cell_width_px = 4k`, `cell_height_px = 9k`, reference 20 × 45). This supersedes the original 3:5 (M2.16 amendment). Metrics are defined once and shared by the glyph atlas, renderer and hit-testing (`ARCHITECTURE.md` §3.1).

### 6.5 Hit testing

```rust
pub fn hit_test(
    position_px: UiPoint,
    metrics: GridMetrics,
    grid: &CellGrid,
) -> Option<HitTestResult>;

pub struct HitTestResult {
    pub col: u32,
    pub row: u32,
    pub widget_id: Option<WidgetId>,
}
```

Prefer a Newell-owned `UiPoint` over coupling `newell-ui` directly to `winit`.

Sub-cell routing should use a separate pure helper.

### 6.6 Interaction state

Conceptually:

```rust
pub struct InteractionState {
    pub hovered: Option<WidgetId>,
    pub focused: Option<WidgetId>,
    pub pressed: Option<WidgetId>,
}
```

### 6.7 Composition

Reference-state composition should be pure:

```rust
pub fn compose_reference_shell(
    state: &ReferenceUiState,
    interaction: &InteractionState,
    viewport: GridRect,
) -> Result<CellGrid, UiError>;
```

The function must not call GPU APIs.

---

## 7. State Ownership

### 7.1 Persistent UI state

Examples:

```text
ReferenceUiState
├── active_workbench
├── navigation
├── active_command
├── menu_state
├── panel_sizes
└── overlay_state
```

For M2, these may be mock/reference structs.

### 7.2 Temporary interaction state

Examples:

- hovered widget;
- pressed widget;
- keyboard focus;
- pointer capture for panel resizing;
- current modal focus owner.

### 7.3 Render state

`CellGrid` / `Vec<Cell>` is a render representation.

It must not become the authoritative source of application semantics.

### 7.4 Renderer state

`newell-render` owns:

- GPU buffers;
- glyph atlas;
- bind groups;
- pipeline;
- UI interaction uniform;
- upload logic.

It does not own command semantics.

---

## 8. Cell and WidgetId Design

### 8.1 Cell responsibility

A `Cell` carries:

- glyph/empty/logical logo content;
- compact style identity;
- compact renderer-visible flags;
- `WidgetId`.

It must not carry:

- command structs;
- tree children;
- strings for full labels;
- callbacks;
- closures;
- model entities;
- BrepKIT objects;
- layout rectangles.

### 8.2 Cell size target

The design target is approximately **16 bytes per cell**.

This is a target, not an architecture invariant.

The implementation should include a size regression test if the chosen representation is intentionally compact.

Do not distort the design solely to hit 16 bytes if doing so reduces clarity or safety.

### 8.3 Style representation

Preferred design:

```text
Cell
  └── StyleId
        ↓
per-window Theme / style table
```

Reasons:

- avoids repeating full colours per cell;
- keeps GPU upload compact;
- allows consistent theme changes;
- separates semantic styling from raw GPU colour data.

The renderer may expand `StyleId` into GPU-ready values during upload.

### 8.4 WidgetId properties

`WidgetId` must be:

- stable across recomposition;
- independent of screen coordinates;
- independent of GPU resources;
- independent of BrepKIT/model types;
- usable by multiple cells belonging to the same control;
- suitable for future per-window UI instances.

For deterministic mock trees, IDs may be derived from stable mock node IDs.

Do not derive IDs from vector position if insertion/removal would renumber unrelated widgets.

---

## 9. Layout Architecture

M2 must not implement a CSS/Flexbox clone.

Use the minimum primitives necessary for the Newell shell.

Required primitives:

- `GridRect`;
- horizontal split;
- vertical split;
- fixed rows/columns;
- remaining-space region;
- clipping;
- border drawing;
- row writing;
- column writing;
- overlay placement.

The layout must express:

- fixed-height `MenuBar`;
- fixed-height `CommandBar`;
- fixed-height `StatusBar`;
- resizable `NavigationPanel`;
- contextual `ActionPanel`;
- expanding `Viewport3D`;
- nested trees;
- variable-height ActionPanel content;
- overlays.

### Box drawing

M2.2 should express borders logically using Unicode box-drawing semantics.

M2.3 should first test those glyphs through the selected font pipeline.

Do **not** commit to a procedural box-drawing rasterizer before a visual test demonstrates that Fira Code cannot produce acceptable joins at Newell's 4:9 cell metrics (`ARCHITECTURE.md` §3.1; Fira Code's overshoot has been verified to cover the 4:9 cell).

If a procedural solution becomes necessary, it must preserve:

- the single UI draw call;
- the flat cell representation;
- deterministic layout;
- shader-side rendering constraints.

---

## 10. Event and Hit-Testing Flow

Expected event flow:

```text
winit event
    ↓
newell-app
    ↓
pointer/keyboard normalisation
    ↓
newell-ui hit test / focus routing
    ↓
WidgetId
    ↓
logical UI state mutation
    ↓
layout / cell-buffer regeneration if necessary
    ↓
newell-render upload / redraw
```

### 10.1 Pointer mapping

Normal pointer mapping must remain O(1):

```text
col = x / cell_width
row = y / cell_height
index = row * cols + col
widget = cells[index].widget_id
```

No normal path may iterate through widget rectangles.

### 10.2 Shared-border routing

Shared ASCII separators use sub-cell modulo logic.

Example concept:

```text
local_x = x % cell_width
```

The result decides whether the interaction belongs to the left or right adjacent control.

### 10.3 State-change categories

#### Redraw only

Examples:

- hover change if handled entirely through interaction uniform;
- pressed highlight change;
- focus highlight change.

#### Cell-buffer regeneration

Examples:

- menu opened/closed;
- tree expanded/collapsed;
- checkbox reveals new rows;
- cyclic selector value text changes;
- segmented selector changes visible state;
- panel width changes.

#### Layout regeneration

Examples:

- ActionPanel sections appear/disappear;
- window grid dimensions change;
- NavigationPanel width changes;
- overlay placement changes.

---

## 11. Rendering Pipeline

The existing M1.3 clear-colour render pass remains.

M2 introduces an ASCII UI overlay pass:

```text
surface frame
    ↓
3D / clear-colour viewport pass
    ↓
single ASCII UI overlay pass
    ↓
present
```

### 11.1 One UI draw call

The complete logical UI must render with one UI draw call.

Conceptually:

```rust
draw(0..6, 0..cell_count)
```

or an equivalent single instanced draw.

### 11.2 GPU cell format

The CPU `Cell` does not have to equal the GPU struct byte-for-byte.

A conversion layer may produce a GPU-friendly `GpuCell`.

The conversion must remain linear in cell count and simple enough for full-grid uploads.

### 11.3 Interaction uniform

Preferred approach:

- hovered ID;
- focused ID;
- pressed ID;
- optional selected/global interaction flags.

Keep the uniform compact and aligned for `wgpu`.

Do not encode arbitrary high-level UI state in the shader uniform.

### 11.4 Overlays

Menus and modal overlays are composed last into the same cell grid.

They remain part of the same UI draw.

Z-order is a composition concern, not a separate GPU pass.

---

## 12. Font and Glyph Strategy

### 12.1 Font asset

Canonical asset:

```text
assets/FiraCode-Regular.ttf
```

License file:

```text
assets/OFL.txt
```

Recommended asset documentation:

```text
assets/README.md
```

### 12.2 Licensing

Fira Code remains under SIL Open Font License 1.1.

The font is not relicensed under Newell's AGPL-3.0-or-later license.

`assets/OFL.txt` must contain the upstream OFL text.

`assets/README.md` must identify:

- asset filename;
- upstream project;
- copyright;
- SIL OFL 1.1;
- the local licence file.

### 12.3 Font crate

Planned dependency:

```text
ab_glyph = 0.2.32
```

Rationale:

- compatible with workspace MSRV 1.85 based on Architect verification;
- Apache-2.0 licence is already allowed;
- maintained and lightweight;
- sufficient for the glyph pipeline required by M2.

Before adding, Executor must re-run:

```text
cargo deny check
rustup run 1.85.0 cargo check --workspace --locked
```

### 12.4 Glyph atlas

M2.3 should:

- load Fira Code;
- determine glyph metrics;
- rasterise exactly the required atlas set (`ARCHITECTURE.md` §3.1) once per `k`,
  at atlas creation; the atlas is never modified afterwards (no on-demand
  rasterisation, no eviction, no re-packing);
- maintain deterministic glyph atlas placement (a pure function of font,
  required set and `k`);
- draw every character outside the required set as `?`;
- rasterise every glyph, `?` included, with the same integer-`k` uniform scale;
- support ASCII UI and required Unicode symbols used by the reference designs.

The glyph atlas is distinct from the future logo atlas.

### 12.5 Logo atlas

No logo is required by the current near-term M2 implementation.

Therefore:

- do not create a placeholder logo atlas;
- do not generate a logo atlas at runtime;
- do not add a PNG decoding dependency for a placeholder;
- defer logo-atlas implementation until the first sub-milestone that actually renders logos.

When introduced, the logo atlas must follow `ARCHITECTURE.md` exactly. That includes
the logo resampling exception of §3.3: logos are always downscaled by the shader
with their own filtered sampler, while text glyphs stay nearest-neighbour.

---

## 13. Widget Primitives

M2.7 introduces reusable grid-native primitives.

Required initial set:

- label;
- separator;
- button;
- checkbox;
- segmented selector;
- cyclic selector;
- numeric/text field;
- tree row.

These are composition helpers, not retained widget objects.

### 13.1 Segmented selectors

Use when alternatives should remain visible simultaneously.

Examples:

```text
Solid | Surface
New | Add | Remove
```

Selection is semantic and structurally visible.

### 13.2 Cyclic selectors

Use when one ordered value is shown at a time.

Example:

```text
◀      Blind      ▶
```

Possible values:

```text
Blind
Up to
Through all
```

The number of options does not decide the component type.

### 13.3 Boolean controls

Example:

```text
□ Starting Offset
■ Starting Offset
```

### 13.4 Fields

Example:

```text
Depth [0.0 mm   ☰]
```

M2 fields expose UI behaviour only.

No formula evaluation occurs.

### 13.5 Trees

Tree rows support:

- indentation;
- expanded/collapsed state;
- selection;
- hover;
- focus;
- stable node identity.

---

## 14. Dynamic ActionPanel Strategy

The ActionPanel is generated from command state.

Conceptual flow:

```text
ExtrudeUiState
      ↓
ActionPanel composition
      ↓
rows/cells
```

No permanent fixed form is required.

### 14.1 Extrude reference state

M2.11 implements only UI state.

Suggested UI-only types:

```text
ExtrudeGeometryType
  - Solid
  - Surface

ExtrudeResultMode
  - New
  - Add
  - Remove

ExtrudeTermination
  - Blind
  - UpTo
  - ThroughAll
```

Optional booleans:

- Invert;
- Starting Offset;
- Symmetric;
- Second End Position;
- Merge with all.

Mock selection state:

- none;
- one element;
- multiple elements.

### 14.2 Progressive disclosure

Example:

```text
□ Starting Offset
```

becomes:

```text
■ Starting Offset
  ◀      Blind      ▶
  Depth [0.0 mm   ☰]
```

Rows are inserted/removed from composition.

No invisible reserved rows are required.

### 14.3 No CAD semantics

The Extrude mock must not:

- call BrepKIT;
- create a feature;
- update geometry;
- perform Boolean operations;
- resolve real model selections.

---

## 15. Overlay Strategy

M2.8 and M2.12 introduce overlays.

Required overlay classes:

- popup menu;
- SelectionPanel;
- FormulaEditor;
- generic confirmation area.

### 15.1 Focus trapping

When a modal overlay owns focus:

- underlying covered controls do not receive pointer events;
- normal keyboard focus remains within the overlay;
- Cancel/Escape returns focus according to defined UI state.

Focus trapping is Controller logic.

### 15.2 SelectionPanel

Reference:

```text
Selection
┌───────────────────────┐
│ □ Rim_Body           ▲│
│ ■ Hub_Body           ┃│
│ □ Inner_Surface      ▼│
└───────────────────────┘
        OK | Cancel
```

M2 uses deterministic mock entries.

### 15.3 FormulaEditor

Reference:

```text
Formula

Fx = (#EntraxeA - 2mm) / 5 * 3
     * sin(#Angle1 - #Angle2 + 3deg)

        OK | Cancel
```

M2 implements only UI presentation/editing state.

No parser/evaluator is required.

---

## 16. Testing Strategy

Testing follows M1.2 conventions.

Prefer pure deterministic tests over GPU/window integration tests.

Use `insta` only where snapshots add meaningful coverage.

### 16.1 M2.1 tests

- grid allocation;
- row/column to flat index;
- bounds handling;
- stable `WidgetId`;
- cell default state;
- target `Cell` size if compact layout remains intentional.

### 16.2 M2.2 tests

- rectangular clipping;
- row/column layout;
- border composition;
- pane split calculations;
- too-small regions;
- zero-sized regions.

### 16.3 M2.3 tests

- font asset can be loaded;
- representative glyph rasterises;
- glyph metrics are deterministic enough for the chosen pipeline;
- box-drawing glyph visual/metric viability is assessed before procedural fallback.

### 16.4 M2.4 tests

Pure tests:
- CPU `Cell` -> `GpuCell` conversion;
- buffer lengths;
- style resolution;
- interaction ID packing.

GPU tests:
- ignored by default where a real adapter/window is needed.

### 16.5 M2.5 tests

- pixel -> cell mapping;
- edges and corners;
- out-of-grid coordinates;
- shared-border modulo routing.

### 16.6 M2.6 tests

- hover transition;
- press/release;
- focus movement;
- disabled widget behaviour;
- checked/selected state semantics.

### 16.7–M2.12 tests

- tree expand/collapse;
- segmented selector transitions;
- cyclic selector wrap-around;
- progressive disclosure;
- popup open/close;
- overlay focus trapping;
- SelectionPanel confirmation/cancellation;
- FormulaEditor confirmation/cancellation.

### 16.8 Reference snapshots

High-value logical cell-buffer snapshots:

- base application shell;
- File menu open;
- Create menu open;
- Extrude / New;
- Extrude / Add;
- Extrude / Remove;
- Starting Offset disabled;
- Starting Offset enabled;
- SelectionPanel open;
- FormulaEditor open;
- collapsed Features tree;
- expanded Features tree;
- narrow layout;
- wide layout.

Snapshots should target logical UI output, not GPU screenshots.

Avoid tests that merely reimplement the function under test.

---

## 17. Dependency and Asset Changes

### Planned dependency

For M2.3:

```text
ab_glyph = 0.2.32
```

Add only when M2.3 starts.

### Assets to add before M2.3

```text
assets/FiraCode-Regular.ttf
assets/OFL.txt
assets/README.md
```

### No logo dependency yet

Do not add:

- PNG decoder;
- placeholder atlas generator;
- image dependency solely for future logos.

### Required verification after dependency changes

```text
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo deny check
rustup run 1.85.0 cargo check --workspace --locked
```

---

## 18. Exact Files to Create or Modify

This list is milestone-level and should be narrowed by each sub-milestone before Executor work.

### Documentation

```text
docs/plans/M2-ui-framework.md
docs/ui_design/README_UI.md
docs/ui_design/*                 # reference ASCII states, only when design work requires edits
docs/backlog.md                  # milestone tracking only after acceptance
```

### Assets

```text
assets/FiraCode-Regular.ttf
assets/OFL.txt
assets/README.md
```

### `newell-ui`

Likely:

```text
crates/newell-ui/Cargo.toml
crates/newell-ui/src/lib.rs
crates/newell-ui/src/cell.rs
crates/newell-ui/src/grid.rs
crates/newell-ui/src/metrics.rs
crates/newell-ui/src/widget_id.rs
crates/newell-ui/src/style.rs
crates/newell-ui/src/interaction.rs
crates/newell-ui/src/hit_test.rs
crates/newell-ui/src/layout/*
crates/newell-ui/src/widgets/*
crates/newell-ui/src/overlays/*
crates/newell-ui/src/reference/*
crates/newell-ui/tests/it/*
```

### `newell-render`

Likely:

```text
crates/newell-render/Cargo.toml
crates/newell-render/src/lib.rs
crates/newell-render/src/ui/*
crates/newell-render/tests/it/*
```

### `newell-app`

Likely:

```text
crates/newell-app/Cargo.toml
crates/newell-app/src/app.rs
crates/newell-app/src/lib.rs
crates/newell-app/tests/it/*
```

Do not create all of these at once.

Each M2.x Executor task must touch only files required for that sub-milestone.

---

## 19. Acceptance Criteria by Sub-Milestone

### M2.1 — Cell, style, WidgetId, metrics and flat buffer

Complete when:

- core `Cell` exists;
- compact style reference exists;
- stable `WidgetId` exists;
- `CellGrid`/flat buffer exists;
- row/column indexing is safe and tested;
- metrics represent the grid cell dimensions;
- no retained widget tree is introduced;
- tests pass;
- no rendering changes are required yet.

### M2.2 — Character-grid layout primitives

Complete when:

- basic rect/split primitives exist;
- clipping works;
- borders can be composed;
- shell regions can be calculated using whole cells;
- no generic flexbox/layout engine is introduced;
- tests cover small/degenerate layouts.

### M2.3 — Fira Code glyph rasterisation

Complete when:

- Fira Code asset is committed with OFL;
- `ab_glyph` is integrated;
- required glyphs rasterise;
- atlas placement is deterministic;
- representative box-drawing glyph behaviour is assessed;
- no logo atlas is introduced;
- MSRV/licence checks pass.

### M2.4 — Single-draw ASCII renderer

Complete when:

- the entire logical cell buffer can be uploaded;
- one UI `wgpu` draw call renders the ASCII overlay;
- the existing viewport pass remains separate;
- interaction state reaches the shader as designed;
- public API does not leak unnecessary wgpu types into `newell-ui`;
- architecture decision is recorded if an ADR is still warranted.

### M2.5 — O(1) hit-testing

Complete when:

- pixels map directly to row/column;
- row/column maps directly to cell/widget;
- shared-border routing works through modulo logic;
- no bounding-box iteration exists in the normal path.

Accepted M2.5 decision (binding for M2.6+):

- `Cell` contains both `widget_id` and `shared_border`.
- When `shared_border` is `Some`, it takes precedence over `widget_id`.
- Shared-border routing is by sub-cell position: vertical borders use local x,
  horizontal borders use local y; `low` = left/upper, `high` = right/lower.
- This preserves O(1) routing without neighbour scans or a side table.
- Accepted trade-off: the CPU-side `Cell` grows. `GpuCell` carries the owning widget
  (`widget` + `flags`, with `WIDGET_PRESENT` / `WIDGET_LEFT_EDGE`) for shader-side hover.
  A shared-border cell has no GPU owner (see M2.6).
- Layout composition order: (1) draw visual borders, then (2) stamp
  shared-border interaction metadata. `draw_border` clears `widget_id` and
  `shared_border` on the cells it rewrites.

### M2.6 — Interaction state

Complete when:

- hover;
- pressed;
- focus;
- selected;
- disabled;
- expanded;
- checked

are represented consistently and tested.

Accepted M2.6 decisions:

- `InteractionState { hovered, focused, pressed }` and
  `WidgetState { selected, disabled, expanded, checked }` live in `newell-ui`.
  Only `hovered` is uploaded to the GPU (via the uniform buffer). `focused` and
  `pressed` are not uploaded yet.
- Hover is resolved by the M2.5 `hit_test` and applied in the shader. The overlay
  stays one draw call and hover changes do not re-upload cells.
- Per-row `WIDGET_LEFT_EDGE`: the hover left-border stripe is drawn on the first
  owned cell of each row run, not once per widget.
- Shared-border hover is **deferred to M2.7**. If a widget's visual left edge is a
  shared border, M2.6 currently highlights the first owned cell one cell inside the
  border. M2.7 must decide whether shared border cells remain visually neutral or
  participate in hover styling. Current rendering is unchanged by this note.

### M2.7 — Widget primitives

Complete when reusable composition helpers exist for:

- labels;
- separators;
- buttons;
- checkboxes;
- segmented selectors;
- cyclic selectors;
- fields;
- tree rows.

No retained widget instances are introduced.

Accepted M2.7 decisions:

- Primitives live in `newell-ui::widgets` as pure `draw_*` functions writing into a
  `CellGrid` inside a `GridRect`, clipped to the rect and grid. Styles come from
  `WidgetStyles` (placeholder palette slots); focus/press/selected/disabled are
  composed as style, hover is never composed (shader-side).
- Interactive primitives stamp caller-supplied `WidgetId`s on every cell they own
  (blanks included). Disabled controls keep their ids.
- **Shared-border hover: the border stays visually neutral (option A).** A shared
  border cell has `shared_border` routing and no GPU owner, so it never paints the
  hover stripe; the neighbouring control's stripe is painted on its first owned cell,
  one cell inside the border. Rationale: a shared cell has two owners by sub-cell half
  (ARCHITECTURE.md §3.4) but the GPU cell carries one, so border hover would need a
  second pixel-level routing mechanism in the shader; and the border is structure, not
  text (§3.5 stripe is a text-cell treatment). Pointer routing across the border is
  unchanged (M2.5).
- Segmented active-segment rule: separators are `│`; the separators touching the active
  segment are `║`. A segmented selector needs width `>= 2n - 1`.
- Cyclic selector: `◀` / `▶` own their cells, the value is unowned; `cycle_next` /
  `cycle_prev` wrap. The selector type is the caller's choice, never derived from count.
- Tree row: one id per row; guides `├ └ │` muted; marker `[-]`/`[+]` (branch) or `□`/`■`
  (leaf, from `WidgetState::checked`; meaning still open, README §12.2).
- **Disabled hover (Controller policy, recorded for later composition).** Disabled
  controls may retain their `WidgetId` for deterministic hit-testing and rendering, but
  Controller code MUST NOT set them as hovered, focused or pressed interaction targets.
  Disabled state therefore takes precedence over pointer interaction policy. No global
  disabled-widget registry is added; `InteractionState` is unchanged.

### M2.8 — Overlay primitives

Complete when:

- popup menu;
- modal container;
- scrolling selection list;
- OK/Cancel area

can be composed into the same cell grid.

### M2.9 — Menu and keyboard interaction

Complete when:

- application menu interaction works;
- command menu interaction works;
- mnemonics work;
- focus traversal works;
- keyboard activation mutates the same logical state as pointer input.

### M2.10 — Dynamic ActionPanel framework

Complete when:

- sections can appear/disappear from command state;
- progressive disclosure is stable;
- panel recomposition remains whole-cell based.

### M2.11 — Reference Extrude ActionPanel

Complete when the mock UI supports:

- Solid / Surface segmented selector;
- New / Add / Remove segmented selector;
- Selection field;
- Blind / Up to / Through all cyclic selector;
- Depth;
- Invert;
- Starting Offset;
- Symmetric;
- Second End Position;
- Merge controls where relevant;
- OK / Cancel.

No geometry operation occurs.

### M2.12 — SelectionPanel and FormulaEditor

Complete when both overlays:

- open;
- trap focus;
- accept/cancel;
- restore prior interaction state appropriately;
- use deterministic mock data.

No formula evaluation or real geometry selection occurs.

### M2.13 — Complete reference application shell

Complete when Newell can reproduce the intended reference UI with deterministic mock data:

- MenuBar;
- CommandBar;
- Features tree;
- Elements tree;
- Viewport3D;
- static NavCube placeholder;
- Extrude ActionPanel;
- StatusBar.

### M2.14 — Responsive whole-cell resizing

Complete when:

- OS resize recomputes grid dimensions;
- NavigationPanel / ActionPanel widths can change in whole cells;
- user-dragged separators snap by character column;
- narrow/wide layouts remain deterministic.

### M2.15 — Reference-state snapshots

Complete when:

- high-value logical states have deterministic snapshots;
- snapshots correspond to `docs/ui_design/` reference states;
- redundant snapshots are avoided;
- no GPU screenshot framework is required.

---

### M2.16–M2.24 amendment: dependencies and cross-cutting rules

```text
M2.16 ─► M2.17 ─► M2.18 ─► M2.19 ─► M2.20 ─► M2.21 ─► M2.22
  │         │        │                          │
  │         │        └────────────────► M2.23 ◄─┘ (needs M2.21 Extrude fields)
  └─────────┴──────────────────────────────────────────► M2.24 (last)
```

- M2.16 is the foundation: 4:9 metrics, uniform glyph scale, monochrome pair
  uniform, hover removal. Nothing later may reintroduce a ratio, colour or hover
  assumption.
- M2.17 needs M2.16 metrics and owns the **automatic per-monitor adaptation**:
  it derives `effective_k` from the monitor, mode and `preferred_scale_percent`,
  at start-up and on every monitor, resolution, display-scale, fullscreen or
  preference change, with one atomic transition and a safe window fit
  (`ARCHITECTURE.md` §3.1.1). M2.18 needs M2.17 and M2.16 (`k`-derived metrics,
  theme uniform); it adds **only the View menu selector** of the preference (and
  the Theme and Fullscreen rows). There is no rule that production stays at
  `k = 5` until M2.18, and M2.18 holds no `k`, monitor or window-fit logic.
- M2.19–M2.22 compose reference ASCII exactly and depend on M2.16 (state
  visuals) and M2.18 only for the View/Theme chrome they sit inside. M2.22
  depends on M2.21 (shared Action Box grammar). M2.23 depends on M2.21 (numeric
  field rows) and M2.12 (existing FormulaEditor overlay). M2.24 is last.
- Every milestone keeps: one UI draw call, O(1) hit-testing, flat cell buffer,
  monochrome `#000000`/`#FFFFFF`, no hover, and no state visual that is not a
  documented ASCII structure (`ARCHITECTURE.md` §3.5).
- Canonical `.txt` references are not edited by these milestones. The approved
  edits (UI Scale value line, canonical 31-cell Boolean footer, space-only Help
  popup, `Sketch` popup, `▏` in `Special_ASCII.txt`) are already applied by the
  architecture amendment.
- Reference sizes: shell 82 × 28 (`Interface_Exemple.txt`, unchanged); minimum
  window grid 82 × 30. The two extra rows belong to the flexible central body;
  snapshots at 82 × 30 are compared with the reference by the two-step rule of
  `README_UI.md` §25 (M2.15 and M2.24 snapshots).
- Defaults of every new window: `preferred_scale_percent` 100%, Light theme, neither
  inherited nor detected from the OS (`ARCHITECTURE.md` §3.1.1, §3.6).

### M2.16 — Typography, monochrome rendering, hover removal

Scope:

- Cell geometry 4:9, `k = 5` (20 × 45). One `k`-derived metrics value replaces
  the 24 × 40 constants in `newell-ui` and `newell-render`; no other literal W/H.
- Glyph atlas builder takes the metrics as input. Uniform scale
  `s = W / advance`, line box `L = 8k`, `top = k div 2`,
  `baseline_y = top + 6k`, pen origin at cell x = 0, clipping at the cell edge.
  `GLYPH_WIDEN` and every per-glyph or horizontal factor are deleted.
- Shader: nearest-neighbour atlas sampling, one texel per screen pixel; the
  atlas is a coverage mask; output colour is only the theme foreground or
  background (or a coverage blend of the two). Theme pair is a uniform; the
  default is Light (`#000000` on `#FFFFFF`). No per-cell colour.
- Hover removal: delete `hovered`, the hover uniform/ID/flag, the left-edge
  stripe, the logo brightness multiplier and pointer-motion redraws. Press,
  drag, scroll, keyboard focus, selection and disabled semantics remain.
- State visuals: delete the `>`/`<` marker pass (`mark_states`,
  `STATE_MARK_LEADING/TRAILING`) and any fg/bg inversion of widgets. Remove
  palette/muted/grey style slots; `Style` resolves to foreground or background
  only. Focus/pressed/selected stay as logical state with no visual unless an
  ASCII structure exists (`README_UI.md` §5.10).
- The M2.7 segmented selector keeps its `║` active-segment separators; this is
  a temporary accepted deviation ended by M2.21 (`ARCHITECTURE.md` §3.5). They are
  not extended to new widgets.
- Glyph coverage versus atlas coverage (`ARCHITECTURE.md` §3.1): the atlas is
  built for the required atlas set (printable ASCII, every character of
  `docs/ui_design/*.txt`, `▏`); an unsupported character is drawn as `?`.
- Defaults at this stage: `k = 5` fixed (no monitor derivation before M2.17), Light theme.
- Not in scope: runtime `k` change, per-monitor derivation, atlas regeneration on
  scale change and window changes (all M2.17); the UI Scale selector and the
  theme *control* (M2.18).

Acceptance:

- No `3:5`, `24`, `40` or widening constant remains outside historical docs.
- Metrics for all six `k` = 2…7 (§3.1 table, including `k = 2`: 8 × 18, `L = 16`,
  `top = 1`, `baseline_y = 13`) are unit-tested as pure arithmetic.
- Box-drawing continuity (horizontal and vertical joins across adjacent cells)
  is tested on the rasterised atlas for each `k` in 2…7, even though runtime
  uses `k = 5`; the builder is parameterised only so the test can exercise it.
- Negative test: pointer motion without a press changes no UI state and
  produces an identical cell buffer and frame.
- Monochrome test: every UI pixel is `#000000`, `#FFFFFF` or a coverage blend
  of the two, in both themes.
- Font coverage test: every character of the required atlas set is present in
  `assets/FiraCode-Regular.ttf`; atlas coverage test: each is rasterised for
  every `k` in 2…7. Any character outside the required set (including one the
  font contains, such as `é`) is drawn as `?`; rendering text with unknown
  characters leaves the atlas bytes, size and slot map unchanged (determinism).
- Block-element tiling test: `█` (and `░ ▒ ▓`) tile horizontally across five
  cells with no gap for each `k`; vertical coverage is exactly the line box `L`
  (`ARCHITECTURE.md` §3.1); no test requires vertical tiling.
- Hit-test round trip: for every cell, the pixel at its corners and centre maps
  back to that cell with the same W and H as the renderer.
- Snapshots are regenerated once for the 4:9 geometry; no snapshot content
  changes because of hover removal other than removed markers.

### M2.17 — Frameless window, automatic per-monitor scale and 82 × 30 minimum grid

Scope:

- **Automatic per-monitor UI scale** (`ARCHITECTURE.md` §3.1.1; ADR 0003 D3, D13).
  UI Scale is a user preference; the cell size adapts to the monitor. Three
  separate per-window values: `preferred_scale_percent` (60/80/100/120;
  fixed at the default 100 until the M2.18 control exists, but every function
  takes it as a parameter and is tested with all five), `monitor_reference_scale`
  (`k_fit(windowed) ≤ 3 ? max(k_fit(windowed) − 1, 2) : min(k_fit(windowed), 5)`;
  amended 2026-10-16, Full HD → 2, no monitor name tested) and `effective_k`
  (`clamp(k_req, 2, clamp(k_fit(mode), 2, 7))`,
  `k_req = (reference × percent + 50) / 100`), with
  `k_fit(mode) = min(⌊w/328⌋, ⌊avail_h/270⌋)`, `avail_h = h − h/16` windowed and
  `h` fullscreen. Pure integer functions in `newell-ui` (no `winit` types).
  Unknown monitor: reference 5, `k_cap` 7, re-derived when a snapshot becomes
  available. The estimate is not a work-area measurement and monitor size is
  never assumed to equal the work area.
- **Start-up.** Derive `k` before the window is created from the primary monitor
  (`ActiveEventLoop`, `winit` 0.30), build the `k`-derived metrics and atlas, and
  create the window with explicit `PhysicalSize` initial and minimum dimensions
  (initial = the 82 × 30 minimum `328k × 270k`; `LogicalSize` is never used).
  After creation the window's own monitor is read back and the derivation is
  repeated once if it differs from the primary monitor.
- **Re-evaluation triggers** (all six of §3.1.1): start-up; window moves to
  another monitor; monitor resolution change; macOS display scale change;
  fullscreen enter/exit (application- or OS-initiated); preference change (stub
  until M2.18). `winit` has no monitor-change event: keep a monitor snapshot
  `(handle, physical size, scale_factor bits)` compared on `Moved`,
  `ScaleFactorChanged`, `Resized`, `Focused(true)`, fullscreen changes and at most
  every 500 ms. A window `Resized` is never itself a trigger.
- **Atomic transition** (`ARCHITECTURE.md` §3.1.1 *Transition*), one event-loop
  turn, one `ScaleState` value published together: cell metrics, glyph atlas,
  `CellGrid` and flat cell buffer, GPU buffers/uniforms/bind groups, window
  minimum size, pointer-mapping metrics. Phase A prepares everything with no
  live state touched and abandons the whole transition on any failure (log once
  per input tuple, no retry until an input changes); Phase B cancels press/drag
  and resize gestures, clears the stored pointer position, swaps the one value,
  then `set_min_inner_size` → `request_inner_size` → `set_outer_position`. The
  atlas is rebuilt only when `k` changes (fast path otherwise). Frame
  presentation is gated for at most 250 ms until the matching `Resized`.
- **No oscillation / no recursive resize.** `effective_k` depends only on
  (monitor snapshot, mode, preference); window size and position are outputs.
  Idempotent re-derivation; `expected_inner_size` + generation so the echoes of
  the transition's own `Resized`/`Moved`/`ScaleFactorChanged` never start a
  transition; the `ScaleFactorChanged` size writer writes the expected size (or
  the unchanged physical size), never `winit`'s logical conversion; a monitor
  change during a window drag is applied once after 150 ms without `Moved`
  (`ControlFlow::WaitUntil`); at most one corrective size/position request per
  input tuple, then log once and accept.
- **Safe window fit and repositioning** (`fit_window`, pure, `newell-ui`): keep
  `desired_cols × desired_rows` (updated only by user resizes, maximise and
  restore, never by derived changes); clamp to
  `[82, ⌊mon_w/W⌋] × [30, ⌊usable_h/H⌋]` whole cells; shift the position the
  minimum distance so the whole outer rectangle lies inside the usable rectangle
  (monitor minus `h/16` at the top); no-fit case (`k_fit < 2`): 82 × 30 at
  `k = 2`, top-left of the usable rectangle, log once. Fullscreen enter stores
  the windowed grid and position; exit restores through the same fit; fullscreen
  is refused (logged once) when 656 × 540 does not fit the monitor. Size,
  position and monitor are read back after every change.
- Documented behaviour, not a defect: a small monitor clamps several preferences
  to one `effective_k` (1920 × 1080 windowed: 60/80/100/120% all give `k` = 2).
- Native decorations off. The OS title bar and controls are replaced by the
  in-grid MenuBar row, including `[-] [□] [X]` (`Interface_Exemple.txt`).
- Window controls are ordinary grid hit targets (minimise, maximise/restore,
  close). The window stays manually resizable and movable (`ARCHITECTURE.md`
  §3.1.1 *Window move and resize*, `README_UI.md` §25.1). Moving uses
  `drag_window`. Resizing first calls `drag_resize_window`; every call returns a
  `Result` that is handled, never unwrapped. `drag_resize_window` always returns
  `NotSupported` on macOS (verified in `winit` 0.30.13): this is logged once per
  window, remembered, and the press starts the **application-managed resize**:
  snapshot (outer position, inner size, zone, grab point) on press; on every
  `CursorMoved` re-derive the target from the snapshot and
  `outer_position() + pointer` (never accumulate); round to whole cells; clamp
  to `82·W × 30·H` and the surface limit; when the snapped target changes,
  `request_inner_size(PhysicalSize)` then `set_outer_position(PhysicalPosition)`
  (this order, because macOS derives the vertical flip from the current frame);
  cancel on release (also outside the window), `Focused(false)`,
  `ScaleFactorChanged`, `CloseRequested`, maximise/fullscreen, UI Scale change,
  `Escape` or an `outer_position()` error. Resize zones are the outer frame ring
  (cells with no `widget_id`); a cell with a `widget_id` always wins; inactive
  while maximised or fullscreen. Safe `winit` API only: no `unsafe`, no raw
  handles, no platform shims. Known limitation (reported, not hidden): west and
  north zones need two non-atomic calls; they are verified manually
  (`README_UI.md` §25.1) and any failure is reported to the human.
- Pure, testable logic in `newell-ui` (no `winit` types): `resize_zone`
  (zone from pixel, surface size and metrics) and `resize_target` (snapshot +
  screen pointer + metrics + limits → origin and size). The `winit` calls and
  the small state machine live in `newell-app`.
- Minimum inner size enforcement: set with `set_min_inner_size(PhysicalSize)`
  where supported, **re-applied** after `ScaleFactorChanged` (which also keeps
  the physical surface size through the size writer) and after a monitor change
  (`current_monitor()` checked after `Moved`/`ScaleFactorChanged`) and by every
  transition. The applied result is never assumed.
- Layout composes `max(82, ⌊w/W⌋) × max(30, ⌊h/H⌋)` cells; a surface smaller than
  the minimum (asynchronous resize, rounding at a fractional scale factor, an OS
  that ignores the minimum) is cropped by the renderer, with no panic, no
  zero-size surface and no change of ratio.
- **Post-creation and post-transition verification:** read back inner size,
  outer size, outer position and current monitor; treat the first matching
  `Resized` event as authoritative; at most one corrective request, then log once
  if the window is smaller than the minimum or its outer rectangle exceeds the
  usable rectangle. If even `k = 2` does not fit the monitor (`k_fit < 2`), keep
  `k = 2`, keep 82 × 30 and log the physical limitation once per monitor
  snapshot.
- Pointer events use physical coordinates only (macOS Retina: no `scale_factor`
  arithmetic in the grid path).
- Not in scope: the View menu UI Scale selector, Theme row and `Fullscreen` menu
  item (M2.18). M2.17 contains no widget for the preference.

Acceptance:

- **Derivation unit tests (pure arithmetic).** For each monitor of the
  `ARCHITECTURE.md` §3.1.1 example table (1366×768, 1920×1080, 1920×1200,
  2560×1440, 2560×1664, 3024×1964, 3840×2160), both modes and all five
  percentages, `effective_k` equals the table; in particular 2560×1664 at 100%
  → `k = 5` (20×45) and 1920×1080 at 100% → `k = 2` (8×18); the unknown-monitor
  case; `k_fit < 2` runs at `k = 2`; the reference is 2 for every `k_fit ≤ 3`,
  `min(k_fit, 5)` above, and the derivation reads no monitor name (a test passes
  the same size with different names and gets the same result); `k_req` is monotone in the percentage; no
  rounding tie exists (exhaustive over reference 2…5 × five percentages);
  `effective_k` never leaves 2…7 and never exceeds `k_cap`; several percentages
  clamp to the same `k` on a small monitor (documented behaviour); every `k` gives
  exactly `W = 4k`, `H = 9k` (ratio 4:9).
- **Preference unchanged:** a sequence of monitor, resolution, display-scale and
  fullscreen changes leaves `preferred_scale_percent` unchanged and returns to the
  original `effective_k` when the original inputs return.
- **Transition tests.** Metrics, atlas, `CellGrid`, GPU buffers, minimum size and
  pointer mapping carry the same `k` after every transition (no observer sees a
  mixed state); a failure injected in each Phase A step leaves the old state
  intact and is not retried until an input changes; the atlas is rebuilt only
  when `k` changes; a hit-test round trip is correct on the new metrics; the
  stored pointer position is cleared and a pending press is cancelled and its
  release ignored.
- **Oscillation tests (event-sequence simulation, no real window).** The echo
  events of a transition (`Resized`, `Moved`, `ScaleFactorChanged`) start no
  second transition; the same input tuple twice is a no-op; a monitor change
  during a simulated drag is applied once after 150 ms; a move back and forth
  across the boundary during one drag gives at most one transition; at most one
  corrective request per input tuple; an OS that ignores or adjusts the request
  ends in logged acceptance, not a loop.
- **`fit_window` tests.** Moving a 100 × 40 grid from `k = 5` to `k = 2` on
  1920×1080: columns/rows clamped to whole cells inside the usable rectangle,
  never below 82 × 30; the rectangle is moved the minimum distance; return to the
  larger monitor restores the desired grid; the no-fit case; negative origins and
  `i32`/`u32` extremes saturate without panic; the MenuBar row stays inside the
  monitor; fullscreen enter/exit restores the windowed grid; fullscreen refused
  below 656 × 540.
- Initial and minimum sizes equal `328k × 270k` for the derived `k` and are
  passed as `PhysicalSize`.
- Layout never composes a grid below 82 × 30 and fails closed (clamps/crops) on a
  smaller resize request; a surface of exactly `328k × 270k` gives exactly
  82 × 30 and one pixel less still composes 82 × 30.
- A `NotSupported` result from `drag_resize_window` is handled without panic,
  logged once per window, and switches that window to the application-managed
  resize; `Ignored`/`Os` errors are logged and do nothing; `drag_window` likewise.
- `resize_zone` tests: each of the 8 zones and the interior at k = 2, 3, 5, 7; the
  leftover pixels right of/below the last cell belong to east/south; a surface
  cropped below the minimum; a corner is exactly one cell; a cell carrying a
  `widget_id` (including `[-] [□] [X]`, a popup clipped over the ring, a menu
  item in row 1) never yields a zone.
- `resize_target` tests: east/south/south-east change only the size and keep the
  origin; west/north keep the opposite edge fixed; sizes are whole cells and
  never below 82 × 30 (the origin stops moving at the minimum); the surface
  limit; negative origins and `i32`/`u32` extremes saturate without panic; the
  result is a pure function of the snapshot and pointer (same inputs, same
  output regardless of previously applied sizes).
- State-machine tests: a press in a zone sets no widget pressed state; the
  release is consumed and never activates a control; each cancel trigger ends the
  resize and sends no further request; no request is sent while the snapped
  target is unchanged; inactive while maximised or fullscreen; a press on a
  MenuBar inert cell calls `drag_window`, not a resize.
- The macOS manual checklists of `README_UI.md` §25.1 (edge resize) and the
  multi-monitor checklist (MacBook Air Retina ↔ external 1920×1080) are run and
  their result is recorded in the Executor report; unresolved items are reported
  as limitations.
- Window controls and drag/resize regions resolve by O(1) division;
  `[-] [□] [X]` cells keep their documented positions in an 82-column grid.

### M2.18 — MenuBar, UI Scale selector and Light/Dark theme controls

Scope (user-facing controls only; every `k`, monitor, window-fit and transition
rule belongs to M2.17):

- MenuBar File/Edit/View/Help with the exact reference menus, including the
  View menu and the space-only Help popup in `Application_Menu_Bar.txt`.
- `UI Scale ◀ 100% ▶`: a cyclic selector over the five preference values
  60/80/100/120. It **shows and stores `preferred_scale_percent`**, never the
  derived `effective_k`, so it reads `100%` on every monitor. `◀`/`▶` wrap; **no
  value is skipped or hidden**, even when the current monitor clamps several to
  the same `k`. A step stores the new preference and calls the M2.17 transition;
  if `effective_k` does not change nothing visible happens and no transition runs.
  The control contains no `k` arithmetic, no monitor logic and no window fit.
- `Theme [ ◐ Light ]` / `[ ◑ Dark  ]` toggles the uniform pair; no atlas or
  cell-buffer regeneration. Light is the initial theme of every window.
- `Fullscreen` asks the M2.17 fullscreen gate (full monitor, refusal logged once
  when 656 × 540 does not fit; leaving restores the windowed fit); the item adds
  no sizing logic.
- View items `Tree panel`, `Task bar`, `Status bar`, `Navigation cube`,
  `Fullscreen` act on UI state. `Perspective`, `Shaded`, `Grid Overlay`,
  `[N]ormal to`, `Reset Cam` hold UI state only; there is no camera before M3.
- Open menus stay open and are recomposed at their new position after any
  transition (menu state survives the M2.17 transition).

Acceptance:

- The row shows the stored preference on both a Retina and a 1080p monitor and
  does not change when the window moves between them (stub monitor snapshots in
  tests, manual check with real monitors).
- `◀`/`▶` visit all four values in order and wrap, on a monitor that maps all of them
  to one `k` (1920 × 1080: `k` = 2/2/2/2, so every step changes only the
  stored preference and runs no transition).
- A step that changes `effective_k` calls the M2.17 transition exactly once;
  a press cancelled by it does not activate on release; a second click acts on
  the control now under the pointer.
- Theme change leaves the cell buffer byte-identical and the atlas untouched.
- Snapshots contain the reference rows `│ UI Scale ◀ 100% ▶ │` and
  `│ Theme [ ◐ Light ] │` (Dark: `[ ◑ Dark  ]`) and the Help popup of
  `Application_Menu_Bar.txt` row for row; the five `UI Scale` states of
  `Application_Menu_Bar.txt` render exactly.
- Fullscreen item: refused below 656 × 540 with one log entry; otherwise the
  M2.17 derivation applies (1920 × 1080 → `k_fit(fs) = 4`).

### M2.19 — CommandBar and centered menus

**Reference update:** the current `Command_Bar.txt` supersedes the historical
menu list below. Use Workbench, Datum, Create, Shape, Modify, Pattern and
Utilities, with the exact entries and separators in that file. Every popup is
19 cells wide (Workbench 19 × 11 at col 31, row 9; Datum 19 × 6 at col 31,
row 12 on an 82 × 30 grid). Boolean now belongs to Modify; existing Action Box
behavior and keyboard aliases are preserved. The older CommandBar row in
`Interface_Exemple.txt` is superseded; its other rows are unchanged.

Scope: CommandBar row and the Workbench/Sketch/Create/Operate/Pattern/Refine/
Construct/Inspect menus exactly as in `Command_Bar.txt` (the `Sketch` popup
`╭─Sketch─────────╮ / │ [N]ew Sketch   │ / ╰────────────────╯` is UI-only and
mock-only: no sketch geometry), mnemonics (`[E]xtrude`, `[Mi]rror`, …) scoped to
the open popup, and keyboard activation through the same logical state as the
pointer.

Placement: CommandBar popups are centred in the whole composed grid by
`README_UI.md` §29.1:
`col = ⌊(grid_cols − popup_width) / 2⌋`, `row = ⌊(grid_rows − popup_height) / 2⌋`,
with a negative difference clamped to 0 (clipped at the right/bottom edge, never
at the left/top). The popup is composed last, in the foreground. Existing modal
and keyboard routing (M2.8/M2.9) is unchanged. MenuBar popups are not centred.

Acceptance: popup cells equal the `.txt` rows; at 82 × 30 the Workbench popup
(18 × 11) is at col 32, row 9 and the Sketch popup (18 × 3) at col 32, row 13;
a grid smaller than the popup clamps to 0 without panic; covered cells do not
reach the underlying widgets.

### M2.20 — TreeView layout, playhead, separators and Rename

Scope: Features and Elements trees as in `Interface_Exemple.txt` (`╰╴` last
child, `[-]`/`[+]`, `□`/`■`, the `│ ─────────────────── ` playhead separator
row), the Unicode ellipsis truncation of `README_UI.md` §26 (keep the leading
characters, final available cell is `…` U+2026, one scalar = one cell),
whole-cell separators between panels, and the `Rename` modal
(`Interface_Elements.txt`, "Validation Operation Box"; `Enter Name` shares the
same interaction). Selection and expansion are logical state drawn only by the
documented glyphs. `Rename` opens on a **double-click** (left button) of a
user-named Feature or Element row name (`README_UI.md` §23): two presses on the
same name `widget_id` within 500 ms (recognised by the Controller; `winit` 0.30
has no click count), no other target pressed in between; markers, checkboxes,
category headers and the playhead row are not targets. No keyboard shortcut in
M2.20; any later one must be documented in `README_UI.md` first.

Acceptance: tree rows match the reference cell-for-cell at 82 columns, including
the truncated `Sketch_Profile_Ma…` row; `…` is in the atlas for every `k`; the
playhead row is not a selectable feature; Rename starts with the full existing
name (not the `…` text), OK/Cancel restore focus. Double-click tests: two presses
within 500 ms on the same name open the modal; 501 ms, a different row, an
intervening press elsewhere, or a press on `[-]`/`[+]`/`□`/`■`/a header do not;
the first press still only selects.

### M2.21 — Exact Action Box grammar and Extrude

Scope: the 31-cell fixed-width Action Box (`README_UI.md` §2) composed from
semantic state, with variant substitution (§2.4), exact branch topology and the
segmented-selector border topology of `Action_Box_Extrude.txt` /
`Interface_Elements.txt` "Head Action Box variants". The `║` active-segment
separator from M2.7 is replaced by the documented topology; this ends the
temporary accepted deviation (`ARCHITECTURE.md` §3.5). Extrude: every state in
§7–§8 of `README_UI.md`, including the `Selection` row of `Up to`,
`Extend = Through all` and the `Invert` row placed under `Through all`
(`README_UI.md` §8.9). Composition follows `README_UI.md` §2.6: the complete
base boxes are the authority, the `"... variants"` snippets are partial
excerpts, a variant replaces only its own line or branch, and no documented
sibling control (`Profile`, `Symetric`, `2nd side`/`Second end position`,
`Merge with all`/`Targets`, OK/Cancel) is ever deleted. `Extend = Blind` is the
complete base box (`Depth`, `Along`, `Starting Offset`, `End Offset`); `Up to`
and `Through all` are their snippets; an expanded offset that is the last child
uses the documented prefix transformation of §2.6. `Action_Box_Extrude.txt` is
not edited. The open reference points of §2.6 (label drift `Starting
Offset`/`Start Offset`, no `End Offset` under `Up to`/`Through all`, no expanded
`Offset`) are reproduced as written and reported, not resolved here.

Acceptance: composed rows equal the reference rows character-for-character for
all documented states; no undocumented rows appear (§32, §39). Tests: for each
Extend type, each offset state and `Symetric`/`2nd side`/`Merge with all` on and
off, the siblings are present with unchanged text and column; switching `Extend`
never removes `Profile`, `Symetric`, `Second end position`, `Merge with all` or
OK/Cancel; the `Blind` default equals the first base box row for row; last-child
offsets are 31 cells wide and differ from the non-last form only in prefix
cells 1–6; the canonical `.txt` files are byte-identical before and after.

### M2.22 — Revolve, Loft, Sweep and Boolean Action Boxes

Scope: `Action_Box_Revolve/Loft/Sweep/Boolean.txt` through the M2.21 grammar. The
Boolean `Add` footer is now the canonical 31-cell
`│      OK      │    Cancel    │` (README §17); every Boolean row is exactly 31
cells wide.

### M2.23 — Numeric fields and FormulaEditor

Scope: editable `[+|0.0 mm       ☰]` fields (direction prefix, magnitude, unit,
`☰` as its own interaction region), the text-editing caret `▏` and keyboard
editing, the FormulaEditor overlay from M2.12 aligned with
`Interface_Elements.txt`. Formula evaluation stays out of scope. Selection-field
`☰` and the profile handle `☰` keep their different meanings (`README_UI.md`
§5.6).

Caret (`ARCHITECTURE.md` §3.5, `README_UI.md` §5.11): `▏` appears only while a
field is actively edited, is composed into the flat cell buffer (no second draw
call), is drawn in the foreground colour, does not blink, and never changes the
edited text value. It occupies its own cell between characters; with text area
width `N`, caret index `c`, the visible window starts at
`s = max(0, c + 1 − N)`, so a full field scrolls and clips whole cells (suffix
first) and the caret stays visible and never overwrites a delimiter, prefix or
`☰`.

Acceptance: field cells are fixed-width and never resize with the value;
`+|`/`-|`, value and `☰` are separate hit regions; caret tests cover empty text,
caret in the middle, text of `N − 1`, `N` and `N + 1` characters with the caret
at the start and at the end, the ` mm` suffix clipping, and that the committed
value never contains `▏`.

### M2.24 — Live FPS, progress bar and final UI fidelity

Scope: StatusBar `Roue V1* │ Part Design │ Fully Constrained │ 98fps │ Progress [███░ ] 65%`
with a live frame-rate value and the 21 progress states of `Interface_Elements.txt`
(5 cells, 5% steps, `░ ▒ ▓ █`), plus a final cell-for-cell fidelity pass of the
82 × 28 reference at `k = 5` and snapshots at the 82 × 30 minimum.

Challenger findings resolved by the Architect (2026-10-16): quantization, FPS value
rules, input precedence and outside-press handling, as stated below and in
`README_UI.md` §27 and §27.1. `ARCHITECTURE.md` is unchanged.

The frame-rate segment is fixed-width (`README_UI.md` §27): exactly 7 cells,
a 3-cell right-aligned space-padded numeric field then `fps` then one blank, so
the digit count never moves the separators, `Progress …` or the right border.
Value rule (`README_UI.md` §27): a NaN, `+∞`, `−∞` or negative input → `0`; any
other input is rounded to the nearest integer, and a value above 999 → `999`
(saturated, field never widens). At `98` the row equals `Interface_Exemple.txt`.
Other status text stays free text clipped whole-cell at the right border.
Progress cells are `L`-tall block elements, an accepted limitation
(`ARCHITECTURE.md` §3.1).

Progress interaction (`README_UI.md` §27.1, `ARCHITECTURE.md` §3.5 exception):
the whole `Progress […] N%` segment is one click target with one stable
`WidgetId`, found by O(1) cell lookup (no hover, no retained tree), activated on
left-button press, once per press (holding the button never repeats it). Text-cell
updates only, no shader animation, one GPU draw call. Initial 0 %.
The segment is a fixed 21 cells: `Progress [` + 5 bar cells + `]` + one separator
space + a 4-cell left-aligned `N%` field with trailing padding (`0%  `, `5%  `,
`10% `, `65% `, `100%`; `README_UI.md` §24, §27.1). Every value equals the
`Interface_Elements.txt` row once trailing spaces are removed.
Click at 0 % animates to 100 % in 5 s; click at 100 % animates to 0 % in 1 s;
repeat indefinitely; clicks while `elapsed < duration` are ignored. The
displayed value is `floor5(100·t/D)` ascending and `ceil5(100·(D − t)/D)`
descending, with `t = min(elapsed, D)`, `D = duration`, computed in exact integer
arithmetic on the monotonic elapsed time from an injectable controller clock
(`Instant`, never frame count). `floor5(x)` is the largest multiple of 5 `≤ x`
and `ceil5(x)` the smallest multiple of 5 `≥ x`; neither is nearest rounding
(`README_UI.md` §27.1). The 21 states are kept.
Wake-ups use `ControlFlow::WaitUntil(min(next fps tick, next progress step))`.
The 60 FPS M1.3 redraw bootstrap is retired at M2.24 (`ARCHITECTURE.md` §3.5).
Idle progress adds no deadline. The FPS field stays at its independent 5 Hz
refresh. Scope limit: this one control only; no generic animation infrastructure.
Consequence: while idle the redraw cadence is the 5 Hz tick, so a measured
value reads about 5 fps when idle; `98` is an injected snapshot value only.
Input precedence (`README_UI.md` §27.1, which is §25.1 plus the overlay rule): an
active resize owns the pointer and the segment receives no press; a cell carrying
a `widget_id`, including an overlay or popup cell, wins over a resize zone; a
resize-zone press starts a resize without dismissing an open overlay; while an
overlay is open the segment receives no press, and an outside press is consumed
and dismisses the overlay only if that overlay's existing rule dismisses on
outside presses.

Progress acceptance: with a fake clock, a click at 0 % shows `100%` only from
exactly 5 s and a click at 100 % shows `0%` only from exactly 1 s; ascending
intermediate times give `floor5`, descending give `ceil5` (e.g. ascending at
4.99 s shows `95%`, descending at 0.999 s shows `5%`); a unit test asserts `floor5(x) ≤ x`,
`ceil5(x) ≥ x` and that exact multiples are returned unchanged (`floor5(100) = 100`,
`ceil5(0) = 0`), so neither is a nearest rounding; a single jump of the clock
(skipped frames) gives the same result as many small ones; a press at any
instant strictly inside an animation changes nothing; a press and hold
activates once; presses on every cell of the segment (first `P`, brackets,
separator space, digits, `%`, trailing padding) are accepted when idle and the
cells immediately left (` │ `) and right of it are not; an outside press while an
overlay is open is consumed and leaves the progress value and clock unchanged,
and dismisses the overlay only if that overlay's existing rule dismisses on outside
presses (a test overlay with a non-dismissing rule stays open); a resize-zone press
while an overlay is open starts a resize, dismisses nothing and leaves the segment
unchanged;
the segment keeps one `WidgetId` across all 21 states; the 21-cell segment gives
`Progress [     ] 0%  `, `Progress [░    ] 5%  `, `Progress [▒    ] 10% ` and
`Progress [█████] 100%` (each equal to its `Interface_Elements.txt` row after
trailing-space trim), and the separators, the right border and the segment bounds
never move; the wake-up deadline is
`min(fps tick, progress step)`, idle schedules no progress redraw, the FPS
sampling cadence is unchanged by a running animation; no hover state exists.

Acceptance: FPS is measured, not assigned (the composition function takes it as
an input, snapshots inject `98`); table test for `0, 5, 98, 99, 100, 144, 998.5,
999, 1000, 123456, -3, -0.4, NaN, +∞, −∞` giving the 7-cell strings of §27
(`998.5`, `999`, `1000` and `123456` → `999fps `; `-3`, `-0.4`, `NaN`, `+∞` and
`−∞` → `  0fps `), with the cells
outside the field byte-identical across all values; updating it dirties only
the status cells;
every progress state renders exactly; the whole-application snapshot at each
theme matches `Interface_Exemple.txt` at 82 × 28 content and, at 82 × 30, by the
two-step rule of `README_UI.md` §25 (rows 26–27 removed equal the reference; the
two rows equal the blank body-row template); no difference is invented.

---

### M2.25 — ASCII hover highlight

Depends on M2.19–M2.24 (menus, Action Box rows, modals, StatusBar and the
event-driven redraw of M2.24 must exist). Authority: `ARCHITECTURE.md` §3.5
(*ASCII Hover Highlight*), `README_UI.md` §5.12, `Interface_Elements.txt`
("Effet Hover"), `Special_ASCII.txt`.

Scope:

- `hovered: Option<WidgetId>` in the Controller (the only new state; no other
  hover field, no timer, no per-cell flag, no uniform).
- Four transformations, byte-exact, cell count unchanged:
  Menu `│ Bridging Curve  │` → `╞ Bridging Curve  ╡`;
  Parameter `│ │ Along     [Select…       ]│` → `│ ╞ Along     [Select…       ]╡`;
  Checkbox `│ ╰╴□ End Offset              │` → `│ ╘ □ End Offset              ╡`;
  Button `│      OK      │    Cancel    │` → `╞      OK      ╡    Cancel    │`.
- Junction rule (`README_UI.md` §5.12): left slot `│` → `╞`; left slot `╰╴` →
  `╘` + space; right slot `│` → `╡`. Header button boundaries additionally
  accept `┆`; no other glyph is substituted.
  Slots are the innermost `│`/`╰╴` left of the target and the `│` right of it
  (frame, branch, nested connector or the separator between buttons).
- Composition-time only: the composer receives `hovered` as an input and writes
  at most three substituted glyphs while it composes the target's row. Hover is
  never stored in persistent UI state, in the model or in a cell that outlives the
  frame, and the `widget_id`/flags of every cell are identical with and without
  hover. No change to `newell-render`, the glyph atlas, the shader or any
  uniform (`╞ ╡ ╘` are already in the required set).
- Targets: Menu rows, all Action Box parameter/checkbox rows (including cyclic
  selectors), every Action Box header segment, and footer/modal buttons.
  Parameter labels, padding and cyclic value text inherit a hover id from their
  row through at most 31 grid-cell reads; click routing remains unchanged.
  Header buttons substitute their own left/right `│` or `┆` with `╞`/`╡`,
  preserving horizontal active-segment topology.
  Disabled controls and other non-interactive cells never hover; a Menu row with `◀ ▶`, a
  `[ ◐ Light ]` field or a checkbox is a Menu row.

Controller rules:

- Pointer motion: one integer cell resolution and one `widget_id` read (O(1),
  with at most 31 cell reads for an inert Action Box label/value,
  `ARCHITECTURE.md` §3.4), compare with the stored value, request a hover-related
  redraw only when the hovered `WidgetId` changes, including transitions to or
  from `None`. Preserve the existing region-specific `WidgetId` values; do not
  replace them with a row-level key. Moving between two interactive regions in
  the same row may change the hovered id and trigger one redraw without any
  visible glyph change; this redundant redraw is accepted. Motion within the
  same `WidgetId` requests no hover-related redraw.
- `CursorLeft`, a move onto an ineligible cell, overlay opening over the target,
  and resize start clear `hovered` (one redraw if it was `Some`).
- Except for the hover-only Action Box label/value extension above, eligibility
  equals "a left press here would activate this enabled widget now":
  topmost overlay only; menus and modals make covered and outside cells
  ineligible; a cell with a `widget_id` wins over a resize zone; an active
  resize owns the pointer and suppresses hover updates until the first move
  after it ends.
- After each recomposition: one O(1) re-check of the cell under the last known
  pointer position; clear when it no longer carries `hovered`. Never acquire a
  new target without pointer motion.
- Hover never alters press, click, drag, focus, selection or keyboard handling,
  and does not disturb the M2.24 `WaitUntil` schedule (no new deadline).

Acceptance:

- Table test per kind: normal row → hovered row equals the four strings above
  exactly (every character and space); the derived rows of `README_UI.md` §5.12
  (`│ ╞ □ Starting Offset         ╡`, `│ ╘ End       [Select…       ]╡`,
  Cancel hovered, modal OK/Cancel) pass the same test; the set of differing
  cells equals the slots, nothing else.
- Hovered frame vs normal frame: same dimensions, same `widget_id`/flags per
  cell, same hit-test result for every cell.
- Exit restores the normal frame cell for cell in the same frame; a hover→exit
  sequence leaves no residue (frame equals the never-hovered frame).
- Only one hovered id at any time; moving A → B → exit gives 1, 1, 1 redraws;
  moving within A gives 0 extra redraws; exit with `None` gives 0.
- Precedence: with a menu or modal open, covered controls and outside cells never
  hover; a resize in progress clears hover and ignores motion; a popup cell on
  the resize ring hovers; disabled rows and frame cells never hover;
  Action Box labels and cyclic values highlight their own parameter row;
  a recomposition that moves the target from under the pointer clears hover and
  does not acquire another.
- No hover on tree rows, MenuBar/CommandBar labels, window controls,
  the progress segment or modal text fields.
- Snapshots: M2.15/M2.24 snapshots (hover `None`) are unchanged except row 19 of
  the shell, which follows the `README_UI.md` §25 *Row 19* rule; add one hover
  snapshot per kind and one for row 19 (equal to the unmodified reference row).
  Normal-state `.txt` references stay untouched.
- `cargo test`/clippy gate as usual; the diff shows no change in
  `newell-render`, `assets/` or shader code.

Out of scope: hover for any other widget, hover colour or motion, tooltips,
cursor-shape changes, touch hover, keyboard-focus visuals.

---

## 20. Risks and Deferred Work

### 20.1 Cell representation may evolve

The 16-byte target is useful but must not become an artificial constraint.

If clarity/safety require a slightly larger `Cell`, record and justify it.

### 20.2 Box-drawing rendering

Fira Code box-drawing glyphs may or may not join acceptably at Newell's cell metrics.

Do not pre-optimise.

First test the font pipeline.

A procedural fallback is deferred until evidence requires it.

### 20.3 Continuous M1.3 60 FPS loop

The M1.3 frame loop remains bootstrap behaviour.

M2 should not silently redefine the final redraw architecture.

Later event-driven invalidation may reduce unnecessary redraws.

M2.24 is that step: the 60 FPS cadence is retired and the next wake-up is
`min(5 Hz FPS tick, next progress step)` (`ARCHITECTURE.md` §3.5). `Renderer` and
`FramePacer` are reused at 5 Hz.

### 20.4 Public test-only APIs

Do not expose production modules publicly solely for integration tests unless the trade-off is accepted.

Prefer unit tests for private pure logic where appropriate.

### 20.5 Logo atlas

Deferred until actual logo rendering is required.

No placeholder infrastructure is needed now.

### 20.6 Real CAD state

Features, Elements, Extrude parameters, selections and formulas remain mock UI state in M2.

Their real model/kernel binding belongs to later milestones.

---

## 21. Genuine Blockers / Decisions

### Resolved decisions

- UI specification source is now `docs/ui_design/README_UI.md`.
- ASCII states are reference designs under `docs/ui_design/`.
- High-level UI state may use plain structs/enums.
- The flat 1D cell array is the rendered UI representation, not the entirety of Controller state.
- Segmented selectors and cyclic selectors are different semantic components.
- `Solid | Surface` is segmented.
- `New | Add | Remove` is segmented.
- `◀ Blind ▶` is cyclic.
- Fira Code under SIL OFL 1.1 is accepted as a third-party asset.
- `ab_glyph 0.2.32` is the planned font crate, subject to Executor verification.
- No placeholder logo atlas is required.
- No runtime-generated logo atlas is allowed as a substitute for the architecture-defined PNG atlas.
- Logo work is deferred until needed.
- UI Scale is a user preference (60/80/100/120%); the cell size `k` adapts
  automatically to the monitor (`ARCHITECTURE.md` §3.1.1, ADR 0003 D3/D13):
  `preferred_scale_percent`, `monitor_reference_scale` and `effective_k` are
  separate; the View menu always shows the preference; a small monitor may map
  several percentages to one `k`; `k = 2` (8 × 18) is the smallest cell (second
  amendment of ADR 0003, 2026-10-16: Full HD 100% = `k = 2`). The automatic mechanism is M2.17; the View menu
  selector is M2.18. Approved by the human (2026-10-16).
- Frameless window: manual resizing is kept through the application-managed
  resize of M2.17 (native `drag_resize_window` first); west/north zones are a
  reported, manually verified limitation.
- Status-bar FPS is a fixed 7-cell segment (3-cell right-aligned field,
  saturating at 999); the `▏` caret is approved for M2.23 (no blink, composed
  into the cell buffer, no extra draw call).
- The glyph atlas is deterministic (required set per `k`, never modified; `?`
  for everything else); filtered sampling is for logo textures only.
- `Rename` opens on double-click in M2.20; no keyboard shortcut.
- M2.25 hover (approved by the human, `ARCHITECTURE.md` §3.5): glyph
  substitution `╞ ╡ ╘` at composition time on one `hovered: Option<WidgetId>`;
  four target kinds; exact rules in `README_UI.md` §5.12.
- Extrude: complete base boxes outrank partial snippets; siblings are never
  deleted by a variant substitution (`README_UI.md` §2.6).

### Before M2.3

The following files must exist:

```text
assets/FiraCode-Regular.ttf
assets/OFL.txt
assets/README.md
```

### Before implementation of M2.1

No further design blocker is currently known.

The next workflow step should be a Challenger review of this plan.

If the Challenger returns `READY FOR EXECUTOR`, only **M2.1** should be implemented first.

Do not implement M2.1–M2.15 in one Executor pass.

---

## Recommended Execution Order

```text
Architect plan
    ↓
Challenger review
    ↓
Executor M2.1
    ↓
Auditor M2.1
    ↓
accept M2.1
    ↓
repeat per sub-milestone
```

M2.1 is deliberately isolated because `Cell`, `WidgetId`, styles, metrics and the flat buffer become foundations for the rest of Milestone 2.
