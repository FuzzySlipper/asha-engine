export interface RuntimeSessionRestartResult {
  readonly sequenceId: number;
  readonly tick: number;
  readonly restartCount: number;
  readonly sessionHash: string;
}

export type RuntimeSessionLifecycleScenario =
  | 'current_session'
  | 'generated_tunnel_enemy_defeated'
  | 'generated_tunnel_player_defeated';
export type RuntimeSessionLifecycleRole = 'player' | 'enemy';
export type RuntimeSessionLifecycleOutcomeKind = 'in_progress' | 'won' | 'lost';
export type RuntimeSessionLifecycleEventKind =
  | 'runtime_lifecycle.enemy_defeated.v0'
  | 'runtime_lifecycle.player_defeated.v0';

export interface RuntimeSessionLifecycleStatusRequest {
  readonly scenario?: RuntimeSessionLifecycleScenario;
}

export interface RuntimeSessionLifecycleHealthReadout {
  readonly entity: number;
  readonly current: number;
  readonly max: number;
  readonly dead: boolean;
  readonly healthHash: string;
}

export interface RuntimeSessionLifecycleParticipantReadout {
  readonly role: RuntimeSessionLifecycleRole;
  readonly health: RuntimeSessionLifecycleHealthReadout;
  readonly dead: boolean;
}

export interface RuntimeSessionLifecycleEventReadout {
  readonly kind: RuntimeSessionLifecycleEventKind;
  readonly entity: number;
  readonly tick: number;
  readonly reason: 'combat_health_zero' | 'fixture_player_damage';
  readonly eventHash: string;
}

export interface RuntimeSessionLifecycleStatusReadout {
  readonly kind: 'runtime_session.lifecycle_status.v0';
  readonly scenario: RuntimeSessionLifecycleScenario;
  readonly sequenceId: number;
  readonly tick: number;
  readonly sessionHash: string;
  readonly player: RuntimeSessionLifecycleParticipantReadout;
  readonly enemy: RuntimeSessionLifecycleParticipantReadout;
  readonly outcome: {
    readonly kind: RuntimeSessionLifecycleOutcomeKind;
    readonly terminal: boolean;
    readonly reason: 'none' | 'enemy_defeated' | 'player_defeated';
    readonly label: string;
  };
  readonly restart: {
    readonly eligible: boolean;
    readonly intentKind: 'runtime.restart_session_intent';
    readonly reason: 'always_resettable_reference_fixture' | 'rust_epoch_restart';
  };
  readonly events: readonly RuntimeSessionLifecycleEventReadout[];
  readonly reset: {
    readonly seed: number;
    readonly resetHash: string;
  };
  readonly hashes: {
    readonly lifecycleHash: string;
    readonly playerHealthHash: string;
    readonly enemyHealthHash: string;
    readonly replayHash: string;
  };
  readonly nonClaims: readonly [
    'not_save_load_persistence',
    'not_ui_authority',
    'not_demo_local_lifecycle',
  ];
}

export type RuntimeSessionRestartIntentSource = 'hud_menu' | 'programmatic';
export type RuntimeSessionRestartIntentStatus = 'accepted' | 'rejected';
export type RuntimeSessionRestartIntentRejectionReason =
  | 'session_not_terminal'
  | 'session_hash_mismatch'
  | 'invalid_restart_intent';

export interface RuntimeSessionRestartIntent {
  readonly kind: 'runtime.restart_session_intent';
  readonly source: RuntimeSessionRestartIntentSource;
  readonly requireTerminal?: boolean;
  readonly expectedSessionHash?: string;
}

export interface RuntimeSessionRestartIntentRejection {
  readonly reason: RuntimeSessionRestartIntentRejectionReason;
  readonly detail: string;
}

export interface RuntimeSessionLifecycleRestartReceipt {
  readonly kind: 'runtime_session.restart_receipt.v0';
  readonly sequenceId: number;
  readonly intent: RuntimeSessionRestartIntent;
  readonly accepted: boolean;
  readonly status: RuntimeSessionRestartIntentStatus;
  readonly rejection: RuntimeSessionRestartIntentRejection | null;
  readonly statusBefore: RuntimeSessionLifecycleStatusReadout;
  readonly statusAfter: RuntimeSessionLifecycleStatusReadout;
  readonly restart: RuntimeSessionRestartResult | null;
  readonly sessionHashBefore: string;
  readonly sessionHashAfter: string;
  readonly resetHash: string;
}

export interface RuntimeSessionLifecycleState {
  readonly player: RuntimeSessionLifecycleHealthReadout;
  readonly enemy: RuntimeSessionLifecycleHealthReadout;
  readonly terminalEvent: RuntimeSessionLifecycleEventReadout | null;
  readonly revision: number;
}
