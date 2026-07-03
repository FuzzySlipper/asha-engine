export type CombatReadoutScenario = 'generated_tunnel_fire_hit' | 'generated_tunnel_geometry_blocked_miss';

export type CombatFireOutcomeReadout =
  | {
      readonly kind: 'hit';
      readonly target: 20;
      readonly distance: 3.5;
      readonly hitPosition: null;
      readonly defeated: true;
    }
  | {
      readonly kind: 'miss';
      readonly reason: 'geometryBlocked' | 'noTarget';
    };

export type CombatEventReadout =
  | {
      readonly kind: 'fire_hit';
      readonly shooter: 10;
      readonly target: 20;
      readonly distance: 3.5;
      readonly tick: 7;
    }
  | {
      readonly kind: 'fire_missed';
      readonly shooter: 10;
      readonly reason: 'geometryBlocked' | 'noTarget';
      readonly tick: 7;
    }
  | {
      readonly kind: 'damage_applied';
      readonly target: 20;
      readonly amount: 40;
      readonly before: 40;
      readonly after: 0;
    }
  | {
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

export const GENERATED_TUNNEL_FIRE_HIT_READOUT: CombatRuntimeReadout = {
  scenario: 'generated_tunnel_fire_hit',
  outcome: {
    kind: 'hit',
    target: 20,
    distance: 3.5,
    hitPosition: null,
    defeated: true,
  },
  events: [
    { kind: 'fire_hit', shooter: 10, target: 20, distance: 3.5, tick: 7 },
    { kind: 'damage_applied', target: 20, amount: 40, before: 40, after: 0 },
    { kind: 'entity_defeated', target: 20 },
  ],
  health: [{ entity: 20, current: 0, max: 40, dead: true }],
  nextFireControl: {
    ammo: 2,
    cooldownTicksRemaining: 4,
    cooldownTicksAfterFire: 4,
  },
  healthHash: '3c89045230f2d9d9',
  replayHash: '6b133026c511b0f5',
  fixture: 'harness/fixtures/combat/generated-tunnel-fire.snapshot.txt',
};

export const GENERATED_TUNNEL_FIRE_MISS_READOUT: CombatRuntimeReadout = {
  scenario: 'generated_tunnel_geometry_blocked_miss',
  outcome: {
    kind: 'miss',
    reason: 'geometryBlocked',
  },
  events: [{ kind: 'fire_missed', shooter: 10, reason: 'geometryBlocked', tick: 7 }],
  health: [{ entity: 20, current: 100, max: 100, dead: false }],
  nextFireControl: {
    ammo: 2,
    cooldownTicksRemaining: 4,
    cooldownTicksAfterFire: 4,
  },
  healthHash: '56b1331c0f202ff1',
  replayHash: '3b1e1a9897571bc4',
  fixture: null,
};
