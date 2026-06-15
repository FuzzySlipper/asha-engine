// @asha/devtools — observational voxel diagnostics read model (voxel-capability-15).
//
// Tool-only, **observational**: it formats projected diagnostics + editor context
// for inspectors and overlays; it never mutates authority and exposes no command
// path. Imports `@asha/contracts` + `@asha/editor-tools` (tool-only); enforced by
// the dependency graph.
import { previewTargets } from '@asha/editor-tools';
// Scene/world outliner + inspector read models (#2377).
export * from './scene-outliner.js';
// Asset catalog, lock-drift, and material inspector read models (#2378).
export * from './asset-inspector.js';
// World-bundle save/load and diagnostics panel read models (#2379).
export * from './bundle-panel.js';
// Generic entity authoring inspector read model (#2485).
export * from './entity-inspector.js';
/** Deterministic display lines for a scene report (pure formatter). */
export function summarizeScene(report) {
    const lines = [
        `chunks resident=${report.resident} pending=${report.pending} unloaded=${report.unloaded}`,
        `colliders=${report.colliderChunks} dirty=${report.dirtyChunks}`,
    ];
    for (const q of report.queue) {
        if (q.count > 0) {
            lines.push(`queue ${q.kind}=${q.count}`);
        }
    }
    return lines;
}
/** A pure read of the editor context for devtools display. */
export function inspectEditor(ctx) {
    return {
        tool: ctx.tool,
        brushShape: ctx.brushShape,
        material: ctx.material,
        selectedVoxel: ctx.selection ? [ctx.selection.voxel.x, ctx.selection.voxel.y, ctx.selection.voxel.z] : null,
        selectedFace: ctx.selection?.face ?? null,
        affectedCells: previewTargets(ctx).length,
    };
}
//# sourceMappingURL=index.js.map