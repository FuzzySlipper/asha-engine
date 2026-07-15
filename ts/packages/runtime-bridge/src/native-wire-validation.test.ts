import assert from 'node:assert/strict';
import test from 'node:test';
import type { NativeAddon } from '@asha/native-bridge';

import { RuntimeBridgeError } from './bridge.js';
import { MANIFEST_OPERATIONS } from './generated/operations.js';
import {
  NATIVE_WIRED_OPERATIONS,
  NativeRuntimeBridge,
  classifyNativeAddonError,
} from './native.js';
import {
  parseOperationOutput,
  validateOperationInput,
} from './wire-validation.js';

function assertWireRejection(
  action: () => void,
  kind: 'internal' | 'invalid_input',
  detail: string,
): void {
  assert.throws(
    action,
    (error: object) =>
      error instanceof RuntimeBridgeError &&
      error.kind === kind &&
      error.details.includes(detail) &&
      error.path !== null,
  );
}

void test('generated native input contracts reject scalar and tagged-union drift', () => {
  assertWireRejection(
    () => validateOperationInput('initialize_engine', { seed: '1' } as object),
    'invalid_input',
    'wrong_type',
  );
  assertWireRejection(
    () => validateOperationInput('apply_time_control_command', { operation: 'rewind' }),
    'invalid_input',
    'unknown_variant',
  );
  assertWireRejection(
    () => validateOperationInput('apply_time_control_command', { operation: 'pause', extra: true }),
    'invalid_input',
    'unknown_field',
  );
});

void test('handle, lifecycle, and gameplay inputs fail closed before native invocation', () => {
  assertWireRejection(
    () => validateOperationInput('get_buffer', -1),
    'invalid_input',
    'noncanonical_identifier',
  );
  assertWireRejection(
    () => validateOperationInput('read_fps_encounter_director', {
      outcomeKind: 'in_progress',
      terminal: false,
      enemyDead: false,
      playerDead: false,
    }),
    'invalid_input',
    'missing_field',
  );
  assertWireRejection(
    () => validateOperationInput('apply_generated_tunnel_to_runtime_world', {
      preset: 'tiny-enclosed',
      seed: 1,
      authorityEscape: true,
    }),
    'invalid_input',
    'unknown_field',
  );
});

void test('operation limits and tampered native responses reject with typed evidence', () => {
  const oversized = { commands: [], padding: 'x'.repeat(2 * 1024 * 1024) };
  assertWireRejection(
    () => validateOperationInput('submit_commands', oversized),
    'invalid_input',
    'payload_too_large',
  );
  assertWireRejection(
    () => parseOperationOutput('step_simulation', '{"tick":1,"diffCount":"four"}'),
    'internal',
    'wrong_type',
  );
  assertWireRejection(
    () => parseOperationOutput('step_simulation', '{"tick":1,"diffCount":4,"extra":true}'),
    'internal',
    'unknown_field',
  );
});

void test('custom native response contracts validate nested values recursively', () => {
  assertWireRejection(
    () => parseOperationOutput('read_render_diffs', JSON.stringify({
      ops: [{ op: 'replaceMeshPayload', handle: 1, payload: { garbage: true } }],
    })),
    'internal',
    'unknown_field',
  );
  assertWireRejection(
    () => parseOperationOutput('read_voxel_mesh_evidence', JSON.stringify({
      grid: 1,
      fixtureId: 'fixture',
      voxelStateHash: 'fnv1a64:0000000000000001',
      meshingStrategy: 'greedy',
      chunks: [{ garbage: true }],
      diagnostics: [],
    })),
    'internal',
    'unknown_field',
  );
  assertWireRejection(
    () => parseOperationOutput('get_buffer', JSON.stringify({ handle: 1, bytes: [0, 256] })),
    'internal',
    'out_of_range',
  );
  assertWireRejection(
    () => parseOperationOutput('read_fps_runtime_session', JSON.stringify({
      backend: 'native_rust',
      authoritySurface: 'runtime_session.fps.v0',
      projectBundle: 'fixture',
      sessionEpoch: 1,
      lifecycleStatus: { state: 'active' },
      playerEntity: 1,
      enemyEntity: 2,
      health: [{ garbage: true }],
      policyBindings: [],
      replayRecords: [],
      readSets: [],
      entityHash: 'fnv1a64:0000000000000001',
      healthHash: 'fnv1a64:0000000000000002',
      replayHash: 'fnv1a64:0000000000000003',
    })),
    'internal',
    'unknown_field',
  );
  assertWireRejection(
    () => parseOperationOutput('validate_game_rule_catalog', JSON.stringify({
      accepted: false,
      catalogHash: 'fnv1a64:0000000000000001',
      diagnostics: [{ garbage: true }],
      trace: [],
      evidence: [],
    })),
    'internal',
    'unknown_field',
  );
});

void test('native errors decode only from the structured envelope', () => {
  const structured = classifyNativeAddonError(new Error(JSON.stringify({
    schemaVersion: 1,
    code: 'invalid_input',
    operation: 'load_project_bundle',
    path: '$.bundleSchemaVersion',
    retryable: false,
    message: 'unsupported bundle schema',
    details: ['unsupported_schema'],
    provenance: 'native_rust',
  })));
  assert.equal(structured.kind, 'invalid_input');
  assert.equal(structured.operation, 'load_project_bundle');
  assert.equal(structured.path, '$.bundleSchemaVersion');
  assert.deepEqual(structured.details, ['unsupported_schema']);
  assert.equal(structured.provenance, 'native_rust');

  const legacy = classifyNativeAddonError(new Error('InvalidInput: legacy prose'));
  assert.equal(legacy.kind, 'internal');
  assert.deepEqual(legacy.details, ['invalid_native_error_envelope']);
});

void test('native addon semantic errors retain the active public operation', () => {
  const addon = {
    initializeEngine: () => 1,
    loadProjectBundle: () => {
      throw new Error(JSON.stringify({
        schemaVersion: 1,
        code: 'invalid_input',
        operation: 'load_project_bundle',
        path: '$.bundleSchemaVersion',
        retryable: false,
        message: 'unsupported bundle schema 99 / protocol 1',
        details: ['unsupported_schema'],
        provenance: 'native_rust',
      }));
    },
  } as unknown as NativeAddon;
  const bridge = new NativeRuntimeBridge(addon);
  bridge.initializeEngine({ seed: 1 });

  assert.throws(
    () => bridge.loadProjectBundle({ bundleSchemaVersion: 99, protocolVersion: 1, sceneId: 1 }),
    (error: unknown) =>
      error instanceof RuntimeBridgeError &&
      error.kind === 'invalid_input' &&
      error.operation === 'load_project_bundle' &&
      error.path === '$.bundleSchemaVersion' &&
      error.message.includes('unsupported bundle schema 99 / protocol 1'),
  );
});

void test('wired native names are real manifest operations', () => {
  const manifestNames = new Set(MANIFEST_OPERATIONS.map((operation) => operation.manifestName));
  for (const name of NATIVE_WIRED_OPERATIONS) {
    assert.ok(manifestNames.has(name), `${name} in NATIVE_WIRED_OPERATIONS is not a manifest op`);
  }
});
