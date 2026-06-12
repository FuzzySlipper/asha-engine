import type { EditorContext } from '@asha/editor-tools';
/** A plain mirror of the Rust `voxel-diagnostics` scene report (carried over the
 *  bridge as projected data — devtools never reads authority directly). */
export interface SceneReportSummary {
    readonly resident: number;
    readonly pending: number;
    readonly unloaded: number;
    readonly colliderChunks: number;
    readonly dirtyChunks: number;
    readonly queue: ReadonlyArray<{
        readonly kind: string;
        readonly count: number;
    }>;
}
/** Deterministic display lines for a scene report (pure formatter). */
export declare function summarizeScene(report: SceneReportSummary): string[];
/** Observational inspector view of the editor tool context (no hidden state). */
export interface EditorInspection {
    readonly tool: EditorContext['tool'];
    readonly selectedVoxel: readonly [number, number, number] | null;
    readonly selectedFace: string | null;
    readonly affectedCells: number;
}
/** A pure read of the editor context for devtools display. */
export declare function inspectEditor(ctx: EditorContext): EditorInspection;
//# sourceMappingURL=index.d.ts.map