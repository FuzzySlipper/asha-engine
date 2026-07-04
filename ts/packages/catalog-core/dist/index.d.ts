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
    readonly collisionProjectionHash: 'fnv1a64:78b242163cf67524';
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
}
export declare const GENERATED_TUNNEL_DEFAULT_FPS_PRESET: FpsGameplayPreset;
export declare const GENERATED_TUNNEL_GAMEPLAY_PRESET_CATALOG: FpsGameplayPresetCatalog;
export declare function validateFpsGameplayPreset(preset: FpsGameplayPreset): FpsGameplayPresetValidationReport;
export declare function readDefaultFpsGameplayPreset(): FpsGameplayPresetReadout;
export declare function readFpsGameplayPresetCatalog(): FpsGameplayPresetCatalogReadout;
//# sourceMappingURL=index.d.ts.map