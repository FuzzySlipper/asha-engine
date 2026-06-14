import { test } from 'node:test';
import assert from 'node:assert/strict';
import { EditorStore, initialEditorContext } from '@asha/editor-tools';
import { defaultCamera, cameraPointerRay, orbitYaw, dolly, clampCameraOutOfSolid, inspect, previewOverlayDiffs, OVERLAY_HANDLE_BASE, } from './index.js';
function selectedContext(over = {}) {
    return {
        ...initialEditorContext(0),
        tool: 'place',
        selection: { voxel: { x: 5, y: 0, z: 0 }, face: 'negX' },
        ...over,
    };
}
// ── camera ──────────────────────────────────────────────────────────────────
test('camera transforms are deterministic and keep the target', () => {
    const cam = defaultCamera();
    const a = orbitYaw(cam, Math.PI / 2);
    const b = orbitYaw(cam, Math.PI / 2);
    assert.deepEqual(a, b); // deterministic
    assert.deepEqual(a.target, cam.target); // orbit preserves target
    // Dolly moves along the target→position ray.
    const d = dolly(cam, 0.5);
    assert.deepEqual(d.position, [4, 4, 4]);
});
test('camera collision clamps out of a solid using the injected query', () => {
    const cam = { position: [0, 0, 0], target: [-1, 0, 0], up: [0, 1, 0], fovDegrees: 60 };
    // Everything with x <= 2 is "solid"; the camera should be pushed to x > 2.
    const isSolid = (p) => p[0] <= 2;
    const out = clampCameraOutOfSolid(cam, isSolid, 0.5);
    assert.ok(!isSolid(out.position), 'camera ends in free space');
    assert.ok(out.position[0] > 2);
    // No clamp when already free.
    const free = { ...cam, position: [10, 0, 0] };
    assert.deepEqual(clampCameraOutOfSolid(free, isSolid), free);
});
// ── inspector (pure read model) ───────────────────────────────────────────────
test('inspector is a pure function of editor context + diagnostics, holding no copy', () => {
    const ctx = selectedContext({ brushSize: 3, material: 9 });
    const readout = inspect(ctx, { residentChunks: 4, lastMeshQuads: 96 });
    assert.equal(readout.tool, 'place');
    assert.equal(readout.brushSize, 3);
    assert.equal(readout.material, 9);
    assert.deepEqual(readout.selectedVoxel, { x: 5, y: 0, z: 0 });
    assert.equal(readout.selectedFace, 'negX');
    assert.equal(readout.affectedCells, 27); // 3³ brush
    assert.equal(readout.diagnostics.residentChunks, 4);
    // Same inputs → identical readout (no hidden state).
    assert.deepEqual(inspect(ctx, { residentChunks: 4, lastMeshQuads: 96 }), readout);
});
test('inspector reflects store changes without storing a voxel-state copy', () => {
    const store = new EditorStore(selectedContext());
    const before = inspect(store.getState());
    store.dispatch({ type: 'setTool', tool: 'remove' });
    const after = inspect(store.getState());
    assert.equal(before.tool, 'place');
    assert.equal(after.tool, 'remove');
    // The readout exposes the picked anchor, never voxel contents.
    assert.deepEqual(Object.keys(after).sort(), [
        'affectedCells', 'brushSize', 'diagnostics', 'material', 'previewEnabled',
        'selectedFace', 'selectedVoxel', 'selectionMode', 'snapping', 'tool',
    ]);
});
// ── debug overlay (non-authoritative) ─────────────────────────────────────────
test('preview overlay emits debug-layer wireframe diffs, never scene/authority', () => {
    const ctx = selectedContext({ brushSize: 1 });
    const diffs = previewOverlayDiffs(ctx);
    assert.equal(diffs.length, 1);
    const op = diffs[0];
    assert.equal(op.op, 'create');
    if (op.op === 'create') {
        assert.equal(op.node.layer, 'debug'); // overlay only on the debug layer
        assert.equal(op.node.material.wireframe, true); // visually distinct
        assert.equal(op.handle, OVERLAY_HANDLE_BASE);
        assert.deepEqual(op.node.transform.translation, [4.5, 0.5, 0.5]); // cell centre
    }
});
test('preview overlay is empty when preview is disabled or nothing selected', () => {
    assert.deepEqual(previewOverlayDiffs(selectedContext({ preview: { enabled: false } })), []);
    assert.deepEqual(previewOverlayDiffs(initialEditorContext(0)), []); // no selection
});
test('cameraPointerRay: centre pointer casts from the camera toward its target', () => {
    const cam = defaultCamera(); // position [8,8,8] looking at origin
    const ray = cameraPointerRay(cam, [0, 0], 1, 1);
    assert.equal(ray.grid, 1);
    assert.deepEqual(ray.origin, [8, 8, 8]);
    // The centre ray points straight at the target: normalize(target - position).
    const inv = 1 / Math.sqrt(3);
    for (let i = 0; i < 3; i++) {
        assert.ok(Math.abs(ray.direction[i] - -inv) < 1e-9, `dir[${i}]`);
    }
});
test('cameraPointerRay: a right-of-centre pointer aims further along +x in world space', () => {
    // Camera on +Z looking at origin, world up +Y: screen-right maps to world +X.
    const cam = { position: [0, 0, 10], target: [0, 0, 0], up: [0, 1, 0], fovDegrees: 60 };
    const centre = cameraPointerRay(cam, [0, 0], 1, 1);
    const right = cameraPointerRay(cam, [0.5, 0], 1, 1);
    assert.ok(Math.abs(centre.direction[0]) < 1e-9, 'centre ray has no x component');
    assert.ok(right.direction[0] > 0, 'a right pointer tilts the ray toward +x');
    // It is a unit direction (pure geometry, not a DDA).
    assert.ok(Math.abs(Math.hypot(...right.direction) - 1) < 1e-9);
});
//# sourceMappingURL=ui.test.js.map