// @asha/runtime-bridge — the public, transport-agnostic runtime facade (ADR 0006).
//
// App / UI / renderer / devtools couple ONLY to this package for runtime. They do
// not know whether the implementation is napi-rs (`@asha/native-bridge`), a mock,
// or the WASM replay path. The facade exports generated-compatible contract types
// and explicit buffer-handle APIs — never raw addon exports, WASM memory, or JSON
// escape hatches.
//
// The public facade is hand-written for readability but MUST satisfy the
// manifest-derived conformance test (see conformance.test.ts).
import { loadNativeAddon, NativeAddonUnavailable } from '@asha/native-bridge';
import { MANIFEST_OPERATIONS } from './generated/operations.js';
export { MANIFEST_OPERATIONS } from './generated/operations.js';
// Render-diff decode (moved from the former @asha/wasm-bridge). Transport-neutral
// payload → contract types; backs `readRenderDiffs`. See render-decode.ts.
export { decodeRenderDiff, decodeRenderFrameDiff, RenderDecodeError, RenderDiffStream, FrameMemory, } from './render-decode.js';
export const frameCursor = (frame) => frame;
/** Typed, classified error for every facade operation. No JSON error blobs. */
export class RuntimeBridgeError extends Error {
    kind;
    constructor(kind, message) {
        super(`runtime bridge error [${kind}]: ${message}`);
        this.kind = kind;
        this.name = 'RuntimeBridgeError';
    }
}
function finite(value, field) {
    if (!Number.isFinite(value)) {
        throw new RuntimeBridgeError('invalid_input', `${field} must be finite`);
    }
    return value;
}
function validateViewport(viewport) {
    if (!Number.isInteger(viewport.width) || viewport.width <= 0) {
        throw new RuntimeBridgeError('invalid_input', 'viewport width must be a positive integer');
    }
    if (!Number.isInteger(viewport.height) || viewport.height <= 0) {
        throw new RuntimeBridgeError('invalid_input', 'viewport height must be a positive integer');
    }
}
function validateProjection(projection) {
    finite(projection.fovYDegrees, 'fovYDegrees');
    finite(projection.near, 'near');
    finite(projection.far, 'far');
    if (projection.fovYDegrees <= 0 || projection.fovYDegrees >= 180) {
        throw new RuntimeBridgeError('invalid_input', 'fovYDegrees must be in (0, 180)');
    }
    if (projection.near <= 0 || projection.far <= projection.near) {
        throw new RuntimeBridgeError('invalid_input', 'projection near/far must satisfy 0 < near < far');
    }
}
function basisFromPose(pose) {
    const yaw = (pose.yawDegrees * Math.PI) / 180;
    const pitch = (pose.pitchDegrees * Math.PI) / 180;
    const cp = Math.cos(pitch);
    const sp = Math.sin(pitch);
    const sy = Math.sin(yaw);
    const cy = Math.cos(yaw);
    return {
        forward: [sy * cp, sp, -cy * cp],
        right: [cy, 0, sy],
        up: [-sy * sp, cp, cy * sp],
    };
}
function projectionSnapshot(snapshot, viewport = snapshot.viewport) {
    return {
        ...snapshot,
        viewport,
        // Placeholder deterministic matrices for #2564 facade coverage. #2565 owns
        // the Rust/reference golden math and will replace this with strict evidence.
        viewMatrix: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, -snapshot.pose.position[0], -snapshot.pose.position[1], -snapshot.pose.position[2], 1],
        projectionMatrix: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, -1, -1, 0, 0, -snapshot.projection.near * 2, 0],
        viewProjectionMatrix: [1, 0, 0, 0, 0, 1, 0, 0, 0, -snapshot.pose.position[1], -1, -1, -snapshot.pose.position[0], 0, -snapshot.projection.near * 2, 0],
        projectionHash: `sha256:mock-camera-${snapshot.camera}-${snapshot.tick}`,
    };
}
export class MockRuntimeBridge {
    #engine = null;
    #buffer = new Uint8Array();
    #replaySteps = 0;
    #loadedWorld = null;
    #nextCamera = 1;
    #cameras = new Map();
    initializeEngine(config) {
        if (!Number.isInteger(config.seed) || config.seed < 0) {
            throw new RuntimeBridgeError('invalid_input', `seed must be a non-negative integer`);
        }
        const handle = config.seed;
        this.#engine = handle;
        // Deterministic: little-endian seed bytes, mirroring ReferenceBridge.
        const bytes = new Uint8Array(8);
        new DataView(bytes.buffer).setBigUint64(0, BigInt(config.seed), true);
        this.#buffer = bytes;
        return handle;
    }
    stepSimulation(input) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'step before initializeEngine');
        }
        return { tick: input.tick, diffCount: input.tick % 4 };
    }
    submitCommands(batch) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'submitCommands before initializeEngine');
        }
        // The mock is a transport stand-in, NOT authority: it does not re-validate the
        // voxel edit (Rust `rule-voxel-edit` owns that, exercised on the native path).
        // It fail-closes on transport preconditions (init) and accepts well-typed
        // commands, returning the classified result shape with no rejections.
        return { accepted: batch.commands.length, rejected: 0, rejections: [] };
    }
    pickVoxel(ray) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'pickVoxel before initializeEngine');
        }
        // The mock hosts no authority voxel geometry (Rust `svc-collision` owns the
        // raycast on the native path), so a pick always classifies as a miss. It still
        // fail-closes on the transport precondition (init) and validates the ray shape.
        if (ray.direction.every((c) => c === 0)) {
            throw new RuntimeBridgeError('invalid_input', 'pick ray direction must be non-zero');
        }
        return { outcome: 'miss', rejection: { reason: 'noHit' } };
    }
    readRenderDiffs(cursor) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'readRenderDiffs before initializeEngine');
        }
        if (!Number.isInteger(cursor) || cursor < 0) {
            throw new RuntimeBridgeError('invalid_input', `frame cursor must be a non-negative integer`);
        }
        return { ops: [] };
    }
    createCamera(request) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'createCamera before initializeEngine');
        }
        validateProjection(request.projection);
        validateViewport(request.viewport);
        for (const [index, value] of request.initialPose.position.entries()) {
            finite(value, `initialPose.position[${index}]`);
        }
        finite(request.initialPose.yawDegrees, 'initialPose.yawDegrees');
        finite(request.initialPose.pitchDegrees, 'initialPose.pitchDegrees');
        const camera = this.#nextCamera++;
        const snapshot = {
            camera,
            tick: 0,
            pose: request.initialPose,
            basis: basisFromPose(request.initialPose),
            projection: request.projection,
            viewport: request.viewport,
        };
        this.#cameras.set(camera, snapshot);
        return snapshot;
    }
    applyFirstPersonCameraInput(envelope) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'applyFirstPersonCameraInput before initializeEngine');
        }
        const prior = this.#cameras.get(envelope.camera);
        if (!prior) {
            throw new RuntimeBridgeError('unknown_handle', `no camera for handle ${envelope.camera}`);
        }
        const i = envelope.input;
        finite(i.moveForward, 'moveForward');
        finite(i.moveRight, 'moveRight');
        finite(i.moveUp, 'moveUp');
        finite(i.yawDeltaDegrees, 'yawDeltaDegrees');
        finite(i.pitchDeltaDegrees, 'pitchDeltaDegrees');
        finite(i.dtSeconds, 'dtSeconds');
        finite(i.moveSpeedUnitsPerSecond, 'moveSpeedUnitsPerSecond');
        if (i.dtSeconds < 0 || i.moveSpeedUnitsPerSecond < 0) {
            throw new RuntimeBridgeError('invalid_input', 'dtSeconds and moveSpeedUnitsPerSecond must be non-negative');
        }
        const basis = prior.basis;
        const distance = i.dtSeconds * i.moveSpeedUnitsPerSecond;
        const position = [
            prior.pose.position[0] + (basis.forward[0] * i.moveForward + basis.right[0] * i.moveRight + basis.up[0] * i.moveUp) * distance,
            prior.pose.position[1] + (basis.forward[1] * i.moveForward + basis.right[1] * i.moveRight + basis.up[1] * i.moveUp) * distance,
            prior.pose.position[2] + (basis.forward[2] * i.moveForward + basis.right[2] * i.moveRight + basis.up[2] * i.moveUp) * distance,
        ];
        const pitchDegrees = Math.max(-89, Math.min(89, prior.pose.pitchDegrees + i.pitchDeltaDegrees));
        const pose = {
            position,
            yawDegrees: prior.pose.yawDegrees + i.yawDeltaDegrees,
            pitchDegrees,
        };
        const snapshot = {
            ...prior,
            tick: envelope.tick,
            pose,
            basis: basisFromPose(pose),
        };
        this.#cameras.set(envelope.camera, snapshot);
        return snapshot;
    }
    readCameraProjection(request) {
        if (this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', 'readCameraProjection before initializeEngine');
        }
        const snapshot = this.#cameras.get(request.camera);
        if (!snapshot) {
            throw new RuntimeBridgeError('unknown_handle', `no camera for handle ${request.camera}`);
        }
        if (request.viewport !== null)
            validateViewport(request.viewport);
        return projectionSnapshot(snapshot, request.viewport ?? snapshot.viewport);
    }
    getBuffer(handle) {
        if (handle !== 0) {
            throw new RuntimeBridgeError('unknown_handle', `no buffer for handle ${handle}`);
        }
        return { handle, bytes: this.#buffer };
    }
    releaseBuffer(handle) {
        if (handle !== 0) {
            throw new RuntimeBridgeError('unknown_handle', `no buffer for handle ${handle}`);
        }
        this.#buffer = new Uint8Array();
    }
    loadWorldBundle(request) {
        // Fail closed on a newer bundle; the prior loaded world is left untouched
        // (we only set #loadedWorld on success — the staged commit/swap).
        if (request.bundleSchemaVersion > 1 || request.protocolVersion > 1) {
            throw new RuntimeBridgeError('invalid_input', `unsupported bundle schema ${request.bundleSchemaVersion} / protocol ${request.protocolVersion}`);
        }
        this.#loadedWorld = request.sceneId;
        return { loadedWorld: request.sceneId, fatalCount: 0, totalCount: 0, blocksLoad: false };
    }
    saveCurrentWorld() {
        if (this.#loadedWorld === null) {
            throw new RuntimeBridgeError('not_initialized', 'saveCurrentWorld with no world loaded');
        }
        return { artifactsWritten: 3, compactedEdits: 0, retainedEdits: 0 };
    }
    getCompositionStatus() {
        return { loadedWorld: this.#loadedWorld, fatalCount: 0, totalCount: 0, blocksLoad: false };
    }
    unloadWorld() {
        this.#loadedWorld = null;
    }
    loadReplayFixture(fixture) {
        this.#replaySteps = fixture.steps;
        return 0;
    }
    runReplayStep(session) {
        const step = this.#replaySteps;
        this.#replaySteps = Math.max(0, this.#replaySteps - 1);
        return { step, hash: `mock-${session}-${step}`, diverged: false };
    }
}
/** Construct the default mock bridge. */
export function createMockRuntimeBridge() {
    return new MockRuntimeBridge();
}
// ── Native implementation factory ─────────────────────────────────────────────
// The ONLY place that touches `@asha/native-bridge`. Wraps the addon's wired
// exports and re-classifies load failures into the bridge error taxonomy.
//
// Fail-closed by construction: `NativeRuntimeBridge` implements `RuntimeBridge`
// directly — it does NOT extend `MockRuntimeBridge`, so an unwired operation can
// never silently inherit mock/reference behaviour. Every stable + quarantined
// operation is either routed to a real `#[napi]` export (and listed in
// NATIVE_WIRED_OPERATIONS) or throws a classified `operation_unimplemented`.
// `native-fail-closed.test.ts` enforces that this stays true for every manifest op.
/**
 * Manifest names of operations whose native (`#[napi]`) implementation is actually
 * wired. Everything else on {@link NativeRuntimeBridge} fail-closes with
 * `operation_unimplemented`. Adding a name here is the explicit signal that a
 * native implementation landed; the native conformance test keeps this set and the
 * routed methods in lockstep with the bridge manifest.
 */
export const NATIVE_WIRED_OPERATIONS = new Set([
    'initialize_engine',
    'step_simulation',
]);
function nativeUnimplemented(manifestName) {
    return new RuntimeBridgeError('operation_unimplemented', `native bridge operation '${manifestName}' is not wired; the native facade is ` +
        `fail-closed (no mock fallback). Wire its #[napi] export and add it to ` +
        `NATIVE_WIRED_OPERATIONS.`);
}
export class NativeRuntimeBridge {
    #addon;
    #seed = 0;
    #initialized = false;
    constructor(addon) {
        this.#addon = addon;
    }
    // ── Wired native operations ───────────────────────────────────────────────
    initializeEngine(config) {
        if (!Number.isInteger(config.seed) || config.seed < 0) {
            throw new RuntimeBridgeError('invalid_input', `seed must be a non-negative integer`);
        }
        this.#seed = config.seed;
        const handle = this.#addon.initializeEngine(config.seed);
        this.#initialized = true;
        return handle;
    }
    stepSimulation(input) {
        if (!this.#initialized) {
            throw new RuntimeBridgeError('not_initialized', 'step before initializeEngine');
        }
        const diffCount = this.#addon.stepSimulation(this.#seed, input.tick);
        return { tick: input.tick, diffCount };
    }
    // ── Unwired operations: fail-closed, never mock-backed ─────────────────────
    // Replace each body with its real native call (and add the manifest name to
    // NATIVE_WIRED_OPERATIONS) when the codegen emitter wires the `#[napi]` export.
    submitCommands() {
        throw nativeUnimplemented('submit_commands');
    }
    pickVoxel() {
        throw nativeUnimplemented('pick_voxel');
    }
    readRenderDiffs() {
        throw nativeUnimplemented('read_render_diffs');
    }
    createCamera() {
        throw nativeUnimplemented('create_camera');
    }
    applyFirstPersonCameraInput() {
        throw nativeUnimplemented('apply_first_person_camera_input');
    }
    readCameraProjection() {
        throw nativeUnimplemented('read_camera_projection');
    }
    getBuffer() {
        throw nativeUnimplemented('get_buffer');
    }
    releaseBuffer() {
        throw nativeUnimplemented('release_buffer');
    }
    loadWorldBundle() {
        throw nativeUnimplemented('load_world_bundle');
    }
    saveCurrentWorld() {
        throw nativeUnimplemented('save_current_world');
    }
    getCompositionStatus() {
        throw nativeUnimplemented('get_composition_status');
    }
    unloadWorld() {
        throw nativeUnimplemented('unload_world');
    }
    loadReplayFixture() {
        throw nativeUnimplemented('load_replay_fixture');
    }
    runReplayStep() {
        throw nativeUnimplemented('run_replay_step');
    }
}
/**
 * Construct the native (napi-rs) bridge. Throws a classified
 * {@link RuntimeBridgeError} of kind `native_unavailable` if the addon is not built
 * — callers can fall back to the mock for tests/dev.
 */
export function createNativeRuntimeBridge(modulePath) {
    try {
        const addon = modulePath ? loadNativeAddon(modulePath) : loadNativeAddon();
        return new NativeRuntimeBridge(addon);
    }
    catch (cause) {
        if (cause instanceof NativeAddonUnavailable) {
            throw new RuntimeBridgeError('native_unavailable', cause.message);
        }
        throw cause;
    }
}
/** Operation count for quick sanity in consumers/tests. */
export const STABLE_OPERATION_COUNT = MANIFEST_OPERATIONS.filter((o) => o.surface === 'stable').length;
//# sourceMappingURL=index.js.map