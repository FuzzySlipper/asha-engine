use std::collections::BTreeMap;

use core_ids::EntityId;
use protocol_presentation::{
    AnimationProjectionOp, PresentationOp, PresentationOpMeta, PresentationOriginKind,
    PresentationOriginRef,
};
use protocol_render::RenderHandle;
use render_animation::AnimationControllerProjector;
use rule_animation_controller::{
    AnimationControllerChange, AnimationControllerState, AnimationInputOrigin,
    AnimationParameterValue, AnimationTransitionFactMoment, AnimationTransitionTimingFact,
    ResolvedAnimationMotion,
};

fn change(entity: u64, revision: u64) -> AnimationControllerChange {
    AnimationControllerChange {
        previous_state_hash: None,
        state: AnimationControllerState {
            entity,
            graph_id: "player".into(),
            graph_version: 1,
            graph_hash: "fnv1a64:graph".into(),
            current_state_id: "idle".into(),
            revision,
            parameters: BTreeMap::from([("speed".into(), AnimationParameterValue::Float(0))]),
            motion: ResolvedAnimationMotion {
                clip_a: "idle".into(),
                clip_b: None,
                blend_weight_milli: 0,
                speed_milli: 1_000,
            },
            transition: None,
            timing_fact: None,
            state_hash: format!("fnv1a64:state-{revision}"),
        },
    }
}

fn fact_change(entity: u64) -> AnimationControllerChange {
    let mut value = change(entity, 1);
    value.state.timing_fact = Some(AnimationTransitionTimingFact {
        fact_id: "combat.primary-fire.accepted:9:animation:7:ready.primary_fire:started".into(),
        source: AnimationInputOrigin {
            source_fact_id: "combat.primary-fire.accepted:9".into(),
            authority_tick: 9,
            causation_id: "combat.primary-fire:9".into(),
            correlation_id: "fps.session:1".into(),
        },
        controller_input_sequence: 3,
        controller_tick: 1,
        entity,
        graph_id: "player".into(),
        transition_id: "ready.primary_fire".into(),
        from_state_id: "ready".into(),
        to_state_id: "primary_fire".into(),
        moment: AnimationTransitionFactMoment::Started,
        duration_ticks: 4,
        resulting_revision: 1,
        fact_hash: "fnv1a64:fact".into(),
    });
    value
}

fn meta(sequence: u32) -> PresentationOpMeta {
    PresentationOpMeta {
        sequence,
        origin: Some(PresentationOriginRef {
            kind: PresentationOriginKind::CapabilityState,
            id: format!("animation:player:{sequence}"),
            authority_tick: u64::from(sequence),
            causation_id: Some("input:move".into()),
            correlation_id: Some("actor:player".into()),
        }),
    }
}

#[test]
fn authority_changes_project_to_one_stable_g1_lifecycle() {
    let entity = EntityId::new(7);
    let mut projector = AnimationControllerProjector::new();
    let create = projector
        .create(
            entity,
            RenderHandle::new(99),
            "mesh-animation/character",
            50,
            &change(7, 0),
            meta(0),
        )
        .expect("create");
    let PresentationOp::Animation {
        meta: create_meta,
        op,
    } = create
    else {
        panic!("animation domain");
    };
    assert_eq!(
        create_meta
            .origin
            .expect("origin")
            .correlation_id
            .as_deref(),
        Some("actor:player")
    );
    let AnimationProjectionOp::Create { handle, descriptor } = op else {
        panic!("create op");
    };
    assert_eq!(handle.raw(), 1);
    assert_eq!(descriptor.target.raw(), 99);
    assert_eq!(descriptor.controller.graph_hash, "fnv1a64:graph");

    let update = projector
        .update(entity, &change(7, 1), meta(1))
        .expect("update");
    let PresentationOp::Animation {
        op:
            AnimationProjectionOp::Update {
                handle: updated,
                controller,
            },
        ..
    } = update
    else {
        panic!("update op");
    };
    assert_eq!(updated, handle);
    assert_eq!(controller.revision, 1);

    let destroy = projector.destroy(entity, meta(2)).expect("destroy");
    let PresentationOp::Animation {
        op: AnimationProjectionOp::Destroy { handle: destroyed },
        ..
    } = destroy
    else {
        panic!("destroy op");
    };
    assert_eq!(destroyed, handle);
    assert!(projector.handle(entity).is_none());
}

#[test]
fn projector_rejects_mismatched_authority_identity() {
    let mut projector = AnimationControllerProjector::new();
    assert!(projector
        .create(
            EntityId::new(7),
            RenderHandle::new(99),
            "mesh-animation/character",
            50,
            &change(8, 0),
            meta(0),
        )
        .is_err());
}

#[test]
fn projector_rejects_trace_metadata_that_does_not_match_the_authority_fact() {
    let mut projector = AnimationControllerProjector::new();
    assert_eq!(
        projector
            .create(
                EntityId::new(7),
                RenderHandle::new(99),
                "mesh-animation/character",
                50,
                &fact_change(7),
                meta(9),
            )
            .expect_err("fabricated projection origin must fail"),
        render_animation::AnimationProjectionError::OriginMismatch
    );
}
