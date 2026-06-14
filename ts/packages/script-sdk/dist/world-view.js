// @asha/script-sdk — the read-only world-layer view a policy is handed (#2391).
//
// This is the world-layer counterpart to the script `PolicyView`: a generated,
// read-only projection of entities/transforms/source/assets a constrained policy
// reasons over. It adds *ergonomics only* on top of @asha/contracts — it never
// duplicates the generated shapes, never mutates, and never reaches for the
// renderer, UI, bridge, clock, or ambient randomness. The Rust `svc-policy-view`
// projector is the sole producer of a real view; the helpers here are for fixtures
// and reads.
// ── World command builders ───────────────────────────────────────────────────────
/**
 * Builders that produce well-formed `PolicyWorldCommand` proposals without
 * hand-writing union literals. Each returns a generated `PolicyWorldCommand`, so
 * there is no parallel command shape to drift. A builder proposes; it never submits.
 */
export const worldCommands = {
    setTransform: (entity, transform) => ({
        kind: 'requestSetTransform',
        entity,
        transform,
    }),
    addLabel: (entity, label) => ({
        kind: 'requestAddLabel',
        entity,
        label,
    }),
    disable: (entity) => ({ kind: 'requestDisable', entity }),
    noop: (note) => ({ kind: 'noopMarker', note }),
};
// ── Summary derivation (deterministic) ──────────────────────────────────────────
/**
 * Recompute the aggregate summary from a view's entities and assets. Keeping this
 * pure and derived means a fixture cannot carry a summary that disagrees with its
 * own contents.
 */
export function deriveSummary(tick, entities, assets) {
    return {
        tick,
        activeEntities: entities.filter((e) => e.lifecycle === 'active').length,
        spatialEntities: entities.filter((e) => e.spatial).length,
        assetCount: assets.length,
        missingAssets: assets.filter((a) => a.status === 'missing').length,
    };
}
// ── Fixtures (policy tests only) ─────────────────────────────────────────────────
/** The view of an empty world at a given tick. */
export function emptyWorldView(tick = 0) {
    return { tick, entities: [], assets: [], summary: deriveSummary(tick, [], []) };
}
/**
 * Build a `PolicyWorldView` from partial parts, deriving the summary so it always
 * matches the contents. Scoped to fixtures and tests — production views come from
 * the Rust projector.
 */
export function makeWorldView(parts = {}) {
    const tick = parts.tick ?? 0;
    const entities = parts.entities ?? [];
    const assets = parts.assets ?? [];
    return { tick, entities, assets, summary: deriveSummary(tick, entities, assets) };
}
// ── Read-only queries ────────────────────────────────────────────────────────────
/** Ergonomic, non-mutating reads over a `PolicyWorldView`. */
export const worldQuery = {
    entity: (view, id) => view.entities.find((e) => e.id === id),
    hasEntity: (view, id) => view.entities.some((e) => e.id === id),
    spatialEntities: (view) => view.entities.filter((e) => e.spatial),
    activeEntities: (view) => view.entities.filter((e) => e.lifecycle === 'active'),
    entityHasLabel: (view, id, label) => view.entities.find((e) => e.id === id)?.labels.includes(label) ?? false,
    assetStatus: (view, assetId) => view.assets.find((a) => a.id === assetId)?.status,
};
//# sourceMappingURL=world-view.js.map