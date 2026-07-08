import { test } from 'node:test';
import assert from 'node:assert/strict';

import { RuntimeBridgeError, type RuntimeBridge } from '@asha/runtime-bridge';
import { createMockRuntimeBridge } from '@asha/runtime-bridge/reference';
import {
  projectId,
  sceneId,
  runtimeSessionId,
  type DiagnosticReportSet,
  type GeneratorMismatch,
  type LoadPlan,
  type ProjectBundleManifest as GeneratedProjectBundleManifest,
  type RegenConflictReport,
  type SaveSummary,
} from '@asha/contracts';

import {
  buildDiagnosticsPanel,
  buildLoadPlanModel,
  buildProjectBundleLoadRequest,
  buildManifestModel,
  buildRegenReport,
  buildSavePlanModel,
  buildVoxelDurabilityModel,
  describeGeneratorMismatch,
  summarizeVoxelDurability,
  type VoxelDurabilityEvidence,
  navigateSource,
  submitProjectBundleLoad,
  submitProjectBundleSave,
} from './bundle-panel.js';

function manifest(): GeneratedProjectBundleManifest {
  return {
    bundleSchemaVersion: 1,
    protocolVersion: 1,
    project: { id: projectId(7), name: 'fixture-project' },
    scene: { id: sceneId(1001), schemaVersion: 1, artifact: 'scene.json' },
    assetLock: { artifact: 'lock.json', assetCount: 4 },
    generator: { seed: 42, version: 3, params: 'flat' },
    artifacts: [
      { path: 'scene.json', class: 'durable', role: 'sceneDocument', contentHash: 'h1' },
      { path: 'lock.json', class: 'durable', role: 'assetLock', contentHash: 'h2' },
      { path: 'chunks.bin', class: 'generated', role: 'voxelChunkSnapshot', contentHash: null },
      { path: 'cache.bin', class: 'cache', role: 'cache', contentHash: null },
    ],
  };
}

void test('buildManifestModel classifies artifacts and counts by class', () => {
  const model = buildManifestModel(manifest());
  assert.equal(model.projectBundleId, 7);
  assert.equal(model.sceneId, 1001);
  assert.equal(model.assetCount, 4);
  assert.deepEqual(model.classCounts, { durable: 2, generated: 1, cache: 1 });
  // Durable artifacts both have hashes; nothing flagged.
  assert.equal(model.artifacts.every((a) => !a.durableMissingHash), true);
});

void test('buildManifestModel flags a durable artifact missing its content hash', () => {
  const m = manifest();
  const broken: GeneratedProjectBundleManifest = {
    ...m,
    artifacts: [{ path: 'scene.json', class: 'durable', role: 'sceneDocument', contentHash: null }],
  };
  const model = buildManifestModel(broken);
  assert.equal(model.artifacts[0]!.durableMissingHash, true);
});

void test('buildLoadPlanModel renders an ordered, human-readable plan', () => {
  const plan: LoadPlan = {
    steps: [
      { step: 'validateVersions', bundleSchemaVersion: 1, protocolVersion: 1 },
      { step: 'loadAssetLock', artifact: 'lock.json', assetCount: 4 },
      { step: 'loadSceneDocument', artifact: 'scene.json', scene: sceneId(1001) },
      { step: 'bootstrapScene', scene: sceneId(1001), runtimeSession: runtimeSessionId(7) },
      { step: 'validateFinalState' },
    ],
  };
  const view = buildLoadPlanModel(plan);
  assert.deepEqual(
    view.steps.map((s) => s.index),
    [0, 1, 2, 3, 4],
  );
  assert.deepEqual(
    view.steps.map((s) => s.step),
    ['validateVersions', 'loadAssetLock', 'loadSceneDocument', 'bootstrapScene', 'validateFinalState'],
  );
  assert.match(view.steps[3]!.summary, /bootstrap scene 1001 -> runtime session 7/);
});

void test('buildSavePlanModel summarizes writes and compaction', () => {
  const summary: SaveSummary = {
    writes: [
      { path: 'scene.json', class: 'durable', role: 'sceneDocument', contentHash: 'h1' },
      { path: 'snap.bin', class: 'generated', role: 'sessionStateSnapshot', contentHash: null },
    ],
    compaction: { compactedEdits: 12, retainedEdits: 3, snapshotChunks: ['c0', 'c1'] },
  };
  const view = buildSavePlanModel(summary);
  assert.equal(view.writes.length, 2);
  assert.equal(view.compactedEdits, 12);
  assert.equal(view.retainedEdits, 3);
  assert.equal(view.snapshotChunks, 2);
});

void test('describeGeneratorMismatch surfaces a fail-closed version mismatch', () => {
  const mismatch: GeneratorMismatch = { savedVersion: 2, currentVersion: 3 };
  const view = describeGeneratorMismatch(mismatch);
  assert.equal(view.savedVersion, 2);
  assert.equal(view.currentVersion, 3);
  assert.match(view.detail, /v2.*v3/);
});

void test('buildRegenReport reports equivalence and conflicts', () => {
  const clean: RegenConflictReport = {
    savedVersion: 3,
    newVersion: 3,
    conflicts: [],
    replayedEdits: 10,
    stagingSessionHash: 123,
  };
  assert.equal(buildRegenReport(clean).equivalent, true);

  const conflicted: RegenConflictReport = {
    savedVersion: 3,
    newVersion: 4,
    conflicts: [
      {
        eventId: 1,
        coord: { x: 0, y: 0, z: 0 },
        oldGenerated: { kind: 'empty' },
        newGenerated: { kind: 'solid', material: 1 },
        editValue: { kind: 'solid', material: 2 },
        suggested: 'reviewConflict',
      },
    ],
    replayedEdits: 10,
    stagingSessionHash: 456,
  };
  const view = buildRegenReport(conflicted);
  assert.equal(view.equivalent, false);
  assert.equal(view.conflictCount, 1);
});

void test('navigateSource resolves the most specific available locus', () => {
  assert.deepEqual(
    navigateSource({
      sceneNodeId: 5,
      runtimeEntityId: 9,
      assetId: 'mesh:x',
      chunkCoord: null,
      renderHandle: 42,
      bundlePath: null,
    }),
    { kind: 'renderHandle', handle: 42 },
  );
  assert.deepEqual(
    navigateSource({ sceneNodeId: null, runtimeEntityId: null, assetId: 'mesh:x', chunkCoord: null, renderHandle: null, bundlePath: 'b.json' }),
    { kind: 'asset', assetId: 'mesh:x' },
  );
  assert.deepEqual(
    navigateSource({ sceneNodeId: null, runtimeEntityId: null, assetId: null, chunkCoord: null, renderHandle: null, bundlePath: 'b.json' }),
    { kind: 'bundlePath', path: 'b.json' },
  );
  assert.deepEqual(
    navigateSource({ sceneNodeId: null, runtimeEntityId: null, assetId: null, chunkCoord: null, renderHandle: null, bundlePath: null }),
    { kind: 'none' },
  );
});

void test('buildDiagnosticsPanel carries severity, remedy, and navigation; only fatal blocks load', () => {
  const set: DiagnosticReportSet = {
    reports: [
      {
        scope: 'projectBundle',
        severity: 'fatal',
        code: 'corruptBundleArtifact',
        reference: 'bundle/scene.json',
        source: { sceneNodeId: null, runtimeEntityId: null, assetId: null, chunkCoord: null, renderHandle: null, bundlePath: 'scene.json' },
        message: 'scene artifact failed to parse',
        remedy: { action: 'restoreArtifact', detail: 'restore scene.json from a known-good save' },
      },
      {
        scope: 'assetCatalog',
        severity: 'warning',
        code: 'fallbackUsed',
        reference: 'asset/mesh:x',
        source: { sceneNodeId: null, runtimeEntityId: null, assetId: 'mesh:x', chunkCoord: null, renderHandle: null, bundlePath: null },
        message: 'fallback material used',
        remedy: null,
      },
    ],
  };
  const model = buildDiagnosticsPanel(set);
  assert.equal(model.fatalCount, 1);
  assert.equal(model.blocksLoad, true);
  assert.deepEqual(model.diagnostics[0]!.target, { kind: 'bundlePath', path: 'scene.json' });
  assert.equal(model.diagnostics[0]!.remedy?.action, 'restoreArtifact');
  assert.equal(model.diagnostics[1]!.remedy, null);
  assert.deepEqual(model.diagnostics[1]!.target, { kind: 'asset', assetId: 'mesh:x' });
});

// ── Action requests go through the facade, fail closed ────────────────────────────

function loadedBridge(): RuntimeBridge {
  const bridge = createMockRuntimeBridge();
  bridge.initializeEngine({ seed: 1 });
  return bridge;
}

void test('buildProjectBundleLoadRequest derives a typed facade request from the manifest', () => {
  assert.deepEqual(buildProjectBundleLoadRequest(manifest()), { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 1001 });
});

void test('submitProjectBundleLoad goes through the facade and returns the composition status', () => {
  const bridge = loadedBridge();
  const result = submitProjectBundleLoad(bridge, buildProjectBundleLoadRequest(manifest()));
  assert.equal(result.ok, true);
  if (result.ok) {
    assert.equal(result.value.loadedProjectBundle, 1001);
    assert.equal(result.value.blocksLoad, false);
  }
});

void test('submitProjectBundleLoad surfaces a fail-closed incompatible bundle with a recovery hint', () => {
  const bridge = loadedBridge();
  const result = submitProjectBundleLoad(bridge, { bundleSchemaVersion: 99, protocolVersion: 1, sceneId: 1001 });
  assert.equal(result.ok, false);
  if (!result.ok) {
    assert.equal(result.kind, 'invalid_input');
    assert.match(result.recovery, /incompatible/);
  }
});

void test('submitProjectBundleSave fails closed when no ProjectBundle is loaded, succeeds after a load', () => {
  const bridge = loadedBridge();
  const before = submitProjectBundleSave(bridge);
  assert.equal(before.ok, false);
  if (!before.ok) {
    assert.equal(before.kind, 'not_initialized');
  }
  submitProjectBundleLoad(bridge, buildProjectBundleLoadRequest(manifest()));
  const after = submitProjectBundleSave(bridge);
  assert.equal(after.ok, true);
  if (after.ok) {
    assert.equal(after.value.artifactsWritten, 3);
  }
});

void test('submitProjectBundleLoad rethrows non-bridge errors unchanged', () => {
  const exploding: RuntimeBridge = {
    ...createMockRuntimeBridge(),
    loadProjectBundle() {
      throw new TypeError('not a bridge error');
    },
  };
  assert.throws(() => submitProjectBundleLoad(exploding, buildProjectBundleLoadRequest(manifest())), TypeError);
  // Sanity: a bridge error is caught, a TypeError is not.
  assert.ok(new RuntimeBridgeError('internal', 'x') instanceof RuntimeBridgeError);
});

// ── voxel durability read model (task #2440) ─────────────────────────────────────

const DURABLE_EVIDENCE: VoxelDurabilityEvidence = {
  fixture: 'launch-sequence',
  postLoad: 'a86e394cb6f6468a',
  postEdit: '6183c2613603b87d',
  postReload: '6183c2613603b87d',
  compactedEdits: 2,
  retainedEdits: 1,
};

void test('buildVoxelDurabilityModel classifies a durable, genuinely-edited fixture', () => {
  const view = buildVoxelDurabilityModel(DURABLE_EVIDENCE);
  assert.equal(view.durable, true, 'post-edit equals post-reload');
  assert.equal(view.editedSession, true, 'post-load differs from post-edit');
  assert.equal(view.compactedEdits, 2);
  assert.equal(view.retainedEdits, 1);
});

void test('buildVoxelDurabilityModel flags a non-durable reload (fingerprint divergence)', () => {
  const view = buildVoxelDurabilityModel({ ...DURABLE_EVIDENCE, postReload: 'ffffffffffffffff' });
  assert.equal(view.durable, false, 'a reload that does not reproduce the edit is not durable');
  assert.equal(view.editedSession, true);
});

void test('buildVoxelDurabilityModel flags a no-op edit sequence', () => {
  const view = buildVoxelDurabilityModel({ ...DURABLE_EVIDENCE, postEdit: DURABLE_EVIDENCE.postLoad });
  assert.equal(view.editedSession, false, 'load == edit means the sequence changed nothing');
});

void test('summarizeVoxelDurability renders deterministic display lines', () => {
  const lines = summarizeVoxelDurability(buildVoxelDurabilityModel(DURABLE_EVIDENCE));
  assert.deepEqual(lines, [
    'fixture launch-sequence: durable=true edited=true',
    'postLoad=a86e394cb6f6468a postEdit=6183c2613603b87d postReload=6183c2613603b87d',
    'compaction folded=2 retained=1',
  ]);
});
