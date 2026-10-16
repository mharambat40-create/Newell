# ADR 0004: BrepKIT 4.2.2 pin and Rust 1.88 baseline

**Date:** 2026-10-16  
**Status:** Accepted for M3

## Context

M3 introduces Newell's first geometric-kernel boundary. ADR 0001 selects the
AGPL-licensed BrepKIT line, and `ARCHITECTURE.md` §5.1 requires an exact
version pin recorded in an ADR. The upstream package is split into crates;
there is no cargo package named `brepkit`.

## Decision

`newell-kernel` alone declares these exact direct dependencies:

- `brepkit-operations = "=4.2.2"`
- `brepkit-topology = "=4.2.2"`
- `brepkit-math = "=4.2.2"`

The workspace Rust baseline is Rust 1.88 because BrepKIT 4.2.2 declares that
minimum compiler version. The current development compiler remains newer than
that baseline. No other Newell crate names a BrepKIT package; public kernel
methods expose only `NewellSolid`, `NewellMesh`, and `NewellKernelError`.
The dependency set is AGPL-3.0-only, a compatible and explicitly allowed form
of Newell's AGPL-3.0-or-later project license.

## Consequences

Kernel upgrades require an explicit version change here, execution of the
kernel regression tests and review of every golden-file change. The renderer
receives already-converted Newell mesh buffers, preserving the anti-corruption
layer while allowing M3's camera and picking work.
