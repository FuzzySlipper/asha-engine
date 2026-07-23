//! Public-height, statically composed gameplay RuntimeSession host.
//!
//! This is the Rust host seam a downstream native provider can compose. It owns
//! no module discovery and accepts no callbacks: the module topology is a
//! concrete GameplayStaticComposition supplied at construction.

#![forbid(unsafe_code)]

mod authored_behavior;
mod authority_verbs;
mod interaction;
mod owner_router;
mod prefab;
mod project_activation;
mod project_admission;
#[cfg(test)]
mod reset_tests;
mod scheduler;
mod transaction;

pub use interaction::*;
pub use prefab::*;
pub use project_activation::GameplayRuntimeActivatedProjectIdentity;
pub use project_admission::*;
pub use scheduler::*;
use transaction::activation_hash;
pub use transaction::{GameplayRuntimeResetCheckpoint, GameplayRuntimeTransactionCheckpoint};

use owner_router::{RuntimeSessionDecisionOwner, RuntimeSessionOwnerRouter};

use std::collections::{BTreeMap, BTreeSet};

use core_entity::{
    Aabb, EntityLifecycleCommand, EntitySource, EntityStore, EntityTransform, MovementCommand,
    MovementEvent, TransformCommand,
};
use core_math::Vec3;
pub use gameplay_module_sdk::{
    gameplay_runtime_composition_identity, GameplayConfigurationValueKind,
    GameplayProjectConfigurationAuthority, GameplayRuntimeCompositionIdentity,
    GameplayRuntimeDeclaredReadPlan, GameplayStaticComposition,
};
use protocol_diagnostics::DiagnosticSeverity;
use protocol_game_extension::{
    GameplayCompositionDiagnostic, GameplayCompositionDiagnosticCode, GameplayCompositionLoadMode,
    GameplayCompositionRequirement, GameplayEventEnvelope, GameplayEventPhase,
    GameplayModuleBindingActivationReceipt, GameplayModuleBindingRegistry, GameplayOwnerRef,
    GameplayProposalEnvelope,
};
use rule_gameplay_fabric::{
    adapt_session_tick, gameplay_module_payload_hash, FrozenGameplayViews,
    GameplayDecisionContinuations, GameplayEntityScopeIndex, GameplayFabricCoordinator,
    GameplayFrozenReadSet, GameplayHostError, GameplayModuleStateError, GameplayObserveReceipt,
    GameplayOwnerEventContext, GameplayOwnerQueryProvider, GameplayPrefabInstanceBinding,
    GameplayPrefabInstanceIndex, GameplayReactionFrame, GameplayReactionSourceFact,
    GameplayReadAssembler, GameplayReadAssemblyError, GameplayReadDiagnostic,
    GameplayReadDiagnosticCode, GameplayReadPlan, GameplayReadSelector, GameplayRuntimeDiagnostic,
    GameplayRuntimeLimits, GameplayTriggerOverlapQueryProvider, GameplayViewSource,
    GameplayWaveAuthority, GameplayWaveStateHashes,
};
use rule_project_bundle::{
    GameplayBindingActivationError, GameplayBoundProjectBundleSession, SessionStateArtifact,
};
use rule_scheduler::{GameplayActionScheduler, GameplaySchedulerCommand};
use serde::{Deserialize, Serialize};

// These are deliberately re-exported from the public host altitude. Consumers
// can execute a normal ProjectBundle load without naming private engine crates.
pub use core_ids::{EntityId, RuntimeSessionId, SceneId};
pub use core_scene::BootstrapResolutionContext;
pub use protocol_project_bundle::{
    GameplayTriggerDefinition, GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
};
pub use rule_gameplay_fabric::{
    GameplayDecisionContinuation, GameplayDecisionMoment, GameplayDecisionReceipt,
    GameplayDecisionStatus, GameplayModuleStateReadout, GameplayOperationWorkspace,
    GameplayRoutingEvidence,
};
pub use rule_project_bundle::{
    execute_load_plan, BundleArtifacts, GameplayBindingEntityTargets, ProjectBundleLoadResult,
};
pub use rule_trigger_volume::{
    TriggerReconcileCause, TriggerReconcileReceipt, TriggerVolumeDiagnostic,
};
use svc_serialization::{
    encode_prefab_registry, ArtifactEntry, ArtifactRole, PrefabRegistry,
    PrefabRegistryValidationContext, ValidatedPrefabRegistry, PREFAB_REGISTRY_SCHEMA_VERSION,
};
pub use svc_serialization::{LoadPlan, LoadStep};

pub const GAMEPLAY_RUNTIME_HOST_SNAPSHOT_PATH: &str = "session/gameplay-runtime-host.snapshot.json";
/// Machine-readable quarantine diagnostic for direct host construction.
pub const GAMEPLAY_RUNTIME_HOST_COMPATIBILITY_DIAGNOSTIC: &str =
    "asha.compat.wave1.standalone-gameplay-runtime-host";
const GAMEPLAY_RUNTIME_HOST_SNAPSHOT_VERSION: u32 = 5;
const MAX_REACTION_FRAMES: usize = 256;
const MAX_DECISION_RECEIPTS: usize = 256;

#[derive(Debug)]
pub enum GameplayRuntimeHostError {
    Load(String),
    Prefab(String),
    Snapshot(String),
    Activation(GameplayBindingActivationError),
    Compatibility(Vec<GameplayCompositionDiagnostic>),
    MissingEntityAuthority,
    Transform { entity: u64, code: &'static str },
    SpatialEntity { entity: u64, code: &'static str },
    Movement { entity: u64, code: &'static str },
    State(GameplayModuleStateError),
    Codec(String),
    Scheduler(GameplaySchedulerError),
    SchedulerRouting(GameplayRuntimeDiagnostic),
    AuthoredProgram(String),
}

impl core::fmt::Display for GameplayRuntimeHostError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for GameplayRuntimeHostError {}

impl From<GameplayBindingActivationError> for GameplayRuntimeHostError {
    fn from(value: GameplayBindingActivationError) -> Self {
        Self::Activation(value)
    }
}

impl From<GameplayModuleStateError> for GameplayRuntimeHostError {
    fn from(value: GameplayModuleStateError) -> Self {
        Self::State(value)
    }
}

impl From<GameplaySchedulerError> for GameplayRuntimeHostError {
    fn from(value: GameplaySchedulerError) -> Self {
        Self::Scheduler(value)
    }
}

pub struct GameplayRuntimeHostInput {
    pub bundle: ProjectBundleLoadResult,
    pub composition: GameplayStaticComposition,
    pub composition_requirement: Option<GameplayCompositionRequirement>,
    pub bindings: GameplayModuleBindingRegistry,
    pub entity_targets: GameplayBindingEntityTargets,
    pub spatial_entities: Vec<GameplayRuntimeSpatialEntity>,
    pub declared_reads: Vec<GameplayRuntimeDeclaredReadPlan>,
    pub triggers: Vec<GameplayTriggerDefinition>,
    pub scheduler: GameplayRuntimeSchedulerDefinition,
}

/// Compiler-owned activation parts. The canonical admission artifact is the
/// only constructor; downstream consumers cannot assemble this topology.
pub(crate) struct RuntimeProjectActivationInput {
    pub load_plan: LoadPlan,
    pub artifacts: BundleArtifacts,
    /// Independently validated external identities for typed scene references.
    /// Marker identities remain inside the scene and are not supplied here.
    pub bootstrap_resolution: BootstrapResolutionContext,
    pub composition: GameplayStaticComposition,
    pub composition_requirement: Option<GameplayCompositionRequirement>,
    pub bindings: GameplayModuleBindingRegistry,
    pub entity_targets: GameplayBindingEntityTargets,
    pub spatial_entities: Vec<GameplayRuntimeSpatialEntity>,
    pub declared_reads: Vec<GameplayRuntimeDeclaredReadPlan>,
    pub triggers: Vec<GameplayTriggerDefinition>,
    pub scheduler: GameplayRuntimeSchedulerDefinition,
    pub authored_program: Option<authored_behavior::CompiledAuthoredProgram>,
}

/// Typed bootstrap data for runtime entities that participate in gameplay
/// geometry. This remains an explicit host input until generated ProjectBundle
/// entity definitions carry the same capability data.
#[derive(Debug, Clone, PartialEq)]
pub struct GameplayRuntimeSpatialEntity {
    pub entity: EntityId,
    pub translation: [f32; 3],
    pub half_extents: [f32; 3],
    pub static_collider: bool,
}

/// Narrow statically linked Rust authority port for one pre-commit owner. The
/// public gameplay host resolves the closed owner from its registry; a
/// consumer implements only revision readback and one atomic route.
pub trait GameplayRuntimeDecisionOwner {
    fn revision_hash(&self, owner: &GameplayOwnerRef) -> String;

    fn route_precommit(
        &mut self,
        owner: &GameplayOwnerRef,
        operation: &GameplayProposalEnvelope,
    ) -> GameplayRuntimeDecisionOwnerOutput;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameplayRuntimeDecisionOwnerOutput {
    pub accepted: bool,
    pub fact_hashes: Vec<String>,
    pub diagnostic_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRuntimeHostReadout {
    pub gameplay_registry_digest: String,
    pub semantic_compatibility_digest: String,
    pub artifact_provenance_digest: String,
    pub composition_load_mode: GameplayCompositionLoadMode,
    pub compatibility_diagnostics: Vec<GameplayCompositionDiagnostic>,
    pub binding_registry_hash: String,
    pub activation_hash: String,
    pub module_state_hash: String,
    pub authority_state_hash: String,
    pub trigger_revision: u64,
    pub trigger_snapshot_hash: String,
    pub active_overlap_count: u32,
    pub reaction_frame_count: u32,
    pub last_reaction_frame_hash: Option<String>,
    pub decision_receipt_count: u32,
    pub pending_decision_count: u32,
    pub last_decision_receipt_hash: Option<String>,
    pub scheduler: GameplayRuntimeSchedulerReadout,
    pub runtime_host_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayRuntimeReactionReceipt {
    pub observe: GameplayObserveReceipt,
    pub frame: GameplayReactionFrame,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayRuntimeTriggerReceipt {
    pub collision: TriggerReconcileReceipt,
    pub gameplay_events: Vec<GameplayEventEnvelope>,
    pub reactions: Vec<GameplayRuntimeReactionReceipt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameplayRuntimeMovementReceipt {
    pub movement: MovementEvent,
    pub triggers: GameplayRuntimeTriggerReceipt,
}

pub struct GameplayRuntimeHost {
    session: GameplayBoundProjectBundleSession,
    project_configuration_authority: GameplayProjectConfigurationAuthority,
    prefab_registry: ValidatedPrefabRegistry,
    declared_reads: Vec<GameplayRuntimeDeclaredReadPlan>,
    reaction_frames: Vec<GameplayReactionFrame>,
    decision_continuations: GameplayDecisionContinuations,
    decision_receipts: Vec<GameplayDecisionReceipt>,
    scheduler: GameplayActionScheduler,
    authored_program: Option<authored_behavior::AuthoredProgramRuntime>,
    composition_load_mode: GameplayCompositionLoadMode,
    compatibility_diagnostics: Vec<GameplayCompositionDiagnostic>,
    activated_project: Option<project_activation::ValidatedRuntimeProjectState>,
}

impl GameplayRuntimeHost {
    /// Produce the closed ProjectBundle authoring admission view for this
    /// statically composed host. Provider registry and typed-codec resolution
    /// remain in the gameplay RuntimeSession lane.
    pub fn project_content_admission(
        &self,
    ) -> rule_project_bundle::GameplayProjectContentAdmission {
        rule_project_bundle::GameplayProjectContentAdmission::new(
            self.project_configuration_authority.clone(),
        )
    }

    /// Transfer the live entity authority into a surrounding composed
    /// RuntimeSession cell. This integration seam is intentionally ownership
    /// based: no clone or mutable handle can create a shadow authority store.
    #[doc(hidden)]
    pub fn take_entity_authority(&mut self) -> Result<EntityStore, GameplayRuntimeHostError> {
        self.session
            .bundle
            .runtime_entities
            .take()
            .ok_or(GameplayRuntimeHostError::MissingEntityAuthority)
    }

    /// Return the composed cell's sole entity authority for one in-process
    /// gameplay operation. A host may never retain a second store.
    #[doc(hidden)]
    pub fn install_entity_authority(
        &mut self,
        entities: EntityStore,
    ) -> Result<(), GameplayRuntimeHostError> {
        if self.session.bundle.runtime_entities.is_some() {
            return Err(GameplayRuntimeHostError::Snapshot(
                "gameplay host already has entity authority installed".to_owned(),
            ));
        }
        self.session.bundle.runtime_entities = Some(entities);
        Ok(())
    }

    /// Load authored ProjectBundle artifacts, validate one complete prefab
    /// registry, apply every placement atomically in staging, then activate the
    /// closed gameplay topology against the resulting part-role authority.
    pub(crate) fn activate_project_with_prefabs(
        input: RuntimeProjectActivationInput,
        prefabs: GameplayRuntimePrefabBootstrap,
    ) -> Result<Self, GameplayRuntimeHostError> {
        let authored_program = input.authored_program;
        let mut bundle = rule_project_bundle::execute_load_plan_resolved(
            &input.load_plan,
            &input.artifacts,
            &input.bootstrap_resolution,
        )
        .map_err(|error| GameplayRuntimeHostError::Load(format!("{error:?}")))?;
        let (prefab_registry, prefab_scene_instances) =
            apply_prefab_bootstrap(&mut bundle, prefabs)?;
        let mut entity_targets = input.entity_targets;
        for (scene_instance_id, instance, lineage) in prefab_scene_instances {
            entity_targets.bind_validated_prefab_instance(scene_instance_id, instance, lineage);
        }
        Self::activate_with_prefab_registry(
            GameplayRuntimeHostInput {
                bundle,
                composition: input.composition,
                composition_requirement: input.composition_requirement,
                bindings: input.bindings,
                entity_targets,
                spatial_entities: input.spatial_entities,
                declared_reads: input.declared_reads,
                triggers: input.triggers,
                scheduler: input.scheduler,
            },
            prefab_registry,
            authored_program,
        )
    }

    /// Validate the authored prefab source and placement commands before
    /// restoring the saved Session. The snapshot remains authoritative for the
    /// live entity/role map and must match the binding activation evidence.
    pub(crate) fn restore_project_with_prefabs(
        input: RuntimeProjectActivationInput,
        prefabs: GameplayRuntimePrefabBootstrap,
        snapshot_text: &str,
    ) -> Result<Self, GameplayRuntimeHostError> {
        let authored_program = input.authored_program;
        let mut bundle = rule_project_bundle::execute_load_plan_resolved(
            &input.load_plan,
            &input.artifacts,
            &input.bootstrap_resolution,
        )
        .map_err(|error| GameplayRuntimeHostError::Load(format!("{error:?}")))?;
        let (prefab_registry, prefab_scene_instances) =
            apply_prefab_bootstrap(&mut bundle, prefabs)?;
        let mut entity_targets = input.entity_targets;
        for (scene_instance_id, instance, lineage) in prefab_scene_instances {
            entity_targets.bind_validated_prefab_instance(scene_instance_id, instance, lineage);
        }
        Self::restore_with_prefab_registry(
            GameplayRuntimeHostInput {
                bundle,
                composition: input.composition,
                composition_requirement: input.composition_requirement,
                bindings: input.bindings,
                entity_targets,
                spatial_entities: input.spatial_entities,
                declared_reads: input.declared_reads,
                triggers: input.triggers,
                scheduler: input.scheduler,
            },
            snapshot_text,
            prefab_registry,
            authored_program,
        )
    }

    pub fn activate(input: GameplayRuntimeHostInput) -> Result<Self, GameplayRuntimeHostError> {
        Self::activate_with_prefab_registry(input, empty_prefab_registry(), None)
    }

    fn activate_with_prefab_registry(
        mut input: GameplayRuntimeHostInput,
        prefab_registry: ValidatedPrefabRegistry,
        authored_program: Option<authored_behavior::CompiledAuthoredProgram>,
    ) -> Result<Self, GameplayRuntimeHostError> {
        prepare_runtime_entities(&mut input)?;
        let composition_identity =
            gameplay_runtime_composition_identity(input.composition.registry(), &input.bindings);
        let (composition_load_mode, mut compatibility_diagnostics) =
            validate_composition_requirement(
                &composition_identity,
                input.composition_requirement.as_ref(),
            )?;
        let trigger_definitions =
            resolve_trigger_definitions(&input.bundle, core::mem::take(&mut input.triggers))?;
        let project_configuration_authority = input.composition.project_configuration_authority();
        let mut session = GameplayBoundProjectBundleSession::activate_with_mode(
            input.bundle,
            input.composition,
            input.bindings,
            &input.entity_targets,
            composition_load_mode,
        )?;
        compatibility_diagnostics
            .extend(session.activation.compatibility_diagnostics.iter().cloned());
        session.install_trigger_definitions(trigger_definitions)?;
        validate_scheduler_definition(
            session.registry(),
            &input.scheduler,
            authored_program.is_some(),
        )?;
        let scheduler = input.scheduler.build();
        Ok(Self {
            session,
            project_configuration_authority,
            prefab_registry,
            declared_reads: input.declared_reads,
            reaction_frames: Vec::new(),
            decision_continuations: GameplayDecisionContinuations::default(),
            decision_receipts: Vec::new(),
            scheduler,
            authored_program: authored_program
                .map(authored_behavior::AuthoredProgramRuntime::activate),
            composition_load_mode,
            compatibility_diagnostics,
            activated_project: None,
        })
    }

    pub fn restore(
        input: GameplayRuntimeHostInput,
        snapshot_text: &str,
    ) -> Result<Self, GameplayRuntimeHostError> {
        Self::restore_with_prefab_registry(input, snapshot_text, empty_prefab_registry(), None)
    }

    fn restore_with_prefab_registry(
        mut input: GameplayRuntimeHostInput,
        snapshot_text: &str,
        prefab_registry: ValidatedPrefabRegistry,
        authored_program: Option<authored_behavior::CompiledAuthoredProgram>,
    ) -> Result<Self, GameplayRuntimeHostError> {
        prepare_runtime_entities(&mut input)?;
        let stored: StoredGameplayRuntimeHostSnapshot = serde_json::from_str(snapshot_text)
            .map_err(|error| GameplayRuntimeHostError::Snapshot(error.to_string()))?;
        if stored.schema_version != GAMEPLAY_RUNTIME_HOST_SNAPSHOT_VERSION
            || stored.snapshot_hash != gameplay_runtime_snapshot_hash(&stored)
        {
            return Err(GameplayRuntimeHostError::Snapshot(
                "runtime host snapshot version or hash mismatch".to_owned(),
            ));
        }
        let composition_identity =
            gameplay_runtime_composition_identity(input.composition.registry(), &input.bindings);
        let (composition_load_mode, mut compatibility_diagnostics) =
            validate_composition_requirement(
                &composition_identity,
                input.composition_requirement.as_ref(),
            )?;
        if stored.semantic_compatibility_digest
            != composition_identity.semantic_compatibility_digest
            || stored.artifact_provenance_digest != composition_identity.artifact_provenance_digest
        {
            return Err(GameplayRuntimeHostError::Snapshot(
                "saved gameplay producer identity does not match the restoring composition"
                    .to_owned(),
            ));
        }
        let trigger_definitions =
            resolve_trigger_definitions(&input.bundle, core::mem::take(&mut input.triggers))?;
        let project_configuration_authority = input.composition.project_configuration_authority();
        let session = GameplayBoundProjectBundleSession::restore_with_mode(
            input.bundle,
            input.composition,
            input.bindings,
            &input.entity_targets,
            &stored.session_snapshot,
            composition_load_mode,
        )?;
        compatibility_diagnostics
            .extend(session.activation.compatibility_diagnostics.iter().cloned());
        validate_scheduler_definition(
            session.registry(),
            &input.scheduler,
            authored_program.is_some(),
        )?;
        let expected_triggers = rule_trigger_volume::TriggerVolumeRule::new(trigger_definitions)
            .map_err(|error| {
                GameplayRuntimeHostError::Activation(GameplayBindingActivationError::Trigger(
                    error.diagnostics,
                ))
            })?
            .snapshot()
            .definitions;
        if session.trigger_rule().snapshot().definitions != expected_triggers {
            return Err(GameplayRuntimeHostError::Snapshot(
                "authored trigger definitions do not match the saved host".to_owned(),
            ));
        }
        let scheduler = GameplayActionScheduler::decode_snapshot(&stored.scheduler_snapshot)?;
        let expected_scheduler = input.scheduler.build();
        if scheduler.owner() != expected_scheduler.owner()
            || scheduler.declared_events() != expected_scheduler.declared_events()
            || scheduler.declared_proposals() != expected_scheduler.declared_proposals()
        {
            return Err(GameplayRuntimeHostError::Snapshot(
                "authored scheduler definition does not match the saved host".to_owned(),
            ));
        }
        validate_replayed_scheduler_codecs(
            session.registry(),
            &scheduler,
            authored_program.is_some(),
        )?;
        validate_replayed_reaction_frames(session.registry(), &stored.reaction_frames)?;
        validate_replayed_decision_evidence(
            session.registry(),
            &stored.decision_continuations,
            &stored.decision_receipts,
        )?;
        let authored_program = match (authored_program, stored.authored_program) {
            (Some(plan), Some(snapshot)) => Some(
                authored_behavior::AuthoredProgramRuntime::restore(plan, snapshot)
                    .map_err(GameplayRuntimeHostError::Snapshot)?,
            ),
            (None, None) => None,
            _ => {
                return Err(GameplayRuntimeHostError::Snapshot(
                    "saved authored-program presence does not match admitted content".to_owned(),
                ))
            }
        };
        if let Some(program) = &authored_program {
            program
                .validate_scheduler(&scheduler)
                .map_err(GameplayRuntimeHostError::Snapshot)?;
        } else if scheduler
            .pending_actions()
            .iter()
            .any(|action| action.id().as_str().starts_with("authored."))
            || scheduler
                .outstanding_dispatches()
                .iter()
                .any(|dispatch| dispatch.action_id.as_str().starts_with("authored."))
        {
            return Err(GameplayRuntimeHostError::Snapshot(
                "saved authored continuation has no admitted program".to_owned(),
            ));
        }
        Ok(Self {
            session,
            project_configuration_authority,
            prefab_registry,
            declared_reads: input.declared_reads,
            reaction_frames: stored.reaction_frames,
            decision_continuations: stored.decision_continuations,
            decision_receipts: stored.decision_receipts,
            scheduler,
            authored_program,
            composition_load_mode,
            compatibility_diagnostics,
            activated_project: None,
        })
    }

    pub fn observe(
        &mut self,
        event: GameplayEventEnvelope,
    ) -> Result<GameplayRuntimeReactionReceipt, GameplayRuntimeHostError> {
        self.observe_with_source_facts(event, Vec::new())
    }

    /// Deliver one authoritative owner-emitted batch as a single root cascade.
    /// This is the in-process composition seam used by engine rules: TypeScript
    /// never fabricates, orders, or ferries the semantic events.
    pub fn observe_owner_events(
        &mut self,
        events: Vec<GameplayEventEnvelope>,
    ) -> Result<GameplayRuntimeReactionReceipt, GameplayRuntimeHostError> {
        self.observe_routed_events_with_source_facts(events, Vec::new())
    }

    pub fn tick(
        &mut self,
        tick: u64,
    ) -> Result<GameplayRuntimeReactionReceipt, GameplayRuntimeHostError> {
        let due_authored_actions = self
            .scheduler
            .due_action_ids(tick)
            .into_iter()
            .filter(|action_id| action_id.as_str().starts_with("authored."))
            .collect::<Vec<_>>();
        for action_id in due_authored_actions {
            self.scheduler
                .apply(GameplaySchedulerCommand::ExecuteTick {
                    action_id: action_id.clone(),
                    tick,
                    validity: rule_scheduler::ScheduledActionValidity::CURRENT,
                })?;
        }
        let authored_dispatches = self
            .scheduler
            .outstanding_dispatches()
            .into_iter()
            .filter(|dispatch| {
                dispatch.proposal.proposal == authored_behavior::authored_program_step_contract()
            })
            .cloned()
            .collect::<Vec<_>>();
        for dispatch in authored_dispatches {
            let ready = self
                .authored_program
                .as_ref()
                .ok_or_else(|| {
                    GameplayRuntimeHostError::AuthoredProgram(
                        "scheduled authored continuation has no active program".to_owned(),
                    )
                })?
                .continuation_is_ready(
                    &dispatch.proposal,
                    self.session
                        .bundle
                        .runtime_entities
                        .as_ref()
                        .ok_or(GameplayRuntimeHostError::MissingEntityAuthority)?,
                )
                .map_err(GameplayRuntimeHostError::AuthoredProgram)?;
            if ready {
                self.route_scheduled_action(&dispatch.action_id)?;
            }
        }
        let pending_deliveries = self
            .scheduler
            .outstanding_event_deliveries()
            .into_iter()
            .filter(|delivery| delivery.action_id.as_str().starts_with("authored."))
            .map(|delivery| delivery.action_id.clone())
            .collect::<Vec<_>>();
        for action_id in pending_deliveries {
            self.route_scheduled_action(&action_id)?;
        }
        let event = adapt_session_tick(&GameplayOwnerEventContext {
            owner_id: "runtime-session".to_owned(),
            tick,
            root_id: format!("session-tick:{tick}"),
            root_sequence: tick,
            first_event_sequence: 0,
            parent_event_id: None,
        })
        .map_err(|error| GameplayRuntimeHostError::Snapshot(error.to_string()))?;
        self.observe(event)
    }

    /// Execute one closed Guard -> Transform -> React pre-commit transaction.
    /// Continuation authority and evidence remain host-owned; the supplied
    /// owner is a statically linked Rust port for the final atomic route only.
    pub fn decide(
        &mut self,
        moment: GameplayDecisionMoment,
        owner: &mut dyn GameplayRuntimeDecisionOwner,
    ) -> GameplayDecisionReceipt {
        let records_receipt = moment.resume_token.is_none()
            || self
                .decision_continuations
                .pending(&moment.decision_id)
                .is_some();
        let entities = self
            .session
            .bundle
            .runtime_entities
            .as_ref()
            .expect("runtime entity authority initialized");
        let receipt = GameplayFabricCoordinator::new(
            self.session.registry(),
            limits_from_registry(self.session.registry()),
        )
        .decide(
            moment,
            &mut self.decision_continuations,
            &RuntimeSessionViews {
                registry: self.session.registry(),
                module_state: &self.session.module_state,
                entities,
                triggers: self.session.trigger_rule(),
                prefab_registry: &self.prefab_registry,
                prefab_instances: &self.session.bundle.prefab_instances,
                declared_reads: &self.declared_reads,
                scheduler: &self.scheduler,
            },
            self.session.invocation_host(),
            &mut RuntimeSessionDecisionOwner { owner },
        );
        if records_receipt {
            if self.decision_receipts.len() == MAX_DECISION_RECEIPTS {
                self.decision_receipts.remove(0);
            }
            self.decision_receipts.push(receipt.clone());
        }
        receipt
    }

    pub fn reconcile_triggers(
        &mut self,
        tick: u64,
        cause: TriggerReconcileCause,
    ) -> Result<GameplayRuntimeTriggerReceipt, GameplayRuntimeHostError> {
        let (collision, gameplay_events) = self.session.reconcile_trigger_events(tick, cause)?;
        let mut reactions = Vec::with_capacity(gameplay_events.len());
        for (event, fact) in gameplay_events.iter().cloned().zip(&collision.facts) {
            let source_fact = GameplayReactionSourceFact::new(
                "rule-trigger-volume".to_owned(),
                format!("trigger.{:?}", fact.kind),
                serde_json::to_vec(fact).expect("trigger fact serializes"),
            );
            reactions.push(self.observe_with_source_facts(event, vec![source_fact])?);
        }
        Ok(GameplayRuntimeTriggerReceipt {
            collision,
            gameplay_events,
            reactions,
        })
    }

    /// Bind an accepted actor pose to the same EntityStore sampled by trigger
    /// authority, then reconcile at that named movement moment.
    pub fn set_actor_translation_and_reconcile(
        &mut self,
        entity: EntityId,
        translation: [f32; 3],
        tick: u64,
    ) -> Result<GameplayRuntimeTriggerReceipt, GameplayRuntimeHostError> {
        let entities = self
            .session
            .bundle
            .runtime_entities
            .as_mut()
            .ok_or(GameplayRuntimeHostError::MissingEntityAuthority)?;
        let current = entities
            .transform(entity)
            .ok_or(GameplayRuntimeHostError::Transform {
                entity: entity.raw(),
                code: "notTransformEligible",
            })?
            .transform;
        entities
            .apply_transform(TransformCommand::Set {
                id: entity,
                transform: EntityTransform {
                    translation: Vec3::new(translation[0], translation[1], translation[2]),
                    ..current
                },
            })
            .map_err(|error| GameplayRuntimeHostError::Transform {
                entity: entity.raw(),
                code: error.label(),
            })?;
        self.reconcile_triggers(tick, TriggerReconcileCause::Teleport)
    }

    /// Apply collision-constrained authority movement, then sample semantic
    /// trigger overlaps against the accepted (possibly blocked/slid) pose.
    pub fn move_actor_and_reconcile(
        &mut self,
        entity: EntityId,
        delta: [f32; 3],
        tick: u64,
    ) -> Result<GameplayRuntimeMovementReceipt, GameplayRuntimeHostError> {
        let movement = self
            .session
            .bundle
            .runtime_entities
            .as_mut()
            .ok_or(GameplayRuntimeHostError::MissingEntityAuthority)?
            .apply_movement(MovementCommand {
                id: entity,
                delta: Vec3::new(delta[0], delta[1], delta[2]),
            })
            .map_err(|error| GameplayRuntimeHostError::Movement {
                entity: entity.raw(),
                code: error.label(),
            })?;
        let triggers = self.reconcile_triggers(tick, TriggerReconcileCause::Movement)?;
        Ok(GameplayRuntimeMovementReceipt { movement, triggers })
    }

    pub fn compose_snapshot(&self) -> Result<SessionStateArtifact, GameplayRuntimeHostError> {
        let session = self
            .session
            .compose_gameplay_session_snapshot()
            .map_err(GameplayRuntimeHostError::from)?;
        let composition_identity =
            gameplay_runtime_composition_identity(self.session.registry(), self.session.bindings());
        let mut stored = StoredGameplayRuntimeHostSnapshot {
            schema_version: GAMEPLAY_RUNTIME_HOST_SNAPSHOT_VERSION,
            semantic_compatibility_digest: composition_identity.semantic_compatibility_digest,
            artifact_provenance_digest: composition_identity.artifact_provenance_digest,
            session_snapshot: session.text,
            reaction_frames: self.reaction_frames.clone(),
            decision_continuations: self.decision_continuations.clone(),
            decision_receipts: self.decision_receipts.clone(),
            scheduler_snapshot: self.scheduler.encode_snapshot()?,
            authored_program: self
                .authored_program
                .as_ref()
                .map(|program| program.snapshot()),
            snapshot_hash: String::new(),
        };
        stored.snapshot_hash = gameplay_runtime_snapshot_hash(&stored);
        let text = serde_json::to_string(&stored)
            .map_err(|error| GameplayRuntimeHostError::Snapshot(error.to_string()))?;
        let entry = ArtifactEntry::durable(
            GAMEPLAY_RUNTIME_HOST_SNAPSHOT_PATH,
            ArtifactRole::Other("gameplayRuntimeHostSnapshot".to_owned()),
            text.as_bytes(),
        );
        Ok(SessionStateArtifact { entry, text })
    }

    pub fn readout(&self) -> GameplayRuntimeHostReadout {
        let active_overlap_count = self.session.trigger_rule().active_overlaps().count();
        let last_reaction_frame_hash = self
            .reaction_frames
            .last()
            .map(|frame| frame.frame_hash.clone());
        let last_decision_receipt_hash = self
            .decision_receipts
            .last()
            .map(|receipt| receipt.receipt_hash.clone());
        let authority_state_hash = self.current_authority_state_hash();
        let scheduler = self.scheduler_readout();
        let composition_identity =
            gameplay_runtime_composition_identity(self.session.registry(), self.session.bindings());
        let runtime_host_hash = gameplay_module_payload_hash(
            format!(
                "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}",
                self.session.registry().registry_digest(),
                composition_identity.semantic_compatibility_digest,
                self.session.bindings().registry_hash,
                self.session.module_state.state_hash(),
                authority_state_hash,
                self.session.trigger_rule().snapshot().snapshot_hash,
                last_reaction_frame_hash.as_deref().unwrap_or("none"),
                last_decision_receipt_hash.as_deref().unwrap_or("none"),
                self.decision_continuations.pending_count(),
                scheduler.state_hash,
                self.composition_load_mode,
            )
            .as_bytes(),
        );
        GameplayRuntimeHostReadout {
            gameplay_registry_digest: self.session.registry().registry_digest().to_owned(),
            semantic_compatibility_digest: composition_identity.semantic_compatibility_digest,
            artifact_provenance_digest: composition_identity.artifact_provenance_digest,
            composition_load_mode: self.composition_load_mode,
            compatibility_diagnostics: self.compatibility_diagnostics.clone(),
            binding_registry_hash: self.session.bindings().registry_hash.clone(),
            activation_hash: activation_hash(&self.session.activation),
            module_state_hash: self.session.module_state.state_hash(),
            authority_state_hash,
            trigger_revision: self.session.trigger_rule().revision(),
            trigger_snapshot_hash: self.session.trigger_rule().snapshot().snapshot_hash,
            active_overlap_count: u32::try_from(active_overlap_count).unwrap_or(u32::MAX),
            reaction_frame_count: u32::try_from(self.reaction_frames.len()).unwrap_or(u32::MAX),
            last_reaction_frame_hash,
            decision_receipt_count: u32::try_from(self.decision_receipts.len()).unwrap_or(u32::MAX),
            pending_decision_count: u32::try_from(self.decision_continuations.pending_count())
                .unwrap_or(u32::MAX),
            last_decision_receipt_hash,
            scheduler,
            runtime_host_hash,
        }
    }

    fn current_authority_state_hash(&self) -> String {
        let authority = self
            .session
            .bundle
            .compose_session_state_snapshot()
            .expect("runtime host always owns current entity authority");
        gameplay_module_payload_hash(
            format!(
                "{}|{}",
                authority.text,
                self.authored_program
                    .as_ref()
                    .map(authored_behavior::AuthoredProgramRuntime::state_hash)
                    .unwrap_or_else(|| "none".to_owned())
            )
            .as_bytes(),
        )
    }

    pub fn prefab_readout(&self) -> GameplayRuntimePrefabReadout {
        prefab_readout(&self.session.bundle)
    }

    pub fn module_state_readouts(&self) -> Vec<GameplayModuleStateReadout> {
        self.session.module_state.readouts()
    }

    pub fn activation(&self) -> &GameplayModuleBindingActivationReceipt {
        &self.session.activation
    }

    pub fn trigger_diagnostics(
        error: &GameplayRuntimeHostError,
    ) -> Option<&[TriggerVolumeDiagnostic]> {
        match error {
            GameplayRuntimeHostError::Activation(GameplayBindingActivationError::Trigger(
                diagnostics,
            )) => Some(diagnostics),
            _ => None,
        }
    }

    pub fn reaction_frames(&self) -> &[GameplayReactionFrame] {
        &self.reaction_frames
    }

    pub fn decision_receipts(&self) -> &[GameplayDecisionReceipt] {
        &self.decision_receipts
    }

    /// Rust-computed identity of the currently admitted direct authored program.
    pub fn authored_program_hash(&self) -> Option<&str> {
        self.authored_program
            .as_ref()
            .map(authored_behavior::AuthoredProgramRuntime::program_hash)
    }

    /// Bounded diagnostic count of accepted typed owner operations.
    pub fn authored_program_accepted_fact_count(&self) -> u32 {
        self.authored_program
            .as_ref()
            .map(|program| u32::try_from(program.accepted_facts().len()).unwrap_or(u32::MAX))
            .unwrap_or(0)
    }

    fn observe_with_source_facts(
        &mut self,
        event: GameplayEventEnvelope,
        source_facts: Vec<GameplayReactionSourceFact>,
    ) -> Result<GameplayRuntimeReactionReceipt, GameplayRuntimeHostError> {
        self.observe_event_batch_with_source_facts(vec![event], false, source_facts)
    }

    fn observe_routed_events_with_source_facts(
        &mut self,
        events: Vec<GameplayEventEnvelope>,
        source_facts: Vec<GameplayReactionSourceFact>,
    ) -> Result<GameplayRuntimeReactionReceipt, GameplayRuntimeHostError> {
        self.observe_event_batch_with_source_facts(events, true, source_facts)
    }

    fn observe_event_batch_with_source_facts(
        &mut self,
        mut events: Vec<GameplayEventEnvelope>,
        routed: bool,
        source_facts: Vec<GameplayReactionSourceFact>,
    ) -> Result<GameplayRuntimeReactionReceipt, GameplayRuntimeHostError> {
        let state_hash_before = self.session.module_state.state_hash();
        let module_state_checkpoint = self.session.module_state.checkpoint();
        let scheduler_checkpoint = self.scheduler.clone();
        let authored_program_checkpoint = self.authored_program.clone();
        let mut authority_entities = self
            .session
            .bundle
            .runtime_entities
            .take()
            .expect("runtime entity authority initialized");
        let authority_before = authority_entities.clone();
        if let Some(program) = self.authored_program.as_mut() {
            if let Err(error) = program.react(
                self.session.registry(),
                &events,
                &mut authority_entities,
                &mut self.scheduler,
            ) {
                self.session.bundle.runtime_entities = Some(authority_entities);
                return Err(GameplayRuntimeHostError::AuthoredProgram(error));
            }
        }
        let observe = {
            let cells = self.session.runtime_cells();
            let coordinator = GameplayFabricCoordinator::new(
                cells.registry,
                limits_from_registry(cells.registry),
            );
            let mut authority = RuntimeSessionWaveAuthority {
                registry: cells.registry,
                module_state: cells.module_state,
                entities: &mut authority_entities,
                triggers: cells.triggers,
                prefab_registry: &self.prefab_registry,
                prefab_instances: cells.prefab_instances,
                declared_reads: &self.declared_reads,
                scheduler: &self.scheduler,
            };
            if routed {
                coordinator.observe_routed_events_transactional(
                    events,
                    &mut authority,
                    cells.invocation_host,
                )
            } else {
                let event = events
                    .pop()
                    .expect("single root-event delivery is nonempty");
                coordinator.observe_transactional(event, &mut authority, cells.invocation_host)
            }
        };
        if !observe.accepted() {
            authority_entities = authority_before;
            self.session
                .module_state
                .restore_checkpoint(module_state_checkpoint);
            self.scheduler = scheduler_checkpoint;
            self.authored_program = authored_program_checkpoint;
        }
        self.session.bundle.runtime_entities = Some(authority_entities);
        let mut accepted_facts = Vec::new();
        if observe.accepted() {
            accepted_facts.clone_from(&observe.module_facts);
        }
        let state_hash_after = self.session.module_state.state_hash();
        let final_session_hash = self
            .session
            .module_state
            .final_session_hash(&self.session.activation.receipt_hash);
        let frame = GameplayReactionFrame::from_observe(
            self.session.registry(),
            &observe,
            source_facts,
            &accepted_facts,
            state_hash_before,
            state_hash_after,
            final_session_hash,
        );
        if self.reaction_frames.len() == MAX_REACTION_FRAMES {
            self.reaction_frames.remove(0);
        }
        self.reaction_frames.push(frame.clone());
        Ok(GameplayRuntimeReactionReceipt { observe, frame })
    }
}

fn validate_replayed_reaction_frames(
    registry: &svc_gameplay_fabric::GameplayFabricRegistry,
    frames: &[GameplayReactionFrame],
) -> Result<(), GameplayRuntimeHostError> {
    if frames.len() > MAX_REACTION_FRAMES {
        return Err(GameplayRuntimeHostError::Snapshot(
            "reaction frame count exceeds the durable host limit".to_owned(),
        ));
    }
    for (frame_index, frame) in frames.iter().enumerate() {
        if frame.registry_digest != registry.registry_digest()
            || frame.frame_hash != frame.canonical_hash()
        {
            return Err(GameplayRuntimeHostError::Snapshot(format!(
                "reaction frame {frame_index} registry or canonical hash mismatch"
            )));
        }
        for (kind, events) in [
            ("root", frame.root_events.as_slice()),
            ("delivered", frame.delivered_events.as_slice()),
        ] {
            for (event_index, event) in events.iter().enumerate() {
                registry.admit_event(event).map_err(|error| {
                    GameplayRuntimeHostError::Snapshot(format!(
                        "reaction frame {frame_index} {kind} event {event_index} failed codec admission: {error}"
                    ))
                })?;
            }
        }
    }
    Ok(())
}

fn validate_replayed_decision_evidence(
    registry: &svc_gameplay_fabric::GameplayFabricRegistry,
    continuations: &GameplayDecisionContinuations,
    receipts: &[GameplayDecisionReceipt],
) -> Result<(), GameplayRuntimeHostError> {
    if receipts.len() > MAX_DECISION_RECEIPTS
        || !continuations.snapshot_is_valid(registry.registry_digest())
    {
        return Err(GameplayRuntimeHostError::Snapshot(
            "decision continuation table or receipt count is invalid".to_owned(),
        ));
    }
    for (receipt_index, receipt) in receipts.iter().enumerate() {
        if receipt.registry_digest != registry.registry_digest()
            || !receipt.nested_hashes_are_valid()
        {
            return Err(GameplayRuntimeHostError::Snapshot(format!(
                "decision receipt {receipt_index} registry or nested hash mismatch"
            )));
        }
    }
    let represented_pending = receipts
        .iter()
        .filter(|receipt| {
            let Some(pending) = continuations.pending(&receipt.decision_id) else {
                return false;
            };
            receipt
                .continuation
                .as_ref()
                .is_some_and(|recorded| recorded == pending)
        })
        .map(|receipt| receipt.decision_id.as_str())
        .collect::<BTreeSet<_>>();
    if represented_pending.len() != continuations.pending_count() {
        return Err(GameplayRuntimeHostError::Snapshot(
            "pending continuation has no matching suspended decision receipt".to_owned(),
        ));
    }
    Ok(())
}

fn validate_composition_requirement(
    identity: &GameplayRuntimeCompositionIdentity,
    requirement: Option<&GameplayCompositionRequirement>,
) -> Result<
    (
        GameplayCompositionLoadMode,
        Vec<GameplayCompositionDiagnostic>,
    ),
    GameplayRuntimeHostError,
> {
    let Some(requirement) = requirement else {
        return Ok((
            GameplayCompositionLoadMode::Compatible,
            vec![GameplayCompositionDiagnostic {
                code: GameplayCompositionDiagnosticCode::LegacyCompatibilityDefaulted,
                severity: DiagnosticSeverity::Warning,
                path: "gameplayRuntime.compositionRequirement".to_owned(),
                expected: None,
                actual: Some(identity.semantic_compatibility_digest.clone()),
                message: "legacy ProjectBundle has no explicit composition requirement; compatible mode was selected"
                    .to_owned(),
            }],
        ));
    };

    let semantic_actual = &identity.semantic_compatibility_digest;
    if requirement.semantic_compatibility_digest != semantic_actual.as_str() {
        return Err(GameplayRuntimeHostError::Compatibility(vec![
            GameplayCompositionDiagnostic {
                code: GameplayCompositionDiagnosticCode::SemanticCompatibilityMismatch,
                severity: DiagnosticSeverity::Error,
                path: "gameplayRuntime.compositionRequirement.semanticCompatibilityDigest"
                    .to_owned(),
                expected: Some(requirement.semantic_compatibility_digest.clone()),
                actual: Some(semantic_actual.clone()),
                message: "authored gameplay semantic compatibility identity does not match the linked composition"
                    .to_owned(),
            },
        ]));
    }

    let artifact_actual = identity.artifact_provenance_digest.as_str();
    match (
        requirement.load_mode,
        requirement.artifact_provenance_digest.as_deref(),
    ) {
        (GameplayCompositionLoadMode::Exact, None) => {
            Err(GameplayRuntimeHostError::Compatibility(vec![
                GameplayCompositionDiagnostic {
                    code: GameplayCompositionDiagnosticCode::MissingExactArtifactProvenance,
                    severity: DiagnosticSeverity::Error,
                    path: "gameplayRuntime.compositionRequirement.artifactProvenanceDigest"
                        .to_owned(),
                    expected: None,
                    actual: Some(artifact_actual.to_owned()),
                    message: "exact composition mode requires authored artifact provenance"
                        .to_owned(),
                },
            ]))
        }
        (_, Some(expected)) if expected != artifact_actual => {
            let diagnostic = GameplayCompositionDiagnostic {
                code: GameplayCompositionDiagnosticCode::ArtifactProvenanceMismatch,
                severity: if requirement.load_mode == GameplayCompositionLoadMode::Exact {
                    DiagnosticSeverity::Error
                } else {
                    DiagnosticSeverity::Warning
                },
                path: "gameplayRuntime.compositionRequirement.artifactProvenanceDigest"
                    .to_owned(),
                expected: Some(expected.to_owned()),
                actual: Some(artifact_actual.to_owned()),
                message: "linked artifact provenance differs while semantic compatibility remains unchanged"
                    .to_owned(),
            };
            if requirement.load_mode == GameplayCompositionLoadMode::Exact {
                Err(GameplayRuntimeHostError::Compatibility(vec![diagnostic]))
            } else {
                Ok((requirement.load_mode, vec![diagnostic]))
            }
        }
        _ => Ok((requirement.load_mode, Vec::new())),
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredGameplayRuntimeHostSnapshot {
    schema_version: u32,
    semantic_compatibility_digest: String,
    artifact_provenance_digest: String,
    session_snapshot: String,
    reaction_frames: Vec<GameplayReactionFrame>,
    decision_continuations: GameplayDecisionContinuations,
    decision_receipts: Vec<GameplayDecisionReceipt>,
    scheduler_snapshot: Vec<u8>,
    authored_program: Option<authored_behavior::AuthoredProgramSnapshot>,
    snapshot_hash: String,
}

fn gameplay_runtime_snapshot_hash(snapshot: &StoredGameplayRuntimeHostSnapshot) -> String {
    let frames = snapshot
        .reaction_frames
        .iter()
        .map(|frame| frame.frame_hash.as_str())
        .collect::<Vec<_>>()
        .join("|");
    let decisions = snapshot
        .decision_receipts
        .iter()
        .map(|receipt| receipt.receipt_hash.as_str())
        .collect::<Vec<_>>()
        .join("|");
    let continuations = serde_json::to_string(&snapshot.decision_continuations)
        .expect("decision continuations serialize");
    gameplay_module_payload_hash(
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}",
            snapshot.schema_version,
            snapshot.semantic_compatibility_digest,
            snapshot.artifact_provenance_digest,
            snapshot.session_snapshot,
            frames,
            decisions,
            continuations,
            gameplay_module_payload_hash(&snapshot.scheduler_snapshot),
            snapshot
                .authored_program
                .as_ref()
                .map(|program| gameplay_module_payload_hash(
                    &serde_json::to_vec(program).expect("authored snapshot serializes")
                ))
                .unwrap_or_else(|| "none".to_owned()),
        )
        .as_bytes(),
    )
}

struct RuntimeSessionViews<'a> {
    registry: &'a svc_gameplay_fabric::GameplayFabricRegistry,
    module_state: &'a rule_gameplay_fabric::GameplayModuleStateStore,
    entities: &'a EntityStore,
    triggers: &'a rule_trigger_volume::TriggerVolumeRule,
    prefab_registry: &'a ValidatedPrefabRegistry,
    prefab_instances: &'a rule_project_bundle::PrefabInstanceAuthority,
    declared_reads: &'a [GameplayRuntimeDeclaredReadPlan],
    scheduler: &'a GameplayActionScheduler,
}

struct RuntimeSessionWaveAuthority<'a> {
    registry: &'a svc_gameplay_fabric::GameplayFabricRegistry,
    module_state: &'a mut rule_gameplay_fabric::GameplayModuleStateStore,
    entities: &'a mut EntityStore,
    triggers: &'a rule_trigger_volume::TriggerVolumeRule,
    prefab_registry: &'a ValidatedPrefabRegistry,
    prefab_instances: &'a rule_project_bundle::PrefabInstanceAuthority,
    declared_reads: &'a [GameplayRuntimeDeclaredReadPlan],
    scheduler: &'a GameplayActionScheduler,
}

impl RuntimeSessionWaveAuthority<'_> {
    fn views(&self) -> RuntimeSessionViews<'_> {
        RuntimeSessionViews {
            registry: self.registry,
            module_state: self.module_state,
            entities: self.entities,
            triggers: self.triggers,
            prefab_registry: self.prefab_registry,
            prefab_instances: self.prefab_instances,
            declared_reads: self.declared_reads,
            scheduler: self.scheduler,
        }
    }

    fn prefab_state_hash(&self) -> String {
        gameplay_module_payload_hash(
            format!(
                "{}|{}",
                encode_prefab_registry(self.prefab_registry),
                self.prefab_instances.state_hash(self.entities)
            )
            .as_bytes(),
        )
    }
}

impl GameplayWaveAuthority for RuntimeSessionWaveAuthority<'_> {
    fn freeze(&self, root_id: &str, wave: u32) -> FrozenGameplayViews {
        self.views().freeze(root_id, wave)
    }

    fn freeze_declared_reads(
        &self,
        module_id: &str,
        invocation_id: &str,
        event: &GameplayEventEnvelope,
    ) -> Result<Option<GameplayFrozenReadSet>, GameplayReadAssemblyError> {
        self.views()
            .freeze_declared_reads(module_id, invocation_id, event)
    }

    fn route(
        &mut self,
        call: &rule_gameplay_fabric::GameplayOwnerRoutingCall,
    ) -> rule_gameplay_fabric::GameplayOwnerRoutingOutput {
        rule_gameplay_fabric::GameplayProposalRouter::route(
            &mut RuntimeSessionOwnerRouter {
                entities: self.entities,
            },
            call,
        )
    }

    fn apply_module_facts_atomic(
        &mut self,
        facts: &[rule_gameplay_fabric::GameplayModuleFact],
    ) -> Result<(), GameplayHostError> {
        self.module_state
            .apply_facts_atomic(facts)
            .map_err(|error| GameplayHostError {
                code: "moduleFactApplyFailed".to_owned(),
                message: error.to_string(),
            })
    }

    fn state_hashes(&self) -> GameplayWaveStateHashes {
        GameplayWaveStateHashes {
            authority_state_hash: gameplay_module_payload_hash(
                format!("{}|{}", self.entities.hash().0, self.scheduler.state_hash()).as_bytes(),
            ),
            module_state_hash: self.module_state.state_hash(),
            prefab_state_hash: self.prefab_state_hash(),
            trigger_state_hash: self.triggers.snapshot().snapshot_hash,
        }
    }
}

impl RuntimeSessionViews<'_> {
    fn assemble_declared_reads(
        &self,
        module_id: &str,
        invocation_id: &str,
        event: &GameplayEventEnvelope,
    ) -> Result<Option<GameplayFrozenReadSet>, GameplayReadAssemblyError> {
        let matching = self
            .declared_reads
            .iter()
            .filter(|plan| plan.module_id == module_id && plan.invocation_id == invocation_id)
            .collect::<Vec<_>>();
        if matching.is_empty() {
            let requires_reads = self
                .registry
                .module(module_id)
                .and_then(|module| {
                    module
                        .invocations
                        .iter()
                        .find(|invocation| invocation.invocation_id == invocation_id)
                })
                .is_some_and(|invocation| !invocation.read_requirements.is_empty());
            if requires_reads {
                return Err(read_assembly_error(
                    "missingPlan",
                    "the invocation declares reads but no matching runtime read plan was supplied",
                ));
            }
            return Ok(None);
        }
        if matching.len() != 1 {
            return Err(read_assembly_error(
                "duplicatePlan",
                "a module invocation must have exactly one declared read plan",
            ));
        }
        let plan = GameplayReadPlan {
            module_id: module_id.to_owned(),
            invocation_id: invocation_id.to_owned(),
            event_id: event.event_id.clone(),
            wave: event.wave,
            requests: matching[0].requests.clone(),
        };
        let invocation = self
            .registry
            .module(module_id)
            .and_then(|module| {
                module
                    .invocations
                    .iter()
                    .find(|invocation| invocation.invocation_id == invocation_id)
            })
            .expect("coordinator invocation is closed-registry topology");
        let expected_topology = invocation
            .read_requirements
            .iter()
            .map(|requirement| (requirement.request_id.as_str(), &requirement.view))
            .collect::<BTreeSet<_>>();
        let runtime_topology = plan
            .requests
            .iter()
            .map(|request| (request.request_id.as_str(), &request.view))
            .collect::<BTreeSet<_>>();
        if runtime_topology != expected_topology {
            return Err(read_assembly_error(
                "topologyDrift",
                "the runtime read plan does not exactly match the invocation's typed read requirements",
            ));
        }
        let provider_ids = plan
            .requests
            .iter()
            .filter_map(|request| match &request.selector {
                GameplayReadSelector::OwnerQuery { .. } => self
                    .registry
                    .read_view_provider(&request.view)
                    .map(|provider| provider.provider_id.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let trigger_providers = provider_ids
            .into_iter()
            .map(|provider_id| GameplayTriggerOverlapQueryProvider::new(provider_id, self.triggers))
            .collect::<Vec<_>>();
        let owner_query_providers = trigger_providers
            .iter()
            .map(|provider| provider as &dyn GameplayOwnerQueryProvider)
            .collect();
        let mut prefab_instances = GameplayPrefabInstanceIndex::default();
        for instance in self.prefab_instances.instances() {
            prefab_instances
                .insert(
                    instance.record.instance,
                    GameplayPrefabInstanceBinding {
                        prefab: instance.record.prefab,
                        part_entities: instance
                            .parts
                            .iter()
                            .map(|part| (part.part, part.entity))
                            .collect::<BTreeMap<_, _>>(),
                    },
                )
                .expect("validated prefab authority has unique instance ids");
        }
        let mut scopes = GameplayEntityScopeIndex::default();
        for definition in self.triggers.definitions() {
            scopes.bind(definition.scope.clone(), EntityId::new(definition.trigger));
        }
        GameplayReadAssembler::new(
            self.registry,
            self.entities,
            self.module_state,
            self.prefab_registry,
            &prefab_instances,
            &scopes,
            owner_query_providers,
        )?
        .assemble(&plan, event)
        .map(Some)
    }
}

fn empty_prefab_registry() -> ValidatedPrefabRegistry {
    ValidatedPrefabRegistry::new(
        PrefabRegistry {
            schema_version: PREFAB_REGISTRY_SCHEMA_VERSION,
            definitions: Vec::new(),
        },
        &PrefabRegistryValidationContext::default(),
    )
    .expect("empty prefab registry is valid")
}

impl GameplayViewSource for RuntimeSessionViews<'_> {
    fn freeze(&self, root_id: &str, wave: u32) -> FrozenGameplayViews {
        FrozenGameplayViews {
            epoch: u64::from(wave),
            view_hash: gameplay_module_payload_hash(
                format!(
                    "{}|{}|{}|{}|{}|{}|{}|{}",
                    self.registry.registry_digest(),
                    self.module_state.state_hash(),
                    self.entities.hash().0,
                    gameplay_module_payload_hash(
                        format!(
                            "{}|{}",
                            encode_prefab_registry(self.prefab_registry),
                            self.prefab_instances.state_hash(self.entities)
                        )
                        .as_bytes()
                    ),
                    self.triggers.snapshot().snapshot_hash,
                    self.scheduler.state_hash(),
                    root_id,
                    wave
                )
                .as_bytes(),
            ),
        }
    }

    fn freeze_declared_reads(
        &self,
        module_id: &str,
        invocation_id: &str,
        event: &GameplayEventEnvelope,
    ) -> Result<Option<GameplayFrozenReadSet>, GameplayReadAssemblyError> {
        self.assemble_declared_reads(module_id, invocation_id, event)
    }

    fn freeze_declared_decision_reads(
        &self,
        module_id: &str,
        invocation_id: &str,
        moment: &GameplayDecisionMoment,
    ) -> Result<Option<GameplayFrozenReadSet>, GameplayReadAssemblyError> {
        let event = GameplayEventEnvelope {
            event_id: format!("decision-read:{}", moment.decision_id),
            event: moment.operation.proposal.clone(),
            tick: moment.operation.tick,
            root_sequence: moment.operation.root_sequence,
            wave: moment.operation.wave,
            event_sequence: moment.operation.proposal_sequence,
            phase: GameplayEventPhase::DecisionMoment,
            emitter: moment.operation.emitter.clone(),
            causation: moment.operation.causation.clone(),
            source: moment.operation.source.clone(),
            subjects: Vec::new(),
            targets: moment.operation.targets.clone(),
            scope: None,
            tags: Vec::new(),
            payload_hash: moment.operation.payload_hash.clone(),
            canonical_payload: moment.operation.canonical_payload.clone(),
        };
        self.assemble_declared_reads(module_id, invocation_id, &event)
    }
}

fn read_assembly_error(request_id: &str, message: &str) -> GameplayReadAssemblyError {
    GameplayReadAssemblyError {
        diagnostics: vec![GameplayReadDiagnostic {
            code: GameplayReadDiagnosticCode::DuplicateRequest,
            request_id: request_id.to_owned(),
            message: message.to_owned(),
        }],
    }
}

fn limits_from_registry(
    registry: &svc_gameplay_fabric::GameplayFabricRegistry,
) -> GameplayRuntimeLimits {
    let mut limits = GameplayRuntimeLimits {
        max_waves: 1,
        max_events_per_root: 1,
        max_proposals_per_root: 1,
        max_invocations_per_root: 1,
        max_payload_bytes_per_root: 1,
    };
    for module_id in registry.module_order() {
        let budget = &registry
            .module(module_id)
            .expect("closed module order")
            .budget;
        limits.max_waves = limits.max_waves.max(budget.max_waves);
        limits.max_events_per_root = limits
            .max_events_per_root
            .saturating_add(budget.max_events_per_root);
        limits.max_proposals_per_root = limits
            .max_proposals_per_root
            .saturating_add(budget.max_proposals_per_root);
        limits.max_invocations_per_root = limits
            .max_invocations_per_root
            .saturating_add(budget.max_invocations_per_root);
        limits.max_payload_bytes_per_root = limits
            .max_payload_bytes_per_root
            .saturating_add(budget.max_payload_bytes_per_root);
    }
    limits
}

fn prepare_runtime_entities(
    input: &mut GameplayRuntimeHostInput,
) -> Result<(), GameplayRuntimeHostError> {
    if input.bundle.runtime_entities.is_none() {
        input.bundle.runtime_entities = Some(EntityStore::from_snapshot(
            input.bundle.spatial_session.entity_snapshot(),
        ));
    }
    install_spatial_entities(
        input
            .bundle
            .runtime_entities
            .as_mut()
            .expect("runtime entity authority initialized"),
        core::mem::take(&mut input.spatial_entities),
    )
}

fn resolve_trigger_definitions(
    bundle: &ProjectBundleLoadResult,
    definitions: Vec<GameplayTriggerDefinition>,
) -> Result<Vec<rule_trigger_volume::KinematicTriggerDefinition>, GameplayRuntimeHostError> {
    definitions
        .into_iter()
        .map(|definition| {
            if definition.schema_version != GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION {
                return Err(GameplayRuntimeHostError::Snapshot(format!(
                    "trigger {} uses unsupported schema version {}",
                    definition.scene_instance_id, definition.schema_version
                )));
            }
            let resolved = bundle
                .bootstrap
                .resolved_instances
                .iter()
                .find(|instance| instance.instance_id == definition.scene_instance_id)
                .ok_or_else(|| {
                    GameplayRuntimeHostError::Snapshot(format!(
                        "trigger target scene instance {} does not resolve",
                        definition.scene_instance_id
                    ))
                })?;
            let entities = bundle
                .runtime_entities
                .as_ref()
                .ok_or(GameplayRuntimeHostError::MissingEntityAuthority)?;
            let bounds = entities.bounds(resolved.entity).ok_or_else(|| {
                GameplayRuntimeHostError::Snapshot(format!(
                    "trigger target scene instance {} has no collision bounds",
                    definition.scene_instance_id
                ))
            })?;
            if bounds.bounds.min.x >= bounds.bounds.max.x
                || bounds.bounds.min.y >= bounds.bounds.max.y
                || bounds.bounds.min.z >= bounds.bounds.max.z
            {
                return Err(GameplayRuntimeHostError::Snapshot(format!(
                    "trigger target scene instance {} has unusable collision bounds",
                    definition.scene_instance_id
                )));
            }
            let collision = entities.collision(resolved.entity).ok_or_else(|| {
                GameplayRuntimeHostError::Snapshot(format!(
                    "trigger target scene instance {} has no collision capability",
                    definition.scene_instance_id
                ))
            })?;
            if collision.static_collider {
                return Err(GameplayRuntimeHostError::Snapshot(format!(
                    "trigger target scene instance {} must use a non-static collision body",
                    definition.scene_instance_id
                )));
            }
            Ok(rule_trigger_volume::KinematicTriggerDefinition::new(
                resolved.entity,
                definition.scope,
                definition.tags,
            ))
        })
        .collect()
}

fn install_spatial_entities(
    entities: &mut EntityStore,
    definitions: Vec<GameplayRuntimeSpatialEntity>,
) -> Result<(), GameplayRuntimeHostError> {
    for definition in definitions {
        if !definition
            .translation
            .into_iter()
            .chain(definition.half_extents)
            .all(f32::is_finite)
            || definition.half_extents.into_iter().any(|axis| axis <= 0.0)
        {
            return Err(GameplayRuntimeHostError::SpatialEntity {
                entity: definition.entity.raw(),
                code: "invalidGeometry",
            });
        }
        if !entities.contains(definition.entity) {
            entities
                .apply(EntityLifecycleCommand::Create {
                    id: definition.entity,
                    source: EntitySource::RuntimeCreated { by: None },
                    labels: Vec::new(),
                })
                .map_err(|_| GameplayRuntimeHostError::SpatialEntity {
                    entity: definition.entity.raw(),
                    code: "createRejected",
                })?;
        }
        let translation = Vec3::new(
            definition.translation[0],
            definition.translation[1],
            definition.translation[2],
        );
        let half_extents = Vec3::new(
            definition.half_extents[0],
            definition.half_extents[1],
            definition.half_extents[2],
        );
        let negative_half_extents = Vec3::new(-half_extents.x, -half_extents.y, -half_extents.z);
        if !entities.attach_transform(definition.entity, EntityTransform::at(translation))
            || !entities.attach_bounds(
                definition.entity,
                Aabb::new(negative_half_extents, half_extents),
            )
            || !entities.attach_collision(definition.entity, definition.static_collider)
        {
            return Err(GameplayRuntimeHostError::SpatialEntity {
                entity: definition.entity.raw(),
                code: "capabilityAttachRejected",
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_entity::{Aabb, EntityLifecycleCommand, EntitySource};
    use core_ids::SceneNodeId;
    use core_scene::{
        encode, SceneEntityInstance, SceneEntityReference, SceneMetadata, SceneNode, SceneNodeKind,
        SceneTree,
    };
    use gameplay_module_sdk::*;
    use protocol_game_extension::{
        GameplayInvocationReadRequirement, GameplayModuleBinding, GameplayModuleBindingTarget,
        GameplayModuleConfiguration,
    };
    use serde::{Deserialize, Serialize};

    fn emit_integration_evidence(
        phase: &str,
        session: u64,
        wave_or_action: &str,
        readout: &GameplayRuntimeHostReadout,
        evidence_hashes: &[&str],
    ) {
        let artifact = serde_json::json!({
            "schemaVersion": 1,
            "phase": phase,
            "session": session,
            "waveOrAction": wave_or_action,
            "registryDigest": readout.gameplay_registry_digest,
            "runtimeHostHash": readout.runtime_host_hash,
            "evidenceHashes": evidence_hashes.iter().take(8).collect::<Vec<_>>(),
        });
        eprintln!("ASHA_GAMEPLAY_RUNTIME_HOST_EVIDENCE={artifact}");
    }

    pub(super) fn empty_scheduler_definition() -> GameplayRuntimeSchedulerDefinition {
        GameplayRuntimeSchedulerDefinition::new(
            GameplayOwnerRef {
                owner_id: "authority.fixture-scheduler".to_owned(),
                provider_id: "provider.fixture-scheduler".to_owned(),
            },
            Vec::new(),
            Vec::new(),
        )
    }

    const WAVE_FIXTURE_MODULE_ID: &str = "fixture.wave-barrier.module";

    fn fixture_schema_descriptor(namespace: &str, name: &str) -> String {
        format!("fixture:{namespace}.{name};canonical-json-v1")
    }

    fn fixture_declaration(event: GameplayContractRef) -> GameplayEventSchemaDeclaration {
        GameplayEventSchemaDeclaration {
            codec_id: gameplay_canonical_codec_id(&event.schema_hash),
            event,
        }
    }

    fn fixture_json_codec<T>(event: GameplayContractRef) -> TypedGameplayEventCodec<T>
    where
        T: Serialize + for<'de> Deserialize<'de> + 'static,
    {
        let descriptor = fixture_schema_descriptor(&event.namespace, &event.name);
        TypedGameplayEventCodec::new(
            fixture_declaration(event),
            descriptor,
            |value: &T| serde_json::to_vec(value).map_err(|error| error.to_string()),
            |bytes| serde_json::from_slice(bytes).map_err(|error| error.to_string()),
        )
    }

    fn test_provenance() -> GameplayModuleBuildProvenance {
        GameplayModuleBuildProvenance::from_build_inputs(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            &[include_bytes!("lib.rs")],
            include_bytes!("../../../../Cargo.lock"),
            &[],
        )
    }

    fn wave_fixture_contract(name: &str) -> GameplayContractRef {
        gameplay_contract(
            "fixture.wave-barrier",
            name,
            1,
            &fixture_schema_descriptor("fixture.wave-barrier", name),
        )
    }

    fn wave_fixture_owner() -> GameplayOwnerRef {
        GameplayOwnerRef {
            owner_id: "authority.fixture-wave-barrier".to_owned(),
            provider_id: "provider.fixture-wave-barrier".to_owned(),
        }
    }

    fn wave_fixture_base_module_ref() -> GameplayModuleRef {
        GameplayModuleRef {
            module_id: WAVE_FIXTURE_MODULE_ID.to_owned(),
            namespace: "fixture.wave-barrier".to_owned(),
            version: "1.0.0".to_owned(),
            sdk_hash: "sha256:gameplay-sdk-v1".to_owned(),
            contract_hash: "sha256:fixture-wave-barrier-contract".to_owned(),
            artifact_hash: "sha256:fixture-wave-barrier-artifact".to_owned(),
            provider_id: wave_fixture_owner().provider_id,
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct WaveFixtureConfiguration {
        initial_value: u64,
    }

    struct WaveFixtureStateAdapter;

    impl GameplayTypedModuleStateAdapter for WaveFixtureStateAdapter {
        type Config = WaveFixtureConfiguration;
        type State = u64;
        type Fact = u64;
        type View = u64;

        fn module_id(&self) -> &str {
            WAVE_FIXTURE_MODULE_ID
        }

        fn state_schema(&self) -> &GameplayContractRef {
            static VALUE: std::sync::OnceLock<GameplayContractRef> = std::sync::OnceLock::new();
            VALUE.get_or_init(|| wave_fixture_contract("state"))
        }

        fn fact_schema(&self) -> &GameplayContractRef {
            static VALUE: std::sync::OnceLock<GameplayContractRef> = std::sync::OnceLock::new();
            VALUE.get_or_init(|| wave_fixture_contract("fact"))
        }

        fn owner(&self) -> &GameplayOwnerRef {
            static VALUE: std::sync::OnceLock<GameplayOwnerRef> = std::sync::OnceLock::new();
            VALUE.get_or_init(wave_fixture_owner)
        }

        fn decode_config(&self, bytes: &[u8]) -> Result<Self::Config, String> {
            serde_json::from_slice(bytes).map_err(|error| error.to_string())
        }

        fn initialize(&self, config: &Self::Config) -> Result<Self::State, String> {
            Ok(config.initial_value)
        }

        fn decode_state(&self, bytes: &[u8]) -> Result<Self::State, String> {
            serde_json::from_slice(bytes).map_err(|error| error.to_string())
        }

        fn encode_state(&self, state: &Self::State) -> Result<Vec<u8>, String> {
            serde_json::to_vec(state).map_err(|error| error.to_string())
        }

        fn decode_fact(&self, bytes: &[u8]) -> Result<Self::Fact, String> {
            serde_json::from_slice(bytes).map_err(|error| error.to_string())
        }

        fn apply_fact(
            &self,
            state: &Self::State,
            fact: &Self::Fact,
        ) -> Result<Self::State, String> {
            Ok(state.saturating_add(*fact))
        }

        fn migrate(&self, _from_version: u32, state: &Self::State) -> Result<Self::State, String> {
            Ok(*state)
        }

        fn view_schema(&self) -> Option<&GameplayContractRef> {
            static VALUE: std::sync::OnceLock<GameplayContractRef> = std::sync::OnceLock::new();
            Some(VALUE.get_or_init(|| wave_fixture_contract("view")))
        }

        fn project_view(&self, state: &Self::State) -> Result<Self::View, String> {
            Ok(*state)
        }

        fn encode_view(&self, view: &Self::View) -> Result<Vec<u8>, String> {
            serde_json::to_vec(view).map_err(|error| error.to_string())
        }
    }

    struct WaveFixtureBehavior;

    impl GameplayModuleBehavior for WaveFixtureBehavior {
        fn invoke(
            &self,
            context: &GameplayModuleContext<'_>,
        ) -> Result<GameplayModuleActions, GameplayModuleError> {
            let current_revision: u64 = context.event_payload()?;
            if current_revision > 0 {
                let prior_wave_state: u64 = context.named_view("prior-module-state")?;
                if prior_wave_state != 1 {
                    return Err(GameplayModuleError {
                        code: "priorWaveStateNotVisible".to_owned(),
                        message: format!("expected module state 1, got {prior_wave_state}"),
                    });
                }
            }
            let mut actions = context.actions();
            if current_revision == 0 {
                actions.record_local_fact_json(
                    wave_fixture_contract("fact"),
                    wave_fixture_contract("state"),
                    GameplayModuleStateScope::Session,
                    current_revision,
                    &1_u64,
                )?;
            }
            actions.emit(
                &fixture_json_codec::<u64>(wave_fixture_contract("loop")),
                &current_revision.saturating_add(1),
                None,
                Vec::new(),
                Vec::new(),
            )?;
            Ok(actions)
        }
    }

    fn wave_fixture_manifest() -> GameplayModuleManifest {
        let owner = wave_fixture_owner();
        let invocation = |invocation_id: &str, input_contract: GameplayContractRef| {
            GameplayInvocationDescriptor {
                invocation_id: invocation_id.to_owned(),
                family: GameplayInvocationFamily::Observe,
                input_contract,
                output_contract: wave_fixture_contract("loop"),
                read_requirements: if invocation_id.ends_with("observe-loop") {
                    vec![GameplayInvocationReadRequirement {
                        request_id: "prior-module-state".to_owned(),
                        view: wave_fixture_contract("view"),
                    }]
                } else {
                    Vec::new()
                },
                max_outputs: 2,
                max_payload_bytes: 1_024,
            }
        };
        let subscription =
            |subscription_id: &str, event: GameplayContractRef, invocation_id: &str| {
                GameplaySubscriptionDeclaration {
                    subscription_id: subscription_id.to_owned(),
                    event,
                    invocation_id: invocation_id.to_owned(),
                    selector: GameplayHeaderSelector {
                        source: None,
                        target: None,
                        scope: None,
                        required_tags: Vec::new(),
                    },
                    max_deliveries_per_root: 4,
                }
            };
        let mut manifest = GameplayModuleManifest {
            module_ref: wave_fixture_base_module_ref(),
            published_events: vec![
                fixture_declaration(wave_fixture_contract("root")),
                fixture_declaration(wave_fixture_contract("loop")),
            ],
            subscriptions: vec![
                subscription(
                    "fixture.wave-barrier.root",
                    wave_fixture_contract("root"),
                    "fixture.wave-barrier.observe-root",
                ),
                subscription(
                    "fixture.wave-barrier.loop",
                    wave_fixture_contract("loop"),
                    "fixture.wave-barrier.observe-loop",
                ),
            ],
            invocations: vec![
                invocation(
                    "fixture.wave-barrier.observe-root",
                    wave_fixture_contract("root"),
                ),
                invocation(
                    "fixture.wave-barrier.observe-loop",
                    wave_fixture_contract("loop"),
                ),
            ],
            read_views: vec![GameplayReadViewRequirement {
                view: wave_fixture_contract("view"),
                provider_id: owner.provider_id.clone(),
                kind: GameplayReadViewKind::ModuleNamed,
                fields: vec!["value".to_owned()],
                selector_capabilities: vec![GameplayReadSelectorCapability::ModuleStateScope],
                max_items: 1,
            }],
            proposal_kinds: Vec::new(),
            state_schemas: vec![GameplayOwnedSchemaDeclaration {
                schema: wave_fixture_contract("state"),
                owner: owner.clone(),
            }],
            fact_schemas: vec![GameplayOwnedSchemaDeclaration {
                schema: wave_fixture_contract("fact"),
                owner: owner.clone(),
            }],
            ordering: Vec::new(),
            budget: GameplayExecutionBudget {
                max_waves: 2,
                max_events_per_root: 8,
                max_proposals_per_root: 1,
                max_invocations_per_root: 8,
                max_payload_bytes_per_root: 8_192,
            },
            deterministic_requirements: vec!["canonical-json".to_owned()],
            source_hash: "sha256:fixture-wave-barrier-source".to_owned(),
        };
        test_provenance().apply_to_manifest::<WaveFixtureBehavior>(&mut manifest);
        manifest
    }

    fn wave_fixture_module_ref() -> GameplayModuleRef {
        wave_fixture_manifest().module_ref
    }

    fn wave_fixture_provider() -> GameplayStaticModuleProvider {
        let owner = wave_fixture_owner();
        let manifest = wave_fixture_manifest();
        let configuration_metadata = GameplayConfigurationSchemaMetadata {
            module_id: WAVE_FIXTURE_MODULE_ID.to_owned(),
            configuration: wave_fixture_contract("configuration"),
            codec_id: "codec.fixture-wave-barrier.configuration".to_owned(),
            fields: vec![GameplayConfigurationFieldMetadata {
                name: "initialValue".to_owned(),
                label: "Initial value".to_owned(),
                value_kind: GameplayConfigurationValueKind::Integer,
                required: true,
                reference_kind: None,
                integer_min: Some(0),
                integer_max: None,
                number_min: None,
                number_max: None,
            }],
        };
        let event_codec = |event: GameplayContractRef, _legacy_codec_id: &str| {
            GameplayEventCodecRegistration::typed(fixture_json_codec::<u64>(event))
        };
        GameplayStaticModuleProvider::linked_from_manifest(
            manifest,
            &test_provenance(),
            WaveFixtureBehavior,
        )
        .event_codec(event_codec(
            wave_fixture_contract("root"),
            "codec.fixture-wave-barrier.root",
        ))
        .event_codec(event_codec(
            wave_fixture_contract("loop"),
            "codec.fixture-wave-barrier.loop",
        ))
        .state_owner(GameplayStateOwnerRegistration {
            schema: wave_fixture_contract("state"),
            owner: owner.clone(),
        })
        .state_owner(GameplayStateOwnerRegistration {
            schema: wave_fixture_contract("fact"),
            owner,
        })
        .state_adapter(GameplayModuleStateRegistration::typed(
            WaveFixtureStateAdapter,
        ))
        .read_view_provider(GameplayReadViewProviderRegistration {
            view: wave_fixture_contract("view"),
            provider_id: wave_fixture_owner().provider_id,
            kind: GameplayReadViewKind::ModuleNamed,
            fields: vec!["value".to_owned()],
            selector_capabilities: vec![GameplayReadSelectorCapability::ModuleStateScope],
            max_items: 1,
            ordering: "singleValue".to_owned(),
        })
        .configuration_schema(configuration_metadata.clone())
        .configuration_codec(GameplayConfigurationCodecRegistration::typed::<
            WaveFixtureConfiguration,
        >(configuration_metadata))
    }

    fn wave_fixture_host_input() -> GameplayRuntimeHostInput {
        let canonical_config = serde_json::to_vec(&WaveFixtureConfiguration { initial_value: 0 })
            .expect("wave fixture configuration serializes");
        let configuration = GameplayModuleConfiguration {
            configuration_id: "fixture.wave-barrier.default".to_owned(),
            module: wave_fixture_module_ref(),
            configuration: wave_fixture_contract("configuration"),
            codec_id: "codec.fixture-wave-barrier.configuration".to_owned(),
            config_hash: gameplay_module_payload_hash(&canonical_config),
            canonical_config,
        };
        let binding = GameplayModuleBinding {
            binding_id: "fixture.wave-barrier.session".to_owned(),
            module_id: WAVE_FIXTURE_MODULE_ID.to_owned(),
            configuration_id: configuration.configuration_id.clone(),
            state_schema: wave_fixture_contract("state"),
            target: GameplayModuleBindingTarget::Session,
            required_reads: Vec::new(),
            output_contracts: vec![wave_fixture_contract("loop")],
            enabled: true,
        };
        let mut bindings = GameplayModuleBindingRegistryBuilder::new();
        bindings.configuration(configuration).binding(binding);
        let mut composition = GameplayStaticCompositionBuilder::new();
        composition.add_provider(wave_fixture_provider());
        GameplayRuntimeHostInput {
            bundle: bundle(),
            composition: composition.build().expect("wave fixture composition"),
            composition_requirement: None,
            bindings: bindings.build(),
            entity_targets: GameplayBindingEntityTargets::new(),
            spatial_entities: Vec::new(),
            declared_reads: vec![GameplayRuntimeDeclaredReadPlan {
                module_id: WAVE_FIXTURE_MODULE_ID.to_owned(),
                invocation_id: "fixture.wave-barrier.observe-loop".to_owned(),
                requests: vec![GameplayReadRequest {
                    request_id: "prior-module-state".to_owned(),
                    view: wave_fixture_contract("view"),
                    fields: vec!["value".to_owned()],
                    selector: GameplayReadSelector::ModuleNamed {
                        scope: GameplayModuleStateScope::Session,
                    },
                }],
            }],
            triggers: Vec::new(),
            scheduler: empty_scheduler_definition(),
        }
    }

    fn wave_fixture_root_event() -> GameplayEventEnvelope {
        let canonical_payload = serde_json::to_vec(&0_u64).expect("root payload serializes");
        GameplayEventEnvelope {
            event_id: "fixture.wave-barrier.root-event".to_owned(),
            event: wave_fixture_contract("root"),
            tick: 1,
            root_sequence: 1,
            wave: 0,
            event_sequence: 0,
            phase: GameplayEventPhase::PostCommit,
            emitter: protocol_game_extension::GameplayEmitterRef::Owner {
                owner_id: wave_fixture_owner().owner_id,
            },
            causation: protocol_game_extension::GameplayCausationRef {
                root_id: "fixture.wave-barrier.root".to_owned(),
                parent_event_id: None,
                decision_id: None,
            },
            source: None,
            subjects: Vec::new(),
            targets: Vec::new(),
            scope: None,
            tags: Vec::new(),
            payload_hash: gameplay_canonical_payload_hash(&canonical_payload),
            canonical_payload,
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DecisionWorkspace {
        amount: u64,
        transformed: bool,
    }

    struct DecisionBehavior;

    impl GameplayModuleBehavior for DecisionBehavior {
        fn invoke(
            &self,
            context: &GameplayModuleContext<'_>,
        ) -> Result<GameplayModuleActions, GameplayModuleError> {
            let mut workspace: DecisionWorkspace = context.decision_workspace()?;
            let mut actions = context.actions();
            match context.invocation_id() {
                "fixture.decision.transform" => {
                    if context.read("target-collision").is_none() {
                        return Err(GameplayModuleError {
                            code: "missingTargetCollisionRead".to_owned(),
                            message: "decision transform requires its declared target read"
                                .to_owned(),
                        });
                    }
                    if !workspace.transformed {
                        workspace.amount = workspace.amount.saturating_add(3);
                        workspace.transformed = true;
                    }
                    actions.transform_workspace_json(
                        decision_contract("workspace"),
                        context
                            .decision_workspace_hash()
                            .expect("decision Workspace hash"),
                        &workspace,
                    )?;
                }
                "fixture.decision.react" if context.decision_resume_token().is_none() => {
                    actions.react(
                        GameplayReactionDisposition::Suspend {
                            token: "fixture-reaction-window".to_owned(),
                        },
                        None,
                    );
                }
                "fixture.decision.react" => {
                    actions.react(GameplayReactionDisposition::Continue, None);
                }
                _ => {
                    return Err(GameplayModuleError {
                        code: "unexpectedInvocation".to_owned(),
                        message: context.invocation_id().to_owned(),
                    });
                }
            }
            Ok(actions)
        }
    }

    fn decision_contract(name: &str) -> GameplayContractRef {
        gameplay_contract(
            "fixture.decision",
            name,
            1,
            &fixture_schema_descriptor("fixture.decision", name),
        )
    }

    fn decision_owner_ref() -> GameplayOwnerRef {
        GameplayOwnerRef {
            owner_id: "authority.fixture-decision".to_owned(),
            provider_id: "provider.fixture-decision".to_owned(),
        }
    }

    fn decision_provider() -> GameplayStaticModuleProvider {
        let proposal = decision_contract("operation");
        let view = decision_contract("target-collision-view");
        let owner = decision_owner_ref();
        let mut manifest = GameplayModuleManifest {
            module_ref: GameplayModuleRef {
                module_id: "fixture.decision.module".to_owned(),
                namespace: "fixture.decision".to_owned(),
                version: "1.0.0".to_owned(),
                sdk_hash: "sha256:gameplay-sdk-v1".to_owned(),
                contract_hash: "sha256:fixture-decision-contract".to_owned(),
                artifact_hash: "sha256:fixture-decision-artifact".to_owned(),
                provider_id: "provider.fixture-decision".to_owned(),
            },
            published_events: Vec::new(),
            subscriptions: Vec::new(),
            invocations: vec![
                GameplayInvocationDescriptor {
                    invocation_id: "fixture.decision.transform".to_owned(),
                    family: GameplayInvocationFamily::Transform,
                    input_contract: proposal.clone(),
                    output_contract: decision_contract("workspace"),
                    read_requirements: vec![GameplayInvocationReadRequirement {
                        request_id: "target-collision".to_owned(),
                        view: view.clone(),
                    }],
                    max_outputs: 1,
                    max_payload_bytes: 4_096,
                },
                GameplayInvocationDescriptor {
                    invocation_id: "fixture.decision.react".to_owned(),
                    family: GameplayInvocationFamily::React,
                    input_contract: proposal.clone(),
                    output_contract: decision_contract("workspace"),
                    read_requirements: Vec::new(),
                    max_outputs: 1,
                    max_payload_bytes: 4_096,
                },
            ],
            read_views: vec![GameplayReadViewRequirement {
                view: view.clone(),
                provider_id: "provider.fixture-decision".to_owned(),
                kind: GameplayReadViewKind::EntityCapability,
                fields: vec!["staticCollider".to_owned()],
                selector_capabilities: vec![
                    GameplayReadSelectorCapability::EventTarget,
                    GameplayReadSelectorCapability::CollisionCapability,
                ],
                max_items: 1,
            }],
            proposal_kinds: vec![GameplayProposalDeclaration {
                proposal: proposal.clone(),
                owner: owner.clone(),
            }],
            state_schemas: Vec::new(),
            fact_schemas: Vec::new(),
            ordering: Vec::new(),
            budget: GameplayExecutionBudget {
                max_waves: 4,
                max_events_per_root: 8,
                max_proposals_per_root: 4,
                max_invocations_per_root: 12,
                max_payload_bytes_per_root: 16_384,
            },
            deterministic_requirements: vec!["canonical-json".to_owned()],
            source_hash: "sha256:fixture-decision-source".to_owned(),
        };
        test_provenance().apply_to_manifest::<DecisionBehavior>(&mut manifest);
        GameplayStaticModuleProvider::linked_from_manifest(
            manifest,
            &test_provenance(),
            DecisionBehavior,
        )
        .proposal_codec(GameplayEventCodecRegistration::typed(fixture_json_codec::<
            DecisionWorkspace,
        >(
            proposal.clone()
        )))
        .proposal_owner(GameplayProposalOwnerRegistration { proposal, owner })
        .read_view_provider(GameplayReadViewProviderRegistration {
            view,
            provider_id: "provider.fixture-decision".to_owned(),
            kind: GameplayReadViewKind::EntityCapability,
            fields: vec!["staticCollider".to_owned()],
            selector_capabilities: vec![
                GameplayReadSelectorCapability::EventTarget,
                GameplayReadSelectorCapability::CollisionCapability,
            ],
            max_items: 1,
            ordering: "entityIdAscending".to_owned(),
        })
    }

    #[derive(Default)]
    pub(super) struct DecisionOwnerFixture {
        revision: u64,
        committed_payloads: Vec<Vec<u8>>,
    }

    impl GameplayRuntimeDecisionOwner for DecisionOwnerFixture {
        fn revision_hash(&self, owner: &GameplayOwnerRef) -> String {
            assert_eq!(owner, &decision_owner_ref());
            format!("revision:{}", self.revision)
        }

        fn route_precommit(
            &mut self,
            owner: &GameplayOwnerRef,
            operation: &GameplayProposalEnvelope,
        ) -> GameplayRuntimeDecisionOwnerOutput {
            assert_eq!(owner, &decision_owner_ref());
            self.committed_payloads
                .push(operation.canonical_payload.clone());
            self.revision = self.revision.saturating_add(1);
            GameplayRuntimeDecisionOwnerOutput {
                accepted: true,
                fact_hashes: vec![gameplay_module_payload_hash(&operation.canonical_payload)],
                ..GameplayRuntimeDecisionOwnerOutput::default()
            }
        }
    }

    pub(super) fn bundle() -> ProjectBundleLoadResult {
        let scene = SceneTree {
            id: SceneId::new(1),
            schema_version: 4,
            metadata: SceneMetadata {
                name: Some("host-fixture".to_owned()),
                authoring_format_version: 4,
            },
            dependencies: Vec::new(),
            roots: vec![
                SceneNode::leaf(
                    SceneNodeId::new(10),
                    SceneNodeKind::EntityInstance(SceneEntityInstance {
                        instance_id: "fixture.host.trigger".to_owned(),
                        reference: SceneEntityReference::EntityDefinition {
                            stable_id: "fixture/host-trigger".to_owned(),
                        },
                        spawn_marker_id: None,
                    }),
                ),
                SceneNode::leaf(
                    SceneNodeId::new(20),
                    SceneNodeKind::EntityInstance(SceneEntityInstance {
                        instance_id: "fixture.host.subject".to_owned(),
                        reference: SceneEntityReference::EntityDefinition {
                            stable_id: "fixture/host-subject".to_owned(),
                        },
                        spawn_marker_id: None,
                    }),
                ),
            ],
        };
        let plan = LoadPlan {
            steps: vec![
                LoadStep::ValidateVersions {
                    bundle_schema_version: 2,
                    protocol_version: 1,
                },
                LoadStep::LoadAssetLock {
                    artifact: "assets/lock.json".to_owned(),
                    asset_count: 0,
                },
                LoadStep::LoadSceneDocument {
                    artifact: "scene/scene.json".to_owned(),
                    scene: SceneId::new(1),
                },
                LoadStep::BootstrapScene {
                    scene: SceneId::new(1),
                    runtime_session: RuntimeSessionId::new(1),
                },
                LoadStep::ValidateFinalState,
            ],
        };
        let artifacts = BundleArtifacts::new()
            .with_artifact("assets/lock.json", "{ \"entries\": [] }\n")
            .with_artifact("scene/scene.json", encode(&scene.to_flat()));
        let resolution = core_scene::BootstrapResolutionContext {
            entity_definition_ids: [
                "fixture/host-trigger".to_owned(),
                "fixture/host-subject".to_owned(),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        rule_project_bundle::execute_load_plan_resolved(&plan, &artifacts, &resolution).unwrap()
    }

    pub(super) fn create_spatial(
        bundle: &mut ProjectBundleLoadResult,
        entity: EntityId,
        x: f32,
        static_collider: bool,
    ) {
        let entities = bundle.runtime_entities.get_or_insert_default();
        entities
            .apply(EntityLifecycleCommand::Create {
                id: entity,
                source: EntitySource::RuntimeCreated { by: None },
                labels: Vec::new(),
            })
            .unwrap();
        entities.attach_transform(entity, EntityTransform::at(Vec3::new(x, 0.0, 0.0)));
        entities.attach_bounds(entity, Aabb::new(Vec3::splat(-0.5), Vec3::splat(0.5)));
        entities.attach_collision(entity, static_collider);
    }

    pub(super) fn decision_host_input() -> GameplayRuntimeHostInput {
        let mut bundle = bundle();
        create_spatial(&mut bundle, EntityId::new(20), 0.0, true);
        let mut composition = GameplayStaticCompositionBuilder::new();
        composition.add_provider(decision_provider());
        GameplayRuntimeHostInput {
            bundle,
            composition: composition.build().expect("decision composition"),
            composition_requirement: None,
            bindings: GameplayModuleBindingRegistryBuilder::new().build(),
            entity_targets: GameplayBindingEntityTargets::new(),
            spatial_entities: Vec::new(),
            declared_reads: vec![GameplayRuntimeDeclaredReadPlan {
                module_id: "fixture.decision.module".to_owned(),
                invocation_id: "fixture.decision.transform".to_owned(),
                requests: vec![GameplayReadRequest {
                    request_id: "target-collision".to_owned(),
                    view: decision_contract("target-collision-view"),
                    fields: vec!["staticCollider".to_owned()],
                    selector: GameplayReadSelector::Capability {
                        binding: GameplayEventEntityBinding::Target { index: 0 },
                        capability: GameplayCapabilityReadKind::Collision,
                    },
                }],
            }],
            triggers: Vec::new(),
            scheduler: empty_scheduler_definition(),
        }
    }

    pub(super) fn scheduler_host_input() -> GameplayRuntimeHostInput {
        scheduler_host_input_for("authority.fixture-scheduler")
    }

    fn scheduler_host_input_for(owner_id: &str) -> GameplayRuntimeHostInput {
        let mut bundle = bundle();
        create_spatial(&mut bundle, EntityId::new(10), 0.0, true);
        let mut composition = GameplayStaticCompositionBuilder::new();
        composition.include_standard_owner_events();
        GameplayRuntimeHostInput {
            bundle,
            composition: composition.build().expect("scheduler composition"),
            composition_requirement: None,
            bindings: GameplayModuleBindingRegistryBuilder::new().build(),
            entity_targets: GameplayBindingEntityTargets::new(),
            spatial_entities: Vec::new(),
            declared_reads: Vec::new(),
            triggers: Vec::new(),
            scheduler: GameplayRuntimeSchedulerDefinition::new(
                GameplayOwnerRef {
                    owner_id: owner_id.to_owned(),
                    provider_id: format!("provider.{owner_id}"),
                },
                Vec::new(),
                vec![
                    rule_gameplay_fabric::StandardGameplayProposalKind::SetCapabilityActivation
                        .contract(),
                ],
            ),
        }
    }

    pub(super) fn scheduled_collision_deactivation() -> TickScheduledActionDraft {
        let payload = rule_gameplay_fabric::CapabilityActivationGameplayProposal {
            entity: 10,
            capability: "collision".to_owned(),
            action: "deactivate".to_owned(),
        };
        let canonical_payload = serde_json::to_vec(&payload).expect("proposal serializes");
        TickScheduledActionDraft {
            id: ScheduledActionId::new("fixture.scheduler.deactivate-collision"),
            execute_at: 5,
            priority: 0,
            proposal: GameplayProposalEnvelope {
                proposal_id: "draft.scheduler.deactivate-collision".to_owned(),
                proposal:
                    rule_gameplay_fabric::StandardGameplayProposalKind::SetCapabilityActivation
                        .contract(),
                tick: 0,
                root_sequence: 5,
                wave: 0,
                proposal_sequence: 0,
                emitter: protocol_game_extension::GameplayEmitterRef::Owner {
                    owner_id: "authority.fixture".to_owned(),
                },
                causation: protocol_game_extension::GameplayCausationRef {
                    root_id: "fixture.scheduler.root".to_owned(),
                    parent_event_id: None,
                    decision_id: None,
                },
                originating_event_id: None,
                source: None,
                targets: vec![protocol_game_extension::GameplayEntityRef {
                    entity: EntityId::new(10),
                }],
                payload_hash: gameplay_canonical_payload_hash(&canonical_payload),
                canonical_payload,
            },
            source: protocol_game_extension::GameplayEmitterRef::Owner {
                owner_id: "authority.fixture".to_owned(),
            },
            causation: protocol_game_extension::GameplayCausationRef {
                root_id: "fixture.scheduler.root".to_owned(),
                parent_event_id: None,
                decision_id: None,
            },
        }
    }

    pub(super) fn decision_moment(
        decision_id: &str,
        owner_revision: u64,
    ) -> GameplayDecisionMoment {
        let payload = serde_json::to_vec(&DecisionWorkspace {
            amount: 4,
            transformed: false,
        })
        .expect("Workspace serializes");
        let workspace = GameplayOperationWorkspace::from_payload(
            decision_contract("workspace"),
            payload.clone(),
        );
        GameplayDecisionMoment {
            decision_id: decision_id.to_owned(),
            operation: GameplayProposalEnvelope {
                proposal_id: format!("proposal-{decision_id}"),
                proposal: decision_contract("operation"),
                tick: 1,
                root_sequence: 1,
                wave: 0,
                proposal_sequence: 0,
                emitter: GameplayEmitterRef::Owner {
                    owner_id: "rulebench.fixture".to_owned(),
                },
                causation: GameplayCausationRef {
                    root_id: decision_id.to_owned(),
                    parent_event_id: None,
                    decision_id: Some(decision_id.to_owned()),
                },
                originating_event_id: None,
                source: Some(GameplayEntityRef {
                    entity: EntityId::new(10),
                }),
                targets: vec![GameplayEntityRef {
                    entity: EntityId::new(20),
                }],
                canonical_payload: payload.clone(),
                payload_hash: gameplay_canonical_payload_hash(&payload),
            },
            expected_owner_revision: format!("revision:{owner_revision}"),
            workspace,
            resume_token: None,
        }
    }

    fn composition_requirement(
        input: &GameplayRuntimeHostInput,
        load_mode: GameplayCompositionLoadMode,
        artifact_provenance_digest: Option<String>,
    ) -> GameplayCompositionRequirement {
        let identity =
            gameplay_runtime_composition_identity(input.composition.registry(), &input.bindings);
        GameplayCompositionRequirement {
            load_mode,
            semantic_compatibility_digest: identity.semantic_compatibility_digest,
            artifact_provenance_digest,
        }
    }

    #[test]
    fn composition_load_policy_warns_for_legacy_and_benign_drift_but_exact_rejects() {
        let legacy = GameplayRuntimeHost::activate(wave_fixture_host_input()).unwrap();
        assert!(legacy
            .readout()
            .compatibility_diagnostics
            .iter()
            .any(|item| {
                item.code == GameplayCompositionDiagnosticCode::LegacyCompatibilityDefaulted
                    && item.severity == DiagnosticSeverity::Warning
            }));

        let mut compatible = wave_fixture_host_input();
        compatible.composition_requirement = Some(composition_requirement(
            &compatible,
            GameplayCompositionLoadMode::Compatible,
            Some("fnv1a64:0000000000000000".to_owned()),
        ));
        let compatible = GameplayRuntimeHost::activate(compatible).unwrap();
        let compatible_readout = compatible.readout();
        assert_eq!(
            compatible_readout.composition_load_mode,
            GameplayCompositionLoadMode::Compatible
        );
        assert!(compatible_readout
            .compatibility_diagnostics
            .iter()
            .any(|item| {
                item.code == GameplayCompositionDiagnosticCode::ArtifactProvenanceMismatch
                    && item.severity == DiagnosticSeverity::Warning
            }));
        assert_ne!(
            compatible_readout.artifact_provenance_digest,
            compatible_readout.gameplay_registry_digest
        );

        let mut exact = wave_fixture_host_input();
        exact.composition_requirement = Some(composition_requirement(
            &exact,
            GameplayCompositionLoadMode::Exact,
            Some("fnv1a64:0000000000000000".to_owned()),
        ));
        assert!(matches!(
            GameplayRuntimeHost::activate(exact),
            Err(GameplayRuntimeHostError::Compatibility(diagnostics))
                if diagnostics.iter().any(|item| {
                    item.code == GameplayCompositionDiagnosticCode::ArtifactProvenanceMismatch
                        && item.severity == DiagnosticSeverity::Error
                })
        ));

        let mut exact_match = wave_fixture_host_input();
        let exact_artifact = gameplay_runtime_composition_identity(
            exact_match.composition.registry(),
            &exact_match.bindings,
        )
        .artifact_provenance_digest;
        exact_match.composition_requirement = Some(composition_requirement(
            &exact_match,
            GameplayCompositionLoadMode::Exact,
            Some(exact_artifact),
        ));
        assert_eq!(
            GameplayRuntimeHost::activate(exact_match)
                .unwrap()
                .readout()
                .composition_load_mode,
            GameplayCompositionLoadMode::Exact
        );

        let mut missing_exact = wave_fixture_host_input();
        missing_exact.composition_requirement = Some(composition_requirement(
            &missing_exact,
            GameplayCompositionLoadMode::Exact,
            None,
        ));
        assert!(matches!(
            GameplayRuntimeHost::activate(missing_exact),
            Err(GameplayRuntimeHostError::Compatibility(diagnostics))
                if diagnostics.iter().any(|item| {
                    item.code == GameplayCompositionDiagnosticCode::MissingExactArtifactProvenance
                })
        ));

        let mut semantic_mismatch = wave_fixture_host_input();
        let artifact = gameplay_runtime_composition_identity(
            semantic_mismatch.composition.registry(),
            &semantic_mismatch.bindings,
        )
        .artifact_provenance_digest;
        semantic_mismatch.composition_requirement = Some(GameplayCompositionRequirement {
            load_mode: GameplayCompositionLoadMode::Compatible,
            semantic_compatibility_digest: "fnv1a64:0000000000000000".to_owned(),
            artifact_provenance_digest: Some(artifact),
        });
        assert!(matches!(
            GameplayRuntimeHost::activate(semantic_mismatch),
            Err(GameplayRuntimeHostError::Compatibility(diagnostics))
                if diagnostics.iter().any(|item| {
                    item.code == GameplayCompositionDiagnosticCode::SemanticCompatibilityMismatch
                })
        ));

        let baseline = wave_fixture_host_input();
        let expected =
            composition_requirement(&baseline, GameplayCompositionLoadMode::Compatible, None);
        let mut binding_mismatch = wave_fixture_host_input();
        binding_mismatch.bindings.bindings[0].enabled = false;
        binding_mismatch.bindings.registry_hash =
            gameplay_module_sdk::gameplay_module_binding_registry_hash(&binding_mismatch.bindings);
        binding_mismatch.composition_requirement = Some(expected);
        assert!(matches!(
            GameplayRuntimeHost::activate(binding_mismatch),
            Err(GameplayRuntimeHostError::Compatibility(diagnostics))
                if diagnostics.iter().any(|item| {
                    item.code == GameplayCompositionDiagnosticCode::SemanticCompatibilityMismatch
                })
        ));
    }

    #[test]
    fn public_decision_host_delivers_reads_persists_continuations_and_consumes_tokens() {
        let mut host = GameplayRuntimeHost::activate(decision_host_input()).unwrap();
        let mut owner = DecisionOwnerFixture::default();

        let suspended = host.decide(decision_moment("decision-1", 0), &mut owner);
        assert_eq!(suspended.status, GameplayDecisionStatus::Suspended);
        assert_eq!(owner.committed_payloads.len(), 0);
        assert_eq!(suspended.invocations.len(), 2);
        assert!(suspended.invocations[0]
            .declared_read_set_hash
            .as_deref()
            .is_some_and(|hash| hash.starts_with("fnv1a64:")));
        let continuation = suspended.continuation.clone().expect("continuation");
        assert_eq!(host.readout().pending_decision_count, 1);

        let mut missing = decision_moment("decision-1", 0);
        missing.workspace = continuation.workspace.clone();
        let missing_receipt = host.decide(missing, &mut owner);
        assert_eq!(missing_receipt.status, GameplayDecisionStatus::Failed);
        assert!(missing_receipt.invocations.is_empty());
        assert_eq!(owner.committed_payloads.len(), 0);

        let mut wrong = decision_moment("decision-1", 0);
        wrong.workspace = continuation.workspace.clone();
        wrong.resume_token = Some("wrong-token".to_owned());
        let wrong_receipt = host.decide(wrong, &mut owner);
        assert_eq!(wrong_receipt.status, GameplayDecisionStatus::Failed);
        assert!(wrong_receipt.invocations.is_empty());

        let snapshot = host.compose_snapshot().unwrap();
        let mut restored = GameplayRuntimeHost::restore(decision_host_input(), &snapshot.text)
            .expect("pending decision restores");
        assert_eq!(restored.readout().pending_decision_count, 1);
        assert_eq!(restored.decision_receipts().len(), 3);

        let mut resumed = decision_moment("decision-1", 0);
        resumed.workspace = continuation.workspace.clone();
        resumed.resume_token = Some(continuation.token.clone());
        let accepted = restored.decide(resumed, &mut owner);
        emit_integration_evidence(
            "continuation-resumed",
            1,
            "decision:decision-1",
            &restored.readout(),
            &[
                suspended.receipt_hash.as_str(),
                accepted.receipt_hash.as_str(),
            ],
        );
        assert_eq!(accepted.status, GameplayDecisionStatus::Accepted);
        assert_eq!(owner.committed_payloads.len(), 1);
        let committed: DecisionWorkspace =
            serde_json::from_slice(&owner.committed_payloads[0]).unwrap();
        assert_eq!(
            committed,
            DecisionWorkspace {
                amount: 7,
                transformed: true,
            }
        );
        assert_eq!(restored.readout().pending_decision_count, 0);

        let mut replayed = decision_moment("decision-1", 0);
        replayed.workspace = continuation.workspace;
        replayed.resume_token = Some(continuation.token);
        let before_replay = restored.readout();
        let frames_before_replay = restored.reaction_frames().to_vec();
        let replayed_receipt = restored.decide(replayed, &mut owner);
        assert_eq!(replayed_receipt.status, GameplayDecisionStatus::Failed);
        assert!(replayed_receipt.invocations.is_empty());
        assert_eq!(owner.committed_payloads.len(), 1);
        assert_eq!(restored.readout(), before_replay);
        assert_eq!(restored.reaction_frames(), frames_before_replay);

        let suspended_stale = restored.decide(decision_moment("decision-2", 1), &mut owner);
        let stale_continuation = suspended_stale.continuation.expect("stale continuation");
        owner.revision = 2;
        let mut stale_resume = decision_moment("decision-2", 1);
        stale_resume.workspace = stale_continuation.workspace.clone();
        stale_resume.resume_token = Some(stale_continuation.token.clone());
        let stale = restored.decide(stale_resume, &mut owner);
        assert_eq!(stale.status, GameplayDecisionStatus::Stale);
        assert!(stale.invocations.is_empty());
        assert_eq!(owner.committed_payloads.len(), 1);

        owner.revision = 1;
        let mut stale_replay = decision_moment("decision-2", 1);
        stale_replay.workspace = stale_continuation.workspace;
        stale_replay.resume_token = Some(stale_continuation.token);
        let unavailable = restored.decide(stale_replay, &mut owner);
        assert_eq!(unavailable.status, GameplayDecisionStatus::Failed);
        assert!(unavailable.invocations.is_empty());

        let final_snapshot = restored.compose_snapshot().unwrap();
        let final_restored =
            GameplayRuntimeHost::restore(decision_host_input(), &final_snapshot.text)
                .expect("decision evidence restores");
        assert_eq!(
            final_restored.readout().last_decision_receipt_hash,
            restored.readout().last_decision_receipt_hash
        );
        assert_eq!(
            final_restored.decision_receipts(),
            restored.decision_receipts()
        );
    }

    #[test]
    fn later_wave_rejection_restores_pre_root_module_state_and_barrier_evidence() {
        let mut host = GameplayRuntimeHost::activate(wave_fixture_host_input()).unwrap();
        let state_before = host.readout().module_state_hash;

        let reaction = host
            .observe_with_source_facts(wave_fixture_root_event(), Vec::new())
            .expect("Observe rejection is represented by its reaction receipt");

        assert!(!reaction.observe.accepted());
        assert!(
            reaction.observe.diagnostics.iter().any(|diagnostic| {
                diagnostic.code
                    == rule_gameplay_fabric::GameplayRuntimeDiagnosticCode::WaveBudgetExceeded
            }),
            "{:#?}",
            reaction.observe.diagnostics
        );
        assert_eq!(reaction.observe.wave_barriers.len(), 2);
        assert_eq!(
            reaction.observe.wave_barriers[0].state_after,
            reaction.observe.wave_barriers[1].state_before
        );
        assert_ne!(
            reaction.observe.wave_barriers[0]
                .state_before
                .module_state_hash,
            reaction.observe.wave_barriers[1]
                .state_after
                .module_state_hash
        );

        assert_eq!(host.readout().module_state_hash, state_before);
        assert_eq!(reaction.frame.state_hash_before, state_before);
        assert_eq!(reaction.frame.state_hash_after, state_before);
        let state = host.module_state_readouts();
        assert_eq!(state.len(), 1);
        assert_eq!(state[0].revision, 0);

        let snapshot = host.compose_snapshot().expect("rejected frame snapshots");
        let restored = GameplayRuntimeHost::restore(wave_fixture_host_input(), &snapshot.text)
            .expect("rejected frame restores");
        emit_integration_evidence(
            "root-rollback",
            1,
            "wave:1",
            &restored.readout(),
            &[
                reaction.frame.frame_hash.as_str(),
                reaction.frame.wave_barriers[0].barrier_hash.as_str(),
                reaction.frame.wave_barriers[1].barrier_hash.as_str(),
            ],
        );
        assert_eq!(restored.readout().module_state_hash, state_before);
        assert_eq!(
            restored.reaction_frames(),
            std::slice::from_ref(&reaction.frame)
        );
        let rollback_golden = format!(
            "outcome=rejected\ndiagnostic=WaveBudgetExceeded\nbarriers={}\nfirst_barrier_module_changed={}\nsecond_barrier_module_changed={}\nroot_state_restored={}\nsnapshot_state_restored={}\n",
            restored.reaction_frames()[0].wave_barriers.len(),
            restored.reaction_frames()[0].wave_barriers[0]
                .state_before
                .module_state_hash
                != restored.reaction_frames()[0].wave_barriers[0]
                    .state_after
                    .module_state_hash,
            restored.reaction_frames()[0].wave_barriers[1]
                .state_before
                .module_state_hash
                != restored.reaction_frames()[0].wave_barriers[1]
                    .state_after
                    .module_state_hash,
            reaction.frame.state_hash_before == reaction.frame.state_hash_after,
            restored.readout().module_state_hash == state_before,
        );
        assert_eq!(
            rollback_golden,
            include_str!("fixtures/rejected-root-wave-rollback.golden")
        );
    }

    #[test]
    fn snapshot_replay_rejects_noncanonical_event_even_with_rehashed_evidence() {
        let mut host = GameplayRuntimeHost::activate(wave_fixture_host_input()).unwrap();
        host.observe_with_source_facts(wave_fixture_root_event(), Vec::new())
            .expect("reaction frame is recorded");
        let snapshot = host.compose_snapshot().expect("snapshot");
        let mut stored: StoredGameplayRuntimeHostSnapshot =
            serde_json::from_str(&snapshot.text).expect("stored host snapshot");
        let tampered_payload = b" 0".to_vec();
        let root_event = &mut stored.reaction_frames[0].root_events[0];
        root_event.canonical_payload = tampered_payload.clone();
        root_event.payload_hash = gameplay_canonical_payload_hash(&tampered_payload);
        stored.reaction_frames[0].frame_hash = stored.reaction_frames[0].canonical_hash();
        stored.snapshot_hash = gameplay_runtime_snapshot_hash(&stored);
        let tampered = serde_json::to_string(&stored).expect("tampered snapshot serializes");

        let error = match GameplayRuntimeHost::restore(wave_fixture_host_input(), &tampered) {
            Ok(_) => panic!("replayed envelope must pass canonical codec admission"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            GameplayRuntimeHostError::Snapshot(message)
                if message.contains("failed codec admission")
                    && message.contains("not canonical")
        ));
    }

    #[test]
    fn snapshot_restore_validates_every_nested_decision_evidence_hash() {
        let mut host = GameplayRuntimeHost::activate(decision_host_input()).unwrap();
        let mut owner = DecisionOwnerFixture::default();
        let suspended = host.decide(decision_moment("decision-1", 0), &mut owner);
        assert_eq!(suspended.status, GameplayDecisionStatus::Suspended);
        let snapshot = host.compose_snapshot().expect("decision snapshot");

        let restore_error = |mut stored: StoredGameplayRuntimeHostSnapshot| {
            stored.snapshot_hash = gameplay_runtime_snapshot_hash(&stored);
            let text = serde_json::to_string(&stored).expect("tampered snapshot serializes");
            match GameplayRuntimeHost::restore(decision_host_input(), &text) {
                Ok(_) => panic!("tampered nested decision evidence must fail restore"),
                Err(error) => error,
            }
        };

        let mut receipt_tampered: StoredGameplayRuntimeHostSnapshot =
            serde_json::from_str(&snapshot.text).unwrap();
        receipt_tampered.decision_receipts[0].decision_id = "different-decision".to_owned();
        assert!(matches!(
            restore_error(receipt_tampered),
            GameplayRuntimeHostError::Snapshot(message)
                if message.contains("decision receipt 0")
        ));

        let mut read_tampered: StoredGameplayRuntimeHostSnapshot =
            serde_json::from_str(&snapshot.text).unwrap();
        read_tampered.decision_receipts[0].invocations[0]
            .declared_reads
            .as_mut()
            .expect("decision fixture records frozen reads")
            .reads[0]
            .value_hash = "fnv1a64:tampered".to_owned();
        assert!(matches!(
            restore_error(read_tampered),
            GameplayRuntimeHostError::Snapshot(message)
                if message.contains("decision receipt 0")
        ));

        let mut continuation_value: serde_json::Value =
            serde_json::from_str(&snapshot.text).unwrap();
        continuation_value["decisionContinuations"]["pending"]["decision-1"]["registryDigest"] =
            serde_json::Value::String("fnv1a64:foreign".to_owned());
        let continuation_tampered: StoredGameplayRuntimeHostSnapshot =
            serde_json::from_value(continuation_value).unwrap();
        assert!(matches!(
            restore_error(continuation_tampered),
            GameplayRuntimeHostError::Snapshot(message)
                if message.contains("continuation table")
        ));
    }

    #[test]
    fn scheduler_owner_rejection_is_canonical_and_preserves_authority() {
        let mut host = GameplayRuntimeHost::activate(scheduler_host_input()).unwrap();
        let authority_before = host.readout().authority_state_hash;
        let mut draft = scheduled_collision_deactivation();
        let rejected_payload = rule_gameplay_fabric::CapabilityActivationGameplayProposal {
            entity: 10,
            capability: "unsupported-capability".to_owned(),
            action: "deactivate".to_owned(),
        };
        draft.proposal.canonical_payload =
            serde_json::to_vec(&rejected_payload).expect("rejected proposal serializes");
        draft.proposal.payload_hash =
            gameplay_canonical_payload_hash(&draft.proposal.canonical_payload);
        let action_id = draft.id.clone();
        {
            let mut scheduler = host.scheduler_port();
            scheduler
                .apply(GameplayRuntimeSchedulerCommand::ScheduleTick(draft))
                .unwrap();
            scheduler
                .apply(GameplayRuntimeSchedulerCommand::ExecuteTick {
                    action_id: action_id.clone(),
                    tick: 5,
                    validity: ScheduledActionValidity::CURRENT,
                })
                .unwrap();
        }
        let rejected = host.scheduler_port().route(&action_id).unwrap();
        emit_integration_evidence(
            "owner-rejected",
            1,
            &format!("action:{}", action_id.as_str()),
            &host.readout(),
            &[
                rejected.routing.proposal_hash.as_str(),
                rejected.routing.routing_hash.as_str(),
            ],
        );
        assert!(!rejected.routing.accepted);
        assert_eq!(rejected.routing.diagnostic_codes, ["unsupportedCapability"]);
        assert!(rejected.delivered_events.is_empty());
        assert!(rejected.reaction.is_none());
        assert_eq!(host.readout().authority_state_hash, authority_before);
        assert_eq!(host.scheduler_readout().pending_action_count, 0);
        assert_eq!(host.scheduler_readout().outstanding_dispatch_count, 0);
    }

    #[test]
    fn scheduler_ports_bind_one_live_host_across_concurrent_sessions_and_restore() {
        let mut first = GameplayRuntimeHost::activate(scheduler_host_input_for(
            "authority.fixture-scheduler-first",
        ))
        .unwrap();
        let second = GameplayRuntimeHost::activate(scheduler_host_input_for(
            "authority.fixture-scheduler-second",
        ))
        .unwrap();
        let second_before = second.scheduler_readout();
        let action_id = ScheduledActionId::new("fixture.scheduler.deactivate-collision");
        let mut draft = scheduled_collision_deactivation();

        // Owner-shaped strings are event provenance, not command authority.
        // Even a label copied from the other Session cannot redirect the port.
        let foreign_label = "authority.fixture-scheduler-second".to_owned();
        draft.source = protocol_game_extension::GameplayEmitterRef::Owner {
            owner_id: foreign_label.clone(),
        };
        draft.proposal.emitter = protocol_game_extension::GameplayEmitterRef::Owner {
            owner_id: foreign_label,
        };
        first
            .scheduler_port()
            .apply(GameplayRuntimeSchedulerCommand::ScheduleTick(draft))
            .unwrap();
        assert_eq!(first.scheduler_readout().pending_action_count, 1);
        assert_eq!(second.scheduler_readout(), second_before);

        let snapshot = first.compose_snapshot().unwrap();
        let mut restored = GameplayRuntimeHost::restore(
            scheduler_host_input_for("authority.fixture-scheduler-first"),
            &snapshot.text,
        )
        .expect("restored host mints its own new port");
        assert_eq!(restored.scheduler_readout().pending_action_count, 1);

        // A port borrowed from the original host cannot target the restored
        // host. Cancelling through it changes only the original instance.
        first
            .scheduler_port()
            .apply(GameplayRuntimeSchedulerCommand::Cancel {
                action_id: action_id.clone(),
                reason: "original-session-only".to_owned(),
            })
            .unwrap();
        assert_eq!(first.scheduler_readout().pending_action_count, 0);
        assert_eq!(restored.scheduler_readout().pending_action_count, 1);

        let mut restored_port = restored.scheduler_port();
        let executed = restored_port
            .apply(GameplayRuntimeSchedulerCommand::ExecuteTick {
                action_id: action_id.clone(),
                tick: 5,
                validity: ScheduledActionValidity::CURRENT,
            })
            .unwrap();
        let state_after_execution = executed.readout.state_hash;
        assert!(matches!(
            restored_port.apply(GameplayRuntimeSchedulerCommand::ExecuteTick {
                action_id,
                tick: 5,
                validity: ScheduledActionValidity::CURRENT,
            }),
            Err(GameplayRuntimeHostError::Scheduler(
                GameplaySchedulerError::UnknownAction
            ))
        ));
        drop(restored_port);
        assert_eq!(
            restored.scheduler_readout().state_hash,
            state_after_execution
        );

        let error = match GameplayRuntimeHost::restore(
            scheduler_host_input_for("authority.fixture-scheduler-second"),
            &snapshot.text,
        ) {
            Ok(_) => panic!("foreign scheduler owner must not restore as the saved host"),
            Err(error) => error,
        };
        assert!(matches!(error, GameplayRuntimeHostError::Snapshot(_)));
    }

    #[test]
    fn scheduler_restore_delivers_recorded_owner_events_without_rerouting_authority() {
        let mut host = GameplayRuntimeHost::activate(scheduler_host_input()).unwrap();
        let action_id = ScheduledActionId::new("fixture.scheduler.deactivate-collision");
        {
            let mut scheduler = host.scheduler_port();
            scheduler
                .apply(GameplayRuntimeSchedulerCommand::ScheduleTick(
                    scheduled_collision_deactivation(),
                ))
                .unwrap();
            scheduler
                .apply(GameplayRuntimeSchedulerCommand::ExecuteTick {
                    action_id: action_id.clone(),
                    tick: 5,
                    validity: ScheduledActionValidity::CURRENT,
                })
                .unwrap();
        }

        // Model interruption after authority routing was durably recorded but
        // before the returned owner event entered its next Observe wave.
        let dispatch = host.scheduler.outstanding_dispatches()[0].clone();
        let mut entities = host.session.bundle.runtime_entities.take().unwrap();
        let route = GameplayFabricCoordinator::new(
            host.session.registry(),
            limits_from_registry(host.session.registry()),
        )
        .route_proposal(
            dispatch.proposal,
            &mut RuntimeSessionOwnerRouter {
                entities: &mut entities,
            },
        )
        .unwrap();
        host.scheduler
            .apply(rule_scheduler::GameplaySchedulerCommand::RecordRouting {
                action_id: action_id.clone(),
                receipt: route,
            })
            .unwrap();
        host.session.bundle.runtime_entities = Some(entities);
        assert_eq!(host.scheduler.outstanding_event_deliveries().len(), 1);
        let authority_after_route = host.readout().authority_state_hash;

        let snapshot = host.compose_snapshot().unwrap();
        let mut restored = GameplayRuntimeHost::restore(scheduler_host_input(), &snapshot.text)
            .expect("pending event delivery restores");
        assert_eq!(
            restored
                .scheduler_readout()
                .outstanding_event_delivery_count,
            1
        );
        let delivered = restored.scheduler_port().route(&action_id).unwrap();
        emit_integration_evidence(
            "scheduler-recovered",
            1,
            &format!("action:{}", action_id.as_str()),
            &restored.readout(),
            &[
                delivered.routing.proposal_hash.as_str(),
                delivered.routing.routing_hash.as_str(),
                delivered
                    .reaction
                    .as_ref()
                    .expect("recovered event delivery reacts")
                    .frame
                    .frame_hash
                    .as_str(),
            ],
        );
        assert_eq!(delivered.delivered_events.len(), 1);
        assert!(rule_gameplay_fabric::verify_gameplay_routing_evidence(
            &delivered.routing,
            &delivered.delivered_events,
        ));
        assert_eq!(
            delivered.delivered_events[0].emitter,
            protocol_game_extension::GameplayEmitterRef::Owner {
                owner_id: delivered.routing.owner_id.clone(),
            }
        );
        assert_eq!(
            gameplay_canonical_payload_hash(&delivered.delivered_events[0].canonical_payload),
            delivered.delivered_events[0].payload_hash,
        );
        assert!(delivered.reaction.as_ref().unwrap().observe.accepted());
        assert!(matches!(
            delivered.delivery_fact,
            Some(GameplaySchedulerFact::EventDeliveryCompleted { .. })
        ));
        assert_eq!(
            restored
                .scheduler_readout()
                .outstanding_event_delivery_count,
            0
        );
        assert_eq!(
            restored.readout().authority_state_hash,
            authority_after_route
        );
        assert_eq!(restored.reaction_frames().len(), 1);
        assert!(matches!(
            restored.scheduler_port().route(&action_id),
            Err(GameplayRuntimeHostError::Scheduler(
                GameplaySchedulerError::UnknownAction
            ))
        ));
    }
}
