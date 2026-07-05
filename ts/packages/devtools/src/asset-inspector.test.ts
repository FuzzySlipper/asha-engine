import { test } from 'node:test';
import assert from 'node:assert/strict';

import type {
  AssetReference,
  Catalog,
  CatalogEntry,
  CatalogValidationReport,
  FallbackDecision,
  LockValidationReport,
  MaterialProjection,
} from '@asha/contracts';

import {
  buildCatalogModel,
  buildLockDriftModel,
  classifyFallback,
  impactOfChangedAsset,
  inspectMaterial,
} from './asset-inspector.js';

function ref(id: string): AssetReference {
  return { id, version: { req: 'any' }, hash: null };
}

function entry(over: Partial<CatalogEntry> & { id: string; kind: CatalogEntry['kind'] }): CatalogEntry {
  return {
    id: over.id,
    kind: over.kind,
    version: over.version ?? 1,
    hash: over.hash ?? null,
    sourcePath: over.sourcePath ?? `src/${over.id}`,
    label: over.label ?? null,
    dependencies: over.dependencies ?? [],
    material: over.material ?? null,
  };
}

const material: MaterialProjection = {
  render: { color: { r: 1, g: 0, b: 0, a: 1 }, texture: null, roughness: 0.5, emissive: 0, uvStrategy: 'flat' },
  collision: { solid: true, collidable: true, occludes: true, structuralClass: 'solid' },
};

// A valid catalog: a mesh depending on a material, plus a standalone texture.
function validCatalog(): Catalog {
  return {
    entries: [
      entry({ id: 'mesh:wall', kind: 'mesh', dependencies: [ref('material:stone')] }),
      entry({ id: 'material:stone', kind: 'material', material, dependencies: [ref('texture:stone')] }),
      entry({ id: 'texture:stone', kind: 'texture' }),
    ],
  };
}

void test('buildCatalogModel resolves entries and the dependency DAG of a valid catalog', () => {
  const model = buildCatalogModel(validCatalog());
  assert.deepEqual(
    model.entries.map((e) => e.id),
    ['mesh:wall', 'material:stone', 'texture:stone'],
  );
  assert.deepEqual(model.dependencyEdges.get('mesh:wall'), ['material:stone']);
  assert.deepEqual(model.dependencyEdges.get('material:stone'), ['texture:stone']);
  assert.equal(model.cycles.length, 0);
  assert.equal(model.structuralIssues.length, 0);
  // The material entry is flagged as carrying a projection.
  assert.equal(model.entries.find((e) => e.id === 'material:stone')?.hasMaterial, true);
});

void test('buildCatalogModel records a missing dependency without dropping it', () => {
  const catalog: Catalog = {
    entries: [entry({ id: 'mesh:wall', kind: 'mesh', dependencies: [ref('material:absent')] })],
  };
  const model = buildCatalogModel(catalog);
  const wall = model.entries[0]!;
  assert.deepEqual(wall.dependencies, ['material:absent']);
  assert.deepEqual(wall.missingDependencies, ['material:absent']);
  // Absent deps are not edges in the present-asset DAG.
  assert.deepEqual(model.dependencyEdges.get('mesh:wall'), []);
});

void test('buildCatalogModel detects a dependency cycle over present assets', () => {
  const catalog: Catalog = {
    entries: [
      entry({ id: 'a', kind: 'mesh', dependencies: [ref('b')] }),
      entry({ id: 'b', kind: 'mesh', dependencies: [ref('a')] }),
    ],
  };
  const model = buildCatalogModel(catalog);
  assert.equal(model.cycles.length, 1);
  assert.deepEqual([...model.cycles[0]!].sort(), ['a', 'b']);
});

void test('buildCatalogModel surfaces classified structural issues from a generated report', () => {
  const report: CatalogValidationReport = {
    errors: [
      {
        code: 'wrong-kind-reference',
        id: null,
        kind: null,
        from: 'mesh:wall',
        slot: '0',
        expected: 'material',
        actual: 'texture',
        reference: null,
        dependency: null,
        cyclePath: [],
      },
    ],
  };
  const model = buildCatalogModel(validCatalog(), report);
  assert.equal(model.structuralIssues.length, 1);
  assert.equal(model.structuralIssues[0]!.code, 'wrong-kind-reference');
  assert.match(model.structuralIssues[0]!.detail, /expected material, found texture/);
});

void test('buildLockDriftModel classifies drift and never silently relocks', () => {
  const report: LockValidationReport = {
    findings: [
      {
        id: 'texture:stone',
        code: 'stale-hash',
        lockedKind: 'texture',
        currentKind: 'texture',
        lockedVersion: 1,
        currentVersion: 1,
        lockedHash: 'aaa',
        currentHash: 'bbb',
        addedDependencies: [],
        removedDependencies: [],
      },
      {
        id: 'mesh:new',
        code: 'new-in-catalog',
        lockedKind: null,
        currentKind: 'mesh',
        lockedVersion: null,
        currentVersion: 1,
        lockedHash: null,
        currentHash: null,
        addedDependencies: [],
        removedDependencies: [],
      },
    ],
  };
  const model = buildLockDriftModel(report);
  assert.equal(model.hasDrift, true);
  assert.equal(model.findings.find((f) => f.id === 'texture:stone')?.severity, 'drift');
  assert.equal(model.findings.find((f) => f.id === 'mesh:new')?.severity, 'info');
});

void test('a lock report with only new-in-catalog findings reports no drift', () => {
  const report: LockValidationReport = {
    findings: [
      {
        id: 'mesh:new',
        code: 'new-in-catalog',
        lockedKind: null,
        currentKind: 'mesh',
        lockedVersion: null,
        currentVersion: 1,
        lockedHash: null,
        currentHash: null,
        addedDependencies: [],
        removedDependencies: [],
      },
    ],
  };
  assert.equal(buildLockDriftModel(report).hasDrift, false);
});

void test('inspectMaterial exposes render and collision as disjoint read views', () => {
  const view = inspectMaterial(entry({ id: 'material:stone', kind: 'material', material }))!;
  // The two projections are separate objects with no overlapping keys — they can
  // never be presented or edited as one mixed material.
  const renderKeys = Object.keys(view.render);
  const collisionKeys = Object.keys(view.collision);
  assert.equal(renderKeys.some((k) => collisionKeys.includes(k)), false);
  assert.equal(view.render.uvStrategy, 'flat');
  assert.equal(view.collision.structuralClass, 'solid');
  // No texture/color leaks into the collision view, no solid/collidable into render.
  assert.equal('texture' in view.collision, false);
  assert.equal('solid' in view.render, false);
});

void test('inspectMaterial returns null for a non-material asset', () => {
  assert.equal(inspectMaterial(entry({ id: 'mesh:wall', kind: 'mesh' })), null);
});

void test('classifyFallback marks only the useFallback outcome as fallback-used', () => {
  const used: FallbackDecision = { outcome: 'useFallback', reason: 'missing', visual: 'magentaSquare' };
  const failed: FallbackDecision = { outcome: 'failClosed', reason: 'collision-critical' };
  assert.deepEqual(classifyFallback(used), {
    outcome: 'useFallback',
    reason: 'missing',
    visual: 'magentaSquare',
    fallbackUsed: true,
  });
  assert.equal(classifyFallback(failed).fallbackUsed, false);
  assert.equal(classifyFallback(failed).visual, null);
});

void test('impactOfChangedAsset reports transitive dependents', () => {
  // texture:stone ← material:stone ← mesh:wall
  const report = impactOfChangedAsset(validCatalog(), 'texture:stone');
  assert.deepEqual(report.dependents, ['material:stone', 'mesh:wall']);
  assert.equal(report.unknownAsset, false);
});

void test('impactOfChangedAsset flags an unknown asset and an asset with no dependents', () => {
  const unknown = impactOfChangedAsset(validCatalog(), 'mesh:absent');
  assert.equal(unknown.unknownAsset, true);
  assert.deepEqual(unknown.dependents, []);

  const leaf = impactOfChangedAsset(validCatalog(), 'mesh:wall');
  assert.deepEqual(leaf.dependents, []);
});

import {
  buildAssetSourceTrace,
  formatAssetSourceTrace,
  type AssetSourceTraceInput,
} from './asset-inspector.js';

function trace(over: Partial<AssetSourceTraceInput>): AssetSourceTraceInput {
  return {
    guid: '28426a627e8870ba9fdefd6a0d998bfc',
    source: 'assets/crate.mesh.json',
    catalogId: 'mesh/crate',
    artifacts: [{ path: 'crate.staticmesh.json', hash: '8899aabbccddeeff' }],
    status: 'unchanged',
    ...over,
  };
}

void test('source trace surfaces a tracked GUID and clean status', () => {
  const v = buildAssetSourceTrace(trace({}));
  assert.equal(v.tracked, true);
  assert.equal(v.needsReimport, false);
  assert.equal(v.needsInit, false);
  assert.equal(v.artifactCount, 1);
});

void test('content change under a stable GUID flags a reimport, not re-init', () => {
  const v = buildAssetSourceTrace(trace({ status: 'contentChanged' }));
  assert.equal(v.tracked, true);
  assert.equal(v.needsReimport, true);
  assert.equal(v.needsInit, false);
});

void test('a missing sidecar / absent GUID flags init', () => {
  const missing = buildAssetSourceTrace(trace({ status: 'missingSidecar' }));
  assert.equal(missing.needsInit, true);
  const guidless = buildAssetSourceTrace(trace({ guid: null }));
  assert.equal(guidless.tracked, false);
  assert.equal(guidless.needsInit, true);
});

void test('formatAssetSourceTrace is deterministic and greppable', () => {
  const lines = formatAssetSourceTrace(buildAssetSourceTrace(trace({ status: 'movedFile' })));
  assert.ok(lines[0]!.includes('guid=28426a627e8870ba9fdefd6a0d998bfc'));
  assert.ok(lines[0]!.includes('status=movedFile'));
  assert.ok(lines[2]!.includes('needsReimport=false'));
});
