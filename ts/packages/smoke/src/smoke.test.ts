// Smoke harness tests: a passing mock run carries trustworthy evidence; failures
// are categorized to the exact subsystem (#2395/#2396/#2397/#2398).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  RuntimeBridgeError,
  type RuntimeBridge,
} from '@asha/runtime-bridge';
import { createMockRuntimeBridge } from '@asha/runtime-bridge/reference';

import { authorityBootBridge, runSmoke } from './harness.js';
import { formatResult } from './result.js';
import {
  FIXTURE_PROJECT_ID,
  FIXTURE_PROJECT_MANIFEST_HASH,
  fixtureEditUpdateFrame,
  fixtureRenderFrame,
} from './fixtures.js';

/** The canonical 10-stage launchable-voxel proof order (task #2441). */
const STAGE_ORDER = [
  'boot',
  'load',
  'render',
  'pick',
  'preview',
  'command-submit',
  'authority-classify',
  'render-update',
  'close-reload-replay',
  'cleanup',
];

function mockBoot() {
  return {
    bridge: createMockRuntimeBridge(),
    mode: 'mock' as const,
    intent: 'reference' as const,
    nativeAvailable: false,
  };
}

void test('mock run passes and reports trustworthy evidence', async () => {
  const result = await runSmoke({ bootBridge: mockBoot });
  assert.equal(result.ok, true);
  assert.equal(result.runtimeMode, 'mock');
  assert.equal(result.smokeMode, 'reference');
  assert.equal(result.outcome, 'mock_reference_passed');
  assert.equal(result.nativeAvailable, false);
  // Capabilities honestly distinguish real (renderer) from mock-backed.
  assert.equal(result.capabilities.renderer, 'ok');
  assert.equal(result.capabilities.projectLoad, 'mock');
  assert.equal(result.capabilities.projection, 'mock');
  // Deterministic fixture evidence.
  assert.equal(result.fixture.id, FIXTURE_PROJECT_ID);
  assert.equal(result.fixture.manifestHash, FIXTURE_PROJECT_MANIFEST_HASH);
  // The full 10-stage launchable proof ran, every stage green.
  assert.deepEqual(
    result.stages.map((s) => s.name),
    STAGE_ORDER,
  );
  assert.ok(result.stages.every((s) => s.ok));
  assert.equal(result.render.applied, true);
  assert.ok(result.render.sceneNodes > 0);
  assert.equal(result.failures.length, 0);
  // Resource lifecycle is bounded: created handles were destroyed, no leak/buffer held.
  assert.equal(result.counters.leakedHandles, 0);
  assert.equal(result.counters.outstandingBuffers, 0);
  assert.ok(result.counters.peakHandles > 0, 'handles were actually created during the run');
  assert.ok(result.counters.debugNodes > 0, 'a preview overlay was drawn on the debug layer');
});

void test('every required launchable stage is present and ordered', async () => {
  const names = (await runSmoke({ bootBridge: mockBoot })).stages.map((s) => s.name);
  assert.deepEqual(names, STAGE_ORDER, '10-stage proof: boot→…→cleanup');
});

void test('picking stage classifies the reference miss and clears selection (no swallowed error)', async () => {
  const result = await runSmoke({ bootBridge: mockBoot });
  const pick = result.stages.find((s) => s.name === 'pick');
  assert.ok(pick?.ok);
  assert.match(pick!.detail, /classified miss/);
});

void test('preview stage holds the remesh guardrail (debug overlay, scene untouched)', async () => {
  const result = await runSmoke({ bootBridge: mockBoot });
  const preview = result.stages.find((s) => s.name === 'preview');
  assert.ok(preview?.ok, 'preview must pass without remeshing authority');
  assert.match(preview!.detail, /scene unchanged=true/);
  assert.ok(result.counters.debugNodes >= 1);
});

void test('close/reload/replay stage proves the canonical lifecycle through the facade', async () => {
  const result = await runSmoke({ bootBridge: mockBoot });
  const stage = result.stages.find((s) => s.name === 'close-reload-replay');
  assert.ok(stage?.ok);
  assert.match(stage!.detail, /reloaded canonical project/);
  assert.match(stage!.detail, /diverged=false/);
});

void test('a thrown pick surfaces a classified pick_failure, not a generic internal error', async () => {
  const broken = bridgeWith({
    pickVoxel: () => {
      throw new RuntimeBridgeError('invalid_input', 'bad ray');
    },
  });
  const result = await runSmoke({
    bootBridge: () => ({ bridge: broken, mode: 'mock', intent: 'reference', nativeAvailable: false }),
  });
  assert.equal(result.ok, false);
  assert.ok(result.failures.some((f) => f.category === 'pick_failure'));
});

void test('a thrown replay surfaces a classified replay_failure', async () => {
  const broken = bridgeWith({
    runReplayStep: () => {
      throw new RuntimeBridgeError('internal', 'replay engine fault');
    },
  });
  const result = await runSmoke({
    bootBridge: () => ({ bridge: broken, mode: 'mock', intent: 'reference', nativeAvailable: false }),
  });
  assert.equal(result.ok, false);
  assert.ok(result.failures.some((f) => f.category === 'replay_failure'));
});

void test('formatResult is deterministic and lists every stage', async () => {
  const a = formatResult(await runSmoke({ bootBridge: mockBoot }));
  const b = formatResult(await runSmoke({ bootBridge: mockBoot }));
  assert.equal(a, b);
  assert.match(a, /asha-smoke: PASS/);
  assert.match(a, /stage render: ok/);
  assert.match(a, /stage close-reload-replay: ok/);
  assert.match(a, /stage cleanup: ok/);
  assert.match(a, /counters: leakedHandles=0/);
});

void test('reference smoke matches the committed golden snapshot', async () => {
  // dist/smoke.test.js → repo root is four levels up.
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '../../../..');
  const committed = readFileSync(resolve(root, 'harness/fixtures/smoke/reference-smoke.txt'), 'utf8');
  const rendered = formatResult(await runSmoke({ bootBridge: mockBoot }));
  assert.equal(
    rendered,
    committed,
    'reference smoke drifted from harness/fixtures/smoke/reference-smoke.txt; ' +
      'regenerate it from the reference run if the change is intended',
  );
});

/** A bridge that delegates to a real mock but lets one method be overridden. */
function bridgeWith(overrides: Partial<RuntimeBridge>): RuntimeBridge {
  const base = createMockRuntimeBridge();
  return new Proxy(base, {
    get(target, property, receiver) {
      if (Reflect.has(overrides, property)) {
        return Reflect.get(overrides, property) as unknown;
      }
      const value = Reflect.get(target, property, receiver) as unknown;
      if (typeof value !== 'function') return value;
      const method = value as (this: RuntimeBridge, ...args: readonly unknown[]) => unknown;
      return method.bind(target);
    },
  });
}

void test('a rejected canonical project load is categorized to the load subsystem', async () => {
  const failing = bridgeWith({
    loadRuntimeProject: (request) => ({
      accepted: false,
      source: request.source,
      activeProject: null,
      lifecycle: request.expectedLifecycle,
      diagnostics: [{
        phase: 'runtimeAdmission',
        code: 'fixture_rejected',
        documentId: null,
        path: null,
        message: 'fixture rejected',
      }],
    }),
  });
  const result = await runSmoke({
    bootBridge: () => ({ bridge: failing, mode: 'mock', intent: 'reference', nativeAvailable: false }),
  });
  assert.equal(result.ok, false);
  assert.equal(result.outcome, 'failed');
  assert.equal(result.capabilities.projectLoad, 'unavailable');
  const loadFailure = result.failures.find((f) => f.category === 'load_failure');
  assert.ok(loadFailure, 'expected a classified load_failure');
  assert.ok(loadFailure!.nextStep.length > 0, 'failure carries an actionable next step');
});

void test('a thrown bridge load surfaces a classified failure', async () => {
  const throwing = bridgeWith({
    loadRuntimeProject: () => {
      throw new RuntimeBridgeError('invalid_input', 'bad bundle');
    },
  });
  const result = await runSmoke({
    bootBridge: () => ({ bridge: throwing, mode: 'mock', intent: 'reference', nativeAvailable: false }),
  });
  assert.equal(result.ok, false);
  assert.ok(result.failures.some((f) => f.category === 'load_failure'));
});

// ── Authority-path smoke (#2424) ──────────────────────────────────────────────

/** An authority-capable bridge: a mock that serves real render diffs through the
 *  facade, standing in for a wired native runtime in tests. The cursor advances —
 *  cursor 0 yields the initial projection (creates); later cursors yield the post-edit
 *  update — so the render-update stage applies an UPDATE, not a duplicate create. */
function authorityBridge(): RuntimeBridge {
  return bridgeWith({
    readRenderDiffs: (cursor) => ((cursor as number) === 0 ? fixtureRenderFrame() : fixtureEditUpdateFrame()),
  });
}

void test('authority run reads diffs through the facade and earns native_authority_passed', async () => {
  const result = await runSmoke({
    bootBridge: () => ({
      bridge: authorityBridge(),
      mode: 'native',
      intent: 'authority',
      nativeAvailable: true,
    }),
  });
  assert.equal(result.ok, true);
  assert.equal(result.smokeMode, 'authority');
  assert.equal(result.outcome, 'native_authority_passed');
  // Capabilities report real (not mock) once the authority path passes.
  assert.equal(result.capabilities.projectLoad, 'ok');
  assert.equal(result.capabilities.projection, 'ok');
  // The render stage consumed bridge.readRenderDiffs, not the local fixture frame.
  const render = result.stages.find((s) => s.name === 'render');
  assert.ok(render?.detail.includes('bridge.readRenderDiffs'));
  assert.ok(result.render.sceneNodes > 0);
});

void test('authority run fails closed (not blank success) when readRenderDiffs is empty', async () => {
  // A fail-closed native bridge (post-#2423) whose projection is not wired: the
  // mock returns an empty frame; authority intent must classify, not pass.
  const result = await runSmoke({
    bootBridge: () => ({
      bridge: createMockRuntimeBridge(),
      mode: 'native',
      intent: 'authority',
      nativeAvailable: true,
    }),
  });
  assert.equal(result.ok, false);
  assert.equal(result.outcome, 'failed');
  assert.ok(result.failures.some((f) => f.category === 'missing_native_bridge'));
});

void test('authority boot fails closed and honest when the native addon is unavailable', async (t) => {
  // The real authority boot in offline CI: no native addon → classified failure,
  // never downgraded to a mock pass.
  const boot = authorityBootBridge();
  if (boot.bridge !== null) {
    t.skip('native addon is built in this environment; offline-failure path not exercised');
    return;
  }
  assert.equal(boot.nativeAvailable, false);
  const result = await runSmoke({ bootBridge: authorityBootBridge });
  assert.equal(result.ok, false);
  assert.equal(result.smokeMode, 'authority');
  assert.equal(result.outcome, 'failed');
  assert.ok(result.failures.some((f) => f.category === 'missing_native_bridge'));
  assert.equal(result.capabilities.runtimeBridge, 'unavailable');
});

void test('real native authority boot fails closed at an unwired op (no mock success)', async (t) => {
  // When the native addon IS built, the authority path still must not pass on
  // mock behaviour: post-#2423 the native facade fail-closes unwired ops, so the
  // load stage fails honestly rather than reporting a blank success.
  const boot = authorityBootBridge();
  if (boot.bridge === null) {
    t.skip('native addon not built; honest-failure path covered by the offline test');
    return;
  }
  const result = await runSmoke({ bootBridge: authorityBootBridge });
  assert.equal(result.smokeMode, 'authority');
  assert.equal(result.runtimeMode, 'native');
  assert.equal(result.ok, false);
  assert.equal(result.outcome, 'failed');
  assert.ok(result.failures.some((f) => f.category === 'missing_native_bridge'));
});
