// @asha/devtools — world-bundle save/load and diagnostics panel read models
// (#2379).
//
// Observational read models for the world-bundle manifest, the ordered authority
// load plan, the save/compaction plan, generator-mismatch + round-trip diagnostics,
// and a navigable diagnostics panel. The load/save *actions* submit typed requests
// through the runtime-bridge facade only — this module never touches the filesystem
// and never mutates authority. Fail-closed outcomes are surfaced, never papered over.

import {
  RuntimeBridgeError,
  type CompositionStatus,
  type RuntimeBridge,
  type WorldLoadRequest,
  type WorldSaveSummary,
} from '@asha/runtime-bridge';
import type {
  ArtifactClass,
  ArtifactEntry,
  CompactionSummary,
  DiagnosticReport,
  DiagnosticReportSet,
  DiagnosticSeverity,
  DiagnosticSourceRef,
  GeneratorMismatch,
  LoadPlan,
  LoadStep,
  RegenConflictReport,
  SaveSummary,
  WorldBundleManifest,
} from '@asha/contracts';

// ── Manifest read model ──────────────────────────────────────────────────────────

/** One artifact row with its persistence class and whether a durable hash is present. */
export interface ArtifactView {
  readonly path: string;
  readonly class: ArtifactClass;
  readonly role: string;
  readonly contentHash: string | null;
  /** A durable artifact must carry a content hash; flagged here when it does not. */
  readonly durableMissingHash: boolean;
}

export interface ManifestModel {
  readonly bundleSchemaVersion: number;
  readonly protocolVersion: number;
  readonly worldId: number;
  readonly sceneId: number;
  readonly assetCount: number;
  readonly artifacts: readonly ArtifactView[];
  /** Artifact counts grouped by persistence class. */
  readonly classCounts: Readonly<Record<ArtifactClass, number>>;
}

/** Build the manifest inspector model from a generated world-bundle manifest. */
export function buildManifestModel(manifest: WorldBundleManifest): ManifestModel {
  const classCounts: Record<ArtifactClass, number> = { durable: 0, generated: 0, cache: 0 };
  const artifacts: ArtifactView[] = manifest.artifacts.map((artifact: ArtifactEntry) => {
    classCounts[artifact.class] += 1;
    return {
      path: artifact.path,
      class: artifact.class,
      role: artifact.role,
      contentHash: artifact.contentHash,
      durableMissingHash: artifact.class === 'durable' && artifact.contentHash === null,
    };
  });
  return {
    bundleSchemaVersion: manifest.bundleSchemaVersion,
    protocolVersion: manifest.protocolVersion,
    worldId: manifest.world.id as number,
    sceneId: manifest.scene.id as number,
    assetCount: manifest.assetLock.assetCount,
    artifacts,
    classCounts,
  };
}

// ── Load plan read model ─────────────────────────────────────────────────────────

export interface LoadStepView {
  readonly index: number;
  readonly step: LoadStep['step'];
  readonly summary: string;
}

function describeLoadStep(step: LoadStep): string {
  switch (step.step) {
    case 'validateVersions':
      return `validate versions (bundle ${step.bundleSchemaVersion}, protocol ${step.protocolVersion})`;
    case 'loadAssetLock':
      return `load asset lock ${step.artifact} (${step.assetCount} assets)`;
    case 'loadSceneDocument':
      return `load scene document ${step.artifact} (scene ${step.scene as number})`;
    case 'generateTerrain':
      return `generate terrain (seed ${step.seed}, generator v${step.version})`;
    case 'applyVoxelEdits':
      return `apply voxel edits (${step.editLogs.length} logs, ${step.snapshots.length} snapshots)`;
    case 'bootstrapScene':
      return `bootstrap scene ${step.scene as number} → world ${step.world as number}`;
    case 'validateFinalState':
      return `validate final state`;
  }
}

export interface LoadPlanView {
  readonly steps: readonly LoadStepView[];
}

/** Build the ordered load-plan read model from a generated load plan. */
export function buildLoadPlanModel(plan: LoadPlan): LoadPlanView {
  return {
    steps: plan.steps.map((step, index) => ({ index, step: step.step, summary: describeLoadStep(step) })),
  };
}

// ── Save / compaction read model ─────────────────────────────────────────────────

export interface SavePlanView {
  readonly writes: readonly ArtifactView[];
  readonly compactedEdits: number;
  readonly retainedEdits: number;
  readonly snapshotChunks: number;
}

/** Build the save/compaction read model from a generated save summary. */
export function buildSavePlanModel(summary: SaveSummary): SavePlanView {
  const writes: ArtifactView[] = summary.writes.map((artifact) => ({
    path: artifact.path,
    class: artifact.class,
    role: artifact.role,
    contentHash: artifact.contentHash,
    durableMissingHash: artifact.class === 'durable' && artifact.contentHash === null,
  }));
  const compaction: CompactionSummary = summary.compaction;
  return {
    writes,
    compactedEdits: compaction.compactedEdits,
    retainedEdits: compaction.retainedEdits,
    snapshotChunks: compaction.snapshotChunks.length,
  };
}

// ── Voxel save/reload/replay durability read model (task #2440) ──────────────────
//
// A projected mirror of the Rust `rule-world-bundle` durability evidence (the
// post-load / post-edit / post-reload world fingerprints for the canonical fixture).
// Observational only: devtools never computes the checkpoints — authority owns the
// fingerprints; this formats them so a panel/agent can read whether the edited world
// survives a save→reload→replay cycle.

/** Projected durability checkpoints for a fixture (mirrors `DurabilityEvidence`). */
export interface VoxelDurabilityEvidence {
  readonly fixture: string;
  /** World fingerprint after the base fixture loads (generation only). */
  readonly postLoad: string;
  /** World fingerprint after the canonical edit sequence. */
  readonly postEdit: string;
  /** World fingerprint after compaction + reload. */
  readonly postReload: string;
  readonly compactedEdits: number;
  readonly retainedEdits: number;
}

/** The summarized durability status: durable iff the reload reproduces the edit. */
export interface VoxelDurabilityView {
  readonly fixture: string;
  readonly postLoad: string;
  readonly postEdit: string;
  readonly postReload: string;
  /** A no-op edit (load == edit) is suspicious — the sequence changed nothing. */
  readonly editedWorld: boolean;
  /** Durability holds iff post-edit and post-reload fingerprints agree. */
  readonly durable: boolean;
  readonly compactedEdits: number;
  readonly retainedEdits: number;
}

/** Build the durability read model from projected evidence (pure, no authority read). */
export function buildVoxelDurabilityModel(evidence: VoxelDurabilityEvidence): VoxelDurabilityView {
  return {
    fixture: evidence.fixture,
    postLoad: evidence.postLoad,
    postEdit: evidence.postEdit,
    postReload: evidence.postReload,
    editedWorld: evidence.postLoad !== evidence.postEdit,
    durable: evidence.postEdit === evidence.postReload,
    compactedEdits: evidence.compactedEdits,
    retainedEdits: evidence.retainedEdits,
  };
}

/** Deterministic display lines summarizing save/reload/replay durability. */
export function summarizeVoxelDurability(view: VoxelDurabilityView): string[] {
  return [
    `fixture ${view.fixture}: durable=${view.durable} edited=${view.editedWorld}`,
    `postLoad=${view.postLoad} postEdit=${view.postEdit} postReload=${view.postReload}`,
    `compaction folded=${view.compactedEdits} retained=${view.retainedEdits}`,
  ];
}

// ── Generator mismatch + round-trip equivalence read model ───────────────────────

export interface GeneratorMismatchView {
  readonly savedVersion: number;
  readonly currentVersion: number;
  readonly detail: string;
}

/** Describe a fail-closed generator version mismatch (never rewrites a save). */
export function describeGeneratorMismatch(mismatch: GeneratorMismatch): GeneratorMismatchView {
  return {
    savedVersion: mismatch.savedVersion,
    currentVersion: mismatch.currentVersion,
    detail: `save used generator v${mismatch.savedVersion}; current build is v${mismatch.currentVersion} — regenerate-and-replay to inspect conflicts`,
  };
}

export interface RegenConflictView {
  readonly savedVersion: number;
  readonly newVersion: number;
  readonly replayedEdits: number;
  readonly conflictCount: number;
  readonly stagingWorldHash: number;
  /** True when every replayed edit landed without a generated-context conflict. */
  readonly equivalent: boolean;
}

/** Build the round-trip / regenerate-and-replay read model (a diagnostic, never a rewrite). */
export function buildRegenReport(report: RegenConflictReport): RegenConflictView {
  return {
    savedVersion: report.savedVersion,
    newVersion: report.newVersion,
    replayedEdits: report.replayedEdits,
    conflictCount: report.conflicts.length,
    stagingWorldHash: report.stagingWorldHash,
    equivalent: report.conflicts.length === 0,
  };
}

// ── Diagnostics panel with source navigation ─────────────────────────────────────

/** The most specific authority locus a diagnostic points at, for navigation. */
export type DiagnosticTarget =
  | { readonly kind: 'renderHandle'; readonly handle: number }
  | { readonly kind: 'sceneNode'; readonly sceneNodeId: number }
  | { readonly kind: 'entity'; readonly entityId: number }
  | { readonly kind: 'asset'; readonly assetId: string }
  | { readonly kind: 'chunk'; readonly coord: readonly [number, number, number] }
  | { readonly kind: 'bundlePath'; readonly path: string }
  | { readonly kind: 'none' };

/**
 * Resolve a diagnostic's source ref to the most specific available target, so a
 * panel can navigate to the failing render handle / scene node / entity / asset /
 * chunk / artifact path. Returns `none` when no locus is present (never silent).
 */
export function navigateSource(source: DiagnosticSourceRef): DiagnosticTarget {
  if (source.renderHandle !== null) {
    return { kind: 'renderHandle', handle: source.renderHandle };
  }
  if (source.sceneNodeId !== null) {
    return { kind: 'sceneNode', sceneNodeId: source.sceneNodeId };
  }
  if (source.runtimeEntityId !== null) {
    return { kind: 'entity', entityId: source.runtimeEntityId };
  }
  if (source.assetId !== null) {
    return { kind: 'asset', assetId: source.assetId };
  }
  if (source.chunkCoord !== null) {
    return { kind: 'chunk', coord: source.chunkCoord };
  }
  if (source.bundlePath !== null) {
    return { kind: 'bundlePath', path: source.bundlePath };
  }
  return { kind: 'none' };
}

export interface DiagnosticView {
  readonly scope: DiagnosticReport['scope'];
  readonly severity: DiagnosticSeverity;
  readonly code: DiagnosticReport['code'];
  readonly message: string;
  /** Advisory remedy, when the diagnostic carries one. */
  readonly remedy: { readonly action: string; readonly detail: string } | null;
  readonly target: DiagnosticTarget;
}

export interface DiagnosticsPanelModel {
  readonly diagnostics: readonly DiagnosticView[];
  readonly fatalCount: number;
  /** Only a fatal diagnostic blocks a load. */
  readonly blocksLoad: boolean;
}

/** Build the diagnostics panel model: severity, remedy, and navigable source per report. */
export function buildDiagnosticsPanel(set: DiagnosticReportSet): DiagnosticsPanelModel {
  const diagnostics: DiagnosticView[] = set.reports.map((report: DiagnosticReport) => ({
    scope: report.scope,
    severity: report.severity,
    code: report.code,
    message: report.message,
    remedy: report.remedy === null ? null : { action: report.remedy.action, detail: report.remedy.detail },
    target: navigateSource(report.source),
  }));
  const fatalCount = diagnostics.filter((d) => d.severity === 'fatal').length;
  return { diagnostics, fatalCount, blocksLoad: fatalCount > 0 };
}

// ── Load / save action requests (through the facade only) ─────────────────────────

/** Derive the typed facade load request from a manifest (no local mutation). */
export function buildLoadRequest(manifest: WorldBundleManifest): WorldLoadRequest {
  return {
    bundleSchemaVersion: manifest.bundleSchemaVersion,
    protocolVersion: manifest.protocolVersion,
    sceneId: manifest.scene.id as number,
  };
}

/** A classified outcome of a load/save action — fail-closed errors are surfaced. */
export type BundleActionResult<T> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly kind: string; readonly message: string; readonly recovery: string };

function recoveryHint(error: RuntimeBridgeError): string {
  switch (error.kind) {
    case 'invalid_input':
      return 'bundle is incompatible with this build — inspect the manifest version/protocol diagnostics';
    case 'not_initialized':
      return 'load a world before saving';
    case 'native_unavailable':
      return 'the native runtime is unavailable — retry on the mock facade or rebuild the addon';
    default:
      return 'inspect composition diagnostics for the failing artifact';
  }
}

/**
 * Submit a world-bundle load through the facade. The prior world is left untouched
 * on failure (the facade stages the swap); this returns a classified result rather
 * than throwing, so a panel can render the fail-closed outcome.
 */
export function submitLoad(bridge: RuntimeBridge, request: WorldLoadRequest): BundleActionResult<CompositionStatus> {
  try {
    return { ok: true, value: bridge.loadWorldBundle(request) };
  } catch (error) {
    if (error instanceof RuntimeBridgeError) {
      return { ok: false, kind: error.kind, message: error.message, recovery: recoveryHint(error) };
    }
    throw error;
  }
}

/** Submit a save through the facade, returning a classified result. */
export function submitSave(bridge: RuntimeBridge): BundleActionResult<WorldSaveSummary> {
  try {
    return { ok: true, value: bridge.saveCurrentWorld() };
  } catch (error) {
    if (error instanceof RuntimeBridgeError) {
      return { ok: false, kind: error.kind, message: error.message, recovery: recoveryHint(error) };
    }
    throw error;
  }
}
