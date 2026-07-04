export type EncounterPresetId = 'generated-tunnel-small-encounter';

export type EncounterDirectorReadoutKind = 'runtime_session.encounter_director.v0';

export type EncounterTransitionReceiptKind = 'runtime_session.encounter_transition_receipt.v0';

export type EncounterConfigKind = 'encounter_config.generated_tunnel_small.v0';

export type EncounterSourceKind = 'project_bundle.encounter_preset';

export type EncounterSpawnMarkerSourceKind = 'generated_tunnel.spawn_marker';

export type EncounterEntityDefinitionSourceKind = 'project_bundle.entity_definition';

export type EncounterStatus = 'pending' | 'active' | 'cleared' | 'failed';

export type EncounterSpawnInstanceStatus = 'pending' | 'spawned' | 'defeated' | 'blocked';

export type EncounterTransitionAction = 'activate' | 'sync_lifecycle' | 'reset';

export type EncounterTransitionStatus = 'accepted' | 'rejected';

export type EncounterTransitionRejectionReason =
  | 'encounter_not_pending'
  | 'invalid_encounter_transition'
  | 'unknown_encounter_preset';

export type EncounterLifecycleOutcomeKind = 'in_progress' | 'won' | 'lost';

export type EncounterLifecycleScenario =
  | 'active'
  | 'generated_tunnel_enemy_defeated'
  | 'generated_tunnel_player_defeated';

export type EncounterEntityInstanceId = 'encounter.generated_tunnel_small.wave_1.enemy_001';

export interface EncounterEntityDefinitionRef {
  readonly source: EncounterEntityDefinitionSourceKind;
  readonly definitionId: 'entity.enemy.generated_tunnel.basic.v0';
  readonly entityDefinitionId: 'generated-tunnel.enemy.basic';
}

export interface EncounterEnemyDefinitionReadout {
  readonly ref: EncounterEntityDefinitionRef;
  readonly displayName: 'Generated Tunnel Enemy';
  readonly count: 1;
  readonly runtimeEntityId: 20;
  readonly capabilities: readonly ['combat.health', 'enemy_policy', 'nav.agent'];
}

export interface EncounterSpawnMarkerRef {
  readonly source: EncounterSpawnMarkerSourceKind;
  readonly markerId: 'exit_hint';
  readonly world: readonly [number, number, number];
  readonly yawDegrees: 180;
}

export interface EncounterWaveReadout {
  readonly waveId: 'wave.1';
  readonly order: 0;
  readonly enemyCount: 1;
  readonly spawnMarkerIds: readonly ['exit_hint'];
}

export interface EncounterConfigReadout {
  readonly kind: EncounterConfigKind;
  readonly source: EncounterSourceKind;
  readonly presetId: EncounterPresetId;
  readonly seed: 17;
  readonly fixturePath: 'harness/fixtures/encounters/generated-tunnel-small-encounter.snapshot.txt';
  readonly configHash: string;
  readonly spawnOrderHash: string;
  readonly enemyDefinitions: readonly [EncounterEnemyDefinitionReadout];
  readonly spawnMarkerRefs: readonly [EncounterSpawnMarkerRef];
  readonly waves: readonly [EncounterWaveReadout];
}

export interface EncounterDirectorState {
  readonly presetId: EncounterPresetId;
  readonly status: EncounterStatus;
  readonly spawnedEnemyIds: readonly EncounterEntityInstanceId[];
  readonly defeatedEnemyIds: readonly EncounterEntityInstanceId[];
  readonly revision: number;
  readonly lastTransition:
    | 'initialized'
    | 'activated'
    | 'cleared'
    | 'failed'
    | 'reset';
}

export interface EncounterLifecycleInput {
  readonly outcomeKind: EncounterLifecycleOutcomeKind;
  readonly terminal: boolean;
  readonly enemyDead: boolean;
  readonly playerDead: boolean;
  readonly lifecycleHash: string;
}

export interface EncounterSpawnInstanceReadout {
  readonly instanceId: EncounterEntityInstanceId;
  readonly runtimeEntityId: 20;
  readonly waveId: 'wave.1';
  readonly order: 0;
  readonly enemy: EncounterEntityDefinitionRef;
  readonly spawnMarker: EncounterSpawnMarkerRef;
  readonly status: EncounterSpawnInstanceStatus;
}

export interface EncounterDirectorStateReadout {
  readonly status: EncounterStatus;
  readonly revision: number;
  readonly lastTransition: EncounterDirectorState['lastTransition'];
  readonly activeEnemyCount: number;
  readonly pendingEnemyCount: number;
  readonly defeatedEnemyCount: number;
  readonly spawnedEnemyCount: number;
  readonly failedReason?: 'player_defeated';
  readonly clearedReason?: 'all_enemies_defeated';
}

export interface EncounterLifecycleReadout {
  readonly outcomeKind: EncounterLifecycleOutcomeKind;
  readonly terminal: boolean;
  readonly enemyDead: boolean;
  readonly playerDead: boolean;
  readonly lifecycleHash: string;
}

export interface EncounterDirectorReadout {
  readonly kind: EncounterDirectorReadoutKind;
  readonly sequenceId: number;
  readonly tick: number;
  readonly presetId: EncounterPresetId;
  readonly sessionSeed: number;
  readonly config: EncounterConfigReadout;
  readonly state: EncounterDirectorStateReadout;
  readonly spawns: readonly EncounterSpawnInstanceReadout[];
  readonly lifecycle: EncounterLifecycleReadout;
  readonly hashes: {
    readonly encounterHash: string;
    readonly spawnOrderHash: string;
    readonly replayHash: string;
    readonly sessionHash: string;
  };
  readonly nonClaims: readonly [
    'not_wave_design',
    'not_demo_local_spawn_state',
    'not_loot_or_scoring',
    'not_arbitrary_json_encounter_config',
  ];
}

export interface EncounterDirectorReadoutRequest {
  readonly presetId?: EncounterPresetId;
  readonly lifecycleScenario?: EncounterLifecycleScenario;
}

export interface EncounterTransitionRequest {
  readonly kind: 'runtime_session.encounter_transition_request.v0';
  readonly presetId: EncounterPresetId;
  readonly action: EncounterTransitionAction;
  readonly lifecycleScenario?: EncounterLifecycleScenario;
}

export interface EncounterTransitionEvent {
  readonly kind:
    | 'runtime_encounter.activated.v0'
    | 'runtime_encounter.lifecycle_synced.v0'
    | 'runtime_encounter.reset.v0';
  readonly eventHash: string;
}

export interface RuntimeSessionEncounterTransitionReceipt {
  readonly kind: EncounterTransitionReceiptKind;
  readonly sequenceId: number;
  readonly request: EncounterTransitionRequest;
  readonly status: EncounterTransitionStatus;
  readonly accepted: boolean;
  readonly rejectionReason?: EncounterTransitionRejectionReason;
  readonly event?: EncounterTransitionEvent;
  readonly before: EncounterDirectorReadout;
  readonly after: EncounterDirectorReadout;
  readonly hashes: {
    readonly transitionHash: string;
    readonly sessionHashBefore: string;
  readonly sessionHashAfter: string;
  };
}

export interface EncounterDirectorStateHashRecord {
  readonly presetId: EncounterPresetId;
  readonly status: EncounterStatus;
  readonly spawnedEnemyIds: readonly EncounterEntityInstanceId[];
  readonly defeatedEnemyIds: readonly EncounterEntityInstanceId[];
  readonly revision: number;
  readonly lastTransition: EncounterDirectorState['lastTransition'];
}

type EncounterHashPrimitive = string | number | boolean | null;
type EncounterHashValue =
  | EncounterHashPrimitive
  | readonly EncounterHashValue[]
  | object;

export interface EncounterTransitionResult {
  readonly accepted: boolean;
  readonly state: EncounterDirectorState;
  readonly eventKind?: EncounterTransitionEvent['kind'];
  readonly rejectionReason?: EncounterTransitionRejectionReason;
}

const ENCOUNTER_INSTANCE_ID: EncounterEntityInstanceId =
  'encounter.generated_tunnel_small.wave_1.enemy_001';

const ENCOUNTER_ENTITY_REF: EncounterEntityDefinitionRef = {
  source: 'project_bundle.entity_definition',
  definitionId: 'entity.enemy.generated_tunnel.basic.v0',
  entityDefinitionId: 'generated-tunnel.enemy.basic',
};

const ENCOUNTER_SPAWN_MARKER: EncounterSpawnMarkerRef = {
  source: 'generated_tunnel.spawn_marker',
  markerId: 'exit_hint',
  world: [3.5, 1.5, 7.5],
  yawDegrees: 180,
};

const ENCOUNTER_CONFIG_BASE = {
  kind: 'encounter_config.generated_tunnel_small.v0',
  source: 'project_bundle.encounter_preset',
  presetId: 'generated-tunnel-small-encounter',
  seed: 17,
  fixturePath:
    'harness/fixtures/encounters/generated-tunnel-small-encounter.snapshot.txt',
  enemyDefinitions: [
    {
      ref: ENCOUNTER_ENTITY_REF,
      displayName: 'Generated Tunnel Enemy',
      count: 1,
      runtimeEntityId: 20,
      capabilities: ['combat.health', 'enemy_policy', 'nav.agent'],
    },
  ],
  spawnMarkerRefs: [ENCOUNTER_SPAWN_MARKER],
  waves: [
    {
      waveId: 'wave.1',
      order: 0,
      enemyCount: 1,
      spawnMarkerIds: ['exit_hint'],
    },
  ],
} as const;

export const GENERATED_TUNNEL_SMALL_ENCOUNTER_CONFIG: EncounterConfigReadout = {
  ...ENCOUNTER_CONFIG_BASE,
  configHash: stableHash({
    kind: ENCOUNTER_CONFIG_BASE.kind,
    source: ENCOUNTER_CONFIG_BASE.source,
    presetId: ENCOUNTER_CONFIG_BASE.presetId,
    seed: ENCOUNTER_CONFIG_BASE.seed,
    enemyDefinitions: ENCOUNTER_CONFIG_BASE.enemyDefinitions,
    spawnMarkerRefs: ENCOUNTER_CONFIG_BASE.spawnMarkerRefs,
    waves: ENCOUNTER_CONFIG_BASE.waves,
  }),
  spawnOrderHash: stableHash({
    seed: ENCOUNTER_CONFIG_BASE.seed,
    spawns: [
      {
        instanceId: ENCOUNTER_INSTANCE_ID,
        order: 0,
        markerId: ENCOUNTER_SPAWN_MARKER.markerId,
        definitionId: ENCOUNTER_ENTITY_REF.definitionId,
      },
    ],
  }),
};

export function initialEncounterDirectorState(): EncounterDirectorState {
  return {
    presetId: 'generated-tunnel-small-encounter',
    status: 'pending',
    spawnedEnemyIds: [],
    defeatedEnemyIds: [],
    revision: 0,
    lastTransition: 'initialized',
  };
}

export function validateEncounterDirectorReadoutRequest(
  request: EncounterDirectorReadoutRequest | undefined,
): void {
  if (request?.presetId !== undefined && request.presetId !== 'generated-tunnel-small-encounter') {
    throw new TypeError(`Unsupported encounter preset: ${request.presetId}`);
  }

  if (
    request?.lifecycleScenario !== undefined &&
    request.lifecycleScenario !== 'active' &&
    request.lifecycleScenario !== 'generated_tunnel_enemy_defeated' &&
    request.lifecycleScenario !== 'generated_tunnel_player_defeated'
  ) {
    throw new TypeError(`Unsupported encounter lifecycle scenario: ${request.lifecycleScenario}`);
  }
}

export function validateEncounterTransitionRequest(
  request: EncounterTransitionRequest,
): EncounterTransitionRejectionReason | undefined {
  if (request.presetId !== 'generated-tunnel-small-encounter') {
    return 'unknown_encounter_preset';
  }

  if (
    request.action !== 'activate' &&
    request.action !== 'sync_lifecycle' &&
    request.action !== 'reset'
  ) {
    return 'invalid_encounter_transition';
  }

  if (
    request.lifecycleScenario !== undefined &&
    request.lifecycleScenario !== 'active' &&
    request.lifecycleScenario !== 'generated_tunnel_enemy_defeated' &&
    request.lifecycleScenario !== 'generated_tunnel_player_defeated'
  ) {
    return 'invalid_encounter_transition';
  }

  return undefined;
}

export function transitionEncounterDirectorState(input: {
  readonly state: EncounterDirectorState;
  readonly action: EncounterTransitionAction;
  readonly lifecycle: EncounterLifecycleInput;
}): EncounterTransitionResult {
  if (input.action === 'reset') {
    return {
      accepted: true,
      state: {
        ...initialEncounterDirectorState(),
        revision: input.state.revision + 1,
        lastTransition: 'reset',
      },
      eventKind: 'runtime_encounter.reset.v0',
    };
  }

  if (input.action === 'activate') {
    if (input.state.status !== 'pending') {
      return {
        accepted: false,
        state: input.state,
        rejectionReason: 'encounter_not_pending',
      };
    }

    return {
      accepted: true,
      state: {
        ...input.state,
        status: 'active',
        spawnedEnemyIds: [ENCOUNTER_INSTANCE_ID],
        revision: input.state.revision + 1,
        lastTransition: 'activated',
      },
      eventKind: 'runtime_encounter.activated.v0',
    };
  }

  if (input.lifecycle.playerDead || input.lifecycle.outcomeKind === 'lost') {
    return {
      accepted: true,
      state: {
        ...input.state,
        status: 'failed',
        revision: input.state.revision + 1,
        lastTransition: 'failed',
      },
      eventKind: 'runtime_encounter.lifecycle_synced.v0',
    };
  }

  if (input.lifecycle.enemyDead || input.lifecycle.outcomeKind === 'won') {
    return {
      accepted: true,
      state: {
        ...input.state,
        status: 'cleared',
        spawnedEnemyIds: [ENCOUNTER_INSTANCE_ID],
        defeatedEnemyIds: [ENCOUNTER_INSTANCE_ID],
        revision: input.state.revision + 1,
        lastTransition: 'cleared',
      },
      eventKind: 'runtime_encounter.lifecycle_synced.v0',
    };
  }

  return {
    accepted: true,
    state: {
      ...input.state,
      revision: input.state.revision + 1,
    },
    eventKind: 'runtime_encounter.lifecycle_synced.v0',
  };
}

export function buildEncounterDirectorReadout(input: {
  readonly state: EncounterDirectorState;
  readonly sequenceId: number;
  readonly tick: number;
  readonly sessionSeed: number;
  readonly sessionHash: string;
  readonly lifecycle: EncounterLifecycleInput;
}): EncounterDirectorReadout {
  const spawns = spawnInstancesForState(input.state);
  const activeEnemyCount = spawns.filter((spawn) => spawn.status === 'spawned').length;
  const pendingEnemyCount = spawns.filter((spawn) => spawn.status === 'pending').length;
  const defeatedEnemyCount = spawns.filter((spawn) => spawn.status === 'defeated').length;
  const spawnedEnemyCount = input.state.spawnedEnemyIds.length;
  const stateReadout: EncounterDirectorStateReadout = {
    status: input.state.status,
    revision: input.state.revision,
    lastTransition: input.state.lastTransition,
    activeEnemyCount,
    pendingEnemyCount,
    defeatedEnemyCount,
    spawnedEnemyCount,
    ...(input.state.status === 'failed' ? { failedReason: 'player_defeated' } : {}),
    ...(input.state.status === 'cleared'
      ? { clearedReason: 'all_enemies_defeated' }
      : {}),
  };
  const encounterHash = stableHash({
    presetId: input.state.presetId,
    sequenceId: input.sequenceId,
    tick: input.tick,
    state: stateReadout,
    spawns,
    lifecycle: input.lifecycle,
    configHash: GENERATED_TUNNEL_SMALL_ENCOUNTER_CONFIG.configHash,
  });

  return {
    kind: 'runtime_session.encounter_director.v0',
    sequenceId: input.sequenceId,
    tick: input.tick,
    presetId: input.state.presetId,
    sessionSeed: input.sessionSeed,
    config: GENERATED_TUNNEL_SMALL_ENCOUNTER_CONFIG,
    state: stateReadout,
    spawns,
    lifecycle: {
      outcomeKind: input.lifecycle.outcomeKind,
      terminal: input.lifecycle.terminal,
      enemyDead: input.lifecycle.enemyDead,
      playerDead: input.lifecycle.playerDead,
      lifecycleHash: input.lifecycle.lifecycleHash,
    },
    hashes: {
      encounterHash,
      spawnOrderHash: GENERATED_TUNNEL_SMALL_ENCOUNTER_CONFIG.spawnOrderHash,
      replayHash: stableHash({
        kind: 'encounter_director.replay_fixture.v0',
        encounterHash,
        spawnOrderHash: GENERATED_TUNNEL_SMALL_ENCOUNTER_CONFIG.spawnOrderHash,
      }),
      sessionHash: input.sessionHash,
    },
    nonClaims: [
      'not_wave_design',
      'not_demo_local_spawn_state',
      'not_loot_or_scoring',
      'not_arbitrary_json_encounter_config',
    ],
  };
}

export function buildEncounterTransitionReceipt(input: {
  readonly request: EncounterTransitionRequest;
  readonly sequenceId: number;
  readonly before: EncounterDirectorReadout;
  readonly after: EncounterDirectorReadout;
  readonly result: EncounterTransitionResult;
  readonly sessionHashBefore: string;
  readonly sessionHashAfter: string;
}): RuntimeSessionEncounterTransitionReceipt {
  const event =
    input.result.eventKind === undefined
      ? undefined
      : {
          kind: input.result.eventKind,
          eventHash: stableHash({
            kind: input.result.eventKind,
            request: input.request,
            before: input.before.hashes.encounterHash,
            after: input.after.hashes.encounterHash,
          }),
        };

  return {
    kind: 'runtime_session.encounter_transition_receipt.v0',
    sequenceId: input.sequenceId,
    request: input.request,
    status: input.result.accepted ? 'accepted' : 'rejected',
    accepted: input.result.accepted,
    ...(input.result.rejectionReason === undefined
      ? {}
      : { rejectionReason: input.result.rejectionReason }),
    ...(event === undefined ? {} : { event }),
    before: input.before,
    after: input.after,
    hashes: {
      transitionHash: stableHash({
        request: input.request,
        accepted: input.result.accepted,
        rejectionReason: input.result.rejectionReason ?? null,
        eventKind: input.result.eventKind ?? null,
        before: input.before.hashes.encounterHash,
        after: input.after.hashes.encounterHash,
        sessionHashBefore: input.sessionHashBefore,
        sessionHashAfter: input.sessionHashAfter,
      }),
      sessionHashBefore: input.sessionHashBefore,
      sessionHashAfter: input.sessionHashAfter,
    },
  };
}

export function encounterDirectorStateHashRecord(
  state: EncounterDirectorState,
): EncounterDirectorStateHashRecord {
  return {
    presetId: state.presetId,
    status: state.status,
    spawnedEnemyIds: state.spawnedEnemyIds,
    defeatedEnemyIds: state.defeatedEnemyIds,
    revision: state.revision,
    lastTransition: state.lastTransition,
  };
}

function spawnInstancesForState(
  state: EncounterDirectorState,
): readonly EncounterSpawnInstanceReadout[] {
  const status = spawnStatusForState(state);

  return [
    {
      instanceId: ENCOUNTER_INSTANCE_ID,
      runtimeEntityId: 20,
      waveId: 'wave.1',
      order: 0,
      enemy: ENCOUNTER_ENTITY_REF,
      spawnMarker: ENCOUNTER_SPAWN_MARKER,
      status,
    },
  ];
}

function spawnStatusForState(state: EncounterDirectorState): EncounterSpawnInstanceStatus {
  if (state.defeatedEnemyIds.includes(ENCOUNTER_INSTANCE_ID)) {
    return 'defeated';
  }

  if (state.status === 'failed') {
    return state.spawnedEnemyIds.includes(ENCOUNTER_INSTANCE_ID) ? 'spawned' : 'blocked';
  }

  if (state.spawnedEnemyIds.includes(ENCOUNTER_INSTANCE_ID)) {
    return 'spawned';
  }

  return 'pending';
}

function stableHash(value: EncounterHashValue | undefined): string {
  const json = stableStringify(value);
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  const mask = 0xffffffffffffffffn;

  for (let index = 0; index < json.length; index += 1) {
    hash ^= BigInt(json.charCodeAt(index));
    hash = (hash * prime) & mask;
  }

  return hash.toString(16).padStart(16, '0');
}

function stableStringify(value: EncounterHashValue | undefined): string {
  if (value === undefined) {
    return 'undefined';
  }

  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }

  if (Array.isArray(value)) {
    return `[${value.map((entry) => stableStringify(entry)).join(',')}]`;
  }

  const record = value as Record<string, EncounterHashValue | undefined>;
  return `{${Object.keys(record)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
    .join(',')}}`;
}
