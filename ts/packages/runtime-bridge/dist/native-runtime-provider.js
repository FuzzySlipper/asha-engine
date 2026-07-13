import { RuntimeBridgeError, } from './bridge.js';
export const NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND = 'asha.runtime_bridge.native_rust_provider.v1';
export const LEGACY_ASHA_DEMO_NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND = 'asha_demo.native_runtime_bridge_provider.v1';
export const NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_GLOBALS = [
    'ashaRuntimeBridge',
    'ashaDemoRuntimeBridge',
];
export const NATIVE_RUST_RUNTIME_BRIDGE_REQUIRED_METHODS = [
    'initializeEngine',
    'loadProjectBundle', // vocab-allow: provider compatibility check must require the legacy bridge operation.
    'getProjectBundleCompositionStatus',
    'createCamera',
    'applyCollisionConstrainedCameraInput',
    'loadFpsRuntimeSession',
    'readFpsRuntimeSession',
    'applyFpsPrimaryFire',
    'invokeGameExtensionWeaponEffect',
    'restartFpsRuntimeSession',
    'applyEnemyDirectNavMovement',
    'planVoxelConversion',
    'registerVoxelConversionSource',
    'registerVoxelConversionMeshAsset',
    'importVoxelConversionMeshSource',
    'readVoxelConversionSourceMetadata',
    'previewVoxelConversion',
    'applyVoxelConversion',
    'exportVoxelConversionEvidence',
    'readVoxelModelInfo',
    'readVoxelModelWindow',
    'readVoxelEditHistory',
    'previewVoxelEditRevert',
    'applyVoxelEditRevert',
    'undoVoxelEdit',
    'redoVoxelEdit',
    'unloadProjectBundle',
];
export function createNativeRustRuntimeBridgeProvider(request) {
    const hasBridge = request.bridge !== undefined;
    const hasFactory = request.createRuntimeBridge !== undefined;
    if (hasBridge === hasFactory) {
        throw new RuntimeBridgeError('invalid_input', 'Native RuntimeBridge provider requires exactly one bridge or createRuntimeBridge factory.');
    }
    if (hasFactory) {
        const createRuntimeBridge = request.createRuntimeBridge;
        if (createRuntimeBridge === undefined) {
            throw new RuntimeBridgeError('invalid_input', 'Native RuntimeBridge provider factory was missing.');
        }
        return {
            kind: NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND,
            backend: 'native_rust',
            productAuthority: true,
            referenceFallback: false,
            createRuntimeBridge,
        };
    }
    const bridge = request.bridge;
    if (bridge === undefined) {
        throw new RuntimeBridgeError('invalid_input', 'Native RuntimeBridge provider bridge was missing.');
    }
    return {
        kind: NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND,
        backend: 'native_rust',
        productAuthority: true,
        referenceFallback: false,
        bridge,
    };
}
export function installNativeRustRuntimeBridgeProvider(request) {
    const provider = request.provider ?? createNativeRustRuntimeBridgeProvider(request);
    if (!isNativeRustRuntimeBridgeProvider(provider, [NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND])) {
        throw new RuntimeBridgeError('invalid_input', 'Standalone host must install the public native Rust RuntimeBridge provider contract with product authority and no reference fallback.');
    }
    const providerGlobalName = request.providerGlobalName ?? 'ashaRuntimeBridge';
    const globalScope = request.globalScope ?? defaultNativeRustRuntimeBridgeProviderGlobalTarget();
    globalScope[providerGlobalName] = provider;
    const providerGlobal = `globalThis.${providerGlobalName}`;
    return {
        provider,
        providerGlobal,
        profile: nativeRustRuntimeBridgeProviderProfile(providerGlobal, provider.kind),
    };
}
export async function resolveNativeRustRuntimeBridgeProvider(request = {}) {
    const lookup = readProviderCandidate(request);
    const providerKinds = request.providerKinds ?? [
        NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND,
        LEGACY_ASHA_DEMO_NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND,
    ];
    const fallbackKind = providerKinds[0] ?? NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND;
    const invalidProfile = nativeRustRuntimeBridgeProviderProfile(lookup.providerGlobal, fallbackKind);
    if (lookup.provider === null) {
        return unavailableNativeRustRuntimeBridgeProvider(lookup.providerGlobal, fallbackKind, 'missing_rust_runtime_backend', 'ASHA requires a public native Rust RuntimeBridge provider; static host does not fall back to reference authority.');
    }
    if (!isNativeRustRuntimeBridgeProvider(lookup.provider, providerKinds)) {
        return {
            status: 'unavailable',
            provider: null,
            bridge: null,
            providerGlobal: lookup.providerGlobal,
            profile: invalidProfile,
            diagnostics: [{
                    code: 'invalid_rust_runtime_provider',
                    severity: 'error',
                    message: `RuntimeBridge provider must use the public native Rust contract (${providerKinds.join(' or ')}) with product authority and no reference fallback.`,
                }],
        };
    }
    try {
        const bridge = await readProvidedRuntimeBridge(lookup.provider);
        const missingOperation = NATIVE_RUST_RUNTIME_BRIDGE_REQUIRED_METHODS.find((method) => typeof bridge[method] !== 'function');
        if (missingOperation !== undefined) {
            return unavailableNativeRustRuntimeBridgeProvider(lookup.providerGlobal, lookup.provider.kind, 'missing_runtime_bridge_operation', `RuntimeBridge provider is missing required operation: ${missingOperation}`);
        }
        return {
            status: 'available',
            provider: lookup.provider,
            bridge,
            providerGlobal: lookup.providerGlobal,
            profile: nativeRustRuntimeBridgeProviderProfile(lookup.providerGlobal, lookup.provider.kind),
            diagnostics: [],
        };
    }
    catch (error) {
        return unavailableNativeRustRuntimeBridgeProvider(lookup.providerGlobal, lookup.provider.kind, 'missing_runtime_bridge', error instanceof Error ? error.message : String(error));
    }
}
export function validateNativeRustRuntimeBridgeAuthority(input) {
    if (input.ecrpAuthority.mode === 'rust'
        && input.ecrpAuthority.source === 'rust_bridge'
        && input.fpsSnapshot.backend === 'native_rust') {
        return { ok: true, diagnostics: [] };
    }
    return {
        ok: false,
        diagnostics: [{
                code: 'non_native_runtime_authority',
                severity: 'error',
                message: `ASHA rejected non-native RuntimeBridge provider: ECRP source=${input.ecrpAuthority.source}, FPS backend=${input.fpsSnapshot.backend}`,
            }],
    };
}
export function assertNativeRustRuntimeBridgeAuthority(input) {
    const validation = validateNativeRustRuntimeBridgeAuthority(input);
    if (!validation.ok) {
        throw new RuntimeBridgeError('invalid_input', validation.diagnostics[0]?.message ?? 'non-native RuntimeBridge authority rejected');
    }
}
function readProviderCandidate(request) {
    if (request.provider !== undefined) {
        return { provider: request.provider, providerGlobal: null };
    }
    const scope = request.globalScope ?? defaultNativeRustRuntimeBridgeProviderGlobals();
    for (const name of request.providerGlobalNames ?? NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_GLOBALS) {
        if (scope[name] !== undefined && scope[name] !== null) {
            return { provider: scope[name], providerGlobal: `globalThis.${name}` };
        }
    }
    return { provider: null, providerGlobal: null };
}
function defaultNativeRustRuntimeBridgeProviderGlobals() {
    const globals = globalThis;
    return {
        ashaRuntimeBridge: globals.ashaRuntimeBridge,
        ashaDemoRuntimeBridge: globals.ashaDemoRuntimeBridge,
    };
}
function defaultNativeRustRuntimeBridgeProviderGlobalTarget() {
    return globalThis;
}
function isNativeRustRuntimeBridgeProvider(value, providerKinds) {
    return value !== null
        && isNativeRustRuntimeBridgeProviderKind(value.kind, providerKinds)
        && value.backend === 'native_rust'
        && value.productAuthority === true
        && value.referenceFallback === false;
}
function isNativeRustRuntimeBridgeProviderKind(value, providerKinds) {
    return value !== undefined && providerKinds.includes(value);
}
async function readProvidedRuntimeBridge(provider) {
    const candidate = typeof provider.createRuntimeBridge === 'function'
        ? provider.createRuntimeBridge()
        : provider.bridge;
    const bridge = await candidate;
    if (bridge === undefined || bridge === null || typeof bridge !== 'object') {
        throw new RuntimeBridgeError('invalid_input', 'RuntimeBridge provider did not return a bridge object');
    }
    return bridge;
}
function unavailableNativeRustRuntimeBridgeProvider(providerGlobal, providerContract, code, message) {
    return {
        status: 'unavailable',
        provider: null,
        bridge: null,
        providerGlobal,
        profile: nativeRustRuntimeBridgeProviderProfile(providerGlobal, providerContract),
        diagnostics: [{ code, severity: 'error', message }],
    };
}
function nativeRustRuntimeBridgeProviderProfile(providerGlobal, providerContract) {
    return {
        kind: 'runtime_bridge.native_rust_provider_profile.v1',
        mode: 'rust',
        transport: 'public_runtime_bridge_provider',
        providerGlobal,
        providerContract,
        requiredBackend: 'native_rust',
        productAuthority: true,
        referenceFallback: false,
    };
}
//# sourceMappingURL=native-runtime-provider.js.map