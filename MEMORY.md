# Newell - Project Memory & Logbook

> **Agent Instruction:** Read this file to understand the current state of the project. Append new lessons learned at the bottom. Keep it concise. Do NOT paste source code here.

## Current Status
* **Active Milestone:** M2 (ASCII UI Framework). M2.1–M2.15 are complete; M2.16 (typography, monochrome, hover removal) is in progress.
* **Plan:** `docs/plans/M2-ui-framework.md` §19 (M2.16–M2.24). Font: `assets/FiraCode-Regular.ttf`.

## Pitfalls & Lessons Learned (Do not repeat these mistakes)
* **wgpu Cell Ratio:** The cell ratio is 4:9 (W = 4k, H = 9k; reference 20×45 px), not 3:5 (superseded in M2.16). Glyphs use one uniform scale, `advance = W`, and are never stretched. Do not render square logos into 2x2 cells. Allocate 7x3 cells for major tool icons and 4x2 for minor ones (`ARCHITECTURE.md` §3.1, §3.3).
* **UI Scale:** Exactly 60/80/100/120/140% (preference), default 100%. `k` range is 2..7 (cells 8×18 … 28×63 px); on the reference display (reference 5) the five values are k 3..7; the compact class (`k_fit(windowed) ≤ 3`, e.g. Full HD, detected by pixels only, never by name) has reference 2, so 100% = k 2 (8×18, min window 656×540); MacBook Retina 100% stays k 5 (ADR 0003 second amendment, 2026-10-16, approved by the human). M2.17 chooses `k` once at start-up (before window creation, `PhysicalSize`, estimate `usable_height = h − h/16`, `k_max = min(⌊w/328⌋, ⌊usable/270⌋)`); M2.18 adds the interactive selector and run-time changes. Preference and effective scale are separate. The window never goes below 82×30 cells (the 82×28 `Interface_Exemple.txt` is unchanged; extra rows go to the central body); no fractional cells, no glyph resampling except logos (`ARCHITECTURE.md` §3.1.1, §3.3, ADR 0003).
* **No hover, no inversion:** No hover state or visual. Widget states are drawn only with documented ASCII structures: no `>`/`<` markers, no fg/bg inversion. Strictly `#000000`/`#FFFFFF` (`ARCHITECTURE.md` §3.5, §3.6). Temporary accepted deviation until M2.21: the M2.7 `║` active-segment separators. The only approved edit indicator is the `▏` text caret (M2.23).
* **Frameless resize:** The macOS frameless window must stay manually resizable. `drag_resize_window` is `NotSupported` (winit 0.30.13), so M2.17 uses the application-managed resize (`ARCHITECTURE.md` §3.1.1, `README_UI.md` §25.1): safe winit API only, size requested before position, whole cells, min 82×30, cancel on release/focus loss, controls win. West/north zones are a reported limitation (manual check).
* **Glyph atlas:** Deterministic: required set per `k`, never modified, everything else is `?`. Filtered sampling for logos only; glyphs stay nearest-neighbour.
* **StatusBar FPS:** Fixed 7-cell segment (3-digit right-aligned field + `fps` + blank), saturates at 999, equals the reference at 98 (M2.24). Caret `▏`: no blink, no stored text change, no extra draw call (M2.23). Rename opens on double-click (M2.20).
* **Extrude variants:** Complete base boxes outrank partial snippets; a variant never deletes sibling controls (`README_UI.md` §2.6). Never edit `Action_Box_Extrude.txt`.
* **UI doc path:** The UI specification is `docs/ui_design/README_UI.md` (not `README.md`).
* **Texture Atlas:** The logo atlas is a single RGBA `.png` with 8px inward padding. Do NOT add external padding, as it breaks the Power of Two (POT) rule.
* **Geometric Kernel:** BrepKIT is our ONLY geometric kernel. Do not attempt to import or suggest OpenCASCADE or Truck crates.

## Completed Milestones
* **M0:** Architecture defined, AGPL-3.0 license adopted, BrepKIT dependency validated.
