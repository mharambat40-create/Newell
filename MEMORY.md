# Newell - Project Memory & Logbook

> **Agent Instruction:** Read this file to understand the current state of the project. Append new lessons learned at the bottom. Keep it concise. Do NOT paste source code here.

## Current Status
* **Current work:** M3 camera mechanics (Controller-owned zero-roll camera, dynamic pivot, pan/zoom), with M2.17–M2.18 native resize/cross-monitor acceptance and M3.10 physical-device/visual acceptance still open. Use `docs/plans/M3-camera-mechanics.md`; the earlier navigation plan is superseded.
* **Verification (2026-10-20):** All 38 inherited failures are corrected: **737 tests pass, 0 fail**, plus **9 GPU tests pass** explicitly. Strict Clippy, formatting, docs and Rust 1.88 compilation pass. Physical-device/visual acceptance remains separate. The 2026-10-19 report describes the earlier red baseline; use `docs/validation/dual-provider-workflow-2026-10-20.md` for current evidence.
* **Working tree:** M3 and partial M4 changes are already present, including staged, unstaged and untracked files. M4 backlog boxes are not acceptance evidence. Preserve these inputs; HEAD alone omits them.
* **Agents:** Claude and Codex can each perform all four primary roles and optional Explorer. Shared instructions: `docs/agents/README.md` and the selected role document. The human controls transitions; roles work sequentially in the existing `main` checkout. Challenger review is optional and advisory; Executor requires explicit plan approval and an implementation request.

## Pitfalls & Lessons Learned (Do not repeat these mistakes)
* **wgpu Cell Ratio:** The cell ratio is 4:9 (W = 4k, H = 9k; reference 20×45 px), not 3:5 (superseded in M2.16). Glyphs use one uniform scale, `advance = W`, and are never stretched. Do not render square logos into 2x2 cells. Allocate 7x3 cells for major tool icons and 4x2 for minor ones (`ARCHITECTURE.md` §3.1, §3.3).
* **UI Scale:** Exactly 60/80/100/120/140%, default 100%; `effective_k` is 2..7, derived from monitor physical pixels, window mode and preference. Keep preference/reference/effective values distinct. M2.17 owns automatic per-monitor adaptation, atomic transitions, bounded fit/resize and trigger handling; M2.18 adds the selector. Full HD reference is 2, Retina reference can be 5. Grid minimum is 82×30, cells stay 4:9 with no fractional sizes or glyph resampling. Read current `ARCHITECTURE.md` §3.1.1 instead of applying the superseded startup-only rule.
* **ASCII hover only:** M2.25 permits Controller-owned `hovered: Option<WidgetId>` and instantaneous documented Unicode border substitutions. No color changes, inversion, animations, shader effects or invented focus markers. Follow current `ARCHITECTURE.md` §3.5 and `README_UI.md` §5.12. The old blanket no-hover rule and temporary M2.7 `║` deviation are superseded; the non-blinking edit caret remains `▏`.
* **Frameless resize:** The macOS frameless window must stay manually resizable. `drag_resize_window` is `NotSupported` (winit 0.30.13), so M2.17 uses the application-managed resize (`ARCHITECTURE.md` §3.1.1, `README_UI.md` §25.1): safe winit API only, size requested before position, whole cells, min 82×30, cancel on release/focus loss, controls win. West/north zones are a reported limitation (manual check).
* **Glyph atlas:** Deterministic: required set per `k`, never modified, everything else is `?`. Filtered sampling for logos only; glyphs stay nearest-neighbour.
* **StatusBar FPS:** Fixed 7-cell segment (3-digit right-aligned field + `fps` + blank), saturates at 999, equals the reference at 98 (M2.24). Caret `▏`: no blink, no stored text change, no extra draw call (M2.23). Rename opens on double-click (M2.20).
* **Extrude variants:** Complete base boxes outrank partial snippets; a variant never deletes sibling controls (`README_UI.md` §2.6). Never edit `Action_Box_Extrude.txt`.
* **UI doc path:** The UI specification is `docs/ui_design/README_UI.md` (not `README.md`).
* **Texture Atlas:** The logo atlas is a single RGBA `.png` with 8px inward padding. Do NOT add external padding, as it breaks the Power of Two (POT) rule.
* **Geometric Kernel:** BrepKIT is our ONLY geometric kernel. Do not attempt to import or suggest OpenCASCADE or Truck crates.

## Completed Milestones
* **M0:** Architecture defined, AGPL-3.0 license adopted, BrepKIT dependency validated.
