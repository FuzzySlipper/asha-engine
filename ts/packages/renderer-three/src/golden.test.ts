// Render golden check: apply a named render-diff fixture to the Three.js
// renderer and diff its deterministic scene snapshot against a committed golden.
// Run headlessly (no GL context). Driven by `harness/ci/check-render-goldens.sh`.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { ThreeRenderer } from './index.js';

const repoRoot = resolve(import.meta.dirname, '../../../..');

function loadFixture(name: string): unknown {
  return JSON.parse(
    readFileSync(resolve(repoRoot, 'harness/fixtures/render-diffs', `${name}.json`), 'utf8'),
  );
}

function loadGolden(name: string): string {
  return readFileSync(resolve(repoRoot, 'harness/goldens/render-diffs', `${name}.snapshot`), 'utf8');
}

function checkGolden(name: string): void {
  const renderer = new ThreeRenderer();
  try {
    renderer.applyEncodedFrame(loadFixture(name));
  } catch (e) {
    assert.fail(`RENDERER FAILURE while applying ${name}: ${String(e)}`);
  }

  const actual = renderer.snapshot();
  const golden = loadGolden(name);
  assert.equal(
    actual,
    golden,
    `GOLDEN MISMATCH: rendered scene drifted from ` +
      `harness/goldens/render-diffs/${name}.snapshot — regenerate if intended.`,
  );
}

test('scene-showcase fixture renders to the committed golden snapshot', () => {
  checkGolden('scene-showcase');
});

test('static-mesh-instances fixture renders to the committed golden snapshot', () => {
  // Two instances share one defined asset geometry; per-instance overrides and
  // transforms differ. Drift means the static-mesh asset/instance path changed.
  checkGolden('static-mesh-instances');
});

test('sprite-showcase fixture renders to the committed golden snapshot', () => {
  // Plane-geometry sprites with billboard/pivot/size/depth + a deterministic
  // projection-driven frame update (handle 1 advances to frame 3).
  checkGolden('sprite-showcase');
});
