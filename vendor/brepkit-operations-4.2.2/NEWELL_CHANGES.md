# brepkit-operations-4.2.2 experimental source ledger

Baseline: published Cargo archive; source and SHA-256 in ../manifest.toml.
Original AGPL-3.0-only notices, version, Cargo.toml.orig and all shipped files
retained. No registry edits, offset default changes or production adoption.

## M50-OPERATIONS-001 — analytic circle offset candidate (M5 revision 4)

Changed: `Cargo.toml` (non-default newell-m50 forwards math feature),
`src/lib.rs` (conditional export). Added `src/certified_offset.rs`.
All extension code is safe Rust; no new dependency, unsafe or FFI.

An analytic full-circle offset retains its frame and encloses r+distance, with
explicit invalid/cancel/resource/unresolved outcomes. The kernel candidate test
constructs native exact topology and extrudes it. General conic/spline offset,
joins, self-intersection/topology, automatic arrangements and complete M4 replay
qualification remain open. This API does not replace offset_wire/offset_trim.

Evidence and limits: docs/validation/M5-sketcher.md revision-5 section.
Maintenance owner: unassigned; required before production adoption. Rebases must
retain original error/topology/parameter semantics and pass default parity,
curve/region/golden regressions, Rust 1.88 and license/advisory checks.
