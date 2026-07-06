// Backend-neutral browser render surface host.

import type { CameraBasis, RenderFrameDiff } from '@asha/contracts';
import {
  RenderProjection,
  type RenderProjectionInstruction,
  type RenderProjectionSnapshot,
} from '@asha/render-projection';
import {
  createAshaRendererBrowserSurfaceFrame as createBackendBrowserSurfaceFrame,
  createAshaRendererGeneratedTunnelRoomSurfaceFrame as createBackendGeneratedTunnelRoomSurfaceFrame,
  mountAshaRendererBrowserSurface as mountThreeBackedBrowserSurface,
} from '@asha/renderer-three/backend';
import type {
  AshaRendererGeneratedTunnelRoomSurfaceInput as BackendGeneratedTunnelRoomSurfaceInput,
} from '@asha/renderer-three/backend';

export const ASHA_RENDERER_HOST_COMPATIBILITY_VERSION = 'renderer-host.v0';

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

export interface AshaRendererSurfaceControlsOptions {
  readonly enabled?: boolean;
  readonly eyeHeight?: number;
  readonly initialPitchDegrees?: number;
  readonly initialPosition?: readonly [number, number, number];
  readonly initialYawDegrees?: number;
  readonly mouseSensitivity?: number;
  readonly movementAuthority?: AshaRendererSurfaceMovementAuthority;
  readonly moveSpeed?: number;
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

export interface AshaRendererSurfaceFireResult {
  readonly distance: number | null;
  readonly hit: boolean;
  readonly label: string | null;
  readonly remainingTargets: number;
  readonly shotsFired: number;
  readonly targetHealth: number | null;
}

export interface AshaRendererSurfaceInteractionState {
  readonly hits: number;
  readonly lastEvent: string;
  readonly remainingTargets: number;
  readonly shotsFired: number;
  readonly totalTargets: number;
}

export type AshaRendererSurfaceVec3 = readonly [number, number, number];

export interface AshaRendererSurfaceTargetProjection {
  readonly lastEvent?: string;
  readonly position?: AshaRendererSurfaceVec3;
  readonly scale?: AshaRendererSurfaceVec3;
  readonly visible: boolean;
}

export interface AshaRendererGeneratedTunnelRoomTarget {
  readonly label?: string;
  readonly position: AshaRendererSurfaceVec3;
  readonly scale?: AshaRendererSurfaceVec3;
}

export type AshaRendererSurfaceColor = readonly [number, number, number, number];

export interface AshaRendererGeneratedTunnelMaterialPalette {
  readonly accent: AshaRendererSurfaceColor;
  readonly exitMarker: AshaRendererSurfaceColor;
  readonly floor: AshaRendererSurfaceColor;
  readonly playerMarker: AshaRendererSurfaceColor;
  readonly wall: AshaRendererSurfaceColor;
}

export interface AshaRendererGeneratedTunnelReadout {
  readonly volume: {
    readonly tunnelDims: readonly [number, number, number];
  };
  readonly spawnMarkers: readonly AshaRendererGeneratedTunnelSpawnMarker[];
}

export interface AshaRendererGeneratedTunnelSpawnMarker {
  readonly id: string;
  readonly kind: 'player' | 'exit';
  readonly world: AshaRendererSurfaceVec3;
}

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
  readonly projectionSnapshot: () => RenderProjectionSnapshot;
  readonly cameraPose: () => AshaRendererSurfaceCameraPose;
  readonly firePrimary: () => AshaRendererSurfaceFireResult;
  readonly interactionState: () => AshaRendererSurfaceInteractionState;
  readonly lockPointer: () => void;
  readonly movementState: () => AshaRendererSurfaceMovementState;
  readonly pointerLocked: () => boolean;
  readonly projectTargetProjection: (projection: AshaRendererSurfaceTargetProjection) => void;
  readonly reset: () => void;
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
  return createBackendGeneratedTunnelRoomSurfaceFrame({
    ...(input.enemy === undefined ? {} : { enemy: input.enemy }),
    ...(input.materials === undefined ? {} : { materials: input.materials }),
    tunnel: input.tunnel as BackendGeneratedTunnelRoomSurfaceInput['tunnel'],
  });
}

export function mountAshaRendererSurface(
  canvas: HTMLCanvasElement,
  options: AshaRendererSurfaceOptions = {},
): AshaRendererSurface {
  const frame = options.frame ?? createAshaRendererDefaultSurfaceFrame();
  const projection = new RenderProjection();
  projection.applyFrame(frame);
  const controls = createAshaRendererSurfaceFirstPersonControls(canvas, options.controls);
  const interactions = createAshaRendererSurfaceInteractionController(frame);

  const backendSurface = mountThreeBackedBrowserSurface(canvas, {
    autoStart: false,
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

  const reset = (): void => {
    controls.resetCamera();
    interactions.reset((projectionUpdate) => backendSurface.projectObjectProjection(projectionUpdate));
    lastRenderTimeMs = null;
    renderOnce(0);
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
    projectionSnapshot: () => projection.snapshot(),
    cameraPose: () => controls.cameraPose(),
    firePrimary: () =>
      interactions.firePrimary(
        (labels) => backendSurface.pickCenterObject({ labels }),
        (projectionUpdate) => backendSurface.projectObjectProjection(projectionUpdate),
      ),
    interactionState: () => interactions.state(),
    lockPointer: () => controls.lockPointer(),
    movementState: () => controls.movementState(),
    pointerLocked: () => controls.pointerLocked(),
    projectTargetProjection: (targetProjection) =>
      interactions.projectTargetProjection(targetProjection, (projectionUpdate) =>
        backendSurface.projectObjectProjection(projectionUpdate),
      ),
    reset,
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
  readonly resetCamera: () => void;
  readonly update: (deltaSeconds: number) => void;
}

function createAshaRendererSurfaceFirstPersonControls(
  canvas: HTMLCanvasElement,
  options: AshaRendererSurfaceControlsOptions | undefined,
): AshaRendererSurfaceFirstPersonControls {
  const enabled = options?.enabled !== false;
  const ownerDocument = canvas.ownerDocument;
  const moveSpeed = options?.moveSpeed ?? 5.8;
  const mouseSensitivity = options?.mouseSensitivity ?? 0.0021;
  const eyeHeight = options?.eyeHeight ?? 1.62;
  const initialPosition = options?.initialPosition ?? [0, eyeHeight, 8];
  const movementAuthority = options?.movementAuthority;
  const pressedKeys = new Set<string>();
  let authorityBasis: AshaRendererSurfaceCameraBasis | null = null;
  let controlTick = 0;
  let pendingPitchDeltaDegrees = 0;
  let pendingYawDeltaDegrees = 0;
  let pitchRadians = degreesToRadians(options?.initialPitchDegrees ?? 0);
  let pointerLocked = false;
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

  const cameraPose = (): AshaRendererSurfaceCameraPose => ({
    position: [round4(position[0]), round4(position[1]), round4(position[2])],
    pitchDegrees: round2(radiansToDegrees(pitchRadians)),
    yawDegrees: round2(radiansToDegrees(yawRadians)),
  });

  const resetCamera = (): void => {
    pressedKeys.clear();
    authorityBasis = null;
    controlTick = 0;
    pendingPitchDeltaDegrees = 0;
    pendingYawDeltaDegrees = 0;
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
    pointerLocked = ownerDocument.pointerLockElement === canvas;
    if (!pointerLocked) {
      pressedKeys.clear();
    }
  };

  const onPointerDown = (event: PointerEvent): void => {
    if (event.button === 0) {
      requestLock(event);
    }
  };

  const onClick = (event: MouseEvent): void => {
    requestLock(event);
  };

  const onMouseMove = (event: MouseEvent): void => {
    if (!pointerLocked) {
      return;
    }
    const yawDeltaRadians = event.movementX * mouseSensitivity;
    const pitchDeltaRadians = -event.movementY * mouseSensitivity;
    yawRadians += yawDeltaRadians;
    pitchRadians = clamp(pitchRadians + pitchDeltaRadians, degreesToRadians(-85), degreesToRadians(85));
    authorityBasis = null;
    pendingYawDeltaDegrees += radiansToDegrees(yawDeltaRadians);
    pendingPitchDeltaDegrees += radiansToDegrees(pitchDeltaRadians);
  };

  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.key === 'Escape') {
      ownerDocument.exitPointerLock();
      pressedKeys.clear();
      return;
    }
    if (!controlsHaveKeyboardFocus(canvas, pointerLocked) || !isFirstPersonMovementKey(event.code)) {
      return;
    }
    event.preventDefault();
    pressedKeys.add(event.code);
  };

  const onKeyUp = (event: KeyboardEvent): void => {
    if (!isFirstPersonMovementKey(event.code)) {
      return;
    }
    event.preventDefault();
    pressedKeys.delete(event.code);
  };

  const update = (deltaSeconds: number): void => {
    if (!enabled || !controlsHaveKeyboardFocus(canvas, pointerLocked)) {
      return;
    }

    const forward = movementAxis(pressedKeys, 'KeyW', 'ArrowUp', 'KeyS', 'ArrowDown');
    const strafe = movementAxis(pressedKeys, 'KeyD', 'ArrowRight', 'KeyA', 'ArrowLeft');
    const hasLookDelta = pendingYawDeltaDegrees !== 0 || pendingPitchDeltaDegrees !== 0;
    if (forward === 0 && strafe === 0 && !hasLookDelta) {
      return;
    }

    if (movementAuthority !== undefined) {
      controlTick += 1;
      const authorityResult = movementAuthority({
        dtSeconds: Math.max(0, deltaSeconds),
        moveForward: forward,
        moveRight: strafe,
        moveSpeedUnitsPerSecond: moveSpeed,
        moveUp: 0,
        pitchDeltaDegrees: pendingPitchDeltaDegrees,
        poseBefore: cameraPose(),
        tick: controlTick,
        yawDeltaDegrees: pendingYawDeltaDegrees,
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
      pendingPitchDeltaDegrees = 0;
      pendingYawDeltaDegrees = 0;
      return;
    }

    pendingPitchDeltaDegrees = 0;
    pendingYawDeltaDegrees = 0;
    if (deltaSeconds <= 0) {
      return;
    }

    const movement = calculateCameraRelativeMovement(yawRadians, forward, strafe);
    if (movement === null) {
      return;
    }
    const step = moveSpeed * deltaSeconds;
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
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('click', onClick);
    ownerDocument.removeEventListener('pointerlockchange', onPointerLockChange);
    ownerDocument.removeEventListener('mousemove', onMouseMove);
    ownerDocument.removeEventListener('keydown', onKeyDown);
    ownerDocument.removeEventListener('keyup', onKeyUp);
    if (ownerDocument.pointerLockElement === canvas) {
      ownerDocument.exitPointerLock();
    }
  };

  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('click', onClick);
  ownerDocument.addEventListener('pointerlockchange', onPointerLockChange);
  ownerDocument.addEventListener('mousemove', onMouseMove);
  ownerDocument.addEventListener('keydown', onKeyDown);
  ownerDocument.addEventListener('keyup', onKeyUp);

  return {
    cameraPose,
    cameraSnapshot,
    dispose,
    lockPointer: () => requestLock(),
    movementState: () => lastMovementState,
    pointerLocked: () => pointerLocked,
    resetCamera,
    update,
  };
}

interface AshaRendererSurfaceBackendObjectProjection {
  readonly color?: AshaRendererSurfaceColor;
  readonly label: string;
  readonly position?: AshaRendererSurfaceVec3;
  readonly scale?: AshaRendererSurfaceVec3;
  readonly visible: boolean;
}

interface AshaRendererSurfaceBackendPickResult {
  readonly distance: number;
  readonly label: string;
}

interface AshaRendererSurfaceInteractionController {
  readonly firePrimary: (
    pickCenterObject: (labels: readonly string[]) => AshaRendererSurfaceBackendPickResult | null,
    projectObject: (projection: AshaRendererSurfaceBackendObjectProjection) => void,
  ) => AshaRendererSurfaceFireResult;
  readonly projectTargetProjection: (
    projection: AshaRendererSurfaceTargetProjection,
    projectObject: (projection: AshaRendererSurfaceBackendObjectProjection) => void,
  ) => void;
  readonly reset: (projectObject: (projection: AshaRendererSurfaceBackendObjectProjection) => void) => void;
  readonly state: () => AshaRendererSurfaceInteractionState;
}

interface AshaRendererSurfaceTargetState {
  readonly label: string;
  readonly maxHealth: number;
  health: number;
}

function createAshaRendererSurfaceInteractionController(
  frame: RenderFrameDiff,
): AshaRendererSurfaceInteractionController {
  const targets = collectAshaRendererSurfaceTargets(frame);
  let hits = 0;
  let lastEvent = 'Ready';
  let shotsFired = 0;

  const state = (): AshaRendererSurfaceInteractionState => ({
    hits,
    lastEvent,
    remainingTargets: targets.filter((target) => target.health > 0).length,
    shotsFired,
    totalTargets: targets.length,
  });

  const firePrimary = (
    pickCenterObject: (labels: readonly string[]) => AshaRendererSurfaceBackendPickResult | null,
    projectObject: (projection: AshaRendererSurfaceBackendObjectProjection) => void,
  ): AshaRendererSurfaceFireResult => {
    shotsFired += 1;
    const liveTargets = targets.filter((target) => target.health > 0);
    const picked = pickCenterObject(liveTargets.map((target) => target.label));
    if (picked === null) {
      lastEvent = 'Miss';
      return missFireResult(state().remainingTargets, shotsFired);
    }
    const target = liveTargets.find((candidate) => candidate.label === picked.label);
    if (target === undefined) {
      lastEvent = 'Miss';
      return missFireResult(state().remainingTargets, shotsFired);
    }

    target.health -= 1;
    hits += 1;
    if (target.health <= 0) {
      lastEvent = `Destroyed ${displayTargetLabel(target.label)}`;
      projectObject({ label: target.label, visible: false });
    } else {
      lastEvent = `Hit ${displayTargetLabel(target.label)}`;
      projectObject({ color: [1, 0.28, 0.18, 1], label: target.label, visible: true });
    }

    return {
      distance: picked.distance,
      hit: true,
      label: displayTargetLabel(target.label),
      remainingTargets: state().remainingTargets,
      shotsFired,
      targetHealth: Math.max(0, target.health),
    };
  };

  const reset = (projectObject: (projection: AshaRendererSurfaceBackendObjectProjection) => void): void => {
    hits = 0;
    lastEvent = 'Reset';
    shotsFired = 0;
    for (const target of targets) {
      target.health = target.maxHealth;
      projectObject({ label: target.label, visible: true });
    }
  };

  const projectTargetProjection = (
    projection: AshaRendererSurfaceTargetProjection,
    projectObject: (projection: AshaRendererSurfaceBackendObjectProjection) => void,
  ): void => {
    lastEvent = projection.lastEvent ?? lastEvent;
    for (const target of targets) {
      target.health = projection.visible ? target.maxHealth : 0;
      projectObject({
        label: target.label,
        ...(projection.position === undefined ? {} : { position: projection.position }),
        ...(projection.scale === undefined ? {} : { scale: projection.scale }),
        visible: projection.visible,
      });
    }
  };

  return {
    firePrimary,
    projectTargetProjection,
    reset,
    state,
  };
}

function collectAshaRendererSurfaceTargets(frame: RenderFrameDiff): AshaRendererSurfaceTargetState[] {
  const targets: AshaRendererSurfaceTargetState[] = [];
  for (const op of frame.ops) {
    if (op.op !== 'create') {
      continue;
    }
    const label = op.node.metadata.label;
    if (label === null || label === undefined || !isAshaRendererSurfaceTargetLabel(label)) {
      continue;
    }
    targets.push({ health: 2, label, maxHealth: 2 });
  }
  return targets;
}

function isAshaRendererSurfaceTargetLabel(label: string): boolean {
  return label.includes('generated-tunnel-enemy') || label.startsWith('asha-renderer-random-cube-');
}

function displayTargetLabel(label: string): string {
  return label.replace('asha-renderer-random-cube-', 'cube ');
}

function missFireResult(remainingTargets: number, shotsFired: number): AshaRendererSurfaceFireResult {
  return {
    distance: null,
    hit: false,
    label: null,
    remainingTargets,
    shotsFired,
    targetHealth: null,
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

function isFirstPersonMovementKey(code: string): boolean {
  return (
    code === 'KeyW' ||
    code === 'KeyA' ||
    code === 'KeyS' ||
    code === 'KeyD' ||
    code === 'ArrowUp' ||
    code === 'ArrowDown' ||
    code === 'ArrowLeft' ||
    code === 'ArrowRight'
  );
}

function movementAxis(
  keys: ReadonlySet<string>,
  positivePrimary: string,
  positiveSecondary: string,
  negativePrimary: string,
  negativeSecondary: string,
): number {
  const positive = keys.has(positivePrimary) || keys.has(positiveSecondary) ? 1 : 0;
  const negative = keys.has(negativePrimary) || keys.has(negativeSecondary) ? 1 : 0;
  return positive - negative;
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
