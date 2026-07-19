import type {
  CommandBatch,
  CommandResult,
  ProjectContentAuthoringRequest,
  ProjectContentAuthoringCommand,
  ProjectContentAuthoringResult,
  ProjectContentCodecResult,
  ProjectContentDecodeRequest,
  ProjectContentEncodeRequest,
  ProjectArtifactRelocation,
  ProjectStoreIdentity,
  ProjectWriteConfirmReceipt,
  ProjectWritePrepareReceipt,
  ProjectWritePublication,
  ProceduralEnvironmentApplyRequest,
  ProceduralEnvironmentApplyResult,
  ProceduralEnvironmentPreviewRequest,
  ProceduralEnvironmentPreviewResult,
  RenderFrameDiff,
  SceneDocumentCodecResult,
  SceneDocumentDecodeRequest,
  VoxelInstancePickRequest,
  VoxelInstancePickResult,
  VoxelProjectionBindingReceipt,
  VoxelProjectionInstanceBinding,
} from '@asha/contracts';
import type { RuntimeSessionFacade } from './facade.js';
import type { RuntimeSessionProjectIdentity } from './facade-core.js';
import type { RuntimeSessionProjectSource } from './facade-project.js';
import type {
  CompositionStatus,
  FrameCursor,
  ProjectBundleLoadRequest,
} from './transport-contracts.js';

export interface WorkspaceAuthoringOpenInput {
  readonly authoringId: string;
  readonly seed: number;
  readonly project: RuntimeSessionProjectIdentity;
  readonly projectBundle: ProjectBundleLoadRequest;
}

/** Ordinary project-source entrypoint for editor and content-pipeline hosts. */
export interface WorkspaceAuthoringProjectOpenInput {
  readonly authoringId: string;
  readonly seed: number;
  readonly workspaceId: string;
  readonly source: RuntimeSessionProjectSource;
}

export interface WorkspaceAuthoringProjectOpenReceipt {
  readonly state: WorkspaceAuthoringStateSummary;
  readonly manifestJson: string;
  readonly projectContent: ProjectContentCodecResult | null;
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

export interface WorkspaceAuthoringProjectionSummary {
  readonly kind: 'workspace_authoring.projection.v0';
  readonly workspaceId: string;
  readonly generation: number;
  readonly workingRevision: number;
  readonly cursor: FrameCursor;
  readonly nextCursor: FrameCursor;
  readonly delivery: 'replace' | 'apply';
  readonly frame: RenderFrameDiff;
  readonly renderDiffCount: number;
  readonly projectionHash: string;
}

export interface WorkspaceAuthoringProjectionRequest {
  readonly expectedWorkspaceId: string;
  readonly expectedGeneration: number;
  readonly expectedWorkingRevision: number;
  readonly cursor: FrameCursor;
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

export interface WorkspaceVoxelProjectionBindingInput {
  readonly registryDigest: string;
  readonly instances: readonly VoxelProjectionInstanceBinding[];
}

/**
 * Host observations and path choices for a Rust-owned whole-project save.
 * Workspace identity, generation, and revision are supplied by the facade.
 */
export interface WorkspaceProjectWritePrepareInput {
  readonly observedPrior: ProjectStoreIdentity;
  readonly priorManifestJson: string;
  readonly relocations?: readonly ProjectArtifactRelocation[];
}

export type WorkspaceVoxelInstancePickInput = Omit<
  VoxelInstancePickRequest,
  | 'workspaceId'
  | 'workspaceGeneration'
  | 'workingRevision'
  | 'registryDigest'
  | 'bindingHash'
>;

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
  openProject(input: WorkspaceAuthoringProjectOpenInput): Promise<WorkspaceAuthoringProjectOpenReceipt>;
  readState(): WorkspaceAuthoringStateSummary;
  readProjection(): WorkspaceAuthoringProjectionSummary;
  /** Decode and install one canonical scene into the Engine-owned workspace set. */
  decodeSceneDocument(input: SceneDocumentDecodeRequest): SceneDocumentCodecResult;
  configureVoxelProjectionInstances(
    input: WorkspaceVoxelProjectionBindingInput,
  ): VoxelProjectionBindingReceipt;
  pickVoxelInstance(input: WorkspaceVoxelInstancePickInput): VoxelInstancePickResult;
  submitCommands(batch: CommandBatch): CommandResult;
  decodeProjectContent(input: ProjectContentDecodeRequest): ProjectContentCodecResult;
  encodeProjectContent(input: ProjectContentEncodeRequest): ProjectContentCodecResult;
  applyProjectContentAuthoring(
    input: ProjectContentAuthoringRequest,
  ): ProjectContentAuthoringResult;
  /** Apply one typed command against the facade's current Rust-owned set/revision. */
  applyProjectContentCommand(command: ProjectContentAuthoringCommand): ProjectContentAuthoringResult;
  previewProceduralEnvironment(
    input: ProceduralEnvironmentPreviewRequest,
  ): ProceduralEnvironmentPreviewResult;
  applyProceduralEnvironment(
    input: ProceduralEnvironmentApplyRequest,
  ): ProceduralEnvironmentApplyResult;
  prepareProjectWrite(input: WorkspaceProjectWritePrepareInput): ProjectWritePrepareReceipt;
  confirmProjectWrite(publication: ProjectWritePublication): ProjectWriteConfirmReceipt;
  confirmStored(
    input: WorkspaceAuthoringStoredConfirmationInput,
  ): WorkspaceAuthoringStoredConfirmationReceipt;
  close(input: WorkspaceAuthoringCloseInput): WorkspaceAuthoringCloseReceipt;
}
