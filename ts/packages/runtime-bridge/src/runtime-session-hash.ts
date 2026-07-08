import type { RenderFrameDiff } from '@asha/contracts';
import type { CompositionStatus, ProjectBundleLoadRequest } from './bridge.js';
import type { EncounterDirectorState } from './encounter-director.js';
import type {
  RuntimeSessionHashRecord,
  RuntimeSessionHashValue,
  RuntimeSessionIdentity,
  RuntimeSessionLifecycleHealthReadout,
  RuntimeSessionLifecycleState,
  RuntimeSessionNonClaim,
} from './runtime-session.js';

// These hashes are deterministic TypeScript readout/projection fingerprints.
// Live Rust-backed authority hashes must come from bridge snapshots/results.

export function referenceRuntimeSessionNonClaims(): readonly RuntimeSessionNonClaim[] {
  return [
    'not_native_runtime',
    'not_raw_state_store',
    'not_arbitrary_json_bridge',
    'not_product_authority',
    'not_gameplay_loop',
    'not_renderer',
  ];
}

export function identityHashRecord(identity: RuntimeSessionIdentity): RuntimeSessionHashRecord {
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

export function encounterStateHashRecord(state: EncounterDirectorState): RuntimeSessionHashRecord {
  return {
    presetId: state.presetId,
    status: state.status,
    spawnedEnemyIds: state.spawnedEnemyIds,
    defeatedEnemyIds: state.defeatedEnemyIds,
    revision: state.revision,
    lastTransition: state.lastTransition,
  };
}

export function lifecycleStateHashRecord(state: RuntimeSessionLifecycleState): RuntimeSessionHashRecord {
  return {
    player: lifecycleHealthHashRecord(state.player),
    enemy: lifecycleHealthHashRecord(state.enemy),
    terminalEventHash: state.terminalEvent?.eventHash ?? null,
    revision: state.revision,
  };
}

function lifecycleHealthHashRecord(health: RuntimeSessionLifecycleHealthReadout): RuntimeSessionHashRecord {
  return {
    entity: health.entity,
    current: health.current,
    max: health.max,
    dead: health.dead,
  };
}

export function projectBundleHashRecord(projectBundle: ProjectBundleLoadRequest): RuntimeSessionHashRecord {
  return {
    bundleSchemaVersion: projectBundle.bundleSchemaVersion,
    protocolVersion: projectBundle.protocolVersion,
    sceneId: projectBundle.sceneId,
  };
}

export function compositionHashRecord(composition: CompositionStatus): RuntimeSessionHashRecord {
  return {
    loadedProjectBundle: composition.loadedProjectBundle,
    fatalCount: composition.fatalCount,
    totalCount: composition.totalCount,
    blocksLoad: composition.blocksLoad,
  };
}

export function renderFrameHashRecord(frame: RenderFrameDiff): RuntimeSessionHashRecord {
  return {
    opCount: frame.ops.length,
    opKinds: frame.ops.map((op) => op.op),
  };
}

export function stableHash(value: RuntimeSessionHashValue | undefined): string {
  return `fnv1a64:${fnv1a64(stableStringify(value))}`;
}

function stableStringify(value: RuntimeSessionHashValue | undefined): string {
  if (value === undefined) {
    return 'undefined';
  }
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    const entries = value as readonly RuntimeSessionHashValue[];
    return `[${entries.map((entry) => stableStringify(entry)).join(',')}]`;
  }
  const record = value as RuntimeSessionHashRecord;
  return `{${Object.keys(record)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
    .join(',')}}`;
}

function fnv1a64(text: string): string {
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  const mask = 0xffffffffffffffffn;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= BigInt(text.charCodeAt(index));
    hash = (hash * prime) & mask;
  }
  return hash.toString(16).padStart(16, '0');
}
