import type {
  CameraCollisionSnapshot,
  CameraProjectionSnapshot,
  CameraSnapshot,
  CommandBatch,
  CommandResult,
  ModelMaterialPreviewRequest,
  ModelMaterialPreviewSnapshot,
  PickResult,
  RenderFrameDiff,
  SceneObjectCommandResult,
  SceneObjectSnapshot,
  VoxelConversionApplyRequest,
  VoxelConversionEvidenceRef,
  VoxelConversionPlan,
  VoxelConversionPlanRequest,
  VoxelConversionPreview,
  VoxelConversionPreviewRequest,
  VoxelConversionReceipt,
  VoxelConversionSourceRegistration,
  VoxelConversionSourceRegistrationRequest,
  VoxelSelectionSnapshot,
  VoxelModelInfoReadout,
  VoxelModelInfoRequest,
  GameExtensionHookReceipt,
  GameExtensionReplayEvidence,
  GameRuleCatalog,
  GameRuleResolutionReceipt,
  GameRuleResolutionRequest,
} from '@asha/contracts';
import { loadNativeAddon, NativeAddonUnavailable, type NativeAddon } from '@asha/native-bridge';
import { MANIFEST_OPERATIONS } from './generated/operations.js';
import {
  RuntimeBridgeError,
  nonNegativeSafeInteger,
  u32,
  type CompositionStatus,
  type EnemyDirectNavMovementRequest,
  type EnemyDirectNavMovementResult,
  type EngineConfig,
  type EngineHandle,
  type FrameCursor,
  type FpsEncounterDirectorSnapshot,
  type FpsEncounterLifecycleInput,
  type FpsEncounterTransitionRequest,
  type FpsEncounterTransitionResult,
  type GameExtensionWeaponEffectInvocationRequest,
  type GameExtensionWeaponEffectInvocationResult,
  type GameRuleCatalogValidationReceipt,
  type GameRuleEffectIntentRequest,
  type GameRuleRuntimeReadout,
  type FpsLifecycleStatus,
  type FpsPrimaryFireRequest,
  type FpsPrimaryFireResult,
  type FpsRuntimeAuthorityTransport,
  type FpsRuntimeRole,
  type FpsRuntimeSessionLoadRequest,
  type FpsRuntimeSessionRestartRequest,
  type FpsRuntimeSessionSnapshot,
  type ReplaySessionHandle,
  type ReplayStepReport,
  type RuntimeBridge,
  type RuntimeBridgeErrorKind,
  type RuntimeBufferView,
  type StepInputEnvelope,
  type StepResult,
  type VoxelMeshEvidenceSnapshot,
  type WorldLoadRequest,
  type WorldSaveSummary,
} from './bridge.js';

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
export const NATIVE_WIRED_OPERATIONS: ReadonlySet<string> = new Set<string>([
  'initialize_engine',
  'load_world_bundle',
  'submit_commands',
  'step_simulation',
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
  'preview_voxel_conversion',
  'apply_voxel_conversion',
  'export_voxel_conversion_evidence',
  'read_voxel_model_info',
  'read_render_diffs',
  'save_current_world',
  'get_composition_status',
]);

function nativeUnimplemented(manifestName: string): RuntimeBridgeError {
  return new RuntimeBridgeError(
    'operation_unimplemented',
    `native bridge operation '${manifestName}' is not wired; the native facade is ` +
      `fail-closed (no mock fallback). Wire its #[napi] export and add it to ` +
      `NATIVE_WIRED_OPERATIONS.`,
  );
}

const RUST_ERROR_KIND: Readonly<Record<string, RuntimeBridgeErrorKind>> = {
  NotInitialized: 'not_initialized',
  InvalidInput: 'invalid_input',
  UnknownHandle: 'unknown_handle',
  BufferExpired: 'buffer_expired',
  Internal: 'internal',
};

function classifyNativeAddonError(cause: RuntimeBridgeError | Error | string | object): RuntimeBridgeError {
  if (cause instanceof RuntimeBridgeError) return cause;
  const message = cause instanceof Error ? cause.message : String(cause);
  const match = /^(\w+):\s*(.*)$/u.exec(message);
  if (match?.[1]) {
    const kind = RUST_ERROR_KIND[match[1]];
    if (kind) return new RuntimeBridgeError(kind, match[2] || message);
  }
  return new RuntimeBridgeError('internal', message);
}

function callNative<T>(body: () => T): T {
  try {
    return body();
  } catch (cause) {
    throw classifyNativeAddonError(cause as RuntimeBridgeError | Error | string | object);
  }
}

function parseNativeJson<T>(payload: string, field: string): T {
  try {
    return JSON.parse(payload) as T;
  } catch (cause) {
    const reason = cause instanceof Error ? cause.message : String(cause);
    throw new RuntimeBridgeError('internal', `native ${field} was not valid JSON: ${reason}`);
  }
}

function nativeVec3(value: readonly [number, number, number], field: string): { readonly x: number; readonly y: number; readonly z: number } {
  if (value.length !== 3 || value.some((component) => !Number.isFinite(component))) {
    throw new RuntimeBridgeError('invalid_input', `${field} must be a finite vec3`);
  }
  return { x: value[0], y: value[1], z: value[2] };
}

function nativeOptionalObject<T extends object>(value: T | null): T | undefined {
  return value === null ? undefined : value;
}

function bridgeVec3(
  value: { readonly x: number; readonly y: number; readonly z: number },
  field: string,
): readonly [number, number, number] {
  if (!Number.isFinite(value.x) || !Number.isFinite(value.y) || !Number.isFinite(value.z)) {
    throw new RuntimeBridgeError('internal', `native ${field} was not a finite vec3`);
  }
  return [value.x, value.y, value.z];
}

function nativeAuthoritySource(value: string): 'seeded_from_request' | 'rust_entity_store' {
  if (value === 'seeded_from_request' || value === 'rust_entity_store') {
    return value;
  }
  throw new RuntimeBridgeError('internal', `unknown native enemy movement authority source '${value}'`);
}

function fpsBackend(value: string): FpsRuntimeAuthorityTransport {
  if (value === 'native_rust' || value === 'reference_bridge') {
    return value;
  }
  // The Rust reference bridge reports reference_bridge_rust internally; the TS
  // native facade classifies the transport path as native_rust.
  if (value === 'reference_bridge_rust') {
    return 'native_rust';
  }
  throw new RuntimeBridgeError('internal', `unknown native FPS backend '${value}'`);
}

function fpsRole(value: FpsRuntimeRole): FpsRuntimeRole {
  if (value === 'player' || value === 'enemy' || value === 'neutral') return value;
  throw new RuntimeBridgeError('invalid_input', `unknown FPS role '${String(value)}'`);
}

function fpsLifecycleStatus(value: { readonly state: string; readonly entity?: number; readonly tick?: number }): FpsLifecycleStatus {
  if (value.state === 'active') return { state: 'active' };
  if (value.state === 'enemy_defeated') {
    return {
      state: 'enemy_defeated',
      entity: nonNegativeSafeInteger(value.entity ?? -1, 'lifecycleStatus.entity'),
      tick: nonNegativeSafeInteger(value.tick ?? -1, 'lifecycleStatus.tick'),
    };
  }
  throw new RuntimeBridgeError('internal', `unknown native FPS lifecycle status '${value.state}'`);
}

function hashString(value: string, field: string): string {
  if (!/^fnv1a64:[0-9a-f]{16}$/u.test(value)) {
    throw new RuntimeBridgeError('internal', `native ${field} was not an fnv1a64 hash`);
  }
  return value;
}

function normalizeFpsSnapshot(value: FpsRuntimeSessionSnapshot): FpsRuntimeSessionSnapshot {
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

function normalizeEncounterSnapshot(value: FpsEncounterDirectorSnapshot): FpsEncounterDirectorSnapshot {
  return {
    ...value,
    backend: fpsBackend(value.backend),
    encounterHash: hashString(value.encounterHash, 'encounterHash'),
    replayHash: hashString(value.replayHash, 'replayHash'),
  };
}

function normalizeEncounterTransition(value: FpsEncounterTransitionResult): FpsEncounterTransitionResult {
  return {
    ...value,
    backend: fpsBackend(value.backend),
    encounterHash: hashString(value.encounterHash, 'encounterHash'),
    replayHash: hashString(value.replayHash, 'replayHash'),
  };
}

function nativeFpsLoadRequest(request: FpsRuntimeSessionLoadRequest) {
  if (request.projectBundle.trim() === '') {
    throw new RuntimeBridgeError('invalid_input', 'projectBundle is required');
  }
  if (request.definitions.length === 0) {
    throw new RuntimeBridgeError('invalid_input', 'definitions must not be empty');
  }
  const definitions = request.definitions.map((definition, index) => {
    nonNegativeSafeInteger(definition.entity, `definitions[${index}].entity`);
    fpsRole(definition.role);
    const transform = definition.transform === null
      ? null
      : {
          translation: nativeVec3(definition.transform.translation, `definitions[${index}].transform.translation`),
          rotation: definition.transform.rotation,
          scale: nativeVec3(definition.transform.scale, `definitions[${index}].transform.scale`),
        };
    if (definition.transform !== null) {
      if (definition.transform.rotation.length !== 4 || definition.transform.rotation.some((value) => !Number.isFinite(value))) {
        throw new RuntimeBridgeError('invalid_input', `definitions[${index}].transform.rotation must be a finite quat`);
      }
    }
    const bounds = definition.bounds === null
      ? null
      : {
          min: nativeVec3(definition.bounds.min, `definitions[${index}].bounds.min`),
          max: nativeVec3(definition.bounds.max, `definitions[${index}].bounds.max`),
        };
    if (definition.bounds !== null) {
    }
    if (definition.health !== null) {
      u32(definition.health.current, `definitions[${index}].health.current`);
      u32(definition.health.max, `definitions[${index}].health.max`);
    }
    if (definition.weapon !== null) {
      u32(definition.weapon.damage, `definitions[${index}].weapon.damage`);
      u32(definition.weapon.rangeUnits, `definitions[${index}].weapon.rangeUnits`);
      u32(definition.weapon.ammo, `definitions[${index}].weapon.ammo`);
      u32(definition.weapon.cooldownTicksAfterFire, `definitions[${index}].weapon.cooldownTicksAfterFire`);
    }
    return {
      entity: definition.entity,
      stableId: definition.stableId,
      displayName: definition.displayName,
      sourcePath: definition.sourcePath,
      role: definition.role,
      transform: nativeOptionalObject(transform),
      bounds: nativeOptionalObject(bounds),
      tags: [...definition.tags],
      renderVisible: definition.renderVisible,
      staticCollider: definition.staticCollider,
      health: nativeOptionalObject(definition.health),
      weapon: definition.weapon === null
        ? undefined
        : {
            weaponId: definition.weapon.weaponId,
            damage: definition.weapon.damage,
            rangeUnits: definition.weapon.rangeUnits,
            ammo: definition.weapon.ammo,
            cooldownTicksAfterFire: definition.weapon.cooldownTicksAfterFire,
          },
      policyBinding: definition.policyBinding === null
        ? undefined
        : {
            ...definition.policyBinding,
            allowedIntents: [...definition.policyBinding.allowedIntents],
          },
    };
  });
  return { projectBundle: request.projectBundle, definitions };
}

export class NativeRuntimeBridge implements RuntimeBridge {
  readonly #addon: NativeAddon;
  #seed = 0;
  #initialized = false;

  #engineHandle: EngineHandle | null = null;

  constructor(addon: NativeAddon) {
    this.#addon = addon;
  }

  // ── Wired native operations ───────────────────────────────────────────────
  initializeEngine(config: EngineConfig): EngineHandle {
    if (!Number.isInteger(config.seed) || config.seed < 0) {
      throw new RuntimeBridgeError('invalid_input', `seed must be a non-negative integer`);
    }
    this.#seed = config.seed;
    const handle = this.#addon.initializeEngine(config.seed) as EngineHandle;
    this.#engineHandle = handle;
    this.#initialized = true;
    return handle;
  }

  #requireHandle(operation: string): EngineHandle {
    if (!this.#initialized || this.#engineHandle === null) {
      throw new RuntimeBridgeError('not_initialized', `${operation} before initializeEngine`);
    }
    return this.#engineHandle;
  }

  loadWorldBundle(request: WorldLoadRequest): CompositionStatus {
    const handle = this.#requireHandle('loadWorldBundle');
    const bundleSchemaVersion = u32(request.bundleSchemaVersion, 'bundleSchemaVersion');
    const protocolVersion = u32(request.protocolVersion, 'protocolVersion');
    const sceneId = nonNegativeSafeInteger(request.sceneId, 'sceneId');
    return callNative(() =>
      this.#addon.loadWorldBundle(handle, bundleSchemaVersion, protocolVersion, sceneId) as CompositionStatus,
    );
  }

  submitCommands(batch: CommandBatch): CommandResult {
    const handle = this.#requireHandle('submitCommands');
    return callNative(() => this.#addon.submitCommands(handle, JSON.stringify(batch.commands)) as CommandResult);
  }

  stepSimulation(input: StepInputEnvelope): StepResult {
    const handle = this.#requireHandle('stepSimulation');
    const tick = nonNegativeSafeInteger(input.tick, 'tick');
    const diffCount = callNative(() => this.#addon.stepSimulation(handle, tick));
    return { tick, diffCount };
  }

  applyEnemyDirectNavMovement(request: EnemyDirectNavMovementRequest): EnemyDirectNavMovementResult {
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
    const result = callNative(() =>
      this.#addon.applyEnemyDirectNavMovement(
        handle,
        entity,
        seedPosition,
        target,
        request.maxStepUnits,
      ),
    );
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

  loadFpsRuntimeSession(request: FpsRuntimeSessionLoadRequest): FpsRuntimeSessionSnapshot {
    const handle = this.#requireHandle('loadFpsRuntimeSession');
    const nativeRequest = nativeFpsLoadRequest(request);
    const result = callNative(() =>
      this.#addon.loadFpsRuntimeSession(
        handle,
        nativeRequest.projectBundle,
        nativeRequest.definitions,
        JSON.stringify(request.gameRuleModules),
      ) as FpsRuntimeSessionSnapshot,
    );
    return normalizeFpsSnapshot(result);
  }

  readFpsRuntimeSession(): FpsRuntimeSessionSnapshot {
    const handle = this.#requireHandle('readFpsRuntimeSession');
    const result = callNative(() => this.#addon.readFpsRuntimeSession(handle) as FpsRuntimeSessionSnapshot);
    return normalizeFpsSnapshot(result);
  }

  applyFpsPrimaryFire(request: FpsPrimaryFireRequest): FpsPrimaryFireResult {
    const handle = this.#requireHandle('applyFpsPrimaryFire');
    const tick = nonNegativeSafeInteger(request.tick, 'tick');
    const origin = nativeVec3(request.origin, 'origin');
    const direction = nativeVec3(request.direction, 'direction');
    const result = callNative(() =>
      this.#addon.applyFpsPrimaryFire(handle, tick, origin, direction) as FpsPrimaryFireResult,
    );
    return {
      ...result,
      backend: fpsBackend(result.backend),
      lifecycleStatus: fpsLifecycleStatus(result.lifecycleStatus),
      entityHash: hashString(result.entityHash, 'entityHash'),
      healthHash: hashString(result.healthHash, 'healthHash'),
      replayHash: hashString(result.replayHash, 'replayHash'),
    };
  }

  invokeGameExtensionWeaponEffect(
    request: GameExtensionWeaponEffectInvocationRequest,
  ): GameExtensionWeaponEffectInvocationResult {
    const handle = this.#requireHandle('invokeGameExtensionWeaponEffect');
    const tick = nonNegativeSafeInteger(request.primaryFire.tick, 'primaryFire.tick');
    const origin = nativeVec3(request.primaryFire.origin, 'primaryFire.origin');
    const direction = nativeVec3(request.primaryFire.direction, 'primaryFire.direction');
    const result = callNative(() =>
      this.#addon.invokeGameExtensionWeaponEffect(
        handle,
        JSON.stringify(request.hook),
        tick,
        origin,
        direction,
      ),
    ) as {
      readonly hookReceiptJson: string;
      readonly replayEvidenceJson: string;
      readonly primaryFire?: FpsPrimaryFireResult | null;
    };
    return {
      hookReceipt: parseNativeJson<GameExtensionHookReceipt>(result.hookReceiptJson, 'game extension hook receipt'),
      replayEvidence: parseNativeJson<GameExtensionReplayEvidence>(
        result.replayEvidenceJson,
        'game extension replay evidence',
      ),
      primaryFire: result.primaryFire === undefined || result.primaryFire === null
        ? null
        : {
            ...result.primaryFire,
            backend: fpsBackend(result.primaryFire.backend),
            lifecycleStatus: fpsLifecycleStatus(result.primaryFire.lifecycleStatus),
            entityHash: hashString(result.primaryFire.entityHash, 'entityHash'),
            healthHash: hashString(result.primaryFire.healthHash, 'healthHash'),
            replayHash: hashString(result.primaryFire.replayHash, 'replayHash'),
          },
    };
  }

  validateGameRuleCatalog(catalog: GameRuleCatalog): GameRuleCatalogValidationReceipt {
    const handle = this.#requireHandle('validateGameRuleCatalog');
    return parseNativeJson<GameRuleCatalogValidationReceipt>(
      callNative(() => this.#addon.validateGameRuleCatalog(handle, JSON.stringify(catalog))),
      'game-rule catalog validation receipt',
    );
  }

  submitGameRuleEffectIntent(input: GameRuleEffectIntentRequest): GameRuleResolutionReceipt {
    const handle = this.#requireHandle('submitGameRuleEffectIntent');
    return parseNativeJson<GameRuleResolutionReceipt>(
      callNative(() =>
        this.#addon.submitGameRuleEffectIntent(
          handle,
          JSON.stringify(input.catalog),
          JSON.stringify(input.request),
        )),
      'game-rule resolution receipt',
    );
  }

  readGameRuleRuntimeReadout(): GameRuleRuntimeReadout {
    const handle = this.#requireHandle('readGameRuleRuntimeReadout');
    const readout = parseNativeJson<GameRuleRuntimeReadout>(
      callNative(() => this.#addon.readGameRuleRuntimeReadout(handle)),
      'game-rule runtime readout',
    );
    return { ...readout, backend: fpsBackend(readout.backend) };
  }

  restartFpsRuntimeSession(request: FpsRuntimeSessionRestartRequest): FpsRuntimeSessionSnapshot {
    const handle = this.#requireHandle('restartFpsRuntimeSession');
    const expectedEpoch = nonNegativeSafeInteger(request.expectedEpoch, 'expectedEpoch');
    const result = callNative(() =>
      this.#addon.restartFpsRuntimeSession(handle, expectedEpoch) as FpsRuntimeSessionSnapshot,
    );
    return normalizeFpsSnapshot(result);
  }

  readFpsEncounterDirector(lifecycle: FpsEncounterLifecycleInput): FpsEncounterDirectorSnapshot {
    const handle = this.#requireHandle('readFpsEncounterDirector');
    const result = callNative(() =>
      this.#addon.readFpsEncounterDirector(handle, lifecycle) as FpsEncounterDirectorSnapshot,
    );
    return normalizeEncounterSnapshot(result);
  }

  applyFpsEncounterTransition(request: FpsEncounterTransitionRequest): FpsEncounterTransitionResult {
    const handle = this.#requireHandle('applyFpsEncounterTransition');
    const result = callNative(() =>
      this.#addon.applyFpsEncounterTransition(handle, request) as FpsEncounterTransitionResult,
    );
    return normalizeEncounterTransition(result);
  }

  readModelMaterialPreview(request: ModelMaterialPreviewRequest): ModelMaterialPreviewSnapshot {
    void request;
    throw nativeUnimplemented('read_model_material_preview');
  }

  readSceneObjectSnapshot(): SceneObjectSnapshot {
    throw nativeUnimplemented('read_scene_object_snapshot');
  }

  applySceneObjectCommand(): SceneObjectCommandResult {
    throw nativeUnimplemented('apply_scene_object_command');
  }

  readRenderDiffs(cursor: FrameCursor): RenderFrameDiff {
    const handle = this.#requireHandle('readRenderDiffs');
    const frame = nonNegativeSafeInteger(cursor as number, 'frame cursor') as FrameCursor;
    return callNative(() => this.#addon.readRenderDiffs(handle, frame) as RenderFrameDiff);
  }

  saveCurrentWorld(): WorldSaveSummary {
    const handle = this.#requireHandle('saveCurrentWorld');
    return callNative(() => this.#addon.saveCurrentWorld(handle) as WorldSaveSummary);
  }

  getCompositionStatus(): CompositionStatus {
    const handle = this.#requireHandle('getCompositionStatus');
    return callNative(() => this.#addon.getCompositionStatus(handle) as CompositionStatus);
  }

  planVoxelConversion(request: VoxelConversionPlanRequest): VoxelConversionPlan {
    const handle = this.#requireHandle('planVoxelConversion');
    const payload = callNative(() => this.#addon.planVoxelConversion(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelConversionPlan>(payload, 'voxel conversion plan');
  }

  registerVoxelConversionSource(
    request: VoxelConversionSourceRegistrationRequest,
  ): VoxelConversionSourceRegistration {
    const handle = this.#requireHandle('registerVoxelConversionSource');
    const payload = callNative(() => this.#addon.registerVoxelConversionSource(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelConversionSourceRegistration>(payload, 'voxel conversion source registration');
  }

  previewVoxelConversion(request: VoxelConversionPreviewRequest): VoxelConversionPreview {
    const handle = this.#requireHandle('previewVoxelConversion');
    const payload = callNative(() => this.#addon.previewVoxelConversion(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelConversionPreview>(payload, 'voxel conversion preview');
  }

  applyVoxelConversion(request: VoxelConversionApplyRequest): VoxelConversionReceipt {
    const handle = this.#requireHandle('applyVoxelConversion');
    const payload = callNative(() => this.#addon.applyVoxelConversion(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelConversionReceipt>(payload, 'voxel conversion receipt');
  }

  exportVoxelConversionEvidence(
    evidence: readonly VoxelConversionEvidenceRef[],
  ): readonly VoxelConversionEvidenceRef[] {
    const handle = this.#requireHandle('exportVoxelConversionEvidence');
    const payload = callNative(() =>
      this.#addon.exportVoxelConversionEvidence(handle, JSON.stringify(evidence)),
    );
    return parseNativeJson<readonly VoxelConversionEvidenceRef[]>(payload, 'voxel conversion evidence');
  }

  readVoxelModelInfo(request: VoxelModelInfoRequest): VoxelModelInfoReadout {
    const handle = this.#requireHandle('readVoxelModelInfo');
    const payload = callNative(() => this.#addon.readVoxelModelInfo(handle, JSON.stringify(request)));
    return parseNativeJson<VoxelModelInfoReadout>(payload, 'voxel model info');
  }

  // ── Unwired operations: fail-closed, never mock-backed ─────────────────────
  // Replace each body with its real native call (and add the manifest name to
  // NATIVE_WIRED_OPERATIONS) when the codegen emitter wires the `#[napi]` export.
  pickVoxel(): PickResult {
    throw nativeUnimplemented('pick_voxel');
  }

  applyCollisionConstrainedCameraInput(): CameraCollisionSnapshot {
    throw nativeUnimplemented('apply_collision_constrained_camera_input');
  }

  selectVoxel(): VoxelSelectionSnapshot {
    throw nativeUnimplemented('select_voxel');
  }

  readVoxelMeshEvidence(): VoxelMeshEvidenceSnapshot {
    throw nativeUnimplemented('read_voxel_mesh_evidence');
  }

  createCamera(): CameraSnapshot {
    throw nativeUnimplemented('create_camera');
  }

  applyFirstPersonCameraInput(): CameraSnapshot {
    throw nativeUnimplemented('apply_first_person_camera_input');
  }

  readCameraProjection(): CameraProjectionSnapshot {
    throw nativeUnimplemented('read_camera_projection');
  }

  getBuffer(): RuntimeBufferView {
    throw nativeUnimplemented('get_buffer');
  }

  releaseBuffer(): void {
    throw nativeUnimplemented('release_buffer');
  }

  unloadWorld(): void {
    throw nativeUnimplemented('unload_world');
  }

  loadReplayFixture(): ReplaySessionHandle {
    throw nativeUnimplemented('load_replay_fixture');
  }

  runReplayStep(): ReplayStepReport {
    throw nativeUnimplemented('run_replay_step');
  }
}

/**
 * Construct the native (napi-rs) bridge. Throws a classified
 * {@link RuntimeBridgeError} of kind `native_unavailable` if the addon is not built
 * — callers can fall back to the mock for tests/dev.
 */
export function createNativeRuntimeBridge(modulePath?: string): RuntimeBridge {
  try {
    const addon = modulePath ? loadNativeAddon(modulePath) : loadNativeAddon();
    return new NativeRuntimeBridge(addon);
  } catch (cause) {
    if (cause instanceof NativeAddonUnavailable) {
      throw new RuntimeBridgeError('native_unavailable', cause.message);
    }
    throw cause;
  }
}

/** Operation count for quick sanity in consumers/tests. */
export const STABLE_OPERATION_COUNT = MANIFEST_OPERATIONS.filter(
  (o) => o.surface === 'stable',
).length;
