# Newell

**Newell** is a next-generation parametric CAD (Computer-Aided Design) application. 
Designed for extreme performance and a minimal memory footprint, Newell adopts a "TUI / Hacker" aesthetic while leveraging modern GPU hardware acceleration. The development is driven by a structured AI-agent workflow, under the strict supervision of a human designer.

## Technical Vision & Stack
* **Language:** Rust (Memory safety, Zero-cost abstractions).
* **Geometric Kernel:** [BrepKIT](https://github.com/brepkit/brepkit) (Pure-Rust B-Rep engine).
* **Graphics:** `wgpu` rendering a hardware-accelerated Character Grid over a 3D viewport.
* **Performance:** 60 FPS UI guarantee; geometry runs on background threads.
* **License:** **AGPL-3.0** (Open Source).

## The Agentic Workflow
Newell's codebase is generated and refactored by four pipeline AI agents (Architect, Challenger, Executor, Auditor) and an optional read-only Explorer helper, working in ephemeral sessions to prevent context drift:

1. **Architect:** Translates concepts into Rust implementation plans.
2. **Challenger:** Reviews plans for logical flaws and borrow-checker risks.
3. **Executor:** Implements the approved plan and makes its tests pass.
4. **Auditor:** Refactors green implementations (Clippy, docs, AGPL headers).

**Explorer (optional helper):** Read-only conversational partner for project navigation. It is not a pipeline stage.

**Every role is available to both Claude and ChatGPT through Codex.** The human
assigns each role and controls every transition. Parallel writing tasks use
separate branches/worktrees and an identified common baseline. Start with the
[shared workflow and prompt template](docs/agents/README.md).

## Documentation & AI Entry Points
* **Project Vision & Roadmap:** See `Description.md` for the macro-level design and goals.
* **Binding Rules:** See `ARCHITECTURE.md` for the strict, machine-checkable constraints.
* **AI Instructions:** `CLAUDE.md` for Claude; `AGENTS.md` for Codex. Both load the same instructions under `docs/agents/`.
