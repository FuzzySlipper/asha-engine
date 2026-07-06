//! Lifecycle rule composition for the narrow FPS RuntimeSession authority slice.
//!
//! # Lane
//!
//! `rust-rule` composes state/protocol/service crates into explicit lifecycle
//! transitions. This crate does not render, run UI, or execute policy scripts.
//! For the current FPS demo loop it owns the ProjectBundle bootstrap readout,
//! health/death lifecycle state, primary-fire application, and render visibility
//! lifecycle projection over the lower-level `svc-*` substrates.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use core_entity::{EntityLifecycle, EntityStore};
use core_ids::EntityId;
use core_space::WorldPos;
use protocol_entity_authoring::{
    AuthoringCapability, EntityAuthoringCommand, EntityDefinition, EntityDefinitionCapability,
};
use svc_collision::{CollisionProjection, Ray};
use svc_combat::{
    apply_fire_intent, CombatEvent, CombatFireOutcome, CombatOutcome, CombatReadout,
    CombatRejectionReason, CombatState, CombatTarget, FireControlState, FireIntentCommand,
    HealthState,
};
use svc_entity_authoring::{
    bootstrap_project_bundle_entity_definitions, EcrpRuleOwner,
    ProjectBundleEntityDefinitionBootstrapEntry, ProjectBundleEntityDefinitionBootstrapError,
    ProjectBundleEntityDefinitionBootstrapRecord, ProjectBundleEntityDefinitionBootstrapRequest,
    RuleOwnedEntityAuthoringOutcome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FpsRuntimeRole {
    Player,
    Enemy,
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsWeaponMount {
    pub weapon_id: String,
    pub damage: u32,
    pub range_units: u32,
    pub ammo: u32,
    pub cooldown_ticks_after_fire: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsRenderProjectionState {
    pub projection: String,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsPolicyBinding {
    pub binding_id: String,
    pub policy_id: String,
    pub view_kind: String,
    pub view_version: String,
    pub allowed_intents: Vec<String>,
    pub runtime_moment: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FpsStoredEntityDefinition {
    pub entity: EntityId,
    pub definition: EntityDefinition,
    pub role: FpsRuntimeRole,
    pub health: Option<HealthState>,
    pub weapon: Option<FpsWeaponMount>,
    pub render_projection: Option<FpsRenderProjectionState>,
    pub policy_binding: Option<FpsPolicyBinding>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FpsProjectBundleLoadInput {
    pub project_bundle: String,
    pub definitions: Vec<FpsStoredEntityDefinition>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FpsRuntimeError {
    MissingProjectBundle,
    EmptyDefinitions,
    DuplicateEntity {
        entity: EntityId,
    },
    DuplicateStableId {
        stable_id: String,
    },
    MissingPlayer,
    MissingEnemy,
    MissingPlayerWeapon {
        entity: EntityId,
    },
    MissingEnemyHealth {
        entity: EntityId,
    },
    MissingEnemyBounds {
        entity: EntityId,
    },
    InvalidHealth {
        entity: EntityId,
    },
    InvalidPolicyBinding {
        entity: EntityId,
        field: &'static str,
    },
    Bootstrap(ProjectBundleEntityDefinitionBootstrapError),
    RuleMutationRejected {
        entity: EntityId,
        command: &'static str,
    },
    CombatRejected(CombatRejectionReason),
    UnknownEncounterPreset {
        preset_id: String,
    },
    InvalidEncounterTransition {
        action: String,
    },
    EncounterNotPending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FpsLifecycleStatus {
    Active,
    EnemyDefeated { entity: EntityId, tick: u64 },
}

pub const FPS_GENERATED_TUNNEL_ENCOUNTER_PRESET: &str = "generated-tunnel-small-encounter";
pub const FPS_GENERATED_TUNNEL_ENCOUNTER_INSTANCE: &str =
    "encounter.generated_tunnel_small.wave_1.enemy_001";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FpsEncounterStatus {
    Pending,
    Active,
    Cleared,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FpsEncounterLastTransition {
    Initialized,
    Activated,
    Cleared,
    Failed,
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FpsEncounterTransitionAction {
    Activate,
    SyncLifecycle,
    Reset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsEncounterState {
    pub preset_id: String,
    pub status: FpsEncounterStatus,
    pub spawned_enemy_ids: Vec<String>,
    pub defeated_enemy_ids: Vec<String>,
    pub revision: u64,
    pub last_transition: FpsEncounterLastTransition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsEncounterLifecycleInput {
    pub outcome_kind: String,
    pub terminal: bool,
    pub enemy_dead: bool,
    pub player_dead: bool,
    pub lifecycle_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsEncounterTransitionReceipt {
    pub accepted: bool,
    pub rejection_reason: Option<&'static str>,
    pub event_kind: Option<&'static str>,
    pub state: FpsEncounterState,
    pub encounter_hash: u64,
    pub replay_hash: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FpsRuntimeSessionState {
    pub entities: EntityStore,
    pub combat: CombatState,
    pub project_bundle: String,
    pub bootstrap: ProjectBundleEntityDefinitionBootstrapRecord,
    pub definitions: BTreeMap<EntityId, FpsStoredEntityDefinition>,
    pub roles: BTreeMap<FpsRuntimeRole, EntityId>,
    pub render_projection: BTreeMap<EntityId, FpsRenderProjectionState>,
    pub lifecycle_status: FpsLifecycleStatus,
    pub encounter: FpsEncounterState,
    pub replay_records: Vec<FpsReplayRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsReplayRecord {
    pub kind: &'static str,
    pub entity_hash: u64,
    pub health_hash: u64,
    pub record_hash: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FpsPrimaryFireReceipt {
    pub shooter: EntityId,
    pub target: Option<EntityId>,
    pub target_health_before: Option<HealthState>,
    pub target_health_after: Option<HealthState>,
    pub combat: CombatReadout,
    pub lifecycle_status: FpsLifecycleStatus,
    pub target_render_visible: Option<bool>,
    pub entity_hash: u64,
    pub health_hash: u64,
    pub replay_hash: u64,
}

pub fn load_fps_project_bundle(
    input: FpsProjectBundleLoadInput,
) -> Result<FpsRuntimeSessionState, FpsRuntimeError> {
    validate_load_input(&input)?;

    let mut entities = EntityStore::new();
    let request = ProjectBundleEntityDefinitionBootstrapRequest {
        project_bundle: input.project_bundle.clone(),
        entries: input
            .definitions
            .iter()
            .map(|entry| ProjectBundleEntityDefinitionBootstrapEntry {
                entity: entry.entity,
                definition: entry.definition.clone(),
            })
            .collect(),
    };
    let bootstrap = bootstrap_project_bundle_entity_definitions(&mut entities, &request)
        .map_err(FpsRuntimeError::Bootstrap)?;

    let mut combat = CombatState::new();
    let mut definitions = BTreeMap::new();
    let mut roles = BTreeMap::new();
    let mut render_projection = BTreeMap::new();

    for entry in input.definitions {
        if let Some(health) = entry.health {
            if combat.attach_health(entry.entity, health) != CombatOutcome::Accepted {
                return Err(FpsRuntimeError::InvalidHealth {
                    entity: entry.entity,
                });
            }
        }
        if let Some(render) = &entry.render_projection {
            attach_render_projection(&mut entities, entry.entity, render.visible)?;
            render_projection.insert(entry.entity, render.clone());
        }
        if matches!(entry.role, FpsRuntimeRole::Player | FpsRuntimeRole::Enemy) {
            roles.insert(entry.role, entry.entity);
        }
        definitions.insert(entry.entity, entry);
    }

    let health_hash = combat.health_hash();
    let entity_hash = entities.hash().0;
    let bootstrap_record = FpsReplayRecord {
        kind: "runtime_session.fps.bootstrap.v0",
        entity_hash,
        health_hash,
        record_hash: hash_bootstrap(&bootstrap, health_hash),
    };

    Ok(FpsRuntimeSessionState {
        entities,
        combat,
        project_bundle: input.project_bundle,
        bootstrap,
        definitions,
        roles,
        render_projection,
        lifecycle_status: FpsLifecycleStatus::Active,
        encounter: initial_fps_encounter_state(),
        replay_records: vec![bootstrap_record],
    })
}

impl FpsRuntimeSessionState {
    pub fn apply_encounter_transition(
        &mut self,
        preset_id: &str,
        action: FpsEncounterTransitionAction,
        lifecycle: &FpsEncounterLifecycleInput,
    ) -> Result<FpsEncounterTransitionReceipt, FpsRuntimeError> {
        if preset_id != FPS_GENERATED_TUNNEL_ENCOUNTER_PRESET {
            return Err(FpsRuntimeError::UnknownEncounterPreset {
                preset_id: preset_id.to_string(),
            });
        }

        let mut accepted = true;
        let mut rejection_reason = None;
        let mut event_kind = None;
        let next = match action {
            FpsEncounterTransitionAction::Reset => {
                event_kind = Some("runtime_encounter.reset.v0");
                FpsEncounterState {
                    revision: self.encounter.revision.saturating_add(1),
                    last_transition: FpsEncounterLastTransition::Reset,
                    ..initial_fps_encounter_state()
                }
            }
            FpsEncounterTransitionAction::Activate => {
                if self.encounter.status != FpsEncounterStatus::Pending {
                    accepted = false;
                    rejection_reason = Some("encounter_not_pending");
                    self.encounter.clone()
                } else {
                    event_kind = Some("runtime_encounter.activated.v0");
                    FpsEncounterState {
                        status: FpsEncounterStatus::Active,
                        spawned_enemy_ids: vec![FPS_GENERATED_TUNNEL_ENCOUNTER_INSTANCE.to_string()],
                        revision: self.encounter.revision.saturating_add(1),
                        last_transition: FpsEncounterLastTransition::Activated,
                        ..self.encounter.clone()
                    }
                }
            }
            FpsEncounterTransitionAction::SyncLifecycle => {
                event_kind = Some("runtime_encounter.lifecycle_synced.v0");
                if lifecycle.player_dead || lifecycle.outcome_kind == "lost" {
                    FpsEncounterState {
                        status: FpsEncounterStatus::Failed,
                        revision: self.encounter.revision.saturating_add(1),
                        last_transition: FpsEncounterLastTransition::Failed,
                        ..self.encounter.clone()
                    }
                } else if lifecycle.enemy_dead || lifecycle.outcome_kind == "won" {
                    FpsEncounterState {
                        status: FpsEncounterStatus::Cleared,
                        spawned_enemy_ids: vec![FPS_GENERATED_TUNNEL_ENCOUNTER_INSTANCE.to_string()],
                        defeated_enemy_ids: vec![
                            FPS_GENERATED_TUNNEL_ENCOUNTER_INSTANCE.to_string()
                        ],
                        revision: self.encounter.revision.saturating_add(1),
                        last_transition: FpsEncounterLastTransition::Cleared,
                        ..self.encounter.clone()
                    }
                } else {
                    FpsEncounterState {
                        revision: self.encounter.revision.saturating_add(1),
                        ..self.encounter.clone()
                    }
                }
            }
        };

        if accepted {
            self.encounter = next;
        }
        let encounter_hash = hash_encounter_state(&self.encounter, lifecycle);
        let replay_hash = hash_encounter_transition(
            preset_id,
            action,
            accepted,
            rejection_reason,
            event_kind,
            encounter_hash,
        );
        if accepted {
            self.replay_records.push(FpsReplayRecord {
                kind: event_kind.unwrap_or("runtime_session.fps.encounter_transition.v0"),
                entity_hash: self.entities.hash().0,
                health_hash: self.combat.health_hash(),
                record_hash: replay_hash,
            });
        }
        Ok(FpsEncounterTransitionReceipt {
            accepted,
            rejection_reason,
            event_kind,
            state: self.encounter.clone(),
            encounter_hash,
            replay_hash,
        })
    }

    pub fn apply_primary_fire(
        &mut self,
        projection: &CollisionProjection,
        ray: Ray,
        tick: u64,
    ) -> Result<FpsPrimaryFireReceipt, FpsRuntimeError> {
        let shooter = self.role_entity(FpsRuntimeRole::Player)?;
        let target = self.role_entity(FpsRuntimeRole::Enemy)?;
        let shooter_definition = self
            .definitions
            .get(&shooter)
            .expect("role map is populated from definitions");
        let weapon = shooter_definition
            .weapon
            .as_ref()
            .ok_or(FpsRuntimeError::MissingPlayerWeapon { entity: shooter })?;
        let target_before = self.combat.health(target);
        let combat_target = self.combat_target(target)?;
        let combat = apply_fire_intent(
            &mut self.combat,
            projection,
            &[combat_target],
            FireIntentCommand {
                shooter,
                ray,
                max_distance: weapon.range_units as f64,
                damage: weapon.damage,
                fire_control: FireControlState::ready(
                    weapon.ammo,
                    weapon.cooldown_ticks_after_fire,
                ),
                tick,
            },
        )
        .map_err(FpsRuntimeError::CombatRejected)?;

        let hit_target = match combat.outcome {
            CombatFireOutcome::Hit { target, .. } => Some(target),
            CombatFireOutcome::Miss { .. } => None,
        };
        if combat.events.iter().any(|event| {
            matches!(
                event,
                CombatEvent::EntityDefeated { target: defeated } if *defeated == target
            )
        }) {
            self.apply_enemy_defeated(target, tick)?;
        }

        let target_after = self.combat.health(target);
        let entity_hash = self.entities.hash().0;
        let health_hash = self.combat.health_hash();
        let replay_hash = hash_primary_fire(shooter, target, tick, &combat, entity_hash);
        self.replay_records.push(FpsReplayRecord {
            kind: "runtime_session.fps.primary_fire.v0",
            entity_hash,
            health_hash,
            record_hash: replay_hash,
        });

        Ok(FpsPrimaryFireReceipt {
            shooter,
            target: hit_target,
            target_health_before: target_before,
            target_health_after: target_after,
            combat,
            lifecycle_status: self.lifecycle_status,
            target_render_visible: self
                .render_projection
                .get(&target)
                .map(|render| render.visible),
            entity_hash,
            health_hash,
            replay_hash,
        })
    }

    pub fn health(&self, entity: EntityId) -> Option<HealthState> {
        self.combat.health(entity)
    }

    pub fn entity_lifecycle(&self, entity: EntityId) -> Option<EntityLifecycle> {
        self.entities.lifecycle(entity)
    }

    pub fn role_entity(&self, role: FpsRuntimeRole) -> Result<EntityId, FpsRuntimeError> {
        self.roles.get(&role).copied().ok_or(match role {
            FpsRuntimeRole::Player => FpsRuntimeError::MissingPlayer,
            FpsRuntimeRole::Enemy => FpsRuntimeError::MissingEnemy,
            FpsRuntimeRole::Neutral => FpsRuntimeError::MissingEnemy,
        })
    }

    fn apply_enemy_defeated(&mut self, entity: EntityId, tick: u64) -> Result<(), FpsRuntimeError> {
        let disable = svc_entity_authoring::validate_and_apply_rule_owned(
            &mut self.entities,
            EcrpRuleOwner::LifecycleRule,
            &EntityAuthoringCommand::Disable { id: entity },
        );
        match disable {
            RuleOwnedEntityAuthoringOutcome::Accepted { .. } => {}
            RuleOwnedEntityAuthoringOutcome::Rejected { .. }
            | RuleOwnedEntityAuthoringOutcome::Forbidden { .. } => {
                return Err(FpsRuntimeError::RuleMutationRejected {
                    entity,
                    command: "disable",
                });
            }
        }

        if let Some(render) = self.render_projection.get_mut(&entity) {
            render.visible = false;
            attach_render_projection(&mut self.entities, entity, false)?;
        }
        self.lifecycle_status = FpsLifecycleStatus::EnemyDefeated { entity, tick };
        Ok(())
    }

    fn combat_target(&self, entity: EntityId) -> Result<CombatTarget, FpsRuntimeError> {
        let definition = self
            .definitions
            .get(&entity)
            .ok_or(FpsRuntimeError::MissingEnemyBounds { entity })?;
        let bounds = definition
            .definition
            .capabilities
            .iter()
            .find_map(|capability| match capability {
                EntityDefinitionCapability::Bounds { min, max } => Some((*min, *max)),
                _ => None,
            })
            .ok_or(FpsRuntimeError::MissingEnemyBounds { entity })?;
        Ok(CombatTarget {
            entity,
            min: WorldPos::new(bounds.0[0] as f64, bounds.0[1] as f64, bounds.0[2] as f64),
            max: WorldPos::new(bounds.1[0] as f64, bounds.1[1] as f64, bounds.1[2] as f64),
        })
    }
}

fn validate_load_input(input: &FpsProjectBundleLoadInput) -> Result<(), FpsRuntimeError> {
    if input.project_bundle.trim().is_empty() {
        return Err(FpsRuntimeError::MissingProjectBundle);
    }
    if input.definitions.is_empty() {
        return Err(FpsRuntimeError::EmptyDefinitions);
    }

    let mut stable_ids = BTreeSet::new();
    let mut entities = BTreeSet::new();
    let mut has_player = false;
    let mut has_enemy = false;
    for entry in &input.definitions {
        if !entities.insert(entry.entity) {
            return Err(FpsRuntimeError::DuplicateEntity {
                entity: entry.entity,
            });
        }
        if !stable_ids.insert(entry.definition.stable_id.clone()) {
            return Err(FpsRuntimeError::DuplicateStableId {
                stable_id: entry.definition.stable_id.clone(),
            });
        }
        if entry
            .health
            .is_some_and(|health| health.max == 0 || health.current > health.max)
        {
            return Err(FpsRuntimeError::InvalidHealth {
                entity: entry.entity,
            });
        }
        match entry.role {
            FpsRuntimeRole::Player => {
                has_player = true;
                if entry.weapon.is_none() {
                    return Err(FpsRuntimeError::MissingPlayerWeapon {
                        entity: entry.entity,
                    });
                }
            }
            FpsRuntimeRole::Enemy => {
                has_enemy = true;
                if entry.health.is_none() {
                    return Err(FpsRuntimeError::MissingEnemyHealth {
                        entity: entry.entity,
                    });
                }
            }
            FpsRuntimeRole::Neutral => {}
        }
        if let Some(binding) = &entry.policy_binding {
            validate_policy_binding(entry.entity, binding)?;
        }
    }
    if !has_player {
        return Err(FpsRuntimeError::MissingPlayer);
    }
    if !has_enemy {
        return Err(FpsRuntimeError::MissingEnemy);
    }
    Ok(())
}

fn validate_policy_binding(
    entity: EntityId,
    binding: &FpsPolicyBinding,
) -> Result<(), FpsRuntimeError> {
    for (field, value) in [
        ("binding_id", binding.binding_id.as_str()),
        ("policy_id", binding.policy_id.as_str()),
        ("view_kind", binding.view_kind.as_str()),
        ("view_version", binding.view_version.as_str()),
        ("runtime_moment", binding.runtime_moment.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(FpsRuntimeError::InvalidPolicyBinding { entity, field });
        }
    }
    if binding
        .allowed_intents
        .iter()
        .any(|intent| intent.trim().is_empty())
        || binding.allowed_intents.is_empty()
    {
        return Err(FpsRuntimeError::InvalidPolicyBinding {
            entity,
            field: "allowed_intents",
        });
    }
    Ok(())
}

fn attach_render_projection(
    store: &mut EntityStore,
    entity: EntityId,
    visible: bool,
) -> Result<(), FpsRuntimeError> {
    let outcome = svc_entity_authoring::validate_and_apply_rule_owned(
        store,
        EcrpRuleOwner::RenderProjectionRule,
        &EntityAuthoringCommand::AttachCapability {
            id: entity,
            capability: AuthoringCapability::Render { visible },
        },
    );
    match outcome {
        RuleOwnedEntityAuthoringOutcome::Accepted { .. } => Ok(()),
        RuleOwnedEntityAuthoringOutcome::Rejected { .. }
        | RuleOwnedEntityAuthoringOutcome::Forbidden { .. } => {
            Err(FpsRuntimeError::RuleMutationRejected {
                entity,
                command: "attachRenderProjection",
            })
        }
    }
}

pub fn initial_fps_encounter_state() -> FpsEncounterState {
    FpsEncounterState {
        preset_id: FPS_GENERATED_TUNNEL_ENCOUNTER_PRESET.to_string(),
        status: FpsEncounterStatus::Pending,
        spawned_enemy_ids: Vec::new(),
        defeated_enemy_ids: Vec::new(),
        revision: 0,
        last_transition: FpsEncounterLastTransition::Initialized,
    }
}

fn hash_bootstrap(
    bootstrap: &ProjectBundleEntityDefinitionBootstrapRecord,
    health_hash: u64,
) -> u64 {
    let mut h = Fnv1a::new();
    h.write_str("runtime_session.fps.bootstrap.v0");
    h.write_str(&bootstrap.project_bundle);
    h.write_u64(bootstrap.entity_hash.0);
    h.write_u64(health_hash);
    for record in &bootstrap.records {
        h.write_str(&record.stable_id);
        h.write_u64(record.entity.raw());
        h.write_u64(record.entity_hash.0);
    }
    h.finish()
}

fn hash_primary_fire(
    shooter: EntityId,
    target: EntityId,
    tick: u64,
    combat: &CombatReadout,
    entity_hash: u64,
) -> u64 {
    let mut h = Fnv1a::new();
    h.write_str("runtime_session.fps.primary_fire.v0");
    h.write_u64(shooter.raw());
    h.write_u64(target.raw());
    h.write_u64(tick);
    h.write_u64(combat.health_hash);
    h.write_u64(combat.replay_hash);
    h.write_u64(entity_hash);
    h.finish()
}

fn hash_encounter_state(state: &FpsEncounterState, lifecycle: &FpsEncounterLifecycleInput) -> u64 {
    let mut h = Fnv1a::new();
    h.write_str("runtime_session.fps.encounter_state.v0");
    h.write_str(&state.preset_id);
    h.write_str(encounter_status_label(state.status));
    for id in &state.spawned_enemy_ids {
        h.write_str(id);
    }
    for id in &state.defeated_enemy_ids {
        h.write_str(id);
    }
    h.write_u64(state.revision);
    h.write_str(encounter_transition_label(state.last_transition));
    h.write_str(&lifecycle.outcome_kind);
    h.write_bool(lifecycle.terminal);
    h.write_bool(lifecycle.enemy_dead);
    h.write_bool(lifecycle.player_dead);
    h.write_str(&lifecycle.lifecycle_hash);
    h.finish()
}

fn hash_encounter_transition(
    preset_id: &str,
    action: FpsEncounterTransitionAction,
    accepted: bool,
    rejection_reason: Option<&str>,
    event_kind: Option<&str>,
    encounter_hash: u64,
) -> u64 {
    let mut h = Fnv1a::new();
    h.write_str("runtime_session.fps.encounter_transition.v0");
    h.write_str(preset_id);
    h.write_str(encounter_action_label(action));
    h.write_bool(accepted);
    h.write_str(rejection_reason.unwrap_or("none"));
    h.write_str(event_kind.unwrap_or("none"));
    h.write_u64(encounter_hash);
    h.finish()
}

fn encounter_action_label(action: FpsEncounterTransitionAction) -> &'static str {
    match action {
        FpsEncounterTransitionAction::Activate => "activate",
        FpsEncounterTransitionAction::SyncLifecycle => "sync_lifecycle",
        FpsEncounterTransitionAction::Reset => "reset",
    }
}

fn encounter_status_label(status: FpsEncounterStatus) -> &'static str {
    match status {
        FpsEncounterStatus::Pending => "pending",
        FpsEncounterStatus::Active => "active",
        FpsEncounterStatus::Cleared => "cleared",
        FpsEncounterStatus::Failed => "failed",
    }
}

fn encounter_transition_label(transition: FpsEncounterLastTransition) -> &'static str {
    match transition {
        FpsEncounterLastTransition::Initialized => "initialized",
        FpsEncounterLastTransition::Activated => "activated",
        FpsEncounterLastTransition::Cleared => "cleared",
        FpsEncounterLastTransition::Failed => "failed",
        FpsEncounterLastTransition::Reset => "reset",
    }
}

struct Fnv1a {
    value: u64,
}

impl Fnv1a {
    fn new() -> Self {
        Self {
            value: 0xcbf2_9ce4_8422_2325,
        }
    }

    fn write_u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.write_byte(byte);
        }
    }

    fn write_bool(&mut self, value: bool) {
        self.write_byte(u8::from(value));
    }

    fn write_str(&mut self, value: &str) {
        for byte in value.as_bytes() {
            self.write_byte(*byte);
        }
    }

    fn write_byte(&mut self, byte: u8) {
        self.value ^= byte as u64;
        self.value = self.value.wrapping_mul(0x0000_0100_0000_01b3);
    }

    fn finish(self) -> u64 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_space::WorldVec;
    use protocol_entity_authoring::{AuthoringTransform, EntityDefinitionSourceTrace};
    use svc_levelgen::{generate_tunnel, TunnelGeneratorConfig};

    fn tunnel_projection() -> CollisionProjection {
        let tunnel = generate_tunnel(TunnelGeneratorConfig::tiny_enclosed(17)).expect("tunnel");
        CollisionProjection::build(&tunnel.world)
    }

    fn definition(
        stable_id: &str,
        display_name: &str,
        bounds: ([f32; 3], [f32; 3]),
    ) -> EntityDefinition {
        EntityDefinition {
            stable_id: stable_id.into(),
            display_name: display_name.into(),
            source: EntityDefinitionSourceTrace {
                project_bundle: "custom-demo".into(),
                relative_path: format!("catalogs/actors/{stable_id}.entity.json"),
            },
            tags: Vec::new(),
            metadata: Vec::new(),
            capabilities: vec![
                EntityDefinitionCapability::Transform {
                    transform: AuthoringTransform {
                        translation: [0.0, 0.0, 0.0],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        scale: [1.0, 1.0, 1.0],
                    },
                },
                EntityDefinitionCapability::Bounds {
                    min: bounds.0,
                    max: bounds.1,
                },
                EntityDefinitionCapability::Render { visible: true },
                EntityDefinitionCapability::Collision {
                    static_collider: false,
                },
            ],
        }
    }

    fn load_custom_session() -> FpsRuntimeSessionState {
        load_fps_project_bundle(FpsProjectBundleLoadInput {
            project_bundle: "custom-demo".into(),
            definitions: vec![
                FpsStoredEntityDefinition {
                    entity: EntityId::new(101),
                    definition: definition(
                        "actor/custom-player",
                        "Custom Player",
                        ([2.2, 1.0, 1.0], [2.8, 2.0, 2.0]),
                    ),
                    role: FpsRuntimeRole::Player,
                    health: Some(HealthState::new(88, 88)),
                    weapon: Some(FpsWeaponMount {
                        weapon_id: "weapon.custom.primary".into(),
                        damage: 75,
                        range_units: 16,
                        ammo: 3,
                        cooldown_ticks_after_fire: 4,
                    }),
                    render_projection: Some(FpsRenderProjectionState {
                        projection: "first_person_camera".into(),
                        visible: true,
                    }),
                    policy_binding: None,
                },
                FpsStoredEntityDefinition {
                    entity: EntityId::new(777),
                    definition: definition(
                        "actor/custom-enemy",
                        "Custom Enemy",
                        ([2.2, 1.0, 5.0], [2.8, 2.0, 5.8]),
                    ),
                    role: FpsRuntimeRole::Enemy,
                    health: Some(HealthState::new(75, 75)),
                    weapon: None,
                    render_projection: Some(FpsRenderProjectionState {
                        projection: "target_cube".into(),
                        visible: true,
                    }),
                    policy_binding: Some(FpsPolicyBinding {
                        binding_id: "binding.enemy.custom.v0".into(),
                        policy_id: "policy.enemy.custom.v0".into(),
                        view_kind: "runtime_session.nav_policy_view.v0".into(),
                        view_version: "v0".into(),
                        allowed_intents: vec![
                            "runtime.intent.move_direct_nav.v0".into(),
                            "runtime.intent.primary_fire.v0".into(),
                        ],
                        runtime_moment: "runtime.tick.enemy_policy.v0".into(),
                    }),
                },
            ],
        })
        .expect("load session")
    }

    #[test]
    fn loaded_fps_entities_seed_combat_and_primary_fire_drives_death_lifecycle() {
        let projection = tunnel_projection();
        let mut session = load_custom_session();
        let enemy = EntityId::new(777);

        assert_eq!(session.bootstrap.records.len(), 2);
        assert_eq!(session.health(enemy), Some(HealthState::new(75, 75)));
        assert_eq!(
            session.entity_lifecycle(enemy),
            Some(EntityLifecycle::Active)
        );
        assert_eq!(
            session.render_projection.get(&enemy).map(|r| r.visible),
            Some(true)
        );
        assert_ne!(session.replay_records[0].record_hash, 0);

        let receipt = session
            .apply_primary_fire(
                &projection,
                Ray::new(WorldPos::new(2.5, 1.5, 1.5), WorldVec::new(0.0, 0.0, 1.0)),
                9,
            )
            .expect("primary fire");

        assert_eq!(receipt.shooter, EntityId::new(101));
        assert_eq!(receipt.target, Some(enemy));
        assert_eq!(receipt.target_health_before, Some(HealthState::new(75, 75)));
        assert_eq!(receipt.target_health_after, Some(HealthState::new(0, 75)));
        assert_eq!(
            receipt.lifecycle_status,
            FpsLifecycleStatus::EnemyDefeated {
                entity: enemy,
                tick: 9
            }
        );
        assert_eq!(
            session.entity_lifecycle(enemy),
            Some(EntityLifecycle::Disabled)
        );
        assert_eq!(receipt.target_render_visible, Some(false));
        assert_eq!(
            session.render_projection.get(&enemy).map(|r| r.visible),
            Some(false)
        );
        assert!(matches!(
            receipt.combat.outcome,
            CombatFireOutcome::Hit {
                target,
                defeated: true,
                ..
            } if target == enemy
        ));
        assert!(receipt.combat.events.iter().any(
            |event| matches!(event, CombatEvent::EntityDefeated { target } if *target == enemy)
        ));
        assert_eq!(receipt.health_hash, session.combat.health_hash());
        assert_ne!(receipt.health_hash, 0);
        assert_ne!(receipt.replay_hash, 0);
        assert_eq!(session.replay_records.len(), 2);
        assert_eq!(session.replay_records[1].record_hash, receipt.replay_hash);
    }
}
