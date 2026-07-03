//! Atomic bootstrap: deterministic authority init, fail-before-mutation, single
//! replay unit, and runtime/scene transform divergence (subtask #2316).

use core_assets::{markers, AssetRef, AssetReference, AssetVersionReq};
use core_entity::EntitySource;
use core_ids::{EntityId, SceneId, SceneNodeId, WorldId};
use core_math::Vec3;
use core_scene::{
    bootstrap_scene, BootstrapError, BootstrapPlan, FlatSceneDocument, NodeMetadata, SceneMetadata,
    SceneNodeKind, SceneNodeRecord, SceneTransform, SceneValidationError, TransformInvalid,
};

fn mesh_ref(id: &str) -> AssetReference {
    AssetRef::<markers::StaticMesh>::parse(id, AssetVersionReq::Any, None)
        .unwrap()
        .erase()
}

fn record(id: u64, parent: Option<u64>, order: u32, kind: SceneNodeKind) -> SceneNodeRecord {
    SceneNodeRecord {
        id: SceneNodeId::new(id),
        parent: parent.map(SceneNodeId::new),
        child_order: order,
        transform: SceneTransform {
            translation: Vec3::new(id as f32, 0.0, 0.0),
            ..SceneTransform::IDENTITY
        },
        kind,
        metadata: NodeMetadata::default(),
    }
}

/// A minimal valid scene: a root empty group with one static-mesh child.
fn minimal_doc() -> FlatSceneDocument {
    FlatSceneDocument {
        id: SceneId::new(100),
        schema_version: 1,
        metadata: SceneMetadata {
            name: Some("boot".into()),
            authoring_format_version: 1,
        },
        dependencies: vec![mesh_ref("mesh/static-mesh-fixture-a")],
        nodes: vec![
            record(1, None, 0, SceneNodeKind::EmptyGroup),
            record(
                2,
                Some(1),
                0,
                SceneNodeKind::StaticMesh(mesh_ref("mesh/static-mesh-fixture-a")),
            ),
        ],
    }
}

#[test]
fn bootstrap_from_valid_scene_is_deterministic() {
    let doc = minimal_doc();
    let (world_a, rec_a) = bootstrap_scene(&doc, WorldId::new(7)).expect("valid scene");
    let (world_b, rec_b) = bootstrap_scene(&doc, WorldId::new(7)).expect("valid scene");

    // Same input → identical authority state and fingerprint.
    assert_eq!(world_a, world_b);
    assert_eq!(rec_a.world_hash, rec_b.world_hash);
    assert_eq!(world_a.hash(), rec_a.world_hash);

    assert_eq!(world_a.entity_count(), 2);
    assert_eq!(rec_a.node_count, 2);
    assert_eq!(rec_a.entity_count, 2);

    // Initial transforms were copied into authority for each scene node.
    let e1 = world_a.entity_for_node(SceneNodeId::new(1)).unwrap();
    let e2 = world_a.entity_for_node(SceneNodeId::new(2)).unwrap();
    assert_eq!(
        world_a.transform(e1).unwrap().translation,
        Vec3::new(1.0, 0.0, 0.0)
    );
    assert_eq!(
        world_a.transform(e2).unwrap().translation,
        Vec3::new(2.0, 0.0, 0.0)
    );
    // Provenance is recorded both directions.
    assert_eq!(world_a.source_node(e1), Some(SceneNodeId::new(1)));
}

#[test]
fn invalid_scene_fails_before_producing_a_world() {
    let mut doc = minimal_doc();
    // Collapse a scale axis → invalid transform.
    doc.nodes[1].transform.scale = Vec3::new(0.0, 1.0, 1.0);

    match BootstrapPlan::prepare(&doc, WorldId::new(1)) {
        Err(BootstrapError::Invalid(report)) => {
            assert!(report.errors.iter().any(|e| matches!(
                e,
                SceneValidationError::InvalidTransform {
                    reason: TransformInvalid::ZeroScaleAxis,
                    ..
                }
            )));
        }
        other => panic!("expected Invalid, got {other:?}"),
    }
    // The convenience path also refuses, with no world handed back.
    assert!(bootstrap_scene(&doc, WorldId::new(1)).is_err());
}

#[test]
fn unsupported_schema_version_fails_closed() {
    let mut doc = minimal_doc();
    doc.schema_version = 999;
    assert_eq!(
        BootstrapPlan::prepare(&doc, WorldId::new(1)),
        Err(BootstrapError::UnsupportedSchemaVersion {
            found: 999,
            supported: 1,
        })
    );
}

#[test]
fn replay_sees_one_bootstrap_unit() {
    let (_world, record) = bootstrap_scene(&minimal_doc(), WorldId::new(3)).unwrap();
    // One record stands in for the whole init — counts summarize it, not N events.
    assert_eq!(record.replay_unit_label(), "scene.bootstrap");
    assert_eq!(record.node_count, record.entity_count);
    assert_eq!(record.source_trace.len(), 2);
    assert_eq!(record.scene_id, SceneId::new(100));
}

#[test]
fn ecrp_project_bundle_scene_bootstrap_seeds_session_capability_state() {
    // Current implementation compatibility:
    // - stored ProjectBundle-like content is represented by FlatSceneDocument;
    // - RuntimeSession/SessionState is represented by core_scene::WorldState.
    // The proof stays in Rust authority and uses deterministic replay/hash
    // readouts instead of exposing StateStore or inventing a TS JSON hatch.
    let doc = minimal_doc();
    let (world, record) = bootstrap_scene(&doc, WorldId::new(44)).unwrap();

    assert_eq!(record.replay_unit_label(), "scene.bootstrap");
    assert_eq!(record.world_id, WorldId::new(44));
    assert_eq!(record.world_hash, world.hash());
    assert_eq!(record.source_trace.len(), 2);

    let mesh_entity = world.entity_for_node(SceneNodeId::new(2)).unwrap();
    let runtime = world.entity(mesh_entity).unwrap();
    assert_eq!(runtime.source_node, Some(SceneNodeId::new(2)));
    assert_eq!(
        runtime.transform.unwrap().translation,
        Vec3::new(2.0, 0.0, 0.0)
    );

    let snapshot = world.entity_snapshot();
    let mesh_record = snapshot
        .records
        .iter()
        .find(|record| record.core.id == mesh_entity)
        .expect("bootstrapped mesh entity is snapshotted");
    assert!(matches!(
        &mesh_record.core.source,
        EntitySource::SceneBootstrap { node } if *node == SceneNodeId::new(2)
    ));
    assert_eq!(
        mesh_record.transform.unwrap().transform.translation,
        Vec3::new(2.0, 0.0, 0.0)
    );

    let baseline_hash = world.entity_hash();
    assert_eq!(baseline_hash, world.entity_hash());
    assert_eq!(
        doc,
        minimal_doc(),
        "bootstrap does not mutate stored content"
    );
}

#[test]
fn runtime_transform_diverges_without_mutating_scene_document() {
    let doc = minimal_doc();
    let (mut world, record) = bootstrap_scene(&doc, WorldId::new(5)).unwrap();
    let entity = world.entity_for_node(SceneNodeId::new(2)).unwrap();

    let moved = SceneTransform {
        translation: Vec3::new(99.0, 99.0, 99.0),
        ..SceneTransform::IDENTITY
    };
    assert!(world.set_transform(entity, moved));

    // World moved...
    assert_eq!(
        world.transform(entity).unwrap().translation,
        Vec3::new(99.0, 99.0, 99.0)
    );
    assert_ne!(world.hash(), record.world_hash);
    // ...but the authored document is untouched.
    assert_eq!(doc, minimal_doc());
    assert_eq!(doc.nodes[1].transform.translation, Vec3::new(2.0, 0.0, 0.0));
}

#[test]
fn runtime_created_entity_has_no_scene_provenance() {
    let (mut world, _) = bootstrap_scene(&minimal_doc(), WorldId::new(8)).unwrap();
    let scene_entity = world.entity_for_node(SceneNodeId::new(1)).unwrap();
    assert!(world.source_node(scene_entity).is_some());

    let runtime_entity = EntityId::new(10_000);
    assert!(world.create_runtime_entity(runtime_entity, SceneTransform::IDENTITY));
    assert_eq!(world.source_node(runtime_entity), None);
    assert!(world.entity_for_node(SceneNodeId::new(1)).is_some());
}

#[test]
fn allocation_order_is_canonical_regardless_of_input_order() {
    // Author nodes out of id order; allocation must still be ascending node id.
    let mut doc = minimal_doc();
    doc.nodes.reverse();
    let plan = BootstrapPlan::prepare(&doc, WorldId::new(1)).unwrap();
    let allocs = plan.allocations();
    assert_eq!(allocs[0].node, SceneNodeId::new(1));
    assert_eq!(allocs[0].entity, EntityId::new(1));
    assert_eq!(allocs[1].node, SceneNodeId::new(2));
    assert_eq!(allocs[1].entity, EntityId::new(2));
}
