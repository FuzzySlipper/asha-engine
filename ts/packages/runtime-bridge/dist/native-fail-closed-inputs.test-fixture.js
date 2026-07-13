import { createDefaultBrowserInputCatalog } from './browser-input-host.js';
export const INPUT_SESSION_CONFIGURE_REQUEST = {
    catalog: createDefaultBrowserInputCatalog(),
    initialContexts: ['gameplay'],
};
export const INPUT_CONTEXT_COMMAND = { operation: 'push', contextId: 'menu' };
export const RAW_INPUT_SAMPLE = {
    sequence: 0,
    platformKind: 'keyboardKey',
    control: 'KeyW',
    phase: 'pressed',
    value: { kind: 'button', pressed: true },
};
export const RECORDED_INPUT_ACTION = {
    schemaVersion: 1,
    action: {
        sequence: 0,
        actionId: 'gameplay.move.forward',
        contextId: 'gameplay',
        bindingId: 'gameplay-forward',
        phase: 'pressed',
        value: { kind: 'button', pressed: true },
    },
    catalogHash: 'fnv1a64:aaaaaaaaaaaaaaaa',
    contextHash: 'fnv1a64:bbbbbbbbbbbbbbbb',
    recordHash: 'fnv1a64:cccccccccccccccc',
};
export function createNativeInputHandlers(hashA, hashB, hashC) {
    return {
        configureInputSession: () => JSON.stringify({
            catalogHash: hashA,
            contextState: inputContextState(0, ['gameplay'], hashB),
        }),
        applyInputContextCommand: () => JSON.stringify({
            accepted: true,
            state: inputContextState(1, ['gameplay', 'menu'], hashC),
            diagnostics: [],
        }),
        submitRawInput: () => JSON.stringify({
            sequence: 0,
            accepted: true,
            consumed: true,
            action: {
                sequence: 0, actionId: 'gameplay.move.forward', contextId: 'gameplay',
                bindingId: 'gameplay-forward', phase: 'pressed', value: { kind: 'button', pressed: true },
            },
            diagnostics: [],
            catalogHash: hashA,
            contextHash: hashB,
            inputHash: hashC,
            resolutionHash: hashA,
            record: null,
        }),
        replayResolvedInputAction: () => JSON.stringify({
            accepted: true,
            action: RECORDED_INPUT_ACTION.action,
            diagnostics: [],
            catalogHash: hashA,
            contextHash: hashB,
            recordHash: hashC,
            replayHash: hashA,
        }),
        readInputContextState: () => JSON.stringify(inputContextState(0, ['gameplay'], hashB)),
        applyTimeControlCommand: () => JSON.stringify({
            accepted: true,
            before: timeControlState('running', 0, 0, hashA),
            after: timeControlState('paused', 1, 0, hashB),
            exactTicksAdvanced: 0,
            rejection: null,
            receiptHash: hashC,
        }),
        readTimeControlState: () => JSON.stringify(timeControlState('running', 0, 0, hashA)),
    };
}
function timeControlState(mode, revision, authorityTick, stateHash) {
    return { schemaVersion: 1, mode, speedMultiplier: 1, revision, authorityTick, stateHash };
}
function inputContextState(revision, contextIds, stateHash) {
    return {
        schemaVersion: 1,
        revision,
        activeContexts: contextIds.map((contextId, stackOrder) => ({ contextId, stackOrder })),
        stateHash,
    };
}
export const MODEL_MATERIAL_PREVIEW_REQUEST = {
    catalogEntry: {
        id: 'material.copper',
        kind: 'material',
        version: 1,
        hash: 'sha256-material-copper',
        sourcePath: null,
        label: 'Copper',
        dependencies: [],
        material: {
            render: { color: { r: 0.8, g: 0.4, b: 0.2, a: 1 }, texture: null, roughness: 0.6, textureTint: { r: 1, g: 1, b: 1, a: 1 }, emissionColor: { r: 0.8, g: 0.4, b: 0.2, a: 1 }, emissive: 0, uvStrategy: 'flat' },
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
export const CAMERA_CREATE_REQUEST = {
    initialPose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 },
    projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
    viewport: { width: 1280, height: 720 },
};
export const CAMERA_INPUT = {
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
export const COLLISION_CAMERA_INPUT = {
    ...CAMERA_INPUT,
    grid: 1,
    movementMode: 'grounded',
    shape: { halfExtents: [0.2, 0.2, 0.2] },
    policy: { mode: 'axis_separable_slide', maxIterations: 3 },
};
//# sourceMappingURL=native-fail-closed-inputs.test-fixture.js.map