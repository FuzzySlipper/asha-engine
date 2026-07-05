import type { RuntimeSessionEcrpProjectDiagnostic, RuntimeSessionEcrpProjectLoadInput, RuntimeSessionEcrpProjectState, RuntimeSessionEcrpReadout, RuntimeSessionIdentity, RuntimeSessionInitializeInput, RuntimeSessionLifecycleState } from './runtime-session.js';
export declare function defaultRuntimeSessionEcrpProjectLoadInput(input: RuntimeSessionInitializeInput): RuntimeSessionEcrpProjectLoadInput;
export declare function validateEcrpProjectLoadInput(input: RuntimeSessionEcrpProjectLoadInput): readonly RuntimeSessionEcrpProjectDiagnostic[];
export declare function buildEcrpProjectState(input: RuntimeSessionEcrpProjectLoadInput): RuntimeSessionEcrpProjectState;
export declare function lifecycleStateFromEcrpProject(state: RuntimeSessionEcrpProjectState): RuntimeSessionLifecycleState;
export declare function buildEcrpRuntimeReadout(input: {
    readonly identity: RuntimeSessionIdentity;
    readonly projectState: RuntimeSessionEcrpProjectState | null;
    readonly lifecycleState: RuntimeSessionLifecycleState;
    readonly sequenceId: number;
    readonly tick: number;
    readonly sessionHash: string;
}): RuntimeSessionEcrpReadout;
//# sourceMappingURL=runtime-session-ecrp.d.ts.map