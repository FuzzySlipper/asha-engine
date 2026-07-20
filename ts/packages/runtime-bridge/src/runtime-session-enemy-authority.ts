import type {
  EnemyPolicyProposal,
  EnemyPolicyVec3,
} from '@asha/runtime-session';
import {
  GENERATED_TUNNEL_NAV_PROJECTION,
  type NavPathReadout,
  type NavPathScenario,
} from '@asha/runtime-session';
import type {
  RuntimeSessionHashValue,
  RuntimeSessionLifecycleRole,
} from '@asha/runtime-session';
import type {
  RuntimeSessionEcrpEntityState,
  RuntimeSessionEcrpProjectState,
  RuntimeSessionEcrpTransformState,
} from './runtime-session-ecrp.js';
import { stableHash } from './runtime-session-hash.js';

export const RUNTIME_SESSION_ENEMY_MOVEMENT_AUTHORITY = {
  navServiceCrate: 'svc-pathfinding',
  runtimeTransformAuthorityCrate: 'core-scene',
  lifecycleRuleCrate: 'rule-lifecycle',
  replayUnit: 'runtime_session.enemy.direct_nav_movement.v0',
} as const;

export function buildRuntimeSessionEnemyNavPath(input: {
  readonly scenario?: NavPathScenario;
  readonly enemyPosition?: EnemyPolicyVec3;
  readonly targetPosition?: EnemyPolicyVec3;
  readonly queryFixturePath: (scenario?: NavPathScenario) => NavPathReadout;
}): NavPathReadout {
  if (input.scenario !== undefined || input.enemyPosition === undefined || input.targetPosition === undefined) {
    return input.queryFixturePath(input.scenario ?? 'generated_tunnel_reachable');
  }
  return buildDirectNavPath(input.enemyPosition, input.targetPosition);
}
export function transformForAutonomousMovementProposal(input: {
  readonly projectState: RuntimeSessionEcrpProjectState | null;
  readonly proposal: Extract<EnemyPolicyProposal, { readonly kind: 'enemy_policy.move_toward_target.v0' }>;
  readonly runtimeTransforms: ReadonlyMap<number, RuntimeSessionEcrpTransformState>;
  readonly enemyDead: boolean;
}): { readonly entity: number; readonly transform: RuntimeSessionEcrpTransformState } | null {
  const enemy = input.projectState?.entities.find((entity) => entity.role === 'enemy');
  if (enemy === undefined || input.proposal.nextWaypoint === null || input.enemyDead) {
    return null;
  }
  const current = ecrpRuntimeTransformForEntity(enemy, input.runtimeTransforms);
  return {
    entity: enemy.entity,
    transform: {
      position: input.proposal.nextWaypoint,
      yawDegrees: current?.yawDegrees ?? 0,
      pitchDegrees: current?.pitchDegrees ?? 0,
    },
  };
}

export function ecrpActorPosition(input: {
  readonly projectState: RuntimeSessionEcrpProjectState | null;
  readonly runtimeTransforms: ReadonlyMap<number, RuntimeSessionEcrpTransformState>;
  readonly role: RuntimeSessionLifecycleRole;
}): EnemyPolicyVec3 | null {
  const entity = input.projectState?.entities.find((candidate) => candidate.role === input.role);
  return entity === undefined ? null : ecrpRuntimeTransformForEntity(entity, input.runtimeTransforms)?.position ?? null;
}

export function ecrpEntityTransform(input: {
  readonly entity: RuntimeSessionEcrpEntityState;
  readonly runtimeTransforms: ReadonlyMap<number, RuntimeSessionEcrpTransformState>;
}): RuntimeSessionEcrpTransformState | null {
  return ecrpRuntimeTransformForEntity(input.entity, input.runtimeTransforms);
}

export function runtimeTransformHashRecord(
  transforms: ReadonlyMap<number, RuntimeSessionEcrpTransformState>,
): RuntimeSessionHashValue {
  return [...transforms.entries()]
    .sort(([left], [right]) => left - right)
    .map(([entity, transform]) => ({
      entity,
      position: transform.position,
      yawDegrees: transform.yawDegrees,
      pitchDegrees: transform.pitchDegrees,
    }));
}

function buildDirectNavPath(start: EnemyPolicyVec3, goal: EnemyPolicyVec3): NavPathReadout {
  const path = buildDirectNavWaypoints(start, goal, 0.35);
  const query = {
    start: { kind: 'voxel' as const, coord: start },
    goal: { kind: 'voxel' as const, coord: goal },
    maxVisited: 128,
  };
  const pathRecord = {
    replayUnit: RUNTIME_SESSION_ENEMY_MOVEMENT_AUTHORITY.replayUnit,
    navServiceCrate: RUNTIME_SESSION_ENEMY_MOVEMENT_AUTHORITY.navServiceCrate,
    projection: GENERATED_TUNNEL_NAV_PROJECTION.projectionHash,
    query,
    path,
  };
  return {
    scenario: 'generated_tunnel_reachable',
    projection: GENERATED_TUNNEL_NAV_PROJECTION,
    query,
    outcome: 'reached',
    rejectionReason: null,
    visited: path.length,
    path,
    pathHash: stableHash(pathRecord),
  };
}

function buildDirectNavWaypoints(
  start: EnemyPolicyVec3,
  goal: EnemyPolicyVec3,
  maxStepUnits: number,
): readonly EnemyPolicyVec3[] {
  const dx = goal[0] - start[0];
  const dy = goal[1] - start[1];
  const dz = goal[2] - start[2];
  const distance = Math.sqrt(dx * dx + dy * dy + dz * dz);
  if (distance <= maxStepUnits) {
    return [roundVec3(start), roundVec3(goal)];
  }
  const steps = Math.max(1, Math.ceil(distance / maxStepUnits));
  const path: EnemyPolicyVec3[] = [];
  for (let index = 0; index <= steps; index += 1) {
    const ratio = index / steps;
    path.push(roundVec3([
      start[0] + dx * ratio,
      start[1] + dy * ratio,
      start[2] + dz * ratio,
    ]));
  }
  return path;
}

function ecrpRuntimeTransformForEntity(
  entity: RuntimeSessionEcrpEntityState,
  runtimeTransforms: ReadonlyMap<number, RuntimeSessionEcrpTransformState>,
): RuntimeSessionEcrpTransformState | null {
  const runtimeTransform = runtimeTransforms.get(entity.entity);
  if (runtimeTransform !== undefined) {
    return runtimeTransform;
  }
  const [x, y, z, w] = entity.worldTransform.rotation;
  const pitchRadians = Math.asin(Math.max(-1, Math.min(1, 2 * (w * x - y * z))));
  const yawRadians = Math.atan2(2 * (w * y + x * z), 1 - 2 * (x * x + y * y));
  return {
    position: entity.worldTransform.translation,
    yawDegrees: yawRadians * 180 / Math.PI,
    pitchDegrees: pitchRadians * 180 / Math.PI,
  };
}

function roundVec3(value: EnemyPolicyVec3): EnemyPolicyVec3 {
  return [
    Number(value[0].toFixed(3)),
    Number(value[1].toFixed(3)),
    Number(value[2].toFixed(3)),
  ];
}
