---
status: current
audience: agent
tags: [adr, render, protocol]
supersedes: []
see-also: []
---

# ADR 0004 — Retained-mode render diff protocol

**Status:** Accepted

Rust emits render diffs (create/update/destroy/overlay).
The renderer consumes diffs; it never inspects StateStore.
Large payloads travel via memory handles, not structured messages.
