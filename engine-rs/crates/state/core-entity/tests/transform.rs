//! Runtime transform tests (#2388): transform-eligible vs ineligible families,
//! replay/hash stability, projection-update signalling, and fail-closed negatives.

use core_entity::command::EntityLifecycleCommand as Cmd;
use core_entity::core::EntitySource;
use core_entity::store::EntityStore;
use core_entity::transform::{TransformCommand, TransformError};
use core_entity::{fixtures, EntityTransform};
use core_ids::EntityId;
use core_math::Vec3;

fn e(id: u64) -> EntityId {
    EntityId::new(id)
}

fn runtime_entity(store: &mut EntityStore, id: u64) -> EntityId {
    let entity = e(id);
    store
        .apply(Cmd::Create {
            id: entity,
            source: EntitySource::RuntimeCreated { by: None },
            labels: vec![],
        })
        .unwrap();
    entity
}

#[test]
fn transform_eligible_spatial_entity_can_be_set_and_translated() {
    // Family 1: a spatial rendered entity (id 1 has transform + visible render).
    let mut store = fixtures::spatial_rendered_family();
    let ev = store
        .apply_transform(TransformCommand::Set {
            id: e(1),
            transform: EntityTransform::at(Vec3::new(9.0, 0.0, 0.0)),
        })
        .unwrap();
    assert_eq!(ev.transform.translation.x, 9.0);
    assert!(
        ev.projection_changed,
        "a visible rendered entity signals a projection update"
    );

    let ev2 = store
        .apply_transform(TransformCommand::Translate {
            id: e(1),
            delta: Vec3::new(1.0, 2.0, 0.0),
        })
        .unwrap();
    assert_eq!(ev2.transform.translation, Vec3::new(10.0, 2.0, 0.0));
    assert_eq!(store.transform(e(1)).unwrap().transform.translation.x, 10.0);
}

#[test]
fn non_rendered_spatial_entity_does_not_signal_projection() {
    // Family 2: navigation_anchor_entity (id 2) has a transform but no render.
    let mut store = fixtures::spatial_non_rendered_family();
    let ev = store
        .apply_transform(TransformCommand::Translate {
            id: e(2),
            delta: Vec3::new(0.0, 0.0, 5.0),
        })
        .unwrap();
    assert!(
        !ev.projection_changed,
        "no render capability ⇒ no projection update"
    );
}

#[test]
fn non_spatial_logical_entity_rejects_transform() {
    // Family 3: logical entities have no transform capability.
    let mut store = fixtures::non_spatial_logical_family();
    let before = store.hash();
    assert_eq!(
        store.apply_transform(TransformCommand::Set {
            id: e(1),
            transform: EntityTransform::IDENTITY,
        }),
        Err(TransformError::NotTransformEligible { id: e(1) })
    );
    assert_eq!(
        store.hash(),
        before,
        "a rejected transform must not mutate state"
    );
}

#[test]
fn contained_only_entity_rejects_transform() {
    // Family 4: a contained record has no transform.
    let mut store = fixtures::contained_family();
    assert_eq!(
        store.apply_transform(TransformCommand::Translate {
            id: e(2),
            delta: Vec3::ONE,
        }),
        Err(TransformError::NotTransformEligible { id: e(2) })
    );
}

#[test]
fn tombstoned_and_disabled_entities_reject_transform() {
    let mut store = EntityStore::new();
    let a = runtime_entity(&mut store, 1);
    store.attach_transform(a, EntityTransform::IDENTITY);
    store.apply(Cmd::Disable { id: a }).unwrap();
    assert_eq!(
        store.apply_transform(TransformCommand::Set {
            id: a,
            transform: EntityTransform::IDENTITY
        }),
        Err(TransformError::Disabled { id: a })
    );

    let b = runtime_entity(&mut store, 2);
    store.attach_transform(b, EntityTransform::IDENTITY);
    store.apply(Cmd::Destroy { id: b }).unwrap();
    assert_eq!(
        store.apply_transform(TransformCommand::Set {
            id: b,
            transform: EntityTransform::IDENTITY
        }),
        Err(TransformError::Tombstoned { id: b })
    );
}

#[test]
fn immovable_static_entity_rejects_transform() {
    let mut store = EntityStore::new();
    let s = runtime_entity(&mut store, 1);
    store.attach_transform(s, EntityTransform::IDENTITY);
    store.attach_collision(s, true); // static collider ⇒ immovable
    assert_eq!(
        store.apply_transform(TransformCommand::Translate {
            id: s,
            delta: Vec3::ONE
        }),
        Err(TransformError::Immovable { id: s })
    );
}

#[test]
fn unknown_entity_rejects_transform() {
    let mut store = EntityStore::new();
    assert_eq!(
        store.apply_transform(TransformCommand::Set {
            id: e(7),
            transform: EntityTransform::IDENTITY
        }),
        Err(TransformError::UnknownEntity { id: e(7) })
    );
}

#[test]
fn non_finite_transform_is_rejected() {
    let mut store = EntityStore::new();
    let a = runtime_entity(&mut store, 1);
    store.attach_transform(a, EntityTransform::IDENTITY);
    let nan = EntityTransform {
        translation: Vec3::new(f32::NAN, 0.0, 0.0),
        ..EntityTransform::IDENTITY
    };
    assert_eq!(
        store.apply_transform(TransformCommand::Set {
            id: a,
            transform: nan
        }),
        Err(TransformError::NonFinite { id: a })
    );
    // The prior (identity) transform is untouched.
    assert_eq!(
        store.transform(a).unwrap().transform.translation,
        Vec3::ZERO
    );
}

#[test]
fn transform_updates_are_replayable_and_hash_stable() {
    let commands = [
        TransformCommand::Set {
            id: e(1),
            transform: EntityTransform::at(Vec3::new(2.0, 2.0, 2.0)),
        },
        TransformCommand::Translate {
            id: e(1),
            delta: Vec3::new(1.0, 0.0, 0.0),
        },
        TransformCommand::Set {
            id: e(3),
            transform: EntityTransform::at(Vec3::new(7.0, 0.0, 0.0)),
        },
    ];
    let replay = || {
        let mut store = fixtures::spatial_rendered_family();
        for c in commands {
            store.apply_transform(c).unwrap();
        }
        store
    };
    let a = replay();
    let b = replay();
    assert_eq!(a.hash(), b.hash(), "transform replay must be hash-stable");

    // Save→reload preserves the runtime transforms.
    let restored = EntityStore::from_snapshot(a.snapshot());
    assert_eq!(restored.hash(), a.hash());
    assert_eq!(
        restored.transform(e(1)).unwrap().transform.translation,
        Vec3::new(3.0, 2.0, 2.0)
    );
}

#[test]
fn save_reload_does_not_invent_transforms_for_non_spatial_entities() {
    let store = fixtures::non_spatial_logical_family();
    let restored = EntityStore::from_snapshot(store.snapshot());
    for id in [1u64, 2, 3] {
        assert!(
            restored.transform(e(id)).is_none(),
            "no phantom transform after reload"
        );
    }
}
