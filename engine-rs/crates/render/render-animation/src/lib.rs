//! One-way projection from animation controller authority into the G1 domain.
//!
//! The projector owns presentation handle lifecycle only. It copies resolved
//! controller state and trace metadata; it never samples clips or feeds host
//! state back into authority.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use core_ids::EntityId;
use protocol_presentation::{
    AnimationControllerProjectionState, AnimationProjectionDescriptor, AnimationProjectionHandle,
    AnimationProjectionOp, AnimationResolvedMotion, AnimationTransitionFactMoment,
    AnimationTransitionFactRef, AnimationTransitionProjection, PresentationOp, PresentationOpMeta,
};
use protocol_render::RenderHandle;
use rule_animation_controller::{
    AnimationControllerChange, AnimationControllerState,
    AnimationTransitionFactMoment as AuthorityFactMoment, ResolvedAnimationMotion,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationProjectionError {
    EntityMismatch { expected: u64, actual: u64 },
    ControllerAlreadyProjected(u64),
    ControllerNotProjected(u64),
    HandleExhausted,
    InvalidDescriptor(&'static str),
    OriginMismatch,
}

impl core::fmt::Display for AnimationProjectionError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AnimationProjectionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectionBinding {
    handle: AnimationProjectionHandle,
}

#[derive(Debug, Clone)]
pub struct AnimationControllerProjector {
    next_handle: u64,
    bindings: BTreeMap<EntityId, ProjectionBinding>,
}

impl Default for AnimationControllerProjector {
    fn default() -> Self {
        Self {
            next_handle: 1,
            bindings: BTreeMap::new(),
        }
    }
}

impl AnimationControllerProjector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(
        &mut self,
        entity: EntityId,
        target: RenderHandle,
        asset: impl Into<String>,
        tick_duration_millis: u32,
        change: &AnimationControllerChange,
        meta: PresentationOpMeta,
    ) -> Result<PresentationOp, AnimationProjectionError> {
        verify_entity(entity, &change.state)?;
        verify_origin(change, &meta)?;
        if self.bindings.contains_key(&entity) {
            return Err(AnimationProjectionError::ControllerAlreadyProjected(
                entity.raw(),
            ));
        }
        let asset = asset.into();
        if asset.is_empty() {
            return Err(AnimationProjectionError::InvalidDescriptor(
                "animation asset id is empty",
            ));
        }
        if tick_duration_millis == 0 {
            return Err(AnimationProjectionError::InvalidDescriptor(
                "tick duration must be non-zero",
            ));
        }
        let handle = AnimationProjectionHandle::new(self.next_handle);
        self.next_handle = self
            .next_handle
            .checked_add(1)
            .ok_or(AnimationProjectionError::HandleExhausted)?;
        self.bindings.insert(entity, ProjectionBinding { handle });
        Ok(PresentationOp::Animation {
            meta,
            op: AnimationProjectionOp::Create {
                handle,
                descriptor: AnimationProjectionDescriptor {
                    target,
                    asset,
                    tick_duration_millis,
                    controller: project_state(&change.state),
                },
            },
        })
    }

    pub fn update(
        &self,
        entity: EntityId,
        change: &AnimationControllerChange,
        meta: PresentationOpMeta,
    ) -> Result<PresentationOp, AnimationProjectionError> {
        verify_entity(entity, &change.state)?;
        verify_origin(change, &meta)?;
        let binding =
            self.bindings
                .get(&entity)
                .ok_or(AnimationProjectionError::ControllerNotProjected(
                    entity.raw(),
                ))?;
        Ok(PresentationOp::Animation {
            meta,
            op: AnimationProjectionOp::Update {
                handle: binding.handle,
                controller: project_state(&change.state),
            },
        })
    }

    pub fn destroy(
        &mut self,
        entity: EntityId,
        meta: PresentationOpMeta,
    ) -> Result<PresentationOp, AnimationProjectionError> {
        let binding = self.bindings.remove(&entity).ok_or(
            AnimationProjectionError::ControllerNotProjected(entity.raw()),
        )?;
        Ok(PresentationOp::Animation {
            meta,
            op: AnimationProjectionOp::Destroy {
                handle: binding.handle,
            },
        })
    }

    pub fn handle(&self, entity: EntityId) -> Option<AnimationProjectionHandle> {
        self.bindings.get(&entity).map(|binding| binding.handle)
    }
}

fn verify_entity(
    entity: EntityId,
    state: &AnimationControllerState,
) -> Result<(), AnimationProjectionError> {
    if entity.raw() != state.entity {
        return Err(AnimationProjectionError::EntityMismatch {
            expected: entity.raw(),
            actual: state.entity,
        });
    }
    Ok(())
}

fn verify_origin(
    change: &AnimationControllerChange,
    meta: &PresentationOpMeta,
) -> Result<(), AnimationProjectionError> {
    let Some(fact) = &change.state.timing_fact else {
        return Ok(());
    };
    let Some(origin) = &meta.origin else {
        return Err(AnimationProjectionError::OriginMismatch);
    };
    if origin.id != fact.source.source_fact_id
        || origin.authority_tick != fact.source.authority_tick
        || origin.causation_id.as_deref() != Some(fact.source.causation_id.as_str())
        || origin.correlation_id.as_deref() != Some(fact.source.correlation_id.as_str())
    {
        return Err(AnimationProjectionError::OriginMismatch);
    }
    Ok(())
}

fn project_state(state: &AnimationControllerState) -> AnimationControllerProjectionState {
    AnimationControllerProjectionState {
        graph_id: state.graph_id.clone(),
        graph_version: state.graph_version,
        graph_hash: state.graph_hash.clone(),
        state_id: state.current_state_id.clone(),
        revision: state.revision,
        state_hash: state.state_hash.clone(),
        motion: project_motion(&state.motion),
        transition: state
            .transition
            .as_ref()
            .map(|transition| AnimationTransitionProjection {
                transition_id: transition.transition_id.clone(),
                from_state_id: transition.from_state_id.clone(),
                to_state_id: transition.to_state_id.clone(),
                elapsed_ticks: transition.elapsed_ticks,
                duration_ticks: transition.duration_ticks,
                target_motion: project_motion(&transition.target_motion),
            }),
        timing_fact: state.timing_fact.as_ref().map(|fact| {
            Box::new(AnimationTransitionFactRef {
                fact_id: fact.fact_id.clone(),
                source_fact_id: fact.source.source_fact_id.clone(),
                authority_tick: fact.source.authority_tick,
                controller_input_sequence: fact.controller_input_sequence,
                controller_tick: fact.controller_tick,
                causation_id: fact.source.causation_id.clone(),
                correlation_id: fact.source.correlation_id.clone(),
                transition_id: fact.transition_id.clone(),
                from_state_id: fact.from_state_id.clone(),
                to_state_id: fact.to_state_id.clone(),
                moment: match fact.moment {
                    AuthorityFactMoment::Started => AnimationTransitionFactMoment::Started,
                    AuthorityFactMoment::Completed => AnimationTransitionFactMoment::Completed,
                },
                duration_ticks: fact.duration_ticks,
                fact_hash: fact.fact_hash.clone(),
            })
        }),
    }
}

fn project_motion(motion: &ResolvedAnimationMotion) -> AnimationResolvedMotion {
    AnimationResolvedMotion {
        clip_a: motion.clip_a.clone(),
        clip_b: motion.clip_b.clone(),
        blend_weight_milli: motion.blend_weight_milli,
        speed_milli: motion.speed_milli,
    }
}
