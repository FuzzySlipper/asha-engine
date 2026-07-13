export const frameCursor = (frame) => frame;
/** Typed, classified error for every facade operation. No JSON error blobs. */
export class RuntimeBridgeError extends Error {
    kind;
    constructor(kind, message) {
        super(`runtime bridge error [${kind}]: ${message}`);
        this.kind = kind;
        this.name = 'RuntimeBridgeError';
    }
}
export function nonNegativeSafeInteger(value, field) {
    if (!Number.isSafeInteger(value) || value < 0) {
        throw new RuntimeBridgeError('invalid_input', `${field} must be a non-negative safe integer`);
    }
    return value;
}
export function u32(value, field) {
    nonNegativeSafeInteger(value, field);
    if (value > 0xffffffff) {
        throw new RuntimeBridgeError('invalid_input', `${field} must fit in u32`);
    }
    return value;
}
/**
 * Produce fixed typed views over one root. Every property is statically named;
 * callers cannot request arbitrary capabilities or discover mutable state.
 */
export function runtimeBridgePorts(bridge) {
    return {
        input: bridge,
        timeSimulation: bridge,
        sceneEntities: bridge,
        voxelAssetsBuffers: bridge,
        camera: bridge,
        gameplay: bridge,
        projection: bridge,
        bundleLifecycle: bridge,
        replayEvidence: bridge,
    };
}
/** Reviewable lifecycle rules for the fixed port set. */
export const RUNTIME_BRIDGE_PORT_CONTRACTS = {
    input: {
        initialization: 'requiresEngine',
        projectBundle: 'retainedAcrossLoadUnload',
        snapshotHash: 'inputEvidence',
        resourceLifetime: 'session',
    },
    timeSimulation: {
        initialization: 'requiresEngine',
        projectBundle: 'retainedAcrossLoadUnload',
        snapshotHash: 'timeState',
        resourceLifetime: 'session',
    },
    sceneEntities: {
        initialization: 'requiresEngine',
        projectBundle: 'retainedAcrossLoadUnload',
        snapshotHash: 'sceneDocument',
        resourceLifetime: 'session',
    },
    voxelAssetsBuffers: {
        initialization: 'requiresEngine',
        projectBundle: 'retainedAcrossLoadUnload',
        snapshotHash: 'voxelStateAndResources',
        resourceLifetime: 'mixedExplicitAndSession',
    },
    camera: {
        initialization: 'requiresEngine',
        projectBundle: 'retainedAcrossLoadUnload',
        snapshotHash: 'cameraProjection',
        resourceLifetime: 'session',
    },
    gameplay: {
        initialization: 'requiresEngine',
        projectBundle: 'retainedAcrossLoadUnload',
        snapshotHash: 'gameplaySessionAndReplay',
        resourceLifetime: 'session',
    },
    projection: {
        initialization: 'requiresEngine',
        projectBundle: 'retainedAcrossLoadUnload',
        snapshotHash: 'projectionFrame',
        resourceLifetime: 'frame',
    },
    bundleLifecycle: {
        initialization: 'createsEngine',
        projectBundle: 'ownsLoadUnload',
        snapshotHash: 'compositionStatus',
        resourceLifetime: 'session',
    },
    replayEvidence: {
        initialization: 'requiresEngine',
        projectBundle: 'retainedAcrossLoadUnload',
        snapshotHash: 'replayEvidence',
        resourceLifetime: 'session',
    },
};
//# sourceMappingURL=bridge.js.map