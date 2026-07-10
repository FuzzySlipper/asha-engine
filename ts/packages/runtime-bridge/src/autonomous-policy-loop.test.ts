import { test } from 'node:test';
import assert from 'node:assert/strict';

import type { CameraCreateRequest } from '@asha/contracts';
import { createMockRuntimeSession } from './reference.js';

function sessionInput() {
  return {
    sessionId: 'runtime-session.autonomous-policy.reference',
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
    initialPose: {
      position: [1, 1.5, 1],
      yawDegrees: 180,
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
}

void test('RuntimeSession runs deterministic autonomous enemy policy ticks through typed proposals', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const camera = session.createCamera(cameraRequest()).snapshot.camera;

  const first = session.runAutonomousPolicyTick({
    targetCamera: camera,
    policySource: 'export const policy = (view) => view;',
  });

  assert.equal(first.kind, 'runtime_session.autonomous_policy_tick.v0');
  assert.equal(first.loopId, 'generated_tunnel_enemy_policy_loop.v0');
  assert.equal(first.tick, 1);
  assert.equal(first.nav.pathHash, '09ed0284f7c175e1');
  assert.equal(first.nav.outcome, 'reached');
  assert.equal(first.nav.pathLength, 9);
  assert.deepEqual(
    first.policy.proposalFrame.proposals.map((proposal) => proposal.kind),
    ['enemy_policy.move_toward_target.v0', 'enemy_policy.primary_fire_intent.v0'],
  );
  assert.equal(first.policy.sourceChecked, true);
  assert.deepEqual(first.policy.sourceDiagnostics, []);
  assert.deepEqual(first.policy.proposalValidationDiagnostics, []);
  assert.equal(first.proposalSummary.acceptedProposalCount, 2);
  assert.equal(first.proposalSummary.unsupportedProposalCount, 0);
  assert.equal(first.proposalSummary.rejectedProposalCount, 0);
  assert.equal(first.commandSummary.acceptedRuntimeActionCount, 1);
  assert.equal(first.commandSummary.rejectedRuntimeActionCount, 0);
  assert.equal(first.commandSummary.acceptedCommandCount, 0);
  assert.equal(first.commandSummary.rejectedCommandCount, 0);
  assert.equal(first.movementSummary?.status, 'accepted');
  assert.equal(first.movementSummary?.reason, null);
  assert.equal(first.movementSummary?.authoritySource, 'seeded_from_request');
  assert.equal(first.movementSummary?.authorityTransport, 'reference_bridge');
  assert.match(first.movementSummary?.transformHash ?? '', /^fnv1a64:[0-9a-f]{16}$/);
  assert.deepEqual(first.movementSummary?.nextWaypoint, [3.844, 1.024, 7.688]);
  assert.equal(first.combatSummary?.status, 'accepted');
  assert.equal(first.combatSummary?.outcome?.kind, 'hit');
  assert.match(first.combatSummary?.healthHash ?? '', /^fnv1a64:[0-9a-f]{16}$/);
  assert.match(first.combatSummary?.replayHash ?? '', /^fnv1a64:[0-9a-f]{16}$/);
  assert.equal(first.proposalReceipts[0]?.status, 'accepted');
  assert.equal(first.proposalReceipts[0]?.rejection, null);
  assert.equal(first.proposalReceipts[1]?.status, 'accepted');
  assert.equal(first.proposalReceipts[1]?.actionReceipt?.accepted, true);
  assert.deepEqual(first.proposalReceipts[1]?.actionReceipt?.combatReadout?.health[0], {
    entity: 10,
    current: 90,
    max: 100,
    dead: false,
  });
  assert.equal(first.replay.lastRecordKind, 'runAutonomousPolicyTick');
  assert.equal(first.replay.recordHashes.every((hash) => hash.startsWith('fnv1a64:')), true);
  assert.ok(first.tickHash.startsWith('fnv1a64:'));
  assert.ok(first.replay.recordCount >= 5);
  assert.ok(first.nonClaims.includes('not_generic_event_bus'));
  assert.notEqual(first.sessionHashAfter, first.sessionHashBefore);
  assert.equal(
    session.readTelemetry().replayRecords.some((record) => record.kind === 'lifecycleDeath'),
    false,
  );
  const movedEnemy = session.readEcrpRuntimeReadout().entities.find(
    (entity) => entity.definitionStableId === 'actor/generated-tunnel-enemy',
  );
  const movedEnemyTransform = movedEnemy?.capabilities.find((capability) => capability.kind === 'transform');
  assert.equal(movedEnemyTransform?.kind, 'transform');
  assert.deepEqual(movedEnemyTransform?.position, [3.844, 1.024, 7.688]);
  assert.match(movedEnemyTransform?.stateHash ?? '', /^fnv1a64:[0-9a-f]{16}$/);

  const second = session.runAutonomousPolicyTick({ targetCamera: camera });

  assert.equal(second.tick, 2);
  assert.equal(second.policy.sourceChecked, false);
  assert.equal(second.movementSummary?.authoritySource, 'rust_entity_store');
  assert.equal(second.movementSummary?.authorityTransport, 'reference_bridge');
  assert.equal(second.replay.lastRecordKind, 'runAutonomousPolicyTick');
  assert.ok(second.replay.recordCount > first.replay.recordCount);
  assert.notEqual(second.tickHash, first.tickHash);
  assert.equal(
    session.readTelemetry().replayRecords.filter((record) => record.kind === 'runAutonomousPolicyTick').length,
    2,
  );
});

void test('RuntimeSession rejects autonomous policy proposals when source references forbidden capabilities', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const camera = session.createCamera(cameraRequest()).snapshot.camera;

  const readout = session.runAutonomousPolicyTick({
    targetCamera: camera,
    policySource: "Date.now(); Math.random(); fetch('/bad');",
  });

  assert.deepEqual(
    readout.policy.sourceDiagnostics.map((diagnostic) => diagnostic.token),
    ['Date', 'Math.random', 'fetch'],
  );
  assert.deepEqual(
    readout.proposalReceipts.map((receipt) => receipt.status),
    ['rejected', 'rejected'],
  );
  assert.deepEqual(
    readout.proposalReceipts.map((receipt) => receipt.rejection?.reason),
    ['policy_source_forbidden_capability', 'policy_source_forbidden_capability'],
  );
  assert.equal(readout.proposalSummary.acceptedProposalCount, 0);
  assert.equal(readout.proposalSummary.rejectedProposalCount, 2);
  assert.equal(readout.proposalSummary.unsupportedProposalCount, 0);
  assert.equal(readout.commandSummary.acceptedRuntimeActionCount, 0);
  assert.equal(readout.commandSummary.rejectedRuntimeActionCount, 0);
  assert.equal(readout.movementSummary, null);
  assert.equal(readout.combatSummary, null);
  assert.equal(readout.replay.lastRecordKind, 'runAutonomousPolicyTick');
  assert.equal(
    session.readTelemetry().replayRecords.some((record) => record.kind === 'submitRuntimeActionIntent'),
    false,
  );
});
