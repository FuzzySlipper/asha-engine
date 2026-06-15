// @asha/ui-dom — camera controls, inspectors, and non-authoritative debug overlays.
//
// Per ADR 0008 these are all **read/projection** concerns: the camera is plain
// deterministic data (the renderer turns it into a THREE camera), inspectors are
// pure functions of `(EditorContext, projected diagnostics)` holding no
// authoritative copy, and the brush/selection overlay is a set of `debug`-layer
// render diffs that mutate nothing. Imports `@asha/contracts` + `@asha/editor-tools`
// only — no `three`, no policy, no native bridge.
import { renderHandle } from '@asha/contracts';
import { IDENTITY_AUTHORING_TRANSFORM, movementEligibility, previewTargets, proposeAttachCapability, proposeCreateEntity, proposeDestroyEntity, proposeMove, proposeSetContainment, proposeSetEntityTransform, proposeCommand, transformEligibility, } from '@asha/editor-tools';
/** A fixed default camera (deterministic): looking at the origin from +X/+Y/+Z. */
export function defaultCamera() {
    return { position: [8, 8, 8], target: [0, 0, 0], up: [0, 1, 0], fovDegrees: 60 };
}
const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const add = (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
const scale = (a, s) => [a[0] * s, a[1] * s, a[2] * s];
const length = (a) => Math.hypot(a[0], a[1], a[2]);
const cross = (a, b) => [
    a[1] * b[2] - a[2] * b[1],
    a[2] * b[0] - a[0] * b[2],
    a[0] * b[1] - a[1] * b[0],
];
const normalize = (a) => {
    const l = length(a);
    return l === 0 ? a : scale(a, 1 / l);
};
/**
 * Build the world-space {@link PickRay} for a pointer over the viewport, given the
 * deterministic camera and viewport aspect (width / height). This is plain camera
 * un-projection (perspective, vertical `fovDegrees`) — the renderer/UI's job. The
 * voxel-grid raycast itself stays in Rust authority (`pickVoxel`); the renderer
 * never owns voxel coordinates or runs a parallel DDA.
 */
export function cameraPointerRay(cam, pointer, aspect, grid, maxDistance = 1_000) {
    const forward = normalize(sub(cam.target, cam.position));
    // Right-handed basis; guard a degenerate up parallel to forward.
    let right = cross(forward, cam.up);
    if (length(right) === 0) {
        right = cross(forward, [0, 0, 1]);
    }
    right = normalize(right);
    const trueUp = cross(right, forward);
    const tanHalfFov = Math.tan((cam.fovDegrees * Math.PI) / 360);
    const [px, py] = pointer;
    const dir = normalize(add(add(forward, scale(right, px * aspect * tanHalfFov)), scale(trueUp, py * tanHalfFov)));
    return {
        grid,
        origin: [...cam.position],
        direction: [...dir],
        maxDistance,
    };
}
/** Dolly the camera toward/away from its target by a factor (clamped > 0). */
export function dolly(cam, factor) {
    const offset = sub(cam.position, cam.target);
    const f = Math.max(0.01, factor);
    return { ...cam, position: add(cam.target, scale(offset, f)) };
}
/** Orbit the camera around its target by `yaw` (about up/Y) — deterministic. */
export function orbitYaw(cam, yawRadians) {
    const o = sub(cam.position, cam.target);
    const c = Math.cos(yawRadians);
    const s = Math.sin(yawRadians);
    // Rotate the offset about the Y axis.
    const rotated = [o[0] * c + o[2] * s, o[1], -o[0] * s + o[2] * c];
    return { ...cam, position: add(cam.target, rotated) };
}
/**
 * Camera collision: pull the camera out of any solid voxel using the shared
 * collision query (`isSolid`, backed by `svc-collision` when wired — injected so
 * this stays a pure, testable function). Steps the camera back along the
 * target→position ray until it is in free space (bounded iterations).
 */
export function clampCameraOutOfSolid(cam, isSolid, step = 0.5, maxSteps = 64) {
    if (!isSolid(cam.position)) {
        return cam;
    }
    const dir = sub(cam.position, cam.target);
    const len = length(dir);
    if (len === 0) {
        return cam;
    }
    const unit = scale(dir, 1 / len);
    let pos = cam.position;
    for (let i = 0; i < maxSteps; i++) {
        pos = add(pos, scale(unit, step));
        if (!isSolid(pos)) {
            break;
        }
    }
    return { ...cam, position: pos };
}
/** Build the inspector readout from editor context + (optional) projected diagnostics. */
export function inspect(ctx, diagnostics = {}) {
    return {
        tool: ctx.tool,
        brushShape: ctx.brushShape,
        brushSize: ctx.brushSize,
        material: ctx.material,
        selectionMode: ctx.selectionMode,
        snapping: ctx.snapping,
        previewEnabled: ctx.preview.enabled,
        selectedVoxel: ctx.selection?.voxel ?? null,
        selectedFace: ctx.selection?.face ?? null,
        affectedCells: previewTargets(ctx).length,
        diagnostics,
    };
}
/**
 * Build the material palette from the loaded fixture/catalog material ids. Labels
 * default to `Material <id>` but a caller may pass catalog-sourced names. The
 * palette is data the UI offers — the editor never hardcodes a product palette.
 */
export function materialPalette(materialIds, labelFor = (id) => `Material ${id}`) {
    return materialIds.map((id) => ({ id, label: labelFor(id) }));
}
const TOOL_LABELS = {
    place: 'Place',
    remove: 'Remove',
    paint: 'Paint',
    select: 'Select',
    inspect: 'Inspect',
};
const SHAPE_LABELS = {
    single: 'Single cell',
    box: 'Box fill',
};
const opt = (value, label, current) => ({
    value,
    label,
    selected: value === current,
});
/** The maximum box side the brush-size slider offers (first-scope cap). */
export const MAX_BRUSH_SIZE = 8;
/**
 * The full accessible control set for the editor toolbar, derived purely from the
 * editor context and the (catalog-sourced) material palette. Commit is disabled
 * when there is no proposable edit; cancel when there is nothing selected; brush
 * size only applies to the `box` shape.
 */
export function buildEditorControls(ctx, palette) {
    const tools = ['place', 'remove', 'paint', 'select', 'inspect'];
    return [
        {
            id: 'tool',
            role: 'radiogroup',
            label: 'Tool',
            value: ctx.tool,
            options: tools.map((t) => opt(t, TOOL_LABELS[t], ctx.tool)),
        },
        {
            id: 'material',
            role: 'listbox',
            label: 'Material',
            value: String(ctx.material),
            options: palette.map((m) => opt(String(m.id), m.label, String(ctx.material))),
        },
        {
            id: 'brush-shape',
            role: 'radiogroup',
            label: 'Brush shape',
            value: ctx.brushShape,
            options: ['single', 'box'].map((s) => opt(s, SHAPE_LABELS[s], ctx.brushShape)),
        },
        {
            id: 'brush-size',
            role: 'slider',
            label: 'Brush size',
            value: String(ctx.brushSize),
            min: 1,
            max: MAX_BRUSH_SIZE,
            disabled: ctx.brushShape !== 'box',
        },
        {
            id: 'snapping',
            role: 'switch',
            label: 'Snapping',
            value: ctx.snapping ? 'on' : 'off',
        },
        {
            id: 'preview',
            role: 'switch',
            label: 'Preview overlay',
            value: ctx.preview.enabled ? 'on' : 'off',
        },
        {
            id: 'commit',
            role: 'button',
            label: 'Commit edit',
            value: 'commit',
            disabled: proposeCommand(ctx) === null,
        },
        {
            id: 'cancel',
            role: 'button',
            label: 'Cancel edit',
            value: 'cancel',
            disabled: ctx.selection === null,
        },
    ];
}
/**
 * Map a control interaction (`id` + chosen `value`) to the editor action to
 * dispatch, or `null` for the app-level command buttons (`commit`/`cancel`) which
 * the app handles (submit / clear draft). Centralises the control→action contract
 * so the DOM/agent layer only forwards interactions.
 */
export function controlToAction(id, value) {
    switch (id) {
        case 'tool':
            return { type: 'setTool', tool: value };
        case 'material':
            return { type: 'setMaterial', material: Number(value) };
        case 'brush-shape':
            return { type: 'setBrushShape', shape: value };
        case 'brush-size':
            return { type: 'setBrushSize', size: Number(value) };
        case 'snapping':
            return { type: 'setSnapping', snapping: value === 'on' };
        case 'preview':
            return { type: 'setPreviewEnabled', enabled: value === 'on' };
        default:
            return null; // commit / cancel are app-level
    }
}
function gatedLabel(base, gate) {
    return gate.eligible ? base : `${base} (${gate.reason})`;
}
/**
 * The accessible authoring control set for a selected entity, derived purely from
 * its capability flags. Transform/move are eligibility-gated (disabled + reason);
 * attach/contain/destroy reflect lifecycle. `create` is selection-independent.
 */
export function buildEntityAuthoringControls(flags) {
    const transformGate = transformEligibility(flags);
    const moveGate = movementEligibility(flags);
    const alive = flags.lifecycle === 'active';
    return [
        { id: 'entity-create', role: 'button', label: 'Create entity', value: 'create' },
        {
            id: 'entity-set-transform',
            role: 'button',
            label: gatedLabel('Set transform', transformGate),
            value: 'setTransform',
            disabled: !transformGate.eligible,
        },
        {
            id: 'entity-move',
            role: 'button',
            label: gatedLabel('Move', moveGate),
            value: 'move',
            disabled: !moveGate.eligible,
        },
        {
            id: 'entity-attach-render',
            role: 'button',
            label: 'Attach render',
            value: 'attachRender',
            disabled: !alive,
        },
        {
            id: 'entity-attach-collision',
            role: 'button',
            label: 'Attach collision',
            value: 'attachCollision',
            disabled: !alive,
        },
        {
            id: 'entity-contain',
            role: 'button',
            label: 'Contain in…',
            value: 'contain',
            disabled: !alive,
        },
        {
            id: 'entity-destroy',
            role: 'button',
            label: 'Destroy',
            value: 'destroy',
            disabled: flags.lifecycle === 'tombstoned',
        },
    ];
}
/**
 * Map an authoring control interaction to a proposal command, or `null` if the
 * control needs a parameter that was not supplied (e.g. a containment target). The
 * app submits the returned command to Rust validation; the UI never applies it.
 * `target` is the selected entity (or, for `create`, the allocated new id).
 */
export function entityAuthoringControlToCommand(controlId, target, params = {}) {
    switch (controlId) {
        case 'entity-create':
            return proposeCreateEntity(params.newEntityId ?? target, { kind: 'runtimeCreated', by: null });
        case 'entity-set-transform':
            return proposeSetEntityTransform(target, params.transform ?? IDENTITY_AUTHORING_TRANSFORM);
        case 'entity-move':
            return params.moveDelta ? proposeMove(target, params.moveDelta) : null;
        case 'entity-attach-render':
            return proposeAttachCapability(target, { kind: 'render', visible: true });
        case 'entity-attach-collision':
            return proposeAttachCapability(target, { kind: 'collision', staticCollider: false });
        case 'entity-contain':
            return params.container !== undefined ? proposeSetContainment(target, params.container) : null;
        case 'entity-destroy':
            return proposeDestroyEntity(target);
        default:
            return null;
    }
}
// ── Debug overlay (non-authoritative `debug`-layer render diffs) ───────────────
/** Reserved handle base for editor overlay nodes; well above projected scene handles. */
export const OVERLAY_HANDLE_BASE = 1_000_000;
/**
 * Render diffs that draw the current brush/selection preview as wireframe debug
 * cubes on the **debug** layer — visually distinct from committed terrain and
 * authoritative of nothing. Returns `create` ops (the caller destroys the previous
 * overlay handles before applying). Empty when preview is disabled or nothing is
 * selected.
 */
export function previewOverlayDiffs(ctx, voxelSize = 1, handleBase = OVERLAY_HANDLE_BASE) {
    if (!ctx.preview.enabled) {
        return [];
    }
    return previewTargets(ctx).map((cell, i) => {
        const handle = renderHandle(handleBase + i);
        return {
            op: 'create',
            handle,
            parent: null,
            node: {
                geometry: { shape: 'cube' },
                // Translucent magenta wireframe — clearly not committed terrain.
                material: { color: [1, 0, 1, 0.5], wireframe: true },
                transform: {
                    translation: [
                        (cell.x + 0.5) * voxelSize,
                        (cell.y + 0.5) * voxelSize,
                        (cell.z + 0.5) * voxelSize,
                    ],
                    rotation: [0, 0, 0, 1],
                    scale: [voxelSize, voxelSize, voxelSize],
                },
                visible: true,
                layer: 'debug',
                metadata: { source: null, tags: [], label: 'brush-preview' },
            },
        };
    });
}
//# sourceMappingURL=index.js.map