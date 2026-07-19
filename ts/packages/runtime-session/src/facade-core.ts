import type {
  CommandBatch,
  CommandResult,
  RenderFrameDiff,
  RuntimeProjectionFrame,
} from '@asha/contracts';
import type { RuntimeActionIntentEnvelope } from './runtime-action.js';
import type {
  CompositionStatus,
  EngineHandle,
  FrameCursor,
  ProjectBundleLoadRequest,
  StepResult,
} from './transport-contracts.js';

export type RuntimeSessionMode = 'reference' | 'rust';

export interface RuntimeSessionProjectIdentity {
  readonly gameId: string;
  readonly workspaceId: string;
}

export interface RuntimeSessionInitializeInput {
  readonly sessionId: string;
  readonly seed: number;
  readonly project: RuntimeSessionProjectIdentity;
  /** Compatibility-only manual bootstrap descriptor. New project runtimes
   * initialize unloaded and call `loadProject({ source })`. */
  readonly projectBundle?: ProjectBundleLoadRequest;
}

export interface RuntimeSessionIdentity {
  readonly sessionId: string;
  readonly mode: RuntimeSessionMode;
  readonly seed: number;
  readonly project: RuntimeSessionProjectIdentity;
  readonly projectBundle: ProjectBundleLoadRequest | null;
  readonly nonClaims: readonly RuntimeSessionNonClaim[];
}

export type RuntimeSessionNonClaim =
  | 'not_native_runtime'
  | 'not_raw_state_store'
  | 'not_arbitrary_json_bridge'
  | 'not_product_authority'
  | 'not_gameplay_loop'
  | 'not_renderer';

export interface RuntimeSessionStateSummary {
  readonly identity: RuntimeSessionIdentity;
  readonly engine: EngineHandle;
  readonly composition: CompositionStatus;
  readonly sequenceId: number;
  readonly tick: number;
  readonly sessionHash: string;
}

export interface RuntimeSessionTickInput {
  readonly tick?: number;
}

export interface RuntimeSessionTickResult {
  readonly sequenceId: number;
  readonly tick: number;
  readonly step: StepResult;
  readonly composition: CompositionStatus;
  readonly sessionHash: string;
}

export interface RuntimeSessionCommandReceipt {
  readonly sequenceId: number;
  readonly batch: CommandBatch;
  readonly result: CommandResult;
  readonly acceptedCommandCount: number;
  readonly rejectedCommandCount: number;
  readonly sessionHashBefore: string;
  readonly sessionHashAfter: string;
}

export interface RuntimeSessionProjectionSummary {
  readonly sequenceId: number;
  readonly cursor: FrameCursor;
  readonly frame: RenderFrameDiff;
  readonly runtimeFrame: RuntimeProjectionFrame;
  readonly composition: CompositionStatus;
  readonly renderDiffCount: number;
  readonly presentationOpCount: number;
  readonly projectionHash: string;
}

export interface RuntimeSessionReplayRecord {
  readonly sequenceId: number;
  readonly kind:
    | 'initialize'
    | 'submitCommands'
    | 'tick'
    | 'createCamera'
    | 'applyCameraModeCommand'
    | 'applyCameraNavigationInput'
    | 'applyFirstPersonCameraInput'
    | 'applyCollisionConstrainedCameraInput'
    | 'loadEcrpProject'
    | 'loadProject'
    | 'closeProject'
    | 'submitRuntimeActionIntent'
    | 'submitGameExtensionWeaponEffect'
    | 'validateGameRuleCatalog'
    | 'submitGameRuleEffectIntent'
    | 'lifecycleDeath'
    | 'runAutonomousPolicyTick'
    | 'requestGeneratedTunnelOperation'
    | 'requestEncounterTransition'
    | 'requestSessionRestart'
    | 'restart';
  readonly actionSource?: RuntimeActionIntentEnvelope['source'];
  readonly recordHash: string;
}

export interface RuntimeSessionTelemetrySummary {
  readonly sequenceId: number;
  readonly tick: number;
  readonly composition: CompositionStatus;
  readonly acceptedCommandCount: number;
  readonly rejectedCommandCount: number;
  readonly restartCount: number;
  readonly sessionHash: string;
  readonly replayRecords: readonly RuntimeSessionReplayRecord[];
}

export type RuntimeSessionHashPrimitive = string | number | boolean | null;
export type RuntimeSessionHashValue =
  | RuntimeSessionHashPrimitive
  | readonly RuntimeSessionHashValue[]
  | RuntimeSessionHashRecord;

export interface RuntimeSessionHashRecord {
  readonly [key: string]: RuntimeSessionHashValue | undefined;
}
