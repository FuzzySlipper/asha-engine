export interface RuntimeSessionProgressSnapshot {
    readonly sequenceId: number;
    readonly sessionTick: number;
    readonly latestProjectionTick: number;
    readonly acceptedCommandCount: number;
    readonly rejectedCommandCount: number;
    readonly restartCount: number;
}
/**
 * Owns facade-local progress bookkeeping. Session time and projection time are
 * deliberately separate: authority can advance without publishing a G1 frame.
 */
export declare class RuntimeSessionProgress {
    #private;
    initialize(): void;
    get sequenceId(): number;
    get sessionTick(): number;
    get latestProjectionTick(): number;
    get acceptedCommandCount(): number;
    get rejectedCommandCount(): number;
    get restartCount(): number;
    snapshot(): RuntimeSessionProgressSnapshot;
    nextSimulationTick(requested?: number): number;
    advanceSequence(): number;
    recordCommandBatch(accepted: number, rejected: number): void;
    recordSimulationTick(tick: number): void;
    observeAuthorityTick(tick: number): void;
    recordProjectionTick(tick: number): void;
    recordProjectedAuthorityTick(tick: number): void;
    restart(): void;
}
//# sourceMappingURL=runtime-session-rust-progress.d.ts.map