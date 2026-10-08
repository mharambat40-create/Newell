# Newell CAD - UI/UX Architecture & Rendering Specs

**Target:** `wgpu` ASCII-Grid UI
**Status:** Active

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
* **Modals & Dropdowns (> 3 options):** Rendered as centered overlays bridging the `Viewport3D`. Require an opaque background and act as a focus trap (halting lower Z-index event propagation).

## 3. Sprite & Atlas Specifications
* **Texture Atlas:** Single RGBA `.png` sprite sheet, constrained to Power-of-Two (POT) dimensions.
* **Sprite Metrics:** $256 \times 256$px per sprite, strictly mapped to a $2 \times 2$ grid cell matrix. Hitboxes correlate exactly to this $2 \times 2$ cluster.
* **Bleed Margin:** Strict 8px internal transparent padding (active artbox: $240 \times 240$px) to prevent mipmapping and texture-bleeding artifacts during downsampling.
* **Alignment:** Absolute centering over the underlying cell matrix. Supports alpha blending and multi-channel coloring.

## 4. Interaction & Rendering Pipeline
* **State Feedback:** Hover/Focus events trigger immediate cell background hue shifts.
* **Z-Index Render Passes (Strict Sequence):**
  1. `Pass 1`: Grid background fills.
  2. `Pass 2`: `Fira Code` typographic glyphs.
  3. `Pass 3`: Atlas sprites (alpha-blended).
  4. `Pass 4`: Modals/Focus Traps (highest Z-index, intercepts UI input).
