import { createReadStream } from 'node:fs';
import { readFile, stat } from 'node:fs/promises';
import { createServer } from 'node:http';
import { extname, isAbsolute, relative, resolve } from 'node:path';
import { createNativeRuntimeBridge, installNativeRustRuntimeBridgeProvider, MANIFEST_OPERATIONS, resolveNativeRustRuntimeBridgeProvider, } from '@asha/runtime-bridge';
export const ASHA_BROWSER_HOST_COMPATIBILITY_VERSION = 'browser-host.v0';
export const ASHA_BROWSER_HOST_PROVIDER_GLOBAL = 'ashaRuntimeBridge';
export const ASHA_BROWSER_HOST_PROVIDER_KIND = 'asha.runtime_bridge.native_rust_provider.v1';
export const ASHA_BROWSER_HOST_BRIDGE_CLIENT_HEADER = 'X-ASHA-Runtime-Bridge-Client';
export const ASHA_BROWSER_HOST_MAX_BRIDGE_CLIENTS = 8;
export const ASHA_BROWSER_HOST_COMMAND = 'asha-browser-host --ui-root dist/ui --host 0.0.0.0 --port 5173';
export const ASHA_BROWSER_HOST_BRIDGE_METHODS = MANIFEST_OPERATIONS.map(({ facadeMethod }) => facadeMethod);
export function describeNativeBrowserHostCommand() {
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
export function installNativeBrowserHostProvider(options = {}) {
    const globalScope = options.globalScope ?? defaultGlobalScope();
    return installNativeRustRuntimeBridgeProvider({
        globalScope,
        providerGlobalName: ASHA_BROWSER_HOST_PROVIDER_GLOBAL,
        createRuntimeBridge: options.createRuntimeBridge ?? createNativeRuntimeBridge,
    });
}
export async function readNativeBrowserHostProviderStatus(globalScope = defaultGlobalScope()) {
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
export async function launchNativeBrowserHost(options) {
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
export async function startNativeBrowserHost(options, provider, bridge, createRuntimeBridge) {
    const host = options.host ?? '0.0.0.0';
    const port = options.port ?? 5173;
    const uiRoot = resolve(options.uiRoot);
    const bridgePool = createNativeBrowserHostBridgePool(bridge, createRuntimeBridge);
    const server = createServer((request, response) => {
        void handleNativeBrowserHostRequest(request, response, options, provider, uiRoot, bridgePool).catch((error) => {
            handleNativeBrowserHostRequestFailure(response, error);
        });
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
            bridgePool.bridges.clear();
            server.removeAllListeners();
        },
    };
}
function handleNativeBrowserHostRequestFailure(response, error) {
    if (response.destroyed || response.writableEnded) {
        return;
    }
    if (response.headersSent) {
        response.end();
        return;
    }
    sendJson(response, 500, {
        error: {
            message: error instanceof Error ? error.message : String(error),
        },
    });
}
async function handleNativeBrowserHostRequest(request, response, options, provider, uiRoot, bridgePool) {
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
    if (request.url === '/asha/browser-host/native-provider.js') {
        sendText(response, 200, nativeBrowserHostProviderScript(), 'text/javascript; charset=utf-8');
        return;
    }
    if (request.url?.startsWith('/asha/browser-host/runtime-bridge/')) {
        await handleRuntimeBridgeInvocation(request, response, bridgePool);
        return;
    }
    const assetPath = request.url === '/' ? '/index.html' : decodeURIComponent(request.url ?? '/index.html');
    await sendStaticAssetFromRoot(response, uiRoot, assetPath, bridgePool.bridges.has('0'));
}
function defaultGlobalScope() {
    return globalThis;
}
function listen(server, port, host) {
    return new Promise((resolveListen, rejectListen) => {
        const onError = (error) => {
            server.off('listening', onListening);
            rejectListen(error);
        };
        const onListening = () => {
            server.off('error', onError);
            resolveListen();
        };
        server.once('error', onError);
        server.once('listening', onListening);
        server.listen(port, host);
    });
}
function readSelectedPort(server, fallbackPort) {
    const address = server.address();
    if (typeof address === 'object' && address !== null) {
        return address.port;
    }
    return fallbackPort;
}
function closeServer(server) {
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
async function sendStaticAssetFromRoot(response, root, requestPath, injectProviderScript) {
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
    }
    catch {
        response.writeHead(404);
        response.end('Not found');
    }
}
function isPathInsideRoot(root, filePath) {
    const relativePath = relative(root, filePath);
    return relativePath === '' || (!relativePath.startsWith('..') && !isAbsolute(relativePath));
}
async function handleRuntimeBridgeInvocation(request, response, bridgePool) {
    if (request.method !== 'POST') {
        sendJson(response, 405, { error: { message: 'RuntimeBridge host endpoint requires POST.' } });
        return;
    }
    const methodName = readRuntimeBridgeMethodName(request.url ?? '');
    if (methodName === null) {
        sendJson(response, 404, { error: { message: 'Unknown RuntimeBridge host operation.' } });
        return;
    }
    try {
        const bridge = await readNativeBrowserHostBridge(request, bridgePool);
        const invocation = await readInvocationBody(request);
        const method = bridge[methodName];
        const result = Reflect.apply(method, bridge, invocation.args ?? []);
        sendJson(response, 200, { result: result ?? null });
    }
    catch (error) {
        sendJson(response, 500, {
            error: {
                message: error instanceof Error ? error.message : String(error),
            },
        });
    }
}
function createNativeBrowserHostBridgePool(bridge, createRuntimeBridge) {
    const bridges = new Map();
    if (bridge !== undefined) {
        bridges.set('0', Promise.resolve(bridge));
    }
    return {
        bridges,
        ...(createRuntimeBridge === undefined ? {} : { createRuntimeBridge }),
    };
}
async function readNativeBrowserHostBridge(request, pool) {
    const header = request.headers[ASHA_BROWSER_HOST_BRIDGE_CLIENT_HEADER.toLowerCase()];
    const clientId = header === undefined ? '0' : readBridgeClientId(header);
    const existing = pool.bridges.get(clientId);
    if (existing !== undefined) {
        return await existing;
    }
    if (pool.createRuntimeBridge === undefined) {
        throw new Error(`RuntimeBridge client ${clientId} requested a new Session, but this host has no bridge factory.`);
    }
    const pending = Promise.resolve().then(pool.createRuntimeBridge);
    pool.bridges.set(clientId, pending);
    try {
        return await pending;
    }
    catch (error) {
        pool.bridges.delete(clientId);
        throw error;
    }
}
function readBridgeClientId(header) {
    if (Array.isArray(header)) {
        throw new Error('RuntimeBridge client identity must be a single header value.');
    }
    if (!/^(?:0|[1-9][0-9]*)$/u.test(header)) {
        throw new Error('RuntimeBridge client identity must be a canonical non-negative integer.');
    }
    const client = Number(header);
    if (!Number.isSafeInteger(client) || client >= ASHA_BROWSER_HOST_MAX_BRIDGE_CLIENTS) {
        throw new Error(`RuntimeBridge client identity exceeds the ${ASHA_BROWSER_HOST_MAX_BRIDGE_CLIENTS}-Session host limit.`);
    }
    return String(client);
}
function readRuntimeBridgeMethodName(url) {
    const prefix = '/asha/browser-host/runtime-bridge/';
    if (!url.startsWith(prefix)) {
        return null;
    }
    const candidate = decodeURIComponent(url.slice(prefix.length));
    if (ASHA_BROWSER_HOST_BRIDGE_METHODS.includes(candidate)) {
        return candidate;
    }
    return null;
}
function readInvocationBody(request) {
    return new Promise((resolveBody, rejectBody) => {
        const chunks = [];
        request.on('data', (chunk) => {
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
            const parsed = JSON.parse(text);
            if (parsed.args !== undefined && !Array.isArray(parsed.args)) {
                rejectBody(new Error('RuntimeBridge host invocation args must be an array.'));
                return;
            }
            resolveBody(parsed);
        });
    });
}
function sendJson(response, statusCode, value) {
    response.writeHead(statusCode, { 'Content-Type': 'application/json; charset=utf-8' });
    response.end(`${JSON.stringify(value, null, 2)}\n`);
}
function sendText(response, statusCode, value, contentTypeValue) {
    response.writeHead(statusCode, { 'Content-Type': contentTypeValue });
    response.end(value);
}
function injectNativeProviderScript(html) {
    const scriptTag = '<script src="/asha/browser-host/native-provider.js"></script>';
    if (html.includes('/asha/browser-host/native-provider.js')) {
        return html;
    }
    if (html.includes('</head>')) {
        return html.replace('</head>', `${scriptTag}\n</head>`);
    }
    return `${scriptTag}\n${html}`;
}
function nativeBrowserHostProviderScript() {
    return `(() => {
  const methods = ${JSON.stringify(ASHA_BROWSER_HOST_BRIDGE_METHODS)};
  let nextBridgeClient = 0;
  const invoke = (method, args, bridgeClient) => {
    const request = new XMLHttpRequest();
    request.open('POST', '/asha/browser-host/runtime-bridge/' + encodeURIComponent(method), false);
    request.setRequestHeader('Content-Type', 'application/json; charset=utf-8');
    if (bridgeClient !== null) {
      request.setRequestHeader('${ASHA_BROWSER_HOST_BRIDGE_CLIENT_HEADER}', String(bridgeClient));
    }
    request.send(JSON.stringify({ args }));
    const payload = JSON.parse(request.responseText || '{}');
    if (request.status < 200 || request.status >= 300) {
      throw new Error(payload.error?.message || 'ASHA native RuntimeBridge host invocation failed.');
    }
    return payload.result;
  };
  const createRuntimeBridge = () => {
    const bridgeClient = nextBridgeClient;
    nextBridgeClient += 1;
    const bridge = {};
    for (const method of methods) {
      bridge[method] = (...args) => invoke(method, args, bridgeClient);
    }
    return bridge;
  };
  globalThis.${ASHA_BROWSER_HOST_PROVIDER_GLOBAL} = {
    kind: '${ASHA_BROWSER_HOST_PROVIDER_KIND}',
    backend: 'native_rust',
    productAuthority: true,
    referenceFallback: false,
    createRuntimeBridge,
  };
})();\n`;
}
function contentType(filePath) {
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
//# sourceMappingURL=host.js.map