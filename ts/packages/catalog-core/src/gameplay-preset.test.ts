import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  GENERATED_TUNNEL_DEFAULT_FPS_PRESET,
  GENERATED_TUNNEL_FPS_ECRP_OBJECT_MODEL,
  findFpsEcrpObjectModelEntry,
  readDefaultFpsGameplayPreset,
  readFpsEcrpObjectModel,
  readFpsGameplayPresetCatalog,
  validateFpsGameplayPreset,
  type FpsGameplayPreset,
} from './index.js';

void test('default FPS gameplay preset validates and exposes stable readout references', () => {
  const report = validateFpsGameplayPreset(GENERATED_TUNNEL_DEFAULT_FPS_PRESET);

  assert.equal(report.kind, 'fps_gameplay_preset_validation.v0');
  assert.equal(report.valid, true);
  assert.deepEqual(report.diagnostics, []);
  assert.equal(report.rejectedHash, null);
  assert.ok(report.readout);
  assert.equal(report.readout.kind, 'fps_gameplay_preset_readout.v0');
  assert.equal(report.readout.preset.presetId, 'asha.generated_tunnel.default_fps.v0');
  assert.equal(report.readout.preset.playerController.moveSpeedUnitsPerSecond, 3);
  assert.equal(report.readout.preset.playerController.lookSensitivityDegreesPerPixel, 0.1);
  assert.equal(report.readout.preset.weapon.damage, 40);
  assert.equal(report.readout.preset.weapon.cooldownTicks, 4);
  assert.equal(report.readout.preset.enemyBehavior.policyRef, 'generated_tunnel_enemy_policy_loop.v0');
  assert.equal(report.readout.preset.encounter.presetId, 'generated-tunnel-small-encounter');
  assert.equal(report.readout.preset.generator.presetId, 'tiny-enclosed');
  assert.equal(report.readout.fixturePath, 'harness/fixtures/gameplay-presets/generated-tunnel-default-fps.snapshot.txt');
  assert.equal(report.readout.migration.playerControllerConstants, 'BrowserFpsInputCollector options');
  assert.ok(report.readout.nonClaims.includes('not_arbitrary_json_catalog'));
  assert.equal(report.readout.hashes.presetHash, 'fnv1a64:450137ad940ba1fb');
  assert.equal(report.readout.hashes.tuningHash, 'fnv1a64:09e0d75a647d6617');
  assert.equal(report.readout.hashes.referenceHash, 'fnv1a64:16fe3b71072981e3');
});

void test('FPS gameplay preset validator rejects invalid ranges and references deterministically', () => {
  const invalid = {
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
  } as never as FpsGameplayPreset;

  const report = validateFpsGameplayPreset(invalid);
  const diagnostics = report.diagnostics.map((diagnostic) => `${diagnostic.code}:${diagnostic.path}`);

  assert.equal(report.valid, false);
  assert.equal(report.readout, null);
  assert.equal(report.rejectedHash, 'fnv1a64:f58279b2225614a0');
  assert.ok(diagnostics.includes('invalidNumberRange:playerController.moveSpeedUnitsPerSecond'));
  assert.ok(diagnostics.includes('invalidNumberRange:playerController.collisionHalfExtents.1'));
  assert.ok(diagnostics.includes('invalidIntegerRange:weapon.cooldownTicks'));
  assert.ok(diagnostics.includes('invalidIntegerRange:encounter.enemyCount'));
  assert.ok(diagnostics.includes('duplicateReference:encounter.spawnMarkerIds.1'));
});

void test('FPS gameplay preset validator rejects arbitrary payload hatches and unexpected keys', () => {
  const withPayload = {
    ...GENERATED_TUNNEL_DEFAULT_FPS_PRESET,
    payload: { localConstants: true },
    weapon: {
      ...GENERATED_TUNNEL_DEFAULT_FPS_PRESET.weapon,
      payload: { raw: 'json' },
    },
  } as FpsGameplayPreset;

  const report = validateFpsGameplayPreset(withPayload);

  assert.equal(report.valid, false);
  assert.equal(report.readout, null);
  assert.deepEqual(
    report.diagnostics.map((diagnostic) => diagnostic.code),
    ['arbitraryPayloadRejected', 'arbitraryPayloadRejected'],
  );
  assert.deepEqual(
    report.diagnostics.map((diagnostic) => diagnostic.path),
    ['preset.payload', 'weapon.payload'],
  );
  assert.equal('payload' in (readDefaultFpsGameplayPreset() as object), false);
});

void test('FPS gameplay preset catalog readout lists consumer ownership and stable hashes', () => {
  const catalog = readFpsGameplayPresetCatalog();

  assert.equal(catalog.kind, 'fps_gameplay_preset_catalog_readout.v0');
  assert.equal(catalog.catalog.kind, 'fps_gameplay_preset_catalog.v0');
  assert.equal(catalog.catalog.defaultPresetId, 'asha.generated_tunnel.default_fps.v0');
  assert.equal(catalog.catalog.presets.length, 1);
  assert.equal(catalog.defaultPreset.hashes.presetHash, readDefaultFpsGameplayPreset().hashes.presetHash);
  assert.deepEqual(catalog.consumerOwnership.gameOwned, [
    'displayName',
    'playerController',
    'weapon',
    'enemyBehavior',
    'encounter',
    'generator',
  ]);
  assert.deepEqual(catalog.consumerOwnership.engineOwned, [
    'schemaValidation',
    'runtimeAuthority',
    'collisionResolution',
    'combatDamageApplication',
    'policyExecution',
    'proceduralGeneration',
  ]);
  assert.equal(catalog.hashes.catalogHash, 'fnv1a64:5a0603a299b79dc4');
  assert.equal(catalog.hashes.defaultPresetHash, 'fnv1a64:450137ad940ba1fb');
});

void test('FPS gameplay preset validation is descriptive and cannot authorize runtime behavior', () => {
  const report = validateFpsGameplayPreset(GENERATED_TUNNEL_DEFAULT_FPS_PRESET);
  assert.equal(report.valid, true);
  assert.ok(report.readout);

  const boundary = report.readout.authorityBoundary;
  assert.equal(boundary.catalogCoreRole, 'descriptive_config');
  assert.equal(boundary.shapeValidation.owner, '@asha/catalog-core');
  assert.equal(boundary.shapeValidation.scope, 'dto_shape_and_consumer_tuning_ranges_only');
  assert.equal(boundary.shapeValidation.authorizesRuntime, false);
  assert.equal(boundary.runtimeValidation.owner, 'rust_runtime_session_authority');
  assert.ok(boundary.runtimeValidation.surfaces.includes('RuntimeSessionFacade.loadEcrpProject'));
  assert.ok(boundary.runtimeValidation.surfaces.includes('RuntimeSessionFacade.submitRuntimeActionIntent'));
  assert.equal(boundary.semanticOwners.bootstrap, 'svc-entity-authoring');
  assert.equal(boundary.semanticOwners.combat, 'svc-combat');
  assert.equal(boundary.semanticOwners.collision, 'svc-collision');
  assert.ok(boundary.nonClaims.includes('not_runtime_acceptance_authority'));
  assert.ok(boundary.nonClaims.includes('not_combat_damage_authority'));

  const catalog = readFpsGameplayPresetCatalog();
  assert.equal(catalog.authorityBoundary.shapeValidation.authorizesRuntime, false);
  assert.deepEqual(catalog.authorityBoundary.runtimeValidation.ownerDocs, [
    'docs/entity-definition-schema.md',
    'docs/ecrp-capability-rule-ownership.md',
    'docs/runtime-session-facade.md',
  ]);
});

void test('FPS ECRP object model maps playable roles to public RuntimeSession surfaces', () => {
  const readout = readFpsEcrpObjectModel();
  const player = findFpsEcrpObjectModelEntry('player');
  const enemy = findFpsEcrpObjectModelEntry('enemy');

  assert.equal(readout.kind, 'fps_ecrp_object_model_readout.v0');
  assert.equal(readout.model, GENERATED_TUNNEL_FPS_ECRP_OBJECT_MODEL);
  assert.equal(readout.model.runtimeContract.ecrpReadoutKind, 'runtime_session.ecrp_readout.v0');
  assert.equal(readout.model.runtimeContract.projectBundleId, 'asha-demo');
  assert.equal(readout.model.entries.length, 2);
  assert.equal(player.entityDefinitionId, 'actor/demo-player');
  assert.equal(enemy.entityDefinitionId, 'actor/generated-tunnel-enemy');
  assert.deepEqual(player.capabilityKinds, [
    'transform',
    'collisionBody',
    'controller',
    'health',
    'weaponMount',
    'renderProjection',
    'faction',
  ]);
  assert.ok(enemy.capabilityKinds.includes('policyBinding'));
  assert.ok(enemy.capabilityKinds.includes('spawnMarker'));
  assert.ok(player.runtimeSurfaces.includes('RuntimeSessionFacade.applyCollisionConstrainedCameraInput'));
  assert.ok(enemy.runtimeSurfaces.includes('RuntimeSessionFacade.runAutonomousPolicyTick'));
  assert.ok(readout.migrationTargets.runtimeReadout === 'RuntimeSessionFacade.readEcrpRuntimeReadout');
  assert.ok(readout.nonClaims.includes('not_demo_local_entity_model'));
  assert.match(readout.hashes.modelHash, /^fnv1a64:[0-9a-f]{16}$/);
  assert.match(readout.hashes.playerEntryHash, /^fnv1a64:[0-9a-f]{16}$/);
  assert.match(readout.hashes.enemyEntryHash, /^fnv1a64:[0-9a-f]{16}$/);
  assert.match(readout.hashes.surfaceHash, /^fnv1a64:[0-9a-f]{16}$/);
});
