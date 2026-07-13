// Backend-neutral browser render surface host.

import type {
  CameraBasis,
  EntityId,
  RenderFrameDiff,
  RenderHandle,
  RenderLayer,
  TagId,
} from '@asha/contracts';
import {
  createGeneratedTunnelRoomFrame,
  RenderProjection,
  type GeneratedTunnelFrameReadout,
  type RenderProjectionInstruction,
  type RenderProjectionSnapshot,
  type TunnelViewportMaterialPalette,
} from '@asha/render-projection';
import {
  createAshaRendererBrowserSurfaceFrame as createBackendBrowserSurfaceFrame,
  mountAshaRendererBrowserSurface as mountThreeBackedBrowserSurface,
  type AnimatedMeshAssetSource,
} from '@asha/renderer-three/backend';
import {
  BrowserFpsResolvedActionConsumer,
  BrowserInputHost,
  type BrowserInputHostReadout,
  type BrowserInputSessionPort,
} from '@asha/runtime-bridge';
import {
  animationPlaybackReadout,
  loadRendererAnimatedMeshSource,
  type AshaRendererAnimatedMeshProjection,
  type AshaRendererAnimatedMeshFrameReceipt,
  type AshaRendererAnimatedMeshPlaybackReadout,
  type AshaRendererAnimatedMeshResourceManifest,
  type AshaRendererAnimatedMeshResourceResolver,
} from './animated-mesh-host.js';

export const ASHA_RENDERER_HOST_COMPATIBILITY_VERSION = 'renderer-host.v1';

export type AshaRendererBackendFamily = 'threejs';

export interface AshaRendererBackendDiagnostics {
  readonly family: AshaRendererBackendFamily;
  readonly implementation: 'engine-owned-renderer-backend';
  readonly publicContract: 'asha-renderer-surface.v0';
}

export interface AshaRendererSurfaceOptions {
  readonly autoStart?: boolean;
  readonly clearColor?: number;
  readonly controls?: AshaRendererSurfaceControlsOptions;
  readonly frame?: RenderFrameDiff;
  readonly pixelRatio?: number;
}

export interface AshaRendererAnimatedMeshSurfaceOptions extends AshaRendererSurfaceOptions {
  readonly animatedMeshManifest: AshaRendererAnimatedMeshResourceManifest;
  readonly resolveAnimatedMeshResource?: AshaRendererAnimatedMeshResourceResolver;
}

export interface AshaRendererSurfaceControlsOptions {
  readonly enabled?: boolean;
  readonly eyeHeight?: number;
  readonly initialPitchDegrees?: number;
  readonly initialPosition?: readonly [number, number, number];
  readonly initialYawDegrees?: number;
  readonly mouseSensitivity?: number;
  readonly movementAuthority?: AshaRendererSurfaceMovementAuthority;
  readonly moveSpeed?: number;
  /** Public RuntimeSession input surface. Controls stay inactive when omitted. */
  readonly inputSession?: BrowserInputSessionPort;
  readonly initialInputContexts?: readonly string[];
}

export interface AshaRendererSurfaceCameraPose {
  readonly position: readonly [number, number, number];
  readonly pitchDegrees: number;
  readonly yawDegrees: number;
}

export type AshaRendererSurfaceCameraBasis = CameraBasis;

export interface AshaRendererSurfaceMovementAuthorityInput {
  readonly dtSeconds: number;
  readonly moveForward: number;
  readonly moveRight: number;
  readonly moveSpeedUnitsPerSecond: number;
  readonly moveUp: number;
  readonly pitchDeltaDegrees: number;
  readonly poseBefore: AshaRendererSurfaceCameraPose;
  readonly tick: number;
  readonly yawDeltaDegrees: number;
}

export interface AshaRendererSurfaceMovementAuthorityResult {
  readonly basis?: AshaRendererSurfaceCameraBasis;
  readonly blockedAxes?: readonly string[];
  readonly collided?: boolean;
  readonly movementHash?: string | null;
  readonly pose: AshaRendererSurfaceCameraPose;
}

export type AshaRendererSurfaceMovementAuthority = (
  input: AshaRendererSurfaceMovementAuthorityInput,
) => AshaRendererSurfaceMovementAuthorityResult;

export interface AshaRendererSurfaceMovementState {
  readonly authority: 'free_camera' | 'external_collision';
  readonly blockedAxes: readonly string[];
  readonly collided: boolean;
  readonly movementHash: string | null;
}

export type AshaRendererSurfaceVec3 = readonly [number, number, number];

export type AshaRendererSurfacePickRay =
  | {
      readonly kind: 'viewport';
      /** Normalized device coordinates, each bounded to [-1, 1]. */
      readonly point: readonly [number, number];
    }
  | {
      readonly kind: 'world_ray';
      readonly direction: AshaRendererSurfaceVec3;
      readonly origin: AshaRendererSurfaceVec3;
    };

export interface AshaRendererSurfacePickFilter {
  readonly handles?: readonly RenderHandle[];
  readonly labels?: readonly string[];
  readonly layers?: readonly RenderLayer[];
  readonly tags?: readonly TagId[];
}

export interface AshaRendererSurfacePickRequest {
  readonly filter?: AshaRendererSurfacePickFilter;
  readonly maxDistance?: number;
  readonly ray: AshaRendererSurfacePickRay;
}

export type AshaRendererSurfacePickDiagnosticCode =
  | 'invalid_viewport_point'
  | 'invalid_world_ray'
  | 'invalid_max_distance'
  | 'filter_limit_exceeded';

export interface AshaRendererSurfacePickDiagnostic {
  readonly code: AshaRendererSurfacePickDiagnosticCode;
  readonly message: string;
}

/** Disposable projection evidence. This is never a combat or authority receipt. */
export interface AshaRendererSurfacePickHint {
  readonly channel: 'render_projection';
  readonly distance: number;
  readonly handle: RenderHandle;
  readonly label: string | null;
  readonly layer: RenderLayer;
  readonly normal: AshaRendererSurfaceVec3;
  readonly position: AshaRendererSurfaceVec3;
  readonly sourceTrace: {
    readonly entity: EntityId;
    readonly kind: 'render_metadata_entity';
  } | null;
  readonly tags: readonly TagId[];
}

export interface AshaRendererSurfacePickReceipt {
  readonly diagnostics: readonly AshaRendererSurfacePickDiagnostic[];
  readonly hint: AshaRendererSurfacePickHint | null;
  readonly kind: 'asha_renderer_surface_pick.v0';
}

export interface AshaRendererGeneratedTunnelRoomTarget {
  readonly label?: string;
  readonly position: AshaRendererSurfaceVec3;
  readonly scale?: AshaRendererSurfaceVec3;
}

export type AshaRendererGeneratedTunnelMaterialPalette = TunnelViewportMaterialPalette;

export type AshaRendererGeneratedTunnelReadout = GeneratedTunnelFrameReadout;

export interface AshaRendererGeneratedTunnelRoomSurfaceInput {
  readonly enemy?: AshaRendererGeneratedTunnelRoomTarget | null;
  readonly materials?: Partial<AshaRendererGeneratedTunnelMaterialPalette>;
  readonly tunnel: AshaRendererGeneratedTunnelReadout;
}

export interface AshaRendererSurfaceProjectionReceipt {
  readonly instructions: readonly RenderProjectionInstruction[];
  readonly snapshot: RenderProjectionSnapshot;
}

export interface AshaRendererSurface {
  readonly kind: 'asha_renderer_surface.v0';
  readonly backend: AshaRendererBackendDiagnostics;
  readonly canvas: HTMLCanvasElement;
  readonly frame: RenderFrameDiff;
  readonly animatedMeshPlayback: (handle: RenderHandle) => AshaRendererAnimatedMeshPlaybackReadout;
  /** Renderer-only realization port for authority-authored G1 animation state. */
  readonly animationProjection: AshaRendererAnimatedMeshProjection;
  readonly applyFrame: (frame: RenderFrameDiff) => AshaRendererAnimatedMeshFrameReceipt;
  readonly projectionSnapshot: () => RenderProjectionSnapshot;
  readonly cameraPose: () => AshaRendererSurfaceCameraPose;
  readonly pick: (request: AshaRendererSurfacePickRequest) => AshaRendererSurfacePickReceipt;
  readonly lockPointer: () => void;
  readonly movementState: () => AshaRendererSurfaceMovementState;
  readonly pointerLocked: () => boolean;
  readonly inputReadout: () => BrowserInputHostReadout | null;
  readonly resetCamera: () => void;
  readonly snapshot: () => string;
  readonly renderOnce: (timeMs?: number) => void;
  readonly start: () => void;
  readonly stop: () => void;
  readonly dispose: () => void;
}

const THREE_BACKEND_DIAGNOSTICS: AshaRendererBackendDiagnostics = {
  family: 'threejs',
  implementation: 'engine-owned-renderer-backend',
  publicContract: 'asha-renderer-surface.v0',
};

export function createAshaRendererSurfaceProjection(frame: RenderFrameDiff): AshaRendererSurfaceProjectionReceipt {
  const projection = new RenderProjection();
  const instructions = projection.applyFrame(frame);
  return {
    instructions,
    snapshot: projection.snapshot(),
  };
}

export function createAshaRendererDefaultSurfaceFrame(): RenderFrameDiff {
  return createBackendBrowserSurfaceFrame();
}

export function createAshaRendererGeneratedTunnelRoomSurfaceFrame(
  input: AshaRendererGeneratedTunnelRoomSurfaceInput,
): RenderFrameDiff {
  return createGeneratedTunnelRoomFrame({
    ...(input.enemy === undefined ? {} : { enemy: input.enemy }),
    ...(input.materials === undefined ? {} : { materials: input.materials }),
    tunnel: input.tunnel,
  });
}

export function mountAshaRendererSurface(
  canvas: HTMLCanvasElement,
  options: AshaRendererSurfaceOptions = {},
): AshaRendererSurface {
  return mountPreparedAshaRendererSurface(canvas, options);
}

export async function mountAshaRendererAnimatedMeshSurface(
  canvas: HTMLCanvasElement,
  options: AshaRendererAnimatedMeshSurfaceOptions,
): Promise<AshaRendererSurface> {
  const source = await loadRendererAnimatedMeshSource(
    options.animatedMeshManifest,
    options.resolveAnimatedMeshResource,
  );
  return mountPreparedAshaRendererSurface(canvas, options, source as AnimatedMeshAssetSource);
}

function mountPreparedAshaRendererSurface(
  canvas: HTMLCanvasElement,
  options: AshaRendererSurfaceOptions,
  animatedMeshSource?: AnimatedMeshAssetSource,
): AshaRendererSurface {
  const frame = options.frame ?? createAshaRendererDefaultSurfaceFrame();
  const projection = new RenderProjection();
  projection.applyFrame(frame);
  const controls = createAshaRendererSurfaceFirstPersonControls(canvas, options.controls);

  const backendSurface = mountThreeBackedBrowserSurface(canvas, {
    autoStart: false,
    ...(animatedMeshSource === undefined ? {} : { animatedMeshSource }),
    camera: { initialPose: controls.cameraPose() },
    ...(options.clearColor === undefined ? {} : { clearColor: options.clearColor }),
    ...(options.pixelRatio === undefined ? {} : { pixelRatio: options.pixelRatio }),
    frame,
  });

  let animationFrame: number | null = null;
  let lastRenderTimeMs: number | null = null;

  const renderOnce = (timeMs = globalThis.performance?.now() ?? 0): void => {
    const deltaSeconds =
      lastRenderTimeMs === null
        ? 0
        : Math.min(0.05, Math.max(0, (timeMs - lastRenderTimeMs) / 1000));
    lastRenderTimeMs = timeMs;
    controls.update(deltaSeconds);
    const camera = controls.cameraSnapshot();
    backendSurface.setCameraPose(camera.pose, camera.basis ?? undefined);
    backendSurface.renderOnce(timeMs);
  };

  const tick = (timeMs: number): void => {
    renderOnce(timeMs);
    animationFrame = globalThis.requestAnimationFrame(tick);
  };

  const start = (): void => {
    if (animationFrame !== null) {
      return;
    }
    animationFrame = globalThis.requestAnimationFrame(tick);
  };

  const stop = (): void => {
    if (animationFrame === null) {
      return;
    }
    globalThis.cancelAnimationFrame(animationFrame);
    animationFrame = null;
  };

  const resetCamera = (): void => {
    controls.resetCamera();
    lastRenderTimeMs = null;
    renderOnce(0);
  };

  const applyFrame = (nextFrame: RenderFrameDiff): AshaRendererAnimatedMeshFrameReceipt => {
    try {
      backendSurface.applyFrame(nextFrame);
      projection.applyFrame(nextFrame);
      return { applied: true, diagnostics: [] };
    } catch (cause) {
      return {
        applied: false,
        diagnostics: [{
          code: 'animated_mesh_frame_rejected',
          message: cause instanceof Error ? cause.message : String(cause),
          asset: null,
          handle: null,
        }],
      };
    }
  };

  const animationProjection: AshaRendererAnimatedMeshProjection = {
    kind: 'asha_renderer_animated_mesh_projection.v0',
    applyFrame,
    // The mounted browser surface already advances mixer time in its render
    // loop. AshaAnimationHost still calls this port after updating weights, but
    // must not advance the same renderer a second time.
    advance: () => ({ applied: true, diagnostics: [] }),
    playback: (handle) => animationPlaybackReadout(
      handle,
      backendSurface.renderer.animatedMeshPlayback(handle),
    ),
    snapshot: () => backendSurface.renderer.snapshot(),
    hasAnimationTarget: (handle) => backendSurface.renderer.has(handle),
    setAnimationControllerWeights: (handle, clips) => {
      backendSurface.renderer.setAnimationControllerWeights(handle, clips);
    },
    hasAnimationClips: (handle, clipIds) =>
      backendSurface.renderer.hasAnimationControllerClips(handle, clipIds),
    clearAnimationControllerWeights: (handle) => {
      backendSurface.renderer.clearAnimationControllerWeights(handle);
    },
  };

  renderOnce(0);
  if (options.autoStart !== false) {
    start();
  }

  return {
    kind: 'asha_renderer_surface.v0',
    backend: THREE_BACKEND_DIAGNOSTICS,
    canvas,
    frame,
    animationProjection,
    animatedMeshPlayback: (handle) => animationPlaybackReadout(handle, backendSurface.animatedMeshPlayback(handle)),
    applyFrame,
    projectionSnapshot: () => projection.snapshot(),
    cameraPose: () => controls.cameraPose(),
    pick: (request) => {
      const receipt = backendSurface.pick(request);
      return {
        diagnostics: receipt.diagnostics,
        hint: receipt.hit,
        kind: 'asha_renderer_surface_pick.v0',
      };
    },
    lockPointer: () => controls.lockPointer(),
    movementState: () => controls.movementState(),
    pointerLocked: () => controls.pointerLocked(),
    inputReadout: () => controls.inputReadout(),
    resetCamera,
    snapshot: () => backendSurface.snapshot(),
    renderOnce,
    start,
    stop,
    dispose: () => {
      stop();
      controls.dispose();
      backendSurface.dispose();
    },
  };
}

interface AshaRendererSurfaceCameraSnapshot {
  readonly basis?: AshaRendererSurfaceCameraBasis;
  readonly pose: AshaRendererSurfaceCameraPose;
}

interface AshaRendererSurfaceFirstPersonControls {
  readonly cameraPose: () => AshaRendererSurfaceCameraPose;
  readonly cameraSnapshot: () => AshaRendererSurfaceCameraSnapshot;
  readonly dispose: () => void;
  readonly lockPointer: () => void;
  readonly movementState: () => AshaRendererSurfaceMovementState;
  readonly pointerLocked: () => boolean;
  readonly inputReadout: () => BrowserInputHostReadout | null;
  readonly resetCamera: () => void;
  readonly update: (deltaSeconds: number) => void;
}

function createAshaRendererSurfaceFirstPersonControls(
  canvas: HTMLCanvasElement,
  options: AshaRendererSurfaceControlsOptions | undefined,
): AshaRendererSurfaceFirstPersonControls {
  const enabled = options?.enabled !== false && options?.inputSession !== undefined;
  const ownerDocument = canvas.ownerDocument;
  const moveSpeed = options?.moveSpeed ?? 5.8;
  const mouseSensitivity = options?.mouseSensitivity ?? 0.0021;
  const eyeHeight = options?.eyeHeight ?? 1.62;
  const initialPosition = options?.initialPosition ?? [0, eyeHeight, 8];
  const movementAuthority = options?.movementAuthority;
  const actionConsumer = new BrowserFpsResolvedActionConsumer();
  const inputHost = options?.inputSession === undefined
    ? null
    : new BrowserInputHost({
        session: options.inputSession,
        initialContexts: options.initialInputContexts ?? ['gameplay'],
        consumers: {
          'gameplay.move.forward': 'renderer.fpsCamera',
          'gameplay.move.backward': 'renderer.fpsCamera',
          'gameplay.move.left': 'renderer.fpsCamera',
          'gameplay.move.right': 'renderer.fpsCamera',
          'gameplay.look': 'renderer.fpsCamera',
          'gameplay.primaryFire': 'runtime.gameplay',
          'menu.open': 'shell.menu',
          'menu.close': 'shell.menu',
        },
        onResolvedAction: (action) => actionConsumer.accept(action),
        onContextChanged: () => actionConsumer.reset(),
      });
  let authorityBasis: AshaRendererSurfaceCameraBasis | null = null;
  let controlTick = 0;
  let pitchRadians = degreesToRadians(options?.initialPitchDegrees ?? 0);
  let position: AshaRendererSurfaceVec3 = [initialPosition[0], initialPosition[1], initialPosition[2]];
  let yawRadians = degreesToRadians(options?.initialYawDegrees ?? 0);
  let lastMovementState: AshaRendererSurfaceMovementState = {
    authority: movementAuthority === undefined ? 'free_camera' : 'external_collision',
    blockedAxes: [],
    collided: false,
    movementHash: null,
  };

  if (canvas.tabIndex < 0) {
    canvas.tabIndex = 0;
  }
  canvas.style.touchAction = 'none';

  const focusCanvas = (): void => {
    canvas.focus({ preventScroll: true });
  };

  const requestLock = (event?: Event): void => {
    if (!enabled) {
      return;
    }
    event?.preventDefault();
    focusCanvas();
    if (ownerDocument.pointerLockElement !== canvas) {
      void canvas.requestPointerLock();
    }
  };

  const detachInput = inputHost?.attachDom({
    pointerTarget: canvas,
    keyboardTarget: ownerDocument,
    acceptsKeyboard: () => controlsHaveKeyboardFocus(canvas, inputHost.readout().pointerLocked),
    onPointerLockIntent: (intent, event) => {
      if (intent.kind === 'requestPointerLock') requestLock(event);
      else ownerDocument.exitPointerLock();
    },
  });

  const cameraPose = (): AshaRendererSurfaceCameraPose => ({
    position: [round4(position[0]), round4(position[1]), round4(position[2])],
    pitchDegrees: round2(radiansToDegrees(pitchRadians)),
    yawDegrees: round2(radiansToDegrees(yawRadians)),
  });

  const resetCamera = (): void => {
    actionConsumer.reset();
    authorityBasis = null;
    controlTick = 0;
    pitchRadians = degreesToRadians(options?.initialPitchDegrees ?? 0);
    position = [initialPosition[0], initialPosition[1], initialPosition[2]];
    yawRadians = degreesToRadians(options?.initialYawDegrees ?? 0);
    lastMovementState = {
      authority: movementAuthority === undefined ? 'free_camera' : 'external_collision',
      blockedAxes: [],
      collided: false,
      movementHash: null,
    };
  };

  const onPointerLockChange = (): void => {
    const pointerLocked = ownerDocument.pointerLockElement === canvas;
    inputHost?.setPointerLockActive(pointerLocked);
    if (!pointerLocked) {
      actionConsumer.reset();
    }
  };

  const update = (deltaSeconds: number): void => {
    if (!enabled || inputHost === null || !controlsHaveKeyboardFocus(canvas, inputHost.readout().pointerLocked)) {
      return;
    }

    const frame = actionConsumer.drain();
    const safeDeltaSeconds = Math.max(0, deltaSeconds);
    const forward = frame.moveForward;
    const strafe = frame.moveRight;
    const yawDeltaDegrees = frame.yawDeltaPixels * radiansToDegrees(mouseSensitivity);
    const pitchDeltaDegrees = -frame.pitchDeltaPixels * radiansToDegrees(mouseSensitivity);
    const hasLookDelta = yawDeltaDegrees !== 0 || pitchDeltaDegrees !== 0;
    if (forward === 0 && strafe === 0 && !hasLookDelta) {
      return;
    }

    if (movementAuthority !== undefined) {
      controlTick += 1;
      const authorityResult = movementAuthority({
        dtSeconds: safeDeltaSeconds,
        moveForward: forward,
        moveRight: strafe,
        moveSpeedUnitsPerSecond: moveSpeed,
        moveUp: 0,
        pitchDeltaDegrees,
        poseBefore: cameraPose(),
        tick: controlTick,
        yawDeltaDegrees,
      });
      position = authorityResult.pose.position;
      yawRadians = degreesToRadians(authorityResult.pose.yawDegrees);
      pitchRadians = degreesToRadians(authorityResult.pose.pitchDegrees);
      authorityBasis = authorityResult.basis ?? null;
      lastMovementState = {
        authority: 'external_collision',
        blockedAxes: authorityResult.blockedAxes ?? [],
        collided: authorityResult.collided ?? false,
        movementHash: authorityResult.movementHash ?? null,
      };
      return;
    }

    yawRadians += degreesToRadians(yawDeltaDegrees);
    pitchRadians = clamp(
      pitchRadians + degreesToRadians(pitchDeltaDegrees),
      degreesToRadians(-85),
      degreesToRadians(85),
    );
    authorityBasis = null;
    if (safeDeltaSeconds <= 0) {
      return;
    }

    const movement = calculateCameraRelativeMovement(yawRadians, forward, strafe);
    if (movement === null) {
      return;
    }
    const step = moveSpeed * safeDeltaSeconds;
    position = [position[0] + movement[0] * step, eyeHeight, position[2] + movement[2] * step];
    lastMovementState = {
      authority: 'free_camera',
      blockedAxes: [],
      collided: false,
      movementHash: null,
    };
  };

  const cameraSnapshot = (): AshaRendererSurfaceCameraSnapshot => ({
    ...(authorityBasis === null ? {} : { basis: authorityBasis }),
    pose: cameraPose(),
  });

  const dispose = (): void => {
    ownerDocument.removeEventListener('pointerlockchange', onPointerLockChange);
    detachInput?.();
    if (ownerDocument.pointerLockElement === canvas) {
      ownerDocument.exitPointerLock();
    }
  };

  ownerDocument.addEventListener('pointerlockchange', onPointerLockChange);

  return {
    cameraPose,
    cameraSnapshot,
    dispose,
    lockPointer: () => requestLock(),
    movementState: () => lastMovementState,
    pointerLocked: () => inputHost?.readout().pointerLocked ?? false,
    inputReadout: () => inputHost?.readout() ?? null,
    resetCamera,
    update,
  };
}

function calculateCameraRelativeMovement(
  yawRadians: number,
  forwardAxis: number,
  strafeAxis: number,
): AshaRendererSurfaceVec3 | null {
  const forward: AshaRendererSurfaceVec3 = [-Math.sin(yawRadians), 0, -Math.cos(yawRadians)];
  const right: AshaRendererSurfaceVec3 = [Math.cos(yawRadians), 0, -Math.sin(yawRadians)];
  const movement: AshaRendererSurfaceVec3 = [
    forward[0] * forwardAxis + right[0] * strafeAxis,
    0,
    forward[2] * forwardAxis + right[2] * strafeAxis,
  ];
  const length = Math.hypot(movement[0], movement[2]);
  if (length === 0) {
    return null;
  }
  return [movement[0] / length, 0, movement[2] / length];
}

function controlsHaveKeyboardFocus(canvas: HTMLCanvasElement, pointerLocked: boolean): boolean {
  return pointerLocked || canvas.ownerDocument.activeElement === canvas;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function degreesToRadians(degrees: number): number {
  return (degrees * Math.PI) / 180;
}

function radiansToDegrees(radians: number): number {
  return (radians * 180) / Math.PI;
}

function round2(value: number): number {
  return Number(value.toFixed(2));
}

function round4(value: number): number {
  return Number(value.toFixed(4));
}
