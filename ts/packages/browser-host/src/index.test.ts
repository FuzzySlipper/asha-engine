import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { request } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';
import assert from 'node:assert/strict';

import {
  MANIFEST_OPERATIONS,
  NativeRuntimeBridge,
  type RuntimeBridge,
} from '@asha/runtime-bridge';

import {
  ASHA_BROWSER_HOST_BRIDGE_METHODS,
  ASHA_BROWSER_HOST_BRIDGE_CLIENT_HEADER,
  ASHA_BROWSER_HOST_BRIDGE_SESSION_HEADER,
  ASHA_BROWSER_HOST_COMMAND,
  ASHA_BROWSER_HOST_COMPATIBILITY_VERSION,
  describeNativeBrowserHostCommand,
  installNativeBrowserHostProvider,
  launchNativeBrowserHost,
  readNativeBrowserHostProviderStatus,
} from './index.js';

void test('browser host command shape documents public native provider boot', () => {
  assert.deepEqual(describeNativeBrowserHostCommand(), {
    command: ASHA_BROWSER_HOST_COMMAND,
    packageRoot: '@asha/browser-host',
    providerGlobal: 'globalThis.ashaRuntimeBridge',
    providerKind: 'asha.runtime_bridge.native_rust_provider.v1',
    bootstrapOrder: 'install_provider_before_app_boot',
    hostDefault: '0.0.0.0',
    portDefault: 5173,
    referenceFallback: false,
    privateImportsRequired: false,
  });
});

void test('browser host RPC surface follows every generated RuntimeBridge manifest operation', () => {
  assert.deepEqual(
    ASHA_BROWSER_HOST_BRIDGE_METHODS,
    MANIFEST_OPERATIONS.map(({ facadeMethod }) => facadeMethod),
  );
});

void test('browser host installs a public native provider and reports rust authority status', async () => {
  const globalScope = {};
  const installation = installNativeBrowserHostProvider({
    globalScope,
    createRuntimeBridge: createFakeRuntimeBridge,
  });

  assert.equal(installation.providerGlobal, 'globalThis.ashaRuntimeBridge');
  assert.equal(installation.profile.providerContract, 'asha.runtime_bridge.native_rust_provider.v1');
  assert.equal(installation.profile.referenceFallback, false);

  const status = await readNativeBrowserHostProviderStatus(globalScope);
  assert.equal(status.status, 'rust_authority');
  assert.equal(status.available, true);
  assert.equal(status.profile.providerGlobal, 'globalThis.ashaRuntimeBridge');
});

void test('browser host fails closed for missing and spoofed providers', async () => {
  const missing = await readNativeBrowserHostProviderStatus({});
  assert.equal(missing.status, 'missing_rust_backend');
  assert.equal(missing.available, false);
  assert.equal(missing.diagnostics[0]?.code, 'missing_rust_runtime_backend');

  const spoofed = await readNativeBrowserHostProviderStatus({
    ashaRuntimeBridge: {
      kind: 'asha.runtime_bridge.native_rust_provider.v1',
      backend: 'reference_bridge',
      productAuthority: true,
      referenceFallback: true,
      createRuntimeBridge: createFakeRuntimeBridge,
    },
  });
  assert.equal(spoofed.status, 'missing_rust_backend');
  assert.equal(spoofed.available, false);
  assert.equal(spoofed.diagnostics[0]?.code, 'invalid_rust_runtime_provider');
});

void test('browser host serves a downstream UI root with provider status evidence', async () => {
  const tempRoot = await mkdtemp(join(tmpdir(), 'asha-browser-host-'));
  try {
    await writeFile(join(tempRoot, 'index.html'), '<!doctype html><title>ASHA demo</title>');
    const globalScope = {};
    const host = await launchNativeBrowserHost({
      uiRoot: tempRoot,
      host: '127.0.0.1',
      port: 0,
      healthProject: 'asha-demo',
      provider: {
        globalScope,
        createRuntimeBridge: createFakeRuntimeBridge,
      },
    });
    try {
      assert.equal(host.kind, 'asha.browser_host.native_runtime_provider.v0');
      assert.equal(host.compatibilityVersion, ASHA_BROWSER_HOST_COMPATIBILITY_VERSION);
      assert.equal(host.provider.status, 'rust_authority');

      const health = await readJson(`${host.url}/health`);
      assert.deepEqual(health, {
        ok: true,
        project: 'asha-demo',
        compatibilityVersion: ASHA_BROWSER_HOST_COMPATIBILITY_VERSION,
      });

      const provider = await readJson(`${host.url}/asha/browser-host/runtime-provider.json`);
      assert.equal(provider['status'], 'rust_authority');
      assert.equal(provider['available'], true);

      const page = await fetch(host.url);
      assert.equal(page.status, 200);
      const html = await page.text();
      assert.match(html, /ASHA demo/);
      assert.match(html, /\/asha\/browser-host\/native-provider\.js/);

      const script = await fetch(`${host.url}/asha/browser-host/native-provider.js`);
      assert.equal(script.status, 200);
      const scriptText = await script.text();
      assert.match(scriptText, /globalThis\.ashaRuntimeBridge/);

      const browserScope: Record<string, unknown> = {};
      runInNewContext(scriptText, browserScope);
      const browserProvider = browserScope['ashaRuntimeBridge'] as {
        readonly createRuntimeBridge: () => Record<string, unknown>;
      };
      const browserBridge = browserProvider.createRuntimeBridge();
      const secondBrowserBridge = browserProvider.createRuntimeBridge();
      assert.notEqual(secondBrowserBridge, browserBridge);
      for (const { facadeMethod } of MANIFEST_OPERATIONS) {
        assert.equal(
          typeof browserBridge[facadeMethod],
          'function',
          `served native provider must install ${facadeMethod}`,
        );
      }

      const invocation = await fetch(`${host.url}/asha/browser-host/runtime-bridge/initializeEngine`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ args: [{ seed: 17 }] }),
      });
      assert.equal(invocation.status, 200);
      assert.deepEqual(await invocation.json(), { result: { called: true } });
    } finally {
      await host.close();
    }
  } finally {
    await rm(tempRoot, { recursive: true, force: true });
  }
});

void test('browser host isolates RuntimeBridge factory clients and bounds their identities', async () => {
  const tempRoot = await mkdtemp(join(tmpdir(), 'asha-browser-host-'));
  const sessionCalls: string[][] = [];
  try {
    await writeFile(join(tempRoot, 'index.html'), '<!doctype html><title>ASHA demo</title>');
    const host = await launchNativeBrowserHost({
      uiRoot: tempRoot,
      host: '127.0.0.1',
      port: 0,
      provider: {
        globalScope: {},
        createRuntimeBridge: () => {
          const calls: string[] = [];
          sessionCalls.push(calls);
          return createFakeNativeRuntimeBridge(calls);
        },
      },
    });
    try {
      const invoke = (client: string, method: string, args: readonly unknown[]) => fetch(
        `${host.url}/asha/browser-host/runtime-bridge/${method}`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            [ASHA_BROWSER_HOST_BRIDGE_CLIENT_HEADER]: client,
          },
          body: JSON.stringify({ args }),
        },
      );

      assert.equal((await invoke('0', 'initializeEngine', [{ seed: 17 }])).status, 200);
      assert.equal((await invoke('1', 'initializeEngine', [{ seed: 23 }])).status, 200);
      assert.equal(sessionCalls.length, 2);

      assert.equal((await invoke('0', 'getProjectBundleCompositionStatus', [])).status, 200);
      assert.equal((await invoke('1', 'getProjectBundleCompositionStatus', [])).status, 200);
      assert.deepEqual(sessionCalls, [
        ['initialize:17', 'compositionStatus'],
        ['initialize:23', 'compositionStatus'],
      ]);

      const nonCanonical = await invoke('01', 'initializeEngine', [{ seed: 29 }]);
      assert.equal(nonCanonical.status, 500);
      assert.match(await nonCanonical.text(), /canonical non-negative integer/);
      const overLimit = await invoke('8', 'initializeEngine', [{ seed: 31 }]);
      assert.equal(overLimit.status, 500);
      assert.match(await overLimit.text(), /8-Session host limit/);
      assert.equal(sessionCalls.length, 2);
    } finally {
      await host.close();
    }
  } finally {
    await rm(tempRoot, { recursive: true, force: true });
  }
});

void test('browser host gives ASHA Studio pages isolated one-cell lifecycles and rejects stale sessions', async () => {
  const tempRoot = await mkdtemp(join(tmpdir(), 'asha-browser-host-studio-'));
  const cells: Array<{
    readonly id: number;
    readonly calls: string[];
    unloads: number;
  }> = [];
  let hostClosed = false;
  try {
    await writeFile(join(tempRoot, 'index.html'), '<!doctype html><title>ASHA Studio</title>');
    const host = await launchNativeBrowserHost({
      uiRoot: tempRoot,
      host: '127.0.0.1',
      port: 0,
      healthProject: 'asha-studio',
      provider: {
        globalScope: {},
        createRuntimeBridge: () => {
          const cell = { id: cells.length, calls: [] as string[], unloads: 0 };
          cells.push(cell);
          return createTrackedRuntimeBridge(cell);
        },
      },
    });
    try {
      const firstScript = await (await fetch(`${host.url}/asha/browser-host/native-provider.js`)).text();
      const secondScript = await (await fetch(`${host.url}/asha/browser-host/native-provider.js`)).text();
      const firstScope: Record<string, unknown> = {};
      const secondScope: Record<string, unknown> = {};
      runInNewContext(firstScript, firstScope);
      runInNewContext(secondScript, secondScope);
      const firstProvider = firstScope['ashaRuntimeBridge'] as {
        readonly browserHostCompatibilityVersion: string;
        readonly browserHostSessionId: string;
        readonly createRuntimeBridge: () => Record<string, unknown>;
      };
      const secondProvider = secondScope['ashaRuntimeBridge'] as typeof firstProvider;
      assert.equal(firstProvider.browserHostCompatibilityVersion, 'browser-host.v0');
      assert.notEqual(firstProvider.browserHostSessionId, secondProvider.browserHostSessionId);
      assert.match(firstProvider.browserHostSessionId, /^[A-Za-z0-9_-]{32}$/u);
      assert.match(secondProvider.browserHostSessionId, /^[A-Za-z0-9_-]{32}$/u);

      const firstBridge = firstProvider.createRuntimeBridge() as Record<string, unknown> & {
        readonly browserHostLifecycle: {
          readonly compatibilityVersion: string;
          readonly sessionId: string;
          status(): string;
        };
      };
      assert.equal(firstBridge.browserHostLifecycle.compatibilityVersion, 'browser-host.v0');
      assert.equal(firstBridge.browserHostLifecycle.sessionId, firstProvider.browserHostSessionId);
      assert.equal(firstBridge.browserHostLifecycle.status(), 'active');
      assert.deepEqual(
        Object.keys(firstBridge).sort(),
        [...ASHA_BROWSER_HOST_BRIDGE_METHODS].sort(),
      );

      const firstHeaders = browserHostHeaders(firstProvider.browserHostSessionId, '0');
      const secondHeaders = browserHostHeaders(secondProvider.browserHostSessionId, '0');
      assert.equal((await invokeBrowserHostBridge(host.url, firstHeaders, 'initializeEngine', [{ seed: 11 }])).status, 200);
      assert.equal((await invokeBrowserHostBridge(host.url, secondHeaders, 'initializeEngine', [{ seed: 22 }])).status, 200);
      assert.equal((await invokeBrowserHostBridge(host.url, firstHeaders, 'loadProjectBundle', [{ sceneId: 101 }])).status, 200);
      assert.equal((await invokeBrowserHostBridge(host.url, secondHeaders, 'loadProjectBundle', [{ sceneId: 202 }])).status, 200);
      const firstCamera = await invokeBrowserHostBridge(host.url, firstHeaders, 'createCamera', [{}]);
      const secondCamera = await invokeBrowserHostBridge(host.url, secondHeaders, 'createCamera', [{}]);
      assert.deepEqual(await firstCamera.json(), { result: { cellId: 1 } });
      assert.deepEqual(await secondCamera.json(), { result: { cellId: 2 } });

      const firstBuffer = await invokeBrowserHostBridge(host.url, firstHeaders, 'getBuffer', [7]);
      const secondBuffer = await invokeBrowserHostBridge(host.url, secondHeaders, 'getBuffer', [7]);
      assert.deepEqual(await firstBuffer.json(), { result: { cellId: 1, handle: 7 } });
      assert.deepEqual(await secondBuffer.json(), { result: { cellId: 2, handle: 7 } });

      const firstVoxel = await invokeBrowserHostBridge(host.url, firstHeaders, 'submitCommands', [{ commands: [] }]);
      const secondVoxel = await invokeBrowserHostBridge(host.url, secondHeaders, 'submitCommands', [{ commands: [] }]);
      assert.deepEqual(await firstVoxel.json(), { result: { cellId: 1 } });
      assert.deepEqual(await secondVoxel.json(), { result: { cellId: 2 } });

      const firstReadout = await invokeBrowserHostBridge(
        host.url,
        firstHeaders,
        'readComposedRuntimeSession',
        [],
      );
      const secondReadout = await invokeBrowserHostBridge(
        host.url,
        secondHeaders,
        'readComposedRuntimeSession',
        [],
      );
      assert.deepEqual(await firstReadout.json(), {
        result: { cellId: 1, project: 101, schedulerStateHash: 'scheduler-1' },
      });
      assert.deepEqual(await secondReadout.json(), {
        result: { cellId: 2, project: 202, schedulerStateHash: 'scheduler-2' },
      });

      const forgedSession = forgeBrowserSessionCapability(firstProvider.browserHostSessionId);
      const forgedHeaders = browserHostHeaders(forgedSession, '0');
      const forgedInvocation = await invokeBrowserHostBridge(
        host.url,
        forgedHeaders,
        'readComposedRuntimeSession',
        [],
      );
      assert.equal(forgedInvocation.status, 500);
      assert.match(await forgedInvocation.text(), /was not issued by this host/u);

      const forgedRetirement = await fetch(
        `${host.url}/asha/browser-host/runtime-bridge/session/${forgedSession}/disconnect`,
        { method: 'POST', body: '{}' },
      );
      assert.equal(forgedRetirement.status, 500);
      assert.match(await forgedRetirement.text(), /was not issued by this host/u);
      const firstAfterForgery = await invokeBrowserHostBridge(
        host.url,
        firstHeaders,
        'readComposedRuntimeSession',
        [],
      );
      assert.equal(firstAfterForgery.status, 200);
      assert.deepEqual(await firstAfterForgery.json(), {
        result: { cellId: 1, project: 101, schedulerStateHash: 'scheduler-1' },
      });

      const switchDisconnect = await fetch(
        `${host.url}/asha/browser-host/runtime-bridge/client/disconnect`,
        { method: 'POST', headers: firstHeaders, body: '{}' },
      );
      assert.equal(switchDisconnect.status, 200);
      assert.equal(cells[1]?.unloads, 1);
      const switchedBridge = firstProvider.createRuntimeBridge() as typeof firstBridge;
      assert.equal(switchedBridge.browserHostLifecycle.status(), 'active');
      const switchedHeaders = browserHostHeaders(firstProvider.browserHostSessionId, '1');
      assert.equal((await invokeBrowserHostBridge(host.url, switchedHeaders, 'initializeEngine', [{ seed: 33 }])).status, 200);
      assert.equal((await invokeBrowserHostBridge(host.url, switchedHeaders, 'loadProjectBundle', [{ sceneId: 303 }])).status, 200);
      const switchedReadout = await invokeBrowserHostBridge(
        host.url,
        switchedHeaders,
        'readComposedRuntimeSession',
        [],
      );
      assert.deepEqual(await switchedReadout.json(), {
        result: { cellId: 3, project: 303, schedulerStateHash: 'scheduler-3' },
      });

      const explicitDisconnect = await fetch(
        `${host.url}/asha/browser-host/runtime-bridge/client/disconnect`,
        { method: 'POST', headers: switchedHeaders, body: '{}' },
      );
      assert.equal(explicitDisconnect.status, 200);
      assert.deepEqual(await explicitDisconnect.json(), {
        status: 'disconnected',
        scope: 'client',
        browserSession: firstProvider.browserHostSessionId,
        clientId: '1',
        released: 1,
      });
      assert.equal(cells[3]?.unloads, 1);
      const closedClient = await invokeBrowserHostBridge(host.url, switchedHeaders, 'readComposedRuntimeSession', []);
      assert.equal(closedClient.status, 500);
      assert.deepEqual(await closedClient.json(), {
        error: {
          kind: 'not_initialized',
          message: 'runtime bridge error [not_initialized]: RuntimeBridge client 1 belongs to a closed or stale browser Session.',
          operation: 'browserHost.invoke',
          path: null,
          retryable: false,
          details: [],
          provenance: 'transport_loader',
        },
      });

      const pageClose = await fetch(
        `${host.url}/asha/browser-host/runtime-bridge/session/${secondProvider.browserHostSessionId}/disconnect`,
        { method: 'POST', body: '{}' },
      );
      assert.equal(pageClose.status, 200);
      assert.equal(cells[2]?.unloads, 1);
      const staleSession = await invokeBrowserHostBridge(host.url, secondHeaders, 'readComposedRuntimeSession', []);
      assert.equal(staleSession.status, 500);
      const stalePayload = await staleSession.json() as { readonly error?: { readonly kind?: string; readonly message?: string } };
      assert.equal(stalePayload.error?.kind, 'not_initialized');
      assert.match(stalePayload.error?.message ?? '', /closed or stale/u);

      assert.equal((await invokeBrowserHostBridge(host.url, {}, 'initializeEngine', [{ seed: 33 }])).status, 200);
      assert.equal((await invokeBrowserHostBridge(host.url, {}, 'loadProjectBundle', [{ sceneId: 404 }])).status, 200);
      await host.close();
      hostClosed = true;
      assert.equal(cells[0]?.unloads, 1);
    } finally {
      if (!hostClosed) {
        await host.close();
      }
    }
  } finally {
    await rm(tempRoot, { recursive: true, force: true });
  }
});

void test('browser host preserves native RuntimeBridge receiver binding over HTTP', async () => {
  const tempRoot = await mkdtemp(join(tmpdir(), 'asha-browser-host-'));
  const calls: string[] = [];
  try {
    await writeFile(join(tempRoot, 'index.html'), '<!doctype html><title>ASHA demo</title>');
    const host = await launchNativeBrowserHost({
      uiRoot: tempRoot,
      host: '127.0.0.1',
      port: 0,
      provider: {
        globalScope: {},
        createRuntimeBridge: () => createFakeNativeRuntimeBridge(calls),
      },
    });
    try {
      const invocation = await fetch(`${host.url}/asha/browser-host/runtime-bridge/initializeEngine`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ args: [{ seed: 23 }] }),
      });

      assert.equal(invocation.status, 200);
      assert.deepEqual(await invocation.json(), { result: 123 });
      assert.deepEqual(calls, ['initialize:23']);

      const cameraInvocation = await fetch(`${host.url}/asha/browser-host/runtime-bridge/createCamera`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          args: [{
            initialPose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 },
            projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
            viewport: { width: 1280, height: 720 },
          }],
        }),
      });

      assert.equal(cameraInvocation.status, 200);
      const cameraPayload = await cameraInvocation.json() as Record<string, unknown>;
      assert.deepEqual(cameraPayload['result'], {
        camera: 1,
        tick: 0,
        pose: { position: [0, 1.6, 0], yawDegrees: 0, pitchDegrees: 0 },
        basis: {
          forward: [0, 0, -1],
          right: [1, 0, 0],
          up: [0, 1, 0],
        },
        projection: { fovYDegrees: 60, near: 0.1, far: 1000 },
        viewport: { width: 1280, height: 720 },
      });

      const compositionInvocation = await fetch(
        `${host.url}/asha/browser-host/runtime-bridge/getProjectBundleCompositionStatus`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ args: [] }),
        },
      );

      assert.equal(compositionInvocation.status, 200);
      assert.deepEqual(await compositionInvocation.json(), {
        result: {
          loadedProjectBundle: 4103,
          fatalCount: 0,
          totalCount: 0,
          blocksLoad: false,
        },
      });
      assert.deepEqual(calls, ['initialize:23', 'createCamera', 'compositionStatus']);
    } finally {
      await host.close();
    }
  } finally {
    await rm(tempRoot, { recursive: true, force: true });
  }
});

void test('browser host contains native RuntimeBridge invocation failures', async () => {
  const tempRoot = await mkdtemp(join(tmpdir(), 'asha-browser-host-'));
  const calls: string[] = [];
  try {
    await writeFile(join(tempRoot, 'index.html'), '<!doctype html><title>ASHA demo</title>');
    const host = await launchNativeBrowserHost({
      uiRoot: tempRoot,
      host: '127.0.0.1',
      port: 0,
      provider: {
        globalScope: {},
        createRuntimeBridge: () => createFakeNativeRuntimeBridge(calls, true),
      },
    });
    try {
      const initializeInvocation = await fetch(
        `${host.url}/asha/browser-host/runtime-bridge/initializeEngine`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ args: [{ seed: 23 }] }),
        },
      );
      assert.equal(initializeInvocation.status, 200);

      const failedInvocation = await fetch(
        `${host.url}/asha/browser-host/runtime-bridge/getProjectBundleCompositionStatus`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ args: [] }),
        },
      );

      assert.equal(failedInvocation.status, 500);
      assert.deepEqual(await failedInvocation.json(), {
        error: {
          kind: 'internal',
          message: 'runtime bridge error [internal]: native composition status failed',
          operation: 'get_project_bundle_composition_status',
          path: '$',
          retryable: false,
          details: ['invalid_native_error_envelope'],
          provenance: 'transport_loader',
        },
      });
      assert.deepEqual(calls, ['initialize:23', 'compositionStatus']);

      const health = await fetch(`${host.url}/health`);
      assert.equal(health.status, 200);
      assert.deepEqual(await health.json(), {
        ok: true,
        project: 'asha-game-project',
        compatibilityVersion: ASHA_BROWSER_HOST_COMPATIBILITY_VERSION,
      });
    } finally {
      await host.close();
    }
  } finally {
    await rm(tempRoot, { recursive: true, force: true });
  }
});

void test('browser host rejects raw traversal into sibling directories outside ui root', async () => {
  const tempRoot = await mkdtemp(join(tmpdir(), 'asha-browser-host-'));
  try {
    const uiRoot = join(tempRoot, 'ui');
    const secretRoot = join(tempRoot, 'ui-secret');
    await mkdir(uiRoot);
    await mkdir(secretRoot);
    await writeFile(join(uiRoot, 'index.html'), '<!doctype html><title>ASHA demo</title>');
    await writeFile(join(secretRoot, 'secret.txt'), 'outside-ui-root');

    const host = await launchNativeBrowserHost({
      uiRoot,
      host: '127.0.0.1',
      port: 0,
      provider: {
        globalScope: {},
        createRuntimeBridge: createFakeRuntimeBridge,
      },
    });
    try {
      const response = await readRawHttpPath(host.url, '/%2e%2e/ui-secret/secret.txt');
      assert.equal(response.statusCode, 403);
      assert.equal(response.body, 'Forbidden');
    } finally {
      await host.close();
    }
  } finally {
    await rm(tempRoot, { recursive: true, force: true });
  }
});

async function readJson(url: string): Promise<Record<string, unknown>> {
  const response = await fetch(url);
  assert.equal(response.status, 200);
  return await response.json() as Record<string, unknown>;
}

function readRawHttpPath(baseUrl: string, path: string): Promise<{ readonly statusCode: number; readonly body: string }> {
  const url = new URL(baseUrl);
  return new Promise((resolveRead, rejectRead) => {
    const requestHandle = request({
      hostname: url.hostname,
      method: 'GET',
      path,
      port: Number(url.port),
    }, (response) => {
      const chunks: Buffer[] = [];
      response.on('data', (chunk: Buffer) => {
        chunks.push(chunk);
      });
      response.on('end', () => {
        resolveRead({
          statusCode: response.statusCode ?? 0,
          body: Buffer.concat(chunks).toString('utf8'),
        });
      });
    });
    requestHandle.on('error', rejectRead);
    requestHandle.end();
  });
}

function browserHostHeaders(browserSession: string, clientId: string): Record<string, string> {
  return {
    'Content-Type': 'application/json',
    [ASHA_BROWSER_HOST_BRIDGE_SESSION_HEADER]: browserSession,
    [ASHA_BROWSER_HOST_BRIDGE_CLIENT_HEADER]: clientId,
  };
}

function forgeBrowserSessionCapability(browserSession: string): string {
  const finalCharacter = browserSession.at(-1);
  const replacement = finalCharacter === 'A' ? 'B' : 'A';
  return `${browserSession.slice(0, -1)}${replacement}`;
}

function invokeBrowserHostBridge(
  baseUrl: string,
  headers: Record<string, string>,
  method: string,
  args: readonly unknown[],
): Promise<Response> {
  return fetch(`${baseUrl}/asha/browser-host/runtime-bridge/${method}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...headers },
    body: JSON.stringify({ args }),
  });
}

function createTrackedRuntimeBridge(cell: {
  readonly id: number;
  readonly calls: string[];
  unloads: number;
}): RuntimeBridge {
  let project: number | null = null;
  return {
    ...createFakeRuntimeBridge(),
    initializeEngine(input) {
      cell.calls.push(`initialize:${input.seed}`);
      return cell.id as never;
    },
    loadProjectBundle(input) { // vocab-allow: tracked public bridge fixture
      project = (input as { readonly sceneId?: number }).sceneId ?? null;
      cell.calls.push(`load:${project ?? 'none'}`);
      return { loadedProjectBundle: project, fatalCount: 0, totalCount: 0, blocksLoad: false } as never;
    },
    unloadProjectBundle() {
      cell.calls.push(`unload:${project ?? 'none'}`);
      cell.unloads += 1;
      project = null;
    },
    createCamera() {
      cell.calls.push('camera');
      return { cellId: cell.id } as never;
    },
    getBuffer(handle) {
      cell.calls.push(`buffer:${handle}`);
      return { cellId: cell.id, handle } as never;
    },
    submitCommands() {
      cell.calls.push('voxel');
      return { cellId: cell.id } as never;
    },
    readComposedRuntimeSession() {
      cell.calls.push(`read:${project ?? 'none'}`);
      return { cellId: cell.id, project, schedulerStateHash: `scheduler-${cell.id}` } as never;
    },
  };
}

function createFakeRuntimeBridge(): RuntimeBridge {
  const operation = () => ({ called: true }) as never;
  return {
    initializeEngine: operation,
    configureInputSession: operation,
    applyInputContextCommand: operation,
    submitRawInput: operation,
    replayResolvedInputAction: operation,
    readInputContextState: operation,
    applyTimeControlCommand: operation,
    readTimeControlState: operation,
    loadProjectBundle: operation, // vocab-allow: fake bridge must satisfy the legacy RuntimeBridge method name.
    getProjectBundleCompositionStatus: operation,
    submitCommands: operation,
    stepSimulation: operation,
    createCamera: operation,
    applyCameraModeCommand: operation,
    applyCameraNavigationInput: operation,
    readCameraControllerState: operation,
    applyFirstPersonCameraInput: operation,
    readCameraProjection: operation,
    pickVoxel: operation,
    configureVoxelProjectionInstances: operation,
    pickVoxelInstance: operation,
    selectVoxel: operation,
    readVoxelMeshEvidence: operation,
    readRenderDiffs: operation,
    readProjectionFrame: operation,
    readDeveloperConsole: operation,
    saveProjectBundle: operation,
    applyCollisionConstrainedCameraInput: operation,
    applyGeneratedTunnelToRuntimeWorld: operation,
    readModelMaterialPreview: operation,
    decodeSceneDocument: operation,
    encodeSceneDocument: operation,
    applySceneDocumentAuthoring: operation,
    loadFpsRuntimeSession: operation,
    readFpsRuntimeSession: operation,
    applyFpsPrimaryFire: operation,
    readComposedRuntimeSession: operation,
    readGameplayModuleView: operation,
    applyGameplayPrefabPartInteraction: operation,
    invokeGameExtensionWeaponEffect: operation,
    validateGameRuleCatalog: operation,
    submitGameRuleEffectIntent: operation,
    readGameRuleRuntimeReadout: operation,
    restartFpsRuntimeSession: operation,
    readFpsEncounterDirector: operation,
    applyFpsEncounterTransition: operation,
    readSceneObjectSnapshot: operation,
    applySceneObjectCommand: operation,
    applyEnemyDirectNavMovement: operation,
    planVoxelConversion: operation,
    registerVoxelConversionSource: operation,
    registerVoxelConversionMeshAsset: operation,
    importVoxelConversionMeshSource: operation,
    readVoxelConversionSourceMetadata: operation,
    previewVoxelConversion: operation,
    applyVoxelConversion: operation,
    exportVoxelConversionEvidence: operation,
    readVoxelModelInfo: operation,
    readVoxelModelWindow: operation,
    exportVoxelVolumeAsset: operation,
    saveVoxelVolumeAsset: operation,
    updateVoxelVolumeAssetPalette: operation,
    initializeVoxelVolumeAuthoring: operation,
    loadVoxelVolumeAsset: operation,
    unloadVoxelVolumeAsset: operation,
    validateVoxelAnnotationLayer: operation,
    loadVoxelAnnotationLayer: operation,
    readVoxelAnnotationQuery: operation,
    applyVoxelAnnotationEdit: operation,
    exportVoxelAnnotationLayer: operation,
    readVoxelEditHistory: operation,
    previewVoxelEditRevert: operation,
    applyVoxelEditRevert: operation,
    undoVoxelEdit: operation,
    redoVoxelEdit: operation,
    getBuffer: operation,
    releaseBuffer: operation,
    unloadProjectBundle: operation,
    loadReplayFixture: operation,
    runReplayStep: operation,
  };
}

function createFakeNativeRuntimeBridge(calls: string[], failCompositionStatus = false): RuntimeBridge {
  const addon = {
    initializeEngine: (seed: number) => {
      calls.push(`initialize:${seed}`);
      return seed + 100;
    },
    createCamera: (_handle: number, request: Parameters<RuntimeBridge['createCamera']>[0]) => {
      calls.push('createCamera');
      return {
        camera: 1,
        tick: 0,
        pose: request.initialPose,
        basis: {
          forward: [0, 0, -1],
          right: [1, 0, 0],
          up: [0, 1, 0],
        },
        projection: request.projection,
        viewport: request.viewport,
      };
    },
    getProjectBundleCompositionStatus: (handle: number) => {
      void handle;
      calls.push('compositionStatus');
      if (failCompositionStatus) {
        throw new Error('native composition status failed');
      }
      return {
        loadedProjectBundle: 4103,
        fatalCount: 0,
        totalCount: 0,
        blocksLoad: false,
      };
    },
  } as unknown as ConstructorParameters<typeof NativeRuntimeBridge>[0];
  return new NativeRuntimeBridge(addon);
}
