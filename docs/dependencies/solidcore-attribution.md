# SolidCore source attribution

Newell's sole B-Rep kernel is SolidCore 1.0.1 from
[the upstream repository](https://github.com/tangency-labs/solidcore), pinned at
`85bf36811a32670e20c0e147c326129213d42cca`. Cargo resolves the local
`vendor/solidcore-1.0.1` path patch for approved Rust 1.88 compatibility
corrections. `vendor/nurbs-grid-mesh-0.1.0` is the accompanying meshing support
package at the same revision. Both packages retain their original Apache-2.0
LICENSE, NOTICE and attribution files and their `NEWELL_CHANGES.md` patch ledgers.

The independent `faer` 0.22.6 sparse rank source remains under its MIT license,
with its approved changes recorded in `vendor/faer-0.22.6/NEWELL_CHANGES.md`.
The root `vendor/manifest.toml` records exact source revisions and maintenance
ownership for the retained copies. Removed dependency packages and their
notices are not distributed in the current tree.
