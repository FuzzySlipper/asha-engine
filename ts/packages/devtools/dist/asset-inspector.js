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
function depIds(deps) {
    return deps.map((d) => d.id);
}
function detectDependencyCycles(edges) {
    const cycles = [];
    const state = new Map();
    const stack = [];
    const visit = (id) => {
        state.set(id, 'visiting');
        stack.push(id);
        for (const dep of edges.get(id) ?? []) {
            const depState = state.get(dep);
            if (depState === undefined) {
                visit(dep);
            }
            else if (depState === 'visiting') {
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
function describeCatalogError(error) {
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
export function buildCatalogModel(catalog, validation) {
    const present = new Set(catalog.entries.map((e) => e.id));
    const dependencyEdges = new Map();
    const entries = catalog.entries.map((entry) => {
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
    const structuralIssues = (validation?.errors ?? []).map((error) => ({
        code: error.code,
        id: error.id,
        detail: describeCatalogError(error),
        cyclePath: error.cyclePath,
    }));
    return { entries, dependencyEdges, cycles, structuralIssues };
}
function lockSeverity(code) {
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
function describeLockFinding(finding) {
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
export function buildLockDriftModel(report) {
    const findings = report.findings.map((finding) => ({
        id: finding.id,
        code: finding.code,
        severity: lockSeverity(finding.code),
        detail: describeLockFinding(finding),
    }));
    return { findings, hasDrift: findings.some((f) => f.severity !== 'info') };
}
/** Inspect a catalog entry's material projection, or null for a non-material asset. */
export function inspectMaterial(entry) {
    if (entry.material === null) {
        return null;
    }
    const projection = entry.material;
    // Returned as two distinct objects — never spread into one mixed record.
    return { render: projection.render, collision: projection.collision };
}
/** Classify a fallback decision for display (never authorizes a substitution). */
export function classifyFallback(decision) {
    if (decision.outcome === 'useFallback') {
        return { outcome: 'useFallback', reason: decision.reason, visual: decision.visual, fallbackUsed: true };
    }
    return { outcome: decision.outcome, reason: decision.reason, visual: null, fallbackUsed: false };
}
/**
 * Report which catalog entries are impacted by a change to `changedId` — every
 * asset that transitively depends on it. Pure; reads the catalog's declared
 * dependency edges and never mutates.
 */
export function impactOfChangedAsset(catalog, changedId) {
    const present = new Set(catalog.entries.map((e) => e.id));
    // Reverse adjacency: dependency id → assets that declare it.
    const dependentsOf = new Map();
    for (const entry of catalog.entries) {
        for (const dep of entry.dependencies) {
            const bucket = dependentsOf.get(dep.id);
            if (bucket === undefined) {
                dependentsOf.set(dep.id, [entry.id]);
            }
            else {
                bucket.push(entry.id);
            }
        }
    }
    const impacted = new Set();
    const queue = [changedId];
    while (queue.length > 0) {
        const current = queue.shift();
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
/** Build the source-trace read model for one imported asset. Pure. */
export function buildAssetSourceTrace(input) {
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
export function formatAssetSourceTrace(view) {
    return [
        `sourceTrace guid=${view.guid ?? '-'} tracked=${view.tracked} status=${view.status}`,
        `  source=${view.source} catalog=${view.catalogId} artifacts=${view.artifactCount}`,
        `  needsReimport=${view.needsReimport} needsInit=${view.needsInit}`,
    ];
}
//# sourceMappingURL=asset-inspector.js.map