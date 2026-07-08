import { test } from 'node:test';
import assert from 'node:assert/strict';
import { GENERATED_TUNNEL_NAV_PROJECTION as bridgeNavProjection, TINY_GENERATED_TUNNEL_READOUT as bridgeTunnelReadout, buildCombatFeedbackProjection as buildBridgeCombatFeedbackProjection, } from './index.js';
import { GENERATED_TUNNEL_NAV_PROJECTION as sessionNavProjection, TINY_GENERATED_TUNNEL_READOUT as sessionTunnelReadout, buildCombatFeedbackProjection as buildSessionCombatFeedbackProjection, } from '@asha/runtime-session';
void test('runtime-bridge root keeps one compatibility re-export for runtime-session semantics', () => {
    assert.equal(bridgeTunnelReadout, sessionTunnelReadout);
    assert.equal(bridgeNavProjection, sessionNavProjection);
    assert.equal(buildBridgeCombatFeedbackProjection, buildSessionCombatFeedbackProjection);
});
//# sourceMappingURL=semantic-compat.test.js.map