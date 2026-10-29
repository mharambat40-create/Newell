# Controlled M5.0 third-party sources

These are published Cargo archives, not new Newell workspace crates. Original
licenses and unchanged upstream unsafe source are retained under ADR0009.
All new correction/extension code must be safe Rust; no FFI or BLAS.
manifest.toml records upstream archive integrity, local patch IDs, MSRV and
license. Per-package NEWELL_CHANGES.md records exact edits and evidence.
Default API parity is required; candidate APIs use non-default newell-m50.
No production adoption, external publication or automatic upgrades.
Maintenance/advisory/rebase ownership must be assigned before production adoption.
