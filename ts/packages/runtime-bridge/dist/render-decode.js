// @asha/runtime-bridge / render-decode — decodes retained render-diff payloads
// into generated contract types for renderer consumption (ADR 0006).
//
// Moved here from the former `@asha/wasm-bridge`: decoding a render-diff payload
// into validated `@asha/contracts` `RenderFrameDiff` values is transport-neutral
// and belongs behind the runtime facade (it backs `readRenderDiffs`), reused by
// the native and WASM-replay paths alike. It applies nothing to a scene and
// imports no policy, renderer, UI, or Electron package. `FrameMemory` is the
// borrowed-bytes lifetime primitive for large payloads (cf. the facade
// `getBuffer`/`releaseBuffer` buffer handles).
import { renderHandle, entityId, tagId, } from '@asha/contracts';
/** Raised when a payload does not match the render-diff contract shape. */
export class RenderDecodeError extends Error {
    path;
    constructor(message, path) {
        super(`render decode error at ${path}: ${message}`);
        this.path = path;
        this.name = 'RenderDecodeError';
    }
}
// ── Primitive validators ──────────────────────────────────────────────────────
function asObject(v, path) {
    if (typeof v !== 'object' || v === null || Array.isArray(v)) {
        throw new RenderDecodeError('expected an object', path);
    }
    return v;
}
function asNumber(v, path) {
    if (typeof v !== 'number' || !Number.isFinite(v)) {
        throw new RenderDecodeError('expected a finite number', path);
    }
    return v;
}
function asBoolean(v, path) {
    if (typeof v !== 'boolean') {
        throw new RenderDecodeError('expected a boolean', path);
    }
    return v;
}
function asArray(v, path) {
    if (!Array.isArray(v)) {
        throw new RenderDecodeError('expected an array', path);
    }
    return v;
}
function asNumberArray(v, len, path) {
    const arr = asArray(v, path);
    if (arr.length !== len) {
        throw new RenderDecodeError(`expected ${len} numbers, got ${arr.length}`, path);
    }
    return arr.map((x, i) => asNumber(x, `${path}[${i}]`));
}
function tuple3(v, path) {
    const [a, b, c] = asNumberArray(v, 3, path);
    return [a, b, c];
}
function tuple4(v, path) {
    const [a, b, c, d] = asNumberArray(v, 4, path);
    return [a, b, c, d];
}
function nullable(v, decode) {
    return v === null ? null : decode(v);
}
// ── Component validators ──────────────────────────────────────────────────────
function decodeHandle(v, path) {
    return renderHandle(asNumber(v, path));
}
function decodeTransform(v, path) {
    const o = asObject(v, path);
    return {
        translation: tuple3(o.translation, `${path}.translation`),
        rotation: tuple4(o.rotation, `${path}.rotation`),
        scale: tuple3(o.scale, `${path}.scale`),
    };
}
function decodeMaterial(v, path) {
    const o = asObject(v, path);
    return {
        color: tuple4(o.color, `${path}.color`),
        wireframe: asBoolean(o.wireframe, `${path}.wireframe`),
    };
}
function decodeGeometry(v, path) {
    const o = asObject(v, path);
    const shape = o.shape;
    switch (shape) {
        case 'cube':
        case 'sphere':
        case 'quad':
        case 'point':
            return { shape };
        case 'line':
            return {
                shape,
                a: tuple3(o.a, `${path}.a`),
                b: tuple3(o.b, `${path}.b`),
            };
        default:
            throw new RenderDecodeError(`unknown geometry shape ${JSON.stringify(shape)}`, `${path}.shape`);
    }
}
function decodeLayer(v, path) {
    if (v === 'scene' || v === 'debug') {
        return v;
    }
    throw new RenderDecodeError(`unknown layer ${JSON.stringify(v)}`, path);
}
function decodeMetadata(v, path) {
    const o = asObject(v, path);
    return {
        source: nullable(o.source, (s) => entityId(asNumber(s, `${path}.source`))),
        tags: asArray(o.tags, `${path}.tags`).map((t, i) => tagId(asNumber(t, `${path}.tags[${i}]`))),
        label: nullable(o.label, (l) => {
            if (typeof l !== 'string') {
                throw new RenderDecodeError('expected a string', `${path}.label`);
            }
            return l;
        }),
    };
}
function decodeNode(v, path) {
    const o = asObject(v, path);
    return {
        geometry: decodeGeometry(o.geometry, `${path}.geometry`),
        material: decodeMaterial(o.material, `${path}.material`),
        transform: decodeTransform(o.transform, `${path}.transform`),
        visible: asBoolean(o.visible, `${path}.visible`),
        layer: decodeLayer(o.layer, `${path}.layer`),
        metadata: decodeMetadata(o.metadata, `${path}.metadata`),
    };
}
// ── Diff validators ───────────────────────────────────────────────────────────
/** Decode a single render diff (`create` / `update` / `destroy`). */
export function decodeRenderDiff(v, path = '$') {
    const o = asObject(v, path);
    switch (o.op) {
        case 'create':
            return {
                op: 'create',
                handle: decodeHandle(o.handle, `${path}.handle`),
                parent: nullable(o.parent, (p) => decodeHandle(p, `${path}.parent`)),
                node: decodeNode(o.node, `${path}.node`),
            };
        case 'update':
            return {
                op: 'update',
                handle: decodeHandle(o.handle, `${path}.handle`),
                transform: nullable(o.transform, (t) => decodeTransform(t, `${path}.transform`)),
                material: nullable(o.material, (m) => decodeMaterial(m, `${path}.material`)),
                visible: nullable(o.visible, (b) => asBoolean(b, `${path}.visible`)),
                metadata: nullable(o.metadata, (m) => decodeMetadata(m, `${path}.metadata`)),
            };
        case 'destroy':
            return {
                op: 'destroy',
                handle: decodeHandle(o.handle, `${path}.handle`),
            };
        default:
            throw new RenderDecodeError(`unknown render diff op ${JSON.stringify(o.op)}`, `${path}.op`);
    }
}
/** Decode a whole frame of render diffs into the generated contract type. */
export function decodeRenderFrameDiff(v, path = '$') {
    const o = asObject(v, path);
    const ops = asArray(o.ops, `${path}.ops`).map((op, i) => decodeRenderDiff(op, `${path}.ops[${i}]`));
    return { ops };
}
// ── Stream access for renderer consumption ────────────────────────────────────
/**
 * A small FIFO of decoded render frames for a renderer to drain each tick.
 *
 * The renderer pulls validated, contract-shaped frames out of here; it never
 * touches the raw payload or any WASM memory directly.
 */
export class RenderDiffStream {
    #frames = [];
    /** Decode and enqueue a raw frame payload. Throws `RenderDecodeError` if malformed. */
    push(payload) {
        this.#frames.push(decodeRenderFrameDiff(payload));
    }
    /** Remove and return all enqueued frames, in arrival order. */
    drain() {
        const frames = this.#frames;
        this.#frames = [];
        return frames;
    }
    /** How many decoded frames are waiting. */
    get pending() {
        return this.#frames.length;
    }
}
// ── Memory-view lifetime contract (placeholder for large payloads) ────────────
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
export class FrameMemory {
    #bytes;
    constructor(bytes) {
        this.#bytes = bytes;
    }
    /** The borrowed bytes. Throws `RenderDecodeError` if the view was invalidated. */
    bytes() {
        if (this.#bytes === null) {
            throw new RenderDecodeError('frame memory view used after invalidation', '$');
        }
        return this.#bytes;
    }
    /** Whether this view is still usable. */
    get valid() {
        return this.#bytes !== null;
    }
    /** Drop the borrow; subsequent `bytes()` calls throw. */
    invalidate() {
        this.#bytes = null;
    }
}
//# sourceMappingURL=render-decode.js.map