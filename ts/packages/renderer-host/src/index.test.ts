import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

import { renderHandle, type RenderFrameDiff } from '@asha/contracts';
import {
  ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST,
  ASHA_RENDERER_HOST_COMPATIBILITY_VERSION,
  ASHA_RENDERER_INSPECTION_SURFACE_COMPATIBILITY_VERSION,
  AshaRendererHostError,
  createAshaRendererAnimatedMeshProjection,
  createAshaRendererSurfaceProjection,
  createAshaRendererDefaultSurfaceFrame,
  resolveAshaStoredEditorCamera,
} from './index.js';

function animationIntentFrame(clip = 'run'): RenderFrameDiff {
  const resource = ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST.resources[0];
  assert.ok(resource);
  return {
    ops: [
      {
        op: 'defineAnimatedMesh',
        asset: {
          asset: resource.asset,
          runtimeFormat: 'glb',
          contentHash: resource.contentHash,
          clips: [
            { id: 'idle', name: 'Idle', durationSeconds: 1.04166662693024 },
            { id: 'run', name: 'Run', durationSeconds: 0.666666686534882 },
            { id: 'jump', name: 'Jump', durationSeconds: 0.5 },
          ],
          defaultClip: 'idle',
          materialSlots: [],
          bounds: { min: [-0.02, -0.01, 0], max: [0.02, 0.01, 0.04] },
        },
      },
      {
        op: 'createAnimatedMeshInstance',
        handle: renderHandle(4100),
        parent: null,
        instance: {
          asset: resource.asset,
          transform: { translation: [0, 0, -2.5], rotation: [0, 0, 0, 1], scale: [40, 40, 40] },
          materialOverrides: [],
          playback: null,
          metadata: { source: null, tags: [], label: 'runtime-session animated enemy visual' },
        },
      },
      {
        op: 'setAnimatedMeshPlayback',
        handle: renderHandle(4100),
        playback: { action: 'play', clip, loop: 'repeat', speed: 1, weight: 1, restart: false, fadeSeconds: 0.1 },
      },
    ],
  };
}

function fixtureResolver(): Promise<ArrayBuffer> {
  const descriptor = ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST.resources[0];
  assert.ok(descriptor);
  const bytes = readFileSync(fileURLToPath(descriptor.resourceUrl));
  return Promise.resolve(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength));
}

void test('renderer-host projects render frames through the neutral projection model', () => {
  const frame: RenderFrameDiff = {
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

  assert.equal(ASHA_RENDERER_HOST_COMPATIBILITY_VERSION, 'renderer-host.v1');
  assert.equal(receipt.instructions.length, 1);
  assert.equal(receipt.snapshot.nodes.length, 1);
  assert.equal(receipt.snapshot.nodes[0]?.handle, 4385001);
});

void test('renderer-host can create the default visible surface frame', () => {
  const frame = createAshaRendererDefaultSurfaceFrame();

  assert.ok(frame.ops.length > 0);
  assert.ok(frame.ops.some((op) => op.op === 'create'));
});

void test('renderer-host root exports stored editor camera resolution without backend types', () => {
  const result = resolveAshaStoredEditorCamera({
    position: [0, 0, 5],
    target: [0, 0, 0],
    up: [0, 1, 0],
    projection: { fovYDegrees: 55, near: 0.05, far: 1000 },
  });

  assert.equal(result.ok, true);
});

void test('renderer-host public projection loads the real fixture and advances command-selected run playback', async () => {
  const testGlobal = globalThis as unknown as { self: unknown };
  const priorSelf = testGlobal.self;
  testGlobal.self = globalThis;
  const priorWarn = console.warn;
  const priorError = console.error;
  console.warn = () => undefined;
  console.error = () => undefined;
  try {
    const projection = await createAshaRendererAnimatedMeshProjection({
      manifest: ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST,
      resolveResource: fixtureResolver,
    });
    const applied = projection.applyFrame(animationIntentFrame());
    assert.equal(applied.applied, true);
    const selected = projection.playback(renderHandle(4100));
    assert.equal(selected.selectedClip, 'run');
    assert.equal(selected.status, 'playing');
    assert.equal(selected.commandSelected, true);
    assert.equal(selected.projectionOnly, true);
    assert.deepEqual(selected.diagnostics, []);

    assert.equal(projection.advance(0.25).applied, true);
    const advanced = projection.playback(renderHandle(4100));
    assert.ok(advanced.mixerTimeSeconds > selected.mixerTimeSeconds);
    assert.ok((advanced.actionTimeSeconds ?? 0) > (selected.actionTimeSeconds ?? 0));
    assert.notDeepEqual(
      advanced.poseSample?.hierarchyRotationSum,
      selected.poseSample?.hierarchyRotationSum,
    );
  } finally {
    console.warn = priorWarn;
    console.error = priorError;
    testGlobal.self = priorSelf;
  }
});

void test('renderer-host animation resources and playback fail closed with typed diagnostics', async () => {
  const testGlobal = globalThis as unknown as { self: unknown };
  const priorSelf = testGlobal.self;
  testGlobal.self = globalThis;
  const priorWarn = console.warn;
  const priorError = console.error;
  console.warn = () => undefined;
  console.error = () => undefined;
  try {
    const badManifest = {
      ...ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST,
      resources: ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST.resources.map((resource) => ({
        ...resource,
        contentHash: `sha256:${'0'.repeat(64)}` as const,
      })),
    };
    await assert.rejects(
      createAshaRendererAnimatedMeshProjection({ manifest: badManifest, resolveResource: fixtureResolver }),
      (error: unknown) =>
        error instanceof AshaRendererHostError &&
        error.diagnostics[0]?.code === 'animated_mesh_content_hash_mismatch',
    );

    const projection = await createAshaRendererAnimatedMeshProjection({
      manifest: ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST,
      resolveResource: fixtureResolver,
    });
    const unavailable = projection.playback(renderHandle(999));
    assert.equal(unavailable.status, 'unavailable');
    assert.equal(unavailable.commandSelected, false);
    assert.equal(unavailable.diagnostics[0]?.code, 'animated_mesh_handle_unavailable');
    const rejected = projection.applyFrame(animationIntentFrame('missing'));
    assert.equal(rejected.applied, false);
    assert.equal(rejected.diagnostics[0]?.code, 'animated_mesh_frame_rejected');
  } finally {
    console.warn = priorWarn;
    console.error = priorError;
    testGlobal.self = priorSelf;
  }
});

void test('renderer-host declarations do not expose concrete Three.js backend types', () => {
  const declarationPath = fileURLToPath(new URL('./index.d.ts', import.meta.url));
  const declarationText = readFileSync(declarationPath, 'utf8');
  const surfaceDeclarationPath = fileURLToPath(new URL('./surface.d.ts', import.meta.url));
  const surfaceDeclarationText = readFileSync(surfaceDeclarationPath, 'utf8');
  const editorViewportDeclarationPath = fileURLToPath(new URL('./editor-viewport.d.ts', import.meta.url));
  const editorViewportDeclarationText = readFileSync(editorViewportDeclarationPath, 'utf8');
  const inspectionSurfaceDeclarationPath = fileURLToPath(new URL('./inspection-surface.d.ts', import.meta.url));
  const inspectionSurfaceDeclarationText = readFileSync(inspectionSurfaceDeclarationPath, 'utf8');

  assert.doesNotMatch(declarationText, /@asha\/renderer-three/);
  assert.doesNotMatch(declarationText, /ThreeRenderer/);
  assert.doesNotMatch(declarationText, /WebGLRenderer/);
  assert.doesNotMatch(declarationText, /from ['"]three['"]/);
  assert.doesNotMatch(declarationText, /@asha\/runtime-bridge/);
  assert.match(declarationText, /mountAshaRendererInspectionSurface/);
  assert.equal(ASHA_RENDERER_INSPECTION_SURFACE_COMPATIBILITY_VERSION, 'inspection-surface.v0');
  assert.doesNotMatch(editorViewportDeclarationText, /@asha\/renderer-three/);
  assert.doesNotMatch(editorViewportDeclarationText, /ThreeRenderer/);
  assert.doesNotMatch(editorViewportDeclarationText, /WebGLRenderer/);
  assert.doesNotMatch(editorViewportDeclarationText, /from ['"]three['"]/);
  assert.doesNotMatch(editorViewportDeclarationText, /Scene|Object3D|Shader/);
  assert.match(editorViewportDeclarationText, /AshaRendererEditorViewportChannelHandle/);
  assert.match(editorViewportDeclarationText, /AshaRendererEditorViewportPickHint/);
  assert.match(editorViewportDeclarationText, /runtime_authority/);
  assert.match(editorViewportDeclarationText, /stored_editor/);
  assert.match(inspectionSurfaceDeclarationText, /projection_only_inspection/);
  assert.match(inspectionSurfaceDeclarationText, /readonly applyRuntimeFrame:/);
  assert.match(inspectionSurfaceDeclarationText, /readonly clearRuntimeProjection:/);
  assert.match(inspectionSurfaceDeclarationText, /readonly replaceFrame:/);
  assert.match(inspectionSurfaceDeclarationText, /readonly setGrid:/);
  assert.match(inspectionSurfaceDeclarationText, /readonly initialGrid\?: EditorGridDescriptor/);
  assert.match(inspectionSurfaceDeclarationText, /readonly lastCameraChange: AshaRendererInspectionCameraChange/);
  assert.match(inspectionSurfaceDeclarationText, /readonly cameraDistance: number/);
  assert.match(inspectionSurfaceDeclarationText, /readonly grid: EditorGridProjectionReadout \| null/);
  assert.match(inspectionSurfaceDeclarationText, /readonly runtimeFrameHash: string/);
  assert.match(inspectionSurfaceDeclarationText, /readonly runtimeGeneration: number/);
  assert.match(inspectionSurfaceDeclarationText, /readonly runtimeRetainedOpCount: number/);
  assert.doesNotMatch(inspectionSurfaceDeclarationText, /@asha\/renderer-three/);
  assert.doesNotMatch(inspectionSurfaceDeclarationText, /@asha\/runtime-bridge/);
  assert.doesNotMatch(inspectionSurfaceDeclarationText, /from ['"]three['"]/);
  assert.match(surfaceDeclarationText, /AshaRendererSurfacePickRequest/);
  assert.match(surfaceDeclarationText, /AshaRendererSurfacePickHint/);
  assert.match(surfaceDeclarationText, /readonly pick:/);
  assert.match(surfaceDeclarationText, /readonly resetCamera:/);
  assert.doesNotMatch(surfaceDeclarationText, /firePrimary/);
  assert.doesNotMatch(surfaceDeclarationText, /interactionState/);
  assert.doesNotMatch(surfaceDeclarationText, /targetHealth/);
  assert.doesNotMatch(surfaceDeclarationText, /shotsFired/);
  assert.doesNotMatch(surfaceDeclarationText, /remainingTargets/);
  assert.doesNotMatch(surfaceDeclarationText, /projectTargetProjection/);
  assert.doesNotMatch(surfaceDeclarationText, /projectRenderTargetProjection/);
});
