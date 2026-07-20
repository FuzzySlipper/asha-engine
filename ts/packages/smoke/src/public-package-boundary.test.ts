import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  BrowserFpsResolvedActionConsumer,
  BrowserInputHost,
  RuntimeBridgeError,
  type CameraCreateRequest,
  type CollisionConstrainedCameraInputEnvelope,
} from '@asha/runtime-bridge';
import {
  GENERATED_TUNNEL_FIRE_HIT_READOUT,
  type RuntimeActionIntentEnvelope,
} from '@asha/runtime-session';
import {
  REFERENCE_RUNTIME_BACKEND_PROFILE,
  createMockRuntimeSession,
} from '@asha/runtime-bridge/reference';
import {
  readDefaultFpsGameplayPreset,
  readFpsGameplayPresetCatalog,
} from '@asha/catalog-core';
import { buildHudProjection, hudControlToIntent } from '@asha/ui-dom';

function sessionInput() {
  return {
    sessionId: 'runtime-session.public-package-boundary',
    seed: 17,
    project: {
      gameId: 'provider-boundary-fixture',
      workspaceId: 'workspace.local',
    },
  };
}

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

void test('public package roots compose RuntimeSession readouts and HUD/menu projection', () => {
  const session = createMockRuntimeSession();
  const initialized = session.initialize(sessionInput());
  assert.equal(initialized.identity.mode, 'reference');
  assert.equal(REFERENCE_RUNTIME_BACKEND_PROFILE.productAuthority, false);
  assert.ok(REFERENCE_RUNTIME_BACKEND_PROFILE.nonClaims.includes('not_product_authority'));
  assert.ok(initialized.identity.nonClaims.includes('not_arbitrary_json_bridge'));
  assert.ok(initialized.identity.nonClaims.includes('not_product_authority'));

  const gameplayPreset = readDefaultFpsGameplayPreset();
  assert.equal(gameplayPreset.kind, 'fps_gameplay_preset_readout.v0');
  assert.equal(gameplayPreset.preset.playerController.moveSpeedUnitsPerSecond, 3);
  assert.equal(gameplayPreset.preset.weapon.damage, 40);
  assert.equal(gameplayPreset.preset.encounter.presetId, 'generated-tunnel-small-encounter');
  assert.equal(gameplayPreset.hashes.presetHash, 'fnv1a64:b39b8794318889a7');
  const gameplayCatalog = readFpsGameplayPresetCatalog();
  assert.equal(gameplayCatalog.hashes.defaultPresetHash, gameplayPreset.hashes.presetHash);
  assert.ok(gameplayCatalog.consumerOwnership.gameOwned.includes('playerController'));
  assert.ok(gameplayCatalog.consumerOwnership.engineOwned.includes('runtimeAuthority'));

  const camera = session.createCamera(cameraRequest).snapshot.camera;
  const fpsInput = new BrowserFpsResolvedActionConsumer();
  const inputHost = new BrowserInputHost({
    session,
    onResolvedAction: (action) => fpsInput.accept(action),
  });

  inputHost.handleKeyDown({ code: 'KeyW' });
  inputHost.setPointerLockActive(true);
  inputHost.handleMouseMove({ movementX: 6, movementY: -2 });
  const frame = fpsInput.drain();
  const motion = session.applyFirstPersonCameraInput({
    camera,
    tick: 1,
    input: {
      moveForward: frame.moveForward,
      moveRight: frame.moveRight,
      moveUp: 0,
      yawDeltaDegrees: frame.yawDeltaPixels * 0.1,
      pitchDeltaDegrees: -frame.pitchDeltaPixels * 0.1,
      dtSeconds: 1 / 60,
      moveSpeedUnitsPerSecond: 3,
    },
  });
  assert.equal(motion.snapshot.tick, 1);
  assert.notDeepEqual(motion.snapshot.pose.position, cameraRequest.initialPose.position);

  const collisionEnvelope: CollisionConstrainedCameraInputEnvelope = {
    camera: motion.snapshot.camera,
    grid: 1,
    movementMode: 'grounded',
    input: {
      moveForward: 1,
      moveRight: 0,
      moveUp: 0,
      yawDeltaDegrees: 0,
      pitchDeltaDegrees: 0,
      dtSeconds: 1,
      moveSpeedUnitsPerSecond: 99,
    },
    tick: 2,
    shape: {
      halfExtents: [0.25, 0.25, 0.25],
    },
    policy: {
      mode: 'axis_separable_slide',
      maxIterations: 3,
    },
  };
  const collision = session.applyCollisionConstrainedCameraInput(collisionEnvelope);
  assert.equal(collision.collided, true);
  assert.deepEqual(collision.blockedAxes, ['z']);
  assert.equal(collision.snapshot.collision.movementMode, 'grounded');
  assert.equal(collision.snapshot.after.pose.position[1], collision.snapshot.before.pose.position[1]);
  assert.ok(collision.collisionProjectionHash.startsWith('fnv1a64:'));

  const encounter = session.readEncounterDirector();
  assert.equal(encounter.kind, 'runtime_session.encounter_director.v0');
  assert.equal(encounter.state.status, 'pending');
  assert.equal(encounter.state.pendingEnemyCount, 1);
  assert.equal(encounter.config.source, 'project_bundle.encounter_preset');
  assert.equal(encounter.spawns[0]?.spawnMarker.markerId, 'exit_hint');
  const encounterActivated = session.requestEncounterTransition({
    kind: 'runtime_session.encounter_transition_request.v0',
    presetId: 'generated-tunnel-small-encounter',
    action: 'activate',
  });
  assert.equal(encounterActivated.accepted, true);
  assert.equal(encounterActivated.after.state.status, 'active');
  assert.equal(encounterActivated.after.state.activeEnemyCount, 1);
  assert.equal(encounterActivated.event?.kind, 'runtime_encounter.activated.v0');

  const primaryFireIntent: RuntimeActionIntentEnvelope = {
    kind: 'runtime_action_intent.v0',
    action: 'primary_fire',
    phase: 'pressed',
    camera: motion.snapshot.camera,
    tick: 7,
    source: 'programmatic',
    pressed: true,
  };
  const primaryFire = session.submitRuntimeActionIntent(primaryFireIntent);
  assert.equal(primaryFire.accepted, true);
  assert.equal(primaryFire.combatReadout?.outcome.kind, 'hit');
  assert.deepEqual(primaryFire.combatReadout?.outcome, GENERATED_TUNNEL_FIRE_HIT_READOUT.outcome);
  const health = primaryFire.combatReadout?.health[0];
  assert.ok(health);
  assert.equal(health.dead, true);

  const combatFeedback = session.readCombatFeedbackProjection({
    scenario: 'generated_tunnel_fire_hit',
    camera: motion.snapshot.camera,
  });
  assert.equal(combatFeedback.kind, 'combat_feedback_projection.v0');
  assert.equal(combatFeedback.marker.tone, 'hit');
  assert.equal(combatFeedback.notifications.at(-1)?.eventKind, 'entity_defeated');
  assert.equal(combatFeedback.hud.status[0]?.text, 'Entity 20 defeated');
  assert.equal(
    combatFeedback.debug.fixturePath,
    'harness/fixtures/combat-feedback/generated-tunnel-hit-feedback.snapshot.txt',
  );
  assert.ok(combatFeedback.hashes.projectionHash.startsWith('fnv1a64:'));

  const unsupportedUse = session.submitRuntimeActionIntent({
    kind: 'runtime_action_intent.v0',
    action: 'use',
    phase: 'pressed',
    camera: motion.snapshot.camera,
    tick: 8,
    source: 'programmatic',
    pressed: true,
  });
  assert.equal(unsupportedUse.status, 'unsupported');
  assert.equal(unsupportedUse.rejection?.reason, 'combat_runtime_not_wired');
  assert.equal('payload' in unsupportedUse, false);

  const navProjection = session.readNavProjection();
  assert.equal(navProjection.available, true);
  assert.equal(navProjection.projectionHash, '59b4093625b10e49');
  const reachable = session.queryNavPath({ scenario: 'generated_tunnel_reachable' });
  assert.equal(reachable.outcome, 'reached');
  assert.equal(reachable.pathHash, '09ed0284f7c175e1');
  const blocked = session.queryNavPath({ scenario: 'generated_tunnel_no_path' });
  assert.equal(blocked.outcome, 'no_path');
  assert.equal(blocked.rejectionReason, 'blocked');
  const policyView = session.readNavPolicyView();
  assert.equal(policyView.readOnly, true);
  assert.equal(policyView.proposalOnly, true);
  assert.equal('mutate' in policyView, false);
  assert.equal('applyPath' in policyView, false);

  const lifecycle = session.readLifecycleStatus();
  assert.equal(lifecycle.kind, 'runtime_session.lifecycle_status.v0');
  assert.equal(lifecycle.outcome.kind, 'won');
  assert.equal(lifecycle.enemy.dead, true);
  assert.equal(lifecycle.enemy.health.current, 0);
  assert.equal(lifecycle.hashes.lifecycleHash, 'fnv1a64:5fbf190733451da1');
  const playerLossFixture = session.readLifecycleStatus({ scenario: 'generated_tunnel_player_defeated' });
  assert.equal(playerLossFixture.outcome.kind, 'lost');
  assert.equal(playerLossFixture.player.dead, true);

  const encounterCleared = session.requestEncounterTransition({
    kind: 'runtime_session.encounter_transition_request.v0',
    presetId: 'generated-tunnel-small-encounter',
    action: 'sync_lifecycle',
  });
  assert.equal(encounterCleared.accepted, true);
  assert.equal(encounterCleared.after.state.status, 'cleared');
  assert.equal(encounterCleared.after.state.defeatedEnemyCount, 1);
  const lifecycleAfterEncounterSync = session.readLifecycleStatus();

  const hud = buildHudProjection({
    health: lifecycleAfterEncounterSync.player.health,
    status: [
      { id: 'lifecycle', tone: 'info', text: lifecycleAfterEncounterSync.outcome.label },
      ...combatFeedback.hud.status,
    ],
    nonClaims: initialized.identity.nonClaims,
    menuOpen: true,
  });
  assert.equal(hud.kind, 'hud_projection.v0');
  assert.equal(hud.health.label, 'Health 100/100');
  assert.equal(hud.status.some((status) => status.id === 'combat-feedback'), true);
  const restartIntent = hudControlToIntent('hud-restart');
  assert.deepEqual(restartIntent, { kind: 'runtime.restart_session_intent', source: 'hud_menu' });
  if (restartIntent?.kind !== 'runtime.restart_session_intent') {
    throw new Error('hud-restart did not produce a runtime restart intent');
  }
  const restartReceipt = session.requestSessionRestart({
    ...restartIntent,
    requireTerminal: true,
    expectedSessionHash: lifecycleAfterEncounterSync.sessionHash,
  });
  assert.equal(restartReceipt.accepted, true);
  assert.equal(restartReceipt.statusAfter.outcome.kind, 'in_progress');
  assert.equal(restartReceipt.statusAfter.reset.resetHash, lifecycleAfterEncounterSync.reset.resetHash);
  assert.deepEqual(hudControlToIntent('hud-options'), {
    kind: 'ui.open_options_intent',
    source: 'hud_menu',
  });

  assert.throws(
    () => session.queryNavPath({ maxVisited: 0 }),
    (error: unknown) => error instanceof RuntimeBridgeError && error.kind === 'invalid_input',
  );
});

void test('browser condition imports runtime bridge without native-only exports', () => {
  const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  const boundaryProbe = `
    const surface = await import('@asha/runtime-bridge');
    const reference = await import('@asha/runtime-bridge/reference');
    const required = ['BrowserInputHost', 'BrowserFpsResolvedActionConsumer', 'RuntimeBridgeError'];
    const referenceRequired = ['createMockRuntimeSession', 'createMockRuntimeBridge', 'REFERENCE_RUNTIME_BACKEND_PROFILE'];
    const forbidden = ['NativeRuntimeBridge', 'createNativeRuntimeBridge', 'NATIVE_WIRED_OPERATIONS', 'createMockRuntimeSession', 'createMockRuntimeBridge'];
    const missing = required.filter((name) => !(name in surface));
    const referenceMissing = referenceRequired.filter((name) => !(name in reference));
    const leaked = forbidden.filter((name) => name in surface);
    if (missing.length > 0 || referenceMissing.length > 0 || leaked.length > 0) {
      throw new Error(JSON.stringify({ missing, referenceMissing, leaked }));
    }
  `;
  execFileSync(process.execPath, ['--conditions=browser', '--input-type=module', '--eval', boundaryProbe], {
    cwd: packageRoot,
    stdio: 'pipe',
  });
});
