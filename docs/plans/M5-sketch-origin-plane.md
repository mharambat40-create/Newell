# M5 — Origin-plane Sketch setup

Revision 1 · 2026-11-08 · Architect / Codex  
Status: proposed for explicit Executor approval.

## Scope and authority

Implement the window-local entry shell from [ARCHITECTURE.md §3.7](../../ARCHITECTURE.md#37-scoped-3d-navigation-widget-exception-2026-10-27)
and [README_UI.md §29.2](../ui_design/README_UI.md#292-sketch-setup-and-sketch-bar).
This focused slice of the [M5 Sketcher plan](M5-sketcher.md) starts setup,
selects or replaces a built-in support, confirms entry to the Sketch Bar,
and exits the local mode. The human authorized the specification amendments
and this plan. Implementation needs separate approval of revision 1.

No model or kernel Sketch feature, geometry, drawing tools, constraints,
solver, planar camera, new viewport rendering, persistence, datum/face
support or dependency changes. This slice supplies no evidence for closing
M5.0 or completing the full M5 editing shell.

Starting checkout: `main` at `310f382` with an empty Git index. Unrelated
unstaged adaptive grid changes were present in `ARCHITECTURE.md`,
`docs/ui_design/README_UI.md` and `crates/newell-render/src/{grid.rs,grid.wgsl}`.
`docs/plans/viewport-grid-continuity.md` was already untracked.
Preserve them and the index. Recheck starting state at implementation.

## State and routing

1. **Start a command.** Add Sketch to the existing command-start route
   (Datum menu activation, shell operation, controller command intent,
   application dispatch). Allocate a fresh command lifetime. Branch for
   Sketch before allocating numeric/geometry inputs, parameter drafts or
   playhead-bound model state. Use normal command replacement cleanup when
   switching from an existing Action Box. Show setup only for the accepted
   lifetime. Route setup OK/Cancel and confirmed Escape as local intents
   through the controller, independent of the CAD commit path.
2. **Own one support.** The window's App owns a document- and
   command-qualified state with phases `Setup(Option<OriginPlane>)` and
   `Active(OriginPlane)`, plus the absent state. Use a closed three-case
   semantic support type with a checked conversion from the existing
   `OriginReference`. Derive the display name from the existing reference
   name and the frame from §3.7. A shell projection carries the current
   phase and field text or identity, and derives button enablement. The
   shell, picker and navigation hold no second applied value. Seed setup
   from exactly one eligible identity in the current origin selection.
3. **Use the existing viewport click.** In the validated
   `finish_origin_reference_click` route, retain current exposure,
   press/release identity, drag threshold, overlap and freshness checks.
   Preserve the ordinary origin selection and tree reveal. If this is a
   click on a plane in the same live unassigned setup, assign it and
   focus the Plane field. No new ray query or axis/plane precedence.
   Stamp or invalidate a pending click across draft transitions so a
   release cannot bind to a replacement draft. Tree clicks may preselect
   before command start through the ordinary origin selection route.
4. **Reuse command-field selection.** Give Plane an explicit single-reference
   purpose. Use the current field-open, candidate chooser, and completion
   routes, including their document, modal and command-field lifetime checks.
   Supply the fixed candidate set in XY/XZ/YZ order. If the chooser uses
   opaque row keys, map them only within this field transaction to the
   three typed identities. Never construct model entity IDs for origin
   planes or resolve them through model geometry candidates. Stage selection
   inside the modal. Only a single validated candidate from the live picker
   can replace the applied support. Support disabled OK for an empty
   Sketch picker through the existing activation path, leaving other
   pickers' empty-confirm policies intact. Preserve the same field-body
   focus after acceptance or cancellation. Reject stale, empty, forged
   or wrong-field completions without writing the draft.
5. **Confirm and compose.** Recheck the support and live lifetime on setup OK.
   Close setup and publish the confirmed mode and support together. Use
   one command-menu definition per context throughout composition, hit
   testing, click and keyboard dispatch. Keep normal popup widths and
   use 22 cells for Sketch tool popups, with the shared Workbench popup
   retaining its normal size. Close stale command popups and clear focus
   tied to removed controls during transitions. Keep the existing task-bar
   visibility preference, separate Application Menu Bar and one flat
   ASCII batch. Setup Cancel or replacement drops the draft and picker.
   Exit, document replacement and window close invalidate the corresponding
   session and any pending intents. Enter and exit make no model, worker
   or camera calls on behalf of Sketch.

## Executor file scope

| Files | Ownership and intended change |
|---|---|
| `crates/newell-app/src/{app,controller,selection}.rs` | App owns support and lifecycle. Dispatch the Sketch intents, resolve the origin hit and picker result, project accepted state, and preserve existing command cleanup and input precedence. Controller transports published state and intents. |
| `crates/newell-ui/src/{lib,action_box,selection}.rs`, new `crates/newell-ui/src/sketch.rs` | Support identity and derived presentation, fixed 27-cell panel, Plane field descriptor, picker policy and command intents. Use existing Action Box placement and focus machinery. |
| `crates/newell-ui/src/shell/{mod,data,compose,tree}.rs` | Register Sketch, lay out the correct command strip, route focus/activation, and install the field and mode projection. Reuse the current menu state and semantic field IDs. |
| `crates/newell-ui/src/{menu/mod,overlays/selection}.rs` | Minimal shared menu or chooser adjustments only where needed for per-group width or required single selection. Preserve existing callers' behavior. |
| `crates/newell-ui/src/{shell/tests,operations_tests,overlays/tests}.rs` and existing test module in `crates/newell-app/src/app.rs` | Reference-backed panel/menu checks and real input-routing regressions. Put new tests in existing modules. |

The binding docs and ASCII states are already provided with this revision.
Preserve the constant-time grid routing, existing viewport hit rules,
AGPL headers and Rust 1.88 compatibility.

## Acceptance and verification

Start each of the first three paths in a fresh document with only built-in
Origin references. Use real shell actions and viewport clicks for manual
acceptance. Automated coverage should assert semantic identity and UI output,
not simply compare labels.

| Case | Expected result |
|---|---|
| Plane first | Click a visible origin plane, then Datum > Sketch. Field shows its canonical name and setup OK is enabled. Normal bar remains until OK. |
| Sketch first | Open Sketch with no plane selected. Field shows `Select…` and OK is disabled. Click a visible origin plane. Its canonical name appears, the tree reflects ordinary origin selection, and Plane receives focus. |
| Plane field first | Activate `Select…`; confirm a candidate via the Selection box. Plane shows its canonical name and has focus. Window selection and the normal bar remain unchanged until setup OK. Exercise keyboard activation too. |
| Replace / cancel picker | Start from XY. Stage XZ and cancel with Escape; XY and field focus survive. Repeat and confirm; field becomes XZ and window/tree selection stays as it was. Reopen to verify the applied row. Also cancel an initially empty picker. |
| Invalid or missing support | Verify origin point, axes, empty clicks and multiple preselected planes cannot seed or populate the field. Disabled setup and picker OK cannot activate by mouse or keyboard. Reject invalid or stale completion after replacing or cancelling the originating draft. |
| Cancel setup | Cancel both an empty and a populated setup. Verify no support carries into a fresh draft except through current origin preselection. The normal command strip and document remain intact. |
| Confirm and leave | On setup OK, verify the exact Sketch Bar group order, popup labels and centred frames. Check unchanged menu bar, workbench and document. Verify Escape closes a Sketch popup first, then exits when unowned and restores the normal bar. Check task-bar visibility in both modes. |
| Isolation and invariants | Exercise XY/XZ/YZ and verify the fixed frame basis for each. Verify a second window sees no draft or mode changes. Compare document version, model selection and history before/after setup and mode entry. |

Executor records the baseline before code changes and runs focused app/UI
checks plus the workspace check, format check, strict Clippy, nextest
(or the documented cargo test fallback) and doc tests from the
[shared verification instructions](../agents/README.md#verification-and-handoff).
Set `INSTA_UPDATE=no` for snapshot-bearing runs. Report manual app results
separately from automated checks and distinguish baseline failures from
regressions. No GPU pipeline changes are planned.
