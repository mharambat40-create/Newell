# Newell - Project Backlog

## Current checkpoint — 2026-10-20

Claude and Codex can each perform every role using the shared `docs/agents/`
instructions. The 38 previously inherited test failures are resolved:
**737 standard tests and 9 explicitly run GPU tests pass**. Strict Clippy,
formatting, documentation and Rust 1.88 compilation pass. Earlier checkpoints
below are historical. M2.17–M2.18 native resize/cross-monitor checks and M3.10
physical-device/visual acceptance remain open. See the
[workflow and UI correction report](validation/dual-provider-workflow-2026-10-20.md).

## Milestone 1: Skeleton
- [x] M1.1: Initialize Rust workspace, `.claudesignore`, and CI/CD pipeline.
- [x] M1.2: Setup testing architecture workspace-wide (`proptest`, `insta`, `thiserror`) and standard `tests/` directories.
- [x] M1.3: Setup `winit` window and basic `wgpu` surface in `crates/newell-app` and `crates/newell-render` (window with empty grid at 60 FPS).

## Milestone 2: ASCII UI Framework & Application Shell

- [x] M2.1: Define the core `Cell`, style and `widget_id` representation and
  implement the flat 1D UI cell buffer in `newell-ui`.
- [x] M2.2: Implement the character-grid layout engine, including rows,
  columns, rectangular regions, clipping and box-drawing borders.
- [x] M2.3: Load `assets/FiraCode-Regular.ttf` and implement glyph
  rasterization and the glyph texture atlas in `newell-render`.
- [x] M2.4: Implement the `newell-render` ASCII pipeline so that the complete
  UI cell buffer is uploaded and rendered in a single GPU draw call.
- [x] M2.5: Implement O(1) pixel-to-cell hit-testing and sub-cell border
  routing for shared ASCII separators.
- [x] M2.6: Implement the core grid-native interaction state:
  focus, hover, pressed, selected, disabled, expanded and checked.
  The historical shader hover was removed in M2.16; M2.25 uses only
  composition-time Unicode border substitutions.
- [x] M2.7: Implement reusable ASCII widget primitives:
  labels, separators, buttons, checkboxes, segmented selectors,
  cyclic left/right selectors, numeric/text fields and tree rows.
- [x] M2.8: Implement overlay primitives:
  popup menus, modal panels, scrolling selection lists and
  OK/Cancel confirmation areas.
- [x] M2.9: Implement menu and keyboard interaction:
  application menus, contextual command menus, keyboard mnemonics,
  focus traversal and command activation.
- [x] M2.10: Implement the dynamic Action Panel framework.
  The panel must support conditional sections and progressive disclosure
  driven entirely by UI command state.
- [x] M2.11: Implement the reference Extrude Action Panel as a UI-only mock:
  Solid/Surface, New/Add/Remove, Selection, Blind/Up-to/Through-all,
  Depth, Invert, Starting Offset, Symmetric, Second End Position,
  Merge options and OK/Cancel.
  No geometric operation is performed.
- [x] M2.12: Implement the Formula editor and Selection panel as UI-only
  overlays. Formula evaluation and real model selection remain out of scope.
- [x] M2.13: Compose the complete reference Newell application shell:
  application menu bar, contextual command bar, Features tree,
  Elements tree, 3D viewport area, static NavCube placeholder,
  Action Panel and status bar.
- [x] M2.14: Implement responsive character-grid layout and user-resizable
  panel boundaries with whole-cell snapping.
- [x] M2.15: Add deterministic UI-state tests and reference cell-buffer
  snapshots for the application shell, menus and Action Panel variants.
- [x] M2.16: typography (4:9 cells, k = 5), monochrome rendering and hover removal.
- [ ] M2.17: frameless native window and 82 × 30 minimum grid.
- [ ] M2.18: Application MenuBar, UI Scale labels (80/100/120/140%, mapped to rendering factors 60/80/100/120%) and Light/Dark theme controls; automatic checks pass, cross-monitor acceptance remains open.
- [x] M2.19: CommandBar and centered menus.
- [x] M2.20: TreeView layout, playhead, separators and Rename.
- [x] M2.21: exact Action Box grammar and Extrude.
- [x] M2.22: Revolve, Loft, Sweep and Boolean Action Boxes.
- [x] M2.23: numeric fields and FormulaEditor.
- [x] M2.24: live FPS, interactive progress and event-driven rendering; exact
  shell fidelity and runtime timing validated automatically and visually.
- [x] M2.25 — ASCII Hover Highlight. Its M2.24 scheduling prerequisite is now
  implemented; hover regression tests and prior sampled visual checks pass.

## Milestone 3: Viewport & Kernel Boundary
- [x] M3.1: Setup `newell-kernel` crate with BrepKIT dependency and define internal wrapping structures (e.g., NewellSolid) without leaking BrepKIT types.
- [x] M3.2: 3D camera setup and display of a BrepKIT-generated solid through `newell-kernel`.
- [x] M3.3: Map BrepKIT topology to the `wgpu` mesh buffers and implement 3D picking.
- [x] M3.4: Controller-owned f64 camera state and event-driven screen-space yaw/pitch
  orbit with strict Z-up zero roll; the View consumes immutable camera matrices.
  Signed pitch is clamped before rotation, with immediate reversal at the pole margin.
- [x] M3.5: Dynamic pivot from the nearest displayed mesh hit or exposed-view AABB
  centre. Freeze it per gesture and preserve its projected pixel without press-time recentering.
- [x] M3.6: Context-depth middle-button pan, cursor-surface-anchored wheel/trackpad/pinch
  zoom, model-scale limits and unobstructed fit recovery. Preserve perspective, orthographic,
  principal-axis and projection-stable resize behavior; verify with CPU math/input tests.
- [ ] M3.10: Visually accept mouse and trackpad feel, native pinch, natural scrolling,
  Action Box toggles, extreme zoom, pole departure and viewport resizing in the running app.

The historical M3.7–M3.9 navigation items are consolidated into M3.4–M3.6 under
[M3 camera mechanics](plans/M3-camera-mechanics.md). The earlier fixed-target trackball
plan is superseded. CPU geometry queries are O(T) in triangle count; UI hit testing
remains O(1). Exposed-view bounds exclude clipped/covered geometry but do not remove
surfaces hidden behind other model surfaces.

### Mechanics revision — 2026-10-19

Camera state, CPU picking and gestures now belong to `newell-app`; `newell-render`
only consumes matrices. Zero-roll orbit uses a 0.5-degree pole margin. Surface zoom
anchors are frozen for phased gestures, and orthographic lens shift uses affine
translation. Mesh replacement preserves pose and invalidates query/capture state.
The implementation corrections and validation are recorded in the mechanics plan
and [validation report](validation/M3-mechanics-2026-10-19.md). Earlier validation
checkpoints below describe historical implementations, not acceptance of this revision.

You are the Architect for the Newell CAD project. Your role is strictly to write a development plan. Do not write production code.

We have completed M3.1 to M3.3. I now want you to tackle the remaining tasks for Milestone 3: M3.4, M3.5, and M3.6 (Onshape-style Viewport camera mechanics).

Below, I have provided the content of our ARCHITECTURE.md and backlog.md. Read them carefully.

Write a detailed technical plan in markdown format. It must include:

The state variables and math required for the Screen-Space Free Orbit (camera-relative Yaw/Pitch with a strict zero-roll constraint).

The implementation strategy for the dynamic pivot point (raycasting on a geometric hit vs calculating the visible bounding box center for a miss).

The logic for focal-distance proportional Panning and Cursor-targeted Zoom.

The exact file paths to modify. You must respect our strict MVC pattern: newell-app (Controller) handles winit mouse events and state updates, while newell-render (View) consumes this state to update the wgpu camera uniform buffers.

A list of math-focused unit tests to validate the camera quaternion and matrix transformations without requiring a GPU.

Output the plan clearly so I can save it to docs/plans/M3-camera-mechanics.md.

M3.4–M3.9 add event-driven right-button arcball rotation, middle-button
camera-plane pan and cursor-anchored wheel zoom. Camera math is tested without
a window; the full workspace and all hardware-dependent renderer tests pass.
The zoom bounds are recalculated from each uploaded mesh's bounding sphere.

## Milestone 4: CAD Command & Parametric Interaction
- [ ] M4.1: Define the generic command-state/controller interface used by
  contextual Action Panels without coupling `newell-ui` to BrepKIT.
- [ ] M4.2: Bind the Features tree to the real parametric feature model and
  the Elements tree to resulting geometric entities.
- [ ] M4.3: Implement shared selection state between the viewport, trees and
  Selection panel.
- [x] M4.4: Implement parameter values with engineering units and bind numeric
  Action Panel fields to command parameters.
- [ ] M4.5: Implement the expression/formula data model and connect the
  Formula editor to parameter values.
- [ ] M4.6: Implement the real Extrude command using the existing Action Panel:
  Solid/Surface and New/Add/Remove modes.
- [ ] M4.7: Implement Extrude termination modes:
  Blind, Up to and Through all.
- [ ] M4.8: Implement Extrude modifiers:
  Invert, Starting Offset, Symmetric and Second End Position.
- [ ] M4.9: Implement Add/Remove target handling:
  Merge with all and explicit Merge scope.
- [ ] M4.10: Implement command preview, OK/Cancel semantics and rollback of
  uncommitted command state.

### M4.5–M4.10 prototype closure — 2026-10-29

**Implementation complete for prototype scope; acceptance pending.** The human
explicitly deferred the two advanced geometry classes below in the
[dated scope amendment](plans/M4.5-M4.10-extrude-pipeline.md#12-prototype-scope-amendment--2026-10-29).
This is not completion of every original revision-2 requirement. Supported exact
geometry and transaction correctness remain required. Fresh desktop workflows,
regression checks and the finite remaining human-only checklist are in the
[closure record](validation/M4.5-M4.10-extrude-pipeline.md#prototype-closure--2026-10-29).
M4.5–M4.10 checkboxes remain unchecked pending that acceptance; the previously
confirmed M4.4 validation is retained without requesting repetition.

### Deferred advanced Extrude geometry

- [ ] **M4.D1 — Nonlinear curved-support correspondence for Add/Remove.**
  Explicitly deferred, not implemented. Preserve the supported generated-domain
  arrangements. Fixture, pinned-API limitations and future acceptance:
  [C((s+s²)/2) and exact 4L/L results](validation/M4.5-M4.10-extrude-pipeline.md#deferred-nonlinear-correspondence).
  Require a certified coincidence/parameter map, exact manifold result, unique
  retained provenance, downstream mesh, real-worker replay/identity and atomic
  failure/draft/retry coverage before checking this item.
- [ ] **M4.D2 — General trimmed rational targets.** Explicitly deferred where
  the required trim/intersection/finite-hit operations are unestablished.
  [Documented rational patch with a trimmed hole, limitations and future criteria](validation/M4.5-M4.10-extrude-pipeline.md#deferred-trimmed-rational-targets).
  Require actual trim-aware hits/misses (both directions, filled/open profiles),
  exact geometry/topology, stable references/mesh and atomic real-worker/draft
  recovery. Keep the currently supported partial ruled/cylindrical cases green.

### M4.4 verification history — 2026-10-27

- Manual validation: **passed by the human**; the human tested M4.4 and
  confirmed the result was OK.
- Auditor: **green** for revision 2 and the stabilization changes. The final
  workspace run reported 915 tests passed and 14 skipped; workspace check,
  formatting, strict Clippy, documentation tests and `git diff --check` passed.
  No failures were reported. The existing `block v0.1.6` future-incompatibility
  notice remains informational.


## Milestone 5: Complete, responsive parametric Sketcher

Plan: [M5 — Complete, responsive parametric Sketcher](plans/M5-sketcher.md),
revision 2, §9.1. Planning is authorized; production execution follows human
acceptance of M4's real command/parameter/geometry pipeline. Listing these
slices does not approve implementation or establish completion.

- [ ] M5.0: Feasibility and requirements — approved decisions, hardware/fixture
  baselines, rank, cancellation and curve/tolerance capability evidence.
- [ ] M5.1: Model/session/journal — stable IDs, shared data, support/frame,
  Sketch feature, typed edits, undo/redo and versioned data transfer.
- [ ] M5.2: Incremental numerical foundation — dependency graph, sparse solves,
  scaling, bounded service classes and cooperative cancellation.
- [ ] M5.3: Editing shell and navigation — panels, support picker, plane camera,
  input routing, selection and numeric/Formula tools.
- [ ] M5.4: Basic draw and constraints — points, lines, rectangles, polygons,
  circles, arcs, slots and dimensional/geometric relations.
- [ ] M5.5: Regions and selection — exact arrangements, holes/islands,
  open chains, selected-region unions and repair diagnostics.
- [ ] M5.6: Real CAD integration — atomic Sketch commit, region/wire profiles
  for M4 Extrude, downstream replay and profile repair.
- [ ] M5.7: Modify — trim, extend, split, fillet, chamfer, offset and relation
  transfer with atomic undo.
- [ ] M5.8: Patterns/transforms — copy, move, rotate, scale, mirror and
  associative linear/circular patterns.
- [ ] M5.9: Advanced curves — ellipses/arcs, bounded conics, Bézier curves,
  fit splines and exact control refinement.
- [ ] M5.10: Advanced constraints/diagnostics — normal/curvature relations,
  spline tangency, rank/DOF certification and conflict diagnostics.
- [ ] M5.11: References — Projected/Intersect/Insert, linked/copy modes and
  dependency repair.
- [ ] M5.12: Text — supported text entry, exact font contours, placement,
  scaling, conversion and text-profile extrusion.
- [ ] M5.13: Large-sketch usability — filtered/virtual lists, label placement,
  search/isolation, label counts and slice/grid controls.
- [ ] M5.14: Multi-window and stress — conflicts/rebase, late results,
  history/parameter integration, memory and worker fairness.
- [ ] M5.15: Milestone acceptance — all advertised tools, benchmark classes,
  workspace/GPU checks and physical-device review.
