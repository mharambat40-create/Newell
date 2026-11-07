# SolidCore production migration — validation record

## Source and authority

Executor cutover in the existing `main` checkout, starting at
`29321e9efc1a18753e18a081de598b5ac2f9f70b`. The human authorized the
source and dependency removal before remaining operation regressions pass, and
waived the separate worker qualification gate. The pinned SolidCore revision is
`85bf36811a32670e20c0e147c326129213d42cca`; the root path patch uses its
controlled Rust 1.88 compatibility copy. No commits or publication were made.

The pre-cutover baseline was 1177 workspace tests passing with 23 skipped, plus
default and all-feature Rust 1.88 checks, formatting, Clippy and app build.
Those results describe the pre-cutover route and are not a claim about this
cutover. Earlier SolidCore candidate work exercised 82 private tests and the
feature-selected worker's model creation and Extrude preview, confirmation and
Cancel flows. The surface UpTo modifier matrix accepted 105 cases on that
feature-selected route. These are retained as historical evidence, not as a
claim that all current checks pass.

## Cutover implementation

`NewellGeometry` now names the SolidCore owner in the default worker. The
standalone box wrapper uses the same owner. The prior geometry backend, source
modules, dependency declarations, root patches and vendored packages were
removed. Newell-owned request, reference, mesh and result types remain in
`newell-kernel`. The controlled `faer` source and rank research remain.

## Checks and open regressions

The Rust 1.88 default application build passed. The default worker tests for
model creation and a named local extrusion, and for formula-backed Extrude
preview, confirmation and Cancel, passed. The solid UpTo modifier matrix failed
with `extrusion source identity is unproven` after a two-sided Boolean. The
focused two-sided fixture shows that the native result retains six distinct
source face tags and twelve distinct source edge tags, but Newell does not yet
reconstruct stable roles and occurrences from them. No downstream-reference
success is claimed. Other broad regression and lifecycle cases remain to be
measured on the default route.

The default and all-feature dependency graphs resolve SolidCore 1.0.1 and no
other B-Rep kernel. The case-insensitive current-tree scan, including untracked
text and paths and excluding `.git` and generated build artifacts, returned
zero matches for the retired name, its spaced spelling and its hyphenated
spelling. No mandatory-attribution exception was required: removed dependency
material was removed with its notices; retained third-party notices remain.

## Final check detail

`cargo +1.88.0 build -p newell-app --locked` and
`cargo +1.88.0 check --workspace --all-features --locked` passed.
`cargo +1.88.0 test --workspace --all-features --no-run --locked` passed.
`cargo fmt --all -- --check`, stable-toolchain workspace/all-target/all-feature
Clippy with `-D warnings`, `git diff --check` and `cargo deny check` passed.
Clippy is not installed in the local Rust 1.88 toolchain, so that command was
run with the installed stable toolchain. `cargo deny` reported only duplicate
version and unused license-allowance warnings.

The default-path kernel box/mesh and invalid-input integration tests both
passed after translating nonfinite and nonpositive dimensions at the Newell
boundary. A fail-fast default workspace nextest run started 1089 tests; 39
passed and one failed before it stopped. The failing app test was
`nested_second_offset_is_separate_and_hidden_values_survive_branches`, where
an expected value was absent at `crates/newell-app/src/app.rs:5867`.
The focused solid UpTo modifier test still failed with `extrusion source
identity is unproven`. Its Boolean provenance fix and a complete non-fail-fast
workspace run remain outstanding.
