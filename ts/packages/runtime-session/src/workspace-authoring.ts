import type {
  CommandBatch,
  CommandResult,
} from '@asha/contracts';
import type { RuntimeSessionFacade } from './facade.js';
import type { RuntimeSessionProjectIdentity } from './facade-core.js';
import type {
  CompositionStatus,
  ProjectBundleLoadRequest,
} from './transport-contracts.js';

export interface WorkspaceAuthoringOpenInput {
  readonly authoringId: string;
  readonly seed: number;
  readonly project: RuntimeSessionProjectIdentity;
  readonly projectBundle: ProjectBundleLoadRequest;
}

export interface WorkspaceAuthoringIdentity {
  readonly kind: 'workspace_authoring.identity.v0';
  readonly authoringId: string;
  readonly mode: 'rust';
  readonly generation: number;
  readonly seed: number;
  readonly project: RuntimeSessionProjectIdentity;
  readonly projectBundle: ProjectBundleLoadRequest;
  readonly nonClaims: readonly [
    'not_gameplay_runtime_session',
    'not_simulation_loop',
    'not_stored_truth',
    'not_renderer_authority',
  ];
}

export interface WorkspaceAuthoringStateSummary {
  readonly kind: 'workspace_authoring.state.v0';
  readonly status: 'open' | 'closed';
  readonly identity: WorkspaceAuthoringIdentity;
  readonly composition: CompositionStatus;
  readonly workingRevision: number;
  readonly storedRevision: number;
  readonly dirty: boolean;
  readonly lastStoredCanonicalJsonHash: string | null;
  readonly authoritySnapshotHash: string;
  readonly lifecycleHash: string;
}

export interface WorkspaceAuthoringStoredConfirmationInput {
  readonly expectedWorkspaceId: string;
  readonly expectedGeneration: number;
  readonly hostPath: string;
  readonly canonicalJsonHash: string;
}

export interface WorkspaceAuthoringStoredConfirmationReceipt {
  readonly kind: 'workspace_authoring.stored_confirmation.v0';
  readonly accepted: true;
  readonly workspaceId: string;
  readonly generation: number;
  readonly hostPath: string;
  readonly canonicalJsonHash: string;
  readonly storedRevision: number;
  readonly lifecycleHash: string;
}

export interface WorkspaceAuthoringCloseInput {
  readonly expectedWorkspaceId: string;
  readonly expectedGeneration: number;
  readonly discardUnsavedWorkingState?: boolean;
}

export interface WorkspaceAuthoringCloseReceipt {
  readonly kind: 'workspace_authoring.close_receipt.v0';
  readonly closed: true;
  readonly workspaceId: string;
  readonly generation: number;
  readonly discardedUnsavedWorkingState: boolean;
  readonly lifecycleHash: string;
}

type WorkspaceAuthoringVoxelOperations = Pick<
  RuntimeSessionFacade,
  | 'registerVoxelConversionSource'
  | 'registerVoxelConversionMeshAsset'
  | 'importVoxelConversionMeshSource'
  | 'readVoxelConversionSourceMetadata'
  | 'planVoxelConversion'
  | 'previewVoxelConversion'
  | 'applyVoxelConversion'
  | 'exportVoxelConversionEvidence'
  | 'readVoxelModelInfo'
  | 'readVoxelModelWindow'
  | 'exportVoxelVolumeAsset'
  | 'saveVoxelVolumeAsset'
  | 'updateVoxelVolumeAssetPalette'
  | 'initializeVoxelVolumeAuthoring'
  | 'loadVoxelVolumeAsset'
  | 'validateVoxelAnnotationLayer'
  | 'loadVoxelAnnotationLayer'
  | 'readVoxelAnnotationQuery'
  | 'applyVoxelAnnotationEdit'
  | 'exportVoxelAnnotationLayer'
  | 'readVoxelEditHistory'
  | 'previewVoxelEditRevert'
  | 'applyVoxelEditRevert'
  | 'undoVoxelEdit'
  | 'redoVoxelEdit'
  | 'readDeveloperConsole'
>;

/**
 * Rust-backed authority for durable workspace asset authoring.
 *
 * This deliberately omits gameplay, ticking, camera, and live-runtime lifecycle
 * operations. A stored asset crosses into a gameplay RuntimeSession only through
 * a separate consumer-owned load transaction.
 */
export interface WorkspaceAuthoringFacade extends WorkspaceAuthoringVoxelOperations {
  open(input: WorkspaceAuthoringOpenInput): WorkspaceAuthoringStateSummary;
  readState(): WorkspaceAuthoringStateSummary;
  submitCommands(batch: CommandBatch): CommandResult;
  confirmStored(
    input: WorkspaceAuthoringStoredConfirmationInput,
  ): WorkspaceAuthoringStoredConfirmationReceipt;
  close(input: WorkspaceAuthoringCloseInput): WorkspaceAuthoringCloseReceipt;
}
