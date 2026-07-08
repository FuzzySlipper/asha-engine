import { test } from 'node:test';
import assert from 'node:assert/strict';

import type { DiagnosticReport, RendererResourceReport } from '@asha/contracts';

import {
  buildOperatorConsole,
  classifyLane,
  formatOperatorConsole,
  toOperatorJson,
  type OperatorConsoleInput,
} from './operator-console.js';

function diag(over: Partial<DiagnosticReport>): DiagnosticReport {
  return {
    scope: 'worldComposition',
    severity: 'error',
    code: 'roundTripMismatch',
    reference: 'x',
    source: {
      sceneNodeId: null,
      runtimeEntityId: null,
      assetId: null,
      chunkCoord: null,
      renderHandle: null,
      bundlePath: null,
    },
    message: 'm',
    remedy: null,
    ...over,
  };
}

const resources: RendererResourceReport = {
  liveHandles: 4,
  geometries: 3,
  materials: 2,
  spriteInstances: 0,
  spritesUpdatedLastTick: 0,
  resourcesCreated: 5,
  resourcesDisposed: 5,
  fallbackMaterials: 1,
};

function canonical(): OperatorConsoleInput {
  return {
    runtime: {
      mode: 'reference',
      loadedProjectBundleId: 7,
      worldHash: '9d281709bd588a99',
      protocolVersion: 1,
      schemaVersion: 1,
      capabilities: [
        { operation: 'submitCommands', available: true, note: null },
        { operation: 'pickVoxel', available: true, note: null },
      ],
    },
    diagnostics: { reports: [] },
    sourceTraces: [
      { renderHandle: 1000, sceneNodeId: 1, runtimeEntityId: 1, assetId: 'mesh/a', assetResolved: true },
    ],
    resources,
    persistence: {
      operation: 'save',
      status: 'ok',
      worldHash: '9d281709bd588a99',
      artifactRoles: ['sceneDocument', 'sessionStateSnapshot'],
      detail: null,
    },
    policy: null,
    commands: [{ source: 'ui', accepted: 2, rejected: 0, affected: ['entity:1'] }],
    limitations: [
      { id: 'native-submit', lane: 'bridge', summary: 'native submit unwired', activeInMode: false },
    ],
  };
}

void test('classifyLane routes scope + code overrides to owning lanes', () => {
  assert.equal(classifyLane(diag({ scope: 'scene' })), 'stateRules');
  assert.equal(classifyLane(diag({ scope: 'assetCatalog' })), 'assetCatalog');
  assert.equal(classifyLane(diag({ scope: 'renderProjection' })), 'renderProjection');
  assert.equal(classifyLane(diag({ scope: 'rendererResources' })), 'rendererResources');
  assert.equal(classifyLane(diag({ scope: 'worldBundle' })), 'persistenceReplay'); // vocab-allow: generated diagnostic scope keeps legacy name until #5049.
  assert.equal(classifyLane(diag({ scope: 'worldComposition' })), 'persistenceReplay');
  // Protocol mismatch overrides scope.
  assert.equal(classifyLane(diag({ scope: 'worldBundle', code: 'manifestProtocolMismatch' })), 'protocolContracts'); // vocab-allow: generated diagnostic scope keeps legacy name until #5049.
});

void test('canonical fixture populates every section and reports ready', () => {
  const model = buildOperatorConsole(canonical());
  assert.equal(model.ready, true);
  assert.equal(model.runtime.mode, 'reference');
  assert.equal(model.laneFailures.length, 0);
  assert.equal(model.sourceTraces[0]!.broken, false);
  assert.equal(model.resources!.suspectedLeak, false);
  assert.equal(model.persistence!.artifactRoles.includes('sessionStateSnapshot'), true);
  assert.equal(model.commands.length, 1);

  // The export is stable, agent-parseable JSON round-tripping the model.
  const json = toOperatorJson(model);
  assert.deepEqual(JSON.parse(json), model);
  assert.equal(toOperatorJson(buildOperatorConsole(canonical())), json); // deterministic
});

void test('a failure case classifies by lane, marks not-ready, and flags broken traces + leak', () => {
  const input: OperatorConsoleInput = {
    ...canonical(),
    runtime: { ...canonical().runtime, mode: 'degraded' },
    diagnostics: {
      reports: [
        diag({ scope: 'assetCatalog', code: 'missingAsset', severity: 'error' }),
        diag({ scope: 'worldComposition', code: 'roundTripMismatch', severity: 'fatal' }),
      ],
    },
    sourceTraces: [
      { renderHandle: 1001, sceneNodeId: 2, runtimeEntityId: null, assetId: 'mesh/missing', assetResolved: false },
    ],
    resources: { ...resources, resourcesCreated: 20, resourcesDisposed: 2 },
  };
  const model = buildOperatorConsole(input);
  assert.equal(model.ready, false); // fatal present
  const lanes = model.laneFailures.map((l) => l.lane);
  assert.deepEqual(lanes, ['assetCatalog', 'persistenceReplay']); // sorted
  assert.equal(model.laneFailures.find((l) => l.lane === 'persistenceReplay')!.maxSeverity, 'fatal');
  assert.equal(model.sourceTraces[0]!.broken, true);
  assert.equal(model.resources!.suspectedLeak, true); // 20-2 > 4 live handles

  const text = formatOperatorConsole(model);
  assert.ok(text.some((l) => l.includes('lane persistenceReplay failures=1 maxSeverity=fatal')));
  assert.ok(text.some((l) => l.includes('runtime mode=degraded') && l.includes('ready=false')));
});
