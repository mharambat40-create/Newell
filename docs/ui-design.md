# Newell CAD - UI/UX Architecture & Rendering Specs

**Target:** `wgpu` ASCII-Grid UI
**Status:** Active

This document is subordinate to `ARCHITECTURE.md`. Where they conflict, `ARCHITECTURE.md` wins.

This document dictates the strict UI/UX constraints, layout topology, and rendering logic for Newell.

## 1. Grid Topology & Layout Architecture
* **Base Unit:** Discrete ASCII cell, rigidly constrained to a `3:5` (Width:Height) aspect ratio to prevent typographic distortion.
* **View Structure (6-Pane System):**
  * `MenuBar` (Top): Fixed height, fluid width.
  * `ToolBar` (Below Menu): Fixed height, fluid width.
  * `TreePanel` (Left): Fluid height, user-adjustable width.
  * `TaskPanel` (Right): Fluid height, user-adjustable width.
  * `StateBar` (Bottom): Fixed height, fluid width.
  * `Viewport3D` (Center): Fluid width/height (absorbs remaining delta).
* **Resizing & Snapping:** All layout mutations (OS window resize or user drag) resolve strictly character-by-character (no sub-cell rendering). Side panels (`TreePanel`, `TaskPanel`) are horizontally resizable via vertical border (`║`) drag-and-drop handles.

## 2. Component Design System
* **Typography:** `Fira Code` (Monospaced). Every grid cell supports independent foreground (glyph) and background (fill) color encoding.
* **Pane Borders:** Rendered exclusively via double-line box characters (`╔`, `╗`, `╚`, `╝`, `║`, `═`, `╬`).
* **Feature Tree (Hierarchy):** Uses single-line characters (`├─`, `└─`, `│`). Collapsed/Expanded states use `[-]` and `[+]`. Nodes must maintain strict vertical alignment.
* **Inline Selectors (≤ 3 options):** Rendered as `◀ {Value} ▶`. Arrow click events trigger cyclic state mutations (carousel effect). Double-click events on the central `{Value}` directly step to the next available state.
* **Modals & Dropdowns (> 3 options):** Rendered as centered overlays bridging the `Viewport3D`. Require an opaque background and act as a focus trap (halting lower Z-index event propagation). Focus trap is implemented in `newell-ui` (Controller). It is not a render pass.

## 3. Sprite & Atlas Specifications
* **Texture Atlas:** Single RGBA `.png` sprite sheet, constrained to Power-of-Two (POT) dimensions.
* **Sprite Metrics:** $256 \times 256$px source slice per sprite (visible art $240 \times 240$px). Major tools occupy **5 columns × 3 rows**; minor tools occupy **3 columns × 2 rows** (`ARCHITECTURE.md` §3.3). Hitboxes are the allocated block. Centering is done by the fragment shader ("Scale to Fit"); CPU code never computes sub-pixel offsets.
* **Bleed Margin:** Strict 8px internal transparent padding (active artbox: $240 \times 240$px) to prevent mipmapping and texture-bleeding artifacts during downsampling.
* **Alignment:** Absolute centering over the underlying cell matrix. Supports alpha blending and multi-channel coloring.

## 4. Interaction & Rendering Pipeline
* **State Feedback:** Hover is computed on the CPU as a `WidgetState` (`Normal | Hovered`) and rendered in the shader: text shifts colour and the leftmost pixels of the cell are highlighted; logos use a brightness multiplier (`color * 1.4`). The CPU never computes fades or colour transitions. (`ARCHITECTURE.md` §3.5.)
* **Single batched draw call:** The Controller writes one flat 1D cell array (`ARCHITECTURE.md` §3.4). `newell-render` draws the whole ASCII overlay in one `wgpu` draw call. Layers are ordered by their position in that array, from back to front: grid background, glyphs, logo sprites, then modals/dropdowns. Modals block input through the Controller's focus trap, not through an extra render pass.
* The 3D viewport is drawn by its own pass before the overlay. This is not part of the UI draw.
