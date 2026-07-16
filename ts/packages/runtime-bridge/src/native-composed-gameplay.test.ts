import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { NativeAddon } from '@asha/native-bridge';
import { NativeRuntimeBridge, RuntimeBridgeError } from './index.js';
import { createNativeComposedGameplayHandlers } from './native-composed-gameplay.test-fixture.js';

const HASH_A = 'fnv1a64:00000000000000aa';
const HASH_B = 'fnv1a64:00000000000000bb';
const HASH_C = 'fnv1a64:00000000000000cc';

function composedAddon(calls: string[]): NativeAddon {
  return {
    initializeEngine: (seed: number) => {
      calls.push(`initialize:${seed}`);
      return seed + 100;
    },
    ...createNativeComposedGameplayHandlers(calls, HASH_A, HASH_B, HASH_C),
  } as unknown as NativeAddon;
}

void test('native facade routes composed evidence views and prefab interaction', () => {
  const calls: string[] = [];
  const bridge = new NativeRuntimeBridge(composedAddon(calls));
  bridge.initializeEngine({ seed: 1 });

  const composed = bridge.readComposedRuntimeSession();
  assert.equal(composed.runtimeSessionHash, HASH_A);
  assert.equal(composed.gameplay.semanticCompatibilityDigest, HASH_B);
  assert.equal(composed.gameplay.artifactProvenanceDigest, HASH_A);
  assert.equal(composed.gameplay.compositionLoadMode, 'compatible');
  const moduleView = bridge.readGameplayModuleView({
    view: { namespace: 'fixture.pulse', name: 'pulse-state-view', version: 1, schemaHash: HASH_A },
    scope: { kind: 'session' },
    expectedRuntimeSessionHash: HASH_A,
  });
  assert.equal(new TextDecoder().decode(Uint8Array.from(moduleView.canonicalPayload)), '4');
  const interaction = bridge.applyGameplayPrefabPartInteraction({
    actor: 101,
    instance: 700,
    role: 'interaction/target',
    expectedTarget: 777,
    tick: 12,
    expectedRuntimeSessionHash: HASH_A,
  });
  assert.equal(interaction.target, 777);
  assert.equal(interaction.reactionFrameHash, HASH_C);
  assert.deepEqual(calls, [
    'initialize:1',
    'composedRead',
    'moduleView:fixture.pulse:pulse-state-view:session:none',
    'prefabInteraction:101:700:interaction/target:777',
  ]);
});

void test('native facade canonicalizes omitted napi Option hashes as null', () => {
  const calls: string[] = [];
  const addon = composedAddon(calls);
  const readComposed = addon.readComposedRuntimeSession;
  addon.readComposedRuntimeSession = (handle) => {
    const value = readComposed(handle);
    const gameplay = { ...value.gameplay } as Record<string, unknown>;
    delete gameplay['lastReactionFrameHash'];
    delete gameplay['lastDecisionReceiptHash'];
    const omitted = { ...value, gameplay } as Record<string, unknown>;
    delete omitted['fpsReplayHash'];
    return omitted as unknown as ReturnType<typeof readComposed>;
  };
  const bridge = new NativeRuntimeBridge(addon);
  bridge.initializeEngine({ seed: 1 });

  const readout = bridge.readComposedRuntimeSession();
  assert.equal(readout.fpsReplayHash, null);
  assert.equal(readout.gameplay.lastReactionFrameHash, null);
  assert.equal(readout.gameplay.lastDecisionReceiptHash, null);
});

void test('composed gameplay requests and native identities fail closed', () => {
  const calls: string[] = [];
  const addon = composedAddon(calls);
  const bridge = new NativeRuntimeBridge(addon);
  bridge.initializeEngine({ seed: 1 });

  assert.throws(
    () => bridge.readGameplayModuleView({
      view: { namespace: '', name: 'pulse-state-view', version: 1, schemaHash: HASH_A },
      scope: { kind: 'session' },
      expectedRuntimeSessionHash: HASH_A,
    }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
  assert.throws(
    () => bridge.readGameplayModuleView({
      view: { namespace: 'fixture.pulse', name: 'pulse-state-view', version: 1, schemaHash: HASH_A },
      scope: { kind: 'entity', entity: -1 },
      expectedRuntimeSessionHash: HASH_A,
    }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
  assert.throws(
    () => bridge.applyGameplayPrefabPartInteraction({
      actor: 101,
      instance: 700,
      role: '   ',
      expectedTarget: 777,
      tick: 12,
      expectedRuntimeSessionHash: HASH_A,
    }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
  assert.deepEqual(calls, ['initialize:1']);

  const readModuleView = addon.readGameplayModuleView;
  addon.readGameplayModuleView = (...input) => ({
    ...readModuleView(...input),
    view: { namespace: 'fixture.tampered', name: input[2], version: input[3], schemaHash: input[4] },
  });
  assert.throws(
    () => bridge.readGameplayModuleView({
      view: { namespace: 'fixture.pulse', name: 'pulse-state-view', version: 1, schemaHash: HASH_A },
      scope: { kind: 'session' },
      expectedRuntimeSessionHash: HASH_A,
    }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'internal',
  );

  const applyInteraction = addon.applyGameplayPrefabPartInteraction;
  addon.applyGameplayPrefabPartInteraction = (...input) => ({
    ...applyInteraction(...input),
    target: input[4] + 1,
  });
  assert.throws(
    () => bridge.applyGameplayPrefabPartInteraction({
      actor: 101,
      instance: 700,
      role: 'interaction/target',
      expectedTarget: 777,
      tick: 12,
      expectedRuntimeSessionHash: HASH_A,
    }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'internal',
  );
});
