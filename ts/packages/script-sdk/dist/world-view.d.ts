import type { EntityId, PolicyAssetStatus, PolicyAssetView, PolicyEntityView, PolicyTransform, PolicyWorldCommand, PolicyWorldSummary, PolicyWorldView, TagId } from '@asha/contracts';
export type { PolicyWorldView, PolicyEntityView, PolicyAssetView, PolicyAssetStatus, PolicyEntityLifecycle, PolicyEntitySource, PolicyTransform, PolicyWorldSummary, PolicyWorldCommand, PolicyWorldEvent, PolicyWorldOutcome, PolicyWorldRejection, } from '@asha/contracts';
/**
 * A world-layer policy: a pure function from the read-only world view to proposed
 * world commands. Like the script `Policy`, it receives no context object — no
 * clock, no RNG, no I/O. Determinism is a function of the view alone (a richer
 * deterministic envelope is added in #2393).
 */
export type WorldPolicy = (view: PolicyWorldView) => readonly PolicyWorldCommand[];
/**
 * A world-layer policy that consumes the deterministic envelope (#2393) for tick
 * context and seeded randomness. The envelope is the only source of time/random —
 * the same `(view, env)` always yields the same proposals.
 */
export type WorldPolicyWithEnv = (view: PolicyWorldView, env: import('./env.js').PolicyEnv) => readonly PolicyWorldCommand[];
/**
 * Builders that produce well-formed `PolicyWorldCommand` proposals without
 * hand-writing union literals. Each returns a generated `PolicyWorldCommand`, so
 * there is no parallel command shape to drift. A builder proposes; it never submits.
 */
export declare const worldCommands: {
    readonly setTransform: (entity: EntityId, transform: PolicyTransform) => PolicyWorldCommand;
    readonly addLabel: (entity: EntityId, label: TagId) => PolicyWorldCommand;
    readonly disable: (entity: EntityId) => PolicyWorldCommand;
    readonly noop: (note: string) => PolicyWorldCommand;
};
/**
 * Recompute the aggregate summary from a view's entities and assets. Keeping this
 * pure and derived means a fixture cannot carry a summary that disagrees with its
 * own contents.
 */
export declare function deriveSummary(tick: number, entities: readonly PolicyEntityView[], assets: readonly PolicyAssetView[]): PolicyWorldSummary;
/** The view of an empty world at a given tick. */
export declare function emptyWorldView(tick?: number): PolicyWorldView;
/**
 * Build a `PolicyWorldView` from partial parts, deriving the summary so it always
 * matches the contents. Scoped to fixtures and tests — production views come from
 * the Rust projector.
 */
export declare function makeWorldView(parts?: {
    readonly tick?: number;
    readonly entities?: readonly PolicyEntityView[];
    readonly assets?: readonly PolicyAssetView[];
}): PolicyWorldView;
/** Ergonomic, non-mutating reads over a `PolicyWorldView`. */
export declare const worldQuery: {
    readonly entity: (view: PolicyWorldView, id: EntityId) => PolicyEntityView | undefined;
    readonly hasEntity: (view: PolicyWorldView, id: EntityId) => boolean;
    readonly spatialEntities: (view: PolicyWorldView) => readonly PolicyEntityView[];
    readonly activeEntities: (view: PolicyWorldView) => readonly PolicyEntityView[];
    readonly entityHasLabel: (view: PolicyWorldView, id: EntityId, label: TagId) => boolean;
    readonly assetStatus: (view: PolicyWorldView, assetId: string) => PolicyAssetStatus | undefined;
};
//# sourceMappingURL=world-view.d.ts.map