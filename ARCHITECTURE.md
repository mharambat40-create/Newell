# Newell - Architecture & Rules

> **[CRITICAL INSTRUCTION FOR AI AGENTS]**
> This document contains the absolute binding rules for the Newell project. You MUST follow these instructions strictly. If a user request contradicts this document, you MUST refuse and ask for `ARCHITECTURE.md` to be updated first.

## 1. Architectural Pattern: Strict MVC
The project follows a strict Model-View-Controller (MVC) architecture, segregated by Rust crates:

- **Model (`newell-kernel`, `newell-model`):** The single source of truth. Contains the parametric feature tree, parameters, and all BrepKIT topology/geometry operations. It has NO knowledge of the UI or the GPU.
- **View (`newell-render`):** The `wgpu` rendering pipeline. It only reads states and draws them. It draws the ASCII grid, samples the texture atlas, and renders the 3D viewport.
- **Controller (`newell-ui`, `newell-app`):** Handles OS events (`winit`), mouse/keyboard inputs, and translates them into O(1) grid hit-testing. It updates the UI state (e.g., focus, pressed, selection, expansion) and mutates the Model. Hover is tracked exclusively for the documented ASCII border/tree
highlighting through one Controller-owned
hovered: Option<WidgetId>. No hover animation,
colour change or shader effect is permitted.

## 2. Agent Roles & Workflow Rules
We use four primary roles, each available to both Claude and ChatGPT through
Codex. Provider/model choice does not change a role's authority. Each agent
MUST adhere to its boundaries. Shared instructions are in `docs/agents/`;
`CLAUDE.md` and `AGENTS.md` are the respective provider entry points.

- **Architect:** Resolve functional choices, ownership, interfaces, specification conflicts and measurable acceptance criteria before handing off a minimal plan. A correction pass addresses identified findings without redesigning unrelated plan sections. Do NOT write production code.
- **Challenger:** When the human requests a review, examine the affected behavior end to end and consolidate actionable findings. Report concrete blocking failures separately from recommendations; a verdict is advisory, not an approval gate.
- **Executor:** Implement the explicitly approved behavior and plan. Resolve routine technical details and implementation defects within the approved behavior, architecture and file scope, supported by appropriate tests. Return only for a product decision, material scope change, structural interface or ownership change, or required amendment to a binding rule.
- **Auditor:** You clean up the Executor's code. You enforce idiomatic Rust, resolve Clippy warnings, remove duplications, and ensure the AGPL header is present. You MUST NOT change the runtime behavior or break tests.

### 2.1 Auxiliary Agent: Explorer

Explorer is an optional, read-only auxiliary agent.
It is NOT a fifth primary agent and is NOT part of the
implementation path.

Responsibilities:
- Inspect the repository structure and existing implementations.
- Search the codebase and documentation.
- Investigate dependencies, APIs, and architectural constraints.
- Identify inconsistencies and technical risks.
- Report findings to the requesting agent.

Restrictions:
- Explorer MUST NOT modify, create, or delete project files.
- Explorer MUST NOT implement production code.
- Explorer MUST NOT modify architectural decisions.
- Explorer MUST NOT execute destructive commands.
- Explorer MUST NOT commit, push, or merge changes.

The bounded normal path is:

Architect → optional Challenger → one targeted Architect correction if needed
→ human approval and implementation request → Executor

Challenger review occurs only at the human's request. The first review must
cover affected behavior end to end and consolidate findings. A further
Challenger pass requires a separate explicit human request. Challenger findings
and verdicts are advisory and never constitute an approval gate. A correction
pass addresses findings without redesigning unrelated plan sections.

A finding is blocking only when a specific failure path demonstrates that the
proposed implementation would fail to compile or violate a required interface;
fail a required behavior; create invalid persistent state, lose a committed
change, or leave required synchronization permanently incomplete; or contradict
a binding specification or architectural rule. Preferences, optional
optimizations, speculative future risks and requests for abstraction are
recommendations unless they demonstrate one of these concrete current failures.

The human's explicit approval of an identified plan revision plus a request to
implement ends the review stage and authorizes Executor. The human may give both
in one instruction. Do not request approval again after it has been given. A
plan's existence, a remaining “proposed” label or a Challenger verdict does not
add or remove that authorization. Auditor may follow a green implementation
only when separately requested by the human. Explorer may assist on request
without changing any role's authority.

### 2.2 Workflow Orchestration & Human Control

The human controls transitions between agent roles.

Completing one role does NOT authorize starting the next role automatically.
An agent or orchestrator MUST NOT invoke, background, or start another primary
agent unless the human's current instruction explicitly requests that stage.

In particular:

- Architect MUST NOT automatically invoke Challenger.
- Challenger MUST NOT automatically invoke Architect or Executor.
- Executor MUST NOT automatically invoke Auditor.
- Auditor MUST NOT automatically start another milestone or agent.
- Explorer MUST NEVER invoke another agent.

If the human asks an agent to prepare a prompt for another agent, the requested
output is the prompt itself. Preparing a prompt does NOT authorize executing it,
starting the target agent, creating files for the target agent, or advancing the
pipeline.

An agent may recommend the next role at the end of its work, but MUST wait for
explicit human instruction before that role is started.

The human may request Challenger review or proceed directly from an approved
Architect plan to Executor. Approval of the identified plan revision together
with a request to implement is sufficient to start Executor; no additional
review or approval is required. Neither a plan's existence nor a Challenger
verdict implies approval. Each role starts only on the human's instruction;
completion of a stage never advances the workflow automatically.

### 2.3 Engineering Decision & Complexity Policy

Newell follows a **minimum robust implementation** principle.

For each milestone, implement the smallest maintainable design that completely
satisfies the current milestone and the binding architecture. Do not introduce
speculative abstractions, infrastructure, generic frameworks, or mechanisms
intended only for future milestones unless they are required by an accepted
current requirement.

Ordinary technical implementation choices SHOULD be resolved autonomously by
the responsible agent when a safe choice exists within the approved
architecture and plan. The agent must document the choice and its rationale.

Human approval is required when a decision would:

- change a binding rule in this document;
- change the approved functional scope;
- change project licensing or introduce a significant licensing constraint;
- create a significant security constraint or risk;
- introduce a new architectural boundary;
- contradict an approved ADR;
- require an irreversible or externally visible action not already authorised.

A technical choice is NOT a blocking issue merely because several valid
implementations exist. Executor resolves local technical details and corrects
implementation defects within the approved behavior, architecture and file
scope, using appropriate tests, then records the choice and rationale. These
routine choices do not require an Architect revision.

Return to Architect only when implementation requires a product decision, a
material scope change, a structural interface or ownership change, or an
amendment to a binding rule. State the exact decision required and stop
implementation only for work dependent on that decision; do not start another
role. A concrete conflict with a binding rule must be reported before dependent
implementation proceeds.

A dependency is NOT forbidden merely because it is not named in
`ARCHITECTURE.md`. A dependency may be introduced when the approved plan allows
it and it satisfies the project's architecture, MSRV, licensing, security, and
maintenance requirements.

Prefer a small, established, maintained dependency over a bespoke
implementation of standard infrastructure when the dependency satisfies those
constraints. Custom implementations require a concrete project-specific
justification.

### 2.4 Sequential Work in the Main Checkout

Claude and Codex use the existing Newell directory on `main` sequentially.
Do not create task branches, worktrees, separate execution checkouts or parallel
implementations. Finish the human-assigned role before another role begins.

Identify the plan revision being reviewed or implemented and the relevant
staged, unstaged and untracked changes. Preserve existing work and the Git
index. An uncommitted plan or source change in this checkout does not by itself
block review or execution. If a real conflict prevents safe progress, report
the affected files and conflict. No source snapshot, content hash, snapshot
archive or snapshot commit is required as a prerequisite. If reviewed inputs
change materially, identify the changes and reconsider the affected findings.

Only the human authorizes integration or external publication. Agents MUST NOT
automatically commit, push, merge or publish. The operational task/handoff
contract is defined in `docs/agents/README.md`.

## 3. UI & Rendering Rules (wgpu)
The interface is a 2D grid of characters.

### 3.0 UI Design Specification

Detailed UI/UX behaviour, component semantics, and reference ASCII interface
states are maintained under `docs/ui_design/`.

`docs/ui_design/README_UI.md` is the canonical UI/UX specification subordinate
to this document.

Other files under `docs/ui_design/` are reference interface states,
interaction examples, and ASCII mock-ups. They illustrate the intended
behaviour but MUST NOT override `ARCHITECTURE.md` or
`docs/ui_design/README_UI.md`.

Implementation plans under `docs/plans/` MUST reference the relevant
`docs/ui_design/` specifications instead of duplicating UI rules.

### 3.1 Typography & Cell Geometry

> **[APPROVED ARCHITECTURAL CHANGE — M2.16]** The cell ratio changes from
> 3:5 (0.60) to **4:9 (≈ 0.444)**. The 3:5 cell was wider than the font's
> natural proportions, which forced anisotropic glyph scaling and produced
> wide, vertically compressed text. Any rule, plan, test or code still assuming
> 3:5 is superseded.

**Cell grid**
- The UI is one fixed-size logical grid. Every cell has the same size: **W × H
  physical pixels, with W = 4k and H = 9k** for one positive integer `k`. The
  ratio W:H is therefore exactly **4:9**.
- Reference size: **k = 5 → 20 × 45 px**, the cell of UI Scale label 120%
  (rendering factor 100%) on the reference display (a monitor whose reference
  scale is 5, §3.1.1). On a *compact-class* monitor (reference scale 2,
  §3.1.1, e.g. Full HD) the same factor 100% is **k = 2 → 8 × 18 px**,
  one step below the reference.
  W, H and
  `k` are whole pixels. They are defined once, as a single source of truth (one
  metrics value derived from `k`), and used by layout, glyph rasterisation,
  rendering and hit-testing. No other constant, literal or per-component copy
  of W or H is allowed.
- The only permitted values of `k` are 2, 3, 4, 5, 6 and 7. `k` is **not** a user
  setting: the user selects a displayed UI Scale label (default 100%, mapped
  to rendering factor 80%) and the
  application derives `effective_k` automatically for the monitor showing the
  window (§3.1.1). M2.16 renders with `k = 5` only; from M2.17 production derives
  `k` at start-up and again on every monitor, resolution, display-scale,
  fullscreen or preference change (M2.17 mechanism, M2.18 View menu selector).
  All code MUST obtain W, H and the font metrics from the `k`-derived metrics
  value and MUST NOT assume `k = 5` anywhere else.
- The grid only resizes by whole cells (`cols = floor(window_w / W)`,
  `rows = floor(window_h / H)`, never below the 82 × 30 minimum composed by
  §3.1.1). Leftover pixels on the right and bottom edges belong to no cell.

**Font and scale**
- Text is rendered from one monospaced `.ttf` font (`assets/FiraCode-Regular.ttf`)
  through a CPU glyph cache (`ab_glyph`), at one size.
- **One uniform scale** applies to every glyph on both axes:
  `s = W / advance` (pixels per font unit), where `advance` is the font's
  monospace advance width. So one glyph advance is exactly one cell width.
- **Line box:** `L = round((ascender − descender + line_gap) × s)` from the
  font's `hhea` metrics. `L ≤ H` MUST hold. The leading `H − L` is split
  deterministically: `top = (H − L) div 2`, and the rest goes below. (Integer
  division on whole pixels; `div` rounds toward zero.)
- **Baseline:** every glyph shares one integer baseline,
  `baseline_y = top + round(ascender × s)`, measured from the top edge of the
  cell.
- **Horizontal origin:** each glyph's pen origin is at `x = 0` of its cell.
  Because the advance equals W, no per-glyph centring or offset is computed.
- **Clipping:** ink outside the cell rectangle is clipped at the cell edges.
  Glyphs are never rescaled to fit.

**Glyph coverage versus atlas coverage**
- *Font coverage* is whether `assets/FiraCode-Regular.ttf` contains a glyph for
  a code point (its `cmap`). *Atlas coverage* is whether that glyph has been
  rasterised into the current glyph atlas for the current `k`. Neither implies
  the other.
- The **required atlas set** is: printable ASCII (U+0020–U+007E), every
  character occurring in `docs/ui_design/*.txt`, and `▏` (U+258F, text caret).
  It includes characters the palette file `Special_ASCII.txt` does not list
  (`…` U+2026, `◐` U+25D0, `◑` U+25D1, `╻`, `╹`, `×`). Verified 2026-10-15 by
  parsing the `cmap` of `assets/FiraCode-Regular.ttf`: every
  distinct character of the `.txt` references, and `▏`, is covered by the font.
- The atlas builder rasterises the whole required set for the given `k`. A test
  MUST assert font coverage of the required set (so a font swap or a new
  reference character fails loudly) and atlas coverage for each of `k` = 2…7.
- **The atlas is deterministic.** Its content is a pure function of
  (`assets/FiraCode-Regular.ttf`, the required set, `k`). It is built completely
  when it is created (start-up or any `k` transition, §3.1.1), and is never
  modified afterwards: no glyph is added, evicted, re-packed or re-rasterised
  while a given `k` is in use, and no rasterisation happens during a frame or an
  event. There is no on-demand rasterisation.
- **Unknown glyphs.** Every character outside the required set (for example a
  letter typed into a field that is not printable ASCII) is drawn as `?`
  (U+003F, always in the atlas), even if the font contains a glyph for it. The
  same applies to a character the font lacks. The stored text is not modified;
  only its composed cell shows `?`. Wide, combining and right-to-left characters
  are not supported by the one-char-one-cell grid. Extending the required set is
  a documented, deterministic change of this section (and of the font-coverage
  test), never a run-time behaviour.
- **One rasteriser path.** Every glyph in the atlas, including `?`, is
  rasterised by the same CPU routine at the same integer `k` and the same
  uniform scale `s = k / 300`; there is no per-glyph or per-category scaling.

**Verified metrics for Fira Code Regular** (font units, upem 1950): advance
1200, ascender 1800, descender −600, line gap 0. The font's natural cell is
1:2. At 4:9 the leading is 300 units, i.e. 1/8 of the line box. With
`s = 4k / 1200 = k / 300`, every value below is an exact integer for every
permitted `k`: `L = 8k`, `H − L = k`, `top = k div 2`,
`baseline_y = top + 6k`. At k = 5: `s = 1/60`, `L = 40`, `top = 2`,
`baseline_y = 32`. Re-verified 2026-10-15 from the `head`/`hhea` tables of
`assets/FiraCode-Regular.ttf`; `k = 2` re-verified 2026-10-16 (below). If the
font asset changes, these values and the continuity checks below MUST be
re-verified.

| k | Displayed UI Scale label on reference 5 | W × H (px) | L | top | baseline_y |
|---|---|---|---|---|---|
| 2 | — (minimum clamp; reached on compact monitors) | 8 × 18 | 16 | 1 | 13 |
| 3 | 80% (factor 60%) | 12 × 27 | 24 | 1 | 19 |
| 4 | 100% (factor 80%, default) | 16 × 36 | 32 | 2 | 26 |
| 5 | 120% (factor 100%) | 20 × 45 | 40 | 2 | 32 |
| 6 | 140% (factor 120%) | 24 × 54 | 48 | 3 | 39 |
| 7 | — (no menu preset requests k = 7) | 28 × 63 | 56 | 3 | 45 |

`k = 2` is the smallest permitted cell. On a reference-scale-5 monitor the
80% label requests factor 60%, which rounds to `k = 3`; on a compact monitor
the selected presets can be clamped to `k = 2` (§3.1.1).

**Fira Code at `k = 2` (8 × 18 px), verified 2026-10-16.** `s = 1/150`, one em =
13 px, line box `L = 16`, leading `H − L = 2` (`top = 1`, one blank row below),
`baseline_y = 13`. Method: the atlas rasteriser (`CellLayout` formula, `ab_glyph`
0.2.32, uniform scale, baseline at `top + round(ascender × s)`) was re-run in a
scratch program on `assets/FiraCode-Regular.ttf` for `k` = 2…7, and the
coverage rasters were inspected as ASCII dumps; production code was not touched.
This is a numeric and coverage check, **not** a visual check on a screen.
- *Coverage.* All 150 non-space characters of the required set (§Glyph
  coverage) are in the font and rasterise; all 95 printable ASCII glyphs and all
  150 required characters give pairwise different rasters (no two collapse into
  one at 8 × 18).
- *Proportions.* Cap height 10 rows (3…12), x-height 8 rows (5…12), descender
  to row 15 (`g`), so rows 16–17 stay free below the baseline block and the
  ascender-to-descender clearance between adjacent text rows is 5 px. Stems are
  anti-aliased and about 1.2 px wide; `0` shows its inner slash/dot, `O` does
  not; `l`, `1` and `I` differ by their serif structure.
- *Weakest pairs.* The closest distinct rasters differ by only 2–3 px of summed
  coverage: `.`/`,`, `:`/`;`, `` ` ``/`'`, `h`/`n` (3), and among box drawing
  `─`/`┄`/`┈`, `║`/`╟`/`╢`, `╓`/`╥`/`╖` (2–3). They remain distinct but are
  hard to tell apart by eye. `┄`, `┈`, `╟`, `╢`, `╓`, `╥`, `╖` occur only in the
  palette file `Special_ASCII.txt`, in no interactive reference state, so this is
  accepted. The acceptance of the punctuation pairs (`.`/`,`, `:`/`;`) is a
  legibility risk to confirm visually on a Full HD monitor (M2.17 manual check).
- *Glyph continuity.* `─` spans the full cell width in rows 7–8 with identical
  coverage (51 / 221) on the left and right edges, `│` spans the full height in
  columns 3–4 with identical coverage (136) on the top and bottom edges, and
  every tested join has edge-identical rows or columns with its mate: `┌ ┐ └ ┘ ├
  ┤ ┬ ┴ ┼` with `─`/`│`, and `╔ ╗ ╚ ╝ ╠ ╣ ╦ ╩ ╬` with `═`/`║`. Box-drawing
  glyphs therefore join without a gap or step. The font's vertical extent at
  `k = 2` is −750…1950 units, inside the −1000…2000 stroke range.
- *Known weakness (accepted).* At `k = 2` a stroke is only about 1.07 px thick
  and is split over two pixels, so box lines and frames are drawn at 40–85%
  coverage (20–87% for `─`, 53% for `│`), never as a solid full-coverage line
  (anti-aliased blends are permitted, §3.6): thinner and lower in contrast than at `k` ≥ 4, where full-coverage
  pixels exist. They are continuous, not broken. If this is judged too faint on
  screen, the remedy is a decision of the human (a heavier font weight or a
  procedural box-drawing rasteriser), never stretching.
- *Shading and caret.* `░ ▒ ▓ █` give mean coverage 0.22 / 0.44 / 0.67 / 0.89
  (distinct dither rows of the 16-row line box); the caret `▏` is exactly
  1 px wide (`k / 2`, no anti-aliasing at even `k`) and 16 rows tall.
- *Hit-testing.* A half cell (sub-cell modulo logic, §3.4) is 4 px wide, still a
  whole number of pixels.

**Forbidden**
- Anisotropic scaling (`scale_x ≠ scale_y`), horizontal glyph stretching or
  "widening" factors.
- Per-character scale factors, per-glyph narrowing, or fitting glyphs to the
  cell by rescaling.
- Artificial inter-character spacing (tracking or letter-spacing). Cells are
  adjacent with no gap.
- Visual correction using extra blank cells, padding columns or padding rows.
- Fractional cell sizes, and resampling the glyph raster at draw time. One
  atlas texel maps to one screen pixel and is sampled nearest-neighbour
  (**sampling contract of the glyph atlas**). The only permitted filtered
  sampling is that of logo textures (§3.3); it never applies to a glyph.

**Unicode continuity**
- Box drawing (U+2500–U+257F) MUST join across adjacent cells horizontally and
  vertically using the font's own glyphs at the uniform scale. Verified for
  Fira Code at 4:9: horizontal strokes span −10…1210 units, which covers the
  cell's 0…1200; vertical strokes span −1000…2000 units, which covers the
  cell's vertical extent. The cell's vertical extent in font units is
  `(1800 + 300·(k div 2)/k) − 2700 … 1800 + 300·(k div 2)/k`: −750…1950 for even
  `k` (k = 2: −750…1950), and slightly lower for odd `k` (k = 3: −800…1900;
  k = 5: −780…1920; k = 7: ≈ −771…1929). All six permitted `k` stay inside
  −1000…2000.
  Overshoot is removed by clipping. A procedural box-drawing rasteriser stays
  deferred, and needs approval if a future font fails this check at any
  permitted `k`.
- Block elements (U+2580–U+259F) tile horizontally: the glyph covers the full
  advance. Vertically they cover only the line box (L, not H), so vertical
  tiling of block elements is NOT guaranteed: stacked block rows show `k` px of
  background between them. No reference state stacks them. Two consequences are
  accepted and MUST NOT be fixed by stretching, extra rows or a procedural
  rasteriser: the progress-bar cells (`█ ▓ ▒ ░`, one row) are `L = 8k` px tall
  inside the `H = 9k` cell, and the text caret `▏` (U+258F) covers the left
  eighth of the cell, `k/2` px wide, also `L` tall (anti-aliased at odd `k`).
  Horizontal tiling of `█` across five cells MUST be tested for each `k`.

**Single coordinate system**
- All UI geometry uses one space: **physical surface pixels**, origin at the
  top-left of the window surface, `x` to the right and `y` downward, integers.
- Cell `(col, row)` covers `[col·W, (col+1)·W) × [row·H, (row+1)·H)`.
- The glyph raster for a cell is exactly W × H pixels in this same space.
  Layout, the atlas slot, the GPU cell quad and hit-testing all use the same
  W and H. No component may use its own cell size, offset or scale.
- Pointer positions are floored to integer pixels and then resolved by §3.4
  division and modulo. Because W = 4k is always even, the two halves used by
  Sub-Cell Modulo Logic are exactly W/2 each.
- OS display scaling (macOS Retina / HiDPI) never scales or resamples the
  rendered grid. All coordinates are physical pixels as reported by `winit`
  (`PhysicalSize`, `PhysicalPosition`, the surface size). Logical points and
  `scale_factor` are never used to compute W, H, cell counts or hit-tests.
  The user-visible size of the UI is chosen by the UI Scale preference together
  with the physical dimensions of the monitor in pixels (`effective_k`,
  §3.1.1); a change of macOS display scale acts only through the new monitor
  pixel size it produces.
- **Physical-size boundary semantics.** Every window size handed to `winit`
  (initial size, minimum size, `request_inner_size`, the `ScaleFactorChanged`
  size writer) is an explicit `PhysicalSize` (`Size::Physical`), never a
  `LogicalSize`. `winit` may convert it through the window's current
  `scale_factor` and the OS may round, so the size actually applied can differ
  from the request: at an integer factor such as Retina 2.0 the minimum
  `328k × 270k` is exact (both even), at a fractional factor it can be off by a
  pixel. The size reported after creation, by `Resized` or by `inner_size()`,
  is the only truth and the grid is derived from it with `floor`. The minimum is
  inclusive: a surface of exactly `82·W × 30·H` gives exactly 82 × 30 cells; one
  pixel less would give 81 columns or 29 rows, which is never composed (§3.1.1).

### 3.1.1 UI Scale

> **[APPROVED ARCHITECTURAL CHANGE — M2.17 / M2.18, amended 2026-10-16:
> automatic per-monitor adaptation]** UI Scale is a **user preference**, not a
> fixed physical size. The physical cell size (`k`) is derived automatically for
> the monitor that displays the window, and is re-derived whenever that monitor,
> its resolution, the window mode or the preference changes. **M2.17** owns the
> whole automatic mechanism (derivation, triggers, atomic transition, window
> fit and repositioning) with the selected label fixed at 100% (factor 80%).
> **M2.18** adds only
> the user-facing View menu selector that changes the preference. The previous
> one-shot start-up rule and the "one percentage = one `k` on every monitor"
> assumption are withdrawn.

**UI Scale presets.** Exactly four labels are selectable: 80%, **100% (default)**,
120% and 140%. The displayed label is not the multiplier used by the existing
scale calculation. Each label maps to the former rendering multiplier as
follows:

| Displayed label | `scale_factor_percent` | Former preset |
|---|---:|---:|
| 80% | 60% | 60% |
| 100% (default) | 80% | 80% |
| 120% | 100% | 100% |
| 140% | 120% | 120% |

The former 140% preset is removed. There is no displayed 60% preset. The
rendering multiplier is used by the unchanged per-monitor derivation below; on a
monitor with reference scale 5, the four choices request `k` = 3 / 4 / 5 / 6
(cell sizes 12×27 / 16×36 / 20×45 / 24×54 px). On a compact-class monitor
(reference scale 2, e.g. Full HD), all four choices can resolve to `k = 2`,
8×18 px, under the existing minimum-cell clamp.

**Four separate values per window (§4.2).** They are never merged:

| Name | Meaning | Changed by |
|---|---|---|
| `ui_scale_label_percent` | The selected visible label, one of 80/100/120/140. Default 100. | Only an explicit user action on the UI Scale control (M2.18). Never by a monitor, mode or resolution change. |
| `scale_factor_percent` | The rendering multiplier paired with the label: 60/80/100/120. Default 80. | Derived from the selected label using the fixed mapping above. |
| `monitor_reference_scale` | The integer `k` corresponding to a 100% rendering multiplier on the monitor; range 2…5 (only 2, 4 and 5 occur, below). | Derived from the monitor only (below). |
| `effective_k` | The integer `k` in use: the only value from which W = 4k and H = 9k come. Range 2…7. | Derived (below); never stored as a preference. |

The View menu shows `ui_scale_label_percent`, so the startup choice reads
`100%` on every monitor. It never shows a value derived from `effective_k`, and
the adaptation is invisible to the user: no dialog, no notice, no change of the
displayed label. A change of `effective_k` never changes the selected label, so
returning to the original monitor restores the original look without any user
action. Persisting the selected label across sessions is not defined by this
document. A new window starts with UI Scale label 100% (factor 80%), the Dark
theme and Color unchecked (§3.6), regardless of other windows and of any OS
appearance setting.

**Inputs of the derivation.** `effective_k` is a pure function of exactly four
inputs and of nothing else:

1. the monitor snapshot: physical width `mon_w` and height `mon_h` in pixels as
   reported by `winit` (`MonitorHandle::size`);
2. the window mode: *windowed* (including maximised) or *fullscreen*;
3. `scale_factor_percent` (mapped from `ui_scale_label_percent`);
4. the supported integer cell sizes `2…7` and the minimum grid 82 × 30
   (`328k × 270k` physical px, below).

The window's current size and position, the OS `scale_factor`, the previous
`effective_k` and the event history are **not** inputs (§3.1 HiDPI rule; see
*No oscillation*). The monitor's physical size already contains the macOS
display scale: a different macOS "scaled resolution" reports a different pixel
size and therefore re-derives `k`, while `scale_factor` itself is still never
used to compute a cell size.

**Derivation (all integer arithmetic, deterministic).**

```text
// 1. Largest k whose 82×30 grid fits the monitor (may be < 2 or > 7)
avail_h(windowed)   = mon_h - mon_h / 16        // §Window fit reserve estimate
avail_h(fullscreen) = mon_h                      // no reserve
k_fit(mode)         = min( mon_w / 328 , avail_h(mode) / 270 )     // floor divisions

// 2. Monitor reference scale = what a 100% rendering multiplier means here
//    compact class (k_fit <= 3): one step below the fit, floor 2  -> always 2
//    otherwise: the fit, capped at 5                              -> 4 or 5
monitor_reference_scale = ( k_fit(windowed) <= 3 )
                        ? max( k_fit(windowed) - 1, 2 )
                        : min( k_fit(windowed), 5 )

// 3. Requested k: round-half-up of reference × percent
k_req = ( monitor_reference_scale * scale_factor_percent + 50 ) / 100

// 4. Clamp to the supported range and to what fits in the current mode
k_cap       = clamp( k_fit(mode), 2, 7 )
effective_k = clamp( k_req, 2, k_cap )          // = min( max(k_req, 2), k_cap )
```

*Rounding rule.* Step 3 is round-half-up on non-negative integers. A tie would
need `reference × percent ≡ 50 (mod 100)`; every rendering multiplier is a multiple of 20,
so `reference × percent` is a multiple of 20 and never ends in 50: **no tie can
occur** and the rounding is unambiguous. `k_req` is monotonically non-decreasing
in the percentage, so a larger preference never gives a smaller `effective_k`.

*Clamping rule.* Step 4 clamps in this order: lower bound 2 (the smallest
supported cell), then upper bound `k_cap` (2…7, from the monitor and mode). The
lower bound wins when `k_fit < 2`: the application then runs at `k = 2`, keeps
the exact 4:9 ratio and the 82 × 30 minimum, and logs the physical limitation
**once** per distinct monitor snapshot (the window may overflow the monitor, see
*When even `k = 2` does not fit*). The ratio is never relaxed, and the grid is
never reduced below 82 × 30. The lower bound also raises a `k_req` below 2:
at reference 2, factor 60% requests `k = 1` and is raised to 2; the other
available factors request 2 or more and are also clamped by fit as needed.

*Why the reference is capped at 5 and taken from the windowed fit.* `k = 5`
(20 × 45 px) is the design reference size (§3.1); a very large monitor does not
make a 100% rendering multiplier larger than the reference, it only makes
larger rendering factors available. The reference is taken from the windowed fit in **both** modes so
that entering or leaving fullscreen does not change what the 100% rendering
multiplier means: only the cap `k_cap` differs between the modes.

*Why the compact class is one step below its fit (amended 2026-10-16).* On a
monitor whose windowed fit is 3 (usable height 810…1079 px and width ≥ 984, e.g. a Full HD
1920 × 1080 display) the 82 × 30 grid at `k = 3` already needs 984 × 810 px, about 80% of the
usable height (810 of 1013 px), and the human decision is that 100% is one step
smaller there: `k = 2`, 8 × 18 px (656 × 540 px minimum, about half the height). The
class is recognised **only** by the pixel-derived value `k_fit(windowed) ≤ 3`;
no monitor name, model, vendor, resolution string or DPI value is ever tested
or hard-coded, and any monitor with the same fit behaves the same. The MacBook
Retina rows and every monitor with `k_fit(windowed) ≥ 4` are unchanged. The
reference therefore takes only the values 2, 4 and 5; 3 is reachable as
`effective_k` (a preference on a larger monitor, or the cap) but is never a
reference. The selected label does not change: 100% still maps to factor 80% on
both kinds of monitor.

**Examples** (`mon_w × mon_h` → `k_fit` windowed / fullscreen → reference →
`effective_k` for rendering factors 60 / 80 / 100 / 120; displayed labels are
80 / 100 / 120 / 140 in the same order):

| Monitor (physical px) | `k_fit` win / fs | Reference | Windowed factors 60 / 80 / 100 / 120 | Fullscreen factors 60 / 80 / 100 / 120 |
|---|---|---|---|---|
| 1366 × 768 | 2 / 2 | 2 | 2 / 2 / 2 / 2 → cells 8 × 18, fits (656 × 540 ≤ 1366 × 720) | 2 / 2 / 2 / 2 |
| **1920 × 1080 (Full HD, external)** | 3 / 4 | **2** | 2 / 2 / **2** / 2 → every label uses cells **8 × 18** | 2 / 2 / 2 / 2 |
| 1920 × 1200 | 4 / 4 | 4 | 2 / 3 / 4 / 4 | 2 / 3 / 4 / 4 |
| 2560 × 1440 | 5 / 5 | 5 | 3 / 4 / 5 / 5 | 3 / 4 / 5 / 5 |
| **2560 × 1664 (MacBook Air Retina)** | 5 / 6 | **5** | 3 / **4** / 5 / 5 → default label 100% gives **16 × 36** | 3 / 4 / 5 / 6 |
| 3024 × 1964 | 6 / 7 | 5 | 3 / 4 / 5 / 6 | 3 / 4 / 5 / 6 |
| 3840 × 2160 | 7 / 8 | 5 | 3 / 4 / 5 / 6 | 3 / 4 / 5 / 6 |

The MacBook Air and Full HD rows demonstrate the startup choice: visible label
100% maps to the former 80% rendering multiplier, producing `k = 4` (16 × 36
px) on the Retina display and `k = 2` (8 × 18 px) on Full HD. The exact pixel
size the macOS Retina panel reports depends on its selected scaled resolution.
On a monitor able to fit all sizes, visible labels 80 / 100 / 120 / 140 produce
the former multiplier sizes `k = 3 / 4 / 5 / 6`.

**Several labels may share one `k` (documented behaviour).** On 1920 × 1080
windowed all four labels resolve to `k = 2`; on 2560 × 1664 visible labels
120% and 140% both resolve to `k = 5`. This is intended under the existing
monitor-fit behavior. Selecting another label that maps to the same
`effective_k` stores that choice and triggers no transition. The menu never
skips or hides a label: all four are selectable.

**Invariants at every `effective_k`.** Exact 4:9 cells with `W = 4k`,
`H = 9k`; one uniform glyph scale (`s = k / 300`) and natural Fira Code
proportions; whole physical pixels; no resampling of text glyph rasters at draw
time; the layout grid, glyph atlas, GPU cell quads and hit-testing all use the
same `k`-derived metrics value; 82 × 30 minimum grid; O(1) hit-testing; one UI
draw call. The derivation changes only `k`; it adds no rendering path.

**Re-evaluation triggers (M2.17).** `effective_k` is re-derived, and the
transition below runs if it differs, when and only when one of these happens:

1. the application starts (before window creation, from the primary monitor;
   if no monitor is reported the reference is 5 and `k_cap` is 7, so
   `effective_k = clamp(k_req, 2, 7)`, and the derivation is repeated as soon
   as a monitor snapshot becomes available);
2. the window moves to another monitor (the monitor reported by `winit` for the
   window, `Window::current_monitor`, differs from the last snapshot);
3. the monitor's resolution changes (its physical size differs from the
   snapshot);
4. the macOS display scale changes (monitor size and/or `ScaleFactorChanged`;
   the new snapshot is what counts);
5. the window enters or exits fullscreen, whether requested by the application
   or by the OS;
6. `scale_factor_percent` changes (M2.18 control; M2.17 tests it with a
   stub).

`winit` 0.30 has no monitor-change event. The Controller therefore takes a
snapshot `(monitor handle, physical size, scale_factor bits)` and compares it
with the stored one on `Moved`, `ScaleFactorChanged`, `Resized`,
`Focused(true)`, every fullscreen change and at most every 500 ms from the
event loop's idle point. The comparison reads only the monitor, never the
window size, and costs O(1); a different snapshot is trigger 2, 3 or 4.
`Resized` of the window itself is **not** a trigger (it is the echo of the
transition, see *No oscillation*).

**Transition (atomic, one event-loop turn, M2.17).** One function applies a
re-derivation; nothing else may change `k`, the atlas, the grid or the minimum
size. It runs on the event-loop thread, before any other event of the turn is
handled, in two phases.

*Phase A — prepare (no live state is touched).* From the new `effective_k` and
the window mode compute, as values:
(1) the new metrics (`W = 4k`, `H = 9k`, font metrics);
(2) the new glyph atlas (§Glyph atlas below) and its GPU texture and bind group;
(3) the target physical window size and position (*Safe window fit*, below);
(4) the new `CellGrid` and flat cell buffer: `cols`/`rows` computed from the
**target** surface size, never below 82 × 30, then the full recomposition;
(5) the new GPU cell buffer(s) and uniform data (cell size, grid size, surface
size, atlas parameters), created and filled but not bound;
(6) the new minimum inner size `PhysicalSize(328k, 270k)`;
(7) the new pointer-mapping metrics.
If any step fails (allocation, atlas build, GPU creation error) the transition
is **abandoned as a whole**: the old state stays live and consistent, the failure
is logged once for that input tuple, and the same tuple is not retried until an
input changes (no retry loop).

*Phase B — commit (cannot fail).* In this order:
(1) cancel any in-progress press, drag or application-managed resize, and set
the stored pointer position to *unknown* (it was reported in the old pixel
space and may belong to another monitor; nothing is synthesised, and a press
cancelled here is ignored on release);
(2) publish, as **one replacement of one `ScaleState` value**, the metrics,
glyph atlas, `CellGrid`, flat cell buffer, GPU buffers, uniforms and
pointer-mapping metrics, and drop the old atlas and buffers (at most one atlas is
live per window);
(3) `set_min_inner_size(PhysicalSize)` for the new `k`; then
`request_inner_size(PhysicalSize)`; then `set_outer_position` if the safe fit
moved it (this order: the lowered minimum cannot clamp a shrinking request, a
raised minimum cannot exceed the requested target, and macOS derives the vertical
flip from the current frame);
(4) request a redraw; logical focus, selection, expansion and open-overlay
state survive and are recomposed.
Rendering, layout, atlas, hit-testing, minimum size and pointer mapping
therefore never observe different `k` values: they are fields of the same
published value. Presentation of frames between commit and the first `Resized`
that matches the target is **gated for at most 250 ms** (the previous frame
stays on screen); after the deadline the frame is drawn cropped at the actual
surface edge exactly as for any asynchronous size (*Minimum window grid*), and
a mismatch is logged once.

*Fast path.* If only the window mode, monitor or position changed but
`effective_k` is unchanged, no atlas, buffer or metrics value is rebuilt: only
the window fit, the minimum size and the grid for the new surface are applied.
The atlas depends on `k` alone, so it is regenerated **only when `k` changes**
and is never touched by a theme change, a window resize, or a monitor change
with the same `k`.

**No oscillation and no recursive resize.**
- *One-way dependency.* `effective_k` is a function of
  (monitor snapshot, mode, preference). The window size and position are
  **outputs** of the transition, never inputs of the derivation, so a resize cannot
  change `k`, and a `k` change cannot feed back into itself.
- *Idempotence.* Re-running the derivation with an unchanged input tuple is a
  no-op: same `k` → fast path → nothing requested if the window already has the
  target size and position.
- *Echoes.* The transition stores `expected_inner_size` and a generation number.
  `Resized`, `Moved` and `ScaleFactorChanged` events that arrive while it is
  pending are used only to verify the result and to recompose the grid at the
  same `k`; they never start a transition. The `ScaleFactorChanged` size writer
  always writes the transition's expected size, or, when no transition is
  pending, the unchanged current physical size (never the value `winit`
  proposes, which is a logical-point conversion).
- *Settling during a move.* While the window is being dragged (a `Moved` event
  within the last 150 ms, or a pointer gesture active) a monitor change is
  recorded but **not applied**; it is applied once, after 150 ms without `Moved`
  (the event loop wakes with `ControlFlow::WaitUntil`), or immediately for the
  non-move triggers (4, 5, 6). Crossing a monitor boundary back and forth
  during one drag therefore produces at most one transition, for the monitor in
  which the window comes to rest. During those ≤ 150 ms the old `k` stays in use
  and is cropped or letter-boxed by the renderer; this brief interval is
  accepted.
- *Bounded correction.* After the commit, the actual size and position are read
  back. If they differ from the target, at most **one** corrective
  size/position request is made per input tuple; any remaining difference is
  logged once and accepted. There is no loop that chases the OS.
- *Fit invariant.* After a successful fit the window's outer rectangle lies
  inside the monitor, so the monitor reported for it cannot flip back because of
  the transition itself.

**Glyph atlas.** The atlas depends on `k` only: it is rasterised on the CPU
(`ab_glyph`) at the new `k` in Phase A and is otherwise reused. It is a coverage
mask, so it is theme-independent; theme changes, window resizes, fullscreen and
monitor changes with the same `k` never regenerate it. Regeneration is
synchronous on the event-loop thread.

**Pointer coordinate mapping.** Pointer positions are physical pixels (§3.1);
the cell under the pointer is `floor(x / W), floor(y / H)` with the `W`, `H` of
the **same published `ScaleState`**, so hit-testing remains O(1) and can never
use a different `k` than the renderer. After a transition the pointer is over
whatever cell is now at its physical position, and only the next `CursorMoved`
(in the new coordinate space) updates it. A control moved from under the
pointer is not re-activated by the same click, and a repeated click acts on the
control that is now there.

**Logo allocations** keep their whole-cell sizes (§3.3), so they scale with `k`
(major 7 × 3 cells = 28k × 27k px). The "no resampling" rule above concerns text
glyph rasters; logos follow the §3.3 exception.

**Minimum window grid.** The window MUST be at least **82 columns × 30 rows** at
every `effective_k`: `82·4k × 30·9k` = `328k × 270k` physical px (k = 5:
1640 × 1350; k = 3: 984 × 810; k = 2: 656 × 540; k = 7: 2296 × 1890).
- *Enforcement.* The OS minimum inner size is set from this value as an explicit
  `PhysicalSize` at window creation and **re-applied by every transition** (any
  change of `k`, monitor or mode) and after `ScaleFactorChanged`. Setting it may
  be unsupported or silently adjusted by the platform; the result is never
  assumed.
- *Composition.* Layout never composes a grid smaller than 82 × 30, whatever the
  surface size: `cols = max(82, floor(surface_w / W))`,
  `rows = max(30, floor(surface_h / H))`.
- *Asynchronous or short surfaces.* While the platform has not yet applied a
  size (resizes are asynchronous) or delivers a surface smaller than
  `328k × 270k`, the 82 × 30 grid is still composed and the renderer crops it at
  the surface edge: cells (or parts of cells) outside the surface are not
  drawn, hit-testing of positions outside the surface returns nothing, and no
  panic, zero-size surface or stretching occurs. A zero-sized surface
  (minimised) skips drawing as before. The ratio is never changed to fit.
- *Window above the minimum.* Any larger size is whole-cell
  (`floor`); leftover pixels on the right and bottom belong to no cell.
- *Reference shell.* `Interface_Exemple.txt` is an 82 × 28 reference. The authorized
  2026-10-26 tree endpoint amendment replaces its one terminal playhead blank
  with `─`; its dimensions and other cells are preserved. The two rows by which the minimum exceeds it belong to the flexible
  central body, never to the fixed MenuBar, CommandBar or StatusBar heights; how
  82 × 30 snapshots are compared with the reference is defined in
  `docs/ui_design/README_UI.md` §25.

**Window fit (reserve estimate).** `winit` 0.30 reports a monitor's physical
size but not the OS work area (menu bar, Dock, taskbar), and `unsafe` is
forbidden, so the work area is estimated. The *usable rectangle* of a monitor
is the monitor rectangle minus `mon_h / 16` (integer division) at its **top**
edge (the menu bar is always on top on macOS; a Dock or taskbar at another edge
is not modelled). In fullscreen the usable rectangle is the whole monitor.
The 1/16 reserve is a **conservative estimate, not a guarantee**: the
application MUST NOT assume that the monitor size equals the available area and
MUST NOT treat the estimate as proof that a window fits. This is the same
`usable_height = mon_h − mon_h / 16` used by `k_fit(windowed)`.

**Safe window fit (monitor change, `k` change, M2.17).** The transition
computes the target size and position with one pure function
(`fit_window`, `newell-ui`, no `winit` types), from the new metrics, the
usable rectangle of the new monitor, the *desired grid* and the current outer
position:
1. *Desired grid.* The window remembers `desired_cols × desired_rows`, the grid
   the user last chose (initial window, an application-managed resize, a
   maximise/restore). Derived changes never overwrite it, so a window moved to a
   smaller monitor and back regains its previous grid, exactly as the preference
   is kept.
2. *Size.* `cols = clamp(desired_cols, 82, max(82, floor(mon_w / W)))`,
   `rows = clamp(desired_rows, 30, max(30, floor(usable_h / H)))`; the target
   inner size is `(cols·W, rows·H)` whole cells. Because `effective_k ≤ k_fit`
   (when `k_fit ≥ 2`) the minimum 82 × 30 always fits, so the clamp interval is
   never empty.
3. *Position (minimum move).* Keep the current outer position and shift it only
   as far as needed so that the whole outer rectangle lies inside the usable
   rectangle: `x' = clamp(x, usable_left, usable_right − w)`,
   `y' = clamp(y, usable_top, usable_bottom − h)`. The window has no native
   decorations, so inner and outer rectangles coincide. With `w ≤ mon_w` and
   `h ≤ usable_h` the interval is non-empty. Keeping the whole rectangle
   inside guarantees that the in-grid MenuBar row (the only move handle of a
   frameless window) and `[-] [□] [X]` stay reachable.
4. *No fit possible* (`k_fit < 2`): size 82 × 30 at `k = 2`; position the top-left
   corner of the outer rectangle at the top-left of the usable rectangle;
   the overflow is logged once.
5. *Fullscreen.* Entering fullscreen stores the windowed desired grid and
   position and lets the OS size the window to the monitor; the grid is
   `max(82,floor(mon_w/W)) × max(30,floor(mon_h/H))`. Leaving restores the
   desired grid through steps 2–3 on the monitor then showing the window. A
   fullscreen request when `k = 2` would not fit the monitor
   (`k_fit(fullscreen) < 2`, i.e. below 656 × 540 physical px) is **refused**: the
   window stays as it is, the request has no visual effect, and the refusal is
   logged once.
6. *Maximised.* A maximised window keeps the size the OS gives it; the
   derivation uses the windowed rule; only the minimum size and the grid are
   re-applied.
7. *Verification.* After the commit the application reads back inner size,
   outer size, outer position and the current monitor
   (`Window::current_monitor`); `winit` window operations may complete
   asynchronously, so the first matching `Resized`/`Moved` events and the
   read-back are the truth, not the request. At most one corrective request is
   made (*No oscillation*); a remaining mismatch is logged once. Positions
   returned by `winit` on a multi-monitor desktop with different scale factors
   are compared only with monitor rectangles read in the same turn.
The true work area is unknown, so a window can still overlap a Dock or taskbar.

**When even `k = 2` does not fit.** If `k_fit(windowed) < 2` the application
runs at `k = 2`, preserves the exact 4:9 grid and the 82 × 30 minimum (the window
may overflow the monitor), and logs the physical limitation **once** per
monitor snapshot. It never reduces the grid below 82 × 30, never changes the 4:9
ratio and never picks a fractional `k`. Neither the cell ratio nor the minimum
grid is ever violated silently.

**Milestone split.**
- *M2.17 — automatic per-monitor adaptation (everything above except the
  control).* Pure derivation (`monitor_reference_scale`, `k_req`, `effective_k`,
  `fit_window`) in `newell-ui`; the per-window triple of §3.1.1 with
  `scale_factor_percent` fixed at 80% (the default 100% menu label); start-up selection before
  window creation with explicit `PhysicalSize` initial and minimum dimensions
  (initial size = the 82 × 30 minimum at the derived `k`, or the fitted desired
  grid); monitor snapshot and all six re-evaluation triggers (trigger 6 is
  exercised by tests and a stub until M2.18); the atomic transition with
  rollback; settling, echo handling and bounded correction; the safe window
  fit/reposition; fullscreen entry/exit handling and the refusal rule, whichever
  way fullscreen is requested; atlas rebuild on `k` change only; handling of
  unsupported platform APIs; the minimum-size enforcement; logging; and the
  frameless move/resize mechanism below (including the application-managed edge
  resize). No View-menu control.
- *M2.18 — user-facing selector only.* The View menu row
  `UI Scale ◀ 100% ▶` showing the selected label and mapping four labels to
  rendering factors 60/80/100/120, bounded at 80% and 140% with hidden blank
  arrow cells and no wrapping; the `Theme` row; the `Fullscreen` item, which
  asks the M2.17 fullscreen gate and adds no sizing logic of its own. Changing the
  preference calls the M2.17 transition; M2.18 contains no `k` arithmetic,
  window fit or monitor logic.

**Window move and resize (frameless window, M2.17).** The window has no native
decorations, so it MUST stay manually resizable and movable from the grid.
- *Move.* `drag_window` (a `Result`, handled without `unwrap`/`panic`).
- *Resize, native first.* A press on an edge or corner region (zones and
  precedence: `docs/ui_design/README_UI.md` §25.1) first calls
  `drag_resize_window`. `Ok` hands the resize to the OS. `drag_resize_window`
  always returns `ExternalError::NotSupported` on macOS (verified in `winit`
  0.30.13, `platform_impl/macos/window_delegate.rs`) and on iOS, Android and Web;
  other platforms may also refuse. `NotSupported` is logged once per window,
  remembered, and the press then starts the **application-managed resize**
  below. `Ignored` and `Os` errors are logged and do nothing.
- *Application-managed resize.* It uses only supported, safe `winit` 0.30 API:
  pointer events (`CursorMoved`, `MouseInput`, `Focused`, `ScaleFactorChanged`),
  `Window::outer_position`, `inner_size`, `request_inner_size`,
  `set_outer_position`, `set_min_inner_size`, `is_maximized`, `fullscreen`. No
  `unsafe`, no raw window handles, no platform shim, no undocumented behaviour.
  1. *Start.* On a left-button press in a resize zone (the cell under the
     pointer carries no `widget_id`) the Controller records a snapshot: outer
     position `O0` (physical, `i32`, may be negative), inner size `S0`, the zone,
     and the grab point `G0 = O0 + pointer`. It sets no widget pressed state and
     routes nothing else until the resize ends.
  2. *Update (each `CursorMoved`).* The pointer in screen space is
     `P = outer_position() + pointer`, read in the same handler. The target is
     re-derived from the snapshot every time (`d = P − G0`; east/south edges add
     `d` to the size, west/north edges subtract it and move the origin so that the
     opposite edge stays fixed). It is never accumulated from previously applied
     sizes, so asynchronous or adjusted resizes cannot drift.
  3. *Whole cells and minimum.* The target inner size is rounded to the nearest
     whole number of cells and clamped to `[82·W × 30·H, device surface limit]`
     (§3.1.1). A west/north origin is computed from the clamped size, so the
     window stops at the minimum instead of sliding.
  4. *Apply.* Only when the snapped target differs from the last requested one:
     `request_inner_size(Size::Physical)` **first**, then `set_outer_position`
     (`Position::Physical`) with the computed origin, always, because on macOS
     `set_outer_position` derives its vertical flip from the window's current
     frame and `request_inner_size` returns `None` (applied asynchronously,
     reported by `Resized`). Neither call is assumed to succeed or to produce
     the requested size; the layout follows `Resized` / `inner_size()` only
     (§3.1, physical-size boundary semantics).
  5. *Cancel.* The resize ends, with no further size request, on left-button
     release (also outside the window), `Focused(false)`, `ScaleFactorChanged`,
     `CloseRequested`, the window becoming maximised or fullscreen, a UI Scale
     change (§3.1.1 Transition), `Escape`, or `outer_position()` returning an
     error. The final release is consumed by the resize and never activates a
     control.
  6. *Gating.* The mechanism is inactive while maximised or fullscreen, and on
     any platform where `drag_resize_window` is not `NotSupported`.
- *Limitation (binding, reported).* `winit` has no call that moves and resizes a
  window atomically, and pointer coordinates are window-relative. East, south
  and the south-east corner keep the origin fixed and are therefore robust. West,
  north and the other corners need two non-atomic calls and a pointer position
  derived from a moving origin; a stale pointer sample can produce visible
  jitter that cannot be excluded without a runtime test. M2.17 MUST verify those
  five zones manually on macOS (README_UI §25.1 checklist) and report the
  result; if they are unacceptable the Executor reports it and the human
  decides. No unsafe or platform-specific workaround is allowed to fix it.
- *Controls.* `[-] [□] [X]` (minimise, maximise/restore, close) and the MenuBar
  drag region are unaffected: they are ordinary grid targets, and every cell
  that carries a `widget_id` wins over a resize zone.

### 3.2 The Texture Atlas (Logos)
- Tool icons (Extrude, Sketch, etc.) MUST be loaded from a **single RGBA `.png` Texture Atlas**.
- **Coverage-mask ink (§3.6):** The single RGBA logo/icon Texture Atlas keeps its RGBA format, Power-of-Two resolution and padded slices. The UI shader MUST treat its alpha channel (or equivalent luminance) as coverage only; atlas RGB MUST NOT reach the screen. Each logo/icon cell is assigned a semantic ink role by its component: `Text` by default, or `Action` only when that cell is explicitly an actionable affordance. With View > Color unchecked, both roles paint the current monochrome theme foreground over its background. With Color checked, coverage blends the selected role ink over the current palette background. Neither Theme nor Color regenerates the atlas. This does not alter the 240 × 240 visible art within each 256 × 256 slice, the 8 px transparent inward padding, or the one batched GPU UI draw call.
- **Resolution:** The atlas must be a Power of Two (POT), e.g., 1024x1024 or 2048x2048.
- **Internal Padding:** Each logo is allocated a strict 256x256 pixel slice in the atlas. The visible logo MUST be drawn at 240x240, leaving an 8px transparent inward padding to prevent texture bleeding during GPU downscaling.

### 3.3 Asymmetric Grid Allocation & Shader Centering
Because the cell ratio is 4:9 (§3.1), square logos must be allocated asymmetric blocks in the UI logic. In cell units (4 wide, 9 tall), an exact square needs 9 columns × 4 rows. That is too large for toolbars, so the near-square blocks below are used:
- **Major Tools:** Allocate **7 columns × 3 rows** (28 × 27 units, ratio ≈ 1.04; at k = 5: 140 × 135 px, logo drawn at 135 × 135).
- **Minor Tools:** Allocate **4 columns × 2 rows** (16 × 18 units, ratio ≈ 0.89; at k = 5: 80 × 90 px, logo drawn at 80 × 80).
- The previous 5 × 3 and 3 × 2 allocations assumed 3:5 cells and are superseded.
- Logo allocations are whole cells only. They MUST NOT be adjusted with blank padding cells (§3.1).
- **Logos are not implemented in M2.16–M2.24.** This section binds the first milestone that renders logos (see `docs/plans/M2-ui-framework.md` §12.5).
- **Logo resampling exception.** The “no resampling, nearest-neighbour” rule of §3.1 applies to the text glyph atlas only. A logo is a 240 × 240 drawing in a 256 × 256 atlas slice shown in the existing whole-cell major allocation of 7 columns × 3 rows or minor allocation of 4 columns × 2 rows, at most 28k × 27k px (at most 196 × 189 px for `k` ≤ 7), so the fragment shader's Scale-to-Fit downscales it. Logos are the single permitted resampled content: they use the logo atlas's own filtered sampler, separate from the glyph atlas sampler and bind group, without mipmaps under the existing 8 px padding rule. The logo atlas remains a coverage mask (§3.2). Each logo cell uses its component-assigned `Text` ink, or `Action` ink only for an explicitly actionable icon, blended by coverage over the selected background (§3.6); with Color unchecked this resolves to the exact current monochrome foreground/background pair. The colour lookup does not change the single batched UI draw call, the separate logo sampler, the allocations or the centering geometry. ASCII and Unicode text glyph cells always use the glyph atlas's nearest-neighbour sampler in the same frame. Filtered sampling is permitted for logo textures only. At small `k` aliasing is accepted; at `k = 2` the minor logo (32 × 32 px drawn from 240 × 240) is minified about 7.5:1 by a bilinear sampler without mipmaps, so thin strokes may drop out or shimmer. This must be re-judged visually by the first milestone that renders logos (ADR 0003, *Open compatibility points*).
- **Shader Rule:** The wgpu Fragment Shader MUST use "Scale to Fit" logic. It receives the allocated grid area and the 256x256 texture UVs, and centers the square texture within the rectangular bounds perfectly, leaving transparent gaps. The Executor (CPU logic) must NEVER calculate sub-pixel offsets.

### 3.4 UI State & Hit-Testing Rules
- **No DOM / Retained Widget Trees:** The rendered UI representation produced
  by `newell-ui` MUST be a flat 1D array of cells.

  High-level Controller state MAY be stored in plain Newell-owned structs and
  enums for command state, focus, selection, tree expansion, panel state,
  overlays, and similar interaction data.

  Such state MUST NOT form a retained DOM-style widget tree and MUST NOT be
  used for bounding-box iteration during normal hit-testing.
- **O(1) Hit-Testing:** Pointer interaction MUST be calculated via direct mathematical division (pixels to grid coordinates), using the same W and H, in the same physical-pixel space, as rendering (§3.1). Iterating through bounding boxes for hit-testing is strictly FORBIDDEN.
- **Sub-Cell Modulo Logic:** For shared ASCII separator characters, the Controller MUST compute the modulo of the pixel coordinate to determine which half of the cell was clicked, routing the interaction to the correct adjacent `widget_id`.
- **Single GPU Draw Call:** `newell-render` MUST consume this flat array and render the entire ASCII UI overlay in a single `wgpu` draw call to maintain the lowest possible rendering overhead.

### 3.5 Interaction Model (ASCII Hover Highlight)

> **[APPROVED ARCHITECTURAL CHANGE — M2.16; M2.25 EXCEPTION]** M2.16
> removed the previous hover implementation, including colour-based
> highlighting, brightness changes and GPU effects. Its absence in code is
> NOT a regression. M2.25 explicitly authorizes hover exclusively through
> instantaneous Unicode border/tree-character substitutions, including the
> human-authorized 2026-10-26 tree amendment below,
> without colour changes, animations or GPU effects. Any hover implementation
> outside these documented exceptions remains an architectural violation.

- **Pointer routing (unchanged):** The Controller resolves pointer position to a grid cell and `widget_id` via O(1) division (`mouse_x / cell_width`), including Sub-Cell Modulo Logic (§3.4). This applies to click, press, release, drag and scroll. No bounding-box iteration is allowed.
- **Retained interaction mechanisms:** Hit-testing, clicking, dragging, keyboard focus and navigation, selection, pressed state during a click/drag, disabled state, expansion/checked state and all other non-hover interactions remain fully in force.
### ASCII Hover Highlight (M2.25)

M2.25 authorizes discrete, text-only hover highlighting. The human-authorized
2026-10-26 tree amendment below extends it to Features and Elements rows.

The approved Action Box extension includes all parameter and checkbox rows,
including cyclic selectors, and every header segment (Solid, Surface, New,
Add, Remove and Boolean operations). Header segments use their own two
boundaries like OK/Cancel: `│` or `┆` becomes `╞` on the left and `╡` on the
right, without changing the active-segment horizontal topology.
Parameter labels, padding and cyclic value text may inherit an existing
region id for hover only through at most 31 cell reads in their Action Box
row. Click routing and region-specific WidgetIds remain unchanged. This
fixed-width lookup preserves O(1) resolution and introduces no stored state.

**Tree interaction amendment (2026-10-26, explicitly authorized by the human).**
`docs/ui_design/README_UI.md` §26.1–§26.5 defines the exact tree structures.
Features alone owns the playhead; collapse changes its presentation, never its
logical insertion anchor. Tree row hover uses `├→╞`, leaf `╰╴→╘ ` (branch
`╰→╘` beside its unchanged handle), and right frame `║→╣` (Color on:
`│→╡`). Selection uses row-local `│→┃`, `├→┣`, `╰→┗`, `╴→╸`.
Selection wins at hierarchy cells when also hovered; hover still substitutes
the right frame. These are font glyph substitutions at unchanged cell geometry,
not increased raster stroke width, selection colour, inversion or animation.
Root rows have no hierarchy slots to replace. Focus alone has no visual.
Tree hover normalizes the hit row's part to its row WidgetId in O(1); original
click/drag ownership remains intact. Hierarchy/playhead ink remains Text,
frame ink remains Contour and expansion/visibility controls remain Action.
This tree-specific amendment supersedes the older tree exclusion and the
`╰╴→╘ ` wording of ADR 0006 only for trees; Action Box behavior and all
palette decisions remain unchanged. No new selection semantics are authorized.

**Controller**
- The Controller MAY store `hovered: Option<WidgetId>`.
- Update it through pointer movement and pointer exit events.
- Resolve targets through existing O(1) CellGrid hit-testing.
- At most one WidgetId may be hovered.
- Request a redraw only when the hovered target changes.
- Clear hover when the pointer leaves or the target disappears.

**Visual representation**
- Hover changes only documented Unicode border and tree hierarchy characters.
- Hover never changes a cell's semantic ink role or palette entry (§3.6); no brightness changes, fades or transitions. Every replacement glyph inherits the role of its replaced cell.
- No hover animations or shader effects.
- No changes to layout dimensions, spacing or text content.
- Restore the current non-hovered state immediately on exit (selected tree
  glyphs if selected, otherwise normal glyphs).

**Canonical transformations**

Menu row:
`│ Bridging Curve  │`
→ `╞ Bridging Curve  ╡`

Parameter:
`│ │ Along     [Select…       ]│`
→ `│ ╞ Along     [Select…       ]╡`

Checkbox:
`│ ╰╴□ End Offset              │`
→ `│ ╘ □ End Offset              ╡`

Button:
`│      OK      │    Cancel    │`
→ `╞      OK      ╡    Cancel    │`

**Interaction precedence**
- Apply the existing pointer and overlay precedence rules.
- Active window resizing suppresses hover updates.
- Only the topmost eligible interactive element may be hovered.
- Open menus and modals prevent hover on obscured elements.
- Disabled and non-interactive elements never receive hover.
- Hover never changes click, selection or focus semantics.

**Architectural constraints**
- Preserve MVC separation.
- Preserve the flat CellGrid.
- Preserve O(1) hit-testing.
- Preserve exactly one batched GPU UI draw call.
- No retained widget tree.
- No bounding-box scans.
- No additional rendering passes.

The M2.24 StatusBar progress animation remains the only
authorized animation. M2.25 hover is instantaneous and
does not constitute an animation.

No other hover visual effects are authorized.
- **State visuals:** The Controller keeps logical focus, selection, pressed, disabled, checked and expanded state (they drive keyboard navigation, activation and drag), but a state is drawn only through an ASCII structure that `docs/ui_design/` already documents: checkbox glyphs `□`/`■`, tree markers `[-]`/`[+]`, the cyclic-selector value text between `◀ ▶`, the segmented-selector border topology, the `+|`/`-|` prefix and the other glyphs defined in `docs/ui_design/README_UI.md`. Where no documented ASCII structure exists for a state, that state has **no visual**. The approved additions include the tree selection glyph structures of §26 in `docs/ui_design/README_UI.md` and the text-editing caret `▏` (U+258F), an ASCII structure now documented in `docs/ui_design/README_UI.md` §5.11 and `Special_ASCII.txt` (M2.23, see below). The following are forbidden: generic foreground/background inversion of a widget or run of cells; invented focus or selection markers (for example `>` or `<`); underlines, brightness, or state-dependent colour, intensity or alpha levels. The fixed semantic palette of §3.6 is permitted when View > Color is checked; roles are assigned by structural/control semantics, never by focus, selection, press, disabled state, hover or glyph identity. Widget-level inversion remains forbidden; palette selection is global to the window. Adding a visual for a state requires first adding its exact ASCII structure to `docs/ui_design/` and `docs/ui_design/README_UI.md`.
- **Selected viewport entity brightness (M4.3 exception):** Geometry belonging to a selected entity is rendered with its existing RGB multiplied by `1.10` after normal lighting, clamped to `1.0` before framebuffer encoding. Deselecting restores the unmodified output. Apply the multiplier once to that entity's existing surface, line, point and hidden-line fragments in every viewport display mode. The effect is independent of Theme and View > Color; alpha, background, depth, clipping and occlusion are unchanged. The Controller supplies only the derived selected boolean; the renderer performs no picking or selection mutation. This exception authorizes no selection outline, animation or extra rendering pass or draw call. It does not change UI ink, palette, hover, focus, or any other state visual. All other brightness and state-dependent UI color/intensity changes remain forbidden. This exception is limited to Newell's viewport geometry; it does not enable general brightness variation across the UI.
- **Text-editing caret (approved for M2.23).** The character `▏` is the caret of a text field that is **actively being edited**, and only then. It uses the `Text` ink role (§3.6) as an ordinary cell of the flat cell buffer, so it needs **no second draw call**, no caret-specific shader data and no hover or focus mechanism. It does not blink (no animation). It is composed from the edit state and never stored in, or copied to, the edited text value. It occupies its own cell between characters, shifting the following text one cell to the right; the text area keeps its fixed width, so when the field is full the visible window scrolls so that the caret stays visible and cells beyond the area boundary are clipped whole-cell, without any marker. The exact rule (caret index, window start `s = max(0, c + 1 − N)`, suffix clipping, delimiters) is `docs/ui_design/README_UI.md` §5.11. This is not a focus marker: focused-but-not-editing fields still have no visual.
- **Temporary accepted deviation (until M2.21).** The segmented-selector active-segment `║` separators built in M2.7 are not a documented structure. They may remain, unmodified and not extended to any new widget, until M2.21 replaces them with the exact canonical border topology of `docs/ui_design/`. M2.16 does not need to remove them. After M2.21 their presence is a violation of this section.
- **Animation (M2.24 exception):**

  The StatusBar progress indicator is the only authorized UI
  animation. All decorative, hover, fade and shader animations
  remain prohibited.

**Progress segment geometry**
- The clickable Progress segment occupies exactly 21 cells.
- Its width remains constant across all 21 progress states.
- The percentage field is preceded by exactly one space after `]`.
- The percentage field occupies 4 cells, left-aligned:
  `0%  `, `5%  `, `10% `, `100%`.
- Trailing padding belongs to the clickable segment.
- The entire segment, including its padding, uses one stable WidgetId.
- Separators outside the segment are not clickable.
- Preserve the exact ASCII reference layout and border positions.
- No variable-width layout or shifting of adjacent components.

  **Interaction**
  - Activate on left-button press, once per press.
  - Initial state: 0%.
  - At 0%, clicking starts a 5-second animation to 100%.
  - At 100%, clicking starts a 1-second animation to 0%.
  - Ignore clicks during animation without queuing or restarting.
  - Animation activity is determined by controller state,
    not by the displayed percentage.
  - Existing input precedence applies: active resizing and
    modal overlays take priority.

  **Timing**
  - Use monotonic elapsed time, never frame counts.
  - Display exactly 21 states in increments of 5%.
  - Ascending values use floor quantization.
  - Descending values use ceil quantization.
  - Final values are reached at their exact deadlines.
  - Update only ordinary text cells in the flat CellGrid.

  **Rendering**
  - FPS is measured from actual rendered frames and its
    displayed value is refreshed independently at 5 Hz.
  - Use ControlFlow::WaitUntil with the earliest deadline:
    min(next FPS tick, next progress step).
  - While progress is idle, schedule no progress-related redraws.
  - The continuous 60 FPS M1.3 bootstrap redraw loop is retired
    at M2.24; rendering remains event-driven.
- Preserve O(1) hit-testing, exactly one GPU UI draw call,
  and the absence of retained widget trees, colour-based hover,
  hover animations or shader-based hover effects.
  M2.25 ASCII border substitutions remain explicitly authorized.

  **Overlay input precedence**
- Active window resizing has priority over Progress interactions.
- Open menus and modal overlays intercept pointer presses
  before the underlying StatusBar.
- An outside press may dismiss an overlay according to its
  existing dismissal rules, but MUST NOT activate Progress
  during the same press.
- A subsequent independent press is required to activate Progress.
- No click-through or deferred activation is permitted.

### 3.6 UI Palette, Color and Themes

The narrowly scoped input-placeholder exception in §3.6.1 applies to this
section and the state-visual restrictions of §3.5. All ordinary ink, hover,
selection and frame rules below remain unchanged.

Approved by the human on 2026-10-26 through [ADR 0006](docs/adr/0006-view-color-semantic-palette.md).

The ASCII UI has two independent per-window choices: Light/Dark Theme and View > Color (initially unchecked). Every new window starts Dark with Color unchecked. An unchecked window is strictly monochrome: Dark uses background `#000000` and ink `#FFFFFF`; Light uses background `#FFFFFF` and ink `#000000`. Every composed glyph cell carries one of three semantic ink roles, which affect output only when Color is checked: `Text` for ordinary text, values, labels, caret, progress, menu popup/dialog/selection-panel title letters and treeview hierarchy strokes; `Contour` for box/panel/menu/dialog/selection-panel/field frames and shared structural joins; `Action` for explicit actionable affordances such as checkbox glyphs, tree `[-]`/`[+]` handles, selector arrows, selection-field controls, direction toggle and formula control. A logo/icon cell uses `Text` unless its component explicitly designates it as an `Action` affordance (§§3.2–3.3). Dark + Color uses background `#071521`, Text `#FFFFFF`, Contour `#41B4E5`, Action `#FFA24A`. Light + Color uses background `#FFFFFF`, Text `#111827`, Contour `#0969A0`, Action `#9A4D00`. Roles are assigned by the component's semantics, never inferred from a Unicode glyph or a WidgetId. For menu popup, dialog and selection-panel title rows, the title letters are Text while adjacent top-edge strokes, corners, frame junctions and structural separators are Contour. The View Theme row keeps exactly `Theme [ ◑ Dark  ]` or `Theme [ ◐ Light ]`: `Theme` and `Dark`/`Light` are Text, its `[`/`]` and `◑`/`◐` are Action, its spaces have no visible ink, and the popup's outer `│` borders are Contour. This role allocation does not change the Theme row's cell positions, ownership or activation. A shared cell takes the role of its visible structural stroke; frame intersections and segment dividers are Contour even when routing is split, while adjacent tree guides stay Text. Overlay composition overwrites glyph and role together.

With Color checked, only Contour cells use the same-cell glyph map `═→─`, `║→│`, `╔→┌`, `╗→┐`, `╚→└`, `╝→┘`, `╠→├`, `╣→┤`, `╦→┬`, `╩→┴`, `╬→┼`. Single-stroke and rounded corners, dashed separators, tree strokes and geometry do not change. The approved hover substitutions `│→╞/╡` and `╰╴→╘ ` remain and preserve the replaced cell's semantic role; together with the tree-specific substitutions of §3.5, they are the only stroke-form exceptions while hovered. Tree selection uses the documented heavy hierarchy glyphs without changing semantic roles; the same-cell Contour map must preserve the tree hovered right border `╣` rather than converting it to `┤` (compose it as `╡` when Color is checked). Hover adds no colour or shader effect. Focus, selection, press, disabled, checked and expanded states cannot alter a role or palette entry. Theme and Color changes select a new palette for that window; a Color change also recomposes the glyph grid. A Theme change with View open changes the displayed Theme row and uploads its recomposed cells; palette-only updating is permitted only when the glyph grid is unchanged. The glyph and logo atlases remain coverage masks and are not regenerated by either choice; text glyph sampling remains nearest and logo sampling remains filtered under §§3.2–3.3. The viewport clear follows the selected background; model display styles are unchanged apart from the selected-entity brightness exception in §3.5. The renderer uses one role word per cell, one palette per window and exactly one batched GPU UI draw call. O(1) hit testing and MVC separation remain binding. No gradients, tints, glow or decorative UI colour are permitted. UI ink/brightness and state-dependent UI colour remain forbidden; selected-entity viewport brightness follows §3.5. Glyph-edge antialiasing remains coverage blending.

| Theme / Color | Background | Text, values, labels, tree guides | Contour | Action |
|---|---|---|---|---|
| Dark / off | `#000000` | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` |
| Light / off | `#FFFFFF` | `#000000` | `#000000` | `#000000` |
| Dark / on | `#071521` | `#FFFFFF` | `#41B4E5` | `#FFA24A` |
| Light / on | `#FFFFFF` | `#111827` | `#0969A0` | `#9A4D00` |

At a cell where a tree hierarchy guide actually joins a frame, the frame/junction owns that one cell as `Contour`; the guide's next nonshared cell stays `Text`. A divider between controls remains one `Contour` cell even when `SharedBorder` routes its halves to different controls. Interaction ownership never determines ink. Scrollbar arrows and draggable thumbs, plus explicit selection-list control glyphs, are Action; inert annotations remain Text. Tree playhead strokes remain Text despite their draggable owner. For the Action Box two-cell hover replacement `╰╴→╘ `, `╘` inherits the former `╰` role and the blank retains the former `╴` metadata.

Theme activation by pointer or Enter/Space retains its existing open-popup and focused-row behavior. Color uses the established View checkbox behavior: toggle, keep the popup open and keep its row focused. Both choices are independent of OS appearance and other windows.

### 3.6.1 Scoped input-placeholder exception (2026-10-27)

The human explicitly authorized this Architect-stage amendment on 2026-10-27.
For initial numeric magnitude placeholders and the `none` placeholder of the
Validation Operation Box (Enter Name), this section supersedes the blanket
brightness, state-visual, three-role and exact-black/white-only restrictions in
§§3.5–3.6 and the corresponding restrictions in accepted ADR 0003 D7/D8 and
ADR 0006. Those ADRs remain historical records; all their other rules apply.

Add one semantic ink role, `Placeholder`, reserved exclusively for those
placeholder characters. Its source sRGB components are
`P = floor((4*T + B)/5 + 0.5)`, independently per 8-bit channel, where T is the
current Text ink and B the current background. Exact full-coverage inks are:

| Theme / Color | Placeholder |
|---|---|
| Dark / off | `#CCCCCC` |
| Light / off | `#333333` |
| Dark / on | `#CDD0D3` |
| Light / on | `#414652` |

This fixed 80% Text / 20% background mixture is computed in source sRGB, before
existing target-format conversion and glyph coverage blending. It changes ink,
never glyph alpha. Color off permits these neutral greys for placeholders only;
ordinary roles retain exact black/white. Color on retains the existing three
ordinary inks. The caret always uses normal Text ink. No hover, selection,
frame, unit, direction, Formula control, title, button, diagnostic, ordinary
label or accepted value receives Placeholder ink. The Formula `Fx = ` prefix
and selection-field prompts are outside this exception.

Placeholder identity is explicit Controller-owned metadata, never inferred from
zero, rounded text, an empty formula, a label or a glyph. It is not document
content. Clearing on entry to editing only creates an empty local raw editor;
it cannot mutate a quantity, discard a binding/expression, publish a command or
mark the document dirty. The complete lifecycle is canonical in
`docs/ui_design/README_UI.md` §5.13 and §23. App-owned accepted numeric drafts,
precision, input certificates, deferred Formula opening and commit-before-close
remain binding. Component composition supplies the new ink through the existing
per-cell role word and per-window palette in the same single ASCII draw call.
No new atlas, animation, pass, hit target or general disabled/focus dimming is
authorized. Overlay replacement copies glyph and role together.

### 3.7 Scoped 3D navigation-widget exception (2026-10-27)

The human's 2026-10-27 Architect request authorizes this architectural/UI
specification amendment for exactly the axis gizmo and navigation cube.
Implementation still requires an explicit implementation request for the plan.
Within these two viewport widgets only, this section takes precedence over
blanket ASCII-only, monochrome, gradient, font-sampling, subpixel-position,
no-extra-pass and no-bounding-box-iteration language in §§3.1–3.6 and §1.
Accepted ADRs 0002 D6, 0003 D5/D7/D11 and 0006 continue to govern the ASCII
interface; their earlier exclusion of additional widget rendering is superseded
only for this scope. The decision is recorded in new ADR 0007; prior ADRs are
preserved as historical records.

- These are window-local navigation graphics, never document entities,
  construction planes, model selection targets or BrepKIT geometry.
- The origin marker, positive-only dashed reference axes and positive-quadrant
  XY/XZ/YZ reference planes are renderer-owned graphics anchored at world origin
  `(0, 0, 0)`. `newell-render::Scene` derives a uniform world scale from the main
  camera projection and origin depth for fixed apparent size, retaining orbit
  foreshortening, ordinary pan, scene depth, viewport clipping and UI occlusion.
  Reference fragments keep their existing opacity where unobstructed and use
  20% of it where sampled scene depth shows model-surface occlusion, including
  in Points, Wireframe, Line and HiddenLine display modes. This depth-aware
  blend uses an isolated model-surface depth target, does not modify scene
  depth, and does not change model transparency.
  Invalid/behind-camera/out-of-depth-range origins are omitted, never clamped.
  Their explicit pixel dimensions, plane insets and neutral plane material, as
  well as axis colors, are specified in README_UI.md §41.2. They are excluded
  from document geometry, picking, history
  and model-fitting bounds. The live Features presentation adds a built-in
  Origin folder with seven immutable semantic reference rows; these are separate
  from operation IDs and Elements, and have no insertion, modeling, selection,
  visibility or rename actions. Existing folder expansion infrastructure is used. The corner Gizmo remains screen-space and retains only its
  directional stems, caps, rings and labels; it does not duplicate the origin
  marker or reference planes.
- `newell-app` retains sole mutable scene-camera and input authority.
  `newell-ui` may provide pure fixed geometry/projection/hit mathematics and
  layout values; `newell-render` consumes immutable derived data only.
- The exact visual/interaction contract is `docs/ui_design/README_UI.md` §41.
  It permits fixed corner-axis colors, matte and translucent shading,
  restrained spatial line fading, beveled cube surfaces, face-local cube labels,
  antialiasing and a separate filtered label coverage atlas using the existing
  font. The focused Executor request of 2026-10-27 additionally authorizes only
  the corner marker hover states in README_UI.md §41: negative ring opacity
  0.55→1.0 and saturation 0.8→1.0; positive disc transparent interior with
  axis-colored outline and letter. The 2026-10-28 quick fix increases corner
  marker diameters 15%, retaining outline widths and world-reference geometry.
  Hover is immediate Controller-owned semantic state, cleared under captures
  and input blocking, never renderer picking. No other hover, pressed or
  selection material changes, animation, extra gestures, refraction or blur
  are authorized. These corner-marker hover states do not apply to world-origin
  references; ASCII hover is unchanged.
- Fixed bounded tests of the cube's 26 convex components and six axis caps are
  permitted after ordinary O(1) ASCII routing. No retained widget hierarchy,
  variable-size widget scan or GPU picking/readback is introduced.
- Dedicated navigation rendering may use a separate local color/depth target
  and composite pass. It never uses scene depth, display-mode pipelines,
  selected-entity brightness or ASCII palette inks. The existing ASCII overlay
  remains exactly one batched draw call. Separate widget draw work must be
  counted and reported explicitly.
- Menus, modals, Action Boxes and active resize/capture keep their precedence.
  Widget render targets clear to transparent and composite only their graphics;
  their rectangular allocation never masks scene rendering, bounds or ASCII
  cells. Only actual semantic targets own input: complete circles for all axis
  markers (including hollow centers), and the existing 26 cube components.
  Empty tile space passes input to the scene. A widget press cannot select
  geometry or start an orbit. Existing scene captures retain ownership. View > View Navigator controls
  visibility; when checked it reveals a child Gizmo/Cube selector and displays
  exactly the selected widget. Each window starts checked with Gizmo selected;
  hiding retains that choice. Appearance remains independent of View > Color.

## 4. Multi-Window & State Management Architecture
To support unlimited, synchronized windows, the application architecture must strictly enforce the MVC pattern and the separation of crates.

### 4.1 Global Document State (The Model)
- **Centralized Source of Truth:** All topological data and feature trees live in the `newell-model` and `newell-kernel` crates. This global state has NO knowledge of the UI or the GPU.
- **Event-Driven Synchronization:** Implement a Pub/Sub pattern. When a window mutates the global model, the state manager dispatches an event. All other windows subscribed to the modified entities must intercept this event and flag their local view for a redraw.
- **Geometry Workers:** Feature tree recomputations and booleans run in the background (e.g., via a thread pool) to ensure the main thread never waits on geometry.

### 4.2 Local Window State (The View/Controller)
- **Isolated UI Context:** The Controller (`newell-ui`, `newell-app`) handles OS events via `winit` per window. There is no global UI singleton. Each window instance exclusively owns its active Workbench, selection state, Feature Tree expansion state, UI Scale choice (`ui_scale_label_percent`, mapped `scale_factor_percent`, `monitor_reference_scale` and `effective_k`, §3.1.1) and Theme plus Color (§3.6). Each new window initializes the 100% UI Scale label (factor 80%), Dark and Color unchecked independently of other windows and OS appearance. The selected label is user state; the multiplier is mapped from it, and the remaining scale values are derived from the window's current monitor and recomputed, never inherited.
- **Projection:** Each new window starts with Orthographic selected. Perspective remains available through the View selector; projection choice is local to the window and is not inherited.
- **Display style:** Each new window starts with Hidden Line selected. The seven display styles remain available through the View selector, and the choice is local to the window.
- **Render Context:** The View (`newell-render`) reads states and draws them independently for each window. Each window owns its independent swapchain and command encoder to render the 3D viewport and the ASCII grid overlay.

### 4.4 3D viewport navigation

Viewport navigation is mutable per-window Controller state in `newell-app`.
`newell-render` consumes immutable camera matrices and uploads them to WGPU;
it owns no gesture, pivot selection, CPU ray queries or camera mutation.
The approved implementation contract is [M3 camera mechanics](docs/plans/M3-camera-mechanics.md).

Primary-button drag over exposed viewport pixels performs camera-relative
yaw/pitch with strict zero roll against
world Z-up. Level the yaw result before applying pitch; clamp signed elevation
before constructing the pitch rotation, to 0.5 degrees short of either pole.
Exact principal-axis views retain a deterministic heading for departure.
Quaternions and an orthonormal camera basis avoid accumulated Euler state.
Select a contextual pivot once on press: nearest displayed mesh hit, otherwise
the world AABB centre of geometry clipped to the frustum and exposed viewport
regions. Rotate both eye offset and orientation by the same final quaternion,
preserving the pivot's projected pixel without recentering at gesture start.

Secondary-button drag pans in the camera-facing plane at contextual focal depth;
middle-button drag remains a pan fallback. Primary and secondary clicks capture
navigation only after the existing click/drag threshold. A button-free scroll
stream pans in the screen plane when it is `PixelDelta` and begins with
`TouchPhase::Started` over exposed viewport pixels; its frozen anchor uses the
same contextual focal depth as mouse pan. The `winit` deltas are already in
physical pixels and retain the OS scroll direction, so the Controller applies
no second DPI conversion. `LineDelta` and `PixelDelta` without a captured
`Started` phase retain cursor-anchored wheel zoom; native pinch zoom is unchanged.
Menus, modals, trees and selection lists receive scroll first. `winit` does not
report finger count, so routing uses the phased `PixelDelta` event pattern as a
proxy: a phased high-resolution mouse wheel may pan, while an unphased trackpad
scroll remains zoom. Phased gestures retain their anchor and input owner.
The absolute minimum navigation distance is 0.001 model units; maximum distance
is four times the aspect-aware sphere-fit distance with a 10% margin. Mesh
replacement updates cached bounds without reframing. Home over exposed scene
pixels fits the whole object in an unobstructed region, unless text is edited.

Only exposed scene pixels may admit gestures after O(1) UI hit testing and
menu/modal routing. Geometric queries are CPU mesh queries, separately O(T)
for T triangles; they require no GPU readback. Action Boxes only occlude the
scene. Resizing preserves physical-pixel scale and projection position using
lens shift. Captured drags accept positions outside the viewport while inside
the window, and only their initiating button ends them. Focus loss, pointer
exit, layout/occlusion, mesh and projection changes cancel capture without
moving the camera. Rendering stays event-driven with no inertia loop.

### 4.3 Window Manager (Lifecycle)
- The `newell-app` binary crate is responsible for spawning, tracking, and destroying window instances and wiring the event loop.
- Closing a window drops only its local UI state and render context, leaving the global data intact.

## 5. Geometric Kernel (BrepKIT) Rules
- **Exclusivity:** BrepKIT is the ONLY geometric kernel allowed. No FFI bindings to OpenCASCADE or other engines.
- **Safety:** The `unsafe` keyword is strictly FORBIDDEN in Newell's codebase.
- **Error Handling:** No `unwrap()`, `expect()`, or `panic!()` in production code. All geometric operations MUST return a `Result<T, NewellError>`.

### 5.1 Geometric Kernel Boundary (Anti-Corruption Layer)
The integration of the `brepkit` kernel must strictly follow the Anti-Corruption Layer (ACL) pattern to immunize the broader application against upstream API breaks or topological changes.

- **Strict Isolation:** BrepKIT crates (`brepkit-operations`, `brepkit-topology` and their transitive packages) MUST ONLY be declared as dependencies in `crates/newell-kernel/Cargo.toml`. No other crate (`newell-model`, `newell-ui`, etc.) is allowed to import or know about BrepKIT.
- **No Type Leaking (Wrapping):** `brepkit` specific types (e.g., topology structures, faces, edges) MUST NEVER appear in the public API signatures of `newell-kernel`. They must be encapsulated within Newell-owned wrapper structs (e.g., `NewellSolid`, `NewellFace`).
- **Error Translation:** `newell-kernel` MUST intercept all native `brepkit` errors and map them to a domain-specific `NewellKernelError` enum using the `thiserror` crate.
- **Version Pinning:** Each direct BrepKIT dependency MUST be strictly pinned to an exact version in `Cargo.toml` (for example, `brepkit-operations = "=x.y.z"`). Implicit minor/patch updates are forbidden. The selected M3.1 version is recorded in ADR 0004.
- **Golden File Validation:** Kernel updates must be validated against "Golden Files" (reference 3D models stored in `tests/fixtures/`). Any update to `brepkit` requires running geometric regression tests to ensure boolean and meshing algorithms remain mathematically consistent with the reference files.

## 6. Licensing Standard
Newell is licensed under the GNU Affero General Public License, version 3 or (at your option) any later version (**SPDX: `AGPL-3.0-or-later`**). The `LICENSE` file contains the full text of AGPL version 3; the project-specific notice below expressly permits later versions.

- **Cargo metadata:** Each applicable `Cargo.toml` package license declaration MUST use `license = "AGPL-3.0-or-later"`.
- **Changes to licensing:** Agents MUST NOT change the project's licensing terms without explicit human approval. Third-party dependency licenses MUST be checked independently.

**Auditor Rule:** Every `.rs` file MUST begin with the following header:

```rust
// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
```
