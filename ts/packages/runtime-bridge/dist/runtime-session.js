import { cameraHandle, } from '@asha/contracts';
import { RuntimeBridgeError, frameCursor, } from './bridge.js';
import { GENERATED_TUNNEL_FIRE_HIT_READOUT, GENERATED_TUNNEL_FIRE_MISS_READOUT, } from './combat-readout.js';
import { buildCombatFeedbackProjection, defaultCombatFeedbackIntent, } from './combat-feedback.js';
import { TINY_GENERATED_TUNNEL_READOUT, } from './generated-tunnel.js';
import { createGeneratedTunnelEnemyPolicyFixture, validateEnemyPolicySource, } from './enemy-policy.js';
import { buildEncounterDirectorReadout, buildEncounterTransitionReceipt, initialEncounterDirectorState, transitionEncounterDirectorState, validateEncounterDirectorReadoutRequest, validateEncounterTransitionRequest, } from './encounter-director.js';
import { GENERATED_TUNNEL_NAV_POLICY_VIEW, GENERATED_TUNNEL_NAV_PROJECTION, GENERATED_TUNNEL_NO_PATH, GENERATED_TUNNEL_REACHABLE_PATH, } from './nav-readout.js';
import { buildRuntimeSessionEnemyNavPath, ecrpActorPosition, ecrpEntityTransform, runtimeTransformHashRecord, } from './runtime-session-enemy-authority.js';
import { buildEcrpProjectState, buildEcrpRuntimeReadout, defaultRuntimeSessionEcrpProjectLoadInput, lifecycleStateFromEcrpProject, validateEcrpProjectLoadInput, } from './runtime-session-ecrp.js';
import { acceptedAutonomousMovementReceipt, applyReferenceCombatReadoutToLifecycleState, buildReferenceRuntimeSessionPrimaryFireReadout, combatReadoutTick, generatedTunnelEnemyDefeatedLifecycleState, generatedTunnelPlayerDefeatedLifecycleState, initialRuntimeSessionLifecycleState, lifecycleStatusReadout, lifecycleStatusToEncounterLifecycle, rejectedAutonomousPolicyProposalReceipt, runtimeActionReceiptToAutonomousReceipt, validateAutonomousPolicyProposal, validateAutonomousPolicyTickInput, validateGeneratedTunnelOperationRequest, validateGeneratedTunnelReadoutRequest, validateInitializeInput, validateLifecycleStatusRequest, validateRestartIntent, validateRuntimeActionIntentEnvelope, } from './runtime-session-lifecycle.js';
import { compositionHashRecord, encounterStateHashRecord, identityHashRecord, lifecycleStateHashRecord, referenceRuntimeSessionNonClaims, renderFrameHashRecord, stableHash, } from './runtime-session-hash.js';
import { RustBackedRuntimeSessionFacade } from './runtime-session-rust-facade.js';
export function createRuntimeSessionFacade(options) {
    if (options.mode === 'reference') {
        return new ReferenceRuntimeSessionFacade(options.bridge);
    }
    return new RustBackedRuntimeSessionFacade(options.bridge);
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
    #runtimeTransforms = new Map();
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
        this.#runtimeTransforms = new Map();
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
        this.#runtimeTransforms = new Map();
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
            ? buildReferenceRuntimeSessionPrimaryFireReadout({
                projectState: this.#ecrpProjectState,
                lifecycleState: this.#lifecycleState,
                source: envelope.source,
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
    submitGameExtensionWeaponEffect(hook, primaryFire) {
        this.#requireInitialized('submitGameExtensionWeaponEffect');
        const before = this.#sessionHash();
        const result = this.#bridge.invokeGameExtensionWeaponEffect({ hook, primaryFire });
        this.#sequenceId += 1;
        this.#record('submitGameExtensionWeaponEffect');
        return {
            sequenceId: this.#sequenceId,
            request: { hook, primaryFire },
            hookReceipt: result.hookReceipt,
            replayEvidence: result.replayEvidence,
            primaryFire: result.primaryFire,
            sessionHashBefore: before,
            sessionHashAfter: this.#sessionHash(),
        };
    }
    validateGameRuleCatalog(catalog) {
        this.#requireInitialized('validateGameRuleCatalog');
        const before = this.#sessionHash();
        const receipt = this.#bridge.validateGameRuleCatalog(catalog);
        this.#sequenceId += 1;
        this.#record('validateGameRuleCatalog');
        return {
            ...receipt,
            sequenceId: this.#sequenceId,
            catalog,
            sessionHashBefore: before,
            sessionHashAfter: this.#sessionHash(),
        };
    }
    submitGameRuleEffectIntent(catalog, request) {
        this.#requireInitialized('submitGameRuleEffectIntent');
        const before = this.#sessionHash();
        const receipt = this.#bridge.submitGameRuleEffectIntent({ catalog, request });
        this.#sequenceId += 1;
        this.#record('submitGameRuleEffectIntent');
        return {
            ...receipt,
            sequenceId: this.#sequenceId,
            catalog,
            request,
            sessionHashBefore: before,
            sessionHashAfter: this.#sessionHash(),
        };
    }
    readGameRuleRuntimeReadout() {
        this.#requireInitialized('readGameRuleRuntimeReadout');
        return this.#bridge.readGameRuleRuntimeReadout();
    }
    runAutonomousPolicyTick(input) {
        this.#requireInitialized('runAutonomousPolicyTick');
        validateAutonomousPolicyTickInput(input);
        const sequenceIdBefore = this.#sequenceId;
        const sessionHashBefore = this.#sessionHash();
        const step = this.tick(input.tick === undefined ? {} : { tick: input.tick });
        const usesLivePolicyPositions = input.enemy?.position !== undefined || input.target?.position !== undefined;
        const enemyPolicyPosition = input.enemy?.position ??
            ecrpActorPosition({
                projectState: this.#ecrpProjectState,
                runtimeTransforms: this.#runtimeTransforms,
                role: 'enemy',
            }) ??
            undefined;
        const targetPolicyPosition = input.target?.position ??
            ecrpActorPosition({
                projectState: this.#ecrpProjectState,
                runtimeTransforms: this.#runtimeTransforms,
                role: 'player',
            }) ??
            undefined;
        const navPath = buildRuntimeSessionEnemyNavPath({
            ...(input.navScenario === undefined ? {} : { scenario: input.navScenario }),
            ...(!usesLivePolicyPositions || enemyPolicyPosition === undefined ? {} : { enemyPosition: enemyPolicyPosition }),
            ...(!usesLivePolicyPositions || targetPolicyPosition === undefined ? {} : { targetPosition: targetPolicyPosition }),
            queryFixturePath: (scenario) => this.queryNavPath(scenario === undefined ? {} : { scenario }),
        });
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
                const movement = this.#applyAutonomousMovementProposal(proposal, targetPolicyPosition);
                proposalReceipts.push(acceptedAutonomousMovementReceipt(proposal, movement));
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
    planVoxelConversion(_request) {
        void _request;
        this.#requireInitialized('planVoxelConversion');
        throw new RuntimeBridgeError('operation_unimplemented', 'Voxel conversion authority is not wired into the reference RuntimeSession');
    }
    previewVoxelConversion(_request) {
        void _request;
        this.#requireInitialized('previewVoxelConversion');
        throw new RuntimeBridgeError('operation_unimplemented', 'Voxel conversion preview is not wired into the reference RuntimeSession');
    }
    applyVoxelConversion(_request) {
        void _request;
        this.#requireInitialized('applyVoxelConversion');
        throw new RuntimeBridgeError('operation_unimplemented', 'Voxel conversion apply is not wired into the reference RuntimeSession');
    }
    exportVoxelConversionEvidence(_evidence) {
        void _evidence;
        this.#requireInitialized('exportVoxelConversionEvidence');
        throw new RuntimeBridgeError('operation_unimplemented', 'Voxel conversion evidence export is not wired into the reference RuntimeSession');
    }
    readVoxelModelInfo(_request) {
        void _request;
        this.#requireInitialized('readVoxelModelInfo');
        throw new RuntimeBridgeError('operation_unimplemented', 'Voxel model info is not wired into the reference RuntimeSession');
    }
    readEcrpRuntimeReadout() {
        const identity = this.#requireInitialized('readEcrpRuntimeReadout');
        return buildEcrpRuntimeReadout({
            identity,
            projectState: this.#ecrpProjectState,
            lifecycleState: this.#lifecycleState,
            runtimeTransforms: this.#runtimeTransforms,
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
        this.#runtimeTransforms = new Map();
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
    #applyAutonomousMovementProposal(proposal, targetPosition) {
        const enemy = this.#ecrpProjectState?.entities.find((entity) => entity.role === 'enemy');
        if (enemy === undefined || proposal.nextWaypoint === null || this.#lifecycleState.enemy.dead) {
            throw new RuntimeBridgeError('invalid_input', 'enemy movement proposal cannot be applied without a live ECRP enemy');
        }
        const movement = this.#bridge.applyEnemyDirectNavMovement({
            entity: enemy.entity,
            seedPosition: proposal.from,
            target: targetPosition ?? proposal.nextWaypoint,
            maxStepUnits: 0.35,
        });
        const current = ecrpEntityTransform({
            entity: enemy,
            runtimeTransforms: this.#runtimeTransforms,
        });
        this.#runtimeTransforms.set(enemy.entity, {
            position: movement.nextWaypoint,
            yawDegrees: current?.yawDegrees ?? 0,
            pitchDegrees: current?.pitchDegrees ?? 0,
        });
        return movement;
    }
    #applyCombatLifecycleReadout(readout, tick) {
        const applied = applyReferenceCombatReadoutToLifecycleState({
            state: this.#lifecycleState,
            readout,
            tick,
        });
        this.#lifecycleState = applied.state;
        if (applied.recordLifecycleDeath) {
            this.#record('lifecycleDeath');
        }
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
                ...(this.#runtimeTransforms.size === 0
                    ? {}
                    : { runtimeTransforms: runtimeTransformHashRecord(this.#runtimeTransforms) }),
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
            ...(this.#identity === null || this.#runtimeTransforms.size === 0
                ? {}
                : { runtimeTransforms: runtimeTransformHashRecord(this.#runtimeTransforms) }),
            encounter: this.#identity === null ? null : encounterStateHashRecord(this.#encounterState),
            composition: this.#identity === null ? null : compositionHashRecord(this.#bridge.getCompositionStatus()),
        });
    }
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
//# sourceMappingURL=runtime-session.js.map