import { test } from 'node:test';
import assert from 'node:assert/strict';
import { entityId } from '@asha/contracts';
import { createRuntimeSessionFacade, readRuntimeSessionPlayableLoopState, } from './index.js';
import { MockRuntimeBridge } from './mock.js';
import { stableHash } from './runtime-session-hash.js';
const PLAYER_ENTITY = 101;
const ENEMY_ENTITY = 202;
class GameExtensionFireBridgeDouble extends MockRuntimeBridge {
    gameExtensionFireRequests = [];
    #playerHealth = 88;
    #enemyHealth = 55;
    #snapshot = this.#snapshotFor('fnv1a64:0000000000000001');
    loadFpsRuntimeSession(request) {
        this.#playerHealth = request.definitions.find((definition) => definition.role === 'player')?.health?.current ?? 88;
        this.#enemyHealth = request.definitions.find((definition) => definition.role === 'enemy')?.health?.current ?? 55;
        this.#snapshot = this.#snapshotFor('fnv1a64:0000000000000002');
        return this.#snapshot;
    }
    readFpsRuntimeSession() {
        return this.#snapshot;
    }
    invokeGameExtensionWeaponEffect(request) {
        this.gameExtensionFireRequests.push(request);
        const before = this.#enemyHealth;
        this.#enemyHealth = 0;
        const primaryFire = {
            backend: 'native_rust',
            authoritySurface: 'runtime_session.fps.primary_fire.v0',
            mutationOwner: 'rule-lifecycle + svc-combat',
            workspaceTrace: ['validated FireIntentCommand against svc-combat'],
            shooter: PLAYER_ENTITY,
            target: ENEMY_ENTITY,
            targetHealthBefore: { current: before, max: 55 },
            targetHealthAfter: { current: this.#enemyHealth, max: 55 },
            lifecycleStatus: { state: 'enemy_defeated', entity: ENEMY_ENTITY, tick: request.primaryFire.tick },
            targetRenderVisible: false,
            entityHash: 'fnv1a64:00000000000000aa',
            healthHash: 'fnv1a64:00000000000000cc',
            replayHash: 'fnv1a64:0000000000000033',
        };
        this.#snapshot = this.#snapshotFor(primaryFire.replayHash);
        return {
            hookReceipt: acceptedHookReceipt(request.hook),
            replayEvidence: acceptedReplayEvidence(request.hook, primaryFire),
            primaryFire,
        };
    }
    #snapshotFor(replayHash) {
        return {
            backend: 'native_rust',
            authoritySurface: 'runtime_session.fps.authority.v0',
            projectBundle: 'custom-demo:custom-demo.scene',
            sessionEpoch: 1,
            lifecycleStatus: this.#enemyHealth <= 0
                ? { state: 'enemy_defeated', entity: ENEMY_ENTITY, tick: 7 }
                : { state: 'active' },
            playerEntity: PLAYER_ENTITY,
            enemyEntity: ENEMY_ENTITY,
            health: [
                { entity: PLAYER_ENTITY, current: this.#playerHealth, max: 88 },
                { entity: ENEMY_ENTITY, current: this.#enemyHealth, max: 55 },
            ],
            policyBindings: [],
            replayRecords: [{
                    replayUnit: 'runtime_session.fps.primary_fire.v0',
                    entityHash: 'fnv1a64:00000000000000aa',
                    healthHash: this.#enemyHealth <= 0 ? 'fnv1a64:00000000000000cc' : 'fnv1a64:00000000000000bb',
                    recordHash: replayHash,
                }],
            readSets: [{
                    viewKind: 'runtime_session.fps.lifecycle_health.v0',
                    owner: 'rule-lifecycle',
                    readSet: ['entity.lifecycle', 'capability.health'],
                }],
            entityHash: 'fnv1a64:00000000000000aa',
            healthHash: this.#enemyHealth <= 0 ? 'fnv1a64:00000000000000cc' : 'fnv1a64:00000000000000bb',
            replayHash,
        };
    }
}
void test('Rust-backed game-extension primary fire mutates health and playable counters', () => {
    const bridge = new GameExtensionFireBridgeDouble();
    const session = createRuntimeSessionFacade({ bridge, mode: 'rust' });
    session.initialize(sessionInput());
    session.loadEcrpProject(ecrpProjectLoadInput());
    const before = readRuntimeSessionPlayableLoopState(session);
    assert.equal(before.counters.shotsFired, 0);
    assert.equal(before.health.enemy.current, 55);
    const primaryFire = {
        tick: 7,
        origin: [1, 1.7, 2],
        direction: [0, 0, -1],
        shooterRole: 'player',
        targetRole: 'enemy',
    };
    const receipt = session.submitGameExtensionWeaponEffect(weaponEffectHook(), primaryFire);
    assert.equal(receipt.hookReceipt.status, 'proposed');
    assert.equal(receipt.replayEvidence.validationStatus, 'accepted');
    assert.equal(receipt.primaryFire?.target, ENEMY_ENTITY);
    assert.equal(receipt.primaryFire?.targetHealthBefore?.current, 55);
    assert.equal(receipt.primaryFire?.targetHealthAfter?.current, 0);
    assert.deepEqual(bridge.gameExtensionFireRequests.map((request) => request.primaryFire), [primaryFire]);
    const readout = session.readEcrpRuntimeReadout();
    const enemy = readout.entities.find((entity) => entity.entity === ENEMY_ENTITY);
    const enemyHealth = enemy?.capabilities.find((capability) => capability.kind === 'health');
    assert.equal(enemyHealth?.kind, 'health');
    assert.equal(enemyHealth?.current, 0);
    assert.equal(enemyHealth?.dead, true);
    const playable = readRuntimeSessionPlayableLoopState(session);
    assert.equal(playable.counters.actionTick, 1);
    assert.equal(playable.counters.shotsFired, 1);
    assert.equal(playable.counters.hits, 1);
    assert.equal(playable.counters.remainingTargets, 0);
    assert.equal(playable.health.enemy.current, 0);
    assert.deepEqual(playable.commands.blockedReasons, ['target_defeated']);
});
function sessionInput() {
    return {
        sessionId: 'runtime-session.game-extension-fire.test',
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
        kind: 'runtime_session.load_ecrp_project.v0',
        projectBundle: {
            kind: 'ProjectBundle',
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
            entityDefinition('actor/custom-player', 'player', 88),
            entityDefinition('actor/custom-enemy', 'enemy', 55),
        ],
        sceneDocument: {
            kind: 'SceneDocument',
            sceneId: 'custom-demo.scene',
            placements: [
                { entityDefinitionId: 'actor/custom-player', runtimeEntityId: PLAYER_ENTITY },
                { entityDefinitionId: 'actor/custom-enemy', runtimeEntityId: ENEMY_ENTITY },
            ],
        },
        gameRuleModules: [gameRuleModuleManifest()],
    };
}
function entityDefinition(stableId, role, health) {
    return {
        kind: 'EntityDefinition',
        stableId,
        displayName: role === 'player' ? 'Custom Player' : 'Custom Enemy',
        source: {
            projectBundle: 'custom-demo',
            relativePath: `catalogs/actors/${stableId.split('/')[1]}.entity.json`,
        },
        capabilities: [
            {
                kind: 'transform',
                initial: {
                    position: role === 'player' ? [1, 1.7, 2] : [4, 1.2, -6],
                    yawDegrees: role === 'player' ? 15 : 180,
                    pitchDegrees: 0,
                },
            },
            { kind: 'collisionBody', halfExtents: [0.3, 0.7, 0.3] },
            { kind: 'health', current: health, max: health },
            ...(role === 'player'
                ? [
                    { kind: 'controller', controller: 'player_input' },
                    { kind: 'weaponMount', weaponId: 'weapon.custom.primary' },
                ]
                : [
                    { kind: 'policyBinding', policyId: 'policy.enemy.custom.v0' },
                    { kind: 'spawnMarker', markerId: 'spawn.enemy.custom' },
                ]),
            {
                kind: 'renderProjection',
                projection: role === 'player' ? 'first_person_camera' : 'target_cube',
            },
            { kind: 'faction', factionId: role === 'player' ? 'player' : 'hostile' },
        ],
    };
}
function gameRuleModuleManifest() {
    return {
        moduleRef: {
            moduleId: 'demo.primary_fire_effect',
            version: '0.1.0',
            contractHash: 'sha256:demo-primary-fire-effect-contract-v0',
        },
        declaredHooks: [{
                hookId: 'demo.primary_fire_effect.weapon',
                kind: 'weaponEffect',
                inputContract: 'WeaponEffectHookRequest.v0',
                outputContract: 'GameExtensionProposal.v0',
                requiredCapabilities: ['health', 'weaponMount'],
            }],
        deterministicRequirements: ['no-wall-clock', 'no-ambient-random'],
        sourceHash: 'sha256:demo-primary-fire-effect-source-v0',
    };
}
function weaponEffectHook() {
    return {
        moduleRef: gameRuleModuleManifest().moduleRef,
        hookId: 'demo.primary_fire_effect.weapon',
        requestId: 'asha-demo.primary-fire.7',
        tick: 7,
        source: entityId(PLAYER_ENTITY),
        target: entityId(ENEMY_ENTITY),
        baseDamage: 40,
        rangeMillimeters: 8_000,
        tags: ['asha-demo', 'primary-fire'],
        inputHash: 'fnv1a64:demo-primary-fire-input',
    };
}
function acceptedHookReceipt(hook) {
    return {
        moduleRef: hook.moduleRef,
        hookId: hook.hookId,
        requestId: hook.requestId,
        status: 'proposed',
        inputHash: hook.inputHash,
        proposal: {
            kind: 'damageModifier',
            proposalId: `${hook.requestId}.registered_damage_bonus`,
            target: hook.target ?? entityId(ENEMY_ENTITY),
            channelId: 'combat.primary_fire.damage',
            amountDelta: 5,
            tags: ['registered-rust-module'],
            proposalHash: stableHash({ kind: 'game-extension-proposal', hook: hookHashRecord(hook) }),
        },
        diagnostics: [],
        trace: [{
                step: 1,
                code: 'module.proposed_damage_modifier',
                message: 'test bridge returned a typed damage modifier',
                refs: [hook.moduleRef.moduleId, hook.moduleRef.version, hook.moduleRef.contractHash],
            }],
        proposalHash: stableHash({ kind: 'game-extension-hook', hook: hookHashRecord(hook) }),
    };
}
function acceptedReplayEvidence(hook, primaryFire) {
    return {
        moduleRef: hook.moduleRef,
        hookId: hook.hookId,
        requestId: hook.requestId,
        inputHash: hook.inputHash,
        proposalHash: stableHash({ kind: 'game-extension-hook', hook: hookHashRecord(hook) }),
        validationStatus: 'accepted',
        eventHashes: [primaryFire.replayHash],
        rejectionHashes: [],
        replayHash: stableHash({
            kind: 'game-extension-replay',
            hook: hookHashRecord(hook),
            primaryFire: primaryFire.replayHash,
        }),
    };
}
function hookHashRecord(hook) {
    return {
        moduleId: hook.moduleRef.moduleId,
        moduleVersion: hook.moduleRef.version,
        hookId: hook.hookId,
        requestId: hook.requestId,
        tick: hook.tick,
        source: hook.source,
        target: hook.target,
        baseDamage: hook.baseDamage,
        rangeMillimeters: hook.rangeMillimeters,
        inputHash: hook.inputHash,
    };
}
//# sourceMappingURL=runtime-session-game-extension-fire.test.js.map