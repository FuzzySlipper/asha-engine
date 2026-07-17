---
status: current
audience: agent
tags: [adr, replay, boundary]
supersedes: []
see-also: []
---

# ADR 0002 — Replay boundary

**Status:** Accepted

Replay records accepted domain events plus state hashes.
The canonical replay target is WASM semantics.
Native builds are for tooling and fast iteration only.
