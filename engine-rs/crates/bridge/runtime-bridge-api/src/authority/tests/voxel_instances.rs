use super::*;

fn transform(translation: [f32; 3]) -> SceneTransformDto {
    SceneTransformDto {
        translation,
        rotation: [
            0.0,
            0.0,
            std::f32::consts::FRAC_1_SQRT_2,
            std::f32::consts::FRAC_1_SQRT_2,
        ],
        scale: [2.0, 3.0, 0.5],
    }
}

fn binding_request(revision: u64) -> VoxelProjectionBindingRequest {
    VoxelProjectionBindingRequest {
        workspace_id: "workspace/studio".to_owned(),
        workspace_generation: 7,
        working_revision: revision,
        registry_digest: "sha256:registry-a".to_owned(),
        instances: vec![
            VoxelProjectionInstanceBinding {
                instance_id: "scene-node/10".to_owned(),
                scene_node_id: 10,
                asset_id: "voxel/house".to_owned(),
                transform: transform([10.0, 20.0, 30.0]),
            },
            VoxelProjectionInstanceBinding {
                instance_id: "scene-node/20".to_owned(),
                scene_node_id: 20,
                asset_id: "voxel/house".to_owned(),
                transform: SceneTransformDto {
                    translation: [-5.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                },
            },
        ],
    }
}

fn pick_request(receipt: &VoxelProjectionBindingReceipt) -> VoxelInstancePickRequest {
    VoxelInstancePickRequest {
        workspace_id: receipt.workspace_id.clone(),
        workspace_generation: receipt.workspace_generation,
        working_revision: receipt.working_revision,
        registry_digest: receipt.registry_digest.clone(),
        binding_hash: receipt.binding_hash.clone(),
        instance_id: "scene-node/10".to_owned(),
        // Local +X maps to world +Y. Start in local cell x=4 so the
        // launch fixture's earlier terrain cells cannot intercept the ray.
        origin: [8.5, 28.0, 30.25],
        direction: [0.0, 1.0, 0.0],
        max_distance: 20.0,
        renderer_hint: VoxelInstancePickHint {
            local_voxel: VoxelCoord::new(5, 0, 0),
            local_face: Face::NegX,
        },
    }
}

#[test]
fn public_binding_projects_two_independent_roots_and_hashes_every_transform() {
    let mut bridge = init_bridge();
    bridge
        .submit_commands(CommandBatch {
            commands: vec![VoxelCommand::SetVoxel {
                grid: GridId::new(1),
                coord: VoxelCoord::new(5, 0, 0),
                value: VoxelValue::solid_raw(1),
            }],
        })
        .unwrap();
    let receipt = bridge
        .configure_voxel_projection_instances(binding_request(3))
        .unwrap();
    assert_eq!(receipt.instance_count, 2);
    assert!(receipt.binding_hash.starts_with("fnv1a64:"));
    let frame = bridge.read_render_diffs(0).unwrap();
    let roots: Vec<_> = frame
        .ops
        .iter()
        .filter_map(|op| match op {
            protocol_render::RenderDiff::Create {
                handle,
                parent: None,
                node,
            } if node
                .metadata
                .label
                .as_deref()
                .is_some_and(|label| label.starts_with("voxel instance")) =>
            {
                Some(*handle)
            }
            _ => None,
        })
        .collect();
    assert_eq!(roots.len(), 2);
    assert_ne!(roots[0], roots[1]);
    assert!(frame.ops.iter().any(|op| matches!(
        op,
        protocol_render::RenderDiff::Create { parent: Some(parent), .. } if roots.contains(parent)
    )));

    let mut changed = binding_request(3);
    changed.instances[0].transform.translation[0] += 1.0;
    let changed_receipt = bridge
        .configure_voxel_projection_instances(changed)
        .unwrap();
    assert_ne!(changed_receipt.binding_hash, receipt.binding_hash);
    assert_eq!(
        changed_receipt.projection_op_count, 1,
        "only root A updates"
    );
}

#[test]
fn public_pick_is_transform_aware_and_rejects_stale_or_untrusted_inputs() {
    let mut bridge = init_bridge();
    bridge
        .submit_commands(CommandBatch {
            commands: vec![VoxelCommand::SetVoxel {
                grid: GridId::new(1),
                coord: VoxelCoord::new(5, 0, 0),
                value: VoxelValue::solid_raw(1),
            }],
        })
        .unwrap();
    let receipt = bridge
        .configure_voxel_projection_instances(binding_request(3))
        .unwrap();

    let accepted = bridge.pick_voxel_instance(pick_request(&receipt)).unwrap();
    match accepted.outcome {
        VoxelInstancePickOutcome::Hit(hit) => {
            assert_eq!(hit.local_voxel, VoxelCoord::new(5, 0, 0));
            assert_eq!(hit.local_place_anchor, VoxelCoord::new(4, 0, 0));
            assert!((hit.world_distance - 2.0).abs() < 1e-5);
        }
        other => panic!("expected transformed hit, got {other:?}"),
    }

    let mut wrong_hint = pick_request(&receipt);
    wrong_hint.renderer_hint.local_voxel = VoxelCoord::new(4, 0, 0);
    assert!(matches!(
        bridge.pick_voxel_instance(wrong_hint).unwrap().outcome,
        VoxelInstancePickOutcome::Rejected(VoxelInstancePickRejection::RendererHintMismatch)
    ));

    let mut wrong_digest = pick_request(&receipt);
    wrong_digest.registry_digest = "sha256:registry-b".to_owned();
    assert!(matches!(
        bridge.pick_voxel_instance(wrong_digest).unwrap().outcome,
        VoxelInstancePickOutcome::Rejected(VoxelInstancePickRejection::RegistryDigestChanged)
    ));

    let mut wrong_hash = pick_request(&receipt);
    wrong_hash.binding_hash = "fnv1a64:wrong".to_owned();
    assert!(matches!(
        bridge.pick_voxel_instance(wrong_hash).unwrap().outcome,
        VoxelInstancePickOutcome::Rejected(VoxelInstancePickRejection::BindingHashMismatch)
    ));

    bridge
        .submit_commands(CommandBatch {
            commands: vec![VoxelCommand::SetVoxel {
                grid: GridId::new(1),
                coord: VoxelCoord::new(6, 0, 0),
                value: VoxelValue::solid_raw(1),
            }],
        })
        .unwrap();
    assert!(matches!(
        bridge
            .pick_voxel_instance(pick_request(&receipt))
            .unwrap()
            .outcome,
        VoxelInstancePickOutcome::Rejected(VoxelInstancePickRejection::StaleWorkingRevision)
    ));

    let newer = bridge
        .configure_voxel_projection_instances(binding_request(4))
        .unwrap();
    let replayed = bridge.pick_voxel_instance(pick_request(&receipt)).unwrap();
    assert!(matches!(
        replayed.outcome,
        VoxelInstancePickOutcome::Rejected(VoxelInstancePickRejection::StaleWorkingRevision)
    ));
    assert!(matches!(
        bridge
            .pick_voxel_instance(pick_request(&newer))
            .unwrap()
            .outcome,
        VoxelInstancePickOutcome::Hit(_)
    ));
}

#[test]
fn explicit_empty_instance_binding_never_recreates_the_legacy_default() {
    let mut bridge = init_bridge();
    bridge
        .submit_commands(CommandBatch {
            commands: vec![VoxelCommand::SetVoxel {
                grid: GridId::new(1),
                coord: VoxelCoord::new(5, 0, 0),
                value: VoxelValue::solid_raw(1),
            }],
        })
        .unwrap();
    bridge
        .configure_voxel_projection_instances(binding_request(3))
        .unwrap();
    let _ = bridge.read_render_diffs(0).unwrap();

    let mut empty = binding_request(4);
    empty.instances.clear();
    let receipt = bridge.configure_voxel_projection_instances(empty).unwrap();
    assert_eq!(receipt.instance_count, 0);
    assert_eq!(receipt.projection_op_count, 2);

    let teardown = bridge.read_render_diffs(1).unwrap();
    assert_eq!(teardown.ops.len(), 2);
    assert!(teardown
        .ops
        .iter()
        .all(|op| matches!(op, protocol_render::RenderDiff::Destroy { .. })));

    bridge
        .submit_commands(CommandBatch {
            commands: vec![VoxelCommand::SetVoxel {
                grid: GridId::new(1),
                coord: VoxelCoord::new(6, 0, 0),
                value: VoxelValue::solid_raw(1),
            }],
        })
        .unwrap();
    let after_edit = bridge.read_render_diffs(2).unwrap();
    assert!(after_edit.ops.is_empty());
}
