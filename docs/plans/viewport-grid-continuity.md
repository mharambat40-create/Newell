# Viewport grid continuity

Revision 2 · Proposed · 2026-11-08

Architect / Codex. Starting checkout is `main` at `310f382` with an empty
index. The working tree already contains the revision 1 renderer and
specification changes, including the later symmetric fade. This revision
supersedes the earlier subdivision design and uses that working tree as
the implementation baseline. The reference to Fusion 360 describes the
requested visual hierarchy.

## Stable grid and zoom levels

Grid Overlay remains a world XY grid at Z=0. Stable pitches are `5^n`
model units, anchored at zero in both directions. At each stable level,
indices divisible by five use the reinforced width. The four intervening
lines use the fine width. Apply the same rule to negative indices. The
zero-coordinate lines remain neutral and use the reinforced width, with
the colored origin references rendered separately.

Let `w` be the existing focal-plane model units per physical pixel.
Use `target = max(120 * w, 1e-12)`, `level = log(target) / log(5)`,
`s = 5^floor(level)` and `progress = 1 - fract(level)`. The active pair
has fine pitch `s` and coarse pitch `5s`. As the user zooms in, progress
moves from 0 to 1 within the pair. At an exact power-of-five boundary,
the fully resolved fine lattice is identical to the coarse lattice of the
next pair. Zooming out reverses the same calculation without history or
hysteresis. Both axes and all exposed draw regions use one pair and one
progress value per frame.

The fine pitch spans approximately 24–120 physical pixels at the
reference plane during a pair's transition. The fivefold ratio requires
replacing the old 60–120 pixel band in the specification. Local density
filtering still suppresses lines that project too closely in oblique or
distant regions.

## Line roles during the transition

Classify each axis independently. In fine-lattice indices, a coarse
line has index divisible by five. A coarse major line has index
divisible by twenty-five.

| Fine-lattice index | Opacity before view and density attenuation | Nominal width |
| --- | --- | --- |
| Divisible by 25 | Full | Reinforced throughout |
| Divisible by 5, other than multiples of 25 | Full | Fine below progress 0.5, reinforced at and above 0.5 |
| All other indices | `smoothstep(0.15, 0.85, progress)` | Fine throughout |

The four new lines appear together at fixed world positions between
each pair of coarse lines. The shared lines retain their positions and
base center opacity throughout. At the midpoint, the shared lines that
change role switch directly between the two width classes. This is a
deliberate discrete change at the line shoulders. Interpolating width
would create an intermediate nominal width, and fading a shared line
through zero would temporarily remove an existing reference.

Use nominal full widths of 1.0 and 1.25 physical pixels. Define nominal
width by the half-coverage contour of an isolated line after normalizing
for its opacity. Use antialiasing with the falloff centered on the
selected half-width. Distance may soften the falloff around that
contour and attenuate opacity. It must leave the two nominal contours
fixed. Crossings combine coverage by maximum, including the origin.

## Rendering scope

In `crates/newell-render/src/grid.wgsl`, replace the dyadic pitch
selection and the blend of two complete coverage fields. Evaluate
disjoint candidates per axis from the shared lattice and the inserted
lines. The inserted candidate must exclude multiples of five even
when the nearest fine index is a shared line. A shared line contributes
only once. Apply the selected width and opacity before taking maximum
coverage across candidates and axes. For shared lines, transition the
scalar density attenuation from the existing filter evaluated at
`5s` to the same filter evaluated at `s`, using the insertion opacity
as the weight. This keeps density attenuation continuous when changing
pitch pairs. Inserted lines use the density filter at `s`.

Retain world-hit reconstruction, the mirrored fade about the projected
origin, the orthographic forward-edge fade and the perspective horizon
guard. Retain the existing fallback when the origin is behind the
camera or outside the viewport's vertical span. Actual model depth
and UI masks apply to the real fragment. Mirroring the fade does not
mirror these masks. Maintain finite coverage at extreme supported
scales and parallel-ray rejection. Use signed-safe divisibility for
negative coordinates and test precision near the supported zoom limits.

After approval, update `ARCHITECTURE.md` §4.5 and
`docs/ui_design/README_UI.md` §28.3, Grid Overlay paragraph. Replace the
powers-of-two sequence, reference pixel band and every-other-line
transition wording. Specify the fivefold transition, the two nominal
widths and neutral origin lines. Preserve the existing fade, depth,
visibility and compositing rules in those sections.

Implementation files are `crates/newell-render/src/grid.wgsl` and
`crates/newell-render/src/grid.rs` for GPU readback tests, plus the two
approved specification sections. Keep the Controller-owned
`GridSnapshot` and renderer ownership as they are. The change requires
no new uniforms, draws, dependencies, MSRV or license changes. Grid
Overlay remains independent of sketch snapping and model geometry.

## Verification and acceptance

1. Record the shared Executor baseline before editing. Add focused
   GPU assertions that fail on the dyadic implementation. Reuse the
   readback harness and independently project expected world positions
   in perspective and orthographic fixtures.
2. **Stable levels and placement.** In both axis directions, test at
   least three consecutive stabilized pitches with positive and
   negative indices. Verify one reinforced line followed by four
   fine lines. Use sufficiently large, well-sampled fixtures to cover
   at least two complete groups in each direction. Check the origin
   crossing against a single reinforced line. Pan away from the
   viewport center and verify anchoring to the same world positions.
3. **Transitions in both directions.** Sweep at least two consecutive
   boundaries. Sample just before and after insertion begins, the role
   switch, insertion completion and the pitch-pair boundary. At each
   resolved interval verify four equal-spaced insertions, with no peak
   beyond one physical pixel from a projected lattice line and no
   duplicated shared peak. Verify the four insertion opacities track
   the same monotonic curve after accounting for local density and
   view attenuation. Shared centers must match their expected single
   contribution through the role switch. At fixed matrices, perturb
   the published scale on both sides of a pitch-pair boundary and
   require the image to agree within two readback code values. A
   return to identical matrices and scale must reproduce identical pixels.
4. **Width and crossings.** Sample isolated perpendicular profiles
   in linear light at several subpixel phases, then normalize by
   each line's peak before comparing against the reference coverage
   curves. Verify the fine and reinforced profiles at stable levels,
   on each side of the role switch, and during partial insertion.
   Fit the half-coverage contour within 0.1 physical pixels of the
   expected class. Test the full-opacity shared lines and the
   origin crossing against the corresponding single-line coverage.
   Include distant samples to confirm softened shoulders with the
   same nominal half-coverage contours wherever the signal is
   strong enough to measure.
5. Keep the existing GPU regressions for mirrored fade with an
   off-center origin, forward edge, horizon, model and UI occlusion,
   clear-depth independence, viewport offsets and scissor seams,
   themes and extreme zoom. Retarget dyadic fixtures to fivefold
   pitches while preserving their acceptance conditions. Run the
   explicit renderer GPU suite and shared workspace verification
   with snapshot updates disabled. Separate inherited failures from
   new regressions.
6. Inspect the baseline and rebuilt app in both projections. Zoom
   slowly through at least two transitions each way and pause at
   stable levels. Check four insertions per interval, steady shared
   line centers, a subtle width switch and no double-brightened
   crossings. Pan the origin off-center and orbit to an oblique and
   a grazing view. Verify the vertical fade remains centered on the
   visible origin. Repeat with a solid crossing the grid and an
   opaque menu. Record the viewport size, repeatable camera setup
   and representative captures. Report visual results separately
   from GPU readback.
