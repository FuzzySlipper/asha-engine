import type { Policy, PolicyCommand, PolicyView } from '@asha/script-sdk';
export * from './sandbox.js';
export * from './isolation.js';
export * from './tick.js';
/** A policy paired with a stable name, so diagnostics can identify it. */
export interface NamedPolicy {
    readonly name: string;
    readonly policy: Policy;
}
/** Convenience constructor for a {@link NamedPolicy}. */
export declare function definePolicy(name: string, policy: Policy): NamedPolicy;
/** A structured note about something that went wrong during invocation. */
export interface HostDiagnostic {
    readonly kind: 'policy-threw';
    /** The `name` of the policy that produced this diagnostic. */
    readonly policy: string;
    /** A human-readable message extracted from the thrown value. */
    readonly message: string;
}
/** The result of running one or more policies over a single view. */
export interface HostRunResult {
    /** Proposed commands, in the order policies emitted them. */
    readonly commands: readonly PolicyCommand[];
    /** Structured diagnostics; empty on a fully successful run. */
    readonly diagnostics: readonly HostDiagnostic[];
}
/**
 * An explicit, ordered buffer of proposed commands.
 *
 * The buffer never validates or transforms commands — it only preserves order.
 * {@link CommandBuffer.collected} returns a fresh array so a caller cannot reach
 * back in and mutate the host's internal state.
 */
export declare class CommandBuffer {
    #private;
    /** Append one command. */
    push(command: PolicyCommand): void;
    /** Append a sequence of commands, preserving their order. */
    extend(commands: readonly PolicyCommand[]): void;
    /** The commands collected so far, as a defensive copy. */
    collected(): readonly PolicyCommand[];
    /** How many commands have been collected. */
    get length(): number;
}
/**
 * Invoke a single named policy against a read-only view and collect its
 * proposed commands. If the policy throws, the throw is captured as a structured
 * diagnostic and the returned command list is empty for that policy.
 */
export declare function invokePolicy(named: NamedPolicy, view: PolicyView): HostRunResult;
/**
 * Invoke several named policies against the same view, collecting all proposed
 * commands in policy order. A policy that throws contributes a diagnostic but
 * does not abort the remaining policies, and contributes no commands.
 */
export declare function invokePolicies(policies: readonly NamedPolicy[], view: PolicyView): HostRunResult;
//# sourceMappingURL=index.d.ts.map