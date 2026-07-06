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
  AshaRendererBrowserSurfaceControlsOptions as BackendControlsOptions,
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

  const backendSurface = mountThreeBackedBrowserSurface(canvas, {
    ...(options.autoStart === undefined ? {} : { autoStart: options.autoStart }),
    ...(options.clearColor === undefined ? {} : { clearColor: options.clearColor }),
    ...(options.pixelRatio === undefined ? {} : { pixelRatio: options.pixelRatio }),
    ...(options.controls === undefined ? {} : { controls: toBackendControls(options.controls) }),
    frame,
  });

  return {
    kind: 'asha_renderer_surface.v0',
    backend: THREE_BACKEND_DIAGNOSTICS,
    canvas,
    frame,
    projectionSnapshot: () => projection.snapshot(),
    cameraPose: () => backendSurface.cameraPose(),
    firePrimary: () => backendSurface.firePrimary(),
    interactionState: () => backendSurface.interactionState(),
    lockPointer: () => backendSurface.lockPointer(),
    movementState: () => backendSurface.movementState(),
    pointerLocked: () => backendSurface.pointerLocked(),
    projectTargetProjection: (targetProjection) => backendSurface.projectTargetProjection(targetProjection),
    reset: () => backendSurface.reset(),
    snapshot: () => backendSurface.snapshot(),
    renderOnce: (timeMs) => backendSurface.renderOnce(timeMs),
    start: () => backendSurface.start(),
    stop: () => backendSurface.stop(),
    dispose: () => backendSurface.dispose(),
  };
}

function toBackendControls(options: AshaRendererSurfaceControlsOptions): BackendControlsOptions {
  const movementAuthority = options.movementAuthority;
  return {
    ...(options.enabled === undefined ? {} : { enabled: options.enabled }),
    ...(options.eyeHeight === undefined ? {} : { eyeHeight: options.eyeHeight }),
    ...(options.initialPitchDegrees === undefined ? {} : { initialPitchDegrees: options.initialPitchDegrees }),
    ...(options.initialPosition === undefined ? {} : { initialPosition: options.initialPosition }),
    ...(options.initialYawDegrees === undefined ? {} : { initialYawDegrees: options.initialYawDegrees }),
    ...(options.mouseSensitivity === undefined ? {} : { mouseSensitivity: options.mouseSensitivity }),
    ...(options.moveSpeed === undefined ? {} : { moveSpeed: options.moveSpeed }),
    ...(movementAuthority === undefined
      ? {}
      : { movementAuthority: (input) => movementAuthority(input) }),
  };
}
