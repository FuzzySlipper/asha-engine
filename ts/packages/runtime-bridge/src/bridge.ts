import type {
  CameraCollisionSnapshot,
  CameraCreateRequest,
  CameraProjectionRequest,
  CameraProjectionSnapshot,
  CameraSnapshot,
  CollisionConstrainedCameraInputEnvelope,
  CommandBatch,
  CommandResult,
  FirstPersonCameraInputEnvelope,
  GameExtensionHookReceipt,
  GameExtensionReplayEvidence,
  GameRuleCatalog,
  GameRuleDiagnostic,
  GameRuleEvidenceRef,
  GameRuleModifierState,
  GameRuleResolutionReceipt,
  GameRuleResolutionRequest,
  GameRuleTraceEntry,
  GameRuleModuleManifest,
  ModelMaterialPreviewRequest,
  ModelMaterialPreviewSnapshot,
  PickRay,
  PickResult,
  RenderFrameDiff,
  SceneObjectCommandRequest,
  SceneObjectCommandResult,
  SceneObjectSnapshot,
  ScreenPointToPickRayRequest,
  VoxelSelectionSnapshot,
  VoxelConversionApplyRequest,
  VoxelConversionEvidenceRef,
  VoxelConversionPlan,
  VoxelConversionPlanRequest,
  VoxelConversionPreview,
  VoxelConversionPreviewRequest,
  VoxelConversionReceipt,
  VoxelConversionSourceRegistration,
  VoxelConversionSourceRegistrationRequest,
  VoxelModelInfoReadout,
  VoxelModelInfoRequest,
  VoxelVolumeAssetExportReceipt,
  VoxelVolumeAssetExportRequest,
  WeaponEffectHookRequest,
} from '@asha/contracts';

// ── Opaque handle types ───────────────────────────────────────────────────────
// Branded numbers so a buffer handle can't be passed where an engine handle is
// expected. They carry no transport detail and never expose a StateStore.

export type EngineHandle = number & { readonly __brand: 'EngineHandle' };
export type RuntimeBufferHandle = number & { readonly __brand: 'RuntimeBufferHandle' };
export type FrameCursor = number & { readonly __brand: 'FrameCursor' };
export type ReplaySessionHandle = number & { readonly __brand: 'ReplaySessionHandle' };

export const frameCursor = (frame: number): FrameCursor => frame as FrameCursor;

// ── Error taxonomy ────────────────────────────────────────────────────────────

export type RuntimeBridgeErrorKind =
  | 'not_initialized'
  | 'invalid_input'
  | 'unknown_handle'
  | 'buffer_expired'
  | 'native_unavailable'
  | 'voxel_conversion_unavailable'
  | 'unsupported_source_asset'
  | 'source_hash_mismatch'
  | 'invalid_material_map'
  | 'output_limit_exceeded'
  | 'stale_authority_snapshot'
  | 'conversion_replay_mismatch'
  // A stable operation exists on the facade but has no native implementation
  // wired yet. The native bridge throws this instead of silently falling back to
  // mock/reference behaviour — the seam is explicit and fail-closed.
  | 'operation_unimplemented'
  | 'internal';

/** Typed, classified error for every facade operation. No JSON error blobs. */
export class RuntimeBridgeError extends Error {
  constructor(readonly kind: RuntimeBridgeErrorKind, message: string) {
    super(`runtime bridge error [${kind}]: ${message}`);
    this.name = 'RuntimeBridgeError';
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
// The simplified world DTOs below are deliberate subsets of the generated
// protocol contracts (@asha/contracts: WorldBundleManifest / SaveSummary /
// DiagnosticReportSet). `world-dto-conformance.test.ts` is a compile-time guard
// that fails when a shared field's type drifts in the generated contract, keeping
// this prototype debt visible until the DTOs are replaced outright.

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
export type BridgeVec3 = readonly [number, number, number];
export type EnemyDirectNavAuthoritySource = 'seeded_from_request' | 'rust_entity_store';
export type EnemyDirectNavAuthorityTransport = 'native_rust' | 'reference_bridge';
export interface EnemyDirectNavMovementRequest {
  readonly entity: number;
  readonly seedPosition: BridgeVec3;
  readonly target: BridgeVec3;
  readonly maxStepUnits: number;
}
export interface EnemyDirectNavMovementResult {
  readonly entity: number;
  readonly authoritySource: EnemyDirectNavAuthoritySource;
  readonly authorityTransport: EnemyDirectNavAuthorityTransport;
  readonly from: BridgeVec3;
  readonly target: BridgeVec3;
  readonly nextWaypoint: BridgeVec3;
  readonly distanceUnits: number;
  readonly reached: boolean;
  readonly pathHash: string;
  readonly transformHash: string;
  readonly projectionChanged: boolean;
}
export type FpsRuntimeRole = 'player' | 'enemy' | 'neutral';
export type FpsRuntimeAuthorityTransport = 'native_rust' | 'reference_bridge';
export interface FpsTransformCapability {
  readonly translation: BridgeVec3;
  readonly rotation: readonly [number, number, number, number];
  readonly scale: BridgeVec3;
}
export interface FpsBoundsCapability {
  readonly min: BridgeVec3;
  readonly max: BridgeVec3;
}
export interface FpsHealth {
  readonly current: number;
  readonly max: number;
}
export interface FpsWeaponMount {
  readonly weaponId: string;
  readonly damage: number;
  readonly rangeUnits: number;
  readonly ammo: number;
  readonly cooldownTicksAfterFire: number;
}
export interface FpsPolicyBinding {
  readonly bindingId: string;
  readonly policyId: string;
  readonly viewKind: string;
  readonly viewVersion: string;
  readonly allowedIntents: readonly string[];
  readonly runtimeMoment: string;
}
export interface FpsStoredEntityDefinition {
  readonly entity: number;
  readonly stableId: string;
  readonly displayName: string;
  readonly sourcePath: string;
  readonly tags: readonly string[];
  readonly role: FpsRuntimeRole;
  readonly transform: FpsTransformCapability | null;
  readonly bounds: FpsBoundsCapability | null;
  readonly renderVisible: boolean | null;
  readonly staticCollider: boolean | null;
  readonly health: FpsHealth | null;
  readonly weapon: FpsWeaponMount | null;
  readonly policyBinding: FpsPolicyBinding | null;
}
export interface FpsRuntimeSessionLoadRequest {
  readonly projectBundle: string;
  readonly definitions: readonly FpsStoredEntityDefinition[];
  readonly gameRuleModules: readonly GameRuleModuleManifest[];
}
export interface FpsRuntimeSessionRestartRequest {
  readonly expectedEpoch: number;
}
export interface FpsPrimaryFireRequest {
  readonly tick: number;
  readonly origin: BridgeVec3;
  readonly direction: BridgeVec3;
}
export type FpsLifecycleStatus =
  | { readonly state: 'active' }
  | { readonly state: 'enemy_defeated'; readonly entity: number; readonly tick: number };
export interface FpsReadSetEvidence {
  readonly viewKind: string;
  readonly owner: string;
  readonly readSet: readonly string[];
}
export interface FpsReplayEvidence {
  readonly replayUnit: string;
  readonly entityHash: string;
  readonly healthHash: string;
  readonly recordHash: string;
}
export interface FpsEntityHealthReadout {
  readonly entity: number;
  readonly current: number;
  readonly max: number;
}
export interface FpsPolicyBindingReadout extends FpsPolicyBinding {
  readonly entity: number;
}
export interface FpsRuntimeSessionSnapshot {
  readonly backend: FpsRuntimeAuthorityTransport;
  readonly authoritySurface: string;
  readonly projectBundle: string;
  readonly sessionEpoch: number;
  readonly lifecycleStatus: FpsLifecycleStatus;
  readonly playerEntity: number;
  readonly enemyEntity: number;
  readonly health: readonly FpsEntityHealthReadout[];
  readonly policyBindings: readonly FpsPolicyBindingReadout[];
  readonly replayRecords: readonly FpsReplayEvidence[];
  readonly readSets: readonly FpsReadSetEvidence[];
  readonly entityHash: string;
  readonly healthHash: string;
  readonly replayHash: string;
}
export interface FpsPrimaryFireResult {
  readonly backend: FpsRuntimeAuthorityTransport;
  readonly authoritySurface: string;
  readonly mutationOwner: string;
  readonly workspaceTrace: readonly string[];
  readonly shooter: number;
  readonly target: number | null;
  readonly targetHealthBefore: FpsHealth | null;
  readonly targetHealthAfter: FpsHealth | null;
  readonly lifecycleStatus: FpsLifecycleStatus;
  readonly targetRenderVisible: boolean | null;
  readonly entityHash: string;
  readonly healthHash: string;
  readonly replayHash: string;
}
export interface GameExtensionWeaponEffectInvocationRequest {
  readonly hook: WeaponEffectHookRequest;
  readonly primaryFire: FpsPrimaryFireRequest;
}
export interface GameExtensionWeaponEffectInvocationResult {
  readonly hookReceipt: GameExtensionHookReceipt;
  readonly replayEvidence: GameExtensionReplayEvidence;
  readonly primaryFire: FpsPrimaryFireResult | null;
}
export interface GameRuleCatalogValidationReceipt {
  readonly accepted: boolean;
  readonly catalogHash: string;
  readonly diagnostics: readonly GameRuleDiagnostic[];
  readonly trace: readonly GameRuleTraceEntry[];
  readonly evidence: readonly GameRuleEvidenceRef[];
}
export interface GameRuleEffectIntentRequest {
  readonly catalog: GameRuleCatalog;
  readonly request: GameRuleResolutionRequest;
}
export interface GameRuleRuntimeReadout {
  readonly backend: FpsRuntimeAuthorityTransport;
  readonly authoritySurface: string;
  readonly activeModifiers: readonly GameRuleModifierState[];
  readonly recentTrace: readonly GameRuleTraceEntry[];
  readonly recentReplayHashes: readonly string[];
  readonly latestReplayHash: string | null;
}
export type FpsEncounterStatus = 'pending' | 'active' | 'cleared' | 'failed';
export type FpsEncounterLastTransition = 'initialized' | 'activated' | 'cleared' | 'failed' | 'reset';
export type FpsEncounterTransitionAction = 'activate' | 'sync_lifecycle' | 'reset';
export interface FpsEncounterLifecycleInput {
  readonly outcomeKind: 'in_progress' | 'won' | 'lost';
  readonly terminal: boolean;
  readonly enemyDead: boolean;
  readonly playerDead: boolean;
  readonly lifecycleHash: string;
}
export interface FpsEncounterTransitionRequest {
  readonly presetId: string;
  readonly action: FpsEncounterTransitionAction;
  readonly lifecycle: FpsEncounterLifecycleInput;
}
export interface FpsEncounterStateReadout {
  readonly presetId: string;
  readonly status: FpsEncounterStatus;
  readonly spawnedEnemyIds: readonly string[];
  readonly defeatedEnemyIds: readonly string[];
  readonly revision: number;
  readonly lastTransition: FpsEncounterLastTransition;
}
export interface FpsEncounterDirectorSnapshot {
  readonly backend: FpsRuntimeAuthorityTransport;
  readonly authoritySurface: string;
  readonly mutationOwner: string;
  readonly workspaceTrace: readonly string[];
  readonly state: FpsEncounterStateReadout;
  readonly lifecycle: FpsEncounterLifecycleInput;
  readonly readSets: readonly FpsReadSetEvidence[];
  readonly encounterHash: string;
  readonly replayHash: string;
}
export interface FpsEncounterTransitionResult {
  readonly backend: FpsRuntimeAuthorityTransport;
  readonly authoritySurface: string;
  readonly mutationOwner: string;
  readonly workspaceTrace: readonly string[];
  readonly accepted: boolean;
  readonly rejectionReason: 'encounter_not_pending' | 'invalid_encounter_transition' | 'unknown_encounter_preset' | null;
  readonly eventKind:
    | 'runtime_encounter.activated.v0'
    | 'runtime_encounter.lifecycle_synced.v0'
    | 'runtime_encounter.reset.v0'
    | null;
  readonly state: FpsEncounterStateReadout;
  readonly lifecycle: FpsEncounterLifecycleInput;
  readonly encounterHash: string;
  readonly replayHash: string;
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
// World load/save composition payloads (#2363). PROTOTYPE: replaced by generated
// protocol_world_bundle / protocol_diagnostics contracts once the emitter wires them.
export interface WorldLoadRequest {
  readonly bundleSchemaVersion: number;
  readonly protocolVersion: number;
  readonly sceneId: number;
}
export interface CompositionStatus {
  readonly loadedWorld: number | null;
  readonly fatalCount: number;
  readonly totalCount: number;
  readonly blocksLoad: boolean;
}
export interface WorldSaveSummary {
  readonly artifactsWritten: number;
  readonly compactedEdits: number;
  readonly retainedEdits: number;
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
  readonly worldHash: string;
  readonly meshingStrategy: string;
  readonly chunks: readonly VoxelMeshChunkEvidence[];
  readonly diagnostics: readonly string[];
}

// ── The facade surface ────────────────────────────────────────────────────────
// Bounded verbs only — mirrors bridge-manifest.toml. No generic call(method, json).

export interface RuntimeBridge {
  initializeEngine(config: EngineConfig): EngineHandle;
  stepSimulation(input: StepInputEnvelope): StepResult;
  submitCommands(batch: CommandBatch): CommandResult;
  pickVoxel(ray: PickRay): PickResult;
  applyCollisionConstrainedCameraInput(input: CollisionConstrainedCameraInputEnvelope): CameraCollisionSnapshot;
  selectVoxel(request: ScreenPointToPickRayRequest): VoxelSelectionSnapshot;
  readVoxelMeshEvidence(request: VoxelMeshEvidenceRequest): VoxelMeshEvidenceSnapshot;
  planVoxelConversion(request: VoxelConversionPlanRequest): VoxelConversionPlan;
  registerVoxelConversionSource(request: VoxelConversionSourceRegistrationRequest): VoxelConversionSourceRegistration;
  previewVoxelConversion(request: VoxelConversionPreviewRequest): VoxelConversionPreview;
  applyVoxelConversion(request: VoxelConversionApplyRequest): VoxelConversionReceipt;
  exportVoxelConversionEvidence(evidence: readonly VoxelConversionEvidenceRef[]): readonly VoxelConversionEvidenceRef[];
  readVoxelModelInfo(request: VoxelModelInfoRequest): VoxelModelInfoReadout;
  exportVoxelVolumeAsset(request: VoxelVolumeAssetExportRequest): VoxelVolumeAssetExportReceipt;
  loadFpsRuntimeSession(request: FpsRuntimeSessionLoadRequest): FpsRuntimeSessionSnapshot;
  readFpsRuntimeSession(): FpsRuntimeSessionSnapshot;
  applyFpsPrimaryFire(request: FpsPrimaryFireRequest): FpsPrimaryFireResult;
  invokeGameExtensionWeaponEffect(
    request: GameExtensionWeaponEffectInvocationRequest,
  ): GameExtensionWeaponEffectInvocationResult;
  validateGameRuleCatalog(catalog: GameRuleCatalog): GameRuleCatalogValidationReceipt;
  submitGameRuleEffectIntent(input: GameRuleEffectIntentRequest): GameRuleResolutionReceipt;
  readGameRuleRuntimeReadout(): GameRuleRuntimeReadout;
  restartFpsRuntimeSession(request: FpsRuntimeSessionRestartRequest): FpsRuntimeSessionSnapshot;
  readFpsEncounterDirector(lifecycle: FpsEncounterLifecycleInput): FpsEncounterDirectorSnapshot;
  applyFpsEncounterTransition(request: FpsEncounterTransitionRequest): FpsEncounterTransitionResult;
  readModelMaterialPreview(request: ModelMaterialPreviewRequest): ModelMaterialPreviewSnapshot;
  readSceneObjectSnapshot(): SceneObjectSnapshot;
  applySceneObjectCommand(request: SceneObjectCommandRequest): SceneObjectCommandResult;
  readRenderDiffs(cursor: FrameCursor): RenderFrameDiff;
  createCamera(request: CameraCreateRequest): CameraSnapshot;
  applyFirstPersonCameraInput(input: FirstPersonCameraInputEnvelope): CameraSnapshot;
  applyEnemyDirectNavMovement(request: EnemyDirectNavMovementRequest): EnemyDirectNavMovementResult;
  readCameraProjection(request: CameraProjectionRequest): CameraProjectionSnapshot;
  getBuffer(handle: RuntimeBufferHandle): RuntimeBufferView;
  releaseBuffer(handle: RuntimeBufferHandle): void;
  // World load/save composition (operational; not a replay-verification replacement).
  loadWorldBundle(request: WorldLoadRequest): CompositionStatus;
  saveCurrentWorld(): WorldSaveSummary;
  getCompositionStatus(): CompositionStatus;
  unloadWorld(): void;
  // Quarantined: replay/golden harness, not the production renderer path.
  loadReplayFixture(fixture: ReplayFixture): ReplaySessionHandle;
  runReplayStep(session: ReplaySessionHandle): ReplayStepReport;
}
