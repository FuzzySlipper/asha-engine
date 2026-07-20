// Native facade parity / fail-closed conformance (task #2423).
//
// Proves the seam closed in this task: a *loaded* native facade either executes a
// real native implementation or throws a classified `operation_unimplemented`
// error for every manifest operation. It must NEVER silently inherit mock /
// reference behaviour for an unwired op (the prior `extends MockRuntimeBridge`
// hazard). We inject a fake addon so the test runs without a built `.node` binary.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type {
  CameraCreateRequest,
  CollisionConstrainedCameraInputEnvelope,
  VoxelConversionApplyRequest,
  VoxelConversionPlanRequest,
  VoxelConversionPreviewRequest,
  VoxelConversionSourceMetadataRequest,
  VoxelConversionSourceRegistrationRequest,
  VoxelModelInfoRequest,
  VoxelModelWindowRequest,
  VoxelAnnotationEditRequest,
  VoxelAnnotationLayer,
  VoxelAnnotationLayerExportRequest,
  VoxelAnnotationLayerLoadRequest,
  VoxelAnnotationLayerValidationRequest,
  VoxelAnnotationQueryRequest,
  VoxelEditHistoryReadRequest,
  VoxelEditHistoryRedoRequest,
  VoxelEditHistoryRevertRequest,
  VoxelEditHistoryUndoRequest,
  VoxelVolumeAssetExportRequest,
  VoxelVolumeAssetLoadRequest,
  VoxelVolumeAssetSaveRequest,
  VoxelVolumeAssetUnloadRequest,
  VoxelVolumeAuthoringInitializeRequest,
} from '@asha/contracts';
import { entityId } from '@asha/contracts';
import type { NativeAddon } from '@asha/native-bridge';
import {
  MANIFEST_OPERATIONS,
  NATIVE_WIRED_OPERATIONS,
  NativeRuntimeBridge,
  RuntimeBridgeError,
  frameCursor,
  type RuntimeBridge,
} from './index.js';
import { REQUIRED_NATIVE_CONFORMANCE_OPS } from './native-conformance-operations.test-fixture.js';
import {
  VOXEL_CONVERSION_MESH_ASSET_REGISTRATION_REQUEST,
  VOXEL_CONVERSION_MESH_SOURCE_IMPORT_REQUEST,
  createNativeVoxelMeshSourceHandlers,
} from './native-voxel-mesh-source.test-fixture.js';
import { createVoxelPaletteUpdateHandler, voxelPaletteUpdateRequest } from './native-voxel-palette.test-fixture.js';
import {
  CAMERA_CREATE_REQUEST,
  CAMERA_INPUT,
  COLLISION_CAMERA_INPUT,
  INPUT_CONTEXT_COMMAND,
  INPUT_SESSION_CONFIGURE_REQUEST,
  MODEL_MATERIAL_PREVIEW_REQUEST,
  RAW_INPUT_SAMPLE, RECORDED_INPUT_ACTION,
  createNativeInputHandlers,
} from './native-fail-closed-inputs.test-fixture.js';
import { createNativeOperationInvocations } from './native-operation-invocations.test-fixture.js';
import {
  CAMERA_MODE_COMMAND,
  CAMERA_NAVIGATION_INPUT,
  createNativeCameraControllerHandlers,
} from './native-camera-controller.test-fixture.js';
import { createNativeComposedGameplayHandlers } from './native-composed-gameplay.test-fixture.js';

const HASH_A = 'fnv1a64:00000000000000aa';
const HASH_B = 'fnv1a64:00000000000000bb';
const HASH_C = 'fnv1a64:00000000000000cc';
const VOXEL_PLAN_HASH = 'fnv1a64:0000000000000102';
const VOXEL_PREVIEW_HASH = 'fnv1a64:0000000000000103';
const GAME_RULE_CATALOG = {
  catalog: { catalogId: 'catalog.game-rules.native', version: '0.1.0', contentHash: HASH_A },
  valueChannels: [{ channelId: 'value.health', displayName: 'Health' }],
  bundles: [{
    bundleId: 'bundle.poisoned-impact',
    effectOps: [
      { kind: 'applyDelta', opId: 'op.impact-damage', channelId: 'value.health', amount: -3, tags: ['tag.impact'] },
      {
        kind: 'schedulePeriodicEffect',
        opId: 'op.schedule-poison',
        modifierId: 'modifier.poison',
        cadence: { periodTicks: 2 },
        duration: { kind: 'ticks', ticks: 6 },
        tags: ['tag.poison'],
      },
    ],
    modifiers: [{
      modifierId: 'modifier.poison',
      stackPolicy: { kind: 'refresh' },
      duration: { kind: 'ticks', ticks: 6 },
      tickCadence: { periodTicks: 2 },
      tags: ['tag.poison'],
      effectOpIds: ['op.poison-tick'],
      sourceHash: HASH_B,
    }],
    tags: ['tag.poison'],
    sourceHash: HASH_C,
  }],
} as const;

const GAME_RULE_REQUEST = {
  catalog: GAME_RULE_CATALOG.catalog,
  bundleId: 'bundle.poisoned-impact',
  source: entityId(101),
  target: entityId(777),
  values: [{ channelId: 'value.health', min: 0, current: 75, max: 75 }],
  tick: 9,
} as const;

const VOXEL_CONVERSION_PLAN_REQUEST = {
  source: {
    assetId: 'mesh/quad',
    assetKind: 'mesh',
    assetVersion: 1,
    sourceHash: 'sha256:quad',
    meshPrimitive: null,
  },
  target: {
    grid: 1,
    volumeAssetId: 'voxel/generated',
    origin: { x: 0, y: 0, z: 0 },
  },
  settings: {
    mode: 'surface',
    fitPolicy: 'contain',
    originPolicy: 'target_min',
    resolution: [4, 4, 1] as const,
    voxelSize: 1,
    maxOutputVoxels: 16,
    transform: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1] as const,
    materialMap: {
      entries: [{ sourceMaterialSlot: 0, sourceMaterialId: 'mat/a', voxelMaterial: 3 }],
      textureAssets: [],
      textureBindings: [],
      defaultVoxelMaterial: 3,
    },
  },
} as const;

const VOXEL_CONVERSION_SOURCE_REGISTRATION_REQUEST = {
  source: {
    assetId: 'mesh/native-registered-triangle',
    assetKind: 'mesh',
    assetVersion: 2,
    sourceHash: 'sha256:native-registered-triangle',
    meshPrimitive: 'default',
  },
  positions: [[0, 0, 0], [1, 0, 0], [0, 1, 0]] as const,
  triangles: [{ indices: [0, 1, 2] as const, sourceMaterialSlot: 0 }],
  materialSlots: [{ sourceMaterialSlot: 0, sourceMaterialId: 'mat/a' }],
} satisfies VoxelConversionSourceRegistrationRequest;

const VOXEL_CONVERSION_EVIDENCE = [
  {
    kind: 'plan',
    uri: 'asha://voxel-conversion/plan/fnv1a64:0000000000000101',
    contentHash: VOXEL_PLAN_HASH,
  },
] as const;

const VOXEL_MODEL_INFO_REQUEST = {
  grid: 1,
  volumeAssetId: 'voxel/generated',
  includeMaterialCounts: true,
} satisfies VoxelModelInfoRequest;

const VOXEL_MODEL_WINDOW_REQUEST = {
  grid: 1,
  volumeAssetId: 'voxel/generated',
  bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
  includeEmpty: false,
  materialFilter: [],
  maxSamples: 1,
} satisfies VoxelModelWindowRequest;

const VOXEL_VOLUME_ASSET_EXPORT_REQUEST = {
  grid: 1,
  volumeAssetId: 'voxel/generated',
  targetAssetId: 'voxel-volume/native-export',
  label: 'Native export',
  createdBy: 'native-fail-closed-test',
  sourceTool: '@asha/runtime-bridge',
  maxSparseRuns: 16,
  expectedSessionHash: 'fnv1a64:0000000000000105',
} satisfies VoxelVolumeAssetExportRequest;

const VOXEL_VOLUME_ASSET_LOAD_REQUEST = {
  asset: {
    assetId: 'voxel-volume/native-export',
    schemaVersion: 1,
    mediaType: 'application/vnd.asha.voxel-volume+json;version=1',
    grid: {
      origin: [0, 0, 0],
      cellSize: 1,
      coordinateSystem: 'y_up_right_handed',
    },
    bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
    representation: {
      kind: 'sparse_runs',
      sparseRuns: [{ start: { x: 0, y: 0, z: 0 }, length: 1, material: 3 }],
    },
    materialPalette: [{
      voxelMaterial: 3,
      paletteEntryId: 'voxel-material/surface-a',
      displayName: 'Surface A',
      materialAssetId: 'material/surface-a',
      materialCatalogBindingId: 'catalog-binding/surface-a',
    }],
    provenance: [{
      kind: 'runtime_export',
      uri: 'asha://runtime-session/voxel-volume-export/voxel-volume/native-export',
      contentHash: 'fnv1a64:0000000000000107',
    }],
    authoring: {
      label: 'Native export',
      createdBy: 'native-fail-closed-test',
      sourceTool: '@asha/runtime-bridge',
    },
    validationDiagnostics: [],
    contentHashes: {
      canonicalJson: 'fnv1a64:0000000000000108',
      voxelData: 'fnv1a64:0000000000000109',
    },
  },
  targetGrid: 1,
  targetVolumeAssetId: 'voxel/generated',
  replaceExisting: true,
  includeMaterialCounts: true,
} satisfies VoxelVolumeAssetLoadRequest;

const VOXEL_VOLUME_ASSET_SAVE_REQUEST = {
  exportRequest: VOXEL_VOLUME_ASSET_EXPORT_REQUEST,
  targetProjectBundle: 'asha-demo',
  targetAssetPath: 'assets/voxels/native-export.avxl.json',
  representationKind: 'sparse_runs',
  expectedExistingCanonicalJsonHash: null,
  expectedCanonicalJsonHash: 'fnv1a64:0000000000000108',
  expectedVoxelDataHash: 'fnv1a64:0000000000000109',
} satisfies VoxelVolumeAssetSaveRequest;

const VOXEL_VOLUME_ASSET_UNLOAD_REQUEST = {
  grid: 1,
  volumeAssetId: 'voxel/generated',
  expectedSessionHash: 'fnv1a64:0000000000000110',
} satisfies VoxelVolumeAssetUnloadRequest;

const VOXEL_VOLUME_AUTHORING_INITIALIZE_REQUEST = {
  grid: 1,
  volumeAssetId: 'voxel/authored',
  seedChunk: { x: 0, y: 0, z: 0 },
  materialPalette: VOXEL_VOLUME_ASSET_LOAD_REQUEST.asset.materialPalette,
  authoring: { label: 'Native authored volume', createdBy: 'native-fail-closed-test', sourceTool: '@asha/runtime-bridge' },
  maxMaterialBindings: 8,
} satisfies VoxelVolumeAuthoringInitializeRequest;

const VOXEL_ANNOTATION_LAYER = {
  layerId: 'voxel-annotation/native-fixture',
  schemaVersion: 1,
  mediaType: 'application/vnd.asha.voxel-annotation+json;version=1',
  targetVoxelVolumeAssetId: 'voxel/generated',
  targetVoxelDataHash: 'fnv1a64:0000000000000109',
  targetBounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
  regions: [{
    regionId: 'region/native-room',
    label: 'Native room',
    kind: 'room',
    tags: ['fixture'],
    parentRegionId: null,
    bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
    selection: { sparseRuns: [{ start: { x: 0, y: 0, z: 0 }, length: 1 }] },
  }],
  provenance: [{
    kind: 'authored',
    uri: 'asha://runtime-bridge/native-fail-closed/annotation',
    contentHash: 'fnv1a64:0000000000000112',
  }],
  contentHashes: {
    canonicalJson: 'fnv1a64:0000000000000113',
    membershipData: 'fnv1a64:0000000000000114',
  },
  validationDiagnostics: [],
} satisfies VoxelAnnotationLayer;

const VOXEL_ANNOTATION_VALIDATION_REQUEST = {
  input: { kind: 'finalized', layer: VOXEL_ANNOTATION_LAYER },
  expectedTargetVoxelVolumeAssetId: 'voxel/generated',
  expectedTargetVoxelDataHash: 'fnv1a64:0000000000000109',
  maxRegions: 16,
  maxSparseRunsPerRegion: 16,
  maxTotalAssignedCells: 16,
} satisfies VoxelAnnotationLayerValidationRequest;
const VOXEL_ANNOTATION_LOAD_REQUEST = {
  layer: VOXEL_ANNOTATION_LAYER,
  targetGrid: 1,
  replaceExisting: true,
  expectedSessionHash: 'fnv1a64:0000000000000110',
} satisfies VoxelAnnotationLayerLoadRequest;

const VOXEL_ANNOTATION_QUERY_REQUEST = {
  runtimeLayerId: 'runtime-annotation/voxel-annotation/native-fixture',
  layerId: VOXEL_ANNOTATION_LAYER.layerId,
  mode: 'cell',
  cell: { x: 0, y: 0, z: 0 },
  bounds: null,
  regionId: null,
  maxRegions: 4,
  expectedLayerHash: VOXEL_ANNOTATION_LAYER.contentHashes.canonicalJson,
} satisfies VoxelAnnotationQueryRequest;

const VOXEL_ANNOTATION_EDIT_REQUEST = {
  runtimeLayerId: 'runtime-annotation/voxel-annotation/native-fixture',
  layerId: VOXEL_ANNOTATION_LAYER.layerId,
  expectedLayerHash: VOXEL_ANNOTATION_LAYER.contentHashes.canonicalJson,
  operation: 'set_label',
  regionId: 'region/native-room',
  region: null,
  sparseRuns: [],
  tags: [],
  label: 'Native room edited',
  kind: null,
  parentRegionId: null,
} satisfies VoxelAnnotationEditRequest;

const VOXEL_ANNOTATION_EXPORT_REQUEST = {
  runtimeLayerId: 'runtime-annotation/voxel-annotation/native-fixture',
  layerId: VOXEL_ANNOTATION_LAYER.layerId,
  expectedLayerHash: 'fnv1a64:0000000000000115',
  includeDiagnostics: true,
} satisfies VoxelAnnotationLayerExportRequest;

const VOXEL_EDIT_HISTORY_READ_REQUEST = {
  historyId: 'history/native-fixture',
  cursorId: null,
  maxEntries: 4,
  includeRedoTail: true,
  expectedHistoryHash: null,
} satisfies VoxelEditHistoryReadRequest;

const VOXEL_EDIT_HISTORY_REVERT_REQUEST = {
  historyId: 'history/native-fixture',
  mode: 'preview_revert',
  target: { transactionId: null, cursorId: 'cursor/0', cursorIndex: 0 },
  expectedHistoryHash: 'fnv1a64:history',
  expectedCursorHash: 'fnv1a64:cursor',
  maxReplaySteps: 16,
  maxDiffVoxels: 32,
  includeSampleWindow: false,
} satisfies VoxelEditHistoryRevertRequest;

const VOXEL_EDIT_HISTORY_UNDO_REQUEST = {
  historyId: 'history/native-fixture',
  expectedHistoryHash: 'fnv1a64:history',
  expectedCursorHash: 'fnv1a64:cursor',
  maxReplaySteps: 16,
  maxDiffVoxels: 32,
} satisfies VoxelEditHistoryUndoRequest;

const VOXEL_EDIT_HISTORY_REDO_REQUEST = {
  historyId: 'history/native-fixture',
  expectedHistoryHash: 'fnv1a64:history',
  expectedCursorHash: 'fnv1a64:cursor',
  maxReplaySteps: 16,
  maxDiffVoxels: 32,
} satisfies VoxelEditHistoryRedoRequest;

function parseJsonFixture<T>(payload: string): T {
  return JSON.parse(payload) as T;
}

function voxelHistoryRevertFixture(request: VoxelEditHistoryRevertRequest, applied: boolean) {
  const cursor = {
    cursorId: 'cursor/native',
    cursorKind: 'applied',
    appliedTransactionId: null,
    parentCursorId: null,
    historyHash: applied ? 'fnv1a64:history-native-after' : 'fnv1a64:history-native',
    voxelStateHash: 'fnv1a64:voxel-native',
    materialCatalogHash: 'fnv1a64:materials-native',
    undoDepth: applied ? 0 : 1,
    redoDepth: applied ? 1 : 0,
    entryCount: 1,
    checkpointCount: 0,
  };
  return {
    request,
    applied,
    preview: request.mode === 'preview_revert',
    historyId: request.historyId,
    cursorBefore: cursor,
    cursorAfter: cursor,
    durableEntry: null,
    previewEvidence: null,
    diffSummary: null,
    replayHash: 'fnv1a64:replay-native',
    historyHashBefore: 'fnv1a64:history-native',
    historyHashAfter: cursor.historyHash,
    diagnostics: [],
  };
}

// A fake addon with sentinel return values distinct from MockRuntimeBridge, so a
// silent mock fallback would be observable in the wired-op assertions below.
function workspaceAuthoringStateFixture(
  input: Parameters<RuntimeBridge['openWorkspaceAuthoring']>[0],
  status: 'open' | 'closed' = 'open',
) {
  return {
    kind: 'workspace_authoring.state.v0' as const,
    status,
    identity: {
      kind: 'workspace_authoring.identity.v0' as const,
      authoringId: input.authoringId,
      mode: 'rust' as const,
      generation: 1,
      seed: input.seed,
      project: input.project,
      projectBundle: input.projectBundle,
      nonClaims: [
        'not_gameplay_runtime_session',
        'not_simulation_loop',
        'not_stored_truth',
        'not_renderer_authority',
      ] as const,
    },
    composition: {
      loadedProjectBundle: input.projectBundle.sceneId,
      fatalCount: 0,
      totalCount: 0,
      blocksLoad: false,
    },
    workingRevision: 0,
    storedRevision: 0,
    dirty: false,
    lastStoredCanonicalJsonHash: null,
    authoritySnapshotHash: HASH_A,
    lifecycleHash: HASH_B,
  };
}

function fakeAddon(calls: string[] = []): NativeAddon {
  return {
    initializeEngine: (seed: number) => {
      calls.push(`initialize:${seed}`);
      return seed + 100;
    },
    openWorkspaceAuthoring: (existingHandle: number, requestJson: string) => {
      calls.push(`workspaceAuthoringOpen:${requestJson}`);
      const request = parseJsonFixture<Parameters<RuntimeBridge['openWorkspaceAuthoring']>[0]>(requestJson);
      void workspaceAuthoringStateFixture(request);
      return existingHandle >= 0 ? existingHandle : 107;
    },
    readWorkspaceAuthoringState: (handle: number) => {
      void handle;
      return JSON.stringify(workspaceAuthoringStateFixture({
        authoringId: 'workspace-authoring.native-fixture',
        seed: 7,
        project: { gameId: 'native-fixture', workspaceId: 'workspace/native-fixture' },
        projectBundle: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 1 },
      }));
    },
    readWorkspaceAuthoringProjection: (_handle: number, requestJson: string) => {
      const request = parseJsonFixture<Parameters<RuntimeBridge['readWorkspaceAuthoringProjection']>[0]>(requestJson);
      return JSON.stringify({
        kind: 'workspace_authoring.projection.v0',
        workspaceId: request.expectedWorkspaceId,
        generation: request.expectedGeneration,
        workingRevision: request.expectedWorkingRevision,
        cursor: request.cursor,
        nextCursor: request.cursor + 1,
        delivery: request.cursor === 0 ? 'replace' : 'apply',
        frameJson: '{"ops":[]}',
        renderDiffCount: 0,
        projectionHash: HASH_C,
      });
    },
    confirmWorkspaceAuthoringStored: (_handle: number, requestJson: string) => {
      const request = parseJsonFixture<Parameters<RuntimeBridge['confirmWorkspaceAuthoringStored']>[0]>(requestJson);
      return JSON.stringify({
        kind: 'workspace_authoring.stored_confirmation.v0',
        accepted: true,
        workspaceId: request.expectedWorkspaceId,
        generation: request.expectedGeneration,
        hostPath: request.hostPath,
        canonicalJsonHash: request.canonicalJsonHash,
        storedRevision: 0,
        lifecycleHash: HASH_B,
      });
    },
    closeWorkspaceAuthoring: (_handle: number, requestJson: string) => {
      const request = parseJsonFixture<Parameters<RuntimeBridge['closeWorkspaceAuthoring']>[0]>(requestJson);
      return JSON.stringify({
        kind: 'workspace_authoring.close_receipt.v0',
        closed: true,
        workspaceId: request.expectedWorkspaceId,
        generation: request.expectedGeneration,
        discardedUnsavedWorkingState: request.discardUnsavedWorkingState ?? false,
        lifecycleHash: HASH_B,
      });
    },
    submitCommands: (_handle: number, commandsJson: string) => {
      calls.push(`submit:${commandsJson}`);
      const commands: unknown = JSON.parse(commandsJson);
      return { accepted: Array.isArray(commands) ? commands.length : 0, rejected: 0, rejections: [] };
    },
    stepSimulation: (_handle: number, tick: number) => {
      calls.push(`step:${tick}`);
      return { tick, diffCount: 9 };
    },
    ...createNativeInputHandlers(HASH_A, HASH_B, HASH_C),
    ...createNativeCameraControllerHandlers(calls, HASH_A, HASH_B, HASH_C),
    createCamera: (_handle: number, request: CameraCreateRequest) => {
      calls.push(`createCamera:${request.initialPose.position.join(',')}`);
      return {
        camera: 1,
        tick: 0,
        pose: request.initialPose,
        basis: {
          forward: [0, 0, -1],
          right: [1, 0, 0],
          up: [0, 1, 0],
        },
        projection: request.projection,
        viewport: request.viewport,
      };
    },
    applyCollisionConstrainedCameraInput: (_handle: number, envelope: CollisionConstrainedCameraInputEnvelope) => {
      calls.push(`cameraCollision:${envelope.camera}:${envelope.grid}:${envelope.tick}`);
      const before = {
        camera: envelope.camera,
        tick: envelope.tick - 1,
        pose: CAMERA_CREATE_REQUEST.initialPose,
        basis: {
          forward: [0, 0, -1] as const,
          right: [1, 0, 0] as const,
          up: [0, 1, 0] as const,
        },
        projection: CAMERA_CREATE_REQUEST.projection,
        viewport: CAMERA_CREATE_REQUEST.viewport,
      };
      const attempted = {
        ...before,
        tick: envelope.tick,
        pose: { ...CAMERA_CREATE_REQUEST.initialPose, position: [0, 1.6, -0.05] as const },
      };
      const after = {
        ...before,
        tick: envelope.tick,
        pose: { ...CAMERA_CREATE_REQUEST.initialPose, position: [0, 1.6, -0.04] as const },
      };
      return {
        camera: envelope.camera,
        tick: envelope.tick,
        before,
        attempted,
        after,
        collision: {
          grid: envelope.grid,
          movementMode: envelope.movementMode,
          shape: envelope.shape,
          policy: envelope.policy,
          collided: true,
          blockedAxes: ['z'] as const,
          correction: [0, 0, 0.01] as const,
          queriedAabb: { min: [-0.2, 1.4, -0.25] as const, max: [0.2, 1.8, 0.15] as const },
          collisionSourceHash: 'fnv1a64:sentinel-collision-source',
          collisionProjectionHash: 'fnv1a64:sentinel-collision-projection',
        },
        movementHash: 'fnv1a64:sentinel-movement',
      };
    },
    applyEnemyDirectNavMovement: (
      _handle: number,
      entity: number,
      seedPosition: { readonly x: number; readonly y: number; readonly z: number },
      target: { readonly x: number; readonly y: number; readonly z: number },
      maxStepUnits: number,
    ) => {
      calls.push(`enemyMove:${entity}:${seedPosition.x},${seedPosition.y},${seedPosition.z}:${target.x},${target.y},${target.z}:${maxStepUnits}`);
      return {
        entity,
        authoritySource: 'rust_entity_store',
        from: seedPosition,
        target,
        nextWaypoint: { x: 2, y: 1, z: 7 },
        distanceUnits: 4.01,
        reached: false,
        pathHash: 'fnv1a64:sentinel-path',
        transformHash: 'fnv1a64:sentinel-transform',
        projectionChanged: true,
      };
    },
    readFpsRuntimeSession: (handle: number) => {
      void handle;
      calls.push('fpsRead');
      return {
        backend: 'engine_bridge_rust',
        authoritySurface: 'runtime_session.fps.authority.v0',
        projectBundle: 'custom-demo',
        sessionEpoch: 1,
        lifecycleStatus: { state: 'active' },
        playerEntity: 101,
        enemyEntity: 777,
        health: [{ entity: 777, current: 75, max: 75 }],
        policyBindings: [],
        replayRecords: [{ replayUnit: 'runtime_session.fps.bootstrap.v0', entityHash: HASH_A, healthHash: HASH_B, recordHash: HASH_C }],
        readSets: [],
        entityHash: HASH_A,
        healthHash: HASH_B,
        replayHash: HASH_C,
      };
    },
    applyFpsPrimaryFire: (
      _handle: number,
      tick: number,
      origin: { readonly x: number; readonly y: number; readonly z: number },
      direction: { readonly x: number; readonly y: number; readonly z: number },
    ) => {
      calls.push(`fpsFire:${tick}:${origin.x},${origin.y},${origin.z}:${direction.x},${direction.y},${direction.z}`);
      return {
        backend: 'engine_bridge_rust',
        authoritySurface: 'runtime_session.fps.primary_fire.v0',
        mutationOwner: 'rule-lifecycle + svc-combat',
        workspaceTrace: ['accepted'],
        shooter: 101,
        target: 777,
        targetHealthBefore: { current: 75, max: 75 },
        targetHealthAfter: { current: 0, max: 75 },
        lifecycleStatus: { state: 'enemy_defeated', entity: 777, tick },
        targetRenderVisible: false,
        entityHash: HASH_A,
        healthHash: HASH_B,
        replayHash: HASH_C,
      };
    },
    ...createNativeComposedGameplayHandlers(calls, HASH_A, HASH_B, HASH_C),
    invokeGameExtensionWeaponEffect: (
      _handle: number,
      hookJson: string,
      tick: number,
      origin: { readonly x: number; readonly y: number; readonly z: number },
      direction: { readonly x: number; readonly y: number; readonly z: number },
    ) => {
      calls.push(`gameExtension:${tick}:${origin.x},${origin.y},${origin.z}:${direction.x},${direction.y},${direction.z}`);
      const hook = parseJsonFixture<{
        readonly moduleRef: { readonly moduleId: string; readonly version: string; readonly contractHash: string };
        readonly hookId: string;
        readonly requestId: string;
        readonly inputHash: string;
        readonly target: number | null;
      }>(hookJson);
      return {
        hookReceiptJson: JSON.stringify({
          moduleRef: hook.moduleRef,
          hookId: hook.hookId,
          requestId: hook.requestId,
          status: 'proposed',
          inputHash: hook.inputHash,
          proposal: hook.target === null
            ? null
            : {
                kind: 'damageModifier',
                proposalId: `${hook.requestId}.native`,
                target: hook.target,
                channelId: 'combat.primary_fire.damage',
                amountDelta: 5,
                tags: ['native-fixture'],
                proposalHash: HASH_A,
              },
          diagnostics: [],
          trace: [],
          proposalHash: HASH_A,
        }),
        replayEvidenceJson: JSON.stringify({
          moduleRef: hook.moduleRef,
          hookId: hook.hookId,
          requestId: hook.requestId,
          inputHash: hook.inputHash,
          proposalHash: HASH_A,
          validationStatus: 'accepted',
          eventHashes: [HASH_C],
          rejectionHashes: [],
          replayHash: HASH_B,
        }),
        primaryFire: {
          backend: 'engine_bridge_rust',
          authoritySurface: 'runtime_session.fps.primary_fire.v0',
          mutationOwner: 'rule-lifecycle + svc-combat',
          workspaceTrace: ['accepted extension'],
          shooter: 101,
          target: 777,
          targetHealthBefore: { current: 75, max: 75 },
          targetHealthAfter: { current: 0, max: 75 },
          lifecycleStatus: { state: 'enemy_defeated', entity: 777, tick },
          targetRenderVisible: false,
          entityHash: HASH_A,
          healthHash: HASH_B,
          replayHash: HASH_C,
        },
      };
    },
    validateGameRuleCatalog: (_handle: number, catalogJson: string) => {
      const catalog = parseJsonFixture<{ readonly catalog: { readonly catalogId: string } }>(catalogJson);
      calls.push(`gameRuleValidate:${catalog.catalog.catalogId}`);
      return JSON.stringify({
        accepted: true,
        catalogHash: HASH_A,
        diagnostics: [],
        trace: [{ step: 1, code: 'catalog.accepted', message: 'sentinel catalog accepted', refs: [] }],
        evidence: [{ kind: 'catalogValidation', uri: 'asha://game-rules/catalog-validation/native', contentHash: HASH_B }],
      });
    },
    submitGameRuleEffectIntent: (_handle: number, catalogJson: string, requestJson: string) => {
      const catalog = parseJsonFixture<{ readonly catalog: { readonly catalogId: string } }>(catalogJson);
      const request = parseJsonFixture<{ readonly bundleId: string; readonly source: number; readonly target: number; readonly tick: number }>(requestJson);
      calls.push(`gameRuleSubmit:${catalog.catalog.catalogId}:${request.bundleId}`);
      return JSON.stringify({
        accepted: true,
        requestHash: HASH_A,
        pendingValueDeltas: [{ channelId: 'value.health', amount: -3 }],
        appliedModifiers: [{
          modifierId: 'modifier.poison',
          source: request.source,
          target: request.target,
          stacks: 1,
          appliedTick: request.tick,
          expiresTick: request.tick + 6,
          nextTick: request.tick + 2,
          sourceHash: HASH_B,
        }],
        diagnostics: [],
        trace: [{ step: 1, code: 'resolution.accepted', message: 'sentinel effect accepted', refs: [] }],
        evidence: [{ kind: 'resolutionReceipt', uri: 'asha://game-rules/receipt/native', contentHash: HASH_C }],
        replayHash: HASH_C,
      });
    },
    readGameRuleRuntimeReadout: (_handle: number) => {
      void _handle;
      calls.push('gameRuleReadout');
      return JSON.stringify({
        backend: 'engine_bridge_rust',
        authoritySurface: 'runtime_session.game_rules.v0',
        activeModifiers: [{
          modifierId: 'modifier.poison',
          source: 101,
          target: 777,
          stacks: 1,
          appliedTick: 9,
          expiresTick: 15,
          nextTick: 11,
          sourceHash: HASH_B,
        }],
        recentTrace: [{ step: 1, code: 'resolution.accepted', message: 'sentinel effect accepted', refs: [] }],
        recentReplayHashes: [HASH_C],
        latestReplayHash: HASH_C,
      });
    },
    restartFpsRuntimeSession: (_handle: number, expectedEpoch: number) => {
      calls.push(`fpsRestart:${expectedEpoch}`);
      return {
        backend: 'engine_bridge_rust',
        authoritySurface: 'runtime_session.fps.authority.v0',
        projectBundle: 'custom-demo',
        sessionEpoch: expectedEpoch + 1,
        lifecycleStatus: { state: 'active' },
        playerEntity: 101,
        enemyEntity: 777,
        health: [{ entity: 777, current: 75, max: 75 }],
        policyBindings: [],
        replayRecords: [{ replayUnit: 'runtime_session.fps.bootstrap.v0', entityHash: HASH_A, healthHash: HASH_B, recordHash: HASH_C }],
        readSets: [],
        entityHash: HASH_A,
        healthHash: HASH_B,
        replayHash: HASH_C,
      };
    },
    readFpsEncounterDirector: (_handle: number, lifecycle: unknown) => {
      calls.push('fpsEncounterRead');
      return {
        backend: 'engine_bridge_rust',
        authoritySurface: 'runtime_session.fps.encounter_director.v0',
        mutationOwner: 'rule-lifecycle',
        workspaceTrace: ['sentinel encounter read'],
        state: {
          presetId: 'generated-tunnel-small-encounter',
          status: 'pending',
          spawnedEnemyIds: [],
          defeatedEnemyIds: [],
          revision: 0,
          lastTransition: 'initialized',
        },
        lifecycle,
        readSets: [{ viewKind: 'runtime_session.encounter_director.v0', owner: 'rule-lifecycle', readSet: ['FpsRuntimeSessionState.encounter'] }],
        encounterHash: 'fnv1a64:00000000000000dd',
        replayHash: 'fnv1a64:00000000000000ee',
      };
    },
    applyFpsEncounterTransition: (_handle: number, request: { readonly lifecycle: unknown }) => {
      calls.push('fpsEncounterTransition');
      return {
        backend: 'engine_bridge_rust',
        authoritySurface: 'runtime_session.fps.encounter_transition.v0',
        mutationOwner: 'rule-lifecycle',
        workspaceTrace: ['sentinel encounter transition'],
        accepted: true,
        rejectionReason: null,
        eventKind: 'runtime_encounter.activated.v0',
        state: {
          presetId: 'generated-tunnel-small-encounter',
          status: 'active',
          spawnedEnemyIds: ['encounter.generated_tunnel_small.wave_1.enemy_001'],
          defeatedEnemyIds: [],
          revision: 1,
          lastTransition: 'activated',
        },
        lifecycle: request.lifecycle,
        encounterHash: 'fnv1a64:00000000000000ef',
        replayHash: 'fnv1a64:00000000000000f0',
      };
    },
    readRenderDiffs: (_handle: number, cursor: number) => {
      calls.push(`render:${cursor}`);
      return JSON.stringify({ ops: [] });
    },
    readProjectionFrame: (_handle: number, cursor: number) => {
      calls.push(`projection:${cursor}`);
      return { schemaVersion: 1, authorityTick: cursor, scene: { ops: [] }, presentation: { replayScope: 'excludedFromReplayTruth', ops: [] } };
    },
    readDeveloperConsole: () => ({
      schemaVersion: 1,
      records: [],
      droppedRecordCount: 0,
      firstSequence: null,
      nextSequence: 0,
      snapshotHash: 'fnv1a64:console-empty',
    }),
    planVoxelConversion: (_handle: number, requestJson: string) => {
      calls.push(`voxelPlan:${requestJson}`);
      const request = parseJsonFixture<VoxelConversionPlanRequest>(requestJson);
      return JSON.stringify({
        planId: 'fnv1a64:0000000000000101',
        source: {
          assetId: 'mesh/quad',
          assetKind: 'mesh',
          assetVersion: 1,
          sourceHash: 'sha256:quad',
          meshPrimitive: null,
        },
        target: {
          grid: 1,
          volumeAssetId: 'voxel/generated',
          origin: { x: 0, y: 0, z: 0 },
        },
        settings: request.settings,
        authorityVersion: 'svc-voxel-conversion.v0',
        expectedSourceHash: 'sha256:quad',
        settingsHash: 'fnv1a64:0000000000000102',
        planHash: VOXEL_PLAN_HASH,
        estimatedOutputVoxels: 1,
        estimatedBounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
        diagnostics: [],
        evidence: [{ kind: 'plan', uri: 'asha://voxel-conversion/plan/fnv1a64:0000000000000101', contentHash: 'fnv1a64:0000000000000102' }],
      });
    },
    registerVoxelConversionSource: (_handle: number, requestJson: string) => {
      calls.push(`voxelRegister:${requestJson}`);
      const request = parseJsonFixture<VoxelConversionSourceRegistrationRequest>(requestJson);
      return JSON.stringify({
        source: request.source,
        registered: true,
        materialSlots: request.materialSlots,
        diagnostics: [],
        evidence: [{
          kind: 'source_snapshot',
          uri: `asha://voxel-conversion/source/${request.source.assetId}`,
          contentHash: request.source.sourceHash,
        }],
      });
    },
    ...createNativeVoxelMeshSourceHandlers(calls),
    readVoxelConversionSourceMetadata: (_handle: number, requestJson: string) => {
      calls.push(`voxelSourceMetadata:${requestJson}`);
      const request = parseJsonFixture<VoxelConversionSourceMetadataRequest>(requestJson);
      return JSON.stringify({
        request,
        registered: true,
        source: request.source,
        sourcePath: 'assets/mesh/quad.mesh.json',
        sourceBounds: { min: [0, 0, 0], max: [1, 1, 0] },
        vertexCount: 3,
        triangleCount: 1,
        groups: [{
          groupId: 'group:0:material-slot:0',
          label: 'Group 0 / material slot 0',
          materialSlot: 0,
          start: 0,
          count: 3,
          bounds: { min: [0, 0, 0], max: [1, 1, 0] },
        }],
        materialSlots: [{ sourceMaterialSlot: 0, sourceMaterialId: 'mat/a' }],
        latestPlanId: null,
        latestPlanTransform: null,
        diagnostics: [],
        evidence: [{
          kind: 'source_snapshot',
          uri: `asha://voxel-conversion/source/${request.source.assetId}`,
          contentHash: request.source.sourceHash,
        }],
      });
    },
    previewVoxelConversion: (_handle: number, requestJson: string) => {
      calls.push(`voxelPreview:${requestJson}`);
      const request = parseJsonFixture<VoxelConversionPreviewRequest>(requestJson);
      return JSON.stringify({
        planId: request.planId,
        outputHash: 'fnv1a64:0000000000000103',
        outputVoxelCount: 1,
        outputBounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
        sampleVoxels: [{ coord: { x: 0, y: 0, z: 0 }, material: 3 }],
        diagnostics: [],
        evidence: [{ kind: 'preview', uri: 'asha://voxel-conversion/preview/fnv1a64:0000000000000101', contentHash: 'fnv1a64:0000000000000103' }],
      });
    },
    applyVoxelConversion: (_handle: number, requestJson: string) => {
      calls.push(`voxelApply:${requestJson}`);
      const request = parseJsonFixture<VoxelConversionApplyRequest>(requestJson);
      return JSON.stringify({
        planId: request.planId,
        applied: true,
        outputHash: 'fnv1a64:0000000000000103',
        outputVoxelCount: 1,
        outputBounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
        diagnostics: [],
        evidence: [{ kind: 'apply_receipt', uri: 'asha://voxel-conversion/apply/fnv1a64:0000000000000101', contentHash: 'fnv1a64:0000000000000104' }],
      });
    },
    exportVoxelConversionEvidence: (_handle: number, evidenceJson: string) => {
      calls.push(`voxelEvidence:${evidenceJson}`);
      return evidenceJson;
    },
    readVoxelModelInfo: (_handle: number, requestJson: string) => {
      calls.push(`voxelModelInfo:${requestJson}`);
      const request = parseJsonFixture<VoxelModelInfoRequest>(requestJson);
      return JSON.stringify({
        request,
        resident: true,
        modelId: 'voxel-model:grid:1:volume:voxel/generated',
        volumeAssetId: 'voxel/generated',
        grid: 1,
        bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
        voxelCount: 1,
        materialCounts: [{ material: 3, voxelCount: 1 }],
        source: VOXEL_CONVERSION_PLAN_REQUEST.source,
        latestPlanId: 'fnv1a64:0000000000000101',
        latestOutputHash: VOXEL_PREVIEW_HASH,
        sessionHash: 'fnv1a64:0000000000000105',
        replayHash: 'fnv1a64:0000000000000106',
        evidence: VOXEL_CONVERSION_EVIDENCE,
        diagnostics: [],
      });
    },
    readVoxelModelWindow: (_handle: number, requestJson: string) => {
      calls.push(`voxelModelWindow:${requestJson}`);
      const request = parseJsonFixture<VoxelModelWindowRequest>(requestJson);
      return JSON.stringify({
        request,
        resident: true,
        modelId: 'voxel-model:grid:1:volume:voxel/generated',
        volumeAssetId: 'voxel/generated',
        grid: 1,
        requestedBounds: request.bounds,
        modelBounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
        scannedVoxelCount: 1,
        returnedSampleCount: 1,
        samples: [{ coord: { x: 0, y: 0, z: 0 }, occupied: true, material: 3 }],
        sessionHash: 'fnv1a64:0000000000000107',
        replayHash: 'fnv1a64:0000000000000108',
        diagnostics: [],
      });
    },
    exportVoxelVolumeAsset: (_handle: number, requestJson: string) => {
      calls.push(`voxelVolumeAssetExport:${requestJson}`);
      const request = parseJsonFixture<VoxelVolumeAssetExportRequest>(requestJson);
      const asset = {
        assetId: request.targetAssetId,
        schemaVersion: 1,
        mediaType: 'application/vnd.asha.voxel-volume+json;version=1',
        grid: {
          origin: [0, 0, 0],
          cellSize: 1,
          coordinateSystem: 'y_up_right_handed',
        },
        bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
        representation: {
          kind: 'sparse_runs',
          sparseRuns: [{ start: { x: 0, y: 0, z: 0 }, length: 1, material: 3 }],
        },
        materialPalette: [{
          voxelMaterial: 3,
          paletteEntryId: 'voxel-material/a',
          displayName: 'A',
          materialAssetId: 'mat/a',
          materialCatalogBindingId: 'catalog-binding/a',
        }],
        provenance: [{
          kind: 'runtime_export',
          uri: `asha://runtime-session/voxel-volume-export/${request.targetAssetId}`,
          contentHash: 'fnv1a64:0000000000000107',
        }],
        authoring: {
          label: request.label,
          createdBy: request.createdBy,
          sourceTool: request.sourceTool,
        },
        validationDiagnostics: [],
        contentHashes: {
          canonicalJson: 'fnv1a64:0000000000000108',
          voxelData: 'fnv1a64:0000000000000109',
        },
      };
      return JSON.stringify({
        request,
        exported: true,
        asset,
        canonicalJson: `${JSON.stringify(asset)}\n`,
        canonicalJsonHash: 'fnv1a64:0000000000000108',
        voxelDataHash: 'fnv1a64:0000000000000109',
        diagnostics: [],
      });
    },
    saveVoxelVolumeAsset: (_handle: number, requestJson: string) => {
      calls.push(`voxelVolumeAssetSave:${requestJson}`);
      const request = parseJsonFixture<VoxelVolumeAssetSaveRequest>(requestJson);
      const asset = {
        assetId: request.exportRequest.targetAssetId,
        schemaVersion: 1,
        mediaType: 'application/vnd.asha.voxel-volume+json;version=1',
        grid: {
          origin: [0, 0, 0],
          cellSize: 1,
          coordinateSystem: 'y_up_right_handed',
        },
        bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
        representation: {
          kind: 'sparse_runs',
          sparseRuns: [{ start: { x: 0, y: 0, z: 0 }, length: 1, material: 3 }],
        },
        materialPalette: [{
          voxelMaterial: 3,
          paletteEntryId: 'voxel-material/a',
          displayName: 'A',
          materialAssetId: 'mat/a',
          materialCatalogBindingId: 'catalog-binding/a',
        }],
        provenance: [{
          kind: 'runtime_export',
          uri: `asha://runtime-session/voxel-volume-export/${request.exportRequest.targetAssetId}`,
          contentHash: 'fnv1a64:0000000000000107',
        }],
        authoring: {
          label: request.exportRequest.label,
          createdBy: request.exportRequest.createdBy,
          sourceTool: request.exportRequest.sourceTool,
        },
        validationDiagnostics: [],
        contentHashes: {
          canonicalJson: 'fnv1a64:0000000000000108',
          voxelData: 'fnv1a64:0000000000000109',
        },
      };
      return JSON.stringify({
        request,
        saved: true,
        diff: {
          projectBundle: request.targetProjectBundle,
          assetId: asset.assetId,
          assetPath: request.targetAssetPath,
          operation: 'create',
          previousCanonicalJsonHash: null,
          nextCanonicalJsonHash: asset.contentHashes.canonicalJson,
          nextVoxelDataHash: asset.contentHashes.voxelData,
          representationKind: 'sparse_runs',
          sparseRunCount: 1,
          voxelCount: 1,
          materialCount: 1,
          provenanceCount: 1,
          runtimeSessionHash: request.exportRequest.expectedSessionHash ?? 'fnv1a64:0000000000000105',
        },
        asset,
        canonicalJson: `${JSON.stringify(asset)}\n`,
        canonicalJsonHash: asset.contentHashes.canonicalJson,
        voxelDataHash: asset.contentHashes.voxelData,
        diagnostics: [],
      });
    },
    updateVoxelVolumeAssetPalette: createVoxelPaletteUpdateHandler(calls),
    initializeVoxelVolumeAuthoring: (_handle: number, requestJson: string) => {
      calls.push(`voxelVolumeAuthoringInitialize:${requestJson}`);
      const request = parseJsonFixture<VoxelVolumeAuthoringInitializeRequest>(requestJson);
      return JSON.stringify({
        request,
        initialized: true,
        modelId: `voxel-model:grid:${request.grid}:volume:${request.volumeAssetId}`,
        volumeAssetId: request.volumeAssetId,
        grid: request.grid,
        sessionHash: 'fnv1a64:0000000000000112',
        replayHash: 'fnv1a64:0000000000000113',
        diagnostics: [],
      });
    },
    loadVoxelVolumeAsset: (_handle: number, requestJson: string) => {
      calls.push(`voxelVolumeAssetLoad:${requestJson}`);
      const request = parseJsonFixture<VoxelVolumeAssetLoadRequest>(requestJson);
      return JSON.stringify({
        requestAssetId: request.asset.assetId,
        loaded: true,
        modelId: `voxel-model:grid:${request.targetGrid}:volume:${request.targetVolumeAssetId}`,
        volumeAssetId: request.targetVolumeAssetId,
        grid: request.targetGrid,
        bounds: request.asset.bounds,
        voxelCount: 1,
        materialCounts: [{ material: 3, voxelCount: 1 }],
        provenance: request.asset.provenance,
        canonicalJsonHash: request.asset.contentHashes.canonicalJson,
        voxelDataHash: request.asset.contentHashes.voxelData,
        sessionHash: 'fnv1a64:0000000000000110',
        replayHash: 'fnv1a64:0000000000000111',
        diagnostics: [],
      });
    },
    unloadVoxelVolumeAsset: (_handle: number, requestJson: string) => {
      calls.push(`voxelVolumeAssetUnload:${requestJson}`);
      const request = parseJsonFixture<VoxelVolumeAssetUnloadRequest>(requestJson);
      return JSON.stringify({
        request,
        unloaded: true,
        modelId: `voxel-model:grid:${request.grid}:volume:${request.volumeAssetId}`,
        volumeAssetId: request.volumeAssetId,
        grid: request.grid,
        removedVoxelCount: 1,
        sessionHash: 'fnv1a64:0000000000000116',
        replayHash: 'fnv1a64:0000000000000117',
        diagnostics: [],
      });
    },
    validateVoxelAnnotationLayer: (_handle: number, requestJson: string) => {
      calls.push(`voxelAnnotationValidate:${requestJson}`);
      const request = parseJsonFixture<VoxelAnnotationLayerValidationRequest>(requestJson);
      const layer = request.input.kind === 'finalized' ? request.input.layer : VOXEL_ANNOTATION_LAYER;
      return JSON.stringify({
        layerId: layer.layerId,
        valid: true,
        normalizedLayer: layer,
        canonicalJsonHash: layer.contentHashes.canonicalJson,
        membershipDataHash: layer.contentHashes.membershipData,
        regionCount: layer.regions.length,
        sparseRunCount: 1,
        assignedCellCount: 1,
        diagnostics: [],
      });
    },
    loadVoxelAnnotationLayer: (_handle: number, requestJson: string) => {
      calls.push(`voxelAnnotationLoad:${requestJson}`);
      const request = parseJsonFixture<VoxelAnnotationLayerLoadRequest>(requestJson);
      return JSON.stringify({
        requestLayerId: request.layer.layerId,
        loaded: true,
        runtimeLayerId: `runtime-annotation/${request.layer.layerId}`,
        targetVoxelVolumeAssetId: request.layer.targetVoxelVolumeAssetId,
        targetVoxelDataHash: request.layer.targetVoxelDataHash,
        regionCount: request.layer.regions.length,
        assignedCellCount: 1,
        layerHash: request.layer.contentHashes.canonicalJson,
        sessionHash: 'fnv1a64:0000000000000110',
        replayHash: 'fnv1a64:0000000000000116',
        diagnostics: [],
      });
    },
    readVoxelAnnotationQuery: (_handle: number, requestJson: string) => {
      calls.push(`voxelAnnotationQuery:${requestJson}`);
      const request = parseJsonFixture<VoxelAnnotationQueryRequest>(requestJson);
      return JSON.stringify({
        request,
        matchedRegions: [{
          regionId: 'region/native-room',
          label: 'Native room',
          kind: 'room',
          tags: ['fixture'],
          parentRegionId: null,
          bounds: VOXEL_ANNOTATION_LAYER.targetBounds,
          assignedCellCount: 1,
        }],
        regionCount: 1,
        truncated: false,
        layerHash: request.expectedLayerHash,
        diagnostics: [],
      });
    },
    applyVoxelAnnotationEdit: (_handle: number, requestJson: string) => {
      calls.push(`voxelAnnotationEdit:${requestJson}`);
      const request = parseJsonFixture<VoxelAnnotationEditRequest>(requestJson);
      return JSON.stringify({
        request,
        edited: true,
        layerHashBefore: request.expectedLayerHash,
        layerHashAfter: 'fnv1a64:0000000000000115',
        regionCount: 1,
        assignedCellCount: 1,
        diagnostics: [],
        replayHash: 'fnv1a64:0000000000000117',
      });
    },
    exportVoxelAnnotationLayer: (_handle: number, requestJson: string) => {
      calls.push(`voxelAnnotationExport:${requestJson}`);
      const request = parseJsonFixture<VoxelAnnotationLayerExportRequest>(requestJson);
      const layer = {
        ...VOXEL_ANNOTATION_LAYER,
        contentHashes: {
          canonicalJson: request.expectedLayerHash,
          membershipData: VOXEL_ANNOTATION_LAYER.contentHashes.membershipData,
        },
      };
      return JSON.stringify({
        request,
        exported: true,
        layer,
        canonicalJson: `${JSON.stringify(layer)}\n`,
        canonicalJsonHash: layer.contentHashes.canonicalJson,
        membershipDataHash: layer.contentHashes.membershipData,
        diagnostics: [],
      });
    },
    readVoxelEditHistory: (_handle: number, requestJson: string) => {
      calls.push(`voxelHistoryRead:${requestJson}`);
      const request = parseJsonFixture<VoxelEditHistoryReadRequest>(requestJson);
      return JSON.stringify({
        historyId: request.historyId,
        schemaVersion: 1,
        mediaType: 'application/vnd.asha.voxel-edit-history+json;version=1',
        targetGrid: 1,
        targetVoxelVolumeAssetId: 'voxel/generated',
        baseVoxelHash: 'fnv1a64:base-native',
        materialCatalogHash: 'fnv1a64:materials-native',
        cursor: voxelHistoryRevertFixture(
          { ...VOXEL_EDIT_HISTORY_REVERT_REQUEST, historyId: request.historyId },
          false,
        ).cursorBefore,
        entries: [],
        retainedRedoTransactionIds: [],
        historyHash: 'fnv1a64:history-native',
        diagnostics: [],
      });
    },
    previewVoxelEditRevert: (_handle: number, requestJson: string) => {
      calls.push(`voxelHistoryPreview:${requestJson}`);
      return JSON.stringify(voxelHistoryRevertFixture(
        parseJsonFixture<VoxelEditHistoryRevertRequest>(requestJson),
        false,
      ));
    },
    applyVoxelEditRevert: (_handle: number, requestJson: string) => {
      calls.push(`voxelHistoryApply:${requestJson}`);
      return JSON.stringify(voxelHistoryRevertFixture(
        parseJsonFixture<VoxelEditHistoryRevertRequest>(requestJson),
        true,
      ));
    },
    undoVoxelEdit: (_handle: number, requestJson: string) => {
      calls.push(`voxelHistoryUndo:${requestJson}`);
      return JSON.stringify({
        request: parseJsonFixture<VoxelEditHistoryUndoRequest>(requestJson),
        receipt: voxelHistoryRevertFixture({ ...VOXEL_EDIT_HISTORY_REVERT_REQUEST, mode: 'undo' }, true),
      });
    },
    redoVoxelEdit: (_handle: number, requestJson: string) => {
      calls.push(`voxelHistoryRedo:${requestJson}`);
      return JSON.stringify({
        request: parseJsonFixture<VoxelEditHistoryRedoRequest>(requestJson),
        receipt: voxelHistoryRevertFixture({ ...VOXEL_EDIT_HISTORY_REVERT_REQUEST, mode: 'redo' }, true),
      });
    },
  } as unknown as NativeAddon;
}

const INVOKE = createNativeOperationInvocations({
  collisionCamera: COLLISION_CAMERA_INPUT,
  cameraInput: CAMERA_INPUT,
  cameraCreate: CAMERA_CREATE_REQUEST,
  cameraMode: CAMERA_MODE_COMMAND,
  cameraNavigation: CAMERA_NAVIGATION_INPUT,
  gameRuleCatalog: GAME_RULE_CATALOG,
  gameRuleRequest: GAME_RULE_REQUEST,
  hashA: HASH_A,
  voxelPlan: VOXEL_CONVERSION_PLAN_REQUEST,
  voxelSource: VOXEL_CONVERSION_SOURCE_REGISTRATION_REQUEST,
  voxelMeshAsset: VOXEL_CONVERSION_MESH_ASSET_REGISTRATION_REQUEST,
  voxelMeshImport: VOXEL_CONVERSION_MESH_SOURCE_IMPORT_REQUEST,
  voxelPlanHash: VOXEL_PLAN_HASH,
  voxelPreviewHash: VOXEL_PREVIEW_HASH,
  voxelEvidence: VOXEL_CONVERSION_EVIDENCE,
  voxelModelInfo: VOXEL_MODEL_INFO_REQUEST,
  voxelModelWindow: VOXEL_MODEL_WINDOW_REQUEST,
  voxelExport: VOXEL_VOLUME_ASSET_EXPORT_REQUEST,
  voxelSave: VOXEL_VOLUME_ASSET_SAVE_REQUEST,
  voxelPaletteUpdate: voxelPaletteUpdateRequest(VOXEL_VOLUME_ASSET_LOAD_REQUEST.asset),
  voxelAuthoring: VOXEL_VOLUME_AUTHORING_INITIALIZE_REQUEST,
  voxelLoad: VOXEL_VOLUME_ASSET_LOAD_REQUEST,
  voxelUnload: VOXEL_VOLUME_ASSET_UNLOAD_REQUEST,
  annotationValidation: VOXEL_ANNOTATION_VALIDATION_REQUEST,
  annotationLoad: VOXEL_ANNOTATION_LOAD_REQUEST,
  annotationQuery: VOXEL_ANNOTATION_QUERY_REQUEST,
  annotationEdit: VOXEL_ANNOTATION_EDIT_REQUEST,
  annotationExport: VOXEL_ANNOTATION_EXPORT_REQUEST,
  historyRead: VOXEL_EDIT_HISTORY_READ_REQUEST,
  historyRevert: VOXEL_EDIT_HISTORY_REVERT_REQUEST,
  historyUndo: VOXEL_EDIT_HISTORY_UNDO_REQUEST,
  historyRedo: VOXEL_EDIT_HISTORY_REDO_REQUEST,
  materialPreview: MODEL_MATERIAL_PREVIEW_REQUEST,
  inputConfigure: INPUT_SESSION_CONFIGURE_REQUEST,
  inputContextCommand: INPUT_CONTEXT_COMMAND,
  rawInput: RAW_INPUT_SAMPLE,
  recordedInput: RECORDED_INPUT_ACTION,
  timeControlCommand: { operation: 'pause' },
});

void test('every manifest op has a native invocation in this test', () => {
  for (const op of MANIFEST_OPERATIONS) {
    assert.ok(INVOKE.has(op.facadeMethod), `missing invocation for ${op.facadeMethod}`);
  }
});

void test('unwired native ops fail closed with operation_unimplemented (no mock fallback)', () => {
  for (const op of MANIFEST_OPERATIONS) {
    if (NATIVE_WIRED_OPERATIONS.has(op.manifestName)) continue;
    const invoke = INVOKE.get(op.facadeMethod);
    assert.ok(invoke, `missing invocation for ${op.facadeMethod}`);
    const bridge = new NativeRuntimeBridge(fakeAddon());
    // A fresh, initialized bridge: proves the throw is fail-closed classification,
    // not an incidental `not_initialized`.
    bridge.initializeEngine({ seed: 1 });
    assert.throws(
      () => invoke(bridge),
      (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'operation_unimplemented',
      `${op.manifestName} must fail closed, not inherit mock behaviour`,
    );
  }
});

void test('required native conformance operations are declared wired', () => {
  for (const manifestName of REQUIRED_NATIVE_CONFORMANCE_OPS) {
    assert.ok(
      NATIVE_WIRED_OPERATIONS.has(manifestName),
      `${manifestName} must be wired for native authority conformance`,
    );
  }
});

void test('native conformance sequence routes through the addon without mock fallback', () => {
  const calls: string[] = [];
  const bridge: RuntimeBridge = new NativeRuntimeBridge(fakeAddon(calls));

  assert.equal(bridge.initializeEngine({ seed: 7 }) as number, 107);
  assert.deepEqual(
    bridge.submitCommands({
      commands: [
        { op: 'setVoxel', grid: 1, coord: { x: 0, y: 0, z: 0 }, value: { kind: 'solid', material: 1 } },
      ],
    }),
    { accepted: 1, rejected: 0, rejections: [] },
  );
  assert.deepEqual(bridge.stepSimulation({ tick: 6 }), { tick: 6, diffCount: 9 });
  assert.deepEqual(bridge.createCamera(CAMERA_CREATE_REQUEST), {
    camera: 1,
    tick: 0,
    pose: CAMERA_CREATE_REQUEST.initialPose,
    basis: {
      forward: [0, 0, -1],
      right: [1, 0, 0],
      up: [0, 1, 0],
    },
    projection: CAMERA_CREATE_REQUEST.projection,
    viewport: CAMERA_CREATE_REQUEST.viewport,
  });
  const constrainedCamera = bridge.applyCollisionConstrainedCameraInput(COLLISION_CAMERA_INPUT);
  assert.equal(constrainedCamera.camera, COLLISION_CAMERA_INPUT.camera);
  assert.equal(constrainedCamera.tick, COLLISION_CAMERA_INPUT.tick);
  assert.equal(constrainedCamera.collision.collided, true);
  assert.deepEqual(constrainedCamera.collision.blockedAxes, ['z']);
  assert.equal(constrainedCamera.movementHash, 'fnv1a64:sentinel-movement');
  assert.equal(bridge.applyCameraModeCommand(CAMERA_MODE_COMMAND).after.mode, 'orbit');
  assert.equal(bridge.applyCameraNavigationInput(CAMERA_NAVIGATION_INPUT).after.distance, 5);
  assert.equal(bridge.readCameraControllerState({ camera: CAMERA_INPUT.camera }).stateHash, HASH_B);
  assert.deepEqual(bridge.applyEnemyDirectNavMovement({
    entity: 777,
    seedPosition: [0, 0.5, -2.6],
    target: [0, 1.62, 1.25],
    maxStepUnits: 0.35,
  }), {
    entity: 777,
    authoritySource: 'rust_entity_store',
    authorityTransport: 'native_rust',
    from: [0, 0.5, -2.6],
    target: [0, 1.62, 1.25],
    nextWaypoint: [2, 1, 7],
    distanceUnits: 4.01,
    reached: false,
    pathHash: 'fnv1a64:sentinel-path',
    transformHash: 'fnv1a64:sentinel-transform',
    projectionChanged: true,
  });
  const fired = bridge.applyFpsPrimaryFire({ tick: 9, origin: [2.5, 1.5, 1.5], direction: [0, 0, 1] });
  assert.equal(fired.backend, 'native_rust');
  assert.deepEqual(fired.lifecycleStatus, { state: 'enemy_defeated', entity: 777, tick: 9 });
  assert.equal(fired.targetHealthAfter?.current, 0);
  const catalogValidation = bridge.validateGameRuleCatalog(GAME_RULE_CATALOG);
  assert.equal(catalogValidation.accepted, true);
  const gameRuleReceipt = bridge.submitGameRuleEffectIntent({
    catalog: GAME_RULE_CATALOG,
    request: GAME_RULE_REQUEST,
  });
  assert.equal(gameRuleReceipt.accepted, true);
  assert.equal(gameRuleReceipt.appliedModifiers[0]?.nextTick, 11);
  const gameRuleReadout = bridge.readGameRuleRuntimeReadout();
  assert.equal(gameRuleReadout.backend, 'native_rust');
  assert.equal(gameRuleReadout.activeModifiers[0]?.modifierId, 'modifier.poison');
  assert.equal(bridge.readFpsRuntimeSession().replayHash, HASH_C);
  assert.equal(bridge.restartFpsRuntimeSession({ expectedEpoch: 1 }).sessionEpoch, 2);
  const encounter = bridge.readFpsEncounterDirector({
    outcomeKind: 'in_progress',
    terminal: false,
    enemyDead: false,
    playerDead: false,
    lifecycleHash: HASH_A,
  });
  assert.equal(encounter.backend, 'native_rust');
  assert.equal(encounter.encounterHash, 'fnv1a64:00000000000000dd');
  const encounterTransition = bridge.applyFpsEncounterTransition({
    presetId: 'generated-tunnel-small-encounter',
    action: 'activate',
    lifecycle: {
      outcomeKind: 'in_progress',
      terminal: false,
      enemyDead: false,
      playerDead: false,
      lifecycleHash: HASH_A,
    },
  });
  assert.equal(encounterTransition.accepted, true);
  assert.equal(encounterTransition.replayHash, 'fnv1a64:00000000000000f0');
  const registration = bridge.registerVoxelConversionSource(VOXEL_CONVERSION_SOURCE_REGISTRATION_REQUEST);
  assert.equal(registration.registered, true);
  assert.equal(registration.source.assetId, 'mesh/native-registered-triangle');
  assert.equal(registration.materialSlots[0]?.sourceMaterialId, 'mat/a');
  const meshAssetRegistration = bridge.registerVoxelConversionMeshAsset(VOXEL_CONVERSION_MESH_ASSET_REGISTRATION_REQUEST);
  assert.equal(meshAssetRegistration.registered, true);
  assert.equal(meshAssetRegistration.source.assetId, 'mesh/quad');
  assert.equal(meshAssetRegistration.materialSlots[0]?.sourceMaterialId, 'mat/a');
  assert.deepEqual(bridge.readRenderDiffs(frameCursor(0)), { ops: [] });
  bridge.readProjectionFrame(frameCursor(0));
  assert.deepEqual(calls, [
    'initialize:7',
    'submit:[{"op":"setVoxel","grid":1,"coord":{"x":0,"y":0,"z":0},"value":{"kind":"solid","material":1}}]',
    'step:6',
    'createCamera:0,1.6,0',
    'cameraCollision:1:1:1',
    `cameraMode:${JSON.stringify(CAMERA_MODE_COMMAND)}`,
    `cameraNavigation:${JSON.stringify(CAMERA_NAVIGATION_INPUT)}`,
    'cameraControllerRead:{"camera":1}',
    'enemyMove:777:0,0.5,-2.6:0,1.62,1.25:0.35',
    'fpsFire:9:2.5,1.5,1.5:0,0,1',
    'gameRuleValidate:catalog.game-rules.native',
    'gameRuleSubmit:catalog.game-rules.native:bundle.poisoned-impact',
    'gameRuleReadout',
    'fpsRead',
    'fpsRestart:1',
    'fpsEncounterRead',
    'fpsEncounterTransition',
    'voxelRegister:{"source":{"assetId":"mesh/native-registered-triangle","assetKind":"mesh","assetVersion":2,"sourceHash":"sha256:native-registered-triangle","meshPrimitive":"default"},"positions":[[0,0,0],[1,0,0],[0,1,0]],"triangles":[{"indices":[0,1,2],"sourceMaterialSlot":0}],"materialSlots":[{"sourceMaterialSlot":0,"sourceMaterialId":"mat/a"}]}',
    'voxelMeshAssetRegister:{"source":{"assetId":"mesh/quad","assetKind":"mesh","assetVersion":1,"sourceHash":"sha256:quad","meshPrimitive":null},"meshAsset":{"assetId":"mesh/quad","sourcePath":"assets/mesh/quad.mesh.json","positions":[[0,0,0],[1,0,0],[0,1,0]],"normals":[],"indices":[0,1,2],"groups":[{"materialSlot":0,"start":0,"count":3}],"materialSlots":[{"sourceMaterialSlot":0,"sourceMaterialId":"mat/a"}]}}',
    'render:0',
    'projection:0',
  ]);
});

void test('native voxel volume unload routes generated request and receipt JSON', () => {
  const calls: string[] = [];
  const bridge: RuntimeBridge = new NativeRuntimeBridge(fakeAddon(calls));
  bridge.initializeEngine({ seed: 3 });

  const receipt = bridge.unloadVoxelVolumeAsset(VOXEL_VOLUME_ASSET_UNLOAD_REQUEST);

  assert.deepEqual(receipt.request, VOXEL_VOLUME_ASSET_UNLOAD_REQUEST);
  assert.equal(receipt.unloaded, true);
  assert.equal(receipt.modelId, 'voxel-model:grid:1:volume:voxel/generated');
  assert.equal(receipt.removedVoxelCount, 1);
  assert.equal(receipt.sessionHash, 'fnv1a64:0000000000000116');
  assert.deepEqual(calls, [
    'initialize:3',
    `voxelVolumeAssetUnload:${JSON.stringify(VOXEL_VOLUME_ASSET_UNLOAD_REQUEST)}`,
  ]);
});

void test('native facade validates numeric simulation inputs before addon casts can wrap', () => {
  const calls: string[] = [];
  const bridge: RuntimeBridge = new NativeRuntimeBridge(fakeAddon(calls));
  bridge.initializeEngine({ seed: 1 });

  assert.throws(
    () => bridge.stepSimulation({ tick: -1 }),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'invalid_input',
  );
  assert.throws(
    () => bridge.readRenderDiffs(frameCursor(-1)),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'invalid_input',
  );
  assert.deepEqual(calls, ['initialize:1']);
});

void test('native developer console restores nullable fields omitted by napi', () => {
  const addon = fakeAddon();
  addon.readDeveloperConsole = () => ({
    schemaVersion: 1,
    records: [{
      sequence: 0,
      severity: 'warning',
      category: 'resource',
      source: 'projection',
      message: 'audio unavailable',
      detail: { code: 'resource_degraded' },
    }],
    droppedRecordCount: 0,
    nextSequence: 1,
    snapshotHash: 'fnv1a64:console',
  }) as never;
  const bridge = new NativeRuntimeBridge(addon);
  bridge.initializeEngine({ seed: 1 });

  const snapshot = bridge.readDeveloperConsole();
  assert.equal(snapshot.firstSequence, null);
  assert.equal(snapshot.records[0]?.correlation, null);
  assert.equal(snapshot.records[0]?.detail.resourceId, null);
});

void test('wired native ops route through the addon, not the mock', () => {
  const calls: string[] = [];
  const bridge = new NativeRuntimeBridge(fakeAddon(calls));
  // The reference bridge has no queued authority commands and returns diffCount
  // 0; this addon fixture proves native results are forwarded unchanged.
  assert.equal(bridge.initializeEngine({ seed: 7 }) as number, 107);
  assert.deepEqual(bridge.stepSimulation({ tick: 6 }), { tick: 6, diffCount: 9 });
  assert.deepEqual(calls, ['initialize:7', 'step:6']);
});

void test('native bridge does not extend MockRuntimeBridge (no inherited mock methods)', () => {
  // Guards against re-introducing the `extends MockRuntimeBridge` seam: every
  // own/inherited facade method must be declared on NativeRuntimeBridge itself.
  const proto = NativeRuntimeBridge.prototype as unknown as Record<string, unknown>;
  for (const op of MANIFEST_OPERATIONS) {
    assert.ok(
      Object.prototype.hasOwnProperty.call(
        Object.getPrototypeOf(new NativeRuntimeBridge(fakeAddon())),
        op.facadeMethod,
      ),
      `${op.facadeMethod} must be declared on NativeRuntimeBridge, not inherited`,
    );
    assert.equal(typeof proto[op.facadeMethod], 'function');
  }
});

void test('native bridge step before init fails closed (not_initialized)', () => {
  const bridge = new NativeRuntimeBridge(fakeAddon());
  assert.throws(
    () => bridge.stepSimulation({ tick: 1 }),
    (e: unknown) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized',
  );
});
