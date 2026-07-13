import { type RenderFrameDiff } from '@asha/contracts';
import type { RuntimeSessionAnimationIntentInput, RuntimeSessionAnimationIntentReadout } from '@asha/runtime-session';
export type { RuntimeSessionAnimationIntentAuthority, RuntimeSessionAnimationIntentInput, RuntimeSessionAnimationIntentNonClaim, RuntimeSessionAnimationIntentReadout, RuntimeSessionAnimationSelectionReason, } from '@asha/runtime-session';
export declare function buildRuntimeSessionAnimationIntentReadout(input: RuntimeSessionAnimationIntentInput): RuntimeSessionAnimationIntentReadout;
/**
 * Reuses the compatibility readout's hash-pinned mesh bootstrap without
 * applying its direct clip command. Controller-driven consumers should apply
 * this frame once, then realize G1 animation operations through AshaAnimationHost.
 */
export declare function buildRuntimeSessionAnimationControllerTargetFrame(readout: RuntimeSessionAnimationIntentReadout): RenderFrameDiff;
//# sourceMappingURL=runtime-session-animation.d.ts.map