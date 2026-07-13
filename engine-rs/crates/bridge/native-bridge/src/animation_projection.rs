use napi_derive::napi;
use runtime_bridge_api::{
    AnimationControllerProjectionState, AnimationProjectionDescriptor, AnimationProjectionOp,
    AnimationResolvedMotion, AnimationTransitionFactMoment, AnimationTransitionFactRef,
    AnimationTransitionProjection,
};

#[napi(object)]
pub struct NativeAnimationResolvedMotion {
    pub clip_a: String,
    pub clip_b: Option<String>,
    pub blend_weight_milli: i32,
    pub speed_milli: i32,
}

impl From<AnimationResolvedMotion> for NativeAnimationResolvedMotion {
    fn from(value: AnimationResolvedMotion) -> Self {
        Self {
            clip_a: value.clip_a,
            clip_b: value.clip_b,
            blend_weight_milli: value.blend_weight_milli,
            speed_milli: value.speed_milli,
        }
    }
}

#[napi(object)]
pub struct NativeAnimationTransitionProjection {
    pub transition_id: String,
    pub from_state_id: String,
    pub to_state_id: String,
    pub elapsed_ticks: u32,
    pub duration_ticks: u32,
    pub target_motion: NativeAnimationResolvedMotion,
}

impl From<AnimationTransitionProjection> for NativeAnimationTransitionProjection {
    fn from(value: AnimationTransitionProjection) -> Self {
        Self {
            transition_id: value.transition_id,
            from_state_id: value.from_state_id,
            to_state_id: value.to_state_id,
            elapsed_ticks: value.elapsed_ticks,
            duration_ticks: value.duration_ticks,
            target_motion: value.target_motion.into(),
        }
    }
}

#[napi(object)]
pub struct NativeAnimationTransitionFactRef {
    pub fact_id: String,
    pub source_fact_id: String,
    pub authority_tick: i64,
    pub controller_input_sequence: i64,
    pub controller_tick: i64,
    pub causation_id: String,
    pub correlation_id: String,
    pub transition_id: String,
    pub from_state_id: String,
    pub to_state_id: String,
    pub moment: String,
    pub duration_ticks: u32,
    pub fact_hash: String,
}

impl From<AnimationTransitionFactRef> for NativeAnimationTransitionFactRef {
    fn from(value: AnimationTransitionFactRef) -> Self {
        Self {
            fact_id: value.fact_id,
            source_fact_id: value.source_fact_id,
            authority_tick: value.authority_tick as i64,
            controller_input_sequence: value.controller_input_sequence as i64,
            controller_tick: value.controller_tick as i64,
            causation_id: value.causation_id,
            correlation_id: value.correlation_id,
            transition_id: value.transition_id,
            from_state_id: value.from_state_id,
            to_state_id: value.to_state_id,
            moment: match value.moment {
                AnimationTransitionFactMoment::Started => "started",
                AnimationTransitionFactMoment::Completed => "completed",
            }
            .to_string(),
            duration_ticks: value.duration_ticks,
            fact_hash: value.fact_hash,
        }
    }
}

#[napi(object)]
pub struct NativeAnimationControllerProjectionState {
    pub graph_id: String,
    pub graph_version: u32,
    pub graph_hash: String,
    pub state_id: String,
    pub revision: i64,
    pub state_hash: String,
    pub motion: NativeAnimationResolvedMotion,
    pub transition: Option<NativeAnimationTransitionProjection>,
    pub timing_fact: Option<NativeAnimationTransitionFactRef>,
}

impl From<AnimationControllerProjectionState> for NativeAnimationControllerProjectionState {
    fn from(value: AnimationControllerProjectionState) -> Self {
        Self {
            graph_id: value.graph_id,
            graph_version: value.graph_version,
            graph_hash: value.graph_hash,
            state_id: value.state_id,
            revision: value.revision as i64,
            state_hash: value.state_hash,
            motion: value.motion.into(),
            transition: value.transition.map(Into::into),
            timing_fact: value.timing_fact.map(|fact| (*fact).into()),
        }
    }
}

#[napi(object)]
pub struct NativeAnimationProjectionDescriptor {
    pub target: i64,
    pub asset: String,
    pub tick_duration_millis: u32,
    pub controller: NativeAnimationControllerProjectionState,
}

impl From<AnimationProjectionDescriptor> for NativeAnimationProjectionDescriptor {
    fn from(value: AnimationProjectionDescriptor) -> Self {
        Self {
            target: value.target.raw() as i64,
            asset: value.asset,
            tick_duration_millis: value.tick_duration_millis,
            controller: value.controller.into(),
        }
    }
}

#[napi(object)]
pub struct NativeAnimationProjectionOp {
    pub op: String,
    pub handle: i64,
    pub descriptor: Option<NativeAnimationProjectionDescriptor>,
    pub controller: Option<NativeAnimationControllerProjectionState>,
}

impl From<AnimationProjectionOp> for NativeAnimationProjectionOp {
    fn from(value: AnimationProjectionOp) -> Self {
        match value {
            AnimationProjectionOp::Create { handle, descriptor } => Self {
                op: "create".to_string(),
                handle: handle.raw() as i64,
                descriptor: Some(descriptor.into()),
                controller: None,
            },
            AnimationProjectionOp::Update { handle, controller } => Self {
                op: "update".to_string(),
                handle: handle.raw() as i64,
                descriptor: None,
                controller: Some(controller.into()),
            },
            AnimationProjectionOp::Destroy { handle } => Self {
                op: "destroy".to_string(),
                handle: handle.raw() as i64,
                descriptor: None,
                controller: None,
            },
        }
    }
}
