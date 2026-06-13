//! Relation taxonomy tests (#2389): the five relation kinds behave differently;
//! transform attachment propagates and is cycle-checked; containment does not
//! propagate transforms; render grouping is refused as projection-only.

use core_entity::command::EntityLifecycleCommand as Cmd;
use core_entity::core::EntitySource;
use core_entity::relation::{RelationCommand, RelationError, RelationKind};
use core_entity::store::EntityStore;
use core_entity::{fixtures, EntityTransform};
use core_ids::EntityId;
use core_math::Vec3;

fn e(id: u64) -> EntityId {
    EntityId::new(id)
}

fn spatial(store: &mut EntityStore, id: u64, at: Vec3) -> EntityId {
    let entity = e(id);
    store
        .apply(Cmd::Create {
            id: entity,
            source: EntitySource::RuntimeCreated { by: None },
            labels: vec![],
        })
        .unwrap();
    store.attach_transform(entity, EntityTransform::at(at));
    entity
}

fn logical(store: &mut EntityStore, id: u64) -> EntityId {
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
fn transform_parent_propagates_child_world_transform() {
    let mut store = EntityStore::new();
    let parent = spatial(&mut store, 1, Vec3::new(10.0, 0.0, 0.0));
    let child = spatial(&mut store, 2, Vec3::new(1.0, 2.0, 0.0));
    store
        .apply_relation(RelationCommand::AttachTransformParent { child, parent })
        .unwrap();

    // Local transform unchanged; world transform composes the parent offset.
    assert_eq!(
        store.transform(child).unwrap().transform.translation,
        Vec3::new(1.0, 2.0, 0.0)
    );
    assert_eq!(
        store.world_transform(child).unwrap().translation,
        Vec3::new(11.0, 2.0, 0.0)
    );
    // Detach re-roots the child to world space.
    store
        .apply_relation(RelationCommand::DetachTransformParent { child })
        .unwrap();
    assert_eq!(
        store.world_transform(child).unwrap().translation,
        Vec3::new(1.0, 2.0, 0.0)
    );
}

#[test]
fn transform_attachment_cycles_are_rejected() {
    let mut store = EntityStore::new();
    let a = spatial(&mut store, 1, Vec3::ZERO);
    let b = spatial(&mut store, 2, Vec3::ZERO);
    let c = spatial(&mut store, 3, Vec3::ZERO);
    store
        .apply_relation(RelationCommand::AttachTransformParent {
            child: b,
            parent: a,
        })
        .unwrap();
    store
        .apply_relation(RelationCommand::AttachTransformParent {
            child: c,
            parent: b,
        })
        .unwrap();
    // a → c would close the cycle a→b→c→a.
    assert_eq!(
        store.apply_relation(RelationCommand::AttachTransformParent {
            child: a,
            parent: c
        }),
        Err(RelationError::Cycle {
            kind: RelationKind::TransformParent,
            at: a
        })
    );
    // Self-attachment is also rejected.
    assert_eq!(
        store.apply_relation(RelationCommand::AttachTransformParent {
            child: a,
            parent: a
        }),
        Err(RelationError::SelfRelation {
            kind: RelationKind::TransformParent,
            id: a
        })
    );
}

#[test]
fn transform_attachment_requires_both_ends_spatial() {
    let mut store = EntityStore::new();
    let spatial_child = spatial(&mut store, 1, Vec3::ZERO);
    let logical_parent = logical(&mut store, 2);
    assert_eq!(
        store.apply_relation(RelationCommand::AttachTransformParent {
            child: spatial_child,
            parent: logical_parent,
        }),
        Err(RelationError::NotTransformEligible { id: logical_parent })
    );
}

#[test]
fn containment_does_not_propagate_transform_and_is_not_attachment() {
    let mut store = EntityStore::new();
    let container = logical(&mut store, 1);
    let member = logical(&mut store, 2);
    store
        .apply_relation(RelationCommand::SetContainment { member, container })
        .unwrap();
    // Containment is recorded, but it is NOT a transform parent and creates no
    // transform (the member stays non-spatial).
    assert_eq!(store.containment(member).unwrap().container, container);
    assert!(
        store.transform_parent_of(member).is_none(),
        "containment is not transform attachment"
    );
    assert!(
        store.world_transform(member).is_none(),
        "no transform implied by containment"
    );
}

#[test]
fn containment_cycles_are_rejected() {
    let mut store = EntityStore::new();
    let a = logical(&mut store, 1);
    let b = logical(&mut store, 2);
    store
        .apply_relation(RelationCommand::SetContainment {
            member: b,
            container: a,
        })
        .unwrap();
    // a contained_in b would cycle.
    assert_eq!(
        store.apply_relation(RelationCommand::SetContainment {
            member: a,
            container: b
        }),
        Err(RelationError::Cycle {
            kind: RelationKind::Containment,
            at: a
        })
    );
}

#[test]
fn source_ancestry_is_read_only_and_allows_dangling_origin() {
    let mut store = EntityStore::new();
    let origin = logical(&mut store, 1);
    let derived = logical(&mut store, 2);
    store
        .apply_relation(RelationCommand::SetDerivedFrom { derived, origin })
        .unwrap();
    assert_eq!(store.derived_from(derived), Some(origin));
    // Destroying the origin leaves the ancestry trace dangling (by design).
    store.apply(Cmd::Destroy { id: origin }).unwrap();
    assert_eq!(
        store.derived_from(derived),
        Some(origin),
        "ancestry trace is retained"
    );
}

#[test]
fn render_grouping_is_refused_as_projection_only() {
    let mut store = EntityStore::new();
    let m = logical(&mut store, 1);
    assert_eq!(
        store.apply_relation(RelationCommand::SetRenderGroup { member: m }),
        Err(RelationError::ProjectionOnly {
            kind: RelationKind::RenderGrouping
        })
    );
}

#[test]
fn destroying_a_transform_parent_reroots_children() {
    let mut store = EntityStore::new();
    let parent = spatial(&mut store, 1, Vec3::new(10.0, 0.0, 0.0));
    let child = spatial(&mut store, 2, Vec3::new(1.0, 0.0, 0.0));
    store
        .apply_relation(RelationCommand::AttachTransformParent { child, parent })
        .unwrap();
    store.apply(Cmd::Destroy { id: parent }).unwrap();
    // The child is detached (re-rooted to world); no dangling parent pointer.
    assert!(store.transform_parent_of(child).is_none());
    assert_eq!(
        store.world_transform(child).unwrap().translation,
        Vec3::new(1.0, 0.0, 0.0)
    );
}

#[test]
fn destroying_a_container_orphans_members() {
    let mut store = EntityStore::new();
    let container = logical(&mut store, 1);
    let member = logical(&mut store, 2);
    store
        .apply_relation(RelationCommand::SetContainment { member, container })
        .unwrap();
    store.apply(Cmd::Destroy { id: container }).unwrap();
    assert!(
        store.containment(member).is_none(),
        "member orphaned on container destroy"
    );
}

#[test]
fn unknown_and_tombstoned_endpoints_fail_closed() {
    let mut store = EntityStore::new();
    let a = spatial(&mut store, 1, Vec3::ZERO);
    assert_eq!(
        store.apply_relation(RelationCommand::AttachTransformParent {
            child: a,
            parent: e(9)
        }),
        Err(RelationError::UnknownEntity { id: e(9) })
    );
    let b = spatial(&mut store, 2, Vec3::ZERO);
    store.apply(Cmd::Destroy { id: b }).unwrap();
    assert_eq!(
        store.apply_relation(RelationCommand::AttachTransformParent {
            child: a,
            parent: b
        }),
        Err(RelationError::Tombstoned { id: b })
    );
}

#[test]
fn relations_round_trip_through_save_reload() {
    let store = fixtures::attachment_contrast_family();
    let restored = EntityStore::from_snapshot(store.snapshot());
    assert_eq!(restored, store);
    assert_eq!(restored.hash(), store.hash());
    assert_eq!(restored.transform_parent_of(e(2)), Some(e(1)));
    assert_eq!(restored.derived_from(e(6)), Some(e(5)));
    assert_eq!(restored.containment(e(4)).unwrap().container, e(3));
}
