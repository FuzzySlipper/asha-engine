import type { CommandBatch, CommandResult, RenderFrameDiff } from '@asha/contracts';
import {
  RuntimeBridgeError,
  frameCursor,
  type CompositionStatus,
  type FrameCursor,
  type RuntimeBridge,
  type WorldLoadRequest,
} from './bridge.js';
import { createMockRuntimeBridge } from './mock.js';

// ── Game runtime launcher facade types (#3653) ────────────────────────────────
// Higher-level public types for game consumers. These describe launch/session
// read models without granting access to raw transports or private runtime code.

export type GameRuntimeMode = 'reference' | 'native' | 'wasm' | 'degraded';

export type GameRuntimeBackendMode = 'reference' | 'native' | 'wasm';

export type GameRuntimeBackendTransport = 'reference_mock' | 'napi_native' | 'wasm_module';

export type GameRuntimeNonClaim =
  | 'not_native_runtime'
  | 'not_hardware_gpu'
  | 'not_performance_evidence'
  | 'not_publish_artifact'
  | 'not_product_authority'
  | 'not_wasm_authority';

export type GameRuntimeDiagnosticCode =
  | 'missing_compatibility'
  | 'missing_world_bundle'
  | 'unsupported_runtime_entry'
  | 'unsupported_backend_mode'
  | 'missing_backend_evidence'
  | 'private_transport_hint'
  | 'backend_claim_mismatch'
  | 'runtime_unavailable'
  | 'operation_unimplemented'
  | 'command_rejected'
  | 'stale_sequence'
  | 'stale_readback'
  | 'internal';

export interface GameRuntimeDiagnostic {
  readonly code: GameRuntimeDiagnosticCode;
  readonly severity: 'info' | 'warning' | 'error';
  readonly message: string;
}

export interface GameRuntimeCompatibility {
  readonly contractsPackageVersion: string;
  readonly runtimeBridgePackageVersion: string;
  readonly devtoolsProtocolVersion?: string;
  readonly publishArtifactVersion?: string;
}

export interface GameRuntimeProfile {
  readonly profileId: string;
  readonly runtimeMode: GameRuntimeMode;
  readonly launcherName: string;
  readonly bridgeCompatibility: GameRuntimeCompatibility;
  readonly nonClaims: readonly GameRuntimeNonClaim[];
}

export interface GameRuntimeBackendProfile {
  readonly profileId: string;
  readonly mode: GameRuntimeBackendMode;
  readonly transport: GameRuntimeBackendTransport;
  readonly launcherName: string;
  readonly bridgeCompatibility: GameRuntimeCompatibility;
  readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
  readonly nonClaims: readonly GameRuntimeNonClaim[];
}

export type GameRuntimeBackendProfileValidation =
  | {
      readonly ok: true;
      readonly profile: GameRuntimeBackendProfile;
      readonly diagnostics: readonly [];
    }
  | {
      readonly ok: false;
      readonly diagnostics: readonly GameRuntimeDiagnostic[];
    };

export interface GameRuntimeResourceProfile {
  readonly profileId: string;
  readonly runtimeEntry: string;
  readonly worldBundleId: string;
  readonly resourceManifestHash?: string;
  readonly estimatedBytes?: number;
}

export interface GameRuntimeEvidenceRef {
  readonly kind: 'projection' | 'render_diff' | 'replay' | 'evidence_export' | 'telemetry' | 'diagnostic';
  readonly id: string;
  readonly path?: string;
  readonly sha256?: string;
  readonly sequenceId?: number;
}

export interface GameRuntimeConfig {
  readonly gameId: string;
  readonly workspaceId: string;
  readonly runtimeEntry: string;
  readonly compatibility: GameRuntimeCompatibility;
  readonly resourceProfile: GameRuntimeResourceProfile;
  readonly world: WorldLoadRequest;
  readonly startedAtIso?: string;
}

export interface GameRuntimeIdentity {
  readonly gameId: string;
  readonly workspaceId: string;
  readonly runtimeMode: GameRuntimeMode;
  readonly runtimeEntry: string;
  readonly startedAtIso: string;
  readonly compatibility: GameRuntimeCompatibility;
  readonly nonClaims: readonly GameRuntimeNonClaim[];
}

export interface GameRuntimeProjectionSummary {
  readonly sequenceId: number;
  readonly worldHash: string;
  readonly authorityHash: string;
  readonly loadedWorld: number | null;
  readonly fatalCount: number;
  readonly totalDiagnosticCount: number;
  readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}

export interface GameRuntimeLaunchResult {
  readonly status: 'launched' | 'degraded' | 'failed';
  readonly identity: GameRuntimeIdentity;
  readonly runtimeProfile: GameRuntimeProfile;
  readonly resourceProfile: GameRuntimeResourceProfile;
  readonly projection: GameRuntimeProjectionSummary;
  readonly diagnostics: readonly GameRuntimeDiagnostic[];
  readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}

export interface GameRuntimeCommandProposalResult {
  readonly sequenceId: number;
  readonly status: 'accepted' | 'rejected' | 'failed';
  readonly batch: CommandBatch;
  readonly result: CommandResult | null;
  readonly authorityHashBefore: string;
  readonly authorityHashAfter: string;
  readonly diagnostics: readonly GameRuntimeDiagnostic[];
  readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}

export interface GameRuntimeRenderDiffSnapshot {
  readonly sequenceId: number;
  readonly cursor: FrameCursor;
  readonly frame: RenderFrameDiff;
  readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}

export interface GameRuntimeTelemetrySnapshot {
  readonly sequenceId: number;
  readonly runtimeMode: GameRuntimeMode;
  readonly acceptedCommandCount: number;
  readonly rejectedCommandCount: number;
  readonly diagnostics: readonly GameRuntimeDiagnostic[];
  readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}

export interface GameRuntimeReplayExportRequest {
  readonly replayId: string;
}

export interface GameRuntimeReplayExport {
  readonly replayId: string;
  readonly sequenceId: number;
  readonly authorityHash: string;
  readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}

export interface GameRuntimeEvidenceExportRequest {
  readonly evidenceId: string;
}

export interface GameRuntimeEvidenceExport {
  readonly evidenceId: string;
  readonly sequenceId: number;
  readonly projection: GameRuntimeProjectionSummary;
  readonly nonClaims: readonly GameRuntimeNonClaim[];
  readonly evidenceRefs: readonly GameRuntimeEvidenceRef[];
}

export interface GameRuntimeSession {
  readonly launch: GameRuntimeLaunchResult;
  readonly identity: GameRuntimeIdentity;
  pullProjection(): Promise<GameRuntimeProjectionSummary>;
  pullRenderDiff(cursor?: FrameCursor): Promise<GameRuntimeRenderDiffSnapshot>;
  pullTelemetry(): Promise<GameRuntimeTelemetrySnapshot>;
  proposeCommands(batch: CommandBatch): Promise<GameRuntimeCommandProposalResult>;
  exportReplay(request: GameRuntimeReplayExportRequest): Promise<GameRuntimeReplayExport>;
  exportEvidence(request: GameRuntimeEvidenceExportRequest): Promise<GameRuntimeEvidenceExport>;
  shutdown(): Promise<void>;
}

export interface GameRuntimeLauncher {
  readonly mode: GameRuntimeMode;
  launch(config: GameRuntimeConfig): Promise<GameRuntimeSession>;
}

type GameRuntimeProfileValue =
  | object
  | string
  | number
  | undefined;

interface GameRuntimeProfileRecord {
  readonly [key: string]: GameRuntimeProfileValue;
  readonly bridgeCompatibility?: GameRuntimeProfileValue;
  readonly contractsPackageVersion?: GameRuntimeProfileValue;
  readonly devtoolsProtocolVersion?: GameRuntimeProfileValue;
  readonly evidenceRefs?: GameRuntimeProfileValue;
  readonly id?: GameRuntimeProfileValue;
  readonly kind?: GameRuntimeProfileValue;
  readonly launcherName?: GameRuntimeProfileValue;
  readonly mode?: GameRuntimeProfileValue;
  readonly nonClaims?: GameRuntimeProfileValue;
  readonly path?: GameRuntimeProfileValue;
  readonly profileId?: GameRuntimeProfileValue;
  readonly publishArtifactVersion?: GameRuntimeProfileValue;
  readonly runtimeBridgePackageVersion?: GameRuntimeProfileValue;
  readonly sequenceId?: GameRuntimeProfileValue;
  readonly sha256?: GameRuntimeProfileValue;
  readonly transport?: GameRuntimeProfileValue;
}

function requireNonEmpty(value: string, field: string): string {
  if (value.trim().length === 0) {
    throw new RuntimeBridgeError('invalid_input', `${field} must be a non-empty string`);
  }
  return value;
}

function referenceNonClaims(): readonly GameRuntimeNonClaim[] {
  return [
    'not_native_runtime',
    'not_hardware_gpu',
    'not_performance_evidence',
    'not_publish_artifact',
    'not_product_authority',
    'not_wasm_authority',
  ];
}

function selectedNativeNonClaims(): readonly GameRuntimeNonClaim[] {
  return ['not_hardware_gpu', 'not_performance_evidence', 'not_publish_artifact', 'not_wasm_authority'];
}

function backendProfileDiagnostic(
  code: GameRuntimeDiagnosticCode,
  message: string,
  severity: GameRuntimeDiagnostic['severity'] = 'error',
): GameRuntimeDiagnostic {
  return { code, severity, message };
}

function isPlainRecord(value: GameRuntimeProfileValue): value is GameRuntimeProfileRecord {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function hasOnlyKeys(value: GameRuntimeProfileRecord, keys: readonly string[]): boolean {
  const allowed = new Set(keys);
  return Object.keys(value).every((key) => allowed.has(key));
}

function containsPrivateTransportHint(value: string): boolean {
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

function isGameRuntimeEvidenceRef(value: GameRuntimeProfileValue): value is GameRuntimeEvidenceRef {
  if (!isPlainRecord(value) || !hasOnlyKeys(value, ['kind', 'id', 'path', 'sha256', 'sequenceId'])) {
    return false;
  }
  return (
    (value.kind === 'projection'
      || value.kind === 'render_diff'
      || value.kind === 'replay'
      || value.kind === 'evidence_export'
      || value.kind === 'telemetry'
      || value.kind === 'diagnostic')
    && typeof value.id === 'string'
    && (value.path === undefined || typeof value.path === 'string')
    && (value.sha256 === undefined || typeof value.sha256 === 'string')
    && (value.sequenceId === undefined || Number.isInteger(value.sequenceId))
  );
}

function isGameRuntimeCompatibility(value: GameRuntimeProfileValue): value is GameRuntimeCompatibility {
  return isPlainRecord(value)
    && typeof value.contractsPackageVersion === 'string'
    && typeof value.runtimeBridgePackageVersion === 'string'
    && (value.devtoolsProtocolVersion === undefined || typeof value.devtoolsProtocolVersion === 'string')
    && (value.publishArtifactVersion === undefined || typeof value.publishArtifactVersion === 'string');
}

function isGameRuntimeNonClaim(value: GameRuntimeProfileValue): value is GameRuntimeNonClaim {
  return value === 'not_native_runtime'
    || value === 'not_hardware_gpu'
    || value === 'not_performance_evidence'
    || value === 'not_publish_artifact'
    || value === 'not_product_authority'
    || value === 'not_wasm_authority';
}

export function validateGameRuntimeBackendProfile(
  input: object,
): GameRuntimeBackendProfileValidation {
  const diagnostics: GameRuntimeDiagnostic[] = [];
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
  } else if (containsPrivateTransportHint(input.profileId)) {
    diagnostics.push(backendProfileDiagnostic('private_transport_hint', 'profileId must not contain private transport hints'));
  }
  if (typeof input.launcherName !== 'string' || input.launcherName.trim().length === 0) {
    diagnostics.push(backendProfileDiagnostic('missing_backend_evidence', 'launcherName must be a non-empty string'));
  } else if (containsPrivateTransportHint(input.launcherName)) {
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
    if (
      !nonClaims.includes('not_native_runtime')
      || !nonClaims.includes('not_product_authority')
      || !nonClaims.includes('not_wasm_authority')
    ) {
      diagnostics.push(backendProfileDiagnostic('backend_claim_mismatch', 'reference mode must preserve native/product/WASM non-claims'));
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
    if (nonClaims.includes('not_product_authority')) {
      diagnostics.push(backendProfileDiagnostic('backend_claim_mismatch', 'native mode cannot carry not_product_authority'));
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
    profile: input as unknown as GameRuntimeBackendProfile,
    diagnostics: [],
  };
}

export function referenceBackendProfile(config: GameRuntimeConfig): GameRuntimeBackendProfile {
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

export function nativeBackendProfile(config: GameRuntimeConfig): GameRuntimeBackendProfile {
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

function referenceRuntimeProfile(config: GameRuntimeConfig): GameRuntimeProfile {
  const profile = referenceBackendProfile(config);
  return {
    profileId: profile.profileId,
    runtimeMode: profile.mode,
    launcherName: profile.launcherName,
    bridgeCompatibility: profile.bridgeCompatibility,
    nonClaims: profile.nonClaims,
  };
}

function projectionSummary(
  config: GameRuntimeConfig,
  runtimeMode: GameRuntimeMode,
  status: CompositionStatus,
  sequenceId: number,
  acceptedCommandCount: number,
): GameRuntimeProjectionSummary {
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

class ReferenceGameRuntimeSession implements GameRuntimeSession {
  readonly identity: GameRuntimeIdentity;
  readonly launch: GameRuntimeLaunchResult;
  readonly #runtimeProfile: GameRuntimeProfile;
  #sequenceId = 0;
  #acceptedCommandCount = 0;
  #rejectedCommandCount = 0;
  #shutdown = false;

  constructor(
    private readonly bridge: RuntimeBridge,
    private readonly config: GameRuntimeConfig,
    runtimeProfile: GameRuntimeProfile,
    initialStatus: CompositionStatus,
  ) {
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

  async pullProjection(): Promise<GameRuntimeProjectionSummary> {
    this.#assertOpen();
    return projectionSummary(this.config, this.#runtimeProfile.runtimeMode, this.bridge.getCompositionStatus(), this.#sequenceId, this.#acceptedCommandCount);
  }

  async pullRenderDiff(cursor: FrameCursor = frameCursor(0)): Promise<GameRuntimeRenderDiffSnapshot> {
    this.#assertOpen();
    const frame = this.bridge.readRenderDiffs(cursor);
    return {
      sequenceId: this.#sequenceId,
      cursor,
      frame,
      evidenceRefs: [{ kind: 'render_diff', id: `render-diff:${cursor as number}`, sequenceId: this.#sequenceId }],
    };
  }

  async pullTelemetry(): Promise<GameRuntimeTelemetrySnapshot> {
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

  async proposeCommands(batch: CommandBatch): Promise<GameRuntimeCommandProposalResult> {
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
          code: 'command_rejected' as const,
          severity: 'warning' as const,
          message: `command ${index} rejected: ${rejection.reason}`,
        })),
        evidenceRefs: [{ kind: 'replay', id: `command:${this.#sequenceId}`, sequenceId: this.#sequenceId }],
      };
    } catch (cause) {
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

  async exportReplay(request: GameRuntimeReplayExportRequest): Promise<GameRuntimeReplayExport> {
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

  async exportEvidence(request: GameRuntimeEvidenceExportRequest): Promise<GameRuntimeEvidenceExport> {
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

  async shutdown(): Promise<void> {
    if (this.#shutdown) return;
    this.bridge.unloadWorld();
    this.#shutdown = true;
  }

  #assertOpen(): void {
    if (this.#shutdown) {
      throw new RuntimeBridgeError('not_initialized', 'game runtime session has been shut down');
    }
  }
}

export class ReferenceGameRuntimeLauncher implements GameRuntimeLauncher {
  readonly mode = 'reference';

  async launch(config: GameRuntimeConfig): Promise<GameRuntimeSession> {
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

export function createReferenceGameRuntimeLauncher(): GameRuntimeLauncher {
  return new ReferenceGameRuntimeLauncher();
}

function runtimeProfileFromBackendProfile(profile: GameRuntimeBackendProfile): GameRuntimeProfile {
  return {
    profileId: profile.profileId,
    runtimeMode: profile.mode,
    launcherName: profile.launcherName,
    bridgeCompatibility: profile.bridgeCompatibility,
    nonClaims: profile.nonClaims,
  };
}

export interface SelectedBackendLauncherOptions {
  readonly profile?: GameRuntimeBackendProfile;
  readonly nativeModulePath?: string;
  readonly bridgeFactory?: () => RuntimeBridge;
}

export class SelectedBackendGameRuntimeLauncher implements GameRuntimeLauncher {
  readonly mode: GameRuntimeMode;

  constructor(private readonly options: SelectedBackendLauncherOptions = {}) {
    this.mode = options.profile?.mode ?? 'native';
  }

  async launch(config: GameRuntimeConfig): Promise<GameRuntimeSession> {
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
    if (validation.profile.mode === 'reference') {
      throw new RuntimeBridgeError('invalid_input', 'selected backend launcher cannot use reference_mock as product authority');
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
    return new ReferenceGameRuntimeSession(
      bridge,
      config,
      runtimeProfileFromBackendProfile(validation.profile),
      status,
    );
  }
}

export function createSelectedBackendGameRuntimeLauncher(
  options: SelectedBackendLauncherOptions = {},
): GameRuntimeLauncher {
  return new SelectedBackendGameRuntimeLauncher(options);
}

export function createNativeGameRuntimeLauncher(
  options: SelectedBackendLauncherOptions = {},
): GameRuntimeLauncher {
  return createSelectedBackendGameRuntimeLauncher(options);
}

async function createNativeBridgeForSelectedBackend(nativeModulePath?: string): Promise<RuntimeBridge> {
  const nativeModule = await import('./native.js');
  return nativeModule.createNativeRuntimeBridge(nativeModulePath);
}
