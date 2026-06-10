// @asha/script-sdk — the surface a constrained policy author programs against.
//
// A policy is a pure function from a generated, read-only `PolicyView` to an
// array of proposed `PolicyCommand`s. This package adds *ergonomics only* on top
// of @asha/contracts: it never duplicates the protocol shapes (they are
// re-exported from the generated contracts), never mutates state, and never
// reaches for the renderer, UI, bridge, Electron, filesystem, network, clock, or
// ambient randomness. The Rust authority core remains the sole validator.

import type {
  EntityId,
  ModeId,
  ProcessId,
  SignalId,
  SubjectId,
  TagId,
  ScriptView,
  EntityView,
  ProcessView,
  Command,
} from '@asha/contracts';

// Re-export the generated contract surface a policy needs, so authors have a
// single import. Render and replay families are intentionally NOT re-exported:
// a policy has no business with them.
export type {
  EntityId,
  SubjectId,
  ProcessId,
  ModeId,
  SignalId,
  TagId,
  ScriptView,
  EntityView,
  ProcessView,
  Command,
  CommandEnvelope,
  CommandKind,
  EntityCommand,
  SubjectCommand,
  ProcessCommand,
  ModeCommand,
  SignalCommand,
  TagCommand,
  ScriptRejection,
  ScriptOutcome,
} from '@asha/contracts';

export {
  entityId,
  subjectId,
  processId,
  modeId,
  signalId,
  tagId,
} from '@asha/contracts';

// ── Policy-facing aliases ─────────────────────────────────────────────────────

/** The read-only view a policy is handed. Backed by the generated `ScriptView`. */
export type PolicyView = ScriptView;

/** A single proposed command. Backed by the generated `Command` union. */
export type PolicyCommand = Command;

/**
 * A policy: a pure function from a read-only view to proposed commands.
 *
 * The return type is a `readonly` array of `readonly` command values — a policy
 * proposes, it does not own or mutate. It receives no context object on
 * purpose: no clock, no RNG, no I/O. Determinism is a function of the view
 * alone.
 */
export type Policy = (view: PolicyView) => readonly PolicyCommand[];

// ── Command construction helpers ──────────────────────────────────────────────

/**
 * Builders that produce well-formed `PolicyCommand` values without making the
 * author hand-write discriminated-union literals. Each returns a value of the
 * generated `Command` type, so there is no parallel command shape to drift.
 */
export const commands = {
  createEntity: (id: EntityId): PolicyCommand => ({
    domain: 'entity',
    command: { kind: 'create', id },
  }),
  deleteEntity: (id: EntityId): PolicyCommand => ({
    domain: 'entity',
    command: { kind: 'delete', id },
  }),
  addTag: (id: EntityId, tag: TagId): PolicyCommand => ({
    domain: 'entity',
    command: { kind: 'addTag', id, tag },
  }),
  removeTag: (id: EntityId, tag: TagId): PolicyCommand => ({
    domain: 'entity',
    command: { kind: 'removeTag', id, tag },
  }),
  createSubject: (id: SubjectId): PolicyCommand => ({
    domain: 'subject',
    command: { kind: 'create', id },
  }),
  deleteSubject: (id: SubjectId): PolicyCommand => ({
    domain: 'subject',
    command: { kind: 'delete', id },
  }),
  startProcess: (id: ProcessId): PolicyCommand => ({
    domain: 'process',
    command: { kind: 'start', id },
  }),
  setProcessMode: (id: ProcessId, mode: ModeId): PolicyCommand => ({
    domain: 'process',
    command: { kind: 'setMode', id, mode },
  }),
  stopProcess: (id: ProcessId): PolicyCommand => ({
    domain: 'process',
    command: { kind: 'stop', id },
  }),
  defineMode: (id: ModeId): PolicyCommand => ({
    domain: 'mode',
    command: { kind: 'define', id },
  }),
  defineSignal: (id: SignalId): PolicyCommand => ({
    domain: 'signal',
    command: { kind: 'define', id },
  }),
  defineTag: (id: TagId): PolicyCommand => ({
    domain: 'tag',
    command: { kind: 'define', id },
  }),
} as const;

// ── Read-only view queries ────────────────────────────────────────────────────

/**
 * Ergonomic read helpers over a `PolicyView`. These never mutate and never hide
 * the contract border — they are thin lookups returning generated view shapes.
 */
export const query = {
  entity: (view: PolicyView, id: EntityId): EntityView | undefined =>
    view.entities.find((e) => e.id === id),

  hasEntity: (view: PolicyView, id: EntityId): boolean =>
    view.entities.some((e) => e.id === id),

  entityHasTag: (view: PolicyView, id: EntityId, tag: TagId): boolean =>
    view.entities.find((e) => e.id === id)?.tags.includes(tag) ?? false,

  process: (view: PolicyView, id: ProcessId): ProcessView | undefined =>
    view.processes.find((p) => p.id === id),

  processMode: (view: PolicyView, id: ProcessId): ModeId | undefined => {
    const found = view.processes.find((p) => p.id === id);
    return found?.mode ?? undefined;
  },

  hasTagDefined: (view: PolicyView, tag: TagId): boolean =>
    view.tags.includes(tag),
} as const;

// ── Test-harness utilities (policy fixtures only) ─────────────────────────────

/** The view of an empty world. */
export function emptyView(): PolicyView {
  return {
    entities: [],
    subjects: [],
    processes: [],
    modes: [],
    signals: [],
    tags: [],
  };
}

/**
 * Build a `PolicyView` from a partial override, defaulting every absent field
 * to empty. Scoped to policy fixtures and tests — not for production view
 * construction (the script host owns that in Phase 3.3).
 */
export function makeView(parts: Partial<PolicyView> = {}): PolicyView {
  const base = emptyView();
  return {
    entities: parts.entities ?? base.entities,
    subjects: parts.subjects ?? base.subjects,
    processes: parts.processes ?? base.processes,
    modes: parts.modes ?? base.modes,
    signals: parts.signals ?? base.signals,
    tags: parts.tags ?? base.tags,
  };
}

/**
 * Invoke a policy against a view and return its proposed commands. A stable
 * test seam: today it is just application, but it gives fixtures one place to
 * call policies through.
 */
export function runPolicy(policy: Policy, view: PolicyView): readonly PolicyCommand[] {
  return policy(view);
}
