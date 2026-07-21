---
status: current
audience: agent
tags: [perf, baseline, timing, ci]
supersedes: []
see-also: [launchable-voxel.md, guardrails.md]
---

# Performance Baseline

A deterministic, logged performance scenario over the canonical launch fixture, run on one stable host for trend/regression tracking. Not a product performance target and not part of the normal CI gate.

## Commands

```sh
cd ts
ASHA_PERF_HOST=<stable-host-label> pnpm --filter @asha/smoke dev:asha-perf
```

See `topics/ci/perf-baseline.md` for field stability, output paths, and known limitations.
That document also defines the bounded native `readVoxelUpdateTelemetry` workflow for
correlating command, dirty-chunk, remesh, render-op, and mesh-evidence structure without
turning host timing into authority or a default CI threshold.
