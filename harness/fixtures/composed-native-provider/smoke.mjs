import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { cp, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const [addonPath, repoRoot] = process.argv.slice(2);
if (!addonPath || !repoRoot) {
  throw new Error('usage: node smoke.mjs <addon-path> <repo-root>');
}

const runtimeBridge = await import(pathToFileURL(
  join(repoRoot, 'ts/packages/runtime-bridge/dist/index.js'),
));
const browserHost = await import(pathToFileURL(
  join(repoRoot, 'ts/packages/browser-host/dist/index.js'),
));
const gameWorkspace = await import(pathToFileURL(
  join(repoRoot, 'ts/packages/game-workspace/dist/index.js'),
));

const committedProject = join(
  repoRoot,
  'harness/fixtures/canonical-project-consumer/project',
);
const scratchParent = await mkdtemp(join(tmpdir(), 'asha-canonical-project-consumer-'));
const projectRoot = join(scratchParent, 'project');
await cp(committedProject, projectRoot, { recursive: true });

try {
  const developmentSource = await browserHost.createAshaProjectDirectorySource(projectRoot);
  const initial = await bootProject(developmentSource, 'development');
  assert.equal(initial.receipt.source.kind, 'developmentDirectory');
  assert.equal(initial.receipt.activeProject.sceneCount, 2);
  assert.ok(initial.receipt.activeProject.entityCount >= 2);
  assert.equal(initial.receipt.activeProject.voxelAssetCount, 1);
  assertProjectedHouse(initial.session.readProjection());
  assertCollision(initial.session, initial.receipt.activeProject.voxelBindings[0].grid);
  assertGameplayConfiguration(initial.bridge);
  initial.session.closeProject();

  const observed = await browserHost.observeAshaProjectStore(projectRoot);
  const authoringBridge = runtimeBridge.createNativeRuntimeBridge(addonPath);
  const authoring = runtimeBridge.createWorkspaceAuthoringFacade({ bridge: authoringBridge });
  const opened = await authoring.openProject({
    authoringId: 'canonical-project-consumer.authoring',
    seed: 5997,
    workspaceId: 'canonical-project-consumer.authoring',
    source: await browserHost.createAshaProjectDirectorySource(projectRoot),
  });
  assert.equal(opened.state.status, 'open');
  assert.equal(opened.projectContent?.accepted, true);
  assert.ok(opened.projectContent?.documents.some(
    (document) => document.documentId === 'prefabs/demo-registry.json',
  ));
  assertProjectedHouse(authoring.readProjection());

  applyContentCommand(authoring, {
    kind: 'upsert',
    document: {
      kind: 'assetCatalog',
      documentId: 'catalogs/split-a.json',
      catalog: { entries: [] },
    },
  });
  applyContentCommand(authoring, {
    kind: 'upsert',
    document: {
      kind: 'assetCatalog',
      documentId: 'catalogs/split-b.json',
      catalog: { entries: [] },
    },
  });
  applyContentCommand(authoring, {
    kind: 'delete',
    documentId: 'catalogs/split-source.json',
    documentKind: 'assetCatalog',
  });
  applyContentCommand(authoring, {
    kind: 'delete',
    documentId: 'presentation/delete-me.json',
    documentKind: 'presentationCatalog',
  });

  const prepared = authoring.prepareProjectWrite({
    observedPrior: observed.identity,
    priorManifestJson: observed.manifestJson,
    relocations: [{
      from: 'scenes/secondary.scene.json',
      to: 'scenes/archive/secondary.scene.json',
    }],
  });
  assert.equal(prepared.accepted, true, JSON.stringify(prepared.diagnostics));
  assert.ok(prepared.candidate);
  assert.ok(prepared.candidate.writes.some((write) => write.path === 'catalogs/split-a.json'));
  assert.ok(prepared.candidate.writes.some((write) => write.path === 'catalogs/split-b.json'));
  assert.ok(prepared.candidate.deletes.some(
    (entry) => entry.path === 'catalogs/split-source.json',
  ));
  assert.ok(prepared.candidate.deletes.some(
    (entry) => entry.path === 'presentation/delete-me.json',
  ));
  assert.ok(
    prepared.candidate.moves.some(
      (entry) => entry.from === 'scenes/secondary.scene.json'
        && entry.to === 'scenes/archive/secondary.scene.json',
    ) || prepared.candidate.writes.some(
      (entry) => entry.path === 'scenes/archive/secondary.scene.json',
    ),
  );

  await browserHost.applyAshaProjectWriteCandidate({
    projectRoot,
    candidate: prepared.candidate,
    readResource: async (resource) => authoringBridge.getBuffer(resource.handle).bytes,
    releaseResource: (resource) => authoringBridge.releaseBuffer(resource.handle),
    confirm: (publication) => authoring.confirmProjectWrite(publication).accepted,
  });
  assert.equal(authoring.readState().dirty, false);
  authoring.close({
    expectedWorkspaceId: opened.state.identity.project.workspaceId,
    expectedGeneration: opened.state.identity.generation,
  });

  const saved = await gameWorkspace.loadAshaProjectSource(
    await browserHost.createAshaProjectDirectorySource(projectRoot),
  );
  assert.ok(saved.manifest.scenes.some(
    (scene) => scene.artifact === 'scenes/archive/secondary.scene.json',
  ));
  assert.ok(saved.files.some((file) => file.path === 'catalogs/split-a.json'));
  assert.ok(!saved.files.some((file) => file.path === 'catalogs/split-source.json'));

  const freshDevelopment = await bootProject(
    await browserHost.createAshaProjectDirectorySource(projectRoot),
    'fresh-development',
  );
  assertProjectedHouse(freshDevelopment.session.readProjection());
  const developmentIdentity = comparableIdentity(freshDevelopment.receipt.activeProject);
  freshDevelopment.session.closeProject();

  const packageFiles = new Map([
    [gameWorkspace.ASHA_PROJECT_BUNDLE_MANIFEST_PATH, new TextEncoder().encode(saved.manifestJson)],
    ...saved.files.map((file) => [file.path, file.bytes]),
  ]);
  const packagedSource = gameWorkspace.createPackagedAshaProjectSource(
    'package:canonical-project-consumer.asha',
    gameWorkspace.encodeAshaProjectPackage(packageFiles),
  );
  const packaged = await bootProject(packagedSource, 'packaged');
  assert.equal(packaged.receipt.source.kind, 'packagedProject');
  assert.deepEqual(comparableIdentity(packaged.receipt.activeProject), developmentIdentity);
  assertProjectedHouse(packaged.session.readProjection());
  assertGameplayConfiguration(packaged.bridge);
  packaged.session.closeProject();

  console.log(JSON.stringify({
    projectId: developmentIdentity.projectId,
    sceneCount: developmentIdentity.sceneCount,
    entityCount: developmentIdentity.entityCount,
    voxelAssetCount: developmentIdentity.voxelAssetCount,
    structuralEdit: 'add-move-split-delete',
    sourceParity: ['development-directory', 'packaged-archive'],
  }));
} finally {
  await rm(scratchParent, { recursive: true, force: true });
}

async function bootProject(source, label) {
  const bridge = runtimeBridge.createNativeRuntimeBridge(addonPath);
  const session = runtimeBridge.createRuntimeSessionFacade({ bridge, mode: 'rust' });
  session.initialize({
    sessionId: `canonical-project-consumer.${label}`,
    seed: 5997,
    project: { gameId: '5997', workspaceId: `canonical-project-consumer.${label}` },
  });
  const receipt = await session.loadProject({ source });
  assert.equal(receipt.accepted, true, JSON.stringify(receipt.diagnostics));
  assert.ok(receipt.activeProject);
  return { bridge, session, receipt };
}

function applyContentCommand(authoring, command) {
  const result = authoring.applyProjectContentCommand(command);
  assert.equal(result.accepted, true, JSON.stringify(result.diagnostics));
  return result;
}

function assertProjectedHouse(projection) {
  assert.ok(projection.frame.ops.some(
    (operation) => operation.op === 'create'
      && operation.node.transform.translation[0] === 3
      && operation.node.transform.translation[1] === 0
      && operation.node.transform.translation[2] === -4,
  ), JSON.stringify(projection.frame.ops));
  assert.ok(projection.frame.ops.some((operation) => operation.op === 'replaceMeshPayload'));
}

function assertCollision(session, grid) {
  const camera = session.createCamera({
    initialPose: { position: [5, 1.5, 2], yawDegrees: 0, pitchDegrees: 0 },
    projection: { fovYDegrees: 60, near: 0.1, far: 100 },
    viewport: { width: 1280, height: 720 },
  }).snapshot.camera;
  const collision = session.applyCollisionConstrainedCameraInput({
    camera,
    grid,
    movementMode: 'grounded',
    input: {
      moveForward: 1,
      moveRight: 0,
      moveUp: 0,
      yawDeltaDegrees: 0,
      pitchDeltaDegrees: 0,
      dtSeconds: 1,
      moveSpeedUnitsPerSecond: 10,
    },
    tick: 1,
    shape: { halfExtents: [0.25, 0.7, 0.25] },
    policy: { mode: 'axis_separable_slide', maxIterations: 3 },
  });
  assert.equal(collision.collided, true);
  assert.ok(collision.blockedAxes.includes('z'));
}

function assertGameplayConfiguration(bridge) {
  const authored = JSON.parse(readFileSync(
    join(repoRoot, 'harness/fixtures/gameplay-module-sdk/downstream-module/project/gameplay-project.json'),
    'utf8',
  ));
  const declared = authored.declaredReads.find((read) => read.requestId === 'pulse-state');
  assert.ok(declared);
  const composed = bridge.readComposedRuntimeSession();
  const view = bridge.readGameplayModuleView({
    view: declared.view,
    scope: { kind: 'session' },
    expectedRuntimeSessionHash: composed.runtimeSessionHash,
  });
  assert.equal(new TextDecoder().decode(Uint8Array.from(view.canonicalPayload)), '4');
}

function comparableIdentity(identity) {
  return {
    projectId: identity.projectId,
    entrySceneId: identity.entrySceneId,
    sceneCount: identity.sceneCount,
    entityCount: identity.entityCount,
    voxelAssetCount: identity.voxelAssetCount,
    voxelBindings: identity.voxelBindings,
    contentSetHash: identity.contentSetHash,
    compositionHash: identity.compositionHash,
  };
}
