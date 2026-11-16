# ADR 0011 — Principal-plane Sketch strokes

Date 2026-11-09. Status proposed for implementation approval.

ADR 0002 requires a decision for a new render pass. ARCHITECTURE.md §3.8
authorizes a narrowly scoped viewport stroke batch for visible saved
Line and Rectangle recipes and the active draft. Reuse the existing procedural grid pass with a frame
uniform. Keep one ASCII draw, current menu and panel occlusion, and ordinary
B-Rep rendering. Submit at most one additional batched stroke draw per exposed
viewport rectangle. A completed recipe draws in this pass when checked in
Features. An active draft replaces that recipe in the batch while editing.
Recipes contribute no B-Rep outputs.

Renderer inputs are immutable endpoints, viewport metrics, and selection flags.
Use theme Text ink and the widths specified in README_UI.md §42. The renderer
owns no tool state, picking or camera mutation. Stroke geometry remains above
background CAD depth and beneath opaque UI. No atlas, dependencies, GPU
readback, or changes to previously accepted ADR files are required.
