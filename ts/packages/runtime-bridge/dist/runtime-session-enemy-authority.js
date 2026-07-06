import { GENERATED_TUNNEL_NAV_PROJECTION, } from './nav-readout.js';
import { stableHash } from './runtime-session-hash.js';
export const RUNTIME_SESSION_ENEMY_MOVEMENT_AUTHORITY = {
    navServiceCrate: 'svc-pathfinding',
    runtimeTransformAuthorityCrate: 'core-scene',
    lifecycleRuleCrate: 'rule-lifecycle',
    replayUnit: 'runtime_session.enemy.direct_nav_movement.v0',
};
export function buildRuntimeSessionEnemyNavPath(input) {
    if (input.scenario !== undefined || input.enemyPosition === undefined || input.targetPosition === undefined) {
        return input.queryFixturePath(input.scenario ?? 'generated_tunnel_reachable');
    }
    return buildDirectNavPath(input.enemyPosition, input.targetPosition);
}
export function transformForAutonomousMovementProposal(input) {
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
export function ecrpActorPosition(input) {
    const entity = input.projectState?.entities.find((candidate) => candidate.role === input.role);
    return entity === undefined ? null : ecrpRuntimeTransformForEntity(entity, input.runtimeTransforms)?.position ?? null;
}
export function ecrpEntityTransform(input) {
    return ecrpRuntimeTransformForEntity(input.entity, input.runtimeTransforms);
}
export function runtimeTransformHashRecord(transforms) {
    return [...transforms.entries()]
        .sort(([left], [right]) => left - right)
        .map(([entity, transform]) => ({
        entity,
        position: transform.position,
        yawDegrees: transform.yawDegrees,
        pitchDegrees: transform.pitchDegrees,
    }));
}
function buildDirectNavPath(start, goal) {
    const path = buildDirectNavWaypoints(start, goal, 0.35);
    const query = {
        start: { kind: 'voxel', coord: start },
        goal: { kind: 'voxel', coord: goal },
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
function buildDirectNavWaypoints(start, goal, maxStepUnits) {
    const dx = goal[0] - start[0];
    const dy = goal[1] - start[1];
    const dz = goal[2] - start[2];
    const distance = Math.sqrt(dx * dx + dy * dy + dz * dz);
    if (distance <= maxStepUnits) {
        return [roundVec3(start), roundVec3(goal)];
    }
    const steps = Math.max(1, Math.ceil(distance / maxStepUnits));
    const path = [];
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
function ecrpRuntimeTransformForEntity(entity, runtimeTransforms) {
    const runtimeTransform = runtimeTransforms.get(entity.entity);
    if (runtimeTransform !== undefined) {
        return runtimeTransform;
    }
    const definitionTransform = entity.definition.capabilities.find((capability) => capability.kind === 'transform');
    if (definitionTransform?.kind !== 'transform') {
        return null;
    }
    return {
        position: definitionTransform.initial.position,
        yawDegrees: definitionTransform.initial.yawDegrees,
        pitchDegrees: definitionTransform.initial.pitchDegrees,
    };
}
function roundVec3(value) {
    return [
        Number(value[0].toFixed(3)),
        Number(value[1].toFixed(3)),
        Number(value[2].toFixed(3)),
    ];
}
//# sourceMappingURL=runtime-session-enemy-authority.js.map