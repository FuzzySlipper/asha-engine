import {
  validateGeneratedWireValue,
  type GeneratedWireValidationResult,
  type GeneratedWireValue,
} from '@asha/contracts';
import { RuntimeBridgeError, type RuntimeBridgeErrorKind } from './bridge.js';
import {
  MANIFEST_OPERATIONS,
  type BridgeOperation,
  type BridgeWireTypeRef,
} from './generated/operations.js';

type WireDirection = 'input' | 'output';
type WireCandidate = object | boolean | number | string | null;
type CustomFieldKind =
  | 'array'
  | 'boolean'
  | 'nullable_boolean'
  | 'nullable_number'
  | 'nullable_object'
  | 'nullable_string'
  | 'number'
  | 'object'
  | 'string';

interface CustomWireShape {
  readonly fields: Readonly<Record<string, CustomFieldKind>>;
  readonly optional?: readonly string[];
}

const CUSTOM_WIRE_SHAPES: Readonly<Record<string, CustomWireShape>> = {
  EngineConfig: { fields: { seed: 'number' } },
  StepInputEnvelope: { fields: { tick: 'number' } },
  StepResult: { fields: { tick: 'number', diffCount: 'number' } },
  ProjectBundleLoadRequest: {
    fields: { bundleSchemaVersion: 'number', protocolVersion: 'number', sceneId: 'number' },
  },
  CompositionStatus: {
    fields: {
      loadedProjectBundle: 'nullable_number',
      fatalCount: 'number',
      totalCount: 'number',
      blocksLoad: 'boolean',
    },
  },
  ProjectBundleSaveSummary: {
    fields: { artifactsWritten: 'number', compactedEdits: 'number', retainedEdits: 'number' },
  },
  RuntimeBufferView: { fields: { handle: 'number', bytes: 'array' } },
  ReplayFixture: { fields: { name: 'string', steps: 'number' } },
  ReplayStepReport: { fields: { step: 'number', hash: 'string', diverged: 'boolean' } },
  VoxelMeshEvidenceRequest: { fields: { grid: 'number', chunks: 'array' } },
  VoxelMeshEvidenceSnapshot: {
    fields: {
      grid: 'number',
      fixtureId: 'string',
      voxelStateHash: 'string',
      meshingStrategy: 'string',
      chunks: 'array',
      diagnostics: 'array',
    },
  },
  EnemyDirectNavMovementRequest: {
    fields: { entity: 'number', seedPosition: 'array', target: 'array', maxStepUnits: 'number' },
  },
  EnemyDirectNavMovementResult: {
    fields: {
      entity: 'number',
      authoritySource: 'string',
      authorityTransport: 'string',
      from: 'array',
      target: 'array',
      nextWaypoint: 'array',
      distanceUnits: 'number',
      reached: 'boolean',
      pathHash: 'string',
      transformHash: 'string',
      projectionChanged: 'boolean',
    },
  },
  FpsRuntimeSessionLoadRequest: {
    fields: { projectBundle: 'string', definitions: 'array', gameRuleModules: 'array' },
    optional: ['gameRuleModules'],
  },
  FpsRuntimeSessionRestartRequest: { fields: { expectedEpoch: 'number' } },
  FpsPrimaryFireRequest: {
    fields: {
      tick: 'number',
      origin: 'array',
      direction: 'array',
      shooterRole: 'string',
      targetRole: 'string',
    },
    optional: ['shooterRole', 'targetRole'],
  },
  FpsRuntimeSessionSnapshot: {
    fields: {
      backend: 'string',
      authoritySurface: 'string',
      projectBundle: 'string',
      sessionEpoch: 'number',
      lifecycleStatus: 'object',
      playerEntity: 'number',
      enemyEntity: 'number',
      health: 'array',
      policyBindings: 'array',
      replayRecords: 'array',
      readSets: 'array',
      entityHash: 'string',
      healthHash: 'string',
      replayHash: 'string',
    },
  },
  FpsPrimaryFireResult: {
    fields: {
      backend: 'string',
      authoritySurface: 'string',
      mutationOwner: 'string',
      workspaceTrace: 'array',
      shooter: 'number',
      target: 'nullable_number',
      targetHealthBefore: 'nullable_object',
      targetHealthAfter: 'nullable_object',
      lifecycleStatus: 'object',
      targetRenderVisible: 'nullable_boolean',
      entityHash: 'string',
      healthHash: 'string',
      replayHash: 'string',
    },
  },
  GameExtensionWeaponEffectInvocationRequest: {
    fields: { hook: 'object', primaryFire: 'object' },
  },
  GameExtensionWeaponEffectInvocationResult: {
    fields: { hookReceipt: 'object', replayEvidence: 'object', primaryFire: 'object' },
  },
  GameRuleCatalogValidationReceipt: {
    fields: {
      accepted: 'boolean',
      catalogHash: 'string',
      diagnostics: 'array',
      trace: 'array',
      evidence: 'array',
    },
  },
  GameRuleEffectIntentRequest: { fields: { catalog: 'object', request: 'object' } },
  GameRuleRuntimeReadout: {
    fields: {
      backend: 'string',
      authoritySurface: 'string',
      activeModifiers: 'array',
      recentTrace: 'array',
      recentReplayHashes: 'array',
      latestReplayHash: 'nullable_string',
    },
  },
  FpsEncounterLifecycleInput: {
    fields: {
      outcomeKind: 'string',
      terminal: 'boolean',
      enemyDead: 'boolean',
      playerDead: 'boolean',
      lifecycleHash: 'string',
    },
  },
  FpsEncounterTransitionRequest: {
    fields: { presetId: 'string', action: 'string', lifecycle: 'object' },
  },
  FpsEncounterDirectorSnapshot: {
    fields: {
      backend: 'string',
      authoritySurface: 'string',
      mutationOwner: 'string',
      workspaceTrace: 'array',
      state: 'object',
      lifecycle: 'object',
      readSets: 'array',
      encounterHash: 'string',
      replayHash: 'string',
    },
  },
  FpsEncounterTransitionResult: {
    fields: {
      backend: 'string',
      authoritySurface: 'string',
      mutationOwner: 'string',
      workspaceTrace: 'array',
      accepted: 'boolean',
      rejectionReason: 'nullable_string',
      eventKind: 'string',
      state: 'object',
      lifecycle: 'object',
      encounterHash: 'string',
      replayHash: 'string',
    },
  },
};

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

function fieldMatches(kind: CustomFieldKind, value: GeneratedWireValue): boolean {
  if (kind === 'nullable_boolean') return value === null || typeof value === 'boolean';
  if (kind === 'nullable_number') return value === null || (typeof value === 'number' && Number.isFinite(value));
  if (kind === 'nullable_object') return value === null || (typeof value === 'object' && !Array.isArray(value));
  if (kind === 'nullable_string') return value === null || typeof value === 'string';
  if (value === null) return false;
  if (kind === 'array') return Array.isArray(value);
  if (kind === 'object') return typeof value === 'object' && !Array.isArray(value);
  if (kind === 'number') return typeof value === 'number' && Number.isFinite(value);
  return typeof value === kind;
}

function validateCustom(
  operation: string,
  direction: WireDirection,
  reference: BridgeWireTypeRef,
  value: GeneratedWireValue,
): void {
  const shape = CUSTOM_WIRE_SHAPES[reference.name];
  if (shape === undefined) {
    throw wireError(
      operation,
      direction,
      'operation_unimplemented',
      '$',
      `custom wire validator '${reference.name}' is not registered`,
      'missing_custom_validator',
    );
  }
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw wireError(operation, direction, direction === 'input' ? 'invalid_input' : 'internal', '$', 'expected object', 'wrong_type');
  }
  const objectValue = value as Readonly<Record<string, GeneratedWireValue>>;
  const optional = new Set(shape.optional ?? []);
  for (const field of Object.keys(objectValue)) {
    if (shape.fields[field] === undefined) {
      throw wireError(operation, direction, direction === 'input' ? 'invalid_input' : 'internal', `$.${field}`, 'unknown field', 'unknown_field');
    }
  }
  for (const [field, kind] of Object.entries(shape.fields)) {
    if (!(field in objectValue)) {
      if (optional.has(field)) continue;
      throw wireError(operation, direction, direction === 'input' ? 'invalid_input' : 'internal', `$.${field}`, 'missing required field', 'missing_field');
    }
    const fieldValue = objectValue[field] ?? null;
    if (!fieldMatches(kind, fieldValue)) {
      throw wireError(operation, direction, direction === 'input' ? 'invalid_input' : 'internal', `$.${field}`, `expected ${kind}`, 'wrong_type');
    }
  }
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
  serializeOperationInput(operation, value);
}

export function parseOperationOutput<T extends WireCandidate>(operation: string, payload: string): T {
  const contract = operationContract(operation);
  const parsed = parseJson(operation, 'output', payload, contract.maxOutputBytes);
  validateReference(operation, 'output', contract.outputWire, parsed);
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
