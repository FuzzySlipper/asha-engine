import type { DiagnosticReport, DiagnosticReportSet, DiagnosticSeverity, RendererResourceReport, SourceTrace } from '@asha/contracts';
import type { PolicyRunSummary } from './policy-panel.js';
/** The owning lane a failure routes to — the operator console's routing taxonomy. */
export type OperatorLane = 'bridge' | 'protocolContracts' | 'stateRules' | 'renderProjection' | 'rendererResources' | 'policySandbox' | 'assetCatalog' | 'persistenceReplay';
/** Classify a diagnostic into its owning lane (scope first, with code overrides). */
export declare function classifyLane(report: DiagnosticReport): OperatorLane;
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
    readonly loadedProjectBundleId: number | null;
    readonly worldHash: string | null;
    readonly protocolVersion: number | null;
    readonly schemaVersion: number | null;
    readonly capabilities: readonly CapabilityStatus[];
}
/** Last persistence operation readout (from the bundle/world-state panels). */
export interface PersistenceReadout {
    readonly operation: 'save' | 'load' | 'replay';
    readonly status: 'ok' | 'failed';
    readonly worldHash: string | null;
    /** Artifact roles touched (e.g. sceneDocument, sessionStateSnapshot, voxelEditLog). */
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
    readonly resources: (RendererResourceReport & {
        readonly suspectedLeak: boolean;
    }) | null;
    readonly persistence: PersistenceReadout | null;
    readonly policy: PolicyRunSummary | null;
    readonly commands: readonly CommandHistoryEntry[];
    readonly limitations: readonly KnownLimitation[];
    /** True when no diagnostic is `error`/`fatal` and the mode is not unavailable. */
    readonly ready: boolean;
}
/** Build the unified operator console model. Pure and deterministic. */
export declare function buildOperatorConsole(input: OperatorConsoleInput): OperatorConsoleModel;
/**
 * Stable JSON export of the operator console for smoke/agents/CI. The model is
 * built from plain objects in fixed field order, so `JSON.stringify` output is
 * deterministic for a given input.
 */
export declare function toOperatorJson(model: OperatorConsoleModel): string;
/** Deterministic, greppable text projection of the operator console. */
export declare function formatOperatorConsole(model: OperatorConsoleModel): string[];
//# sourceMappingURL=operator-console.d.ts.map