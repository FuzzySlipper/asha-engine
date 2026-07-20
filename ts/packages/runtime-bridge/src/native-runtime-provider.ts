import {
  RuntimeBridgeError,
  type RuntimeBridge,
} from './bridge.js';

export const NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND = 'asha.runtime_bridge.native_rust_provider.v1';

export const NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_GLOBALS = [
  'ashaRuntimeBridge',
] as const;
export type NativeRustRuntimeBridgeProviderGlobalName = typeof NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_GLOBALS[number];

export const NATIVE_RUST_RUNTIME_BRIDGE_REQUIRED_METHODS = [
  'initializeEngine',
  'beginRuntimeProjectSourceResources',
  'stageRuntimeProjectSourceResource',
  'admitRuntimeProjectSourceBatch',
  'loadRuntimeProject',
  'readActiveRuntimeProjectContent',
  'closeRuntimeProject',
  'createCamera',
  'applyCollisionConstrainedCameraInput',
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
] as const;

export type NativeRustRuntimeBridgeProviderKind = typeof NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND;

export type NativeRustRuntimeBridgeProviderDiagnosticCode =
  | 'missing_rust_runtime_backend'
  | 'invalid_rust_runtime_provider'
  | 'missing_runtime_bridge'
  | 'missing_runtime_bridge_operation'
  | 'non_native_runtime_authority';

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
  readonly providerGlobalNames?: readonly NativeRustRuntimeBridgeProviderGlobalName[];
  readonly providerKinds?: readonly NativeRustRuntimeBridgeProviderKind[];
}

interface NativeRustRuntimeBridgeProviderGlobalThis {
  readonly ashaRuntimeBridge?: NativeRustRuntimeBridgeProviderCandidate | null;
}

export type NativeRustRuntimeBridgeProviderResolution =
  | {
      readonly status: 'available';
      readonly provider: NativeRustRuntimeBridgeProvider;
      readonly bridge: RuntimeBridge;
      readonly providerGlobal: string | null;
      readonly profile: NativeRustRuntimeBridgeProviderProfile;
      readonly diagnostics: readonly [];
    }
  | {
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

export type NativeRustRuntimeAuthorityValidation =
  | {
      readonly ok: true;
      readonly diagnostics: readonly [];
    }
  | {
      readonly ok: false;
      readonly diagnostics: readonly NativeRustRuntimeBridgeProviderDiagnostic[];
    };

export function createNativeRustRuntimeBridgeProvider(
  request: CreateNativeRustRuntimeBridgeProviderRequest,
): NativeRustRuntimeBridgeProvider {
  const hasBridge = request.bridge !== undefined;
  const hasFactory = request.createRuntimeBridge !== undefined;
  if (hasBridge === hasFactory) {
    throw new RuntimeBridgeError(
      'invalid_input',
      'Native RuntimeBridge provider requires exactly one bridge or createRuntimeBridge factory.',
    );
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

export function installNativeRustRuntimeBridgeProvider(
  request: InstallNativeRustRuntimeBridgeProviderRequest,
): NativeRustRuntimeBridgeProviderInstallation {
  const provider = request.provider ?? createNativeRustRuntimeBridgeProvider(request);
  if (!isNativeRustRuntimeBridgeProvider(provider, [NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND])) {
    throw new RuntimeBridgeError(
      'invalid_input',
      'Standalone host must install the public native Rust RuntimeBridge provider contract with product authority and no reference fallback.',
    );
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

export async function resolveNativeRustRuntimeBridgeProvider(
  request: ResolveNativeRustRuntimeBridgeProviderRequest = {},
): Promise<NativeRustRuntimeBridgeProviderResolution> {
  const lookup = readProviderCandidate(request);
  const providerKinds = request.providerKinds ?? [NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND];
  const fallbackKind = providerKinds[0] ?? NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_KIND;
  const invalidProfile = nativeRustRuntimeBridgeProviderProfile(lookup.providerGlobal, fallbackKind);
  if (lookup.provider === null) {
    return unavailableNativeRustRuntimeBridgeProvider(
      lookup.providerGlobal,
      fallbackKind,
      'missing_rust_runtime_backend',
      'ASHA requires a public native Rust RuntimeBridge provider; static host does not fall back to reference authority.',
    );
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
    const missingOperation = NATIVE_RUST_RUNTIME_BRIDGE_REQUIRED_METHODS.find(
      (method) => typeof bridge[method] !== 'function',
    );
    if (missingOperation !== undefined) {
      return unavailableNativeRustRuntimeBridgeProvider(
        lookup.providerGlobal,
        lookup.provider.kind,
        'missing_runtime_bridge_operation',
        `RuntimeBridge provider is missing required operation: ${missingOperation}`,
      );
    }
    return {
      status: 'available',
      provider: lookup.provider,
      bridge,
      providerGlobal: lookup.providerGlobal,
      profile: nativeRustRuntimeBridgeProviderProfile(lookup.providerGlobal, lookup.provider.kind),
      diagnostics: [],
    };
  } catch (error) {
    return unavailableNativeRustRuntimeBridgeProvider(
      lookup.providerGlobal,
      lookup.provider.kind,
      'missing_runtime_bridge',
      error instanceof Error ? error.message : String(error),
    );
  }
}

export function validateNativeRustRuntimeBridgeAuthority(
  input: NativeRustRuntimeAuthorityInput,
): NativeRustRuntimeAuthorityValidation {
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

export function assertNativeRustRuntimeBridgeAuthority(input: NativeRustRuntimeAuthorityInput): void {
  const validation = validateNativeRustRuntimeBridgeAuthority(input);
  if (!validation.ok) {
    throw new RuntimeBridgeError('invalid_input', validation.diagnostics[0]?.message ?? 'non-native RuntimeBridge authority rejected');
  }
}

function readProviderCandidate(request: ResolveNativeRustRuntimeBridgeProviderRequest): {
  readonly provider: NativeRustRuntimeBridgeProviderCandidate | null;
  readonly providerGlobal: string | null;
} {
  if (request.provider !== undefined) {
    return { provider: request.provider, providerGlobal: null };
  }
  const scope = request.globalScope ?? defaultNativeRustRuntimeBridgeProviderGlobals();
  for (const name of request.providerGlobalNames ?? NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_GLOBALS) {
    if (!(NATIVE_RUST_RUNTIME_BRIDGE_PROVIDER_GLOBALS as readonly string[]).includes(name)) {
      continue;
    }
    if (scope[name] !== undefined && scope[name] !== null) {
      return { provider: scope[name], providerGlobal: `globalThis.${name}` };
    }
  }
  return { provider: null, providerGlobal: null };
}

function defaultNativeRustRuntimeBridgeProviderGlobals(): Record<string, NativeRustRuntimeBridgeProviderCandidate | null | undefined> {
  const globals = globalThis as typeof globalThis & NativeRustRuntimeBridgeProviderGlobalThis;
  return {
    ashaRuntimeBridge: globals.ashaRuntimeBridge,
  };
}

function defaultNativeRustRuntimeBridgeProviderGlobalTarget(): Record<string, NativeRustRuntimeBridgeProviderCandidate | null | undefined> {
  return globalThis as unknown as Record<string, NativeRustRuntimeBridgeProviderCandidate | null | undefined>;
}

function isNativeRustRuntimeBridgeProvider(
  value: NativeRustRuntimeBridgeProviderCandidate | null,
  providerKinds: readonly NativeRustRuntimeBridgeProviderKind[],
): value is NativeRustRuntimeBridgeProvider {
  return value !== null
    && isNativeRustRuntimeBridgeProviderKind(value.kind, providerKinds)
    && value.backend === 'native_rust'
    && value.productAuthority === true
    && value.referenceFallback === false;
}

function isNativeRustRuntimeBridgeProviderKind(
  value: string | undefined,
  providerKinds: readonly NativeRustRuntimeBridgeProviderKind[],
): value is NativeRustRuntimeBridgeProviderKind {
  return value !== undefined && (providerKinds as readonly string[]).includes(value);
}

async function readProvidedRuntimeBridge(provider: NativeRustRuntimeBridgeProvider): Promise<RuntimeBridge> {
  const candidate = typeof provider.createRuntimeBridge === 'function'
      ? provider.createRuntimeBridge()
      : provider.bridge;
  const bridge = await candidate;
  if (bridge === undefined || bridge === null || typeof bridge !== 'object') {
    throw new RuntimeBridgeError('invalid_input', 'RuntimeBridge provider did not return a bridge object');
  }
  return bridge as RuntimeBridge;
}

function unavailableNativeRustRuntimeBridgeProvider(
  providerGlobal: string | null,
  providerContract: NativeRustRuntimeBridgeProviderKind,
  code: NativeRustRuntimeBridgeProviderDiagnosticCode,
  message: string,
): NativeRustRuntimeBridgeProviderResolution {
  return {
    status: 'unavailable',
    provider: null,
    bridge: null,
    providerGlobal,
    profile: nativeRustRuntimeBridgeProviderProfile(providerGlobal, providerContract),
    diagnostics: [{ code, severity: 'error', message }],
  };
}

function nativeRustRuntimeBridgeProviderProfile(
  providerGlobal: string | null,
  providerContract: NativeRustRuntimeBridgeProviderKind,
): NativeRustRuntimeBridgeProviderProfile {
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
