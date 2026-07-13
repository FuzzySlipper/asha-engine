import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { renderHandle } from '@asha/contracts';
import { createMockRuntimeSession } from '@asha/runtime-bridge/reference';
import { createGeneratedTunnelRoomFrame, FIRST_PERSON_TUNNEL_VIEWPORT_FIXTURE_NAME, summarizeFirstPersonTunnelViewport, } from '@asha/render-projection';
import { createAshaRendererBrowserSurfaceFrame, renderFirstPersonTunnelViewport, renderProjectedFrame, } from './backend.js';
function sessionInput() {
    return {
        sessionId: 'renderer-three.generated-tunnel.viewport',
        seed: 17,
        project: {
            gameId: 'asha-demo',
            workspaceId: 'workspace.local',
        },
        projectBundle: {
            bundleSchemaVersion: 1,
            protocolVersion: 1,
            sceneId: 42,
        },
    };
}
void test('first-person tunnel viewport renders generated tunnel frame from runtime camera projection', () => {
    const session = createMockRuntimeSession();
    session.initialize(sessionInput());
    const tunnel = session.readGeneratedTunnelReadout({ presetId: 'tiny-enclosed', seed: 17 });
    const camera = session.createCamera({
        initialPose: { position: [1.5, 1.5, 1.5], yawDegrees: 0, pitchDegrees: 0 },
        projection: { fovYDegrees: 60, near: 0.1, far: 100 },
        viewport: { width: 1280, height: 720 },
    }).snapshot.camera;
    const cameraProjection = session.readCameraProjection({ camera, viewport: null }).snapshot;
    const result = renderFirstPersonTunnelViewport({ tunnel, camera: cameraProjection });
    assert.equal(result.summary.kind, 'first_person_tunnel_viewport.v0');
    assert.equal(result.summary.fixture, FIRST_PERSON_TUNNEL_VIEWPORT_FIXTURE_NAME);
    assert.equal(result.summary.seed, 17);
    assert.equal(result.summary.camera.camera, camera);
    assert.equal(result.summary.camera.projectionHash, cameraProjection.projectionHash);
    assert.equal(result.summary.tunnel.dims.join('x'), '5x4x9');
    assert.deepEqual(result.summary.tunnel.spawnMarkers, ['player_start', 'exit_hint']);
    assert.deepEqual(result.summary.tunnel.materialRoles, ['wall:1', 'floor:2', 'accent:3']);
    assert.equal(result.summary.debug.generatorHash, 'fnv1a64:0821a0c2aea17dff');
    assert.equal(result.summary.debug.renderProjectionHash, 'fnv1a64:21eb8696f6f3b5c4');
    assert.equal(result.summary.debug.collisionProjectionHash, 'fnv1a64:627389be013a3154');
    assert.equal(result.summary.scene.opCount, 18);
    assert.equal(result.summary.scene.instanceCount, 8);
    assert.equal(result.summary.scene.frameHash, 'fnv1a64:db081afd570c2f30');
    assert.equal(result.summary.scene.structuralHash, 'fnv1a64:3abd4f9fa73fea4c');
    assert.equal(result.projection.handleCount, 8);
    assert.equal(result.renderer.handleCount, 8);
    assert.equal(result.renderer.instanceCountFor('mesh/generated-tunnel-wall'), 3);
    assert.equal(result.renderer.fallbackMaterialCount, 0);
    assert.match(result.structuralSnapshot, /label "generated-tunnel-floor"/);
    assert.match(result.structuralSnapshot, /label "generated-tunnel-spawn-player_start"/);
    const staticMeshDefs = result.frame.ops.filter(isDefineStaticMeshDiff);
    const floorAsset = staticMeshDefs.find((op) => op.asset.asset === 'mesh/generated-tunnel-floor');
    assert.equal(floorAsset?.asset.payload.layout.vertexCount, 24);
    assert.equal(floorAsset?.asset.payload.layout.indexCount, 36);
    const floor = result.renderer.objectFor(renderHandle(100));
    const westWall = result.renderer.objectFor(renderHandle(102));
    assert.deepEqual(floor?.scale.toArray(), [5, 0.1, 9]);
    assert.deepEqual(westWall?.scale.toArray(), [0.1, 4, 9]);
    assert.ok(result.summary.nonClaims.includes('not_runtime_authority'));
    assert.ok(result.summary.nonClaims.includes('not_pixel_golden'));
});
function isDefineStaticMeshDiff(op) {
    return op.op === 'defineStaticMesh';
}
void test('first-person tunnel viewport summary can carry optional collision debug hashes', () => {
    const session = createMockRuntimeSession();
    session.initialize(sessionInput());
    const tunnel = session.readGeneratedTunnelReadout({ presetId: 'tiny-enclosed', seed: 17 });
    const camera = session.createCamera({
        initialPose: { position: [1.5, 1.5, 1.5], yawDegrees: 0, pitchDegrees: 0 },
        projection: { fovYDegrees: 60, near: 0.1, far: 100 },
        viewport: { width: 800, height: 600 },
    }).snapshot.camera;
    const cameraProjection = session.readCameraProjection({ camera, viewport: null }).snapshot;
    const collision = {
        collided: true,
        blockedAxes: ['z'],
        collisionSourceHash: 'fnv1a64:test-collision-source',
        collisionProjectionHash: tunnel.collisionProjection.hash,
        movementHash: 'fnv1a64:test-move',
    };
    const result = renderFirstPersonTunnelViewport({
        tunnel,
        camera: cameraProjection,
        collision,
    });
    const summaryOnly = summarizeFirstPersonTunnelViewport({
        tunnel,
        camera: cameraProjection,
        frame: result.frame,
        structuralSnapshot: result.structuralSnapshot,
        collision,
    });
    assert.deepEqual(result.summary.debug.collision, collision);
    assert.equal(summaryOnly.scene.structuralHash, result.summary.scene.structuralHash);
});
void test('render-projection package root exposes renderer-neutral tunnel helpers under browser conditions', () => {
    const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
    const proof = `
    const surface = await import('@asha/render-projection');
    const required = [
      'createGeneratedTunnelRoomFrame',
      'createGeneratedTunnelViewportFrame',
      'summarizeFirstPersonTunnelViewport'
    ];
    const forbidden = [
      'NativeRuntimeBridge',
      'createNativeRuntimeBridge',
      'NATIVE_WIRED_OPERATIONS',
      'ThreeRenderer',
      'mountAshaRendererBrowserSurface',
      'createAshaRendererBrowserSurfaceFrame',
      'renderFirstPersonTunnelViewport'
    ];
    const missing = required.filter((name) => !(name in surface));
    const leaked = forbidden.filter((name) => name in surface);
    if (missing.length > 0 || leaked.length > 0) {
      throw new Error(JSON.stringify({ missing, leaked }));
    }
  `;
    execFileSync(process.execPath, ['--conditions=browser', '--input-type=module', '--eval', proof], {
        cwd: packageRoot,
        stdio: 'pipe',
    });
});
void test('renderer-three package root no longer exports renderer-neutral generated tunnel frame builders', () => {
    const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
    const proof = `
    const surface = await import('@asha/renderer-three');
    const leaked = [
      'createGeneratedTunnelRoomFrame',
      'createGeneratedTunnelViewportFrame',
      'summarizeFirstPersonTunnelViewport',
      'createAshaRendererGeneratedTunnelRoomSurfaceFrame'
    ].filter((name) => name in surface);
    if (leaked.length > 0) {
      throw new Error(JSON.stringify({ leaked }));
    }
  `;
    execFileSync(process.execPath, ['--conditions=browser', '--input-type=module', '--eval', proof], {
        cwd: packageRoot,
        stdio: 'pipe',
    });
});
void test('browser surface frame is an ASHA render diff consumed by the retained renderer', () => {
    const frame = createAshaRendererBrowserSurfaceFrame();
    const result = renderProjectedFrame(frame);
    assert.equal(frame.ops.length, 33);
    assert.equal(result.projection.handleCount, 33);
    assert.equal(result.renderer.handleCount, 33);
    assert.match(result.structuralSnapshot, /asha-renderer-flat-plane/);
    assert.match(result.structuralSnapshot, /asha-renderer-collision-wall-north/);
    assert.match(result.structuralSnapshot, /asha-renderer-random-cube-01/);
    assert.match(result.structuralSnapshot, /asha-renderer-random-cube-28/);
});
void test('renderer-three backend declarations stay render-backend scoped', () => {
    const declarationPath = fileURLToPath(new URL('../dist/browser-surface.d.ts', import.meta.url));
    const declarationText = readFileSync(declarationPath, 'utf8');
    assert.match(declarationText, /mountAshaRendererBrowserSurface/);
    assert.match(declarationText, /readonly pick:/);
    assert.match(declarationText, /AshaRendererBrowserSurfacePickReceipt/);
    assert.doesNotMatch(declarationText, /pickCenterObject/);
    assert.doesNotMatch(declarationText, /createAshaRendererGeneratedTunnelRoomSurfaceFrame/);
    assert.doesNotMatch(declarationText, /firePrimary/);
    assert.doesNotMatch(declarationText, /lockPointer/);
    assert.doesNotMatch(declarationText, /movementAuthority/);
    assert.doesNotMatch(declarationText, /pointerLocked/);
});
void test('generated tunnel browser surface frame carries combat target metadata', () => {
    const session = createMockRuntimeSession();
    session.initialize(sessionInput());
    const tunnel = session.readGeneratedTunnelReadout({ presetId: 'tiny-enclosed', seed: 17 });
    const frame = createGeneratedTunnelRoomFrame({
        tunnel,
        enemy: {
            label: 'generated-tunnel-enemy',
            position: [0, 1.1, -1.35],
            scale: [0.7, 1.8, 0.7],
        },
    });
    const result = renderProjectedFrame(frame);
    assert.ok(frame.ops.length > createAshaRendererBrowserSurfaceFrame().ops.length / 2);
    assert.match(result.structuralSnapshot, /generated-tunnel-floor/);
    assert.match(result.structuralSnapshot, /generated-tunnel-enemy/);
    assert.match(result.structuralSnapshot, /generated-tunnel-wall-rib-west-1/);
    assert.match(result.structuralSnapshot, /generated-tunnel-low-cover-east/);
    assert.match(result.structuralSnapshot, /generated-tunnel-ceiling-crossbeam/);
    const enemy = result.renderer.objectFor(renderHandle(4103901));
    assert.equal(enemy?.name, 'generated-tunnel-enemy');
});
//# sourceMappingURL=tunnel-viewport.test.js.map