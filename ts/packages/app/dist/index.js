// @asha/app — composition + the authority-safe command submission path (ADR 0008).
//
// `app` is the ONLY package that submits commands: it turns an `@asha/editor-tools`
// proposal into a submission through the approved bridge path. UI/editor packages
// produce proposals and preview targets but never mutate authoritative state.
import { EditorStore, proposeCommand, previewTargets } from '@asha/editor-tools';
export { EditorStore } from '@asha/editor-tools';
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
//# sourceMappingURL=index.js.map