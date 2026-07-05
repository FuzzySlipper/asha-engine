import { cameraHandle, } from '@asha/contracts';
import { RuntimeBridgeError, frameCursor, } from './bridge.js';
import { GENERATED_TUNNEL_FIRE_HIT_READOUT, GENERATED_TUNNEL_FIRE_MISS_READOUT, } from './combat-readout.js';
import { buildCombatFeedbackProjection, defaultCombatFeedbackIntent, } from './combat-feedback.js';
import { TINY_GENERATED_TUNNEL_READOUT, } from './generated-tunnel.js';
import { createGeneratedTunnelEnemyPolicyFixture, validateEnemyPolicySource, } from './enemy-policy.js';
import { buildEncounterDirectorReadout, buildEncounterTransitionReceipt, initialEncounterDirectorState, transitionEncounterDirectorState, validateEncounterDirectorReadoutRequest, validateEncounterTransitionRequest, } from './encounter-director.js';
import { createMockRuntimeBridge } from './mock.js';
import { GENERATED_TUNNEL_NAV_POLICY_VIEW, GENERATED_TUNNEL_NAV_PROJECTION, GENERATED_TUNNEL_NO_PATH, GENERATED_TUNNEL_REACHABLE_PATH, } from './nav-readout.js';
export function createMockRuntimeSession(options = {}) {
    return new ReferenceRuntimeSessionFacade(options.bridge ?? createMockRuntimeBridge());
}
class ReferenceRuntimeSessionFacade {
    #bridge;
    #identity = null;
    #engine = null;
    #sequenceId = 0;
    #tick = 0;
    #acceptedCommandCount = 0;
    #rejectedCommandCount = 0;
    #restartCount = 0;
    #lifecycleState = initialRuntimeSessionLifecycleState();
    #encounterState = initialEncounterDirectorState();
    #ecrpProjectState = null;
    #replayRecords = [];
    constructor(bridge) {
        this.#bridge = bridge;
    }
    initialize(input) {
        validateInitializeInput(input);
        const engine = this.#bridge.initializeEngine({ seed: input.seed });
        const composition = this.#bridge.loadWorldBundle(input.projectBundle);
        this.#engine = engine;
        this.#identity = {
            sessionId: input.sessionId,
            mode: 'reference',
            seed: input.seed,
            project: input.project,
            projectBundle: input.projectBundle,
            nonClaims: referenceRuntimeSessionNonClaims(),
        };
        this.#sequenceId = 0;
        this.#tick = 0;
        this.#acceptedCommandCount = 0;
        this.#rejectedCommandCount = 0;
        this.#ecrpProjectState = buildEcrpProjectState(defaultRuntimeSessionEcrpProjectLoadInput(input));
        this.#lifecycleState = lifecycleStateFromEcrpProject(this.#ecrpProjectState);
        this.#encounterState = initialEncounterDirectorState();
        this.#replayRecords = [];
        this.#record('initialize');
        return this.#stateSummary(composition);
    }
    loadEcrpProject(input) {
        const identity = this.#requireInitialized('loadEcrpProject');
        const before = this.#sessionHash();
        const diagnostics = validateEcrpProjectLoadInput(input);
        this.#sequenceId += 1;
        if (diagnostics.length > 0) {
            this.#record('loadEcrpProject');
            return {
                kind: 'runtime_session.ecrp_project_load_receipt.v0',
                sequenceId: this.#sequenceId,
                accepted: false,
                diagnostics,
                entityCount: 0,
                bootstrapHash: null,
                sessionHashBefore: before,
                sessionHashAfter: this.#sessionHash(),
            };
        }
        const state = buildEcrpProjectState(input);
        this.#bridge.loadWorldBundle(input.projectBundle.runtimeRequest);
        this.#identity = {
            ...identity,
            project: input.projectBundle.project,
            projectBundle: input.projectBundle.runtimeRequest,
        };
        this.#ecrpProjectState = state;
        this.#lifecycleState = lifecycleStateFromEcrpProject(state);
        this.#record('loadEcrpProject');
        return {
            kind: 'runtime_session.ecrp_project_load_receipt.v0',
            sequenceId: this.#sequenceId,
            accepted: true,
            diagnostics: [],
            entityCount: state.entities.length,
            bootstrapHash: state.bootstrapHash,
            sessionHashBefore: before,
            sessionHashAfter: this.#sessionHash(),
        };
    }
    submitCommands(batch) {
        this.#requireInitialized('submitCommands');
        const before = this.#sessionHash();
        const result = this.#bridge.submitCommands(batch);
        this.#acceptedCommandCount += result.accepted;
        this.#rejectedCommandCount += result.rejected;
        this.#sequenceId += 1;
        this.#record('submitCommands');
        return {
            sequenceId: this.#sequenceId,
            batch,
            result,
            acceptedCommandCount: this.#acceptedCommandCount,
            rejectedCommandCount: this.#rejectedCommandCount,
            sessionHashBefore: before,
            sessionHashAfter: this.#sessionHash(),
        };
    }
    tick(input = {}) {
        this.#requireInitialized('tick');
        const nextTick = input.tick ?? this.#tick + 1;
        const step = this.#bridge.stepSimulation({ tick: nextTick });
        this.#tick = step.tick;
        this.#sequenceId += 1;
        this.#record('tick');
        return {
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            step,
            composition: this.#bridge.getCompositionStatus(),
            sessionHash: this.#sessionHash(),
        };
    }
    createCamera(request) {
        this.#requireInitialized('createCamera');
        const snapshot = this.#bridge.createCamera(request);
        this.#sequenceId += 1;
        this.#record('createCamera');
        return {
            sequenceId: this.#sequenceId,
            request,
            snapshot,
            sessionHash: this.#sessionHash(),
        };
    }
    applyFirstPersonCameraInput(envelope) {
        this.#requireInitialized('applyFirstPersonCameraInput');
        const before = this.#sessionHash();
        const snapshot = this.#bridge.applyFirstPersonCameraInput(envelope);
        this.#sequenceId += 1;
        this.#record('applyFirstPersonCameraInput');
        return {
            sequenceId: this.#sequenceId,
            envelope,
            snapshot,
            sessionHashBefore: before,
            sessionHashAfter: this.#sessionHash(),
        };
    }
    applyCollisionConstrainedCameraInput(envelope) {
        this.#requireInitialized('applyCollisionConstrainedCameraInput');
        const before = this.#sessionHash();
        const snapshot = this.#bridge.applyCollisionConstrainedCameraInput(envelope);
        this.#sequenceId += 1;
        this.#record('applyCollisionConstrainedCameraInput');
        return {
            sequenceId: this.#sequenceId,
            envelope,
            snapshot,
            collided: snapshot.collision.collided,
            blockedAxes: snapshot.collision.blockedAxes,
            worldHash: snapshot.collision.worldHash,
            collisionProjectionHash: snapshot.collision.collisionProjectionHash,
            movementHash: snapshot.movementHash,
            sessionHashBefore: before,
            sessionHashAfter: this.#sessionHash(),
        };
    }
    submitRuntimeActionIntent(envelope) {
        this.#requireInitialized('submitRuntimeActionIntent');
        validateRuntimeActionIntentEnvelope(envelope);
        const before = this.#sessionHash();
        this.#sequenceId += 1;
        this.#record('submitRuntimeActionIntent');
        const combatReadout = envelope.action === 'primary_fire' && envelope.phase === 'pressed'
            ? buildRuntimeSessionPrimaryFireReadout({
                projectState: this.#ecrpProjectState,
                lifecycleState: this.#lifecycleState,
                tick: envelope.tick,
            })
            : null;
        const accepted = combatReadout !== null || (envelope.action === 'primary_fire' && envelope.phase === 'released');
        if (combatReadout !== null) {
            this.#applyCombatLifecycleReadout(combatReadout, envelope.tick);
        }
        return {
            sequenceId: this.#sequenceId,
            envelope,
            accepted,
            status: accepted ? 'accepted' : 'unsupported',
            rejection: accepted
                ? null
                : {
                    reason: 'combat_runtime_not_wired',
                    detail: 'Only primary_fire press/release is wired in the #4051 reference combat slice.',
                },
            combatReadout,
            sessionHashBefore: before,
            sessionHashAfter: this.#sessionHash(),
        };
    }
    runAutonomousPolicyTick(input) {
        this.#requireInitialized('runAutonomousPolicyTick');
        validateAutonomousPolicyTickInput(input);
        const sequenceIdBefore = this.#sequenceId;
        const sessionHashBefore = this.#sessionHash();
        const step = this.tick(input.tick === undefined ? {} : { tick: input.tick });
        const navPath = this.queryNavPath({ scenario: input.navScenario ?? 'generated_tunnel_reachable' });
        const navPolicyView = {
            ...this.readNavPolicyView(),
            latestPath: navPath,
        };
        const sourceDiagnostics = input.policySource === undefined ? [] : validateEnemyPolicySource(input.policySource);
        const fixture = createGeneratedTunnelEnemyPolicyFixture({
            tick: step.tick,
            nav: navPolicyView,
            target: {
                ...(input.target ?? {}),
                camera: input.targetCamera,
            },
            ...(input.enemy === undefined ? {} : { enemy: input.enemy }),
            ...(input.combat === undefined ? {} : { combat: input.combat }),
        });
        const proposalValidationDiagnostics = [];
        const proposalReceipts = [];
        for (const proposal of fixture.frame.proposals) {
            const validation = validateAutonomousPolicyProposal(proposal, step.tick);
            if (validation !== null) {
                proposalValidationDiagnostics.push(validation);
                proposalReceipts.push(rejectedAutonomousPolicyProposalReceipt(proposal, validation));
                continue;
            }
            if (sourceDiagnostics.length > 0) {
                proposalReceipts.push(rejectedAutonomousPolicyProposalReceipt(proposal, {
                    reason: 'policy_source_forbidden_capability',
                    detail: `policy source referenced ${sourceDiagnostics.map((diagnostic) => diagnostic.token).join(', ')}`,
                }));
                continue;
            }
            if (proposal.kind === 'enemy_policy.move_toward_target.v0') {
                proposalReceipts.push(unsupportedAutonomousMovementReceipt(proposal));
                continue;
            }
            const actionReceipt = this.submitRuntimeActionIntent(proposal.intent);
            proposalReceipts.push(runtimeActionReceiptToAutonomousReceipt(proposal, actionReceipt));
        }
        this.#sequenceId += 1;
        this.#record('runAutonomousPolicyTick');
        const telemetry = this.readTelemetry();
        const movementSummary = proposalReceipts.find((receipt) => receipt.movement !== null)?.movement ?? null;
        const combatSummary = proposalReceipts.find((receipt) => receipt.combat !== null)?.combat ?? null;
        const acceptedRuntimeActionCount = proposalReceipts.filter((receipt) => receipt.actionReceipt?.accepted === true).length;
        const rejectedRuntimeActionCount = proposalReceipts.filter((receipt) => receipt.actionReceipt !== null && receipt.actionReceipt.accepted === false).length;
        const recordHashes = telemetry.replayRecords.map((record) => record.recordHash);
        const tickHash = stableHash({
            loopId: 'generated_tunnel_enemy_policy_loop.v0',
            tick: step.tick,
            proposalFrameHash: fixture.frame.proposalHash,
            receiptStatuses: proposalReceipts.map((receipt) => receipt.status),
            receiptRejections: proposalReceipts.map((receipt) => receipt.rejection?.reason ?? null),
            navPathHash: navPath.pathHash,
            replayRecordHashes: recordHashes,
            sequenceIdAfter: telemetry.sequenceId,
        });
        return {
            kind: 'runtime_session.autonomous_policy_tick.v0',
            loopId: 'generated_tunnel_enemy_policy_loop.v0',
            sequenceIdBefore,
            sequenceIdAfter: telemetry.sequenceId,
            sessionHashBefore,
            sessionHashAfter: telemetry.sessionHash,
            tick: step.tick,
            step,
            policy: {
                fixtureKind: fixture.kind,
                proposalFrame: fixture.frame,
                sourceChecked: input.policySource !== undefined,
                sourceDiagnostics,
                proposalValidationDiagnostics,
            },
            nav: {
                projectionHash: navPath.projection.projectionHash,
                pathHash: navPath.pathHash,
                outcome: navPath.outcome,
                visited: navPath.visited,
                pathLength: navPath.path.length,
            },
            proposalReceipts,
            proposalSummary: {
                acceptedProposalCount: proposalReceipts.filter((receipt) => receipt.status === 'accepted').length,
                rejectedProposalCount: proposalReceipts.filter((receipt) => receipt.status === 'rejected').length,
                unsupportedProposalCount: proposalReceipts.filter((receipt) => receipt.status === 'unsupported').length,
            },
            commandSummary: {
                acceptedCommandCount: telemetry.acceptedCommandCount,
                rejectedCommandCount: telemetry.rejectedCommandCount,
                acceptedRuntimeActionCount,
                rejectedRuntimeActionCount,
            },
            movementSummary,
            combatSummary,
            replay: {
                recordCount: telemetry.replayRecords.length,
                lastRecordKind: telemetry.replayRecords.at(-1)?.kind ?? null,
                recordHashes,
            },
            tickHash,
            nonClaims: [
                'not_generic_event_bus',
                'not_behavior_tree',
                'not_demo_local_authority',
                'movement_authority_not_wired',
            ],
        };
    }
    readLifecycleStatus(request = {}) {
        const identity = this.#requireInitialized('readLifecycleStatus');
        validateLifecycleStatusRequest(request);
        const scenario = request.scenario ?? 'current_session';
        const state = scenario === 'generated_tunnel_enemy_defeated'
            ? generatedTunnelEnemyDefeatedLifecycleState()
            : scenario === 'generated_tunnel_player_defeated'
                ? generatedTunnelPlayerDefeatedLifecycleState()
                : this.#lifecycleState;
        return lifecycleStatusReadout({
            scenario,
            state,
            identity,
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            restartCount: this.#restartCount,
            sessionHash: this.#sessionHash(),
        });
    }
    requestSessionRestart(intent) {
        this.#requireInitialized('requestSessionRestart');
        validateRestartIntent(intent);
        const statusBefore = this.readLifecycleStatus();
        const sessionHashBefore = this.#sessionHash();
        if (intent.expectedSessionHash !== undefined && intent.expectedSessionHash !== sessionHashBefore) {
            return this.#rejectSessionRestart(intent, statusBefore, sessionHashBefore, {
                reason: 'session_hash_mismatch',
                detail: 'Restart intent expectedSessionHash did not match the current RuntimeSession hash.',
            });
        }
        if (intent.requireTerminal === true && !statusBefore.outcome.terminal) {
            return this.#rejectSessionRestart(intent, statusBefore, sessionHashBefore, {
                reason: 'session_not_terminal',
                detail: 'Restart intent required a terminal win/loss lifecycle state.',
            });
        }
        const restart = this.restart();
        const statusAfter = this.readLifecycleStatus();
        return {
            kind: 'runtime_session.restart_receipt.v0',
            sequenceId: restart.sequenceId,
            intent,
            accepted: true,
            status: 'accepted',
            rejection: null,
            statusBefore,
            statusAfter,
            restart,
            sessionHashBefore,
            sessionHashAfter: restart.sessionHash,
            resetHash: statusAfter.fixture.resetHash,
        };
    }
    readEncounterDirector(request = {}) {
        const identity = this.#requireInitialized('readEncounterDirector');
        validateEncounterDirectorReadoutRequest(request);
        const lifecycle = this.#encounterLifecycleFromScenario(request.lifecycleScenario);
        return buildEncounterDirectorReadout({
            state: this.#encounterState,
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            sessionSeed: identity.seed,
            sessionHash: this.#sessionHash(),
            lifecycle,
        });
    }
    requestEncounterTransition(request) {
        this.#requireInitialized('requestEncounterTransition');
        const sessionHashBefore = this.#sessionHash();
        const validationRejection = validateEncounterTransitionRequest(request);
        const lifecycle = validationRejection === undefined
            ? this.#encounterLifecycleFromScenario(request.lifecycleScenario)
            : this.#encounterLifecycleFromScenario();
        const identity = this.#requireInitialized('requestEncounterTransition');
        const before = buildEncounterDirectorReadout({
            state: this.#encounterState,
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            sessionSeed: identity.seed,
            sessionHash: sessionHashBefore,
            lifecycle,
        });
        const result = validationRejection === undefined
            ? transitionEncounterDirectorState({
                state: this.#encounterState,
                action: request.action,
                lifecycle,
            })
            : {
                accepted: false,
                state: this.#encounterState,
                rejectionReason: validationRejection,
            };
        if (result.accepted) {
            this.#encounterState = result.state;
        }
        this.#sequenceId += 1;
        this.#record('requestEncounterTransition');
        const after = buildEncounterDirectorReadout({
            state: this.#encounterState,
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            sessionSeed: identity.seed,
            sessionHash: this.#sessionHash(),
            lifecycle,
        });
        return buildEncounterTransitionReceipt({
            request,
            sequenceId: this.#sequenceId,
            before,
            after,
            result,
            sessionHashBefore,
            sessionHashAfter: this.#sessionHash(),
        });
    }
    readCombatReadout(request = {}) {
        this.#requireInitialized('readCombatReadout');
        const scenario = request.scenario ?? 'generated_tunnel_fire_hit';
        switch (scenario) {
            case 'generated_tunnel_fire_hit':
                return GENERATED_TUNNEL_FIRE_HIT_READOUT;
            case 'generated_tunnel_geometry_blocked_miss':
                return GENERATED_TUNNEL_FIRE_MISS_READOUT;
            default:
                throw new RuntimeBridgeError('invalid_input', 'unknown combat readout scenario');
        }
    }
    readCombatFeedbackProjection(request = {}) {
        this.#requireInitialized('readCombatFeedbackProjection');
        const combatReadout = this.readCombatReadout(request);
        const cameraProjection = request.camera === undefined
            ? null
            : this.readCameraProjection({
                camera: request.camera,
                viewport: request.viewport ?? null,
            }).snapshot;
        return buildCombatFeedbackProjection({
            sequenceId: this.#sequenceId,
            ...defaultCombatFeedbackIntent({
                camera: request.camera ?? cameraHandle(0),
                tick: combatReadoutTick(combatReadout),
            }),
            combatReadout,
            camera: cameraProjection,
        });
    }
    readNavProjection() {
        this.#requireInitialized('readNavProjection');
        return GENERATED_TUNNEL_NAV_PROJECTION;
    }
    queryNavPath(request = {}) {
        this.#requireInitialized('queryNavPath');
        validateNavPathQueryRequest(request);
        return request.scenario === 'generated_tunnel_no_path' ? GENERATED_TUNNEL_NO_PATH : GENERATED_TUNNEL_REACHABLE_PATH;
    }
    readNavPolicyView() {
        this.#requireInitialized('readNavPolicyView');
        return GENERATED_TUNNEL_NAV_POLICY_VIEW;
    }
    readGeneratedTunnelReadout(request = {}) {
        this.#requireInitialized('readGeneratedTunnelReadout');
        validateGeneratedTunnelReadoutRequest(request);
        return TINY_GENERATED_TUNNEL_READOUT;
    }
    requestGeneratedTunnelOperation(request) {
        this.#requireInitialized('requestGeneratedTunnelOperation');
        validateGeneratedTunnelOperationRequest(request);
        const before = this.#sessionHash();
        this.#sequenceId += 1;
        this.#record('requestGeneratedTunnelOperation');
        return {
            sequenceId: this.#sequenceId,
            request,
            operation: request.operation,
            status: 'unsupported',
            reason: 'generated_tunnel_operation_not_wired',
            detail: 'Generated tunnel regenerate/apply operations are not runtime commands in this slice.',
            sessionHashBefore: before,
            sessionHashAfter: this.#sessionHash(),
        };
    }
    readEcrpRuntimeReadout() {
        const identity = this.#requireInitialized('readEcrpRuntimeReadout');
        return buildEcrpRuntimeReadout({
            identity,
            projectState: this.#ecrpProjectState,
            lifecycleState: this.#lifecycleState,
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            sessionHash: this.#sessionHash(),
        });
    }
    readCameraProjection(request) {
        this.#requireInitialized('readCameraProjection');
        const snapshot = this.#bridge.readCameraProjection(request);
        return {
            sequenceId: this.#sequenceId,
            request,
            snapshot,
            projectionHash: snapshot.projectionHash,
        };
    }
    readProjection() {
        this.#requireInitialized('readProjection');
        const cursor = frameCursor(this.#sequenceId);
        const frame = this.#bridge.readRenderDiffs(cursor);
        const composition = this.#bridge.getCompositionStatus();
        return {
            sequenceId: this.#sequenceId,
            cursor,
            frame,
            composition,
            renderDiffCount: frame.ops.length,
            projectionHash: stableHash({
                sequenceId: this.#sequenceId,
                composition: compositionHashRecord(composition),
                frame: renderFrameHashRecord(frame),
            }),
        };
    }
    readTelemetry() {
        this.#requireInitialized('readTelemetry');
        return {
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            composition: this.#bridge.getCompositionStatus(),
            acceptedCommandCount: this.#acceptedCommandCount,
            rejectedCommandCount: this.#rejectedCommandCount,
            restartCount: this.#restartCount,
            sessionHash: this.#sessionHash(),
            replayRecords: [...this.#replayRecords],
        };
    }
    restart() {
        const identity = this.#requireInitialized('restart');
        this.#bridge.unloadWorld();
        this.#bridge.initializeEngine({ seed: identity.seed });
        const composition = this.#bridge.loadWorldBundle(identity.projectBundle);
        this.#sequenceId += 1;
        this.#tick = 0;
        this.#acceptedCommandCount = 0;
        this.#rejectedCommandCount = 0;
        if (this.#ecrpProjectState !== null) {
            this.#lifecycleState = lifecycleStateFromEcrpProject(this.#ecrpProjectState);
        }
        else {
            this.#lifecycleState = initialRuntimeSessionLifecycleState();
        }
        this.#encounterState = initialEncounterDirectorState();
        this.#restartCount += 1;
        this.#record('restart');
        return {
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            composition,
            restartCount: this.#restartCount,
            sessionHash: this.#sessionHash(),
        };
    }
    #rejectSessionRestart(intent, statusBefore, sessionHashBefore, rejection) {
        this.#sequenceId += 1;
        this.#record('requestSessionRestart');
        const statusAfter = this.readLifecycleStatus();
        return {
            kind: 'runtime_session.restart_receipt.v0',
            sequenceId: this.#sequenceId,
            intent,
            accepted: false,
            status: 'rejected',
            rejection,
            statusBefore,
            statusAfter,
            restart: null,
            sessionHashBefore,
            sessionHashAfter: this.#sessionHash(),
            resetHash: statusAfter.fixture.resetHash,
        };
    }
    #applyCombatLifecycleReadout(readout, tick) {
        const defeated = readout.health.find((health) => health.dead);
        if (defeated === undefined || this.#lifecycleState.enemy.dead) {
            return;
        }
        const enemy = lifecycleHealth(this.#lifecycleState.enemy.entity, defeated.current, this.#lifecycleState.enemy.max, defeated.dead);
        const event = lifecycleEvent('runtime_lifecycle.enemy_defeated.v0', enemy.entity, tick, 'combat_health_zero');
        this.#lifecycleState = {
            player: this.#lifecycleState.player,
            enemy,
            terminalEvent: event,
            revision: this.#lifecycleState.revision + 1,
        };
        this.#record('lifecycleDeath');
    }
    #encounterLifecycleFromScenario(scenario) {
        const lifecycleScenario = scenario === undefined || scenario === 'active' ? 'current_session' : scenario;
        return lifecycleStatusToEncounterLifecycle(this.readLifecycleStatus({ scenario: lifecycleScenario }));
    }
    #requireInitialized(operation) {
        if (this.#identity === null || this.#engine === null) {
            throw new RuntimeBridgeError('not_initialized', `${operation} before RuntimeSession initialize`);
        }
        return this.#identity;
    }
    #stateSummary(composition) {
        const identity = this.#requireInitialized('stateSummary');
        return {
            identity,
            engine: this.#engine,
            composition,
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            sessionHash: this.#sessionHash(),
        };
    }
    #record(kind) {
        this.#replayRecords.push({
            sequenceId: this.#sequenceId,
            kind,
            recordHash: stableHash({
                kind,
                sequenceId: this.#sequenceId,
                tick: this.#tick,
                acceptedCommandCount: this.#acceptedCommandCount,
                rejectedCommandCount: this.#rejectedCommandCount,
                restartCount: this.#restartCount,
                lifecycle: lifecycleStateHashRecord(this.#lifecycleState),
                encounter: encounterStateHashRecord(this.#encounterState),
                composition: compositionHashRecord(this.#bridge.getCompositionStatus()),
            }),
        });
    }
    #sessionHash() {
        return stableHash({
            identity: this.#identity === null ? null : identityHashRecord(this.#identity),
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            acceptedCommandCount: this.#acceptedCommandCount,
            rejectedCommandCount: this.#rejectedCommandCount,
            restartCount: this.#restartCount,
            lifecycle: this.#identity === null ? null : lifecycleStateHashRecord(this.#lifecycleState),
            encounter: this.#identity === null ? null : encounterStateHashRecord(this.#encounterState),
            composition: this.#identity === null ? null : compositionHashRecord(this.#bridge.getCompositionStatus()),
        });
    }
}
function initialRuntimeSessionLifecycleState() {
    return {
        player: lifecycleHealth(10, 100, 100, false),
        enemy: lifecycleHealth(20, 40, 40, false),
        terminalEvent: null,
        revision: 0,
    };
}
function generatedTunnelEnemyDefeatedLifecycleState() {
    const enemy = lifecycleHealth(20, 0, 40, true);
    return {
        player: lifecycleHealth(10, 100, 100, false),
        enemy,
        terminalEvent: lifecycleEvent('runtime_lifecycle.enemy_defeated.v0', enemy.entity, 7, 'combat_health_zero'),
        revision: 1,
    };
}
function generatedTunnelPlayerDefeatedLifecycleState() {
    const player = lifecycleHealth(10, 0, 100, true);
    return {
        player,
        enemy: lifecycleHealth(20, 40, 40, false),
        terminalEvent: lifecycleEvent('runtime_lifecycle.player_defeated.v0', player.entity, 11, 'fixture_player_damage'),
        revision: 1,
    };
}
function lifecycleHealth(entity, current, max, dead) {
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
function buildRuntimeSessionPrimaryFireReadout(input) {
    const shooter = input.lifecycleState.player.entity;
    const targetBefore = input.lifecycleState.enemy;
    const amount = targetBefore.current;
    const targetAfter = lifecycleHealth(targetBefore.entity, 0, targetBefore.max, true);
    if (shooter === 10 &&
        targetBefore.entity === 20 &&
        targetBefore.current === 40 &&
        targetBefore.max === 40 &&
        input.tick === 7) {
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
    const events = [
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
function lifecycleEvent(kind, entity, tick, reason) {
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
function lifecycleStatusReadout(input) {
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
function lifecycleOutcome(state) {
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
function lifecycleStatusToEncounterLifecycle(status) {
    return {
        outcomeKind: status.outcome.kind,
        terminal: status.outcome.terminal,
        enemyDead: status.enemy.dead,
        playerDead: status.player.dead,
        lifecycleHash: status.hashes.lifecycleHash,
    };
}
function validateLifecycleStatusRequest(request) {
    if (request.scenario !== undefined &&
        request.scenario !== 'current_session' &&
        request.scenario !== 'generated_tunnel_enemy_defeated' &&
        request.scenario !== 'generated_tunnel_player_defeated') {
        throw new RuntimeBridgeError('invalid_input', 'unknown lifecycle status scenario');
    }
}
function validateRestartIntent(intent) {
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
function validateAutonomousPolicyTickInput(input) {
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
    if (input.navScenario !== undefined &&
        input.navScenario !== 'generated_tunnel_reachable' &&
        input.navScenario !== 'generated_tunnel_no_path') {
        throw new RuntimeBridgeError('invalid_input', 'unknown autonomous policy nav scenario');
    }
}
function validateAutonomousPolicyProposal(proposal, tick) {
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
function invalidAutonomousPolicyProposal(detail) {
    return {
        reason: 'invalid_policy_proposal',
        detail,
    };
}
function isEnemyPolicyVec3(value) {
    return value.length === 3 && value.every((component) => Number.isFinite(component));
}
function rejectedAutonomousPolicyProposalReceipt(proposal, rejection) {
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
function unsupportedAutonomousMovementReceipt(proposal) {
    const rejection = {
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
function runtimeActionReceiptToAutonomousReceipt(proposal, actionReceipt) {
    const status = actionReceipt.accepted ? 'accepted' : 'rejected';
    const rejection = actionReceipt.accepted
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
function validateInitializeInput(input) {
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
function validateRuntimeActionIntentEnvelope(envelope) {
    if (envelope.kind !== 'runtime_action_intent.v0') {
        throw new RuntimeBridgeError('invalid_input', 'runtime action intent kind must be runtime_action_intent.v0');
    }
    if (envelope.action !== 'primary_fire' && envelope.action !== 'use') {
        throw new RuntimeBridgeError('invalid_input', 'runtime action intent action is unsupported');
    }
    if (envelope.phase !== 'pressed' && envelope.phase !== 'released') {
        throw new RuntimeBridgeError('invalid_input', 'runtime action intent phase is unsupported');
    }
    if (envelope.source !== 'browser_fps_pointer' &&
        envelope.source !== 'programmatic' &&
        envelope.source !== 'enemy_policy') {
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
function combatReadoutTick(readout) {
    const fireEvent = readout.events.find((event) => event.kind === 'fire_hit' || event.kind === 'fire_missed');
    return fireEvent?.tick ?? 0;
}
function validateGeneratedTunnelReadoutRequest(request) {
    if (request.presetId !== undefined && request.presetId !== 'tiny-enclosed') {
        throw new RuntimeBridgeError('invalid_input', 'only the tiny-enclosed generated tunnel readout is available');
    }
    if (request.seed !== undefined && request.seed !== 17) {
        throw new RuntimeBridgeError('invalid_input', 'only seed 17 generated tunnel fixture readout is available');
    }
}
function validateGeneratedTunnelOperationRequest(request) {
    if (request.operation !== 'regenerate' && request.operation !== 'apply_to_runtime_world') {
        throw new RuntimeBridgeError('invalid_input', 'generated tunnel operation is unsupported');
    }
    validateGeneratedTunnelReadoutRequest(request);
}
function defaultRuntimeSessionEcrpProjectLoadInput(input) {
    return {
        kind: 'runtime_session.load_ecrp_project.v0',
        projectBundle: {
            kind: 'ProjectBundle',
            project: input.project,
            runtimeRequest: input.projectBundle,
        },
        entityDefinitions: [
            {
                kind: 'EntityDefinition',
                stableId: 'actor/demo-player',
                displayName: 'Demo Player',
                source: {
                    projectBundle: input.project.gameId,
                    relativePath: 'catalogs/actors/demo-player.entity.json',
                },
                capabilities: [
                    {
                        kind: 'transform',
                        initial: {
                            position: [0, 1.62, 0],
                            yawDegrees: 0,
                            pitchDegrees: 0,
                        },
                    },
                    {
                        kind: 'collisionBody',
                        halfExtents: [0.5, 1.4, 0.5],
                    },
                    {
                        kind: 'controller',
                        controller: 'player_input',
                    },
                    {
                        kind: 'health',
                        current: 100,
                        max: 100,
                    },
                    {
                        kind: 'weaponMount',
                        weaponId: 'weapon.demo.primary',
                    },
                    {
                        kind: 'renderProjection',
                        projection: 'first_person_camera',
                    },
                    {
                        kind: 'faction',
                        factionId: 'player',
                    },
                ],
            },
            {
                kind: 'EntityDefinition',
                stableId: 'actor/generated-tunnel-enemy',
                displayName: 'Generated Tunnel Enemy',
                source: {
                    projectBundle: input.project.gameId,
                    relativePath: 'catalogs/actors/generated-tunnel-enemy.entity.json',
                },
                capabilities: [
                    {
                        kind: 'transform',
                        initial: {
                            position: [0, 1.1, -3.5],
                            yawDegrees: 180,
                            pitchDegrees: 0,
                        },
                    },
                    {
                        kind: 'collisionBody',
                        halfExtents: [0.7, 1.8, 0.7],
                    },
                    {
                        kind: 'health',
                        current: 40,
                        max: 40,
                    },
                    {
                        kind: 'renderProjection',
                        projection: 'target_cube',
                    },
                    {
                        kind: 'policyBinding',
                        policyId: 'policy.enemy.generated_tunnel.v0',
                    },
                    {
                        kind: 'spawnMarker',
                        markerId: 'spawn.enemy.primary',
                    },
                    {
                        kind: 'faction',
                        factionId: 'hostile',
                    },
                ],
            },
        ],
        sceneDocument: {
            kind: 'SceneDocument',
            sceneId: `compat.scene.${input.projectBundle.sceneId}`,
            placements: [
                {
                    entityDefinitionId: 'actor/demo-player',
                    runtimeEntityId: 10,
                    spawnMarkerId: 'spawn.player.start',
                },
                {
                    entityDefinitionId: 'actor/generated-tunnel-enemy',
                    runtimeEntityId: 20,
                    spawnMarkerId: 'spawn.enemy.primary',
                },
            ],
        },
    };
}
function validateEcrpProjectLoadInput(input) {
    const diagnostics = [];
    if (input === null || typeof input !== 'object' || input.kind !== 'runtime_session.load_ecrp_project.v0') {
        return [
            {
                code: 'missingProjectBundle',
                path: 'input.kind',
                detail: 'ECRP project load input kind must be runtime_session.load_ecrp_project.v0',
            },
        ];
    }
    if (input.projectBundle?.kind !== 'ProjectBundle') {
        diagnostics.push({
            code: 'missingProjectBundle',
            path: 'projectBundle.kind',
            detail: 'projectBundle.kind must be ProjectBundle',
        });
    }
    if (!Array.isArray(input.entityDefinitions) || input.entityDefinitions.length === 0) {
        diagnostics.push({
            code: 'emptyEntityDefinitionList',
            path: 'entityDefinitions',
            detail: 'at least one EntityDefinition is required',
        });
    }
    const definitions = new Map();
    input.entityDefinitions?.forEach((definition, index) => {
        if (definition.kind !== 'EntityDefinition' || definition.stableId.trim().length === 0) {
            diagnostics.push({
                code: 'missingEntityDefinition',
                path: `entityDefinitions.${index}.stableId`,
                detail: 'EntityDefinition stableId is required',
            });
            return;
        }
        if (definitions.has(definition.stableId)) {
            diagnostics.push({
                code: 'duplicateEntityDefinition',
                path: `entityDefinitions.${index}.stableId`,
                detail: `duplicate EntityDefinition ${definition.stableId}`,
            });
        }
        definitions.set(definition.stableId, definition);
        validateEcrpCapabilities(definition, `entityDefinitions.${index}.capabilities`, diagnostics);
    });
    if (input.sceneDocument?.kind !== 'SceneDocument' || !Array.isArray(input.sceneDocument.placements)) {
        diagnostics.push({
            code: 'missingPlacement',
            path: 'sceneDocument.placements',
            detail: 'SceneDocument placements are required',
        });
        return diagnostics;
    }
    const placed = new Set();
    const runtimeIds = new Set();
    input.sceneDocument.placements.forEach((placement, index) => {
        if (!definitions.has(placement.entityDefinitionId)) {
            diagnostics.push({
                code: 'unknownEntityDefinition',
                path: `sceneDocument.placements.${index}.entityDefinitionId`,
                detail: `placement references unknown EntityDefinition ${placement.entityDefinitionId}`,
            });
        }
        if (placed.has(placement.entityDefinitionId)) {
            diagnostics.push({
                code: 'duplicatePlacement',
                path: `sceneDocument.placements.${index}.entityDefinitionId`,
                detail: `duplicate placement for EntityDefinition ${placement.entityDefinitionId}`,
            });
        }
        placed.add(placement.entityDefinitionId);
        if (placement.runtimeEntityId !== undefined) {
            if (!Number.isSafeInteger(placement.runtimeEntityId) || placement.runtimeEntityId <= 0) {
                diagnostics.push({
                    code: 'invalidCapability',
                    path: `sceneDocument.placements.${index}.runtimeEntityId`,
                    detail: 'runtimeEntityId must be a positive safe integer',
                });
            }
            else if (runtimeIds.has(placement.runtimeEntityId)) {
                diagnostics.push({
                    code: 'duplicatePlacement',
                    path: `sceneDocument.placements.${index}.runtimeEntityId`,
                    detail: `duplicate runtimeEntityId ${placement.runtimeEntityId}`,
                });
            }
            runtimeIds.add(placement.runtimeEntityId);
        }
    });
    for (const definition of definitions.values()) {
        if (!placed.has(definition.stableId)) {
            diagnostics.push({
                code: 'missingPlacement',
                path: `sceneDocument.placements.${definition.stableId}`,
                detail: `missing SceneDocument placement for ${definition.stableId}`,
            });
        }
    }
    return diagnostics;
}
function validateEcrpCapabilities(definition, path, diagnostics) {
    const capabilityKinds = new Set();
    for (const capability of definition.capabilities ?? []) {
        capabilityKinds.add(capability.kind);
        if (capability.kind === 'transform' && !isVec3(capability.initial?.position)) {
            diagnostics.push({
                code: 'invalidCapability',
                path: `${path}.transform.initial.position`,
                detail: 'transform initial position must be a finite vec3',
            });
        }
        if (capability.kind === 'collisionBody' && !isVec3(capability.halfExtents)) {
            diagnostics.push({
                code: 'invalidCapability',
                path: `${path}.collisionBody.halfExtents`,
                detail: 'collisionBody halfExtents must be a finite vec3',
            });
        }
        if (capability.kind === 'health' && (!Number.isFinite(capability.current) || capability.current < 0 || !Number.isFinite(capability.max) || capability.max <= 0)) {
            diagnostics.push({
                code: 'invalidCapability',
                path: `${path}.health`,
                detail: 'health current/max must be finite and max must be positive',
            });
        }
    }
    for (const required of ['transform', 'health', 'renderProjection']) {
        if (!capabilityKinds.has(required)) {
            diagnostics.push({
                code: 'missingCapability',
                path,
                detail: `${definition.stableId} missing required ${required} capability`,
            });
        }
    }
}
function isVec3(value) {
    return Array.isArray(value) && value.length === 3 && value.every((component) => Number.isFinite(component));
}
function buildEcrpProjectState(input) {
    const placements = new Map(input.sceneDocument.placements.map((placement, index) => [placement.entityDefinitionId, { placement, index }]));
    const entities = input.entityDefinitions.map((definition, index) => {
        const placement = placements.get(definition.stableId)?.placement;
        const entity = placement?.runtimeEntityId ?? inferredRuntimeEntityId(definition, index);
        return {
            entity,
            definition,
            role: inferRuntimeRole(definition),
        };
    });
    return {
        input,
        entities,
        bootstrapHash: stableHash({
            project: {
                gameId: input.projectBundle.project.gameId,
                workspaceId: input.projectBundle.project.workspaceId,
            },
            runtimeRequest: projectBundleHashRecord(input.projectBundle.runtimeRequest),
            sceneId: input.sceneDocument.sceneId,
            entityIds: entities.map((entity) => entity.entity),
            definitionIds: entities.map((entity) => entity.definition.stableId),
            capabilityKinds: entities.map((entity) => entity.definition.capabilities.map((capability) => capability.kind)),
        }),
    };
}
function inferredRuntimeEntityId(definition, index) {
    const role = inferRuntimeRole(definition);
    if (role === 'player') {
        return 10;
    }
    if (role === 'enemy') {
        return 20;
    }
    return 100 + index;
}
function inferRuntimeRole(definition) {
    const faction = definition.capabilities.find((capability) => capability.kind === 'faction');
    if (faction?.kind === 'faction') {
        if (faction.factionId === 'player') {
            return 'player';
        }
        if (faction.factionId === 'hostile') {
            return 'enemy';
        }
    }
    const controller = definition.capabilities.find((capability) => capability.kind === 'controller');
    if (controller?.kind === 'controller' && controller.controller === 'player_input') {
        return 'player';
    }
    if (definition.capabilities.some((capability) => capability.kind === 'policyBinding')) {
        return 'enemy';
    }
    return 'neutral';
}
function lifecycleStateFromEcrpProject(state) {
    const player = state.entities.find((entity) => entity.role === 'player');
    const enemy = state.entities.find((entity) => entity.role === 'enemy');
    return {
        player: lifecycleHealthFromEntity(player, 100),
        enemy: lifecycleHealthFromEntity(enemy, 40),
        terminalEvent: null,
        revision: 0,
    };
}
function lifecycleHealthFromEntity(entity, fallbackMax) {
    const health = entity?.definition.capabilities.find((capability) => capability.kind === 'health');
    if (entity !== undefined && health?.kind === 'health') {
        return lifecycleHealth(entity.entity, health.current, health.max, health.current <= 0);
    }
    return lifecycleHealth(entity?.entity ?? 0, fallbackMax, fallbackMax, false);
}
function ecrpCapabilitiesForEntity(entity, lifecycleState) {
    return entity.definition.capabilities.map((capability) => ecrpCapabilityForDefinition(entity, capability, lifecycleState));
}
function ecrpCapabilityForDefinition(entity, capability, lifecycleState) {
    switch (capability.kind) {
        case 'transform':
            return ecrpTransform(capability.initial.position, capability.initial.yawDegrees, capability.initial.pitchDegrees);
        case 'collisionBody':
            return ecrpCollisionBody(capability.staticCollider ?? false, capability.halfExtents);
        case 'controller':
            return ecrpController(capability.controller);
        case 'health':
            return ecrpHealth(runtimeHealthForEntity(entity, capability, lifecycleState));
        case 'weaponMount':
            return ecrpWeaponMount(capability.weaponId);
        case 'renderProjection':
            return ecrpRenderProjection(renderVisibleForEntity(entity, capability, lifecycleState), capability.projection);
        case 'policyBinding':
            return ecrpPolicyBinding(capability.policyId);
        case 'spawnMarker':
            return ecrpSpawnMarker(capability.markerId);
        case 'faction':
            return ecrpFaction(capability.factionId);
    }
}
function runtimeHealthForEntity(entity, capability, lifecycleState) {
    if (entity.role === 'player') {
        return lifecycleState.player;
    }
    if (entity.role === 'enemy') {
        return lifecycleState.enemy;
    }
    return lifecycleHealth(entity.entity, capability.current, capability.max, capability.current <= 0);
}
function renderVisibleForEntity(entity, capability, lifecycleState) {
    if (capability.visible !== undefined) {
        return capability.visible;
    }
    if (entity.role === 'enemy') {
        return !lifecycleState.enemy.dead;
    }
    if (entity.role === 'player') {
        return !lifecycleState.player.dead;
    }
    return true;
}
function buildEcrpRuntimeReadout(input) {
    const projectState = input.projectState ?? buildEcrpProjectState(defaultRuntimeSessionEcrpProjectLoadInput({
        sessionId: input.identity.sessionId,
        seed: input.identity.seed,
        project: input.identity.project,
        projectBundle: input.identity.projectBundle,
    }));
    const entities = projectState.entities.map((entity) => ecrpEntityReadout({
        entity: entity.entity,
        definition: entity.definition,
        capabilities: ecrpCapabilitiesForEntity(entity, input.lifecycleState),
        events: ecrpEventsForEntity(input.lifecycleState, entity.entity),
    }));
    const capabilityStateHash = stableHash(entities.map((entity) => entity.capabilities.map((capability) => capability.stateHash)));
    const eventReadoutHash = stableHash(entities.map((entity) => entity.recentEvents.map((event) => event.eventHash)));
    const entityReadoutHash = stableHash({
        entities: entities.map((entity) => entity.entityHash),
        capabilityStateHash,
        eventReadoutHash,
    });
    return {
        kind: 'runtime_session.ecrp_readout.v0',
        sequenceId: input.sequenceId,
        tick: input.tick,
        sessionHash: input.sessionHash,
        project: input.identity.project,
        projectBundle: input.identity.projectBundle,
        entities,
        entityCount: entities.length,
        hashes: {
            entityReadoutHash,
            capabilityStateHash,
            eventReadoutHash,
        },
        nonClaims: [
            'not_raw_state_store',
            'not_authoring_mode',
            'not_demo_local_authority',
        ],
    };
}
function ecrpEntityReadout(input) {
    const capabilityKinds = input.capabilities.map((capability) => capability.kind);
    const entityHash = stableHash({
        entity: input.entity,
        definitionStableId: input.definition.stableId,
        displayName: input.definition.displayName,
        sourcePath: input.definition.source.relativePath,
        capabilityKinds,
        capabilityStateHashes: input.capabilities.map((capability) => capability.stateHash),
        eventHashes: input.events.map((event) => event.eventHash),
    });
    return {
        entity: input.entity,
        lifecycle: 'active',
        definitionStableId: input.definition.stableId,
        displayName: input.definition.displayName,
        source: {
            projectBundle: input.definition.source.projectBundle,
            relativePath: input.definition.source.relativePath,
        },
        capabilityKinds,
        capabilities: input.capabilities,
        recentEvents: input.events,
        entityHash,
    };
}
function ecrpEventsForEntity(state, entity) {
    const events = [
        {
            kind: 'runtime_session.bootstrap_entity.v0',
            entity,
            tick: 0,
            eventHash: stableHash({
                kind: 'runtime_session.bootstrap_entity.v0',
                entity,
            }),
        },
    ];
    if (state.terminalEvent !== null && state.terminalEvent.entity === entity) {
        events.push({
            kind: state.terminalEvent.kind,
            entity,
            tick: state.terminalEvent.tick,
            eventHash: state.terminalEvent.eventHash,
        });
    }
    return events;
}
function ecrpTransform(position, yawDegrees, pitchDegrees) {
    const state = { kind: 'transform', position, yawDegrees, pitchDegrees };
    return { ...state, stateHash: stableHash(state) };
}
function ecrpCollisionBody(staticCollider, bounds) {
    const state = { kind: 'collisionBody', staticCollider, bounds };
    return { ...state, stateHash: stableHash(state) };
}
function ecrpController(controller) {
    const state = { kind: 'controller', controller };
    return { ...state, stateHash: stableHash(state) };
}
function ecrpHealth(health) {
    const state = {
        kind: 'health',
        current: health.current,
        max: health.max,
        dead: health.dead,
    };
    return { ...state, stateHash: stableHash(state) };
}
function ecrpWeaponMount(weaponId) {
    const state = { kind: 'weaponMount', weaponId };
    return { ...state, stateHash: stableHash(state) };
}
function ecrpRenderProjection(visible, projection) {
    const state = { kind: 'renderProjection', visible, projection };
    return { ...state, stateHash: stableHash(state) };
}
function ecrpPolicyBinding(policyId) {
    const state = { kind: 'policyBinding', policyId };
    return { ...state, stateHash: stableHash(state) };
}
function ecrpSpawnMarker(markerId) {
    const state = { kind: 'spawnMarker', markerId };
    return { ...state, stateHash: stableHash(state) };
}
function ecrpFaction(factionId) {
    const state = { kind: 'faction', factionId };
    return { ...state, stateHash: stableHash(state) };
}
function validateNavPathQueryRequest(request) {
    if (request.scenario !== undefined &&
        request.scenario !== 'generated_tunnel_reachable' &&
        request.scenario !== 'generated_tunnel_no_path') {
        throw new RuntimeBridgeError('invalid_input', 'unknown nav path scenario');
    }
    if (request.maxVisited !== undefined && (!Number.isSafeInteger(request.maxVisited) || request.maxVisited <= 0)) {
        throw new RuntimeBridgeError('invalid_input', 'nav path maxVisited must be a positive safe integer');
    }
}
function referenceRuntimeSessionNonClaims() {
    return [
        'not_native_runtime',
        'not_raw_state_store',
        'not_arbitrary_json_bridge',
        'not_gameplay_loop',
        'not_renderer',
    ];
}
function identityHashRecord(identity) {
    return {
        sessionId: identity.sessionId,
        mode: identity.mode,
        seed: identity.seed,
        project: {
            gameId: identity.project.gameId,
            workspaceId: identity.project.workspaceId,
        },
        projectBundle: projectBundleHashRecord(identity.projectBundle),
        nonClaims: identity.nonClaims,
    };
}
function runtimeSessionResetHash(identity) {
    return stableHash({
        seed: identity.seed,
        projectBundle: projectBundleHashRecord(identity.projectBundle),
        lifecycle: lifecycleStateHashRecord(initialRuntimeSessionLifecycleState()),
        encounter: encounterStateHashRecord(initialEncounterDirectorState()),
    });
}
function encounterStateHashRecord(state) {
    return {
        presetId: state.presetId,
        status: state.status,
        spawnedEnemyIds: state.spawnedEnemyIds,
        defeatedEnemyIds: state.defeatedEnemyIds,
        revision: state.revision,
        lastTransition: state.lastTransition,
    };
}
function lifecycleStateHashRecord(state) {
    return {
        player: lifecycleHealthHashRecord(state.player),
        enemy: lifecycleHealthHashRecord(state.enemy),
        terminalEventHash: state.terminalEvent?.eventHash ?? null,
        revision: state.revision,
    };
}
function lifecycleHealthHashRecord(health) {
    return {
        entity: health.entity,
        current: health.current,
        max: health.max,
        dead: health.dead,
    };
}
function projectBundleHashRecord(projectBundle) {
    return {
        bundleSchemaVersion: projectBundle.bundleSchemaVersion,
        protocolVersion: projectBundle.protocolVersion,
        sceneId: projectBundle.sceneId,
    };
}
function compositionHashRecord(composition) {
    return {
        loadedWorld: composition.loadedWorld,
        fatalCount: composition.fatalCount,
        totalCount: composition.totalCount,
        blocksLoad: composition.blocksLoad,
    };
}
function renderFrameHashRecord(frame) {
    return {
        opCount: frame.ops.length,
        opKinds: frame.ops.map((op) => op.op),
    };
}
function stableHash(value) {
    return `fnv1a64:${fnv1a64(stableStringify(value))}`;
}
function stableStringify(value) {
    if (value === undefined) {
        return 'undefined';
    }
    if (value === null || typeof value !== 'object') {
        return JSON.stringify(value);
    }
    if (Array.isArray(value)) {
        return `[${value.map((entry) => stableStringify(entry)).join(',')}]`;
    }
    const record = value;
    return `{${Object.keys(record)
        .sort()
        .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
        .join(',')}}`;
}
function fnv1a64(text) {
    let hash = 0xcbf29ce484222325n;
    const prime = 0x100000001b3n;
    const mask = 0xffffffffffffffffn;
    for (let index = 0; index < text.length; index += 1) {
        hash ^= BigInt(text.charCodeAt(index));
        hash = (hash * prime) & mask;
    }
    return hash.toString(16).padStart(16, '0');
}
//# sourceMappingURL=runtime-session.js.map