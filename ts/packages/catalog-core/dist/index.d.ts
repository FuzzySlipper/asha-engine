export type FpsGameplayPresetKind = 'fps_gameplay_preset.v0';
export type FpsGameplayPresetCatalogKind = 'fps_gameplay_preset_catalog.v0';
export type FpsGameplayPresetId = 'asha.generated_tunnel.default_fps.v0';
export type FpsGameplayPresetSourceKind = 'project_bundle.gameplay_preset';
export type FpsGameplayCatalogSourceKind = 'project_bundle.gameplay_catalog';
export type FpsGameplayPresetDiagnosticCode = 'arbitraryPayloadRejected' | 'duplicateReference' | 'emptyReference' | 'invalidIntegerRange' | 'invalidKind' | 'invalidNumberRange' | 'missingRequiredField' | 'unexpectedField';
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
        'generator'
    ];
    readonly engineOwned: readonly [
        'schemaValidation',
        'runtimeAuthority',
        'collisionResolution',
        'combatDamageApplication',
        'policyExecution',
        'proceduralGeneration'
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
        'not_editor_ui'
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
            'RuntimeSessionFacade.requestSessionRestart'
        ];
        readonly ownerDocs: readonly [
            'docs/entity-definition-schema.md',
            'docs/ecrp-capability-rule-ownership.md',
            'docs/runtime-session-facade.md'
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
        'not_procedural_generation_authority'
    ];
}
export type FpsEcrpObjectModelKind = 'fps_ecrp_object_model.v0';
export type FpsEcrpRuntimeRole = 'player' | 'enemy';
export type FpsEcrpCapabilityKind = 'transform' | 'collisionBody' | 'controller' | 'health' | 'weaponMount' | 'renderProjection' | 'policyBinding' | 'spawnMarker' | 'faction';
export type FpsEcrpRuleOwner = 'EntityBootstrap' | 'LifecycleRule' | 'TransformRule' | 'MovementRule' | 'CollisionRule' | 'RenderProjectionRule' | 'RelationRule' | 'CombatRule' | 'EncounterRule' | 'PolicyRule' | 'NavRule';
export type FpsEcrpPolicyRef = 'browser_fps_input_collector.v0' | 'policy.enemy.generated_tunnel.v0' | 'generated_tunnel_enemy_policy_loop.v0';
export type FpsEcrpDomainEventRef = 'runtime_session.bootstrap_entity.v0' | 'runtime_session.camera_input.v0' | 'runtime_session.collision_constrained_camera_input.v0' | 'runtime_action.primary_fire.v0' | 'runtime_lifecycle.enemy_defeated.v0' | 'enemy_policy.move_toward_target.v0' | 'enemy_policy.primary_fire_intent.v0';
export type FpsEcrpProjectionRef = 'runtime_session.ecrp_readout.v0' | 'runtime_session.camera_projection.v0' | 'runtime_session.combat_readout.v0' | 'runtime_session.combat_feedback_projection.v0' | 'runtime_session.lifecycle_status.v0' | 'runtime_session.generated_tunnel_readout.v0' | 'runtime_session.nav_projection.v0' | 'renderer_three.browser_surface.v0' | 'demo_hud_overlay.v0';
export type FpsEcrpRuntimeSurfaceRef = 'RuntimeSessionFacade.readEcrpRuntimeReadout' | 'RuntimeSessionFacade.applyCollisionConstrainedCameraInput' | 'RuntimeSessionFacade.submitRuntimeActionIntent' | 'RuntimeSessionFacade.readCameraProjection' | 'RuntimeSessionFacade.readCombatReadout' | 'RuntimeSessionFacade.readCombatFeedbackProjection' | 'RuntimeSessionFacade.readLifecycleStatus' | 'RuntimeSessionFacade.readGeneratedTunnelReadout' | 'RuntimeSessionFacade.runAutonomousPolicyTick' | 'RuntimeSessionFacade.readNavProjection' | 'BrowserFpsInputCollector' | 'mountAshaRendererBrowserSurface';
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
            'render projection state'
        ];
        readonly consumerOwned: readonly [
            'input collection',
            'HUD placement',
            'browser pointer-lock shell',
            'render canvas mounting'
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
        'not_arbitrary_json_payload'
    ];
}
export declare const GENERATED_TUNNEL_DEFAULT_FPS_PRESET: FpsGameplayPreset;
export declare const GENERATED_TUNNEL_GAMEPLAY_PRESET_CATALOG: FpsGameplayPresetCatalog;
export declare const GENERATED_TUNNEL_FPS_ECRP_OBJECT_MODEL: FpsEcrpObjectModel;
export declare function validateFpsGameplayPreset(preset: FpsGameplayPreset): FpsGameplayPresetValidationReport;
export declare function readDefaultFpsGameplayPreset(): FpsGameplayPresetReadout;
export declare function readFpsGameplayPresetCatalog(): FpsGameplayPresetCatalogReadout;
export declare function readFpsEcrpObjectModel(): FpsEcrpObjectModelReadout;
export declare function findFpsEcrpObjectModelEntry(role: FpsEcrpRuntimeRole): FpsEcrpObjectModelEntry;
//# sourceMappingURL=index.d.ts.map