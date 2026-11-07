# SolidCore production migration

**Approved implementation plan, amended by the human on 2026-11-07.**

## Decision and scope

Use SolidCore 1.0.1 at revision
`85bf36811a32670e20c0e147c326129213d42cca` as Newell's sole B-Rep
kernel. Keep Newell-owned public geometry types and the worker's scratch,
preview, confirmation, Cancel and failure semantics. Retain independent `faer`
rank research. Remove the retired kernel's source, dependency closure, flags,
patches, fallback paths, build wiring, vendor copies and references throughout
the current checkout. Preserve Git history and mandatory attribution for
retained material.

The human explicitly authorized retirement before Boolean provenance and the
solid UpTo matrix pass. Those failures are follow-up regressions on the only
production route, not gates for source cleanup. The separate worker
qualification gate is waived.

## Verification

Build the default app on Rust 1.88. Run default-path model creation, Extrude
preview, confirmation and Cancel smoke checks. Run workspace default and
all-feature checks and relevant geometry tests with snapshot updates disabled.
Inspect both dependency graphs and scan project text and paths, including
untracked files, case-insensitively for the retired name and its spaced and
hyphenated spellings; exclude `.git` and generated artifacts. Report dependency
and name cleanup separately from build and regression status.

## Known follow-up

The two-sided solid Boolean yields a valid body but Newell does not yet rebuild
cap, wall and edge occurrence identity from surviving native attributes. The
solid UpTo modifier matrix fails at extrusion tagging with an unproven source
identity. Split and merged descendants need explicit ambiguity handling before
references can be replayed through later operations.
