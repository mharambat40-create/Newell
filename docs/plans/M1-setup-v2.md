# Plan M1.1 + M1.2 (revision 2): Workspace, CI/CD, Testing Architecture

**Author:** Architect · **Status:** Draft, waiting for Challenger review
**Backlog:** `docs/backlog.md:4` (M1.1), `docs/backlog.md:5` (M1.2). M1.3 (`docs/backlog.md:6`) is out of scope.
**Supersedes:** `docs/plans/M1-setup.md` (prior draft, kept unchanged for comparison).
**Binding:** `ARCHITECTURE.md` §1, §2, §5, §5.1, §6. Reference: `Description.md` §6.1, §7.1, §8.7, §10 (M1 row).

---

## 0. Context and Open Decisions

Repository today: a root `Cargo.toml` naming five crates that don't exist, a CI file, a `.claudesignore`. No Rust code, no `Cargo.lock`, and no Rust toolchain on the dev machine (`cargo`/`rustc` not found).

Changes from the prior draft:
- Several choices the prior draft made on its own are now open decisions for the human (D1–D7).
- Third-party actions are pinned to commit SHAs (`Description.md` §8.7). The prior draft pinned them to tags.
- The header check compares all 6 lines of the header, not only line 1.
- The lint list matches `Description.md` §7.1 exactly. The prior draft added `unimplemented`, which no document requires.
- `.claudesignore` gets a small rewrite. The prior draft left it unchanged.

| ID | Decision | Options | Status |
|----|----------|---------|--------|
| D1 | GitHub rulesets (`Description.md` §10, M1 row) | (a) in M1.1: protect `main`, require the CI checks in §2.4; (b) defer to a later item | **pending** |
| D2 | `msrv` and `pr-title` CI jobs (`Description.md` §8.7) | (a) add both in M1.1; (b) add only `msrv`; (c) defer both | **pending** |
| D3 | `thiserror` version | (a) `"2"` (current major); (b) `"1.0"` (current `Cargo.toml:25`) | **pending** |
| D4 | `tests/` scope | (a) workspace-wide: `tests/it/` in all 5 crates; (b) `newell-model` only, as a canary | **pending** |
| D5 | Clippy/test flags | (a) add `--all-features` and `--locked`; (b) `--locked` only; (c) neither (the `CLAUDE.md` form) | **pending** |
| D6 | Copyright placeholder `ARCHITECTURE.md:88` `[Your Name/Company]` | The human edits `ARCHITECTURE.md`. The Architect must not. | **pending** (blocking, Q1) |
| D7 | Backlog tick for M1.1 | HEAD has `[x]`. The working tree has `[ ]` (uncommitted). Keep `[ ]` until this plan merges? | **pending** |

In this plan, `<D3>`, `<D5-FLAGS>` and `<HOLDER>` are placeholders. The Executor fills them in from the human's answers. No Executor work starts before D6 is resolved.

---

## 1. Current-State Audit

| # | Location | Defect | Fix in |
|---|----------|--------|--------|
| A1 | `Cargo.toml:2-8` | Lists 5 members, but `crates/` does not exist, so every `cargo` command fails to load the workspace. | M1.1 |
| A2 | `Cargo.toml:22-25` | `proptest`, `insta` and `thiserror` sit under `[workspace.lints.clippy]`. Cargo parses them as lints with invalid levels (`"1.4"`), so the manifest is rejected. They belong in `[workspace.dependencies]`. | M1.2 |
| A3 | `Cargo.toml:25` | `thiserror = "1.0"`: the version choice is open (D3). | M1.2 |
| A4 | `Cargo.toml:9` | `resolver = "2"` and no `[workspace.package]`, so there is no shared `edition`, `license` or `rust-version`. Crates would default to edition 2015 and carry no SPDX license. | M1.1 |
| A5 | `Cargo.toml:12-20` | The workspace lints apply to nothing, because no crate sets `[lints] workspace = true`. | M1.1 |
| A6 | `Cargo.toml:11`, `Cargo.toml:22`; `.claudesignore:8` | Comments are in French. Project files are in English. | M1.1 |
| A7 | (missing) `clippy.toml` | `Description.md` §7.1 requires `allow-unwrap-in-tests` and `allow-expect-in-tests`. Without them, `clippy --all-targets -D warnings` fails on test code. | M1.2 |
| A8 | `.github/workflows/ci.yml:22,24,34,36,39,51,53,54,56,64,66,69,78,80,81,92,94` | Actions are pinned by tag (`@v4`, `@stable`, `@v2`, `@nextest`). `Description.md` §8.7 requires full commit SHAs. | M1.1 |
| A9 | `.github/workflows/ci.yml:9-15` | No `permissions:` block. `Description.md` §8.7 requires least privilege. | M1.1 |
| A10 | `.github/workflows/ci.yml:93-96` | `cargo-deny-action` is used as an install step and then `cargo deny check` runs as a plain `run:` step. The action does not put `cargo-deny` on `PATH`, so the step fails. There is also no `deny.toml`. | M1.1 |
| A11 | `.github/workflows/ci.yml:58` | nextest does not run doctests. `INSTA_UPDATE` is not set, so CI could write snapshots instead of failing. | M1.2 |
| A12 | `.github/workflows/ci.yml:47-48` | The test matrix has no `fail-fast: false`, so one OS failing cancels the other runs. | M1.1 |
| A13 | `.github/workflows/ci.yml:17-96` | No `msrv` or `pr-title` job (D2). No machine check of the §6 header. | M1.1 |
| A14 | `.github/workflows/ci.yml:42,58,73,84` | Flags are inconsistent: `--all-features` on two jobs, no `--locked` anywhere, and no `Cargo.lock` exists (D5). | M1.1 |
| A15 | `.gitignore:1-2` | Pending insta files (`*.snap.new`, `*.pending-snap`) are not ignored. | M1.2 |
| A16 | `.claudesignore:9` | Lists `docs/plans/archive/`, which does not exist (harmless). Does not exclude `Cargo.lock`, which is large and of no use to agents. | M1.1 |
| A17 | `ARCHITECTURE.md:88` | Copyright placeholder (D6). | Human |
| A18 | `ARCHITECTURE.md:86-92` | The §6 header code fence is never closed (no closing fence before EOF). This plan treats lines 87-92 as the full 6-line header. | Human (Q2) |
| A19 | `docs/backlog.md:4` | HEAD marks M1.1 done, but nothing exists (D7). | Human |
| A20 | `MEMORY.md:7-8` | "Current Task" describes M1.3/M2 work. It is stale against the backlog. | Human / after merge |
| A21 | (missing) `rust-toolchain.toml`, `Cargo.lock`, `deny.toml`, `.gitattributes` | No pinned toolchain, no reproducible builds, no LF policy (insta snapshots on Windows). | M1.1 |

---

## 2. M1.1: Workspace, `.claudesignore`, CI/CD

### 2.1 Files

```
Cargo.toml                       REWRITE (2.2)
Cargo.lock                       GENERATE + COMMIT (binary app)
rust-toolchain.toml              CREATE: channel "stable", components rustfmt, clippy
deny.toml                        CREATE (2.5)
.gitattributes                   CREATE: "* text=auto eol=lf"; *.png *.ttf *.stl *.step binary
.claudesignore                   REWRITE (2.6)
.github/workflows/ci.yml         REWRITE (2.4)
crates/newell-{kernel,model,ui,render}/Cargo.toml + src/lib.rs   CREATE
crates/newell-app/Cargo.toml + src/main.rs                       CREATE
```

### 2.2 Root `Cargo.toml` (complete after M1.1 + M1.2)

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
version = "0.1.0"
edition = "2024"
rust-version = "1.85"          # minimum for edition 2024; the msrv job (D2) builds with it
license = "AGPL-3.0-or-later"
publish = false

# Shared versions. Crates opt in with `<dep>.workspace = true`.
# ARCHITECTURE.md §5.1: `brepkit` must NEVER appear here. It is declared,
# pinned with `=x.y.z`, only in crates/newell-kernel/Cargo.toml (M3.1).
[workspace.dependencies]
thiserror = "<D3>"             # M1.2
proptest = "1"                 # M1.2, dev only
insta = "1"                    # M1.2, dev only

# Auditor rules for the whole project (ARCHITECTURE.md §5, Description.md §7.1).
[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
dbg_macro = "deny"

[profile.dev.package.insta]    # M1.2: faster snapshot diffs
opt-level = 3
[profile.dev.package.similar]
opt-level = 3
```

If M1.1 and M1.2 land as separate PRs, M1.1 omits `[workspace.dependencies]` and the `profile.dev.package` tables.
Before committing, the Executor raises `proptest`/`insta` to the latest published minor (`cargo search <crate> --limit 1`) and keeps the caret form. Only `brepkit` uses exact `=` pinning.

### 2.3 Per-crate manifests and stubs

Every crate manifest contains:
- `[package]` with `name`, a one-line `description`, and `version`/`edition`/`rust-version`/`license`/`publish` all set with `.workspace = true`;
- empty `[dependencies]`;
- `[lints] workspace = true`.

`newell-app` also declares `[[bin]] name = "newell", path = "src/main.rs"`.

| Crate | MVC role (§1) | `description` |
|-------|---------------|---------------|
| `newell-kernel` | Model | Anti-corruption layer over the BrepKIT kernel. |
| `newell-model` | Model | Parametric feature tree and parameters. |
| `newell-ui` | Controller | Flat cell grid, O(1) hit-testing, per-window UI state. |
| `newell-render` | View | wgpu rendering of the ASCII grid and 3D viewport. |
| `newell-app` | Controller | Binary entry point and window manager. |

Stubs: the §6 header (6 lines, verbatim, with `<HOLDER>` filled in), a blank line, then `//! <description>`.
- `main.rs` adds `fn main() {}`.
- `lib.rs` has no items. Delete the `add()` function and test that `cargo new` generates.
- No inter-crate dependencies in M1. The MVC dependency graph is wired when real types exist (M1.3+).

### 2.4 CI job list (`.github/workflows/ci.yml`)

Global settings:
- `on: push/pull_request` to `main`;
- `permissions: contents: read`;
- `env: CARGO_TERM_COLOR=always, RUSTFLAGS=-Dwarnings`;
- keep the existing `concurrency` block (`ci.yml:13-15`).

Every `uses:` is pinned as `owner/action@<40-char SHA> # vX.Y.Z`. The Executor resolves the SHAs with `gh api repos/<owner>/<action>/commits/<tag> --jq .sha`.

| Job | Runner | Steps | Condition |
|-----|--------|-------|-----------|
| `fmt` | ubuntu | `cargo fmt --all -- --check` | always |
| `license-header` | ubuntu | Bash: for each `*.rs` under `crates/`, strip `\r`, then compare the first 6 lines byte-for-byte to the §6 block. Emit `::error file=` and exit 1 on a mismatch. | always |
| `clippy` | ubuntu | `cargo clippy --workspace --all-targets <D5-FLAGS> -- -D warnings` | always |
| `test` | ubuntu, macos, windows; `fail-fast: false` | `cargo nextest run --workspace <D5-FLAGS>` + `cargo test --doc --workspace <D5-FLAGS>`; env `INSTA_UPDATE=no` (M1.2) | always |
| `aarch64` | ubuntu | install `gcc-aarch64-linux-gnu`, then `cargo check --workspace --target aarch64-unknown-linux-gnu` | always |
| `doc` | ubuntu | `cargo doc --workspace --no-deps --document-private-items`, `RUSTDOCFLAGS=-D warnings` | always |
| `deny` | ubuntu | `EmbarkStudios/cargo-deny-action` with `command: check` (no separate `run:` step) | always |
| `msrv` | ubuntu | toolchain `1.85`, then `cargo check --workspace --all-targets` | only if D2 = a/b |
| `pr-title` | ubuntu, `pull_request` only | Conventional Commits check, e.g. `amannn/action-semantic-pull-request` (SHA-pinned, `pull-requests: read`) | only if D2 = a |
| `bench` | n/a | Out of scope: nothing to benchmark before M2. | deferred |

If D1 = a, the required status checks are all of the jobs above that exist, plus `test` for each OS. The human applies the ruleset (an outward-facing action, not a repo file). The Executor can only document it in the PR body.

### 2.5 `deny.toml` (summary; the prior draft §2.6 content is reused, with these rules)
- `[licenses] allow`: AGPL-3.0-or-later, AGPL-3.0-only, MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0. Set `private = { ignore = true }`.
- `[bans]`:
  - `{ crate = "brepkit", wrappers = ["newell-kernel"] }`: a machine check of §5.1;
  - deny `opencascade`, `opencascade-sys`, `truck-*` (§5 exclusivity);
  - `wildcards = "deny"`, `multiple-versions = "warn"`.
- `[sources]`: deny unknown registries and git sources. `[advisories]`: `yanked = "deny"`.

### 2.6 `.claudesignore` (full content)
```
# Files agents must never read (context cost)
target/
assets/
Cargo.lock
*.png
*.ttf
*.stl
*.step
tests/fixtures/golden/
docs/plans/archive/
```

### 2.7 M1.1 verification (run in order, each must exit 0)
```sh
cargo check --workspace                      # first run creates Cargo.lock
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace                # fallback: cargo test --workspace
cargo test --doc --workspace
cargo run --release                          # exits 0 immediately
```
Before these commands, the human installs the toolchain once: rustup stable, then `cargo install --locked cargo-nextest cargo-insta cargo-deny`.

### 2.8 M1.1 acceptance criteria
1. Every command in §2.7 exits 0 on macOS. With zero tests, nextest must not fail. If it does, use `--no-tests=pass` (Challenger: see §8).
2. CI is green on every job that exists after D2, including all 3 OSes and aarch64.
3. Adding `pub fn f() { None::<u8>.unwrap(); }` to any `src/lib.rs` makes `clippy` fail. Adding `unsafe {}` makes `check` fail.
4. Removing or altering one header line in any `.rs` file makes `license-header` fail.
5. `grep -rn brepkit Cargo.toml crates/*/Cargo.toml` prints nothing.
6. `grep -n '@v\|@stable\|@nextest' .github/workflows/ci.yml` prints nothing (every action is SHA-pinned).
7. `Cargo.lock` is committed.

---

## 3. M1.2: Testing Architecture

### 3.1 Dependencies
- Workspace: `thiserror`, `proptest` and `insta` move to `[workspace.dependencies]` (§2.2, fixes A2/A3).
- Every crate with a `tests/it/` directory (D4) adds `[dev-dependencies] proptest.workspace = true, insta.workspace = true`.
- **No crate depends on `thiserror` in M1.** There are no fallible operations yet, and a placeholder error enum would be speculative API. The first consumers are `newell-ui` (M2.1) and `NewellKernelError` in `newell-kernel` (M3.1, §5.1).

### 3.2 Test layout (one integration binary per crate)
```
crates/<crate>/
├── src/*.rs                 unit tests: #[cfg(test)] mod tests { use super::*; }
└── tests/it/
    ├── main.rs              AGPL header + allow attribute + `mod` lines only
    ├── <area>.rs            one module per public-API area
    └── snapshots/           insta output, COMMITTED (it__<module>__<test>.snap)
```
Rules:
- No top-level `tests/*.rs` files: each one compiles into its own binary.
- proptest regression files are committed.
- `.snap.new` and `.pending-snap` files are never committed.
- `newell-app` is a binary, so its tests cannot import it. Testable logic lives in library crates.

Scope depends on D4:
- **D4 = a:** `tests/it/{main.rs,harness.rs}` in all 5 crates.
- **D4 = b:** only `crates/newell-model/tests/it/`. Other crates get `tests/it/` with their first public API (M2.1 `newell-ui`, M3.1 `newell-kernel`).

`tests/it/main.rs` (after the header):
```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod harness;
```
This crate-level allow covers helper functions outside `#[test]`, which `clippy.toml` does not cover. It is scoped to the test binary and never reaches production code (§5).

`clippy.toml` (new, per `Description.md` §7.1):
```toml
# ARCHITECTURE.md §5 bans unwrap/expect/panic in production code only.
allow-unwrap-in-tests = true
allow-expect-in-tests = true
```
`allow-panic-in-tests` is not added: `Description.md` §7.1 does not list it, and the `main.rs` allow already covers integration tests.

### 3.3 Canary tests (`tests/it/harness.rs`)

These tests prove the toolchain end to end: proptest, insta, nextest, the clippy exemptions and the CI snapshot policy. All data is local to the test file, and **no production items are added**. Delete the canaries when real tests land in the same crate.

| Test | Body | Proves |
|------|------|--------|
| `insta_snapshot_is_wired` | `assert_debug_snapshot!` on a test-local `#[derive(Debug)] struct Canary { id: u32, label: &'static str }` with fixed values | Snapshot read/write works. CI fails on a missing snapshot. |
| `proptest_is_wired` | `proptest!` over `any::<i64>()`: `x.to_string().parse::<i64>() == Ok(x)` | proptest runs. The default strategy hits `0`, `i64::MIN`, `i64::MAX`. |
| `unwrap_allowed_in_tests` | `Some(1u8).unwrap() == 1` | The test exemptions pass `clippy --all-targets -D warnings`. |

### 3.4 insta wiring
- `.gitignore` adds `*.snap.new` and `*.pending-snap`.
- CI `test` job: `INSTA_UPDATE=no`.
- `.gitattributes` forces LF, so snapshots are byte-identical on Windows.
- Locally: `cargo insta test --workspace --test-runner nextest`, then `cargo insta review`.

### 3.5 Golden-fixture directory
- Path: `tests/fixtures/golden/`, at the repo root (§5.1). The workspace is virtual, so root `tests/` is data only, not a package.
- M1.2 creates only `tests/fixtures/golden/README.md`. It states:
  - naming: `<feature>_<case>.<ext>`;
  - each fixture is paired with an insta snapshot of derived invariants (volume, F/E/V counts, bounding box, at a fixed rounding) in `crates/newell-kernel/tests/it/snapshots/`;
  - a `brepkit` bump requires `cargo nextest run -p newell-kernel`, and any snapshot change requires human review;
  - agents do not read fixtures (`.claudesignore`).
- Path from kernel tests (M3.1): `concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/golden/<file>")`.
- No fixture files and no kernel tests in M1. `brepkit` is not added in M1.

### 3.6 M1.2 verification
```sh
cargo check --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo insta test --workspace --test-runner nextest && cargo insta review   # accept canary once
cargo nextest run --workspace                # fallback: cargo test --workspace
cargo test --doc --workspace
INSTA_UPDATE=no cargo nextest run --workspace
```

### 3.7 M1.2 acceptance criteria
1. Every command in §3.6 exits 0 on macOS, and CI is green.
2. The 3 canaries pass in every crate covered by D4.
3. Deleting a committed `.snap` makes the CI `test` job fail.
4. Changing the canary's `label` makes the test fail and creates a `.snap.new`, which git ignores.
5. `proptest`, `insta` and `thiserror` appear only under `[workspace.dependencies]` in the root manifest.
6. `tests/fixtures/golden/README.md` exists. No other files exist under `tests/`.

---

## 4. Files Touched

| Action | Path | Milestone |
|--------|------|-----------|
| Rewrite | `Cargo.toml` | M1.1 (+ M1.2 deps) |
| Generate + commit | `Cargo.lock` | M1.1 |
| Create | `rust-toolchain.toml` | M1.1 |
| Create | `deny.toml` | M1.1 |
| Create | `.gitattributes` | M1.1 |
| Rewrite | `.claudesignore` | M1.1 |
| Rewrite | `.github/workflows/ci.yml` | M1.1 (+ `INSTA_UPDATE` in M1.2) |
| Create | `crates/newell-{kernel,model,ui,render}/Cargo.toml`, `src/lib.rs` | M1.1 |
| Create | `crates/newell-app/Cargo.toml`, `src/main.rs` | M1.1 |
| Edit | `crates/*/Cargo.toml` (dev-deps, per D4) | M1.2 |
| Create | `clippy.toml` | M1.2 |
| Edit | `.gitignore` | M1.2 |
| Create | `crates/<D4 scope>/tests/it/main.rs`, `harness.rs`, `snapshots/it__harness__insta_snapshot_is_wired.snap` | M1.2 |
| Create | `tests/fixtures/golden/README.md` | M1.2 |
| Human only | `ARCHITECTURE.md` (D6, Q2), `docs/backlog.md` (D7), `MEMORY.md` (A20) | n/a |
| Do not touch | `LICENSE`, `docs/plans/M1-setup.md`, anything in M1.3 | n/a |

---

## 5. Constraints Check

| Rule | Source | How this plan complies |
|------|--------|------------------------|
| Strict MVC crate split | §1 | The 5 crates match §1 exactly. No cross-crate deps in M1, so no layer violation is possible. |
| Role boundaries | §2 | This file is a plan only. The Executor writes code. Human-only files are listed in §4. |
| BrepKIT is the only kernel | §5 | `deny.toml` bans OpenCASCADE/truck crates. |
| No `unsafe` | §5 | `unsafe_code = "forbid"` in workspace lints, and every crate sets `[lints] workspace = true`. Acceptance criterion 2.8-3. |
| No `unwrap`/`expect`/`panic!` in production | §5 | Clippy denies them. The exemptions are limited to `#[test]`/`#[cfg(test)]` (`clippy.toml`) and the `tests/it` binaries. The stubs contain no such calls. |
| `Result<T, NewellError>` for geometric ops | §5 | Not applicable: there are no geometric ops in M1. |
| `brepkit` only in `newell-kernel`, exact pin | §5.1 | **Not added in M1.** Comment in the root manifest; `deny.toml` `wrappers` rule; acceptance criterion 2.8-5. ADR 0001 not consulted (pinning not touched). |
| No type leaking / `NewellKernelError` via `thiserror` | §5.1 | Deferred to M3.1. `thiserror` is declared in the workspace and has no consumer yet. |
| Golden files in `tests/fixtures/` | §5.1 | The directory and its contract README are created (§3.5). |
| AGPL header on every `.rs` file | §6 | Every stub and test file starts with the 6-line block, enforced by the `license-header` CI job. **Blocked by D6.** |

---

## 6. Test List (including degenerate cases)

| # | Test / check | Kind | Degenerate case covered |
|---|--------------|------|-------------------------|
| T1 | `insta_snapshot_is_wired` | integration | Missing snapshot with `INSTA_UPDATE=no` fails, not passes |
| T2 | `proptest_is_wired` | property | `0`, `i64::MIN`, `i64::MAX`, negative numbers |
| T3 | `unwrap_allowed_in_tests` | lint canary | Clippy with `--all-targets` on test code |
| T4 | `cargo nextest run` on a crate with zero tests | CI | Empty test set must not fail (see §8) |
| T5 | `cargo test --doc` with zero doctests | CI | Passes with no doctests |
| T6 | `license-header` on a CRLF-encoded `.rs` file | CI script | `\r` stripped before comparing |
| T7 | `license-header` on an empty `.rs` file or one with fewer than 6 lines | CI script | Must fail, not crash or pass |
| T8 | `license-header` on a file with a UTF-8 BOM | CI script | Must fail (the BOM breaks byte equality); documents the policy |
| T9 | `license-header` on a `.rs` file under `tests/it/` | CI script | `find crates` recurses into test dirs |
| T10 | Snapshot on the Windows runner | CI | LF policy keeps the bytes identical |
| T11 | `cargo deny check` with zero third-party deps (M1.1 alone) | CI | Passes on an empty graph |
| T12 | `cargo check --target aarch64-unknown-linux-gnu` with no deps | CI | Cross target installed, no linker needed |
| T13 | Mutation: add `unwrap()` to a `lib.rs` | manual | Clippy fails (production lints are live) |
| T14 | Mutation: delete the `.snap` | manual | CI `test` fails |
| T15 | `msrv` on 1.85 (if D2 = a/b) | CI | Edition 2024 builds on the minimum toolchain |

---

## 7. Open Questions

| # | Question | Blocking? |
|---|----------|-----------|
| Q1 | D6: what is the copyright holder for `ARCHITECTURE.md:88`? No `.rs` file can be written without it. | **Blocking** |
| Q2 | `ARCHITECTURE.md:86-92`: the header fence is unclosed. Is the 6-line block the complete header, or is a warranty/licence-URL paragraph missing? | **Blocking** (it defines what `license-header` checks) |
| Q3 | Rust toolchain is not installed on the dev machine. Install it before Executor work. | **Blocking** for verification |
| Q4 | D1–D5, D7 (§0). | Non-blocking: the plan has a branch for each option |
| Q5 | One PR for M1.1 + M1.2, or two? Two PRs means M1.1 ships without `[workspace.dependencies]`. | Non-blocking |
| Q6 | `MEMORY.md:7-8` "Current Task" is stale. Update it after merge? | Non-blocking |
| Q7 | Output path: the request left `[OUTPUT_PATH]` unset. This file was written to `docs/plans/M1-setup-v2.md`. Rename it if another name is preferred. | Non-blocking |

---

## 8. Notes for the Challenger

- **Zero-test nextest:** recent nextest versions exit non-zero when no tests run. In M1.1 (before the canaries) this could break `test`. Check whether `--no-tests=pass` is needed or whether M1.1 and M1.2 must merge together.
- **`resolver = "3"` + `rust-version = "1.85"`:** confirm that the chosen `proptest`/`insta` minors don't need a newer rustc. Otherwise raise `rust-version`, which also affects the `msrv` job (D2).
- **`multiple-versions = "warn"`:** is this too lax? `deny` would block M1.3 (wgpu/winit pull duplicate `windows-sys`/`bitflags`).
- **`wrappers = ["newell-kernel"]`:** check that cargo-deny's semantics really block an indirect path (for example `newell-model -> newell-kernel -> brepkit` must be allowed, and `newell-ui -> brepkit` must be denied).
- **`.claudesignore` excludes `tests/fixtures/golden/`:** this also hides the README. Should it exclude `*.step`/`*.stl` only (already listed)?
- **Error type naming:** `ARCHITECTURE.md` §5 says `NewellError` and §5.1 says `NewellKernelError`. This doesn't matter for M1, but M3.1 will need one answer. Flag it to the human.
- **Agent count:** `ARCHITECTURE.md` §2 says "4-agent system", while `Description.md` §6.1 lists 5 (it adds Explorer). Not in M1 scope.
- **`RUSTFLAGS=-Dwarnings` globally** also applies to the `msrv` job. Newer lints are absent on 1.85, so this is fine, but confirm it doesn't double-deny with `-D warnings` on clippy.
- **Header check:** it only scans `crates/`. Should it also scan `build.rs` and root-level `.rs` files? None exist in M1.
