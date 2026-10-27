# Desktop working-folder cleanup - 2026-10-26

The human requested consolidation into the existing Newell checkout and removal
of the three sibling working folders from the Desktop. No project code or current
specification was replaced.

Recovered documents:

- [Dark startup proposal](dark-theme-startup-alignment-2026-10-22.md): the missing
  original plan, superseded by accepted ADR 0005 and later decisions.
- [M4 roadmap](M4-cad-command-parametric-roadmap-2026-10-20.md): the missing
  revision-2 proposal; retained as context for later milestones, not approval.
- [Earlier interface annotations](Interface_Elements-before-merge-2026-10-26.txt):
  unique pre-merge scrollbar/tree reference text, retained without overwriting the
  current UI reference.

The original document bytes are preserved after each recovery notice. Old paths,
source snapshots and approval gates inside those historical texts do not apply to
current work. Current architecture, specifications and approved milestone plans
remain authoritative.

The old draft ADR 0005 was superseded by the accepted copy already in docs/adr.
Other saved UI references were already present in Newell, with only line-ending
or later intentional reference differences. Both retired worktree branches had
no commits absent from main. Their branch references are retained. The three
Desktop folders are moved to the user's Trash for recoverability, and the two
obsolete Desktop worktree registrations are removed. Main and its Git index are
preserved; no commit, merge or push is performed.
