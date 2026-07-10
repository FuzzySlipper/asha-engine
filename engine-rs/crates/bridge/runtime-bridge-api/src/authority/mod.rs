use crate::*;

// Product authority coordinator behind native transport marshaling.
//
// Domain mutation remains delegated to the owning rules and services. This type
// holds bridge-visible session state and coordinates typed RuntimeBridge verbs.

/// Engine-owned RuntimeBridge authority state. Large payloads are owned by the
/// [`RuntimeBufferProvider`]; the seed buffer is allocated as the first handle
/// (`0`) so buffer verbs exercise the real provider rather than a bespoke `Vec`.
#[derive(Debug, Default)]
pub struct EngineBridge {
    engine: Option<EngineHandle>,
    buffers: buffer_provider::RuntimeBufferProvider,
    /// The currently-loaded ProjectBundle scene identity.
    loaded_project_bundle: Option<u64>,
    /// Live voxel authority for the launch/edit loop (launchable-voxel, #2436).
    /// Present once `initialize_engine` has set up the runtime.
    voxel: Option<VoxelWorld>,
    /// Translation from canonical voxel coordinates into the active runtime
    /// room frame. Generic voxel worlds use zero; generated centered rooms set it.
    collision_world_offset: [f64; 3],
    /// Rust-owned accepted voxel transaction timeline for the live voxel authority.
    voxel_edit_history: Option<rule_voxel_edit::history::VoxelEditHistory>,
    /// The material catalog voxel edits validate against.
    materials: MaterialCatalog,
    /// Bridge-owned runtime view cameras (view/projection evidence, not gameplay authority).
    cameras: BTreeMap<u64, CameraSnapshot>,
    next_camera: u64,
    /// Minimal authority-owned runtime entity state for bridge-level actor
    /// movement verbs. TypeScript may propose targets, but transform mutation is
    /// applied here through `core-entity`.
    entities: EntityStore,
    /// FPS/ECRP RuntimeSession authority state. Stored definitions seed this
    /// through rule-lifecycle; TS callers only receive typed readouts/receipts.
    fps_session: Option<FpsRuntimeSessionState>,
    fps_seed: Option<FpsRuntimeSessionLoadRequest>,
    fps_epoch: u64,
    game_rule_modules: BTreeMap<String, GameRuleModuleManifest>,
    game_rule_active_modifiers: Vec<GameRuleModifierState>,
    game_rule_recent_trace: Vec<GameRuleTraceEntry>,
    game_rule_recent_replay_hashes: Vec<String>,
    /// Last planned voxel conversion. This is bridge-owned authority state used
    /// by preview/apply hash guards; callers cannot provide their own output.
    voxel_conversion_sources: BTreeMap<String, StaticMeshSource>,
    voxel_conversion_source_metadata: BTreeMap<String, VoxelConversionSourceMetadataAuthority>,
    voxel_conversion_targets: BTreeMap<(u64, Option<String>), VoxelConversionTargetAuthority>,
    voxel_conversion_plan: Option<PlannedConversion>,
    voxel_conversion_evidence: Vec<VoxelConversionEvidenceRef>,
    voxel_model_infos: BTreeMap<(u64, Option<String>), VoxelModelInfoAuthority>,
    voxel_annotation_layers: BTreeMap<String, VoxelAnnotationLayer>,
}

/// The bundle schema and protocol versions this engine bridge understands.
const ENGINE_SUPPORTED_VERSION: u32 = 1;
const BUILT_IN_GAME_RULE_MODULE_ID: &str = "asha.engine.primary_fire_damage_modifier";
#[cfg(test)]
const BUILT_IN_GAME_RULE_MODULE_VERSION: &str = "0.1.0";
#[cfg(test)]
const BUILT_IN_GAME_RULE_CONTRACT_HASH: &str = "sha256:asha-engine-primary-fire-damage-modifier-v0";
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

#[cfg(test)]
fn built_in_game_rule_module_ref() -> GameRuleModuleRef {
    GameRuleModuleRef {
        module_id: BUILT_IN_GAME_RULE_MODULE_ID.to_string(),
        version: BUILT_IN_GAME_RULE_MODULE_VERSION.to_string(),
        contract_hash: BUILT_IN_GAME_RULE_CONTRACT_HASH.to_string(),
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

#[cfg(test)]
fn built_in_game_rule_declared_manifest() -> GameRuleModuleManifest {
    built_in_game_rule_manifest(built_in_game_rule_module_ref())
}

mod camera;
mod fps_and_rules;
mod project_and_sources;
mod runtime_bridge_impl;
mod voxel_annotations;
mod voxel_assets;
mod voxel_history;
mod voxel_palette_limits;

#[cfg(test)]
mod game_extension_tests;
#[cfg(test)]
pub(super) mod tests;
#[cfg(test)]
mod voxel_history_tests;
