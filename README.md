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
Newell's codebase is generated and refactored by a team of 5 AI Agents working in ephemeral sessions to prevent context drift:

1. **Explorer (Haiku):** Read-only conversational partner for project navigation.
2. **Architect (Opus/Sonnet):** Translates concepts into Rust implementation plans.
3. **Challenger (Sonnet):** Reviews plans for logical flaws and borrow-checker risks.
4. **Executor (Sonnet):** The blind coder; strictly follows the plan and makes tests pass.
5. **Auditor (Haiku/Sonnet):** Refactors green PRs (Clippy, docs, AGPL headers).

## Documentation & AI Entry Points
* **Project Vision & Roadmap:** See `Description.md` for the macro-level design and goals.
* **Binding Rules:** See `ARCHITECTURE.md` for the strict, machine-checkable constraints.
* **AI Instructions:** See `CLAUDE.md` for CLI context management and commands.
