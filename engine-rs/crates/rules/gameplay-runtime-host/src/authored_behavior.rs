//! Strict compilation and direct execution for bounded authored programs.
//!
//! Public project content names typed/versioned meanings. This module resolves
//! those names to a private numeric plan. No authored program becomes a
//! Gameplay Module, provider, proposal owner, or module-state adapter.

use std::collections::BTreeMap;

use core_ids::{EntityId, ModeId, ProcessId};
use gameplay_module_sdk::{gameplay_contract, gameplay_module_payload_hash};
use protocol_game_extension::{
    GameplayCausationRef, GameplayContractRef, GameplayEmitterRef, GameplayEntityRef,
    GameplayEventEnvelope, GameplayProposalEnvelope,
};
use protocol_project_content::{
    AuthoredBehaviorArgumentDto, AuthoredBehaviorOperationDto, AuthoredBehaviorValueDto,
    ProjectContentDocumentDto, AUTHORED_BEHAVIOR_PACKAGE_SCHEMA_VERSION,
    AUTHORED_PREDICATE_STATE_IS, AUTHORED_VERB_SET_CAPABILITY_ACTIVE,
    AUTHORED_VERB_SET_RELATIVE_TRANSLATION, AUTHORED_VERB_TRANSITION_STATE,
};
use rule_gameplay_fabric::{
    gameplay_payload_hash, GameplayOwnerEventContext, StandardGameplayEventKind,
};
use rule_scheduler::{
    GameplayActionScheduler, GameplaySchedulerCommand, ScheduledActionId, ScheduledGameplayAction,
    TickScheduledActionDraft,
};
use rule_state_machine::MachineInstance;
use serde::{Deserialize, Serialize};
use svc_gameplay_fabric::{
    GameplayEventFilterField, GameplayEventFilterFieldShape, GameplayEventFilterValue,
    GameplayEventFilterValueKind, GameplayFabricRegistry,
};
use svc_project_content::ValidatedProjectContentSet;

use crate::{
    authority_verbs::{
        machine_spec, AuthorityCapability, AuthorityMachine, AuthorityOwnerFact, AuthorityVerb,
        AuthorityVerbExecution, AuthorityVerbExecutor, DIRECT_AUTHORITY_OWNER_ID,
    },
    GameplayRuntimePrefabBootstrap, RuntimeProjectEntitySeed,
};

const AUTHORED_PROGRAM_SCHEDULER_SCHEMA: &str =
    "AuthoredProgramContinuation{programHash:string,packageId:string,behaviorId:string,stepId:string};canonical-json-v1";
const MAX_ACCEPTED_AUTHORED_FACTS: usize = 256;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompiledAuthoredProgram {
    pub schema_version: u32,
    pub content_set_hash: String,
    pub program_hash: String,
    pub sources: Vec<CompiledAuthoredSource>,
    pub machines: Vec<CompiledAuthoredMachine>,
    pub behaviors: Vec<CompiledAuthoredBehavior>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompiledAuthoredSource {
    pub package_id: String,
    pub source_module: String,
    pub source_path: String,
    pub source_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompiledAuthoredMachine {
    pub package_id: String,
    pub machine_id: String,
    pub machine: u64,
    pub entity: u64,
    pub initial_state: u64,
    pub states: Vec<CompiledAuthoredState>,
    pub transitions: Vec<CompiledAuthoredTransition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompiledAuthoredState {
    pub state_id: String,
    pub state: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompiledAuthoredTransition {
    pub transition_id: String,
    pub from_state: u64,
    pub to_state: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompiledAuthoredSignal {
    pub event: GameplayContractRef,
    pub filter_descriptor_hash: String,
    pub arguments: Vec<CompiledAuthoredSignalArgument>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompiledAuthoredSignalArgument {
    pub name: String,
    pub value: CompiledAuthoredValue,
}

/// Numeric/data-only lowering for the public typed value vocabulary. Signal
/// families are deliberately not enum variants: an exact statically composed
/// event contract resolves the signal at admission and event time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum CompiledAuthoredValue {
    Entity { entity: u64 },
    PrefabPart { instance: u64, role: String },
    StateMachine { machine_index: usize },
    State { machine_index: usize, state: u64 },
    Text { value: String },
    Boolean { value: bool },
    Integer { value: i64 },
    Number { value: f64 },
    Vector3 { value: [f32; 3] },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum CompiledAuthoredPredicate {
    StateIs { machine_index: usize, state: u64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum CompiledAuthoredOperation {
    TransitionState {
        machine_index: usize,
        from_state: u64,
        to_state: u64,
    },
    SetRelativeTranslation {
        entity: u64,
        base_translation: [f32; 3],
        offset: [f32; 3],
    },
    SetCapabilityActive {
        entity: u64,
        capability: String,
        active: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompiledAuthoredStep {
    pub step_id: String,
    pub after_step_id: Option<String>,
    pub delay_ticks: u32,
    pub operations: Vec<CompiledAuthoredOperation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompiledAuthoredBehavior {
    pub package_id: String,
    pub behavior_id: String,
    pub signal: CompiledAuthoredSignal,
    pub predicates: Vec<CompiledAuthoredPredicate>,
    pub steps: Vec<CompiledAuthoredStep>,
}

pub(crate) fn authored_program_step_contract() -> protocol_game_extension::GameplayContractRef {
    gameplay_contract(
        "asha.internal",
        "authored-program-continuation",
        1,
        AUTHORED_PROGRAM_SCHEDULER_SCHEMA,
    )
}

pub(crate) fn compile_authored_program(
    content: &ValidatedProjectContentSet,
    prefabs: &GameplayRuntimePrefabBootstrap,
    entity_seeds: &[RuntimeProjectEntitySeed],
    registry: &GameplayFabricRegistry,
) -> Result<Option<CompiledAuthoredProgram>, String> {
    let mut packages = content
        .result()
        .documents
        .iter()
        .filter_map(|document| match document {
            ProjectContentDocumentDto::BehaviorPackage { package, .. } => Some(package),
            _ => None,
        })
        .collect::<Vec<_>>();
    if packages.is_empty() {
        return Ok(None);
    }
    packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));

    let entities = entity_seeds
        .iter()
        .map(|seed| (seed.instance_id.as_str(), seed))
        .collect::<BTreeMap<_, _>>();
    let prefab_instances = prefabs
        .placements
        .iter()
        .map(|placement| (placement.scene_instance_id.as_str(), placement.instance))
        .collect::<BTreeMap<_, _>>();
    let mut sources = Vec::new();
    let mut machines = Vec::new();
    let mut machine_lookup = BTreeMap::<(String, String), usize>::new();

    for package in &packages {
        sources.push(CompiledAuthoredSource {
            package_id: package.package_id.clone(),
            source_module: package.provenance.source_module.clone(),
            source_path: package.provenance.source_path.clone(),
            source_hash: package.provenance.source_hash.clone(),
        });
        let mut package_machines = package.state_machines.iter().collect::<Vec<_>>();
        package_machines.sort_by(|left, right| left.machine_id.cmp(&right.machine_id));
        for machine in package_machines {
            let seed = entities
                .get(machine.target_scene_instance_id.as_str())
                .copied()
                .ok_or_else(|| {
                    format!(
                        "{}: authored machine `{}` target is not materialized",
                        package.provenance.source_path, machine.machine_id
                    )
                })?;
            let machine_number = u64::try_from(machines.len())
                .map_err(|_| "authored machine identity overflow".to_owned())?
                .saturating_add(1);
            let mut source_states = machine.states.iter().collect::<Vec<_>>();
            source_states.sort_by(|left, right| left.state_id.cmp(&right.state_id));
            let states = source_states
                .into_iter()
                .enumerate()
                .map(|(index, state)| CompiledAuthoredState {
                    state_id: state.state_id.clone(),
                    state: u64::try_from(index).unwrap_or(u64::MAX).saturating_add(1),
                })
                .collect::<Vec<_>>();
            let resolve_state = |state_id: &str| {
                states
                    .iter()
                    .find(|state| state.state_id == state_id)
                    .map(|state| state.state)
                    .ok_or_else(|| format!("authored state `{state_id}` was not compiled"))
            };
            let mut source_transitions = machine.transitions.iter().collect::<Vec<_>>();
            source_transitions.sort_by(|left, right| left.transition_id.cmp(&right.transition_id));
            let transitions = source_transitions
                .into_iter()
                .map(|transition| {
                    Ok(CompiledAuthoredTransition {
                        transition_id: transition.transition_id.clone(),
                        from_state: resolve_state(&transition.from_state_id)?,
                        to_state: resolve_state(&transition.to_state_id)?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            machine_lookup.insert(
                (package.package_id.clone(), machine.machine_id.clone()),
                machines.len(),
            );
            machines.push(CompiledAuthoredMachine {
                package_id: package.package_id.clone(),
                machine_id: machine.machine_id.clone(),
                machine: machine_number,
                entity: seed.entity.raw(),
                initial_state: resolve_state(&machine.initial_state_id)?,
                states,
                transitions,
            });
        }
    }

    let mut behaviors = Vec::new();
    for package in packages {
        let mut package_behaviors = package.behaviors.iter().collect::<Vec<_>>();
        package_behaviors.sort_by(|left, right| left.behavior_id.cmp(&right.behavior_id));
        let signal_references = AuthoredSignalCompilationReferences {
            package_id: &package.package_id,
            entities: &entities,
            prefab_instances: &prefab_instances,
            machine_lookup: &machine_lookup,
            machines: &machines,
        };
        for behavior in package_behaviors {
            let signal = compile_signal(
                &behavior.signal.signal.semantic_id,
                behavior.signal.signal.version,
                &behavior.signal.arguments,
                &signal_references,
                registry,
            )?;
            let predicates = behavior
                .conditions
                .iter()
                .map(|condition| {
                    if condition.predicate.semantic_id != AUTHORED_PREDICATE_STATE_IS {
                        return Err("unsupported authored predicate reached compilation".to_owned());
                    }
                    let AuthoredBehaviorValueDto::State {
                        machine_id,
                        state_id,
                    } = argument(&condition.arguments, "state")?
                    else {
                        return Err("state-is predicate has an invalid compiled value".to_owned());
                    };
                    let machine_index =
                        resolve_machine(&machine_lookup, &package.package_id, machine_id)?;
                    let state = resolve_machine_state(&machines[machine_index], state_id)?;
                    Ok(CompiledAuthoredPredicate::StateIs {
                        machine_index,
                        state,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let mut source_steps = behavior.steps.iter().collect::<Vec<_>>();
            source_steps.sort_by(|left, right| left.step_id.cmp(&right.step_id));
            let steps = source_steps
                .into_iter()
                .map(|step| {
                    let operations = step
                        .operations
                        .iter()
                        .map(|operation| {
                            compile_operation(
                                operation,
                                &package.package_id,
                                &machine_lookup,
                                &machines,
                                &entities,
                            )
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    Ok(CompiledAuthoredStep {
                        step_id: step.step_id.clone(),
                        after_step_id: step.after_step_ids.first().cloned(),
                        delay_ticks: step.delay_ticks,
                        operations,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            behaviors.push(CompiledAuthoredBehavior {
                package_id: package.package_id.clone(),
                behavior_id: behavior.behavior_id.clone(),
                signal,
                predicates,
                steps,
            });
        }
    }

    let mut program = CompiledAuthoredProgram {
        schema_version: AUTHORED_BEHAVIOR_PACKAGE_SCHEMA_VERSION,
        content_set_hash: content.set_hash().to_owned(),
        program_hash: String::new(),
        sources,
        machines,
        behaviors,
    };
    // Source paths/hashes identify diagnostic provenance. They are deliberately
    // excluded from the executable identity: moving or reformatting equivalent
    // TypeScript must not create different Rust authority semantics.
    program.program_hash = gameplay_module_payload_hash(
        &serde_json::to_vec(&(
            program.schema_version,
            &program.machines,
            &program.behaviors,
        ))
        .map_err(|error| format!("compiled authored program did not serialize: {error}"))?,
    );
    Ok(Some(program))
}

struct AuthoredSignalCompilationReferences<'a> {
    package_id: &'a str,
    entities: &'a BTreeMap<&'a str, &'a RuntimeProjectEntitySeed>,
    prefab_instances: &'a BTreeMap<&'a str, u64>,
    machine_lookup: &'a BTreeMap<(String, String), usize>,
    machines: &'a [CompiledAuthoredMachine],
}

fn compile_signal(
    semantic_id: &str,
    version: u32,
    arguments: &[AuthoredBehaviorArgumentDto],
    references: &AuthoredSignalCompilationReferences<'_>,
    registry: &GameplayFabricRegistry,
) -> Result<CompiledAuthoredSignal, String> {
    let event = registry
        .published_event(&format!("{semantic_id}.v{version}"))
        .cloned()
        .ok_or_else(|| "unpublished authored signal reached compilation".to_owned())?;
    let mut compiled_arguments = arguments
        .iter()
        .map(|source| {
            Ok(CompiledAuthoredSignalArgument {
                name: source.name.clone(),
                value: compile_authored_value(&source.value, references)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    compiled_arguments.sort_by(|left, right| left.name.cmp(&right.name));
    let filter_fields = compiled_arguments
        .iter()
        .map(compiled_signal_argument_shape)
        .collect::<Result<Vec<_>, String>>()?;
    let filter_descriptor_hash = registry
        .validate_event_filter_shape(&event, &filter_fields)
        .map_err(|error| error.to_string())?;
    Ok(CompiledAuthoredSignal {
        event,
        filter_descriptor_hash,
        arguments: compiled_arguments,
    })
}

fn compile_authored_value(
    value: &AuthoredBehaviorValueDto,
    references: &AuthoredSignalCompilationReferences<'_>,
) -> Result<CompiledAuthoredValue, String> {
    match value {
        AuthoredBehaviorValueDto::SceneEntity { scene_instance_id } => {
            Ok(CompiledAuthoredValue::Entity {
                entity: references
                    .entities
                    .get(scene_instance_id.as_str())
                    .ok_or_else(|| "authored signal entity was not materialized".to_owned())?
                    .entity
                    .raw(),
            })
        }
        AuthoredBehaviorValueDto::PrefabPart {
            scene_instance_id,
            role,
        } => Ok(CompiledAuthoredValue::PrefabPart {
            instance: *references
                .prefab_instances
                .get(scene_instance_id.as_str())
                .ok_or_else(|| "prefab-part signal instance was not materialized".to_owned())?,
            role: role.clone(),
        }),
        AuthoredBehaviorValueDto::StateMachine { machine_id } => {
            Ok(CompiledAuthoredValue::StateMachine {
                machine_index: resolve_machine(
                    references.machine_lookup,
                    references.package_id,
                    machine_id,
                )?,
            })
        }
        AuthoredBehaviorValueDto::State {
            machine_id,
            state_id,
        } => {
            let machine_index =
                resolve_machine(references.machine_lookup, references.package_id, machine_id)?;
            Ok(CompiledAuthoredValue::State {
                machine_index,
                state: resolve_machine_state(&references.machines[machine_index], state_id)?,
            })
        }
        AuthoredBehaviorValueDto::Text { value } => Ok(CompiledAuthoredValue::Text {
            value: value.clone(),
        }),
        AuthoredBehaviorValueDto::Boolean { value } => {
            Ok(CompiledAuthoredValue::Boolean { value: *value })
        }
        AuthoredBehaviorValueDto::Integer { value } => {
            Ok(CompiledAuthoredValue::Integer { value: *value })
        }
        AuthoredBehaviorValueDto::Number { value } => {
            Ok(CompiledAuthoredValue::Number { value: *value })
        }
        AuthoredBehaviorValueDto::Vector3 { value } => {
            Ok(CompiledAuthoredValue::Vector3 { value: *value })
        }
    }
}

fn compile_operation(
    operation: &AuthoredBehaviorOperationDto,
    package_id: &str,
    machine_lookup: &BTreeMap<(String, String), usize>,
    machines: &[CompiledAuthoredMachine],
    entities: &BTreeMap<&str, &RuntimeProjectEntitySeed>,
) -> Result<CompiledAuthoredOperation, String> {
    match operation.verb.semantic_id.as_str() {
        AUTHORED_VERB_TRANSITION_STATE => {
            let AuthoredBehaviorValueDto::StateMachine { machine_id } =
                argument(&operation.arguments, "machine")?
            else {
                return Err("transition-state machine value is invalid".to_owned());
            };
            let AuthoredBehaviorValueDto::Text { value } =
                argument(&operation.arguments, "transition")?
            else {
                return Err("transition-state transition value is invalid".to_owned());
            };
            let machine_index = resolve_machine(machine_lookup, package_id, machine_id)?;
            let transition = machines[machine_index]
                .transitions
                .iter()
                .find(|transition| transition.transition_id == *value)
                .ok_or_else(|| "authored transition was not compiled".to_owned())?;
            Ok(CompiledAuthoredOperation::TransitionState {
                machine_index,
                from_state: transition.from_state,
                to_state: transition.to_state,
            })
        }
        AUTHORED_VERB_SET_RELATIVE_TRANSLATION => {
            let seed = operation_entity(&operation.arguments, entities)?;
            let AuthoredBehaviorValueDto::Vector3 { value } =
                argument(&operation.arguments, "value")?
            else {
                return Err("set-relative-translation value is invalid".to_owned());
            };
            Ok(CompiledAuthoredOperation::SetRelativeTranslation {
                entity: seed.entity.raw(),
                base_translation: seed.world_translation,
                offset: *value,
            })
        }
        AUTHORED_VERB_SET_CAPABILITY_ACTIVE => {
            let seed = operation_entity(&operation.arguments, entities)?;
            let AuthoredBehaviorValueDto::Text { value: capability } =
                argument(&operation.arguments, "capability")?
            else {
                return Err("set-capability-active capability is invalid".to_owned());
            };
            let AuthoredBehaviorValueDto::Boolean { value: active } =
                argument(&operation.arguments, "active")?
            else {
                return Err("set-capability-active flag is invalid".to_owned());
            };
            Ok(CompiledAuthoredOperation::SetCapabilityActive {
                entity: seed.entity.raw(),
                capability: capability.clone(),
                active: *active,
            })
        }
        _ => Err("unsupported authored verb reached compilation".to_owned()),
    }
}

fn operation_entity<'a>(
    arguments: &[AuthoredBehaviorArgumentDto],
    entities: &BTreeMap<&str, &'a RuntimeProjectEntitySeed>,
) -> Result<&'a RuntimeProjectEntitySeed, String> {
    let AuthoredBehaviorValueDto::SceneEntity { scene_instance_id } =
        argument(arguments, "entity")?
    else {
        return Err("authored entity argument is invalid".to_owned());
    };
    entities
        .get(scene_instance_id.as_str())
        .copied()
        .ok_or_else(|| "authored operation entity was not materialized".to_owned())
}

fn argument<'a>(
    arguments: &'a [AuthoredBehaviorArgumentDto],
    name: &str,
) -> Result<&'a AuthoredBehaviorValueDto, String> {
    arguments
        .iter()
        .find(|argument| argument.name == name)
        .map(|argument| &argument.value)
        .ok_or_else(|| format!("missing compiled authored argument `{name}`"))
}

fn resolve_machine(
    lookup: &BTreeMap<(String, String), usize>,
    package_id: &str,
    machine_id: &str,
) -> Result<usize, String> {
    lookup
        .get(&(package_id.to_owned(), machine_id.to_owned()))
        .copied()
        .ok_or_else(|| format!("authored machine `{machine_id}` was not compiled"))
}

fn resolve_machine_state(machine: &CompiledAuthoredMachine, state_id: &str) -> Result<u64, String> {
    machine
        .states
        .iter()
        .find(|state| state.state_id == state_id)
        .map(|state| state.state)
        .ok_or_else(|| format!("authored state `{state_id}` was not compiled"))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AuthoredProgramSnapshot {
    pub schema_version: u32,
    pub program_hash: String,
    pub machines: Vec<AuthoredMachineSnapshot>,
    pub accepted_facts: Vec<AuthorityOwnerFact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AuthoredMachineSnapshot {
    pub machine: u64,
    pub entity: u64,
    pub current_state: u64,
    pub revision: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct AuthoredProgramRuntime {
    plan: CompiledAuthoredProgram,
    machines: Vec<AuthorityMachine>,
    accepted_facts: Vec<AuthorityOwnerFact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AuthoredProgramContinuation {
    program_hash: String,
    package_id: String,
    behavior_id: String,
    step_id: String,
}

impl AuthoredProgramRuntime {
    pub fn activate(plan: CompiledAuthoredProgram) -> Self {
        let machines = build_runtime_machines(&plan, None)
            .expect("strictly compiled authored program has valid initial machines");
        Self {
            plan,
            machines,
            accepted_facts: Vec::new(),
        }
    }

    pub fn restore(
        plan: CompiledAuthoredProgram,
        snapshot: AuthoredProgramSnapshot,
    ) -> Result<Self, String> {
        if snapshot.schema_version != AUTHORED_BEHAVIOR_PACKAGE_SCHEMA_VERSION
            || snapshot.program_hash != plan.program_hash
        {
            return Err(
                "saved authored program identity does not match admitted content".to_owned(),
            );
        }
        let machines = build_runtime_machines(&plan, Some(&snapshot.machines))?;
        Ok(Self {
            plan,
            machines,
            accepted_facts: snapshot.accepted_facts,
        })
    }

    pub fn snapshot(&self) -> AuthoredProgramSnapshot {
        AuthoredProgramSnapshot {
            schema_version: AUTHORED_BEHAVIOR_PACKAGE_SCHEMA_VERSION,
            program_hash: self.plan.program_hash.clone(),
            machines: self
                .machines
                .iter()
                .map(|machine| AuthoredMachineSnapshot {
                    machine: machine.instance.machine.raw(),
                    entity: machine.instance.entity.raw(),
                    current_state: machine.instance.current.raw(),
                    revision: machine.instance.revision,
                })
                .collect(),
            accepted_facts: self.accepted_facts.clone(),
        }
    }

    pub fn state_hash(&self) -> String {
        gameplay_module_payload_hash(
            &serde_json::to_vec(&self.snapshot()).expect("authored program snapshot serializes"),
        )
    }

    pub fn program_hash(&self) -> &str {
        &self.plan.program_hash
    }

    pub fn accepted_facts(&self) -> &[AuthorityOwnerFact] {
        &self.accepted_facts
    }

    /// Resolve the same admitted signal binding and live predicates used by
    /// event reaction. The interaction query and submit paths both call this,
    /// so an unhandled or currently ineligible prefab part cannot be presented
    /// as actionable or accepted as a no-op.
    pub fn prefab_interaction_is_eligible(&self, instance: u64, role: &str) -> bool {
        self.plan
            .behaviors
            .iter()
            .enumerate()
            .any(|(behavior_index, behavior)| {
                behavior.signal.event == StandardGameplayEventKind::PrefabPartInteracted.contract()
                    && prefab_part_interaction_target_matches(&behavior.signal, instance, role)
                    && self.predicates_match(behavior_index)
            })
    }

    pub fn validate_scheduler(&self, scheduler: &GameplayActionScheduler) -> Result<(), String> {
        for action in scheduler.pending_actions() {
            let proposal = match action {
                ScheduledGameplayAction::Tick { proposal, .. }
                | ScheduledGameplayAction::EventConditioned { proposal, .. } => proposal,
            };
            self.validate_continuation(proposal)?;
        }
        for dispatch in scheduler.outstanding_dispatches() {
            self.validate_continuation(&dispatch.proposal)?;
        }
        Ok(())
    }

    pub fn react(
        &mut self,
        registry: &GameplayFabricRegistry,
        events: &[GameplayEventEnvelope],
        entities: &mut core_entity::EntityStore,
        scheduler: &mut GameplayActionScheduler,
    ) -> Result<Vec<GameplayEventEnvelope>, String> {
        let runtime_checkpoint = self.clone();
        let entity_checkpoint = entities.clone();
        let scheduler_checkpoint = scheduler.clone();
        let result = (|| {
            let mut emitted = Vec::new();
            for event in events {
                let mut matching = Vec::new();
                for (index, behavior) in self.plan.behaviors.iter().enumerate() {
                    if signal_matches(registry, &behavior.signal, event)? {
                        matching.push(index);
                    }
                }
                for behavior_index in matching {
                    if !self.predicates_match(behavior_index) {
                        continue;
                    }
                    let roots = self.plan.behaviors[behavior_index]
                        .steps
                        .iter()
                        .enumerate()
                        .filter_map(|(index, step)| step.after_step_id.is_none().then_some(index))
                        .collect::<Vec<_>>();
                    for step_index in roots {
                        self.execute_or_schedule_step(
                            behavior_index,
                            step_index,
                            event.tick,
                            &event.event_id,
                            event.root_sequence,
                            Some(event.event_id.clone()),
                            entities,
                            scheduler,
                            &mut emitted,
                        )?;
                    }
                }
            }
            Ok(emitted)
        })();
        if result.is_err() {
            *self = runtime_checkpoint;
            *entities = entity_checkpoint;
            *scheduler = scheduler_checkpoint;
        }
        result
    }

    pub fn execute_continuation(
        &mut self,
        proposal: &GameplayProposalEnvelope,
        entities: &mut core_entity::EntityStore,
        scheduler: &mut GameplayActionScheduler,
    ) -> Result<AuthorityVerbExecution, String> {
        if proposal.proposal != authored_program_step_contract() {
            return Err("scheduled proposal is not an authored-program continuation".to_owned());
        }
        let (behavior_index, step_index) = self.validate_continuation(proposal)?;
        let mut emitted = Vec::new();
        let execution = self.execute_step(
            behavior_index,
            step_index,
            proposal.tick,
            &proposal.causation.root_id,
            proposal.root_sequence,
            proposal.originating_event_id.clone(),
            entities,
        )?;
        emitted.extend(execution.events.iter().cloned());
        let children = self.child_steps(behavior_index, step_index);
        for child in children {
            self.execute_or_schedule_step(
                behavior_index,
                child,
                proposal.tick,
                &proposal.causation.root_id,
                proposal.root_sequence,
                proposal.originating_event_id.clone(),
                entities,
                scheduler,
                &mut emitted,
            )?;
        }
        Ok(AuthorityVerbExecution {
            facts: execution.facts,
            events: emitted,
        })
    }

    pub fn continuation_is_ready(
        &self,
        proposal: &GameplayProposalEnvelope,
        entities: &core_entity::EntityStore,
    ) -> Result<bool, String> {
        let (behavior_index, step_index) = self.validate_continuation(proposal)?;
        let mut trial_runtime = self.clone();
        let mut trial_entities = entities.clone();
        match trial_runtime.execute_step(
            behavior_index,
            step_index,
            proposal.tick,
            &proposal.causation.root_id,
            proposal.root_sequence,
            proposal.originating_event_id.clone(),
            &mut trial_entities,
        ) {
            Ok(_) => Ok(true),
            Err(code) if code == "authorityVerbCollisionOccupied" => Ok(false),
            Err(code) => Err(code),
        }
    }

    fn validate_continuation(
        &self,
        proposal: &GameplayProposalEnvelope,
    ) -> Result<(usize, usize), String> {
        if proposal.proposal != authored_program_step_contract() {
            return Err("scheduled proposal is not an authored-program continuation".to_owned());
        }
        let continuation: AuthoredProgramContinuation =
            serde_json::from_slice(&proposal.canonical_payload)
                .map_err(|_| "scheduled authored continuation did not decode".to_owned())?;
        if continuation.program_hash != self.plan.program_hash {
            return Err("scheduled authored continuation targets stale program content".to_owned());
        }
        let behavior_index = self
            .plan
            .behaviors
            .iter()
            .position(|behavior| {
                behavior.package_id == continuation.package_id
                    && behavior.behavior_id == continuation.behavior_id
            })
            .ok_or_else(|| "scheduled authored behavior was not compiled".to_owned())?;
        let step_index = self.plan.behaviors[behavior_index]
            .steps
            .iter()
            .position(|step| step.step_id == continuation.step_id)
            .ok_or_else(|| "scheduled authored step was not compiled".to_owned())?;
        Ok((behavior_index, step_index))
    }

    fn predicates_match(&self, behavior_index: usize) -> bool {
        self.plan.behaviors[behavior_index]
            .predicates
            .iter()
            .all(|predicate| match predicate {
                CompiledAuthoredPredicate::StateIs {
                    machine_index,
                    state,
                } => self
                    .machines
                    .get(*machine_index)
                    .is_some_and(|machine| machine.instance.current.raw() == *state),
            })
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_or_schedule_step(
        &mut self,
        behavior_index: usize,
        step_index: usize,
        tick: u64,
        root_id: &str,
        root_sequence: u64,
        parent_event_id: Option<String>,
        entities: &mut core_entity::EntityStore,
        scheduler: &mut GameplayActionScheduler,
        emitted: &mut Vec<GameplayEventEnvelope>,
    ) -> Result<(), String> {
        let step = &self.plan.behaviors[behavior_index].steps[step_index];
        if step.delay_ticks > 0 {
            return self.schedule_step(
                behavior_index,
                step_index,
                tick,
                root_id,
                root_sequence,
                parent_event_id,
                scheduler,
            );
        }
        let execution = self.execute_step(
            behavior_index,
            step_index,
            tick,
            root_id,
            root_sequence,
            parent_event_id.clone(),
            entities,
        )?;
        emitted.extend(execution.events);
        for child in self.child_steps(behavior_index, step_index) {
            self.execute_or_schedule_step(
                behavior_index,
                child,
                tick,
                root_id,
                root_sequence,
                parent_event_id.clone(),
                entities,
                scheduler,
                emitted,
            )?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_step(
        &mut self,
        behavior_index: usize,
        step_index: usize,
        tick: u64,
        root_id: &str,
        root_sequence: u64,
        parent_event_id: Option<String>,
        entities: &mut core_entity::EntityStore,
    ) -> Result<AuthorityVerbExecution, String> {
        let operations = self.plan.behaviors[behavior_index].steps[step_index]
            .operations
            .clone();
        let verbs = operations
            .iter()
            .map(|operation| self.runtime_verb(operation))
            .collect::<Result<Vec<_>, String>>()?;
        let execution = AuthorityVerbExecutor {
            entities,
            machines: &mut self.machines,
        }
        .execute_atomic(
            &verbs,
            &GameplayOwnerEventContext {
                owner_id: DIRECT_AUTHORITY_OWNER_ID.to_owned(),
                tick,
                root_id: root_id.to_owned(),
                root_sequence,
                first_event_sequence: 0,
                parent_event_id,
            },
        )
        .map_err(|error| error.code().to_owned())?;
        self.accepted_facts.extend(execution.facts.iter().cloned());
        if self.accepted_facts.len() > MAX_ACCEPTED_AUTHORED_FACTS {
            let excess = self.accepted_facts.len() - MAX_ACCEPTED_AUTHORED_FACTS;
            self.accepted_facts.drain(..excess);
        }
        Ok(execution)
    }

    fn runtime_verb(&self, operation: &CompiledAuthoredOperation) -> Result<AuthorityVerb, String> {
        match operation {
            CompiledAuthoredOperation::TransitionState {
                machine_index,
                from_state,
                to_state,
            } => {
                let machine = self
                    .machines
                    .get(*machine_index)
                    .ok_or_else(|| "compiled authored machine is unavailable".to_owned())?;
                Ok(AuthorityVerb::TransitionState {
                    machine_index: *machine_index,
                    expected: ModeId::new(*from_state),
                    next: ModeId::new(*to_state),
                    expected_revision: machine.instance.revision,
                })
            }
            CompiledAuthoredOperation::SetRelativeTranslation {
                entity,
                base_translation,
                offset,
            } => Ok(AuthorityVerb::SetRelativeTranslation {
                entity: EntityId::new(*entity),
                base_translation: *base_translation,
                offset: *offset,
            }),
            CompiledAuthoredOperation::SetCapabilityActive {
                entity,
                capability,
                active,
            } => Ok(AuthorityVerb::SetCapabilityActive {
                entity: EntityId::new(*entity),
                capability: match capability.as_str() {
                    "collision" => AuthorityCapability::Collision,
                    _ => return Err("compiled authored capability is unsupported".to_owned()),
                },
                active: *active,
            }),
        }
    }

    fn child_steps(&self, behavior_index: usize, step_index: usize) -> Vec<usize> {
        let parent = &self.plan.behaviors[behavior_index].steps[step_index].step_id;
        self.plan.behaviors[behavior_index]
            .steps
            .iter()
            .enumerate()
            .filter_map(|(index, step)| {
                (step.after_step_id.as_deref() == Some(parent.as_str())).then_some(index)
            })
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn schedule_step(
        &self,
        behavior_index: usize,
        step_index: usize,
        tick: u64,
        root_id: &str,
        root_sequence: u64,
        parent_event_id: Option<String>,
        scheduler: &mut GameplayActionScheduler,
    ) -> Result<(), String> {
        let behavior = &self.plan.behaviors[behavior_index];
        let step = &behavior.steps[step_index];
        let execute_at = tick
            .checked_add(u64::from(step.delay_ticks))
            .ok_or_else(|| "authored continuation tick overflow".to_owned())?;
        let continuation = AuthoredProgramContinuation {
            program_hash: self.plan.program_hash.clone(),
            package_id: behavior.package_id.clone(),
            behavior_id: behavior.behavior_id.clone(),
            step_id: step.step_id.clone(),
        };
        let canonical_payload = serde_json::to_vec(&continuation)
            .map_err(|error| format!("authored continuation did not serialize: {error}"))?;
        let identity = gameplay_module_payload_hash(
            format!(
                "{}|{}|{}|{}|{}",
                self.plan.program_hash,
                behavior.package_id,
                behavior.behavior_id,
                step.step_id,
                root_id
            )
            .as_bytes(),
        );
        let action_id = ScheduledActionId::new(format!(
            "authored.{}",
            identity.strip_prefix("fnv1a64:").unwrap_or(&identity)
        ));
        let targets = step
            .operations
            .iter()
            .filter_map(|operation| match operation {
                CompiledAuthoredOperation::TransitionState { machine_index, .. } => self
                    .plan
                    .machines
                    .get(*machine_index)
                    .map(|machine| machine.entity),
                CompiledAuthoredOperation::SetRelativeTranslation { entity, .. }
                | CompiledAuthoredOperation::SetCapabilityActive { entity, .. } => Some(*entity),
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|entity| GameplayEntityRef {
                entity: EntityId::new(entity),
            })
            .collect();
        let causation = GameplayCausationRef {
            root_id: format!("scheduled:{}", action_id.as_str()),
            parent_event_id: parent_event_id.clone(),
            decision_id: None,
        };
        let proposal = GameplayProposalEnvelope {
            proposal_id: format!("{}/execute", action_id.as_str()),
            proposal: authored_program_step_contract(),
            tick: execute_at,
            root_sequence,
            wave: 0,
            proposal_sequence: 0,
            emitter: GameplayEmitterRef::Owner {
                owner_id: DIRECT_AUTHORITY_OWNER_ID.to_owned(),
            },
            causation: causation.clone(),
            originating_event_id: parent_event_id,
            source: None,
            targets,
            payload_hash: gameplay_payload_hash(&canonical_payload),
            canonical_payload,
        };
        scheduler
            .apply(GameplaySchedulerCommand::ScheduleTick(
                TickScheduledActionDraft {
                    id: action_id,
                    execute_at,
                    priority: 0,
                    proposal,
                    source: GameplayEmitterRef::Owner {
                        owner_id: DIRECT_AUTHORITY_OWNER_ID.to_owned(),
                    },
                    causation,
                },
            ))
            .map(|_| ())
            .map_err(|error| format!("authored continuation schedule rejected: {error}"))
    }
}

fn build_runtime_machines(
    plan: &CompiledAuthoredProgram,
    restored: Option<&[AuthoredMachineSnapshot]>,
) -> Result<Vec<AuthorityMachine>, String> {
    if restored.is_some_and(|saved| saved.len() != plan.machines.len()) {
        return Err("saved authored machine set does not match admitted content".to_owned());
    }
    plan.machines
        .iter()
        .enumerate()
        .map(|(index, machine)| {
            let saved = restored.and_then(|saved| saved.get(index));
            if saved.is_some_and(|saved| {
                saved.machine != machine.machine
                    || saved.entity != machine.entity
                    || !machine
                        .states
                        .iter()
                        .any(|state| state.state == saved.current_state)
            }) {
                return Err("saved authored machine identity or state is invalid".to_owned());
            }
            let transitions = machine
                .transitions
                .iter()
                .map(|transition| (transition.from_state, transition.to_state));
            Ok(AuthorityMachine {
                spec: machine_spec(
                    machine.machine,
                    machine.states.iter().map(|state| state.state),
                    transitions,
                ),
                instance: MachineInstance {
                    entity: EntityId::new(machine.entity),
                    machine: ProcessId::new(machine.machine),
                    current: ModeId::new(
                        saved.map_or(machine.initial_state, |saved| saved.current_state),
                    ),
                    revision: saved.map_or(0, |saved| saved.revision),
                },
            })
        })
        .collect()
}

fn signal_matches(
    registry: &GameplayFabricRegistry,
    signal: &CompiledAuthoredSignal,
    event: &GameplayEventEnvelope,
) -> Result<bool, String> {
    if signal.event != event.event {
        return Ok(false);
    }
    let fields = signal
        .arguments
        .iter()
        .map(compiled_signal_filter_field)
        .collect::<Result<Vec<_>, String>>()?;
    registry
        .matches_event_filter(&event.event, &event.canonical_payload, &fields)
        .map_err(|error| error.to_string())
}

fn prefab_part_interaction_target_matches(
    signal: &CompiledAuthoredSignal,
    candidate_instance: u64,
    candidate_role: &str,
) -> bool {
    matches!(
        compiled_signal_argument(signal, "part"),
        Some(CompiledAuthoredValue::PrefabPart { instance, role })
            if *instance == candidate_instance && role == candidate_role
    )
}

fn compiled_signal_argument<'a>(
    signal: &'a CompiledAuthoredSignal,
    name: &str,
) -> Option<&'a CompiledAuthoredValue> {
    signal
        .arguments
        .iter()
        .find(|argument| argument.name == name)
        .map(|argument| &argument.value)
}

fn compiled_signal_argument_shape(
    argument: &CompiledAuthoredSignalArgument,
) -> Result<GameplayEventFilterFieldShape, String> {
    let value_kind = match argument.value {
        CompiledAuthoredValue::Entity { .. } => GameplayEventFilterValueKind::Entity,
        CompiledAuthoredValue::PrefabPart { .. } => GameplayEventFilterValueKind::PrefabPart,
        CompiledAuthoredValue::Text { .. } => GameplayEventFilterValueKind::Text,
        CompiledAuthoredValue::Boolean { .. } => GameplayEventFilterValueKind::Boolean,
        CompiledAuthoredValue::Integer { .. } => GameplayEventFilterValueKind::Integer,
        CompiledAuthoredValue::Number { .. } => GameplayEventFilterValueKind::Number,
        CompiledAuthoredValue::Vector3 { .. } => GameplayEventFilterValueKind::Vector3,
        CompiledAuthoredValue::StateMachine { .. } | CompiledAuthoredValue::State { .. } => {
            return Err(format!(
                "authored signal filter `{}` cannot contain symbolic state",
                argument.name
            ));
        }
    };
    Ok(GameplayEventFilterFieldShape {
        name: argument.name.clone(),
        value_kind,
    })
}

fn compiled_signal_filter_field(
    argument: &CompiledAuthoredSignalArgument,
) -> Result<GameplayEventFilterField, String> {
    let value = match &argument.value {
        CompiledAuthoredValue::Entity { entity } => GameplayEventFilterValue::Entity(*entity),
        CompiledAuthoredValue::PrefabPart { instance, role } => {
            GameplayEventFilterValue::PrefabPart {
                instance: *instance,
                role: role.clone(),
            }
        }
        CompiledAuthoredValue::Text { value } => GameplayEventFilterValue::Text(value.clone()),
        CompiledAuthoredValue::Boolean { value } => GameplayEventFilterValue::Boolean(*value),
        CompiledAuthoredValue::Integer { value } => GameplayEventFilterValue::Integer(*value),
        CompiledAuthoredValue::Number { value } => GameplayEventFilterValue::Number(*value),
        CompiledAuthoredValue::Vector3 { value } => GameplayEventFilterValue::Vector3(*value),
        CompiledAuthoredValue::StateMachine { .. } | CompiledAuthoredValue::State { .. } => {
            return Err(format!(
                "compiled signal filter `{}` contains symbolic state",
                argument.name
            ));
        }
    };
    Ok(GameplayEventFilterField {
        name: argument.name.clone(),
        value,
    })
}
