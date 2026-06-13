// Smoke harness tests: a passing mock run carries trustworthy evidence; failures
// are categorized to the exact subsystem (#2395/#2396/#2397/#2398).

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  createMockRuntimeBridge,
  RuntimeBridgeError,
  type RuntimeBridge,
} from '@asha/runtime-bridge';

import { runSmoke } from './harness.js';
import { formatResult } from './result.js';
import { FIXTURE_WORLD, fixtureWorldHash } from './fixtures.js';

function mockBoot() {
  return { bridge: createMockRuntimeBridge(), mode: 'mock' as const, nativeAvailable: false };
}

test('mock run passes and reports trustworthy evidence', () => {
  const result = runSmoke({ bootBridge: mockBoot });
  assert.equal(result.ok, true);
  assert.equal(result.runtimeMode, 'mock');
  assert.equal(result.nativeAvailable, false);
  // Capabilities honestly distinguish real (renderer) from mock-backed.
  assert.equal(result.capabilities.renderer, 'ok');
  assert.equal(result.capabilities.worldLoad, 'mock');
  assert.equal(result.capabilities.projection, 'mock');
  // Deterministic fixture evidence.
  assert.equal(result.fixture.id, FIXTURE_WORLD.sceneId);
  assert.equal(result.fixture.worldHash, fixtureWorldHash(FIXTURE_WORLD));
  // Real load → projection → render and edit/save stages all ran.
  assert.deepEqual(
    result.stages.map((s) => s.name),
    ['boot', 'load', 'render', 'edit-save'],
  );
  assert.ok(result.stages.every((s) => s.ok));
  assert.equal(result.render.applied, true);
  assert.ok(result.render.sceneNodes > 0);
  assert.equal(result.failures.length, 0);
});

test('formatResult is deterministic and lists every stage', () => {
  const a = formatResult(runSmoke({ bootBridge: mockBoot }));
  const b = formatResult(runSmoke({ bootBridge: mockBoot }));
  assert.equal(a, b);
  assert.match(a, /asha-smoke: PASS/);
  assert.match(a, /stage render: ok/);
  assert.match(a, /stage edit-save: ok/);
});

/** A bridge that delegates to a real mock but lets one method be overridden. */
function bridgeWith(overrides: Partial<RuntimeBridge>): RuntimeBridge {
  const base = createMockRuntimeBridge();
  return {
    initializeEngine: base.initializeEngine.bind(base),
    stepSimulation: base.stepSimulation.bind(base),
    submitCommands: base.submitCommands.bind(base),
    readRenderDiffs: base.readRenderDiffs.bind(base),
    getBuffer: base.getBuffer.bind(base),
    releaseBuffer: base.releaseBuffer.bind(base),
    loadWorldBundle: base.loadWorldBundle.bind(base),
    saveCurrentWorld: base.saveCurrentWorld.bind(base),
    getCompositionStatus: base.getCompositionStatus.bind(base),
    unloadWorld: base.unloadWorld.bind(base),
    loadReplayFixture: base.loadReplayFixture.bind(base),
    runReplayStep: base.runReplayStep.bind(base),
    ...overrides,
  };
}

test('a failing world load is categorized to the load subsystem, not a blank success', () => {
  const failing = bridgeWith({
    loadWorldBundle: () => ({ loadedWorld: null, fatalCount: 1, totalCount: 1, blocksLoad: true }),
  });
  const result = runSmoke({
    bootBridge: () => ({ bridge: failing, mode: 'mock', nativeAvailable: false }),
  });
  assert.equal(result.ok, false);
  assert.equal(result.capabilities.worldLoad, 'unavailable');
  const loadFailure = result.failures.find((f) => f.category === 'load_failure');
  assert.ok(loadFailure, 'expected a classified load_failure');
  assert.ok(loadFailure!.nextStep.length > 0, 'failure carries an actionable next step');
});

test('a thrown bridge load surfaces a classified failure', () => {
  const throwing = bridgeWith({
    loadWorldBundle: () => {
      throw new RuntimeBridgeError('invalid_input', 'bad bundle');
    },
  });
  const result = runSmoke({
    bootBridge: () => ({ bridge: throwing, mode: 'mock', nativeAvailable: false }),
  });
  assert.equal(result.ok, false);
  assert.ok(result.failures.some((f) => f.category === 'load_failure'));
});
