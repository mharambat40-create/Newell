# Newell — shared agent workflow

## Roles and authority

Claude and ChatGPT through Codex can each perform every role below. No role is
reserved for a provider. Select one role per task/conversation:

| Role | Instructions | Output | Write scope |
|---|---|---|---|
| Architect | [architect.md](architect.md) | Implementation plan; permitted ADR draft | Assigned plan and ADR draft only |
| Challenger | [challenger.md](challenger.md) | Severity-ranked plan review | Read-only |
| Executor | [executor.md](executor.md) | Implementation and verification report | Approved plan's files and tests |
| Auditor | [auditor.md](auditor.md) | Behavior-preserving cleanup and report | Already green implementation's diff |
| Explorer | [explorer.md](explorer.md) | Findings, file map or requested prompt | Read-only |

`ARCHITECTURE.md` defines binding project constraints. Do not silently bypass a
conflict, modify those rules to accommodate code, or treat a plan as an
architectural amendment. Report the exact conflict for human resolution.
Accepted ADRs apply; report contradictions between authoritative sources.

For UI work, precedence is `ARCHITECTURE.md` →
`docs/ui_design/README_UI.md` → relevant `.txt` references → implementation plan.
Read the applicable semantics and reference states before proposing UI changes.
Never use the deleted legacy `ui_design.md`.

`Description.md` supplies vision. `MEMORY.md`, backlog entries and historical
validation reports provide context and evidence; they do not override current
specifications or establish that today's checkout passes verification.

The human controls every transition in Architect → Challenger → Executor →
Auditor. An agent may recommend a next step and must then stop. Preparing a
prompt does not authorize executing it, creating its output files, dispatching
its target agent, or moving to the next stage. Only the human merges into `main`.

## Task contract

Establish these fields from the request and available state; ask only for
information essential to the requested stage:

- selected role and concrete objective;
- task identifier, milestone and acceptance criteria;
- checkout path, branch and baseline commit; included uncommitted changes;
- exact plan path and approved revision for execution;
- input files/directories and permitted output files;
- excluded work and required verification.

Do not assume plan approval from its existence or from a Challenger verdict.
Record the role, baseline and plan revision in the deliverable. For uncommitted
inputs, record content hashes or an equivalent reviewable snapshot; a commit
hash alone does not identify the working tree.

Resolve ordinary technical choices autonomously within the approved scope.
Prefer the minimum robust implementation for the current milestone. Do not add
speculative abstractions or infrastructure for later milestones. Dependencies
need approved-plan scope and architecture, MSRV, licensing, security and
maintenance compatibility; absence from ARCHITECTURE.md alone is not a ban.

Keep context narrow. Search relevant directories only; do not scan the whole
repository unless explicitly requested. Do not load generated `target/`, binary
assets, fixture bodies or the full Cargo.lock as background context. Inspect a
specific excluded input only when needed for the task. Keep replies concise;
use English for committed documents and the human's language for conversation.

## Parallel Claude and Codex work

Any role may run in parallel, including independent proposals or implementations
of the same task, when explicitly assigned by the human. This does not authorize
skipping a stage or starting another agent automatically.

- Each writing task uses its own branch and checkout/worktree. Never run two
  writers in one checkout: formatting, snapshots, lockfiles and the Git index
  can overlap even when the assigned source files differ.
- Identify a common baseline first. A worktree from HEAD excludes staged,
  unstaged and untracked work. Do not commit, stash, reset or discard another
  task's work to prepare it. If inputs are uncommitted, use an explicitly
  authorized snapshot including them, or wait for a human-provided commit.
  Report missing inputs rather than implementing against stale HEAD.
- Independent plans use distinct paths such as `docs/plans/<task>-claude.md`
  and `<task>-codex.md`. The human selects the plan/revision to approve.
  Alternative implementations remain on separate branches.
- Reviews consume an identified commit/diff or file snapshot. If inputs change
  during review, identify the change and re-review affected inputs before a verdict.
- Keep build outputs local to each checkout; avoid sharing CARGO_TARGET_DIR
  across simultaneous builds. Never format or update snapshots in another task's checkout.
- Auditor takes over the same green implementation branch after its writer
  stops. An independent audit checkout starts at that exact implementation
  commit. The human controls integration; never automatically merge or cherry-pick.

This file-based workflow does not register agents or dispatch messages. Claude
settings are not shared with Codex. Use available read-only execution modes for
Challenger/Explorer. Instructions do not replace tool restrictions and CI.

## Verification and handoff

Only Executor and Auditor execute build/test/application commands. Explorer and
Challenger remain read-only; Architect does not run implementation checks.

Before code changes, record a baseline with snapshot updates disabled. Use:

```sh
cargo check --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
INSTA_UPDATE=no cargo nextest run --workspace --all-features --locked
cargo test --workspace --doc --locked
```

Without nextest, use
`INSTA_UPDATE=no cargo test --workspace --all-features --locked --no-fail-fast`.
Run focused checks during implementation, then required workspace checks.
Verify docs, licensing/advisories, declared MSRV and GPU tests when relevant.
GPU tests need a device and run explicitly with
`cargo test -p newell-render --locked -- --ignored`.

If compilation remains stuck after two repair attempts, stop with the error,
attempted fixes and a precise request for help. Never disable lints, weaken
tests, skip failures or regenerate snapshots merely to obtain green checks.
Existing test/reference changes need an approved, specification-backed reason.
Format only assigned files in a dirty checkout.

Separate baseline failures from new regressions. No new failures does not mean
the suite is green or satisfy Auditor's green-entry gate. Never claim visual or
physical-device acceptance from CPU/GPU tests alone.

Every handoff reports role/provider, objective, baseline, plan revision,
inspected/changed files, actual check results, unresolved failures, pending manual
checks and the recommended next stage. A recommendation never starts it.

## Prompt template for either provider

Replace bracketed fields. Use a fresh conversation per role. For a plain ChatGPT
conversation without repository access, attach the current shared instructions,
architecture and required task inputs; never assume access to a local path.

```text
Act as Newell's [Architect / Challenger / Executor / Auditor / Explorer].
Read [CLAUDE.md for Claude / AGENTS.md for Codex], docs/agents/README.md,
ARCHITECTURE.md and docs/agents/[selected-role].md.

Task: [identifier, milestone and concrete objective]
Checkout and branch: [absolute path and branch]
Baseline: [commit; snapshot/hash of any uncommitted inputs]
Plan: [exact path and revision; explicit human approval if Executor]
Input scope: [exact files/directories]
Allowed output files: [paths, or none for a read-only role]
Out of scope: [exclusions]
Acceptance and verification: [criteria and relevant checks]

Perform only this role. Preserve other work and the Git index.
Return the role's deliverable with evidence and remaining limitations.
Do not start another role or agent. Stop after delivering this stage.
```

Source: [Codex instruction discovery](https://learn.chatgpt.com/docs/agent-configuration/agents-md).
Shared role files are project conventions loaded by the entry points, not
automatic Codex subagent registration or model selection.
