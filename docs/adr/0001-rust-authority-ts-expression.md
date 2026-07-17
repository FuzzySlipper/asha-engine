---
status: current
audience: agent
tags: [adr, authority, expression]
supersedes: []
see-also: []
---

# ADR 0001 — Rust authority, TypeScript expression

**Status:** Accepted

Rust owns canonical state, validation, event application, replay, and simulation.
TypeScript proposes commands; Rust validates and applies them.
TypeScript never mutates authoritative state.
