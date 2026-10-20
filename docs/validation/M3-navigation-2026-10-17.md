# Viewport navigation validation — 2026-10-17

Scope: refine existing Rust camera and input handling. BrepKIT mesh generation,
WGPU rendering, ASCII UI, event-driven redraw and constant-time grid hit testing
are preserved. No dependency changes or commits were made. Existing working-tree
changes were retained, including changes already present in the touched files.

## Interaction

Right-drag uses a press-centred sphere/hyperbolic quaternion trackball expressed
in camera screen axes, with a fixed target. Middle-drag pans on the target-depth
plane. Wheel and fractional, DPI-normalized trackpad scrolling zoom toward the
pointer on that plane. Native pinch is supported when delivered by winit.
Phased gestures cannot start under UI and then enter the scene mid-stream.
Camera input retains physical subpixel positions; UI continues using grid pixels.
Panel scrolling retains its prior conversion, independent of camera sensitivity.

Mesh bounds are cached at upload without resetting camera state. Zoom has an
absolute 0.001 minimum distance and an aspect-aware model-scale maximum. Clip
planes follow mesh depth. Home over visible scene pixels fits the model into
the largest unobstructed rectangular region, preserving orientation; text-field
editing retains Home. Fit is explicit recovery after unrestricted panning.

Action Boxes only change occlusion. Resizing changes focal-angle and projection
shift together to preserve physical-pixel image positions at every depth,
without moving the eye or target. Layout changes safely end active navigation.

## Files changed by this task

- `crates/newell-render/src/scene.rs`: camera math, bounds, clipping, capture,
  occlusion, fit, camera unit tests and GPU navigation integration assertions.
- `crates/newell-render/src/renderer.rs`: fit-view entry point.
- `crates/newell-app/src/app.rs`: gesture events, physical pointer tracking,
  lifecycle cancellation, trackpad handling and Home recovery.
- `crates/newell-app/src/controller.rs`: visible-scene input eligibility.
- `crates/newell-app/src/input.rs`: wheel/pinch normalization and gesture ownership.
- `crates/newell-app/tests/it/input.rs`: delta and gesture ownership regressions.
- `crates/newell-app/tests/it/controller.rs`: UI precedence and Action Box stability.
- `ARCHITECTURE.md` §4.4, `docs/plans/M3-camera-navigation.md`, `docs/backlog.md`:
  consistent interaction contract, trade-offs and manual acceptance backlog.
- This validation record.

## Automated results

- `cargo check --workspace`: passed.
- `cargo fmt --all --check`: passed. Formatting was restricted to task edits;
  unrelated working-tree files were not formatted.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test -p newell-render`: 25 unit tests and 35 integration tests passed;
  9 hardware tests ignored in this invocation.
- `cargo test -p newell-render -- --ignored`: all 9 hardware tests passed,
  including a real BrepKIT mesh rendered through WGPU, navigation occlusion,
  mesh replacement, resize cancellation and unobstructed fit assertions.
- `cargo test -p newell-app --test it input::`: all 8 tests passed.
- `cargo test -p newell-app --test it controller::`: all 34 tests passed.
- `git diff --check`: passed.

The broader `cargo test --workspace --no-fail-fast` run encountered 34 failures
outside this task: 1 app playhead-separator assertion, 6 UI overlay unit tests,
and 27 UI integration tests (including layout snapshots). The playhead failure
was reproduced with the starting controller implementation, then the new
controller was restored. No `newell-ui` implementation or test files were
changed by this task, and their failing expectations were not rewritten.

Cargo also reports the existing dependency `block v0.1.6` as having future Rust
incompatibilities; this does not fail strict Clippy on the current toolchain.

## Visual acceptance still required

No claim of physical-device or desktop visual acceptance is made. Check:

1. Right/up drags at viewport centre and corners; repeated orbit across poles.
2. Mouse wheel, smooth two-finger scrolling and native pinch on supported hardware,
   including OS natural-scrolling preferences and momentum after crossing UI.
3. Action Box opening/closing and live window/navigation-panel resize without
   image movement; explicit Home recovery, also with an Action Box open.
4. Extreme zoom on very small/large geometry, near-surface clipping and pan recovery.

Target-plane anchoring intentionally avoids mesh scanning and GPU readback.
Points at another surface depth can slide slightly during zoom; exact picked
surface anchoring is deferred. Native pinch uses the last pointer position
because its event does not provide a separate gesture centroid. Off-axis resize
preserves image position, so enlarging a window does not automatically recenter
its contents; Home performs that explicit reframing.
