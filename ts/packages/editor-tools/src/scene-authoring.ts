// @asha/editor-tools — proposal-only scene authoring controls (#2380).
//
// These builders turn authoring intent (add a static-mesh node, add a sprite,
// group/reparent, set an initial transform, edit metadata/tags) into typed
// **proposals** built from generated `@asha/contracts` scene types. They are pure
// and they NEVER submit, validate, or mutate authority — Rust owns scene
// validation and the runtime facade owns submission.
//
// `applyProposalToDraft` produces a UI-local *draft* document for preview and for
// handing to authority validation. The draft is explicitly NOT authority truth: a
// proposal is accepted only when Rust validation passes and authority replays it.
// `summarizeValidation` turns the authoritative `SceneValidationReport` (from Rust)
// into an accept/reject readout — the UI reflects that, it does not decide it.

import type {
  AssetReference,
  FlatSceneDocument,
  SceneNodeId,
  SceneNodeKind,
  SceneLight,
  SceneNodeRecord,
  SceneTransform,
  SceneValidationCode,
  SceneValidationError,
  SceneValidationReport,
} from '@asha/contracts';

const IDENTITY_TRANSFORM: SceneTransform = {
  translation: [0, 0, 0],
  rotation: [0, 0, 0, 1],
  scale: [1, 1, 1],
};

// ── Proposal DTOs ────────────────────────────────────────────────────────────────

/**
 * One proposed scene edit, expressed over generated contract types. A proposal is
 * the thing a UI submits to authority; authority accepts or rejects it. There is
 * no proposal that mutates state directly.
 */
export type SceneEditProposal =
  | { readonly op: 'addNode'; readonly node: SceneNodeRecord }
  | { readonly op: 'reparent'; readonly node: SceneNodeId; readonly newParent: SceneNodeId | null; readonly childOrder: number }
  | { readonly op: 'setTransform'; readonly node: SceneNodeId; readonly transform: SceneTransform }
  | { readonly op: 'setLight'; readonly node: SceneNodeId; readonly sceneLight: SceneLight }
  | { readonly op: 'setMetadata'; readonly node: SceneNodeId; readonly label: string | null; readonly tags: readonly string[] };

/** Optional authored fields shared by the add-node builders. */
export interface NodeAuthoring {
  readonly parent?: SceneNodeId | null;
  readonly childOrder?: number;
  readonly label?: string | null;
  readonly tags?: readonly string[];
  readonly transform?: SceneTransform;
}

function newRecord(id: SceneNodeId, kind: SceneNodeKind, authoring: NodeAuthoring): SceneNodeRecord {
  return {
    id,
    parent: authoring.parent ?? null,
    childOrder: authoring.childOrder ?? 0,
    label: authoring.label ?? null,
    tags: authoring.tags ?? [],
    transform: authoring.transform ?? IDENTITY_TRANSFORM,
    kind,
  };
}

/** Propose adding a static-mesh node bound to a catalog mesh asset. */
export function proposeAddStaticMesh(id: SceneNodeId, asset: AssetReference, authoring: NodeAuthoring = {}): SceneEditProposal {
  return { op: 'addNode', node: newRecord(id, { kind: 'staticMesh', asset }, authoring) };
}

/** Propose adding a sprite node bound to a catalog sprite asset. */
export function proposeAddSprite(id: SceneNodeId, asset: AssetReference, authoring: NodeAuthoring = {}): SceneEditProposal {
  return { op: 'addNode', node: newRecord(id, { kind: 'sprite', asset }, authoring) };
}

/** Propose adding an empty grouping/transform node. */
export function proposeAddGroup(id: SceneNodeId, authoring: NodeAuthoring = {}): SceneEditProposal {
  return { op: 'addNode', node: newRecord(id, { kind: 'emptyGroup' }, authoring) };
}

/** Propose adding a stored renderer-neutral light node. */
export function proposeAddLight(id: SceneNodeId, sceneLight: SceneLight, authoring: NodeAuthoring = {}): SceneEditProposal {
  return { op: 'addNode', node: newRecord(id, { kind: 'light', sceneLight }, authoring) };
}

/** Propose replacing the authored light properties without changing its pose. */
export function proposeSetLight(node: SceneNodeId, sceneLight: SceneLight): SceneEditProposal {
  return { op: 'setLight', node, sceneLight };
}

/** Propose reparenting (or grouping) a node under a new parent at a sibling index. */
export function proposeReparent(node: SceneNodeId, newParent: SceneNodeId | null, childOrder = 0): SceneEditProposal {
  return { op: 'reparent', node, newParent, childOrder };
}

/** Propose replacing a node's initial transform. */
export function proposeSetTransform(node: SceneNodeId, transform: SceneTransform): SceneEditProposal {
  return { op: 'setTransform', node, transform };
}

/** Propose replacing a node's debug label and tags (never authority semantics). */
export function proposeSetMetadata(node: SceneNodeId, label: string | null, tags: readonly string[] = []): SceneEditProposal {
  return { op: 'setMetadata', node, label, tags };
}

// ── Draft application (UI-local preview, not authority) ───────────────────────────

/**
 * Apply a proposal to a copy of `doc`, producing a UI-local **draft** for preview
 * and for handing to authority validation. Pure — returns a new document and never
 * mutates the input. The draft is not authority truth; only a validated, replayed
 * proposal becomes truth.
 *
 * Returns the unchanged document (structurally copied) when the proposal targets a
 * node that is absent — authority will reject it; the draft never invents the node.
 */
export function applyProposalToDraft(doc: FlatSceneDocument, proposal: SceneEditProposal): FlatSceneDocument {
  const nodes = doc.nodes.map((n) => ({ ...n }));
  const indexOf = (id: SceneNodeId): number => nodes.findIndex((n) => (n.id as number) === (id as number));

  switch (proposal.op) {
    case 'addNode': {
      // A draft does not dedupe ids — a duplicate is a validation concern, surfaced
      // by authority rather than silently swallowed here.
      nodes.push({ ...proposal.node });
      break;
    }
    case 'reparent': {
      const at = indexOf(proposal.node);
      if (at >= 0) {
        nodes[at] = { ...nodes[at]!, parent: proposal.newParent, childOrder: proposal.childOrder };
      }
      break;
    }
    case 'setTransform': {
      const at = indexOf(proposal.node);
      if (at >= 0) {
        nodes[at] = { ...nodes[at]!, transform: proposal.transform };
      }
      break;
    }
    case 'setLight': {
      const at = indexOf(proposal.node);
      if (at >= 0 && nodes[at]!.kind.kind === 'light') {
        nodes[at] = { ...nodes[at]!, kind: { kind: 'light', sceneLight: proposal.sceneLight } };
      }
      break;
    }
    case 'setMetadata': {
      const at = indexOf(proposal.node);
      if (at >= 0) {
        nodes[at] = { ...nodes[at]!, label: proposal.label, tags: [...proposal.tags] };
      }
      break;
    }
  }

  return { ...doc, nodes };
}

// ── Validation feedback read model ────────────────────────────────────────────────

/** One classified validation issue, lifted from the authoritative Rust report. */
export interface ProposalIssue {
  readonly code: SceneValidationCode;
  readonly node: SceneNodeId | null;
  readonly detail: string;
}

/** Whether authority accepted the proposal, plus any classified rejection reasons. */
export interface ProposalFeedback {
  readonly accepted: boolean;
  readonly issues: readonly ProposalIssue[];
}

function describeValidationError(error: SceneValidationError): string {
  switch (error.code) {
    case 'duplicate-node-id':
      return `node ${error.node as number} duplicates an existing id`;
    case 'unknown-parent':
      return `node ${error.node as number} names absent parent ${error.parent as number}`;
    case 'cycle':
      return `parent cycle: ${error.cyclePath.map((id) => id as number).join(' → ')}`;
    case 'invalid-transform':
      return `node ${error.node as number} has an invalid transform${error.transformReason ? ` (${error.transformReason})` : ''}`;
    case 'invalid-voxel-volume-transform':
      return `node ${error.node as number} has an unsupported voxel transform${error.detailReason ? ` (${error.detailReason})` : ''}`;
    case 'asset-kind-mismatch':
      return `node ${error.node as number} expected ${error.expectedKind}, found ${error.actualKind}`;
    case 'invalid-light':
      return `node ${error.node as number} has an invalid light${error.lightReason ? ` (${error.lightReason})` : ''}`;
    case 'duplicate-marker-id':
      return `node ${error.node as number} duplicates marker ${error.instanceId}`;
    case 'invalid-marker':
      return `node ${error.node as number} has an invalid marker${error.detailReason ? ` (${error.detailReason})` : ''}`;
    case 'duplicate-entity-instance-id':
      return `node ${error.node as number} duplicates entity instance ${error.instanceId}`;
    case 'invalid-entity-instance':
      return `node ${error.node as number} has an invalid entity instance${error.detailReason ? ` (${error.detailReason})` : ''}`;
    case 'duplicate-bootstrap-node':
      return `node ${error.node as number} duplicates the scene bootstrap node`;
    case 'invalid-bootstrap':
      return `node ${error.node as number} has invalid bootstrap bindings${error.detailReason ? ` (${error.detailReason})` : ''}`;
    case 'duplicate-catalog-binding':
      return `node ${error.node as number} duplicates catalog binding ${error.bindingId}`;
  }
}

/**
 * Turn an authoritative `SceneValidationReport` (produced by Rust validation of a
 * proposal's draft) into an accept/reject readout the UI can display. The UI never
 * decides acceptance — it reflects this.
 */
export function summarizeValidation(report: SceneValidationReport): ProposalFeedback {
  const issues: ProposalIssue[] = report.errors.map((error) => ({
    code: error.code,
    node: error.node,
    detail: describeValidationError(error),
  }));
  return { accepted: issues.length === 0, issues };
}
