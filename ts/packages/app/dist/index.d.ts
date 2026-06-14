import type { PickRay, PickResult, VoxelCommand, VoxelCoord } from '@asha/contracts';
import { EditorStore } from '@asha/editor-tools';
import type { CommandResult, RuntimeBridge } from '@asha/runtime-bridge';
export { EditorStore } from '@asha/editor-tools';
export type { EditorContext, EditorAction, VoxelSelection } from '@asha/editor-tools';
/**
 * Where committed commands go. The real wiring is {@link bridgeCommandSink}, which
 * sends the batch through `@asha/runtime-bridge` (`submitCommands`) to Rust for
 * validation + application. Injected so the editor controller stays decoupled from
 * the transport and is trivially testable.
 */
export type CommandSink = (commands: readonly VoxelCommand[]) => void;
/** Observes the classified accept/reject summary authority returns for a batch. */
export type CommandResultHandler = (result: CommandResult) => void;
/**
 * The real command sink: submit the generated `VoxelCommand`s through the runtime
 * facade's `submitCommands` verb (carrying the `protocol_voxel::CommandBatch`
 * border straight to Rust authority) and forward the classified {@link
 * CommandResult} to `onResult` for UI/diagnostics. The app is the ONLY package
 * permitted to take this transport dependency.
 */
export declare function bridgeCommandSink(bridge: RuntimeBridge, onResult?: CommandResultHandler): CommandSink;
/**
 * The single authority-safe edit path. Holds the persistent {@link EditorStore},
 * computes a non-authoritative preview, and — only on {@link commit} — submits the
 * proposed command through the injected {@link CommandSink}. It never mutates voxel
 * state itself.
 */
export declare class VoxelEditController {
    #private;
    readonly store: EditorStore;
    constructor(sink: CommandSink, store?: EditorStore);
    /** The cells the current brush would affect — non-authoritative preview data. */
    preview(): VoxelCoord[];
    /** The command the current context would commit, without submitting it. */
    proposal(): VoxelCommand | null;
    /**
     * Submit the current proposal through the bridge path (the only mutation route).
     * Returns the submitted command, or `null` if there was nothing to commit (no
     * selection / non-editing tool) — in which case the sink is not called.
     */
    commit(): VoxelCommand | null;
}
/**
 * Casts a world-space {@link PickRay} against authority and returns the classified
 * {@link PickResult}. The real implementation is {@link bridgePicker} (backed by
 * `@asha/runtime-bridge` `pickVoxel`); injected so the selection flow stays
 * decoupled from the transport and trivially testable.
 */
export type VoxelPicker = (ray: PickRay) => PickResult;
/** The real picker: route the ray through the runtime facade's `pickVoxel` verb. */
export declare function bridgePicker(bridge: RuntimeBridge): VoxelPicker;
/**
 * The single pointer→selection path: cast `ray` against authority (Rust owns the
 * voxel-grid raycast — the renderer only built the ray) and update editor selection
 * through pure actions. A hit selects the struck voxel + face; a classified miss
 * clears the selection. Selection is keyed on authority voxel coordinates, never a
 * render handle, so it survives reprojection. Returns the authority result for UI
 * diagnostics. Never mutates voxel state.
 */
export declare function pickAndSelect(store: EditorStore, pick: VoxelPicker, ray: PickRay): PickResult;
//# sourceMappingURL=index.d.ts.map