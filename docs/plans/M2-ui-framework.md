# Newell — Milestone 2 UI Framework Plan

**Milestone:** M2 — ASCII UI Framework & Application Shell  
**Status:** Planned  
**Implementation scope:** `newell-ui`, `newell-render`, `newell-app`  
**Architecture authority:** `ARCHITECTURE.md`

This document is an implementation plan. It does not override `ARCHITECTURE.md` or the UI design specification.

---

## 0. Specification Sources and Precedence

Milestone 2 uses the following source order:

1. `ARCHITECTURE.md`
2. `docs/ui_design/README.md`
3. Relevant ASCII reference states under `docs/ui_design/`
4. This implementation plan
5. Existing implementation details, where they do not conflict with the above

If an ASCII reference conflicts with `docs/ui_design/README.md`, the README wins.

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

Shared component semantics belong in `docs/ui_design/README.md`, not duplicated across every ASCII mock-up.

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

The intended logical ratio remains 3:5.

### 6.5 Hit testing

```rust
pub fn hit_test(
    position_px: UiPoint,
    metrics: GridMetrics,
    grid: &CellGrid,
) -> Option<WidgetId>;
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

Do **not** commit to a procedural box-drawing rasterizer before a visual test demonstrates that Fira Code cannot produce acceptable joins at Newell's 3:5 cell metrics.

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
- rasterise only required glyphs initially;
- maintain deterministic glyph atlas placement;
- support ASCII UI and required Unicode symbols used by the reference designs.

The glyph atlas is distinct from the future logo atlas.

### 12.5 Logo atlas

No logo is required by the current near-term M2 implementation.

Therefore:

- do not create a placeholder logo atlas;
- do not generate a logo atlas at runtime;
- do not add a PNG decoding dependency for a placeholder;
- defer logo-atlas implementation until the first sub-milestone that actually renders logos.

When introduced, the logo atlas must follow `ARCHITECTURE.md` exactly.

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
docs/ui_design/README.md
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

- UI specification source is now `docs/ui_design/README.md`.
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
