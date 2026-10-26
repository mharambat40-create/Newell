# M3 secondary-click trackpad pan validation — 2026-10-21

**Role/provider:** Executor / Codex. **Objective:** respond to physical-device
feedback that one-finger click-drag orbits, while two-finger click-drag does not
translate the scene.

**Baseline:** `/Users/martin/Desktop/Newell`, branch `main`, HEAD
`1eb4a9a4b9fb45a4bbfaa223e348c0e5e9a69f1b`. The Git index was empty and
is unchanged. The working tree was dirty before this task, including ongoing
app navigation, architecture, plan, renderer, UI, UI-reference and validation
work. The relevant starting file hashes were:

Initial unstaged paths were `ARCHITECTURE.md`,
`crates/newell-app/src/{app,navigation}.rs`,
`crates/newell-app/tests/it/{controller,navigation}.rs`,
`crates/newell-render/src/{scene,ui/theme}.rs`,
`crates/newell-render/tests/it/ui_gpu.rs`,
`crates/newell-ui/src/{create_box,overlays/selection,shell/compose,shell/data,shell/mod,shell/tests}.rs`,
`crates/newell-ui/tests/it/{menu_bar,snapshots}.rs`,
`docs/plans/M3-camera-mechanics.md`,
`docs/ui_design/Application_Menu_Bar.txt`, and `docs/ui_design/README_UI.md`.
Initial untracked paths were
`docs/ui_design/Action_Box_{Axis,Construction,Plan}.txt` and
`docs/validation/M3-primary-orbit-2026-10-20.md`. A task-owned temporary
baseline directory was created after recording this status and removed at
handoff.

- `crates/newell-app/src/app.rs`: `1c5a06743380aea05d4178f845de86148d65ccb7913990fb555ae13edac32c33`
- `crates/newell-app/src/navigation.rs`: `37f140948c4782c09457064b7adaf1c2b4410cc71b326d6ef0d05faff4bfb3eb`
- `crates/newell-app/tests/it/navigation.rs`: `4e305bc22365320742eb2cfbd61ac5fb377e3ca2bd5c9b9c9d448bc861261f7c`
- Approved `docs/plans/M3-camera-mechanics.md` revision: `8e4d08e627d5ae190312150fdd1d49923b9b37dc50087612dbbd129022813aca`

The starting implementation already mapped a secondary-button pointer drag to
contextual-depth pan. On macOS, winit can deliver two-finger movement as
`MouseWheel::PixelDelta` while the secondary button is down. The previous wheel
route could neither zoom (the pending pan owned navigation) nor advance pan
(only pointer motion advanced it), matching the reported symptom.

The Controller now adds physical pixel-scroll displacement to the same captured
secondary pan. It accumulates with pointer motion, passes through the existing
four-logical-pixel click/drag threshold, and uses the existing pan mathematics,
frozen contextual depth and redraw path. A secondary press still must start on
an exposed scene pixel. A phased scroll stream consumed by pan cannot later
scroll UI or zoom the scene through momentum after button release. A fresh
scroll start remains available for ordinary cursor-targeted zoom. Primary orbit,
middle-button pan, unpressed wheel/trackpad zoom and native pinch retain their
existing routes. No renderer, geometry, UI-routing or camera-math code changed.

## Files changed by this task

- `crates/newell-app/src/app.rs`
- `crates/newell-app/src/navigation.rs`
- `crates/newell-app/tests/it/navigation.rs`
- `docs/validation/M3-secondary-scroll-pan-2026-10-21.md` (this report)

## Checks

- Baseline and final `cargo check --workspace --locked`: passed.
- Baseline and final `cargo fmt --all -- --check`: passed. Only the three task
  source/test files were formatted directly.
- Baseline and final `cargo clippy --workspace --all-targets --all-features
  --locked -- -D warnings`: passed.
- Focused navigation tests: **40 passed, 0 failed**.
- Focused controller tests: **35 passed, 0 failed**.
- Baseline `INSTA_UPDATE=no cargo test --workspace --all-features --locked
  --no-fail-fast`: **754 passed, 4 failed, 9 ignored**.
- Final same workspace command: **759 passed, 4 failed, 9 ignored**. The five
  new navigation tests pass; no new failure appeared.
- `cargo test --workspace --doc --locked`: passed.
- Task-file `git diff --check`: passed. No staging, commit, push or merge.

All four workspace failures were inherited: renderer glyph tests
`glyph::canonical_set_covers_every_ui_reference_symbol` and
`glyph::every_required_glyph_is_in_the_atlas_for_every_k` reject `à` in the UI
reference; UI integration tests `menu_bar::the_view_popup_fits_the_minimum_grid`
and `menu_bar::view_popup_matches_the_reference_in_its_default_state` expect a
different View-menu row. Renderer GPU tests were not run because this task
changes only Controller input routing.

## Physical check still required

On macOS, press the trackpad with two fingers on a visible cube face and drag
past four logical pixels in both axes. The scene should translate in the
movement direction without orbiting or zooming; a short secondary click should
leave the camera still. Repeat near an Action Box, menu, tree and resize zone;
they must retain ownership. Release during a drag and check that subsequent
movement does not pan. Then try a normal two-finger scroll without a click,
wheel zoom, pinch zoom, one-finger orbit and middle-button pan. A physical
trackpad result cannot be inferred from the CPU tests: this route specifically
requires macOS to deliver a secondary-button press together with pixel-scroll
events. If the gesture still does nothing, capture the actual winit event
sequence before changing the mapping again.
