//! Spatial-world invariant (#2425, decision: option 1).
//!
//! `SpatialSessionState` is the spatial scene-runtime world: every live entity it holds has
//! a transform capability, enforced by construction. These tests prove that across
//! the public surface — so `SpatialSessionState::hash` never feeds a transform-less record
//! into the fingerprint and cannot panic from a normal public API path.

use core_assets::{markers, AssetRef, AssetReference, AssetVersionReq};
use core_ids::{EntityId, SceneNodeId, WorldId};
use core_math::Vec3;
use core_scene::{
    bootstrap_scene, FlatSceneDocument, NodeMetadata, SceneMetadata, SceneNodeKind,
    SceneNodeRecord, SceneTransform, SpatialSessionState,
};

fn at(x: f32) -> SceneTransform {
    SceneTransform {
        translation: Vec3::new(x, 0.0, 0.0),
        ..SceneTransform::IDENTITY
    }
}

fn mesh_ref(id: &str) -> AssetReference {
    AssetRef::<markers::StaticMesh>::parse(id, AssetVersionReq::Any, None)
        .unwrap()
        .erase()
}

/// A minimal valid scene so bootstrap-sourced entities are exercised too.
fn minimal_doc() -> FlatSceneDocument {
    let node = |id: u64, parent: Option<u64>, kind: SceneNodeKind| SceneNodeRecord {
        id: SceneNodeId::new(id),
        parent: parent.map(SceneNodeId::new),
        child_order: 0,
        transform: at(id as f32),
        kind,
        metadata: NodeMetadata::default(),
    };
    FlatSceneDocument {
        id: core_ids::SceneId::new(100),
        schema_version: 1,
        metadata: SceneMetadata {
            name: Some("inv".into()),
            authoring_format_version: 1,
        },
        dependencies: vec![mesh_ref("mesh/static-mesh-fixture-a")],
        nodes: vec![
            node(1, None, SceneNodeKind::EmptyGroup),
            node(
                2,
                Some(1),
                SceneNodeKind::StaticMesh(mesh_ref("mesh/static-mesh-fixture-a")),
            ),
        ],
    }
}

/// Assert the spatial-world invariant holds for `world`: every live entity exposed
/// by the public surface carries a transform, and `hash` agrees.
fn assert_spatial_invariant(world: &SpatialSessionState) {
    for (id, rec) in world.entities() {
        assert!(
            rec.transform.is_some(),
            "live world entity {id:?} must have a transform (spatial-world invariant)"
        );
        // The single-entity accessor agrees with the iterator.
        assert!(world.entity(id).is_some());
        assert!(world.entity(id).unwrap().transform.is_some());
        assert!(world.transform(id).is_some());
    }
    // hash() does not panic and is deterministic.
    assert_eq!(world.hash(), world.hash());
}

#[test]
fn runtime_created_entities_always_have_a_transform() {
    let mut world = SpatialSessionState::empty(WorldId::new(1));
    // The public constructor *requires* a transform argument — a transform-less
    // world entity is unconstructable through the API, not merely unused.
    assert!(world.create_runtime_entity(EntityId::new(10), at(1.0)));
    assert!(world.create_runtime_entity(EntityId::new(20), at(2.0)));
    assert_spatial_invariant(&world);
    assert_eq!(world.entities().count(), 2);
}

#[test]
fn bootstrapped_world_satisfies_the_invariant() {
    let (world, record) = bootstrap_scene(&minimal_doc(), WorldId::new(7)).unwrap();
    assert_spatial_invariant(&world);
    // The fingerprint computed here matches the one bootstrap recorded.
    assert_eq!(world.hash(), record.spatial_session_hash);
}

#[test]
fn empty_spatial_session_hash_does_not_panic_and_is_stable() {
    let a = SpatialSessionState::empty(WorldId::new(42));
    let b = SpatialSessionState::empty(WorldId::new(42));
    assert_eq!(a.hash(), b.hash());
    // A different world id yields a different fingerprint.
    assert_ne!(
        a.hash(),
        SpatialSessionState::empty(WorldId::new(43)).hash()
    );
}

#[test]
fn set_transform_preserves_the_invariant_and_changes_the_hash() {
    let mut world = SpatialSessionState::empty(WorldId::new(2));
    let e = EntityId::new(5);
    assert!(world.create_runtime_entity(e, at(0.0)));
    let before = world.hash();

    assert!(world.set_transform(e, at(9.0)));
    assert_spatial_invariant(&world);
    assert_ne!(
        before,
        world.hash(),
        "moving an entity changes the fingerprint"
    );

    // set_transform on an unknown entity is a no-op (no transform-less entity is
    // ever created as a side effect).
    assert!(!world.set_transform(EntityId::new(999), at(1.0)));
    assert_eq!(world.entities().count(), 1);
}
