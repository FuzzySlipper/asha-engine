---
status: current
audience: agent
tags: [time, simulation, authority]
supersedes: []
see-also: []
---

# Simulation Time Control

Status: implemented public Session capability (#5613).

Simulation time control is authority pacing, not a second clock and not scaled
simulation delta. Rust owns the current mode, cadence multiplier, exact-step
validation, revision, authority tick, and deterministic receipts. TypeScript and
UI code only propose generated commands through `RuntimeSessionFacade`.

## Commands

- `pause` stops ordinary authority tick advancement. Render, menu, diagnostics,
  projection, and inspection reads remain available.
- `resume` re-enables ordinary fixed-tick advancement.
- `setSpeedMultiplier` accepts integer multipliers from 1 through 16. The value
  controls how many ordinary fixed-tick pipeline iterations the runtime executes
  per wall-clock cadence pulse; it never changes fixed tick delta or state/replay
  calculations.
- `stepTicks` accepts 1 through 10,000 ticks only while paused. It advances the
  authority by executing that many ordinary fixed-tick pipeline iterations and
  leaves the Session paused.

Rejected commands return a classified, hash-bound receipt and mutate neither
the controller revision nor authority tick. Repeated pause/resume, invalid speed,
invalid step count, and exact-step attempts while running all fail closed.

## Consumer Paths

All consumer shapes converge before authority:

- headless tools call `applyTimeControlCommand` directly;
- HUD `ui.pause_intent` / `ui.resume_intent` values map through
  `hudIntentToTimeControlCommand`;
- named-input consumers map `runtime.time.pause`, `runtime.time.resume`, and
  `runtime.time.step_one` resolved actions through `ResolvedTimeControlConsumer`.

These are adapters to one generated command vocabulary, not separate pause
systems. The browser owns event attachment and cadence scheduling; it does not
own simulation state.

## Public Surface

Generated contracts live in `@asha/contracts` as `TimeControlCommand`,
`TimeControlState`, and `TimeControlReceipt`. `@asha/runtime-session` exposes
`applyTimeControlCommand` and `readTimeControlState`. Concrete native/reference
construction remains in `@asha/runtime-bridge`.

The native operation pair is `apply_time_control_command` and
`read_time_control_state`. Native `stepSimulation` treats one call as a cadence
pulse, executes the configured number of sequential fixed ticks, and returns the
final authority tick plus the aggregate count of authority events actually
applied. Each fixed tick drains the commands scheduled for that exact tick into
`sim-runner`, which performs the normal typed validation → event accumulation →
`StateStore` application pipeline. No-input ticks therefore report zero diffs;
there is no cadence-derived or tick-modulo synthetic counter. While paused, the
returned tick is unchanged. `stepTicks` uses this same runner-owned per-tick path,
including command consumption and state evolution.

## Non-Claims

This slice does not provide slow-motion physics, scaled deltas, per-zone clocks,
rewind, automatic pause policy, or UI-owned authority. Browser hosts still own
wall-clock pulse timing, while Rust consumes the cadence multiplier and keeps
simulation and replay pacing-independent.
