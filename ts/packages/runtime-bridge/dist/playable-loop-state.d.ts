import type { RuntimeSessionEcrpCapabilityState, RuntimeSessionFacade } from './runtime-session.js';
export interface RuntimeSessionPlayableLoopStateRequest {
    readonly shell?: {
        readonly paused?: boolean;
        readonly menuMode?: 'closed' | 'paused' | 'options' | 'exit' | string;
    };
    readonly unavailableReason?: string;
}
export interface RuntimeSessionPlayableLoopHealthState {
    readonly current: number;
    readonly max: number;
    readonly dead: boolean;
    readonly percent: number;
}
export interface RuntimeSessionPlayableLoopState {
    readonly kind: 'runtime_session.playable_loop_state.v0';
    readonly status: 'runtime_authority' | 'missing_backend';
    readonly sequenceId: number;
    readonly tick: number;
    readonly sessionHash: string;
    readonly currentEpoch: {
        readonly restartCount: number;
        readonly replayRecordStartIndex: number;
        readonly replayRecordCount: number;
        readonly source: 'after_last_restart_record';
    };
    readonly counters: {
        readonly actionTick: number;
        readonly shotsFired: number;
        readonly hits: number;
        readonly remainingTargets: number;
        readonly totalTargets: number;
    };
    readonly health: {
        readonly player: RuntimeSessionPlayableLoopHealthState;
        readonly enemy: RuntimeSessionPlayableLoopHealthState;
    };
    readonly commands: {
        readonly canFire: boolean;
        readonly canRestart: boolean;
        readonly blockedReasons: readonly RuntimeSessionPlayableLoopCommandBlockReason[];
    };
    readonly shell: {
        readonly paused: boolean;
        readonly menuMode: string;
    };
    readonly target: Extract<RuntimeSessionEcrpCapabilityState, {
        readonly kind: 'renderProjection';
    }>['target'] | null;
    readonly diagnostics: readonly {
        readonly code: 'missing_runtime_session';
        readonly message: string;
    }[];
    readonly nonClaims: readonly [
        'not_ui_authority',
        'not_replay_history_counter',
        'not_demo_local_authority'
    ];
}
export type RuntimeSessionPlayableLoopCommandBlockReason = 'missing_backend' | 'paused' | 'player_dead' | 'target_defeated';
type RuntimeSessionPlayableLoopFacade = Pick<RuntimeSessionFacade, 'readEcrpRuntimeReadout' | 'readLifecycleStatus' | 'readTelemetry'>;
export declare function readRuntimeSessionPlayableLoopState(session: RuntimeSessionPlayableLoopFacade | null, request?: RuntimeSessionPlayableLoopStateRequest): RuntimeSessionPlayableLoopState;
export {};
//# sourceMappingURL=playable-loop-state.d.ts.map