import { test } from 'node:test';
import assert from 'node:assert/strict';
import { RuntimeBridgeError, createMockRuntimeSession } from './index.js';
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
test('RuntimeSession initializes, ticks, reads projection and telemetry, then restarts', () => {
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
test('RuntimeSession fails closed before initialize and on unsupported ProjectBundle', () => {
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
test('RuntimeSession applies collision-constrained camera input against the static room fixture', () => {
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
    assert.deepEqual(blocked.blockedAxes, ['z']);
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
test('RuntimeSession exposes the generated tunnel fixture readout and fail-closed operations', () => {
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
test('RuntimeSession exposes fire combat health readouts from typed action intents', () => {
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
test('RuntimeSession exposes read-only nav projection, path, and policy view readouts', () => {
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