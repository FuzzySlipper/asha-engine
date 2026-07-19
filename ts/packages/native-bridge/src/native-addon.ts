import type {
  CameraCollisionSnapshot,
  CameraCreateRequest,
  CameraSnapshot,
  CollisionConstrainedCameraInputEnvelope,
  CommandResult,
  DeveloperConsoleSnapshot,
  FirstPersonCameraInputEnvelope,
  GameplayCompositionDiagnostic,
  RuntimeProjectionFrame,
} from '@asha/contracts';
import {
  GENERATED_NATIVE_ADDON_EXPORTS,
  type GeneratedNativeAddonDeclaration,
} from './generated/addon-surface.js';

export interface NativeVec3 {
  readonly x: number;
  readonly y: number;
  readonly z: number;
}

type NativeFpsRole = 'player' | 'enemy' | 'neutral';

interface NativeEnemyDirectNavMovementResult {
  readonly entity: number;
  readonly authoritySource: string;
  readonly from: NativeVec3;
  readonly target: NativeVec3;
  readonly nextWaypoint: NativeVec3;
  readonly distanceUnits: number;
  readonly reached: boolean;
  readonly pathHash: string;
  readonly transformHash: string;
  readonly projectionChanged: boolean;
}

interface NativeFpsTransformCapability {
  readonly translation: NativeVec3;
  readonly rotation: readonly [number, number, number, number];
  readonly scale: NativeVec3;
}

interface NativeFpsBoundsCapability {
  readonly min: NativeVec3;
  readonly max: NativeVec3;
}

interface NativeFpsHealth {
  readonly current: number;
  readonly max: number;
}

interface NativeFpsWeaponMount {
  readonly weaponId: string;
  readonly damage: number;
  readonly rangeUnits: number;
  readonly ammo: number;
  readonly cooldownTicksAfterFire: number;
}

interface NativeFpsPolicyBinding {
  readonly bindingId: string;
  readonly policyId: string;
  readonly viewKind: string;
  readonly viewVersion: string;
  readonly allowedIntents: readonly string[];
  readonly runtimeMoment: string;
}

interface NativeFpsStoredEntityDefinition {
  readonly entity: number;
  readonly stableId: string;
  readonly displayName: string;
  readonly sourcePath: string;
  readonly tags: readonly string[];
  readonly role: string;
  readonly transform: NativeFpsTransformCapability | undefined;
  readonly bounds: NativeFpsBoundsCapability | undefined;
  readonly renderVisible: boolean | null;
  readonly staticCollider: boolean | null;
  readonly health: NativeFpsHealth | undefined;
  readonly weapon: NativeFpsWeaponMount | undefined;
  readonly policyBinding: NativeFpsPolicyBinding | undefined;
}

interface NativeFpsRuntimeSessionSnapshot {
  readonly backend: string;
  readonly authoritySurface: string;
  readonly projectBundle: string;
  readonly sessionEpoch: number;
  readonly lifecycleStatus: { readonly state: string; readonly entity?: number; readonly tick?: number };
  readonly playerEntity: number;
  readonly enemyEntity: number;
  readonly health: readonly { readonly entity: number; readonly current: number; readonly max: number }[];
  readonly policyBindings: readonly (NativeFpsPolicyBinding & { readonly entity: number })[];
  readonly replayRecords: readonly {
    readonly replayUnit: string;
    readonly entityHash: string;
    readonly healthHash: string;
    readonly recordHash: string;
  }[];
  readonly readSets: readonly { readonly viewKind: string; readonly owner: string; readonly readSet: readonly string[] }[];
  readonly entityHash: string;
  readonly healthHash: string;
  readonly replayHash: string;
}

export interface NativeGeneratedTunnelRuntimeApplyReceipt {
  readonly presetId: string;
  readonly seed: number;
  readonly grid: number;
  readonly configHash: string;
  readonly outputHash: string;
  readonly collisionSourceHash: string;
  readonly collisionProjectionHash: string;
  readonly runtimeFrame: {
    readonly worldOffset: readonly number[];
    readonly playableMin: readonly number[];
    readonly playableMax: readonly number[];
  };
}

interface NativeFpsPrimaryFireResult {
  readonly backend: string;
  readonly authoritySurface: string;
  readonly mutationOwner: string;
  readonly workspaceTrace: readonly string[];
  readonly shooter: number;
  readonly target?: number | null;
  readonly targetHealthBefore?: NativeFpsHealth | null;
  readonly targetHealthAfter?: NativeFpsHealth | null;
  readonly lifecycleStatus: { readonly state: string; readonly entity?: number; readonly tick?: number };
  readonly targetRenderVisible?: boolean | null;
  readonly entityHash: string;
  readonly healthHash: string;
  readonly replayHash: string;
}

interface NativeComposedRuntimeSessionReadout {
  readonly schemaVersion: number;
  readonly entityAuthorityHash: string;
  readonly gameplay: {
    readonly gameplayRegistryDigest: string;
    readonly semanticCompatibilityDigest: string;
    readonly artifactProvenanceDigest: string;
    readonly compositionLoadMode: 'compatible' | 'exact';
    readonly compatibilityDiagnostics: readonly GameplayCompositionDiagnostic[];
    readonly bindingRegistryHash: string;
    readonly activationHash: string;
    readonly moduleStateHash: string;
    readonly authorityStateHash: string;
    readonly triggerRevision: number;
    readonly triggerSnapshotHash: string;
    readonly activeOverlapCount: number;
    readonly reactionFrameCount: number;
    readonly lastReactionFrameHash: string | null;
    readonly decisionReceiptCount: number;
    readonly pendingDecisionCount: number;
    readonly lastDecisionReceiptHash: string | null;
    readonly schedulerStateHash: string;
    readonly schedulerPendingActionCount: number;
    readonly schedulerOutstandingDispatchCount: number;
    readonly schedulerOutstandingEventDeliveryCount: number;
    readonly schedulerFactCount: number;
    readonly schedulerTruncated: boolean;
    readonly runtimeHostHash: string;
  };
  readonly fpsSessionEpoch: number;
  readonly fpsReplayHash: string | null;
  readonly runtimeSessionHash: string;
}

interface NativeGameplayModuleViewSnapshot {
  readonly view: {
    readonly namespace: string;
    readonly name: string;
    readonly version: number;
    readonly schemaHash: string;
  };
  readonly providerId: string;
  readonly scopeKind: string;
  readonly scopeValue: number | null;
  readonly revision: number;
  readonly canonicalPayload: Uint8Array;
  readonly viewHash: string;
  readonly runtimeSessionHash: string;
}

interface NativeGameplayPrefabPartInteractionReceipt {
  readonly actor: number;
  readonly instance: number;
  readonly role: string;
  readonly target: number;
  readonly eventHash: string;
  readonly reactionFrameHash: string;
  readonly runtimeSessionHash: string;
}

interface NativeGameExtensionWeaponEffectInvocationResult {
  readonly hookReceiptJson: string;
  readonly replayEvidenceJson: string;
  readonly primaryFire: NativeFpsPrimaryFireResult | null;
}

interface NativeFpsEncounterLifecycleInput {
  readonly outcomeKind: 'in_progress' | 'won' | 'lost';
  readonly terminal: boolean;
  readonly enemyDead: boolean;
  readonly playerDead: boolean;
  readonly lifecycleHash: string;
}

interface NativeFpsEncounterTransitionRequest {
  readonly presetId: string;
  readonly action: 'activate' | 'sync_lifecycle' | 'reset';
  readonly lifecycle: NativeFpsEncounterLifecycleInput;
}

interface NativeFpsEncounterStateReadout {
  readonly presetId: string;
  readonly status: 'pending' | 'active' | 'cleared' | 'failed';
  readonly spawnedEnemyIds: readonly string[];
  readonly defeatedEnemyIds: readonly string[];
  readonly revision: number;
  readonly lastTransition: 'initialized' | 'activated' | 'cleared' | 'failed' | 'reset';
}

interface NativeFpsEncounterDirectorSnapshot {
  readonly backend: string;
  readonly authoritySurface: string;
  readonly mutationOwner: string;
  readonly workspaceTrace: readonly string[];
  readonly state: NativeFpsEncounterStateReadout;
  readonly lifecycle: NativeFpsEncounterLifecycleInput;
  readonly readSets: readonly { readonly viewKind: string; readonly owner: string; readonly readSet: readonly string[] }[];
  readonly encounterHash: string;
  readonly replayHash: string;
}

interface NativeFpsEncounterTransitionResult extends NativeFpsEncounterDirectorSnapshot {
  readonly accepted: boolean;
  readonly rejectionReason: 'encounter_not_pending' | 'invalid_encounter_transition' | 'unknown_encounter_preset' | null;
  readonly eventKind:
    | 'runtime_encounter.activated.v0'
    | 'runtime_encounter.lifecycle_synced.v0'
    | 'runtime_encounter.reset.v0'
    | null;
}

/**
 * The typed surface the compiled addon exports. Mirrors the `#[napi]` functions in
 * `native-bridge/src/lib.rs`. Kept in lockstep with the bridge manifest's stable
 * operations; the generated `#[napi]` wrappers (one-in/one-out) replace the
 * hand-written stubs once the codegen emitter lands.
 */
interface NativeAddonBindings {
  initializeEngine(seed: number): number;
  openWorkspaceAuthoring(existingHandle: number, requestJson: string): number;
  readWorkspaceAuthoringState(handle: number): string;
  readWorkspaceAuthoringProjection(handle: number, requestJson: string): string;
  confirmWorkspaceAuthoringStored(handle: number, requestJson: string): string;
  closeWorkspaceAuthoring(handle: number, requestJson: string): string;
  beginRuntimeProjectSourceResources(
    handle: number,
    requestJson: string,
  ): { readonly generation: number; readonly manifestHash: string };
  stageRuntimeProjectSourceResource(
    handle: number,
    generation: number,
    path: string,
    bytes: Uint8Array,
  ): { readonly handle: number; readonly generation: number; readonly version: number; readonly byteLen: number };
  admitRuntimeProjectSourceBatch(handle: number, requestJson: string): string;
  loadProjectBundle(
    handle: number,
    bundleSchemaVersion: number,
    protocolVersion: number,
    sceneId: number,
  ): {
    loadedProjectBundle: number | null;
    fatalCount: number;
    totalCount: number;
    blocksLoad: boolean;
  };
  submitCommands(handle: number, commandsJson: string): CommandResult;
  stepSimulation(handle: number, tick: number): { readonly tick: number; readonly diffCount: number };
  configureInputSession(handle: number, requestJson: string): string;
  applyInputContextCommand(handle: number, commandJson: string): string;
  submitRawInput(handle: number, sampleJson: string): string;
  replayResolvedInputAction(handle: number, recordJson: string): string;
  readInputContextState(handle: number): string;
  applyTimeControlCommand(handle: number, commandJson: string): string;
  readTimeControlState(handle: number): string;
  createCamera(handle: number, request: CameraCreateRequest): CameraSnapshot;
  applyCameraModeCommand(handle: number, commandJson: string): string;
  applyCameraNavigationInput(handle: number, envelopeJson: string): string;
  readCameraControllerState(handle: number, requestJson: string): string;
  applyCollisionConstrainedCameraInput(
    handle: number,
    envelope: CollisionConstrainedCameraInputEnvelope,
  ): CameraCollisionSnapshot;
  applyFirstPersonCameraInput(
    handle: number,
    envelope: FirstPersonCameraInputEnvelope,
  ): CameraSnapshot;
  readCameraProjection(handle: number, requestJson: string): string;
  pickVoxel(handle: number, requestJson: string): string;
  configureVoxelProjectionInstances(handle: number, requestJson: string): string;
  pickVoxelInstance(handle: number, requestJson: string): string;
  selectVoxel(handle: number, requestJson: string): string;
  readVoxelMeshEvidence(handle: number, requestJson: string): string;
  getBuffer(handle: number, bufferHandle: number): {
    readonly handle: number;
    readonly bytes: readonly number[];
  };
  releaseBuffer(handle: number, bufferHandle: number): void;
  unloadProjectBundle(handle: number): void;
  readModelMaterialPreview(handle: number, requestJson: string): string;
  decodeSceneDocument(handle: number, requestJson: string): string;
  encodeSceneDocument(handle: number, requestJson: string): string;
  applySceneDocumentAuthoring(handle: number, requestJson: string): string;
  decodeProjectContent(handle: number, requestJson: string): string;
  encodeProjectContent(handle: number, requestJson: string): string;
  applyProjectContentAuthoring(handle: number, requestJson: string): string;
  previewProceduralEnvironment(handle: number, requestJson: string): string;
  applyProceduralEnvironment(handle: number, requestJson: string): string;
  readSceneObjectSnapshot(handle: number): string;
  applySceneObjectCommand(handle: number, requestJson: string): string;
  applyGeneratedTunnelToRuntimeWorld(
    handle: number,
    presetId: string,
    seed: number,
  ): NativeGeneratedTunnelRuntimeApplyReceipt;
  applyEnemyDirectNavMovement(
    handle: number,
    entity: number,
    seedPosition: NativeVec3,
    target: NativeVec3,
    maxStepUnits: number,
  ): NativeEnemyDirectNavMovementResult;
  loadFpsRuntimeSession(
    handle: number,
    projectBundle: string,
    sceneDocumentJson: string,
    bootstrapResolutionRegistryJson: string,
    definitions: readonly NativeFpsStoredEntityDefinition[],
    gameRuleModulesJson: string,
  ): NativeFpsRuntimeSessionSnapshot;
  readFpsRuntimeSession(handle: number): NativeFpsRuntimeSessionSnapshot;
  applyFpsPrimaryFire(
    handle: number,
    tick: number,
    origin: NativeVec3,
    direction: NativeVec3,
    shooterRole?: NativeFpsRole,
    targetRole?: NativeFpsRole,
  ): NativeFpsPrimaryFireResult;
  readComposedRuntimeSession(handle: number): NativeComposedRuntimeSessionReadout;
  readGameplayModuleView(
    handle: number,
    namespace: string,
    name: string,
    version: number,
    schemaHash: string,
    scopeKind: string,
    scopeValue: number | undefined,
    expectedRuntimeSessionHash: string,
  ): NativeGameplayModuleViewSnapshot;
  applyGameplayPrefabPartInteraction(
    handle: number,
    actor: number,
    instance: number,
    role: string,
    expectedTarget: number,
    tick: number,
    expectedRuntimeSessionHash: string,
  ): NativeGameplayPrefabPartInteractionReceipt;
  invokeGameExtensionWeaponEffect(
    handle: number,
    hookJson: string,
    tick: number,
    origin: NativeVec3,
    direction: NativeVec3,
    shooterRole?: NativeFpsRole,
    targetRole?: NativeFpsRole,
  ): NativeGameExtensionWeaponEffectInvocationResult;
  validateGameRuleCatalog(handle: number, catalogJson: string): string;
  submitGameRuleEffectIntent(handle: number, catalogJson: string, requestJson: string): string;
  readGameRuleRuntimeReadout(handle: number): string;
  restartFpsRuntimeSession(handle: number, expectedEpoch: number): NativeFpsRuntimeSessionSnapshot;
  readFpsEncounterDirector(
    handle: number,
    lifecycle: NativeFpsEncounterLifecycleInput,
  ): NativeFpsEncounterDirectorSnapshot;
  applyFpsEncounterTransition(
    handle: number,
    request: NativeFpsEncounterTransitionRequest,
  ): NativeFpsEncounterTransitionResult;
  readRenderDiffs(handle: number, cursor: number): string;
  readProjectionFrame(handle: number, cursor: number): RuntimeProjectionFrame;
  readDeveloperConsole(handle: number): DeveloperConsoleSnapshot;
  saveProjectBundle(handle: number): {
    artifactsWritten: number;
    compactedEdits: number;
    retainedEdits: number;
  };
  getProjectBundleCompositionStatus(handle: number): {
    loadedProjectBundle: number | null;
    fatalCount: number;
    totalCount: number;
    blocksLoad: boolean;
  };
  planVoxelConversion(handle: number, requestJson: string): string;
  registerVoxelConversionSource(handle: number, requestJson: string): string;
  registerVoxelConversionMeshAsset(handle: number, requestJson: string): string;
  importVoxelConversionMeshSource(handle: number, requestJson: string): string;
  readVoxelConversionSourceMetadata(handle: number, requestJson: string): string;
  previewVoxelConversion(handle: number, requestJson: string): string;
  applyVoxelConversion(handle: number, requestJson: string): string;
  exportVoxelConversionEvidence(handle: number, evidenceJson: string): string;
  readVoxelModelInfo(handle: number, requestJson: string): string;
  readVoxelModelWindow(handle: number, requestJson: string): string;
  exportVoxelVolumeAsset(handle: number, requestJson: string): string;
  saveVoxelVolumeAsset(handle: number, requestJson: string): string;
  updateVoxelVolumeAssetPalette(handle: number, requestJson: string): string;
  initializeVoxelVolumeAuthoring(handle: number, requestJson: string): string;
  loadVoxelVolumeAsset(handle: number, requestJson: string): string;
  unloadVoxelVolumeAsset(handle: number, requestJson: string): string;
  validateVoxelAnnotationLayer(handle: number, requestJson: string): string;
  loadVoxelAnnotationLayer(handle: number, requestJson: string): string;
  readVoxelAnnotationQuery(handle: number, requestJson: string): string;
  applyVoxelAnnotationEdit(handle: number, requestJson: string): string;
  exportVoxelAnnotationLayer(handle: number, requestJson: string): string;
  readVoxelEditHistory(handle: number, requestJson: string): string;
  previewVoxelEditRevert(handle: number, requestJson: string): string;
  applyVoxelEditRevert(handle: number, requestJson: string): string;
  undoVoxelEdit(handle: number, requestJson: string): string;
  redoVoxelEdit(handle: number, requestJson: string): string;
}

export type NativeAddon = GeneratedNativeAddonDeclaration<NativeAddonBindings>;
export const REQUIRED_NATIVE_ADDON_EXPORTS = GENERATED_NATIVE_ADDON_EXPORTS;
