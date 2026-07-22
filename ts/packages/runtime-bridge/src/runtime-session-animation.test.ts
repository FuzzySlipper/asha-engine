import { test } from 'node:test';
import assert from 'node:assert/strict';

import { cameraHandle } from '@asha/contracts';

import { createMockRuntimeSession } from './reference.js';
import { buildRuntimeSessionAnimationControllerTargetFrame } from './runtime-session-animation.js';

function sessionInput() {
  return {
    sessionId: 'runtime-session.animation-intent.reference',
    seed: 17,
    project: {
      gameId: 'asha-demo',
      workspaceId: 'workspace.local',
    },
  };
}

void test('RuntimeSession animation intent selects projection-only clips from lifecycle state', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());

  const active = session.readAnimationIntent();
  assert.equal(active.kind, 'runtime_session.animation_intent.v0');
  assert.equal(active.asset.asset, 'mesh-animation/kenney-retro-character-medium');
  assert.equal(active.selectedClipId, 'run');
  assert.equal(active.selectionReason, 'enemy_active_visual_run');
  assert.equal(active.authority.projectionOnly, true);
  assert.deepEqual(active.authority.readSets, ['lifecycle.player.health', 'lifecycle.enemy.health']);
  assert.ok(active.nonClaims.includes('not_mixer_authority'));
  assert.equal(active.frame.ops[0]?.op, 'defineAnimatedMesh');
  assert.equal(active.frame.ops[1]?.op, 'createAnimatedMeshInstance');
  const activePlaybackOp = active.frame.ops[2];
  assert.equal(activePlaybackOp?.op, 'setAnimatedMeshPlayback');
  assert.equal(
    activePlaybackOp?.op === 'setAnimatedMeshPlayback' && activePlaybackOp.playback.action === 'play'
      ? activePlaybackOp.playback.clip
      : null,
    'run',
  );

  session.submitRuntimeActionIntent({
    kind: 'runtime_action_intent.v0',
    action: 'primary_fire',
    phase: 'pressed',
    camera: cameraHandle(1),
    tick: 7,
    source: 'programmatic',
    pressed: true,
  });

  const defeated = session.readAnimationIntent();
  assert.equal(defeated.selectedClipId, 'idle');
  assert.equal(defeated.selectionReason, 'enemy_defeated_visual_idle');
  assert.equal(defeated.playback.action, 'play');
  assert.equal(defeated.playback.clip, 'idle');
  assert.ok(defeated.nonClaims.includes('not_gameplay_outcome_authority'));
});

void test('controller target frame binds the admitted animated-mesh resource identity', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const frame = buildRuntimeSessionAnimationControllerTargetFrame(session.readAnimationIntent(), {
    asset: 'mesh/authored-character',
    contentHash: 'bd44b76d0424bd16',
    clipIds: ['idle', 'jump'],
  });
  const define = frame.ops.find((operation) => operation.op === 'defineAnimatedMesh');
  const create = frame.ops.find((operation) => operation.op === 'createAnimatedMeshInstance');
  assert.equal(define?.op, 'defineAnimatedMesh');
  if (define?.op === 'defineAnimatedMesh') {
    assert.equal(define.asset.asset, 'mesh/authored-character');
    assert.equal(define.asset.contentHash, 'bd44b76d0424bd16');
    assert.deepEqual(define.asset.clips.map((clip) => clip.id), ['idle', 'jump']);
  }
  assert.equal(create?.op, 'createAnimatedMeshInstance');
  if (create?.op === 'createAnimatedMeshInstance') {
    assert.equal(create.instance.asset, 'mesh/authored-character');
  }
  assert.equal(frame.ops.some((operation) => operation.op === 'setAnimatedMeshPlayback'), false);
});
