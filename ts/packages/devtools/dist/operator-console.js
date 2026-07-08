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
/** Classify a diagnostic into its owning lane (scope first, with code overrides). */
export function classifyLane(report) {
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
        case 'worldBundle': // vocab-allow: generated diagnostic scope keeps legacy name until #5049.
        case 'worldComposition':
            return 'persistenceReplay';
    }
}
const SEVERITY_RANK = { info: 0, warning: 1, error: 2, fatal: 3 };
function worse(a, b) {
    return SEVERITY_RANK[a] >= SEVERITY_RANK[b] ? a : b;
}
function classifySourceTrace(t) {
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
export function buildOperatorConsole(input) {
    // Roll diagnostics up by owning lane.
    const byLane = new Map();
    let worstOverall = 'info';
    for (const report of input.diagnostics.reports) {
        const lane = classifyLane(report);
        const prior = byLane.get(lane);
        byLane.set(lane, {
            count: (prior?.count ?? 0) + 1,
            maxSeverity: prior ? worse(prior.maxSeverity, report.severity) : report.severity,
        });
        worstOverall = worse(worstOverall, report.severity);
    }
    const laneFailures = [...byLane.entries()]
        .map(([lane, v]) => ({ lane, count: v.count, maxSeverity: v.maxSeverity }))
        .sort((a, b) => a.lane.localeCompare(b.lane));
    const resources = input.resources
        ? {
            ...input.resources,
            // A created/disposed imbalance beyond live handles hints at a leak.
            suspectedLeak: input.resources.resourcesCreated - input.resources.resourcesDisposed > input.resources.liveHandles,
        }
        : null;
    const ready = input.runtime.mode !== 'unavailable' && SEVERITY_RANK[worstOverall] < SEVERITY_RANK.error;
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
export function toOperatorJson(model) {
    return JSON.stringify(model, null, 2);
}
/** Deterministic, greppable text projection of the operator console. */
export function formatOperatorConsole(model) {
    const lines = [];
    const r = model.runtime;
    lines.push(`runtime mode=${r.mode} world=${r.loadedProjectBundleId ?? '-'} hash=${r.worldHash ?? '-'} ` +
        `protocol=${r.protocolVersion ?? '-'} schema=${r.schemaVersion ?? '-'} ready=${model.ready}`);
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
        lines.push(`resources handles=${model.resources.liveHandles} geometries=${model.resources.geometries} ` +
            `materials=${model.resources.materials} fallbacks=${model.resources.fallbackMaterials} ` +
            `suspectedLeak=${model.resources.suspectedLeak}`);
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
//# sourceMappingURL=operator-console.js.map