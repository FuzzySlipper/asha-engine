---
status: current
audience: agent
tags: [nav, pathfinding, spatial, authority]
supersedes: []
see-also: [runtime-session-facade.md, gameplay-declared-reads.md]
---

# Navigation and Pathfinding

Navigation projection and path queries are Rust-owned through `svc-pathfinding`. The RuntimeSession exposes read-only nav views and typed path queries.

## Nav Pathfinding Substrate

Read-only voxel navigation projection and deterministic path query evidence. See `docs/nav-pathfinding-substrate.md`.

## Nav Runtime Readout

`readNavProjection` and `queryNavPath` expose generated-tunnel nav availability, hashes, and reachable/no-path readouts. `readNavPolicyView` returns a read-only/proposal-only policy-facing nav view. See `docs/nav-runtime-readout.md`.
