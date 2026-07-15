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
} from '@asha/runtime-session';
import {
  RuntimeBridgeError,
  frameCursor,
  nonNegativeSafeInteger,
  type RuntimeBridge,
} from './bridge.js';

function validateRequiredIdentity(value: string, field: string): string {
  const normalized = value.trim();
  if (normalized.length === 0) {
    throw new RuntimeBridgeError('invalid_input', `${field} must be non-empty`);
  }
  return normalized;
}

function fnv1a64(value: string): string {
  let hash = 0xcbf29ce484222325n;
  for (const byte of new TextEncoder().encode(value)) {
    hash ^= BigInt(byte);
    hash = BigInt.asUintN(64, hash * 0x100000001b3n);
  }
  return `fnv1a64:${hash.toString(16).padStart(16, '0')}`;
}

export class RustBackedWorkspaceAuthoringFacade implements WorkspaceAuthoringFacade {
  readonly #bridge: RuntimeBridge;
  #identity: WorkspaceAuthoringIdentity | null = null;
  #status: 'open' | 'closed' = 'closed';
  #composition: WorkspaceAuthoringStateSummary['composition'] = {
    loadedProjectBundle: null,
    fatalCount: 0,
    totalCount: 0,
    blocksLoad: false,
  };
  #generation = 0;
  #workingRevision = 0;
  #storedRevision = 0;
  #lastStoredCanonicalJsonHash: string | null = null;
  #pendingStoredCandidate: {
    readonly canonicalJsonHash: string;
    readonly workingRevision: number;
  } | null = null;
  #projectionGenerationInitialized = false;
  #authoritySnapshotHash = 'fnv1a64:not-open';

  constructor(bridge: RuntimeBridge) {
    this.#bridge = bridge;
  }

  open(input: WorkspaceAuthoringOpenInput): WorkspaceAuthoringStateSummary {
    if (this.#status === 'open') {
      throw new RuntimeBridgeError(
        'invalid_input',
        'workspace authoring is already open; close it before opening another workspace',
      );
    }
    const authoringId = validateRequiredIdentity(input.authoringId, 'authoringId');
    const gameId = validateRequiredIdentity(input.project.gameId, 'project.gameId');
    const workspaceId = validateRequiredIdentity(input.project.workspaceId, 'project.workspaceId');
    nonNegativeSafeInteger(input.seed, 'seed');
    nonNegativeSafeInteger(input.projectBundle.bundleSchemaVersion, 'projectBundle.bundleSchemaVersion');
    nonNegativeSafeInteger(input.projectBundle.protocolVersion, 'projectBundle.protocolVersion');
    nonNegativeSafeInteger(input.projectBundle.sceneId, 'projectBundle.sceneId');

    this.#bridge.initializeEngine({ seed: input.seed });
    const composition = this.#bridge.loadProjectBundle(input.projectBundle);
    if (composition.blocksLoad || composition.loadedProjectBundle !== input.projectBundle.sceneId) {
      try {
        this.#bridge.unloadProjectBundle();
      } catch {
        // Preserve the original classified open failure.
      }
      throw new RuntimeBridgeError(
        'invalid_input',
        'workspace authoring ProjectBundle did not become the active authority input',
      );
    }

    this.#generation += 1;
    this.#identity = {
      kind: 'workspace_authoring.identity.v0',
      authoringId,
      mode: 'rust',
      generation: this.#generation,
      seed: input.seed,
      project: { gameId, workspaceId },
      projectBundle: input.projectBundle,
      nonClaims: [
        'not_gameplay_runtime_session',
        'not_simulation_loop',
        'not_stored_truth',
        'not_renderer_authority',
      ],
    };
    this.#status = 'open';
    this.#composition = composition;
    this.#workingRevision = 0;
    this.#storedRevision = 0;
    this.#lastStoredCanonicalJsonHash = null;
    this.#pendingStoredCandidate = null;
    this.#projectionGenerationInitialized = false;
    this.#authoritySnapshotHash = this.#bridge.readDeveloperConsole().snapshotHash;
    return this.readState();
  }

  readState(): WorkspaceAuthoringStateSummary {
    const identity = this.#requireIdentity('readState', false);
    if (this.#status === 'open') {
      this.#authoritySnapshotHash = this.#bridge.readDeveloperConsole().snapshotHash;
    }
    const stateWithoutHash = {
      kind: 'workspace_authoring.state.v0' as const,
      status: this.#status,
      identity,
      composition: this.#composition,
      workingRevision: this.#workingRevision,
      storedRevision: this.#storedRevision,
      dirty: this.#workingRevision !== this.#storedRevision,
      lastStoredCanonicalJsonHash: this.#lastStoredCanonicalJsonHash,
      authoritySnapshotHash: this.#authoritySnapshotHash,
    };
    return {
      ...stateWithoutHash,
      lifecycleHash: fnv1a64(JSON.stringify(stateWithoutHash)),
    };
  }

  readProjection(): WorkspaceAuthoringProjectionSummary {
    const identity = this.#requireOpen('readProjection');
    const cursor = frameCursor(this.#workingRevision);
    const frame = this.#bridge.readRenderDiffs(cursor);
    const delivery: WorkspaceAuthoringProjectionSummary['delivery'] =
      this.#projectionGenerationInitialized ? 'apply' : 'replace';
    this.#projectionGenerationInitialized = true;
    const summaryWithoutHash = {
      kind: 'workspace_authoring.projection.v0' as const,
      workspaceId: identity.project.workspaceId,
      generation: identity.generation,
      workingRevision: this.#workingRevision,
      cursor,
      delivery,
      frame,
      renderDiffCount: frame.ops.length,
    };
    return {
      ...summaryWithoutHash,
      projectionHash: fnv1a64(JSON.stringify(summaryWithoutHash)),
    };
  }

  submitCommands(...args: Parameters<WorkspaceAuthoringFacade['submitCommands']>): ReturnType<WorkspaceAuthoringFacade['submitCommands']> {
    this.#requireOpen('submitCommands');
    const result = this.#bridge.submitCommands(...args);
    if (result.accepted > 0) this.#recordWorkingMutation();
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
    if (receipt.applied) this.#recordWorkingMutation();
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
    const receipt = this.#bridge.saveVoxelVolumeAsset(...args);
    if (receipt.saved && receipt.canonicalJsonHash !== null) {
      this.#pendingStoredCandidate = {
        canonicalJsonHash: receipt.canonicalJsonHash,
        workingRevision: this.#workingRevision,
      };
    }
    return receipt;
  }

  updateVoxelVolumeAssetPalette(...args: Parameters<WorkspaceAuthoringFacade['updateVoxelVolumeAssetPalette']>): ReturnType<WorkspaceAuthoringFacade['updateVoxelVolumeAssetPalette']> {
    this.#requireOpen('updateVoxelVolumeAssetPalette');
    const receipt = this.#bridge.updateVoxelVolumeAssetPalette(...args);
    if (receipt.updated) this.#recordWorkingMutation();
    return receipt;
  }

  initializeVoxelVolumeAuthoring(...args: Parameters<WorkspaceAuthoringFacade['initializeVoxelVolumeAuthoring']>): ReturnType<WorkspaceAuthoringFacade['initializeVoxelVolumeAuthoring']> {
    this.#requireOpen('initializeVoxelVolumeAuthoring');
    const receipt = this.#bridge.initializeVoxelVolumeAuthoring(...args);
    if (receipt.initialized) this.#recordWorkingMutation();
    return receipt;
  }

  loadVoxelVolumeAsset(...args: Parameters<WorkspaceAuthoringFacade['loadVoxelVolumeAsset']>): ReturnType<WorkspaceAuthoringFacade['loadVoxelVolumeAsset']> {
    this.#requireOpen('loadVoxelVolumeAsset');
    const receipt = this.#bridge.loadVoxelVolumeAsset(...args);
    if (receipt.loaded) {
      this.#recordWorkingMutation();
      this.#storedRevision = this.#workingRevision;
      this.#lastStoredCanonicalJsonHash = receipt.canonicalJsonHash;
      this.#pendingStoredCandidate = null;
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
    if (receipt.loaded) this.#recordWorkingMutation();
    return receipt;
  }

  readVoxelAnnotationQuery(...args: Parameters<WorkspaceAuthoringFacade['readVoxelAnnotationQuery']>): ReturnType<WorkspaceAuthoringFacade['readVoxelAnnotationQuery']> {
    this.#requireOpen('readVoxelAnnotationQuery');
    return this.#bridge.readVoxelAnnotationQuery(...args);
  }

  applyVoxelAnnotationEdit(...args: Parameters<WorkspaceAuthoringFacade['applyVoxelAnnotationEdit']>): ReturnType<WorkspaceAuthoringFacade['applyVoxelAnnotationEdit']> {
    this.#requireOpen('applyVoxelAnnotationEdit');
    const receipt = this.#bridge.applyVoxelAnnotationEdit(...args);
    if (receipt.edited) this.#recordWorkingMutation();
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
    if (receipt.applied) this.#recordWorkingMutation();
    return receipt;
  }

  undoVoxelEdit(...args: Parameters<WorkspaceAuthoringFacade['undoVoxelEdit']>): ReturnType<WorkspaceAuthoringFacade['undoVoxelEdit']> {
    this.#requireOpen('undoVoxelEdit');
    const receipt = this.#bridge.undoVoxelEdit(...args);
    if (receipt.receipt.applied) this.#recordWorkingMutation();
    return receipt;
  }

  redoVoxelEdit(...args: Parameters<WorkspaceAuthoringFacade['redoVoxelEdit']>): ReturnType<WorkspaceAuthoringFacade['redoVoxelEdit']> {
    this.#requireOpen('redoVoxelEdit');
    const receipt = this.#bridge.redoVoxelEdit(...args);
    if (receipt.receipt.applied) this.#recordWorkingMutation();
    return receipt;
  }

  readDeveloperConsole(...args: Parameters<WorkspaceAuthoringFacade['readDeveloperConsole']>): ReturnType<WorkspaceAuthoringFacade['readDeveloperConsole']> {
    this.#requireOpen('readDeveloperConsole');
    return this.#bridge.readDeveloperConsole(...args);
  }

  confirmStored(
    input: WorkspaceAuthoringStoredConfirmationInput,
  ): WorkspaceAuthoringStoredConfirmationReceipt {
    const identity = this.#requireOpen('confirmStored');
    this.#validateBoundIdentity(input.expectedWorkspaceId, input.expectedGeneration);
    const hostPath = validateRequiredIdentity(input.hostPath, 'hostPath');
    const canonicalJsonHash = validateRequiredIdentity(
      input.canonicalJsonHash,
      'canonicalJsonHash',
    );
    if (
      this.#pendingStoredCandidate === null
      || this.#pendingStoredCandidate.workingRevision !== this.#workingRevision
      || this.#pendingStoredCandidate.canonicalJsonHash !== canonicalJsonHash
    ) {
      throw new RuntimeBridgeError(
        'invalid_input',
        'storage confirmation must match the current Rust save candidate and working revision',
      );
    }
    this.#storedRevision = this.#workingRevision;
    this.#lastStoredCanonicalJsonHash = canonicalJsonHash;
    this.#pendingStoredCandidate = null;
    const receiptWithoutHash = {
      kind: 'workspace_authoring.stored_confirmation.v0' as const,
      accepted: true as const,
      workspaceId: identity.project.workspaceId,
      generation: identity.generation,
      hostPath,
      canonicalJsonHash,
      storedRevision: this.#storedRevision,
    };
    return {
      ...receiptWithoutHash,
      lifecycleHash: fnv1a64(JSON.stringify(receiptWithoutHash)),
    };
  }

  close(input: WorkspaceAuthoringCloseInput): WorkspaceAuthoringCloseReceipt {
    const identity = this.#requireOpen('close');
    this.#validateBoundIdentity(input.expectedWorkspaceId, input.expectedGeneration);
    const dirty = this.#workingRevision !== this.#storedRevision;
    if (dirty && input.discardUnsavedWorkingState !== true) {
      throw new RuntimeBridgeError(
        'invalid_input',
        'workspace authoring has unsaved working state; store it or explicitly discard it before close',
      );
    }
    this.#bridge.unloadProjectBundle();
    this.#composition = this.#bridge.getProjectBundleCompositionStatus();
    this.#status = 'closed';
    this.#pendingStoredCandidate = null;
    this.#projectionGenerationInitialized = false;
    const receiptWithoutHash = {
      kind: 'workspace_authoring.close_receipt.v0' as const,
      closed: true as const,
      workspaceId: identity.project.workspaceId,
      generation: identity.generation,
      discardedUnsavedWorkingState: dirty,
    };
    return {
      ...receiptWithoutHash,
      lifecycleHash: fnv1a64(JSON.stringify(receiptWithoutHash)),
    };
  }

  #recordWorkingMutation(): void {
    this.#workingRevision += 1;
    this.#pendingStoredCandidate = null;
  }

  #requireOpen(operation: string): WorkspaceAuthoringIdentity {
    if (this.#status !== 'open') {
      throw new RuntimeBridgeError(
        'not_initialized',
        `${operation} requires an open workspace authoring authority`,
      );
    }
    return this.#requireIdentity(operation, true);
  }

  #requireIdentity(operation: string, requireOpen: boolean): WorkspaceAuthoringIdentity {
    if (this.#identity === null || (requireOpen && this.#status !== 'open')) {
      throw new RuntimeBridgeError(
        'not_initialized',
        `${operation} called before workspace authoring open`,
      );
    }
    return this.#identity;
  }

  #validateBoundIdentity(expectedWorkspaceId: string, expectedGeneration: number): void {
    const identity = this.#requireOpen('validateBoundIdentity');
    if (expectedWorkspaceId !== identity.project.workspaceId) {
      throw new RuntimeBridgeError(
        'stale_authority_snapshot',
        'workspace authoring request targeted a different workspace identity',
      );
    }
    if (expectedGeneration !== identity.generation) {
      throw new RuntimeBridgeError(
        'stale_authority_snapshot',
        'workspace authoring request targeted a stale generation',
      );
    }
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
