//! Standard engine-owner facts adapted into open gameplay-fabric events at the
//! semantic boundary. These adapters do not reimplement or apply owner logic.

use core_entity::{CapabilityActivationEvent, EntityLifecycleEvent};
use core_events::DomainEvent;
use core_ids::EntityId;
use protocol_game_extension::{
    GameplayCausationRef, GameplayContractRef, GameplayEmitterRef, GameplayEntityRef,
    GameplayEventEnvelope, GameplayEventPhase, GameplayEventSchemaDeclaration,
    GameplayExecutionBudget, GameplayModuleManifest, GameplayModuleRef, GameplayOwnerRef,
    GameplayProposalDeclaration,
};
use rule_state_machine::StateMachineEvent;
use rule_trigger_volume::{TriggerOverlapFact, TriggerOverlapFactKind};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use svc_combat::{CombatEvent, CombatReadout, FireMissReason};
use svc_game_rules::EffectResolutionRequest;
use svc_gameplay_fabric::{
    gameplay_canonical_codec_id, gameplay_contract, GameplayFabricRegistryBuilder,
    GameplayLinkedProvider, GameplayProposalOwnerRegistration, TypedGameplayEventCodec,
};

use crate::gameplay_payload_hash;

const STANDARD_OWNER_MODULE_ID: &str = "asha.owner-events";
const STANDARD_OWNER_PROVIDER_ID: &str = "provider.asha-owner-events";
pub const CAPABILITY_ACTIVATION_PROPOSAL_OWNER_ID: &str = "authority.capability-activation";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StandardGameplayProposalKind {
    SetCapabilityActivation,
}

impl StandardGameplayProposalKind {
    pub fn schema_descriptor(self) -> &'static str {
        match self {
            Self::SetCapabilityActivation => {
                "CapabilityActivationGameplayProposal{entity:u64,capability:string,action:string};canonical-json-v1"
            }
        }
    }

    pub fn contract(self) -> GameplayContractRef {
        match self {
            Self::SetCapabilityActivation => gameplay_contract(
                "asha.entity",
                "set-capability-activation",
                1,
                self.schema_descriptor(),
            ),
        }
    }

    pub fn owner(self) -> GameplayOwnerRef {
        GameplayOwnerRef {
            owner_id: CAPABILITY_ACTIVATION_PROPOSAL_OWNER_ID.to_owned(),
            provider_id: STANDARD_OWNER_PROVIDER_ID.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityActivationGameplayProposal {
    pub entity: u64,
    pub capability: String,
    pub action: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StandardGameplayEventKind {
    EntityCreated,
    EntityDestroyed,
    EntityLifecycleChanged,
    CapabilityActivationChanged,
    TriggerEntered,
    TriggerExited,
    CombatFireHit,
    CombatFireMissed,
    CombatDamageApplied,
    CombatEntityDefeated,
    StateMachineAttached,
    StateMachineTransitioned,
    ProcessStarted,
    ProcessModeSet,
    ProcessStopped,
    ValueDeltaResolved,
    ModifierApplied,
    SessionTick,
    ScheduledMomentDue,
}

impl StandardGameplayEventKind {
    pub const ALL: [Self; 19] = [
        Self::EntityCreated,
        Self::EntityDestroyed,
        Self::EntityLifecycleChanged,
        Self::CapabilityActivationChanged,
        Self::TriggerEntered,
        Self::TriggerExited,
        Self::CombatFireHit,
        Self::CombatFireMissed,
        Self::CombatDamageApplied,
        Self::CombatEntityDefeated,
        Self::StateMachineAttached,
        Self::StateMachineTransitioned,
        Self::ProcessStarted,
        Self::ProcessModeSet,
        Self::ProcessStopped,
        Self::ValueDeltaResolved,
        Self::ModifierApplied,
        Self::SessionTick,
        Self::ScheduledMomentDue,
    ];

    pub fn contract(self) -> GameplayContractRef {
        let (namespace, name) = match self {
            Self::EntityCreated => ("asha.entity", "created"),
            Self::EntityDestroyed => ("asha.entity", "destroyed"),
            Self::EntityLifecycleChanged => ("asha.entity", "lifecycle-changed"),
            Self::CapabilityActivationChanged => ("asha.entity", "capability-activation-changed"),
            Self::TriggerEntered => ("asha.trigger", "entered"),
            Self::TriggerExited => ("asha.trigger", "exited"),
            Self::CombatFireHit => ("asha.combat", "fire-hit"),
            Self::CombatFireMissed => ("asha.combat", "fire-missed"),
            Self::CombatDamageApplied => ("asha.combat", "damage-applied"),
            Self::CombatEntityDefeated => ("asha.combat", "entity-defeated"),
            Self::StateMachineAttached => ("asha.state-machine", "attached"),
            Self::StateMachineTransitioned => ("asha.state-machine", "transitioned"),
            Self::ProcessStarted => ("asha.process", "started"),
            Self::ProcessModeSet => ("asha.process", "mode-set"),
            Self::ProcessStopped => ("asha.process", "stopped"),
            Self::ValueDeltaResolved => ("asha.game-rules", "value-delta-resolved"),
            Self::ModifierApplied => ("asha.game-rules", "modifier-applied"),
            Self::SessionTick => ("asha.session", "tick"),
            Self::ScheduledMomentDue => ("asha.scheduler", "moment-due"),
        };
        gameplay_contract(namespace, name, 1, self.schema_descriptor())
    }

    pub fn schema_descriptor(self) -> &'static str {
        match self {
            Self::EntityCreated | Self::EntityDestroyed | Self::EntityLifecycleChanged => {
                "EntityLifecycleGameplayPayload{entity:u64,action:string,sourceKind:?string,labels:[u64]};canonical-json-v1"
            }
            Self::CapabilityActivationChanged => {
                "CapabilityActivationGameplayPayload{entity:u64,capability:string,from:string,to:string};canonical-json-v1"
            }
            Self::TriggerEntered | Self::TriggerExited => {
                "TriggerOverlapGameplayPayload{trigger:u64,subject:u64,action:string,scope:string,tags:[string],tick:u64,cause:string,pairHash:string};canonical-json-v1"
            }
            Self::CombatFireHit
            | Self::CombatFireMissed
            | Self::CombatDamageApplied
            | Self::CombatEntityDefeated => {
                "CombatGameplayPayload{shooter:?u64,target:?u64,distance:?f64,missReason:?string,damage:?u32,healthBefore:?u32,healthAfter:?u32,defeated:bool,tick:u64,combatReplayHash:u64};canonical-json-v1"
            }
            Self::StateMachineAttached | Self::StateMachineTransitioned => {
                "StateMachineGameplayPayload{entity:u64,machine:u64,from:?u64,to:u64,revision:u64};canonical-json-v1"
            }
            Self::ProcessStarted | Self::ProcessModeSet | Self::ProcessStopped => {
                "ProcessGameplayPayload{process:u64,mode:?u64,action:string};canonical-json-v1"
            }
            Self::ValueDeltaResolved => {
                "ValueDeltaGameplayPayload{source:u64,target:u64,bundleId:string,channelId:string,amount:i64,requestHash:string,replayHash:string};canonical-json-v1"
            }
            Self::ModifierApplied => {
                "ModifierGameplayPayload{source:u64,target:u64,modifierId:string,stacks:u32,appliedTick:u64,expiresTick:?u64,nextTick:?u64,sourceHash:string,requestHash:string,replayHash:string};canonical-json-v1"
            }
            Self::SessionTick => "SessionTickGameplayPayload{tick:u64};canonical-json-v1",
            Self::ScheduledMomentDue => {
                "ScheduledMomentGameplayPayload{scheduleId:string,dueTick:u64,proposalKind:GameplayContractRef};canonical-json-v1"
            }
        }
    }

    pub fn declaration(self) -> GameplayEventSchemaDeclaration {
        let event = self.contract();
        GameplayEventSchemaDeclaration {
            codec_id: gameplay_canonical_codec_id(&event.schema_hash),
            event,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayOwnerEventContext {
    pub owner_id: String,
    pub tick: u64,
    pub root_id: String,
    pub root_sequence: u64,
    pub first_event_sequence: u32,
    pub parent_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayOwnerEventError {
    Encode(String),
    SequenceOverflow,
}

struct GameplayOwnerEventRoute {
    source: Option<GameplayEntityRef>,
    subjects: Vec<GameplayEntityRef>,
    targets: Vec<GameplayEntityRef>,
    scope: Option<String>,
    tags: Vec<String>,
    phase: GameplayEventPhase,
}

impl core::fmt::Display for GameplayOwnerEventError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for GameplayOwnerEventError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityLifecycleGameplayPayload {
    pub entity: u64,
    pub action: String,
    pub source_kind: Option<String>,
    pub labels: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityActivationGameplayPayload {
    pub entity: u64,
    pub capability: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerOverlapGameplayPayload {
    pub trigger: u64,
    pub subject: u64,
    pub action: String,
    pub scope: String,
    pub tags: Vec<String>,
    pub tick: u64,
    pub cause: String,
    pub pair_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CombatGameplayPayload {
    pub shooter: Option<u64>,
    pub target: Option<u64>,
    pub distance: Option<f64>,
    pub miss_reason: Option<String>,
    pub damage: Option<u32>,
    pub health_before: Option<u32>,
    pub health_after: Option<u32>,
    pub defeated: bool,
    pub tick: u64,
    pub combat_replay_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateMachineGameplayPayload {
    pub entity: u64,
    pub machine: u64,
    pub from: Option<u64>,
    pub to: u64,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessGameplayPayload {
    pub process: u64,
    pub mode: Option<u64>,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueDeltaGameplayPayload {
    pub source: u64,
    pub target: u64,
    pub bundle_id: String,
    pub channel_id: String,
    pub amount: i64,
    pub request_hash: String,
    pub replay_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModifierGameplayPayload {
    pub source: u64,
    pub target: u64,
    pub modifier_id: String,
    pub stacks: u32,
    pub applied_tick: u64,
    pub expires_tick: Option<u64>,
    pub next_tick: Option<u64>,
    pub source_hash: String,
    pub request_hash: String,
    pub replay_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTickGameplayPayload {
    pub tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledMomentGameplayPayload {
    pub schedule_id: String,
    pub due_tick: u64,
    pub proposal_kind: GameplayContractRef,
}

pub fn standard_owner_event_manifest() -> GameplayModuleManifest {
    GameplayModuleManifest {
        module_ref: GameplayModuleRef {
            module_id: STANDARD_OWNER_MODULE_ID.to_owned(),
            namespace: "asha".to_owned(),
            version: "1.0.0".to_owned(),
            sdk_hash: gameplay_payload_hash(b"gameplay-fabric-sdk-v1"),
            contract_hash: gameplay_payload_hash(b"asha-owner-events-contract-v1"),
            artifact_hash: gameplay_payload_hash(b"asha-owner-events-artifact-v1"),
            provider_id: STANDARD_OWNER_PROVIDER_ID.to_owned(),
        },
        published_events: StandardGameplayEventKind::ALL
            .into_iter()
            .map(StandardGameplayEventKind::declaration)
            .collect(),
        subscriptions: Vec::new(),
        invocations: Vec::new(),
        read_views: Vec::new(),
        proposal_kinds: vec![GameplayProposalDeclaration {
            proposal: StandardGameplayProposalKind::SetCapabilityActivation.contract(),
            owner: StandardGameplayProposalKind::SetCapabilityActivation.owner(),
        }],
        state_schemas: Vec::new(),
        fact_schemas: Vec::new(),
        ordering: Vec::new(),
        budget: GameplayExecutionBudget {
            max_waves: 16,
            max_events_per_root: 256,
            max_proposals_per_root: 256,
            max_invocations_per_root: 256,
            max_payload_bytes_per_root: 1_048_576,
        },
        deterministic_requirements: vec![
            "canonical-payload-codecs".to_owned(),
            "semantic-origin-adaptation".to_owned(),
        ],
        source_hash: gameplay_payload_hash(b"rule-gameplay-fabric-owner-events-v1"),
    }
}

pub fn register_standard_owner_events(builder: &mut GameplayFabricRegistryBuilder) {
    let manifest = standard_owner_event_manifest();
    builder
        .register_linked_provider(GameplayLinkedProvider {
            provider_id: manifest.module_ref.provider_id.clone(),
            module_id: manifest.module_ref.module_id.clone(),
            version: manifest.module_ref.version.clone(),
            contract_hash: manifest.module_ref.contract_hash.clone(),
            artifact_hash: manifest.module_ref.artifact_hash.clone(),
            sdk_hash: manifest.module_ref.sdk_hash.clone(),
            source_hash: manifest.source_hash.clone(),
        })
        .register_module(manifest)
        .register_proposal_owner(GameplayProposalOwnerRegistration {
            proposal: StandardGameplayProposalKind::SetCapabilityActivation.contract(),
            owner: StandardGameplayProposalKind::SetCapabilityActivation.owner(),
        });

    register_codec::<EntityLifecycleGameplayPayload>(
        builder,
        StandardGameplayEventKind::EntityCreated,
    );
    register_codec::<EntityLifecycleGameplayPayload>(
        builder,
        StandardGameplayEventKind::EntityDestroyed,
    );
    register_codec::<EntityLifecycleGameplayPayload>(
        builder,
        StandardGameplayEventKind::EntityLifecycleChanged,
    );
    register_codec::<CapabilityActivationGameplayPayload>(
        builder,
        StandardGameplayEventKind::CapabilityActivationChanged,
    );
    register_codec::<TriggerOverlapGameplayPayload>(
        builder,
        StandardGameplayEventKind::TriggerEntered,
    );
    register_codec::<TriggerOverlapGameplayPayload>(
        builder,
        StandardGameplayEventKind::TriggerExited,
    );
    register_codec::<CombatGameplayPayload>(builder, StandardGameplayEventKind::CombatFireHit);
    register_codec::<CombatGameplayPayload>(builder, StandardGameplayEventKind::CombatFireMissed);
    register_codec::<CombatGameplayPayload>(
        builder,
        StandardGameplayEventKind::CombatDamageApplied,
    );
    register_codec::<CombatGameplayPayload>(
        builder,
        StandardGameplayEventKind::CombatEntityDefeated,
    );
    register_codec::<StateMachineGameplayPayload>(
        builder,
        StandardGameplayEventKind::StateMachineAttached,
    );
    register_codec::<StateMachineGameplayPayload>(
        builder,
        StandardGameplayEventKind::StateMachineTransitioned,
    );
    register_codec::<ProcessGameplayPayload>(builder, StandardGameplayEventKind::ProcessStarted);
    register_codec::<ProcessGameplayPayload>(builder, StandardGameplayEventKind::ProcessModeSet);
    register_codec::<ProcessGameplayPayload>(builder, StandardGameplayEventKind::ProcessStopped);
    register_codec::<ValueDeltaGameplayPayload>(
        builder,
        StandardGameplayEventKind::ValueDeltaResolved,
    );
    register_codec::<ModifierGameplayPayload>(builder, StandardGameplayEventKind::ModifierApplied);
    register_codec::<SessionTickGameplayPayload>(builder, StandardGameplayEventKind::SessionTick);
    register_codec::<ScheduledMomentGameplayPayload>(
        builder,
        StandardGameplayEventKind::ScheduledMomentDue,
    );
    let proposal = StandardGameplayProposalKind::SetCapabilityActivation;
    let contract = proposal.contract();
    builder.register_event_codec(TypedGameplayEventCodec::new(
        GameplayEventSchemaDeclaration {
            codec_id: gameplay_canonical_codec_id(&contract.schema_hash),
            event: contract,
        },
        proposal.schema_descriptor(),
        encode_json::<CapabilityActivationGameplayProposal>,
        decode_json::<CapabilityActivationGameplayProposal>,
    ));
}

fn register_codec<T>(builder: &mut GameplayFabricRegistryBuilder, kind: StandardGameplayEventKind)
where
    T: Serialize + DeserializeOwned + 'static,
{
    builder.register_event_codec(TypedGameplayEventCodec::new(
        kind.declaration(),
        kind.schema_descriptor(),
        encode_json::<T>,
        decode_json::<T>,
    ));
}

fn encode_json<T: Serialize>(payload: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(payload).map_err(|error| error.to_string())
}

fn decode_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    serde_json::from_slice(bytes).map_err(|error| error.to_string())
}

pub fn adapt_entity_lifecycle_event(
    context: &GameplayOwnerEventContext,
    event: &EntityLifecycleEvent,
) -> Result<GameplayEventEnvelope, GameplayOwnerEventError> {
    let (kind, payload, entity) = match event {
        EntityLifecycleEvent::Created { id, source, labels } => (
            StandardGameplayEventKind::EntityCreated,
            EntityLifecycleGameplayPayload {
                entity: id.raw(),
                action: "created".to_owned(),
                source_kind: Some(source.label().to_owned()),
                labels: sorted_ids(labels.iter().map(|label| label.raw())),
            },
            *id,
        ),
        EntityLifecycleEvent::Destroyed { id } => (
            StandardGameplayEventKind::EntityDestroyed,
            EntityLifecycleGameplayPayload {
                entity: id.raw(),
                action: "destroyed".to_owned(),
                source_kind: None,
                labels: Vec::new(),
            },
            *id,
        ),
        EntityLifecycleEvent::Disabled { id } => (
            StandardGameplayEventKind::EntityLifecycleChanged,
            EntityLifecycleGameplayPayload {
                entity: id.raw(),
                action: "disabled".to_owned(),
                source_kind: None,
                labels: Vec::new(),
            },
            *id,
        ),
        EntityLifecycleEvent::Enabled { id } => (
            StandardGameplayEventKind::EntityLifecycleChanged,
            EntityLifecycleGameplayPayload {
                entity: id.raw(),
                action: "enabled".to_owned(),
                source_kind: None,
                labels: Vec::new(),
            },
            *id,
        ),
        EntityLifecycleEvent::LabelAdded { id, tag } => (
            StandardGameplayEventKind::EntityLifecycleChanged,
            EntityLifecycleGameplayPayload {
                entity: id.raw(),
                action: "labelAdded".to_owned(),
                source_kind: None,
                labels: vec![tag.raw()],
            },
            *id,
        ),
        EntityLifecycleEvent::LabelRemoved { id, tag } => (
            StandardGameplayEventKind::EntityLifecycleChanged,
            EntityLifecycleGameplayPayload {
                entity: id.raw(),
                action: "labelRemoved".to_owned(),
                source_kind: None,
                labels: vec![tag.raw()],
            },
            *id,
        ),
    };
    envelope(
        context,
        0,
        kind,
        &payload,
        GameplayOwnerEventRoute {
            source: None,
            subjects: vec![entity_ref(entity)],
            targets: Vec::new(),
            scope: Some("entity-lifecycle".to_owned()),
            tags: vec![payload.action.clone()],
            phase: GameplayEventPhase::PostCommit,
        },
    )
}

pub fn adapt_capability_activation_event(
    context: &GameplayOwnerEventContext,
    event: CapabilityActivationEvent,
) -> Result<GameplayEventEnvelope, GameplayOwnerEventError> {
    let payload = CapabilityActivationGameplayPayload {
        entity: event.entity.raw(),
        capability: event.capability.label().to_owned(),
        from: event.from.label().to_owned(),
        to: event.to.label().to_owned(),
    };
    envelope(
        context,
        0,
        StandardGameplayEventKind::CapabilityActivationChanged,
        &payload,
        GameplayOwnerEventRoute {
            source: None,
            subjects: vec![entity_ref(event.entity)],
            targets: Vec::new(),
            scope: Some("capability-activation".to_owned()),
            tags: vec![payload.capability.clone(), payload.to.clone()],
            phase: GameplayEventPhase::PostCommit,
        },
    )
}

/// Adapt an accepted collision-owner enter/exit fact at its semantic origin.
/// The trigger is the source, the overlapping entity is the subject, and all
/// definition scope/tags are preserved in the immutable gameplay envelope.
pub fn adapt_trigger_overlap_fact(
    context: &GameplayOwnerEventContext,
    fact: &TriggerOverlapFact,
) -> Result<GameplayEventEnvelope, GameplayOwnerEventError> {
    let kind = match fact.kind {
        TriggerOverlapFactKind::Enter => StandardGameplayEventKind::TriggerEntered,
        TriggerOverlapFactKind::Exit => StandardGameplayEventKind::TriggerExited,
    };
    let payload = TriggerOverlapGameplayPayload {
        trigger: fact.trigger,
        subject: fact.subject,
        action: fact.kind.as_str().to_owned(),
        scope: fact.scope.clone(),
        tags: fact.tags.clone(),
        tick: fact.tick,
        cause: fact.cause.as_str().to_owned(),
        pair_hash: fact.pair_hash.clone(),
    };
    let mut tags = fact.tags.clone();
    tags.push(fact.kind.as_str().to_owned());
    envelope(
        context,
        0,
        kind,
        &payload,
        GameplayOwnerEventRoute {
            source: Some(entity_ref(EntityId::new(fact.trigger))),
            subjects: vec![entity_ref(EntityId::new(fact.subject))],
            targets: Vec::new(),
            scope: Some(fact.scope.clone()),
            tags,
            phase: GameplayEventPhase::PostCommit,
        },
    )
}

pub fn adapt_combat_readout(
    context: &GameplayOwnerEventContext,
    readout: &CombatReadout,
) -> Result<Vec<GameplayEventEnvelope>, GameplayOwnerEventError> {
    let mut envelopes = Vec::with_capacity(readout.events.len());
    let mut accepted_shooter = None;
    for (ordinal, event) in readout.events.iter().enumerate() {
        let (kind, payload, source, targets, tags) = match event {
            CombatEvent::FireHit {
                shooter,
                target,
                distance,
                tick,
            } => {
                accepted_shooter = Some(*shooter);
                (
                    StandardGameplayEventKind::CombatFireHit,
                    CombatGameplayPayload {
                        shooter: Some(shooter.raw()),
                        target: Some(target.raw()),
                        distance: Some(*distance),
                        miss_reason: None,
                        damage: None,
                        health_before: None,
                        health_after: None,
                        defeated: false,
                        tick: *tick,
                        combat_replay_hash: readout.replay_hash,
                    },
                    Some(entity_ref(*shooter)),
                    vec![entity_ref(*target)],
                    vec!["hit".to_owned()],
                )
            }
            CombatEvent::FireMissed {
                shooter,
                reason,
                tick,
            } => (
                StandardGameplayEventKind::CombatFireMissed,
                CombatGameplayPayload {
                    shooter: Some(shooter.raw()),
                    target: None,
                    distance: None,
                    miss_reason: Some(miss_reason_label(*reason).to_owned()),
                    damage: None,
                    health_before: None,
                    health_after: None,
                    defeated: false,
                    tick: *tick,
                    combat_replay_hash: readout.replay_hash,
                },
                Some(entity_ref(*shooter)),
                Vec::new(),
                vec!["missed".to_owned(), miss_reason_label(*reason).to_owned()],
            ),
            CombatEvent::DamageApplied {
                target,
                amount,
                before,
                after,
            } => (
                StandardGameplayEventKind::CombatDamageApplied,
                CombatGameplayPayload {
                    shooter: accepted_shooter.map(|entity| entity.raw()),
                    target: Some(target.raw()),
                    distance: None,
                    miss_reason: None,
                    damage: Some(*amount),
                    health_before: Some(*before),
                    health_after: Some(*after),
                    defeated: *after == 0,
                    tick: context.tick,
                    combat_replay_hash: readout.replay_hash,
                },
                accepted_shooter.map(entity_ref),
                vec![entity_ref(*target)],
                vec!["damage".to_owned()],
            ),
            CombatEvent::EntityDefeated { target } => (
                StandardGameplayEventKind::CombatEntityDefeated,
                CombatGameplayPayload {
                    shooter: accepted_shooter.map(|entity| entity.raw()),
                    target: Some(target.raw()),
                    distance: None,
                    miss_reason: None,
                    damage: None,
                    health_before: None,
                    health_after: Some(0),
                    defeated: true,
                    tick: context.tick,
                    combat_replay_hash: readout.replay_hash,
                },
                accepted_shooter.map(entity_ref),
                vec![entity_ref(*target)],
                vec!["defeated".to_owned()],
            ),
        };
        envelopes.push(envelope(
            context,
            ordinal,
            kind,
            &payload,
            GameplayOwnerEventRoute {
                source,
                subjects: targets.clone(),
                targets,
                scope: Some("combat".to_owned()),
                tags,
                phase: GameplayEventPhase::PostCommit,
            },
        )?);
    }
    Ok(envelopes)
}

pub fn adapt_state_machine_event(
    context: &GameplayOwnerEventContext,
    event: StateMachineEvent,
) -> Result<GameplayEventEnvelope, GameplayOwnerEventError> {
    let (kind, payload, entity) = match event {
        StateMachineEvent::MachineAttached {
            entity,
            machine,
            state,
            revision,
        } => (
            StandardGameplayEventKind::StateMachineAttached,
            StateMachineGameplayPayload {
                entity: entity.raw(),
                machine: machine.raw(),
                from: None,
                to: state.raw(),
                revision,
            },
            entity,
        ),
        StateMachineEvent::StateTransitioned {
            entity,
            machine,
            from,
            to,
            revision,
        } => (
            StandardGameplayEventKind::StateMachineTransitioned,
            StateMachineGameplayPayload {
                entity: entity.raw(),
                machine: machine.raw(),
                from: Some(from.raw()),
                to: to.raw(),
                revision,
            },
            entity,
        ),
    };
    envelope(
        context,
        0,
        kind,
        &payload,
        GameplayOwnerEventRoute {
            source: None,
            subjects: vec![entity_ref(entity)],
            targets: Vec::new(),
            scope: Some(format!("state-machine:{}", payload.machine)),
            tags: vec!["state-machine".to_owned()],
            phase: GameplayEventPhase::PostCommit,
        },
    )
}

pub fn adapt_process_domain_event(
    context: &GameplayOwnerEventContext,
    event: &DomainEvent,
) -> Result<Option<GameplayEventEnvelope>, GameplayOwnerEventError> {
    let (kind, payload) = match event {
        DomainEvent::ProcessStarted { id } => (
            StandardGameplayEventKind::ProcessStarted,
            ProcessGameplayPayload {
                process: id.raw(),
                mode: None,
                action: "started".to_owned(),
            },
        ),
        DomainEvent::ProcessModeSet { id, mode } => (
            StandardGameplayEventKind::ProcessModeSet,
            ProcessGameplayPayload {
                process: id.raw(),
                mode: Some(mode.raw()),
                action: "modeSet".to_owned(),
            },
        ),
        DomainEvent::ProcessStopped { id } => (
            StandardGameplayEventKind::ProcessStopped,
            ProcessGameplayPayload {
                process: id.raw(),
                mode: None,
                action: "stopped".to_owned(),
            },
        ),
        _ => return Ok(None),
    };
    envelope(
        context,
        0,
        kind,
        &payload,
        GameplayOwnerEventRoute {
            source: None,
            subjects: Vec::new(),
            targets: Vec::new(),
            scope: Some(format!("process:{}", payload.process)),
            tags: vec![payload.action.clone()],
            phase: GameplayEventPhase::PostCommit,
        },
    )
    .map(Some)
}

pub fn adapt_game_rule_resolution(
    context: &GameplayOwnerEventContext,
    request: &EffectResolutionRequest,
    receipt: &protocol_game_rules::GameRuleResolutionReceipt,
) -> Result<Vec<GameplayEventEnvelope>, GameplayOwnerEventError> {
    if !receipt.accepted {
        return Ok(Vec::new());
    }
    let mut envelopes = Vec::new();
    for delta in &receipt.pending_value_deltas {
        let payload = ValueDeltaGameplayPayload {
            source: request.source.raw(),
            target: request.target.raw(),
            bundle_id: request.bundle_id.clone(),
            channel_id: delta.channel_id.clone(),
            amount: delta.amount,
            request_hash: receipt.request_hash.clone(),
            replay_hash: receipt.replay_hash.clone(),
        };
        let ordinal = envelopes.len();
        envelopes.push(envelope(
            context,
            ordinal,
            StandardGameplayEventKind::ValueDeltaResolved,
            &payload,
            GameplayOwnerEventRoute {
                source: Some(entity_ref(request.source)),
                subjects: vec![entity_ref(request.target)],
                targets: vec![entity_ref(request.target)],
                scope: Some("game-rule-resolution".to_owned()),
                tags: vec![delta.channel_id.clone()],
                phase: GameplayEventPhase::PostCommit,
            },
        )?);
    }
    for modifier in &receipt.applied_modifiers {
        let payload = ModifierGameplayPayload {
            source: modifier.source.raw(),
            target: modifier.target.raw(),
            modifier_id: modifier.modifier_id.clone(),
            stacks: modifier.stacks,
            applied_tick: modifier.applied_tick,
            expires_tick: modifier.expires_tick,
            next_tick: modifier.next_tick,
            source_hash: modifier.source_hash.clone(),
            request_hash: receipt.request_hash.clone(),
            replay_hash: receipt.replay_hash.clone(),
        };
        let ordinal = envelopes.len();
        envelopes.push(envelope(
            context,
            ordinal,
            StandardGameplayEventKind::ModifierApplied,
            &payload,
            GameplayOwnerEventRoute {
                source: Some(entity_ref(modifier.source)),
                subjects: vec![entity_ref(modifier.target)],
                targets: vec![entity_ref(modifier.target)],
                scope: Some("game-rule-resolution".to_owned()),
                tags: vec![modifier.modifier_id.clone()],
                phase: GameplayEventPhase::PostCommit,
            },
        )?);
    }
    Ok(envelopes)
}

pub fn adapt_session_tick(
    context: &GameplayOwnerEventContext,
) -> Result<GameplayEventEnvelope, GameplayOwnerEventError> {
    envelope(
        context,
        0,
        StandardGameplayEventKind::SessionTick,
        &SessionTickGameplayPayload { tick: context.tick },
        GameplayOwnerEventRoute {
            source: None,
            subjects: Vec::new(),
            targets: Vec::new(),
            scope: Some("session".to_owned()),
            tags: vec!["tick".to_owned()],
            phase: GameplayEventPhase::ScheduledMoment,
        },
    )
}

pub fn adapt_scheduled_moment(
    context: &GameplayOwnerEventContext,
    schedule_id: String,
    due_tick: u64,
    proposal_kind: GameplayContractRef,
) -> Result<GameplayEventEnvelope, GameplayOwnerEventError> {
    envelope(
        context,
        0,
        StandardGameplayEventKind::ScheduledMomentDue,
        &ScheduledMomentGameplayPayload {
            schedule_id: schedule_id.clone(),
            due_tick,
            proposal_kind,
        },
        GameplayOwnerEventRoute {
            source: None,
            subjects: Vec::new(),
            targets: Vec::new(),
            scope: Some("scheduler".to_owned()),
            tags: vec![schedule_id],
            phase: GameplayEventPhase::ScheduledMoment,
        },
    )
}

fn envelope<T: Serialize>(
    context: &GameplayOwnerEventContext,
    ordinal: usize,
    kind: StandardGameplayEventKind,
    payload: &T,
    mut route: GameplayOwnerEventRoute,
) -> Result<GameplayEventEnvelope, GameplayOwnerEventError> {
    let ordinal = u32::try_from(ordinal).map_err(|_| GameplayOwnerEventError::SequenceOverflow)?;
    let event_sequence = context
        .first_event_sequence
        .checked_add(ordinal)
        .ok_or(GameplayOwnerEventError::SequenceOverflow)?;
    route.subjects.sort_by_key(|subject| subject.entity.raw());
    route.subjects.dedup_by_key(|subject| subject.entity.raw());
    route.targets.sort_by_key(|target| target.entity.raw());
    route.targets.dedup_by_key(|target| target.entity.raw());
    route.tags.sort();
    route.tags.dedup();
    let event = kind.contract();
    let canonical_payload = encode_json(payload).map_err(GameplayOwnerEventError::Encode)?;
    Ok(GameplayEventEnvelope {
        event_id: format!("{}:{}:{}", context.root_id, event_sequence, event.key()),
        event,
        tick: context.tick,
        root_sequence: context.root_sequence,
        wave: 0,
        event_sequence,
        phase: route.phase,
        emitter: GameplayEmitterRef::Owner {
            owner_id: context.owner_id.clone(),
        },
        causation: GameplayCausationRef {
            root_id: context.root_id.clone(),
            parent_event_id: context.parent_event_id.clone(),
            decision_id: None,
        },
        source: route.source,
        subjects: route.subjects,
        targets: route.targets,
        scope: route.scope,
        tags: route.tags,
        payload_hash: gameplay_payload_hash(&canonical_payload),
        canonical_payload,
    })
}

fn entity_ref(entity: core_ids::EntityId) -> GameplayEntityRef {
    GameplayEntityRef { entity }
}

fn miss_reason_label(reason: FireMissReason) -> &'static str {
    match reason {
        FireMissReason::NoTarget => "noTarget",
        FireMissReason::GeometryBlocked => "geometryBlocked",
    }
}

fn sorted_ids(values: impl IntoIterator<Item = u64>) -> Vec<u64> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    values
}
