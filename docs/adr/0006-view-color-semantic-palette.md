# ADR 0006: Per-window View > Color semantic palette

**Status:** Accepted — functional design and architectural/UI-specification amendments approved by the human in the 2026-10-26 instruction “Act as Newell’s Architect to resume View > Color.” This authorizes this documentation stage; production implementation requires the human’s implementation request.
**Proposed:** 2026-10-23. **Accepted:** 2026-10-26. **Role:** Architect (Codex).
**Maintained location:** `docs/adr/0006-view-color-semantic-palette.md` in `/Users/martin/Desktop/Newell` on `main`.

## Context and approval record

At review, `ARCHITECTURE.md` §§3.2–3.3 and §§3.5–3.6 plus accepted ADR 0003 D7 bound the UI to monochrome foreground/background ink. ADR 0005 changed only startup to Dark. `Application_Menu_Bar.txt` showed `□ Color`, while the higher-priority `README_UI.md` §28.3 supplied no Color semantics. Artwork did not authorize colour output.

The human explicitly approved the functional design of plan revision 3 and draft ADR 0006, including the proposed architectural and UI-specification amendments, and authorized applying them despite the Architect role’s ordinary documentation restrictions. These amendments are now applied. ADR 0005 is retained verbatim as the historical Dark-startup decision; this ADR supersedes its monochrome-only rendering description for the new Color mode without changing that startup decision.

The reviewed migration inputs were read only from the existing `view-color-architect/Newell` checkout: plan revision 3 SHA-256 `12cdcb4a091a71c7a86eefaac95eb81769205fede53f302efe0cf612db0b9500`; draft ADR SHA-256 `02e119643456c72c3335b565c423e73e8b3db379b7b8c8404c5f7bf8816cba42`. Both matched when recovered on 2026-10-26. Their historical source hashes identify the old review, not execution prerequisites. The maintained implementation plan is revision 4 in the main checkout and adapts the same behavior to current M4.2 interfaces. No old source file was restored.

## Accepted decision

Add a per-window `View > Color` checkbox, initially unchecked. It uses the existing View checkbox activation and open-popup behavior. `Theme` remains a separate, manually switched per-window Light/Dark choice, with its existing focused-row and open-popup behavior. Color and Theme form four combinations; changing either choice leaves the other unchanged. A new window starts Dark + Color unchecked and inherits neither choice from another window or OS appearance.

When Color is unchecked, Light is exactly black `#000000` ink on white `#FFFFFF`; Dark is exactly white `#FFFFFF` ink on black `#000000`. Existing glyphs, hover substitutions, cell geometry, atlas, hit map and monochrome snapshots remain unchanged apart from the newly specified View-menu row. No semantic role is visible while unchecked.

When Color is checked, the UI uses **three semantic ink roles** in the existing flat `CellGrid`: `Text`, `Contour`, `Action`. A cell has one role independent of its glyph and interaction-state `StyleId`; empty cells have no visible ink. Color is a palette lookup, never a focus, selection, pressed, disabled, checked, or hover visual. The role follows the cell through clipping, overlay composition, and hover substitutions. The four palette states are:

| Theme / Color | Background | Text, values, labels, tree guides | Contour | Action |
|---|---|---|---|---|
| Dark / off | `#000000` | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` |
| Light / off | `#FFFFFF` | `#000000` | `#000000` | `#000000` |
| Dark / on | `#071521` | `#FFFFFF` | `#41B4E5` | `#FFA24A` |
| Light / on | `#FFFFFF` | `#111827` | `#0969A0` | `#9A4D00` |

The Light + Color foregrounds have respectively 17.74:1, 5.93:1 and 6.11:1 contrast against white (sRGB WCAG formula); Dark + Color foregrounds have 18.45:1, 7.81:1 and 9.24:1 against navy. These are the intended source hex colours; target-format conversion and glyph coverage blending follow the existing shader rules. No white text is placed on the Light background.

`Text` owns ordinary text, numeric values and units, labels, editing caret, progress, inert scrollbar/viewport annotations, menu popup, dialog and selection-panel **title letters**, and all treeview hierarchy guides (`│`, `├`, `╰`, `╴`, playhead `─`) even beside blue pane frames. `Contour` owns box, panel, menu, dialog, selection-panel, ordinary edit/numeric-field and segmented-control outlines, separators, and their corner/junction cells. On a mixed title row the title letters are `Text` while the surrounding corner, horizontal stroke and frame junction cells are `Contour`; the role is assigned at each cell, never to the whole row. `Action` owns only visible actionable affordances: checkbox `□`/`■`, entire tree expand/collapse handle `[-]`/`[+]`, cyclic selector arrows `◀`/`▶`, selection-field `[`/`]` and trigger glyphs, numeric direction `+`/`-` and formula `☰`, scrollbar arrows and draggable thumbs, selection-list control glyphs, and other explicit controls. The adjacent `|` after a numeric direction sign is a `Contour` separator. Their labels and displayed values remain `Text` even if the whole row shares one WidgetId; button captions and menu command text are Text, with their delimiters/frames showing the available action. Inert NavCube outlines are `Contour`; its title is `Text`. Disabled controls keep the same semantic role; disabled is not a fourth colour.

The View Theme control has exact role-bearing spans in both `Theme [ ◑ Dark  ]` and `Theme [ ◐ Light ]`: `Theme` and the displayed `Dark`/`Light` value are `Text`; `[` and `]` and the `◑`/`◐` theme glyph are `Action`. The spaces inside and outside the brackets are empty cells with no visible ink, retaining the default `Text` role. The popup's visible outer `│` delimiters are `Contour` (and stay so when hover substitutes `╞`/`╡`). The Theme row remains one activation target with its existing action ID, `WidgetId`, pointer/keyboard behavior and glyph layout. Assign these roles from menu-item semantics; do not infer them from brackets or theme glyphs globally.

At a shared-border cell, the **visible stroke's structural owner** sets its ink role, independent of routing metadata. A border/junction of two pane frames is one `Contour` cell, including the navigation divider and horizontal pane divider. A divider between two button segments is `Contour` even though `shared_border` routes its two halves to different controls. A tree guide beside either is a separate `Text` cell. A popup written over the shell replaces the underlying cell and role together; no role leaks through. At a cell where a hierarchy guide actually joins a frame, the frame/junction owns that one cell (`Contour`); the guide's next nonshared cell stays `Text`. Interaction owner and colour owner are deliberately distinct.

In Color mode, double-stroke frame glyphs become the same-topology single-stroke glyphs at the **same cell coordinates**: `═→─`, `║→│`, `╔→┌`, `╗→┐`, `╚→└`, `╝→┘`, `╠→├`, `╣→┤`, `╦→┬`, `╩→┴`, `╬→┼`. Already-single frame glyphs `─│┌┐└┘├┤┬┴┼`, rounded popup/dialog corners `╭╮╰╯`, and single/dashed segment separators `│┆` stay as they are. These mappings cover corners, T joins, four-way joins, horizontal/vertical separators and shared borders. Only cells composed as `Contour` are mapped: a tree `│` or `├` is never changed by the frame rule. No atlas character is added.

Hover still makes the exact approved Unicode substitutions at the same slots: a left `│→╞`, a right `│→╡`, and a branch `╰╴→╘ `; Action Box segment boundaries `│` or `┆` use `╞`/`╡` as today. These hover glyphs are the explicit exception to the single-stroke contour form; they are not recoloured. The substituted cell keeps its role: blue frame remains blue, orange actionable boundary remains orange when explicitly owned as an Action affordance, and white tree branch remains white. For `╰╴→╘ `, the surviving `╘` inherits the former `╰` role and the blank keeps the former `╴` metadata. Hover never adds a palette entry, brightness, fade, animation, shader effect, or a new hit target.

The frame clear, including empty UI cells and viewport background, follows the selected palette background. The 3D model's display modes, mesh/material colours, selection rendering, depth and camera are unchanged; only the clear behind them changes. The glyph and future logo atlases remain coverage masks and are not rebuilt for Theme or Color. Text glyphs keep nearest sampling; future logos keep their separate filtered sampler and current cell allocations. An icon/logo cell uses `Text` ink unless its component explicitly makes it an Action affordance, in which case it uses `Action` ink; when Color is unchecked both roles resolve to the exact current monochrome foreground. The UI remains one batched GPU draw call with per-cell role lookup in that call. O(1) hit testing and MVC boundaries remain intact. A Theme change with an open View menu changes the visible Theme-row text and therefore requires a recomposed cell upload; a palette-only update is valid only when the glyph grid is unchanged.

### Visual allocation considered

| Allocation | Visual result | Decision |
|---|---|---|
| Requested: blue contours, orange affordances, white/dark ordinary ink | Reads as a quiet structural layer plus a small, repeatable action layer; values remain high contrast | **Accepted** |
| Quieter A: blue only outer panes, white internal/dialog contours, orange affordances | Fewer blue lines but obscures which internal lines bound fields or dialogs; violates the requested contour allocation | Reject for this mode |
| Quieter B: blue contours, orange only focused/hovered affordance | Less orange at rest but makes colour a state indicator and weakens discovery; conflicts with hover's no-colour rule | Reject |

The btop image informs only the structural blue/action orange/quiet text hierarchy. Newell retains its own layout and features.

## Applied authoritative amendments

- `ARCHITECTURE.md` §§3.2–3.3: complete logo coverage-mask and resampling clauses now specify Text/Action ink under both Color states; padded atlas, major/minor allocations, filtered logo sampler, nearest glyph sampling, shader centering and one UI draw remain binding. Logos are not implemented by this increment.
- `ARCHITECTURE.md` §§3.5–3.6: semantic roles, exact four palettes, same-cell frame map, title/Theme spans, shared-cell ownership and hover inheritance replace blanket monochrome-only constraints. The caret uses Text without caret-specific GPU data. Viewport clear follows the palette; scene display modes do not change. Theme changes upload cells whenever the visible menu label changes.
- `ARCHITECTURE.md` §§3.1.1/4.2: new windows start Dark + Color unchecked; both choices are local to each window, independent of OS appearance and document identity.
- ADR 0003 D7/D9: amended for the accepted palette and per-window Color flag. D8 is reconciled with the existing M2.25 hover authorization and Text caret; it no longer contradicts the architecture by permanently banning all hover. No new hover behavior is added.
- `docs/ui_design/README_UI.md` §§5.9–5.12, 28.3, 36: define role ink, coverage, caret and hover inheritance; add the unchecked Color row and checked state, independent Theme/Color behavior, exact palette and mixed titles. The `.txt` menu artwork already contains the unchecked row and is retained. Existing historical reference fixtures and M4.2 live-tree semantics remain intact.
- ADR 0005 remains unchanged; its “no palette expansion” statements describe that earlier decision only. No old plan or reference image overrides this accepted decision.

## Consequences and implementation boundary

The renderer transfers one semantic role word beside each glyph coordinate and looks up three inks in a per-window palette, in the existing UI draw. Cell dimensions, routing, model identities and document events are unaffected. Literal Action Box rows, dynamic tree rows, popup/dialog/selection titles and the Theme control require explicit role annotations; glyph-only global recolouring is rejected. No new dependency, font asset, render pass, model edit or logo implementation is authorized.

The 2026-10-26 request ends after documentation and plan revision 4. It does not start Executor or another review. Runtime, GPU, visual, physical-device and live multi-window acceptance remain unverified until the relevant implementation and checks are requested and performed.
