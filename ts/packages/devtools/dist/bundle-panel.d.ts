import { type CompositionStatus, type RuntimeBridge, type WorldLoadRequest, type WorldSaveSummary } from '@asha/runtime-bridge';
import type { ArtifactClass, DiagnosticReport, DiagnosticReportSet, DiagnosticSeverity, DiagnosticSourceRef, GeneratorMismatch, LoadPlan, LoadStep, RegenConflictReport, SaveSummary, WorldBundleManifest } from '@asha/contracts';
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
export declare function buildManifestModel(manifest: WorldBundleManifest): ManifestModel;
export interface LoadStepView {
    readonly index: number;
    readonly step: LoadStep['step'];
    readonly summary: string;
}
export interface LoadPlanView {
    readonly steps: readonly LoadStepView[];
}
/** Build the ordered load-plan read model from a generated load plan. */
export declare function buildLoadPlanModel(plan: LoadPlan): LoadPlanView;
export interface SavePlanView {
    readonly writes: readonly ArtifactView[];
    readonly compactedEdits: number;
    readonly retainedEdits: number;
    readonly snapshotChunks: number;
}
/** Build the save/compaction read model from a generated save summary. */
export declare function buildSavePlanModel(summary: SaveSummary): SavePlanView;
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
export declare function buildVoxelDurabilityModel(evidence: VoxelDurabilityEvidence): VoxelDurabilityView;
/** Deterministic display lines summarizing save/reload/replay durability. */
export declare function summarizeVoxelDurability(view: VoxelDurabilityView): string[];
export interface GeneratorMismatchView {
    readonly savedVersion: number;
    readonly currentVersion: number;
    readonly detail: string;
}
/** Describe a fail-closed generator version mismatch (never rewrites a save). */
export declare function describeGeneratorMismatch(mismatch: GeneratorMismatch): GeneratorMismatchView;
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
export declare function buildRegenReport(report: RegenConflictReport): RegenConflictView;
/** The most specific authority locus a diagnostic points at, for navigation. */
export type DiagnosticTarget = {
    readonly kind: 'renderHandle';
    readonly handle: number;
} | {
    readonly kind: 'sceneNode';
    readonly sceneNodeId: number;
} | {
    readonly kind: 'entity';
    readonly entityId: number;
} | {
    readonly kind: 'asset';
    readonly assetId: string;
} | {
    readonly kind: 'chunk';
    readonly coord: readonly [number, number, number];
} | {
    readonly kind: 'bundlePath';
    readonly path: string;
} | {
    readonly kind: 'none';
};
/**
 * Resolve a diagnostic's source ref to the most specific available target, so a
 * panel can navigate to the failing render handle / scene node / entity / asset /
 * chunk / artifact path. Returns `none` when no locus is present (never silent).
 */
export declare function navigateSource(source: DiagnosticSourceRef): DiagnosticTarget;
export interface DiagnosticView {
    readonly scope: DiagnosticReport['scope'];
    readonly severity: DiagnosticSeverity;
    readonly code: DiagnosticReport['code'];
    readonly message: string;
    /** Advisory remedy, when the diagnostic carries one. */
    readonly remedy: {
        readonly action: string;
        readonly detail: string;
    } | null;
    readonly target: DiagnosticTarget;
}
export interface DiagnosticsPanelModel {
    readonly diagnostics: readonly DiagnosticView[];
    readonly fatalCount: number;
    /** Only a fatal diagnostic blocks a load. */
    readonly blocksLoad: boolean;
}
/** Build the diagnostics panel model: severity, remedy, and navigable source per report. */
export declare function buildDiagnosticsPanel(set: DiagnosticReportSet): DiagnosticsPanelModel;
/** Derive the typed facade load request from a manifest (no local mutation). */
export declare function buildLoadRequest(manifest: WorldBundleManifest): WorldLoadRequest;
/** A classified outcome of a load/save action — fail-closed errors are surfaced. */
export type BundleActionResult<T> = {
    readonly ok: true;
    readonly value: T;
} | {
    readonly ok: false;
    readonly kind: string;
    readonly message: string;
    readonly recovery: string;
};
/**
 * Submit a world-bundle load through the facade. The prior world is left untouched
 * on failure (the facade stages the swap); this returns a classified result rather
 * than throwing, so a panel can render the fail-closed outcome.
 */
export declare function submitLoad(bridge: RuntimeBridge, request: WorldLoadRequest): BundleActionResult<CompositionStatus>;
/** Submit a save through the facade, returning a classified result. */
export declare function submitSave(bridge: RuntimeBridge): BundleActionResult<WorldSaveSummary>;
//# sourceMappingURL=bundle-panel.d.ts.map