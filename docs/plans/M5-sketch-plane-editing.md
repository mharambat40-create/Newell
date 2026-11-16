# M5 — Principal-plane Sketch editing

Revision 1 · 2026-11-09 · Architect / Codex  
Status: proposed for Executor approval.

## Contract

Implement [ARCHITECTURE.md §3.8](../../ARCHITECTURE.md#38-scoped-principal-plane-sketch-editing-2026-11-09),
§§4.4–4.5, [README_UI.md §42](../ui_design/README_UI.md#42-principal-plane-sketch-editing)
and the [stroke ADR](../adr/0011-principal-plane-sketch-strokes.md). Approval
of this revision includes adoption of that scoped rendering decision. The
human authorized the binding documentation amendments. This plan requires
separate Executor approval. The [entry-shell plan](M5-sketch-origin-plane.md)
remains the contract up to setup confirmation.

Starting state is the existing checkout on `main` at `310f382` in
`/Users/martin/Documents/Perso/Projet/Newell` with an empty Git index. The
unstaged entry-shell implementation and specs, adaptive grid edits, and
untracked `Picto/` and plans are inputs to preserve. Recheck status before
editing. Architect ran only documentation and Git checks.

Deliver Line and axis-aligned Rectangle with viewport previews, validation and
private cancellation. Reopen a saved recipe by stable Features identity and
edit it using drawing or Select/Delete. Save means commit into the open
document. File serialization, region/profile publication, downstream Sketch
references, constraint solving, other Sketch tools and general M5 adoption are
outside this revision. There are no dependency, MSRV, or license changes.
Retain Rust 1.88, existing SolidCore isolation and AGPL headers.

## Implementation

1. **Model and publication.** Add a model-owned principal support enum and
   recipe with `Line { start, end }` and `Rectangle { min, max }` in local mm.
   Use the binding §3.7 basis and derive Rectangle sides. Keep one stable
   primitive key per draft item, including an atomic Rectangle group. Use the
   same finite and degeneracy validation on placement and commit. This slice
   uses the M5 geometric threshold `max(1e-7 mm, 1e-12 * local_extent)` with
   local extent taken from the primitive's spans. Use overflow-safe length and
   coordinate calculations and reject unrepresentable input. Keep the
   model-free UI support projection and convert at the App boundary.
   Add `Feature::Sketch` with a name and recipe and empty B-Rep outputs.
   Update model history, rename, and exhaustive replay paths so Sketch is a
   zero-output history operation. Insert at the captured playhead gap under
   the current document identity and revision. Invalid or stale anchors leave
   the draft open. Model insertion checks ID capacity, validates the whole
   candidate and picks the first collision-free exact `SketchN` at or above a
   document-owned checked ordinal initialized to one. Update the ordinal
   only after successful insert. Preserve it across clones, prepared document
   replacement and replay. Replacement checks the target ID and revision
   and preserves name, ID and history position. Reuse DocumentSession's
   synchronous revision check and notification pattern for these recipe-only
   edits. Keep B-Rep generation, mesh and worker token unchanged. A redraw
   failure after model acceptance triggers projection retry, never a second
   commit. Any in-flight preparation from an older revision remains stale.
2. **Session and panel.** Extend the existing App-owned setup state to one
   active draft keyed by document/command and, for edits, source FeatureId and
   captured revision. Publish only panel availability, support label, grid
   state, tool intents and ready status through the existing Controller.
   Setup OK starts editing. Active OK is a distinct commit intent, confirmed
   against the current lifetime and readiness. Keep the fixed 27-cell editing
   panel and existing Sketch Bar groups. Bind only Line and Rectangle menu
   actions and route Sketch Grid to the existing View preference. Extend the
   existing Properties request to branch by authoritative feature kind.
   Reopen directly into Select with a private copy of the stored recipe.
   Ensure the setup-only Action Box reference test consumes the two setup
   fixtures and adds coverage for the third, editing fixture.
3. **Camera and grid.** Save the displayed entry camera/projection in App
   navigation before entering. Build an orthographic target with positive
   normal and V up, preserving apparent scale at the projected support
   origin. Reuse the named-view transition and current viewport resizing
   helpers. Install orthographic projection at transition start and only
   interpolate the pose. Follow §42 for interrupted alignment, suppressed
   view requests and restoration. Publish one immutable grid frame chosen
   from world XY and the active support. In the current shader, intersect rays
   with that plane and project hit differences and derivatives onto U/V.
   Use signed distance to the same plane for grazing/forward fades and retain
   scene-depth clipping, spacing, axis reinforcement, theme and opacity.
   Restore world XY on exit. No grid toggle mutation during entry or exit.
4. **Geometry and rendering.** After existing O(1) shell and viewport target
   admission, unproject a forward ray using the current physical viewport and
   intersect the plane. Project to local U/V in f64. Use the existing
   press/release threshold and stamp events with the active session and
   camera/layout validity without treating a preview-only redraw as a new input
   lifetime. Distinguish Line's initial pending point from its
   ready chain endpoint. Tool switch, focus loss, layout changes and Escape
   clear the appropriate transient state. Completed actions stay private
   until OK. Controller hit tests against completed strokes using the
   five-logical-pixel rule and stable tie order. Cap each recipe at 4,096
   primitives. Decline a placement at capacity and keep the draft editable. One traversal of the bounded draft strokes per
   admitted Select click is sufficient for this slice.
   Submit the draft, selection and preview as one immutable stroke packet.
   Render per ADR 0011 and keep picking and mutation out of the renderer.
   Clear active packets and pending events on commit, Cancel and document
   replacement. Status text uses existing cells for any rejected placement
   or commit diagnostic.

## Executor file scope

| Area | Files |
|---|---|
| Model | `crates/newell-model/src/{lib,geometry}.rs`, new `src/sketch.rs`, and existing model unit tests there. |
| App | `crates/newell-app/src/{lib,app,controller,document,navigation,camera,input}.rs`, new `src/sketch.rs`. Minimal exhaustive feature handling in `geometry_worker.rs`, `parameters.rs` and `extrude_session.rs` if needed. App/navigation/document regression tests in their existing test modules. |
| ASCII UI | `crates/newell-ui/src/{lib,sketch}.rs`, `src/shell/{mod,data,compose,tree,tests}.rs`. Preserve old tool order and popup fixtures. |
| Renderer | `crates/newell-render/src/{lib,camera,renderer,grid}.rs`, `src/grid.wgsl`, new `src/{sketch.rs,sketch.wgsl}`. Existing grid readback tests plus a bounded stroke validation/readback test in the new module. |

The amended specifications accompany this revision. Resolve no additional
binding rules in production code. Keep all existing kernel pins and tests.

## Acceptance

- In a fresh origin-only document, confirm XY. Verify U right, V up, positive
  normal, stable orientation under pan/zoom, and unchanged Application Menu
  Bar. Check Sketch Bar and active Action Box simultaneously at 82 × 30.
- With Grid Overlay checked, confirm XZ or YZ and verify only that support's
  grid appears. Compare basis, origin and adaptive zoom subdivisions against
  XY. Check both toggle controls and preservation on OK and Cancel. Check an
  initially unchecked entry and exit. Include a GPU check of a non-XY plane.
- On each support, use real input routing to draw connected Line segments and
  a two-corner Rectangle. Assert exact model-local endpoints within the
  numeric projection tolerance, world points on the support, four derived
  Rectangle sides, and previews clipped at UI boundaries. Reject invalid
  ray/point/extent and stale gestures. Verify Escape cancels an unfinished
  gesture without removing completed geometry and that drawing suppresses
  orbit but preserves pan and zoom.
- Validate XY as `Sketch1`. Cancel a populated draft, then validate the
  non-XY sketch as `Sketch2`. Confirm one revision and one Features row per
  accepted insert. Test duplicate OK, name collision, rename/reorder,
  revision and identity exhaustion, stale draft and unchanged B-Rep token.
- Reopen via Properties, delete a Rectangle group and draw a replacement.
  Verify Cancel retains the original. Reopen, apply the edit and verify
  unchanged feature ID, name and order, changed recipe and no ordinal
  consumption. Verify another window sees only committed revisions.
- Verify normal Command Bar and entry camera restoration on both exits.
  Separate live viewport and GPU observations from CPU results.

Executor records baseline and runs focused checks, workspace check,
formatting, strict Clippy, tests with `INSTA_UPDATE=no`, doc tests and the
relevant ignored renderer GPU tests on a capable device. Report inherited
failures separately. Native viewport acceptance must exercise both planes.
