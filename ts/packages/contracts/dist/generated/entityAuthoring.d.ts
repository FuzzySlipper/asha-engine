import type { EntityId, TagId, ProcessId, SubjectId } from './ids.js';
import type { SceneNodeId } from './scene.js';
export interface AuthoringTransform {
    readonly translation: readonly [number, number, number];
    readonly rotation: readonly [number, number, number, number];
    readonly scale: readonly [number, number, number];
}
export type AuthoringSource = {
    readonly kind: 'sceneBootstrap';
    readonly node: SceneNodeId;
} | {
    readonly kind: 'runtimeCreated';
    readonly by: ProcessId | null;
} | {
    readonly kind: 'imported';
    readonly asset: string;
} | {
    readonly kind: 'diagnosticTooling';
} | {
    readonly kind: 'policyProposed';
    readonly by: SubjectId;
};
export type AuthoringCapability = {
    readonly kind: 'transform';
    readonly transform: AuthoringTransform;
} | {
    readonly kind: 'render';
    readonly visible: boolean;
} | {
    readonly kind: 'collision';
    readonly staticCollider: boolean;
} | {
    readonly kind: 'bounds';
    readonly min: readonly [number, number, number];
    readonly max: readonly [number, number, number];
};
export type EntityAuthoringCommand = {
    readonly kind: 'create';
    readonly id: EntityId;
    readonly source: AuthoringSource;
    readonly labels: readonly TagId[];
} | {
    readonly kind: 'destroy';
    readonly id: EntityId;
} | {
    readonly kind: 'disable';
    readonly id: EntityId;
} | {
    readonly kind: 'enable';
    readonly id: EntityId;
} | {
    readonly kind: 'addLabel';
    readonly id: EntityId;
    readonly tag: TagId;
} | {
    readonly kind: 'removeLabel';
    readonly id: EntityId;
    readonly tag: TagId;
} | {
    readonly kind: 'attachCapability';
    readonly id: EntityId;
    readonly capability: AuthoringCapability;
} | {
    readonly kind: 'setTransform';
    readonly id: EntityId;
    readonly transform: AuthoringTransform;
} | {
    readonly kind: 'move';
    readonly id: EntityId;
    readonly delta: readonly [number, number, number];
} | {
    readonly kind: 'attachTransformParent';
    readonly child: EntityId;
    readonly parent: EntityId;
} | {
    readonly kind: 'detachTransformParent';
    readonly child: EntityId;
} | {
    readonly kind: 'setContainment';
    readonly member: EntityId;
    readonly container: EntityId;
} | {
    readonly kind: 'clearContainment';
    readonly member: EntityId;
} | {
    readonly kind: 'setDerivedFrom';
    readonly derived: EntityId;
    readonly origin: EntityId;
};
export type AuthoringEventKind = 'created' | 'destroyed' | 'disabled' | 'enabled' | 'labelAdded' | 'labelRemoved' | 'capabilityAttached' | 'transformSet' | 'moved' | 'relationSet' | 'relationCleared';
export interface EntityAuthoringEvent {
    readonly kind: AuthoringEventKind;
    readonly entity: EntityId;
}
export type AuthoringRejectionReason = 'unknownEntity' | 'alreadyExists' | 'idRetired' | 'tombstoned' | 'entityNotAlive' | 'invalidTransition' | 'labelAlreadyPresent' | 'labelAbsent' | 'notTransformEligible' | 'immovable' | 'nonFinite' | 'notSpatial' | 'noCollider' | 'selfRelation' | 'relationCycle' | 'endpointNotTransformEligible' | 'noSuchRelation' | 'projectionOnly' | 'invalidAsset';
export interface EntityAuthoringRejection {
    readonly reason: AuthoringRejectionReason;
    readonly entity: EntityId;
}
export type EntityAuthoringOutcome = {
    readonly status: 'accepted';
    readonly event: EntityAuthoringEvent;
} | {
    readonly status: 'rejected';
    readonly rejection: EntityAuthoringRejection;
};
//# sourceMappingURL=entityAuthoring.d.ts.map