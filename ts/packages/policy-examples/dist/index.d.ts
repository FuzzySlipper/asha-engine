import { type Policy, type SignalId, type TagId, type WorldPolicy } from '@asha/script-sdk';
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
export declare function thresholdPolicy(config: ThresholdConfig): Policy;
/**
 * The named fixture instance used by tests and the `harness/fixtures` golden
 * inputs/outputs: raise signal `1` once at least three entities carry tag `1`.
 */
export declare const tagCountThreshold: Policy;
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
export declare function labelSpatialPolicy(config: LabelSpatialConfig): WorldPolicy;
/** The named fixture instance: label every active spatial entity with tag `9`. */
export declare const labelSpatialEntities: WorldPolicy;
//# sourceMappingURL=index.d.ts.map