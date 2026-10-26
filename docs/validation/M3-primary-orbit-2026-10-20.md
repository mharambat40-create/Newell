# M3 primary-button orbit validation — 2026-10-20

**Role/provider:** Executor / Codex. **Objective:** complete the narrow input
mapping for macOS one-finger trackpad click-drag, which winit reports as primary
mouse press and pointer motion. A physical left-button drag has the same mapping.

**Baseline:** branch `main`, HEAD
`1eb4a9a4b9fb45a4bbfaa223e348c0e5e9a69f1b`. The Git index was empty and
remains byte-for-byte empty. Fifteen files had unstaged edits before this task:
`crates/newell-app/src/{app,navigation}.rs`,
`crates/newell-app/tests/it/navigation.rs`,
`crates/newell-render/src/{scene,ui/theme}.rs`,
`crates/newell-render/tests/it/ui_gpu.rs`,
`crates/newell-ui/src/{create_box,overlays/selection,shell/compose,shell/data,shell/mod,shell/tests}.rs`,
`crates/newell-ui/tests/it/menu_bar.rs`, and
`docs/ui_design/{Application_Menu_Bar.txt,README_UI.md}`. The approved camera
mechanics plan was read at `docs/plans/M3-camera-mechanics.md`, SHA-256
`524386b42c0b8fd58ec328fe52d2e5b1b5330a91c8a315246ccb4f418ab609fc`.
The user explicitly narrowed this execution to primary-button orbit input;
architecture and canonical UI documents were not changed by this task.

The starting dirty tree already contained a partial primary-orbit mapping and
four tests. This task completes it by capturing the existing orbit's camera
and pivot at press, waiting for the existing four-logical-pixel radial threshold,
then handing the stored drag to the same orbit path used by the right button.
No camera motion occurs for a click or subthreshold jitter. The application
continues its existing picking on the press and uses the subpixel pointer only
for navigation. Shell routing first admits only exposed scene pixels; controls,
menus, overlays, tree, divider, resize zones and modal background retain their
own behavior. Release always clears the primary capture, including when a
resize consumes the release. Focus loss, window exit and layout changes cancel
it through existing paths. Wheel/pinch input waits until a pending press ends,
so the frozen orbit pose cannot snap back after an intervening zoom. Right orbit,
middle pan and standalone zoom retain their existing paths. All camera state
and input handling remain in `newell-app`; rendering remains event driven.

## Files changed by this task

- `crates/newell-app/src/app.rs`
- `crates/newell-app/src/navigation.rs`
- `crates/newell-app/tests/it/controller.rs`
- `crates/newell-app/tests/it/navigation.rs`
- `docs/validation/M3-primary-orbit-2026-10-20.md` (this report)

No renderer, kernel, UI, picking or camera mathematics files were edited by
this task. No dependencies, snapshots or Git index entries were changed.

## Verification

- Baseline `cargo check --workspace --locked`: passed.
- Baseline and final `cargo fmt --all -- --check`: passed. Only the four task
  source/test files were formatted directly.
- Baseline and final `cargo clippy --workspace --all-targets --all-features
  --locked -- -D warnings`: passed.
- Focused `INSTA_UPDATE=no cargo test -p newell-app --all-features --locked
  --test it navigation::`: **31 passed, 0 failed**.
- Focused `INSTA_UPDATE=no cargo test -p newell-app --all-features --locked
  --test it controller::`: **35 passed, 0 failed**.
- Baseline `INSTA_UPDATE=no cargo test --workspace --all-features --locked
  --no-fail-fast`: **737 passed, 6 failed, 9 ignored**.
- Final same workspace command: **748 passed, 4 failed, 9 ignored**. All nine
  tests added by this task pass in the workspace run. No new test failures.

The four remaining failures were present in the baseline: renderer glyph tests
`glyph::canonical_set_covers_every_ui_reference_symbol` and
`glyph::every_required_glyph_is_in_the_atlas_for_every_k` both reject the `à`
symbol in `docs/ui_design/Interface_Elements.txt`; UI integration tests
`menu_bar::the_view_popup_fits_the_minimum_grid` and
`menu_bar::view_popup_matches_the_reference_in_its_default_state` expect a
Color row where the current View popup shows Tree panel. The baseline also
failed `shell::tests::profile_field_opens_the_exact_profile_selection_box`
and `snapshots::selection_panel_open`; these passed in the final run after
independent concurrent UI edits. Those edits were neither created nor changed
by this task. The already present `block v0.1.6` future-compatibility notice
persists; Clippy itself passes. Renderer hardware tests were not run because
this change touches input routing only.

## Physical trackpad acceptance still needed

In the running macOS application, click-drag with one finger over a clear cube
face and empty viewport; the object should orbit smoothly with no initial jump.
Click a face with less than four logical pixels of motion and verify selection
without camera motion. Try menu, tree, Action Box, overlay, divider and resize
targets to verify their existing actions win. Check right-button orbit,
middle-button pan, wheel scroll and native pinch. Begin a primary drag, move
outside the window, release and return; pointer motion should no longer orbit.
Repeat after loss of focus. Physical-device behavior has not been accepted by
automated tests. No commit, push or merge was made.
