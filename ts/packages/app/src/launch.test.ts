import { test } from 'node:test';
import assert from 'node:assert/strict';

import { RuntimeBridgeError } from '@asha/runtime-bridge';

import {
  bootForMode,
  defaultFixtures,
  launchShell,
  referenceBoot,
  runHeadlessLaunch,
} from './launch.js';

void test('headless reference launch assembles a coherent, loaded shell readout', () => {
  const readout = runHeadlessLaunch({ mode: 'reference', renderer: null });
  assert.equal(readout.runtime.availability, 'reference');
  assert.equal(readout.host.name, 'headless');
  assert.equal(readout.world.fixtureId, 'launch-grid');
  assert.equal(readout.world.loaded, true);
  // The control model + devtools inspection both come from the one store.
  assert.ok(readout.controls.some((c) => c.id === 'tool'));
  assert.equal(readout.editor.tool, 'place');
});

void test('default fixture catalog is runtime-selectable (more than one)', () => {
  const fixtures = defaultFixtures();
  assert.ok(fixtures.length >= 2);
  assert.deepEqual(fixtures.map((f) => f.id), ['launch-grid', 'alt-grid']);
});

void test('authority launch with no native addon is reported unavailable, never downgraded', () => {
  // Exercise the closed-failure contract by injection so the test is stable whether the
  // local machine has the native addon built or not.
  const bootError = new RuntimeBridgeError('native_unavailable', 'addon unavailable in test');
  const readout = runHeadlessLaunch({
    mode: 'authority',
    renderer: null,
    bootBridge: () => ({
      bridge: null,
      mode: 'native',
      intent: 'authority',
      nativeAvailable: false,
      bootError,
    }),
  });
  assert.equal(readout.runtime.availability, 'unavailable');
  assert.equal(readout.world.loaded, false);
});

void test('bootForMode selects reference vs authority intent', () => {
  assert.equal(referenceBoot().intent, 'reference');
  assert.equal(bootForMode('reference').intent, 'reference');
  assert.equal(bootForMode('authority').intent, 'authority');
});

void test('launchShell returns a live shell that can keep being driven', () => {
  const shell = launchShell({ mode: 'reference', renderer: null });
  shell.applyControl('tool', 'remove');
  assert.equal(shell.editorInspection().tool, 'remove');
});
