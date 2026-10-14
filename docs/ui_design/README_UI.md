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
2. `docs/ui_design/README.md`
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
│ │ └ Dist  [+|0.0 mm       ☰]│
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

---

## 2.5 Variants do not redefine the panel width

For example, these three states all occupy the same Action Box width:

```text
│ │ □ Start Offset            │
```

```text
│ │ ■ Start Offset ◀ Blind ▶  │
│ │ └ Dist  [+|0.0 mm       ☰]│
```

```text
│ │ ■ Start Offset ◀ Up to ▶  │
│ │ └ Target  [Select…       ]│
```

The text field is aligned according to the reference, not according to a generic
form layout.

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
│ └ □ End Offset              │
```

means:

- first `│`: left Action Box border;
- `└`: last child of the current parent branch;
- `□ End Offset`: child control;
- final `│`: right Action Box border.

Example:

```text
│   └ ■ End Offset ◀ Blind ▶  │
│     └ Dist  [0.0 mm       ☰]│
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
│ └ □ End Offset              │
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
│ └ □ End Offset              │
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
│ └ □ End Offset              │
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
│ └ □ Starting Offset         │

│ Extend   ◀     Up to     ▶  │
│ Selection   [Select…       ]│
│ □ Offset                    │

"Start Offset variants"
│ │ □ Start Offset            │

│ │ ■ Start Offset ◀ Blind ▶  │
│ │ └ Dist  [+|0.0 mm       ☰]│

│ │ ■ Start Offset ◀ Up to ▶  │
│ │ └ Target  [Select…       ]│

"End Offset variants 1"
│ │ □ End Offset              │

│ │ ■ End Offset   ◀ Blind ▶  │
│ │ └ Dist  [+|0.0 mm       ☰]│

│ │ ■ End Offset   ◀ Up to ▶  │
│ │ └ Target  [Select…       ]│

"End Offset variants 2"
│   └ □ End Offset            │

│   └ ■ End Offset ◀ Blind ▶  │
│     └ Dist  [0.0 mm       ☰]│

│   └ ■ End Offset ◀ Up to ▶  │
│     └ Target[Select…       ]│

"Symetric variants"
│ □ Symetric                  │
│ □ 2nd side                  │

│ ■ Symetric                  │

"2nd side variants"
│ └ □ 2nd side                │

│ └ ■ 2nd side     ◀ Blind ▶  │
│   └ Depth [+|0.0 mm       ☰]│

│ └ ■ 2nd side     ◀ Up to ▶  │
│   │ Target  [Select…       ]│
│   └ □ End Offset            │

"Merge with all variants"
│ ■ Merge with all            │

│ □ Merge with all            │
│ └ Targets   [Select…       ]│
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

Exact variant:

```text
│ Extend   ◀     Blind     ▶  │
│ │ Depth   [+|0.0 mm       ☰]│
│ │ Along     [Select…       ]│
│ └ □ Starting Offset         │
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
│ Selection   [Select…       ]│
│ □ Offset                    │
```

`Selection` identifies the target entity at which the extrusion terminates.

`Offset` optionally shifts the termination relative to that target.

---

## 8.3 Start Offset

Collapsed:

```text
│ │ □ Start Offset            │
```

Blind:

```text
│ │ ■ Start Offset ◀ Blind ▶  │
│ │ └ Dist  [+|0.0 mm       ☰]│
```

Up to:

```text
│ │ ■ Start Offset ◀ Up to ▶  │
│ │ └ Target  [Select…       ]│
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
│ │ └ Dist  [+|0.0 mm       ☰]│
```

Up to:

```text
│ │ ■ End Offset   ◀ Up to ▶  │
│ │ └ Target  [Select…       ]│
```

This is the End Offset variant used as a direct child of the main extent.

---

## 8.5 End Offset — nested second-side branch

Collapsed:

```text
│   └ □ End Offset            │
```

Blind:

```text
│   └ ■ End Offset ◀ Blind ▶  │
│     └ Dist  [0.0 mm       ☰]│
```

Up to:

```text
│   └ ■ End Offset ◀ Up to ▶  │
│     └ Target[Select…       ]│
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
│ └ □ 2nd side                │
```

Blind:

```text
│ └ ■ 2nd side     ◀ Blind ▶  │
│   └ Depth [+|0.0 mm       ☰]│
```

Up to:

```text
│ └ ■ 2nd side     ◀ Up to ▶  │
│   │ Target  [Select…       ]│
│   └ □ End Offset            │
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
│ └ Targets   [Select…       ]│
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

This is an available boolean direction state.

The current `.txt` does not place this variant in one of the complete Extrude
Action Box states.

Its exact location therefore MUST NOT be invented by an implementation.

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
│ └ □ 2nd side                │

│ □ Full Revolve   ◀ Up to ▶  │
│ │ Target    [Select…       ]│
│ │ □ Start Offset            │
│ │ □ End Offset              │
│ │ □ Symetric                │
│ └ □ 2nd side                │

    
"Start Offset variants"
│ │ □ Start Offset            │

│ │ ■ Start Offset ◀ Blind ▶  │
│ │ └ Angle [+|0.0 deg      ☰]│

│ │ ■ Start Offset ◀ Up to ▶  │
│ │ └ Target  [Select…       ]│

"End Offset variants 1"
│ │ □ End Offset              │

│ │ ■ End Offset   ◀ Blind ▶  │
│ │ └ Angle   [0.0 deg      ☰]│

│ │ ■ End Offset   ◀ Up to ▶  │
│ │ └ Target  [Select…       ]│

"End Offset variants 2"
│   └ □ End Offset            │

│   └ ■ End Offset ◀ Blind ▶  │
│     └ Angle [0.0 deg      ☰]│

│   └ ■ End Offset ◀ Up to ▶  │
│     └ Target[Select…       ]│

"2nd side variants"
│ └ □ 2nd side                │

│ └ ■ 2nd side     ◀ Blind ▶  │
│   └ Angle [+|0.0 deg      ☰]│

│ └ ■ 2nd side     ◀ Up to ▶  │
│   │ Target  [Select…       ]│
│   └ □ End Offset            │

"Symetric variants"
│ │ □ Symetric                │
│ └ □ 2nd side                │

│ └ ■ Symetric                │

"Merge with all variants"
│ ■ Merge with all            │

│ □ Merge with all            │
│ └ Targets   [Select…       ]│
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
│ └ □ 2nd side                │
```

The user defines a signed angular extent.

Up-to partial revolution:

```text
│ □ Full Revolve   ◀ Up to ▶  │
│ │ Target    [Select…       ]│
│ │ □ Start Offset            │
│ │ □ End Offset              │
│ │ □ Symetric                │
│ └ □ 2nd side                │
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
│ └ Magnitude [0.0          ☰]│

│ ■ Start type   ◀ Tengent ▶  │
│ └ Magnitude [0.0          ☰]│

│ ■ Start type   ◀  Match  ▶  │
│ └ Entity    [Select…       ]│

"End Type variants"
│ □ End type                  │

│ ■ End type     ◀ Normal  ▶  │
│ └ Magnitude [0.0          ☰]│

│ ■ End type     ◀ Tengent ▶  │
│ └ Magnitude [0.0          ☰]│

│ ■ End type     ◀  Match  ▶  │
│ └ Entity    [Select…       ]│

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
│ └ End       [Select…       ]│

"Merge with all variants"
│ ■ Merge with all            │

│ □ Merge with all            │
│ └ Targets   [Select…       ]│
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
│ └ Magnitude [0.0          ☰]│
```

Tangent:

```text
│ ■ Start type   ◀ Tengent ▶  │
│ └ Magnitude [0.0          ☰]│
```

Match:

```text
│ ■ Start type   ◀  Match  ▶  │
│ └ Entity    [Select…       ]│
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
│ └ End       [Select…       ]│
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
│ └ □ Twist                   │


"Twist variants"
│ └ □ Twist                   │

│ └ ■ Twist [+|0.0 deg      ☰]│

"Scale variants"
│ □ Scale                     │

│ ■ Scale     [0.0          ☰]│

"Merge with all variants"
│ ■ Merge with all            │

│ □ Merge with all            │
│ └ Targets   [Select…       ]│
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
│ └ □ Twist                   │
```

Enabling `Keep orientation` reveals the optional `Twist` child.

---

## 14.5 Twist

Disabled:

```text
│ └ □ Twist                   │
```

Enabled:

```text
│ └ ■ Twist [+|0.0 deg      ☰]│
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
│     OK     │   Cancel   │
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
│ └ ■ Offset all              │
│   └ Dist  [+|0.0 mm       ☰]│

"Offset All variants"
│ └ ■ Offset all              │
│   └ Dist  [+|0.0 mm       ☰]│

│ └ □ Offset all              │
│   └ Faces   [none          ]│
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
│ └ ■ Offset all              │
│   └ Dist  [+|0.0 mm       ☰]│
```

Offset is a Cut-specific optional operation in the current reference.

Its child is `Offset all`.

---

## 16.8 Offset all

Enabled:

```text
│ └ ■ Offset all              │
│   └ Dist  [+|0.0 mm       ☰]│
```

The distance applies to all applicable faces.

Disabled:

```text
│ └ □ Offset all              │
│   └ Faces   [none          ]│
│     Dist  [+|0.0 mm       ☰]│
```

Disabling `Offset all` reveals an explicit Faces selection.

The following `Dist` line belongs to the same Offset-all branch.

The different indentation between:

```text
│   └ Faces
```

and:

```text
│     Dist
```

MUST be preserved exactly as defined by the reference.

---

# 17. Boolean footer anomaly

The current Add-state reference contains:

```text
│     OK     │   Cancel   │
```

while the other Boolean states contain:

```text
│      OK      │    Cancel    │
```

The first row is shorter than the otherwise 31-cell Boolean frame.

This README deliberately reproduces the source exactly and does not silently
insert spaces.

This should be treated as a current reference inconsistency.

If it is corrected, the correction should be made in
`Action_BoxBoolean.txt` and then reflected here.

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

The second starts with the existing name.

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

---

# 26. Navigation trees

Example:

```text
[-]Features
 ├ □ Sketch_Profile_Ma.
 ├[-] Hub_Features
 │ ├ Extrude_Base
 │ ├ ■ Sketch_Hole
 │ └ Extrude_Cut_Center
 ├ Fillet_Inner
 ├ Chamfer_Edges
 └ Pattern_Circular
```

Tree topology uses:

```text
├
└
│
```

for hierarchy.

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

Checkbox entries control visibility.

`[N]ormal to` contains an explicit keyboard mnemonic.

`Reset Cam` resets the camera.

`Fullscreen` toggles full-screen display.

---

## 28.4 Help

Current entries:

```text
Tutorial
Docs
Shortcuts
Bug
Update
About
```

---

# 29. Command Bar

The Command Bar exposes one Workbench menu and command groups.

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
particular implementation mechanism for multi-character mnemonics.

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
│ │ └ Dist  [+|0.0 mm       ☰]│
```

and the base second-side line must be replaced by:

```text
│ └ ■ 2nd side     ◀ Up to ▶  │
│   │ Target  [Select…       ]│
│   └ ■ End Offset ◀ Blind ▶  │
│     └ Dist  [0.0 mm       ☰]│
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

The palette defines the available visual vocabulary.

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
| `Action_BoxBoolean.txt` | Boolean Add/Cut/Inter panels and line/branch variants |
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
│ └ End       [Select…       ]│
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
