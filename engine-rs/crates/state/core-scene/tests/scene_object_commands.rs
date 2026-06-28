//! Authority tests for canonical scene-object hierarchy commands.

use core_assets::{markers, AssetRef, AssetReference, AssetVersionReq};
use core_ids::{SceneId, SceneNodeId};
use core_scene::{
    apply_scene_object_command, scene_object_snapshot, FlatSceneDocument, NodeMetadata,
    SceneMetadata, SceneNode, SceneNodeKind, SceneNodeRecord, SceneObjectCommand,
    SceneObjectCommandRejection, SceneTransform, SceneTree,
};

fn mesh_ref(id: &str) -> AssetReference {
    AssetRef::<markers::StaticMesh>::parse(id, AssetVersionReq::Any, None)
        .unwrap()
        .erase()
}

fn base_doc() -> FlatSceneDocument {
    let mesh = SceneNode::leaf(
        SceneNodeId::new(2),
        SceneNodeKind::StaticMesh(mesh_ref("mesh/static-mesh-fixture-a")),
    );
    let group =
        SceneNode::leaf(SceneNodeId::new(3), SceneNodeKind::EmptyGroup).with_children(vec![mesh]);
    let root =
        SceneNode::leaf(SceneNodeId::new(1), SceneNodeKind::EmptyGroup).with_children(vec![group]);

    SceneTree {
        id: SceneId::new(42),
        schema_version: 1,
        metadata: SceneMetadata {
            name: Some("hierarchy".into()),
            authoring_format_version: 1,
        },
        dependencies: vec![mesh_ref("mesh/static-mesh-fixture-a")],
        roots: vec![root],
    }
    .to_flat()
}

#[test]
fn snapshot_projects_scene_objects_not_render_handles() {
    let snapshot = scene_object_snapshot(&base_doc());

    assert_eq!(snapshot.objects.len(), 3);
    assert_eq!(snapshot.objects[0].id, SceneNodeId::new(1));
    assert_eq!(snapshot.objects[1].parent, Some(SceneNodeId::new(3)));
    assert_eq!(snapshot.objects[1].kind, "staticMesh");
    assert!(snapshot.objects[1].has_renderable_asset);
    assert_eq!(snapshot.objects[2].kind, "emptyGroup");
    assert!(!snapshot.objects[2].has_renderable_asset);
}

#[test]
fn rename_uses_expected_snapshot_and_returns_new_snapshot() {
    let doc = base_doc();
    let hash = scene_object_snapshot(&doc).document_hash;

    let outcome = apply_scene_object_command(
        &doc,
        hash,
        SceneObjectCommand::Rename {
            id: SceneNodeId::new(2),
            label: Some("Crate".into()),
        },
    )
    .expect("rename accepted");

    let renamed = outcome
        .document
        .nodes
        .iter()
        .find(|node| node.id == SceneNodeId::new(2))
        .unwrap();
    assert_eq!(renamed.metadata.label.as_deref(), Some("Crate"));
    assert_eq!(outcome.selected, Some(SceneNodeId::new(2)));
    assert_ne!(outcome.snapshot.document_hash, hash);
}

#[test]
fn stale_snapshot_is_rejected_before_mutation() {
    let doc = base_doc();
    let stale = scene_object_snapshot(&doc).document_hash;
    let updated = apply_scene_object_command(
        &doc,
        stale,
        SceneObjectCommand::Rename {
            id: SceneNodeId::new(2),
            label: Some("Crate".into()),
        },
    )
    .unwrap()
    .document;

    let err = apply_scene_object_command(
        &updated,
        stale,
        SceneObjectCommand::Select {
            id: Some(SceneNodeId::new(2)),
        },
    )
    .unwrap_err();

    assert!(matches!(
        err,
        SceneObjectCommandRejection::StaleSnapshot { .. }
    ));
    assert_eq!(err.label(), "stale-scene-object-snapshot");
}

#[test]
fn create_and_delete_subtree_are_validated_authority_commands() {
    let doc = base_doc();
    let hash = scene_object_snapshot(&doc).document_hash;
    let created = apply_scene_object_command(
        &doc,
        hash,
        SceneObjectCommand::Create {
            record: SceneNodeRecord {
                id: SceneNodeId::new(4),
                parent: Some(SceneNodeId::new(1)),
                child_order: 1,
                transform: SceneTransform::IDENTITY,
                kind: SceneNodeKind::EmptyGroup,
                metadata: NodeMetadata {
                    label: Some("Folder".into()),
                    tags: Vec::new(),
                },
            },
        },
    )
    .expect("create accepted");

    assert!(created
        .snapshot
        .objects
        .iter()
        .any(|object| object.id == SceneNodeId::new(4)));

    let deleted = apply_scene_object_command(
        &created.document,
        created.snapshot.document_hash,
        SceneObjectCommand::Delete {
            id: SceneNodeId::new(3),
        },
    )
    .expect("delete accepted");

    assert!(!deleted
        .document
        .nodes
        .iter()
        .any(|node| node.id == SceneNodeId::new(2)));
    assert!(!deleted
        .document
        .nodes
        .iter()
        .any(|node| node.id == SceneNodeId::new(3)));
}

#[test]
fn reparent_cycle_is_rejected_with_invalid_after_classification() {
    let doc = base_doc();
    let hash = scene_object_snapshot(&doc).document_hash;

    let err = apply_scene_object_command(
        &doc,
        hash,
        SceneObjectCommand::Reparent {
            id: SceneNodeId::new(1),
            parent: Some(SceneNodeId::new(2)),
            child_order: 0,
        },
    )
    .unwrap_err();

    assert!(matches!(
        err,
        SceneObjectCommandRejection::InvalidAfter { .. }
    ));
    assert_eq!(err.label(), "invalid-scene-after-command");
}

#[test]
fn blank_label_is_rejected_without_a_private_edit_path() {
    let doc = base_doc();
    let hash = scene_object_snapshot(&doc).document_hash;

    let err = apply_scene_object_command(
        &doc,
        hash,
        SceneObjectCommand::Rename {
            id: SceneNodeId::new(2),
            label: Some("   ".into()),
        },
    )
    .unwrap_err();

    assert!(matches!(
        err,
        SceneObjectCommandRejection::BlankLabel { .. }
    ));
}
