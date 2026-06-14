// @asha/policy-examples — abstract, constrained policy fixtures.
//
// Policies here use only the Phase 1/2 abstract vocabulary (Entity, Subject,
// Process, Mode, Signal, Tag). They are pure functions of the read-only view,
// propose commands only, and obey the ts-policy sandbox (no wall-clock, no
// ambient randomness, no DOM/renderer/bridge/Electron/filesystem/network).

import {
  commands,
  signalId,
  tagId,
  worldCommands,
  worldQuery,
  type Policy,
  type SignalId,
  type TagId,
  type WorldPolicy,
} from '@asha/script-sdk';

/** Configuration for {@link thresholdPolicy}. */
export interface ThresholdConfig {
  /** The tag whose bearers are counted. */
  readonly watchTag: TagId;
  /** The count at or above which the signal is raised. */
  readonly threshold: number;
  /** The signal proposed when the threshold is reached. */
  readonly raiseSignal: SignalId;
}

/**
 * A threshold policy: when at least `threshold` entities carry `watchTag`, it
 * proposes defining `raiseSignal`. It is deterministic and idempotent — once the
 * signal is already defined in the view, it proposes nothing further, so
 * re-running on the resulting state is a fixed point.
 *
 * This is the canonical fixture proving the Phase 3 loop: a policy reads a
 * read-only view and returns a generated `PolicyCommand`.
 */
export function thresholdPolicy(config: ThresholdConfig): Policy {
  return (view) => {
    // Idempotent: nothing to do once the signal exists.
    if (view.signals.includes(config.raiseSignal)) {
      return [];
    }
    const bearers = view.entities.filter((e) =>
      e.tags.includes(config.watchTag),
    ).length;
    if (bearers < config.threshold) {
      return [];
    }
    return [commands.defineSignal(config.raiseSignal)];
  };
}

/**
 * The named fixture instance used by tests and the `harness/fixtures` golden
 * inputs/outputs: raise signal `1` once at least three entities carry tag `1`.
 */
export const tagCountThreshold: Policy = thresholdPolicy({
  watchTag: tagId(1),
  threshold: 3,
  raiseSignal: signalId(1),
});

// ── World-layer policy fixtures (#2392) ──────────────────────────────────────────

/** Configuration for {@link labelSpatialPolicy}. */
export interface LabelSpatialConfig {
  /** The label proposed for every active spatial entity that lacks it. */
  readonly label: TagId;
}

/**
 * A world-layer policy over the generated `PolicyWorldView`: it proposes adding
 * `label` to every active, spatial entity that does not already carry it. It is
 * deterministic and idempotent — once every spatial entity is labelled, it
 * proposes nothing, so re-running on the accepted result is a fixed point.
 *
 * This is the canonical fixture proving the world-layer loop: a policy reads the
 * read-only world view and returns generated `PolicyWorldCommand` proposals. It
 * never mutates — authority (Rust `svc-policy-view`) validates and applies.
 */
export function labelSpatialPolicy(config: LabelSpatialConfig): WorldPolicy {
  return (view) =>
    worldQuery
      .activeEntities(view)
      .filter((entity) => entity.spatial && !entity.labels.includes(config.label))
      .map((entity) => worldCommands.addLabel(entity.id, config.label));
}

/** The named fixture instance: label every active spatial entity with tag `9`. */
export const labelSpatialEntities: WorldPolicy = labelSpatialPolicy({ label: tagId(9) });
