import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { animationProjectionHandle, renderHandle, } from '@asha/contracts';
import { ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST, AshaAnimationHost, applyAshaRuntimeProjectionFrame, createAshaRendererAnimatedMeshProjection, } from './index.js';
function sceneFrame() {
    const resource = ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST.resources[0];
    assert.ok(resource);
    return {
        ops: [
            {
                op: 'defineAnimatedMesh',
                asset: {
                    asset: resource.asset,
                    runtimeFormat: 'glb',
                    contentHash: resource.contentHash,
                    clips: [
                        { id: 'idle', name: 'Idle', durationSeconds: 1.04166662693024 },
                        { id: 'run', name: 'Run', durationSeconds: 0.666666686534882 },
                        { id: 'jump', name: 'Jump', durationSeconds: 0.5 },
                    ],
                    defaultClip: 'idle',
                    materialSlots: [],
                    bounds: { min: [-0.02, -0.01, 0], max: [0.02, 0.01, 0.04] },
                },
            },
            {
                op: 'createAnimatedMeshInstance',
                handle: renderHandle(4100),
                parent: null,
                instance: {
                    asset: resource.asset,
                    transform: { translation: [0, 0, -2.5], rotation: [0, 0, 0, 1], scale: [40, 40, 40] },
                    materialOverrides: [],
                    playback: null,
                    metadata: { source: null, tags: [], label: 'controller target' },
                },
            },
        ],
    };
}
function controller(revision, elapsedTicks, targetClip = 'run') {
    return {
        graphId: 'player',
        graphVersion: 1,
        graphHash: 'fnv1a64:graph',
        stateId: 'idle',
        revision,
        stateHash: `fnv1a64:state-${revision}-${String(elapsedTicks)}`,
        motion: {
            clipA: 'idle',
            clipB: null,
            blendWeightMilli: 0,
            speedMilli: 1_000,
        },
        transition: elapsedTicks === null ? null : {
            transitionId: 'idle.move',
            fromStateId: 'idle',
            toStateId: 'locomotion',
            elapsedTicks,
            durationTicks: 2,
            targetMotion: {
                clipA: targetClip,
                clipB: null,
                blendWeightMilli: 0,
                speedMilli: 1_000,
            },
        },
        timingFact: null,
    };
}
function createFrame() {
    const resource = ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST.resources[0];
    assert.ok(resource);
    return {
        replayScope: 'excludedFromReplayTruth',
        ops: [{
                domain: 'animation',
                meta: {
                    sequence: 0,
                    origin: {
                        kind: 'capabilityState',
                        id: 'animation:player:0',
                        authorityTick: 0,
                        causationId: 'input:move',
                        correlationId: 'actor:player',
                    },
                },
                op: {
                    op: 'create',
                    handle: animationProjectionHandle(1),
                    descriptor: {
                        target: renderHandle(4100),
                        asset: resource.asset,
                        tickDurationMillis: 50,
                        controller: controller(0, null),
                    },
                },
            }],
    };
}
function updateFrame(targetClip = 'run') {
    return {
        replayScope: 'excludedFromReplayTruth',
        ops: [{
                domain: 'animation',
                meta: { sequence: 1, origin: null },
                op: {
                    op: 'update',
                    handle: animationProjectionHandle(1),
                    // FSM revision remains zero while fixed-tick transition progress moves.
                    controller: controller(0, 1, targetClip),
                },
            }],
    };
}
function fixtureResolver() {
    const descriptor = ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST.resources[0];
    assert.ok(descriptor);
    const bytes = readFileSync(fileURLToPath(descriptor.resourceUrl));
    return Promise.resolve(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength));
}
void test('G1 controller sequence drives deterministic renderer-local blend and smooth sampling', async () => {
    const testGlobal = globalThis;
    const priorSelf = testGlobal.self;
    testGlobal.self = globalThis;
    const priorWarn = console.warn;
    const priorError = console.error;
    console.warn = () => undefined;
    console.error = () => undefined;
    try {
        const projection = await createAshaRendererAnimatedMeshProjection({
            manifest: ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST,
            resolveResource: fixtureResolver,
        });
        assert.equal(projection.applyFrame(sceneFrame()).applied, true);
        const resource = ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST.resources[0];
        assert.ok(resource);
        const host = new AshaAnimationHost(projection, {
            cues: [{
                    cueId: 'locomotion.footfall',
                    asset: resource.asset,
                    clip: 'run',
                    atSeconds: 0.04,
                    signal: { domain: 'particle', id: 'locomotion.footfall.spark' },
                }],
        });
        assert.equal(host.applyPresentation(createFrame()).applied, 1);
        assert.deepEqual(projection.playback(renderHandle(4100)).controllerClips, [
            { clip: 'idle', weight: 1, speed: 1 },
        ]);
        assert.equal(host.applyPresentation(updateFrame()).applied, 1);
        const firstSample = host.advance(0.025);
        assert.deepEqual(firstSample.cues, []);
        assert.deepEqual(projection.playback(renderHandle(4100)).controllerClips, [
            { clip: 'idle', weight: 0.75, speed: 1 },
            { clip: 'run', weight: 0.25, speed: 1 },
        ]);
        const halfwayPose = projection.playback(renderHandle(4100)).poseSample;
        const cueSample = host.advance(0.025);
        assert.deepEqual(projection.playback(renderHandle(4100)).controllerClips, [
            { clip: 'idle', weight: 0.5, speed: 1 },
            { clip: 'run', weight: 0.5, speed: 1 },
        ]);
        assert.notDeepEqual(projection.playback(renderHandle(4100)).poseSample?.hierarchyRotationSum, halfwayPose?.hierarchyRotationSum);
        assert.deepEqual(cueSample.cues, [{
                kind: 'asha.animation.sampled_cue.v1',
                cueId: 'locomotion.footfall',
                handle: animationProjectionHandle(1),
                target: renderHandle(4100),
                asset: resource.asset,
                clip: 'run',
                markerSeconds: 0.04,
                sampledAtSeconds: 0.05,
                signal: { domain: 'particle', id: 'locomotion.footfall.spark' },
                origin: createFrame().ops[0]?.meta.origin ?? null,
                replayScope: 'excludedFromReplayTruth',
                authorityMutation: 'forbidden',
            }]);
        assert.deepEqual(host.advance(0.025).cues, []);
        assert.equal(host.readout().sampledFrames, 3);
        const cleanup = host.cleanup();
        assert.equal(cleanup.applied, 1);
        assert.equal(cleanup.readout.activeControllers, 0);
        assert.equal(projection.playback(renderHandle(4100)).status, 'stopped');
        const rebuilt = new AshaAnimationHost(projection);
        assert.equal(rebuilt.applyPresentation(createFrame()).applied, 1);
        assert.equal(rebuilt.applyPresentation(updateFrame()).applied, 1);
        const destroyed = rebuilt.applyPresentation({
            replayScope: 'excludedFromReplayTruth',
            ops: [{
                    domain: 'animation',
                    meta: { sequence: 2, origin: null },
                    op: { op: 'destroy', handle: animationProjectionHandle(1) },
                }],
        });
        assert.equal(destroyed.applied, 1);
        assert.equal(destroyed.readout.activeControllers, 0);
        assert.equal(projection.playback(renderHandle(4100)).status, 'stopped');
    }
    finally {
        console.warn = priorWarn;
        console.error = priorError;
        testGlobal.self = priorSelf;
    }
});
void test('animation host isolates missing targets and clips with origin-preserving diagnostics', async () => {
    const testGlobal = globalThis;
    const priorSelf = testGlobal.self;
    testGlobal.self = globalThis;
    try {
        const projection = await createAshaRendererAnimatedMeshProjection({
            manifest: ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST,
            resolveResource: fixtureResolver,
        });
        projection.applyFrame(sceneFrame());
        const host = new AshaAnimationHost(projection);
        host.applyPresentation(createFrame());
        const missingClip = host.applyPresentation(updateFrame('missing'));
        assert.equal(missingClip.applied, 0);
        assert.equal(missingClip.diagnostics[0]?.code, 'clipMissing');
        const missingTarget = createFrame();
        const operation = missingTarget.ops[0];
        assert.ok(operation?.domain === 'animation' && operation.op.op === 'create');
        const otherHost = new AshaAnimationHost(projection);
        const receipt = otherHost.applyPresentation({
            ...missingTarget,
            ops: [{
                    ...operation,
                    op: {
                        ...operation.op,
                        descriptor: { ...operation.op.descriptor, target: renderHandle(999) },
                    },
                }],
        });
        assert.equal(receipt.diagnostics[0]?.code, 'unknownTarget');
        assert.equal(receipt.diagnostics[0]?.origin?.correlationId, 'actor:player');
    }
    finally {
        testGlobal.self = priorSelf;
    }
});
void test('missing animation host degrades after scene application without authority callbacks', async () => {
    let sceneApplied = false;
    const receipt = await applyAshaRuntimeProjectionFrame({
        schemaVersion: 1,
        authorityTick: 9,
        scene: { ops: [] },
        presentation: createFrame(),
    }, { applyScene: () => { sceneApplied = true; } });
    assert.equal(sceneApplied, true);
    assert.equal(receipt.animation.applied, 0);
    assert.equal(receipt.animation.diagnostics[0]?.code, 'unavailableHost');
    assert.equal(receipt.animation.diagnostics[0]?.origin?.causationId, 'input:move');
    assert.equal('callback' in receipt.animation.readout, false);
});
//# sourceMappingURL=animation-host.test.js.map