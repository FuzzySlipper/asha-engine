// @asha/editor-tools — proposal-only generic entity authoring controls (#2485).
//
// Builders that turn authoring intent (create an abstract entity, attach a
// capability, transform/move it, attach/contain/relate, destroy it) into typed
// **proposals** built from generated `@asha/contracts` entity-authoring types.
// They are pure and they NEVER submit, validate, or mutate authority — Rust
// (`svc-entity-authoring`) owns validation and the runtime facade owns submission.
//
// `summarizeAuthoringOutcome` turns the authoritative `EntityAuthoringOutcome`
// (from Rust) into an accept/reject readout — the UI reflects that, it does not
// decide it. `classifyEntity` is a pure projection that buckets an entity into the
// abstract fixture vocabulary (spatial-rendered, collider, logical, contained,
// attached, kinematic) from its capability/relation flags, for devtools/outliner
// display. No product-domain nouns appear here.
export const IDENTITY_AUTHORING_TRANSFORM = {
    translation: [0, 0, 0],
    rotation: [0, 0, 0, 1],
    scale: [1, 1, 1],
};
// ── Proposal builders (pure) ───────────────────────────────────────────────────
/** Propose creating a generic entity with a source provenance and label set. */
export function proposeCreateEntity(id, source, labels = []) {
    return { kind: 'create', id, source, labels: [...labels] };
}
/** Propose destroying (tombstoning) an entity. */
export function proposeDestroyEntity(id) {
    return { kind: 'destroy', id };
}
/** Propose attaching a capability (transform/render/collision/bounds) to a live entity. */
export function proposeAttachCapability(id, capability) {
    return { kind: 'attachCapability', id, capability };
}
/** Propose overwriting a transform-eligible entity's runtime transform. */
export function proposeSetEntityTransform(id, transform) {
    return { kind: 'setTransform', id, transform };
}
/** Propose a kinematic move by a world-space delta (requires transform + collider). */
export function proposeMove(id, delta) {
    return { kind: 'move', id, delta: [delta[0], delta[1], delta[2]] };
}
/** Propose a transform-attachment (child world transform derives from parent). */
export function proposeAttachTransformParent(child, parent) {
    return { kind: 'attachTransformParent', child, parent };
}
/** Propose placing `member` inside `container` (logical containment, not transform). */
export function proposeSetContainment(member, container) {
    return { kind: 'setContainment', member, container };
}
/** Propose recording a source-ancestry trace (read-only provenance, not a graph). */
export function proposeSetDerivedFrom(derived, origin) {
    return { kind: 'setDerivedFrom', derived, origin };
}
/**
 * Turn an authoritative `EntityAuthoringOutcome` (produced by Rust validation of a
 * proposal) into an accept/reject readout the UI can display. The UI never decides
 * acceptance — it reflects this.
 */
export function summarizeAuthoringOutcome(outcome) {
    if (outcome.status === 'accepted') {
        return { accepted: true, detail: outcome.event.kind, entity: outcome.event.entity };
    }
    return { accepted: false, detail: outcome.rejection.reason, entity: outcome.rejection.entity };
}
/**
 * Classify an entity into the abstract fixture vocabulary from its capability flags.
 * Returns every class that applies (an entity may be both spatial-rendered and
 * attached, say) so the outliner can label it precisely. Pure.
 */
export function classifyEntity(flags) {
    if (flags.lifecycle === 'tombstoned') {
        return ['tombstoned'];
    }
    const classes = [];
    if (flags.hasTransform && flags.hasRender) {
        classes.push('spatialRendered');
    }
    if (flags.hasTransform && flags.hasCollision && !flags.hasRender) {
        classes.push('spatialCollider');
    }
    if (!flags.hasTransform) {
        classes.push('nonSpatialLogical');
    }
    if (flags.containedIn !== null) {
        classes.push('contained');
    }
    if (flags.transformParent !== null) {
        classes.push('attached');
    }
    return classes;
}
/**
 * Whether a transform/move control should be enabled for an entity, with a reason
 * when not — capability discipline mirrored on the UI side so an ineligible control
 * is disabled *and explained* before any proposal is built. Authority is still the
 * decider; this only prevents obviously-doomed proposals and labels the control.
 */
export function transformEligibility(flags) {
    if (flags.lifecycle === 'tombstoned') {
        return { eligible: false, reason: 'tombstoned' };
    }
    if (flags.lifecycle === 'disabled') {
        return { eligible: false, reason: 'disabled' };
    }
    if (!flags.hasTransform) {
        return { eligible: false, reason: 'notTransformEligible' };
    }
    if (flags.hasCollision && flags.staticCollider) {
        return { eligible: false, reason: 'immovable' };
    }
    return { eligible: true, reason: null };
}
/** Whether a kinematic move is eligible (requires transform + a non-static collider). */
export function movementEligibility(flags) {
    const transform = transformEligibility(flags);
    if (!transform.eligible) {
        // A static collider is `immovable` for transform but `immovable` for movement too;
        // a missing transform is `notSpatial` in movement terms.
        if (transform.reason === 'notTransformEligible') {
            return { eligible: false, reason: 'notSpatial' };
        }
        return transform;
    }
    if (!flags.hasCollision) {
        return { eligible: false, reason: 'noCollider' };
    }
    return { eligible: true, reason: null };
}
//# sourceMappingURL=entity-authoring.js.map