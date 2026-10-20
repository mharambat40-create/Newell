# M3 camera mechanics validation — 2026-10-19

## Implemented interaction and ownership

The approved mechanics plan replaces the previous fixed-target trackball.
`newell-app` owns f64 camera pose/projection, quaternion math, drag snapshots,
trackpad stream ownership, CPU picking and exposed-view bounds. `newell-render`
consumes immutable finite-f32 matrices and writes the existing 96-byte scene
uniform. BrepKIT mesh generation, WGPU pipelines/shaders, the ASCII interface,
UI hit testing and event-driven redraw are preserved. No dependencies changed.

Right-drag applies camera-relative yaw, levels the horizon against Z-up, and
applies signed pitch clamped before rotation. Both eye offset and orientation
use the same final quaternion, keeping a frozen dynamic pivot fixed in camera
space and at its original pixel. The pivot is the nearest positive displayed
mesh hit, or the AABB centre of triangle portions clipped to the WGPU frustum
and exposed viewport rectangles. Starting a gesture changes no camera state.
The pole margin is 0.5 degrees; discarded vertical overscroll allows immediate
reversal. Exact top/bottom commands preserve a deterministic departure heading.

Middle-drag pans at contextual geometric depth. Wheel and smooth DPI-normalized
scroll/pinch zoom toward a surface hit or focal-plane anchor. Phased gestures
freeze their anchor, including zero-delta start events. Orthographic projection,
parallel-ray picking, anchoring, principal-axis alignment and fit are supported.
Fit recovers geometry after unrestricted pan and uses the largest exposed region.
Viewport resizing preserves physical-pixel image positions using lens shift;
Action Box changes do not reframe the camera. Mesh replacement preserves pose.

Only the initiating mouse button releases capture. UI ownership, layout,
projection, mesh, focus, exit and cancellation end capture without moving the
camera. A rejected trackpad stream cannot enter the viewport midway through it.
Panel scrolling keeps its earlier conversion independently of camera sensitivity.

## Corrections to the proposed mathematics

1. **Level yaw before bounded pitch:** unrestricted pitch followed by `asin`
   can fold elevation through a pole and reverse heading. The implementation
   adds signed pitch to the leveled yaw elevation and clamps before constructing
   the final orientation. Pitch acts about the leveled screen-right axis.
2. **Discard pole overscroll:** gesture origin adjustment prevents a dead zone
   when reversing at a saturated pole. Unclamped drags still use a start snapshot.
3. **Orthographic projection:** lens shift is an affine translation, not the
   perspective depth-column shift. Returning to perspective after orthographic
   zoom preserves framing at the focal plane by translating eye along forward.
4. **Anchored limits:** perspective zoom intersects anchor-depth and focal-depth
   scale intervals. New bounds after mesh replacement do not snap the existing
   camera; input moves toward the valid interval, and fit explicitly recovers it.

These details are also recorded in §13 of the approved mechanics plan.

## Validation results

- `cargo check --workspace`: passed.
- `cargo fmt --all --check`: passed. Only task-related source files were formatted.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test --workspace --no-fail-fast`: **698 passed, 38 failed, 9 ignored**.
  All 38 failure names exactly match the baseline run before any task edits;
  there are no added failures. Baseline: 670 passed, 38 failed, 9 ignored.
- New GPU-independent coverage: **19 camera tests, 19 navigation/picking tests,
  1 input cancellation test and 1 renderer payload test**, all passing. Twelve
  previous renderer camera tests were replaced by the Controller-side coverage.
- Focused input suite: **9 passed**. Existing controller integration tests also
  pass in the workspace run.
- `cargo test -p newell-render -- --ignored`: **all 9 GPU tests passed**, including
  BrepKIT mesh rendering, immutable-camera consumption across mesh/layout updates,
  UI monochrome output, atlas reuse and surface cropping.
- `git diff --check`: passed.
- Git index checked byte-for-byte via the staged binary diff: unchanged.
- Existing file hashes were captured before editing. Task edits are confined to
  the files listed below. A separate external edit appeared during execution in
  `crates/newell-ui/src/shell/compose.rs` (status-bar-hidden body height); this
  task did not create, format, revert or overwrite it. The final workspace run
  includes that edit and has the same baseline failures. Other unrelated files
  remain identical to the captured starting contents. No commit was created.

The existing dependency `block v0.1.6` emits a future-Rust-incompatibility notice;
strict Clippy succeeds on the installed toolchain.

## Files changed by this task

- `ARCHITECTURE.md`
- `crates/newell-app/src/app.rs`
- `crates/newell-app/src/camera.rs`
- `crates/newell-app/src/input.rs`
- `crates/newell-app/src/lib.rs`
- `crates/newell-app/src/navigation.rs`
- `crates/newell-app/src/picking.rs`
- `crates/newell-app/tests/it/camera.rs`
- `crates/newell-app/tests/it/input.rs`
- `crates/newell-app/tests/it/main.rs`
- `crates/newell-app/tests/it/navigation.rs`
- `crates/newell-render/src/camera.rs`
- `crates/newell-render/src/lib.rs`
- `crates/newell-render/src/renderer.rs`
- `crates/newell-render/src/scene.rs`
- `docs/backlog.md`
- `docs/plans/M3-camera-mechanics.md`
- `docs/plans/M3-camera-navigation.md`
- `docs/validation/M3-mechanics-2026-10-19.md`

## Inherited failures reproduced before implementation

One app playhead-separator test, six UI overlay unit tests and 31 UI integration
checks/snapshots fail in both baseline and final workspace runs. No UI source,
UI tests, snapshots or unrelated expectations were changed by this task.

- `command_bar::pointer_opens_switches_activates_and_dismisses`
- `hover::formula_modal_ok_hovers_and_the_box_beneath_does_not`
- `menu_bar::projection_and_shading_states_match_the_reference`
- `menu_bar::the_view_popup_fits_the_minimum_grid`
- `menu_bar::theme_row_switches_and_leaves_the_cells_untouched`
- `menu_bar::ui_scale_visits_the_four_reference_states_and_wraps`
- `menu_bar::view_popup_matches_the_reference_in_its_default_state`
- `overlays::tests::list_draws_window_with_scrollbar`
- `overlays::tests::list_scrolled_and_clamped_offsets`
- `overlays::tests::list_small_and_zero_regions`
- `overlays::tests::list_widget_ids_hit_test`
- `overlays::tests::selection_panel_style_composition_end_to_end`
- `overlays::tests::thumb_tracks_position_and_proportion`
- `snapshots::base_shell`
- `snapshots::boolean_cut_offset_faces`
- `snapshots::create_menu_open`
- `snapshots::divider_dragged_up`
- `snapshots::extrude_add`
- `snapshots::extrude_new`
- `snapshots::extrude_remove`
- `snapshots::features_tree_collapsed`
- `snapshots::file_menu_open`
- `snapshots::formula_editor_open`
- `snapshots::hover_button_cancel`
- `snapshots::hover_button_ok`
- `snapshots::hover_checkbox_row_nineteen`
- `snapshots::hover_menu_row`
- `snapshots::hover_parameter_row`
- `snapshots::loft_start_type_match`
- `snapshots::playhead_after_hub_features`
- `snapshots::rename_modal_open`
- `snapshots::revolve_partial_up_to`
- `snapshots::selection_panel_open`
- `snapshots::starting_offset_disabled_then_enabled`
- `snapshots::sweep_keep_orientation_twist`
- `snapshots::wide_layout`
- `status::complete_shell_matches_reference_in_both_themes_and_both_heights`
- `tree::dragging_the_playhead_row_moves_only_the_separator`

## Physical-device and visual acceptance still required

No manual acceptance of the running application is claimed. Check:

- Mouse right/up rotation at viewport centre/edges, diagonal drags, both pole
  limits, reversal after overscroll, and departure from exact top/bottom views.
- Wheel, two-finger scrolling, OS natural-scroll settings, native pinch and
  trackpad momentum, including pointer movement across UI during a stream.
- Off-centre surface pivots, miss pivots after pan, Action Box toggles and live
  viewport resizing, checking for any apparent image movement.
- Perspective/orthographic switches after anchored zoom, principal-axis views,
  extreme model sizes, near-surface clipping and Home/View-menu recovery.

Ray hits use the displayed BrepKIT tessellation rather than exact analytic
B-Rep intersections. Exposed bounds do not remove surfaces hidden behind other
model surfaces. Geometry queries remain O(T) in triangle count with cached bounds;
UI lookup remains O(1). Native pinch uses the last pointer position because its
event supplies no separate centroid. The first nonzero drag from an exact pole
enters the documented 0.5-degree orbit band. Rendering precision is still f32 at
the WGPU mesh/uniform boundary despite f64 Controller calculations.
