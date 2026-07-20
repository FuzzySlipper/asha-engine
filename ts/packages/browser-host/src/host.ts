import { randomBytes } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { readFile, stat } from 'node:fs/promises';
import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http';
import { extname, isAbsolute, relative, resolve } from 'node:path';

import {
  createNativeRuntimeBridge,
  installNativeRustRuntimeBridgeProvider,
  MANIFEST_OPERATIONS,
  resolveNativeRustRuntimeBridgeProvider,
  type NativeRustRuntimeBridgeProviderCandidate,
  type NativeRustRuntimeBridgeProviderDiagnostic,
  type NativeRustRuntimeBridgeProviderInstallation,
  type NativeRustRuntimeBridgeProviderProfile,
  type RuntimeBridge,
  RuntimeBridgeError,
} from '@asha/runtime-bridge';

export const ASHA_BROWSER_HOST_COMPATIBILITY_VERSION = 'browser-host.v0';
export const ASHA_BROWSER_HOST_PROVIDER_GLOBAL = 'ashaRuntimeBridge';
export const ASHA_BROWSER_HOST_PROVIDER_KIND = 'asha.runtime_bridge.native_rust_provider.v1';
export const ASHA_BROWSER_HOST_BRIDGE_CLIENT_HEADER = 'X-ASHA-Runtime-Bridge-Client';
export const ASHA_BROWSER_HOST_BRIDGE_SESSION_HEADER = 'X-ASHA-Runtime-Bridge-Session';
export const ASHA_BROWSER_HOST_MAX_BRIDGE_CLIENTS = 8;
export const ASHA_BROWSER_HOST_MAX_BROWSER_SESSIONS = 32;
export const ASHA_BROWSER_HOST_PROJECT_RESOURCE_CONTENT_TYPE = 'application/octet-stream';
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

export type NativeBrowserHostClientLifecycleStatus = 'active' | 'disconnected';

export interface NativeBrowserHostClientLifecycle {
  readonly compatibilityVersion: typeof ASHA_BROWSER_HOST_COMPATIBILITY_VERSION;
  readonly sessionId: string;
  status(): NativeBrowserHostClientLifecycleStatus;
  disconnect(): void;
}

export type NativeBrowserHostRuntimeBridge = RuntimeBridge & {
  readonly browserHostLifecycle: NativeBrowserHostClientLifecycle;
};

type NativeBrowserHostBridgeMethod = Extract<keyof RuntimeBridge, string>;

export const ASHA_BROWSER_HOST_BRIDGE_METHODS: readonly NativeBrowserHostBridgeMethod[] =
  MANIFEST_OPERATIONS.map(
    ({ facadeMethod }) => facadeMethod as NativeBrowserHostBridgeMethod,
  );

interface NativeBrowserHostBridgeInvocation {
  readonly args?: readonly unknown[];
}

const PROJECT_RESOURCE_STAGE_METHOD = 'stageRuntimeProjectSourceResource';
const PROJECT_RESOURCE_STAGE_MAX_INPUT_BYTES = MANIFEST_OPERATIONS.find(
  (operation) => operation.facadeMethod === PROJECT_RESOURCE_STAGE_METHOD,
)?.maxInputBytes ?? 0;

interface NativeBrowserHostBridgeEntry {
  readonly bridge: Promise<RuntimeBridge>;
  projectBundleLoaded: boolean;
}

interface NativeBrowserHostBridgePool {
  readonly bridges: Map<string, NativeBrowserHostBridgeEntry>;
  readonly issuedBrowserSessions: Set<string>;
  readonly retiredBrowserSessions: Set<string>;
  readonly closedBridgeClients: Set<string>;
  readonly createRuntimeBridge?: () => RuntimeBridge | Promise<RuntimeBridge>;
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
  const installation = installNativeBrowserHostProvider({
    globalScope: providerScope,
    ...(options.provider?.createRuntimeBridge !== undefined
      ? { createRuntimeBridge: options.provider.createRuntimeBridge }
      : {}),
  });
  const bridgeResolution = await resolveNativeRustRuntimeBridgeProvider({
    globalScope: providerScope,
    providerGlobalNames: [ASHA_BROWSER_HOST_PROVIDER_GLOBAL],
    providerKinds: [ASHA_BROWSER_HOST_PROVIDER_KIND],
  });
  if (bridgeResolution.status !== 'available') {
    const diagnostic = bridgeResolution.diagnostics[0]?.message ?? 'native Rust RuntimeBridge provider unavailable';
    throw new Error(`ASHA browser host failed closed before serving UI: ${diagnostic}`);
  }
  return startNativeBrowserHost({ ...options }, {
    status: 'rust_authority',
    available: true,
    diagnostics: [],
    profile: bridgeResolution.profile,
    providerGlobal: bridgeResolution.providerGlobal,
  }, bridgeResolution.bridge, installation.provider.createRuntimeBridge);
}

export async function startNativeBrowserHost(
  options: NativeBrowserHostServeOptions,
  provider: NativeBrowserHostProviderStatus,
  bridge?: RuntimeBridge,
  createRuntimeBridge?: () => RuntimeBridge | Promise<RuntimeBridge>,
): Promise<NativeBrowserHostServer> {
  const host = options.host ?? '0.0.0.0';
  const port = options.port ?? 5173;
  const uiRoot = resolve(options.uiRoot);
  const bridgePool = createNativeBrowserHostBridgePool(bridge, createRuntimeBridge);
  const server = createServer((request, response) => {
    void handleNativeBrowserHostRequest(request, response, options, provider, uiRoot, bridgePool).catch(
      (error: unknown) => {
        handleNativeBrowserHostRequestFailure(response, error);
      },
    );
  });
  await listen(server, port, host);
  const selectedPort = readSelectedPort(server, port);
  return {
    kind: 'asha.browser_host.native_runtime_provider.v0',
    compatibilityVersion: ASHA_BROWSER_HOST_COMPATIBILITY_VERSION,
    url: `http://${host}:${selectedPort}`,
    server,
    provider,
    close: async () => {
      await closeServer(server);
      try {
        await releaseAllNativeBrowserHostBridges(bridgePool);
      } finally {
        server.removeAllListeners();
      }
    },
  };
}

function handleNativeBrowserHostRequestFailure(response: ServerResponse, error: unknown): void {
  if (response.destroyed || response.writableEnded) {
    return;
  }
  if (response.headersSent) {
    response.end();
    return;
  }
  sendNativeBrowserHostError(response, error);
}

async function handleNativeBrowserHostRequest(
  request: IncomingMessage,
  response: ServerResponse,
  options: NativeBrowserHostServeOptions,
  provider: NativeBrowserHostProviderStatus,
  uiRoot: string,
  bridgePool: NativeBrowserHostBridgePool,
): Promise<void> {
  response.setHeader('X-ASHA-Browser-Host', ASHA_BROWSER_HOST_COMPATIBILITY_VERSION);
  const requestPath = readRequestPathname(request.url);
  if (requestPath === '/health') {
    sendJson(response, 200, {
      ok: true,
      project: options.healthProject ?? 'asha-game-project',
      compatibilityVersion: ASHA_BROWSER_HOST_COMPATIBILITY_VERSION,
    });
    return;
  }
  if (requestPath === '/asha/browser-host/runtime-provider.json') {
    sendJson(response, provider.available ? 200 : 503, provider);
    return;
  }
  if (requestPath === '/asha/browser-host/native-provider.js') {
    const browserSession = issueNativeBrowserHostSession(bridgePool);
    response.setHeader('Cache-Control', 'no-store');
    sendText(
      response,
      200,
      nativeBrowserHostProviderScript(browserSession),
      'text/javascript; charset=utf-8',
    );
    return;
  }
  if (requestPath === '/asha/browser-host/runtime-bridge/client/disconnect') {
    await handleRuntimeBridgeClientDisconnect(request, response, bridgePool);
    return;
  }
  if (requestPath.startsWith('/asha/browser-host/runtime-bridge/session/')) {
    await handleRuntimeBridgeSessionDisconnect(request, response, bridgePool, requestPath);
    return;
  }
  if (requestPath.startsWith('/asha/browser-host/runtime-bridge/')) {
    await handleRuntimeBridgeInvocation(request, response, bridgePool, requestPath);
    return;
  }
  const assetPath = requestPath === '/' ? '/index.html' : decodeURIComponent(requestPath);
  await sendStaticAssetFromRoot(response, uiRoot, assetPath, bridgePool.bridges.has('server:0'));
}

function readRequestPathname(requestTarget: string | undefined): string {
  const target = requestTarget ?? '/';
  const queryIndex = target.indexOf('?');
  const pathname = queryIndex < 0 ? target : target.slice(0, queryIndex);
  return pathname.length === 0 ? '/' : pathname;
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
  injectProviderScript: boolean,
): Promise<void> {
  const normalizedPath = requestPath.replace(/^\/+/, '');
  const filePath = resolve(root, normalizedPath);
  if (!isPathInsideRoot(root, filePath)) {
    response.writeHead(403);
    response.end('Forbidden');
    return;
  }

  try {
    const fileStat = await stat(filePath);
    if (!fileStat.isFile()) {
      throw new Error('not a file');
    }
    if (injectProviderScript && filePath.endsWith('.html')) {
      const html = await readFile(filePath, 'utf8');
      sendText(response, 200, injectNativeProviderScript(html), contentType(filePath));
      return;
    }
    response.writeHead(200, { 'Content-Type': contentType(filePath) });
    createReadStream(filePath).pipe(response);
  } catch {
    response.writeHead(404);
    response.end('Not found');
  }
}

function isPathInsideRoot(root: string, filePath: string): boolean {
  const relativePath = relative(root, filePath);
  return relativePath === '' || (!relativePath.startsWith('..') && !isAbsolute(relativePath));
}

async function handleRuntimeBridgeInvocation(
  request: IncomingMessage,
  response: ServerResponse,
  bridgePool: NativeBrowserHostBridgePool,
  requestPath: string,
): Promise<void> {
  if (request.method !== 'POST') {
    sendJson(response, 405, { error: { message: 'RuntimeBridge host endpoint requires POST.' } });
    return;
  }
  const methodName = readRuntimeBridgeMethodName(requestPath);
  if (methodName === null) {
    sendJson(response, 404, { error: { message: 'Unknown RuntimeBridge host operation.' } });
    return;
  }

  try {
    const identity = readNativeBrowserHostBridgeIdentity(request, bridgePool);
    const entry = await readNativeBrowserHostBridge(identity, bridgePool);
    const bridge = await entry.bridge;
    const invocation = methodName === PROJECT_RESOURCE_STAGE_METHOD
      ? await readProjectResourceInvocation(request)
      : await readInvocationBody(request);
    const method = bridge[methodName] as (...args: readonly unknown[]) => unknown;
    const result = Reflect.apply(method, bridge, invocation.args ?? []);
    if (methodName === 'loadProjectBundle') {
      entry.projectBundleLoaded = didProjectBundleLoad(result);
    } else if (methodName === 'unloadProjectBundle') {
      entry.projectBundleLoaded = false;
    }
    sendJson(response, 200, { result: result ?? null });
  } catch (error) {
    sendNativeBrowserHostError(response, error);
  }
}

function didProjectBundleLoad(result: unknown): boolean {
  if (typeof result !== 'object' || result === null) {
    return false;
  }
  const status = result as {
    readonly blocksLoad?: unknown;
    readonly loadedProjectBundle?: unknown;
  };
  return status.blocksLoad === false && status.loadedProjectBundle !== null
    && status.loadedProjectBundle !== undefined;
}

interface NativeBrowserHostBridgeIdentity {
  readonly browserSession: string;
  readonly clientId: string;
  readonly key: string;
}

function createNativeBrowserHostBridgePool(
  bridge: RuntimeBridge | undefined,
  createRuntimeBridge: (() => RuntimeBridge | Promise<RuntimeBridge>) | undefined,
): NativeBrowserHostBridgePool {
  const bridges = new Map<string, NativeBrowserHostBridgeEntry>();
  if (bridge !== undefined) {
    bridges.set('server:0', {
      bridge: Promise.resolve(bridge),
      projectBundleLoaded: false,
    });
  }
  return {
    bridges,
    issuedBrowserSessions: new Set(),
    retiredBrowserSessions: new Set(),
    closedBridgeClients: new Set(),
    ...(createRuntimeBridge === undefined ? {} : { createRuntimeBridge }),
  };
}

async function readNativeBrowserHostBridge(
  identity: NativeBrowserHostBridgeIdentity,
  pool: NativeBrowserHostBridgePool,
): Promise<NativeBrowserHostBridgeEntry> {
  if (pool.closedBridgeClients.has(identity.key)) {
    throw new RuntimeBridgeError(
      'not_initialized',
      `RuntimeBridge client ${identity.clientId} belongs to a closed or stale browser Session.`,
      { operation: 'browserHost.invoke', provenance: 'transport_loader' },
    );
  }
  const existing = pool.bridges.get(identity.key);
  if (existing !== undefined) {
    return existing;
  }
  if (pool.createRuntimeBridge === undefined) {
    throw new RuntimeBridgeError(
      'native_unavailable',
      `RuntimeBridge client ${identity.clientId} requested a new Session, but this host has no bridge factory.`,
      { operation: 'browserHost.createRuntimeBridge', provenance: 'transport_loader' },
    );
  }
  const pending = Promise.resolve().then(pool.createRuntimeBridge);
  const entry: NativeBrowserHostBridgeEntry = {
    bridge: pending,
    projectBundleLoaded: false,
  };
  pool.bridges.set(identity.key, entry);
  try {
    await pending;
    return entry;
  } catch (error) {
    pool.bridges.delete(identity.key);
    throw error;
  }
}

function readNativeBrowserHostBridgeIdentity(
  request: IncomingMessage,
  pool: NativeBrowserHostBridgePool,
): NativeBrowserHostBridgeIdentity {
  const clientHeader = request.headers[ASHA_BROWSER_HOST_BRIDGE_CLIENT_HEADER.toLowerCase()];
  const sessionHeader = request.headers[ASHA_BROWSER_HOST_BRIDGE_SESSION_HEADER.toLowerCase()];
  const clientId = clientHeader === undefined ? '0' : readBridgeClientId(clientHeader);
  const browserSession = readBridgeSessionId(sessionHeader, pool);
  return {
    browserSession,
    clientId,
    key: `${browserSession}:${clientId}`,
  };
}

function readBridgeClientId(header: string | string[]): string {
  if (Array.isArray(header)) {
    throw invalidBrowserHostIdentity('RuntimeBridge client identity must be a single header value.');
  }
  if (!/^(?:0|[1-9][0-9]*)$/u.test(header)) {
    throw invalidBrowserHostIdentity('RuntimeBridge client identity must be a canonical non-negative integer.');
  }
  const client = Number(header);
  if (!Number.isSafeInteger(client) || client >= ASHA_BROWSER_HOST_MAX_BRIDGE_CLIENTS) {
    throw invalidBrowserHostIdentity(
      `RuntimeBridge client identity exceeds the ${ASHA_BROWSER_HOST_MAX_BRIDGE_CLIENTS}-Session host limit.`,
    );
  }
  return String(client);
}

function readBridgeSessionId(
  header: string | string[] | undefined,
  pool: NativeBrowserHostBridgePool,
): string {
  if (header === undefined) {
    return 'server';
  }
  if (Array.isArray(header) || !/^[A-Za-z0-9_-]{32}$/u.test(header)) {
    throw invalidBrowserHostIdentity(
      'RuntimeBridge browser Session identity must be one host-issued opaque capability.',
    );
  }
  if (pool.retiredBrowserSessions.has(header)) {
    throw new RuntimeBridgeError(
      'not_initialized',
      `RuntimeBridge browser Session ${header} is closed or stale.`,
      { operation: 'browserHost.invoke', provenance: 'transport_loader' },
    );
  }
  if (!pool.issuedBrowserSessions.has(header)) {
    throw invalidBrowserHostIdentity(`RuntimeBridge browser Session ${header} was not issued by this host.`);
  }
  return header;
}

function invalidBrowserHostIdentity(message: string): RuntimeBridgeError {
  return new RuntimeBridgeError('invalid_input', message, {
    operation: 'browserHost.resolveSession',
    provenance: 'transport_loader',
  });
}

function issueNativeBrowserHostSession(pool: NativeBrowserHostBridgePool): string {
  if (pool.issuedBrowserSessions.size >= ASHA_BROWSER_HOST_MAX_BROWSER_SESSIONS) {
    throw new RuntimeBridgeError(
      'output_limit_exceeded',
      `Browser host has reached its ${ASHA_BROWSER_HOST_MAX_BROWSER_SESSIONS}-Session limit.`,
      { operation: 'browserHost.issueSession', provenance: 'transport_loader' },
    );
  }
  let session = randomBytes(24).toString('base64url');
  while (pool.issuedBrowserSessions.has(session) || pool.retiredBrowserSessions.has(session)) {
    session = randomBytes(24).toString('base64url');
  }
  pool.issuedBrowserSessions.add(session);
  return session;
}

async function handleRuntimeBridgeClientDisconnect(
  request: IncomingMessage,
  response: ServerResponse,
  pool: NativeBrowserHostBridgePool,
): Promise<void> {
  if (request.method !== 'POST') {
    sendJson(response, 405, { error: { message: 'RuntimeBridge client disconnect requires POST.' } });
    return;
  }
  try {
    const identity = readNativeBrowserHostBridgeIdentity(request, pool);
    const released = await releaseNativeBrowserHostBridge(pool, identity.key);
    sendJson(response, 200, {
      status: released ? 'disconnected' : 'already_disconnected',
      scope: 'client',
      browserSession: identity.browserSession,
      clientId: identity.clientId,
      released: released ? 1 : 0,
    });
  } catch (error) {
    sendNativeBrowserHostError(response, error);
  }
}

async function handleRuntimeBridgeSessionDisconnect(
  request: IncomingMessage,
  response: ServerResponse,
  pool: NativeBrowserHostBridgePool,
  requestPath: string,
): Promise<void> {
  if (request.method !== 'POST') {
    sendJson(response, 405, { error: { message: 'RuntimeBridge Session disconnect requires POST.' } });
    return;
  }
  const match = requestPath.match(
    /^\/asha\/browser-host\/runtime-bridge\/session\/([A-Za-z0-9_-]{32})\/disconnect$/u,
  );
  if (match === null || match === undefined) {
    sendJson(response, 404, { error: { message: 'Unknown RuntimeBridge Session lifecycle operation.' } });
    return;
  }
  const browserSession = match[1];
  if (browserSession === undefined) {
    sendJson(response, 404, { error: { message: 'RuntimeBridge Session identity is missing.' } });
    return;
  }
  try {
    const alreadyDisconnected = pool.retiredBrowserSessions.has(browserSession);
    const released = await releaseNativeBrowserHostSession(pool, browserSession);
    sendJson(response, 200, {
      status: alreadyDisconnected ? 'already_disconnected' : 'disconnected',
      scope: 'browser_session',
      browserSession,
      released,
    });
  } catch (error) {
    sendNativeBrowserHostError(response, error);
  }
}

async function releaseNativeBrowserHostSession(
  pool: NativeBrowserHostBridgePool,
  browserSession: string,
): Promise<number> {
  if (pool.retiredBrowserSessions.has(browserSession)) {
    return 0;
  }
  if (!pool.issuedBrowserSessions.delete(browserSession)) {
    throw invalidBrowserHostIdentity(`RuntimeBridge browser Session ${browserSession} was not issued by this host.`);
  }
  pool.retiredBrowserSessions.add(browserSession);
  const keys = [...pool.bridges.keys()].filter((key) => key.startsWith(`${browserSession}:`));
  let released = 0;
  const failures: string[] = [];
  for (const key of keys) {
    try {
      if (await releaseNativeBrowserHostBridge(pool, key)) {
        released += 1;
      }
    } catch (error) {
      failures.push(error instanceof Error ? error.message : String(error));
    }
  }
  if (failures.length > 0) {
    throw new RuntimeBridgeError(
      'internal',
      'RuntimeBridge browser Session cleanup reported one or more unload failures.',
      {
        operation: 'browserHost.disconnectSession',
        details: failures,
        provenance: 'transport_loader',
      },
    );
  }
  return released;
}

async function releaseNativeBrowserHostBridge(
  pool: NativeBrowserHostBridgePool,
  key: string,
): Promise<boolean> {
  if (pool.closedBridgeClients.has(key)) {
    return false;
  }
  pool.closedBridgeClients.add(key);
  const entry = pool.bridges.get(key);
  pool.bridges.delete(key);
  if (entry === undefined) {
    return false;
  }
  const bridge = await entry.bridge;
  if (entry.projectBundleLoaded) {
    bridge.unloadProjectBundle();
    entry.projectBundleLoaded = false;
  }
  return true;
}

async function releaseAllNativeBrowserHostBridges(pool: NativeBrowserHostBridgePool): Promise<void> {
  const keys = [...pool.bridges.keys()];
  const failures: string[] = [];
  for (const key of keys) {
    try {
      await releaseNativeBrowserHostBridge(pool, key);
    } catch (error) {
      failures.push(error instanceof Error ? error.message : String(error));
    }
  }
  pool.issuedBrowserSessions.clear();
  if (failures.length > 0) {
    throw new RuntimeBridgeError(
      'internal',
      'Browser host shutdown reported one or more RuntimeBridge unload failures.',
      {
        operation: 'browserHost.shutdown',
        details: failures,
        provenance: 'transport_loader',
      },
    );
  }
}

function readRuntimeBridgeMethodName(url: string): NativeBrowserHostBridgeMethod | null {
  const prefix = '/asha/browser-host/runtime-bridge/';
  if (!url.startsWith(prefix)) {
    return null;
  }
  const candidate = decodeURIComponent(url.slice(prefix.length));
  if ((ASHA_BROWSER_HOST_BRIDGE_METHODS as readonly string[]).includes(candidate)) {
    return candidate as NativeBrowserHostBridgeMethod;
  }
  return null;
}

function readInvocationBody(request: IncomingMessage): Promise<NativeBrowserHostBridgeInvocation> {
  return new Promise((resolveBody, rejectBody) => {
    const chunks: Buffer[] = [];
    request.on('data', (chunk: Buffer) => {
      chunks.push(chunk);
      const totalBytes = chunks.reduce((total, item) => total + item.byteLength, 0);
      if (totalBytes > 1_000_000) {
        rejectBody(new Error('RuntimeBridge host invocation exceeded 1MB.'));
        request.destroy();
      }
    });
    request.on('error', rejectBody);
    request.on('end', () => {
      const text = Buffer.concat(chunks).toString('utf8');
      if (text.length === 0) {
        resolveBody({});
        return;
      }
      const parsed = JSON.parse(text) as NativeBrowserHostBridgeInvocation;
      if (parsed.args !== undefined && !Array.isArray(parsed.args)) {
        rejectBody(new Error('RuntimeBridge host invocation args must be an array.'));
        return;
      }
      resolveBody(parsed);
    });
  });
}

async function readProjectResourceInvocation(
  request: IncomingMessage,
): Promise<NativeBrowserHostBridgeInvocation> {
  if (request.headers['content-type'] !== ASHA_BROWSER_HOST_PROJECT_RESOURCE_CONTENT_TYPE) {
    throw new RuntimeBridgeError(
      'invalid_input',
      `RuntimeBridge project resources require ${ASHA_BROWSER_HOST_PROJECT_RESOURCE_CONTENT_TYPE}.`,
      { operation: 'browserHost.stageProjectResource', provenance: 'transport_loader' },
    );
  }
  const target = new URL(request.url ?? '/', 'http://asha-browser-host.invalid');
  const fields = [...target.searchParams.keys()];
  if (
    fields.length !== 2
    || !fields.includes('generation')
    || !fields.includes('path')
  ) {
    throw new RuntimeBridgeError(
      'invalid_input',
      'RuntimeBridge project resource metadata must contain exactly generation and path.',
      { operation: 'browserHost.stageProjectResource', provenance: 'transport_loader' },
    );
  }
  const generationText = target.searchParams.get('generation') ?? '';
  if (!/^(?:0|[1-9][0-9]*)$/u.test(generationText)) {
    throw new RuntimeBridgeError(
      'invalid_input',
      'RuntimeBridge project resource generation must be a canonical non-negative integer.',
      { operation: 'browserHost.stageProjectResource', provenance: 'transport_loader' },
    );
  }
  const generation = Number(generationText);
  const path = target.searchParams.get('path') ?? '';
  if (!Number.isSafeInteger(generation) || path.length === 0) {
    throw new RuntimeBridgeError(
      'invalid_input',
      'RuntimeBridge project resource metadata is outside its bounded contract.',
      { operation: 'browserHost.stageProjectResource', provenance: 'transport_loader' },
    );
  }
  const bytes = await readBoundedBinaryBody(request, PROJECT_RESOURCE_STAGE_MAX_INPUT_BYTES);
  return { args: [{ generation, path, bytes }] };
}

function readBoundedBinaryBody(request: IncomingMessage, maxBytes: number): Promise<Uint8Array> {
  const declaredLength = request.headers['content-length'];
  if (
    typeof declaredLength === 'string'
    && (/^(?:0|[1-9][0-9]*)$/u.test(declaredLength) === false
      || Number(declaredLength) > maxBytes)
  ) {
    return Promise.reject(new RuntimeBridgeError(
      'invalid_input',
      `RuntimeBridge project resource body exceeds ${maxBytes} bytes.`,
      { operation: 'browserHost.stageProjectResource', provenance: 'transport_loader' },
    ));
  }
  return new Promise((resolveBody, rejectBody) => {
    const chunks: Buffer[] = [];
    let totalBytes = 0;
    request.on('data', (chunk: Buffer) => {
      totalBytes += chunk.byteLength;
      if (totalBytes > maxBytes) {
        rejectBody(new RuntimeBridgeError(
          'invalid_input',
          `RuntimeBridge project resource body exceeds ${maxBytes} bytes.`,
          { operation: 'browserHost.stageProjectResource', provenance: 'transport_loader' },
        ));
        request.destroy();
        return;
      }
      chunks.push(chunk);
    });
    request.on('error', rejectBody);
    request.on('end', () => {
      const body = Buffer.concat(chunks, totalBytes);
      resolveBody(new Uint8Array(body.buffer, body.byteOffset, body.byteLength).slice());
    });
  });
}

function sendNativeBrowserHostError(response: ServerResponse, error: unknown): void {
  const classified = error instanceof RuntimeBridgeError
    ? error
    : new RuntimeBridgeError(
        'internal',
        error instanceof Error ? error.message : String(error),
        { provenance: 'transport_loader' },
      );
  sendJson(response, 500, {
    error: {
      kind: classified.kind,
      message: classified.message,
      operation: classified.operation,
      path: classified.path,
      retryable: classified.retryable,
      details: classified.details,
      provenance: classified.provenance,
    },
  });
}

function sendJson(response: ServerResponse, statusCode: number, value: unknown): void {
  response.writeHead(statusCode, { 'Content-Type': 'application/json; charset=utf-8' });
  response.end(`${JSON.stringify(value, null, 2)}\n`);
}

function sendText(response: ServerResponse, statusCode: number, value: string, contentTypeValue: string): void {
  response.writeHead(statusCode, { 'Content-Type': contentTypeValue });
  response.end(value);
}

function injectNativeProviderScript(html: string): string {
  const scriptTag = '<script src="/asha/browser-host/native-provider.js"></script>';
  if (html.includes('/asha/browser-host/native-provider.js')) {
    return html;
  }
  if (html.includes('</head>')) {
    return html.replace('</head>', `${scriptTag}\n</head>`);
  }
  return `${scriptTag}\n${html}`;
}

function nativeBrowserHostProviderScript(browserSession: string): string {
  return `(() => {
  const methods = ${JSON.stringify(ASHA_BROWSER_HOST_BRIDGE_METHODS)};
  const browserSession = '${browserSession}';
  const disconnectedClients = new Set();
  let nextBridgeClient = 0;
  const classifiedError = (payload, fallback) => {
    const detail = payload.error || {};
    const error = new Error(detail.message || fallback);
    error.name = 'RuntimeBridgeError';
    Object.assign(error, {
      kind: detail.kind || 'internal',
      operation: detail.operation || null,
      path: detail.path || null,
      retryable: detail.retryable === true,
      details: Array.isArray(detail.details) ? detail.details : [],
      provenance: detail.provenance || 'transport_loader',
    });
    return error;
  };
  const invoke = (method, args, bridgeClient) => {
    if (disconnectedClients.has(bridgeClient)) {
      throw classifiedError({ error: {
        kind: 'not_initialized',
        message: 'RuntimeBridge browser client is disconnected.',
        operation: method,
        provenance: 'transport_loader',
      } }, 'RuntimeBridge browser client is disconnected.');
    }
    const projectResource = method === '${PROJECT_RESOURCE_STAGE_METHOD}';
    const resourceInput = projectResource ? args[0] : null;
    if (projectResource && (
      args.length !== 1
      || resourceInput === null
      || typeof resourceInput !== 'object'
      || !Number.isSafeInteger(resourceInput.generation)
      || resourceInput.generation < 0
      || typeof resourceInput.path !== 'string'
      || resourceInput.path.length === 0
      || !ArrayBuffer.isView(resourceInput.bytes)
      || Object.prototype.toString.call(resourceInput.bytes) !== '[object Uint8Array]'
    )) {
      throw classifiedError({ error: {
        kind: 'invalid_input',
        message: 'RuntimeBridge project resource requires generation, path, and Uint8Array bytes.',
        operation: method,
        provenance: 'transport_loader',
      } }, 'Invalid RuntimeBridge project resource input.');
    }
    const query = projectResource
      ? '?generation=' + encodeURIComponent(String(resourceInput.generation))
        + '&path=' + encodeURIComponent(resourceInput.path)
      : '';
    const request = new XMLHttpRequest();
    request.open('POST', '/asha/browser-host/runtime-bridge/' + encodeURIComponent(method) + query, false);
    request.setRequestHeader('Content-Type', projectResource
      ? '${ASHA_BROWSER_HOST_PROJECT_RESOURCE_CONTENT_TYPE}'
      : 'application/json; charset=utf-8');
    request.setRequestHeader('${ASHA_BROWSER_HOST_BRIDGE_SESSION_HEADER}', browserSession);
    request.setRequestHeader('${ASHA_BROWSER_HOST_BRIDGE_CLIENT_HEADER}', String(bridgeClient));
    request.send(projectResource ? resourceInput.bytes : JSON.stringify({ args }));
    const payload = JSON.parse(request.responseText || '{}');
    if (request.status < 200 || request.status >= 300) {
      throw classifiedError(payload, 'ASHA native RuntimeBridge host invocation failed.');
    }
    return payload.result;
  };
  const disconnect = (bridgeClient) => {
    if (disconnectedClients.has(bridgeClient)) {
      return;
    }
    const request = new XMLHttpRequest();
    request.open('POST', '/asha/browser-host/runtime-bridge/client/disconnect', false);
    request.setRequestHeader('${ASHA_BROWSER_HOST_BRIDGE_SESSION_HEADER}', browserSession);
    request.setRequestHeader('${ASHA_BROWSER_HOST_BRIDGE_CLIENT_HEADER}', String(bridgeClient));
    try {
      request.send('{}');
      const payload = JSON.parse(request.responseText || '{}');
      if (request.status < 200 || request.status >= 300) {
        throw classifiedError(payload, 'ASHA native RuntimeBridge disconnect failed.');
      }
    } finally {
      disconnectedClients.add(bridgeClient);
    }
  };
  const createRuntimeBridge = () => {
    if (nextBridgeClient >= ${ASHA_BROWSER_HOST_MAX_BRIDGE_CLIENTS}) {
      throw classifiedError({ error: {
        kind: 'output_limit_exceeded',
        message: 'RuntimeBridge browser client limit exceeded.',
        operation: 'browserHost.createRuntimeBridge',
        provenance: 'transport_loader',
      } }, 'RuntimeBridge browser client limit exceeded.');
    }
    const bridgeClient = nextBridgeClient;
    nextBridgeClient += 1;
    const bridge = {};
    for (const method of methods) {
      bridge[method] = (...args) => invoke(method, args, bridgeClient);
    }
    Object.defineProperty(bridge, 'browserHostLifecycle', {
      enumerable: false,
      value: Object.freeze({
        compatibilityVersion: '${ASHA_BROWSER_HOST_COMPATIBILITY_VERSION}',
        sessionId: browserSession,
        status: () => disconnectedClients.has(bridgeClient) ? 'disconnected' : 'active',
        disconnect: () => disconnect(bridgeClient),
      }),
    });
    return bridge;
  };
  if (typeof globalThis.addEventListener === 'function') {
    globalThis.addEventListener('pagehide', () => {
      const path = '/asha/browser-host/runtime-bridge/session/' + browserSession + '/disconnect';
      for (let client = 0; client < nextBridgeClient; client += 1) {
        disconnectedClients.add(client);
      }
      if (globalThis.navigator && typeof globalThis.navigator.sendBeacon === 'function') {
        globalThis.navigator.sendBeacon(path, '{}');
      } else if (typeof globalThis.fetch === 'function') {
        void globalThis.fetch(path, { method: 'POST', body: '{}', keepalive: true });
      }
    }, { once: true });
  }
  globalThis.${ASHA_BROWSER_HOST_PROVIDER_GLOBAL} = {
    kind: '${ASHA_BROWSER_HOST_PROVIDER_KIND}',
    backend: 'native_rust',
    productAuthority: true,
    referenceFallback: false,
    browserHostCompatibilityVersion: '${ASHA_BROWSER_HOST_COMPATIBILITY_VERSION}',
    browserHostSessionId: browserSession,
    createRuntimeBridge,
  };
})();\n`;
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
