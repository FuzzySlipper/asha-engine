// Manifest operation registry (stands in for generated/conformance.json until the
// codegen emitter lands). The names here MUST match bridge-manifest.toml — parity
// is checked mechanically by harness/bridge/validate-manifest.py.
//
// Each entry maps the manifest's snake_case operation name to the camelCase facade
// method and records its surface. The conformance test asserts the facade exposes
// exactly these methods.

export type BridgeSurface = 'stable' | 'quarantined';

export interface BridgeOperation {
  /** snake_case name, identical to bridge-manifest.toml `[[operation]].name`. */
  readonly manifestName: string;
  /** camelCase method on the `RuntimeBridge` facade. */
  readonly facadeMethod: string;
  readonly surface: BridgeSurface;
}

export const MANIFEST_OPERATIONS: readonly BridgeOperation[] = [
  { manifestName: 'initialize_engine', facadeMethod: 'initializeEngine', surface: 'stable' },
  { manifestName: 'step_simulation', facadeMethod: 'stepSimulation', surface: 'stable' },
  { manifestName: 'submit_commands', facadeMethod: 'submitCommands', surface: 'stable' },
  { manifestName: 'read_render_diffs', facadeMethod: 'readRenderDiffs', surface: 'stable' },
  { manifestName: 'get_buffer', facadeMethod: 'getBuffer', surface: 'stable' },
  { manifestName: 'release_buffer', facadeMethod: 'releaseBuffer', surface: 'stable' },
  { manifestName: 'load_replay_fixture', facadeMethod: 'loadReplayFixture', surface: 'quarantined' },
  { manifestName: 'run_replay_step', facadeMethod: 'runReplayStep', surface: 'quarantined' },
];
