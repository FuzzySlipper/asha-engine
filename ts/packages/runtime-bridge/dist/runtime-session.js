import { RuntimeBridgeError, frameCursor, } from './bridge.js';
import { createMockRuntimeBridge } from './mock.js';
export function createMockRuntimeSession(options = {}) {
    return new ReferenceRuntimeSessionFacade(options.bridge ?? createMockRuntimeBridge());
}
class ReferenceRuntimeSessionFacade {
    #bridge;
    #identity = null;
    #engine = null;
    #sequenceId = 0;
    #tick = 0;
    #acceptedCommandCount = 0;
    #rejectedCommandCount = 0;
    #restartCount = 0;
    #replayRecords = [];
    constructor(bridge) {
        this.#bridge = bridge;
    }
    initialize(input) {
        validateInitializeInput(input);
        const engine = this.#bridge.initializeEngine({ seed: input.seed });
        const composition = this.#bridge.loadWorldBundle(input.projectBundle);
        this.#engine = engine;
        this.#identity = {
            sessionId: input.sessionId,
            mode: 'reference',
            seed: input.seed,
            project: input.project,
            projectBundle: input.projectBundle,
            nonClaims: referenceRuntimeSessionNonClaims(),
        };
        this.#sequenceId = 0;
        this.#tick = 0;
        this.#acceptedCommandCount = 0;
        this.#rejectedCommandCount = 0;
        this.#replayRecords = [];
        this.#record('initialize');
        return this.#stateSummary(composition);
    }
    submitCommands(batch) {
        this.#requireInitialized('submitCommands');
        const before = this.#sessionHash();
        const result = this.#bridge.submitCommands(batch);
        this.#acceptedCommandCount += result.accepted;
        this.#rejectedCommandCount += result.rejected;
        this.#sequenceId += 1;
        this.#record('submitCommands');
        return {
            sequenceId: this.#sequenceId,
            batch,
            result,
            acceptedCommandCount: this.#acceptedCommandCount,
            rejectedCommandCount: this.#rejectedCommandCount,
            sessionHashBefore: before,
            sessionHashAfter: this.#sessionHash(),
        };
    }
    tick(input = {}) {
        this.#requireInitialized('tick');
        const nextTick = input.tick ?? this.#tick + 1;
        const step = this.#bridge.stepSimulation({ tick: nextTick });
        this.#tick = step.tick;
        this.#sequenceId += 1;
        this.#record('tick');
        return {
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            step,
            composition: this.#bridge.getCompositionStatus(),
            sessionHash: this.#sessionHash(),
        };
    }
    createCamera(request) {
        this.#requireInitialized('createCamera');
        const snapshot = this.#bridge.createCamera(request);
        this.#sequenceId += 1;
        this.#record('createCamera');
        return {
            sequenceId: this.#sequenceId,
            request,
            snapshot,
            sessionHash: this.#sessionHash(),
        };
    }
    applyFirstPersonCameraInput(envelope) {
        this.#requireInitialized('applyFirstPersonCameraInput');
        const before = this.#sessionHash();
        const snapshot = this.#bridge.applyFirstPersonCameraInput(envelope);
        this.#sequenceId += 1;
        this.#record('applyFirstPersonCameraInput');
        return {
            sequenceId: this.#sequenceId,
            envelope,
            snapshot,
            sessionHashBefore: before,
            sessionHashAfter: this.#sessionHash(),
        };
    }
    readCameraProjection(request) {
        this.#requireInitialized('readCameraProjection');
        const snapshot = this.#bridge.readCameraProjection(request);
        return {
            sequenceId: this.#sequenceId,
            request,
            snapshot,
            projectionHash: snapshot.projectionHash,
        };
    }
    readProjection() {
        this.#requireInitialized('readProjection');
        const cursor = frameCursor(this.#sequenceId);
        const frame = this.#bridge.readRenderDiffs(cursor);
        const composition = this.#bridge.getCompositionStatus();
        return {
            sequenceId: this.#sequenceId,
            cursor,
            frame,
            composition,
            renderDiffCount: frame.ops.length,
            projectionHash: stableHash({
                sequenceId: this.#sequenceId,
                composition: compositionHashRecord(composition),
                frame: renderFrameHashRecord(frame),
            }),
        };
    }
    readTelemetry() {
        this.#requireInitialized('readTelemetry');
        return {
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            composition: this.#bridge.getCompositionStatus(),
            acceptedCommandCount: this.#acceptedCommandCount,
            rejectedCommandCount: this.#rejectedCommandCount,
            restartCount: this.#restartCount,
            sessionHash: this.#sessionHash(),
            replayRecords: [...this.#replayRecords],
        };
    }
    restart() {
        const identity = this.#requireInitialized('restart');
        this.#bridge.unloadWorld();
        this.#bridge.initializeEngine({ seed: identity.seed });
        const composition = this.#bridge.loadWorldBundle(identity.projectBundle);
        this.#sequenceId += 1;
        this.#tick = 0;
        this.#acceptedCommandCount = 0;
        this.#rejectedCommandCount = 0;
        this.#restartCount += 1;
        this.#record('restart');
        return {
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            composition,
            restartCount: this.#restartCount,
            sessionHash: this.#sessionHash(),
        };
    }
    #requireInitialized(operation) {
        if (this.#identity === null || this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', `${operation} before RuntimeSession initialize`);
        }
        return this.#identity;
    }
    #stateSummary(composition) {
        const identity = this.#requireInitialized('stateSummary');
        return {
            identity,
            engine: this.#engine,
            composition,
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            sessionHash: this.#sessionHash(),
        };
    }
    #record(kind) {
        this.#replayRecords.push({
            sequenceId: this.#sequenceId,
            kind,
            recordHash: stableHash({
                kind,
                sequenceId: this.#sequenceId,
                tick: this.#tick,
                acceptedCommandCount: this.#acceptedCommandCount,
                rejectedCommandCount: this.#rejectedCommandCount,
                restartCount: this.#restartCount,
                composition: compositionHashRecord(this.#bridge.getCompositionStatus()),
            }),
        });
    }
    #sessionHash() {
        return stableHash({
            identity: this.#identity === null ? null : identityHashRecord(this.#identity),
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            acceptedCommandCount: this.#acceptedCommandCount,
            rejectedCommandCount: this.#rejectedCommandCount,
            restartCount: this.#restartCount,
            composition: this.#identity === null ? null : compositionHashRecord(this.#bridge.getCompositionStatus()),
        });
    }
}
function validateInitializeInput(input) {
    if (input.sessionId.trim().length === 0) {
        throw new RuntimeBridgeError('invalid_input', 'sessionId must be non-empty');
    }
    if (input.project.gameId.trim().length === 0) {
        throw new RuntimeBridgeError('invalid_input', 'project.gameId must be non-empty');
    }
    if (input.project.workspaceId.trim().length === 0) {
        throw new RuntimeBridgeError('invalid_input', 'project.workspaceId must be non-empty');
    }
    if (!Number.isSafeInteger(input.seed) || input.seed < 0) {
        throw new RuntimeBridgeError('invalid_input', 'seed must be a non-negative safe integer');
    }
}
function referenceRuntimeSessionNonClaims() {
    return [
        'not_native_runtime',
        'not_raw_state_store',
        'not_arbitrary_json_bridge',
        'not_gameplay_loop',
        'not_renderer',
    ];
}
function identityHashRecord(identity) {
    return {
        sessionId: identity.sessionId,
        mode: identity.mode,
        seed: identity.seed,
        project: {
            gameId: identity.project.gameId,
            workspaceId: identity.project.workspaceId,
        },
        projectBundle: projectBundleHashRecord(identity.projectBundle),
        nonClaims: identity.nonClaims,
    };
}
function projectBundleHashRecord(projectBundle) {
    return {
        bundleSchemaVersion: projectBundle.bundleSchemaVersion,
        protocolVersion: projectBundle.protocolVersion,
        sceneId: projectBundle.sceneId,
    };
}
function compositionHashRecord(composition) {
    return {
        loadedWorld: composition.loadedWorld,
        fatalCount: composition.fatalCount,
        totalCount: composition.totalCount,
        blocksLoad: composition.blocksLoad,
    };
}
function renderFrameHashRecord(frame) {
    return {
        opCount: frame.ops.length,
        opKinds: frame.ops.map((op) => op.op),
    };
}
function stableHash(value) {
    return `fnv1a64:${fnv1a64(stableStringify(value))}`;
}
function stableStringify(value) {
    if (value === undefined) {
        return 'undefined';
    }
    if (value === null || typeof value !== 'object') {
        return JSON.stringify(value);
    }
    if (Array.isArray(value)) {
        return `[${value.map((entry) => stableStringify(entry)).join(',')}]`;
    }
    const record = value;
    return `{${Object.keys(record)
        .sort()
        .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
        .join(',')}}`;
}
function fnv1a64(text) {
    let hash = 0xcbf29ce484222325n;
    const prime = 0x100000001b3n;
    const mask = 0xffffffffffffffffn;
    for (let index = 0; index < text.length; index += 1) {
        hash ^= BigInt(text.charCodeAt(index));
        hash = (hash * prime) & mask;
    }
    return hash.toString(16).padStart(16, '0');
}
//# sourceMappingURL=runtime-session.js.map