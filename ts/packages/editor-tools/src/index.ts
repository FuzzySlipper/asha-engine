// @asha/editor-tools — the persistent editor tool context (ADR 0008).
//
// The third state category: not Rust authority, not throwaway DOM state. A small,
// dependency-free observable store of *what the user is about to do* (tool, brush,
// material, selection, preview), plus pure functions that turn that context into
// generated `@asha/contracts` `VoxelCommand` proposals and brush-preview targets.
//
// It imports `@asha/contracts` ONLY — no DOM, `three`, policy, bridge, or renderer.
// It produces proposals; it never submits them and never mutates authority (the
// `app` command-submission path does that). See docs/voxel-ui-architecture.md.

import type { VoxelCommand, VoxelCoord, Face, VoxelValue } from '@asha/contracts';

// Proposal-only scene authoring controls with Rust validation feedback (#2380).
export * from './scene-authoring.js';

// Canonical scene-object hierarchy snapshot/proposal helpers over FlatSceneDocument.
export * from './scene-object-hierarchy.js';

// Proposal-only generic entity authoring controls with Rust validation feedback (#2485).
export * from './entity-authoring.js';

// ── Editor tool context ────────────────────────────────────────────────────────

// Authoring tools: `place` adds a voxel across the struck face; `remove` clears the
// struck voxel; `paint` recolours the struck voxel with the current material (same
// SetVoxel command, different anchor — not a new command variant). `select`/`inspect`
// author nothing.
export type ToolMode = 'place' | 'remove' | 'paint' | 'select' | 'inspect';
export type SelectionMode = 'voxel' | 'face';

/** Brush shape: a single cell, or an axis-aligned box of side `brushSize` (fill). */
export type BrushShape = 'single' | 'box';

/** The editing tools that author voxels (have a preview + a proposal). */
export const EDITING_TOOLS: readonly ToolMode[] = ['place', 'remove', 'paint'];

function isEditingTool(tool: ToolMode): boolean {
  return EDITING_TOOLS.includes(tool);
}

/** A picked anchor (from collision/picking) — a coord + the struck face. */
export interface VoxelSelection {
  readonly voxel: VoxelCoord;
  readonly face: Face;
}

/** The full persistent editor context. Immutable snapshot; updated via actions. */
export interface EditorContext {
  /** Which voxel grid edits target (GridId raw). */
  readonly grid: number;
  readonly tool: ToolMode;
  /** Whether an edit affects a single cell or a box (explicit fill mode). */
  readonly brushShape: BrushShape;
  /** Box side length in voxels (>= 1); used only when `brushShape === 'box'`. */
  readonly brushSize: number;
  /** Material id used by the `place`/`paint` tools (VoxelMaterialId raw). */
  readonly material: number;
  readonly snapping: boolean;
  readonly selectionMode: SelectionMode;
  readonly preview: { readonly enabled: boolean };
  /** Current picked anchor, or null when nothing is selected. */
  readonly selection: VoxelSelection | null;
}

/** The initial editor context. */
export function initialEditorContext(grid = 0): EditorContext {
  return {
    grid,
    tool: 'place',
    brushShape: 'single',
    brushSize: 1,
    material: 1,
    snapping: true,
    selectionMode: 'voxel',
    preview: { enabled: true },
    selection: null,
  };
}

// ── Actions (pure reducer) ─────────────────────────────────────────────────────

export type EditorAction =
  | { readonly type: 'setTool'; readonly tool: ToolMode }
  | { readonly type: 'setBrushShape'; readonly shape: BrushShape }
  | { readonly type: 'setBrushSize'; readonly size: number }
  | { readonly type: 'setMaterial'; readonly material: number }
  | { readonly type: 'setSnapping'; readonly snapping: boolean }
  | { readonly type: 'setSelectionMode'; readonly mode: SelectionMode }
  | { readonly type: 'setPreviewEnabled'; readonly enabled: boolean }
  | { readonly type: 'setSelection'; readonly selection: VoxelSelection }
  | { readonly type: 'clearSelection' };

/** Pure reducer. Validates/normalises (e.g. brush size clamped to `>= 1` integer). */
export function reduce(state: EditorContext, action: EditorAction): EditorContext {
  switch (action.type) {
    case 'setTool':
      return { ...state, tool: action.tool };
    case 'setBrushShape':
      return { ...state, brushShape: action.shape };
    case 'setBrushSize':
      return { ...state, brushSize: Math.max(1, Math.floor(action.size)) };
    case 'setMaterial':
      return { ...state, material: Math.max(0, Math.floor(action.material)) };
    case 'setSnapping':
      return { ...state, snapping: action.snapping };
    case 'setSelectionMode':
      return { ...state, selectionMode: action.mode };
    case 'setPreviewEnabled':
      return { ...state, preview: { enabled: action.enabled } };
    case 'setSelection':
      return { ...state, selection: action.selection };
    case 'clearSelection':
      return { ...state, selection: null };
  }
}

/** A change listener. */
export type EditorListener = (state: EditorContext) => void;

/**
 * The persistent editor-context store: one instance lives in `app` for the whole
 * session, so context survives camera movement and panel remounts. Devtools can
 * `subscribe` for visibility. Holds no authoritative voxel data.
 */
export class EditorStore {
  #state: EditorContext;
  readonly #listeners = new Set<EditorListener>();

  constructor(initial: EditorContext = initialEditorContext()) {
    this.#state = initial;
  }

  getState(): EditorContext {
    return this.#state;
  }

  /** Apply an action; notifies listeners only when the state actually changes. */
  dispatch(action: EditorAction): EditorContext {
    const next = reduce(this.#state, action);
    if (next !== this.#state) {
      this.#state = next;
      for (const l of this.#listeners) {
        l(next);
      }
    }
    return this.#state;
  }

  subscribe(listener: EditorListener): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }
}

// ── Geometry helpers (contract-typed, pure) ────────────────────────────────────

function faceOffset(face: Face): readonly [number, number, number] {
  switch (face) {
    case 'posX':
      return [1, 0, 0];
    case 'negX':
      return [-1, 0, 0];
    case 'posY':
      return [0, 1, 0];
    case 'negY':
      return [0, -1, 0];
    case 'posZ':
      return [0, 0, 1];
    case 'negZ':
      return [0, 0, -1];
  }
}

/** The voxel across `face` from `voxel` — the anchor a *place* edit builds on. */
export function faceNeighbor(voxel: VoxelCoord, face: Face): VoxelCoord {
  const [dx, dy, dz] = faceOffset(face);
  return { x: voxel.x + dx, y: voxel.y + dy, z: voxel.z + dz };
}

/** Half-open `[min, max)` box of side `size` (>= 1) centred on `center`. */
export function brushBox(center: VoxelCoord, size: number): { min: VoxelCoord; max: VoxelCoord } {
  const n = Math.max(1, Math.floor(size));
  const off = Math.floor((n - 1) / 2);
  const min = { x: center.x - off, y: center.y - off, z: center.z - off };
  return { min, max: { x: min.x + n, y: min.y + n, z: min.z + n } };
}

const solid = (material: number): VoxelValue => ({ kind: 'solid', material });
const EMPTY: VoxelValue = { kind: 'empty' };

// ── Proposals & preview (pure; never submit, never mutate) ─────────────────────

/**
 * The anchor cell + value an editing tool would write, or `null` for a non-editing
 * tool / no selection. `place` builds across the struck face; `remove` clears the
 * struck voxel; `paint` recolours the struck voxel with the current material.
 */
function editTarget(ctx: EditorContext): { center: VoxelCoord; value: VoxelValue } | null {
  if (ctx.selection === null || !isEditingTool(ctx.tool)) {
    return null;
  }
  switch (ctx.tool) {
    case 'place':
      return { center: faceNeighbor(ctx.selection.voxel, ctx.selection.face), value: solid(ctx.material) };
    case 'remove':
      return { center: ctx.selection.voxel, value: EMPTY };
    case 'paint':
      return { center: ctx.selection.voxel, value: solid(ctx.material) };
    default:
      return null;
  }
}

/** Expand `[min, max)` into its cells in deterministic z,y,x order. */
function boxCells(min: VoxelCoord, max: VoxelCoord): VoxelCoord[] {
  const out: VoxelCoord[] = [];
  for (let z = min.z; z < max.z; z++) {
    for (let y = min.y; y < max.y; y++) {
      for (let x = min.x; x < max.x; x++) {
        out.push({ x, y, z });
      }
    }
  }
  return out;
}

/**
 * The voxel coordinates a brush action would affect — for the non-authoritative
 * preview overlay. `select`/`inspect`, or no selection, affect nothing. `single`
 * affects one cell; `box` affects the `brushSize`-sided box around the anchor.
 */
export function previewTargets(ctx: EditorContext): VoxelCoord[] {
  const target = editTarget(ctx);
  if (target === null) {
    return [];
  }
  if (ctx.brushShape === 'single') {
    return [target.center];
  }
  const { min, max } = brushBox(target.center, ctx.brushSize);
  return boxCells(min, max);
}

/**
 * Turn the editor context + selection into a generated `VoxelCommand` proposal, or
 * `null` when there is nothing to commit (no selection, or a non-editing tool).
 * `single` → `SetVoxel`; `box` → `FillRegion`. Pure — it does not submit; the `app`
 * command path does that on commit.
 */
export function proposeCommand(ctx: EditorContext): VoxelCommand | null {
  const target = editTarget(ctx);
  if (target === null) {
    return null;
  }
  if (ctx.brushShape === 'single') {
    return { op: 'setVoxel', grid: ctx.grid, coord: target.center, value: target.value };
  }
  return { op: 'fillRegion', grid: ctx.grid, ...brushBox(target.center, ctx.brushSize), value: target.value };
}
