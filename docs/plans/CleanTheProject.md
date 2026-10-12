# Plan: Clean the Project Before M1 (CleanTheProject)

**Author:** Architect
**Revision:** 2 (decisions approved by the human on 2026-10-12; supersedes revision 1)
**Backlog items:** None. Pre-M1 hygiene pass. Unblocks M1.1 and M1.2.
**Status:** Approved decisions. Ready for Challenger review, then Executor.
**Binding:** `ARCHITECTURE.md` is the source of truth. The only exceptions are the decisions in §1, which the human approved explicitly.

---

## 0. Scope and Rules for this Plan

- **Documentation corrections** (§3) change `.md` files and `.github/**` only. They align specifications with `ARCHITECTURE.md`.
- **Code and config fixes** (§4) change `Cargo.toml`, `crates/**` stubs, toolchain and lint config. No production logic.
- **Out of scope:** any geometry, UI, or render logic; any new dependency not listed here; any refactor unrelated to consistency.
- **Geometric kernel:** BRepKIT remains the only geometric kernel. No OpenCASCADE, OCCT, or other kernel is introduced, referenced, or planned. `brepkit` appears only in `crates/newell-kernel/Cargo.toml` and is added in M3.1, not here.
- **No placeholders:** no empty trait, no stub module, no `TODO`. A crate stub contains only its AGPL header and the minimum needed to compile.
- **No Architect code:** this plan is the only file the Architect writes.

### 0.1 Current state (verified, 2026-10-12)

- `cargo` is not installed. Nothing has been built or linted.
- `crates/`, `tests/`, `assets/` do not exist. `Cargo.toml` lists five members that are absent.
- `HEAD` = `89cc92c`. Working tree clean. `docs/backlog.md` M1.1 is `[ ]` at `HEAD`.
- `LICENSE` is present (AGPL-3.0 text).

---

## 1. Approved Decisions

| ID | Decision | Applied in |
|---|---|---|
| D1 | Five-crate workspace (`newell-kernel`, `newell-model`, `newell-ui`, `newell-render`, `newell-app`). `newell-sketch` and `newell-io` are removed from `Description.md` §3.1 and are added later only through an `ARCHITECTURE.md` amendment. | DOC-6 |
| D2 | Four primary agents (Architect, Challenger, Executor, Auditor). Explorer is an **optional read-only helper**, not a pipeline stage. | DOC-3, DOC-6, DOC-13, DOC-15 |
| D3 | BRepKIT pin version: not chosen now. The exact version is pinned in M3.1 as `=x.y.z`, chosen from the 3.0+ line in ADR 0001. Not blocking this PR. | DOC-4 |
| D4 | Logo sprite grid: 5×3 (major), 3×2 (minor), per `ARCHITECTURE.md` §3.3. | DOC-8 |
| D5 | `--locked` and `--all-features` on every Cargo job in CI. | CI-2 |
| D6 | Single batched `wgpu` draw call for the ASCII overlay (`ARCHITECTURE.md` §3.4). **No four-pass design.** `ui-design.md` is rewritten to match. | DOC-9 |
| D7 | Copyright line: `Copyright (c) 2026 Martin Harambat`. Fix the unclosed code fence in `ARCHITECTURE.md` §6. Preserve the 6-line header content as written. Do not invent a licence. | DOC-1, DOC-2, CODE-2 |
| D8 | Commit `Cargo.lock`. | CODE-3 |
| D9 | Add `msrv` and `pr-title` CI jobs. | CI-4 |
| D10 | `thiserror = "2"`, declared in `[workspace.dependencies]`. Crates opt in only when they use it. | CODE-1 |
| D11 | Backlog M1.1 stays `[ ]` until the cleanup and M1.1 merge. `docs/backlog.md` is not edited here. | — |
| D12 | Font: `assets/FiraCode-Regular.ttf` (backlog M2.3 name). `MEMORY.md` is corrected to match. | DOC-12 |
| D13 | `.claudesignore` keeps `*.stl`, `*.step`, `*.png`, `*.ttf`, `assets/`, excluded from agent context. Golden tests read fixtures by explicit path. | none |
| D14 | One PR for the cleanup. M1.1 and M1.2 follow as separate PRs. | — |
| — | `ARCHITECTURE.md` is authoritative for any conflict not listed above. | all |

### 1.1 Licence identifier (derived, to confirm in review)

The §6 header grants "version 3 of the License … or (at your option) any later version". The licence identifier that matches the grant is `AGPL-3.0-or-later`. The `LICENSE` file is unchanged. This is the only value not stated explicitly by the human. The Challenger must confirm it. If it is wrong, change one line in `Cargo.toml` (CODE-1).

---

## 2. Remaining Blocking Items

Only three items block execution. Everything else has a default in this plan.

| # | Item | Why it blocks | Resolution |
|---|---|---|---|
| B1 | **Agent permission rules.** `.claude/settings.json` denies `Edit`/`Write` on `ARCHITECTURE.md`, `.github/**`, and `docs/adr/**`. The Executor cannot apply DOC-1–DOC-3, DOC-4, DOC-14, CI-1–CI-4 under those rules. | The Executor must not bypass the rule (for example with `Bash`/`sed`/`git apply`). The rule exists to stop agents editing binding files. | Human, once, before the Executor starts: remove the three deny groups from `.claude/settings.json` for this PR only, or grant the Executor an explicit exception. Restore the rules after merge. |
| B2 | **Rust toolchain.** Not installed. | Steps 7.2–7.6 cannot run. | Human: `! curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh`, then `cargo --version`. |
| B3 | **GitHub Action SHAs** (CI-1). The Executor must not guess SHAs. | A wrong SHA is a broken or unsafe workflow. | Executor resolves each SHA at execution time with `gh api` (see CI-1). If `gh` is not authenticated, the Executor stops and asks. |

Non-blocking, tracked for M2.4 (not this PR):
- **Single draw call with two textures.** The glyph atlas and the logo atlas (`ARCHITECTURE.md` §3.2, §3.4) must be bound in the one draw call. This needs a single bind group, a texture array, or a combined atlas. The choice is an ADR in M2.4. The documents in this PR say "one draw call" and do not choose the mechanism.

---

## 3. Documentation Corrections

These change only documentation and CI. Each item states the exact text to write. The Executor applies them after B1 is lifted.

### DOC-1 `ARCHITECTURE.md` §6, line 88 (copyright)
Replace
```
// Copyright (C) 2026 [Your Name/Company]
```
with
```
// Copyright (c) 2026 Martin Harambat
```

### DOC-2 `ARCHITECTURE.md` §6, code fence
The block at lines 86–92 opens with ` ```rust ` and has no closing fence. Add a closing ` ``` ` line directly after line 92. No other change to the header text.

### DOC-3 `ARCHITECTURE.md` §2, add Explorer as optional helper
Insert after the four bullets of §2 (before §3):

```markdown
- **Explorer (optional helper):** A read-only assistant the human or another agent may consult at any point to locate code or discuss options, with `path:line` references. It is **not** a pipeline stage, is not in the approval chain of §2, never edits, creates or deletes files, and never runs commands. The four agents above are the only primary agents.
```

### DOC-4 `ARCHITECTURE.md` §5.1, version pin example
Replace `brepkit = "=0.1.0"` with `brepkit = "=x.y.z"` and add after the Version Pinning bullet: "The exact version is chosen when M3.1 starts and recorded in an ADR." The `0.1.0` example conflicted with ADR 0001 (BRepKIT 3.0+).

### DOC-5 `ARCHITECTURE.md` §1, crate list
No change. The five crates are already listed there and are the reference for D1.

### DOC-6 `Description.md`
- §3.1 tree: keep only the five crates in D1. Remove `newell-sketch/` and `newell-io/`. Add one line under the tree: "Further crates (for example sketching or file I/O) are added only through an amendment to `ARCHITECTURE.md` §1."
- §6 heading and intro: "a team of 5 AI agents" → "a team of 4 AI agents, plus an optional read-only Explorer helper".
- §6.1 table: keep the Architect, Challenger, Executor and Auditor rows. Delete the Explorer row from the table. Add below the table: "**Explorer** (optional helper, Haiku 5.5): see `ARCHITECTURE.md` §2. It is not a pipeline stage." Change the §6.1 heading "The 5 agents" → "The 4 agents".
- §6.2 pipeline text: any reference to "five agents" → "four agents".

### DOC-7 `docs/adr/` — new ADR 0002 (this is a new file, not an edit of 0001)
Create `docs/adr/0002-workspace-layout-agent-roles-and-ui-rendering.md` from `0000-template.md`:
- Date 2026-10-12; Status **Approved**; Agent Author "Architect (Claude Opus 5.5)".
- Context: three documents disagreed on crate count, agent count, and the render model.
- Decision: D1 (five crates), D2 (four agents plus optional Explorer), D6 (single batched draw call), D10 (thiserror 2), D8 (commit `Cargo.lock`).
- Consequences: `Description.md` §3.1, §6 and `ui-design.md` §3–§4 are corrected to match. Future crates and any extra render pass require a new ADR.
- Do not edit `docs/adr/0001-*.md`.

### DOC-8 `docs/ui-design.md` §3, Sprite & Atlas
Replace the sprite-metrics bullet
"$256 \times 256$px per sprite, strictly mapped to a $2 \times 2$ grid cell matrix. Hitboxes correlate exactly to this $2 \times 2$ cluster."
with:
"$256 \times 256$px source slice per sprite (visible art $240 \times 240$px). Major tools occupy **5 columns × 3 rows**; minor tools occupy **3 columns × 2 rows** (`ARCHITECTURE.md` §3.3). Hitboxes are the allocated block. Centering is done by the fragment shader ("Scale to Fit"); CPU code never computes sub-pixel offsets."

Also replace the "Bleed Margin" and "Alignment" bullets' wording only where they mention 2×2 (none beyond the above).

### DOC-9 `docs/ui-design.md` §4, Interaction & Rendering Pipeline
- Replace "Hover/Focus events trigger immediate cell background hue shifts." with: "Hover is computed on the CPU as a `WidgetState` (`Normal | Hovered`) and rendered in the shader: text shifts colour and the leftmost pixels of the cell are highlighted; logos use a brightness multiplier (`color * 1.4`). The CPU never computes fades or colour transitions." (`ARCHITECTURE.md` §3.5.)
- Replace the whole "Z-Index Render Passes (Strict Sequence)" list with:

```markdown
* **Single batched draw call:** The Controller writes one flat 1D cell array (`ARCHITECTURE.md` §3.4). `newell-render` draws the whole ASCII overlay in one `wgpu` draw call. Layers are ordered by their position in that array, from back to front: grid background, glyphs, logo sprites, then modals/dropdowns. Modals block input through the Controller's focus trap, not through an extra render pass.
* The 3D viewport is drawn by its own pass before the overlay. This is not part of the UI draw.
```

### DOC-10 `docs/ui-design.md` §2, Modals
Append: "Focus trap is implemented in `newell-ui` (Controller). It is not a render pass."

### DOC-11 `docs/ui-design.md` header
Add under the Status line: "This document is subordinate to `ARCHITECTURE.md`. Where they conflict, `ARCHITECTURE.md` wins."

### DOC-12 `MEMORY.md`
- "Current Task": "Complete M1.1 (workspace, CI/CD, `.claudesignore`) per `docs/plans/CleanTheProject.md`, then M1.2."
- Pitfall "Texture Atlas": unchanged in substance. Keep the 8px padding rule.
- Font filename: `assets/FiraCode-Regular.ttf` (D12).

### DOC-13 `README.md`
- Agents list: "Four pipeline agents (Architect, Challenger, Executor, Auditor) and an optional read-only Explorer helper."
- Models per agent file: Architect = Opus 5.5; Challenger, Executor, Auditor = Sonnet 5.5; Explorer = Haiku 5.5.
- Remove "5 AI Agents" wording and "Explorer (Haiku): … project navigation" as a numbered pipeline item.

### DOC-14 `.claude/agents/explorer.md`
- `name: explorer` (lowercase, matching the other agents).
- `description:` replace "Dispatcher" with "optional read-only helper; not a pipeline stage". Keep the rest.

### DOC-15 `.claude/agents/architect.md`, `challenger.md`
- Remove `Glob` from `tools:`. Keep `Grep` with an explicit path rule in the body: "Search only inside the directories named in the task." This follows `CLAUDE.md`.

### DOC-16 Repository hygiene (not a specification change)
- `git mv docs/plans/M1-setup.md docs/plans/archive/M1-setup.md`. `docs/plans/M1-setup-v2.md` stays. The `.claudesignore` line `docs/plans/archive/` then points to a real folder.
- `docs/plans/CleanTheProject.md` (this file) stays in `docs/plans/`.

---

## 4. Code and Config Fixes

No production logic. Everything here is manifest, lint, toolchain, and header.

### CODE-1 Root `Cargo.toml` (full replacement)

The AGPL header is required only in `.rs` files (`ARCHITECTURE.md` §6). `Cargo.toml` gets no header.

```toml
[workspace]
members = [
    "crates/newell-app",
    "crates/newell-render",
    "crates/newell-ui",
    "crates/newell-model",
    "crates/newell-kernel",
]
resolver = "3"

[workspace.package]
edition = "2024"
rust-version = "1.85"
license = "AGPL-3.0-or-later"   # §1.1: to confirm in review
publish = false

[workspace.dependencies]
thiserror = "2"
proptest = "1.4"
insta = "1.38"
# brepkit is NEVER declared here. Only in crates/newell-kernel/Cargo.toml (M3.1), pinned =x.y.z.

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
dbg_macro = "deny"
```

Notes:
- `proptest` and `insta` are declared now because `ARCHITECTURE.md` §5 and the backlog (M1.2) name them. They are not used by any crate until M1.2. Crates opt in with `<dep>.workspace = true` only when they use them.
- `thiserror` is not added to any crate in this PR (no code uses it yet).
- Comments are English (the French comments are removed).

### CODE-2 Crate stubs (five crates)

For each of `newell-kernel`, `newell-model`, `newell-ui`, `newell-render`, `newell-app`:

`crates/<name>/Cargo.toml`:
```toml
[package]
name = "<name>"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
```

Crate-specific content:
- `newell-kernel`, `newell-model`, `newell-ui`, `newell-render`: `src/lib.rs` containing only the 6-line AGPL header from `ARCHITECTURE.md` §6 (with the D7 copyright line) followed by one `//!` line describing the crate's role in `ARCHITECTURE.md` §1.
- `newell-app`: `src/main.rs` containing the same header, one `//!` line, and `fn main() {}`.

Each `.rs` file starts with the header on lines 1–6 exactly as in `ARCHITECTURE.md` §6, with no BOM.

Dependency rules (`ARCHITECTURE.md` §1, §5.1): `newell-kernel` is the only crate allowed to name `brepkit`, and that happens in M3.1. No crate in this PR depends on `wgpu`, `winit`, or `brepkit`.

Dependencies between crates are not added in this PR. They are added with the code that needs them.

### CODE-3 Toolchain, lint config, lockfile

- `rust-toolchain.toml`:
  ```toml
  [toolchain]
  channel = "stable"
  components = ["rustfmt", "clippy"]
  ```
- `clippy.toml` (test code only; production stays strict per `ARCHITECTURE.md` §5):
  ```toml
  allow-unwrap-in-tests = true
  allow-expect-in-tests = true
  allow-panic-in-tests = true
  ```
- `Cargo.lock`: generate with `cargo generate-lockfile` (or the first `cargo check`) and commit it (D8).
- `deny.toml`: allow-list of AGPL-3.0-compatible licences only (`AGPL-3.0-or-later`, `MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Unicode-3.0`, `Zlib`). Unknown licences are denied. Advisories denied by default.

### CODE-4 `.gitignore`
Add:
```
*.snap.new
*.pending-snap
```

### CODE-5 `.claudesignore`
- Translate the French comment to English.
- Add `Cargo.lock` (large, not useful as agent context; still committed).
- Keep `docs/plans/archive/`.

---

## 5. CI Corrections (`.github/workflows/ci.yml`)

These are documentation-class changes to CI config. They are part of this PR, and B1 must be lifted first.

### CI-1 SHA-pin every action
Replace each `uses: owner/action@tag` with `uses: owner/action@<full-40-char-sha> # <tag>`. The Executor resolves each SHA at execution time:

```bash
gh api repos/actions/checkout/git/ref/tags/v4 --jq .object.sha
```

Repeat for `dtolnay/rust-toolchain` (`stable`, branch ref), `Swatinem/rust-cache` (`v2`), `taiki-e/install-action` (`nextest`), `EmbarkStudios/cargo-deny-action` (`v2`), `amannn/action-semantic-pull-request` (`v5`). If a tag resolves to an annotated tag object, dereference it to the commit SHA. If any lookup fails, stop (B3).

### CI-2 Flags
- Every `cargo` command in CI gets `--locked`.
- `--all-features` kept on `clippy`, `nextest`, and `doc` (D5). Removed from none.

### CI-3 Top-level hardening
Add at the top level of the workflow:
```yaml
permissions:
  contents: read
```
And in the `test` job strategy:
```yaml
fail-fast: false
```

### CI-4 Jobs

- `fmt`, `clippy`, `test`, `aarch64`, `doc`: keep, with CI-2 flags.
- `test` job: add a step after nextest:
  ```yaml
  - name: Run doctests
    run: cargo test --workspace --doc --locked
  ```
  nextest does not run doctests (C7).
- `deny` job: replace the two steps with one:
  ```yaml
  - name: Run cargo-deny
    uses: EmbarkStudios/cargo-deny-action@<sha> # v2
    with:
      command: check
  ```
  The action runs `cargo deny` itself. The separate `run: cargo deny check` step is removed (C3).
- `msrv` job (new, D9):
  ```yaml
  msrv:
    name: MSRV (1.85)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@<sha> # v4
      - name: Install Rust 1.85
        uses: dtolnay/rust-toolchain@<sha> # 1.85
        with:
          toolchain: "1.85"
      - name: Check on minimum toolchain
        run: cargo check --workspace --locked
  ```
- `pr-title` job (new, D9), runs on `pull_request` only:
  ```yaml
  pr-title:
    name: PR title
    runs-on: ubuntu-latest
    permissions:
      pull-requests: read
    steps:
      - uses: amannn/action-semantic-pull-request@<sha> # v5
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  ```

---

## 6. Execution Order

The Executor runs these in order and stops at the first failure it cannot fix within two attempts.

0. **Prerequisites:** B1 lifted, B2 done, B3 resolvable.
1. Repository hygiene: DOC-16, CODE-4, CODE-5.
2. Root manifest: CODE-1. Toolchain and lint files: CODE-3.
3. Crate stubs: CODE-2.
4. Documentation: DOC-1, DOC-2 (`ARCHITECTURE.md`), DOC-3, DOC-4 (`ARCHITECTURE.md`), DOC-6 (`Description.md`), DOC-7 (new ADR 0002), DOC-8–DOC-11 (`ui-design.md`), DOC-12 (`MEMORY.md`), DOC-13 (`README.md`), DOC-14–DOC-15 (agents).
5. CI: CI-1 through CI-4.
6. Verification (§7).
7. Commit as one PR (D14). Do not merge. Do not push without the human's go-ahead.

---

## 7. Verification Gate

Run each command from the repo root. Each must exit 0 unless noted.

1. `cargo fmt --all -- --check`
2. `cargo check --workspace --locked`
3. `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
4. `cargo nextest run --workspace --all-features --locked` (zero tests passes). If nextest is not installed, `cargo test --workspace --locked`.
5. `cargo test --workspace --doc --locked` (zero doctests passes).
6. `cargo doc --workspace --no-deps --document-private-items --locked` with `RUSTDOCFLAGS="-D warnings"`.
7. `cargo deny check`
8. `cargo check --workspace --locked --target aarch64-unknown-linux-gnu` if the target is installed. If not, skip and say so.
9. Grep gates (each must return no matches unless noted):
   - `grep -rn "unsafe" crates --include=*.rs`
   - `grep -rn "unwrap()\|expect(\|panic!" crates --include=*.rs`
   - `grep -rln "brepkit\|opencascade\|occt" crates Cargo.toml` (must be empty in this PR)
   - Every `.rs` file starts with the header: for f in `find crates -name '*.rs'`, `head -6 "$f"` must equal the header in `ARCHITECTURE.md` §6.
   - `grep -rn "2×2\|2x2\|four render passes\|hue shift" docs/ui-design.md` (empty)
   - `grep -rn "5 AI agents\|5 agents\|newell-sketch\|newell-io" Description.md README.md` (empty, except the explicit "not in this workspace" note in DOC-6)
   - `grep -n "Your Name/Company" ARCHITECTURE.md` (empty)
10. `git diff --stat` matches §6 exactly. `docs/backlog.md` is unchanged. `LICENSE` is unchanged. `docs/adr/0001-*.md` is unchanged.

---

## 8. Acceptance Criteria

- AC1: Gates 7.1–7.7 pass on a machine with Rust installed. If any fails, the output is pasted in the PR description.
- AC2: `ARCHITECTURE.md` changes are only DOC-1 to DOC-4. All other `ARCHITECTURE.md` text is byte-identical to `HEAD`.
- AC3: All specifications (`Description.md`, `README.md`, `ui-design.md`, `MEMORY.md`, agent files) agree with `ARCHITECTURE.md` and the approved decisions in §1.
- AC4: No production logic, no placeholder implementation, no unused dependency, no new kernel.
- AC5: Every file in §6 changes and no other file does.

---

## 9. Executor Prompt

Paste this to start the Executor after B1 is lifted:

> You are the Executor for Newell. Read, in this order: `ARCHITECTURE.md`, then `docs/plans/CleanTheProject.md` (revision 2). Follow the plan exactly. Do not edit any file not listed in its §6.
>
> Rules:
> - Do not implement production logic. Crates are stubs containing only the header and the minimum needed to compile.
> - Do not add BRepKIT or any other geometric kernel. Do not add dependencies beyond those in CODE-1.
> - Do not bypass the permission rules in `.claude/settings.json`. If a write is denied, stop and report it.
> - For CI-1, resolve action SHAs with `gh api` at execution time. Do not guess. If a lookup fails, stop.
> - Do not run `Glob` or `Grep` over the whole repository. Search only inside the paths named in the plan.
> - Do not commit or push. Leave the diff for the human to review.
>
> Steps: §6 in order. Run the §7 gate and paste each command with its exit status. Stop after two failed attempts at the same command and report the output. For any documentation change that differs from the text in the plan, stop and ask.

---

## 10. Notes for the Challenger

- Confirm `resolver = "3"` with `edition = "2024"` and `rust-version = "1.85"`. Confirm `workspace.lints` is valid for those.
- Confirm the licence identifier in §1.1.
- Confirm DOC-3 wording does not let Explorer become a pipeline stage.
- Confirm DOC-9's layer ordering is achievable in one batched draw call, and flag the glyph-atlas plus logo-atlas binding question from §2 as an M2.4 risk.
- Confirm DOC-4 does not pin any version. The pin is M3.1.
