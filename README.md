# Newell

**Newell** is a next-generation parametric CAD (Computer-Aided Design) application. 
Designed for extreme performance and a minimal memory footprint, Newell adopts a "TUI / Hacker" aesthetic while leveraging modern GPU hardware acceleration. The development is driven by a structured AI-agent workflow, under the strict supervision of a human designer.

## Technical Vision & Stack

Newell eliminates the overhead of heavy UI frameworks (like Qt or Electron) to allocate 100% of system resources to geometric computation.

* **Language:** Rust (Memory safety, Zero-cost abstractions, Concurrency).
* **Geometric Kernel:** [BrepKIT](https://github.com/brepkit/brepkit) - A pure-Rust B-Rep engine, strictly typed (no `unsafe`), optimized for AI-generation.
* **Lightweight Philosophy:** Drastically reduced RAM consumption, allowing complex models to run on modest hardware (e.g., Raspberry Pi 4/5).
* **Performance:** Guaranteed 60 FPS minimum for UI and navigation. Geometry recalculations never block the main render thread.
* **License:** **AGPL-3.0** (Open Source).

## Interface & Graphics (wgpu)

The User Interface discards the traditional DOM for a hardware-accelerated **Character Grid** overlaid on a 3D viewport.

* **Renderer:** `wgpu` (Low-level graphics API).
* **Typography:** Dynamic vector text generated from a `.ttf` font (e.g., Fira Code).
* **Iconography:** Tool logos (Extrude, Sketch, etc.) are sampled from a single **RGBA `.png` Texture Atlas**.
* **Shader-Driven UX:** Visual interactions (hover effects, color shifts, centering) are computed entirely on the GPU via Fragment Shaders, costing zero CPU overhead.

## The Agentic Workflow

Newell's codebase is generated, critiqued, and refactored by a team of 4 AI Agents, coordinated via ephemeral sessions to prevent context drift.

1.  **Architect:** Translates human concepts into Rust specifications, defines traits, and writes implementation plans.
2.  **Challenger:** A "one-shot" reviewer that stress-tests the Architect's plan for logical flaws and borrow-checker conflicts.
3.  **Executor:** The blind coder. Follows the plan strictly to implement the feature and make the tests pass.
4.  **Auditor:** The refactorer. Steps in only on green tests to clean up code, enforce Clippy rules, and apply idioms.

*Note: As the human lead, I design the constraints, enforce the architecture, and merge the final Pull Requests.*

##  Testing Architecture

Validation is layered and strictly impenetrable:
1. **Syntax & Memory:** `rustc`, strict `clippy` (no unwraps in production code).
2. **Geometry (B-Rep):** Unit tests on BrepKIT operations (manifoldness, normals, boolean intersections).
3. **UI & State:** Headless state testing for grid hit-testing and MVC controller logic.

##  License
This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0)**. See the `LICENSE` file for details.
