// Browser/canvas surface built on the retained ASHA ThreeRenderer.

import * as THREE from 'three';
import { RenderProjection } from '@asha/render-projection';
import { renderHandle, type CameraBasis, type Geometry, type RenderFrameDiff, type RenderNode, type Transform } from '@asha/contracts';
import type { RenderDiff } from '@asha/contracts';
import { ThreeRenderer } from './three-renderer.js';
import {
  createGeneratedTunnelViewportFrame,
  summarizeFirstPersonTunnelViewport,
  type FirstPersonTunnelViewportInput,
  type FirstPersonTunnelViewportSummary,
  type TunnelViewportMaterialPalette,
  type TunnelViewportVec3,
} from './tunnel-viewport.js';
import type { GeneratedTunnelReadout } from '@asha/runtime-bridge';

export interface ProjectedThreeRenderResult {
  readonly projection: RenderProjection;
  readonly renderer: ThreeRenderer;
  readonly structuralSnapshot: string;
}

export interface FirstPersonTunnelViewportRenderResult extends ProjectedThreeRenderResult {
  readonly frame: RenderFrameDiff;
  readonly summary: FirstPersonTunnelViewportSummary;
}

export interface AshaRendererBrowserSurfaceOptions {
  readonly autoStart?: boolean;
  readonly clearColor?: number;
  readonly controls?: AshaRendererBrowserSurfaceControlsOptions;
  readonly frame?: RenderFrameDiff;
  readonly pixelRatio?: number;
}

export interface AshaRendererBrowserSurfaceControlsOptions {
  readonly enabled?: boolean;
  readonly eyeHeight?: number;
  readonly initialPitchDegrees?: number;
  readonly initialPosition?: readonly [number, number, number];
  readonly initialYawDegrees?: number;
  readonly mouseSensitivity?: number;
  readonly movementAuthority?: AshaRendererBrowserSurfaceMovementAuthority;
  readonly moveSpeed?: number;
}

export interface AshaRendererBrowserSurfaceCameraPose {
  readonly position: readonly [number, number, number];
  readonly pitchDegrees: number;
  readonly yawDegrees: number;
}

export type AshaRendererBrowserSurfaceCameraBasis = CameraBasis;

export interface AshaRendererBrowserSurfaceMovementAuthorityInput {
  readonly dtSeconds: number;
  readonly moveForward: number;
  readonly moveRight: number;
  readonly moveSpeedUnitsPerSecond: number;
  readonly moveUp: number;
  readonly pitchDeltaDegrees: number;
  readonly poseBefore: AshaRendererBrowserSurfaceCameraPose;
  readonly tick: number;
  readonly yawDeltaDegrees: number;
}

export interface AshaRendererBrowserSurfaceMovementAuthorityResult {
  readonly basis?: AshaRendererBrowserSurfaceCameraBasis;
  readonly blockedAxes?: readonly string[];
  readonly collided?: boolean;
  readonly movementHash?: string | null;
  readonly pose: AshaRendererBrowserSurfaceCameraPose;
}

export type AshaRendererBrowserSurfaceMovementAuthority = (
  input: AshaRendererBrowserSurfaceMovementAuthorityInput,
) => AshaRendererBrowserSurfaceMovementAuthorityResult;

export interface AshaRendererBrowserSurfaceMovementState {
  readonly authority: 'free_camera' | 'external_collision';
  readonly blockedAxes: readonly string[];
  readonly collided: boolean;
  readonly movementHash: string | null;
}

export interface AshaRendererBrowserSurfaceFireResult {
  readonly distance: number | null;
  readonly hit: boolean;
  readonly label: string | null;
  readonly remainingTargets: number;
  readonly shotsFired: number;
  readonly targetHealth: number | null;
}

export interface AshaRendererBrowserSurfaceInteractionState {
  readonly hits: number;
  readonly lastEvent: string;
  readonly remainingTargets: number;
  readonly shotsFired: number;
  readonly totalTargets: number;
}

export interface AshaRendererBrowserSurfaceTargetProjection {
  readonly lastEvent?: string;
  readonly visible: boolean;
}

export interface AshaRendererGeneratedTunnelRoomTarget {
  readonly label?: string;
  readonly position: TunnelViewportVec3;
  readonly scale?: TunnelViewportVec3;
}

export interface AshaRendererGeneratedTunnelRoomSurfaceInput {
  readonly enemy?: AshaRendererGeneratedTunnelRoomTarget | null;
  readonly materials?: Partial<TunnelViewportMaterialPalette>;
  readonly tunnel: GeneratedTunnelReadout;
}

export interface AshaRendererBrowserSurface {
  readonly kind: 'asha_renderer_browser_surface.v0';
  readonly canvas: HTMLCanvasElement;
  readonly renderer: ThreeRenderer;
  readonly frame: RenderFrameDiff;
  readonly cameraPose: () => AshaRendererBrowserSurfaceCameraPose;
  readonly firePrimary: () => AshaRendererBrowserSurfaceFireResult;
  readonly interactionState: () => AshaRendererBrowserSurfaceInteractionState;
  readonly lockPointer: () => void;
  readonly movementState: () => AshaRendererBrowserSurfaceMovementState;
  readonly pointerLocked: () => boolean;
  readonly projectTargetProjection: (projection: AshaRendererBrowserSurfaceTargetProjection) => void;
  readonly reset: () => void;
  readonly snapshot: () => string;
  readonly renderOnce: (timeMs?: number) => void;
  readonly start: () => void;
  readonly stop: () => void;
  readonly dispose: () => void;
}

/**
 * Apply a render frame through the renderer-neutral projection and then the
 * retained Three.js renderer. This is the package-root bridge used by demo
 * proofs: no authority state, no raw transport, no arbitrary JSON tunnel.
 */
export function renderProjectedFrame(
  frame: RenderFrameDiff,
  renderer: ThreeRenderer = new ThreeRenderer(),
): ProjectedThreeRenderResult {
  const projection = new RenderProjection();
  projection.applyFrame(frame);
  renderer.applyFrame(frame);
  return {
    projection,
    renderer,
    structuralSnapshot: renderer.snapshot(),
  };
}

export function renderFirstPersonTunnelViewport(
  input: FirstPersonTunnelViewportInput,
  renderer: ThreeRenderer = new ThreeRenderer(),
): FirstPersonTunnelViewportRenderResult {
  const frame = createGeneratedTunnelViewportFrame(input.tunnel, input.materials);
  const rendered = renderProjectedFrame(frame, renderer);
  return {
    ...rendered,
    frame,
    summary: summarizeFirstPersonTunnelViewport({
      tunnel: input.tunnel,
      camera: input.camera,
      frame,
      structuralSnapshot: rendered.structuralSnapshot,
      ...(input.collision === undefined ? {} : { collision: input.collision }),
    }),
  };
}

/**
 * A tiny public browser surface for consumers that need to prove the real
 * renderer path: ASHA render diffs -> retained ThreeRenderer -> WebGL canvas.
 *
 * The consumer owns only the canvas element. Three.js scene/camera/WebGL details
 * stay inside `@asha/renderer-three`.
 */
export function mountAshaRendererBrowserSurface(
  canvas: HTMLCanvasElement,
  options: AshaRendererBrowserSurfaceOptions = {},
): AshaRendererBrowserSurface {
  const renderer = new ThreeRenderer();
  const frame = options.frame ?? createAshaRendererBrowserSurfaceFrame();
  renderer.applyFrame(frame);

  const webgl = new THREE.WebGLRenderer({ canvas, antialias: true });
  webgl.setClearColor(options.clearColor ?? 0x101820, 1);
  webgl.setPixelRatio(options.pixelRatio ?? globalThis.devicePixelRatio ?? 1);

  const camera = new THREE.PerspectiveCamera(55, 1, 0.1, 100);
  const controls = createBrowserSurfaceFirstPersonControls(canvas, camera, options.controls);
  const interactions = createBrowserSurfaceInteractionController(renderer.scene, camera);

  let animationFrame: number | null = null;
  let lastRenderTimeMs: number | null = null;

  const resize = (): void => {
    const width = Math.max(1, canvas.clientWidth || canvas.width || 800);
    const height = Math.max(1, canvas.clientHeight || canvas.height || 450);
    if (canvas.width !== width || canvas.height !== height) {
      webgl.setSize(width, height, false);
    }
    camera.aspect = width / height;
    camera.updateProjectionMatrix();
  };

  const renderOnce = (timeMs = globalThis.performance?.now() ?? 0): void => {
    resize();
    const deltaSeconds =
      lastRenderTimeMs === null
        ? 0
        : Math.min(0.05, Math.max(0, (timeMs - lastRenderTimeMs) / 1000));
    lastRenderTimeMs = timeMs;
    controls.update(deltaSeconds);
    webgl.render(renderer.scene, camera);
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

  const dispose = (): void => {
    stop();
    controls.dispose();
    webgl.dispose();
  };

  const reset = (): void => {
    controls.resetCamera();
    interactions.reset();
    lastRenderTimeMs = null;
    renderOnce(0);
  };

  renderOnce(0);
  if (options.autoStart !== false) {
    start();
  }

  return {
    kind: 'asha_renderer_browser_surface.v0',
    canvas,
    renderer,
    frame,
    cameraPose: () => controls.cameraPose(),
    firePrimary: () => interactions.firePrimary(),
    interactionState: () => interactions.state(),
    lockPointer: () => controls.lockPointer(),
    movementState: () => controls.movementState(),
    pointerLocked: () => controls.pointerLocked(),
    projectTargetProjection: (projection) => interactions.projectTargetProjection(projection),
    reset,
    snapshot: () => renderer.snapshot(),
    renderOnce,
    start,
    stop,
    dispose,
  };
}

export function createAshaRendererBrowserSurfaceFrame(): RenderFrameDiff {
  const cubeSpecs = createBrowserSurfaceCubeSpecs();
  return {
    ops: [
      {
        op: 'create',
        handle: renderHandle(4103001),
        parent: null,
        node: primitiveNode('asha-renderer-flat-plane', 'cube', [0, -0.08, 0], [18, 0.16, 18], [
          0.16,
          0.22,
          0.2,
          1,
        ]),
      },
      {
        op: 'create',
        handle: renderHandle(4103002),
        parent: null,
        node: primitiveNode('asha-renderer-collision-wall-north', 'cube', [0, 0.5, -2.5], [6, 3, 1], [
          0.32,
          0.38,
          0.42,
          1,
        ]),
      },
      {
        op: 'create',
        handle: renderHandle(4103003),
        parent: null,
        node: primitiveNode('asha-renderer-collision-wall-south', 'cube', [0, 0.5, 2.5], [6, 3, 1], [
          0.32,
          0.38,
          0.42,
          1,
        ]),
      },
      {
        op: 'create',
        handle: renderHandle(4103004),
        parent: null,
        node: primitiveNode('asha-renderer-collision-wall-west', 'cube', [-2.5, 0.5, 0], [1, 3, 6], [
          0.27,
          0.34,
          0.37,
          1,
        ]),
      },
      {
        op: 'create',
        handle: renderHandle(4103005),
        parent: null,
        node: primitiveNode('asha-renderer-collision-wall-east', 'cube', [2.5, 0.5, 0], [1, 3, 6], [
          0.27,
          0.34,
          0.37,
          1,
        ]),
      },
      ...cubeSpecs.map((cube, index) => ({
        op: 'create' as const,
        handle: renderHandle(4103100 + index),
        parent: null,
        node: primitiveNode(
          `asha-renderer-random-cube-${String(index + 1).padStart(2, '0')}`,
          'cube',
          [cube.position[0], cube.size[1] / 2, cube.position[1]],
          cube.size,
          cube.color,
        ),
      })),
    ],
  };
}

export function createAshaRendererGeneratedTunnelRoomSurfaceFrame(
  input: AshaRendererGeneratedTunnelRoomSurfaceInput,
): RenderFrameDiff {
  const base = createGeneratedTunnelViewportFrame(input.tunnel, input.materials);
  const centeredBaseOps = base.ops.map((op) => offsetRenderOp(op, [-2.5, 0, -4.5]));
  const enemy = input.enemy ?? {
    label: 'generated-tunnel-enemy',
    position: [0, 1.1, -1.35],
    scale: [0.7, 1.8, 0.7],
  };
  return {
    ops: [
      ...centeredBaseOps,
      ...generatedTunnelRoomDepthCueOps(),
      {
        op: 'create',
        handle: renderHandle(4103901),
        parent: null,
        node: primitiveNode(
          enemy.label ?? 'generated-tunnel-enemy',
          'cube',
          enemy.position,
          enemy.scale ?? [0.7, 1.8, 0.7],
          [0.92, 0.22, 0.18, 1],
        ),
      },
      {
        op: 'create',
        handle: renderHandle(4103902),
        parent: null,
        node: primitiveNode(
          'generated-tunnel-centerline',
          'cube',
          [0, 0.02, -0.4],
          [0.28, 0.04, 4.8],
          [0.94, 0.62, 0.2, 1],
        ),
      },
    ],
  };
}

function generatedTunnelRoomDepthCueOps(): RenderDiff[] {
  const wallRibColor = [0.28, 0.32, 0.36, 1] as const;
  const coverColor = [0.34, 0.38, 0.34, 1] as const;
  const ceilingColor = [0.38, 0.42, 0.47, 1] as const;
  const ribZ = [-3.55, -2.25, -0.95, 0.35] as const;
  const ops: RenderDiff[] = [];
  ribZ.forEach((z, index) => {
    ops.push(
      {
        op: 'create',
        handle: renderHandle(4103910 + index * 2),
        parent: null,
        node: primitiveNode(
          `generated-tunnel-wall-rib-west-${index + 1}`,
          'cube',
          [-2.42, 1.45, z],
          [0.18, 2.9, 0.18],
          wallRibColor,
        ),
      },
      {
        op: 'create',
        handle: renderHandle(4103911 + index * 2),
        parent: null,
        node: primitiveNode(
          `generated-tunnel-wall-rib-east-${index + 1}`,
          'cube',
          [2.42, 1.45, z],
          [0.18, 2.9, 0.18],
          wallRibColor,
        ),
      },
    );
  });
  return [
    ...ops,
    {
      op: 'create',
      handle: renderHandle(4103920),
      parent: null,
      node: primitiveNode(
        'generated-tunnel-low-cover-west',
        'cube',
        [-1.25, 0.24, -1.65],
        [0.72, 0.48, 0.7],
        coverColor,
      ),
    },
    {
      op: 'create',
      handle: renderHandle(4103921),
      parent: null,
      node: primitiveNode(
        'generated-tunnel-low-cover-east',
        'cube',
        [1.25, 0.24, -3.05],
        [0.72, 0.48, 0.7],
        coverColor,
      ),
    },
    {
      op: 'create',
      handle: renderHandle(4103922),
      parent: null,
      node: primitiveNode(
        'generated-tunnel-ceiling-crossbeam',
        'cube',
        [0, 3.08, -2.55],
        [4.75, 0.2, 0.24],
        ceilingColor,
      ),
    },
  ];
}

function offsetRenderOp(op: RenderDiff, offset: TunnelViewportVec3): RenderDiff {
  if (op.op === 'createStaticMeshInstance') {
    return {
      ...op,
      instance: {
        ...op.instance,
        transform: offsetTransform(op.instance.transform, offset),
      },
    };
  }
  if (op.op === 'create') {
    return {
      ...op,
      node: {
        ...op.node,
        transform: offsetTransform(op.node.transform, offset),
      },
    };
  }
  return op;
}

function offsetTransform(transform: Transform, offset: TunnelViewportVec3): Transform {
  return {
    ...transform,
    translation: [
      transform.translation[0] + offset[0],
      transform.translation[1] + offset[1],
      transform.translation[2] + offset[2],
    ],
  };
}

interface BrowserSurfaceFirstPersonControls {
  readonly cameraPose: () => AshaRendererBrowserSurfaceCameraPose;
  readonly dispose: () => void;
  readonly lockPointer: () => void;
  readonly movementState: () => AshaRendererBrowserSurfaceMovementState;
  readonly pointerLocked: () => boolean;
  readonly resetCamera: () => void;
  readonly update: (deltaSeconds: number) => void;
}

function createBrowserSurfaceFirstPersonControls(
  canvas: HTMLCanvasElement,
  camera: THREE.PerspectiveCamera,
  options: AshaRendererBrowserSurfaceControlsOptions | undefined,
): BrowserSurfaceFirstPersonControls {
  const enabled = options?.enabled !== false;
  const ownerDocument = canvas.ownerDocument;
  const moveSpeed = options?.moveSpeed ?? 5.8;
  const mouseSensitivity = options?.mouseSensitivity ?? 0.0021;
  const eyeHeight = options?.eyeHeight ?? 1.62;
  const initialPosition = options?.initialPosition ?? [0, eyeHeight, 8];
  const movementAuthority = options?.movementAuthority;
  const cameraForward = new THREE.Vector3();
  const cameraLookTarget = new THREE.Vector3();
  const cameraRight = new THREE.Vector3();
  const movement = new THREE.Vector3();
  const pressedKeys = new Set<string>();
  let controlTick = 0;
  let lastMovementState: AshaRendererBrowserSurfaceMovementState = {
    authority: movementAuthority === undefined ? 'free_camera' : 'external_collision',
    blockedAxes: [],
    collided: false,
    movementHash: null,
  };
  let pendingPitchDeltaDegrees = 0;
  let pendingYawDeltaDegrees = 0;
  let pointerLocked = false;
  let yawRadians = degreesToRadians(options?.initialYawDegrees ?? 0);
  let pitchRadians = degreesToRadians(options?.initialPitchDegrees ?? 0);
  let authorityBasis: AshaRendererBrowserSurfaceCameraBasis | null = null;

  if (canvas.tabIndex < 0) {
    canvas.tabIndex = 0;
  }
  canvas.style.touchAction = 'none';
  camera.rotation.order = 'YXZ';
  camera.position.set(initialPosition[0], initialPosition[1], initialPosition[2]);

  const applyCameraRotation = (): void => {
    camera.up.set(0, 1, 0);
    camera.rotation.x = pitchRadians;
    camera.rotation.y = yawRadians;
    camera.rotation.z = 0;
  };

  const applyCameraBasis = (basis: AshaRendererBrowserSurfaceCameraBasis): void => {
    camera.up.set(basis.up[0], basis.up[1], basis.up[2]);
    cameraLookTarget.set(
      camera.position.x + basis.forward[0],
      camera.position.y + basis.forward[1],
      camera.position.z + basis.forward[2],
    );
    camera.lookAt(cameraLookTarget);
  };

  const applyCameraOrientation = (): void => {
    if (authorityBasis === null) {
      applyCameraRotation();
      return;
    }
    applyCameraBasis(authorityBasis);
  };

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

  const resetCamera = (): void => {
    pressedKeys.clear();
    pendingPitchDeltaDegrees = 0;
    pendingYawDeltaDegrees = 0;
    controlTick = 0;
    lastMovementState = {
      authority: movementAuthority === undefined ? 'free_camera' : 'external_collision',
      blockedAxes: [],
      collided: false,
      movementHash: null,
    };
    yawRadians = degreesToRadians(options?.initialYawDegrees ?? 0);
    pitchRadians = degreesToRadians(options?.initialPitchDegrees ?? 0);
    authorityBasis = null;
    camera.position.set(initialPosition[0], initialPosition[1], initialPosition[2]);
    applyCameraOrientation();
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
    pitchRadians += pitchDeltaRadians;
    pitchRadians = clamp(pitchRadians, degreesToRadians(-85), degreesToRadians(85));
    authorityBasis = null;
    pendingYawDeltaDegrees += radiansToDegrees(yawDeltaRadians);
    pendingPitchDeltaDegrees += radiansToDegrees(pitchDeltaRadians);
    applyCameraOrientation();
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
    applyCameraOrientation();
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
      applyAuthorityPose(authorityResult.pose, authorityResult.basis);
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

    camera.updateMatrixWorld(true);
    camera.getWorldDirection(cameraForward);
    cameraForward.y = 0;
    if (cameraForward.lengthSq() > 0) {
      cameraForward.normalize();
    }
    cameraRight.setFromMatrixColumn(camera.matrixWorld, 0);
    cameraRight.y = 0;
    if (cameraRight.lengthSq() > 0) {
      cameraRight.normalize();
    }

    movement.set(0, 0, 0);
    movement.addScaledVector(cameraForward, forward);
    movement.addScaledVector(cameraRight, strafe);
    if (movement.lengthSq() === 0) {
      return;
    }
    movement.normalize();
    const step = moveSpeed * deltaSeconds;
    camera.position.addScaledVector(movement, step);
    camera.position.y = eyeHeight;
    lastMovementState = {
      authority: 'free_camera',
      blockedAxes: [],
      collided: false,
      movementHash: null,
    };
  };

  const cameraPose = (): AshaRendererBrowserSurfaceCameraPose => ({
    position: [
      Number(camera.position.x.toFixed(4)),
      Number(camera.position.y.toFixed(4)),
      Number(camera.position.z.toFixed(4)),
    ],
    pitchDegrees: Number(radiansToDegrees(pitchRadians).toFixed(2)),
    yawDegrees: Number(radiansToDegrees(yawRadians).toFixed(2)),
  });

  const applyAuthorityPose = (
    pose: AshaRendererBrowserSurfaceCameraPose,
    basis?: AshaRendererBrowserSurfaceCameraBasis,
  ): void => {
    camera.position.set(pose.position[0], pose.position[1], pose.position[2]);
    yawRadians = degreesToRadians(pose.yawDegrees);
    pitchRadians = degreesToRadians(pose.pitchDegrees);
    authorityBasis = basis ?? null;
    applyCameraOrientation();
  };

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

  applyCameraOrientation();
  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('click', onClick);
  ownerDocument.addEventListener('pointerlockchange', onPointerLockChange);
  ownerDocument.addEventListener('mousemove', onMouseMove);
  ownerDocument.addEventListener('keydown', onKeyDown);
  ownerDocument.addEventListener('keyup', onKeyUp);

  return {
    cameraPose,
    dispose,
    lockPointer: () => requestLock(),
    movementState: () => lastMovementState,
    pointerLocked: () => pointerLocked,
    resetCamera,
    update,
  };
}

interface BrowserSurfaceInteractionController {
  readonly firePrimary: () => AshaRendererBrowserSurfaceFireResult;
  readonly projectTargetProjection: (projection: AshaRendererBrowserSurfaceTargetProjection) => void;
  readonly reset: () => void;
  readonly state: () => AshaRendererBrowserSurfaceInteractionState;
}

interface BrowserSurfaceTarget {
  readonly baseColor: THREE.Color;
  readonly label: string;
  readonly material: THREE.MeshBasicMaterial;
  readonly mesh: THREE.Mesh;
  readonly maxHealth: number;
  health: number;
}

function createBrowserSurfaceInteractionController(
  scene: THREE.Scene,
  camera: THREE.PerspectiveCamera,
): BrowserSurfaceInteractionController {
  const raycaster = new THREE.Raycaster();
  const targets = collectBrowserSurfaceTargets(scene);
  let hits = 0;
  let lastEvent = 'Ready';
  let shotsFired = 0;

  const state = (): AshaRendererBrowserSurfaceInteractionState => ({
    hits,
    lastEvent,
    remainingTargets: targets.filter((target) => target.health > 0).length,
    shotsFired,
    totalTargets: targets.length,
  });

  const firePrimary = (): AshaRendererBrowserSurfaceFireResult => {
    shotsFired += 1;
    scene.updateMatrixWorld(true);
    raycaster.setFromCamera(new THREE.Vector2(0, 0), camera);
    const liveTargets = targets.filter((target) => target.health > 0);
    const intersections = raycaster.intersectObjects(liveTargets.map((target) => target.mesh), false);
    const intersection = intersections[0];

    if (intersection === undefined) {
      lastEvent = 'Miss';
      return {
        distance: null,
        hit: false,
        label: null,
        remainingTargets: state().remainingTargets,
        shotsFired,
        targetHealth: null,
      };
    }

    const target = liveTargets.find((candidate) => candidate.mesh === intersection.object);
    if (target === undefined) {
      lastEvent = 'Miss';
      return {
        distance: null,
        hit: false,
        label: null,
        remainingTargets: state().remainingTargets,
        shotsFired,
        targetHealth: null,
      };
    }

    target.health -= 1;
    hits += 1;
    if (target.health <= 0) {
      target.mesh.visible = false;
      lastEvent = `Destroyed ${target.label}`;
    } else {
      target.material.color.setRGB(1, 0.28, 0.18);
      lastEvent = `Hit ${target.label}`;
    }

    return {
      distance: Number(intersection.distance.toFixed(2)),
      hit: true,
      label: target.label,
      remainingTargets: state().remainingTargets,
      shotsFired,
      targetHealth: Math.max(0, target.health),
    };
  };

  const reset = (): void => {
    hits = 0;
    lastEvent = 'Reset';
    shotsFired = 0;
    for (const target of targets) {
      target.health = target.maxHealth;
      target.mesh.visible = true;
      target.material.color.copy(target.baseColor);
    }
  };

  const projectTargetProjection = (projection: AshaRendererBrowserSurfaceTargetProjection): void => {
    lastEvent = projection.lastEvent ?? lastEvent;
    for (const target of targets) {
      target.mesh.visible = projection.visible;
      target.health = projection.visible ? target.maxHealth : 0;
      if (projection.visible) {
        target.material.color.copy(target.baseColor);
      }
    }
  };

  return {
    firePrimary,
    projectTargetProjection,
    reset,
    state,
  };
}

function collectBrowserSurfaceTargets(scene: THREE.Scene): BrowserSurfaceTarget[] {
  const targets: BrowserSurfaceTarget[] = [];
  scene.traverse((object) => {
    const isCombatTarget = object.name.includes('generated-tunnel-enemy');
    if (!isCombatTarget && !object.name.startsWith('asha-renderer-random-cube-')) {
      return;
    }
    const mesh = object as THREE.Mesh;
    const material = Array.isArray(mesh.material) ? mesh.material[0] : mesh.material;
    if (!(material instanceof THREE.MeshBasicMaterial)) {
      return;
    }
    targets.push({
      baseColor: material.color.clone(),
      health: 2,
      label: object.name.replace('asha-renderer-random-cube-', 'cube '),
      material,
      maxHealth: 2,
      mesh,
    });
  });
  return targets;
}

interface BrowserSurfaceCubeSpec {
  readonly color: readonly [number, number, number, number];
  readonly position: readonly [number, number];
  readonly size: readonly [number, number, number];
}

function createBrowserSurfaceCubeSpecs(): readonly BrowserSurfaceCubeSpec[] {
  const random = deterministicUnitGenerator(0x4103c0de);
  const colors: readonly (readonly [number, number, number, number])[] = [
    [0.28, 0.66, 0.92, 1],
    [0.92, 0.54, 0.32, 1],
    [0.46, 0.78, 0.42, 1],
    [0.82, 0.58, 0.92, 1],
    [0.92, 0.76, 0.28, 1],
  ];
  const cubes: BrowserSurfaceCubeSpec[] = [
    {
      color: colors[0] as readonly [number, number, number, number],
      position: [0, -1.35],
      size: [0.62, 2.2, 0.62],
    },
    {
      color: colors[1] as readonly [number, number, number, number],
      position: [1.25, -0.65],
      size: [0.48, 0.85, 0.48],
    },
    {
      color: colors[2] as readonly [number, number, number, number],
      position: [-1.15, -0.9],
      size: [0.52, 1.05, 0.52],
    },
    {
      color: colors[3] as readonly [number, number, number, number],
      position: [0.85, 1.1],
      size: [0.44, 0.75, 0.44],
    },
  ];
  for (let index = cubes.length; index < 28; index += 1) {
    const width = round2(0.55 + random() * 1.55);
    const height = round2(0.65 + random() * 2.8);
    const depth = round2(0.55 + random() * 1.55);
    let x = round2(-7 + random() * 14);
    let z = round2(-7 + random() * 14);
    if (x > -3.5 && x < 3.5 && z > -3.5 && z < 3.5) {
      z = round2(z < 0 ? z - 3.75 : z + 3.75);
    }
    cubes.push({
      color: colors[index % colors.length] as readonly [number, number, number, number],
      position: [x, z],
      size: [width, height, depth],
    });
  }
  return cubes;
}

function primitiveNode(
  label: string,
  shape: Exclude<Geometry['shape'], 'line'>,
  translation: readonly [number, number, number],
  scale: readonly [number, number, number],
  color: readonly [number, number, number, number],
): RenderNode {
  return {
    geometry: { shape },
    material: { color, wireframe: false },
    transform: identityTransform(translation, scale),
    visible: true,
    layer: 'scene',
    metadata: { source: null, tags: [], label },
  };
}

function identityTransform(
  translation: readonly [number, number, number],
  scale: readonly [number, number, number],
): Transform {
  return {
    translation,
    rotation: [0, 0, 0, 1],
    scale,
  };
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

function deterministicUnitGenerator(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    return state / 0x100000000;
  };
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

// ── Snapshot lines (deterministic golden artifact) ────────────────────────────
