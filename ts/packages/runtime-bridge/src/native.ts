import type {
  CameraCollisionSnapshot,
  CameraControllerReadRequest,
  CameraControllerState,
  CameraCreateRequest,
  CameraModeChangeReceipt,
  CameraModeCommand,
  CameraNavigationInputEnvelope,
  CameraNavigationReceipt,
  CameraProjectionRequest,
  CameraProjectionSnapshot,
  CameraSnapshot,
  CommandBatch,
  CommandResult,
  DeveloperConsoleSnapshot,
  CollisionConstrainedCameraInputEnvelope,
  FirstPersonCameraInputEnvelope,
  ModelMaterialPreviewRequest,
  ModelMaterialPreviewSnapshot,
  PickResult,
  PickRay,
  VoxelInstancePickRequest,
  VoxelInstancePickResult,
  VoxelProjectionBindingRequest,
  VoxelProjectionBindingReceipt,
  RenderFrameDiff,
  RuntimeProjectionFrame,
  TimeControlCommand,
  TimeControlReceipt,
  TimeControlState,
  SceneObjectCommandResult,
  SceneObjectCommandRequest,
  SceneObjectSnapshot,
  SceneDocumentCodecResult,
  SceneDocumentAuthoringRequest,
  SceneDocumentAuthoringResult,
  SceneDocumentDecodeRequest,
  SceneDocumentEncodeRequest,
  ProjectContentAuthoringRequest,
  ProjectContentAuthoringResult,
  ProjectContentCodecResult,
  ProjectContentDecodeRequest,
  ProjectContentEncodeRequest,
  ProjectWriteConfirmRequest,
  ProjectWriteConfirmReceipt,
  ProjectWritePrepareRequest,
  ProjectWritePrepareReceipt,
  ProjectResourceBeginRequest,
  ProjectResourceTransactionReceipt,
  ProjectSourceBatchValidationReceipt,
  RuntimeProjectCloseReceipt,
  RuntimeProjectCloseRequest,
  RuntimeProjectLoadReceipt,
  RuntimeProjectLoadRequest,
  RuntimeProjectSourceBatch,
  StagedProjectResourceRef,
  ProceduralEnvironmentApplyRequest,
  ProceduralEnvironmentApplyResult,
  ProceduralEnvironmentPreviewRequest,
  ProceduralEnvironmentPreviewResult,
  VoxelConversionApplyRequest,
  VoxelConversionEvidenceRef,
  VoxelConversionMeshAssetRegistrationRequest,
  VoxelConversionMeshSourceImportReceipt,
  VoxelConversionMeshSourceImportRequest,
  VoxelConversionPlan,
  VoxelConversionPlanRequest,
  VoxelConversionPreview,
  VoxelConversionPreviewRequest,
  VoxelConversionReceipt,
  VoxelConversionSourceMetadataReadout,
  VoxelConversionSourceMetadataRequest,
  VoxelConversionSourceRegistration,
  VoxelConversionSourceRegistrationRequest,
  VoxelSelectionSnapshot,
  VoxelModelInfoReadout,
  VoxelModelInfoRequest,
  VoxelModelWindowReadout,
  VoxelModelWindowRequest,
  VoxelAnnotationEditReceipt,
  VoxelAnnotationEditRequest,
  VoxelAnnotationLayerExportReceipt,
  VoxelAnnotationLayerExportRequest,
  VoxelAnnotationLayerLoadReceipt,
  VoxelAnnotationLayerLoadRequest,
  VoxelAnnotationLayerValidationReport,
  VoxelAnnotationLayerValidationRequest,
  VoxelAnnotationQueryReadout,
  VoxelAnnotationQueryRequest,
  VoxelEditHistoryReadRequest,
  VoxelEditHistoryRedoReceipt,
  VoxelEditHistoryRedoRequest,
  VoxelEditHistoryRevertReceipt,
  VoxelEditHistoryRevertRequest,
  VoxelEditHistorySummary,
  VoxelEditHistoryUndoReceipt,
  VoxelEditHistoryUndoRequest,
  VoxelVolumeAssetExportReceipt,
  VoxelVolumeAssetExportRequest,
  VoxelVolumeAssetLoadReceipt,
  VoxelVolumeAssetLoadRequest,
  VoxelVolumeAssetUnloadReceipt,
  VoxelVolumeAssetUnloadRequest,
  VoxelVolumeAssetPaletteUpdateReceipt,
  VoxelVolumeAssetPaletteUpdateRequest,
  VoxelVolumeAssetSaveReceipt,
  VoxelVolumeAssetSaveRequest,
  VoxelVolumeAuthoringInitializeReceipt,
  VoxelVolumeAuthoringInitializeRequest,
  GameExtensionHookReceipt,
  GameExtensionReplayEvidence,
  GameRuleCatalog,
  GameRuleResolutionReceipt,
  InputActionReplayReceipt,
  InputContextChangeReceipt,
  InputContextCommand,
  InputContextStackState,
  InputResolutionReceipt,
  InputSessionConfigureRequest,
  InputSessionSnapshot,
  RawInputSample,
  RecordedInputAction,
  ScreenPointToPickRayRequest,
} from '@asha/contracts';
import { loadNativeAddon, NativeAddonUnavailable, type NativeAddon } from '@asha/native-bridge';
import { MANIFEST_OPERATIONS } from './generated/operations.js';
import {
  RuntimeBridgeError,
  frameCursor,
  nonNegativeSafeInteger,
  u32,
  type CompositionStatus,
  type ComposedRuntimeSessionReadout,
  type EnemyDirectNavMovementRequest,
  type EnemyDirectNavMovementResult,
  type EngineConfig,
  type EngineHandle,
  type FrameCursor,
  type FpsEncounterDirectorSnapshot,
  type FpsEncounterLifecycleInput,
  type FpsEncounterTransitionRequest,
  type FpsEncounterTransitionResult,
  type GameExtensionWeaponEffectInvocationRequest,
  type GameExtensionWeaponEffectInvocationResult,
  type GameRuleCatalogValidationReceipt,
  type GameRuleEffectIntentRequest,
  type GameRuleRuntimeReadout,
  type GameplayModuleViewRequest,
  type GameplayModuleViewScope,
  type GameplayModuleViewSnapshot,
  type GameplayPrefabPartInteractionReceipt,
  type GameplayPrefabPartInteractionRequest,
  type GeneratedTunnelRuntimeApplyReceipt,
  type GeneratedTunnelRuntimeApplyRequest,
  type FpsLifecycleStatus,
  type FpsPrimaryFireRequest,
  type FpsPrimaryFireResult,
  type FpsRuntimeAuthorityTransport,
  type FpsRuntimeRole,
  type FpsRuntimeSessionLoadRequest,
  type FpsRuntimeSessionRestartRequest,
  type FpsRuntimeSessionSnapshot,
  type ReplaySessionHandle,
  type ReplayStepReport,
  type RuntimeBridge,
  type RuntimeBufferView,
  type RuntimeBufferHandle,
  type StepInputEnvelope,
  type StepResult,
  type VoxelMeshEvidenceSnapshot,
  type VoxelMeshEvidenceRequest,
  type ProjectBundleLoadRequest,
  type ProjectResourceStageInput,
  type ProjectBundleSaveSummary,
  type WorkspaceAuthoringCloseInput,
  type WorkspaceAuthoringCloseReceipt,
  type WorkspaceAuthoringOpenInput,
  type WorkspaceAuthoringProjectionRequest,
  type WorkspaceAuthoringProjectionSummary,
  type WorkspaceAuthoringStateSummary,
  type WorkspaceAuthoringStoredConfirmationInput,
  type WorkspaceAuthoringStoredConfirmationReceipt,
} from './bridge.js';
import {
  callNative,
  nativeUnimplemented,
  parseNativeJson,
  runNativeOperation,
  type NativeFacadeValue,
} from './native-operation-boundary.js';
import { decodeRenderFrameDiff } from './render-decode.js';
import {
  projectionFrameFromNative,
  type NativeRuntimeProjectionFrameDto,
} from './native-presentation-normalization.js';
import { parseGeneratedOperationOutput } from './wire-validation.js';

export { classifyNativeAddonError } from './native-operation-boundary.js';

// ── Native implementation factory ─────────────────────────────────────────────
// The ONLY place that touches `@asha/native-bridge`. Wraps the addon's wired
// exports and re-classifies load failures into the bridge error taxonomy.
//
// Fail-closed by construction: `NativeRuntimeBridge` implements `RuntimeBridge`
// directly — it does NOT extend `MockRuntimeBridge`, so an unwired operation can
// never silently inherit mock/reference behaviour. Every stable + quarantined
// operation is either routed to a real `#[napi]` export (and listed in
// NATIVE_WIRED_OPERATIONS) or throws a classified `operation_unimplemented`.
// `native-fail-closed.test.ts` enforces that this stays true for every manifest op.

export { NATIVE_WIRED_OPERATIONS } from './generated/operations.js';

const OPERATION_BY_FACADE_METHOD: ReadonlyMap<string, string> = new Map(
  MANIFEST_OPERATIONS.map((operation) => [operation.facadeMethod, operation.manifestName]),
);

type NativeFacadeMethod = (...args: NativeFacadeValue[]) => NativeFacadeValue;

interface NativeProjectBundleCompositionStatus {
  readonly loadedProjectBundle: number | null;
  readonly fatalCount: number;
  readonly totalCount: number;
  readonly blocksLoad: boolean;
}

interface NativeWorkspaceAuthoringProjectionReceipt
  extends Omit<WorkspaceAuthoringProjectionSummary, 'frame'> {
  readonly frameJson: string;
}

function projectBundleCompositionStatusFromNative(
  status: NativeProjectBundleCompositionStatus,
): CompositionStatus {
  return {
    loadedProjectBundle: status.loadedProjectBundle ?? null,
    fatalCount: status.fatalCount,
    totalCount: status.totalCount,
    blocksLoad: status.blocksLoad,
  };
}

function nativeVec3(value: readonly [number, number, number], field: string): { readonly x: number; readonly y: number; readonly z: number } {
  if (value.length !== 3 || value.some((component) => !Number.isFinite(component))) {
    throw new RuntimeBridgeError('invalid_input', `${field} must be a finite vec3`);
  }
  return { x: value[0], y: value[1], z: value[2] };
}

function nativeOptionalObject<T extends object>(value: T | null): T | undefined {
  return value == null ? undefined : value;
}

function requiredString(value: string | null | undefined, field: string): string {
  if (typeof value !== 'string' || value.trim().length === 0) {
    throw new RuntimeBridgeError('invalid_input', `${field} must be a non-empty string`);
  }
  return value;
}

function requiredStringArray(value: readonly string[] | null | undefined, field: string): readonly string[] {
  if (!isTypedArray(value)) {
    throw new RuntimeBridgeError('invalid_input', `${field} must be an array of non-empty strings`);
  }
  return value.map((entry, index) => requiredString(entry, `${field}[${index}]`));
}

function requiredBoolean(value: boolean, field: string): boolean {
  if (typeof value !== 'boolean') {
    throw new RuntimeBridgeError('internal', `native ${field} was not a boolean`);
  }
  return value;
}

function bridgeVec3(
  value: { readonly x: number; readonly y: number; readonly z: number },
  field: string,
): readonly [number, number, number] {
  if (!Number.isFinite(value.x) || !Number.isFinite(value.y) || !Number.isFinite(value.z)) {
    throw new RuntimeBridgeError('internal', `native ${field} was not a finite vec3`);
  }
  return [value.x, value.y, value.z];
}

function bridgeVec3Array(value: readonly number[], field: string): readonly [number, number, number] {
  if (value.length !== 3 || value.some((component) => !Number.isFinite(component))) {
    throw new RuntimeBridgeError('internal', 'native ' + field + ' was not a finite vec3');
  }
  return [value[0]!, value[1]!, value[2]!];
}

function isTypedArray<T>(value: readonly T[] | null | undefined): value is readonly T[] {
  return Array.isArray(value);
}

function nativeAuthoritySource(value: string): 'seeded_from_request' | 'rust_entity_store' {
  if (value === 'seeded_from_request' || value === 'rust_entity_store') {
    return value;
  }
  throw new RuntimeBridgeError('internal', `unknown native enemy movement authority source '${value}'`);
}

function fpsBackend(value: string): FpsRuntimeAuthorityTransport {
  if (value === 'native_rust' || value === 'reference_bridge') {
    return value;
  }
  // The Rust engine bridge reports engine_bridge_rust internally; the TS
  // native facade classifies the transport path as native_rust.
  if (value === 'engine_bridge_rust') {
    return 'native_rust';
  }
  throw new RuntimeBridgeError('internal', `unknown native FPS backend '${value}'`);
}

function fpsRole(value: FpsRuntimeRole): FpsRuntimeRole {
  if (value === 'player' || value === 'enemy' || value === 'neutral') return value;
  throw new RuntimeBridgeError('invalid_input', `unknown FPS role '${String(value)}'`);
}

function fpsLifecycleStatus(value: { readonly state: string; readonly entity?: number; readonly tick?: number }): FpsLifecycleStatus {
  if (value.state === 'active') return { state: 'active' };
  if (value.state === 'enemy_defeated') {
    return {
      state: 'enemy_defeated',
      entity: nonNegativeSafeInteger(value.entity ?? -1, 'lifecycleStatus.entity'),
      tick: nonNegativeSafeInteger(value.tick ?? -1, 'lifecycleStatus.tick'),
    };
  }
  throw new RuntimeBridgeError('internal', `unknown native FPS lifecycle status '${value.state}'`);
}

function normalizeFpsPrimaryFireResult(result: FpsPrimaryFireResult): FpsPrimaryFireResult {
  return {
    ...result,
    backend: fpsBackend(result.backend),
    target: result.target ?? null,
    targetHealthBefore: result.targetHealthBefore ?? null,
    targetHealthAfter: result.targetHealthAfter ?? null,
    lifecycleStatus: fpsLifecycleStatus(result.lifecycleStatus),
    targetRenderVisible: result.targetRenderVisible ?? null,
    entityHash: hashString(result.entityHash, 'entityHash'),
    healthHash: hashString(result.healthHash, 'healthHash'),
    replayHash: hashString(result.replayHash, 'replayHash'),
  };
}

function nativeModuleViewScope(scope: GameplayModuleViewScope): {
  readonly kind: 'session' | 'entity' | 'prefabInstance';
  readonly value: number | undefined;
} {
  if (scope.kind === 'session') return { kind: 'session', value: undefined };
  if (scope.kind === 'entity') {
    return { kind: 'entity', value: nonNegativeSafeInteger(scope.entity, 'scope.entity') };
  }
  if (scope.kind === 'prefabInstance') {
    return {
      kind: 'prefabInstance',
      value: nonNegativeSafeInteger(scope.instance, 'scope.instance'),
    };
  }
  throw new RuntimeBridgeError('invalid_input', 'unknown gameplay module view scope');
}

function normalizeComposedRuntimeSessionReadout(
  value: ComposedRuntimeSessionReadout,
): ComposedRuntimeSessionReadout {
  return {
    ...value,
    schemaVersion: u32(value.schemaVersion, 'schemaVersion'),
    entityAuthorityHash: hashString(value.entityAuthorityHash, 'entityAuthorityHash'),
    fpsSessionEpoch: nonNegativeSafeInteger(value.fpsSessionEpoch, 'fpsSessionEpoch'),
    fpsReplayHash: value.fpsReplayHash == null
      ? null
      : hashString(value.fpsReplayHash, 'fpsReplayHash'),
    runtimeSessionHash: hashString(value.runtimeSessionHash, 'runtimeSessionHash'),
    gameplay: {
      ...value.gameplay,
      compatibilityDiagnostics: value.gameplay.compatibilityDiagnostics.map((diagnostic) => ({
        ...diagnostic,
        expected: diagnostic.expected ?? null,
        actual: diagnostic.actual ?? null,
      })),
      gameplayRegistryDigest: hashString(
        value.gameplay.gameplayRegistryDigest,
        'gameplay.gameplayRegistryDigest',
      ),
      semanticCompatibilityDigest: hashString(
        value.gameplay.semanticCompatibilityDigest,
        'gameplay.semanticCompatibilityDigest',
      ),
      artifactProvenanceDigest: hashString(
        value.gameplay.artifactProvenanceDigest,
        'gameplay.artifactProvenanceDigest',
      ),
      bindingRegistryHash: hashString(
        value.gameplay.bindingRegistryHash,
        'gameplay.bindingRegistryHash',
      ),
      activationHash: hashString(value.gameplay.activationHash, 'gameplay.activationHash'),
      moduleStateHash: hashString(value.gameplay.moduleStateHash, 'gameplay.moduleStateHash'),
      authorityStateHash: hashString(
        value.gameplay.authorityStateHash,
        'gameplay.authorityStateHash',
      ),
      triggerRevision: nonNegativeSafeInteger(
        value.gameplay.triggerRevision,
        'gameplay.triggerRevision',
      ),
      triggerSnapshotHash: hashString(
        value.gameplay.triggerSnapshotHash,
        'gameplay.triggerSnapshotHash',
      ),
      activeOverlapCount: u32(value.gameplay.activeOverlapCount, 'gameplay.activeOverlapCount'),
      reactionFrameCount: u32(value.gameplay.reactionFrameCount, 'gameplay.reactionFrameCount'),
      lastReactionFrameHash: value.gameplay.lastReactionFrameHash == null
        ? null
        : hashString(value.gameplay.lastReactionFrameHash, 'gameplay.lastReactionFrameHash'),
      decisionReceiptCount: u32(
        value.gameplay.decisionReceiptCount,
        'gameplay.decisionReceiptCount',
      ),
      pendingDecisionCount: u32(
        value.gameplay.pendingDecisionCount,
        'gameplay.pendingDecisionCount',
      ),
      lastDecisionReceiptHash: value.gameplay.lastDecisionReceiptHash == null
        ? null
        : hashString(value.gameplay.lastDecisionReceiptHash, 'gameplay.lastDecisionReceiptHash'),
      schedulerStateHash: hashString(
        value.gameplay.schedulerStateHash,
        'gameplay.schedulerStateHash',
      ),
      schedulerPendingActionCount: u32(
        value.gameplay.schedulerPendingActionCount,
        'gameplay.schedulerPendingActionCount',
      ),
      schedulerOutstandingDispatchCount: u32(
        value.gameplay.schedulerOutstandingDispatchCount,
        'gameplay.schedulerOutstandingDispatchCount',
      ),
      schedulerOutstandingEventDeliveryCount: u32(
        value.gameplay.schedulerOutstandingEventDeliveryCount,
        'gameplay.schedulerOutstandingEventDeliveryCount',
      ),
      schedulerFactCount: u32(value.gameplay.schedulerFactCount, 'gameplay.schedulerFactCount'),
      schedulerTruncated: requiredBoolean(
        value.gameplay.schedulerTruncated,
        'gameplay.schedulerTruncated',
      ),
      runtimeHostHash: hashString(value.gameplay.runtimeHostHash, 'gameplay.runtimeHostHash'),
    },
  };
}

function hashString(value: string, field: string): string {
  if (!/^fnv1a64:[0-9a-f]{16}$/u.test(value)) {
    throw new RuntimeBridgeError('internal', `native ${field} was not an fnv1a64 hash`);
  }
  return value;
}

function hexHashString(value: string, field: string): string {
  if (!/^[0-9a-f]{16}$/u.test(value)) {
    throw new RuntimeBridgeError('internal', `native ${field} was not a 16-character hex hash`);
  }
  return value;
}

function generatedTunnelPreset(value: string): 'tiny-enclosed' {
  if (value !== 'tiny-enclosed') {
    throw new RuntimeBridgeError('internal', 'native generated tunnel preset was unknown');
  }
  return value;
}

function normalizeFpsSnapshot(value: FpsRuntimeSessionSnapshot): FpsRuntimeSessionSnapshot {
  return {
    ...value,
    backend: fpsBackend(value.backend),
    lifecycleStatus: fpsLifecycleStatus(value.lifecycleStatus),
    entityHash: hashString(value.entityHash, 'entityHash'),
    healthHash: hashString(value.healthHash, 'healthHash'),
    replayHash: hashString(value.replayHash, 'replayHash'),
    replayRecords: value.replayRecords.map((record) => ({
      ...record,
      entityHash: hashString(record.entityHash, 'replayRecords.entityHash'),
      healthHash: hashString(record.healthHash, 'replayRecords.healthHash'),
      recordHash: hashString(record.recordHash, 'replayRecords.recordHash'),
    })),
  };
}

function normalizeEncounterSnapshot(value: FpsEncounterDirectorSnapshot): FpsEncounterDirectorSnapshot {
  return {
    ...value,
    backend: fpsBackend(value.backend),
    encounterHash: hashString(value.encounterHash, 'encounterHash'),
    replayHash: hashString(value.replayHash, 'replayHash'),
  };
}

function normalizeEncounterTransition(value: FpsEncounterTransitionResult): FpsEncounterTransitionResult {
  return {
    ...value,
    backend: fpsBackend(value.backend),
    encounterHash: hashString(value.encounterHash, 'encounterHash'),
    replayHash: hashString(value.replayHash, 'replayHash'),
  };
}

function nativeFpsLoadRequest(request: FpsRuntimeSessionLoadRequest) {
  if (request.projectBundle.trim() === '') {
    throw new RuntimeBridgeError('invalid_input', 'projectBundle is required');
  }
  if (request.definitions.length === 0) {
    throw new RuntimeBridgeError('invalid_input', 'definitions must not be empty');
  }
  const definitions = request.definitions.map((definition, index) => {
    nonNegativeSafeInteger(definition.entity, `definitions[${index}].entity`);
    fpsRole(definition.role);
    const stableId = requiredString(definition.stableId, `definitions[${index}].stableId`);
    const displayName = requiredString(definition.displayName, `definitions[${index}].displayName`);
    const sourcePath = requiredString(definition.sourcePath, `definitions[${index}].sourcePath`);
    const tags = requiredStringArray(definition.tags, `definitions[${index}].tags`);
    const transform = definition.transform == null
      ? null
      : {
          translation: nativeVec3(definition.transform.translation, `definitions[${index}].transform.translation`),
          rotation: definition.transform.rotation,
          scale: nativeVec3(definition.transform.scale, `definitions[${index}].transform.scale`),
        };
    if (definition.transform != null) {
      if (definition.transform.rotation.length !== 4 || definition.transform.rotation.some((value) => !Number.isFinite(value))) {
        throw new RuntimeBridgeError('invalid_input', `definitions[${index}].transform.rotation must be a finite quat`);
      }
    }
    const bounds = definition.bounds == null
      ? null
      : {
          min: nativeVec3(definition.bounds.min, `definitions[${index}].bounds.min`),
          max: nativeVec3(definition.bounds.max, `definitions[${index}].bounds.max`),
        };
    if (definition.bounds != null) {
    }
    if (definition.health != null) {
      u32(definition.health.current, `definitions[${index}].health.current`);
      u32(definition.health.max, `definitions[${index}].health.max`);
    }
    if (definition.weapon != null) {
      requiredString(definition.weapon.weaponId, `definitions[${index}].weapon.weaponId`);
      u32(definition.weapon.damage, `definitions[${index}].weapon.damage`);
      u32(definition.weapon.rangeUnits, `definitions[${index}].weapon.rangeUnits`);
      u32(definition.weapon.ammo, `definitions[${index}].weapon.ammo`);
      u32(definition.weapon.cooldownTicksAfterFire, `definitions[${index}].weapon.cooldownTicksAfterFire`);
    }
    const policyBinding = definition.policyBinding == null
      ? undefined
      : {
          bindingId: requiredString(definition.policyBinding.bindingId, `definitions[${index}].policyBinding.bindingId`),
          policyId: requiredString(definition.policyBinding.policyId, `definitions[${index}].policyBinding.policyId`),
          viewKind: requiredString(definition.policyBinding.viewKind, `definitions[${index}].policyBinding.viewKind`),
          viewVersion: requiredString(definition.policyBinding.viewVersion, `definitions[${index}].policyBinding.viewVersion`),
          allowedIntents: requiredStringArray(definition.policyBinding.allowedIntents, `definitions[${index}].policyBinding.allowedIntents`),
          runtimeMoment: requiredString(definition.policyBinding.runtimeMoment, `definitions[${index}].policyBinding.runtimeMoment`),
        };
    return {
      entity: definition.entity,
      stableId,
      displayName,
      sourcePath,
      role: definition.role,
      transform: nativeOptionalObject(transform),
      bounds: nativeOptionalObject(bounds),
      tags: [...tags],
      renderVisible: definition.renderVisible,
      staticCollider: definition.staticCollider,
      health: nativeOptionalObject(definition.health),
      weapon: definition.weapon == null
        ? undefined
        : {
            weaponId: definition.weapon.weaponId,
            damage: definition.weapon.damage,
            rangeUnits: definition.weapon.rangeUnits,
            ammo: definition.weapon.ammo,
            cooldownTicksAfterFire: definition.weapon.cooldownTicksAfterFire,
          },
      policyBinding,
    };
  });
  return {
    projectBundle: request.projectBundle,
    bootstrapResolutionRegistry: nativeBootstrapResolutionRegistry(request.bootstrapResolutionRegistry),
    sceneDocument: request.sceneDocument,
    definitions,
  };
}

function nativeBootstrapResolutionRegistry(
  registry: FpsRuntimeSessionLoadRequest['bootstrapResolutionRegistry'],
) {
  if (registry.schemaVersion !== 1) {
    throw new RuntimeBridgeError('invalid_input', 'bootstrapResolutionRegistry.schemaVersion must be 1');
  }
  const uniqueStrings = (values: readonly string[], field: string): readonly string[] => {
    const normalized = requiredStringArray(values, field);
    if (new Set(normalized).size !== normalized.length) {
      throw new RuntimeBridgeError('invalid_input', `${field} must not contain duplicates`);
    }
    return normalized;
  };
  const entityDefinitionIds = uniqueStrings(
    registry.entityDefinitionIds,
    'bootstrapResolutionRegistry.entityDefinitionIds',
  );
  const catalogIds = uniqueStrings(registry.catalogIds, 'bootstrapResolutionRegistry.catalogIds');
  const prefabIds = registry.prefabIds.map((prefabId, index) => {
    nonNegativeSafeInteger(prefabId, `bootstrapResolutionRegistry.prefabIds[${index}]`);
    if (prefabId === 0) {
      throw new RuntimeBridgeError(
        'invalid_input',
        `bootstrapResolutionRegistry.prefabIds[${index}] must be positive`,
      );
    }
    return prefabId;
  });
  if (new Set(prefabIds).size !== prefabIds.length) {
    throw new RuntimeBridgeError('invalid_input', 'bootstrapResolutionRegistry.prefabIds must not contain duplicates');
  }
  const generatorPresets = registry.generatorPresets.map((preset, index) => ({
    providerId: requiredString(
      preset.providerId,
      `bootstrapResolutionRegistry.generatorPresets[${index}].providerId`,
    ),
    presetId: requiredString(
      preset.presetId,
      `bootstrapResolutionRegistry.generatorPresets[${index}].presetId`,
    ),
  }));
  const generatorIdentities = generatorPresets.map((preset) => `${preset.providerId}\u0000${preset.presetId}`);
  if (new Set(generatorIdentities).size !== generatorIdentities.length) {
    throw new RuntimeBridgeError(
      'invalid_input',
      'bootstrapResolutionRegistry.generatorPresets must not contain duplicates',
    );
  }
  return {
    schemaVersion: 1,
    entityDefinitionIds,
    prefabIds,
    generatorPresets,
    catalogIds,
  } as const;
}

export class NativeRuntimeBridge implements RuntimeBridge {
  readonly #addon: NativeAddon;
  #seed = 0;
  #initialized = false;

  #engineHandle: EngineHandle | null = null;

  constructor(addon: NativeAddon) {
    this.#addon = addon;
    return new Proxy(this, {
      get: (target, property) => {
        const member = Reflect.get(target, property, target) as NativeFacadeMethod | NativeFacadeValue;
        if (typeof property !== 'string' || typeof member !== 'function') return member;
        const operation = OPERATION_BY_FACADE_METHOD.get(property);
        if (operation === undefined) {
          return (...args: NativeFacadeValue[]) =>
            Reflect.apply(member, target, args) as NativeFacadeValue;
        }
        return (...args: NativeFacadeValue[]) => {
          const input = args.length === 0 ? null : args[0] ?? null;
          return runNativeOperation(
            operation,
            input,
            () => Reflect.apply(member, target, args) as NativeFacadeValue,
          );
        };
      },
    });
  }

  // ── Wired native operations ───────────────────────────────────────────────
  initializeEngine(config: EngineConfig): EngineHandle {
    if (!Number.isInteger(config.seed) || config.seed < 0) {
      throw new RuntimeBridgeError('invalid_input', `seed must be a non-negative integer`);
    }
    this.#seed = config.seed;
    const handle = callNative(() => this.#addon.initializeEngine(config.seed)) as EngineHandle;
    this.#engineHandle = handle;
    this.#initialized = true;
    return handle;
  }

  openWorkspaceAuthoring(input: WorkspaceAuthoringOpenInput): WorkspaceAuthoringStateSummary {
    const existingHandle = this.#engineHandle ?? -1;
    const handle = callNative(() =>
      this.#addon.openWorkspaceAuthoring(existingHandle, JSON.stringify(input)),
    );
    this.#engineHandle = nonNegativeSafeInteger(handle, 'workspace authoring handle') as EngineHandle;
    this.#seed = input.seed;
    this.#initialized = true;
    const statePayload = callNative(() =>
      this.#addon.readWorkspaceAuthoringState(this.#engineHandle as EngineHandle),
    );
    return parseNativeJson<WorkspaceAuthoringStateSummary>(
      statePayload,
      'workspace authoring open state',
    );
  }

  readWorkspaceAuthoringState(): WorkspaceAuthoringStateSummary {
    const handle = this.#requireHandle('readWorkspaceAuthoringState');
    const payload = callNative(() => this.#addon.readWorkspaceAuthoringState(handle));
    return parseNativeJson<WorkspaceAuthoringStateSummary>(
      payload,
      'workspace authoring state',
    );
  }

  readWorkspaceAuthoringProjection(
    request: WorkspaceAuthoringProjectionRequest,
  ): WorkspaceAuthoringProjectionSummary {
    const handle = this.#requireHandle('readWorkspaceAuthoringProjection');
    const payload = callNative(() =>
      this.#addon.readWorkspaceAuthoringProjection(handle, JSON.stringify(request)),
    );
    const receipt = parseGeneratedOperationOutput<NativeWorkspaceAuthoringProjectionReceipt>(
      'read_workspace_authoring_projection',
      'projectBundle.WorkspaceAuthoringProjectionReceipt',
      payload,
    );
    const { frameJson, ...summary } = receipt;
    const framePayload = parseGeneratedOperationOutput<Record<string, unknown>>(
      'read_workspace_authoring_projection',
      'render.RenderFrameDiff',
      frameJson,
    );
    return {
      ...summary,
      cursor: frameCursor(nonNegativeSafeInteger(summary.cursor as number, 'projection cursor')),
      nextCursor: frameCursor(
        nonNegativeSafeInteger(summary.nextCursor as number, 'projection next cursor'),
      ),
      frame: decodeRenderFrameDiff(framePayload),
    };
  }

  confirmWorkspaceAuthoringStored(
    input: WorkspaceAuthoringStoredConfirmationInput,
  ): WorkspaceAuthoringStoredConfirmationReceipt {
    const handle = this.#requireHandle('confirmWorkspaceAuthoringStored');
    const payload = callNative(() =>
      this.#addon.confirmWorkspaceAuthoringStored(handle, JSON.stringify(input)),
    );
    return parseNativeJson<WorkspaceAuthoringStoredConfirmationReceipt>(
      payload,
      'workspace authoring stored confirmation',
    );
  }

  prepareProjectWrite(input: ProjectWritePrepareRequest): ProjectWritePrepareReceipt {
    const handle = this.#requireHandle('prepareProjectWrite');
    const payload = callNative(() =>
      this.#addon.prepareProjectWrite(handle, JSON.stringify(input)),
    );
    return parseGeneratedOperationOutput<ProjectWritePrepareReceipt>(
      'prepare_project_write',
      'projectBundle.ProjectWritePrepareReceipt',
      payload,
    );
  }

  confirmProjectWrite(input: ProjectWriteConfirmRequest): ProjectWriteConfirmReceipt {
    const handle = this.#requireHandle('confirmProjectWrite');
    const payload = callNative(() =>
      this.#addon.confirmProjectWrite(handle, JSON.stringify(input)),
    );
    return parseGeneratedOperationOutput<ProjectWriteConfirmReceipt>(
      'confirm_project_write',
      'projectBundle.ProjectWriteConfirmReceipt',
      payload,
    );
  }

  closeWorkspaceAuthoring(input: WorkspaceAuthoringCloseInput): WorkspaceAuthoringCloseReceipt {
    const handle = this.#requireHandle('closeWorkspaceAuthoring');
    const payload = callNative(() =>
      this.#addon.closeWorkspaceAuthoring(
        handle,
        JSON.stringify({
          ...input,
          discardUnsavedWorkingState: input.discardUnsavedWorkingState ?? false,
        }),
      ),
    );
    return parseNativeJson<WorkspaceAuthoringCloseReceipt>(
      payload,
      'workspace authoring close receipt',
    );
  }

  #requireHandle(operation: string): EngineHandle {
    if (!this.#initialized || this.#engineHandle === null) {
      throw new RuntimeBridgeError('not_initialized', `${operation} before initializeEngine`);
    }
    return this.#engineHandle;
  }

  configureInputSession(request: InputSessionConfigureRequest): InputSessionSnapshot {
    const handle = this.#requireHandle('configureInputSession');
    const payload = callNative(() =>
      this.#addon.configureInputSession(handle, JSON.stringify(request)),
    );
    return parseNativeJson<InputSessionSnapshot>(payload, 'input session snapshot');
  }

  applyInputContextCommand(command: InputContextCommand): InputContextChangeReceipt {
    const handle = this.#requireHandle('applyInputContextCommand');
    const payload = callNative(() =>
      this.#addon.applyInputContextCommand(handle, JSON.stringify(command)),
    );
    return parseNativeJson<InputContextChangeReceipt>(payload, 'input context change receipt');
  }

  submitRawInput(sample: RawInputSample): InputResolutionReceipt {
    const handle = this.#requireHandle('submitRawInput');
    const payload = callNative(() => this.#addon.submitRawInput(handle, JSON.stringify(sample)));
    return parseNativeJson<InputResolutionReceipt>(payload, 'input resolution receipt');
  }

  replayResolvedInputAction(record: RecordedInputAction): InputActionReplayReceipt {
    const handle = this.#requireHandle('replayResolvedInputAction');
    const payload = callNative(() =>
      this.#addon.replayResolvedInputAction(handle, JSON.stringify(record)),
    );
    return parseNativeJson<InputActionReplayReceipt>(payload, 'input action replay receipt');
  }

  readInputContextState(): InputContextStackState {
    const handle = this.#requireHandle('readInputContextState');
    const payload = callNative(() => this.#addon.readInputContextState(handle));
    return parseNativeJson<InputContextStackState>(payload, 'input context state');
  }

  applyTimeControlCommand(command: TimeControlCommand): TimeControlReceipt {
    const handle = this.#requireHandle('applyTimeControlCommand');
    const payload = callNative(() =>
      this.#addon.applyTimeControlCommand(handle, JSON.stringify(command)),
    );
    return parseNativeJson<TimeControlReceipt>(payload, 'time control receipt');
  }

  readTimeControlState(): TimeControlState {
    const handle = this.#requireHandle('readTimeControlState');
    const payload = callNative(() => this.#addon.readTimeControlState(handle));
    return parseNativeJson<TimeControlState>(payload, 'time control state');
  }

  loadProjectBundle(request: ProjectBundleLoadRequest): CompositionStatus {
    const handle = this.#requireHandle('loadProjectBundle');
    const bundleSchemaVersion = u32(request.bundleSchemaVersion, 'bundleSchemaVersion');
    const protocolVersion = u32(request.protocolVersion, 'protocolVersion');
    const sceneId = nonNegativeSafeInteger(request.sceneId, 'sceneId');
    const status = callNative(() =>
      this.#addon.loadProjectBundle(handle, bundleSchemaVersion, protocolVersion, sceneId),
    );
    return projectBundleCompositionStatusFromNative(status);
  }

  beginRuntimeProjectSourceResources(
    request: ProjectResourceBeginRequest,
  ): ProjectResourceTransactionReceipt {
    const handle = this.#requireHandle('beginRuntimeProjectSourceResources');
    const receipt = callNative(() =>
      this.#addon.beginRuntimeProjectSourceResources(handle, JSON.stringify(request)),
    );
    return {
      generation: nonNegativeSafeInteger(receipt.generation, 'project resource generation'),
      manifestHash: String(receipt.manifestHash),
    };
  }

  stageRuntimeProjectSourceResource(
    request: ProjectResourceStageInput,
  ): StagedProjectResourceRef {
    const handle = this.#requireHandle('stageRuntimeProjectSourceResource');
    const generation = nonNegativeSafeInteger(request.generation, 'project resource generation');
    const receipt = callNative(() =>
      this.#addon.stageRuntimeProjectSourceResource(handle, generation, request.path, request.bytes),
    );
    return {
      handle: nonNegativeSafeInteger(receipt.handle, 'project resource handle'),
      generation: nonNegativeSafeInteger(receipt.generation, 'project resource generation'),
      version: u32(receipt.version, 'project resource version'),
      byteLen: nonNegativeSafeInteger(receipt.byteLen, 'project resource byte length'),
    };
  }

  admitRuntimeProjectSourceBatch(
    request: RuntimeProjectSourceBatch,
  ): ProjectSourceBatchValidationReceipt {
    const handle = this.#requireHandle('admitRuntimeProjectSourceBatch');
    const payload = callNative(() =>
      this.#addon.admitRuntimeProjectSourceBatch(handle, JSON.stringify(request)),
    );
    return parseGeneratedOperationOutput<ProjectSourceBatchValidationReceipt>(
      'admit_runtime_project_source_batch',
      'projectBundle.ProjectSourceBatchValidationReceipt',
      payload,
    );
  }

  loadRuntimeProject(request: RuntimeProjectLoadRequest): RuntimeProjectLoadReceipt {
    const handle = this.#requireHandle('loadRuntimeProject');
    const payload = callNative(() =>
      this.#addon.loadRuntimeProject(handle, JSON.stringify(request)),
    );
    return parseGeneratedOperationOutput<RuntimeProjectLoadReceipt>(
      'load_runtime_project',
      'projectBundle.RuntimeProjectLoadReceipt',
      payload,
    );
  }

  closeRuntimeProject(request: RuntimeProjectCloseRequest): RuntimeProjectCloseReceipt {
    const handle = this.#requireHandle('closeRuntimeProject');
    const payload = callNative(() =>
      this.#addon.closeRuntimeProject(handle, JSON.stringify(request)),
    );
    return parseGeneratedOperationOutput<RuntimeProjectCloseReceipt>(
      'close_runtime_project',
      'projectBundle.RuntimeProjectCloseReceipt',
      payload,
    );
  }

  submitCommands(batch: CommandBatch): CommandResult {
    const handle = this.#requireHandle('submitCommands');
    return callNative(() => this.#addon.submitCommands(handle, JSON.stringify(batch.commands)));
  }

  stepSimulation(input: StepInputEnvelope): StepResult {
    const handle = this.#requireHandle('stepSimulation');
    const tick = nonNegativeSafeInteger(input.tick, 'tick');
    const result = callNative(() => this.#addon.stepSimulation(handle, tick));
    return {
      tick: nonNegativeSafeInteger(result.tick, 'native step tick'),
      diffCount: u32(result.diffCount, 'native step diffCount'),
    };
  }

  applyEnemyDirectNavMovement(request: EnemyDirectNavMovementRequest): EnemyDirectNavMovementResult {
    const handle = this.#requireHandle('applyEnemyDirectNavMovement');
    const entity = nonNegativeSafeInteger(request.entity, 'entity');
    if (entity === 0) {
      throw new RuntimeBridgeError('invalid_input', 'entity must be positive');
    }
    const seedPosition = nativeVec3(request.seedPosition, 'seedPosition');
    const target = nativeVec3(request.target, 'target');
    if (!Number.isFinite(request.maxStepUnits) || request.maxStepUnits <= 0) {
      throw new RuntimeBridgeError('invalid_input', 'maxStepUnits must be finite and positive');
    }
    const result = callNative(() =>
      this.#addon.applyEnemyDirectNavMovement(
        handle,
        entity,
        seedPosition,
        target,
        request.maxStepUnits,
      ),
    );
    return {
      entity: result.entity,
      authoritySource: nativeAuthoritySource(result.authoritySource),
      authorityTransport: 'native_rust',
      from: bridgeVec3(result.from, 'from'),
      target: bridgeVec3(result.target, 'target'),
      nextWaypoint: bridgeVec3(result.nextWaypoint, 'nextWaypoint'),
      distanceUnits: result.distanceUnits,
      reached: result.reached,
      pathHash: result.pathHash,
      transformHash: result.transformHash,
      projectionChanged: result.projectionChanged,
    };
  }

  loadFpsRuntimeSession(request: FpsRuntimeSessionLoadRequest): FpsRuntimeSessionSnapshot {
    const handle = this.#requireHandle('loadFpsRuntimeSession');
    const nativeRequest = nativeFpsLoadRequest(request);
    const gameRuleModules = request.gameRuleModules ?? [];
    const result = callNative(() =>
      this.#addon.loadFpsRuntimeSession(
        handle,
        nativeRequest.projectBundle,
        JSON.stringify(nativeRequest.sceneDocument),
        JSON.stringify(nativeRequest.bootstrapResolutionRegistry),
        nativeRequest.definitions,
        JSON.stringify(gameRuleModules),
      ) as FpsRuntimeSessionSnapshot,
    );
    return normalizeFpsSnapshot(result);
  }

  readFpsRuntimeSession(): FpsRuntimeSessionSnapshot {
    const handle = this.#requireHandle('readFpsRuntimeSession');
    const result = callNative(() => this.#addon.readFpsRuntimeSession(handle) as FpsRuntimeSessionSnapshot);
    return normalizeFpsSnapshot(result);
  }

  applyFpsPrimaryFire(request: FpsPrimaryFireRequest): FpsPrimaryFireResult {
    const handle = this.#requireHandle('applyFpsPrimaryFire');
    const tick = nonNegativeSafeInteger(request.tick, 'tick');
    const origin = nativeVec3(request.origin, 'origin');
    const direction = nativeVec3(request.direction, 'direction');
    const shooterRole = request.shooterRole === undefined ? undefined : fpsRole(request.shooterRole);
    const targetRole = request.targetRole === undefined ? undefined : fpsRole(request.targetRole);
    const result = callNative(() =>
      this.#addon.applyFpsPrimaryFire(handle, tick, origin, direction, shooterRole, targetRole) as FpsPrimaryFireResult,
    );
    return normalizeFpsPrimaryFireResult(result);
  }

  readComposedRuntimeSession(): ComposedRuntimeSessionReadout {
    const handle = this.#requireHandle('readComposedRuntimeSession');
    const result = callNative(() => this.#addon.readComposedRuntimeSession(handle));
    return normalizeComposedRuntimeSessionReadout(result);
  }

  readGameplayModuleView(request: GameplayModuleViewRequest): GameplayModuleViewSnapshot {
    const handle = this.#requireHandle('readGameplayModuleView');
    const namespace = requiredString(request.view.namespace, 'view.namespace');
    const name = requiredString(request.view.name, 'view.name');
    const version = u32(request.view.version, 'view.version');
    const schemaHash = requiredString(request.view.schemaHash, 'view.schemaHash');
    const scope = nativeModuleViewScope(request.scope);
    const expectedRuntimeSessionHash = hashString(
      request.expectedRuntimeSessionHash,
      'expectedRuntimeSessionHash',
    );
    const result = callNative(() => this.#addon.readGameplayModuleView(
      handle,
      namespace,
      name,
      version,
      schemaHash,
      scope.kind,
      scope.value,
      expectedRuntimeSessionHash,
    ));
    const resultScope: GameplayModuleViewScope = result.scopeKind === 'session'
      ? { kind: 'session' }
      : result.scopeKind === 'entity'
        ? { kind: 'entity', entity: nonNegativeSafeInteger(result.scopeValue ?? -1, 'scopeValue') }
        : result.scopeKind === 'prefabInstance'
          ? {
              kind: 'prefabInstance',
              instance: nonNegativeSafeInteger(result.scopeValue ?? -1, 'scopeValue'),
            }
          : (() => {
              throw new RuntimeBridgeError('internal', 'native module view returned an unknown scope');
            })();
    if (
      result.view.namespace !== namespace
      || result.view.name !== name
      || result.view.version !== version
      || result.view.schemaHash !== schemaHash
      || JSON.stringify(resultScope) !== JSON.stringify(request.scope)
    ) {
      throw new RuntimeBridgeError('internal', 'native module view identity did not match the request');
    }
    const canonicalPayload = Array.from(result.canonicalPayload);
    if (canonicalPayload.length > 8_388_608) {
      throw new RuntimeBridgeError('output_limit_exceeded', 'module view payload exceeded bridge limit');
    }
    return {
      view: result.view,
      providerId: requiredString(result.providerId, 'providerId'),
      scope: resultScope,
      revision: nonNegativeSafeInteger(result.revision, 'revision'),
      canonicalPayload,
      viewHash: hashString(result.viewHash, 'viewHash'),
      runtimeSessionHash: hashString(result.runtimeSessionHash, 'runtimeSessionHash'),
    };
  }

  applyGameplayPrefabPartInteraction(
    request: GameplayPrefabPartInteractionRequest,
  ): GameplayPrefabPartInteractionReceipt {
    const handle = this.#requireHandle('applyGameplayPrefabPartInteraction');
    const actor = nonNegativeSafeInteger(request.actor, 'actor');
    const instance = nonNegativeSafeInteger(request.instance, 'instance');
    const role = requiredString(request.role, 'role');
    const expectedTarget = nonNegativeSafeInteger(request.expectedTarget, 'expectedTarget');
    const tick = nonNegativeSafeInteger(request.tick, 'tick');
    const expectedRuntimeSessionHash = hashString(
      request.expectedRuntimeSessionHash,
      'expectedRuntimeSessionHash',
    );
    const result = callNative(() => this.#addon.applyGameplayPrefabPartInteraction(
      handle,
      actor,
      instance,
      role,
      expectedTarget,
      tick,
      expectedRuntimeSessionHash,
    ));
    if (
      result.actor !== actor
      || result.instance !== instance
      || result.role !== role
      || result.target !== expectedTarget
    ) {
      throw new RuntimeBridgeError('internal', 'native prefab interaction identity did not match the request');
    }
    return {
      actor,
      instance,
      role,
      target: expectedTarget,
      eventHash: hashString(result.eventHash, 'eventHash'),
      reactionFrameHash: hashString(result.reactionFrameHash, 'reactionFrameHash'),
      runtimeSessionHash: hashString(result.runtimeSessionHash, 'runtimeSessionHash'),
    };
  }

  invokeGameExtensionWeaponEffect(
    request: GameExtensionWeaponEffectInvocationRequest,
  ): GameExtensionWeaponEffectInvocationResult {
    const handle = this.#requireHandle('invokeGameExtensionWeaponEffect');
    const tick = nonNegativeSafeInteger(request.primaryFire.tick, 'primaryFire.tick');
    const origin = nativeVec3(request.primaryFire.origin, 'primaryFire.origin');
    const direction = nativeVec3(request.primaryFire.direction, 'primaryFire.direction');
    const shooterRole = request.primaryFire.shooterRole === undefined
      ? undefined
      : fpsRole(request.primaryFire.shooterRole);
    const targetRole = request.primaryFire.targetRole === undefined
      ? undefined
      : fpsRole(request.primaryFire.targetRole);
    const result = callNative(() =>
      this.#addon.invokeGameExtensionWeaponEffect(
        handle,
        JSON.stringify(request.hook),
        tick,
        origin,
        direction,
        shooterRole,
        targetRole,
      ),
    ) as {
      readonly hookReceiptJson: string;
      readonly replayEvidenceJson: string;
      readonly primaryFire?: FpsPrimaryFireResult | null;
    };
    return {
      hookReceipt: parseNativeJson<GameExtensionHookReceipt>(result.hookReceiptJson, 'game extension hook receipt'),
      replayEvidence: parseNativeJson<GameExtensionReplayEvidence>(
        result.replayEvidenceJson,
        'game extension replay evidence',
      ),
      primaryFire: result.primaryFire === undefined || result.primaryFire === null
        ? null
        : normalizeFpsPrimaryFireResult(result.primaryFire),
    };
  }

  validateGameRuleCatalog(catalog: GameRuleCatalog): GameRuleCatalogValidationReceipt {
    const handle = this.#requireHandle('validateGameRuleCatalog');
    return parseNativeJson<GameRuleCatalogValidationReceipt>(
      callNative(() => this.#addon.validateGameRuleCatalog(handle, JSON.stringify(catalog))),
      'game-rule catalog validation receipt',
    );
  }

  submitGameRuleEffectIntent(input: GameRuleEffectIntentRequest): GameRuleResolutionReceipt {
    const handle = this.#requireHandle('submitGameRuleEffectIntent');
    return parseNativeJson<GameRuleResolutionReceipt>(
      callNative(() =>
        this.#addon.submitGameRuleEffectIntent(
          handle,
          JSON.stringify(input.catalog),
          JSON.stringify(input.request),
        )),
      'game-rule resolution receipt',
    );
  }

  readGameRuleRuntimeReadout(): GameRuleRuntimeReadout {
    const handle = this.#requireHandle('readGameRuleRuntimeReadout');
    const readout = parseNativeJson<GameRuleRuntimeReadout>(
      callNative(() => this.#addon.readGameRuleRuntimeReadout(handle)),
      'game-rule runtime readout',
    );
    return { ...readout, backend: fpsBackend(readout.backend) };
  }

  restartFpsRuntimeSession(request: FpsRuntimeSessionRestartRequest): FpsRuntimeSessionSnapshot {
    const handle = this.#requireHandle('restartFpsRuntimeSession');
    const expectedEpoch = nonNegativeSafeInteger(request.expectedEpoch, 'expectedEpoch');
    const result = callNative(() =>
      this.#addon.restartFpsRuntimeSession(handle, expectedEpoch) as FpsRuntimeSessionSnapshot,
    );
    return normalizeFpsSnapshot(result);
  }

  readFpsEncounterDirector(lifecycle: FpsEncounterLifecycleInput): FpsEncounterDirectorSnapshot {
    const handle = this.#requireHandle('readFpsEncounterDirector');
    const result = callNative(() =>
      this.#addon.readFpsEncounterDirector(handle, lifecycle) as FpsEncounterDirectorSnapshot,
    );
    return normalizeEncounterSnapshot(result);
  }

  applyFpsEncounterTransition(request: FpsEncounterTransitionRequest): FpsEncounterTransitionResult {
    const handle = this.#requireHandle('applyFpsEncounterTransition');
    const result = callNative(() =>
      this.#addon.applyFpsEncounterTransition(handle, request) as FpsEncounterTransitionResult,
    );
    return normalizeEncounterTransition(result);
  }

  readModelMaterialPreview(request: ModelMaterialPreviewRequest): ModelMaterialPreviewSnapshot {
    const handle = this.#requireHandle('readModelMaterialPreview');
    const payload = callNative(() => this.#addon.readModelMaterialPreview(handle, JSON.stringify(request)));
    return parseNativeJson<ModelMaterialPreviewSnapshot>(payload, 'model material preview snapshot');
  }

  decodeSceneDocument(request: SceneDocumentDecodeRequest): SceneDocumentCodecResult {
    const handle = this.#requireHandle('decodeSceneDocument');
    const payload = callNative(() => this.#addon.decodeSceneDocument(handle, JSON.stringify(request)));
    return parseNativeJson<SceneDocumentCodecResult>(payload, 'scene document decode result');
  }

  encodeSceneDocument(request: SceneDocumentEncodeRequest): SceneDocumentCodecResult {
    const handle = this.#requireHandle('encodeSceneDocument');
    const payload = callNative(() => this.#addon.encodeSceneDocument(handle, JSON.stringify(request)));
    return parseNativeJson<SceneDocumentCodecResult>(payload, 'scene document encode result');
  }

  applySceneDocumentAuthoring(request: SceneDocumentAuthoringRequest): SceneDocumentAuthoringResult {
    const handle = this.#requireHandle('applySceneDocumentAuthoring');
    const payload = callNative(() => this.#addon.applySceneDocumentAuthoring(handle, JSON.stringify(request)));
    return parseNativeJson<SceneDocumentAuthoringResult>(payload, 'scene document authoring result');
  }

  decodeProjectContent(request: ProjectContentDecodeRequest): ProjectContentCodecResult {
    const handle = this.#requireHandle('decodeProjectContent');
    const payload = callNative(() => this.#addon.decodeProjectContent(handle, JSON.stringify(request)));
    return parseGeneratedOperationOutput<ProjectContentCodecResult>(
      'decode_project_content',
      'projectContent.ProjectContentCodecResult',
      payload,
    );
  }

  encodeProjectContent(request: ProjectContentEncodeRequest): ProjectContentCodecResult {
    const handle = this.#requireHandle('encodeProjectContent');
    const payload = callNative(() => this.#addon.encodeProjectContent(handle, JSON.stringify(request)));
    return parseGeneratedOperationOutput<ProjectContentCodecResult>(
      'encode_project_content',
      'projectContent.ProjectContentCodecResult',
      payload,
    );
  }

  applyProjectContentAuthoring(
    request: ProjectContentAuthoringRequest,
  ): ProjectContentAuthoringResult {
    const handle = this.#requireHandle('applyProjectContentAuthoring');
    const payload = callNative(() =>
      this.#addon.applyProjectContentAuthoring(handle, JSON.stringify(request)),
    );
    return parseGeneratedOperationOutput<ProjectContentAuthoringResult>(
      'apply_project_content_authoring',
      'projectContent.ProjectContentAuthoringResult',
      payload,
    );
  }

  previewProceduralEnvironment(
    request: ProceduralEnvironmentPreviewRequest,
  ): ProceduralEnvironmentPreviewResult {
    const handle = this.#requireHandle('previewProceduralEnvironment');
    const payload = callNative(() =>
      this.#addon.previewProceduralEnvironment(handle, JSON.stringify(request)),
    );
    return parseGeneratedOperationOutput<ProceduralEnvironmentPreviewResult>(
      'preview_procedural_environment',
      'projectContent.ProceduralEnvironmentPreviewResult',
      payload,
    );
  }

  applyProceduralEnvironment(
    request: ProceduralEnvironmentApplyRequest,
  ): ProceduralEnvironmentApplyResult {
    const handle = this.#requireHandle('applyProceduralEnvironment');
    const payload = callNative(() =>
      this.#addon.applyProceduralEnvironment(handle, JSON.stringify(request)),
    );
    return parseGeneratedOperationOutput<ProceduralEnvironmentApplyResult>(
      'apply_procedural_environment',
      'projectContent.ProceduralEnvironmentApplyResult',
      payload,
    );
  }

  readSceneObjectSnapshot(): SceneObjectSnapshot {
    const handle = this.#requireHandle('readSceneObjectSnapshot');
    const payload = callNative(() => this.#addon.readSceneObjectSnapshot(handle));
    return parseNativeJson<SceneObjectSnapshot>(payload, 'scene object snapshot');
  }

  applySceneObjectCommand(request: SceneObjectCommandRequest): SceneObjectCommandResult {
    const handle = this.#requireHandle('applySceneObjectCommand');
    const payload = callNative(() => this.#addon.applySceneObjectCommand(handle, JSON.stringify(request)));
    return parseNativeJson<SceneObjectCommandResult>(payload, 'scene object command result');
  }

  readRenderDiffs(cursor: FrameCursor): RenderFrameDiff {
    const handle = this.#requireHandle('readRenderDiffs');
    const frame = nonNegativeSafeInteger(cursor as number, 'frame cursor') as FrameCursor;
    const payload = callNative(() => this.#addon.readRenderDiffs(handle, frame));
    return decodeRenderFrameDiff(parseNativeJson<RenderFrameDiff>(payload, 'render frame diff'));
  }

  readProjectionFrame(cursor: FrameCursor): RuntimeProjectionFrame {
    const handle = this.#requireHandle('readProjectionFrame');
    const frame = nonNegativeSafeInteger(cursor as number, 'frame cursor') as FrameCursor;
    const nativeFrame = callNative(
      () => this.#addon.readProjectionFrame(handle, frame) as NativeRuntimeProjectionFrameDto,
    );
    return projectionFrameFromNative(nativeFrame);
  }

  readDeveloperConsole(): DeveloperConsoleSnapshot {
    const handle = this.#requireHandle('readDeveloperConsole');
    const nativeSnapshot = callNative(() => this.#addon.readDeveloperConsole(handle));
    const snapshot: DeveloperConsoleSnapshot = {
      ...nativeSnapshot,
      firstSequence: nativeSnapshot.firstSequence ?? null,
      records: nativeSnapshot.records.map((record) => ({
        ...record,
        correlation: record.correlation ?? null,
        authorityTick: record.authorityTick ?? null,
        session: record.session ?? null,
        detail: {
          ...record.detail,
          operation: record.detail.operation ?? null,
          resourceKind: record.detail.resourceKind ?? null,
          resourceId: record.detail.resourceId ?? null,
          reason: record.detail.reason ?? null,
        },
      })),
    };
    return snapshot;
  }

  saveProjectBundle(): ProjectBundleSaveSummary {
    const handle = this.#requireHandle('saveProjectBundle');
    return callNative(() => this.#addon.saveProjectBundle(handle) as ProjectBundleSaveSummary);
  }

  getProjectBundleCompositionStatus(): CompositionStatus {
    const handle = this.#requireHandle('getProjectBundleCompositionStatus');
    const status = callNative(() => this.#addon.getProjectBundleCompositionStatus(handle));
    return projectBundleCompositionStatusFromNative(status);
  }

  planVoxelConversion(request: VoxelConversionPlanRequest): VoxelConversionPlan {
    const handle = this.#requireHandle('planVoxelConversion');
    const payload = callNative(() => this.#addon.planVoxelConversion(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelConversionPlan>(payload, 'voxel conversion plan');
  }

  registerVoxelConversionSource(
    request: VoxelConversionSourceRegistrationRequest,
  ): VoxelConversionSourceRegistration {
    const handle = this.#requireHandle('registerVoxelConversionSource');
    const payload = callNative(() => this.#addon.registerVoxelConversionSource(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelConversionSourceRegistration>(payload, 'voxel conversion source registration');
  }

  registerVoxelConversionMeshAsset(
    request: VoxelConversionMeshAssetRegistrationRequest,
  ): VoxelConversionSourceRegistration {
    const handle = this.#requireHandle('registerVoxelConversionMeshAsset');
    const payload = callNative(() =>
      this.#addon.registerVoxelConversionMeshAsset(handle, JSON.stringify(request)),
    );
    return parseNativeJson<VoxelConversionSourceRegistration>(payload, 'voxel conversion mesh asset registration');
  }

  importVoxelConversionMeshSource(
    request: VoxelConversionMeshSourceImportRequest,
  ): VoxelConversionMeshSourceImportReceipt {
    const handle = this.#requireHandle('importVoxelConversionMeshSource');
    const payload = callNative(() =>
      this.#addon.importVoxelConversionMeshSource(handle, JSON.stringify(request)),
    );
    return parseNativeJson<VoxelConversionMeshSourceImportReceipt>(
      payload,
      'voxel conversion mesh source import',
    );
  }

  readVoxelConversionSourceMetadata(
    request: VoxelConversionSourceMetadataRequest,
  ): VoxelConversionSourceMetadataReadout {
    const handle = this.#requireHandle('readVoxelConversionSourceMetadata');
    const payload = callNative(() =>
      this.#addon.readVoxelConversionSourceMetadata(handle, JSON.stringify(request)),
    );
    return parseNativeJson<VoxelConversionSourceMetadataReadout>(
      payload,
      'voxel conversion source metadata',
    );
  }

  previewVoxelConversion(request: VoxelConversionPreviewRequest): VoxelConversionPreview {
    const handle = this.#requireHandle('previewVoxelConversion');
    const payload = callNative(() => this.#addon.previewVoxelConversion(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelConversionPreview>(payload, 'voxel conversion preview');
  }

  applyVoxelConversion(request: VoxelConversionApplyRequest): VoxelConversionReceipt {
    const handle = this.#requireHandle('applyVoxelConversion');
    const payload = callNative(() => this.#addon.applyVoxelConversion(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelConversionReceipt>(payload, 'voxel conversion receipt');
  }

  exportVoxelConversionEvidence(
    evidence: readonly VoxelConversionEvidenceRef[],
  ): readonly VoxelConversionEvidenceRef[] {
    const handle = this.#requireHandle('exportVoxelConversionEvidence');
    const payload = callNative(() =>
      this.#addon.exportVoxelConversionEvidence(handle, JSON.stringify(evidence)),
    );
    return parseNativeJson<readonly VoxelConversionEvidenceRef[]>(payload, 'voxel conversion evidence');
  }

  readVoxelModelInfo(request: VoxelModelInfoRequest): VoxelModelInfoReadout {
    const handle = this.#requireHandle('readVoxelModelInfo');
    const payload = callNative(() => this.#addon.readVoxelModelInfo(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelModelInfoReadout>(payload, 'voxel model info');
  }

  readVoxelModelWindow(request: VoxelModelWindowRequest): VoxelModelWindowReadout {
    const handle = this.#requireHandle('readVoxelModelWindow');
    const payload = callNative(() => this.#addon.readVoxelModelWindow(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelModelWindowReadout>(payload, 'voxel model window');
  }

  exportVoxelVolumeAsset(request: VoxelVolumeAssetExportRequest): VoxelVolumeAssetExportReceipt {
    const handle = this.#requireHandle('exportVoxelVolumeAsset');
    const payload = callNative(() => this.#addon.exportVoxelVolumeAsset(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelVolumeAssetExportReceipt>(payload, 'voxel volume asset export receipt');
  }

  saveVoxelVolumeAsset(request: VoxelVolumeAssetSaveRequest): VoxelVolumeAssetSaveReceipt {
    const handle = this.#requireHandle('saveVoxelVolumeAsset');
    const payload = callNative(() => this.#addon.saveVoxelVolumeAsset(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelVolumeAssetSaveReceipt>(payload, 'voxel volume asset save receipt');
  }

  updateVoxelVolumeAssetPalette(
    request: VoxelVolumeAssetPaletteUpdateRequest,
  ): VoxelVolumeAssetPaletteUpdateReceipt {
    const handle = this.#requireHandle('updateVoxelVolumeAssetPalette');
    const payload = callNative(() => this.#addon.updateVoxelVolumeAssetPalette(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelVolumeAssetPaletteUpdateReceipt>(payload, 'voxel volume asset palette update receipt');
  }

  initializeVoxelVolumeAuthoring(
    request: VoxelVolumeAuthoringInitializeRequest,
  ): VoxelVolumeAuthoringInitializeReceipt {
    const handle = this.#requireHandle('initializeVoxelVolumeAuthoring');
    const payload = callNative(() =>
      this.#addon.initializeVoxelVolumeAuthoring(handle, JSON.stringify(request)),
    );
    return parseNativeJson<VoxelVolumeAuthoringInitializeReceipt>(
      payload,
      'voxel volume authoring initialize receipt',
    );
  }

  loadVoxelVolumeAsset(request: VoxelVolumeAssetLoadRequest): VoxelVolumeAssetLoadReceipt {
    const handle = this.#requireHandle('loadVoxelVolumeAsset');
    const payload = callNative(() => this.#addon.loadVoxelVolumeAsset(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelVolumeAssetLoadReceipt>(payload, 'voxel volume asset load receipt');
  }

  unloadVoxelVolumeAsset(request: VoxelVolumeAssetUnloadRequest): VoxelVolumeAssetUnloadReceipt {
    const handle = this.#requireHandle('unloadVoxelVolumeAsset');
    const payload = callNative(() => this.#addon.unloadVoxelVolumeAsset(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelVolumeAssetUnloadReceipt>(payload, 'voxel volume asset unload receipt');
  }

  validateVoxelAnnotationLayer(
    request: VoxelAnnotationLayerValidationRequest,
  ): VoxelAnnotationLayerValidationReport {
    const handle = this.#requireHandle('validateVoxelAnnotationLayer');
    const payload = callNative(() => this.#addon.validateVoxelAnnotationLayer(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelAnnotationLayerValidationReport>(payload, 'voxel annotation validation report');
  }

  loadVoxelAnnotationLayer(request: VoxelAnnotationLayerLoadRequest): VoxelAnnotationLayerLoadReceipt {
    const handle = this.#requireHandle('loadVoxelAnnotationLayer');
    const payload = callNative(() => this.#addon.loadVoxelAnnotationLayer(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelAnnotationLayerLoadReceipt>(payload, 'voxel annotation load receipt');
  }

  readVoxelAnnotationQuery(request: VoxelAnnotationQueryRequest): VoxelAnnotationQueryReadout {
    const handle = this.#requireHandle('readVoxelAnnotationQuery');
    const payload = callNative(() => this.#addon.readVoxelAnnotationQuery(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelAnnotationQueryReadout>(payload, 'voxel annotation query readout');
  }

  applyVoxelAnnotationEdit(request: VoxelAnnotationEditRequest): VoxelAnnotationEditReceipt {
    const handle = this.#requireHandle('applyVoxelAnnotationEdit');
    const payload = callNative(() => this.#addon.applyVoxelAnnotationEdit(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelAnnotationEditReceipt>(payload, 'voxel annotation edit receipt');
  }

  exportVoxelAnnotationLayer(request: VoxelAnnotationLayerExportRequest): VoxelAnnotationLayerExportReceipt {
    const handle = this.#requireHandle('exportVoxelAnnotationLayer');
    const payload = callNative(() => this.#addon.exportVoxelAnnotationLayer(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelAnnotationLayerExportReceipt>(payload, 'voxel annotation export receipt');
  }

  // ── Unwired operations: fail-closed, never mock-backed ─────────────────────
  // Replace each body with its real native call (and add the manifest name to
  // NATIVE_WIRED_OPERATIONS) when the codegen emitter wires the `#[napi]` export.
  pickVoxel(ray: PickRay): PickResult {
    const handle = this.#requireHandle('pickVoxel');
    const payload = callNative(() => this.#addon.pickVoxel(handle, JSON.stringify(ray)));
    return parseNativeJson<PickResult>(payload, 'voxel pick result');
  }

  configureVoxelProjectionInstances(
    request: VoxelProjectionBindingRequest,
  ): VoxelProjectionBindingReceipt {
    const handle = this.#requireHandle('configureVoxelProjectionInstances');
    const payload = callNative(() =>
      this.#addon.configureVoxelProjectionInstances(handle, JSON.stringify(request)),
    );
    return parseNativeJson<VoxelProjectionBindingReceipt>(payload, 'voxel projection binding receipt');
  }

  pickVoxelInstance(request: VoxelInstancePickRequest): VoxelInstancePickResult {
    const handle = this.#requireHandle('pickVoxelInstance');
    const payload = callNative(() => this.#addon.pickVoxelInstance(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelInstancePickResult>(payload, 'voxel instance pick result');
  }

  applyCollisionConstrainedCameraInput(envelope: CollisionConstrainedCameraInputEnvelope): CameraCollisionSnapshot {
    const handle = this.#requireHandle('applyCollisionConstrainedCameraInput');
    return callNative(() => this.#addon.applyCollisionConstrainedCameraInput(handle, envelope));
  }

  applyGeneratedTunnelToRuntimeWorld(
    request: GeneratedTunnelRuntimeApplyRequest,
  ): GeneratedTunnelRuntimeApplyReceipt {
    const handle = this.#requireHandle('applyGeneratedTunnelToRuntimeWorld');
    if (request.preset !== 'tiny-enclosed') {
      throw new RuntimeBridgeError('invalid_input', 'only the tiny-enclosed generated tunnel preset is supported');
    }
    const seed = nonNegativeSafeInteger(request.seed, 'seed');
    const receipt = callNative(() =>
      this.#addon.applyGeneratedTunnelToRuntimeWorld(handle, request.preset, seed),
    );
    return {
      preset: generatedTunnelPreset(receipt.presetId),
      seed: nonNegativeSafeInteger(receipt.seed, 'receipt.seed'),
      grid: nonNegativeSafeInteger(receipt.grid, 'receipt.grid'),
      configHash: hexHashString(receipt.configHash, 'generatedTunnel.configHash'),
      outputHash: hexHashString(receipt.outputHash, 'generatedTunnel.outputHash'),
      collisionSourceHash: hexHashString(receipt.collisionSourceHash, 'generatedTunnel.collisionSourceHash'),
      collisionProjectionHash: hashString(
        receipt.collisionProjectionHash,
        'generatedTunnel.collisionProjectionHash',
      ),
      runtimeFrame: {
        worldOffset: bridgeVec3Array(receipt.runtimeFrame.worldOffset, 'generatedTunnel.runtimeFrame.worldOffset'),
        playableMin: bridgeVec3Array(receipt.runtimeFrame.playableMin, 'generatedTunnel.runtimeFrame.playableMin'),
        playableMax: bridgeVec3Array(receipt.runtimeFrame.playableMax, 'generatedTunnel.runtimeFrame.playableMax'),
      },
    };
  }

  selectVoxel(request: ScreenPointToPickRayRequest): VoxelSelectionSnapshot {
    const handle = this.#requireHandle('selectVoxel');
    const payload = callNative(() => this.#addon.selectVoxel(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelSelectionSnapshot>(payload, 'voxel selection snapshot');
  }

  readVoxelMeshEvidence(request: VoxelMeshEvidenceRequest): VoxelMeshEvidenceSnapshot {
    const handle = this.#requireHandle('readVoxelMeshEvidence');
    const payload = callNative(() => this.#addon.readVoxelMeshEvidence(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelMeshEvidenceSnapshot>(payload, 'voxel mesh evidence snapshot');
  }

  readVoxelEditHistory(request: VoxelEditHistoryReadRequest): VoxelEditHistorySummary {
    const handle = this.#requireHandle('readVoxelEditHistory');
    const payload = callNative(() => this.#addon.readVoxelEditHistory(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelEditHistorySummary>(payload, 'voxel edit history summary');
  }

  previewVoxelEditRevert(request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt {
    const handle = this.#requireHandle('previewVoxelEditRevert');
    const payload = callNative(() => this.#addon.previewVoxelEditRevert(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelEditHistoryRevertReceipt>(payload, 'voxel edit history revert preview');
  }

  applyVoxelEditRevert(request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt {
    const handle = this.#requireHandle('applyVoxelEditRevert');
    const payload = callNative(() => this.#addon.applyVoxelEditRevert(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelEditHistoryRevertReceipt>(payload, 'voxel edit history revert apply');
  }

  undoVoxelEdit(request: VoxelEditHistoryUndoRequest): VoxelEditHistoryUndoReceipt {
    const handle = this.#requireHandle('undoVoxelEdit');
    const payload = callNative(() => this.#addon.undoVoxelEdit(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelEditHistoryUndoReceipt>(payload, 'voxel edit history undo receipt');
  }

  redoVoxelEdit(request: VoxelEditHistoryRedoRequest): VoxelEditHistoryRedoReceipt {
    const handle = this.#requireHandle('redoVoxelEdit');
    const payload = callNative(() => this.#addon.redoVoxelEdit(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelEditHistoryRedoReceipt>(payload, 'voxel edit history redo receipt');
  }

  createCamera(request: CameraCreateRequest): CameraSnapshot {
    const handle = this.#requireHandle('createCamera');
    return callNative(() => this.#addon.createCamera(handle, request));
  }

  applyCameraModeCommand(command: CameraModeCommand): CameraModeChangeReceipt {
    const handle = this.#requireHandle('applyCameraModeCommand');
    const payload = callNative(() =>
      this.#addon.applyCameraModeCommand(handle, JSON.stringify(command)),
    );
    return parseNativeJson<CameraModeChangeReceipt>(payload, 'camera mode change receipt');
  }

  applyCameraNavigationInput(input: CameraNavigationInputEnvelope): CameraNavigationReceipt {
    const handle = this.#requireHandle('applyCameraNavigationInput');
    const payload = callNative(() =>
      this.#addon.applyCameraNavigationInput(handle, JSON.stringify(input)),
    );
    return parseNativeJson<CameraNavigationReceipt>(payload, 'camera navigation receipt');
  }

  readCameraControllerState(request: CameraControllerReadRequest): CameraControllerState {
    const handle = this.#requireHandle('readCameraControllerState');
    const payload = callNative(() =>
      this.#addon.readCameraControllerState(handle, JSON.stringify(request)),
    );
    return parseNativeJson<CameraControllerState>(payload, 'camera controller state');
  }

  applyFirstPersonCameraInput(input: FirstPersonCameraInputEnvelope): CameraSnapshot {
    const handle = this.#requireHandle('applyFirstPersonCameraInput');
    return callNative(() => this.#addon.applyFirstPersonCameraInput(handle, input));
  }

  readCameraProjection(request: CameraProjectionRequest): CameraProjectionSnapshot {
    const handle = this.#requireHandle('readCameraProjection');
    const payload = callNative(() => this.#addon.readCameraProjection(handle, JSON.stringify(request)));
    return parseNativeJson<CameraProjectionSnapshot>(payload, 'camera projection snapshot');
  }

  getBuffer(bufferHandle: RuntimeBufferHandle): RuntimeBufferView {
    const handle = this.#requireHandle('getBuffer');
    const validatedBufferHandle = nonNegativeSafeInteger(bufferHandle, 'buffer handle');
    const view = callNative(() => this.#addon.getBuffer(handle, validatedBufferHandle));
    return {
      handle: nonNegativeSafeInteger(view.handle, 'returned buffer handle') as RuntimeBufferHandle,
      bytes: Uint8Array.from(view.bytes),
    };
  }

  releaseBuffer(bufferHandle: RuntimeBufferHandle): void {
    const handle = this.#requireHandle('releaseBuffer');
    const validatedBufferHandle = nonNegativeSafeInteger(bufferHandle, 'buffer handle');
    callNative(() => this.#addon.releaseBuffer(handle, validatedBufferHandle));
  }

  unloadProjectBundle(): void {
    const handle = this.#requireHandle('unloadProjectBundle');
    callNative(() => this.#addon.unloadProjectBundle(handle));
  }

  loadReplayFixture(): ReplaySessionHandle {
    throw nativeUnimplemented('load_replay_fixture');
  }

  runReplayStep(): ReplayStepReport {
    throw nativeUnimplemented('run_replay_step');
  }
}

/**
 * Construct the native (napi-rs) bridge. Throws a classified
 * {@link RuntimeBridgeError} of kind `native_unavailable` if the addon is not built
 * — callers can fall back to the mock for tests/dev.
 */
export function createNativeRuntimeBridge(modulePath?: string): RuntimeBridge {
  try {
    const addon = modulePath ? loadNativeAddon(modulePath) : loadNativeAddon();
    return new NativeRuntimeBridge(addon);
  } catch (cause) {
    if (cause instanceof NativeAddonUnavailable) {
      throw new RuntimeBridgeError('native_unavailable', cause.message);
    }
    throw cause;
  }
}

/** Operation count for quick sanity in consumers/tests. */
export const STABLE_OPERATION_COUNT = MANIFEST_OPERATIONS.filter(
  (o) => o.surface === 'stable',
).length;
