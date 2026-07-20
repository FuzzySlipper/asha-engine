import type {
  WorkspaceAuthoringCloseInput,
  WorkspaceAuthoringCloseReceipt,
  WorkspaceAuthoringFacade,
  WorkspaceAuthoringIdentity,
  WorkspaceAuthoringProjectOpenInput,
  WorkspaceAuthoringProjectOpenReceipt,
  WorkspaceAuthoringProjectionSummary,
  WorkspaceProjectWritePrepareInput,
  WorkspaceAuthoringStateSummary,
  WorkspaceAuthoringStoredConfirmationInput,
  WorkspaceAuthoringStoredConfirmationReceipt,
  WorkspaceVoxelInstancePickInput,
  WorkspaceVoxelProjectionBindingInput,
} from '@asha/runtime-session';
import {
  validateGeneratedWireValue,
  type GeneratedWireValue,
  type VoxelProjectionBindingReceipt,
  type VoxelVolumeAsset,
} from '@asha/contracts';
import type {
  ProjectContentDocumentKind,
  ProjectContentAuthoringCommand,
  ProjectContentAuthoringRequest,
  ProjectContentAuthoringResult,
  ProjectContentCodecResult,
  ProjectContentDecodeRequest,
  ProjectContentEncodeRequest,
  ProceduralEnvironmentApplyRequest,
  ProceduralEnvironmentApplyResult,
  ProceduralEnvironmentPreviewRequest,
  ProceduralEnvironmentPreviewResult,
  ProjectWriteConfirmReceipt,
  ProjectWritePrepareReceipt,
  ProjectWritePublication,
  SceneDocumentCodecResult,
  SceneDocumentDecodeRequest,
  WorkspaceAuthoringOpenRequest,
} from '@asha/contracts';
import { loadAshaProjectSource } from '@asha/game-workspace';
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
  return {
    kind: value.kind,
    status: value.status,
    identity: {
      kind: value.identity.kind,
      authoringId: value.identity.authoringId,
      mode: value.identity.mode,
      generation: value.identity.generation,
      seed: value.identity.seed,
      project: value.identity.project,
      nonClaims: value.identity.nonClaims,
    },
    composition: value.composition,
    workingRevision: value.workingRevision,
    storedRevision: value.storedRevision,
    dirty: value.dirty,
    lastStoredCanonicalJsonHash: value.lastStoredCanonicalJsonHash,
    authoritySnapshotHash: value.authoritySnapshotHash,
    lifecycleHash: value.lifecycleHash,
  } as WorkspaceAuthoringStateSummary;
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
  #projectContent: ProjectContentCodecResult | null = null;

  constructor(bridge: RuntimeBridge) {
    this.#bridge = bridge;
  }

  #openTransport(input: WorkspaceAuthoringOpenRequest): WorkspaceAuthoringStateSummary {
    const state = workspaceAuthoringStateFromContract(
      this.#bridge.openWorkspaceAuthoring(input),
    );
    this.#state = state;
    this.#nextProjectionCursor = frameCursor(0);
    this.#voxelProjectionBinding = null;
    this.#projectContent = null;
    return state;
  }

  async openProject(
    input: WorkspaceAuthoringProjectOpenInput,
  ): Promise<WorkspaceAuthoringProjectOpenReceipt> {
    const loaded = await loadAshaProjectSource(input.source);
    const opened = this.#openTransport({
      authoringId: input.authoringId,
      seed: input.seed,
      project: {
        gameId: String(loaded.manifest.project.id),
        workspaceId: input.workspaceId,
      },
      projectBundle: {
        bundleSchemaVersion: loaded.manifest.bundleSchemaVersion,
        protocolVersion: loaded.manifest.protocolVersion,
        sceneId: loaded.manifest.entryScene,
      },
    });
    try {
      const files = new Map(loaded.files.map((file) => [file.path, file.bytes]));
      const sceneDocuments = [];
      for (const scene of loaded.manifest.scenes) {
        const source = requiredProjectText(files, scene.artifact);
        const decoded = this.decodeSceneDocument({ sourceText: source });
        if (!decoded.accepted) {
          throw new RuntimeBridgeError(
            'invalid_input',
            `Rust rejected scene "${scene.artifact}": ${formatDiagnostics(decoded.diagnostics)}`,
          );
        }
        if (decoded.document === null) {
          throw new RuntimeBridgeError(
            'internal',
            `Rust accepted scene "${scene.artifact}" without returning its canonical document.`,
          );
        }
        sceneDocuments.push(decoded.document);
      }
      const projectContentSources = loaded.manifest.artifacts
        .filter((artifact) => isProjectContentRole(artifact.role))
        .map((artifact) => {
          const sourceText = requiredProjectText(files, artifact.path);
          const identity = projectContentIdentity(sourceText, artifact.path);
          return {
            sourcePath: artifact.path,
            documentId: identity.documentId,
            kind: identity.kind,
            sourceText,
          };
        });
      let projectContent: ProjectContentCodecResult | null = null;
      if (projectContentSources.length > 0) {
        const decoded = this.decodeProjectContent({ sources: projectContentSources });
        if (!decoded.accepted) {
          throw new RuntimeBridgeError(
            'invalid_input',
            `Rust rejected ProjectContent: ${formatDiagnostics(decoded.diagnostics)}`,
          );
        }
        projectContent = decoded;
      }
      const voxelArtifacts = loaded.manifest.artifacts.filter(
        (artifact) => artifact.role === 'voxelVolumeAsset',
      );
      for (const [index, artifact] of voxelArtifacts.entries()) {
        const sourceText = requiredProjectText(files, artifact.path);
        const asset = parseVoxelVolumeAsset(sourceText, artifact.path);
        const loadedAsset = this.loadVoxelVolumeAsset({
          asset,
          targetGrid: index + 1,
          targetVolumeAssetId: asset.assetId,
          replaceExisting: true,
          includeMaterialCounts: true,
        });
        if (!loadedAsset.loaded) {
          throw new RuntimeBridgeError(
            'invalid_input',
            `Rust rejected voxel asset "${artifact.path}": ${formatDiagnostics(loadedAsset.diagnostics)}`,
          );
        }
      }
      return {
        state: this.readState(),
        manifestJson: loaded.manifestJson,
        sceneDocuments,
        projectContent,
      };
    } catch (error) {
      const openErrorMessage = error instanceof Error ? error.message : String(error);
      try {
        this.close({
          expectedWorkspaceId: opened.identity.project.workspaceId,
          expectedGeneration: opened.identity.generation,
          discardUnsavedWorkingState: true,
        });
      } catch (cleanupError) {
        const cleanupErrorMessage = cleanupError instanceof Error
          ? cleanupError.message
          : String(cleanupError);
        throw new RuntimeBridgeError(
          'internal',
          `openProject rejected (${openErrorMessage}) and workspace cleanup also failed: ${cleanupErrorMessage}`,
        );
      }
      throw error;
    }
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
    const result = this.#bridge.decodeProjectContent(input);
    if (result.accepted) this.#projectContent = result;
    return result;
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
      this.#projectContent = result;
      this.#refreshAfterMutation();
    }
    return result;
  }

  applyProjectContentCommand(
    command: ProjectContentAuthoringCommand,
  ): ProjectContentAuthoringResult {
    const state = this.#requireOpenState('applyProjectContentCommand');
    const current = this.#projectContent;
    if (!current?.accepted || current.setHash === null) {
      throw new RuntimeBridgeError(
        'invalid_input',
        'applyProjectContentCommand requires an accepted current ProjectContent set',
      );
    }
    return this.applyProjectContentAuthoring({
      expectedWorkspaceId: state.identity.project.workspaceId,
      expectedGeneration: state.identity.generation,
      expectedWorkingRevision: state.workingRevision,
      expectedSetHash: current.setHash,
      command,
    });
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

  prepareProjectWrite(input: WorkspaceProjectWritePrepareInput): ProjectWritePrepareReceipt {
    const state = this.#requireOpenState('prepareProjectWrite');
    return this.#bridge.prepareProjectWrite({
      expectedWorkspaceId: state.identity.project.workspaceId,
      expectedGeneration: state.identity.generation,
      expectedWorkingRevision: state.workingRevision,
      observedPrior: input.observedPrior,
      priorManifestJson: input.priorManifestJson,
      relocations: [...(input.relocations ?? [])],
    });
  }

  confirmProjectWrite(publication: ProjectWritePublication): ProjectWriteConfirmReceipt {
    const state = this.#requireOpenState('confirmProjectWrite');
    const receipt = this.#bridge.confirmProjectWrite({
      expectedWorkspaceId: state.identity.project.workspaceId,
      expectedGeneration: state.identity.generation,
      expectedWorkingRevision: state.workingRevision,
      publication,
    });
    if (receipt.accepted) this.#refreshAfterMutation();
    return receipt;
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
    this.#projectContent = null;
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

const PROJECT_CONTENT_KINDS: readonly ProjectContentDocumentKind[] = [
  'entityDefinition',
  'assetCatalog',
  'prefabRegistry',
  'gameplayConfiguration',
  'presentationCatalog',
];

function isProjectContentRole(role: string): boolean {
  return role === 'projectContent'
    || role === 'prefabRegistry'
    || role === 'entityDefinitionCatalog'
    || role === 'materialCatalog';
}

function requiredProjectText(files: ReadonlyMap<string, Uint8Array>, path: string): string {
  const bytes = files.get(path);
  if (bytes === undefined) throw new RuntimeBridgeError('invalid_input', `project source is missing "${path}"`);
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    throw new RuntimeBridgeError('invalid_input', `project source "${path}" is not UTF-8 text`);
  }
}

function projectContentIdentity(
  sourceText: string,
  path: string,
): { readonly documentId: string; readonly kind: ProjectContentDocumentKind } {
  let value: GeneratedWireValue;
  try {
    value = JSON.parse(sourceText) as GeneratedWireValue;
  } catch {
    throw new RuntimeBridgeError('invalid_input', `ProjectContent "${path}" is not JSON`);
  }
  if (
    typeof value !== 'object'
    || value === null
    || !('documentId' in value)
    || !('documentKind' in value)
  ) {
    throw new RuntimeBridgeError(
      'invalid_input',
      `ProjectContent "${path}" has no canonical document identity`,
    );
  }
  const documentId = value['documentId'];
  const kind = value['documentKind'];
  if (typeof documentId !== 'string' || documentId.trim().length === 0) {
    throw new RuntimeBridgeError(
      'invalid_input',
      `ProjectContent "${path}" has an invalid canonical documentId`,
    );
  }
  if (
    typeof kind !== 'string'
    || !PROJECT_CONTENT_KINDS.includes(kind as ProjectContentDocumentKind)
  ) {
    throw new RuntimeBridgeError(
      'invalid_input',
      `ProjectContent "${path}" has an unsupported canonical documentKind`,
    );
  }
  return { documentId, kind: kind as ProjectContentDocumentKind };
}

function parseVoxelVolumeAsset(sourceText: string, path: string): VoxelVolumeAsset {
  let value: GeneratedWireValue;
  try {
    value = JSON.parse(sourceText) as GeneratedWireValue;
  } catch {
    throw new RuntimeBridgeError('invalid_input', `voxel asset "${path}" is not JSON`);
  }
  const validation = validateGeneratedWireValue(
    'voxelAsset.VoxelVolumeAsset',
    value as GeneratedWireValue,
  );
  if (!validation.valid) {
    throw new RuntimeBridgeError(
      'invalid_input',
      `voxel asset "${path}" is malformed at ${validation.issue.path}: ${validation.issue.message}`,
    );
  }
  return value as GeneratedWireValue & VoxelVolumeAsset;
}

function formatDiagnostics(diagnostics: readonly { readonly message: string }[]): string {
  return diagnostics.map((diagnostic) => diagnostic.message).join('; ');
}

export interface WorkspaceAuthoringFacadeOptions {
  readonly bridge: RuntimeBridge;
}

export function createWorkspaceAuthoringFacade(
  options: WorkspaceAuthoringFacadeOptions,
): WorkspaceAuthoringFacade {
  return new RustBackedWorkspaceAuthoringFacade(options.bridge);
}
