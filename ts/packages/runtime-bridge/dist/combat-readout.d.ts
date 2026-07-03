export type CombatReadoutScenario = 'generated_tunnel_fire_hit' | 'generated_tunnel_geometry_blocked_miss';
export type CombatFireOutcomeReadout = {
    readonly kind: 'hit';
    readonly target: 20;
    readonly distance: 3.5;
    readonly hitPosition: null;
    readonly defeated: true;
} | {
    readonly kind: 'miss';
    readonly reason: 'geometryBlocked' | 'noTarget';
};
export type CombatEventReadout = {
    readonly kind: 'fire_hit';
    readonly shooter: 10;
    readonly target: 20;
    readonly distance: 3.5;
    readonly tick: 7;
} | {
    readonly kind: 'fire_missed';
    readonly shooter: 10;
    readonly reason: 'geometryBlocked' | 'noTarget';
    readonly tick: 7;
} | {
    readonly kind: 'damage_applied';
    readonly target: 20;
    readonly amount: 40;
    readonly before: 40;
    readonly after: 0;
} | {
    readonly kind: 'entity_defeated';
    readonly target: 20;
};
export interface CombatHealthReadout {
    readonly entity: 20;
    readonly current: number;
    readonly max: number;
    readonly dead: boolean;
}
export interface CombatFireControlReadout {
    readonly ammo: 2;
    readonly cooldownTicksRemaining: 4;
    readonly cooldownTicksAfterFire: 4;
}
export interface CombatRuntimeReadout {
    readonly scenario: CombatReadoutScenario;
    readonly outcome: CombatFireOutcomeReadout;
    readonly events: readonly CombatEventReadout[];
    readonly health: readonly CombatHealthReadout[];
    readonly nextFireControl: CombatFireControlReadout;
    readonly healthHash: string;
    readonly replayHash: string;
    readonly fixture: 'harness/fixtures/combat/generated-tunnel-fire.snapshot.txt' | null;
}
export declare const GENERATED_TUNNEL_FIRE_HIT_READOUT: CombatRuntimeReadout;
export declare const GENERATED_TUNNEL_FIRE_MISS_READOUT: CombatRuntimeReadout;
//# sourceMappingURL=combat-readout.d.ts.map