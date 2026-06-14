// @asha/script-host — invokes constrained policies and collects their proposed
// commands deterministically.
//
// The host is a *coordinator*, not an authority. It hands a policy a read-only
// view, gathers the commands the policy proposes (in order), and reports
// structured diagnostics if a policy misbehaves. It never validates commands,
// never owns or mutates state, and never reaches outside the policy lane — Rust
// remains the sole validator (proven over the bridge in a later task).
// Sandboxed world-policy invocation with classified violations (#2393).
export * from './sandbox.js';
// Runtime-enforced policy isolation: deep-frozen views + capability quarantine (#2427).
export * from './isolation.js';
// Deterministic policy tick stage (#2394).
export * from './tick.js';
/** Convenience constructor for a {@link NamedPolicy}. */
export function definePolicy(name, policy) {
    return { name, policy };
}
// ── Command collection ────────────────────────────────────────────────────────
/**
 * An explicit, ordered buffer of proposed commands.
 *
 * The buffer never validates or transforms commands — it only preserves order.
 * {@link CommandBuffer.collected} returns a fresh array so a caller cannot reach
 * back in and mutate the host's internal state.
 */
export class CommandBuffer {
    #commands = [];
    /** Append one command. */
    push(command) {
        this.#commands.push(command);
    }
    /** Append a sequence of commands, preserving their order. */
    extend(commands) {
        for (const command of commands) {
            this.#commands.push(command);
        }
    }
    /** The commands collected so far, as a defensive copy. */
    collected() {
        return [...this.#commands];
    }
    /** How many commands have been collected. */
    get length() {
        return this.#commands.length;
    }
}
// ── Invocation ────────────────────────────────────────────────────────────────
/** Extract a readable message from an unknown thrown value. */
function messageOf(error) {
    if (error instanceof Error) {
        return error.message;
    }
    return String(error);
}
/**
 * Invoke a single named policy against a read-only view and collect its
 * proposed commands. If the policy throws, the throw is captured as a structured
 * diagnostic and the returned command list is empty for that policy.
 */
export function invokePolicy(named, view) {
    return invokePolicies([named], view);
}
/**
 * Invoke several named policies against the same view, collecting all proposed
 * commands in policy order. A policy that throws contributes a diagnostic but
 * does not abort the remaining policies, and contributes no commands.
 */
export function invokePolicies(policies, view) {
    const buffer = new CommandBuffer();
    const diagnostics = [];
    for (const { name, policy } of policies) {
        try {
            buffer.extend(policy(view));
        }
        catch (error) {
            diagnostics.push({
                kind: 'policy-threw',
                policy: name,
                message: messageOf(error),
            });
        }
    }
    return { commands: buffer.collected(), diagnostics };
}
//# sourceMappingURL=index.js.map