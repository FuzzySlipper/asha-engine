use core_entity::fixtures;
use core_entity::{FirstPersonMotionCommand, FirstPersonMotionInput, MovementOutcome};
use core_ids::EntityId;
use core_math::Vec3;

fn e(id: u64) -> EntityId {
    EntityId::new(id)
}

fn forward(speed: f32) -> FirstPersonMotionInput {
    FirstPersonMotionInput {
        move_forward: 1.0,
        move_right: 0.0,
        move_up: 0.0,
        yaw_delta_degrees: 0.0,
        pitch_delta_degrees: 0.0,
        dt_seconds: 1.0,
        move_speed_units_per_second: speed,
    }
}

#[test]
fn first_person_motion_in_static_room_stops_at_wall() {
    let mut store = fixtures::static_room_collision_family();
    let event = store
        .apply_first_person_motion_with_collision(FirstPersonMotionCommand {
            id: e(1),
            input: forward(3.0),
            tick: 1,
        })
        .unwrap();

    assert_eq!(event.attempted.position, Vec3::new(0.0, 0.0, -3.0));
    assert_eq!(event.to.position, Vec3::ZERO);
    assert_eq!(
        event.movement.outcome,
        MovementOutcome::Blocked { at: Vec3::ZERO }
    );
    assert_eq!(event.collision.hit, Some(e(2)));
    assert_eq!(
        store.transform(e(1)).unwrap().transform.translation,
        Vec3::ZERO
    );
}

#[test]
fn first_person_motion_in_static_room_moves_through_empty_space() {
    let mut store = fixtures::static_room_collision_family();
    let event = store
        .apply_first_person_motion_with_collision(FirstPersonMotionCommand {
            id: e(1),
            input: FirstPersonMotionInput {
                move_forward: 0.0,
                move_right: 1.0,
                ..forward(1.0)
            },
            tick: 2,
        })
        .unwrap();

    assert_eq!(event.to.position, Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(
        event.movement.outcome,
        MovementOutcome::Moved {
            to: Vec3::new(1.0, 0.0, 0.0)
        }
    );
    assert_eq!(event.collision.hit, None);
    assert!(event.collision.projection_changed);
}

#[test]
fn static_room_collision_replay_hash_is_stable() {
    let replay = || {
        let mut store = fixtures::static_room_collision_family();
        let _ = store
            .apply_first_person_motion_with_collision(FirstPersonMotionCommand {
                id: e(1),
                input: forward(3.0),
                tick: 1,
            })
            .unwrap();
        let _ = store
            .apply_first_person_motion_with_collision(FirstPersonMotionCommand {
                id: e(1),
                input: FirstPersonMotionInput {
                    move_forward: 0.0,
                    move_right: 1.0,
                    ..forward(1.0)
                },
                tick: 2,
            })
            .unwrap();
        store
    };

    assert_eq!(replay().hash(), replay().hash());
}
