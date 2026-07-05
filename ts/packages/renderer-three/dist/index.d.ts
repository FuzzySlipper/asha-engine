import * as THREE from 'three';
import { RenderProjection } from '@asha/render-projection';
import { type RuntimeBufferHandle, type RuntimeBufferView } from '@asha/runtime-bridge';
import type { MeshPickHit, CameraBasis, RenderDiff, RenderFrameDiff, RenderHandle, SpritePickHit, RenderMaterialDescriptor, TextureDescriptor, SpriteAtlasDescriptor } from '@asha/contracts';
import { type FirstPersonTunnelViewportInput, type FirstPersonTunnelViewportSummary } from './tunnel-viewport.js';
export * from './static-room.js';
export * from './tunnel-viewport.js';
/** Raised when a diff cannot be applied (duplicate, unknown, or stale handle). */
export declare class RenderApplyError extends Error {
    constructor(message: string);
}
/**
 * The capability the renderer needs to upload a handle-backed mesh payload.
 *
 * Lifetime semantics (#2428): **borrow → copy → release**. The renderer borrows the
 * bridge-owned bytes with {@link getBuffer}, copies every declared stream out into
 * fresh, renderer-owned typed arrays, and then returns the borrow with
 * {@link releaseBuffer} — on both the success and the failure path. It never retains
 * the borrowed view, never mutates authority, and never owns the bridge's bytes.
 *
 * Satisfied by the `@asha/runtime-bridge` facade
 * (`Pick<RuntimeBridge, 'getBuffer' | 'releaseBuffer'>`).
 */
export interface MeshBufferSource {
    getBuffer(handle: RuntimeBufferHandle): RuntimeBufferView;
    releaseBuffer(handle: RuntimeBufferHandle): void;
}
/**
 * A retained Three.js scene driven entirely by render diffs.
 *
 * Nodes are addressed by `RenderHandle`; the registry maps each handle to a
 * Three.js `Object3D`. Scene and debug layers are separate groups so overlays
 * can be toggled independently.
 */
export declare class ThreeRenderer {
    #private;
    readonly scene: THREE.Scene<THREE.Object3DEventMap>;
    constructor(options?: {
        meshBufferSource?: MeshBufferSource;
    });
    /** Apply a whole frame of diffs in order. */
    applyFrame(frame: RenderFrameDiff): void;
    /** Decode a raw payload through `@asha/runtime-bridge` and apply it. */
    applyEncodedFrame(payload: unknown): void;
    /** Apply a single diff. Throws `RenderApplyError` on a bad handle. */
    applyDiff(diff: RenderDiff): void;
    /**
     * Register the flat colour used for a material slot (the initial flat/debug
     * material strategy — ADR 0007). Unregistered slots fall back to a deterministic
     * per-slot colour, so a payload always maps to *some* visible material.
     */
    registerSlotColor(slot: number, r: number, g: number, b: number): void;
    /** Whether a handle is currently live in the scene. */
    has(handle: RenderHandle): boolean;
    /** Number of live scene handles. */
    get handleCount(): number;
    /** The Three.js object for a handle, for inspection/tests. */
    objectFor(handle: RenderHandle): THREE.Object3D | undefined;
    /**
     * A deterministic textual snapshot of the rendered scene — one line per live
     * handle (sorted), capturing layer, shape, transform, visibility, and colour.
     *
     * This is the "render artifact" the golden check diffs. It is a structural
     * snapshot rather than a pixel screenshot: GPU pixel output is
     * non-deterministic across drivers and headless GL is a heavy native
     * dependency, whereas this is exact, reviewable, and needs no GL context.
     */
    snapshot(): string;
    /** How many live instances reference a defined static mesh asset (0 if undefined). */
    instanceCountFor(asset: string): number;
    /** A registered catalog material descriptor by id, for inspection/tests. */
    materialDescriptor(id: string): RenderMaterialDescriptor | undefined;
    /** Total placeholder-fallback material resolutions so far (fallback diagnostic). */
    get fallbackMaterialCount(): number;
    /** Catalog material ids that resolved to a placeholder fallback (no descriptor). */
    fallbackMaterials(): string[];
    /** A registered texture descriptor by id, for inspection/tests. */
    textureDescriptor(id: string): TextureDescriptor | undefined;
    /** A registered sprite atlas by id, for inspection/tests. */
    spriteAtlas(id: string): SpriteAtlasDescriptor | undefined;
    /** Total sprite-frame fallbacks (no atlas / unknown frame) so far. */
    get spriteFallbackCount(): number;
    /**
     * Resolve a renderer-side sprite pick to an authority-facing trace: render
     * handle + source entity/scene-node ids + asset ref + attachment point. The
     * renderer decides no gameplay action — authority revalidates and acts.
     */
    pickSprite(handle: RenderHandle): SpritePickHit | undefined;
    /**
     * Resolve a renderer-side mesh pick to an authority source trace: the render handle
     * + the provenance of the uploaded mesh. Only a **hint** — authority picking
     * (`pickVoxel`) revalidates before any selection/edit acts on it. Returns
     * `undefined` for a handle with no uploaded mesh, or a stale/destroyed/unknown
     * handle (fail closed — the renderer never invents a source for missing metadata).
     */
    pickMesh(handle: RenderHandle): MeshPickHit | undefined;
}
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
//# sourceMappingURL=index.d.ts.map