# Origin-plane clipping and zero-grid lines — 2026-10-30

Role/provider: Executor / Codex. Authority: the focused rendering implementation
request, including narrow canonical documentation corrections; no separate plan
revision. Checkout: main, HEAD 29321e9efc1a18753e18a081de598b5ac2f9f70b.
Index was empty and remains unchanged. Unrelated SolidCore, kernel, dependency,
UI and documentation work was preserved. No branch, worktree, source snapshot,
agent dispatch, integration or review stage was created.

## Sequential ownership

The same request was being implemented in the existing Quick Fix chat. This
chat made no source edits while that work was active. At the human's direction,
it waited for that chat to finish, retained its completed changes as inputs,
then completed verification and corrected one remaining origin-plane picking
failure. The additional edits here are navigation.rs and this report.

The retained implementation touches camera.rs, navigation.rs, grid.rs,
grid.wgsl, ARCHITECTURE.md and README_UI.md. Other existing edits in those files
were preserved.

## Demonstrated causes and fixes

The old reference clip envelope used a fixed radius of 1.01 model units, while
fixed apparent reference dimensions grow in world units during zoom-out. A
regression demonstrates a plane corner beyond the old far clip plane. The
reference-aware snapshot now encloses the actual scaled extent, including the
156-pixel plane diagonals. Its finite orthographic depth interval may extend
behind the nominal eye plane when necessary. Ordinary camera snapshots retain
their existing near-plane behavior; perspective still requires positive depth.
No reference is moved toward the camera and no depth test is globally disabled.
The main reference renderer and its model-depth comparison consume the same
snapshot. Rectangle vertices, 20% positive-quadrant inset, neutral materials,
alpha and 20%-opacity model-occlusion rule are unchanged.

The grid shader explicitly suppressed a band around X=0/Y=0. Removing that band
restores the neutral lines through the exact world origin. Each spacing level
uses the same analytic coverage; levels are mixed rather than accumulated and
the two directions use maximum coverage at their crossing. No extra colored
axis or internal reference-plane grid was introduced.

After the handoff, a further regression reproduced a visible orthographic XY
plane returning no selection at height 200 in an offset viewport. The expanded
reference depth slab could draw it behind the nominal eye, but reference picking
still began at that eye plane. Origin-plane picking now begins at the rendered
orthographic near plane when it is negative. Perspective and ordinary mesh
picking retain their original rays. The new test fails before this correction
and passes after it, at two zoom heights and two offset/wide viewport sizes.

## Actual verification

- Workspace check and app build with --locked: passed.
- Workspace all-feature tests, INSTA_UPDATE=no: 1,132 passed, 0 failed,
  23 ignored. This includes the new picking regression.
- Explicit renderer GPU suite: 8 unit + 14 integration tests passed, including
  zero-line continuity across 1/2/5 transitions, crossing intensity, depth
  occlusion, clipping and resizing coverage.
- Navigation-focused tests: 8 passed. Reference-envelope and legacy-cutoff
  regressions and existing close-camera clipping test also passed separately.
- Workspace formatting, standalone doc tests, diff whitespace checks: passed.
- App + renderer all-target Clippy with warnings denied: passed.
- Workspace all-feature Clippy remains blocked by unrelated kernel SolidCore
  findings: large enum variant, chunks_exact, type complexity and collapsible if.

The initial baseline overlapped the other chat's edits and caught a temporary
orthographic camera regression, plus unrelated SolidCore test/lint failures.
The later stable workspace test run above passed; no failure was suppressed and
no test snapshot was regenerated.

## Visual evidence and limits

Quick Fix reported visual confirmation of intact planes and the zero-grid cross
at startup and one orthographic zoom-out using the checkout-built executable.
This chat rebuilt target/debug/newell and launched that exact executable through
/tmp/NewellGridFix.app, whose executable symlink was verified to target this
checkout. The final app screenshot shows intact oblique planes at startup.
No obsolete application was opened.

Further native input failed repeatedly with noWindowsAvailable, including after
raising and reconnecting to the app window. Therefore this chat could not finish
interactive zoom-in/out sweeps at several distances, principal/oblique changes,
perspective switching, resizing or clicking the corrected plane. These visual
checks remain pending; passing CPU/GPU regressions is not full manual acceptance.
The next step is that bounded visual sweep when native input is available. No
further role or review cycle was started.
