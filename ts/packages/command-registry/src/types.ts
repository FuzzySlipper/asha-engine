// @asha/command-registry — declarative studio command metadata.
//
// This package describes command identity, schemas, exposure, GUI mirrors, retry,
// undo, artifacts, and ownership. It does not execute commands and must not import
// runtime, renderer, UI, native bridge, browser, or Den surfaces.

import type {
  CatalogEntry,
  MaterialProjection,
  RenderFrameDiff,
  SceneObjectCommandRequest,
  SceneObjectCommandResult,
  SceneObjectSnapshot,
  ScreenPointToPickRayRequest,
  StaticMeshAsset,
  VoxelCommand,
  VoxelConversionApplyRequest,
  VoxelConversionEvidenceRef,
  VoxelConversionPlan,
  VoxelConversionPlanRequest,
  VoxelConversionPreview,
  VoxelConversionPreviewRequest,
  VoxelConversionReceipt,
  VoxelCoord,
  VoxelSelectionSnapshot,
} from '@asha/contracts';

export type StudioCommandId =
  | 'workspace.open_game_manifest'
  | 'workspace.validate_game_manifest'
  | 'inspection.session_status'
  | 'inspection.world_summary'
  | 'inspection.editor_state'
  | 'inspection.material'
  | 'inspection.model'
  | 'preview.model_material'
  | 'voxel_conversion.plan'
  | 'voxel_conversion.preview'
  | 'voxel_conversion.apply'
  | 'voxel_conversion.export_evidence'
  | 'scene.load_asset'
  | 'scene.read_object_snapshot'
  | 'scene.apply_object_command'
  | 'selection.voxel_from_screen_point'
  | 'selection.set_active_entity'
  | 'entity.set_name'
  | 'transform.translate_entity'
  | 'inspection.voxel'
  | 'preview.voxel_brush'
  | 'authority.voxel.apply_brush'
  | 'inspection.last_command_result'
  | 'render.capture_before_after'
  | 'export.agent_readout';

export type CommandCategory =
  | 'session'
  | 'inspection'
  | 'selection'
  | 'preview'
  | 'scene'
  | 'entity'
  | 'authority_edit'
  | 'render_evidence'
  | 'diagnostics'
  | 'export'
  | 'workspace';

export type OperationClass =
  | 'read_only'
  | 'editor_local'
  | 'authority_mutating'
  | 'render_evidence'
  | 'diagnostic_export'
  | 'workspace_io';

export type AshaLane =
  | 'contract-steward'
  | 'ts-command-registry'
  | 'ts-shell'
  | 'ts-tools'
  | 'rust-bridge'
  | 'rust-render'
  | 'rust-rule'
  | 'rust-service';

export type ContractRef =
  | { readonly package: '@asha/contracts'; readonly exportName: 'ScreenPointToPickRayRequest' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'VoxelCoord' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'VoxelSelectionSnapshot' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'VoxelCommand' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'CatalogEntry' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'MaterialProjection' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'StaticMeshAsset' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'RenderFrameDiff' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'SceneObjectSnapshot' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'SceneObjectCommandRequest' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'SceneObjectCommandResult' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'VoxelConversionPlanRequest' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'VoxelConversionPlan' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'VoxelConversionPreviewRequest' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'VoxelConversionPreview' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'VoxelConversionApplyRequest' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'VoxelConversionReceipt' }
  | { readonly package: '@asha/contracts'; readonly exportName: 'VoxelConversionEvidenceRef' };

export type RuntimeBridgeOperationRef =
  | 'initialize_engine'
  | 'pick_voxel'
  | 'select_voxel'
  | 'submit_commands'
  | 'read_voxel_mesh_evidence'
  | 'read_render_diffs'
  | 'read_model_material_preview'
  | 'read_scene_object_snapshot'
  | 'apply_scene_object_command'
  | 'read_active_runtime_project_content';

export type RuntimeSessionFacadeMethodRef =
  | 'planVoxelConversion'
  | 'previewVoxelConversion'
  | 'applyVoxelConversion'
  | 'exportVoxelConversionEvidence';

export type SchemaScalarKind = 'string' | 'number' | 'boolean' | 'integer' | 'state_hash' | 'artifact_ref' | 'null';

export type SchemaShape =
  | { readonly kind: 'empty' }
  | { readonly kind: 'contract'; readonly ref: ContractRef }
  | { readonly kind: 'object'; readonly fields: readonly SchemaField[]; readonly allowExtraFields: false }
  | { readonly kind: 'array'; readonly items: SchemaShape; readonly minItems?: number }
  | { readonly kind: 'literal'; readonly values: readonly string[] }
  | { readonly kind: 'nullable'; readonly inner: SchemaShape }
  | { readonly kind: 'scalar'; readonly scalar: SchemaScalarKind };

export interface SchemaField {
  readonly name: string;
  readonly required: boolean;
  readonly shape: SchemaShape;
  readonly summary: string;
}

export interface SchemaRef {
  readonly name: string;
  readonly version: number;
  readonly shape: SchemaShape;
}

export type AgentExposure =
  | { readonly kind: 'hidden'; readonly reason: string }
  | { readonly kind: 'read_only' }
  | { readonly kind: 'workspace_io'; readonly batchable: boolean }
  | { readonly kind: 'editor_local' }
  | { readonly kind: 'authority_mutating'; readonly requiresPreview?: boolean; readonly batchable: boolean }
  | { readonly kind: 'diagnostic_export' }
  | { readonly kind: 'render_evidence' };

export interface GuiMirror {
  readonly required: boolean;
  readonly menuPath: readonly string[];
  readonly commandPaletteVisible: boolean;
  readonly argumentSummary: string;
  readonly resultSummary: string;
  readonly artifactSummary: string;
  readonly panel?: 'timeline' | 'inspector' | 'viewport' | 'evidence' | 'export' | 'diagnostics';
  readonly dialog?: 'none' | 'simple_form' | 'advanced_form' | 'readout_only';
}

export type UndoPosture =
  | { readonly kind: 'not_undoable'; readonly reason: string }
  | { readonly kind: 'editor_local'; readonly inverseData: readonly string[] }
  | { readonly kind: 'authority_reversing'; readonly inverseCommandRefs: readonly StudioCommandId[]; readonly requiresSameStateHash: boolean }
  | { readonly kind: 'snapshot_restore'; readonly artifactType: StudioArtifactType; readonly requiresHumanConfirmation: boolean };

export type RetryPosture =
  | 'safe_to_retry'
  | 'safe_to_retry_if_state_hash_unchanged'
  | 'retry_after_status_readback'
  | 'not_idempotent'
  | 'requires_human_or_planner_decision';

export type IdempotencyPosture =
  | { readonly kind: 'idempotent'; readonly keyFields: readonly string[] }
  | { readonly kind: 'conditional'; readonly condition: string }
  | { readonly kind: 'non_idempotent'; readonly reason: string };

export type StudioArtifactType =
  | 'command_manifest'
  | 'scenario_manifest'
  | 'session_status'
  | 'world_summary'
  | 'editor_state'
  | 'selection_snapshot'
  | 'voxel_inspection'
  | 'voxel_preview'
  | 'model_metadata'
  | 'material_metadata'
  | 'voxel_conversion_plan'
  | 'voxel_conversion_preview'
  | 'voxel_conversion_receipt'
  | 'voxel_conversion_evidence'
  | 'render_diff_preview'
  | 'game_workspace'
  | 'command_result'
  | 'render_before_after'
  | 'agent_readout';

export interface ArtifactDeclaration {
  readonly type: StudioArtifactType;
  readonly required: boolean;
  readonly producedWhen: 'always' | 'on_success' | 'on_rejection' | 'when_available';
  readonly summary: string;
}

export interface StateImpact {
  readonly authority: 'none' | 'read' | 'mutate';
  readonly editor: 'none' | 'read' | 'mutate';
  readonly render: 'none' | 'read' | 'capture';
  readonly workspace: 'none' | 'read' | 'write';
}

export type RuntimeRequirement =
  | { readonly kind: 'none' }
  | { readonly kind: 'runtime_bridge_operation'; readonly operation: RuntimeBridgeOperationRef }
  | { readonly kind: 'runtime_session_facade_method'; readonly method: RuntimeSessionFacadeMethodRef }
  | { readonly kind: 'editor_store' }
  | { readonly kind: 'render_surface' }
  | { readonly kind: 'artifact_writer' };

export interface CompatibilityRequirement {
  readonly contracts: 'contracts.v0';
  readonly runtimeBridge?: 'runtime-bridge.v0';
  readonly commandRegistry: 'command-registry.v0';
}

export interface StudioCommandDefinition<Input, Output> {
  readonly id: StudioCommandId;
  readonly version: number;
  readonly label: string;
  readonly summary: string;
  readonly description?: string;
  readonly category: CommandCategory;
  readonly menuPath: readonly string[];
  readonly commandPalette: {
    readonly visible: boolean;
    readonly keywords: readonly string[];
  };
  readonly inputSchema: SchemaRef;
  readonly outputSchema: SchemaRef;
  readonly inputContractRefs: readonly ContractRef[];
  readonly outputContractRefs: readonly ContractRef[];
  readonly operationClass: OperationClass;
  readonly agentExposure: AgentExposure;
  readonly guiMirror: GuiMirror;
  readonly undo: UndoPosture;
  readonly retry: RetryPosture;
  readonly idempotency: IdempotencyPosture;
  readonly artifacts: readonly ArtifactDeclaration[];
  readonly stateImpact: StateImpact;
  readonly owningLane: AshaLane;
  readonly owningPackage: '@asha/command-registry' | '@asha/editor-tools' | '@asha/runtime-bridge' | '@asha/devtools';
  readonly runtimeRequirements: readonly RuntimeRequirement[];
  readonly compatibility: CompatibilityRequirement;
  readonly knownLimitations?: readonly string[];
  readonly typedInputExample: Input;
  readonly typedOutputExample: Output;
}

export interface EmptyInput { readonly kind: 'empty'; }
export interface EmptyOutput { readonly kind: 'ok'; }
export interface GameWorkspaceManifestInput { readonly workspaceRoot: string; readonly manifestPath: string; }
export interface GameWorkspaceOpenOutput {
  readonly workspaceVersion: string;
  readonly workspaceRoot: string;
  readonly manifestPath: string;
  readonly gameId: string;
  readonly engineVersion: string;
  readonly contractsVersion: string;
  readonly runtimeBridgeVersion: string;
  readonly devtoolsProtocolVersion: string;
  readonly sceneRoots: readonly string[];
  readonly assetRoots: readonly string[];
  readonly catalogPackages: readonly string[];
  readonly policyPackages: readonly string[];
  readonly attachEndpoint: string;
  readonly devCommand: string;
  readonly publishCommand: string;
  readonly workspaceHash: string;
}
export interface GameWorkspaceValidateOutput {
  readonly valid: boolean;
  readonly workspaceHash: string | null;
  readonly diagnostics: readonly {
    readonly code: string;
    readonly message: string;
    readonly source: string | null;
  }[];
}
export interface SessionIdInput { readonly sessionId: string; }
export interface SessionStatusOutput { readonly sessionId: string; readonly status: 'not_started' | 'ready' | 'degraded' | 'unavailable'; }
export interface WorldSummaryOutput { readonly authorityHash: string | null; readonly voxelVolumeCount: number; readonly sceneNodeCount: number; }
export interface EditorStateOutput { readonly editorVersion: string; readonly selectedVoxel: VoxelCoord | null; }
export interface MaterialInspectionInput { readonly sessionId: string; readonly materialId: string; }
export interface MaterialInspectionOutput { readonly materialId: string; readonly catalogEntry: CatalogEntry; readonly material: MaterialProjection; }
export interface ModelInspectionInput { readonly sessionId: string; readonly assetId: string; }
export interface ModelInspectionOutput { readonly assetId: string; readonly meshAsset: StaticMeshAsset; readonly materialSlots: readonly string[]; }
export interface ModelMaterialPreviewInput { readonly sessionId: string; readonly modelAsset: StaticMeshAsset; readonly materialId: string; }
export interface ModelMaterialPreviewOutput { readonly previewDiff: RenderFrameDiff; readonly rendererClassification: 'reference_preview' | 'runtime_readback'; readonly diagnostics: readonly string[]; }
export interface VoxelConversionPlanCommandInput { readonly sessionId: string; readonly request: VoxelConversionPlanRequest; }
export interface VoxelConversionPlanCommandOutput { readonly plan: VoxelConversionPlan; }
export interface VoxelConversionPreviewCommandInput { readonly sessionId: string; readonly request: VoxelConversionPreviewRequest; }
export interface VoxelConversionPreviewCommandOutput { readonly preview: VoxelConversionPreview; }
export interface VoxelConversionApplyCommandInput { readonly sessionId: string; readonly request: VoxelConversionApplyRequest; }
export interface VoxelConversionApplyCommandOutput { readonly receipt: VoxelConversionReceipt; }
export interface VoxelConversionEvidenceExportInput { readonly sessionId: string; readonly evidence: readonly VoxelConversionEvidenceRef[]; }
export interface VoxelConversionEvidenceExportOutput { readonly evidence: readonly VoxelConversionEvidenceRef[]; }
export interface LoadSceneAssetPlacement { readonly translation: readonly number[]; readonly rotation: readonly number[]; readonly scale: readonly number[]; }
export interface LoadSceneAssetInput { readonly sessionId: string; readonly assetId: string; readonly materialId: string; readonly placement: LoadSceneAssetPlacement; }
export interface LoadSceneAssetOutput { readonly assetId: string; readonly renderableIds: readonly string[]; readonly loadDiff: RenderFrameDiff; readonly rendererClassification: 'reference_placement' | 'runtime_readback'; readonly diagnostics: readonly string[]; }
export interface ReadSceneObjectSnapshotOutput { readonly snapshot: SceneObjectSnapshot; }
export interface ApplySceneObjectCommandInput { readonly sessionId: string; readonly request: SceneObjectCommandRequest; }
export interface ApplySceneObjectCommandOutput { readonly result: SceneObjectCommandResult; }
export interface ScreenPointInput { readonly sessionId: string; readonly request: ScreenPointToPickRayRequest; }
export interface VoxelSelectionOutput { readonly selection: VoxelSelectionSnapshot; }
export interface SetActiveEntityInput { readonly sessionId: string; readonly entityId: string; }
export interface SetActiveEntityOutput { readonly entityId: string; readonly renderableId: string; readonly selectionHash: string; readonly selected: boolean; }
export interface SetEntityNameInput { readonly sessionId: string; readonly entityId: string; readonly name: string; }
export interface SetEntityNameOutput { readonly entityId: string; readonly renderableId: string; readonly name: string; readonly nameHash: string; readonly applied: boolean; }
export type TransformAxis = 'x' | 'y' | 'z';
export type TransformEditMode = 'preview' | 'apply';
export interface TranslateEntityInput { readonly sessionId: string; readonly entityId: string; readonly axis: TransformAxis; readonly delta: number; readonly mode: TransformEditMode; }
export interface TranslateEntityOutput { readonly entityId: string; readonly renderableId: string; readonly axis: TransformAxis; readonly delta: number; readonly mode: TransformEditMode; readonly translationBefore: readonly [number, number, number]; readonly translationAfter: readonly [number, number, number]; readonly transformHash: string; readonly applied: boolean; }
export interface VoxelInspectionInput { readonly sessionId: string; readonly voxel: VoxelCoord; }
export interface VoxelInspectionOutput { readonly voxel: VoxelCoord; readonly materialId: number | null; readonly occupied: boolean; }
export interface VoxelBrushPreviewInput { readonly sessionId: string; readonly anchor: VoxelCoord; readonly commands: readonly VoxelCommand[]; }
export interface VoxelBrushPreviewOutput { readonly targetVoxels: readonly VoxelCoord[]; readonly previewVersion: string; }
export interface ApplyVoxelBrushInput { readonly sessionId: string; readonly commands: readonly VoxelCommand[]; readonly expectedStateHash: string | null; }
export interface ApplyVoxelBrushOutput { readonly accepted: boolean; readonly authorityBeforeHash: string | null; readonly authorityAfterHash: string | null; }
export interface LastCommandResultOutput { readonly sequenceId: string | null; readonly status: 'ok' | 'rejected' | 'partial' | 'failed' | 'unavailable' | null; }
export interface CaptureBeforeAfterInput { readonly sessionId: string; readonly beforeArtifactId: string; readonly afterArtifactId: string; }
export interface CaptureBeforeAfterOutput { readonly artifactId: string; readonly renderBeforeHash: string | null; readonly renderAfterHash: string | null; }
export interface ExportAgentReadoutInput { readonly sessionId: string; readonly includeVisualEvidence: boolean; }
export interface ExportAgentReadoutOutput { readonly artifactId: string; readonly commandCount: number; }

export interface StudioCommandCatalogEntry {
  readonly id: StudioCommandId;
  readonly version: number;
  readonly label: string;
  readonly summary: string;
  readonly category: CommandCategory;
  readonly operationClass: OperationClass;
  readonly agentExposureKind: AgentExposure['kind'];
  readonly menuPath: readonly string[];
  readonly commandPaletteVisible: boolean;
  readonly commandPaletteKeywords: readonly string[];
  readonly guiMirror: GuiMirror;
  readonly inputSchemaName: string;
  readonly outputSchemaName: string;
  readonly inputContractRefs: readonly ContractRef[];
  readonly outputContractRefs: readonly ContractRef[];
  readonly artifacts: readonly ArtifactDeclaration[];
  readonly stateImpact: StateImpact;
  readonly owningLane: AshaLane;
  readonly owningPackage: StudioCommandDefinition<object, object>['owningPackage'];
  readonly runtimeRequirements: readonly RuntimeRequirement[];
}

export interface StudioCommandCatalog {
  readonly schemaVersion: 1;
  readonly generatedFrom: 'COMMAND_MANIFEST';
  readonly commandRegistryVersion: CompatibilityRequirement['commandRegistry'];
  readonly commands: readonly StudioCommandCatalogEntry[];
}

export type StudioCommandManifest = readonly StudioCommandDefinition<object, object>[];
