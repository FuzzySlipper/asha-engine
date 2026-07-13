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
const loaded = addon.loadFpsRuntimeSession(
  handle,
  'composed-native-provider',
  definitions,
  '[]',
);
assert.equal(loaded.health.find((entry) => entry.entity === 777).current, 150);
const secondHandle = addon.initializeEngine(42);
addon.loadProjectBundle(secondHandle, 1, 1, 2);
const secondLoaded = addon.loadFpsRuntimeSession(
  secondHandle,
  'composed-native-provider-second',
  definitions,
  '[]',
);
assert.equal(secondLoaded.health.find((entry) => entry.entity === 777).current, 150);
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

console.log(
  JSON.stringify({
    schemaVersion: 1,
    exportCount: expectedExports.length,
    targetHealthAfter: fired.targetHealthAfter.current,
    replayHash: fired.replayHash,
    workspaceTrace: fired.workspaceTrace,
  }),
);
