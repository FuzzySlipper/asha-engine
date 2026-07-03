export const GENERATED_TUNNEL_FIRE_HIT_READOUT = {
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
export const GENERATED_TUNNEL_FIRE_MISS_READOUT = {
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
//# sourceMappingURL=combat-readout.js.map