const ENCOUNTER_INSTANCE_ID = 'encounter.generated_tunnel_small.wave_1.enemy_001';
const ENCOUNTER_ENTITY_REF = {
    source: 'project_bundle.entity_definition',
    definitionId: 'entity.enemy.generated_tunnel.basic.v0',
    entityDefinitionId: 'generated-tunnel.enemy.basic',
};
const ENCOUNTER_SPAWN_MARKER = {
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
    fixturePath: 'harness/fixtures/encounters/generated-tunnel-small-encounter.snapshot.txt',
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
};
export const GENERATED_TUNNEL_SMALL_ENCOUNTER_CONFIG = {
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
export function initialEncounterDirectorState() {
    return {
        presetId: 'generated-tunnel-small-encounter',
        status: 'pending',
        spawnedEnemyIds: [],
        defeatedEnemyIds: [],
        revision: 0,
        lastTransition: 'initialized',
    };
}
export function validateEncounterDirectorReadoutRequest(request) {
    if (request?.presetId !== undefined && request.presetId !== 'generated-tunnel-small-encounter') {
        throw new TypeError(`Unsupported encounter preset: ${request.presetId}`);
    }
    if (request?.lifecycleScenario !== undefined &&
        request.lifecycleScenario !== 'active' &&
        request.lifecycleScenario !== 'generated_tunnel_enemy_defeated' &&
        request.lifecycleScenario !== 'generated_tunnel_player_defeated') {
        throw new TypeError(`Unsupported encounter lifecycle scenario: ${request.lifecycleScenario}`);
    }
}
export function validateEncounterTransitionRequest(request) {
    if (request.presetId !== 'generated-tunnel-small-encounter') {
        return 'unknown_encounter_preset';
    }
    if (request.action !== 'activate' &&
        request.action !== 'sync_lifecycle' &&
        request.action !== 'reset') {
        return 'invalid_encounter_transition';
    }
    if (request.lifecycleScenario !== undefined &&
        request.lifecycleScenario !== 'active' &&
        request.lifecycleScenario !== 'generated_tunnel_enemy_defeated' &&
        request.lifecycleScenario !== 'generated_tunnel_player_defeated') {
        return 'invalid_encounter_transition';
    }
    return undefined;
}
export function transitionEncounterDirectorState(input) {
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
export function buildEncounterDirectorReadout(input) {
    const spawns = spawnInstancesForState(input.state);
    const activeEnemyCount = spawns.filter((spawn) => spawn.status === 'spawned').length;
    const pendingEnemyCount = spawns.filter((spawn) => spawn.status === 'pending').length;
    const defeatedEnemyCount = spawns.filter((spawn) => spawn.status === 'defeated').length;
    const spawnedEnemyCount = input.state.spawnedEnemyIds.length;
    const stateReadout = {
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
export function buildEncounterTransitionReceipt(input) {
    const event = input.result.eventKind === undefined
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
export function encounterDirectorStateHashRecord(state) {
    return {
        presetId: state.presetId,
        status: state.status,
        spawnedEnemyIds: state.spawnedEnemyIds,
        defeatedEnemyIds: state.defeatedEnemyIds,
        revision: state.revision,
        lastTransition: state.lastTransition,
    };
}
function spawnInstancesForState(state) {
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
function spawnStatusForState(state) {
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
function stableHash(value) {
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
function stableStringify(value) {
    if (value === undefined) {
        return 'undefined';
    }
    if (value === null || typeof value !== 'object') {
        return JSON.stringify(value);
    }
    if (Array.isArray(value)) {
        return `[${value.map((entry) => stableStringify(entry)).join(',')}]`;
    }
    const record = value;
    return `{${Object.keys(record)
        .sort()
        .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
        .join(',')}}`;
}
//# sourceMappingURL=encounter-director.js.map