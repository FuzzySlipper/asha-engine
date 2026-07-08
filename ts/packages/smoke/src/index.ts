// @asha/smoke — public surface for the developer smoke harness.

export {
  runSmoke,
  defaultBootBridge,
  authorityBootBridge,
  bootForMode,
  SMOKE_COMMAND,
  AUTHORITY_SMOKE_COMMAND,
} from './harness.js';
export type { SmokeOptions, BridgeBoot } from './harness.js';
export { formatResult } from './result.js';
export type {
  SmokeResult,
  SmokeStage,
  SmokeFailure,
  SmokeFailureCategory,
  SmokeCounters,
  CapabilityStatus,
  RuntimeMode,
  SmokeMode,
  SmokeOutcome,
} from './result.js';
export {
  FIXTURE_PROJECT_BUNDLE,
  fixtureProjectBundleHash,
  fixtureRenderFrame,
  fixtureEditUpdateFrame,
  fixturePickRay,
  fixtureCommandBatch,
  fixtureVoxelCommand,
  fixtureCommandEnvelopes,
} from './fixtures.js';
export { runPerf, formatPerf, PERF_COMMAND, DEFAULT_EDIT_CYCLES } from './perf.js';
export type {
  PerfResult,
  PerfMetadata,
  PerfTiming,
  PerfCounters,
  PerfInvariant,
  PerfOptions,
  PerfClock,
} from './perf.js';
export { runGpuPerf, formatGpuPerf, GPU_PERF_COMMAND } from './gpu-perf.js';
export type {
  ExternalCalibration,
  GpuDescriptor,
  GpuPerfGating,
  GpuPerfMetadata,
  GpuPerfOptions,
  GpuPerfRenderContext,
  GpuPerfResult,
  GpuPerfSkip,
  GpuPerfSkipReason,
  GpuPerfStatus,
} from './gpu-perf.js';
