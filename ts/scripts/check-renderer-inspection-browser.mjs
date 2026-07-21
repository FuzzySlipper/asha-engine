import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { constants as fsConstants } from 'node:fs';
import { access, mkdtemp, readFile, rm } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { extname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPOSITORY_ROOT = resolve(fileURLToPath(new URL('../..', import.meta.url)));
const TEST_PATH = '/renderer-inspection-browser-test';

const INITIAL_GRID = {
  visible: true,
  grid: {
    coordinateSystem: 'rightHandedYUp',
    origin: [0, 0, 0],
    spacing: [1, 1, 1],
  },
  plane: 'xz',
  snapAnchor: 'boundary',
  style: {
    minorColor: [0.2, 0.2, 0.2, 0.5],
    majorColor: [0.4, 0.4, 0.4, 0.8],
    xAxisColor: [1, 0, 0, 1],
    yAxisColor: [0, 1, 0, 1],
    zAxisColor: [0, 0, 1, 1],
    majorLineEvery: 10,
    opacity: 0.8,
    fadeStart: 20,
    fadeEnd: 100,
  },
};

const TEST_PAGE = `<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <style>
    html, body { margin: 0; min-height: 2200px; }
    canvas { display: block; margin: 40px; width: 640px; height: 360px; }
  </style>
  <script type="importmap">
    {
      "imports": {
        "@asha/contracts": "/ts/packages/contracts/dist/index.js",
        "@asha/render-projection": "/ts/packages/render-projection/dist/index.js",
        "@asha/renderer-three/backend": "/ts/packages/renderer-three/dist/backend.js",
        "@asha/runtime-bridge": "/renderer-inspection-runtime-bridge-shim.js",
        "three": "/ts/packages/renderer-three/node_modules/three/build/three.module.js",
        "three/examples/jsm/": "/ts/packages/renderer-three/node_modules/three/examples/jsm/"
      }
    }
  </script>
</head>
<body>
  <canvas id="surface" width="640" height="360" tabindex="0"></canvas>
  <script type="module">
    import { mountAshaRendererInspectionSurface } from '/ts/packages/renderer-host/dist/inspection-surface.js';

    const canvas = document.querySelector('#surface');
    const positions = [0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0];
    const normals = [0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1];
    const indices = [0, 1, 2, 0, 2, 3];
    const voxelMeshBytes = new Uint8Array((positions.length + normals.length + indices.length) * 4);
    const voxelMeshView = new DataView(voxelMeshBytes.buffer);
    let voxelMeshOffset = 0;
    for (const value of positions) {
      voxelMeshView.setFloat32(voxelMeshOffset, value, true);
      voxelMeshOffset += 4;
    }
    for (const value of normals) {
      voxelMeshView.setFloat32(voxelMeshOffset, value, true);
      voxelMeshOffset += 4;
    }
    for (const value of indices) {
      voxelMeshView.setUint32(voxelMeshOffset, value, true);
      voxelMeshOffset += 4;
    }
    const buffers = new Map([[7, voxelMeshBytes]]);
    const borrowedBuffers = [];
    const releasedBuffers = [];
    const bufferSource = {
      borrow: handle => {
        const bytes = buffers.get(handle);
        if (bytes === undefined) throw new Error('unknown browser fixture buffer ' + handle);
        borrowedBuffers.push(handle);
        return bytes;
      },
      release: handle => releasedBuffers.push(handle),
    };
    const surface = await mountAshaRendererInspectionSurface(
      canvas,
      {
        autoStart: false,
        bufferSource,
        initialGrid: ${JSON.stringify(INITIAL_GRID)},
        controls: { initialPosition: [0, 19, 1], minimumDistance: 2, maximumDistance: 20 },
      },
    );
    window.__ashaInspection = {
      canvas,
      applyRuntimeFrame: frame => surface.applyRuntimeFrame(frame),
      clearRuntimeProjection: () => surface.clearRuntimeProjection(),
      bufferLifecycle: () => ({ borrowed: [...borrowedBuffers], released: [...releasedBuffers] }),
      render: timeMs => surface.renderOnce(timeMs),
      setGrid: descriptor => surface.setGrid(descriptor),
      snapshot: () => surface.readout(),
    };
    document.documentElement.dataset.ready = 'true';
  </script>
</body>
</html>`;

async function main() {
  const chromium = await findChromium();
  const server = createStaticServer();
  await listen(server);
  const address = server.address();
  assert.ok(address && typeof address === 'object');
  const testUrl = `http://127.0.0.1:${address.port}${TEST_PATH}`;
  const profile = await mkdtemp(resolve(tmpdir(), 'asha-inspection-browser-'));
  const debugPort = await reservePort();
  const browser = spawn(chromium, [
    '--headless=new',
    '--no-sandbox',
    '--disable-dev-shm-usage',
    '--enable-unsafe-swiftshader',
    '--use-angle=swiftshader',
    `--remote-debugging-port=${debugPort}`,
    `--user-data-dir=${profile}`,
    'about:blank',
  ], { stdio: ['ignore', 'ignore', 'pipe'] });

  let client;
  try {
    await waitForDevTools(browser, debugPort);
    const targetResponse = await fetch(
      `http://127.0.0.1:${debugPort}/json/new?${encodeURIComponent(testUrl)}`,
      { method: 'PUT' },
    );
    assert.equal(targetResponse.ok, true, `Chromium target creation failed: ${targetResponse.status}`);
    const target = await targetResponse.json();
    assert.equal(typeof target.webSocketDebuggerUrl, 'string');
    client = await CdpClient.connect(target.webSocketDebuggerUrl);
    await client.send('Runtime.enable');
    await client.send('Page.enable');
    await client.send('Network.enable');
    await client.send('Log.enable');
    await waitForInspectionSurface(client);

    const point = await evaluate(client, `(() => {
      const rect = window.__ashaInspection.canvas.getBoundingClientRect();
      return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
    })()`);
    const initial = await snapshot(client);
    assert.equal(initial.grid.descriptor.visible, true);
    assert.ok(initial.grid.renderedLineCount > 0, 'real browser backend should realize visible grid lines');
    assert.notEqual(initial.grid.bounds, null);
    assert.equal(initial.gridRevision, 1);
    assert.ok(
      Math.abs(initial.camera.pose.pitchDegrees) <= 85.000_001,
      'real browser mount must clamp the initial inspection camera pitch',
    );

    const voxelDefined = await evaluate(client, `(() => {
      const receipt = window.__ashaInspection.applyRuntimeFrame({ ops: [
        {
          op: 'create', handle: 71, parent: null,
          node: {
            geometry: { shape: 'cube' },
            material: { color: [0.7, 0.6, 0.4, 1], wireframe: false },
            transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
            visible: true, layer: 'scene',
            metadata: { source: null, tags: [], label: 'runtime-voxel-chunk' },
          },
        },
        {
          op: 'replaceMeshPayload', handle: 71,
          payload: {
            layout: {
              vertexCount: 4, indexCount: 6, indexWidth: 'u32',
              attributes: [
                { name: 'position', components: 3, kind: 'f32' },
                { name: 'normal', components: 3, kind: 'f32' },
              ],
            },
            groups: [{ materialSlot: 0, start: 0, count: 6 }],
            bounds: { min: [0, 0, 0], max: [1, 1, 0] },
            source: {
              kind: 'handle', buffer: 7,
              positionsByteOffset: 0, normalsByteOffset: 48, indicesByteOffset: 96,
            },
            provenance: 'voxelChunk',
          },
        },
      ] });
      return {
        receipt,
        readout: window.__ashaInspection.snapshot(),
        buffers: window.__ashaInspection.bufferLifecycle(),
      };
    })()`);
    assert.equal(voxelDefined.receipt.applied, true);
    assert.equal(voxelDefined.readout.runtimeGeneration, 1);
    assert.equal(voxelDefined.readout.runtimeRetainedOpCount, 2);
    assert.deepEqual(voxelDefined.buffers, { borrowed: [7], released: [7] });

    const cameraBeforeRuntimeUpdate = voxelDefined.readout.camera;
    const voxelUpdated = await evaluate(client, `(() => {
      const receipt = window.__ashaInspection.applyRuntimeFrame({ ops: [{
        op: 'update', handle: 71,
        transform: { translation: [2, 0, -1], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
        material: null, visible: null, metadata: null,
      }] });
      window.__ashaInspection.render(10);
      return {
        receipt,
        readout: window.__ashaInspection.snapshot(),
        buffers: window.__ashaInspection.bufferLifecycle(),
      };
    })()`);
    assert.equal(voxelUpdated.receipt.applied, true);
    assert.equal(voxelUpdated.readout.runtimeGeneration, 2);
    assert.equal(voxelUpdated.readout.runtimeRetainedOpCount, 3);
    assert.deepEqual(voxelUpdated.readout.camera, cameraBeforeRuntimeUpdate);
    assert.deepEqual(voxelUpdated.buffers, { borrowed: [7, 7], released: [7, 7] });

    await client.send('Input.dispatchMouseEvent', {
      type: 'mousePressed', x: point.x, y: point.y, button: 'left', buttons: 1, clickCount: 1,
    });
    const pressed = await snapshot(client);
    assert.equal(pressed.dragging, true, 'real browser pointerdown should begin captured drag');
    await client.send('Input.dispatchMouseEvent', {
      type: 'mouseMoved', x: point.x + 80, y: point.y - 40, button: 'none', buttons: 1,
    });
    const capturedMove = await snapshot(client);
    assert.equal(
      capturedMove.lastCameraChange,
      'pointer_orbit',
      'real browser pointermove should orbit before release',
    );
    await client.send('Input.dispatchMouseEvent', {
      type: 'mouseReleased', x: point.x + 80, y: point.y - 40, button: 'left', buttons: 0, clickCount: 1,
    });
    const pointerOrbit = await snapshot(client);
    assert.notDeepEqual(pointerOrbit.camera.pose.position, initial.camera.pose.position);
    assert.equal(pointerOrbit.lastCameraChange, 'pointer_orbit');
    assert.equal(pointerOrbit.dragging, false);

    await dispatchKey(client, 'ArrowLeft', 'ArrowLeft', 37, async () => {
      await evaluate(client, 'window.__ashaInspection.render(1000)');
    });
    const keyboardOrbit = await snapshot(client);
    assert.equal(keyboardOrbit.lastCameraChange, 'keyboard_orbit');
    assert.ok(keyboardOrbit.cameraRevision > pointerOrbit.cameraRevision);

    await dispatchKey(client, 'w', 'KeyW', 87, async () => {
      await evaluate(client, 'window.__ashaInspection.render(2000)');
    });
    const movement = await snapshot(client);
    assert.equal(movement.lastCameraChange, 'keyboard_movement');

    await dispatchKey(client, '+', 'Equal', 187, undefined, 8);
    const keyboardZoom = await snapshot(client);
    assert.equal(keyboardZoom.lastCameraChange, 'keyboard_zoom');
    assert.ok(keyboardZoom.cameraDistance < movement.cameraDistance);

    await client.send('Input.dispatchMouseEvent', {
      type: 'mouseWheel', x: point.x, y: point.y, deltaX: 0, deltaY: 120,
    });
    await delay(50);
    const wheelZoom = await snapshot(client);
    assert.equal(wheelZoom.lastCameraChange, 'wheel_zoom');
    assert.ok(wheelZoom.cameraDistance > keyboardZoom.cameraDistance);
    assert.equal(await evaluate(client, 'window.scrollY'), 0, 'consumed controls must not scroll the page');

    const cleared = await evaluate(client, `(() => {
      const receipt = window.__ashaInspection.setGrid(null);
      return { receipt, readout: window.__ashaInspection.snapshot() };
    })()`);
    assert.equal(cleared.receipt.applied, true);
    assert.equal(cleared.readout.grid, null);
    assert.equal(cleared.readout.gridRevision, 2);

    const replacement = structuredClone(INITIAL_GRID);
    replacement.grid.spacing = [2, 2, 2];
    const replaced = await evaluate(client, `(() => {
      const receipt = window.__ashaInspection.setGrid(${JSON.stringify(replacement)});
      return { receipt, readout: window.__ashaInspection.snapshot() };
    })()`);
    assert.equal(replaced.receipt.applied, true);
    assert.deepEqual(replaced.readout.grid.descriptor.grid.spacing, [2, 2, 2]);
    assert.equal(replaced.readout.gridRevision, 3);

    const voxelDeleted = await evaluate(client, `(() => {
      const receipt = window.__ashaInspection.applyRuntimeFrame({
        ops: [{ op: 'destroy', handle: 71 }],
      });
      return {
        receipt,
        readout: window.__ashaInspection.snapshot(),
        buffers: window.__ashaInspection.bufferLifecycle(),
      };
    })()`);
    assert.equal(voxelDeleted.receipt.applied, true);
    assert.equal(voxelDeleted.readout.runtimeGeneration, 3);
    assert.equal(voxelDeleted.readout.runtimeRetainedOpCount, 4);
    assert.deepEqual(voxelDeleted.buffers, { borrowed: [7, 7, 7], released: [7, 7, 7] });

    const runtimeCleared = await evaluate(client, `(() => ({
      receipt: window.__ashaInspection.clearRuntimeProjection(),
      readout: window.__ashaInspection.snapshot(),
      buffers: window.__ashaInspection.bufferLifecycle(),
    }))()`);
    assert.equal(runtimeCleared.receipt.applied, true);
    assert.equal(runtimeCleared.readout.runtimeGeneration, 4);
    assert.equal(runtimeCleared.readout.runtimeRetainedOpCount, 0);
    assert.deepEqual(runtimeCleared.buffers, { borrowed: [7, 7, 7], released: [7, 7, 7] });

    process.stdout.write('Renderer inspection browser interaction: OK\n');
  } finally {
    client?.close();
    browser.kill('SIGTERM');
    await waitForBrowserExit(browser);
    server.closeAllConnections();
    await new Promise(resolveClose => server.close(resolveClose));
    await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 50 });
  }
}

function createStaticServer() {
  return createServer(async (request, response) => {
    try {
      const requestUrl = new URL(request.url ?? '/', 'http://127.0.0.1');
      if (requestUrl.pathname === TEST_PATH) {
        response.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
        response.end(TEST_PAGE);
        return;
      }
      if (requestUrl.pathname === '/renderer-inspection-runtime-bridge-shim.js') {
        response.writeHead(200, { 'Content-Type': 'text/javascript; charset=utf-8' });
        response.end(
          "export { decodeRenderFrameDiff } from '/ts/packages/runtime-bridge/dist/render-decode.js';\n"
          + "export { RuntimeBridgeError } from '/ts/packages/runtime-bridge/dist/bridge.js';\n",
        );
        return;
      }
      const filePath = resolve(REPOSITORY_ROOT, `.${decodeURIComponent(requestUrl.pathname)}`);
      if (!filePath.startsWith(`${REPOSITORY_ROOT}/`)) {
        response.writeHead(403).end();
        return;
      }
      const bytes = await readFile(filePath);
      response.writeHead(200, { 'Content-Type': contentType(filePath) });
      response.end(bytes);
    } catch {
      response.writeHead(404).end();
    }
  });
}

function contentType(filePath) {
  switch (extname(filePath)) {
    case '.js': return 'text/javascript; charset=utf-8';
    case '.json': return 'application/json; charset=utf-8';
    default: return 'application/octet-stream';
  }
}

async function listen(server) {
  await new Promise((resolveListen, rejectListen) => {
    server.once('error', rejectListen);
    server.listen(0, '127.0.0.1', resolveListen);
  });
}

async function reservePort() {
  const server = createServer();
  await listen(server);
  const address = server.address();
  assert.ok(address && typeof address === 'object');
  const port = address.port;
  await new Promise(resolveClose => server.close(resolveClose));
  return port;
}

async function findChromium() {
  const candidates = [
    process.env['CHROMIUM_BIN'],
    '/usr/bin/chromium',
    '/usr/bin/chromium-browser',
    '/usr/bin/google-chrome',
  ].filter(Boolean);
  for (const candidate of candidates) {
    try {
      await access(candidate, fsConstants.X_OK);
      return candidate;
    } catch {
      // Try the next canonical browser installation.
    }
  }
  throw new Error('Chromium is required for the renderer inspection browser regression; set CHROMIUM_BIN.');
}

async function waitForDevTools(browser, port) {
  let diagnostics = '';
  browser.stderr.setEncoding('utf8');
  browser.stderr.on('data', chunk => {
    diagnostics = `${diagnostics}${chunk}`.slice(-4000);
  });
  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    if (browser.exitCode !== null) {
      throw new Error(`Chromium exited before DevTools became available:\n${diagnostics}`);
    }
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/version`);
      if (response.ok) return;
    } catch {
      // Browser startup is still in progress.
    }
    await delay(50);
  }
  throw new Error(`Timed out waiting for Chromium DevTools:\n${diagnostics}`);
}

async function waitForInspectionSurface(client) {
  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    const ready = await evaluate(client, `({
      ready: document.documentElement?.dataset.ready === 'true',
      state: document.readyState,
    })`);
    if (ready.ready) return;
    await delay(50);
  }
  const pageState = await evaluate(client, `({
    body: document.body?.innerText ?? '',
    resources: performance.getEntriesByType('resource').map(entry => entry.name),
    state: document.readyState,
  })`);
  throw new Error(`Timed out waiting for the real-browser inspection surface fixture.\n`
    + `${JSON.stringify(pageState)}\n${client.diagnostics()}`);
}

async function dispatchKey(client, key, code, virtualKeyCode, whileHeld, modifiers = 0) {
  await client.send('Input.dispatchKeyEvent', {
    type: 'keyDown', key, code, modifiers, windowsVirtualKeyCode: virtualKeyCode,
  });
  if (whileHeld !== undefined) await whileHeld();
  await client.send('Input.dispatchKeyEvent', {
    type: 'keyUp', key, code, modifiers, windowsVirtualKeyCode: virtualKeyCode,
  });
}

async function snapshot(client) {
  return evaluate(client, 'window.__ashaInspection.snapshot()');
}

async function evaluate(client, expression) {
  const response = await client.send('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (response.exceptionDetails !== undefined) {
    throw new Error(response.exceptionDetails.exception?.description ?? response.exceptionDetails.text);
  }
  return response.result.value;
}

function delay(milliseconds) {
  return new Promise(resolveDelay => setTimeout(resolveDelay, milliseconds));
}

async function waitForBrowserExit(browser) {
  if (browser.exitCode !== null || browser.signalCode !== null) return;
  await new Promise(resolveExit => {
    const timeout = setTimeout(() => browser.kill('SIGKILL'), 2_000);
    browser.once('exit', () => {
      clearTimeout(timeout);
      resolveExit();
    });
  });
}

class CdpClient {
  static async connect(url) {
    const socket = new WebSocket(url);
    await new Promise((resolveOpen, rejectOpen) => {
      socket.addEventListener('open', resolveOpen, { once: true });
      socket.addEventListener('error', rejectOpen, { once: true });
    });
    return new CdpClient(socket);
  }

  #nextId = 1;
  #pending = new Map();
  #events = [];

  constructor(socket) {
    this.socket = socket;
    socket.addEventListener('message', event => {
      const message = JSON.parse(event.data);
      if (message.id === undefined) {
        if (
          message.method === 'Runtime.exceptionThrown'
          || message.method === 'Runtime.consoleAPICalled'
          || message.method === 'Log.entryAdded'
          || message.method === 'Network.loadingFailed'
          || (
            message.method === 'Network.responseReceived'
            && Number(message.params?.response?.status ?? 0) >= 400
          )
        ) {
          this.#events.push(message);
        }
        return;
      }
      const pending = this.#pending.get(message.id);
      if (pending === undefined) return;
      this.#pending.delete(message.id);
      if (message.error !== undefined) pending.reject(new Error(message.error.message));
      else pending.resolve(message.result ?? {});
    });
  }

  send(method, params = {}) {
    const id = this.#nextId;
    this.#nextId += 1;
    return new Promise((resolveSend, rejectSend) => {
      const timeout = setTimeout(() => {
        this.#pending.delete(id);
        rejectSend(new Error(`Timed out waiting for CDP method ${method}.`));
      }, 10_000);
      this.#pending.set(id, {
        resolve: value => {
          clearTimeout(timeout);
          resolveSend(value);
        },
        reject: error => {
          clearTimeout(timeout);
          rejectSend(error);
        },
      });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }

  close() {
    this.socket.close();
  }

  diagnostics() {
    return this.#events.map(event => JSON.stringify(event)).join('\n').slice(-8000);
  }
}

await main();
