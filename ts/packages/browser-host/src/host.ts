import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http';
import { extname, resolve } from 'node:path';

import {
  createNativeRuntimeBridge,
  installNativeRustRuntimeBridgeProvider,
  resolveNativeRustRuntimeBridgeProvider,
  type NativeRustRuntimeBridgeProviderCandidate,
  type NativeRustRuntimeBridgeProviderDiagnostic,
  type NativeRustRuntimeBridgeProviderInstallation,
  type NativeRustRuntimeBridgeProviderProfile,
  type RuntimeBridge,
} from '@asha/runtime-bridge';

export const ASHA_BROWSER_HOST_COMPATIBILITY_VERSION = 'browser-host.v0';
export const ASHA_BROWSER_HOST_PROVIDER_GLOBAL = 'ashaRuntimeBridge';
export const ASHA_BROWSER_HOST_PROVIDER_KIND = 'asha.runtime_bridge.native_rust_provider.v1';
export const ASHA_BROWSER_HOST_COMMAND =
  'asha-browser-host --ui-root dist/ui --host 0.0.0.0 --port 5173';

export type NativeBrowserHostProviderScope = Record<
  string,
  NativeRustRuntimeBridgeProviderCandidate | null | undefined
>;

export interface NativeBrowserHostProviderInstallOptions {
  readonly createRuntimeBridge?: () => RuntimeBridge | Promise<RuntimeBridge>;
  readonly globalScope?: NativeBrowserHostProviderScope;
}

export type NativeBrowserHostProviderStatus =
  | {
      readonly status: 'rust_authority';
      readonly available: true;
      readonly diagnostics: readonly [];
      readonly profile: NativeRustRuntimeBridgeProviderProfile;
      readonly providerGlobal: string | null;
    }
  | {
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

export function describeNativeBrowserHostCommand(): NativeBrowserHostCommandShape {
  return {
    command: ASHA_BROWSER_HOST_COMMAND,
    packageRoot: '@asha/browser-host',
    providerGlobal: 'globalThis.ashaRuntimeBridge',
    providerKind: ASHA_BROWSER_HOST_PROVIDER_KIND,
    bootstrapOrder: 'install_provider_before_app_boot',
    hostDefault: '0.0.0.0',
    portDefault: 5173,
    referenceFallback: false,
    privateImportsRequired: false,
  };
}

export function installNativeBrowserHostProvider(
  options: NativeBrowserHostProviderInstallOptions = {},
): NativeRustRuntimeBridgeProviderInstallation {
  const globalScope = options.globalScope ?? defaultGlobalScope();
  return installNativeRustRuntimeBridgeProvider({
    globalScope,
    providerGlobalName: ASHA_BROWSER_HOST_PROVIDER_GLOBAL,
    createRuntimeBridge: options.createRuntimeBridge ?? createNativeRuntimeBridge,
  });
}

export async function readNativeBrowserHostProviderStatus(
  globalScope: NativeBrowserHostProviderScope = defaultGlobalScope(),
): Promise<NativeBrowserHostProviderStatus> {
  const resolution = await resolveNativeRustRuntimeBridgeProvider({
    globalScope,
    providerGlobalNames: [ASHA_BROWSER_HOST_PROVIDER_GLOBAL],
    providerKinds: [ASHA_BROWSER_HOST_PROVIDER_KIND],
  });
  if (resolution.status === 'available') {
    return {
      status: 'rust_authority',
      available: true,
      diagnostics: [],
      profile: resolution.profile,
      providerGlobal: resolution.providerGlobal,
    };
  }
  return {
    status: 'missing_rust_backend',
    available: false,
    diagnostics: resolution.diagnostics,
    profile: resolution.profile,
    providerGlobal: resolution.providerGlobal,
  };
}

export async function launchNativeBrowserHost(
  options: NativeBrowserHostLaunchOptions,
): Promise<NativeBrowserHostServer> {
  const providerScope = options.provider?.globalScope ?? defaultGlobalScope();
  installNativeBrowserHostProvider({
    globalScope: providerScope,
    ...(options.provider?.createRuntimeBridge !== undefined
      ? { createRuntimeBridge: options.provider.createRuntimeBridge }
      : {}),
  });
  const provider = await readNativeBrowserHostProviderStatus(providerScope);
  if (!provider.available) {
    const diagnostic = provider.diagnostics[0]?.message ?? 'native Rust RuntimeBridge provider unavailable';
    throw new Error(`ASHA browser host failed closed before serving UI: ${diagnostic}`);
  }
  return startNativeBrowserHost({ ...options }, provider);
}

export async function startNativeBrowserHost(
  options: NativeBrowserHostServeOptions,
  provider: NativeBrowserHostProviderStatus,
): Promise<NativeBrowserHostServer> {
  const host = options.host ?? '0.0.0.0';
  const port = options.port ?? 5173;
  const uiRoot = resolve(options.uiRoot);
  const server = createServer((request, response) => {
    void handleNativeBrowserHostRequest(request, response, options, provider, uiRoot);
  });
  await listen(server, port, host);
  const selectedPort = readSelectedPort(server, port);
  return {
    kind: 'asha.browser_host.native_runtime_provider.v0',
    compatibilityVersion: ASHA_BROWSER_HOST_COMPATIBILITY_VERSION,
    url: `http://${host}:${selectedPort}`,
    server,
    provider,
    close: () => closeServer(server),
  };
}

async function handleNativeBrowserHostRequest(
  request: IncomingMessage,
  response: ServerResponse,
  options: NativeBrowserHostServeOptions,
  provider: NativeBrowserHostProviderStatus,
  uiRoot: string,
): Promise<void> {
  response.setHeader('X-ASHA-Browser-Host', ASHA_BROWSER_HOST_COMPATIBILITY_VERSION);
  if (request.url === '/health') {
    sendJson(response, 200, {
      ok: true,
      project: options.healthProject ?? 'asha-game-project',
      compatibilityVersion: ASHA_BROWSER_HOST_COMPATIBILITY_VERSION,
    });
    return;
  }
  if (request.url === '/asha/browser-host/runtime-provider.json') {
    sendJson(response, provider.available ? 200 : 503, provider);
    return;
  }
  const assetPath = request.url === '/' ? '/index.html' : decodeURIComponent(request.url ?? '/index.html');
  await sendStaticAssetFromRoot(response, uiRoot, assetPath);
}

function defaultGlobalScope(): NativeBrowserHostProviderScope {
  return globalThis as unknown as NativeBrowserHostProviderScope;
}

function listen(server: Server, port: number, host: string): Promise<void> {
  return new Promise((resolveListen, rejectListen) => {
    const onError = (error: Error): void => {
      server.off('listening', onListening);
      rejectListen(error);
    };
    const onListening = (): void => {
      server.off('error', onError);
      resolveListen();
    };
    server.once('error', onError);
    server.once('listening', onListening);
    server.listen(port, host);
  });
}

function readSelectedPort(server: Server, fallbackPort: number): number {
  const address = server.address();
  if (typeof address === 'object' && address !== null) {
    return address.port;
  }
  return fallbackPort;
}

function closeServer(server: Server): Promise<void> {
  return new Promise((resolveClose, rejectClose) => {
    server.close((error) => {
      if (error) {
        rejectClose(error);
        return;
      }
      resolveClose();
    });
  });
}

async function sendStaticAssetFromRoot(
  response: ServerResponse,
  root: string,
  requestPath: string,
): Promise<void> {
  const normalizedPath = requestPath.replace(/^\/+/, '');
  const filePath = resolve(root, normalizedPath);
  if (!filePath.startsWith(root)) {
    response.writeHead(403);
    response.end('Forbidden');
    return;
  }

  try {
    const fileStat = await stat(filePath);
    if (!fileStat.isFile()) {
      throw new Error('not a file');
    }
    response.writeHead(200, { 'Content-Type': contentType(filePath) });
    createReadStream(filePath).pipe(response);
  } catch {
    response.writeHead(404);
    response.end('Not found');
  }
}

function sendJson(response: ServerResponse, statusCode: number, value: unknown): void {
  response.writeHead(statusCode, { 'Content-Type': 'application/json; charset=utf-8' });
  response.end(`${JSON.stringify(value, null, 2)}\n`);
}

function contentType(filePath: string): string {
  switch (extname(filePath)) {
    case '.css':
      return 'text/css; charset=utf-8';
    case '.html':
      return 'text/html; charset=utf-8';
    case '.js':
      return 'text/javascript; charset=utf-8';
    case '.json':
      return 'application/json; charset=utf-8';
    case '.toml':
      return 'text/plain; charset=utf-8';
    default:
      return 'application/octet-stream';
  }
}
