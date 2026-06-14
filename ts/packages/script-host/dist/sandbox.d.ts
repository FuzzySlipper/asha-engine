import type { PolicyEnv, PolicyWorldCommand, WorldPolicyWithEnv } from '@asha/script-sdk';
/** A world policy paired with a stable name, so diagnostics can identify it. */
export interface NamedWorldPolicy {
    readonly name: string;
    readonly policy: WorldPolicyWithEnv;
}
/** Convenience constructor for a {@link NamedWorldPolicy}. */
export declare function defineWorldPolicy(name: string, policy: WorldPolicyWithEnv): NamedWorldPolicy;
/** The classified kinds of sandbox violation a guarded run can surface. */
export type SandboxViolationCode = 'policyThrew' | 'nonArrayResult' | 'malformedCommand';
/** A structured, stable diagnostic about a sandbox violation. */
export interface SandboxViolation {
    readonly code: SandboxViolationCode;
    /** The `name` of the offending policy. */
    readonly policy: string;
    /** A human-readable, agent-legible detail. */
    readonly detail: string;
}
/** The result of a guarded world-policy run. */
export interface SandboxRunResult {
    /** Only the well-formed proposed commands, in order. */
    readonly commands: readonly PolicyWorldCommand[];
    /** Classified violations; empty on a clean run. */
    readonly violations: readonly SandboxViolation[];
}
/** Whether `value` is structurally a `PolicyWorldCommand` (shape only, not semantics). */
export declare function isWellFormedCommand(value: unknown): value is PolicyWorldCommand;
/**
 * Invoke one named world policy behind the sandbox guard. A throw, a non-array
 * result, or any malformed element is classified into a {@link SandboxViolation};
 * the run still returns whatever well-formed commands were produced (a throw yields
 * none). Never throws.
 */
export declare function runWorldPolicySandboxed(named: NamedWorldPolicy, view: Parameters<WorldPolicyWithEnv>[0], env: PolicyEnv): SandboxRunResult;
//# sourceMappingURL=sandbox.d.ts.map