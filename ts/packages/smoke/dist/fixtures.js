// Deterministic abstract fixtures for the smoke harness. No product nouns; these
// exercise the real facade load path and the real renderer upload path.
import { renderHandle } from '@asha/contracts';
/** The abstract fixture world the smoke harness loads through the facade. */
export const FIXTURE_WORLD = {
    bundleSchemaVersion: 1,
    protocolVersion: 1,
    sceneId: 1001,
};
/** A deterministic FNV-1a hash over the fixture world definition (stable evidence). */
export function fixtureWorldHash(request) {
    let hash = 0xcbf29ce484222325n;
    const prime = 0x100000001b3n;
    const mask = (1n << 64n) - 1n;
    const writeU32 = (value) => {
        for (let i = 0; i < 4; i++) {
            hash ^= BigInt((value >>> (i * 8)) & 0xff);
            hash = (hash * prime) & mask;
        }
    };
    writeU32(request.bundleSchemaVersion);
    writeU32(request.protocolVersion);
    writeU32(request.sceneId);
    return hash.toString(16).padStart(16, '0');
}
/** A minimal mesh node to host the fixture geometry. */
function meshNode() {
    return {
        geometry: { shape: 'cube' },
        material: { color: [1, 1, 1, 1], wireframe: false },
        transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
        visible: true,
        layer: 'scene',
        metadata: { source: null, tags: [], label: 'smoke-fixture' },
    };
}
/** A small inline quad payload (2 triangles, two material-slot groups). */
function quadPayload() {
    return {
        layout: {
            vertexCount: 4,
            indexCount: 6,
            indexWidth: 'u32',
            attributes: [
                { name: 'position', components: 3, kind: 'f32' },
                { name: 'normal', components: 3, kind: 'f32' },
            ],
        },
        groups: [
            { materialSlot: 1, start: 0, count: 3 },
            { materialSlot: 2, start: 3, count: 3 },
        ],
        bounds: { min: [0, 0, 0], max: [1, 1, 0] },
        source: {
            kind: 'inline',
            positions: [0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0],
            normals: [0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1],
            indices: [0, 1, 2, 0, 2, 3],
        },
        provenance: 'generated',
    };
}
/**
 * A deterministic fixture render frame: create one mesh node, then upload the quad
 * payload. Drives the renderer through its real create→replaceMeshPayload path.
 */
export function fixtureRenderFrame() {
    const handle = renderHandle(1);
    return {
        ops: [
            { op: 'create', handle, parent: null, node: meshNode() },
            { op: 'replaceMeshPayload', handle, payload: quadPayload() },
        ],
    };
}
//# sourceMappingURL=fixtures.js.map