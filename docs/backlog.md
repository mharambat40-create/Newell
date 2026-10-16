# Newell - Project Backlog

## Milestone 1: Skeleton
- [x] M1.1: Initialize Rust workspace, `.claudesignore`, and CI/CD pipeline.
- [x] M1.2: Setup testing architecture workspace-wide (`proptest`, `insta`, `thiserror`) and standard `tests/` directories.
- [x] M1.3: Setup `winit` window and basic `wgpu` surface in `crates/newell-app` and `crates/newell-render` (window with empty grid at 60 FPS).

## Milestone 2: ASCII UI Framework & Application Shell

- [x] M2.1: Define the core `Cell`, style and `widget_id` representation and
  implement the flat 1D UI cell buffer in `newell-ui`.
- [x] M2.2: Implement the character-grid layout engine, including rows,
  columns, rectangular regions, clipping and box-drawing borders.
- [x] M2.3: Load `assets/FiraCode-Regular.ttf` and implement glyph
  rasterization and the glyph texture atlas in `newell-render`.
- [x] M2.4: Implement the `newell-render` ASCII pipeline so that the complete
  UI cell buffer is uploaded and rendered in a single GPU draw call.
- [x] M2.5: Implement O(1) pixel-to-cell hit-testing and sub-cell border
  routing for shared ASCII separators.
- [x] M2.6: Implement the core grid-native interaction state:
  focus, hover, pressed, selected, disabled, expanded and checked.
  The historical shader hover was removed in M2.16; M2.25 uses only
  composition-time Unicode border substitutions.
- [x] M2.7: Implement reusable ASCII widget primitives:
  labels, separators, buttons, checkboxes, segmented selectors,
  cyclic left/right selectors, numeric/text fields and tree rows.
- [x] M2.8: Implement overlay primitives:
  popup menus, modal panels, scrolling selection lists and
  OK/Cancel confirmation areas.
- [x] M2.9: Implement menu and keyboard interaction:
  application menus, contextual command menus, keyboard mnemonics,
  focus traversal and command activation.
- [x] M2.10: Implement the dynamic Action Panel framework.
  The panel must support conditional sections and progressive disclosure
  driven entirely by UI command state.
- [x] M2.11: Implement the reference Extrude Action Panel as a UI-only mock:
  Solid/Surface, New/Add/Remove, Selection, Blind/Up-to/Through-all,
  Depth, Invert, Starting Offset, Symmetric, Second End Position,
  Merge options and OK/Cancel.
  No geometric operation is performed.
- [x] M2.12: Implement the Formula editor and Selection panel as UI-only
  overlays. Formula evaluation and real model selection remain out of scope.
- [x] M2.13: Compose the complete reference Newell application shell:
  application menu bar, contextual command bar, Features tree,
  Elements tree, 3D viewport area, static NavCube placeholder,
  Action Panel and status bar.
- [x] M2.14: Implement responsive character-grid layout and user-resizable
  panel boundaries with whole-cell snapping.
- [x] M2.15: Add deterministic UI-state tests and reference cell-buffer
  snapshots for the application shell, menus and Action Panel variants.
- [x] M2.16: typography (4:9 cells, k = 5), monochrome rendering and hover removal.
- [ ] M2.17: frameless native window and 82 × 30 minimum grid.
- [ ] M2.18: Application MenuBar, UI Scale (60/80/100/120%) and Light/Dark theme controls.
- [x] M2.19: CommandBar and centered menus.
- [x] M2.20: TreeView layout, playhead, separators and Rename.
- [x] M2.21: exact Action Box grammar and Extrude.
- [x] M2.22: Revolve, Loft, Sweep and Boolean Action Boxes.
- [x] M2.23: numeric fields and FormulaEditor.
- [ ] M2.24: live FPS, progress bar and final UI fidelity.
  Blocked: FPS and progress are fixed reference values; the interactive
  progress animation and event-driven scheduling are not implemented.
- [ ] M2.25 — ASCII Hover Highlight
  Hover implementation and sampled visual checks pass; final acceptance retains
  the M2.24 scheduling prerequisite required by the plan.

### Validation checkpoint — 2026-10-16

All 665 standard tests and 8 GPU tests pass. These passing tests do not prove
that every M2 requirement is implemented. Visual checks of the current ASCII
application confirmed monochrome typography, both themes, parameter/header
hover, scale preference cycling, maximise/restore and fullscreen round-trip
on Full HD. M2.16 and M2.19–M2.23 are checked above.

M2.17–M2.18 still require confirmed cross-monitor and native edge-resize
checks. M2.24 is incomplete: FPS is fixed at 98, progress at 65%, and the
60 FPS bootstrap pacer remains active. M2.25's sampled hover visuals pass,
but the plan's M2.24 scheduling prerequisite remains open. See
[the automated report](validation/M1-M2-2026-10-16.md) and
[the visual report](validation/M2-visual-2026-10-16.md).
M3 and later milestones are not implemented and remain unchecked.

## Milestone 3: Viewport & Kernel Boundary
- [ ] M3.1: Setup `newell-kernel` crate with BrepKIT dependency and define internal wrapping structures (e.g., NewellSolid) without leaking BrepKIT types.
- [ ] M3.2: 3D camera setup and display of a BrepKIT-generated solid through `newell-kernel`.
- [ ] M3.3: Map BrepKIT topology to the `wgpu` mesh buffers and implement 3D picking.

## Milestone 4: CAD Command & Parametric Interaction
- [ ] M4.1: Define the generic command-state/controller interface used by
  contextual Action Panels without coupling `newell-ui` to BrepKIT.
- [ ] M4.2: Bind the Features tree to the real parametric feature model and
  the Elements tree to resulting geometric entities.
- [ ] M4.3: Implement shared selection state between the viewport, trees and
  Selection panel.
- [ ] M4.4: Implement parameter values with engineering units and bind numeric
  Action Panel fields to command parameters.
- [ ] M4.5: Implement the expression/formula data model and connect the
  Formula editor to parameter values.
- [ ] M4.6: Implement the real Extrude command using the existing Action Panel:
  Solid/Surface and New/Add/Remove modes.
- [ ] M4.7: Implement Extrude termination modes:
  Blind, Up to and Through all.
- [ ] M4.8: Implement Extrude modifiers:
  Invert, Starting Offset, Symmetric and Second End Position.
- [ ] M4.9: Implement Add/Remove target handling:
  Merge with all and explicit Merge scope.
- [ ] M4.10: Implement command preview, OK/Cancel semantics and rollback of
  uncommitted command state.
