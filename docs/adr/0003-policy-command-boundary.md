---
status: current
audience: agent
tags: [adr, policy, commands]
supersedes: []
see-also: []
---

# ADR 0003 — Policy command boundary

**Status:** Accepted

TypeScript policy receives read-only generated views and returns proposed commands.
It may not observe authoritative state directly or validate commands.
Rust is the sole validator.
