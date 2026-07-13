import { test } from 'node:test';
import assert from 'node:assert/strict';
import { cameraHandle } from '@asha/contracts';
import { RuntimeBridgeError, createNativeRuntimeBridge, createRuntimeSessionFacade, readRuntimeSessionPlayableEncounterTick, readRuntimeSessionPlayableLoopState, } from './index.js';
import { createMockRuntimeBridge } from './mock.js';
import { stableHash } from './runtime-session-hash.js';
function sessionInput() {
    return {
        sessionId: 'runtime-session.asha-demo.rust-enemy-fire',
        seed: 17,
        project: {
            gameId: 'asha-demo',
            workspaceId: 'workspace.local',
        },
        projectBundle: {
            bundleSchemaVersion: 1,
            protocolVersion: 1,
            sceneId: 42,
        },
    };
}
function hash(value) {
    return stableHash(value);
}
function snapshot(input) {
    return {
        backend: 'native_rust',
        authoritySurface: 'runtime_session.fps.reference.v0',
        projectBundle: 'custom-demo:custom-demo.scene',
        sessionEpoch: input.epoch,
        lifecycleStatus: input.enemyHealth <= 0
            ? { state: 'enemy_defeated', entity: input.enemy, tick: 7 }
            : { state: 'active' },
        playerEntity: input.player,
        enemyEntity: input.enemy,
        health: [
            { entity: input.player, current: input.playerHealth, max: 100 },
            { entity: input.enemy, current: input.enemyHealth, max: 55 },
        ],
        policyBindings: [{
                entity: input.enemy,
                bindingId: 'actor/custom-enemy:policy',
                policyId: 'policy.enemy.custom.v0',
                viewKind: 'runtime_session.fps.policy_view.v0',
                viewVersion: 'v0',
                allowedIntents: ['runtime.intent.move_direct_nav.v0', 'runtime.intent.primary_fire.v0'],
                runtimeMoment: 'autonomous_policy_tick',
            }],
        replayRecords: [{
                replayUnit: 'fps-runtime-session',
                entityHash: 'fnv1a64:00000000000000aa',
                healthHash: 'fnv1a64:00000000000000bb',
                recordHash: input.replayHash,
            }],
        readSets: [{
                viewKind: 'runtime_session.fps.lifecycle_health.v0',
                owner: 'rule-lifecycle',
                readSet: ['entity.lifecycle', 'capability.health'],
            }],
        entityHash: 'fnv1a64:00000000000000aa',
        healthHash: input.playerHealth < 100 || input.enemyHealth <= 0
            ? 'fnv1a64:00000000000000cc'
            : 'fnv1a64:00000000000000bb',
        replayHash: input.replayHash,
    };
}
function enemyFireBridgeDouble() {
    const base = createMockRuntimeBridge();
    const fireCalls = [];
    const loadRequests = [];
    const projectionCursors = [];
    let player = 10;
    let enemy = 20;
    let playerHealth = 100;
    let enemyHealth = 55;
    let current = snapshot({
        epoch: 1,
        player,
        enemy,
        playerHealth,
        enemyHealth,
        replayHash: 'fnv1a64:0000000000000001',
    });
    const bridge = new Proxy(base, {
        get(target, property, receiver) {
            if (property === 'loadFpsRuntimeSession') {
                return (request) => {
                    loadRequests.push(request);
                    player = request.definitions.find((definition) => definition.role === 'player')?.entity ?? player;
                    enemy = request.definitions.find((definition) => definition.role === 'enemy')?.entity ?? enemy;
                    playerHealth = request.definitions.find((definition) => definition.entity === player)?.health?.current ?? playerHealth;
                    enemyHealth = request.definitions.find((definition) => definition.entity === enemy)?.health?.current ?? enemyHealth;
                    current = snapshot({ epoch: 1, player, enemy, playerHealth, enemyHealth, replayHash: hash(request) });
                    return current;
                };
            }
            if (property === 'readFpsRuntimeSession') {
                return () => current;
            }
            if (property === 'readProjectionFrame') {
                return (cursor) => {
                    projectionCursors.push(cursor);
                    return target.readProjectionFrame(cursor);
                };
            }
            if (property === 'applyEnemyDirectNavMovement') {
                return (request) => ({
                    entity: request.entity,
                    authoritySource: 'rust_entity_store',
                    authorityTransport: 'native_rust',
                    from: request.seedPosition,
                    target: request.target,
                    nextWaypoint: request.target,
                    distanceUnits: 0.35,
                    reached: true,
                    pathHash: hash({ kind: 'path', request }),
                    transformHash: hash({ kind: 'transform', request }),
                    projectionChanged: true,
                });
            }
            if (property === 'applyFpsPrimaryFire') {
                return (request) => {
                    fireCalls.push(request);
                    if (request.shooterRole !== 'enemy' || request.targetRole !== 'player') {
                        throw new RuntimeBridgeError('invalid_input', 'expected enemy-to-player fire request');
                    }
                    const before = playerHealth;
                    playerHealth = Math.max(0, playerHealth - 10);
                    current = snapshot({
                        epoch: 1,
                        player,
                        enemy,
                        playerHealth,
                        enemyHealth,
                        replayHash: hash({ request, playerHealth }),
                    });
                    return {
                        backend: 'native_rust',
                        authoritySurface: 'runtime_session.fps.primary_fire.v0',
                        mutationOwner: 'svc-combat',
                        workspaceTrace: ['svc-combat.apply_damage'],
                        shooter: enemy,
                        target: player,
                        targetHealthBefore: { current: before, max: 100 },
                        targetHealthAfter: { current: playerHealth, max: 100 },
                        lifecycleStatus: { state: 'active' },
                        targetRenderVisible: true,
                        entityHash: current.entityHash,
                        healthHash: current.healthHash,
                        replayHash: current.replayHash,
                    };
                };
            }
            void receiver;
            const value = target[property];
            if (typeof value === 'function') {
                const method = value;
                return method.bind(target);
            }
            return value;
        },
    });
    return { bridge, fireCalls, loadRequests, projectionCursors };
}
void test('Rust-backed RuntimeSession autonomous enemy fire can defeat the player through bridge authority', () => {
    const { bridge, fireCalls, loadRequests, projectionCursors } = enemyFireBridgeDouble();
    const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
    session.initialize(sessionInput());
    const loaded = loadRequests[0];
    assert.ok(loaded !== undefined);
    const playerDefinition = loaded.definitions.find((definition) => definition.role === 'player');
    const enemyDefinition = loaded.definitions.find((definition) => definition.role === 'enemy');
    assert.deepEqual(playerDefinition?.bounds, {
        min: [-0.5, 0.2200000000000002, -0.5],
        max: [0.5, 3.02, 0.5],
    });
    assert.deepEqual(enemyDefinition?.bounds, {
        min: [-0.7, -0.7, -4.2],
        max: [0.7, 2.9000000000000004, -2.8],
    });
    let latest = session.runAutonomousPolicyTick({
        targetCamera: cameraHandle(1),
        tick: 1,
        enemy: { position: [3, 1, 7] },
        target: { position: [1, 1, 1] },
    });
    for (let tick = 2; tick <= 10; tick += 1) {
        latest = session.runAutonomousPolicyTick({
            targetCamera: cameraHandle(1),
            tick,
            enemy: { position: [3, 1, 7] },
            target: { position: [1, 1, 1] },
        });
    }
    assert.equal(fireCalls.length, 10);
    assert.ok(fireCalls.every((request) => request.shooterRole === 'enemy' && request.targetRole === 'player'));
    assert.equal(latest.combatSummary?.status, 'accepted');
    const outcome = latest.combatSummary?.outcome ?? null;
    assert.equal(outcome?.kind, 'hit');
    assert.equal(outcome?.kind === 'hit' ? outcome.target : null, 10);
    const lifecycle = session.readLifecycleStatus();
    assert.equal(lifecycle.player.dead, true);
    assert.equal(lifecycle.player.health.current, 0);
    assert.equal(lifecycle.enemy.dead, false);
    const playable = readRuntimeSessionPlayableLoopState(session);
    assert.equal(playable.counters.shotsFired, 0);
    assert.equal(playable.counters.actionTick, 0);
    assert.equal(playable.health.player.dead, true);
    assert.equal(session.readProjection().cursor, 10);
    assert.deepEqual(projectionCursors, [10]);
});
void test('native Rust facade maps ECRP enemy policy to authorized movement and replay evidence', (t) => {
    let bridge;
    try {
        bridge = createNativeRuntimeBridge();
    }
    catch (error) {
        if (error instanceof RuntimeBridgeError && error.kind === 'native_unavailable') {
            t.skip('native addon not built (run harness/ci/check-native.sh)');
            return;
        }
        throw error;
    }
    const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
    session.initialize(sessionInput());
    const before = bridge.readFpsRuntimeSession();
    const applied = session.requestGeneratedTunnelOperation({
        operation: 'apply_to_runtime_world',
        presetId: 'tiny-enclosed',
        seed: 17,
    });
    assert.equal(applied.status, 'applied');
    const encounterTick = readRuntimeSessionPlayableEncounterTick(session, {
        targetCamera: cameraHandle(1),
        targetPosition: [1, 1, 1],
        tick: 1,
    });
    assert.equal(encounterTick.status, 'advanced');
    assert.equal(encounterTick.blockedReason, null);
    assert.equal(encounterTick.movementSummary?.status, 'accepted');
    assert.equal(encounterTick.movementSummary?.authoritySource, 'rust_entity_store');
    const after = bridge.readFpsRuntimeSession();
    assert.notEqual(after.entityHash, before.entityHash);
    assert.ok(after.replayRecords.length > before.replayRecords.length);
    assert.ok(after.replayRecords.some((record) => record.replayUnit === 'runtime_session.fps.autonomous_movement.v0'));
});
//# sourceMappingURL=runtime-session-rust-enemy-fire.test.js.map