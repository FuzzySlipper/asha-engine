// Projection-only interactive viewer for downstream visual-authoring tools.

import type {
  PerspectiveProjection,
  RenderFrameDiff,
} from '@asha/contracts';
import {
  mountAshaRendererEditorViewport,
  type AshaRendererEditorViewport,
  type AshaRendererEditorViewportBufferSource,
  type AshaRendererEditorViewportCamera,
  type AshaRendererEditorViewportChannelReceipt,
  type AshaRendererEditorViewportPickReceipt,
  type AshaRendererEditorViewportPickRequest,
  type AshaRendererEditorViewportSize,
  type AshaRendererEditorViewportSizeReceipt,
} from './editor-viewport.js';
import {
  type AshaRendererAnimatedMeshResourceManifest,
  type AshaRendererAnimatedMeshResourceResolver,
} from './animated-mesh-host.js';
import { resolveAshaStoredEditorCamera } from './stored-editor-camera.js';

export const ASHA_RENDERER_INSPECTION_SURFACE_COMPATIBILITY_VERSION = 'inspection-surface.v0';

type InspectionVector = readonly [number, number, number];

export interface AshaRendererInspectionSurfaceControlsOptions {
  readonly enabled?: boolean;
  readonly initialPosition?: InspectionVector;
  readonly initialTarget?: InspectionVector;
  /** World units travelled per second while a movement key is held. */
  readonly moveSpeed?: number;
  /** Orbit degrees applied per mouse pixel while the primary button is held. */
  readonly orbitDegreesPerPixel?: number;
  readonly projection?: PerspectiveProjection;
}

export interface AshaRendererInspectionSurfaceOptions {
  readonly animatedMeshManifest?: AshaRendererAnimatedMeshResourceManifest;
  readonly autoStart?: boolean;
  readonly bufferSource?: AshaRendererEditorViewportBufferSource;
  readonly clearColor?: number;
  readonly controls?: AshaRendererInspectionSurfaceControlsOptions;
  /** A complete retained projection frame. Later replacements are atomic. */
  readonly frame?: RenderFrameDiff;
  readonly pixelRatio?: number;
  readonly resolveAnimatedMeshResource?: AshaRendererAnimatedMeshResourceResolver;
}

export type AshaRendererInspectionSurfaceStatus = 'mounted' | 'running' | 'stopped' | 'disposed';

export interface AshaRendererInspectionSurfaceReadout {
  readonly kind: 'asha_renderer_inspection_surface_readout.v0';
  readonly compatibilityVersion: typeof ASHA_RENDERER_INSPECTION_SURFACE_COMPATIBILITY_VERSION;
  /** Camera input here is disposable renderer state and never RuntimeSession authority. */
  readonly authority: 'projection_only_inspection';
  readonly camera: AshaRendererEditorViewportCamera;
  readonly dragging: boolean;
  readonly pressedMovementKeys: readonly string[];
  readonly retainedFrameHash: string;
  readonly retainedOpCount: number;
  readonly status: AshaRendererInspectionSurfaceStatus;
  readonly viewportHash: string;
}

export interface AshaRendererInspectionSurface {
  readonly kind: 'asha_renderer_inspection_surface.v0';
  readonly authority: 'projection_only_inspection';
  readonly canvas: HTMLCanvasElement;
  readonly camera: () => AshaRendererEditorViewportCamera;
  readonly dispose: () => void;
  readonly pick: (request: AshaRendererEditorViewportPickRequest) => AshaRendererEditorViewportPickReceipt;
  readonly readout: () => AshaRendererInspectionSurfaceReadout;
  readonly renderOnce: (timeMs?: number) => void;
  readonly replaceFrame: (frame: RenderFrameDiff) => AshaRendererEditorViewportChannelReceipt;
  readonly resize: (size: AshaRendererEditorViewportSize) => AshaRendererEditorViewportSizeReceipt;
  readonly resizeToCanvas: () => AshaRendererEditorViewportSizeReceipt;
  readonly start: () => void;
  readonly stop: () => void;
}

interface AshaRendererInspectionAnimationScheduler {
  readonly cancel: (handle: number) => void;
  readonly now: () => number;
  readonly request: (callback: (timeMs: number) => void) => number;
}

interface AshaRendererInspectionResizeObserver {
  readonly disconnect: () => void;
  readonly observe: (target: Element) => void;
}

interface AshaRendererInspectionEnvironment {
  readonly animation: AshaRendererInspectionAnimationScheduler;
  readonly createResizeObserver: (
    callback: () => void,
  ) => AshaRendererInspectionResizeObserver | null;
  readonly devicePixelRatio: () => number;
}

interface InspectionControls {
  readonly camera: () => AshaRendererEditorViewportCamera;
  readonly dispose: () => void;
  readonly dragging: () => boolean;
  readonly pressedMovementKeys: () => readonly string[];
  readonly update: (deltaSeconds: number) => void;
}

const DEFAULT_PROJECTION: PerspectiveProjection = {
  fovYDegrees: 55,
  near: 0.05,
  far: 1000,
};
const MOVEMENT_KEYS = ['KeyA', 'KeyD', 'KeyS', 'KeyW'] as const;

export async function mountAshaRendererInspectionSurface(
  canvas: HTMLCanvasElement,
  options: AshaRendererInspectionSurfaceOptions = {},
): Promise<AshaRendererInspectionSurface> {
  const viewport = await mountAshaRendererEditorViewport(canvas, {
    autoStart: false,
    ...(options.animatedMeshManifest === undefined
      ? {}
      : { animatedMeshManifest: options.animatedMeshManifest }),
    ...(options.bufferSource === undefined ? {} : { bufferSource: options.bufferSource }),
    ...(options.clearColor === undefined ? {} : { clearColor: options.clearColor }),
    ...(options.pixelRatio === undefined ? {} : { pixelRatio: options.pixelRatio }),
    ...(options.resolveAnimatedMeshResource === undefined
      ? {}
      : { resolveAnimatedMeshResource: options.resolveAnimatedMeshResource }),
  });
  try {
    return createAshaRendererInspectionSurfaceWithViewport(
      canvas,
      viewport,
      options,
      browserInspectionEnvironment(),
    );
  } catch (error) {
    viewport.dispose();
    throw error;
  }
}

/** Internal conformance seam; downstream consumers use the package-root mount helper. */
export function createAshaRendererInspectionSurfaceWithViewport(
  canvas: HTMLCanvasElement,
  viewport: AshaRendererEditorViewport,
  options: AshaRendererInspectionSurfaceOptions = {},
  environment: AshaRendererInspectionEnvironment = browserInspectionEnvironment(),
): AshaRendererInspectionSurface {
  const controls = createInspectionControls(canvas, viewport, options.controls);
  let animationHandle: number | null = null;
  let lastRenderTimeMs: number | null = null;
  let status: AshaRendererInspectionSurfaceStatus = 'mounted';

  const resizeToCanvas = (): AshaRendererEditorViewportSizeReceipt => viewport.resize({
    width: Math.max(1, Math.round(canvas.clientWidth || canvas.width || 1)),
    height: Math.max(1, Math.round(canvas.clientHeight || canvas.height || 1)),
    pixelRatio: options.pixelRatio ?? environment.devicePixelRatio(),
  });

  let resizeObserver: AshaRendererInspectionResizeObserver | null = null;

  const renderOnce = (timeMs = environment.animation.now()): void => {
    if (status === 'disposed') {
      return;
    }
    const deltaSeconds = lastRenderTimeMs === null
      ? 0
      : Math.min(0.1, Math.max(0, (timeMs - lastRenderTimeMs) / 1000));
    lastRenderTimeMs = timeMs;
    controls.update(deltaSeconds);
    viewport.renderOnce(timeMs);
  };

  const tick = (timeMs: number): void => {
    if (status !== 'running') {
      return;
    }
    renderOnce(timeMs);
    animationHandle = environment.animation.request(tick);
  };

  const start = (): void => {
    if (status === 'disposed' || status === 'running') {
      return;
    }
    status = 'running';
    lastRenderTimeMs = null;
    animationHandle = environment.animation.request(tick);
  };

  const stop = (): void => {
    if (status === 'disposed' || status === 'stopped') {
      return;
    }
    if (animationHandle !== null) {
      environment.animation.cancel(animationHandle);
      animationHandle = null;
    }
    status = 'stopped';
    lastRenderTimeMs = null;
  };

  const replaceFrame = (frame: RenderFrameDiff): AshaRendererEditorViewportChannelReceipt =>
    viewport.channels.authored.replace(frame);

  try {
    resizeObserver = environment.createResizeObserver(() => {
      if (status !== 'disposed') {
        resizeToCanvas();
      }
    });
    resizeObserver?.observe(canvas);

    if (options.frame !== undefined) {
      const initialReceipt = replaceFrame(options.frame);
      if (!initialReceipt.applied) {
        const diagnostic = initialReceipt.diagnostics[0];
        throw new TypeError(diagnostic?.message ?? 'inspection surface rejected its initial frame');
      }
    }

    resizeToCanvas();
    renderOnce(0);
    if (options.autoStart !== false) {
      start();
    }
  } catch (error) {
    controls.dispose();
    resizeObserver?.disconnect();
    viewport.dispose();
    throw error;
  }

  return {
    kind: 'asha_renderer_inspection_surface.v0',
    authority: 'projection_only_inspection',
    canvas,
    camera: () => controls.camera(),
    pick: (request) => viewport.pick(request),
    readout: () => {
      const viewportReadout = viewport.readout();
      const authored = viewportReadout.channels.find((channel) => channel.channel === 'authored');
      return {
        kind: 'asha_renderer_inspection_surface_readout.v0',
        compatibilityVersion: ASHA_RENDERER_INSPECTION_SURFACE_COMPATIBILITY_VERSION,
        authority: 'projection_only_inspection',
        camera: controls.camera(),
        dragging: controls.dragging(),
        pressedMovementKeys: controls.pressedMovementKeys(),
        retainedFrameHash: authored?.hash ?? '',
        retainedOpCount: authored?.retainedOpCount ?? 0,
        status,
        viewportHash: viewportReadout.viewportHash,
      };
    },
    renderOnce,
    replaceFrame,
    resize: (size) => viewport.resize(size),
    resizeToCanvas,
    start,
    stop,
    dispose: () => {
      if (status === 'disposed') {
        return;
      }
      stop();
      status = 'disposed';
      controls.dispose();
      resizeObserver?.disconnect();
      viewport.dispose();
    },
  };
}

function createInspectionControls(
  canvas: HTMLCanvasElement,
  viewport: AshaRendererEditorViewport,
  options: AshaRendererInspectionSurfaceControlsOptions | undefined,
): InspectionControls {
  const enabled = options?.enabled !== false;
  const moveSpeed = requirePositiveFinite(options?.moveSpeed ?? 5, 'inspection moveSpeed');
  const orbitDegreesPerPixel = requirePositiveFinite(
    options?.orbitDegreesPerPixel ?? 0.24,
    'inspection orbitDegreesPerPixel',
  );
  const projection = options?.projection ?? DEFAULT_PROJECTION;
  const initialPosition = options?.initialPosition ?? [4, 4, 8];
  let target: InspectionVector = [...(options?.initialTarget ?? [0, 0, 0])];
  const offset = subtract(initialPosition, target);
  const distance = vectorLength(offset);
  if (!allFinite([initialPosition, target]) || distance <= 0.000_001) {
    throw new TypeError('inspection initialPosition and initialTarget must be finite and distinct');
  }
  let yawRadians = Math.atan2(offset[0], offset[2]);
  let pitchRadians = Math.asin(clamp(offset[1] / distance, -1, 1));
  let camera = resolveCamera(positionFromOrbit(target, distance, yawRadians, pitchRadians), target, projection);
  let dragging = false;
  const pressedKeys = new Set<string>();
  const ownerDocument = canvas.ownerDocument;
  const ownerWindow = ownerDocument.defaultView;

  if (canvas.tabIndex < 0) {
    canvas.tabIndex = 0;
  }
  canvas.style.touchAction = 'none';

  const commitCamera = (
    nextTarget: InspectionVector,
    nextYawRadians: number,
    nextPitchRadians: number,
  ): boolean => {
    const nextCamera = resolveCamera(
      positionFromOrbit(nextTarget, distance, nextYawRadians, nextPitchRadians),
      nextTarget,
      projection,
    );
    const receipt = viewport.setCamera(nextCamera);
    if (!receipt.applied) {
      return false;
    }
    target = nextTarget;
    yawRadians = nextYawRadians;
    pitchRadians = nextPitchRadians;
    camera = nextCamera;
    return true;
  };

  const onPointerDown = (event: PointerEvent): void => {
    if (!enabled || event.button !== 0) {
      return;
    }
    event.preventDefault();
    canvas.focus({ preventScroll: true });
    dragging = true;
  };
  const onPointerUp = (event: PointerEvent): void => {
    if (event.button === 0) {
      dragging = false;
    }
  };
  const onMouseMove = (event: MouseEvent): void => {
    if (!enabled || !dragging || !Number.isFinite(event.movementX) || !Number.isFinite(event.movementY)) {
      return;
    }
    event.preventDefault();
    const nextYawRadians = yawRadians - degreesToRadians(event.movementX * orbitDegreesPerPixel);
    const nextPitchRadians = clamp(
      pitchRadians + degreesToRadians(event.movementY * orbitDegreesPerPixel),
      degreesToRadians(-85),
      degreesToRadians(85),
    );
    commitCamera(target, nextYawRadians, nextPitchRadians);
  };
  const onKeyDown = (event: KeyboardEvent): void => {
    if (!enabled || ownerDocument.activeElement !== canvas || !isMovementKey(event.code)) {
      return;
    }
    event.preventDefault();
    pressedKeys.add(event.code);
  };
  const onKeyUp = (event: KeyboardEvent): void => {
    if (!isMovementKey(event.code)) {
      return;
    }
    event.preventDefault();
    pressedKeys.delete(event.code);
  };
  const clearInputState = (): void => {
    dragging = false;
    pressedKeys.clear();
  };
  const onVisibilityChange = (): void => {
    if (ownerDocument.visibilityState !== 'visible') {
      clearInputState();
    }
  };

  if (!commitCamera(target, yawRadians, pitchRadians)) {
    throw new TypeError('inspection camera was rejected during mount');
  }
  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointercancel', clearInputState);
  canvas.addEventListener('blur', clearInputState);
  ownerDocument.addEventListener('pointerup', onPointerUp);
  ownerDocument.addEventListener('mousemove', onMouseMove);
  ownerDocument.addEventListener('keydown', onKeyDown);
  ownerDocument.addEventListener('keyup', onKeyUp);
  ownerDocument.addEventListener('visibilitychange', onVisibilityChange);
  ownerWindow?.addEventListener('blur', clearInputState);

  return {
    camera: () => camera,
    dragging: () => dragging,
    pressedMovementKeys: () => [...pressedKeys].sort(),
    update: (deltaSeconds) => {
      if (!enabled || pressedKeys.size === 0 || deltaSeconds <= 0) {
        return;
      }
      const forwardAxis = (pressedKeys.has('KeyW') ? 1 : 0) - (pressedKeys.has('KeyS') ? 1 : 0);
      const rightAxis = (pressedKeys.has('KeyD') ? 1 : 0) - (pressedKeys.has('KeyA') ? 1 : 0);
      const movement = horizontalMovement(camera, forwardAxis, rightAxis);
      if (movement === null) {
        return;
      }
      const step = moveSpeed * deltaSeconds;
      const nextTarget = add(target, scale(movement, step));
      commitCamera(nextTarget, yawRadians, pitchRadians);
    },
    dispose: () => {
      canvas.removeEventListener('pointerdown', onPointerDown);
      canvas.removeEventListener('pointercancel', clearInputState);
      canvas.removeEventListener('blur', clearInputState);
      ownerDocument.removeEventListener('pointerup', onPointerUp);
      ownerDocument.removeEventListener('mousemove', onMouseMove);
      ownerDocument.removeEventListener('keydown', onKeyDown);
      ownerDocument.removeEventListener('keyup', onKeyUp);
      ownerDocument.removeEventListener('visibilitychange', onVisibilityChange);
      ownerWindow?.removeEventListener('blur', clearInputState);
      clearInputState();
    },
  };
}

function resolveCamera(
  position: InspectionVector,
  target: InspectionVector,
  projection: PerspectiveProjection,
): AshaRendererEditorViewportCamera {
  const resolution = resolveAshaStoredEditorCamera({ position, target, up: [0, 1, 0], projection });
  if (!resolution.ok) {
    throw new TypeError(resolution.diagnostic.message);
  }
  return resolution.camera;
}

function horizontalMovement(
  camera: AshaRendererEditorViewportCamera,
  forwardAxis: number,
  rightAxis: number,
): InspectionVector | null {
  const forward = normalizeHorizontal(camera.basis.forward);
  const right = normalizeHorizontal(camera.basis.right);
  if (forward === null || right === null) {
    return null;
  }
  return normalize([
    forward[0] * forwardAxis + right[0] * rightAxis,
    0,
    forward[2] * forwardAxis + right[2] * rightAxis,
  ]);
}

function positionFromOrbit(
  target: InspectionVector,
  distance: number,
  yawRadians: number,
  pitchRadians: number,
): InspectionVector {
  const horizontalDistance = Math.cos(pitchRadians) * distance;
  return [
    target[0] + Math.sin(yawRadians) * horizontalDistance,
    target[1] + Math.sin(pitchRadians) * distance,
    target[2] + Math.cos(yawRadians) * horizontalDistance,
  ];
}

function browserInspectionEnvironment(): AshaRendererInspectionEnvironment {
  return {
    animation: {
      cancel: (handle) => globalThis.cancelAnimationFrame(handle),
      now: () => globalThis.performance?.now() ?? 0,
      request: (callback) => globalThis.requestAnimationFrame(callback),
    },
    createResizeObserver: (callback) => {
      if (globalThis.ResizeObserver === undefined) {
        return null;
      }
      return new globalThis.ResizeObserver(callback);
    },
    devicePixelRatio: () => globalThis.devicePixelRatio ?? 1,
  };
}

function isMovementKey(code: string): code is (typeof MOVEMENT_KEYS)[number] {
  return MOVEMENT_KEYS.some((movementKey) => movementKey === code);
}

function requirePositiveFinite(value: number, label: string): number {
  if (!Number.isFinite(value) || value <= 0) {
    throw new TypeError(`${label} must be finite and positive`);
  }
  return value;
}

function allFinite(vectors: readonly InspectionVector[]): boolean {
  return vectors.every((vector) => vector.every(Number.isFinite));
}

function vectorLength(vector: InspectionVector): number {
  return Math.hypot(vector[0], vector[1], vector[2]);
}

function normalize(vector: InspectionVector): InspectionVector | null {
  const length = vectorLength(vector);
  return length <= 0.000_001 ? null : scale(vector, 1 / length);
}

function normalizeHorizontal(vector: InspectionVector): InspectionVector | null {
  return normalize([vector[0], 0, vector[2]]);
}

function add(left: InspectionVector, right: InspectionVector): InspectionVector {
  return [left[0] + right[0], left[1] + right[1], left[2] + right[2]];
}

function subtract(left: InspectionVector, right: InspectionVector): InspectionVector {
  return [left[0] - right[0], left[1] - right[1], left[2] - right[2]];
}

function scale(vector: InspectionVector, amount: number): InspectionVector {
  return [vector[0] * amount, vector[1] * amount, vector[2] * amount];
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function degreesToRadians(degrees: number): number {
  return (degrees * Math.PI) / 180;
}
