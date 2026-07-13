import type { CameraBasis, RenderFrameDiff, RenderHandle } from '@asha/contracts';
import { type GeneratedTunnelFrameReadout, type RenderProjectionInstruction, type RenderProjectionSnapshot, type TunnelViewportMaterialPalette } from '@asha/render-projection';
import { type BrowserInputHostReadout, type BrowserInputSessionPort } from '@asha/runtime-bridge';
import { type AshaRendererAnimatedMeshProjection, type AshaRendererAnimatedMeshFrameReceipt, type AshaRendererAnimatedMeshPlaybackReadout, type AshaRendererAnimatedMeshResourceManifest, type AshaRendererAnimatedMeshResourceResolver } from './animated-mesh-host.js';
export declare const ASHA_RENDERER_HOST_COMPATIBILITY_VERSION = "renderer-host.v0";
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
export interface AshaRendererSurfaceRenderTargetIdentity {
    readonly kind: 'runtime_session.ecrp_render_target.v0';
    readonly renderLabel: string;
    readonly position: AshaRendererSurfaceVec3;
    readonly scale: AshaRendererSurfaceVec3 | null;
    readonly visible: boolean;
}
export interface AshaRendererGeneratedTunnelRoomTarget {
    readonly label?: string;
    readonly position: AshaRendererSurfaceVec3;
    readonly scale?: AshaRendererSurfaceVec3;
}
export type AshaRendererSurfaceColor = readonly [number, number, number, number];
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
    readonly firePrimary: () => AshaRendererSurfaceFireResult;
    readonly interactionState: () => AshaRendererSurfaceInteractionState;
    readonly lockPointer: () => void;
    readonly movementState: () => AshaRendererSurfaceMovementState;
    readonly pointerLocked: () => boolean;
    readonly inputReadout: () => BrowserInputHostReadout | null;
    readonly projectRenderTargetProjection: (target: AshaRendererSurfaceRenderTargetIdentity, options?: {
        readonly lastEvent?: string;
    }) => void;
    readonly projectTargetProjection: (projection: AshaRendererSurfaceTargetProjection) => void;
    readonly reset: () => void;
    readonly snapshot: () => string;
    readonly renderOnce: (timeMs?: number) => void;
    readonly start: () => void;
    readonly stop: () => void;
    readonly dispose: () => void;
}
export declare function createAshaRendererSurfaceProjection(frame: RenderFrameDiff): AshaRendererSurfaceProjectionReceipt;
export declare function createAshaRendererDefaultSurfaceFrame(): RenderFrameDiff;
export declare function createAshaRendererGeneratedTunnelRoomSurfaceFrame(input: AshaRendererGeneratedTunnelRoomSurfaceInput): RenderFrameDiff;
export declare function surfaceTargetProjectionFromRenderTarget(target: AshaRendererSurfaceRenderTargetIdentity, options?: {
    readonly lastEvent?: string;
}): AshaRendererSurfaceTargetProjection & {
    readonly label: string;
};
export declare function mountAshaRendererSurface(canvas: HTMLCanvasElement, options?: AshaRendererSurfaceOptions): AshaRendererSurface;
export declare function mountAshaRendererAnimatedMeshSurface(canvas: HTMLCanvasElement, options: AshaRendererAnimatedMeshSurfaceOptions): Promise<AshaRendererSurface>;
//# sourceMappingURL=surface.d.ts.map