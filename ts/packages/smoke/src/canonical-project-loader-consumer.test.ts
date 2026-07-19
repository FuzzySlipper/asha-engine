import assert from 'node:assert/strict';
import test from 'node:test';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { createAshaProjectDirectorySource } from '@asha/browser-host';
import { createMockRuntimeSession } from '@asha/runtime-bridge/reference';

const fixtureRoot = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '../../../../harness/fixtures/canonical-project-loader',
);

void test('independent consumer boots a development project without topology assembly', async () => {
  const runtimeSession = createMockRuntimeSession();
  runtimeSession.initialize({
    sessionId: 'runtime-session.independent-canonical-consumer',
    seed: 71,
    project: { gameId: 'canonical-loader-consumer', workspaceId: 'workspace.fixture' },
  });

  const source = await createAshaProjectDirectorySource(fixtureRoot);
  const receipt = await runtimeSession.loadProject({ source });

  assert.equal(receipt.accepted, true, JSON.stringify(receipt.diagnostics));
  assert.equal(receipt.source.kind, 'developmentDirectory');
  assert.equal(receipt.activeProject?.lifecycle.generation, 1);
});
