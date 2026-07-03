use core_entity::command::EntityLifecycleCommand as Cmd;
use core_entity::core::EntitySource;
use core_entity::{
    EntityStore, EntityTransform, FirstPersonMotionCommand, FirstPersonMotionError,
    FirstPersonMotionInput,
};
use core_ids::EntityId;
use core_math::Vec3;

fn e(id: u64) -> EntityId {
    EntityId::new(id)
}

fn spatial_actor(store: &mut EntityStore) -> EntityId {
    let id = e(1);
    store
        .apply(Cmd::Create {
            id,
            source: EntitySource::RuntimeCreated { by: None },
            labels: vec![],
        })
        .unwrap();
    store.attach_transform(id, EntityTransform::at(Vec3::new(0.0, 1.6, 0.0)));
    store.attach_render_projection(id, true);
    id
}

fn input() -> FirstPersonMotionInput {
    FirstPersonMotionInput {
        move_forward: 1.0,
        move_right: 1.0,
        move_up: 0.0,
        yaw_delta_degrees: 15.0,
        pitch_delta_degrees: -5.0,
        dt_seconds: 1.0,
        move_speed_units_per_second: 3.0,
    }
}

#[test]
fn first_person_input_updates_authority_transform_and_projection_readout() {
    let mut store = EntityStore::new();
    let id = spatial_actor(&mut store);

    let event = store
        .apply_first_person_motion(FirstPersonMotionCommand {
            id,
            input: input(),
            tick: 10,
        })
        .unwrap();

    assert_eq!(event.from.position, Vec3::new(0.0, 1.6, 0.0));
    assert_eq!(event.to.position, Vec3::new(3.0, 1.6, -3.0));
    assert_eq!(event.to.yaw_degrees, 15.0);
    assert_eq!(event.to.pitch_degrees, -5.0);
    assert!(event.transform.projection_changed);
    assert_eq!(
        store.transform(id).unwrap().transform.translation,
        Vec3::new(3.0, 1.6, -3.0)
    );
    assert_eq!(event.readout.tick, 10);
    assert_eq!(event.readout.pose, event.to);
    assert!(event.readout.pose_hash != 0);
    assert!(approx(event.readout.basis.forward.x, 0.25783416));
    assert!(approx(event.readout.basis.forward.y, -0.08715574));
    assert!(approx(event.readout.basis.forward.z, -0.9622502));
}

#[test]
fn repeated_first_person_motion_reads_prior_authority_pose() {
    let mut store = EntityStore::new();
    let id = spatial_actor(&mut store);

    store
        .apply_first_person_motion(FirstPersonMotionCommand {
            id,
            input: FirstPersonMotionInput {
                move_forward: 0.0,
                move_right: 0.0,
                move_up: 0.0,
                yaw_delta_degrees: 90.0,
                pitch_delta_degrees: 0.0,
                dt_seconds: 0.0,
                move_speed_units_per_second: 3.0,
            },
            tick: 1,
        })
        .unwrap();
    let second = store
        .apply_first_person_motion(FirstPersonMotionCommand {
            id,
            input: FirstPersonMotionInput {
                move_forward: 1.0,
                move_right: 0.0,
                move_up: 0.0,
                yaw_delta_degrees: 0.0,
                pitch_delta_degrees: 0.0,
                dt_seconds: 1.0,
                move_speed_units_per_second: 2.0,
            },
            tick: 2,
        })
        .unwrap();

    assert!(approx(second.from.yaw_degrees, 90.0));
    assert!(approx(second.to.position.x, 2.0));
    assert!(approx(second.to.position.z, 0.0));
}

#[test]
fn malformed_first_person_input_rejects_without_mutation() {
    let mut store = EntityStore::new();
    let id = spatial_actor(&mut store);
    let before = store.hash();

    let result = store.apply_first_person_motion(FirstPersonMotionCommand {
        id,
        input: FirstPersonMotionInput {
            yaw_delta_degrees: f32::NAN,
            ..input()
        },
        tick: 1,
    });

    assert_eq!(result, Err(FirstPersonMotionError::NonFinite { id }));
    assert_eq!(store.hash(), before);
}

#[test]
fn non_spatial_entity_rejects_first_person_motion() {
    let mut store = EntityStore::new();
    let id = e(1);
    store
        .apply(Cmd::Create {
            id,
            source: EntitySource::RuntimeCreated { by: None },
            labels: vec![],
        })
        .unwrap();

    let result = store.apply_first_person_motion(FirstPersonMotionCommand {
        id,
        input: input(),
        tick: 1,
    });

    assert_eq!(
        result,
        Err(FirstPersonMotionError::Transform(
            core_entity::TransformError::NotTransformEligible { id }
        ))
    );
}

#[test]
fn unknown_entity_rejects_first_person_motion_as_unknown() {
    let mut store = EntityStore::new();
    let id = e(99);
    let result = store.apply_first_person_motion(FirstPersonMotionCommand {
        id,
        input: input(),
        tick: 1,
    });

    assert_eq!(
        result,
        Err(FirstPersonMotionError::Transform(
            core_entity::TransformError::UnknownEntity { id }
        ))
    );
}

fn approx(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() < 0.0001
}
