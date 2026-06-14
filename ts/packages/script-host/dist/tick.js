// @asha/script-host — the deterministic policy tick stage (#2394).
//
// This is the TS (proposal) half of the world-layer policy pipeline — an explicit,
// ordered stage, NOT a generic event bus:
//
//   project view → run policies (sandboxed) → collect proposed commands
//
// Authority (`svc-policy-view`) owns the rest of the loop (validate → apply →
// record). This stage takes an already-projected view plus a deterministic
// envelope, runs each registered policy behind the sandbox guard, and collects the
// proposed commands and classified violations. A policy crash, malformed output, or
// sandbox violation is isolated to that policy — the remaining policies still run,
// and nothing here mutates authority.
import { makeEnv } from '@asha/script-sdk';
import { runWorldPolicySandboxed, } from './sandbox.js';
/**
 * Run the deterministic policy tick stage. Each policy gets its own deterministic
 * envelope derived from `(tick, seed + index)`, so policy order is stable and one
 * policy's RNG draws cannot perturb another's. Re-running with the same input
 * reproduces the same proposals and violations exactly.
 */
export function runPolicyTickStage(input) {
    const proposed = [];
    const violations = [];
    const executions = [];
    input.policies.forEach((named, index) => {
        const policySeed = input.seed + index;
        const env = makeEnv(input.tick, policySeed);
        const result = runWorldPolicySandboxed(named, input.view, env);
        proposed.push(...result.commands);
        violations.push(...result.violations);
        executions.push({
            policy: named.name,
            seed: policySeed,
            proposedCount: result.commands.length,
            violationCount: result.violations.length,
        });
    });
    return { tick: input.tick, seed: input.seed, proposed, violations, executions };
}
//# sourceMappingURL=tick.js.map