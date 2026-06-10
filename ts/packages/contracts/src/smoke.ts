// Import/typecheck smoke for @asha/contracts.
//
// This is the proof for the Phase 2 exit criterion "a TypeScript package can
// import generated branded IDs and command unions" (see
// governance/protocol-border-consumers.md). It is NOT part of the public API
// (index.ts does not re-export it). Its only job is to fail `tsc` if the
// generated contracts stop being importable or usable — proving that branded
// IDs and the command/view/diff/replay unions compile when consumed exactly as
// a downstream package would consume them, with no policy, renderer, UI,
// bridge, Electron, or browser globals in scope.
//
// It is value-level on purpose: constructing real union values exercises the
// discriminants and field shapes, not just the type names.

import {
  entityId,
  modeId,
  tagId,
  renderHandle,
  stepIndex,
  replayHash,
  REPLAY_FORMAT_VERSION,
  type EntityId,
  type Command,
  type CommandEnvelope,
  type ScriptView,
  type ScriptOutcome,
  type RenderDiff,
  type ReplayRecord,
} from './index.js';

// Branded IDs are nominally typed and built through their constructors.
const entity: EntityId = entityId(1);

// A command authored the way a policy would author it.
const addTag: Command = {
  domain: 'entity',
  command: { kind: 'addTag', id: entity, tag: tagId(2) },
};

const envelope: CommandEnvelope = { kind: 'policy', command: addTag };

// A read-only view value.
const view: ScriptView = {
  entities: [{ id: entity, tags: [tagId(2)] }],
  subjects: [],
  processes: [],
  modes: [modeId(3)],
  signals: [],
  tags: [tagId(2)],
};

const outcome: ScriptOutcome = { status: 'accepted' };

// A retained-mode render diff value.
const diff: RenderDiff = { op: 'destroy', handle: renderHandle(5) };

// A replay record value, with the format version sourced from the contract.
const record: ReplayRecord = {
  formatVersion: REPLAY_FORMAT_VERSION,
  initialHash: replayHash(0),
  steps: [
    {
      index: stepIndex(0),
      command: envelope,
      outcome: { status: 'accepted', events: [{ event: 'entityCreated', id: entity }] },
      postHash: replayHash(1),
    },
  ],
  snapshots: [],
};

// Exported so the values are "used" (lint-clean) and tree-shakeable. Consumers
// of @asha/contracts never see this — it is not re-exported by index.ts.
export const __contractSmoke = {
  entity,
  addTag,
  envelope,
  view,
  outcome,
  diff,
  record,
} as const;
