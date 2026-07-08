import { test } from 'node:test';
import assert from 'node:assert/strict';

import { cameraHandle } from '@asha/contracts';

import {
  GENERATED_TUNNEL_NAV_POLICY_VIEW,
  GENERATED_TUNNEL_NAV_PROJECTION,
  GENERATED_TUNNEL_FIRE_HIT_READOUT,
  TINY_GENERATED_TUNNEL_READOUT,
  buildEncounterDirectorReadout,
  buildCombatFeedbackProjection,
  createGeneratedTunnelEnemyPolicyFixture,
  defaultCombatFeedbackIntent,
  type RuntimeActionIntentEnvelope,
} from './index.js';

void test('@asha/runtime-session exposes semantic readouts without a bridge backend', () => {
  const envelope: RuntimeActionIntentEnvelope = {
    kind: 'runtime_action_intent.v0',
    action: 'primary_fire',
    phase: 'pressed',
    camera: cameraHandle(1),
    tick: 3,
    source: 'programmatic',
    pressed: true,
  };

  const projection = buildCombatFeedbackProjection({
    ...defaultCombatFeedbackIntent(envelope),
    sequenceId: 7,
    combatReadout: GENERATED_TUNNEL_FIRE_HIT_READOUT,
  });

  assert.equal(TINY_GENERATED_TUNNEL_READOUT.status, 'available');
  assert.equal(GENERATED_TUNNEL_NAV_PROJECTION.available, true);
  assert.equal(projection.trace.result, 'hit');
  assert.equal(projection.intent.accepted, true);
});

void test('@asha/runtime-session root owns generated-tunnel semantic helpers', () => {
  const camera = cameraHandle(9);
  const enemyPolicy = createGeneratedTunnelEnemyPolicyFixture({
    target: { camera },
    nav: GENERATED_TUNNEL_NAV_POLICY_VIEW,
  });
  const encounter = buildEncounterDirectorReadout({
    state: {
      presetId: 'generated-tunnel-small-encounter',
      status: 'active',
      spawnedEnemyIds: ['encounter.generated_tunnel_small.wave_1.enemy_001'],
      defeatedEnemyIds: [],
      revision: 1,
      lastTransition: 'activated',
    },
    sequenceId: 1,
    tick: 3,
    sessionSeed: 17,
    sessionHash: 'fnv1a64:test-session',
    lifecycle: {
      outcomeKind: 'in_progress',
      terminal: false,
      enemyDead: false,
      playerDead: false,
      lifecycleHash: 'fnv1a64:test-lifecycle',
    },
  });

  assert.equal(enemyPolicy.kind, 'generated_tunnel_enemy_policy_fixture.v0');
  assert.equal(enemyPolicy.frame.proposals.length, 2);
  assert.equal(encounter.kind, 'runtime_session.encounter_director.v0');
  assert.equal(encounter.state.status, 'active');
});
