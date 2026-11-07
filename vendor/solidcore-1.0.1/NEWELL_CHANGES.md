# SolidCore 1.0.1 source ledger

Source: official `tangency-labs/solidcore` commit
`85bf36811a32670e20c0e147c326129213d42cca`, `crates/solidcore`.
The exact upstream package source was copied from the GitHub archive for that
commit. Root LICENSE, NOTICE and ATTRIBUTION.md accompany it. Newell's workspace
excludes this dependency source; its Cargo.toml resolves inherited edition 2021,
Apache-2.0, Rust 1.85, thiserror 2, and upstream lint settings explicitly.
The original release manifest is retained as `Cargo.toml.orig`.

The human approved reapplying previously approved local corrections where still
needed for the 2026-11-06 pin change. The following corrections were ported from
the earlier `vendor/solidcore-0.0.1` ledger without changing the algorithms:

- `SOLIDCORE-MSRV-001`: three owned-error guard patterns in
  `src/blend/fillet_corner/p1.rs`, `src/heal/face_props.rs` and
  `src/surface/volume/p2.rs` inspect a borrow in `matches!` before returning
  the original error. Rust 1.88 otherwise emits six E0382 errors.
- `SOLIDCORE-MSRV-002`: `src/geom/local_solve_nd.rs` removes one overwritten
  initial vector allocation while retaining the assigned solve result.
- `SOLIDCORE-COMPAT-003`: `src/geom/context.rs` replaces deprecated
  `fetch_update` with the same checked, Relaxed `compare_exchange_weak` key
  reservation loop.

On 2026-11-07 the human approved `SOLIDCORE-MSRV-004`: the exact call-site change
in `src/blend/blend_corner/mixed.rs:1426` from `of2m(&mut side_curves, f)` to
`of2m::<[Option<BSplineCurve>; 4]>(&mut side_curves, f)`. This preserves the
selected outer array and side index and makes Rust 1.88 infer the intended
element type. The three-line failing and corrected compiler reproducers and
candidate build results are in Newell's migration validation report.
Maintenance owner: Martin Harambat.
