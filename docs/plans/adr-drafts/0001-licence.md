# ADR 0001: Licence strategy for BrepKIT

Status: Accepted (human decision, 2026-10-08). Draft staged here because `docs/adr/` is human-owned and write-protected for agents; move it to `docs/adr/0001-licence.md` on merge.

## Context

Description.md §9: BrepKIT is licensed AGPL-3.0-only or under a commercial licence. It is the mandatory, exclusive geometry kernel (§3.2), and replacing it is not an allowed mitigation. The decision must be made in M0, before any BrepKIT-dependent production code is merged into `main` or any binary is distributed.

## Options

1. **Use BrepKIT under an AGPL-compatible strategy.** Newell is distributed under an AGPL-compatible licence, and its source is available to users, including users of a network service.
2. **Buy a BrepKIT commercial licence.** Newell's licence is then not constrained by AGPL.

## Decision

Option 1: use BrepKIT under an AGPL-compatible licensing strategy.

## Consequences

- Newell's distributed source must be available under an AGPL-compatible licence. The exact Newell licence (for example AGPL-3.0-only or AGPL-3.0-or-later) is still to be chosen and recorded in `LICENSE` and a follow-up ADR.
- `deny.toml` (`cargo-deny`) must allow AGPL-3.0 for the BrepKIT dependency. Other licences stay under the allowlist.
- Network use of Newell triggers the AGPL source obligation. Any future hosted or web feature must be re-checked against this ADR.
- Commercial licensing remains available later. Switching to it would be a new ADR.
- M0 spikes may continue locally; no spike output is merged until this ADR is accepted.

## Open points

- Choose the exact Newell licence version (see Consequences).
- Confirm BrepKIT's current licence text and version to pin, from its repository, before the first merge.
