use core_ids::{ProjectId, SceneId};

use super::*;

fn source_fixture() -> (String, Vec<u8>, Vec<u8>, Vec<u8>) {
    let lock = b"asset-lock".to_vec();
    let scene = b"entry-scene".to_vec();
    let voxel = b"voxel-house".to_vec();
    let manifest = svc_serialization::ProjectBundleManifest {
        bundle_schema_version: svc_serialization::BUNDLE_SCHEMA_VERSION,
        protocol_version: svc_serialization::SUPPORTED_PROTOCOL_VERSION,
        project: svc_serialization::ProjectSection {
            id: ProjectId::new(7),
            name: Some("bridge-source-fixture".into()),
        },
        entry_scene: SceneId::new(10),
        scenes: vec![svc_serialization::SceneSection {
            id: SceneId::new(10),
            schema_version: 1,
            artifact: "scene/entry.json".into(),
        }],
        asset_lock: svc_serialization::AssetLockSection {
            artifact: "assets/lock.json".into(),
            asset_count: 0,
        },
        generation_provenance: None,
        artifacts: vec![
            svc_serialization::ArtifactEntry::durable(
                "assets/lock.json",
                svc_serialization::ArtifactRole::AssetLock,
                &lock,
            ),
            svc_serialization::ArtifactEntry::durable(
                "scene/entry.json",
                svc_serialization::ArtifactRole::SceneDocument,
                &scene,
            ),
            svc_serialization::ArtifactEntry::durable(
                "voxel/house.avox",
                svc_serialization::ArtifactRole::VoxelVolumeAsset,
                &voxel,
            ),
        ],
    };
    (svc_serialization::encode(&manifest), lock, scene, voxel)
}

fn stage_batch(
    bridge: &mut EngineBridge,
    manifest_json: &str,
    lock: Vec<u8>,
    scene: Vec<u8>,
    voxel: Vec<u8>,
) -> protocol_project_bundle::RuntimeProjectSourceBatch {
    let transaction = bridge
        .begin_runtime_project_source_resources(manifest_json)
        .expect("begin source transaction");
    let staged = bridge
        .stage_runtime_project_source_resource(transaction, "voxel/house.avox", voxel)
        .expect("stage voxel bytes");
    protocol_project_bundle::RuntimeProjectSourceBatch {
        manifest_json: manifest_json.to_string(),
        resource_generation: Some(transaction.generation()),
        bodies: vec![
            protocol_project_bundle::ProjectSourceBody::Inline {
                path: "assets/lock.json".into(),
                bytes: lock,
            },
            protocol_project_bundle::ProjectSourceBody::Inline {
                path: "scene/entry.json".into(),
                bytes: scene,
            },
            protocol_project_bundle::ProjectSourceBody::Resource {
                path: "voxel/house.avox".into(),
                resource: protocol_project_bundle::StagedProjectResourceRef {
                    handle: staged.handle.raw(),
                    generation: staged.generation,
                    version: staged.version,
                    byte_len: staged.byte_len,
                },
            },
        ],
    }
}

#[test]
fn rejected_source_batch_cleans_staging_without_replacing_prior_admission() {
    let mut bridge = init_bridge();
    let (manifest_json, lock, scene, voxel) = source_fixture();
    let accepted_batch = stage_batch(
        &mut bridge,
        &manifest_json,
        lock.clone(),
        scene.clone(),
        voxel.clone(),
    );
    let accepted = bridge
        .admit_runtime_project_source_batch(accepted_batch)
        .expect("admit valid source batch");
    assert!(accepted.accepted);
    let accepted_hash = bridge
        .pending_project_source()
        .expect("opaque admitted source")
        .manifest_hash();

    let mut rejected_batch = stage_batch(&mut bridge, &manifest_json, lock, scene, voxel);
    let protocol_project_bundle::ProjectSourceBody::Inline { bytes, .. } =
        &mut rejected_batch.bodies[0]
    else {
        panic!("fixture first body is inline");
    };
    bytes.push(0);
    let rejected = bridge
        .admit_runtime_project_source_batch(rejected_batch)
        .expect("classified rejection receipt");

    assert!(!rejected.accepted);
    assert_eq!(
        rejected.diagnostics[0].code,
        protocol_project_bundle::ProjectSourceBatchErrorCode::ContentHashMismatch
    );
    assert_eq!(
        bridge
            .pending_project_source()
            .expect("prior admission remains authoritative")
            .manifest_hash(),
        accepted_hash
    );
    assert_eq!(bridge.bundle.project_resource_staging.staged_count(), 0);
}
