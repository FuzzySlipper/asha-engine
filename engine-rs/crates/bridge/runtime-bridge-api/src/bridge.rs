use crate::*;

// ── The bridge surface ────────────────────────────────────────────────────────

/// The bounded set of verbs every transport implements. There is no generic
/// `call(method, json)` — adding a verb here is a reviewed boundary change.
pub trait RuntimeBridge {
    fn initialize_engine(&mut self, config: EngineConfig) -> BridgeResult<EngineHandle>;
    fn step_simulation(&mut self, input: StepInputEnvelope) -> BridgeResult<StepResult>;
    /// Submit a batch of proposed voxel commands for Rust-side validation + apply
    /// (mirrors manifest `submit_commands`). Accepted commands mutate authority and
    /// mark dirty chunks; rejected commands are classified and leave state unchanged.
    fn submit_commands(&mut self, batch: CommandBatch) -> BridgeResult<CommandResult>;
    /// Raycast a world-space [`PickRay`] against authority voxel state and return the
    /// nearest classified [`PickResult`] (mirrors manifest `pick_voxel`). Rust owns
    /// the voxel-grid raycast; the renderer only builds the ray. Reads authority —
    /// never mutates it.
    fn pick_voxel(&self, ray: PickRay) -> BridgeResult<PickResult>;
    /// Apply first-person view input while constraining translation against the
    /// authority-derived voxel collision projection.
    fn apply_collision_constrained_camera_input(
        &mut self,
        input: CollisionConstrainedCameraInputEnvelope,
    ) -> BridgeResult<CameraCollisionSnapshot>;
    /// Materialize a selected generated tunnel as the authority collision world.
    fn apply_generated_tunnel_to_runtime_world(
        &mut self,
        request: GeneratedTunnelRuntimeApplyRequest,
    ) -> BridgeResult<GeneratedTunnelRuntimeApplyReceipt>;
    /// Derive a camera/projection-sourced ray and authority selection evidence.
    fn select_voxel(
        &self,
        request: ScreenPointToPickRayRequest,
    ) -> BridgeResult<VoxelSelectionSnapshot>;
    /// Read compact deterministic voxel mesh evidence for resident/requested chunks.
    /// This summarizes authority-derived `svc-mesh` output with hashes/stats, not
    /// renderer-owned objects or inline Three.js geometry.
    fn read_voxel_mesh_evidence(
        &self,
        request: VoxelMeshEvidenceRequest,
    ) -> BridgeResult<VoxelMeshEvidenceSnapshot>;
    /// Plan a bounded static-mesh to voxel conversion through Rust authority.
    /// The request/response are generated protocol DTOs; no Studio-specific
    /// transport or renderer buffer shape crosses this boundary.
    fn plan_voxel_conversion(
        &mut self,
        request: VoxelConversionPlanRequest,
    ) -> BridgeResult<VoxelConversionPlan>;
    /// Register inline static-mesh geometry as an authority-visible conversion
    /// source. This is source ingestion only; voxelization still happens through
    /// plan/preview/apply authority operations.
    fn register_voxel_conversion_source(
        &mut self,
        request: VoxelConversionSourceRegistrationRequest,
    ) -> BridgeResult<VoxelConversionSourceRegistration>;
    /// Register a project/catalog static-mesh asset as an authority-visible
    /// conversion source. Rust validates/parses the mesh asset; callers provide
    /// selection and identity, not parsed conversion authority.
    fn register_voxel_conversion_mesh_asset(
        &mut self,
        request: VoxelConversionMeshAssetRegistrationRequest,
    ) -> BridgeResult<VoxelConversionSourceRegistration>;
    /// Read authority-owned metadata for a registered conversion source. Unknown
    /// sources return diagnostics instead of requiring Studio to infer catalog
    /// metadata from paths or private state.
    fn read_voxel_conversion_source_metadata(
        &self,
        request: VoxelConversionSourceMetadataRequest,
    ) -> BridgeResult<VoxelConversionSourceMetadataReadout>;
    /// Preview the most recently planned conversion, guarded by the plan hash.
    fn preview_voxel_conversion(
        &mut self,
        request: VoxelConversionPreviewRequest,
    ) -> BridgeResult<VoxelConversionPreview>;
    /// Apply the current conversion output into voxel authority via the existing
    /// generated voxel command path, guarded by plan/preview hashes.
    fn apply_voxel_conversion(
        &mut self,
        request: VoxelConversionApplyRequest,
    ) -> BridgeResult<VoxelConversionReceipt>;
    /// Export selected evidence refs from the current conversion authority state.
    fn export_voxel_conversion_evidence(
        &self,
        evidence: Vec<VoxelConversionEvidenceRef>,
    ) -> BridgeResult<Vec<VoxelConversionEvidenceRef>>;
    /// Read bounded authority-owned model information for an applied voxel
    /// conversion target. Missing/unknown models return typed diagnostics in the
    /// readout rather than exposing private state or raw JSON.
    fn read_voxel_model_info(
        &self,
        request: VoxelModelInfoRequest,
    ) -> BridgeResult<VoxelModelInfoReadout>;
    /// Read a bounded voxel-space window from an authority-owned model. The
    /// request is quota-guarded so agents can inspect cells/cross-sections
    /// without dumping full volumes or bypassing Rust authority.
    fn read_voxel_model_window(
        &self,
        request: VoxelModelWindowRequest,
    ) -> BridgeResult<VoxelModelWindowReadout>;
    /// Export a complete resident runtime voxel model as an Asha-native stored
    /// voxel-volume asset proposal. Rust owns the sparse-run representation,
    /// material palette validation, canonical JSON, and content hashes.
    fn export_voxel_volume_asset(
        &self,
        request: VoxelVolumeAssetExportRequest,
    ) -> BridgeResult<VoxelVolumeAssetExportReceipt>;
    /// Validate and package an explicit runtime-to-stored voxel asset save
    /// transaction. The bridge returns the stored diff and canonical payload; host
    /// code owns the actual file write after accepting the receipt.
    fn save_voxel_volume_asset(
        &self,
        request: VoxelVolumeAssetSaveRequest,
    ) -> BridgeResult<VoxelVolumeAssetSaveReceipt>;
    /// Validate and package a bounded stored-only material palette replacement.
    /// This operation returns a ProjectBundle diff and never mutates runtime state.
    fn update_voxel_volume_asset_palette(
        &self,
        request: VoxelVolumeAssetPaletteUpdateRequest,
    ) -> BridgeResult<VoxelVolumeAssetPaletteUpdateReceipt>;
    /// Load a validated stored voxel-volume asset into runtime authority through
    /// an explicit operation. Rejected assets leave runtime voxel state untouched.
    fn load_voxel_volume_asset(
        &mut self,
        request: VoxelVolumeAssetLoadRequest,
    ) -> BridgeResult<VoxelVolumeAssetLoadReceipt>;
    /// Validate and canonicalize a stored voxel annotation layer through Rust
    /// authority without mutating runtime state.
    fn validate_voxel_annotation_layer(
        &self,
        request: VoxelAnnotationLayerValidationRequest,
    ) -> BridgeResult<VoxelAnnotationLayerValidationReport>;
    /// Load a validated annotation layer into runtime annotation state. This
    /// attaches semantic metadata to a target voxel-volume asset id/hash; it does
    /// not mutate voxel occupancy.
    fn load_voxel_annotation_layer(
        &mut self,
        request: VoxelAnnotationLayerLoadRequest,
    ) -> BridgeResult<VoxelAnnotationLayerLoadReceipt>;
    /// Query a loaded annotation layer through bounded Rust-owned query helpers.
    fn read_voxel_annotation_query(
        &self,
        request: VoxelAnnotationQueryRequest,
    ) -> BridgeResult<VoxelAnnotationQueryReadout>;
    /// Apply a hash-guarded runtime annotation edit and revalidate before commit.
    fn apply_voxel_annotation_edit(
        &mut self,
        request: VoxelAnnotationEditRequest,
    ) -> BridgeResult<VoxelAnnotationEditReceipt>;
    /// Explicitly export a runtime annotation layer back to stored DTO form.
    fn export_voxel_annotation_layer(
        &self,
        request: VoxelAnnotationLayerExportRequest,
    ) -> BridgeResult<VoxelAnnotationLayerExportReceipt>;
    /// Read bounded voxel edit history/cursor authority for a loaded timeline.
    fn read_voxel_edit_history(
        &self,
        request: VoxelEditHistoryReadRequest,
    ) -> BridgeResult<VoxelEditHistorySummary>;
    /// Preview a revert target without mutating voxel authority.
    fn preview_voxel_edit_revert(
        &self,
        request: VoxelEditHistoryRevertRequest,
    ) -> BridgeResult<VoxelEditHistoryRevertReceipt>;
    /// Apply a guarded revert target through Rust-owned history authority.
    fn apply_voxel_edit_revert(
        &mut self,
        request: VoxelEditHistoryRevertRequest,
    ) -> BridgeResult<VoxelEditHistoryRevertReceipt>;
    /// Undo one retained voxel edit transaction through Rust-owned history.
    fn undo_voxel_edit(
        &mut self,
        request: VoxelEditHistoryUndoRequest,
    ) -> BridgeResult<VoxelEditHistoryUndoReceipt>;
    /// Redo one retained voxel edit transaction through Rust-owned history.
    fn redo_voxel_edit(
        &mut self,
        request: VoxelEditHistoryRedoRequest,
    ) -> BridgeResult<VoxelEditHistoryRedoReceipt>;
    /// Load an FPS/ECRP ProjectBundle-shaped session through Rust authority.
    /// Stored definitions are validated/bootstraped by rule-lifecycle and
    /// svc-entity-authoring; failure leaves any prior FPS session untouched.
    fn load_fps_runtime_session(
        &mut self,
        request: FpsRuntimeSessionLoadRequest,
    ) -> BridgeResult<FpsRuntimeSessionSnapshot>;
    /// Read typed FPS/ECRP RuntimeSession projection from Rust authority.
    fn read_fps_runtime_session(&self) -> BridgeResult<FpsRuntimeSessionSnapshot>;
    /// Submit a primary-fire intent. Rust owns combat, lifecycle, replay/hash,
    /// and render-visibility effects; callers receive projection evidence only.
    fn apply_fps_primary_fire(
        &mut self,
        request: FpsPrimaryFireRequest,
    ) -> BridgeResult<FpsPrimaryFireResult>;
    /// Invoke a declared game-owned Rust weapon-effect hook, validate its
    /// bounded proposal, and apply accepted output through FPS combat authority.
    fn invoke_game_extension_weapon_effect(
        &mut self,
        request: GameExtensionWeaponEffectInvocationRequest,
    ) -> BridgeResult<GameExtensionWeaponEffectInvocationResult>;
    /// Validate a generated generic game-rules catalog through Rust authority.
    /// This is a bounded semantic verb, not a raw rules/JSON dispatch surface.
    fn validate_game_rule_catalog(
        &mut self,
        catalog: GameRuleCatalog,
    ) -> BridgeResult<GameRuleCatalogValidationReceipt>;
    /// Submit one typed effect-resolution intent through the generic
    /// `svc-game-rules` substrate. Accepted modifier/readout evidence remains
    /// bridge-owned authority state until a later rule-event migration commits it
    /// into broader session state.
    fn submit_game_rule_effect_intent(
        &mut self,
        input: GameRuleEffectIntentRequest,
    ) -> BridgeResult<GameRuleResolutionReceipt>;
    /// Read bounded recent game-rules state/evidence without exposing raw state.
    fn read_game_rule_runtime_readout(&self) -> BridgeResult<GameRuleRuntimeReadout>;
    /// Restart the FPS/ECRP session by replaying the validated stored bundle into
    /// a fresh authority session, guarded by the caller's current epoch.
    fn restart_fps_runtime_session(
        &mut self,
        request: FpsRuntimeSessionRestartRequest,
    ) -> BridgeResult<FpsRuntimeSessionSnapshot>;
    /// Read the Rust-owned encounter/spawn director projection for the loaded
    /// FPS/ECRP RuntimeSession. Configuration is descriptive; transition state
    /// and hashes come from rule-lifecycle authority.
    fn read_fps_encounter_director(
        &self,
        lifecycle: FpsEncounterLifecycleInput,
    ) -> BridgeResult<FpsEncounterDirectorSnapshot>;
    /// Apply a Rust-owned encounter transition for the loaded FPS/ECRP session.
    fn apply_fps_encounter_transition(
        &mut self,
        request: FpsEncounterTransitionRequest,
    ) -> BridgeResult<FpsEncounterTransitionResult>;
    fn create_camera(&mut self, request: CameraCreateRequest) -> BridgeResult<CameraSnapshot>;
    fn apply_first_person_camera_input(
        &mut self,
        input: FirstPersonCameraInputEnvelope,
    ) -> BridgeResult<CameraSnapshot>;
    /// Apply a Rust-owned enemy direct-nav movement transaction. The operation
    /// combines the `svc-pathfinding` direct-nav proposal with `core-entity`
    /// transform authority so callers receive projection evidence instead of
    /// mutating runtime transforms themselves.
    fn apply_enemy_direct_nav_movement(
        &mut self,
        request: EnemyDirectNavMovementRequest,
    ) -> BridgeResult<EnemyDirectNavMovementResult>;
    fn read_camera_projection(
        &self,
        request: CameraProjectionRequest,
    ) -> BridgeResult<CameraProjectionSnapshot>;
    fn get_buffer(&self, handle: RuntimeBufferHandle) -> BridgeResult<RuntimeBufferView<'_>>;
    fn release_buffer(&mut self, handle: RuntimeBufferHandle) -> BridgeResult<()>;

    // ── ProjectBundle load/save composition (#2363) ──
    /// Load a ProjectBundle into authority. Fails closed (and leaves any prior
    /// RuntimeSession untouched) on an unsupported version.
    fn load_project_bundle(
        &mut self,
        request: ProjectBundleLoadRequest,
    ) -> BridgeResult<CompositionStatus>;
    /// Save the current ProjectBundle/session content. Fails closed with
    /// `NotInitialized` if none loaded.
    fn save_project_bundle(&mut self) -> BridgeResult<ProjectBundleSaveSummary>;
    /// Read composition status/diagnostics without mutating authority.
    fn get_project_bundle_composition_status(&self) -> BridgeResult<CompositionStatus>;
    /// Unload the staged/live ProjectBundle, returning to an empty runtime.
    fn unload_project_bundle(&mut self) -> BridgeResult<()>;
}
