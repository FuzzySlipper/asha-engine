// Facade conformance + mock smoke (task #2250).
//
// (1) Conformance: the hand-written facade exposes EXACTLY the manifest operations.
// (2) Mock smoke: the default mock implements the facade with typed, classified
//     errors and deterministic behaviour matching the Rust ReferenceBridge.
// (3) Native unavailable: the native factory throws a classified bridge error when
//     the addon is not built (the expected state in offline CI).
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';
const MODEL_MATERIAL_PREVIEW_REQUEST = {
    catalogEntry: {
        id: 'material.copper',
        kind: 'material',
        version: 1,
        hash: 'sha256-material-copper',
        sourcePath: null,
        label: 'Copper',
        dependencies: [],
        material: {
            render: { color: { r: 0.8, g: 0.4, b: 0.2, a: 1 }, texture: null, roughness: 0.6, emissive: 0, uvStrategy: 'flat' },
            collision: { solid: true, collidable: true, occludes: true, structuralClass: 'solid' },
        },
    },
    meshAsset: {
        asset: 'mesh.preview-cube',
        payload: {
            layout: { vertexCount: 8, indexCount: 36, indexWidth: 'u32', attributes: [{ name: 'position', components: 3, kind: 'f32' }] },
            groups: [{ materialSlot: 0, start: 0, count: 36 }],
            bounds: { min: [-0.5, -0.5, -0.5], max: [0.5, 0.5, 0.5] },
            source: { kind: 'inline', positions: [], normals: [], indices: [] },
            provenance: 'staticAsset',
        },
        materialSlots: [{ slot: 0, material: 'material.copper' }],
        collision: { kind: 'aabbFallback' },
    },
    instanceHandle: 7001,
};
import { MANIFEST_OPERATIONS, MockRuntimeBridge, RuntimeBridgeError, createMockRuntimeBridge, createNativeRuntimeBridge, createReferenceGameRuntimeLauncher, frameCursor, } from './index.js';
test('facade exposes exactly the manifest operations (conformance)', () => {
    const bridge = createMockRuntimeBridge();
    const expected = MANIFEST_OPERATIONS.map((o) => o.facadeMethod).sort();
    const actual = MANIFEST_OPERATIONS.map((o) => o.facadeMethod)
        .filter((m) => typeof bridge[m] === 'function')
        .sort();
    assert.deepEqual(actual, expected, 'every manifest op must be a facade method');
    // No extra public methods beyond the manifest on the mock prototype.
    const proto = Object.getOwnPropertyNames(MockRuntimeBridge.prototype).filter((n) => n !== 'constructor');
    const known = new Set(MANIFEST_OPERATIONS.map((o) => o.facadeMethod));
    assert.deepEqual(proto.filter((n) => !known.has(n)), [], 'mock must not expose methods outside the manifest');
});
test('game runtime launcher public DTOs compile as package-root consumer fixtures', () => {
    const compatibility = {
        contractsPackageVersion: '0.1.0',
        runtimeBridgePackageVersion: '0.1.0',
        devtoolsProtocolVersion: 'devtools-protocol.v0',
        publishArtifactVersion: 'publish-artifact.v0',
    };
    const resourceProfile = {
        profileId: 'demo.reference.resources.v1',
        runtimeEntry: 'harness/conformance/fixtures/minimal-world.json',
        worldBundleId: 'world.minimal',
        resourceManifestHash: 'sha256-resource-profile',
        estimatedBytes: 4096,
    };
    const config = {
        gameId: 'asha-demo',
        workspaceId: 'workspace.local',
        runtimeEntry: resourceProfile.runtimeEntry,
        compatibility,
        resourceProfile,
        world: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 7 },
        startedAtIso: '2026-06-28T00:00:00.000Z',
    };
    const runtimeProfile = {
        profileId: 'reference.launcher.v1',
        runtimeMode: 'reference',
        launcherName: 'reference-game-runtime-launcher',
        bridgeCompatibility: compatibility,
        nonClaims: ['not_native_runtime', 'not_hardware_gpu', 'not_performance_evidence', 'not_publish_artifact'],
    };
    const projection = {
        sequenceId: 0,
        worldHash: 'world:minimal:0',
        authorityHash: 'authority:minimal:0',
        loadedWorld: config.world.sceneId,
        fatalCount: 0,
        totalDiagnosticCount: 0,
        evidenceRefs: [{ kind: 'projection', id: 'projection:0', sequenceId: 0 }],
    };
    const launch = {
        status: 'launched',
        identity: {
            gameId: config.gameId,
            workspaceId: config.workspaceId,
            runtimeMode: 'reference',
            runtimeEntry: config.runtimeEntry,
            startedAtIso: config.startedAtIso ?? '2026-06-28T00:00:00.000Z',
            compatibility,
            nonClaims: runtimeProfile.nonClaims,
        },
        runtimeProfile,
        resourceProfile,
        projection,
        diagnostics: [],
        evidenceRefs: projection.evidenceRefs,
    };
    const commandProposal = {
        sequenceId: 1,
        status: 'accepted',
        batch: { commands: [] },
        result: { accepted: 0, rejected: 0, rejections: [] },
        authorityHashBefore: launch.projection.authorityHash,
        authorityHashAfter: 'authority:minimal:1',
        diagnostics: [],
        evidenceRefs: [{ kind: 'replay', id: 'replay:1', sequenceId: 1 }],
    };
    assert.equal(launch.identity.runtimeMode, 'reference');
    assert.equal(commandProposal.status, 'accepted');
});
test('reference game runtime launcher launches fixture and advances command projection', async () => {
    const launcher = createReferenceGameRuntimeLauncher();
    const config = {
        gameId: 'asha-demo',
        workspaceId: 'workspace.local',
        runtimeEntry: 'harness/conformance/fixtures/minimal-world.json',
        compatibility: {
            contractsPackageVersion: '0.1.0',
            runtimeBridgePackageVersion: '0.1.0',
            devtoolsProtocolVersion: 'devtools-protocol.v0',
            publishArtifactVersion: 'publish-artifact.v0',
        },
        resourceProfile: {
            profileId: 'demo.reference.resources.v1',
            runtimeEntry: 'harness/conformance/fixtures/minimal-world.json',
            worldBundleId: 'world.minimal',
            resourceManifestHash: 'sha256-resource-profile',
        },
        world: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 7 },
        startedAtIso: '2026-06-28T00:00:00.000Z',
    };
    const session = await launcher.launch(config);
    assert.equal(launcher.mode, 'reference');
    assert.equal(session.identity.runtimeMode, 'reference');
    assert.ok(session.identity.nonClaims.includes('not_native_runtime'));
    assert.ok(!session.identity.nonClaims.includes('not_publish_artifact') || session.identity.runtimeMode === 'reference');
    const before = await session.pullProjection();
    const command = {
        op: 'setVoxel',
        grid: 1,
        coord: { x: 0, y: 0, z: 0 },
        value: { kind: 'solid', material: 1 },
    };
    const receipt = await session.proposeCommands({ commands: [command] });
    const after = await session.pullProjection();
    assert.equal(receipt.status, 'accepted');
    assert.equal(receipt.result?.accepted, 1);
    assert.equal(receipt.authorityHashBefore, before.authorityHash);
    assert.equal(receipt.authorityHashAfter, after.authorityHash);
    assert.notEqual(after.authorityHash, before.authorityHash);
    const rejected = await session.proposeCommands({
        commands: [{
                op: 'setVoxel',
                grid: 1,
                coord: { x: 0, y: 0, z: 0 },
                value: { kind: 'solid', material: 999 },
            }],
    });
    const afterRejected = await session.pullProjection();
    assert.equal(rejected.status, 'rejected');
    assert.equal(rejected.result?.accepted, 0);
    assert.equal(rejected.result?.rejected, 1);
    assert.equal(rejected.authorityHashBefore, after.authorityHash);
    assert.equal(rejected.authorityHashAfter, after.authorityHash);
    assert.equal(afterRejected.authorityHash, after.authorityHash);
    const telemetry = await session.pullTelemetry();
    assert.equal(telemetry.runtimeMode, 'reference');
    assert.equal(telemetry.acceptedCommandCount, 1);
    assert.equal(telemetry.rejectedCommandCount, 1);
    const evidence = await session.exportEvidence({ evidenceId: 'evidence:reference-launch' });
    assert.equal(evidence.sequenceId, afterRejected.sequenceId);
    assert.ok(evidence.nonClaims.includes('not_hardware_gpu'));
    await session.shutdown();
});
test('reference game runtime launcher fails closed on unsupported world bundle', async () => {
    const launcher = createReferenceGameRuntimeLauncher();
    await assert.rejects(() => launcher.launch({
        gameId: 'asha-demo',
        workspaceId: 'workspace.local',
        runtimeEntry: 'harness/conformance/fixtures/minimal-world.json',
        compatibility: {
            contractsPackageVersion: '0.1.0',
            runtimeBridgePackageVersion: '0.1.0',
        },
        resourceProfile: {
            profileId: 'demo.reference.resources.v1',
            runtimeEntry: 'harness/conformance/fixtures/minimal-world.json',
            worldBundleId: 'world.minimal',
        },
        world: { bundleSchemaVersion: 99, protocolVersion: 1, sceneId: 7 },
    }), (e) => e instanceof RuntimeBridgeError && e.kind === 'invalid_input');
});
test('manifest exposes public camera view operations', () => {
    const cameraOps = MANIFEST_OPERATIONS.filter((op) => op.facadeMethod.includes('Camera'));
    assert.deepEqual(cameraOps.map((op) => [op.manifestName, op.facadeMethod, op.surface]), [
        ['apply_collision_constrained_camera_input', 'applyCollisionConstrainedCameraInput', 'stable'],
        ['create_camera', 'createCamera', 'stable'],
        ['apply_first_person_camera_input', 'applyFirstPersonCameraInput', 'stable'],
        ['read_camera_projection', 'readCameraProjection', 'stable'],
    ]);
});
test('mock: init then step is deterministic', () => {
    const bridge = createMockRuntimeBridge();
    const handle = bridge.initializeEngine({ seed: 7 });
    assert.equal(handle, 7);
    assert.deepEqual(bridge.stepSimulation({ tick: 6 }), { tick: 6, diffCount: 2 });
});
test('mock: camera view operations produce deterministic public evidence', () => {
    const bridge = createMockRuntimeBridge();
    bridge.initializeEngine({ seed: 1 });
    const create = {
        initialPose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 },
        projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
        viewport: { width: 1280, height: 720 },
    };
    const created = bridge.createCamera(create);
    assert.equal(created.camera, 1);
    assert.deepEqual(created.pose, create.initialPose);
    const input = {
        camera: created.camera,
        tick: 1,
        input: {
            moveForward: 1,
            moveRight: 0,
            moveUp: 0,
            yawDeltaDegrees: 15,
            pitchDeltaDegrees: -5,
            dtSeconds: 1 / 60,
            moveSpeedUnitsPerSecond: 3,
        },
    };
    const moved = bridge.applyFirstPersonCameraInput(input);
    assert.equal(moved.tick, 1);
    assert.notDeepEqual(moved.pose, created.pose);
    const projection = { camera: moved.camera, viewport: null };
    const snapshot = bridge.readCameraProjection(projection);
    assert.equal(snapshot.viewMatrix.length, 16);
    assert.equal(snapshot.projectionMatrix.length, 16);
    assert.equal(snapshot.viewProjectionMatrix.length, 16);
});
test('mock: selectVoxel derives camera ray and edit anchor from generated view contracts', () => {
    const bridge = createMockRuntimeBridge();
    bridge.initializeEngine({ seed: 1 });
    const camera = bridge.createCamera({
        initialPose: { position: [1.5, 1.5, 4], yawDegrees: 0, pitchDegrees: 0 },
        projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
        viewport: { width: 1280, height: 720 },
    });
    const selection = bridge.selectVoxel({
        camera: camera.camera,
        grid: 1,
        viewport: null,
        screenPoint: { x: 0.5, y: 0.5, space: 'normalized_0_1' },
        maxDistance: 10,
    });
    assert.equal(selection.outcome, 'hit');
    assert.deepEqual(selection.pickRay.direction, [0, 0, -1]);
    assert.deepEqual(selection.selectedVoxel, { x: 1, y: 1, z: 0 });
    assert.equal(selection.selectedFace, 'posZ');
    assert.deepEqual(selection.editAnchor, { x: 1, y: 1, z: 1 });
    const miss = bridge.selectVoxel({
        camera: camera.camera,
        grid: 1,
        viewport: null,
        screenPoint: { x: 0.5, y: 0.5, space: 'normalized_0_1' },
        maxDistance: 1,
    });
    assert.equal(miss.outcome, 'miss');
    assert.equal(miss.selectedVoxel, null);
});
test('mock: camera-first-person-basic matches committed golden fixture', () => {
    const fixtureUrl = new URL('../../../../harness/camera/goldens/camera-first-person-basic.json', import.meta.url);
    const golden = JSON.parse(readFileSync(fixtureUrl, 'utf8'));
    const bridge = createMockRuntimeBridge();
    bridge.initializeEngine({ seed: 1 });
    const created = bridge.createCamera({
        initialPose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 },
        projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
        viewport: { width: 1280, height: 720 },
    });
    const moved = bridge.applyFirstPersonCameraInput({
        camera: created.camera,
        tick: 1,
        input: {
            moveForward: 1,
            moveRight: 0,
            moveUp: 0,
            yawDeltaDegrees: 15,
            pitchDeltaDegrees: -5,
            dtSeconds: 1 / 60,
            moveSpeedUnitsPerSecond: 3,
        },
    });
    const projection = bridge.readCameraProjection({ camera: moved.camera, viewport: null });
    assert.deepEqual(moved, golden.expected.moved);
    assert.deepEqual(projection, golden.expected.projection);
});
test('mock: step before init throws a classified error', () => {
    const bridge = createMockRuntimeBridge();
    assert.throws(() => bridge.stepSimulation({ tick: 1 }), (e) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized');
});
test('mock: buffer round-trip and unknown handle classification', () => {
    const bridge = createMockRuntimeBridge();
    bridge.initializeEngine({ seed: 0x01020304 });
    const view = bridge.getBuffer(0);
    const expected = new Uint8Array(8);
    new DataView(expected.buffer).setBigUint64(0, BigInt(0x01020304), true);
    assert.deepEqual(view.bytes, expected);
    assert.throws(() => bridge.getBuffer(99), (e) => e instanceof RuntimeBridgeError && e.kind === 'unknown_handle');
});
test('mock: readModelMaterialPreview returns public render-diff evidence without renderer internals', () => {
    const bridge = createMockRuntimeBridge();
    bridge.initializeEngine({ seed: 1 });
    const snapshot = bridge.readModelMaterialPreview(MODEL_MATERIAL_PREVIEW_REQUEST);
    assert.equal(snapshot.catalogEntry.id, 'material.copper');
    assert.equal(snapshot.meshAsset.asset, 'mesh.preview-cube');
    assert.equal(snapshot.rendererClassification, 'reference_preview');
    assert.deepEqual(snapshot.previewDiff.ops.map((op) => op.op), ['defineMaterial', 'defineStaticMesh', 'createStaticMeshInstance']);
    assert.ok(snapshot.diagnostics.some((diagnostic) => diagnostic.includes('fail closed')));
});
test('mock: scene-object snapshot and apply command use typed public contracts', () => {
    const bridge = createMockRuntimeBridge();
    bridge.initializeEngine({ seed: 1 });
    const snapshot = bridge.readSceneObjectSnapshot();
    assert.equal(snapshot.objects[0]?.kind, 'emptyGroup');
    assert.ok(snapshot.objects.some((object) => object.hasRenderableAsset));
    const result = bridge.applySceneObjectCommand({
        expectedDocumentHash: snapshot.documentHash,
        command: { kind: 'rename', id: snapshot.objects[0].id, label: 'Renamed root' },
    });
    assert.equal(result.accepted, true);
    assert.equal(result.rejection, null);
    assert.equal(result.outcome?.selected, snapshot.objects[0].id);
    assert.equal(result.outcome?.snapshot.objects[0]?.label, 'Renamed root');
    const stale = bridge.applySceneObjectCommand({
        expectedDocumentHash: snapshot.documentHash,
        command: { kind: 'select', id: snapshot.objects[0].id },
    });
    assert.equal(stale.accepted, false);
    assert.equal(stale.rejection?.code, 'stale-scene-object-snapshot');
});
test('mock: readRenderDiffs returns a contract-shaped frame', () => {
    const bridge = createMockRuntimeBridge();
    bridge.initializeEngine({ seed: 1 });
    const frame = bridge.readRenderDiffs(frameCursor(0));
    assert.deepEqual(frame, { ops: [] });
});
test('mock: world load → save → status → unload, with fail-closed save', () => {
    const bridge = createMockRuntimeBridge();
    // Save before load fails closed.
    assert.throws(() => bridge.saveCurrentWorld(), (e) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized');
    const status = bridge.loadWorldBundle({
        bundleSchemaVersion: 1,
        protocolVersion: 1,
        sceneId: 100,
    });
    assert.equal(status.loadedWorld, 100);
    assert.equal(status.blocksLoad, false);
    assert.deepEqual(bridge.saveCurrentWorld(), {
        artifactsWritten: 3,
        compactedEdits: 0,
        retainedEdits: 0,
    });
    assert.equal(bridge.getCompositionStatus().loadedWorld, 100);
    bridge.unloadWorld();
    assert.equal(bridge.getCompositionStatus().loadedWorld, null);
});
test('mock: an unsupported bundle version fails closed without swapping the world', () => {
    const bridge = createMockRuntimeBridge();
    bridge.loadWorldBundle({ bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 7 });
    assert.throws(() => bridge.loadWorldBundle({ bundleSchemaVersion: 99, protocolVersion: 1, sceneId: 8 }), (e) => e instanceof RuntimeBridgeError && e.kind === 'invalid_input');
    // The prior world stays loaded (no partial swap).
    assert.equal(bridge.getCompositionStatus().loadedWorld, 7);
});
test('mock: submitCommands carries the generated VoxelCommand union (the launch path)', () => {
    const bridge = createMockRuntimeBridge();
    bridge.initializeEngine({ seed: 1 });
    // A real generated voxel command — the authority-owned union, not a `{ kind }` blob.
    const command = {
        op: 'setVoxel',
        grid: 1,
        coord: { x: 0, y: 0, z: 0 },
        value: { kind: 'solid', material: 1 },
    };
    const result = bridge.submitCommands({ commands: [command] });
    assert.deepEqual(result, { accepted: 1, rejected: 0, rejections: [] });
});
test('mock: submitCommands before init fails closed', () => {
    const bridge = createMockRuntimeBridge();
    assert.throws(() => bridge.submitCommands({ commands: [] }), (e) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized');
});
test('an ad-hoc `{ kind }` command is NOT the launch path (compile-time guard)', () => {
    const bridge = createMockRuntimeBridge();
    bridge.initializeEngine({ seed: 1 });
    // The placeholder command shape the launch path used to accept must no longer
    // type-check: `submitCommands` takes the generated VoxelCommand union only.
    // @ts-expect-error — `{ kind: 'smoke-edit' }` is not a VoxelCommand.
    const bad = { commands: [{ kind: 'smoke-edit' }] };
    assert.equal(bad.commands.length, 1);
});
test('mock: pickVoxel carries a PickRay and returns a classified PickResult', () => {
    const bridge = createMockRuntimeBridge();
    bridge.initializeEngine({ seed: 1 });
    const result = bridge.pickVoxel({
        grid: 1,
        origin: [0, 0, 0],
        direction: [1, 0, 0],
        maxDistance: 10,
    });
    // The mock hosts no geometry, so it classifies as a miss (Rust authority owns hits).
    assert.deepEqual(result, { outcome: 'miss', rejection: { reason: 'noHit' } });
});
test('mock: readVoxelMeshEvidence returns compact chunk evidence and fails closed', () => {
    const bridge = createMockRuntimeBridge();
    assert.throws(() => bridge.readVoxelMeshEvidence({ grid: 1, chunks: [] }), (e) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized');
    bridge.initializeEngine({ seed: 1 });
    const snapshot = bridge.readVoxelMeshEvidence({ grid: 1, chunks: [] });
    assert.equal(snapshot.fixtureId, 'basic-voxel-landscape-interaction');
    assert.equal(snapshot.meshingStrategy, 'visible-face');
    assert.equal(snapshot.chunks.length, 1);
    assert.equal(snapshot.chunks[0]?.meshHash, 'fnv1a64:mock-mesh');
});
test('mock: pickVoxel before init fails closed', () => {
    const bridge = createMockRuntimeBridge();
    assert.throws(() => bridge.pickVoxel({ grid: 1, origin: [0, 0, 0], direction: [1, 0, 0], maxDistance: 10 }), (e) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized');
});
test('native factory classifies a missing addon path', () => {
    assert.throws(() => createNativeRuntimeBridge('./definitely-not-built.node'), (e) => e instanceof RuntimeBridgeError && e.kind === 'native_unavailable');
});
test('native bridge matches the mock when the addon is built (else skip)', (t) => {
    let bridge;
    try {
        bridge = createNativeRuntimeBridge();
    }
    catch (e) {
        if (e instanceof RuntimeBridgeError && e.kind === 'native_unavailable') {
            t.skip('native addon not built (run harness/ci/check-native.sh)');
            return;
        }
        throw e;
    }
    // Parity with MockRuntimeBridge / Rust ReferenceBridge for the native authority sequence.
    const handle = bridge.initializeEngine({ seed: 7 });
    assert.equal(typeof handle, 'number');
    assert.deepEqual(bridge.loadWorldBundle({ bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 1001 }), {
        loadedWorld: 1001,
        fatalCount: 0,
        totalCount: 0,
        blocksLoad: false,
    });
    assert.deepEqual(bridge.submitCommands({
        commands: [
            { op: 'setVoxel', grid: 1, coord: { x: 0, y: 0, z: 0 }, value: { kind: 'solid', material: 1 } },
        ],
    }), { accepted: 1, rejected: 0, rejections: [] });
    assert.deepEqual(bridge.stepSimulation({ tick: 6 }), { tick: 6, diffCount: 2 });
    assert.deepEqual(bridge.readRenderDiffs(frameCursor(0)), { ops: [] });
    assert.deepEqual(bridge.saveCurrentWorld(), { artifactsWritten: 3, compactedEdits: 0, retainedEdits: 0 });
    assert.deepEqual(bridge.getCompositionStatus(), {
        loadedWorld: 1001,
        fatalCount: 0,
        totalCount: 0,
        blocksLoad: false,
    });
});
//# sourceMappingURL=conformance.test.js.map