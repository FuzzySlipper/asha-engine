// @asha/devtools — asset catalog, lock-drift, and material inspector read models
// (#2378).
//
// Observational, **read-only** projections built from generated catalog/diagnostic
// contracts. TS holds no catalog-validation authority: it never re-locks, relocates,
// or silently fixes assets. Every missing/wrong-kind/stale/cycle/fallback state is
// classified and surfaced, never hidden.
//
// The material projection split is structural: the renderer-facing `render` view
// and the authority-facing `collision` view are exposed as two disjoint read
// objects. There is deliberately no mixed material object a UI could edit as one.

import type {
  AssetKind,
  AssetReference,
  Catalog,
  CatalogEntry,
  CatalogValidationReport,
  CollisionMaterial,
  FallbackDecision,
  LockFinding,
  LockIssueCode,
  LockValidationReport,
  MaterialProjection,
  RenderMaterial,
} from '@asha/contracts';

// ── Catalog + dependency DAG read model ─────────────────────────────────────────

/** One catalog entry as displayed: identity, kind, and resolved dependency ids. */
export interface CatalogEntryView {
  readonly id: string;
  readonly kind: AssetKind;
  readonly version: number;
  readonly label: string | null;
  readonly hasMaterial: boolean;
  /** Dependency asset ids, in declared order. */
  readonly dependencies: readonly string[];
  /** Dependency ids that are not present in the catalog (classified, not dropped). */
  readonly missingDependencies: readonly string[];
}

/** A classified catalog readout derived from a generated validation report. */
export interface CatalogStructuralIssue {
  readonly code: CatalogValidationReport['errors'][number]['code'];
  readonly id: string | null;
  readonly detail: string;
  /** Non-empty only for dependency-cycle. */
  readonly cyclePath: readonly string[];
}

export interface CatalogModel {
  readonly entries: readonly CatalogEntryView[];
  /** Adjacency: asset id → dependency ids present in the catalog. */
  readonly dependencyEdges: ReadonlyMap<string, readonly string[]>;
  /** Cycles detected over present dependencies (each path starts at its lowest id). */
  readonly cycles: readonly (readonly string[])[];
  readonly structuralIssues: readonly CatalogStructuralIssue[];
}

function depIds(deps: readonly AssetReference[]): string[] {
  return deps.map((d) => d.id);
}

function detectDependencyCycles(edges: ReadonlyMap<string, readonly string[]>): string[][] {
  const cycles: string[][] = [];
  const state = new Map<string, 'visiting' | 'done'>();
  const stack: string[] = [];

  const visit = (id: string): void => {
    state.set(id, 'visiting');
    stack.push(id);
    for (const dep of edges.get(id) ?? []) {
      const depState = state.get(dep);
      if (depState === undefined) {
        visit(dep);
      } else if (depState === 'visiting') {
        const from = stack.indexOf(dep);
        if (from >= 0) {
          cycles.push(stack.slice(from));
        }
      }
    }
    stack.pop();
    state.set(id, 'done');
  };

  // Deterministic: start from ids in sorted order.
  for (const id of [...edges.keys()].sort()) {
    if (!state.has(id)) {
      visit(id);
    }
  }
  return cycles;
}

function describeCatalogError(error: CatalogValidationReport['errors'][number]): string {
  switch (error.code) {
    case 'duplicate-asset-id':
      return `asset id ${error.id} is declared more than once`;
    case 'material-payload-missing':
      return `material asset ${error.id} is missing its material payload`;
    case 'material-payload-on-non-material':
      return `non-material asset ${error.id} carries a material payload`;
    case 'wrong-kind-reference':
      return `${error.from} slot ${error.slot} expected ${error.expected}, found ${error.actual}`;
    case 'unknown-dependency':
      return `asset ${error.id} depends on unknown ${error.dependency}`;
    case 'dependency-cycle':
      return `dependency cycle: ${error.cyclePath.join(' → ')}`;
    case 'empty-source-path':
      return `asset ${error.id} has an empty source path`;
  }
}

/**
 * Build the catalog inspector model: per-entry views, the dependency DAG over
 * present assets, detected cycles, and classified structural issues from a
 * generated validation report (when one is supplied).
 */
export function buildCatalogModel(catalog: Catalog, validation?: CatalogValidationReport): CatalogModel {
  const present = new Set<string>(catalog.entries.map((e) => e.id));
  const dependencyEdges = new Map<string, readonly string[]>();
  const entries: CatalogEntryView[] = catalog.entries.map((entry: CatalogEntry) => {
    const deps = depIds(entry.dependencies);
    const presentDeps = deps.filter((d) => present.has(d));
    dependencyEdges.set(entry.id, presentDeps);
    return {
      id: entry.id,
      kind: entry.kind,
      version: entry.version,
      label: entry.label,
      hasMaterial: entry.material !== null,
      dependencies: deps,
      missingDependencies: deps.filter((d) => !present.has(d)),
    };
  });

  const cycles = detectDependencyCycles(dependencyEdges);
  const structuralIssues: CatalogStructuralIssue[] = (validation?.errors ?? []).map((error) => ({
    code: error.code,
    id: error.id,
    detail: describeCatalogError(error),
    cyclePath: error.cyclePath,
  }));

  return { entries, dependencyEdges, cycles, structuralIssues };
}

// ── Asset-lock drift read model ─────────────────────────────────────────────────

/** A lock finding's display severity. `new-in-catalog` is informational. */
export type LockDriftSeverity = 'info' | 'warning' | 'drift';

function lockSeverity(code: LockIssueCode): LockDriftSeverity {
  switch (code) {
    case 'new-in-catalog':
      return 'info';
    case 'stale-version':
    case 'dependency-drift':
      return 'warning';
    case 'missing':
    case 'wrong-kind':
    case 'stale-hash':
      return 'drift';
  }
}

export interface LockFindingView {
  readonly id: string;
  readonly code: LockIssueCode;
  readonly severity: LockDriftSeverity;
  readonly detail: string;
}

export interface LockDriftModel {
  readonly findings: readonly LockFindingView[];
  /** True when any finding is more than informational — a save must not silently relock. */
  readonly hasDrift: boolean;
}

function describeLockFinding(finding: LockFinding): string {
  switch (finding.code) {
    case 'missing':
      return `locked asset ${finding.id} is no longer in the catalog`;
    case 'wrong-kind':
      return `asset ${finding.id} kind changed ${finding.lockedKind} → ${finding.currentKind}`;
    case 'stale-version':
      return `asset ${finding.id} version changed ${finding.lockedVersion} → ${finding.currentVersion}`;
    case 'stale-hash':
      return `asset ${finding.id} content hash changed`;
    case 'dependency-drift':
      return `asset ${finding.id} dependencies changed (+${finding.addedDependencies.length}/-${finding.removedDependencies.length})`;
    case 'new-in-catalog':
      return `asset ${finding.id} is new in the catalog and not yet locked`;
  }
}

/** Build the lock-drift inspector model from a generated lock validation report. */
export function buildLockDriftModel(report: LockValidationReport): LockDriftModel {
  const findings: LockFindingView[] = report.findings.map((finding) => ({
    id: finding.id,
    code: finding.code,
    severity: lockSeverity(finding.code),
    detail: describeLockFinding(finding),
  }));
  return { findings, hasDrift: findings.some((f) => f.severity !== 'info') };
}

// ── Material projection inspector (disjoint render vs collision) ─────────────────

/**
 * The material inspector view. The two projections are exposed as separate read
 * objects so a UI cannot present (or edit) them as one mixed material: the pure
 * render path consumes only `render`, authority consumes only `collision`.
 */
export interface MaterialInspection {
  readonly render: RenderMaterial;
  readonly collision: CollisionMaterial;
}

/** Inspect a catalog entry's material projection, or null for a non-material asset. */
export function inspectMaterial(entry: CatalogEntry): MaterialInspection | null {
  if (entry.material === null) {
    return null;
  }
  const projection: MaterialProjection = entry.material;
  // Returned as two distinct objects — never spread into one mixed record.
  return { render: projection.render, collision: projection.collision };
}

// ── Fallback decision readout ────────────────────────────────────────────────────

export interface FallbackReadout {
  readonly outcome: FallbackDecision['outcome'];
  readonly reason: string;
  /** The concrete placeholder, present only when a fallback is actually used. */
  readonly visual: string | null;
  /** True only for the `useFallback` outcome — a missing asset is being substituted. */
  readonly fallbackUsed: boolean;
}

/** Classify a fallback decision for display (never authorizes a substitution). */
export function classifyFallback(decision: FallbackDecision): FallbackReadout {
  if (decision.outcome === 'useFallback') {
    return { outcome: 'useFallback', reason: decision.reason, visual: decision.visual, fallbackUsed: true };
  }
  return { outcome: decision.outcome, reason: decision.reason, visual: null, fallbackUsed: false };
}

// ── Changed-asset impact report ───────────────────────────────────────────────────

export interface ImpactReport {
  readonly changed: string;
  /** Catalog entries that depend (transitively) on the changed asset. */
  readonly dependents: readonly string[];
  /** True when the changed id is not present in the catalog. */
  readonly unknownAsset: boolean;
}

/**
 * Report which catalog entries are impacted by a change to `changedId` — every
 * asset that transitively depends on it. Pure; reads the catalog's declared
 * dependency edges and never mutates.
 */
export function impactOfChangedAsset(catalog: Catalog, changedId: string): ImpactReport {
  const present = new Set<string>(catalog.entries.map((e) => e.id));
  // Reverse adjacency: dependency id → assets that declare it.
  const dependentsOf = new Map<string, string[]>();
  for (const entry of catalog.entries) {
    for (const dep of entry.dependencies) {
      const bucket = dependentsOf.get(dep.id);
      if (bucket === undefined) {
        dependentsOf.set(dep.id, [entry.id]);
      } else {
        bucket.push(entry.id);
      }
    }
  }

  const impacted = new Set<string>();
  const queue: string[] = [changedId];
  while (queue.length > 0) {
    const current = queue.shift()!;
    for (const dependent of dependentsOf.get(current) ?? []) {
      if (!impacted.has(dependent)) {
        impacted.add(dependent);
        queue.push(dependent);
      }
    }
  }

  return {
    changed: changedId,
    dependents: [...impacted].sort(),
    unknownAsset: !present.has(changedId),
  };
}

// ── Source-asset GUID / sidecar trace read model (#2486) ────────────────────────
//
// Surfaces the file-driven asset metadata (sidecar GUID, source URI, generated
// artifacts, sidecar reconcile status) the importer records, projected from the
// import manifest — so an operator/agent sees source-asset identity and drift
// without reading raw manifest files. Observational only; TS never re-inits a GUID
// or re-locks an asset.

/** Sidecar reconcile status, mirrored from the Rust `SidecarStatus`. */
export type AssetSidecarStatus = 'unchanged' | 'movedFile' | 'contentChanged' | 'missingSidecar';

/** One imported asset's source-trace projection (from its import manifest/sidecar). */
export interface AssetSourceTraceInput {
  /** The stable sidecar GUID, or null for a source not yet tracked. */
  readonly guid: string | null;
  readonly source: string;
  /** The catalog/mesh asset id the lock pins (trace endpoint). */
  readonly catalogId: string;
  readonly artifacts: readonly { readonly path: string; readonly hash: string }[];
  readonly status: AssetSidecarStatus;
}

/** A classified source-trace readout: identity, trackedness, and actionable drift. */
export interface AssetSourceTraceView {
  readonly guid: string | null;
  /** True when a stable GUID exists (the source is tracked). */
  readonly tracked: boolean;
  readonly source: string;
  readonly catalogId: string;
  readonly artifactCount: number;
  readonly status: AssetSidecarStatus;
  /** Content changed under a stable GUID — derived artifacts are stale. */
  readonly needsReimport: boolean;
  /** No GUID / no sidecar — `init` is required before the source is tracked. */
  readonly needsInit: boolean;
}

/** Build the source-trace read model for one imported asset. Pure. */
export function buildAssetSourceTrace(input: AssetSourceTraceInput): AssetSourceTraceView {
  return {
    guid: input.guid,
    tracked: input.guid !== null,
    source: input.source,
    catalogId: input.catalogId,
    artifactCount: input.artifacts.length,
    status: input.status,
    needsReimport: input.status === 'contentChanged',
    needsInit: input.guid === null || input.status === 'missingSidecar',
  };
}

/** Deterministic, greppable rendering of a source-trace view (golden-friendly). */
export function formatAssetSourceTrace(view: AssetSourceTraceView): string[] {
  return [
    `sourceTrace guid=${view.guid ?? '-'} tracked=${view.tracked} status=${view.status}`,
    `  source=${view.source} catalog=${view.catalogId} artifacts=${view.artifactCount}`,
    `  needsReimport=${view.needsReimport} needsInit=${view.needsInit}`,
  ];
}
