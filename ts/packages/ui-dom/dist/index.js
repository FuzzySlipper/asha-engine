// @asha/ui-dom — camera controls, inspectors, and non-authoritative debug overlays.
//
// Per ADR 0008 these are all **read/projection** concerns: the camera is plain
// deterministic data (the renderer turns it into a THREE camera), inspectors are
// pure functions of `(EditorContext, projected diagnostics)` holding no
// authoritative copy, and the brush/selection overlay is a set of `debug`-layer
// render diffs that mutate nothing. Imports `@asha/contracts` + `@asha/editor-tools`
// only — no `three`, no policy, no native bridge.
import { renderHandle } from '@asha/contracts';
import { previewTargets } from '@asha/editor-tools';
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