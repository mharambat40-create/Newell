# Plan M1.1 + M1.2: Workspace Skeleton, CI/CD, and Testing Architecture

**Author:** Architect
**Backlog items:** M1.1 (workspace, `.claudesignore`, CI/CD), M1.2 (workspace-wide testing architecture)
**Status:** Draft, waiting for Challenger review
**Binding constraints:** `ARCHITECTURE.md` §1 (crate split), §5 (no `unsafe`, no `unwrap`/`expect`/`panic!`), §5.1 (BrepKIT only in `newell-kernel`, golden files in `tests/fixtures/`), §6 (AGPL header)

---

## 0. Current State Audit (why M1.1 is reopened)

`docs/backlog.md` marks M1.1 as done, but the repository does not match:

| # | Finding | Impact |
|---|---------|--------|
| A1 | `crates/` does not exist, but `Cargo.toml` lists 5 members. | Every `cargo` command fails ("failed to load manifest for workspace member"). |
| A2 | `proptest`, `insta`, `thiserror` sit inside `[workspace.lints.clippy]`. | Cargo reads them as lints `clippy::proptest = "1.4"`, an invalid lint level, and rejects the manifest. They must move to `[workspace.dependencies]`. |
| A3 | `thiserror = "1.0"`. | 2.x is the current major. New code starts on 2.x. |
| A4 | No `[workspace.package]`, no `edition`, no `license`. | Crates would default to edition 2015 and have no SPDX license. `cargo-deny` cannot classify our own crates. |
| A5 | No crate opts into workspace lints (`[lints] workspace = true`). | The lints in `[workspace.lints]` apply to nothing. |
| A6 | CI `deny` job runs `cargo deny check` as a plain `run:` step after `cargo-deny-action`. | The action runs in Docker and does not put `cargo-deny` on `PATH`, so the step fails with `no such command: deny`. There is also no `deny.toml`, so the default config rejects every license. |
| A7 | Clippy denies `unwrap_used`/`expect_used`/`panic` with `--all-targets`. | Test code (`#[cfg(test)]`, `tests/`) would fail Clippy. Needs `clippy.toml` plus an explicit allow in integration-test crates (§5.1 bans these in *production* code only). |
| A8 | Nextest does not run doctests. | Doc examples would never run in CI. |
| A9 | Comments in `Cargo.toml` are in French. | Project docs are in English (architect agent rule). |
| A10 | `cargo` / `rustc` are not installed on the dev machine. | The Executor cannot run any checks until the toolchain is installed (§1 step 0). |

`.claudesignore` already exists and is correct. Leave it unchanged.

---

## 1. Folder Structure and Commands

### 1.1 Target tree after M1.1 + M1.2

```
Newell/
├── .config/
│   └── nextest.toml              # NEW: nextest profiles (default + ci)
├── .github/
│   └── workflows/
│       └── ci.yml                # REWRITTEN (§3)
├── .gitattributes                # NEW: force LF (insta snapshots stay stable on the Windows runner)
├── .gitignore                    # UPDATED: insta pending files
├── .claudesignore                # unchanged
├── Cargo.toml                    # REWRITTEN (§2.1)
├── Cargo.lock                    # GENERATED and COMMITTED (binary app, CI uses --locked)
├── clippy.toml                   # NEW: test-code exemptions
├── deny.toml                     # NEW: license/advisory/ban policy
├── rust-toolchain.toml           # NEW: stable + rustfmt + clippy
├── crates/
│   ├── newell-app/               # Controller: binary, window manager (M1.3+)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   ├── newell-render/            # View: wgpu (M1.3+)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   ├── newell-ui/                # Controller: flat cell grid, hit-testing (M2+)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   ├── newell-model/             # Model: feature tree, parameters
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   └── lib.rs
│   │   └── tests/
│   │       └── it/
│   │           ├── main.rs       # integration-test binary root
│   │           ├── harness.rs    # canary tests (proptest + insta)
│   │           └── snapshots/
│   │               └── it__harness__insta_snapshot_is_wired.snap
│   └── newell-kernel/            # Model: BrepKIT ACL (M3.1 adds brepkit)
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs
├── tests/
│   └── fixtures/
│       └── golden/
│           └── README.md         # documents the golden-file contract (§4.4)
└── docs/plans/M1-setup.md        # this file
```

### 1.2 Commands (Executor runs them in this order)

**Step 0: install the toolchain (one-time, on the human's machine).**
```sh
# Install rustup from https://rustup.rs, then:
rustup toolchain install stable --component rustfmt clippy
cargo install --locked cargo-nextest cargo-insta cargo-deny
```

**Step 1: create the crates.** Run these from the repo root. Cargo will warn about the workspace until all five directories exist. That is expected.
```sh
cargo new --vcs none --lib crates/newell-kernel
cargo new --vcs none --lib crates/newell-model
cargo new --vcs none --lib crates/newell-ui
cargo new --vcs none --lib crates/newell-render
cargo new --vcs none --bin crates/newell-app
mkdir -p crates/newell-model/tests/it tests/fixtures/golden
```
If `cargo new` fails instead of warning (the behavior depends on the Cargo version), create the directories and files by hand with the contents in §2. The result is the same.

**Step 2: overwrite the generated files** with the exact contents in §2 and §4. Replace every generated `Cargo.toml`, `lib.rs` and `main.rs`, because the generated `lib.rs` contains an `add()` example and an `assert_eq!` test.

**Step 3: verify locally** (each command must pass before the next):
```sh
cargo check --workspace --all-targets --locked     # first run: drop --locked to generate Cargo.lock
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo insta test --workspace --test-runner nextest  # creates the .snap.new for the canary
cargo insta review                                  # accept the canary snapshot
cargo nextest run --workspace --locked
cargo test --doc --workspace --locked
cargo doc --workspace --no-deps --document-private-items
cargo deny check
cargo run --release                                  # must exit 0 immediately (empty main)
```

---

## 2. Manifests and Config Files

### 2.1 Root `Cargo.toml`

```toml
[workspace]
members = [
    "crates/newell-app",
    "crates/newell-render",
    "crates/newell-ui",
    "crates/newell-model",
    "crates/newell-kernel",
]
# Virtual workspaces must set the resolver explicitly. "3" = MSRV-aware resolution.
resolver = "3"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
license = "AGPL-3.0-or-later"
publish = false

# Shared dependency versions. Crates opt in with `<dep>.workspace = true`.
# RULE (ARCHITECTURE.md §5.1): `brepkit` must NEVER appear here.
# It is declared, pinned with `=x.y.z`, only in crates/newell-kernel/Cargo.toml.
[workspace.dependencies]
# Error handling (ARCHITECTURE.md §5, §5.1)
thiserror = "2.0"

# Test-only
proptest = "1.6"
insta = "1.43"

# Auditor rules enforced across the whole project (ARCHITECTURE.md §5).
[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"

# insta recommends optimizing its diffing deps even in debug builds.
[profile.dev.package.insta]
opt-level = 3

[profile.dev.package.similar]
opt-level = 3
```

Version note: `"2.0"`, `"1.6"`, `"1.43"` are caret minimums. Before committing, the Executor runs `cargo search thiserror proptest insta --limit 1` and raises each minimum to the latest published minor. Keep the caret form (no `=`). Exact pinning applies only to `brepkit`.

Decision: `thiserror` is declared in the workspace but **no crate depends on it yet**. M1 has no fallible operations, and an empty or placeholder error enum would be speculative API. The first consumers are `newell-ui` (M2.1) and `newell-kernel`'s `NewellKernelError` (M3.1).

### 2.2 Per-crate manifests

All four library crates use the same template. Only `name` and `description` change.

`crates/newell-model/Cargo.toml`:
```toml
[package]
name = "newell-model"
description = "Newell Model: parametric feature tree and parameters (single source of truth)."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]

[dev-dependencies]
proptest.workspace = true
insta.workspace = true

[lints]
workspace = true
```

| Crate | `description` |
|-------|---------------|
| `newell-kernel` | `"Newell Model: anti-corruption layer over the BrepKIT geometric kernel."` |
| `newell-model` | (above) |
| `newell-ui` | `"Newell Controller: flat 1D cell grid, O(1) hit-testing, per-window UI state."` |
| `newell-render` | `"Newell View: wgpu rendering of the ASCII grid and 3D viewport."` |

`crates/newell-app/Cargo.toml`:
```toml
[package]
name = "newell-app"
description = "Newell Controller: binary entry point and window manager."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[[bin]]
name = "newell"
path = "src/main.rs"

[dependencies]

[dev-dependencies]
proptest.workspace = true
insta.workspace = true

[lints]
workspace = true
```

Decision: **no inter-crate dependencies in M1.** The MVC dependency graph (which crate imports which) gets wired in M1.3/M4.1, when real types exist.

### 2.3 Source stubs

Every `.rs` file starts with the AGPL header from ARCHITECTURE.md §6, verbatim. A crate-level doc comment follows it. No items, no placeholder functions.

`crates/<lib-crate>/src/lib.rs`:
```rust
// <AGPL header from ARCHITECTURE.md §6, verbatim>

//! <same sentence as the crate's `description`>
```

`crates/newell-app/src/main.rs`:
```rust
// <AGPL header from ARCHITECTURE.md §6, verbatim>

//! Newell binary entry point. Window creation lands in M1.3.

fn main() {}
```

### 2.4 `rust-toolchain.toml`
```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

### 2.5 `clippy.toml`
```toml
# ARCHITECTURE.md §5 bans unwrap/expect/panic in *production* code.
# Tests may use them: a panic is how a test fails.
# These options cover `#[test]` functions and `#[cfg(test)]` modules only.
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
```
Integration-test helper functions (non-`#[test]` code under `tests/`) are not covered by `clippy.toml`. Each `tests/it/main.rs` therefore carries a crate-level allow (§4.2). That allow is scoped to the test binary and can never leak into production code.

### 2.6 `deny.toml`
```toml
[graph]
all-features = true

[advisories]
yanked = "deny"

[licenses]
# Every license here must be AGPL-3.0-compatible.
allow = [
    "AGPL-3.0-or-later",
    "AGPL-3.0-only",                  # BrepKIT (M3.1); verify the exact SPDX id then
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "Unicode-3.0",
]
confidence-threshold = 0.9
# Our own crates are publish = false.
private = { ignore = true }

[bans]
multiple-versions = "warn"
wildcards = "deny"
allow-wildcard-paths = true
deny = [
    # ARCHITECTURE.md §5.1: only newell-kernel may depend on brepkit directly.
    { crate = "brepkit", wrappers = ["newell-kernel"] },
    # ARCHITECTURE.md §5 / MEMORY.md: BrepKIT is the only kernel allowed.
    { crate = "opencascade" },
    { crate = "opencascade-sys" },
    { crate = "truck-modeling" },
    { crate = "truck-topology" },
]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```
This gives CI a machine check of the §5.1 isolation rule. Today `brepkit` is absent, so the rule passes trivially. From M3.1 it fails the build if any other crate imports `brepkit`.

### 2.7 `.config/nextest.toml`
```toml
[profile.default]
slow-timeout = { period = "30s", terminate-after = 4 }

[profile.ci]
fail-fast = false
failure-output = "immediate-final"
slow-timeout = { period = "60s", terminate-after = 2 }
```

### 2.8 `.gitattributes`
```
* text=auto eol=lf
*.png binary
*.ttf binary
*.stl binary
*.step binary
```

### 2.9 `.gitignore` (full content)
```
.DS_Store
target/
# insta pending snapshots (accept with `cargo insta review`, never commit)
*.snap.new
*.pending-snap
```

---

## 3. `.github/workflows/ci.yml` (full replacement)

```yaml
name: Newell CI

on:
  push:
    branches: ["main"]
  pull_request:
    branches: ["main"]

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always
  CARGO_INCREMENTAL: 0
  RUSTFLAGS: "-Dwarnings"

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

jobs:
  fmt:
    name: Rustfmt
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - name: Check formatting
        run: cargo fmt --all -- --check

  license-header:
    name: AGPL header (ARCHITECTURE.md §6)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Every .rs file must start with the AGPL header
        run: |
          fail=0
          while IFS= read -r -d '' f; do
            if [ "$(head -n 1 "$f")" != "// Newell - Parametric CAD Software" ]; then
              echo "::error file=$f::missing AGPL-3.0 header"
              fail=1
            fi
          done < <(find crates -name '*.rs' -print0)
          exit $fail

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - name: Run Clippy
        run: cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

  test:
    name: Tests (${{ matrix.os }})
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    env:
      # Never write snapshots in CI: a missing or changed snapshot fails the job.
      INSTA_UPDATE: "no"
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@nextest
      - name: Unit + integration tests
        run: cargo nextest run --workspace --all-features --locked --profile ci
      - name: Doctests (nextest does not run them)
        run: cargo test --doc --workspace --all-features --locked

  aarch64:
    name: Check (Raspberry Pi / aarch64)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-unknown-linux-gnu
      - uses: Swatinem/rust-cache@v2
      # `cargo check` does not link. The cross linker is installed now so that
      # M1.3 (wgpu/winit) can upgrade this job to `cargo build` without changes.
      - name: Install cross linker
        run: sudo apt-get update && sudo apt-get install -y gcc-aarch64-linux-gnu
      - name: Check build for aarch64
        run: cargo check --workspace --target aarch64-unknown-linux-gnu --locked

  doc:
    name: Documentation
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Build documentation
        run: cargo doc --workspace --no-deps --document-private-items --locked
        env:
          RUSTDOCFLAGS: "-D warnings"

  deny:
    name: Cargo Deny (licenses, advisories, bans, sources)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
        with:
          command: check
          arguments: --all-features
```

Changes from the current file:
- Fixed the broken `deny` step (A6) and added `deny.toml`.
- New `license-header` job: a machine check for ARCHITECTURE.md §6.
- Doctests now run (A8).
- `--locked` everywhere: CI fails if `Cargo.lock` is stale.
- `INSTA_UPDATE=no`: snapshots are never written in CI.
- `fail-fast: false` on the test matrix: one OS failing does not hide the others.
- `permissions: contents: read` (least privilege).

---

## 4. Testing Architecture

### 4.1 Three test tiers

| Tier | Location | Purpose | Tools |
|------|----------|---------|-------|
| Unit | `#[cfg(test)] mod tests` at the bottom of the file under test | Private functions, invariants, degenerate inputs | `assert!`, `proptest!` |
| Integration | `crates/<crate>/tests/it/*.rs` | The crate's public API only, black-box | `proptest!`, `insta` |
| Golden (geometric regression) | Data in `tests/fixtures/golden/`, tests in `crates/newell-kernel/tests/it/golden.rs` (M3.1+) | ARCHITECTURE.md §5.1: BrepKIT upgrades must reproduce the reference models | `insta` + fixtures |

GPU-dependent code (`newell-render`) is **not** tested through a real GPU in CI. Its testable logic (UV math, buffer layout) must be pure functions, unit-tested on the CPU. Policy for M1.3+.

### 4.2 Integration-test layout: one binary per crate

Cargo compiles each top-level `tests/*.rs` file as a separate binary, which costs link time and cache space. Convention: **one** test binary per crate, auto-discovered at `tests/it/main.rs`. New files are added as modules.

```
crates/<crate>/
├── src/
│   └── foo.rs                    # unit tests: #[cfg(test)] mod tests { use super::*; ... }
├── tests/
│   └── it/
│       ├── main.rs               # crate-level allow + `mod` declarations only
│       ├── <feature>.rs          # one module per public-API area
│       ├── common/
│       │   └── mod.rs            # shared helpers/strategies (create when first needed)
│       └── snapshots/            # insta output, COMMITTED
│           └── it__<module>__<test_name>.snap
└── proptest-regressions/ or tests/it/*.proptest-regressions   # proptest failure seeds, COMMITTED
```

Rules:
1. Do **not** create `tests/foo.rs` at the top level. Always `tests/it/foo.rs` plus `mod foo;` in `main.rs`.
2. `tests/it/main.rs` always starts with the AGPL header, then:
   ```rust
   #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
   ```
3. Snapshots are reviewed with `cargo insta review` and committed. `.snap.new` / `.pending-snap` files are never committed (`.gitignore`).
4. proptest regression files are committed: they pin previously found counterexamples.
5. Create `tests/it/` in a crate when it gets its first public API (M2.1 for `newell-ui`, M3.1 for `newell-kernel`). M1.2 creates it only in `newell-model`, as the canary.
6. `newell-app` is a binary, so integration tests cannot import it. Keep `main.rs` thin and push testable logic into library crates.

### 4.3 Canary tests (M1.2 deliverable)

Their purpose is to prove that `proptest`, `insta`, nextest, the Clippy test exemptions and the CI snapshot policy all work end to end before real code exists. All test data is local to the test file. **No production items are added to `newell-model`.** Delete the canary when M2.1/M3.1 add real tests to the workspace.

`crates/newell-model/tests/it/main.rs`:
```rust
// <AGPL header>
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;
```

`crates/newell-model/tests/it/harness.rs`: three tests:

| Test | What it does | Proves |
|------|--------------|--------|
| `insta_snapshot_is_wired` | `insta::assert_debug_snapshot!` on a test-local `#[derive(Debug)] struct Canary { id: u32, label: &'static str }` built with fixed values | insta writes/reads `tests/it/snapshots/it__harness__insta_snapshot_is_wired.snap`; CI fails if the snapshot is missing |
| `proptest_is_wired` | `proptest! { #[test] fn proptest_is_wired(x in any::<i64>()) { prop_assert_eq!(x.to_string().parse::<i64>(), Ok(x)); } }` | proptest runs, and covers the degenerate values `i64::MIN`, `i64::MAX`, `0` through its default strategy |
| `unwrap_allowed_in_tests` | `let v: Option<u8> = Some(1); assert_eq!(v.unwrap(), 1);` | The `clippy.toml` + crate-level allow pass `clippy --all-targets -D warnings` |

### 4.4 Golden fixtures: `tests/fixtures/golden/`

The root `tests/` is **not** a Cargo package (the workspace is virtual). It only holds data, at the path ARCHITECTURE.md §5.1 mandates. Kernel tests reference it with:
```rust
concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/golden/<file>")
```
M1.2 only creates `tests/fixtures/golden/README.md`, stating:
- the file naming scheme: `<feature>_<case>.<ext>` (e.g. `boolean_union_two_boxes.step`);
- each fixture is paired with an insta snapshot of derived invariants (volume, face/edge/vertex counts, bounding box, rounded to a fixed tolerance) in `crates/newell-kernel/tests/it/snapshots/`;
- any `brepkit` version bump must run `cargo nextest run -p newell-kernel` and must not accept snapshot changes without a human review;
- agents must not read these files (`.claudesignore` excludes `*.stl`/`*.step`).

### 4.5 Local developer workflow

```sh
cargo nextest run --workspace                    # all tests
cargo nextest run -p newell-model                # one crate
cargo insta test --workspace --test-runner nextest && cargo insta review   # after a snapshot change
PROPTEST_CASES=10000 cargo nextest run -p <crate>                          # deep property run before a release
```

---

## 5. Files Touched (Executor checklist)

| Action | Path |
|--------|------|
| Rewrite | `Cargo.toml` |
| Rewrite | `.github/workflows/ci.yml` |
| Rewrite | `.gitignore` |
| Create | `.gitattributes`, `clippy.toml`, `deny.toml`, `rust-toolchain.toml`, `.config/nextest.toml` |
| Create | `crates/newell-{kernel,model,ui,render}/Cargo.toml` + `src/lib.rs` |
| Create | `crates/newell-app/Cargo.toml` + `src/main.rs` |
| Create | `crates/newell-model/tests/it/main.rs`, `harness.rs`, `snapshots/it__harness__insta_snapshot_is_wired.snap` |
| Create | `tests/fixtures/golden/README.md` |
| Generate + commit | `Cargo.lock` |
| Do not touch | `ARCHITECTURE.md`, `.claudesignore`, `LICENSE` |

## 6. Acceptance Criteria

1. Every command in §1.2 step 3 exits 0 on macOS.
2. CI on the PR is green on all jobs, all three OSes, plus aarch64.
3. Deleting the committed `.snap` file makes the CI `test` job fail (proves `INSTA_UPDATE=no`).
4. Adding `fn f() { None::<u8>.unwrap(); }` to any `src/lib.rs` makes `clippy` fail (proves production lints are live).
5. A `.rs` file without the header makes `license-header` fail.
6. `grep -r brepkit crates/*/Cargo.toml Cargo.toml` returns nothing.

## 7. Open Questions for the Human (blocking)

1. **Copyright holder.** The ARCHITECTURE.md §6 header still says `[Your Name/Company]`. The Executor cannot add a header with a placeholder. Please update ARCHITECTURE.md with the real name (Architect is not allowed to edit it).
2. **Toolchain.** `cargo`/`rustc` are not installed on this machine (A10). Please run §1.2 step 0.
3. **Backlog.** M1.1 is ticked but not done (§0). Untick it, or tick both items when this plan lands.

## 8. Notes for the Challenger

- `allow-panic-in-tests` needs a recent Clippy. If the pinned stable rejects the key, remove it and rely on the crate-level allow in `tests/it/main.rs`. Unit tests in `src/` don't call `panic!` directly.
- `deny.toml` uses `crate = ...` syntax in `[bans]`, which needs cargo-deny ≥ 0.14. The v2 action ships the latest.
- Is `multiple-versions = "warn"` too lax? wgpu/winit pull duplicate `windows-sys`/`bitflags` versions, so `deny` would block M1.3.
- `resolver = "3"` + `rust-version = "1.85"`: confirm that no dev-dependency raises the MSRV above 1.85. Otherwise bump `rust-version`.
