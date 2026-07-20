#!/usr/bin/env bash
# Build the napi-rs native addon and verify it round-trips from TS (ADR 0006, #2250).
#
# The comprehensive tier owns this gate. It needs the native toolchain and may
# need a first-run network fetch for napi crates.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CRATE_DIR="$REPO_ROOT/engine-rs/crates/bridge/native-bridge"
DEST="$REPO_ROOT/ts/packages/native-bridge/dist/native-bridge.node"

echo "==> Verifying atomic native-addon installation"
"$REPO_ROOT/harness/ci/test-install-native-addon.sh"

echo "==> Running native-bridge Rust tests"
python3 "$REPO_ROOT/harness/identity/execution.py" \
  --execution rust.native-bridge.lib \
  --attribution gate.native-browser-host

echo "==> Building native-bridge cdylib (release)"
( cd "$CRATE_DIR" && cargo build --release )

echo "==> Installing addon -> $DEST"
# cdylib is libnative_bridge.so on Linux / .dylib on macOS / native_bridge.dll on Windows.
ARTIFACT="$(find "$CRATE_DIR/target/release" -maxdepth 1 \
  \( -name 'libnative_bridge.so' -o -name 'libnative_bridge.dylib' -o -name 'native_bridge.dll' \) \
  | head -1)"
if [ -z "$ARTIFACT" ]; then
  echo "FAIL: no cdylib artifact found in $CRATE_DIR/target/release" >&2
  exit 1
fi
"$REPO_ROOT/harness/ci/install-native-addon.sh" "$ARTIFACT" "$DEST"

echo "==> Building TS bridge packages used by the native smoke"
( cd "$REPO_ROOT/ts" && pnpm --filter @asha/native-bridge build && pnpm --filter @asha/runtime-bridge build )

echo "==> Native addon smoke (required exports, facade load, voxel conversion path)"
node --input-type=module -e "
import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { createNativeRuntimeBridge } from '$REPO_ROOT/ts/packages/runtime-bridge/dist/index.js';
import { REQUIRED_NATIVE_ADDON_EXPORTS, loadNativeAddon } from '$REPO_ROOT/ts/packages/native-bridge/dist/index.js';
const require = createRequire('file://$DEST');
const rawAddon = require('$DEST');
const exportNames = Object.keys(rawAddon).sort();
assert.deepEqual(exportNames, [...REQUIRED_NATIVE_ADDON_EXPORTS].sort());
const a = loadNativeAddon('$DEST');
const h = a.initializeEngine(7);
assert.equal(typeof h, 'number');
assert.deepEqual(a.submitCommands(h, JSON.stringify([{ op: 'setVoxel', grid: 1, coord: { x: 0, y: 0, z: 0 }, value: { kind: 'solid', material: 1 } }])), { accepted: 1, rejected: 0, rejections: [] });
assert.deepEqual(a.stepSimulation(h, 6), { tick: 6, diffCount: 0 });
assert.equal(JSON.parse(a.applyTimeControlCommand(h, JSON.stringify({ operation: 'pause' }))).accepted, true);
assert.deepEqual(a.stepSimulation(h, 7), { tick: 6, diffCount: 0 });
assert.equal(JSON.parse(a.applyTimeControlCommand(h, JSON.stringify({ operation: 'stepTicks', ticks: 2 }))).after.authorityTick, 8);
assert.equal(JSON.parse(a.readTimeControlState(h)).authorityTick, 8);
assert.equal(JSON.parse(a.applyTimeControlCommand(h, JSON.stringify({ operation: 'resume' }))).accepted, true);
const renderFrame = JSON.parse(a.readRenderDiffs(h, 0));
assert.ok(renderFrame.ops.some((operation) => operation.op === 'replaceMeshPayload'));

const bridge = createNativeRuntimeBridge('$DEST');
bridge.initializeEngine({ seed: 1 });
const camera = bridge.createCamera({
  initialPose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 },
  projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
  viewport: { width: 1280, height: 720 },
});
assert.equal(camera.camera, 1);
assert.equal(camera.pose.position.length, 3);
assert.ok(Math.abs(camera.pose.position[0] - 0) < 0.00001);
assert.ok(Math.abs(camera.pose.position[1] - 1.6) < 0.00001);
assert.ok(Math.abs(camera.pose.position[2] - 0) < 0.00001);
assert.equal(camera.viewport.width, 1280);

const registrationRequest = {
  source: {
    assetId: 'mesh/check-native-registered-triangle',
    assetKind: 'mesh',
    assetVersion: 2,
    sourceHash: 'sha256:check-native-registered-triangle',
    meshPrimitive: 'default',
  },
  positions: [[0, 0, 0], [1, 0, 0], [0, 1, 0]],
  triangles: [{ indices: [0, 1, 2], sourceMaterialSlot: 0 }],
  materialSlots: [{ sourceMaterialSlot: 0, sourceMaterialId: 'material/surface-a' }],
};
const registration = bridge.registerVoxelConversionSource(registrationRequest);
assert.equal(registration.registered, true);
assert.equal(registration.diagnostics.length, 0);
assert.equal(registration.source.assetVersion, 2);
assert.equal(registration.materialSlots[0]?.sourceMaterialId, 'material/surface-a');
assert.equal(registration.evidence[0]?.kind, 'source_snapshot');

const meshAssetRegistrationRequest = {
  source: {
    assetId: 'mesh/check-native-project-quad',
    assetKind: 'mesh',
    assetVersion: 3,
    sourceHash: 'sha256:check-native-project-quad',
    meshPrimitive: 'default',
  },
  meshAsset: {
    assetId: 'mesh/check-native-project-quad',
    sourcePath: 'assets/mesh/check-native-project-quad.mesh.json',
    positions: [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]],
    normals: [],
    indices: [0, 1, 2, 0, 2, 3],
    groups: [{ materialSlot: 0, start: 0, count: 6 }],
    materialSlots: [{ sourceMaterialSlot: 0, sourceMaterialId: 'material/surface-a' }],
  },
};
const meshAssetRegistration = bridge.registerVoxelConversionMeshAsset(meshAssetRegistrationRequest);
assert.equal(meshAssetRegistration.registered, true);
assert.equal(meshAssetRegistration.source.assetVersion, 3);
assert.equal(meshAssetRegistration.materialSlots[0]?.sourceMaterialId, 'material/surface-a');
const meshMetadata = bridge.readVoxelConversionSourceMetadata({
  source: meshAssetRegistrationRequest.source,
});
assert.equal(meshMetadata.registered, true);
assert.equal(meshMetadata.sourcePath, 'assets/mesh/check-native-project-quad.mesh.json');
assert.equal(meshMetadata.vertexCount, 4);
assert.equal(meshMetadata.triangleCount, 2);
assert.equal(meshMetadata.groups[0]?.count, 6);
assert.equal(meshMetadata.materialSlots[0]?.sourceMaterialId, 'material/surface-a');

const referenceMeshImport = bridge.importVoxelConversionMeshSource({
  sourceAssetId: 'mesh/kenney-wall-a',
  assetVersion: 1,
  sourcePath: 'assets/reference/kenney-wall-a.glb',
  format: 'glb',
  sourceBytes: [...readFileSync('$REPO_ROOT/harness/fixtures/voxel-conversion/kenney-wall-a.glb')],
  meshPrimitive: null,
});
assert.equal(referenceMeshImport.imported, true);
assert.equal(referenceMeshImport.sourceByteCount, 3352);
assert.equal(referenceMeshImport.source.sourceHash, 'sha256:6fceda24c30d2c22694f232f03fe2115fb1a462046fbbf719a90eea10dc9af00');
assert.equal(referenceMeshImport.vertexCount, 48);
assert.equal(referenceMeshImport.triangleCount, 12);
assert.equal(referenceMeshImport.groups.length, 2);
assert.equal(referenceMeshImport.materialSlots.length, 2);
assert.deepEqual(referenceMeshImport.sourceBounds, { min: [-0.5, 0, -0.5], max: [0.5, 1, 0.5] });
const referencePlan = bridge.planVoxelConversion({
  source: referenceMeshImport.source,
  target: { grid: 2, volumeAssetId: 'voxel/generated', origin: { x: 0, y: 0, z: 0 } },
  settings: {
    mode: 'surface',
    fitPolicy: 'contain',
    originPolicy: 'target_min',
    resolution: [8, 8, 8],
    voxelSize: 0.25,
    maxOutputVoxels: 512,
    transform: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
    materialMap: {
      entries: referenceMeshImport.materialSlots.map((slot, index) => ({
        sourceMaterialSlot: slot.sourceMaterialSlot,
        sourceMaterialId: slot.sourceMaterialId,
        voxelMaterial: index + 1,
      })),
      textureAssets: [],
      textureBindings: [],
      defaultVoxelMaterial: null,
    },
  },
});
assert.equal(referencePlan.diagnostics.length, 0);
const referencePreview = bridge.previewVoxelConversion({
  planId: referencePlan.planId,
  expectedPlanHash: referencePlan.planHash,
});
assert.ok(referencePreview.outputVoxelCount > 0);
const referenceApply = bridge.applyVoxelConversion({
  planId: referencePlan.planId,
  expectedPlanHash: referencePlan.planHash,
  expectedPreviewHash: referencePreview.outputHash,
});
assert.equal(referenceApply.applied, true);
assert.ok(referenceApply.outputVoxelCount > 0);
const referenceMetadata = bridge.readVoxelConversionSourceMetadata({ source: referenceMeshImport.source });
assert.equal(referenceMetadata.vertexCount, 48);
assert.equal(referenceMetadata.triangleCount, 12);
assert.equal(referenceMetadata.groups.length, 2);
const referenceModel = bridge.readVoxelModelInfo({
  grid: 2,
  volumeAssetId: 'voxel/generated',
  includeMaterialCounts: true,
});
assert.equal(referenceModel.resident, true);
assert.equal(referenceModel.voxelCount, referenceApply.outputVoxelCount);

const rejectedRegistration = bridge.registerVoxelConversionSource({
  ...registrationRequest,
  source: {
    ...registrationRequest.source,
    assetId: 'mesh/check-native-missing-geometry',
    sourceHash: 'sha256:check-native-missing-geometry',
  },
  positions: [],
});
assert.equal(rejectedRegistration.registered, false);
assert.equal(rejectedRegistration.diagnostics[0]?.code, 'unsupported_source_asset');

const planRequest = {
  source: meshAssetRegistrationRequest.source,
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
      entries: [{
        sourceMaterialSlot: 0,
        sourceMaterialId: 'material/surface-a',
        voxelMaterial: 3,
      }],
      textureAssets: [],
      textureBindings: [],
      defaultVoxelMaterial: null,
    },
  },
};
const plan = bridge.planVoxelConversion(planRequest);
assert.equal(plan.authorityVersion, 'svc-voxel-conversion.v0');
assert.equal(plan.expectedSourceHash, 'sha256:check-native-project-quad');
assert.equal(plan.diagnostics.length, 0);
assert.match(plan.planHash, /^fnv1a64:[0-9a-f]{16}$/u);

const stalePreview = bridge.previewVoxelConversion({
  planId: plan.planId,
  expectedPlanHash: 'fnv1a64:0000000000000000',
});
assert.equal(stalePreview.diagnostics[0]?.code, 'stale_authority_snapshot');

const preview = bridge.previewVoxelConversion({
  planId: plan.planId,
  expectedPlanHash: plan.planHash,
});
assert.equal(preview.diagnostics.length, 0);
assert.ok(preview.outputVoxelCount > 0);
assert.match(preview.outputHash, /^fnv1a64:[0-9a-f]{16}$/u);

const receipt = bridge.applyVoxelConversion({
  planId: plan.planId,
  expectedPlanHash: plan.planHash,
  expectedPreviewHash: preview.outputHash,
});
assert.equal(receipt.applied, true);
assert.equal(receipt.outputHash, preview.outputHash);
assert.equal(receipt.diagnostics.length, 0);

const constrainedCamera = bridge.applyCollisionConstrainedCameraInput({
  camera: camera.camera,
  grid: 1,
  movementMode: 'grounded',
  input: {
    moveForward: 1,
    moveRight: 0,
    moveUp: 0,
    yawDeltaDegrees: 0,
    pitchDeltaDegrees: 0,
    dtSeconds: 1 / 60,
    moveSpeedUnitsPerSecond: 3,
  },
  tick: 1,
  shape: { halfExtents: [0.2, 0.2, 0.2] },
  policy: { mode: 'axis_separable_slide', maxIterations: 3 },
});
assert.equal(constrainedCamera.camera, camera.camera);
assert.equal(constrainedCamera.collision.grid, 1);
assert.match(constrainedCamera.movementHash, /^fnv1a64:[0-9a-f]{16}$/u);

const exportedEvidence = bridge.exportVoxelConversionEvidence([
  ...plan.evidence,
  ...preview.evidence,
  ...receipt.evidence,
]);
assert.ok(exportedEvidence.length >= 3);
const modelInfo = bridge.readVoxelModelInfo({
  grid: 1,
  volumeAssetId: 'voxel/generated',
  includeMaterialCounts: true,
});
const exportedAsset = bridge.exportVoxelVolumeAsset({
  grid: 1,
  volumeAssetId: 'voxel/generated',
  targetAssetId: 'voxel-volume/check-native-export',
  label: 'Check native export',
  createdBy: 'harness/ci/check-native.sh',
  sourceTool: '@asha/runtime-bridge',
  maxSparseRuns: 16,
  expectedSessionHash: modelInfo.sessionHash,
});
assert.equal(exportedAsset.exported, true);
assert.equal(exportedAsset.asset.assetId, 'voxel-volume/check-native-export');
assert.match(exportedAsset.canonicalJsonHash, /^fnv1a64:[0-9a-f]{16}$/u);
const savedAsset = bridge.saveVoxelVolumeAsset({
  exportRequest: {
    grid: 1,
    volumeAssetId: 'voxel/generated',
    targetAssetId: 'voxel-volume/check-native-export',
    label: 'Check native export',
    createdBy: 'harness/ci/check-native.sh',
    sourceTool: '@asha/runtime-bridge',
    maxSparseRuns: 16,
    expectedSessionHash: modelInfo.sessionHash,
  },
  targetProjectBundle: 'asha-demo',
  targetAssetPath: 'assets/voxels/check-native-export.avxl.json',
  representationKind: 'sparse_runs',
  expectedExistingCanonicalJsonHash: null,
  expectedCanonicalJsonHash: exportedAsset.canonicalJsonHash,
  expectedVoxelDataHash: exportedAsset.voxelDataHash,
});
assert.equal(savedAsset.saved, true);
assert.equal(savedAsset.diff.assetPath, 'assets/voxels/check-native-export.avxl.json');
assert.equal(savedAsset.canonicalJsonHash, exportedAsset.canonicalJsonHash);
const unloadedAsset = bridge.unloadVoxelVolumeAsset({
  grid: 1,
  volumeAssetId: 'voxel/generated',
  expectedSessionHash: modelInfo.sessionHash,
});
assert.equal(unloadedAsset.unloaded, true);
assert.equal(unloadedAsset.removedVoxelCount, modelInfo.voxelCount);
const unloadedModelInfo = bridge.readVoxelModelInfo({
  grid: 1,
  volumeAssetId: 'voxel/generated',
  includeMaterialCounts: true,
});
assert.equal(unloadedModelInfo.resident, false);
const loadedAsset = bridge.loadVoxelVolumeAsset({
  asset: exportedAsset.asset,
  targetGrid: 1,
  targetVolumeAssetId: 'voxel/generated',
  replaceExisting: true,
  includeMaterialCounts: true,
});
assert.equal(loadedAsset.loaded, true);
assert.equal(loadedAsset.requestAssetId, 'voxel-volume/check-native-export');
assert.equal(loadedAsset.voxelCount, modelInfo.voxelCount);
const reloadedModelInfo = bridge.readVoxelModelInfo({
  grid: 1,
  volumeAssetId: 'voxel/generated',
  includeMaterialCounts: true,
});
assert.equal(reloadedModelInfo.resident, true);
console.log('Native addon smoke: OK');
"

echo "==> runtime-bridge facade tests (native parity test now runs, not skipped)"
python3 "$REPO_ROOT/harness/identity/execution.py" \
  --execution ts.native-runtime-bridge.tests \
  --attribution gate.native-browser-host

echo "==> native browser-host sustained lifecycle regression"
python3 "$REPO_ROOT/harness/identity/execution.py" \
  --execution ts.native-browser-host.tests \
  --attribution gate.native-browser-host

echo "==> public RuntimeSession non-default-grid voxel annotation provider regression"
( cd "$REPO_ROOT/ts" && pnpm --filter @asha/smoke test:voxel-annotation-provider )

echo "==> public RuntimeSession exhaustive voxel command provider regression"
( cd "$REPO_ROOT/ts" && pnpm --filter @asha/smoke test:voxel-command-provider )
