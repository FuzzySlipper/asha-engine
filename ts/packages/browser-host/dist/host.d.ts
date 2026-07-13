import { type Server } from 'node:http';
import { type NativeRustRuntimeBridgeProviderCandidate, type NativeRustRuntimeBridgeProviderDiagnostic, type NativeRustRuntimeBridgeProviderInstallation, type NativeRustRuntimeBridgeProviderProfile, type RuntimeBridge } from '@asha/runtime-bridge';
export declare const ASHA_BROWSER_HOST_COMPATIBILITY_VERSION = "browser-host.v0";
export declare const ASHA_BROWSER_HOST_PROVIDER_GLOBAL = "ashaRuntimeBridge";
export declare const ASHA_BROWSER_HOST_PROVIDER_KIND = "asha.runtime_bridge.native_rust_provider.v1";
export declare const ASHA_BROWSER_HOST_BRIDGE_CLIENT_HEADER = "X-ASHA-Runtime-Bridge-Client";
export declare const ASHA_BROWSER_HOST_MAX_BRIDGE_CLIENTS = 8;
export declare const ASHA_BROWSER_HOST_COMMAND = "asha-browser-host --ui-root dist/ui --host 0.0.0.0 --port 5173";
export type NativeBrowserHostProviderScope = Record<string, NativeRustRuntimeBridgeProviderCandidate | null | undefined>;
export interface NativeBrowserHostProviderInstallOptions {
    readonly createRuntimeBridge?: () => RuntimeBridge | Promise<RuntimeBridge>;
    readonly globalScope?: NativeBrowserHostProviderScope;
}
export type NativeBrowserHostProviderStatus = {
    readonly status: 'rust_authority';
    readonly available: true;
    readonly diagnostics: readonly [];
    readonly profile: NativeRustRuntimeBridgeProviderProfile;
    readonly providerGlobal: string | null;
} | {
    readonly status: 'missing_rust_backend';
    readonly available: false;
    readonly diagnostics: readonly NativeRustRuntimeBridgeProviderDiagnostic[];
    readonly profile: NativeRustRuntimeBridgeProviderProfile;
    readonly providerGlobal: string | null;
};
export interface NativeBrowserHostServeOptions {
    readonly healthProject?: string;
    readonly host?: string;
    readonly port?: number;
    readonly uiRoot: string;
}
export interface NativeBrowserHostServer {
    readonly kind: 'asha.browser_host.native_runtime_provider.v0';
    readonly compatibilityVersion: typeof ASHA_BROWSER_HOST_COMPATIBILITY_VERSION;
    readonly url: string;
    readonly server: Server;
    readonly provider: NativeBrowserHostProviderStatus;
    readonly close: () => Promise<void>;
}
export interface NativeBrowserHostLaunchOptions extends NativeBrowserHostServeOptions {
    readonly provider?: NativeBrowserHostProviderInstallOptions;
}
export interface NativeBrowserHostCommandShape {
    readonly command: typeof ASHA_BROWSER_HOST_COMMAND;
    readonly packageRoot: '@asha/browser-host';
    readonly providerGlobal: `globalThis.${typeof ASHA_BROWSER_HOST_PROVIDER_GLOBAL}`;
    readonly providerKind: typeof ASHA_BROWSER_HOST_PROVIDER_KIND;
    readonly bootstrapOrder: 'install_provider_before_app_boot';
    readonly hostDefault: '0.0.0.0';
    readonly portDefault: 5173;
    readonly referenceFallback: false;
    readonly privateImportsRequired: false;
}
type NativeBrowserHostBridgeMethod = Extract<keyof RuntimeBridge, string>;
export declare const ASHA_BROWSER_HOST_BRIDGE_METHODS: readonly NativeBrowserHostBridgeMethod[];
export declare function describeNativeBrowserHostCommand(): NativeBrowserHostCommandShape;
export declare function installNativeBrowserHostProvider(options?: NativeBrowserHostProviderInstallOptions): NativeRustRuntimeBridgeProviderInstallation;
export declare function readNativeBrowserHostProviderStatus(globalScope?: NativeBrowserHostProviderScope): Promise<NativeBrowserHostProviderStatus>;
export declare function launchNativeBrowserHost(options: NativeBrowserHostLaunchOptions): Promise<NativeBrowserHostServer>;
export declare function startNativeBrowserHost(options: NativeBrowserHostServeOptions, provider: NativeBrowserHostProviderStatus, bridge?: RuntimeBridge, createRuntimeBridge?: () => RuntimeBridge | Promise<RuntimeBridge>): Promise<NativeBrowserHostServer>;
export {};
//# sourceMappingURL=host.d.ts.map