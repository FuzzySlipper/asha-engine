import { type RuntimeBridge } from './bridge.js';
export declare const NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND = "asha.runtime_bridge.native_rust_provider.v1";
export declare const LEGACY_ASHA_DEMO_NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND = "asha_demo.native_runtime_bridge_provider.v1";
export declare const NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_GLOBALS: readonly ["ashaRuntimeBridge", "ashaDemoRuntimeBridge"];
export type NativeRustRuntimeBridgeProviderGlobalName = typeof NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_GLOBALS[number];
export declare const NATIVE_RUST_RUNTIME_BRIDGE_REQUIRED_METHODS: readonly ["initializeEngine", "loadProjectBundle", "getProjectBundleCompositionStatus", "createCamera", "applyCollisionConstrainedCameraInput", "loadFpsRuntimeSession", "readFpsRuntimeSession", "applyFpsPrimaryFire", "invokeGameExtensionWeaponEffect", "restartFpsRuntimeSession", "applyEnemyDirectNavMovement", "planVoxelConversion", "registerVoxelConversionSource", "previewVoxelConversion", "applyVoxelConversion", "exportVoxelConversionEvidence", "readVoxelModelInfo", "unloadProjectBundle"];
export type NativeRustRuntimeBridgeProviderKind = typeof NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND | typeof LEGACY_ASHA_DEMO_NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND;
export type NativeRustRuntimeBridgeProviderDiagnosticCode = 'missing_rust_runtime_backend' | 'invalid_rust_runtime_provider' | 'missing_runtime_bridge' | 'missing_runtime_bridge_operation' | 'non_native_runtime_authority';
export interface NativeRustRuntimeBridgeProviderDiagnostic {
    readonly code: NativeRustRuntimeBridgeProviderDiagnosticCode;
    readonly severity: 'error';
    readonly message: string;
}
export interface NativeRustRuntimeBridgeProviderProfile {
    readonly kind: 'runtime_bridge.native_rust_provider_profile.v1';
    readonly mode: 'rust';
    readonly transport: 'public_runtime_bridge_provider';
    readonly providerGlobal: string | null;
    readonly providerContract: NativeRustRuntimeBridgeProviderKind;
    readonly requiredBackend: 'native_rust';
    readonly productAuthority: true;
    readonly referenceFallback: false;
}
export interface NativeRustRuntimeBridgeProvider {
    readonly kind: NativeRustRuntimeBridgeProviderKind;
    readonly backend: 'native_rust';
    readonly productAuthority: true;
    readonly referenceFallback: false;
    readonly createRuntimeBridge?: () => RuntimeBridge | Promise<RuntimeBridge>;
    readonly bridge?: RuntimeBridge | Promise<RuntimeBridge>;
}
export interface NativeRustRuntimeBridgeProviderCandidate {
    readonly kind?: string;
    readonly backend?: string;
    readonly productAuthority?: boolean;
    readonly referenceFallback?: boolean;
    readonly createRuntimeBridge?: () => RuntimeBridge | Promise<RuntimeBridge>;
    readonly bridge?: RuntimeBridge | Promise<RuntimeBridge> | null;
}
export interface CreateNativeRustRuntimeBridgeProviderRequest {
    readonly bridge?: RuntimeBridge | Promise<RuntimeBridge>;
    readonly createRuntimeBridge?: () => RuntimeBridge | Promise<RuntimeBridge>;
}
export interface InstallNativeRustRuntimeBridgeProviderRequest extends CreateNativeRustRuntimeBridgeProviderRequest {
    readonly globalScope?: Record<string, NativeRustRuntimeBridgeProviderCandidate | null | undefined>;
    readonly providerGlobalName?: NativeRustRuntimeBridgeProviderGlobalName;
    readonly provider?: NativeRustRuntimeBridgeProvider;
}
export interface NativeRustRuntimeBridgeProviderInstallation {
    readonly provider: NativeRustRuntimeBridgeProvider;
    readonly providerGlobal: string;
    readonly profile: NativeRustRuntimeBridgeProviderProfile;
}
export interface ResolveNativeRustRuntimeBridgeProviderRequest {
    readonly provider?: NativeRustRuntimeBridgeProviderCandidate | null;
    readonly globalScope?: Record<string, NativeRustRuntimeBridgeProviderCandidate | null | undefined>;
    readonly providerGlobalNames?: readonly string[];
    readonly providerKinds?: readonly NativeRustRuntimeBridgeProviderKind[];
}
export type NativeRustRuntimeBridgeProviderResolution = {
    readonly status: 'available';
    readonly provider: NativeRustRuntimeBridgeProvider;
    readonly bridge: RuntimeBridge;
    readonly providerGlobal: string | null;
    readonly profile: NativeRustRuntimeBridgeProviderProfile;
    readonly diagnostics: readonly [];
} | {
    readonly status: 'unavailable';
    readonly provider: null;
    readonly bridge: null;
    readonly providerGlobal: string | null;
    readonly profile: NativeRustRuntimeBridgeProviderProfile;
    readonly diagnostics: readonly NativeRustRuntimeBridgeProviderDiagnostic[];
};
export interface NativeRustRuntimeAuthorityInput {
    readonly ecrpAuthority: {
        readonly mode: string;
        readonly source: string;
    };
    readonly fpsSnapshot: {
        readonly backend: string;
    };
}
export type NativeRustRuntimeAuthorityValidation = {
    readonly ok: true;
    readonly diagnostics: readonly [];
} | {
    readonly ok: false;
    readonly diagnostics: readonly NativeRustRuntimeBridgeProviderDiagnostic[];
};
export declare function createNativeRustRuntimeBridgeProvider(request: CreateNativeRustRuntimeBridgeProviderRequest): NativeRustRuntimeBridgeProvider;
export declare function installNativeRustRuntimeBridgeProvider(request: InstallNativeRustRuntimeBridgeProviderRequest): NativeRustRuntimeBridgeProviderInstallation;
export declare function resolveNativeRustRuntimeBridgeProvider(request?: ResolveNativeRustRuntimeBridgeProviderRequest): Promise<NativeRustRuntimeBridgeProviderResolution>;
export declare function validateNativeRustRuntimeBridgeAuthority(input: NativeRustRuntimeAuthorityInput): NativeRustRuntimeAuthorityValidation;
export declare function assertNativeRustRuntimeBridgeAuthority(input: NativeRustRuntimeAuthorityInput): void;
//# sourceMappingURL=native-runtime-provider.d.ts.map