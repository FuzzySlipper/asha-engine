// Perf-harness structure + invariant tests (#2460).
//
// These assert the perf record's SHAPE and structural invariants — never timing
// values (those are same-host trend data, not a correctness gate). Timing is driven
// by an injected monotonic clock so phase durations are deterministic here.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { RuntimeBridgeError } from '@asha/runtime-bridge';
import { defaultBootBridge } from './harness.js';
import { DEFAULT_EDIT_CYCLES, formatPerf, PERF_COMMAND, runPerf } from './perf.js';
/** A clock that advances 1ms per read, so each timed phase reports exactly 1ms. */
function steppedClock() {
    let t = 0;
    return () => t++;
}
const EXPECTED_PHASES = [
    'initialize',
    'world-load',
    'render-projection-initial',
    'renderer-apply-initial',
    'edit-one-cell',
    'edit-region',
    'edit-inverse',
    'render-update',
    'preview-overlay',
    'save',
    'reload',
    'replay',
    'edit-render-cycles',
];
void test('reference perf run is structurally sound and all invariants hold', async () => {
    const result = await runPerf({
        mode: 'reference',
        editCycles: 4,
        clock: steppedClock(),
        meta: { commit: 'abc1234', branch: 'task-test', hostLabel: 'ci-host' },
    });
    assert.equal(result.ok, true);
    assert.ok(result.invariants.every((i) => i.held), `all invariants held: ${JSON.stringify(result.invariants)}`);
    // Metadata is comparable + host-anchored.
    assert.equal(result.meta.schema, 1);
    assert.equal(result.meta.command, PERF_COMMAND);
    assert.equal(result.meta.runtimeMode, 'mock');
    assert.equal(result.meta.smokeMode, 'reference');
    assert.equal(result.meta.commit, 'abc1234');
    assert.equal(result.meta.branch, 'task-test');
    assert.equal(result.meta.hostLabel, 'ci-host');
    assert.equal(result.meta.fixtureId, 1001);
    assert.match(result.meta.fixtureWorldHash, /^[0-9a-f]{16}$/);
    assert.ok(result.meta.cpus >= 1);
    // Every expected phase is present, in order, each a finite non-negative duration.
    assert.deepEqual(result.timings.map((t) => t.phase), EXPECTED_PHASES);
    for (const t of result.timings) {
        assert.ok(Number.isFinite(t.ms) && t.ms >= 0, `${t.phase} ms finite`);
    }
    // The injected stepped clock makes each single-shot phase exactly 1ms.
    assert.equal(result.timings.find((t) => t.phase === 'initialize')?.ms, 1);
    assert.equal(result.timings.find((t) => t.phase === 'edit-render-cycles')?.iterations, 4);
});
void test('counters reflect a leak-free run with all commands accepted', async () => {
    const result = await runPerf({ mode: 'reference', editCycles: 5, clock: steppedClock() });
    const c = result.counters;
    assert.equal(c.leakedHandles, 0);
    assert.equal(c.outstandingBuffers, 0);
    // one-cell + region + inverse + 5 cycles, all accepted by the deterministic facade.
    assert.equal(c.commandsAccepted, 3 + 5);
    assert.equal(c.commandsRejected, 0);
    assert.equal(c.editCycles, 5);
    assert.equal(c.replayDiverged, false);
    assert.ok(c.peakHandles >= 1);
    assert.ok(c.renderOpsApplied >= 1);
});
void test('edit-cycle count is configurable and folded into the aggregate timing', async () => {
    const result = await runPerf({ mode: 'reference', editCycles: 2, clock: steppedClock() });
    const cycles = result.timings.find((t) => t.phase === 'edit-render-cycles');
    assert.equal(cycles?.iterations, 2);
    assert.equal(result.counters.editCycles, 2);
});
void test('default edit-cycle count is used when unspecified', async () => {
    const result = await runPerf({ mode: 'reference', clock: steppedClock() });
    assert.equal(result.counters.editCycles, DEFAULT_EDIT_CYCLES);
});
void test('a boot that fails closed is an honest structural failure, not a faked pass', async () => {
    const failedBoot = () => ({
        bridge: null,
        mode: 'native',
        intent: 'authority',
        nativeAvailable: false,
        bootError: new RuntimeBridgeError('native_unavailable', 'addon not built'),
    });
    const result = await runPerf({ mode: 'authority', bootBridge: failedBoot, clock: steppedClock() });
    assert.equal(result.ok, false);
    assert.equal(result.timings.length, 0);
    const boot = result.invariants.find((i) => i.name === 'bridge-boot');
    assert.equal(boot?.held, false);
    assert.match(boot?.detail ?? '', /addon not built/);
});
void test('formatPerf renders a readable summary with status, timings, and invariants', async () => {
    const result = await runPerf({ mode: 'reference', editCycles: 2, clock: steppedClock() });
    const text = formatPerf(result);
    assert.match(text, /asha-perf OK/);
    assert.match(text, /timings \(ms\):/);
    assert.match(text, /invariants:/);
    assert.match(text, /no-handle-leak/);
});
void test('default boot bridge produces a reference run (smoke of the production wiring)', async () => {
    const result = await runPerf({ bootBridge: defaultBootBridge, editCycles: 1, clock: steppedClock() });
    assert.equal(result.meta.runtimeMode, 'mock');
    assert.equal(result.ok, true);
});
//# sourceMappingURL=perf.test.js.map