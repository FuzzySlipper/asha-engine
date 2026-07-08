// Discrete-GPU/WebGL performance lane schema + skip behavior (#2461).
//
// These tests keep the GPU lane opt-in and non-gating: no local GPU or external
// WebGL calibration is required for the normal test suite.

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { runGpuPerf, GPU_PERF_COMMAND } from './gpu-perf.js';
import { formatPerf, runPerf, type PerfResult } from './perf.js';

function fakeBaseResult(): PerfResult {
  return {
    ok: true,
    meta: {
      schema: 1,
      command: 'base-command',
      commit: 'abc1234',
      branch: 'task-test',
      hostLabel: 'gpu-lab-1',
      runtimeMode: 'mock',
      smokeMode: 'reference',
      fixtureId: 1001,
      fixtureProjectBundleHash: '0123456789abcdef',
      node: 'v0.0.0-test',
      platform: 'linux',
      arch: 'x64',
      cpus: 16,
      cpuModel: 'test cpu',
      totalMemMb: 32768,
      timestamp: '2026-06-15T00:00:00.000Z',
    },
    timings: [{ phase: 'renderer-apply-initial', ms: 1, iterations: 1 }],
    counters: {
      peakHandles: 2,
      leakedHandles: 0,
      sceneNodes: 1,
      overlayCells: 1,
      fallbackMaterials: 0,
      spriteFallbacks: 0,
      commandsAccepted: 4,
      commandsRejected: 0,
      renderOpsApplied: 8,
      editCycles: 1,
      replaySteps: 4,
      replayDiverged: false,
      outstandingBuffers: 0,
    },
    invariants: [{ name: 'no-handle-leak', held: true, detail: 'ok' }],
  };
}

void test('GPU perf run skips with a classified reason when no real GL context is enabled', async () => {
  const result = await runGpuPerf({
    runBasePerf: async () => fakeBaseResult(),
    env: {},
  });

  assert.equal(result.status, 'skipped');
  assert.equal(result.ok, true);
  assert.equal(result.meta.command, GPU_PERF_COMMAND);
  assert.equal(result.meta.lane, 'discrete-gpu-gl-render');
  assert.equal(result.meta.gating, 'non-gating');
  assert.equal(result.skip?.reason, 'gpu_context_not_enabled');
  assert.equal(result.asha, null);
  assert.deepEqual(result.externalCalibrations, []);
});

void test('GPU perf run records host/GPU metadata and omits WebGL calibration without failing', async () => {
  const result = await runGpuPerf({
    runBasePerf: async () => fakeBaseResult(),
    env: {
      ASHA_GPU_PERF_ENABLE: '1',
      ASHA_GPU_PERF_CONTEXT: 'electron-webgl',
      ASHA_GPU_NAME: 'Example RTX',
      ASHA_GPU_DRIVER: '535.0-test',
      ASHA_GPU_BROWSER: 'Chromium 126',
      ASHA_GPU_RUNTIME: 'Electron 31',
    },
  });

  assert.equal(result.status, 'completed');
  assert.equal(result.ok, true);
  assert.equal(result.skip, null);
  assert.equal(result.meta.renderContext, 'electron-webgl');
  assert.equal(result.meta.gpu.name, 'Example RTX');
  assert.equal(result.meta.gpu.driver, '535.0-test');
  assert.equal(result.meta.browser, 'Chromium 126');
  assert.equal(result.meta.runtime, 'Electron 31');
  assert.equal(result.meta.fixtureId, 1001);
  assert.equal(result.externalCalibrations.length, 0);
  assert.equal(result.asha?.counters.leakedHandles, 0);
});

void test('GPU perf run accepts contextual external WebGL calibration as non-gating data', async () => {
  const result = await runGpuPerf({
    runBasePerf: async () => fakeBaseResult(),
    env: {
      ASHA_GPU_PERF_ENABLE: '1',
      ASHA_GPU_PERF_CONTEXT: 'browser-webgl',
      ASHA_GPU_EXTERNAL_CALIBRATION: JSON.stringify([
        { name: 'MotionMark', score: 123.4, unit: 'score', source: 'manual', notes: 'operator supplied' },
      ]),
    },
  });

  assert.equal(result.status, 'completed');
  assert.equal(result.externalCalibrations.length, 1);
  assert.equal(result.externalCalibrations[0]?.gating, 'non-gating');
  assert.equal(result.externalCalibrations[0]?.name, 'MotionMark');
});

void test('same-machine perf run remains independent of the GPU lane', async () => {
  const base = await runPerf({ editCycles: 1, clock: (() => {
    let t = 0;
    return () => t++;
  })() });

  assert.equal(base.meta.command, 'pnpm --filter @asha/smoke dev:asha-perf');
  assert.equal('lane' in base.meta, false);
  assert.match(formatPerf(base), /asha-perf OK|asha-perf FAILED/);
});
