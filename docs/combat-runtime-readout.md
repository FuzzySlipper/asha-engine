# Combat Runtime Readout

Status: task #4051 public RuntimeSession combat/fire/health readout slice.

Public import path:

```ts
import {
  GENERATED_TUNNEL_FIRE_HIT_READOUT,
  createMockRuntimeSession,
  type CombatRuntimeReadout,
  type RuntimeActionIntentEnvelope,
} from '@asha/runtime-bridge';
```

Primary fire uses the typed action intent protocol from #4036:

```ts
{
  kind: 'runtime_action_intent.v0',
  action: 'primary_fire',
  phase: 'pressed',
  camera,
  tick,
  source,
  pressed: true
}
```

`RuntimeSessionFacade.submitRuntimeActionIntent()` returns an accepted receipt for
`primary_fire` press intents in the reference slice, with a `CombatRuntimeReadout`.
The readout is pinned to the #4040 generated-tunnel fire fixture:

- outcome: hit target `20`, distance `3.5`, defeated `true`
- health: entity `20`, current `0`, max `40`, dead `true`
- events: `fire_hit`, `damage_applied`, `entity_defeated`
- next fire control: ammo `2`, cooldown `4`, after-fire cooldown `4`
- health hash `3c89045230f2d9d9`
- replay hash `6b133026c511b0f5`

`RuntimeSessionFacade.readCombatReadout()` can also return the
`generated_tunnel_geometry_blocked_miss` readout for miss/HUD proof:

- outcome: miss, reason `geometryBlocked`
- health remains entity `20`, current `100`, max `100`, dead `false`
- health hash `56b1331c0f202ff1`
- replay hash `3b1e1a9897571bc4`

Non-claims:

- No demo HUD rendering; #4043 owns HUD/menu projection.
- No enemy policy behavior.
- No local demo combat authority.
- No generic JSON action tunnel.
