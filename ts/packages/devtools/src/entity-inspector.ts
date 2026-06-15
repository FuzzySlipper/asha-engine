// @asha/devtools — generic entity authoring inspector read model (#2485).
//
// Tool-only and **observational**: it projects an authority-sourced entity view
// (capability flags, relations, source provenance) plus the last authoring command
// outcome into a deterministic, agent-readable read model + text. It never mutates
// authority and holds no second source of truth — the records it reads come from
// the runtime facade (Rust). Capability classification + eligibility come from
// `@asha/editor-tools` so the UI and the authority validator agree on vocabulary.

import {
  classifyEntity,
  movementEligibility,
  summarizeAuthoringOutcome,
  transformEligibility,
  type EntityCapabilityFlags,
  type EntityClass,
} from '@asha/editor-tools';
import type { EntityAuthoringOutcome, EntityId } from '@asha/contracts';

/** Source provenance of an entity, as the inspector displays it. */
export type EntitySourceLabel =
  | { readonly kind: 'sceneBootstrap'; readonly node: number }
  | { readonly kind: 'runtimeCreated'; readonly by: number | null }
  | { readonly kind: 'imported'; readonly asset: string }
  | { readonly kind: 'diagnosticTooling' }
  | { readonly kind: 'policyProposed'; readonly by: number };

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
  readonly lastResult: { readonly accepted: boolean; readonly detail: string; readonly entity: EntityId } | null;
  /** Per-class counts so a panel can summarize without rescanning. */
  readonly classCounts: Readonly<Record<EntityClass, number>>;
}

function capabilityList(r: AuthoringEntityRecord): string[] {
  const caps: string[] = [];
  if (r.hasTransform) caps.push('transform');
  if (r.hasRender) caps.push('render');
  if (r.hasCollision) caps.push(r.staticCollider ? 'collision(static)' : 'collision');
  if (r.hasBounds) caps.push('bounds');
  return caps;
}

function relationList(r: AuthoringEntityRecord): string[] {
  const rels: string[] = [];
  if (r.transformParent !== null) rels.push(`transformParent=${r.transformParent as number}`);
  if (r.containedIn !== null) rels.push(`containedIn=${r.containedIn as number}`);
  if (r.derivedFrom !== null) rels.push(`derivedFrom=${r.derivedFrom as number}`);
  return rels;
}

function emptyClassCounts(): Record<EntityClass, number> {
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
export function buildEntityInspector(
  records: readonly AuthoringEntityRecord[],
  lastOutcome: EntityAuthoringOutcome | null,
): EntityInspectorView {
  const classCounts = emptyClassCounts();
  const rows: EntityInspectionRow[] = records.map((r) => {
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
      controlLabel: `entity-${r.id as number}-authoring-controls`,
    };
  });
  return {
    rows,
    lastResult: lastOutcome ? summarizeAuthoringOutcome(lastOutcome) : null,
    classCounts,
  };
}

/** Deterministic, greppable text rendering of the inspector (golden-friendly). */
export function formatEntityInspector(view: EntityInspectorView): string[] {
  const lines: string[] = [];
  for (const row of view.rows) {
    const caps = row.capabilities.length > 0 ? row.capabilities.join(',') : '-';
    const rels = row.relations.length > 0 ? row.relations.join(',') : '-';
    lines.push(
      `entity ${row.id as number} ${row.lifecycle} source=${row.sourceKind} ` +
        `classes=[${row.classes.join(',')}] caps=[${caps}] rels=[${rels}] ` +
        `transformEligible=${row.transformEligible} movementEligible=${row.movementEligible}`,
    );
  }
  if (view.lastResult) {
    const r = view.lastResult;
    lines.push(`lastResult ${r.accepted ? 'accepted' : 'rejected'} ${r.detail} entity=${r.entity as number}`);
  }
  return lines;
}
