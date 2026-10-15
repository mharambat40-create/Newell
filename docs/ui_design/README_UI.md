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
3. The `Offset` row of `Up to` has no documented expanded form: toggling it
   changes only `□`/`■`; no child row is composed.

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

is the magnitude entered by the user.

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
font's own patterns at the uniform scale; their coverage blend is part of the
two-colour rule (`ARCHITECTURE.md` §3.6).

---

## 5.10 Interaction state visuals

Newell has no hover state and no hover visual (`ARCHITECTURE.md` §3.5).

Logical focus, selection, pressed, disabled, checked and expanded state exist
for keyboard navigation, activation and dragging. They are drawn **only** by
the ASCII structures documented in this directory:

- `□` / `■` for unchecked / checked and hidden / visible (§5.1, §26);
- `[-]` / `[+]` for expanded / collapsed tree nodes (§26);
- the value text between `◀` and `▶` for cyclic selectors (§5.2);
- the border topology of segmented selectors (§5.3);
- `+|` / `-|` for direction (§5.4);
- the selection-field texts `Select…`, `1 Element`, `N Elements` (§5.5);
- the text-editing caret `▏` while a field is actively edited (§5.11).

A state with no documented structure has no visual. An implementation MUST NOT
add a generic foreground/background inversion, a marker such as `>` or `<`, an
underline, or any third colour or intensity to represent focus, selection or
pressed state. Focus markers are not invented: a focused control without a
documented structure has no visual (the keyboard focus target is observable
only through its effect). The only foreground/background exchange is the global Light/Dark
theme (`ARCHITECTURE.md` §3.6): Light is `#000000` on `#FFFFFF`, Dark is
`#FFFFFF` on `#000000`.

---

## 5.11 Text-editing caret

`▏` (U+258F, LEFT ONE EIGHTH BLOCK, listed in `Special_ASCII.txt`) is the
caret of a text field that is **actively being edited** (`Rename`, `Enter Name`,
the value of a numeric field, the Formula editor text area). It is the only
approved indicator of editing and is specified for M2.23.

- **When.** Drawn only while exactly one field is in edit mode. Not drawn for
  merely focused, selected, hovered (there is no hover) or pressed fields.
- **Colour.** The theme foreground (`#000000` Light, `#FFFFFF` Dark) over the
  background; no third colour, no inversion, **no blinking** (no animation,
  `ARCHITECTURE.md` §3.5). The glyph is an ordinary cell in the flat cell
  buffer; it needs no second draw call and no shader data.
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

Exact reference:

```text
╭─Selection───────────────────╮
│┌───────────────────────────┐│
││ □ Rim_Body               ▲││
││ ■ Hub_Body               ┃││
││ □ Inner_Surface          ▼││
│└───────────────────────────┘│
│─────────────────────────────│
│      OK      │    Cancel    │
╰─────────────────────────────╯
```

Inside this modal:

```text
□
```

means the element is not included in the current selection.

```text
■
```

means the element is included.

This use is contextual and distinct from visibility checkboxes in the document
tree.

---

# 20. Profile Selection box

Exact reference:

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

Each line represents an already selected profile.

Example:

```text
││ ☰ Profil_3             × ╻││
```

means:

- first `│`: outer modal border;
- second `│`: inner list border;
- space: fixed padding;
- `☰`: reorder handle;
- space: fixed separation;
- `Profil_3`: selected profile name;
- spaces: fixed flexible display area;
- `×`: remove profile from the list;
- space: separation before scrollbar;
- `╻`: current scrollbar state;
- `│`: inner list border;
- `│`: outer modal border.

Profile order is part of the data required by operations such as Loft.

---

# 21. Scroll bar states

The current alternatives are:

```text
▲ ▲ ▲ ▲ ▲
┃ ╻      
  ╹ ┃ ╻  
      ╹ ┃
▼ ▼ ▼ ▼ ▼
```

These columns represent different scroll positions.

The drawing is a state reference and MUST NOT be converted into a proportional
scrollbar with different glyph positions without updating the UI specification.

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

The text area contains the expression.

`Fx =` communicates that the entered content is a formula rather than only a
literal number.

The UI reference permits:

- parameter references;
- arithmetic;
- units;
- functions.

The UI specification does not itself define the expression evaluator.

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

Both use the same modal interaction.

The first starts from an empty/default value.

The second starts with the existing name (the full stored name, never the
`…`-truncated text of a tree row, §26).

**Rename trigger (M2.20).** `Rename` opens on a **double-click** of the primary
(left) mouse button on the name text of a user-named tree row (a Feature row or
an Element body row, §26). The category headers (`Features`, `Elements`,
`Solid Bodies(N)`, `Surfaces(N)`, `Construct`), the playhead separator row, the
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

The reference is 82 columns × 28 rows and is preserved unchanged. The minimum
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

**Cursor shape (optional).** The implementation MAY set the OS resize cursor
while the pointer is in a zone. This is an OS cursor shape, not a UI rendering
effect (`ARCHITECTURE.md` §3.5): it stores no UI state, redraws nothing and
changes no cell. Pointer motion without a press still changes no cell buffer.

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

The exact reference is the NavigationPanel of `Interface_Exemple.txt`
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

Features represents the parametric/history structure.

Elements groups resulting entities such as:

```text
Solid Bodies
Surfaces
Construct
```

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
| > 999 | `999fps ` (saturated, see below) |

Consequences: the separators, `Progress [███░ ] 65%` and the right border stay in
the same columns for every value from 0 to 999; no adjacent field shifts when
the digit count changes; at 98 the row equals the reference cell for cell. At
three digits the field touches the left separator, an accepted consequence of
keeping the reference width.

**Value rules** (pure composition function; the input is a measured number, not
a clock read): round to the nearest integer (halves away from zero), a NaN,
infinite or negative input is shown as `0`, and any value above 999 is shown as
`999` (saturation: the field never grows to four digits and never shows a `+`,
`>` or `…`). The digits are always the plain ASCII digits `0`–`9`.

**Other status text.** The segments are composed left to right, joined by
` │ `. The document name and the contextual status are free text: a different
length moves the segments after it, and text that would pass the right border is
clipped whole-cell and never wraps (the §26 ellipsis rule does not apply to the
status bar). Only the frame-rate field is fixed-width, so the frame rate itself
never moves another field. The status bar contains no pointer target whose
position matters to the user; the progress bar is display-only. A snapshot
comparison with `Interface_Exemple.txt` injects the value `98`.

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
│ ◀  Perspective  ▶ │
│ ◀    Shaded     ▶ │
│ UI Scale ◀ 100% ▶ │
│ Theme [ ◐ Light ] │
│ ■ Tree panel      │
│ ■ Task bar        │
│ ■ Status bar      │
│ ■ Navigation cube │
│ □ Grid Overlay    │
│ [N]ormal to       │
│ Reset Cam         │
│ Fullscreen        │
╰───────────────────╯
```

Projection selector states:

```text
◀  Perspective  ▶
◀ Orthographic  ▶
```

Display-style states:

```text
◀    Points     ▶
◀  Wireframe    ▶
◀  Hidden line  ▶
◀  No shading   ▶
◀    Shaded     ▶
◀   Flat line   ▶
```

UI Scale states (`Application_Menu_Bar.txt`):

```text
│ UI Scale ◀  60% ▶ │
│ UI Scale ◀  80% ▶ │
│ UI Scale ◀ 100% ▶ │
│ UI Scale ◀ 120% ▶ │
│ UI Scale ◀ 140% ▶ │
```

`UI Scale` is a cyclic selector (§5.2) over exactly five **preference** values
(60%, 80%, 100%, 120%, 140%). A percentage is not a fixed physical size: the
application derives the integer cell size factor `k` automatically for the
monitor showing the window (`ARCHITECTURE.md` §3.1.1). On the reference display
the values are 60% = 12×27 px, 80% = 16×36, 100% = 20×45 (default),
120% = 24×54, 140% = 28×63. On another monitor the same percentage can give a
smaller cell: 100% is 20×45 on the MacBook Air Retina display and 8×18 (`k = 2`,
one step smaller) on a Full HD 1920×1080 monitor.

- The row **always shows the stored preference**, never the derived `k`: it reads
  `100%` on both monitors, and moving the window between monitors never changes
  the text of the row or the stored preference. The adaptation shows no dialog
  and no notice.
- `◀` selects the previous and `▶` the next of the five values, wrapping like
  every other cyclic selector, and stores it as the new preference. **No value is
  skipped or hidden**, even when the current monitor clamps several of them to
  the same cell size: on a 1920×1080 monitor 60/80/100/120% all give 8×18 px and
  140% gives 12×27, so most steps change the preference and nothing visible. This is intended
  (`ARCHITECTURE.md` §3.1.1, *Several percentages may share one `k`*).
- `100%` is the default preference, and the menu opens in Light theme by default.
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
other one. Light is black on white; Dark is white on black
(`ARCHITECTURE.md` §3.6).

Checkbox entries control visibility.

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

The Command Bar exposes one Workbench menu, one Sketch menu and command groups.

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

Sketch (`Command_Bar.txt`):

```text
╭─Sketch─────────╮
│ [N]ew Sketch   │
╰────────────────╯
```

`[N]ew Sketch` is UI-only and mock-only like every command in M2: it performs
only the existing mock action (closing the popup and any mock state change the
plan defines). It creates no sketch, no geometry and no feature, and calls
neither the Model nor BrepKIT. Sketch editing is a later milestone.

Current command groups:

| Group | Commands |
|---|---|
| Create | `[E]xtrude`, `[R]evolve`, `[L]oft`, `[S]weep` |
| Operate | `[B]oolean`, `[S]plit [F]ace`, `[S]plit [B]ody`, `[T]ransform` |
| Pattern | `[Mi]rror`, `[Li]near`, `[Ci]rcular`, `[Cu]rve` |
| Refine | `[F]illet`, `[C]hamfer`, `[S]hell`, `[D]raft` |
| Construct | `[Pl]ane`, `[A]xis`, `[P]oint` |
| Inspect | `[M]esure`, `[S]plit [V]iew`, `Curvature`, `Properties` |

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

The CommandBar popups (Workbench, Sketch, Create, Operate, Pattern, Refine,
Construct, Inspect) are centred in the grid, not anchored under their header.
For a popup of `popup_width × popup_height` cells (frame included) in a grid of
`grid_cols × grid_rows` cells:

```text
col = floor((grid_cols − popup_width) / 2)
row = floor((grid_rows − popup_height) / 2)
```

with integer arithmetic on non-negative values: if a difference is negative the
coordinate is 0 (the popup is clipped at the right/bottom grid edge, never at
the left/top edge and never wrapped). The popup size is that of its `.txt`
reference (for example Workbench 18 × 11, Sketch 18 × 3), unchanged by
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

The final rendered representation remains a flat grid of cells.

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
interaction target.

This remains subject to the O(1) hit-testing rules in `ARCHITECTURE.md`.

---

# 38. Reference index

| File | Purpose |
|---|---|
| `Interface_Exemple.txt` | complete application-shell composition |
| `Interface_Elements.txt` | reusable fields, modals, progress, profile ordering, Formula editor and Action Box header states |
| `Application_Menu_Bar.txt` | File/Edit/View/Help menu states |
| `Command_Bar.txt` | workbench and CAD command menus |
| `Action_Box_Extrude.txt` | Extrude base panels and every currently defined line/branch variant |
| `Action_Box_Revolve.txt` | Revolve base panels and every currently defined line/branch variant |
| `Action_Box_Loft.txt` | Loft base panels and every currently defined line/branch variant |
| `Action_Box_Sweep.txt` | Sweep base panels and every currently defined line/branch variant |
| `Action_Box_Boolean.txt` | Boolean Add/Cut/Inter panels and line/branch variants |
| `Special_ASCII.txt` | visual character palette |

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
