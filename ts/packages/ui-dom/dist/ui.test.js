import { test } from 'node:test';
import assert from 'node:assert/strict';
import { EditorStore, initialEditorContext, reduce } from '@asha/editor-tools';
import { defaultCamera, cameraPointerRay, orbitYaw, dolly, clampCameraOutOfSolid, inspect, previewOverlayDiffs, materialPalette, buildEditorControls, controlToAction, buildHudProjection, hudControlToIntent, MAX_BRUSH_SIZE, OVERLAY_HANDLE_BASE, } from './index.js';
function selectedContext(over = {}) {
    return {
        ...initialEditorContext(0),
        tool: 'place',
        selection: { voxel: { x: 5, y: 0, z: 0 }, face: 'negX' },
        ...over,
    };
}
// ── camera ──────────────────────────────────────────────────────────────────
void test('camera transforms are deterministic and keep the target', () => {
    const cam = defaultCamera();
    const a = orbitYaw(cam, Math.PI / 2);
    const b = orbitYaw(cam, Math.PI / 2);
    assert.deepEqual(a, b); // deterministic
    assert.deepEqual(a.target, cam.target); // orbit preserves target
    // Dolly moves along the target→position ray.
    const d = dolly(cam, 0.5);
    assert.deepEqual(d.position, [4, 4, 4]);
});
void test('camera collision clamps out of a solid using the injected query', () => {
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
void test('inspector is a pure function of editor context + diagnostics, holding no copy', () => {
    const ctx = selectedContext({ brushShape: 'box', brushSize: 3, material: 9 });
    const readout = inspect(ctx, { residentChunks: 4, lastMeshQuads: 96 });
    assert.equal(readout.tool, 'place');
    assert.equal(readout.brushShape, 'box');
    assert.equal(readout.brushSize, 3);
    assert.equal(readout.material, 9);
    assert.deepEqual(readout.selectedVoxel, { x: 5, y: 0, z: 0 });
    assert.equal(readout.selectedFace, 'negX');
    assert.equal(readout.affectedCells, 27); // 3³ box fill
    assert.equal(readout.diagnostics.residentChunks, 4);
    // Same inputs → identical readout (no hidden state).
    assert.deepEqual(inspect(ctx, { residentChunks: 4, lastMeshQuads: 96 }), readout);
});
void test('inspector reflects store changes without storing a voxel-state copy', () => {
    const store = new EditorStore(selectedContext());
    const before = inspect(store.getState());
    store.dispatch({ type: 'setTool', tool: 'remove' });
    const after = inspect(store.getState());
    assert.equal(before.tool, 'place');
    assert.equal(after.tool, 'remove');
    // The readout exposes the picked anchor, never voxel contents.
    assert.deepEqual(Object.keys(after).sort(), [
        'affectedCells', 'brushShape', 'brushSize', 'diagnostics', 'material', 'previewEnabled',
        'selectedFace', 'selectedVoxel', 'selectionMode', 'snapping', 'tool',
    ]);
});
// ── debug overlay (non-authoritative) ─────────────────────────────────────────
void test('preview overlay emits debug-layer wireframe diffs, never scene/authority', () => {
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
void test('preview overlay is empty when preview is disabled or nothing selected', () => {
    assert.deepEqual(previewOverlayDiffs(selectedContext({ preview: { enabled: false } })), []);
    assert.deepEqual(previewOverlayDiffs(initialEditorContext(0)), []); // no selection
});
void test('cameraPointerRay: centre pointer casts from the camera toward its target', () => {
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
void test('cameraPointerRay: a right-of-centre pointer aims further along +x in world space', () => {
    // Camera on +Z looking at origin, world up +Y: screen-right maps to world +X.
    const cam = { position: [0, 0, 10], target: [0, 0, 0], up: [0, 1, 0], fovDegrees: 60 };
    const centre = cameraPointerRay(cam, [0, 0], 1, 1);
    const right = cameraPointerRay(cam, [0.5, 0], 1, 1);
    assert.ok(Math.abs(centre.direction[0]) < 1e-9, 'centre ray has no x component');
    assert.ok(right.direction[0] > 0, 'a right pointer tilts the ray toward +x');
    // It is a unit direction (pure geometry, not a DDA).
    assert.ok(Math.abs(Math.hypot(...right.direction) - 1) < 1e-9);
});
// ── material palette + accessible editor controls (#2438) ──────────────────────
void test('materialPalette is built from the loaded catalog ids, not a hardcoded palette', () => {
    // Ids come from the loaded fixture/catalog read model.
    assert.deepEqual(materialPalette([1, 2, 3]), [
        { id: 1, label: 'Material 1' },
        { id: 2, label: 'Material 2' },
        { id: 3, label: 'Material 3' },
    ]);
    // A caller may supply catalog-sourced names.
    assert.deepEqual(materialPalette([7], (id) => `stone-${id}`), [{ id: 7, label: 'stone-7' }]);
});
const findControl = (controls, id) => {
    const c = controls.find((x) => x.id === id);
    assert.ok(c, `control ${id} present`);
    return c;
};
void test('buildEditorControls exposes accessible labels + roles for every control (agent-navigable)', () => {
    const ctx = { ...initialEditorContext(0), tool: 'paint', material: 2 };
    const controls = buildEditorControls(ctx, materialPalette([1, 2, 3]));
    // Every control has a stable id, an ARIA role, and a non-empty accessible label.
    for (const c of controls) {
        assert.ok(c.id.length > 0);
        assert.ok(c.role.length > 0);
        assert.ok(c.label.length > 0, `control ${c.id} has an accessible label`);
    }
    // Ids are unique (stable test handles for Playwright/agents).
    assert.equal(new Set(controls.map((c) => c.id)).size, controls.length);
    // The full editor surface is offered.
    for (const id of ['tool', 'material', 'brush-shape', 'brush-size', 'snapping', 'preview', 'commit', 'cancel']) {
        findControl(controls, id);
    }
    // Selected options reflect the context.
    const tool = findControl(controls, 'tool');
    assert.equal(tool.options?.find((o) => o.selected)?.value, 'paint');
    const material = findControl(controls, 'material');
    assert.deepEqual(material.options?.map((o) => o.label), ['Material 1', 'Material 2', 'Material 3']);
    assert.equal(material.options?.find((o) => o.selected)?.value, '2');
});
void test('control enablement: commit needs a proposal, cancel needs a selection, brush-size needs box', () => {
    const palette = materialPalette([1, 2, 3]);
    // No selection → commit + cancel disabled; single shape → brush-size disabled.
    const empty = buildEditorControls(initialEditorContext(0), palette);
    assert.equal(findControl(empty, 'commit').disabled, true);
    assert.equal(findControl(empty, 'cancel').disabled, true);
    assert.equal(findControl(empty, 'brush-size').disabled, true);
    // A place tool with a selection → commit + cancel enabled.
    const active = buildEditorControls({ ...initialEditorContext(0), tool: 'place', selection: { voxel: { x: 0, y: 0, z: 0 }, face: 'posX' } }, palette);
    assert.equal(findControl(active, 'commit').disabled, false);
    assert.equal(findControl(active, 'cancel').disabled, false);
    // Box shape → brush-size enabled, with first-scope bounds.
    const box = buildEditorControls({ ...initialEditorContext(0), brushShape: 'box' }, palette);
    const size = findControl(box, 'brush-size');
    assert.equal(size.disabled, false);
    assert.equal(size.min, 1);
    assert.equal(size.max, MAX_BRUSH_SIZE);
});
void test('controlToAction maps interactions to editor actions and round-trips through reduce', () => {
    let ctx = initialEditorContext(0);
    const apply = (id, value) => {
        const action = controlToAction(id, value);
        assert.ok(action, `${id} yields an action`);
        ctx = reduce(ctx, action);
    };
    apply('tool', 'paint');
    apply('material', '3');
    apply('brush-shape', 'box');
    apply('brush-size', '5');
    apply('snapping', 'off');
    apply('preview', 'off');
    assert.equal(ctx.tool, 'paint');
    assert.equal(ctx.material, 3);
    assert.equal(ctx.brushShape, 'box');
    assert.equal(ctx.brushSize, 5);
    assert.equal(ctx.snapping, false);
    assert.equal(ctx.preview.enabled, false);
    // Command buttons are app-level, not editor actions.
    assert.equal(controlToAction('commit', 'commit'), null);
    assert.equal(controlToAction('cancel', 'cancel'), null);
});
// ── HUD/menu projection (#4043) ───────────────────────────────────────────────
void test('buildHudProjection exposes health, status, non-claims, and menu controls', () => {
    const projection = buildHudProjection({
        health: { entity: 20, current: 24, max: 40, dead: false },
        status: [{ id: 'runtime', tone: 'info', text: 'Reference runtime' }],
        nonClaims: ['not_native_runtime', 'not_gameplay_loop'],
        menuOpen: true,
    });
    assert.equal(projection.kind, 'hud_projection.v0');
    assert.equal(projection.health.entity, 20);
    assert.equal(projection.health.current, 24);
    assert.equal(projection.health.max, 40);
    assert.equal(projection.health.ratio, 0.6);
    assert.equal(projection.health.label, 'Health 24/40');
    assert.deepEqual(projection.status, [{ id: 'runtime', tone: 'info', text: 'Reference runtime' }]);
    assert.deepEqual(projection.nonClaims, ['not_native_runtime', 'not_gameplay_loop']);
    const controlsById = new Map(projection.menu.controls.map((control) => [control.id, control]));
    assert.equal(projection.menu.open, true);
    assert.equal(controlsById.get('hud-resume')?.disabled, false);
    assert.equal(controlsById.get('hud-restart')?.label, 'Restart session');
    assert.equal(controlsById.get('hud-options')?.label, 'Options');
    assert.equal(controlsById.get('hud-exit')?.label, 'Exit');
    for (const control of projection.menu.controls) {
        assert.equal(control.role, 'button');
        assert.ok(control.label.length > 0);
    }
});
void test('HUD menu controls map to typed intents only', () => {
    assert.deepEqual(hudControlToIntent('hud-restart'), {
        kind: 'runtime.restart_session_intent',
        source: 'hud_menu',
    });
    assert.deepEqual(hudControlToIntent('hud-options'), {
        kind: 'ui.open_options_intent',
        source: 'hud_menu',
    });
    assert.deepEqual(hudControlToIntent('hud-exit'), {
        kind: 'ui.exit_to_menu_intent',
        source: 'hud_menu',
    });
    assert.deepEqual(hudControlToIntent('hud-resume'), {
        kind: 'ui.resume_intent',
        source: 'hud_menu',
    });
    assert.equal(hudControlToIntent('commit'), null);
    const restart = hudControlToIntent('hud-restart');
    assert.ok(restart);
    assert.equal('payload' in restart, false);
    assert.equal('command' in restart, false);
    assert.equal('submit' in restart, false);
});
void test('HUD health projection validates invalid readout data and marks defeated state', () => {
    const defeated = buildHudProjection({
        health: { entity: 20, current: 0, max: 40, dead: true },
        status: [{ id: 'combat', tone: 'danger', text: 'Defeated' }],
        nonClaims: [],
    });
    assert.equal(defeated.health.dead, true);
    assert.equal(defeated.health.ratio, 0);
    assert.equal(defeated.health.label, 'Health 0/40 defeated');
    assert.equal(defeated.menu.controls.find((control) => control.id === 'hud-resume')?.disabled, true);
    assert.throws(() => buildHudProjection({ health: { entity: 1, current: 41, max: 40, dead: false }, status: [], nonClaims: [] }), /current must not exceed max/);
});
//# sourceMappingURL=ui.test.js.map