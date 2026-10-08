# ADR 0001: Selection of BrepKIT and Adoption of AGPL-3.0 License

**Date:** 2026-10-08
**Status:** Approved
**Agent Author:** Architect (Claude Opus 5.5)

## 1. Context
Newell requires a 3D geometric B-Rep (Boundary Representation) and solid-modeling kernel to drive its parametric feature tree. To satisfy our strict constraints (low memory footprint, 60 FPS UI stability, AI-agent compatibility, and absolute memory safety without `unsafe` Rust), we must select a kernel. 

We have explicitly rejected heavy C++ wrappers (like OpenCASCADE via FFI/WASM) to avoid abstraction overhead and unpredictable runtime panics that AI agents cannot easily debug. `BrepKIT` is the only pure-Rust kernel that fits our technical requirements. However, as of version 3.0.0, BrepKIT is distributed under a dual-license model: AGPL-3.0 or a Commercial License.

## 2. Options Considered
* **Option A: Use BrepKIT 3.0+ under AGPL-3.0.** 
  * *Pros:* Free access to the latest optimizations, pure-Rust geometry, active updates, strict typings perfect for our AI Executor.
  * *Cons:* AGPL-3.0 is a strong copyleft (viral) license. Newell must be open-sourced entirely under AGPL-3.0 or a compatible license.
* **Option B: Use BrepKIT 3.0+ under a Commercial License.** 
  * *Pros:* Allows Newell to remain closed-source and proprietary.
  * *Cons:* Requires upfront financial investment to Collective Context, LLC, which is premature for the current M0/M1 prototyping phase.
* **Option C: Use Legacy BrepKIT 2.129.x (MIT/Apache-2.0).** 
  * *Pros:* Free and allows proprietary distribution.
  * *Cons:* Deprecated. Deprives the project of critical performance updates, bug fixes, and newer geometry features (like advanced fillets and robust booleans) needed for a modern CAD app.

## 3. Decision
We select **Option A**. Newell will use the latest version of **BrepKIT** and will be licensed under the **AGPL-3.0** license. 

BrepKIT is the single source of truth for B-Rep construction in Newell. No fallback kernel will be introduced. The viral nature of AGPL-3.0 is accepted as the standard operating model for this project's public repository.

## 4. Consequences
* **Positive:** The AI agents (Executor and Challenger) can leverage a heavily typed, zero-`unsafe` Rust API, drastically reducing unpredictable geometric failures. The software remains free to develop.
* **Negative / Risks:** We cannot distribute Newell as a closed-source proprietary application. 
* **Mitigation:** If the business model shifts in the future and proprietary distribution becomes a requirement, a commercial license from BrepKIT's authors must be acquired before any such distribution occurs. For now, the Auditor agent is strictly instructed to append the AGPL-3.0 header to every new `.rs` file to maintain compliance.
