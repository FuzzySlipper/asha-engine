// @asha/script-sdk — the deterministic execution envelope for a policy (#2393).
//
// A constrained policy must never read wall-clock time or ambient randomness: those
// are non-deterministic and would break replay. Instead, the *only* source of time,
// tick context, and randomness is this explicit envelope, handed to the policy. The
// same `(view, env)` always yields the same proposals.
//
// The RNG is a deterministic splitmix64 stream seeded from the envelope. It uses
// BigInt arithmetic (no Math.random, no Date) so the sandbox lint rules hold here
// too.
const MASK64 = (1n << 64n) - 1n;
const GAMMA = 0x9e3779b97f4a7c15n;
/**
 * A deterministic splitmix64 random stream. Construct it from a seed; the sequence
 * it produces depends only on that seed, so two streams with the same seed are
 * identical. It carries its own cursor — it never reads ambient state.
 */
export class DeterministicRng {
    #state;
    constructor(seed) {
        if (!Number.isInteger(seed) || seed < 0) {
            throw new RangeError('DeterministicRng seed must be a non-negative integer');
        }
        this.#state = BigInt(seed) & MASK64;
    }
    /** The next 64-bit value in the stream, advancing the cursor. */
    #nextU64() {
        this.#state = (this.#state + GAMMA) & MASK64;
        let z = this.#state;
        z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & MASK64;
        z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & MASK64;
        z = z ^ (z >> 31n);
        return z & MASK64;
    }
    /** The next unsigned 32-bit integer. */
    nextU32() {
        return Number(this.#nextU64() >> 32n);
    }
    /** The next float in `[0, 1)`. */
    nextFloat() {
        return this.nextU32() / 0x1_0000_0000;
    }
    /** A deterministic integer in `[min, max)` (`min` when the range is empty). */
    nextInRange(min, max) {
        if (max <= min) {
            return min;
        }
        return min + (this.nextU32() % (max - min));
    }
}
/** Construct a fresh deterministic envelope for a tick + seed. */
export function makeEnv(tick, seed) {
    return { tick, seed, rng: new DeterministicRng(seed) };
}
//# sourceMappingURL=env.js.map