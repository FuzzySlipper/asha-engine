export type FpsGameplayPresetKind = 'fps_gameplay_preset.v0';

export type FpsGameplayPresetCatalogKind = 'fps_gameplay_preset_catalog.v0';

export type FpsGameplayPresetId = 'asha.generated_tunnel.default_fps.v0';

export type FpsGameplayPresetSourceKind = 'project_bundle.gameplay_preset';

export type FpsGameplayCatalogSourceKind = 'project_bundle.gameplay_catalog';

export type FpsGameplayPresetDiagnosticCode =
  | 'arbitraryPayloadRejected'
  | 'duplicateReference'
  | 'emptyReference'
  | 'invalidIntegerRange'
  | 'invalidKind'
  | 'invalidNumberRange'
  | 'missingRequiredField'
  | 'unexpectedField';

export interface FpsGameplayPresetSourceTrace {
  readonly kind: FpsGameplayPresetSourceKind;
  readonly projectId: string;
  readonly path: string;
}

export interface FpsGameplayCatalogSourceTrace {
  readonly kind: FpsGameplayCatalogSourceKind;
  readonly projectId: string;
  readonly path: string;
}

export interface FpsPlayerControllerTuning {
  readonly moveSpeedUnitsPerSecond: number;
  readonly sprintMultiplier: number;
  readonly lookSensitivityDegreesPerPixel: number;
  readonly cameraHeightUnits: number;
  readonly collisionHalfExtents: readonly [number, number, number];
  readonly maxPitchDegrees: number;
}

export interface FpsWeaponFireTuning {
  readonly weaponId: 'weapon.primary_fire.generated_tunnel.v0';
  readonly action: 'primary_fire';
  readonly damage: number;
  readonly rangeUnits: number;
  readonly cooldownTicks: number;
  readonly ammo: number;
  readonly traceRadiusUnits: number;
}

export interface FpsEnemyBehaviorTuning {
  readonly policyRef: 'generated_tunnel_enemy_policy_loop.v0';
  readonly entityDefinitionId: 'entity.enemy.generated_tunnel.basic.v0';
  readonly navProjectionRef: 'generated_tunnel_nav_projection';
  readonly desiredRangeUnits: number;
  readonly primaryFireEnabled: boolean;
}

export interface FpsEncounterTuning {
  readonly presetId: 'generated-tunnel-small-encounter';
  readonly enemyDefinitionId: 'entity.enemy.generated_tunnel.basic.v0';
  readonly enemyCount: number;
  readonly spawnMarkerIds: readonly ['exit_hint'];
}

export interface FpsGeneratorPresetRef {
  readonly presetId: 'tiny-enclosed';
  readonly seed: 17;
  readonly outputHash: 'a9b504096397f5b4';
  readonly renderProjectionHash: 'fnv1a64:21eb8696f6f3b5c4';
  readonly collisionProjectionHash: 'fnv1a64:b2312fbcfb060db3';
}

export interface FpsGameplayOwnership {
  readonly gameOwned: readonly [
    'displayName',
    'playerController',
    'weapon',
    'enemyBehavior',
    'encounter',
    'generator',
  ];
  readonly engineOwned: readonly [
    'schemaValidation',
    'runtimeAuthority',
    'collisionResolution',
    'combatDamageApplication',
    'policyExecution',
    'proceduralGeneration',
  ];
}

export interface FpsGameplayPreset {
  readonly kind: FpsGameplayPresetKind;
  readonly presetId: FpsGameplayPresetId;
  readonly displayName: string;
  readonly source: FpsGameplayPresetSourceTrace;
  readonly playerController: FpsPlayerControllerTuning;
  readonly weapon: FpsWeaponFireTuning;
  readonly enemyBehavior: FpsEnemyBehaviorTuning;
  readonly encounter: FpsEncounterTuning;
  readonly generator: FpsGeneratorPresetRef;
  readonly ownership: FpsGameplayOwnership;
}

export interface FpsGameplayPresetCatalog {
  readonly kind: FpsGameplayPresetCatalogKind;
  readonly catalogId: 'asha.generated_tunnel.gameplay_catalog.v0';
  readonly source: FpsGameplayCatalogSourceTrace;
  readonly defaultPresetId: FpsGameplayPresetId;
  readonly presets: readonly [FpsGameplayPreset];
}

export interface FpsGameplayPresetDiagnostic {
  readonly code: FpsGameplayPresetDiagnosticCode;
  readonly path: string;
  readonly detail: string;
}

export interface FpsGameplayPresetReadout {
  readonly kind: 'fps_gameplay_preset_readout.v0';
  readonly preset: FpsGameplayPreset;
  readonly fixturePath: 'harness/fixtures/gameplay-presets/generated-tunnel-default-fps.snapshot.txt';
  readonly hashes: {
    readonly presetHash: string;
    readonly tuningHash: string;
    readonly referenceHash: string;
  };
  readonly migration: {
    readonly playerControllerConstants: 'BrowserFpsInputCollector options';
    readonly weaponConstants: 'RuntimeSession primary_fire combat readout defaults';
    readonly enemyConstants: 'generated tunnel enemy policy fixture';
    readonly encounterConstants: 'generated-tunnel-small-encounter';
    readonly generatorConstants: 'tiny-enclosed generated tunnel readout';
  };
  readonly nonClaims: readonly [
    'not_runtime_authority',
    'not_demo_local_constants',
    'not_arbitrary_json_catalog',
    'not_editor_ui',
  ];
  readonly authorityBoundary: FpsGameplayPresetAuthorityBoundary;
}

export interface FpsGameplayPresetValidationReport {
  readonly kind: 'fps_gameplay_preset_validation.v0';
  readonly valid: boolean;
  readonly diagnostics: readonly FpsGameplayPresetDiagnostic[];
  readonly readout: FpsGameplayPresetReadout | null;
  readonly rejectedHash: string | null;
}

export interface FpsGameplayPresetCatalogReadout {
  readonly kind: 'fps_gameplay_preset_catalog_readout.v0';
  readonly catalog: FpsGameplayPresetCatalog;
  readonly defaultPreset: FpsGameplayPresetReadout;
  readonly hashes: {
    readonly catalogHash: string;
    readonly defaultPresetHash: string;
  };
  readonly consumerOwnership: FpsGameplayOwnership;
  readonly authorityBoundary: FpsGameplayPresetAuthorityBoundary;
}

export interface FpsGameplayPresetAuthorityBoundary {
  readonly catalogCoreRole: 'descriptive_config';
  readonly shapeValidation: {
    readonly owner: '@asha/catalog-core';
    readonly scope: 'dto_shape_and_consumer_tuning_ranges_only';
    readonly authorizesRuntime: false;
  };
  readonly runtimeValidation: {
    readonly owner: 'rust_runtime_session_authority';
    readonly surfaces: readonly [
      'RuntimeSessionFacade.loadEcrpProject',
      'RuntimeSessionFacade.applyCollisionConstrainedCameraInput',
      'RuntimeSessionFacade.submitRuntimeActionIntent',
      'RuntimeSessionFacade.runAutonomousPolicyTick',
      'RuntimeSessionFacade.requestEncounterTransition',
      'RuntimeSessionFacade.requestSessionRestart',
    ];
    readonly ownerDocs: readonly [
      'docs/entity-definition-schema.md',
      'docs/ecrp-capability-rule-ownership.md',
      'docs/runtime-session-facade.md',
    ];
  };
  readonly semanticOwners: {
    readonly bootstrap: 'svc-entity-authoring';
    readonly lifecycle: 'rule-lifecycle';
    readonly collision: 'svc-collision';
    readonly combat: 'svc-combat';
    readonly nav: 'svc-pathfinding';
    readonly generation: 'svc-levelgen';
  };
  readonly nonClaims: readonly [
    'not_runtime_acceptance_authority',
    'not_capability_mutation_authority',
    'not_combat_damage_authority',
    'not_collision_resolution_authority',
    'not_policy_execution_authority',
    'not_procedural_generation_authority',
  ];
}

export type FpsEcrpObjectModelKind = 'fps_ecrp_object_model.v0';

export type FpsEcrpRuntimeRole = 'player' | 'enemy';

export type FpsEcrpCapabilityKind =
  | 'transform'
  | 'collisionBody'
  | 'controller'
  | 'health'
  | 'weaponMount'
  | 'renderProjection'
  | 'policyBinding'
  | 'spawnMarker'
  | 'faction';

export type FpsEcrpRuleOwner =
  | 'EntityBootstrap'
  | 'LifecycleRule'
  | 'TransformRule'
  | 'MovementRule'
  | 'CollisionRule'
  | 'RenderProjectionRule'
  | 'RelationRule'
  | 'CombatRule'
  | 'EncounterRule'
  | 'PolicyRule'
  | 'NavRule';

export type FpsEcrpPolicyRef =
  | 'browser_fps_input_collector.v0'
  | 'policy.enemy.generated_tunnel.v0'
  | 'generated_tunnel_enemy_policy_loop.v0';

export type FpsEcrpDomainEventRef =
  | 'runtime_session.bootstrap_entity.v0'
  | 'runtime_session.camera_input.v0'
  | 'runtime_session.collision_constrained_camera_input.v0'
  | 'runtime_action.primary_fire.v0'
  | 'runtime_lifecycle.enemy_defeated.v0'
  | 'enemy_policy.move_toward_target.v0'
  | 'enemy_policy.primary_fire_intent.v0';

export type FpsEcrpProjectionRef =
  | 'runtime_session.ecrp_readout.v0'
  | 'runtime_session.camera_projection.v0'
  | 'runtime_session.combat_readout.v0'
  | 'runtime_session.combat_feedback_projection.v0'
  | 'runtime_session.lifecycle_status.v0'
  | 'runtime_session.generated_tunnel_readout.v0'
  | 'runtime_session.nav_projection.v0'
  | 'renderer_three.browser_surface.v0'
  | 'demo_hud_overlay.v0';

export type FpsEcrpRuntimeSurfaceRef =
  | 'RuntimeSessionFacade.readEcrpRuntimeReadout'
  | 'RuntimeSessionFacade.applyCollisionConstrainedCameraInput'
  | 'RuntimeSessionFacade.submitRuntimeActionIntent'
  | 'RuntimeSessionFacade.readCameraProjection'
  | 'RuntimeSessionFacade.readCombatReadout'
  | 'RuntimeSessionFacade.readCombatFeedbackProjection'
  | 'RuntimeSessionFacade.readLifecycleStatus'
  | 'RuntimeSessionFacade.readGeneratedTunnelReadout'
  | 'RuntimeSessionFacade.runAutonomousPolicyTick'
  | 'RuntimeSessionFacade.readNavProjection'
  | 'BrowserFpsInputCollector'
  | 'mountAshaRendererBrowserSurface';

export interface FpsEcrpObjectModelEntry {
  readonly runtimeRole: FpsEcrpRuntimeRole;
  readonly entityDefinitionId: string;
  readonly displayName: string;
  readonly sourcePath: string;
  readonly gameplayPresetRefs: readonly FpsGameplayPresetId[];
  readonly capabilityKinds: readonly FpsEcrpCapabilityKind[];
  readonly ruleOwners: readonly FpsEcrpRuleOwner[];
  readonly policyRefs: readonly FpsEcrpPolicyRef[];
  readonly domainEvents: readonly FpsEcrpDomainEventRef[];
  readonly projections: readonly FpsEcrpProjectionRef[];
  readonly runtimeSurfaces: readonly FpsEcrpRuntimeSurfaceRef[];
}

export interface FpsEcrpObjectModel {
  readonly kind: FpsEcrpObjectModelKind;
  readonly modelId: 'asha.generated_tunnel.fps_ecrp_object_model.v0';
  readonly source: FpsGameplayCatalogSourceTrace;
  readonly entries: readonly FpsEcrpObjectModelEntry[];
  readonly runtimeContract: {
    readonly ecrpReadoutKind: 'runtime_session.ecrp_readout.v0';
    readonly projectBundleId: 'asha-demo';
    readonly gameplayCatalogId: FpsGameplayPresetCatalog['catalogId'];
  };
  readonly ownership: {
    readonly authoritative: readonly [
      'runtime entity lifecycle',
      'capability state mutation',
      'collision resolution',
      'combat damage application',
      'policy proposal validation',
      'nav/path projection',
      'render projection state',
    ];
    readonly consumerOwned: readonly [
      'input collection',
      'HUD placement',
      'browser pointer-lock shell',
      'render canvas mounting',
    ];
  };
}

export interface FpsEcrpObjectModelReadout {
  readonly kind: 'fps_ecrp_object_model_readout.v0';
  readonly model: FpsEcrpObjectModel;
  readonly hashes: {
    readonly modelHash: string;
    readonly playerEntryHash: string;
    readonly enemyEntryHash: string;
    readonly surfaceHash: string;
  };
  readonly migrationTargets: {
    readonly projectBundle: 'ProjectBundle';
    readonly entityDefinitions: 'EntityDefinition[]';
    readonly sceneDocument: 'SceneDocument';
    readonly runtimeReadout: 'RuntimeSessionFacade.readEcrpRuntimeReadout';
    readonly rendererSurface: 'mountAshaRendererBrowserSurface';
  };
  readonly nonClaims: readonly [
    'not_runtime_authority',
    'not_demo_local_entity_model',
    'not_framework_ecs',
    'not_arbitrary_json_payload',
  ];
}

type GameplayHashPrimitive = string | number | boolean | null;
type GameplayHashValue =
  | GameplayHashPrimitive
  | readonly GameplayHashValue[]
  | object;

interface GameplayHashRecord {
  readonly [key: string]: GameplayHashValue | undefined;
}

const DEFAULT_OWNERSHIP: FpsGameplayOwnership = {
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

const FPS_GAMEPLAY_PRESET_AUTHORITY_BOUNDARY: FpsGameplayPresetAuthorityBoundary = {
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

export const GENERATED_TUNNEL_DEFAULT_FPS_PRESET: FpsGameplayPreset = {
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

export const GENERATED_TUNNEL_GAMEPLAY_PRESET_CATALOG: FpsGameplayPresetCatalog = {
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

export const GENERATED_TUNNEL_FPS_ECRP_OBJECT_MODEL: FpsEcrpObjectModel = {
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

export function validateFpsGameplayPreset(
  preset: FpsGameplayPreset,
): FpsGameplayPresetValidationReport {
  const diagnostics: FpsGameplayPresetDiagnostic[] = [];
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

export function readDefaultFpsGameplayPreset(): FpsGameplayPresetReadout {
  const report = validateFpsGameplayPreset(GENERATED_TUNNEL_DEFAULT_FPS_PRESET);
  if (report.readout === null) {
    throw new Error('Default FPS gameplay preset failed validation');
  }
  return report.readout;
}

export function readFpsGameplayPresetCatalog(): FpsGameplayPresetCatalogReadout {
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

export function readFpsEcrpObjectModel(): FpsEcrpObjectModelReadout {
  const playerEntry = findFpsEcrpObjectModelEntry('player');
  const enemyEntry = findFpsEcrpObjectModelEntry('enemy');
  const surfaceRefs = GENERATED_TUNNEL_FPS_ECRP_OBJECT_MODEL.entries.flatMap(
    (entry) => entry.runtimeSurfaces,
  );
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

export function findFpsEcrpObjectModelEntry(
  role: FpsEcrpRuntimeRole,
): FpsEcrpObjectModelEntry {
  const entry = GENERATED_TUNNEL_FPS_ECRP_OBJECT_MODEL.entries.find(
    (candidate) => candidate.runtimeRole === role,
  );
  if (entry === undefined) {
    throw new Error(`Unknown FPS ECRP object model role: ${role}`);
  }
  return entry;
}

function buildFpsGameplayPresetReadout(preset: FpsGameplayPreset): FpsGameplayPresetReadout {
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

function validateRoot(
  preset: FpsGameplayPreset,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
  if (!requireObject(preset, 'preset', diagnostics)) {
    return;
  }
  requireExactKeys(
    preset,
    [
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
    ],
    'preset',
    diagnostics,
  );
  requireLiteral(preset.kind, 'fps_gameplay_preset.v0', 'kind', diagnostics);
  requireNonEmptyString(preset.presetId, 'presetId', diagnostics);
  requireNonEmptyString(preset.displayName, 'displayName', diagnostics);
}

function validateSource(
  source: FpsGameplayPresetSourceTrace,
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
  if (!requireObject(source, path, diagnostics)) {
    return;
  }
  requireExactKeys(source, ['kind', 'projectId', 'path'], path, diagnostics);
  requireLiteral(source.kind, 'project_bundle.gameplay_preset', `${path}.kind`, diagnostics);
  requireNonEmptyString(source.projectId, `${path}.projectId`, diagnostics);
  requireNonEmptyString(source.path, `${path}.path`, diagnostics);
}

function validatePlayerController(
  tuning: FpsPlayerControllerTuning,
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
  if (!requireObject(tuning, path, diagnostics)) {
    return;
  }
  requireExactKeys(
    tuning,
    [
      'moveSpeedUnitsPerSecond',
      'sprintMultiplier',
      'lookSensitivityDegreesPerPixel',
      'cameraHeightUnits',
      'collisionHalfExtents',
      'maxPitchDegrees',
    ],
    path,
    diagnostics,
  );
  requireFiniteRange(tuning.moveSpeedUnitsPerSecond, 0.01, 20, `${path}.moveSpeedUnitsPerSecond`, diagnostics);
  requireFiniteRange(tuning.sprintMultiplier, 1, 4, `${path}.sprintMultiplier`, diagnostics);
  requireFiniteRange(tuning.lookSensitivityDegreesPerPixel, 0.001, 2, `${path}.lookSensitivityDegreesPerPixel`, diagnostics);
  requireFiniteRange(tuning.cameraHeightUnits, 0.1, 4, `${path}.cameraHeightUnits`, diagnostics);
  requireFiniteRange(tuning.maxPitchDegrees, 1, 89, `${path}.maxPitchDegrees`, diagnostics);
  validateVec3(tuning.collisionHalfExtents, `${path}.collisionHalfExtents`, diagnostics);
}

function validateWeapon(
  tuning: FpsWeaponFireTuning,
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
  if (!requireObject(tuning, path, diagnostics)) {
    return;
  }
  requireExactKeys(
    tuning,
    ['weaponId', 'action', 'damage', 'rangeUnits', 'cooldownTicks', 'ammo', 'traceRadiusUnits'],
    path,
    diagnostics,
  );
  requireNonEmptyString(tuning.weaponId, `${path}.weaponId`, diagnostics);
  requireLiteral(tuning.action, 'primary_fire', `${path}.action`, diagnostics);
  requireFiniteRange(tuning.damage, 1, 1000, `${path}.damage`, diagnostics);
  requireFiniteRange(tuning.rangeUnits, 0.1, 1000, `${path}.rangeUnits`, diagnostics);
  requireIntegerRange(tuning.cooldownTicks, 0, 600, `${path}.cooldownTicks`, diagnostics);
  requireIntegerRange(tuning.ammo, 0, 999, `${path}.ammo`, diagnostics);
  requireFiniteRange(tuning.traceRadiusUnits, 0, 10, `${path}.traceRadiusUnits`, diagnostics);
}

function validateEnemyBehavior(
  tuning: FpsEnemyBehaviorTuning,
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
  if (!requireObject(tuning, path, diagnostics)) {
    return;
  }
  requireExactKeys(
    tuning,
    ['policyRef', 'entityDefinitionId', 'navProjectionRef', 'desiredRangeUnits', 'primaryFireEnabled'],
    path,
    diagnostics,
  );
  requireNonEmptyString(tuning.policyRef, `${path}.policyRef`, diagnostics);
  requireNonEmptyString(tuning.entityDefinitionId, `${path}.entityDefinitionId`, diagnostics);
  requireNonEmptyString(tuning.navProjectionRef, `${path}.navProjectionRef`, diagnostics);
  requireFiniteRange(tuning.desiredRangeUnits, 0, 100, `${path}.desiredRangeUnits`, diagnostics);
  requireBoolean(tuning.primaryFireEnabled, `${path}.primaryFireEnabled`, diagnostics);
}

function validateEncounter(
  tuning: FpsEncounterTuning,
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
  if (!requireObject(tuning, path, diagnostics)) {
    return;
  }
  requireExactKeys(tuning, ['presetId', 'enemyDefinitionId', 'enemyCount', 'spawnMarkerIds'], path, diagnostics);
  requireNonEmptyString(tuning.presetId, `${path}.presetId`, diagnostics);
  requireNonEmptyString(tuning.enemyDefinitionId, `${path}.enemyDefinitionId`, diagnostics);
  requireIntegerRange(tuning.enemyCount, 1, 100, `${path}.enemyCount`, diagnostics);
  validateStringRefs(tuning.spawnMarkerIds, `${path}.spawnMarkerIds`, diagnostics);
}

function validateGenerator(
  tuning: FpsGeneratorPresetRef,
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
  if (!requireObject(tuning, path, diagnostics)) {
    return;
  }
  requireExactKeys(
    tuning,
    ['presetId', 'seed', 'outputHash', 'renderProjectionHash', 'collisionProjectionHash'],
    path,
    diagnostics,
  );
  requireNonEmptyString(tuning.presetId, `${path}.presetId`, diagnostics);
  requireIntegerRange(tuning.seed, 0, Number.MAX_SAFE_INTEGER, `${path}.seed`, diagnostics);
  requireNonEmptyString(tuning.outputHash, `${path}.outputHash`, diagnostics);
  requireNonEmptyString(tuning.renderProjectionHash, `${path}.renderProjectionHash`, diagnostics);
  requireNonEmptyString(tuning.collisionProjectionHash, `${path}.collisionProjectionHash`, diagnostics);
}

function validateOwnership(
  ownership: FpsGameplayOwnership,
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
  if (!requireObject(ownership, path, diagnostics)) {
    return;
  }
  requireExactKeys(ownership, ['gameOwned', 'engineOwned'], path, diagnostics);
  validateStringRefs(ownership.gameOwned, `${path}.gameOwned`, diagnostics);
  validateStringRefs(ownership.engineOwned, `${path}.engineOwned`, diagnostics);
}

function requireObject(
  value: object,
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): boolean {
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

function requireExactKeys(
  value: object,
  allowedKeys: readonly string[],
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
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

function requireLiteral(
  value: string,
  expected: string,
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
  if (value !== expected) {
    diagnostics.push({
      code: 'invalidKind',
      path,
      detail: `${path} must be ${expected}`,
    });
  }
}

function requireNonEmptyString(
  value: string,
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
  if (typeof value !== 'string' || value.trim().length === 0) {
    diagnostics.push({
      code: 'emptyReference',
      path,
      detail: `${path} must be a non-empty string`,
    });
  }
}

function requireBoolean(
  value: boolean,
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
  if (typeof value !== 'boolean') {
    diagnostics.push({
      code: 'missingRequiredField',
      path,
      detail: `${path} must be boolean`,
    });
  }
}

function requireFiniteRange(
  value: number,
  minInclusive: number,
  maxInclusive: number,
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
  if (!Number.isFinite(value) || value < minInclusive || value > maxInclusive) {
    diagnostics.push({
      code: 'invalidNumberRange',
      path,
      detail: `${path} must be finite and in [${minInclusive}, ${maxInclusive}]`,
    });
  }
}

function requireIntegerRange(
  value: number,
  minInclusive: number,
  maxInclusive: number,
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
  if (!Number.isSafeInteger(value) || value < minInclusive || value > maxInclusive) {
    diagnostics.push({
      code: 'invalidIntegerRange',
      path,
      detail: `${path} must be a safe integer in [${minInclusive}, ${maxInclusive}]`,
    });
  }
}

function validateVec3(
  value: readonly [number, number, number],
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
  if (!Array.isArray(value) || value.length !== 3) {
    diagnostics.push({
      code: 'missingRequiredField',
      path,
      detail: `${path} must be a vec3`,
    });
    return;
  }
  value.forEach((component, index) =>
    requireFiniteRange(component, 0.001, 10, `${path}.${index}`, diagnostics),
  );
}

function validateStringRefs(
  refs: readonly string[],
  path: string,
  diagnostics: FpsGameplayPresetDiagnostic[],
): void {
  if (refs.length === 0) {
    diagnostics.push({
      code: 'emptyReference',
      path,
      detail: `${path} must contain at least one reference`,
    });
    return;
  }
  const seen = new Set<string>();
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

function stringValue(value: string): string {
  return typeof value === 'string' ? value : '<invalid>';
}

function stableHash(value: GameplayHashValue | undefined): string {
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

function stableStringify(value: GameplayHashValue | undefined): string {
  if (value === undefined) {
    return 'undefined';
  }
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    const entries = value as readonly GameplayHashValue[];
    return `[${entries.map((entry) => stableStringify(entry)).join(',')}]`;
  }
  const record = value as GameplayHashRecord;
  return `{${Object.keys(record)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
    .join(',')}}`;
}
