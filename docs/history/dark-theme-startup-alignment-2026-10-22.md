> Historical document recovered during Desktop cleanup on 2026-10-26.
> This proposal is superseded by accepted ADR 0005, later palette/scale decisions,
> and the current architecture. It is not an active execution plan.
> The original text below is retained for context; its checkout and approval
> instructions describe the old workflow and are not current requirements.

# Dark Theme on Startup — Implementation Plan

**Role:** Architect  
**Plan revision:** v1 (draft; not approved for execution)  
**Date:** 2026-10-22  
**Task:** Align the startup theme so every new window starts Dark.  
**Checkout:** `/Users/martin/Desktop/Newell-dark-theme-startup-alignment`  
**Branch / baseline:** `codex/dark-theme-startup-alignment` at `1eb4a9a4b9fb45a4bbfaa223e348c0e5e9a69f1b` (`main` at task start).  
**Allowed deliverables:** this plan and `docs/adr/0005-dark-theme-startup.md` only.

## Architectural conflict and resolution proposal

The human has requested Dark as the startup theme. The current binding architecture still requires Light in two places, and accepted ADR 0003 repeats that decision. The dirty canonical UI specification has been edited to require Dark, but it has lower precedence than `ARCHITECTURE.md`; it cannot amend the architecture by itself. The current `main` version of that UI file still says Light in the corresponding startup sentence. This plan treats the human's request as a proposed architectural change, not as an already-approved override. The formal decision is captured in the accompanying **draft ADR 0005**; no accepted rule is amended by this deliverable.

The following exact replacements are proposed for human approval:

1. **`ARCHITECTURE.md` §3.1.1.** Replace the final startup sentence:

   > A new window starts with preference 100% and the Light theme (§3.6), regardless of other windows and of any OS appearance setting.

   with:

   > A new window starts with preference 100% and the Dark theme (§3.6), regardless of other windows and of any OS appearance setting.

2. **`ARCHITECTURE.md` §3.6.** Replace the current `Theme switch` bullet with:

   > **Theme switch:** Switching themes exchanges foreground and background globally; it is the only difference between the themes and the only place where foreground and background are exchanged (§3.5 forbids widget-level inversion). A theme is therefore a single foreground/background pair, supplied to the shader as theme data (e.g. a uniform or per-frame constant), never per cell. Dark is the default theme of every new window; each window owns its theme independently, the OS appearance setting is not consulted, and a new window does not inherit another window's theme (§4.2). The theme control is `Theme [ ◐ Light ]` / `[ ◑ Dark  ]` in the View menu (M2.18); manual switching keeps its existing behavior, and the glyph atlas is a coverage mask and is not regenerated on a theme change.

   The Light and Dark colour definitions and monochrome constraints remain unchanged.

3. **Accepted `docs/adr/0003-cell-ratio-3-5-to-4-9-and-ui-scale.md`, D7.** Amend D7 to read:

   > **D7 (strict monochrome themes):** the 2D UI uses only `#000000` and `#FFFFFF`. Dark (white on black) is the default for every new window; Light (black on white) is the alternate. The theme switch is the only foreground/background exchange; anti-aliased glyph coverage blends are permitted; no greys, tints, gradients, shadows or brightness multipliers (`ARCHITECTURE.md` §3.6).

4. **Accepted ADR 0003, D9.** Amend only its startup clause to read:

   > **D9 (per-window state):** `preferred_scale_percent`, `monitor_reference_scale`, `effective_k` and theme are per-window state (`ARCHITECTURE.md` §4.2); each new window starts at 100% and Dark and inherits neither scale preference nor theme from an existing window. Only the preference is user state; the other two scale values are recomputed for the window's current monitor.

   The per-window ownership rule remains. The OS appearance setting remains unused, and no cross-session theme persistence is introduced.

5. **Canonical UI rule in `docs/ui_design/README_UI.md` §28.3.** The dirty input snapshot already replaces the old Light startup sentence with Dark. For a clean baseline or any later copy that still has the old sentence, replace:

   > `100%` is the default preference, and the menu opens in Light theme by default.

   with:

   > `100%` is the default preference. Each new window starts in Dark theme and Wireframe shading; its initial renderer draws white foreground on a black background until the user changes the View-menu theme.

   This makes the lower-priority UI rule explicit and consistent after approval; it does not itself authorize implementation.

6. **Existing implementation plan `docs/plans/M2-ui-framework.md`.** Update the plan overview and M2.18 current startup statement to Dark. The M2.16 shader/default statements describe an earlier milestone; retain them as historical facts and annotate them as superseded for current startup behavior, rather than rewriting the history as if M2.16 originally shipped Dark. Exact current-rule replacement for the overview is:

   > Defaults of every new window: `preferred_scale_percent` 100%, Dark theme, neither inherited from another window nor detected from the OS (`ARCHITECTURE.md` §§3.1.1, 3.6).

   Exact current-rule replacement for M2.18 is:

   > `Theme [ ◐ Light ]` / `[ ◑ Dark  ]` toggles the uniform pair; no atlas or cell-buffer regeneration. Dark is the initial theme of every window; Light remains available through the existing View-menu control.

   The M2.16 shader-default line as well as its fixed-stage default line should be annotated as historical and superseded for current startup behavior.

No other accepted architectural rule is in conflict. `ARCHITECTURE.md` §4.2 already makes theme state window-local; its ownership wording need not change. The UI colour pair, menu labels, manual switching behavior, theme-only rendering uniform, and glyph-atlas behavior stay as they are.

## Baseline and uncommitted input snapshot

The shared source checkout was `main` at `1eb4a9a4b9fb45a4bbfaa223e348c0e5e9a69f1b`. Its Git index was empty. It had unrelated uncommitted M3 navigation, UI, rendering and documentation changes, including the requested Dark startup state. None were edited or staged. This plan was written in the separate worktree above, created from the same HEAD; that clean worktree does not silently include those changes.

The reviewable scoped input patch was captured outside the repository at `/tmp/newell-dark-theme-input.diff`; SHA-256: `2317a47c3de5e388d17d818a2ed757b9e21d79c98e9fd01cf9751ad354ffc328`. It includes the source snapshot for the files relevant to this proposal, not the entire dirty worktree. Key input content hashes at task start:

| Input | SHA-256 |
|---|---|
| `ARCHITECTURE.md` working-tree content | `6d0fb2ab2d00bffc02493026b7fd27bc76abd3d413dccd33eeedc9885ebc24e9` |
| `docs/ui_design/README_UI.md` working-tree content | `e1e22f4eb654e51cf877feabea197af9e96e03c1b701b6628ce1cda9f29f45d2` |
| accepted ADR 0003 working-tree content | `d1a7890a835c0565e12aa4dd37e5bae11400305589478a9d1f70ff6796d60496` |
| `docs/plans/M2-ui-framework.md` working-tree content | `b034ac72b0348333d52675ecccbb2a4fc28ea646bf68c37ab89cbc6022ea8e18` |
| `crates/newell-render/src/ui/theme.rs` working-tree content | `fa45f7911ec42f77e061d685de580cda8da3103b01ee8795d8606c332d2276e8` |
| `crates/newell-ui/src/shell/data.rs` working-tree content | `caac5fb3cd2d6a01dd2002ea51b95ea1d7e3004eff2d210f6f1e0869fd7fc326` |
| `crates/newell-app/src/app.rs` working-tree content | `df6a23c49ee73311d47151d61d0dfeea821c85338dc84e4edd839f92f7ca37f4` |
| `crates/newell-ui/src/shell/tests.rs` working-tree content | `e6cde89c1339a326ab2662aca7bda23b5c352869abd988694091b7567f4f7f62` |
| `crates/newell-ui/tests/it/menu_bar.rs` working-tree content | `5f4e18215d071b791223458397c62a1a9a167ba2820ec66997ef0d0effb18271` |
| `crates/newell-render/tests/it/ui_gpu.rs` working-tree content | `1f8c02ee5f3f5454a4fff8bacf6e32b8737d77a7d149a4393a4875118a083cfe` |

The dirty snapshot already contains `Theme::default() == Dark`, a default shell theme of Dark, and a Dark-default test. These are uncommitted inputs, not approved architectural evidence; the Executor must re-check them against the approved ADR and current branch before relying on them. The same snapshot changes the README §28.3 startup rule to Dark. The source at the clean baseline still requires Light in architecture, ADR 0003, M2 plan, and the corresponding README sentence.

## Implementation scope after approval

Keep the change limited to per-window startup initialization and its verification:

- `crates/newell-render/src/ui/theme.rs`: keep `Theme::default()` as `Theme::Dark` (or make that change if absent after the human-approved baseline is selected). `Theme::Light` remains a valid explicit value.
- `crates/newell-ui/src/shell/data.rs` and shell construction: initialize each new shell/view state with `ThemeChoice::Dark`; do not use a process-global mutable theme or copy a sibling window's current value.
- `crates/newell-app/src/app.rs`: preserve the existing per-window initialization path that maps the new shell's theme into its renderer before the first presented frame. If a window factory exists in the approved implementation, apply the same local default there; do not introduce window management as part of this change.
- Keep View-menu toggling and focus/popup behavior unchanged. Selecting Light or Dark remains a manual per-window action and changes the renderer's theme uniform without rebuilding the glyph atlas or cell buffer.
- Do not infer theme from OS appearance or add persistence. A newly constructed window state always starts Dark even if another existing window is Light.

No dependency, unsafe code, platform API, or new global state is needed. No geometry or renderer interface changes are expected beyond the existing theme default and initial synchronization.

## Existing tests to update or add

The scoped dirty snapshot already changes several of these checks. On the approved execution baseline, preserve valid existing assertions and adjust only tests that encode the old Light default:

- `crates/newell-render/src/ui/theme.rs` unit test: assert `Theme::default() == Theme::Dark` (`dark_is_the_default_theme`). Keep the exact black/white color assertions and inversion test.
- `crates/newell-ui/src/shell/tests.rs`: add or retain an explicit assertion that a fresh shell is `ThemeChoice::Dark`; verify the View popup still toggles Dark → Light → Dark with the existing focus/open behavior.
- `crates/newell-ui/tests/it/menu_bar.rs`: add or retain a fresh View-menu row assertion showing `[ ◑ Dark  ]`, and preserve existing explicit Light/Dark switching coverage.
- `crates/newell-render/tests/it/ui_gpu.rs`: keep the GPU assertion that the default frame is white-on-black, alongside explicit Light/Dark exact-colour and reversibility tests. This is GPU-ignored/manual where currently marked; it does not replace the state-level check.
- `crates/newell-app` startup tests: verify the initial shell theme reaches the renderer before first presentation. If a window-instance constructor can be exercised without a native window, instantiate two independent instances: changing the first to Light must leave the second Dark, and a subsequently created third instance must start Dark. If the current app has no multi-window factory, keep the acceptance at the per-window state-construction boundary and document that there is no OS window manager test in scope.

`docs/plans/M2-ui-framework.md` is the only existing milestone plan found with startup-theme claims. `docs/plans/M3-camera-mechanics.md` has no theme-default rule and is not part of this plan.

## Acceptance criteria

1. Every fresh application/window state initializes its own theme to Dark before its first frame: the UI shell reports Dark, the renderer reports/draws Dark, foreground is `#FFFFFF`, and background is `#000000`.
2. Starting another window never inherits the active theme of an existing window. A window opened while another window is Light starts Dark; switching one window never mutates another window's state.
3. The OS appearance setting has no effect. No cross-session persistence is added or implied.
4. The existing View-menu control still displays the current choice, manually switches both directions, keeps its current open/focus behavior, and routes the selected theme to that window's renderer.
5. Light remains black on white and Dark remains white on black; theme changes do not rebuild the glyph atlas or cell buffer.
6. The architecture, accepted ADR amendment, canonical UI rule and M2 plan use the same startup rule after human approval.

## Verification and handoff gate

This Architect stage runs no build, test, or application checks. After formal approval, the Executor should run focused theme and shell/menu tests, then the project-required formatting, Clippy, and workspace tests. GPU checks require the documented GPU environment and should be reported separately from deterministic CPU/state checks.

**Formal approval required before execution:** the human must approve draft ADR 0005 (or an equivalent explicit ADR amendment) and the exact architecture changes to §§3.1.1 and 3.6. After approval, the accepted decision must be recorded in the ADR history and the authoritative architecture text amended before implementation resumes. Updating the lower-priority README or this plan alone is not sufficient. The human controls the next role; this plan does not start it.
