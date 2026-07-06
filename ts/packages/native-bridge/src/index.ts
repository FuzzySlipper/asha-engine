// @asha/native-bridge — thin, typed loader for the napi-rs runtime addon.
//
// Scope (ADR 0006): this package wraps the compiled `native-bridge.<platform>.node`
// addon (built from engine-rs/crates/bridge/native-bridge) and exposes its exports
// with explicit TypeScript signatures. It contains NO semantic logic and NO schema
// definitions — it is transport glue. It is imported ONLY by `@asha/runtime-bridge`
// (enforced by governance/ownership.toml); app/UI/renderer never import it.

import { createRequire } from 'node:module';
import type { CommandResult, RenderFrameDiff } from '@asha/contracts';

interface NativeVec3 {
  readonly x: number;
  readonly y: number;
  readonly z: number;
}

interface NativeEnemyDirectNavMovementResult {
  readonly entity: number;
  readonly authoritySource: string;
  readonly from: NativeVec3;
  readonly target: NativeVec3;
  readonly nextWaypoint: NativeVec3;
  readonly distanceUnits: number;
  readonly reached: boolean;
  readonly pathHash: string;
  readonly transformHash: string;
  readonly projectionChanged: boolean;
}

interface NativeFpsTransformCapability {
  readonly translation: NativeVec3;
  readonly rotation: readonly [number, number, number, number];
  readonly scale: NativeVec3;
}

interface NativeFpsBoundsCapability {
  readonly min: NativeVec3;
  readonly max: NativeVec3;
}

interface NativeFpsHealth {
  readonly current: number;
  readonly max: number;
}

interface NativeFpsWeaponMount {
  readonly weaponId: string;
  readonly damage: number;
  readonly rangeUnits: number;
  readonly ammo: number;
  readonly cooldownTicksAfterFire: number;
}

interface NativeFpsPolicyBinding {
  readonly bindingId: string;
  readonly policyId: string;
  readonly viewKind: string;
  readonly viewVersion: string;
  readonly allowedIntents: readonly string[];
  readonly runtimeMoment: string;
}

interface NativeFpsStoredEntityDefinition {
  readonly entity: number;
  readonly stableId: string;
  readonly displayName: string;
  readonly sourcePath: string;
  readonly tags: readonly string[];
  readonly role: string;
  readonly transform: NativeFpsTransformCapability | null;
  readonly bounds: NativeFpsBoundsCapability | null;
  readonly renderVisible: boolean | null;
  readonly staticCollider: boolean | null;
  readonly health: NativeFpsHealth | null;
  readonly weapon: NativeFpsWeaponMount | null;
  readonly policyBinding: NativeFpsPolicyBinding | null;
}

interface NativeFpsRuntimeSessionSnapshot {
  readonly backend: string;
  readonly authoritySurface: string;
  readonly projectBundle: string;
  readonly sessionEpoch: number;
  readonly lifecycleStatus: { readonly state: string; readonly entity?: number; readonly tick?: number };
  readonly playerEntity: number;
  readonly enemyEntity: number;
  readonly health: readonly { readonly entity: number; readonly current: number; readonly max: number }[];
  readonly policyBindings: readonly (NativeFpsPolicyBinding & { readonly entity: number })[];
  readonly replayRecords: readonly {
    readonly replayUnit: string;
    readonly entityHash: string;
    readonly healthHash: string;
    readonly recordHash: string;
  }[];
  readonly readSets: readonly { readonly viewKind: string; readonly owner: string; readonly readSet: readonly string[] }[];
  readonly entityHash: string;
  readonly healthHash: string;
  readonly replayHash: string;
}

interface NativeFpsPrimaryFireResult {
  readonly backend: string;
  readonly authoritySurface: string;
  readonly mutationOwner: string;
  readonly workspaceTrace: readonly string[];
  readonly shooter: number;
  readonly target: number | null;
  readonly targetHealthBefore: NativeFpsHealth | null;
  readonly targetHealthAfter: NativeFpsHealth | null;
  readonly lifecycleStatus: { readonly state: string; readonly entity?: number; readonly tick?: number };
  readonly targetRenderVisible: boolean | null;
  readonly entityHash: string;
  readonly healthHash: string;
  readonly replayHash: string;
}

/**
 * The typed surface the compiled addon exports. Mirrors the `#[napi]` functions in
 * `native-bridge/src/lib.rs`. Kept in lockstep with the bridge manifest's stable
 * operations; the generated `#[napi]` wrappers (one-in/one-out) replace the
 * hand-written stubs once the codegen emitter lands.
 */
export interface NativeAddon {
  initializeEngine(seed: number): number;
  loadWorldBundle(
    handle: number,
    bundleSchemaVersion: number,
    protocolVersion: number,
    sceneId: number,
  ): {
    loadedWorld: number | null;
    fatalCount: number;
    totalCount: number;
    blocksLoad: boolean;
  };
  submitCommands(handle: number, commandsJson: string): CommandResult;
  stepSimulation(handle: number, tick: number): number;
  applyEnemyDirectNavMovement(
    handle: number,
    entity: number,
    seedPosition: NativeVec3,
    target: NativeVec3,
    maxStepUnits: number,
  ): NativeEnemyDirectNavMovementResult;
  loadFpsRuntimeSession(
    handle: number,
    projectBundle: string,
    definitions: readonly NativeFpsStoredEntityDefinition[],
  ): NativeFpsRuntimeSessionSnapshot;
  readFpsRuntimeSession(handle: number): NativeFpsRuntimeSessionSnapshot;
  applyFpsPrimaryFire(
    handle: number,
    tick: number,
    origin: NativeVec3,
    direction: NativeVec3,
  ): NativeFpsPrimaryFireResult;
  restartFpsRuntimeSession(handle: number, expectedEpoch: number): NativeFpsRuntimeSessionSnapshot;
  readRenderDiffs(handle: number, cursor: number): RenderFrameDiff;
  saveCurrentWorld(handle: number): {
    artifactsWritten: number;
    compactedEdits: number;
    retainedEdits: number;
  };
  getCompositionStatus(handle: number): {
    loadedWorld: number | null;
    fatalCount: number;
    totalCount: number;
    blocksLoad: boolean;
  };
}

/** Raised when the native addon cannot be loaded (missing build / ABI mismatch). */
export class NativeAddonUnavailable extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'NativeAddonUnavailable';
  }
}

const REQUIRED_EXPORTS = [
  'initializeEngine',
  'loadWorldBundle',
  'submitCommands',
  'stepSimulation',
  'applyEnemyDirectNavMovement',
  'loadFpsRuntimeSession',
  'readFpsRuntimeSession',
  'applyFpsPrimaryFire',
  'restartFpsRuntimeSession',
  'readRenderDiffs',
  'saveCurrentWorld',
  'getCompositionStatus',
] as const;

/**
 * Attempt to load the compiled addon. Returns a typed handle or throws a
 * classified {@link NativeAddonUnavailable} — never a raw module-resolution error,
 * so `@asha/runtime-bridge` can re-map it to a `native_unavailable` bridge error.
 *
 * Build the addon with `napi build --platform --release` in the native-bridge crate.
 */
export function loadNativeAddon(modulePath = './native-bridge.node'): NativeAddon {
  const require = createRequire(import.meta.url);
  try {
    const mod = require(modulePath) as Partial<Record<(typeof REQUIRED_EXPORTS)[number], unknown>>;
    const missing = REQUIRED_EXPORTS.filter((name) => typeof mod[name] !== 'function');
    if (missing.length > 0) {
      throw new NativeAddonUnavailable(
        `addon at ${modulePath} is missing expected exports (${missing.join(', ')})`,
      );
    }
    return mod as NativeAddon;
  } catch (cause) {
    if (cause instanceof NativeAddonUnavailable) throw cause;
    const reason = cause instanceof Error ? cause.message : String(cause);
    throw new NativeAddonUnavailable(`failed to load native addon at ${modulePath}: ${reason}`);
  }
}
