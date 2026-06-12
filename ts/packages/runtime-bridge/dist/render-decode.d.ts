import { type RenderFrameDiff, type RenderDiff, type MeshPayloadDescriptor, type StaticMeshAsset, type SpriteInstanceDescriptor } from '@asha/contracts';
/** Raised when a payload does not match the render-diff contract shape. */
export declare class RenderDecodeError extends Error {
    readonly path: string;
    constructor(message: string, path: string);
}
/** Decode and structurally validate a mesh payload descriptor. */
export declare function decodeMeshPayloadDescriptor(v: unknown, path?: string): MeshPayloadDescriptor;
/** Decode a static mesh asset, validating slot uniqueness and group bindings. */
export declare function decodeStaticMeshAsset(v: unknown, path?: string): StaticMeshAsset;
/** Decode and validate a sprite instance descriptor. */
export declare function decodeSpriteInstance(v: unknown, path?: string): SpriteInstanceDescriptor;
/** Decode a single render diff (`create` / `update` / `destroy` / `replaceMeshPayload`). */
export declare function decodeRenderDiff(v: unknown, path?: string): RenderDiff;
/** Decode a whole frame of render diffs into the generated contract type. */
export declare function decodeRenderFrameDiff(v: unknown, path?: string): RenderFrameDiff;
/**
 * A small FIFO of decoded render frames for a renderer to drain each tick.
 *
 * The renderer pulls validated, contract-shaped frames out of here; it never
 * touches the raw payload or any WASM memory directly.
 */
export declare class RenderDiffStream {
    #private;
    /** Decode and enqueue a raw frame payload. Throws `RenderDecodeError` if malformed. */
    push(payload: unknown): void;
    /** Remove and return all enqueued frames, in arrival order. */
    drain(): RenderFrameDiff[];
    /** How many decoded frames are waiting. */
    get pending(): number;
}
/**
 * A borrowed view over WASM-owned bytes for a single frame.
 *
 * This is a placeholder for future large render payloads (e.g. vertex/index
 * buffers) that will be passed by reference into WASM memory rather than copied
 * through JSON. LIFETIME: a `FrameMemory` view is valid only for the frame it
 * was issued for. When the frame is superseded the host calls `invalidate()`,
 * after which `bytes()` throws — consumers must copy out anything they need to
 * retain *before* the next frame. Policy packages never receive one.
 */
export declare class FrameMemory {
    #private;
    constructor(bytes: Uint8Array);
    /** The borrowed bytes. Throws `RenderDecodeError` if the view was invalidated. */
    bytes(): Uint8Array;
    /** Whether this view is still usable. */
    get valid(): boolean;
    /** Drop the borrow; subsequent `bytes()` calls throw. */
    invalidate(): void;
}
//# sourceMappingURL=render-decode.d.ts.map