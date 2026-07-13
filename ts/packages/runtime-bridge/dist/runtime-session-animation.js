import { renderHandle, } from '@asha/contracts';
import { renderFrameHashRecord, stableHash } from './runtime-session-hash.js';
const ANIMATED_MESH_ASSET = {
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
export function buildRuntimeSessionAnimationIntentReadout(input) {
    const selection = selectRuntimeSessionAnimationClip(input.lifecycleState);
    const playback = {
        action: 'play',
        clip: selection.clipId,
        loop: 'repeat',
        speed: 1,
        weight: 1,
        restart: false,
        fadeSeconds: 0.1,
    };
    const frame = {
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
export function buildRuntimeSessionAnimationControllerTargetFrame(readout) {
    return {
        ops: readout.frame.ops.filter((operation) => operation.op !== 'setAnimatedMeshPlayback'),
    };
}
function selectRuntimeSessionAnimationClip(lifecycleState) {
    if (lifecycleState.player.dead) {
        return { clipId: 'idle', reason: 'player_defeated_visual_idle' };
    }
    if (lifecycleState.enemy.dead) {
        return { clipId: 'idle', reason: 'enemy_defeated_visual_idle' };
    }
    return { clipId: 'run', reason: 'enemy_active_visual_run' };
}
//# sourceMappingURL=runtime-session-animation.js.map