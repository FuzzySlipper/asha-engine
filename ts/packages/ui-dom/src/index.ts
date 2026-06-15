// @asha/ui-dom — camera controls, inspectors, and non-authoritative debug overlays.
//
// Per ADR 0008 these are all **read/projection** concerns: the camera is plain
// deterministic data (the renderer turns it into a THREE camera), inspectors are
// pure functions of `(EditorContext, projected diagnostics)` holding no
// authoritative copy, and the brush/selection overlay is a set of `debug`-layer
// render diffs that mutate nothing. Imports `@asha/contracts` + `@asha/editor-tools`
// only — no `three`, no policy, no native bridge.

import type {
  AuthoringTransform,
  EntityAuthoringCommand,
  EntityId,
  Face,
  PickRay,
  RenderDiff,
  RenderHandle,
  VoxelCoord,
} from '@asha/contracts';
import { renderHandle } from '@asha/contracts';
import {
  type BrushShape,
  type EditorAction,
  type EditorContext,
  type EntityCapabilityFlags,
  type ToolMode,
  IDENTITY_AUTHORING_TRANSFORM,
  movementEligibility,
  previewTargets,
  proposeAttachCapability,
  proposeCreateEntity,
  proposeDestroyEntity,
  proposeMove,
  proposeSetContainment,
  proposeSetEntityTransform,
  proposeCommand,
  transformEligibility,
} from '@asha/editor-tools';

// ── Camera (deterministic data; renderer builds the THREE camera) ──────────────

export type Vec3 = readonly [number, number, number];

/** A deterministic camera description — stable for screenshot/golden configs. */
export interface CameraConfig {
  readonly position: Vec3;
  readonly target: Vec3;
  readonly up: Vec3;
  readonly fovDegrees: number;
}

/** A fixed default camera (deterministic): looking at the origin from +X/+Y/+Z. */
export function defaultCamera(): CameraConfig {
  return { position: [8, 8, 8], target: [0, 0, 0], up: [0, 1, 0], fovDegrees: 60 };
}

const sub = (a: Vec3, b: Vec3): Vec3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const add = (a: Vec3, b: Vec3): Vec3 => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
const scale = (a: Vec3, s: number): Vec3 => [a[0] * s, a[1] * s, a[2] * s];
const length = (a: Vec3): number => Math.hypot(a[0], a[1], a[2]);
const cross = (a: Vec3, b: Vec3): Vec3 => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
];
const normalize = (a: Vec3): Vec3 => {
  const l = length(a);
  return l === 0 ? a : scale(a, 1 / l);
};

// ── Pointer + camera → world-space pick ray (pure; no DDA, no authority) ───────

/** A pointer in normalized device coordinates: `x,y ∈ [-1, 1]`, `+y` up, centre `[0,0]`. */
export type PointerNdc = readonly [number, number];

/**
 * Build the world-space {@link PickRay} for a pointer over the viewport, given the
 * deterministic camera and viewport aspect (width / height). This is plain camera
 * un-projection (perspective, vertical `fovDegrees`) — the renderer/UI's job. The
 * voxel-grid raycast itself stays in Rust authority (`pickVoxel`); the renderer
 * never owns voxel coordinates or runs a parallel DDA.
 */
export function cameraPointerRay(
  cam: CameraConfig,
  pointer: PointerNdc,
  aspect: number,
  grid: number,
  maxDistance = 1_000,
): PickRay {
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
  const dir = normalize(
    add(
      add(forward, scale(right, px * aspect * tanHalfFov)),
      scale(trueUp, py * tanHalfFov),
    ),
  );
  return {
    grid,
    origin: [...cam.position] as [number, number, number],
    direction: [...dir] as [number, number, number],
    maxDistance,
  };
}

/** Dolly the camera toward/away from its target by a factor (clamped > 0). */
export function dolly(cam: CameraConfig, factor: number): CameraConfig {
  const offset = sub(cam.position, cam.target);
  const f = Math.max(0.01, factor);
  return { ...cam, position: add(cam.target, scale(offset, f)) };
}

/** Orbit the camera around its target by `yaw` (about up/Y) — deterministic. */
export function orbitYaw(cam: CameraConfig, yawRadians: number): CameraConfig {
  const o = sub(cam.position, cam.target);
  const c = Math.cos(yawRadians);
  const s = Math.sin(yawRadians);
  // Rotate the offset about the Y axis.
  const rotated: Vec3 = [o[0] * c + o[2] * s, o[1], -o[0] * s + o[2] * c];
  return { ...cam, position: add(cam.target, rotated) };
}

/**
 * Camera collision: pull the camera out of any solid voxel using the shared
 * collision query (`isSolid`, backed by `svc-collision` when wired — injected so
 * this stays a pure, testable function). Steps the camera back along the
 * target→position ray until it is in free space (bounded iterations).
 */
export function clampCameraOutOfSolid(
  cam: CameraConfig,
  isSolid: (p: Vec3) => boolean,
  step = 0.5,
  maxSteps = 64,
): CameraConfig {
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

// ── Inspector (pure read model — no hidden authoritative copy) ─────────────────

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
export function inspect(ctx: EditorContext, diagnostics: Diagnostics = {}): InspectorReadout {
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

// ── Material palette (read model from the loaded catalog — never hardcoded) ─────

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
export function materialPalette(
  materialIds: readonly number[],
  labelFor: (id: number) => string = (id) => `Material ${id}`,
): MaterialOption[] {
  return materialIds.map((id) => ({ id, label: labelFor(id) }));
}

// ── Accessible editor controls (pure model; a DOM/Playwright layer renders these) ──
//
// The toolbar is described as data so both a user (via the DOM) and an agent (via
// Playwright `getByRole`/`getByLabel`) can drive the editor. Each control carries a
// stable `id` (test handle), an ARIA `role`, an accessible `label`, its current
// `value`, and (for choices) `options`. State changes route through
// `controlToAction` → an `EditorAction`; the two command buttons are app-level.

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

const TOOL_LABELS: Record<ToolMode, string> = {
  place: 'Place',
  remove: 'Remove',
  paint: 'Paint',
  select: 'Select',
  inspect: 'Inspect',
};

const SHAPE_LABELS: Record<BrushShape, string> = {
  single: 'Single cell',
  box: 'Box fill',
};

const opt = (value: string, label: string, current: string): ControlOption => ({
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
export function buildEditorControls(
  ctx: EditorContext,
  palette: readonly MaterialOption[],
): EditorControl[] {
  const tools: ToolMode[] = ['place', 'remove', 'paint', 'select', 'inspect'];
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
      options: (['single', 'box'] as BrushShape[]).map((s) => opt(s, SHAPE_LABELS[s], ctx.brushShape)),
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
export function controlToAction(id: string, value: string): EditorAction | null {
  switch (id) {
    case 'tool':
      return { type: 'setTool', tool: value as ToolMode };
    case 'material':
      return { type: 'setMaterial', material: Number(value) };
    case 'brush-shape':
      return { type: 'setBrushShape', shape: value as BrushShape };
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

// ── Accessible generic entity authoring controls (#2485) ───────────────────────
//
// The authoring panel is described as data (like the voxel toolbar) so both a user
// (DOM) and an agent (Playwright `getByRole`/`getByLabel`) can drive generic entity
// authoring. Each control carries a stable `id`/test handle and an ARIA `label`;
// eligibility-gated verbs are `disabled` with the classified reason appended to the
// label so an ineligible control is visibly explained. State never lives here — the
// controls are a pure function of the selected entity's capability flags, and a
// control interaction maps to a proposal the app submits to Rust validation.

/** Values a value-carrying authoring control needs when its command is built. */
export interface EntityAuthoringParams {
  readonly newEntityId?: EntityId;
  readonly transform?: AuthoringTransform;
  readonly moveDelta?: readonly [number, number, number];
  readonly container?: EntityId;
}

function gatedLabel(base: string, gate: { readonly eligible: boolean; readonly reason: string | null }): string {
  return gate.eligible ? base : `${base} (${gate.reason})`;
}

/**
 * The accessible authoring control set for a selected entity, derived purely from
 * its capability flags. Transform/move are eligibility-gated (disabled + reason);
 * attach/contain/destroy reflect lifecycle. `create` is selection-independent.
 */
export function buildEntityAuthoringControls(flags: EntityCapabilityFlags): EditorControl[] {
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
export function entityAuthoringControlToCommand(
  controlId: string,
  target: EntityId,
  params: EntityAuthoringParams = {},
): EntityAuthoringCommand | null {
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
export function previewOverlayDiffs(
  ctx: EditorContext,
  voxelSize = 1,
  handleBase: number = OVERLAY_HANDLE_BASE,
): RenderDiff[] {
  if (!ctx.preview.enabled) {
    return [];
  }
  return previewTargets(ctx).map((cell, i): RenderDiff => {
    const handle: RenderHandle = renderHandle(handleBase + i);
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
