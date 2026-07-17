---
status: current
audience: agent
tags: [replay, determinism, time, module-state, authority]
supersedes: []
see-also: [gameplay-module-state-replay.md, determinism.md, simulation-time-control.md]
---

# Replay and Determinism

Replay is the core audit mechanism for agent-written changes. Determinism is enforced through explicit services and canonical replay targets.

## Replay Model

Recording proposed commands, accepted events, state hashes, snapshots, and divergence reports. Headless replay tests with compact diagnostics. See `docs/replay-model.md`.

## Determinism

Shipping WASM semantics are the replay authority. Native builds are for tools and fast iteration. All authoritative randomness comes from deterministic engine services. See `docs/determinism.md`.

## Simulation Time Control

Pause, resume, cadence-multiplier, and exact-step commands through Rust Session authority. Exact steps require paused mode and advance precisely the requested tick count. See `docs/simulation-time-control.md`.

## Gameplay Module State Replay

`GameplayReactionFrame` captures the inspectable causal boundary for one fabric reaction. Two replay modes: `playback_frame` (applies recorded facts) and `run_verification_replay` (reruns the fabric and categorizes divergences). See `docs/gameplay-module-state-replay.md`.
