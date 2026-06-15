import type { AuthoringTransform, EntityAuthoringCommand, EntityId, Face, PickRay, RenderDiff, VoxelCoord } from '@asha/contracts';
import { type BrushShape, type EditorAction, type EditorContext, type EntityCapabilityFlags } from '@asha/editor-tools';
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
    readonly brushShape: BrushShape;
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
/** One selectable material in the palette: its id and a human/agent-readable label. */
export interface MaterialOption {
    readonly id: number;
    readonly label: string;
}
/**
 * Build the material palette from the loaded fixture/catalog material ids. Labels
 * default to `Material <id>` but a caller may pass catalog-sourced names. The
 * palette is data the UI offers — the editor never hardcodes a product palette.
 */
export declare function materialPalette(materialIds: readonly number[], labelFor?: (id: number) => string): MaterialOption[];
export type ControlRole = 'radiogroup' | 'listbox' | 'slider' | 'switch' | 'button';
/** One selectable option of a radiogroup/listbox control. */
export interface ControlOption {
    readonly value: string;
    readonly label: string;
    readonly selected: boolean;
}
/** An accessible, render-agnostic editor control descriptor. */
export interface EditorControl {
    /** Stable id / test handle (e.g. `data-testid`); also the `controlToAction` key. */
    readonly id: string;
    readonly role: ControlRole;
    /** Accessible label (aria-label) — what `getByLabel` / a screen reader sees. */
    readonly label: string;
    /** Current value as a string. */
    readonly value: string;
    /** Choices, for `radiogroup` / `listbox`. */
    readonly options?: readonly ControlOption[];
    /** Bounds, for `slider`. */
    readonly min?: number;
    readonly max?: number;
    /** Whether the control is currently actionable (e.g. commit needs a proposal). */
    readonly disabled?: boolean;
}
/** The maximum box side the brush-size slider offers (first-scope cap). */
export declare const MAX_BRUSH_SIZE = 8;
/**
 * The full accessible control set for the editor toolbar, derived purely from the
 * editor context and the (catalog-sourced) material palette. Commit is disabled
 * when there is no proposable edit; cancel when there is nothing selected; brush
 * size only applies to the `box` shape.
 */
export declare function buildEditorControls(ctx: EditorContext, palette: readonly MaterialOption[]): EditorControl[];
/**
 * Map a control interaction (`id` + chosen `value`) to the editor action to
 * dispatch, or `null` for the app-level command buttons (`commit`/`cancel`) which
 * the app handles (submit / clear draft). Centralises the control→action contract
 * so the DOM/agent layer only forwards interactions.
 */
export declare function controlToAction(id: string, value: string): EditorAction | null;
/** Values a value-carrying authoring control needs when its command is built. */
export interface EntityAuthoringParams {
    readonly newEntityId?: EntityId;
    readonly transform?: AuthoringTransform;
    readonly moveDelta?: readonly [number, number, number];
    readonly container?: EntityId;
}
/**
 * The accessible authoring control set for a selected entity, derived purely from
 * its capability flags. Transform/move are eligibility-gated (disabled + reason);
 * attach/contain/destroy reflect lifecycle. `create` is selection-independent.
 */
export declare function buildEntityAuthoringControls(flags: EntityCapabilityFlags): EditorControl[];
/**
 * Map an authoring control interaction to a proposal command, or `null` if the
 * control needs a parameter that was not supplied (e.g. a containment target). The
 * app submits the returned command to Rust validation; the UI never applies it.
 * `target` is the selected entity (or, for `create`, the allocated new id).
 */
export declare function entityAuthoringControlToCommand(controlId: string, target: EntityId, params?: EntityAuthoringParams): EntityAuthoringCommand | null;
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