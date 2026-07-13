// @asha/app owns composition and the sole authority-safe command submission path.
import { EditorStore, proposeCommand, previewTargets, } from '@asha/editor-tools';
export { EditorStore } from '@asha/editor-tools';
export * from './shell.js';
export * from './editor-input-composition.js';
/**
 * The real command sink: submit the generated `VoxelCommand`s through the runtime
 * facade's `submitCommands` verb (carrying the `protocol_voxel::CommandBatch`
 * border straight to Rust authority) and forward the classified {@link
 * CommandResult} to `onResult` for UI/diagnostics. The app is the ONLY package
 * permitted to take this transport dependency.
 */
export function bridgeCommandSink(bridge, onResult) {
    return (commands) => {
        const batch = { commands: [...commands] };
        const result = bridge.submitCommands(batch);
        onResult?.(result);
    };
}
/**
 * The single authority-safe edit path. Holds the persistent {@link EditorStore},
 * computes a non-authoritative preview, and — only on {@link commit} — submits the
 * proposed command through the injected {@link CommandSink}. It never mutates voxel
 * state itself.
 */
export class VoxelEditController {
    store;
    #sink;
    constructor(sink, store = new EditorStore()) {
        this.#sink = sink;
        this.store = store;
    }
    /** The cells the current brush would affect — non-authoritative preview data. */
    preview() {
        return previewTargets(this.store.getState());
    }
    /** The command the current context would commit, without submitting it. */
    proposal() {
        return proposeCommand(this.store.getState());
    }
    /**
     * Submit the current proposal through the bridge path (the only mutation route).
     * Returns the submitted command, or `null` if there was nothing to commit (no
     * selection / non-editing tool) — in which case the sink is not called.
     */
    commit() {
        const command = this.proposal();
        if (command === null) {
            return null;
        }
        this.#sink([command]);
        return command;
    }
    /**
     * Cancel the current draft: clear the selection (and therefore the preview)
     * without submitting anything. Symmetric with {@link commit} — the edit lifecycle
     * ends either by committing the proposal or cancelling it. Never calls the sink.
     */
    cancel() {
        this.store.dispatch({ type: 'clearSelection' });
    }
}
/**
 * The single authority-safe generic-entity authoring path. Forwards a proposal
 * (built by `@asha/ui-dom` / `@asha/editor-tools`) through the injected
 * {@link EntityAuthoringSink} and records the classified outcome for the devtools
 * inspector to display. It never mutates entity authority itself.
 */
export class EntityAuthoringController {
    #sink;
    #last = null;
    constructor(sink) {
        this.#sink = sink;
    }
    /**
     * Submit a proposed authoring command for validation. Returns the classified
     * outcome (also retained as {@link lastOutcome}); on rejection authority is
     * unchanged — the controller mutates nothing locally either way.
     */
    submit(command) {
        const outcome = this.#sink(command);
        this.#last = outcome;
        return outcome;
    }
    /** The last authoring outcome, for the inspector's "last command result" readout. */
    lastOutcome() {
        return this.#last;
    }
}
/** The real picker: route the ray through the runtime facade's `pickVoxel` verb. */
export function bridgePicker(bridge) {
    return (ray) => bridge.pickVoxel(ray);
}
/**
 * The single pointer→selection path: cast `ray` against authority (Rust owns the
 * voxel-grid raycast — the renderer only built the ray) and update editor selection
 * through pure actions. A hit selects the struck voxel + face; a classified miss
 * clears the selection. Selection is keyed on authority voxel coordinates, never a
 * render handle, so it survives reprojection. Returns the authority result for UI
 * diagnostics. Never mutates voxel state.
 */
export function pickAndSelect(store, pick, ray) {
    const result = pick(ray);
    if (result.outcome === 'hit') {
        store.dispatch({
            type: 'setSelection',
            selection: { voxel: result.hit.voxel, face: result.hit.face },
        });
    }
    else {
        store.dispatch({ type: 'clearSelection' });
    }
    return result;
}
/**
 * Revalidate a renderer pick hint against the authoritative pick. Authority is the
 * sole source of voxel coordinates — the renderer's claim is never trusted for
 * selection. If authority hit a voxel/face that disagrees with the claim, the hint
 * was stale (a desynced renderer mesh): returns a classified `hitMismatch` rejection
 * so the caller fails closed instead of acting on the wrong cell. A confirmed hit or
 * a plain miss passes the authority result through unchanged.
 */
export function revalidatePickHint(authority, claim) {
    if (authority.outcome !== 'hit') {
        return authority;
    }
    const { voxel, face } = authority.hit;
    const matches = voxel.x === claim.voxel.x &&
        voxel.y === claim.voxel.y &&
        voxel.z === claim.voxel.z &&
        face === claim.face;
    if (matches) {
        return authority;
    }
    return {
        outcome: 'miss',
        rejection: {
            reason: 'hitMismatch',
            authoritativeVoxel: voxel,
            authoritativeFace: face,
            claimedVoxel: claim.voxel,
            claimedFace: claim.face,
        },
    };
}
//# sourceMappingURL=index.js.map