import * as THREE from 'three';
import { RenderProjection, type FirstPersonTunnelViewportInput, type FirstPersonTunnelViewportSummary } from '@asha/render-projection';
import { type CameraBasis, type EntityId, type RenderFrameDiff, type RenderHandle, type RenderLayer, type TagId } from '@asha/contracts';
import { ThreeRenderer } from './three-renderer.js';
import type { AnimatedMeshAssetSource, AnimatedMeshPlaybackReadout } from './animated-mesh.js';
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
    readonly animatedMeshSource?: AnimatedMeshAssetSource;
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
export type AshaRendererBrowserSurfacePickRay = {
    readonly kind: 'viewport';
    /** Normalized device coordinates, each bounded to [-1, 1]. */
    readonly point: readonly [number, number];
} | {
    readonly kind: 'world_ray';
    readonly direction: readonly [number, number, number];
    readonly origin: readonly [number, number, number];
};
export interface AshaRendererBrowserSurfacePickFilter {
    readonly handles?: readonly RenderHandle[];
    readonly labels?: readonly string[];
    readonly layers?: readonly RenderLayer[];
    readonly tags?: readonly TagId[];
}
export interface AshaRendererBrowserSurfacePickRequest {
    readonly filter?: AshaRendererBrowserSurfacePickFilter;
    readonly maxDistance?: number;
    readonly ray: AshaRendererBrowserSurfacePickRay;
}
export type AshaRendererBrowserSurfacePickDiagnosticCode = 'invalid_viewport_point' | 'invalid_world_ray' | 'invalid_max_distance' | 'filter_limit_exceeded';
export interface AshaRendererBrowserSurfacePickDiagnostic {
    readonly code: AshaRendererBrowserSurfacePickDiagnosticCode;
    readonly message: string;
}
export interface AshaRendererBrowserSurfacePickHit {
    readonly channel: 'render_projection';
    readonly distance: number;
    readonly handle: RenderHandle;
    readonly label: string | null;
    readonly layer: RenderLayer;
    readonly normal: readonly [number, number, number];
    readonly position: readonly [number, number, number];
    readonly sourceTrace: {
        readonly entity: EntityId;
        readonly kind: 'render_metadata_entity';
    } | null;
    readonly tags: readonly TagId[];
}
export interface AshaRendererBrowserSurfacePickReceipt {
    readonly diagnostics: readonly AshaRendererBrowserSurfacePickDiagnostic[];
    readonly hit: AshaRendererBrowserSurfacePickHit | null;
    readonly kind: 'asha_renderer_browser_surface_pick.v0';
}
export interface AshaRendererBrowserSurface {
    readonly kind: 'asha_renderer_browser_surface.v0';
    readonly canvas: HTMLCanvasElement;
    readonly renderer: ThreeRenderer;
    readonly frame: RenderFrameDiff;
    readonly cameraPose: () => AshaRendererBrowserSurfaceCameraPose;
    readonly animatedMeshPlayback: (handle: import('@asha/contracts').RenderHandle) => AnimatedMeshPlaybackReadout | undefined;
    readonly applyFrame: (frame: RenderFrameDiff) => void;
    readonly pick: (request: AshaRendererBrowserSurfacePickRequest) => AshaRendererBrowserSurfacePickReceipt;
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
export declare function pickProjectedObject(renderer: ThreeRenderer, camera: THREE.PerspectiveCamera, raycaster: THREE.Raycaster, center: THREE.Vector2, request: AshaRendererBrowserSurfacePickRequest): AshaRendererBrowserSurfacePickReceipt;
//# sourceMappingURL=browser-surface.d.ts.map