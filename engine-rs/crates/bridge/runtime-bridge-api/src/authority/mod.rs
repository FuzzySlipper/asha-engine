use crate::*;

mod collision_camera;
mod enemy_navigation;
mod entity_appearance_projection;
mod fps_animation_catalog;
mod initialization;
mod input;
mod material_catalog;
mod procedural_environment;
mod project_write;
mod scene_and_preview;
mod time_control;
mod voxel_instances;
mod voxel_projection;
mod workspace_authoring;

// Product authority coordinator behind native transport marshaling.
//
// Domain mutation remains delegated to the owning rules and services. This type
// holds bridge-visible session state and coordinates typed RuntimeBridge verbs.

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BridgeCapabilityPortContract {
    pub id: &'static str,
    pub initialization: &'static str,
    pub runtime_project: &'static str,
    pub snapshot_hash: &'static str,
    pub resource_lifetime: &'static str,
}

#[cfg(test)]
pub(crate) const ENGINE_BRIDGE_CAPABILITY_PORTS: &[BridgeCapabilityPortContract] = &[
    BridgeCapabilityPortContract {
        id: "input",
        initialization: "requiresEngine",
        runtime_project: "retainedAcrossProjectChanges",
        snapshot_hash: "inputEvidence",
        resource_lifetime: "session",
    },
    BridgeCapabilityPortContract {
        id: "timeSimulation",
        initialization: "requiresEngine",
        runtime_project: "retainedAcrossProjectChanges",
        snapshot_hash: "timeState",
        resource_lifetime: "session",
    },
    BridgeCapabilityPortContract {
        id: "sceneEntities",
        initialization: "requiresEngine",
        runtime_project: "retainedAcrossProjectChanges",
        snapshot_hash: "sceneDocument",
        resource_lifetime: "session",
    },
    BridgeCapabilityPortContract {
        id: "voxelAssetsBuffers",
        initialization: "requiresEngine",
        runtime_project: "retainedAcrossProjectChanges",
        snapshot_hash: "voxelStateAndResources",
        resource_lifetime: "mixedExplicitAndSession",
    },
    BridgeCapabilityPortContract {
        id: "camera",
        initialization: "requiresEngine",
        runtime_project: "retainedAcrossProjectChanges",
        snapshot_hash: "cameraProjection",
        resource_lifetime: "session",
    },
    BridgeCapabilityPortContract {
        id: "gameplay",
        initialization: "requiresEngine",
        runtime_project: "retainedAcrossProjectChanges",
        snapshot_hash: "gameplaySessionAndReplay",
        resource_lifetime: "session",
    },
    BridgeCapabilityPortContract {
        id: "projection",
        initialization: "requiresEngine",
        runtime_project: "retainedAcrossProjectChanges",
        snapshot_hash: "projectionFrame",
        resource_lifetime: "frame",
    },
    BridgeCapabilityPortContract {
        id: "workspaceAuthoring",
        initialization: "createsEngine",
        runtime_project: "ownsProjectLifecycle",
        snapshot_hash: "workspaceAuthoringAuthority",
        resource_lifetime: "session",
    },
    BridgeCapabilityPortContract {
        id: "runtimeProjectLifecycle",
        initialization: "createsEngine",
        runtime_project: "ownsProjectLifecycle",
        snapshot_hash: "activeProjectContent",
        resource_lifetime: "session",
    },
    BridgeCapabilityPortContract {
        id: "replayEvidence",
        initialization: "requiresEngine",
        runtime_project: "retainedAcrossProjectChanges",
        snapshot_hash: "replayEvidence",
        resource_lifetime: "session",
    },
];

#[derive(Debug, Default)]
struct BridgeRuntimeProjectLifecycleState {
    engine: Option<EngineHandle>,
    runtime_project_generation: u64,
    runtime_project_revision: u64,
    active_runtime_project: Option<ActiveRuntimeProjectAuthority>,
    project_resource_staging: svc_serialization::ProjectResourceStaging,
    pending_project_source: Option<svc_serialization::AdmittedRuntimeProjectSourceBatch>,
    pending_gameplay_snapshot: Option<String>,
}

#[derive(Debug, Clone)]
struct ActiveRuntimeProjectAuthority {
    project_id: u64,
    manifest_hash: String,
    admission_hash: String,
    content_set_hash: String,
    composition_hash: String,
    entry_scene_id: u64,
    scene_count: u32,
    entity_count: u32,
    voxel_asset_count: u32,
    voxel_bindings: Vec<protocol_project_bundle::RuntimeProjectVoxelBinding>,
    source: svc_serialization::AdmittedRuntimeProjectSourceBatch,
}

#[derive(Debug, Default)]
struct BridgeInputState {
    input_session: Option<InputSessionResolver>,
}

#[derive(Debug, Default)]
struct BridgeTimeSimulationState {
    time_controller: TimeController,
    simulation: SimulationAuthority,
    authority_tick: u64,
}

#[derive(Debug, Default)]
struct BridgeSceneEntityState {
    scene_document: Option<core_scene::FlatSceneDocument>,
    entities: EntityStore,
}

#[derive(Debug, Default)]
struct BridgeVoxelAssetBufferState {
    buffers: buffer_provider::RuntimeBufferProvider,
    voxel: Option<VoxelWorld>,
    collision_world_offset: [f64; 3],
    voxel_edit_history: Option<rule_voxel_edit::history::VoxelEditHistory>,
    materials: MaterialCatalog,
    voxel_conversion_sources: BTreeMap<String, StaticMeshSource>,
    voxel_conversion_source_metadata: BTreeMap<String, VoxelConversionSourceMetadataAuthority>,
    voxel_conversion_targets: BTreeMap<(u64, Option<String>), VoxelConversionTargetAuthority>,
    voxel_conversion_plan: Option<PlannedConversion>,
    voxel_model_infos: BTreeMap<(u64, Option<String>), VoxelModelInfoAuthority>,
    active_voxel_model: Option<(u64, Option<String>)>,
    voxel_annotation_layers: BTreeMap<String, VoxelAnnotationLayer>,
}

#[derive(Debug, Default)]
struct BridgeCameraState {
    cameras: BTreeMap<u64, CameraSnapshot>,
    camera_controllers: BTreeMap<u64, CameraControllerState>,
    next_camera: u64,
}

#[derive(Default)]
struct BridgeGameplayState {
    fps_session: Option<FpsRuntimeSessionState>,
    fps_seed: Option<FpsProjectBundleLoadInput>,
    fps_epoch: u64,
    static_gameplay_host: Option<gameplay_runtime_host::GameplayRuntimeHost>,
    static_gameplay_composition: Option<gameplay_runtime_host::GameplayStaticComposition>,
    static_project_domain_adapter: Option<RuntimeProjectDomainAdapter>,
    static_project_content_admission: Option<rule_project_bundle::GameplayProjectContentAdmission>,
    static_gameplay_reset_checkpoint: Option<gameplay_runtime_host::GameplayRuntimeResetCheckpoint>,
    static_gameplay_base_entities: Option<EntityStore>,
    game_rule_modules: BTreeMap<String, GameRuleModuleManifest>,
    game_rule_active_modifiers: Vec<GameRuleModifierState>,
    game_rule_recent_trace: Vec<GameRuleTraceEntry>,
}

#[derive(Debug, Default)]
struct BridgeProjectionState {
    projection_frame: Option<RuntimeProjectionFrame>,
    entity_appearances:
        BTreeMap<EntityId, entity_appearance_projection::EntityAppearanceProjectionSeed>,
    entity_appearance_handles: BTreeMap<EntityId, protocol_render::RenderHandle>,
    entity_appearance_defined_assets: BTreeSet<String>,
    voxel_projector: VoxelChunkProjector,
    pending_voxel_frame: RenderFrameDiff,
    voxel_instance_binding: Option<VoxelInstanceBindingAuthority>,
    audio_projector: Option<AudioProjector>,
    billboard_projector: Option<BillboardProjector>,
    particle_projector: Option<ParticleProjector>,
    presentation_catalog: presentation_catalog::InstalledPresentationCatalog,
    animation_controller: Option<rule_animation_controller::AnimationControllerAuthority>,
    animation_projector: Option<render_animation::AnimationControllerProjector>,
    animation_tick: u64,
    telemetry_overlay_projector: Option<TelemetryOverlayProjector>,
    voxel_update_telemetry: VoxelUpdateTelemetryState,
}

#[derive(Debug, Default)]
struct VoxelUpdateTelemetryState {
    pending_committed_command_batches: u64,
    pending_accepted_commands: u64,
    pending_touched_voxels: u64,
    latest: Option<VoxelUpdateTelemetryReadout>,
}

#[derive(Debug, Clone)]
struct VoxelInstanceBindingAuthority {
    workspace_id: String,
    workspace_generation: u64,
    working_revision: u64,
    registry_digest: String,
    binding_hash: String,
    world_hash: u64,
    instances: BTreeMap<String, VoxelProjectionInstanceBinding>,
}

#[derive(Debug, Default)]
struct BridgeReplayEvidenceState {
    game_rule_recent_replay_hashes: Vec<String>,
    voxel_conversion_evidence: Vec<VoxelConversionEvidenceRef>,
}

#[derive(Debug, Default)]
struct BridgeDeveloperConsoleState {
    records: Vec<DeveloperConsoleRecord>,
    dropped_record_count: u64,
    next_sequence: u64,
    admitted_tick: Option<u64>,
    admitted_count: usize,
}

#[derive(Debug, Clone)]
struct WorkspaceAuthoringSaveCandidate {
    canonical_json_hash: String,
    working_revision: u64,
}

struct PendingProjectWriteCandidate {
    candidate_hash: String,
    working_revision: u64,
    authorized: svc_serialization::AuthorizedProjectWriteCandidate,
}

#[derive(Debug, Clone)]
struct PendingProceduralEnvironmentCandidate {
    candidate_hash: String,
    base_scene_hash: String,
    working_revision: u64,
    scene_path: String,
    asset_path: String,
    voxel_node_id: SceneNodeId,
    materialized: svc_environment_authoring::MaterializedEnvironment,
}

struct WorkspaceAuthoringAuthority {
    identity: WorkspaceAuthoringIdentity,
    composition: WorkspaceAuthoringCompositionStatus,
    open: bool,
    working_revision: u64,
    stored_revision: u64,
    last_stored_canonical_json_hash: Option<String>,
    pending_save_candidate: Option<WorkspaceAuthoringSaveCandidate>,
    pending_project_write: Option<PendingProjectWriteCandidate>,
    pending_procedural_environment: Option<PendingProceduralEnvironmentCandidate>,
    next_projection_cursor: u64,
    projection_initialized: bool,
    last_projection_receipt: Option<WorkspaceAuthoringProjectionReceipt>,
    loaded_voxel_assets: BTreeMap<String, VoxelVolumeAsset>,
    project_write_voxel_assets: BTreeMap<String, VoxelVolumeAsset>,
    project_write_generation_provenance: Option<svc_serialization::GeneratorMetadata>,
    project_content_scenes: BTreeMap<u64, FlatSceneDocumentDto>,
    project_content_reference_revision: u64,
    project_content_current: Option<svc_project_content::ValidatedProjectContentSet>,
    project_content_admission: rule_project_bundle::GameplayProjectContentAdmission,
}

pub(crate) struct DeveloperConsoleEmission {
    pub severity: DiagnosticSeverity,
    pub category: DeveloperConsoleCategory,
    pub source: DeveloperConsoleSource,
    pub message: String,
    pub correlation: Option<String>,
    pub authority_tick: Option<u64>,
    pub detail: DeveloperConsoleDetail,
}

/// Engine-owned RuntimeBridge authority state. Large payloads are owned by the
/// [`RuntimeBufferProvider`]; the seed buffer is allocated as the first handle
/// (`0`) so buffer verbs exercise the real provider rather than a bespoke `Vec`.
#[derive(Default)]
pub struct EngineBridge {
    runtime_project: BridgeRuntimeProjectLifecycleState,
    input: BridgeInputState,
    time: BridgeTimeSimulationState,
    scene: BridgeSceneEntityState,
    voxel: BridgeVoxelAssetBufferState,
    camera: BridgeCameraState,
    gameplay: BridgeGameplayState,
    projection: BridgeProjectionState,
    evidence: BridgeReplayEvidenceState,
    developer_console: RefCell<BridgeDeveloperConsoleState>,
    workspace_authoring: Option<WorkspaceAuthoringAuthority>,
    workspace_authoring_epoch: u64,
}

/// The bundle schema and protocol versions this engine bridge understands.
const ENGINE_SUPPORTED_BUNDLE_VERSION: u32 = svc_serialization::BUNDLE_SCHEMA_VERSION;
const ENGINE_SUPPORTED_PROTOCOL_VERSION: u32 = svc_serialization::SUPPORTED_PROTOCOL_VERSION;
const BUILT_IN_GAME_RULE_MODULE_ID: &str = "asha.engine.primary_fire_damage_modifier";
const BUILT_IN_GAME_RULE_HOOK_ID: &str = "weapon.primary.damage_modifier";
const WEAPON_EFFECT_INPUT_CONTRACT: &str = "WeaponEffectHookRequest.v0";
const GAME_EXTENSION_PROPOSAL_CONTRACT: &str = "GameExtensionProposal.v0";
const GAME_RULE_DETERMINISTIC_REQUIREMENTS: &[&str] = &[
    "no-wall-clock",
    "no-ambient-random",
    "no-filesystem",
    "no-network",
    "no-ts-callback",
];
const VOXEL_MODEL_WINDOW_MAX_SAMPLES: u64 = 4096;

impl EngineBridge {
    pub(crate) fn reset_developer_console(&self) {
        *self.developer_console.borrow_mut() = BridgeDeveloperConsoleState::default();
    }

    pub(crate) fn record_developer_console(&self, emission: DeveloperConsoleEmission) {
        let mut state = self.developer_console.borrow_mut();
        if state.admitted_tick != emission.authority_tick {
            state.admitted_tick = emission.authority_tick;
            state.admitted_count = 0;
        }
        if state.admitted_count >= DEVELOPER_CONSOLE_MAX_RECORDS_PER_TICK {
            state.dropped_record_count = state.dropped_record_count.saturating_add(1);
            return;
        }
        state.admitted_count += 1;

        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.saturating_add(1);
        let session = self
            .runtime_project
            .engine
            .map(|handle| format!("engine:{}", handle.raw()));
        state.records.push(DeveloperConsoleRecord {
            sequence,
            severity: emission.severity,
            category: emission.category,
            source: emission.source,
            message: Self::bounded_console_text(&emission.message, 512),
            correlation: emission
                .correlation
                .map(|value| Self::bounded_console_text(&value, 160)),
            authority_tick: emission.authority_tick,
            session,
            detail: DeveloperConsoleDetail {
                code: Self::bounded_console_text(&emission.detail.code, 96),
                operation: emission
                    .detail
                    .operation
                    .map(|value| Self::bounded_console_text(&value, 96)),
                resource_kind: emission
                    .detail
                    .resource_kind
                    .map(|value| Self::bounded_console_text(&value, 96)),
                resource_id: emission
                    .detail
                    .resource_id
                    .map(|value| Self::bounded_console_text(&value, 160)),
                reason: emission
                    .detail
                    .reason
                    .map(|value| Self::bounded_console_text(&value, 256)),
            },
        });
        if state.records.len() > DEVELOPER_CONSOLE_MAX_RECORDS {
            state.records.remove(0);
            state.dropped_record_count = state.dropped_record_count.saturating_add(1);
        }
    }

    pub(crate) fn developer_console_snapshot(&self) -> DeveloperConsoleSnapshot {
        let state = self.developer_console.borrow();
        let first_sequence = state.records.first().map(|record| record.sequence);
        let canonical = serde_json::to_string(&(
            DEVELOPER_CONSOLE_SCHEMA_VERSION,
            &state.records,
            state.dropped_record_count,
            first_sequence,
            state.next_sequence,
        ))
        .expect("developer console contract is serializable");
        DeveloperConsoleSnapshot {
            schema_version: DEVELOPER_CONSOLE_SCHEMA_VERSION,
            records: state.records.clone(),
            dropped_record_count: state.dropped_record_count,
            first_sequence,
            next_sequence: state.next_sequence,
            snapshot_hash: format!("fnv1a64:{}", Self::fnv1a64(&canonical)),
        }
    }

    fn bounded_console_text(value: &str, max_chars: usize) -> String {
        value.chars().take(max_chars).collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
struct VoxelConversionTargetAuthority {
    spec: VoxelGridSpec,
    volume_asset_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct VoxelConversionSourceMetadataAuthority {
    source: protocol_voxel_conversion::VoxelConversionSourceRef,
    source_path: Option<String>,
    source_bounds: Option<VoxelConversionSourceBounds>,
    vertex_count: u32,
    triangle_count: u32,
    groups: Vec<VoxelConversionSourceGroupMetadata>,
    material_slots: Vec<VoxelConversionSourceMaterialSlot>,
    evidence: Vec<VoxelConversionEvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VoxelModelInfoAuthority {
    model_id: String,
    volume_asset_id: Option<String>,
    grid: u64,
    bounds: Option<protocol_voxel_conversion::VoxelConversionBounds>,
    voxel_count: u64,
    material_counts: Vec<VoxelModelMaterialCount>,
    source: protocol_voxel_conversion::VoxelConversionSourceRef,
    latest_plan_id: String,
    latest_output_hash: String,
    session_hash: String,
    replay_hash: String,
    evidence: Vec<VoxelConversionEvidenceRef>,
    authoring_edit_count: u64,
    material_palette: Vec<VoxelAssetMaterialBinding>,
    authoring: VoxelAssetAuthoringMetadata,
    resident_voxels: BTreeMap<VoxelCoord, VoxelValue>,
    prior_voxels: BTreeMap<VoxelCoord, VoxelValue>,
}

struct BuiltInDamageModifierModule {
    manifest: GameRuleModuleManifest,
}

impl BuiltInDamageModifierModule {
    fn new(module_ref: GameRuleModuleRef) -> Self {
        Self {
            manifest: built_in_game_rule_manifest(module_ref),
        }
    }
}

impl GameRuleModule for BuiltInDamageModifierModule {
    fn manifest(&self) -> &GameRuleModuleManifest {
        &self.manifest
    }

    fn evaluate_weapon_effect(
        &self,
        request: &WeaponEffectHookRequest,
    ) -> GameRuleExtensionResult<GameExtensionProposal> {
        if request.hook_id != BUILT_IN_GAME_RULE_HOOK_ID {
            return Err(unsupported_hook_diagnostic(
                &request.hook_id,
                "built-in game rule module only implements primary-fire damage modifier",
            ));
        }
        let Some(target) = request.target else {
            return Ok(GameExtensionProposal::Reject {
                proposal_id: format!("{}.reject_no_target", request.request_id),
                code: GameExtensionDiagnosticCode::InvalidProposal,
                message: "weapon effect requires a target entity".to_string(),
                proposal_hash: "fnv1a64:no-target".to_string(),
            });
        };
        Ok(GameExtensionProposal::DamageModifier {
            proposal_id: format!("{}.damage_bonus", request.request_id),
            target,
            channel_id: "combat.primary_fire.damage".to_string(),
            amount_delta: 5,
            tags: vec!["engine-rust-module".to_string()],
            proposal_hash: format!(
                "fnv1a64:{}",
                EngineBridge::fnv1a64(&format!(
                    "{}|{}|{}|{}",
                    request.request_id,
                    target.raw(),
                    request.base_damage,
                    request.input_hash
                ))
            ),
        })
    }
}

struct RegisteredDamageModifierModule {
    manifest: GameRuleModuleManifest,
}

impl RegisteredDamageModifierModule {
    fn new(manifest: GameRuleModuleManifest) -> Self {
        Self { manifest }
    }
}

impl GameRuleModule for RegisteredDamageModifierModule {
    fn manifest(&self) -> &GameRuleModuleManifest {
        &self.manifest
    }

    fn evaluate_weapon_effect(
        &self,
        request: &WeaponEffectHookRequest,
    ) -> GameRuleExtensionResult<GameExtensionProposal> {
        if !self.manifest.declared_hooks.iter().any(|hook| {
            hook.hook_id == request.hook_id && hook.kind == GameExtensionHookKind::WeaponEffect
        }) {
            return Err(unsupported_hook_diagnostic(
                &request.hook_id,
                "registered game rule module did not declare the requested weapon-effect hook",
            ));
        }
        let Some(target) = request.target else {
            return Ok(GameExtensionProposal::Reject {
                proposal_id: format!("{}.reject_no_target", request.request_id),
                code: GameExtensionDiagnosticCode::InvalidProposal,
                message: "weapon effect requires a target entity".to_string(),
                proposal_hash: format!(
                    "fnv1a64:{}",
                    EngineBridge::fnv1a64(&format!(
                        "{}|{}|no-target",
                        self.manifest.module_ref.module_id, request.request_id
                    ))
                ),
            });
        };
        Ok(GameExtensionProposal::DamageModifier {
            proposal_id: format!("{}.registered_damage_bonus", request.request_id),
            target,
            channel_id: "combat.primary_fire.damage".to_string(),
            amount_delta: 5,
            tags: vec![
                "registered-rust-module".to_string(),
                self.manifest.module_ref.module_id.clone(),
            ],
            proposal_hash: format!(
                "fnv1a64:{}",
                EngineBridge::fnv1a64(&format!(
                    "{}|{}|{}|{}|{}|{}|{}",
                    self.manifest.module_ref.module_id,
                    self.manifest.module_ref.version,
                    self.manifest.module_ref.contract_hash,
                    request.request_id,
                    target.raw(),
                    request.base_damage,
                    request.input_hash
                ))
            ),
        })
    }
}

enum ResolvedGameRuleModule {
    BuiltIn(BuiltInDamageModifierModule),
    Registered(RegisteredDamageModifierModule),
}

impl ResolvedGameRuleModule {
    fn manifest(&self) -> &GameRuleModuleManifest {
        match self {
            Self::BuiltIn(module) => module.manifest(),
            Self::Registered(module) => module.manifest(),
        }
    }

    fn evaluate_weapon_effect(
        &self,
        request: &WeaponEffectHookRequest,
    ) -> GameRuleExtensionResult<GameExtensionProposal> {
        match self {
            Self::BuiltIn(module) => module.evaluate_weapon_effect(request),
            Self::Registered(module) => module.evaluate_weapon_effect(request),
        }
    }
}

impl GameRuleModule for ResolvedGameRuleModule {
    fn manifest(&self) -> &GameRuleModuleManifest {
        ResolvedGameRuleModule::manifest(self)
    }

    fn evaluate_weapon_effect(
        &self,
        request: &WeaponEffectHookRequest,
    ) -> GameRuleExtensionResult<GameExtensionProposal> {
        ResolvedGameRuleModule::evaluate_weapon_effect(self, request)
    }
}

fn built_in_game_rule_manifest(module_ref: GameRuleModuleRef) -> GameRuleModuleManifest {
    GameRuleModuleManifest {
        module_ref,
        declared_hooks: vec![GameRuleHookDeclaration {
            hook_id: BUILT_IN_GAME_RULE_HOOK_ID.to_string(),
            kind: GameExtensionHookKind::WeaponEffect,
            input_contract: WEAPON_EFFECT_INPUT_CONTRACT.to_string(),
            output_contract: GAME_EXTENSION_PROPOSAL_CONTRACT.to_string(),
            required_capabilities: vec!["health".to_string(), "weaponMount".to_string()],
        }],
        deterministic_requirements: GAME_RULE_DETERMINISTIC_REQUIREMENTS
            .iter()
            .map(|requirement| (*requirement).to_string())
            .collect(),
        source_hash: "sha256:asha-engine-primary-fire-module-source".to_string(),
    }
}

mod camera;
mod composition;
mod fps_and_rules;
mod fps_project_diagnostics;
mod fps_runtime_session;
mod presentation_catalog;
mod project_and_sources;
mod runtime_bridge_impl;
mod runtime_project_public;
mod voxel_annotations;
mod voxel_assets;
mod voxel_authoring;
mod voxel_history;
mod voxel_palette_limits;

pub use composition::{
    ComposedRuntimeSessionReadout, DeferredRuntimeSessionBuilder, RuntimeProjectDomainAdapter,
    StaticProjectAuthoringBuilder,
};
pub use project_and_sources::{
    RuntimeProjectActivationReceipt, RuntimeProjectLifecycleVersion, RuntimeProjectLoadError,
    RuntimeProjectUnloadReceipt,
};

#[cfg(test)]
pub(super) mod tests;
#[cfg(test)]
mod voxel_history_tests;
#[cfg(test)]
mod workspace_authoring_tests;
