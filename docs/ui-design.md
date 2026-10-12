# Newell CAD - UI/UX Architecture & Rendering Specs

**Target:** `wgpu` ASCII-Grid UI  
**Status:** Active

This document is subordinate to `ARCHITECTURE.md`. Where they conflict, `ARCHITECTURE.md` wins.

This document defines the UI/UX language, layout topology, interaction model, and rendering constraints for Newell. The interface is intentionally **grid-native**: application chrome, controls, borders, menus, trees, contextual panels, and overlays are all represented through the same discrete character-cell system.

---

## 1. Core Design Principles

### 1.1 Grid-Native UI

* **Base Unit:** One discrete ASCII cell, rigidly constrained to a `3:5` (Width:Height) aspect ratio to preserve typographic proportions.
* All UI geometry is expressed in whole cells. Arbitrary pixel-positioned widgets are forbidden.
* OS-window resizing and panel resizing resolve strictly character-by-character.
* UI state may insert, remove, resize, or replace rows and regions, but the resulting interface must always be representable as the same flat cell grid consumed by `newell-render`.
* Colour may reinforce a state, but important state changes must remain understandable from text, glyphs, borders, or layout alone.

### 1.2 Text-First Interaction

Newell favours text, box-drawing characters, compact symbolic controls, and direct keyboard interaction over conventional graphical chrome.

The same logical control should remain understandable with:
* mouse input;
* keyboard focus;
* mnemonic shortcuts;
* colour disabled or unavailable.

### 1.3 Dynamic Context Rather Than Static Forms

Contextual command interfaces are generated from the active command state.

For example, an Extrude operation may expose different sections depending on:
* `Solid` vs `Surface`;
* `New` vs `Add` vs `Remove`;
* termination mode;
* enabled modifiers;
* current selection;
* merge scope.

The `ActionPanel` is therefore not a fixed property form. It is a dynamic projection of command state into grid-native controls.

---

## 2. Application Shell & Layout Topology

The reference Newell application shell is composed of six persistent regions.

### 2.1 `MenuBar`

Top application menu bar.

Typical entries:

```text
File   Edit   View   Help
```

* Fixed height.
* Fluid width.
* Opens grid-native popup menus.
* Supports mouse and keyboard mnemonics.

### 2.2 `CommandBar`

Contextual command bar directly below the `MenuBar`.

Typical command groups:

```text
Workbench   Sketch   Create   Operate   Pattern   Refine   Construct   Inspect
```

* Fixed height.
* Fluid width.
* Content may change with the active workbench.
* Entries may open popup menus or activate commands directly.

`CommandBar` replaces the ambiguous term `ToolBar` where the UI is primarily textual.

### 2.3 `NavigationPanel`

Left-side navigation region.

It contains two distinct document representations:

#### Features

History / parametric-operation oriented tree.

Example:

```text
Features
├─ Sketch_Profile_Main
├─ Hub_Features
│  ├─ Extrude_Base
│  ├─ Sketch_Hole
│  └─ Extrude_Cut_Center
├─ Fillet_Inner
├─ Chamfer_Edges
└─ Pattern_Circular
```

#### Elements

Resulting geometric entities grouped by semantic category rather than feature history.

Example:

```text
Elements
├─ Solid Bodies
│  ├─ Rim_Body
│  └─ Hub_Body
├─ Surfaces
└─ Construct
```

The UI must not assume that `Features` and `Elements` share the same underlying model representation.

The `NavigationPanel`:
* has fluid height;
* has user-adjustable width;
* supports expandable/collapsible tree rows;
* may split its vertical space between the two trees.

### 2.4 `Viewport3D`

Central region that absorbs remaining width and height.

During early UI milestones it may contain only:
* the clear-colour viewport;
* static placeholder content;
* a static NavCube placeholder.

Real camera control, display modes, picking, and geometry belong to later viewport/kernel milestones.

### 2.5 `ActionPanel`

Contextual command panel shown beside or over the viewport when a command requires parameters.

Examples:
* Extrude;
* Revolve;
* Fillet;
* Chamfer;
* Pattern.

The panel:
* represents the active command state;
* may grow or shrink vertically as options are enabled or disabled;
* must use reusable grid-native controls;
* may overlay the viewport when necessary;
* must not become a generic always-visible property inspector.

### 2.6 `StatusBar`

Bottom application state bar.

Typical fields:

```text
Roue V1* | Part Design | Fully Constrained | 98fps | Progress [██████░░] 82%
```

Possible contents:
* document name;
* dirty/modified marker;
* active workbench;
* contextual constraint/status information;
* rendering performance;
* background-task progress.

Fields are contextual and may appear or disappear.

### 2.7 Pane Resizing & Snapping

* Side regions are horizontally resizable through vertical box-drawing borders used as drag handles.
* All resizing snaps to whole columns or rows.
* No sub-cell panel geometry is allowed.
* The viewport absorbs the remaining layout delta.

---

## 3. Component Design System

### 3.1 Typography & Cell Styling

* **Typeface:** `Fira Code` (monospaced).
* Every cell supports independent foreground and background colour data.
* Text alignment must remain exact across rows and columns.

### 3.2 Borders

Persistent pane boundaries use double-line box characters:

```text
╔ ╗ ╚ ╝ ║ ═ ╬
```

Hierarchical content and lightweight internal grouping may use single-line characters:

```text
├─ └─ │ ─
```

Border geometry is part of the interaction language: connected, separated, open, or highlighted borders may communicate state.

### 3.3 Tree Rows

Tree rows support:
* nested indentation;
* expandable/collapsible groups;
* selected state;
* hover state;
* optional icon/logo block;
* strict vertical alignment.

Collapsed/expanded markers may use forms such as:

```text
[+] Group
[-] Group
```

or another consistent grid-native equivalent.

### 3.4 Segmented Selectors

Mutually exclusive choices with a small number of states may be rendered as connected segments.

Examples:

```text
Solid | Surface
New | Add | Remove
```

The active state should be communicated structurally through borders, separators, glyphs, or fill treatment, not solely through colour.

### 3.5 Cyclic Selectors

Enumerated values that are naturally stepped through use:

```text
◀      Blind      ▶
```

Examples:

```text
Blind
Up to
Through all
```

* Left/right arrows cycle values.
* The centre value is focusable.
* Double-clicking the centre may step to the next state where appropriate.
* Keyboard left/right should provide the equivalent interaction.

### 3.6 Checkboxes / Boolean Controls

Boolean state uses an explicit glyph representation.

Example:

```text
□ Starting Offset
■ Starting Offset
```

Enabling a boolean may reveal dependent controls directly below it.

Example:

```text
■ Starting Offset
  ◀      Blind      ▶
  Depth [0.0 mm   ☰]
```

This progressive-disclosure behaviour is a core UI pattern.

### 3.7 Numeric Fields

Engineering values display their units explicitly.

Example:

```text
Depth [0.0 mm   ☰]
```

A numeric field may contain:
* a literal value;
* an expression-backed value.

The UI must visually distinguish those states.

The expression/menu affordance opens the Formula Editor.

### 3.8 Selection Fields

Compact selection fields communicate cardinality.

Examples:

```text
[none       ]
[1 Element  ]
[4 Elements ]
```

Activating the field may open a larger `SelectionPanel`.

### 3.9 Buttons & Confirmation Areas

Commands use compact textual actions.

Examples:

```text
OK | Cancel
Apply | Cancel
```

Confirmation controls must have explicit focus and pressed states.

### 3.10 Popup Menus

Top-level and contextual command entries open grid-native popup menus.

Example:

```text
Create
├─ [E]xtrude
├─ [R]evolve
├─ [L]oft
└─ [S]weep
```

Mnemonic letters may be exposed directly in labels.

Popup menus:
* are positioned on the character grid;
* trap relevant keyboard focus while open;
* block pointer events to covered controls;
* remain part of the same UI cell buffer.

### 3.11 Modal / Overlay Panels

Complex interactions may use opaque overlays over `Viewport3D`.

Examples:
* `SelectionPanel`;
* `FormulaEditor`;
* command-specific transient dialogs.

Overlays act as focus traps in `newell-ui` (Controller). They are **not** separate render passes.

---

## 4. Contextual Action Panel Model

### 4.1 General Rule

An `ActionPanel` is rendered from command state, not from a fixed rectangle of always-visible controls.

The conceptual flow is:

```text
Command State
     │
     ▼
UI Description / Layout State
     │
     ▼
Flat Cell Buffer
```

UI-only state is owned by the Controller side (`newell-ui` / `newell-app`) and rendered by `newell-render`.

### 4.2 Extrude Reference Panel

The Extrude mock is the reference interaction example for the UI framework.

#### Primary type

```text
Solid | Surface
```

#### Result relationship

```text
New | Add | Remove
```

#### Common parameters

Possible common sections include:
* Selection;
* termination mode;
* Depth;
* Invert;
* Starting Offset;
* Symmetric;
* Second End Position;
* OK / Cancel.

#### Conditional parameters

`Add` and `Remove` may additionally expose:
* Merge with all;
* Merge scope.

The panel must insert or remove these sections according to state rather than reserving permanent blank space.

### 4.3 Progressive Disclosure

Controls that are irrelevant to the active command state are omitted from layout.

Example:

```text
□ Starting Offset
```

may become:

```text
■ Starting Offset
  ◀      Blind      ▶
  Depth [0.0 mm   ☰]
```

The grid is recomputed in whole rows after such changes.

---

## 5. Selection Panel

Complex selection is represented through a dedicated overlay.

Example:

```text
Selection
┌───────────────────────┐
│ □ Rim_Body           ▲│
│ ■ Hub_Body           ┃│
│ □ Inner_Surface      ▼│
└───────────────────────┘
        OK | Cancel
```

The panel supports:
* single or multiple selectable entries;
* checked / unchecked state;
* scrolling;
* confirmation;
* cancellation;
* keyboard focus and navigation.

During the UI-only milestone, entries may come from deterministic mock data.

---

## 6. Formula Editor

Numeric parameters may be backed by expressions.

Reference presentation:

```text
Formula
Fx = (#EntraxeA - 2mm) / 5 * 3
     * sin(#Angle1 - #Angle2 + 3deg)

        OK | Cancel
```

The Formula Editor:
* is a focused overlay;
* provides explicit OK / Cancel actions;
* distinguishes literal values from expression-backed values in the calling field;
* may use deterministic mock text during the UI milestone.

Expression parsing, unit evaluation, dependency resolution, and model recomputation are **not** part of the initial ASCII UI milestone.

---

## 7. Sprite & Atlas Specifications

### 7.1 Texture Atlas

Tool logos use a single RGBA `.png` sprite sheet constrained to Power-of-Two (POT) dimensions.

### 7.2 Sprite Metrics

* Source slice per sprite: `256 × 256 px`.
* Visible art: `240 × 240 px`.
* Internal transparent padding: `8 px` on each edge.

This protects against mipmapping / texture bleeding during downsampling.

### 7.3 Grid Allocation

Per `ARCHITECTURE.md`:

* Major tools: **5 columns × 3 rows**.
* Minor tools: **3 columns × 2 rows**.

The allocated cell block is the interaction hitbox.

### 7.4 Alignment

* Sprite centering is performed by the fragment shader using **Scale to Fit**.
* CPU code must never calculate sub-pixel centering offsets.
* Alpha blending and multi-channel colour treatment are supported.

---

## 8. Interaction Model

### 8.1 Widget State

Reusable controls may expose states such as:

```text
Normal
Hovered
Pressed
Focused
Selected
Disabled
Expanded
Checked
```

The exact Rust representation may use separate enums/flags if that produces a cleaner model; this list describes interaction semantics, not a mandatory single enum.

### 8.2 O(1) Hit-Testing

Pointer hit-testing is derived directly from pixel position and grid metrics.

Conceptually:

```text
column = mouse_x / cell_width
row    = mouse_y / cell_height
```

The Controller maps the resulting cell directly to its `widget_id`.

Iterating through widget bounding boxes for normal grid hit-testing is forbidden.

### 8.3 Shared-Border Routing

Where an ASCII separator visually belongs to adjacent controls, the Controller uses sub-cell modulo logic to determine which side receives the interaction.

This preserves O(1) hit-testing while allowing borders to behave as intuitive hit targets.

### 8.4 Hover Rendering

Per `ARCHITECTURE.md`:

* CPU computes logical hover state.
* Fragment shader renders hover appearance.
* Text hover uses colour shift and left-border emphasis.
* Logo hover uses a brightness multiplier (for example `color * 1.4`).
* CPU code never computes fade animations or colour interpolation.

### 8.5 Keyboard Interaction

The UI framework should support:
* focus traversal;
* directional navigation where appropriate;
* menu mnemonics;
* activation through keyboard;
* cyclic selector left/right control;
* modal focus trapping;
* Escape / Cancel semantics where defined.

Keyboard and pointer interaction must mutate the same logical widget state.

---

## 9. Rendering Pipeline

### 9.1 Flat UI Buffer

`newell-ui` produces one flat 1D array of cells.

Each rendered cell carries the data required for the renderer, including:
* glyph;
* foreground/background styling;
* widget identity / state encoding required by the approved architecture;
* logo-atlas reference where applicable.

The UI must not require a DOM-like render tree.

### 9.2 Single Batched UI Draw Call

`newell-render` draws the complete ASCII UI overlay in a single `wgpu` draw call.

The exact atlas-binding mechanism must remain compatible with the single-draw-call constraint.

### 9.3 Layer Ordering

Logical UI composition is flattened before rendering.

Expected visual ordering is conceptually:

1. cell backgrounds / pane fills;
2. glyphs and box-drawing characters;
3. logo sprites;
4. popup menus;
5. modal / overlay panels.

This ordering does not imply multiple UI render passes.

### 9.4 3D Viewport

The 3D viewport is rendered by its own pass before the ASCII overlay.

It is not part of the single UI draw call.

---

## 10. Milestone 2 Reference UI State

Milestone 2 should culminate in a deterministic, interactive **UI-only reference shell** that reproduces the intended Newell interface without implementing CAD geometry semantics.

A representative reference state should contain:

```text
Document: Roue V1
Workbench: Part Design

MenuBar:
File | Edit | View | Help

CommandBar:
Workbench | Sketch | Create | Operate | Pattern | Refine | Construct | Inspect

NavigationPanel:
- populated Features tree
- populated Elements tree

Viewport3D:
- clear-colour viewport
- static NavCube placeholder

ActionPanel:
- Extrude mock
- Solid selected
- Add selected
- deterministic selection
- Blind termination
- numeric depth field
- conditional modifier sections
- OK / Cancel

StatusBar:
- document name
- dirty marker
- active workbench
- representative contextual status
```

### 10.1 Expected Milestone 2 Behaviour

Milestone 2 may implement:
* hover;
* focus;
* button presses;
* menu opening/closing;
* tree expand/collapse;
* segmented selection;
* cyclic selectors;
* checkboxes;
* text/numeric-field interaction;
* dynamic ActionPanel sections;
* SelectionPanel UI;
* FormulaEditor UI;
* whole-cell panel resizing;
* deterministic mock data.

### 10.2 Explicitly Out of Scope for Milestone 2

Milestone 2 must **not** require:
* BrepKIT;
* real parametric features;
* Boolean geometry;
* real Extrude execution;
* real model recomputation;
* formula evaluation;
* topology naming;
* actual geometric selection;
* functional camera / NavCube behaviour;
* real viewport display-mode changes.

Those later systems may bind to the UI states established here without changing the UI grammar.

---

## 11. Reference-State Testing

The ASCII UI is well suited to deterministic snapshot testing.

The UI framework should be able to construct reference states from mock data and compare their resulting logical cell buffers against committed snapshots.

Useful snapshot states include:
* base application shell;
* open application menu;
* open command menu;
* Extrude `New`;
* Extrude `Add`;
* Extrude `Remove`;
* Starting Offset disabled/enabled;
* SelectionPanel open;
* FormulaEditor open;
* collapsed / expanded navigation trees;
* narrow and wide panel layouts.

Snapshot tests should validate **logical cell-buffer output and interaction state**, not GPU pixel output, unless a later milestone explicitly introduces visual-regression testing.
