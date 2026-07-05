# Combat Runtime Readout

Status: public RuntimeSession combat/fire/health readout slice. The committed
generated-tunnel readouts remain as fixtures, while live runtime action receipts
now derive from the loaded ECRP RuntimeSession project state.

Public import path:

```ts
import {
  GENERATED_TUNNEL_FIRE_HIT_READOUT,
  type CombatRuntimeReadout,
  type RuntimeActionIntentEnvelope,
} from '@asha/runtime-bridge';
import { createMockRuntimeSession } from '@asha/runtime-bridge/reference';
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
When a ProjectBundle has been loaded through `loadEcrpProject()`, that receipt is
derived from the loaded player/enemy RuntimeSession state:

- outcome target equals the loaded enemy runtime entity id;
- health current/max/dead comes from the loaded enemy `health` CapabilityState;
- damage amount is the enemy's current health for the current reference slice;
- events use the loaded shooter/target entity ids and submitted tick;
- `fixture` is `null` and hashes are computed from the loaded combat record.

The #4040 generated-tunnel fire fixture is still exported and available through
`readCombatReadout()` for compatibility/golden evidence:

- outcome: hit target `20`, distance `3.5`, defeated `true`
- health: entity `20`, current `0`, max `40`, dead `true`
- events: `fire_hit`, `damage_applied`, `entity_defeated`
- next fire control: ammo `2`, cooldown `4`, after-fire cooldown `4`
- health hash `3c89045230f2d9d9`
- replay hash `6b133026c511b0f5`

`RuntimeSessionFacade.readCombatReadout()` can also return the
`generated_tunnel_geometry_blocked_miss` readout for committed miss/HUD proof:

- outcome: miss, reason `geometryBlocked`
- health remains entity `20`, current `100`, max `100`, dead `false`
- health hash `56b1331c0f202ff1`
- replay hash `3b1e1a9897571bc4`

Non-claims:

- No demo HUD rendering; #4043 owns HUD/menu projection.
- No local demo combat authority.
- No generic JSON action tunnel.
- No native-runtime combat bridge claim yet; the reference slice is the public
  TypeScript RuntimeSession authority until the equivalent native/protocol route
  lands.
