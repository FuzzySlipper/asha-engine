import { RuntimeBridgeError } from './bridge.js';
import { createMockRuntimeBridge } from './mock.js';
import { BridgeGameRuntimeSession, } from './launcher.js';
function requireNonEmpty(value, field) {
    if (value.trim().length === 0) {
        throw new RuntimeBridgeError('invalid_input', `${field} must be a non-empty string`);
    }
    return value;
}
function referenceNonClaims() {
    return [
        'not_native_runtime',
        'not_hardware_gpu',
        'not_performance_evidence',
        'not_publish_artifact',
        'not_product_authority',
        'not_wasm_authority',
    ];
}
export function referenceBackendProfile(config) {
    return {
        profileId: 'reference.launcher.v1',
        mode: 'reference',
        transport: 'reference_mock',
        launcherName: 'reference-game-runtime-launcher',
        bridgeCompatibility: config.compatibility,
        evidenceRefs: [{ kind: 'diagnostic', id: 'backend-profile:reference' }],
        nonClaims: referenceNonClaims(),
    };
}
function referenceRuntimeProfile(config) {
    const profile = referenceBackendProfile(config);
    return {
        profileId: profile.profileId,
        runtimeMode: profile.mode,
        launcherName: profile.launcherName,
        bridgeCompatibility: profile.bridgeCompatibility,
        nonClaims: profile.nonClaims,
    };
}
export class ReferenceGameRuntimeSession extends BridgeGameRuntimeSession {
}
export class ReferenceGameRuntimeLauncher {
    mode = 'reference';
    async launch(config) {
        requireNonEmpty(config.gameId, 'gameId');
        requireNonEmpty(config.workspaceId, 'workspaceId');
        requireNonEmpty(config.runtimeEntry, 'runtimeEntry');
        requireNonEmpty(config.compatibility.contractsPackageVersion, 'compatibility.contractsPackageVersion');
        requireNonEmpty(config.compatibility.runtimeBridgePackageVersion, 'compatibility.runtimeBridgePackageVersion');
        if (config.runtimeEntry !== config.resourceProfile.runtimeEntry) {
            throw new RuntimeBridgeError('invalid_input', 'runtimeEntry must match resourceProfile.runtimeEntry');
        }
        const bridge = createMockRuntimeBridge();
        bridge.initializeEngine({ seed: config.projectBundle.sceneId });
        const status = bridge.loadProjectBundle(config.projectBundle);
        if (status.blocksLoad || status.loadedProjectBundle === null) {
            throw new RuntimeBridgeError('invalid_input', 'project bundle failed to load for reference launcher');
        }
        return new ReferenceGameRuntimeSession(bridge, config, referenceRuntimeProfile(config), status);
    }
}
export function createReferenceGameRuntimeLauncher() {
    return new ReferenceGameRuntimeLauncher();
}
//# sourceMappingURL=reference-launcher.js.map