import { test } from 'node:test';
import assert from 'node:assert/strict';
import { cameraHandle } from '@asha/contracts';
import { RuntimeBridgeError, createRuntimeSessionFacade, } from './index.js';
import { createMockRuntimeBridge } from './mock.js';
import { REFERENCE_RUNTIME_BACKEND_PROFILE, createMockRuntimeSession, } from './reference.js';
const HASH_ENTITY = 'fnv1a64:1000000000000001';
const HASH_HEALTH_ACTIVE = 'fnv1a64:1000000000000002';
const HASH_HEALTH_DEFEATED = 'fnv1a64:1000000000000003';
const HASH_REPLAY_ACTIVE = 'fnv1a64:1000000000000004';
const HASH_REPLAY_FIRE = 'fnv1a64:1000000000000005';
const HASH_REPLAY_RESTART = 'fnv1a64:1000000000000006';
const CAMERA_REQUEST = {
    initialPose: { position: [0, 1.6, 1.25], yawDegrees: 0, pitchDegrees: 0 },
    projection: { fovYDegrees: 60, near: 0.1, far: 100 },
    viewport: { width: 1280, height: 720 },
};
function sessionInput() {
    return {
        sessionId: 'runtime-session.evidence-lane',
        seed: 17,
        project: {
            gameId: 'asha-evidence',
            workspaceId: 'workspace.evidence',
        },
        projectBundle: {
            bundleSchemaVersion: 1,
            protocolVersion: 1,
            sceneId: 42,
        },
    };
}
function rustSnapshot(input) {
    return {
        backend: 'native_rust',
        authoritySurface: 'runtime_session.fps.authority.v0',
        projectBundle: 'evidence-product-loop',
        sessionEpoch: input.epoch,
        lifecycleStatus: input.enemyHealth <= 0
            ? { state: 'enemy_defeated', entity: input.enemy, tick: 7 }
            : { state: 'active' },
        playerEntity: input.player,
        enemyEntity: input.enemy,
        health: [
            { entity: input.player, current: 100, max: 100 },
            { entity: input.enemy, current: input.enemyHealth, max: 40 },
        ],
        policyBindings: [{
                entity: input.enemy,
                bindingId: 'binding.evidence.enemy.v0',
                policyId: 'policy.evidence.enemy.v0',
                viewKind: 'runtime_session.nav_policy_view.v0',
                viewVersion: 'v0',
                allowedIntents: ['runtime.intent.primary_fire.v0'],
                runtimeMoment: 'runtime.tick.enemy_policy.v0',
            }],
        replayRecords: [{
                replayUnit: 'runtime_session.fps.evidence.v0',
                entityHash: HASH_ENTITY,
                healthHash: input.enemyHealth <= 0 ? HASH_HEALTH_DEFEATED : HASH_HEALTH_ACTIVE,
                recordHash: input.replayHash,
            }],
        readSets: [{
                viewKind: 'runtime_session.health.v0',
                owner: 'svc-combat',
                readSet: ['CombatState.health'],
            }],
        entityHash: HASH_ENTITY,
        healthHash: input.enemyHealth <= 0 ? HASH_HEALTH_DEFEATED : HASH_HEALTH_ACTIVE,
        replayHash: input.replayHash,
    };
}
function encounterState(status) {
    return {
        presetId: 'generated-tunnel-small-encounter',
        status,
        spawnedEnemyIds: status === 'pending' ? [] : ['encounter.generated_tunnel_small.wave_1.enemy_001'],
        defeatedEnemyIds: status === 'cleared' ? ['encounter.generated_tunnel_small.wave_1.enemy_001'] : [],
        revision: status === 'pending' ? 0 : 1,
        lastTransition: status === 'pending' ? 'initialized' : status === 'cleared' ? 'cleared' : 'activated',
    };
}
function encounterSnapshot(state, lifecycle) {
    return {
        backend: 'native_rust',
        authoritySurface: 'runtime_session.fps.encounter_director.v0',
        mutationOwner: 'rule-lifecycle',
        workspaceTrace: ['evidence.rust.encounter_read'],
        state,
        lifecycle,
        readSets: [{
                viewKind: 'runtime_session.encounter_director.v0',
                owner: 'rule-lifecycle',
                readSet: ['FpsRuntimeSessionState.encounter'],
            }],
        encounterHash: 'fnv1a64:1000000000000007',
        replayHash: 'fnv1a64:1000000000000008',
    };
}
function rustEvidenceBridge() {
    const base = createMockRuntimeBridge();
    const calls = {
        collision: [],
        fire: [],
        restart: [],
        encounterTransition: [],
    };
    let epoch = 1;
    let player = 10;
    let enemy = 20;
    let enemyHealth = 40;
    let snapshot = rustSnapshot({ epoch, player, enemy, enemyHealth, replayHash: HASH_REPLAY_ACTIVE });
    let currentEncounterState = encounterState('pending');
    const bridge = new Proxy(base, {
        get(target, property, receiver) {
            if (property === 'applyCollisionConstrainedCameraInput') {
                return (input) => {
                    calls.collision.push(input);
                    return target.applyCollisionConstrainedCameraInput(input);
                };
            }
            if (property === 'loadFpsRuntimeSession') {
                return (request) => {
                    const requestedPlayer = request.definitions.find((definition) => definition.role === 'player');
                    const requestedEnemy = request.definitions.find((definition) => definition.role === 'enemy');
                    player = requestedPlayer?.entity ?? player;
                    enemy = requestedEnemy?.entity ?? enemy;
                    enemyHealth = requestedEnemy?.health?.current ?? enemyHealth;
                    snapshot = rustSnapshot({ epoch, player, enemy, enemyHealth, replayHash: HASH_REPLAY_ACTIVE });
                    currentEncounterState = encounterState('pending');
                    return snapshot;
                };
            }
            if (property === 'readFpsRuntimeSession') {
                return () => {
                    return snapshot;
                };
            }
            if (property === 'applyFpsPrimaryFire') {
                return (request) => {
                    calls.fire.push(request);
                    const before = enemyHealth;
                    enemyHealth = 0;
                    snapshot = rustSnapshot({ epoch, player, enemy, enemyHealth, replayHash: HASH_REPLAY_FIRE });
                    return {
                        backend: 'native_rust',
                        authoritySurface: 'runtime_session.fps.primary_fire.v0',
                        mutationOwner: 'svc-combat',
                        workspaceTrace: ['evidence.rust.primary_fire', 'svc-combat.apply_damage'],
                        shooter: player,
                        target: enemy,
                        targetHealthBefore: { current: before, max: 40 },
                        targetHealthAfter: { current: 0, max: 40 },
                        lifecycleStatus: { state: 'enemy_defeated', entity: enemy, tick: request.tick },
                        targetRenderVisible: false,
                        entityHash: HASH_ENTITY,
                        healthHash: HASH_HEALTH_DEFEATED,
                        replayHash: HASH_REPLAY_FIRE,
                    };
                };
            }
            if (property === 'restartFpsRuntimeSession') {
                return (request) => {
                    calls.restart.push(request);
                    if (request.expectedEpoch !== epoch) {
                        throw new RuntimeBridgeError('invalid_input', 'stale restart epoch');
                    }
                    epoch += 1;
                    enemyHealth = 40;
                    snapshot = rustSnapshot({ epoch, player, enemy, enemyHealth, replayHash: HASH_REPLAY_RESTART });
                    currentEncounterState = encounterState('pending');
                    return snapshot;
                };
            }
            if (property === 'readFpsEncounterDirector') {
                return (lifecycle) => {
                    return encounterSnapshot(currentEncounterState, lifecycle);
                };
            }
            if (property === 'applyFpsEncounterTransition') {
                return (request) => {
                    calls.encounterTransition.push(request);
                    currentEncounterState = request.lifecycle.enemyDead
                        ? encounterState('cleared')
                        : encounterState('active');
                    return {
                        backend: 'native_rust',
                        authoritySurface: 'runtime_session.fps.encounter_transition.v0',
                        mutationOwner: 'rule-lifecycle',
                        workspaceTrace: ['evidence.rust.encounter_transition'],
                        accepted: true,
                        rejectionReason: null,
                        eventKind: request.lifecycle.enemyDead
                            ? 'runtime_encounter.lifecycle_synced.v0'
                            : 'runtime_encounter.activated.v0',
                        state: currentEncounterState,
                        lifecycle: request.lifecycle,
                        encounterHash: 'fnv1a64:1000000000000009',
                        replayHash: 'fnv1a64:1000000000000010',
                    };
                };
            }
            const value = Reflect.get(target, property, receiver);
            if (typeof value === 'function') {
                const method = value;
                return method.bind(target);
            }
            return value;
        },
    });
    return { bridge, calls };
}
void test('[reference evidence] RuntimeSession fixture lane is explicitly non-product authority', () => {
    const session = createMockRuntimeSession();
    const initialized = session.initialize(sessionInput());
    assert.equal(initialized.identity.mode, 'reference');
    assert.equal(REFERENCE_RUNTIME_BACKEND_PROFILE.backendKind, 'reference_fixture');
    assert.equal(REFERENCE_RUNTIME_BACKEND_PROFILE.transport, 'reference_bridge');
    assert.equal(REFERENCE_RUNTIME_BACKEND_PROFILE.productAuthority, false);
    assert.ok(REFERENCE_RUNTIME_BACKEND_PROFILE.disallowedUse.includes('live-demo-default'));
    assert.ok(initialized.identity.nonClaims.includes('not_product_authority'));
    assert.ok(initialized.identity.nonClaims.includes('not_native_runtime'));
    const camera = session.createCamera(CAMERA_REQUEST).snapshot.camera;
    const receipt = session.submitRuntimeActionIntent({
        kind: 'runtime_action_intent.v0',
        action: 'primary_fire',
        phase: 'pressed',
        camera,
        tick: 7,
        source: 'programmatic',
        pressed: true,
    });
    assert.equal(receipt.accepted, true);
    assert.equal(receipt.combatReadout?.authority.source, 'reference_fixture');
    assert.equal(receipt.combatReadout?.authority.backend, null);
    assert.equal(receipt.combatReadout?.authority.surface, 'runtime_session.reference_fixture.generated_tunnel_combat.v0');
    assert.equal(receipt.combatReadout?.authority.mutationOwner, 'reference-runtime-session');
});
void test('[rust authority evidence] public RuntimeSession facade reports backend provenance for product loop', () => {
    const { bridge, calls } = rustEvidenceBridge();
    const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
    const initialized = session.initialize(sessionInput());
    assert.equal(initialized.identity.mode, 'rust');
    assert.equal(initialized.identity.nonClaims.includes('not_product_authority'), false);
    const camera = session.createCamera(CAMERA_REQUEST).snapshot.camera;
    const collisionEnvelope = {
        camera,
        grid: 1,
        input: {
            moveForward: 1,
            moveRight: 0,
            moveUp: 0,
            yawDeltaDegrees: 0,
            pitchDeltaDegrees: 0,
            dtSeconds: 1,
            moveSpeedUnitsPerSecond: 99,
        },
        tick: 2,
        shape: { halfExtents: [0.25, 0.25, 0.25] },
        policy: { mode: 'axis_separable_slide', maxIterations: 3 },
    };
    const collision = session.applyCollisionConstrainedCameraInput(collisionEnvelope);
    assert.equal(calls.collision.length, 1);
    assert.equal(collision.collided, true);
    assert.ok(collision.collisionProjectionHash.startsWith('fnv1a64:'));
    const fire = session.submitRuntimeActionIntent({
        kind: 'runtime_action_intent.v0',
        action: 'primary_fire',
        phase: 'pressed',
        camera: cameraHandle(camera),
        tick: 7,
        source: 'programmatic',
        pressed: true,
    });
    assert.equal(fire.accepted, true);
    assert.equal(fire.combatReadout?.fixture, null);
    assert.equal(fire.combatReadout?.authority.source, 'rust_bridge');
    assert.equal(fire.combatReadout?.authority.backend, 'native_rust');
    assert.equal(fire.combatReadout?.authority.surface, 'runtime_session.fps.primary_fire.v0');
    assert.deepEqual(calls.fire.map((call) => call.tick), [7]);
    const lifecycle = session.readLifecycleStatus();
    assert.equal(lifecycle.outcome.kind, 'won');
    assert.equal(lifecycle.restart.reason, 'rust_epoch_restart');
    assert.match(lifecycle.hashes.replayHash, /^fnv1a64:[0-9a-f]{16}$/);
    const encounter = session.requestEncounterTransition({
        kind: 'runtime_session.encounter_transition_request.v0',
        presetId: 'generated-tunnel-small-encounter',
        action: 'sync_lifecycle',
    });
    assert.equal(encounter.accepted, true);
    assert.equal(encounter.after.authority.source, 'rust_bridge');
    assert.equal(encounter.after.authority.backend, 'native_rust');
    assert.equal(encounter.after.state.status, 'cleared');
    assert.equal(calls.encounterTransition[0]?.lifecycle.outcomeKind, 'won');
    const restart = session.requestSessionRestart({
        kind: 'runtime.restart_session_intent',
        source: 'programmatic',
        requireTerminal: true,
        expectedSessionHash: encounter.hashes.sessionHashAfter,
    });
    assert.equal(restart.accepted, true);
    assert.equal(restart.statusAfter.outcome.kind, 'in_progress');
    assert.deepEqual(calls.restart, [{ expectedEpoch: 1 }]);
    assert.match(session.readLifecycleStatus().hashes.replayHash, /^fnv1a64:[0-9a-f]{16}$/);
});
//# sourceMappingURL=runtime-session-evidence.test.js.map