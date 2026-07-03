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
