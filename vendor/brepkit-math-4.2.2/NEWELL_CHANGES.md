# brepkit-math-4.2.2 experimental source ledger

Baseline: published Cargo archive; source and SHA-256 in ../manifest.toml.
Original AGPL-3.0-only notices, version, Cargo.toml.orig and all shipped files
retained. No registry edits or default-call algorithm replacement.

## M50-MATH-001 — feature-isolated certified domains (M5 revisions 4–5)

Changed: `Cargo.toml` (non-default newell-m50), `src/nurbs/mod.rs` (conditional
exports). Added `src/nurbs/certified_overlap.rs`,
`src/nurbs/certified_intersection.rs`. All new code is safe Rust, no dependencies.

Exact bounded integer-coordinate affine predicates and rational parameter maps;
conservative positive-weight quadratic/horizontal-segment contact isolation;
original non-unit knot-domain maps for affine spans; exact single-span rational
control-net identity/reversal with positive weights. Root arithmetic uses outward
intervals and an exact witness for the admitted double-root case. Unsupported
curves/ambiguous roots are unresolved, never empty accepted geometry.

No general Bezier-clipping, knot insertion, periodic-seam, differently refined
curved overlap or all-family certification is claimed. Those remain required.
Evidence: kernel curve tests and docs/validation/M5-sketcher.md revision-5 section.

Maintenance owner: unassigned; required before production adoption. Preserve the
original parameter/error and resource contracts when rebasing; rerun the full
advanced-curve corpus, golden geometry and default/candidate parity checks.
