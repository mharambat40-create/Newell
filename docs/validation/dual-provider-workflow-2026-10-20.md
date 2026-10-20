# Shared Claude/Codex workflow and UI regression correction — 2026-10-20

## Authorization and baseline

The human requested correction of the workflow inconsistencies and full role
parity between Claude and ChatGPT through Codex. The human also explicitly
included UI corrections according to canonical specifications.

Baseline: `a13ffce0e235f884cb5b8eee5dfafeb8ed6248a2`, plus the pre-existing
staged, unstaged and untracked working tree. HEAD alone does not contain those
inputs. Before edits, content hashes of 227 files, the full staged binary diff
and repository status were recorded under `/tmp/newell-workflow-Rwkv6Y/`.
Temporary files are local diagnostic evidence, not a durable source of rules.

The baseline rerun reproduced **698 passed, 38 failed, 9 ignored** using
`INSTA_UPDATE=no cargo test --workspace --all-features --locked --no-fail-fast`.
The 38 failures match the categories reported on 2026-10-19: one app playhead
test, six list/overlay unit tests and 31 UI integration checks/snapshots.

## Shared workflow

- `AGENTS.md` and `CLAUDE.md` now load `docs/agents/README.md` and only the
  selected role. Architect, Challenger, Executor, Auditor and optional Explorer
  are available to both providers.
- The five Claude adapters load the same role documents. Executor again has an
  implementation mission; Challenger and Explorer have explicitly read-only
  Claude tool lists. Existing model choices are retained for Claude only.
- ARCHITECTURE.md §2.4 records human-assigned parallel work, separate writing
  checkouts, explicit baselines/plan revisions, independent plan paths and
  human-controlled integration. No stage dispatches another automatically.
- The shared guide includes a task/handoff contract, verification commands and
  a copyable role prompt. It distinguishes Codex instructions from Claude's
  configuration and explains providing inputs to plain ChatGPT without repo access.
- MEMORY.md, README.md and Description.md reflect the current state and role
  parity. Historical reports remain unchanged.
- CI's MSRV job now installs Rust 1.88, matching Cargo.toml and accepted ADR 0004.
  No dependency versions or tool permission settings changed.

## UI corrections

Corrections restore the established M2 reference behavior while preserving M3
camera ownership and M4 model-binding APIs:

- Restore the `3D Viewport` reference caption and exact View shading labels;
  the longer replacement labels had enlarged and misaligned the View popup.
- Restore both cyclic arrows and wrapping UI Scale behavior. Add the missing
  **140%** preference required by ARCHITECTURE.md §3.1.1. The menu reference,
  README_UI semantics, all monitor example rows and cyclic navigation coverage
  now include all five values. The original four-value tests were obsolete.
- Restore scrollbars for eligible list rectangles, including lists with five
  or fewer items; they must not disappear based on an undocumented item threshold.
- Restore centered Selection/Formula modal routing and source Action Box
  preservation. Formula uses its existing verbatim-text, fixed-width two-row
  caret contract, without injecting an immutable `Fx=` prefix into stored text.
- Restore the planned top-level playhead intervals and reference separator
  grammar instead of splitting a feature subtree with new undocumented states.
- Include centered modal rectangles in scene occlusions without reframing the
  camera. New coverage checks both Selection and Formula with the source panel
  still open.

Tests changed for undocumented in-place modals, hidden cyclic arrows, removed
caption and nested playheads were restored or rewritten against these contracts.
The two previously altered tree snapshots were restored to the existing HEAD
reference contents. No reference snapshot was regenerated or accepted automatically;
other snapshots and `Interface_Exemple.txt` are unchanged.

The 23 pre-existing ignored `.snap.new` diagnostic files were also preserved
byte-for-byte. Successful insta runs remove stale diagnostics; these files were
reconstructed from the original UI inputs in an isolated temporary workspace
and restored only after matching their pre-task SHA-256 hashes. They are old
diagnostics, not evidence of failures in the corrected working tree.

## Final verification

| Check | Result |
|---|---|
| `cargo check --workspace --locked` | Passed |
| `cargo fmt --all -- --check` | Passed; only assigned files were formatted |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Passed |
| `INSTA_UPDATE=no cargo test --workspace --all-features --locked --no-fail-fast` | **737 passed, 0 failed, 9 ignored**; includes doctests |
| `cargo test -p newell-render --locked -- --ignored` | **9 GPU tests passed** |
| `cargo doc --workspace --no-deps --document-private-items --all-features --locked`, `RUSTDOCFLAGS=-D warnings` | Passed |
| `cargo +1.88.0 check --workspace --locked` | Passed on installed Rust 1.88.0 |
| Workflow YAML parse and MSRV job fields | Passed |
| Both entry points, all five shared role links, adapter identity and read-only tool lists | Passed |
| Git staged binary diff vs baseline | Unchanged |
| `git diff --check` | Passed |

All 38 inherited test failures are resolved. One additional regression test
checks centered modal scene occlusion. The existing `block v0.1.6` dependency
still emits its future-Rust-incompatibility notice; strict Clippy passes.

## Remaining acceptance and working state

Automated verification is green. No manual acceptance of physical gestures or
cross-monitor behavior is claimed: retain M2.17–M2.18 native resize/cross-monitor
checks and M3.10 mouse/trackpad/native pinch/visual acceptance.

Existing work and the Git index remain in place. No commit, stash, reset,
branch switch, worktree creation, push or merge was performed. This setup does
not launch agents or register Codex subagents. Start a fresh conversation in
the intended checkout to load the new entry point. Before actual concurrent
writing, prepare the identified baseline in separate checkouts as the shared
guide requires; the current dirty checkout is not itself a parallel workspace.
