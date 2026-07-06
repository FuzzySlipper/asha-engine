import { test } from 'node:test';
import assert from 'node:assert/strict';
import { cameraHandle } from '@asha/contracts';
import { RuntimeBridgeError, createRuntimeSessionFacade, } from './index.js';
import { createMockRuntimeBridge } from './mock.js';
import { createMockRuntimeSession } from './reference.js';
import { RUNTIME_SESSION_RUST_FPS_AUTHORITY } from './runtime-session-rust-fps-authority.js';
import { stableHash } from './runtime-session-hash.js';
function sessionInput() {
    return {
        sessionId: 'runtime-session.asha-demo.reference',
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
function ecrpProjectLoadInput() {
    return {
        kind: 'runtime_session.load_ecrp_project.v0',
        projectBundle: {
            kind: 'ProjectBundle',
            project: {
                gameId: 'custom-demo',
                workspaceId: 'workspace.custom',
            },
            runtimeRequest: {
                bundleSchemaVersion: 1,
                protocolVersion: 1,
                sceneId: 77,
            },
        },
        entityDefinitions: [
            {
                kind: 'EntityDefinition',
                stableId: 'actor/custom-player',
                displayName: 'Custom Player',
                source: {
                    projectBundle: 'custom-demo',
                    relativePath: 'catalogs/actors/custom-player.entity.json',
                },
                capabilities: [
                    {
                        kind: 'transform',
                        initial: {
                            position: [1, 1.7, 2],
                            yawDegrees: 15,
                            pitchDegrees: 0,
                        },
                    },
                    {
                        kind: 'collisionBody',
                        halfExtents: [0.3, 0.7, 0.3],
                    },
                    {
                        kind: 'controller',
                        controller: 'player_input',
                    },
                    {
                        kind: 'health',
                        current: 88,
                        max: 88,
                    },
                    {
                        kind: 'weaponMount',
                        weaponId: 'weapon.custom.primary',
                    },
                    {
                        kind: 'renderProjection',
                        projection: 'first_person_camera',
                    },
                    {
                        kind: 'faction',
                        factionId: 'player',
                    },
                ],
            },
            {
                kind: 'EntityDefinition',
                stableId: 'actor/custom-enemy',
                displayName: 'Custom Enemy',
                source: {
                    projectBundle: 'custom-demo',
                    relativePath: 'catalogs/actors/custom-enemy.entity.json',
                },
                capabilities: [
                    {
                        kind: 'transform',
                        initial: {
                            position: [4, 1.2, -6],
                            yawDegrees: 180,
                            pitchDegrees: 0,
                        },
                    },
                    {
                        kind: 'collisionBody',
                        halfExtents: [0.8, 1, 0.8],
                    },
                    {
                        kind: 'health',
                        current: 55,
                        max: 55,
                    },
                    {
                        kind: 'renderProjection',
                        projection: 'target_cube',
                    },
                    {
                        kind: 'policyBinding',
                        policyId: 'policy.enemy.custom.v0',
                    },
                    {
                        kind: 'spawnMarker',
                        markerId: 'spawn.enemy.custom',
                    },
                    {
                        kind: 'faction',
                        factionId: 'hostile',
                    },
                ],
            },
        ],
        sceneDocument: {
            kind: 'SceneDocument',
            sceneId: 'custom-demo.scene',
            placements: [
                {
                    entityDefinitionId: 'actor/custom-player',
                    runtimeEntityId: 101,
                    spawnMarkerId: 'spawn.player.custom',
                },
                {
                    entityDefinitionId: 'actor/custom-enemy',
                    runtimeEntityId: 202,
                    spawnMarkerId: 'spawn.enemy.custom',
                },
            ],
        },
    };
}
function rustFpsSnapshot(input) {
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
            { entity: input.player, current: 88, max: 88 },
            { entity: input.enemy, current: input.enemyHealth, max: 55 },
        ],
        policyBindings: [{
                entity: input.enemy,
                bindingId: 'actor/custom-enemy:policy',
                policyId: 'policy.enemy.custom.v0',
                viewKind: 'runtime_session.fps.policy_view.v0',
                viewVersion: 'v0',
                allowedIntents: ['enemy_policy.move_toward_target.v0', 'enemy_policy.primary_fire_intent.v0'],
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
        healthHash: input.enemyHealth <= 0 ? 'fnv1a64:00000000000000cc' : 'fnv1a64:00000000000000bb',
        replayHash: input.replayHash,
    };
}
function rustRuntimeSessionBridgeDouble() {
    const base = createMockRuntimeBridge();
    const calls = { load: [], fire: [], restart: [] };
    let player = 10;
    let enemy = 20;
    let epoch = 1;
    let snapshot = rustFpsSnapshot({
        epoch,
        player,
        enemy,
        enemyHealth: 40,
        replayHash: 'fnv1a64:0000000000000001',
    });
    const bridge = new Proxy(base, {
        get(target, property, receiver) {
            if (property === 'loadFpsRuntimeSession') {
                return (request) => {
                    calls.load.push(request);
                    player = request.definitions.find((definition) => definition.role === 'player')?.entity ?? player;
                    enemy = request.definitions.find((definition) => definition.role === 'enemy')?.entity ?? enemy;
                    const enemyDefinition = request.definitions.find((definition) => definition.entity === enemy);
                    snapshot = rustFpsSnapshot({
                        epoch,
                        player,
                        enemy,
                        enemyHealth: enemyDefinition?.health?.current ?? 40,
                        replayHash: 'fnv1a64:0000000000000002',
                    });
                    return snapshot;
                };
            }
            if (property === 'applyFpsPrimaryFire') {
                return (request) => {
                    calls.fire.push(request.tick);
                    snapshot = rustFpsSnapshot({
                        epoch,
                        player,
                        enemy,
                        enemyHealth: 0,
                        replayHash: 'fnv1a64:0000000000000003',
                    });
                    return {
                        backend: 'native_rust',
                        authoritySurface: 'runtime_session.fps.reference.v0',
                        mutationOwner: 'svc-combat',
                        workspaceTrace: ['workspace.primary_fire', 'svc-combat.apply_damage', 'rule-lifecycle.enemy_defeated'],
                        shooter: player,
                        target: enemy,
                        targetHealthBefore: { current: 55, max: 55 },
                        targetHealthAfter: { current: 0, max: 55 },
                        lifecycleStatus: { state: 'enemy_defeated', entity: enemy, tick: request.tick },
                        targetRenderVisible: false,
                        entityHash: 'fnv1a64:00000000000000aa',
                        healthHash: 'fnv1a64:00000000000000cc',
                        replayHash: 'fnv1a64:0000000000000003',
                    };
                };
            }
            if (property === 'readFpsRuntimeSession') {
                return () => snapshot;
            }
            if (property === 'restartFpsRuntimeSession') {
                return (request) => {
                    calls.restart.push(request);
                    assert.equal(request.expectedEpoch, epoch);
                    epoch += 1;
                    snapshot = rustFpsSnapshot({
                        epoch,
                        player,
                        enemy,
                        enemyHealth: 55,
                        replayHash: 'fnv1a64:0000000000000004',
                    });
                    return snapshot;
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
    return { bridge, calls };
}
void test('RuntimeSession initializes, ticks, reads projection and telemetry, then restarts', () => {
    const session = createMockRuntimeSession();
    const initialized = session.initialize(sessionInput());
    assert.equal(initialized.identity.sessionId, 'runtime-session.asha-demo.reference');
    assert.equal(initialized.identity.mode, 'reference');
    assert.equal(initialized.composition.loadedWorld, 42);
    assert.ok(initialized.identity.nonClaims.includes('not_raw_state_store'));
    assert.ok(initialized.identity.nonClaims.includes('not_arbitrary_json_bridge'));
    const command = {
        op: 'setVoxel',
        grid: 1,
        coord: { x: 0, y: 0, z: 0 },
        value: { kind: 'solid', material: 1 },
    };
    const receipt = session.submitCommands({ commands: [command] });
    assert.equal(receipt.result.accepted, 1);
    assert.equal(receipt.result.rejected, 0);
    assert.notEqual(receipt.sessionHashAfter, receipt.sessionHashBefore);
    const tick = session.tick();
    assert.equal(tick.tick, 1);
    assert.equal(tick.composition.loadedWorld, 42);
    const projection = session.readProjection();
    assert.equal(projection.sequenceId, tick.sequenceId);
    assert.equal(projection.renderDiffCount, 0);
    assert.ok(projection.projectionHash.startsWith('fnv1a64:'));
    const telemetry = session.readTelemetry();
    assert.equal(telemetry.acceptedCommandCount, 1);
    assert.equal(telemetry.rejectedCommandCount, 0);
    assert.equal(telemetry.replayRecords.map((record) => record.kind).join(','), 'initialize,submitCommands,tick');
    const restarted = session.restart();
    assert.equal(restarted.tick, 0);
    assert.equal(restarted.restartCount, 1);
    assert.equal(restarted.composition.loadedWorld, 42);
    const afterRestart = session.readTelemetry();
    assert.equal(afterRestart.acceptedCommandCount, 0);
    assert.equal(afterRestart.rejectedCommandCount, 0);
    assert.equal(afterRestart.replayRecords.at(-1)?.kind, 'restart');
});
void test('Rust-backed RuntimeSession routes ECRP load, primary fire, and restart through bridge authority', () => {
    const { bridge, calls } = rustRuntimeSessionBridgeDouble();
    const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
    const initialized = session.initialize(sessionInput());
    assert.equal(initialized.identity.mode, 'rust');
    assert.equal(initialized.identity.nonClaims.includes('not_native_runtime'), false);
    assert.equal(calls.load.length, 1);
    assert.equal(calls.load[0]?.definitions.some((definition) => definition.role === 'enemy'), true);
    const load = session.loadEcrpProject(ecrpProjectLoadInput());
    assert.equal(load.accepted, true);
    assert.equal(load.entityCount, 2);
    assert.equal(load.bootstrapHash, 'fnv1a64:00000000000000aa');
    assert.equal(calls.load.at(-1)?.projectBundle, 'custom-demo:custom-demo.scene');
    assert.equal(calls.load.at(-1)?.definitions.find((definition) => definition.role === 'enemy')?.entity, 202);
    assert.equal(calls.load.at(-1)?.definitions.find((definition) => definition.role === 'enemy')?.policyBinding?.policyId, 'policy.enemy.custom.v0');
    const receipt = session.submitRuntimeActionIntent({
        kind: 'runtime_action_intent.v0',
        action: 'primary_fire',
        phase: 'pressed',
        camera: cameraHandle(1),
        tick: 7,
        source: 'programmatic',
        pressed: true,
    });
    assert.equal(receipt.accepted, true);
    assert.equal(receipt.combatReadout?.fixture, null);
    assert.equal(receipt.combatReadout?.outcome.kind, 'hit');
    assert.equal(receipt.combatReadout?.outcome.kind === 'hit' ? receipt.combatReadout.outcome.target : null, 202);
    assert.equal(receipt.combatReadout?.health[0]?.current, 0);
    assert.equal(receipt.combatReadout?.healthHash, 'fnv1a64:00000000000000cc');
    assert.deepEqual(calls.fire, [7]);
    const lifecycle = session.readLifecycleStatus();
    assert.equal(lifecycle.restart.reason, 'rust_epoch_restart');
    assert.equal(lifecycle.outcome.kind, 'won');
    assert.equal(lifecycle.enemy.health.entity, 202);
    assert.equal(lifecycle.enemy.health.dead, true);
    const restart = session.requestSessionRestart({
        kind: 'runtime.restart_session_intent',
        source: 'programmatic',
        requireTerminal: true,
        expectedSessionHash: receipt.sessionHashAfter,
    });
    assert.equal(restart.accepted, true);
    assert.equal(restart.statusAfter.outcome.kind, 'in_progress');
    assert.deepEqual(calls.restart, [{ expectedEpoch: 1 }]);
    const staleRestart = session.requestSessionRestart({
        kind: 'runtime.restart_session_intent',
        source: 'programmatic',
        expectedSessionHash: receipt.sessionHashAfter,
    });
    assert.equal(staleRestart.accepted, false);
    assert.equal(staleRestart.rejection?.reason, 'session_hash_mismatch');
    assert.deepEqual(calls.restart, [{ expectedEpoch: 1 }]);
});
void test('Rust-backed RuntimeSession fails closed for unwired live policy helpers', () => {
    const { bridge } = rustRuntimeSessionBridgeDouble();
    const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
    session.initialize(sessionInput());
    assert.throws(() => session.runAutonomousPolicyTick({ targetCamera: cameraHandle(1), tick: 2 }), (error) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented');
    assert.throws(() => session.readNavProjection(), (error) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented');
    assert.throws(() => session.requestSessionRestart({
        kind: 'runtime.restart_session_intent',
        source: 'programmatic',
        expectedSessionHash: '',
    }), (error) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input');
});
void test('RuntimeSession fails closed before initialize and on unsupported ProjectBundle', () => {
    const session = createMockRuntimeSession();
    assert.throws(() => session.tick(), (error) => error instanceof RuntimeBridgeError && error.kind === 'not_initialized');
    assert.throws(() => session.initialize({
        ...sessionInput(),
        projectBundle: {
            bundleSchemaVersion: 99,
            protocolVersion: 1,
            sceneId: 42,
        },
    }), (error) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input');
});
void test('RuntimeSession exposes public ECRP entity and CapabilityState readouts', () => {
    const session = createMockRuntimeSession();
    assert.throws(() => session.readEcrpRuntimeReadout(), (error) => error instanceof RuntimeBridgeError && error.kind === 'not_initialized');
    session.initialize(sessionInput());
    const initial = session.readEcrpRuntimeReadout();
    assert.equal(initial.kind, 'runtime_session.ecrp_readout.v0');
    assert.equal(initial.entityCount, 2);
    assert.ok(initial.nonClaims.includes('not_raw_state_store'));
    assert.ok(initial.nonClaims.includes('not_demo_local_authority'));
    const player = initial.entities.find((entity) => entity.definitionStableId === 'actor/demo-player');
    const enemy = initial.entities.find((entity) => entity.definitionStableId === 'actor/generated-tunnel-enemy');
    assert.ok(player);
    assert.ok(enemy);
    assert.deepEqual(player.capabilityKinds, [
        'transform',
        'collisionBody',
        'controller',
        'health',
        'weaponMount',
        'renderProjection',
        'faction',
    ]);
    assert.ok(enemy.capabilityKinds.includes('health'));
    assert.ok(enemy.capabilityKinds.includes('policyBinding'));
    const initialEnemyHealth = enemy.capabilities.find((capability) => capability.kind === 'health');
    assert.equal(initialEnemyHealth?.kind, 'health');
    assert.equal(initialEnemyHealth?.dead, false);
    assert.equal(enemy.recentEvents.length, 1);
    assert.equal(enemy.recentEvents[0]?.kind, 'runtime_session.bootstrap_entity.v0');
    const receipt = session.submitRuntimeActionIntent({
        kind: 'runtime_action_intent.v0',
        action: 'primary_fire',
        phase: 'pressed',
        camera: cameraHandle(1),
        tick: 7,
        source: 'programmatic',
        pressed: true,
    });
    assert.equal(receipt.accepted, true);
    assert.equal(receipt.combatReadout?.scenario, 'generated_tunnel_fire_hit');
    assert.equal(receipt.combatReadout?.outcome.kind, 'hit');
    assert.equal(receipt.combatReadout?.outcome.kind === 'hit' ? receipt.combatReadout.outcome.target : null, 20);
    const afterFire = session.readEcrpRuntimeReadout();
    const defeatedEnemy = afterFire.entities.find((entity) => entity.entity === 20);
    const defeatedHealth = defeatedEnemy?.capabilities.find((capability) => capability.kind === 'health');
    const defeatedRender = defeatedEnemy?.capabilities.find((capability) => capability.kind === 'renderProjection');
    assert.equal(defeatedHealth?.kind, 'health');
    assert.equal(defeatedHealth?.dead, true);
    assert.equal(defeatedHealth?.current, 0);
    assert.equal(defeatedRender?.kind, 'renderProjection');
    assert.equal(defeatedRender?.visible, false);
    assert.ok(defeatedEnemy?.recentEvents.some((event) => event.kind === 'runtime_lifecycle.enemy_defeated.v0'));
    assert.notEqual(afterFire.hashes.capabilityStateHash, initial.hashes.capabilityStateHash);
    assert.notEqual(afterFire.hashes.eventReadoutHash, initial.hashes.eventReadoutHash);
});
void test('RuntimeSession loads ECRP ProjectBundle content into live readouts', () => {
    const session = createMockRuntimeSession();
    session.initialize(sessionInput());
    const load = session.loadEcrpProject(ecrpProjectLoadInput());
    assert.equal(load.kind, 'runtime_session.ecrp_project_load_receipt.v0');
    assert.equal(load.accepted, true);
    assert.deepEqual(load.diagnostics, []);
    assert.equal(load.entityCount, 2);
    assert.match(load.bootstrapHash ?? '', /^fnv1a64:[0-9a-f]{16}$/);
    const readout = session.readEcrpRuntimeReadout();
    assert.equal(readout.project.gameId, 'custom-demo');
    assert.equal(readout.projectBundle.sceneId, 77);
    assert.equal(readout.entityCount, 2);
    const player = readout.entities.find((entity) => entity.definitionStableId === 'actor/custom-player');
    const enemy = readout.entities.find((entity) => entity.definitionStableId === 'actor/custom-enemy');
    assert.equal(player?.entity, 101);
    assert.equal(enemy?.entity, 202);
    assert.equal(player?.source.relativePath, 'catalogs/actors/custom-player.entity.json');
    const playerTransform = player?.capabilities.find((capability) => capability.kind === 'transform');
    assert.equal(playerTransform?.kind, 'transform');
    assert.deepEqual(playerTransform?.position, [1, 1.7, 2]);
    const enemyHealth = enemy?.capabilities.find((capability) => capability.kind === 'health');
    assert.equal(enemyHealth?.kind, 'health');
    assert.equal(enemyHealth?.current, 55);
    assert.equal(enemyHealth?.max, 55);
    assert.equal(enemyHealth?.dead, false);
    const receipt = session.submitRuntimeActionIntent({
        kind: 'runtime_action_intent.v0',
        action: 'primary_fire',
        phase: 'pressed',
        camera: cameraHandle(2),
        tick: 9,
        source: 'programmatic',
        pressed: true,
    });
    assert.equal(receipt.accepted, true);
    assert.equal(receipt.combatReadout?.scenario, 'runtime_session_loaded_project_fire_hit');
    assert.equal(receipt.combatReadout?.outcome.kind, 'hit');
    assert.equal(receipt.combatReadout?.outcome.kind === 'hit' ? receipt.combatReadout.outcome.target : null, 202);
    assert.equal(receipt.combatReadout?.events.find((event) => event.kind === 'fire_hit')?.shooter, 101);
    assert.equal(receipt.combatReadout?.events.find((event) => event.kind === 'damage_applied')?.target, 202);
    assert.equal(receipt.combatReadout?.events.find((event) => event.kind === 'damage_applied')?.amount, 55);
    assert.deepEqual(receipt.combatReadout?.health[0], {
        entity: 202,
        current: 0,
        max: 55,
        dead: true,
    });
    assert.equal(receipt.combatReadout?.replayHash, stableHash({
        replayUnit: RUNTIME_SESSION_RUST_FPS_AUTHORITY.primaryFireReplayUnit,
        ruleCrate: RUNTIME_SESSION_RUST_FPS_AUTHORITY.ruleCrate,
        combatServiceCrate: RUNTIME_SESSION_RUST_FPS_AUTHORITY.combatServiceCrate,
        scenario: 'runtime_session_loaded_project_fire_hit',
        shooter: 101,
        target: 202,
        weaponId: 'weapon.custom.primary',
        health: [
            {
                entity: 202,
                current: 0,
                max: 55,
                dead: true,
            },
        ],
        events: [
            {
                kind: 'fire_hit',
                shooter: 101,
                target: 202,
                distance: 3.5,
                tick: 9,
            },
            {
                kind: 'damage_applied',
                target: 202,
                amount: 55,
                before: 55,
                after: 0,
            },
            {
                kind: 'entity_defeated',
                target: 202,
            },
        ],
    }));
    assert.equal(receipt.combatReadout?.fixture, null);
    const afterFire = session.readEcrpRuntimeReadout();
    const defeatedEnemy = afterFire.entities.find((entity) => entity.entity === 202);
    const defeatedHealth = defeatedEnemy?.capabilities.find((capability) => capability.kind === 'health');
    const defeatedRender = defeatedEnemy?.capabilities.find((capability) => capability.kind === 'renderProjection');
    assert.equal(defeatedHealth?.kind, 'health');
    assert.equal(defeatedHealth?.current, 0);
    assert.equal(defeatedHealth?.dead, true);
    assert.equal(defeatedRender?.kind, 'renderProjection');
    assert.equal(defeatedRender?.visible, false);
    assert.ok(defeatedEnemy?.recentEvents.some((event) => event.kind === 'runtime_lifecycle.enemy_defeated.v0'));
});
void test('RuntimeSession rejects invalid ECRP ProjectBundle content without replacing live state', () => {
    const session = createMockRuntimeSession();
    session.initialize(sessionInput());
    const before = session.readEcrpRuntimeReadout();
    const invalid = {
        ...ecrpProjectLoadInput(),
        sceneDocument: {
            ...ecrpProjectLoadInput().sceneDocument,
            placements: [
                {
                    entityDefinitionId: 'actor/unknown',
                    runtimeEntityId: 404,
                },
            ],
        },
    };
    const load = session.loadEcrpProject(invalid);
    assert.equal(load.accepted, false);
    assert.ok(load.diagnostics.some((diagnostic) => diagnostic.code === 'unknownEntityDefinition'));
    assert.ok(load.diagnostics.some((diagnostic) => diagnostic.code === 'missingPlacement'));
    assert.equal(load.bootstrapHash, null);
    const after = session.readEcrpRuntimeReadout();
    assert.deepEqual(after.entities.map((entity) => entity.definitionStableId), before.entities.map((entity) => entity.definitionStableId));
});
void test('RuntimeSession applies collision-constrained camera input against the static room fixture', () => {
    const session = createMockRuntimeSession();
    session.initialize(sessionInput());
    const cameraRequest = {
        initialPose: {
            position: [0, 1.6, 0],
            yawDegrees: 0,
            pitchDegrees: 0,
        },
        projection: {
            fovYDegrees: 60,
            near: 0.1,
            far: 100,
        },
        viewport: {
            width: 1280,
            height: 720,
        },
    };
    const camera = session.createCamera(cameraRequest).snapshot.camera;
    const collisionShape = { halfExtents: [0.25, 0.25, 0.25] };
    const collisionPolicy = { mode: 'axis_separable_slide', maxIterations: 3 };
    const blockedEnvelope = {
        camera,
        grid: 1,
        input: {
            moveForward: 1,
            moveRight: 0,
            moveUp: 0,
            yawDeltaDegrees: 10,
            pitchDeltaDegrees: -2,
            dtSeconds: 1,
            moveSpeedUnitsPerSecond: 99,
        },
        tick: 1,
        shape: collisionShape,
        policy: collisionPolicy,
    };
    const blocked = session.applyCollisionConstrainedCameraInput(blockedEnvelope);
    assert.equal(blocked.collided, true);
    assert.deepEqual(blocked.blockedAxes, ['x', 'z']);
    assert.deepEqual(blocked.snapshot.after.pose.position, blocked.snapshot.before.pose.position);
    assert.ok(blocked.snapshot.attempted.pose.position[2] < -90);
    assert.equal(blocked.snapshot.after.pose.yawDegrees, 10);
    assert.equal(blocked.snapshot.after.pose.pitchDegrees, -2);
    assert.ok(blocked.worldHash.startsWith('fnv1a64:'));
    assert.ok(blocked.collisionProjectionHash.startsWith('fnv1a64:'));
    assert.ok(blocked.movementHash.startsWith('fnv1a64:'));
    const lateralEnvelope = {
        ...blockedEnvelope,
        input: {
            moveForward: 0,
            moveRight: 1,
            moveUp: 0,
            yawDeltaDegrees: 0,
            pitchDeltaDegrees: 0,
            dtSeconds: 1,
            moveSpeedUnitsPerSecond: 1,
        },
        tick: 2,
    };
    const lateral = session.applyCollisionConstrainedCameraInput(lateralEnvelope);
    assert.equal(lateral.collided, false);
    assert.deepEqual(lateral.blockedAxes, []);
    assert.ok(lateral.snapshot.after.pose.position[0] > lateral.snapshot.before.pose.position[0]);
    assert.notDeepEqual(lateral.snapshot.after.pose.position, lateral.snapshot.before.pose.position);
    const telemetry = session.readTelemetry();
    assert.equal(telemetry.replayRecords.at(-1)?.kind, 'applyCollisionConstrainedCameraInput');
});
void test('collision-constrained camera movement is horizontal and target-obstacle constrained', () => {
    const session = createMockRuntimeSession();
    session.initialize(sessionInput());
    const collisionShape = { halfExtents: [0.25, 0.7, 0.25] };
    const collisionPolicy = { mode: 'axis_separable_slide', maxIterations: 3 };
    const camera = session.createCamera({
        initialPose: {
            position: [0, 1.62, 0],
            yawDegrees: 0,
            pitchDegrees: 55,
        },
        projection: {
            fovYDegrees: 60,
            near: 0.1,
            far: 100,
        },
        viewport: {
            width: 1280,
            height: 720,
        },
    }).snapshot.camera;
    const intoTarget = session.applyCollisionConstrainedCameraInput({
        camera,
        grid: 1,
        input: {
            moveForward: 1,
            moveRight: 0,
            moveUp: 0,
            yawDeltaDegrees: 0,
            pitchDeltaDegrees: 0,
            dtSeconds: 1,
            moveSpeedUnitsPerSecond: 2,
        },
        tick: 1,
        shape: collisionShape,
        policy: collisionPolicy,
    });
    assert.equal(intoTarget.collided, true);
    assert.deepEqual(intoTarget.blockedAxes, ['z']);
    assert.ok(Math.abs(intoTarget.snapshot.attempted.pose.position[1] - 1.62) < 0.00001);
    assert.ok(Math.abs(intoTarget.snapshot.after.pose.position[1] - 1.62) < 0.00001);
    const yawedCamera = session.createCamera({
        initialPose: {
            position: [0, 1.62, 0],
            yawDegrees: 45,
            pitchDegrees: 55,
        },
        projection: {
            fovYDegrees: 60,
            near: 0.1,
            far: 100,
        },
        viewport: {
            width: 1280,
            height: 720,
        },
    }).snapshot.camera;
    const yawedForward = session.applyCollisionConstrainedCameraInput({
        camera: yawedCamera,
        grid: 1,
        input: {
            moveForward: 1,
            moveRight: 0,
            moveUp: 0,
            yawDeltaDegrees: 0,
            pitchDeltaDegrees: 0,
            dtSeconds: 0.1,
            moveSpeedUnitsPerSecond: 2,
        },
        tick: 1,
        shape: collisionShape,
        policy: collisionPolicy,
    });
    assert.equal(yawedForward.collided, false);
    assert.ok(yawedForward.snapshot.after.pose.position[0] > 0);
    assert.ok(yawedForward.snapshot.after.pose.position[2] < 0);
    assert.ok(Math.abs(yawedForward.snapshot.after.pose.position[1] - 1.62) < 0.00001);
});
void test('collision-constrained camera movement uses same-tick look deltas for forward movement', () => {
    const session = createMockRuntimeSession();
    session.initialize(sessionInput());
    const camera = session.createCamera({
        initialPose: {
            position: [0, 1.62, 1.5],
            yawDegrees: 0,
            pitchDegrees: 0,
        },
        projection: {
            fovYDegrees: 60,
            near: 0.1,
            far: 100,
        },
        viewport: {
            width: 1280,
            height: 720,
        },
    }).snapshot.camera;
    const moved = session.applyCollisionConstrainedCameraInput({
        camera,
        grid: 1,
        input: {
            moveForward: 1,
            moveRight: 0,
            moveUp: 0,
            yawDeltaDegrees: 45,
            pitchDeltaDegrees: 0,
            dtSeconds: 0.1,
            moveSpeedUnitsPerSecond: 2,
        },
        tick: 1,
        shape: { halfExtents: [0.25, 0.7, 0.25] },
        policy: { mode: 'axis_separable_slide', maxIterations: 3 },
    });
    assert.equal(moved.collided, false);
    assert.equal(moved.snapshot.after.pose.yawDegrees, 45);
    assert.ok(moved.snapshot.after.pose.position[0] > 0);
    assert.ok(moved.snapshot.after.pose.position[2] < 1.5);
    assert.ok(Math.abs(moved.snapshot.after.pose.position[1] - 1.62) < 0.00001);
});
void test('RuntimeSession exposes the generated tunnel fixture readout and fail-closed operations', () => {
    const session = createMockRuntimeSession();
    session.initialize(sessionInput());
    const readout = session.readGeneratedTunnelReadout({ presetId: 'tiny-enclosed', seed: 17 });
    assert.equal(readout.status, 'available');
    assert.equal(readout.generator.generatorId, 'asha.tunnel.enclosed.v1');
    assert.equal(readout.generator.presetId, 'tiny-enclosed');
    assert.equal(readout.generator.seed, 17);
    assert.equal(readout.generator.configHash, 'e1d156c6b55137a7');
    assert.equal(readout.generator.outputHash, 'a9b504096397f5b4');
    assert.equal(readout.replayHash, 'fnv1a64:0821a0c2aea17dff');
    assert.deepEqual(readout.volume.tunnelDims, [5, 4, 9]);
    assert.equal(readout.volume.solidVoxels, 138);
    assert.equal(readout.corridors.count, 1);
    assert.equal(readout.rooms.count, 0);
    assert.deepEqual(readout.spawnMarkers.map((marker) => marker.id), ['player_start', 'exit_hint']);
    assert.deepEqual(readout.materials.map((material) => `${material.role}:${material.material}`), [
        'wall:1',
        'floor:2',
        'accent:3',
    ]);
    assert.equal(readout.renderProjection.hash, 'fnv1a64:21eb8696f6f3b5c4');
    assert.equal(readout.collisionProjection.hash, 'fnv1a64:78b242163cf67524');
    const operation = session.requestGeneratedTunnelOperation({
        operation: 'regenerate',
        presetId: 'tiny-enclosed',
        seed: 17,
    });
    assert.equal(operation.status, 'unsupported');
    assert.equal(operation.reason, 'generated_tunnel_operation_not_wired');
    assert.equal('payload' in operation, false);
    assert.equal(session.readTelemetry().replayRecords.at(-1)?.kind, 'requestGeneratedTunnelOperation');
    assert.throws(() => session.readGeneratedTunnelReadout({ presetId: 'tiny-enclosed', seed: 18 }), (error) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input');
});
void test('RuntimeSession exposes fire combat health readouts from typed action intents', () => {
    const session = createMockRuntimeSession();
    session.initialize(sessionInput());
    const camera = session.createCamera({
        initialPose: { position: [2.5, 1.5, 1.5], yawDegrees: 180, pitchDegrees: 0 },
        projection: { fovYDegrees: 60, near: 0.1, far: 100 },
        viewport: { width: 1280, height: 720 },
    }).snapshot.camera;
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
    assert.equal(receipt.status, 'accepted');
    assert.equal(receipt.rejection, null);
    assert.equal(receipt.combatReadout?.outcome.kind, 'hit');
    assert.equal(receipt.combatReadout?.outcome.kind === 'hit' ? receipt.combatReadout.outcome.target : null, 20);
    assert.equal(receipt.combatReadout?.outcome.kind === 'hit' ? receipt.combatReadout.outcome.distance : null, 3.5);
    assert.equal(receipt.combatReadout?.health[0]?.current, 0);
    assert.equal(receipt.combatReadout?.health[0]?.max, 40);
    assert.equal(receipt.combatReadout?.health[0]?.dead, true);
    assert.deepEqual(receipt.combatReadout?.events.map((event) => event.kind), [
        'fire_hit',
        'damage_applied',
        'entity_defeated',
    ]);
    assert.equal(receipt.combatReadout?.healthHash, '3c89045230f2d9d9');
    assert.equal(receipt.combatReadout?.replayHash, '6b133026c511b0f5');
    assert.equal('payload' in receipt, false);
    const miss = session.readCombatReadout({ scenario: 'generated_tunnel_geometry_blocked_miss' });
    assert.equal(miss.outcome.kind, 'miss');
    assert.equal(miss.outcome.kind === 'miss' ? miss.outcome.reason : null, 'geometryBlocked');
    assert.deepEqual(miss.events.map((event) => event.kind), ['fire_missed']);
    assert.equal(miss.health[0]?.current, 100);
    assert.equal(miss.health[0]?.dead, false);
    assert.equal(miss.healthHash, '56b1331c0f202ff1');
    assert.equal(miss.replayHash, '3b1e1a9897571bc4');
    const useReceipt = session.submitRuntimeActionIntent({
        kind: 'runtime_action_intent.v0',
        action: 'use',
        phase: 'pressed',
        camera,
        tick: 8,
        source: 'programmatic',
        pressed: true,
    });
    assert.equal(useReceipt.accepted, false);
    assert.equal(useReceipt.status, 'unsupported');
    assert.equal(useReceipt.rejection?.reason, 'combat_runtime_not_wired');
});
void test('RuntimeSession exposes read-only nav projection, path, and policy view readouts', () => {
    const session = createMockRuntimeSession();
    session.initialize(sessionInput());
    const projection = session.readNavProjection();
    assert.equal(projection.id, 'generated_tunnel_nav_projection');
    assert.equal(projection.available, true);
    assert.equal(projection.walkableCells, 66);
    assert.equal(projection.projectionHash, 'd1f6ac3e051d6b6e');
    const reachable = session.queryNavPath({ scenario: 'generated_tunnel_reachable' });
    assert.equal(reachable.outcome, 'reached');
    assert.equal(reachable.visited, 21);
    assert.equal(reachable.path.length, 9);
    assert.deepEqual(reachable.path[0], [3, 1, 7]);
    assert.deepEqual(reachable.path.at(-1), [1, 1, 1]);
    assert.equal(reachable.pathHash, 'e8e1ea7a09811ced');
    const noPath = session.queryNavPath({ scenario: 'generated_tunnel_no_path' });
    assert.equal(noPath.outcome, 'no_path');
    assert.equal(noPath.rejectionReason, 'blocked');
    assert.deepEqual(noPath.path, []);
    assert.equal(noPath.pathHash, 'a8c7f832281a39c5');
    const policyView = session.readNavPolicyView();
    assert.equal(policyView.kind, 'nav_policy_view.v0');
    assert.equal(policyView.readOnly, true);
    assert.equal(policyView.proposalOnly, true);
    assert.equal('mutate' in policyView, false);
    assert.equal('applyPath' in policyView, false);
    assert.equal(policyView.latestPath.pathHash, reachable.pathHash);
    assert.throws(() => session.queryNavPath({ maxVisited: 0 }), (error) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input');
});
//# sourceMappingURL=runtime-session.test.js.map