import type { EntityId, ModeId, ProcessId, SignalId, SubjectId, TagId, ScriptView, EntityView, ProcessView, Command } from '@asha/contracts';
export * from './world-view.js';
export * from './env.js';
export type { EntityId, SubjectId, ProcessId, ModeId, SignalId, TagId, ScriptView, EntityView, ProcessView, Command, CommandEnvelope, CommandKind, EntityCommand, SubjectCommand, ProcessCommand, ModeCommand, SignalCommand, TagCommand, ScriptRejection, ScriptOutcome, } from '@asha/contracts';
export { entityId, subjectId, processId, modeId, signalId, tagId, } from '@asha/contracts';
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
/**
 * Builders that produce well-formed `PolicyCommand` values without making the
 * author hand-write discriminated-union literals. Each returns a value of the
 * generated `Command` type, so there is no parallel command shape to drift.
 */
export declare const commands: {
    readonly createEntity: (id: EntityId) => PolicyCommand;
    readonly deleteEntity: (id: EntityId) => PolicyCommand;
    readonly addTag: (id: EntityId, tag: TagId) => PolicyCommand;
    readonly removeTag: (id: EntityId, tag: TagId) => PolicyCommand;
    readonly createSubject: (id: SubjectId) => PolicyCommand;
    readonly deleteSubject: (id: SubjectId) => PolicyCommand;
    readonly startProcess: (id: ProcessId) => PolicyCommand;
    readonly setProcessMode: (id: ProcessId, mode: ModeId) => PolicyCommand;
    readonly stopProcess: (id: ProcessId) => PolicyCommand;
    readonly defineMode: (id: ModeId) => PolicyCommand;
    readonly defineSignal: (id: SignalId) => PolicyCommand;
    readonly defineTag: (id: TagId) => PolicyCommand;
};
/**
 * Ergonomic read helpers over a `PolicyView`. These never mutate and never hide
 * the contract border — they are thin lookups returning generated view shapes.
 */
export declare const query: {
    readonly entity: (view: PolicyView, id: EntityId) => EntityView | undefined;
    readonly hasEntity: (view: PolicyView, id: EntityId) => boolean;
    readonly entityHasTag: (view: PolicyView, id: EntityId, tag: TagId) => boolean;
    readonly process: (view: PolicyView, id: ProcessId) => ProcessView | undefined;
    readonly processMode: (view: PolicyView, id: ProcessId) => ModeId | undefined;
    readonly hasTagDefined: (view: PolicyView, tag: TagId) => boolean;
};
/** The view of an empty world. */
export declare function emptyView(): PolicyView;
/**
 * Build a `PolicyView` from a partial override, defaulting every absent field
 * to empty. Scoped to policy fixtures and tests — not for production view
 * construction (the script host owns that in Phase 3.3).
 */
export declare function makeView(parts?: Partial<PolicyView>): PolicyView;
/**
 * Invoke a policy against a view and return its proposed commands. A stable
 * test seam: today it is just application, but it gives fixtures one place to
 * call policies through.
 */
export declare function runPolicy(policy: Policy, view: PolicyView): readonly PolicyCommand[];
//# sourceMappingURL=index.d.ts.map