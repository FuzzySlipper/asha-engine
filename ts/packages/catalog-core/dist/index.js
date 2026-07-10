const DEFAULT_OWNERSHIP = {
    gameOwned: [
        'displayName',
        'playerController',
        'weapon',
        'enemyBehavior',
        'encounter',
        'generator',
    ],
    engineOwned: [
        'schemaValidation',
        'runtimeAuthority',
        'collisionResolution',
        'combatDamageApplication',
        'policyExecution',
        'proceduralGeneration',
    ],
};
const FPS_GAMEPLAY_PRESET_AUTHORITY_BOUNDARY = {
    catalogCoreRole: 'descriptive_config',
    shapeValidation: {
        owner: '@asha/catalog-core',
        scope: 'dto_shape_and_consumer_tuning_ranges_only',
        authorizesRuntime: false,
    },
    runtimeValidation: {
        owner: 'rust_runtime_session_authority',
        surfaces: [
            'RuntimeSessionFacade.loadEcrpProject',
            'RuntimeSessionFacade.applyCollisionConstrainedCameraInput',
            'RuntimeSessionFacade.submitRuntimeActionIntent',
            'RuntimeSessionFacade.runAutonomousPolicyTick',
            'RuntimeSessionFacade.requestEncounterTransition',
            'RuntimeSessionFacade.requestSessionRestart',
        ],
        ownerDocs: [
            'docs/entity-definition-schema.md',
            'docs/ecrp-capability-rule-ownership.md',
            'docs/runtime-session-facade.md',
        ],
    },
    semanticOwners: {
        bootstrap: 'svc-entity-authoring',
        lifecycle: 'rule-lifecycle',
        collision: 'svc-collision',
        combat: 'svc-combat',
        nav: 'svc-pathfinding',
        generation: 'svc-levelgen',
    },
    nonClaims: [
        'not_runtime_acceptance_authority',
        'not_capability_mutation_authority',
        'not_combat_damage_authority',
        'not_collision_resolution_authority',
        'not_policy_execution_authority',
        'not_procedural_generation_authority',
    ],
};
export const GENERATED_TUNNEL_DEFAULT_FPS_PRESET = {
    kind: 'fps_gameplay_preset.v0',
    presetId: 'asha.generated_tunnel.default_fps.v0',
    displayName: 'Generated Tunnel Default FPS',
    source: {
        kind: 'project_bundle.gameplay_preset',
        projectId: 'asha-demo',
        path: 'catalog/gameplay/generated-tunnel-default-fps.json',
    },
    playerController: {
        moveSpeedUnitsPerSecond: 3,
        sprintMultiplier: 1,
        lookSensitivityDegreesPerPixel: 0.1,
        cameraHeightUnits: 1.5,
        collisionHalfExtents: [0.25, 0.25, 0.25],
        maxPitchDegrees: 89,
    },
    weapon: {
        weaponId: 'weapon.primary_fire.generated_tunnel.v0',
        action: 'primary_fire',
        damage: 40,
        rangeUnits: 8,
        cooldownTicks: 4,
        ammo: 2,
        traceRadiusUnits: 0,
    },
    enemyBehavior: {
        policyRef: 'generated_tunnel_enemy_policy_loop.v0',
        entityDefinitionId: 'entity.enemy.generated_tunnel.basic.v0',
        navProjectionRef: 'generated_tunnel_nav_projection',
        desiredRangeUnits: 3.5,
        primaryFireEnabled: true,
    },
    encounter: {
        presetId: 'generated-tunnel-small-encounter',
        enemyDefinitionId: 'entity.enemy.generated_tunnel.basic.v0',
        enemyCount: 1,
        spawnMarkerIds: ['exit_hint'],
    },
    generator: {
        presetId: 'tiny-enclosed',
        seed: 17,
        outputHash: 'a9b504096397f5b4',
        renderProjectionHash: 'fnv1a64:21eb8696f6f3b5c4',
        collisionProjectionHash: 'fnv1a64:b2312fbcfb060db3',
    },
    ownership: DEFAULT_OWNERSHIP,
};
export const GENERATED_TUNNEL_GAMEPLAY_PRESET_CATALOG = {
    kind: 'fps_gameplay_preset_catalog.v0',
    catalogId: 'asha.generated_tunnel.gameplay_catalog.v0',
    source: {
        kind: 'project_bundle.gameplay_catalog',
        projectId: 'asha-demo',
        path: 'catalog/gameplay/catalog.json',
    },
    defaultPresetId: 'asha.generated_tunnel.default_fps.v0',
    presets: [GENERATED_TUNNEL_DEFAULT_FPS_PRESET],
};
export const GENERATED_TUNNEL_FPS_ECRP_OBJECT_MODEL = {
    kind: 'fps_ecrp_object_model.v0',
    modelId: 'asha.generated_tunnel.fps_ecrp_object_model.v0',
    source: {
        kind: 'project_bundle.gameplay_catalog',
        projectId: 'asha-demo',
        path: 'catalog/ecrp/fps-object-model.json',
    },
    entries: [
        {
            runtimeRole: 'player',
            entityDefinitionId: 'actor/demo-player',
            displayName: 'Demo Player',
            sourcePath: 'catalogs/actors/demo-player.entity.json',
            gameplayPresetRefs: ['asha.generated_tunnel.default_fps.v0'],
            capabilityKinds: [
                'transform',
                'collisionBody',
                'controller',
                'health',
                'weaponMount',
                'renderProjection',
                'faction',
            ],
            ruleOwners: [
                'EntityBootstrap',
                'LifecycleRule',
                'TransformRule',
                'MovementRule',
                'CollisionRule',
                'CombatRule',
                'RenderProjectionRule',
            ],
            policyRefs: ['browser_fps_input_collector.v0'],
            domainEvents: [
                'runtime_session.bootstrap_entity.v0',
                'runtime_session.camera_input.v0',
                'runtime_session.collision_constrained_camera_input.v0',
                'runtime_action.primary_fire.v0',
            ],
            projections: [
                'runtime_session.ecrp_readout.v0',
                'runtime_session.camera_projection.v0',
                'runtime_session.combat_readout.v0',
                'runtime_session.lifecycle_status.v0',
                'renderer_three.browser_surface.v0',
                'demo_hud_overlay.v0',
            ],
            runtimeSurfaces: [
                'RuntimeSessionFacade.readEcrpRuntimeReadout',
                'RuntimeSessionFacade.applyCollisionConstrainedCameraInput',
                'RuntimeSessionFacade.submitRuntimeActionIntent',
                'RuntimeSessionFacade.readCameraProjection',
                'RuntimeSessionFacade.readCombatReadout',
                'RuntimeSessionFacade.readLifecycleStatus',
                'BrowserFpsInputCollector',
                'mountAshaRendererBrowserSurface',
            ],
        },
        {
            runtimeRole: 'enemy',
            entityDefinitionId: 'actor/generated-tunnel-enemy',
            displayName: 'Generated Tunnel Enemy',
            sourcePath: 'catalogs/actors/generated-tunnel-enemy.entity.json',
            gameplayPresetRefs: ['asha.generated_tunnel.default_fps.v0'],
            capabilityKinds: [
                'transform',
                'collisionBody',
                'health',
                'renderProjection',
                'policyBinding',
                'spawnMarker',
                'faction',
            ],
            ruleOwners: [
                'EntityBootstrap',
                'LifecycleRule',
                'CollisionRule',
                'CombatRule',
                'EncounterRule',
                'PolicyRule',
                'NavRule',
                'RenderProjectionRule',
            ],
            policyRefs: [
                'policy.enemy.generated_tunnel.v0',
                'generated_tunnel_enemy_policy_loop.v0',
            ],
            domainEvents: [
                'runtime_session.bootstrap_entity.v0',
                'enemy_policy.move_toward_target.v0',
                'enemy_policy.primary_fire_intent.v0',
                'runtime_action.primary_fire.v0',
                'runtime_lifecycle.enemy_defeated.v0',
            ],
            projections: [
                'runtime_session.ecrp_readout.v0',
                'runtime_session.combat_readout.v0',
                'runtime_session.combat_feedback_projection.v0',
                'runtime_session.lifecycle_status.v0',
                'runtime_session.generated_tunnel_readout.v0',
                'runtime_session.nav_projection.v0',
                'renderer_three.browser_surface.v0',
                'demo_hud_overlay.v0',
            ],
            runtimeSurfaces: [
                'RuntimeSessionFacade.readEcrpRuntimeReadout',
                'RuntimeSessionFacade.submitRuntimeActionIntent',
                'RuntimeSessionFacade.readCombatReadout',
                'RuntimeSessionFacade.readCombatFeedbackProjection',
                'RuntimeSessionFacade.readLifecycleStatus',
                'RuntimeSessionFacade.readGeneratedTunnelReadout',
                'RuntimeSessionFacade.runAutonomousPolicyTick',
                'RuntimeSessionFacade.readNavProjection',
                'mountAshaRendererBrowserSurface',
            ],
        },
    ],
    runtimeContract: {
        ecrpReadoutKind: 'runtime_session.ecrp_readout.v0',
        projectBundleId: 'asha-demo',
        gameplayCatalogId: 'asha.generated_tunnel.gameplay_catalog.v0',
    },
    ownership: {
        authoritative: [
            'runtime entity lifecycle',
            'capability state mutation',
            'collision resolution',
            'combat damage application',
            'policy proposal validation',
            'nav/path projection',
            'render projection state',
        ],
        consumerOwned: [
            'input collection',
            'HUD placement',
            'browser pointer-lock shell',
            'render canvas mounting',
        ],
    },
};
export function validateFpsGameplayPreset(preset) {
    const diagnostics = [];
    validateRoot(preset, diagnostics);
    validateSource(preset.source, 'source', diagnostics);
    validatePlayerController(preset.playerController, 'playerController', diagnostics);
    validateWeapon(preset.weapon, 'weapon', diagnostics);
    validateEnemyBehavior(preset.enemyBehavior, 'enemyBehavior', diagnostics);
    validateEncounter(preset.encounter, 'encounter', diagnostics);
    validateGenerator(preset.generator, 'generator', diagnostics);
    validateOwnership(preset.ownership, 'ownership', diagnostics);
    if (diagnostics.length > 0) {
        return {
            kind: 'fps_gameplay_preset_validation.v0',
            valid: false,
            diagnostics,
            readout: null,
            rejectedHash: stableHash({
                presetId: stringValue(preset.presetId),
                diagnosticCodes: diagnostics.map((diagnostic) => diagnostic.code),
                diagnosticPaths: diagnostics.map((diagnostic) => diagnostic.path),
            }),
        };
    }
    return {
        kind: 'fps_gameplay_preset_validation.v0',
        valid: true,
        diagnostics: [],
        readout: buildFpsGameplayPresetReadout(preset),
        rejectedHash: null,
    };
}
export function readDefaultFpsGameplayPreset() {
    const report = validateFpsGameplayPreset(GENERATED_TUNNEL_DEFAULT_FPS_PRESET);
    if (report.readout === null) {
        throw new Error('Default FPS gameplay preset failed validation');
    }
    return report.readout;
}
export function readFpsGameplayPresetCatalog() {
    const defaultPreset = readDefaultFpsGameplayPreset();
    return {
        kind: 'fps_gameplay_preset_catalog_readout.v0',
        catalog: GENERATED_TUNNEL_GAMEPLAY_PRESET_CATALOG,
        defaultPreset,
        hashes: {
            catalogHash: stableHash({
                catalogId: GENERATED_TUNNEL_GAMEPLAY_PRESET_CATALOG.catalogId,
                defaultPresetId: GENERATED_TUNNEL_GAMEPLAY_PRESET_CATALOG.defaultPresetId,
                presetHashes: [defaultPreset.hashes.presetHash],
            }),
            defaultPresetHash: defaultPreset.hashes.presetHash,
        },
        consumerOwnership: DEFAULT_OWNERSHIP,
        authorityBoundary: FPS_GAMEPLAY_PRESET_AUTHORITY_BOUNDARY,
    };
}
export function readFpsEcrpObjectModel() {
    const playerEntry = findFpsEcrpObjectModelEntry('player');
    const enemyEntry = findFpsEcrpObjectModelEntry('enemy');
    const surfaceRefs = GENERATED_TUNNEL_FPS_ECRP_OBJECT_MODEL.entries.flatMap((entry) => entry.runtimeSurfaces);
    const playerEntryHash = stableHash(playerEntry);
    const enemyEntryHash = stableHash(enemyEntry);
    const surfaceHash = stableHash([...new Set(surfaceRefs)].sort());
    return {
        kind: 'fps_ecrp_object_model_readout.v0',
        model: GENERATED_TUNNEL_FPS_ECRP_OBJECT_MODEL,
        hashes: {
            modelHash: stableHash({
                model: GENERATED_TUNNEL_FPS_ECRP_OBJECT_MODEL,
                playerEntryHash,
                enemyEntryHash,
                surfaceHash,
            }),
            playerEntryHash,
            enemyEntryHash,
            surfaceHash,
        },
        migrationTargets: {
            projectBundle: 'ProjectBundle',
            entityDefinitions: 'EntityDefinition[]',
            sceneDocument: 'SceneDocument',
            runtimeReadout: 'RuntimeSessionFacade.readEcrpRuntimeReadout',
            rendererSurface: 'mountAshaRendererBrowserSurface',
        },
        nonClaims: [
            'not_runtime_authority',
            'not_demo_local_entity_model',
            'not_framework_ecs',
            'not_arbitrary_json_payload',
        ],
    };
}
export function findFpsEcrpObjectModelEntry(role) {
    const entry = GENERATED_TUNNEL_FPS_ECRP_OBJECT_MODEL.entries.find((candidate) => candidate.runtimeRole === role);
    if (entry === undefined) {
        throw new Error(`Unknown FPS ECRP object model role: ${role}`);
    }
    return entry;
}
function buildFpsGameplayPresetReadout(preset) {
    const tuningHash = stableHash({
        playerController: preset.playerController,
        weapon: preset.weapon,
        enemyBehavior: preset.enemyBehavior,
        encounter: preset.encounter,
        generator: preset.generator,
    });
    const referenceHash = stableHash({
        entityDefinitionId: preset.enemyBehavior.entityDefinitionId,
        encounterPresetId: preset.encounter.presetId,
        generatorPresetId: preset.generator.presetId,
        generatorOutputHash: preset.generator.outputHash,
        policyRef: preset.enemyBehavior.policyRef,
    });
    return {
        kind: 'fps_gameplay_preset_readout.v0',
        preset,
        fixturePath: 'harness/fixtures/gameplay-presets/generated-tunnel-default-fps.snapshot.txt',
        hashes: {
            presetHash: stableHash({
                presetId: preset.presetId,
                displayName: preset.displayName,
                source: preset.source,
                tuningHash,
                referenceHash,
            }),
            tuningHash,
            referenceHash,
        },
        migration: {
            playerControllerConstants: 'BrowserFpsInputCollector options',
            weaponConstants: 'RuntimeSession primary_fire combat readout defaults',
            enemyConstants: 'generated tunnel enemy policy fixture',
            encounterConstants: 'generated-tunnel-small-encounter',
            generatorConstants: 'tiny-enclosed generated tunnel readout',
        },
        nonClaims: [
            'not_runtime_authority',
            'not_demo_local_constants',
            'not_arbitrary_json_catalog',
            'not_editor_ui',
        ],
        authorityBoundary: FPS_GAMEPLAY_PRESET_AUTHORITY_BOUNDARY,
    };
}
function validateRoot(preset, diagnostics) {
    if (!requireObject(preset, 'preset', diagnostics)) {
        return;
    }
    requireExactKeys(preset, [
        'kind',
        'presetId',
        'displayName',
        'source',
        'playerController',
        'weapon',
        'enemyBehavior',
        'encounter',
        'generator',
        'ownership',
    ], 'preset', diagnostics);
    requireLiteral(preset.kind, 'fps_gameplay_preset.v0', 'kind', diagnostics);
    requireNonEmptyString(preset.presetId, 'presetId', diagnostics);
    requireNonEmptyString(preset.displayName, 'displayName', diagnostics);
}
function validateSource(source, path, diagnostics) {
    if (!requireObject(source, path, diagnostics)) {
        return;
    }
    requireExactKeys(source, ['kind', 'projectId', 'path'], path, diagnostics);
    requireLiteral(source.kind, 'project_bundle.gameplay_preset', `${path}.kind`, diagnostics);
    requireNonEmptyString(source.projectId, `${path}.projectId`, diagnostics);
    requireNonEmptyString(source.path, `${path}.path`, diagnostics);
}
function validatePlayerController(tuning, path, diagnostics) {
    if (!requireObject(tuning, path, diagnostics)) {
        return;
    }
    requireExactKeys(tuning, [
        'moveSpeedUnitsPerSecond',
        'sprintMultiplier',
        'lookSensitivityDegreesPerPixel',
        'cameraHeightUnits',
        'collisionHalfExtents',
        'maxPitchDegrees',
    ], path, diagnostics);
    requireFiniteRange(tuning.moveSpeedUnitsPerSecond, 0.01, 20, `${path}.moveSpeedUnitsPerSecond`, diagnostics);
    requireFiniteRange(tuning.sprintMultiplier, 1, 4, `${path}.sprintMultiplier`, diagnostics);
    requireFiniteRange(tuning.lookSensitivityDegreesPerPixel, 0.001, 2, `${path}.lookSensitivityDegreesPerPixel`, diagnostics);
    requireFiniteRange(tuning.cameraHeightUnits, 0.1, 4, `${path}.cameraHeightUnits`, diagnostics);
    requireFiniteRange(tuning.maxPitchDegrees, 1, 89, `${path}.maxPitchDegrees`, diagnostics);
    validateVec3(tuning.collisionHalfExtents, `${path}.collisionHalfExtents`, diagnostics);
}
function validateWeapon(tuning, path, diagnostics) {
    if (!requireObject(tuning, path, diagnostics)) {
        return;
    }
    requireExactKeys(tuning, ['weaponId', 'action', 'damage', 'rangeUnits', 'cooldownTicks', 'ammo', 'traceRadiusUnits'], path, diagnostics);
    requireNonEmptyString(tuning.weaponId, `${path}.weaponId`, diagnostics);
    requireLiteral(tuning.action, 'primary_fire', `${path}.action`, diagnostics);
    requireFiniteRange(tuning.damage, 1, 1000, `${path}.damage`, diagnostics);
    requireFiniteRange(tuning.rangeUnits, 0.1, 1000, `${path}.rangeUnits`, diagnostics);
    requireIntegerRange(tuning.cooldownTicks, 0, 600, `${path}.cooldownTicks`, diagnostics);
    requireIntegerRange(tuning.ammo, 0, 999, `${path}.ammo`, diagnostics);
    requireFiniteRange(tuning.traceRadiusUnits, 0, 10, `${path}.traceRadiusUnits`, diagnostics);
}
function validateEnemyBehavior(tuning, path, diagnostics) {
    if (!requireObject(tuning, path, diagnostics)) {
        return;
    }
    requireExactKeys(tuning, ['policyRef', 'entityDefinitionId', 'navProjectionRef', 'desiredRangeUnits', 'primaryFireEnabled'], path, diagnostics);
    requireNonEmptyString(tuning.policyRef, `${path}.policyRef`, diagnostics);
    requireNonEmptyString(tuning.entityDefinitionId, `${path}.entityDefinitionId`, diagnostics);
    requireNonEmptyString(tuning.navProjectionRef, `${path}.navProjectionRef`, diagnostics);
    requireFiniteRange(tuning.desiredRangeUnits, 0, 100, `${path}.desiredRangeUnits`, diagnostics);
    requireBoolean(tuning.primaryFireEnabled, `${path}.primaryFireEnabled`, diagnostics);
}
function validateEncounter(tuning, path, diagnostics) {
    if (!requireObject(tuning, path, diagnostics)) {
        return;
    }
    requireExactKeys(tuning, ['presetId', 'enemyDefinitionId', 'enemyCount', 'spawnMarkerIds'], path, diagnostics);
    requireNonEmptyString(tuning.presetId, `${path}.presetId`, diagnostics);
    requireNonEmptyString(tuning.enemyDefinitionId, `${path}.enemyDefinitionId`, diagnostics);
    requireIntegerRange(tuning.enemyCount, 1, 100, `${path}.enemyCount`, diagnostics);
    validateStringRefs(tuning.spawnMarkerIds, `${path}.spawnMarkerIds`, diagnostics);
}
function validateGenerator(tuning, path, diagnostics) {
    if (!requireObject(tuning, path, diagnostics)) {
        return;
    }
    requireExactKeys(tuning, ['presetId', 'seed', 'outputHash', 'renderProjectionHash', 'collisionProjectionHash'], path, diagnostics);
    requireNonEmptyString(tuning.presetId, `${path}.presetId`, diagnostics);
    requireIntegerRange(tuning.seed, 0, Number.MAX_SAFE_INTEGER, `${path}.seed`, diagnostics);
    requireNonEmptyString(tuning.outputHash, `${path}.outputHash`, diagnostics);
    requireNonEmptyString(tuning.renderProjectionHash, `${path}.renderProjectionHash`, diagnostics);
    requireNonEmptyString(tuning.collisionProjectionHash, `${path}.collisionProjectionHash`, diagnostics);
}
function validateOwnership(ownership, path, diagnostics) {
    if (!requireObject(ownership, path, diagnostics)) {
        return;
    }
    requireExactKeys(ownership, ['gameOwned', 'engineOwned'], path, diagnostics);
    validateStringRefs(ownership.gameOwned, `${path}.gameOwned`, diagnostics);
    validateStringRefs(ownership.engineOwned, `${path}.engineOwned`, diagnostics);
}
function requireObject(value, path, diagnostics) {
    if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        diagnostics.push({
            code: 'missingRequiredField',
            path,
            detail: `${path} must be an object`,
        });
        return false;
    }
    return true;
}
function requireExactKeys(value, allowedKeys, path, diagnostics) {
    if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return;
    }
    const allowed = new Set(allowedKeys);
    for (const key of Object.keys(value)) {
        if (!allowed.has(key)) {
            diagnostics.push({
                code: key === 'payload' ? 'arbitraryPayloadRejected' : 'unexpectedField',
                path: `${path}.${key}`,
                detail: `${path}.${key} is not part of fps_gameplay_preset.v0`,
            });
        }
    }
    for (const key of allowedKeys) {
        if (!Object.hasOwn(value, key)) {
            diagnostics.push({
                code: 'missingRequiredField',
                path: `${path}.${key}`,
                detail: `${path}.${key} is required`,
            });
        }
    }
}
function requireLiteral(value, expected, path, diagnostics) {
    if (value !== expected) {
        diagnostics.push({
            code: 'invalidKind',
            path,
            detail: `${path} must be ${expected}`,
        });
    }
}
function requireNonEmptyString(value, path, diagnostics) {
    if (typeof value !== 'string' || value.trim().length === 0) {
        diagnostics.push({
            code: 'emptyReference',
            path,
            detail: `${path} must be a non-empty string`,
        });
    }
}
function requireBoolean(value, path, diagnostics) {
    if (typeof value !== 'boolean') {
        diagnostics.push({
            code: 'missingRequiredField',
            path,
            detail: `${path} must be boolean`,
        });
    }
}
function requireFiniteRange(value, minInclusive, maxInclusive, path, diagnostics) {
    if (!Number.isFinite(value) || value < minInclusive || value > maxInclusive) {
        diagnostics.push({
            code: 'invalidNumberRange',
            path,
            detail: `${path} must be finite and in [${minInclusive}, ${maxInclusive}]`,
        });
    }
}
function requireIntegerRange(value, minInclusive, maxInclusive, path, diagnostics) {
    if (!Number.isSafeInteger(value) || value < minInclusive || value > maxInclusive) {
        diagnostics.push({
            code: 'invalidIntegerRange',
            path,
            detail: `${path} must be a safe integer in [${minInclusive}, ${maxInclusive}]`,
        });
    }
}
function validateVec3(value, path, diagnostics) {
    if (!Array.isArray(value) || value.length !== 3) {
        diagnostics.push({
            code: 'missingRequiredField',
            path,
            detail: `${path} must be a vec3`,
        });
        return;
    }
    value.forEach((component, index) => requireFiniteRange(component, 0.001, 10, `${path}.${index}`, diagnostics));
}
function validateStringRefs(refs, path, diagnostics) {
    if (refs.length === 0) {
        diagnostics.push({
            code: 'emptyReference',
            path,
            detail: `${path} must contain at least one reference`,
        });
        return;
    }
    const seen = new Set();
    refs.forEach((ref, index) => {
        requireNonEmptyString(ref, `${path}.${index}`, diagnostics);
        if (seen.has(ref)) {
            diagnostics.push({
                code: 'duplicateReference',
                path: `${path}.${index}`,
                detail: `${path} contains duplicate reference ${ref}`,
            });
        }
        seen.add(ref);
    });
}
function stringValue(value) {
    return typeof value === 'string' ? value : '<invalid>';
}
function stableHash(value) {
    const json = stableStringify(value);
    let hash = 0xcbf29ce484222325n;
    const prime = 0x100000001b3n;
    const mask = 0xffffffffffffffffn;
    for (let index = 0; index < json.length; index += 1) {
        hash ^= BigInt(json.charCodeAt(index));
        hash = (hash * prime) & mask;
    }
    return `fnv1a64:${hash.toString(16).padStart(16, '0')}`;
}
function stableStringify(value) {
    if (value === undefined) {
        return 'undefined';
    }
    if (value === null || typeof value !== 'object') {
        return JSON.stringify(value);
    }
    if (Array.isArray(value)) {
        const entries = value;
        return `[${entries.map((entry) => stableStringify(entry)).join(',')}]`;
    }
    const record = value;
    return `{${Object.keys(record)
        .sort()
        .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
        .join(',')}}`;
}
//# sourceMappingURL=index.js.map