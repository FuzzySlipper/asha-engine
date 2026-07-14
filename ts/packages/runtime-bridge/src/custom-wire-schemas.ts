import type { GeneratedWireTypeName } from '@asha/contracts';

export type CustomWireSchema =
  | { readonly kind: 'array'; readonly item: CustomWireSchema }
  | { readonly kind: 'boolean' }
  | { readonly kind: 'custom'; readonly name: string }
  | { readonly kind: 'enum'; readonly values: readonly string[] }
  | { readonly kind: 'generated'; readonly name: GeneratedWireTypeName }
  | { readonly kind: 'nullable'; readonly value: CustomWireSchema }
  | {
      readonly kind: 'number';
      readonly integer?: boolean;
      readonly minimum?: number;
      readonly maximum?: number;
    }
  | CustomWireObjectSchema
  | { readonly kind: 'string' }
  | {
      readonly kind: 'taggedUnion';
      readonly tag: string;
      readonly variants: Readonly<Record<string, CustomWireObjectSchema>>;
    }
  | { readonly kind: 'tuple'; readonly items: readonly CustomWireSchema[] };

export interface CustomWireObjectSchema {
  readonly kind: 'object';
  readonly fields: Readonly<Record<string, CustomWireSchema>>;
  readonly optional?: readonly string[];
}

const array = (item: CustomWireSchema): CustomWireSchema => ({ kind: 'array', item });
const custom = (name: string): CustomWireSchema => ({ kind: 'custom', name });
const enumeration = (...values: readonly string[]): CustomWireSchema => ({ kind: 'enum', values });
const generated = (name: GeneratedWireTypeName): CustomWireSchema => ({ kind: 'generated', name });
const nullable = (value: CustomWireSchema): CustomWireSchema => ({ kind: 'nullable', value });
const object = (
  fields: Readonly<Record<string, CustomWireSchema>>,
  optional?: readonly string[],
): CustomWireObjectSchema =>
  optional === undefined
    ? { kind: 'object', fields }
    : { kind: 'object', fields, optional };
const taggedUnion = (
  tag: string,
  variants: Readonly<Record<string, CustomWireObjectSchema>>,
): CustomWireSchema => ({ kind: 'taggedUnion', tag, variants });
const tuple = (...items: readonly CustomWireSchema[]): CustomWireSchema => ({ kind: 'tuple', items });

const BOOLEAN: CustomWireSchema = { kind: 'boolean' };
const NUMBER: CustomWireSchema = { kind: 'number' };
const INTEGER: CustomWireSchema = { kind: 'number', integer: true };
const NON_NEGATIVE_INTEGER: CustomWireSchema = { kind: 'number', integer: true, minimum: 0 };
const BYTE: CustomWireSchema = { kind: 'number', integer: true, minimum: 0, maximum: 255 };
const STRING: CustomWireSchema = { kind: 'string' };
const STRING_ARRAY = array(STRING);
const VEC3 = tuple(NUMBER, NUMBER, NUMBER);
const QUATERNION = tuple(NUMBER, NUMBER, NUMBER, NUMBER);

const COORDINATE = object({ x: INTEGER, y: INTEGER, z: INTEGER });
const FPS_ROLE = enumeration('player', 'enemy', 'neutral');
const FPS_AUTHORITY_TRANSPORT = enumeration('native_rust', 'reference_bridge');
const FPS_BACKEND_WIRE = enumeration('engine_bridge_rust', 'native_rust', 'reference_bridge');
const FPS_HEALTH = object({ current: NON_NEGATIVE_INTEGER, max: NON_NEGATIVE_INTEGER });
const FPS_TRANSFORM = object({ translation: VEC3, rotation: QUATERNION, scale: VEC3 });
const FPS_BOUNDS = object({ min: VEC3, max: VEC3 });
const FPS_WEAPON = object({
  weaponId: STRING,
  damage: NON_NEGATIVE_INTEGER,
  rangeUnits: NON_NEGATIVE_INTEGER,
  ammo: NON_NEGATIVE_INTEGER,
  cooldownTicksAfterFire: NON_NEGATIVE_INTEGER,
});
const FPS_POLICY_BINDING = object({
  bindingId: STRING,
  policyId: STRING,
  viewKind: STRING,
  viewVersion: STRING,
  allowedIntents: STRING_ARRAY,
  runtimeMoment: STRING,
});
const FPS_STORED_ENTITY = object({
  entity: NON_NEGATIVE_INTEGER,
  stableId: STRING,
  displayName: STRING,
  sourcePath: STRING,
  tags: STRING_ARRAY,
  role: FPS_ROLE,
  transform: nullable(FPS_TRANSFORM),
  bounds: nullable(FPS_BOUNDS),
  renderVisible: nullable(BOOLEAN),
  staticCollider: nullable(BOOLEAN),
  health: nullable(FPS_HEALTH),
  weapon: nullable(FPS_WEAPON),
  policyBinding: nullable(FPS_POLICY_BINDING),
});
const FPS_LIFECYCLE = taggedUnion('state', {
  active: object({}),
  enemy_defeated: object({ entity: NON_NEGATIVE_INTEGER, tick: NON_NEGATIVE_INTEGER }),
});
const FPS_READ_SET = object({ viewKind: STRING, owner: STRING, readSet: STRING_ARRAY });
const FPS_REPLAY = object({
  replayUnit: STRING,
  entityHash: STRING,
  healthHash: STRING,
  recordHash: STRING,
});
const FPS_HEALTH_READOUT = object({
  entity: NON_NEGATIVE_INTEGER,
  current: NON_NEGATIVE_INTEGER,
  max: NON_NEGATIVE_INTEGER,
});
const FPS_POLICY_BINDING_READOUT = object({
  entity: NON_NEGATIVE_INTEGER,
  bindingId: STRING,
  policyId: STRING,
  viewKind: STRING,
  viewVersion: STRING,
  allowedIntents: STRING_ARRAY,
  runtimeMoment: STRING,
});
const GAMEPLAY_VIEW_SCOPE = taggedUnion('kind', {
  session: object({}),
  entity: object({ entity: NON_NEGATIVE_INTEGER }),
  prefabInstance: object({ instance: NON_NEGATIVE_INTEGER }),
});
const GAMEPLAY_CONTRACT_REF = generated('gameExtension.GameplayContractRef');
const COMPOSED_GAMEPLAY_READOUT = object({
  gameplayRegistryDigest: STRING,
  bindingRegistryHash: STRING,
  activationHash: STRING,
  moduleStateHash: STRING,
  authorityStateHash: STRING,
  triggerRevision: NON_NEGATIVE_INTEGER,
  triggerSnapshotHash: STRING,
  activeOverlapCount: NON_NEGATIVE_INTEGER,
  reactionFrameCount: NON_NEGATIVE_INTEGER,
  lastReactionFrameHash: nullable(STRING),
  decisionReceiptCount: NON_NEGATIVE_INTEGER,
  pendingDecisionCount: NON_NEGATIVE_INTEGER,
  lastDecisionReceiptHash: nullable(STRING),
  schedulerStateHash: STRING,
  schedulerPendingActionCount: NON_NEGATIVE_INTEGER,
  schedulerOutstandingDispatchCount: NON_NEGATIVE_INTEGER,
  schedulerOutstandingEventDeliveryCount: NON_NEGATIVE_INTEGER,
  schedulerFactCount: NON_NEGATIVE_INTEGER,
  schedulerTruncated: BOOLEAN,
  runtimeHostHash: STRING,
});
const FPS_ENCOUNTER_LIFECYCLE = object({
  outcomeKind: enumeration('in_progress', 'won', 'lost'),
  terminal: BOOLEAN,
  enemyDead: BOOLEAN,
  playerDead: BOOLEAN,
  lifecycleHash: STRING,
});
const FPS_ENCOUNTER_STATE = object({
  presetId: STRING,
  status: enumeration('pending', 'active', 'cleared', 'failed'),
  spawnedEnemyIds: STRING_ARRAY,
  defeatedEnemyIds: STRING_ARRAY,
  revision: NON_NEGATIVE_INTEGER,
  lastTransition: enumeration('initialized', 'activated', 'cleared', 'failed', 'reset'),
});

export const CUSTOM_WIRE_SCHEMAS: Readonly<Record<string, CustomWireSchema>> = {
  EngineConfig: object({ seed: NON_NEGATIVE_INTEGER }),
  StepInputEnvelope: object({ tick: NON_NEGATIVE_INTEGER }),
  StepResult: object({ tick: NON_NEGATIVE_INTEGER, diffCount: NON_NEGATIVE_INTEGER }),
  ProjectBundleLoadRequest: object({
    bundleSchemaVersion: NON_NEGATIVE_INTEGER,
    protocolVersion: NON_NEGATIVE_INTEGER,
    sceneId: NON_NEGATIVE_INTEGER,
  }),
  CompositionStatus: object({
    loadedProjectBundle: nullable(NON_NEGATIVE_INTEGER),
    fatalCount: NON_NEGATIVE_INTEGER,
    totalCount: NON_NEGATIVE_INTEGER,
    blocksLoad: BOOLEAN,
  }),
  ProjectBundleSaveSummary: object({
    artifactsWritten: NON_NEGATIVE_INTEGER,
    compactedEdits: NON_NEGATIVE_INTEGER,
    retainedEdits: NON_NEGATIVE_INTEGER,
  }),
  RuntimeBufferView: object({ handle: NON_NEGATIVE_INTEGER, bytes: array(BYTE) }),
  ReplayFixture: object({ name: STRING, steps: NON_NEGATIVE_INTEGER }),
  ReplayStepReport: object({ step: NON_NEGATIVE_INTEGER, hash: STRING, diverged: BOOLEAN }),
  VoxelMeshEvidenceRequest: object({ grid: NON_NEGATIVE_INTEGER, chunks: array(COORDINATE) }),
  VoxelMeshEvidenceSnapshot: object({
    grid: NON_NEGATIVE_INTEGER,
    fixtureId: STRING,
    voxelStateHash: STRING,
    meshingStrategy: STRING,
    chunks: array(object({
      coord: COORDINATE,
      resident: BOOLEAN,
      visible: BOOLEAN,
      contentHash: nullable(STRING),
      meshHash: nullable(STRING),
      stats: nullable(object({
        vertices: NON_NEGATIVE_INTEGER,
        indices: NON_NEGATIVE_INTEGER,
        quads: NON_NEGATIVE_INTEGER,
        facesEmitted: NON_NEGATIVE_INTEGER,
        facesCulled: NON_NEGATIVE_INTEGER,
      })),
      bounds: nullable(object({ min: VEC3, max: VEC3 })),
      materialSlots: array(NON_NEGATIVE_INTEGER),
    })),
    diagnostics: STRING_ARRAY,
  }),
  EnemyDirectNavMovementRequest: object({
    entity: NON_NEGATIVE_INTEGER,
    seedPosition: VEC3,
    target: VEC3,
    maxStepUnits: NUMBER,
  }),
  EnemyDirectNavMovementResult: object({
    entity: NON_NEGATIVE_INTEGER,
    authoritySource: enumeration('seeded_from_request', 'rust_entity_store'),
    authorityTransport: FPS_AUTHORITY_TRANSPORT,
    from: VEC3,
    target: VEC3,
    nextWaypoint: VEC3,
    distanceUnits: NUMBER,
    reached: BOOLEAN,
    pathHash: STRING,
    transformHash: STRING,
    projectionChanged: BOOLEAN,
  }),
  FpsRuntimeSessionLoadRequest: object({
    projectBundle: STRING,
    definitions: array(FPS_STORED_ENTITY),
    gameRuleModules: array(generated('gameExtension.GameRuleModuleManifest')),
  }, ['gameRuleModules']),
  FpsRuntimeSessionRestartRequest: object({ expectedEpoch: NON_NEGATIVE_INTEGER }),
  FpsPrimaryFireRequest: object({
    tick: NON_NEGATIVE_INTEGER,
    origin: VEC3,
    direction: VEC3,
    shooterRole: FPS_ROLE,
    targetRole: FPS_ROLE,
  }, ['shooterRole', 'targetRole']),
  FpsRuntimeSessionSnapshot: object({
    backend: FPS_BACKEND_WIRE,
    authoritySurface: STRING,
    projectBundle: STRING,
    sessionEpoch: NON_NEGATIVE_INTEGER,
    lifecycleStatus: FPS_LIFECYCLE,
    playerEntity: NON_NEGATIVE_INTEGER,
    enemyEntity: NON_NEGATIVE_INTEGER,
    health: array(FPS_HEALTH_READOUT),
    policyBindings: array(FPS_POLICY_BINDING_READOUT),
    replayRecords: array(FPS_REPLAY),
    readSets: array(FPS_READ_SET),
    entityHash: STRING,
    healthHash: STRING,
    replayHash: STRING,
  }),
  FpsPrimaryFireResult: object({
    backend: FPS_BACKEND_WIRE,
    authoritySurface: STRING,
    mutationOwner: STRING,
    workspaceTrace: STRING_ARRAY,
    shooter: NON_NEGATIVE_INTEGER,
    target: nullable(NON_NEGATIVE_INTEGER),
    targetHealthBefore: nullable(FPS_HEALTH),
    targetHealthAfter: nullable(FPS_HEALTH),
    lifecycleStatus: FPS_LIFECYCLE,
    targetRenderVisible: nullable(BOOLEAN),
    entityHash: STRING,
    healthHash: STRING,
    replayHash: STRING,
  }),
  ComposedRuntimeSessionReadout: object({
    schemaVersion: NON_NEGATIVE_INTEGER,
    entityAuthorityHash: STRING,
    gameplay: COMPOSED_GAMEPLAY_READOUT,
    fpsSessionEpoch: NON_NEGATIVE_INTEGER,
    fpsReplayHash: nullable(STRING),
    runtimeSessionHash: STRING,
  }),
  GameplayModuleViewRequest: object({
    view: GAMEPLAY_CONTRACT_REF,
    scope: GAMEPLAY_VIEW_SCOPE,
    expectedRuntimeSessionHash: STRING,
  }),
  GameplayModuleViewSnapshot: object({
    view: GAMEPLAY_CONTRACT_REF,
    providerId: STRING,
    scope: GAMEPLAY_VIEW_SCOPE,
    revision: NON_NEGATIVE_INTEGER,
    canonicalPayload: array(BYTE),
    viewHash: STRING,
    runtimeSessionHash: STRING,
  }),
  GameplayPrefabPartInteractionRequest: object({
    actor: NON_NEGATIVE_INTEGER,
    instance: NON_NEGATIVE_INTEGER,
    role: STRING,
    expectedTarget: NON_NEGATIVE_INTEGER,
    tick: NON_NEGATIVE_INTEGER,
    expectedRuntimeSessionHash: STRING,
  }),
  GameplayPrefabPartInteractionReceipt: object({
    actor: NON_NEGATIVE_INTEGER,
    instance: NON_NEGATIVE_INTEGER,
    role: STRING,
    target: NON_NEGATIVE_INTEGER,
    eventHash: STRING,
    reactionFrameHash: STRING,
    runtimeSessionHash: STRING,
  }),
  GameExtensionWeaponEffectInvocationRequest: object({
    hook: generated('gameExtension.WeaponEffectHookRequest'),
    primaryFire: custom('FpsPrimaryFireRequest'),
  }),
  GameExtensionWeaponEffectInvocationResult: object({
    hookReceipt: generated('gameExtension.GameExtensionHookReceipt'),
    replayEvidence: generated('gameExtension.GameExtensionReplayEvidence'),
    primaryFire: nullable(custom('FpsPrimaryFireResult')),
  }),
  GameRuleCatalogValidationReceipt: object({
    accepted: BOOLEAN,
    catalogHash: STRING,
    diagnostics: array(generated('gameRules.GameRuleDiagnostic')),
    trace: array(generated('gameRules.GameRuleTraceEntry')),
    evidence: array(generated('gameRules.GameRuleEvidenceRef')),
  }),
  GameRuleEffectIntentRequest: object({
    catalog: generated('gameRules.GameRuleCatalog'),
    request: generated('gameRules.GameRuleResolutionRequest'),
  }),
  GameRuleRuntimeReadout: object({
    backend: FPS_BACKEND_WIRE,
    authoritySurface: STRING,
    activeModifiers: array(generated('gameRules.GameRuleModifierState')),
    recentTrace: array(generated('gameRules.GameRuleTraceEntry')),
    recentReplayHashes: STRING_ARRAY,
    latestReplayHash: nullable(STRING),
  }),
  FpsEncounterLifecycleInput: FPS_ENCOUNTER_LIFECYCLE,
  FpsEncounterTransitionRequest: object({
    presetId: STRING,
    action: enumeration('activate', 'sync_lifecycle', 'reset'),
    lifecycle: FPS_ENCOUNTER_LIFECYCLE,
  }),
  FpsEncounterDirectorSnapshot: object({
    backend: FPS_BACKEND_WIRE,
    authoritySurface: STRING,
    mutationOwner: STRING,
    workspaceTrace: STRING_ARRAY,
    state: FPS_ENCOUNTER_STATE,
    lifecycle: FPS_ENCOUNTER_LIFECYCLE,
    readSets: array(FPS_READ_SET),
    encounterHash: STRING,
    replayHash: STRING,
  }),
  FpsEncounterTransitionResult: object({
    backend: FPS_BACKEND_WIRE,
    authoritySurface: STRING,
    mutationOwner: STRING,
    workspaceTrace: STRING_ARRAY,
    accepted: BOOLEAN,
    rejectionReason: nullable(enumeration(
      'encounter_not_pending',
      'invalid_encounter_transition',
      'unknown_encounter_preset',
    )),
    eventKind: nullable(enumeration(
      'runtime_encounter.activated.v0',
      'runtime_encounter.lifecycle_synced.v0',
      'runtime_encounter.reset.v0',
    )),
    state: FPS_ENCOUNTER_STATE,
    lifecycle: FPS_ENCOUNTER_LIFECYCLE,
    encounterHash: STRING,
    replayHash: STRING,
  }),
};
