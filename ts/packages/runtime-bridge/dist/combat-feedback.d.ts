import type { CameraProjectionSnapshot } from '@asha/contracts';
import type { CombatReadoutScenario, CombatRuntimeReadout } from './combat-readout.js';
import type { RuntimeActionIntentEnvelope, RuntimeActionIntentRejection, RuntimeActionIntentStatus } from './runtime-action.js';
export type CombatFeedbackProjectionKind = 'combat_feedback_projection.v0';
export type CombatFeedbackScenario = CombatReadoutScenario | 'runtime_action_unsupported';
export type CombatFeedbackTraceResult = 'hit' | 'miss' | 'not_fired';
export type CombatFeedbackMarkerTone = 'hit' | 'blocked' | 'inactive';
export type CombatFeedbackHudTone = 'info' | 'warning' | 'danger';
export type CombatFeedbackFixturePath = 'harness/fixtures/combat-feedback/generated-tunnel-hit-feedback.snapshot.txt' | 'harness/fixtures/combat-feedback/generated-tunnel-miss-feedback.snapshot.txt' | null;
export interface CombatFeedbackIntentInput {
    readonly envelope: RuntimeActionIntentEnvelope;
    readonly accepted: boolean;
    readonly status: RuntimeActionIntentStatus;
    readonly rejection: RuntimeActionIntentRejection | null;
}
export interface CombatFeedbackActionReceiptInput extends CombatFeedbackIntentInput {
    readonly sequenceId: number;
    readonly combatReadout: CombatRuntimeReadout | null;
}
export interface CombatFeedbackProjectionInput extends CombatFeedbackIntentInput {
    readonly sequenceId: number;
    readonly combatReadout: CombatRuntimeReadout | null;
    readonly camera?: CameraProjectionSnapshot | null;
}
export interface CombatFeedbackIntentProjection {
    readonly kind: 'combat_feedback.intent.v0';
    readonly action: RuntimeActionIntentEnvelope['action'];
    readonly phase: RuntimeActionIntentEnvelope['phase'];
    readonly tick: number;
    readonly source: RuntimeActionIntentEnvelope['source'];
    readonly accepted: boolean;
    readonly status: RuntimeActionIntentStatus;
    readonly rejectionReason: RuntimeActionIntentRejection['reason'] | null;
}
export interface CombatFeedbackTraceProjection {
    readonly kind: 'combat_feedback.trace.v0';
    readonly result: CombatFeedbackTraceResult;
    readonly shooter: number | null;
    readonly target: number | null;
    readonly reason: 'geometryBlocked' | 'noTarget' | 'intent_not_accepted' | null;
    readonly distance: number | null;
    readonly origin: readonly [number, number, number] | null;
    readonly direction: readonly [number, number, number] | null;
    readonly endpoint: readonly [number, number, number] | null;
    readonly cameraProjectionHash: string | null;
}
export interface CombatFeedbackMarkerProjection {
    readonly kind: 'combat_feedback.marker.v0';
    readonly visible: boolean;
    readonly tone: CombatFeedbackMarkerTone;
    readonly label: 'Hit' | 'Blocked' | 'Unavailable';
    readonly durationMs: 0 | 120 | 160;
    readonly screenAnchor: {
        readonly space: 'normalized';
        readonly x: 0.5;
        readonly y: 0.5;
    };
}
export interface CombatFeedbackNotificationProjection {
    readonly id: string;
    readonly tone: CombatFeedbackHudTone;
    readonly text: string;
    readonly entity: number | null;
    readonly eventKind: string;
}
export interface CombatFeedbackHudStatusDescriptor {
    readonly id: string;
    readonly tone: CombatFeedbackHudTone;
    readonly text: string;
}
export interface CombatFeedbackHudProjection {
    readonly reticle: {
        readonly tone: CombatFeedbackMarkerTone;
        readonly pulseMs: 0 | 120 | 160;
        readonly label: CombatFeedbackMarkerProjection['label'];
    };
    readonly status: readonly CombatFeedbackHudStatusDescriptor[];
    readonly ammo: number | null;
    readonly cooldownTicksRemaining: number | null;
}
export interface CombatFeedbackProjection {
    readonly kind: CombatFeedbackProjectionKind;
    readonly sequenceId: number;
    readonly scenario: CombatFeedbackScenario;
    readonly intent: CombatFeedbackIntentProjection;
    readonly trace: CombatFeedbackTraceProjection;
    readonly marker: CombatFeedbackMarkerProjection;
    readonly notifications: readonly CombatFeedbackNotificationProjection[];
    readonly hud: CombatFeedbackHudProjection;
    readonly health: CombatRuntimeReadout['health'];
    readonly debug: {
        readonly fixturePath: CombatFeedbackFixturePath;
        readonly combatReplayHash: string | null;
        readonly healthHash: string | null;
        readonly cameraProjectionHash: string | null;
        readonly viewport: {
            readonly width: number;
            readonly height: number;
        } | null;
    };
    readonly hashes: {
        readonly traceHash: string;
        readonly markerHash: string;
        readonly notificationHash: string;
        readonly projectionHash: string;
    };
    readonly nonClaims: readonly [
        'not_combat_authority',
        'not_renderer_state',
        'not_ui_state',
        'not_animation_or_audio'
    ];
}
export declare function buildCombatFeedbackProjectionFromReceipt(receipt: CombatFeedbackActionReceiptInput, camera?: CameraProjectionSnapshot | null): CombatFeedbackProjection;
export declare function buildCombatFeedbackProjection(input: CombatFeedbackProjectionInput): CombatFeedbackProjection;
export declare function defaultCombatFeedbackIntent(input: {
    readonly camera: RuntimeActionIntentEnvelope['camera'];
    readonly tick: number;
}): CombatFeedbackIntentInput;
//# sourceMappingURL=combat-feedback.d.ts.map