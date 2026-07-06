export type CombatReadoutScenario = 'generated_tunnel_fire_hit' | 'generated_tunnel_geometry_blocked_miss' | 'runtime_session_loaded_project_fire_hit';
export type CombatFireOutcomeReadout = {
    readonly kind: 'hit';
    readonly target: number;
    readonly distance: number;
    readonly hitPosition: null;
    readonly defeated: boolean;
} | {
    readonly kind: 'miss';
    readonly reason: 'geometryBlocked' | 'noTarget';
};
export type CombatEventReadout = {
    readonly kind: 'fire_hit';
    readonly shooter: number;
    readonly target: number;
    readonly distance: number;
    readonly tick: number;
} | {
    readonly kind: 'fire_missed';
    readonly shooter: number;
    readonly reason: 'geometryBlocked' | 'noTarget';
    readonly tick: number;
} | {
    readonly kind: 'damage_applied';
    readonly target: number;
    readonly amount: number;
    readonly before: number;
    readonly after: number;
} | {
    readonly kind: 'entity_defeated';
    readonly target: number;
};
export interface CombatHealthReadout {
    readonly entity: number;
    readonly current: number;
    readonly max: number;
    readonly dead: boolean;
}
export interface CombatFireControlReadout {
    readonly ammo: 2;
    readonly cooldownTicksRemaining: 4;
    readonly cooldownTicksAfterFire: 4;
}
export type CombatRuntimeAuthoritySource = 'rust_bridge' | 'reference_bridge' | 'reference_fixture';
export interface CombatRuntimeAuthorityReadout {
    readonly source: CombatRuntimeAuthoritySource;
    readonly backend: 'native_rust' | 'reference_bridge' | null;
    readonly surface: string;
    readonly mutationOwner: string | null;
    readonly workspaceTrace: readonly string[];
}
export interface CombatRuntimeReadout {
    readonly scenario: CombatReadoutScenario;
    readonly outcome: CombatFireOutcomeReadout;
    readonly events: readonly CombatEventReadout[];
    readonly health: readonly CombatHealthReadout[];
    readonly nextFireControl: CombatFireControlReadout;
    readonly healthHash: string;
    readonly replayHash: string;
    readonly authority: CombatRuntimeAuthorityReadout;
    readonly fixture: 'harness/fixtures/combat/generated-tunnel-fire.snapshot.txt' | null;
}
export declare const GENERATED_TUNNEL_FIRE_HIT_READOUT: CombatRuntimeReadout;
export declare const GENERATED_TUNNEL_FIRE_MISS_READOUT: CombatRuntimeReadout;
//# sourceMappingURL=combat-readout.d.ts.map