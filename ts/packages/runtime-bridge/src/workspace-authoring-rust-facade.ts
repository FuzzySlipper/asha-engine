import type {
  WorkspaceAuthoringCloseInput,
  WorkspaceAuthoringCloseReceipt,
  WorkspaceAuthoringFacade,
  WorkspaceAuthoringIdentity,
  WorkspaceAuthoringOpenInput,
  WorkspaceAuthoringProjectionSummary,
  WorkspaceAuthoringStateSummary,
  WorkspaceAuthoringStoredConfirmationInput,
  WorkspaceAuthoringStoredConfirmationReceipt,
  WorkspaceVoxelInstancePickInput,
  WorkspaceVoxelProjectionBindingInput,
} from '@asha/runtime-session';
import type { VoxelProjectionBindingReceipt } from '@asha/contracts';
import type {
  ProjectContentAuthoringRequest,
  ProjectContentAuthoringResult,
  ProjectContentCodecResult,
  ProjectContentDecodeRequest,
  ProjectContentEncodeRequest,
  ProceduralEnvironmentApplyRequest,
  ProceduralEnvironmentApplyResult,
  ProceduralEnvironmentPreviewRequest,
  ProceduralEnvironmentPreviewResult,
  SceneDocumentCodecResult,
  SceneDocumentDecodeRequest,
} from '@asha/contracts';
import {
  RuntimeBridgeError,
  frameCursor,
  type RuntimeBridge,
} from './bridge.js';

function validateRequiredIdentity(value: string, field: string): string {
  const normalized = value.trim();
  if (normalized.length === 0) {
    throw new RuntimeBridgeError('invalid_input', `${field} must be non-empty`);
  }
  return normalized;
}

function workspaceAuthoringStateFromContract(
  value: ReturnType<RuntimeBridge['readWorkspaceAuthoringState']>,
): WorkspaceAuthoringStateSummary {
  const expectedNonClaims = [
    'not_gameplay_runtime_session',
    'not_simulation_loop',
    'not_stored_truth',
    'not_renderer_authority',
  ];
  if (
    value.kind !== 'workspace_authoring.state.v0'
    || (value.status !== 'open' && value.status !== 'closed')
    || value.identity.kind !== 'workspace_authoring.identity.v0'
    || value.identity.mode !== 'rust'
    || value.identity.nonClaims.length !== expectedNonClaims.length
    || value.identity.nonClaims.some((entry, index) => entry !== expectedNonClaims[index])
  ) {
    throw new RuntimeBridgeError('internal', 'Rust returned an invalid workspace-authoring state');
  }
  return value as WorkspaceAuthoringStateSummary;
}

function storedConfirmationFromContract(
  value: ReturnType<RuntimeBridge['confirmWorkspaceAuthoringStored']>,
): WorkspaceAuthoringStoredConfirmationReceipt {
  if (value.kind !== 'workspace_authoring.stored_confirmation.v0' || value.accepted !== true) {
    throw new RuntimeBridgeError('internal', 'Rust returned an invalid stored confirmation');
  }
  return value as WorkspaceAuthoringStoredConfirmationReceipt;
}

function closeReceiptFromContract(
  value: ReturnType<RuntimeBridge['closeWorkspaceAuthoring']>,
): WorkspaceAuthoringCloseReceipt {
  if (value.kind !== 'workspace_authoring.close_receipt.v0' || value.closed !== true) {
    throw new RuntimeBridgeError('internal', 'Rust returned an invalid workspace close receipt');
  }
  return value as WorkspaceAuthoringCloseReceipt;
}

export class RustBackedWorkspaceAuthoringFacade implements WorkspaceAuthoringFacade {
  readonly #bridge: RuntimeBridge;
  #state: WorkspaceAuthoringStateSummary | null = null;
  #nextProjectionCursor = frameCursor(0);
  #voxelProjectionBinding: VoxelProjectionBindingReceipt | null = null;

  constructor(bridge: RuntimeBridge) {
    this.#bridge = bridge;
  }

  open(input: WorkspaceAuthoringOpenInput): WorkspaceAuthoringStateSummary {
    const state = workspaceAuthoringStateFromContract(
      this.#bridge.openWorkspaceAuthoring(input),
    );
    this.#state = state;
    this.#nextProjectionCursor = frameCursor(0);
    this.#voxelProjectionBinding = null;
    return state;
  }

  readState(): WorkspaceAuthoringStateSummary {
    const state = workspaceAuthoringStateFromContract(
      this.#bridge.readWorkspaceAuthoringState(),
    );
    this.#state = state;
    return state;
  }

  readProjection(): WorkspaceAuthoringProjectionSummary {
    const state = this.#requireOpenState('readProjection');
    const projection = this.#bridge.readWorkspaceAuthoringProjection({
      expectedWorkspaceId: state.identity.project.workspaceId,
      expectedGeneration: state.identity.generation,
      expectedWorkingRevision: state.workingRevision,
      cursor: this.#nextProjectionCursor,
    });
    this.#nextProjectionCursor = projection.nextCursor;
    return projection;
  }

  decodeSceneDocument(input: SceneDocumentDecodeRequest): SceneDocumentCodecResult {
    this.#requireOpen('decodeSceneDocument');
    return this.#bridge.decodeSceneDocument(input);
  }

  decodeProjectContent(input: ProjectContentDecodeRequest): ProjectContentCodecResult {
    this.#requireOpen('decodeProjectContent');
    return this.#bridge.decodeProjectContent(input);
  }

  encodeProjectContent(input: ProjectContentEncodeRequest): ProjectContentCodecResult {
    this.#requireOpen('encodeProjectContent');
    return this.#bridge.encodeProjectContent(input);
  }

  applyProjectContentAuthoring(
    input: ProjectContentAuthoringRequest,
  ): ProjectContentAuthoringResult {
    this.#requireOpen('applyProjectContentAuthoring');
    const result = this.#bridge.applyProjectContentAuthoring(input);
    if (result.accepted) {
      this.#refreshAfterMutation();
    }
    return result;
  }

  previewProceduralEnvironment(
    input: ProceduralEnvironmentPreviewRequest,
  ): ProceduralEnvironmentPreviewResult {
    this.#requireOpen('previewProceduralEnvironment');
    return this.#bridge.previewProceduralEnvironment(input);
  }

  applyProceduralEnvironment(
    input: ProceduralEnvironmentApplyRequest,
  ): ProceduralEnvironmentApplyResult {
    this.#requireOpen('applyProceduralEnvironment');
    const result = this.#bridge.applyProceduralEnvironment(input);
    if (result.accepted) {
      this.#refreshAfterMutation();
    }
    return result;
  }

  configureVoxelProjectionInstances(
    input: WorkspaceVoxelProjectionBindingInput,
  ): VoxelProjectionBindingReceipt {
    const state = this.#requireOpenState('configureVoxelProjectionInstances');
    const identity = state.identity;
    const registryDigest = validateRequiredIdentity(input.registryDigest, 'registryDigest');
    const receipt = this.#bridge.configureVoxelProjectionInstances({
      workspaceId: identity.project.workspaceId,
      workspaceGeneration: identity.generation,
      workingRevision: state.workingRevision,
      registryDigest,
      instances: [...input.instances],
    });
    this.#voxelProjectionBinding = receipt;
    return receipt;
  }

  pickVoxelInstance(input: WorkspaceVoxelInstancePickInput): ReturnType<WorkspaceAuthoringFacade['pickVoxelInstance']> {
    const state = this.#requireOpenState('pickVoxelInstance');
    const identity = state.identity;
    const binding = this.#voxelProjectionBinding;
    if (binding === null || binding.workingRevision !== state.workingRevision) {
      throw new RuntimeBridgeError(
        'stale_authority_snapshot',
        'voxel instance picking requires projection bindings for the current working revision',
      );
    }
    return this.#bridge.pickVoxelInstance({
      ...input,
      workspaceId: identity.project.workspaceId,
      workspaceGeneration: identity.generation,
      workingRevision: state.workingRevision,
      registryDigest: binding.registryDigest,
      bindingHash: binding.bindingHash,
    });
  }

  submitCommands(...args: Parameters<WorkspaceAuthoringFacade['submitCommands']>): ReturnType<WorkspaceAuthoringFacade['submitCommands']> {
    this.#requireOpen('submitCommands');
    const result = this.#bridge.submitCommands(...args);
    if (result.accepted > 0) this.#refreshAfterMutation();
    return result;
  }

  registerVoxelConversionSource(...args: Parameters<WorkspaceAuthoringFacade['registerVoxelConversionSource']>): ReturnType<WorkspaceAuthoringFacade['registerVoxelConversionSource']> {
    this.#requireOpen('registerVoxelConversionSource');
    return this.#bridge.registerVoxelConversionSource(...args);
  }

  registerVoxelConversionMeshAsset(...args: Parameters<WorkspaceAuthoringFacade['registerVoxelConversionMeshAsset']>): ReturnType<WorkspaceAuthoringFacade['registerVoxelConversionMeshAsset']> {
    this.#requireOpen('registerVoxelConversionMeshAsset');
    return this.#bridge.registerVoxelConversionMeshAsset(...args);
  }

  importVoxelConversionMeshSource(...args: Parameters<WorkspaceAuthoringFacade['importVoxelConversionMeshSource']>): ReturnType<WorkspaceAuthoringFacade['importVoxelConversionMeshSource']> {
    this.#requireOpen('importVoxelConversionMeshSource');
    return this.#bridge.importVoxelConversionMeshSource(...args);
  }

  readVoxelConversionSourceMetadata(...args: Parameters<WorkspaceAuthoringFacade['readVoxelConversionSourceMetadata']>): ReturnType<WorkspaceAuthoringFacade['readVoxelConversionSourceMetadata']> {
    this.#requireOpen('readVoxelConversionSourceMetadata');
    return this.#bridge.readVoxelConversionSourceMetadata(...args);
  }

  planVoxelConversion(...args: Parameters<WorkspaceAuthoringFacade['planVoxelConversion']>): ReturnType<WorkspaceAuthoringFacade['planVoxelConversion']> {
    this.#requireOpen('planVoxelConversion');
    return this.#bridge.planVoxelConversion(...args);
  }

  previewVoxelConversion(...args: Parameters<WorkspaceAuthoringFacade['previewVoxelConversion']>): ReturnType<WorkspaceAuthoringFacade['previewVoxelConversion']> {
    this.#requireOpen('previewVoxelConversion');
    return this.#bridge.previewVoxelConversion(...args);
  }

  applyVoxelConversion(...args: Parameters<WorkspaceAuthoringFacade['applyVoxelConversion']>): ReturnType<WorkspaceAuthoringFacade['applyVoxelConversion']> {
    this.#requireOpen('applyVoxelConversion');
    const receipt = this.#bridge.applyVoxelConversion(...args);
    if (receipt.applied) this.#refreshAfterMutation();
    return receipt;
  }

  exportVoxelConversionEvidence(...args: Parameters<WorkspaceAuthoringFacade['exportVoxelConversionEvidence']>): ReturnType<WorkspaceAuthoringFacade['exportVoxelConversionEvidence']> {
    this.#requireOpen('exportVoxelConversionEvidence');
    return this.#bridge.exportVoxelConversionEvidence(...args);
  }

  readVoxelModelInfo(...args: Parameters<WorkspaceAuthoringFacade['readVoxelModelInfo']>): ReturnType<WorkspaceAuthoringFacade['readVoxelModelInfo']> {
    this.#requireOpen('readVoxelModelInfo');
    return this.#bridge.readVoxelModelInfo(...args);
  }

  readVoxelModelWindow(...args: Parameters<WorkspaceAuthoringFacade['readVoxelModelWindow']>): ReturnType<WorkspaceAuthoringFacade['readVoxelModelWindow']> {
    this.#requireOpen('readVoxelModelWindow');
    return this.#bridge.readVoxelModelWindow(...args);
  }

  exportVoxelVolumeAsset(...args: Parameters<WorkspaceAuthoringFacade['exportVoxelVolumeAsset']>): ReturnType<WorkspaceAuthoringFacade['exportVoxelVolumeAsset']> {
    this.#requireOpen('exportVoxelVolumeAsset');
    return this.#bridge.exportVoxelVolumeAsset(...args);
  }

  saveVoxelVolumeAsset(...args: Parameters<WorkspaceAuthoringFacade['saveVoxelVolumeAsset']>): ReturnType<WorkspaceAuthoringFacade['saveVoxelVolumeAsset']> {
    this.#requireOpen('saveVoxelVolumeAsset');
    return this.#bridge.saveVoxelVolumeAsset(...args);
  }

  updateVoxelVolumeAssetPalette(...args: Parameters<WorkspaceAuthoringFacade['updateVoxelVolumeAssetPalette']>): ReturnType<WorkspaceAuthoringFacade['updateVoxelVolumeAssetPalette']> {
    this.#requireOpen('updateVoxelVolumeAssetPalette');
    const receipt = this.#bridge.updateVoxelVolumeAssetPalette(...args);
    if (receipt.updated) this.#refreshAfterMutation();
    return receipt;
  }

  initializeVoxelVolumeAuthoring(...args: Parameters<WorkspaceAuthoringFacade['initializeVoxelVolumeAuthoring']>): ReturnType<WorkspaceAuthoringFacade['initializeVoxelVolumeAuthoring']> {
    this.#requireOpen('initializeVoxelVolumeAuthoring');
    const receipt = this.#bridge.initializeVoxelVolumeAuthoring(...args);
    if (receipt.initialized) this.#refreshAfterMutation();
    return receipt;
  }

  loadVoxelVolumeAsset(...args: Parameters<WorkspaceAuthoringFacade['loadVoxelVolumeAsset']>): ReturnType<WorkspaceAuthoringFacade['loadVoxelVolumeAsset']> {
    this.#requireOpen('loadVoxelVolumeAsset');
    const receipt = this.#bridge.loadVoxelVolumeAsset(...args);
    if (receipt.loaded) {
      this.#refreshAfterMutation();
    }
    return receipt;
  }

  validateVoxelAnnotationLayer(...args: Parameters<WorkspaceAuthoringFacade['validateVoxelAnnotationLayer']>): ReturnType<WorkspaceAuthoringFacade['validateVoxelAnnotationLayer']> {
    this.#requireOpen('validateVoxelAnnotationLayer');
    return this.#bridge.validateVoxelAnnotationLayer(...args);
  }

  loadVoxelAnnotationLayer(...args: Parameters<WorkspaceAuthoringFacade['loadVoxelAnnotationLayer']>): ReturnType<WorkspaceAuthoringFacade['loadVoxelAnnotationLayer']> {
    this.#requireOpen('loadVoxelAnnotationLayer');
    const receipt = this.#bridge.loadVoxelAnnotationLayer(...args);
    if (receipt.loaded) this.#refreshAfterMutation();
    return receipt;
  }

  readVoxelAnnotationQuery(...args: Parameters<WorkspaceAuthoringFacade['readVoxelAnnotationQuery']>): ReturnType<WorkspaceAuthoringFacade['readVoxelAnnotationQuery']> {
    this.#requireOpen('readVoxelAnnotationQuery');
    return this.#bridge.readVoxelAnnotationQuery(...args);
  }

  applyVoxelAnnotationEdit(...args: Parameters<WorkspaceAuthoringFacade['applyVoxelAnnotationEdit']>): ReturnType<WorkspaceAuthoringFacade['applyVoxelAnnotationEdit']> {
    this.#requireOpen('applyVoxelAnnotationEdit');
    const receipt = this.#bridge.applyVoxelAnnotationEdit(...args);
    if (receipt.edited) this.#refreshAfterMutation();
    return receipt;
  }

  exportVoxelAnnotationLayer(...args: Parameters<WorkspaceAuthoringFacade['exportVoxelAnnotationLayer']>): ReturnType<WorkspaceAuthoringFacade['exportVoxelAnnotationLayer']> {
    this.#requireOpen('exportVoxelAnnotationLayer');
    return this.#bridge.exportVoxelAnnotationLayer(...args);
  }

  readVoxelEditHistory(...args: Parameters<WorkspaceAuthoringFacade['readVoxelEditHistory']>): ReturnType<WorkspaceAuthoringFacade['readVoxelEditHistory']> {
    this.#requireOpen('readVoxelEditHistory');
    return this.#bridge.readVoxelEditHistory(...args);
  }

  previewVoxelEditRevert(...args: Parameters<WorkspaceAuthoringFacade['previewVoxelEditRevert']>): ReturnType<WorkspaceAuthoringFacade['previewVoxelEditRevert']> {
    this.#requireOpen('previewVoxelEditRevert');
    return this.#bridge.previewVoxelEditRevert(...args);
  }

  applyVoxelEditRevert(...args: Parameters<WorkspaceAuthoringFacade['applyVoxelEditRevert']>): ReturnType<WorkspaceAuthoringFacade['applyVoxelEditRevert']> {
    this.#requireOpen('applyVoxelEditRevert');
    const receipt = this.#bridge.applyVoxelEditRevert(...args);
    if (receipt.applied) this.#refreshAfterMutation();
    return receipt;
  }

  undoVoxelEdit(...args: Parameters<WorkspaceAuthoringFacade['undoVoxelEdit']>): ReturnType<WorkspaceAuthoringFacade['undoVoxelEdit']> {
    this.#requireOpen('undoVoxelEdit');
    const receipt = this.#bridge.undoVoxelEdit(...args);
    if (receipt.receipt.applied) this.#refreshAfterMutation();
    return receipt;
  }

  redoVoxelEdit(...args: Parameters<WorkspaceAuthoringFacade['redoVoxelEdit']>): ReturnType<WorkspaceAuthoringFacade['redoVoxelEdit']> {
    this.#requireOpen('redoVoxelEdit');
    const receipt = this.#bridge.redoVoxelEdit(...args);
    if (receipt.receipt.applied) this.#refreshAfterMutation();
    return receipt;
  }

  readDeveloperConsole(...args: Parameters<WorkspaceAuthoringFacade['readDeveloperConsole']>): ReturnType<WorkspaceAuthoringFacade['readDeveloperConsole']> {
    this.#requireOpen('readDeveloperConsole');
    return this.#bridge.readDeveloperConsole(...args);
  }

  confirmStored(
    input: WorkspaceAuthoringStoredConfirmationInput,
  ): WorkspaceAuthoringStoredConfirmationReceipt {
    this.#requireOpen('confirmStored');
    const receipt = storedConfirmationFromContract(
      this.#bridge.confirmWorkspaceAuthoringStored(input),
    );
    this.readState();
    return receipt;
  }

  close(input: WorkspaceAuthoringCloseInput): WorkspaceAuthoringCloseReceipt {
    this.#requireOpen('close');
    const receipt = closeReceiptFromContract(this.#bridge.closeWorkspaceAuthoring({
      ...input,
      discardUnsavedWorkingState: input.discardUnsavedWorkingState ?? false,
    }));
    this.#state = workspaceAuthoringStateFromContract(
      this.#bridge.readWorkspaceAuthoringState(),
    );
    this.#nextProjectionCursor = frameCursor(0);
    this.#voxelProjectionBinding = null;
    return receipt;
  }

  #refreshAfterMutation(): void {
    this.readState();
    this.#voxelProjectionBinding = null;
  }

  #requireOpen(operation: string): WorkspaceAuthoringIdentity {
    return this.#requireOpenState(operation).identity;
  }

  #requireOpenState(operation: string): WorkspaceAuthoringStateSummary {
    const state = this.#state === null ? null : this.readState();
    if (state === null || state.status !== 'open') {
      throw new RuntimeBridgeError(
        'not_initialized',
        `${operation} requires an open workspace authoring authority`,
      );
    }
    return state;
  }
}

export interface WorkspaceAuthoringFacadeOptions {
  readonly bridge: RuntimeBridge;
}

export function createWorkspaceAuthoringFacade(
  options: WorkspaceAuthoringFacadeOptions,
): WorkspaceAuthoringFacade {
  return new RustBackedWorkspaceAuthoringFacade(options.bridge);
}
