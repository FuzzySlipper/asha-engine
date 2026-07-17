---
status: current
audience: agent
tags: [combat, health, lifecycle, authority]
supersedes: []
see-also: [runtime-session-facade.md, ecrp-runtime-session-readout.md]
---

# Combat and Lifecycle

Combat and lifecycle authority are Rust-owned through `rule-lifecycle` and `svc-combat`. The RuntimeSession exposes typed action intents and readouts.

## Combat Authority Substrate

Combat/health/fire-intent authority surface with replay evidence. See `topics/authority/combat-authority-substrate.md`.

## Combat Runtime Readout

Committed generated-tunnel combat fixture readouts for compatibility/golden evidence. See `topics/authority/combat-runtime-readout.md`.

## Lifecycle Status and Restart

`readLifecycleStatus` reads player/enemy lifecycle status, win/loss/in-progress outcome, restart eligibility, and terminal death events. `requestSessionRestart` validates a typed restart intent and resets the session deterministically. See `topics/authority/runtime-session-facade.md`.
