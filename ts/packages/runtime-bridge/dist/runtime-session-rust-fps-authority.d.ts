import { type CombatRuntimeReadout } from './combat-readout.js';
import type { RuntimeSessionEcrpProjectState, RuntimeSessionLifecycleState } from './runtime-session.js';
export declare const RUNTIME_SESSION_RUST_FPS_AUTHORITY: {
    readonly ruleCrate: "rule-lifecycle";
    readonly combatServiceCrate: "svc-combat";
    readonly entityBootstrapServiceCrate: "svc-entity-authoring";
    readonly primaryFireReplayUnit: "runtime_session.fps.primary_fire.v0";
};
export declare function buildRustFpsAuthorityPrimaryFireReadout(input: {
    readonly projectState: RuntimeSessionEcrpProjectState | null;
    readonly lifecycleState: RuntimeSessionLifecycleState;
    readonly tick: number;
}): CombatRuntimeReadout;
//# sourceMappingURL=runtime-session-rust-fps-authority.d.ts.map