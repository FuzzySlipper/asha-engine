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

    // ── World load/save composition (#2363) ──
    /// Load a world bundle into authority. Fails closed (and leaves any prior
    /// world untouched) on an unsupported version.
    fn load_world_bundle(&mut self, request: WorldLoadRequest) -> BridgeResult<CompositionStatus>;
    /// Save the current world. Fails closed with `NotInitialized` if none loaded.
    fn save_current_world(&mut self) -> BridgeResult<WorldSaveSummary>;
    /// Read composition status/diagnostics without mutating authority.
    fn get_composition_status(&self) -> BridgeResult<CompositionStatus>;
    /// Unload the staged/live world, returning to an empty runtime.
    fn unload_world(&mut self) -> BridgeResult<()>;
}
