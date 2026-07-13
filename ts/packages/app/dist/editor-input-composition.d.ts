import type { VoxelCommand } from '@asha/contracts';
import { type EditorResolvedInputFrame } from '@asha/editor-tools';
import { BrowserInputHost, type BrowserInputSessionPort } from '@asha/runtime-bridge';
export interface EditorCameraInputPort {
    apply(frame: EditorResolvedInputFrame): void;
}
export interface EditorToolInputPort {
    commit(): VoxelCommand | null;
    cancel(): void;
}
export interface AppEditorInputCompositionOptions {
    readonly session: BrowserInputSessionPort;
    readonly editor: EditorToolInputPort;
    readonly camera: EditorCameraInputPort;
}
export interface AppEditorInputDrainReceipt {
    readonly frame: EditorResolvedInputFrame;
    readonly committed: VoxelCommand | null;
    readonly cancelled: boolean;
}
/**
 * Production editor input composition. The browser host owns DOM normalization,
 * Session owns action resolution/context consumption, editor-tools owns the
 * expression accumulator, and app alone applies drained camera/tool intent.
 */
export declare class AppEditorInputComposition {
    #private;
    readonly host: BrowserInputHost;
    constructor(options: AppEditorInputCompositionOptions);
    drain(): AppEditorInputDrainReceipt;
    reset(): void;
}
//# sourceMappingURL=editor-input-composition.d.ts.map