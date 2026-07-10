import type {
  CameraCreateRequest,
  CameraProjectionRequest,
  CollisionConstrainedCameraInputEnvelope,
  CommandBatch,
  FirstPersonCameraInputEnvelope,
  GameRuleCatalog,
  GameRuleResolutionRequest,
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
  VoxelVolumeAssetPaletteUpdateReceipt,
  VoxelVolumeAssetPaletteUpdateRequest,
  VoxelVolumeAssetSaveReceipt,
  VoxelVolumeAssetSaveRequest,
  WeaponEffectHookRequest,
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
  RuntimeSessionEcrpProjectLoadInput,
  RuntimeSessionEcrpProjectLoadReceipt,
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
  RuntimeSessionGeneratedTunnelOperationReceipt,
} from './facade-gameplay.js';
import type {
  RuntimeSessionLifecycleRestartReceipt,
  RuntimeSessionLifecycleStatusReadout,
  RuntimeSessionLifecycleStatusRequest,
  RuntimeSessionRestartIntent,
  RuntimeSessionRestartResult,
} from './facade-lifecycle.js';
import type {
  GeneratedTunnelOperationRequest,
  GeneratedTunnelReadout,
  GeneratedTunnelReadoutRequest,
} from './generated-tunnel.js';
import type { NavPathQueryRequest, NavPathReadout, NavPolicyViewReadout, NavProjectionReadout } from './nav-readout.js';
import type { RuntimeActionIntentEnvelope } from './runtime-action.js';
import type {
  FpsPrimaryFireRequest,
  GameRuleRuntimeReadout,
} from './transport-contracts.js';

export interface RuntimeSessionFacade {
  initialize(input: RuntimeSessionInitializeInput): RuntimeSessionStateSummary;
  loadEcrpProject(input: RuntimeSessionEcrpProjectLoadInput): RuntimeSessionEcrpProjectLoadReceipt;
  submitCommands(batch: CommandBatch): RuntimeSessionCommandReceipt;
  tick(input?: RuntimeSessionTickInput): RuntimeSessionTickResult;
  createCamera(request: CameraCreateRequest): RuntimeSessionCameraCreateReceipt;
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
  readGeneratedTunnelReadout(request?: GeneratedTunnelReadoutRequest): GeneratedTunnelReadout;
  readNavProjection(): NavProjectionReadout;
  queryNavPath(request?: NavPathQueryRequest): NavPathReadout;
  readNavPolicyView(): NavPolicyViewReadout;
  requestGeneratedTunnelOperation(
    request: GeneratedTunnelOperationRequest,
  ): RuntimeSessionGeneratedTunnelOperationReceipt;
  registerVoxelConversionSource(request: VoxelConversionSourceRegistrationRequest): VoxelConversionSourceRegistration;
  registerVoxelConversionMeshAsset(request: VoxelConversionMeshAssetRegistrationRequest): VoxelConversionSourceRegistration;
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
  loadVoxelVolumeAsset(request: VoxelVolumeAssetLoadRequest): VoxelVolumeAssetLoadReceipt;
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
  readTelemetry(): RuntimeSessionTelemetrySummary;
  restart(): RuntimeSessionRestartResult;
}
