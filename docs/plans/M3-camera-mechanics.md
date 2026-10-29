# M3 Camera Mechanics Development Plan

**Suggested destination:** `docs/plans/M3-camera-mechanics.md`
**Role:** Architect — specification only; no production changes.

## 1. Scope and repository baseline

Implement Onshape-style viewport navigation while preserving BrepKIT geometry generation, WGPU rendering, the ASCII interface, and event-driven redraw.

The requested interaction model is:

- Primary-button drag: screen-space yaw/pitch orbit with a level horizon.
- Secondary-button drag: focal-distance proportional pan; middle-button drag is a pan fallback.
- Primary and secondary presses capture navigation only after the click/drag threshold.
- `LineDelta` wheel input, unphased `PixelDelta` wheel input, and native pinch where available: cursor-targeted zoom.
- A button-free phased `PixelDelta` stream that starts over exposed viewport pixels: two-axis screen-plane pan at contextual focal depth.
- Orbit pivot: geometric hit under the pointer; otherwise the visible model bounding-box centre.
- Navigation state and input handling: `newell-app`.
- Camera matrix consumption and uniform upload: `newell-render`.

The inspected working tree already contains navigation implementation and marks M3.4–M3.9 complete. Its current architecture specifies a fixed-target quaternion trackball. This plan therefore describes a **revision of that implementation**, rather than assuming those files contain only M3.1–M3.3.

Before implementation, reconcile `ARCHITECTURE.md` §4.4 and the backlog with this requested model. Specifically, replace the fixed-target trackball requirement with the zero-roll and dynamic-pivot contract below. Preserve existing staged and unstaged work.

For this plan, organize the requested work as:

| Task | Deliverable |
|---|---|
| M3.4 | Controller-owned camera state, renderer snapshot API, screen-space orbit |
| M3.5 | Dynamic pivot selection and gesture lifecycle |
| M3.6 | Pan, cursor-targeted zoom, limits, and regression tests |

## 2. Define “strict zero roll”

Camera-relative yaw and pitch alone do **not** guarantee a level horizon: successive rotations about changing local axes can accumulate roll.

Use this explicit definition:

> Camera up must equal the normalized projection of model-world up onto the plane perpendicular to the viewing direction.

Newell uses Z-up:

\[
W=(0,0,1)
\]

Given unit viewing direction \(F\):

\[
R=\operatorname{normalize}(F\times W)
\]

\[
U=R\times F
\]

This produces an orthonormal, level camera basis.

There is an unavoidable singularity when \(F\) is parallel to \(W\). Consequently, strict zero roll requires a pole policy. For M3:

- Limit elevation to \(\pm(\pi/2-\varepsilon)\).
- Use an initial pole margin of \(0.5^\circ\), expressed as one named constant.
- Retain the previous horizontal heading when a candidate direction becomes numerically vertical.
- Allow immediate motion away from the limit.
- Do not flip heading or silently introduce roll.

“Free orbit” here means unrestricted horizontal orbit and camera-relative pointer control within that elevation range. Continuous passage through the poles and a globally level horizon cannot both be guaranteed.

Existing principal-axis commands may produce an exact top or bottom view. Preserve those commands, retain a deterministic heading there, and handle departure from the singular view explicitly rather than normalizing a zero-length cross product.

## 3. MVC ownership and data flow

### Controller: `newell-app`

Own all mutable navigation state:

- Camera pose and projection parameters.
- Active gesture and initiating button.
- Pointer positions and normalized input deltas.
- Pivot selection.
- CPU ray construction and geometric queries.
- Pan, orbit, zoom, fitting, and bounds enforcement.
- Camera revision and redraw decisions.

Maintain a CPU navigation cache for the currently displayed tessellation. Preview replacement and committed model replacement must update this cache together with the rendered mesh.

### View: `newell-render`

Consume an immutable camera snapshot containing:

- View matrix.
- Projection matrix.
- View-projection matrix.
- Camera revision.
- Any camera-dependent values actually required by the scene shader.

The renderer uploads this snapshot to its camera uniform buffer. It must not interpret pointer events, choose pivots, retain gesture state, or modify camera pose.

Replace the existing renderer navigation entry points with a setter such as `set_scene_camera(snapshot)`. Remove their old implementations after all callers migrate.

Keep viewport rectangles and UI occlusion available to rendering for viewport/scissor configuration. Their authoritative layout still comes from the existing controller/UI path.

### Model boundary

Keep all BrepKIT operations inside `newell-kernel`. Camera navigation operates on Newell-owned mesh data and entity identifiers; it must not import upstream BrepKIT types.

No new crate or mathematical dependency is required for this milestone.

## 4. State variables

Use `f64` for controller camera calculations and convert matrices to finite `f32` values at the renderer boundary.

| State | Purpose |
|---|---|
| `eye: Vec3` | World-space camera position |
| `orientation: UnitQuaternion` | Camera-local to world rotation |
| `focal_distance: f64` | Positive distance defining the camera-facing navigation plane |
| `world_up: Vec3` | Fixed model Z-up reference |
| `last_heading: Vec3` | Stable horizontal heading near singular views |
| `projection_mode` | Preserve existing perspective/orthographic behavior |
| `vertical_fov` | Perspective vertical field of view |
| `orthographic_height` | Independent orthographic view extent |
| `lens_shift: Vec2` | Existing off-axis projection behavior |
| `near`, `far` | Valid clipping range derived from scene depth |
| `viewport` | Physical-pixel projection rectangle |
| `mesh_revision` | Reject stale query results |
| `camera_revision` | Identify changed renderer snapshots |
| `gesture: Option<Gesture>` | Captured navigation session |

Derive the camera basis from the quaternion:

\[
R=q(1,0,0),\quad U=q(0,1,0),\quad F=q(0,0,-1)
\]

A conventional look target, when needed, is derived:

\[
T=E+dF
\]

Do not maintain independently mutable eye, quaternion, up vector, and target. That creates redundant state that can disagree.

### Gesture snapshot

Capture:

- Gesture kind and initiating button.
- Start pointer in physical subpixels.
- Initial eye, orientation, focal distance, and projection.
- Frozen viewport and mesh revision.
- Frozen world-space pivot.
- Initial camera basis and heading.
- Pan start-plane intersection, when applicable.

Evaluate ordinary drag motion from this snapshot and total pointer displacement. This avoids event-frequency dependent accumulation.

Release only the button that owns the gesture. Releasing another button must not cancel it.

## 5. Screen-space orbit mathematics

Use a right-handed camera looking along local \(-Z\), column vectors, and:

\[
VP=P\,V
\]

Pointer coordinates have positive X rightward and positive Y downward.

For displacement from the press position:

\[
\Delta x=x-x_0,\qquad \Delta y=y-y_0
\]

Convert movement to angles using logical pixels so display scaling does not change physical-device sensitivity:

\[
\alpha=-k_x\frac{\Delta x}{s}
\]

\[
\beta=-k_y\frac{\Delta y}{s}
\]

Here \(s\) is the window scale factor. Use named sensitivity constants; start with \(k_x=k_y=0.005\) radians per logical pixel and validate manually.

The negative signs make dragging right move the visible front of the object right, and dragging upward move it upward.

### Candidate orientation

From the gesture-start basis:

1. Construct yaw quaternion \(q_y\) around initial screen up \(U_0\).
2. Rotate initial screen right by yaw:

\[
R_1=q_yR_0
\]

3. Construct pitch quaternion \(q_p\) around \(R_1\).
4. Form the candidate orientation:

\[
q_c=\operatorname{normalize}(q_pq_yq_0)
\]

### Remove roll and enforce the pole policy

Extract candidate forward \(F_c\). Separate its horizontal heading and elevation:

\[
h=F_c-(F_c\cdot W)W
\]

\[
\theta=\arcsin(\operatorname{clamp}(F_c\cdot W,-1,1))
\]

Normalize \(h\) when valid; otherwise reuse the captured heading. Clamp elevation and rebuild:

\[
F=\cos(\theta_{\text{clamped}})\hat h+
\sin(\theta_{\text{clamped}})W
\]

Construct \(R,U\) using the zero-roll equations, then build normalized \(q_{\text{new}}\) from basis columns:

\[
[R,\ U,\ -F]
\]

Choose quaternion sign consistently with the previous quaternion to avoid apparent sign discontinuities in diagnostics.

### Rotate around the contextual pivot

Do not implement pivot selection by making the camera look directly at the hit point.

Compute:

\[
Q=q_{\text{new}}q_0^{-1}
\]

\[
E_{\text{new}}=P+Q(E_0-P)
\]

Apply this **same final rotation**, including roll correction, to the eye offset and camera orientation.

This preserves the pivot’s camera-space coordinates:

\[
q_{\text{new}}^{-1}(P-E_{\text{new}})
=
q_0^{-1}(P-E_0)
\]

Therefore, the pivot remains at its original projected pixel. Selecting an off-centre pivot does not recenter it.

At zero pointer displacement, return the original snapshot exactly.

## 6. Dynamic pivot strategy

Choose the pivot once at orbit start. Freeze it until that gesture ends.

### Geometric hit

After UI routing confirms visible scene pixels:

1. Construct a ray using the current camera and projection.
2. Intersect the current displayed tessellation.
3. Select the nearest positive intersection.
4. Recover the world-space hit:

\[
P=O+tD
\]

Return Newell-owned hit information: point, distance, face identifier, and mesh revision.

Use tessellated geometry for M3, matching the displayed BrepKIT-generated mesh. Exact analytic B-Rep intersection is outside this milestone.

Move the existing CPU intersection logic into the controller query module. Do not request a GPU depth readback.

### Miss: visible bounding-box centre

Define “visible bounds” precisely:

> Bounds of displayed geometry intersecting the view frustum and exposed viewport regions, excluding hidden model entities. Exact removal of surfaces hidden behind other geometry is not required.

For the current single-solid application:

- Cache mesh bounds and triangle bounds when the mesh changes.
- Recompute exposed-view bounds when camera, viewport, visibility, or occlusion revisions change.
- Clip relevant triangle portions against the view frustum and exposed viewport rectangles.
- Accumulate a world-space AABB from the surviving portions.
- Choose:

\[
P=\frac{B_{\min}+B_{\max}}{2}
\]

Projection-space clipping must retain corresponding world positions. Test near-plane intersections and geometry crossing viewport edges.

If no exposed geometry remains, retain the current navigation-plane centre. If the scene has no valid geometry, use the existing default camera reference.

Selecting a pivot must change no rendered pose or projection.

### Query cost

UI hit testing remains O(1). Geometric intersection and visible-bounds calculation have separate complexity.

For M3, reuse CPU triangle queries and cache bounds; do not claim triangle picking is O(1). If measured query time warrants acceleration, add a controller-owned spatial index as a separate, justified optimization.

## 7. Focal-distance proportional panning

At pan start, choose a contextual depth using the geometric hit or visible-bounds centre:

\[
d=\max(d_{\min},(P-E)\cdot F)
\]

Freeze that depth during the gesture.

For perspective projection, world units per physical pixel are:

\[
u=\frac{2d\tan(\phi_y/2)}{H}
\]

where \(H\) is viewport height.

For drag displacement:

\[
\Delta E=-u\,\Delta x\,R+u\,\Delta y\,U
\]

Update eye and any retained world-space navigation reference by the same translation. Preserve orientation.

This makes the scene follow the pointer:

- Dragging to the right moves the camera left.
- Downward drag moves the camera up.
- Twice the focal distance produces twice the world translation.
- Twice the viewport height produces half the translation for identical physical-pixel movement.

For an off-axis projection, use ray intersections with the frozen camera-facing plane as the authoritative formulation. The difference between start and current intersections gives the same translation while respecting lens shift.

For orthographic projection:

\[
u=\frac{\text{orthographic height}}{H}
\]

Captured pan continues across viewport boundaries while the pointer remains in the application window.

### Button-free phased trackpad pan

On macOS, winit 0.30.13 maps precise `scrollWheel:` events to
`MouseScrollDelta::PixelDelta` and maps the native scroll lifecycle to
`TouchPhase::{Started, Moved, Ended}`. The event does not report finger count.
Route a `PixelDelta` stream to camera pan only when its `Started` phase begins
over exposed viewport pixels and no UI target consumes that scroll. Freeze the
pointer anchor and contextual focal depth at that start; add the physical X/Y
deltas to the same pointer-plane pan calculation used by mouse drags. Positive
deltas follow winit's documented content direction (right/down). Winit already
converts macOS precise deltas to physical pixels, so do not apply another scale
factor. Preserve fractional values through camera math.

The UI keeps first refusal on every scroll event. A stream started over a menu,
modal, tree, selection list or other UI remains UI-owned through its end even if
the pointer later reaches the viewport. Focus loss, pointer exit, layout changes,
projection changes and scene changes cancel the pan and reject the rest of that
stream. Scroll pan has no application-generated inertia or render loop.

Winit does not identify the number of fingers or distinguish a phased
high-resolution mouse wheel from trackpad scrolling. Therefore the routing rule
is exact and intentionally limited: a phased `PixelDelta` stream with an
observed `Started` phase pans; `LineDelta` and `PixelDelta` without that start
retain wheel zoom. A trackpad event delivered without a start phase cannot pan
under this rule, while a phased high-resolution mouse wheel can.

## 8. Cursor-targeted zoom

### Input conversion

Keep camera normalization separate from existing UI-panel scrolling.

- Wheel line deltas retain fractional values.
- Unphased smooth physical-pixel wheel deltas are divided by display scale.
- Initially map 100 logical pixels to one wheel step.
- Ignore horizontal-only zoom input.
- Reject non-finite values.
- Respect the OS scroll direction.
- Support native pinch using its fractional magnification.
- Use the last pointer location when the platform supplies no pinch centroid.

Track phased gesture ownership so a gesture beginning over UI cannot begin affecting the scene midway through its stream.

### Anchor

At gesture start, raycast under the pointer:

- Hit: use the world-space surface point.
- Miss: intersect the ray with the camera-facing focal plane.

Freeze the anchor during a phased gesture to avoid jumping between surface depths. Independent wheel ticks may acquire a new anchor.

### Perspective update

Let the anchor be \(A\), its forward depth be \(d_A\), and normalized zoom input be \(z\):

\[
d'=\operatorname{clamp}(d_Ae^{-kz},d_{\min},d_{\max})
\]

\[
\lambda=\frac{d'}{d_A}
\]

\[
E'=A+\lambda(E-A)
\]

Keep orientation and projection unchanged. Because the vector to the anchor changes only by a positive scalar, its projected pixel remains unchanged.

Scale the retained focal distance by \(\lambda\), subject to valid navigation limits.

### Orthographic update

Scale orthographic height exponentially within bounds. Then unproject the cursor before and after scaling and translate the eye by their difference. Do not change eye distance merely to simulate orthographic zoom.

### Limits and recovery

Retain an absolute positive minimum, initially `0.001` model units.

For a mesh bounding sphere of radius \(r\):

\[
\phi_x=2\arctan(a\tan(\phi_y/2))
\]

\[
d_{\text{fit}}=
\frac{1.1r}{\sin(\min(\phi_x,\phi_y)/2)}
\]

Use a documented multiple, initially \(4d_{\text{fit}}\), as the maximum navigation distance. Recompute derived limits after mesh or projection changes without automatically changing camera pose.

Distance limits cannot guarantee visibility after arbitrary pan. Preserve explicit fit recovery, fitting into an unobstructed viewport region.

## 9. Input routing, resizing, and lifecycle

Only begin navigation when:

- The pointer lies inside the rendered viewport.
- No modal or menu owns input.
- The O(1) grid lookup resolves no UI control.
- The pointer is outside opaque Action Box and popup footprints.
- No window resize or competing controller gesture owns the pointer.

Once admitted, capture the initiating button and gesture snapshot.

End or cancel capture on:

- Initiating-button release.
- Focus loss or window exit.
- Modal/menu activation.
- Mesh replacement.
- Projection-mode change.
- Viewport or occlusion change.
- Explicit cancellation.

Cancellation retains the current camera pose.

Preserve the existing projection behavior that prevents Action Box toggles or viewport resize from recentering the scene. In the new ownership model, the controller computes projection changes; the renderer only consumes them.

Request redraw only when camera, geometry, or UI state changes. Automatic named-view
changes use the separately specified 250 ms event-driven camera transition; request
frames only until that transition completes. Add no continuous rendering loop or
animation for direct pointer navigation.

## 10. Exact file changes

All paths are relative to `/Users/martin/Desktop/Newell`.

| File | Planned change |
|---|---|
| `crates/newell-app/src/camera.rs` — new | GPU-independent camera state, quaternion/basis math, projection, unprojection, pan, zoom, limits |
| `crates/newell-app/src/navigation.rs` — new | Gesture snapshots, orbit/pan/zoom controller, capture ownership and cancellation |
| `crates/newell-app/src/picking.rs` — new | CPU ray intersections, geometric hit positions, mesh bounds and exposed-view bounds cache |
| `crates/newell-app/src/lib.rs` | Register camera, navigation, and picking modules |
| `crates/newell-app/src/app.rs` | Own navigation state; route winit events; synchronize mesh/query revisions; submit camera snapshots |
| `crates/newell-app/src/input.rs` | Keep wheel/pinch normalization and phased-gesture admission testable |
| `crates/newell-app/src/controller.rs` | Expose authoritative visible-scene eligibility and UI ownership |
| `crates/newell-render/src/camera.rs` — new | Immutable renderer-facing camera snapshot type; no navigation behavior |
| `crates/newell-render/src/lib.rs` | Export the camera snapshot API |
| `crates/newell-render/src/renderer.rs` | Add camera snapshot setter; migrate callers from renderer-owned navigation APIs |
| `crates/newell-render/src/scene.rs` | Consume supplied matrices; remove mutable navigation and CPU camera-input logic |
| `crates/newell-app/tests/it/camera.rs` — new | Camera mathematical integration tests |
| `crates/newell-app/tests/it/navigation.rs` — new | Gesture and pivot integration tests |
| `crates/newell-app/tests/it/main.rs` | Register new test modules |
| `crates/newell-app/tests/it/input.rs` | Extend delta and gesture-phase tests |
| `crates/newell-app/tests/it/controller.rs` | Extend overlay, resize, and capture-precedence tests |
| `ARCHITECTURE.md` | Reconcile §4.4 with this interaction model and strict MVC ownership |
| `docs/backlog.md` | Reconcile milestone numbering and distinguish automated completion from visual acceptance |
| `docs/plans/M3-camera-navigation.md` | Mark superseded behavior and point to the new plan |
| `docs/plans/M3-camera-mechanics.md` | Store this plan |

Preserve the current shader uniform layout where possible; no shader change is expected. No changes are planned to kernel operations, parametric features, UI layout, or dependencies.

## 11. GPU-independent tests

Use analytic fixtures and the existing property-test dependency. Compare rotations through their action on vectors, accounting for equivalent quaternion signs.

### Quaternion and orbit

1. Rightward drag moves a front-facing reference point right on screen.
2. Upward drag moves it upward.
3. Identical displacement has identical response at viewport centre and edges.
4. Zero displacement preserves pose exactly.
5. Quaternion norm remains one after repeated gestures.
6. Basis vectors remain orthogonal and unit length.
7. Camera up matches projected world up within tolerance.
8. Mixed yaw/pitch sequences introduce no horizon roll.
9. Pole limits remain finite and preserve heading.
10. Motion away from a pole limit responds immediately.
11. The pivot’s camera-space coordinates and projected pixel remain fixed.
12. Distance from eye to pivot remains constant during orbit.
13. One drag evaluated through different intermediate event partitions produces the same final pose.

### Pivot and geometric queries

14. Choose the nearest positive triangle hit and return its world position.
15. Reject intersections behind the eye and degenerate triangles.
16. Ray hit positions agree with barycentric reconstruction.
17. A miss selects the exposed-view AABB centre.
18. Hidden entities, clipped geometry, and fully occluded regions do not contribute.
19. Bounds remain valid for near-plane-crossing triangles.
20. Empty scenes use a finite deterministic fallback.
21. Selecting a new pivot changes no projected reference points.
22. Mesh revision changes invalidate cached queries and capture.

### Matrices and rays

23. Camera eye transforms to the view-space origin.
24. Points along forward have negative view-space Z.
25. Project/unproject round trips work with nonzero viewport origin and lens shift.
26. A ray reconstructed from a projected point passes through that point.
27. Perspective and orthographic rays follow their distinct origin/direction rules.
28. WGPU depth maps near/far planes into `[0,1]`.
29. Resizing preserves intended physical-pixel projection positions.
30. Invalid dimensions or non-finite inputs produce no camera mutation.

### Pan and zoom

31. Pan translates the projected anchor by the pointer displacement.
32. Pan preserves orientation.
33. Pan scale follows focal distance and viewport dimensions.
34. A phased, button-free `PixelDelta` stream starting on exposed viewport pixels pans both screen axes with fractional deltas and the frozen contextual depth.
35. A stream starting on UI, an occlusion, or another ineligible region cannot later pan after entering the viewport.
36. `LineDelta`, unphased `PixelDelta`, and native pinch retain their existing zoom behavior.
37. Perspective zoom preserves the surface anchor’s projected pixel.
38. Orthographic zoom preserves its anchor without changing orientation.
39. Minimum and maximum limits apply without crossing the anchor.
40. Bounds scale with mesh size and account for narrow aspect ratios.
41. Partitioned fractional zoom events produce equivalent results for a frozen anchor.
42. DPI-normalized deltas produce equivalent camera changes.
43. Fit recovery contains the complete model within the selected unobstructed rectangle.

### Input ownership

44. Gestures cannot start outside the scene or under UI.
45. A rejected phased gesture cannot enter the scene midway through its stream.
46. Releasing an unrelated button preserves active capture.
47. Focus loss, modal activation, and resize end capture without changing pose.
48. Action Box opening and closing changes occlusion without reframing.

## 12. Implementation order and acceptance

1. Reconcile architecture and milestone documentation.
2. Extract GPU-independent camera math and introduce the immutable renderer snapshot.
3. Move mutable navigation state into `newell-app`.
4. Implement and test zero-roll orbit.
5. Implement geometric queries, visible bounds, and frozen dynamic pivots.
6. Implement pan, cursor-targeted zoom, and recovery.
7. Complete event routing and lifecycle tests.
8. Run formatting, workspace tests, strict Clippy, and available GPU tests.
9. Perform visual acceptance with mouse and trackpad.

The visual review must specifically assess direction, horizon stability, pole behavior, off-centre pivots, near-edge starts, trackpad sensitivity, panel occlusion, viewport resizing, and recovery after extreme pan/zoom.

Record inherited test failures separately. Do not rewrite unrelated expectations, modify unrelated working-tree changes, or create a commit as part of implementation unless explicitly requested.


## 13. Executor implementation notes — 2026-10-19

The original design above is retained as the approved plan. The following
mathematical corrections and implementation details govern the implementation:

### Level yaw before bounded pitch

Constructing an unrestricted pitch quaternion and then extracting elevation with
`asin` folds angles past a pole back into the elevation range. It can reverse the
horizontal heading even though the final elevation is clamped. The implementation
instead applies camera-relative yaw about the captured screen-up axis, extracts
its heading, levels that result, and adds signed pitch to its elevation **before**
clamping to ±(π/2 − 0.5°). It rebuilds the unit quaternion from the level basis.
Pitch therefore acts about the leveled screen-right axis, rather than the plan's
potentially rolled intermediate right axis. This preserves direct pointer signs
and prevents large pitch inputs from passing through a pole.

The eye offset and orientation both use the same final quaternion difference,
including horizon correction. The plan's camera-space pivot-invariance identity
therefore still holds. Zero displacement returns the exact captured pose.
Exact top/bottom commands retain a stored heading; their first nonzero orbit
movement departs into the 0.5° orbit band without deriving heading from a zero vector.

### Pole overscroll

Once pitch saturates, discard unconsumed vertical pointer motion by adjusting the
captured pointer origin. Reversal responds immediately in the same gesture.
Unsaturated orbit is evaluated from the original camera snapshot and is independent
of intermediate event frequency; saturation deliberately introduces anti-windup
state instead of retaining an overscroll dead zone.

### Projection and zoom consistency

Orthographic projection uses lens shift in the affine translation column, unlike
perspective projection's depth column. Parallel orthographic rays originate at
distinct screen-plane points. Resizing preserves image positions at every depth
in both modes. An unchanged viewport does not round-trip its FOV through trig
functions. Returning from orthographic zoom to perspective preserves the focal
plane target and scale by moving the eye along forward.

Perspective zoom intersects the valid anchor-depth and focal-distance scaling
intervals, then moves along the anchor ray; it does not independently clamp the
focal distance after moving the eye. If mesh replacement makes an existing pose
exceed new limits, zoom moves toward the valid interval without snapping the
camera in the opposite direction. Fit explicitly restores full visibility.
Orthographic zoom scales height and translates within the anchor plane.

### Queries and ownership

The Controller caches triangle vertices, per-triangle bounds, face ordinals, mesh
bounds and an exposed-bounds result keyed by camera/occlusion state. Mesh replacement
invalidates those caches. Visible bounds clip triangle polygons in homogeneous
coordinates against the WGPU frustum and each exposed scene rectangle, carrying
world positions through clipping. Hidden entities are excluded by the displayed
mesh; depth-buffer occlusion between model surfaces is outside this definition.

Mouse capture releases only on its initiating button. Phased scroll/pinch streams
freeze the surface/focal-plane anchor even when Started carries zero delta. UI,
layout, mesh, projection, focus and exit cancellation preserve pose and reject the
remaining active trackpad stream. Platforms delivering only Moved can begin a new
stream while idle. A missing pointer at Started counts as rejected scene input.
The existing panel-scroll conversion remains independent of camera sensitivity.

The renderer's existing 96-byte scene uniform and shaders remain unchanged. Its
camera snapshot validates finite f32 matrices at the API boundary and retains no
pose, geometry query or gesture behavior. CPU camera regressions previously housed
in the renderer now live in `newell-app`.

Validation results and physical-device limitations are recorded in
[2026-10-19 validation](../validation/M3-mechanics-2026-10-19.md).
