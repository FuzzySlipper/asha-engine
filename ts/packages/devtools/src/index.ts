// @asha/devtools — observational voxel diagnostics read model (voxel-capability-15).
//
// Tool-only, **observational**: it formats projected diagnostics + editor context
// for inspectors and overlays; it never mutates authority and exposes no command
// path. Imports `@asha/contracts` + `@asha/editor-tools` (tool-only); enforced by
// the dependency graph.

import type { EditorContext } from '@asha/editor-tools';
import { previewTargets } from '@asha/editor-tools';

// Scene/world outliner + inspector read models (#2377).
export * from './scene-outliner.js';

// Asset catalog, lock-drift, and material inspector read models (#2378).
export * from './asset-inspector.js';

// World-bundle save/load and diagnostics panel read models (#2379).
export * from './bundle-panel.js';

// Generic entity authoring inspector read model (#2485).
export * from './entity-inspector.js';

/** A plain mirror of the Rust `voxel-diagnostics` scene report (carried over the
 *  bridge as projected data — devtools never reads authority directly). */
export interface SceneReportSummary {
  readonly resident: number;
  readonly pending: number;
  readonly unloaded: number;
  readonly colliderChunks: number;
  readonly dirtyChunks: number;
  readonly queue: ReadonlyArray<{ readonly kind: string; readonly count: number }>;
}

/** Deterministic display lines for a scene report (pure formatter). */
export function summarizeScene(report: SceneReportSummary): string[] {
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

/** Observational inspector view of the editor tool context (no hidden state). */
export interface EditorInspection {
  readonly tool: EditorContext['tool'];
  readonly brushShape: EditorContext['brushShape'];
  readonly material: number;
  readonly selectedVoxel: readonly [number, number, number] | null;
  readonly selectedFace: string | null;
  readonly affectedCells: number;
}

/** A pure read of the editor context for devtools display. */
export function inspectEditor(ctx: EditorContext): EditorInspection {
  return {
    tool: ctx.tool,
    brushShape: ctx.brushShape,
    material: ctx.material,
    selectedVoxel: ctx.selection ? [ctx.selection.voxel.x, ctx.selection.voxel.y, ctx.selection.voxel.z] : null,
    selectedFace: ctx.selection?.face ?? null,
    affectedCells: previewTargets(ctx).length,
  };
}
