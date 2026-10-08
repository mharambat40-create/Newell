# Newell - Project Memory & Logbook

> **Agent Instruction:** Read this file to understand the current state of the project. Append new lessons learned at the bottom. Keep it concise. Do NOT paste source code here.

## Current Status
* **Active Milestone:** M1 (Skeleton & UI Rendering)
* **Current Task:** Setup the wgpu rendering pipeline and load the `FiraCode.ttf` font.

## Pitfalls & Lessons Learned (Do not repeat these mistakes)
* **wgpu Cell Ratio:** The base font ratio is 3:5. Do not attempt to render square logos into 2x2 cells. Always allocate 5x3 cells for major tool icons.
* **Texture Atlas:** The logo atlas is a single RGBA `.png` with 8px inward padding. Do NOT add external padding, as it breaks the Power of Two (POT) rule.
* **Geometric Kernel:** BrepKIT is our ONLY geometric kernel. Do not attempt to import or suggest OpenCASCADE or Truck crates.

## Completed Milestones
* **M0:** Architecture defined, AGPL-3.0 license adopted, BrepKIT dependency validated.
