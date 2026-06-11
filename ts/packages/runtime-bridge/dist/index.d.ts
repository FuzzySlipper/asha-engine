import type { RenderFrameDiff } from '@asha/contracts';
export { MANIFEST_OPERATIONS } from './generated/operations.js';
export type { BridgeOperation, BridgeSurface } from './generated/operations.js';
export { decodeRenderDiff, decodeRenderFrameDiff, RenderDecodeError, RenderDiffStream, FrameMemory, } from './render-decode.js';
export type EngineHandle = number & {
    readonly __brand: 'EngineHandle';
};
export type RuntimeBufferHandle = number & {
    readonly __brand: 'RuntimeBufferHandle';
};
export type FrameCursor = number & {
    readonly __brand: 'FrameCursor';
};
export type ReplaySessionHandle = number & {
    readonly __brand: 'ReplaySessionHandle';
};
export declare const frameCursor: (frame: number) => FrameCursor;
export type RuntimeBridgeErrorKind = 'not_initialized' | 'invalid_input' | 'unknown_handle' | 'buffer_expired' | 'native_unavailable' | 'internal';
/** Typed, classified error for every facade operation. No JSON error blobs. */
export declare class RuntimeBridgeError extends Error {
    readonly kind: RuntimeBridgeErrorKind;
    constructor(kind: RuntimeBridgeErrorKind, message: string);
}
export interface EngineConfig {
    readonly seed: number;
}
export interface StepInputEnvelope {
    readonly tick: number;
}
export interface StepResult {
    readonly tick: number;
    readonly diffCount: number;
}
export interface ProposedCommand {
    readonly kind: string;
}
export interface CommandBatch {
    readonly commands: readonly ProposedCommand[];
}
export interface CommandResult {
    readonly accepted: number;
    readonly rejected: number;
}
/** Borrowed, read-only view over bridge-owned bytes (large payloads, e.g. mesh). */
export interface RuntimeBufferView {
    readonly handle: RuntimeBufferHandle;
    readonly bytes: Uint8Array;
}
export interface ReplayFixture {
    readonly name: string;
    readonly steps: number;
}
export interface ReplayStepReport {
    readonly step: number;
    readonly hash: string;
    readonly diverged: boolean;
}
export interface RuntimeBridge {
    initializeEngine(config: EngineConfig): EngineHandle;
    stepSimulation(input: StepInputEnvelope): StepResult;
    submitCommands(batch: CommandBatch): CommandResult;
    readRenderDiffs(cursor: FrameCursor): RenderFrameDiff;
    getBuffer(handle: RuntimeBufferHandle): RuntimeBufferView;
    releaseBuffer(handle: RuntimeBufferHandle): void;
    loadReplayFixture(fixture: ReplayFixture): ReplaySessionHandle;
    runReplayStep(session: ReplaySessionHandle): ReplayStepReport;
}
export declare class MockRuntimeBridge implements RuntimeBridge {
    #private;
    initializeEngine(config: EngineConfig): EngineHandle;
    stepSimulation(input: StepInputEnvelope): StepResult;
    submitCommands(batch: CommandBatch): CommandResult;
    readRenderDiffs(cursor: FrameCursor): RenderFrameDiff;
    getBuffer(handle: RuntimeBufferHandle): RuntimeBufferView;
    releaseBuffer(handle: RuntimeBufferHandle): void;
    loadReplayFixture(fixture: ReplayFixture): ReplaySessionHandle;
    runReplayStep(session: ReplaySessionHandle): ReplayStepReport;
}
/** Construct the default mock bridge. */
export declare function createMockRuntimeBridge(): RuntimeBridge;
/**
 * Construct the native (napi-rs) bridge. Throws a classified
 * {@link RuntimeBridgeError} of kind `native_unavailable` if the addon is not built
 * — callers can fall back to the mock for tests/dev.
 */
export declare function createNativeRuntimeBridge(modulePath?: string): RuntimeBridge;
/** Operation count for quick sanity in consumers/tests. */
export declare const STABLE_OPERATION_COUNT: number;
//# sourceMappingURL=index.d.ts.map