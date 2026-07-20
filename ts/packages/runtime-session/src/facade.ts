import type {
  CameraCreateRequest,
  CameraControllerReadRequest,
  CameraControllerState,
  CameraModeChangeReceipt,
  CameraModeCommand,
  CameraNavigationInputEnvelope,
  CameraNavigationReceipt,
  CameraProjectionRequest,
  CollisionConstrainedCameraInputEnvelope,
  CommandBatch,
  DeveloperConsoleSnapshot,
  FirstPersonCameraInputEnvelope,
  GameRuleCatalog,
  GameRuleResolutionRequest,
  InputActionReplayReceipt,
  InputContextChangeReceipt,
  InputContextCommand,
  InputContextStackState,
  InputResolutionReceipt,
  InputSessionConfigureRequest,
  InputSessionSnapshot,
  VoxelAnnotationEditReceipt,
  VoxelAnnotationEditRequest,
  VoxelAnnotationLayerExportReceipt,
  VoxelAnnotationLayerExportRequest,
  VoxelAnnotationLayerLoadReceipt,
  VoxelAnnotationLayerLoadRequest,
  VoxelAnnotationLayerValidationReport,
  VoxelAnnotationLayerValidationRequest,
  VoxelAnnotationQueryReadout,
  VoxelAnnotationQueryRequest,
  VoxelConversionApplyRequest,
  VoxelConversionEvidenceRef,
  VoxelConversionMeshAssetRegistrationRequest,
  VoxelConversionMeshSourceImportReceipt,
  VoxelConversionMeshSourceImportRequest,
  VoxelConversionPlan,
  VoxelConversionPlanRequest,
  VoxelConversionPreview,
  VoxelConversionPreviewRequest,
  VoxelConversionReceipt,
  VoxelConversionSourceMetadataReadout,
  VoxelConversionSourceMetadataRequest,
  VoxelConversionSourceRegistration,
  VoxelConversionSourceRegistrationRequest,
  VoxelEditHistoryReadRequest,
  VoxelEditHistoryRedoReceipt,
  VoxelEditHistoryRedoRequest,
  VoxelEditHistoryRevertReceipt,
  VoxelEditHistoryRevertRequest,
  VoxelEditHistorySummary,
  VoxelEditHistoryUndoReceipt,
  VoxelEditHistoryUndoRequest,
  VoxelModelInfoReadout,
  VoxelModelInfoRequest,
  VoxelModelWindowReadout,
  VoxelModelWindowRequest,
  VoxelVolumeAssetExportReceipt,
  VoxelVolumeAssetExportRequest,
  VoxelVolumeAssetLoadReceipt,
  VoxelVolumeAssetLoadRequest,
  VoxelVolumeAssetUnloadReceipt,
  VoxelVolumeAssetUnloadRequest,
  VoxelVolumeAssetPaletteUpdateReceipt,
  VoxelVolumeAssetPaletteUpdateRequest,
  VoxelVolumeAssetSaveReceipt,
  VoxelVolumeAssetSaveRequest,
  VoxelVolumeAuthoringInitializeReceipt,
  VoxelVolumeAuthoringInitializeRequest,
  WeaponEffectHookRequest,
  RawInputSample,
  RecordedInputAction,
  TimeControlCommand,
  TimeControlReceipt,
  TimeControlState,
  SceneDocumentCodecResult,
  SceneDocumentDecodeRequest,
  SceneDocumentEncodeRequest,
} from '@asha/contracts';
import type { CombatFeedbackProjection } from './combat-feedback.js';
import type { CombatRuntimeReadout } from './combat-readout.js';
import type {
  EncounterDirectorReadout,
  EncounterDirectorReadoutRequest,
  EncounterTransitionRequest,
  RuntimeSessionEncounterTransitionReceipt,
} from './encounter-director.js';
import type {
  RuntimeSessionCommandReceipt,
  RuntimeSessionInitializeInput,
  RuntimeSessionProjectionSummary,
  RuntimeSessionStateSummary,
  RuntimeSessionTelemetrySummary,
  RuntimeSessionTickInput,
  RuntimeSessionTickResult,
} from './facade-core.js';
import type {
  RuntimeSessionEcrpReadout,
} from './facade-ecrp.js';
import type {
  RuntimeSessionActionIntentReceipt,
  RuntimeSessionAnimationIntentReadout,
  RuntimeSessionAutonomousPolicyTickInput,
  RuntimeSessionAutonomousPolicyTickReadout,
  RuntimeSessionCameraCollisionInputReceipt,
  RuntimeSessionCameraCreateReceipt,
  RuntimeSessionCameraInputReceipt,
  RuntimeSessionCameraProjectionReadout,
  RuntimeSessionCombatFeedbackProjectionRequest,
  RuntimeSessionCombatReadoutRequest,
  RuntimeSessionGameExtensionWeaponEffectReceipt,
  RuntimeSessionGameRuleCatalogValidationReceipt,
  RuntimeSessionGameRuleEffectIntentReceipt,
} from './facade-gameplay.js';
import type {
  RuntimeSessionLifecycleRestartReceipt,
  RuntimeSessionLifecycleStatusReadout,
  RuntimeSessionLifecycleStatusRequest,
  RuntimeSessionRestartIntent,
  RuntimeSessionRestartResult,
} from './facade-lifecycle.js';
import type {
  RuntimeSessionProjectCloseReceipt,
  RuntimeSessionProjectLoadInput,
  RuntimeSessionProjectLoadReceipt,
} from './facade-project.js';
import type { NavPathQueryRequest, NavPathReadout, NavPolicyViewReadout, NavProjectionReadout } from './nav-readout.js';
import type { RuntimeActionIntentEnvelope } from './runtime-action.js';
import type {
  FpsPrimaryFireRequest,
  GameRuleRuntimeReadout,
} from './transport-contracts.js';

export interface RuntimeSessionFacade {
  initialize(input: RuntimeSessionInitializeInput): RuntimeSessionStateSummary;
  loadProject(input: RuntimeSessionProjectLoadInput): Promise<RuntimeSessionProjectLoadReceipt>;
  closeProject(): RuntimeSessionProjectCloseReceipt;
  configureInputSession(request: InputSessionConfigureRequest): InputSessionSnapshot;
  applyInputContextCommand(command: InputContextCommand): InputContextChangeReceipt;
  submitRawInput(sample: RawInputSample): InputResolutionReceipt;
  replayResolvedInputAction(record: RecordedInputAction): InputActionReplayReceipt;
  readInputContextState(): InputContextStackState;
  applyTimeControlCommand(command: TimeControlCommand): TimeControlReceipt;
  readTimeControlState(): TimeControlState;
  decodeSceneDocument(request: SceneDocumentDecodeRequest): SceneDocumentCodecResult;
  encodeSceneDocument(request: SceneDocumentEncodeRequest): SceneDocumentCodecResult;
  submitCommands(batch: CommandBatch): RuntimeSessionCommandReceipt;
  tick(input?: RuntimeSessionTickInput): RuntimeSessionTickResult;
  createCamera(request: CameraCreateRequest): RuntimeSessionCameraCreateReceipt;
  applyCameraModeCommand(command: CameraModeCommand): CameraModeChangeReceipt;
  applyCameraNavigationInput(input: CameraNavigationInputEnvelope): CameraNavigationReceipt;
  readCameraControllerState(request: CameraControllerReadRequest): CameraControllerState;
  applyFirstPersonCameraInput(envelope: FirstPersonCameraInputEnvelope): RuntimeSessionCameraInputReceipt;
  applyCollisionConstrainedCameraInput(
    envelope: CollisionConstrainedCameraInputEnvelope,
  ): RuntimeSessionCameraCollisionInputReceipt;
  submitRuntimeActionIntent(envelope: RuntimeActionIntentEnvelope): RuntimeSessionActionIntentReceipt;
  submitGameExtensionWeaponEffect(
    hook: WeaponEffectHookRequest,
    primaryFire: FpsPrimaryFireRequest,
  ): RuntimeSessionGameExtensionWeaponEffectReceipt;
  validateGameRuleCatalog(catalog: GameRuleCatalog): RuntimeSessionGameRuleCatalogValidationReceipt;
  submitGameRuleEffectIntent(
    catalog: GameRuleCatalog,
    request: GameRuleResolutionRequest,
  ): RuntimeSessionGameRuleEffectIntentReceipt;
  readGameRuleRuntimeReadout(): GameRuleRuntimeReadout;
  runAutonomousPolicyTick(input: RuntimeSessionAutonomousPolicyTickInput): RuntimeSessionAutonomousPolicyTickReadout;
  readLifecycleStatus(request?: RuntimeSessionLifecycleStatusRequest): RuntimeSessionLifecycleStatusReadout;
  requestSessionRestart(intent: RuntimeSessionRestartIntent): RuntimeSessionLifecycleRestartReceipt;
  readEncounterDirector(request?: EncounterDirectorReadoutRequest): EncounterDirectorReadout;
  requestEncounterTransition(
    request: EncounterTransitionRequest,
  ): RuntimeSessionEncounterTransitionReceipt;
  readCombatReadout(request?: RuntimeSessionCombatReadoutRequest): CombatRuntimeReadout;
  readCombatFeedbackProjection(
    request?: RuntimeSessionCombatFeedbackProjectionRequest,
  ): CombatFeedbackProjection;
  readNavProjection(): NavProjectionReadout;
  queryNavPath(request?: NavPathQueryRequest): NavPathReadout;
  readNavPolicyView(): NavPolicyViewReadout;
  registerVoxelConversionSource(request: VoxelConversionSourceRegistrationRequest): VoxelConversionSourceRegistration;
  registerVoxelConversionMeshAsset(request: VoxelConversionMeshAssetRegistrationRequest): VoxelConversionSourceRegistration;
  importVoxelConversionMeshSource(request: VoxelConversionMeshSourceImportRequest): VoxelConversionMeshSourceImportReceipt;
  readVoxelConversionSourceMetadata(request: VoxelConversionSourceMetadataRequest): VoxelConversionSourceMetadataReadout;
  planVoxelConversion(request: VoxelConversionPlanRequest): VoxelConversionPlan;
  previewVoxelConversion(request: VoxelConversionPreviewRequest): VoxelConversionPreview;
  applyVoxelConversion(request: VoxelConversionApplyRequest): VoxelConversionReceipt;
  exportVoxelConversionEvidence(evidence: readonly VoxelConversionEvidenceRef[]): readonly VoxelConversionEvidenceRef[];
  readVoxelModelInfo(request: VoxelModelInfoRequest): VoxelModelInfoReadout;
  readVoxelModelWindow(request: VoxelModelWindowRequest): VoxelModelWindowReadout;
  exportVoxelVolumeAsset(request: VoxelVolumeAssetExportRequest): VoxelVolumeAssetExportReceipt;
  saveVoxelVolumeAsset(request: VoxelVolumeAssetSaveRequest): VoxelVolumeAssetSaveReceipt;
  updateVoxelVolumeAssetPalette(request: VoxelVolumeAssetPaletteUpdateRequest): VoxelVolumeAssetPaletteUpdateReceipt;
  initializeVoxelVolumeAuthoring(request: VoxelVolumeAuthoringInitializeRequest): VoxelVolumeAuthoringInitializeReceipt;
  loadVoxelVolumeAsset(request: VoxelVolumeAssetLoadRequest): VoxelVolumeAssetLoadReceipt;
  unloadVoxelVolumeAsset(request: VoxelVolumeAssetUnloadRequest): VoxelVolumeAssetUnloadReceipt;
  validateVoxelAnnotationLayer(request: VoxelAnnotationLayerValidationRequest): VoxelAnnotationLayerValidationReport;
  loadVoxelAnnotationLayer(request: VoxelAnnotationLayerLoadRequest): VoxelAnnotationLayerLoadReceipt;
  readVoxelAnnotationQuery(request: VoxelAnnotationQueryRequest): VoxelAnnotationQueryReadout;
  applyVoxelAnnotationEdit(request: VoxelAnnotationEditRequest): VoxelAnnotationEditReceipt;
  exportVoxelAnnotationLayer(request: VoxelAnnotationLayerExportRequest): VoxelAnnotationLayerExportReceipt;
  readVoxelEditHistory(request: VoxelEditHistoryReadRequest): VoxelEditHistorySummary;
  previewVoxelEditRevert(request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt; applyVoxelEditRevert(request: VoxelEditHistoryRevertRequest): VoxelEditHistoryRevertReceipt;
  undoVoxelEdit(request: VoxelEditHistoryUndoRequest): VoxelEditHistoryUndoReceipt; redoVoxelEdit(request: VoxelEditHistoryRedoRequest): VoxelEditHistoryRedoReceipt;
  readEcrpRuntimeReadout(): RuntimeSessionEcrpReadout;
  readCameraProjection(request: CameraProjectionRequest): RuntimeSessionCameraProjectionReadout;
  readAnimationIntent(): RuntimeSessionAnimationIntentReadout; readProjection(): RuntimeSessionProjectionSummary;
  readDeveloperConsole(): DeveloperConsoleSnapshot;
  readTelemetry(): RuntimeSessionTelemetrySummary;
  restart(): RuntimeSessionRestartResult;
}
