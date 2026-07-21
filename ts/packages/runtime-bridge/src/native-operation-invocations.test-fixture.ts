import { entityId, projectId, type FlatSceneDocument, type SceneId, type SceneNodeId } from '@asha/contracts';
import {
  frameCursor,
  type ReplaySessionHandle,
  type RuntimeBridge,
  type RuntimeBufferHandle,
} from './bridge.js';
import { MANIFEST_OPERATIONS } from './generated/operations.js';

export type NativeOperationInvocation = (bridge: RuntimeBridge) => unknown;

const SCENE_DOCUMENT: FlatSceneDocument = {
  schemaVersion: 4,
  id: 1 as SceneId,
  metadata: { name: 'Invocation fixture', authoringFormatVersion: 4 },
  dependencies: [],
  nodes: [{
    id: 1 as SceneNodeId,
    parent: null,
    childOrder: 0,
    label: 'Root',
    tags: [],
    transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
    kind: { kind: 'emptyGroup' },
  }],
};

export interface NativeOperationInvocationInputs {
  readonly collisionCamera: Parameters<RuntimeBridge['applyCollisionConstrainedCameraInput']>[0];
  readonly cameraInput: Parameters<RuntimeBridge['applyFirstPersonCameraInput']>[0];
  readonly cameraCreate: Parameters<RuntimeBridge['createCamera']>[0];
  readonly cameraMode: Parameters<RuntimeBridge['applyCameraModeCommand']>[0];
  readonly cameraNavigation: Parameters<RuntimeBridge['applyCameraNavigationInput']>[0];
  readonly gameRuleCatalog: Parameters<RuntimeBridge['validateGameRuleCatalog']>[0];
  readonly gameRuleRequest: Parameters<RuntimeBridge['submitGameRuleEffectIntent']>[0]['request'];
  readonly hashA: string;
  readonly voxelPlan: Parameters<RuntimeBridge['planVoxelConversion']>[0];
  readonly voxelSource: Parameters<RuntimeBridge['registerVoxelConversionSource']>[0];
  readonly voxelMeshAsset: Parameters<RuntimeBridge['registerVoxelConversionMeshAsset']>[0];
  readonly voxelMeshImport: Parameters<RuntimeBridge['importVoxelConversionMeshSource']>[0];
  readonly voxelPlanHash: string;
  readonly voxelPreviewHash: string;
  readonly voxelEvidence: Parameters<RuntimeBridge['exportVoxelConversionEvidence']>[0];
  readonly voxelModelInfo: Parameters<RuntimeBridge['readVoxelModelInfo']>[0];
  readonly voxelModelWindow: Parameters<RuntimeBridge['readVoxelModelWindow']>[0];
  readonly voxelExport: Parameters<RuntimeBridge['exportVoxelVolumeAsset']>[0];
  readonly voxelSave: Parameters<RuntimeBridge['saveVoxelVolumeAsset']>[0];
  readonly voxelPaletteUpdate: Parameters<RuntimeBridge['updateVoxelVolumeAssetPalette']>[0];
  readonly voxelAuthoring: Parameters<RuntimeBridge['initializeVoxelVolumeAuthoring']>[0];
  readonly voxelLoad: Parameters<RuntimeBridge['loadVoxelVolumeAsset']>[0];
  readonly voxelUnload: Parameters<RuntimeBridge['unloadVoxelVolumeAsset']>[0];
  readonly annotationValidation: Parameters<RuntimeBridge['validateVoxelAnnotationLayer']>[0];
  readonly annotationLoad: Parameters<RuntimeBridge['loadVoxelAnnotationLayer']>[0];
  readonly annotationQuery: Parameters<RuntimeBridge['readVoxelAnnotationQuery']>[0];
  readonly annotationEdit: Parameters<RuntimeBridge['applyVoxelAnnotationEdit']>[0];
  readonly annotationExport: Parameters<RuntimeBridge['exportVoxelAnnotationLayer']>[0];
  readonly historyRead: Parameters<RuntimeBridge['readVoxelEditHistory']>[0];
  readonly historyRevert: Parameters<RuntimeBridge['previewVoxelEditRevert']>[0];
  readonly historyUndo: Parameters<RuntimeBridge['undoVoxelEdit']>[0];
  readonly historyRedo: Parameters<RuntimeBridge['redoVoxelEdit']>[0];
  readonly materialPreview: Parameters<RuntimeBridge['readModelMaterialPreview']>[0];
  readonly inputConfigure: Parameters<RuntimeBridge['configureInputSession']>[0];
  readonly inputContextCommand: Parameters<RuntimeBridge['applyInputContextCommand']>[0];
  readonly rawInput: Parameters<RuntimeBridge['submitRawInput']>[0];
  readonly recordedInput: Parameters<RuntimeBridge['replayResolvedInputAction']>[0];
  readonly timeControlCommand: Parameters<RuntimeBridge['applyTimeControlCommand']>[0];
}

export function createNativeOperationInvocations(
  input: NativeOperationInvocationInputs,
): ReadonlyMap<string, NativeOperationInvocation> {
  return composeNativeOperationInvocations([
    ['initializeEngine', (bridge) => bridge.initializeEngine({ seed: 7 })],
    ['readWorkspaceAuthoringState', (bridge) => bridge.readWorkspaceAuthoringState()],
    ['readWorkspaceAuthoringProjection', (bridge) => bridge.readWorkspaceAuthoringProjection({
      expectedWorkspaceId: 'workspace/native-fixture',
      expectedGeneration: 1,
      expectedWorkingRevision: 0,
      cursor: frameCursor(0),
    })],
    ['confirmWorkspaceAuthoringStored', (bridge) => bridge.confirmWorkspaceAuthoringStored({
      expectedWorkspaceId: 'workspace/native-fixture',
      expectedGeneration: 1,
      hostPath: 'assets/voxels/native-fixture.avxl.json',
      canonicalJsonHash: input.hashA,
    })],
    ['closeWorkspaceAuthoring', (bridge) => bridge.closeWorkspaceAuthoring({
      expectedWorkspaceId: 'workspace/native-fixture',
      expectedGeneration: 1,
      discardUnsavedWorkingState: true,
    })],
    ['configureInputSession', (bridge) => bridge.configureInputSession(input.inputConfigure)],
    ['applyInputContextCommand', (bridge) => bridge.applyInputContextCommand(input.inputContextCommand)],
    ['submitRawInput', (bridge) => bridge.submitRawInput(input.rawInput)],
    ['replayResolvedInputAction', (bridge) => bridge.replayResolvedInputAction(input.recordedInput)],
    ['readInputContextState', (bridge) => bridge.readInputContextState()],
    ['applyTimeControlCommand', (bridge) => bridge.applyTimeControlCommand(input.timeControlCommand)],
    ['readTimeControlState', (bridge) => bridge.readTimeControlState()],
    ['stepSimulation', (bridge) => bridge.stepSimulation({ tick: 6 })],
    ['submitCommands', (bridge) => bridge.submitCommands({ commands: [] })],
    ['pickVoxel', (bridge) => bridge.pickVoxel({ grid: 1, origin: [0, 0, 0], direction: [1, 0, 0], maxDistance: 10 })],
    ['configureVoxelProjectionInstances', (bridge) => bridge.configureVoxelProjectionInstances({
      workspaceId: 'workspace/native-fixture',
      workspaceGeneration: 1,
      workingRevision: 0,
      registryDigest: input.hashA,
      instances: [{
        instanceId: 'scene-node/1',
        sceneNodeId: 1,
        assetId: 'voxel/fixture',
        transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
      }],
    })],
    ['pickVoxelInstance', (bridge) => bridge.pickVoxelInstance({
      workspaceId: 'workspace/native-fixture',
      workspaceGeneration: 1,
      workingRevision: 0,
      registryDigest: input.hashA,
      bindingHash: input.hashA,
      instanceId: 'scene-node/1',
      origin: [0, 0.5, 0.5],
      direction: [1, 0, 0],
      maxDistance: 10,
      rendererHint: { localVoxel: { x: 0, y: 0, z: 0 }, localFace: 'negX' },
    })],
    ['applyCollisionConstrainedCameraInput', (bridge) => bridge.applyCollisionConstrainedCameraInput(input.collisionCamera)],
    ['selectVoxel', (bridge) => bridge.selectVoxel({
      camera: input.cameraInput.camera,
      grid: 1,
      viewport: null,
      screenPoint: { x: 0.5, y: 0.5, space: 'normalized_0_1' },
      maxDistance: 10,
    })],
    ['readVoxelMeshEvidence', (bridge) => bridge.readVoxelMeshEvidence({ grid: 1, chunks: [] })],
    ['readVoxelUpdateTelemetry', (bridge) => {
      bridge.readRenderDiffs(frameCursor(0));
      return bridge.readVoxelUpdateTelemetry({ grid: 1, projectionCursor: 0 });
    }],
    ['readFpsRuntimeSession', (bridge) => bridge.readFpsRuntimeSession()],
    ['applyFpsPrimaryFire', (bridge) => bridge.applyFpsPrimaryFire({ tick: 9, origin: [2.5, 1.5, 1.5], direction: [0, 0, 1] })],
    ['readComposedRuntimeSession', (bridge) => bridge.readComposedRuntimeSession()],
    ['readGameplayModuleView', (bridge) => bridge.readGameplayModuleView({
      view: {
        namespace: 'asha.fixture.gameplay',
        name: 'pulse-state',
        version: 1,
        schemaHash: input.hashA,
      },
      scope: { kind: 'session' },
      expectedRuntimeSessionHash: input.hashA,
    })],
    ['applyGameplayPrefabPartInteraction', (bridge) => bridge.applyGameplayPrefabPartInteraction({
      actor: 101,
      instance: 1,
      role: 'interaction-target',
      expectedTarget: 777,
      tick: 9,
      expectedRuntimeSessionHash: input.hashA,
    })],
    ['invokeGameExtensionWeaponEffect', (bridge) => bridge.invokeGameExtensionWeaponEffect({
      hook: {
        moduleRef: {
          moduleId: 'asha.reference.primary_fire_damage_modifier',
          version: '0.1.0',
          contractHash: 'sha256:asha-reference-primary-fire-damage-modifier-v0',
        },
        hookId: 'weapon.primary.damage_modifier',
        requestId: 'request.native-fixture',
        tick: 9,
        source: entityId(101),
        target: entityId(777),
        baseDamage: 75,
        rangeMillimeters: 16000,
        tags: ['primary-fire'],
        inputHash: input.hashA,
      },
      primaryFire: { tick: 9, origin: [2.5, 1.5, 1.5], direction: [0, 0, 1] },
    })],
    ['validateGameRuleCatalog', (bridge) => bridge.validateGameRuleCatalog(input.gameRuleCatalog)],
    ['submitGameRuleEffectIntent', (bridge) => bridge.submitGameRuleEffectIntent({
      catalog: input.gameRuleCatalog,
      request: input.gameRuleRequest,
    })],
    ['readGameRuleRuntimeReadout', (bridge) => bridge.readGameRuleRuntimeReadout()],
    ['restartFpsRuntimeSession', (bridge) => bridge.restartFpsRuntimeSession({ expectedEpoch: 1 })],
    ['readFpsEncounterDirector', (bridge) => bridge.readFpsEncounterDirector({
      outcomeKind: 'in_progress', terminal: false, enemyDead: false, playerDead: false,
      lifecycleHash: input.hashA,
    })],
    ['applyFpsEncounterTransition', (bridge) => bridge.applyFpsEncounterTransition({
      presetId: 'generated-tunnel-small-encounter',
      action: 'activate',
      lifecycle: {
        outcomeKind: 'in_progress', terminal: false, enemyDead: false, playerDead: false,
        lifecycleHash: input.hashA,
      },
    })],
    ['planVoxelConversion', (bridge) => bridge.planVoxelConversion(input.voxelPlan)],
    ['registerVoxelConversionSource', (bridge) => bridge.registerVoxelConversionSource(input.voxelSource)],
    ['registerVoxelConversionMeshAsset', (bridge) => bridge.registerVoxelConversionMeshAsset(input.voxelMeshAsset)],
    ['importVoxelConversionMeshSource', (bridge) => bridge.importVoxelConversionMeshSource(input.voxelMeshImport)],
    ['readVoxelConversionSourceMetadata', (bridge) => bridge.readVoxelConversionSourceMetadata({ source: input.voxelSource.source })],
    ['previewVoxelConversion', (bridge) => bridge.previewVoxelConversion({ planId: 'fnv1a64:0000000000000101', expectedPlanHash: input.voxelPlanHash })],
    ['applyVoxelConversion', (bridge) => bridge.applyVoxelConversion({
      planId: 'fnv1a64:0000000000000101',
      expectedPlanHash: input.voxelPlanHash,
      expectedPreviewHash: input.voxelPreviewHash,
    })],
    ['exportVoxelConversionEvidence', (bridge) => bridge.exportVoxelConversionEvidence(input.voxelEvidence)],
    ['readVoxelModelInfo', (bridge) => bridge.readVoxelModelInfo(input.voxelModelInfo)],
    ['readVoxelModelWindow', (bridge) => bridge.readVoxelModelWindow(input.voxelModelWindow)],
    ['exportVoxelVolumeAsset', (bridge) => bridge.exportVoxelVolumeAsset(input.voxelExport)],
    ['saveVoxelVolumeAsset', (bridge) => bridge.saveVoxelVolumeAsset(input.voxelSave)],
    ['updateVoxelVolumeAssetPalette', (bridge) => bridge.updateVoxelVolumeAssetPalette(input.voxelPaletteUpdate)],
    ['initializeVoxelVolumeAuthoring', (bridge) => bridge.initializeVoxelVolumeAuthoring(input.voxelAuthoring)],
    ['loadVoxelVolumeAsset', (bridge) => bridge.loadVoxelVolumeAsset(input.voxelLoad)],
    ['unloadVoxelVolumeAsset', (bridge) => bridge.unloadVoxelVolumeAsset(input.voxelUnload)],
    ['validateVoxelAnnotationLayer', (bridge) => bridge.validateVoxelAnnotationLayer(input.annotationValidation)],
    ['loadVoxelAnnotationLayer', (bridge) => bridge.loadVoxelAnnotationLayer(input.annotationLoad)],
    ['readVoxelAnnotationQuery', (bridge) => bridge.readVoxelAnnotationQuery(input.annotationQuery)],
    ['applyVoxelAnnotationEdit', (bridge) => bridge.applyVoxelAnnotationEdit(input.annotationEdit)],
    ['exportVoxelAnnotationLayer', (bridge) => bridge.exportVoxelAnnotationLayer(input.annotationExport)],
    ['readVoxelEditHistory', (bridge) => bridge.readVoxelEditHistory(input.historyRead)],
    ['previewVoxelEditRevert', (bridge) => bridge.previewVoxelEditRevert(input.historyRevert)],
    ['applyVoxelEditRevert', (bridge) => bridge.applyVoxelEditRevert({ ...input.historyRevert, mode: 'apply_revert' })],
    ['undoVoxelEdit', (bridge) => bridge.undoVoxelEdit(input.historyUndo)],
    ['redoVoxelEdit', (bridge) => bridge.redoVoxelEdit(input.historyRedo)],
    ['decodeSceneDocument', (bridge) => bridge.decodeSceneDocument({ sourceText: JSON.stringify(SCENE_DOCUMENT) })],
    ['encodeSceneDocument', (bridge) => bridge.encodeSceneDocument({ document: SCENE_DOCUMENT })],
    ['previewProceduralEnvironment', (bridge) => bridge.previewProceduralEnvironment({
      expectedWorkspaceId: 'workspace/native-fixture',
      expectedGeneration: 1,
      expectedWorkingRevision: 0,
      expectedSceneContentHash: input.hashA,
      providerId: 'asha.tunnel.enclosed.v2',
      presetId: 'tiny-enclosed',
      seed: 42,
      target: {
        sceneId: SCENE_DOCUMENT.id,
        scenePath: 'scenes/native-fixture.scene.json',
        assetId: 'voxel-volume/native-fixture',
        assetPath: 'assets/native-fixture.avxl.json',
        voxelNodeId: 10 as SceneNodeId,
        voxelParentId: null,
        voxelChildOrder: 1,
        voxelLabel: 'Native fixture',
        voxelTransform: {
          translation: [0, 0, 0],
          rotation: [0, 0, 0, 1],
          scale: [1, 1, 1],
        },
        markerTargets: [{
          sourceMarkerId: 'player_start',
          nodeId: 11 as SceneNodeId,
          markerId: 'spawn/player',
          childOrder: 0,
        }, {
          sourceMarkerId: 'exit_hint',
          nodeId: 12 as SceneNodeId,
          markerId: 'navigation/exit',
          childOrder: 1,
        }],
      },
      materialPalette: [{
        voxelMaterial: 1,
        paletteEntryId: 'voxel-material/native-fixture',
        displayName: 'Native fixture',
        materialAssetId: 'material/native-fixture',
        materialCatalogBindingId: null,
      }],
      authoring: {
        label: 'Native fixture',
        createdBy: 'native-operation-invocations',
        sourceTool: 'native-operation-invocations',
      },
      limits: { maxVoxels: 10_000, maxSparseRuns: 10_000, maxMarkers: 8 },
    })],
    ['applyProceduralEnvironment', (bridge) => bridge.applyProceduralEnvironment({
      expectedWorkspaceId: 'workspace/native-fixture',
      expectedGeneration: 1,
      expectedWorkingRevision: 0,
      candidateHash: input.hashA,
    })],
    ['applySceneDocumentAuthoring', (bridge) => bridge.applySceneDocumentAuthoring({
      currentProjectId: projectId(1),
      expectedContentHash: 'fnv1a64:fixture',
      currentDocument: SCENE_DOCUMENT,
      command: {
        kind: 'refreshProjection',
        target: { projectId: projectId(1), sceneId: SCENE_DOCUMENT.id },
      },
    })],
    ['decodeProjectContent', (bridge) => bridge.decodeProjectContent({
      sources: [],
    })],
    ['encodeProjectContent', (bridge) => bridge.encodeProjectContent({
      documents: [],
    })],
    ['applyProjectContentAuthoring', (bridge) => bridge.applyProjectContentAuthoring({
      expectedWorkspaceId: 'workspace/native-fixture',
      expectedGeneration: 1,
      expectedWorkingRevision: 0,
      expectedSetHash: input.hashA,
      command: {
        kind: 'delete',
        documentId: 'entity/native-fixture',
        documentKind: 'entityDefinition',
      },
    })],
    ['prepareProjectWrite', (bridge) => bridge.prepareProjectWrite({
      expectedWorkspaceId: 'workspace/native-fixture',
      expectedGeneration: 1,
      expectedWorkingRevision: 0,
      observedPrior: {
        revision: 0,
        manifestHash: input.hashA,
        contentSetHash: input.hashA,
        indexHash: null,
      },
      priorManifestJson: '{}',
      relocations: [],
    })],
    ['confirmProjectWrite', (bridge) => bridge.confirmProjectWrite({
      expectedWorkspaceId: 'workspace/native-fixture',
      expectedGeneration: 1,
      expectedWorkingRevision: 0,
      publication: {
        candidateHash: input.hashA,
        published: {
          revision: 1,
          manifestHash: input.hashA,
          contentSetHash: input.hashA,
          indexHash: null,
        },
      },
    })],
    ['readModelMaterialPreview', (bridge) => bridge.readModelMaterialPreview(input.materialPreview)],
    ['readSceneObjectSnapshot', (bridge) => bridge.readSceneObjectSnapshot()],
    ['applySceneObjectCommand', (bridge) => bridge.applySceneObjectCommand({ expectedDocumentHash: 1, command: { kind: 'select', id: null } })],
    ['readRenderDiffs', (bridge) => bridge.readRenderDiffs(frameCursor(0))],
    ['readProjectionFrame', (bridge) => bridge.readProjectionFrame(frameCursor(0))],
    ['readDeveloperConsole', (bridge) => bridge.readDeveloperConsole()],
    ['createCamera', (bridge) => bridge.createCamera(input.cameraCreate)],
    ['applyCameraModeCommand', (bridge) => bridge.applyCameraModeCommand(input.cameraMode)],
    ['applyCameraNavigationInput', (bridge) => bridge.applyCameraNavigationInput(input.cameraNavigation)],
    ['readCameraControllerState', (bridge) => bridge.readCameraControllerState({ camera: input.cameraMode.camera })],
    ['applyFirstPersonCameraInput', (bridge) => bridge.applyFirstPersonCameraInput(input.cameraInput)],
    ['applyEnemyDirectNavMovement', (bridge) => bridge.applyEnemyDirectNavMovement({
      entity: 777, seedPosition: [0, 0.5, -2.6], target: [0, 1.62, 1.25], maxStepUnits: 0.35,
    })],
    ['readCameraProjection', (bridge) => bridge.readCameraProjection({ camera: input.cameraInput.camera, viewport: null })],
    ['getBuffer', (bridge) => bridge.getBuffer(0 as RuntimeBufferHandle)],
    ['releaseBuffer', (bridge) => bridge.releaseBuffer(0 as RuntimeBufferHandle)],
    ['beginRuntimeProjectSourceResources', (bridge) => bridge.beginRuntimeProjectSourceResources({ manifestJson: '{}' })],
    ['stageRuntimeProjectSourceResource', (bridge) => bridge.stageRuntimeProjectSourceResource({ generation: 1, path: 'voxel/probe.avox', bytes: Uint8Array.of(1, 2, 3) })],
    ['admitRuntimeProjectSourceBatch', (bridge) => bridge.admitRuntimeProjectSourceBatch({ manifestJson: '{}', resourceGeneration: null, bodies: [] })],
    ['loadRuntimeProject', (bridge) => bridge.loadRuntimeProject({
      source: { kind: 'inMemory', identity: 'fixture', materializationHash: 'fnv1a64:0000000000000000' },
      expectedLifecycle: { generation: 0, revision: 0 },
    })],
    ['readActiveRuntimeProjectContent', (bridge) => bridge.readActiveRuntimeProjectContent()],
    ['closeRuntimeProject', (bridge) => bridge.closeRuntimeProject({
      expectedLifecycle: { generation: 0, revision: 0 },
    })],
    ['loadReplayFixture', (bridge) => bridge.loadReplayFixture({ name: 'x', steps: 1 })],
    ['runReplayStep', (bridge) => bridge.runReplayStep(0 as ReplaySessionHandle)],
  ]);
}

/**
 * Builds the invocation fixture against the generated bridge-manifest catalog.
 * Duplicate, missing, and non-manifest methods fail while the module loads, so
 * additive verbs cannot silently escape the real native conformance sequence.
 */
export function composeNativeOperationInvocations(
  entries: readonly (readonly [string, NativeOperationInvocation])[],
): ReadonlyMap<string, NativeOperationInvocation> {
  const invocations = new Map(entries);
  if (invocations.size !== entries.length) {
    throw new Error('native operation invocation fixture contains a duplicate facade method');
  }
  const expected = new Set(MANIFEST_OPERATIONS.map((operation) => operation.facadeMethod));
  const unexpected = [...invocations.keys()].filter((method) => !expected.has(method));
  const missing = [...expected].filter((method) => !invocations.has(method));
  if (unexpected.length > 0 || missing.length > 0) {
    throw new Error(
      `native operation invocation fixture drifted; missing=${missing.join(',')} unexpected=${unexpected.join(',')}`,
    );
  }
  return invocations;
}
