import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  GENERATED_TUNNEL_SMALL_ENCOUNTER_CONFIG,
  type CameraCreateRequest,
  type EncounterTransitionRequest,
} from './index.js';
import { createMockRuntimeSession } from './reference.js';

function sessionInput() {
  return {
    sessionId: 'runtime-session.encounter-director.reference',
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

function cameraRequest(): CameraCreateRequest {
  return {
    initialPose: { position: [2.5, 1.5, 1.5], yawDegrees: 180, pitchDegrees: 0 },
    projection: { fovYDegrees: 60, near: 0.1, far: 100 },
    viewport: { width: 1280, height: 720 },
  };
}

function activateRequest(): EncounterTransitionRequest {
  return {
    kind: 'runtime_session.encounter_transition_request.v0',
    presetId: 'generated-tunnel-small-encounter',
    action: 'activate',
  };
}

function syncLifecycleRequest(): EncounterTransitionRequest {
  return {
    kind: 'runtime_session.encounter_transition_request.v0',
    presetId: 'generated-tunnel-small-encounter',
    action: 'sync_lifecycle',
  };
}

void test('RuntimeSession exposes deterministic encounter director pending and active readouts', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());

  const pending = session.readEncounterDirector();
  assert.equal(pending.kind, 'runtime_session.encounter_director.v0');
  assert.equal(pending.presetId, 'generated-tunnel-small-encounter');
  assert.equal(pending.config.source, 'project_bundle.encounter_preset');
  assert.equal(pending.config.configHash, GENERATED_TUNNEL_SMALL_ENCOUNTER_CONFIG.configHash);
  assert.equal(pending.config.spawnOrderHash, GENERATED_TUNNEL_SMALL_ENCOUNTER_CONFIG.spawnOrderHash);
  assert.equal(pending.config.configHash, '3c8c9c0d0541935f');
  assert.equal(pending.config.spawnOrderHash, '68fafab271825648');
  assert.equal(pending.config.fixturePath, 'harness/fixtures/encounters/generated-tunnel-small-encounter.snapshot.txt');
  assert.equal(pending.state.status, 'pending');
  assert.equal(pending.state.pendingEnemyCount, 1);
  assert.equal(pending.state.activeEnemyCount, 0);
  assert.equal(pending.state.spawnedEnemyCount, 0);
  assert.deepEqual(pending.spawns.map((spawn) => spawn.instanceId), [
    'encounter.generated_tunnel_small.wave_1.enemy_001',
  ]);
  assert.deepEqual(pending.spawns.map((spawn) => spawn.spawnMarker.markerId), ['exit_hint']);
  assert.deepEqual(pending.spawns[0]?.spawnMarker.world, [3.5, 1.5, 7.5]);
  assert.equal(pending.spawns[0]?.enemy.definitionId, 'entity.enemy.generated_tunnel.basic.v0');
  assert.equal(pending.lifecycle.outcomeKind, 'in_progress');
  assert.equal(pending.hashes.spawnOrderHash, GENERATED_TUNNEL_SMALL_ENCOUNTER_CONFIG.spawnOrderHash);
  assert.equal(pending.hashes.encounterHash, 'b0a61b9060f6cb3f');
  assert.equal(pending.hashes.replayHash, '5a3bf9685c280cec');

  const sameSeedSession = createMockRuntimeSession();
  sameSeedSession.initialize(sessionInput());
  const sameSeedPending = sameSeedSession.readEncounterDirector();
  assert.equal(sameSeedPending.hashes.spawnOrderHash, pending.hashes.spawnOrderHash);
  assert.deepEqual(
    sameSeedPending.spawns.map((spawn) => spawn.instanceId),
    pending.spawns.map((spawn) => spawn.instanceId),
  );

  const activated = session.requestEncounterTransition(activateRequest());
  assert.equal(activated.kind, 'runtime_session.encounter_transition_receipt.v0');
  assert.equal(activated.accepted, true);
  assert.equal(activated.status, 'accepted');
  assert.equal(activated.event?.kind, 'runtime_encounter.activated.v0');
  assert.equal(activated.before.state.status, 'pending');
  assert.equal(activated.after.state.status, 'active');
  assert.equal(activated.after.state.activeEnemyCount, 1);
  assert.equal(activated.after.state.pendingEnemyCount, 0);
  assert.equal(activated.after.spawns[0]?.status, 'spawned');
  assert.equal(activated.hashes.transitionHash, '0fc329ac95377fa8');
  assert.notEqual(activated.hashes.sessionHashAfter, activated.hashes.sessionHashBefore);
  assert.equal(session.readTelemetry().replayRecords.at(-1)?.kind, 'requestEncounterTransition');
});

void test('RuntimeSession encounter director syncs lifecycle clear/fail and restart reset', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const camera = session.createCamera(cameraRequest()).snapshot.camera;

  session.requestEncounterTransition(activateRequest());
  session.submitRuntimeActionIntent({
    kind: 'runtime_action_intent.v0',
    action: 'primary_fire',
    phase: 'pressed',
    camera,
    tick: 7,
    source: 'programmatic',
    pressed: true,
  });

  const cleared = session.requestEncounterTransition(syncLifecycleRequest());
  assert.equal(cleared.accepted, true);
  assert.equal(cleared.event?.kind, 'runtime_encounter.lifecycle_synced.v0');
  assert.equal(cleared.after.state.status, 'cleared');
  assert.equal(cleared.after.state.clearedReason, 'all_enemies_defeated');
  assert.equal(cleared.after.state.defeatedEnemyCount, 1);
  assert.equal(cleared.after.spawns[0]?.status, 'defeated');
  assert.equal(cleared.after.lifecycle.outcomeKind, 'won');
  assert.equal(cleared.after.lifecycle.enemyDead, true);
  assert.equal(cleared.hashes.transitionHash, '2b0132ff232c7bd0');

  session.restart();
  const reset = session.readEncounterDirector();
  assert.equal(reset.state.status, 'pending');
  assert.equal(reset.state.revision, 0);
  assert.equal(reset.state.pendingEnemyCount, 1);
  assert.equal(reset.lifecycle.outcomeKind, 'in_progress');

  const failingSession = createMockRuntimeSession();
  failingSession.initialize(sessionInput());
  const failed = failingSession.requestEncounterTransition({
    kind: 'runtime_session.encounter_transition_request.v0',
    presetId: 'generated-tunnel-small-encounter',
    action: 'sync_lifecycle',
    lifecycleScenario: 'generated_tunnel_player_defeated',
  });
  assert.equal(failed.accepted, true);
  assert.equal(failed.after.state.status, 'failed');
  assert.equal(failed.after.state.failedReason, 'player_defeated');
  assert.equal(failed.after.lifecycle.outcomeKind, 'lost');
  assert.equal(failed.after.lifecycle.playerDead, true);
  assert.equal(failed.hashes.transitionHash, '3dfdeccbed7fc1f9');
});

void test('RuntimeSession encounter transition fails closed with typed rejection receipts', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const rejected = session.requestEncounterTransition({
    kind: 'runtime_session.encounter_transition_request.v0',
    presetId: 'unknown-encounter-preset',
    action: 'activate',
  } as unknown as EncounterTransitionRequest);

  assert.equal(rejected.accepted, false);
  assert.equal(rejected.status, 'rejected');
  assert.equal(rejected.rejectionReason, 'unknown_encounter_preset');
  assert.equal(rejected.event, undefined);
  assert.equal(rejected.before.state.status, 'pending');
  assert.equal(rejected.after.state.status, 'pending');
  assert.equal(rejected.after.state.revision, rejected.before.state.revision);
  assert.equal(rejected.after.spawns[0]?.status, 'pending');
  assert.ok(rejected.hashes.transitionHash.length > 0);
  assert.equal('payload' in rejected, false);
  assert.equal(session.readTelemetry().replayRecords.at(-1)?.kind, 'requestEncounterTransition');
});
