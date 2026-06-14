import { type PolicyWorldCommand, type PolicyWorldView } from '@asha/script-sdk';
import { type NamedWorldPolicy, type SandboxViolation } from './sandbox.js';
/** The deterministic input to a policy tick stage. */
export interface PolicyTickStageInput {
    readonly tick: number;
    /** Seed for the deterministic RNG envelope handed to policies. */
    readonly seed: number;
    /** The read-only world view projected for this tick (authority-produced). */
    readonly view: PolicyWorldView;
    /** The policies to run, in order. */
    readonly policies: readonly NamedWorldPolicy[];
}
/** Per-policy execution metadata, for replay/diagnostics. */
export interface PolicyExecutionRecord {
    readonly policy: string;
    /** The seed of the deterministic envelope this policy ran with. */
    readonly seed: number;
    readonly proposedCount: number;
    readonly violationCount: number;
}
/** The result of a policy tick stage: proposals to hand to authority, plus audit. */
export interface PolicyTickStageResult {
    readonly tick: number;
    readonly seed: number;
    /** All well-formed proposed commands, in policy order. */
    readonly proposed: readonly PolicyWorldCommand[];
    /** Classified sandbox violations across all policies (never silent). */
    readonly violations: readonly SandboxViolation[];
    /** Per-policy execution metadata recorded for the replay/diagnostics path. */
    readonly executions: readonly PolicyExecutionRecord[];
}
/**
 * Run the deterministic policy tick stage. Each policy gets its own deterministic
 * envelope derived from `(tick, seed + index)`, so policy order is stable and one
 * policy's RNG draws cannot perturb another's. Re-running with the same input
 * reproduces the same proposals and violations exactly.
 */
export declare function runPolicyTickStage(input: PolicyTickStageInput): PolicyTickStageResult;
//# sourceMappingURL=tick.d.ts.map