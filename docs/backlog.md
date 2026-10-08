# Newell - Project Backlog

## Milestone 1: Core UI & Window (Active)
- [x] M1.1: Initialize Rust workspace, `.claudesignore`, and CI/CD pipeline.
- [ ] M1.2: Setup testing architecture workspace-wide (`proptest`, `insta`, `thiserror`) and standard `tests/` directories.
- [ ] M1.3: Setup `winit` window and basic `wgpu` surface in `crates/newell-app` and `crates/newell-render`.
- [ ] M1.4: Load `assets/FiraCode-Regular.ttf` and implement glyph rasterization (Texture Atlas) in `newell-render`.
- [ ] M1.5: Implement the core 2D ASCII Grid data structure and the text rendering pipeline.
- [ ] M1.6: Implement the 6-pane responsive UI layout (Menu, Toolbar, Tree, Task, State, Viewport).
- [ ] M1.7: Implement user-draggable panel resizing (character-by-character grid snapping).

## Milestone 2: Geometry & Kernel
- [ ] M2.1: Setup newell-kernel crate with BrepKIT dependency and define internal wrapping structures (e.g., NewellSolid) without leaking BrepKIT types.
- [ ] M2.2: Map BrepKIT topology to the `wgpu` mesh buffers.
