import type { RenderFrameDiff } from '@asha/contracts';
export type CosmeticEffectKind = 'screen_flash' | 'hit_spark' | 'view_kick';
export type CosmeticEffectDiagnosticCode = 'invalidDuration' | 'invalidIntensity' | 'invalidStartTick' | 'missingEffectId';
export type CosmeticSource = {
    readonly kind: 'render_frame_diff';
    readonly renderOpCount: number;
    readonly renderOpKinds: readonly string[];
} | {
    readonly kind: 'local_ui_event';
    readonly eventId: string;
};
export type CosmeticEffectDescriptor = {
    readonly effectId: string;
    readonly kind: CosmeticEffectKind;
    readonly source: CosmeticSource;
    readonly startsAtTick: number;
    readonly durationTicks: number;
    readonly intensity: number;
    readonly color: readonly [number, number, number, number] | null;
    readonly anchor: readonly [number, number, number] | null;
    readonly replayScope: 'excluded_from_replay_truth';
};
export type CosmeticEffectViewModel = {
    readonly effectId: string;
    readonly kind: CosmeticEffectKind;
    readonly active: boolean;
    readonly progress: number;
    readonly opacity: number;
    readonly intensity: number;
    readonly color: readonly [number, number, number, number] | null;
    readonly anchor: readonly [number, number, number] | null;
};
export type CosmeticFrameViewModel = {
    readonly kind: 'cosmetic_frame_view_model.v0';
    readonly tick: number;
    readonly effects: readonly CosmeticEffectViewModel[];
    readonly diagnostics: readonly CosmeticEffectDiagnostic[];
    readonly nonAuthority: CosmeticNonAuthorityReadout;
};
export type CosmeticEffectDiagnostic = {
    readonly code: CosmeticEffectDiagnosticCode;
    readonly effectId: string | null;
    readonly detail: string;
};
export type CosmeticNonAuthorityReadout = {
    readonly kind: 'cosmetic_non_authority_readout.v0';
    readonly commandCount: 0;
    readonly replayRecordCount: 0;
    readonly authoritativeMutationCount: 0;
    readonly rendererBackendCoupling: false;
    readonly runtimeTruth: 'not_authoritative';
};
export type CosmeticAuthorityBoundary = {
    readonly packageRole: '@asha/cosmetic';
    readonly owns: readonly ['transient_effect_descriptors', 'local_view_models'];
    readonly consumes: readonly ['generated_render_frame_diff_descriptors', 'local_ui_events'];
    readonly doesNotProduce: readonly ['authority_commands', 'replay_records', 'state_mutations', 'renderer_backend_calls'];
};
export type ScreenFlashInput = {
    readonly effectId: string;
    readonly renderFrame: RenderFrameDiff;
    readonly startsAtTick: number;
    readonly durationTicks: number;
    readonly intensity: number;
    readonly color?: readonly [number, number, number, number] | null;
};
export type HitSparkInput = {
    readonly effectId: string;
    readonly sourceEventId: string;
    readonly startsAtTick: number;
    readonly durationTicks: number;
    readonly intensity: number;
    readonly anchor: readonly [number, number, number];
    readonly color?: readonly [number, number, number, number] | null;
};
export declare const COSMETIC_AUTHORITY_BOUNDARY: CosmeticAuthorityBoundary;
export declare const COSMETIC_NON_AUTHORITY_READOUT: CosmeticNonAuthorityReadout;
export declare function createScreenFlashDescriptor(input: ScreenFlashInput): CosmeticEffectDescriptor;
export declare function createHitSparkDescriptor(input: HitSparkInput): CosmeticEffectDescriptor;
export declare function projectCosmeticFrame(descriptors: readonly CosmeticEffectDescriptor[], tick: number): CosmeticFrameViewModel;
export declare function validateCosmeticEffectDescriptor(descriptor: CosmeticEffectDescriptor): readonly CosmeticEffectDiagnostic[];
export declare function readCosmeticAuthorityBoundary(): CosmeticAuthorityBoundary;
//# sourceMappingURL=effects.d.ts.map