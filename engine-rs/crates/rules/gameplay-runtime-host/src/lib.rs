//! Public-height, statically composed gameplay RuntimeSession host.
//!
//! This is the Rust host seam a downstream native provider can compose. It owns
//! no module discovery and accepts no callbacks: the module topology is a
//! concrete GameplayStaticComposition supplied at construction.

#![forbid(unsafe_code)]

mod owner_router;
mod prefab;
mod scheduler;

pub use prefab::*;
pub use scheduler::*;

use owner_router::{RuntimeSessionDecisionOwner, RuntimeSessionOwnerRouter};

use std::collections::{BTreeMap, BTreeSet};

use core_entity::{
    Aabb, EntityLifecycleCommand, EntitySource, EntityStore, EntityTransform, MovementCommand,
    MovementEvent, TransformCommand,
};
use core_math::Vec3;
use gameplay_module_sdk::GameplayStaticComposition;
use protocol_game_extension::{
    GameplayEventEnvelope, GameplayEventPhase, GameplayModuleBindingActivationReceipt,
    GameplayModuleBindingRegistry, GameplayOwnerRef, GameplayProposalEnvelope,
};
use rule_gameplay_fabric::{
    adapt_session_tick, gameplay_module_payload_hash, FrozenGameplayViews,
    GameplayDecisionContinuations, GameplayEntityScopeIndex, GameplayFabricCoordinator,
    GameplayFrozenReadSet, GameplayModuleStateError, GameplayObserveReceipt,
    GameplayOwnerEventContext, GameplayOwnerQueryProvider, GameplayPrefabInstanceBinding,
    GameplayPrefabInstanceIndex, GameplayReactionFrame, GameplayReactionSourceFact,
    GameplayReadAssembler, GameplayReadAssemblyError, GameplayReadDiagnostic,
    GameplayReadDiagnosticCode, GameplayReadPlan, GameplayReadRequest, GameplayReadSelector,
    GameplayRuntimeDiagnostic, GameplayRuntimeLimits, GameplayTriggerOverlapQueryProvider,
    GameplayViewSource,
};
use rule_project_bundle::{
    GameplayBindingActivationError, GameplayBoundProjectBundleSession, SessionStateArtifact,
};
use serde::{Deserialize, Serialize};

// These are deliberately re-exported from the public host altitude. Consumers
// can execute a normal ProjectBundle load without naming private engine crates.
pub use core_ids::{EntityId, RuntimeSessionId, SceneId};
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
    ArtifactEntry, ArtifactRole, PrefabRegistry, PrefabRegistryValidationContext,
    ValidatedPrefabRegistry, PREFAB_REGISTRY_SCHEMA_VERSION,
};
pub use svc_serialization::{LoadPlan, LoadStep};

pub const GAMEPLAY_RUNTIME_HOST_SNAPSHOT_PATH: &str = "session/gameplay-runtime-host.snapshot.json";
const GAMEPLAY_RUNTIME_HOST_SNAPSHOT_VERSION: u32 = 3;
const MAX_REACTION_FRAMES: usize = 256;
const MAX_DECISION_RECEIPTS: usize = 256;

#[derive(Debug)]
pub enum GameplayRuntimeHostError {
    Load(String),
    Prefab(String),
    Snapshot(String),
    Activation(GameplayBindingActivationError),
    MissingEntityAuthority,
    Transform { entity: u64, code: &'static str },
    SpatialEntity { entity: u64, code: &'static str },
    Movement { entity: u64, code: &'static str },
    State(GameplayModuleStateError),
    Scheduler(GameplaySchedulerError),
    SchedulerRouting(GameplayRuntimeDiagnostic),
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
    pub bindings: GameplayModuleBindingRegistry,
    pub entity_targets: GameplayBindingEntityTargets,
    pub spatial_entities: Vec<GameplayRuntimeSpatialEntity>,
    pub declared_reads: Vec<GameplayRuntimeDeclaredReadPlan>,
    pub triggers: Vec<GameplayTriggerDefinition>,
    pub scheduler: GameplayRuntimeSchedulerDefinition,
}

/// Public loading form for consumers that have authored ProjectBundle
/// artifacts but do not already own an engine-internal load result.
pub struct GameplayRuntimeProjectInput {
    pub load_plan: LoadPlan,
    pub artifacts: BundleArtifacts,
    pub composition: GameplayStaticComposition,
    pub bindings: GameplayModuleBindingRegistry,
    pub entity_targets: GameplayBindingEntityTargets,
    pub spatial_entities: Vec<GameplayRuntimeSpatialEntity>,
    pub declared_reads: Vec<GameplayRuntimeDeclaredReadPlan>,
    pub triggers: Vec<GameplayTriggerDefinition>,
    pub scheduler: GameplayRuntimeSchedulerDefinition,
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

/// Event identity and wave are supplied by the host at delivery time; a
/// consumer declares only the statically composed module/invocation plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayRuntimeDeclaredReadPlan {
    pub module_id: String,
    pub invocation_id: String,
    pub requests: Vec<GameplayReadRequest>,
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
    pub events: Vec<GameplayEventEnvelope>,
    pub diagnostic_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRuntimeHostReadout {
    pub gameplay_registry_digest: String,
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
    prefab_registry: ValidatedPrefabRegistry,
    declared_reads: Vec<GameplayRuntimeDeclaredReadPlan>,
    reaction_frames: Vec<GameplayReactionFrame>,
    decision_continuations: GameplayDecisionContinuations,
    decision_receipts: Vec<GameplayDecisionReceipt>,
    scheduler: GameplayActionScheduler,
}

impl GameplayRuntimeHost {
    pub fn activate_project(
        input: GameplayRuntimeProjectInput,
    ) -> Result<Self, GameplayRuntimeHostError> {
        let bundle = execute_load_plan(&input.load_plan, &input.artifacts)
            .map_err(|error| GameplayRuntimeHostError::Load(format!("{error:?}")))?;
        Self::activate(GameplayRuntimeHostInput {
            bundle,
            composition: input.composition,
            bindings: input.bindings,
            entity_targets: input.entity_targets,
            spatial_entities: input.spatial_entities,
            declared_reads: input.declared_reads,
            triggers: input.triggers,
            scheduler: input.scheduler,
        })
    }

    /// Load authored ProjectBundle artifacts, validate one complete prefab
    /// registry, apply every placement atomically in staging, then activate the
    /// closed gameplay topology against the resulting part-role authority.
    pub fn activate_project_with_prefabs(
        input: GameplayRuntimeProjectInput,
        prefabs: GameplayRuntimePrefabBootstrap,
    ) -> Result<Self, GameplayRuntimeHostError> {
        let mut bundle = execute_load_plan(&input.load_plan, &input.artifacts)
            .map_err(|error| GameplayRuntimeHostError::Load(format!("{error:?}")))?;
        let prefab_registry = apply_prefab_bootstrap(&mut bundle, prefabs)?;
        Self::activate_with_prefab_registry(
            GameplayRuntimeHostInput {
                bundle,
                composition: input.composition,
                bindings: input.bindings,
                entity_targets: input.entity_targets,
                spatial_entities: input.spatial_entities,
                declared_reads: input.declared_reads,
                triggers: input.triggers,
                scheduler: input.scheduler,
            },
            prefab_registry,
        )
    }

    pub fn restore_project(
        input: GameplayRuntimeProjectInput,
        snapshot_text: &str,
    ) -> Result<Self, GameplayRuntimeHostError> {
        let bundle = execute_load_plan(&input.load_plan, &input.artifacts)
            .map_err(|error| GameplayRuntimeHostError::Load(format!("{error:?}")))?;
        Self::restore(
            GameplayRuntimeHostInput {
                bundle,
                composition: input.composition,
                bindings: input.bindings,
                entity_targets: input.entity_targets,
                spatial_entities: input.spatial_entities,
                declared_reads: input.declared_reads,
                triggers: input.triggers,
                scheduler: input.scheduler,
            },
            snapshot_text,
        )
    }

    /// Validate the authored prefab source and placement commands before
    /// restoring the saved Session. The snapshot remains authoritative for the
    /// live entity/role map and must match the binding activation evidence.
    pub fn restore_project_with_prefabs(
        input: GameplayRuntimeProjectInput,
        prefabs: GameplayRuntimePrefabBootstrap,
        snapshot_text: &str,
    ) -> Result<Self, GameplayRuntimeHostError> {
        let mut bundle = execute_load_plan(&input.load_plan, &input.artifacts)
            .map_err(|error| GameplayRuntimeHostError::Load(format!("{error:?}")))?;
        let prefab_registry = apply_prefab_bootstrap(&mut bundle, prefabs)?;
        Self::restore_with_prefab_registry(
            GameplayRuntimeHostInput {
                bundle,
                composition: input.composition,
                bindings: input.bindings,
                entity_targets: input.entity_targets,
                spatial_entities: input.spatial_entities,
                declared_reads: input.declared_reads,
                triggers: input.triggers,
                scheduler: input.scheduler,
            },
            snapshot_text,
            prefab_registry,
        )
    }

    pub fn activate(input: GameplayRuntimeHostInput) -> Result<Self, GameplayRuntimeHostError> {
        Self::activate_with_prefab_registry(input, empty_prefab_registry())
    }

    fn activate_with_prefab_registry(
        mut input: GameplayRuntimeHostInput,
        prefab_registry: ValidatedPrefabRegistry,
    ) -> Result<Self, GameplayRuntimeHostError> {
        prepare_runtime_entities(&mut input)?;
        let mut session = GameplayBoundProjectBundleSession::activate(
            input.bundle,
            input.composition,
            input.bindings,
            &input.entity_targets,
        )?;
        session.install_trigger_definitions(resolve_trigger_definitions(input.triggers)?)?;
        validate_scheduler_definition(session.registry(), &input.scheduler)?;
        let scheduler = input.scheduler.build();
        Ok(Self {
            session,
            prefab_registry,
            declared_reads: input.declared_reads,
            reaction_frames: Vec::new(),
            decision_continuations: GameplayDecisionContinuations::default(),
            decision_receipts: Vec::new(),
            scheduler,
        })
    }

    pub fn restore(
        input: GameplayRuntimeHostInput,
        snapshot_text: &str,
    ) -> Result<Self, GameplayRuntimeHostError> {
        Self::restore_with_prefab_registry(input, snapshot_text, empty_prefab_registry())
    }

    fn restore_with_prefab_registry(
        mut input: GameplayRuntimeHostInput,
        snapshot_text: &str,
        prefab_registry: ValidatedPrefabRegistry,
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
        let session = GameplayBoundProjectBundleSession::restore(
            input.bundle,
            input.composition,
            input.bindings,
            &input.entity_targets,
            &stored.session_snapshot,
        )?;
        validate_scheduler_definition(session.registry(), &input.scheduler)?;
        let expected_triggers = rule_trigger_volume::TriggerVolumeRule::new(
            resolve_trigger_definitions(input.triggers)?,
        )
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
        Ok(Self {
            session,
            prefab_registry,
            declared_reads: input.declared_reads,
            reaction_frames: stored.reaction_frames,
            decision_continuations: stored.decision_continuations,
            decision_receipts: stored.decision_receipts,
            scheduler,
        })
    }

    pub fn observe(
        &mut self,
        event: GameplayEventEnvelope,
    ) -> Result<GameplayRuntimeReactionReceipt, GameplayRuntimeHostError> {
        self.observe_with_source_facts(event, Vec::new())
    }

    pub fn tick(
        &mut self,
        tick: u64,
    ) -> Result<GameplayRuntimeReactionReceipt, GameplayRuntimeHostError> {
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
            },
            self.session.invocation_host(),
            &mut RuntimeSessionDecisionOwner { owner },
        );
        if self.decision_receipts.len() == MAX_DECISION_RECEIPTS {
            self.decision_receipts.remove(0);
        }
        self.decision_receipts.push(receipt.clone());
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
        let mut stored = StoredGameplayRuntimeHostSnapshot {
            schema_version: GAMEPLAY_RUNTIME_HOST_SNAPSHOT_VERSION,
            session_snapshot: session.text,
            reaction_frames: self.reaction_frames.clone(),
            decision_continuations: self.decision_continuations.clone(),
            decision_receipts: self.decision_receipts.clone(),
            scheduler_snapshot: self.scheduler.encode_snapshot()?,
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
        let runtime_host_hash = gameplay_module_payload_hash(
            format!(
                "{}|{}|{}|{}|{}|{}|{}|{}|{}",
                self.session.registry().registry_digest(),
                self.session.bindings().registry_hash,
                self.session.module_state.state_hash(),
                authority_state_hash,
                self.session.trigger_rule().snapshot().snapshot_hash,
                last_reaction_frame_hash.as_deref().unwrap_or("none"),
                last_decision_receipt_hash.as_deref().unwrap_or("none"),
                self.decision_continuations.pending_count(),
                scheduler.state_hash,
            )
            .as_bytes(),
        );
        GameplayRuntimeHostReadout {
            gameplay_registry_digest: self.session.registry().registry_digest().to_owned(),
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
        gameplay_module_payload_hash(authority.text.as_bytes())
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

    fn observe_with_source_facts(
        &mut self,
        event: GameplayEventEnvelope,
        source_facts: Vec<GameplayReactionSourceFact>,
    ) -> Result<GameplayRuntimeReactionReceipt, GameplayRuntimeHostError> {
        let state_hash_before = self.session.module_state.state_hash();
        let mut authority_entities = self
            .session
            .bundle
            .runtime_entities
            .take()
            .expect("runtime entity authority initialized");
        let authority_before = authority_entities.snapshot_durable();
        let frozen_entities = EntityStore::from_snapshot(authority_before.clone());
        let observe = GameplayFabricCoordinator::new(
            self.session.registry(),
            limits_from_registry(self.session.registry()),
        )
        .observe(
            event,
            &RuntimeSessionViews {
                registry: self.session.registry(),
                module_state: &self.session.module_state,
                entities: &frozen_entities,
                triggers: self.session.trigger_rule(),
                prefab_registry: &self.prefab_registry,
                prefab_instances: &self.session.bundle.prefab_instances,
                declared_reads: &self.declared_reads,
            },
            self.session.invocation_host(),
            &mut RuntimeSessionOwnerRouter {
                entities: &mut authority_entities,
            },
        );
        if !observe.accepted() {
            authority_entities = EntityStore::from_snapshot(authority_before.clone());
        }
        self.session.bundle.runtime_entities = Some(authority_entities);
        let mut accepted_facts = Vec::new();
        if observe.accepted() {
            if let Err(error) = self
                .session
                .module_state
                .apply_facts_atomic(&observe.module_facts)
            {
                self.session.bundle.runtime_entities =
                    Some(EntityStore::from_snapshot(authority_before));
                return Err(error.into());
            }
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredGameplayRuntimeHostSnapshot {
    schema_version: u32,
    session_snapshot: String,
    reaction_frames: Vec<GameplayReactionFrame>,
    decision_continuations: GameplayDecisionContinuations,
    decision_receipts: Vec<GameplayDecisionReceipt>,
    scheduler_snapshot: Vec<u8>,
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
            "{}|{}|{}|{}|{}|{}",
            snapshot.schema_version,
            snapshot.session_snapshot,
            frames,
            decisions,
            continuations,
            gameplay_module_payload_hash(&snapshot.scheduler_snapshot),
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
                    "{}|{}|{}|{}|{}|{}",
                    self.registry.registry_digest(),
                    self.module_state.state_hash(),
                    self.entities.hash().0,
                    self.triggers.snapshot().snapshot_hash,
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

fn activation_hash(activation: &GameplayModuleBindingActivationReceipt) -> String {
    let bytes = serde_json::to_vec(activation).expect("activation receipt serializes");
    rule_gameplay_fabric::gameplay_module_payload_hash(&bytes)
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
    definitions: Vec<GameplayTriggerDefinition>,
) -> Result<Vec<rule_trigger_volume::KinematicTriggerDefinition>, GameplayRuntimeHostError> {
    definitions
        .into_iter()
        .map(|definition| {
            if definition.schema_version != GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION {
                return Err(GameplayRuntimeHostError::Snapshot(format!(
                    "trigger {} uses unsupported schema version {}",
                    definition.entity, definition.schema_version
                )));
            }
            Ok(rule_trigger_volume::KinematicTriggerDefinition::new(
                EntityId::new(definition.entity),
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
    use core_scene::{encode, SceneMetadata, SceneNode, SceneNodeKind, SceneTree};
    use gameplay_module_sdk::*;
    use protocol_game_extension::GameplayInvocationReadRequirement;
    use rule_trigger_volume::TriggerOverlapFactKind;
    use serde::{Deserialize, Serialize};

    fn empty_scheduler_definition() -> GameplayRuntimeSchedulerDefinition {
        GameplayRuntimeSchedulerDefinition::new(
            GameplayOwnerRef {
                owner_id: "authority.fixture-scheduler".to_owned(),
                provider_id: "provider.fixture-scheduler".to_owned(),
            },
            Vec::new(),
            Vec::new(),
        )
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
        GameplayContractRef {
            namespace: "fixture.decision".to_owned(),
            name: name.to_owned(),
            version: 1,
            schema_hash: format!("sha256:fixture-decision-{name}"),
        }
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
        let manifest = GameplayModuleManifest {
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
        GameplayStaticModuleProvider::linked_from_manifest(manifest, DecisionBehavior)
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
    struct DecisionOwnerFixture {
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

    fn bundle() -> ProjectBundleLoadResult {
        let scene = SceneTree {
            id: SceneId::new(1),
            schema_version: 1,
            metadata: SceneMetadata {
                name: Some("host-fixture".to_owned()),
                authoring_format_version: 1,
            },
            dependencies: Vec::new(),
            roots: vec![SceneNode::leaf(
                SceneNodeId::new(1),
                SceneNodeKind::EmptyGroup,
            )],
        };
        let plan = LoadPlan {
            steps: vec![
                LoadStep::ValidateVersions {
                    bundle_schema_version: 1,
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
        execute_load_plan(&plan, &artifacts).unwrap()
    }

    fn create_spatial(
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

    fn decision_host_input() -> GameplayRuntimeHostInput {
        let mut bundle = bundle();
        create_spatial(&mut bundle, EntityId::new(20), 0.0, true);
        let mut composition = GameplayStaticCompositionBuilder::new();
        composition.add_provider(decision_provider());
        GameplayRuntimeHostInput {
            bundle,
            composition: composition.build().expect("decision composition"),
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

    fn decision_moment(decision_id: &str, owner_revision: u64) -> GameplayDecisionMoment {
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
                payload_hash: gameplay_module_payload_hash(&payload),
            },
            expected_owner_revision: format!("revision:{owner_revision}"),
            workspace,
            resume_token: None,
        }
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
        let replayed_receipt = restored.decide(replayed, &mut owner);
        assert_eq!(replayed_receipt.status, GameplayDecisionStatus::Failed);
        assert!(replayed_receipt.invocations.is_empty());
        assert_eq!(owner.committed_payloads.len(), 1);

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
    fn public_height_host_binds_actor_pose_to_trigger_authority_and_snapshot() {
        let mut bundle = bundle();
        create_spatial(&mut bundle, EntityId::new(10), 0.0, true);
        create_spatial(&mut bundle, EntityId::new(20), 2.0, false);
        let mut composition = GameplayStaticCompositionBuilder::new();
        composition.include_standard_owner_events();
        let bindings = GameplayModuleBindingRegistryBuilder::new().build();
        let mut host = GameplayRuntimeHost::activate(GameplayRuntimeHostInput {
            bundle,
            composition: composition.build().unwrap(),
            bindings,
            entity_targets: GameplayBindingEntityTargets::new(),
            spatial_entities: Vec::new(),
            declared_reads: Vec::new(),
            triggers: vec![GameplayTriggerDefinition {
                schema_version: GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
                entity: 10,
                scope: "zone.host".to_owned(),
                tags: vec!["door".to_owned()],
            }],
            scheduler: empty_scheduler_definition(),
        })
        .unwrap();
        let authority_hash_before = host.readout().authority_state_hash;
        let runtime_hash_before = host.readout().runtime_host_hash;
        assert!(host
            .reconcile_triggers(1, TriggerReconcileCause::Tick)
            .unwrap()
            .collision
            .facts
            .is_empty());
        let moved_without_overlap_change = host
            .set_actor_translation_and_reconcile(EntityId::new(20), [3.0, 0.0, 0.0], 2)
            .unwrap();
        assert!(moved_without_overlap_change.collision.facts.is_empty());
        assert_ne!(host.readout().authority_state_hash, authority_hash_before);
        assert_ne!(host.readout().runtime_host_hash, runtime_hash_before);
        let entered = host
            .set_actor_translation_and_reconcile(EntityId::new(20), [0.0, 0.0, 0.0], 3)
            .unwrap();
        assert_eq!(
            entered.collision.facts[0].kind,
            TriggerOverlapFactKind::Enter
        );
        assert_eq!(host.readout().active_overlap_count, 1);
        assert!(host
            .compose_snapshot()
            .unwrap()
            .text
            .contains("triggerSnapshot"));
    }
}
