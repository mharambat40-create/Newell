# ADR 0002: Workspace Layout, Agent Roles, and UI Rendering Model

**Date:** 2026-10-12
**Status:** Approved
**Agent Author:** Architect (Claude Opus 5.5)

## 1. Context
Three project documents disagreed on three points:
* **Crate count:** `Description.md` §3.1 listed seven crates, including `newell-sketch` and `newell-io`. `ARCHITECTURE.md` §1 lists five.
* **Agent count:** `Description.md` §6 described five agents, including Explorer as a pipeline stage. `ARCHITECTURE.md` §2 defines four primary agents and an optional, read-only Explorer.
* **Render model:** `docs/ui-design.md` described four ordered render passes for the ASCII overlay. `ARCHITECTURE.md` §3.4 requires a single GPU draw call.

The disagreements must be resolved before M1.1 (workspace and CI) so that the code and the specifications are built against one model.

## 2. Options Considered
* **Option A:** Keep the documents as they are and resolve conflicts case by case. (Pros: no change. Cons: agents and humans keep working from contradictory specifications.)
* **Option B:** Align every specification with `ARCHITECTURE.md` and record the decisions here. (Pros: one source of truth, auditable decisions. Cons: several documents change in one PR.)
* **Option C:** Amend `ARCHITECTURE.md` to match the older documents. (Pros: fewer edits to the other documents. Cons: weakens the binding rules that govern all agents.)

## 3. Decision
We select **Option B**. The following decisions apply:
* **D1 (five-crate workspace):** `newell-kernel`, `newell-model`, `newell-ui`, `newell-render`, `newell-app`. `newell-sketch` and `newell-io` are not part of the workspace. They can be added later only through an amendment to `ARCHITECTURE.md` §1.
* **D2 (agents):** Four primary agents (Architect, Challenger, Executor, Auditor). Explorer is an optional, read-only helper and is not a pipeline stage.
* **D6 (render model):** The ASCII overlay is drawn in a single batched `wgpu` draw call, as required by `ARCHITECTURE.md` §3.4. No multi-pass design is used for the overlay.
* **D10 (error handling dependency):** `thiserror = "2"` is declared in `[workspace.dependencies]`. Each crate opts in only when it uses it.
* **D8 (lockfile):** `Cargo.lock` is committed so that builds are reproducible.

## 4. Consequences
* **Positive:** One set of rules applies across the specifications. The agent pipeline and the render model are unambiguous before any geometry or UI code is written.
* **Positive:** Adding a crate or a render pass now requires a new ADR, which keeps the architecture deliberate.
* **Negative / Risks:** `Description.md` §3.1, §6 and `docs/ui-design.md` §3–§4 had to be rewritten, which is a larger documentation change. The glyph atlas and logo atlas must share one draw call. The binding mechanism (single bind group, texture array, or combined atlas) is not chosen here and will be decided in an ADR during M2.4.
* **Mitigation:** The M2.4 ADR must confirm that the chosen binding mechanism keeps the single draw call required by `ARCHITECTURE.md` §3.4.
* **Note:** `docs/adr/0001-*.md` is not modified by this decision.
