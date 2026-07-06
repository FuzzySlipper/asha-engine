import { loadNativeAddon, NativeAddonUnavailable } from '@asha/native-bridge';
import { MANIFEST_OPERATIONS } from './generated/operations.js';
import { RuntimeBridgeError, nonNegativeSafeInteger, u32, } from './bridge.js';
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
    'load_world_bundle',
    'submit_commands',
    'step_simulation',
    'apply_enemy_direct_nav_movement',
    'read_render_diffs',
    'save_current_world',
    'get_composition_status',
]);
function nativeUnimplemented(manifestName) {
    return new RuntimeBridgeError('operation_unimplemented', `native bridge operation '${manifestName}' is not wired; the native facade is ` +
        `fail-closed (no mock fallback). Wire its #[napi] export and add it to ` +
        `NATIVE_WIRED_OPERATIONS.`);
}
const RUST_ERROR_KIND = {
    NotInitialized: 'not_initialized',
    InvalidInput: 'invalid_input',
    UnknownHandle: 'unknown_handle',
    BufferExpired: 'buffer_expired',
    Internal: 'internal',
};
function classifyNativeAddonError(cause) {
    if (cause instanceof RuntimeBridgeError)
        return cause;
    const message = cause instanceof Error ? cause.message : String(cause);
    const match = /^(\w+):\s*(.*)$/u.exec(message);
    if (match?.[1]) {
        const kind = RUST_ERROR_KIND[match[1]];
        if (kind)
            return new RuntimeBridgeError(kind, match[2] || message);
    }
    return new RuntimeBridgeError('internal', message);
}
function callNative(body) {
    try {
        return body();
    }
    catch (cause) {
        throw classifyNativeAddonError(cause);
    }
}
function nativeVec3(value, field) {
    if (value.length !== 3 || value.some((component) => !Number.isFinite(component))) {
        throw new RuntimeBridgeError('invalid_input', `${field} must be a finite vec3`);
    }
    return { x: value[0], y: value[1], z: value[2] };
}
function bridgeVec3(value, field) {
    if (!Number.isFinite(value.x) || !Number.isFinite(value.y) || !Number.isFinite(value.z)) {
        throw new RuntimeBridgeError('internal', `native ${field} was not a finite vec3`);
    }
    return [value.x, value.y, value.z];
}
function nativeAuthoritySource(value) {
    if (value === 'seeded_from_request' || value === 'rust_entity_store') {
        return value;
    }
    throw new RuntimeBridgeError('internal', `unknown native enemy movement authority source '${value}'`);
}
export class NativeRuntimeBridge {
    #addon;
    #seed = 0;
    #initialized = false;
    #engineHandle = null;
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
        this.#engineHandle = handle;
        this.#initialized = true;
        return handle;
    }
    #requireHandle(operation) {
        if (!this.#initialized || this.#engineHandle === null) {
            throw new RuntimeBridgeError('not_initialized', `${operation} before initializeEngine`);
        }
        return this.#engineHandle;
    }
    loadWorldBundle(request) {
        const handle = this.#requireHandle('loadWorldBundle');
        const bundleSchemaVersion = u32(request.bundleSchemaVersion, 'bundleSchemaVersion');
        const protocolVersion = u32(request.protocolVersion, 'protocolVersion');
        const sceneId = nonNegativeSafeInteger(request.sceneId, 'sceneId');
        return callNative(() => this.#addon.loadWorldBundle(handle, bundleSchemaVersion, protocolVersion, sceneId));
    }
    submitCommands(batch) {
        const handle = this.#requireHandle('submitCommands');
        return callNative(() => this.#addon.submitCommands(handle, JSON.stringify(batch.commands)));
    }
    stepSimulation(input) {
        const handle = this.#requireHandle('stepSimulation');
        const tick = nonNegativeSafeInteger(input.tick, 'tick');
        const diffCount = callNative(() => this.#addon.stepSimulation(handle, tick));
        return { tick, diffCount };
    }
    applyEnemyDirectNavMovement(request) {
        const handle = this.#requireHandle('applyEnemyDirectNavMovement');
        const entity = nonNegativeSafeInteger(request.entity, 'entity');
        if (entity === 0) {
            throw new RuntimeBridgeError('invalid_input', 'entity must be positive');
        }
        const seedPosition = nativeVec3(request.seedPosition, 'seedPosition');
        const target = nativeVec3(request.target, 'target');
        if (!Number.isFinite(request.maxStepUnits) || request.maxStepUnits <= 0) {
            throw new RuntimeBridgeError('invalid_input', 'maxStepUnits must be finite and positive');
        }
        const result = callNative(() => this.#addon.applyEnemyDirectNavMovement(handle, entity, seedPosition, target, request.maxStepUnits));
        return {
            entity: result.entity,
            authoritySource: nativeAuthoritySource(result.authoritySource),
            authorityTransport: 'native_rust',
            from: bridgeVec3(result.from, 'from'),
            target: bridgeVec3(result.target, 'target'),
            nextWaypoint: bridgeVec3(result.nextWaypoint, 'nextWaypoint'),
            distanceUnits: result.distanceUnits,
            reached: result.reached,
            pathHash: result.pathHash,
            transformHash: result.transformHash,
            projectionChanged: result.projectionChanged,
        };
    }
    readModelMaterialPreview(request) {
        void request;
        throw nativeUnimplemented('read_model_material_preview');
    }
    readSceneObjectSnapshot() {
        throw nativeUnimplemented('read_scene_object_snapshot');
    }
    applySceneObjectCommand() {
        throw nativeUnimplemented('apply_scene_object_command');
    }
    readRenderDiffs(cursor) {
        const handle = this.#requireHandle('readRenderDiffs');
        const frame = nonNegativeSafeInteger(cursor, 'frame cursor');
        return callNative(() => this.#addon.readRenderDiffs(handle, frame));
    }
    saveCurrentWorld() {
        const handle = this.#requireHandle('saveCurrentWorld');
        return callNative(() => this.#addon.saveCurrentWorld(handle));
    }
    getCompositionStatus() {
        const handle = this.#requireHandle('getCompositionStatus');
        return callNative(() => this.#addon.getCompositionStatus(handle));
    }
    // ── Unwired operations: fail-closed, never mock-backed ─────────────────────
    // Replace each body with its real native call (and add the manifest name to
    // NATIVE_WIRED_OPERATIONS) when the codegen emitter wires the `#[napi]` export.
    pickVoxel() {
        throw nativeUnimplemented('pick_voxel');
    }
    applyCollisionConstrainedCameraInput() {
        throw nativeUnimplemented('apply_collision_constrained_camera_input');
    }
    selectVoxel() {
        throw nativeUnimplemented('select_voxel');
    }
    readVoxelMeshEvidence() {
        throw nativeUnimplemented('read_voxel_mesh_evidence');
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
//# sourceMappingURL=native.js.map