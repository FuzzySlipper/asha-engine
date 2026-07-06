import { RenderProjection } from '@asha/render-projection';
import { type CameraBasis, type RenderFrameDiff } from '@asha/contracts';
import { ThreeRenderer } from './three-renderer.js';
import { type FirstPersonTunnelViewportInput, type FirstPersonTunnelViewportSummary, type TunnelViewportMaterialPalette, type TunnelViewportVec3 } from './tunnel-viewport.js';
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
    readonly camera?: AshaRendererBrowserSurfaceCameraOptions;
    readonly clearColor?: number;
    readonly frame?: RenderFrameDiff;
    readonly pixelRatio?: number;
}
export interface AshaRendererBrowserSurfaceCameraPose {
    readonly position: readonly [number, number, number];
    readonly pitchDegrees: number;
    readonly yawDegrees: number;
}
export type AshaRendererBrowserSurfaceCameraBasis = CameraBasis;
export interface AshaRendererBrowserSurfaceCameraOptions {
    readonly initialBasis?: AshaRendererBrowserSurfaceCameraBasis;
    readonly initialPose?: AshaRendererBrowserSurfaceCameraPose;
}
export interface AshaRendererBrowserSurfaceObjectProjection {
    readonly color?: readonly [number, number, number, number];
    readonly label: string;
    readonly lastEvent?: string;
    readonly position?: TunnelViewportVec3;
    readonly scale?: TunnelViewportVec3;
    readonly visible: boolean;
}
export interface AshaRendererBrowserSurfacePickRequest {
    readonly labels: readonly string[];
}
export interface AshaRendererBrowserSurfacePickResult {
    readonly distance: number;
    readonly label: string;
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
    readonly pickCenterObject: (request: AshaRendererBrowserSurfacePickRequest) => AshaRendererBrowserSurfacePickResult | null;
    readonly projectObjectProjection: (projection: AshaRendererBrowserSurfaceObjectProjection) => void;
    readonly snapshot: () => string;
    readonly renderOnce: (timeMs?: number) => void;
    readonly setCameraPose: (pose: AshaRendererBrowserSurfaceCameraPose, basis?: AshaRendererBrowserSurfaceCameraBasis) => void;
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
export declare function createAshaRendererGeneratedTunnelRoomSurfaceFrame(input: AshaRendererGeneratedTunnelRoomSurfaceInput): RenderFrameDiff;
//# sourceMappingURL=browser-surface.d.ts.map