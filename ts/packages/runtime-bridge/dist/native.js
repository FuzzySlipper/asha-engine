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
    'load_project_bundle',
    'submit_commands',
    'step_simulation',
    'create_camera',
    'apply_collision_constrained_camera_input',
    'apply_generated_tunnel_to_runtime_world',
    'apply_enemy_direct_nav_movement',
    'load_fps_runtime_session',
    'read_fps_runtime_session',
    'apply_fps_primary_fire',
    'invoke_game_extension_weapon_effect',
    'validate_game_rule_catalog',
    'submit_game_rule_effect_intent',
    'read_game_rule_runtime_readout',
    'restart_fps_runtime_session',
    'read_fps_encounter_director',
    'apply_fps_encounter_transition',
    'plan_voxel_conversion',
    'register_voxel_conversion_source',
    'register_voxel_conversion_mesh_asset',
    'import_voxel_conversion_mesh_source',
    'read_voxel_conversion_source_metadata',
    'preview_voxel_conversion',
    'apply_voxel_conversion',
    'export_voxel_conversion_evidence',
    'read_voxel_model_info',
    'read_voxel_model_window',
    'export_voxel_volume_asset',
    'save_voxel_volume_asset',
    'update_voxel_volume_asset_palette',
    'initialize_voxel_volume_authoring',
    'load_voxel_volume_asset',
    'unload_voxel_volume_asset',
    'validate_voxel_annotation_layer',
    'load_voxel_annotation_layer',
    'read_voxel_annotation_query',
    'apply_voxel_annotation_edit',
    'export_voxel_annotation_layer',
    'read_voxel_edit_history',
    'preview_voxel_edit_revert',
    'apply_voxel_edit_revert',
    'undo_voxel_edit',
    'redo_voxel_edit',
    'read_render_diffs',
    'save_project_bundle',
    'get_project_bundle_composition_status',
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
function parseNativeJson(payload, field) {
    try {
        return JSON.parse(payload);
    }
    catch (cause) {
        const reason = cause instanceof Error ? cause.message : String(cause);
        throw new RuntimeBridgeError('internal', `native ${field} was not valid JSON: ${reason}`);
    }
}
function projectBundleCompositionStatusFromNative(status) {
    return {
        loadedProjectBundle: status.loadedProjectBundle,
        fatalCount: status.fatalCount,
        totalCount: status.totalCount,
        blocksLoad: status.blocksLoad,
    };
}
function nativeVec3(value, field) {
    if (value.length !== 3 || value.some((component) => !Number.isFinite(component))) {
        throw new RuntimeBridgeError('invalid_input', `${field} must be a finite vec3`);
    }
    return { x: value[0], y: value[1], z: value[2] };
}
function nativeOptionalObject(value) {
    return value == null ? undefined : value;
}
function requiredString(value, field) {
    if (typeof value !== 'string' || value.trim().length === 0) {
        throw new RuntimeBridgeError('invalid_input', `${field} must be a non-empty string`);
    }
    return value;
}
function requiredStringArray(value, field) {
    if (!isTypedArray(value)) {
        throw new RuntimeBridgeError('invalid_input', `${field} must be an array of non-empty strings`);
    }
    return value.map((entry, index) => requiredString(entry, `${field}[${index}]`));
}
function bridgeVec3(value, field) {
    if (!Number.isFinite(value.x) || !Number.isFinite(value.y) || !Number.isFinite(value.z)) {
        throw new RuntimeBridgeError('internal', `native ${field} was not a finite vec3`);
    }
    return [value.x, value.y, value.z];
}
function bridgeVec3Array(value, field) {
    if (value.length !== 3 || value.some((component) => !Number.isFinite(component))) {
        throw new RuntimeBridgeError('internal', 'native ' + field + ' was not a finite vec3');
    }
    return [value[0], value[1], value[2]];
}
function isTypedArray(value) {
    return Array.isArray(value);
}
function nativeAuthoritySource(value) {
    if (value === 'seeded_from_request' || value === 'rust_entity_store') {
        return value;
    }
    throw new RuntimeBridgeError('internal', `unknown native enemy movement authority source '${value}'`);
}
function fpsBackend(value) {
    if (value === 'native_rust' || value === 'reference_bridge') {
        return value;
    }
    // The Rust engine bridge reports engine_bridge_rust internally; the TS
    // native facade classifies the transport path as native_rust.
    if (value === 'engine_bridge_rust') {
        return 'native_rust';
    }
    throw new RuntimeBridgeError('internal', `unknown native FPS backend '${value}'`);
}
function fpsRole(value) {
    if (value === 'player' || value === 'enemy' || value === 'neutral')
        return value;
    throw new RuntimeBridgeError('invalid_input', `unknown FPS role '${String(value)}'`);
}
function fpsLifecycleStatus(value) {
    if (value.state === 'active')
        return { state: 'active' };
    if (value.state === 'enemy_defeated') {
        return {
            state: 'enemy_defeated',
            entity: nonNegativeSafeInteger(value.entity ?? -1, 'lifecycleStatus.entity'),
            tick: nonNegativeSafeInteger(value.tick ?? -1, 'lifecycleStatus.tick'),
        };
    }
    throw new RuntimeBridgeError('internal', `unknown native FPS lifecycle status '${value.state}'`);
}
function normalizeFpsPrimaryFireResult(result) {
    return {
        ...result,
        backend: fpsBackend(result.backend),
        target: result.target ?? null,
        targetHealthBefore: result.targetHealthBefore ?? null,
        targetHealthAfter: result.targetHealthAfter ?? null,
        lifecycleStatus: fpsLifecycleStatus(result.lifecycleStatus),
        targetRenderVisible: result.targetRenderVisible ?? null,
        entityHash: hashString(result.entityHash, 'entityHash'),
        healthHash: hashString(result.healthHash, 'healthHash'),
        replayHash: hashString(result.replayHash, 'replayHash'),
    };
}
function hashString(value, field) {
    if (!/^fnv1a64:[0-9a-f]{16}$/u.test(value)) {
        throw new RuntimeBridgeError('internal', `native ${field} was not an fnv1a64 hash`);
    }
    return value;
}
function hexHashString(value, field) {
    if (!/^[0-9a-f]{16}$/u.test(value)) {
        throw new RuntimeBridgeError('internal', `native ${field} was not a 16-character hex hash`);
    }
    return value;
}
function generatedTunnelPreset(value) {
    if (value !== 'tiny-enclosed') {
        throw new RuntimeBridgeError('internal', 'native generated tunnel preset was unknown');
    }
    return value;
}
function normalizeFpsSnapshot(value) {
    return {
        ...value,
        backend: fpsBackend(value.backend),
        lifecycleStatus: fpsLifecycleStatus(value.lifecycleStatus),
        entityHash: hashString(value.entityHash, 'entityHash'),
        healthHash: hashString(value.healthHash, 'healthHash'),
        replayHash: hashString(value.replayHash, 'replayHash'),
        replayRecords: value.replayRecords.map((record) => ({
            ...record,
            entityHash: hashString(record.entityHash, 'replayRecords.entityHash'),
            healthHash: hashString(record.healthHash, 'replayRecords.healthHash'),
            recordHash: hashString(record.recordHash, 'replayRecords.recordHash'),
        })),
    };
}
function normalizeEncounterSnapshot(value) {
    return {
        ...value,
        backend: fpsBackend(value.backend),
        encounterHash: hashString(value.encounterHash, 'encounterHash'),
        replayHash: hashString(value.replayHash, 'replayHash'),
    };
}
function normalizeEncounterTransition(value) {
    return {
        ...value,
        backend: fpsBackend(value.backend),
        encounterHash: hashString(value.encounterHash, 'encounterHash'),
        replayHash: hashString(value.replayHash, 'replayHash'),
    };
}
function nativeFpsLoadRequest(request) {
    if (request.projectBundle.trim() === '') {
        throw new RuntimeBridgeError('invalid_input', 'projectBundle is required');
    }
    if (request.definitions.length === 0) {
        throw new RuntimeBridgeError('invalid_input', 'definitions must not be empty');
    }
    const definitions = request.definitions.map((definition, index) => {
        nonNegativeSafeInteger(definition.entity, `definitions[${index}].entity`);
        fpsRole(definition.role);
        const stableId = requiredString(definition.stableId, `definitions[${index}].stableId`);
        const displayName = requiredString(definition.displayName, `definitions[${index}].displayName`);
        const sourcePath = requiredString(definition.sourcePath, `definitions[${index}].sourcePath`);
        const tags = requiredStringArray(definition.tags, `definitions[${index}].tags`);
        const transform = definition.transform == null
            ? null
            : {
                translation: nativeVec3(definition.transform.translation, `definitions[${index}].transform.translation`),
                rotation: definition.transform.rotation,
                scale: nativeVec3(definition.transform.scale, `definitions[${index}].transform.scale`),
            };
        if (definition.transform != null) {
            if (definition.transform.rotation.length !== 4 || definition.transform.rotation.some((value) => !Number.isFinite(value))) {
                throw new RuntimeBridgeError('invalid_input', `definitions[${index}].transform.rotation must be a finite quat`);
            }
        }
        const bounds = definition.bounds == null
            ? null
            : {
                min: nativeVec3(definition.bounds.min, `definitions[${index}].bounds.min`),
                max: nativeVec3(definition.bounds.max, `definitions[${index}].bounds.max`),
            };
        if (definition.bounds != null) {
        }
        if (definition.health != null) {
            u32(definition.health.current, `definitions[${index}].health.current`);
            u32(definition.health.max, `definitions[${index}].health.max`);
        }
        if (definition.weapon != null) {
            requiredString(definition.weapon.weaponId, `definitions[${index}].weapon.weaponId`);
            u32(definition.weapon.damage, `definitions[${index}].weapon.damage`);
            u32(definition.weapon.rangeUnits, `definitions[${index}].weapon.rangeUnits`);
            u32(definition.weapon.ammo, `definitions[${index}].weapon.ammo`);
            u32(definition.weapon.cooldownTicksAfterFire, `definitions[${index}].weapon.cooldownTicksAfterFire`);
        }
        const policyBinding = definition.policyBinding == null
            ? undefined
            : {
                bindingId: requiredString(definition.policyBinding.bindingId, `definitions[${index}].policyBinding.bindingId`),
                policyId: requiredString(definition.policyBinding.policyId, `definitions[${index}].policyBinding.policyId`),
                viewKind: requiredString(definition.policyBinding.viewKind, `definitions[${index}].policyBinding.viewKind`),
                viewVersion: requiredString(definition.policyBinding.viewVersion, `definitions[${index}].policyBinding.viewVersion`),
                allowedIntents: requiredStringArray(definition.policyBinding.allowedIntents, `definitions[${index}].policyBinding.allowedIntents`),
                runtimeMoment: requiredString(definition.policyBinding.runtimeMoment, `definitions[${index}].policyBinding.runtimeMoment`),
            };
        return {
            entity: definition.entity,
            stableId,
            displayName,
            sourcePath,
            role: definition.role,
            transform: nativeOptionalObject(transform),
            bounds: nativeOptionalObject(bounds),
            tags: [...tags],
            renderVisible: definition.renderVisible,
            staticCollider: definition.staticCollider,
            health: nativeOptionalObject(definition.health),
            weapon: definition.weapon == null
                ? undefined
                : {
                    weaponId: definition.weapon.weaponId,
                    damage: definition.weapon.damage,
                    rangeUnits: definition.weapon.rangeUnits,
                    ammo: definition.weapon.ammo,
                    cooldownTicksAfterFire: definition.weapon.cooldownTicksAfterFire,
                },
            policyBinding,
        };
    });
    return { projectBundle: request.projectBundle, definitions };
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
    loadProjectBundle(request) {
        const handle = this.#requireHandle('loadProjectBundle');
        const bundleSchemaVersion = u32(request.bundleSchemaVersion, 'bundleSchemaVersion');
        const protocolVersion = u32(request.protocolVersion, 'protocolVersion');
        const sceneId = nonNegativeSafeInteger(request.sceneId, 'sceneId');
        const status = callNative(() => this.#addon.loadProjectBundle(handle, bundleSchemaVersion, protocolVersion, sceneId));
        return projectBundleCompositionStatusFromNative(status);
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
    loadFpsRuntimeSession(request) {
        const handle = this.#requireHandle('loadFpsRuntimeSession');
        const nativeRequest = nativeFpsLoadRequest(request);
        const gameRuleModules = request.gameRuleModules ?? [];
        const result = callNative(() => this.#addon.loadFpsRuntimeSession(handle, nativeRequest.projectBundle, nativeRequest.definitions, JSON.stringify(gameRuleModules)));
        return normalizeFpsSnapshot(result);
    }
    readFpsRuntimeSession() {
        const handle = this.#requireHandle('readFpsRuntimeSession');
        const result = callNative(() => this.#addon.readFpsRuntimeSession(handle));
        return normalizeFpsSnapshot(result);
    }
    applyFpsPrimaryFire(request) {
        const handle = this.#requireHandle('applyFpsPrimaryFire');
        const tick = nonNegativeSafeInteger(request.tick, 'tick');
        const origin = nativeVec3(request.origin, 'origin');
        const direction = nativeVec3(request.direction, 'direction');
        const shooterRole = request.shooterRole === undefined ? undefined : fpsRole(request.shooterRole);
        const targetRole = request.targetRole === undefined ? undefined : fpsRole(request.targetRole);
        const result = callNative(() => this.#addon.applyFpsPrimaryFire(handle, tick, origin, direction, shooterRole, targetRole));
        return normalizeFpsPrimaryFireResult(result);
    }
    invokeGameExtensionWeaponEffect(request) {
        const handle = this.#requireHandle('invokeGameExtensionWeaponEffect');
        const tick = nonNegativeSafeInteger(request.primaryFire.tick, 'primaryFire.tick');
        const origin = nativeVec3(request.primaryFire.origin, 'primaryFire.origin');
        const direction = nativeVec3(request.primaryFire.direction, 'primaryFire.direction');
        const shooterRole = request.primaryFire.shooterRole === undefined
            ? undefined
            : fpsRole(request.primaryFire.shooterRole);
        const targetRole = request.primaryFire.targetRole === undefined
            ? undefined
            : fpsRole(request.primaryFire.targetRole);
        const result = callNative(() => this.#addon.invokeGameExtensionWeaponEffect(handle, JSON.stringify(request.hook), tick, origin, direction, shooterRole, targetRole));
        return {
            hookReceipt: parseNativeJson(result.hookReceiptJson, 'game extension hook receipt'),
            replayEvidence: parseNativeJson(result.replayEvidenceJson, 'game extension replay evidence'),
            primaryFire: result.primaryFire === undefined || result.primaryFire === null
                ? null
                : normalizeFpsPrimaryFireResult(result.primaryFire),
        };
    }
    validateGameRuleCatalog(catalog) {
        const handle = this.#requireHandle('validateGameRuleCatalog');
        return parseNativeJson(callNative(() => this.#addon.validateGameRuleCatalog(handle, JSON.stringify(catalog))), 'game-rule catalog validation receipt');
    }
    submitGameRuleEffectIntent(input) {
        const handle = this.#requireHandle('submitGameRuleEffectIntent');
        return parseNativeJson(callNative(() => this.#addon.submitGameRuleEffectIntent(handle, JSON.stringify(input.catalog), JSON.stringify(input.request))), 'game-rule resolution receipt');
    }
    readGameRuleRuntimeReadout() {
        const handle = this.#requireHandle('readGameRuleRuntimeReadout');
        const readout = parseNativeJson(callNative(() => this.#addon.readGameRuleRuntimeReadout(handle)), 'game-rule runtime readout');
        return { ...readout, backend: fpsBackend(readout.backend) };
    }
    restartFpsRuntimeSession(request) {
        const handle = this.#requireHandle('restartFpsRuntimeSession');
        const expectedEpoch = nonNegativeSafeInteger(request.expectedEpoch, 'expectedEpoch');
        const result = callNative(() => this.#addon.restartFpsRuntimeSession(handle, expectedEpoch));
        return normalizeFpsSnapshot(result);
    }
    readFpsEncounterDirector(lifecycle) {
        const handle = this.#requireHandle('readFpsEncounterDirector');
        const result = callNative(() => this.#addon.readFpsEncounterDirector(handle, lifecycle));
        return normalizeEncounterSnapshot(result);
    }
    applyFpsEncounterTransition(request) {
        const handle = this.#requireHandle('applyFpsEncounterTransition');
        const result = callNative(() => this.#addon.applyFpsEncounterTransition(handle, request));
        return normalizeEncounterTransition(result);
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
    saveProjectBundle() {
        const handle = this.#requireHandle('saveProjectBundle');
        return callNative(() => this.#addon.saveProjectBundle(handle));
    }
    getProjectBundleCompositionStatus() {
        const handle = this.#requireHandle('getProjectBundleCompositionStatus');
        const status = callNative(() => this.#addon.getProjectBundleCompositionStatus(handle));
        return projectBundleCompositionStatusFromNative(status);
    }
    planVoxelConversion(request) {
        const handle = this.#requireHandle('planVoxelConversion');
        const payload = callNative(() => this.#addon.planVoxelConversion(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel conversion plan');
    }
    registerVoxelConversionSource(request) {
        const handle = this.#requireHandle('registerVoxelConversionSource');
        const payload = callNative(() => this.#addon.registerVoxelConversionSource(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel conversion source registration');
    }
    registerVoxelConversionMeshAsset(request) {
        const handle = this.#requireHandle('registerVoxelConversionMeshAsset');
        const payload = callNative(() => this.#addon.registerVoxelConversionMeshAsset(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel conversion mesh asset registration');
    }
    importVoxelConversionMeshSource(request) {
        const handle = this.#requireHandle('importVoxelConversionMeshSource');
        const payload = callNative(() => this.#addon.importVoxelConversionMeshSource(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel conversion mesh source import');
    }
    readVoxelConversionSourceMetadata(request) {
        const handle = this.#requireHandle('readVoxelConversionSourceMetadata');
        const payload = callNative(() => this.#addon.readVoxelConversionSourceMetadata(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel conversion source metadata');
    }
    previewVoxelConversion(request) {
        const handle = this.#requireHandle('previewVoxelConversion');
        const payload = callNative(() => this.#addon.previewVoxelConversion(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel conversion preview');
    }
    applyVoxelConversion(request) {
        const handle = this.#requireHandle('applyVoxelConversion');
        const payload = callNative(() => this.#addon.applyVoxelConversion(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel conversion receipt');
    }
    exportVoxelConversionEvidence(evidence) {
        const handle = this.#requireHandle('exportVoxelConversionEvidence');
        const payload = callNative(() => this.#addon.exportVoxelConversionEvidence(handle, JSON.stringify(evidence)));
        return parseNativeJson(payload, 'voxel conversion evidence');
    }
    readVoxelModelInfo(request) {
        const handle = this.#requireHandle('readVoxelModelInfo');
        const payload = callNative(() => this.#addon.readVoxelModelInfo(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel model info');
    }
    readVoxelModelWindow(request) {
        const handle = this.#requireHandle('readVoxelModelWindow');
        const payload = callNative(() => this.#addon.readVoxelModelWindow(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel model window');
    }
    exportVoxelVolumeAsset(request) {
        const handle = this.#requireHandle('exportVoxelVolumeAsset');
        const payload = callNative(() => this.#addon.exportVoxelVolumeAsset(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel volume asset export receipt');
    }
    saveVoxelVolumeAsset(request) {
        const handle = this.#requireHandle('saveVoxelVolumeAsset');
        const payload = callNative(() => this.#addon.saveVoxelVolumeAsset(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel volume asset save receipt');
    }
    updateVoxelVolumeAssetPalette(request) {
        const handle = this.#requireHandle('updateVoxelVolumeAssetPalette');
        const payload = callNative(() => this.#addon.updateVoxelVolumeAssetPalette(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel volume asset palette update receipt');
    }
    initializeVoxelVolumeAuthoring(request) {
        const handle = this.#requireHandle('initializeVoxelVolumeAuthoring');
        const payload = callNative(() => this.#addon.initializeVoxelVolumeAuthoring(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel volume authoring initialize receipt');
    }
    loadVoxelVolumeAsset(request) {
        const handle = this.#requireHandle('loadVoxelVolumeAsset');
        const payload = callNative(() => this.#addon.loadVoxelVolumeAsset(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel volume asset load receipt');
    }
    unloadVoxelVolumeAsset(request) {
        const handle = this.#requireHandle('unloadVoxelVolumeAsset');
        const payload = callNative(() => this.#addon.unloadVoxelVolumeAsset(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel volume asset unload receipt');
    }
    validateVoxelAnnotationLayer(request) {
        const handle = this.#requireHandle('validateVoxelAnnotationLayer');
        const payload = callNative(() => this.#addon.validateVoxelAnnotationLayer(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel annotation validation report');
    }
    loadVoxelAnnotationLayer(request) {
        const handle = this.#requireHandle('loadVoxelAnnotationLayer');
        const payload = callNative(() => this.#addon.loadVoxelAnnotationLayer(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel annotation load receipt');
    }
    readVoxelAnnotationQuery(request) {
        const handle = this.#requireHandle('readVoxelAnnotationQuery');
        const payload = callNative(() => this.#addon.readVoxelAnnotationQuery(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel annotation query readout');
    }
    applyVoxelAnnotationEdit(request) {
        const handle = this.#requireHandle('applyVoxelAnnotationEdit');
        const payload = callNative(() => this.#addon.applyVoxelAnnotationEdit(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel annotation edit receipt');
    }
    exportVoxelAnnotationLayer(request) {
        const handle = this.#requireHandle('exportVoxelAnnotationLayer');
        const payload = callNative(() => this.#addon.exportVoxelAnnotationLayer(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel annotation export receipt');
    }
    // ── Unwired operations: fail-closed, never mock-backed ─────────────────────
    // Replace each body with its real native call (and add the manifest name to
    // NATIVE_WIRED_OPERATIONS) when the codegen emitter wires the `#[napi]` export.
    pickVoxel() {
        throw nativeUnimplemented('pick_voxel');
    }
    applyCollisionConstrainedCameraInput(envelope) {
        const handle = this.#requireHandle('applyCollisionConstrainedCameraInput');
        return callNative(() => this.#addon.applyCollisionConstrainedCameraInput(handle, envelope));
    }
    applyGeneratedTunnelToRuntimeWorld(request) {
        const handle = this.#requireHandle('applyGeneratedTunnelToRuntimeWorld');
        if (request.preset !== 'tiny-enclosed') {
            throw new RuntimeBridgeError('invalid_input', 'only the tiny-enclosed generated tunnel preset is supported');
        }
        const seed = nonNegativeSafeInteger(request.seed, 'seed');
        const receipt = callNative(() => this.#addon.applyGeneratedTunnelToRuntimeWorld(handle, request.preset, seed));
        return {
            preset: generatedTunnelPreset(receipt.presetId),
            seed: nonNegativeSafeInteger(receipt.seed, 'receipt.seed'),
            grid: nonNegativeSafeInteger(receipt.grid, 'receipt.grid'),
            configHash: hexHashString(receipt.configHash, 'generatedTunnel.configHash'),
            outputHash: hexHashString(receipt.outputHash, 'generatedTunnel.outputHash'),
            collisionSourceHash: hexHashString(receipt.collisionSourceHash, 'generatedTunnel.collisionSourceHash'),
            collisionProjectionHash: hashString(receipt.collisionProjectionHash, 'generatedTunnel.collisionProjectionHash'),
            runtimeFrame: {
                worldOffset: bridgeVec3Array(receipt.runtimeFrame.worldOffset, 'generatedTunnel.runtimeFrame.worldOffset'),
                playableMin: bridgeVec3Array(receipt.runtimeFrame.playableMin, 'generatedTunnel.runtimeFrame.playableMin'),
                playableMax: bridgeVec3Array(receipt.runtimeFrame.playableMax, 'generatedTunnel.runtimeFrame.playableMax'),
            },
        };
    }
    selectVoxel() {
        throw nativeUnimplemented('select_voxel');
    }
    readVoxelMeshEvidence() {
        throw nativeUnimplemented('read_voxel_mesh_evidence');
    }
    readVoxelEditHistory(request) {
        const handle = this.#requireHandle('readVoxelEditHistory');
        const payload = callNative(() => this.#addon.readVoxelEditHistory(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel edit history summary');
    }
    previewVoxelEditRevert(request) {
        const handle = this.#requireHandle('previewVoxelEditRevert');
        const payload = callNative(() => this.#addon.previewVoxelEditRevert(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel edit history revert preview');
    }
    applyVoxelEditRevert(request) {
        const handle = this.#requireHandle('applyVoxelEditRevert');
        const payload = callNative(() => this.#addon.applyVoxelEditRevert(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel edit history revert apply');
    }
    undoVoxelEdit(request) {
        const handle = this.#requireHandle('undoVoxelEdit');
        const payload = callNative(() => this.#addon.undoVoxelEdit(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel edit history undo receipt');
    }
    redoVoxelEdit(request) {
        const handle = this.#requireHandle('redoVoxelEdit');
        const payload = callNative(() => this.#addon.redoVoxelEdit(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel edit history redo receipt');
    }
    createCamera(request) {
        const handle = this.#requireHandle('createCamera');
        return callNative(() => this.#addon.createCamera(handle, request));
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
    unloadProjectBundle() {
        throw nativeUnimplemented('unload_project_bundle');
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