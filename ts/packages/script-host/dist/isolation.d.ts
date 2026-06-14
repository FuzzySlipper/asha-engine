/** Thrown when a policy touches a quarantined host capability at runtime. */
export declare class PolicyCapabilityError extends Error {
    readonly capability: string;
    constructor(capability: string);
}
/**
 * Recursively freeze `value` and everything reachable from it. Returns the same
 * reference (frozen in place) so a shared view is protected for every later policy.
 */
export declare function deepFreeze<T>(value: T): T;
/** Ambient global property names neutralized for the duration of a policy call. */
export declare const QUARANTINED_GLOBALS: readonly string[];
/**
 * Run `fn` with the quarantined globals replaced by throwing proxies, restoring the
 * originals afterwards (even if `fn` throws). Synchronous by contract.
 */
export declare function runQuarantined<T>(fn: () => T): T;
//# sourceMappingURL=isolation.d.ts.map