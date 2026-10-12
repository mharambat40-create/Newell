# Plan M1.1 + M1.2 (revision 3): Reconciliation with the Current Repository

**Author:** Architect · **Status:** Revised, for Challenger review
**Backlog:** `docs/backlog.md:4` (M1.1), `docs/backlog.md:5` (M1.2). M1.3 (`docs/backlog.md:6`) is out of scope.
**Binding:** `ARCHITECTURE.md` §1, §2, §5, §5.1, §6; `docs/adr/0002-*.md`. Reference: `Description.md` §3.1, §7.1–7.3, §8.7, §10.
**Supersedes:** revision 2. Most of revision 2's audit (A1–A21) is obsolete; the repository already contains the M1.1 workspace and CI.

---

## 0. Approved Decisions

| ID | Decision |
|----|----------|
| D1 | GitHub ruleset on `main`. **Human task**, see §4. |
| D2 | `msrv` and `pr-title` CI jobs: both included. Already present in `ci.yml`. |
| D3 | `thiserror = "2"`. Already present. |
| D4 | Workspace-wide `tests/it/` in all five crates. Tests stay per-crate; no production code added for tests. |
| D5 | `--locked` and `--all-features` where applicable. Already present in `ci.yml`. |
| D6 | Copyright line in `ARCHITECTURE.md` §6 is `Copyright (c) 2026 Martin Harambat`. Resolved. Do not touch `ARCHITECTURE.md`. |
| D7 | Backlog M1.1 and M1.2 stay `[ ]` until verification passes and the human approves. |

Toolchain: Rust 1.99.0 is installed at `~/.cargo/bin` (not on the shell PATH). Every command below assumes `export PATH="$HOME/.cargo/bin:$PATH"`.

---

## 1. Reconciliation: Verified State

Baseline: HEAD `cc85aab` plus the working tree as of 2026-10-12. Items marked **Done** need no work. Line references are to the working tree.

| Requirement | State | Evidence |
|-------------|-------|----------|
| Workspace with five members, `resolver = "3"` | Done | `Cargo.toml` `[workspace]` |
| `[workspace.package]` edition 2024, rust-version 1.85, AGPL-3.0-or-later | Done | `Cargo.toml` `[workspace.package]` |
| Shared lints in `[workspace.lints]` | Done | `Cargo.toml` |
| Every crate inherits workspace package and lints | Done | `crates/*/Cargo.toml` |
| `thiserror = "2"` in `[workspace.dependencies]`, unused by any crate | Done | `Cargo.toml`; no crate depends on it |
| `Cargo.lock` committed | Done | `git ls-files` |
| `rust-toolchain.toml` | Done (channel `stable`) | `rust-toolchain.toml` |
| `clippy.toml` unwrap/expect exemptions | Done, with one extra line (§2.3) | `clippy.toml` |
| `.gitignore` ignores `*.snap.new`, `*.pending-snap`, `target/` | Done | `.gitignore` |
| `.claudesignore` | Done (English, `Cargo.lock` excluded, archive excluded) | `.claudesignore` |
| `ci.yml`: least-privilege `permissions`, SHA-pinned actions, `fail-fast: false`, concurrency | Done | `.github/workflows/ci.yml` |
| `ci.yml`: fmt, clippy (`--all-features --locked`), test on 3 OSes with `--no-tests=pass` and doctests, aarch64, doc, deny, msrv (1.85), pr-title | Done | `.github/workflows/ci.yml` |
| AGPL 6-line header on every `.rs` stub | Done | `crates/*/src/*.rs` |
| `deny.toml` licence allow-list, advisories | Done (partial, §2.4) | `deny.toml` |
| Local gates pass: `fmt --check`, `clippy -D warnings`, `cargo deny check`, `nextest` with zero tests | Done (verified 2026-10-12) | this session |

The old audit items A1–A21 are closed, except those listed in §2.

---

## 2. Remaining Gaps (M1.1 and M1.2)

### M1.1 — Workspace, `.claudesignore`, CI/CD

| # | Gap | Change | Who |
|---|-----|--------|-----|
| G1 | `.gitattributes` missing; snapshots must be byte-identical on Windows | Create `.gitattributes` (§2.1) | Executor |
| G2 | Crate manifests have no `description` | Add one line per crate (§2.2) | Executor |
| G3 | `newell-app` binary is named `newell-app`, not `newell` | Add `[[bin]]` (§2.2) | Executor |
| G4 | `Cargo.toml` had a stray comment `§1.1: to confirm in review` | **Applied** (working tree) | Executor |
| G5 | `deny.toml` lacks `[bans]` and `[sources]` rules required by `ARCHITECTURE.md` §5 and §5.1 | Add them (§2.3) | Executor |
| G6 | `ci.yml` has no licence-header job (`ARCHITECTURE.md` §6 Auditor Rule) | **Human patch** (§4) | Human |

### M1.2 — Testing architecture

| # | Gap | Change | Who |
|---|-----|--------|-----|
| G7 | `proptest = "1.4"`, `insta = "1.38"` were outdated minimums | **Applied** (working tree): `1.12`, `1.49` | Executor |
| G8 | No `[dev-dependencies]` in any crate | Add `proptest.workspace = true`, `insta.workspace = true` to all five crates | Executor |
| G9 | No `tests/it/` in any crate | Create `tests/it/{main.rs,harness.rs}` per crate (§3) | Executor |
| G10 | No committed insta snapshot | Create one per crate (§3.3) | Executor |
| G11 | `clippy.toml:3` sets `allow-panic-in-tests = true`, which neither the plan nor `Description.md` §7.1 requires | Remove that line | Executor |
| G12 | `ci.yml` does not set `INSTA_UPDATE=no` | **Human patch** (§4) | Human |
| G13 | `tests/fixtures/golden/` does not exist | Create `README.md` only (§3.4) | Executor |

Dropped from revision 2 (not required by `Description.md` or the backlog):
- `[profile.dev.package.insta|similar]` tuning: no measured need.
- The blanket `#![allow(clippy::unwrap_used, ...)]` in `tests/it/main.rs`: `clippy.toml` already exempts `#[test]` code, and no helper outside a test exists. Keeping the allow would widen the exception for no benefit. If clippy rejects a helper, revisit.

---

## 3. Work Items for the Executor

### 3.1 Manifests

- `Cargo.toml`: G4 and G7 are applied (see §2).
- Each `crates/<name>/Cargo.toml`: add `description` (one line, from the table below) after `version`; add:
  ```toml
  [dev-dependencies]
  proptest.workspace = true
  insta.workspace = true
  ```
- `crates/newell-app/Cargo.toml`: add
  ```toml
  [[bin]]
  name = "newell"
  path = "src/main.rs"
  ```

| Crate | `description` |
|-------|---------------|
| `newell-kernel` | Anti-corruption layer over the BrepKIT kernel. |
| `newell-model` | Parametric feature tree and parameters. |
| `newell-ui` | Flat cell grid, O(1) hit-testing, per-window UI state. |
| `newell-render` | wgpu rendering of the ASCII grid and 3D viewport. |
| `newell-app` | Binary entry point and window manager. |

No inter-crate dependencies. `brepkit` is not declared anywhere (`ARCHITECTURE.md` §5.1; first use is M3.1).

### 3.2 Repository files

- `.gitattributes` (new):
  ```
  * text=auto eol=lf
  *.png binary
  *.ttf binary
  *.stl binary
  *.step binary
  ```
- `clippy.toml`: delete the `allow-panic-in-tests` line (G11, applied). The file keeps `allow-unwrap-in-tests` and `allow-expect-in-tests`.
- `.claudesignore`: add `tests/fixtures/` so agents do not read golden data (applied). This also hides `tests/fixtures/golden/README.md` from agents; the README is for humans and the Executor reads it only when a kernel test needs it.
- `deny.toml`: add, keeping the existing `[licenses]` and `[advisories]` tables:
  ```toml
  [bans]
  multiple-versions = "warn"
  wildcards = "deny"
  deny = [
      { crate = "opencascade" },
      { crate = "opencascade-sys" },
      { crate = "brepkit", wrappers = ["newell-kernel"] },
  ]

  [sources]
  unknown-registry = "deny"
  unknown-git = "deny"
  ```
  Add `private = { ignore = true }` under `[licenses]`. `truck-*` is not added: cargo-deny's crate matching is exact, and no truck crate is planned. Revisit in M3.1 if a wildcard form is confirmed.
  Decision: `multiple-versions = "warn"`, not `deny`. `Description.md` §7.6 names duplicate dependencies as a `cargo-deny` purpose; `deny` would block M1.3 (wgpu/winit pull duplicate `windows-sys`/`bitflags`). Revisit in M3.1.

### 3.3 Testing architecture (all five crates)

Layout per crate, identical in each:
```
crates/<crate>/tests/it/
├── main.rs        licence header, then `mod harness;`
├── harness.rs     three canary tests, licence header
└── snapshots/
    └── it__harness__insta_snapshot_is_wired.snap
```

`main.rs` (after the 6-line header):
```rust
mod harness;
```

`harness.rs` (after the 6-line header). All data is local to the test file; no production item is added. Literal `Some(1u8).unwrap()` is rejected by `clippy::unnecessary_literal_unwrap` under `-D warnings`, and a struct whose fields are never read fails `dead_code`, so the canaries use a parsed value and a tuple.
```rust
use proptest::prelude::*;

#[test]
fn insta_snapshot_is_wired() {
    insta::assert_debug_snapshot!((1u32, "canary"));
}

proptest! {
    #[test]
    fn proptest_is_wired(x in any::<i64>()) {
        prop_assert_eq!(x.to_string().parse::<i64>(), Ok(x));
    }
}

#[test]
fn unwrap_allowed_in_tests() {
    assert_eq!("1".parse::<u8>().unwrap(), 1);
}
```

Snapshot creation (done once, 2026-10-12): `INSTA_UPDATE=always cargo nextest run --workspace --all-features --locked`. Each `.snap` body is the pretty-printed tuple `(\n    1,\n    "canary",\n)`. The `source:` header differs per crate (its path); the body is identical. No `.snap.new` may remain. Commit the five `.snap` files.

### 3.4 Golden-fixture contract

`tests/fixtures/golden/README.md` (new). Content:
- Naming: `<feature>_<case>.<ext>`.
- Each fixture has a paired insta snapshot of derived invariants (volume, face/edge/vertex counts, bounding box at fixed rounding) in `crates/newell-kernel/tests/it/snapshots/`.
- A `brepkit` version bump requires `cargo nextest run -p newell-kernel`, and any snapshot change requires human review.
- Agents do not read fixtures (`.claudesignore`).
- Kernel tests reference fixtures via `concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/golden/<file>")`.

No fixture files, no kernel tests and no `brepkit` in M1.

---

## 4. Human Tasks (not executed by agents)

`.claude/settings.json` denies Edit and Write on `.github/**` and `docs/adr/**`, and `ARCHITECTURE.md` is human-owned. The following are for you.

**4.1 CI patch to `.github/workflows/ci.yml`** (G6, G12)

- In the `test` job, add a job-level env:
  ```yaml
  env:
    INSTA_UPDATE: "no"
  ```
- Add a `license-header` job:
  ```yaml
  license-header:
    name: AGPL header
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4
      - name: Check licence header
        shell: bash
        run: |
          header=$(mktemp)
          awk '/^## 6\./{s=1} s && /^```rust/{f=1; next} f && /^```/{exit} f' ARCHITECTURE.md > "$header"
          test "$(wc -l < "$header")" -eq 6
          status=0
          while IFS= read -r -d '' file; do
            if ! head -n 6 "$file" | tr -d '\r' | cmp -s - "$header"; then
              echo "::error file=$file::AGPL header missing or altered"
              status=1
            fi
          done < <(find crates -name '*.rs' -print0)
          rm -f "$header"
          exit $status
  ```
  The script is checked locally in §5.4 before you apply it. It writes its temp file with `mktemp`, so nothing lands in the repository.
- Rulesets (D1): after the CI checks above are green on a PR, in GitHub → Settings → Rules → Rulesets → New branch ruleset: target `main`; enforce; require a pull request; require status checks `Rustfmt`, `Clippy`, `Tests (nextest) (ubuntu-latest)`, `Tests (nextest) (macos-latest)`, `Tests (nextest) (windows-latest)`, `Build (Raspberry Pi / aarch64)`, `Documentation`, `Cargo Deny (Licenses & Advisories)`, `MSRV (1.85)`, `PR title`, and `AGPL header`; block force pushes; block deletions. The matrix job names are the ones GitHub shows on the PR; confirm them there before saving the ruleset.

**4.2 Backlog (D7):** tick M1.1 and M1.2 in `docs/backlog.md` only after §5 passes and you approve.

**4.3 MEMORY.md:** stale "Current Task" (A20). Update after merge (Q6).

---

## 5. Verification

### 5.1 Commands (each must exit 0)

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo check --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
INSTA_UPDATE=no cargo nextest run --workspace --all-features --locked --no-tests=pass
cargo test --workspace --doc --locked
cargo doc --workspace --no-deps --document-private-items --all-features --locked   # RUSTDOCFLAGS="-D warnings"
cargo deny check
cargo run --release                                  # exits 0 immediately
```

### 5.2 Acceptance criteria

1. Every command in §5.1 exits 0 locally.
2. Each of the five crates runs its three canaries, reported by nextest.
3. Deleting a committed `.snap` makes the `INSTA_UPDATE=no` run fail.
4. Changing a canary's `label` fails and leaves a `.snap.new`, which git ignores.
5. `proptest`, `insta` and `thiserror` appear only under `[workspace.dependencies]` in the root and as `.workspace = true` entries in crates.
6. `grep -rn brepkit Cargo.toml crates/*/Cargo.toml` prints nothing.
7. `grep -rn '@v\|@stable\|@nextest' .github/workflows/ci.yml` prints nothing (already true).
8. `Cargo.lock` is current: `cargo check --workspace --locked` passes with the committed lock (verified 2026-10-12, after the dev-dependency change).
8. CI is green after the §4.1 patch (checked by you on the PR).

### 5.3 Mutation checks (manual, reverted after)

- Add `pub fn f() { None::<u8>.unwrap(); }` to `crates/newell-ui/src/lib.rs`: clippy fails.
- Add `unsafe {}` to any lib: `cargo check` fails.
- Remove one header line from a `.rs`: the header script fails.

### 5.4 Header script check

Run the §4.1 script locally with its temp file in `mktemp`, not in the repository. It must exit 0 on the current tree and exit 1 after a single header line is altered in a temporary copy (verified 2026-10-12).

---

## 6. Test List

| # | Test / check | Kind | Degenerate case |
|---|--------------|------|-----------------|
| T1 | `insta_snapshot_is_wired` ×5 | integration | Missing snapshot fails under `INSTA_UPDATE=no` |
| T2 | `proptest_is_wired` ×5 | property | Default strategy covers `0`, `i64::MIN`, `i64::MAX` |
| T3 | `unwrap_allowed_in_tests` ×5 | clippy canary | `clippy --all-targets -D warnings` accepts unwrap in test code |
| T4 | nextest on a crate with zero real tests | CI/local | Canaries keep the count above zero; `--no-tests=pass` covers the rest |
| T5 | `cargo test --doc` with zero doctests | local | Passes |
| T6 | Header script on a CRLF file | §5.4 | `\r` stripped before comparison |
| T7 | Header script on an empty or short file | §5.4 | Fails, does not pass |
| T8 | Header script on a UTF-8 BOM | §5.4 | Fails; the BOM breaks byte equality (policy) |

---

## 7. Out of Scope and Open Notes

- **M1.3** (`winit` window, `wgpu` surface) is not started.
- **`bench`** job: deferred (nothing to benchmark before M2).
- **`rust-toolchain.toml`** pins `channel = "stable"`. `Description.md` §3.1 says "pinned toolchain". This may need an exact version; it is a human decision, not changed here.
- **`cargo deny` warnings** (`license-not-encountered` for `Unicode-3.0`, `Zlib`): intentional allowances for later dependencies. They do not fail the check.
- **Error type naming** (`NewellError` vs `NewellKernelError`, `ARCHITECTURE.md` §5 and §5.1): unresolved, first needed in M3.1. Flag to the human then.
- **Header check scope**: `crates/` only. `build.rs` and root `.rs` files do not exist yet.
