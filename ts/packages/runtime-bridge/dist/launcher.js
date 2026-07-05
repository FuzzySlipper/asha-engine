import { RuntimeBridgeError, frameCursor, } from './bridge.js';
import { createMockRuntimeBridge } from './mock.js';
function requireNonEmpty(value, field) {
    if (value.trim().length === 0) {
        throw new RuntimeBridgeError('invalid_input', `${field} must be a non-empty string`);
    }
    return value;
}
function referenceNonClaims() {
    return ['not_native_runtime', 'not_hardware_gpu', 'not_performance_evidence', 'not_publish_artifact', 'not_wasm_authority'];
}
function selectedNativeNonClaims() {
    return ['not_hardware_gpu', 'not_performance_evidence', 'not_publish_artifact', 'not_wasm_authority'];
}
function backendProfileDiagnostic(code, message, severity = 'error') {
    return { code, severity, message };
}
function isPlainRecord(value) {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function hasOnlyKeys(value, keys) {
    const allowed = new Set(keys);
    return Object.keys(value).every((key) => allowed.has(key));
}
function containsPrivateTransportHint(value) {
    return [
        '@asha/native-bridge',
        '@asha/wasm-bridge',
        '@asha/wasm-replay-bridge',
        'native-bridge.node',
        'wasm.memory',
        '/src/',
        'engine-rs/',
    ].some((hint) => value.includes(hint));
}
function isGameRuntimeEvidenceRef(value) {
    if (!isPlainRecord(value) || !hasOnlyKeys(value, ['kind', 'id', 'path', 'sha256', 'sequenceId'])) {
        return false;
    }
    return ((value.kind === 'projection'
        || value.kind === 'render_diff'
        || value.kind === 'replay'
        || value.kind === 'evidence_export'
        || value.kind === 'telemetry'
        || value.kind === 'diagnostic')
        && typeof value.id === 'string'
        && (value.path === undefined || typeof value.path === 'string')
        && (value.sha256 === undefined || typeof value.sha256 === 'string')
        && (value.sequenceId === undefined || Number.isInteger(value.sequenceId)));
}
function isGameRuntimeCompatibility(value) {
    return isPlainRecord(value)
        && typeof value.contractsPackageVersion === 'string'
        && typeof value.runtimeBridgePackageVersion === 'string'
        && (value.devtoolsProtocolVersion === undefined || typeof value.devtoolsProtocolVersion === 'string')
        && (value.publishArtifactVersion === undefined || typeof value.publishArtifactVersion === 'string');
}
function isGameRuntimeNonClaim(value) {
    return value === 'not_native_runtime'
        || value === 'not_hardware_gpu'
        || value === 'not_performance_evidence'
        || value === 'not_publish_artifact'
        || value === 'not_wasm_authority';
}
export function validateGameRuntimeBackendProfile(input) {
    const diagnostics = [];
    if (!isPlainRecord(input) || !hasOnlyKeys(input, [
        'profileId',
        'mode',
        'transport',
        'launcherName',
        'bridgeCompatibility',
        'evidenceRefs',
        'nonClaims',
    ])) {
        return {
            ok: false,
            diagnostics: [backendProfileDiagnostic('private_transport_hint', 'backend profile must use the closed public shape')],
        };
    }
    const mode = input.mode;
    const transport = input.transport;
    if (mode !== 'reference' && mode !== 'native' && mode !== 'wasm') {
        diagnostics.push(backendProfileDiagnostic('unsupported_backend_mode', `unsupported backend mode: ${String(mode)}`));
    }
    if (transport !== 'reference_mock' && transport !== 'napi_native' && transport !== 'wasm_module') {
        diagnostics.push(backendProfileDiagnostic('private_transport_hint', `unsupported backend transport: ${String(transport)}`));
    }
    if (typeof input.profileId !== 'string' || input.profileId.trim().length === 0) {
        diagnostics.push(backendProfileDiagnostic('missing_backend_evidence', 'profileId must be a non-empty string'));
    }
    else if (containsPrivateTransportHint(input.profileId)) {
        diagnostics.push(backendProfileDiagnostic('private_transport_hint', 'profileId must not contain private transport hints'));
    }
    if (typeof input.launcherName !== 'string' || input.launcherName.trim().length === 0) {
        diagnostics.push(backendProfileDiagnostic('missing_backend_evidence', 'launcherName must be a non-empty string'));
    }
    else if (containsPrivateTransportHint(input.launcherName)) {
        diagnostics.push(backendProfileDiagnostic('private_transport_hint', 'launcherName must not contain private transport hints'));
    }
    if (!isGameRuntimeCompatibility(input.bridgeCompatibility)) {
        diagnostics.push(backendProfileDiagnostic('missing_compatibility', 'bridgeCompatibility must include public compatibility metadata'));
    }
    if (!Array.isArray(input.evidenceRefs) || !input.evidenceRefs.every(isGameRuntimeEvidenceRef)) {
        diagnostics.push(backendProfileDiagnostic('missing_backend_evidence', 'evidenceRefs must be typed public evidence refs'));
    }
    if (!Array.isArray(input.nonClaims) || !input.nonClaims.every(isGameRuntimeNonClaim)) {
        diagnostics.push(backendProfileDiagnostic('backend_claim_mismatch', 'nonClaims must use the public non-claim vocabulary'));
    }
    const evidenceRefs = Array.isArray(input.evidenceRefs) && input.evidenceRefs.every(isGameRuntimeEvidenceRef)
        ? input.evidenceRefs
        : [];
    const nonClaims = Array.isArray(input.nonClaims) && input.nonClaims.every(isGameRuntimeNonClaim)
        ? input.nonClaims
        : [];
    if (mode === 'reference') {
        if (transport !== 'reference_mock') {
            diagnostics.push(backendProfileDiagnostic('backend_claim_mismatch', 'reference mode must use reference_mock transport'));
        }
        if (!nonClaims.includes('not_native_runtime') || !nonClaims.includes('not_wasm_authority')) {
            diagnostics.push(backendProfileDiagnostic('backend_claim_mismatch', 'reference mode must preserve native/WASM non-claims'));
        }
    }
    if (mode === 'native') {
        if (transport !== 'napi_native') {
            diagnostics.push(backendProfileDiagnostic('backend_claim_mismatch', 'native mode must use napi_native transport'));
        }
        if (evidenceRefs.length === 0) {
            diagnostics.push(backendProfileDiagnostic('missing_backend_evidence', 'native mode requires backend evidence refs'));
        }
        if (nonClaims.includes('not_native_runtime')) {
            diagnostics.push(backendProfileDiagnostic('backend_claim_mismatch', 'native mode cannot carry not_native_runtime'));
        }
    }
    if (mode === 'wasm') {
        if (transport !== 'wasm_module') {
            diagnostics.push(backendProfileDiagnostic('backend_claim_mismatch', 'wasm mode must use wasm_module transport'));
        }
        if (evidenceRefs.length === 0) {
            diagnostics.push(backendProfileDiagnostic('missing_backend_evidence', 'wasm mode requires backend evidence refs'));
        }
    }
    if (diagnostics.length > 0) {
        return { ok: false, diagnostics };
    }
    return {
        ok: true,
        profile: input,
        diagnostics: [],
    };
}
export function referenceBackendProfile(config) {
    return {
        profileId: 'reference.launcher.v1',
        mode: 'reference',
        transport: 'reference_mock',
        launcherName: 'reference-game-runtime-launcher',
        bridgeCompatibility: config.compatibility,
        evidenceRefs: [{ kind: 'diagnostic', id: 'backend-profile:reference' }],
        nonClaims: referenceNonClaims(),
    };
}
export function nativeBackendProfile(config) {
    return {
        profileId: 'native.napi.launcher.v1',
        mode: 'native',
        transport: 'napi_native',
        launcherName: 'native-game-runtime-launcher',
        bridgeCompatibility: config.compatibility,
        evidenceRefs: [{ kind: 'diagnostic', id: 'backend-profile:native:napi', path: config.runtimeEntry }],
        nonClaims: selectedNativeNonClaims(),
    };
}
function referenceRuntimeProfile(config) {
    const profile = referenceBackendProfile(config);
    return {
        profileId: profile.profileId,
        runtimeMode: profile.mode,
        launcherName: profile.launcherName,
        bridgeCompatibility: profile.bridgeCompatibility,
        nonClaims: profile.nonClaims,
    };
}
function projectionSummary(config, runtimeMode, status, sequenceId, acceptedCommandCount) {
    const loadedWorld = status.loadedWorld;
    const worldHash = `${runtimeMode}-world:${config.gameId}:${loadedWorld ?? 'none'}:accepted:${acceptedCommandCount}`;
    const authorityHash = `${runtimeMode}-authority:${config.workspaceId}:${loadedWorld ?? 'none'}:accepted:${acceptedCommandCount}`;
    return {
        sequenceId,
        worldHash,
        authorityHash,
        loadedWorld,
        fatalCount: status.fatalCount,
        totalDiagnosticCount: status.totalCount,
        evidenceRefs: [{ kind: 'projection', id: `projection:${sequenceId}`, sequenceId }],
    };
}
class ReferenceGameRuntimeSession {
    bridge;
    config;
    identity;
    launch;
    #runtimeProfile;
    #sequenceId = 0;
    #acceptedCommandCount = 0;
    #rejectedCommandCount = 0;
    #shutdown = false;
    constructor(bridge, config, runtimeProfile, initialStatus) {
        this.bridge = bridge;
        this.config = config;
        this.#runtimeProfile = runtimeProfile;
        const startedAtIso = config.startedAtIso ?? new Date(0).toISOString();
        this.identity = {
            gameId: config.gameId,
            workspaceId: config.workspaceId,
            runtimeMode: runtimeProfile.runtimeMode,
            runtimeEntry: config.runtimeEntry,
            startedAtIso,
            compatibility: config.compatibility,
            nonClaims: runtimeProfile.nonClaims,
        };
        const projection = projectionSummary(config, runtimeProfile.runtimeMode, initialStatus, this.#sequenceId, this.#acceptedCommandCount);
        this.launch = {
            status: 'launched',
            identity: this.identity,
            runtimeProfile,
            resourceProfile: config.resourceProfile,
            projection,
            diagnostics: [],
            evidenceRefs: projection.evidenceRefs,
        };
    }
    async pullProjection() {
        this.#assertOpen();
        return projectionSummary(this.config, this.#runtimeProfile.runtimeMode, this.bridge.getCompositionStatus(), this.#sequenceId, this.#acceptedCommandCount);
    }
    async pullRenderDiff(cursor = frameCursor(0)) {
        this.#assertOpen();
        const frame = this.bridge.readRenderDiffs(cursor);
        return {
            sequenceId: this.#sequenceId,
            cursor,
            frame,
            evidenceRefs: [{ kind: 'render_diff', id: `render-diff:${cursor}`, sequenceId: this.#sequenceId }],
        };
    }
    async pullTelemetry() {
        this.#assertOpen();
        return {
            sequenceId: this.#sequenceId,
            runtimeMode: this.#runtimeProfile.runtimeMode,
            acceptedCommandCount: this.#acceptedCommandCount,
            rejectedCommandCount: this.#rejectedCommandCount,
            diagnostics: [],
            evidenceRefs: [{ kind: 'telemetry', id: `telemetry:${this.#sequenceId}`, sequenceId: this.#sequenceId }],
        };
    }
    async proposeCommands(batch) {
        this.#assertOpen();
        const before = await this.pullProjection();
        this.#sequenceId += 1;
        try {
            const result = this.bridge.submitCommands(batch);
            this.#acceptedCommandCount += result.accepted;
            this.#rejectedCommandCount += result.rejected;
            const after = await this.pullProjection();
            return {
                sequenceId: this.#sequenceId,
                status: result.rejected > 0 ? 'rejected' : 'accepted',
                batch,
                result,
                authorityHashBefore: before.authorityHash,
                authorityHashAfter: after.authorityHash,
                diagnostics: result.rejections.map((rejection, index) => ({
                    code: 'command_rejected',
                    severity: 'warning',
                    message: `command ${index} rejected: ${rejection.reason}`,
                })),
                evidenceRefs: [{ kind: 'replay', id: `command:${this.#sequenceId}`, sequenceId: this.#sequenceId }],
            };
        }
        catch (cause) {
            const error = cause instanceof RuntimeBridgeError ? cause : new RuntimeBridgeError('internal', String(cause));
            const after = await this.pullProjection();
            return {
                sequenceId: this.#sequenceId,
                status: 'failed',
                batch,
                result: null,
                authorityHashBefore: before.authorityHash,
                authorityHashAfter: after.authorityHash,
                diagnostics: [{ code: 'internal', severity: 'error', message: error.message }],
                evidenceRefs: [{ kind: 'diagnostic', id: `command-failed:${this.#sequenceId}`, sequenceId: this.#sequenceId }],
            };
        }
    }
    async exportReplay(request) {
        this.#assertOpen();
        requireNonEmpty(request.replayId, 'replayId');
        const projection = await this.pullProjection();
        return {
            replayId: request.replayId,
            sequenceId: this.#sequenceId,
            authorityHash: projection.authorityHash,
            evidenceRefs: [{ kind: 'replay', id: request.replayId, sequenceId: this.#sequenceId }],
        };
    }
    async exportEvidence(request) {
        this.#assertOpen();
        requireNonEmpty(request.evidenceId, 'evidenceId');
        const projection = await this.pullProjection();
        return {
            evidenceId: request.evidenceId,
            sequenceId: this.#sequenceId,
            projection,
            nonClaims: this.identity.nonClaims,
            evidenceRefs: [{ kind: 'evidence_export', id: request.evidenceId, sequenceId: this.#sequenceId }],
        };
    }
    async shutdown() {
        if (this.#shutdown)
            return;
        this.bridge.unloadWorld();
        this.#shutdown = true;
    }
    #assertOpen() {
        if (this.#shutdown) {
            throw new RuntimeBridgeError('not_initialized', 'game runtime session has been shut down');
        }
    }
}
export class ReferenceGameRuntimeLauncher {
    mode = 'reference';
    async launch(config) {
        requireNonEmpty(config.gameId, 'gameId');
        requireNonEmpty(config.workspaceId, 'workspaceId');
        requireNonEmpty(config.runtimeEntry, 'runtimeEntry');
        requireNonEmpty(config.compatibility.contractsPackageVersion, 'compatibility.contractsPackageVersion');
        requireNonEmpty(config.compatibility.runtimeBridgePackageVersion, 'compatibility.runtimeBridgePackageVersion');
        if (config.runtimeEntry !== config.resourceProfile.runtimeEntry) {
            throw new RuntimeBridgeError('invalid_input', 'runtimeEntry must match resourceProfile.runtimeEntry');
        }
        const bridge = createMockRuntimeBridge();
        bridge.initializeEngine({ seed: config.world.sceneId });
        const status = bridge.loadWorldBundle(config.world);
        if (status.blocksLoad || status.loadedWorld === null) {
            throw new RuntimeBridgeError('invalid_input', 'world bundle failed to load for reference launcher');
        }
        return new ReferenceGameRuntimeSession(bridge, config, referenceRuntimeProfile(config), status);
    }
}
export function createReferenceGameRuntimeLauncher() {
    return new ReferenceGameRuntimeLauncher();
}
function runtimeProfileFromBackendProfile(profile) {
    return {
        profileId: profile.profileId,
        runtimeMode: profile.mode,
        launcherName: profile.launcherName,
        bridgeCompatibility: profile.bridgeCompatibility,
        nonClaims: profile.nonClaims,
    };
}
export class SelectedBackendGameRuntimeLauncher {
    options;
    mode;
    constructor(options = {}) {
        this.options = options;
        this.mode = options.profile?.mode ?? 'native';
    }
    async launch(config) {
        requireNonEmpty(config.gameId, 'gameId');
        requireNonEmpty(config.workspaceId, 'workspaceId');
        requireNonEmpty(config.runtimeEntry, 'runtimeEntry');
        requireNonEmpty(config.compatibility.contractsPackageVersion, 'compatibility.contractsPackageVersion');
        requireNonEmpty(config.compatibility.runtimeBridgePackageVersion, 'compatibility.runtimeBridgePackageVersion');
        if (config.runtimeEntry !== config.resourceProfile.runtimeEntry) {
            throw new RuntimeBridgeError('invalid_input', 'runtimeEntry must match resourceProfile.runtimeEntry');
        }
        const profile = this.options.profile ?? nativeBackendProfile(config);
        const validation = validateGameRuntimeBackendProfile(profile);
        if (!validation.ok) {
            throw new RuntimeBridgeError('invalid_input', validation.diagnostics.map((diagnostic) => diagnostic.message).join('; '));
        }
        if (validation.profile.mode !== 'native') {
            throw new RuntimeBridgeError('invalid_input', 'selected backend launcher currently supports native mode only');
        }
        const bridge = this.options.bridgeFactory?.() ?? await createNativeBridgeForSelectedBackend(this.options.nativeModulePath);
        bridge.initializeEngine({ seed: config.world.sceneId });
        const status = bridge.loadWorldBundle(config.world);
        if (status.blocksLoad || status.loadedWorld === null) {
            throw new RuntimeBridgeError('invalid_input', 'world bundle failed to load for selected backend launcher');
        }
        return new ReferenceGameRuntimeSession(bridge, config, runtimeProfileFromBackendProfile(validation.profile), status);
    }
}
export function createSelectedBackendGameRuntimeLauncher(options = {}) {
    return new SelectedBackendGameRuntimeLauncher(options);
}
export function createNativeGameRuntimeLauncher(options = {}) {
    return createSelectedBackendGameRuntimeLauncher(options);
}
async function createNativeBridgeForSelectedBackend(nativeModulePath) {
    const nativeModule = await import('./native.js');
    return nativeModule.createNativeRuntimeBridge(nativeModulePath);
}
//# sourceMappingURL=launcher.js.map