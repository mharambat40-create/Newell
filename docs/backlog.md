# Newell - Project Backlog

## Milestone 1: Skeleton (Active)
- [x] M1.1: Initialize Rust workspace, `.claudesignore`, and CI/CD pipeline.
- [ ] M1.2: Setup testing architecture workspace-wide (`proptest`, `insta`, `thiserror`) and standard `tests/` directories.
- [ ] M1.3: Setup `winit` window and basic `wgpu` surface in `crates/newell-app` and `crates/newell-render` (window with empty grid at 60 FPS).

## Milestone 2: Grid UI
- [ ] M2.1: Load `assets/FiraCode-Regular.ttf` and implement glyph rasterization (Texture Atlas) in `newell-render`.
- [ ] M2.2: Implement the core 2D ASCII Grid data structure and the text rendering pipeline.
- [ ] M2.3: Implement the 6-pane responsive UI layout (Menu, Toolbar, Tree, Task, State, Viewport).
- [ ] M2.4: Implement user-draggable panel resizing (character-by-character grid snapping).
- [ ] M2.5: Implement hover in shader, hit testing, and keyboard navigation.

## Milestone 3: Viewport & Kernel Boundary
- [ ] M3.1: Setup `newell-kernel` crate with BrepKIT dependency and define internal wrapping structures (e.g., NewellSolid) without leaking BrepKIT types.
- [ ] M3.2: 3D camera setup and display of a BrepKIT-generated solid through `newell-kernel`.
- [ ] M3.3: Map BrepKIT topology to the `wgpu` mesh buffers and implement 3D picking.
