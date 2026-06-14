// @asha/script-host — sandboxed world-policy invocation with classified violations
// (#2393).
//
// A constrained world policy may misbehave: it may throw (a crash or a touched
// forbidden global surfaces as a thrown ReferenceError), return something that is
// not an array, or emit a value that is not a well-formed `PolicyWorldCommand`.
// None of these may silently corrupt the run. This module invokes a policy behind a
// guard that classifies every such violation into a stable, agent-legible
// diagnostic and keeps only the well-formed commands.
//
// The host never validates command *semantics* (that is Rust authority via
// `svc-policy-view`); it only guards *shape* and *isolation*.
//
// Isolation is runtime-enforced, not lint-only (#2427): the view is deep-frozen and
// ambient host capabilities are quarantined for the synchronous policy call (see
// ./isolation.ts), so view mutation and `process`/timer/`Function`-escape attempts
// throw and are classified as `policyThrew` rather than silently succeeding.
import { deepFreeze, runQuarantined } from './isolation.js';
/** Convenience constructor for a {@link NamedWorldPolicy}. */
export function defineWorldPolicy(name, policy) {
    return { name, policy };
}
const KNOWN_COMMAND_KINDS = new Set([
    'requestSetTransform',
    'requestAddLabel',
    'requestDisable',
    'noopMarker',
]);
/** Whether `value` is structurally a `PolicyWorldCommand` (shape only, not semantics). */
export function isWellFormedCommand(value) {
    if (typeof value !== 'object' || value === null) {
        return false;
    }
    const kind = value.kind;
    return typeof kind === 'string' && KNOWN_COMMAND_KINDS.has(kind);
}
function messageOf(error) {
    return error instanceof Error ? `${error.name}: ${error.message}` : String(error);
}
/**
 * Invoke one named world policy behind the sandbox guard. A throw, a non-array
 * result, or any malformed element is classified into a {@link SandboxViolation};
 * the run still returns whatever well-formed commands were produced (a throw yields
 * none). Never throws.
 */
export function runWorldPolicySandboxed(named, view, env) {
    let produced;
    try {
        // Freeze the view (shared across policies + replay diagnostics) and quarantine
        // ambient capabilities for the synchronous call. Either boundary turns a
        // hostile action into a throw, classified below — never a silent corruption.
        const frozenView = deepFreeze(view);
        produced = runQuarantined(() => named.policy(frozenView, env));
    }
    catch (error) {
        return {
            commands: [],
            violations: [{ code: 'policyThrew', policy: named.name, detail: messageOf(error) }],
        };
    }
    if (!Array.isArray(produced)) {
        return {
            commands: [],
            violations: [
                {
                    code: 'nonArrayResult',
                    policy: named.name,
                    detail: `policy returned ${typeof produced}, expected an array of commands`,
                },
            ],
        };
    }
    const commands = [];
    const violations = [];
    for (const [index, candidate] of produced.entries()) {
        if (isWellFormedCommand(candidate)) {
            commands.push(candidate);
        }
        else {
            violations.push({
                code: 'malformedCommand',
                policy: named.name,
                detail: `result[${index}] is not a well-formed PolicyWorldCommand`,
            });
        }
    }
    return { commands, violations };
}
//# sourceMappingURL=sandbox.js.map