import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  createGeneratedTunnelEnemyPolicyFixture,
  validateEnemyPolicySource,
} from '@asha/runtime-session';
import { createMockRuntimeSession } from './reference.js';

function sessionInput() {
  return {
    sessionId: 'runtime-session.enemy-policy.reference',
    seed: 17,
    project: {
      gameId: 'asha-demo',
      workspaceId: 'workspace.local',
    },
  };
}
void test('enemy policy fixture proposes movement and typed fire intent from read-only nav policy view', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const camera = session.createCamera({
    initialPose: { position: [2.5, 1.5, 1.5], yawDegrees: 180, pitchDegrees: 0 },
    projection: { fovYDegrees: 60, near: 0.1, far: 100 },
    viewport: { width: 1280, height: 720 },
  }).snapshot.camera;

  const fixture = createGeneratedTunnelEnemyPolicyFixture({
    tick: 11,
    nav: session.readNavPolicyView(),
    target: { camera },
  });

  assert.equal(fixture.kind, 'generated_tunnel_enemy_policy_fixture.v0');
  assert.ok(fixture.nonClaims.includes('not_authority'));
  assert.equal(fixture.view.readOnly, true);
  assert.equal(fixture.view.proposalOnly, true);
  assert.equal('mutate' in fixture.view, false);
  assert.equal('applyPath' in fixture.view.nav, false);
  assert.equal(fixture.frame.kind, 'enemy_policy_proposal_frame.v0');
  assert.equal(fixture.frame.tick, 11);
  assert.equal(fixture.frame.diagnostics.length, 0);
  assert.match(fixture.frame.proposalHash, /^[0-9a-f]{16}$/);

  const moveProposal = fixture.frame.proposals.find(
    (proposal) => proposal.kind === 'enemy_policy.move_toward_target.v0',
  );
  assert.ok(moveProposal);
  assert.equal(moveProposal.actor, 'generated-tunnel.enemy.1');
  assert.equal(moveProposal.pathHash, '09ed0284f7c175e1');
  assert.deepEqual(moveProposal.nextWaypoint, [3, 1, 8]);
  assert.equal(moveProposal.authority, 'rust_runtime_must_validate');

  const fireProposal = fixture.frame.proposals.find(
    (proposal) => proposal.kind === 'enemy_policy.primary_fire_intent.v0',
  );
  assert.ok(fireProposal);
  assert.equal(fireProposal.intent.kind, 'runtime_action_intent.v0');
  assert.equal(fireProposal.intent.source, 'enemy_policy');
  assert.equal(fireProposal.intent.tick, 11);
  assert.equal(fireProposal.intent.pressed, true);
  assert.equal(fireProposal.authority, 'rust_runtime_must_validate');

  const receipt = session.submitRuntimeActionIntent(fireProposal.intent);
  assert.equal(receipt.accepted, true);
  assert.equal(receipt.status, 'accepted');
  assert.equal(receipt.rejection, null);
  assert.equal(receipt.envelope.source, 'enemy_policy');
  assert.equal(receipt.combatReadout?.outcome.kind, 'hit');
  assert.deepEqual(receipt.combatReadout?.health[0], {
    entity: 10,
    current: 90,
    max: 100,
    dead: false,
  });
  assert.equal('payload' in receipt, false);
});
void test('enemy policy fixture records proposal diagnostics without mutating authority', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const camera = session.createCamera({
    initialPose: { position: [2.5, 1.5, 1.5], yawDegrees: 180, pitchDegrees: 0 },
    projection: { fovYDegrees: 60, near: 0.1, far: 100 },
    viewport: { width: 1280, height: 720 },
  }).snapshot.camera;

  const fixture = createGeneratedTunnelEnemyPolicyFixture({
    tick: 12,
    nav: {
      ...session.readNavPolicyView(),
      latestPath: session.queryNavPath({ scenario: 'generated_tunnel_no_path' }),
    },
    target: { camera, position: [100, 1, 100] },
    combat: { lineOfSight: 'blocked', primaryFireRangeUnits: 4 },
  });

  assert.deepEqual(fixture.frame.proposals, []);
  assert.deepEqual(
    fixture.frame.diagnostics.map((diagnostic) => diagnostic.code),
    ['blocked_nav_path', 'target_out_of_range'],
  );
  assert.equal(session.readTelemetry().replayRecords.at(-1)?.kind, 'createCamera');
});

void test('enemy policy source validator rejects forbidden capabilities', () => {
  const diagnostics = validateEnemyPolicySource(`
    const now = Date.now();
    const roll = Math.random();
    fetch('/state');
    window.location.href;
    const fs = await import('node:fs');
    const escape = Function('return process')();
  `);

  assert.deepEqual(
    diagnostics.map((diagnostic) => `${diagnostic.capability}:${diagnostic.token}`),
    [
      'clock:Date',
      'random:Math.random',
      'network:fetch',
      'dom:window',
      'filesystem:node:fs',
      'process:process',
      'dynamic_code:Function',
      'module_import:import(',
    ],
  );

  assert.deepEqual(validateEnemyPolicySource('export const policy = (view) => [];'), []);
});
