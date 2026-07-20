import { test } from 'node:test';
import assert from 'node:assert/strict';

import type { CameraCreateRequest } from '@asha/contracts';
import { RuntimeBridgeError } from './index.js';
import { createMockRuntimeSession } from './reference.js';

function sessionInput() {
  return {
    sessionId: 'runtime-session.lifecycle.reference',
    seed: 17,
    project: {
      gameId: 'asha-demo',
      workspaceId: 'workspace.local',
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

void test('RuntimeSession lifecycle readout tracks enemy death from typed combat authority', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const camera = session.createCamera(cameraRequest()).snapshot.camera;

  const initial = session.readLifecycleStatus();
  assert.equal(initial.kind, 'runtime_session.lifecycle_status.v0');
  assert.equal(initial.scenario, 'current_session');
  assert.equal(initial.player.dead, false);
  assert.equal(initial.player.health.current, 100);
  assert.equal(initial.enemy.dead, false);
  assert.equal(initial.enemy.health.current, 40);
  assert.equal(initial.outcome.kind, 'in_progress');
  assert.equal(initial.outcome.terminal, false);
  assert.equal(initial.restart.eligible, true);
  assert.equal(initial.reset.resetHash.startsWith('fnv1a64:'), true);
  assert.equal(initial.hashes.lifecycleHash.startsWith('fnv1a64:'), true);

  const rejectedRestart = session.requestSessionRestart({
    kind: 'runtime.restart_session_intent',
    source: 'hud_menu',
    requireTerminal: true,
  });
  assert.equal(rejectedRestart.accepted, false);
  assert.equal(rejectedRestart.status, 'rejected');
  assert.equal(rejectedRestart.rejection?.reason, 'session_not_terminal');
  assert.equal(rejectedRestart.statusAfter.outcome.kind, 'in_progress');
  assert.equal(session.readTelemetry().replayRecords.at(-1)?.kind, 'requestSessionRestart');

  const fire = session.submitRuntimeActionIntent({
    kind: 'runtime_action_intent.v0',
    action: 'primary_fire',
    phase: 'pressed',
    camera,
    tick: 7,
    source: 'programmatic',
    pressed: true,
  });
  assert.equal(fire.accepted, true);
  assert.equal(fire.combatReadout?.health[0]?.dead, true);

  const defeated = session.readLifecycleStatus();
  assert.equal(defeated.outcome.kind, 'won');
  assert.equal(defeated.outcome.reason, 'enemy_defeated');
  assert.equal(defeated.enemy.dead, true);
  assert.equal(defeated.enemy.health.current, 0);
  assert.equal(defeated.enemy.health.max, 40);
  assert.equal(defeated.events[0]?.kind, 'runtime_lifecycle.enemy_defeated.v0');
  assert.equal(defeated.events[0]?.tick, 7);
  assert.equal(defeated.hashes.lifecycleHash, 'fnv1a64:5fbf190733451da1');
  assert.equal(defeated.hashes.enemyHealthHash, 'fnv1a64:380624a28ba625b3');
  assert.equal(session.readTelemetry().replayRecords.some((record) => record.kind === 'lifecycleDeath'), true);
});

void test('RuntimeSession lifecycle exposes deterministic player defeat fixture', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());

  const playerDefeated = session.readLifecycleStatus({ scenario: 'generated_tunnel_player_defeated' });

  assert.equal(playerDefeated.outcome.kind, 'lost');
  assert.equal(playerDefeated.outcome.reason, 'player_defeated');
  assert.equal(playerDefeated.player.dead, true);
  assert.equal(playerDefeated.player.health.current, 0);
  assert.equal(playerDefeated.enemy.dead, false);
  assert.equal(playerDefeated.events[0]?.kind, 'runtime_lifecycle.player_defeated.v0');
  assert.equal(playerDefeated.events[0]?.reason, 'fixture_player_damage');
  assert.equal(playerDefeated.hashes.lifecycleHash, 'fnv1a64:32322a108d4f2767');
  assert.equal(playerDefeated.hashes.playerHealthHash, 'fnv1a64:4c9316192318edb7');
});

void test('RuntimeSession typed restart intent resets lifecycle deterministically after terminal state', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const camera = session.createCamera(cameraRequest()).snapshot.camera;
  const initial = session.readLifecycleStatus();

  session.submitRuntimeActionIntent({
    kind: 'runtime_action_intent.v0',
    action: 'primary_fire',
    phase: 'pressed',
    camera,
    tick: 7,
    source: 'programmatic',
    pressed: true,
  });
  const terminal = session.readLifecycleStatus();
  assert.equal(terminal.outcome.terminal, true);

  const receipt = session.requestSessionRestart({
    kind: 'runtime.restart_session_intent',
    source: 'hud_menu',
    requireTerminal: true,
    expectedSessionHash: terminal.sessionHash,
  });

  assert.equal(receipt.kind, 'runtime_session.restart_receipt.v0');
  assert.equal(receipt.accepted, true);
  assert.equal(receipt.status, 'accepted');
  assert.equal(receipt.rejection, null);
  assert.equal(receipt.statusBefore.outcome.kind, 'won');
  assert.equal(receipt.statusAfter.outcome.kind, 'in_progress');
  assert.equal(receipt.statusAfter.player.health.current, 100);
  assert.equal(receipt.statusAfter.enemy.health.current, 40);
  assert.equal(receipt.statusAfter.hashes.lifecycleHash, initial.hashes.lifecycleHash);
  assert.equal(receipt.resetHash, initial.reset.resetHash);
  assert.equal(receipt.restart?.tick, 0);
  assert.equal(receipt.restart?.restartCount, 1);
  assert.equal(session.readTelemetry().replayRecords.at(-1)?.kind, 'restart');
});

void test('RuntimeSession restart intent rejects stale session hashes and malformed inputs', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());

  const receipt = session.requestSessionRestart({
    kind: 'runtime.restart_session_intent',
    source: 'programmatic',
    expectedSessionHash: 'fnv1a64:stale',
  });

  assert.equal(receipt.accepted, false);
  assert.equal(receipt.rejection?.reason, 'session_hash_mismatch');
  assert.equal(receipt.statusAfter.outcome.kind, 'in_progress');
  assert.equal(session.readTelemetry().restartCount, 0);
  assert.equal(session.readTelemetry().replayRecords.at(-1)?.kind, 'requestSessionRestart');

  assert.throws(
    () =>
      session.requestSessionRestart({
        kind: 'runtime.restart_session_intent',
        source: 'programmatic',
        expectedSessionHash: '',
      }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
});
