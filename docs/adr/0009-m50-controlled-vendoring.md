# ADR 0009: Controlled M5.0 dependency sources

Date: 2026-10-29. Status: Accepted for experimental M5.0 only, by the human's
explicit approval of M5 revision 4 and request to apply V0, then execute V1–V4.
Role/provider for V0: Architect / Codex.

## Decision

Permit the published Cargo baselines faer 0.22.6 (MIT), brepkit-math 4.2.2 and
brepkit-operations 4.2.2 (AGPL-3.0-only) under their versioned vendor directories.
Keep topology and all other dependencies on the current resolution unless
separately approved. Record source integrity, patch IDs, licenses, tests and
maintenance obligations in vendor/manifest.toml and per-package ledgers.
Root Cargo path patches resolve these candidates; no BrepKIT dependency is
introduced outside newell-kernel or in workspace.dependencies. Exclude vendor
packages from the five-crate Newell workspace.

This supersedes ADR 0004's registry source selection only for those local
experimental math/operations sources. Exact =4.2.2 pins, Rust 1.88 baseline,
ACL isolation, error translation, original licensing and golden regressions
remain required. Preserve ADR 0004 verbatim as history.

Architecture §§5.2/6 distinguish third-party source from Newell-authored code:
retain upstream headers/license metadata and unchanged upstream unsafe; all new
extension/correction code is safe Rust. No new unsafe, FFI, native library/BLAS,
C/Fortran build, alternate geometric kernel or weakening of audit policy.

Use additive non-default newell-m50 APIs and m50-feasibility wiring. Default
application behavior must remain equivalent to the published baseline. Changes
may be built and qualified by the explicitly requested Executor stage; this is
not production adoption. Accepted regular rank, singular/uncertain semantics,
certified curve/error/topology cases and original timing/workspace budgets are
not reduced. Failed qualification keeps M5.0 open.

## Consequences

The local copies incur provenance, advisory, patch/rebase and supported-platform
maintenance. Their owner must be assigned before any production adoption.
No upstream issue/PR/publication, commit, new checkout, M5.1 or runtime switching
is automatic. Production adoption requires its concrete qualification dossier
and a separate explicit human request. Full scope is M5 plan revision 4 §9.5.
