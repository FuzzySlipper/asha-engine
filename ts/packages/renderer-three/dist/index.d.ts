import * as THREE from 'three';
import { type RuntimeBufferHandle, type RuntimeBufferView } from '@asha/runtime-bridge';
import type { RenderDiff, RenderFrameDiff, RenderHandle, SpritePickHit, RenderMaterialDescriptor, TextureDescriptor, SpriteAtlasDescriptor } from '@asha/contracts';
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
}
//# sourceMappingURL=index.d.ts.map