import {
  validateGeneratedWireValue,
  type GeneratedWireValidationResult,
  type GeneratedWireTypeName,
  type GeneratedWireValue,
} from '@asha/contracts';
import { RuntimeBridgeError, type RuntimeBridgeErrorKind } from './bridge.js';
import {
  MANIFEST_OPERATIONS,
  type BridgeOperation,
  type BridgeWireTypeRef,
} from './generated/operations.js';
import {
  CUSTOM_WIRE_SCHEMAS,
  type CustomWireObjectSchema,
  type CustomWireSchema,
} from './custom-wire-schemas.js';

type WireDirection = 'input' | 'output';
type WireCandidate = object | boolean | number | string | null;

const OPERATIONS: ReadonlyMap<string, BridgeOperation> = new Map(
  MANIFEST_OPERATIONS.map((operation) => [operation.manifestName, operation]),
);

function wireError(
  operation: string,
  direction: WireDirection,
  kind: RuntimeBridgeErrorKind,
  path: string,
  detail: string,
  detailCode: string,
): RuntimeBridgeError {
  return new RuntimeBridgeError(kind, `${direction} wire contract rejected: ${detail}`, {
    operation,
    path,
    retryable: false,
    details: [detailCode],
    provenance: 'runtime_facade',
  });
}

function operationContract(operation: string): BridgeOperation {
  const contract = OPERATIONS.get(operation);
  if (contract === undefined) {
    throw wireError(operation, 'input', 'operation_unimplemented', '$', 'unknown operation', 'unknown_operation');
  }
  return contract;
}

function byteLength(payload: string): number {
  return new TextEncoder().encode(payload).byteLength;
}

function stringifyWireValue(value: WireCandidate): string | undefined {
  return JSON.stringify(value, (_key: string, nestedValue: WireCandidate) =>
    nestedValue instanceof Uint8Array ? Array.from(nestedValue) : nestedValue,
  );
}

function validateProjectResourceStageInput(
  operation: string,
  value: WireCandidate,
  maxInputBytes: number,
): void {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw wireError(operation, 'input', 'invalid_input', '$', 'expected object', 'wrong_type');
  }
  const input = value as Readonly<Record<string, unknown>>;
  const allowedFields = new Set(['generation', 'path', 'bytes']);
  const unknownField = Object.keys(input).find((field) => !allowedFields.has(field));
  if (unknownField !== undefined) {
    throw wireError(
      operation,
      'input',
      'invalid_input',
      `$.${unknownField}`,
      'unknown field',
      'unknown_field',
    );
  }
  if (!Number.isSafeInteger(input['generation']) || (input['generation'] as number) < 0) {
    throw wireError(
      operation,
      'input',
      'invalid_input',
      '$.generation',
      'expected non-negative safe integer',
      'noncanonical_identifier',
    );
  }
  if (typeof input['path'] !== 'string' || input['path'].length === 0) {
    throw wireError(operation, 'input', 'invalid_input', '$.path', 'expected nonempty string', 'wrong_type');
  }
  if (!(input['bytes'] instanceof Uint8Array)) {
    throw wireError(
      operation,
      'input',
      'invalid_input',
      '$.bytes',
      'expected Uint8Array',
      'wrong_type',
    );
  }
  // Count a compact metadata envelope plus the raw buffer length. Deliberately
  // do not stringify the byte array: this operation exists to keep large and
  // binary project resources out of JSON/base64 paths.
  const metadataBytes = byteLength(
    JSON.stringify({ generation: input['generation'], path: input['path'] }),
  );
  const actualBytes = metadataBytes + input['bytes'].byteLength;
  if (actualBytes > maxInputBytes) {
    throw wireError(
      operation,
      'input',
      'invalid_input',
      '$.bytes',
      `payload has ${actualBytes} bytes; limit is ${maxInputBytes}`,
      'payload_too_large',
    );
  }
}

function parseJson(
  operation: string,
  direction: WireDirection,
  payload: string,
  maxBytes: number,
): GeneratedWireValue {
  const actualBytes = byteLength(payload);
  if (actualBytes > maxBytes) {
    throw wireError(
      operation,
      direction,
      direction === 'input' ? 'invalid_input' : 'internal',
      '$',
      `payload has ${actualBytes} bytes; limit is ${maxBytes}`,
      'payload_too_large',
    );
  }
  try {
    return JSON.parse(payload) as GeneratedWireValue;
  } catch {
    throw wireError(
      operation,
      direction,
      direction === 'input' ? 'invalid_input' : 'internal',
      '$',
      'payload is not valid JSON',
      'invalid_json',
    );
  }
}

function validationErrorKind(direction: WireDirection): RuntimeBridgeErrorKind {
  return direction === 'input' ? 'invalid_input' : 'internal';
}

function nestedGeneratedPath(path: string, generatedPath: string): string {
  if (path === '$') return generatedPath;
  if (generatedPath === '$') return path;
  return `${path}${generatedPath.slice(1)}`;
}

function validateCustomObject(
  operation: string,
  direction: WireDirection,
  schema: CustomWireObjectSchema,
  value: GeneratedWireValue,
  path: string,
  allowedFields: ReadonlySet<string> = new Set(),
): void {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw wireError(
      operation,
      direction,
      validationErrorKind(direction),
      path,
      'expected object',
      'wrong_type',
    );
  }
  const objectValue = value as Readonly<Record<string, GeneratedWireValue>>;
  const optional = new Set(schema.optional ?? []);
  for (const field of Object.keys(objectValue)) {
    if (schema.fields[field] === undefined && !allowedFields.has(field)) {
      throw wireError(
        operation,
        direction,
        validationErrorKind(direction),
        `${path}.${field}`,
        'unknown field',
        'unknown_field',
      );
    }
  }
  for (const [field, fieldSchema] of Object.entries(schema.fields)) {
    if (!(field in objectValue)) {
      if (optional.has(field)) continue;
      throw wireError(
        operation,
        direction,
        validationErrorKind(direction),
        `${path}.${field}`,
        'missing required field',
        'missing_field',
      );
    }
    validateCustomSchema(
      operation,
      direction,
      fieldSchema,
      objectValue[field] ?? null,
      `${path}.${field}`,
    );
  }
}

function validateCustomSchema(
  operation: string,
  direction: WireDirection,
  schema: CustomWireSchema,
  value: GeneratedWireValue,
  path: string,
): void {
  if (schema.kind === 'nullable') {
    if (value !== null) validateCustomSchema(operation, direction, schema.value, value, path);
    return;
  }
  if (schema.kind === 'array') {
    if (!Array.isArray(value)) {
      throw wireError(operation, direction, validationErrorKind(direction), path, 'expected array', 'wrong_type');
    }
    const arrayValue = value as readonly GeneratedWireValue[];
    for (let index = 0; index < arrayValue.length; index += 1) {
      validateCustomSchema(
        operation,
        direction,
        schema.item,
        arrayValue[index] ?? null,
        `${path}[${index}]`,
      );
    }
    return;
  }
  if (schema.kind === 'tuple') {
    if (!Array.isArray(value) || value.length !== schema.items.length) {
      throw wireError(
        operation,
        direction,
        validationErrorKind(direction),
        path,
        `expected ${schema.items.length}-item tuple`,
        'wrong_type',
      );
    }
    const tupleValue = value as readonly GeneratedWireValue[];
    for (let index = 0; index < schema.items.length; index += 1) {
      const itemSchema = schema.items[index];
      if (itemSchema === undefined) {
        throw wireError(
          operation,
          direction,
          'internal',
          path,
          'tuple schema is incomplete',
          'missing_custom_validator',
        );
      }
      validateCustomSchema(
        operation,
        direction,
        itemSchema,
        tupleValue[index] ?? null,
        `${path}[${index}]`,
      );
    }
    return;
  }
  if (schema.kind === 'object') {
    validateCustomObject(operation, direction, schema, value, path);
    return;
  }
  if (schema.kind === 'taggedUnion') {
    if (typeof value !== 'object' || value === null || Array.isArray(value)) {
      throw wireError(operation, direction, validationErrorKind(direction), path, 'expected object', 'wrong_type');
    }
    const objectValue = value as Readonly<Record<string, GeneratedWireValue>>;
    const tagValue = objectValue[schema.tag];
    if (typeof tagValue !== 'string') {
      throw wireError(
        operation,
        direction,
        validationErrorKind(direction),
        `${path}.${schema.tag}`,
        'expected string discriminator',
        'wrong_type',
      );
    }
    const variant = schema.variants[tagValue];
    if (variant === undefined) {
      throw wireError(
        operation,
        direction,
        validationErrorKind(direction),
        `${path}.${schema.tag}`,
        `unknown variant '${tagValue}'`,
        'unknown_variant',
      );
    }
    validateCustomObject(operation, direction, variant, value, path, new Set([schema.tag]));
    return;
  }
  if (schema.kind === 'custom') {
    const referencedSchema = CUSTOM_WIRE_SCHEMAS[schema.name];
    if (referencedSchema === undefined) {
      throw wireError(
        operation,
        direction,
        'operation_unimplemented',
        path,
        `custom wire validator '${schema.name}' is not registered`,
        'missing_custom_validator',
      );
    }
    validateCustomSchema(operation, direction, referencedSchema, value, path);
    return;
  }
  if (schema.kind === 'generated') {
    const result = validateGeneratedWireValue(schema.name, value);
    if (!result.valid) {
      throw wireError(
        operation,
        direction,
        validationErrorKind(direction),
        nestedGeneratedPath(path, result.issue.path),
        result.issue.message,
        result.issue.code,
      );
    }
    return;
  }
  if (schema.kind === 'boolean') {
    if (typeof value !== 'boolean') {
      throw wireError(operation, direction, validationErrorKind(direction), path, 'expected boolean', 'wrong_type');
    }
    return;
  }
  if (schema.kind === 'string') {
    if (typeof value !== 'string') {
      throw wireError(operation, direction, validationErrorKind(direction), path, 'expected string', 'wrong_type');
    }
    return;
  }
  if (schema.kind === 'enum') {
    if (typeof value !== 'string' || !schema.values.includes(value)) {
      throw wireError(operation, direction, validationErrorKind(direction), path, 'unknown enum variant', 'unknown_variant');
    }
    return;
  }
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    throw wireError(operation, direction, validationErrorKind(direction), path, 'expected finite number', 'wrong_type');
  }
  if (schema.integer === true && !Number.isSafeInteger(value)) {
    throw wireError(
      operation,
      direction,
      validationErrorKind(direction),
      path,
      'expected safe integer',
      'noncanonical_identifier',
    );
  }
  if (
    (schema.minimum !== undefined && value < schema.minimum) ||
    (schema.maximum !== undefined && value > schema.maximum)
  ) {
    throw wireError(operation, direction, validationErrorKind(direction), path, 'number is out of range', 'out_of_range');
  }
}

function validateCustom(
  operation: string,
  direction: WireDirection,
  reference: BridgeWireTypeRef,
  value: GeneratedWireValue,
): void {
  const schema = CUSTOM_WIRE_SCHEMAS[reference.name];
  if (schema === undefined) {
    throw wireError(
      operation,
      direction,
      'operation_unimplemented',
      '$',
      `custom wire validator '${reference.name}' is not registered`,
      'missing_custom_validator',
    );
  }
  validateCustomSchema(operation, direction, schema, value, '$');
}

function validateReference(
  operation: string,
  direction: WireDirection,
  reference: BridgeWireTypeRef,
  value: GeneratedWireValue,
): void {
  if (reference.repeated) {
    if (!Array.isArray(value)) {
      throw wireError(operation, direction, direction === 'input' ? 'invalid_input' : 'internal', '$', 'expected array', 'wrong_type');
    }
    const singular = { ...reference, repeated: false };
    const arrayValue = value as readonly GeneratedWireValue[];
    for (let index = 0; index < arrayValue.length; index += 1) {
      validateReference(operation, direction, singular, arrayValue[index] ?? null);
    }
    return;
  }
  if (reference.owner === 'unit') {
    if (value !== null) {
      throw wireError(operation, direction, direction === 'input' ? 'invalid_input' : 'internal', '$', 'expected null unit value', 'wrong_type');
    }
    return;
  }
  if (reference.owner === 'handle') {
    if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0) {
      throw wireError(operation, direction, direction === 'input' ? 'invalid_input' : 'internal', '$', 'expected non-negative safe-integer handle', 'noncanonical_identifier');
    }
    return;
  }
  if (reference.owner === 'custom') {
    validateCustom(operation, direction, reference, value);
    return;
  }
  const result: GeneratedWireValidationResult = validateGeneratedWireValue(reference.name, value);
  if (!result.valid) {
    throw wireError(
      operation,
      direction,
      direction === 'input' ? 'invalid_input' : 'internal',
      result.issue.path,
      result.issue.message,
      result.issue.code,
    );
  }
}

export function serializeOperationInput(operation: string, value: WireCandidate): string {
  const contract = operationContract(operation);
  const payload = stringifyWireValue(value);
  if (payload === undefined) {
    throw wireError(operation, 'input', 'invalid_input', '$', 'value is not JSON serializable', 'invalid_json');
  }
  const parsed = parseJson(operation, 'input', payload, contract.maxInputBytes);
  validateReference(operation, 'input', contract.inputWire, parsed);
  return payload;
}

export function validateOperationInput(operation: string, value: WireCandidate): void {
  const contract = operationContract(operation);
  if (
    contract.inputWire.owner === 'custom' &&
    contract.inputWire.name === 'ProjectResourceStageInput'
  ) {
    validateProjectResourceStageInput(operation, value, contract.maxInputBytes);
    return;
  }
  serializeOperationInput(operation, value);
}

export function parseOperationOutput<T extends WireCandidate>(operation: string, payload: string): T {
  const contract = operationContract(operation);
  const parsed = parseJson(operation, 'output', payload, contract.maxOutputBytes);
  validateReference(operation, 'output', contract.outputWire, parsed);
  return parsed as T;
}

/** Decode one transport-only generated DTO used to adapt the native wire
 * receipt into a different public facade output within the same operation. */
export function parseGeneratedOperationOutput<T extends WireCandidate>(
  operation: string,
  generatedType: GeneratedWireTypeName,
  payload: string,
): T {
  const contract = operationContract(operation);
  const parsed = parseJson(operation, 'output', payload, contract.maxOutputBytes);
  const result = validateGeneratedWireValue(generatedType, parsed);
  if (!result.valid) {
    throw wireError(
      operation,
      'output',
      'internal',
      result.issue.path,
      result.issue.message,
      result.issue.code,
    );
  }
  return parsed as T;
}

export function validateOperationOutput<T extends WireCandidate>(operation: string, value: T): T {
  const contract = operationContract(operation);
  const payload = stringifyWireValue(value);
  if (payload === undefined) {
    throw wireError(operation, 'output', 'internal', '$', 'value is not JSON serializable', 'invalid_json');
  }
  const parsed = parseJson(operation, 'output', payload, contract.maxOutputBytes);
  validateReference(operation, 'output', contract.outputWire, parsed);
  return value;
}
