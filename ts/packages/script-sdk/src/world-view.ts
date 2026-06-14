// @asha/script-sdk — the read-only world-layer view a policy is handed (#2391).
//
// This is the world-layer counterpart to the script `PolicyView`: a generated,
// read-only projection of entities/transforms/source/assets a constrained policy
// reasons over. It adds *ergonomics only* on top of @asha/contracts — it never
// duplicates the generated shapes, never mutates, and never reaches for the
// renderer, UI, bridge, clock, or ambient randomness. The Rust `svc-policy-view`
// projector is the sole producer of a real view; the helpers here are for fixtures
// and reads.

import type {
  EntityId,
  PolicyAssetStatus,
  PolicyAssetView,
  PolicyEntityView,
  PolicyTransform,
  PolicyWorldCommand,
  PolicyWorldSummary,
  PolicyWorldView,
  TagId,
} from '@asha/contracts';

// Re-export the generated world-view + command surface so a policy author has one
// import. The command/event/outcome shapes mirror the Rust `svc-policy-view`
// validator; a policy proposes commands, authority returns the outcome.
export type {
  PolicyWorldView,
  PolicyEntityView,
  PolicyAssetView,
  PolicyAssetStatus,
  PolicyEntityLifecycle,
  PolicyEntitySource,
  PolicyTransform,
  PolicyWorldSummary,
  PolicyWorldCommand,
  PolicyWorldEvent,
  PolicyWorldOutcome,
  PolicyWorldRejection,
} from '@asha/contracts';

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
export type WorldPolicyWithEnv = (
  view: PolicyWorldView,
  env: import('./env.js').PolicyEnv,
) => readonly PolicyWorldCommand[];

// ── World command builders ───────────────────────────────────────────────────────

/**
 * Builders that produce well-formed `PolicyWorldCommand` proposals without
 * hand-writing union literals. Each returns a generated `PolicyWorldCommand`, so
 * there is no parallel command shape to drift. A builder proposes; it never submits.
 */
export const worldCommands = {
  setTransform: (entity: EntityId, transform: PolicyTransform): PolicyWorldCommand => ({
    kind: 'requestSetTransform',
    entity,
    transform,
  }),
  addLabel: (entity: EntityId, label: TagId): PolicyWorldCommand => ({
    kind: 'requestAddLabel',
    entity,
    label,
  }),
  disable: (entity: EntityId): PolicyWorldCommand => ({ kind: 'requestDisable', entity }),
  noop: (note: string): PolicyWorldCommand => ({ kind: 'noopMarker', note }),
} as const;

// ── Summary derivation (deterministic) ──────────────────────────────────────────

/**
 * Recompute the aggregate summary from a view's entities and assets. Keeping this
 * pure and derived means a fixture cannot carry a summary that disagrees with its
 * own contents.
 */
export function deriveSummary(
  tick: number,
  entities: readonly PolicyEntityView[],
  assets: readonly PolicyAssetView[],
): PolicyWorldSummary {
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
export function emptyWorldView(tick = 0): PolicyWorldView {
  return { tick, entities: [], assets: [], summary: deriveSummary(tick, [], []) };
}

/**
 * Build a `PolicyWorldView` from partial parts, deriving the summary so it always
 * matches the contents. Scoped to fixtures and tests — production views come from
 * the Rust projector.
 */
export function makeWorldView(
  parts: { readonly tick?: number; readonly entities?: readonly PolicyEntityView[]; readonly assets?: readonly PolicyAssetView[] } = {},
): PolicyWorldView {
  const tick = parts.tick ?? 0;
  const entities = parts.entities ?? [];
  const assets = parts.assets ?? [];
  return { tick, entities, assets, summary: deriveSummary(tick, entities, assets) };
}

// ── Read-only queries ────────────────────────────────────────────────────────────

/** Ergonomic, non-mutating reads over a `PolicyWorldView`. */
export const worldQuery = {
  entity: (view: PolicyWorldView, id: EntityId): PolicyEntityView | undefined =>
    view.entities.find((e) => e.id === id),

  hasEntity: (view: PolicyWorldView, id: EntityId): boolean =>
    view.entities.some((e) => e.id === id),

  spatialEntities: (view: PolicyWorldView): readonly PolicyEntityView[] =>
    view.entities.filter((e) => e.spatial),

  activeEntities: (view: PolicyWorldView): readonly PolicyEntityView[] =>
    view.entities.filter((e) => e.lifecycle === 'active'),

  entityHasLabel: (view: PolicyWorldView, id: EntityId, label: TagId): boolean =>
    view.entities.find((e) => e.id === id)?.labels.includes(label) ?? false,

  assetStatus: (view: PolicyWorldView, assetId: string): PolicyAssetStatus | undefined =>
    view.assets.find((a) => a.id === assetId)?.status,
} as const;
