# Newell UI/UX Specification

**Status:** Canonical UI/UX specification for Milestone 2 and later UI work.
**Subordinate to:** `ARCHITECTURE.md`

This document defines the shared UI semantics used by the ASCII reference
states in this directory. It organises those references; it does not add CAD
functionality. Where it is silent, the reference files and the implementation
plan in `docs/plans/` apply, within the limits below.

---

## 1. Source Precedence

1. `ARCHITECTURE.md`
2. `docs/ui_design/README.md` (this file)
3. Relevant ASCII reference files in `docs/ui_design/`
4. Implementation plans in `docs/plans/`

ASCII reference files illustrate UI states. They do not override this README.
If a reference conflicts with this README, this README wins. If this README
conflicts with `ARCHITECTURE.md`, `ARCHITECTURE.md` wins.

Shared component semantics are defined here once and must not be redefined
inside individual reference files or plans.

---

## 2. Core UI Principles

- **Grid-native UI.** Every visible element occupies whole terminal-style cells.
  Layout, borders, selectors and text are all expressed in cell coordinates.
- **Whole-cell layout.** Sizes and positions are integer cell counts. No
  sub-cell placement is part of the UI model.
- **3:5 logical cell ratio.** The base cell is assumed to be 3:5
  (width : height), as stated in `ARCHITECTURE.md` §3.1.
- **Text-first interface.** Meaning is carried by text and by box-drawing
  structure. Icons and glyph symbols (`□ ■ ◀ ▶ ☰`) reinforce text; they do not
  replace it.
- **Colour is reinforcement, not the sole state signal.** Every state that
  colour can show (selected, checked, hovered, focused, disabled) must also be
  visible in text or structure, so the interface remains readable without
  colour.
- **High-level UI state lives in plain structs and enums.** Command state,
  focus, selection, tree expansion and panel visibility are ordinary Newell-owned
  data. There is no retained DOM or widget object tree.
- **Rendered UI is a flat 1D `Cell` buffer.** The buffer is what `newell-render`
  consumes. It is not the entirety of UI state (see §9).
- **O(1) cell-based hit-testing is the target.** Pointer position is converted
  to a cell by division, and the cell's `widget_id` identifies the target.
  Bounding-box iteration is not part of normal hit-testing.
- **Hover feedback is shader-side.** The CPU records hover as a simple state
  value; the visual effect is produced by the fragment shader, as required by
  `ARCHITECTURE.md` §3.5.

---

## 3. Canonical Application Regions

These names are canonical. Reference files may use older labels for the same
regions; where they do, the names below apply.

| Region          | Purpose                                                                 |
|-----------------|-------------------------------------------------------------------------|
| `MenuBar`       | Application menus: File, Edit, View, Help.                              |
| `CommandBar`    | Workbench and command groups: Workbench, Sketch, Create, Operate, Pattern, Refine, Construct, Inspect. |
| `NavigationPanel` | Trees of the document: Features and Elements (§4).                    |
| `Viewport3D`    | The 3D view. In M2 this is the existing clear-colour viewport plus a static NavCube placeholder. |
| `ActionPanel`   | Contextual controls for the active command (§5).                        |
| `StatusBar`     | Document name, workbench name, status text, frame rate and progress, as shown in `Interface_Exemple.txt`. |

---

## 4. NavigationPanel

The NavigationPanel contains two trees:

- **Features tree.** Lists the features of the document as a hierarchy, with
  expand/collapse markers (`[-]`, `[+]`) and nested rows.
- **Elements tree.** Lists the resulting entities (for example solid bodies,
  surfaces and construction items), grouped with counts such as `Solid Bodies(2)`.

These are different concepts. They share the tree-row primitive (M2.7) and the
panel container, but they are different data sources with different meanings.
Neither tree may be treated as a view of the other.

In M2 both trees are populated from deterministic mock data only.

---

## 5. ActionPanel

The ActionPanel shows the controls for the command currently in progress. It is
driven by command state, not by a fixed property layout.

- **Contextual command state.** The panel's title is the command name (for
  example `Extrude`). Its content is determined by the current command state.
- **Progressive disclosure.** Rows appear and disappear according to command
  state. A row that does not apply to the current choice is absent, not greyed
  out. For example, `Depth` appears only when the end condition is `Blind`, and
  `Selection` appears only when it is `Up to`.
- **Stable header.** The top segmented selectors (§6) are shown in every state
  of the panel.
- **Not a generic property inspector.** The ActionPanel is not a retained list of
  every property of the selected object. It shows only the controls the active
  command needs right now.

---

## 6. Selector Semantics

Two selector types exist. They are chosen by how the user should perceive the
choice, not by how many options there are.

### 6.1 Segmented selectors

Use a segmented selector when the alternatives should remain **visible
simultaneously**. The active segment is shown structurally, through the box
edges around it.

Canonical examples:

```text
Solid | Surface
New | Add | Remove
```

### 6.2 Cyclic selectors

Use a cyclic selector when **one ordered value is shown at a time**. The
`◀` and `▶` markers step backwards and forwards through the values.

Canonical example:

```text
◀      Blind      ▶
```

with values such as:

```text
Blind
Up to
Through all
```

### 6.3 The number of choices does not decide the type

`Solid | Surface` has two choices and is a segmented selector. The View menu's
`◀ Perspective ▶` has two choices and is a cyclic selector. `◀ Blind ▶` has
three choices and is also cyclic. The type follows the presentation intent
above, not the count.

---

## 7. Other Controls

### 7.1 Checkbox / boolean state

A boolean control shows its state with a glyph and its label:

```text
□ Invert      unchecked
■ Invert      checked
```

Checked and unchecked states are distinguished by the glyph, not only by colour.

### 7.2 Numeric fields with engineering units

A numeric field shows a value with its unit in the field text, followed by a
`☰` button that opens the FormulaEditor (§7.5):

```text
Depth       [0.0 mm   ☰]
Angle       [0.0 deg  ☰]
```

In M2 a field expresses UI behaviour only. No expression is evaluated, and the
value is deterministic mock data.

### 7.3 Selection fields

A selection field shows the current selection as text. The text states the
count, not the names:

```text
Selection   [none      ]
Selection   [1 Element ]
Selection   [N Elements]
```

The reference pairs the selection field with the SelectionPanel overlay (§7.4)
for choosing items. In M2 no real geometry selection occurs.

### 7.4 Popup menus and the SelectionPanel

Popup menus are the drop-down lists opened from the MenuBar and from command
groups in the CommandBar. Menu items may contain selectors and checkboxes, as
the View menu does.

The SelectionPanel is a modal overlay with a scrollable list of items. Each item
is a checkbox row, and a scroll indicator (`▲ ┃ ▼`) shows the position within the
list. It ends with OK and Cancel.

### 7.5 FormulaEditor

The FormulaEditor is a modal overlay with a single editable text area and OK and
Cancel. It is opened from a field's `☰` button. In M2 it accepts text and
restores the prior state on Cancel. It does not evaluate formulas.

### 7.6 Modal confirmation boxes

Short modal boxes such as `Enter Name` or `Rename` contain one text field and
OK and Cancel. They use the same modal container as §7.4 and §7.5.

### 7.7 Progress indication

Progress is shown as a bar with a percentage, for example
`Progress [███░ ] 65%`, in the StatusBar.

### 7.8 Progressive disclosure

Progressive disclosure is the general rule behind the ActionPanel (§5). A
control reveals the rows that depend on it. Examples from the Extrude reference:

- choosing `Blind` reveals `Depth`;
- choosing `Up to` reveals `Selection`;
- an unchecked `Merge with all` reveals `Merge scope`;
- the merge controls appear only when the result mode is `Add` or `Remove`.

---

## 8. Reference M2 Extrude UI

M2 uses **deterministic mock UI state only**. The Extrude reference is a UI
mock. It is not a working command.

The reference shows these controls, in this order:

- `Solid | Surface` (segmented);
- `New | Add | Remove` (segmented);
- `Selection` (selection field);
- `◀ Blind | Up to | Through all ▶` (cyclic);
- `Depth` (numeric field with `mm`);
- `Invert` (checkbox);
- `Starting Offset` (checkbox);
- `Symmetric` (checkbox);
- `Second end position` (checkbox);
- merge controls where relevant: `Merge with all` (checkbox) and `Merge scope`
  (selection field);
- `OK` and `Cancel`.

OK and Cancel change only the mock UI state. They do not execute an operation.

No geometry, BrepKIT behaviour, model recomputation or real selection belongs
to M2. The spelling `Symmetric` is used here. Some reference files spell it
`Symetric`; see §12.

---

## 9. Rendering Relationship

UI state reaches the screen through this pipeline:

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

- **Persistent UI state** is the document-level and panel-level data described
  in §2 and §5: active command, selector values, expanded tree nodes, open
  panels.
- **Temporary interaction state** is the transient data: hover, pressed, focus
  and the in-progress edit of a field.
- **Layout and composition** are pure functions that regenerate the logical UI
  from both kinds of state.
- **The flat `Vec<Cell>`** is the rendered representation. It is consumed by
  `newell-render` in a single UI draw call (`ARCHITECTURE.md` §3.4).

The Cell buffer is not the whole UI or controller state. Persistent and
temporary state live outside it, and the buffer is regenerated from them.

---

## 10. Borders and Box-Drawing Glyphs

Borders are ordinary Unicode box-drawing cells. The accepted rules are
documented in `crates/newell-ui/src/layout/border.rs`:

- borders are logical Unicode box-drawing cells;
- mixed Single/Double junctions promote the whole junction to Double;
- merging of glyph topology is deterministic and order-independent;
- for `StyleId`, the last border draw to touch a cell supplies the final style;
- a rectangle narrower or shorter than two cells draws no border.

`Special_ASCII.txt` lists the characters the references use. It is a palette of
what the references show, not a list of what `draw_border` supports. The M2.2
border vocabulary covers the single and double families only (see §12).

---

## 11. Logo Policy

No logo atlas is currently required for the M2 reference UI.

- Do not create a placeholder PNG atlas.
- Do not generate a logo atlas at runtime.
- Do not add a PNG dependency solely for placeholders.

Future logos must follow `ARCHITECTURE.md` §3.2 and §3.3: a single RGBA POT PNG
atlas, 256×256 slices with a 240×240 visible logo, and the 5×3 and 3×2 cell
allocations.

---

## 12. Reference Files and Open Points

### 12.1 Reference index

| File | Illustrates |
|------|-------------|
| `Interface_Exemple.txt` | Full shell: MenuBar, CommandBar, NavigationPanel, Viewport3D with NavCube, ActionPanel, StatusBar. |
| `Application_Menu_Bar.txt` | MenuBar drop-downs and the View menu's cyclic and checkbox rows. |
| `Command_Bar.txt` | CommandBar groups and their command entries. |
| `Action_Box_Extrude.txt` | Extrude ActionPanel states, including progressive disclosure. This is the M2.11 reference. |
| `Action_Box_Revolve.txt` | Revolve ActionPanel states. Reference only; Revolve is outside the M2 mock. |
| `Interface_Elements.txt` | Shared elements: progress bar, selection text, SelectionPanel, header variants, FormulaEditor, confirmation boxes. |
| `Special_ASCII.txt` | The box-drawing and symbol palette used by the references. |

### 12.2 Open reference points

These are not decided by this README. They are recorded so later work resolves
them deliberately.

- **Spelling.** The references write `Symetric` and `Revovle`. This README uses
  `Symmetric`, as in the milestone scope. The reference files have not been
  edited.
- **Legacy labels.** `Application_Menu_Bar.txt` labels some View entries with
  older region names. §3 defines the canonical names.
- **Fragments without context.** `Direction [Normal to]` and an `Offset`
  checkbox appear in the Extrude reference without full context. Neither is in
  the M2.11 list, so neither is part of the M2 mock.
- **Tree-row markers.** Tree rows carry `□` / `■` markers. Their meaning
  (visibility, selection or something else) is not specified.
- **Rounded corners.** ActionPanel and popup references use `╭ ╮ ╰ ╯`. These
  are in `Special_ASCII.txt` but are not in the M2.2 border vocabulary. The
  decision belongs to the widget milestones that first need them.
- **Heavy lines.** The SelectionPanel scrollbar uses `┃`. Heavy lines are not in
  the M2.2 border vocabulary.
- **Active-segment rule.** The references show the active segment through box
  edges. The exact glyph rule is derived when the segmented-selector primitive is
  implemented (M2.7).
