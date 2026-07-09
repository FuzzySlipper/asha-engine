import { type AnimatedMeshAsset, type AnimatedMeshPlaybackCommand, type RenderFrameDiff } from '@asha/contracts';
import type { RuntimeSessionLifecycleState } from './runtime-session.js';
export interface RuntimeSessionAnimationIntentReadout {
    readonly kind: 'runtime_session.animation_intent.v0';
    readonly sequenceId: number;
    readonly tick: number;
    readonly asset: AnimatedMeshAsset;
    readonly instanceHandle: number;
    readonly selectedClipId: string;
    readonly selectionReason: RuntimeSessionAnimationSelectionReason;
    readonly playback: AnimatedMeshPlaybackCommand;
    readonly frame: RenderFrameDiff;
    readonly authority: RuntimeSessionAnimationIntentAuthority;
    readonly nonClaims: readonly RuntimeSessionAnimationIntentNonClaim[];
    readonly intentHash: string;
}
export type RuntimeSessionAnimationSelectionReason = 'enemy_active_visual_run' | 'enemy_defeated_visual_idle' | 'player_defeated_visual_idle';
export type RuntimeSessionAnimationIntentNonClaim = 'not_mixer_authority' | 'not_gameplay_outcome_authority' | 'not_collision_authority' | 'not_replay_authority';
export interface RuntimeSessionAnimationIntentAuthority {
    readonly source: 'runtime_session_lifecycle';
    readonly readSets: readonly ['lifecycle.player.health', 'lifecycle.enemy.health'];
    readonly projectionOnly: true;
}
export interface RuntimeSessionAnimationIntentInput {
    readonly sequenceId: number;
    readonly tick: number;
    readonly lifecycleState: RuntimeSessionLifecycleState;
}
export declare function buildRuntimeSessionAnimationIntentReadout(input: RuntimeSessionAnimationIntentInput): RuntimeSessionAnimationIntentReadout;
//# sourceMappingURL=runtime-session-animation.d.ts.map