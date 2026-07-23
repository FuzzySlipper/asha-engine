import {
  AUTHORED_BEHAVIOR_MAX_DELAY_TICKS,
  AUTHORED_BEHAVIOR_PACKAGE_SCHEMA_VERSION,
  AUTHORED_BEHAVIOR_VOCABULARY_HASH,
  AUTHORED_BEHAVIOR_VOCABULARY_VERSION,
  AUTHORED_PREDICATE_STATE_IS,
  AUTHORED_SIGNAL_PREFAB_PART_INTERACTED,
  AUTHORED_VERB_SET_CAPABILITY_ACTIVE,
  AUTHORED_VERB_SET_RELATIVE_TRANSLATION,
  AUTHORED_VERB_TRANSITION_STATE,
  type AuthoredBehaviorArgument,
  type AuthoredBehaviorCondition,
  type AuthoredBehaviorDefinition,
  type AuthoredBehaviorOperation,
  type AuthoredBehaviorPackage,
  type AuthoredBehaviorSemanticRef,
  type AuthoredBehaviorSignal,
  type AuthoredBehaviorState,
  type AuthoredBehaviorStateMachine,
  type AuthoredBehaviorStep,
  type AuthoredBehaviorTransition,
  type AuthoredBehaviorValue,
  type ProjectContentDocument,
} from '@asha/contracts';

export interface AshaAuthoredBehaviorPackageDraft {
  readonly packageId: string;
  readonly stateMachines: readonly AuthoredBehaviorStateMachine[];
  readonly behaviors: readonly AuthoredBehaviorDefinition[];
}

export interface AshaAuthoredBehaviorSource {
  readonly sourceModule: string;
  readonly sourcePath: string;
}

type SceneEntityValue = Extract<AuthoredBehaviorValue, { readonly kind: 'sceneEntity' }>;
type PrefabPartValue = Extract<AuthoredBehaviorValue, { readonly kind: 'prefabPart' }>;

/**
 * Compact immutable gameplay declarations. Every helper lowers to a generated
 * Rust-owned semantic identity and data-only typed arguments.
 */
export const authoredBehavior = {
  state(stateId: string): AuthoredBehaviorState {
    return freeze({ stateId });
  },

  transition(
    transitionId: string,
    fromStateId: string,
    toStateId: string,
  ): AuthoredBehaviorTransition {
    return freeze({ transitionId, fromStateId, toStateId });
  },

  stateMachine(
    machineId: string,
    targetSceneInstanceId: string,
    initialStateId: string,
    states: readonly AuthoredBehaviorState[],
    transitions: readonly AuthoredBehaviorTransition[],
  ): AuthoredBehaviorStateMachine {
    return freeze({
      machineId,
      targetSceneInstanceId,
      initialStateId,
      states: freezeArray(states),
      transitions: freezeArray(transitions),
    });
  },

  sceneEntity(sceneInstanceId: string): SceneEntityValue {
    return freeze({ kind: 'sceneEntity', sceneInstanceId });
  },

  prefabPart(sceneInstanceId: string, role: string): PrefabPartValue {
    return freeze({ kind: 'prefabPart', sceneInstanceId, role });
  },

  prefabPartInteracted(part: PrefabPartValue): AuthoredBehaviorSignal {
    return freeze({
      signal: semantic(AUTHORED_SIGNAL_PREFAB_PART_INTERACTED),
      arguments: freezeArray([argument('part', part)]),
    });
  },

  whenState(machineId: string, stateId: string): AuthoredBehaviorCondition {
    return freeze({
      predicate: semantic(AUTHORED_PREDICATE_STATE_IS),
      arguments: freezeArray([
        argument('state', freeze({ kind: 'state', machineId, stateId })),
      ]),
    });
  },

  transitionState(machineId: string, transitionId: string): AuthoredBehaviorOperation {
    return freeze({
      verb: semantic(AUTHORED_VERB_TRANSITION_STATE),
      arguments: freezeArray([
        argument('machine', freeze({ kind: 'stateMachine', machineId })),
        argument('transition', freeze({ kind: 'text', value: transitionId })),
      ]),
    });
  },

  setRelativeTranslation(
    entity: SceneEntityValue,
    value: readonly [number, number, number],
  ): AuthoredBehaviorOperation {
    return freeze({
      verb: semantic(AUTHORED_VERB_SET_RELATIVE_TRANSLATION),
      arguments: freezeArray([
        argument('entity', entity),
        argument('value', freeze({ kind: 'vector3', value: freezeTuple(value) })),
      ]),
    });
  },

  setCapabilityActive(
    entity: SceneEntityValue,
    capability: 'collision',
    active: boolean,
  ): AuthoredBehaviorOperation {
    return freeze({
      verb: semantic(AUTHORED_VERB_SET_CAPABILITY_ACTIVE),
      arguments: freezeArray([
        argument('entity', entity),
        argument('capability', freeze({ kind: 'text', value: capability })),
        argument('active', freeze({ kind: 'boolean', value: active })),
      ]),
    });
  },

  step(
    stepId: string,
    operations: readonly AuthoredBehaviorOperation[],
  ): AuthoredBehaviorStep {
    return freeze({
      stepId,
      afterStepIds: freezeArray([]),
      delayTicks: 0,
      operations: freezeArray(operations),
    });
  },

  afterTicks(
    stepId: string,
    afterStepId: string,
    delayTicks: number,
    operations: readonly AuthoredBehaviorOperation[],
  ): AuthoredBehaviorStep {
    if (!Number.isSafeInteger(delayTicks) || delayTicks <= 0 || delayTicks > AUTHORED_BEHAVIOR_MAX_DELAY_TICKS) {
      throw new Error(`delayTicks must be an integer from 1 through ${AUTHORED_BEHAVIOR_MAX_DELAY_TICKS}`);
    }
    return freeze({
      stepId,
      afterStepIds: freezeArray([afterStepId]),
      delayTicks,
      operations: freezeArray(operations),
    });
  },

  behavior(
    behaviorId: string,
    signal: AuthoredBehaviorSignal,
    conditions: readonly AuthoredBehaviorCondition[],
    steps: readonly AuthoredBehaviorStep[],
  ): AuthoredBehaviorDefinition {
    return freeze({
      behaviorId,
      signal,
      conditions: freezeArray(conditions),
      steps: freezeArray(steps),
    });
  },
} as const;

/** Normalize one readable declaration into deterministic ProjectContent. */
export function compileAshaAuthoredBehaviorPackage(
  draft: AshaAuthoredBehaviorPackageDraft,
  source: AshaAuthoredBehaviorSource,
): AuthoredBehaviorPackage {
  assertDataOnly(draft, 'draft');
  assertDataOnly(source, 'source');
  const normalizedDraft = {
    source: freeze({
      sourceModule: source.sourceModule,
      sourcePath: source.sourcePath,
    }),
    packageId: draft.packageId,
    stateMachines: [...draft.stateMachines]
      .sort((left, right) => compareText(left.machineId, right.machineId))
      .map(normalizeMachine),
    behaviors: [...draft.behaviors]
      .sort((left, right) => compareText(left.behaviorId, right.behaviorId))
      .map(normalizeBehavior),
  } as const;
  const sourceHash = fnv1a64(JSON.stringify(normalizedDraft));
  return freeze({
    schemaVersion: AUTHORED_BEHAVIOR_PACKAGE_SCHEMA_VERSION,
    packageId: normalizedDraft.packageId,
    provenance: freeze({
      sdkId: '@asha/game-workspace',
      sdkVersion: AUTHORED_BEHAVIOR_VOCABULARY_VERSION,
      vocabularyHash: AUTHORED_BEHAVIOR_VOCABULARY_HASH,
      sourceModule: normalizedDraft.source.sourceModule,
      sourcePath: normalizedDraft.source.sourcePath,
      sourceHash,
    }),
    stateMachines: freezeArray(normalizedDraft.stateMachines),
    behaviors: freezeArray(normalizedDraft.behaviors),
  });
}

export function createAshaAuthoredBehaviorDocument(
  documentId: string,
  draft: AshaAuthoredBehaviorPackageDraft,
  source: AshaAuthoredBehaviorSource,
): ProjectContentDocument {
  return freeze({
    kind: 'behaviorPackage',
    documentId,
    package: compileAshaAuthoredBehaviorPackage(draft, source),
  });
}

function normalizeMachine(machine: AuthoredBehaviorStateMachine): AuthoredBehaviorStateMachine {
  return freeze({
    machineId: machine.machineId,
    targetSceneInstanceId: machine.targetSceneInstanceId,
    initialStateId: machine.initialStateId,
    states: freezeArray(
      [...machine.states]
        .sort((left, right) => compareText(left.stateId, right.stateId))
        .map((state) => freeze({ stateId: state.stateId })),
    ),
    transitions: freezeArray(
      [...machine.transitions]
        .sort((left, right) => compareText(left.transitionId, right.transitionId))
        .map((transition) => freeze({
          transitionId: transition.transitionId,
          fromStateId: transition.fromStateId,
          toStateId: transition.toStateId,
        })),
    ),
  });
}

function normalizeBehavior(behavior: AuthoredBehaviorDefinition): AuthoredBehaviorDefinition {
  return freeze({
    behaviorId: behavior.behaviorId,
    signal: normalizeSignal(behavior.signal),
    conditions: freezeArray(
      [...behavior.conditions]
        .sort((left, right) => compareText(JSON.stringify(left), JSON.stringify(right)))
        .map((condition) => freeze({
          predicate: normalizeSemantic(condition.predicate),
          arguments: normalizeArguments(condition.arguments),
        })),
    ),
    steps: freezeArray(
      [...behavior.steps]
        .sort((left, right) => compareText(left.stepId, right.stepId))
        .map((step) => freeze({
          stepId: step.stepId,
          afterStepIds: freezeArray([...step.afterStepIds].sort(compareText)),
          delayTicks: step.delayTicks,
          operations: freezeArray(step.operations.map(normalizeOperation)),
        })),
    ),
  });
}

function normalizeSignal(signal: AuthoredBehaviorSignal): AuthoredBehaviorSignal {
  return freeze({
    signal: normalizeSemantic(signal.signal),
    arguments: normalizeArguments(signal.arguments),
  });
}

function normalizeOperation(operation: AuthoredBehaviorOperation): AuthoredBehaviorOperation {
  return freeze({
    verb: normalizeSemantic(operation.verb),
    arguments: normalizeArguments(operation.arguments),
  });
}

function normalizeArguments(arguments_: readonly AuthoredBehaviorArgument[]): readonly AuthoredBehaviorArgument[] {
  return freezeArray(
    [...arguments_]
      .sort((left, right) => compareText(left.name, right.name))
      .map((entry) => argument(entry.name, normalizeValue(entry.value))),
  );
}

function normalizeValue(value: AuthoredBehaviorValue): AuthoredBehaviorValue {
  if (value.kind === 'vector3') {
    return freeze({ ...value, value: freezeTuple(value.value) });
  }
  return freeze({ ...value });
}

function semantic(semanticId: string): AuthoredBehaviorSemanticRef {
  return freeze({ semanticId, version: AUTHORED_BEHAVIOR_VOCABULARY_VERSION });
}

function normalizeSemantic(value: AuthoredBehaviorSemanticRef): AuthoredBehaviorSemanticRef {
  return freeze({ semanticId: value.semanticId, version: value.version });
}

function argument(name: string, value: AuthoredBehaviorValue): AuthoredBehaviorArgument {
  return freeze({ name, value });
}

function freeze<T extends object>(value: T): Readonly<T> {
  return Object.freeze(value);
}

function freezeArray<T>(values: readonly T[]): readonly T[] {
  return Object.freeze([...values]);
}

function freezeTuple(values: readonly [number, number, number]): readonly [number, number, number] {
  return Object.freeze([values[0], values[1], values[2]]) as readonly [number, number, number];
}

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function assertDataOnly(value: unknown, path: string, seen = new Set<object>()): void {
  if (value === null || typeof value === 'string' || typeof value === 'boolean') {
    return;
  }
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) {
      throw new Error(`${path} must contain only finite data values`);
    }
    return;
  }
  if (typeof value !== 'object') {
    throw new Error(`${path} must contain data only; executable or ambient values are forbidden`);
  }
  if (seen.has(value)) {
    throw new Error(`${path} must not contain cyclic data`);
  }
  seen.add(value);
  if (Array.isArray(value)) {
    value.forEach((entry, index) => assertDataOnly(entry, `${path}[${index}]`, seen));
    seen.delete(value);
    return;
  }
  const prototype = Object.getPrototypeOf(value) as object | null;
  if (prototype !== Object.prototype && prototype !== null) {
    throw new Error(`${path} must contain plain project data, not browser or class instances`);
  }
  for (const [key, entry] of Object.entries(value)) {
    assertDataOnly(entry, `${path}.${key}`, seen);
  }
  seen.delete(value);
}

function fnv1a64(value: string): string {
  let hash = 14_695_981_039_346_656_037n;
  for (const byte of new TextEncoder().encode(value)) {
    hash ^= BigInt(byte);
    hash = BigInt.asUintN(64, hash * 1_099_511_628_211n);
  }
  return `fnv1a64:${hash.toString(16).padStart(16, '0')}`;
}
