import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { renderHandle } from '@asha/contracts';
import { ASHA_RENDERER_HOST_COMPATIBILITY_VERSION, createAshaRendererSurfaceProjection, createAshaRendererDefaultSurfaceFrame, surfaceTargetProjectionFromRenderTarget, } from './index.js';
void test('renderer-host projects render frames through the neutral projection model', () => {
    const frame = {
        ops: [
            {
                op: 'create',
                handle: renderHandle(4385001),
                parent: null,
                node: {
                    layer: 'scene',
                    geometry: { shape: 'cube' },
                    transform: {
                        translation: [0, 0, 0],
                        rotation: [0, 0, 0, 1],
                        scale: [1, 1, 1],
                    },
                    material: { color: [0.2, 0.4, 0.6, 1], wireframe: false },
                    visible: true,
                    metadata: { source: null, tags: [], label: 'renderer-host-neutral-cube' },
                },
            },
        ],
    };
    const receipt = createAshaRendererSurfaceProjection(frame);
    assert.equal(ASHA_RENDERER_HOST_COMPATIBILITY_VERSION, 'renderer-host.v0');
    assert.equal(receipt.instructions.length, 1);
    assert.equal(receipt.snapshot.nodes.length, 1);
    assert.equal(receipt.snapshot.nodes[0]?.handle, 4385001);
});
void test('renderer-host can create the default visible surface frame', () => {
    const frame = createAshaRendererDefaultSurfaceFrame();
    assert.ok(frame.ops.length > 0);
    assert.ok(frame.ops.some((op) => op.op === 'create'));
});
void test('renderer-host maps runtime render target identity to backend-neutral projection input', () => {
    const projection = surfaceTargetProjectionFromRenderTarget({
        kind: 'runtime_session.ecrp_render_target.v0',
        renderLabel: 'actor/generated-tunnel-enemy',
        position: [0, 0.5, -2.6],
        scale: [0.5, 1, 0.5],
        visible: false,
    }, { lastEvent: 'Enemy defeated' });
    assert.deepEqual(projection, {
        label: 'actor/generated-tunnel-enemy',
        lastEvent: 'Enemy defeated',
        position: [0, 0.5, -2.6],
        scale: [0.5, 1, 0.5],
        visible: false,
    });
});
void test('renderer-host accepts render target identity without a concrete render scale', () => {
    const projection = surfaceTargetProjectionFromRenderTarget({
        kind: 'runtime_session.ecrp_render_target.v0',
        renderLabel: 'actor/demo-player',
        position: [0, 1.62, 1.25],
        scale: null,
        visible: true,
    });
    assert.equal(projection.label, 'actor/demo-player');
    assert.equal('scale' in projection, false);
    assert.equal(projection.visible, true);
});
void test('renderer-host declarations do not expose concrete Three.js backend types', () => {
    const declarationPath = fileURLToPath(new URL('./index.d.ts', import.meta.url));
    const declarationText = readFileSync(declarationPath, 'utf8');
    assert.doesNotMatch(declarationText, /@asha\/renderer-three/);
    assert.doesNotMatch(declarationText, /ThreeRenderer/);
    assert.doesNotMatch(declarationText, /WebGLRenderer/);
    assert.doesNotMatch(declarationText, /from ['"]three['"]/);
    assert.doesNotMatch(declarationText, /@asha\/runtime-bridge/);
});
//# sourceMappingURL=index.test.js.map