import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildPolicyRunSummary, formatPolicyRunSummary } from './policy-panel.js';
const eid = (n) => n;
const tid = (n) => n;
const accepted = (entity, label) => ({
    status: 'accepted',
    event: { kind: 'labelAdded', entity, label },
});
const rejected = (reason) => ({
    status: 'rejected',
    rejection: reason,
});
function run(over = {}) {
    return {
        tick: 7,
        executions: [
            { policyId: 'labelSpatialEntities', version: 1, proposedCount: 2, violationCount: 0 },
            { policyId: 'thresholdSignal', version: 2, proposedCount: 1, violationCount: 1 },
        ],
        outcomes: [accepted(eid(1), tid(9)), rejected('labelAlreadyPresent'), rejected('notSpatial')],
        replayHandle: 'tick-7-abcd',
        ...over,
    };
}
void test('summary counts proposed/accepted/rejected/violations and groups rejections', () => {
    const s = buildPolicyRunSummary(run());
    assert.equal(s.policyCount, 2);
    assert.equal(s.totalProposed, 3);
    assert.equal(s.accepted, 1);
    assert.equal(s.rejected, 2);
    assert.equal(s.violations, 1);
    assert.deepEqual(s.rejectionsByReason, [
        { reason: 'labelAlreadyPresent', count: 1 },
        { reason: 'notSpatial', count: 1 },
    ]);
    assert.equal(s.replayHandle, 'tick-7-abcd');
});
void test('summary carries each policy id + version row', () => {
    const s = buildPolicyRunSummary(run());
    assert.deepEqual(s.rows[0], { policyId: 'labelSpatialEntities', version: 1, proposedCount: 2, violationCount: 0 });
    assert.deepEqual(s.rows[1], { policyId: 'thresholdSignal', version: 2, proposedCount: 1, violationCount: 1 });
});
void test('build is deterministic — same input, same summary', () => {
    assert.deepEqual(buildPolicyRunSummary(run()), buildPolicyRunSummary(run()));
});
void test('formatPolicyRunSummary is deterministic and greppable', () => {
    const lines = formatPolicyRunSummary(buildPolicyRunSummary(run()));
    assert.ok(lines[0].includes('policyRun tick=7 policies=2 proposed=3 accepted=1 rejected=2'));
    assert.ok(lines.some((l) => l.includes('policy labelSpatialEntities v1')));
    assert.ok(lines.some((l) => l.includes('rejected notSpatial=1')));
});
void test('an all-accepted run reports zero rejections and a stable replay handle', () => {
    const s = buildPolicyRunSummary(run({ outcomes: [accepted(eid(1), tid(9)), accepted(eid(2), tid(9))], replayHandle: 'tick-7-clean' }));
    assert.equal(s.rejected, 0);
    assert.deepEqual(s.rejectionsByReason, []);
    assert.equal(s.replayHandle, 'tick-7-clean');
});
//# sourceMappingURL=policy-panel.test.js.map