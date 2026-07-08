import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { createServer } from 'node:http';
import { extname, resolve } from 'node:path';
import { createNativeRuntimeBridge, installNativeRustRuntimeBridgeProvider, resolveNativeRustRuntimeBridgeProvider, } from '@asha/runtime-bridge';
export const ASHA_BROWSER_HOST_COMPATIBILITY_VERSION = 'browser-host.v0';
export const ASHA_BROWSER_HOST_PROVIDER_GLOBAL = 'ashaRuntimeBridge';
export const ASHA_BROWSER_HOST_PROVIDER_KIND = 'asha.runtime_bridge.native_rust_provider.v1';
export const ASHA_BROWSER_HOST_COMMAND = 'asha-browser-host --ui-root dist/ui --host 0.0.0.0 --port 5173';
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
export async function startNativeBrowserHost(options, provider) {
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
async function handleNativeBrowserHostRequest(request, response, options, provider, uiRoot) {
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
async function sendStaticAssetFromRoot(response, root, requestPath) {
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
    }
    catch {
        response.writeHead(404);
        response.end('Not found');
    }
}
function sendJson(response, statusCode, value) {
    response.writeHead(statusCode, { 'Content-Type': 'application/json; charset=utf-8' });
    response.end(`${JSON.stringify(value, null, 2)}\n`);
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