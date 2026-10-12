# Golden fixtures

Reference 3D models compared against kernel output after each change (`Description.md` §7.2, `ARCHITECTURE.md` §5.1).

- **Naming:** `<feature>_<case>.<ext>`.
- **Invariants:** each fixture has a paired insta snapshot of derived invariants (volume, face/edge/vertex counts, bounding box at fixed rounding) in `crates/newell-kernel/tests/it/snapshots/`.
- **Kernel bumps:** a `brepkit` version bump requires `cargo nextest run -p newell-kernel`. Any snapshot change requires human review.
- **Agents:** do not read fixtures (see `.claudesignore`).
- **Path from kernel tests:** `concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/golden/<file>")`.

No fixture files or kernel tests exist yet (M3.1).
