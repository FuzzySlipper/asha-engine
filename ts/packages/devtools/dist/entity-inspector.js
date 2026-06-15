// @asha/devtools — generic entity authoring inspector read model (#2485).
//
// Tool-only and **observational**: it projects an authority-sourced entity view
// (capability flags, relations, source provenance) plus the last authoring command
// outcome into a deterministic, agent-readable read model + text. It never mutates
// authority and holds no second source of truth — the records it reads come from
// the runtime facade (Rust). Capability classification + eligibility come from
// `@asha/editor-tools` so the UI and the authority validator agree on vocabulary.
import { classifyEntity, movementEligibility, summarizeAuthoringOutcome, transformEligibility, } from '@asha/editor-tools';
function capabilityList(r) {
    const caps = [];
    if (r.hasTransform)
        caps.push('transform');
    if (r.hasRender)
        caps.push('render');
    if (r.hasCollision)
        caps.push(r.staticCollider ? 'collision(static)' : 'collision');
    if (r.hasBounds)
        caps.push('bounds');
    return caps;
}
function relationList(r) {
    const rels = [];
    if (r.transformParent !== null)
        rels.push(`transformParent=${r.transformParent}`);
    if (r.containedIn !== null)
        rels.push(`containedIn=${r.containedIn}`);
    if (r.derivedFrom !== null)
        rels.push(`derivedFrom=${r.derivedFrom}`);
    return rels;
}
function emptyClassCounts() {
    return {
        spatialRendered: 0,
        spatialCollider: 0,
        nonSpatialLogical: 0,
        contained: 0,
        attached: 0,
        tombstoned: 0,
    };
}
/**
 * Build the inspector read model from authority-sourced entity records (ascending
 * id order is the caller's responsibility — typically the facade hands them sorted)
 * and the last authoring outcome. Pure.
 */
export function buildEntityInspector(records, lastOutcome) {
    const classCounts = emptyClassCounts();
    const rows = records.map((r) => {
        const classes = classifyEntity(r);
        for (const c of classes) {
            classCounts[c] += 1;
        }
        return {
            id: r.id,
            lifecycle: r.lifecycle,
            classes,
            sourceKind: r.source.kind,
            capabilities: capabilityList(r),
            relations: relationList(r),
            transformEligible: transformEligibility(r).eligible,
            movementEligible: movementEligibility(r).eligible,
            controlLabel: `entity-${r.id}-authoring-controls`,
        };
    });
    return {
        rows,
        lastResult: lastOutcome ? summarizeAuthoringOutcome(lastOutcome) : null,
        classCounts,
    };
}
/** Deterministic, greppable text rendering of the inspector (golden-friendly). */
export function formatEntityInspector(view) {
    const lines = [];
    for (const row of view.rows) {
        const caps = row.capabilities.length > 0 ? row.capabilities.join(',') : '-';
        const rels = row.relations.length > 0 ? row.relations.join(',') : '-';
        lines.push(`entity ${row.id} ${row.lifecycle} source=${row.sourceKind} ` +
            `classes=[${row.classes.join(',')}] caps=[${caps}] rels=[${rels}] ` +
            `transformEligible=${row.transformEligible} movementEligible=${row.movementEligible}`);
    }
    if (view.lastResult) {
        const r = view.lastResult;
        lines.push(`lastResult ${r.accepted ? 'accepted' : 'rejected'} ${r.detail} entity=${r.entity}`);
    }
    return lines;
}
//# sourceMappingURL=entity-inspector.js.map