import { type EntityCapabilityFlags, type EntityClass } from '@asha/editor-tools';
import type { EntityAuthoringOutcome, EntityId } from '@asha/contracts';
/** Source provenance of an entity, as the inspector displays it. */
export type EntitySourceLabel = {
    readonly kind: 'sceneBootstrap';
    readonly node: number;
} | {
    readonly kind: 'runtimeCreated';
    readonly by: number | null;
} | {
    readonly kind: 'imported';
    readonly asset: string;
} | {
    readonly kind: 'diagnosticTooling';
} | {
    readonly kind: 'policyProposed';
    readonly by: number;
};
/**
 * One entity as the inspector reads it: its capability flags (for classification +
 * eligibility) plus its source provenance and labels (for the source trace). This
 * is a projection from authority, never authored here.
 */
export interface AuthoringEntityRecord extends EntityCapabilityFlags {
    readonly source: EntitySourceLabel;
    readonly labels: readonly number[];
}
/** The inspector's read of one entity: classification, eligibility, and relations. */
export interface EntityInspectionRow {
    readonly id: EntityId;
    readonly lifecycle: EntityCapabilityFlags['lifecycle'];
    readonly classes: readonly EntityClass[];
    readonly sourceKind: EntitySourceLabel['kind'];
    readonly capabilities: readonly string[];
    readonly relations: readonly string[];
    readonly transformEligible: boolean;
    readonly movementEligible: boolean;
    /** Stable accessibility/automation label for the row's primary control. */
    readonly controlLabel: string;
}
/** The whole inspector view: classified entity rows plus the last command result. */
export interface EntityInspectorView {
    readonly rows: readonly EntityInspectionRow[];
    /** A readout of the last authoring outcome (accept/reject + reason), or null. */
    readonly lastResult: {
        readonly accepted: boolean;
        readonly detail: string;
        readonly entity: EntityId;
    } | null;
    /** Per-class counts so a panel can summarize without rescanning. */
    readonly classCounts: Readonly<Record<EntityClass, number>>;
}
/**
 * Build the inspector read model from authority-sourced entity records (ascending
 * id order is the caller's responsibility — typically the facade hands them sorted)
 * and the last authoring outcome. Pure.
 */
export declare function buildEntityInspector(records: readonly AuthoringEntityRecord[], lastOutcome: EntityAuthoringOutcome | null): EntityInspectorView;
/** Deterministic, greppable text rendering of the inspector (golden-friendly). */
export declare function formatEntityInspector(view: EntityInspectorView): string[];
//# sourceMappingURL=entity-inspector.d.ts.map