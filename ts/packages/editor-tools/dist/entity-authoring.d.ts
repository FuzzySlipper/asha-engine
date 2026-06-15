import type { AuthoringCapability, AuthoringSource, AuthoringTransform, EntityAuthoringCommand, EntityAuthoringOutcome, EntityId, TagId } from '@asha/contracts';
export declare const IDENTITY_AUTHORING_TRANSFORM: AuthoringTransform;
/** Propose creating a generic entity with a source provenance and label set. */
export declare function proposeCreateEntity(id: EntityId, source: AuthoringSource, labels?: readonly TagId[]): EntityAuthoringCommand;
/** Propose destroying (tombstoning) an entity. */
export declare function proposeDestroyEntity(id: EntityId): EntityAuthoringCommand;
/** Propose attaching a capability (transform/render/collision/bounds) to a live entity. */
export declare function proposeAttachCapability(id: EntityId, capability: AuthoringCapability): EntityAuthoringCommand;
/** Propose overwriting a transform-eligible entity's runtime transform. */
export declare function proposeSetEntityTransform(id: EntityId, transform: AuthoringTransform): EntityAuthoringCommand;
/** Propose a kinematic move by a world-space delta (requires transform + collider). */
export declare function proposeMove(id: EntityId, delta: readonly [number, number, number]): EntityAuthoringCommand;
/** Propose a transform-attachment (child world transform derives from parent). */
export declare function proposeAttachTransformParent(child: EntityId, parent: EntityId): EntityAuthoringCommand;
/** Propose placing `member` inside `container` (logical containment, not transform). */
export declare function proposeSetContainment(member: EntityId, container: EntityId): EntityAuthoringCommand;
/** Propose recording a source-ancestry trace (read-only provenance, not a graph). */
export declare function proposeSetDerivedFrom(derived: EntityId, origin: EntityId): EntityAuthoringCommand;
/** Whether authority accepted the proposal, plus a classified reason when refused. */
export interface AuthoringFeedback {
    readonly accepted: boolean;
    /** The accepted event kind (e.g. `created`), or the rejection reason (e.g. `notTransformEligible`). */
    readonly detail: string;
    /** The entity the outcome concerns. */
    readonly entity: EntityId;
}
/**
 * Turn an authoritative `EntityAuthoringOutcome` (produced by Rust validation of a
 * proposal) into an accept/reject readout the UI can display. The UI never decides
 * acceptance — it reflects this.
 */
export declare function summarizeAuthoringOutcome(outcome: EntityAuthoringOutcome): AuthoringFeedback;
/**
 * The capability/relation flags a UI/devtools surface reads for one entity. This is
 * a projection the app gets from the runtime facade (Rust authority) — never a
 * second source of truth. All fields are derived from authority capability tables.
 */
export interface EntityCapabilityFlags {
    readonly id: EntityId;
    readonly lifecycle: 'active' | 'disabled' | 'tombstoned';
    readonly hasTransform: boolean;
    readonly hasRender: boolean;
    readonly hasCollision: boolean;
    readonly staticCollider: boolean;
    readonly hasBounds: boolean;
    readonly containedIn: EntityId | null;
    readonly transformParent: EntityId | null;
    readonly derivedFrom: EntityId | null;
}
/** The abstract fixture vocabulary class a UI buckets an entity into for display. */
export type EntityClass = 'spatialRendered' | 'spatialCollider' | 'nonSpatialLogical' | 'contained' | 'attached' | 'tombstoned';
/**
 * Classify an entity into the abstract fixture vocabulary from its capability flags.
 * Returns every class that applies (an entity may be both spatial-rendered and
 * attached, say) so the outliner can label it precisely. Pure.
 */
export declare function classifyEntity(flags: EntityCapabilityFlags): readonly EntityClass[];
/**
 * Whether a transform/move control should be enabled for an entity, with a reason
 * when not — capability discipline mirrored on the UI side so an ineligible control
 * is disabled *and explained* before any proposal is built. Authority is still the
 * decider; this only prevents obviously-doomed proposals and labels the control.
 */
export declare function transformEligibility(flags: EntityCapabilityFlags): {
    readonly eligible: boolean;
    readonly reason: string | null;
};
/** Whether a kinematic move is eligible (requires transform + a non-static collider). */
export declare function movementEligibility(flags: EntityCapabilityFlags): {
    readonly eligible: boolean;
    readonly reason: string | null;
};
//# sourceMappingURL=entity-authoring.d.ts.map