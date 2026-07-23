//! Kinematic movement tests (#2390): capability-based eligibility, deterministic
//! blocked/slid/moved outcomes over collision queries, render/collision
//! independence, replay stability, and fail-closed ineligible cases.

use core_entity::command::EntityLifecycleCommand as Cmd;
use core_entity::core::EntitySource;
use core_entity::movement::{MovementCommand, MovementError, MovementOutcome};
use core_entity::store::EntityStore;
use core_entity::{fixtures, Aabb, EntityTransform};
use core_ids::EntityId;
use core_math::Vec3;

fn e(id: u64) -> EntityId {
    EntityId::new(id)
}

fn movable(store: &mut EntityStore, id: u64, at: Vec3) -> EntityId {
    let entity = e(id);
    store
        .apply(Cmd::Create {
            id: entity,
            source: EntitySource::RuntimeCreated { by: None },
            labels: vec![],
        })
        .unwrap();
    store.attach_transform(entity, EntityTransform::at(at));
    store.attach_bounds(entity, Aabb::new(Vec3::splat(-0.5), Vec3::splat(0.5)));
    store.attach_collision(entity, false);
    entity
}

#[test]
fn movable_entity_moves_through_empty_space() {
    let mut store = EntityStore::new();
    let m = movable(&mut store, 1, Vec3::ZERO);
    let ev = store
        .apply_movement(MovementCommand {
            id: m,
            delta: Vec3::new(3.0, 0.0, 0.0),
        })
        .unwrap();
    assert_eq!(
        ev.outcome,
        MovementOutcome::Moved {
            to: Vec3::new(3.0, 0.0, 0.0)
        }
    );
    assert_eq!(
        store.transform(m).unwrap().transform.translation,
        Vec3::new(3.0, 0.0, 0.0)
    );
    assert_eq!(ev.hit, None);
}

#[test]
fn movement_into_a_collider_blocks_deterministically() {
    // Family 7: the obstacle at (1,0,0) blocks a +X mover from (0,0,0) (unit boxes).
    let mut store = fixtures::movement_family();
    let ev = store
        .apply_movement(MovementCommand {
            id: e(1),
            delta: Vec3::new(1.0, 0.0, 0.0),
        })
        .unwrap();
    assert_eq!(ev.outcome, MovementOutcome::Blocked { at: Vec3::ZERO });
    assert_eq!(ev.hit, Some(e(2)));
    // No mutation on a fully-blocked move.
    assert_eq!(
        store.transform(e(1)).unwrap().transform.translation,
        Vec3::ZERO
    );
}

#[test]
fn movement_slides_along_a_blocked_axis() {
    let mut store = EntityStore::new();
    let m = movable(&mut store, 1, Vec3::ZERO);
    // A static obstacle blocking +X but not +Y.
    let obstacle = e(2);
    store
        .apply(Cmd::Create {
            id: obstacle,
            source: EntitySource::RuntimeCreated { by: None },
            labels: vec![],
        })
        .unwrap();
    store.attach_transform(obstacle, EntityTransform::at(Vec3::new(1.0, 0.0, 0.0)));
    store.attach_bounds(obstacle, Aabb::new(Vec3::splat(-0.5), Vec3::splat(0.5)));
    store.attach_collision(obstacle, true);

    let ev = store
        .apply_movement(MovementCommand {
            id: m,
            delta: Vec3::new(1.0, 1.0, 0.0),
        })
        .unwrap();
    // X blocked, Y allowed → slide to (0,1,0).
    assert_eq!(
        ev.outcome,
        MovementOutcome::Slid {
            to: Vec3::new(0.0, 1.0, 0.0),
            blocked: [true, false, false]
        }
    );
    assert_eq!(ev.hit, Some(obstacle));
}

#[test]
fn non_static_collider_blocks_other_movers_without_becoming_immovable() {
    let mut store = EntityStore::new();
    let mover = movable(&mut store, 1, Vec3::ZERO);
    let actor = movable(&mut store, 2, Vec3::new(1.0, 0.0, 0.0));

    let event = store
        .apply_movement(MovementCommand {
            id: mover,
            delta: Vec3::new(1.0, 0.0, 0.0),
        })
        .unwrap();

    assert_eq!(event.outcome, MovementOutcome::Blocked { at: Vec3::ZERO });
    assert_eq!(event.hit, Some(actor));
    assert_eq!(
        store.transform(actor).unwrap().transform.translation,
        Vec3::new(1.0, 0.0, 0.0)
    );
}

#[test]
fn non_spatial_entity_rejects_movement() {
    let mut store = fixtures::movement_family();
    assert_eq!(
        store.apply_movement(MovementCommand {
            id: e(3),
            delta: Vec3::ONE
        }),
        Err(MovementError::NotSpatial { id: e(3) })
    );
}

#[test]
fn immovable_static_entity_rejects_movement() {
    let mut store = fixtures::movement_family();
    assert_eq!(
        store.apply_movement(MovementCommand {
            id: e(4),
            delta: Vec3::ONE
        }),
        Err(MovementError::Immovable { id: e(4) })
    );
}

#[test]
fn rendered_but_non_colliding_entity_rejects_movement() {
    // Render presence must NOT make an entity movement-eligible (no collider).
    let mut store = fixtures::movement_family();
    assert_eq!(
        store.apply_movement(MovementCommand {
            id: e(5),
            delta: Vec3::ONE
        }),
        Err(MovementError::NoCollider { id: e(5) })
    );
}

#[test]
fn collider_without_render_is_movement_eligible() {
    // Collision presence (not render) is what makes movement eligible.
    let mut store = fixtures::movement_family();
    assert!(store.movement_eligible(e(6)).is_ok());
    let ev = store
        .apply_movement(MovementCommand {
            id: e(6),
            delta: Vec3::new(0.0, 0.0, 1.0),
        })
        .unwrap();
    assert!(
        !ev.projection_changed,
        "no render capability ⇒ no projection update"
    );
    assert!(matches!(ev.outcome, MovementOutcome::Moved { .. }));
}

#[test]
fn rendered_movable_signals_projection_update() {
    let mut store = EntityStore::new();
    let m = movable(&mut store, 1, Vec3::ZERO);
    store.attach_render_projection(m, true);
    let ev = store
        .apply_movement(MovementCommand {
            id: m,
            delta: Vec3::new(1.0, 0.0, 0.0),
        })
        .unwrap();
    assert!(ev.projection_changed);
}

#[test]
fn disabled_tombstoned_and_unknown_reject_movement() {
    let mut store = EntityStore::new();
    let a = movable(&mut store, 1, Vec3::ZERO);
    store.apply(Cmd::Disable { id: a }).unwrap();
    assert_eq!(
        store.apply_movement(MovementCommand {
            id: a,
            delta: Vec3::ONE
        }),
        Err(MovementError::Disabled { id: a })
    );
    store.apply(Cmd::Enable { id: a }).unwrap();
    store.apply(Cmd::Destroy { id: a }).unwrap();
    assert_eq!(
        store.apply_movement(MovementCommand {
            id: a,
            delta: Vec3::ONE
        }),
        Err(MovementError::Tombstoned { id: a })
    );
    assert_eq!(
        store.apply_movement(MovementCommand {
            id: e(9),
            delta: Vec3::ONE
        }),
        Err(MovementError::UnknownEntity { id: e(9) })
    );
}

#[test]
fn non_finite_delta_is_rejected() {
    let mut store = EntityStore::new();
    let m = movable(&mut store, 1, Vec3::ZERO);
    assert_eq!(
        store.apply_movement(MovementCommand {
            id: m,
            delta: Vec3::new(f32::INFINITY, 0.0, 0.0)
        }),
        Err(MovementError::NonFinite { id: m })
    );
    assert_eq!(
        store.transform(m).unwrap().transform.translation,
        Vec3::ZERO
    );
}

#[test]
fn movement_is_replayable_and_hash_stable() {
    let commands = [
        MovementCommand {
            id: e(1),
            delta: Vec3::new(0.4, 0.0, 0.0),
        }, // approaches obstacle, no overlap yet
        MovementCommand {
            id: e(6),
            delta: Vec3::new(0.0, 1.0, 0.0),
        },
    ];
    let replay = || {
        let mut store = fixtures::movement_family();
        for c in commands {
            let _ = store.apply_movement(c);
        }
        store
    };
    assert_eq!(replay().hash(), replay().hash());
}
