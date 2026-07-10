import { RuntimeBridgeError, frameCursor, } from './bridge.js';
import { createGeneratedTunnelEnemyPolicyFixture, validateEnemyPolicySource, } from '@asha/runtime-session';
import { buildEncounterDirectorReadout, buildEncounterTransitionReceipt, validateEncounterDirectorReadoutRequest, validateEncounterTransitionRequest, } from '@asha/runtime-session';
import { GENERATED_TUNNEL_NAV_POLICY_VIEW, GENERATED_TUNNEL_NO_PATH, GENERATED_TUNNEL_REACHABLE_PATH, } from '@asha/runtime-session';
import { buildRuntimeSessionEnemyNavPath, ecrpActorPosition, ecrpEntityTransform, } from './runtime-session-enemy-authority.js';
import { buildRuntimeSessionAnimationIntentReadout, } from './runtime-session-animation.js';
import { buildEcrpProjectState, buildEcrpRuntimeReadout, defaultRuntimeSessionEcrpProjectLoadInput, validateEcrpProjectLoadInput, } from './runtime-session-ecrp.js';
import { acceptedAutonomousMovementReceipt, lifecycleStatusReadout, lifecycleStatusToEncounterLifecycle, rejectedAutonomousPolicyProposalReceipt, runtimeActionReceiptToAutonomousReceipt, validateAutonomousPolicyProposal, validateAutonomousPolicyTickInput, validateGeneratedTunnelOperationRequest, validateInitializeInput, validateLifecycleStatusRequest, validateRestartIntent, validateRuntimeActionIntentEnvelope, } from './runtime-session-lifecycle.js';
import { compositionHashRecord, identityHashRecord, renderFrameHashRecord, stableHash, } from './runtime-session-hash.js';
export class RustBackedRuntimeSessionFacade {
    #bridge;
    #identity = null;
    #engine = null;
    #sequenceId = 0;
    #tick = 0;
    #acceptedCommandCount = 0;
    #rejectedCommandCount = 0;
    #restartCount = 0;
    #snapshot = null;
    #ecrpProjectState = null;
    #runtimeTransforms = new Map();
    #replayRecords = [];
    constructor(bridge) {
        this.#bridge = bridge;
    }
    initialize(input) {
        validateInitializeInput(input);
        const engine = this.#bridge.initializeEngine({ seed: input.seed });
        const composition = this.#bridge.loadProjectBundle(input.projectBundle); // vocab-allow: RuntimeSession facade adapts the legacy bridge operation.
        const defaultProject = defaultRuntimeSessionEcrpProjectLoadInput(input);
        const snapshot = this.#bridge.loadFpsRuntimeSession(fpsLoadRequestFromEcrpProject(defaultProject));
        this.#engine = engine;
        this.#identity = {
            sessionId: input.sessionId,
            mode: 'rust',
            seed: input.seed,
            project: input.project,
            projectBundle: input.projectBundle,
            nonClaims: rustRuntimeSessionNonClaims(),
        };
        this.#sequenceId = 0;
        this.#tick = 0;
        this.#acceptedCommandCount = 0;
        this.#rejectedCommandCount = 0;
        this.#restartCount = 0;
        this.#snapshot = snapshot;
        this.#ecrpProjectState = buildEcrpProjectState(defaultProject);
        this.#runtimeTransforms = new Map();
        this.#replayRecords = [];
        this.#record('initialize', snapshot.replayHash);
        return this.#stateSummary(composition);
    }
    loadEcrpProject(input) {
        const identity = this.#requireInitialized('loadEcrpProject');
        const before = this.#sessionHash();
        const diagnostics = validateEcrpProjectLoadInput(input);
        if (diagnostics.length > 0) {
            this.#sequenceId += 1;
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
        this.#bridge.loadProjectBundle(input.projectBundle.runtimeRequest); // vocab-allow: RuntimeSession ECRP load adapts the legacy bridge operation.
        const snapshot = this.#bridge.loadFpsRuntimeSession(fpsLoadRequestFromEcrpProject(input));
        this.#sequenceId += 1;
        this.#identity = {
            ...identity,
            project: input.projectBundle.project,
            projectBundle: input.projectBundle.runtimeRequest,
        };
        this.#snapshot = snapshot;
        this.#ecrpProjectState = buildEcrpProjectState(input);
        this.#runtimeTransforms = new Map();
        this.#record('loadEcrpProject', snapshot.replayHash);
        return {
            kind: 'runtime_session.ecrp_project_load_receipt.v0',
            sequenceId: this.#sequenceId,
            accepted: true,
            diagnostics: [],
            entityCount: snapshot.health.length,
            bootstrapHash: snapshot.entityHash,
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
            composition: this.#bridge.getProjectBundleCompositionStatus(),
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
            collisionSourceHash: snapshot.collision.collisionSourceHash,
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
        if (envelope.action !== 'primary_fire' || envelope.phase !== 'pressed') {
            this.#record('submitRuntimeActionIntent', undefined, envelope.source);
            return {
                sequenceId: this.#sequenceId,
                envelope,
                accepted: envelope.action === 'primary_fire' && envelope.phase === 'released',
                status: envelope.action === 'primary_fire' && envelope.phase === 'released' ? 'accepted' : 'unsupported',
                rejection: envelope.action === 'primary_fire' && envelope.phase === 'released'
                    ? null
                    : {
                        reason: 'combat_runtime_not_wired',
                        detail: 'Rust-backed RuntimeSession only accepts pressed primary_fire intents in this authority slice.',
                    },
                combatReadout: null,
                sessionHashBefore: before,
                sessionHashAfter: this.#sessionHash(),
            };
        }
        const fire = this.#bridge.applyFpsPrimaryFire({
            tick: envelope.tick,
            origin: [0, 1.62, 0],
            direction: [0, 0, -1],
        });
        this.#snapshot = this.#bridge.readFpsRuntimeSession();
        this.#record('submitRuntimeActionIntent', fire.replayHash, envelope.source);
        return {
            sequenceId: this.#sequenceId,
            envelope,
            accepted: true,
            status: 'accepted',
            rejection: null,
            combatReadout: combatReadoutFromFpsPrimaryFire(fire, envelope.tick),
            sessionHashBefore: before,
            sessionHashAfter: this.#sessionHash(),
        };
    }
    submitGameExtensionWeaponEffect(hook, primaryFire) {
        this.#requireInitialized('submitGameExtensionWeaponEffect');
        const before = this.#sessionHash();
        const result = this.#bridge.invokeGameExtensionWeaponEffect({ hook, primaryFire });
        this.#snapshot = this.#bridge.readFpsRuntimeSession();
        this.#sequenceId += 1;
        this.#record('submitGameExtensionWeaponEffect', result.replayEvidence.replayHash);
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
        this.#record('validateGameRuleCatalog', receipt.evidence.at(-1)?.contentHash);
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
        this.#record('submitGameRuleEffectIntent', receipt.replayHash);
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
        const sourceDiagnostics = input.policySource === undefined ? [] : validateEnemyPolicySource(input.policySource);
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
            ...(enemyPolicyPosition === undefined ? {} : { enemyPosition: enemyPolicyPosition }),
            ...(targetPolicyPosition === undefined ? {} : { targetPosition: targetPolicyPosition }),
            queryFixturePath: (scenario) => scenario === 'generated_tunnel_no_path'
                ? GENERATED_TUNNEL_NO_PATH
                : GENERATED_TUNNEL_REACHABLE_PATH,
        });
        const navPolicyView = {
            ...GENERATED_TUNNEL_NAV_POLICY_VIEW,
            latestPath: navPath,
        };
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
                const movement = this.#applyRustAutonomousMovementProposal(proposal, targetPolicyPosition);
                proposalReceipts.push(acceptedAutonomousMovementReceipt(proposal, movement));
                continue;
            }
            const actionReceipt = this.#submitRustEnemyPolicyPrimaryFire(proposal, fixture.view.enemy.position, fixture.view.target.position);
            proposalReceipts.push(runtimeActionReceiptToAutonomousReceipt(proposal, actionReceipt));
        }
        this.#sequenceId += 1;
        const recordHashesBeforePolicyRecord = this.#replayRecords.map((record) => record.recordHash);
        const movementSummary = proposalReceipts.find((receipt) => receipt.movement !== null)?.movement ?? null;
        const combatSummary = proposalReceipts.find((receipt) => receipt.combat !== null)?.combat ?? null;
        const authorityNavPathHash = movementSummary?.pathHash ?? navPath.pathHash;
        const tickHash = stableHash({
            loopId: 'generated_tunnel_enemy_policy_loop.v0',
            authority: 'rust_bridge',
            tick: step.tick,
            proposalFrameHash: fixture.frame.proposalHash,
            receiptStatuses: proposalReceipts.map((receipt) => receipt.status),
            receiptRejections: proposalReceipts.map((receipt) => receipt.rejection?.reason ?? null),
            navPathHash: authorityNavPathHash,
            replayRecordHashes: recordHashesBeforePolicyRecord,
            sequenceIdAfter: this.#sequenceId,
            runtimeSnapshotReplayHash: this.#snapshot?.replayHash ?? null,
        });
        this.#record('runAutonomousPolicyTick', tickHash);
        const telemetry = this.readTelemetry();
        const acceptedRuntimeActionCount = proposalReceipts.filter((receipt) => receipt.actionReceipt?.accepted === true).length;
        const rejectedRuntimeActionCount = proposalReceipts.filter((receipt) => receipt.actionReceipt !== null && receipt.actionReceipt.accepted === false).length;
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
                pathHash: authorityNavPathHash,
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
                recordHashes: telemetry.replayRecords.map((record) => record.recordHash),
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
        if (request.scenario !== undefined && request.scenario !== 'current_session') {
            throw new RuntimeBridgeError('invalid_input', 'Rust-backed RuntimeSession only exposes current_session lifecycle status');
        }
        return lifecycleStatusReadout({
            scenario: 'current_session',
            state: lifecycleStateFromFpsSnapshot(this.#requireSnapshot()),
            identity,
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            restartCount: this.#restartCount,
            sessionHash: this.#sessionHash(),
            restartReason: 'rust_epoch_restart',
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
            resetHash: statusAfter.hashes.replayHash,
        };
    }
    readEncounterDirector(request = {}) {
        const identity = this.#requireInitialized('readEncounterDirector');
        validateEncounterDirectorReadoutRequest(request);
        const lifecycle = this.#encounterLifecycleFromScenario(request.lifecycleScenario);
        const snapshot = this.#bridge.readFpsEncounterDirector(fpsEncounterLifecycleInput(lifecycle));
        return encounterReadoutFromFpsSnapshot({
            snapshot,
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            sessionSeed: identity.seed,
            sessionHash: this.#sessionHash(),
        });
    }
    requestEncounterTransition(request) {
        const identity = this.#requireInitialized('requestEncounterTransition');
        const sessionHashBefore = this.#sessionHash();
        const validationRejection = validateEncounterTransitionRequest(request);
        const lifecycle = validationRejection === undefined
            ? this.#encounterLifecycleFromScenario(request.lifecycleScenario)
            : this.#encounterLifecycleFromScenario();
        const beforeSnapshot = this.#bridge.readFpsEncounterDirector(fpsEncounterLifecycleInput(lifecycle));
        const before = encounterReadoutFromFpsSnapshot({
            snapshot: beforeSnapshot,
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            sessionSeed: identity.seed,
            sessionHash: sessionHashBefore,
        });
        const result = validationRejection === undefined
            ? this.#bridge.applyFpsEncounterTransition({
                presetId: request.presetId,
                action: request.action,
                lifecycle: fpsEncounterLifecycleInput(lifecycle),
            })
            : null;
        this.#sequenceId += 1;
        if (result?.accepted) {
            this.#record('requestEncounterTransition', result.replayHash);
        }
        else {
            this.#record('requestEncounterTransition');
        }
        const afterSnapshot = result === null
            ? beforeSnapshot
            : {
                ...beforeSnapshot,
                backend: result.backend,
                authoritySurface: result.authoritySurface,
                mutationOwner: result.mutationOwner,
                workspaceTrace: result.workspaceTrace,
                state: result.state,
                lifecycle: result.lifecycle,
                encounterHash: result.encounterHash,
                replayHash: result.replayHash,
            };
        const after = encounterReadoutFromFpsSnapshot({
            snapshot: afterSnapshot,
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            sessionSeed: identity.seed,
            sessionHash: this.#sessionHash(),
        });
        return buildEncounterTransitionReceipt({
            request,
            sequenceId: this.#sequenceId,
            before,
            after,
            result: result === null
                ? {
                    accepted: false,
                    state: fpsEncounterStateToReadoutState(beforeSnapshot.state),
                    rejectionReason: validationRejection ?? 'invalid_encounter_transition',
                }
                : encounterTransitionResultForReceipt(result),
            sessionHashBefore,
            sessionHashAfter: this.#sessionHash(),
        });
    }
    readCombatReadout(_request = {}) {
        void _request;
        this.#requireInitialized('readCombatReadout');
        throw new RuntimeBridgeError('operation_unimplemented', 'Rust-backed combat readout requires an action receipt in this slice');
    }
    readCombatFeedbackProjection(_request = {}) {
        void _request;
        this.#requireInitialized('readCombatFeedbackProjection');
        throw new RuntimeBridgeError('operation_unimplemented', 'Rust-backed combat feedback projection is not wired yet');
    }
    readGeneratedTunnelReadout(_request = {}) {
        void _request;
        this.#requireInitialized('readGeneratedTunnelReadout');
        throw new RuntimeBridgeError('operation_unimplemented', 'Rust-backed generated tunnel readout is not wired yet');
    }
    readNavProjection() {
        this.#requireInitialized('readNavProjection');
        throw new RuntimeBridgeError('operation_unimplemented', 'Rust-backed nav projection is not wired yet');
    }
    queryNavPath(_request = {}) {
        void _request;
        this.#requireInitialized('queryNavPath');
        throw new RuntimeBridgeError('operation_unimplemented', 'Rust-backed nav path query is not wired yet');
    }
    readNavPolicyView() {
        this.#requireInitialized('readNavPolicyView');
        throw new RuntimeBridgeError('operation_unimplemented', 'Rust-backed nav policy view is not wired yet');
    }
    requestGeneratedTunnelOperation(request) {
        this.#requireInitialized('requestGeneratedTunnelOperation');
        validateGeneratedTunnelOperationRequest(request);
        const before = this.#sessionHash();
        this.#sequenceId += 1;
        if (request.operation === 'regenerate') {
            this.#record('requestGeneratedTunnelOperation');
            return {
                sequenceId: this.#sequenceId,
                request,
                operation: request.operation,
                status: 'unsupported',
                reason: 'generated_tunnel_operation_not_wired',
                detail: 'Generated tunnel regeneration remains an authoring operation outside RuntimeSession.',
                sessionHashBefore: before,
                sessionHashAfter: this.#sessionHash(),
            };
        }
        const applied = this.#bridge.applyGeneratedTunnelToRuntimeWorld({
            preset: request.presetId ?? 'tiny-enclosed',
            seed: request.seed ?? 17,
        });
        this.#record('requestGeneratedTunnelOperation', applied.collisionProjectionHash);
        return {
            sequenceId: this.#sequenceId,
            request,
            operation: request.operation,
            status: 'applied',
            presetId: applied.preset,
            seed: applied.seed,
            grid: applied.grid,
            configHash: applied.configHash,
            outputHash: applied.outputHash,
            collisionSourceHash: applied.collisionSourceHash,
            collisionProjectionHash: applied.collisionProjectionHash,
            runtimeFrame: applied.runtimeFrame,
            sessionHashBefore: before,
            sessionHashAfter: this.#sessionHash(),
        };
    }
    planVoxelConversion(request) {
        this.#requireInitialized('planVoxelConversion');
        return this.#bridge.planVoxelConversion(request);
    }
    registerVoxelConversionSource(request) {
        this.#requireInitialized('registerVoxelConversionSource');
        return this.#bridge.registerVoxelConversionSource(request);
    }
    registerVoxelConversionMeshAsset(request) {
        this.#requireInitialized('registerVoxelConversionMeshAsset');
        return this.#bridge.registerVoxelConversionMeshAsset(request);
    }
    readVoxelConversionSourceMetadata(request) {
        this.#requireInitialized('readVoxelConversionSourceMetadata');
        return this.#bridge.readVoxelConversionSourceMetadata(request);
    }
    previewVoxelConversion(request) {
        this.#requireInitialized('previewVoxelConversion');
        return this.#bridge.previewVoxelConversion(request);
    }
    applyVoxelConversion(request) {
        this.#requireInitialized('applyVoxelConversion');
        return this.#bridge.applyVoxelConversion(request);
    }
    exportVoxelConversionEvidence(evidence) {
        this.#requireInitialized('exportVoxelConversionEvidence');
        return this.#bridge.exportVoxelConversionEvidence(evidence);
    }
    readVoxelModelInfo(request) {
        this.#requireInitialized('readVoxelModelInfo');
        return this.#bridge.readVoxelModelInfo(request);
    }
    readVoxelModelWindow(request) {
        this.#requireInitialized('readVoxelModelWindow');
        return this.#bridge.readVoxelModelWindow(request);
    }
    exportVoxelVolumeAsset(request) {
        this.#requireInitialized('exportVoxelVolumeAsset');
        return this.#bridge.exportVoxelVolumeAsset(request);
    }
    saveVoxelVolumeAsset(request) {
        this.#requireInitialized('saveVoxelVolumeAsset');
        return this.#bridge.saveVoxelVolumeAsset(request);
    }
    updateVoxelVolumeAssetPalette(request) {
        this.#requireInitialized('updateVoxelVolumeAssetPalette');
        return this.#bridge.updateVoxelVolumeAssetPalette(request);
    }
    loadVoxelVolumeAsset(request) {
        this.#requireInitialized('loadVoxelVolumeAsset');
        return this.#bridge.loadVoxelVolumeAsset(request);
    }
    validateVoxelAnnotationLayer(request) {
        this.#requireInitialized('validateVoxelAnnotationLayer');
        return this.#bridge.validateVoxelAnnotationLayer(request);
    }
    loadVoxelAnnotationLayer(request) {
        this.#requireInitialized('loadVoxelAnnotationLayer');
        return this.#bridge.loadVoxelAnnotationLayer(request);
    }
    readVoxelAnnotationQuery(request) {
        this.#requireInitialized('readVoxelAnnotationQuery');
        return this.#bridge.readVoxelAnnotationQuery(request);
    }
    applyVoxelAnnotationEdit(request) {
        this.#requireInitialized('applyVoxelAnnotationEdit');
        return this.#bridge.applyVoxelAnnotationEdit(request);
    }
    exportVoxelAnnotationLayer(request) {
        this.#requireInitialized('exportVoxelAnnotationLayer');
        return this.#bridge.exportVoxelAnnotationLayer(request);
    }
    readVoxelEditHistory(request) {
        this.#requireInitialized('readVoxelEditHistory');
        return this.#bridge.readVoxelEditHistory(request);
    }
    previewVoxelEditRevert(request) {
        this.#requireInitialized('previewVoxelEditRevert');
        return this.#bridge.previewVoxelEditRevert(request);
    }
    applyVoxelEditRevert(request) {
        this.#requireInitialized('applyVoxelEditRevert');
        return this.#bridge.applyVoxelEditRevert(request);
    }
    undoVoxelEdit(request) {
        this.#requireInitialized('undoVoxelEdit');
        return this.#bridge.undoVoxelEdit(request);
    }
    redoVoxelEdit(request) {
        this.#requireInitialized('redoVoxelEdit');
        return this.#bridge.redoVoxelEdit(request);
    }
    readEcrpRuntimeReadout() {
        const identity = this.#requireInitialized('readEcrpRuntimeReadout');
        const snapshot = this.#requireSnapshot();
        return buildEcrpRuntimeReadout({
            identity,
            projectState: this.#ecrpProjectState,
            lifecycleState: lifecycleStateFromFpsSnapshot(snapshot),
            runtimeTransforms: this.#runtimeTransforms,
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            sessionHash: this.#sessionHash(),
            authority: {
                mode: 'rust',
                source: 'rust_bridge',
                surface: snapshot.authoritySurface,
                readSets: snapshot.readSets,
            },
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
    readAnimationIntent() {
        this.#requireInitialized('readAnimationIntent');
        const snapshot = this.#requireSnapshot();
        return buildRuntimeSessionAnimationIntentReadout({
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            lifecycleState: lifecycleStateFromFpsSnapshot(snapshot),
        });
    }
    readProjection() {
        this.#requireInitialized('readProjection');
        const cursor = frameCursor(this.#sequenceId);
        const frame = this.#bridge.readRenderDiffs(cursor);
        const composition = this.#bridge.getProjectBundleCompositionStatus();
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
            composition: this.#bridge.getProjectBundleCompositionStatus(),
            acceptedCommandCount: this.#acceptedCommandCount,
            rejectedCommandCount: this.#rejectedCommandCount,
            restartCount: this.#restartCount,
            sessionHash: this.#sessionHash(),
            replayRecords: [...this.#replayRecords],
        };
    }
    restart() {
        this.#requireInitialized('restart');
        const before = this.#requireSnapshot();
        const snapshot = this.#bridge.restartFpsRuntimeSession({ expectedEpoch: before.sessionEpoch });
        this.#snapshot = snapshot;
        this.#sequenceId += 1;
        this.#tick = 0;
        this.#acceptedCommandCount = 0;
        this.#rejectedCommandCount = 0;
        this.#runtimeTransforms = new Map();
        this.#restartCount += 1;
        this.#record('restart', snapshot.replayHash);
        return {
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            composition: this.#bridge.getProjectBundleCompositionStatus(),
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
            resetHash: statusAfter.hashes.replayHash,
        };
    }
    #applyRustAutonomousMovementProposal(proposal, targetPosition) {
        const snapshot = this.#requireSnapshot();
        if (proposal.nextWaypoint === null) {
            throw new RuntimeBridgeError('invalid_input', 'enemy movement proposal cannot be applied without a next waypoint');
        }
        const movement = this.#bridge.applyEnemyDirectNavMovement({
            entity: snapshot.enemyEntity,
            seedPosition: proposal.from,
            target: targetPosition ?? proposal.nextWaypoint,
            maxStepUnits: 0.35,
        });
        const enemy = this.#ecrpProjectState?.entities.find((entity) => entity.entity === snapshot.enemyEntity);
        const current = enemy === undefined
            ? null
            : ecrpEntityTransform({
                entity: enemy,
                runtimeTransforms: this.#runtimeTransforms,
            });
        this.#runtimeTransforms.set(snapshot.enemyEntity, {
            position: movement.nextWaypoint,
            yawDegrees: current?.yawDegrees ?? 0,
            pitchDegrees: current?.pitchDegrees ?? 0,
        });
        return movement;
    }
    #submitRustEnemyPolicyPrimaryFire(proposal, enemyPosition, targetPosition) {
        const envelope = proposal.intent;
        const before = this.#sessionHash();
        this.#sequenceId += 1;
        const fire = this.#bridge.applyFpsPrimaryFire({
            tick: envelope.tick,
            origin: enemyPosition,
            direction: directionBetween(enemyPosition, targetPosition),
            shooterRole: 'enemy',
            targetRole: 'player',
        });
        this.#snapshot = this.#bridge.readFpsRuntimeSession();
        this.#record('submitRuntimeActionIntent', fire.replayHash, envelope.source);
        return {
            sequenceId: this.#sequenceId,
            envelope,
            accepted: true,
            status: 'accepted',
            rejection: null,
            combatReadout: combatReadoutFromFpsPrimaryFire(fire, envelope.tick),
            sessionHashBefore: before,
            sessionHashAfter: this.#sessionHash(),
        };
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
    #requireSnapshot() {
        if (this.#snapshot === null) {
            throw new RuntimeBridgeError('not_initialized', 'FPS RuntimeSession snapshot is unavailable before initialize');
        }
        return this.#snapshot;
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
    #record(kind, authorityHash, actionSource) {
        this.#replayRecords.push({
            sequenceId: this.#sequenceId,
            kind,
            ...(actionSource === undefined ? {} : { actionSource }),
            recordHash: authorityHash ?? stableHash({
                kind,
                ...(actionSource === undefined ? {} : { actionSource }),
                sequenceId: this.#sequenceId,
                tick: this.#tick,
                composition: compositionHashRecord(this.#bridge.getProjectBundleCompositionStatus()),
                fps: this.#snapshot === null
                    ? null
                    : {
                        entityHash: this.#snapshot.entityHash,
                        healthHash: this.#snapshot.healthHash,
                        replayHash: this.#snapshot.replayHash,
                        epoch: this.#snapshot.sessionEpoch,
                    },
            }),
        });
    }
    #sessionHash() {
        const snapshot = this.#snapshot;
        return stableHash({
            identity: this.#identity === null ? null : identityHashRecord(this.#identity),
            sequenceId: this.#sequenceId,
            tick: this.#tick,
            acceptedCommandCount: this.#acceptedCommandCount,
            rejectedCommandCount: this.#rejectedCommandCount,
            restartCount: this.#restartCount,
            fps: snapshot === null
                ? null
                : {
                    backend: snapshot.backend,
                    authoritySurface: snapshot.authoritySurface,
                    entityHash: snapshot.entityHash,
                    healthHash: snapshot.healthHash,
                    replayHash: snapshot.replayHash,
                    epoch: snapshot.sessionEpoch,
                },
            composition: this.#identity === null ? null : compositionHashRecord(this.#bridge.getProjectBundleCompositionStatus()),
        });
    }
}
function rustRuntimeSessionNonClaims() {
    return ['not_raw_state_store', 'not_arbitrary_json_bridge', 'not_renderer'];
}
function fpsLoadRequestFromEcrpProject(input) {
    const projectState = buildEcrpProjectState(input);
    const definitions = projectState.entities.map((entity) => fpsStoredEntityDefinition(entity));
    return {
        projectBundle: `${input.projectBundle.project.gameId}:${input.sceneDocument.sceneId}`,
        definitions,
        gameRuleModules: input.gameRuleModules ?? [],
    };
}
function fpsStoredEntityDefinition(entity) {
    const definition = entity.definition;
    const transform = definition.capabilities.find((capability) => capability.kind === 'transform');
    const collisionBody = definition.capabilities.find((capability) => capability.kind === 'collisionBody');
    const health = definition.capabilities.find((capability) => capability.kind === 'health');
    const weapon = definition.capabilities.find((capability) => capability.kind === 'weaponMount');
    const policyBinding = definition.capabilities.find((capability) => capability.kind === 'policyBinding');
    const renderProjection = definition.capabilities.find((capability) => capability.kind === 'renderProjection');
    const faction = definition.capabilities.find((capability) => capability.kind === 'faction');
    const spawnMarker = definition.capabilities.find((capability) => capability.kind === 'spawnMarker');
    return {
        entity: entity.entity,
        stableId: definition.stableId,
        displayName: definition.displayName,
        sourcePath: definition.source.relativePath,
        tags: [
            ...(faction?.kind === 'faction' ? [`faction:${faction.factionId}`] : []),
            ...(spawnMarker?.kind === 'spawnMarker' ? [`spawn:${spawnMarker.markerId}`] : []),
        ],
        role: entity.role,
        transform: transform?.kind === 'transform' ? fpsTransform(transform) : null,
        bounds: collisionBody?.kind === 'collisionBody' ? fpsWorldBounds(transform, collisionBody) : null,
        renderVisible: renderProjection?.kind === 'renderProjection' ? renderProjection.visible ?? true : null,
        staticCollider: collisionBody?.kind === 'collisionBody' ? collisionBody.staticCollider ?? false : null,
        health: health?.kind === 'health' ? { current: health.current, max: health.max } : null,
        weapon: weapon?.kind === 'weaponMount'
            ? {
                weaponId: weapon.weaponId,
                damage: 40,
                rangeUnits: 16,
                ammo: 2,
                cooldownTicksAfterFire: 4,
            }
            : null,
        policyBinding: policyBinding?.kind === 'policyBinding'
            ? {
                bindingId: `${definition.stableId}:policy`,
                policyId: policyBinding.policyId,
                viewKind: 'runtime_session.fps.policy_view.v0',
                viewVersion: 'v0',
                allowedIntents: ['enemy_policy.move_toward_target.v0', 'enemy_policy.primary_fire_intent.v0'],
                runtimeMoment: 'autonomous_policy_tick',
            }
            : null,
    };
}
function fpsTransform(capability) {
    return {
        translation: capability.initial.position,
        rotation: [0, 0, 0, 1],
        scale: [1, 1, 1],
    };
}
function fpsWorldBounds(transform, collisionBody) {
    const position = transform?.kind === 'transform' ? transform.initial.position : [0, 0, 0];
    return {
        min: [
            position[0] - collisionBody.halfExtents[0],
            position[1] - collisionBody.halfExtents[1],
            position[2] - collisionBody.halfExtents[2],
        ],
        max: [
            position[0] + collisionBody.halfExtents[0],
            position[1] + collisionBody.halfExtents[1],
            position[2] + collisionBody.halfExtents[2],
        ],
    };
}
function lifecycleStateFromFpsSnapshot(snapshot) {
    const player = fpsLifecycleHealth(snapshot, snapshot.playerEntity);
    const enemy = fpsLifecycleHealth(snapshot, snapshot.enemyEntity);
    const terminalEvent = snapshot.lifecycleStatus.state === 'enemy_defeated'
        ? {
            kind: 'runtime_lifecycle.enemy_defeated.v0',
            entity: snapshot.lifecycleStatus.entity,
            tick: snapshot.lifecycleStatus.tick,
            reason: 'combat_health_zero',
            eventHash: stableHash({
                kind: 'runtime_lifecycle.enemy_defeated.v0',
                entity: snapshot.lifecycleStatus.entity,
                tick: snapshot.lifecycleStatus.tick,
                reason: 'combat_health_zero',
                replayHash: snapshot.replayHash,
            }),
        }
        : null;
    return {
        player,
        enemy,
        terminalEvent,
        revision: snapshot.replayRecords.length,
    };
}
function fpsLifecycleHealth(snapshot, entity) {
    const health = snapshot.health.find((entry) => entry.entity === entity);
    const current = health?.current ?? 0;
    const max = health?.max ?? 0;
    return {
        entity,
        current,
        max,
        dead: current <= 0,
        healthHash: snapshot.healthHash,
    };
}
function directionBetween(origin, target) {
    const dx = target[0] - origin[0];
    const dy = target[1] - origin[1];
    const dz = target[2] - origin[2];
    const length = Math.hypot(dx, dy, dz);
    if (length === 0)
        return [0, 0, 1];
    return [dx / length, dy / length, dz / length];
}
function combatReadoutFromFpsPrimaryFire(result, tick) {
    if (result.target === null || result.targetHealthBefore === null || result.targetHealthAfter === null) {
        return {
            scenario: 'runtime_session_loaded_project_fire_hit',
            outcome: {
                kind: 'miss',
                reason: 'noTarget',
            },
            events: [{ kind: 'fire_missed', shooter: result.shooter, reason: 'noTarget', tick }],
            health: [],
            nextFireControl: {
                ammo: 2,
                cooldownTicksRemaining: 4,
                cooldownTicksAfterFire: 4,
            },
            healthHash: result.healthHash,
            replayHash: result.replayHash,
            authority: combatAuthorityFromFpsPrimaryFire(result),
            fixture: null,
        };
    }
    const defeated = result.targetHealthAfter.current <= 0;
    return {
        scenario: 'runtime_session_loaded_project_fire_hit',
        outcome: {
            kind: 'hit',
            target: result.target,
            distance: 0,
            hitPosition: null,
            defeated,
        },
        events: [
            { kind: 'fire_hit', shooter: result.shooter, target: result.target, distance: 0, tick },
            {
                kind: 'damage_applied',
                target: result.target,
                amount: result.targetHealthBefore.current - result.targetHealthAfter.current,
                before: result.targetHealthBefore.current,
                after: result.targetHealthAfter.current,
            },
            ...(defeated ? [{ kind: 'entity_defeated', target: result.target }] : []),
        ],
        health: [{
                entity: result.target,
                current: result.targetHealthAfter.current,
                max: result.targetHealthAfter.max,
                dead: defeated,
            }],
        nextFireControl: {
            ammo: 2,
            cooldownTicksRemaining: 4,
            cooldownTicksAfterFire: 4,
        },
        healthHash: result.healthHash,
        replayHash: result.replayHash,
        authority: combatAuthorityFromFpsPrimaryFire(result),
        fixture: null,
    };
}
function combatAuthorityFromFpsPrimaryFire(result) {
    return {
        source: result.backend === 'native_rust' ? 'rust_bridge' : 'reference_bridge',
        backend: result.backend,
        surface: result.authoritySurface,
        mutationOwner: result.mutationOwner,
        workspaceTrace: result.workspaceTrace,
    };
}
function fpsEncounterLifecycleInput(lifecycle) {
    return {
        outcomeKind: lifecycle.outcomeKind,
        terminal: lifecycle.terminal,
        enemyDead: lifecycle.enemyDead,
        playerDead: lifecycle.playerDead,
        lifecycleHash: lifecycle.lifecycleHash,
    };
}
function encounterReadoutFromFpsSnapshot(input) {
    return buildEncounterDirectorReadout({
        state: fpsEncounterStateToReadoutState(input.snapshot.state),
        sequenceId: input.sequenceId,
        tick: input.tick,
        sessionSeed: input.sessionSeed,
        sessionHash: input.sessionHash,
        lifecycle: input.snapshot.lifecycle,
        authority: {
            source: input.snapshot.backend === 'native_rust' ? 'rust_bridge' : 'reference_bridge',
            backend: input.snapshot.backend,
            surface: input.snapshot.authoritySurface,
            mutationOwner: input.snapshot.mutationOwner,
            readSets: input.snapshot.readSets,
            workspaceTrace: input.snapshot.workspaceTrace,
        },
    });
}
function fpsEncounterStateToReadoutState(state) {
    return {
        presetId: requireGeneratedTunnelEncounterPreset(state.presetId),
        status: state.status,
        spawnedEnemyIds: generatedTunnelEncounterIds(state.spawnedEnemyIds),
        defeatedEnemyIds: generatedTunnelEncounterIds(state.defeatedEnemyIds),
        revision: state.revision,
        lastTransition: state.lastTransition,
    };
}
function encounterTransitionResultForReceipt(result) {
    return {
        accepted: result.accepted,
        state: fpsEncounterStateToReadoutState(result.state),
        ...(result.eventKind === null ? {} : { eventKind: result.eventKind }),
        ...(result.rejectionReason === null ? {} : { rejectionReason: result.rejectionReason }),
    };
}
function requireGeneratedTunnelEncounterPreset(value) {
    if (value !== 'generated-tunnel-small-encounter') {
        throw new RuntimeBridgeError('internal', `unsupported Rust encounter preset '${value}'`);
    }
    return value;
}
function generatedTunnelEncounterIds(ids) {
    return ids.map((id) => {
        if (id !== 'encounter.generated_tunnel_small.wave_1.enemy_001') {
            throw new RuntimeBridgeError('internal', `unsupported Rust encounter instance '${id}'`);
        }
        return id;
    });
}
//# sourceMappingURL=runtime-session-rust-facade.js.map