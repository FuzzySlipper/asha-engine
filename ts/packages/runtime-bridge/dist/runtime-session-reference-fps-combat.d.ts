import { type CombatRuntimeReadout } from './combat-readout.js';
import type { RuntimeActionIntentEnvelope } from './runtime-action.js';
import type { RuntimeSessionEcrpProjectState, RuntimeSessionLifecycleState } from './runtime-session.js';
export declare const REFERENCE_FPS_COMBAT_FIXTURE_PROVENANCE: {
    readonly ruleCrate: "rule-lifecycle";
    readonly combatServiceCrate: "svc-combat";
    readonly entityBootstrapServiceCrate: "svc-entity-authoring";
    readonly primaryFireReplayUnit: "runtime_session.fps.primary_fire.v0";
};
export declare function buildReferenceFpsCombatFixturePrimaryFireReadout(input: {
    readonly projectState: RuntimeSessionEcrpProjectState | null;
    readonly lifecycleState: RuntimeSessionLifecycleState;
    readonly source: RuntimeActionIntentEnvelope['source'];
    readonly tick: number;
}): CombatRuntimeReadout;
//# sourceMappingURL=runtime-session-reference-fps-combat.d.ts.map