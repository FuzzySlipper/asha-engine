import { execFileSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import assert from 'node:assert/strict';

import { createMockRuntimeSession } from '@asha/runtime-bridge/reference';
import {
  createAshaRendererBrowserSurfaceFrame,
  type FirstPersonTunnelViewportCollisionDebug,
  FIRST_PERSON_TUNNEL_VIEWPORT_FIXTURE_NAME,
  renderProjectedFrame,
  renderFirstPersonTunnelViewport,
  summarizeFirstPersonTunnelViewport,
} from './index.js';

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
  assert.equal(result.summary.debug.collisionProjectionHash, 'fnv1a64:78b242163cf67524');
  assert.equal(result.summary.scene.opCount, 18);
  assert.equal(result.summary.scene.instanceCount, 8);
  assert.equal(result.summary.scene.frameHash, 'fnv1a64:db081afd570c2f30');
  assert.equal(result.summary.scene.structuralHash, 'fnv1a64:35ad3bca1a9f1667');
  assert.equal(result.projection.handleCount, 8);
  assert.equal(result.renderer.handleCount, 8);
  assert.equal(result.renderer.instanceCountFor('mesh/generated-tunnel-wall'), 3);
  assert.equal(result.renderer.fallbackMaterialCount, 0);
  assert.match(result.structuralSnapshot, /label "generated-tunnel-floor"/);
  assert.match(result.structuralSnapshot, /label "generated-tunnel-spawn-player_start"/);
  assert.ok(result.summary.nonClaims.includes('not_runtime_authority'));
  assert.ok(result.summary.nonClaims.includes('not_pixel_golden'));
});

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
  const collision: FirstPersonTunnelViewportCollisionDebug = {
    collided: true,
    blockedAxes: ['z'],
    worldHash: 'fnv1a64:test-world',
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

void test('renderer-three package root exposes tunnel viewport helpers under browser conditions', () => {
  const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  const proof = `
    const surface = await import('@asha/renderer-three');
    const required = [
      'createAshaRendererBrowserSurfaceFrame',
      'mountAshaRendererBrowserSurface',
      'createGeneratedTunnelViewportFrame',
      'renderFirstPersonTunnelViewport',
      'summarizeFirstPersonTunnelViewport'
    ];
    const forbidden = ['NativeRuntimeBridge', 'createNativeRuntimeBridge', 'NATIVE_WIRED_OPERATIONS'];
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
