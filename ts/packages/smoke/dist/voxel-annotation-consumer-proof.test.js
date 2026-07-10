import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { RuntimeBridgeError, createNativeRuntimeBridge, createRuntimeSessionFacade, } from '@asha/runtime-bridge';
const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../../../..');
const proofPath = resolve(repoRoot, 'harness/smoke-out/voxel-annotation-consumer-proof.json');
const TARGET_GRID = 2;
function gitValue(args) {
    return execFileSync('git', [...args], { cwd: repoRoot, encoding: 'utf8' }).trim();
}
function bootNativeSession(t) {
    let bridge;
    try {
        bridge = createNativeRuntimeBridge();
    }
    catch (error) {
        if (error instanceof RuntimeBridgeError && error.kind === 'native_unavailable') {
            t.skip('native addon not built; run harness/ci/check-native.sh for the voxel annotation proof');
            return null;
        }
        throw error;
    }
    const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
    session.initialize({
        sessionId: 'runtime-session.voxel-annotation.consumer-proof',
        seed: 5278,
        project: {
            gameId: 'asha-annotation-consumer-proof',
            workspaceId: 'workspace.local',
        },
        projectBundle: {
            bundleSchemaVersion: 1,
            protocolVersion: 1,
            sceneId: 5278,
        },
    });
    return session;
}
function createVoxelAsset(session) {
    const registration = session.registerVoxelConversionMeshAsset({
        source: {
            assetId: 'mesh/annotation-consumer-proof-quad',
            assetKind: 'mesh',
            assetVersion: 1,
            sourceHash: 'sha256:annotation-consumer-proof-quad',
            meshPrimitive: 'default',
        },
        meshAsset: {
            assetId: 'mesh/annotation-consumer-proof-quad',
            sourcePath: 'assets/meshes/annotation-consumer-proof-quad.mesh.json',
            positions: [[0, 0, 0], [2, 0, 0], [2, 1, 0], [0, 1, 0]],
            normals: [],
            indices: [0, 1, 2, 0, 2, 3],
            groups: [{ materialSlot: 0, start: 0, count: 6 }],
            materialSlots: [{ sourceMaterialSlot: 0, sourceMaterialId: 'material/surface-a' }],
        },
    });
    assert.equal(registration.registered, true, JSON.stringify(registration.diagnostics));
    const plan = session.planVoxelConversion({
        source: registration.source,
        target: {
            grid: TARGET_GRID,
            volumeAssetId: 'voxel/generated',
            origin: { x: 0, y: 0, z: 0 },
        },
        settings: {
            mode: 'surface',
            fitPolicy: 'contain',
            originPolicy: 'target_min',
            resolution: [4, 4, 1],
            voxelSize: 1,
            maxOutputVoxels: 32,
            transform: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
            materialMap: {
                entries: [{ sourceMaterialSlot: 0, sourceMaterialId: 'material/surface-a', voxelMaterial: 3 }],
                textureAssets: [],
                textureBindings: [],
                defaultVoxelMaterial: null,
            },
        },
    });
    assert.equal(plan.diagnostics.length, 0, JSON.stringify(plan.diagnostics));
    const preview = session.previewVoxelConversion({
        planId: plan.planId,
        expectedPlanHash: plan.planHash,
    });
    assert.equal(preview.diagnostics.length, 0, JSON.stringify(preview.diagnostics));
    const receipt = session.applyVoxelConversion({
        planId: plan.planId,
        expectedPlanHash: plan.planHash,
        expectedPreviewHash: preview.outputHash,
    });
    assert.equal(receipt.applied, true, JSON.stringify(receipt.diagnostics));
    const modelInfo = session.readVoxelModelInfo({
        grid: TARGET_GRID,
        volumeAssetId: 'voxel/generated',
        includeMaterialCounts: true,
    });
    assert.equal(modelInfo.resident, true);
    const exported = session.exportVoxelVolumeAsset({
        grid: TARGET_GRID,
        volumeAssetId: 'voxel/generated',
        targetAssetId: 'voxel-volume/annotation-consumer-proof',
        label: 'Voxel annotation consumer proof',
        createdBy: '@asha/smoke',
        sourceTool: '@asha/runtime-bridge',
        maxSparseRuns: 32,
        expectedSessionHash: modelInfo.sessionHash,
    });
    assert.equal(exported.exported, true, JSON.stringify(exported.diagnostics));
    assert.ok(exported.asset !== null);
    return exported.asset;
}
function firstRegionFromAsset(asset) {
    const run = asset.representation.sparseRuns[0];
    assert.ok(run, 'proof asset must contain at least one sparse run');
    const annotationRun = {
        start: run.start,
        length: run.length,
    };
    const region = {
        regionId: 'region/annotation-proof-room',
        label: 'Annotation proof room',
        kind: 'room',
        tags: ['consumer-proof'],
        parentRegionId: null,
        bounds: {
            min: run.start,
            max: { x: run.start.x + run.length - 1, y: run.start.y, z: run.start.z },
        },
        selection: { sparseRuns: [annotationRun] },
    };
    return { region, queryCell: run.start };
}
function annotationLayer(asset) {
    const { region, queryCell } = firstRegionFromAsset(asset);
    return {
        queryCell,
        layer: {
            layerId: 'voxel-annotation/annotation-consumer-proof',
            schemaVersion: 1,
            mediaType: 'application/vnd.asha.voxel-annotation+json;version=1',
            targetVoxelVolumeAssetId: asset.assetId,
            targetVoxelDataHash: asset.contentHashes.voxelData,
            targetBounds: asset.bounds,
            regions: [region],
            provenance: [{
                    kind: 'authored',
                    uri: 'asha://smoke/voxel-annotation-consumer-proof',
                    contentHash: 'fnv1a64:annotation-consumer-proof-source',
                }],
        },
    };
}
function validationRequest(input, targetVoxelVolumeAssetId, targetVoxelDataHash) {
    return {
        input,
        expectedTargetVoxelVolumeAssetId: targetVoxelVolumeAssetId,
        expectedTargetVoxelDataHash: targetVoxelDataHash,
        maxRegions: 16,
        maxSparseRunsPerRegion: 16,
        maxTotalAssignedCells: 32,
    };
}
void test('voxel annotation public consumer proof validates loads queries edits and exports', (t) => {
    const session = bootNativeSession(t);
    if (session === null)
        return;
    const asset = createVoxelAsset(session);
    const volumeLoad = session.loadVoxelVolumeAsset({
        asset,
        targetGrid: TARGET_GRID,
        targetVolumeAssetId: asset.assetId,
        replaceExisting: true,
        includeMaterialCounts: true,
    });
    assert.equal(volumeLoad.loaded, true, JSON.stringify(volumeLoad.diagnostics));
    assert.equal(volumeLoad.voxelDataHash, asset.contentHashes.voxelData);
    const { layer: draftLayer, queryCell } = annotationLayer(asset);
    const draftValidation = session.validateVoxelAnnotationLayer(validationRequest({ kind: 'draft', draft: draftLayer }, draftLayer.targetVoxelVolumeAssetId, draftLayer.targetVoxelDataHash));
    assert.equal(draftValidation.valid, true, JSON.stringify(draftValidation.diagnostics));
    assert.ok(draftValidation.normalizedLayer !== null);
    assert.match(draftValidation.canonicalJsonHash ?? '', /^fnv1a64:/);
    assert.match(draftValidation.membershipDataHash ?? '', /^fnv1a64:/);
    const layer = draftValidation.normalizedLayer;
    const validation = session.validateVoxelAnnotationLayer(validationRequest({ kind: 'finalized', layer }, layer.targetVoxelVolumeAssetId, layer.targetVoxelDataHash));
    assert.equal(validation.valid, true, JSON.stringify(validation.diagnostics));
    assert.equal(validation.regionCount, 1);
    assert.ok(validation.assignedCellCount >= 1);
    assert.match(validation.canonicalJsonHash ?? '', /^fnv1a64:/);
    assert.match(validation.membershipDataHash ?? '', /^fnv1a64:/);
    const quotaReport = session.validateVoxelAnnotationLayer({
        ...validationRequest({ kind: 'finalized', layer }, layer.targetVoxelVolumeAssetId, layer.targetVoxelDataHash),
        maxTotalAssignedCells: 0,
    });
    assert.equal(quotaReport.valid, false);
    assert.equal(quotaReport.diagnostics[0]?.code, 'quota_exceeded');
    const staleLoad = session.loadVoxelAnnotationLayer({
        layer,
        targetGrid: TARGET_GRID,
        replaceExisting: true,
        expectedSessionHash: 'fnv1a64:0000000000000000',
    });
    assert.equal(staleLoad.loaded, false);
    assert.equal(staleLoad.diagnostics[0]?.code, 'target_voxel_hash_mismatch');
    const load = session.loadVoxelAnnotationLayer({
        layer,
        targetGrid: TARGET_GRID,
        replaceExisting: true,
        expectedSessionHash: volumeLoad.sessionHash,
    });
    assert.equal(load.loaded, true, JSON.stringify(load.diagnostics));
    assert.ok(load.runtimeLayerId !== null);
    assert.match(load.layerHash ?? '', /^fnv1a64:/);
    const query = session.readVoxelAnnotationQuery({
        runtimeLayerId: load.runtimeLayerId,
        layerId: layer.layerId,
        mode: 'cell',
        cell: queryCell,
        bounds: null,
        regionId: null,
        maxRegions: 4,
        expectedLayerHash: load.layerHash,
    });
    assert.equal(query.diagnostics.length, 0, JSON.stringify(query.diagnostics));
    assert.equal(query.matchedRegions[0]?.regionId, 'region/annotation-proof-room');
    const staleEdit = session.applyVoxelAnnotationEdit({
        runtimeLayerId: load.runtimeLayerId,
        layerId: layer.layerId,
        expectedLayerHash: 'fnv1a64:0000000000000000',
        operation: 'set_label',
        regionId: 'region/annotation-proof-room',
        region: null,
        sparseRuns: [],
        tags: [],
        label: 'Edited annotation proof room',
        kind: null,
        parentRegionId: null,
    });
    assert.equal(staleEdit.edited, false);
    assert.equal(staleEdit.diagnostics[0]?.code, 'stale_layer_hash');
    const edit = session.applyVoxelAnnotationEdit({
        runtimeLayerId: load.runtimeLayerId,
        layerId: layer.layerId,
        expectedLayerHash: load.layerHash ?? '',
        operation: 'set_label',
        regionId: 'region/annotation-proof-room',
        region: null,
        sparseRuns: [],
        tags: [],
        label: 'Edited annotation proof room',
        kind: null,
        parentRegionId: null,
    });
    assert.equal(edit.edited, true, JSON.stringify(edit.diagnostics));
    assert.match(edit.layerHashAfter ?? '', /^fnv1a64:/);
    const exported = session.exportVoxelAnnotationLayer({
        runtimeLayerId: load.runtimeLayerId,
        layerId: layer.layerId,
        expectedLayerHash: edit.layerHashAfter ?? '',
        includeDiagnostics: true,
    });
    assert.equal(exported.exported, true, JSON.stringify(exported.diagnostics));
    assert.equal(exported.layer?.regions[0]?.label, 'Edited annotation proof room');
    assert.equal(exported.canonicalJsonHash, edit.layerHashAfter);
    assert.match(exported.membershipDataHash ?? '', /^fnv1a64:/);
    const proof = {
        schemaVersion: 1,
        project: 'asha',
        consumer: '@asha/smoke',
        publicImports: ['@asha/contracts', '@asha/runtime-bridge', '@asha/runtime-session'],
        engineCommit: gitValue(['rev-parse', 'HEAD']),
        engineRef: gitValue(['rev-parse', '--abbrev-ref', 'HEAD']),
        targetAssetId: asset.assetId,
        targetGrid: TARGET_GRID,
        targetVoxelDataHash: asset.contentHashes.voxelData,
        runtimeLayerId: load.runtimeLayerId,
        layerHashBeforeEdit: edit.layerHashBefore,
        layerHashAfterEdit: edit.layerHashAfter,
        canonicalJsonHash: exported.canonicalJsonHash,
        membershipDataHash: exported.membershipDataHash,
        queryMatchedRegions: query.matchedRegions.map((region) => region.regionId),
        diagnostics: {
            draft: draftValidation.diagnostics.map((diagnostic) => diagnostic.code),
            quota: quotaReport.diagnostics[0]?.code,
            staleLoad: staleLoad.diagnostics[0]?.code,
            staleEdit: staleEdit.diagnostics[0]?.code,
        },
    };
    mkdirSync(dirname(proofPath), { recursive: true });
    writeFileSync(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
    assert.deepEqual(proof.publicImports, [
        '@asha/contracts',
        '@asha/runtime-bridge',
        '@asha/runtime-session',
    ]);
    assert.match(proof.engineCommit, /^[0-9a-f]{40}$/u);
    assert.deepEqual(proof.queryMatchedRegions, ['region/annotation-proof-room']);
});
//# sourceMappingURL=voxel-annotation-consumer-proof.test.js.map