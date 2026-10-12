# Newell - Project Backlog

## Milestone 1: Skeleton (Active)
- [ ] M1.1: Initialize Rust workspace, `.claudesignore`, and CI/CD pipeline.
- [ ] M1.2: Setup testing architecture workspace-wide (`proptest`, `insta`, `thiserror`) and standard `tests/` directories.
- [ ] M1.3: Setup `winit` window and basic `wgpu` surface in `crates/newell-app` and `crates/newell-render` (window with empty grid at 60 FPS).

## Milestone 2: Grid UI
- [ ] M2.1: Define the `Cell` data structure (char, style, `widget_id`) and implement the flat 1D array buffer in `newell-ui`.
- [ ] M2.2: Implement the core 2D ASCII Grid data structure and the text rendering pipeline logic.
- [ ] M2.3: Load `assets/FiraCode-Regular.ttf` and implement glyph rasterization (Texture Atlas) in `newell-render`.
- [ ] M2.4: Configure the `newell-render` pipeline to upload the flat buffer and draw the entire grid in a single `wgpu` draw call.
- [ ] M2.5: Implement O(1) hit-testing math (division of mouse pixel coordinates by Fira Code cell dimensions).
- [ ] M2.6: Implement sub-cellular hit-testing (modulo logic) on ASCII separator characters to expand button hitboxes to their visual borders.
- [ ] M2.7: Implement hover in shader, map hit-testing events, and implement keyboard navigation.
- [ ] M2.8: Implement the 6-pane responsive UI layout (Menu, Toolbar, Tree, Task, State, Viewport).
- [ ] M2.9: Implement user-draggable panel resizing (character-by-character grid snapping).

## Milestone 3: Viewport & Kernel Boundary
- [ ] M3.1: Setup `newell-kernel` crate with BrepKIT dependency and define internal wrapping structures (e.g., NewellSolid) without leaking BrepKIT types.
- [ ] M3.2: 3D camera setup and display of a BrepKIT-generated solid through `newell-kernel`.
- [ ] M3.3: Map BrepKIT topology to the `wgpu` mesh buffers and implement 3D picking.

## Milestone 4: Multi-Window Engine Implementation
- [ ] M4.1: Refactor the state structure to enforce strict MVC architecture, decoupling global geometric data (`newell-model`) from UI-specific data (`newell-ui`).
- [ ] M4.2: Update `crates/newell-app` with a root manager capable of spawning multiple OS-level windows via `winit`, each initializing an isolated `newell-ui` and `newell-render` context.
- [ ] M4.3: Implement an internal Event Bus (Pub/Sub) messaging system between the geometry workers and the main thread.
- [ ] M4.4: Bind the window rendering loops to the Event Bus to synchronize state (e.g., Window B automatically redraws when Window A modifies a shared entity).
- [ ] M4.5: Write state tests (without GPU involvement) to verify UI isolation, confirming that actions in one window have zero impact on another.
- [ ] M4.6: Implement graceful resource cleanup to ensure local textures and buffers are dropped when a window closes, without affecting the Global Document State.
