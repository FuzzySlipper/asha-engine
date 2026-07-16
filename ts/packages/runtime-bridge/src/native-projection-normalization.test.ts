import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { NativeAddon } from '@asha/native-bridge';

import { frameCursor } from './bridge.js';
import { NativeRuntimeBridge } from './native.js';

void test('native projection decoding restores nullable contract fields omitted by napi', () => {
  const addon = {
    initializeEngine: () => 1,
    readProjectionFrame: (_handle: number, cursor: number) => ({
      schemaVersion: 1,
      authorityTick: cursor,
      scene: { ops: [] },
      presentation: {
        replayScope: 'excludedFromReplayTruth',
        ops: [
          {
            domain: 'audio',
            meta: {
              sequence: 0,
              origin: { kind: 'ownerFact', id: 'fact:fire', authorityTick: cursor },
            },
            audioOp: {
              op: 'emit',
              signalId: 'primary-fire:1',
              descriptor: {
                clip: { asset: 'audio/fire', contentHash: 'sha256:fire' },
                bus: 'sfx',
                volume: 1,
                pitch: 1,
                looping: false,
                spatialBlend: 1,
                attenuation: 20,
                pan: 0,
                emitter: { kind: 'world3d', position: [0, 1, 2] },
              },
            },
          },
          {
            domain: 'billboard',
            meta: { sequence: 1 },
            billboardOp: {
              op: 'create',
              handle: 4,
              descriptor: {
                anchor: { kind: 'entityAttached', entity: 20, offset: [0, 1, 0] },
                content: {
                  kind: 'value',
                  labelKey: 'enemy.health',
                  fallbackLabel: 'Enemy health',
                  value: '5/45',
                },
                font: { kind: 'system', family: 'sans-serif' },
                heightPixels: 24,
                color: [1, 1, 1, 1],
                background: [0, 0, 0, 1],
                maxDistance: 45,
                layer: 'occluded',
                visible: true,
              },
            },
          },
          {
            domain: 'audio',
            meta: { sequence: 2 },
            audioOp: { op: 'update', handle: 7, patch: { volume: 0.5 } },
          },
          {
            domain: 'particle',
            meta: { sequence: 3 },
            particleOp: { op: 'update', handle: 8, patch: { visible: false } },
          },
          {
            domain: 'telemetryOverlay',
            meta: { sequence: 4 },
            telemetryOverlayOp: { op: 'update', handle: 9, patch: { title: 'Combat' } },
          },
        ],
      },
    }),
  } as unknown as NativeAddon;
  const bridge = new NativeRuntimeBridge(addon);
  bridge.initializeEngine({ seed: 1 });

  const frame = bridge.readProjectionFrame(frameCursor(0));
  assert.deepEqual(frame.presentation.ops[0]?.meta.origin, {
    kind: 'ownerFact',
    id: 'fact:fire',
    authorityTick: 0,
    causationId: null,
    correlationId: null,
  });
  assert.equal(frame.presentation.ops[1]?.meta.origin, null);
  const billboard = frame.presentation.ops[1];
  assert.equal(billboard?.domain, 'billboard');
  if (billboard?.domain === 'billboard' && billboard.op.op === 'create') {
    assert.deepEqual(billboard.op.descriptor.content, {
      kind: 'value',
      labelKey: 'enemy.health',
      fallbackLabel: 'Enemy health',
      value: '5/45',
      unitKey: null,
      fallbackUnit: null,
    });
  }
  const audioUpdate = frame.presentation.ops[2];
  assert.equal(audioUpdate?.domain, 'audio');
  if (audioUpdate?.domain === 'audio' && audioUpdate.op.op === 'update') {
    assert.deepEqual(audioUpdate.op.patch, {
      volume: 0.5,
      pitch: null,
      looping: null,
      spatialBlend: null,
      attenuation: null,
      pan: null,
      emitter: null,
    });
  }
  const particleUpdate = frame.presentation.ops[3];
  assert.equal(particleUpdate?.domain, 'particle');
  if (particleUpdate?.domain === 'particle' && particleUpdate.op.op === 'update') {
    assert.equal(particleUpdate.op.patch.visible, false);
    assert.equal(particleUpdate.op.patch.anchor, null);
    assert.equal(particleUpdate.op.patch.maxParticles, null);
  }
  const telemetryUpdate = frame.presentation.ops[4];
  assert.equal(telemetryUpdate?.domain, 'telemetryOverlay');
  if (telemetryUpdate?.domain === 'telemetryOverlay' && telemetryUpdate.op.op === 'update') {
    assert.deepEqual(telemetryUpdate.op.patch, {
      title: 'Combat',
      corner: null,
      refreshIntervalMs: null,
      maxFrameTimeSamples: null,
      visible: null,
    });
  }
});
