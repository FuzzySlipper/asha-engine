import type { Catalog } from '@asha/contracts';
import {
  GENERATED_TUNNEL_DEFAULT_FPS_PRESET,
  GENERATED_TUNNEL_GAMEPLAY_PRESET_CATALOG,
  readFpsGameplayPresetCatalog,
  validateFpsGameplayPreset,
  type FpsGameplayPreset,
  type FpsGameplayPresetCatalogReadout,
  type FpsGameplayPresetValidationReport,
} from '@asha/catalog-core';

export type CatalogExampleAuthorityBoundary = {
  readonly packageRole: '@asha/catalog-examples';
  readonly owns: readonly ['fixture_data', 'invalid_fixture_builders', 'consumer_examples'];
  readonly doesNotOwn: readonly [
    'runtime_authority',
    'state_mutation',
    'command_validation',
    'collision_resolution',
    'combat_damage_application',
  ];
};

export type GeneratedTunnelCatalogExampleBundle = {
  readonly kind: 'catalog_example_bundle.v0';
  readonly exampleId: 'generated_tunnel.catalog_examples.v0';
  readonly gameplayCatalog: typeof GENERATED_TUNNEL_GAMEPLAY_PRESET_CATALOG;
  readonly generatedAssetCatalog: Catalog;
  readonly authorityBoundary: CatalogExampleAuthorityBoundary;
};

export const CATALOG_EXAMPLE_AUTHORITY_BOUNDARY: CatalogExampleAuthorityBoundary = {
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

export const GENERATED_TUNNEL_ASSET_CATALOG_EXAMPLE: Catalog = {
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

export const GENERATED_TUNNEL_CATALOG_EXAMPLE_BUNDLE: GeneratedTunnelCatalogExampleBundle = {
  kind: 'catalog_example_bundle.v0',
  exampleId: 'generated_tunnel.catalog_examples.v0',
  gameplayCatalog: GENERATED_TUNNEL_GAMEPLAY_PRESET_CATALOG,
  generatedAssetCatalog: GENERATED_TUNNEL_ASSET_CATALOG_EXAMPLE,
  authorityBoundary: CATALOG_EXAMPLE_AUTHORITY_BOUNDARY,
};

export function readGeneratedTunnelCatalogExampleReadout(): FpsGameplayPresetCatalogReadout {
  return readFpsGameplayPresetCatalog();
}

export function validateGeneratedTunnelCatalogExample(): FpsGameplayPresetValidationReport {
  return validateFpsGameplayPreset(GENERATED_TUNNEL_DEFAULT_FPS_PRESET);
}

export function buildInvalidGeneratedTunnelGameplayPresetExample(): FpsGameplayPreset {
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

  return invalidPreset as unknown as FpsGameplayPreset;
}

export function validateInvalidGeneratedTunnelGameplayPresetExample(): FpsGameplayPresetValidationReport {
  return validateFpsGameplayPreset(buildInvalidGeneratedTunnelGameplayPresetExample());
}
