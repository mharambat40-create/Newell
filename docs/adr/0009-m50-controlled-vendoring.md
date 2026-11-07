# ADR 0009: Independent M5 rank source

**Status:** Historical experiment; current source disposition below.

M5.0 investigated sparse rank and geometry feasibility using controlled
third-party source. The original geometry-specific vendor candidates were
retired during the SolidCore production cutover. Their historical patch and
test evidence remains in Git history. This ADR no longer authorizes a second
kernel or its vendor copies in the current tree.

The independent `faer` 0.22.6 rank source remains at
`vendor/faer-0.22.6` under MIT. Its patch ledger and `vendor/manifest.toml`
record the approved Rust 1.88 corrections and maintenance owner. The
`rank-research` feature keeps the rank adapter isolated from default modeling.
Newell-authored code still forbids `unsafe`, FFI and native BLAS integration.

ADR 0010 controls the sole B-Rep kernel, source pin, vendor attribution and
current production boundary.
