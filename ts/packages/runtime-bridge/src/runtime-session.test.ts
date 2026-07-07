import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { cameraHandle, entityId } from '@asha/contracts';
import type {
  CameraCreateRequest,
  CollisionConstrainedCameraInputEnvelope,
  VoxelCommand,
} from '@asha/contracts';
import {
  RuntimeBridgeError,
  createRuntimeSessionFacade,
  readRuntimeSessionPlayableEncounterTick,
  readRuntimeSessionPlayableLoopState,
  type EnemyDirectNavMovementRequest,
  type FpsEncounterDirectorSnapshot,
  type FpsEncounterLifecycleInput,
  type FpsEncounterStateReadout,
  type FpsEncounterTransitionRequest,
  type FpsEncounterTransitionResult,
  type FpsRuntimeSessionLoadRequest,
  type FpsRuntimeSessionRestartRequest,
  type FpsRuntimeSessionSnapshot,
  type RuntimeBridge,
  type WorldLoadRequest,
} from './index.js';
import { createMockRuntimeBridge } from './mock.js';
import { createMockRuntimeSession } from './reference.js';
import { REFERENCE_FPS_COMBAT_FIXTURE_PROVENANCE } from './runtime-session-reference-fps-combat.js';
import { stableHash } from './runtime-session-hash.js';

const runtimeBridgeSourceDir = resolve(dirname(fileURLToPath(import.meta.url)), '../src');

function testHash(value: unknown): string {
  return stableHash(value as never);
}

void test('RuntimeSession fixture helpers do not expose TS Rust authority module names', () => {
  assert.equal(existsSync(resolve(runtimeBridgeSourceDir, 'runtime-session-rust-fps-authority.ts')), false);
  assert.equal(existsSync(resolve(runtimeBridgeSourceDir, 'runtime-session-reference-fps-combat.ts')), true);
});

function sessionInput() {
  return {
    sessionId: 'runtime-session.asha-demo.reference',
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
          {
            kind: 'collisionBody' as const,
            halfExtents: [0.3, 0.7, 0.3] as const,
          },
          {
            kind: 'controller' as const,
            controller: 'player_input' as const,
          },
          {
            kind: 'health' as const,
            current: 88,
            max: 88,
          },
          {
            kind: 'weaponMount' as const,
            weaponId: 'weapon.custom.primary',
          },
          {
            kind: 'renderProjection' as const,
            projection: 'first_person_camera' as const,
          },
          {
            kind: 'faction' as const,
            factionId: 'player',
          },
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
          {
            kind: 'collisionBody' as const,
            halfExtents: [0.8, 1, 0.8] as const,
          },
          {
            kind: 'health' as const,
            current: 55,
            max: 55,
          },
          {
            kind: 'renderProjection' as const,
            projection: 'target_cube' as const,
          },
          {
            kind: 'policyBinding' as const,
            policyId: 'policy.enemy.custom.v0',
          },
          {
            kind: 'spawnMarker' as const,
            markerId: 'spawn.enemy.custom',
          },
          {
            kind: 'faction' as const,
            factionId: 'hostile',
          },
        ],
      },
    ],
    sceneDocument: {
      kind: 'SceneDocument' as const,
      sceneId: 'custom-demo.scene',
      placements: [
        {
          entityDefinitionId: 'actor/custom-player',
          runtimeEntityId: 101,
          spawnMarkerId: 'spawn.player.custom',
        },
        {
          entityDefinitionId: 'actor/custom-enemy',
          runtimeEntityId: 202,
          spawnMarkerId: 'spawn.enemy.custom',
        },
      ],
    },
  };
}

function rustFpsSnapshot(input: {
  readonly epoch: number;
  readonly player: number;
  readonly enemy: number;
  readonly enemyHealth: number;
  readonly replayHash: string;
}): FpsRuntimeSessionSnapshot {
  return {
    backend: 'native_rust',
    authoritySurface: 'runtime_session.fps.reference.v0',
    projectBundle: 'custom-demo:custom-demo.scene',
    sessionEpoch: input.epoch,
    lifecycleStatus: input.enemyHealth <= 0
      ? { state: 'enemy_defeated', entity: input.enemy, tick: 7 }
      : { state: 'active' },
    playerEntity: input.player,
    enemyEntity: input.enemy,
    health: [
      { entity: input.player, current: 88, max: 88 },
      { entity: input.enemy, current: input.enemyHealth, max: 55 },
    ],
    policyBindings: [{
      entity: input.enemy,
      bindingId: 'actor/custom-enemy:policy',
      policyId: 'policy.enemy.custom.v0',
      viewKind: 'runtime_session.fps.policy_view.v0',
      viewVersion: 'v0',
      allowedIntents: ['enemy_policy.move_toward_target.v0', 'enemy_policy.primary_fire_intent.v0'],
      runtimeMoment: 'autonomous_policy_tick',
    }],
    replayRecords: [{
      replayUnit: 'fps-runtime-session',
      entityHash: 'fnv1a64:00000000000000aa',
      healthHash: 'fnv1a64:00000000000000bb',
      recordHash: input.replayHash,
    }],
    readSets: [{
      viewKind: 'runtime_session.fps.lifecycle_health.v0',
      owner: 'rule-lifecycle',
      readSet: ['entity.lifecycle', 'capability.health'],
    }],
    entityHash: 'fnv1a64:00000000000000aa',
    healthHash: input.enemyHealth <= 0 ? 'fnv1a64:00000000000000cc' : 'fnv1a64:00000000000000bb',
    replayHash: input.replayHash,
  };
}

function rustEncounterState(): FpsEncounterStateReadout {
  return {
    presetId: 'generated-tunnel-small-encounter',
    status: 'pending',
    spawnedEnemyIds: [],
    defeatedEnemyIds: [],
    revision: 0,
    lastTransition: 'initialized',
  };
}

function rustEncounterSnapshot(
  state: FpsEncounterStateReadout,
  lifecycle: FpsEncounterLifecycleInput,
  replayHash = 'fnv1a64:00000000000000e1',
): FpsEncounterDirectorSnapshot {
  return {
    backend: 'native_rust',
    authoritySurface: 'runtime_session.fps.encounter_director.v0',
    mutationOwner: 'rule-lifecycle',
    workspaceTrace: ['test bridge encounter readout'],
    state,
    lifecycle,
    readSets: [{
      viewKind: 'runtime_session.encounter_director.v0',
      owner: 'rule-lifecycle',
      readSet: ['FpsRuntimeSessionState.encounter'],
    }],
    encounterHash: testHash({ state, lifecycle }),
    replayHash,
  };
}

function rustRuntimeSessionBridgeDouble(options: {
  readonly rejectProjectBundle?: string;
  readonly rejectWorldSceneId?: number;
} = {}): {
  readonly bridge: RuntimeBridge;
  readonly calls: {
    load: FpsRuntimeSessionLoadRequest[];
    world: WorldLoadRequest[];
    fire: number[];
    nav: EnemyDirectNavMovementRequest[];
    restart: FpsRuntimeSessionRestartRequest[];
    encounterTransitions: FpsEncounterTransitionRequest[];
  };
} {
  const base = createMockRuntimeBridge();
  const calls: {
    load: FpsRuntimeSessionLoadRequest[];
    world: WorldLoadRequest[];
    fire: number[];
    nav: EnemyDirectNavMovementRequest[];
    restart: FpsRuntimeSessionRestartRequest[];
    encounterTransitions: FpsEncounterTransitionRequest[];
  } = { load: [], world: [], fire: [], nav: [], restart: [], encounterTransitions: [] };
  let player = 10;
  let enemy = 20;
  let epoch = 1;
  let encounterState = rustEncounterState();
  let snapshot = rustFpsSnapshot({
    epoch,
    player,
    enemy,
    enemyHealth: 40,
    replayHash: 'fnv1a64:0000000000000001',
  });
  const bridge = new Proxy(base, {
    get(target, property, receiver) {
      if (property === 'loadFpsRuntimeSession') {
        return (request: FpsRuntimeSessionLoadRequest) => {
          if (request.projectBundle === options.rejectProjectBundle) {
            throw new RuntimeBridgeError('invalid_input', `authority rejected ${request.projectBundle}`);
          }
          calls.load.push(request);
          player = request.definitions.find((definition) => definition.role === 'player')?.entity ?? player;
          enemy = request.definitions.find((definition) => definition.role === 'enemy')?.entity ?? enemy;
          const enemyDefinition = request.definitions.find((definition) => definition.entity === enemy);
          snapshot = rustFpsSnapshot({
            epoch,
            player,
            enemy,
            enemyHealth: enemyDefinition?.health?.current ?? 40,
            replayHash: 'fnv1a64:0000000000000002',
          });
          encounterState = rustEncounterState();
          return snapshot;
        };
      }
      if (property === 'loadWorldBundle') { // vocab-allow: proxy intercepts the legacy bridge operation by name.
        return (request: WorldLoadRequest) => {
          calls.world.push(request);
          if (request.sceneId === options.rejectWorldSceneId) {
            throw new RuntimeBridgeError('invalid_input', `authority rejected scene ${request.sceneId}`);
          }
          return target.loadWorldBundle(request); // vocab-allow: test delegates to the legacy bridge operation under the RuntimeSession facade.
        };
      }
      if (property === 'applyFpsPrimaryFire') {
        return (request: { readonly tick: number }) => {
          calls.fire.push(request.tick);
          snapshot = rustFpsSnapshot({
            epoch,
            player,
            enemy,
            enemyHealth: 0,
            replayHash: 'fnv1a64:0000000000000003',
          });
          return {
            backend: 'native_rust' as const,
            authoritySurface: 'runtime_session.fps.primary_fire.v0',
            mutationOwner: 'svc-combat',
            workspaceTrace: ['workspace.primary_fire', 'svc-combat.apply_damage', 'rule-lifecycle.enemy_defeated'],
            shooter: player,
            target: enemy,
            targetHealthBefore: { current: 55, max: 55 },
            targetHealthAfter: { current: 0, max: 55 },
            lifecycleStatus: { state: 'enemy_defeated' as const, entity: enemy, tick: request.tick },
            targetRenderVisible: false,
            entityHash: 'fnv1a64:00000000000000aa',
            healthHash: 'fnv1a64:00000000000000cc',
            replayHash: 'fnv1a64:0000000000000003',
          };
        };
      }
      if (property === 'applyEnemyDirectNavMovement') {
        return (request: EnemyDirectNavMovementRequest) => {
          calls.nav.push(request);
          return {
            entity: request.entity,
            authoritySource: 'rust_entity_store' as const,
            authorityTransport: 'native_rust' as const,
            from: request.seedPosition,
            target: request.target,
            nextWaypoint: request.target,
            distanceUnits: 0.35,
            reached: true,
            pathHash: testHash({ kind: 'direct-nav', request }),
            transformHash: testHash({ kind: 'transform', entity: request.entity, next: request.target }),
            projectionChanged: true,
          };
        };
      }
      if (property === 'readFpsRuntimeSession') {
        return () => snapshot;
      }
      if (property === 'restartFpsRuntimeSession') {
        return (request: FpsRuntimeSessionRestartRequest) => {
          calls.restart.push(request);
          assert.equal(request.expectedEpoch, epoch);
          epoch += 1;
          encounterState = rustEncounterState();
          snapshot = rustFpsSnapshot({
            epoch,
            player,
            enemy,
            enemyHealth: 55,
            replayHash: 'fnv1a64:0000000000000004',
          });
          return snapshot;
        };
      }
      if (property === 'readFpsEncounterDirector') {
        return (lifecycle: FpsEncounterLifecycleInput) => rustEncounterSnapshot(encounterState, lifecycle);
      }
      if (property === 'applyFpsEncounterTransition') {
        return (request: FpsEncounterTransitionRequest): FpsEncounterTransitionResult => {
          calls.encounterTransitions.push(request);
          let accepted = true;
          let rejectionReason: FpsEncounterTransitionResult['rejectionReason'] = null;
          let eventKind: FpsEncounterTransitionResult['eventKind'] = null;
          if (request.presetId !== 'generated-tunnel-small-encounter') {
            accepted = false;
            rejectionReason = 'unknown_encounter_preset';
          } else if (request.action === 'activate') {
            if (encounterState.status !== 'pending') {
              accepted = false;
              rejectionReason = 'encounter_not_pending';
            } else {
              eventKind = 'runtime_encounter.activated.v0';
              encounterState = {
                ...encounterState,
                status: 'active',
                spawnedEnemyIds: ['encounter.generated_tunnel_small.wave_1.enemy_001'],
                revision: encounterState.revision + 1,
                lastTransition: 'activated',
              };
            }
          } else if (request.action === 'sync_lifecycle') {
            eventKind = 'runtime_encounter.lifecycle_synced.v0';
            if (request.lifecycle.enemyDead || request.lifecycle.outcomeKind === 'won') {
              encounterState = {
                ...encounterState,
                status: 'cleared',
                spawnedEnemyIds: ['encounter.generated_tunnel_small.wave_1.enemy_001'],
                defeatedEnemyIds: ['encounter.generated_tunnel_small.wave_1.enemy_001'],
                revision: encounterState.revision + 1,
                lastTransition: 'cleared',
              };
            } else if (request.lifecycle.playerDead || request.lifecycle.outcomeKind === 'lost') {
              encounterState = {
                ...encounterState,
                status: 'failed',
                revision: encounterState.revision + 1,
                lastTransition: 'failed',
              };
            } else {
              encounterState = { ...encounterState, revision: encounterState.revision + 1 };
            }
          } else {
            eventKind = 'runtime_encounter.reset.v0';
            encounterState = { ...rustEncounterState(), revision: encounterState.revision + 1, lastTransition: 'reset' };
          }
          const replayHash = testHash({ request, accepted, rejectionReason, eventKind, encounterState });
          return {
            backend: 'native_rust',
            authoritySurface: 'runtime_session.fps.encounter_transition.v0',
            mutationOwner: 'rule-lifecycle',
            workspaceTrace: ['test bridge encounter transition'],
            accepted,
            rejectionReason,
            eventKind,
            state: encounterState,
            lifecycle: request.lifecycle,
            encounterHash: testHash({ state: encounterState, lifecycle: request.lifecycle }),
            replayHash,
          };
        };
      }
      void receiver;
      const value: unknown = (target as unknown as Record<PropertyKey, unknown>)[property];
      if (typeof value === 'function') {
        const method = value as (this: RuntimeBridge, ...args: readonly unknown[]) => unknown;
        return method.bind(target);
      }
      return value;
    },
  }) as RuntimeBridge;
  return { bridge, calls };
}

void test('RuntimeSession initializes, ticks, reads projection and telemetry, then restarts', () => {
  const session = createMockRuntimeSession();
  const initialized = session.initialize(sessionInput());

  assert.equal(initialized.identity.sessionId, 'runtime-session.asha-demo.reference');
  assert.equal(initialized.identity.mode, 'reference');
  assert.equal(initialized.composition.loadedWorld, 42);
  assert.ok(initialized.identity.nonClaims.includes('not_raw_state_store'));
  assert.ok(initialized.identity.nonClaims.includes('not_arbitrary_json_bridge'));

  const command: VoxelCommand = {
    op: 'setVoxel',
    grid: 1,
    coord: { x: 0, y: 0, z: 0 },
    value: { kind: 'solid', material: 1 },
  };
  const receipt = session.submitCommands({ commands: [command] });
  assert.equal(receipt.result.accepted, 1);
  assert.equal(receipt.result.rejected, 0);
  assert.notEqual(receipt.sessionHashAfter, receipt.sessionHashBefore);

  const tick = session.tick();
  assert.equal(tick.tick, 1);
  assert.equal(tick.composition.loadedWorld, 42);

  const projection = session.readProjection();
  assert.equal(projection.sequenceId, tick.sequenceId);
  assert.equal(projection.renderDiffCount, 0);
  assert.ok(projection.projectionHash.startsWith('fnv1a64:'));

  const telemetry = session.readTelemetry();
  assert.equal(telemetry.acceptedCommandCount, 1);
  assert.equal(telemetry.rejectedCommandCount, 0);
  assert.equal(telemetry.replayRecords.map((record) => record.kind).join(','), 'initialize,submitCommands,tick');

  const restarted = session.restart();
  assert.equal(restarted.tick, 0);
  assert.equal(restarted.restartCount, 1);
  assert.equal(restarted.composition.loadedWorld, 42);

  const afterRestart = session.readTelemetry();
  assert.equal(afterRestart.acceptedCommandCount, 0);
  assert.equal(afterRestart.rejectedCommandCount, 0);
  assert.equal(afterRestart.replayRecords.at(-1)?.kind, 'restart');
});

void test('Rust-backed RuntimeSession routes ECRP load, primary fire, and restart through bridge authority', () => {
  const { bridge, calls } = rustRuntimeSessionBridgeDouble();
  const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
  const initialized = session.initialize(sessionInput());

  assert.equal(initialized.identity.mode, 'rust');
  assert.equal(initialized.identity.nonClaims.includes('not_native_runtime'), false);
  assert.equal(calls.load.length, 1);
  assert.equal(calls.load[0]?.definitions.some((definition) => definition.role === 'enemy'), true);

  const load = session.loadEcrpProject(ecrpProjectLoadInput());
  assert.equal(load.accepted, true);
  assert.equal(load.entityCount, 2);
  assert.equal(load.bootstrapHash, 'fnv1a64:00000000000000aa');
  assert.equal(calls.load.at(-1)?.projectBundle, 'custom-demo:custom-demo.scene');
  assert.equal(calls.load.at(-1)?.definitions.find((definition) => definition.role === 'enemy')?.entity, 202);
  assert.equal(
    calls.load.at(-1)?.definitions.find((definition) => definition.role === 'enemy')?.policyBinding?.policyId,
    'policy.enemy.custom.v0',
  );
  const rustReadout = session.readEcrpRuntimeReadout();
  assert.equal(rustReadout.authority.mode, 'rust');
  assert.equal(rustReadout.authority.source, 'rust_bridge');
  assert.equal(rustReadout.authority.surface, 'runtime_session.fps.reference.v0');
  assert.equal(rustReadout.authority.readSets[0]?.owner, 'rule-lifecycle');

  const receipt = session.submitRuntimeActionIntent({
    kind: 'runtime_action_intent.v0',
    action: 'primary_fire',
    phase: 'pressed',
    camera: cameraHandle(1),
    tick: 7,
    source: 'programmatic',
    pressed: true,
  });

  assert.equal(receipt.accepted, true);
  assert.equal(receipt.combatReadout?.fixture, null);
  assert.equal(receipt.combatReadout?.outcome.kind, 'hit');
  assert.equal(receipt.combatReadout?.outcome.kind === 'hit' ? receipt.combatReadout.outcome.target : null, 202);
  assert.equal(receipt.combatReadout?.health[0]?.current, 0);
  assert.equal(receipt.combatReadout?.healthHash, 'fnv1a64:00000000000000cc');
  assert.equal(receipt.combatReadout?.authority.source, 'rust_bridge');
  assert.equal(receipt.combatReadout?.authority.backend, 'native_rust');
  assert.equal(receipt.combatReadout?.authority.surface, 'runtime_session.fps.primary_fire.v0');
  assert.deepEqual(calls.fire, [7]);

  const lifecycle = session.readLifecycleStatus();
  assert.equal(lifecycle.restart.reason, 'rust_epoch_restart');
  assert.equal(lifecycle.outcome.kind, 'won');
  assert.equal(lifecycle.enemy.health.entity, 202);
  assert.equal(lifecycle.enemy.health.dead, true);

  const pendingEncounter = session.readEncounterDirector();
  assert.equal(pendingEncounter.authority.source, 'rust_bridge');
  assert.equal(pendingEncounter.authority.surface, 'runtime_session.fps.encounter_director.v0');
  assert.equal(pendingEncounter.state.status, 'pending');
  const activatedEncounter = session.requestEncounterTransition({
    kind: 'runtime_session.encounter_transition_request.v0',
    presetId: 'generated-tunnel-small-encounter',
    action: 'activate',
  });
  assert.equal(activatedEncounter.accepted, true);
  assert.equal(activatedEncounter.after.authority.source, 'rust_bridge');
  assert.equal(activatedEncounter.after.state.status, 'active');
  assert.equal(calls.encounterTransitions[0]?.action, 'activate');
  const clearedEncounter = session.requestEncounterTransition({
    kind: 'runtime_session.encounter_transition_request.v0',
    presetId: 'generated-tunnel-small-encounter',
    action: 'sync_lifecycle',
  });
  assert.equal(clearedEncounter.accepted, true);
  assert.equal(clearedEncounter.after.state.status, 'cleared');
  assert.equal(clearedEncounter.after.state.clearedReason, 'all_enemies_defeated');
  assert.equal(calls.encounterTransitions[1]?.lifecycle.outcomeKind, 'won');
  const rejectedEncounter = session.requestEncounterTransition({
    kind: 'runtime_session.encounter_transition_request.v0',
    presetId: 'generated-tunnel-small-encounter',
    action: 'activate',
  });
  assert.equal(rejectedEncounter.accepted, false);
  assert.equal(rejectedEncounter.rejectionReason, 'encounter_not_pending');

  const restart = session.requestSessionRestart({
    kind: 'runtime.restart_session_intent',
    source: 'programmatic',
    requireTerminal: true,
    expectedSessionHash: rejectedEncounter.hashes.sessionHashAfter,
  });
  assert.equal(restart.accepted, true);
  assert.equal(restart.statusAfter.outcome.kind, 'in_progress');
  assert.deepEqual(calls.restart, [{ expectedEpoch: 1 }]);
  const resetEncounter = session.readEncounterDirector();
  assert.equal(resetEncounter.state.status, 'pending');
  assert.equal(resetEncounter.state.revision, 0);

  const staleRestart = session.requestSessionRestart({
    kind: 'runtime.restart_session_intent',
    source: 'programmatic',
    expectedSessionHash: receipt.sessionHashAfter,
  });
  assert.equal(staleRestart.accepted, false);
  assert.equal(staleRestart.rejection?.reason, 'session_hash_mismatch');
  assert.deepEqual(calls.restart, [{ expectedEpoch: 1 }]);
});

void test('Rust-backed ECRP load fails closed on authority rejection without replacing live readout', () => {
  const { bridge, calls } = rustRuntimeSessionBridgeDouble({
    rejectProjectBundle: 'custom-demo:custom-demo.scene',
  });
  const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
  session.initialize(sessionInput());
  const before = session.readEcrpRuntimeReadout();

  assert.throws(
    () => session.loadEcrpProject(ecrpProjectLoadInput()),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );

  const after = session.readEcrpRuntimeReadout();
  assert.equal(after.project.gameId, before.project.gameId);
  assert.equal(after.projectBundle.sceneId, before.projectBundle.sceneId);
  assert.equal(after.authority.source, 'rust_bridge');
  assert.equal(calls.load.length, 1);
});

void test('RuntimeSession exposes bounded game-rules validation, submit, and readout surfaces', () => {
  const session = createRuntimeSessionFacade({ bridge: createMockRuntimeBridge(), mode: 'rust' });
  session.initialize(sessionInput());
  const catalog = {
    catalog: { catalogId: 'catalog.game-rules.test', version: '0.1.0', contentHash: 'fnv1a64:catalog' },
    valueChannels: [{ channelId: 'value.health', displayName: 'Health' }],
    bundles: [{
      bundleId: 'bundle.poisoned-impact',
      effectOps: [
        { kind: 'applyDelta' as const, opId: 'op.impact-damage', channelId: 'value.health', amount: -4, tags: ['tag.impact'] },
        {
          kind: 'schedulePeriodicEffect' as const,
          opId: 'op.schedule-poison',
          modifierId: 'modifier.poison',
          cadence: { periodTicks: 2 },
          duration: { kind: 'ticks' as const, ticks: 6 },
          tags: ['tag.poison'],
        },
      ],
      modifiers: [{
        modifierId: 'modifier.poison',
        stackPolicy: { kind: 'refresh' as const },
        duration: { kind: 'ticks' as const, ticks: 6 },
        tickCadence: { periodTicks: 2 },
        tags: ['tag.poison'],
        effectOpIds: ['op.poison-tick'],
        sourceHash: 'fnv1a64:poison',
      }],
      tags: ['tag.poison'],
      sourceHash: 'fnv1a64:bundle',
    }],
  };

  const validation = session.validateGameRuleCatalog(catalog);
  assert.equal(validation.accepted, true);
  assert.equal(validation.catalog.catalog.catalogId, 'catalog.game-rules.test');
  assert.equal(validation.sequenceId, 1);

  const receipt = session.submitGameRuleEffectIntent(catalog, {
    catalog: catalog.catalog,
    bundleId: 'bundle.poisoned-impact',
    source: entityId(101),
    target: entityId(777),
    values: [{ channelId: 'value.health', min: 0, current: 75, max: 75 }],
    tick: 9,
  });
  assert.equal(receipt.accepted, true);
  assert.deepEqual(receipt.pendingValueDeltas, [{ channelId: 'value.health', amount: -4 }]);
  assert.equal(receipt.appliedModifiers[0]?.modifierId, 'modifier.poison');
  assert.equal(receipt.appliedModifiers[0]?.source, 101);
  assert.equal(receipt.appliedModifiers[0]?.target, 777);
  assert.equal(receipt.appliedModifiers[0]?.nextTick, 11);
  assert.equal(receipt.appliedModifiers[0]?.expiresTick, 15);

  const readout = session.readGameRuleRuntimeReadout();
  assert.equal(readout.backend, 'reference_bridge');
  assert.equal(readout.activeModifiers[0]?.modifierId, 'modifier.poison');
  assert.equal(readout.activeModifiers[0]?.nextTick, 11);
  assert.equal(readout.latestReplayHash, receipt.replayHash);
  assert.equal(session.readTelemetry().replayRecords.at(-1)?.kind, 'submitGameRuleEffectIntent');

  const invalid = session.validateGameRuleCatalog({
    ...catalog,
    catalog: { ...catalog.catalog, contentHash: '' },
  });
  assert.equal(invalid.accepted, false);
  assert.equal(invalid.diagnostics[0]?.severity, 'error');
});

void test('Rust-backed ECRP load stages world authority before FPS runtime mutation', () => {
  const { bridge, calls } = rustRuntimeSessionBridgeDouble({
    rejectWorldSceneId: 77,
  });
  const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
  session.initialize(sessionInput());
  const beforeReadout = session.readEcrpRuntimeReadout();
  const beforeTelemetry = session.readTelemetry();

  assert.throws(
    () => session.loadEcrpProject(ecrpProjectLoadInput()),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );

  const afterReadout = session.readEcrpRuntimeReadout();
  const afterTelemetry = session.readTelemetry();
  assert.equal(calls.world.length, 2);
  assert.equal(calls.world[0]?.sceneId, 42);
  assert.equal(calls.world[1]?.sceneId, 77);
  assert.equal(calls.load.length, 1);
  assert.equal(afterReadout.project.gameId, beforeReadout.project.gameId);
  assert.equal(afterReadout.projectBundle.sceneId, beforeReadout.projectBundle.sceneId);
  assert.equal(afterReadout.authority.source, 'rust_bridge');
  assert.equal(afterReadout.authority.surface, beforeReadout.authority.surface);
  assert.equal(afterTelemetry.sequenceId, beforeTelemetry.sequenceId);
  assert.deepEqual(
    afterTelemetry.replayRecords.map((record) => record.recordHash),
    beforeTelemetry.replayRecords.map((record) => record.recordHash),
  );
});

void test('Rust-backed RuntimeSession routes autonomous policy tick through bridge authority', () => {
  const { bridge, calls } = rustRuntimeSessionBridgeDouble();
  const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
  session.initialize(sessionInput());

  const tick = session.runAutonomousPolicyTick({
    targetCamera: cameraHandle(1),
    tick: 2,
    enemy: { position: [3, 1, 7] },
    target: { position: [1, 1, 1] },
  });
  assert.equal(tick.tick, 2);
  assert.equal(tick.proposalSummary.acceptedProposalCount, 2);
  assert.equal(tick.proposalSummary.rejectedProposalCount, 0);
  assert.equal(tick.movementSummary?.authorityTransport, 'native_rust');
  assert.equal(tick.movementSummary?.authoritySource, 'rust_entity_store');
  assert.equal(tick.combatSummary?.status, 'accepted');
  assert.equal(tick.combatSummary?.healthHash, 'fnv1a64:00000000000000cc');
  assert.deepEqual(calls.nav.map((request) => request.entity), [20]);
  assert.deepEqual(calls.fire, [2]);
  assert.equal(tick.replay.lastRecordKind, 'runAutonomousPolicyTick');
  assert.ok(tick.tickHash.startsWith('fnv1a64:'));

  const rejected = session.runAutonomousPolicyTick({
    targetCamera: cameraHandle(1),
    tick: 3,
    policySource: 'Date.now(); fetch("/forbidden");',
    enemy: { position: [3, 1, 7] },
    target: { position: [1, 1, 1] },
  });
  assert.equal(rejected.policy.sourceDiagnostics.length, 2);
  assert.equal(rejected.proposalSummary.acceptedProposalCount, 0);
  assert.equal(rejected.proposalSummary.rejectedProposalCount, 2);
  assert.equal(rejected.proposalReceipts[0]?.rejection?.reason, 'policy_source_forbidden_capability');
  assert.deepEqual(calls.nav.map((request) => request.entity), [20]);
  assert.deepEqual(calls.fire, [2]);

  assert.throws(
    () => session.readNavProjection(),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'operation_unimplemented',
  );
  assert.throws(
    () =>
      session.requestSessionRestart({
        kind: 'runtime.restart_session_intent',
        source: 'programmatic',
        expectedSessionHash: '',
      }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
});

void test('RuntimeSession fails closed before initialize and on unsupported ProjectBundle', () => {
  const session = createMockRuntimeSession();
  assert.throws(
    () => session.tick(),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'not_initialized',
  );

  assert.throws(
    () =>
      session.initialize({
        ...sessionInput(),
        projectBundle: {
          bundleSchemaVersion: 99,
          protocolVersion: 1,
          sceneId: 42,
        },
      }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
});

void test('RuntimeSession exposes public ECRP entity and CapabilityState readouts', () => {
  const session = createMockRuntimeSession();
  assert.throws(
    () => session.readEcrpRuntimeReadout(),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'not_initialized',
  );
  session.initialize(sessionInput());

  const initial = session.readEcrpRuntimeReadout();

  assert.equal(initial.kind, 'runtime_session.ecrp_readout.v0');
  assert.equal(initial.entityCount, 2);
  assert.equal(initial.authority.mode, 'reference');
  assert.equal(initial.authority.source, 'reference_fixture');
  assert.equal(initial.authority.readSets[0]?.owner, 'reference-runtime-session');
  assert.ok(initial.nonClaims.includes('not_raw_state_store'));
  assert.ok(initial.nonClaims.includes('not_demo_local_authority'));
  const player = initial.entities.find((entity) => entity.definitionStableId === 'actor/demo-player');
  const enemy = initial.entities.find((entity) => entity.definitionStableId === 'actor/generated-tunnel-enemy');
  assert.ok(player);
  assert.ok(enemy);
  assert.deepEqual(player.capabilityKinds, [
    'transform',
    'collisionBody',
    'controller',
    'health',
    'weaponMount',
    'renderProjection',
    'faction',
  ]);
  assert.ok(enemy.capabilityKinds.includes('health'));
  assert.ok(enemy.capabilityKinds.includes('policyBinding'));
  const initialEnemyHealth = enemy.capabilities.find((capability) => capability.kind === 'health');
  const initialEnemyRender = enemy.capabilities.find((capability) => capability.kind === 'renderProjection');
  assert.equal(initialEnemyHealth?.kind, 'health');
  assert.equal(initialEnemyHealth?.dead, false);
  assert.equal(initialEnemyRender?.kind, 'renderProjection');
  assert.deepEqual(initialEnemyRender?.target, {
    kind: 'runtime_session.ecrp_render_target.v0',
    targetId: 'ecrp:20:actor/generated-tunnel-enemy',
    entity: 20,
    definitionStableId: 'actor/generated-tunnel-enemy',
    displayName: 'Generated Tunnel Enemy',
    source: {
      projectBundle: 'asha-demo',
      relativePath: 'catalogs/actors/generated-tunnel-enemy.entity.json',
    },
    role: 'enemy',
    projection: 'target_cube',
    renderLabel: 'actor/generated-tunnel-enemy',
    renderHandle: null,
    visible: true,
    position: [0, 1.1, -3.5],
    yawDegrees: 180,
    pitchDegrees: 0,
    scale: [1.4, 3.6, 1.4],
    targetHash: initialEnemyRender?.target.targetHash,
  });
  assert.match(initialEnemyRender?.target.targetHash ?? '', /^fnv1a64:[0-9a-f]{16}$/);
  assert.equal(enemy.recentEvents.length, 1);
  assert.equal(enemy.recentEvents[0]?.kind, 'runtime_session.bootstrap_entity.v0');

  const receipt = session.submitRuntimeActionIntent({
    kind: 'runtime_action_intent.v0',
    action: 'primary_fire',
    phase: 'pressed',
    camera: cameraHandle(1),
    tick: 7,
    source: 'programmatic',
    pressed: true,
  });
  assert.equal(receipt.accepted, true);
  assert.equal(receipt.combatReadout?.scenario, 'generated_tunnel_fire_hit');
  assert.equal(receipt.combatReadout?.outcome.kind, 'hit');
  assert.equal(receipt.combatReadout?.outcome.kind === 'hit' ? receipt.combatReadout.outcome.target : null, 20);
  assert.equal(receipt.combatReadout?.authority.source, 'reference_fixture');
  const afterFire = session.readEcrpRuntimeReadout();
  const defeatedEnemy = afterFire.entities.find((entity) => entity.entity === 20);
  const defeatedHealth = defeatedEnemy?.capabilities.find((capability) => capability.kind === 'health');
  const defeatedRender = defeatedEnemy?.capabilities.find((capability) => capability.kind === 'renderProjection');

  assert.equal(defeatedHealth?.kind, 'health');
  assert.equal(defeatedHealth?.dead, true);
  assert.equal(defeatedHealth?.current, 0);
  assert.equal(defeatedRender?.kind, 'renderProjection');
  assert.equal(defeatedRender?.visible, false);
  assert.equal(defeatedRender?.target.visible, false);
  assert.equal(defeatedRender?.target.renderLabel, 'actor/generated-tunnel-enemy');
  assert.ok(defeatedEnemy?.recentEvents.some((event) => event.kind === 'runtime_lifecycle.enemy_defeated.v0'));
  assert.notEqual(afterFire.hashes.capabilityStateHash, initial.hashes.capabilityStateHash);
  assert.notEqual(afterFire.hashes.eventReadoutHash, initial.hashes.eventReadoutHash);
});

void test('RuntimeSession loads ECRP ProjectBundle content into live readouts', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());

  const load = session.loadEcrpProject(ecrpProjectLoadInput());
  assert.equal(load.kind, 'runtime_session.ecrp_project_load_receipt.v0');
  assert.equal(load.accepted, true);
  assert.deepEqual(load.diagnostics, []);
  assert.equal(load.entityCount, 2);
  assert.match(load.bootstrapHash ?? '', /^fnv1a64:[0-9a-f]{16}$/);

  const readout = session.readEcrpRuntimeReadout();
  assert.equal(readout.project.gameId, 'custom-demo');
  assert.equal(readout.projectBundle.sceneId, 77);
  assert.equal(readout.entityCount, 2);
  const player = readout.entities.find((entity) => entity.definitionStableId === 'actor/custom-player');
  const enemy = readout.entities.find((entity) => entity.definitionStableId === 'actor/custom-enemy');
  assert.equal(player?.entity, 101);
  assert.equal(enemy?.entity, 202);
  assert.equal(player?.source.relativePath, 'catalogs/actors/custom-player.entity.json');
  const playerTransform = player?.capabilities.find((capability) => capability.kind === 'transform');
  assert.equal(playerTransform?.kind, 'transform');
  assert.deepEqual(playerTransform?.position, [1, 1.7, 2]);
  const enemyHealth = enemy?.capabilities.find((capability) => capability.kind === 'health');
  assert.equal(enemyHealth?.kind, 'health');
  assert.equal(enemyHealth?.current, 55);
  assert.equal(enemyHealth?.max, 55);
  assert.equal(enemyHealth?.dead, false);
  const enemyRender = enemy?.capabilities.find((capability) => capability.kind === 'renderProjection');
  assert.equal(enemyRender?.kind, 'renderProjection');
  assert.deepEqual(enemyRender?.target, {
    kind: 'runtime_session.ecrp_render_target.v0',
    targetId: 'ecrp:202:actor/custom-enemy',
    entity: 202,
    definitionStableId: 'actor/custom-enemy',
    displayName: 'Custom Enemy',
    source: {
      projectBundle: 'custom-demo',
      relativePath: 'catalogs/actors/custom-enemy.entity.json',
    },
    role: 'enemy',
    projection: 'target_cube',
    renderLabel: 'actor/custom-enemy',
    renderHandle: null,
    visible: true,
    position: [4, 1.2, -6],
    yawDegrees: 180,
    pitchDegrees: 0,
    scale: [1.6, 2, 1.6],
    targetHash: enemyRender?.target.targetHash,
  });
  assert.match(enemyRender?.target.targetHash ?? '', /^fnv1a64:[0-9a-f]{16}$/);

  const receipt = session.submitRuntimeActionIntent({
    kind: 'runtime_action_intent.v0',
    action: 'primary_fire',
    phase: 'pressed',
    camera: cameraHandle(2),
    tick: 9,
    source: 'programmatic',
    pressed: true,
  });
  assert.equal(receipt.accepted, true);
  assert.equal(receipt.combatReadout?.scenario, 'runtime_session_loaded_project_fire_hit');
  assert.equal(receipt.combatReadout?.outcome.kind, 'hit');
  assert.equal(receipt.combatReadout?.outcome.kind === 'hit' ? receipt.combatReadout.outcome.target : null, 202);
  assert.equal(receipt.combatReadout?.events.find((event) => event.kind === 'fire_hit')?.shooter, 101);
  assert.equal(receipt.combatReadout?.events.find((event) => event.kind === 'damage_applied')?.target, 202);
  assert.equal(receipt.combatReadout?.events.find((event) => event.kind === 'damage_applied')?.amount, 55);
  assert.deepEqual(receipt.combatReadout?.health[0], {
    entity: 202,
    current: 0,
    max: 55,
    dead: true,
  });
  assert.equal(
    receipt.combatReadout?.replayHash,
    stableHash({
      replayUnit: REFERENCE_FPS_COMBAT_FIXTURE_PROVENANCE.primaryFireReplayUnit,
      ruleCrate: REFERENCE_FPS_COMBAT_FIXTURE_PROVENANCE.ruleCrate,
      combatServiceCrate: REFERENCE_FPS_COMBAT_FIXTURE_PROVENANCE.combatServiceCrate,
      scenario: 'runtime_session_loaded_project_fire_hit',
      shooter: 101,
      target: 202,
      weaponId: 'weapon.custom.primary',
      health: [
        {
          entity: 202,
          current: 0,
          max: 55,
          dead: true,
        },
      ],
      events: [
        {
          kind: 'fire_hit',
          shooter: 101,
          target: 202,
          distance: 3.5,
          tick: 9,
        },
        {
          kind: 'damage_applied',
          target: 202,
          amount: 55,
          before: 55,
          after: 0,
        },
        {
          kind: 'entity_defeated',
          target: 202,
        },
      ],
    }),
  );
  assert.equal(receipt.combatReadout?.fixture, null);
  const afterFire = session.readEcrpRuntimeReadout();
  const defeatedEnemy = afterFire.entities.find((entity) => entity.entity === 202);
  const defeatedHealth = defeatedEnemy?.capabilities.find((capability) => capability.kind === 'health');
  const defeatedRender = defeatedEnemy?.capabilities.find((capability) => capability.kind === 'renderProjection');
  assert.equal(defeatedHealth?.kind, 'health');
  assert.equal(defeatedHealth?.current, 0);
  assert.equal(defeatedHealth?.dead, true);
  assert.equal(defeatedRender?.kind, 'renderProjection');
  assert.equal(defeatedRender?.visible, false);
  assert.ok(defeatedEnemy?.recentEvents.some((event) => event.kind === 'runtime_lifecycle.enemy_defeated.v0'));
});

void test('RuntimeSession playable-loop state reports current epoch HUD counters and reset semantics', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  session.loadEcrpProject(ecrpProjectLoadInput());

  const initial = readRuntimeSessionPlayableLoopState(session);
  assert.equal(initial.kind, 'runtime_session.playable_loop_state.v0');
  assert.equal(initial.status, 'runtime_authority');
  assert.equal(initial.counters.shotsFired, 0);
  assert.equal(initial.counters.hits, 0);
  assert.equal(initial.counters.remainingTargets, 1);
  assert.equal(initial.counters.totalTargets, 1);
  assert.equal(initial.health.player.current, 88);
  assert.equal(initial.health.enemy.current, 55);
  assert.equal(initial.commands.canFire, true);
  assert.equal(initial.target?.definitionStableId, 'actor/custom-enemy');
  assert.equal(initial.target?.renderLabel, 'actor/custom-enemy');

  const fire = session.submitRuntimeActionIntent({
    kind: 'runtime_action_intent.v0',
    action: 'primary_fire',
    phase: 'pressed',
    camera: cameraHandle(2),
    tick: 9,
    source: 'programmatic',
    pressed: true,
  });
  assert.equal(fire.accepted, true);

  const afterFire = readRuntimeSessionPlayableLoopState(session);
  assert.equal(afterFire.counters.actionTick, 1);
  assert.equal(afterFire.counters.shotsFired, 1);
  assert.equal(afterFire.counters.hits, 1);
  assert.equal(afterFire.counters.remainingTargets, 0);
  assert.equal(afterFire.health.enemy.dead, true);
  assert.equal(afterFire.commands.canFire, false);
  assert.deepEqual(afterFire.commands.blockedReasons, ['target_defeated']);
  assert.equal(afterFire.currentEpoch.restartCount, 0);

  session.requestSessionRestart({
    kind: 'runtime.restart_session_intent',
    source: 'hud_menu',
    requireTerminal: false,
    expectedSessionHash: session.readLifecycleStatus().sessionHash,
  });
  const afterReset = readRuntimeSessionPlayableLoopState(session);
  assert.equal(afterReset.currentEpoch.restartCount, 1);
  assert.equal(afterReset.counters.actionTick, 0);
  assert.equal(afterReset.counters.shotsFired, 0);
  assert.equal(afterReset.counters.hits, 0);
  assert.equal(afterReset.counters.remainingTargets, 1);
  assert.equal(afterReset.health.enemy.current, 55);
  assert.equal(afterReset.commands.canFire, true);
  assert.ok(afterReset.currentEpoch.replayRecordStartIndex > 0);
});

void test('RuntimeSession playable-loop state reports shell pause, player defeat, and missing backend gates', () => {
  const missing = readRuntimeSessionPlayableLoopState(null, {
    unavailableReason: 'native runtime bridge not provided',
  });
  assert.equal(missing.status, 'missing_backend');
  assert.equal(missing.commands.canFire, false);
  assert.deepEqual(missing.commands.blockedReasons, ['missing_backend']);
  assert.equal(missing.diagnostics[0]?.message, 'native runtime bridge not provided');

  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const paused = readRuntimeSessionPlayableLoopState(session, { shell: { paused: true, menuMode: 'paused' } });
  assert.equal(paused.shell.paused, true);
  assert.equal(paused.shell.menuMode, 'paused');
  assert.equal(paused.commands.canFire, false);
  assert.deepEqual(paused.commands.blockedReasons, ['paused']);

  const playerDefeatedFacade = {
    readEcrpRuntimeReadout: () => session.readEcrpRuntimeReadout(),
    readTelemetry: () => session.readTelemetry(),
    readLifecycleStatus: () => session.readLifecycleStatus({ scenario: 'generated_tunnel_player_defeated' }),
  };
  const defeated = readRuntimeSessionPlayableLoopState(playerDefeatedFacade);
  assert.equal(defeated.health.player.dead, true);
  assert.equal(defeated.commands.canFire, false);
  assert.deepEqual(defeated.commands.blockedReasons, ['player_dead']);
});

void test('RuntimeSession playable encounter tick derives enemy state and advances policy/combat', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const readout = readRuntimeSessionPlayableEncounterTick(session, {
    targetCamera: cameraHandle(2),
    targetPosition: [0, 1.1, -2.0],
    tick: 7,
  });

  assert.equal(readout.kind, 'runtime_session.playable_encounter_tick.v0');
  assert.equal(readout.status, 'advanced');
  assert.equal(readout.blockedReason, null);
  assert.equal(readout.tick, 7);
  assert.equal(readout.enemy.stableId, 'actor/generated-tunnel-enemy');
  assert.equal(readout.enemy.entity, 20);
  assert.deepEqual(readout.enemy.position, [0, 1.1, -3.5]);
  assert.equal(readout.player.camera, cameraHandle(2));
  assert.equal(readout.autonomousPolicy?.kind, 'runtime_session.autonomous_policy_tick.v0');
  assert.equal(readout.combatSummary?.status, 'accepted');
  assert.equal(readout.combatSummary?.outcome?.kind, 'hit');
  assert.equal(readout.lifecycleAfter?.player.health.current, 90);
  assert.ok(readout.nonClaims.includes('not_shell_scheduler'));
});

void test('RuntimeSession playable encounter tick supports movement-only and fail-closed gates', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const movementOnly = readRuntimeSessionPlayableEncounterTick(session, {
    targetCamera: cameraHandle(3),
    targetPosition: [0, 1.62, 1.25],
    combat: {
      lineOfSight: 'blocked',
      primaryFireRangeUnits: 2.4,
    },
  });
  assert.equal(movementOnly.status, 'advanced');
  assert.equal(movementOnly.movementSummary?.status, 'accepted');
  assert.equal(movementOnly.combatSummary, null);

  const paused = readRuntimeSessionPlayableEncounterTick(session, {
    targetCamera: cameraHandle(3),
    shell: { paused: true },
  });
  assert.equal(paused.status, 'blocked');
  assert.equal(paused.blockedReason, 'paused');
  assert.equal(paused.autonomousPolicy, null);

  const missingEnemy = readRuntimeSessionPlayableEncounterTick(session, {
    targetCamera: cameraHandle(3),
    enemyStableId: 'actor/missing-enemy',
  });
  assert.equal(missingEnemy.status, 'blocked');
  assert.equal(missingEnemy.blockedReason, 'missing_enemy');

  const missingBackend = readRuntimeSessionPlayableEncounterTick(null, {
    targetCamera: cameraHandle(3),
  });
  assert.equal(missingBackend.status, 'blocked');
  assert.equal(missingBackend.blockedReason, 'missing_backend');
});

void test('RuntimeSession playable encounter tick no-ops after terminal enemy defeat', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  session.loadEcrpProject(ecrpProjectLoadInput());
  session.submitRuntimeActionIntent({
    kind: 'runtime_action_intent.v0',
    action: 'primary_fire',
    phase: 'pressed',
    camera: cameraHandle(4),
    tick: 1,
    source: 'programmatic',
    pressed: true,
  });

  const blocked = readRuntimeSessionPlayableEncounterTick(session, {
    targetCamera: cameraHandle(4),
    targetPosition: [0, 1.62, 1.25],
  });
  assert.equal(blocked.status, 'blocked');
  assert.equal(blocked.blockedReason, 'enemy_dead');
  assert.equal(blocked.autonomousPolicy, null);
  assert.equal(blocked.lifecycleBefore?.enemy.dead, true);
});

void test('RuntimeSession rejects invalid ECRP ProjectBundle content without replacing live state', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const before = session.readEcrpRuntimeReadout();
  const invalid = {
    ...ecrpProjectLoadInput(),
    sceneDocument: {
      ...ecrpProjectLoadInput().sceneDocument,
      placements: [
        {
          entityDefinitionId: 'actor/unknown',
          runtimeEntityId: 404,
        },
      ],
    },
  };

  const load = session.loadEcrpProject(invalid);
  assert.equal(load.accepted, false);
  assert.ok(load.diagnostics.some((diagnostic) => diagnostic.code === 'unknownEntityDefinition'));
  assert.ok(load.diagnostics.some((diagnostic) => diagnostic.code === 'missingPlacement'));
  assert.equal(load.bootstrapHash, null);
  const after = session.readEcrpRuntimeReadout();
  assert.deepEqual(
    after.entities.map((entity) => entity.definitionStableId),
    before.entities.map((entity) => entity.definitionStableId),
  );
});

void test('RuntimeSession applies collision-constrained camera input against the static room fixture', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());

  const cameraRequest: CameraCreateRequest = {
    initialPose: {
      position: [0, 1.6, 0],
      yawDegrees: 0,
      pitchDegrees: 0,
    },
    projection: {
      fovYDegrees: 60,
      near: 0.1,
      far: 100,
    },
    viewport: {
      width: 1280,
      height: 720,
    },
  };
  const camera = session.createCamera(cameraRequest).snapshot.camera;
  const collisionShape = { halfExtents: [0.25, 0.25, 0.25] as const };
  const collisionPolicy = { mode: 'axis_separable_slide' as const, maxIterations: 3 };

  const blockedEnvelope: CollisionConstrainedCameraInputEnvelope = {
    camera,
    grid: 1,
    input: {
      moveForward: 1,
      moveRight: 0,
      moveUp: 0,
      yawDeltaDegrees: 10,
      pitchDeltaDegrees: -2,
      dtSeconds: 1,
      moveSpeedUnitsPerSecond: 99,
    },
    tick: 1,
    shape: collisionShape,
    policy: collisionPolicy,
  };
  const blocked = session.applyCollisionConstrainedCameraInput(blockedEnvelope);

  assert.equal(blocked.collided, true);
  assert.deepEqual(blocked.blockedAxes, ['x', 'z']);
  assert.deepEqual(blocked.snapshot.after.pose.position, blocked.snapshot.before.pose.position);
  assert.ok(blocked.snapshot.attempted.pose.position[2] < -90);
  assert.equal(blocked.snapshot.after.pose.yawDegrees, 10);
  assert.equal(blocked.snapshot.after.pose.pitchDegrees, -2);
  assert.ok(blocked.worldHash.startsWith('fnv1a64:'));
  assert.ok(blocked.collisionProjectionHash.startsWith('fnv1a64:'));
  assert.ok(blocked.movementHash.startsWith('fnv1a64:'));

  const lateralEnvelope: CollisionConstrainedCameraInputEnvelope = {
    ...blockedEnvelope,
    input: {
      moveForward: 0,
      moveRight: 1,
      moveUp: 0,
      yawDeltaDegrees: 0,
      pitchDeltaDegrees: 0,
      dtSeconds: 1,
      moveSpeedUnitsPerSecond: 1,
    },
    tick: 2,
  };
  const lateral = session.applyCollisionConstrainedCameraInput(lateralEnvelope);

  assert.equal(lateral.collided, false);
  assert.deepEqual(lateral.blockedAxes, []);
  assert.ok(lateral.snapshot.after.pose.position[0] > lateral.snapshot.before.pose.position[0]);
  assert.notDeepEqual(lateral.snapshot.after.pose.position, lateral.snapshot.before.pose.position);

  const telemetry = session.readTelemetry();
  assert.equal(telemetry.replayRecords.at(-1)?.kind, 'applyCollisionConstrainedCameraInput');
});

void test('collision-constrained camera movement is horizontal and target-obstacle constrained', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const collisionShape = { halfExtents: [0.25, 0.7, 0.25] as const };
  const collisionPolicy = { mode: 'axis_separable_slide' as const, maxIterations: 3 };
  const camera = session.createCamera({
    initialPose: {
      position: [0, 1.62, 0],
      yawDegrees: 0,
      pitchDegrees: 55,
    },
    projection: {
      fovYDegrees: 60,
      near: 0.1,
      far: 100,
    },
    viewport: {
      width: 1280,
      height: 720,
    },
  }).snapshot.camera;

  const intoTarget = session.applyCollisionConstrainedCameraInput({
    camera,
    grid: 1,
    input: {
      moveForward: 1,
      moveRight: 0,
      moveUp: 0,
      yawDeltaDegrees: 0,
      pitchDeltaDegrees: 0,
      dtSeconds: 1,
      moveSpeedUnitsPerSecond: 2,
    },
    tick: 1,
    shape: collisionShape,
    policy: collisionPolicy,
  });

  assert.equal(intoTarget.collided, true);
  assert.deepEqual(intoTarget.blockedAxes, ['z']);
  assert.ok(Math.abs(intoTarget.snapshot.attempted.pose.position[1] - 1.62) < 0.00001);
  assert.ok(Math.abs(intoTarget.snapshot.after.pose.position[1] - 1.62) < 0.00001);

  const yawedCamera = session.createCamera({
    initialPose: {
      position: [0, 1.62, 0],
      yawDegrees: 45,
      pitchDegrees: 55,
    },
    projection: {
      fovYDegrees: 60,
      near: 0.1,
      far: 100,
    },
    viewport: {
      width: 1280,
      height: 720,
    },
  }).snapshot.camera;

  const yawedForward = session.applyCollisionConstrainedCameraInput({
    camera: yawedCamera,
    grid: 1,
    input: {
      moveForward: 1,
      moveRight: 0,
      moveUp: 0,
      yawDeltaDegrees: 0,
      pitchDeltaDegrees: 0,
      dtSeconds: 0.1,
      moveSpeedUnitsPerSecond: 2,
    },
    tick: 1,
    shape: collisionShape,
    policy: collisionPolicy,
  });

  assert.equal(yawedForward.collided, false);
  assert.ok(yawedForward.snapshot.after.pose.position[0] > 0);
  assert.ok(yawedForward.snapshot.after.pose.position[2] < 0);
  assert.ok(Math.abs(yawedForward.snapshot.after.pose.position[1] - 1.62) < 0.00001);
});

void test('collision-constrained camera movement uses same-tick look deltas for forward movement', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const camera = session.createCamera({
    initialPose: {
      position: [0, 1.62, 1.5],
      yawDegrees: 0,
      pitchDegrees: 0,
    },
    projection: {
      fovYDegrees: 60,
      near: 0.1,
      far: 100,
    },
    viewport: {
      width: 1280,
      height: 720,
    },
  }).snapshot.camera;

  const moved = session.applyCollisionConstrainedCameraInput({
    camera,
    grid: 1,
    input: {
      moveForward: 1,
      moveRight: 0,
      moveUp: 0,
      yawDeltaDegrees: 45,
      pitchDeltaDegrees: 0,
      dtSeconds: 0.1,
      moveSpeedUnitsPerSecond: 2,
    },
    tick: 1,
    shape: { halfExtents: [0.25, 0.7, 0.25] },
    policy: { mode: 'axis_separable_slide', maxIterations: 3 },
  });

  assert.equal(moved.collided, false);
  assert.equal(moved.snapshot.after.pose.yawDegrees, 45);
  assert.ok(moved.snapshot.after.pose.position[0] > 0);
  assert.ok(moved.snapshot.after.pose.position[2] < 1.5);
  assert.ok(Math.abs(moved.snapshot.after.pose.position[1] - 1.62) < 0.00001);
});

void test('RuntimeSession exposes the generated tunnel fixture readout and fail-closed operations', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());

  const readout = session.readGeneratedTunnelReadout({ presetId: 'tiny-enclosed', seed: 17 });
  assert.equal(readout.status, 'available');
  assert.equal(readout.generator.generatorId, 'asha.tunnel.enclosed.v1');
  assert.equal(readout.generator.presetId, 'tiny-enclosed');
  assert.equal(readout.generator.seed, 17);
  assert.equal(readout.generator.configHash, 'e1d156c6b55137a7');
  assert.equal(readout.generator.outputHash, 'a9b504096397f5b4');
  assert.equal(readout.replayHash, 'fnv1a64:0821a0c2aea17dff');
  assert.deepEqual(readout.volume.tunnelDims, [5, 4, 9]);
  assert.equal(readout.volume.solidVoxels, 138);
  assert.equal(readout.corridors.count, 1);
  assert.equal(readout.rooms.count, 0);
  assert.deepEqual(readout.spawnMarkers.map((marker) => marker.id), ['player_start', 'exit_hint']);
  assert.deepEqual(readout.materials.map((material) => `${material.role}:${material.material}`), [
    'wall:1',
    'floor:2',
    'accent:3',
  ]);
  assert.equal(readout.renderProjection.hash, 'fnv1a64:21eb8696f6f3b5c4');
  assert.equal(readout.collisionProjection.hash, 'fnv1a64:78b242163cf67524');

  const operation = session.requestGeneratedTunnelOperation({
    operation: 'regenerate',
    presetId: 'tiny-enclosed',
    seed: 17,
  });
  assert.equal(operation.status, 'unsupported');
  assert.equal(operation.reason, 'generated_tunnel_operation_not_wired');
  assert.equal('payload' in operation, false);
  assert.equal(session.readTelemetry().replayRecords.at(-1)?.kind, 'requestGeneratedTunnelOperation');

  assert.throws(
    () => session.readGeneratedTunnelReadout({ presetId: 'tiny-enclosed', seed: 18 }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
});

void test('RuntimeSession exposes fire combat health readouts from typed action intents', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());
  const camera = session.createCamera({
    initialPose: { position: [2.5, 1.5, 1.5], yawDegrees: 180, pitchDegrees: 0 },
    projection: { fovYDegrees: 60, near: 0.1, far: 100 },
    viewport: { width: 1280, height: 720 },
  }).snapshot.camera;

  const receipt = session.submitRuntimeActionIntent({
    kind: 'runtime_action_intent.v0',
    action: 'primary_fire',
    phase: 'pressed',
    camera,
    tick: 7,
    source: 'programmatic',
    pressed: true,
  });

  assert.equal(receipt.accepted, true);
  assert.equal(receipt.status, 'accepted');
  assert.equal(receipt.rejection, null);
  assert.equal(receipt.combatReadout?.outcome.kind, 'hit');
  assert.equal(receipt.combatReadout?.outcome.kind === 'hit' ? receipt.combatReadout.outcome.target : null, 20);
  assert.equal(receipt.combatReadout?.outcome.kind === 'hit' ? receipt.combatReadout.outcome.distance : null, 3.5);
  assert.equal(receipt.combatReadout?.health[0]?.current, 0);
  assert.equal(receipt.combatReadout?.health[0]?.max, 40);
  assert.equal(receipt.combatReadout?.health[0]?.dead, true);
  assert.deepEqual(receipt.combatReadout?.events.map((event) => event.kind), [
    'fire_hit',
    'damage_applied',
    'entity_defeated',
  ]);
  assert.equal(receipt.combatReadout?.healthHash, '3c89045230f2d9d9');
  assert.equal(receipt.combatReadout?.replayHash, '6b133026c511b0f5');
  assert.equal(receipt.combatReadout?.authority.source, 'reference_fixture');
  assert.equal('payload' in receipt, false);

  const miss = session.readCombatReadout({ scenario: 'generated_tunnel_geometry_blocked_miss' });
  assert.equal(miss.outcome.kind, 'miss');
  assert.equal(miss.authority.source, 'reference_fixture');
  assert.equal(miss.outcome.kind === 'miss' ? miss.outcome.reason : null, 'geometryBlocked');
  assert.deepEqual(miss.events.map((event) => event.kind), ['fire_missed']);
  assert.equal(miss.health[0]?.current, 100);
  assert.equal(miss.health[0]?.dead, false);
  assert.equal(miss.healthHash, '56b1331c0f202ff1');
  assert.equal(miss.replayHash, '3b1e1a9897571bc4');

  const useReceipt = session.submitRuntimeActionIntent({
    kind: 'runtime_action_intent.v0',
    action: 'use',
    phase: 'pressed',
    camera,
    tick: 8,
    source: 'programmatic',
    pressed: true,
  });
  assert.equal(useReceipt.accepted, false);
  assert.equal(useReceipt.status, 'unsupported');
  assert.equal(useReceipt.rejection?.reason, 'combat_runtime_not_wired');
});

void test('RuntimeSession exposes read-only nav projection, path, and policy view readouts', () => {
  const session = createMockRuntimeSession();
  session.initialize(sessionInput());

  const projection = session.readNavProjection();
  assert.equal(projection.id, 'generated_tunnel_nav_projection');
  assert.equal(projection.available, true);
  assert.equal(projection.walkableCells, 66);
  assert.equal(projection.projectionHash, 'd1f6ac3e051d6b6e');

  const reachable = session.queryNavPath({ scenario: 'generated_tunnel_reachable' });
  assert.equal(reachable.outcome, 'reached');
  assert.equal(reachable.visited, 21);
  assert.equal(reachable.path.length, 9);
  assert.deepEqual(reachable.path[0], [3, 1, 7]);
  assert.deepEqual(reachable.path.at(-1), [1, 1, 1]);
  assert.equal(reachable.pathHash, 'e8e1ea7a09811ced');

  const noPath = session.queryNavPath({ scenario: 'generated_tunnel_no_path' });
  assert.equal(noPath.outcome, 'no_path');
  assert.equal(noPath.rejectionReason, 'blocked');
  assert.deepEqual(noPath.path, []);
  assert.equal(noPath.pathHash, 'a8c7f832281a39c5');

  const policyView = session.readNavPolicyView();
  assert.equal(policyView.kind, 'nav_policy_view.v0');
  assert.equal(policyView.readOnly, true);
  assert.equal(policyView.proposalOnly, true);
  assert.equal('mutate' in policyView, false);
  assert.equal('applyPath' in policyView, false);
  assert.equal(policyView.latestPath.pathHash, reachable.pathHash);

  assert.throws(
    () => session.queryNavPath({ maxVisited: 0 }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
});
