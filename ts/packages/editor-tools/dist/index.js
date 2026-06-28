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
// Proposal-only scene authoring controls with Rust validation feedback (#2380).
export * from './scene-authoring.js';
// Canonical scene-object hierarchy snapshot/proposal helpers over FlatSceneDocument.
export * from './scene-object-hierarchy.js';
// Proposal-only generic entity authoring controls with Rust validation feedback (#2485).
export * from './entity-authoring.js';
/** The editing tools that author voxels (have a preview + a proposal). */
export const EDITING_TOOLS = ['place', 'remove', 'paint'];
function isEditingTool(tool) {
    return EDITING_TOOLS.includes(tool);
}
/** The initial editor context. */
export function initialEditorContext(grid = 0) {
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
/** Pure reducer. Validates/normalises (e.g. brush size clamped to `>= 1` integer). */
export function reduce(state, action) {
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
/**
 * The persistent editor-context store: one instance lives in `app` for the whole
 * session, so context survives camera movement and panel remounts. Devtools can
 * `subscribe` for visibility. Holds no authoritative voxel data.
 */
export class EditorStore {
    #state;
    #listeners = new Set();
    constructor(initial = initialEditorContext()) {
        this.#state = initial;
    }
    getState() {
        return this.#state;
    }
    /** Apply an action; notifies listeners only when the state actually changes. */
    dispatch(action) {
        const next = reduce(this.#state, action);
        if (next !== this.#state) {
            this.#state = next;
            for (const l of this.#listeners) {
                l(next);
            }
        }
        return this.#state;
    }
    subscribe(listener) {
        this.#listeners.add(listener);
        return () => this.#listeners.delete(listener);
    }
}
// ── Geometry helpers (contract-typed, pure) ────────────────────────────────────
function faceOffset(face) {
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
export function faceNeighbor(voxel, face) {
    const [dx, dy, dz] = faceOffset(face);
    return { x: voxel.x + dx, y: voxel.y + dy, z: voxel.z + dz };
}
/** Half-open `[min, max)` box of side `size` (>= 1) centred on `center`. */
export function brushBox(center, size) {
    const n = Math.max(1, Math.floor(size));
    const off = Math.floor((n - 1) / 2);
    const min = { x: center.x - off, y: center.y - off, z: center.z - off };
    return { min, max: { x: min.x + n, y: min.y + n, z: min.z + n } };
}
const solid = (material) => ({ kind: 'solid', material });
const EMPTY = { kind: 'empty' };
// ── Proposals & preview (pure; never submit, never mutate) ─────────────────────
/**
 * The anchor cell + value an editing tool would write, or `null` for a non-editing
 * tool / no selection. `place` builds across the struck face; `remove` clears the
 * struck voxel; `paint` recolours the struck voxel with the current material.
 */
function editTarget(ctx) {
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
function boxCells(min, max) {
    const out = [];
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
export function previewTargets(ctx) {
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
export function proposeCommand(ctx) {
    const target = editTarget(ctx);
    if (target === null) {
        return null;
    }
    if (ctx.brushShape === 'single') {
        return { op: 'setVoxel', grid: ctx.grid, coord: target.center, value: target.value };
    }
    return { op: 'fillRegion', grid: ctx.grid, ...brushBox(target.center, ctx.brushSize), value: target.value };
}
//# sourceMappingURL=index.js.map