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

The normal implementation path is Architect → optional Challenger → one
targeted Architect correction if needed → human approval and implementation
request → Executor. Challenger review happens only when the human requests it.
The first review is complete and scoped; a further Challenger pass requires a
separate explicit human request. Challenger's verdict and findings are advisory,
not approval gates. No review or correction is automatic beyond that bounded
path.

The human may approve the identified plan revision and request implementation
in the same instruction. That explicit approval plus implementation request
ends the review stage and authorizes Executor, whether or not Challenger
reviewed the plan. Do not ask for approval again after it has been given. A
plan's existence alone is not approval. A remaining “proposed” label or
advisory Challenger verdict does not add another gate. Auditor may follow a
green implementation only on a separate human request. The human controls every
role transition. An agent may recommend a next step and must then stop.
Preparing a prompt does not authorize executing it, creating its output files,
dispatching its target agent, or moving to the next stage. Only the human
authorizes integration or external publication.

## Bounded review and blocking criteria

The first Challenger review examines the affected behavior end to end and
consolidates actionable findings. Each finding names the violated requirement,
a concrete failure path, the smallest correction, and the relevant acceptance
test. Do not expand the milestone or make optional improvements prerequisites.
A correction pass addresses the findings without redesigning unrelated parts of
the plan.

A finding is **blocking** only when a specific failure path demonstrates that
the proposed implementation would:

- fail to compile or violate a required interface;
- fail a required behavior or acceptance criterion;
- create invalid persistent state, lose a committed change, or leave required
  synchronization permanently incomplete; or
- contradict a binding specification or architectural rule.

A technical preference, optional optimization, speculative future risk, or
request for additional abstraction is a recommendation unless it demonstrates
one of those concrete current failures. On a subsequent review explicitly
requested by the human, focus on the corrections and affected dependencies.
Still report any newly discovered serious correctness problem. That finding
does not trigger another role or review automatically.

## Task contract

Establish these fields from the request and available state; ask only for
information essential to the requested stage:

- selected role and concrete objective;
- task identifier, milestone and acceptance criteria;
- existing Newell checkout on `main`; relevant staged, unstaged and untracked
  changes;
- exact plan path and revision; for Executor, the human's explicit approval of
  that revision and request to implement (which may be given together);
- input files/directories and permitted output files;
- excluded work and required verification.

Do not assume plan approval from its existence or from a Challenger verdict.
Once the human explicitly approves the identified revision and requests
implementation, do not request approval again. Record the role, relevant
starting state and plan revision in the deliverable.
Identify uncommitted inputs by their paths and relevant changes. They do not
by themselves block review or execution; report a concrete conflict if one
prevents safe progress. No source snapshot, content hash, snapshot archive or
snapshot commit is a prerequisite for either stage.

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

## Sequential work in the main checkout

All roles use the existing Newell directory on `main`, one at a time. Do not
create task branches, worktrees, separate execution checkouts or parallel
implementations. Keep the workflow linear for this prototype.

- Before writing, inspect relevant staged, unstaged and untracked changes.
  Preserve existing work and the Git index; do not commit, stash, reset or
  discard it to prepare a task. Report an actual file or scope conflict if it
  prevents safe progress.
- Review the identified plan revision and relevant inputs as they exist in this
  checkout. If an input changes materially during review, identify the change
  and reconsider affected findings. A commit or snapshot is not required.
- Executor implements the explicitly approved plan in this checkout, with or
  without Challenger review. If explicitly requested, a subsequent Challenger
  review focuses on the correction and affected dependencies; serious new
  correctness findings are still reported without automatically restarting the
  workflow. Auditor enters only after a green implementation and a separate
  human request.
- Never automatically commit, push, merge or publish. The human authorizes
  integration and external publication.

This file-based workflow does not register agents or dispatch messages. Claude
settings are not shared with Codex. Use available read-only execution modes for
Challenger/Explorer. Instructions do not replace tool restrictions and CI.

## Verification and handoff

Only Executor and Auditor execute build/test/application commands. Explorer and
Challenger remain read-only; Architect does not run implementation checks.

Before code changes, record a verification baseline with automatic test
snapshot updates disabled. Use:

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

Every handoff reports role/provider, objective, relevant starting state, plan revision,
inspected/changed files, actual check results, unresolved failures, pending manual
checks and the recommended next stage. A recommendation never starts it.

## Prompt template for either provider

When drafting or revising a prompt for any role, make it deeper without making
the task bigger. Before writing the final prompt, answer these four questions
from the available task context. This method is adapted from
[Felix Rieseberg's original article](https://felixrieseberg.com/how-to-be-more-ambitious-a-tutorial-for-humans-with-models/):

1. What would a top professional do to achieve this objective that the request
   does not yet specify?
2. What is the rawest available material (for example, source code,
   original specifications, raw measurements or logs), instead of a summary?
3. Which specific methods and permitted tools would that professional use?
4. What evidence independent of the agent's own judgment would show that the
   result is right (for example, specification checks, tests, measured results
   or human/physical-device acceptance where required)?

Then rewrite the prompt to request the useful findings and methods within the
human-assigned objective, role, file scope and verification authority. Name
missing primary material or unavailable validation instead of inventing it.
This is a prompt-writing method, not a new workflow stage, permission to widen
scope, or a requirement for Challenger review. Preparing a prompt still
authorizes returning that prompt only, not performing the target task.

Replace bracketed fields. Use a fresh conversation per role. For a plain ChatGPT
conversation without repository access, attach the current shared instructions,
architecture and required task inputs; never assume access to a local path.

```text
Act as Newell's [Architect / Challenger / Executor / Auditor / Explorer].
Read [CLAUDE.md for Claude / AGENTS.md for Codex], docs/agents/README.md,
ARCHITECTURE.md and docs/agents/[selected-role].md.

Task: [identifier, milestone and concrete objective]
Checkout: [existing Newell directory on main]
Starting state: [relevant staged, unstaged and untracked changes]
Plan: [exact path and revision; explicit human approval and implementation request if Executor]
Input scope: [exact files/directories]
Allowed output files: [paths, or none for a read-only role]
Out of scope: [exclusions]
Acceptance and verification: [criteria and relevant checks]
Expert depth: [additional professional practice within scope; primary material;
specific methods and permitted tools; independent evidence]

Perform only this role. Preserve other work and the Git index.
Return the role's deliverable with evidence and remaining limitations.
Do not start another role or agent. Stop after delivering this stage.
```

Source: [Codex instruction discovery](https://learn.chatgpt.com/docs/agent-configuration/agents-md).
Shared role files are project conventions loaded by the entry points, not
automatic Codex subagent registration or model selection.
