# M3 camera navigation — superseded

The fixed-target quaternion-trackball implementation documented here on 2026-10-17
is superseded by the approved [M3 camera mechanics plan](M3-camera-mechanics.md).

The current implementation uses Controller-owned screen-space yaw/pitch orbit with
strict zero roll and a frozen dynamic pivot, context-depth pan, and cursor-surface-
anchored zoom. The View consumes immutable camera matrices. See `ARCHITECTURE.md`
§4.4, the mechanics plan's implementation notes, and
[2026-10-19 validation](../validation/M3-mechanics-2026-10-19.md).

The [2026-10-17 validation record](../validation/M3-navigation-2026-10-17.md)
remains historical evidence for the previous implementation.
