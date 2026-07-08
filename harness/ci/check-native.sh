#!/usr/bin/env bash
# Build the napi-rs native addon and verify it round-trips from TS (ADR 0006, #2250).
#
# OPT-IN: not part of check-all.sh — it needs the native toolchain + (first run)
# network to fetch napi crates. Run it where those are available.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CRATE_DIR="$REPO_ROOT/engine-rs/crates/bridge/native-bridge"
DEST="$REPO_ROOT/ts/packages/native-bridge/dist/native-bridge.node"

echo "==> Building native-bridge cdylib (release)"
( cd "$CRATE_DIR" && cargo build --release )

echo "==> Installing addon -> $DEST"
mkdir -p "$(dirname "$DEST")"
# cdylib is libnative_bridge.so on Linux / .dylib on macOS / native_bridge.dll on Windows.
ARTIFACT="$(find "$CRATE_DIR/target/release" -maxdepth 1 \
  \( -name 'libnative_bridge.so' -o -name 'libnative_bridge.dylib' -o -name 'native_bridge.dll' \) \
  | head -1)"
if [ -z "$ARTIFACT" ]; then
  echo "FAIL: no cdylib artifact found in $CRATE_DIR/target/release" >&2
  exit 1
fi
cp "$ARTIFACT" "$DEST"

echo "==> Building TS bridge packages used by the native smoke"
( cd "$REPO_ROOT/ts" && pnpm --filter @asha/native-bridge build && pnpm --filter @asha/runtime-bridge build )

echo "==> Native addon smoke (required exports, facade load, voxel conversion path)"
node --input-type=module -e "
import { strict as assert } from 'node:assert';
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
assert.deepEqual(a.loadWorldBundle(h, 1, 1, 1001), { loadedWorld: 1001, fatalCount: 0, totalCount: 0, blocksLoad: false });
assert.deepEqual(a.submitCommands(h, JSON.stringify([{ op: 'setVoxel', grid: 1, coord: { x: 0, y: 0, z: 0 }, value: { kind: 'solid', material: 1 } }])), { accepted: 1, rejected: 0, rejections: [] });
assert.equal(a.stepSimulation(h, 6), 2);    // tick 6 % 4 == 2, matches ReferenceBridge
assert.deepEqual(a.readRenderDiffs(h, 0), { ops: [] });
assert.deepEqual(a.saveCurrentWorld(h), { artifactsWritten: 3, compactedEdits: 0, retainedEdits: 0 });
assert.deepEqual(a.getCompositionStatus(h), { loadedWorld: 1001, fatalCount: 0, totalCount: 0, blocksLoad: false });

const bridge = createNativeRuntimeBridge('$DEST');
bridge.initializeEngine({ seed: 1 });
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
  source: registrationRequest.source,
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
      defaultVoxelMaterial: null,
    },
  },
};
const plan = bridge.planVoxelConversion(planRequest);
assert.equal(plan.authorityVersion, 'svc-voxel-conversion.v0');
assert.equal(plan.expectedSourceHash, 'sha256:check-native-registered-triangle');
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
console.log('Native addon smoke: OK');
"

echo "==> runtime-bridge facade tests (native parity test now runs, not skipped)"
( cd "$REPO_ROOT/ts" && pnpm --filter @asha/runtime-bridge test )
