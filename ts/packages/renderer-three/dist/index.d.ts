import * as THREE from 'three';
import type { RenderDiff, RenderFrameDiff, RenderHandle } from '@asha/contracts';
/** Raised when a diff cannot be applied (duplicate, unknown, or stale handle). */
export declare class RenderApplyError extends Error {
    constructor(message: string);
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
    constructor();
    /** Apply a whole frame of diffs in order. */
    applyFrame(frame: RenderFrameDiff): void;
    /** Decode a raw payload through `@asha/runtime-bridge` and apply it. */
    applyEncodedFrame(payload: unknown): void;
    /** Apply a single diff. Throws `RenderApplyError` on a bad handle. */
    applyDiff(diff: RenderDiff): void;
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
}
//# sourceMappingURL=index.d.ts.map