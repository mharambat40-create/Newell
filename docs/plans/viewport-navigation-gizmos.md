# Viewport navigation gizmos — implementation plan

**Revision:** 2, 2026-10-27. **Status:** Proposed implementation; documentation
amendments authorized by the current request. **Role/provider:** Architect / Codex.
**Task:** viewport-navigation-gizmos, scoped navigation-widget milestone.
**Executor target:** Luna, medium reasoning; follow the ordered increments and
explicit contracts below. No implementation, build, test, application launch,
render experiment, agent dispatch or integration occurred in this stage.

## 1. Contract and inspected starting state

Existing `/Users/martin/Desktop/Newell` checkout, branch `main`; no branch or
worktree. Initially unstaged: `crates/newell-app/src/geometry_worker.rs`,
`crates/newell-model/src/geometry.rs`, `crates/newell-model/src/lib.rs`; initially
no staged or untracked changes. Relevant edits include worker lifetime identity,
boxed worker replies/Extrude state and geometry dependency/merge validation.
These inputs are outside this task and must be preserved. They were still being
edited during inspection; this plan does not claim a frozen or passing baseline.
Reinspect status and affected input interfaces before Executor changes. No source
snapshot or integration commit is required. Never repair unrelated baseline
failures under this plan; report them distinctly.

Authoritative behavior: `ARCHITECTURE.md` §§1, 3.7, 4.4; canonical
`docs/ui_design/README_UI.md` §41 for dimensions, materials, labels, placement,
visibility, input and camera semantics; §§25/25.1, 28.3, 36–37 for existing
layout/menu/grid routing. The plan references these rules rather than redefining
ASCII controls. Existing camera mechanics remain governed by
`docs/plans/M3-camera-mechanics.md`, including its implementation correction.

Architect outputs in this stage: this plan, `ARCHITECTURE.md` §3.7,
`docs/ui_design/README_UI.md` §41 plus its §28.3 cross-reference, and new draft
`docs/adr/0007-viewport-navigation-widgets.md`. User explicitly authorized narrow
architectural/UI amendments despite the ordinary Architect write restriction.
ADRs 0002, 0003, 0004, 0005 and 0006 were inspected; accepted ADR files are
unchanged. §3.7 explicitly limits supersession of the blanket old restrictions.

### Current implementation inventory

| Actual file(s) | Finding / reuse or replacement |
|---|---|
| `crates/newell-app/src/camera.rs` | f64 `Camera`, quaternion, `basis()`, derived `target()`, focal distance, lens shift; reuse. `look_at`/normal alignment already support Z-up, exact poles and heading. Add one pose operation, no second camera. |
| `crates/newell-app/src/navigation.rs` | sole mutable camera, revision, cancel, layout/occlusion, CPU picking, fit, deferred primary orbit, 4 logical-pixel slop, phased pan/zoom ownership; extend with named direction command. |
| `crates/newell-app/src/app.rs` | maps shell grid rects to scene pixels; `left_pressed` calls shell press then starts deferred orbit, release certifies CAD selection; must intercept widget presses before either path. Publishes camera on many routes; consolidate widget synchronization at those existing routes. |
| `crates/newell-app/src/controller.rs` | `ShellController::press`, scene eligibility and resize priority, focus/numeric/hover routing; expose read-only admission classification, not a second camera. |
| `crates/newell-ui/src/shell/compose.rs` | `CUBE_WIDTH=9`, `CUBE_HEIGHT=4`, `draw_viewport` rounded inert ASCII box/title; remove box/title. Four-cell Action Box top reservation is a separate retained layout rule. |
| `crates/newell-ui/src/shell/mod.rs` | scene interior, action/top-overlay occlusions and View visibility index 3; reuse/expose flag and separate UI footprints. |
| `crates/newell-render/src/renderer.rs` | clear → scene → one ASCII draw; add dedicated navigation offscreen work and composite between scene and ASCII. |
| `crates/newell-render/src/camera.rs`, `scene.rs`, scene WGSL | matrix snapshot, exposed-region masking and mode/selection pipelines; reuse scene occlusion API, keep widget resources out of scene paths. |
| `crates/newell-render/src/ui/{pipeline.rs,shader.wgsl}` | one cell draw with role ink, transparent blanks and nearest glyph sampling; leave intact. ASCII cannot alone hide widget pixels under a blank menu cell; widget composite must mask full UI rectangles. |
| `assets/FiraCode-Regular.ttf`, `assets/{README.md,OFL.txt}`, `glyph_atlas.rs` | existing Fira Code Regular with OFL 1.1 notice; reuse font parsing/rasterization concepts, not the fixed-cell atlas contract. |
| app camera/navigation/input integration tests, render GPU tests, shell tests | extend focused regression coverage; preserve geometry/selection/numeric tests. |

There is no existing interactive cube mesh, cube shader, signed named-view table
or axis widget to reuse. Existing named Front/Back semantics were not found;
§41.4 now assigns them consistently with the actual right-handed Z-up basis.
`Interface_Exemple.txt`, referenced by older code/docs, is absent in this checkout.
Do not invent/recreate it or treat it as an available visual acceptance fixture.
At initial inspection the MenuBar reference already had `■ Navigation cube`;
no menu artwork edit was needed for the requested two-widget design. A later
independent selector edit was resolved by the human's follow-up in §8.
Revision 2 renames its parent View Navigator and defines the conditional child. Preserve all other ASCII reference files.

## 2. Camera contract and complete orientation table

Introduce `ViewDirection([i8;3])` with checked construction: components -1/0/+1,
not all zero. The 26 allowed triples are exactly cube patch IDs. `AxisCap`
contains axis X/Y/Z and positive/negative sign and maps to a triple with one
nonzero component. Do not use human label strings to drive commands.

Let N be the integer **target-to-eye** direction in the table. The column F is
an unnormalized forward vector; normalize both F and the listed U independently.
`R = normalize(F × U)`; recompute `U = R × F` for numerical orthogonality. Every
row is deterministic and independent of incoming roll or heading.

| Component | N (target → eye) | F (eye → target), normalize | U, normalize |
|---|---|---|---|
| Top | (0,0,1) | (0,0,-1) | (0,1,0) |
| Bottom | (0,0,-1) | (0,0,1) | (0,-1,0) |
| Front | (0,-1,0) | (0,1,0) | (0,0,1) |
| Back | (0,1,0) | (0,-1,0) | (0,0,1) |
| Left | (-1,0,0) | (1,0,0) | (0,0,1) |
| Right | (1,0,0) | (-1,0,0) | (0,0,1) |
| Top–Front | (0,-1,1) | (0,1,-1) | (0,1,1) |
| Top–Back | (0,1,1) | (0,-1,-1) | (0,-1,1) |
| Top–Left | (-1,0,1) | (1,0,-1) | (1,0,1) |
| Top–Right | (1,0,1) | (-1,0,-1) | (-1,0,1) |
| Bottom–Front | (0,-1,-1) | (0,1,1) | (0,-1,1) |
| Bottom–Back | (0,1,-1) | (0,-1,1) | (0,1,1) |
| Bottom–Left | (-1,0,-1) | (1,0,1) | (-1,0,1) |
| Bottom–Right | (1,0,-1) | (-1,0,1) | (1,0,1) |
| Front–Left | (-1,-1,0) | (1,1,0) | (0,0,1) |
| Front–Right | (1,-1,0) | (-1,1,0) | (0,0,1) |
| Back–Left | (-1,1,0) | (1,-1,0) | (0,0,1) |
| Back–Right | (1,1,0) | (-1,-1,0) | (0,0,1) |
| Top–Front–Left | (-1,-1,1) | (1,1,-1) | (1,1,2) |
| Top–Front–Right | (1,-1,1) | (-1,1,-1) | (-1,1,2) |
| Top–Back–Left | (-1,1,1) | (1,-1,-1) | (1,-1,2) |
| Top–Back–Right | (1,1,1) | (-1,-1,-1) | (-1,-1,2) |
| Bottom–Front–Left | (-1,-1,-1) | (1,1,1) | (-1,-1,2) |
| Bottom–Front–Right | (1,-1,-1) | (-1,1,1) | (1,-1,2) |
| Bottom–Back–Left | (-1,1,-1) | (1,-1,1) | (-1,1,2) |
| Bottom–Back–Right | (1,1,-1) | (-1,-1,1) | (1,1,2) |

For non-poles, equivalent formula is U=normalize(Z−F*dot(Z,F)); keep tests based
on the explicit independent table. At Top/Bottom set `last_heading=(0,1,0)`;
else `last_heading=normalize((F.x,F.y,0))`. These choices reproduce current
`look_at`'s exact-pole heading policy without inheriting arbitrary previous roll.
Widget commands produce exact poles; subsequent ordinary orbit obeys existing
0.5-degree clamp and departure mechanism.

Add camera alignment around an explicit world pivot and screen anchor. For
widget activation, `Navigation::align_view` selects the existing contextual
pivot (scene hit at viewport center, else visible model-bounds center, else the
camera target), captures its projected pixel and perspective depth, then applies
the deterministic named-view basis while retaining that pixel and depth.
Preserve focal distance, orthographic_height, perspective, vertical_fov,
lens_shift and viewport exactly. This changes camera pose only; do not fit,
recenter or translate document geometry. With no visible model, use the existing
target fallback. Invalid anchors leave camera state unchanged.
`Navigation::align_view` cancels gestures, invokes pose operation and uses
existing `record(before)`/snapshot revision machinery. If already aligned,
no camera revision or redraw solely for a pose change. Do not change existing
`normal_to_axis`, Reset Cam or Home command behavior.

## 3. Ownership and interfaces

### Pure UI geometry (`newell-ui`)

Add one public `navigation_widgets` module, plain values/functions, no retained
widgets, OS types, font parser, GPU types or mutable camera:

- `SignedAxis`, `AxisCap`, `ViewDirection`, `NavPart::{Cube(ViewDirection), Axis(AxisCap)}`.
- `NavigatorKind::{Gizmo, Cube}`; `NavLayout { axis: Option<PixelRect>,
  cube: Option<PixelRect>, generation }` with at most one nonempty tile;
  `NavBasis { right:[f64;3], up:[f64;3], forward:[f64;3] }` validated finite and
  orthonormal. Use dedicated physical `PixelRect` to avoid confusing grid units.
- `CubePatch { direction, plane, vertices }`, fixed 26 patches. Generate once
  by clipping each supporting plane polygon against the other half spaces;
  triangulate convex fans; store stable component IDs and outward normals.
  This is bounded widget tessellation, not a CAD/kernel operation. The 26
  halfspaces and thresholds in §41 are the sole cube geometry source.
- `NavGeometry { layout, basis, cube projected polygons/triangles,
  axis descriptors }` computed from existing scene camera basis; size fixed
  apart from bounded convex polygon vertex arrays. No camera target/mesh data.
- `layout_navigation(scene_pixels, surface_pixels, metrics, visible, kind,
  action_box_pixels) -> NavLayout`; `prepare_geometry(layout,basis)`;
  `hit_navigation(&geometry, physical_point) -> Option<NavPart>`; separate
  `reserves_point` returns tile reservation even with no active part.
- `NavMetrics` derives q from published GridMetrics and supplies §41 dimensions,
  thresholds, colors/alphas and axis shape descriptors to both consumers.
  No duplicate Rust constants in app and renderer. GPU colors convert once.

Halfspace picking: transform local pixel ray using basis; origin beyond the
cube's bounding sphere, ray forward F. Clip ray interval against all 26 planes.
Track maximal entering t and active support normals, reject parallel-outside
planes and entry>exit. Entry-plane tie rules are §41.4. Rendering uses the
actual clipped polygons of precisely those planes. Zero-area/edge-on projected
polygons never become visible target patches by numerical inflation.

Axis hit masks use analytic circle/annulus and sphere coverage/depth, identical
primitive order as rendering; skip transparent ring interiors and ignore AA
fringe. Positive shafts only. Return top visible cap after opaque coverage,
not simply nearest projected center. No CPU plane/shaft hit actions.

### Conditional View menu integration (`newell-ui`)

Keep visibility at `view.checks[3]` (existing action 213) and rename only its
label to View Navigator. Add `navigator_kind: NavigatorKind` to window-local
`ViewMock`, default Gizmo; expose a typed read-only shell accessor. Add stable
`ACT_VIEW_NAVIGATOR_KIND=225` (unused in inspected data.rs); avoid shifting
existing actions 214/220–224. Replace fixed `[MenuEntry;13]` with a small local
Vec of 13 unchecked / 14 checked entries built by `with_bar_menus`. Insert child
immediately after the parent only when checked, before Grid Overlay.

Reuse `MenuEntry::Cyclic`: label prefix `"╰╴  "`, two kind values padded to a
six-cell value slot and a ten-cell selector control. Existing cyclic rendering
fills all remaining row width, which would put the right arrow at the wrong
column. Add `control_width: Option<u32>` to `MenuCycle`; None preserves every
existing caller, Some(10) caps only this child control after the existing label
prefix offset. Match the user's 21-cell row exactly: left arrow column 7,
right arrow column 16, outer borders columns 0/20 (zero-based row coordinates).
Keep trailing padding owned by the row; clicks there follow the existing
cyclic-row no-activation behavior. popup_size must honor the compact width
without widening the frame beyond the 21-cell reference. The new branch prefix
uses Text, including hover replacement inheritance; arrows use Action.

On parent toggle recompose popup dimensions and selection immediately; keep
parent focus by its action ID. Cycling keeps child focus. Clear/remap row-based
press/hover IDs after any insertion/removal; never retain a later row index as
an action identity. Preserve existing cursor-key/Enter/Space cyclic semantics
and all other menu item meanings. Test Grid Overlay, Normal to, Reset Cam and
Fullscreen on their shifted rows in both states. Pass the typed kind along
with visibility into the app's layout publication; mode switch invalidates
pending widget capture and schedules one redraw, without changing camera pose.

### Controller (`newell-app`)

Add `navigation_widgets.rs` with window-local `NavInteraction`:
`Idle | Pressed { part, origin_px, max_displacement, scale_at_press,
layout_generation, camera_revision } | CancelledUntilPrimaryRelease`.
The cancelled state is necessary to prevent a delayed release entering scene
selection or shell activation after a resize/overlay event. Track initiating
button explicitly; secondary releases do not end primary capture.

`App` owns latest NavGeometry and this press state; `Navigation` owns the
camera. No app-owned clone is independently mutable. App constructs immutable
geometry from `navigation.camera().basis()` after every camera/layout change.
Use the same published value for hit testing and renderer upload in one event
turn. Include camera revision in `NavSnapshot` for synchronization diagnostics.

Expose ShellController read-only `navigation_admission(point,surface,resizable)`
and UI footprints/visibility forwarding. It performs existing grid hit/modulo
routing, menu/modal gating, controls, resize zones, window-drag and action-box
occlusion classification **without** calling `pointer_pressed` or ending an
edit. This closes the current `left_pressed` hazard: calling `shell.press`
first can activate/dismiss/change focus before a widget has been classified.
No general input framework or retained widget tree is needed.

App input order:

1. Existing active native/managed resize, shell drag and scene gesture captures
   retain ownership. A fresh primary press follows the existing stale-capture
   cancellation policy. Active scene drag moving across widgets continues.
2. Existing menus/modals/ASCII owned cells/window controls/resize/window-move
   route normally. Read menu state before routing: a press that closes a menu
   is consumed and cannot activate widgets or orbit later in that handler.
3. On eligible tile primary press, clear pending scene-selection certificate,
   cancel uncaptured navigation streams, clear ordinary hover via existing
   mechanism, arm component if any and consume. An empty reserved tile press
   uses cancelled-until-release state and is also consumed. Preserve numeric
   edit session/focus; do not call shell press/release activation path.
4. During widget press record maximum displacement from origin; once over
   4*OS-scale physical pixels, cancel permanently until primary release. No
   drag command. Same-component release below threshold and with valid layout,
   camera and mesh lifetime invokes Navigation::align_view exactly once.
   Consume releases including cancellation; no CAD selection certificate.
5. For fresh auxiliary/scroll/pinch streams, UI gets first refusal. Tile
   reservations deny scene admission. Existing captured streams retain their
   frozen owner/anchor even if the pointer crosses a tile. Blocked phased
   streams stay blocked until end/cancel through existing gesture gates.
6. Scene picking/orbit/pan/zoom and ordinary shell pointer updates otherwise
   keep current behavior. Hand cursor is a computed cursor hint, not a second
   hovered target or hover material. Escape cancels widget capture before
   ordinary Escape routing; preserve current shell cancellation semantics.

Cancellation hooks must cover every `navigation.cancel`, mesh replacement,
projection/camera command, layout/occlusion change, scale commit, resize,
focus loss, CursorLeft, minimization/occlusion, close and visibility/kind change.
Camera revision changes cancel widget presses; ordinary shell redraws with
identical geometry/layout do not. Keep widget layout generation stable across
FPS updates. Mesh revision is either stored explicitly or cancels capture in
the existing mesh publication hook; never leave a stale primary release live.

Separate UI occlusions from widget tile reservations. App publishes combined
occlusions to Scene and Navigation for scene masking/picking/fitting/contextual
bounds, and **UI-only** occlusions to widget composition. Deduplicate equal
rectangles; do not feed combined occlusions back into widget placement. On
visibility changes without scene viewport resize, camera pose stays unchanged.

### Renderer (`newell-render`)

Add `NavOverlay` owning immutable input, pipelines, label atlas, buffers and
local targets. Public `Renderer::set_navigation(snapshot)` only consumes
prepared geometry; no picking, gestures, camera mutation or geometry queries.
A snapshot includes layout/camera generations, basis, rectangles, UI-only
occlusions and projected cube data. Passing `None` clears widgets/reservations
on the Controller side as well; hidden widgets issue zero dedicated draws.
Renderer exposes no second scene camera setter for this feature.

`NavOverlay` label fitting stays in View, because it affects only ink, not
component geometry or hit ownership. Fit actual word ink bounds into projected
face polygons as in §41; labels never expand a hit region. Cover only the
front-facing polygon. A word that cannot fit at minimum cap height is omitted,
not shortened. Label orientation is always screen upright, so no winding-
induced mirrored glyphs or per-face text rotations are required.

## 4. Rendering, shaders and serialized data

Use existing wgpu 26 and ab_glyph 0.2.32; no new dependencies, binary assets,
font download, MSRV increase or Cargo.lock edit. Rust baseline remains 1.88
(ADR 0004), safe Rust only. Follow existing explicit little-endian GPU byte
serialization; do not add bytemuck/unsafe casts just for uniform uploads.

### Resources and draw sequence

`NavOverlay` allocates a group-local linear `Rgba8Unorm` premultiplied color
attachment with 4x MSAA, single-sample resolve texture, and separate
`Depth32Float` 4x depth attachment. Confirm supported format/sample-count
capabilities during creation and report a RenderError on unsupported hardware;
do not silently render aliased widgets or change scene resources. Target bounds
are the selected tile rectangle (96q square Gizmo and Cube);
local offsets are translated consistently. At maximum k=7 both tiles are
336×336 px,
not a surface-sized target. Create/recreate only when target size changes, not
on camera movement, theme, Color or display-mode changes. Skip allocation/draw
when the selected navigator is hidden/suppressed; inactive zero-sized surface follows existing skip.
Keep one active target set per window; drop obsolete targets after replacement.

Rendering order per frame:

1. Existing clear and scene passes, with combined UI+tile scene occlusions.
2. Dedicated local navigation render pass: transparent clear, independent depth
   clear to 1, no scene depth attachment or scene uniform. For Gizmo: axis
   analytic quad
   (one draw, no depth writes). For Cube: faces and only the hovered bevel
   patch contribute opaque pixels (one indexed draw, Less depth test and
   writes), followed by eligible front face labels (one
   batched instanced draw, no depth writes). Never draw both representations.
   Empty buffers cause their draw to be skipped.
3. Resolve local 4x color. Composite resolved navigation texture onto surface
   (one draw covering group bounds) with premultiplied source-over. Discard all
   samples outside active tiles, actual scene/surface intersection, or inside
   UI-only occlusion rectangles. This masks even blank ASCII menu/Action Box
   cells. Do not sample CAD scene color/depth. Composite must never clear the
   scene or draw an opaque tile background.
4. Existing ASCII overlay draw, unchanged. Account separately: exactly **one
   ASCII draw**, at most **three dedicated navigation draws for Cube** or
   **two for Gizmo**, plus existing mode-dependent scene draws. Zero navigation
   draws when View Navigator is unchecked or selected widget is suppressed;
   no extra draw for selection, hover or Color.

The local target contains linear RGB: convert §41 sRGB hex colors to linear
before shading/blending, premultiply once. At composite output keep linear RGB
for an sRGB surface (hardware encodes); for an unorm surface encode the final
unpremultiplied color to sRGB then premultiply again before blend, consistent
with that surface's existing color convention. Test both paths. Never apply
the model's 1.10 multiplier or ASCII role lookup to widget fragments. Theme and
Color can change the clear seen through transparent bevels; cube faces and the
currently hovered edge/vertex retain their existing materials.

Axis shader implements §41.2 with bounded analytic local ray evaluation: three
finite planes; sphere; three positive 2D antialiased ribbon shafts with depth
interpolated along projected positive direction. For a shaft whose projection
length is <=1e-9 units, emit no ribbon (cap still exists). Evaluate all coverage
terms before derivative-dependent branches; clamp numerical divisors. Each
plane contributes one premultiplied fill/border sample; sort up to seven samples
by depth/ID, then apply visible cap annotations. Sphere opaque coverage clips
behind-sphere contributions. Plane outline uses analytic signed-distance
coverage, not a grid. Caps have independent center-depth order and sphere
masking as specified. Use explicit-level sampling for cap-label atlas reads
inside variable-control branches. No per-frame randomness or time uniform.

Cube polygons come from UI's halfspaces; use outward winding and backface
culling. Transform with the supplied basis into the widget's local orthographic
clip volume, mapping the finite bounding sphere depths into (0,1). Keep margins
from near/far to avoid clipping a principal view. Labels use clipped face
polygons, no depth bias or duplicate coplanar faces: front polygons of a convex
cube have nonoverlapping interiors; clipping label fragments to their own face
and drawing them after the cube avoids z-fighting. Never show back-face ink.
Cap and label AA remains coverage rather than theme ink.

### Concrete GPU layouts

One shared `NavUniform` is **128 bytes**, eight 16-byte lanes (offsets below).
Use explicit padding/serialization assertions; WGSL vec4 alignment throughout:

| Offset | WGSL field | Contents |
|---:|---|---|
| 0 | `right: vec4<f32>` | camera R xyz, pad 0 |
| 16 | `up: vec4<f32>` | camera U xyz, pad 0 |
| 32 | `forward: vec4<f32>` | camera F xyz, pad 0 |
| 48 | `group: vec4<f32>` | physical surface origin xy, target width/height |
| 64 | `axis: vec4<f32>` | local center xy, q, projection length 31q |
| 80 | `cube: vec4<f32>` | local center xy, basis-matched projection scale, reserved 0 |
| 96 | `flags: vec4<u32>` | hovered cube patch id plus one (zero means none), cube enabled, sRGB surface flag, UI-mask count |
| 112 | `surface_atlas: vec4<f32>` | physical surface width/height, label atlas width/height |

Additional bind groups/data:

- shared uniform at binding 0; geometry/material read-only storage at binding 1
  if needed for analytic descriptors. Encode axis dimensions/colors via a
  fixed `AxisStyle` block of 16-byte lanes generated from UI NavMetrics,
  including sphere, cap/ring and plane values; do not duplicate style literals
  in WGSL. Descriptor size and offsets asserted together with shader tests.
- Cube vertex stride **48 bytes**: position vec3<f32> at 0, patch u32 at 12,
  normal vec3<f32> at 16, material u32 at 28, seam-weight vec4<f32> at 32.
  Optional smooth seam lighting weights are prepared from the fixed geometry;
  never move positions to create softness. Index buffer u32.
- Label instance stride **64 bytes**: rect xywh vec4<f32>, UV rect vec4<f32>,
  face clip-polygon offset/count and label ID/pad vec4<u32>, source RGBA
  vec4<f32>. Face clip vertices are vec4<f32> storage entries, xy screen-local,
  zw reserved. The label shader evaluates convex halfplanes of that face.
- UI masks: read-only storage `array<vec4<f32>>` physical surface left/top/right/
  bottom, explicit count from snapshot. Current shell produces at most two
  footprints. No storage/readback writes. ASCII CPU routing remains O(1);
  this mask loop follows the existing bounded overlay list, not a UI tree.
- Label atlas: `R8Unorm` coverage texture and own clamp/linear sampler, separate
  from ASCII and logo bind groups. Composite: resolve texture plus clamp
  sampler (pixel-aligned 1:1; no scene blur). Use typed renderer resources so
  scene display-mode setter cannot switch a navigation pipeline accidentally.

`NavSnapshot` uses f64 for CPU layout/projection/hit preparation; serialize
finite f32 only. Invalid basis/nonfinite geometry -> hide snapshot and return
an error to app for one-time reporting, never propagate NaNs or activate a
stale hit map. Local GPU allocation/preparation failure must leave both the
old render and hit snapshot active together, or disable both together before
redraw; never publish a new hit layout with old graphics. Integrate atlas/target
preparation into the existing k transition's prepare/commit boundary.

### Font/asset verification and bounds

Read-only asset metadata inspection in this stage found family **Fira Code**,
style **Regular**, cap-height **1377 font units**; `ARCHITECTURE.md` records
units-per-em 1950 and advance 1200. PANOSE family/serif bytes are unspecified
(0/0), so those bytes alone do not classify the style. Use the existing Fira
Code sans-serif monospace letterforms for these short labels; proportional
spacing is not a requirement. No asset substitution or platform/system font.

For desired cap height h, use uniform font scale h/1377 and natural advances;
Bottom advance is `6*1200*h/1377` (about 47.06 at h=9), fitting the 51.2-wide
principal face center before inset. Actual ink bounds, not advance alone,
determine the final fit. Cube-face labels render at 85% of the former size:
7.65q normally and 5.1q minimum after face fitting; X/Y/Z remain 8q. Check cmap
for all required letters in a focused Executor test; absence must be a reported
asset issue, never '?' in a cube word. Rasterize the nine complete labels
once per k, at their largest needed cap height, with at least 2 physical-pixel
transparent padding on every side; no atlas mutation/rasterization during
pointer events or frames. Store glyph/word ink bounds and baselines. Fractional
scale is permitted only here. Filtered atlas sampling handles the bounded
7.65q→5.1q face-label fit; no mipmaps are required for this 1.5:1 minification.

OFL 1.1 permits bundling/embedding; keep original font and `assets/OFL.txt`
unchanged. All new Rust files need the project's AGPL header. Existing ab_glyph
and wgpu dependencies remain unchanged, so no new third-party license/MSRV
assessment or network lookup is needed for this plan. If an Executor finds a
new asset necessary, stop only that dependent label work and report the actual
legibility/coverage failure and license choice; do not silently expand assets.

## 5. Exact implementation file scope

Paths below are relative to the existing checkout. New files are marked **new**.
Executor must preserve unrelated edits within shared files, especially app.rs.
No model/kernel/worker/selection/numeric implementation changes are authorized.

| File | Required responsibility |
|---|---|
| `crates/newell-ui/src/navigation_widgets.rs` **new** | shared types, style/metrics, 26 halfspaces/polygons, pure projections/layout and hits; focused unit tests |
| `crates/newell-ui/src/lib.rs` | export pure module/types |
| `crates/newell-ui/src/shell/compose.rs` | remove placeholder box/title; rename CUBE_HEIGHT reservation to ACTION_PANEL_TOP_RESERVATION without changing its four-cell behavior; remove unused width and placeholder-only title reservation |
| `crates/newell-ui/src/shell/mod.rs` | public navigation visibility/kind/UI-only footprint access; cyclic kind handling and menu focus remapping; document reservation ownership |
| `crates/newell-ui/src/shell/data.rs` | rename check label, add default Gizmo kind and stable action 225, conditional View entries (13/14 rows) |
| `crates/newell-ui/src/menu/mod.rs` | optional compact cyclic control width, default None at existing construction sites; current Enter/Space/arrow semantics preserved |
| `crates/newell-ui/src/menu/draw.rs` | honor explicit compact width for child selector geometry/hits/roles, keep other rows unchanged |
| `crates/newell-ui/src/menu/tests.rs` | update existing literals for optional width and test exact compact child layout/routing |
| `crates/newell-ui/src/shell/tests.rs` | replace inert-placeholder assertion with absence/visibility-contract assertions, retain Action Box placement tests |
| `crates/newell-ui/src/shell/resize_tests.rs` | update misleading placeholder comments/assertions only where affected |
| `crates/newell-ui/tests/it/snapshots.rs` | fixture expectation changes only for removed placeholder |
| `crates/newell-ui/tests/it/snapshots/*.snap` | only listed shell snapshots with removed placeholder artwork and open View menu snapshots with approved new rows; bounded procedure below |
| `crates/newell-app/src/camera.rs` | align_view using current basis/quaternion authority |
| `crates/newell-app/src/navigation.rs` | named direction command, cancel/record and immutable revision access |
| `crates/newell-app/src/navigation_widgets.rs` **new** | capture state, gesture slop/cancel, pure application integration helpers |
| `crates/newell-app/src/lib.rs` | register app module (public if integration tests require it) |
| `crates/newell-app/src/controller.rs` | read-only widget admission, visibility/footprints forwarding; leave ordinary routing behavior intact |
| `crates/newell-app/src/app.rs` | per-window state, precedence, reservation/layout/snapshot sync, all camera/mesh/scale/cancel hooks and in-module integration tests |
| `crates/newell-app/tests/it/{camera.rs,navigation.rs}` | table/preservation/departure regression tests |
| `crates/newell-app/tests/it/navigation_widgets.rs` **new** | capture and admission tests |
| `crates/newell-app/tests/it/main.rs` | register integration module |
| `crates/newell-render/src/navigation_widgets.rs` **new** | NavOverlay, CPU serialization, mesh upload, immutable snapshot, local resources/passes/draw accounting |
| `crates/newell-render/src/navigation_labels.rs` **new** | deterministic nine-label atlas, ink fitting and texture upload |
| `crates/newell-render/src/navigation_widgets.wgsl` **new** | cube/label/composite entry points and explicit source-color conversion |
| `crates/newell-render/src/navigation_axis.wgsl` **new** | bounded analytic axis/glass composition |
| `crates/newell-render/src/{lib.rs,renderer.rs,error.rs}` | exports, resource preparation/commit, composite integration, typed errors |
| `crates/newell-render/tests/it/navigation_gpu.rs` **new** | ignored GPU validation/readback and draw-count checks |
| `crates/newell-render/tests/it/main.rs` | register GPU module |

Documentation/reference outputs remain this plan, architecture §3.7, UI README
§§28.3/41, draft ADR 0007 and Application_Menu_Bar.txt. Revision 2's menu
reference change is only the user-requested obsolete-parent label replacement;
its existing indentation/variant rows are preserved.

No new font/shader library/mesh/image asset, Cargo manifest or lockfile change.
Existing scene.rs, scene WGSL and ASCII shader/pipeline are read-only reuse
inputs; their APIs already support the required occlusion and one-draw path.
Use private helpers in the listed files instead of expanding their scope.

Snapshot scope rule: the exact affected filenames found during this inspection
are below. Visible placeholder cells become blank. Additionally, open View menu fixtures
must show the renamed parent and conditional child, and move subsequent rows
and bottom border by exactly one row when checked. These menu changes are
explicitly approved by the human follow-up; hidden parent state has no child.
Overlays covering removed placeholder cells retain ownership. Reconfirm this list at implementation
because the checkout is changing. No automatic snapshot regeneration to obtain
green tests. Every replacement needs an explicit cell-diff review under §41;
outside removed artwork and the specified View menu differences, all characters and roles remain unchanged. Preserve `3D Viewport` at its
current origin. Removing placeholder width reservation does not reflow text.
A reference snapshot newly added by other work is not implicitly in scope.

- `crates/newell-ui/tests/it/snapshots/it__snapshots__base_shell.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__boolean_cut_offset_faces.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__create_menu_open.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__divider_dragged_up.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__extrude_add.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__extrude_new.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__extrude_remove.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__features_tree_collapsed.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__file_menu_open.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__formula_editor_open.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__hover_button_cancel.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__hover_button_ok.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__hover_checkbox_row_nineteen.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__hover_menu_row.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__hover_parameter_row.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__loft_start_type_match.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__playhead_after_hub_features.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__rename_modal_open.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__revolve_partial_up_to.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__selection_panel_open.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__starting_offset_disabled.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__starting_offset_enabled.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__sweep_keep_orientation_twist.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__view_menu_color_checked_dark.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__view_menu_color_checked_light.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__view_menu_color_unchecked.snap`
- `crates/newell-ui/tests/it/snapshots/it__snapshots__wide_layout.snap`

## 6. Ordered increments and exit criteria

Implement in this order, with focused tests after each increment; do not launch
other roles. Intermediate increments need not be externally integrated.

1. **Record baseline and remove placeholder.** Inspect current dirty/index state;
   run baseline checks with snapshot updates disabled. Remove only NavCube ASCII
   artwork; expose existing flag/kind and UI footprints, preserving Action Box offset.
   Add the specified conditional View menu row and focused tests of its layout,
   keyboard/pointer routing and retained kind. Exit: expected blank placeholder cells, same CAD/menu/tree/numeric
   controls and one ASCII draw. Explicitly list accepted fixture cell deltas.
2. **Shared geometry/layout and camera commands.** Add UI types, clipped convex
   cube mesh/ray tests, analytic cap masks, q-based layout/fallback and app
   `align_view`. Exit: 26 nonempty components and all table camera tests green,
   no camera move from layout/visibility alone. No renderer is needed for these
   pure tests. Use true bevel polygons as hit regions, no painted bands.
3. **Renderer resources and fixed materials.** Add local target, label atlas,
   cube/axis/label/composite pipelines and ignored GPU tests. Expose immutable
   snapshot setter. Exit: <=3 Cube or <=2 Gizmo dedicated draws, one ASCII draw, exact formats,
   no validation errors, no scene-depth/material coupling, atlas never mutated
   during a frame. Prepare/commit scale resources before live publication.
4. **App admission/capture and synchronization.** Intercept widget presses before
   shell activation/orbit, handle cancellation and release consumption, publish
   consistent layout/camera snapshots and separate occlusion lists on every
   existing route. Exit: all input isolation/overlay/capture tests pass, no
   scene selection or numeric commit caused by a widget press; wheel/pinch
   gates preserve ownership. Same-geometry FPS update does not cancel a click.
5. **Regression and acceptance.** Run focused tests then the required workspace
   checks once; perform explicit GPU and real-pointer visual checks. Report
   baseline versus new failures, actual device/backend, measured draw counts,
   visual results and any pending items. No builds/tests were run by Architect.

## 7. Acceptance tests and independent evidence

Tests must check behavior against the table/spec, not merely call the same
production helper twice. f64 pose tolerances: 1e-10 for unit/dot/orientation
invariants; contextual-pivot screen error <=1e-7 physical pixels. Scalars
documented as preserved must be bit-identical. CPU/GPU pixel
comparisons allow one physical-pixel AA fringe only; no broad screenshot
threshold that hides wrong ownership/materials.

### CPU and integration matrix

- Independently enumerate all 6+12+8 entries; assert unique nonzero triples,
  adjacency sums, F=-normalize(N), U matching table, handed orthonormal basis,
  deterministic poles and no accumulated roll. Start from multiple rolled,
  panned and lens-shifted poses in both projections. Assert target/distance,
  FOV/height/lens shift/viewport preservation, repeated-command idempotence,
  finite matrices, unchanged model/document/selection. Test huge/small finite
  distances and invalid zero/nonfinite inputs without panic or NaNs.
- At Top and Bottom, apply each first orbit sign, yaw and pitch; verify finite
  orthonormal zero-roll departure, preserved orbit pivot and existing clamp.
  For all other rows world-Z projection defines up. Camera gesture regression
  suite still passes, including reverse motion near poles.
- Validate cube mesh: all 26 patches nonempty, outward winding, convexity, every
  vertex satisfies all planes within tolerance, shared seams match, no gaps,
  and all eight corner bevels/twelve edge bevels are represented. For each
  component view, center its patch toward the viewer and hit its interior.
  Test interior face/edge/vertex pixels, every face–edge and edge–vertex seam,
  three-way ties, silhouette boundaries +/-epsilon, backfaces, empty tile and
  outside surface. Resolve by specified dimension/ID priority; labels do not
  change ID. Compare CPU hits to independent triangle intersection results
  away from the numerical seam tolerance.
- Axis: exactly three positive shafts, no primitive/endpoints/raster contribution
  for negative shafts. For each signed cap at representative orientations,
  test disc/annulus center/boundaries, hollow center, sphere masking, cap
  overlaps, behind-origin alpha, collapsed projections and axial positive-disc/
  negative-ring overlap. Geometry serialization contains no negative shaft;
  GPU test isolates a generic non-overlapping negative radial segment and
  asserts no shaft ink there while positive radial segment has ink. Plane
  contribution must be disabled in that diagnostic to avoid a false positive.
- Capture: primary same-part release acts once; press different part/release,
  movement exceeding threshold and returning, release outside, other-button
  release, Escape/focus loss/exit, mesh/layout/projection/visibility/scale
  changes consume without action. No pending CAD selection certificate or
  deferred orbit is created by any tile press (including empty tile). Exact
  slop boundary: <=4 logical pixels may activate, >4 cancels. Preserve editing
  buffer, caret and focused field; no numeric commit/cancel or undo entry.
- Precedence: menu/modal over every widget, outside menu dismissal at widget,
  Action Box suppression, selection-list scrolling, resize zones and captures,
  existing scene drag/scroll stream crossing widget. Only subsequent fresh
  eligible press activates after dismissal. UI mask blank cells hide all widget
  fragments; widget reservations do not self-mask the composite.
- Visibility/layout: View index 3 is the View Navigator parent, default checked
  with Gizmo independently per window. Unchecking removes the child row and
  hides the widget; rechecking restores prior kind. Both arrows wrap kinds,
  keyboard Left/Right cycles, Enter/Space follows current no-cycle semantics.
  Parent activation retains focus/open popup; child cycling retains child focus.
  Removed child remaps focus safely to parent, later action IDs remain correct,
  no stale press acts on the row moved into its position. Hidden/suppressed tile
  adds no rendering or scene occlusion and cannot receive hits. Kind/visibility
  changes preserve pose and cancel stale capture. Both widgets fit at exact
  112q-square threshold; just below it the selected threshold hides the widget,
  without switching kind. Restore on growth. Action Box overlap suppresses whole tile.
  k=2…7, OS scales 1/1.5/2 with identical physical layout at same k, every four
  UI Scale labels including those mapping to the same k, monitor transitions,
  clipped and zero surfaces. Match render/hit origin rounding, no extra DPI
  multiplication. Layout generation stable for same geometry; cancellation
  atomic when it changes. Home fit excludes visible tile reservations and
  restores them after a hide/show without reframing on the toggle itself.
- Font: all nine labels have cmap coverage and nonempty distinct rasters;
  cap-height sizing/ink bounds fit at principal views. Word fit uses all ink
  corners plus inset inside the convex face. Oblique face omits a word rather
  than clips it below minimum. Atlas cached through theme/Color/camera changes,
  rebuilt/prepared only for k changes, ASCII atlas counts unaffected by widget
  hover/pose. Serialization offsets/strides/alignment and finite f32 checks.

### Explicit GPU checks (Executor only)

Use existing offscreen test style with `#[ignore]` device-dependent tests.
Create actual shaders/pipelines/targets at k=2 and k=7 and both output format
paths; submit, poll and read back with safe buffer mapping. Fail on uncaptured
validation/device errors. Record backend/adapter; no adapter means **not run**,
not a pass. Test principal and oblique views, exact pole, alpha boundaries,
intersecting glass, empty local background, no scene depth interaction, full UI
footprint clipping, label silhouette clipping and hidden-mode zero draws.

Render each navigator kind separately over a test scene occluder and compare their material
samples across all seven display modes and selected/unselected scene state.
Compare source/opaque cap/cube samples between Color on/off and themes; compare
transparent regions to expected premultiplied blending against each background,
not to identical final pixels. Use analytic depth-order pixel fixtures around
plane intersections; repeated identical frames must be pixel-identical on the
same backend. CPU-hit versus readback component-ID diagnostic at representative
interiors/seams validates geometry agreement; the diagnostic is test-only, not
runtime GPU picking. Count actual draw invocations, including composite; assert zero draws for the unselected kind.

Required workspace checks after implementation (baseline first, same commands):

```sh
cargo check --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
INSTA_UPDATE=no cargo nextest run --workspace --all-features --locked
cargo test --workspace --doc --locked
cargo test -p newell-render --locked -- --ignored
```

Without nextest use `INSTA_UPDATE=no cargo test --workspace --all-features
--locked --no-fail-fast`. Check the declared Rust 1.88 baseline on the available
MSRV toolchain; report unavailable toolchain as unverified, do not install or
upgrade dependencies silently. Format only assigned changed files in this dirty
checkout; a workspace fmt check may report pre-existing formatting failures.
Two failed compilation repair attempts trigger the shared workflow stop rule.

### Separate real-device visual/pointer checklist

This acceptance cannot be established by CPU/GPU tests alone. Executor records
pass/fail/pending with display/backend and captures if available:

- Both Dark/Light × Color on/off; k=2,4,6,7 where reachable; default compact
  display and Retina screen. Restrained sizes/spacing, correct pastel X/Y/Z,
  visible sphere, ultra-thin antialiased positive shafts and **no** negative
  shaft. Negative rings remain hollow and findable, including axis overlap.
- Glass looks milky/translucent with three planes and no grid/refraction/blur;
  intersections do not flash or reorder discontinuously while orbiting. No
  accidental occlusion by CAD surfaces, no opaque rectangular background.
- Cube stays light gray/beveled under every scene mode; full words readable
  when frontal, upright and never mirrored/clipped; edge-on words disappear
  according to fit policy. Assess 5.1q minimum explicitly on Full HD; do not call
  unreadable ink acceptable just because a raster test passes.
- Physically click visible face, edge and vertex patches through multiple
  orientations, all six signed caps, and test boundary transitions/negative
  rings. Confirm requested named view and stable Top/Bottom departure, target
  and zoom retained, no CAD highlight/selection or orbit from tile drag.
- Test open menus/modals/selection lists, blank popup areas, Action Box overlap,
  View visibility toggle, empty tile scroll/pinch, captured scene drag crossing
  the tile, focus/exit cancellation, resize and monitor/UI-scale transitions.
  Verify numeric editing, tree selection and CAD commands still work.
- Idle window schedules no extra widget redraw loop; no hover animation,
  Home icon, rotation arrows or scene construction entities appear.

## 8. Decisions, limitations and handoff

**Product conflict resolved by the human follow-up (2026-10-27).** Revision 1
showed both widgets simultaneously. The human supplied exact View Navigator
parent and conditional child rows: unchecked hides both the navigator and child;
checked displays the selected Gizmo/Cube representation. Revision 2 adopts this
latest instruction, with checked + Gizmo defaults, remembered kind while hidden,
no automatic kind substitution on small windows, and unchanged scene camera.
The on-disk reference still said Navigation cube when this correction began;
only that obsolete label was replaced to match the user's exact supplied rows.
No unresolved product decisions remain for this stage.

Other concrete choices resolved:
Front=-Y; fixed pole heading +Y; one View Navigator visibility toggle and
conditional Gizmo/Cube selector, with only the chosen representation visible;
instant pose changes; fixed materials in both themes; temporarily hide the
selected widget when it cannot fit; whole-tile Action Box suppression; existing Fira Code labels
with fit/omit policy and no hover material. These avoid new preferences,
controls, font dependencies, animation or changes to CAD layout.

Concrete limitations: oblique faces may omit labels; tiny windows may hide the
selected navigator without changing its kind; partially overlapped tiles by Action Box are wholly
suppressed; single primary clicks are supported, no widget dragging or keyboard
focus ring. Fira Code is monospaced and needs human visual acceptance at minimum
size. Local translucent graphics blend differently over theme backgrounds by
design. 4x target support/real GPU behavior, actual performance and pointer
usability have not been exercised. Missing historical shell artwork prevents
comparison to that file. No claim is made that the changing checkout compiles.

Later during inspection additional unrelated changes appeared in
`crates/newell-app/src/{app.rs,lib.rs,parameters.rs,selection.rs}`,
`crates/newell-model/src/parameters.rs`, `docs/ui_design/Interface_Elements.txt`,
and untracked `crates/newell-app/src/extrude_session.rs`. Later still,
`crates/newell-render/src/{lib.rs,renderer.rs,scene.rs}` acquired independent
multi-item scene upload/selection work (`SceneMeshItem`, `set_scene_items`,
`set_scene_item_selected`); the existing scene occlusion and camera APIs remain
available, and navigation must stay independent of those per-item pipelines.
The menu reference change is resolved by the human clarification above. App changes inspected
at that point concern geometry selection candidates/session state and add a
navigation cancellation when opening selection. They do not change the camera
or `left_pressed` contract on which this plan is based. Executor must preserve
those changes and incorporate the new cancellation site; do not restore the
initial source. This is a changing-input limitation, not automatic permission
to expand this task or demand a snapshot. Recheck before editing app.rs.

Documentation consistency verification in this Architect stage consists of
source/spec inspection, complete orientation/ownership reasoning, reviewing the
five documentation/reference outputs in this correction and checking their diff for whitespace/conflicts.
No implementation checks, tests, rendering experiments or application commands
were run. The follow-up starting state also includes independent edits in
`crates/newell-kernel/src/lib.rs`; these remain untouched. No production files
were edited by this Architect stage; all listed
runtime source edits above pre-existed or appeared independently.

Recommended next stage: the human may request Challenger review or explicitly
approve **revision 2** and request Executor implementation directly. Neither
has been started. No commit, index change, branch, worktree, agent dispatch or
external publication is authorized or performed by this stage.
