import type { CameraBasis, EntityId, RenderFrameDiff, RenderHandle, RenderLayer, TagId } from '@asha/contracts';
import { type GeneratedTunnelFrameReadout, type RenderProjectionInstruction, type RenderProjectionSnapshot, type TunnelViewportMaterialPalette } from '@asha/render-projection';
import { type BrowserInputHostReadout, type BrowserInputSessionPort } from '@asha/runtime-bridge';
import { type AshaRendererAnimatedMeshProjection, type AshaRendererAnimatedMeshFrameReceipt, type AshaRendererAnimatedMeshPlaybackReadout, type AshaRendererAnimatedMeshResourceManifest, type AshaRendererAnimatedMeshResourceResolver } from './animated-mesh-host.js';
export declare const ASHA_RENDERER_HOST_COMPATIBILITY_VERSION = "renderer-host.v1";
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
export type AshaRendererSurfaceMovementAuthority = (input: AshaRendererSurfaceMovementAuthorityInput) => AshaRendererSurfaceMovementAuthorityResult;
export interface AshaRendererSurfaceMovementState {
    readonly authority: 'free_camera' | 'external_collision';
    readonly blockedAxes: readonly string[];
    readonly collided: boolean;
    readonly movementHash: string | null;
}
export type AshaRendererSurfaceVec3 = readonly [number, number, number];
export type AshaRendererSurfacePickRay = {
    readonly kind: 'viewport';
    /** Normalized device coordinates, each bounded to [-1, 1]. */
    readonly point: readonly [number, number];
} | {
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
export type AshaRendererSurfacePickDiagnosticCode = 'invalid_viewport_point' | 'invalid_world_ray' | 'invalid_max_distance' | 'filter_limit_exceeded';
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
export declare function createAshaRendererSurfaceProjection(frame: RenderFrameDiff): AshaRendererSurfaceProjectionReceipt;
export declare function createAshaRendererDefaultSurfaceFrame(): RenderFrameDiff;
export declare function createAshaRendererGeneratedTunnelRoomSurfaceFrame(input: AshaRendererGeneratedTunnelRoomSurfaceInput): RenderFrameDiff;
export declare function mountAshaRendererSurface(canvas: HTMLCanvasElement, options?: AshaRendererSurfaceOptions): AshaRendererSurface;
export declare function mountAshaRendererAnimatedMeshSurface(canvas: HTMLCanvasElement, options: AshaRendererAnimatedMeshSurfaceOptions): Promise<AshaRendererSurface>;
//# sourceMappingURL=surface.d.ts.map