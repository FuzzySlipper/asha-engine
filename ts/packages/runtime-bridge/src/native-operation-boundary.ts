import type { GeneratedWireValue } from '@asha/contracts';

import { RuntimeBridgeError, type RuntimeBridgeErrorKind } from './bridge.js';
import {
  parseOperationOutput,
  validateOperationInput,
  validateOperationOutput,
} from './wire-validation.js';

export type NativeFacadeValue = object | boolean | number | string | null | undefined;

const RUNTIME_BRIDGE_ERROR_KINDS: ReadonlySet<string> = new Set([
  'not_initialized',
  'invalid_input',
  'unknown_handle',
  'buffer_expired',
  'native_unavailable',
  'voxel_conversion_unavailable',
  'unsupported_source_asset',
  'source_hash_mismatch',
  'invalid_material_map',
  'output_limit_exceeded',
  'stale_authority_snapshot',
  'conversion_replay_mismatch',
  'operation_unimplemented',
  'internal',
]);

const NATIVE_ERROR_KEYS = new Set([
  'schemaVersion',
  'code',
  'operation',
  'path',
  'retryable',
  'message',
  'details',
  'provenance',
]);

interface NativeErrorEnvelope {
  readonly code: RuntimeBridgeErrorKind;
  readonly details: readonly string[];
  readonly message: string;
  readonly operation: string;
  readonly path: string;
  readonly provenance: 'native_rust';
  readonly retryable: boolean;
}

let activeNativeOperation: string | null = null;

function boundedText(value: string, maxLength: number): string {
  return value.length <= maxLength ? value : `${value.slice(0, maxLength - 1)}…`;
}

function parseNativeErrorEnvelope(message: string): NativeErrorEnvelope | null {
  let parsed: GeneratedWireValue;
  try {
    parsed = JSON.parse(message) as GeneratedWireValue;
  } catch {
    return null;
  }
  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) return null;
  const envelope = parsed as Readonly<Record<string, GeneratedWireValue>>;
  if (Object.keys(envelope).some((key) => !NATIVE_ERROR_KEYS.has(key))) return null;
  if (envelope['schemaVersion'] !== 1) return null;
  if (typeof envelope['code'] !== 'string' || !RUNTIME_BRIDGE_ERROR_KINDS.has(envelope['code'])) return null;
  if (typeof envelope['operation'] !== 'string' || envelope['operation'].length === 0) return null;
  if (typeof envelope['path'] !== 'string' || envelope['path'].length === 0) return null;
  if (typeof envelope['retryable'] !== 'boolean') return null;
  if (typeof envelope['message'] !== 'string' || envelope['message'].length === 0) return null;
  if (envelope['provenance'] !== 'native_rust') return null;
  if (!Array.isArray(envelope['details']) || envelope['details'].some((detail) => typeof detail !== 'string')) {
    return null;
  }
  return {
    code: envelope['code'] as RuntimeBridgeErrorKind,
    details: envelope['details'].slice(0, 8).map((detail) => boundedText(detail as string, 128)),
    message: boundedText(envelope['message'], 512),
    operation: boundedText(envelope['operation'], 128),
    path: boundedText(envelope['path'], 256),
    provenance: 'native_rust',
    retryable: envelope['retryable'],
  };
}

export function classifyNativeAddonError(
  cause: RuntimeBridgeError | Error | string | object,
): RuntimeBridgeError {
  if (cause instanceof RuntimeBridgeError) return cause;
  const message = cause instanceof Error ? cause.message : String(cause);
  const envelope = parseNativeErrorEnvelope(message);
  if (envelope !== null) {
    return new RuntimeBridgeError(envelope.code, envelope.message, {
      details: envelope.details,
      operation: activeNativeOperation ?? envelope.operation,
      path: envelope.path,
      provenance: envelope.provenance,
      retryable: envelope.retryable,
    });
  }
  return new RuntimeBridgeError('internal', boundedText(message, 512), {
    details: ['invalid_native_error_envelope'],
    operation: activeNativeOperation ?? 'native_bridge',
    path: '$',
    provenance: 'transport_loader',
    retryable: false,
  });
}

export function callNative<T>(body: () => T): T {
  try {
    return body();
  } catch (cause) {
    throw classifyNativeAddonError(cause as RuntimeBridgeError | Error | string | object);
  }
}

export function parseNativeJson<T extends object>(payload: string, field: string): T {
  if (activeNativeOperation === null) {
    throw new RuntimeBridgeError('internal', `native ${field} was decoded outside an operation boundary`);
  }
  return parseOperationOutput<T>(activeNativeOperation, payload);
}

export function runNativeOperation(
  operation: string,
  input: NativeFacadeValue,
  body: () => NativeFacadeValue,
): NativeFacadeValue {
  validateOperationInput(operation, input ?? null);
  const previousOperation = activeNativeOperation;
  activeNativeOperation = operation;
  try {
    const output = body();
    validateOperationOutput(operation, output ?? null);
    return output;
  } finally {
    activeNativeOperation = previousOperation;
  }
}

export function nativeUnimplemented(manifestName: string): RuntimeBridgeError {
  return new RuntimeBridgeError(
    'operation_unimplemented',
    `native bridge operation '${manifestName}' is not wired; the native facade is ` +
      `fail-closed (no mock fallback). Wire its #[napi] export and add it to ` +
      `NATIVE_WIRED_OPERATIONS.`,
  );
}
