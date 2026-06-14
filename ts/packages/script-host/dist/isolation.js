// @asha/script-host — runtime-enforced isolation at the policy boundary (#2427).
//
// Lint + try/catch discipline is not enough: a hostile or miscompiled policy that
// runs in the host realm can still (a) mutate the world view shared with later
// policies and replay diagnostics, or (b) reach ambient host capabilities
// (`process`, timers, network, wall-clock) that break determinism/isolation.
//
// This module adds two runtime boundaries the host applies around every policy
// invocation:
//
//   1. `deepFreeze` — the world view is deeply frozen before a policy sees it, so a
//      mutation attempt throws (and is classified) instead of silently corrupting
//      the view observed by the next policy or by replay diagnostics.
//   2. `runQuarantined` — for the *synchronous* duration of the policy call, a
//      denylist of ambient globals (and `Math.random`) is replaced with a throwing
//      proxy, then restored. Touching `process`, `setTimeout`, `fetch`, `Date`,
//      `Math.random`, or `Function('return process')()` therefore throws a
//      classified `PolicyCapabilityError` rather than succeeding.
//
// # Known limitations (documented, not hidden)
//
// This is in-realm hardening, not a separate-realm sandbox. It is safe because
// policy execution is synchronous and single-threaded — the quarantine is active
// only while the policy runs and is always restored in a `finally`. It does NOT
// defend against:
//   - intrinsics reached by constructor reflection (e.g. `(function(){}).constructor`),
//   - capability references a policy captured *before* the call,
//   - asynchronous escapes (a policy that returns a Promise is rejected upstream as
//     a non-array result, so async is not a supported policy shape anyway).
// Full cross-realm isolation (a worker / `node:vm` running policy *source*) is the
// follow-on; until then lint blocks source-authored escapes and this layer blocks
// the listed runtime escapes.
/** Thrown when a policy touches a quarantined host capability at runtime. */
export class PolicyCapabilityError extends Error {
    capability;
    constructor(capability) {
        super(`policy attempted to use a quarantined host capability: ${capability}`);
        this.capability = capability;
        this.name = 'PolicyCapabilityError';
    }
}
/**
 * Recursively freeze `value` and everything reachable from it. Returns the same
 * reference (frozen in place) so a shared view is protected for every later policy.
 */
export function deepFreeze(value) {
    if (value !== null && typeof value === 'object' && !Object.isFrozen(value)) {
        Object.freeze(value);
        for (const key of Object.keys(value)) {
            deepFreeze(value[key]);
        }
    }
    return value;
}
// ── Capability quarantine ─────────────────────────────────────────────────────
/** Ambient global property names neutralized for the duration of a policy call. */
export const QUARANTINED_GLOBALS = [
    'process',
    'fetch',
    'XMLHttpRequest',
    'WebSocket',
    'setTimeout',
    'setInterval',
    'setImmediate',
    'clearTimeout',
    'clearInterval',
    'queueMicrotask',
    'Date',
];
/** A value that throws `PolicyCapabilityError` on call, construct, or property read. */
function throwingCapability(name) {
    const fail = () => {
        throw new PolicyCapabilityError(name);
    };
    // Proxy a function so call (`setTimeout(...)`), construct (`new Date()`), and
    // property read (`process.env`) all fail closed with the same classified error.
    return new Proxy(fail, {
        apply: fail,
        construct: fail,
        get: fail,
    });
}
/**
 * Run `fn` with the quarantined globals replaced by throwing proxies, restoring the
 * originals afterwards (even if `fn` throws). Synchronous by contract.
 */
export function runQuarantined(fn) {
    const g = globalThis;
    const saved = [];
    for (const name of QUARANTINED_GLOBALS) {
        const had = Object.prototype.hasOwnProperty.call(g, name);
        saved.push({ name, had, value: had ? g[name] : undefined });
        try {
            g[name] = throwingCapability(name);
        }
        catch {
            // A non-writable global stays as-is; lint still blocks source use of it.
        }
    }
    // `Math.random` is a method, not a global binding — neutralize it via its own
    // descriptor (computed access keeps the sandbox lint rule satisfied here).
    const mathRandom = Object.getOwnPropertyDescriptor(Math, 'random');
    try {
        Object.defineProperty(Math, 'random', {
            configurable: true,
            writable: true,
            value: throwingCapability('Math.random'),
        });
    }
    catch {
        // ignore — restored below only if we captured a descriptor
    }
    try {
        return fn();
    }
    finally {
        if (mathRandom) {
            Object.defineProperty(Math, 'random', mathRandom);
        }
        for (const { name, had, value } of saved) {
            try {
                if (had) {
                    g[name] = value;
                }
                else {
                    delete g[name];
                }
            }
            catch {
                // best-effort restore; a non-writable global never changed above
            }
        }
    }
}
//# sourceMappingURL=isolation.js.map