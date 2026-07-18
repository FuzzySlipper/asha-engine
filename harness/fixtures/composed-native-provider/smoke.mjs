import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const [addonPath, repoRoot] = process.argv.slice(2);
if (!addonPath || !repoRoot) {
  throw new Error('usage: node smoke.mjs <addon-path> <repo-root>');
}
const generatedSurface = readFileSync(
  `${repoRoot}/ts/packages/native-bridge/src/generated/addon-surface.ts`,
  'utf8',
);
const namesBlock = generatedSurface.match(
  /const GENERATED_NATIVE_ADDON_EXPORT_NAMES = \[([\s\S]*?)\] as const;/,
);
assert.ok(namesBlock, 'generated native export declaration is readable');
const expectedExports = [...namesBlock[1].matchAll(/'([^']+)'/g)].map((match) => match[1]).sort();

const require = createRequire(import.meta.url);
const addon = require(addonPath);
assert.deepEqual(Object.keys(addon).sort(), expectedExports);

const authoredProject = JSON.parse(readFileSync(
  `${repoRoot}/harness/fixtures/gameplay-module-sdk/downstream-module/project/gameplay-project.json`,
  'utf8',
));
const authoredConfiguration = authoredProject.gameplayModuleBindings.configurations[0];
const authoredGameplayDocument = (multiplier) => ({
  schemaVersion: 1,
  configurations: [{
    configurationId: authoredConfiguration.configurationId,
    module: authoredConfiguration.module,
    schemaId: `${authoredConfiguration.configuration.namespace}.${authoredConfiguration.configuration.name}.v${authoredConfiguration.configuration.version}`,
    values: [{ fieldId: 'multiplier', value: { kind: 'integer', value: multiplier } }],
  }],
  bindings: authoredProject.gameplayModuleBindings.bindings,
  overrides: [],
  triggers: [],
});
const authoringHandle = addon.openWorkspaceAuthoring(-1, JSON.stringify({
  authoringId: 'composed-provider-authoring-smoke',
  seed: 41,
  project: { gameId: 'fixture.pulse', workspaceId: 'fixture.pulse.authoring' },
  projectBundle: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 90 },
}));
const decodeGameplayDocument = (multiplier) => JSON.parse(addon.decodeProjectContent(
  authoringHandle,
  JSON.stringify({
    sources: [{
      documentId: 'gameplay/fixture-pulse.json',
      kind: 'gameplayConfiguration',
      sourceText: JSON.stringify(authoredGameplayDocument(multiplier)),
    }],
  }),
));
const malformedAuthoredGameplay = decodeGameplayDocument(-1);
assert.equal(malformedAuthoredGameplay.accepted, false);
assert.ok(malformedAuthoredGameplay.diagnostics.some(
  (diagnostic) => diagnostic.message.includes('typed provider codec rejected configuration'),
));
const acceptedAuthoredGameplay = decodeGameplayDocument(4);
assert.equal(acceptedAuthoredGameplay.accepted, true);
assert.ok(acceptedAuthoredGameplay.fieldMetadata.some(
  (field) => field.path === 'configurationValues.multiplier',
));

const handle = addon.initializeEngine(41);
addon.loadProjectBundle(handle, 1, 1, 1);
const definitions = [
  {
    entity: 101,
    stableId: 'actor/composed-player',
    displayName: 'Composed Player',
    sourcePath: 'catalogs/player.entity.json',
    tags: ['player'],
    role: 'player',
    transform: {
      translation: { x: 2.5, y: 1.5, z: 1.5 },
      rotation: [0, 0, 0, 1],
      scale: { x: 1, y: 1, z: 1 },
    },
    bounds: {
      min: { x: 2.2, y: 1, z: 1 },
      max: { x: 2.8, y: 2, z: 2 },
    },
    renderVisible: true,
    staticCollider: false,
    health: { current: 88, max: 88 },
    weapon: {
      weaponId: 'weapon.composed.primary',
      damage: 75,
      rangeUnits: 16,
      ammo: 3,
      cooldownTicksAfterFire: 4,
    },
  },
  {
    entity: 777,
    stableId: 'actor/composed-enemy',
    displayName: 'Composed Enemy',
    sourcePath: 'catalogs/enemy.entity.json',
    tags: ['enemy'],
    role: 'enemy',
    transform: {
      translation: { x: 2.5, y: 1.5, z: 5.2 },
      rotation: [0, 0, 0, 1],
      scale: { x: 1, y: 1, z: 1 },
    },
    bounds: {
      min: { x: 2.2, y: 1, z: 5 },
      max: { x: 2.8, y: 2, z: 5.8 },
    },
    renderVisible: true,
    staticCollider: false,
    health: { current: 150, max: 150 },
    policyBinding: {
      bindingId: 'binding.enemy',
      policyId: 'policy.enemy',
      viewKind: 'runtime_session.nav_policy_view.v0',
      viewVersion: 'v0',
      allowedIntents: ['runtime.intent.move_direct_nav.v0'],
      runtimeMoment: 'runtime.tick.enemy_policy.v0',
    },
  },
];
const fpsSceneDocument = {
  schemaVersion: 3,
  id: 77,
  metadata: { name: 'Composed provider scene', authoringFormatVersion: 3 },
  dependencies: [],
  nodes: definitions.map((definition, childOrder) => ({
    id: definition.entity,
    parent: null,
    childOrder,
    label: definition.displayName,
    tags: [],
    transform: {
      translation: [
        definition.transform.translation.x,
        definition.transform.translation.y,
        definition.transform.translation.z,
      ],
      rotation: definition.transform.rotation,
      scale: [
        definition.transform.scale.x,
        definition.transform.scale.y,
        definition.transform.scale.z,
      ],
    },
    kind: {
      kind: 'entityInstance',
      instance: {
        instanceId: `${definition.stableId}.instance`,
        reference: { kind: 'entityDefinition', stableId: definition.stableId },
        spawnMarkerId: null,
      },
    },
  })),
};
const fpsBootstrapRegistry = {
  schemaVersion: 1,
  entityDefinitionIds: definitions.map((definition) => definition.stableId),
  prefabIds: [],
  generatorPresets: [],
  catalogIds: [],
};
const loaded = addon.loadFpsRuntimeSession(
  handle,
  'composed-native-provider',
  JSON.stringify(fpsSceneDocument),
  JSON.stringify(fpsBootstrapRegistry),
  definitions,
  '[]',
);
assert.equal(loaded.health.find((entry) => entry.entity === 777).current, 150);
const secondHandle = addon.initializeEngine(42);
addon.loadProjectBundle(secondHandle, 1, 1, 2);
const secondLoaded = addon.loadFpsRuntimeSession(
  secondHandle,
  'composed-native-provider-second',
  JSON.stringify(fpsSceneDocument),
  JSON.stringify(fpsBootstrapRegistry),
  definitions,
  '[]',
);
assert.equal(secondLoaded.health.find((entry) => entry.entity === 777).current, 150);
const composedBefore = addon.readComposedRuntimeSession(handle);
const secondComposedBefore = addon.readComposedRuntimeSession(secondHandle);
const pulseViewContract = {
  namespace: 'fixture.pulse',
  name: 'pulse-state-view',
  version: 1,
  schemaHash: 'fnv1a64:67048ec3babae8be',
};
const pulseBefore = addon.readGameplayModuleView(
  handle,
  pulseViewContract.namespace,
  pulseViewContract.name,
  pulseViewContract.version,
  pulseViewContract.schemaHash,
  'session',
  undefined,
  composedBefore.runtimeSessionHash,
);
assert.equal(JSON.parse(Buffer.from(pulseBefore.canonicalPayload).toString('utf8')), 4);
assert.equal(pulseBefore.runtimeSessionHash, composedBefore.runtimeSessionHash);
const interaction = addon.applyGameplayPrefabPartInteraction(
  handle,
  101,
  700,
  'interaction/target',
  4102412266368810,
  12,
  composedBefore.runtimeSessionHash,
);
assert.equal(interaction.target, 4102412266368810);
assert.notEqual(interaction.runtimeSessionHash, composedBefore.runtimeSessionHash);
const pulseAfter = addon.readGameplayModuleView(
  handle,
  pulseViewContract.namespace,
  pulseViewContract.name,
  pulseViewContract.version,
  pulseViewContract.schemaHash,
  'session',
  undefined,
  interaction.runtimeSessionHash,
);
assert.equal(JSON.parse(Buffer.from(pulseAfter.canonicalPayload).toString('utf8')), 5);
assert.equal(pulseAfter.revision, pulseBefore.revision + 1);
assert.deepEqual(addon.readComposedRuntimeSession(secondHandle), secondComposedBefore);
assert.throws(
  () => addon.applyGameplayPrefabPartInteraction(
    handle,
    101,
    700,
    'interaction/target',
    4102412266368811,
    13,
    interaction.runtimeSessionHash,
  ),
  /target mismatch/,
);
assert.equal(addon.readComposedRuntimeSession(handle).runtimeSessionHash, interaction.runtimeSessionHash);
assert.throws(
  () => addon.applyGameplayPrefabPartInteraction(
    handle,
    101,
    700,
    'interaction/target',
    4102412266368810,
    12,
    composedBefore.runtimeSessionHash,
  ),
  /expected RuntimeSession/,
);
assert.equal(addon.readComposedRuntimeSession(handle).runtimeSessionHash, interaction.runtimeSessionHash);
const fired = addon.applyFpsPrimaryFire(
  handle,
  9,
  { x: 2.5, y: 1.5, z: 1.5 },
  { x: 0, y: 0, z: 1 },
  undefined,
  undefined,
);
assert.equal(fired.targetHealthAfter.current, 0);
assert.equal(
  addon.readFpsRuntimeSession(secondHandle).health.find((entry) => entry.entity === 777).current,
  150,
);
assert.deepEqual(fired.workspaceTrace, [
  'constructed typed primary-fire Workspace from authoritative combat preview',
  'ran Guard -> Transform -> React inside the composed gameplay Fabric',
  'revalidated the final Workspace and committed through rule-lifecycle + svc-combat',
]);
addon.unloadProjectBundle(handle);
assert.throws(() => addon.readComposedRuntimeSession(handle), /not initialized|not built/i);
assert.throws(
  () => addon.readGameplayModuleView(
    handle,
    pulseViewContract.namespace,
    pulseViewContract.name,
    pulseViewContract.version,
    pulseViewContract.schemaHash,
    'session',
    undefined,
    interaction.runtimeSessionHash,
  ),
  /not initialized|not built/i,
);

console.log(
  JSON.stringify({
    schemaVersion: 1,
    exportCount: expectedExports.length,
    targetHealthAfter: fired.targetHealthAfter.current,
    moduleViewRevision: pulseAfter.revision,
    prefabInteractionHash: interaction.eventHash,
    replayHash: fired.replayHash,
    workspaceTrace: fired.workspaceTrace,
  }),
);
