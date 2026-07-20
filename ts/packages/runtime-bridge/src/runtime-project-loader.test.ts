import assert from 'node:assert/strict';
import test from 'node:test';

import {
  ASHA_PROJECT_BUNDLE_MANIFEST_PATH,
  createMemoryAshaProjectSource,
  createPackagedAshaProjectSource,
  encodeAshaProjectPackage,
} from '@asha/game-workspace';
import type { RuntimeSessionProjectSource } from '@asha/runtime-session';
import type { ActiveRuntimeProjectContentReadout } from '@asha/contracts';

import { createRuntimeSessionFacade } from './runtime-session-adapter.js';
import { createMockRuntimeBridge, MockRuntimeBridge } from './mock.js';
import type { FpsRuntimeSessionSnapshot } from './bridge.js';
import { entitySceneDocument } from './native-fps-fixtures.test-fixture.js';

const text = (value: string): Uint8Array => new TextEncoder().encode(value);

function projectFiles(): ReadonlyMap<string, Uint8Array> {
  const manifest = {
    bundleSchemaVersion: 2,
    protocolVersion: 1,
    project: { id: 71, name: 'public-loader-fixture' },
    entryScene: 101,
    scenes: [{ id: 101, schemaVersion: 4, artifact: 'scenes/entry.scene.json' }],
    assetLock: { artifact: 'assets/lock.json', assetCount: 0 },
    generationProvenance: null,
    artifacts: [
      { path: 'assets/lock.json', class: 'durable', role: 'assetLock', contentHash: '0000000000000001' },
      { path: 'scenes/entry.scene.json', class: 'durable', role: 'sceneDocument', contentHash: '0000000000000002' },
    ],
  };
  return new Map([
    [ASHA_PROJECT_BUNDLE_MANIFEST_PATH, text(`${JSON.stringify(manifest)}\n`)],
    ['assets/lock.json', text('{"assets":[]}\n')],
    ['scenes/entry.scene.json', text('{"schemaVersion":4,"id":101,"nodes":[]}\n')],
  ]);
}

function developmentDirectorySource(
  files: ReadonlyMap<string, Uint8Array>,
): RuntimeSessionProjectSource {
  return {
    kind: 'development-directory',
    identity: 'development-directory:/fixture/game',
    read: async (path) => {
      const bytes = files.get(path);
      if (bytes === undefined) throw new Error(`fixture source is missing ${path}`);
      return bytes.slice();
    },
  };
}

void test('loadProject uses one source-only call for development, packaged, and memory projects', async () => {
  const files = projectFiles();
  const sources: readonly RuntimeSessionProjectSource[] = [
    developmentDirectorySource(files),
    createPackagedAshaProjectSource('package:/fixture/game.asha', encodeAshaProjectPackage(files)),
    createMemoryAshaProjectSource('memory:public-loader', files),
  ];
  const expectedKinds = ['developmentDirectory', 'packagedProject', 'inMemory'];

  for (const [index, source] of sources.entries()) {
    const session = createRuntimeSessionFacade({ bridge: createMockRuntimeBridge(), mode: 'rust' });
    const initializeInput = {
      sessionId: `runtime-session.project-loader.${index}`,
      seed: 17,
      project: { gameId: 'loader-fixture', workspaceId: 'workspace.loader' },
    };
    assert.deepEqual(Object.keys(initializeInput).sort(), ['project', 'seed', 'sessionId']);
    session.initialize(initializeInput);

    const receipt = await session.loadProject({ source });
    assert.equal(receipt.accepted, true, JSON.stringify(receipt.diagnostics));
    assert.equal(receipt.source.kind, expectedKinds[index]);
    assert.equal(receipt.activeProject?.lifecycle.generation, 1);
    assert.equal(receipt.activeProject?.contentSetHash, 'mock-content-set');

    const closed = session.closeProject();
    assert.equal(closed.accepted, true);
    assert.equal(closed.lifecycle.revision, 2);
  }
});

void test('loadProject surfaces adapter failure without invoking runtime activation', async () => {
  const session = createRuntimeSessionFacade({ bridge: createMockRuntimeBridge(), mode: 'rust' });
  session.initialize({
    sessionId: 'runtime-session.project-loader.rejected',
    seed: 18,
    project: { gameId: 'loader-fixture', workspaceId: 'workspace.loader' },
  });
  const receipt = await session.loadProject({
    source: {
      kind: 'memory',
      identity: 'memory:missing-manifest',
      read: async () => { throw new Error('manifest is missing'); },
    },
  });
  assert.equal(receipt.accepted, false);
  assert.equal(receipt.diagnostics[0]?.phase, 'sourceAdapter');
  assert.equal(receipt.diagnostics[0]?.code, 'sourceAdapterRejected');
});

class CanonicalFpsProjectBridge extends MockRuntimeBridge {
  override readActiveRuntimeProjectContent(): ActiveRuntimeProjectContentReadout {
    const entityDefinition = (
      stableId: string,
      player: boolean,
    ): ActiveRuntimeProjectContentReadout['content']['documents'][number] => ({
      kind: 'entityDefinition',
      documentId: `entities/${stableId}.json`,
      definition: {
        stableId,
        displayName: player ? 'Canonical Player' : 'Canonical Enemy',
        source: { projectBundle: 'canonical-fps', relativePath: `entities/${stableId}.json` },
        tags: [],
        metadata: [],
        capabilities: [
          {
            kind: 'transform',
            transform: {
              translation: [0, 0, 0],
              rotation: [0, 0, 0, 1],
              scale: [1, 1, 1],
            },
          },
          { kind: 'bounds', min: [-0.5, -0.5, -0.5], max: [0.5, 0.5, 0.5] },
          { kind: 'collision', staticCollider: false },
          { kind: 'health', current: player ? 100 : 40, max: player ? 100 : 40 },
          {
            kind: 'renderProjection',
            projectionId: player ? 'first_person_camera' : 'target_cube',
            visible: true,
          },
          { kind: 'faction', factionId: player ? 'player' : 'hostile' },
          ...(player
            ? [{
                kind: 'weaponMount' as const,
                weaponId: 'weapon.canonical.primary',
                damage: 40,
                rangeUnits: 16,
                ammo: 2,
                cooldownTicksAfterFire: 4,
              }]
            : [{
                kind: 'policyBinding' as const,
                bindingId: 'enemy:policy',
                policyId: 'policy.enemy.canonical',
                viewKind: 'runtime_session.fps.policy_view.v0',
                viewVersion: 'v0',
                allowedIntents: ['runtime.intent.move_direct_nav.v0'],
                runtimeMoment: 'autonomous_policy_tick',
              }]),
        ],
      },
    });
    return {
      projectId: 71,
      manifestHash: 'mock-project-source:canonical-fps',
      contentSetHash: 'mock-content-set',
      entryScene: entitySceneDocument({
        id: 101,
        instances: [
          { entity: 101, definitionId: 'actor/player', spawnMarkerId: null, translation: [0, 1.6, 0] },
          { entity: 102, definitionId: 'actor/enemy', spawnMarkerId: null, translation: [0, 1.1, -3.5] },
        ],
      }),
      content: {
        accepted: true,
        documents: [
          entityDefinition('actor/player', true),
          entityDefinition('actor/enemy', false),
        ],
        canonicalFiles: [],
        setHash: 'mock-content-set',
        providerSchemas: [],
        fieldMetadata: [],
        diagnostics: [],
      },
    };
  }

  override readFpsRuntimeSession(): FpsRuntimeSessionSnapshot {
    return {
      backend: 'native_rust',
      authoritySurface: 'runtime_session.fps.canonical.v0',
      projectBundle: 'canonical-fps',
      sessionEpoch: 1,
      lifecycleStatus: { state: 'active' },
      playerEntity: 101,
      enemyEntity: 102,
      health: [
        { entity: 101, current: 100, max: 100 },
        { entity: 102, current: 40, max: 40 },
      ],
      policyBindings: [],
      replayRecords: [],
      readSets: [{
        viewKind: 'runtime_session.fps.lifecycle_health.v0',
        owner: 'rule-lifecycle',
        readSet: ['capability.health'],
      }],
      entityHash: 'fnv1a64:0000000000000001',
      healthHash: 'fnv1a64:0000000000000002',
      replayHash: 'fnv1a64:0000000000000003',
    };
  }
}

void test('loadProject derives FPS readouts from Rust active content without legacy load calls', async () => {
  const session = createRuntimeSessionFacade({
    bridge: new CanonicalFpsProjectBridge(),
    mode: 'rust',
  });
  session.initialize({
    sessionId: 'runtime-session.project-loader.canonical-fps',
    seed: 6007,
    project: { gameId: 'canonical-fps', workspaceId: 'workspace.loader' },
  });
  const receipt = await session.loadProject({
    source: createMemoryAshaProjectSource('memory:canonical-fps', projectFiles()),
  });
  assert.equal(receipt.accepted, true);
  const readout = session.readEcrpRuntimeReadout();
  assert.equal(readout.authority.source, 'rust_bridge');
  assert.equal(readout.projectBundle, null);
  assert.deepEqual(
    readout.entities.map((entity) => entity.definitionStableId),
    ['actor/player', 'actor/enemy'],
  );
  assert.equal(session.readLifecycleStatus().enemy.health.current, 40);
});
