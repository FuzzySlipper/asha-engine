import assert from 'node:assert/strict';
import { test } from 'node:test';

import type {
  GeneratedTunnelRuntimeApplyReceipt,
  GeneratedTunnelRuntimeApplyRequest,
} from '@asha/contracts';

import { createRuntimeSessionFacade } from './index.js';
import { MockRuntimeBridge } from './mock.js';

class GeneratedTunnelBridge extends MockRuntimeBridge {
  override applyGeneratedTunnelToRuntimeWorld(
    request: GeneratedTunnelRuntimeApplyRequest,
  ): GeneratedTunnelRuntimeApplyReceipt {
    assert.deepEqual(request, { preset: 'tiny-enclosed', seed: 17 });
    return {
      preset: request.preset,
      seed: request.seed,
      grid: 0,
      configHash: 'e1d156c6b55137a7',
      outputHash: 'a9b504096397f5b4',
      collisionSourceHash: 'd32715988a716fb5',
      collisionProjectionHash: 'fnv1a64:08c55764b90ae303',
    };
  }
}

void test('Rust RuntimeSession exposes generated tunnel collision authority receipt', () => {
  const session = createRuntimeSessionFacade({
    bridge: new GeneratedTunnelBridge(),
    mode: 'rust',
  });
  session.initialize({
    sessionId: 'runtime-session.generated-tunnel.test',
    seed: 17,
    project: { gameId: 'asha-demo', workspaceId: 'workspace.local' },
    projectBundle: { bundleSchemaVersion: 1, protocolVersion: 1, sceneId: 4103 },
  });

  const receipt = session.requestGeneratedTunnelOperation({
    operation: 'apply_to_runtime_world',
    presetId: 'tiny-enclosed',
    seed: 17,
  });

  assert.equal(receipt.status, 'applied');
  if (receipt.status !== 'applied') {
    assert.fail('generated tunnel operation must apply');
  }
  assert.equal(receipt.grid, 0);
  assert.equal(receipt.outputHash, 'a9b504096397f5b4');
  assert.equal(receipt.collisionSourceHash, 'd32715988a716fb5');
  assert.equal(receipt.collisionProjectionHash, 'fnv1a64:08c55764b90ae303');
  assert.notEqual(receipt.sessionHashAfter, receipt.sessionHashBefore);
});
