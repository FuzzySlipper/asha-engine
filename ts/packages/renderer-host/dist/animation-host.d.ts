import type { AnimationProjectionDiagnostic, AnimationProjectionHandle, AnimationProjectionReadout, PresentationFrameDiff, PresentationOriginRef, RenderHandle } from '@asha/contracts';
import type { AshaRendererAnimatedMeshProjection } from './animated-mesh-host.js';
export type AshaAnimationCueSignalDomain = 'audio' | 'particle';
export interface AshaAnimationClipCueDefinition {
    readonly cueId: string;
    readonly asset: string;
    readonly clip: string;
    readonly atSeconds: number;
    readonly signal: {
        readonly domain: AshaAnimationCueSignalDomain;
        readonly id: string;
    };
}
export interface AshaAnimationSampledCue {
    readonly kind: 'asha.animation.sampled_cue.v1';
    readonly cueId: string;
    readonly handle: AnimationProjectionHandle;
    readonly target: RenderHandle;
    readonly asset: string;
    readonly clip: string;
    readonly markerSeconds: number;
    readonly sampledAtSeconds: number;
    readonly signal: AshaAnimationClipCueDefinition['signal'];
    readonly origin: PresentationOriginRef | null;
    readonly replayScope: 'excludedFromReplayTruth';
    readonly authorityMutation: 'forbidden';
}
export interface AshaAnimationHostOptions {
    readonly cues?: readonly AshaAnimationClipCueDefinition[];
}
export interface AshaAnimationFrameReceipt {
    readonly applied: number;
    readonly diagnostics: readonly AnimationProjectionDiagnostic[];
    readonly cues: readonly AshaAnimationSampledCue[];
    readonly readout: AnimationProjectionReadout;
}
export declare class AshaAnimationHost {
    #private;
    constructor(projection: AshaRendererAnimatedMeshProjection, options?: AshaAnimationHostOptions);
    applyPresentation(frame: PresentationFrameDiff): AshaAnimationFrameReceipt;
    advance(deltaSeconds: number): AshaAnimationFrameReceipt;
    readout(): AnimationProjectionReadout;
    cleanup(): AshaAnimationFrameReceipt;
}
//# sourceMappingURL=animation-host.d.ts.map