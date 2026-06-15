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
  FIXTURE_WORLD,
  fixtureWorldHash,
  fixtureRenderFrame,
  fixtureEditUpdateFrame,
  fixturePickRay,
  fixtureCommandBatch,
  fixtureVoxelCommand,
  fixtureCommandEnvelopes,
} from './fixtures.js';
