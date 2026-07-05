import { test } from 'node:test';
import assert from 'node:assert/strict';
import { DeterministicRng, makeEnv } from './env.js';
void test('DeterministicRng is reproducible for a given seed', () => {
    const a = new DeterministicRng(42);
    const b = new DeterministicRng(42);
    const seqA = [a.nextU32(), a.nextU32(), a.nextU32()];
    const seqB = [b.nextU32(), b.nextU32(), b.nextU32()];
    assert.deepEqual(seqA, seqB);
});
void test('different seeds produce different streams', () => {
    const a = new DeterministicRng(1);
    const b = new DeterministicRng(2);
    assert.notDeepEqual([a.nextU32(), a.nextU32()], [b.nextU32(), b.nextU32()]);
});
void test('nextFloat is in [0, 1) and nextInRange respects bounds', () => {
    const rng = new DeterministicRng(7);
    for (let i = 0; i < 100; i++) {
        const f = rng.nextFloat();
        assert.ok(f >= 0 && f < 1);
        const n = rng.nextInRange(10, 20);
        assert.ok(n >= 10 && n < 20);
    }
    // Empty range returns the min.
    assert.equal(rng.nextInRange(5, 5), 5);
});
void test('makeEnv carries tick, seed, and a seeded stream', () => {
    const env = makeEnv(3, 99);
    assert.equal(env.tick, 3);
    assert.equal(env.seed, 99);
    // The env stream matches a fresh stream from the same seed.
    assert.equal(env.rng.nextU32(), new DeterministicRng(99).nextU32());
});
void test('a negative or non-integer seed is rejected, never silently coerced', () => {
    assert.throws(() => new DeterministicRng(-1), RangeError);
    assert.throws(() => new DeterministicRng(1.5), RangeError);
});
//# sourceMappingURL=env.test.js.map