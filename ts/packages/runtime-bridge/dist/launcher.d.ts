import type { CommandBatch, CommandResult, RenderFrameDiff } from '@asha/contracts';
import { type CompositionStatus, type FrameCursor, type RuntimeBridge, type WorldLoadRequest } from './bridge.js';
export type GameRuntimeMode = 'reference' | 'native' | 'wasm' | 'degraded';
export type GameRuntimeBackendMode = 'reference' | 'native' | 'wasm';
export type GameRuntimeBackendTransport = 'reference_mock' | 'napi_native' | 'wasm_module';
export type GameRuntimeNonClaim = 'not_native_runtime' | 'not_hardware_gpu' | 'not_performance_evidence' | 'not_publish_artifact' | 'not_product_authority' | 'not_wasm_authority';
export type GameRuntimeDiagnosticCode = 'missing_compatibility' | 'missing_world_bundle' | 'unsupported_runtime_entry' | 'unsupported_backend_mode' | 'missing_backend_evidence' | 'private_transport_hint' | 'backend_claim_mismatch' | 'runtime_unavailable' | 'operation_unimplemented' | 'command_rejected' | 'stale_sequence' | 'stale_readback' | 'internal';
export interface GameRuntimeDiagnostic {
    readonly code: GameRuntimeDiagnosticCode;
    readonly severity: 'info' | 'warning' | 'error';
    readonly message: string;
}
export interface GameRuntimeCompatibility {
    readonly contractsPackageVersion: string;
    readonly runtimeBridgePackageVersion: string;
    readonly devtoolsProtocolVersion?: string;
    readonly publishArtifactVersion?: string;
}
export interface GameRuntimeProfile {
    readonly profileId: string;
    readonly runtimeMode: GameRuntimeMode;
    readonly launcherName: string;
    readonly bridgeCompatibility: GameRuntimeCompatibility;
    readonly nonClaims: readonly GameRuntimeNonClaim[];
}
export interface GameRuntimeBackendProfile {
    readonly profileId: string;
    readonly mode: GameRuntimeBackendMode;
    readonly transport: GameRuntimeBackendTransport;
    readonly launcherName: string;
    readonly bridgeCompatibility: GameRuntimeCompatibility;
    readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
    readonly nonClaims: readonly GameRuntimeNonClaim[];
}
export type GameRuntimeBackendProfileValidation = {
    readonly ok: true;
    readonly profile: GameRuntimeBackendProfile;
    readonly diagnostics: readonly [];
} | {
    readonly ok: false;
    readonly diagnostics: readonly GameRuntimeDiagnostic[];
};
export interface GameRuntimeResourceProfile {
    readonly profileId: string;
    readonly runtimeEntry: string;
    readonly worldBundleId: string;
    readonly resourceManifestHash?: string;
    readonly estimatedBytes?: number;
}
export interface GameRuntimeEvidenceRef {
    readonly kind: 'projection' | 'render_diff' | 'replay' | 'evidence_export' | 'telemetry' | 'diagnostic';
    readonly id: string;
    readonly path?: string;
    readonly sha256?: string;
    readonly sequenceId?: number;
}
export interface GameRuntimeConfig {
    readonly gameId: string;
    readonly workspaceId: string;
    readonly runtimeEntry: string;
    readonly compatibility: GameRuntimeCompatibility;
    readonly resourceProfile: GameRuntimeResourceProfile;
    readonly world: WorldLoadRequest;
    readonly startedAtIso?: string;
}
export interface GameRuntimeIdentity {
    readonly gameId: string;
    readonly workspaceId: string;
    readonly runtimeMode: GameRuntimeMode;
    readonly runtimeEntry: string;
    readonly startedAtIso: string;
    readonly compatibility: GameRuntimeCompatibility;
    readonly nonClaims: readonly GameRuntimeNonClaim[];
}
export interface GameRuntimeProjectionSummary {
    readonly sequenceId: number;
    readonly worldHash: string;
    readonly authorityHash: string;
    readonly loadedWorld: number | null;
    readonly fatalCount: number;
    readonly totalDiagnosticCount: number;
    readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}
export interface GameRuntimeLaunchResult {
    readonly status: 'launched' | 'degraded' | 'failed';
    readonly identity: GameRuntimeIdentity;
    readonly runtimeProfile: GameRuntimeProfile;
    readonly resourceProfile: GameRuntimeResourceProfile;
    readonly projection: GameRuntimeProjectionSummary;
    readonly diagnostics: readonly GameRuntimeDiagnostic[];
    readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}
export interface GameRuntimeCommandProposalResult {
    readonly sequenceId: number;
    readonly status: 'accepted' | 'rejected' | 'failed';
    readonly batch: CommandBatch;
    readonly result: CommandResult | null;
    readonly authorityHashBefore: string;
    readonly authorityHashAfter: string;
    readonly diagnostics: readonly GameRuntimeDiagnostic[];
    readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}
export interface GameRuntimeRenderDiffSnapshot {
    readonly sequenceId: number;
    readonly cursor: FrameCursor;
    readonly frame: RenderFrameDiff;
    readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}
export interface GameRuntimeTelemetrySnapshot {
    readonly sequenceId: number;
    readonly runtimeMode: GameRuntimeMode;
    readonly acceptedCommandCount: number;
    readonly rejectedCommandCount: number;
    readonly diagnostics: readonly GameRuntimeDiagnostic[];
    readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}
export interface GameRuntimeReplayExportRequest {
    readonly replayId: string;
}
export interface GameRuntimeReplayExport {
    readonly replayId: string;
    readonly sequenceId: number;
    readonly authorityHash: string;
    readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}
export interface GameRuntimeEvidenceExportRequest {
    readonly evidenceId: string;
}
export interface GameRuntimeEvidenceExport {
    readonly evidenceId: string;
    readonly sequenceId: number;
    readonly projection: GameRuntimeProjectionSummary;
    readonly nonClaims: readonly GameRuntimeNonClaim[];
    readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}
export interface GameRuntimeSession {
    readonly launch: GameRuntimeLaunchResult;
    readonly identity: GameRuntimeIdentity;
    pullProjection(): Promise<GameRuntimeProjectionSummary>;
    pullRenderDiff(cursor?: FrameCursor): Promise<GameRuntimeRenderDiffSnapshot>;
    pullTelemetry(): Promise<GameRuntimeTelemetrySnapshot>;
    proposeCommands(batch: CommandBatch): Promise<GameRuntimeCommandProposalResult>;
    exportReplay(request: GameRuntimeReplayExportRequest): Promise<GameRuntimeReplayExport>;
    exportEvidence(request: GameRuntimeEvidenceExportRequest): Promise<GameRuntimeEvidenceExport>;
    shutdown(): Promise<void>;
}
export interface GameRuntimeLauncher {
    readonly mode: GameRuntimeMode;
    launch(config: GameRuntimeConfig): Promise<GameRuntimeSession>;
}
export declare function validateGameRuntimeBackendProfile(input: object): GameRuntimeBackendProfileValidation;
export declare function nativeBackendProfile(config: GameRuntimeConfig): GameRuntimeBackendProfile;
export declare class BridgeGameRuntimeSession implements GameRuntimeSession {
    #private;
    private readonly bridge;
    private readonly config;
    readonly identity: GameRuntimeIdentity;
    readonly launch: GameRuntimeLaunchResult;
    constructor(bridge: RuntimeBridge, config: GameRuntimeConfig, runtimeProfile: GameRuntimeProfile, initialStatus: CompositionStatus);
    pullProjection(): Promise<GameRuntimeProjectionSummary>;
    pullRenderDiff(cursor?: FrameCursor): Promise<GameRuntimeRenderDiffSnapshot>;
    pullTelemetry(): Promise<GameRuntimeTelemetrySnapshot>;
    proposeCommands(batch: CommandBatch): Promise<GameRuntimeCommandProposalResult>;
    exportReplay(request: GameRuntimeReplayExportRequest): Promise<GameRuntimeReplayExport>;
    exportEvidence(request: GameRuntimeEvidenceExportRequest): Promise<GameRuntimeEvidenceExport>;
    shutdown(): Promise<void>;
}
export interface SelectedBackendLauncherOptions {
    readonly profile?: GameRuntimeBackendProfile;
    readonly nativeModulePath?: string;
    readonly bridgeFactory?: () => RuntimeBridge;
}
export declare class SelectedBackendGameRuntimeLauncher implements GameRuntimeLauncher {
    private readonly options;
    readonly mode: GameRuntimeMode;
    constructor(options?: SelectedBackendLauncherOptions);
    launch(config: GameRuntimeConfig): Promise<GameRuntimeSession>;
}
export declare function createSelectedBackendGameRuntimeLauncher(options?: SelectedBackendLauncherOptions): GameRuntimeLauncher;
export declare function createNativeGameRuntimeLauncher(options?: SelectedBackendLauncherOptions): GameRuntimeLauncher;
//# sourceMappingURL=launcher.d.ts.map