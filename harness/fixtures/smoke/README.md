# Smoke golden

`reference-smoke.txt` is the committed, deterministic text report of the **reference**
(mock) smoke run — the canonical 10-stage launchable proof from `@asha/smoke`
(`boot → load → render → pick → preview → command-submit → authority-classify →
render-update → save-reload-replay → cleanup`).

It is drift-checked by `ts/packages/smoke/src/smoke.test.ts`
(`reference smoke matches the committed golden snapshot`), run under
`harness/ci/check-ts.sh`. The report includes renderer/resource counters (leaked/peak
handles, scene/debug nodes, fallbacks, outstanding buffers) so a bounded, leak-free
lifecycle is part of the committed evidence.

## Regenerate

When the reference smoke output changes intentionally, re-render it from the fixed
reference boot (`mock` mode, `nativeAvailable=false`) and review the diff:

```bash
cd ts && pnpm --filter @asha/smoke build
node -e "
const { runSmoke } = require('./packages/smoke/dist/harness.js');
const { formatResult } = require('./packages/smoke/dist/result.js');
const { createMockRuntimeBridge } = require('./packages/runtime-bridge/dist/index.js');
const boot = () => ({ bridge: createMockRuntimeBridge(), mode:'mock', intent:'reference', nativeAvailable:false });
process.stdout.write(formatResult(runSmoke({ bootBridge: boot })));
" > ../harness/fixtures/smoke/reference-smoke.txt
```

The live run artifacts (`harness/smoke-out/asha-smoke.{txt,json}`) are gitignored; only
this pinned golden is committed. See `docs/launchable-voxel.md` for the full loop.
