# ADR 0010: SolidCore as Newell's sole B-Rep kernel

**Status:** Accepted; production source cutover executed 2026-11-07.

## Decision

`newell-kernel` is the only Newell crate allowed to depend on a B-Rep kernel.
Its sole kernel is SolidCore 1.0.1 at Git revision
`85bf36811a32670e20c0e147c326129213d42cca`. The root manifest patches
that revision to the controlled vendored copy for approved Rust 1.88
compatibility corrections. The vendor ledger identifies those patches and
retains upstream Apache-2.0 license, NOTICE and attribution.

Newell-owned geometry inputs, topology references, meshes and errors cross the
crate boundary. Native handles remain private. Boolean split, merge and removal
must preserve provable identities or reject ambiguous references. No fallback
kernel or automatic comparison path is permitted.

The human authorized source and dependency retirement before all operation
regressions pass. Failed checks remain visible and are repaired on this route.
The separate worker qualification gate was waived. Rust 1.88 and application
preview, confirmation, Cancel and error recovery remain verification targets.

ADRs 0001 and 0004 are historical records for project licensing and compiler
baseline; their former source selection is superseded. Independent `faer` rank
research is retained without adding another B-Rep kernel.
