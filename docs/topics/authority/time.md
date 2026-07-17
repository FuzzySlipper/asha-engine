---
status: current
audience: agent
tags: [time, determinism, simulation, authority]
supersedes: []
see-also: [replay.md, simulation-time-control.md, determinism.md]
---

# Simulation Time

Time control is Rust Session authority. Pause, resume, cadence multiplier, and exact-step commands are validated and applied atomically.

## Simulation Time Control

`applyTimeControlCommand` accepts generated pause, resume, cadence-multiplier, or exact-step commands. Exact steps require paused mode, advance precisely the requested fixed-tick count, and remain paused. See `docs/simulation-time-control.md`.

## Determinism

Deterministic replay requires explicit time control. Wall-clock time is forbidden inside policy execution. See `docs/determinism.md`.
