import assert from 'node:assert/strict';
import test from 'node:test';

import { RuntimeBridgeError } from './bridge.js';
import { classifyNativeAddonError } from './native.js';
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
