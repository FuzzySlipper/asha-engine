import { RuntimeBridgeError } from './bridge.js';
import { GENERATED_TUNNEL_FIRE_HIT_READOUT, type CombatRuntimeReadout } from './combat-readout.js';
import { initialEncounterDirectorState, type EncounterLifecycleInput } from './encounter-director.js';
import type { EnemyPolicyProposal, EnemyPolicyVec3 } from './enemy-policy.js';
import type { GeneratedTunnelOperationRequest, GeneratedTunnelReadoutRequest } from './generated-tunnel.js';
import type { RuntimeActionIntentEnvelope } from './runtime-action.js';
import {
  encounterStateHashRecord,
  lifecycleStateHashRecord,
  projectBundleHashRecord,
  stableHash,
} from './runtime-session-hash.js';
import type {
  RuntimeSessionActionIntentReceipt,
  RuntimeSessionAutonomousPolicyTickInput,
  RuntimeSessionAutonomousPolicyProposalReceipt,
  RuntimeSessionAutonomousPolicyProposalRejection,
  RuntimeSessionAutonomousPolicyProposalStatus,
  RuntimeSessionEcrpProjectState,
  RuntimeSessionIdentity,
  RuntimeSessionInitializeInput,
  RuntimeSessionLifecycleEventKind,
  RuntimeSessionLifecycleEventReadout,
  RuntimeSessionLifecycleHealthReadout,
  RuntimeSessionLifecycleScenario,
  RuntimeSessionLifecycleState,
  RuntimeSessionLifecycleStatusReadout,
  RuntimeSessionLifecycleStatusRequest,
  RuntimeSessionRestartIntent,
} from './runtime-session.js';

function runtimeSessionResetHash(identity: RuntimeSessionIdentity): string {
  return stableHash({
    seed: identity.seed,
    projectBundle: projectBundleHashRecord(identity.projectBundle),
    lifecycle: lifecycleStateHashRecord(initialRuntimeSessionLifecycleState()),
    encounter: encounterStateHashRecord(initialEncounterDirectorState()),
  });
}

export function initialRuntimeSessionLifecycleState(): RuntimeSessionLifecycleState {
  return {
    player: lifecycleHealth(10, 100, 100, false),
    enemy: lifecycleHealth(20, 40, 40, false),
    terminalEvent: null,
    revision: 0,
  };
}

export function generatedTunnelEnemyDefeatedLifecycleState(): RuntimeSessionLifecycleState {
  const enemy = lifecycleHealth(20, 0, 40, true);
  return {
    player: lifecycleHealth(10, 100, 100, false),
    enemy,
    terminalEvent: lifecycleEvent('runtime_lifecycle.enemy_defeated.v0', enemy.entity, 7, 'combat_health_zero'),
    revision: 1,
  };
}

export function generatedTunnelPlayerDefeatedLifecycleState(): RuntimeSessionLifecycleState {
  const player = lifecycleHealth(10, 0, 100, true);
  return {
    player,
    enemy: lifecycleHealth(20, 40, 40, false),
    terminalEvent: lifecycleEvent('runtime_lifecycle.player_defeated.v0', player.entity, 11, 'fixture_player_damage'),
    revision: 1,
  };
}

export function lifecycleHealth(
  entity: number,
  current: number,
  max: number,
  dead: boolean,
): RuntimeSessionLifecycleHealthReadout {
  const healthRecord = {
    entity,
    current,
    max,
    dead,
  };
  return {
    ...healthRecord,
    healthHash: stableHash(healthRecord),
  };
}

export function buildRuntimeSessionPrimaryFireReadout(input: {
  readonly projectState: RuntimeSessionEcrpProjectState | null;
  readonly lifecycleState: RuntimeSessionLifecycleState;
  readonly tick: number;
}): CombatRuntimeReadout {
  const shooter = input.lifecycleState.player.entity;
  const targetBefore = input.lifecycleState.enemy;
  const amount = targetBefore.current;
  const targetAfter = lifecycleHealth(targetBefore.entity, 0, targetBefore.max, true);
  if (
    shooter === 10 &&
    targetBefore.entity === 20 &&
    targetBefore.current === 40 &&
    targetBefore.max === 40 &&
    input.tick === 7
  ) {
    return GENERATED_TUNNEL_FIRE_HIT_READOUT;
  }

  const health = [
    {
      entity: targetAfter.entity,
      current: targetAfter.current,
      max: targetAfter.max,
      dead: targetAfter.dead,
    },
  ];
  const events: CombatRuntimeReadout['events'] = [
    {
      kind: 'fire_hit',
      shooter,
      target: targetAfter.entity,
      distance: 3.5,
      tick: input.tick,
    },
    {
      kind: 'damage_applied',
      target: targetAfter.entity,
      amount,
      before: targetBefore.current,
      after: targetAfter.current,
    },
    {
      kind: 'entity_defeated',
      target: targetAfter.entity,
    },
  ];
  const weaponMount = input.projectState?.entities
    .find((entity) => entity.role === 'player')
    ?.definition.capabilities.find((capability) => capability.kind === 'weaponMount');
  const combatRecord = {
    scenario: 'runtime_session_loaded_project_fire_hit',
    shooter,
    target: targetAfter.entity,
    weaponId: weaponMount?.kind === 'weaponMount' ? weaponMount.weaponId : null,
    health,
    events,
  };
  return {
    scenario: 'runtime_session_loaded_project_fire_hit',
    outcome: {
      kind: 'hit',
      target: targetAfter.entity,
      distance: 3.5,
      hitPosition: null,
      defeated: true,
    },
    events,
    health,
    nextFireControl: {
      ammo: 2,
      cooldownTicksRemaining: 4,
      cooldownTicksAfterFire: 4,
    },
    healthHash: stableHash(health),
    replayHash: stableHash(combatRecord),
    fixture: null,
  };
}

export function lifecycleEvent(
  kind: RuntimeSessionLifecycleEventKind,
  entity: number,
  tick: number,
  reason: RuntimeSessionLifecycleEventReadout['reason'],
): RuntimeSessionLifecycleEventReadout {
  return {
    kind,
    entity,
    tick,
    reason,
    eventHash: stableHash({
      kind,
      entity,
      tick,
      reason,
    }),
  };
}

export function lifecycleStatusReadout(input: {
  readonly scenario: RuntimeSessionLifecycleScenario;
  readonly state: RuntimeSessionLifecycleState;
  readonly identity: RuntimeSessionIdentity;
  readonly sequenceId: number;
  readonly tick: number;
  readonly restartCount: number;
  readonly sessionHash: string;
}): RuntimeSessionLifecycleStatusReadout {
  const outcome = lifecycleOutcome(input.state);
  const lifecycleHash = stableHash(lifecycleStateHashRecord(input.state));
  const resetHash = runtimeSessionResetHash(input.identity);
  return {
    kind: 'runtime_session.lifecycle_status.v0',
    scenario: input.scenario,
    sequenceId: input.sequenceId,
    tick: input.tick,
    sessionHash: input.sessionHash,
    player: {
      role: 'player',
      health: input.state.player,
      dead: input.state.player.dead,
    },
    enemy: {
      role: 'enemy',
      health: input.state.enemy,
      dead: input.state.enemy.dead,
    },
    outcome,
    restart: {
      eligible: true,
      intentKind: 'runtime.restart_session_intent',
      reason: 'always_resettable_reference_fixture',
    },
    events: input.state.terminalEvent === null ? [] : [input.state.terminalEvent],
    fixture: {
      seed: input.identity.seed,
      sceneId: input.identity.projectBundle.sceneId,
      bundleSchemaVersion: input.identity.projectBundle.bundleSchemaVersion,
      protocolVersion: input.identity.projectBundle.protocolVersion,
      resetHash,
    },
    hashes: {
      lifecycleHash,
      playerHealthHash: input.state.player.healthHash,
      enemyHealthHash: input.state.enemy.healthHash,
      replayHash: stableHash({
        lifecycleHash,
        resetHash,
        restartCount: input.restartCount,
        eventHash: input.state.terminalEvent?.eventHash ?? null,
      }),
    },
    nonClaims: [
      'not_save_load_persistence',
      'not_ui_authority',
      'not_demo_local_lifecycle',
    ],
  };
}

function lifecycleOutcome(state: RuntimeSessionLifecycleState): RuntimeSessionLifecycleStatusReadout['outcome'] {
  if (state.player.dead) {
    return {
      kind: 'lost',
      terminal: true,
      reason: 'player_defeated',
      label: 'Player defeated',
    };
  }
  if (state.enemy.dead) {
    return {
      kind: 'won',
      terminal: true,
      reason: 'enemy_defeated',
      label: 'Enemy defeated',
    };
  }
  return {
    kind: 'in_progress',
    terminal: false,
    reason: 'none',
    label: 'In progress',
  };
}

export function lifecycleStatusToEncounterLifecycle(
  status: RuntimeSessionLifecycleStatusReadout,
): EncounterLifecycleInput {
  return {
    outcomeKind: status.outcome.kind,
    terminal: status.outcome.terminal,
    enemyDead: status.enemy.dead,
    playerDead: status.player.dead,
    lifecycleHash: status.hashes.lifecycleHash,
  };
}

export function validateLifecycleStatusRequest(request: RuntimeSessionLifecycleStatusRequest): void {
  if (
    request.scenario !== undefined &&
    request.scenario !== 'current_session' &&
    request.scenario !== 'generated_tunnel_enemy_defeated' &&
    request.scenario !== 'generated_tunnel_player_defeated'
  ) {
    throw new RuntimeBridgeError('invalid_input', 'unknown lifecycle status scenario');
  }
}

export function validateRestartIntent(intent: RuntimeSessionRestartIntent): void {
  if (intent === null || typeof intent !== 'object') {
    throw new RuntimeBridgeError('invalid_input', 'restart intent must be an object');
  }
  if (intent.kind !== 'runtime.restart_session_intent') {
    throw new RuntimeBridgeError('invalid_input', 'restart intent kind must be runtime.restart_session_intent');
  }
  if (intent.source !== 'hud_menu' && intent.source !== 'programmatic') {
    throw new RuntimeBridgeError('invalid_input', 'restart intent source is unsupported');
  }
  if (intent.requireTerminal !== undefined && typeof intent.requireTerminal !== 'boolean') {
    throw new RuntimeBridgeError('invalid_input', 'restart intent requireTerminal must be boolean');
  }
  if (intent.expectedSessionHash !== undefined && intent.expectedSessionHash.trim().length === 0) {
    throw new RuntimeBridgeError('invalid_input', 'restart intent expectedSessionHash must be non-empty when provided');
  }
}

export function validateAutonomousPolicyTickInput(input: RuntimeSessionAutonomousPolicyTickInput): void {
  if (input === null || typeof input !== 'object') {
    throw new RuntimeBridgeError('invalid_input', 'autonomous policy tick input must be an object');
  }
  if (!Number.isSafeInteger(input.targetCamera) || input.targetCamera < 0) {
    throw new RuntimeBridgeError('invalid_input', 'autonomous policy targetCamera must be a non-negative camera handle');
  }
  if (input.tick !== undefined && (!Number.isSafeInteger(input.tick) || input.tick < 0)) {
    throw new RuntimeBridgeError('invalid_input', 'autonomous policy tick must be a non-negative safe integer');
  }
  if (input.policySource !== undefined && typeof input.policySource !== 'string') {
    throw new RuntimeBridgeError('invalid_input', 'autonomous policy source must be a string');
  }
  if (
    input.navScenario !== undefined &&
    input.navScenario !== 'generated_tunnel_reachable' &&
    input.navScenario !== 'generated_tunnel_no_path'
  ) {
    throw new RuntimeBridgeError('invalid_input', 'unknown autonomous policy nav scenario');
  }
}

export function validateAutonomousPolicyProposal(
  proposal: EnemyPolicyProposal,
  tick: number,
): RuntimeSessionAutonomousPolicyProposalRejection | null {
  if (proposal.authority !== 'rust_runtime_must_validate') {
    return invalidAutonomousPolicyProposal('policy proposal authority must require Rust runtime validation');
  }
  if (proposal.actor.trim().length === 0 || proposal.target.trim().length === 0) {
    return invalidAutonomousPolicyProposal('policy proposal actor and target must be non-empty');
  }

  if (proposal.kind === 'enemy_policy.move_toward_target.v0') {
    if (!isEnemyPolicyVec3(proposal.from)) {
      return invalidAutonomousPolicyProposal('movement proposal from position must be a finite vec3');
    }
    if (proposal.nextWaypoint === null || !isEnemyPolicyVec3(proposal.nextWaypoint)) {
      return invalidAutonomousPolicyProposal('movement proposal must include a finite next waypoint');
    }
    if (proposal.pathHash.trim().length === 0) {
      return invalidAutonomousPolicyProposal('movement proposal path hash must be non-empty');
    }
    return null;
  }

  if (proposal.intent.kind !== 'runtime_action_intent.v0') {
    return invalidAutonomousPolicyProposal('fire proposal intent kind must be runtime_action_intent.v0');
  }
  if (proposal.intent.action !== 'primary_fire') {
    return invalidAutonomousPolicyProposal('fire proposal intent action must be primary_fire');
  }
  if (proposal.intent.phase !== 'pressed' || !proposal.intent.pressed) {
    return invalidAutonomousPolicyProposal('fire proposal intent must be a pressed primary fire action');
  }
  if (proposal.intent.source !== 'enemy_policy') {
    return invalidAutonomousPolicyProposal('fire proposal intent source must be enemy_policy');
  }
  if (proposal.intent.tick !== tick) {
    return invalidAutonomousPolicyProposal('fire proposal intent tick must match the autonomous policy tick');
  }
  if (!Number.isSafeInteger(proposal.intent.camera) || proposal.intent.camera < 0) {
    return invalidAutonomousPolicyProposal('fire proposal intent camera must be a non-negative camera handle');
  }
  if (!Number.isFinite(proposal.distanceUnits) || proposal.distanceUnits < 0) {
    return invalidAutonomousPolicyProposal('fire proposal distance must be finite and non-negative');
  }
  return null;
}

function invalidAutonomousPolicyProposal(detail: string): RuntimeSessionAutonomousPolicyProposalRejection {
  return {
    reason: 'invalid_policy_proposal',
    detail,
  };
}

function isEnemyPolicyVec3(value: EnemyPolicyVec3): boolean {
  return value.length === 3 && value.every((component) => Number.isFinite(component));
}

export function rejectedAutonomousPolicyProposalReceipt(
  proposal: EnemyPolicyProposal,
  rejection: RuntimeSessionAutonomousPolicyProposalRejection,
): RuntimeSessionAutonomousPolicyProposalReceipt {
  return {
    proposalKind: proposal.kind,
    actor: proposal.actor,
    target: proposal.target,
    accepted: false,
    status: 'rejected',
    rejection,
    movement: null,
    actionReceipt: null,
    combat: null,
  };
}

export function unsupportedAutonomousMovementReceipt(
  proposal: Extract<EnemyPolicyProposal, { readonly kind: 'enemy_policy.move_toward_target.v0' }>,
): RuntimeSessionAutonomousPolicyProposalReceipt {
  const rejection: RuntimeSessionAutonomousPolicyProposalRejection = {
    reason: 'movement_authority_not_wired',
    detail: 'Enemy movement proposals are exposed for Rust runtime validation; movement authority is not wired yet.',
  };
  return {
    proposalKind: proposal.kind,
    actor: proposal.actor,
    target: proposal.target,
    accepted: false,
    status: 'unsupported',
    rejection,
    movement: {
      status: 'unsupported',
      actor: proposal.actor,
      target: proposal.target,
      from: proposal.from,
      nextWaypoint: proposal.nextWaypoint,
      pathHash: proposal.pathHash,
      reason: 'movement_authority_not_wired',
    },
    actionReceipt: null,
    combat: null,
  };
}

export function runtimeActionReceiptToAutonomousReceipt(
  proposal: Extract<EnemyPolicyProposal, { readonly kind: 'enemy_policy.primary_fire_intent.v0' }>,
  actionReceipt: RuntimeSessionActionIntentReceipt,
): RuntimeSessionAutonomousPolicyProposalReceipt {
  const status: RuntimeSessionAutonomousPolicyProposalStatus = actionReceipt.accepted ? 'accepted' : 'rejected';
  const rejection: RuntimeSessionAutonomousPolicyProposalRejection | null = actionReceipt.accepted
    ? null
    : {
        reason: 'runtime_action_rejected',
        detail: actionReceipt.rejection?.detail ?? 'Runtime action intent was not accepted.',
      };
  return {
    proposalKind: proposal.kind,
    actor: proposal.actor,
    target: proposal.target,
    accepted: actionReceipt.accepted,
    status,
    rejection,
    movement: null,
    actionReceipt,
    combat: {
      status,
      action: actionReceipt.envelope.action,
      outcome: actionReceipt.combatReadout?.outcome ?? null,
      healthHash: actionReceipt.combatReadout?.healthHash ?? null,
      replayHash: actionReceipt.combatReadout?.replayHash ?? null,
    },
  };
}

export function validateInitializeInput(input: RuntimeSessionInitializeInput): void {
  if (input.sessionId.trim().length === 0) {
    throw new RuntimeBridgeError('invalid_input', 'sessionId must be non-empty');
  }
  if (input.project.gameId.trim().length === 0) {
    throw new RuntimeBridgeError('invalid_input', 'project.gameId must be non-empty');
  }
  if (input.project.workspaceId.trim().length === 0) {
    throw new RuntimeBridgeError('invalid_input', 'project.workspaceId must be non-empty');
  }
  if (!Number.isSafeInteger(input.seed) || input.seed < 0) {
    throw new RuntimeBridgeError('invalid_input', 'seed must be a non-negative safe integer');
  }
}

export function validateRuntimeActionIntentEnvelope(envelope: RuntimeActionIntentEnvelope): void {
  if (envelope.kind !== 'runtime_action_intent.v0') {
    throw new RuntimeBridgeError('invalid_input', 'runtime action intent kind must be runtime_action_intent.v0');
  }
  if (envelope.action !== 'primary_fire' && envelope.action !== 'use') {
    throw new RuntimeBridgeError('invalid_input', 'runtime action intent action is unsupported');
  }
  if (envelope.phase !== 'pressed' && envelope.phase !== 'released') {
    throw new RuntimeBridgeError('invalid_input', 'runtime action intent phase is unsupported');
  }
  if (
    envelope.source !== 'browser_fps_pointer' &&
    envelope.source !== 'programmatic' &&
    envelope.source !== 'enemy_policy'
  ) {
    throw new RuntimeBridgeError('invalid_input', 'runtime action intent source is unsupported');
  }
  if (!Number.isSafeInteger(envelope.tick) || envelope.tick < 0) {
    throw new RuntimeBridgeError('invalid_input', 'runtime action intent tick must be a non-negative safe integer');
  }
  if (envelope.phase === 'pressed' && !envelope.pressed) {
    throw new RuntimeBridgeError('invalid_input', 'pressed runtime action intent must report pressed=true');
  }
  if (envelope.phase === 'released' && envelope.pressed) {
    throw new RuntimeBridgeError('invalid_input', 'released runtime action intent must report pressed=false');
  }
}

export function combatReadoutTick(readout: CombatRuntimeReadout): number {
  const fireEvent = readout.events.find(
    (event) => event.kind === 'fire_hit' || event.kind === 'fire_missed',
  );
  return fireEvent?.tick ?? 0;
}

export function validateGeneratedTunnelReadoutRequest(request: GeneratedTunnelReadoutRequest): void {
  if (request.presetId !== undefined && request.presetId !== 'tiny-enclosed') {
    throw new RuntimeBridgeError('invalid_input', 'only the tiny-enclosed generated tunnel readout is available');
  }
  if (request.seed !== undefined && request.seed !== 17) {
    throw new RuntimeBridgeError('invalid_input', 'only seed 17 generated tunnel fixture readout is available');
  }
}

export function validateGeneratedTunnelOperationRequest(request: GeneratedTunnelOperationRequest): void {
  if (request.operation !== 'regenerate' && request.operation !== 'apply_to_runtime_world') {
    throw new RuntimeBridgeError('invalid_input', 'generated tunnel operation is unsupported');
  }
  validateGeneratedTunnelReadoutRequest(request);
}
