import { test } from 'node:test';
import assert from 'node:assert/strict';
import { COSMETIC_NON_AUTHORITY_READOUT, createHitSparkDescriptor, createScreenFlashDescriptor, projectCosmeticFrame, readCosmeticAuthorityBoundary, validateCosmeticEffectDescriptor, } from './index.js';
const EMPTY_RENDER_FRAME = { ops: [] };
void test('screen flash descriptor consumes generated render frame descriptors only', () => {
    const descriptor = createScreenFlashDescriptor({
        effectId: 'screen-flash/hit-confirm',
        renderFrame: EMPTY_RENDER_FRAME,
        startsAtTick: 4,
        durationTicks: 8,
        intensity: 0.75,
    });
    assert.equal(descriptor.kind, 'screen_flash');
    assert.equal(descriptor.replayScope, 'excluded_from_replay_truth');
    assert.deepEqual(descriptor.source, {
        kind: 'render_frame_diff',
        renderOpCount: 0,
        renderOpKinds: [],
    });
    assert.deepEqual(validateCosmeticEffectDescriptor(descriptor), []);
});
void test('cosmetic frame projection is deterministic and sorted by tick then id', () => {
    const later = createHitSparkDescriptor({
        effectId: 'spark/b',
        sourceEventId: 'ui/fire-2',
        startsAtTick: 6,
        durationTicks: 4,
        intensity: 1,
        anchor: [2, 0, 1],
    });
    const earlier = createHitSparkDescriptor({
        effectId: 'spark/a',
        sourceEventId: 'ui/fire-1',
        startsAtTick: 4,
        durationTicks: 8,
        intensity: 0.5,
        anchor: [1, 0, 1],
    });
    const frame = projectCosmeticFrame([later, earlier], 6);
    assert.equal(frame.kind, 'cosmetic_frame_view_model.v0');
    assert.deepEqual(frame.effects.map((effect) => effect.effectId), ['spark/a', 'spark/b']);
    assert.deepEqual(frame.effects.map((effect) => ({
        active: effect.active,
        progress: effect.progress,
        opacity: effect.opacity,
    })), [
        { active: true, progress: 0.25, opacity: 0.375 },
        { active: true, progress: 0, opacity: 1 },
    ]);
    assert.deepEqual(frame.diagnostics, []);
    assert.equal(frame.nonAuthority, COSMETIC_NON_AUTHORITY_READOUT);
});
void test('invalid cosmetic descriptors fail closed with diagnostics and no active view model', () => {
    const invalid = {
        ...createScreenFlashDescriptor({
            effectId: '',
            renderFrame: EMPTY_RENDER_FRAME,
            startsAtTick: 0,
            durationTicks: 1,
            intensity: 1,
        }),
        durationTicks: 0,
        startsAtTick: -1,
        intensity: 2,
    };
    const frame = projectCosmeticFrame([invalid], 0);
    const codes = frame.diagnostics.map((diagnostic) => diagnostic.code);
    assert.deepEqual(frame.effects, []);
    assert.deepEqual(codes, ['missingEffectId', 'invalidStartTick', 'invalidDuration', 'invalidIntensity']);
});
void test('cosmetic boundary does not expose authority commands or replay records', () => {
    const frame = projectCosmeticFrame([
        createScreenFlashDescriptor({
            effectId: 'screen-flash/no-authority',
            renderFrame: EMPTY_RENDER_FRAME,
            startsAtTick: 1,
            durationTicks: 4,
            intensity: 0.25,
        }),
    ], 2);
    const boundary = readCosmeticAuthorityBoundary();
    assert.deepEqual(boundary.doesNotProduce, [
        'authority_commands',
        'replay_records',
        'state_mutations',
        'renderer_backend_calls',
    ]);
    assert.deepEqual(frame.nonAuthority, {
        kind: 'cosmetic_non_authority_readout.v0',
        commandCount: 0,
        replayRecordCount: 0,
        authoritativeMutationCount: 0,
        rendererBackendCoupling: false,
        runtimeTruth: 'not_authoritative',
    });
    assert.equal('commands' in frame, false);
    assert.equal('replayRecords' in frame, false);
});
//# sourceMappingURL=effects.test.js.map