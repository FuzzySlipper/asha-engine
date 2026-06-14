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

import { makeEnv, type PolicyWorldCommand, type PolicyWorldView } from '@asha/script-sdk';

import {
  runWorldPolicySandboxed,
  type NamedWorldPolicy,
  type SandboxViolation,
} from './sandbox.js';

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
export function runPolicyTickStage(input: PolicyTickStageInput): PolicyTickStageResult {
  const proposed: PolicyWorldCommand[] = [];
  const violations: SandboxViolation[] = [];
  const executions: PolicyExecutionRecord[] = [];

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
