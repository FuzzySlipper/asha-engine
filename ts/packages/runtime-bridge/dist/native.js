import { loadNativeAddon, NativeAddonUnavailable } from '@asha/native-bridge';
import { MANIFEST_OPERATIONS } from './generated/operations.js';
import { parseOperationOutput, validateOperationInput, validateOperationOutput, } from './wire-validation.js';
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
export { NATIVE_WIRED_OPERATIONS } from './generated/operations.js';
function nativeUnimplemented(manifestName) {
    return new RuntimeBridgeError('operation_unimplemented', `native bridge operation '${manifestName}' is not wired; the native facade is ` +
        `fail-closed (no mock fallback). Wire its #[napi] export and add it to ` +
        `NATIVE_WIRED_OPERATIONS.`);
}
const RUNTIME_BRIDGE_ERROR_KINDS = new Set([
    'not_initialized',
    'invalid_input',
    'unknown_handle',
    'buffer_expired',
    'native_unavailable',
    'voxel_conversion_unavailable',
    'unsupported_source_asset',
    'source_hash_mismatch',
    'invalid_material_map',
    'output_limit_exceeded',
    'stale_authority_snapshot',
    'conversion_replay_mismatch',
    'operation_unimplemented',
    'internal',
]);
const NATIVE_ERROR_KEYS = new Set([
    'schemaVersion',
    'code',
    'operation',
    'path',
    'retryable',
    'message',
    'details',
    'provenance',
]);
let activeNativeOperation = null;
const OPERATION_BY_FACADE_METHOD = new Map(MANIFEST_OPERATIONS.map((operation) => [operation.facadeMethod, operation.manifestName]));
function boundedText(value, maxLength) {
    return value.length <= maxLength ? value : `${value.slice(0, maxLength - 1)}…`;
}
function parseNativeErrorEnvelope(message) {
    let parsed;
    try {
        parsed = JSON.parse(message);
    }
    catch {
        return null;
    }
    if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed))
        return null;
    const envelope = parsed;
    if (Object.keys(envelope).some((key) => !NATIVE_ERROR_KEYS.has(key)))
        return null;
    if (envelope['schemaVersion'] !== 1)
        return null;
    if (typeof envelope['code'] !== 'string' || !RUNTIME_BRIDGE_ERROR_KINDS.has(envelope['code']))
        return null;
    if (typeof envelope['operation'] !== 'string' || envelope['operation'].length === 0)
        return null;
    if (typeof envelope['path'] !== 'string' || envelope['path'].length === 0)
        return null;
    if (typeof envelope['retryable'] !== 'boolean')
        return null;
    if (typeof envelope['message'] !== 'string' || envelope['message'].length === 0)
        return null;
    if (envelope['provenance'] !== 'native_rust')
        return null;
    if (!Array.isArray(envelope['details']) || envelope['details'].some((detail) => typeof detail !== 'string')) {
        return null;
    }
    return {
        code: envelope['code'],
        details: envelope['details'].slice(0, 8).map((detail) => boundedText(detail, 128)),
        message: boundedText(envelope['message'], 512),
        operation: boundedText(envelope['operation'], 128),
        path: boundedText(envelope['path'], 256),
        provenance: 'native_rust',
        retryable: envelope['retryable'],
    };
}
export function classifyNativeAddonError(cause) {
    if (cause instanceof RuntimeBridgeError)
        return cause;
    const message = cause instanceof Error ? cause.message : String(cause);
    const envelope = parseNativeErrorEnvelope(message);
    if (envelope !== null) {
        return new RuntimeBridgeError(envelope.code, envelope.message, {
            details: envelope.details,
            operation: activeNativeOperation ?? envelope.operation,
            path: envelope.path,
            provenance: envelope.provenance,
            retryable: envelope.retryable,
        });
    }
    return new RuntimeBridgeError('internal', boundedText(message, 512), {
        details: ['invalid_native_error_envelope'],
        operation: activeNativeOperation ?? 'native_bridge',
        path: '$',
        provenance: 'transport_loader',
        retryable: false,
    });
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
    if (activeNativeOperation === null) {
        throw new RuntimeBridgeError('internal', `native ${field} was decoded outside an operation boundary`);
    }
    return parseOperationOutput(activeNativeOperation, payload);
}
function projectBundleCompositionStatusFromNative(status) {
    return {
        loadedProjectBundle: status.loadedProjectBundle ?? null,
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
function projectionFrameFromNative(native) {
    if (native.schemaVersion !== 1 || !Number.isSafeInteger(native.authorityTick)) {
        throw new RuntimeBridgeError('internal', 'native projection frame header is invalid');
    }
    if (native.presentation?.replayScope !== 'excludedFromReplayTruth') {
        throw new RuntimeBridgeError('internal', 'native projection replay scope is invalid');
    }
    if (!Array.isArray(native.presentation.ops)) {
        throw new RuntimeBridgeError('internal', 'native projection operations must be an array');
    }
    const nativeOperations = native.presentation.ops;
    const ops = nativeOperations.map((operation, index) => {
        if (operation.meta?.sequence !== index) {
            throw new RuntimeBridgeError('internal', 'native presentation sequence is not contiguous');
        }
        if (operation.domain === 'audio'
            && operation.audioOp !== undefined
            && operation.billboardOp === undefined
            && operation.particleOp === undefined
            && operation.telemetryOverlayOp === undefined
            && operation.animationOp === undefined) {
            return { domain: 'audio', meta: operation.meta, op: operation.audioOp };
        }
        if (operation.domain === 'billboard'
            && operation.billboardOp !== undefined
            && operation.audioOp === undefined
            && operation.particleOp === undefined
            && operation.telemetryOverlayOp === undefined
            && operation.animationOp === undefined) {
            return { domain: 'billboard', meta: operation.meta, op: operation.billboardOp };
        }
        if (operation.domain === 'particle'
            && operation.particleOp !== undefined
            && operation.audioOp === undefined
            && operation.billboardOp === undefined
            && operation.telemetryOverlayOp === undefined
            && operation.animationOp === undefined) {
            return { domain: 'particle', meta: operation.meta, op: operation.particleOp };
        }
        if (operation.domain === 'telemetryOverlay'
            && operation.telemetryOverlayOp !== undefined
            && operation.audioOp === undefined
            && operation.billboardOp === undefined
            && operation.particleOp === undefined
            && operation.animationOp === undefined) {
            return {
                domain: 'telemetryOverlay',
                meta: operation.meta,
                op: operation.telemetryOverlayOp,
            };
        }
        if (operation.domain === 'animation'
            && operation.animationOp !== undefined
            && operation.audioOp === undefined
            && operation.billboardOp === undefined
            && operation.particleOp === undefined
            && operation.telemetryOverlayOp === undefined) {
            return {
                domain: 'animation',
                meta: operation.meta,
                op: animationProjectionOperationFromNative(operation.animationOp),
            };
        }
        throw new RuntimeBridgeError('internal', `native presentation operation ${index} has an invalid closed-domain payload`);
    });
    return {
        schemaVersion: native.schemaVersion,
        authorityTick: native.authorityTick,
        scene: native.scene,
        presentation: {
            replayScope: native.presentation.replayScope,
            ops,
        },
    };
}
function animationProjectionOperationFromNative(operation) {
    if (operation.op === 'destroy') {
        return operation;
    }
    if (operation.op === 'create') {
        if (operation.descriptor === undefined) {
            throw new RuntimeBridgeError('internal', 'native animation create descriptor is missing');
        }
        return {
            ...operation,
            descriptor: {
                ...operation.descriptor,
                controller: animationControllerFromNative(operation.descriptor.controller),
            },
        };
    }
    if (operation.controller === undefined) {
        throw new RuntimeBridgeError('internal', 'native animation update controller is missing');
    }
    return {
        ...operation,
        controller: animationControllerFromNative(operation.controller),
    };
}
function animationControllerFromNative(controller) {
    const native = controller;
    return {
        ...native,
        motion: { ...native.motion, clipB: native.motion.clipB ?? null },
        transition: native.transition === undefined
            ? null
            : {
                ...native.transition,
                targetMotion: {
                    ...native.transition.targetMotion,
                    clipB: native.transition.targetMotion.clipB ?? null,
                },
            },
        timingFact: native.timingFact ?? null,
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
        return new Proxy(this, {
            get: (target, property) => {
                const member = Reflect.get(target, property, target);
                if (typeof property !== 'string' || typeof member !== 'function')
                    return member;
                const operation = OPERATION_BY_FACADE_METHOD.get(property);
                if (operation === undefined) {
                    return (...args) => Reflect.apply(member, target, args);
                }
                return (...args) => {
                    const input = args.length === 0 ? null : args[0] ?? null;
                    validateOperationInput(operation, input);
                    const previousOperation = activeNativeOperation;
                    activeNativeOperation = operation;
                    try {
                        const output = Reflect.apply(member, target, args);
                        validateOperationOutput(operation, output ?? null);
                        return output;
                    }
                    finally {
                        activeNativeOperation = previousOperation;
                    }
                };
            },
        });
    }
    // ── Wired native operations ───────────────────────────────────────────────
    initializeEngine(config) {
        if (!Number.isInteger(config.seed) || config.seed < 0) {
            throw new RuntimeBridgeError('invalid_input', `seed must be a non-negative integer`);
        }
        this.#seed = config.seed;
        const handle = callNative(() => this.#addon.initializeEngine(config.seed));
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
    configureInputSession(request) {
        const handle = this.#requireHandle('configureInputSession');
        const payload = callNative(() => this.#addon.configureInputSession(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'input session snapshot');
    }
    applyInputContextCommand(command) {
        const handle = this.#requireHandle('applyInputContextCommand');
        const payload = callNative(() => this.#addon.applyInputContextCommand(handle, JSON.stringify(command)));
        return parseNativeJson(payload, 'input context change receipt');
    }
    submitRawInput(sample) {
        const handle = this.#requireHandle('submitRawInput');
        const payload = callNative(() => this.#addon.submitRawInput(handle, JSON.stringify(sample)));
        return parseNativeJson(payload, 'input resolution receipt');
    }
    replayResolvedInputAction(record) {
        const handle = this.#requireHandle('replayResolvedInputAction');
        const payload = callNative(() => this.#addon.replayResolvedInputAction(handle, JSON.stringify(record)));
        return parseNativeJson(payload, 'input action replay receipt');
    }
    readInputContextState() {
        const handle = this.#requireHandle('readInputContextState');
        const payload = callNative(() => this.#addon.readInputContextState(handle));
        return parseNativeJson(payload, 'input context state');
    }
    applyTimeControlCommand(command) {
        const handle = this.#requireHandle('applyTimeControlCommand');
        const payload = callNative(() => this.#addon.applyTimeControlCommand(handle, JSON.stringify(command)));
        return parseNativeJson(payload, 'time control receipt');
    }
    readTimeControlState() {
        const handle = this.#requireHandle('readTimeControlState');
        const payload = callNative(() => this.#addon.readTimeControlState(handle));
        return parseNativeJson(payload, 'time control state');
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
        const result = callNative(() => this.#addon.stepSimulation(handle, tick));
        return {
            tick: nonNegativeSafeInteger(result.tick, 'native step tick'),
            diffCount: u32(result.diffCount, 'native step diffCount'),
        };
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
        const handle = this.#requireHandle('readModelMaterialPreview');
        const payload = callNative(() => this.#addon.readModelMaterialPreview(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'model material preview snapshot');
    }
    readSceneObjectSnapshot() {
        const handle = this.#requireHandle('readSceneObjectSnapshot');
        const payload = callNative(() => this.#addon.readSceneObjectSnapshot(handle));
        return parseNativeJson(payload, 'scene object snapshot');
    }
    applySceneObjectCommand(request) {
        const handle = this.#requireHandle('applySceneObjectCommand');
        const payload = callNative(() => this.#addon.applySceneObjectCommand(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'scene object command result');
    }
    readRenderDiffs(cursor) {
        const handle = this.#requireHandle('readRenderDiffs');
        const frame = nonNegativeSafeInteger(cursor, 'frame cursor');
        return callNative(() => this.#addon.readRenderDiffs(handle, frame));
    }
    readProjectionFrame(cursor) {
        const handle = this.#requireHandle('readProjectionFrame');
        const frame = nonNegativeSafeInteger(cursor, 'frame cursor');
        const nativeFrame = callNative(() => this.#addon.readProjectionFrame(handle, frame));
        return projectionFrameFromNative(nativeFrame);
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
    pickVoxel(ray) {
        const handle = this.#requireHandle('pickVoxel');
        const payload = callNative(() => this.#addon.pickVoxel(handle, JSON.stringify(ray)));
        return parseNativeJson(payload, 'voxel pick result');
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
    selectVoxel(request) {
        const handle = this.#requireHandle('selectVoxel');
        const payload = callNative(() => this.#addon.selectVoxel(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel selection snapshot');
    }
    readVoxelMeshEvidence(request) {
        const handle = this.#requireHandle('readVoxelMeshEvidence');
        const payload = callNative(() => this.#addon.readVoxelMeshEvidence(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'voxel mesh evidence snapshot');
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
    applyCameraModeCommand(command) {
        const handle = this.#requireHandle('applyCameraModeCommand');
        const payload = callNative(() => this.#addon.applyCameraModeCommand(handle, JSON.stringify(command)));
        return parseNativeJson(payload, 'camera mode change receipt');
    }
    applyCameraNavigationInput(input) {
        const handle = this.#requireHandle('applyCameraNavigationInput');
        const payload = callNative(() => this.#addon.applyCameraNavigationInput(handle, JSON.stringify(input)));
        return parseNativeJson(payload, 'camera navigation receipt');
    }
    readCameraControllerState(request) {
        const handle = this.#requireHandle('readCameraControllerState');
        const payload = callNative(() => this.#addon.readCameraControllerState(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'camera controller state');
    }
    applyFirstPersonCameraInput(input) {
        const handle = this.#requireHandle('applyFirstPersonCameraInput');
        return callNative(() => this.#addon.applyFirstPersonCameraInput(handle, input));
    }
    readCameraProjection(request) {
        const handle = this.#requireHandle('readCameraProjection');
        const payload = callNative(() => this.#addon.readCameraProjection(handle, JSON.stringify(request)));
        return parseNativeJson(payload, 'camera projection snapshot');
    }
    getBuffer(bufferHandle) {
        const handle = this.#requireHandle('getBuffer');
        const validatedBufferHandle = nonNegativeSafeInteger(bufferHandle, 'buffer handle');
        const view = callNative(() => this.#addon.getBuffer(handle, validatedBufferHandle));
        return {
            handle: nonNegativeSafeInteger(view.handle, 'returned buffer handle'),
            bytes: Uint8Array.from(view.bytes),
        };
    }
    releaseBuffer(bufferHandle) {
        const handle = this.#requireHandle('releaseBuffer');
        const validatedBufferHandle = nonNegativeSafeInteger(bufferHandle, 'buffer handle');
        callNative(() => this.#addon.releaseBuffer(handle, validatedBufferHandle));
    }
    unloadProjectBundle() {
        const handle = this.#requireHandle('unloadProjectBundle');
        callNative(() => this.#addon.unloadProjectBundle(handle));
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