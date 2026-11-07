# Adaptive grid — Executor verification, 2026-10-30

Provider/role: Codex / Executor. Authority: the direct adaptive-grid implementation
request and its narrow architecture/UI amendments; no separate numbered plan.
Checkout: main, starting HEAD 29321e9efc1a18753e18a081de598b5ac2f9f70b.
The initial index was empty. Existing app, navigation, controller, kernel,
experimental vendor, plan and validation changes were retained; no staging,
branch, worktree, snapshot, dependency change or agent dispatch was performed.

## Implementation

Changed app redraw publication and navigation-derived immutable camera inputs;
added render/grid.rs and grid.wgsl, wired Renderer, exposed scene viewport,
occlusions and model-surface depth internally, and amended ARCHITECTURE.md and
README_UI.md within the explicit authorization. No Model or input-routing edits.

The existing unchecked Grid Overlay checkbox publishes an optional snapshot.
The GPU intersects camera rays with world Z=0 and evaluates two adjacent 1/2/5
spacing levels with smooth blending, analytic one-pixel coverage, density and
horizon attenuation. Theme alone selects white or black ink at 20% peak alpha.
Axis lines are suppressed. Model-surface depth obscures the grid in all display
styles; existing exposed-region scissors preserve UI occlusion. Draw work is
one triangle per exposed rectangle, independent of zoom or grid extent. ASCII
remains one draw. There are no CPU grid lines or model/selection/tree/fit/history
entries. Camera inversion uses f64 Controller matrices before GPU conversion.

## Actual checks

- Baseline workspace check and formatting passed. Baseline Clippy failed at
  existing kernel sketch/rank.rs Large enum variant (Front continuation).
  The baseline full-test process overlapped implementation and its doctest
  phase saw a transient inconsistent renderer build; final doc checks passed.
- Workspace all-feature tests with INSTA_UPDATE=no: 1,117 passed, 22 ignored,
  zero failures. This run preceded the final inverse precision refinement;
  the refined helper was subsequently checked by the focused camera test.
- Focused camera inverse/world-point reconstruction: passed in orthographic and
  perspective, oblique/panned/top viewpoints; singular matrix rejected.
- Explicit renderer GPU suite: 8 unit GPU tests and 14 integration GPU tests
  passed. New pixel readback covers visibility, white/dark ink, model-depth
  rejection, continuous small zoom changes and scales 0.001, 1, 100, 1e6.
- Final workspace check, app build, fmt check, doc tests, renderer all-target
  Clippy, Rust 1.88 workspace check, git diff --check: passed.
- Workspace all-feature Clippy: same inherited kernel large-enum failure.
- cargo deny check: advisories, bans, licenses and sources passed.

The executable built here (target/debug/newell) was launched successfully and
relaunched after the final build. Native UI automation could not identify the
unbundled executable window; no other Newell application was launched.

## Pending visual acceptance

Interactive menu checkbox-to-viewport toggling; pan/orbit world anchoring;
perceptual subdivision transitions in both projections; Light/Dark and Color
independence; real model/UI overlap; horizon moire and extreme navigation zoom.
GPU tests do not establish physical-device visual acceptance. No review cycle
or next role was started. Human visual validation is the remaining next step.

## Focused grid/camera correction — 2026-10-30

Executor / Codex; authority is the human's focused implementation request (no
separate plan revision). Same main checkout and HEAD as above; index empty on
entry and untouched. Existing adaptive-grid inputs were uncommitted. Concurrent
SolidCore edits, dependency/lock changes and experimental kernel tests are
outside this task and were preserved. Only app.rs, camera.rs, navigation.rs, render/grid.rs,
render/grid.wgsl and this report were changed by this correction.

### Demonstrated causes and corrections

- The procedural grid used the model's finite near/far interval and explicitly
  discarded z outside [0,1]. Intersecting the XY plane with that interval creates
  artificial horizontal cutoffs even when rays hit the plane in front of the
  eye. GPU tests reproduce the failure by changing only scene clip depth.
  Grid rays now use a well-conditioned auxiliary inverse projection, with the
  actual eye for perspective and the eye plane for orthographic. Scene VP is
  retained for model depth comparison, but clear depth never bounds the grid.
  Behind-eye intersections remain rejected. Exposed-region scissors, viewport
  physical-pixel coordinates and model-surface depth are preserved.
- Levels were selected per fragment from finite-difference world-hit derivatives;
  derivative quads/local footprints could reach thresholds at different times.
  The Controller now publishes one continuous focal-plane model-units-per-pixel
  value from the current camera. Both directions and every exposed rectangle use
  the same levels/weight. Analytic ray/plane derivatives independently filter
  dense perspective lines and retain horizon fade. No temporal level state,
  patch geometry or partial uniform update is involved.
- Independent camera regression: near=1e-7 with an eye inside the enclosing sphere
  can round a valid forward perspective surface to clear/far depth in f32/24-bit
  rendering. A test reproduces the old rounding independently of grid rendering.
  The floor is now 0.01 × MIN_DISTANCE (1e-5), leaving the 0.001 navigation
  minimum, cursor anchor, pivots, fit and permitted zoom range unchanged. Forward
  close surfaces remain in range in both projections; behind-eye geometry clips.
  All scene rendering paths still consume the same Controller snapshot.

This establishes a precision defect, not that every reported disappearing model
part had that cause. The visual sweep did not establish camera entry into the
solid. Existing back-face culling and behind-eye clipping legitimately omit
surfaces from an inside-solid view; no collision avoidance was introduced.

### Targeted evidence and visual sweep

Independent source builds against existing compiled dependencies passed 28 render
CPU tests, 8 render GPU tests and 7 camera/navigation tests. Tests cover model
clip independence, perspective ray equivalence, reverse zoom determinism,
continuous level changes, wide/tall offset viewports, split scissor regions,
physical viewport scaling, close depth, and matrix consistency. This fallback
was used while Cargo waited on the concurrent migration's unavailable Git
revision/cache lock; it is distinct from a full workspace validation.

The app's current sources were also explicitly rebuilt against cached dependencies
as /tmp/newell-gridfix-checkout, then launched via a temporary macOS app wrapper
whose executable symlink points to that build. No obsolete application was opened.
Native access initially timed out, then worked after that build was launched.
Observed zoom in/out in orthographic and perspective, oblique and grazing orbits,
model/menu occlusion, Light/Dark, and UI scale 100%→120%→100% with window resizing.
No artificial horizontal cutoff or rectangular transition patch was observed in
that sweep. Near the horizon the grid faded and left the sky empty. A physical
cross-monitor DPI change and exhaustive extreme-scale/device validation remain
pending; automated physical-viewport checks are not a substitute for those.

An additional redraw-order defect was found in the actual app path: grid inputs
were published before sync_navigation_widgets reconciled the camera viewport and
scene snapshot. Resizing/layout changes could therefore pair the old grid camera
with new scene dimensions for one frame. Redraw now reconciles first, then derives
the grid snapshot from that same camera. This ordering correction follows the
visual sweep; the final normal app build includes it.

### Final normal checkout checks

After the concurrent dependency resolution settled, normal Cargo checks resumed:
workspace all-feature tests with INSTA_UPDATE=no passed **1,122 tests**, zero
failures, 23 ignored. The final redraw-order change was subsequently workspace
checked and app-built; it does not change geometry tests. Explicit renderer GPU
tests passed **8 unit + 14 integration tests**, including the final forward-ray
and clip-range coverage assertions. Workspace check, app build, standalone doc
tests, workspace fmt check, renderer all-target Clippy, and diff whitespace checks
passed. Workspace all-feature Clippy retains the pre-existing large-enum failure
in kernel sketch/rank.rs. Initial baseline also encountered a transient unrelated
kernel test syntax error; later normal workspace tests passed after concurrent
work corrected it. No kernel or dependency file was changed for this task.

The final Cargo-built target/debug/newell was relaunched through the same wrapper
for a post-reconciliation resize check. The fallback source build is not needed
to run the delivered implementation. No next role or review was started.
The post-reconciliation final-build check passed visually: enabled the grid,
changed UI scale 100%→120%→100%, and inspected complete exposed-plane coverage
with the View menu open and after closing it. The final build remains open at
100% scale with the grid visible for inspection.
