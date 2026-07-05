import { RenderProjection } from '@asha/render-projection';
import { type CameraBasis, type RenderFrameDiff } from '@asha/contracts';
import { ThreeRenderer } from './three-renderer.js';
import { type FirstPersonTunnelViewportInput, type FirstPersonTunnelViewportSummary } from './tunnel-viewport.js';
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
export type AshaRendererBrowserSurfaceMovementAuthority = (input: AshaRendererBrowserSurfaceMovementAuthorityInput) => AshaRendererBrowserSurfaceMovementAuthorityResult;
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
export declare function renderProjectedFrame(frame: RenderFrameDiff, renderer?: ThreeRenderer): ProjectedThreeRenderResult;
export declare function renderFirstPersonTunnelViewport(input: FirstPersonTunnelViewportInput, renderer?: ThreeRenderer): FirstPersonTunnelViewportRenderResult;
/**
 * A tiny public browser surface for consumers that need to prove the real
 * renderer path: ASHA render diffs -> retained ThreeRenderer -> WebGL canvas.
 *
 * The consumer owns only the canvas element. Three.js scene/camera/WebGL details
 * stay inside `@asha/renderer-three`.
 */
export declare function mountAshaRendererBrowserSurface(canvas: HTMLCanvasElement, options?: AshaRendererBrowserSurfaceOptions): AshaRendererBrowserSurface;
export declare function createAshaRendererBrowserSurfaceFrame(): RenderFrameDiff;
//# sourceMappingURL=browser-surface.d.ts.map