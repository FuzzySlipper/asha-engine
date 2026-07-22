import {
  renderHandle,
  type AnimatedMeshAsset,
  type AnimatedMeshPlaybackCommand,
  type RenderDiff,
  type RenderFrameDiff,
} from '@asha/contracts';
import type {
  RuntimeSessionAnimationIntentInput,
  RuntimeSessionAnimationIntentReadout,
  RuntimeSessionAnimationSelectionReason,
  RuntimeSessionLifecycleState,
} from '@asha/runtime-session';
import { renderFrameHashRecord, stableHash } from './runtime-session-hash.js';

export type {
  RuntimeSessionAnimationIntentAuthority,
  RuntimeSessionAnimationIntentInput,
  RuntimeSessionAnimationIntentNonClaim,
  RuntimeSessionAnimationIntentReadout,
  RuntimeSessionAnimationSelectionReason,
} from '@asha/runtime-session';

const ANIMATED_MESH_ASSET: AnimatedMeshAsset = {
  asset: 'mesh-animation/kenney-retro-character-medium',
  runtimeFormat: 'glb',
  contentHash: 'sha256:c71255a41c0373f0d2ef52593369d5fd9d2f6220ae548aff8cd6bf5edb403674',
  clips: [
    { id: 'idle', name: 'Idle', durationSeconds: 1.04166662693024 },
    { id: 'run', name: 'Run', durationSeconds: 0.666666686534882 },
    { id: 'jump', name: 'Jump', durationSeconds: 0.5 },
  ],
  defaultClip: 'idle',
  materialSlots: [{ slot: 0, material: 'material/kenney-human-male-a' }],
  bounds: {
    min: [-0.0180905014276505, -0.00514235720038414, 0.00000684113911120221],
    max: [0.018095325678587, 0.00533908000215888, 0.0376536995172501],
  },
};

const INSTANCE_HANDLE = renderHandle(4100);

export function buildRuntimeSessionAnimationIntentReadout(
  input: RuntimeSessionAnimationIntentInput,
): RuntimeSessionAnimationIntentReadout {
  const selection = selectRuntimeSessionAnimationClip(input.lifecycleState);
  const playback: AnimatedMeshPlaybackCommand = {
    action: 'play',
    clip: selection.clipId,
    loop: 'repeat',
    speed: 1,
    weight: 1,
    restart: false,
    fadeSeconds: 0.1,
  };
  const frame: RenderFrameDiff = {
    ops: [
      { op: 'defineAnimatedMesh', asset: ANIMATED_MESH_ASSET },
      {
        op: 'createAnimatedMeshInstance',
        handle: INSTANCE_HANDLE,
        parent: null,
        instance: {
          asset: ANIMATED_MESH_ASSET.asset,
          transform: {
            translation: [0, 0, -2.5],
            rotation: [0, 0, 0, 1],
            scale: [40, 40, 40],
          },
          materialOverrides: [],
          playback: null,
          metadata: {
            source: null,
            tags: [],
            label: 'runtime-session animated enemy visual',
          },
        },
      },
      {
        op: 'setAnimatedMeshPlayback',
        handle: INSTANCE_HANDLE,
        playback,
      },
    ],
  };
  return {
    kind: 'runtime_session.animation_intent.v0',
    sequenceId: input.sequenceId,
    tick: input.tick,
    asset: ANIMATED_MESH_ASSET,
    instanceHandle: INSTANCE_HANDLE,
    selectedClipId: selection.clipId,
    selectionReason: selection.reason,
    playback,
    frame,
    authority: {
      source: 'runtime_session_lifecycle',
      readSets: ['lifecycle.player.health', 'lifecycle.enemy.health'],
      projectionOnly: true,
    },
    nonClaims: [
      'not_mixer_authority',
      'not_gameplay_outcome_authority',
      'not_collision_authority',
      'not_replay_authority',
    ],
    intentHash: stableHash({
      kind: 'runtime_session.animation_intent.v0',
      sequenceId: input.sequenceId,
      tick: input.tick,
      selectedClipId: selection.clipId,
      selectionReason: selection.reason,
      frame: renderFrameHashRecord(frame),
    }),
  };
}

/**
 * Reuses the compatibility readout's hash-pinned mesh bootstrap without
 * applying its direct clip command. Controller-driven consumers should apply
 * this frame once, then realize G1 animation operations through AshaAnimationHost.
 */
export function buildRuntimeSessionAnimationControllerTargetFrame(
  readout: RuntimeSessionAnimationIntentReadout,
  target?: {
    readonly asset: string;
    readonly contentHash: string;
    readonly clipIds: readonly string[];
  },
): RenderFrameDiff {
  if (target === undefined) {
    return {
      ops: readout.frame.ops.filter((operation) => operation.op !== 'setAnimatedMeshPlayback'),
    };
  }
  const requestedClips = new Set(target.clipIds);
  const clips = readout.asset.clips.filter((clip) => requestedClips.has(clip.id));
  if (clips.length !== requestedClips.size) {
    const available = new Set(readout.asset.clips.map((clip) => clip.id));
    const missing = target.clipIds.filter((clipId) => !available.has(clipId));
    throw new Error(`animation target is missing runtime clip metadata for ${missing.join(', ')}`);
  }
  const defaultClip = readout.asset.defaultClip !== null && requestedClips.has(readout.asset.defaultClip)
    ? readout.asset.defaultClip
    : clips[0]?.id;
  if (defaultClip === undefined) {
    throw new Error('animation target must retain at least one runtime clip');
  }
  const asset: AnimatedMeshAsset = {
    ...readout.asset,
    asset: target.asset,
    contentHash: target.contentHash,
    clips,
    defaultClip,
  };
  const ops: RenderDiff[] = [];
  for (const operation of readout.frame.ops) {
      if (operation.op === 'setAnimatedMeshPlayback') continue;
      if (operation.op === 'defineAnimatedMesh') {
        ops.push({ ...operation, asset });
        continue;
      }
      if (operation.op === 'createAnimatedMeshInstance') {
        ops.push({
          ...operation,
          instance: { ...operation.instance, asset: target.asset },
        });
        continue;
      }
      ops.push(operation);
  }
  return { ops };
}

function selectRuntimeSessionAnimationClip(
  lifecycleState: RuntimeSessionLifecycleState,
): { readonly clipId: 'idle' | 'run'; readonly reason: RuntimeSessionAnimationSelectionReason } {
  if (lifecycleState.player.dead) {
    return { clipId: 'idle', reason: 'player_defeated_visual_idle' };
  }
  if (lifecycleState.enemy.dead) {
    return { clipId: 'idle', reason: 'enemy_defeated_visual_idle' };
  }
  return { clipId: 'run', reason: 'enemy_active_visual_run' };
}
