/**
 * A deterministic splitmix64 random stream. Construct it from a seed; the sequence
 * it produces depends only on that seed, so two streams with the same seed are
 * identical. It carries its own cursor — it never reads ambient state.
 */
export declare class DeterministicRng {
    #private;
    constructor(seed: number);
    /** The next unsigned 32-bit integer. */
    nextU32(): number;
    /** The next float in `[0, 1)`. */
    nextFloat(): number;
    /** A deterministic integer in `[min, max)` (`min` when the range is empty). */
    nextInRange(min: number, max: number): number;
}
/**
 * The deterministic envelope handed to a capable policy: the world tick, the seed
 * that produced the stream, and the stream itself. There is deliberately nothing
 * else — no clock, no I/O, no ambient capability.
 */
export interface PolicyEnv {
    readonly tick: number;
    readonly seed: number;
    readonly rng: DeterministicRng;
}
/** Construct a fresh deterministic envelope for a tick + seed. */
export declare function makeEnv(tick: number, seed: number): PolicyEnv;
//# sourceMappingURL=env.d.ts.map