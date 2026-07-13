use serde::{Deserialize, Serialize};

use super::{
    stable_hash, ActiveTransition, AnimationAuthorityError, ControllerInstance, ValidatedGraph,
};

/// Durable identity of the accepted gameplay fact that caused an animation
/// authority evaluation. Renderer callbacks never construct this value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationInputOrigin {
    pub source_fact_id: String,
    pub authority_tick: u64,
    pub causation_id: String,
    pub correlation_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimationTransitionFactMoment {
    Started,
    Completed,
}

/// Replayable gameplay timing fact emitted by controller authority when a
/// semantic transition starts or completes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationTransitionTimingFact {
    pub fact_id: String,
    pub source: AnimationInputOrigin,
    pub controller_input_sequence: u64,
    pub controller_tick: u64,
    pub entity: u64,
    pub graph_id: String,
    pub transition_id: String,
    pub from_state_id: String,
    pub to_state_id: String,
    pub moment: AnimationTransitionFactMoment,
    pub duration_ticks: u32,
    pub resulting_revision: u64,
    pub fact_hash: String,
}

pub(super) fn transition_timing_fact(
    graph: &ValidatedGraph,
    controller: &ControllerInstance,
    transition: &ActiveTransition,
    input_sequence: u64,
    tick: u64,
    moment: AnimationTransitionFactMoment,
) -> AnimationTransitionTimingFact {
    let moment_label = match moment {
        AnimationTransitionFactMoment::Started => "started",
        AnimationTransitionFactMoment::Completed => "completed",
    };
    let mut fact = AnimationTransitionTimingFact {
        fact_id: format!(
            "{}:animation:{}:{}:{moment_label}",
            transition.origin.source_fact_id,
            controller.machine.entity.raw(),
            transition.transition_id
        ),
        source: transition.origin.clone(),
        controller_input_sequence: input_sequence,
        controller_tick: tick,
        entity: controller.machine.entity.raw(),
        graph_id: graph.definition.graph_id.clone(),
        transition_id: transition.transition_id.clone(),
        from_state_id: transition.from_state_id.clone(),
        to_state_id: transition.to_state_id.clone(),
        moment,
        duration_ticks: transition.duration_ticks,
        resulting_revision: controller.machine.revision,
        fact_hash: String::new(),
    };
    let encoded = serde_json::to_vec(&fact).expect("animation transition fact serializes");
    fact.fact_hash = stable_hash(&encoded);
    fact
}

pub(super) fn validate_input_origin(
    origin: &AnimationInputOrigin,
) -> Result<(), AnimationAuthorityError> {
    if origin.source_fact_id.is_empty()
        || origin.causation_id.is_empty()
        || origin.correlation_id.is_empty()
    {
        return Err(AnimationAuthorityError::InvalidOrigin(
            "source fact, causation, and correlation ids must be non-empty".to_string(),
        ));
    }
    Ok(())
}
