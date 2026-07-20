import type { FrameCursor } from '@asha/runtime-session';
import type { BridgeErrorFamily } from './generated/operations.js';

export type {
  BridgeVec3,
  ComposedGameplayReadout,
  ComposedRuntimeSessionReadout,
  EnemyDirectNavAuthoritySource,
  EnemyDirectNavAuthorityTransport,
  EnemyDirectNavMovementRequest,
  EnemyDirectNavMovementResult,
  EngineHandle,
  FpsEncounterDirectorSnapshot,
  FpsEncounterLastTransition,
  FpsEncounterLifecycleInput,
  FpsEncounterStateReadout,
  FpsEncounterStatus,
  FpsEncounterTransitionAction,
  FpsEncounterTransitionRequest,
  FpsEncounterTransitionResult,
  FpsEntityHealthReadout,
  FpsHealth,
  FpsLifecycleStatus,
  FpsPolicyBinding,
  FpsPolicyBindingReadout,
  FpsPrimaryFireRequest,
  FpsPrimaryFireResult,
  FpsReadSetEvidence,
  FpsReplayEvidence,
  FpsRuntimeAuthorityTransport,
  FpsRuntimeRole,
  FpsRuntimeSessionRestartRequest,
  FpsRuntimeSessionSnapshot,
  FrameCursor,
  GameExtensionWeaponEffectInvocationRequest,
  GameExtensionWeaponEffectInvocationResult,
  GameRuleCatalogValidationReceipt,
  GameRuleEffectIntentRequest,
  GameRuleRuntimeReadout,
  GameplayModuleViewRequest,
  GameplayModuleViewScope,
  GameplayModuleViewSnapshot,
  GameplayPrefabPartInteractionReceipt,
  GameplayPrefabPartInteractionRequest,
  StepResult,
  WorkspaceAuthoringCloseInput,
  WorkspaceAuthoringCloseReceipt,
  WorkspaceAuthoringProjectionRequest,
  WorkspaceAuthoringProjectionSummary,
  WorkspaceAuthoringStateSummary,
  WorkspaceAuthoringStoredConfirmationInput,
  WorkspaceAuthoringStoredConfirmationReceipt,
} from '@asha/runtime-session';

// ── Opaque handle types ───────────────────────────────────────────────────────
// Branded numbers so a buffer handle can't be passed where an engine handle is
// expected. They carry no transport detail and never expose a StateStore.

export type RuntimeBufferHandle = number & { readonly __brand: 'RuntimeBufferHandle' };
export type ReplaySessionHandle = number & { readonly __brand: 'ReplaySessionHandle' };

export const frameCursor = (frame: number): FrameCursor => frame as FrameCursor;

// ── Error taxonomy ────────────────────────────────────────────────────────────

export type RuntimeBridgeErrorKind = BridgeErrorFamily;

export interface RuntimeBridgeErrorContext {
  readonly operation?: string;
  readonly path?: string;
  readonly retryable?: boolean;
  readonly details?: readonly string[];
  readonly provenance?: 'native_rust' | 'runtime_facade' | 'transport_loader';
}

/** Typed, classified error for every facade operation. No JSON error blobs. */
export class RuntimeBridgeError extends Error {
  readonly operation: string | null;
  readonly path: string | null;
  readonly retryable: boolean;
  readonly details: readonly string[];
  readonly provenance: RuntimeBridgeErrorContext['provenance'];

  constructor(
    readonly kind: RuntimeBridgeErrorKind,
    message: string,
    context: RuntimeBridgeErrorContext = {},
  ) {
    super(`runtime bridge error [${kind}]: ${message}`);
    this.name = 'RuntimeBridgeError';
    this.operation = context.operation ?? null;
    this.path = context.path ?? null;
    this.retryable = context.retryable ?? false;
    this.details = context.details ?? [];
    this.provenance = context.provenance ?? 'runtime_facade';
  }
}
export function nonNegativeSafeInteger(value: number, field: string): number {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new RuntimeBridgeError('invalid_input', `${field} must be a non-negative safe integer`);
  }
  return value;
}

export function u32(value: number, field: string): number {
  nonNegativeSafeInteger(value, field);
  if (value > 0xffffffff) {
    throw new RuntimeBridgeError('invalid_input', `${field} must fit in u32`);
  }
  return value;
}

// ── Prototype operation payloads ──────────────────────────────────────────────
// PROTOTYPE: replaced by generated protocol_runtime / protocol_script contracts
// once the codegen emitter lands. The facade *shape* is the stable part.
//
// The simplified ProjectBundle DTOs below are deliberate subsets of the generated
// bundle/diagnostic contracts (@asha/contracts manifest / SaveSummary /
// DiagnosticReportSet). `world-dto-conformance.test.ts` is a compile-time guard
// that fails when a shared field's type drifts in the generated contract, keeping
// this prototype debt visible until the DTOs are replaced outright.

export interface EngineConfig {
  readonly seed: number;
}
export interface StepInputEnvelope {
  readonly tick: number;
}


// `CommandBatch` / `CommandResult` are NOT prototype DTOs: they are the generated
// voxel command border (imported from `@asha/contracts`). `submitCommands` carries
// the real `VoxelCommand` union — there is no `{ kind: 'smoke-edit' }` placeholder
// command tunnel; an ad-hoc command shape fails to type-check at the call site.
/** Borrowed, read-only view over bridge-owned bytes (large payloads, e.g. mesh). */
export interface RuntimeBufferView {
  readonly handle: RuntimeBufferHandle;
  readonly bytes: Uint8Array;
}
// Quarantined replay payloads.
export interface ReplayFixture {
  readonly name: string;
  readonly steps: number;
}
export interface ReplayStepReport {
  readonly step: number;
  readonly hash: string;
  readonly diverged: boolean;
}
/**
 * Native-buffer input for one staged ProjectBundle resource. This intentionally
 * differs from the generated Rust wire DTO: public hosts lend a Uint8Array to
 * the bridge and never expand binary content into a JSON-compatible number[].
 */
export interface ProjectResourceStageInput {
  readonly generation: number;
  readonly path: string;
  readonly bytes: Uint8Array;
}
// Compact voxel mesh/remesh evidence (#2646). Prototype DTOs until generated
// protocol_render contracts grow the same shapes.
export interface VoxelMeshEvidenceRequest {
  readonly grid: number;
  readonly chunks: readonly { readonly x: number; readonly y: number; readonly z: number }[];
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
  readonly coord: { readonly x: number; readonly y: number; readonly z: number };
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

export {
  RUNTIME_BRIDGE_PORT_CONTRACTS,
  runtimeBridgePorts,
} from './generated/surfaces.js';
export type {
  RuntimeBridge,
  RuntimeBridgePortContract,
  RuntimeBridgePortId,
  RuntimeBridgePorts,
  RuntimeProjectLifecyclePort,
  RuntimeCameraPort,
  RuntimeGameplayPort,
  RuntimeInputPort,
  RuntimeProjectionPort,
  RuntimeReplayEvidencePort,
  RuntimeSceneEntityPort,
  RuntimeTimeSimulationPort,
  RuntimeVoxelAssetBufferPort,
} from './generated/surfaces.js';
