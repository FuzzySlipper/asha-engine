import { lifecycleHealth } from './runtime-session-lifecycle.js';
import { projectBundleHashRecord, stableHash } from './runtime-session-hash.js';
export function defaultRuntimeSessionEcrpProjectLoadInput(input) {
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
export function validateEcrpProjectLoadInput(input) {
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
    if (input.sceneDocument.kind !== 'SceneDocument') {
        diagnostics.push({
            code: 'missingPlacement',
            path: 'sceneDocument.placements',
            detail: 'SceneDocument placements are required',
        });
        return diagnostics;
    }
    const placements = input.sceneDocument.placements;
    const placed = new Set();
    const runtimeIds = new Set();
    placements.forEach((placement, index) => {
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
export function buildEcrpProjectState(input) {
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
export function lifecycleStateFromEcrpProject(state) {
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
export function buildEcrpRuntimeReadout(input) {
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
//# sourceMappingURL=runtime-session-ecrp.js.map