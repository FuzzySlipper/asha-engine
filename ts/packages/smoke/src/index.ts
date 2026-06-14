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
  CapabilityStatus,
  RuntimeMode,
  SmokeMode,
  SmokeOutcome,
} from './result.js';
export {
  FIXTURE_WORLD,
  fixtureWorldHash,
  fixtureRenderFrame,
  fixtureCommandBatch,
  fixtureVoxelCommand,
  fixtureCommandEnvelopes,
} from './fixtures.js';
