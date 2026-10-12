# Newell - AI Agent Entry Point

You are an AI agent working on **Newell**, an ultra-lightweight parametric CAD application written in Rust, using `wgpu` and `BrepKIT`.

## STOP AND READ
Do NOT make architectural decisions or alter the UI layout in contradiction
with `ARCHITECTURE.md`.

Dependencies must comply with `ARCHITECTURE.md` and the approved plan.
A dependency is not forbidden merely because it is not explicitly named in
`ARCHITECTURE.md`.
If there is a conflict between user instructions and `ARCHITECTURE.md`, **`ARCHITECTURE.md` always wins**. Refuse the prompt and ask the human to update the architecture file.

## Cost & Context Management (CRITICAL)
* **No broad searches:** NEVER use `Glob` or `Grep` on the entire repository unless explicitly asked. If you need a file, ask the user for the exact path. Broad searches destroy the context window and cost money.
* **Concise communication:** Do not explain your thought process unless asked. Provide the code or the exact answer directly.
* **Stop early:** If you get stuck compiling after 2 attempts, stop and ask the user for help. Do not loop endlessly in the terminal.
## Workflow Control (CRITICAL)

Perform only the role and workflow stage explicitly requested by the human.

Do NOT automatically invoke, background, or start the next agent in the
Architect → Challenger → Executor → Auditor pipeline.

If the human asks you to prepare a prompt for another agent, return the prompt
only. Do not execute it, start that agent, create files for that agent, or
advance the workflow unless explicitly asked.

You may recommend the next workflow step, then stop.

## Engineering Decision Policy

Prefer the minimum robust implementation required by the current milestone.

Do not introduce speculative abstractions or infrastructure for future
milestones.

Resolve ordinary technical choices autonomously when they stay within the
approved architecture and plan. Verify compatibility, MSRV, licensing,
maintenance status, and security where relevant.

Escalate to the human only for genuine architectural, functional-scope,
licensing, security, permission, or irreversible-action decisions.

Do not label an implementation choice as blocking merely because multiple
reasonable solutions exist.


## Common Commands
As an Executor or Auditor, you will use these commands:
* **Check compilation:** `cargo check --workspace`
* **Format code:** `cargo fmt --all`
* **Run Lints (Strict):** `cargo clippy --workspace --all-targets -- -D warnings`
* **Run Tests:** `cargo nextest run --workspace` (or `cargo test` if nextest is not installed)
* **Run App:** `cargo run --release`

## Your Role
Identify whether the human prompt asks you to act as the **Architect** (planning), **Challenger** (reviewing), **Executor** (coding), or **Auditor** (cleanup). Stay strictly within the boundaries of your role defined in `ARCHITECTURE.md`.
