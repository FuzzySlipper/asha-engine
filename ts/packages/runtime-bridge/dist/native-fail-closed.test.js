// Native facade parity / fail-closed conformance (task #2423).
//
// Proves the seam closed in this task: a *loaded* native facade either executes a
// real native implementation or throws a classified `operation_unimplemented`
// error for every manifest operation. It must NEVER silently inherit mock /
// reference behaviour for an unwired op (the prior `extends MockRuntimeBridge`
// hazard). We inject a fake addon so the test runs without a built `.node` binary.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { MANIFEST_OPERATIONS, NATIVE_WIRED_OPERATIONS, NativeRuntimeBridge, RuntimeBridgeError, frameCursor, } from './index.js';
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
const CAMERA_CREATE_REQUEST = {
    initialPose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 },
    projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
    viewport: { width: 1280, height: 720 },
};
const CAMERA_INPUT = {
    camera: 1,
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
const REQUIRED_NATIVE_CONFORMANCE_OPS = [
    'initialize_engine',
    'load_world_bundle',
    'submit_commands',
    'step_simulation',
    'apply_enemy_direct_nav_movement',
    'read_render_diffs',
    'save_current_world',
    'get_composition_status',
];
// A fake addon with sentinel return values distinct from MockRuntimeBridge, so a
// silent mock fallback would be observable in the wired-op assertions below.
function fakeAddon(calls = []) {
    return {
        initializeEngine: (seed) => {
            calls.push(`initialize:${seed}`);
            return seed + 100;
        },
        loadWorldBundle: (_handle, bundleSchemaVersion, protocolVersion, sceneId) => {
            calls.push(`load:${bundleSchemaVersion}:${protocolVersion}:${sceneId}`);
            return { loadedWorld: sceneId + 1000, fatalCount: 0, totalCount: 0, blocksLoad: false };
        },
        submitCommands: (_handle, commandsJson) => {
            calls.push(`submit:${commandsJson}`);
            const commands = JSON.parse(commandsJson);
            return { accepted: Array.isArray(commands) ? commands.length : 0, rejected: 0, rejections: [] };
        },
        stepSimulation: (_handle, tick) => {
            calls.push(`step:${tick}`);
            return 9;
        },
        applyEnemyDirectNavMovement: (_handle, entity, seedPosition, target, maxStepUnits) => {
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
        readRenderDiffs: (_handle, cursor) => {
            calls.push(`render:${cursor}`);
            return { ops: [{ op: 'sentinel' }] };
        },
        saveCurrentWorld: (handle) => {
            void handle;
            calls.push('save');
            return { artifactsWritten: 5, compactedEdits: 2, retainedEdits: 3 };
        },
        getCompositionStatus: (handle) => {
            void handle;
            calls.push('status');
            return { loadedWorld: 2001, fatalCount: 0, totalCount: 0, blocksLoad: false };
        },
    };
}
// One invocation per facade method. The native bridge is fully initialized first
// so that wired ops exercise their happy path rather than `not_initialized`.
// Typed against the `RuntimeBridge` interface (which carries the operation
// payloads); a `NativeRuntimeBridge` instance is assignable to it.
const INVOKE = new Map([
    ['initializeEngine', (b) => b.initializeEngine({ seed: 7 })],
    ['stepSimulation', (b) => b.stepSimulation({ tick: 6 })],
    ['submitCommands', (b) => b.submitCommands({ commands: [] })],
    [
        'pickVoxel',
        (b) => b.pickVoxel({ grid: 1, origin: [0, 0, 0], direction: [1, 0, 0], maxDistance: 10 }),
    ],
    [
        'applyCollisionConstrainedCameraInput',
        (b) => b.applyCollisionConstrainedCameraInput({
            ...CAMERA_INPUT,
            grid: 1,
            shape: { halfExtents: [0.2, 0.2, 0.2] },
            policy: { mode: 'axis_separable_slide', maxIterations: 3 },
        }),
    ],
    [
        'selectVoxel',
        (b) => b.selectVoxel({
            camera: CAMERA_INPUT.camera,
            grid: 1,
            viewport: null,
            screenPoint: { x: 0.5, y: 0.5, space: 'normalized_0_1' },
            maxDistance: 10,
        }),
    ],
    ['readVoxelMeshEvidence', (b) => b.readVoxelMeshEvidence({ grid: 1, chunks: [] })],
    ['readModelMaterialPreview', (b) => b.readModelMaterialPreview(MODEL_MATERIAL_PREVIEW_REQUEST)],
    ['readSceneObjectSnapshot', (b) => b.readSceneObjectSnapshot()],
    [
        'applySceneObjectCommand',
        (b) => b.applySceneObjectCommand({
            expectedDocumentHash: 1,
            command: { kind: 'select', id: null },
        }),
    ],
    ['readRenderDiffs', (b) => b.readRenderDiffs(frameCursor(0))],
    ['createCamera', (b) => b.createCamera(CAMERA_CREATE_REQUEST)],
    ['applyFirstPersonCameraInput', (b) => b.applyFirstPersonCameraInput(CAMERA_INPUT)],
    [
        'applyEnemyDirectNavMovement',
        (b) => b.applyEnemyDirectNavMovement({
            entity: 777,
            seedPosition: [0, 0.5, -2.6],
            target: [0, 1.62, 1.25],
            maxStepUnits: 0.35,
        }),
    ],
    ['readCameraProjection', (b) => b.readCameraProjection({ camera: CAMERA_INPUT.camera, viewport: null })],
    ['getBuffer', (b) => b.getBuffer(0)],
    ['releaseBuffer', (b) => b.releaseBuffer(0)],
    [
        'loadWorldBundle',
        (b) => b.loadWorldBundle({ bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 1 }),
    ],
    ['saveCurrentWorld', (b) => b.saveCurrentWorld()],
    ['getCompositionStatus', (b) => b.getCompositionStatus()],
    ['unloadWorld', (b) => b.unloadWorld()],
    ['loadReplayFixture', (b) => b.loadReplayFixture({ name: 'x', steps: 1 })],
    ['runReplayStep', (b) => b.runReplayStep(0)],
]);
void test('every manifest op has a native invocation in this test', () => {
    for (const op of MANIFEST_OPERATIONS) {
        assert.ok(INVOKE.has(op.facadeMethod), `missing invocation for ${op.facadeMethod}`);
    }
});
void test('unwired native ops fail closed with operation_unimplemented (no mock fallback)', () => {
    for (const op of MANIFEST_OPERATIONS) {
        if (NATIVE_WIRED_OPERATIONS.has(op.manifestName))
            continue;
        const invoke = INVOKE.get(op.facadeMethod);
        assert.ok(invoke, `missing invocation for ${op.facadeMethod}`);
        const bridge = new NativeRuntimeBridge(fakeAddon());
        // A fresh, initialized bridge: proves the throw is fail-closed classification,
        // not an incidental `not_initialized`.
        bridge.initializeEngine({ seed: 1 });
        assert.throws(() => invoke(bridge), (e) => e instanceof RuntimeBridgeError && e.kind === 'operation_unimplemented', `${op.manifestName} must fail closed, not inherit mock behaviour`);
    }
});
void test('required native conformance operations are declared wired', () => {
    for (const manifestName of REQUIRED_NATIVE_CONFORMANCE_OPS) {
        assert.ok(NATIVE_WIRED_OPERATIONS.has(manifestName), `${manifestName} must be wired for native authority conformance`);
    }
});
void test('native conformance sequence routes through the addon without mock fallback', () => {
    const calls = [];
    const bridge = new NativeRuntimeBridge(fakeAddon(calls));
    assert.equal(bridge.initializeEngine({ seed: 7 }), 107);
    assert.deepEqual(bridge.loadWorldBundle({ bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 1001 }), {
        loadedWorld: 2001,
        fatalCount: 0,
        totalCount: 0,
        blocksLoad: false,
    });
    assert.deepEqual(bridge.submitCommands({
        commands: [
            { op: 'setVoxel', grid: 1, coord: { x: 0, y: 0, z: 0 }, value: { kind: 'solid', material: 1 } },
        ],
    }), { accepted: 1, rejected: 0, rejections: [] });
    assert.deepEqual(bridge.stepSimulation({ tick: 6 }), { tick: 6, diffCount: 9 });
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
    assert.deepEqual(bridge.readRenderDiffs(frameCursor(0)), { ops: [{ op: 'sentinel' }] });
    assert.deepEqual(bridge.saveCurrentWorld(), { artifactsWritten: 5, compactedEdits: 2, retainedEdits: 3 });
    assert.deepEqual(bridge.getCompositionStatus(), {
        loadedWorld: 2001,
        fatalCount: 0,
        totalCount: 0,
        blocksLoad: false,
    });
    assert.deepEqual(calls, [
        'initialize:7',
        'load:1:1:1001',
        'submit:[{"op":"setVoxel","grid":1,"coord":{"x":0,"y":0,"z":0},"value":{"kind":"solid","material":1}}]',
        'step:6',
        'enemyMove:777:0,0.5,-2.6:0,1.62,1.25:0.35',
        'render:0',
        'save',
        'status',
    ]);
});
void test('native facade validates numeric inputs before addon casts can wrap', () => {
    const calls = [];
    const bridge = new NativeRuntimeBridge(fakeAddon(calls));
    bridge.initializeEngine({ seed: 1 });
    assert.throws(() => bridge.loadWorldBundle({ bundleSchemaVersion: 1.5, protocolVersion: 1, sceneId: 1 }), (e) => e instanceof RuntimeBridgeError && e.kind === 'invalid_input');
    assert.throws(() => bridge.loadWorldBundle({ bundleSchemaVersion: 1, protocolVersion: 1, sceneId: -1 }), (e) => e instanceof RuntimeBridgeError && e.kind === 'invalid_input');
    assert.throws(() => bridge.stepSimulation({ tick: -1 }), (e) => e instanceof RuntimeBridgeError && e.kind === 'invalid_input');
    assert.throws(() => bridge.readRenderDiffs(frameCursor(-1)), (e) => e instanceof RuntimeBridgeError && e.kind === 'invalid_input');
    assert.deepEqual(calls, ['initialize:1']);
});
void test('native addon semantic errors are reclassified into RuntimeBridgeError', () => {
    const addon = fakeAddon();
    addon.loadWorldBundle = () => {
        throw new Error('InvalidInput: unsupported bundle schema 99 / protocol 1');
    };
    const bridge = new NativeRuntimeBridge(addon);
    bridge.initializeEngine({ seed: 1 });
    assert.throws(() => bridge.loadWorldBundle({ bundleSchemaVersion: 99, protocolVersion: 1, sceneId: 1 }), (e) => e instanceof RuntimeBridgeError &&
        e.kind === 'invalid_input' &&
        e.message.includes('unsupported bundle schema 99 / protocol 1'));
});
void test('wired native ops route through the addon, not the mock', () => {
    const calls = [];
    const bridge = new NativeRuntimeBridge(fakeAddon(calls));
    // Mock would return the seed (7) and diffCount 2; the addon returns 107 / 9.
    assert.equal(bridge.initializeEngine({ seed: 7 }), 107);
    assert.deepEqual(bridge.stepSimulation({ tick: 6 }), { tick: 6, diffCount: 9 });
    assert.deepEqual(calls, ['initialize:7', 'step:6']);
});
void test('native bridge does not extend MockRuntimeBridge (no inherited mock methods)', () => {
    // Guards against re-introducing the `extends MockRuntimeBridge` seam: every
    // own/inherited facade method must be declared on NativeRuntimeBridge itself.
    const proto = NativeRuntimeBridge.prototype;
    for (const op of MANIFEST_OPERATIONS) {
        assert.ok(Object.prototype.hasOwnProperty.call(Object.getPrototypeOf(new NativeRuntimeBridge(fakeAddon())), op.facadeMethod), `${op.facadeMethod} must be declared on NativeRuntimeBridge, not inherited`);
        assert.equal(typeof proto[op.facadeMethod], 'function');
    }
});
void test('native bridge step before init fails closed (not_initialized)', () => {
    const bridge = new NativeRuntimeBridge(fakeAddon());
    assert.throws(() => bridge.stepSimulation({ tick: 1 }), (e) => e instanceof RuntimeBridgeError && e.kind === 'not_initialized');
});
void test('wired set names are real manifest operations', () => {
    const manifestNames = new Set(MANIFEST_OPERATIONS.map((o) => o.manifestName));
    for (const name of NATIVE_WIRED_OPERATIONS) {
        assert.ok(manifestNames.has(name), `${name} in NATIVE_WIRED_OPERATIONS is not a manifest op`);
    }
});
//# sourceMappingURL=native-fail-closed.test.js.map