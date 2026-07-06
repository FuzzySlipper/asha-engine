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
export type EncounterTransitionRejectionReason = 'encounter_not_pending' | 'invalid_encounter_transition' | 'unknown_encounter_preset';
export type EncounterLifecycleOutcomeKind = 'in_progress' | 'won' | 'lost';
export type EncounterLifecycleScenario = 'active' | 'generated_tunnel_enemy_defeated' | 'generated_tunnel_player_defeated';
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
    readonly lastTransition: 'initialized' | 'activated' | 'cleared' | 'failed' | 'reset';
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
export interface EncounterDirectorAuthorityReadout {
    readonly source: 'rust_bridge' | 'reference_bridge' | 'reference_fixture';
    readonly backend: 'native_rust' | 'reference_bridge' | null;
    readonly surface: string;
    readonly mutationOwner: string;
    readonly readSets: readonly {
        readonly viewKind: string;
        readonly owner: string;
        readonly readSet: readonly string[];
    }[];
    readonly workspaceTrace: readonly string[];
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
    readonly authority: EncounterDirectorAuthorityReadout;
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
        'not_arbitrary_json_encounter_config'
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
    readonly kind: 'runtime_encounter.activated.v0' | 'runtime_encounter.lifecycle_synced.v0' | 'runtime_encounter.reset.v0';
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
export interface EncounterTransitionResult {
    readonly accepted: boolean;
    readonly state: EncounterDirectorState;
    readonly eventKind?: EncounterTransitionEvent['kind'];
    readonly rejectionReason?: EncounterTransitionRejectionReason;
}
export declare const GENERATED_TUNNEL_SMALL_ENCOUNTER_CONFIG: EncounterConfigReadout;
export declare function initialEncounterDirectorState(): EncounterDirectorState;
export declare function validateEncounterDirectorReadoutRequest(request: EncounterDirectorReadoutRequest | undefined): void;
export declare function validateEncounterTransitionRequest(request: EncounterTransitionRequest): EncounterTransitionRejectionReason | undefined;
export declare function transitionEncounterDirectorState(input: {
    readonly state: EncounterDirectorState;
    readonly action: EncounterTransitionAction;
    readonly lifecycle: EncounterLifecycleInput;
}): EncounterTransitionResult;
export declare function buildEncounterDirectorReadout(input: {
    readonly state: EncounterDirectorState;
    readonly sequenceId: number;
    readonly tick: number;
    readonly sessionSeed: number;
    readonly sessionHash: string;
    readonly lifecycle: EncounterLifecycleInput;
    readonly authority?: EncounterDirectorAuthorityReadout;
}): EncounterDirectorReadout;
export declare function buildEncounterTransitionReceipt(input: {
    readonly request: EncounterTransitionRequest;
    readonly sequenceId: number;
    readonly before: EncounterDirectorReadout;
    readonly after: EncounterDirectorReadout;
    readonly result: EncounterTransitionResult;
    readonly sessionHashBefore: string;
    readonly sessionHashAfter: string;
}): RuntimeSessionEncounterTransitionReceipt;
export declare function encounterDirectorStateHashRecord(state: EncounterDirectorState): EncounterDirectorStateHashRecord;
//# sourceMappingURL=encounter-director.d.ts.map