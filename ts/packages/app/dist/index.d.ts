import type { VoxelCommand, VoxelCoord } from '@asha/contracts';
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
//# sourceMappingURL=index.d.ts.map