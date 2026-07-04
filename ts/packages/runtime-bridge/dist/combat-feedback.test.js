import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildCombatFeedbackProjectionFromReceipt, createMockRuntimeSession, } from './index.js';
function sessionInput() {
    return {
        sessionId: 'runtime-session.combat-feedback.reference',
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
function cameraRequest() {
    return {
        initialPose: { position: [2.5, 1.5, 1.5], yawDegrees: 180, pitchDegrees: 0 },
        projection: { fovYDegrees: 60, near: 0.1, far: 100 },
        viewport: { width: 1280, height: 720 },
    };
}
test('Combat feedback projection exposes hit marker, trace, HUD status, and death notices', () => {
    const session = createMockRuntimeSession();
    session.initialize(sessionInput());
    const camera = session.createCamera(cameraRequest()).snapshot.camera;
    const feedback = session.readCombatFeedbackProjection({
        scenario: 'generated_tunnel_fire_hit',
        camera,
    });
    assert.equal(feedback.kind, 'combat_feedback_projection.v0');
    assert.equal(feedback.scenario, 'generated_tunnel_fire_hit');
    assert.equal(feedback.intent.accepted, true);
    assert.equal(feedback.intent.status, 'accepted');
    assert.equal(feedback.trace.result, 'hit');
    assert.equal(feedback.trace.shooter, 10);
    assert.equal(feedback.trace.target, 20);
    assert.equal(feedback.trace.distance, 3.5);
    assert.deepEqual(feedback.trace.origin, [2.5, 1.5, 1.5]);
    assert.deepEqual(feedback.trace.direction, [0, 0, 1]);
    assert.deepEqual(feedback.trace.endpoint, [2.5, 1.5, 5]);
    assert.equal(feedback.marker.visible, true);
    assert.equal(feedback.marker.tone, 'hit');
    assert.equal(feedback.marker.label, 'Hit');
    assert.equal(feedback.notifications.at(-1)?.eventKind, 'entity_defeated');
    assert.equal(feedback.notifications.at(-1)?.text, 'Entity 20 defeated');
    assert.equal(feedback.hud.status[0]?.tone, 'danger');
    assert.equal(feedback.hud.status[0]?.text, 'Entity 20 defeated');
    assert.equal(feedback.hud.ammo, 2);
    assert.equal(feedback.hud.cooldownTicksRemaining, 4);
    assert.equal(feedback.health[0]?.dead, true);
    assert.equal(feedback.debug.fixturePath, 'harness/fixtures/combat-feedback/generated-tunnel-hit-feedback.snapshot.txt');
    assert.equal(feedback.debug.healthHash, '3c89045230f2d9d9');
    assert.equal(feedback.debug.combatReplayHash, '6b133026c511b0f5');
    assert.equal(feedback.debug.cameraProjectionHash, 'fnv1a64:2aa10d532f1ba47c');
    assert.equal(feedback.hashes.traceHash, 'fnv1a64:1aceefd6bae6854b');
    assert.equal(feedback.hashes.markerHash, 'fnv1a64:a315c6bca60902a7');
    assert.equal(feedback.hashes.notificationHash, 'fnv1a64:9642c2b243aba4a6');
    assert.equal(feedback.hashes.projectionHash, 'fnv1a64:bc2c68ffadb58153');
    assert.ok(feedback.nonClaims.includes('not_combat_authority'));
    assert.ok(feedback.nonClaims.includes('not_ui_state'));
});
test('Combat feedback projection exposes blocked miss feedback without damage authority', () => {
    const session = createMockRuntimeSession();
    session.initialize(sessionInput());
    const feedback = session.readCombatFeedbackProjection({
        scenario: 'generated_tunnel_geometry_blocked_miss',
    });
    assert.equal(feedback.scenario, 'generated_tunnel_geometry_blocked_miss');
    assert.equal(feedback.trace.result, 'miss');
    assert.equal(feedback.trace.reason, 'geometryBlocked');
    assert.equal(feedback.trace.origin, null);
    assert.equal(feedback.marker.visible, true);
    assert.equal(feedback.marker.tone, 'blocked');
    assert.equal(feedback.marker.durationMs, 120);
    assert.equal(feedback.notifications[0]?.eventKind, 'fire_missed');
    assert.equal(feedback.notifications[0]?.text, 'Shot blocked');
    assert.equal(feedback.hud.status[0]?.tone, 'warning');
    assert.equal(feedback.hud.status[0]?.text, 'Shot blocked');
    assert.equal(feedback.health[0]?.dead, false);
    assert.equal(feedback.debug.fixturePath, 'harness/fixtures/combat-feedback/generated-tunnel-miss-feedback.snapshot.txt');
    assert.equal(feedback.debug.healthHash, '56b1331c0f202ff1');
    assert.equal(feedback.debug.combatReplayHash, '3b1e1a9897571bc4');
    assert.equal(feedback.debug.cameraProjectionHash, null);
    assert.equal(feedback.hashes.traceHash, 'fnv1a64:e43d8314c447650a');
    assert.equal(feedback.hashes.markerHash, 'fnv1a64:60e974b91a995cb9');
    assert.equal(feedback.hashes.notificationHash, 'fnv1a64:ecb4d2cfba72dc5f');
    assert.equal(feedback.hashes.projectionHash, 'fnv1a64:5bd66a99af374c64');
});
test('Combat feedback projection fails closed for unsupported action receipts', () => {
    const session = createMockRuntimeSession();
    session.initialize(sessionInput());
    const camera = session.createCamera(cameraRequest()).snapshot.camera;
    const receipt = session.submitRuntimeActionIntent({
        kind: 'runtime_action_intent.v0',
        action: 'use',
        phase: 'pressed',
        camera,
        tick: 8,
        source: 'programmatic',
        pressed: true,
    });
    const cameraProjection = session.readCameraProjection({ camera, viewport: null }).snapshot;
    const feedback = buildCombatFeedbackProjectionFromReceipt(receipt, cameraProjection);
    assert.equal(receipt.status, 'unsupported');
    assert.equal(feedback.scenario, 'runtime_action_unsupported');
    assert.equal(feedback.intent.accepted, false);
    assert.equal(feedback.intent.status, 'unsupported');
    assert.equal(feedback.intent.rejectionReason, 'combat_runtime_not_wired');
    assert.equal(feedback.trace.result, 'not_fired');
    assert.equal(feedback.trace.reason, 'intent_not_accepted');
    assert.equal(feedback.marker.visible, false);
    assert.equal(feedback.marker.tone, 'inactive');
    assert.equal(feedback.notifications[0]?.eventKind, 'runtime_action_unsupported');
    assert.equal(feedback.hud.status[0]?.tone, 'warning');
    assert.equal(feedback.health.length, 0);
    assert.equal(feedback.debug.fixturePath, null);
    assert.equal(feedback.debug.healthHash, null);
    assert.equal(feedback.debug.cameraProjectionHash, cameraProjection.projectionHash);
    assert.equal('payload' in feedback, false);
    assert.equal(feedback.hashes.traceHash, 'fnv1a64:568f673eb24fb2ec');
    assert.equal(feedback.hashes.markerHash, 'fnv1a64:a316059c7849f192');
    assert.equal(feedback.hashes.notificationHash, 'fnv1a64:25d43e19dec90931');
    assert.equal(feedback.hashes.projectionHash, 'fnv1a64:fa583dd7482a3c40');
});
//# sourceMappingURL=combat-feedback.test.js.map