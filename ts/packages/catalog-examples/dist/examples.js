import { GENERATED_TUNNEL_DEFAULT_FPS_PRESET, GENERATED_TUNNEL_GAMEPLAY_PRESET_CATALOG, readFpsGameplayPresetCatalog, validateFpsGameplayPreset, } from '@asha/catalog-core';
export const CATALOG_EXAMPLE_AUTHORITY_BOUNDARY = {
    packageRole: '@asha/catalog-examples',
    owns: ['fixture_data', 'invalid_fixture_builders', 'consumer_examples'],
    doesNotOwn: [
        'runtime_authority',
        'state_mutation',
        'command_validation',
        'collision_resolution',
        'combat_damage_application',
    ],
};
export const GENERATED_TUNNEL_ASSET_CATALOG_EXAMPLE = {
    entries: [
        {
            id: 'material/generated-tunnel/wall-grey',
            kind: 'material',
            version: 1,
            hash: 'fnv1a64:8cfd3f4c579e0d41',
            sourcePath: 'catalog/assets/materials/generated-tunnel-wall-grey.json',
            label: 'Generated Tunnel Wall Grey',
            dependencies: [],
            material: {
                render: {
                    color: { r: 0.42, g: 0.45, b: 0.49, a: 1 },
                    texture: null,
                    roughness: 0.84,
                    textureTint: { r: 1, g: 1, b: 1, a: 1 },
                    emissionColor: { r: 0.5, g: 0.5, b: 0.5, a: 1 },
                    emissive: 0,
                    uvStrategy: 'flat',
                },
                collision: {
                    solid: true,
                    collidable: true,
                    occludes: true,
                    structuralClass: 'structural',
                },
            },
        },
        {
            id: 'mesh/generated-tunnel/test-cube',
            kind: 'mesh',
            version: 1,
            hash: 'fnv1a64:48c8a8fd6f82e133',
            sourcePath: 'catalog/assets/meshes/generated-tunnel-test-cube.mesh.json',
            label: 'Generated Tunnel Test Cube',
            dependencies: [
                {
                    id: 'material/generated-tunnel/wall-grey',
                    version: { req: 'exact', value: 1 },
                    hash: 'fnv1a64:8cfd3f4c579e0d41',
                },
            ],
            material: null,
        },
    ],
};
export const GENERATED_TUNNEL_CATALOG_EXAMPLE_BUNDLE = {
    kind: 'catalog_example_bundle.v0',
    exampleId: 'generated_tunnel.catalog_examples.v0',
    gameplayCatalog: GENERATED_TUNNEL_GAMEPLAY_PRESET_CATALOG,
    generatedAssetCatalog: GENERATED_TUNNEL_ASSET_CATALOG_EXAMPLE,
    authorityBoundary: CATALOG_EXAMPLE_AUTHORITY_BOUNDARY,
};
export function readGeneratedTunnelCatalogExampleReadout() {
    return readFpsGameplayPresetCatalog();
}
export function validateGeneratedTunnelCatalogExample() {
    return validateFpsGameplayPreset(GENERATED_TUNNEL_DEFAULT_FPS_PRESET);
}
export function buildInvalidGeneratedTunnelGameplayPresetExample() {
    const invalidPreset = {
        ...GENERATED_TUNNEL_DEFAULT_FPS_PRESET,
        playerController: {
            ...GENERATED_TUNNEL_DEFAULT_FPS_PRESET.playerController,
            moveSpeedUnitsPerSecond: -1,
            collisionHalfExtents: [0.25, 0, 0.25],
        },
        weapon: {
            ...GENERATED_TUNNEL_DEFAULT_FPS_PRESET.weapon,
            cooldownTicks: -1,
        },
        encounter: {
            ...GENERATED_TUNNEL_DEFAULT_FPS_PRESET.encounter,
            enemyCount: 0,
            spawnMarkerIds: ['exit_hint', 'exit_hint'],
        },
        payload: {
            rejectedBy: '@asha/catalog-core',
        },
    };
    return invalidPreset;
}
export function validateInvalidGeneratedTunnelGameplayPresetExample() {
    return validateFpsGameplayPreset(buildInvalidGeneratedTunnelGameplayPresetExample());
}
//# sourceMappingURL=examples.js.map