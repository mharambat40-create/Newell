# ADR 0007: Dedicated viewport navigation widgets

**Date:** 2026-10-27  
**Status:** Draft implementation decision; the human's current Architect request
explicitly authorizes the scoped architectural/UI amendments already applied
in `ARCHITECTURE.md` §3.7 and `README_UI.md` §41. This is not approval to implement
plan revision 2.  
**Agent author:** Architect (Codex).

## Context

The existing cube is an inert ASCII placeholder. The human requests a colored
axis widget and a beveled interactive cube. Blanket palette, font filtering,
ASCII routing and no-extra-pass constraints would otherwise forbid them.
Accepted ADR 0002 requires a new ADR for added rendering passes. Preserve prior
accepted ADRs; the explicit §3.7 scope supersedes only their exclusion of this
navigation path. No second mutable scene camera is permitted.

## Options considered

1. ASCII representations: preserve the old path but fail the requested visuals.
2. Reuse CAD display pipelines: couples widgets to display mode, selection,
   scene depth and geometry; fails material and input independence.
3. Dedicated bounded navigation overlay: fixed UI-owned shapes, app-owned
   camera commands and renderer-owned offscreen graphics; chosen.

## Decision

Use option 3. The existing font and dependencies suffice. A small linear
premultiplied color target, isolated depth and 4x MSAA render fixed widgets;
a dedicated composite follows the scene and precedes the unchanged one-draw
ASCII overlay. View Navigator shows one selected representation: Gizmo (default) or Cube;
its checkbox conditionally reveals the child selector. At most three dedicated
draw calls for Cube (mesh, batched labels, composite), two for Gizmo (axis quad,
composite), and zero when hidden/suppressed. No blur/refraction,
GPU picking, retained hierarchy, hover material, animation or document entity.
The axis shader uses bounded analytic depth composition for intersecting planes.
CPU and GPU share pure widget geometry descriptors, basis and physical layout.
Only app `Navigation` mutates the authoritative `Camera`.

## Consequences

- Target/projection/zoom are preserved across 26 deterministic named directions.
- Widget material colors are independent of Color, themes, model display styles
  and selected-model brightness; transparent edges still blend with background.
- Additional targets and draws have a bounded cost; they are accounted for
  separately from the single ASCII UI draw and allocated only at size changes.
- The user's follow-up resolves the initial simultaneous-widget/menu conflict:
  a View Navigator checkbox and conditional Gizmo/Cube selector,
  default Gizmo; visibility toggles retain the selected kind.
- Undersized windows temporarily suppress the chosen widget without switching
  kind; whole-tile Action Box suppression avoids moving existing CAD controls.
- Edge-on labels can be omitted; actual font readability, GPU support and
  device interaction remain Executor/manual acceptance work.
- New exceptions apply only to these two widgets. Earlier accepted ADRs are
  untouched. No other milestone, role or publication is started.
