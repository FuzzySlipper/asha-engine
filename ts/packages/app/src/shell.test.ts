import { test } from 'node:test';
import assert from 'node:assert/strict';

import type { PickRay, RenderFrameDiff } from '@asha/contracts';
import { renderHandle } from '@asha/contracts';
import {
  frameCursor,
  RuntimeBridgeError,
  type RuntimeBridge,
} from '@asha/runtime-bridge';
import { createMockRuntimeBridge } from '@asha/runtime-bridge/reference';
import { inspectEditor } from '@asha/devtools';

import {
  composeAppShell,
  formatReadout,
  type AppBridgeBoot,
  type FixtureChoice,
  type RendererPort,
} from './shell.js';

const FIXTURES: FixtureChoice[] = [
  {
    id: 'launch-grid',
    label: 'Launch grid',
    materials: [1, 2, 3],
    request: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 1001 },
  },
  {
    id: 'alt-grid',
    label: 'Alternate grid',
    materials: [7],
    request: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 1002 },
  },
];

function referenceBoot(): AppBridgeBoot {
  return { bridge: createMockRuntimeBridge(), mode: 'mock', intent: 'reference', nativeAvailable: false };
}

/** A counting renderer port (no three.js): proves the renderer seam is driven. */
function countingRenderer(): RendererPort & { frames: RenderFrameDiff[] } {
  const frames: RenderFrameDiff[] = [];
  let nodes = 0;
  return {
    frames,
    applyFrame(diff) {
      frames.push(diff);
      nodes += diff.ops.length;
    },
    get sceneNodeCount() {
      return nodes;
    },
  };
}

type RuntimeBridgeMethod = (this: RuntimeBridge, ...args: never[]) => unknown;

function bridgeProxyValue(target: RuntimeBridge, prop: string | symbol, recv: unknown): unknown {
  const value: unknown = Reflect.get(target, prop, recv);
  return typeof value === 'function' ? (value as RuntimeBridgeMethod).bind(target) : value;
}

/** Wrap a bridge so `readRenderDiffs` fails closed like an unwired native op. */
function withProjectionGap(inner: RuntimeBridge): RuntimeBridge {
  return new Proxy(inner, {
    get(target, prop, recv) {
      if (prop === 'readRenderDiffs') {
        return () => {
          throw new RuntimeBridgeError('operation_unimplemented', 'authority projection not wired');
        };
      }
      return bridgeProxyValue(target, prop, recv);
    },
  });
}

void test('reference composition: assembles runtime + UI + devtools off ONE editor store', () => {
  const renderer = countingRenderer();
  const shell = composeAppShell({
    host: { name: 'headless', accessibility: false },
    bootBridge: referenceBoot,
    fixtures: FIXTURES,
    renderer,
  });

  const status = shell.runtimeStatus();
  assert.equal(status.availability, 'reference');
  assert.equal(status.mode, 'mock');

  // Drive the editor through the accessible control model, then read it back via BOTH
  // the shell's devtools inspection and an independent inspectEditor — they must agree,
  // proving there is a single shared store, not parallel copies.
  shell.applyControl('tool', 'place');
  shell.applyControl('material', '2');
  const viaShell = shell.editorInspection();
  const viaDevtools = inspectEditor(shell.controller.store.getState());
  assert.deepEqual(viaShell, viaDevtools);
  assert.equal(viaShell.tool, 'place');
  assert.equal(viaShell.material, 2);
});

void test('runtime mode readout distinguishes reference / degraded / unavailable', () => {
  // reference
  assert.equal(
    composeAppShell({ host: { name: 'h', accessibility: false }, bootBridge: referenceBoot, fixtures: FIXTURES })
      .runtimeStatus().availability,
    'reference',
  );

  // degraded: booted, but a needed capability fails closed at runtime (visible).
  const degraded = composeAppShell({
    host: { name: 'h', accessibility: false },
    bootBridge: () => ({
      bridge: withProjectionGap(createMockRuntimeBridge()),
      mode: 'native',
      intent: 'authority',
      nativeAvailable: true,
    }),
    fixtures: FIXTURES,
    renderer: countingRenderer(),
  });
  degraded.projectAuthority();
  const dStatus = degraded.runtimeStatus();
  assert.equal(dStatus.availability, 'degraded');
  assert.equal(dStatus.error?.kind, 'operation_unimplemented');

  // unavailable: boot failed closed (no bridge) — never a faked mock success.
  const unavailable = composeAppShell({
    host: { name: 'h', accessibility: false },
    bootBridge: () => ({
      bridge: null,
      mode: 'native',
      intent: 'authority',
      nativeAvailable: false,
      bootError: new RuntimeBridgeError('native_unavailable', 'addon missing'),
    }),
    fixtures: FIXTURES,
  });
  const uStatus = unavailable.runtimeStatus();
  assert.equal(uStatus.availability, 'unavailable');
  assert.equal(uStatus.error?.kind, 'native_unavailable');
});

void test('fixture selection is runtime-selectable (data, not compile-time)', () => {
  const shell = composeAppShell({
    host: { name: 'h', accessibility: false },
    bootBridge: referenceBoot,
    fixtures: FIXTURES,
  });
  assert.equal(shell.activeFixture.id, 'launch-grid');
  // Palette tracks the active fixture's catalog materials.
  assert.deepEqual(shell.palette().map((m) => m.id), [1, 2, 3]);

  shell.selectFixture('alt-grid');
  assert.equal(shell.activeFixture.id, 'alt-grid');
  assert.deepEqual(shell.palette().map((m) => m.id), [7]);
  assert.deepEqual(
    shell.fixtureListing().map((f) => ({ id: f.id, active: f.active })),
    [
      { id: 'launch-grid', active: false },
      { id: 'alt-grid', active: true },
    ],
  );

  const world = shell.loadActiveFixture();
  assert.equal(world.loaded, true);
  assert.equal(world.composition?.loadedProjectBundle, 1002);
  assert.throws(() => shell.selectFixture('nope'), /unknown fixture/);
});

void test('controls are accessible and route through the ONE store / controller', () => {
  const shell = composeAppShell({
    host: { name: 'h', accessibility: false },
    bootBridge: referenceBoot,
    fixtures: FIXTURES,
  });
  const controls = shell.controls();
  // Every control carries a stable id, an ARIA role, and an accessible label.
  for (const c of controls) {
    assert.ok(c.id.length > 0 && c.role.length > 0 && c.label.length > 0);
  }
  const commit = controls.find((c) => c.id === 'commit')!;
  assert.equal(commit.disabled, true, 'nothing to commit without a selection');

  // Drive a full place edit purely through accessible controls + a pick selection.
  shell.applyControl('tool', 'place');
  shell.applyControl('material', '1');
  shell.controller.store.dispatch({ type: 'setSelection', selection: { voxel: { x: 0, y: 0, z: 0 }, face: 'posX' } });
  shell.loadActiveFixture();
  assert.equal(shell.controls().find((c) => c.id === 'commit')!.disabled, false);
  shell.applyControl('commit', 'commit');
  assert.deepEqual(shell.readout().lastCommandResult, { accepted: 1, rejected: 0, rejections: [] });
});

void test('projectAuthority reads through the facade and drives the renderer port', () => {
  // A bridge that emits one create op through readRenderDiffs proves the projection path.
  const base = createMockRuntimeBridge();
  const frame: RenderFrameDiff = {
    ops: [
      {
        op: 'create',
        handle: renderHandle(1),
        parent: null,
        node: {
          geometry: { shape: 'cube' },
          material: { color: [1, 1, 1, 1], wireframe: false },
          transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
          visible: true,
          layer: 'scene',
          metadata: { source: null, tags: [], label: 'x' },
        },
      },
    ],
  };
  const bridge = new Proxy(base, {
    get(target, prop, recv) {
      if (prop === 'readRenderDiffs') {
        return () => frame;
      }
      return bridgeProxyValue(target, prop, recv);
    },
  });
  const renderer = countingRenderer();
  const shell = composeAppShell({
    host: { name: 'h', accessibility: false },
    bootBridge: () => ({ bridge, mode: 'native', intent: 'authority', nativeAvailable: true }),
    fixtures: FIXTURES,
    renderer,
  });
  const status = shell.projectAuthority();
  assert.equal(renderer.frames.length, 1);
  assert.equal(status.applied, true);
  assert.equal(status.source, 'authority');
  assert.equal(status.sceneNodes, 1);
  // Sanity: the bridge cursor read happened (no throw) and the readout is coherent.
  assert.doesNotThrow(() => bridge.readRenderDiffs(frameCursor(0)));
});

void test('pick with no bridge clears selection and misses (fail closed)', () => {
  const shell = composeAppShell({
    host: { name: 'h', accessibility: false },
    bootBridge: () => ({ bridge: null, mode: 'native', intent: 'authority', nativeAvailable: false }),
    fixtures: FIXTURES,
  });
  shell.controller.store.dispatch({ type: 'setSelection', selection: { voxel: { x: 1, y: 1, z: 1 }, face: 'posX' } });
  const ray: PickRay = { grid: 1, origin: [0, 0, 0], direction: [1, 0, 0], maxDistance: 10 };
  const result = shell.pick(ray);
  assert.equal(result.outcome, 'miss');
  assert.equal(shell.controller.store.getState().selection, null);
});

void test('formatReadout renders a stable, navigable text report', () => {
  const shell = composeAppShell({
    host: { name: 'headless', accessibility: false },
    bootBridge: referenceBoot,
    fixtures: FIXTURES,
    renderer: countingRenderer(),
  });
  shell.loadActiveFixture();
  shell.projectAuthority();
  const text = formatReadout(shell.readout());
  assert.match(text, /runtime: reference/);
  assert.match(text, /world: launch-grid loaded=true/);
  assert.match(text, /renderer: present=true/);
  assert.match(text, /controls: tool\[radiogroup\]/);
});

void test('empty fixture catalog is rejected at composition time', () => {
  assert.throws(
    () =>
      composeAppShell({ host: { name: 'h', accessibility: false }, bootBridge: referenceBoot, fixtures: [] }),
    /non-empty fixture catalog/,
  );
});
