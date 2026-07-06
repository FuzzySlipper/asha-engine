import {
  GENERATED_TUNNEL_FIRE_HIT_READOUT,
  type CombatEventReadout,
  type CombatRuntimeReadout,
} from './combat-readout.js';
import type { RuntimeActionIntentEnvelope } from './runtime-action.js';
import { stableHash } from './runtime-session-hash.js';
import type {
  RuntimeSessionEcrpProjectState,
  RuntimeSessionLifecycleState,
} from './runtime-session.js';

export const REFERENCE_FPS_COMBAT_FIXTURE_PROVENANCE = {
  ruleCrate: 'rule-lifecycle',
  combatServiceCrate: 'svc-combat',
  entityBootstrapServiceCrate: 'svc-entity-authoring',
  primaryFireReplayUnit: 'runtime_session.fps.primary_fire.v0',
} as const;

export function buildReferenceFpsCombatFixturePrimaryFireReadout(input: {
  readonly projectState: RuntimeSessionEcrpProjectState | null;
  readonly lifecycleState: RuntimeSessionLifecycleState;
  readonly source: RuntimeActionIntentEnvelope['source'];
  readonly tick: number;
}): CombatRuntimeReadout {
  if (input.source === 'enemy_policy') {
    return buildPrimaryFireHitReadout({
      projectState: input.projectState,
      tick: input.tick,
      shooter: input.lifecycleState.enemy.entity,
      targetBefore: input.lifecycleState.player,
      damage: 10,
      distance: 2.25,
      weaponOwnerRole: 'enemy',
    });
  }

  const shooter = input.lifecycleState.player.entity;
  const targetBefore = input.lifecycleState.enemy;

  if (
    shooter === 10 &&
    targetBefore.entity === 20 &&
    targetBefore.current === 40 &&
    targetBefore.max === 40 &&
    input.tick === 7
  ) {
    return GENERATED_TUNNEL_FIRE_HIT_READOUT;
  }

  return buildPrimaryFireHitReadout({
    projectState: input.projectState,
    tick: input.tick,
    shooter,
    targetBefore,
    damage: targetBefore.current,
    distance: 3.5,
    weaponOwnerRole: 'player',
  });
}

function buildPrimaryFireHitReadout(input: {
  readonly projectState: RuntimeSessionEcrpProjectState | null;
  readonly tick: number;
  readonly shooter: number;
  readonly targetBefore: RuntimeSessionLifecycleState['player'];
  readonly damage: number;
  readonly distance: number;
  readonly weaponOwnerRole: 'player' | 'enemy';
}): CombatRuntimeReadout {
  const damage = Math.min(input.damage, input.targetBefore.current);
  const targetAfter = {
    entity: input.targetBefore.entity,
    current: Math.max(0, input.targetBefore.current - damage),
    max: input.targetBefore.max,
    dead: input.targetBefore.current - damage <= 0,
  };
  const health = [targetAfter];
  const events: CombatEventReadout[] = [
    {
      kind: 'fire_hit',
      shooter: input.shooter,
      target: targetAfter.entity,
      distance: input.distance,
      tick: input.tick,
    },
    {
      kind: 'damage_applied',
      target: targetAfter.entity,
      amount: damage,
      before: input.targetBefore.current,
      after: targetAfter.current,
    },
  ];
  if (targetAfter.dead) {
    events.push({
      kind: 'entity_defeated',
      target: targetAfter.entity,
    });
  }
  const weaponMount = input.projectState?.entities
    .find((entity) => entity.role === input.weaponOwnerRole)
    ?.definition.capabilities.find((capability) => capability.kind === 'weaponMount');
  const combatRecord = {
    replayUnit: REFERENCE_FPS_COMBAT_FIXTURE_PROVENANCE.primaryFireReplayUnit,
    ruleCrate: REFERENCE_FPS_COMBAT_FIXTURE_PROVENANCE.ruleCrate,
    combatServiceCrate: REFERENCE_FPS_COMBAT_FIXTURE_PROVENANCE.combatServiceCrate,
    scenario: 'runtime_session_loaded_project_fire_hit',
    shooter: input.shooter,
    target: targetAfter.entity,
    weaponId: weaponMount?.kind === 'weaponMount' ? weaponMount.weaponId : null,
    health,
    events,
  };

  return {
    scenario: 'runtime_session_loaded_project_fire_hit',
    outcome: {
      kind: 'hit',
      target: targetAfter.entity,
      distance: input.distance,
      hitPosition: null,
      defeated: targetAfter.dead,
    },
    events,
    health,
    nextFireControl: {
      ammo: 2,
      cooldownTicksRemaining: 4,
      cooldownTicksAfterFire: 4,
    },
    healthHash: stableHash(health),
    replayHash: stableHash(combatRecord),
    authority: {
      source: 'reference_fixture',
      backend: null,
      surface: 'runtime_session.reference_fixture.fps_combat.v0',
      mutationOwner: 'reference-runtime-session',
      workspaceTrace: ['reference RuntimeSession FPS combat fixture'],
    },
    fixture: null,
  };
}
