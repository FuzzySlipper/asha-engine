// @asha/devtools — unified operator console read model + agent-readable export
// (#2488).
//
// Aggregates the existing devtools read models (runtime status, command/policy
// history, source traces, save/load/replay, renderer resources, known limitations)
// into ONE structured, agent-readable + human-readable model with owning-lane
// failure classification. Observational only: it composes projected data and
// classifies it; it never mutates authority and exposes no command path. The
// DOM/text panels are projections of `buildOperatorConsole`; `toOperatorJson` is
// the stable export smoke/agents/CI parse.

import type {
  DiagnosticReport,
  DiagnosticReportSet,
  DiagnosticSeverity,
  RendererResourceReport,
  SourceTrace,
} from '@asha/contracts';

import type { PolicyRunSummary } from './policy-panel.js';

// ── Owning lanes (failure routing) ──────────────────────────────────────────────

/** The owning lane a failure routes to — the operator console's routing taxonomy. */
export type OperatorLane =
  | 'bridge'
  | 'protocolContracts'
  | 'stateRules'
  | 'renderProjection'
  | 'rendererResources'
  | 'policySandbox'
  | 'assetCatalog'
  | 'persistenceReplay';

/** Classify a diagnostic into its owning lane (scope first, with code overrides). */
export function classifyLane(report: DiagnosticReport): OperatorLane {
  // A protocol/version mismatch is a contract problem regardless of scope.
  if (report.code === 'manifestProtocolMismatch') {
    return 'protocolContracts';
  }
  switch (report.scope) {
    case 'scene':
      return 'stateRules';
    case 'assetCatalog':
      return 'assetCatalog';
    case 'renderProjection':
      return 'renderProjection';
    case 'rendererResources':
      return 'rendererResources';
    case 'worldBundle':
    case 'worldComposition':
      return 'persistenceReplay';
  }
}

// ── Runtime status ──────────────────────────────────────────────────────────────

/** Engine runtime mode (which authority transport backs the session). */
export type RuntimeMode = 'native' | 'reference' | 'mock' | 'degraded' | 'unavailable';

/** One engine capability/operation and whether it is available in this mode. */
export interface CapabilityStatus {
  readonly operation: string;
  readonly available: boolean;
  /** A known native gap or reason, when unavailable. */
  readonly note: string | null;
}

/** The unified runtime status: mode, loaded world identity/versions, capabilities. */
export interface RuntimeStatus {
  readonly mode: RuntimeMode;
  readonly loadedWorldId: number | null;
  readonly worldHash: string | null;
  readonly protocolVersion: number | null;
  readonly schemaVersion: number | null;
  readonly capabilities: readonly CapabilityStatus[];
}

// ── Save / load / replay readout ────────────────────────────────────────────────

/** Last persistence operation readout (from the bundle/world-state panels). */
export interface PersistenceReadout {
  readonly operation: 'save' | 'load' | 'replay';
  readonly status: 'ok' | 'failed';
  readonly worldHash: string | null;
  /** Artifact roles touched (e.g. sceneDocument, worldStateSnapshot, voxelEditLog). */
  readonly artifactRoles: readonly string[];
  /** Classified divergence/compaction summary, when relevant. */
  readonly detail: string | null;
}

/** One recent command batch with provenance, for the command/policy history. */
export interface CommandHistoryEntry {
  readonly source: 'ui' | 'policy' | 'system' | 'replay';
  readonly accepted: number;
  readonly rejected: number;
  /** Affected authority refs (entity/chunk/asset/handle ids), for navigation. */
  readonly affected: readonly string[];
}

/** One known-limitation entry, machine-readable (from a local generated snapshot). */
export interface KnownLimitation {
  readonly id: string;
  readonly lane: OperatorLane;
  readonly summary: string;
  /** Whether it affects the *current* runtime mode. */
  readonly activeInMode: boolean;
}

// ── Aggregated model ────────────────────────────────────────────────────────────

/** Everything the operator console aggregates for one snapshot. */
export interface OperatorConsoleInput {
  readonly runtime: RuntimeStatus;
  readonly diagnostics: DiagnosticReportSet;
  readonly sourceTraces: readonly SourceTrace[];
  readonly resources: RendererResourceReport | null;
  readonly persistence: PersistenceReadout | null;
  readonly policy: PolicyRunSummary | null;
  readonly commands: readonly CommandHistoryEntry[];
  readonly limitations: readonly KnownLimitation[];
}

/** Per-lane failure rollup: count + worst severity, for routing. */
export interface LaneFailure {
  readonly lane: OperatorLane;
  readonly count: number;
  readonly maxSeverity: DiagnosticSeverity;
}

/** One source-trace row classified as resolved or broken (missing/stale hop). */
export interface SourceTraceRow {
  readonly renderHandle: number;
  readonly sceneNodeId: number | null;
  readonly runtimeEntityId: number | null;
  readonly assetId: string | null;
  readonly resolved: boolean;
  readonly broken: boolean;
}

/** The unified operator console model. */
export interface OperatorConsoleModel {
  readonly runtime: RuntimeStatus;
  readonly laneFailures: readonly LaneFailure[];
  readonly sourceTraces: readonly SourceTraceRow[];
  readonly resources: (RendererResourceReport & { readonly suspectedLeak: boolean }) | null;
  readonly persistence: PersistenceReadout | null;
  readonly policy: PolicyRunSummary | null;
  readonly commands: readonly CommandHistoryEntry[];
  readonly limitations: readonly KnownLimitation[];
  /** True when no diagnostic is `error`/`fatal` and the mode is not unavailable. */
  readonly ready: boolean;
}

const SEVERITY_RANK: Record<DiagnosticSeverity, number> = { info: 0, warning: 1, error: 2, fatal: 3 };

function worse(a: DiagnosticSeverity, b: DiagnosticSeverity): DiagnosticSeverity {
  return SEVERITY_RANK[a] >= SEVERITY_RANK[b] ? a : b;
}

function classifySourceTrace(t: SourceTrace): SourceTraceRow {
  // A trace is broken if it claims an asset it could not resolve, or a hop is
  // missing where one is expected (no entity behind a handle).
  const broken = (t.assetId !== null && !t.assetResolved) || t.runtimeEntityId === null;
  return {
    renderHandle: t.renderHandle,
    sceneNodeId: t.sceneNodeId,
    runtimeEntityId: t.runtimeEntityId,
    assetId: t.assetId,
    resolved: t.assetResolved,
    broken,
  };
}

/** Build the unified operator console model. Pure and deterministic. */
export function buildOperatorConsole(input: OperatorConsoleInput): OperatorConsoleModel {
  // Roll diagnostics up by owning lane.
  const byLane = new Map<OperatorLane, { count: number; maxSeverity: DiagnosticSeverity }>();
  let worstOverall: DiagnosticSeverity = 'info';
  for (const report of input.diagnostics.reports) {
    const lane = classifyLane(report);
    const prior = byLane.get(lane);
    byLane.set(lane, {
      count: (prior?.count ?? 0) + 1,
      maxSeverity: prior ? worse(prior.maxSeverity, report.severity) : report.severity,
    });
    worstOverall = worse(worstOverall, report.severity);
  }
  const laneFailures: LaneFailure[] = [...byLane.entries()]
    .map(([lane, v]) => ({ lane, count: v.count, maxSeverity: v.maxSeverity }))
    .sort((a, b) => a.lane.localeCompare(b.lane));

  const resources = input.resources
    ? {
        ...input.resources,
        // A created/disposed imbalance beyond live handles hints at a leak.
        suspectedLeak:
          input.resources.resourcesCreated - input.resources.resourcesDisposed > input.resources.liveHandles,
      }
    : null;

  const ready =
    input.runtime.mode !== 'unavailable' && SEVERITY_RANK[worstOverall] < SEVERITY_RANK.error;

  return {
    runtime: input.runtime,
    laneFailures,
    sourceTraces: input.sourceTraces.map(classifySourceTrace),
    resources,
    persistence: input.persistence,
    policy: input.policy,
    commands: input.commands,
    limitations: input.limitations,
    ready,
  };
}

// ── Agent-readable export ───────────────────────────────────────────────────────

/**
 * Stable JSON export of the operator console for smoke/agents/CI. The model is
 * built from plain objects in fixed field order, so `JSON.stringify` output is
 * deterministic for a given input.
 */
export function toOperatorJson(model: OperatorConsoleModel): string {
  return JSON.stringify(model, null, 2);
}

/** Deterministic, greppable text projection of the operator console. */
export function formatOperatorConsole(model: OperatorConsoleModel): string[] {
  const lines: string[] = [];
  const r = model.runtime;
  lines.push(
    `runtime mode=${r.mode} world=${r.loadedWorldId ?? '-'} hash=${r.worldHash ?? '-'} ` +
      `protocol=${r.protocolVersion ?? '-'} schema=${r.schemaVersion ?? '-'} ready=${model.ready}`,
  );
  for (const cap of r.capabilities) {
    lines.push(`  capability ${cap.operation} available=${cap.available}${cap.note ? ` (${cap.note})` : ''}`);
  }
  for (const lane of model.laneFailures) {
    lines.push(`lane ${lane.lane} failures=${lane.count} maxSeverity=${lane.maxSeverity}`);
  }
  for (const t of model.sourceTraces) {
    lines.push(`trace handle=${t.renderHandle} entity=${t.runtimeEntityId ?? '-'} asset=${t.assetId ?? '-'} broken=${t.broken}`);
  }
  if (model.resources) {
    lines.push(
      `resources handles=${model.resources.liveHandles} geometries=${model.resources.geometries} ` +
        `materials=${model.resources.materials} fallbacks=${model.resources.fallbackMaterials} ` +
        `suspectedLeak=${model.resources.suspectedLeak}`,
    );
  }
  if (model.persistence) {
    const p = model.persistence;
    lines.push(`persistence ${p.operation} ${p.status} hash=${p.worldHash ?? '-'} roles=[${p.artifactRoles.join(',')}]`);
  }
  if (model.policy) {
    lines.push(`policy tick=${model.policy.tick} proposed=${model.policy.totalProposed} accepted=${model.policy.accepted} rejected=${model.policy.rejected}`);
  }
  for (const c of model.commands) {
    lines.push(`command source=${c.source} accepted=${c.accepted} rejected=${c.rejected}`);
  }
  for (const l of model.limitations) {
    lines.push(`limitation ${l.id} lane=${l.lane} activeInMode=${l.activeInMode}`);
  }
  return lines;
}
