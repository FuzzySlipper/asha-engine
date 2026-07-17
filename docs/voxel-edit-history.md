---
status: current
audience: agent
tags: [voxel, edit, history]
supersedes: []
see-also: []
---

# Voxel Edit History, Undo, And Revert

Status: implemented as Rust-owned timeline, ProjectBundle persistence, generated
protocol, and RuntimeBridge read/revert/undo/redo surfaces. Studio remains a
projection and intent client; it does not own the undo stack.

## Decision

Voxel edit history belongs to Rust authority and durable ProjectBundle data.
Studio may keep transient UI state such as selection, hover, pending brush
parameters, and local panel expansion, but it must not own the authoritative undo
stack or rebuild voxel state from a TypeScript-only command log.

The long-term shape is:

- Runtime authority: a Rust-owned loaded voxel edit timeline with a cursor.
- Stored ProjectBundle data: a durable voxel edit history/timeline artifact
  associated with a voxel model or voxel-volume asset, plus hashes tying it to
  the base voxel data and material catalog used to interpret it.
- UI projection: Studio displays history entries, diff summaries, and preview
  readouts returned by Rust. UI buttons request typed undo/redo/revert intents.

## What History Stores

History records accepted authority transactions only. Preview attempts and
rejected receipts may be diagnostic evidence, but they are not durable authoring
history unless a future audit lane explicitly stores them.

Each accepted history entry should include:

- stable transaction id and parent transaction id or previous cursor id;
- operation label/provenance for display;
- transaction receipt hash;
- `before_hash`, `after_hash`, and command/event counts;
- accepted `VoxelEditEvent` log or a compacted equivalent;
- command hash and material catalog hash;
- touched bounds and touched-voxel count;
- optional diff summary for quick UI projection;
- optional checkpoint reference when compaction creates a replay boundary.

The event log remains the first durable source because it already round-trips
through `rule_voxel_edit::persist` and ProjectBundle durability. Snapshots and
checkpoints are acceleration/compaction artifacts, not alternate authority.

## Minimum Viable Feature

The first engine feature should be revert-to-receipt plus bounded diff preview.
Undo/redo should be implemented as a convenience over the same cursor/revert
surface, not as a separate stack with different semantics.

Minimum verbs:

- preview revert to a transaction id or cursor index;
- apply revert to a transaction id or cursor index;
- undo one accepted transaction;
- redo one unapplied transaction when the cursor has not forked;
- read bounded history summaries and diff readouts.

Revert should be implemented by Rust replaying from the accepted base/checkpoint
to the requested cursor and returning a typed receipt. That is slower than an
inverse-patch shortcut, but it is deterministic, easy to verify, and does not
require TypeScript to reason about inverse voxel math. Later versions may add
Rust-generated inverse patches for large histories, but replay remains the
correctness reference.

## Save, Load, And Reopen

ProjectBundle save should persist the applied history cursor and any retained
redo tail explicitly. Reopen should reconstruct the same current voxel state and
the same history cursor from stored data.

Loading must fail closed when:

- the base voxel data hash does not match the history artifact;
- a retained event log cannot replay to the recorded checkpoint hash;
- the material catalog hash required by the history is missing or changed;
- a history entry references a transaction parent that is absent;
- quotas for history entries, replay steps, touched voxels, or checkpoints are
  exceeded.

Compaction may collapse old history into a checkpoint, but it must report what
undo depth remains. A compacted-away transaction may remain as audit metadata,
but Studio must not present it as undoable unless Rust can actually replay or
revert to that cursor.

## Diff Preview

Diff previews are Rust-owned readouts. The first version should expose bounded
summaries:

- changed voxel count;
- material before/after counts;
- touched bounds;
- transaction ids included in the diff;
- projected before/current/target hashes;
- optional sample windows using the bounded voxel window query surface.

Large diffs must fail closed or summarize under explicit quotas. They should not
return unbounded cell arrays to UI packages.

Studio should display diff previews as summaries, not as a local copy of the
changed cells. When Rust marks a summary `partial`, the changed voxel count and
material deltas are quota-bounded evidence rather than a complete diff; the UI
should label the preview as partial, show the typed diagnostic, and avoid
offering per-cell actions that require cells Rust did not return. When a
`sample_window_ref` is present, Studio may issue the bounded voxel window query
for that window and render those samples as inspection evidence. It must not
expand the ref into an unbounded cell dump or infer hidden changed cells from
renderer state.

## Bulk Edits

Bulk transactions from `rule_voxel_edit::execute_transaction` should append one
history entry per accepted transaction. Large generated model-building operations
should not explode into UI-owned per-click history. They may contain many
commands and events internally, but the history cursor should treat the accepted
transaction as one user/agent-authored operation unless Rust returns a grouped
sub-history projection.

Replay and diff budgets are separate from transaction validation budgets. A
transaction can be valid to apply but still too expensive to diff in full; in that
case Rust should return a bounded summary and a classified partial-diff reason.

## Material Changes

Voxel history records material ids, not renderer materials. Reopen/revert must
validate the material catalog hash or an explicit Rust migration receipt. If a
material id disappeared or changed semantics, history replay fails closed until a
material migration operation maps the old ids to accepted new ids.

Material-only catalog edits should live in catalog history, not voxel occupancy
history. A future unified authoring timeline may display both, but each authority
lane keeps its own validation.

## Annotation Changes

Voxel annotation layers should have their own Rust-owned edit receipts and
history because annotation membership is not voxel occupancy. Once annotation
surfaces land, ASHA can add an authoring transaction envelope that commits voxel
edits and annotation edits atomically across both lanes.

Until that cross-surface envelope exists:

- voxel undo/revert changes voxel occupancy/material state only;
- annotation undo/revert changes annotation layers only;
- UI must not imply a single undo action rolls back both unless Rust returned a
  cross-surface receipt proving it did.

## Non-Goals

- No VoxelForge history compatibility promise.
- No TypeScript-owned authoritative undo stack.
- No hidden browser local-storage timeline.
- No unbounded diff payloads.
- No promise that compacted-away audit entries remain undoable.
