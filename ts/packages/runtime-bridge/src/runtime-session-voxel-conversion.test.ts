import { test } from 'node:test';
import assert from 'node:assert/strict';

import type {
  VoxelConversionApplyRequest,
  VoxelConversionEvidenceRef,
  VoxelConversionPlan,
  VoxelConversionPlanRequest,
  VoxelConversionPreview,
  VoxelConversionPreviewRequest,
  VoxelConversionReceipt,
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
    projectBundle: {
      bundleSchemaVersion: 1,
      protocolVersion: 1,
      sceneId: 42,
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
        defaultVoxelMaterial: 3,
      },
    },
  };
}

const PLAN_HASH = 'fnv1a64:plan-hash';
const PREVIEW_HASH = 'fnv1a64:preview-hash';

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

  referenceSession.initialize(sessionInput());
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
});

void test('Rust-backed RuntimeSession delegates voxel conversion to the bridge authority surface', () => {
  const request = voxelConversionPlanRequest();
  const rustSession = createRuntimeSessionFacade({ bridge: createVoxelConversionBridge(), mode: 'rust' });
  rustSession.initialize(sessionInput());

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
  assert.throws(
    () => rustSession.exportVoxelConversionEvidence([
      { kind: 'diagnostics', uri: 'asha://voxel-conversion/diagnostics/missing', contentHash: 'fnv1a64:missing' },
    ]),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
});
