import { test } from 'node:test';
import assert from 'node:assert/strict';

import type {
  VoxelConversionApplyRequest,
  VoxelConversionEvidenceRef,
  VoxelConversionMeshAssetRegistrationRequest,
  VoxelConversionPlan,
  VoxelConversionPlanRequest,
  VoxelConversionPreview,
  VoxelConversionPreviewRequest,
  VoxelConversionReceipt,
  VoxelConversionSourceMetadataReadout,
  VoxelConversionSourceMetadataRequest,
  VoxelConversionSourceRegistration,
  VoxelConversionSourceRegistrationRequest,
  VoxelModelInfoReadout,
  VoxelModelInfoRequest,
  VoxelModelWindowReadout,
  VoxelModelWindowRequest,
  VoxelVolumeAssetExportReceipt,
  VoxelVolumeAssetExportRequest,
  VoxelVolumeAssetLoadReceipt,
  VoxelVolumeAssetLoadRequest,
  VoxelVolumeAssetSaveReceipt,
  VoxelVolumeAssetSaveRequest,
} from '@asha/contracts';

import { RuntimeBridgeError, createRuntimeSessionFacade, type RuntimeBridge } from './index.js';
import { createMockRuntimeBridge } from './mock.js';
import { createMockRuntimeSession } from './reference.js';

function sessionInput() {
  return {
    sessionId: 'runtime-session.asha-demo.reference',
    seed: 17,
    project: {
      gameId: 'asha-demo',
      workspaceId: 'workspace.local',
    },
  };
}

function voxelConversionPlanRequest(): VoxelConversionPlanRequest {
  return {
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
      resolution: [4, 4, 1],
      voxelSize: 1,
      maxOutputVoxels: 16,
      transform: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
      materialMap: {
        entries: [
          {
            sourceMaterialSlot: 0,
            sourceMaterialId: 'mat/a',
            voxelMaterial: 3,
          },
        ],
        textureAssets: [],
        textureBindings: [],
        defaultVoxelMaterial: 3,
      },
    },
  };
}

function voxelConversionSourceRegistrationRequest(): VoxelConversionSourceRegistrationRequest {
  return {
    source: voxelConversionPlanRequest().source,
    positions: [
      [0, 0, 0],
      [1, 0, 0],
      [0, 1, 0],
    ],
    triangles: [
      {
        indices: [0, 1, 2],
        sourceMaterialSlot: 0,
      },
    ],
    materialSlots: [
      {
        sourceMaterialSlot: 0,
        sourceMaterialId: 'mat/a',
      },
    ],
  };
}

function voxelConversionMeshAssetRegistrationRequest(): VoxelConversionMeshAssetRegistrationRequest {
  return {
    source: voxelConversionPlanRequest().source,
    meshAsset: {
      assetId: 'mesh/quad',
      sourcePath: 'assets/mesh/quad.mesh.json',
      positions: [
        [0, 0, 0],
        [1, 0, 0],
        [0, 1, 0],
      ],
      normals: [],
      indices: [0, 1, 2],
      groups: [{ materialSlot: 0, start: 0, count: 3 }],
      materialSlots: [
        {
          sourceMaterialSlot: 0,
          sourceMaterialId: 'mat/a',
        },
      ],
    },
  };
}

const PLAN_HASH = 'fnv1a64:plan-hash';
const PREVIEW_HASH = 'fnv1a64:preview-hash';

function voxelConversionSourceRegistration(
  request: VoxelConversionSourceRegistrationRequest,
): VoxelConversionSourceRegistration {
  return {
    source: request.source,
    registered: true,
    materialSlots: request.materialSlots,
    diagnostics: [],
    evidence: [{ kind: 'diagnostics', uri: 'asha://voxel-conversion/source/mesh/quad', contentHash: request.source.sourceHash }],
  };
}

function voxelConversionMeshAssetRegistration(
  request: VoxelConversionMeshAssetRegistrationRequest,
): VoxelConversionSourceRegistration {
  return {
    source: request.source,
    registered: true,
    materialSlots: request.meshAsset.materialSlots,
    diagnostics: [],
    evidence: [{ kind: 'source_snapshot', uri: `asha://voxel-conversion/source/${request.meshAsset.assetId}`, contentHash: request.source.sourceHash }],
  };
}

function voxelConversionPlan(request: VoxelConversionPlanRequest): VoxelConversionPlan {
  return {
    planId: 'fnv1a64:plan',
    source: request.source,
    target: request.target,
    settings: request.settings,
    authorityVersion: 'svc-voxel-conversion.v0',
    expectedSourceHash: request.source.sourceHash,
    settingsHash: 'fnv1a64:settings',
    planHash: PLAN_HASH,
    estimatedOutputVoxels: 1,
    estimatedBounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
    diagnostics: [],
    evidence: [{ kind: 'plan', uri: 'asha://voxel-conversion/plan/fnv1a64:plan', contentHash: PLAN_HASH }],
  };
}

function voxelConversionPreview(request: VoxelConversionPreviewRequest): VoxelConversionPreview {
  if (request.expectedPlanHash !== PLAN_HASH) {
    return {
      planId: request.planId,
      outputHash: '',
      outputVoxelCount: 0,
      outputBounds: null,
      sampleVoxels: [],
      diagnostics: [{
        code: 'stale_authority_snapshot',
        severity: 'error',
        reference: 'plan',
        message: 'preview request did not match the current authority plan hash',
      }],
      evidence: [],
    };
  }
  return {
    planId: request.planId,
    outputHash: PREVIEW_HASH,
    outputVoxelCount: 1,
    outputBounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
    sampleVoxels: [{ coord: { x: 0, y: 0, z: 0 }, material: 3 }],
    diagnostics: [],
    evidence: [{ kind: 'preview', uri: 'asha://voxel-conversion/preview/fnv1a64:plan', contentHash: PREVIEW_HASH }],
  };
}

function voxelConversionReceipt(request: VoxelConversionApplyRequest): VoxelConversionReceipt {
  if (request.expectedPlanHash !== PLAN_HASH || request.expectedPreviewHash !== PREVIEW_HASH) {
    return {
      planId: request.planId,
      applied: false,
      outputHash: null,
      outputVoxelCount: 0,
      outputBounds: null,
      diagnostics: [{
        code: 'conversion_replay_mismatch',
        severity: 'error',
        reference: 'preview',
        message: 'apply request expected a different preview output hash',
      }],
      evidence: [],
    };
  }
  return {
    planId: request.planId,
    applied: true,
    outputHash: PREVIEW_HASH,
    outputVoxelCount: 1,
    outputBounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
    diagnostics: [],
    evidence: [{ kind: 'apply_receipt', uri: 'asha://voxel-conversion/apply/fnv1a64:plan', contentHash: 'fnv1a64:apply' }],
  };
}

function createVoxelConversionBridge(): RuntimeBridge {
  const bridge = createMockRuntimeBridge();
  const availableEvidence: VoxelConversionEvidenceRef[] = [];
  return new Proxy(bridge, {
    get(target, property, receiver) {
      if (property === 'registerVoxelConversionSource') {
        return (request: VoxelConversionSourceRegistrationRequest): VoxelConversionSourceRegistration =>
          voxelConversionSourceRegistration(request);
      }
      if (property === 'registerVoxelConversionMeshAsset') {
        return (request: VoxelConversionMeshAssetRegistrationRequest): VoxelConversionSourceRegistration =>
          voxelConversionMeshAssetRegistration(request);
      }
      if (property === 'readVoxelConversionSourceMetadata') {
        return (request: VoxelConversionSourceMetadataRequest): VoxelConversionSourceMetadataReadout => ({
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
          latestPlanId: 'fnv1a64:plan',
          latestPlanTransform: voxelConversionPlanRequest().settings.transform,
          diagnostics: [],
          evidence: [{
            kind: 'source_snapshot',
            uri: `asha://voxel-conversion/source/${request.source.assetId}`,
            contentHash: request.source.sourceHash,
          }],
        });
      }
      if (property === 'planVoxelConversion') {
        return (request: VoxelConversionPlanRequest): VoxelConversionPlan => {
          const plan = voxelConversionPlan(request);
          availableEvidence.splice(0, availableEvidence.length, ...plan.evidence);
          return plan;
        };
      }
      if (property === 'previewVoxelConversion') {
        return (request: VoxelConversionPreviewRequest): VoxelConversionPreview => {
          const preview = voxelConversionPreview(request);
          availableEvidence.push(...preview.evidence);
          return preview;
        };
      }
      if (property === 'applyVoxelConversion') {
        return (request: VoxelConversionApplyRequest): VoxelConversionReceipt => {
          const receipt = voxelConversionReceipt(request);
          availableEvidence.push(...receipt.evidence);
          return receipt;
        };
      }
      if (property === 'exportVoxelConversionEvidence') {
        return (evidence: readonly VoxelConversionEvidenceRef[]): readonly VoxelConversionEvidenceRef[] => {
          for (const requested of evidence) {
            if (!availableEvidence.some((entry) => JSON.stringify(entry) === JSON.stringify(requested))) {
              throw new RuntimeBridgeError('invalid_input', `unknown voxel conversion evidence ref ${requested.uri}`);
            }
          }
          return evidence;
        };
      }
      if (property === 'readVoxelModelInfo') {
        return (request: VoxelModelInfoRequest): VoxelModelInfoReadout => {
          if (request.grid !== 1 || request.volumeAssetId !== 'voxel/generated') {
            return {
              request,
              resident: false,
              modelId: `voxel-model:grid:${request.grid}:volume:${request.volumeAssetId ?? 'none'}`,
              volumeAssetId: request.volumeAssetId,
              grid: request.grid,
              bounds: null,
              voxelCount: 0,
              materialCounts: [],
              source: null,
              latestPlanId: null,
              latestOutputHash: null,
              sessionHash: 'fnv1a64:0000000000000201',
              replayHash: 'fnv1a64:0000000000000202',
              evidence: [],
              diagnostics: [{
                code: 'voxel_conversion_unavailable',
                severity: 'error',
                reference: 'model',
                message: 'voxel model is not resident in current authority state',
              }],
            };
          }
          return {
            request,
            resident: true,
            modelId: 'voxel-model:grid:1:volume:voxel/generated',
            volumeAssetId: 'voxel/generated',
            grid: 1,
            bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
            voxelCount: 1,
            materialCounts: request.includeMaterialCounts ? [{ material: 3, voxelCount: 1 }] : [],
            source: voxelConversionPlanRequest().source,
            latestPlanId: 'fnv1a64:plan',
            latestOutputHash: PREVIEW_HASH,
            sessionHash: 'fnv1a64:0000000000000203',
            replayHash: 'fnv1a64:0000000000000204',
            evidence: availableEvidence,
            diagnostics: [],
          };
        };
      }
      if (property === 'readVoxelModelWindow') {
        return (request: VoxelModelWindowRequest): VoxelModelWindowReadout => {
          if (request.grid !== 1 || request.volumeAssetId !== 'voxel/generated') {
            return {
              request,
              resident: false,
              modelId: `voxel-model:grid:${request.grid}:volume:${request.volumeAssetId ?? 'none'}`,
              volumeAssetId: request.volumeAssetId,
              grid: request.grid,
              requestedBounds: request.bounds,
              modelBounds: null,
              scannedVoxelCount: 0,
              returnedSampleCount: 0,
              samples: [],
              sessionHash: 'fnv1a64:0000000000000205',
              replayHash: 'fnv1a64:0000000000000206',
              diagnostics: [{
                code: 'voxel_conversion_unavailable',
                severity: 'error',
                reference: 'model',
                message: 'voxel model is not resident in current authority state',
              }],
            };
          }
          return {
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
            sessionHash: 'fnv1a64:0000000000000207',
            replayHash: 'fnv1a64:0000000000000208',
            diagnostics: [],
          };
        };
      }
      if (property === 'exportVoxelVolumeAsset') {
        return (request: VoxelVolumeAssetExportRequest): VoxelVolumeAssetExportReceipt => {
          if (request.grid !== 1 || request.volumeAssetId !== 'voxel/generated') {
            return {
              request,
              exported: false,
              asset: null,
              canonicalJson: null,
              canonicalJsonHash: null,
              voxelDataHash: null,
              diagnostics: [{
                code: 'runtime_model_unavailable',
                severity: 'error',
                reference: 'runtimeModel',
                message: 'voxel model is not resident in current authority state',
              }],
            };
          }
          const asset = {
            assetId: request.targetAssetId,
            schemaVersion: 1,
            mediaType: 'application/vnd.asha.voxel-volume+json;version=1',
            grid: {
              origin: [0, 0, 0] as const,
              cellSize: 1,
              coordinateSystem: 'y_up_right_handed',
            },
            bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
            representation: {
              kind: 'sparse_runs' as const,
              sparseRuns: [{ start: { x: 0, y: 0, z: 0 }, length: 1, material: 3 }],
            },
            materialPalette: [{
              voxelMaterial: 3,
              paletteEntryId: 'voxel-material/surface-a',
              displayName: 'Surface A',
              materialAssetId: 'material/surface-a',
              materialCatalogBindingId: 'catalog-binding/surface-a',
            }],
            provenance: availableEvidence.map((evidence) => ({
              kind: 'converted' as const,
              uri: evidence.uri,
              contentHash: evidence.contentHash,
            })),
            authoring: {
              label: request.label,
              createdBy: request.createdBy,
              sourceTool: request.sourceTool,
            },
            validationDiagnostics: [],
            contentHashes: {
              canonicalJson: 'fnv1a64:0000000000000205',
              voxelData: 'fnv1a64:0000000000000206',
            },
          };
          return {
            request,
            exported: true,
            asset,
            canonicalJson: `${JSON.stringify(asset)}\n`,
            canonicalJsonHash: asset.contentHashes.canonicalJson,
            voxelDataHash: asset.contentHashes.voxelData,
            diagnostics: [],
          };
        };
      }
      if (property === 'loadVoxelVolumeAsset') {
        return (request: VoxelVolumeAssetLoadRequest): VoxelVolumeAssetLoadReceipt => ({
          requestAssetId: request.asset.assetId,
          loaded: true,
          modelId: `voxel-model:grid:${request.targetGrid}:volume:${request.targetVolumeAssetId ?? request.asset.assetId}`,
          volumeAssetId: request.targetVolumeAssetId ?? request.asset.assetId,
          grid: request.targetGrid,
          bounds: request.asset.bounds,
          voxelCount: request.asset.representation.sparseRuns.reduce((sum, run) => sum + run.length, 0),
          materialCounts: request.includeMaterialCounts ? [{ material: 3, voxelCount: 1 }] : [],
          provenance: request.asset.provenance,
          canonicalJsonHash: request.asset.contentHashes.canonicalJson,
          voxelDataHash: request.asset.contentHashes.voxelData,
          sessionHash: 'fnv1a64:0000000000000207',
          replayHash: 'fnv1a64:0000000000000208',
          diagnostics: [],
        });
      }
      if (property === 'saveVoxelVolumeAsset') {
        return (request: VoxelVolumeAssetSaveRequest): VoxelVolumeAssetSaveReceipt => {
          const exportReceipt = (receiver as RuntimeBridge).exportVoxelVolumeAsset(request.exportRequest);
          if (!exportReceipt.exported || exportReceipt.asset === null) {
            return {
              request,
              saved: false,
              diff: null,
              asset: null,
              canonicalJson: null,
              canonicalJsonHash: null,
              voxelDataHash: null,
              diagnostics: exportReceipt.diagnostics,
            };
          }
          return {
            request,
            saved: true,
            diff: {
              projectBundle: request.targetProjectBundle,
              assetId: exportReceipt.asset.assetId,
              assetPath: request.targetAssetPath,
              operation: request.expectedExistingCanonicalJsonHash === null ? 'create' : 'replace',
              previousCanonicalJsonHash: request.expectedExistingCanonicalJsonHash,
              nextCanonicalJsonHash: exportReceipt.asset.contentHashes.canonicalJson,
              nextVoxelDataHash: exportReceipt.asset.contentHashes.voxelData,
              representationKind: exportReceipt.asset.representation.kind,
              sparseRunCount: exportReceipt.asset.representation.sparseRuns.length,
              voxelCount: exportReceipt.asset.representation.sparseRuns.reduce((sum, run) => sum + run.length, 0),
              materialCount: exportReceipt.asset.materialPalette.length,
              provenanceCount: exportReceipt.asset.provenance.length,
              runtimeSessionHash: request.exportRequest.expectedSessionHash ?? 'fnv1a64:0000000000000203',
            },
            asset: exportReceipt.asset,
            canonicalJson: exportReceipt.canonicalJson,
            canonicalJsonHash: exportReceipt.canonicalJsonHash,
            voxelDataHash: exportReceipt.voxelDataHash,
            diagnostics: [],
          };
        };
      }
      const value = Reflect.get(target, property, receiver) as unknown;
      if (typeof value === 'function') {
        const boundValue: unknown = value.bind(target);
        return boundValue;
      }
      return value;
    },
  }) as RuntimeBridge;
}

void test('reference RuntimeSession voxel conversion facade methods remain typed and fail closed', () => {
  const request = voxelConversionPlanRequest();
  const referenceSession = createMockRuntimeSession();

  assert.throws(
    () => referenceSession.planVoxelConversion(request),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'not_initialized',
  );
  assert.throws(
    () => referenceSession.registerVoxelConversionSource(voxelConversionSourceRegistrationRequest()),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'not_initialized',
  );
  assert.throws(
    () => referenceSession.registerVoxelConversionMeshAsset(voxelConversionMeshAssetRegistrationRequest()),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'not_initialized',
  );

  referenceSession.initialize(sessionInput());
  assert.throws(
    () => referenceSession.registerVoxelConversionSource(voxelConversionSourceRegistrationRequest()),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
  assert.throws(
    () => referenceSession.registerVoxelConversionMeshAsset(voxelConversionMeshAssetRegistrationRequest()),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
  assert.throws(
    () => referenceSession.planVoxelConversion(request),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
  assert.throws(
    () =>
      referenceSession.previewVoxelConversion({
        planId: 'plan',
        expectedPlanHash: 'hash',
      }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
  assert.throws(
    () =>
      referenceSession.applyVoxelConversion({
        planId: 'plan',
        expectedPlanHash: 'hash',
        expectedPreviewHash: null,
      }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
  assert.throws(
    () =>
      referenceSession.exportVoxelConversionEvidence([
        {
          kind: 'plan',
          uri: 'asha://voxel-conversion/plan/plan',
          contentHash: 'fnv1a64:0000000000000000',
        },
      ]),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
  assert.throws(
    () =>
      referenceSession.readVoxelModelInfo({
        grid: 1,
        volumeAssetId: 'voxel/generated',
        includeMaterialCounts: true,
      }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
  assert.throws(
    () =>
      referenceSession.readVoxelModelWindow({
        grid: 1,
        volumeAssetId: 'voxel/generated',
        bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
        includeEmpty: false,
        materialFilter: [],
        maxSamples: 1,
      }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
  assert.throws(
    () =>
      referenceSession.exportVoxelVolumeAsset({
        grid: 1,
        volumeAssetId: 'voxel/generated',
        targetAssetId: 'voxel-volume/generated',
        label: null,
        createdBy: null,
        sourceTool: null,
        maxSparseRuns: 16,
        expectedSessionHash: null,
      }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
  assert.throws(
    () =>
      referenceSession.loadVoxelVolumeAsset({
        asset: {
          assetId: 'voxel-volume/generated',
          schemaVersion: 1,
          mediaType: 'application/vnd.asha.voxel-volume+json;version=1',
          grid: { origin: [0, 0, 0], cellSize: 1, coordinateSystem: 'y_up_right_handed' },
          bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
          representation: { kind: 'sparse_runs', sparseRuns: [] },
          materialPalette: [],
          provenance: [],
          authoring: { label: null, createdBy: null, sourceTool: null },
          validationDiagnostics: [],
          contentHashes: { canonicalJson: '', voxelData: '' },
        },
        targetGrid: 1,
        targetVolumeAssetId: 'voxel/generated',
        replaceExisting: true,
        includeMaterialCounts: true,
      }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
  assert.throws(
    () =>
      referenceSession.saveVoxelVolumeAsset({
        exportRequest: {
          grid: 1,
          volumeAssetId: 'voxel/generated',
          targetAssetId: 'voxel-volume/generated',
          label: null,
          createdBy: null,
          sourceTool: null,
          maxSparseRuns: 16,
          expectedSessionHash: null,
        },
        targetProjectBundle: 'asha-demo',
        targetAssetPath: 'assets/voxels/generated.avxl.json',
        representationKind: 'sparse_runs',
        expectedExistingCanonicalJsonHash: null,
        expectedCanonicalJsonHash: null,
        expectedVoxelDataHash: null,
      }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
});

void test('Rust-backed RuntimeSession delegates voxel conversion to the bridge authority surface', () => {
  const request = voxelConversionPlanRequest();
  const rustSession = createRuntimeSessionFacade({ bridge: createVoxelConversionBridge(), mode: 'rust' });
  rustSession.initialize(sessionInput());

  const registrationRequest = voxelConversionSourceRegistrationRequest();
  const registration = rustSession.registerVoxelConversionSource(registrationRequest);
  assert.equal(registration.source.assetId, 'mesh/quad');
  assert.equal(registration.registered, true);
  assert.deepEqual(registration.materialSlots, registrationRequest.materialSlots);

  const meshAssetRegistrationRequest = voxelConversionMeshAssetRegistrationRequest();
  const meshAssetRegistration = rustSession.registerVoxelConversionMeshAsset(meshAssetRegistrationRequest);
  assert.equal(meshAssetRegistration.source.assetId, 'mesh/quad');
  assert.equal(meshAssetRegistration.registered, true);
  assert.deepEqual(meshAssetRegistration.materialSlots, meshAssetRegistrationRequest.meshAsset.materialSlots);

  const metadata = rustSession.readVoxelConversionSourceMetadata({
    source: meshAssetRegistrationRequest.source,
  });
  assert.equal(metadata.registered, true);
  assert.equal(metadata.sourcePath, 'assets/mesh/quad.mesh.json');
  assert.equal(metadata.vertexCount, 3);
  assert.equal(metadata.triangleCount, 1);
  assert.equal(metadata.groups[0]?.materialSlot, 0);
  assert.deepEqual(metadata.groups[0]?.bounds?.max, [1, 1, 0]);
  assert.equal(metadata.materialSlots[0]?.sourceMaterialId, 'mat/a');
  assert.equal(metadata.latestPlanId, 'fnv1a64:plan');
  assert.deepEqual(metadata.latestPlanTransform, request.settings.transform);
  assert.equal(metadata.diagnostics.length, 0);

  const plan = rustSession.planVoxelConversion(request);
  assert.equal(plan.authorityVersion, 'svc-voxel-conversion.v0');
  assert.deepEqual(plan.source, request.source);

  const stalePreview = rustSession.previewVoxelConversion({
    planId: plan.planId,
    expectedPlanHash: 'fnv1a64:stale',
  });
  assert.equal(stalePreview.diagnostics[0]?.code, 'stale_authority_snapshot');

  const preview = rustSession.previewVoxelConversion({
    planId: plan.planId,
    expectedPlanHash: PLAN_HASH,
  });
  assert.equal(preview.outputHash, PREVIEW_HASH);
  assert.equal(preview.sampleVoxels[0]?.material, 3);

  const staleReceipt = rustSession.applyVoxelConversion({
    planId: plan.planId,
    expectedPlanHash: PLAN_HASH,
    expectedPreviewHash: 'fnv1a64:stale-preview',
  });
  assert.equal(staleReceipt.applied, false);
  assert.equal(staleReceipt.diagnostics[0]?.code, 'conversion_replay_mismatch');

  const receipt = rustSession.applyVoxelConversion({
    planId: plan.planId,
    expectedPlanHash: PLAN_HASH,
    expectedPreviewHash: preview.outputHash,
  });
  assert.equal(receipt.applied, true);
  assert.deepEqual(rustSession.exportVoxelConversionEvidence([...plan.evidence, ...preview.evidence, ...receipt.evidence]), [
    ...plan.evidence,
    ...preview.evidence,
    ...receipt.evidence,
  ]);
  const modelInfo = rustSession.readVoxelModelInfo({
    grid: 1,
    volumeAssetId: 'voxel/generated',
    includeMaterialCounts: true,
  });
  assert.equal(modelInfo.resident, true);
  assert.equal(modelInfo.voxelCount, 1);
  assert.deepEqual(modelInfo.materialCounts, [{ material: 3, voxelCount: 1 }]);
  assert.equal(modelInfo.source?.assetId, 'mesh/quad');

  const modelWindow = rustSession.readVoxelModelWindow({
    grid: 1,
    volumeAssetId: 'voxel/generated',
    bounds: { min: { x: 0, y: 0, z: 0 }, max: { x: 0, y: 0, z: 0 } },
    includeEmpty: false,
    materialFilter: [],
    maxSamples: 1,
  });
  assert.equal(modelWindow.resident, true);
  assert.equal(modelWindow.scannedVoxelCount, 1);
  assert.deepEqual(modelWindow.samples, [
    { coord: { x: 0, y: 0, z: 0 }, occupied: true, material: 3 },
  ]);

  const exportedAsset = rustSession.exportVoxelVolumeAsset({
    grid: 1,
    volumeAssetId: 'voxel/generated',
    targetAssetId: 'voxel-volume/generated',
    label: 'Generated voxel volume',
    createdBy: 'runtime-session-voxel-conversion-test',
    sourceTool: '@asha/runtime-bridge',
    maxSparseRuns: 16,
    expectedSessionHash: modelInfo.sessionHash,
  });
  assert.equal(exportedAsset.exported, true);
  assert.equal(exportedAsset.asset?.assetId, 'voxel-volume/generated');
  assert.equal(exportedAsset.asset?.representation.kind, 'sparse_runs');
  assert.deepEqual(exportedAsset.asset?.representation.sparseRuns, [
    { start: { x: 0, y: 0, z: 0 }, length: 1, material: 3 },
  ]);
  assert.deepEqual(exportedAsset.asset?.materialPalette, [
    {
      voxelMaterial: 3,
      paletteEntryId: 'voxel-material/surface-a',
      displayName: 'Surface A',
      materialAssetId: 'material/surface-a',
      materialCatalogBindingId: 'catalog-binding/surface-a',
    },
  ]);
  assert.equal(exportedAsset.canonicalJsonHash, exportedAsset.asset?.contentHashes.canonicalJson);
  assert.equal(exportedAsset.voxelDataHash, exportedAsset.asset?.contentHashes.voxelData);
  assert.match(exportedAsset.canonicalJson ?? '', /"assetId":"voxel-volume\/generated"/u);

  const loadedAsset = rustSession.loadVoxelVolumeAsset({
    asset: exportedAsset.asset!,
    targetGrid: 1,
    targetVolumeAssetId: 'voxel/generated',
    replaceExisting: true,
    includeMaterialCounts: true,
  });
  assert.equal(loadedAsset.loaded, true);
  assert.equal(loadedAsset.requestAssetId, 'voxel-volume/generated');
  assert.equal(loadedAsset.grid, 1);
  assert.equal(loadedAsset.voxelCount, 1);
  assert.deepEqual(loadedAsset.materialCounts, [{ material: 3, voxelCount: 1 }]);
  assert.equal(loadedAsset.canonicalJsonHash, exportedAsset.asset?.contentHashes.canonicalJson);
  assert.equal(loadedAsset.voxelDataHash, exportedAsset.asset?.contentHashes.voxelData);

  const savedAsset = rustSession.saveVoxelVolumeAsset({
    exportRequest: {
      grid: 1,
      volumeAssetId: 'voxel/generated',
      targetAssetId: 'voxel-volume/generated',
      label: 'Generated voxel volume',
      createdBy: 'runtime-session-voxel-conversion-test',
      sourceTool: '@asha/runtime-bridge',
      maxSparseRuns: 16,
      expectedSessionHash: modelInfo.sessionHash,
    },
    targetProjectBundle: 'asha-demo',
    targetAssetPath: 'assets/voxels/generated.avxl.json',
    representationKind: 'sparse_runs',
    expectedExistingCanonicalJsonHash: null,
    expectedCanonicalJsonHash: exportedAsset.asset?.contentHashes.canonicalJson ?? null,
    expectedVoxelDataHash: exportedAsset.asset?.contentHashes.voxelData ?? null,
  });
  assert.equal(savedAsset.saved, true);
  assert.equal(savedAsset.diff?.projectBundle, 'asha-demo');
  assert.equal(savedAsset.diff?.assetPath, 'assets/voxels/generated.avxl.json');
  assert.equal(savedAsset.diff?.operation, 'create');
  assert.equal(savedAsset.canonicalJsonHash, exportedAsset.asset?.contentHashes.canonicalJson);
  assert.equal(savedAsset.voxelDataHash, exportedAsset.asset?.contentHashes.voxelData);

  const missingModel = rustSession.readVoxelModelInfo({
    grid: 999,
    volumeAssetId: 'voxel/missing',
    includeMaterialCounts: true,
  });
  assert.equal(missingModel.resident, false);
  assert.equal(missingModel.diagnostics[0]?.code, 'voxel_conversion_unavailable');

  assert.throws(
    () => rustSession.exportVoxelConversionEvidence([
      { kind: 'diagnostics', uri: 'asha://voxel-conversion/diagnostics/missing', contentHash: 'fnv1a64:missing' },
    ]),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
});
