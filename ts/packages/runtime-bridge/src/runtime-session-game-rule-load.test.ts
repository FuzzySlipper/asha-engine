import { test } from 'node:test';
import assert from 'node:assert/strict';

import type { GameRuleModuleManifest } from '@asha/contracts';

import {
  createRuntimeSessionFacade,
  type FpsRuntimeSessionLoadRequest,
  type RuntimeBridge,
} from './index.js';
import { MockRuntimeBridge } from './mock.js';

function sessionInput() {
  return {
    sessionId: 'runtime-session.game-rule-load.test',
    seed: 23,
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

function ecrpProjectLoadInput() {
  return {
    kind: 'runtime_session.load_ecrp_project.v0' as const,
    projectBundle: {
      kind: 'ProjectBundle' as const,
      project: {
        gameId: 'custom-demo',
        workspaceId: 'workspace.custom',
      },
      runtimeRequest: {
        bundleSchemaVersion: 1,
        protocolVersion: 1,
        sceneId: 77,
      },
    },
    entityDefinitions: [
      {
        kind: 'EntityDefinition' as const,
        stableId: 'actor/custom-player',
        displayName: 'Custom Player',
        source: {
          projectBundle: 'custom-demo',
          relativePath: 'catalogs/actors/custom-player.entity.json',
        },
        capabilities: [
          {
            kind: 'transform' as const,
            initial: {
              position: [1, 1.7, 2] as const,
              yawDegrees: 15,
              pitchDegrees: 0,
            },
          },
          { kind: 'health' as const, current: 88, max: 88 },
          { kind: 'renderProjection' as const, projection: 'first_person_camera' as const },
          { kind: 'controller' as const, controller: 'player_input' as const },
          { kind: 'faction' as const, factionId: 'player' },
        ],
      },
      {
        kind: 'EntityDefinition' as const,
        stableId: 'actor/custom-enemy',
        displayName: 'Custom Enemy',
        source: {
          projectBundle: 'custom-demo',
          relativePath: 'catalogs/actors/custom-enemy.entity.json',
        },
        capabilities: [
          {
            kind: 'transform' as const,
            initial: {
              position: [4, 1.2, -6] as const,
              yawDegrees: 180,
              pitchDegrees: 0,
            },
          },
          { kind: 'health' as const, current: 55, max: 55 },
          { kind: 'renderProjection' as const, projection: 'target_cube' as const },
          { kind: 'policyBinding' as const, policyId: 'policy.enemy.custom.v0' },
          { kind: 'faction' as const, factionId: 'hostile' },
        ],
      },
    ],
    sceneDocument: {
      kind: 'SceneDocument' as const,
      sceneId: 'custom-demo.scene',
      placements: [
        { entityDefinitionId: 'actor/custom-player', runtimeEntityId: 101 },
        { entityDefinitionId: 'actor/custom-enemy', runtimeEntityId: 202 },
      ],
    },
  };
}

function gameRuleModuleManifest(): GameRuleModuleManifest {
  return {
    moduleRef: {
      moduleId: 'demo.primary_fire_effect',
      version: '0.1.0',
      contractHash: 'sha256-demo-primary-fire-contract',
    },
    declaredHooks: [{
      hookId: 'demo.primary_fire_effect.weapon',
      kind: 'weaponEffect',
      inputContract: 'asha.weapon_effect_input.v0',
      outputContract: 'asha.weapon_effect_output.v0',
      requiredCapabilities: ['health', 'weaponMount'],
    }],
    deterministicRequirements: ['no_wall_clock', 'no_random_without_seed'],
    sourceHash: 'sha256-demo-primary-fire-source',
  };
}

function bridgeWithLoadCapture(): {
  readonly bridge: RuntimeBridge;
  readonly calls: { readonly load: FpsRuntimeSessionLoadRequest[] };
} {
  const calls: { load: FpsRuntimeSessionLoadRequest[] } = { load: [] };
  class CapturingRuntimeBridge extends MockRuntimeBridge {
    override loadFpsRuntimeSession(request: FpsRuntimeSessionLoadRequest) {
      calls.load.push(request);
      return super.loadFpsRuntimeSession(request);
    }
  }
  const bridge = new CapturingRuntimeBridge();
  return { bridge, calls };
}

void test('Rust-backed ECRP load forwards generated game-rule module manifests', () => {
  const { bridge, calls } = bridgeWithLoadCapture();
  const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
  session.initialize(sessionInput());

  const manifest = gameRuleModuleManifest();
  const load = session.loadEcrpProject({
    ...ecrpProjectLoadInput(),
    gameRuleModules: [manifest],
  });

  assert.equal(load.accepted, true);
  assert.equal(calls.load.length, 2);
  assert.equal(calls.load.at(-1)?.projectBundle, 'custom-demo:custom-demo.scene');
  assert.deepEqual(calls.load.at(-1)?.gameRuleModules, [manifest]);
});

void test('Rust-backed ECRP load rejects invalid game-rule manifests before bridge authority', () => {
  const { bridge, calls } = bridgeWithLoadCapture();
  const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
  session.initialize(sessionInput());
  const before = session.readEcrpRuntimeReadout();
  const loadCallsBeforeInvalid = calls.load.length;

  const load = session.loadEcrpProject({
    ...ecrpProjectLoadInput(),
    gameRuleModules: [{
      ...gameRuleModuleManifest(),
      moduleRef: {
        ...gameRuleModuleManifest().moduleRef,
        moduleId: '',
      },
    }],
  });

  assert.equal(load.accepted, false);
  assert.equal(calls.load.length, loadCallsBeforeInvalid);
  assert.ok(load.diagnostics.some((diagnostic) => diagnostic.code === 'invalidGameRuleModuleManifest'));
  assert.equal(session.readEcrpRuntimeReadout().project.gameId, before.project.gameId);
});
