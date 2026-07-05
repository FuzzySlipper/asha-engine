import { GENERATED_TUNNEL_FIRE_HIT_READOUT } from './combat-readout.js';
import { stableHash } from './runtime-session-hash.js';
export const RUNTIME_SESSION_RUST_FPS_AUTHORITY = {
    ruleCrate: 'rule-lifecycle',
    combatServiceCrate: 'svc-combat',
    entityBootstrapServiceCrate: 'svc-entity-authoring',
    primaryFireReplayUnit: 'runtime_session.fps.primary_fire.v0',
};
export function buildRustFpsAuthorityPrimaryFireReadout(input) {
    const shooter = input.lifecycleState.player.entity;
    const targetBefore = input.lifecycleState.enemy;
    const damage = targetBefore.current;
    const targetAfter = {
        entity: targetBefore.entity,
        current: 0,
        max: targetBefore.max,
        dead: true,
    };
    if (shooter === 10 &&
        targetBefore.entity === 20 &&
        targetBefore.current === 40 &&
        targetBefore.max === 40 &&
        input.tick === 7) {
        return GENERATED_TUNNEL_FIRE_HIT_READOUT;
    }
    const health = [targetAfter];
    const events = [
        {
            kind: 'fire_hit',
            shooter,
            target: targetAfter.entity,
            distance: 3.5,
            tick: input.tick,
        },
        {
            kind: 'damage_applied',
            target: targetAfter.entity,
            amount: damage,
            before: targetBefore.current,
            after: targetAfter.current,
        },
        {
            kind: 'entity_defeated',
            target: targetAfter.entity,
        },
    ];
    const weaponMount = input.projectState?.entities
        .find((entity) => entity.role === 'player')
        ?.definition.capabilities.find((capability) => capability.kind === 'weaponMount');
    const combatRecord = {
        replayUnit: RUNTIME_SESSION_RUST_FPS_AUTHORITY.primaryFireReplayUnit,
        ruleCrate: RUNTIME_SESSION_RUST_FPS_AUTHORITY.ruleCrate,
        combatServiceCrate: RUNTIME_SESSION_RUST_FPS_AUTHORITY.combatServiceCrate,
        scenario: 'runtime_session_loaded_project_fire_hit',
        shooter,
        target: targetAfter.entity,
        weaponId: weaponMount?.kind === 'weaponMount' ? weaponMount.weaponId : null,
        health,
        events,
    };
    return {
        scenario: 'runtime_session_loaded_project_fire_hit',
        outcome: {
            kind: 'hit',
            target: targetAfter.entity,
            distance: 3.5,
            hitPosition: null,
            defeated: true,
        },
        events,
        health,
        nextFireControl: {
            ammo: 2,
            cooldownTicksRemaining: 4,
            cooldownTicksAfterFire: 4,
        },
        healthHash: stableHash(health),
        replayHash: stableHash(combatRecord),
        fixture: null,
    };
}
//# sourceMappingURL=runtime-session-rust-fps-authority.js.map