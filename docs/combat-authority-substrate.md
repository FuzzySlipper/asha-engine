---
status: current
audience: agent
tags: [combat, authority, rust]
supersedes: []
see-also: []
---

# Combat Authority Substrate

Task #4040 adds the first upstream combat/health/raycast proof surface. The
public Rust import path is:

```rust
use svc_combat::{
    apply_fire_intent, CombatState, CombatTarget, FireControlState,
    FireIntentCommand, HealthState,
};
```

This is Rust authority infrastructure only. It does not define demo-specific
players, enemies, weapons, UI, policy behavior, or generated TypeScript
contracts.

## Named Surface

- Health state: `HealthState`, stored in `CombatState` keyed by `EntityId`
- Fire command: `FireIntentCommand`
- Weapon/fire validation placeholder: `FireControlState`
- Target read model: `CombatTarget` axis-aligned bounds plus `EntityId`
- Accepted events: `CombatEvent::FireHit`, `FireMissed`, `DamageApplied`,
  `EntityDefeated`
- Rejections: `CombatRejectionReason::{InvalidRay, InvalidDamage, NoAmmo,
  Cooldown, UnknownTargetHealth, InvalidHealth}`
- Readout: `CombatReadout` with `CombatFireOutcome`, next fire-control state,
  health hash, and replay hash

`apply_fire_intent` validates the command, asks `svc-collision` for the nearest
voxel collision blocker, resolves the nearest target AABB hit before that
blocker, mutates health atomically, and emits deterministic events/readout.

## Game-Rules Relationship

The #4532 migration keeps `svc-combat` as the single FPS health mutation and
fire/raycast readout path. `rule-lifecycle` now resolves primary-fire damage as
a generated game-rules `ApplyDelta` effect through `svc-game-rules`, then passes
the resulting bounded damage amount into `svc-combat::apply_fire_intent()` for
the atomic health mutation and compatibility `CombatReadout`.

That split prevents a second health table from forming in the generic
game-rules substrate:

- `svc-game-rules` owns generic catalog validation, effect resolution, modifier
  receipt/readout evidence, and poison/periodic examples.
- `svc-combat` owns target selection, fire validation, health mutation,
  `CombatEvent` compatibility readouts, health hash, and combat replay hash.
- RuntimeSession primary-fire readouts remain compatible; the internal damage
  calculation is now generic effect resolution, not a parallel TS/demo rule.

## Evidence

The committed fixture uses the #4038 generated tunnel collision projection:

- `harness/fixtures/combat/generated-tunnel-fire.snapshot.txt`

The focused tests cover:

- hit and non-lethal damage
- lethal damage plus `EntityDefeated`
- geometry-blocked miss
- invalid fire command rejection without health mutation
- replay/readout hash stability

The fixture hash values are:

- health hash `3c89045230f2d9d9`
- replay hash `6b133026c511b0f5`
