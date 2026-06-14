// @asha/script-sdk — the surface a constrained policy author programs against.
//
// A policy is a pure function from a generated, read-only `PolicyView` to an
// array of proposed `PolicyCommand`s. This package adds *ergonomics only* on top
// of @asha/contracts: it never duplicates the protocol shapes (they are
// re-exported from the generated contracts), never mutates state, and never
// reaches for the renderer, UI, bridge, Electron, filesystem, network, clock, or
// ambient randomness. The Rust authority core remains the sole validator.
// Read-only world-layer view a policy is handed (#2391).
export * from './world-view.js';
// Deterministic execution envelope (#2393): the only source of time/random/tick.
export * from './env.js';
export { entityId, subjectId, processId, modeId, signalId, tagId, } from '@asha/contracts';
// ── Command construction helpers ──────────────────────────────────────────────
/**
 * Builders that produce well-formed `PolicyCommand` values without making the
 * author hand-write discriminated-union literals. Each returns a value of the
 * generated `Command` type, so there is no parallel command shape to drift.
 */
export const commands = {
    createEntity: (id) => ({
        domain: 'entity',
        command: { kind: 'create', id },
    }),
    deleteEntity: (id) => ({
        domain: 'entity',
        command: { kind: 'delete', id },
    }),
    addTag: (id, tag) => ({
        domain: 'entity',
        command: { kind: 'addTag', id, tag },
    }),
    removeTag: (id, tag) => ({
        domain: 'entity',
        command: { kind: 'removeTag', id, tag },
    }),
    createSubject: (id) => ({
        domain: 'subject',
        command: { kind: 'create', id },
    }),
    deleteSubject: (id) => ({
        domain: 'subject',
        command: { kind: 'delete', id },
    }),
    startProcess: (id) => ({
        domain: 'process',
        command: { kind: 'start', id },
    }),
    setProcessMode: (id, mode) => ({
        domain: 'process',
        command: { kind: 'setMode', id, mode },
    }),
    stopProcess: (id) => ({
        domain: 'process',
        command: { kind: 'stop', id },
    }),
    defineMode: (id) => ({
        domain: 'mode',
        command: { kind: 'define', id },
    }),
    defineSignal: (id) => ({
        domain: 'signal',
        command: { kind: 'define', id },
    }),
    defineTag: (id) => ({
        domain: 'tag',
        command: { kind: 'define', id },
    }),
};
// ── Read-only view queries ────────────────────────────────────────────────────
/**
 * Ergonomic read helpers over a `PolicyView`. These never mutate and never hide
 * the contract border — they are thin lookups returning generated view shapes.
 */
export const query = {
    entity: (view, id) => view.entities.find((e) => e.id === id),
    hasEntity: (view, id) => view.entities.some((e) => e.id === id),
    entityHasTag: (view, id, tag) => view.entities.find((e) => e.id === id)?.tags.includes(tag) ?? false,
    process: (view, id) => view.processes.find((p) => p.id === id),
    processMode: (view, id) => {
        const found = view.processes.find((p) => p.id === id);
        return found?.mode ?? undefined;
    },
    hasTagDefined: (view, tag) => view.tags.includes(tag),
};
// ── Test-harness utilities (policy fixtures only) ─────────────────────────────
/** The view of an empty world. */
export function emptyView() {
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
export function makeView(parts = {}) {
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
export function runPolicy(policy, view) {
    return policy(view);
}
//# sourceMappingURL=index.js.map