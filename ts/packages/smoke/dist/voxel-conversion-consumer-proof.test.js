import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { COMMAND_CATALOG, COMMAND_IDS, requireCatalogCommand, } from '@asha/command-registry';
import { RuntimeBridgeError } from '@asha/runtime-bridge';
import { createMockRuntimeSession } from '@asha/runtime-bridge/reference';
const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../../../..');
const fixturePath = resolve(repoRoot, 'harness/fixtures/voxel-conversion/studio-consumer-proof.json');
function readJson(path) {
    return JSON.parse(readFileSync(path, 'utf8'));
}
function readFixture() {
    return readJson(fixturePath);
}
function sessionInput() {
    return {
        sessionId: 'runtime-session.voxel-conversion.consumer-proof',
        seed: 17,
        project: {
            gameId: 'asha-studio-consumer-proof',
            workspaceId: 'workspace.local',
        },
        projectBundle: {
            bundleSchemaVersion: 1,
            protocolVersion: 1,
            sceneId: 42,
        },
    };
}
void test('voxel conversion consumer proof uses public roots and deterministic fixtures', () => {
    const fixture = readFixture();
    assert.deepEqual(fixture.publicImports, [
        '@asha/contracts',
        '@asha/runtime-bridge',
        '@asha/runtime-bridge/reference',
        '@asha/command-registry',
    ]);
    assert.deepEqual(fixture.commandIds, [
        'voxel_conversion.plan',
        'voxel_conversion.preview',
        'voxel_conversion.apply',
        'voxel_conversion.export_evidence',
    ]);
    for (const id of fixture.commandIds) {
        assert.equal(COMMAND_IDS.includes(id), true, id);
    }
    const planCommand = requireCatalogCommand('voxel_conversion.plan', COMMAND_CATALOG);
    assert.deepEqual(planCommand.inputContractRefs.map((ref) => ref.exportName), ['VoxelConversionPlanRequest']);
    assert.deepEqual(planCommand.outputContractRefs.map((ref) => ref.exportName), ['VoxelConversionPlan']);
    assert.deepEqual(planCommand.runtimeRequirements, [{ kind: 'runtime_session_facade_method', method: 'planVoxelConversion' }]);
    const applyCommand = requireCatalogCommand('voxel_conversion.apply', COMMAND_CATALOG);
    assert.equal(applyCommand.operationClass, 'authority_mutating');
    assert.equal(applyCommand.agentExposureKind, 'authority_mutating');
    assert.deepEqual(applyCommand.runtimeRequirements, [{ kind: 'runtime_session_facade_method', method: 'applyVoxelConversion' }]);
    const planRequest = fixture.planRequest;
    const plan = fixture.plan;
    const preview = fixture.preview;
    const receipt = fixture.receipt;
    const evidence = fixture.evidenceExport;
    const authorityFixture = readJson(resolve(repoRoot, fixture.rustAuthorityFixture));
    assert.deepEqual({
        planRequest: fixture.planRequest,
        plan: fixture.plan,
        previewRequest: fixture.previewRequest,
        preview: fixture.preview,
        applyRequest: fixture.applyRequest,
        receipt: fixture.receipt,
        evidenceExport: fixture.evidenceExport,
    }, {
        planRequest: authorityFixture.planRequest,
        plan: authorityFixture.plan,
        previewRequest: authorityFixture.previewRequest,
        preview: authorityFixture.preview,
        applyRequest: authorityFixture.applyRequest,
        receipt: authorityFixture.receipt,
        evidenceExport: authorityFixture.evidenceExport,
    });
    assert.equal(planRequest.source.assetKind, 'mesh');
    assert.equal(plan.authorityVersion, authorityFixture.authorityVersion);
    assert.equal(planRequest.source.assetId, authorityFixture.sourceAssetId);
    assert.equal(plan.estimatedOutputVoxels, 4);
    assert.deepEqual([...new Set(preview.sampleVoxels.map((voxel) => voxel.material))].sort(), [3, 5]);
    assert.equal(receipt.applied, true);
    assert.deepEqual(evidence.map((ref) => ref.kind), ['plan', 'preview', 'apply_receipt']);
    assert.match(plan.planId, /^fnv1a64:/);
    assert.match(plan.settingsHash, /^fnv1a64:/);
    assert.match(plan.planHash, /^fnv1a64:/);
    assert.equal(fixture.previewRequest.expectedPlanHash, plan.planHash);
    assert.equal(fixture.previewRequest.expectedPlanHash, authorityFixture.previewRequest.expectedPlanHash);
    assert.equal(fixture.applyRequest.expectedPlanHash, plan.planHash);
    assert.equal(fixture.applyRequest.expectedPreviewHash, authorityFixture.preview.outputHash);
    const diagnosticCodes = fixture.diagnosticCases.map((diagnostic) => diagnostic.code);
    assert.deepEqual(diagnosticCodes, [
        'unsupported_source_asset',
        'invalid_material_map',
        'output_limit_exceeded',
        'source_hash_mismatch',
        'stale_authority_snapshot',
        'conversion_replay_mismatch',
    ]);
    const rustGolden = readFileSync(resolve(repoRoot, fixture.rustAuthorityGolden), 'utf8');
    assert.match(rustGolden, /quad\.surface\.materials=3,5/);
    assert.match(rustGolden, /cube\.solid\.voxels=8/);
    assert.match(rustGolden, /oversized\.code=output_limit_exceeded/);
    assert.match(rustGolden, /stale\.code=source_hash_mismatch/);
});
void test('voxel conversion consumer proof sees RuntimeSession fail closed without mocked success', () => {
    const fixture = readFixture();
    const session = createMockRuntimeSession();
    assert.throws(() => session.planVoxelConversion(fixture.planRequest), (error) => error instanceof RuntimeBridgeError && error.kind === 'not_initialized');
    session.initialize(sessionInput());
    assert.throws(() => session.planVoxelConversion(fixture.planRequest), (error) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented');
    assert.throws(() => session.previewVoxelConversion(fixture.previewRequest), (error) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented');
    assert.throws(() => session.applyVoxelConversion(fixture.applyRequest), (error) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented');
    assert.throws(() => session.exportVoxelConversionEvidence(fixture.evidenceExport), (error) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented');
});
//# sourceMappingURL=voxel-conversion-consumer-proof.test.js.map