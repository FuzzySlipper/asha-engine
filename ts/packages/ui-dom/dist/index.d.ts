import type { Face, PickRay, RenderDiff, VoxelCoord } from '@asha/contracts';
import { type EditorContext } from '@asha/editor-tools';
export type Vec3 = readonly [number, number, number];
/** A deterministic camera description — stable for screenshot/golden configs. */
export interface CameraConfig {
    readonly position: Vec3;
    readonly target: Vec3;
    readonly up: Vec3;
    readonly fovDegrees: number;
}
/** A fixed default camera (deterministic): looking at the origin from +X/+Y/+Z. */
export declare function defaultCamera(): CameraConfig;
/** A pointer in normalized device coordinates: `x,y ∈ [-1, 1]`, `+y` up, centre `[0,0]`. */
export type PointerNdc = readonly [number, number];
/**
 * Build the world-space {@link PickRay} for a pointer over the viewport, given the
 * deterministic camera and viewport aspect (width / height). This is plain camera
 * un-projection (perspective, vertical `fovDegrees`) — the renderer/UI's job. The
 * voxel-grid raycast itself stays in Rust authority (`pickVoxel`); the renderer
 * never owns voxel coordinates or runs a parallel DDA.
 */
export declare function cameraPointerRay(cam: CameraConfig, pointer: PointerNdc, aspect: number, grid: number, maxDistance?: number): PickRay;
/** Dolly the camera toward/away from its target by a factor (clamped > 0). */
export declare function dolly(cam: CameraConfig, factor: number): CameraConfig;
/** Orbit the camera around its target by `yaw` (about up/Y) — deterministic. */
export declare function orbitYaw(cam: CameraConfig, yawRadians: number): CameraConfig;
/**
 * Camera collision: pull the camera out of any solid voxel using the shared
 * collision query (`isSolid`, backed by `svc-collision` when wired — injected so
 * this stays a pure, testable function). Steps the camera back along the
 * target→position ray until it is in free space (bounded iterations).
 */
export declare function clampCameraOutOfSolid(cam: CameraConfig, isSolid: (p: Vec3) => boolean, step?: number, maxSteps?: number): CameraConfig;
/** Projected/devtools diagnostics the inspector may surface (never stored here). */
export interface Diagnostics {
    readonly residentChunks?: number;
    readonly colliderChunks?: number;
    readonly lastMeshQuads?: number;
}
/**
 * A flat, readonly inspector readout. A pure function of its inputs — it copies no
 * authoritative voxel state; `selection` is a picked anchor, not voxel data.
 */
export interface InspectorReadout {
    readonly tool: EditorContext['tool'];
    readonly brushSize: number;
    readonly material: number;
    readonly selectionMode: EditorContext['selectionMode'];
    readonly snapping: boolean;
    readonly previewEnabled: boolean;
    readonly selectedVoxel: VoxelCoord | null;
    readonly selectedFace: Face | null;
    readonly affectedCells: number;
    readonly diagnostics: Diagnostics;
}
/** Build the inspector readout from editor context + (optional) projected diagnostics. */
export declare function inspect(ctx: EditorContext, diagnostics?: Diagnostics): InspectorReadout;
/** Reserved handle base for editor overlay nodes; well above projected scene handles. */
export declare const OVERLAY_HANDLE_BASE = 1000000;
/**
 * Render diffs that draw the current brush/selection preview as wireframe debug
 * cubes on the **debug** layer — visually distinct from committed terrain and
 * authoritative of nothing. Returns `create` ops (the caller destroys the previous
 * overlay handles before applying). Empty when preview is disabled or nothing is
 * selected.
 */
export declare function previewOverlayDiffs(ctx: EditorContext, voxelSize?: number, handleBase?: number): RenderDiff[];
//# sourceMappingURL=index.d.ts.map