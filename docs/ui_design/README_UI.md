# Newell UI/UX Specification

**Status:** Canonical semantic specification for the ASCII UI references stored in `docs/ui_design/`.  
**Subordinate to:** `ARCHITECTURE.md`

This document defines the intended behaviour and meaning of the ASCII UI
references in this directory.

The `.txt` files are not approximate sketches. They are fixed-grid interface
references. Every character cell matters:

- spaces are part of the layout;
- Unicode border characters are part of the layout;
- branch characters express ownership between controls;
- field brackets delimit interactive regions;
- selector arrows, checkboxes and symbols express UI state;
- Action Box lines have fixed horizontal positions;
- Action Box variants define alternate forms of existing lines or branches.

The ASCII references therefore act as visual state specifications.

This README explains their semantics. It MUST NOT silently modify their spacing,
dimensions, labels or topology.

---

# 1. Source precedence

The UI specification follows this precedence:

1. `ARCHITECTURE.md`
2. `docs/ui_design/README_UI.md`
3. `.txt` references in `docs/ui_design/`
4. implementation plans under `docs/plans/`

`ARCHITECTURE.md` defines the binding architectural rules.

This README defines the semantic interpretation of the UI.

The `.txt` files define the exact visual representation of each documented
state.

If a `.txt` reference is modified in a way that changes the meaning, structure,
spacing or available states of the interface, this README must be updated with
it.

---

# 2. ASCII fidelity contract

## 2.1 One character = one logical UI cell

The UI is a terminal-like character grid.

A reference such as:

```text
│ │ Depth   [+|0.0 mm       ☰]│
```

must be interpreted cell by cell.

Spaces MUST NOT be:

- removed;
- collapsed;
- automatically aligned differently;
- replaced by proportional spacing;
- reformatted by a generic layout engine.

The position of every visible character is intentional.

---

## 2.2 Action Box width

Current Action Boxes use a **31-cell external width** for their normal frame and
content rows.

For example:

```text
╭─Extrude─────┬───────────────╮
│    Solid    │    Surface    │
│ Profile     [Select…       ]│
╰─────────────────────────────╯
```

Each of these rows occupies exactly 31 character cells.

The same fixed horizontal grid is used by the current:

- Extrude Action Box;
- Revolve Action Box;
- Loft Action Box;
- Sweep Action Box;
- Boolean Action Box.

An implementation MUST NOT resize a field merely because its current value is
shorter.

For example:

```text
[Select…       ]
```

reserves the complete field width even though `Select…` itself uses only part of
it.

---

## 2.3 State-specific height

The Action Box width is fixed.

Its height is determined by the currently visible lines.

Progressive disclosure may therefore add or remove rows while retaining the
same horizontal grid.

Example:

```text
│ │ □ Start Offset            │
```

can become:

```text
│ │ ■ Start Offset ◀ Blind ▶  │
│ │ ╰╴Dist  [+|0.0 mm       ☰]│
```

The second state contains one additional row.

The variant is not another independent widget appended elsewhere in the panel.
It is the expanded state of the same `Start Offset` line.

---

## 2.4 Variant substitution rule

Sections named:

```text
"Start Offset variants"
"End Offset variants"
"Symetric variants"
"Merge with all variants"
```

do not describe independent Action Boxes.

They describe **all visual forms that the corresponding line or branch may
take inside the Action Box**.

A composed Action Box is created by:

1. choosing the appropriate complete base Action Box;
2. locating the relevant line;
3. replacing that line or branch by the active variant;
4. recursively applying variants belonging to newly revealed children.

The exact indentation and branch topology from the variant MUST be retained.
Replacement is limited to the line or branch the variant belongs to; sibling
controls are preserved, and complete base states outrank partial snippets
(§2.6).

---

## 2.5 Variants do not redefine the panel width

For example, these three states all occupy the same Action Box width:

```text
│ │ □ Start Offset            │
```

```text
│ │ ■ Start Offset ◀ Blind ▶  │
│ │ ╰╴Dist  [+|0.0 mm       ☰]│
```

```text
│ │ ■ Start Offset ◀ Up to ▶  │
│ │ ╰╴Target  [Select…       ]│
```

The text field is aligned according to the reference, not according to a generic
form layout.

---

## 2.6 Complete base states versus partial variant snippets

`Action_Box_Extrude.txt` remains the visual authority for the Extrude Action
Box; neither this README nor an implementation may edit its artwork. It holds
two kinds of material, with different authority:

- **Complete base states** (the three `"Extrude Action Box"` boxes, including
  their `Same for Surfaces` note). Every row of a complete state is documented
  and authoritative for that state.
- **Partial variant snippets** (`"Extend variants"`, `"Start Offset variants"`,
  `"End Offset variants 1/2"`, `"Symetric variants"`, `"2nd side variants"`,
  `"Merge with all variants"`, the two `Invert` lists). A snippet is
  authoritative for the rows it shows: their text, indentation and glyphs. It
  is an excerpt. **The absence of a row from a snippet is not an instruction to
  delete that row from the composed box.** A documented row is removed only
  when the reference states the removal explicitly (for example an active
  `Symetric` removes `2nd side`: the snippet pair shows both forms and §8.6
  states the rule).

**Branch and sibling.** The *branch* of a line is the line plus the rows
that hang under it, i.e. the rows that carry the branch column (`│` or `╰╴` in
cells 2–3) below it. A *sibling control* is any row of the box outside that
branch at the same level: for Extrude, `Profile`, `Symetric`, `2nd side` /
`Second end position`, `Merge with all` / `Targets`, and the OK/Cancel row.
Substituting an Extend variant replaces **only** the `Extend` row and the rows
of the Extend branch. Every sibling control keeps its position and text. An
implementation MUST NOT replace "everything between `Extend` and `OK`".

**Extend branch composition** (cells are exactly as in the references; only the
rows named here are involved):

| `Extend` | Rows of the branch, in order | Authority |
|---|---|---|
| `Blind` | `Depth`, `Along`, `Starting Offset`, `End Offset` | the complete base boxes (rows 9–12 of each) |
| `Up to` | `Selection`, `Offset` | the `"Extend variants"` snippet |
| `Through all` | `Invert`, `Start Offset` | the `"Extend variants"` snippet |

For `Blind`, the complete base boxes and the Blind snippet differ: the snippet
ends the branch at `Starting Offset` and omits `End Offset`. The base box is a
complete state and wins, so `End Offset` is kept as the last child and
`Starting Offset` is not last. The snippet's `│ ╰╴□ Starting Offset` row is the
closing connector of that partial excerpt, not a deletion of `End Offset`.
`Up to` and `Through all` have no complete base state in the reference; their
branches are exactly the snippet rows. The reference documents no `End Offset`
under them, so none is composed (see *Open reference points* below).

**Last-child form of an expanded offset.** The `Start Offset` / `End Offset`
variants are written for a non-last child (rows start `│ │ `). When an
expanded offset is the last child of the branch (`End Offset` under `Blind`,
`Start Offset` under `Through all`), compose it with the prefix transformation
and nothing else: in the first row cells 1–4 `│ │ ` become `│ ╰╴`, and in each
descendant row cells 1–6 `│ │ ╰╴` become `│   ╰╴` (blank continuation). This is
the convention the reference already shows for last children: `│ ╰╴□ End
Offset`, `│ ╰╴□ Start Offset`, and `│ ╰╴■ 2nd side ◀ Blind ▶` over `│   ╰╴Depth`.
Row width stays 31 cells; text, field columns and brackets are unchanged. The
transformation is part of composition; it does not modify any `.txt` file.

**Open reference points** (reproduced as written, to be settled only by editing
the `.txt` references together with this README):

1. Label drift: the collapsed Blind-form row reads `Starting Offset`; its
   expanded variant and the `Through all` form read `Start Offset`. Each row is
   reproduced with its own label.
2. `Up to` and `Through all` show no `End Offset`; if the design intends one it
   must be added to the reference.
3. Enabling Up to Offset reveals one unsigned formula-capable length below Offset. Its magnitude is a clearance toward the start, measured along the extrusion direction. Zero terminates at the target.

---

# 3. Anatomy of an Action Box line

Consider the Extrude line:

```text
│ │ Depth   [+|0.0 mm       ☰]│
```

It must be decomposed as follows.

| Characters | Meaning |
|---|---|
| first `│` | left border of the Action Box |
| following space | fixed horizontal padding |
| second `│` | dependency branch: this line belongs to the parent state above |
| following space | fixed child indentation |
| `Depth` | parameter label |
| spaces after `Depth` | fixed alignment before the field |
| `[` | opening boundary of the editable field |
| `+|` | positive orientation of the parameter |
| `0.0` | editable numerical magnitude |
| space | separation between value and unit |
| `mm` | engineering unit |
| spaces after `mm` | unused cells reserved by the field |
| `☰` | secondary field action: Formula editor |
| `]` | closing boundary of the field |
| final `│` | right border of the Action Box |

Consequently:

```text
Depth
```

is not part of the editable field.

```text
+|
```

is not part of the numerical magnitude.

```text
0.0
```

is either the untouched initial magnitude placeholder or a real magnitude.
Its explicit provenance, never its spelling, determines placeholder behavior (§5.13).

```text
mm
```

is its unit.

```text
☰
```

is an independent interactive region embedded at the end of the field.

---

# 4. Border and branch semantics

The same vertical character may have different semantic roles depending on its
position.

Example:

```text
│ │ Depth   [+|0.0 mm       ☰]│
```

- first `│`: Action Box border;
- second `│`: dependency branch;
- last `│`: Action Box border.

Example:

```text
│ ╰╴□ End Offset              │
```

means:

- first `│`: left Action Box border;
- `╰╴`: last child of the current parent branch (a two-cell connector: `╰` then `╴`);
- `□ End Offset`: child control;
- final `│`: right Action Box border.

Example:

```text
│   ╰╴■ End Offset ◀ Blind ▶  │
│     ╰╴Dist  [0.0 mm       ☰]│
```

The additional indentation means that `End Offset` itself belongs to another
parent branch, and `Dist` belongs to `End Offset`.

The tree topology is therefore semantically meaningful.

---

# 5. Shared glyph semantics

## 5.1 Checkboxes

```text
□ Option
```

means disabled / unchecked.

```text
■ Option
```

means enabled / checked.

A checked state may reveal additional child rows.

---

## 5.2 Cyclic selectors

Example:

```text
│ Extend   ◀     Blind     ▶  │
```

`◀` selects the previous available value.

`▶` selects the next available value.

`Blind` is the current value.

The spaces around the value are part of the fixed layout.

A cyclic selector only displays one state at a time.

---

## 5.3 Segmented selectors

Examples include:

```text
│    Solid    │    Surface    │
```

and:

```text
│  New  │   Add   ┆  Remove   │
```

and:

```text
│   Add   │  Cut  ┆   Inter   │
```

All choices remain visible at the same time.

The surrounding border topology identifies the active segment.

The `╴` and `┆` glyphs belong to this topology. They do not mean that a segment
is disabled.

> **Temporary accepted deviation (until M2.21).** The segmented selector built
> in M2.7 marks the active segment with `║` separators, which is not a
> documented structure. It is accepted temporarily and MUST NOT be extended or
> copied to new widgets. M2.21 replaces it with the exact border topology of
> `Action_Box_*.txt` and `Interface_Elements.txt` "Head Action Box variants"; the
> deviation ends there.

---

## 5.4 Direction prefix

Directional numerical fields use:

```text
+|
```

or:

```text
-|
```

Example:

```text
Angle   [+|0.0 deg      ☰]
```

versus:

```text
Angle   [-|0.0 deg      ☰]
```

The prefix controls orientation.

The numeric value remains the magnitude.

Therefore reversing the direction does not require entering a negative
magnitude.

---

## 5.5 Selection fields

Empty:

```text
[Select…       ]
```

One selected element:

```text
[1 Element    ☰]
```

Several selected elements:

```text
[N Elements   ☰]
```

`Select…` means that the user can initiate entity selection.

When a selection exists, the compact field displays the number of selected
elements instead of trying to display all names.

---

## 5.6 Contextual meaning of `☰`

`☰` is intentionally context-sensitive.

### Numeric field

```text
[+|0.0 mm       ☰]
```

`☰` opens the Formula editor.

### Populated selection field

```text
[1 Element    ☰]
```

`☰` opens management of the current selected entities.

### Profile-selection row

```text
││ ☰ Profil_3             × ╻││
```

The leading `☰` is a reorder/drag handle for the profile.

It does **not** open the Formula editor.

The action associated with `☰` is therefore defined by the containing control.

---

## 5.7 Remove symbol

Inside an ordered profile list:

```text
×
```

removes that row from the selected profile list.

---

## 5.8 Scroll indicators

The reference uses:

```text
▲
┃
╻
╹
▼
```

`▲` and `▼` indicate scrolling toward the beginning or end.

`┃`, `╻` and `╹` represent positions or portions of the scrollbar thumb.
Selection-box content lists follow the five-row geometry and half-cell model
in §21; these rules do not change other scrollbars.

---

## 5.9 Progress glyphs

Progress uses:

```text
░ ▒ ▓ █
```

A complete cell corresponds to 20%.

The partial characters divide the following 20% cell into 5% increments.

Block-element limits (`ARCHITECTURE.md` §3.1, Unicode continuity): `░ ▒ ▓ █`
tile horizontally, so the five progress cells join seamlessly in their single
row. Vertically a block element covers only the line box (`L = 8k` px of the
cell's `H = 9k`), so a filled progress cell shows `k` px of background in total
(`k div 2` px above, the rest below). This is accepted and MUST NOT be
corrected by stretching, extra rows or a procedural rasteriser. Block elements
are never stacked vertically in any reference. The shade glyphs `░ ▒ ▓` are the
font's own patterns at the uniform scale; their coverage blend uses the
current palette's Text ink over its background (`ARCHITECTURE.md` §3.6).

---

## 5.10 Interaction state visuals

The only hover visual in Newell is the discrete ASCII hover highlight of
§5.12 (M2.25, `ARCHITECTURE.md` §3.5). Hover changes no semantic ink role or palette entry and adds no brightness, fade,
animation or shader effect; it is not a focus, selection or pressed visual.

Logical focus, selection, pressed, disabled, checked and expanded state exist
for keyboard navigation, activation and dragging. They are drawn **only** by
the ASCII structures documented in this directory:

- `□` / `■` for unchecked / checked and hidden / visible (§5.1, §26);
- `[-]` / `[+]` for expanded / collapsed tree nodes (§26);
- the value text between `◀` and `▶` for cyclic selectors (§5.2);
- the border topology of segmented selectors (§5.3);
- `+|` / `-|` for direction (§5.4);
- the selection-field texts `Select…`, `1 Element`, `N Elements` (§5.5);
- the text-editing caret `▏` while a field is actively edited (§5.11);
- the ASCII hover highlight `╞` `╡` `╘` on the single hovered target (§5.12);
- the exact row-local tree hover and heavy selection glyphs (§26.4).

A state with no documented structure has no visual. An implementation MUST NOT
add a generic foreground/background inversion, a marker such as `>` or `<`, an
underline, or any third colour or intensity to represent focus, selection or
pressed state. Focus markers are not invented: a focused control without a
documented structure has no visual (the keyboard focus target is observable
only through its effect). Except for the narrowly scoped placeholder ink of §5.13, no interaction state introduces a colour, intensity or inversion. `View > Color`
selects a fixed semantic palette independent of interaction state; Light/Dark
selects the corresponding background and role colours (`ARCHITECTURE.md` §3.6).
When Color is unchecked, the exact black/white themes apply except for the
neutral placeholder ink of §5.13.

---

## 5.11 Text-editing caret

`▏` (U+258F, LEFT ONE EIGHTH BLOCK, listed in `Special_ASCII.txt`) is the
caret of a text field that is **actively being edited** (`Rename`, `Enter Name`,
the value of a numeric field, the Formula editor text area). It is the only
approved indicator of editing and is specified for M2.23.

- **When.** Drawn only while exactly one field is in edit mode. Not drawn for
  merely focused, selected, hovered (§5.12 changes border glyphs only) or pressed fields.
- **Colour.** The caret uses `Text`: black in unchecked Light, white in unchecked
  Dark, `#111827` in Light + Color and white in Dark + Color. It has no independent
  focus/edit colour, inversion or blink (`ARCHITECTURE.md` §§3.5–3.6). It is an
  ordinary cell in the flat buffer and needs no additional draw call or
  caret-specific shader data.
- **Value untouched.** The caret is a composition-time artefact. The edited
  text value (the model/UI state string) never contains `▏`, and copying,
  validating or committing the value never sees it.
- **Geometry.** The glyph is the left eighth of a cell: ink spans
  `x ∈ [0, W/8) = [0, k/2)` px of the cell (anti-aliased at odd `k`) and, as a
  block element, the line box vertically (`L = 8k` of `H = 9k` px).
- **Placement.** For edited text of `T` characters, caret index `c` (`0 ≤ c ≤ T`)
  and a text area of `N` cells, the composed display string is
  `D = text[0..c] + ▏ + text[c..] + static suffix` (the suffix is fixed text
  that follows the editable text, such as ` mm`). The caret occupies **its own
  cell**, so the characters after the caret move one cell to the right while the
  caret is shown; the cells reserved by the field (README §2.2) never change.
- **Full field.** The visible window of `D` starts at `s = max(0, c + 1 − N)` and
  shows `D[s .. s + N)`. Therefore (a) while `c < N` nothing scrolls and any
  cells of `D` beyond the right boundary of the text area are clipped whole-cell
  (the static suffix is clipped before editable text, which is clipped before the
  caret); (b) when the caret reaches the last cell, the text scrolls left one
  cell per additional character and the first characters leave the left
  boundary. The caret is always visible, and is never drawn over the field
  delimiters `[` `]`, the direction prefix `+|` or the `☰` cell. Clipping is
  per whole cell; no `…`, marker or other indicator of hidden text is added.
- **Not a rejection rule.** A field being visually full does not reject input;
  any maximum text length is a data rule outside this specification.
- **Keyboard only in M2.23.** Pointer placement of the caret is not specified.

---

## 5.12 ASCII hover highlight (M2.25)

Binding rules: `ARCHITECTURE.md` §3.5 (*ASCII Hover Highlight*). Exact examples:
`Interface_Elements.txt` ("Effet Hover") and `Special_ASCII.txt`. The hover
highlight replaces a few border glyphs of the one row or button under the
pointer. Nothing else changes. Every replacement inherits the replaced cell's
semantic ink role (`ARCHITECTURE.md` §3.6), including Contour popup borders,
Action affordances and Text hierarchy guides. For `╰╴→╘ ` the surviving `╘`
retains the old `╰` role; the blank retains the old `╴` metadata. Hover adds no
colour, brightness or shader effect.

**The four non-tree transformations.** Each pair is the exact approved text, spaces
included. Every transformation keeps the cell count of the row.

| Kind | Normal | Hovered | Cells changed |
|---|---|---|---|
| Menu | `│ Bridging Curve  │` | `╞ Bridging Curve  ╡` | left `│` → `╞`, right `│` → `╡` |
| Parameter | `│ │ Along     [Select…       ]│` | `│ ╞ Along     [Select…       ]╡` | branch `│` → `╞`, right `│` → `╡` |
| Checkbox | `│ ╰╴□ End Offset              │` | `│ ╘ □ End Offset              ╡` | `╰╴` → `╘` + space, right `│` → `╡` |
| Button | `│      OK      │    Cancel    │` | `╞      OK      ╡    Cancel    │` | left `│` → `╞`, following `│` → `╡` |

The Checkbox normal text is the canonical row of `Action_Box_Extrude.txt`
(right border `│`), also used in `Interface_Elements.txt` ("Effet Hover") and
`ARCHITECTURE.md` §3.5. The right border becomes `╡` only in the hovered state.

**Context-sensitive junction rule (non-tree controls).** The substituted glyph depends on the glyph
being replaced and on the side of the hovered target, never on the row's text:

| Slot | Original glyph | Hovered glyph |
|---|---|---|
| left boundary of the hovered target | `│` | `╞` |
| left boundary, last-child connector (two cells) | `╰` `╴` | `╘` + one space |
| right boundary of the hovered target | `│` | `╡` |

- The **left slot** is the innermost cell on the left of the target's content
  that is a `│` or the `╰╴` pair: the Action Box frame `│` of a row without a
  connector, the branch `│` of a row with one (the frame `│` before it is not
  changed), the nested `╰╴` of a deeper row (the shallower `│` and `╰╴` are not
  changed), the popup frame `│` of a menu row, or the frame/separator `│` that
  precedes a button.
- The **right slot** is the `│` immediately after the target's content: the
  frame `│` of a row, or the `│` separator that follows a button.
- Except for the header `┆` extension below, a slot holding any other glyph
  (`║`, `├`, `╭ ╮ ╰ ╯`, `─`, `┆`, a space) is never substituted, and a slot
  is never created where none exists. A target
  without a substitutable slot has no hover visual.
- `╞` and `╡` replace `│`; for Action Box header buttons they also replace
  a `┆` separator at that button's exact boundary. `╘` only replaces the `╰╴`
  pair. Header hover never changes the horizontal active-segment topology.
- Examples derived from the same rule (not additional transformations):
  `│ │ □ Starting Offset         │` → `│ ╞ □ Starting Offset         ╡`
  (row 19 of `Interface_Exemple.txt`, a checkbox row with a branch `│`);
  `│ ╰╴End       [Select…       ]│` → `│ ╘ End       [Select…       ]╡`;
  `│      OK      │    Cancel    │` with `Cancel` hovered →
  `│      OK      ╞    Cancel    ╡`; the same rule applies to the OK/Cancel row
  of the modals (`Rename`, `Enter Name`).

**Hoverable targets.** The four original kinds below and tree rows (§26.4) are eligible:

- **Menu**: an enabled item row of an application or CommandBar popup menu
  (§28, §29), including the rows that contain `◀ ▶`, `[ ◐ Light ]` or a
  checkbox: the whole row is the target.
- **Parameter**: any Action Box parameter row, including fields `[ … ]` and
  cyclic selectors `◀ value ▶`. The target is the whole row: the interactive regions of the row (`+|`, the value, `☰`,
  §37) all light the same two slots.
- **Checkbox**: an Action Box row that contains `□` or `■`. The target is the
  whole row.
- **Button**: one Action Box header segment (`Solid`, `Surface`, `New`, `Add`,
  `Remove`, or Boolean `Add`, `Cut`, `Inter`) or one `OK`/`Cancel` button of an
  Action Box or modal. Only that button's own two boundaries change:
  `│` or `┆` becomes `╞` on its left and `╡` on its right.

Action Box parameter labels, padding and cyclic value text inherit a hover
region from the same row. Resolve this only for hover, using at most the fixed
31 cells of the Action Box row; preserve click routing and all existing
region-specific WidgetIds. This applies to every Action Box, including Profile,
Start type and Magnitude in Loft.

Tree rows add the exact transformations and O(1) part normalization in §26.4–§26.5.
Every other control outside these documented targets (MenuBar and CommandBar
labels, window controls, scroll indicators, the progress segment, text fields of
modals and inert chrome) has no hover visual and never becomes the hovered target.

**One hovered target.** The Controller stores at most one
`hovered: Option<WidgetId>`. Moving between two regions of the same row (for
example `+|` to `0.0`) changes the stored id but not the glyphs.

**Eligibility.** Except for the Action Box label/value extension above, a cell
is hoverable exactly when a left-button press on it,
now, would route to an enabled, interactive widget of one of the documented kinds
(§25.1 precedence, rule 1, and the overlay rules of §27.1), so:

- Disabled controls never hover. The Action Box label/value extension above is
  hover-only; other non-interactive cells never hover. Border, frame and inert
  chrome cells are not targets; they carry no `widget_id` and this milestone
  gives them none.
- While a menu, popup or modal is open, only cells of the topmost overlay are
  eligible. Everything it covers, and everything outside it, is not hoverable.
  Opening an overlay clears a hover it obscures; closing it clears the hover.
- An active window resize owns the pointer (§25.1): hover is cleared when the
  resize starts, is not updated while it runs, and is acquired again only by the
  first pointer move after it ends. A resize zone with no `widget_id` is not a
  target.
- A cell that carries a `widget_id` wins over a resize zone (§25.1 rule 1), so a
  popup cell on the ring can hover.
- Hover never changes press, click, drag, focus, selection or keyboard
  behaviour, and does not change which cell or `widget_id` any hit-test returns.
  The hit map of a hovered frame is identical to the normal frame.

**Lifecycle.**

- Set and changed only by pointer motion; cleared by pointer exit
  (`CursorLeft`) or by the pointer moving onto a cell that is not eligible.
  While the Features playhead is captured, tree-row hover is suppressed in both
  trees; ending or cancelling that capture re-resolves hover from the current
  pointer and published hit map.
- After every recomposition (state change, overlay open/close, scroll, panel
  drag, scale or theme change, resize end) the Controller re-checks, with one
  O(1) lookup, that the cell under the last known pointer position still resolves to
  the hovered id (tree parts normalize to their row id in O(1), §26.4); if not, or if the target no longer exists, hover is cleared.
  A recomposition never acquires a new target without a pointer motion, except
  when a captured Features playhead ends or is cancelled: then hover is resolved
  once from the current pointer and published semantic target, never from a
  saved row or slot.
- A redraw is requested only when the stored value changes (`None` → `Some`,
  `Some(a)` → `Some(b)`, `Some` → `None`). Motion inside one target requests
  nothing. Hover wakes no timer and adds no periodic redraw.
- On exit the current non-hovered glyphs reappear (selected tree glyphs if
  selected, otherwise normal glyphs) in the same frame that clears the id: the
  highlight exists only at composition time and is never written into persistent
  state, so there is nothing to restore later and no leftover cell.

**Cost.** Pointer motion resolves the cell with the existing integer division
and one `widget_id` read (O(1), `ARCHITECTURE.md` §3.4), compares it with the
stored value, and stops. No bounding-box scan, retained widget tree, extra
render pass, uniform or draw call exists. The composer substitutes at most
three cells (a Checkbox: the pair and the right slot).

**Not hover.** No colour, brightness, inversion, underline, fade, transition or
animation; no change of text, spacing, width, height or layout; no shader or
atlas change (`╞ ╡ ╘` are already in the required glyph set, §35). The M2.24
progress animation remains the only animation (`ARCHITECTURE.md` §3.5).

**Reference states.** Hover alone does not modify normal reference artwork.
The 2026-10-26 tree amendment updates the endpoint and adds the explicit §26
reference states. A snapshot without a pointer (hover `None`) shows the normal glyphs;
hovered snapshots are separate states.

---

## 5.13 Initial input placeholders (2026-10-27)

Authorized by the human together with `ARCHITECTURE.md` §3.6.1. This exception
applies only to the initial numeric magnitude and the initial `none` instruction
in §23's Enter Name Validation Operation Box. A real zero, an accepted or
Formula-derived value, an existing operation/name (even literally `none`), and
user-entered drafts are ordinary Text. Selection fields, labels, diagnostics,
buttons, titles and the Formula `Fx = ` prefix are excluded.

**Ink.** Use the semantic Placeholder role only for placeholder characters:
Dark/off `#CCCCCC`, Light/off `#333333`, Dark/on `#CDD0D3`, Light/on `#414652`.
These are the deterministic 80% Text / 20% background source-sRGB mixture,
rounded per channel as specified in architecture §3.6.1. Normal caret ink,
coverage alpha, hover substitutions, ordinary roles, frames and selection rules
are unchanged. Theme/Color changes resolve the current placeholder through the
new palette immediately, without changing its identity or the raw editor.

**Ownership.** Untouched/default is explicit per-field Controller metadata,
distinct from an accepted value and from an active raw draft. A numeric default
may retain a valid app-owned zero quantity underneath the placeholder. Accepted
literal zero is never reclassified as initial. Successful nonempty Formula
acceptance and all values loaded from existing data are accepted, including
values that round to `0.0`. Reprojection, branch hiding/revealing, direction
changes and Formula opening/Cancel must not invent or erase that provenance.

**Entry.** Pointer activation of the editable value area focuses and starts
editing in the same event. An initial placeholder becomes an empty raw editor
with the normal `▏` caret at index zero; the placeholder is never inserted into
the draft. Repeated clicks retain an existing raw draft. Real values retain the
existing editing behavior and caret placement. Numeric keyboard Tab/BackTab
arrival at an editable value starts the same edit, as does existing Enter/Space
activation when merely focused. Do not activate the next Formula, direction,
button or selection target while moving focus. Modal text focus, including its
automatic opening focus and Tab/BackTab/Up/Down arrival, has the same empty-editor
behavior. Therefore an auto-focused empty Enter Name opens showing a caret;
its `none` placeholder is visible when its empty text area is inactive.
Passive focus restoration after Enter, Formula close or OS focus gain does not
reenter numeric editing; a new activation/navigation into the field does.

**Numeric exit.** Enter accepts only the field candidate, never the command.
Tab/BackTab, pointer blur, menu entry, field hiding and OS focus loss use the
existing field-resolution path. Leaving a placeholder-origin empty editor with
no successful text mutation emits no numeric edit; the initial placeholder
returns when editing ends. Navigation, rejected keystrokes, focus and clearing
alone are not text mutations. Direction-only changes still emit Direction and
retain magnitude, expression and placeholder identity. Escape discards only the
active raw edit and restores the last accepted field presentation, or the
placeholder if there was no accepted input. User-entered empty/`.` attempts
(including typing then deleting everything) retain the existing Incomplete
rejection path: no accepted quantity or expression is overwritten; the accepted
projection restores the field, including its provenance. Invalid characters
rejected by the compact grammar leave the draft as before. A real accepted
value that is deleted to empty is rejected/restored, never converted to a new
placeholder. Command Cancel and OK retain their existing transactional meaning.

**Validation Operation Box exit.** Moving focus away does not accept a name.
Retain nonempty raw text verbatim and show it normally; restore `none` only when
the Enter Name raw draft is exactly empty and inactive. Whitespace is a draft,
not a placeholder. Enter/OK with a trimmed-empty name remains rejected, leaves
the modal open and preserves the previous applied name. If the text area has
focus it remains empty with a caret; if OK has focus it remains inactive, with
`none` only for an exactly empty Enter Name. Rename never uses a placeholder to
mask deletion of an existing name. Cancel/Escape restores the applied value and
closes; a later empty Enter Name opening repeats this lifecycle, while reopening
Rename loads the full existing name. Accepted names, including `none`, are never
cleared on focus. Surrounding frame, controls, validation and focus-return rules
are unchanged.

**Formula isolation.** The existing formula target (`☰` and its existing owned
closing-bracket cell) opens Formula; it never begins a numeric raw editor.
Resolve any active numeric edit first and preserve M4.4's deferred opening from
the resulting accepted source, including after rejection. Opening and Cancel
leave accepted source/value/provenance intact. Successful nonempty Formula OK
marks its resulting value accepted, even zero; rejected OK leaves the modal and
source draft intact. Empty Formula OK keeps its existing explicit removal of an
expression and retained-literal behavior; preserve the prior placeholder
provenance (do not manufacture an accepted input merely from opening/empty OK).
Formula focus returns to its originating control without starting an empty
numeric edit. Preserve exact terminal acknowledgements, retries, Formula
application before modal close and command commit before command close.

Brackets, direction prefix, unit suffix, Formula control, field dimensions,
alignment and routing are unchanged. The only glyph removal is the placeholder
inside the existing editable span. Placeholder clearing itself is presentation:
no model mutation, dirty flag, command publication, binding removal or loss of
accepted precision/source is permitted.

---

# 6. Common Action Box header

Extrude, Revolve, Loft and Sweep share the same two-level header:

1. output type;
2. operation mode.

Output type:

```text
Solid
Surface
```

Operation mode:

```text
New
Add
Remove
```

The exact border topology is defined by
`Interface_Elements.txt`, `"Head Action Box variants"`.

The six combinations are represented explicitly.

The visual topology MUST be reproduced exactly rather than reconstructed using
different separators.

---

# 7. Complete Extrude Action Box reference

The following is the current reference from
`Action_Box_Extrude.txt`.

Spacing is significant.

```text
"Extrude Action Box"
╭─Extrude─────┬───────────────╮
│    Solid    │    Surface    │
├───────┬─────┘╴╴╴┐╴╴╴╴╴╴╴╴╴╴╴┤
│  New  │   Add   ┆  Remove   │
├───────┘╴╴╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴╴╴╴╴┤
│ Profile     [Select…       ]│
│ Extend   ◀     Blind     ▶  │
│ │ Depth   [+|0.0 mm       ☰]│
│ │ Along     [Select…       ]│
│ │ □ Starting Offset         │
│ ╰╴□ End Offset              │
│ □ Symetric                  │
│ □ Second end position       │
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
╭─Extrude─────┬───────────────╮
│    Solid    │    Surface    │
├───────┬─────┴───┐╴╴╴╴╴╴╴╴╴╴╴┤
│  New  │   Add   │  Remove   │
│╴╴╴╴╴╴╴┴─────────┘╴╴╴╴╴╴╴╴╴╴╴┤
│ Profile     [Select…       ]│
│ Extend   ◀     Blind     ▶  │
│ │ Depth   [+|0.0 mm       ☰]│
│ │ Along     [Select…       ]│
│ │ □ Starting Offset         │
│ ╰╴□ End Offset              │
│ □ Symetric                  │
│ □ Second end position       │
│ ■ Merge with all            │
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
╭─Extrude─────┬───────────────╮
│    Solid    │    Surface    │
├───────┬─────┘╴╴╴┬───────────┤
│  New  ┆   Add   │  Remove   │
├╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴╴╴┴───────────┤
│ Profile     [Select…       ]│
│ Extend   ◀     Blind     ▶  │
│ │ Depth   [+|0.0 mm       ☰]│
│ │ Along     [Select…       ]│
│ │ □ Starting Offset         │
│ ╰╴□ End Offset              │
│ □ Symetric                  │
│ □ Second end position       │
│ ■ Merge with all            │
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
Same for Surfaces

"Invert variants"
│ □ Invert                    │

│ ■ Invert                    │

"Extend variants"
│ Extend   ◀     Blind     ▶  │
│ │ Depth   [+|0.0 mm       ☰]│
│ │ Along     [Select…       ]│
│ ╰╴□ Starting Offset         │

│ Extend   ◀     Up to     ▶  │
│ │ Selection [Select…       ]│
│ ╰╴□ Offset                  │

│ Extend   ◀  Through all  ▶  │
│ │ □ Invert                  │
│ ╰╴□ Start Offset            │

"Start Offset variants"
│ │ □ Start Offset            │

│ │ ■ Start Offset ◀ Blind ▶  │
│ │ ╰╴Dist  [+|0.0 mm       ☰]│

│ │ ■ Start Offset ◀ Up to ▶  │
│ │ ╰╴Target  [Select…       ]│

"End Offset variants 1"
│ │ □ End Offset              │

│ │ ■ End Offset   ◀ Blind ▶  │
│ │ ╰╴Dist  [+|0.0 mm       ☰]│

│ │ ■ End Offset   ◀ Up to ▶  │
│ │ ╰╴Target  [Select…       ]│

"End Offset variants 2"
│   ╰╴□ End Offset            │

│   ╰╴■ End Offset ◀ Blind ▶  │
│     ╰╴Dist  [0.0 mm       ☰]│

│   ╰╴■ End Offset ◀ Up to ▶  │
│     ╰╴Target[Select…       ]│

"Symetric variants"
│ □ Symetric                  │
│ □ 2nd side                  │

│ ■ Symetric                  │

"2nd side variants"
│ ╰╴□ 2nd side                │

│ ╰╴■ 2nd side     ◀ Blind ▶  │
│   ╰╴Depth [+|0.0 mm       ☰]│

│ ╰╴■ 2nd side     ◀ Up to ▶  │
│   │ Target  [Select…       ]│
│   ╰╴□ End Offset            │

"Invert vartiants"
│ □ Invert                    │

│ ■ Invert                    │

"Merge with all variants"
│ ■ Merge with all            │

│ □ Merge with all            │
│ ╰╴Targets   [Select…       ]│
```

---

# 8. Extrude semantics

## 8.1 `Profile`

```text
│ Profile     [Select…       ]│
```

`Profile` identifies the section used by the extrusion.

Extrude accepts one planar bounded face for Solid and a planar open wire or closed boundary for Surface. A face chosen for Surface sweeps all its boundary wires without caps. A wire or edge profile is chosen with a supporting planar face; the candidate label identifies that face and its effective normal defines default sweep direction. A direction tangent to that supporting plane is invalid for this profile contract. Typed profile, Along and target references are chosen through the existing selection-list controls; their modal does not enable viewport picking. Up to accepts one planar bounded face or datum plane; the entire swept section must reach its target trim. Surface Add forms the regular union of compatible sheets, welding shared boundaries and removing overlapping area once. Surface Remove subtracts overlapping area on the same supporting surface; a transverse line intersection removes no area. A nonmanifold result or an operation changing no target is rejected. Solid targets are bodies; Surface targets are surfaces.

`[Select…       ]` is the selection field.

---

## 8.2 `Extend`

`Extend` defines how the main extrusion terminates.

### Blind

Exact snippet (a partial excerpt; the complete `Blind` branch also contains
`End Offset` as its last child, §2.6):

```text
│ Extend   ◀     Blind     ▶  │
│ │ Depth   [+|0.0 mm       ☰]│
│ │ Along     [Select…       ]│
│ ╰╴□ Starting Offset         │
```

`Depth` is the extrusion magnitude.

`+|` controls its orientation.

`0.0 mm` is the editable distance.

`☰` opens its formula definition.

`Along` selects a reference defining or overriding the extrusion direction.

`Starting Offset` enables an independently defined starting position.

### Up to

Exact variant:

```text
│ Extend   ◀     Up to     ▶  │
│ │ Selection [Select…       ]│
│ ╰╴□ Offset                  │
```

`Selection` identifies the target entity at which the extrusion terminates. It
is a child row of `Extend` (second `│` branch) and its field starts one cell
earlier than the `Profile` field because the label is longer; the reference
position MUST be kept.

`Offset` optionally shifts the termination relative to that target. It is the
last child of `Extend` (`╰╴`).

Enabling Up to Offset reveals one unsigned formula-capable length below Offset. Its magnitude is a clearance toward the start, measured along the extrusion direction. Zero terminates at the target.

```text
│ ╰╴■ Offset                  │
│   ╰╴Dist  [0.0 mm         ☰]│
```

### Through all

Exact variant:

```text
│ Extend   ◀  Through all  ▶  │
│ │ □ Invert                  │
│ ╰╴□ Start Offset            │
```

`Through all` terminates the extrusion beyond every entity along its direction;
it has no depth or target. Its children are `Invert` (see §8.9) and
`Start Offset`.

> **Resolved by §2.6.** In `Action_Box_Extrude.txt` the complete base boxes show
> `Starting Offset` and `End Offset` under `Blind`, while the `"Extend variants"`
> Blind snippet ends the branch at `Starting Offset`. The complete base box is the
> authority for `Blind` (all four children, `End Offset` last), the snippet is a
> partial excerpt, and sibling controls are never deleted by a substitution. The
> `Up to` and `Through all` branches are exactly their snippets; the remaining
> label and `End Offset` questions are the *Open reference points* of §2.6.

---

## 8.3 Start Offset

Collapsed:

```text
│ │ □ Start Offset            │
```

Blind:

```text
│ │ ■ Start Offset ◀ Blind ▶  │
│ │ ╰╴Dist  [+|0.0 mm       ☰]│
```

Up to:

```text
│ │ ■ Start Offset ◀ Up to ▶  │
│ │ ╰╴Target  [Select…       ]│
```

The control changes where the extrusion starts.

`Blind` defines that start through a distance.

`Up to` defines it through a selected target.

---

## 8.4 End Offset — main branch

Collapsed:

```text
│ │ □ End Offset              │
```

Blind:

```text
│ │ ■ End Offset   ◀ Blind ▶  │
│ │ ╰╴Dist  [+|0.0 mm       ☰]│
```

Up to:

```text
│ │ ■ End Offset   ◀ Up to ▶  │
│ │ ╰╴Target  [Select…       ]│
```

This is the End Offset variant used as a direct child of the main extent.

---

## 8.5 End Offset — nested second-side branch

Collapsed:

```text
│   ╰╴□ End Offset            │
```

Blind:

```text
│   ╰╴■ End Offset ◀ Blind ▶  │
│     ╰╴Dist  [0.0 mm       ☰]│
```

Up to:

```text
│   ╰╴■ End Offset ◀ Up to ▶  │
│     ╰╴Target[Select…       ]│
```

This is not another unrelated End Offset control.

It is the representation of End Offset when nested inside the second-side
branch.

The different indentation is therefore mandatory.

---

## 8.6 Symetric

Inactive:

```text
│ □ Symetric                  │
│ □ 2nd side                  │
```

Active:

```text
│ ■ Symetric                  │
```

When symmetry is enabled, an independently defined second side is no longer
needed and therefore disappears.

---

## 8.7 Second side

Collapsed:

```text
│ ╰╴□ 2nd side                │
```

Blind:

```text
│ ╰╴■ 2nd side     ◀ Blind ▶  │
│   ╰╴Depth [+|0.0 mm       ☰]│
```

Up to:

```text
│ ╰╴■ 2nd side     ◀ Up to ▶  │
│   │ Target  [Select…       ]│
│   ╰╴□ End Offset            │
```

`2nd side` creates an independent extent on the opposite side.

When its type is `Up to`, a nested End Offset is available.

---

## 8.8 Merge with all

Automatic:

```text
│ ■ Merge with all            │
```

Explicit target selection:

```text
│ □ Merge with all            │
│ ╰╴Targets   [Select…       ]│
```

When enabled, every applicable target is used.

When disabled, `Targets` appears so the user can explicitly choose which
existing entities are affected.

---

Blind depth is measured from the displaced start. Symetric uses the main extent as a per-side reach about the displaced start, so Blind depth D produces total depth 2D before end offsets; the independently defined second side is inactive. A Blind End Offset extends/retracts the nominal endpoint in that side's orientation. An Up to End Offset replaces that endpoint with the selected target station. Main Up to Offset and unsigned second-side End Offset retract toward the start. A Start Up to target must be parallel to the profile plane and translates the start section; a tilted start target is rejected. Tilted main/second termination targets remain permitted. Through all Invert reverses only the main reference direction. Along is used only while Blind is active.

---

## 8.9 Invert

The file defines:

```text
│ □ Invert                    │
```

and:

```text
│ ■ Invert                    │
```

This is a boolean direction state.

Its placement is defined by `Action_Box_Extrude.txt`: `Invert` is the first
child row of `Extend = Through all` and appears nowhere else in the Extrude
Action Box:

```text
│ Extend   ◀  Through all  ▶  │
│ │ □ Invert                  │
│ ╰╴□ Start Offset            │
```

An implementation MUST place `Invert` exactly there, with exactly that
indentation and branch glyphs, and MUST NOT add it to the `Blind` or `Up to`
forms. (The `"Invert variants"` and `"Invert vartiants"` headings in the `.txt`
both only list its two checkbox states.)

---

# 9. Complete Revolve Action Box reference

```text
"Revolve Action Box"
╭─Revolve─────┬───────────────╮
│    Solid    │    Surface    │
├───────┬─────┘╴╴╴┐╴╴╴╴╴╴╴╴╴╴╴┤
│  New  │   Add   ┆  Remove   │
├───────┘╴╴╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴╴╴╴╴┤
│ Profile     [Select…       ]│
│ Axis        [Select…       ]│
│ ■ Full Revolve              │
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
╭─Revolve─────┬───────────────╮
│    Solid    │    Surface    │
├───────┬─────┴───┐╴╴╴╴╴╴╴╴╴╴╴┤
│  New  │   Add   │  Remove   │
│╴╴╴╴╴╴╴┴─────────┘╴╴╴╴╴╴╴╴╴╴╴┤
│ Profile     [Select…       ]│
│ Axis        [Select…       ]│
│ ■ Full Revolve              │
│ ■ Merge with all            │
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
╭─Revolve─────┬───────────────╮
│    Solid    │    Surface    │
├───────┬─────┘╴╴╴┬───────────┤
│  New  ┆   Add   │  Remove   │
├╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴╴╴┴───────────┤
│ Profile     [Select…       ]│
│ Axis        [Select…       ]│
│ ■ Full Revolve              │
│ ■ Merge with all            │
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
Same for Surfaces

"Full Revolve variants"
│ ■ Full Revolve              │

│ □ Full Revolve   ◀ Blind ▶  │
│ │ Angle   [+|0.0 deg      ☰]│
│ │ □ Start Offset            │
│ │ □ Symetric                │
│ ╰╴□ 2nd side                │

│ □ Full Revolve   ◀ Up to ▶  │
│ │ Target    [Select…       ]│
│ │ □ Start Offset            │
│ │ □ End Offset              │
│ │ □ Symetric                │
│ ╰╴□ 2nd side                │

    
"Start Offset variants"
│ │ □ Start Offset            │

│ │ ■ Start Offset ◀ Blind ▶  │
│ │ ╰╴Angle [+|0.0 deg      ☰]│

│ │ ■ Start Offset ◀ Up to ▶  │
│ │ ╰╴Target  [Select…       ]│

"End Offset variants 1"
│ │ □ End Offset              │

│ │ ■ End Offset   ◀ Blind ▶  │
│ │ ╰╴Angle   [0.0 deg      ☰]│

│ │ ■ End Offset   ◀ Up to ▶  │
│ │ ╰╴Target  [Select…       ]│

"End Offset variants 2"
│   ╰╴□ End Offset            │

│   ╰╴■ End Offset ◀ Blind ▶  │
│     ╰╴Angle [0.0 deg      ☰]│

│   ╰╴■ End Offset ◀ Up to ▶  │
│     ╰╴Target[Select…       ]│

"2nd side variants"
│ ╰╴□ 2nd side                │

│ ╰╴■ 2nd side     ◀ Blind ▶  │
│   ╰╴Angle [+|0.0 deg      ☰]│

│ ╰╴■ 2nd side     ◀ Up to ▶  │
│   │ Target  [Select…       ]│
│   ╰╴□ End Offset            │

"Symetric variants"
│ │ □ Symetric                │
│ ╰╴□ 2nd side                │

│ ╰╴■ Symetric                │

"Merge with all variants"
│ ■ Merge with all            │

│ □ Merge with all            │
│ ╰╴Targets   [Select…       ]│
```

---

# 10. Revolve semantics

## 10.1 Profile

```text
│ Profile     [Select…       ]│
```

Defines the section revolved around the selected axis.

---

## 10.2 Axis

```text
│ Axis        [Select…       ]│
```

Defines the axis of revolution.

---

## 10.3 Full Revolve

Enabled:

```text
│ ■ Full Revolve              │
```

The operation performs a complete revolution.

No partial end-condition parameters are required.

Blind partial revolution:

```text
│ □ Full Revolve   ◀ Blind ▶  │
│ │ Angle   [+|0.0 deg      ☰]│
│ │ □ Start Offset            │
│ │ □ Symetric                │
│ ╰╴□ 2nd side                │
```

The user defines a signed angular extent.

Up-to partial revolution:

```text
│ □ Full Revolve   ◀ Up to ▶  │
│ │ Target    [Select…       ]│
│ │ □ Start Offset            │
│ │ □ End Offset              │
│ │ □ Symetric                │
│ ╰╴□ 2nd side                │
```

The revolution terminates on a selected target.

---

## 10.4 Revolve offsets

`Start Offset`, `End Offset`, `Symetric`, `2nd side` and
`Merge with all` follow the same parent/child principles as Extrude.

Distances are replaced by angular values where the parameter represents
revolution.

The exact `[+|0.0 deg      ☰]`, `[0.0 deg      ☰]`,
indentation and spacing shown in the reference MUST be retained.

---

# 11. Complete Loft Action Box reference

```text
"Loft Action Box"
╭─Loft────────┬───────────────╮
│    Solid    │    Surface    │
├───────┬─────┘╴╴╴┐╴╴╴╴╴╴╴╴╴╴╴┤
│  New  │   Add   ┆  Remove   │
├───────┘╴╴╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴╴╴╴╴┤
│ Profile     [Select…       ]│
│ □ Start type                │
│ □ End type                  │
│ □ Guide                     │
│ □ Path                      │
│ □ Connection                │
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
╭─Loft────────┬───────────────╮
│    Solid    │    Surface    │
├───────┬─────┴───┐╴╴╴╴╴╴╴╴╴╴╴┤
│  New  │   Add   │  Remove   │
│╴╴╴╴╴╴╴┴─────────┘╴╴╴╴╴╴╴╴╴╴╴┤
│ Profile     [Select…       ]│
│ □ Start type                │
│ □ End type                  │
│ □ Guide                     │
│ □ Path                      │
│ □ Connection                │
│ ■ Merge with all            │
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
╭─Loft────────┬───────────────╮
│    Solid    │    Surface    │
├───────┬─────┘╴╴╴┬───────────┤
│  New  ┆   Add   │  Remove   │
├╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴╴╴┴───────────┤
│ Profile     [Select…       ]│
│ □ Start type                │
│ □ End type                  │
│ □ Guide                     │
│ □ Path                      │
│ □ Connection                │
│ ■ Merge with all            │
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
Same for Surfaces

"Start Type variants"
│ □ Start type                │

│ ■ Start type   ◀ Normal  ▶  │
│ ╰╴Magnitude [0.0          ☰]│

│ ■ Start type   ◀ Tengent ▶  │
│ ╰╴Magnitude [0.0          ☰]│

│ ■ Start type   ◀  Match  ▶  │
│ ╰╴Entity    [Select…       ]│

"End Type variants"
│ □ End type                  │

│ ■ End type     ◀ Normal  ▶  │
│ ╰╴Magnitude [0.0          ☰]│

│ ■ End type     ◀ Tengent ▶  │
│ ╰╴Magnitude [0.0          ☰]│

│ ■ End type     ◀  Match  ▶  │
│ ╰╴Entity    [Select…       ]│

"Guide variants"
│ □ Guide                     │

│ ■ Guide     [Select…       ]│

"Path variants"
│ □ Path                      │

│ ■ Path      [Select…       ]│

"Connection variants"
│ □ Connection                │

│ ■ Connection                │
│ │ Start     [Select…       ]│
│ ╰╴End       [Select…       ]│

"Merge with all variants"
│ ■ Merge with all            │

│ □ Merge with all            │
│ ╰╴Targets   [Select…       ]│
```

---

# 12. Loft semantics

## 12.1 Profile

```text
│ Profile     [Select…       ]│
```

Loft uses an ordered set of profiles.

The detailed editor for this ordered set is the Profile Selection box described
later.

Profile ordering is therefore semantically meaningful.

---

## 12.2 Start type

Disabled:

```text
│ □ Start type                │
```

Normal:

```text
│ ■ Start type   ◀ Normal  ▶  │
│ ╰╴Magnitude [0.0          ☰]│
```

Tangent:

```text
│ ■ Start type   ◀ Tengent ▶  │
│ ╰╴Magnitude [0.0          ☰]│
```

Match:

```text
│ ■ Start type   ◀  Match  ▶  │
│ ╰╴Entity    [Select…       ]│
```

`Normal` and `Tengent` use a formula-capable scalar `Magnitude`.

`Match` instead reveals an entity-selection field.

The `.txt` currently spells the state `Tengent`; this README does not alter the
ASCII reference.

---

## 12.3 End type

`End type` is independent from `Start type` and uses the same state model:

- disabled;
- Normal + Magnitude;
- Tengent + Magnitude;
- Match + Entity.

---

## 12.4 Guide

Disabled:

```text
│ □ Guide                     │
```

Enabled:

```text
│ ■ Guide     [Select…       ]│
```

Enabling the option reveals the guide selection in the same row.

---

## 12.5 Path

Disabled:

```text
│ □ Path                      │
```

Enabled:

```text
│ ■ Path      [Select…       ]│
```

The selected entity supplies the loft path/centerline constraint.

---

## 12.6 Connection

Disabled:

```text
│ □ Connection                │
```

Enabled:

```text
│ ■ Connection                │
│ │ Start     [Select…       ]│
│ ╰╴End       [Select…       ]│
```

`Start` and `End` are children of the Connection option.

The branch glyphs explicitly encode that relationship.

---

# 13. Complete Sweep Action Box reference

```text
"Sweep Action Box"
╭─Sweep───────┬───────────────╮
│    Solid    │    Surface    │
├───────┬─────┘╴╴╴┐╴╴╴╴╴╴╴╴╴╴╴┤
│  New  │   Add   ┆  Remove   │
├───────┘╴╴╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴╴╴╴╴┤
│ Profile     [Select…       ]│
│ Path        [Select…       ]│
│ □ Full path                 │
│ □ Keep orientation          │
│ □ Scale                     │
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
╭─Sweep───────┬───────────────╮
│    Solid    │    Surface    │
├───────┬─────┴───┐╴╴╴╴╴╴╴╴╴╴╴┤
│  New  │   Add   │  Remove   │
│╴╴╴╴╴╴╴┴─────────┘╴╴╴╴╴╴╴╴╴╴╴┤
│ Profile     [Select…       ]│
│ Path        [Select…       ]│
│ □ Full path                 │
│ □ Keep orientation          │
│ □ Scale                     │
│ ■ Merge with all            │
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
╭─Sweep───────┬───────────────╮
│    Solid    │    Surface    │
├───────┬─────┘╴╴╴┬───────────┤
│  New  ┆   Add   │  Remove   │
├╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴╴╴┴───────────┤
│ Profile     [Select…       ]│
│ Path        [Select…       ]│
│ □ Full path                 │
│ □ Keep orientation          │
│ □ Scale                     │
│ ■ Merge with all            │
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
Same for Surfaces

"Full path variants"
│ □ Full path                 │

│ ■ Full path                 │

"Keep orientation variants"
│ □ Keep orientation          │

│ ■ Keep orientation          │
│ ╰╴□ Twist                   │


"Twist variants"
│ ╰╴□ Twist                   │

│ ╰╴■ Twist [+|0.0 deg      ☰]│

"Scale variants"
│ □ Scale                     │

│ ■ Scale     [0.0          ☰]│

"Merge with all variants"
│ ■ Merge with all            │

│ □ Merge with all            │
│ ╰╴Targets   [Select…       ]│
```

---

# 14. Sweep semantics

## 14.1 Profile

```text
│ Profile     [Select…       ]│
```

Defines the section being swept.

---

## 14.2 Path

```text
│ Path        [Select…       ]│
```

Defines the sweep trajectory.

---

## 14.3 Full path

```text
│ □ Full path                 │
```

or:

```text
│ ■ Full path                 │
```

Controls whether the operation uses the complete available path.

The current reference does not define child rows for this state.

---

## 14.4 Keep orientation

Disabled:

```text
│ □ Keep orientation          │
```

Enabled:

```text
│ ■ Keep orientation          │
│ ╰╴□ Twist                   │
```

Enabling `Keep orientation` reveals the optional `Twist` child.

---

## 14.5 Twist

Disabled:

```text
│ ╰╴□ Twist                   │
```

Enabled:

```text
│ ╰╴■ Twist [+|0.0 deg      ☰]│
```

The twist parameter is embedded on the same row as the checkbox.

`+|` defines orientation.

`0.0 deg` defines the magnitude.

`☰` opens the Formula editor.

---

## 14.6 Scale

Disabled:

```text
│ □ Scale                     │
```

Enabled:

```text
│ ■ Scale     [0.0          ☰]│
```

The scale value is formula-capable.

No unit is displayed, indicating a scalar/dimensionless parameter.

---

# 15. Complete Boolean Action Box reference

```text
"Boolean Action Box"
╭─Boolean─┬───────┬───────────╮
│   Add   │  Cut  ┆   Inter   │
├─────────┘╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴╴╴╴╴┤
│ Target      [Select…       ]│
│ Tools       [Select…       ]│
│ □ Keep tools                │
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
╭─Boolean─┬───────┬───────────╮
│   Add   │  Cut  │   Inter   │
│╴╴╴╴╴╴╴╴╴┴───────┘╴╴╴╴╴╴╴╴╴╴╴┤
│ Target      [Select…       ]│
│ Tools       [Select…       ]│
│ □ Offset                    │
│ □ Keep tools                │
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
╭─Boolean─┬───────┬───────────╮
│   Add   ┆  Cut  │   Inter   │
│╴╴╴╴╴╴╴╴╴┘╴╴╴╴╴╴╴┴───────────┤
│ Target      [Select…       ]│
│ Tools       [Select…       ]│
│ □ Keep tools                │
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯

"Offset variants"
│ □ Offset                    │

│ ■ Offset                    │
│ ╰╴■ Offset all              │
│   ╰╴Dist  [+|0.0 mm       ☰]│

"Offset All variants"
│ ╰╴■ Offset all              │
│   ╰╴Dist  [+|0.0 mm       ☰]│

│ ╰╴□ Offset all              │
│   ╰╴Faces   [none          ]│
│     Dist  [+|0.0 mm       ☰]│

"Keep tools variants"
│ □ Keep tools                │
│ ■ Keep tools                │
```

---

# 16. Boolean semantics

Boolean differs from the Create Action Boxes.

Its header directly selects the boolean operation:

```text
Add
Cut
Inter
```

---

## 16.1 Add

Represents boolean union.

---

## 16.2 Cut

Represents boolean subtraction.

Cut exposes the additional Offset option.

---

## 16.3 Inter

Represents boolean intersection.

---

## 16.4 Target

```text
│ Target      [Select…       ]│
```

Defines the base entity on which the boolean operation is performed.

---

## 16.5 Tools

```text
│ Tools       [Select…       ]│
```

Defines the operand or operands applied to the Target.

---

## 16.6 Keep tools

Disabled:

```text
│ □ Keep tools                │
```

Enabled:

```text
│ ■ Keep tools                │
```

When active, tool operands are preserved after the boolean operation.

---

## 16.7 Offset

Disabled:

```text
│ □ Offset                    │
```

Enabled:

```text
│ ■ Offset                    │
│ ╰╴■ Offset all              │
│   ╰╴Dist  [+|0.0 mm       ☰]│
```

Offset is a Cut-specific optional operation in the current reference.

Its child is `Offset all`.

---

## 16.8 Offset all

Enabled:

```text
│ ╰╴■ Offset all              │
│   ╰╴Dist  [+|0.0 mm       ☰]│
```

The distance applies to all applicable faces.

Disabled:

```text
│ ╰╴□ Offset all              │
│   ╰╴Faces   [none          ]│
│     Dist  [+|0.0 mm       ☰]│
```

Disabling `Offset all` reveals an explicit Faces selection.

The following `Dist` line belongs to the same Offset-all branch.

The different indentation between:

```text
│   ╰╴Faces
```

and:

```text
│     Dist
```

MUST be preserved exactly as defined by the reference.

---

# 17. Boolean footer (anomaly resolved)

`Action_Box_Boolean.txt` previously contained a shorter, 27-cell Add-state footer
row. It is corrected: all three Boolean states now use the canonical 31-cell
footer row shared with every other Action Box:

```text
│      OK      │    Cancel    │
```

The enclosing Boolean Action Box is exactly 31 cells wide in every row of every
state (verified row by row for all `.txt` Action Box references). There is no
remaining Boolean footer exception, and the former workaround of reproducing the
short row "as is" (M2.22) no longer applies.

---

# 18. Shared selection controls

Current compact states from `Interface_Elements.txt`:

```text
[Select…       ] [1 Element    ☰] [N Elements   ☰]
```

Meaning:

- `[Select…       ]`: no selected entity;
- `[1 Element    ☰]`: exactly one selected entity;
- `[N Elements   ☰]`: several selected entities.

The dimensions of the fields remain fixed.

---

# 19. Generic Selection box

**Reference alignment, 2026-10-27.** Empty Selection has exactly one blank
content row. The reference is:

```text
╭─Selection───────────────────╮
│┌───────────────────────────┐│
││                           ││
│└───────────────────────────┘│
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
```

Merge-scope membership uses eligible candidates, including unchecked candidates
when the selected set is empty. The reference is:

```text
╭─Selection───────────────────╮
│┌───────────────────────────┐│
││ □ Rim_Body                ││
││ ■ Hub_Body                ││
││ □ Inner_Surface           ││
│└───────────────────────────┘│
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
```

`□` means excluded from the draft; `■` means included. These are membership
controls, distinct from persistent tree visibility. Candidate filtering follows
the originating semantic field, including the current geometry kind; artwork
names are illustrative, never runtime fallback rows.

## 19.1 Exact shared Selection geometry

Coordinates below are zero-based relative to the logical outer frame; ranges
are half-open. Let `N` be the number of rows in the active presentation and
`V = max(1,min(N,5))`. Width is 31 cells and height is `V+6` cells.

| Part | Exact cells |
|---|---|
| Outer top | y=0; `╭` x=0, `─` x=1, title from x=2, `─` through x=29, `╮` x=30 |
| Outer sides | x=0 and x=30, y in [1,V+5) |
| Inner top | y=1; `┌` x=1, 27 `─` x in [2,29), `┐` x=29 |
| Content | x in [2,29), y in [2,V+2); empty is 27 spaces on y=2 |
| Inner bottom | y=V+2; `└` x=1, 27 `─`, `┘` x=29 |
| Separator | y=V+3; outer `│`, 29 `─` x in [1,30), outer `│` |
| Footer | y=V+4; exact `│      OK      │    Cancel    │` |
| Outer bottom | y=V+5; `╰` x=0, 29 `─`, `╯` x=30 |

Title letters use Text, all frame/separator/divider strokes use Contour. Footer
labels and padding retain their existing Text role and button ownership.
Checkboxes, reorder/remove glyphs and scrollbar arrows/thumb use Action; labels
and ordinary padding use Text. Hover is only the existing footer boundary
substitution (§5.12); list controls acquire no new hover visual or state colour.

The surface owns inert chrome and blank content. Each candidate row owns its
checkbox/name/padding span; no blank empty-list row is an item. For membership
rows, x=2 is padding, x=3 is `□`/`■`, x=4 is separation, and the label starts at
x=5. With a bar, x=28 is the gutter and label capacity is 23; otherwise label
capacity is 24. Clip names in whole cells without changing the stored name.
OK owns footer x in [1,15), Cancel x in [16,30); x=15 is a vertical SharedBorder
with OK on its low half and Cancel on its high half. Outer footer boundaries
remain surface-owned and are substituted only by the established hover writer.
Column 14 is OK padding, not a shared cell. OK hover substitutes the boundaries
at x=0 and x=15; Cancel hover substitutes x=15 and x=30. Glyph substitutions
retain the same ownership, shared-border routing and Contour ink.

**Proposed placement policy for approval with the alignment plan:** keep the
existing distinction: candidate/membership modals are centered in shell overlay
bounds; ordered-item management replaces the originating Action Box at its
usual top/right anchor, using this compact height. Formula stays centered.
Never derive placement from the incorrect screenshot. For a centered modal,
use the existing `center_modal` position/available allocation; draw the logical
art at that origin and crop, without moving internal coordinates to the cropped
right/bottom edge. Action Box placement uses the existing action-panel layout.
Intersect every write/target with that allocation, grid and actual surface.
A clipped frame is not a new smaller artwork. Recompose the whole shell after
size/count changes so old glyphs, roles, WidgetIds and shared borders disappear.

## 19.2 Presentation and candidate admission

Choose by field purpose, never label text. MergeScope always opens candidate
membership. Other current single-reference and unordered membership inputs keep
their existing candidate chooser, cardinality and filters. In that chooser
checkboxes describe candidate admission; they do not redefine every profile as
a merge scope or an ordered selected-item row.

For an input with selected-item management (Loft Profile, Sweep Profile and
Sweep Path), the field body opens its candidate chooser; its populated `☰`
opens the ordered-item manager (§20). Empty fields still open the chooser.
Both existing field parts retain keyboard activation; no shortcut or glyph is
added. This explicit distinction preserves a way to add/replace eligible items
without mixing unchecked candidates into an ordered selected list.

These presentation rules do not raise existing cardinality limits or implement
new topology/commands. In particular, current Sweep Profile/Path and Loft
Profile limits remain as supplied by their descriptors; a one-item manager has
a remove control and an inert reorder handle. Truly multi-item ordered requests
use their supplied order. Unsupported inputs show zero actual candidates,
never illustrative profiles. The field's eligible candidate universe and its
selected ordered vector are separate data even when one is empty.

Edits remain a window-local modal draft. Only OK emits the ordered identity
vector for the exact document/modal/command-field lifetime. Cancel/Escape
produce no destination replacement. Selection OK never commits a CAD feature.

On close, restore the originating control when it still exists in the same
live command-field lifetime. If accepted removal of the last selected item
removes its populated `☰`, restore focus to that field's visible body instead.
Apply this repair when the accepted zero count is published, including delayed
publication; never leave focus on an absent management control. Enter/Space
then opens candidate selection through the body. Cancel retains the applied
membership and restores the surviving original control. If the originating
field or command lifetime no longer exists, clear its stale focus rather than
transferring it to another field or session.

An empty field cannot issue or accept a new ManageSelected request. The shell
rejects activation of an absent `☰`, and the application rechecks nonempty
accepted membership before opening management. Reject a stale request without
allocating/opening a modal or changing the destination. This does not prevent
an already open manager from becoming empty through draft removal and then
confirming that empty vector.

---

# 20. Profile Selection box

Exact ordered-item reference (spaces are significant):

```text
╭─Profils─────────────────────╮
│┌───────────────────────────┐│
││ ☰ Spline_1             × ▲││
││ ☰ Profil_3             × ╻││
││ ☰ Profil_4             × ╹││
││ ☰ Profil_4             ×  ││
││ ☰ Compound_line_1      × ▼││
│└───────────────────────────┘│
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
```

Rows represent already selected identities in draft order, never all eligible
candidates. Equal names in this reference belong to distinct identities. The
track `"╻╹ "` is the N=9, offset=1 state of §21; the five names are illustrative
visible rows in that state. A list of exactly five items has no scrollbar.
The same reference appears in the new Selection section of Interface_Elements.

Use §19.1's 31-cell frame and variable compact height. Title is `Profils` for
profile management; non-profile ordered management uses `Selection`. In every
ordered row x=2 is padding, x=3 is `☰`, x=4 is separation, label begins at x=5.
With a gutter: label x in [5,26) (21 cells), `×` x=26, separation x=27,
gutter x=28. Without a gutter reclaim one cell: label x in [5,27) (22 cells),
`×` x=27, separation x=28. These positions preserve
the artwork's exact control columns. Names are clipped before controls.

`☰` and `×` have separate stable targets tied to the selected identity, not a
visible row or label. Names/padding may focus the row but never toggle membership.
A primary drag beginning on a handle moves that identity within this draft;
release over a visible item inserts it before that item when moving upward and
after it when moving downward. Use whole-cell pointer rows; movement into a
different content row begins the reorder gesture, with no change on the initial
press. Same-row release is a no-op; release outside content or cancellation
discards the provisional move. No auto-scroll, page jump,
new shortcut or preview marker is introduced. Scroll first to reach a different
five-row window. The final reorder changes the draft once on valid release.
Remove deletes only that identity immediately from the draft, retaining survivor
order; its eventual release cannot activate another row. Focus on a removed
control falls back to OK. Cancel discards both operations.

---

# 21. Selection-box content scrolling

## 21.1 Scope, rows and gutter

This rule applies to the content lists of both Generic Selection (§19) and
Profile Selection (§20) boxes. Treeviews, application panels and unrelated
scrollbars retain their existing behavior.

- For `N = 1…5` content items, display all `N` rows, offset zero and no scrollbar.
- For `N > 5`, the logical content window is exactly five rows. Display items
  `[s, s + 5)`. Reserve the rightmost interior column as a one-cell-wide gutter:
  `▲`, three track cells, `▼`, alongside those same five content rows. The bar
  adds no content row and has no separator column of its own.
- An empty list uses exactly one blank content row and the compact surrounding
  chrome of §19.1, with no item, placeholder text or scrollbar. Offset is zero.
- Use the compact outer footprint, title, list borders, footer and row padding
  of §§19–20. Any clipped allocation does not add logical content rows. Without
  a gutter, content reclaims that column; with a gutter,
  labels are clipped to their reduced width before controls or borders. Profile
  `×` and its existing trailing separation stay inside the content region.
- Compute logical geometry before clipping to the available list rect and grid.
  A constrained rect crops whole cells; it does not compress/reposition the
  five-row bar or change the logical window or offset range. Off-grid cells
  have no hit target. A zero-sized region draws nothing. With insufficient
  width the gutter remains at its logical right edge and can be wholly clipped;
  never overlay it on a content cell. Do not change global overlay placement or
  surface-cropping rules to handle this local case.

## 21.2 Exact thumb model

The three track cells have six vertical half-cell slots, numbered `0…5`, upper
half first. For `N > 5`, compute:

```text
M = N − 5
s = clamp(requested content offset, 0, M)
L = max(1, 11 − N)
T = 6 − L
p = round_half_up(T × s / M)
occupied slots = [p, p + L)
```

For each track cell, both halves occupied gives `┃`, upper half only gives
`╹`, lower half only gives `╻`, and neither gives a space. Keep the existing
stroke thickness and these glyphs. Do not replace them with full-cell
proportional thumbs or new symbols.

Each quoted string below contains exactly the THREE TRACK CELLS, top to
bottom. Spaces inside quotes are significant. `▲` and `▼` stay above and below
all states. Successive strings correspond to successive integer content offsets
starting at zero:

```text
N=6:  ["┃┃╹", "╻┃┃"]
N=7:  ["┃┃ ", "╻┃╹", " ┃┃"]
N=8:  ["┃╹ ", "╻┃ ", " ┃╹", " ╻┃"]
N=9:  ["┃  ", "╻╹ ", " ┃ ", " ╻╹", "  ┃"]
N=10: ["╹  ", "╻  ", " ╹ ", " ╻ ", "  ╹", "  ╻"]
```

Verification by substitution: for `N=6…10`, `(L,T,M)` is respectively
`(5,1,1)`, `(4,2,2)`, `(3,3,3)`, `(2,4,4)`, `(1,5,5)`. Thus `p=s`; filling
`[s,s+L)` gives every string above and one half-cell of movement per content row.
For `N > 10`, `L=1`, `T=5`, and `p=round_half_up(5s/(N−5))`.
Multiple content offsets may share a position. Quantization affects only the
thumb: wheel, arrows and keyboard retain exact integer item offsets, including
steps that leave the thumb glyph unchanged. Position is monotone, with exact
endpoints `p(0)=0` and `p(M)=T`.

**Deterministic arithmetic.** All ratios are nonnegative. For numerator `a` and
positive denominator `b`, let `q=a div b`, `r=a mod b`; return
`q + (r >= b div 2 + b mod 2 ? 1 : 0)`. Ties round upward; do not use floating
point, ties-to-even, or `(2a+b)/(2b)`. Current counts/offsets are `u32`; widen to
`u64` before multiplication, including inverse drag products. Compute `L` with
saturating subtraction (`11.saturating_sub(N).max(1)`), not unsigned `11-N`.
Branch on `N<=5` before dividing. Clamp invalid/stale offsets before arithmetic;
negative input deltas saturate toward zero, positive deltas toward `M`.

## 21.3 State, input and identity

The per-window Controller owns content offset and selection. Composition derives
thumb geometry without mutating selection, item order, checked/draft flags or
item WidgetIds. IDs belong to original items, never visible row numbers or
labels (duplicate names remain distinct). Opening still starts at offset zero;
OK, Cancel, Escape, focus return and overlay input precedence are unchanged.

After a length change, retain the numeric offset if valid, otherwise clamp it
to `max(0,N−5)`; growth does not jump to the bottom. At `N<=5`, store zero and
remove the entire gutter and all arrow/thumb routing metadata. Clear obsolete
pressed/capture/cursor state; unavailable focused arrows follow the existing
fallback to OK. Retain surviving item identity and selection; removing an item
removes only that item's state. Runtime lists use the application-supplied rows and draft identities of §§19–20;
reference fixtures are explicit test-only data.

Retain existing arrow activation (one content row), wheel/trackpad direction and
step translation, keyboard focus ring and minimal ensure-visible scrolling.
Disabled endpoint arrows remain drawn but cannot activate. Keep the established
whole-cell thumb hit target on every cell containing an occupied half; blank
track cells are inert. Track clicks, page jumps, auto-repeat and new keyboard commands are not
introduced. Ordered-item reorder/removal is defined separately in §20.

Existing thumb dragging keeps its whole-row pointer sampling and capture
lifecycle. On press retain pointer row `r0`, exact content offset `s0`, thumb
position `p0`, item count and list geometry. For subsequent pointer row `r`,
compute signed `d=r−r0` before clamping, then `t=clamp(p0+2d,0,T)`. With no row
movement, retain `s0` (no press-time snap of a quantized offset). Otherwise set
`s=round_half_up(M×t/T)` and draw from the forward model. Clamp outside-track
motion to the endpoints. This preserves the grabbed row relationship and avoids
unsigned-subtraction errors above the track. Dragging may traverse several
items per pointer row as before; individual content rows remain reachable by
arrows, wheel and keyboard. Never quantize those inputs through thumb position.

A drag's own successful list scroll must retain capture across recomposition.
Release, pointer exit, focus loss, resize/scale cancellation, overlay dismissal
or external geometry/count change invalidates the grab; a disappearing bar
cannot retain capture or a Grab/Grabbing cursor. Preserve all other capture
routing and consume releases as before. No gesture starts on a stale target.

Recomposition fully replaces glyph, ink, style, WidgetId and shared-border
metadata throughout the old/new list allocation (including cleared surplus
rows/gutter). Arrow and occupied thumb glyphs use `Action`, labels use `Text`,
frames use `Contour`; blank track cells have no visible ink or thumb target.
Follow §5.12 and `ARCHITECTURE.md` §§3.5–3.6: scroll indicators gain no hover
visual, colors, stroke thickening or animation. Existing Grab/Grabbing OS cursor
feedback (§25.1) remains.

---

# 22. Formula editor

Exact reference:

```text
╭─Formula─────────────────────────╮
│┌───────────────────────────────┐│
││Fx = (#EntraxeA - 2mm ) / 5 * 3││
││* sin(#Angle1 - #Angle2 + 3deg)││
│└───────────────────────────────┘│
│─────────────────────────────────│
│       OK       │     Cancel     │
╰─────────────────────────────────╯
```

The Formula editor is opened by `☰` from formula-capable fields.

## 22.1 Fixed prefix and text viewport

The empty reference is:

```text
╭─Formula─────────────────────────╮
│┌───────────────────────────────┐│
││Fx =                           ││
││                               ││
│└───────────────────────────────┘│
│─────────────────────────────────│
│       OK       │     Cancel     │
╰─────────────────────────────────╯
```

The exact five-character `Fx = ` (including its trailing space) is fixed Text
presentation on the first inner row. It remains visible when empty, scrolled,
unfocused or rejected. It is not part of the expression, edit selection,
clipboard/source payload, source-origin map, parser, evaluator or validation.
Do not strip a user-entered matching string: only the separately composed prefix
is excluded. Opening/reopening retains expression characters and whitespace
verbatim; visual wrapping never inserts stored newlines.

The logical outer frame remains 35×8. Title is `Formula` at x in [2,9), y=0;
outer corners x=0,34 at y=0,7; inner frame x=1,33 with top y=1 and bottom y=4.
Its two 31-cell text rows are x in [2,33), y=2,3. Prefix occupies x in [2,7),
y=2. Expression slots are x in [7,33), y=2 (26 cells), then x in [2,33), y=3
(31 cells): total capacity 57. Separator is y=5, x in [1,34); footer y=6 has
OK x in [1,17), SharedBorder x=17, Cancel x in [18,34). Frame/title/text/footer
ownership follows Selection's corresponding structural rules; the inner frame,
prefix and expression slots share the existing text-area target. Clicking the
prefix only focuses text; it never places the caret in the prefix.

For expression characters E of length T, caret index c in [0,T], compose
D = E[0:c] + `▏` + E[c:T] while editing, otherwise D=E. Capacity C=57 and
start s=max(0,c+1-C) while editing, otherwise s=0. For each i in [0,57), draw
D[s+i] (or a blank): if i<26, position (7+i,2); else (2+i-26,3).
Prefix stays at (2,2) independently of s. Thus inactive capacity is 57 source
characters and editing capacity is 56 source characters plus one caret. Scrolling
is the existing linear whole-cell caret window over these asymmetric rows;
there is no extra scrollbar or indicator. Apply allocation/grid/surface clipping
after mapping logical cells; clipped cells never move the caret onto a frame.
Very small surfaces may physically crop the caret; logical capacity is unchanged.

Home/End and their existing selection variants address expression start/end,
not visual lines. SelectAll selects E only. Backspace at c=0 and Delete at c=T
are harmless; selection replacement/deletion never touches the prefix. Preserve
Left/Right, Tab/BackTab, Up/Down focus ring, Enter/OK confirmation and
Escape/Cancel with their existing meanings; Enter is not a newline. No new
shortcuts or clipboard integration are introduced. Focus restoration returns to
the originating field control after accepted OK or Cancel; rejected OK stays
open with exact draft, caret, selection and diagnostic.

The existing typed-input limit remains 256 expression characters (prefix/caret
consume none). Opening must not silently truncate supplied accepted source.
If longer source is supplied, preserve it; permit navigation, deletion and edits
that do not grow it, while rejecting growth until it is within the existing cap.
This is source preservation, not expression-language expansion.

Preserve M4.4 deferred opening after same-event numeric acceptance/rejection,
accepted-source initialization, one-shot delivery/retry, rejected-source recovery,
Formula application before close, and command commit before command close.
Empty OK submits an empty expression and retains the accepted literal; Cancel
submits no formula change. The prefix never affects any of those transactions.

The UI reference permits:

- parameter references;
- arithmetic;
- units;
- functions.

The UI specification does not itself define the expression evaluator.

---

Formula accepts `#Name := expression` to name or explicitly redefine a parameter, or an expression alone. Names use ASCII letters/underscore followed by letters/digits/underscore and are case-sensitive. Names and stable parameter identities are distinct. Changing only the header name of an existing field binding renames that identity; a colliding name is rejected. Formula OK accepts only the owning command draft. Command OK publishes its named definitions and reachable active bindings atomically; Cancel discards them. In existing non-Extrude panels, OK may publish explicitly named definitions but creates no CAD feature. Newly created standalone definitions belong to the document; updating an existing definition preserves its original owner and dependency restrictions.

---

# 23. Enter Name and Rename

Exact references:

```text
╭─Enter Name──────────────╮
│[none                   ]│
│─────────────────────────│
│     OK     │   Cancel   │
╰─────────────────────────╯
```

and:

```text
╭─Rename──────────────────╮
│[Rim_Body               ]│
│─────────────────────────│
│     OK     │   Cancel   │
╰─────────────────────────╯
```

These are the **Validation Operation Box** references in `Interface_Elements.txt`.
Both use the same modal interaction and the placeholder contract in §5.13.

The first starts from an empty applied value; `none` is presentation only, never
the initial name. Automatic text focus shows the empty editor and caret. The
reference above illustrates its inactive placeholder state.

The second starts with the existing name (the full stored name, never the
`…`-truncated text of a tree row, §26).

**Rename trigger (M2.20).** `Rename` opens on a **double-click** of the primary
(left) mouse button on the name text of a user-named tree row (a Feature row or
an Element body row, §26). The category headers (`Features`, `Elements`,
`Bodies(N)`, `Surfaces(N)`, `Construct`, and the historical reference fixture's
`Solid Bodies(N)`), the playhead separator row, the
`[-]`/`[+]` marker cells and the `□`/`■` cells are not rename targets; a click
on a marker or checkbox keeps its own action and never starts a rename.
`winit` 0.30 reports no click count, so the Controller recognises a double-click
itself: two left presses on the same tree-row name `widget_id` within 500 ms,
with no press on any other target in between. The first press keeps its normal
selection meaning; the second opens the modal. No keyboard shortcut opens
`Rename` in M2.20; one may be added later only after it is documented here.
Opening, editing (caret, §5.11), OK/Cancel and focus restoration follow the
common modal interaction above; Cancel keeps the old name.

---

Utilities > Properties opens the Extrude operation named by the currently focused operation row in Features. This target is independent of selected output entities, so an operation with zero or consumed outputs remains editable. With no focused Features operation, or a focused non-Extrude operation, Properties opens no editor and reports an ordinary contextual status. Its stable feature and parameter identities are retained. OK replaces that feature and recomputes affected following history atomically; Cancel leaves it unchanged. Double-click remains Rename.

---

# 24. Progress bar

Exact states defined by `Interface_Elements.txt`:

```text
Progress [     ] 0%
Progress [░    ] 5%
Progress [▒    ] 10%
Progress [▓    ] 15%
Progress [█    ] 20%
Progress [█░   ] 25%
Progress [█▒   ] 30%
Progress [█▓   ] 35%
Progress [██   ] 40%
Progress [██░  ] 45%
Progress [██▒  ] 50%
Progress [██▓  ] 55%
Progress [███  ] 60%
Progress [███░ ] 65%
Progress [███▒ ] 70%
Progress [███▓ ] 75%
Progress [████ ] 80%
Progress [████░] 85%
Progress [████▒] 90%
Progress [████▓] 95%
Progress [█████] 100%
```

The five-cell bar represents 100%.

Each cell therefore corresponds to 20%.

Partial fill characters encode 5% increments.

**Percentage field (M2.24).** The bar and the `N%` value are composed from the
same displayed value. The segment is exactly **21 cells**: `Progress [` (10) +
the 5-cell bar + `]` (1) + one separator space (1) + a 4-cell **left-aligned**
percentage field with trailing padding (4). The field values are `0%  `, `5%  `,
`10% `, `65% `, `100%`. Every row of `Interface_Elements.txt`, including the rows
for 0 %, 5 % and 100 %, equals the rendered segment once trailing spaces are
removed. The canonical `.txt` is not edited.

---

# 25. Application shell

`Interface_Exemple.txt` represents the complete application composition.

Its canonical regions are:

| Region | Purpose |
|---|---|
| `MenuBar` | File, Edit, View and Help |
| `CommandBar` | workbench and CAD command groups |
| `NavigationPanel / Features` | parametric feature/history tree |
| `NavigationPanel / Elements` | resulting model entities grouped by type |
| `Viewport3D` | main 3D scene |
| `NavCube` | camera/orientation control |
| `ActionPanel` | current contextual Action Box |
| `StatusBar` | document and application state |

The complete reference currently shows an Extrude Action Box over the right side
of the viewport.

**Actual startup state.** The reference is an example composition, not the
application's launch state. On launch no contextual Action Box is displayed;
the command controller is idle and there is no Extrude preview. Extrude opens
only when the user activates its command.

The reference is 82 columns × 28 rows. The tree amendment of 2026-10-26
changes only its playhead terminal blank to `─`; other cells remain unchanged. The minimum
window grid is **82 columns × 30 rows** at every UI Scale and on every monitor
(`ARCHITECTURE.md` §3.1.1). When the window moves to a smaller monitor, the
number of columns and rows the user last chose is kept as far as it fits (never
below 82 × 30), is restored when the window returns to a larger monitor, and the
window is moved the minimum distance needed to stay fully on the monitor
(`ARCHITECTURE.md` §3.1.1, *Safe window fit*).

**Fixed and flexible rows.** In the reference, eight rows are fixed-height
regions: row 1 (top frame), row 2 (MenuBar), row 3 (separator), row 4
(CommandBar), row 5 (separator above the body), row 26 (separator below the
body), row 27 (StatusBar) and row 28 (bottom frame). Rows 6–25 are the
flexible central body (20 rows). The MenuBar, CommandBar and StatusBar heights
never grow. At a grid of `R` rows the body has `R − 8` rows: 20 at 82 × 28 and
22 at 82 × 30. Every additional row belongs to the body and, at the default
split, to the lower (`Elements`) pane and to the Viewport3D below the Action
Box; the Features pane keeps its reference height (rows 6–16, with the divider
at row 17) and the Action Box and NavCube keep their reference rows, anchored
to the top of the viewport.

**Comparing an 82 × 30 snapshot with the 82 × 28 reference.** A snapshot of the
default state at 82 × 30 is compared in two steps, without inventing any
difference:

1. Remove snapshot rows 26 and 27 (the two additional body rows, immediately
   below reference row 25). The remaining 28 rows MUST equal
   `Interface_Exemple.txt` row for row, cell for cell (snapshot rows 28–30 are
   reference rows 26–28).
2. Snapshot rows 26 and 27 MUST each equal the blank body-row template
   `║` + 23 spaces + `║` + 56 spaces + `║` (82 cells: the outer frame, the
   Elements pane interior, the panel separator and the viewport interior). No
   other content appears in them.

**Row 19 is a hover state.** Row 19 of `Interface_Exemple.txt`
(`│ ╞ □ Starting Offset         ╡║`) shows the pointer over the `Starting Offset`
checkbox row (§5.12); its normal state is `│ │ □ Starting Offset         │║`, as
in `Action_Box_Extrude.txt`. The reference file is not edited. The default
snapshot (hover `None`) is compared with the reference after replacing, in
reference row 19 only, `╞` by `│` at the branch column and `╡` by `│` at the
right frame column of the Action Box; a separate hover snapshot (that row
hovered) equals the unmodified reference row. No other cell of the reference is
a hover state.

Any other expected difference between the two sizes is a defect in the
snapshot or in the layout, not in the reference. States that differ from the
reference by design (open menus, other Action Box states, panel resizing,
Dark theme) have their own snapshots and are not compared with the reference.

The right end of the MenuBar row shows the window controls `[-] [□] [X]`.
They are grid cells of the frameless window (minimise, maximise, close) and
follow the same hit-testing rules as every other control.

## 25.1 Frameless window: move and resize regions

The window has no native decorations (M2.17) and stays movable, resizable,
minimisable, maximisable and closable from the grid. The binding mechanism is
`ARCHITECTURE.md` §3.1.1 (*Window move and resize*); this section defines the
regions and the precedence.

**Resize ring.** The outer frame of the composed grid (`╔ ═ ╗ ║ ╚ ╝` of
`Interface_Exemple.txt`, plus the junction glyphs `╠ ╣ ╦ ╩` that sit on it) is
inert chrome: those cells carry no `widget_id`. With the surface `sw × sh`
physical px and the pointer at `(x, y)` (floored, §3.4), the zones are:

```text
west   x <  W            east   x >= sw - W
north  y <  H            south  y >= sh - H
```

A corner is the intersection of two zones (north-west, north-east, south-west,
south-east), one cell `W × H` in size; there is no larger corner target. The
pixels right of or below the last whole cell (§3.1 leftover) belong to the
east/south zone. All tests are integer comparisons (O(1)).

**Precedence** (first match wins, evaluated on a left-button press):

1. A cell that carries a `widget_id` is a control: it wins, including the
   window controls `[-] [□] [X]`, menu items and any overlay or popup cell
   composed over the ring (for example a popup clipped at the grid edge, §29.1,
   or a surface cropped below the minimum).
2. A resize zone (above) starts a resize, whether or not a menu or modal
   overlay is open. Pressing it does not dismiss the overlay.
3. An inert cell of the MenuBar row (row 1, between the menu items and the
   window controls) starts a window drag (`drag_window`).
4. Anything else follows normal routing.

While a resize is in progress no widget receives a pressed state, a drag or a
release; the resize owns the pointer until it ends. The release that ends it is
consumed. Maximise/restore stays the `[□]` control; minimise and close are
unchanged. Resizing is inactive while the window is maximised or fullscreen.

**Result of a resize.** The window size is a whole number of cells, never below
`82 × 30` (`328k × 270k` px). Dragging west or north keeps the opposite edge
fixed, including when the minimum is reached. The grid is recomposed from the
size reported by `Resized`, never from the requested size (§3.1.1).

**OS cursor shape.** The cursor reflects the actionable pointer target, using
the same O(1) hit test and precedence as pointer input. Window edges and corners
show their matching resize cursor; the Navigation divider shows horizontal
resize, and the Features / Elements divider shows vertical resize. The Features
playhead and draggable scrollbar thumbs show Grab on hover and Grabbing while
captured. Unused MenuBar drag space shows Move. A widget or popup covering a
resize zone takes precedence, as it does for a press. The normal cursor returns
when the pointer leaves the window, focus is lost, or an overlay/state change
makes the handle unavailable. Cursor shape is an OS cursor shape, not a UI
rendering effect (`ARCHITECTURE.md` §3.5): it stores no UI state, redraws
nothing and changes no cell.

**Manual verification (M2.17, macOS).** The Executor records the result of each
item in the milestone report: (1) east, south and south-east drags follow the
pointer and stop at 82 × 30; (2) west, north, north-west, north-east and
south-west drags keep the opposite edge fixed with no visible jump, flicker or
drift, at k = 2, 3 and 5, on Retina (k = 2 on the Full HD monitor, with the 8 px
wide ring as the pointer target); (3) releasing outside the window ends the
resize; (4) Cmd-Tab during a drag ends it; (5) `[-] [□] [X]`, the MenuBar drag
and maximise/restore still work; (6) a press on a menu item in row 1 never
starts a resize. An item that fails is reported as a limitation, not patched
with unsafe code or a platform workaround.

**Manual verification (M2.17, multi-monitor, macOS).** On the MacBook Air
Retina display and an external 1920×1080 monitor, the Executor records: (1) the
View menu preference stays 100% on both and the cells are 20×45 and 8×18 px
(measured from a snapshot or log of `effective_k`); (2) dragging the window from
the Retina display to the 1080p monitor and back changes the cell size once,
with no flicker loop, no repeated resize and no oscillation (the log shows at
most one transition per move); (3) the window is fully on the 1080p monitor
after the move, its MenuBar row and `[-] [□] [X]` are reachable, and the grid is
at least 82 × 30; (4) moving back restores the earlier grid size on the Retina
display; (5) entering and leaving fullscreen on each monitor re-derives `k`
without moving the 100% reference; (6) changing the macOS display scale or the
external monitor resolution re-derives `k`; (7) clicking and menu hit-testing are
correct immediately after each transition; (8) on the 1920×1080 monitor (`k = 2`,
8×18 px) text, box-drawing frames, `.`/`,`, `:`/`;`, the `▏` caret and the
`░ ▒ ▓ █` bars are legible and frames show no gap, judged by eye and recorded
(ADR 0003, *Open compatibility points* 3). An item that fails is reported as a
limitation, never patched with unsafe code or a platform workaround.

---

# 26. Navigation trees

The exact historical reference fixture is the NavigationPanel of `Interface_Exemple.txt`
(columns 1–25 of rows 5–26, shown here with the panel borders; row 16 and rows
24–25 are blank panel rows):

```text
╠═══════════════════════╦
║[-]Features            ║
║ ├ □ Sketch_Profile_Ma…║
║ ├[-] Hub_Features     ║
║ │ ├ Extrude_Base      ║
║ │ ├ ■ Sketch_Hole     ║
║ │ ╰╴Extrude_Cut_Center║
║ ├ Fillet_Inner        ║
║ │ ─────────────────── ║
║ ├ Chamfer_Edges       ║
║ ╰╴Pattern_Circular    ║
║                       ║
╠═══════════════════════╣
║[-]Elements            ║
║ ├[-]Solid Bodies(2)   ║
║ │ ├ ■ Rim_Body        ║
║ │ ╰╴□ Hub_Body        ║
║ ├ Surfaces(0)         ║
║ ╰[+]Construct         ║
║                       ║
║                       ║
╠═══════════════════════╩
```

**Live labels and the reference exception (M4.2).** Live model-bound documents use
exactly three Elements groups, in order: `Bodies(N)`, `Surfaces(N)`, `Construct`.
`N` counts the group's direct entities, including hidden entities and zero.
Construct has no count suffix and contains construction planes, axes, curves and
points. The existing exact fixture above and `Interface_Exemple.txt` retain
`Solid Bodies(2)` unchanged. That historical label and its preset document state
apply only to the explicit reference fixture and its exact assertions; they do
not prescribe live startup contents or override the live `Bodies(N)` label.
The exact-reference comparisons in §25 concern that fixture, not a live document's
projected Features/Elements contents. The later tree amendment changes only the playhead endpoint artwork; M4.2
labels, identities and preset history gaps remain unchanged.

Tree topology uses:

```text
├
╰╴
│
```

for hierarchy: `├` for a child with following siblings, `╰╴` (two cells) for
the last child, `│` for a continuing branch. In the Elements tree the last
top-level entry is `╰[+]` followed directly by the marker, exactly as in the
reference. `Interface_Exemple.txt` is the more recent reference; the earlier
`└`-based tree examples in this section were stale and are removed.

The row `│ ─────────────────── ` (row 13 of the reference) is the *playhead
separator* between two features. It is not a feature and is not selectable.

**Built-in Origin folder (focused Executor amendment).** In every live document,
`Origin` is the first child of Features, before all operation-history rows. Its
children, in exact order, are `O`, `X`, `Y`, `Z`, `XY`, `XZ`, `YZ`. Use existing
named-folder/leaf row grammar, with no checkboxes. Origin starts collapsed;
expansion is per-window presentation state. Stable tagged `OriginFolder` and
`OriginReference` identities are separate from operation/folder/entity IDs and
remain stable across history publication. The fixed historical artwork fixture
above remains unchanged; it is not live startup content.

Origin and its children are immutable reference rows: no geometric selection,
visibility, rename, delete, reorder, insertion boundary or playhead host. They
never enter `ModelTreeProjection.features` history or Elements/counts. Existing
operation order and selection remain unchanged. Presentation maps history rows
by semantic identity, never by an offset from the Features header. A playhead
drag over any Origin row leaves the cursor unchanged. The first operation gap
is displayed after the entire visible Origin subtree; an empty history's End
is likewise after that subtree. Collapse/expand retains the same history gap.

**Live insertion cursor (M4.2).** The playhead identifies where a new operation
will enter history. In a live document it may be placed before the first
operation, between operations, or at End after all operations, including End of
an empty history. Live initialization and document replacement default to End.
The existing separator glyph is used at each boundary; it remains a drag handle,
never a selectable operation. When Features is expanded, End of an empty history
is shown after the built-in Origin subtree. Later operations remain displayed. Moving the cursor
does not roll back, recompute or hide geometry.

An insertion boundary and its display location are distinct. Folder headers do
not count as operations. The separator may be displayed before a folder header
or after its whole subtree while its insertion anchor remains the next operation
in history, or End if no operation follows. Folder-boundary display placement
must be explicit, not inferred from a folder name. The historical fixture keeps
its original interior top-level gaps and initial separator after `Fillet_Inner`.
In particular, its first admitted gap is displayed before the `Hub_Features`
header and anchored before that folder's first operation, `Extrude_Base`.
These fixture placements remain unchanged; endpoint assertions migrate to §26.1.

**Truncated labels.** A label that does not fit its row is shortened to the
available cell count by keeping its leading characters and replacing the final
available cell with the single character `…` (U+2026), as in
`Sketch_Profile_Ma…` (the label occupies the 23-cell panel interior exactly,
with no cell left before the panel border). One Unicode scalar value is one
cell; labels are limited to single-cell characters. `…` is also the character
used in `[Select…       ]`. It is a font glyph (verified present in
`assets/FiraCode-Regular.ttf`) and belongs to the required atlas set
(`ARCHITECTURE.md` §3.1, Glyph coverage).

```text
[-]
```

means expanded.

```text
[+]
```

means collapsed.

The square state displayed beside model entities is intended as the entity
display/visibility state:

```text
□
```

hidden/off.

```text
■
```

visible/on.

The Features tree and Elements tree are different views of model information.

Features represents modeling operations in history order, with the built-in
Origin reference folder first. Startup seeded geometry is listed only in
Elements; its stable producer identity remains available for dependencies.

In a live document, each Elements square targets its row's stable entity ID. A
Base operation row in Features also has a visibility square when it has real
outputs; its state is checked only when all those output entities are visible,
and toggling it updates those entities by ID. The corresponding Elements rows
therefore stay synchronized, including when names or row order change. A
visibility change keeps the operation, outputs and geometry intact, while
removing hidden entities from viewport drawing, viewport picking and visible
model fitting. Other history rows and reference/navigation graphics remain
independent.

Live Elements groups resulting entities as:

```text
Bodies
Surfaces
Construct
```


## 26.1 Features-only playhead geometry

Amendment authorized on 2026-10-26. This section supersedes old M2/M4.2
playhead terminal padding, collapse relocation and tree-visual exclusions.
Elements never contains a playhead, its hit target or an insertion action.
A playhead is not a node, selection target, focus target or history operation.

Use zero-based grid coordinates. A pane frame has left column `L`, outer width
`O`, right border `R=L+O-1`, interior start `I=L+1` and interior width
`W=max(O-2,0)`. Interior offsets are `0..W-1`; borders are never overwritten.
The stroke ends at column `R-2=I+W-2`, leaving the final interior cell blank
before the panel-owned right border.

**Proposed left endpoint, resolved here as the implementation policy:** the
original requirement did not finish its left-endpoint sentence. The following
depth-based choice preserves the illustrated separation and existing two-cell
indentation; it must not be attributed to that incomplete supplied sentence.
For a gap at structural depth `d>=1`, set `S=2*d+1` (interior offset). Draw
`─` at offsets `[S,W-1)`. The count is `max(W-1-S,0)`. Ancestor guide levels
`j=1..d` occupy offsets `2*j-1`, with a blank immediately after each at `2*j`.
For `j<d`, draw `│` iff the ancestor at depth `j` has a later sibling
under its own parent. For `j=d`, draw `│` for BeforeNode (the referenced child
still follows the gap), and for AfterSubtree iff that node has a later sibling
under the same parent. EndOfTree draws no guide. Otherwise leave the slot blank.
Determine siblings from validated structural preorder/depth, not labels,
visibility, selection or the previous row's glyph. The innermost guide therefore
continues exactly when a following child remains at that level. Preserve
outer continuing guides even when the innermost branch has ended. No horizontal
stroke crosses a guide. EndOfTree has depth 1 and no continuing root-child guide.
BeforeNode and AfterSubtree retain their structural node depth; their ancestors
are found by identity/preorder. Clip to interior cells with bounded arithmetic;
if `S>=W`, draw only fitting guides, with no stroke hit span.

At `O=25`, `W=23`: depth 1 starts at global `L+4` (offset 3), with 19
stroke cells; depth 2 starts at `L+6` (offset 5), with 17 stroke cells.
These equal-width reference rows have continuing hierarchy guides:

```text
║ ├ Fillet_Inner        ║
║ │ ─────────────────── ║
║ ├ Chamfer_Edges       ║

║ │ ├ ■ Sketch_Hole     ║
║ │ │ ───────────────── ║
║ │ ╰╴Extrude_Cut_Center║
```

Only the horizontal stroke cells own PLAYHEAD_ID and admit drag. Gap guides,
indentation and separator blanks are inert; panel borders retain resize
ownership. This makes the stroke itself the deterministic interaction target.

## 26.2 Collapsed inline playhead

Keep the entire `PlacedHistoryGap { insertion, display }` unchanged on collapse
and expand; never call `placed_gap(AfterSubtree(folder))` just to collapse.
For BeforeNode/AfterSubtree, its containing folders are the strict folder
ancestors of the referenced node; a gap before a folder or after its complete
subtree is outside that folder. EndOfTree belongs to Features itself. This
identity-based rule distinguishes an internal AfterSubtree(last child) from an
external AfterSubtree(folder) even if their operation anchors coincide.

If one of those ancestors is collapsed, show the indicator on the visible
collapsed ancestor row (the outermost collapsed ancestor on the path from
Features). Inner collapsed ancestors remain hidden. Draw no separate gap row.
Re-expansion progressively exposes the deeper ancestor or the original gap;
neither action moves the anchor or changes document/history. Collapsing the
Features root hides the indicator entirely, retaining the pair. Scrolling can
hide the row normally. Refresh/reconciliation retains the existing M4.2 anchor
survival/deletion rules, but must not auto-expand ancestors just to show a
retained cursor. Explicit cursor placement may retain its existing reveal rule.

Let `P` be the natural fixed prefix length before this folder's label, and `N`
its full name length. If `W>=P+6`, allocate `A=W-P-5` cells to the label,
`D=min(N,A)` displayed cells, then one blank, then `W-P-D-2` stroke cells and
one trailing blank. The stroke ends at `R-2`, leaving `R-1` blank.
Ellipsize with `…` if `N>A`: retain the first `A-1` scalars then `…`.
There are always at least three visible `─` cells, right-aligned to `R-2`.
Short names produce a longer stroke, never additional blanks. Preserve the
underlying full name and semantic target; the inline indicator is not its name.

If `W<P+6`, fitting the fixed controls, one label cell, separator, three
stroke cells and trailing blank is impossible. Suppress the inline indicator and its hit span,
render the ordinary clipped folder row and retain the anchor. Do not shorten
controls, overwrite guides/borders, or draw a misleading one/two-cell stroke.
This is the explicit narrow/deep fallback, not permission to relax the minimum
for a displayed inline indicator. Width measurement never includes its stroke.

Name cells alone retain Rename ownership; marker cells retain expansion,
checkbox cells retain visibility; the one separating blank belongs to Row.
Only the `─` cells have PLAYHEAD_ID. Pressing the stroke clears a pending
Rename double-click and captures playhead drag without selecting/toggling the
folder. A drag staying on its own inline host row is a no-op, retaining its
internal anchor. Moving onto another row uses existing M4.2 admitted gaps and
drop rules (a deliberate drop on a different folder remains AfterSubtree).
Elements cannot initiate a playhead action; movement there during an already
owned Features drag retains the existing Features-only row clamping policy.

Examples below include their full interior prefix; widths are derived, not
copied from the old padded sketches:

```text
║ ├[+] Hub_Features ─── ║
║ ├[+] Hub_Feat… ─── ║
║ ├[+] Hub_Features ────── ║
```

Their outer widths are respectively 25, 22 and 28 cells.

## 26.3 Shared tree-column maximum width

Measure both Features and Elements in the current document/reference
projection, including roots/headings, all group headings/count suffixes and
all descendants even if collapsed or scrolled out. They share one column and
one cap. Include hidden entities' rows. Measurement is independent of expansion,
scrolling, hover, selection, visibility state and the current displayed width.
Do not measure already ellipsized strings. A scalar occupies one cell under §26.

For a row at depth `d`, the hierarchy prefix length is `H=0` at depth 0;
otherwise `H=2*d` for a branch and `H=2*d+1` for a leaf. Natural row length is:

`C = H + (branch ? 3 : 0) + (branch && spaced ? 1 : 0)
       + (has_visibility_square ? 2 : 0) + full_label_scalar_count`.

This includes leading indentation, connectors/tails, the full three-cell handle,
its required spacing, and the square plus following blank. Use the same
`full_label` and prefix geometry for measurement and drawing. Empty groups use
the actual leaf/branch presentation; headings remain relevant even when no
history/entities exist. Labels unsupported by the atlas still each consume
one fallback cell. No playhead line, filler, inline reservation, hover substitute,
outer border or viewport annotation contributes to `C`.

Let `M=max(C)` across those rows, `content_interior_max=M+2` (two trailing
padding cells), and `content_outer_max=M+4` (two separate border cells).
The padding is only a cap allowance; labels may use all available interior
cells at smaller widths. A playhead stroke ends at `R-2`, preserving the final
interior blank before the right border.
For the explicit reference fixture, M=24 from ` ├ □ Sketch_Profile_Main`:
its content cap is 26 interior / 28 outer cells, not 24 or 26 outer cells.

Keep the existing outer minimum 14 and half-window ceiling. For `cols` columns:
`U=min(content_outer_max, floor(cols/2))`, `B=min(14,U)`,
`effective_outer=clamp(preferred_outer,B,U)`. Roots ensure M>=11 in ordinary
projections, so the content cap is at least 15 and does not conflict with the
14-cell minimum. The window ceiling wins for synthetic grids too narrow for
that minimum. Production retains the 82×30 window minimum. At zero/tiny widths,
empty rectangles and saturating border arithmetic remain safe; no pointer owner
may leak into Elements, viewport or off-grid cells.

Initialize `preferred_outer=min(25,content_outer_max)`. A resize request stores
`clamp(request,14,content_outer_max)` (ordinary projections), independent of the
window ceiling. Content shrink permanently clamps the stored preference down;
content growth retains it, never auto-expands. A temporary window shrink only
reduces effective width, so growing the window restores the stored preference
subject to the current cap. Recompute the content cap on projection replacement,
rename/label change, add/remove/reparent/depth change, group-count or heading
change, including same-document updates. Clamp before publishing the frame and
hit map. Collapse, scroll, playhead movement, selection, hover, Theme and Color
do not change the cap. Window/metrics and panel visibility changes recompute
only effective layout; hiding the column retains its preference and content cap.

## 26.4 Exact row glyph states

These rules apply to both trees' visible node rows. Selected is a semantic
membership predicate, never inferred from focus, hover, checked or expanded.
For now consume M4.2's existing `ShellState.selected` identity; do not alter
activation behavior or create shared-selection policy. M4.3 owns the authoritative
viewport/tree/Selection-panel selection integration. Its backlog entry and the
current model `Selection` entity collection do not yet define a feature/folder
membership mapping or modifier behavior; this amendment does not invent one.
When that contract is specified, feed its membership predicate to the same row
visual without adding a second selection store. Hidden selected rows retain
identity and regain their visual when shown.

| Structural slot | Normal | Hovered | Selected | Selected + hovered |
|---|---|---|---|---|
| Continuing ancestor on this row | `│` | `│` | `┃` | `┃` |
| Non-last connector | `├` | `╞` | `┣` | `┣` |
| Last leaf connector + tail | `╰╴` | `╘ ` | `┗╸` | `┗╸` |
| Last branch connector before handle | `╰` | `╘` | `┗` | `┗` |
| Right frame, Color off | `║` | `╣` | `║` | `╣` |
| Right frame, Color on | `│` | `╡` | `│` | `╡` |

Blank ancestor slots stay blank. Non-last leaf spacing stays blank. Root rows
have no connector or ancestor slots, so hover changes only their right frame;
selection alone adds no root marker. Names, expansion handles, squares, spaces,
left frame and all unrelated rows/borders stay unchanged. The hierarchy table
uses Text ink in all four palettes; right frame stays Contour. A selected
inline-host row follows this table, but its horizontal playhead stays `─`.
For a hovered last leaf, the tree tail cell is blank. The selected-tree `┗╸`
form is reserved for selection; Action Box hover continues to follow its own
connector-slot rule in §5.12.

The exact supplied 25-cell rows, preserving `╸`, are:

```text
Normal
║ │ ├ Extrude_Base      ║
║ │ ╰╴Extrude_Cut_Center║
Hovered (each row independently)
║ │ ╞ Extrude_Base      ╣
║ │ ╘ Extrude_Cut_Center╣
Selected
║ ┃ ┣ Extrude_Base      ║
║ ┃ ┗╸Extrude_Cut_Center║
Selected + hovered (each row independently)
║ ┃ ┣ Extrude_Base      ╣
║ ┃ ┗╸Extrude_Cut_Center╣
```

Only one row can hover at once; the paired hover examples are alternatives,
not a simultaneous two-row hover state. Selection wins hierarchy slots in the
combined state: this uses existing required glyphs while making membership
stable under pointer movement. On pointer exit, restore selected glyphs if
still selected, otherwise normal glyphs. On deselection while hovered, restore
the hover-only mapping. Recompose from semantic state, never reverse-map the
previous glyphs. No inversion, animation, colour shift or increased pixel line
width: heavy glyphs use the unchanged regular font and uniform atlas scale.

## 26.5 Hover routing, publication and coverage

Resolve a hit via the published CellGrid and visible slot table. Row, Name,
Marker and Check parts normalize to that semantic row's existing Row WidgetId
for hover only, in O(1). Click routing and Rename identity remain unchanged.
Inert frames, PLAYHEAD_ID cells and gap guides never acquire row hover. Map
validation after recomposition must use the same normalization rather than
comparing a Name/Marker/Check id directly with the stored Row id. No row-width
scan or bounding-box search is allowed. Scrolled/reused slots, disappearing
rows, modals/menus and resize clear stale hover through existing precedence.

A right-frame hover glyph is visual only, with unchanged resize hit metadata.
Compose it after Color's contour normalization, or explicitly exempt that slot
and emit `╡` in Color mode; the generic `╣→┤` conversion must not erase it.
Glyph/ink changes alone do not change routing generation. Name truncation,
inline stroke ownership, row count, width and slot changes do. Publish grid,
slots and routing policy atomically. Preserve the controller's owned capture
identity/origin and rebind only the matching drag's newly published generation;
unrelated map changes cancel capture as in M4.2. Release, focus/pointer loss,
overlays, scale and document replacement retain their cancellation rules.

While a Features playhead drag is captured, suppress row-hover glyphs in both
Features and Elements. Clear any tree hover when the capture begins; pointer
movement across rows or between the trees must not acquire tree hover. This
applies to standalone and collapsed-folder inline playheads. Use the existing
captured `Separator::Playhead` state as the sole authority; do not add a second
suppression flag. Selection glyphs, focus, expansion, insertion anchors and
playhead movement remain unchanged. On release or cancellation, resolve hover
again from the current pointer against the current published semantic map, so a
reused slot can only hover its current target. Ordinary pointer motion without
playhead capture keeps the existing hover behavior.

Required glyphs are `─ │ ├ ╰ ╴ ╞ ╘ ╸ ┃ ┣ ┗ ║ ╣ ╡ …`, plus existing controls
and Color frame glyphs. Static inspection on 2026-10-26 confirms all occur in
`glyph_atlas.rs::UI_SYMBOLS` and in the bundled Fira Code cmap (formats 4/12).
No atlas-set addition or font change is necessary. Runtime atlas coverage and
visual continuity at k=2…7 must still be verified by Executor; static font/set
coverage does not establish raster or physical-display acceptance.

---

# 27. Status bar

The complete example contains:

```text
Roue V1* │ Part Design │ Fully Constrained │ 98fps │ Progress [███░ ] 65%
```

Meaning:

- `Roue V1`: current document name;
- `*`: document contains unsaved changes;
- `Part Design`: current workbench;
- `Fully Constrained`: contextual status;
- `98fps`: current rendering frame rate;
- `Progress [███░ ] 65%`: current progress state.

**Fixed-width frame-rate field.** The frame rate is a measured integer. Its
segment is exactly **7 cells** between the two `│` separators and never changes
width:

The 7 cells are a **3-cell right-aligned numeric field**, then `fps`, then one
blank cell. The leading blank of the reference ` 98fps ` *is* the field's first
(padding) position, so no cell is added to the reference row. The field is the
three cells directly after the left separator, right-aligned, padded on the left
with spaces (never zeros):

| value | 7 cells (between `│` and `│`) |
|---|---|
| 0 | `  0fps ` |
| 5 | `  5fps ` |
| 98 | ` 98fps ` (identical to `Interface_Exemple.txt`) |
| 144 | `144fps ` (the left blank is used by the third digit) |
| 999 | `999fps ` |
| finite, above 999 (for example 1000) | `999fps ` (saturated, see below) |
| NaN, `+∞`, `−∞`, negative | `  0fps ` |

Consequences: the separators, `Progress [███░ ] 65%` and the right border stay in
the same columns for every value from 0 to 999; no adjacent field shifts when
the digit count changes; at 98 the row equals the reference cell for cell. At
three digits the field touches the left separator, an accepted consequence of
keeping the reference width.

**Value rules** (pure composition function; the input is a measured number, not
a clock read), applied in this order:

1. A NaN, `+∞`, `−∞` or negative input is shown as `0`. Infinite values are not
   "finite values above 999" and never saturate to `999`.
2. Any other input (finite and non-negative) is rounded to the nearest integer,
   halves away from zero.
3. A value above 999 after step 2 is shown as `999` (saturation: the field never
   grows to four digits and never shows a `+`, `>` or `…`).

The digits are always the plain ASCII digits `0`–`9`.

**Other status text.** The segments are composed left to right, joined by
` │ `. The document name and the contextual status are free text: a different
length moves the segments after it, and text that would pass the right border is
clipped whole-cell and never wraps (the §26 ellipsis rule does not apply to the
status bar). Only the frame-rate field is fixed-width, so the frame rate itself
never moves another field. The status bar's only pointer target is the
progress segment (below). A snapshot comparison with `Interface_Exemple.txt`
injects the value `98` and the progress value `65`.

## 27.1 Progress segment: click-driven demo animation

**Target.** The entire `Progress [█████] N%` segment (from `P` through `%`,
every cell it owns, including the brackets, the separator space, the percentage
digits and their trailing padding) is one click target with **one stable
`WidgetId`**. The segment is located by O(1)
cell lookup (`ARCHITECTURE.md` §3.4); there is no bounding-box scan, no
retained widget tree and no hover effect (§3.5, §5.12: the progress segment is
not one of the four hoverable kinds). Cells outside the segment,
including the ` │ ` separator before it, are not part of the target. The
segment activates on **left-button press**, once per press; holding the button
down never repeats the activation. This is the one animation authorised by
`ARCHITECTURE.md` §3.5; it is text-cell updates only, with no shader animation.

**State machine.** The progress value is one of the 21 states of §24
(0–100 %, steps of 5 %). The initial state is `0%`.

| State | Click | Result |
|---|---|---|
| idle at 0 % | accepted | animate 0 → 100 % over **5 s** |
| idle at 100 % | accepted | animate 100 → 0 % over **1 s** |
| animating (either direction) | **ignored** | nothing: no queueing, no reversal, no restart |

When an animation ends the state is exactly `100%` or `0%` and the cycle may
be repeated indefinitely. Idle states other than 0 and 100 do not exist for
this control (the other 19 states are only transient frames, or are injected
into the pure composition function by tests and snapshots).

**Time.** Progress is a function of **elapsed time**, never of frame count or
redraw count, read as monotonic `Instant` deltas from an injectable clock.
With `t = min(elapsed, duration)` and `D = duration`, in exact integer arithmetic
on the elapsed nanoseconds (no floating point):

- ascending (0 → 100 %): `p = floor5(100 · t / D)` = `5 · ⌊20 · t / D⌋`, so `100 %`
  only when `t ≥ 5 s`;
- descending (100 → 0 %): `p = ceil5(100 · (D − t) / D)` = `5 · ⌈20 · (D − t) / D⌉`,
  so `0 %` only when `t ≥ 1 s`.

`floor5(x)` is the largest multiple of 5 that is `≤ x`; `ceil5(x)` is the smallest
multiple of 5 that is `≥ x`. Neither is a nearest-value rounding: `floor5` never
returns more than `x`, `ceil5` never returns less than `x`, and an exact multiple
of 5 is returned unchanged. The integer forms keep the endpoints exact at
`t = 5 s` and `t = 1 s`. The animation is
**active while `elapsed < duration`**, and this does not depend on the displayed
value. A slow or stalled frame skips states but never changes the total
duration, and one large clock jump gives the same value as many small steps.
The composition function takes the integer percentage as input and reads no
clock, like the frame-rate field.

**Redraw scheduling.** The next wake-up is `min(next fps tick, next progress
step)` (`ControlFlow::WaitUntil`). Progress steps fall on multiples of
`duration / 20` (250 ms ascending, 50 ms descending). While idle, progress adds
no deadline.

**Independence from FPS.** The `fps` field keeps its own fixed **5 Hz**
refresh (§27 above). Animation redraws do not change the FPS sampling cadence
and the FPS refresh does not advance, restart or stall the animation.

**Input precedence.** On left-button press, the first matching rule applies. The
order is that of §25.1, with the overlay rule of this section added; no other
order exists.

1. An active resize (§25.1) owns the pointer. The segment receives no press.
2. A cell carrying a `widget_id` routes to its widget (§25.1, rule 1). Overlay
   and popup cells are such cells, so they win over resize zones. Exception: while
   an overlay or popup is open, the segment never receives a press; rule 5
   applies to it.
3. A press on a resize zone (§25.1, rule 2) starts a resize and does not dismiss
   an open overlay.
4. An inert MenuBar row cell starts a window drag (§25.1, rule 3).
5. Otherwise, while an overlay or popup is open, the press is an outside press. It
   is consumed and never activates the segment. It dismisses the overlay only if
   that overlay's existing overlay rule (§29.1) dismisses on an outside press; an
   overlay whose rule does not dismiss on outside presses stays open. Consumption
   does not depend on dismissal.
6. Otherwise normal routing applies: a segment cell activates the animation when
   idle.

Hit-testing stays O(1) with one stable `WidgetId`.

**Rendering.** The segment is exactly **21 cells**: `Progress [` (10) + five bar
cells `░ ▒ ▓ █` in 5 % steps (§5.9, §24) + `]` (1) + one separator space (1) +
a **4-cell left-aligned percentage field** with trailing padding (4). Its left
bound (`P`), right bound (`%` or the last padding cell) and the right border never
move. The separator space and padding belong to the clickable segment.

| value | segment (21 cells, between the ` │ ` separators) |
|---|---|
| 0 | `Progress [     ] 0%  ` |
| 5 | `Progress [░    ] 5%  ` |
| 10 | `Progress [▒    ] 10% ` |
| 65 | `Progress [███░ ] 65% ` |
| 100 | `Progress [█████] 100%` |

Every value equals the row of `Interface_Elements.txt` with the same value once
trailing spaces are removed (§24). In `Interface_Exemple.txt` the segment is the
last status segment, so the extra padding cell reduces the trailing blanks before
the right border (`║`) and moves nothing.

---

# 28. Application Menu Bar

`Application_Menu_Bar.txt` defines four menus:

```text
File
Edit
View
Help
```

---

## 28.1 File

```text
New
Open
Recent
Save
Save As
Import
Export
Option
```

Intended actions:

- `New`: create a new document;
- `Open`: open an existing document;
- `Recent`: access recent documents;
- `Save`: save the current document;
- `Save As`: save using another name/location;
- `Import`: import external data;
- `Export`: export data;
- `Option`: application options/preferences.

---

## 28.2 Edit

```text
Undo
Redo
Recompute
```

- `Undo`: reverse the latest reversible operation;
- `Redo`: restore the latest undone operation;
- `Recompute`: explicitly recompute the model/document.

---

## 28.3 View

Current menu:

```text
╭─View──────────────╮
│ ◀ Orthographic  ▶ │
│ ◀  Hidden Line  ▶ │
│ UI Scale ◀ 100% ▶ │
│ Theme [ ◑ Dark  ] │
│ □ Color           │
│ ■ Tree panel      │
│ ■ Task bar        │
│ ■ Status bar      │
│ ■ View Navigator  │
│ ╰╴   ◀ Gizmo  ▶   │
│ □ Grid Overlay    │
│ [N]ormal to       │
│ Reset Cam         │
│ Fullscreen        │
╰───────────────────╯
```

`View > Task bar` controls the complete Command Bar, including its frame. When
unchecked, the bar and frame are absent, their two layout rows are reclaimed,
and the shell content below moves up by two rows. The hidden bar has no pointer
targets and F6 cannot open it. Checking it again restores the original layout.
The File/Edit/View/Help MenuBar remains available.

Projection selector states:

```text
◀  Perspective  ▶
◀ Orthographic  ▶
```

Every new application window starts with Orthographic selected. Perspective
remains available as the adjacent selector state; the choice is per-window.

Display-style states:

```text
◀    Points     ▶
◀     Line      ▶
◀  Wireframe    ▶
◀  Hidden Line  ▶
◀  No shading   ▶
◀    Shaded     ▶
◀    Default    ▶
```

UI Scale states (`Application_Menu_Bar.txt`):

```text
│ UI Scale    80% ▶ │
│ UI Scale ◀ 100% ▶ │
│ UI Scale ◀ 120% ▶ │
│ UI Scale ◀ 140%   │
```

`UI Scale` is a bounded selector (§5.2) over four displayed labels. At 80% the
left arrow is replaced by a blank cell; at 140% the right arrow is replaced by
a blank cell. These cells retain the popup surface hit target but have no
action or hover feedback. The selector never wraps. At 100% and 120% both arrows
are shown. The labels
map to the original rendering multipliers as follows: 80% → factor 60%, 100% →
factor 80%, 120% → factor 100%, and 140% → factor 120%. The old 140% factor is
removed. The label is not used as a new multiplier: the existing integer cell
size calculation continues to use the mapped factor (`ARCHITECTURE.md`
§3.1.1). On a monitor that can fit each step, the displayed values correspond
to 12×27, 16×36, 20×45 and 24×54 px respectively. The new default label is
100%, using the size formerly associated with 80% (16×36 px at reference scale
5); on Full HD, existing minimum-cell behavior resolves the four labels to
8×18 px (`k = 2`).

- The row always shows the selected label, never the derived `k`: it reads
  `100%` on every monitor, and moving the window between monitors never changes
  the label. The adaptation shows no dialog and no notice.
- `◀` selects the previous and `▶` the next label. At 80%, the left arrow is a
  blank, inert cell; at 140%, the right arrow is a blank, inert cell. Neither
  arrow wraps at an endpoint. Both arrows work at 100% and 120%. Other cyclic
  selectors keep their wrapping behavior. Labels are never skipped or hidden,
  even when the current monitor maps several to the same cell size
  (`ARCHITECTURE.md` §3.1.1, *Several labels may share one `k`*).
- There is no 60% entry. An optional 60% label would use the former 40% factor;
  on compact reference-scale-2 displays that factor requests `k = 1` and is
  raised to the existing minimum `k = 2`, duplicating other presets, so it is
  not offered.
- `100%` is the default displayed label (factor 80%). New application, renderer, UI shell and View-menu states
  use Dark theme, Color unchecked, Orthographic projection and Hidden Line shading. The initial renderer therefore draws white
  foreground on a black background and uses hidden-line display before a View-menu value is changed.
- When the cell size changes (a step that changes `effective_k`, or the window
  moving to another monitor, a resolution or display-scale change, or entering or
  leaving fullscreen) the whole UI is recomposed in one atomic transition
  (metrics, glyph atlas, grid, GPU buffers, minimum window size and pointer
  mapping together); an open menu stays open at its new, recomputed position.
  Focus, selection and expansion are kept. The pointer does not move, so the
  control under it is generally a different cell afterwards; a press in progress
  is cancelled and its release does nothing (`ARCHITECTURE.md` §3.1.1,
  *Transition*).
- The automatic part is M2.17 and has no UI; this selector is M2.18 and contains
  no sizing logic.

Theme states:

```text
Theme [ ◐ Light ]
Theme [ ◑ Dark  ]
```

The bracketed field shows the current theme and is activated to switch to the
other one. Activating the Theme row by pointer or Enter/Space toggles the theme
while keeping the View popup open and the Theme row focused, so it can be toggled
again. Theme retains this explicit keep-open behavior as a menu item; View
checkboxes retain their established toggle/open-popup/focused-row behavior, and
other View actions retain their existing menu closing behavior. With Color unchecked, Light is black on white
and Dark is white on black (`ARCHITECTURE.md` §3.6).

Color states are `□ Color` (initial state) and `■ Color`, on the row immediately after Theme. Color is a per-window checkbox and uses the established View checkbox activation: pointer or Enter/Space toggles it and keeps the popup open with that row selected. Theme and Color are independent; new windows start Dark + unchecked, and a Theme change preserves Color while updating the visible Light/Dark menu row. Unchecked preserves the exact existing Light/Dark glyphs and monochrome colours, with only the scoped neutral placeholder ink exception in §5.13. Checked uses the four-state palette and semantic cell ownership in `ARCHITECTURE.md` §3.6. Text and values inside a control use `Text`, except initial placeholder characters explicitly covered by §5.13; only its explicit affordance uses `Action`. In both `Theme [ ◑ Dark  ]` and `Theme [ ◐ Light ]`, the word `Theme` and the displayed value are `Text`, `[`/`]` and `◑`/`◐` are `Action`, spaces have no visible ink, and the popup's outer `│` delimiters are `Contour`. Theme remains one item with its current pointer, Enter/Space, focus and open-popup behavior. Menu popup, dialog and selection-panel title letters use `Text` inside a `Contour` top edge: corners, horizontal strokes, frame junctions and structural separators stay `Contour`. Treeview guides and connectors use `Text` even beside a Contour frame. At one shared or intersecting frame cell the visible frame owns the cell as `Contour`; routing metadata does not determine ink. Color checked maps double-stroke frame glyphs to same-cell single-stroke glyphs exactly as in §3.6. Hover keeps the §5.12 Unicode substitutions and the role of each substituted cell; it adds no colour, brightness, animation or shader effect. The viewport clears to the chosen UI background without changing model display styles.

| Theme / Color | Background | Text, values, labels, tree guides | Contour | Action |
|---|---|---|---|---|
| Dark / off | `#000000` | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` |
| Light / off | `#FFFFFF` | `#000000` | `#000000` | `#000000` |
| Dark / on | `#071521` | `#FFFFFF` | `#41B4E5` | `#FFA24A` |
| Light / on | `#FFFFFF` | `#111827` | `#0969A0` | `#9A4D00` |

The table gives sRGB source colours. Glyph coverage provides antialiasing; it
does not introduce a separate state colour. Color-on single-stroke contours
retain all cell coordinates and hit targets. Hover `╞`/`╡`/`╘` remains the
approved stroke-form exception and inherits the original role.

The seven display-style states remain in the order shown above. Hidden Line is
the startup state; navigation remains cyclic in both directions. Points shows mesh
vertices; Wireframe shows mesh edges, including triangulation. Line shows only
visible solid geometry contour and feature edges. Hidden Line shows the same
geometry edges solid where visible and dashed where occluded, including when
visibility changes within an edge. No shading retains its unlit surfaces;
Shaded retains its lit surfaces. Default combines Shaded surfaces with Line's
visible solid geometry edges. Face tessellation diagonals never appear in Line,
Hidden Line or Default.

The five Tree panel, Task bar, Status bar, View Navigator and Grid Overlay
checkboxes control their corresponding visibility. View Navigator replaces the
former Navigation cube label; its conditional Gizmo/Cube child follows §41. Color selects semantic
ink and contour glyphs independently; it changes none of those visibility flags.

`[N]ormal to` contains an explicit keyboard mnemonic.

`Reset Cam` resets the camera.

`Fullscreen` toggles full-screen display.

---

## 28.4 Help

Current popup (`Application_Menu_Bar.txt`):

```text
╭─Help──────╮
│ Tutorial  │
│ Docs      │
│ Shortcuts │
│ Bug       │
│ Update    │
│ About     │
╰───────────╯
```

The Help popup follows the same fixed-cell conventions as the File and Edit
popups: a 13-cell frame, one leading space before each entry, and entries padded
with spaces to the right frame. The reference contains only space characters;
tab characters MUST NOT appear in any `.txt` reference.

---

# 29. Command Bar

The Command Bar follows `Command_Bar.txt`: Workbench, Datum, Create, Shape,
Modify, Pattern and Utilities, in that order. Its corrected header agrees with
the seven popup definitions. This supersedes the CommandBar row in the older
`Interface_Exemple.txt` shell reference; other shell rows remain authoritative.
Its visibility and two-row layout behavior are controlled by `View > Task bar`
as specified in §28.3.

Current workbenches:

```text
Part design
Assemblies
Mesh
Parametric
Drawing
Simulation
Animation
Manufacture
PLM
```

Datum contains `[S]ketch`, `[Pl]ane`, `[A]xis` and `[P]oint`.
Sketch and the other unimplemented commands remain UI-only mocks: they close
the popup without creating geometry, features or Model/BrepKIT operations.

Current command groups (exact labels, separators and order are in the reference):

| Group | Commands |
|---|---|
| Create | `[E]xtrude`, `[R]evolve`, `[L]oft`, `[S]weep`; Hole, Thread, Thicken; Helix, Primitive Body |
| Shape | Projected Curve, Bridging Curve, Composit Curve, Intersect Curve; Offset Surface, Fill Surface, Constrained |
| Modify | `[F]illet`, `[C]hamfer`, Shell, Draft, Face Blend; Boolean, Split Face, Split Body; Transform, Delete Face, Move Face |
| Pattern | Mirror, Linear, Circular, Curve |
| Utilities | `[M]esure`, Split View, Curvature, Properties |

Semicolons in this table denote separator rows. Existing keyboard aliases for
commands moved between groups are retained within their new popup, including
`B` for Boolean and the multi-letter Pattern aliases (`Mi`, `Li`, `Ci`, `Cu`).

Bracketed characters identify the mnemonic associated with the command.

For example:

```text
[E]xtrude
```

visually identifies `E`.

```text
[Mi]rror
```

visually identifies `Mi`.

The ASCII design specifies the mnemonic but does not by itself require a
particular implementation mechanism for multi-character mnemonics. A mnemonic
is scoped to the popup that is open (`[S]` appears in several popups).

## 29.1 Placement of CommandBar popups

The CommandBar popups (Workbench, Datum, Create, Shape, Modify, Pattern,
Utilities) are centred in the grid, not anchored under their header.
For a popup of `popup_width × popup_height` cells (frame included) in a grid of
`grid_cols × grid_rows` cells:

```text
col = floor((grid_cols − popup_width) / 2)
row = floor((grid_rows − popup_height) / 2)
```

with integer arithmetic on non-negative values: if a difference is negative the
coordinate is 0 (the popup is clipped at the right/bottom grid edge, never at
the left/top edge and never wrapped). The popup size is that of its `.txt`
reference (for example Workbench 19 × 11, Datum 19 × 6), unchanged by
centring. The grid is the whole composed grid (82 × 30 or larger), not the
viewport region, so the result is stable under panel resizing.

The popup is composed last (foreground), over every other region including the
Action Box and other overlays, in the same single cell buffer. Modal and
keyboard routing is unchanged from M2.8/M2.9: cells covered by the popup route
to the popup; an outside press closes it according to the existing overlay
rule; Escape closes it; mnemonics and arrow navigation work as for other
menus. Centring only changes the position, not the routing rules. The
MenuBar popups (File, Edit, View, Help) are not centred; they keep their
existing placement under their MenuBar entry.

---

# 30. `New`, `Add` and `Remove`

For Extrude, Revolve, Loft and Sweep:

## New

Creates an independent result.

No `Merge with all` row is required.

## Add

Combines the created result with existing compatible result geometry.

The Action Box exposes:

```text
│ ■ Merge with all            │
```

by default.

## Remove

Subtracts/removes the generated result from existing compatible result geometry.

The same merge-target selection mechanism is available.

---

# 31. `Solid` and `Surface`

The first Action Box selector chooses the output type:

```text
Solid
Surface
```

Every current Create Action Box reference states:

```text
Same for Surfaces
```

This means that the state grammar shown for the operation also applies when
`Surface` is selected unless a future reference explicitly introduces a
Surface-specific state.

It does not mean that the Solid and Surface tabs should be collapsed into one
control.

---

# 32. Progressive disclosure rules

Controls appear because of state dependencies.

Examples:

```text
□ Start Offset
```

contains no child.

```text
■ Start Offset ◀ Blind ▶
└ Dist ...
```

reveals one child.

Likewise:

```text
□ Keep orientation
```

contains no Twist option.

```text
■ Keep orientation
└ □ Twist
```

reveals Twist.

The implementation should represent these relationships as UI state, then
generate the corresponding rows.

Rows MUST NOT simply remain permanently present and be visually disabled unless
the ASCII reference explicitly shows such a state.

---

# 33. Action Box composition example

Suppose Extrude has these states:

- Solid;
- New;
- Extend = Blind;
- Start Offset enabled;
- Start Offset = Blind;
- End Offset disabled;
- Symetric disabled;
- 2nd side enabled;
- 2nd side = Up to;
- nested End Offset enabled;
- nested End Offset = Blind.

The rendered panel must be assembled by substituting the corresponding exact
variants.

For example, the base line:

```text
│ │ □ Starting Offset         │
```

must be replaced by the selected Start Offset branch:

```text
│ │ ■ Start Offset ◀ Blind ▶  │
│ │ ╰╴Dist  [+|0.0 mm       ☰]│
```

and the base second-side line must be replaced by:

```text
│ ╰╴■ 2nd side     ◀ Up to ▶  │
│   │ Target  [Select…       ]│
│   ╰╴■ End Offset ◀ Blind ▶  │
│     ╰╴Dist  [0.0 mm       ☰]│
```

The renderer must preserve:

- exact indentation;
- exact branch characters;
- exact field widths;
- exact spaces around selector values;
- exact unit placement.

---

# 34. Text labels versus semantic identity

Some current references use different labels for the same semantic control.

The implementation should distinguish between:

1. the semantic identity of the control;
2. the exact text currently specified by the visual reference.

Current examples:

| Reference labels | Semantic identity |
|---|---|
| `Starting Offset` / `Start Offset` | extrusion start-offset control |
| `Second end position` / `2nd side` | independent second-side extent |
| `Symetric` | symmetric extent mode |
| `Tengent` | tangent loft boundary type |
| `Profils` | ordered profiles modal |
| `Mesure` | measure command |
| `Part design` / `Part Design` | Part Design workbench |

The README does not silently rewrite those strings inside ASCII references.

If labels are normalized later, the relevant `.txt` file must be edited at the
same time.

---

# 35. Special ASCII palette

`Special_ASCII.txt` contains the current character palette:

```text
─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼ ╭ ╮ ╰ ╯

═ ║ ╔ ╗ ╚ ╝ ╠ ╣ ╦ ╩ ╬

━ ┃ ┏ ┓ ┗ ┛ ┣ ┫ ┳ ┻ ╋

╞ ╡ ╤ ╧ ╪ ╓ ╒ ╖ ╕ ╙ ╘ ╜ ╛ ╟ ╢ ╥ ╧

╴ ╵ ╶ ╷ ┈ ┊ ┄ ┆

░ ▒ ▓ █

□ ■
◀ ▶
▲
┃
▼
☰
```

The palette defines the available visual vocabulary. It is not an exhaustive
list of the characters used by the references: `…` (U+2026), `◐`, `◑`, `╻`, `╹`,
`×`, `+`, `|` and the ASCII letters are used in the `.txt` files without being
listed here, and `▏` (the text-editing caret, §5.11) is listed here for M2.23.
The set of glyphs the renderer must be able to draw (the *required atlas set*)
is defined by `ARCHITECTURE.md` §3.1 (Glyph coverage) as the union of printable
ASCII and every character occurring in `docs/ui_design/*.txt`, plus `▏`. The
atlas holds exactly that set, built once per `k`; any other character is drawn
as `?` and nothing is added at run time. Adding a character to a `.txt` reference
therefore adds it to the required set (and the font-coverage test must pass).

`╞`, `╡` and `╘` of this palette are the hover glyphs of §5.12 (they also occur
in `Interface_Elements.txt` and `Interface_Exemple.txt`, so no atlas change is
needed); `Special_ASCII.txt` lists the exact hover substitutions.

A character does not receive a universal interaction merely because it appears
in this file.

Its meaning depends on its structural context.

---

# 36. Rendering relationship

The UI remains governed by the architecture:

```text
persistent UI state
temporary interaction state
        ↓
layout / composition
        ↓
flat Vec<Cell>
        ↓
newell-render
```

The ASCII Action Box state should be generated from semantic state rather than
stored as a retained widget hierarchy.

For example:

```text
ExtrudeState {
    output_type,
    result_mode,
    extend,
    start_offset,
    end_offset,
    symmetric,
    second_side,
    merge_targets,
    ...
}
```

may determine which exact rows are composed.

The final rendered representation remains a flat grid of cells. Each cell carries
content, a semantic ink role (`Text`, `Contour`, `Action` or the scoped
`Placeholder` of §5.13), logical style and
existing ownership/routing metadata. A full-cell write/copy includes the role;
overlay replacement cannot retain underlying ink. Palette lookup belongs to
`newell-render`, while component semantics assign roles in `newell-ui`. Neither
roles nor Theme/Color become model document data. The grid still renders in one
batched UI draw call with O(1) pointer routing (`ARCHITECTURE.md` §§3.4–3.6).

---

# 37. Hit testing and character ownership

Each interactive section of a composite ASCII line must be identifiable even
when several controls share one row.

Example:

```text
│ Extend   ◀     Blind     ▶  │
```

contains separate interaction regions for:

- `Extend` label: normally non-interactive;
- `◀`: previous state;
- `Blind`: displayed current state;
- `▶`: next state.

Example:

```text
│ │ Depth   [+|0.0 mm       ☰]│
```

contains separate interaction regions for:

- direction prefix `+|`;
- editable value `0.0 mm`;
- formula action `☰`.

The complete text field MUST NOT necessarily be treated as one indivisible
interaction target. For the hover highlight (§5.12) the regions of one
Parameter, Checkbox or Menu row are one visual target: the row's border slots
change whichever of its regions holds the pointer.

This remains subject to the O(1) hit-testing rules in `ARCHITECTURE.md`.

---

# 38. Reference index

| File | Purpose |
|---|---|
| `Interface_Exemple.txt` | complete application-shell composition |
| `Interface_Elements.txt` | reusable fields, modals, progress, profile ordering, Formula editor, Action Box header states and the four hover examples ("Effet Hover", §5.12) |
| `Application_Menu_Bar.txt` | File/Edit/View/Help menu states |
| `Command_Bar.txt` | workbench and CAD command menus |
| `Action_Box_Extrude.txt` | Extrude base panels and every currently defined line/branch variant |
| `Action_Box_Revolve.txt` | Revolve base panels and every currently defined line/branch variant |
| `Action_Box_Loft.txt` | Loft base panels and every currently defined line/branch variant |
| `Action_Box_Sweep.txt` | Sweep base panels and every currently defined line/branch variant |
| `Action_Box_Boolean.txt` | Boolean Add/Cut/Inter panels and line/branch variants |
| `Special_ASCII.txt` | visual character palette and hover substitutions |

---

# 39. Rule for modifying Action Box references

An Action Box `.txt` file is a state specification.

When changing one, the author must preserve the distinction between:

- complete base Action Boxes;
- individual line variants;
- expanded parent-child branches;
- nested variants.

A variant section should include every visual state that the corresponding line
can take.

For example:

```text
"Scale variants"
│ □ Scale                     │

│ ■ Scale     [0.0          ☰]│
```

completely defines both states of the Scale line.

Likewise:

```text
"Connection variants"
│ □ Connection                │

│ ■ Connection                │
│ │ Start     [Select…       ]│
│ ╰╴End       [Select…       ]│
```

defines both the collapsed row and the complete expanded branch.

No implementation should infer undocumented extra rows.

---

# 40. Rule for future variants

When introducing a new state:

1. add the exact ASCII state to the corresponding `.txt`;
2. retain the fixed Action Box width;
3. retain exact spaces;
4. retain exact field widths;
5. use branch glyphs to show ownership;
6. include all alternatives that modify the visible row structure;
7. update this README with the semantic intention;
8. do not silently substitute a visually similar generic widget.

The `.txt` answers:

> **What exact cells are visible in this state?**

This README answers:

> **What does each of those cells and controls mean?**

Both are required to fully specify the Newell interface.


---

# 41. 3D viewport navigation widgets

**Authorized specification amendment, 2026-10-27.** Applies only to the axis
widget and navigation cube under `ARCHITECTURE.md` §3.7. Other UI and all ASCII
references keep their existing cell, palette and interaction rules. This
section replaces the inert NavCube placeholder; historical placeholder frame
and title artwork is not the visual specification for these 3D widgets.
For this removed placeholder only, the legacy §25 fixed-row/reference-snapshot
comparison excludes its old frame/title cells, which become empty ASCII cells;
all other row and cell comparisons are unchanged. The existing four-row Action
Box reservation remains as specified below.

## 41.1 Placement, scale and occlusion

Use one right-aligned tile at the upper-right of the scene interior, inset
8q physical pixels from its top and right, where `q = effective_k / 2` from the
published grid metrics. No extra OS DPI multiplier applies. Gizmo and Cube use
the same 96q square tile, placement and margins. The cube geometry is scaled to
occupy the same nominal screen-space bounds as the existing Gizmo; Gizmo
centers remain unchanged. The 2026-10-28 quick fix enlarges only Gizmo marker
diameters by 15%; cube size retains its previous baseline. They are mutually
exclusive. Round
tile origin to the nearest physical pixel, ties upward; retain subpixel internal
geometry. Tiles have no frames, opaque panels or captions. There are no rotation
arrows or Home icon. Widget orientation follows the scene camera's rotational
basis only; scene projection, pan, zoom and lens shift do not distort shapes.

Clear offscreen color to transparent black, output premultiplied alpha, and
composite with premultiplied-alpha blending over the already rendered scene.
The tile is only a render allocation: never add it to scene occlusion scissors,
fit/contextual bounds exclusions, ASCII painting/clearing or input blockers.
Uncovered pixels and hollow marker interiors show existing scene pixels.
Only actual semantic targets own new input; menus/modals/Action Boxes and
active scene captures retain their existing precedence and composite masks.

Clip placement against actual surface intersected with the scene interior.
Both widgets require at least 112q square available (tile plus both margins).
If the selected widget does not fit, temporarily hide it.
Do not switch representation or shrink labels. Restore it on growth; retain
visibility preference and kind while suppressed. The Action Box rule below
can also suppress the selected tile without changing either preference.
The scene-origin marker, reference axes and reference planes described below
are world-space scene graphics; they are not positioned in this corner tile.

**Human clarification, 2026-10-27:** `View > View Navigator` replaces the former
Navigation cube checkbox. The exact reference states are:

```text
│ □ View Navigator  │

│ ■ View Navigator  │
│ ╰╴   ◀ Gizmo  ▶   │
```

Child alternatives are `◀ Gizmo  ▶` and `◀  Cube  ▶`, with identical indentation
and spacing. The child row exists only when checked: no blank/disabled retained
row when unchecked. A fresh window starts checked with Gizmo selected; state
is window-local, not inherited or stored in the CAD document. Unchecking hides
the navigator and removes the child row; rechecking restores the previous kind.
Arrows wrap between the two kinds in either direction and keep the popup open
and the child row focused. Keyboard Left/Right performs previous/next; existing
cyclic-row semantics consume Enter/Space without cycling. Parent pointer or
Enter/Space toggles visibility, keeps the popup open and focus on the parent.
Rebuild menu height and later-row hit targets immediately when the child is
inserted/removed; if child focus must be remapped on removal, focus the parent.
MenuBar and other item actions remain stable by action identity, not row index.
The branch glyphs are Text (hierarchy guides), arrows Action, word Text and
popup frame Contour; keep existing ASCII hover border substitutions.
Changing visibility or kind cancels pending widget presses/scene gestures,
updates the single reservation, and redraws without moving/fitting the camera,
changing projection, selecting CAD geometry or committing a numeric edit.

Keep the Action Box's existing four-cell top reservation and upward fallback;
removing the placeholder does not reposition CAD controls. After layout, if
an Action Box intersects a tile rectangle, suppress that whole tile for that
layout. Do not move the Action Box. A menu/modal clips widget pixels under its
footprint; all widgets are input-ineligible while any menu/modal is open.
Outside dismissal never also activates a widget. Empty tile pixels pass scene
input and never occlude scene rendering. Target hit ownership disappears when
hidden/suppressed. Home fitting and contextual bounds use only actual UI
occlusions, never the widget tile. The scene viewport rectangle is unchanged.

## 41.2 Axis materials and geometry

**World-origin scene references (focused Executor amendment).** These graphics
remain anchored at `(0,0,0)` and use the main viewport camera, independently of
the corner navigator. Freeze their camera-relative axis length at **96 physical
pixels** (the reference baseline, not a UI-scale multiplier). The neutral matte
#C9CDD2 origin sphere has diameter **3.84 pixels**, half its former 7.68-pixel
baseline; diffuse shading remains 0.80…1.00, with no specular or cast shadow.

Only positive X/Y/Z axes are drawn, from local 0 to +1, using the same shared
red #F52235, green #00B52A and blue #3067F5 constants as the corner Gizmo.
Retain a one-physical-pixel line width. Use screen-space **6-pixel dashes and
4-pixel gaps**, beginning at the projected origin; alpha 0.85. Orbit naturally
foreshortens directions; do not normalize individual projected axis lengths.

The three planes occupy only positive quadrants. For each non-normal coordinate
use the interval **[0.05,0.70]** times the reference axis length: the 5% inset
applies to both bordering axes, including outlines. All three planes and their
outlines use the same neutral gray #C9CDD2; they are not colored by their
normal axes. Fills have alpha 0.10 and borders alpha 0.20; no internal
grid, texture, animation or document-plane meaning. Omit the collapsed outline
of an exactly edge-on plane so it cannot fill an axis's dash gaps. They need not share centers
with the sphere: the assembly's anchor is world origin, not each plane's center.

Derive world units per pixel as `2 * origin_clip_w / (abs(projection_yy) *
viewport_height)`, valid in both supported camera projections; multiply by 96
for the uniform reference scale. Scale only local geometry around world origin,
never the main camera or origin translation. Perspective depth and orthographic
zoom therefore preserve camera-relative scale; pan follows projected world
origin. Reject nonfinite/degenerate transforms and origins with nonpositive
clip W or clip Z outside [0,W]; do not clamp onto screen edges or camera center.
Individual primitives retain main-camera clipping and exposed-UI scissors.
Scene depth determines each fragment's opacity: unobstructed portions keep the
existing opacity, while portions behind model surfaces use 20% of that opacity.
This applies to the origin sphere, dashed axes, plane outlines and translucent
fills in every display mode, including Points, Wireframe, Line and HiddenLine;
surface depth remains available when a mode does not fill model faces.
References sample a separate depth target generated from actual model surfaces,
but never write or modify model depth and do not change model transparency.
Reference uniforms are separate from mesh uniforms. These graphics
never enter document entities/history, picking, Elements or model-fitting
bounds; the existing camera clipping envelope may still encompass the origin.

**Corner Gizmo.** Project its world directions orthographically with the
current camera basis (screen right R, up U, forward F):
`p(v) = center + 31q * (dot(R,v), -dot(U,v))`. Tile center is (48q,48q).
Depth is `dot(F,v)`; smaller values are nearer the viewer. The widget has no
origin sphere or reference planes. Its positive unit X/Y/Z stems and caps use
red #F52235, green #00B52A and blue #3067F5, respectively. Stem width remains
0.8q (minimum 0.8 physical pixel), with spatial alpha `0.35 + 0.50*t`.
Positive discs have radius 8.05q (7q × 1.15) and resting alpha 1, with centered
**white** X/Y/Z letters at unchanged cap height 8q. On hover, exactly that disc
has a fully transparent interior, an axis-colored inside outline of width 1q,
and the same axis-colored letter. Reveal the actual scene through its interior,
not a background-color substitute, stem or rear widget graphics.
Its diameter, center and label placement never change. The outline uses the
existing thin marker stroke width; at rest it blends into the same-color fill.

Negative markers are hollow, unlabeled rings with outer radius 6.44q (80% of
the positive disc radius), inner radius 5.44q and unchanged 1q stroke. Resting
HSV saturation is 80% of the shared axis color, with hue and value unchanged;
resting alpha remains exactly 0.55 regardless of front/rear position. Hover
restores the full shared axis color and alpha 1.0. Ring interiors stay transparent; no negative shafts, size change,
glow or animation. These changes are explicitly authorized by the focused
Executor request of 2026-10-27, and apply only to the corner Gizmo.

World-origin planes, axes and sphere follow the independent contract above,
including main-camera scene-depth rendering. They are not CAD mesh buffers.
Corner caps are composited in center-depth order, ties by signed axis ID;
opaque positive discs mask rear rings. With the smaller negative rings an axial
rear ring can be completely obscured by its front positive disc; do not shift
or enlarge it to compensate. CPU semantic hits use complete outer circles for both discs
and hollow rings; transparent interiors retain marker input ownership. No negative shaft is evaluated, generated or drawn.

## 41.3 Cube shape, materials and labels

Use a convex beveled cube centered at local origin. It is the intersection of
26 half spaces: signed axis components <=0.5 (six), each signed pair sum <=0.90
(twelve), and each signed triple sum <=1.26 (eight). Signs independently range
+/-; pair inequalities use distinct axes. Each supporting polygon is one
clickable face/edge/vertex component. This yields true bevel geometry rather
than painted hit bands. Center the cube in its tile at (48q,48q); set its local
projection scale from the current camera basis so its outer projected bound
matches the preceding Gizmo baseline (7q/5.6q marker radii); the 15%
Gizmo marker enlargement does not resize the cube. Rotation and screen
projection use the same R/U/F convention as the axis. No perspective
distortion inside this widget.

Main faces: opaque #D9DDE2. Edge bevels: #C8CDD4. Vertex bevels: #BEC5CE.
Use fixed screen-space diffuse light direction normalize(-0.4,+0.6,+1) in
(right,up,toward-viewer) coordinates, multiplier 0.88…1.00. Normals may blend
within 0.02 local units of patch seams for a soft bevel appearance; this affects
lighting only, not silhouettes or hit geometry. No outline, gloss or shadow.
Apply one physical pixel of silhouette antialiasing via the dedicated target.
Cube materials are identical under both themes and all model display modes.
Faces remain visible at rest. Edge and vertex patches have zero opacity at
rest; only the patch currently under the pointer restores its existing
material and opacity. Pointer exit restores transparency immediately. Their
full geometric regions remain clickable under the same frontmost-patch and
overlap rules as visible cube geometry; a hidden rear patch cannot take input
through a visible front face. Transparency does not change face labels or hit
geometry.

Primary face labels are exactly Top, Bottom, Front, Back, Left, Right in dark
#252A31, alpha 1, using existing Fira Code Regular (monospaced sans-serif).
Use cap height 7.65q normally (minimum 5.1q only if needed to fit the
face-local ink bounds), a 15% reduction from the previous 9q/6q sizes. Each
word is centered on its own face plane at `0.5 * normal`.
Its stable local up vector is +Z for Front/Back/Left/Right, +Y for Top and -Y
for Bottom; local right is `up × outward_normal`. Map the atlas through this
local rectangle, using **the identical widget projection as cube geometry**.
The cube's current projection remains orthographic: text rotates and naturally
foreshortens with the face; it is never a screen-facing billboard and has no
independent perspective. Keep a 2q-equivalent inset measured in the face plane
(2/64 cube-local units) from the beveled primary-face polygon. Clip ink to that
face's projected polygon. Reject back-facing and edge-on faces before preparing
labels; do not render text through the cube. Face-local orientation never
changes in response to camera heading. Dark #252A31 labels and existing subdued
face/bevel shading remain unchanged.
Use a deterministic separate coverage atlas, built at k changes, for the six
words and X/Y/Z; filtered sampling is permitted only in this widget path.
ASCII glyph atlas/font size/sampling are unchanged. Font licensing remains OFL.

## 41.4 Commands and input

World is right-handed Z-up. Define named views for the first time as Top +Z,
Bottom -Z, Front -Y, Back +Y, Left -X, Right +X, where the vector points from
camera target toward eye. A face click uses that vector; edge/vertex clicks
use the normalized sum of adjacent face normals. Forward points from eye to
target and is the negative of that vector. Up is world +Z projected normal to
forward, except Top uses +Y and Bottom uses -Y. Both poles retain heading +Y
for subsequent orbit departure. The complete 26-direction table is in the
implementation plan; these definitions determine every entry.

Positive/negative axis caps invoke the matching signed principal view (e.g.
+Y is Back, -Y is Front). Origin, shafts and planes have no camera action.
On activation, capture the existing contextual pivot (the scene point picked
at viewport center, otherwise the visible model-bounds center, otherwise the
current camera target) and its projected screen position. Apply the selected
orientation around that pivot while keeping its projected position fixed.
Preserve focal distance, projection mode, FOV, orthographic height and lens
shift, so orientation changes do not fit, recenter or move the model's screen
anchor. If no model is visible, use the existing camera-target fallback.
Commands are instant, with no animation, pole clamp on named views or added
drag control. Existing orbit pole-clamp and Normal to semantics remain
unchanged.

Only an independent primary press/release on the same visible component
activates it. Use existing 4 logical-pixel click/drag slop (multiply by the
window's OS scale only for this existing slop conversion). Crossing it cancels
activation without starting orbit. No secondary/middle/scroll/pinch action begins in
actual widget targets; empty tile pixels remain ordinary scene input. Only the scoped corner-marker hover colors of §41.2 are
authorized; there is no pressed material or animation. Use a hand cursor over
an activatable component, default elsewhere. Existing scene captures keep
ownership when crossing widgets; a widget cannot steal them.

Cube picking uses an orthographic local ray and the same 26 half spaces as
the rendered mesh. The entry support plane identifies the front component;
back components are excluded. At coincident entry planes, use vertex before
edge before face, then lexicographic signed-normal ID (axis order X,Y,Z;
-1 before 0 before +1). Ties within 1e-9 local units use this priority, with
only a 1e-6 physical-pixel numerical tolerance in projection, no hit inflation.
Label pixels belong to their face. Antialiased fringes outside the mathematical
silhouette pass scene input, as does all other empty tile space.

Axis-cap hits use complete circular areas: positive radius 8.05q, negative
radius 6.44q, including every hollow center. Resolve overlaps by the existing
back-to-front cap order in reverse, even for a transparent front marker.
Cap labels share circle ownership; hover never changes ownership or size.
No cap hit through another opaque primitive. Invisible/suppressed widgets have
no hit regions. A fixed six-cap test and bounded 26-component cube test are
permitted by §3.7; ordinary ASCII routing remains direct grid lookup.

On press, freeze component, point, layout generation and camera revision.
Release must have same component and generation/revision, stay within slop and
retain primary ownership. Cancel on Escape, focus loss, pointer exit, resize,
DPI/k transition, occlusion/layout/visibility change, projection/camera command,
or mesh replacement. A cancelled release is consumed. Another button never
activates or releases a widget press. Menus, modals, managed/native resizing,
ASCII controls and existing captured gestures precede widget admission. A
widget press never enters CAD picking, shell command activation or deferred
primary orbit. Numeric editing/focus is preserved; widgets only invoke camera
commands. Per-window visibility, pose and pending press stay isolated.


### 41.5 Corner-marker hover ownership

The per-window Controller derives `Option<AxisCap>` from the existing semantic
navigation hit test. It supplies immutable hover state to the renderer; shaders
only rasterize the requested material and never pick. Only the front visible
marker under the pointer can be hovered. Clear hover immediately on pointer
exit/unknown position, hidden or suppressed navigator, Cube mode, open menu or
modal, UI occlusion, window/ASCII/navigation capture (including pending presses),
focus loss and resize. Re-evaluate against current geometry under a stationary
pointer whenever the camera, layout, visibility, occlusion or capture changes.
Request an event-driven redraw on a changed hover value; add no animation loop.
Hover never changes hit regions, click destinations, camera pose or ASCII hover.
