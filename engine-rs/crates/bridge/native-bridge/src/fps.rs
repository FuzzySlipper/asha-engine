use napi_derive::napi;
use protocol_game_extension::{
    GameplayCompositionDiagnostic, GameplayCompositionDiagnosticCode, GameplayCompositionLoadMode,
};
use runtime_bridge_api::{
    ComposedRuntimeSessionReadout, FpsBridgeRole, FpsEncounterDirectorSnapshot,
    FpsEncounterLifecycleInput, FpsEncounterStateReadout, FpsEncounterTransitionRequest,
    FpsEncounterTransitionResult, FpsPrimaryFireRequest, FpsPrimaryFireResult,
    FpsRuntimeSessionRestartRequest, FpsRuntimeSessionSnapshot,
    GameExtensionWeaponEffectInvocationRequest, GameRuleEffectIntentRequest, GameplayContractRef,
    GameplayModuleViewRequest, GameplayModuleViewScope, GameplayModuleViewSnapshot,
    GameplayPrefabPartInteractionReceipt, GameplayPrefabPartInteractionRequest, RuntimeBridge,
    RuntimeBridgeError, RuntimeBridgeErrorKind,
};

use crate::{
    game_extension_json, game_rule_json, parse_game_rule_catalog,
    parse_game_rule_resolution_request, parse_weapon_effect_hook_request, to_napi, u32_input,
    u64_input, with_bridge, NativeVec3,
};

#[napi(object)]
pub struct NativeFpsHealth {
    pub current: u32,
    pub max: u32,
}


#[napi(object)]
pub struct NativeFpsLifecycleStatus {
    pub state: String,
    pub entity: Option<i64>,
    pub tick: Option<i64>,
}

#[napi(object)]
pub struct NativeFpsEntityHealthReadout {
    pub entity: i64,
    pub current: u32,
    pub max: u32,
}

#[napi(object)]
pub struct NativeFpsPolicyBindingReadout {
    pub entity: i64,
    pub binding_id: String,
    pub policy_id: String,
    pub view_kind: String,
    pub view_version: String,
    pub allowed_intents: Vec<String>,
    pub runtime_moment: String,
}

#[napi(object)]
pub struct NativeFpsReplayEvidence {
    pub replay_unit: String,
    pub entity_hash: String,
    pub health_hash: String,
    pub record_hash: String,
}

#[napi(object)]
pub struct NativeFpsReadSetEvidence {
    pub view_kind: String,
    pub owner: String,
    pub read_set: Vec<String>,
}

#[napi(object)]
pub struct NativeFpsRuntimeSessionSnapshot {
    pub backend: String,
    pub authority_surface: String,
    pub project_bundle: String,
    pub session_epoch: i64,
    pub lifecycle_status: NativeFpsLifecycleStatus,
    pub player_entity: i64,
    pub enemy_entity: i64,
    pub health: Vec<NativeFpsEntityHealthReadout>,
    pub policy_bindings: Vec<NativeFpsPolicyBindingReadout>,
    pub replay_records: Vec<NativeFpsReplayEvidence>,
    pub read_sets: Vec<NativeFpsReadSetEvidence>,
    pub entity_hash: String,
    pub health_hash: String,
    pub replay_hash: String,
}

#[napi(object)]
pub struct NativeFpsPrimaryFireResult {
    pub backend: String,
    pub authority_surface: String,
    pub mutation_owner: String,
    pub workspace_trace: Vec<String>,
    pub shooter: i64,
    pub target: Option<i64>,
    pub target_health_before: Option<NativeFpsHealth>,
    pub target_health_after: Option<NativeFpsHealth>,
    pub lifecycle_status: NativeFpsLifecycleStatus,
    pub target_render_visible: Option<bool>,
    pub entity_hash: String,
    pub health_hash: String,
    pub replay_hash: String,
}

#[napi(object)]
pub struct NativeGameplayCompositionDiagnostic {
    pub code: String,
    pub severity: String,
    pub path: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub message: String,
}

#[napi(object)]
pub struct NativeComposedGameplayReadout {
    pub gameplay_registry_digest: String,
    pub semantic_compatibility_digest: String,
    pub artifact_provenance_digest: String,
    pub composition_load_mode: String,
    pub compatibility_diagnostics: Vec<NativeGameplayCompositionDiagnostic>,
    pub binding_registry_hash: String,
    pub activation_hash: String,
    pub module_state_hash: String,
    pub authority_state_hash: String,
    pub trigger_revision: i64,
    pub trigger_snapshot_hash: String,
    pub active_overlap_count: u32,
    pub reaction_frame_count: u32,
    pub last_reaction_frame_hash: Option<String>,
    pub decision_receipt_count: u32,
    pub pending_decision_count: u32,
    pub last_decision_receipt_hash: Option<String>,
    pub scheduler_state_hash: String,
    pub scheduler_pending_action_count: u32,
    pub scheduler_outstanding_dispatch_count: u32,
    pub scheduler_outstanding_event_delivery_count: u32,
    pub scheduler_fact_count: u32,
    pub scheduler_truncated: bool,
    pub runtime_host_hash: String,
}

#[napi(object)]
pub struct NativeComposedRuntimeSessionReadout {
    pub schema_version: u32,
    pub entity_authority_hash: String,
    pub gameplay: NativeComposedGameplayReadout,
    pub fps_session_epoch: i64,
    pub fps_replay_hash: Option<String>,
    pub runtime_session_hash: String,
}

#[napi(object)]
pub struct NativeGameplayContractRef {
    pub namespace: String,
    pub name: String,
    pub version: u32,
    pub schema_hash: String,
}

#[napi(object)]
pub struct NativeGameplayModuleViewSnapshot {
    pub view: NativeGameplayContractRef,
    pub provider_id: String,
    pub scope_kind: String,
    pub scope_value: Option<i64>,
    pub revision: i64,
    pub canonical_payload: Vec<u8>,
    pub view_hash: String,
    pub runtime_session_hash: String,
}

#[napi(object)]
pub struct NativeGameplayPrefabPartInteractionReceipt {
    pub actor: i64,
    pub instance: i64,
    pub role: String,
    pub target: i64,
    pub event_hash: String,
    pub reaction_frame_hash: String,
    pub runtime_session_hash: String,
}

#[napi(object)]
pub struct NativeGameExtensionWeaponEffectInvocationResult {
    pub hook_receipt_json: String,
    pub replay_evidence_json: String,
    pub primary_fire: Option<NativeFpsPrimaryFireResult>,
}

#[napi(object)]
pub struct NativeFpsEncounterLifecycleInput {
    pub outcome_kind: String,
    pub terminal: bool,
    pub enemy_dead: bool,
    pub player_dead: bool,
    pub lifecycle_hash: String,
}

#[napi(object)]
pub struct NativeFpsEncounterTransitionRequest {
    pub preset_id: String,
    pub action: String,
    pub lifecycle: NativeFpsEncounterLifecycleInput,
}

#[napi(object)]
pub struct NativeFpsEncounterStateReadout {
    pub preset_id: String,
    pub status: String,
    pub spawned_enemy_ids: Vec<String>,
    pub defeated_enemy_ids: Vec<String>,
    pub revision: i64,
    pub last_transition: String,
}

#[napi(object)]
pub struct NativeFpsEncounterDirectorSnapshot {
    pub backend: String,
    pub authority_surface: String,
    pub mutation_owner: String,
    pub workspace_trace: Vec<String>,
    pub state: NativeFpsEncounterStateReadout,
    pub lifecycle: NativeFpsEncounterLifecycleInput,
    pub read_sets: Vec<NativeFpsReadSetEvidence>,
    pub encounter_hash: String,
    pub replay_hash: String,
}

#[napi(object)]
pub struct NativeFpsEncounterTransitionResult {
    pub backend: String,
    pub authority_surface: String,
    pub mutation_owner: String,
    pub workspace_trace: Vec<String>,
    pub accepted: bool,
    pub rejection_reason: Option<String>,
    pub event_kind: Option<String>,
    pub state: NativeFpsEncounterStateReadout,
    pub lifecycle: NativeFpsEncounterLifecycleInput,
    pub encounter_hash: String,
    pub replay_hash: String,
}

fn native_hash(value: u64) -> String {
    format!("fnv1a64:{value:016x}")
}

fn native_fps_role(value: &str) -> napi::Result<FpsBridgeRole> {
    match value {
        "player" => Ok(FpsBridgeRole::Player),
        "enemy" => Ok(FpsBridgeRole::Enemy),
        "neutral" => Ok(FpsBridgeRole::Neutral),
        other => Err(to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("unknown FPS role '{other}'"),
        ))),
    }
}

fn optional_native_fps_role(
    value: Option<String>,
    field: &str,
) -> napi::Result<Option<FpsBridgeRole>> {
    match value {
        Some(role) => native_fps_role(role.as_str()).map(Some).map_err(|_| {
            to_napi(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("{field} must be player, enemy, or neutral"),
            ))
        }),
        None => Ok(None),
    }
}

fn native_fps_lifecycle_status(
    value: runtime_bridge_api::FpsBridgeLifecycleStatus,
) -> NativeFpsLifecycleStatus {
    match value {
        runtime_bridge_api::FpsBridgeLifecycleStatus::Active => NativeFpsLifecycleStatus {
            state: "active".into(),
            entity: None,
            tick: None,
        },
        runtime_bridge_api::FpsBridgeLifecycleStatus::EnemyDefeated { entity, tick } => {
            NativeFpsLifecycleStatus {
                state: "enemy_defeated".into(),
                entity: Some(entity as i64),
                tick: Some(tick as i64),
            }
        }
    }
}

impl From<FpsRuntimeSessionSnapshot> for NativeFpsRuntimeSessionSnapshot {
    fn from(value: FpsRuntimeSessionSnapshot) -> Self {
        Self {
            backend: value.backend,
            authority_surface: value.authority_surface,
            project_bundle: value.project_bundle,
            session_epoch: value.session_epoch as i64,
            lifecycle_status: native_fps_lifecycle_status(value.lifecycle_status),
            player_entity: value.player_entity as i64,
            enemy_entity: value.enemy_entity as i64,
            health: value
                .health
                .into_iter()
                .map(|health| NativeFpsEntityHealthReadout {
                    entity: health.entity as i64,
                    current: health.current,
                    max: health.max,
                })
                .collect(),
            policy_bindings: value
                .policy_bindings
                .into_iter()
                .map(|binding| NativeFpsPolicyBindingReadout {
                    entity: binding.entity as i64,
                    binding_id: binding.binding_id,
                    policy_id: binding.policy_id,
                    view_kind: binding.view_kind,
                    view_version: binding.view_version,
                    allowed_intents: binding.allowed_intents,
                    runtime_moment: binding.runtime_moment,
                })
                .collect(),
            replay_records: value
                .replay_records
                .into_iter()
                .map(|record| NativeFpsReplayEvidence {
                    replay_unit: record.replay_unit,
                    entity_hash: native_hash(record.entity_hash),
                    health_hash: native_hash(record.health_hash),
                    record_hash: native_hash(record.record_hash),
                })
                .collect(),
            read_sets: value
                .read_sets
                .into_iter()
                .map(|read_set| NativeFpsReadSetEvidence {
                    view_kind: read_set.view_kind,
                    owner: read_set.owner,
                    read_set: read_set.read_set,
                })
                .collect(),
            entity_hash: native_hash(value.entity_hash),
            health_hash: native_hash(value.health_hash),
            replay_hash: native_hash(value.replay_hash),
        }
    }
}

impl From<FpsPrimaryFireResult> for NativeFpsPrimaryFireResult {
    fn from(value: FpsPrimaryFireResult) -> Self {
        Self {
            backend: value.backend,
            authority_surface: value.authority_surface,
            mutation_owner: value.mutation_owner,
            workspace_trace: value.workspace_trace,
            shooter: value.shooter as i64,
            target: value.target.map(|target| target as i64),
            target_health_before: value.target_health_before.map(|health| NativeFpsHealth {
                current: health.current,
                max: health.max,
            }),
            target_health_after: value.target_health_after.map(|health| NativeFpsHealth {
                current: health.current,
                max: health.max,
            }),
            lifecycle_status: native_fps_lifecycle_status(value.lifecycle_status),
            target_render_visible: value.target_render_visible,
            entity_hash: native_hash(value.entity_hash),
            health_hash: native_hash(value.health_hash),
            replay_hash: native_hash(value.replay_hash),
        }
    }
}

impl From<NativeFpsEncounterLifecycleInput> for FpsEncounterLifecycleInput {
    fn from(value: NativeFpsEncounterLifecycleInput) -> Self {
        Self {
            outcome_kind: value.outcome_kind,
            terminal: value.terminal,
            enemy_dead: value.enemy_dead,
            player_dead: value.player_dead,
            lifecycle_hash: value.lifecycle_hash,
        }
    }
}

impl From<FpsEncounterLifecycleInput> for NativeFpsEncounterLifecycleInput {
    fn from(value: FpsEncounterLifecycleInput) -> Self {
        Self {
            outcome_kind: value.outcome_kind,
            terminal: value.terminal,
            enemy_dead: value.enemy_dead,
            player_dead: value.player_dead,
            lifecycle_hash: value.lifecycle_hash,
        }
    }
}

impl From<FpsEncounterStateReadout> for NativeFpsEncounterStateReadout {
    fn from(value: FpsEncounterStateReadout) -> Self {
        Self {
            preset_id: value.preset_id,
            status: value.status,
            spawned_enemy_ids: value.spawned_enemy_ids,
            defeated_enemy_ids: value.defeated_enemy_ids,
            revision: value.revision as i64,
            last_transition: value.last_transition,
        }
    }
}

fn native_fps_read_sets(
    read_sets: Vec<runtime_bridge_api::FpsReadSetEvidence>,
) -> Vec<NativeFpsReadSetEvidence> {
    read_sets
        .into_iter()
        .map(|read_set| NativeFpsReadSetEvidence {
            view_kind: read_set.view_kind,
            owner: read_set.owner,
            read_set: read_set.read_set,
        })
        .collect()
}

impl From<FpsEncounterDirectorSnapshot> for NativeFpsEncounterDirectorSnapshot {
    fn from(value: FpsEncounterDirectorSnapshot) -> Self {
        Self {
            backend: value.backend,
            authority_surface: value.authority_surface,
            mutation_owner: value.mutation_owner,
            workspace_trace: value.workspace_trace,
            state: value.state.into(),
            lifecycle: value.lifecycle.into(),
            read_sets: native_fps_read_sets(value.read_sets),
            encounter_hash: native_hash(value.encounter_hash),
            replay_hash: native_hash(value.replay_hash),
        }
    }
}

impl From<FpsEncounterTransitionResult> for NativeFpsEncounterTransitionResult {
    fn from(value: FpsEncounterTransitionResult) -> Self {
        Self {
            backend: value.backend,
            authority_surface: value.authority_surface,
            mutation_owner: value.mutation_owner,
            workspace_trace: value.workspace_trace,
            accepted: value.accepted,
            rejection_reason: value.rejection_reason,
            event_kind: value.event_kind,
            state: value.state.into(),
            lifecycle: value.lifecycle.into(),
            encounter_hash: native_hash(value.encounter_hash),
            replay_hash: native_hash(value.replay_hash),
        }
    }
}

impl From<ComposedRuntimeSessionReadout> for NativeComposedRuntimeSessionReadout {
    fn from(value: ComposedRuntimeSessionReadout) -> Self {
        let scheduler = value.gameplay.scheduler;
        Self {
            schema_version: value.schema_version,
            entity_authority_hash: value.entity_authority_hash,
            gameplay: NativeComposedGameplayReadout {
                gameplay_registry_digest: value.gameplay.gameplay_registry_digest,
                semantic_compatibility_digest: value.gameplay.semantic_compatibility_digest,
                artifact_provenance_digest: value.gameplay.artifact_provenance_digest,
                composition_load_mode: native_composition_load_mode(
                    value.gameplay.composition_load_mode,
                ),
                compatibility_diagnostics: value
                    .gameplay
                    .compatibility_diagnostics
                    .into_iter()
                    .map(NativeGameplayCompositionDiagnostic::from)
                    .collect(),
                binding_registry_hash: value.gameplay.binding_registry_hash,
                activation_hash: value.gameplay.activation_hash,
                module_state_hash: value.gameplay.module_state_hash,
                authority_state_hash: value.gameplay.authority_state_hash,
                trigger_revision: value.gameplay.trigger_revision as i64,
                trigger_snapshot_hash: value.gameplay.trigger_snapshot_hash,
                active_overlap_count: value.gameplay.active_overlap_count,
                reaction_frame_count: value.gameplay.reaction_frame_count,
                last_reaction_frame_hash: value.gameplay.last_reaction_frame_hash,
                decision_receipt_count: value.gameplay.decision_receipt_count,
                pending_decision_count: value.gameplay.pending_decision_count,
                last_decision_receipt_hash: value.gameplay.last_decision_receipt_hash,
                scheduler_state_hash: scheduler.state_hash,
                scheduler_pending_action_count: scheduler.pending_action_count,
                scheduler_outstanding_dispatch_count: scheduler.outstanding_dispatch_count,
                scheduler_outstanding_event_delivery_count: scheduler
                    .outstanding_event_delivery_count,
                scheduler_fact_count: scheduler.fact_count,
                scheduler_truncated: scheduler.truncated,
                runtime_host_hash: value.gameplay.runtime_host_hash,
            },
            fps_session_epoch: value.fps_session_epoch as i64,
            fps_replay_hash: value.fps_replay_hash.map(native_hash),
            runtime_session_hash: value.runtime_session_hash,
        }
    }
}

fn native_composition_load_mode(mode: GameplayCompositionLoadMode) -> String {
    mode.as_str().to_owned()
}

impl From<GameplayCompositionDiagnostic> for NativeGameplayCompositionDiagnostic {
    fn from(value: GameplayCompositionDiagnostic) -> Self {
        Self {
            code: native_composition_diagnostic_code(value.code),
            severity: value.severity.as_str().to_owned(),
            path: value.path,
            expected: value.expected,
            actual: value.actual,
            message: value.message,
        }
    }
}

fn native_composition_diagnostic_code(code: GameplayCompositionDiagnosticCode) -> String {
    code.as_str().to_owned()
}

#[cfg(test)]
mod composition_readout_tests {
    use super::*;
    use protocol_diagnostics::DiagnosticSeverity;

    #[test]
    fn native_composition_diagnostic_preserves_public_wire_shape() {
        let diagnostic = NativeGameplayCompositionDiagnostic::from(GameplayCompositionDiagnostic {
            code: GameplayCompositionDiagnosticCode::ArtifactProvenanceMismatch,
            severity: DiagnosticSeverity::Warning,
            path: "projectBundle.compositionRequirement.artifactProvenanceDigest".to_owned(),
            expected: Some("fnv1a64:1111111111111111".to_owned()),
            actual: Some("fnv1a64:2222222222222222".to_owned()),
            message: "compatible load retained exact provenance evidence".to_owned(),
        });

        assert_eq!(diagnostic.code, "artifactProvenanceMismatch");
        assert_eq!(diagnostic.severity, "warning");
        assert_eq!(
            native_composition_load_mode(GameplayCompositionLoadMode::Compatible),
            "compatible"
        );
        assert_eq!(
            native_composition_load_mode(GameplayCompositionLoadMode::Exact),
            "exact"
        );
    }
}

impl From<GameplayContractRef> for NativeGameplayContractRef {
    fn from(value: GameplayContractRef) -> Self {
        Self {
            namespace: value.namespace,
            name: value.name,
            version: value.version,
            schema_hash: value.schema_hash,
        }
    }
}

fn native_module_view_scope(scope: GameplayModuleViewScope) -> (String, Option<i64>) {
    match scope {
        GameplayModuleViewScope::Session => ("session".to_owned(), None),
        GameplayModuleViewScope::Entity { entity } => ("entity".to_owned(), Some(entity as i64)),
        GameplayModuleViewScope::PrefabInstance { instance } => {
            ("prefabInstance".to_owned(), Some(instance as i64))
        }
    }
}

impl From<GameplayModuleViewSnapshot> for NativeGameplayModuleViewSnapshot {
    fn from(value: GameplayModuleViewSnapshot) -> Self {
        let (scope_kind, scope_value) = native_module_view_scope(value.scope);
        Self {
            view: value.view.into(),
            provider_id: value.provider_id,
            scope_kind,
            scope_value,
            revision: value.revision as i64,
            canonical_payload: value.canonical_payload,
            view_hash: value.view_hash,
            runtime_session_hash: value.runtime_session_hash,
        }
    }
}

impl From<GameplayPrefabPartInteractionReceipt> for NativeGameplayPrefabPartInteractionReceipt {
    fn from(value: GameplayPrefabPartInteractionReceipt) -> Self {
        Self {
            actor: value.actor as i64,
            instance: value.instance as i64,
            role: value.role,
            target: value.target as i64,
            event_hash: value.event_hash,
            reaction_frame_hash: value.reaction_frame_hash,
            runtime_session_hash: value.runtime_session_hash,
        }
    }
}

fn module_view_scope(kind: &str, value: Option<i64>) -> napi::Result<GameplayModuleViewScope> {
    match (kind, value) {
        ("session", None) => Ok(GameplayModuleViewScope::Session),
        ("entity", Some(entity)) => Ok(GameplayModuleViewScope::Entity {
            entity: u64_input(entity, "scopeValue")?,
        }),
        ("prefabInstance", Some(instance)) => Ok(GameplayModuleViewScope::PrefabInstance {
            instance: u64_input(instance, "scopeValue")?,
        }),
        _ => Err(to_napi(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            "module view scope must be session without a value, or entity/prefabInstance with a value",
        ))),
    }
}

#[napi]
pub fn read_fps_runtime_session(handle: i64) -> napi::Result<NativeFpsRuntimeSessionSnapshot> {
    with_bridge(handle, |bridge| {
        bridge
            .read_fps_runtime_session()
            .map(NativeFpsRuntimeSessionSnapshot::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn apply_fps_primary_fire(
    handle: i64,
    tick: i64,
    origin: NativeVec3,
    direction: NativeVec3,
    shooter_role: Option<String>,
    target_role: Option<String>,
) -> napi::Result<NativeFpsPrimaryFireResult> {
    let tick = u64_input(tick, "tick")?;
    let origin = origin.to_vec3("origin")?;
    let direction = direction.to_vec3("direction")?;
    let shooter_role = optional_native_fps_role(shooter_role, "shooterRole")?;
    let target_role = optional_native_fps_role(target_role, "targetRole")?;
    with_bridge(handle, |bridge| {
        bridge
            .apply_fps_primary_fire(FpsPrimaryFireRequest {
                tick,
                origin: [
                    f64::from(origin.x),
                    f64::from(origin.y),
                    f64::from(origin.z),
                ],
                direction: [
                    f64::from(direction.x),
                    f64::from(direction.y),
                    f64::from(direction.z),
                ],
                shooter_role,
                target_role,
            })
            .map(NativeFpsPrimaryFireResult::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn read_composed_runtime_session(
    handle: i64,
) -> napi::Result<NativeComposedRuntimeSessionReadout> {
    with_bridge(handle, |bridge| {
        bridge
            .read_composed_runtime_session()
            .map(NativeComposedRuntimeSessionReadout::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn read_gameplay_module_view(
    handle: i64,
    namespace: String,
    name: String,
    version: i64,
    schema_hash: String,
    scope_kind: String,
    scope_value: Option<i64>,
    expected_runtime_session_hash: String,
) -> napi::Result<NativeGameplayModuleViewSnapshot> {
    let version = u32_input(version, "version")?;
    let scope = module_view_scope(&scope_kind, scope_value)?;
    with_bridge(handle, |bridge| {
        bridge
            .read_gameplay_module_view(GameplayModuleViewRequest {
                view: GameplayContractRef {
                    namespace,
                    name,
                    version,
                    schema_hash,
                },
                scope,
                expected_runtime_session_hash,
            })
            .map(NativeGameplayModuleViewSnapshot::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn apply_gameplay_prefab_part_interaction(
    handle: i64,
    actor: i64,
    instance: i64,
    role: String,
    expected_target: i64,
    tick: i64,
    expected_runtime_session_hash: String,
) -> napi::Result<NativeGameplayPrefabPartInteractionReceipt> {
    let request = GameplayPrefabPartInteractionRequest {
        actor: u64_input(actor, "actor")?,
        instance: u64_input(instance, "instance")?,
        role,
        expected_target: u64_input(expected_target, "expectedTarget")?,
        tick: u64_input(tick, "tick")?,
        expected_runtime_session_hash,
    };
    with_bridge(handle, |bridge| {
        bridge
            .apply_gameplay_prefab_part_interaction(request)
            .map(NativeGameplayPrefabPartInteractionReceipt::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn invoke_game_extension_weapon_effect(
    handle: i64,
    hook_json: String,
    tick: i64,
    origin: NativeVec3,
    direction: NativeVec3,
    shooter_role: Option<String>,
    target_role: Option<String>,
) -> napi::Result<NativeGameExtensionWeaponEffectInvocationResult> {
    let hook = parse_weapon_effect_hook_request(&hook_json)?;
    let tick = u64_input(tick, "tick")?;
    let origin = origin.to_vec3("origin")?;
    let direction = direction.to_vec3("direction")?;
    let shooter_role = optional_native_fps_role(shooter_role, "shooterRole")?;
    let target_role = optional_native_fps_role(target_role, "targetRole")?;
    with_bridge(handle, |bridge| {
        let result = bridge
            .invoke_game_extension_weapon_effect(GameExtensionWeaponEffectInvocationRequest {
                hook,
                primary_fire: FpsPrimaryFireRequest {
                    tick,
                    origin: [
                        f64::from(origin.x),
                        f64::from(origin.y),
                        f64::from(origin.z),
                    ],
                    direction: [
                        f64::from(direction.x),
                        f64::from(direction.y),
                        f64::from(direction.z),
                    ],
                    shooter_role,
                    target_role,
                },
            })
            .map_err(to_napi)?;
        Ok(NativeGameExtensionWeaponEffectInvocationResult {
            hook_receipt_json: game_extension_json(&result.hook_receipt)?,
            replay_evidence_json: game_extension_json(&result.replay_evidence)?,
            primary_fire: result.primary_fire.map(NativeFpsPrimaryFireResult::from),
        })
    })
}

#[napi]
pub fn validate_game_rule_catalog(handle: i64, catalog_json: String) -> napi::Result<String> {
    let catalog = parse_game_rule_catalog(&catalog_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .validate_game_rule_catalog(catalog)
            .map_err(to_napi)?;
        game_rule_json(&receipt)
    })
}

#[napi]
pub fn submit_game_rule_effect_intent(
    handle: i64,
    catalog_json: String,
    request_json: String,
) -> napi::Result<String> {
    let catalog = parse_game_rule_catalog(&catalog_json)?;
    let request = parse_game_rule_resolution_request(&request_json)?;
    with_bridge(handle, |bridge| {
        let receipt = bridge
            .submit_game_rule_effect_intent(GameRuleEffectIntentRequest { catalog, request })
            .map_err(to_napi)?;
        game_rule_json(&receipt)
    })
}

#[napi]
pub fn read_game_rule_runtime_readout(handle: i64) -> napi::Result<String> {
    with_bridge(handle, |bridge| {
        let readout = bridge.read_game_rule_runtime_readout().map_err(to_napi)?;
        game_rule_json(&readout)
    })
}

#[napi]
pub fn restart_fps_runtime_session(
    handle: i64,
    expected_epoch: i64,
) -> napi::Result<NativeFpsRuntimeSessionSnapshot> {
    let expected_epoch = u64_input(expected_epoch, "expected_epoch")?;
    with_bridge(handle, |bridge| {
        bridge
            .restart_fps_runtime_session(FpsRuntimeSessionRestartRequest { expected_epoch })
            .map(NativeFpsRuntimeSessionSnapshot::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn read_fps_encounter_director(
    handle: i64,
    lifecycle: NativeFpsEncounterLifecycleInput,
) -> napi::Result<NativeFpsEncounterDirectorSnapshot> {
    with_bridge(handle, |bridge| {
        bridge
            .read_fps_encounter_director(lifecycle.into())
            .map(NativeFpsEncounterDirectorSnapshot::from)
            .map_err(to_napi)
    })
}

#[napi]
pub fn apply_fps_encounter_transition(
    handle: i64,
    request: NativeFpsEncounterTransitionRequest,
) -> napi::Result<NativeFpsEncounterTransitionResult> {
    with_bridge(handle, |bridge| {
        bridge
            .apply_fps_encounter_transition(FpsEncounterTransitionRequest {
                preset_id: request.preset_id,
                action: request.action,
                lifecycle: request.lifecycle.into(),
            })
            .map(NativeFpsEncounterTransitionResult::from)
            .map_err(to_napi)
    })
}
