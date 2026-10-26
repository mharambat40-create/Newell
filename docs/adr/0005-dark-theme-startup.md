# ADR 0005: Dark Theme for Every New Window

**Date:** 2026-10-22

**Status:** Accepted — approved by the human on 2026-10-22; amends the startup-theme clauses of ADR 0003 D7 and D9.

**Agent Author:** Architect (Codex)

## 1. Context

The requested product behavior is that every new window starts in Dark theme. At the time of this decision, the canonical UI specification stated this behavior, while the binding `ARCHITECTURE.md` §§3.1.1 and 3.6 and accepted ADR 0003 decisions D7 and D9 still required Light. Under the repository's source precedence, the UI document could not override those accepted architectural rules. This decision resolves that conflict.

Theme is already specified as per-window view state. The startup default can change without changing the monochrome palette, manual theme switching, View-menu behavior, renderer interface, or theme-independent glyph atlas.

## 2. Options Considered

* **Option A — Keep Light as the startup theme.** This preserves accepted architecture but rejects the human's requested behavior and the updated UI direction.
* **Option B — Make Dark the default for each new window.** This matches the requested behavior while preserving independent per-window state and the existing Light/Dark control. It requires explicit amendments to the authoritative architecture and ADR decision text.
* **Option C — Derive the initial theme from OS appearance or another window.** This does not match the request and would make the stated per-window startup rule dependent on external or sibling state.

## 3. Decision

Adopt Option B, approved by the human on 2026-10-22. Every new window initializes its own theme to Dark, independent of other windows and OS appearance. Existing windows keep their own current theme. The View menu continues to offer the current Light/Dark toggle with its existing behavior. Light remains black on white; Dark remains white on black. A theme change continues to exchange only the renderer's foreground/background pair and does not regenerate the glyph atlas or cell buffer.

### Authoritative amendments

**`ARCHITECTURE.md` §3.1.1**, replace:

> A new window starts with preference 100% and the Light theme (§3.6), regardless of other windows and of any OS appearance setting.

with:

> A new window starts with preference 100% and the Dark theme (§3.6), regardless of other windows and of any OS appearance setting.

**`ARCHITECTURE.md` §3.6**, replace its `Theme switch` bullet with:

> **Theme switch:** Switching themes exchanges foreground and background globally; it is the only difference between the themes and the only place where foreground and background are exchanged (§3.5 forbids widget-level inversion). A theme is therefore a single foreground/background pair, supplied to the shader as theme data (e.g. a uniform or per-frame constant), never per cell. Dark is the default theme of every new window; each window owns its theme independently, the OS appearance setting is not consulted, and a new window does not inherit another window's theme (§4.2). The theme control is `Theme [ ◐ Light ]` / `[ ◑ Dark  ]` in the View menu (M2.18); manual switching keeps its existing behavior, and the glyph atlas is a coverage mask and is not regenerated on a theme change.

**Accepted ADR 0003, D7**, replacement:

> **D7 (strict monochrome themes):** the 2D UI uses only `#000000` and `#FFFFFF`. Dark (white on black) is the default for every new window; Light (black on white) is the alternate. The theme switch is the only foreground/background exchange; anti-aliased glyph coverage blends are permitted; no greys, tints, gradients, shadows or brightness multipliers (`ARCHITECTURE.md` §3.6).

**Accepted ADR 0003, D9**, replacement:

> **D9 (per-window state):** the selected UI Scale label, its mapped rendering factor, `monitor_reference_scale`, `effective_k` and theme are per-window state (`ARCHITECTURE.md` §4.2). Each new window starts at UI Scale label 100% (rendering factor 80%) and Dark, and inherits neither scale choice nor theme from an existing window. The selected label is user state; the factor is mapped from the label, and the remaining scale values are recomputed for the window's current monitor. UI Scale's current label mapping is defined by D14 in ADR 0003.

Only the startup default clauses change. Per-window ownership, the default UI Scale label 100% (mapped to rendering factor 80%), monitor-derived scale values, OS-appearance independence, monochrome palette, explicit theme switching and renderer behavior remain in force. The approved amendments are reflected in `ARCHITECTURE.md` and ADR 0003.

## 4. Consequences

* **Positive:** The requested startup behavior is defined at the same authority level as the existing Light rule; fresh windows are deterministic and independent.
* **Positive:** Existing windows can remain Light after a user choice, while any later-created window starts Dark.
* **No palette or rendering expansion:** only the initial theme value changes. The existing View-menu labels and manual switching remain intact.
* **Implementation impact:** Align the per-window shell and renderer defaults and their initial synchronization; update default-state tests, menu/startup tests, and relevant M2 plan wording. No new dependency, platform integration, unsafe code, or global theme state is required.
* **Decision record:** ADR 0003's history and `ARCHITECTURE.md` now reflect the approved Dark startup rule. Implementation must still be verified against the current checkout.
