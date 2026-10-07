# Newell

**Newell** is a next-generation parametric CAD application.
It aims for extreme performance and a minimal memory footprint. Its interface follows a "TUI / Hacker" aesthetic and takes advantage of modern hardware acceleration.
Development follows a structured agentic (AI) workflow, under the strict supervision of a human designer.

> **For AI agents reading this file:** this document is the project's vision and reference. Binding rules live in `ARCHITECTURE.md`; the entry point for Claude Code is `CLAUDE.md`. When this file and `ARCHITECTURE.md` disagree, `ARCHITECTURE.md` wins and the conflict must be reported to the human.

---

## 1. Vision and goals

### Goals

- A parametric CAD tool that stays responsive on any machine, from a workstation down to a Raspberry Pi.
- Every byte of RAM and every CPU cycle goes to geometry, not to the UI framework.
- A keyboard-first, grid-based interface that is fast to learn for technical users.
- A codebase that can be safely produced by AI agents, because every rule is machine-checkable.

### Non-goals (for now)

- Rendering photorealistic images (no ray tracing, no PBR materials).
- Assemblies, simulation (FEA/CFD), CAM toolpaths.
- Real-time collaboration and cloud features.
- Running in a web browser (possible later, since the kernel targets WebAssembly, but not a v1 goal).

Non-goals exist to keep scope under control. Agents must not implement them without an explicit human decision recorded in an ADR (see §6.4).

---

## 2. Technical constraints

Every constraint below must be **measurable**. A constraint that cannot be tested in CI is a wish, not a rule.

| Constraint | Target | How it is verified |
|---|---|---|
| Language | Rust, stable toolchain, pinned in `rust-toolchain.toml` | CI uses the pinned toolchain |
| Memory safety | `unsafe` forbidden in Newell's own crates | `unsafe_code = "forbid"` in workspace lints |
| No panics in production code | No `unwrap()`, `expect()`, `panic!()`, `todo!()` outside tests | Clippy lints set to `deny` (see §7.1) |
| Frame rate | ≥ 60 FPS (≤ 16.6 ms per frame) during navigation and UI interaction | Frame-time benchmark in CI, profiling with `tracy` or `puffin` |
| Memory | RAM budget per reference model, defined in `ARCHITECTURE.md` | Memory benchmark on reference models |
| Low-end target | Usable on Raspberry Pi 4/5 (aarch64) | aarch64 cross-build in CI, manual test on real hardware per release |
| Modeling | 100% parametric, history-based | Feature tree tests (§4) |
| Geometric kernel | **BrepKIT in Rust, exclusively**. No second, fallback or alternative B-Rep / solid-modeling kernel, including through FFI | CI checks the kernel dependency allow-list and crate boundaries; code review rejects any alternative kernel dependency |

### Clarifications

- **"No `unsafe`" applies to Newell's code only.** Dependencies such as `wgpu` use `unsafe` internally. That is accepted; it is audited by their maintainers, not by us.
- **60 FPS is a navigation and UI guarantee, not a geometry guarantee.** A complex boolean or fillet can take seconds. The rule is: geometry recomputation never blocks the render thread (see §3.3). The UI shows progress and stays interactive.
- **Raspberry Pi support depends on GPU drivers.** `wgpu` needs Vulkan (Mesa v3dv driver) or a GLES fallback on the Pi. This must be validated with an early spike before committing to the target.
- **"BrepKIT exclusively" applies to the B-Rep / solid-modeling kernel.** Math, tessellation-support or 2D constraint-solver crates may be used when justified, but they must not introduce a second B-Rep topology/geometry kernel or an FFI bridge to one.

---

## 3. Architecture

### 3.1 Cargo workspace layout

The project is a Cargo workspace with small crates and one-way dependencies. Smaller crates mean faster builds and give agents a narrow, well-defined context.

```
newell/
├── Cargo.toml              # workspace, shared lints and dependency versions
├── rust-toolchain.toml     # pinned toolchain
├── CLAUDE.md               # entry point for Claude Code
├── ARCHITECTURE.md         # binding rules (human-owned)
├── MEMORY.md               # agents' logbook
├── docs/
│   ├── adr/                # Architecture Decision Records
│   └── plans/              # Architect plans, one file per feature
├── crates/
│   ├── newell-kernel/      # sole BrepKIT integration boundary and Newell geometry API
│   ├── newell-model/       # document model: feature tree, parameters, undo/redo
│   ├── newell-sketch/      # 2D sketches and geometric constraint solver
│   ├── newell-io/          # native format + exchange orchestration via newell-kernel
│   ├── newell-ui/          # character grid, layout, hit testing, UI state (no GPU)
│   ├── newell-render/      # wgpu renderer: grid, glyphs, atlas, 3D viewport
│   └── newell-app/         # binary: window, event loop, wiring
├── assets/                 # fonts (.ttf), icon atlas (.png)
├── tests/fixtures/         # reference models and golden files
├── benches/                # performance benchmarks
└── .github/                # CI, templates, CODEOWNERS (see §8)
```

**Dependency rule:** `newell-ui` and `newell-model` must never depend on `newell-render` or `wgpu`. That is what keeps UI and model logic testable without a GPU.

### 3.2 BrepKIT kernel boundary

**BrepKIT, written in Rust, is Newell's one and only geometric kernel.** Newell must not integrate, wrap, call, or fall back to any other B-Rep / solid-modeling kernel, whether native Rust or through FFI.

All BrepKIT dependencies are confined to `newell-kernel`. Other Newell crates depend on Newell-owned APIs and data types exposed by `newell-kernel`; BrepKIT-specific types must not leak across that boundary. This boundary is **not** a pluggable-kernel abstraction and must not be designed to support interchangeable kernel implementations.

Why:
- BrepKIT is the single source of truth for B-Rep construction, topology and solid-modeling operations.
- `newell-kernel` isolates BrepKIT-specific APIs so the rest of Newell stays coherent and testable without duplicating kernel responsibilities.
- `newell-kernel` translates BrepKIT errors into Newell errors and enforces Newell-wide units, tolerances and topology conventions.
- BrepKIT is young and its robustness on booleans and fillets is not yet proven at scale. Robustness gaps must be handled with tests, defensive integration, reproducible bug cases and upstream fixes to BrepKIT — **never by introducing a fallback kernel**.

Exchange-format operations provided by BrepKIT (for example through `brepkit-io`) are also accessed only through `newell-kernel`. `newell-io` owns Newell's file-format orchestration and native document serialization, but it must not depend on BrepKIT crates directly.

### 3.3 Threading model

- **Main thread:** window events, input, UI state, render submission. Must never wait on geometry.
- **Geometry worker(s):** feature tree recomputation, booleans, tessellation. Run in the background (for example with `rayon` or a dedicated thread pool).
- **Communication:** message channels. The render thread shows the last valid tessellated result until a new one is ready.
- Long operations must be **cancellable**: when a parameter changes again, the outdated computation is dropped.

### 3.4 Error handling and logging

- Library crates: typed errors with `thiserror`. Every public operation returns `Result`.
- Binary crate: errors are shown to the user in the UI, never as a crash.
- Logging and tracing: `tracing` crate, with spans around expensive operations so profiling is possible.

---

## 4. Parametric modeling core

This is the heart of a CAD application and the part with the most technical risk. It must be designed before the UI is polished.

### 4.1 Feature tree (history)

- A document is an ordered list of features (Sketch, Extrude, Revolve, Fillet, Boolean…), each with parameters.
- Changing a parameter recomputes the tree from the first affected feature, not from scratch.
- Results of unchanged features are cached.
- Recomputation must be **deterministic**: same inputs, same output, on every platform.

### 4.2 Sketch constraint solver

- Sketches need a 2D geometric constraint solver (coincident, parallel, perpendicular, tangent, distance, angle, equal, fixed).
- The solver must report the sketch state: under-constrained, fully constrained, over-constrained or conflicting.
- This is a significant subsystem. Decide early, in an ADR, whether to write it or use an existing crate. An external 2D constraint solver is allowed only as a solver; it must not introduce another B-Rep / solid-modeling kernel or competing topological model.

### 4.3 Topological naming problem

**This is the top risk of any parametric CAD project.** When a parameter changes, faces and edges are recreated, and features that referenced them (for example "fillet edge #12") can break or pick the wrong edge.

- References to topology must be stable identifiers based on how the entity was created (which feature, which sketch element), not on indices.
- The strategy must be written in an ADR before implementing any feature that references existing faces or edges.
- Dedicated tests: modify an upstream parameter and check that downstream references still target the right entity.

### 4.4 Undo/redo

- Based on the feature tree: every user action is a command that can be undone.
- Undo/redo must not require recomputing the whole model when a cached result exists.

### 4.5 Units and tolerances

- Internal unit: millimetres. Display units are a user preference.
- Geometric tolerances (for coincidence, for example) are defined in one place and documented in `ARCHITECTURE.md`.

### 4.6 File formats

- **Native format:** stores the feature tree and parameters, not just the resulting geometry. It is text-based for readable diffs (for example RON or JSON), **versioned**, with migrations for older versions.
- **Exchange formats:** STEP export first (the industry standard), STL/3MF for 3D printing. BrepKIT's `brepkit-io` crate lists STEP, IGES, STL, 3MF, OBJ, PLY and glTF; each format must be validated with round-trip tests before it is advertised as supported. Calls into `brepkit-io` are made through `newell-kernel`; `newell-io` must not import BrepKIT crates directly.

---

## 5. Interface and graphics rendering

The interface uses neither the DOM nor complex graphical objects. It is a **character grid** (grid-based UI), fully rendered on the GPU.

- **Technology:** Rust + `wgpu` (low-level hardware acceleration), `winit` for windowing and input.
- **Layout:**
  - a 2D ASCII interface, stored in a character array (O(1) lookup for clicks and hover);
  - overlaid on a 3D viewport.

### 5.1 Hybrid typography

- Text and interface are generated dynamically from a `.ttf` file, for crisp vector text at any resolution.
- Tool logos (Extrude, Sketch, Line, etc.) are rendered from **a single RGBA `.png` texture atlas** (256x256, with 8 px internal padding).

### 5.2 Visual integration

- Multicolor logos occupy asymmetric blocks on the ASCII grid (e.g. 5x3 cells for a 3:5 ratio).
- A *fragment shader* centers them to the pixel.

### 5.3 Interactivity

- Hover effects are handled directly by the shader: hue change and side border.
- CPU cost is negligible: the CPU only does an O(1) grid lookup and updates one state value; no relayout and no redraw of the UI on the CPU.
- The UI is only redrawn when its state changes (dirty flag). The 3D viewport redraws only when the camera or the model changes.

### 5.4 Points the current design must address

- **HiDPI and scaling:** cell size must follow the OS scale factor.
- **Keyboard-first:** every tool must be reachable from the keyboard; a command palette is recommended.
- **Text input:** parameter fields need cursor handling, selection, copy/paste, and IME support for non-Latin input.
- **3D picking:** selecting faces, edges and vertices in the viewport needs its own mechanism (for example an ID buffer rendered on the GPU). The O(1) grid lookup only covers the 2D UI.
- **Accessibility:** a character grid is opaque to screen readers. At minimum, plan for high-contrast themes and configurable font size.

---

## 6. Agentic development workflow

Newell's code is generated, tested, and optimized by a "team" of 5 AI agents (Claude Opus 5.5, Sonnet 5.5 and Haiku 5.5). Each agent runs in an ephemeral conversational session: no hidden/session state is trusted across sessions. Persistent project context is carried only through reviewed repository files such as `CLAUDE.md`, `ARCHITECTURE.md`, ADRs, plans and the deliberately short `MEMORY.md`.

**Human role:** define the constraints, own the vision, approve plans, and merge pull requests. **Only the human merges into `main`.**

### 6.1 The 5 agents

| Agent | Model | Input | Output | Must not |
|---|---|---|---|---|
| **Architect** | Opus 5.5 | Issue + `ARCHITECTURE.md` + relevant code | Plan in `docs/plans/<issue>-<slug>.md`: traits, types, files to touch, test list | Write production code |
| **Challenger** | Sonnet 5.5 | The plan only (one-shot) | Written review: logical flaws, borrow checker conflicts, bottlenecks, missing tests | Rewrite the plan itself |
| **Executor** | Sonnet 5.5 | Approved plan + targeted files + docs (e.g. BrepKIT) | Code + tests on a branch, PR opened | Modify tests to make them pass, touch files outside the plan, edit `ARCHITECTURE.md` |
| **Auditor** | Sonnet 5.5 | Diff of the Executor's PR | Cleanup commits: duplication, idioms, Clippy, docs | Change behaviour (tests must stay identical and green) |
| **Explorer** | Haiku 5.5 | Questions from the human or another agent; read access to the repository | Answers and discussion, with `path:line` references; can offer opinions, labelled as such | Edit, create or delete files; run commands; take decisions that belong to the human |

The Explorer is a read-only helper, not a pipeline stage. It can be used at any point (for example, to locate code before planning, or to discuss options with the human), and it never appears in the approval chain of §6.2.

### 6.2 Pipeline

```
Issue (human)
  → Architect writes plan
  → Challenger reviews plan
  → Architect revises (max 2 rounds, then human decides)
  → Human approves plan
  → Executor implements on a branch, opens PR
  → CI green
  → Auditor cleans up on the same branch
  → CI green
  → Human reviews and merges
```

### 6.3 Guardrails

- **Tests come from the plan.** The Architect lists the tests; the Executor writes them first. A test change not listed in the plan is a red flag in review.
- **Iteration limit.** If the Executor fails to reach green CI after a fixed number of attempts (defined in `ARCHITECTURE.md`), it stops and writes a report instead of forcing a solution.
- **Narrow context.** Agents receive only the files they need. Small crates (§3.1) make that possible.
- **No silent scope creep.** Anything outside the plan goes into a new issue, not into the current PR.
- **Traceability.** Agent commits carry a `Co-Authored-By` trailer naming the model, and the PR description states which agent role produced it.

### 6.4 Context files

| File | Purpose | Who edits it |
|---|---|---|
| `CLAUDE.md` | Short entry point: build/test commands, where the rules are, what is forbidden | Human |
| `ARCHITECTURE.md` | Binding rules: crate boundaries, lints, budgets, tolerances, iteration limits | Human only (protected by CODEOWNERS) |
| `docs/adr/NNNN-title.md` | One decision per file: context, options, decision, consequences | Architect drafts, human approves |
| `docs/plans/` | One plan per feature, kept after merge for history | Architect |
| `MEMORY.md` | Logbook: lessons learned, pitfalls, current state | Agents append; human prunes |

**`MEMORY.md` must stay short.** An ever-growing logbook brings back exactly the context drift that ephemeral sessions are meant to avoid. Lasting lessons are promoted to `ARCHITECTURE.md` or an ADR; obsolete entries are deleted.

---

## 7. Code quality and testing

### 7.1 Layer 1: compilation, lints, formatting

- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` must pass.
- Lints are declared once, in the workspace `Cargo.toml`:

```toml
[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
dbg_macro = "deny"
```

- `clippy.toml` sets `allow-unwrap-in-tests = true` and `allow-expect-in-tests = true` so tests stay readable.

### 7.2 Layer 2: geometric validation (B-Rep)

- Unit tests on BrepKIT-backed kernel operations through the `newell-kernel` boundary: intersections, normal integrity, manifoldness, watertightness, volume.
- Nominal **and** degenerate cases (zero-length edges, coplanar faces, tangent surfaces, non-manifold input).
- **Property-based tests** (`proptest`): random parameters, check invariants (volume ≥ 0, closed shell, booleans consistent with volumes).
- **Golden files**: reference models in `tests/fixtures/`, exported and compared after each change.
- **Topological naming tests** (§4.3).

### 7.3 Layer 3: UI validation

- The UI is an abstract grid, so it is validated with state tests, with no GPU involved.
- Check mouse/grid hit testing, state transitions (e.g. `Normal` → `Hovered`), keyboard navigation.
- **Snapshot tests** (`insta`): render the grid to text and compare with the stored snapshot.

### 7.4 Layer 4: performance

- Benchmarks with `criterion` on reference operations (recompute, tessellation, grid update).
- CI compares against `main` and flags regressions above a threshold defined in `ARCHITECTURE.md`.

### 7.5 Layer 5: robustness

- Fuzzing with `cargo-fuzz` on file parsers (native format, STEP import). Parsers are the main crash surface.

### 7.6 Tooling

- `cargo-nextest` to run tests faster.
- `cargo-deny` for licences, security advisories and duplicate dependencies.
- Documentation: `cargo doc` without warnings; every public item has a doc comment.

---

## 8. GitHub management

### 8.1 Repository setup

- **Default branch:** `main`, always releasable.
- **Rulesets on `main`:**
  - pull request required, no direct push, no force push;
  - all CI checks required;
  - at least one approval from a code owner (the human);
  - all review conversations resolved before merge;
  - linear history.
- **Merge method:** squash merge only. One PR = one commit on `main` with a clean message.
- **Security:** enable secret scanning, push protection, Dependabot alerts, and private vulnerability reporting.

### 8.2 Branches

Trunk-based development with short-lived branches (merged within a few days):

```
feat/<issue>-<slug>      new feature
fix/<issue>-<slug>       bug fix
refactor/<issue>-<slug>  refactoring without behaviour change
docs/<issue>-<slug>      documentation
spike/<slug>             throwaway experiment, never merged
```

Every branch is tied to an issue. Agents work only on branches and never on `main`.

### 8.3 Commits

[Conventional Commits](https://www.conventionalcommits.org/) format:

```
feat(sketch): add tangent constraint
fix(kernel): handle coplanar faces in boolean union
perf(render): batch glyph draws into one call
```

Allowed types: `feat`, `fix`, `perf`, `refactor`, `test`, `docs`, `build`, `ci`, `chore`. With squash merge, the **PR title** must follow this format, since it becomes the commit message. A CI check can enforce it.

### 8.4 Issues and project board

- **Issue templates** in `.github/ISSUE_TEMPLATE/`: feature (goal, acceptance criteria), bug (steps, expected vs. actual, model file), ADR proposal.
- **Labels:** `type:*` (feature, bug, perf…), `area:*` (kernel, sketch, ui, render, io), `agent:*` (architect, challenger, executor, auditor, explorer), `priority:*`, `needs-human`.
- **GitHub Projects board** whose columns mirror the pipeline of §6.2: `Backlog → Planning → Challenged → Approved → In progress → Audit → Human review → Done`.
- Milestones match the roadmap (§10).

### 8.5 Pull requests

The template `.github/pull_request_template.md` requires:

- linked issue (`Closes #N`) and link to the plan in `docs/plans/`;
- agent role and model that produced the change;
- summary of the Challenger's points and how they were handled;
- tests added, with confirmation that existing tests were not modified (or why they were);
- performance impact (benchmark result if relevant);
- checklist: fmt, clippy, tests, docs, `MEMORY.md` updated if needed.

Keep PRs small: one feature or fix per PR. A large PR is hard to review for a human and a sign the plan should have been split.

### 8.6 CODEOWNERS

`.github/CODEOWNERS` requires the human's approval for sensitive files, so agents cannot change the rules they are judged by:

```
/ARCHITECTURE.md      @<owner>
/CLAUDE.md            @<owner>
/docs/adr/            @<owner>
/.github/             @<owner>
/Cargo.toml           @<owner>
/rust-toolchain.toml  @<owner>
/deny.toml            @<owner>
```

### 8.7 Continuous integration (GitHub Actions)

Workflow on every PR and on `main`:

| Job | Content |
|---|---|
| `fmt` | `cargo fmt --check` |
| `clippy` | `cargo clippy --all-targets --all-features -- -D warnings` |
| `test` | `cargo nextest run` on Linux, macOS, Windows |
| `aarch64` | cross-build for Raspberry Pi (`aarch64-unknown-linux-gnu`) |
| `msrv` | build with the minimum supported Rust version |
| `doc` | `cargo doc --no-deps` with warnings as errors |
| `deny` | `cargo deny check` (licences, advisories) |
| `bench` | benchmarks compared with `main` (on PRs labelled `perf` or touching hot paths) |
| `pr-title` | check that the PR title follows Conventional Commits |

Best practices for workflows:
- `permissions:` set to the minimum (read-only by default).
- Third-party actions pinned to a full commit SHA, not a tag.
- Cache with `Swatinem/rust-cache`.
- `concurrency` group to cancel outdated runs on the same branch.

### 8.8 Dependencies

- `Cargo.lock` is committed (Newell is an application).
- Dependabot weekly, for `cargo` and `github-actions`, grouped into one PR per ecosystem.
- Every new dependency is justified in the PR (why, licence, maintenance status). Agents may not add dependencies without it being listed in the approved plan.
- **Kernel dependency rule:** BrepKIT is the only approved B-Rep / solid-modeling kernel. Dependencies that provide or bridge to another kernel (including FFI bindings) are forbidden. CI validates the kernel-dependency allow-list; `newell-kernel` is the only Newell crate allowed to depend on BrepKIT crates.

### 8.9 Releases

- [Semantic Versioning](https://semver.org/). Stay in `0.x` until the native file format is stable.
- Changelog and version bumps generated from commits (`release-please` or `git-cliff`).
- Tags `vX.Y.Z` trigger a release workflow that builds binaries for Linux (x86_64, aarch64), macOS, and Windows, and attaches them with checksums to the GitHub Release (`cargo-dist` automates this).

### 8.10 Agent access

- Agents use a dedicated identity (a GitHub App or a fine-grained token) limited to this repository, with permissions to push branches and open PRs only.
- No agent has permission to merge, change rulesets, or edit secrets.

### 8.11 Community files

`README.md`, `LICENSE`, `CONTRIBUTING.md` (including how the agent workflow works), `SECURITY.md`, `CODE_OF_CONDUCT.md`.

---

## 9. Licensing

**This decision must be resolved in M0 before any BrepKIT-dependent production code is merged into `main` or any Newell binary is distributed.** Throwaway local spikes may be used to evaluate BrepKIT, but they are never merged or released before the licence decision.

- According to its crate documentation, BrepKIT is licensed **AGPL-3.0-only or under a commercial licence**.
- Using it under AGPL means Newell must be distributed under an AGPL-compatible licence, and its source must be available to users, including users of a network service.
- Because BrepKIT is mandatory and exclusive, Newell has only two project paths under this architecture: use BrepKIT under an AGPL-compatible licensing strategy, or obtain a BrepKIT commercial licence. **Replacing BrepKIT with another kernel is not an allowed mitigation.**
- If neither licensing path is acceptable, development of BrepKIT-dependent production code stops and the human must explicitly change the project's architectural requirements before work can continue.
- `cargo-deny` enforces the list of allowed licences for all dependencies.

---

## 10. Roadmap

Each milestone ends with a usable, tested state. Do not start a milestone before the previous one is merged.

| Milestone | Content |
|---|---|
| **M0: Spikes** | BrepKIT licence decision first; then BrepKIT robustness spike on booleans/fillets and wgpu spike on Raspberry Pi. No BrepKIT-dependent production code merges before the licence decision. Output: ADRs. |
| **M1: Skeleton** | Workspace, CI, rulesets, `CLAUDE.md`, `ARCHITECTURE.md`, window with empty grid at 60 FPS. |
| **M2: Grid UI** | Glyph rendering, icon atlas, hit testing, hover in shader, keyboard navigation. |
| **M3: Viewport** | 3D camera, display of a BrepKIT-generated solid through `newell-kernel`, picking. |
| **M4: Sketch** | 2D sketch, lines/arcs/circles, constraint solver. |
| **M5: Parametric core** | Feature tree, Extrude/Revolve, recompute, undo/redo, topological naming strategy. |
| **M6: Files** | Native format with versioning, STEP and STL export. |
| **M7: First release** | Fillet/Chamfer, booleans, performance pass, `v0.1.0`. |

---

## 11. Risks and open questions

| Risk | Impact | Mitigation |
|---|---|---|
| BrepKIT maturity is unproven (young project, robustness unknown) | Booleans and fillets may fail on real models | Spike in M0; focused regression/property tests in `newell-kernel`; minimized reproducible cases; upstream fixes/contributions to BrepKIT; no fallback kernel |
| BrepKIT AGPL/commercial licensing | Constrains Newell's licensing/distribution model; project cannot proceed with BrepKIT-dependent production code if neither path is acceptable | Decide first in M0 (§9); use an AGPL-compatible strategy or obtain the commercial licence |
| Topological naming | Parametric models break when edited | ADR before M5, dedicated tests |
| Constraint solver complexity | Major delay on M4 | Evaluate existing crates first |
| wgpu on Raspberry Pi | Low-end target unreachable | Spike in M0; if it fails, redefine the target |
| 60 FPS during heavy recompute | Freezes | Background workers, cancellation (§3.3) |
| AI agents drifting from the plan or gaming tests | Low-quality code passing CI | CODEOWNERS, test rules (§6.3), human review of every PR |
| Grid UI limits (HiDPI, text input, accessibility) | Unusable for some users | Addressed explicitly in §5.4 |