export type { GeneratedTunnelRuntimeApplyReceipt, GeneratedTunnelRuntimeApplyRequest, } from '@asha/contracts';
import type { FrameCursor } from '@asha/runtime-session';
import type { BridgeErrorFamily } from './generated/operations.js';
export type { BridgeVec3, CompositionStatus, EnemyDirectNavAuthoritySource, EnemyDirectNavAuthorityTransport, EnemyDirectNavMovementRequest, EnemyDirectNavMovementResult, EngineHandle, FpsBoundsCapability, FpsEncounterDirectorSnapshot, FpsEncounterLastTransition, FpsEncounterLifecycleInput, FpsEncounterStateReadout, FpsEncounterStatus, FpsEncounterTransitionAction, FpsEncounterTransitionRequest, FpsEncounterTransitionResult, FpsEntityHealthReadout, FpsHealth, FpsLifecycleStatus, FpsPolicyBinding, FpsPolicyBindingReadout, FpsPrimaryFireRequest, FpsPrimaryFireResult, FpsReadSetEvidence, FpsReplayEvidence, FpsRuntimeAuthorityTransport, FpsRuntimeRole, FpsRuntimeSessionLoadRequest, FpsRuntimeSessionRestartRequest, FpsRuntimeSessionSnapshot, FpsStoredEntityDefinition, FpsTransformCapability, FpsWeaponMount, FrameCursor, GameExtensionWeaponEffectInvocationRequest, GameExtensionWeaponEffectInvocationResult, GameRuleCatalogValidationReceipt, GameRuleEffectIntentRequest, GameRuleRuntimeReadout, ProjectBundleLoadRequest, StepResult, } from '@asha/runtime-session';
export type RuntimeBufferHandle = number & {
    readonly __brand: 'RuntimeBufferHandle';
};
export type ReplaySessionHandle = number & {
    readonly __brand: 'ReplaySessionHandle';
};
export declare const frameCursor: (frame: number) => FrameCursor;
export type RuntimeBridgeErrorKind = BridgeErrorFamily;
export interface RuntimeBridgeErrorContext {
    readonly operation?: string;
    readonly path?: string;
    readonly retryable?: boolean;
    readonly details?: readonly string[];
    readonly provenance?: 'native_rust' | 'runtime_facade' | 'transport_loader';
}
/** Typed, classified error for every facade operation. No JSON error blobs. */
export declare class RuntimeBridgeError extends Error {
    readonly kind: RuntimeBridgeErrorKind;
    readonly operation: string | null;
    readonly path: string | null;
    readonly retryable: boolean;
    readonly details: readonly string[];
    readonly provenance: RuntimeBridgeErrorContext['provenance'];
    constructor(kind: RuntimeBridgeErrorKind, message: string, context?: RuntimeBridgeErrorContext);
}
export declare function nonNegativeSafeInteger(value: number, field: string): number;
export declare function u32(value: number, field: string): number;
export interface EngineConfig {
    readonly seed: number;
}
export interface StepInputEnvelope {
    readonly tick: number;
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
export interface ProjectBundleSaveSummary {
    readonly artifactsWritten: number;
    readonly compactedEdits: number;
    readonly retainedEdits: number;
}
export interface VoxelMeshEvidenceRequest {
    readonly grid: number;
    readonly chunks: readonly {
        readonly x: number;
        readonly y: number;
        readonly z: number;
    }[];
}
export interface VoxelMeshStatsEvidence {
    readonly vertices: number;
    readonly indices: number;
    readonly quads: number;
    readonly facesEmitted: number;
    readonly facesCulled: number;
}
export interface VoxelMeshBoundsEvidence {
    readonly min: readonly [number, number, number];
    readonly max: readonly [number, number, number];
}
export interface VoxelMeshChunkEvidence {
    readonly coord: {
        readonly x: number;
        readonly y: number;
        readonly z: number;
    };
    readonly resident: boolean;
    readonly visible: boolean;
    readonly contentHash: string | null;
    readonly meshHash: string | null;
    readonly stats: VoxelMeshStatsEvidence | null;
    readonly bounds: VoxelMeshBoundsEvidence | null;
    readonly materialSlots: readonly number[];
}
export interface VoxelMeshEvidenceSnapshot {
    readonly grid: number;
    readonly fixtureId: string;
    readonly voxelStateHash: string;
    readonly meshingStrategy: string;
    readonly chunks: readonly VoxelMeshChunkEvidence[];
    readonly diagnostics: readonly string[];
}
export { RUNTIME_BRIDGE_PORT_CONTRACTS, runtimeBridgePorts, } from './generated/surfaces.js';
export type { RuntimeBridge, RuntimeBridgePortContract, RuntimeBridgePortId, RuntimeBridgePorts, RuntimeBundleLifecyclePort, RuntimeCameraPort, RuntimeGameplayPort, RuntimeInputPort, RuntimeProjectionPort, RuntimeReplayEvidencePort, RuntimeSceneEntityPort, RuntimeTimeSimulationPort, RuntimeVoxelAssetBufferPort, } from './generated/surfaces.js';
//# sourceMappingURL=bridge.d.ts.map